# 🦀 Rust Masterclass: Hành Trình Từ Con Số 0 Đến System Design

Chào mừng bạn đến với khóa học **Rust Masterclass** bằng tiếng Việt! Đây không phải là một cuốn sách giáo khoa khô khan. Đây là một hành trình được thiết kế đặc biệt dành cho **những người chưa từng viết một dòng code nào** và **không có nền tảng về toán học**.

Dựa trên cốt lõi của cuốn *Rust All-in-One For Dummies*, giáo trình này đã được biên soạn lại hoàn toàn, mở rộng và tùy biến để giải thích những khái niệm phức tạp nhất của khoa học máy tính thông qua các ví dụ thực tế trong đời sống hàng ngày (như quán phở, bãi đỗ xe, thư viện, chiếc tủ lạnh).

---

## 🎯 Đối Tượng Của Khóa Học

- **Người mới bắt đầu tuyệt đối:** Bạn không cần biết gì về lập trình.
- **Người "sợ toán":** Không có công thức đại số, không có hình học không gian. Chúng ta giải thích mọi thứ bằng tư duy logic và ví dụ đời thực.
- **Lập trình viên muốn học Rust:** Nếu bạn đã biết code nhưng thấy Rust quá khó hiểu (đặc biệt là Borrow Checker), những "ví dụ không dùng toán" ở đây sẽ giúp bạn giác ngộ.

---

## 📚 Lộ Trình Học Tập (9 Chủ Đề)

Khóa học bao gồm 50 chương, chia làm 9 chủ đề lớn. Vui lòng xem **[SUMMARY.md](./SUMMARY.md)** để có danh sách toàn bộ 50 chương.

1. **Lập Trình Cơ Bản (Chương 01 - 05):**
   Hiểu cách máy tính hoạt động, CPU, RAM là gì (ví dụ: công tắc đèn), cách cài đặt Rust và các khái niệm biến, hàm, vòng lặp cơ bản.
   
2. **Cú Pháp & Thiết Kế của Rust (Chương 06 - 12):**
   Chinh phục "con quái vật" Borrow Checker của Rust bằng ví dụ về sổ đỏ nhà đất và thẻ thư viện. Học về Struct, Enum, và cách Rust loại bỏ lỗi `null`.

3. **Lập Trình Hàm - Functional Programming (Chương 13 - 16):**
   Làm quen với tư duy lập trình không thay đổi trạng thái (Immutability), Closure và các Iterator mạnh mẽ của Rust.

4. **Siêu Lập Trình - Meta Programming (Chương 17 - 20):**
   Khám phá Macro trong Rust. Học cách viết code tự sinh ra code bằng ví dụ về kính hiển vi phẫu thuật.

5. **Cấu Trúc Dữ Liệu & Thuật Toán - DSA (Chương 21 - 26):**
   Đánh bại Big-O mà không cần dùng đến một công thức đại số nào (dùng ví dụ tìm tên trong danh bạ). Nắm vững Linked List, Tree, Hash Table.

6. **Thiết Kế & Cấu Trúc Cơ Sở Dữ kết (Chương 27 - 32):**
   Tìm hiểu cách các database thực sự hoạt động dưới nền (B-Tree, LSM-Tree, WAL) và tự viết một engine lưu trữ dữ liệu (Bitcask).

7. **Bảo Mật & Tấn Công Hệ Thống - OSCP (Chương 33 - 38):**
   Học về các lỗ hổng bộ nhớ khét tiếng (Buffer Overflow, Use-After-Free) bằng ví dụ "cốc nước đổ lên laptop", và tại sao Rust lại miễn nhiễm với chúng.

8. **Vibe Coding & AI Prompt Engineering (Chương 39 - 43):**
   Chuyển đổi tư duy từ "người gõ phím" sang "kiến trúc sư hệ thống". Học cách dùng AI để viết code hiệu quả thông qua nghệ thuật Prompting.

9. **Thiết Kế Hệ Thống - System Design (Chương 44 - 50):**
   Xây dựng hệ thống lớn, REST vs gRPC (ví dụ trạm thu phí), Caching (ví dụ tủ lạnh gia đình vs siêu thị), và nguyên lý đồng thuận Raft.

---

## 🚀 Hướng Dẫn Sử Dụng

1. **Bắt đầu từ đâu?**
   Hãy mở file **[SUMMARY.md](./SUMMARY.md)**. Đây là Mục Lục chính của toàn bộ khóa học. Hãy click vào `chuong_01.md` và bắt đầu đọc.
2. **Không bỏ nhảy cóc:**
   Đặc biệt là 12 chương đầu tiên. Rust có một triết lý quản lý bộ nhớ rất độc đáo (Ownership). Nếu bạn bỏ qua các chương nền tảng, bạn sẽ gặp khó khăn ở các chương sau.
3. **Thực hành chạy Code:**
   Trong mỗi chương đều có các đoạn mã (code block) bằng ngôn ngữ Rust. Hãy copy chúng vào file `main.rs` trên máy bạn hoặc sử dụng [Rust Playground (play.rust-lang.org)](https://play.rust-lang.org/) để chạy thử. Tất cả mã nguồn trong repo này đều đã được biên dịch thành công.

---

## 🛠 Yêu Cầu Cài Đặt

Để chạy code trong sách, bạn cần cài đặt Rust. (Đừng lo, Chương 02 sẽ hướng dẫn bạn chi tiết).
* Cài đặt nhanh (Mac/Linux): `curl --proto '=https' --tlsv1.2 -sSf https://sh.rustup.rs | sh`
* Windows: Tải file `.exe` từ [rustup.rs](https://rustup.rs/)

Chúc bạn có một hành trình học Rust đầy thú vị và không còn "sợ" lập trình nữa! 🦀🚀
