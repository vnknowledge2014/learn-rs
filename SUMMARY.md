# Mục lục (Table of Contents) - Giáo Trình Rust Toàn Diện Cho Người Mới Bắt Đầu

Chào mừng bạn đến với **Mục lục toàn diện** của bộ giáo trình *Rust Masterclass: Lập Trình Hệ Thống Toàn Diện Cho Người Mới Bắt Đầu*. 

Giáo trình được thiết kế đặc biệt dành cho người học chưa từng có nền tảng toán học chuyên sâu hay kinh nghiệm lập trình từ trước. Toàn bộ 50 chương được chia thành 9 chủ đề lớn, xây dựng lộ trình sư phạm vững chắc từ phần cứng máy tính căn bản, cú pháp ngôn ngữ, cấu trúc dữ liệu, kiến trúc cơ sở dữ liệu, an toàn thông tin, lập trình hiện đại cùng AI (Vibe Coding), cho tới thiết kế hệ thống phân tán triệu kết nối.

---

## Chủ đề 1: Lập trình căn bản (Programming Fundamental) — Chương 01 đến 05
Khám phá nền tảng máy tính từ mức vật lý: CPU, RAM, thanh ghi, cách dữ liệu nhị phân vận hành và những dòng mã Rust đầu tiên.
- [Chương 01: Máy tính hoạt động thế nào? CPU, RAM và Ngôn ngữ máy (How Computers Work: CPU, RAM, Bits & Bytes)](chuong_01.md)
- [Chương 02: Bắt đầu với Rust & Cargo: Cài đặt và chương trình đầu tiên (Getting Started with Rust & Cargo: Installation and First Program)](chuong_02.md)
- [Chương 03: Biến, Bất biến và Kiểu dữ liệu nguyên bản (Variables, Mutability & Primitive Types)](chuong_03.md)
- [Chương 04: Điều khiển dòng chảy: Rẽ nhánh và Vòng lặp (Control Flow: If/Else and Loops)](chuong_04.md)
- [Chương 05: Hàm, Bộ nhớ Ngăn xếp vs Vùng nhớ tự do, và Nhập/Xuất chuẩn (Functions, Stack vs Heap, and Standard I/O)](chuong_05.md)

---

## Chủ đề 2: Cú pháp & Tư duy thiết kế Rust (Rust Syntax & Design) — Chương 06 đến 12
Làm chủ "linh hồn" của Rust: Hệ thống sở hữu độc nhất vô nhị, quy tắc mượn tham chiếu an toàn, thời gian sống của biến và giao ước hướng đối tượng với Traits.
- [Chương 06: Trọng tâm Rust: Quy tắc Sở hữu & Cơ chế Di chuyển (The Core of Rust: Ownership Rules & Move Semantics)](chuong_06.md)
- [Chương 07: Vay mượn & Tham chiếu: Chia sẻ an toàn tuyệt đối (Borrowing & References: Sharing Safely)](chuong_07.md)
- [Chương 08: Vòng đời dữ liệu: Hiểu đúng mà không cần đau đầu (Lifetimes: Mental Models & Safe References)](chuong_08.md)
- [Chương 09: Cấu trúc dữ liệu tự tạo và Phương thức (Structs, Tuples & Associated Functions)](chuong_09.md)
- [Chương 10: Kiểu liệt kê, Option và So khớp mẫu (Enums, Option, and Pattern Matching)](chuong_10.md)
- [Chương 11: Xử lý lỗi chuyên nghiệp: Panic! vs Result<T, E> và Toán tử `?` (Error Handling: Panic vs Result and the `?` Operator)](chuong_11.md)
- [Chương 12: Giao ước hành vi, Kiểu tổng quát và Tổ chức dự án (Traits, Generics, Modules & Crates)](chuong_12.md)

---

## Chủ đề 3: Lập trình hàm (Functional Programming) — Chương 13 đến 16
Tiếp cận tư duy lập trình khai báo hiện đại: Biến bất biến, hàm ẩn danh Closures, và các đường ống xử lý dữ liệu dòng chảy cực kỳ ngắn gọn và hiệu quả.
- [Chương 13: Lập trình hàm là gì? Bất biến và Phong cách khai báo đường ống (Introduction to Functional Programming & Declarative Pipelines)](chuong_13.md)
- [Chương 14: Hàm ẩn danh: Các chế độ bắt giữ giá trị Fn, FnMut, FnOnce (Closures & Capturing Traits: Fn, FnMut, FnOnce)](chuong_14.md)
- [Chương 15: Vòng lặp lười biếng & Bộ chuyển đổi dòng chảy: map, filter, fold, collect (Iterators & Consumers: map, filter, fold, collect)](chuong_15.md)
- [Chương 16: Hàm bậc cao & Mẫu thiết kế lập trình hàm trong Rust (Higher-Order Functions & Functional Design Patterns)](chuong_16.md)

