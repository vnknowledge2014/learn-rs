# 🦀 Rust Masterclass: Hành Trình Từ Con Số 0 Đến System Design

Chào mừng bạn đến với khóa học **Rust Masterclass** bằng tiếng Việt! Đây không phải một cuốn sách giáo khoa khô khan. Đây là một hành trình được thiết kế đặc biệt dành cho **những người chưa từng viết một dòng code nào** và **không có nền tảng về toán học**.

Dựa trên cốt lõi của cuốn *Rust All-in-One For Dummies*, giáo trình đã được biên soạn lại hoàn toàn, mở rộng và tùy biến để giải thích những khái niệm phức tạp nhất của khoa học máy tính thông qua các ví dụ thực tế trong đời sống hằng ngày (quán phở, bãi đỗ xe, thư viện, phòng công chứng, cửa kiểm tra sân bay).

**69 chương · 21 chủ đề · toàn bộ mã nguồn chạy được và có kiểm thử.**

---

## 🎯 Đối Tượng Của Khóa Học

- **Người mới bắt đầu tuyệt đối:** Bạn không cần biết gì về lập trình.
- **Người "sợ toán":** Không có công thức đại số hay hình học nào bắt buộc. Mọi khái niệm — kể cả Big-O hay Vị nhóm — đều được giải thích bằng tư duy logic và ví dụ đời thực trước, ký hiệu toán học chỉ đến sau.
- **Lập trình viên muốn học Rust:** Nếu bạn đã biết code nhưng thấy Rust khó hiểu (đặc biệt là Borrow Checker), những "ví dụ không dùng toán" ở đây sẽ giúp bạn giác ngộ.
- **Kỹ sư muốn đi xuống tầng thấp hoặc ra ngoài web:** Chủ đề 18–21 đưa bạn tới hệ điều hành, giao thức mạng, vi điều khiển `no_std`, thiết kế mạch số, game engine và hệ thống giao dịch — những nơi Rust có lợi thế thật sự chứ không chỉ là lựa chọn thời thượng.
- **Người đã biết Rust muốn học lập trình hàm nghiêm túc:** Chủ đề 3 (Chương 13–20) đi trọn con đường từ hàm thuần túy tới Monad và mô hình hóa nghiệp vụ bằng kiểu — đầy đủ luật, đầy đủ kiểm thử.

---

## 🚀 Bắt Đầu Trong 3 Phút

```bash
# 1. Cài Rust (nếu chưa có)
curl --proto '=https' --tlsv1.2 -sSf https://sh.rustup.rs | sh     # macOS / Linux
# Windows: tải rustup-init.exe từ https://rustup.rs/

# 2. Chạy thử chương trình minh họa của một chương bất kỳ
cd code
cargo run -p ch14        # Chương 14: Ghép hàm, Curry hóa, Áp dụng từng phần
cargo run -p ch20        # Chương 20: Newtype, Smart Constructor, Typestate

# 3. Chạy toàn bộ bài kiểm thử trong sách
cargo test --workspace
```

