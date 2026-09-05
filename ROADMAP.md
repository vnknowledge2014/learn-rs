# Lộ trình học tập (Learning Roadmap)

Tài liệu này trả lời ba câu hỏi mà [`SUMMARY.md`](./SUMMARY.md) không trả lời được: **học theo thứ tự nào**, **bỏ qua được gì**, và **cái gì cố tình không có trong giáo trình**.

- 85 chương · 25 chủ đề · 87 crate · toàn bộ mã nguồn biên dịch và kiểm thử được
- `cargo test --workspace` phải xanh trước khi bạn tin bất cứ điều gì trong sách

---

## 1. Ba chặng

| Chặng | Chương | Sau chặng này bạn làm được gì |
|---|---|---|
| **I — Nền tảng** | 01–30 | Viết Rust thành thạo: sở hữu, lập trình hàm, macro, cấu trúc dữ liệu |
| **II — Hệ thống** | 31–69 | Dựng sản phẩm thật: CSDL, bảo mật, phân tán, web, nhúng, HĐH, game |
| **III — Chuyên sâu** | 70–85 | Bốn lĩnh vực Rust có lợi thế không thể thay thế |

Chặng I là **bắt buộc và tuần tự**. Chặng II và III chia thành các nhánh đọc song song được.

---

## 2. Đồ thị phụ thuộc

```
                    ┌─────────────────────────────┐
                    │  CHẶNG I — NỀN TẢNG 01–30   │
                    │  tuần tự, không nhảy cóc     │
                    └──────────────┬──────────────┘
                                   │
      ┌──────────┬─────────────┬───┴────────┬─────────────┬──────────────┐
      ▼          ▼             ▼            ▼             ▼              ▼
  ┌────────┐ ┌────────┐  ┌──────────┐ ┌─────────┐  ┌──────────┐  ┌────────────┐
  │ CSDL   │ │ Bảo    │  │ Phân tán │ │ Web &   │  │ Nhúng &  │  │ Game       │
  │ 31–36  │ │ mật    │  │ 48–54    │ │ Desktop │  │ Phần cứng│  │ 68         │
  │        │ │ 37–42  │  │ 55,59    │ │ 61–63   │  │ 66–67    │  │            │
  └────┬───┘ └───┬────┘  └────┬─────┘ └─────────┘  └────┬─────┘  └────────────┘
       │         │            │                          │
       │         │            │  ┌───────────────────────┘
       │         ▼            │  │
       │   ┌──────────┐       │  │
       │   │ Blockchain       │  │
       │   │ 70–73    │◄──────┘  │
       │   └────┬─────┘          │
       │        │                │
       ▼        ▼                ▼
  ┌─────────────────────────────────────────────┐
  │  HFT 74–78  ──►  Phần cứng 79–81            │
  │      │                  │                    │
  │      └────────┬─────────┘                    │
  │               ▼                              │
  │      Định lượng 82–84                        │
  │               ▼                              │
  │      ★ 85: HỆ SINH THÁI TÍCH HỢP ★          │
  │      (yêu cầu ĐỦ 74–78; nên đọc sau 70–73)  │
  └─────────────────────────────────────────────┘
```

**Điều kiện tiên quyết thực sự**, không phải gợi ý:

| Chương | Bắt buộc đọc trước | Vì sao |
|---|---|---|
| 70–73 | 01–30 | Cần `BTreeMap`, trait, xử lý lỗi, kiểm thử |
| 71 | 65 (mạng) | Dùng lại khái niệm đóng gói theo tầng |
| 74–78 | 01–30, và 26 (bảng băm/cây) | Sổ lệnh là cấu trúc dữ liệu trước khi là tài chính |
| 79 | 67 (phần cứng số) | Chương 67 dạy cổng logic; chương 79 dùng nó |
| 80–81 | 74 (bố cục bộ nhớ) | Chương 74 giới thiệu dòng cache, AoS/SoA |
| 82–84 | không có | Đọc độc lập được, chỉ cần chặng I |
| **85** | **74, 75, 76, 77, 78** | Nó **nối** đúng năm chương đó lại |

