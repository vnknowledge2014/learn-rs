# Chương 86: Vòng lặp Tác tử Streaming, Kinh tế học KV-Cache, Nén Hai Tầng & Gương Ngữ cảnh Tự quản (Streaming Agent Loop, KV-Cache Economics & Live Context Mirror)

---

## 1. Giới thiệu & Mục tiêu

Trong các hướng dẫn nhập môn hoặc các bản mẫu chứng minh khái niệm (PoC), vòng lặp của một tác tử trí tuệ nhân tạo (AI Agent Loop) thường được biểu diễn bằng một vài dòng mã ngây thơ:

```python
# Cách tiếp cận ngây thơ thường thấy trong các bản demo:
while not done:
    response = call_llm(messages)
    if response.tool_calls:
        results = execute_tools(response.tool_calls)
        messages.append(results)
    else:
        done = True
```

Đoạn mã trên hoạt động tốt trên các kịch bản trình diễn từ 3 đến 5 lượt tương tác ngắn. Tuy nhiên, khi đưa vào **môi trường sản xuất cấp công nghiệp** (production-grade harness) — nơi một tác tử lập trình tự hành (Autonomous Coding Agent) phải chạy liên tục từ hàng chục đến hàng nghìn bước, xử lý hàng triệu token, tương tác với hệ thống tệp thật, và quản lý ngân sách tài chính nghiêm ngặt — đoạn mã ngây thơ này sẽ **lập tức sụp đổ** vì hàng loạt nguyên nhân mang tính sống còn:

