# Chương 89: Độ bền Phục hồi Sự cố — Cổng Mô hình, Nhật ký WAL, Cây Phiên, Outbox, Saga & Giao thức MCP + A2A + ANP (Crash-Resilient Durability, Session Tree, Saga & Agent Protocols)

## 1. Giới thiệu & Mục tiêu

Khi lập trình các ứng dụng web hoặc hệ thống backend truyền thống theo kiến trúc hướng dịch vụ, chúng ta thường dựa vào các giả định rất tiện lợi: các vi dịch vụ (microservices) phần lớn là phi trạng thái (stateless), cơ sở dữ liệu quan hệ bảo đảm đặc tính ACID mạnh mẽ qua các giao dịch hai pha hoặc khóa bi quan, và nếu một tiến trình gặp sự cố (crash), bộ điều phối container như Kubernetes chỉ việc khởi động lại một bản sao mới (replica) và định tuyến lại yêu cầu HTTP.

Tuy nhiên, trong thế giới của **Tác tử Tự hành (Autonomous AI Agents)**, bức tranh hoàn toàn đảo lộn:

1. **Agent là những thực thể mang trạng thái sâu sắc (Stateful Entities)**: Một phiên làm việc của Agent lập trình tự hành có thể kéo dài hàng giờ đồng hồ, trải qua hàng chục lượt suy luận (turns), tích lũy hàng triệu token ngữ cảnh, ghi nhận các suy nghĩ nội tâm (chain-of-thought), và lưu giữ bộ nhớ tiến trình (working memory). Việc mất toàn bộ ngữ cảnh này khi tiến trình bị sập đột ngột (OOM, đứt kết nối mạng, máy ảo khởi động lại) không chỉ làm lãng phí một khoản chi phí API khổng lồ mà còn phá hủy hoàn toàn tiến trình giải quyết bài toán đang dang dở.
2. **Hiệu ứng phụ thế giới thực không thể tự động Rollback**: Khi một backend CRUD gặp lỗi, câu lệnh `ROLLBACK` sẽ hoàn tác các dòng dữ liệu trong bảng. Nhưng khi Agent gọi một công cụ (Tool Call) ra thế giới thực — như đặt lệnh mua tài sản trên sàn chứng khoán, kích hoạt một đường ống CI/CD, gửi email thông báo cho khách hàng, hay tạo tài nguyên hạ tầng đám mây — những hành động này **không thể tự động rút lại**. Nếu Agent gặp sự cố ngay sau khi vừa gửi lệnh nhưng chưa kịp lưu kết quả, và khi khởi động lại nó vô tình thực thi lại lệnh đó, thảm họa **nhân đôi hiệu ứng phụ (duplicate side-effects)** sẽ xảy ra.
3. **Mạng lưới nhà cung cấp LLM là môi trường bất ổn định**: API của các nhà cung cấp mô hình trí tuệ nhân tạo thường xuyên đối mặt với tình trạng biến động độ trễ (latency spikes), chạm trần hạn ngạch (429 Too Many Requests), lỗi máy chủ nội bộ (500/502/503), hoặc thậm chí là lỗi cú pháp/lược đồ yêu cầu (400 Bad Request). Một kiến trúc non nớt nếu coi mọi lỗi đều giống nhau và kích hoạt vòng lặp thử lại điên cuồng sẽ nhanh chóng kích hoạt hiện tượng "sấm sét bầy đàn" (Thundering Herd Problem), làm cạn kiệt ngân sách tài chính (FinOps Budget) chỉ trong vài phút, hoặc làm sập toàn bộ các rào chắn bảo vệ của hệ thống.

Để giải quyết tận gốc các thách thức sống còn trên, Chương 89 xây dựng một nền tảng độ bền phục hồi sự cố toàn diện, được hiện thực hóa **100% bằng ngôn ngữ Rust 2024 thuần thư viện chuẩn `std`** (không phụ thuộc bất kỳ crate bên ngoài nào), với các mục tiêu kỹ thuật cốt lõi:

- **Cổng Mô hình Thông minh (`LlmGateway`)**:
  - Phân loại lỗi mạng và nhà cung cấp thành 4 nhóm riêng biệt: `Fatal4xx` (lỗi nghiệp vụ/schema, cấm tuyệt đối retry và không bao giờ mở cầu dao), `RateLimit429` (thử lại có kiểm soát), `Transient5xx` (lỗi máy chủ tạm thời, tích lũy mở cầu dao), và `ContextOverflow` (tràn cửa sổ ngữ cảnh).
  - Cầu dao ngắt mạch thông minh (`CircuitBreaker`) với 3 trạng thái `Closed`, `Open`, `HalfOpen`, kiên quyết không ngắt mạch oan vì lỗi client.
  - Thuật toán giật lùi độ trễ phi tương quan (`DecorrelatedJitterBackoff`) kết hợp bộ sinh số ngẫu nhiên giả tất định `SplitMix64`.
  - Quản trị ngân sách tài chính FinOps với cơ chế phân bổ tải linh hoạt `stable_fraction` giữa các bậc mô hình (Primary, Secondary, LocalFallback).
- **Nhật ký Ghi trước cho Agent (`TurnWal`)**:
  - Cưỡng chế nghiêm ngặt bất biến cốt lõi: **`"Model-visible means logged"`** — Bất kỳ token, văn bản hay dữ liệu ngữ cảnh nào được đưa vào cửa sổ tầm nhìn của mô hình đều BẮT BUỘC phải được ghi nhận vào nhật ký trước khi truyền đi.
  - Đóng gói khung nhị phân toàn vẹn với mã băm mật mã SHA-256 (chuẩn FIPS 180-4 tự cài đặt thuần `std`).
  - Cơ chế tự gọt đuôi rách (Torn-tail truncation) khi phục hồi sau sự cố sập nguồn đột ngột.
- **Cây Phiên Hội thoại Rẽ nhánh (`SessionTree`)**:
  - Hỗ trợ câu lệnh `/tree` phân nhánh lịch sử hội thoại mà không bao giờ ghi đè hay làm mất dữ liệu của các nhánh cũ.
  - Tái tạo tuyến tính nguyên vẹn từng byte từ nút gốc tới bất kỳ nút lá nào qua con trỏ tổ tiên (`parent_id`).
- **Kho Lũy đẳng Hai pha (`IdempotencyStore`)**:
  - Chuyển trạng thái giao dịch từ `Pending { lease_until }` sang `Committed { output_hash }`.
  - Chống xung đột tương tranh và hỗ trợ thu hồi an toàn các hợp đồng thuê (lease) đã quá hạn khi tiến trình trước bị sập.
- **Hộp thư ra Giao dịch (`TransactionalOutbox`)**:
  - Đảm bảo tính nhất quán giữa trạng thái nội bộ của Agent và các sự kiện phát tán ra bên ngoài theo nguyên lý "at-least-once".
- **Điều phối Bù trừ Ngược (`OrchestratedSaga`)**:
  - Quản lý giao dịch phân tán giữa các bước thực thi công cụ.
  - Tự động hoàn tác ngược chiều $k \to k-1 \to \dots \to 1$ khi xảy ra sự cố, và bảo đảm tính bền bỉ tiếp tục hoàn tác kể cả khi một bước đơn lẻ gặp lỗi.
- **Kiểm soát Tương tranh Lạc quan & Thẻ Rào chắn (`OptimisticFileVersion` & `FencingToken`)**:
  - Cơ chế CAS hai lớp `(version: u64, sha256: [u8; 32])` chống ghi đè tệp mã nguồn.
  - Thẻ rào chắn đơn điệu epoch chặn đứng hoàn toàn tác tử cũ (Zombie Agent) can thiệp vào tài nguyên.
- **Ngăn xếp 3 Giao thức Tác tử Hiện đại (`MCP` + `A2A` + `ANP`)**:
  - `McpJsonRpcRouter`: Định tuyến công cụ chuẩn Model Context Protocol qua JSON-RPC 2.0.
  - `A2aAgentNode`: Giao tiếp Agent-to-Agent với thẻ năng lực `AgentCard` và máy trạng thái tác vụ.
  - `AnpDiscoveryRegistry`: Sổ đăng bạ khám phá mạng lưới tác tử và định tuyến cân bằng tải thông minh (Least-Loaded Routing).

---

## 2. Hình tượng hóa đời sống: Hộp đen máy bay cứu hộ và trạm trung chuyển hàng không

Để có được trực giác sâu sắc về lý do tại sao hệ thống tác tử tự hành lại cần một cấu trúc độ bền chặt chẽ đến vậy, hãy cùng quan sát hai hình tượng quen thuộc trong đời sống thực tế: **Hộp đen máy bay cứu hộ** và **Quy trình hoàn vé của Trạm trung chuyển hàng không quốc tế**.

### 2.1. Hộp đen máy bay cứu hộ và Bất biến "Model-visible means logged"

Hãy tưởng tượng một chiếc máy bay cứu hộ chuyên dụng bay vào vùng bão nhiệt đới để thực hiện nhiệm vụ tìm kiếm cứu nạn. Trong buồng lái có hai phi công và hàng trăm thiết bị đo đạc cảm biến phức tạp. Trên máy bay có một thiết bị đặc biệt được bọc trong vỏ titan siêu bền, có khả năng chịu được sức ép ngàn độ C và va đập ở tốc độ siêu thanh: **Hộp đen ghi dữ liệu chuyến bay (Flight Data Recorder - FDR)**.

```
       BẢNG ĐIỀU KHIỂN BUỒNG LÁI (CONTEXT)
   ┌──────────────────────────────────────────┐
   │ Cảnh báo: Tốc độ gió giật 120 km/h       │
   │ Độ cao: 1.500m                           │
   │ Nhiên liệu còn lại: 35%                  │
   └────────────────────┬─────────────────────┘
                        │
      BẤT BIẾN: PHẢI GHI XONG TRƯỚC!
                        ▼
   ┌──────────────────────────────────────────┐
   │      HỘP ĐEN TITAN (TURN WAL)            │
   │  Khung #104: Wind=120, Alt=1500, Fuel=35 │
   │  Checksum SHA-256: [e3b0c442...] ĐÃ KHÓA │
   └────────────────────┬─────────────────────┘
                        │ (Đã flush xuống đĩa bền vững)
                        ▼
            PHI CÔNG / MÔ HÌNH LLM
   ┌──────────────────────────────────────────┐
   │  Đọc thông tin -> Quyết định hạ cánh khẩn│
   └──────────────────────────────────────────┘
```

Trong ngành hàng không vũ trụ, có một quy tắc an toàn bất khả xâm phạm: **Không một thông tin hay cảnh báo nào được phép hiển thị lên màn hình của phi công trước khi tín hiệu đó được ghi nhận và đồng bộ an toàn vào bộ nhớ của hộp đen**. 

Nếu một nhà sản xuất máy bay hiển thị cảnh báo cho phi công xử lý trước, rồi mới "tranh thủ" ghi vào hộp đen sau khi rảnh rỗi, thì chuyện gì sẽ xảy ra khi máy bay đột ngột mất điện toàn bộ buồng lái ngay tại khoảnh khắc phi công vừa ra quyết định? Khi đội điều tra cứu hộ trục vớt hộp đen từ đáy biển, họ sẽ thấy hộp đen hoàn toàn trống trơn ở giây cuối cùng. Họ không thể biết được phi công đã nhìn thấy điều gì để đưa ra quyết định chuyển hướng lái!