---

## Chủ đề 4: Siêu lập trình (Macro & Meta Programming) — Chương 17 đến 20
Làm chủ nghệ thuật viết mã để tự động sinh ra mã: Từ Declarative Macros đơn giản đến Procedural Macros phân tích cây cú pháp trừu tượng AST cấp cao.
- [Chương 17: Khái niệm Siêu lập trình: Khi code tự động viết code (Declarative Macros: macro_rules! & Syntax Matchers)](chuong_17.md)
- [Chương 18: Tính vệ sinh trong Macro, Mẫu lặp lại & Các trường hợp biên (Macro Hygiene, Repetition Patterns & Edge Cases)](chuong_18.md)
- [Chương 19: Macro thủ tục: syn, quote & Khám phá Cây cú pháp trừu tượng (Procedural Macros: syn, quote & AST Traversal)](chuong_19.md)
- [Chương 20: Chế tạo Macro: Custom Derive, Thuộc tính và Macro dạng hàm (Custom Derive, Attribute & Function-like Macros)](chuong_20.md)

---

## Chủ đề 5: Cấu trúc dữ liệu & Thuật toán (DSA) — Chương 21 đến 26
Xây dựng nền móng thuật toán vững chắc: Độ phức tạp Big-O trực quan hóa không toán học, mảng, danh sách liên kết, ngăn xếp, hàng đợi, cây nhị phân và đồ thị.
- [Chương 21: Độ phức tạp tính toán & Trực quan hóa Big-O (Computational Complexity & Big-O Visualized)](chuong_21.md)
- [Chương 22: Lưu trữ vùng nhớ liền kề: Mảng cố định, Vector động và Lát cắt (Contiguous Memory: Arrays, Vectors & Slices)](chuong_22.md)
- [Chương 23: Danh sách liên kết & Con trỏ thông minh: Box, Rc, RefCell (Linked Lists & Smart Pointers: Box, Rc, RefCell)](chuong_23.md)
- [Chương 24: Ngăn xếp, Hàng đợi & Hàng đợi hai đầu: Triển khai an toàn và Ứng dụng thực tế (Stacks, Queues & VecDeque)](chuong_24.md)
- [Chương 25: Cây, Cây nhị phân tìm kiếm & Duyệt đệ quy an toàn (Trees, Binary Search Trees & Safe Recursive Traversals)](chuong_25.md)
- [Chương 26: Bảng băm, Đồ thị & Các thuật toán tìm kiếm, sắp xếp cốt lõi (Hash Tables, Graphs & Core Search/Sort Algorithms)](chuong_26.md)

---

## Chủ đề 6: Thiết kế & Kiến trúc cơ sở dữ liệu (Database Internal & Design) — Chương 27 đến 32
Bóc tách bí mật đằng sau các hệ quản trị cơ sở dữ liệu: Cấu trúc trang Slotted-Page 4KB, bộ nhớ đệm Buffer Pool, cây chỉ mục B-Tree / B+ Tree, nhật ký ghi trước WAL, LSM-Tree và tự tay dựng công cụ Key-Value Mini-Bitcask.
- [Chương 27: Cơ chế lưu trữ đĩa cứng & Thao tác vào ra tệp nhị phân (Disk Storage & File I/O Mechanics)](chuong_27.md)
- [Chương 28: Kiến trúc trang Slotted-Page & Quản lý bộ nhớ đệm Buffer Pool (Slotted-Page Architecture & Buffer Pool Management)](chuong_28.md)
- [Chương 29: Chỉ mục hiệu năng cao B-Tree & B+ Tree (High-Performance B-Tree & B+ Tree Indexing)](chuong_29.md)
- [Chương 30: Nhật ký ghi trước WAL & Động cơ lưu trữ hiện đại LSM-Tree (Write-Ahead Logging & LSM-Tree Engine)](chuong_30.md)
- [Chương 31: Giao dịch, Đảm bảo ACID & Kiểm soát đồng thời MVCC (Transactions, ACID Guarantees & MVCC Concurrency Control)](chuong_31.md)
- [Chương 32: Dự án lớn: Xây dựng động cơ lưu trữ Mini-Bitcask Key-Value bền vững (Capstone Project: Building a Persistent Mini-Bitcask Key-Value Engine)](chuong_32.md)

---