---

## 3. Bốn nhánh theo mục tiêu

Chọn một nhánh, đọc hết, rồi quay lại chọn nhánh khác. Đừng đọc song song bốn nhánh.

### Nhánh A — Kỹ sư hệ thống / backend
`01–30` → `31–36` (CSDL) → `48–54` (phân tán) → `55` (kiểm thử) → `59` (mở rộng) → `61–63` (web/desktop) → `64–65` (HĐH/mạng)

### Nhánh B — Giao dịch định lượng & HFT
`01–30` → `26` → `69` (sổ lệnh nhập môn) → `74–78` → **`85`** → `82–84` (định lượng) → `79–81` (khi cần chạm trần hiệu năng)

> Đây là nhánh dài nhất và cũng là nhánh có mật độ "lỗi thật" cao nhất trong sách. Đọc chương 85 **sau cùng** trong nhóm 74–78: nó tồn tại để cho thấy năm mảnh đúng riêng lẻ vẫn ghép thành một hệ sai.

### Nhánh C — Blockchain & Web3
`01–30` → `65` (mạng) → `70` (blockchain từ số không) → `71` (P2P) → `72` (CosmWasm/Solana) → `73` (Ethereum) → `78` (thị trường DeFi) → `85` (nếu muốn nối với thị trường truyền thống)

### Nhánh D — Hiệu năng & phần cứng
`01–30` → `74` (bố cục bộ nhớ) → `80` (CPU sâu) → `81` (GPU) → `66–67` (nhúng, phần cứng số) → `79` (FPGA cho giao dịch)

---

## 4. Bản đồ phủ OpenAlgo (13 khoá · 407 chương)

Nguồn: <https://www.openalgo.in/learn> (kiểm kê ngày 05/09/2026). Yêu cầu ban đầu là "tất cả các bài học trong phần learn nên được dạy bằng Rust". Dưới đây là trạng thái thật của từng khoá, kể cả những khoá **không** được chuyển và lý do.

| Khoá | Ch. | Mức | Trạng thái trong giáo trình này |
|---|---:|---|---|
| Technical Analysis | 28 | Cơ bản | ✅ **Chương 82** — nến, mẫu hình, SMA/EMA/RSI/MACD/Bollinger/ATR, kèm bất biến chống nhìn trộm tương lai |
| Options Basics | 26 | Cơ bản | ✅ **Chương 83** — quyền mua/bán, moneyness, Greeks, biến động ngụ ý |
| Options Strategies | 27 | Trung cấp | ✅ **Chương 83** — payoff spread/straddle/condor, điểm hoà vốn |
| Statistical Arbitrage | 17 | Chuyên sâu | ✅ **Chương 84** — tính dừng, đồng liên kết, Kalman, trung tính thị trường |
| Quantitative Trading | 78 | Chuyên sâu | ◐ **Chương 84 + 74–78 + 85** — vi cấu trúc, công nghệ HFT và thực thi nằm ở nhóm HFT; thời gian chuỗi, phái sinh, nghiên cứu alpha, kiểm định trung thực ở ch84 |
| Algo Trading with Python | 32 | Trung cấp | ◐ **Chương 69 + 77 + 85** — chỉ báo, tín hiệu, lệnh, quản trị rủi ro được cài lại bằng Rust; phần SDK riêng của OpenAlgo không chuyển |
| Futures Trading | 27 | Cơ bản | ◐ **Một phần** — ký quỹ, đòn bẩy, định cỡ vị thế nằm trong ch77; phần đặc thù thị trường Ấn Độ không chuyển |
| Risk Management | 33 | Cơ bản | ◐ **Chương 77 + 84** — định cỡ, dừng lỗ, hạn mức, VaR/ES, sụt giảm |
| Python for Traders | 40 | Cơ bản | ✗ **Không chuyển** — đây là khoá dạy chính ngôn ngữ Python. Bản tương ứng cho Rust là chặng I (chương 01–30) |
| Stock Market Basics | 18 | Cơ bản | ✗ **Không chuyển** — kiến thức thị trường, không có phần lập trình được |
| AmiBroker AFL | 36 | Cơ bản | ✗ **Không chuyển** — dạy một ngôn ngữ độc quyền khác |
| Taxation for Traders | 19 | Cơ bản | ✗ **Không chuyển** — luật thuế Ấn Độ, thay đổi theo năm và theo quốc gia |
| Trading Psychology | 26 | Cơ bản | ✗ **Không chuyển** — không có phần lập trình được |