Trong kiến trúc Agent, **Mô hình LLM chính là Phi công**, còn **TurnWal chính là Hộp đen**. Nếu chúng ta gọi API mô hình LLM với một lời nhắc (prompt) chứa các dữ liệu ngữ cảnh vừa được sinh ra, nhưng lại trì hoãn việc ghi vào nhật ký đến sau khi API trả lời, thì khi tiến trình bị hệ điều hành tắt ngang (SIGKILL), Agent sẽ "tỉnh dậy" trong trạng thái mất trí nhớ cục bộ. Nó không biết rằng 5 giây trước, mô hình đã nhìn thấy tài liệu mật gì, đã quyết định thực thi lệnh gì. Cưỡng chế bất biến **"Model-visible means logged"** bảo đảm rằng: bất kể tai nạn gì xảy ra trên đường truyền, trạng thái nhìn thấy của Agent luôn được tái dựng chính xác đến từng bit.

---

### 2.2. Trạm trung chuyển hàng không và Điều phối bù trừ ngược Saga

Bây giờ, hãy chuyển sang một sân bay trung chuyển quốc tế. Một hành khách muốn bay từ Hà Nội đến Buenos Aires (Argentina). Hành trình đòi hỏi 4 chặng bay liên tiếp qua các hãng hàng không khác nhau:
1. Chặng 1: Hà Nội $\to$ Bangkok (Hãng A)
2. Chặng 2: Bangkok $\to$ Dubai (Hãng B)
3. Chặng 3: Dubai $\to$ Sao Paulo (Hãng C)
4. Chặng 4: Sao Paulo $\to$ Buenos Aires (Hãng D)

Khi hành khách đã bay xong chặng 1 và chặng 2, đang ngồi chờ ở Dubai để lên máy bay chặng 3, thì một cơn bão quét qua sân bay Sao Paulo khiến toàn bộ các chuyến bay của chặng 3 và 4 bị hủy hoàn toàn trong 48 giờ tới. Hành khách quyết định hủy toàn bộ chuyến đi để quay về nhà.

Hệ thống đặt vé trung tâm không thể dùng phép màu "Rollback" để xóa bỏ việc hành khách đã bay từ Hà Nội sang Dubai. Thân thể hành khách đã ở Dubai, xăng máy bay đã đốt, bữa ăn trên máy bay đã được phục vụ. Hệ thống bắt buộc phải thực hiện **Quy trình bù trừ ngược (Reverse Compensation)**:
- Không thể hoàn tác chặng 4 vì chặng 4 chưa bay $\to$ Hủy đặt chỗ chặng 4.
- Không thể hoàn tác chặng 3 vì chặng 3 bị hủy $\to$ Đóng hồ sơ chặng 3.
- Bù trừ chặng 2 $\to$ Đặt vé bay ngược từ Dubai về Bangkok cho hành khách.
- Bù trừ chặng 1 $\to$ Đặt vé bay ngược từ Bangkok về Hà Nội và hoàn trả phần tiền chênh lệch.

```
TIẾN TRÌNH THỰC THI XUÔI (FORWARD EXECUTION):
[Bước 1: Bay B1] ──► [Bước 2: Bay B2] ──► [Bước 3: Bay B3] ──► [Bước 4: Bay B4 (GÃY!)]
                                                                       │
                                                       SỰ CỐ XẢY RA!   │
                                                                       ▼
HOÀN TÁC BÙ TRỪ NGƯỢC (REVERSE COMPENSATION):
[Bù trừ B1: Bay về] ◄── [Bù trừ B2: Bay về] ◄── [Bù trừ B3: Hoàn tiền] ◄┘
 (Thực thi cuối)         (Thực thi kế)           (Thực thi đầu tiên)
```

Quan trọng hơn nữa: Nếu trong quá trình bay ngược từ Dubai về Bangkok (Bù trừ bước 2), hệ thống đặt vé của Hãng B gặp trục trặc kỹ thuật, trạm điều phối trung tâm **không được phép buông xuôi và bỏ mặc hành khách tại Dubai**. Nó phải ghi nhận lỗi của bước 2 vào nhật ký đặc biệt để nhân viên hỗ trợ thủ công, đồng thời **vẫn phải tiếp tục tiến hành các bước bù trừ còn lại (bước 1)** để bảo đảm giảm thiểu tối đa thiệt hại. Đây chính là nguyên lý vận hành của `OrchestratedSaga` trong crate `ch89`.

---

## 3. Khái niệm & Cơ chế kỹ thuật chuyên sâu

### 3.1. Phân loại lỗi mô hình & Cầu dao ngắt mạch thông minh (`ErrorClass` & `CircuitBreaker`)

Trong các hệ thống kết nối với LLM Gateway, sai lầm phổ biến nhất của các kỹ sư là xử lý lỗi theo kiểu "bắt tất cả" (catch-all):

```rust,ignore
// ❌ SAI LẦM TAI HẠI: Thử lại mù quáng mọi loại lỗi
match call_llm(prompt) {
    Ok(resp) => process(resp),
    Err(e) => {
        thread::sleep(Duration::from_secs(1));
        call_llm(prompt); // Thử lại bất chấp!
    }
}
```

Nếu lỗi trả về là mã **`400 Bad Request`** (ví dụ: prompt chứa cấu trúc JSON sai lược đồ, hoặc vượt quá số lượng stop sequence tối đa cho phép của nhà cung cấp), thì dù bạn có thử lại 100 lần với cùng một tham số đầu vào, kết quả vẫn sẽ là lỗi 400! Việc thử lại mù quáng các lỗi thuộc nhóm client (`Fatal4xx`) gây ra hai hậu quả khôn lường:
1. Làm lãng phí vô ích tài nguyên CPU và băng thông mạng.
2. Nếu hệ thống có gắn cầu dao ngắt mạch (`CircuitBreaker`), việc tích lũy các lỗi 400 này sẽ làm mở cầu dao (Open), khiến toàn bộ các tác vụ khác trong hệ thống — kể cả những tác vụ hoàn toàn hợp lệ — đều bị chặn đứng oan uổng.

Crate `ch89` định nghĩa enum phân loại lỗi chính quy:

```rust
#[derive(Debug, Clone, PartialEq, Eq)]
pub enum ErrorClass {
    /// Lỗi client chết người: 400 Bad Request, 401 Unauthorized, 403 Forbidden.
    /// Tuyệt đối KHÔNG thử lại và KHÔNG mở CircuitBreaker.
    Fatal4xx {
        status_code: u16,
        message: String,
    },
    /// Vượt ngưỡng tần suất gọi 429 Too Many Requests. Cần thử lại theo backoff.
    RateLimit429 {
        retry_after_ms: u64,
    },
    /// Lỗi máy chủ tạm thời: 500 Internal Error, 502 Bad Gateway, 503 Service Unavailable.
    /// Thử lại có giới hạn và tích lũy mở CircuitBreaker.
    Transient5xx {
        status_code: u16,
        message: String,
    },
    /// Tràn cửa sổ ngữ cảnh context window của mô hình (cần cắt tỉa hoặc fallback).
    ContextOverflow {
        max_tokens: usize,
        requested_tokens: usize,
    },
}
```

Bất biến vận hành của `CircuitBreaker`:
- **Chỉ có lỗi `Transient5xx` mới làm tăng bộ đếm thất bại liên tiếp (`consecutive_failures`)**. Khi vượt ngưỡng `failure_threshold`, mạch mới chuyển từ `Closed` sang `Open`.
- `Fatal4xx`, `RateLimit429`, và `ContextOverflow` **không bao giờ** làm tăng bộ đếm mở mạch.
- Khi mạch ở trạng thái `Open`, mọi yêu cầu mới trong khoảng thời gian làm nguội (`cooldown_ticks`) đều bị từ chối ngay lập tức mà không cần gửi gói tin qua mạng.
- Khi hết thời gian làm nguội, mạch chuyển sang `HalfOpen`, cho phép đúng một yêu cầu thăm dò (probe) đi qua. Nếu thành công, mạch đóng lại (`Closed`); nếu thất bại, mạch lập tức mở lại (`Open`).

Bên cạnh đó, để giải quyết hiện tượng sấm sét bầy đàn khi hàng loạt Agent cùng thử lại sau sự cố, crate cài đặt thuật toán **Decorrelated Jitter Backoff**:

$$\text{sleep}_{i+1} = \min\left(\text{max\_delay}, \text{Uniform}(\text{base\_delay}, \text{sleep}_i \times 3)\right)$$

So với hàm mũ cố định ($2^i \times \text{base}$), độ trễ phi tương quan phân tán hoàn toàn các nhịp phát yêu cầu của các tiến trình độc lập, triệt tiêu nguy cơ cộng hưởng xung đột tại cổng API.

---

### 3.2. Bất biến "Model-visible means logged" & Khung nhị phân `TurnWal`

Một Agent không thể tự nhận là đáng tin cậy nếu lịch sử của nó có thể bị thay đổi hoặc thiếu sót so với những gì mô hình đã thực sự "thấy". Trong bài báo kỹ thuật về kiến trúc suy luận của DeepSeek và các hệ thống tác tử tiên tiến, nguyên lý này được đúc kết thành châm ngôn:

> **"Model-visible means logged" (Mọi thứ mô hình nhìn thấy đều phải được ghi lại).**

Nếu một Agent chuẩn bị gửi một tin nhắn hệ thống (System Prompt) có chèn thêm các quy tắc bảo mật tạm thời, hoặc một kết quả công cụ vừa đọc từ hệ thống tệp, quy trình BẮT BUỘC phải là:

```
1. Chuẩn bị Buffer ngữ cảnh C
2. Ghi khung WAL (TurnWal::append) chứa C xuống đĩa + tính Checksum SHA-256
3. Xác nhận fsync hoàn tất trên đĩa
4. MỚI ĐƯỢC PHÉP gửi C qua socket mạng tới API của LLM
```

Mỗi khung nhị phân trong `TurnWal` được cấu trúc như sau:

| Trường | Kích thước | Mô tả |
|---|---|---|
| `frame_seq` | 8 bytes (u64 big-endian) | Số thứ tự tăng đơn điệu của khung nhật ký |
| `kind` | 1 byte (u8) | Loại dữ liệu: 0=System, 1=User, 2=Output, 3=ToolCall, 4=ToolResult |
| `sess_len` | 2 bytes (u16 big-endian) | Độ dài chuỗi định danh phiên làm việc |
| `session_id` | N bytes UTF-8 | Định danh chuỗi của phiên |
| `payload_len`| 4 bytes (u32 big-endian) | Độ dài mảng byte nội dung |
| `payload` | M bytes | Dữ liệu ngữ cảnh thực tế |
| `checksum` | 32 bytes | Mã băm SHA-256 tính trên `[seq || sess || payload]` |

**Cơ chế Tự gọt đuôi rách (Torn-tail Truncation)**: Khi máy chủ bị mất điện đột ngột trong lúc hệ điều hành đang ghi một phần của khung nhị phân (ví dụ mới ghi được tiêu đề nhưng chưa ghi xong payload hoặc checksum), tệp nhật ký trên đĩa sẽ có một đoạn dữ liệu dở dang ở cuối (torn tail). Hàm `TurnWal::recover_from_bytes` quét tuần tự từng khung, tính lại mã băm SHA-256 và đối chiếu với 32 byte cuối khung. Nếu phát hiện khung bị cắt ngang hoặc mã băm không khớp, bộ phục hồi lập tức **cắt bỏ phần đuôi hỏng này**, giữ lại 100% các khung toàn vẹn phía trước và đưa hệ thống về trạng thái sẵn sàng tiếp tục phiên mà không gây crash hoảng loạn.

---

### 3.3. Cây phiên rẽ nhánh `SessionTree`

Trong trải nghiệm người dùng hiện đại với các trợ lý AI lập trình (như lệnh `/tree` hay tính năng thử nghiệm giải pháp), người dùng thường muốn thử nhiều cách tiếp cận khác nhau từ cùng một xuất phát điểm:
- Từ câu hỏi ban đầu (Gốc), thử Hướng A: "Dùng thuật toán quy hoạch động".
- Nếu thấy Hướng A tốn bộ nhớ, quay lại Gốc và thử Hướng B: "Dùng thuật toán tham lam hai con trỏ".