## Chủ đề 7: An toàn thông tin & Kỹ thuật tấn công/phòng thủ OSCP (Cyber Security) — Chương 33 đến 38
Tư duy bảo mật thâm nhập thực chiến: Bản đồ bộ nhớ ảo, cơ chế khai thác lỗ hổng kinh điển (Buffer Overflow, UAF, Format String), Unsafe Rust và xây dựng công cụ quét mạng đa luồng siêu tốc.
- [Chương 33: Bản đồ bộ nhớ & Không gian địa chỉ ảo (Virtual Address Space & Memory Layout)](chuong_33.md)
- [Chương 34: Tam đại hiểm họa tham nhũng bộ nhớ: Buffer Overflow, Use-After-Free & Format Strings (Memory Corruption: Buffer Overflow, UAF & Format Strings)](chuong_34.md)
- [Chương 35: Kiểm chứng an toàn bộ nhớ Rust vs Unsafe Rust & FFI (Rust Memory Safety Verification vs Unsafe Rust & FFI)](chuong_35.md)
- [Chương 36: Tự chế công cụ quét cổng mạng đa luồng siêu tốc (High-Speed Concurrent Network Port Scanner Tool)](chuong_36.md)
- [Chương 37: Phân tích gói tin mạng không sao chép & Giải mã tệp nhị phân ELF/PE (Zero-Copy Network Packet Inspection & ELF/PE Parsing)](chuong_37.md)
- [Chương 38: Phương pháp luận tấn công OSCP & Gia cố hệ thống bằng Rust (OSCP Offensive Mindset, Threat Modeling & Rust Defense Hardening)](chuong_38.md)

---

## Chủ đề 8: Lập trình hiện đại cùng AI (Vibe Coding) — Chương 39 đến 43
Phương pháp luận lập trình thời đại mới: Trở thành Tổng đạo diễn kiến trúc hệ thống, kiểm soát cửa sổ ngữ cảnh, phát triển theo đặc tả SDD, lấy trình biên dịch làm trọng tài tối cao và hoàn thiện công cụ CLI chuẩn sản xuất.
- [Chương 39: Tư Duy Vibe Coding: Từ Thợ Gõ Cú Pháp Thành Tổng Đạo Diễn Kiến Trúc (The Vibe Coding Paradigm: System Architect vs Syntax Typist)](chuong_39.md)
- [Chương 40: Kỹ Nghệ Prompt Kỹ Thuật Hệ Thống & Quản Lý Cửa Sổ Ngữ Cảnh (Systems Prompt Engineering & Context Management)](chuong_40.md)
- [Chương 41: Quy Trình Phát Triển Dựa Trên Đặc Tả & TDD Cùng AI (Spec-Driven Development SDD & AI-Assisted TDD)](chuong_41.md)
- [Chương 42: Trình Biên Dịch Là Trọng Tài Tối Cao: Tự Sửa Lỗi Cùng AI (Compiler as Supreme Arbiter: AI Self-Correction & Refactoring)](chuong_42.md)
- [Chương 43: Dự Án Thực Chiến: Xây Dựng Công Cụ CLI Chuẩn Sản Xuất Bằng Vibe Coding (Capstone Project: AI-Assisted Production CLI Tool)](chuong_43.md)

---

## Chủ đề 9: Thiết kế hệ thống phân tán & hiệu năng cao (System Design) — Chương 44 đến 50
Đỉnh cao kỹ nghệ phần mềm hệ thống: Lập trình bất đồng bộ Tokio Runtime và Epoll, mô hình Actor giao tiếp qua kênh Channel, REST API với Axum, gRPC với Tonic, Redis Caching, Định lý CAP và Thuật toán đồng thuận phân tán Raft.
- [Chương 44: Kiến trúc tổng thể: Từ khối đơn Monolith đến Microservices hiệu năng cao (System Architecture: Monolith vs High-Performance Microservices)](chuong_44.md)
- [Chương 45: Bí mật xử lý hàng triệu kết nối: Asynchronous Tokio, Event Loop & Epoll (Asynchronous Concurrency: Tokio Runtime, Event Loops & Epoll Internals)](chuong_45.md)
- [Chương 46: Mô hình Actor & Giao tiếp hộp thư độc lập qua Channel (The Actor Model & Thread-Safe Channels)](chuong_46.md)
- [Chương 47: Xây dựng REST API & gRPC Microservice siêu tốc với Axum & Tonic (High-Throughput REST & gRPC Services with Axum & Tonic)](chuong_47.md)
- [Chương 48: Tầng lưu trữ đệm Redis & Hàng đợi thông điệp phân tán (Distributed Caching with Redis & Message Queuing)](chuong_48.md)
- [Chương 49: Nền tảng hệ phân tán, Định lý CAP & Đồng thuận Raft trực quan (Distributed Consensus, CAP Theorem & Raft Protocol)](chuong_49.md)
- [Chương 50: Đại dự án tốt nghiệp: Thiết kế & Lập trình hệ thống phân tán xử lý đơn hàng (Capstone Project: Distributed Order Processing Engine)](chuong_50.md)

---

*Chúc bạn có những trải nghiệm học tập tuyệt vời và vững vàng trên con đường trở thành một Kỹ sư Hệ thống Rust xuất sắc!*