**Tổng kết trung thực.** 98 chương thuộc bốn khoá lập trình được cốt lõi được chuyển đầy đủ sang Rust (Technical Analysis, Options Basics, Options Strategies, Statistical Arbitrage). Thêm khoảng 170 chương của bốn khoá còn lại được phủ một phần qua nhóm HFT và quản trị rủi ro. **139 chương thuộc năm khoá cuối bảng không được chuyển**, vì chúng dạy một ngôn ngữ khác, luật pháp một quốc gia, hoặc tâm lý con người — không phải thứ chuyển sang Rust được. Sao chép chúng sang tiếng Việt sẽ là dịch thuật, không phải giảng dạy lập trình.

---

## 5. Bản đồ phủ LeetCPU & LeetGPU

| Nguồn | Quy mô | Trạng thái |
|---|---|---|
| [leetcpu.com](https://www.leetcpu.com/) | 22 bài, 4 nhóm, chạy trên ChampSim | ✅ **Chương 80** — cả 22 bài được ánh xạ sang kỹ thuật tương ứng, xem bảng trong chương |
| [leetgpu.com](https://leetgpu.com/) | 99 thử thách (19 Dễ / 65 Vừa / 15 Khó) | ✅ **Chương 81** — phân loại 11 nhóm chủ đề, quy về 4 kỹ thuật gốc |

Cả hai trang đều là ứng dụng một trang; dữ liệu được thu thập bằng cách kết xuất trong trình duyệt thật rồi đọc DOM. Nội dung chi tiết từng bài của LeetCPU nằm sau đăng nhập và **không** được truy cập — chúng ta dùng danh sách bài công khai làm phân loại, rồi tự cài kỹ thuật bằng Rust.

---

## 6. Cái gì cố tình KHÔNG có trong giáo trình

Ghi rõ để bạn không đi tìm:

- **Mã dùng framework thật trong workspace.** Các chương 61, 62, 63, 66, 68, 72, 79 chỉ giữ **lõi thuần tuý** trong crate, để `cargo test --workspace` chạy được offline, không cần SDK, không cần mạng, không cần bo mạch. Mã dùng Axum, Leptos, Tauri, `cosmwasm-std`, `solana-program`, RHDL nằm trong phần lý thuyết của chương.
- **Lời khuyên đầu tư.** Sách này dạy cách xây hệ thống, không dạy nên mua gì.
- **Số liệu lãi lỗ đáng tin.** Mọi phiên giao dịch trong sách đều là **tổng hợp**. Chương 85 giải thích rõ vì sao con số lãi lỗ của nó là một sản phẩm phụ của mô hình, và vì sao thứ đáng tin là các **bất biến**.
- **Nội dung bản quyền của các nguồn tham khảo.** Chúng ta dùng phân loại và tên bài để định hướng, rồi tự cài lại từ đầu bằng Rust.

---

## 7. Cách tự kiểm chứng

```bash
cd code
cargo build --workspace          # phải 0 cảnh báo, 0 lỗi
cargo test  --workspace          # phải toàn xanh
cargo run -p ch85                # hệ sinh thái HFT tích hợp, hai sàn
```

Nếu một con số trong sách không khớp với thứ máy bạn in ra, hãy tin cái máy bạn in ra và mở một issue.