Nếu cấu trúc dữ liệu lưu trữ phiên là một mảng phẳng `Vec<Message>`, việc thử Hướng B buộc chúng ta phải xóa bỏ hoặc ghi đè các tin nhắn của Hướng A. Điều này làm mất vĩnh viễn dữ liệu lịch sử và phá hủy khả năng đối chiếu kết quả.

`SessionTree` tổ chức các lượt hội thoại dưới dạng một **Cây có con trỏ tổ tiên (Inverted Tree with Parent Pointers)**:

```
                       [Nút Gốc: Root ID=1]
                      (System: Chuyên gia Rust)
                                │
                                ▼
                       [Nút ID=2: User]
                     (Hỏi: Giải bài toán X)
                        /              \
                       /                \
                      ▼                  ▼
             [Nút ID=3: Assistant]     [Nút ID=5: Assistant]
            (Nhánh A: Quy hoạch động)  (Nhánh B: Tham lam)
                      │
                      ▼
             [Nút ID=4: User]
            (Nhánh A: Tiếp tục tối ưu)
```

Khi Agent cần ngữ cảnh để gửi tới LLM tại nhánh A (tại nút ID=4), hàm `SessionTree::replay_path(4)` lần ngược theo các con trỏ `parent_id`: $4 \to 3 \to 2 \to 1$, sau đó đảo ngược danh sách để tạo thành chuỗi tuyến tính hoàn chỉnh: $[1, 2, 3, 4]$. Nhánh B tại nút ID=5 hoàn toàn độc lập với nhánh A và không hề bị ảnh hưởng. Dữ liệu lịch sử là bất biến (immutable append-only).

---

### 3.4. Kho lũy đẳng hai pha `IdempotencyStore`

Để ngăn chặn việc lặp lại các công cụ có hiệu ứng phụ khi mạng bị chập chờn hoặc Agent phục hồi sau crash, chúng ta áp dụng cơ chế khóa lũy đẳng hai pha:

```
Yêu cầu gọi Tool với Khóa K
          │
          ▼
   Kiểm tra Khóa K trong Store?
   ├── CHƯA CÓ ──► Chèn Pending { lease_until = now + duration } ──► Thực thi Tool (ExecuteNew)
   │
   ├── ĐÃ COMMITTED ──► Trả về kết quả trong Cache (ReturnCached, KHÔNG gọi Tool lại!)
   │
   └── ĐANG PENDING:
       ├── Chưa hết hạn lease ──► Báo lỗi xung đột đang xử lý (InFlightConflict)
       └── Đã quá hạn lease (Worker cũ sập!) ──► Thu hồi lease và thực thi lại (ReclaimExpiredLease)
```

Thuật toán này bảo đảm tính chất **Chính xác một lần (Exactly-Once Semantics)** đối với góc nhìn của thế giới bên ngoài, ngay cả trong điều kiện các tiến trình Agent chết bất đắc kỳ tử.

---

### 3.5. Hộp thư ra giao dịch `TransactionalOutbox`

Khi Agent vừa cập nhật trạng thái nội bộ trong cơ sở dữ liệu, vừa cần bắn một sự kiện (event) ra Message Broker (như Kafka, RabbitMQ) hoặc webhook cho hệ thống bên ngoài, vấn đề "ghi kép" (Dual-Write Problem) xuất hiện:
- Nếu ghi DB trước, bắn Event sau: Nếu DB ghi xong nhưng máy chủ sập trước khi kịp bắn Event $\to$ Hệ thống ngoài không nhận được tin.
- Nếu bắn Event trước, ghi DB sau: Nếu Event đã bắn nhưng DB ghi lỗi $\to$ Hệ thống ngoài hành động dựa trên dữ liệu ma chưa từng tồn tại!

`TransactionalOutbox` giải quyết triệt để bài toán này:
1. Sự kiện không được bắn trực tiếp qua mạng. Nó được ghi vào danh sách Outbox **trong cùng một đơn vị công việc nguyên tử** với trạng thái nội bộ, ở trạng thái ban đầu là `Staged`.
2. Một tiến trình nền độc lập (`Outbox Sweeper`) liên tục quét các sự kiện `Staged`, bắn chúng ra hệ thống ngoài, và chỉ khi nhận được xác nhận thành công (ACK), nó mới đánh dấu sự kiện là `Dispatched`.
3. Nếu tiến trình sập giữa chừng, khi khởi động lại nó quét lại các sự kiện `Staged` và gửi lại. Hệ thống bên ngoài chỉ cần áp dụng cơ chế khử trùng lặp (Deduplication) dựa vào `event_id`.

---

### 3.6. Điều phối bù trừ ngược `OrchestratedSaga`

Trong kiến trúc Agent thực hiện chuỗi tác vụ phức tạp (ví dụ: Tạo tài khoản $\to$ Phân bổ tài nguyên máy chủ $\to$ Cấu hình DNS $\to$ Trừ tiền thẻ tín dụng), chúng ta không thể sử dụng giao thức 2-Phase Commit (2PC) vì các API bên ngoài (Stripe, AWS, Cloudflare) không hỗ trợ giữ khóa tài nguyên phân tán.

Mô hình Saga điều phối trung tâm (`OrchestratedSaga`) giải quyết vấn đề bằng cách định nghĩa cho mỗi bước một cặp hàm:
- **Hành động tiến (Forward Action)**: Thực hiện công việc bình thường.
- **Hành động bù trừ (Compensating Action)**: Hoàn tác lại kết quả của hành động tiến nếu bước sau đó gặp sự cố.

Khi bước $k$ thất bại, bộ điều phối kích hoạt vòng lặp hoàn tác ngược từ bước $k-1$ về bước 1. Đặc biệt, bộ điều phối của `ch89` bảo đảm tính chất phục hồi cao nhất: **Nếu hành động bù trừ của bước $j$ gặp lỗi, hệ thống không được dừng vòng lặp, mà phải ghi nhận lỗi chẩn đoán và tiếp tục bù trừ các bước $j-1, \dots, 1$**.

---

### 3.7. Khóa tương tranh lạc quan CAS & Thẻ rào chắn `FencingToken`

Trong môi trường có nhiều Agent hoặc tiến trình chạy song song:
1. **`OptimisticFileVersion`**: Khi Agent sửa đổi một tệp mã nguồn, nó thực hiện thao tác Kiểm tra và Hoán đổi (Compare-And-Swap - CAS). Thay vì chỉ kiểm tra số phiên bản `version: u64` đơn thuần (vốn dễ bị lỗi ABA nếu có tiến trình rollback phiên bản), hệ thống kiểm tra đồng thời cả cặp `(version: u64, content_hash: Sha256Digest)`. Nếu nội dung tệp trên đĩa đã bị một tác tử khác thay đổi dù chỉ 1 byte, thao tác CAS sẽ từ chối ngay lập tức, ngăn ngừa hoàn toàn việc ghi đè mất mã nguồn.
2. **`FencingToken`**: Khi một tác tử A được cấp quyền xử lý tài nguyên trong một khoảng thời gian (lease), nếu tác tử A bị hiện tượng dừng thế giới (Stop-The-World GC) hoặc treo mạng kéo dài khiến hợp đồng thuê hết hạn, hệ thống điều phối sẽ cấp quyền cho tác tử B với số hiệu kỷ nguyên mới `epoch = 2`. Sau đó, nếu tác tử A bất ngờ "tỉnh lại" và cố tình ghi đè kết quả cũ với thẻ `epoch = 1`, bộ kiểm soát rào chắn (`FencingController`) sẽ chặn đứng tác tử A ngay tại cửa ngõ vì $1 < 2$.

---

### 3.8. Ngăn xếp 3 Giao thức Tác tử: MCP + A2A + ANP

Để Agent có thể tương tác chuẩn hóa với toàn bộ hệ sinh thái phần mềm toàn cầu, crate `ch89` hiện thực hóa ngăn xếp 3 tầng giao thức:

```
┌─────────────────────────────────────────────────────────────┐
│ TẦNG 3: ANP (Agent Network Protocol Discovery & Routing)   │
│ Khám phá mạng lưới, công bố năng lực, định tuyến tải ít nhất│
└──────────────────────────────┬──────────────────────────────┘
                               │
┌──────────────────────────────▼──────────────────────────────┐
│ TẦNG 2: A2A (Agent-to-Agent Communication Protocol)         │
│ Trao đổi ủy quyền tác vụ ngang hàng: Submitted->Working->...│
└──────────────────────────────┬──────────────────────────────┘
                               │
┌──────────────────────────────▼──────────────────────────────┐
│ TẦNG 1: MCP (Model Context Protocol qua JSON-RPC 2.0 Router)│
│ Chuẩn hóa công cụ nội bộ: tools/list, tools/call, ping       │
└─────────────────────────────────────────────────────────────┘
```

1. **MCP (Model Context Protocol)**: Giao thức mở do Anthropic khởi xướng, chuẩn hóa việc mô hình LLM tương tác với máy chủ công cụ cục bộ qua JSON-RPC 2.0. Router trong crate hỗ trợ các phương thức tiêu chuẩn: `tools/list` để mô hình tự khám phá các công cụ hiện có, `tools/call` để truyền tham số thực thi, và `ping` để kiểm tra độ trễ liveness.
2. **A2A (Agent-to-Agent)**: Giao thức trao đổi công việc giữa các Agent độc lập. Mỗi Agent sở hữu một thẻ định danh năng lực `AgentCard` (chứa danh sách capabilities và chi phí `cost_tier`). Tác vụ được quản lý qua máy trạng thái nghiêm ngặt: `Submitted` $\to$ `Working` $\to$ `Completed` hoặc `Failed`.
3. **ANP (Agent Network Protocol)**: Giao thức khám phá và định tuyến phân tán trong mạng lưới nhiều Agent. Sổ đăng bạ `AnpDiscoveryRegistry` theo dõi tải thực tế đang xử lý (`in_flight_load`) và cấp bậc chi phí của từng Agent, từ đó định tuyến các yêu cầu mới tới Agent có tải nhỏ nhất và chi phí kinh tế nhất (Least-Loaded Routing).

---

## 4. Mã nguồn minh họa thực chiến

Toàn bộ mã nguồn dưới đây được lưu trữ tại `code/ch89/src/main.rs`. Mã nguồn tuân thủ 100% chuẩn Rust 2024 (`edition = "2024"`), sử dụng cấu trúc struct có trường định danh có tên (`named fields`), không phụ thuộc bất kỳ thư viện ngoài nào trong `Cargo.toml`, vượt qua toàn bộ các cờ biên dịch khắt khe `-D warnings -F non-ascii-idents` và `cargo clippy --all-targets -- -D warnings`, kèm 32 bài kiểm thử tự động toàn diện.