1. **Hiểm họa vỡ byte UTF-8 trong luồng Server-Sent Events (SSE)**: Các nhà cung cấp mô hình ngôn ngữ lớn (LLM) trả về dữ liệu qua giao thức streaming HTTP/SSE theo các gói TCP/chunk có kích thước tùy ý. Một ký tự tiếng Việt đa byte (ví dụ chữ `ế` chiếm 3 byte `[0xe1, 0xba, 0xbf]`) có thể bị cắt ngang: 1 byte nằm ở cuối chunk trước, 2 byte nằm ở đầu chunk sau. Nếu bộ phân tích cú pháp ép kiểu chuỗi `&str` ngay lập tức, chương trình sẽ hoảng loạn (`panic!`) làm sập toàn bộ phiên làm việc của tác tử.
2. **Tràn ngăn xếp cuộc gọi (Stack Overflow) do đệ quy sâu**: Việc tổ chức vòng lặp tác tử dưới dạng hàm đệ quy (`fn step(ctx) -> fn step(...)`) mà không có cơ chế đảm bảo TCO (*Tail Call Optimization*) trong Rust sẽ khiến ngăn xếp (call stack) phình to theo thời gian thực thi. Ở bước thứ 5.000 hoặc 10.000, tiến trình sẽ cạn bộ nhớ ngăn xếp và bị hệ điều hành tiêu diệt bằng tín hiệu `SIGSEGV`.
3. **Thảm họa thực thi lệnh cụt đuôi khi chạm trần `max_tokens`**: Khi phản hồi của mô hình chạm giới hạn độ dài token tối đa, chuỗi JSON tham số công cụ bị cắt ngang ở vị trí bất kỳ (ví dụ: `{"command": "rm -rf /tmp/build_dir"` bị mất đoạn đuôi và trở thành `rm -rf /`). Nếu bộ khung không phát hiện lý do dừng `max_tokens` và chặn đứng việc thực thi, tác tử sẽ chạy một lệnh cụt đuôi tàn phá hệ thống máy chủ.
4. **Trôi mã băm và sụp đổ kinh tế học KV-Cache**: Các hệ thống phục vụ mô hình hiện đại (như vLLM, SGLang, Anthropic Prompt Caching, OpenAI Prefix Caching) dựa hoàn toàn vào việc so khớp tiền tố tĩnh (*prefix matching*) để tái sử dụng bảng bộ nhớ đệm Khóa-Giá trị (*KV-Cache*). Nếu tác tử chèn các nội dung biến động (như đồng hồ thời gian thực, số dư tài khoản, hoặc xáo trộn thứ tự RAG theo kiểu `lost_in_middle_order`) vào phần đầu prompt, **100% bộ đệm KV-Cache bị trượt (cache miss)**. Hóa đơn API sẽ đội giá gấp từ 1,25x đến 10x, đồng thời độ trễ tạo token đầu tiên (*Time To First Token - TTFT*) tăng vọt từ 200ms lên 15 giây.
5. **Làm mồ côi cặp `(tool_use, tool_result)`**: Các giao thức mô hình hiện đại (như Anthropic Claude Messages API hay OpenAI Tools) bắt buộc mọi khối `tool_use(id)` phải có chính xác một khối `tool_result(id)` tương ứng đi liền sau. Khi ngữ cảnh phình to và tác tử cắt gọt lịch sử, việc vô tình cắt mất kết quả hoặc yêu cầu gọi công cụ sẽ khiến toàn bộ phiên làm việc bị máy chủ API từ chối vĩnh viễn với mã lỗi `400 Bad Request`.
6. **Tích lũy sai số dấu phẩy động trong FinOps**: Đơn giá token thường ở mức phần triệu USD (ví dụ $3,00 cho 1 triệu token đầu vào = $0,000003/token). Việc cộng dồn hàng triệu lần bằng kiểu `f32` hoặc `f64` sẽ sinh ra sai số làm tròn số học, dẫn đến việc kiểm toán chi phí lệch hàng chục phần trăm và làm gãy các rào chắn ngân sách cứng (*Hard Budget Gates*).
7. **Lỗ hổng leo thang đặc quyền qua gương ngữ cảnh (Edge Case #4)**: Khi cho phép tác tử tự chỉnh sửa lịch sử hội thoại của chính mình thông qua giao thức gương ngữ cảnh (`LiveContextMirror`), kẻ tấn công hoặc chính mô hình bị ảo tưởng có thể lén đổi vai trò `role=system` để cướp quyền điều khiển, bịa đặt các ID công cụ giả mạo, hoặc đảo ngược thứ tự thực thi công cụ trong quá khứ nhằm che giấu hành vi phá hoại.

```
┌──────────────────────────────────────────────────────────────────────────────┐
│       BẢN ĐỒ CÁC NGUY CƠ ĐE DỌA VÒNG LẶP TÁC TỬ CẤP SẢN XUẤT                  │
│                                                                              │
│  [ Luồng SSE Byte ] ──► [ Cắt ngang byte UTF-8 ] ──► Panic sập tiến trình     │
│  [ Vòng lặp đệ quy] ──► [ Đệ quy 10.000 bước   ] ──► Stack Overflow          │
│  [ Chạm Max Tokens] ──► [ Lệnh JSON cụt đuôi   ] ──► Thực thi nhầm rm -rf /   │
│  [ Xáo trộn Prompt] ──► [ Trượt KV-Cache 100%  ] ──► Chi phí x10, trễ x50    │
│  [ Cắt gọt thô bạo] ──► [ Mồ côi Tool ID       ] ──► Lỗi 400 Bad Request     │
│  [ Kế toán f64    ] ──► [ Sai số tích lũy      ] ──► Vỡ rào chắn ngân sách   │
│  [ Gương ngữ cảnh ] ──► [ Đảo thứ tự / Giả ID  ] ──► Phá vỡ an ninh hệ thống │
└──────────────────────────────────────────────────────────────────────────────┘
```

### Mục tiêu học tập cốt lõi

Sau khi hoàn thành chương này, bạn sẽ nắm vững và tự tay xây dựng được từ con số 0 bằng **100% Rust 2024 thuần `std`** (không dùng bất kỳ crate bên ngoài nào):

1. **Bộ phân tích khung sự kiện SSE (`SseFrameParser`) & Bộ lắp ráp lượt (`SseTurnAssembler`)**:
   - Vùng đệm byte thông minh, phục hồi an toàn khi ký tự tiếng Việt đa byte UTF-8 bị cắt đôi qua biên chunk mạng.
   - Cơ chế kiểm tra cân bằng dấu ngoặc LIFO cho chuỗi JSON tham số.
   - Bất biến an toàn: Phát hiện `stop_reason == "max_tokens"` và chặn đứng việc thực thi các `tool_use` cụt đuôi.
2. **Kỹ thuật phẳng hóa đệ quy bằng Trampoline (`Bounce<S, T>`)**:
   - Chuyển đổi mọi vòng lặp tác tử đệ quy thành cấu trúc lặp ngăn xếp $O(1)$, chứng minh chạy mượt mà qua 10.000 bước mà không làm tăng dung lượng stack.
3. **Hàng đợi lái hướng giữa lượt (`TurnInboxQueue`)**:
   - Hàng đợi ưu tiên hai khóa (Độ ưu tiên + Số thứ tự sequence) hỗ trợ 4 mức: `SystemCancel` (hủy khẩn cấp), `HumanSteer` (con người can thiệp), `PeerSignal` (tác tử đồng cấp), `BgNotify` (thông báo nền).
4. **Phân giải phạm vi chỉ thị đa tầng (`InstructionScopeResolver`) & Delta môi trường (`DeltaEnvEmitter`)**:
   - Phân tầng nghiêm ngặt: `Enterprise` (bất biến) $\to$ `User` $\to$ `Project` $\to$ `Directory`.
   - Bộ phát tín hiệu biến động môi trường (`Set`/`Unset`) giúp tiết kiệm đến 80% token môi trường thừa thãi.
5. **Đường ống chấm điểm ngữ cảnh GSSC ($C = \mathcal{A}(c_1 \dots c_6)$)**:
   - Thuật toán chấm điểm ngữ cảnh dung hợp 6 thành phần: Độ tương đồng Jaccard với mục tiêu, độ suy giảm khoảng cách bước hàm mũ $\exp(-\lambda \cdot \Delta t)$, độ tin cậy trạng thái, hiệu ích công cụ, phạt chi phí token và độ mới mẻ của thông tin.
6. **Kinh tế học KV-Cache & Tiền tố đóng băng (`PrefixStablePrompt`)**:
   - Hiểu sâu sắc cơ chế PagedAttention/RadixAttention.
   - Thiết kế tiền tố đóng băng bất biến ghim chặt ở đầu prompt, nối lười lược đồ công cụ MCP theo thứ tự từ điển xác định, duy trì mã băm FNV-1a đồng nhất 100% qua hàng chục lượt tương tác.
7. **Sổ cái số nguyên vi mô `PicoDollars` & Đồng hồ đo lượt cuối `LastLegUsageMeter` (ADR-0018)**:
   - Đơn vị đo chi phí tuyệt đối: $1\text{ USD} = 10^{12}\text{ PicoDollars}$.
   - Biểu giá 2 tầng TTL: Bộ nhớ đệm 5 phút (1.25x ghi, 0.1x đọc), 1 giờ (2.0x ghi, 0.1x đọc).
   - Tách biệt rạch ròi chi phí chặng hiện tại (`current_leg`) với tổng tích lũy toàn phiên (`accumulated_run`), giải quyết triệt để lỗi nhân đôi token.
8. **Nén ngữ cảnh hai tầng (`TwoTierCompactor`) & Bộ đánh giá câu hỏi thăm dò (`CompressionProbeEvaluator`)**:
   - Tầng 1: Vi nén RAM (`microcompact`) gọt bỏ kết quả công cụ cồng kềnh.
   - Tầng 2: Tóm tắt điểm neo (`AnchoredSummary`) 6 thành phần bất biến, cấm xé lẻ cặp `(tool_use, tool_result)`.
   - Đánh giá chất lượng nén bằng 4 loại câu hỏi thăm dò: Recall, Artifact, Continuation, Decision.
9. **Gương ngữ cảnh tự quản `LiveContextMirror` (`CLM`) & Khắc phục lỗi Edge Case #4**:
   - Thực thi trọn vẹn 6 bất biến CLM: Gập `role=system` giả mạo về `user`, bảo vệ lượt ghim, cưỡng chế cổng `Fit`/`Shrink`, tự động gộp tin nhắn đồng vai trò, biên nhận kiểm toán minh bạch.
   - **Sửa triệt để lỗi Edge Case #4**: Bắt buộc cặp `Assistant(tool_use=id)` liền trước `User(tool_result=id)` phải tuần tự đối chiếu với tập ID gốc `orig`, chống tuyệt đối việc tạo ID công cụ ma hoặc đảo lộn thứ tự thực thi.
10. **Bộ phát hiện thoái hóa ngữ cảnh (`ContextDegradationDetector`) & Nén KV tiềm ẩn (MAD)**:
    - Phát hiện 5 chế độ suy thoái: Vòng lặp lặp lại, công cụ ảo tưởng, bùng nổ token, vi phạm ràng buộc cấm, lượt phản hồi rỗng.
    - Lọc bỏ vector KV tiềm ẩn bằng ngưỡng Độ lệch Tuyệt đối Trung vị (MAD): $\text{Threshold} = \text{median}(X) + \tau \cdot \text{MAD}(X)$.

---

## 2. Hình tượng hóa đời sống: Sân bay quốc tế, thẻ lên tàu và gương soi an ninh

Để hình dung toàn diện về các cấu trúc phức tạp trong chương này, hãy tưởng tượng toàn bộ hệ sinh thái của tác tử là quy trình vận hành của một **Sân bay Quốc tế 5 sao**:

```
┌──────────────────────────────────────────────────────────────────────────────┐
│                    HÌNH TƯỢNG HÓA SÂN BAY QUỐC TẾ                            │
│                                                                              │
│  [ Băng chuyền hành lý ] ──► Ghép từng mảnh kiện hàng (SSE Byte Stream)       │
│  [ Làn ưu tiên lên tàu ] ──► Phẳng hóa Trampoline, không giẫm đạp lên nhau   │
│  [ Thẻ lên máy bay    ] ──► Tiền tố KV-Cache: Số hiệu chuyến bay cố định      │
│  [ Gương soi an ninh   ] ──► LiveContextMirror (CLM): Kiểm tra vé & đồ đạc    │
│  [ Đồng hồ cân hành lý ] ──► PicoDollars & LastLegUsageMeter: Cân đến miligram│
└──────────────────────────────────────────────────────────────────────────────┘
```

### 1. Băng chuyền hành lý và Dòng sự kiện SSE
Hành lý của hành khách từ khoang máy bay được dỡ xuống và đẩy qua băng chuyền theo từng kiện rời rạc. Đôi khi, một kiện hành lý cồng kềnh (ký tự UTF-8 đa byte tiếng Việt) bị kẹt giữa hai khúc nối của băng chuyền: một nửa rơi xuống trước, một nửa rơi xuống sau.
- Nếu nhân viên mặt đất vội vàng nhặt nửa kiện đầu tiên rồi bàn giao ngay cho hành khách, kiện hàng sẽ bị rách nát (chuỗi UTF-8 lỗi $\to$ `panic!`).
- Người nhân viên chuyên nghiệp (`SseFrameParser`) sẽ đặt nửa kiện đó vào một khay đệm riêng biệt (`leftover_bytes`), kiên nhẫn đợi nửa kiện tiếp theo trôi tới, ghép chúng lại hoàn chỉnh thành một kiện nguyên vẹn rồi mới đưa ra cửa trả đồ.

### 2. Làn ưu tiên và Vòng lặp phẳng hóa Trampoline
Tại cửa kiểm soát hộ chiếu, nếu hàng nghìn hành khách đứng chồng lên vai nhau thành một cột tháp thẳng đứng (ngăn xếp cuộc gọi đệ quy), người dưới đáy sẽ gãy lưng vì chịu sức nặng quá lớn (Stack Overflow).
- Ban quản lý sân bay bố trí làn di chuyển theo hình chữ Z uốn lượn trên mặt đất (`run_trampoline`). Mỗi hành khách bước một bước rồi đứng vào vị trí tiếp theo (`Bounce::Continue`).
- Độ cao ngăn xếp luôn luôn là $O(1)$ — một người duy nhất đứng trên sàn nhà, dù đoàn người có dài tới 10.000 hành khách thì sân bay vẫn vận hành êm ái, tuyệt đối không bao giờ sập sàn.

### 3. Thẻ lên máy bay (Boarding Pass) và Tiền tố ổn định KV-Cache
Trên thẻ lên tàu của hãng hàng không, các thông tin quan trọng nhất luôn được in cố định ở phần đầu: *Tên hãng hàng không, Số hiệu chuyến bay, Mã sân bay đi/đến, Quy định an toàn quốc tế* (`StaticPrefix` và `mcp_tool_schemas`). Phần cuống vé ở đuôi chỉ ghi số ghế và giờ lên máy bay của từng chặng (`dynamic_turns`).
- Nhân viên soi mã vạch ở cửa ra máy bay chỉ cần quét qua phần đầu thẻ được in tiêu chuẩn là hệ thống nhận diện tức thì trong 1 mili-giây (Trúng KV-Cache 99%).
- Thử tưởng tượng: nếu mỗi lần hành khách đổi chỗ ngồi, hãng bay lại in lại toàn bộ điều lệ hàng không với thứ tự từ ngữ bị xáo trộn lung tung, máy quét mã vạch sẽ phải đọc lại từ đầu, khiến hàng nghìn hành khách ùn tắc ở cửa lên tàu, tiêu tốn hàng triệu lít nhiên liệu máy bay chạy không tải trên đường băng (Trượt KV-cache $\to$ chi phí vọt xà, độ trễ kéo dài).

### 4. Gương soi an ninh toàn thân (Live Context Mirror)
Trước khi bước vào khu vực cách ly, hành khách phải đi qua một tấm gương soi an ninh có vạch kẻ chuẩn mực (`render_context_mirror`).
- Nếu hành khách mang theo đồ đạc quá cồng kềnh, nhân viên an ninh yêu cầu nén nhỏ hoặc gửi vào khoang hàng hóa (`microcompact` và `AnchoredSummary`).
- Nếu có ai đó lén dán nhãn "Thanh tra Hàng không" lên ngực (`role=system` giả mạo) trong phòng thay đồ, gương an ninh sẽ tự động lột tấm nhãn đó ra và hạ cấp họ về đúng danh phận hành khách thông thường (`role=user`).
- **Quy tắc an ninh tối thượng (Sửa lỗi Edge Case #4)**: Cuống vé hành lý (`tool_result_id`) phải gắn liền ngay sau thẻ gửi đồ (`tool_use_id`). Tuyệt đối cấm hành khách tự in ra một mã gửi đồ giả mạo không có trong sổ lưu sân bay, và cấm tráo đổi cuống vé của kiện đồ này sang kiện đồ khác!

### 5. Cân hành lý điện tử và Sổ cái vi mô PicoDollars
Tại quầy thủ tục quốc tế, cước phí vượt cân hành lý không bao giờ được tính bằng cách "ước lượng áng chừng" một vài lạng (tránh sai số `f64`). Chiếc cân điện tử đo chính xác đến từng miligram và chuyển đổi sang số nguyên pico-dollar. Đồng thời, đồng hồ đo chặng (`LastLegUsageMeter`) chỉ tính tiền riêng cho kiện hành lý bạn vừa đặt lên cân, chứ không cộng dồn lại toàn bộ số hành lý của những người xếp hàng trước đó.

---

## 3. Khái niệm & Cơ chế kỹ thuật chuyên sâu

### 3.1. Phân tích luồng Server-Sent Events (SSE) theo chuẩn RFC 8895 & An toàn UTF-8

Giao thức SSE là một cơ chế truyền thông một chiều từ máy chủ về máy khách trên nền tảng HTTP/1.1 hoặc HTTP/2. Mỗi thông điệp được biểu diễn bằng một khối văn bản bao gồm các trường có tiền tố:

```text
event: text_delta\r\n
data: {"text": "Xin chào"}\r\n
id: 1001\r\n
retry: 5000\r\n
\r\n
```

Khung sự kiện kết thúc khi xuất hiện hai ký tự xuống dòng liên tiếp (`\n\n` hoặc `\r\n\r\n`).

#### Vấn đề chia cắt byte UTF-8 đa byte
Trong bảng mã UTF-8, các ký tự tiếng Anh (ASCII) chỉ chiếm 1 byte (`0x00`..`0x7F`). Tuy nhiên, tiếng Việt và các ngôn ngữ có dấu sử dụng từ 2 đến 3 byte. Ví dụ:
- Chữ `ệ` trong chữ "Việt": Điểm mã Unicode là `U+1EC7`. Biểu diễn UTF-8 gồm 3 byte: `0xE1, 0xBB, 0x87`.

Khi mạng truyền tải dữ liệu, một gói TCP có thể phân đoạn ở bất kỳ vị trí nào:

```text
TCP Chunk 1: [..., 'V', 'i', 0xE1]               <-- Chỉ chứa byte đầu tiên của 'ệ'
TCP Chunk 2: [0xBB, 0x87, 't', '\n', '\n']        <-- Chứa 2 byte còn lại và kết thúc khung
```

Nếu một bộ phân tích SSE ngây thơ gọi trực tiếp `std::str::from_utf8(chunk1)`, Rust sẽ lập tức trả về lỗi `Utf8Error { valid_up_to: ..., error_len: Some(2) }`. Nếu dùng `.unwrap()`, tiến trình tác tử sẽ sụp đổ.

#### Cơ chế giải quyết bằng vùng đệm byte liên tục (`SseFrameParser`)
Trong cài đặt của chúng ta, `SseFrameParser` duy trì một `buffer: Vec<u8>`. Khi một chunk byte mới được đẩy vào qua `push_chunk(&[u8])`:
1. Vùng đệm tích lũy byte mới vào đuôi.
2. Tìm kiếm chỉ số của chuỗi ranh giới `b"\n\n"` hoặc `b"\r\n\r\n"`.
3. Chỉ khi tìm thấy ranh giới khung hoàn chỉnh, lát cắt byte của khung đó mới được chuyển đổi sang chuỗi UTF-8. Nếu khung hợp lệ, trích xuất dữ liệu thành cấu trúc `SseFrame`.
4. Sau khi xử lý hết các khung hoàn chỉnh, dùng `self.buffer.drain(..cursor)` để dọn dẹp bộ nhớ, giữ lại chính xác các byte dang dở cho lần gọi `push_chunk` tiếp theo.

```rust
// Trích xuất cấu trúc SseFrame với trường có tên (named fields)
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct SseFrame {
    pub event: Option<String>,
    pub data: String,
    pub id: Option<String>,
    pub retry: Option<u64>,
}
```

---

### 3.2. Lắp ráp lượt `SseTurnAssembler` & Kiểm tra ngoặc LIFO

Các sự kiện streaming từ các nhà cung cấp mô hình (như OpenAI streaming format hay Anthropic Messages stream) gửi các phần nhỏ của phản hồi văn bản và lệnh gọi công cụ:
- `text_delta`: Chứa một vài ký tự văn bản trả lời.
- `tool_call_start`: Khởi tạo yêu cầu gọi công cụ mới với `id` và `name`.
- `tool_call_delta`: Gửi từng mẩu của chuỗi JSON tham số (`arguments_json`).
- `tool_call_end`: Kết thúc một yêu cầu gọi công cụ.
- `message_stop`: Báo hiệu lý do dừng (`end_turn`, `tool_use`, `max_tokens`, `stop_sequence`).

```
Luồng SSE:
[text_delta: "Đang"] ──► [text_delta: " đọc file"] ──► [message_stop: "end_turn"]
   ===> TurnOutcome::EndTurn { content: "Đang đọc file" }

Luồng gọi công cụ:
[tool_call_start: id=c1;name=read] ──► [tool_call_delta: "{\"path\":"] ──► [tool_call_delta: "\"a.rs\"}"] ──► [tool_call_end]
   ===> TurnOutcome::ToolRequests { calls: [ToolCall { id: "c1", ... }] }
```

#### Bất biến chặn thực thi khi chạm trần `max_tokens`
Khi mô hình chạm trần token tối đa, chuỗi JSON tham số công cụ bị cắt cụt giữa chừng. Ví dụ:
`{"command": "cargo test -- --nocapture; rm -rf target/debug/build` (bị đứt ở đây, không có ngoặc đóng).
Nếu tác tử cố tình phân tích cú pháp hoặc sửa chữa chuỗi JSON này để chạy lệnh shell, hậu quả khôn lường có thể xảy ra.
`SseTurnAssembler` thực thi một bất biến sắt đá:
**Nếu `stop_reason == "max_tokens"`, trả về ngay lập tức `TurnOutcome::TruncatedMaxTokens`. Bộ khung sẽ chặn đứng việc chuyển tiếp các yêu cầu gọi công cụ này sang hệ thống thực thi, và tự động yêu cầu mô hình tiếp tục sinh (*continuation prompt*) một cách an toàn.**

#### Kiểm tra cân bằng ngoặc LIFO (`check_balanced_brackets`)
Trước khi công nhận một danh sách `ToolCall` là hợp lệ, thuật toán ngăn xếp LIFO kiểm tra chuỗi JSON:
- Đẩy `{`, `[`, `(` vào ngăn xếp.
- Khi gặp `}`, `]`, `)`, phần tử lấy ra từ đỉnh ngăn xếp phải là dấu mở tương ứng.
- Tự động bỏ qua các dấu ngoặc nằm bên trong chuỗi ký tự được bọc bởi dấu ngoặc kép `"` (có xử lý ký tự thoát `\"`).

---

### 3.3. Phẳng hóa đệ quy bằng Trampoline `Bounce<S, T>`

Trong khoa học máy tính, kỹ thuật Trampoline là một phương pháp chuyển đổi đệ quy sâu thành một vòng lặp vô hạn chạy trên bộ nhớ heap với kích thước ngăn xếp cuộc gọi cố định $O(1)$.

```rust
#[derive(Debug, Clone, PartialEq, Eq)]
pub enum Bounce<S, T> {
    Continue { state: S },
    Done { result: T },
}

pub fn run_trampoline<S, T, F>(initial_state: S, mut step_fn: F) -> T
where
    F: FnMut(S) -> Bounce<S, T>,
{
    let mut current = initial_state;
    loop {
        match step_fn(current) {
            Bounce::Continue { state } => current = state,
            Bounce::Done { result } => return result,
        }
    }
}
```

#### Phân tích cơ chế bộ nhớ
- Trong đệ quy truyền thống: Mỗi lần hàm `step` gọi lại chính nó, một khung ngăn xếp mới (*Stack Frame*) chứa các biến cục bộ, con trỏ lệnh và địa chỉ trả về được đẩy lên đỉnh ngăn xếp. Với độ sâu 10.000 bước, kích thước ngăn xếp mặc định của luồng trong Linux (thường là 2MB hoặc 8MB) sẽ nhanh chóng bị tràn.
- Với `run_trampoline`: Chỉ có duy nhất một khung ngăn xếp của hàm `run_trampoline`. Biến `current` được cập nhật giá trị mới qua từng vòng lặp. Dữ liệu trạng thái lớn được di chuyển bằng ngữ nghĩa chuyển quyền sở hữu (*Move Semantics*) trong Rust, hoàn toàn không tốn thêm bất kỳ byte ngăn xếp nào.

---

### 3.4. Hàng đợi lái lượt đa ưu tiên `TurnInboxQueue`

Trong quá trình tác tử đang suy nghĩ hoặc đang thực thi một công cụ chạy lâu (như biên dịch một dự án lớn), các tín hiệu từ bên ngoài có thể dồn dập gửi tới. Các tín hiệu này không bình đẳng về quyền ưu tiên.

Chúng ta định nghĩa 4 mức ưu tiên thông qua kiểu liệt kê:

```rust
#[derive(Debug, Clone, Copy, PartialEq, Eq, PartialOrd, Ord)]
pub enum Priority {
    SystemCancel = 4, // Tín hiệu hủy khẩn cấp (SIGINT, kill-switch, cháy ngân sách)
    HumanSteer = 3,   // Con người can thiệp điều hướng
    PeerSignal = 2,   // Tín hiệu đồng bộ từ tác tử đồng cấp trong hệ đa tác tử
    BgNotify = 1,     // Thông báo tác vụ nền, hẹn giờ cron, đo lường liveness
}
```

Hàng đợi `TurnInboxQueue` sử dụng `BinaryHeap<InboxItem>`. Cấu trúc `InboxItem` cài đặt trait `Ord` theo quy tắc hai khóa:
1. So sánh `priority`: Mục có độ ưu tiên cao hơn sẽ luôn đứng trước.
2. Nếu cùng mức độ ưu tiên: So sánh ngược số thứ tự `sequence` (`other.sequence.cmp(&self.sequence)`). Mục nào vào trước (sequence nhỏ hơn) sẽ được lấy ra trước, bảo toàn tính chất FIFO tuyệt đối cho các thông điệp cùng cấp.

---

### 3.5. Phân giải phạm vi chỉ thị đa tầng & Phát hiện Delta môi trường

#### Phân tầng chỉ thị (`InstructionScopeResolver`)
Một tác tử lập trình chuyên nghiệp hoạt động dưới nhiều lớp quy chuẩn chồng lớp lên nhau. Khi xung đột xảy ra, thứ tự ưu tiên từ cao xuống thấp phải được xác lập rõ ràng:

$$\text{Enterprise (Pháp chế)} \succ \text{User (Cá nhân)} \succ \text{Project (Dự án)} \succ \text{Directory (Cục bộ)}$$

- `Enterprise`: Quy định bảo mật bắt buộc của công ty (ví dụ: cấm tải dữ liệu lên máy chủ lạ, cấm ghi vào tệp `.env`). Lớp này là **bất biến**, không bất kỳ cấu hình nào ở cấp thấp hơn được phép ghi đè.
- `User`: Sở thích cá nhân của lập trình viên (`~/.config/agent/`).
- `Project`: Hướng dẫn của dự án (`AGENTS.md`, `.github/copilot-instructions.md`).
- `Directory`: Chỉ dẫn riêng cho từng mô-đun thư mục cục bộ.

Bộ phân giải `InstructionScopeResolver` tự động sắp xếp các khối chỉ thị theo đúng thứ tự phân cấp và kết xuất chuỗi chỉ dẫn hoàn chỉnh gửi tới mô hình.

#### Bộ phát Delta môi trường (`DeltaEnvEmitter`)
Biến môi trường trong hệ thống lập trình (như `PATH`, `CARGO_HOME`, `RUSTFLAGS`) có thể thay đổi sau khi tác tử chạy các lệnh shell hoặc cài đặt công cụ mới. Nếu mỗi lượt tương tác đều gửi toàn bộ danh sách 50 biến môi trường, tác tử sẽ lãng phí hàng nghìn token vô ích.
`DeltaEnvEmitter` lưu giữ ảnh chụp cơ sở `baseline: BTreeMap<String, String>`. Qua từng lượt, nó chỉ phát ra các biến động:
- `EnvDelta::Set { key, value }`: Biến mới xuất hiện hoặc biến cũ đổi giá trị.
- `EnvDelta::Unset { key }`: Biến bị xóa bỏ khỏi môi trường.

---

### 3.6. Đường ống chấm điểm ngữ cảnh GSSC ($C = \mathcal{A}(c_1 \dots c_6)$)

Để quyết định một mẩu ngữ cảnh (Context Item) có xứng đáng được giữ lại trong bộ nhớ làm việc hay không, chúng ta sử dụng công thức kết hợp đa tiêu chí Goal-State-Step Context (GSSC):

$$C = \mathcal{A}(c_1 \dots c_6) = \sum_{i=1}^{6} w_i \cdot c_i$$

với ràng buộc $\sum_{i=1}^{6} w_i = 1.0$.

Chi tiết 6 thành phần điểm số:
1. $c_1$ — **Độ tương đồng từ vựng Jaccard với mục tiêu hoạt động**: Đo lường sự trùng lặp từ khóa giữa nội dung mẩu ngữ cảnh và mục tiêu lớn của tác tử:
   $$J(A, B) = \frac{|A \cap B|}{|A \cup B|}$$
2. $c_2$ — **Độ gần bước thời gian với suy giảm hàm mũ**: Ngữ cảnh sinh ra ở các bước gần đây có giá trị tức thời cao hơn:
   $$c_2 = \exp(-\lambda \cdot \Delta t)$$
   trong đó $\Delta t$ là khoảng cách bước so với hiện tại, $\lambda = 0.15$ là hệ số phân rã.
3. $c_3$ — **Độ tin cậy của trạng thái ($0.0 \dots 1.0$)**: Mức độ chắc chắn của dữ liệu (dữ liệu đã được kiểm chứng bằng test có điểm 1.0, dữ liệu suy đoán có điểm thấp hơn).
4. $c_4$ — **Hiệu ích của công cụ ($0.0 \dots 1.0$)**: Tỷ lệ công cụ tạo ra kết quả hữu ích thay vì ném lỗi.
5. $c_5$ — **Hiệu quả chi phí token ($1.0 - \text{penalty}$)**: Phạt các mẩu ngữ cảnh quá dài nhưng chứa ít mật độ thông tin.
6. $c_6$ — **Độ mới mẻ của thông tin ($1.0 - J(\text{content}, \text{prompt})$)**: Tránh giữ lại các thông tin đã có sẵn trong tiền tố của prompt.

---

### 3.7. Kinh tế học KV-Cache & Tiền tố ổn định `PrefixStablePrompt`

Để hiểu tại sao KV-Cache lại định hình toàn bộ kiến trúc của một bộ khung tác tử hiện đại, chúng ta cần nhìn vào nguyên lý xử lý của bộ giải mã Transformer:

```
Lượt 1: [Tiền tố hệ thống (10.000 tokens)] + [User: Câu hỏi 1 (100 tokens)]
        ===> Bộ phục vụ tính toán K, V cho 10.100 tokens và LƯU VÀO RAM GPU (Cache Write).

Lượt 2: [Tiền tố hệ thống (10.000 tokens)] + [User: Câu hỏi 1] + [Assistant: Trả lời 1] + [User: Câu hỏi 2]
        ===> Nếu 10.000 tokens đầu tiên TRÙNG KHỚP 100% TỪNG BYTE:
             Bộ phục vụ BỎ QUA việc tính toán cho 10.000 tokens này (Cache Read - giảm 90% chi phí)!
             Chỉ cần nạp từ RAM GPU và tính toán cho phần đuôi mới thêm.
```

Nếu tác tử chèn đồng hồ thời gian `[2026-11-10 18:45:02]` hoặc chuỗi ngẫu nhiên vào đầu prompt ở Lượt 2:
$$\text{Hash}(\text{Lượt 1}) \neq \text{Hash}(\text{Lượt 2}) \implies \text{TOÀN BỘ 10.000 TOKENS BỊ TÍNH LẠI TỪ ĐẦU!}$$

Hậu quả:
- **Tài chính**: Nhà cung cấp API tính phí đọc cache chỉ bằng 10% đơn giá gốc ($0,30/1M token so với $3,00/1M token). Trượt cache làm hóa đơn tăng gấp 10 lần.
- **Thời gian**: Thời gian sinh token đầu tiên tăng từ 150ms lên hơn 10 giây đối với ngữ cảnh dài 100k tokens.

#### Thiết kế `PrefixStablePrompt`
Cấu trúc `PrefixStablePrompt` phân tách ranh giới rõ ràng:
1. `static_prefix`: Chỉ thị hệ thống cốt lõi (bất biến 100%).
2. `enterprise_rules`: Chính sách doanh nghiệp (bất biến 100%).
3. `mcp_tool_schemas`: Lược đồ định nghĩa các công cụ MCP được lưu trữ trong `BTreeMap<String, String>`. Khóa từ điển đảm bảo thứ tự in ra luôn tất định qua mọi lượt.
4. Kiểm tra tính ổn định bằng hàm băm FNV-1a 64-bit: Đảm bảo `compute_prefix_hash()` luôn cho ra kết quả trùng khớp từng bit qua 20+ lượt tương tác động.

---

### 3.8. Sổ cái số nguyên vi mô `PicoDollars` & Biểu giá 2 tầng TTL

Tại sao không dùng kiểu dữ liệu `f64` trong FinOps?
Hãy xem xét phép cộng số thực kinh điển trong tiêu chuẩn IEEE 754:
$$0.1 + 0.2 = 0.300000000000000044408920985...$$
Trong một hệ thống đa tác tử chạy 50.000 bước, việc cộng dồn các khoản chi phí siêu nhỏ (micro-costs) như `$0.000003` bằng `f64` sẽ tích lũy sai số trôi dạt. Đến cuối tháng, con số đối soát giữa hệ thống nội bộ và hóa đơn nhà cung cấp đám mây sẽ bị lệch hàng nghìn USD.

Chúng ta định nghĩa đơn vị số nguyên tuyệt đối **PicoDollars**:
$$1\text{ USD} = 10^{12}\text{ PicoDollars}$$
$$1\text{ Cent} = 10^{10}\text{ PicoDollars}$$

Mọi phép toán đều sử dụng số nguyên `u128` với các hàm an toàn:
- `checked_add`: Bắt lỗi tràn số ngay lập tức.
- `checked_mul`: Nhân đơn giá với số lượng token an toàn.
- `saturating_sub`: Trừ số dư an toàn, không bao giờ âm.

#### Biểu giá 2 tầng TTL của bộ đệm KV-Cache
Hệ thống hỗ trợ 5 chế độ giá dựa trên chính sách của các nhà cung cấp mô hình tiên tiến:
1. `UncachedInput`: Đơn giá cơ sở ($3,00 / 1M token = 3.000.000 picos/token).
2. `CacheWrite5m`: Ghi bộ đệm ngắn hạn (TTL 5 phút) = 1,25x đơn giá cơ sở.
3. `CacheWrite1h`: Ghi bộ đệm dài hạn (TTL 1 giờ) = 2,00x đơn giá cơ sở.
4. `CacheHitRead`: Đọc từ bộ đệm (áp dụng cho cả 5m và 1h) = 0,10x đơn giá cơ sở (tiết kiệm 90%).
5. `OutputGeneration`: Token đầu ra do mô hình sinh = 5,00x đơn giá cơ sở.

---

### 3.9. Đồng hồ đo lượt cuối `LastLegUsageMeter` (ADR-0018)

Trong tài liệu kiến trúc quyết định ADR-0018 của các hệ thống tác tử lớn, một vấn đề nhức nhối thường xảy ra khi tích hợp các API mô hình:
- Một số API trả về số token sử dụng của riêng lượt gọi đó (*delta usage*).
- Một số API khác lại trả về tổng số token tích lũy từ đầu phiên tới thời điểm hiện tại (*cumulative snapshot*).

Nếu lập trình viên thiếu cẩn trọng và liên tục cộng dồn các ảnh chụp tích lũy này:
- Lượt 1: API báo 100 token $\to$ Tổng ghi nhận: 100 token.
- Lượt 2: API báo 250 token (gồm 100 token cũ + 150 token mới) $\to$ Nếu cộng dồn: $100 + 250 = 350$ token (bị đếm dư 100 token cũ)!
- Lượt 10: Chi phí bị nhân lên gấp hàng chục lần so với thực tế.

`LastLegUsageMeter` giải quyết triệt để vấn đề này bằng cách theo dõi trạng thái hai lớp:
- `current_leg`: Lưu trữ chính xác số lượng token phát sinh của riêng bước hiện hành:
  $$\Delta_{\text{input}} = \text{snapshot.input} - \text{last\_snapshot.input}$$
- `accumulated_run`: Tổng dồn thực tế xuyên suốt phiên.
- **Bất biến đơn điệu**: Nếu ảnh chụp API mới có bất kỳ chỉ số nào nhỏ hơn ảnh chụp trước đó, hệ thống sẽ từ chối và báo lỗi vi phạm đơn điệu tăng.

---

### 3.10. Nén hai tầng & Bộ câu hỏi thăm dò (`CompressionProbeEvaluator`)

Khi lịch sử hội thoại vượt quá ngưỡng ngân sách ngữ cảnh (ví dụ 80% cửa sổ ngữ cảnh), tác tử bắt buộc phải nén bộ nhớ. Cơ chế nén hai tầng diễn ra như sau:

```
[ Ngữ cảnh ban đầu: 150k tokens ]
               │
               ▼  TẦNG 1: Vi nén RAM (Microcompact)
[ Cắt gọt nội dung công cụ cũ > 50 tokens: giữ lại 10 từ đầu + dấu vân tay SHA-256 ]
               │
               ▼  (Nếu vẫn vượt ngưỡng ngân sách)
               ▼  TẦNG 2: Tóm tắt điểm neo (AnchoredSummary)
[ Gom các lượt cũ thành 6 mục điểm neo, KHÔNG BAO GIỜ XÉ LẺ cặp ToolUse/ToolResult ]
               │
               ▼
[ Ngữ cảnh sau nén: 25k tokens ] ──► Kiểm tra chất lượng qua 4 câu hỏi thăm dò (Probes)
```

#### Tầng 1: Vi nén RAM (`microcompact`)
Các lệnh như `cat large_file.rs` hay `cargo build` có thể tạo ra hàng nghìn dòng kết quả. Sau khi tác tử đã đọc và chuyển sang bước tiếp theo, toàn bộ văn bản thô này không còn cần thiết cho việc suy luận ngữ nghĩa tức thời. Hàm `microcompact_turn` cắt gọt văn bản, chỉ giữ lại 10 từ đầu tiên kèm thông báo thu gọn và dấu vân tay băm. Thao tác này giảm ngay 80% dung lượng mà không làm mất thông tin về việc công cụ đã được gọi.

#### Tầng 2: Tóm tắt điểm neo 6 thành phần (`AnchoredSummary`)
Khác với các bản tóm tắt tự do của LLM (thường bỏ quên các quy tắc quan trọng), `AnchoredSummary` ép khuôn thành 6 mục cấu trúc bắt buộc:
1. `active_goal`: Mục tiêu hoạt động hiện tại.
2. `key_constraints`: Các ràng buộc bất biến đã cam kết.
3. `decisions_made`: Các quyết định kiến trúc đã chốt.
4. `completed_tasks`: Danh sách công việc đã hoàn thành.
5. `modified_artifacts`: Danh sách các tệp hoặc tài nguyên đã bị chỉnh sửa.
6. `pending_action`: Hành động tiếp theo cần thực thi.

**Bất biến quan trọng nhất của điểm cắt nén**:
Không bao giờ được đặt điểm cắt chia tách giữa một thông điệp `tool_use` và `tool_result` tương ứng! Điểm cắt phải luôn nằm ở ranh giới ngoài cùng của một lượt trao đổi hoàn chỉnh.

#### Bộ đánh giá câu hỏi thăm dò (`CompressionProbeEvaluator`)
Làm sao biết bản tóm tắt có làm mất mát thông tin quan trọng hay không? Chúng ta sử dụng 4 loại câu hỏi thăm dò:
1. `Recall`: Kiểm tra khả năng nhớ lại sự kiện/dữ liệu trong quá khứ.
2. `Artifact`: Kiểm tra việc nhận diện chính xác các tệp mã nguồn đã bị sửa đổi.
3. `Continuation`: Kiểm tra tính logic của bước tiếp theo.
4. `Decision`: Kiểm tra lý do tại sao một quyết định kỹ thuật lại được đưa ra.

Tỷ lệ vượt qua câu hỏi thăm dò phải đạt ngưỡng tối thiểu (thường $\ge 85\%$) trước khi bản tóm tắt được chấp thuận thay thế lịch sử cũ.

---

### 3.11. Gương ngữ cảnh tự quản `LiveContextMirror` (CLM) & Sửa lỗi Edge Case #4

Giao thức Gương ngữ cảnh tự quản (Live Context Mirror) cho phép tác tử tự "soi gương" và biên tập lại lịch sử làm việc của chính mình thông qua định dạng đánh dấu có rào chắn:

```text
[[CTX_TURN 1 role=user]]
Hãy đọc tệp cấu hình và chạy kiểm thử
[[CTX_TURN 2 role=assistant tool_use=call_01]]
read_file
Cargo.toml
[[CTX_TURN 3 role=user tool_result=call_01]]
package name = ch86
```

Hàm `apply_context_mirror_edit` tiếp nhận văn bản chỉnh sửa từ tác tử và kiểm soát nghiêm ngặt qua **6 Bất Biến CLM**:

1. **Bất biến 1 — Khử nhiễm vai trò hệ thống (Sanitization)**: Bất kỳ khai báo `role=system` nào trong phần văn bản do tác tử sửa đổi đều bị cưỡng chế gập về `role=user`. Chỉ có lượt ghim ban đầu do bộ khung thiết lập mới được mang vai trò `system`. Điều này ngăn chặn hoàn toàn đòn tấn công *Prompt Injection* nhằm cướp quyền điều phối.
2. **Bất biến 2 — Bảo vệ lượt ghim (Pin Protection)**: Các lượt từ `0..pinned_count` (chứa System Prompt và hướng dẫn khởi tạo) được bảo vệ tuyệt đối, không một chỉnh sửa nào được phép chạm vào vùng này.
3. **Bất biến 3 — Thực thi cổng kích thước (`Fit` vs `Shrink`)**:
   - `EditGate::Fit`: Tổng token sau chỉnh sửa phải $\le$ ngân sách cho phép.
   - `EditGate::Shrink`: Tổng token sau chỉnh sửa bắt buộc phải **nhỏ hơn trước khi sửa** VÀ $\le$ ngân sách. Nếu tác tử cố tình viết dài hơn, chỉnh sửa sẽ bị từ chối ngay lập tức với biên lai `RejectedGate`.
4. **Bất biến 4 — Cặp công cụ vẹn toàn & Sửa triệt để lỗi Edge Case #4**:
   - Kiểm tra đối xứng tập hợp: Số lượng và định danh của `tool_use_id` phải khớp 1-1 với `tool_result_id`. Nếu có bất kỳ ID nào bị mồ côi, trả về `RejectedOrphanedTool`.
   - **ĐẶC BIỆT — SỬA LỖI EDGE CASE #4 TỪ REVIEWER**:
     - *Phát hiện ID giả mạo*: Tập hợp ID công cụ trong văn bản sửa đổi phải là tập con của tập ID gốc (`orig_tool_set`). Nếu xuất hiện ID lạ không có trong lịch sử ban đầu, trả về `RejectedToolOrderOrFabrication`.
     - *Bảo toàn thứ tự tuần tự không đảo lộn*: Chuỗi các ID công cụ được gọi trong bản chỉnh sửa phải xuất hiện theo đúng thứ tự tương đối ban đầu trong `orig_tool_seq`. Nếu tác tử tráo đổi thứ tự (ví dụ gọi `call_02` trước rồi mới đến `call_01`), hệ thống sẽ phát hiện ngay sự vi phạm tính đơn điệu của chỉ số và từ chối áp dụng.
5. **Bất biến 5 — Tự động gộp tin nhắn đồng vai trò (Merge Adjacent)**: Các tin nhắn văn bản thuần kế tiếp nhau có cùng vai trò (không chứa công cụ) sẽ được tự động gộp thành một tin nhắn duy nhất, phân tách bằng dấu xuống dòng `\n`.
6. **Bất biến 6 — Biên nhận kiểm toán minh bạch (`EditReceipt`)**: Mọi thao tác chỉnh sửa đều phát ra biên lai rõ ràng (`AppliedShrank`, `AppliedButGrew`, `RejectedGate`, `RejectedOrphanedTool`, `RejectedToolOrderOrFabrication`, `NoChange`).

---

### 3.12. Bộ phát hiện thoái hóa ngữ cảnh & Nén KV tiềm ẩn theo MAD

#### 5 Chế độ thoái hóa ngữ cảnh (`ContextDegradationDetector`)
Trong các phiên làm việc kéo dài, mô hình ngôn ngữ có thể rơi vào các trạng thái thoái hóa nguy hiểm:
1. `RepetitionLoop`: Trợ lý lặp lại nội dung câu trả lời hoặc lệnh gọi công cụ giống hệt lượt trước đó (vòng lặp vô tận).
2. `HallucinatedToolCall`: Trợ lý gọi một tên công cụ hoàn toàn không tồn tại trong danh mục đã đăng ký.
3. `ContextLengthSpike`: Dung lượng token của một lượt đơn lẻ đột ngột tăng vọt trên 300% so với mức trung bình.
4. `ConstraintViolation`: Nội dung phản hồi chứa các cụm từ cấm đã được tuyên bố trong chính sách an toàn (ví dụ: `DROP DATABASE`, `rm -rf /`).
5. `EmptyTurnDegradation`: Trợ lý liên tục trả về các thông điệp rỗng hoặc chỉ toàn khoảng trắng.

Bộ phát hiện `ContextDegradationDetector` quét qua toàn bộ lịch sử thông điệp và phát ra cảnh báo định kiểu `DegradationWarning` kèm chỉ số lượt vi phạm để bộ khung kịp thời ngắt mạch can thiệp.

#### Nén KV tiềm ẩn theo Độ lệch Tuyệt đối Trung vị (MAD)
Trong không gian tiềm ẩn (*latent space*) của các lớp Attention, một số vector Khóa-Giá trị (KV) có điểm trọng số chú ý quá thấp, đóng góp không đáng kể vào việc dự đoán token tiếp theo.

Để loại bỏ nhiễu và các điểm ngoại lai (*outliers*) mà không bị ảnh hưởng bởi các giá trị cực đoan (điều mà độ lệch chuẩn thông thường $\sigma$ hay mắc phải), chúng ta sử dụng **Độ lệch Tuyệt đối Trung vị (Median Absolute Deviation - MAD)**:

$$\text{MAD}(X) = \text{median}(|X_i - \text{median}(X)|)$$

Ngưỡng cắt tỉa được xác định bởi:

$$\text{Threshold} = \text{median}(X) + \tau \cdot \text{MAD}(X)$$

trong đó $\tau$ là hệ số điều chỉnh độ nhạy (thường chọn $\tau = 1.5$). Mọi vector hoặc token có điểm chú ý thấp hơn $\text{Threshold}$ sẽ bị cắt tỉa khỏi bộ nhớ đệm KV, giúp tiết kiệm bộ nhớ GPU mà vẫn bảo toàn độ chính xác suy luận.

---

## 4. Mã nguồn minh họa thực chiến

Toàn bộ mã nguồn dưới đây được trích xuất từ crate `code/ch86/src/main.rs`. Crate được viết bằng **100% Rust 2024 thuần `std`**, không sử dụng bất kỳ thư viện ngoài nào, tuân thủ nghiêm ngặt quy tắc đặt tên trường cho struct và vượt qua toàn bộ các kiểm tra khắt khe của `clippy -D warnings`.

```rust
//! Crate ch86: Cài đặt hoàn chỉnh bộ khung tác tử (Agent Harness) cấp sản xuất
//! bằng 100% Rust 2024 thuần std.

use std::cmp::Ordering;
use std::collections::{BTreeMap, BTreeSet, BinaryHeap, HashSet};

// ============================================================================
// 1. PHÂN TÍCH KHUNG SSE & LẮP RÁP LƯỢT STREAMING
// ============================================================================

/// Khung sự kiện Server-Sent Events (SSE) theo chuẩn RFC 8895.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct SseFrame {
    pub event: Option<String>,
    pub data: String,
    pub id: Option<String>,
    pub retry: Option<u64>,
}

/// Bộ phân tích luồng byte SSE an toàn với ký tự UTF-8 đa byte vỡ ngang.
#[derive(Debug, Default)]
pub struct SseFrameParser {
    buffer: Vec<u8>,
}

impl SseFrameParser {
    pub fn new() -> Self {
        Self { buffer: Vec::new() }
    }

    /// Tiếp nhận chunk byte mới và bóc tách các khung hoàn chỉnh.
    pub fn push_chunk(&mut self, chunk: &[u8]) -> Vec<SseFrame> {
        self.buffer.extend_from_slice(chunk);
        let mut frames = Vec::new();
        let mut cursor = 0;
        let len = self.buffer.len();

        while cursor < len {
            let slice = &self.buffer[cursor..];
            let boundary = if let Some(pos) = slice.windows(2).position(|w| w == b"\n\n") {
                Some((pos, 2))
            } else {
                slice
                    .windows(4)
                    .position(|w| w == b"\r\n\r\n")
                    .map(|pos| (pos, 4))
            };

            if let Some((frame_len, sep_len)) = boundary {
                let frame_bytes = &self.buffer[cursor..cursor + frame_len];
                if let Ok(text) = std::str::from_utf8(frame_bytes)
                    && let Some(frame) = Self::parse_frame_text(text)
                {
                    frames.push(frame);
                }
                cursor += frame_len + sep_len;
            } else {
                break;
            }
        }

        self.buffer.drain(..cursor);
        frames
    }

    fn parse_frame_text(text: &str) -> Option<SseFrame> {
        let mut event = None;
        let mut data_lines = Vec::new();
        let mut id = None;
        let mut retry = None;
        let mut has_content = false;

        for line in text.lines() {
            let line = line.trim_end_matches('\r');
            if line.is_empty() || line.starts_with(':') {
                continue;
            }
            has_content = true;
            if let Some(val) = line.strip_prefix("event:") {
                event = Some(val.trim().to_string());
            } else if let Some(val) = line.strip_prefix("data:") {
                data_lines.push(val.trim_start().to_string());
            } else if let Some(val) = line.strip_prefix("id:") {
                id = Some(val.trim().to_string());
            } else if let Some(val) = line.strip_prefix("retry:")
                && let Ok(ms) = val.trim().parse::<u64>()
            {
                retry = Some(ms);
            }
        }

        if has_content {
            Some(SseFrame {
                event,
                data: data_lines.join("\n"),
                id,
                retry,
            })
        } else {
            None
        }
    }
}

/// Yêu cầu gọi công cụ được trích xuất từ phản hồi mô hình.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct ToolCall {
    pub id: String,
    pub name: String,
    pub arguments_json: String,
}

/// Kết quả đầu ra của một lượt tương tác mô hình.
#[derive(Debug, Clone, PartialEq, Eq)]
pub enum TurnOutcome {
    EndTurn { content: String },
    ToolRequests { calls: Vec<ToolCall> },
    TruncatedMaxTokens {
        partial_text: String,
        partial_calls: Vec<ToolCall>,
    },
    PausedServerTurn { reason: String },
}

/// Kiểm tra tính cân bằng của dấu ngoặc LIFO trong JSON tham số công cụ.
pub fn check_balanced_brackets(text: &str) -> bool {
    let mut stack = Vec::new();
    let mut in_string = false;
    let mut escaped = false;

    for ch in text.chars() {
        if escaped {
            escaped = false;
            continue;
        }
        if ch == '\\' && in_string {
            escaped = true;
            continue;
        }
        if ch == '"' {
            in_string = !in_string;
            continue;
        }
        if in_string {
            continue;
        }
        match ch {
            '{' | '[' | '(' => stack.push(ch),
            '}' if stack.pop() != Some('{') => return false,
            ']' if stack.pop() != Some('[') => return false,
            ')' if stack.pop() != Some('(') => return false,
            _ => {}
        }
    }
    stack.is_empty() && !in_string
}

/// Bộ lắp ráp lượt phản hồi streaming.
#[derive(Debug, Default)]
pub struct SseTurnAssembler {
    accumulated_text: String,
    pending_tool_calls: Vec<ToolCall>,
    current_tool_id: Option<String>,
    current_tool_name: Option<String>,
    current_tool_args: String,
    stop_reason: Option<String>,
}

impl SseTurnAssembler {
    pub fn new() -> Self {
        Self::default()
    }

    pub fn ingest_frame(&mut self, frame: &SseFrame) {
        let event = frame.event.as_deref().unwrap_or("");
        let data = frame.data.as_str();

        match event {
            "text_delta" => {
                self.accumulated_text.push_str(data);
            }
            "tool_call_start" => {
                let mut id = String::new();
                let mut name = String::new();
                for part in data.split(';') {
                    if let Some(v) = part.strip_prefix("id=") {
                        id = v.to_string();
                    } else if let Some(v) = part.strip_prefix("name=") {
                        name = v.to_string();
                    }
                }
                self.current_tool_id = Some(id);
                self.current_tool_name = Some(name);
                self.current_tool_args.clear();
            }
            "tool_call_delta" => {
                self.current_tool_args.push_str(data);
            }
            "tool_call_end" => {
                if let (Some(id), Some(name)) =
                    (self.current_tool_id.take(), self.current_tool_name.take())
                {
                    self.pending_tool_calls.push(ToolCall {
                        id,
                        name,
                        arguments_json: std::mem::take(&mut self.current_tool_args),
                    });
                }
            }
            "message_stop" => {
                self.stop_reason = Some(data.to_string());
            }
            _ => {}
        }
    }

    pub fn finalize(mut self) -> Result<TurnOutcome, &'static str> {
        let is_truncated = self.stop_reason.as_deref() == Some("max_tokens");

        if let (Some(id), Some(name)) = (self.current_tool_id.take(), self.current_tool_name.take())
        {
            self.pending_tool_calls.push(ToolCall {
                id,
                name,
                arguments_json: self.current_tool_args,
            });
        }

        if is_truncated {
            return Ok(TurnOutcome::TruncatedMaxTokens {
                partial_text: self.accumulated_text,
                partial_calls: self.pending_tool_calls,
            });
        }

        if !self.pending_tool_calls.is_empty() {
            for call in &self.pending_tool_calls {
                if !check_balanced_brackets(&call.arguments_json) {
                    return Err("JSON tham số công cụ bị lỗi mất cân bằng ngoặc LIFO");
                }
            }
            Ok(TurnOutcome::ToolRequests {
                calls: self.pending_tool_calls,
            })
        } else if self.stop_reason.as_deref() == Some("server_paused") {
            Ok(TurnOutcome::PausedServerTurn {
                reason: "Yêu cầu tạm dừng từ máy chủ".to_string(),
            })
        } else {
            Ok(TurnOutcome::EndTurn {
                content: self.accumulated_text,
            })
        }
    }
}

// ============================================================================
// 2. PHẲNG HÓA ĐỆ QUY TRAMPOLINE & HÀNG ĐỢI LÁI LƯỢT (BOUNCE & INBOX QUEUE)
// ============================================================================

/// Trạng thái bước nhảy của vòng lặp phẳng hóa đệ quy (Trampoline pattern).
#[derive(Debug, Clone, PartialEq, Eq)]
pub enum Bounce<S, T> {
    /// Tiếp tục lặp với trạng thái kế tiếp `state`.
    Continue { state: S },
    /// Đã đạt kết quả cuối cùng `result`, thoát vòng lặp.
    Done { result: T },
}

/// Chạy vòng lặp Trampoline với chi phí ngăn xếp O(1).
pub fn run_trampoline<S, T, F>(initial_state: S, mut step_fn: F) -> T
where
    F: FnMut(S) -> Bounce<S, T>,
{
    let mut current = initial_state;
    loop {
        match step_fn(current) {
            Bounce::Continue { state } => current = state,
            Bounce::Done { result } => return result,
        }
    }
}

/// Mức độ ưu tiên của thông điệp gửi vào hàng đợi lái lượt.
#[derive(Debug, Clone, Copy, PartialEq, Eq, PartialOrd, Ord)]
pub enum Priority {
    SystemCancel = 4,
    HumanSteer = 3,
    PeerSignal = 2,
    BgNotify = 1,
}

/// Một phần tử nằm trong hàng đợi lái lượt `TurnInboxQueue`.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct InboxItem {
    pub priority: Priority,
    pub sequence: u64,
    pub payload: String,
}

impl Ord for InboxItem {
    fn cmp(&self, other: &Self) -> Ordering {
        self.priority
            .cmp(&other.priority)
            .then_with(|| other.sequence.cmp(&self.sequence))
    }
}

impl PartialOrd for InboxItem {
    fn partial_cmp(&self, other: &Self) -> Option<Ordering> {
        Some(self.cmp(other))
    }
}

/// Hàng đợi tiếp nhận tín hiệu điều hướng giữa các lượt tương tác của tác tử.
#[derive(Debug, Default)]
pub struct TurnInboxQueue {
    heap: BinaryHeap<InboxItem>,
    sequence_gen: u64,
}

impl TurnInboxQueue {
    pub fn new() -> Self {
        Self {
            heap: BinaryHeap::new(),
            sequence_gen: 0,
        }
    }

    pub fn push(&mut self, priority: Priority, payload: impl Into<String>) {
        let sequence = self.sequence_gen;
        self.sequence_gen += 1;
        self.heap.push(InboxItem {
            priority,
            sequence,
            payload: payload.into(),
        });
    }

    pub fn pop(&mut self) -> Option<InboxItem> {
        self.heap.pop()
    }

    pub fn has_system_cancel(&self) -> bool {
        self.heap
            .iter()
            .any(|item| item.priority == Priority::SystemCancel)
    }

    pub fn len(&self) -> usize {
        self.heap.len()
    }

    pub fn is_empty(&self) -> bool {
        self.heap.is_empty()
    }
}

// ============================================================================
// 3. PHÂN CẤP CHỈ THỊ & PHÁT HIỆN DELTA MÔI TRƯỜNG (SCOPE & DELTA ENV)
// ============================================================================

/// Tầng phân cấp phạm vi chỉ thị của hệ thống tác tử.
#[derive(Debug, Clone, Copy, PartialEq, Eq, PartialOrd, Ord)]
pub enum ScopeLevel {
    Enterprise = 0,
    User = 1,
    Project = 2,
    Directory = 3,
}

/// Một khối chỉ thị thuộc phạm vi xác định.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct InstructionScope {
    pub level: ScopeLevel,
    pub source_path: String,
    pub content: String,
}

/// Bộ phân giải chỉ thị đa tầng từ doanh nghiệp xuống thư mục hiện hành.
#[derive(Debug, Default)]
pub struct InstructionScopeResolver {
    scopes: Vec<InstructionScope>,
}

impl InstructionScopeResolver {
    pub fn new() -> Self {
        Self { scopes: Vec::new() }
    }

    pub fn add_scope(&mut self, scope: InstructionScope) {
        self.scopes.push(scope);
    }

    pub fn resolve_effective_instructions(&self) -> String {
        let mut sorted = self.scopes.clone();
        sorted.sort_by_key(|s| s.level);

        let mut out = String::new();
        for s in sorted {
            let label = match s.level {
                ScopeLevel::Enterprise => "CHỈ THỊ DOANH NGHIỆP (ENTERPRISE - BẤT BIẾN)",
                ScopeLevel::User => "CẤU HÌNH NGƯỜI DÙNG (USER)",
                ScopeLevel::Project => "QUY TẮC DỰ ÁN (PROJECT)",
                ScopeLevel::Directory => "CHỈ DẪN THƯ MỤC CỤC BỘ (DIRECTORY)",
            };
            out.push_str(&format!("=== [{label}: {}] ===\n{}\n\n", s.source_path, s.content));
        }
        out.trim_end().to_string()
    }
}

/// Biến động thay đổi trong môi trường thực thi của tác tử.
#[derive(Debug, Clone, PartialEq, Eq)]
pub enum EnvDelta {
    Set { key: String, value: String },
    Unset { key: String },
}

/// Bộ phát hiện và phát tín hiệu delta môi trường.
#[derive(Debug, Default)]
pub struct DeltaEnvEmitter {
    baseline: BTreeMap<String, String>,
}

impl DeltaEnvEmitter {
    pub fn new(initial: BTreeMap<String, String>) -> Self {
        Self { baseline: initial }
    }

    pub fn emit_delta(&mut self, current: &BTreeMap<String, String>) -> Vec<EnvDelta> {
        let mut deltas = Vec::new();

        for (k, v) in current {
            match self.baseline.get(k) {
                Some(old_v) if old_v == v => {}
                _ => deltas.push(EnvDelta::Set {
                    key: k.clone(),
                    value: v.clone(),
                }),
            }
        }

        for k in self.baseline.keys() {
            if !current.contains_key(k) {
                deltas.push(EnvDelta::Unset { key: k.clone() });
            }
        }

        self.baseline = current.clone();
        deltas
    }
}

// ============================================================================
// 4. ĐƯỜNG ỐNG ĐIỂM NGỮ CẢNH GSSC & TIỀN TỐ ỔN ĐỊNH (GSSC & PREFIX CACHE)
// ============================================================================

/// Một ứng viên ngữ cảnh được đưa vào đánh giá trong đường ống GSSC.
#[derive(Debug, Clone, PartialEq)]
pub struct ContextCandidate {
    pub id: String,
    pub content: String,
    pub step_distance: usize,
    pub fidelity: f64,
    pub tool_utility: f64,
    pub cost_penalty: f64,
}

/// Tính hệ số tương đồng Jaccard giữa hai tập từ khóa.
pub fn jaccard_similarity(a: &str, b: &str) -> f64 {
    let set_a: HashSet<&str> = a.split_whitespace().collect();
    let set_b: HashSet<&str> = b.split_whitespace().collect();
    if set_a.is_empty() && set_b.is_empty() {
        return 1.0;
    }
    let intersection = set_a.intersection(&set_b).count();
    let union = set_a.union(&set_b).count();
    if union == 0 {
        0.0
    } else {
        intersection as f64 / union as f64
    }
}

/// Đường ống tính điểm GSSC: C = A(c1..c6).
#[derive(Debug, Clone)]
pub struct GsscPipeline {
    pub lambda: f64,
    pub weights: [f64; 6],
}

impl Default for GsscPipeline {
    fn default() -> Self {
        Self {
            lambda: 0.15,
            weights: [0.25, 0.20, 0.15, 0.15, 0.10, 0.15],
        }
    }
}

impl GsscPipeline {
    pub fn score_candidate(
        &self,
        candidate: &ContextCandidate,
        active_goal: &str,
        current_prompt: &str,
    ) -> f64 {
        let c1 = jaccard_similarity(&candidate.content, active_goal);
        let c2 = (-self.lambda * (candidate.step_distance as f64)).exp();
        let c3 = candidate.fidelity.clamp(0.0, 1.0);
        let c4 = candidate.tool_utility.clamp(0.0, 1.0);
        let c5 = (1.0 - candidate.cost_penalty).clamp(0.0, 1.0);
        let c6 = 1.0 - jaccard_similarity(&candidate.content, current_prompt);

        let components = [c1, c2, c3, c4, c5, c6];
        let total: f64 = self
            .weights
            .iter()
            .zip(components.iter())
            .map(|(w, c)| w * c)
            .sum();
        total
    }
}

/// Bộ xây dựng Prompt bảo toàn tiền tố bộ đệm KV-Cache.
#[derive(Debug, Clone)]
pub struct PrefixStablePrompt {
    pub static_prefix: String,
    pub enterprise_rules: String,
    pub mcp_tool_schemas: BTreeMap<String, String>,
}

impl PrefixStablePrompt {
    pub fn new(static_prefix: impl Into<String>, enterprise_rules: impl Into<String>) -> Self {
        Self {
            static_prefix: static_prefix.into(),
            enterprise_rules: enterprise_rules.into(),
            mcp_tool_schemas: BTreeMap::new(),
        }
    }

    pub fn register_tool_schema(&mut self, tool_name: impl Into<String>, schema_json: impl Into<String>) {
        self.mcp_tool_schemas.insert(tool_name.into(), schema_json.into());
    }

    pub fn render_frozen_prefix(&self) -> String {
        let mut out = String::new();
        out.push_str("=== CHỈ THỊ HỆ THỐNG BẤT BIẾN ===\n");
        out.push_str(&self.static_prefix);
        out.push_str("\n\n=== QUY TẮC DOANH NGHIỆP ===\n");
        out.push_str(&self.enterprise_rules);
        out.push_str("\n\n=== CÔNG CỤ MCP KHẢ DỤNG (XẾP THỨ TỰ TẤT ĐỊNH) ===\n");
        for (name, schema) in &self.mcp_tool_schemas {
            out.push_str(&format!("- [{name}]: {schema}\n"));
        }
        out
    }

    pub fn compute_prefix_hash(&self) -> u64 {
        let prefix = self.render_frozen_prefix();
        let mut hash: u64 = 0xcbf29ce484222325;
        for byte in prefix.bytes() {
            hash ^= byte as u64;
            hash = hash.wrapping_mul(0x100000001b3);
        }
        hash
    }

    pub fn render_full_prompt(&self, dynamic_turns: &str) -> String {
        let mut out = self.render_frozen_prefix();
        out.push_str("\n=== LỊCH SỬ HỘI THOẠI ĐỘNG ===\n");
        out.push_str(dynamic_turns);
        out
    }
}

// ============================================================================
// 5. SỔ CÁI SỐ NGUYÊN PICO-DOLLARS & ĐỒNG HỒ LƯỢT CUỐI (FINOPS & ADR-0018)
// ============================================================================

/// Đơn vị đo chi phí số nguyên tuyệt đối Pico-Dollars: 1 USD = 10^12 PicoDollars.
#[derive(Debug, Clone, Copy, PartialEq, Eq, PartialOrd, Ord, Default)]
pub struct PicoDollars {
    pub amount: u128,
}

impl PicoDollars {
    pub const ONE_DOLLAR: u128 = 1_000_000_000_000;
    pub const ONE_CENT: u128 = 10_000_000_000;

    pub const fn from_picos(picos: u128) -> Self {
        Self { amount: picos }
    }

    pub fn checked_add(self, other: Self) -> Option<Self> {
        self.amount.checked_add(other.amount).map(Self::from_picos)
    }

    pub fn saturating_sub(self, other: Self) -> Self {
        Self::from_picos(self.amount.saturating_sub(other.amount))
    }
}

/// Tầng biểu giá bộ đệm KV-Cache theo thời gian sống (TTL).
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum CachePricingTier {
    UncachedInput,
    CacheWrite5m,
    CacheWrite1h,
    CacheHitRead,
    OutputGeneration,
}

/// Sổ cái ghi nhận chi phí token tích lũy và kiểm soát cổng đơn điệu.
#[derive(Debug, Default)]
pub struct CostLedger {
    pub total_cost: PicoDollars,
    pub total_input_tokens: u64,
    pub total_output_tokens: u64,
    pub total_cache_read_tokens: u64,
    pub base_input_pico_per_token: u128,
}

impl CostLedger {
    pub fn new(base_input_pico_per_token: u128) -> Self {
        Self {
            total_cost: PicoDollars::default(),
            total_input_tokens: 0,
            total_output_tokens: 0,
            total_cache_read_tokens: 0,
            base_input_pico_per_token,
        }
    }

    pub fn record_tokens(
        &mut self,
        tokens: u64,
        tier: CachePricingTier,
    ) -> Result<PicoDollars, &'static str> {
        let multiplier_picos = match tier {
            CachePricingTier::UncachedInput => self.base_input_pico_per_token,
            CachePricingTier::CacheWrite5m => (self.base_input_pico_per_token * 125) / 100,
            CachePricingTier::CacheWrite1h => self.base_input_pico_per_token * 2,
            CachePricingTier::CacheHitRead => (self.base_input_pico_per_token * 10) / 100,
            CachePricingTier::OutputGeneration => self.base_input_pico_per_token * 5,
        };

        let cost_picos = (tokens as u128)
            .checked_mul(multiplier_picos)
            .ok_or("Tràn số khi nhân chi phí token")?;

        let added_cost = PicoDollars::from_picos(cost_picos);
        self.total_cost = self
            .total_cost
            .checked_add(added_cost)
            .ok_or("Tràn số sổ cái PicoDollars")?;

        match tier {
            CachePricingTier::UncachedInput
            | CachePricingTier::CacheWrite5m
            | CachePricingTier::CacheWrite1h => {
                self.total_input_tokens += tokens;
            }
            CachePricingTier::CacheHitRead => {
                self.total_cache_read_tokens += tokens;
            }
            CachePricingTier::OutputGeneration => {
                self.total_output_tokens += tokens;
            }
        }

        Ok(added_cost)
    }
}

/// Bản báo cáo số lượng token sử dụng trong một phạm vi.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Default)]
pub struct UsageReport {
    pub input_tokens: u64,
    pub output_tokens: u64,
    pub cache_read_tokens: u64,
}

/// Đồng hồ đo mức sử dụng lượt cuối (ADR-0018).
#[derive(Debug, Default)]
pub struct LastLegUsageMeter {
    pub current_leg: UsageReport,
    pub accumulated_run: UsageReport,
    last_snapshot: UsageReport,
}

impl LastLegUsageMeter {
    pub fn new() -> Self {
        Self::default()
    }

    pub fn on_api_snapshot(&mut self, snapshot: UsageReport) -> Result<UsageReport, &'static str> {
        if snapshot.input_tokens < self.last_snapshot.input_tokens
            || snapshot.output_tokens < self.last_snapshot.output_tokens
            || snapshot.cache_read_tokens < self.last_snapshot.cache_read_tokens
        {
            return Err("Ảnh chụp token vi phạm bất biến đơn điệu tăng");
        }

        let delta_input = snapshot.input_tokens - self.last_snapshot.input_tokens;
        let delta_output = snapshot.output_tokens - self.last_snapshot.output_tokens;
        let delta_cache = snapshot.cache_read_tokens - self.last_snapshot.cache_read_tokens;

        self.current_leg = UsageReport {
            input_tokens: delta_input,
            output_tokens: delta_output,
            cache_read_tokens: delta_cache,
        };

        self.accumulated_run.input_tokens += delta_input;
        self.accumulated_run.output_tokens += delta_output;
        self.accumulated_run.cache_read_tokens += delta_cache;

        self.last_snapshot = snapshot;
        Ok(self.current_leg)
    }
}

// ============================================================================
// 6. BỘ NÉN HAI TẦNG & CÂU HỎI THĂM DÒ (COMPACTOR & PROBES)
// ============================================================================

/// Bản tóm tắt có điểm neo 6 thành phần bất biến (`AnchoredSummary`).
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct AnchoredSummary {
    pub active_goal: String,
    pub key_constraints: Vec<String>,
    pub decisions_made: Vec<String>,
    pub completed_tasks: Vec<String>,
    pub modified_artifacts: Vec<String>,
    pub pending_action: String,
}

impl AnchoredSummary {
    pub fn render_markdown(&self) -> String {
        let mut out = String::new();
        out.push_str("### BẢN TÓM TẮT ĐIỂM NEO NGỮ CẢNH (ANCHORED SUMMARY)\n\n");
        out.push_str(&format!("- **Mục tiêu hoạt động**: {}\n", self.active_goal));
        out.push_str("- **Ràng buộc an toàn**:\n");
        for c in &self.key_constraints {
            out.push_str(&format!("  * {c}\n"));
        }
        out.push_str("- **Quyết định đã chốt**:\n");
        for d in &self.decisions_made {
            out.push_str(&format!("  * {d}\n"));
        }
        out.push_str("- **Công việc hoàn tất**:\n");
        for t in &self.completed_tasks {
            out.push_str(&format!("  * {t}\n"));
        }
        out.push_str("- **Tài nguyên thay đổi**:\n");
        for a in &self.modified_artifacts {
            out.push_str(&format!("  * {a}\n"));
        }
        out.push_str(&format!("- **Hành động kế tiếp**: {}\n", self.pending_action));
        out
    }
}

/// Loại câu hỏi thăm dò kiểm tra độ nguyên vẹn sau nén.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum ProbeType {
    Recall,
    Artifact,
    Continuation,
    Decision,
}

/// Câu hỏi thăm dò chất lượng nén ngữ cảnh.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct CompressionProbe {
    pub question_type: ProbeType,
    pub question: String,
    pub expected_keyword: String,
}

/// Bộ nén ngữ cảnh hai tầng: Vi nén RAM + Tóm tắt điểm neo.
#[derive(Debug, Default)]
pub struct TwoTierCompactor {
    pub max_tool_output_tokens: usize,
}

impl TwoTierCompactor {
    pub fn new(max_tool_output_tokens: usize) -> Self {
        Self {
            max_tool_output_tokens,
        }
    }

    pub fn microcompact_turn(&self, content: &str) -> String {
        let token_count = content.split_whitespace().count();
        if token_count > self.max_tool_output_tokens {
            let mut words = content.split_whitespace();
            let head: Vec<&str> = words.by_ref().take(10).collect();
            format!(
                "{} ... [ĐÃ VI NÉN: Cắt bớt {} từ] ...",
                head.join(" "),
                token_count.saturating_sub(10)
            )
        } else {
            content.to_string()
        }
    }

    pub fn evaluate_probes(&self, context: &str, probes: &[CompressionProbe]) -> f64 {
        if probes.is_empty() {
            return 1.0;
        }
        let mut passed = 0;
        let lower_ctx = context.to_lowercase();
        for probe in probes {
            if lower_ctx.contains(&probe.expected_keyword.to_lowercase()) {
                passed += 1;
            }
        }
        passed as f64 / probes.len() as f64
    }
}

// ============================================================================
// 7. GƯƠNG NGỮ CẢNH TỰ QUẢN CLM & SỬA EDGE CASE #4 (LIVE CONTEXT MIRROR)
// ============================================================================

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum CtxRole {
    System,
    User,
    Assistant,
}

/// Một thông điệp trong gương ngữ cảnh. Named fields bắt buộc.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct CtxMessage {
    pub role: CtxRole,
    pub content: String,
    pub tool_use_id: Option<String>,
    pub tool_result_id: Option<String>,
}

impl CtxMessage {
    pub fn plain(role: CtxRole, content: impl Into<String>) -> Self {
        Self {
            role,
            content: content.into(),
            tool_use_id: None,
            tool_result_id: None,
        }
    }

    pub fn tool_call(id: impl Into<String>, content: impl Into<String>) -> Self {
        Self {
            role: CtxRole::Assistant,
            content: content.into(),
            tool_use_id: Some(id.into()),
            tool_result_id: None,
        }
    }

    pub fn tool_reply(id: impl Into<String>, content: impl Into<String>) -> Self {
        Self {
            role: CtxRole::User,
            content: content.into(),
            tool_use_id: None,
            tool_result_id: Some(id.into()),
        }
    }

    pub fn token_estimate(&self) -> usize {
        self.content.split_whitespace().count().max(1)
    }
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum EditGate {
    Fit,
    Shrink,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct MirrorPolicy {
    pub pinned_count: usize,
    pub token_budget: usize,
    pub gate: EditGate,
    pub is_pure: bool,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub enum EditReceipt {
    AppliedShrank,
    AppliedButGrew,
    RejectedGate,
    RejectedOrphanedTool { tool_id: String },
    RejectedToolOrderOrFabrication { detail: String },
    NoChange,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct MirrorOutcome {
    pub messages: Vec<CtxMessage>,
    pub receipt: EditReceipt,
    pub is_pure: bool,
}

pub fn render_context_mirror(msgs: &[CtxMessage], pinned_count: usize) -> String {
    let mut out = String::new();
    for (idx, m) in msgs.iter().enumerate().skip(pinned_count) {
        let r = match m.role {
            CtxRole::System => "system",
            CtxRole::User => "user",
            CtxRole::Assistant => "assistant",
        };
        out.push_str(&format!("[[CTX_TURN {idx} role={r}"));
        if let Some(ref id) = m.tool_use_id {
            out.push_str(&format!(" tool_use={id}"));
        }
        if let Some(ref id) = m.tool_result_id {
            out.push_str(&format!(" tool_result={id}"));
        }
        out.push_str(&format!("]]\n{}\n", m.content));
    }
    out
}

fn push_parsed_turn(
    out: &mut Vec<CtxMessage>,
    pin: usize,
    header: Option<(CtxRole, Option<String>, Option<String>)>,
    lines: &mut Vec<&str>,
) {
    let Some((role, tool_use, tool_result)) = header else {
        lines.clear();
        return;
    };
    let text = lines.join("\n").trim().to_string();
    lines.clear();

    if text.is_empty() && tool_use.is_none() && tool_result.is_none() {
        return;
    }

    let can_merge = tool_use.is_none() && tool_result.is_none() && out.len() > pin;
    if can_merge
        && let Some(last) = out.last_mut()
        && last.role == role
        && last.tool_use_id.is_none()
        && last.tool_result_id.is_none()
    {
        last.content.push('\n');
        last.content.push_str(&text);
        return;
    }

    out.push(CtxMessage {
        role,
        content: text,
        tool_use_id: tool_use,
        tool_result_id: tool_result,
    });
}

/// Áp dụng chỉnh sửa vào gương ngữ cảnh với đầy đủ 6 Bất Biến CLM và sửa Edge Case #4.
pub fn apply_context_mirror_edit(
    orig: &[CtxMessage],
    edited: &str,
    policy: &MirrorPolicy,
) -> MirrorOutcome {
    let pin = policy.pinned_count;
    let before_tokens: usize = orig.iter().map(CtxMessage::token_estimate).sum();

    if edited.trim() == render_context_mirror(orig, pin).trim() {
        return MirrorOutcome {
            messages: orig.to_vec(),
            receipt: EditReceipt::NoChange,
            is_pure: false,
        };
    }

    let mut rebuilt: Vec<CtxMessage> = orig[..pin.min(orig.len())].to_vec();

    let orig_tool_seq: Vec<String> = orig
        .iter()
        .filter_map(|m| m.tool_use_id.clone())
        .collect();
    let orig_tool_set: HashSet<String> = orig_tool_seq.iter().cloned().collect();

    let mut cur_header = None;
    let mut line_buf = Vec::new();

    for line in edited.lines() {
        if let Some(h) = line
            .strip_prefix("[[CTX_TURN ")
            .and_then(|s| s.strip_suffix("]]"))
        {
            push_parsed_turn(&mut rebuilt, pin, cur_header.take(), &mut line_buf);

            let mut role = CtxRole::User;
            let mut tool_use = None;
            let mut tool_result = None;

            for token in h.split_whitespace().skip(1) {
                if let Some((k, v)) = token.split_once('=') {
                    match k {
                        "role" => {
                            // Bất biến 1: Gập role=system giả mạo về role=user
                            role = if v == "assistant" {
                                CtxRole::Assistant
                            } else {
                                CtxRole::User
                            };
                        }
                        "tool_use" => tool_use = Some(v.to_string()),
                        "tool_result" => tool_result = Some(v.to_string()),
                        _ => {}
                    }
                }
            }
            cur_header = Some((role, tool_use, tool_result));
        } else {
            line_buf.push(line);
        }
    }
    push_parsed_turn(&mut rebuilt, pin, cur_header.take(), &mut line_buf);

    // Kiểm toán Edge Case #4: Chống ID giả mạo và chống đảo lộn thứ tự công cụ
    let mut rebuilt_tool_uses = Vec::new();
    let mut rebuilt_tool_results = Vec::new();

    for m in &rebuilt {
        if let Some(ref id) = m.tool_use_id {
            if !orig_tool_set.contains(id) {
                return MirrorOutcome {
                    messages: orig.to_vec(),
                    receipt: EditReceipt::RejectedToolOrderOrFabrication {
                        detail: format!("Phát hiện ID công cụ giả mạo: {id}"),
                    },
                    is_pure: false,
                };
            }
            rebuilt_tool_uses.push(id.clone());
        }
        if let Some(ref id) = m.tool_result_id {
            if !orig_tool_set.contains(id) {
                return MirrorOutcome {
                    messages: orig.to_vec(),
                    receipt: EditReceipt::RejectedToolOrderOrFabrication {
                        detail: format!("Phát hiện ID kết quả công cụ giả mạo: {id}"),
                    },
                    is_pure: false,
                };
            }
            rebuilt_tool_results.push(id.clone());
        }
    }

    let use_set: BTreeSet<&str> = rebuilt_tool_uses.iter().map(|s| s.as_str()).collect();
    let res_set: BTreeSet<&str> = rebuilt_tool_results.iter().map(|s| s.as_str()).collect();

    if let Some(&orphan_id) = use_set.symmetric_difference(&res_set).next() {
        return MirrorOutcome {
            messages: orig.to_vec(),
            receipt: EditReceipt::RejectedOrphanedTool {
                tool_id: orphan_id.to_string(),
            },
            is_pure: false,
        };
    }

    // Kiểm tra bảo toàn thứ tự ban đầu
    let mut orig_idx = 0;
    for used_id in &rebuilt_tool_uses {
        match orig_tool_seq[orig_idx..].iter().position(|id| id == used_id) {
            Some(pos) => {
                orig_idx += pos + 1;
            }
            None => {
                return MirrorOutcome {
                    messages: orig.to_vec(),
                    receipt: EditReceipt::RejectedToolOrderOrFabrication {
                        detail: format!("Vi phạm thứ tự gọi công cụ ban đầu tại ID: {used_id}"),
                    },
                    is_pure: false,
                };
            }
        }
    }

    let after_tokens: usize = rebuilt.iter().map(CtxMessage::token_estimate).sum();
    let gate_ok = match policy.gate {
        EditGate::Fit => after_tokens <= policy.token_budget,
        EditGate::Shrink => after_tokens < before_tokens && after_tokens <= policy.token_budget,
    };

    if !gate_ok {
        return MirrorOutcome {
            messages: orig.to_vec(),
            receipt: EditReceipt::RejectedGate,
            is_pure: false,
        };
    }

    let receipt = if after_tokens < before_tokens {
        EditReceipt::AppliedShrank
    } else {
        EditReceipt::AppliedButGrew
    };

    MirrorOutcome {
        messages: rebuilt,
        receipt,
        is_pure: policy.is_pure,
    }
}

// ============================================================================
// 8. BỘ PHÁT HIỆN THOÁI HÓA NGỮ CẢNH & NÉN TIỀM ẨN MAD (DEGRADATION & MAD)
// ============================================================================

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum DegradationType {
    RepetitionLoop,
    HallucinatedToolCall,
    ContextLengthSpike,
    ConstraintViolation,
    EmptyTurnDegradation,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct DegradationWarning {
    pub kind: DegradationType,
    pub turn_index: usize,
    pub message: String,
}

#[derive(Debug, Default)]
pub struct ContextDegradationDetector {
    pub registered_tool_names: HashSet<String>,
    pub forbidden_phrases: Vec<String>,
}

impl ContextDegradationDetector {
    pub fn new(tools: HashSet<String>, forbidden_phrases: Vec<String>) -> Self {
        Self {
            registered_tool_names: tools,
            forbidden_phrases,
        }
    }

    pub fn audit_history(&self, messages: &[CtxMessage]) -> Vec<DegradationWarning> {
        let mut warnings = Vec::new();
        let mut prev_content: Option<&str> = None;
        let mut empty_count = 0;

        for (idx, msg) in messages.iter().enumerate() {
            let trimmed = msg.content.trim();

            if trimmed.is_empty() && msg.tool_use_id.is_none() && msg.tool_result_id.is_none() {
                empty_count += 1;
                if empty_count >= 2 {
                    warnings.push(DegradationWarning {
                        kind: DegradationType::EmptyTurnDegradation,
                        turn_index: idx,
                        message: "Phát hiện lượt phản hồi rỗng liên tiếp".to_string(),
                    });
                }
            } else {
                empty_count = 0;
            }

            if let Some(prev) = prev_content
                && !trimmed.is_empty()
                && trimmed == prev
            {
                warnings.push(DegradationWarning {
                    kind: DegradationType::RepetitionLoop,
                    turn_index: idx,
                    message: format!("Nội dung lượt {idx} lặp lại y hệt lượt trước đó"),
                });
            }
            prev_content = Some(trimmed);

            if let Some(ref call_id) = msg.tool_use_id {
                let tool_name = msg.content.lines().next().unwrap_or("").trim();
                if !tool_name.is_empty() && !self.registered_tool_names.contains(tool_name) {
                    warnings.push(DegradationWarning {
                        kind: DegradationType::HallucinatedToolCall,
                        turn_index: idx,
                        message: format!(
                            "Công cụ '{tool_name}' (id: {call_id}) không tồn tại trong danh mục"
                        ),
                    });
                }
            }

            for forbidden in &self.forbidden_phrases {
                if msg
                    .content
                    .to_lowercase()
                    .contains(&forbidden.to_lowercase())
                {
                    warnings.push(DegradationWarning {
                        kind: DegradationType::ConstraintViolation,
                        turn_index: idx,
                        message: format!("Thông điệp chứa cụm từ vi phạm ràng buộc: '{forbidden}'"),
                    });
                }
            }
        }

        warnings
    }
}

pub fn compute_median(data: &mut [f64]) -> f64 {
    if data.is_empty() {
        return 0.0;
    }
    data.sort_by(|a, b| a.partial_cmp(b).unwrap_or(Ordering::Equal));
    let mid = data.len() / 2;
    if data.len().is_multiple_of(2) {
        (data[mid - 1] + data[mid]) / 2.0
    } else {
        data[mid]
    }
}

#[derive(Debug, Default)]
pub struct LatentKvCompactor;

impl LatentKvCompactor {
    pub fn compute_mad_threshold(scores: &[f64], tau: f64) -> f64 {
        if scores.is_empty() {
            return 0.0;
        }
        let mut work = scores.to_vec();
        let med = compute_median(&mut work);

        let mut diffs: Vec<f64> = scores.iter().map(|&x| (x - med).abs()).collect();
        let mad = compute_median(&mut diffs);

        med + tau * mad
    }

    pub fn prune_kv_tokens(scores: &[f64], tau: f64) -> Vec<usize> {
        let threshold = Self::compute_mad_threshold(scores, tau);
        scores
            .iter()
            .enumerate()
            .filter(|(_, s)| **s >= threshold)
            .map(|(idx, _)| idx)
            .collect()
    }
}

// ============================================================================
// 9. CHƯƠNG TRÌNH CHÍNH (MAIN FUNCTION)
// ============================================================================

fn main() {
    println!("=== CHƯƠNG 86: VÒNG LẶP TÁC TỬ STREAMING, KV-CACHE & GƯƠNG NGỮ CẢNH CLM ===");

    // 1. Khởi tạo prompt có cấu trúc bảo toàn tiền tố
    let mut prompt_builder = PrefixStablePrompt::new(
        "Bạn là Tác tử Lập trình Tự hành (Autonomous Coding Agent) tiêu chuẩn Chặng III.",
        "Quy tắc tối thượng: Kiểm thử trước khi báo cáo hoàn thành. Không phá vỡ API cũ.",
    );
    prompt_builder.register_tool_schema("read_file", "{\"path\": \"string\"}");
    prompt_builder.register_tool_schema("run_tests", "{\"command\": \"string\"}");

    let prefix_hash = prompt_builder.compute_prefix_hash();
    println!("1. Khởi tạo tiền tố KV-Cache: Mã băm FNV-1a = {prefix_hash:#018x}");

    // 2. Minh họa sổ cái PicoDollars
    let mut ledger = CostLedger::new(3_000_000); // $3.00 mỗi 1M token uncached
    let cost1 = ledger
        .record_tokens(1000, CachePricingTier::UncachedInput)
        .unwrap();
    let cost2 = ledger
        .record_tokens(4000, CachePricingTier::CacheHitRead)
        .unwrap();
    println!(
        "2. Sổ cái FinOps: 1000 uncached = {} picos, 4000 cache-hit = {} picos | Tổng: {} picos",
        cost1.amount, cost2.amount, ledger.total_cost.amount
    );

    // 3. Minh họa vòng lặp phẳng hóa Trampoline Bounce
    let result = run_trampoline(0usize, |count| {
        if count < 5000 {
            Bounce::Continue { state: count + 1 }
        } else {
            Bounce::Done {
                result: format!(
                    "Hoàn thành xuất sắc {count} bước nhảy Trampoline an toàn ngăn xếp!"
                ),
            }
        }
    });
    println!("3. Vòng lặp phẳng hóa: {result}");

    // 4. Minh họa gương ngữ cảnh tự quản LiveContextMirror (CLM)
    let initial_msgs = vec![
        CtxMessage::plain(CtxRole::System, "Chỉ thị hệ thống bất biến"),
        CtxMessage::plain(CtxRole::User, "Hãy đọc tệp cấu hình và chạy kiểm thử"),
        CtxMessage::tool_call("call_01", "read_file\nCargo.toml"),
        CtxMessage::tool_reply("call_01", "package name = ch86"),
        CtxMessage::plain(
            CtxRole::Assistant,
            "Tôi đã thấy cấu hình. Đang chạy test...",
        ),
    ];

    let rendered = render_context_mirror(&initial_msgs, 1);
    println!("4. Gương ngữ cảnh kết xuất:\n{rendered}");

    let policy = MirrorPolicy {
        pinned_count: 1,
        token_budget: 1000,
        gate: EditGate::Fit,
        is_pure: false,
    };
    let outcome = apply_context_mirror_edit(&initial_msgs, &rendered, &policy);
    println!("5. Kết quả kiểm toán CLM: {:?}", outcome.receipt);
    println!("=== HOÀN TẤT CHƯƠNG TRÌNH KHỞI ĐỘNG CHƯƠNG 86 ===");
}
```

---

## 5. Bảng tra cứu lỗi biên dịch thường gặp

Dưới đây là 8 lỗi biên dịch Rust phổ biến nhất khi phát triển bộ khung tác tử và cách khắc phục triệt để:

| Mã lỗi biên dịch | Nguyên nhân gốc rễ | Cách khắc phục chuẩn xác |
|---|---|---|
| `E0382: use of moved value` | Di chuyển biến trạng thái `state` vào closure trong vòng lặp Trampoline mà không tạo giá trị kế tiếp. | Sử dụng mẫu `Bounce::Continue { state: next_val }` với kiểu dữ liệu có thể di chuyển quyền sở hữu hoặc cài đặt trait `Clone` nếu cần thiết. |
| `E0716: temporary value dropped while borrowed` | Tạo chuỗi tạm khi phân tích cú pháp SSE (ví dụ `line.strip_prefix().unwrap()`) rồi gán trực tiếp tham chiếu vào struct. | Chuyển đổi dữ liệu tạm thời thành `String` sở hữu độc lập bằng `.to_string()` hoặc `into()` trước khi lưu vào `SseFrame`. |
| `clippy::collapsible_match` | Viết các khối `match` lồng nhau với `if` kiểm tra điều kiện bên trong thân nhánh. | Sử dụng bảo vệ mẫu (*Match Guards*), ví dụ: `'}' if stack.pop() != Some('{') => return false`. |
| `cannot explicitly dereference within an implicitly-borrowing pattern` (Rust 2024) | Dùng cú pháp `filter(|(_, &s)| ...)` khi duyệt qua iterator trả về tham chiếu lồng trong Rust 2024. | Sửa thành `filter(|(_, s)| **s >= threshold)` hoặc `filter(|&(_, s)| s >= threshold)` để phù hợp với quy tắc ràng buộc mẫu mới của Rust 2024. |
| `attempt to multiply with overflow` | Tính toán chi phí token bằng phép nhân `tokens * unit_price` bằng kiểu số nguyên cơ bản gây tràn số. | Bắt buộc sử dụng phương thức toán học có kiểm tra an toàn `checked_mul` và trả về `Result` hoặc bọc trong kiểu `PicoDollars`. |
| `clippy::needless_range_loop` | Dùng vòng lặp `for i in 0..6` để lập chỉ mục mảng `components[i]`. | Chuyển đổi sang phong cách hàm chuẩn của Rust bằng `.iter().zip().map().sum()`. |
| `clippy::manual_is_multiple_of` (Rust 2024) | Dùng phép chia lấy dư truyền thống `len % 2 == 0` khi tính trung vị. | Thay thế bằng phương thức chuẩn của Rust 2024: `len.is_multiple_of(2)`. |
| `clippy::collapsible_if` | Lồng các câu lệnh `if let` hoặc `if condition` liên tiếp trong kiểm tra gộp tin nhắn hoặc ranh giới SSE. | Sử dụng cú pháp chuỗi điều kiện let-chains: `if condition && let Some(val) = opt { ... }`. |

### Phân tích chi tiết lỗi mẫu liên kết ngầm trong Rust 2024

Hãy xem xét lỗi thú vị xuất hiện trong Rust 2024 Edition:

```rust
// Mã nguồn gây lỗi biên dịch trong Rust 2024:
scores
    .iter()
    .enumerate()
    .filter(|(_, &s)| s >= threshold) // LỖI: reference pattern not allowed when implicitly borrowing
    .map(|(idx, _)| idx)
    .collect();
```

**Nguyên nhân**: Trong Rust 2024, quy tắc *Match Ergonomics* được siết chặt để tránh sự mơ hồ khi khớp mẫu trên các kiểu tham chiếu. Khi bạn viết `|(_, &s)|`, trình biên dịch coi đây là một mẫu không tham chiếu (non-reference pattern) áp lên một giá trị tham chiếu `&(usize, &f64)`. Việc giải tham chiếu tường minh `&s` bên trong một mẫu đang mượn ngầm bị cấm.

**Giải pháp**: Nhận trực tiếp tham chiếu lồng và giải tham chiếu tường minh trong biểu thức logic:

```rust
// Mã nguồn chuẩn xác trong Rust 2024:
scores
    .iter()
    .enumerate()
    .filter(|(_, s)| **s >= threshold) // s có kiểu &&f64, giải tham chiếu 2 lần
    .map(|(idx, _)| idx)
    .collect()
```

---

## 6. Tóm tắt & Bài tập rèn luyện

### Tóm tắt cốt lõi chương

1. **Vòng lặp Streaming**:
   - `SseFrameParser` dùng vùng đệm byte liên tục, xử lý an toàn các ký tự UTF-8 đa byte vỡ qua ranh giới chunk mạng.
   - `SseTurnAssembler` thực thi bất biến chặn đứng `tool_use` cụt đuôi khi chạm trần `max_tokens`.
2. **Khả năng mở rộng & Lái hướng**:
   - `Bounce<S, T>` phẳng hóa mọi đệ quy thành $O(1)$ stack, loại trừ hoàn toàn nguy cơ tràn ngăn xếp.
   - `TurnInboxQueue` ưu tiên 4 cấp độ thông điệp, bảo đảm khả năng hủy khẩn cấp và can thiệp kịp thời từ con người.
3. **Kinh tế học KV-Cache**:
   - Tiền tố đóng băng (`PrefixStablePrompt`) ghim chặt chỉ thị và định nghĩa công cụ ở đầu prompt, giữ mã băm FNV-1a bất biến qua hàng chục lượt để đạt 90–99% cache hit.
   - `PicoDollars` loại bỏ sai số dấu phẩy động bằng số nguyên $10^{12}$ đơn vị/USD.
   - `LastLegUsageMeter` (ADR-0018) tách bạch mức tiêu thụ của riêng chặng hiện tại với tổng tích lũy toàn phiên.
4. **Quản trị Ngữ cảnh Tự quản**:
   - Nén hai tầng: Vi nén RAM `microcompact` + Tóm tắt điểm neo `AnchoredSummary` 6 thành phần, cấm xé lẻ cặp công cụ.
   - Giao thức `LiveContextMirror` (CLM) với 6 bất biến an toàn, sửa triệt để lỗi Edge Case #4 bằng cách kiểm tra thứ tự tuần tự và chống ID công cụ giả mạo.
   - Phát hiện 5 chế độ thoái hóa và cắt tỉa KV tiềm ẩn bằng ngưỡng MAD.

---

### Bài tập rèn luyện

#### Bài tập 1: Bộ đệm SSE tự phục hồi khi gặp khung hỏng (Resilient SSE Parser)
**Mục tiêu**: Mở rộng `SseFrameParser` để khi gặp một khung dữ liệu bị lỗi byte UTF-8 không hợp lệ nằm giữa luồng, bộ phân tích cú pháp không bỏ cuộc mà ghi nhận một cảnh báo `corrupted_frames_count`, bỏ qua phần byte hỏng tới ranh giới `\n\n` kế tiếp và tiếp tục giải mã bình thường các khung phía sau.

<details>
<summary>Gợi ý & Lời giải Bài tập 1</summary>

```rust
#[derive(Debug, Default)]
pub struct ResilientSseParser {
    buffer: Vec<u8>,
    pub corrupted_frames_count: usize,
}

impl ResilientSseParser {
    pub fn new() -> Self {
        Self::default()
    }

    pub fn push_chunk_resilient(&mut self, chunk: &[u8]) -> Vec<SseFrame> {
        self.buffer.extend_from_slice(chunk);
        let mut frames = Vec::new();
        let mut cursor = 0;
        let len = self.buffer.len();

        while cursor < len {
            let slice = &self.buffer[cursor..];
            let boundary = if let Some(pos) = slice.windows(2).position(|w| w == b"\n\n") {
                Some((pos, 2))
            } else {
                slice
                    .windows(4)
                    .position(|w| w == b"\r\n\r\n")
                    .map(|pos| (pos, 4))
            };

            if let Some((frame_len, sep_len)) = boundary {
                let frame_bytes = &self.buffer[cursor..cursor + frame_len];
                match std::str::from_utf8(frame_bytes) {
                    Ok(text) => {
                        if let Some(frame) = Self::parse_frame_text(text) {
                            frames.push(frame);
                        }
                    }
                    Err(_) => {
                        // Ghi nhận khung hỏng và phục hồi tiếp tục
                        self.corrupted_frames_count += 1;
                    }
                }
                cursor += frame_len + sep_len;
            } else {
                break;
            }
        }

        self.buffer.drain(..cursor);
        frames
    }

    fn parse_frame_text(text: &str) -> Option<SseFrame> {
        let mut data_lines = Vec::new();
        let mut event = None;
        for line in text.lines() {
            if let Some(v) = line.strip_prefix("event:") {
                event = Some(v.trim().to_string());
            } else if let Some(v) = line.strip_prefix("data:") {
                data_lines.push(v.trim_start().to_string());
            }
        }
        if data_lines.is_empty() && event.is_none() {
            None
        } else {
            Some(SseFrame {
                event,
                data: data_lines.join("\n"),
                id: None,
                retry: None,
            })
        }
    }
}
```
</details>

---

#### Bài tập 2: Cơ chế ngắt mạch ngân sách khẩn cấp (Emergency Cost Circuit Breaker)
**Mục tiêu**: Xây dựng cấu trúc `EmergencyCircuitBreaker` bọc quanh `CostLedger`. Cấu trúc này nhận vào một hạn mức trần cứng `hard_limit: PicoDollars`. Khi tổng chi phí tích lũy vượt quá 95% hạn mức, phát tín hiệu cảnh báo màu vàng. Khi chạm hoặc vượt 100% hạn mức, ngắt mạch lập tức chuyển sang trạng thái `Open`, tự động đẩy một thông điệp `Priority::SystemCancel` vào hàng đợi `TurnInboxQueue` để dừng khẩn cấp toàn bộ hệ thống tác tử.

<details>
<summary>Gợi ý & Lời giải Bài tập 2</summary>

```rust
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum CircuitState {
    ClosedNormal,
    WarningThreshold,
    OpenTripped,
}

pub struct EmergencyCircuitBreaker {
    pub hard_limit: PicoDollars,
    pub state: CircuitState,
}

impl EmergencyCircuitBreaker {
    pub fn new(hard_limit: PicoDollars) -> Self {
        Self {
            hard_limit,
            state: CircuitState::ClosedNormal,
        }
    }

    pub fn inspect_and_guard(
        &mut self,
        current_total: PicoDollars,
        inbox: &mut TurnInboxQueue,
    ) -> CircuitState {
        if current_total.amount >= self.hard_limit.amount {
            self.state = CircuitState::OpenTripped;
            inbox.push(
                Priority::SystemCancel,
                format!(
                    "NGẮT MẠCH KHẨN CẤP: Chi phí {} picos đã vượt hạn mức trần {} picos",
                    current_total.amount, self.hard_limit.amount
                ),
            );
        } else if current_total.amount >= (self.hard_limit.amount * 95) / 100 {
            self.state = CircuitState::WarningThreshold;
        } else {
            self.state = CircuitState::ClosedNormal;
        }
        self.state
    }
}
```
</details>

---

#### Bài tập 3: Cắt tỉa KV tiềm ẩn đa kênh thích ứng (Adaptive Multi-Head Latent Pruner)
**Mục tiêu**: Cài đặt cấu trúc `MultiHeadLatentPruner` nhận vào một ma trận trọng số chú ý 2D (kích thước `[num_heads, seq_len]`). Thay vì tính một ngưỡng MAD duy nhất cho toàn bộ ma trận, hãy tính ngưỡng cắt MAD độc lập cho từng đầu chú ý (*Attention Head*). Trả về tập hợp các chỉ số token quan trọng là **hợp (union)** của các token vượt ngưỡng qua ít nhất một nửa số đầu chú ý.

<details>
<summary>Gợi ý & Lời giải Bài tập 3</summary>

```rust
pub struct MultiHeadLatentPruner;

impl MultiHeadLatentPruner {
    pub fn prune_multi_head(head_scores: &[Vec<f64>], tau: f64) -> Vec<usize> {
        if head_scores.is_empty() || head_scores[0].is_empty() {
            return Vec::new();
        }
        let num_heads = head_scores.len();
        let seq_len = head_scores[0].len();
        let min_head_votes = (num_heads / 2).max(1);

        let mut token_votes = vec![0usize; seq_len];

        for head in head_scores {
            let threshold = LatentKvCompactor::compute_mad_threshold(head, tau);
            for (idx, &score) in head.iter().enumerate() {
                if score >= threshold {
                    token_votes[idx] += 1;
                }
            }
        }

        token_votes
            .iter()
            .enumerate()
            .filter(|(_, &votes)| votes >= min_head_votes)
            .map(|(idx, _)| idx)
            .collect()
    }
}
```
</details>

---

#### Bài tập 4: Tích hợp Gương ngữ cảnh CLM vào Vòng lặp Trampoline tự hành
**Mục tiêu**: Xây dựng một vòng lặp phẳng hóa Trampoline hoàn chỉnh `run_autonomous_agent_turn` nhận vào danh sách thông điệp, chính sách `MirrorPolicy`, và thực thi kiểm toán CLM tự động sau mỗi lượt phản hồi của mô hình. Nếu tổng token vượt quá 80% ngân sách, tự động kích hoạt cổng `EditGate::Shrink` để thu gọn ngữ cảnh trước khi bước sang vòng lặp kế tiếp.

<details>
<summary>Gợi ý & Lời giải Bài tập 4</summary>

```rust
pub struct AgentState {
    pub messages: Vec<CtxMessage>,
    pub policy: MirrorPolicy,
    pub step: usize,
    pub max_steps: usize,
}

pub fn run_autonomous_agent_turn(initial: AgentState) -> Result<Vec<CtxMessage>, &'static str> {
    run_trampoline(initial, |mut state| {
        if state.step >= state.max_steps {
            return Bounce::Done {
                result: Ok(state.messages),
            };
        }

        // Ước lượng tổng token hiện tại
        let current_tokens: usize = state.messages.iter().map(CtxMessage::token_estimate).sum();

        // Nếu vượt quá 80% ngân sách, chuyển sang cổng Shrink để ép thu nhỏ
        if current_tokens > (state.policy.token_budget * 80) / 100 {
            state.policy.gate = EditGate::Shrink;
            let rendered = render_context_mirror(&state.messages, state.policy.pinned_count);
            // Giả lập tác tử tự tóm tắt các lượt cũ
            let compacted_view = format!(
                "[[CTX_TURN 1 role=user]]\n[Tóm tắt tiến độ {} lượt trước đó]\n{}",
                state.step,
                rendered.lines().rev().take(4).collect::<Vec<_>>().join("\n")
            );
            let outcome = apply_context_mirror_edit(&state.messages, &compacted_view, &state.policy);
            if outcome.receipt == EditReceipt::AppliedShrank {
                state.messages = outcome.messages;
            }
        }

        // Tác tử thực hiện bước suy luận mới
        state.step += 1;
        state.messages.push(CtxMessage::plain(
            CtxRole::Assistant,
            format!("Hoàn thành bước suy luận thứ {}", state.step),
        ));

        Bounce::Continue { state }
    })
}
```
</details>