Không muốn cài gì cả? Mọi đoạn mã đều copy-paste chạy được trên [Rust Playground](https://play.rust-lang.org/).

---

## 📚 Lộ Trình Học Tập (21 Chủ Đề — 69 Chương)

Mục lục đầy đủ nằm ở **[SUMMARY.md](./SUMMARY.md)**. Tóm tắt từng chủ đề:

| # | Chủ đề | Chương | Nội dung cốt lõi |
|---|---|---|---|
| 1 | **Lập Trình Cơ Bản** | 01–05 | CPU, RAM, bit và byte (ví dụ: công tắc đèn), cài đặt Rust, biến, hàm, vòng lặp, Stack vs Heap |
| 2 | **Cú Pháp & Thiết Kế Rust** | 06–12 | Chinh phục Borrow Checker bằng ví dụ sổ đỏ nhà đất và thẻ thư viện. Struct, Enum, `Option`, `Result`, Trait, Generic |
| 3 | **Lập Trình Hàm** | 13–20 | Hàm thuần túy → ghép hàm & curry hóa → closure → đường ống iterator đầy đủ → bộ kết hợp & lập trình hai đường ray → nửa nhóm/vị nhóm kèm luật → hàm tử/đơn nguyên → mô hình hóa nghiệp vụ bằng kiểu |
| 4 | **Siêu Lập Trình** | 21–24 | `macro_rules!`, tính vệ sinh, macro thủ tục với `syn`/`quote`, custom derive |
| 5 | **Cấu Trúc Dữ Liệu & Thuật Toán** | 25–30 | Big-O không dùng đại số, mảng, danh sách liên kết, ngăn xếp, hàng đợi, cây, bảng băm, đồ thị |
| 6 | **Cơ Sở Dữ Liệu Từ Bên Trong** | 31–36 | Slotted-Page, Buffer Pool, B+ Tree, WAL, LSM-Tree, MVCC, và tự viết engine Mini-Bitcask |
| 7 | **Bảo Mật & Tấn Công Hệ Thống** | 37–42 | Bản đồ bộ nhớ ảo, Buffer Overflow, Use-After-Free, Format String, Unsafe Rust, tự chế công cụ quét cổng |
| 8 | **Vibe Coding cùng AI** | 43–47 | Từ "thợ gõ phím" thành kiến trúc sư: prompt hệ thống, cửa sổ ngữ cảnh, SDD, trình biên dịch làm trọng tài |
| 9 | **Thiết Kế Hệ Thống Phân Tán** | 48–54 | Tokio & epoll, mô hình Actor, REST/gRPC, Redis caching, định lý CAP, thuật toán Raft, đại dự án tốt nghiệp |
| 10 | **Kiểm Thử & Chất Lượng** | 55 | Kim tự tháp kiểm thử: unit/integration/E2E, TDD, BDD, property-based, doctest, mocking, fuzzing |
| 11 | **Kỹ Nghệ Tác Tử AI** | 56 | Context/Harness/Loop/Graph Engineering — ngân sách ngữ cảnh, công cụ như hợp đồng kiểu, vòng lặp có phanh, GraphRAG |
| 12 | **Bảo Mật Web (OSWE)** | 57 | SQLi, XSS, IDOR, SSRF, xác thực, path traversal, Top 10 OWASP dưới góc nhìn Rust |
| 13 | **Kỹ Nghệ Dữ Liệu (DE/DA)** | 58 | Mini-DataFrame dạng cột, ETL, group-by, window, join — nền của Polars/Arrow |
| 14 | **Thiết Kế Hệ Thống Mở Rộng** | 59 | Cân bằng tải, băm nhất quán, giới hạn tần suất, back-pressure |
| 15 | **Khoa Học Máy Tính** | 60 | Quy hoạch động, quay lui, tham lam, lý thuyết số — LeetCode kinh điển |
| 16 | **Web: Backend & Frontend** | 61–62 | Axum (định tuyến, extractor, state); Leptos/WASM (reactivity, Virtual DOM) |
| 17 | **Desktop & Đa Nền Tảng** | 63 | Kiến trúc Elm, IPC; Tauri 2.0 + Svelte, gpui, wgpu |
| 18 | **Hệ Điều Hành & Mạng** | 64–65 | Lập lịch CPU, phân trang & nghịch lý Bélády, bế tắc; đóng gói theo tầng, máy trạng thái TCP, AIMD, CIDR, DNS |
| 19 | **Nhúng & Phần Cứng Số** | 66–67 | `no_std`, MMIO, typestate cho GPIO, số Q16.16; cổng logic, flip-flop, đường ống, đường tới hạn |
| 20 | **Lập Trình Game** | 68 | Vòng lặp bước cố định, Euler nửa ẩn, va chạm AABB, băm không gian, kiến trúc ECS |
| 21 | **Giao Dịch Thuật Toán** | 69 | Sổ lệnh ưu tiên giá–thời gian, động cơ khớp lệnh, cổng rủi ro typestate, bộ kiểm định chiến lược |

**Phụ lục tra cứu:**
- **[Phụ lục A — 24 Cấu trúc Đại số Fantasy Land trong Rust](./PHU_LUC_A_FANTASY_LAND.md)**: bản đồ đầy đủ từ Setoid tới Profunctor, mỗi cấu trúc kèm định nghĩa, luật, ánh xạ sang thư viện chuẩn Rust và mã chạy được. Đọc sau Chương 18–20.
- **[Bảng thuật ngữ Việt–Anh](./THUAT_NGU.md)**: ~320 thuật ngữ, chốt cách dịch nhất quán toàn giáo trình.

---

## 🧭 Cấu Trúc Mỗi Chương

Mọi chương đều theo cùng một khuôn, để bạn luôn biết mình đang ở đâu:

1. **Giới thiệu & Mục tiêu học tập** — bạn sẽ làm được gì sau chương này.
2. **Hình tượng hóa đời sống** — một ví dụ đời thực kèm sơ đồ, trước khi có bất kỳ dòng mã nào.
3. **Khái niệm & Cơ chế kỹ thuật chuyên sâu** — chuyện gì thực sự xảy ra dưới nắp ca-pô.
4. **Mã nguồn minh họa thực chiến** — một chương trình hoàn chỉnh, chạy được, có trong thư mục [`code/`](./code/).
5. **Bảng tra cứu lỗi biên dịch** — những lỗi `rustc` bạn *sẽ* gặp, kèm nguyên nhân và cách sửa.
6. **Tóm tắt & Bài tập rèn luyện** — kèm **Gợi ý** và **Lời giải** ẩn trong thẻ gập (bấm để mở).

---

## 🛠 Mã Nguồn Chạy Được

Toàn bộ chương trình minh họa nằm trong [`code/`](./code/), tổ chức thành một **Cargo workspace** gồm 71 crate:

```bash
cd code
cargo run  -p ch18              # Chương 18: kiểm chứng luật nửa nhóm/vị nhóm
cargo run  -p ch16_mo_rong      # Bộ công cụ Iterator đầy đủ
cargo test -p ch19              # Chạy riêng bài kiểm thử của Chương 19
cargo test --workspace          # Chạy TẤT CẢ
```

Các chương về lập trình hàm đi kèm bộ kiểm thử biến **luật toán học thành bài test chạy được**: luật kết hợp, luật đơn vị, hai luật Functor, ba luật Monad — tất cả đều được kiểm chứng bằng `cargo test`, chứ không chỉ nằm trên giấy.

---

## 📖 Đọc Dưới Dạng Sách Điện Tử (tùy chọn)

Giáo trình đã có sẵn cấu hình [mdBook](https://rust-lang.github.io/mdBook/) — biến 69 tệp Markdown thành một website có mục lục, tìm kiếm toàn văn và chế độ tối:

```bash
cargo install mdbook
mdbook serve --open     # mở sách trong trình duyệt, tự tải lại khi bạn sửa nội dung
```

---

## 💡 Hướng Dẫn Sử Dụng

1. **Bắt đầu từ đâu?** Mở **[SUMMARY.md](./SUMMARY.md)** — đó là mục lục chính. Bấm vào `chuong_01.md` và bắt đầu.
2. **Đừng nhảy cóc**, đặc biệt là 12 chương đầu. Rust có triết lý quản lý bộ nhớ rất độc đáo (Ownership); bỏ qua nền tảng sẽ khiến bạn khổ sở về sau.
3. **Luôn tự làm bài tập trước khi mở Lời giải.** Phần lời giải nằm trong thẻ gập chính là để bạn không vô tình liếc thấy đáp án.
4. **Gõ lại mã, đừng chỉ đọc.** Cách nhanh nhất để hiểu Borrow Checker là để nó từ chối bạn vài chục lần.
5. **Gặp thuật ngữ lạ?** Tra ngay ở **[THUAT_NGU.md](./THUAT_NGU.md)** — bảng đối chiếu Việt–Anh cho toàn bộ giáo trình.

---

## ✅ Cam Kết Chất Lượng

- **Toàn bộ mã nguồn biên dịch được** bằng `rustc` bản ổn định, Rust 2021 Edition — và bạn có thể tự kiểm chứng bằng `cargo build --workspace`.
- **Mọi bài kiểm thử đều xanh**: `cargo test --workspace`.
- Các đoạn mã **cố tình sai** (dùng để minh họa lỗi biên dịch) đều được đánh dấu rõ bằng ký hiệu `❌` và đóng trong dấu chú thích, để chúng không phá vỡ quá trình biên dịch.

---

## 🌱 Nguồn Tham Khảo Để Học Tiếp

Sau khi hoàn thành giáo trình này, đây là những nguồn tiếng Anh đáng đọc tiếp:

- **The Rust Programming Language** (sách chính thức) — <https://doc.rust-lang.org/book/>
- **Rust Book Experiment** (bản có quiz và sơ đồ quyền sở hữu tương tác) — <https://rust-book.cs.brown.edu/>
- **Rust by Example** — <https://doc.rust-lang.org/rust-by-example/>
- **Domain Modeling Made Functional** (Scott Wlaschin) — nền tảng cho Chương 20
- **Functional Programming Made Easier** (Charles Scalfani) — nền tảng cho Chương 18–19
- **fp-core.rs** — <https://github.com/JasonShin/fp-core.rs> — thư viện FP trong Rust, minh họa cách mô phỏng HKT
- **The Embedded Rust Book** — <https://github.com/rust-embedded/book> — nền tảng cho Chương 66
- **30 Days of Rust** — <https://github.com/Hunterdii/30-Days-Of-Rust> — bài tập ngắn ôn lại toàn bộ nền tảng
- **rhdl** (kế thừa `rust-hdl`) — thiết kế phần cứng số bằng Rust; nền cho Chương 67
- **Bevy** — <https://bevyengine.org/> — game engine ECS, tiếp nối Chương 68

Chúc bạn có một hành trình học Rust đầy thú vị và không còn "sợ" lập trình nữa! 🦀🚀