```rust
//! # Chương 89: Độ bền Phục hồi Sự cố — Cổng Mô hình, Nhật ký WAL, Cây Phiên, Outbox, Saga & Giao thức MCP + A2A + ANP
//!
//! Thư viện hoàn chỉnh 100% chuẩn Rust 2024 thuần `std` giải quyết các bài toán sống còn
//! trong kỹ nghệ xây dựng Agent tự hành chịu lỗi (Crash-Resilient Autonomous Agent):
//! 1. `LlmGateway`: Phân loại lỗi `Fatal4xx` vs `RateLimit429` vs `Transient5xx`,
//!    cầu dao `CircuitBreaker` không bao giờ mở oan vì lỗi 4xx client,
//!    thuật toán giật lùi phân tán `DecorrelatedJitterBackoff`, và FinOps `stable_fraction`.
//! 2. `TurnWal`: Nhật ký ghi trước cưỡng chế bất biến cốt lõi **"Model-visible means logged"**,
//!    khung nhị phân toàn vẹn SHA-256, tự gọt đuôi rách (torn-tail truncation) khi sập nguồn.
//! 3. `SessionTree`: Cây hội thoại rẽ nhánh `/tree` không bao giờ ghi đè lịch sử cha,
//!    hỗ trợ phân nhánh độc lập và phát lại tuyến tính từng byte.
//! 4. `IdempotencyStore`: Kho lũy đẳng bền vững hai pha `Pending { lease_until } -> Committed { output_hash }`.
//! 5. `TransactionalOutbox`: Bộ đệm xuất giao dịch bảo đảm tính chất "at-least-once" cho sự kiện liên hệ thống.
//! 6. `OrchestratedSaga`: Bộ điều phối bù trừ ngược từ đĩa, bảo đảm tiếp tục hoàn tác kể cả khi có bước đơn lẻ lỗi.
//! 7. `OptimisticFileVersion` & `FencingToken`: Kiểm soát tương tranh lạc quan CAS `(version, sha256)`
//!    và rào chắn đơn điệu ngăn chặn hoàn toàn tác tử cũ (zombie agent) ghi đè.
//! 8. Ngăn xếp 3 giao thức kết nối: `McpJsonRpcRouter` (Model Context Protocol) +
//!    `A2aAgentNode` (Agent-to-Agent với `AgentCard` & `A2aTaskState`) +
//!    `AnpDiscoveryRegistry` (Agent Network Protocol định tuyến cân bằng tải).

use std::collections::{BTreeMap, HashMap};
use std::fmt;

// ============================================================================
// 0. BẢNG MÃ BĂM MẬT MÃ VÀ PHÁT SINH SỐ NGẪU NHIÊN THUẦN STD (SHA-256 & SPLITMIX64)
// ============================================================================

/// Giá trị băm SHA-256 256-bit bất biến.
#[derive(Debug, Clone, Copy, PartialEq, Eq, PartialOrd, Ord, Hash)]
pub struct Sha256Digest {
    pub bytes: [u8; 32],
}

impl Sha256Digest {
    /// Biểu diễn băm dưới dạng chuỗi hexa 64 ký tự thường.
    pub fn to_hex_string(&self) -> String {
        let mut s = String::with_capacity(64);
        for b in &self.bytes {
            s.push_str(&format!("{b:02x}"));
        }
        s
    }
}

impl fmt::Display for Sha256Digest {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        write!(f, "{}", self.to_hex_string())
    }
}

/// Bộ tính toán SHA-256 thuần Rust `std` tuân chuẩn FIPS 180-4.
#[derive(Debug, Clone)]
pub struct PureSha256 {
    state: [u32; 8],
    buffer: Vec<u8>,
    total_len_bytes: u64,
}

impl Default for PureSha256 {
    fn default() -> Self {
        Self::new()
    }
}

impl PureSha256 {
    const K: [u32; 64] = [
        0x428a2f98, 0x71374491, 0xb5c0fbcf, 0xe9b5dba5, 0x3956c25b, 0x59f111f1, 0x923f82a4,
        0xab1c5ed5, 0xd807aa98, 0x12835b01, 0x243185be, 0x550c7dc3, 0x72be5d74, 0x80deb1fe,
        0x9bdc06a7, 0xc19bf174, 0xe49b69c1, 0xefbe4786, 0x0fc19dc6, 0x240ca1cc, 0x2de92c6f,
        0x4a7484aa, 0x5cb0a9dc, 0x76f988da, 0x983e5152, 0xa831c66d, 0xb00327c8, 0xbf597fc7,
        0xc6e00bf3, 0xd5a79147, 0x06ca6351, 0x14292967, 0x27b70a85, 0x2e1b2138, 0x4d2c6dfc,
        0x53380d13, 0x650a7354, 0x766a0abb, 0x81c2c92e, 0x92722c85, 0xa2bfe8a1, 0xa81a664b,
        0xc24b8b70, 0xc76c51a3, 0xd192e819, 0xd6990624, 0xf40e3585, 0x106aa070, 0x19a4c116,
        0x1e376c08, 0x2748774c, 0x34b0bcb5, 0x391c0cb3, 0x4ed8aa4a, 0x5b9cca4f, 0x682e6ff3,
        0x748f82ee, 0x78a5636f, 0x84c87814, 0x8cc70208, 0x90befffa, 0xa4506ceb, 0xbef9a3f7,
        0xc67178f2,
    ];

    pub fn new() -> Self {
        Self {
            state: [
                0x6a09e667, 0xbb67ae85, 0x3c6ef372, 0xa54ff53a, 0x510e527f, 0x9b05688c,
                0x1f83d9ab, 0x5be0cd19,
            ],
            buffer: Vec::with_capacity(64),
            total_len_bytes: 0,
        }
    }

    pub fn update(&mut self, data: &[u8]) {
        self.total_len_bytes += data.len() as u64;
        let mut idx = 0;
        while idx < data.len() {
            let space = 64 - self.buffer.len();
            let chunk_len = space.min(data.len() - idx);
            self.buffer.extend_from_slice(&data[idx..idx + chunk_len]);
            idx += chunk_len;
            if self.buffer.len() == 64 {
                let block: [u8; 64] = self.buffer.as_slice().try_into().expect("khối 64 byte");
                Self::process_block(&mut self.state, &block);
                self.buffer.clear();
            }
        }
    }

    pub fn finalize(mut self) -> Sha256Digest {
        let bit_len = self.total_len_bytes * 8;
        self.buffer.push(0x80);
        while (self.buffer.len() % 64) != 56 {
            self.buffer.push(0x00);
        }
        self.buffer.extend_from_slice(&bit_len.to_be_bytes());

        let (chunks, _) = self.buffer.as_chunks::<64>();
        for block in chunks {
            Self::process_block(&mut self.state, block);
        }

        let mut out = [0u8; 32];
        for (i, word) in self.state.iter().enumerate() {
            out[i * 4..(i + 1) * 4].copy_from_slice(&word.to_be_bytes());
        }
        Sha256Digest { bytes: out }
    }

    pub fn digest(data: &[u8]) -> Sha256Digest {
        let mut hasher = Self::new();
        hasher.update(data);
        hasher.finalize()
    }

    fn process_block(state: &mut [u32; 8], block: &[u8; 64]) {
        let mut w = [0u32; 64];
        for (i, w_item) in w.iter_mut().take(16).enumerate() {
            *w_item = u32::from_be_bytes([
                block[i * 4],
                block[i * 4 + 1],
                block[i * 4 + 2],
                block[i * 4 + 3],
            ]);
        }
        for i in 16..64 {
            let s0 = w[i - 15].rotate_right(7) ^ w[i - 15].rotate_right(18) ^ (w[i - 15] >> 3);
            let s1 = w[i - 2].rotate_right(17) ^ w[i - 2].rotate_right(19) ^ (w[i - 2] >> 10);
            w[i] = w[i - 16]
                .wrapping_add(s0)
                .wrapping_add(w[i - 7])
                .wrapping_add(s1);
        }

        let mut a = state[0];
        let mut b = state[1];
        let mut c = state[2];
        let mut d = state[3];
        let mut e = state[4];
        let mut f = state[5];
        let mut g = state[6];
        let mut h = state[7];

        for (i, &w_i) in w.iter().enumerate() {
            let s1 = e.rotate_right(6) ^ e.rotate_right(11) ^ e.rotate_right(25);
            let ch = (e & f) ^ ((!e) & g);
            let temp1 = h
                .wrapping_add(s1)
                .wrapping_add(ch)
                .wrapping_add(Self::K[i])
                .wrapping_add(w_i);
            let s0 = a.rotate_right(2) ^ a.rotate_right(13) ^ a.rotate_right(22);
            let maj = (a & b) ^ (a & c) ^ (b & c);
            let temp2 = s0.wrapping_add(maj);

            h = g;
            g = f;
            f = e;
            e = d.wrapping_add(temp1);
            d = c;
            c = b;
            b = a;
            a = temp1.wrapping_add(temp2);
        }

        state[0] = state[0].wrapping_add(a);
        state[1] = state[1].wrapping_add(b);
        state[2] = state[2].wrapping_add(c);
        state[3] = state[3].wrapping_add(d);
        state[4] = state[4].wrapping_add(e);
        state[5] = state[5].wrapping_add(f);
        state[6] = state[6].wrapping_add(g);
        state[7] = state[7].wrapping_add(h);
    }
}

/// Bộ phát số ngẫu nhiên giả tất định SplitMix64 64-bit.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct SplitMix64 {
    pub state: u64,
}

impl SplitMix64 {
    pub fn new(seed: u64) -> Self {
        Self { state: seed }
    }

    pub fn next_u64(&mut self) -> u64 {
        self.state = self.state.wrapping_add(0x9e3779b97f4a7c15);
        let mut z = self.state;
        z = (z ^ (z >> 30)).wrapping_mul(0xbf58476d1ce4e5b9);
        z = (z ^ (z >> 27)).wrapping_mul(0x94d049bb133111eb);
        z ^ (z >> 31)
    }

    /// Trả về số thực ngẫu nhiên trong khoảng `[0.0, 1.0)`.
    pub fn next_f64(&mut self) -> f64 {
        let v = self.next_u64() >> 11;
        (v as f64) / ((1u64 << 53) as f64)
    }

    /// Sinh số nguyên ngẫu nhiên đều trong đoạn `[min, max]`.
    pub fn range_u64(&mut self, min: u64, max: u64) -> u64 {
        if min >= max {
            return min;
        }
        let span = max - min + 1;
        min + (self.next_u64() % span)
    }
}

// ============================================================================
// 1. CỔNG MÔ HÌNH (LLM GATEWAY, ERROR CLASSIFICATION, CIRCUIT BREAKER & FINOPS)
// ============================================================================

/// Trạng thái cầu dao ngắt mạch (Circuit Breaker).
#[derive(Debug, Clone, PartialEq, Eq)]
pub enum CircuitBreakerState {
    /// Bình thường: mạch đóng, cho phép mọi yêu cầu.
    Closed { consecutive_failures: u32 },
    /// Mở ngắt mạch: từ chối ngay lập tức để bảo vệ hạ tầng.
    Open {
        opened_at_tick: u64,
        cooldown_ticks: u64,
    },
    /// Thử nghiệm: mở một phần, gửi một yêu cầu thăm dò (probe).
    HalfOpen { probe_active: bool },
}

/// Cầu dao ngắt mạch thông minh: không bao giờ mở mạch oan vì lỗi Fatal4xx.
#[derive(Debug, Clone)]
pub struct CircuitBreaker {
    pub failure_threshold: u32,
    pub cooldown_ticks: u64,
    pub state: CircuitBreakerState,
}

impl CircuitBreaker {
    pub fn new(failure_threshold: u32, cooldown_ticks: u64) -> Self {
        Self {
            failure_threshold,
            cooldown_ticks,
            state: CircuitBreakerState::Closed {
                consecutive_failures: 0,
            },
        }
    }

    /// Kiểm tra xem tại thời điểm `current_tick` có được phép phát yêu cầu không.
    pub fn can_execute(&mut self, current_tick: u64) -> bool {
        match &mut self.state {
            CircuitBreakerState::Closed { .. } => true,
            CircuitBreakerState::Open {
                opened_at_tick,
                cooldown_ticks,
            } => {
                if current_tick >= *opened_at_tick + *cooldown_ticks {
                    self.state = CircuitBreakerState::HalfOpen {
                        probe_active: true,
                    };
                    true
                } else {
                    false
                }
            }
            CircuitBreakerState::HalfOpen { probe_active } => {
                if !*probe_active {
                    *probe_active = true;
                    true
                } else {
                    false
                }
            }
        }
    }

    /// Ghi nhận phản hồi thành công từ mô hình.
    pub fn record_success(&mut self) {
        match self.state {
            CircuitBreakerState::HalfOpen { .. } | CircuitBreakerState::Closed { .. } => {
                self.state = CircuitBreakerState::Closed {
                    consecutive_failures: 0,
                };
            }
            CircuitBreakerState::Open { .. } => {}
        }
    }

    /// Ghi nhận lỗi: chỉ lỗi `Transient5xx` mới làm tăng bộ đếm mở mạch!
    /// `Fatal4xx`, `RateLimit429`, và `ContextOverflow` KHÔNG BAO GIỜ mở mạch.
    pub fn record_failure(&mut self, error: &ErrorClass, current_tick: u64) -> bool {
        let is_server_crash = matches!(error, ErrorClass::Transient5xx { .. });
        if !is_server_crash {
            return false;
        }

        match &mut self.state {
            CircuitBreakerState::Closed {
                consecutive_failures,
            } => {
                *consecutive_failures += 1;
                if *consecutive_failures >= self.failure_threshold {
                    self.state = CircuitBreakerState::Open {
                        opened_at_tick: current_tick,
                        cooldown_ticks: self.cooldown_ticks,
                    };
                    return true;
                }
            }
            CircuitBreakerState::HalfOpen { .. } => {
                self.state = CircuitBreakerState::Open {
                    opened_at_tick: current_tick,
                    cooldown_ticks: self.cooldown_ticks,
                };
                return true;
            }
            CircuitBreakerState::Open { .. } => {}
        }
        false
    }
}

/// Thuật toán độ trễ giật lùi phi tương quan (Decorrelated Jitter Backoff).
#[derive(Debug, Clone)]
pub struct DecorrelatedJitterBackoff {
    pub base_delay_ms: u64,
    pub max_delay_ms: u64,
    pub rng: SplitMix64,
}

impl DecorrelatedJitterBackoff {
    pub fn new(base_delay_ms: u64, max_delay_ms: u64, seed: u64) -> Self {
        Self {
            base_delay_ms,
            max_delay_ms,
            rng: SplitMix64::new(seed),
        }
    }

    pub fn next_delay(&mut self, prev_delay_ms: u64) -> u64 {
        let ceiling = prev_delay_ms.saturating_mul(3).max(self.base_delay_ms);
        let randomized = self.rng.range_u64(self.base_delay_ms, ceiling);
        randomized.min(self.max_delay_ms)
    }
}

/// Yêu cầu gửi tới Cổng mô hình LLM.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct LlmRequest {
    pub prompt: String,
    pub estimated_tokens: usize,
    pub idempotency_key: String,
    pub allow_fallback: bool,
}

/// Phản hồi nhận được từ Cổng mô hình LLM.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct LlmResponse {
    pub content: String,
    pub model_used: String,
    pub tokens_used: usize,
    pub cost_cents: u64,
}

/// Bộ mô phỏng nhà cung cấp LLM để kiểm thử tất định 100%.
pub trait LlmProviderMock {
    fn call(&self, model: &str, req: &LlmRequest) -> Result<LlmResponse, ErrorClass>;
}

/// Cổng mô hình `LlmGateway` tích hợp quản trị FinOps, Circuit Breaker và Fallback.
pub struct LlmGateway<P: LlmProviderMock> {
    pub provider: P,
    pub primary_model: String,
    pub secondary_model: String,
    pub local_model: String,
    pub circuit_breaker: CircuitBreaker,
    pub backoff: DecorrelatedJitterBackoff,
    pub total_budget_cents: u64,
    pub spent_cents: u64,
    /// Tỷ lệ phần trăm định tuyến vào mô hình ổn định (FinOps `stable_fraction`: 0..=100).
    pub stable_fraction: u32,
    pub rng: SplitMix64,
}

impl<P: LlmProviderMock> LlmGateway<P> {
    pub fn new(
        provider: P,
        primary_model: String,
        secondary_model: String,
        local_model: String,
        total_budget_cents: u64,
        stable_fraction: u32,
        seed: u64,
    ) -> Self {
        Self {
            provider,
            primary_model,
            secondary_model,
            local_model,
            circuit_breaker: CircuitBreaker::new(3, 100),
            backoff: DecorrelatedJitterBackoff::new(50, 2000, seed),
            total_budget_cents,
            spent_cents: 0,
            stable_fraction: stable_fraction.min(100),
            rng: SplitMix64::new(seed.wrapping_add(101)),
        }
    }

    /// Thực hiện gọi mô hình kèm chiến lược chuyển bậc dự phòng (fallback) tự động.
    pub fn execute(&mut self, req: &LlmRequest, current_tick: u64) -> Result<LlmResponse, String> {
        if self.spent_cents >= self.total_budget_cents {
            return Err("Ngân sách FinOps đã cạn kiệt (Budget Exhausted)".to_string());
        }

        let dice = self.rng.range_u64(1, 100) as u32;
        let preferred_model = if dice <= self.stable_fraction {
            &self.primary_model
        } else {
            &self.secondary_model
        };

        if self.circuit_breaker.can_execute(current_tick) {
            match self.provider.call(preferred_model, req) {
                Ok(resp) => {
                    self.circuit_breaker.record_success();
                    self.spent_cents = self.spent_cents.saturating_add(resp.cost_cents);
                    return Ok(resp);
                }
                Err(err) => {
                    if matches!(err, ErrorClass::Fatal4xx { .. }) {
                        return Err(format!("Lỗi client nghiêm trọng không thử lại: {err}"));
                    }
                    self.circuit_breaker.record_failure(&err, current_tick);
                    if !req.allow_fallback {
                        return Err(format!("Gọi mô hình thất bại và cấm fallback: {err}"));
                    }
                }
            }
        }

        if preferred_model != &self.secondary_model {
            match self.provider.call(&self.secondary_model, req) {
                Ok(resp) => {
                    self.spent_cents = self.spent_cents.saturating_add(resp.cost_cents);
                    return Ok(resp);
                }
                Err(err) => {
                    if matches!(err, ErrorClass::Fatal4xx { .. }) {
                        return Err(format!("Lỗi client nghiêm trọng tại secondary: {err}"));
                    }
                }
            }
        }

        match self.provider.call(&self.local_model, req) {
            Ok(resp) => {
                self.spent_cents = self.spent_cents.saturating_add(resp.cost_cents);
                Ok(resp)
            }
            Err(err) => Err(format!("Tất cả các bậc mô hình đã kiệt quệ: {err}")),
        }
    }
}

// ============================================================================
// 2. NHẬT KÝ GHI TRƯỚC (TURN WAL: "MODEL-VISIBLE MEANS LOGGED")
// ============================================================================

/// Loại mục nhật ký trong một lượt hội thoại của Agent.
#[derive(Debug, Clone, PartialEq, Eq)]
pub enum WalPayloadKind {
    SystemPrompt,
    UserMessage,
    ModelInferenceOutput,
    ToolCallExecution,
    ToolResultReturned,
}

/// Khung nhị phân của một mục nhật ký WAL kèm mã kiểm tra SHA-256.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct WalFrame {
    pub frame_seq: u64,
    pub session_id: String,
    pub kind: WalPayloadKind,
    pub payload_bytes: Vec<u8>,
    pub checksum: Sha256Digest,
}

/// Bất biến cốt lõi: `"Model-visible means logged"`.
pub struct TurnWal {
    pub frames: Vec<WalFrame>,
    pub next_seq: u64,
}

impl Default for TurnWal {
    fn default() -> Self {
        Self::new()
    }
}

impl TurnWal {
    pub fn new() -> Self {
        Self {
            frames: Vec::new(),
            next_seq: 1,
        }
    }

    pub fn append(&mut self, session_id: &str, kind: WalPayloadKind, payload: &[u8]) -> &WalFrame {
        let seq = self.next_seq;
        self.next_seq += 1;

        let mut hash_input = Vec::new();
        hash_input.extend_from_slice(&seq.to_be_bytes());
        hash_input.extend_from_slice(session_id.as_bytes());
        hash_input.extend_from_slice(payload);
        let checksum = PureSha256::digest(&hash_input);

        let frame = WalFrame {
            frame_seq: seq,
            session_id: session_id.to_string(),
            kind,
            payload_bytes: payload.to_vec(),
            checksum,
        };
        self.frames.push(frame);
        self.frames.last().expect("vừa push")
    }

    pub fn verify_model_visible_logged(&self, session_id: &str, expected_content: &[u8]) -> bool {
        self.frames.iter().any(|f| {
            f.session_id == session_id
                && (f.payload_bytes == expected_content
                    || f.payload_bytes
                        .windows(expected_content.len())
                        .any(|window| window == expected_content))
        })
    }

    pub fn serialize_to_bytes(&self) -> Vec<u8> {
        let mut buf = Vec::new();
        for f in &self.frames {
            buf.extend_from_slice(&f.frame_seq.to_be_bytes());
            let kind_byte = match f.kind {
                WalPayloadKind::SystemPrompt => 0u8,
                WalPayloadKind::UserMessage => 1u8,
                WalPayloadKind::ModelInferenceOutput => 2u8,
                WalPayloadKind::ToolCallExecution => 3u8,
                WalPayloadKind::ToolResultReturned => 4u8,
            };
            buf.push(kind_byte);
            let sess_bytes = f.session_id.as_bytes();
            buf.extend_from_slice(&(sess_bytes.len() as u16).to_be_bytes());
            buf.extend_from_slice(sess_bytes);
            buf.extend_from_slice(&(f.payload_bytes.len() as u32).to_be_bytes());
            buf.extend_from_slice(&f.payload_bytes);
            buf.extend_from_slice(&f.checksum.bytes);
        }
        buf
    }

    pub fn recover_from_bytes(bytes: &[u8]) -> (Self, usize) {
        let mut wal = Self::new();
        let mut offset = 0;
        let mut recovered_count = 0;

        while offset < bytes.len() {
            if offset + 47 > bytes.len() {
                break;
            }

            let seq = u64::from_be_bytes(
                bytes[offset..offset + 8]
                    .try_into()
                    .expect("đọc u64 seq"),
            );
            let kind = match bytes[offset + 8] {
                0 => WalPayloadKind::SystemPrompt,
                1 => WalPayloadKind::UserMessage,
                2 => WalPayloadKind::ModelInferenceOutput,
                3 => WalPayloadKind::ToolCallExecution,
                4 => WalPayloadKind::ToolResultReturned,
                _ => break,
            };
            let sess_len = u16::from_be_bytes(
                bytes[offset + 9..offset + 11]
                    .try_into()
                    .expect("đọc u16 sess_len"),
            ) as usize;

            let after_sess = offset + 11 + sess_len;
            if after_sess + 4 > bytes.len() {
                break;
            }

            let sess_id = match String::from_utf8(bytes[offset + 11..after_sess].to_vec()) {
                Ok(s) => s,
                Err(_) => break,
            };

            let payload_len = u32::from_be_bytes(
                bytes[after_sess..after_sess + 4]
                    .try_into()
                    .expect("đọc u32 payload_len"),
            ) as usize;
            let after_payload = after_sess + 4 + payload_len;
            if after_payload + 32 > bytes.len() {
                break;
            }

            let payload = &bytes[after_sess + 4..after_payload];
            let expected_checksum: [u8; 32] = bytes[after_payload..after_payload + 32]
                .try_into()
                .expect("đọc 32 byte sha");

            let mut hash_input = Vec::new();
            hash_input.extend_from_slice(&seq.to_be_bytes());
            hash_input.extend_from_slice(sess_id.as_bytes());
            hash_input.extend_from_slice(payload);
            let computed_checksum = PureSha256::digest(&hash_input);

            if computed_checksum.bytes != expected_checksum {
                break;
            }

            wal.frames.push(WalFrame {
                frame_seq: seq,
                session_id: sess_id,
                kind,
                payload_bytes: payload.to_vec(),
                checksum: Sha256Digest {
                    bytes: expected_checksum,
                },
            });
            wal.next_seq = seq + 1;
            recovered_count += 1;
            offset = after_payload + 32;
        }

        (wal, recovered_count)
    }
}

// ============================================================================
// 3. CÂY PHIÊN RẼ NHÁNH (SESSION TREE & TREE-REPLAY)
// ============================================================================

pub type SessionNodeId = u64;

#[derive(Debug, Clone, PartialEq, Eq)]
pub enum SessionRole {
    System,
    User,
    Assistant,
    Tool,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct SessionNode {
    pub node_id: SessionNodeId,
    pub parent_id: Option<SessionNodeId>,
    pub role: SessionRole,
    pub content: String,
    pub metadata: BTreeMap<String, String>,
}

#[derive(Debug, Clone)]
pub struct SessionTree {
    pub nodes: HashMap<SessionNodeId, SessionNode>,
    pub children: HashMap<SessionNodeId, Vec<SessionNodeId>>,
    pub roots: Vec<SessionNodeId>,
    pub next_node_id: SessionNodeId,
}

impl Default for SessionTree {
    fn default() -> Self {
        Self::new()
    }
}

impl SessionTree {
    pub fn new() -> Self {
        Self {
            nodes: HashMap::new(),
            children: HashMap::new(),
            roots: Vec::new(),
            next_node_id: 1,
        }
    }

    pub fn create_root(&mut self, role: SessionRole, content: &str) -> SessionNodeId {
        let id = self.next_node_id;
        self.next_node_id += 1;

        let node = SessionNode {
            node_id: id,
            parent_id: None,
            role,
            content: content.to_string(),
            metadata: BTreeMap::new(),
        };
        self.nodes.insert(id, node);
        self.roots.push(id);
        id
    }

    pub fn fork_branch(
        &mut self,
        parent_id: SessionNodeId,
        role: SessionRole,
        content: &str,
    ) -> Result<SessionNodeId, String> {
        if !self.nodes.contains_key(&parent_id) {
            return Err(format!("Nút cha {parent_id} không tồn tại"));
        }

        let id = self.next_node_id;
        self.next_node_id += 1;

        let node = SessionNode {
            node_id: id,
            parent_id: Some(parent_id),
            role,
            content: content.to_string(),
            metadata: BTreeMap::new(),
        };
        self.nodes.insert(id, node);
        self.children.entry(parent_id).or_default().push(id);
        Ok(id)
    }

    pub fn replay_path(&self, leaf_id: SessionNodeId) -> Result<Vec<SessionNode>, String> {
        let mut path = Vec::new();
        let mut current_id = Some(leaf_id);

        while let Some(id) = current_id {
            let node = self
                .nodes
                .get(&id)
                .ok_or_else(|| format!("Đứt gãy con trỏ tổ tiên tại nút {id}"))?;
            path.push(node.clone());
            current_id = node.parent_id;
        }

        path.reverse();
        Ok(path)
    }

    pub fn list_children(&self, parent_id: SessionNodeId) -> Vec<SessionNodeId> {
        self.children.get(&parent_id).cloned().unwrap_or_default()
    }
}

// ============================================================================
// 4. KHO LŨY ĐẲNG HAI PHA (TWO-PHASE IDEMPOTENCY STORE)
// ============================================================================

#[derive(Debug, Clone, PartialEq, Eq)]
pub enum IdempotencyPhase {
    Pending {
        lease_until_tick: u64,
        request_hash: Sha256Digest,
    },
    Committed {
        output_hash: Sha256Digest,
        response_payload: Vec<u8>,
    },
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub enum IdempotencyAction {
    ExecuteNew,
    ReturnCached { response_payload: Vec<u8> },
    InFlightConflict,
    ReclaimExpiredLease,
}

#[derive(Debug, Clone)]
pub struct IdempotencyStore {
    pub records: HashMap<String, IdempotencyPhase>,
}

impl Default for IdempotencyStore {
    fn default() -> Self {
        Self::new()
    }
}

impl IdempotencyStore {
    pub fn new() -> Self {
        Self {
            records: HashMap::new(),
        }
    }

    pub fn begin_execution(
        &mut self,
        key: &str,
        request_hash: Sha256Digest,
        current_tick: u64,
        lease_duration: u64,
    ) -> IdempotencyAction {
        if let Some(phase) = self.records.get_mut(key) {
            match phase {
                IdempotencyPhase::Committed {
                    response_payload, ..
                } => IdempotencyAction::ReturnCached {
                    response_payload: response_payload.clone(),
                },
                IdempotencyPhase::Pending {
                    lease_until_tick,
                    request_hash: old_hash,
                } => {
                    if current_tick < *lease_until_tick {
                        IdempotencyAction::InFlightConflict
                    } else {
                        *lease_until_tick = current_tick + lease_duration;
                        *old_hash = request_hash;
                        IdempotencyAction::ReclaimExpiredLease
                    }
                }
            }
        } else {
            self.records.insert(
                key.to_string(),
                IdempotencyPhase::Pending {
                    lease_until_tick: current_tick + lease_duration,
                    request_hash,
                },
            );
            IdempotencyAction::ExecuteNew
        }
    }

    pub fn commit_execution(&mut self, key: &str, response_payload: &[u8]) -> Result<(), String> {
        let output_hash = PureSha256::digest(response_payload);
        if let Some(record) = self.records.get_mut(key) {
            *record = IdempotencyPhase::Committed {
                output_hash,
                response_payload: response_payload.to_vec(),
            };
            Ok(())
        } else {
            Err(format!("Khóa {key} không tồn tại để commit"))
        }
    }
}

// ============================================================================
// 5. HỘP THƯ RA GIAO DỊCH (TRANSACTIONAL OUTBOX)
// ============================================================================

#[derive(Debug, Clone, PartialEq, Eq)]
pub enum OutboxStatus {
    Staged,
    Dispatched { dispatched_at_tick: u64 },
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct OutboxEvent {
    pub event_id: u64,
    pub topic: String,
    pub payload: Vec<u8>,
    pub status: OutboxStatus,
    pub created_at_tick: u64,
}

#[derive(Debug, Clone)]
pub struct TransactionalOutbox {
    pub events: Vec<OutboxEvent>,
    pub next_event_id: u64,
}

impl Default for TransactionalOutbox {
    fn default() -> Self {
        Self::new()
    }
}

impl TransactionalOutbox {
    pub fn new() -> Self {
        Self {
            events: Vec::new(),
            next_event_id: 1,
        }
    }

    pub fn stage_event(&mut self, topic: &str, payload: &[u8], current_tick: u64) -> u64 {
        let id = self.next_event_id;
        self.next_event_id += 1;
        self.events.push(OutboxEvent {
            event_id: id,
            topic: topic.to_string(),
            payload: payload.to_vec(),
            status: OutboxStatus::Staged,
            created_at_tick: current_tick,
        });
        id
    }

    pub fn sweep_pending(&self, limit: usize) -> Vec<OutboxEvent> {
        self.events
            .iter()
            .filter(|e| matches!(e.status, OutboxStatus::Staged))
            .take(limit)
            .cloned()
            .collect()
    }

    pub fn mark_dispatched(&mut self, event_id: u64, current_tick: u64) -> bool {
        if let Some(ev) = self.events.iter_mut().find(|e| e.event_id == event_id) {
            ev.status = OutboxStatus::Dispatched {
                dispatched_at_tick: current_tick,
            };
            true
        } else {
            false
        }
    }
}

// ============================================================================
// 6. ĐIỀU PHỐI SAGA BÙ TRỪ TỰ ĐỘNG (ORCHESTRATED SAGA WITH DISK LOG)
// ============================================================================

#[derive(Debug, Clone, PartialEq, Eq)]
pub enum SagaStepStatus {
    Pending,
    ExecutedSuccessfully,
    CompensatedSuccessfully,
    CompensationFailed { error: String },
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct SagaStepDef {
    pub step_id: u32,
    pub name: String,
    pub forward_action: String,
    pub compensation_action: String,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct SagaLogEntry {
    pub step_id: u32,
    pub status: SagaStepStatus,
}

pub struct OrchestratedSaga {
    pub steps: Vec<SagaStepDef>,
    pub execution_log: Vec<SagaLogEntry>,
}

impl OrchestratedSaga {
    pub fn new(steps: Vec<SagaStepDef>) -> Self {
        Self {
            steps,
            execution_log: Vec::new(),
        }
    }

    pub fn run<F>(&mut self, mut runner: F) -> Result<(), Vec<String>>
    where
        F: FnMut(&str, bool) -> Result<(), String>,
    {
        let mut executed_indices = Vec::new();

        for (idx, step) in self.steps.iter().enumerate() {
            match runner(&step.forward_action, false) {
                Ok(()) => {
                    self.execution_log.push(SagaLogEntry {
                        step_id: step.step_id,
                        status: SagaStepStatus::ExecutedSuccessfully,
                    });
                    executed_indices.push(idx);
                }
                Err(forward_err) => {
                    let mut comp_errors = Vec::new();
                    comp_errors.push(format!(
                        "Bước {} ({}) thất bại: {}",
                        step.step_id, step.name, forward_err
                    ));

                    for &rev_idx in executed_indices.iter().rev() {
                        let comp_step = &self.steps[rev_idx];
                        match runner(&comp_step.compensation_action, true) {
                            Ok(()) => {
                                self.execution_log.push(SagaLogEntry {
                                    step_id: comp_step.step_id,
                                    status: SagaStepStatus::CompensatedSuccessfully,
                                });
                            }
                            Err(comp_err) => {
                                self.execution_log.push(SagaLogEntry {
                                    step_id: comp_step.step_id,
                                    status: SagaStepStatus::CompensationFailed {
                                        error: comp_err.clone(),
                                    },
                                });
                                comp_errors.push(format!(
                                    "Hoàn tác bước {} ({}) gặp lỗi: {}",
                                    comp_step.step_id, comp_step.name, comp_err
                                ));
                            }
                        }
                    }

                    return Err(comp_errors);
                }
            }
        }

        Ok(())
    }
}

// ============================================================================
// 7. KHÓA LẠC QUAN & RÀO CHẮN ĐƠN ĐIỆU (OPTIMISTIC FILE VERSION & FENCING TOKEN)
// ============================================================================

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct OptimisticFileVersion {
    pub file_path: String,
    pub version: u64,
    pub content_hash: Sha256Digest,
    pub content_bytes: Vec<u8>,
}

impl OptimisticFileVersion {
    pub fn new(file_path: &str, initial_content: &[u8]) -> Self {
        let content_hash = PureSha256::digest(initial_content);
        Self {
            file_path: file_path.to_string(),
            version: 1,
            content_hash,
            content_bytes: initial_content.to_vec(),
        }
    }

    pub fn compare_and_swap(
        &mut self,
        expected_version: u64,
        expected_hash: Sha256Digest,
        new_content: &[u8],
    ) -> Result<u64, String> {
        if self.version != expected_version {
            return Err(format!(
                "Xung đột phiên bản CAS: kỳ vọng {expected_version}, thực tế là {}",
                self.version
            ));
        }
        if self.content_hash != expected_hash {
            return Err(format!(
                "Xung đột nội dung CAS: hash kỳ vọng {} khác hash thực tế {}",
                expected_hash.to_hex_string(),
                self.content_hash.to_hex_string()
            ));
        }

        let new_hash = PureSha256::digest(new_content);
        self.version += 1;
        self.content_hash = new_hash;
        self.content_bytes = new_content.to_vec();
        Ok(self.version)
    }
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, PartialOrd, Ord)]
pub struct FencingToken {
    pub epoch: u64,
}

#[derive(Debug, Clone)]
pub struct FencingController {
    pub current_epoch: u64,
}

impl Default for FencingController {
    fn default() -> Self {
        Self::new()
    }
}

impl FencingController {
    pub fn new() -> Self {
        Self { current_epoch: 0 }
    }

    pub fn issue_next_token(&mut self) -> FencingToken {
        self.current_epoch += 1;
        FencingToken {
            epoch: self.current_epoch,
        }
    }

    pub fn validate_and_fence(&self, token: &FencingToken) -> Result<(), String> {
        if token.epoch < self.current_epoch {
            Err(format!(
                "Zombie Agent bị chặn: Thẻ epoch {} nhỏ hơn epoch hiện tại {}",
                token.epoch, self.current_epoch
            ))
        } else {
            Ok(())
        }
    }
}

// ============================================================================
// 8. NGĂN XẾP 3 GIAO THỨC TÁC TỬ (MCP + A2A + ANP)
// ============================================================================

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct McpRequest {
    pub jsonrpc: String,
    pub id: u64,
    pub method: String,
    pub params: BTreeMap<String, String>,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct McpResponse {
    pub jsonrpc: String,
    pub id: u64,
    pub result: Option<String>,
    pub error: Option<String>,
}

pub struct McpJsonRpcRouter {
    pub registered_tools: HashMap<String, String>,
}

impl Default for McpJsonRpcRouter {
    fn default() -> Self {
        Self::new()
    }
}

impl McpJsonRpcRouter {
    pub fn new() -> Self {
        Self {
            registered_tools: HashMap::new(),
        }
    }

    pub fn register_tool(&mut self, name: &str, description: &str) {
        self.registered_tools
            .insert(name.to_string(), description.to_string());
    }

    pub fn handle_request(&self, req: &McpRequest) -> McpResponse {
        if req.jsonrpc != "2.0" {
            return McpResponse {
                jsonrpc: "2.0".to_string(),
                id: req.id,
                result: None,
                error: Some("Phiên bản giao thức không hỗ trợ (chỉ chấp nhận '2.0')".to_string()),
            };
        }

        match req.method.as_str() {
            "tools/list" => {
                let mut tools_summary = Vec::new();
                for (name, desc) in &self.registered_tools {
                    tools_summary.push(format!("{name}: {desc}"));
                }
                tools_summary.sort();
                McpResponse {
                    jsonrpc: "2.0".to_string(),
                    id: req.id,
                    result: Some(tools_summary.join("; ")),
                    error: None,
                }
            }
            "tools/call" => {
                let tool_name = req.params.get("name").map(|s| s.as_str()).unwrap_or("");
                if let Some(desc) = self.registered_tools.get(tool_name) {
                    let arg = req.params.get("arg").map(|s| s.as_str()).unwrap_or("");
                    McpResponse {
                        jsonrpc: "2.0".to_string(),
                        id: req.id,
                        result: Some(format!(
                            "Thực thi thành công {tool_name} [{desc}] với arg={arg}"
                        )),
                        error: None,
                    }
                } else {
                    McpResponse {
                        jsonrpc: "2.0".to_string(),
                        id: req.id,
                        result: None,
                        error: Some(format!(
                            "Công cụ '{tool_name}' không tồn tại trong danh mục MCP"
                        )),
                    }
                }
            }
            "ping" => McpResponse {
                jsonrpc: "2.0".to_string(),
                id: req.id,
                result: Some("pong".to_string()),
                error: None,
            },
            other => McpResponse {
                jsonrpc: "2.0".to_string(),
                id: req.id,
                result: None,
                error: Some(format!("Phương thức không được hỗ trợ: '{other}'")),
            },
        }
    }
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct AgentCard {
    pub agent_id: String,
    pub name: String,
    pub capabilities: Vec<String>,
    pub cost_tier: u8,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub enum A2aTaskState {
    Submitted { prompt: String },
    Working { worker_agent_id: String },
    Completed { result: String },
    Failed { reason: String },
}

#[derive(Debug, Clone)]
pub struct A2aAgentNode {
    pub card: AgentCard,
    pub tasks: HashMap<u64, A2aTaskState>,
    pub next_task_id: u64,
}

impl A2aAgentNode {
    pub fn new(card: AgentCard) -> Self {
        Self {
            card,
            tasks: HashMap::new(),
            next_task_id: 1,
        }
    }

    pub fn submit_task(&mut self, prompt: &str) -> u64 {
        let task_id = self.next_task_id;
        self.next_task_id += 1;
        self.tasks.insert(
            task_id,
            A2aTaskState::Submitted {
                prompt: prompt.to_string(),
            },
        );
        task_id
    }

    pub fn start_working(&mut self, task_id: u64) -> Result<(), String> {
        if let Some(task) = self.tasks.get_mut(&task_id) {
            match task {
                A2aTaskState::Submitted { .. } => {
                    *task = A2aTaskState::Working {
                        worker_agent_id: self.card.agent_id.clone(),
                    };
                    Ok(())
                }
                _ => Err("Chỉ có thể chuyển sang Working từ trạng thái Submitted".to_string()),
            }
        } else {
            Err(format!("Không tìm thấy tác vụ {task_id}"))
        }
    }

    pub fn complete_task(&mut self, task_id: u64, result: &str) -> Result<(), String> {
        if let Some(task) = self.tasks.get_mut(&task_id) {
            match task {
                A2aTaskState::Working { .. } => {
                    *task = A2aTaskState::Completed {
                        result: result.to_string(),
                    };
                    Ok(())
                }
                _ => Err("Chỉ có thể Completed khi đang Working".to_string()),
            }
        } else {
            Err(format!("Không tìm thấy tác vụ {task_id}"))
        }
    }
}

#[derive(Debug, Clone)]
pub struct AnpAgentRegistration {
    pub card: AgentCard,
    pub in_flight_load: usize,
    pub is_healthy: bool,
}

#[derive(Debug, Clone)]
pub struct AnpDiscoveryRegistry {
    pub agents: HashMap<String, AnpAgentRegistration>,
}

impl Default for AnpDiscoveryRegistry {
    fn default() -> Self {
        Self::new()
    }
}

impl AnpDiscoveryRegistry {
    pub fn new() -> Self {
        Self {
            agents: HashMap::new(),
        }
    }

    pub fn register_agent(&mut self, card: AgentCard) {
        let agent_id = card.agent_id.clone();
        self.agents.insert(
            agent_id,
            AnpAgentRegistration {
                card,
                in_flight_load: 0,
                is_healthy: true,
            },
        );
    }

    pub fn update_load(&mut self, agent_id: &str, in_flight: usize) {
        if let Some(reg) = self.agents.get_mut(agent_id) {
            reg.in_flight_load = in_flight;
        }
    }

    pub fn route_least_loaded(&self, required_capability: &str) -> Option<String> {
        self.agents
            .values()
            .filter(|reg| {
                reg.is_healthy
                    && reg
                        .card
                        .capabilities
                        .iter()
                        .any(|c| c == required_capability)
            })
            .min_by_key(|reg| (reg.in_flight_load, reg.card.cost_tier))
            .map(|reg| reg.card.agent_id.clone())
    }
}

fn main() {
    println!("=== CHƯƠNG 89: ĐỘ BỀN PHỤC HỒI SỰ CỐ & GIAO THỨC TÁC TỬ (MCP + A2A + ANP) ===");

    let mut wal = TurnWal::new();
    let prompt = b"Hay phan tich do ben cua he thong Agent";
    wal.append("sess-001", WalPayloadKind::UserMessage, prompt);
    let is_logged = wal.verify_model_visible_logged("sess-001", prompt);
    println!(
        "1. Bất biến 'Model-visible means logged': {}",
        if is_logged {
            "ĐẠT (PASSED)"
        } else {
            "VI PHẠM"
        }
    );

    let mut tree = SessionTree::new();
    let root = tree.create_root(SessionRole::System, "Ban la chuyen gia Rust");
    let branch_a = tree
        .fork_branch(root, SessionRole::User, "Viet code WAL")
        .unwrap();
    let branch_b = tree
        .fork_branch(root, SessionRole::User, "Viet code Saga")
        .unwrap();
    println!(
        "2. SessionTree: Đã rẽ nhánh độc lập A (ID={branch_a}) và B (ID={branch_b}) từ gốc (ID={root})"
    );

    let mut fencing = FencingController::new();
    let token_1 = fencing.issue_next_token();
    let token_2 = fencing.issue_next_token();
    let zombie_check = fencing.validate_and_fence(&token_1);
    println!(
        "3. FencingToken: Tiến trình cũ với epoch {} ghi đè -> {}",
        token_1.epoch,
        zombie_check.is_err()
    );
    let valid_check = fencing.validate_and_fence(&token_2);
    println!(
        "   Tiến trình mới với epoch {} ghi đè -> {}",
        token_2.epoch,
        valid_check.is_ok()
    );

    let mut registry = AnpDiscoveryRegistry::new();
    registry.register_agent(AgentCard {
        agent_id: "agent-code-refactor".to_string(),
        name: "Chuyên gia Refactor".to_string(),
        capabilities: vec!["refactor".to_string(), "lint".to_string()],
        cost_tier: 1,
    });
    registry.register_agent(AgentCard {
        agent_id: "agent-code-architect".to_string(),
        name: "Kiến trúc sư Trưởng".to_string(),
        capabilities: vec!["refactor".to_string(), "design".to_string()],
        cost_tier: 3,
    });
    let best = registry.route_least_loaded("refactor").unwrap();
    println!("4. ANP Routing: Tác tử chi phí tối ưu nhất cho 'refactor' là: {best}");
    println!("=== HOÀN TẤT MINH HỌA HỆ THỐNG ĐỘ BỀN ===");
}
```

---

## 5. Bảng tra cứu lỗi biên dịch thường gặp

Khi xây dựng các hệ thống phục hồi sự cố, con trỏ đồ thị cây và điều phối giao dịch phân tán bằng Rust, lập trình viên thường xuyên đối mặt với các lỗi biên dịch đặc thù của Borrow Checker và hệ thống kiểu. Dưới đây là bảng tra cứu chi tiết:

| Mã lỗi / Cảnh báo | Tình huống phát sinh | Nguyên nhân cốt lõi | Cách khắc phục chuẩn chỉ |
|---|---|---|---|
| `E0502: cannot borrow ... as mutable because it is also borrowed as immutable` | Lần ngược cây `SessionTree::replay_path` trong khi đang giữ tham chiếu `&node` | Đang giữ tham chiếu bất biến vào nút trong `HashMap` nhưng lại gọi một hàm làm thay đổi cấu trúc bảng | Lưu trữ danh sách khóa định danh `SessionNodeId` (kiểu `u64` Copy), giải phóng tham chiếu mượn, sau đó truy vấn nút hoặc clone dữ liệu nút cần thiết. |
| `E0382: use of moved value` | Duyệt ngược hoàn tác trong `OrchestratedSaga::run` | Biến `comp_step` hoặc `runner` bị di chuyển quyền sở hữu vào trong closure | Mượn tham chiếu `&comp_step` hoặc sử dụng closure mượn biến bằng `FnMut(&str, bool) -> Result<(), String>` thay vì `FnOnce`. |
| `E0308: mismatched types: expected [u8; 32], found &[u8]` | Tính mã băm `PureSha256::digest` trả về `Sha256Digest` nhưng hàm lưu trữ yêu cầu slice hoặc mảng | Sai lệch giữa kiểu struct bao bọc `Sha256Digest` và mảng thô `[u8; 32]` | Dùng trường công khai `digest.bytes` hoặc cài đặt trait `AsRef<[u8]>` để chuyển đổi zero-cost. |
| `E0499: cannot borrow *self as mutable more than once at a time` | Gọi `circuit_breaker.record_failure` bên trong phương thức `LlmGateway::execute` | Trường `circuit_breaker` và `provider` đều thuộc `self`, borrow checker phát hiện xung đột mượn toàn bộ struct `self` | Tách rời các trường mutable độc lập hoặc truyền tham chiếu tường minh từng trường con vào hàm helper thay vì truyền toàn bộ `&mut self`. |
| `E0277: the trait Copy is not implemented for String` | Sao chép `session_id` trong khung nhị phân `WalFrame` | Chuỗi `String` quản lý bộ nhớ heap, không thể tự động Copy | Dùng `.clone()`, hoặc tái cấu trúc thành `&str` nếu khung nhị phân chỉ tồn tại tạm thời trong vòng đời hàm quét. |
| `clippy::needless_range_loop` | Dùng `for i in 0..64` để gán giá trị mảng `w[i]` trong khối SHA-256 | Lặp theo chỉ số gây kiểm tra biên thừa thãi tại runtime (bounds check overhead) | Chuyển sang `for (i, w_item) in w.iter_mut().enumerate()` để trình biên dịch vector hóa và loại bỏ hoàn toàn bounds check. |

---

## 6. Tóm tắt & Bài tập rèn luyện

### Tóm tắt các bài học sống còn của Chương 89

1. **Phân tách ranh giới lỗi rõ ràng**: Lỗi client (`Fatal4xx`) do sai logic hoặc schema phải được chặn đứng và báo lỗi ngay lập tức, không bao giờ thử lại và không bao giờ mở cầu dao ngắt mạch. Chỉ lỗi máy chủ (`Transient5xx`) mới kích hoạt `CircuitBreaker`.
2. **Khắc cốt ghi tâm bất biến `"Model-visible means logged"`**: Mọi token và ngữ cảnh đưa vào tầm nhìn của mô hình phải được flush xuống đĩa (`TurnWal`) trước khi gửi đi. Sập nguồn sau đó có thể được cứu nhờ cơ chế tự gọt đuôi rách (`Torn-tail truncation`).
3. **Cây phiên bảo toàn dữ liệu**: Đừng bao giờ ghi đè lịch sử hội thoại. Dùng `SessionTree` với con trỏ tổ tiên để hỗ trợ rẽ nhánh suy luận tùy ý và tái tạo tuyến tính hoàn hảo.
4. **Không nhân đôi tác dụng phụ**: Khóa lũy đẳng hai pha `IdempotencyStore` và `TransactionalOutbox` bảo đảm tính nhất quán của các hành động ra thế giới thực.
5. **Hoàn tác kiên cường**: `OrchestratedSaga` hoàn tác ngược từ đĩa và tiếp tục bù trừ kể cả khi một bước đơn lẻ gặp lỗi.
6. **Khóa hai lớp & Thẻ rào chắn**: `OptimisticFileVersion` CAS bằng cặp `(version, sha256)` và `FencingToken` triệt tiêu hoàn toàn nguy cơ ghi đè mã nguồn từ các tác tử cũ (Zombie Agents).
7. **Chuẩn hóa giao thức**: Ngăn xếp 3 tầng `MCP` (công cụ), `A2A` (ủy quyền tác tử), và `ANP` (khám phá mạng lưới & định tuyến tải ít nhất) mở ra khả năng mở rộng quy mô không giới hạn.

---

### Bài tập rèn luyện chuyên sâu

#### Bài tập 1: Tích hợp Điểm kiểm tra (Checkpoint Snapshot) vào `TurnWal`
Khi một phiên làm việc kéo dài hàng nghìn lượt hội thoại, tệp nhật ký `TurnWal` có thể phình to lên hàng trăm Megabyte. Mỗi lần khởi động lại sau crash, việc quét lại từ đầu tệp sẽ tốn nhiều thời gian. Hãy thiết kế cấu trúc `WalCheckpoint`:
- Cứ sau mỗi $K$ khung (ví dụ: $K=100$), ghi một ảnh chụp trạng thái tổng hợp (Snapshot).
- Khi phục hồi, hệ thống nhảy ngay tới Checkpoint gần nhất và chỉ cần phát lại (replay) các khung từ sau Checkpoint đó.

<details>
<summary>Gợi ý & Lời giải Bài tập 1</summary>

```rust
use std::collections::HashMap;

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct WalCheckpoint {
    pub last_seq: u64,
    pub session_states: HashMap<String, Vec<u8>>, // Ảnh chụp trạng thái tóm tắt của từng session
}

pub struct CheckpointedWal {
    pub wal: TurnWal,
    pub latest_checkpoint: Option<WalCheckpoint>,
    pub checkpoint_interval: u64,
}

impl CheckpointedWal {
    pub fn new(interval: u64) -> Self {
        Self {
            wal: TurnWal::new(),
            latest_checkpoint: None,
            checkpoint_interval: interval,
        }
    }

    pub fn append_and_maybe_checkpoint(
        &mut self,
        session_id: &str,
        kind: WalPayloadKind,
        payload: &[u8],
    ) {
        self.wal.append(session_id, kind, payload);
        let current_seq = self.wal.next_seq - 1;

        if current_seq.is_multiple_of(self.checkpoint_interval) {
            // Tạo Checkpoint mới
            let mut states = HashMap::new();
            states.insert(session_id.to_string(), payload.to_vec());
            self.latest_checkpoint = Some(WalCheckpoint {
                last_seq: current_seq,
                session_states: states,
            });
        }
    }

    pub fn fast_recovery_start_seq(&self) -> u64 {
        self.latest_checkpoint
            .as_ref()
            .map(|cp| cp.last_seq + 1)
            .unwrap_or(1)
    }
}

#[test]
fn test_checkpointed_wal_fast_recovery() {
    let mut cwal = CheckpointedWal::new(5);
    for i in 1..=12 {
        cwal.append_and_maybe_checkpoint("s1", WalPayloadKind::UserMessage, format!("Msg {i}").as_bytes());
    }
    // Checkpoint được tạo tại seq 5 và seq 10
    assert_eq!(cwal.fast_recovery_start_seq(), 11);
}
```
</details>

---

#### Bài tập 2: Cửa sổ trượt tính tỷ lệ lỗi (Sliding Window Rate Counter) cho `CircuitBreaker`
Hiện tại, `CircuitBreaker` đếm số lỗi liên tiếp (`consecutive_failures`). Trong thực tế, các hệ thống phân tán lớn thường dùng tỷ lệ phần trăm lỗi trong một cửa sổ trượt (ví dụ: mở mạch nếu trong 100 yêu cầu gần nhất có hơn 50% bị lỗi 5xx). Hãy thiết kế `SlidingWindowCircuitBreaker` thuần `std`.

<details>
<summary>Gợi ý & Lời giải Bài tập 2</summary>

```rust
use std::collections::VecDeque;

#[derive(Debug, Clone)]
pub struct SlidingWindowCircuitBreaker {
    pub window_size: usize,
    pub failure_rate_threshold: f64, // Ví dụ: 0.5 (50%)
    pub samples: VecDeque<bool>, // true = thành công, false = lỗi 5xx
    pub is_open: bool,
}

impl SlidingWindowCircuitBreaker {
    pub fn new(window_size: usize, threshold: f64) -> Self {
        Self {
            window_size,
            failure_rate_threshold: threshold,
            samples: VecDeque::with_capacity(window_size),
            is_open: false,
        }
    }

    pub fn record_result(&mut self, is_success: bool) {
        if self.samples.len() == self.window_size {
            self.samples.pop_front();
        }
        self.samples.push_back(is_success);

        // Chỉ đánh giá khi cửa sổ đã thu thập tối thiểu 50% dung lượng
        if self.samples.len() >= self.window_size / 2 {
            let failures = self.samples.iter().filter(|&&ok| !ok).count();
            let rate = (failures as f64) / (self.samples.len() as f64);
            self.is_open = rate >= self.failure_rate_threshold;
        }
    }

    pub fn can_execute(&self) -> bool {
        !self.is_open
    }
}

#[test]
fn test_sliding_window_circuit_breaker() {
    let mut sw = SlidingWindowCircuitBreaker::new(10, 0.4); // Ngưỡng 40%
    for _ in 0..6 {
        sw.record_result(true);
    }
    assert!(sw.can_execute());

    // Thêm 4 lỗi liên tiếp -> tỷ lệ lỗi 4/10 = 40% -> Mở mạch
    for _ in 0..4 {
        sw.record_result(false);
    }
    assert!(!sw.can_execute());
}
```
</details>

---

#### Bài tập 3: Hàng đợi ưu tiên chống đói tài nguyên trong `A2aAgentNode`
Khi nhiều Agent gửi tác vụ tới cùng một nút làm việc, các tác vụ có độ ưu tiên cao (VIP / Critical) cần được xử lý trước. Tuy nhiên, nếu liên tục có tác vụ VIP tràn vào, các tác vụ thường sẽ bị "chết đói" (Starvation). Hãy mở rộng `A2aAgentNode` để quản lý hàng đợi ưu tiên có cơ chế tăng tuổi (Aging / Anti-Starvation).

<details>
<summary>Gợi ý & Lời giải Bài tập 3</summary>

```rust
use std::collections::BinaryHeap;
use std::cmp::Ordering;

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct PrioritizedTask {
    pub task_id: u64,
    pub base_priority: u32,
    pub submitted_tick: u64,
}

impl PrioritizedTask {
    /// Tính toán độ ưu tiên hiệu dụng tại thời điểm `current_tick`
    /// Mỗi 10 tick trôi qua, độ ưu tiên được cộng thêm 1 để chống đói tài nguyên
    pub fn effective_priority(&self, current_tick: u64) -> u64 {
        let age = current_tick.saturating_sub(self.submitted_tick);
        let bonus = age / 10;
        (self.base_priority as u64) + bonus
    }
}

// Cài đặt Ord để BinaryHeap lấy phần tử có effective_priority lớn nhất
struct HeapItem {
    task: PrioritizedTask,
    current_tick: u64,
}

impl PartialEq for HeapItem {
    fn eq(&self, other: &Self) -> bool {
        self.task.effective_priority(self.current_tick) == other.task.effective_priority(other.current_tick)
    }
}
impl Eq for HeapItem {}

impl Ord for HeapItem {
    fn cmp(&self, other: &Self) -> Ordering {
        self.task.effective_priority(self.current_tick)
            .cmp(&other.task.effective_priority(other.current_tick))
    }
}

impl PartialOrd for HeapItem {
    fn partial_cmp(&self, other: &Self) -> Option<Ordering> {
        Some(self.cmp(other))
    }
}

#[test]
fn test_aging_priority_queue_anti_starvation() {
    let t_normal = PrioritizedTask {
        task_id: 1,
        base_priority: 5,
        submitted_tick: 0,
    };
    let t_vip = PrioritizedTask {
        task_id: 2,
        base_priority: 10,
        submitted_tick: 60,
    };

    // Tại nhịp 0, VIP chưa tới, Normal có priority = 5
    assert_eq!(t_normal.effective_priority(0), 5);

    // Tại nhịp 70:
    // t_vip có age = 10 -> bonus = 1 -> priority = 10 + 1 = 11
    // t_normal có age = 70 -> bonus = 7 -> priority = 5 + 7 = 12 (thắng VIP nhờ cơ chế aging!)
    assert_eq!(t_vip.effective_priority(70), 11);
    assert_eq!(t_normal.effective_priority(70), 12);
    assert!(t_normal.effective_priority(70) > t_vip.effective_priority(70));
}
```
</details>
