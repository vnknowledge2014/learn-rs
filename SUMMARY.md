# Mục lục (Table of Contents) — Giáo Trình Rust Toàn Diện Cho Người Mới Bắt Đầu

Chào mừng bạn đến với **Mục lục toàn diện** của bộ giáo trình *Rust Masterclass: Lập Trình Hệ Thống Toàn Diện Cho Người Mới Bắt Đầu*.

Giáo trình được thiết kế đặc biệt dành cho người học chưa từng có nền tảng toán học chuyên sâu hay kinh nghiệm lập trình từ trước. Toàn bộ **85 chương** được chia thành **25 chủ đề** lớn, xây dựng lộ trình sư phạm vững chắc từ phần cứng máy tính căn bản, cú pháp ngôn ngữ, lập trình hàm, cấu trúc dữ liệu, kiến trúc cơ sở dữ liệu, an toàn thông tin, lập trình cùng AI, hệ thống phân tán, cho tới bốn chặng chuyên sâu cuối: blockchain từ số không, hệ sinh thái giao dịch tần suất cao, hiệu năng cấp phần cứng (FPGA/CPU/GPU) và tài chính định lượng.

Lộ trình chia làm **ba chặng**:

| Chặng | Chương | Mục tiêu |
|---|---|---|
| **Nền tảng** | 01–30 | Từ bit và byte tới sở hữu, lập trình hàm, macro, cấu trúc dữ liệu — đủ để viết Rust thành thạo |
| **Hệ thống** | 31–69 | Cơ sở dữ liệu, bảo mật, phân tán, web, nhúng, hệ điều hành, game — đủ để dựng sản phẩm thật |
| **Chuyên sâu** | 70–85 | Blockchain, HFT, FPGA/CPU/GPU, định lượng — bốn lĩnh vực Rust có lợi thế không thể thay thế |

> **Mã nguồn chạy được**: toàn bộ chương trình minh họa nằm trong thư mục [`code/`](./code/), tổ chức thành một Cargo workspace. Chạy `cargo run -p ch14` để xem chương 14 hoạt động, hay `cargo test --workspace` để kiểm chứng mọi bài kiểm thử trong sách.
> **Thuật ngữ**: xem bảng đối chiếu Việt–Anh tại [`THUAT_NGU.md`](./THUAT_NGU.md).
> **Nên đọc theo thứ tự nào?** [`ROADMAP.md`](./ROADMAP.md) có đồ thị phụ thuộc, bốn nhánh học theo mục tiêu, và bản đồ phủ đầy đủ các nguồn tham khảo.


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
Làm chủ "linh hồn" của Rust: hệ thống sở hữu độc nhất vô nhị, quy tắc mượn tham chiếu an toàn, thời gian sống của biến và giao ước hành vi với Traits.
- [Chương 06: Trọng tâm Rust: Quy tắc Sở hữu & Cơ chế Di chuyển (The Core of Rust: Ownership Rules & Move Semantics)](chuong_06.md)
- [Chương 07: Vay mượn & Tham chiếu: Chia sẻ an toàn tuyệt đối (Borrowing & References: Sharing Safely)](chuong_07.md)
- [Chương 08: Vòng đời dữ liệu: Hiểu đúng mà không cần đau đầu (Lifetimes: Mental Models & Safe References)](chuong_08.md)
- [Chương 09: Cấu trúc dữ liệu tự tạo và Phương thức (Structs, Tuples & Associated Functions)](chuong_09.md)
- [Chương 10: Kiểu liệt kê, Option và So khớp mẫu (Enums, Option, and Pattern Matching)](chuong_10.md)
- [Chương 11: Xử lý lỗi chuyên nghiệp: Panic! vs Result<T, E> và Toán tử `?` (Error Handling: Panic vs Result and the `?` Operator)](chuong_11.md)
- [Chương 12: Giao ước hành vi, Kiểu tổng quát và Tổ chức dự án (Traits, Generics, Modules & Crates)](chuong_12.md)

---

## Chủ đề 3: Lập trình hàm (Functional Programming) — Chương 13 đến 20
Tám chương đi trọn con đường từ tư duy khai báo tới mô hình hóa nghiệp vụ: hàm thuần túy, phép ghép hàm, closure, đường ống iterator đầy đủ, cấu trúc đại số kèm luật, hàm tử/đơn nguyên, và cuối cùng là biến trạng thái sai thành thứ không thể biểu diễn được.
- [Chương 13: Lập trình hàm là gì? Bất biến, Minh bạch tham chiếu và Hàm toàn phần (Introduction to FP: Immutability, Referential Transparency & Total Functions)](chuong_13.md)
- [Chương 14: Ghép hàm, Curry hóa và Áp dụng từng phần (Function Composition, Currying & Partial Application)](chuong_14.md)
- [Chương 15: Hàm ẩn danh: Các chế độ bắt giữ giá trị Fn, FnMut, FnOnce (Closures & Capturing Traits: Fn, FnMut, FnOnce)](chuong_15.md)
- [Chương 16: Bộ lặp lười biếng & Toàn bộ đường ống dữ liệu: map, filter_map, fold, collect (Iterators & the Complete Data Pipeline Toolkit)](chuong_16.md)
- [Chương 17: Hàm bậc cao & Mẫu thiết kế lập trình hàm trong Rust (Higher-Order Functions & Functional Design Patterns)](chuong_17.md)
- [Chương 18: Cấu trúc đại số & Luật: Nửa nhóm, Vị nhóm và cách kiểm chứng (Semigroup, Monoid & Verifying Laws)](chuong_18.md)
- [Chương 19: Hàm tử, Hàm tử áp dụng và Đơn nguyên — bản đồ sang thư viện chuẩn Rust (Functor, Applicative & Monad)](chuong_19.md)
- [Chương 20: Mô hình hóa nghiệp vụ bằng kiểu: Kiểu bọc, Hàm khởi tạo có kiểm chứng và Typestate (Domain Modeling with Types)](chuong_20.md)

---

## Chủ đề 4: Siêu lập trình (Macro & Meta Programming) — Chương 21 đến 24
Làm chủ nghệ thuật viết mã để tự động sinh ra mã: từ Declarative Macros đơn giản đến Procedural Macros phân tích cây cú pháp trừu tượng AST cấp cao.
- [Chương 21: Khái niệm Siêu lập trình: Khi code tự động viết code (Declarative Macros: macro_rules! & Syntax Matchers)](chuong_21.md)
- [Chương 22: Tính vệ sinh trong Macro, Mẫu lặp lại & Các trường hợp biên (Macro Hygiene, Repetition Patterns & Edge Cases)](chuong_22.md)
- [Chương 23: Macro thủ tục: syn, quote & Khám phá Cây cú pháp trừu tượng (Procedural Macros: syn, quote & AST Traversal)](chuong_23.md)
- [Chương 24: Chế tạo Macro: Custom Derive, Thuộc tính và Macro dạng hàm (Custom Derive, Attribute & Function-like Macros)](chuong_24.md)

---

## Chủ đề 5: Cấu trúc dữ liệu & Thuật toán (DSA) — Chương 25 đến 30
Xây dựng nền móng thuật toán vững chắc: độ phức tạp Big-O trực quan hóa không toán học, mảng, danh sách liên kết, ngăn xếp, hàng đợi, cây nhị phân và đồ thị.
- [Chương 25: Độ phức tạp tính toán & Trực quan hóa Big-O (Computational Complexity & Big-O Visualized)](chuong_25.md)
- [Chương 26: Lưu trữ vùng nhớ liền kề: Mảng cố định, Vector động và Lát cắt (Contiguous Memory: Arrays, Vectors & Slices)](chuong_26.md)
- [Chương 27: Danh sách liên kết & Con trỏ thông minh: Box, Rc, RefCell (Linked Lists & Smart Pointers: Box, Rc, RefCell)](chuong_27.md)
- [Chương 28: Ngăn xếp, Hàng đợi & Hàng đợi hai đầu: Triển khai an toàn và Ứng dụng thực tế (Stacks, Queues & VecDeque)](chuong_28.md)
- [Chương 29: Cây, Cây nhị phân tìm kiếm & Duyệt đệ quy an toàn (Trees, Binary Search Trees & Safe Recursive Traversals)](chuong_29.md)
- [Chương 30: Bảng băm, Đồ thị & Các thuật toán tìm kiếm, sắp xếp cốt lõi (Hash Tables, Graphs & Core Search/Sort Algorithms)](chuong_30.md)

---

## Chủ đề 6: Thiết kế & Kiến trúc cơ sở dữ liệu (Database Internal & Design) — Chương 31 đến 36
Bóc tách bí mật đằng sau các hệ quản trị cơ sở dữ liệu: cấu trúc trang Slotted-Page 4KB, bộ nhớ đệm Buffer Pool, cây chỉ mục B-Tree / B+ Tree, nhật ký ghi trước WAL, LSM-Tree và tự tay dựng công cụ Key-Value Mini-Bitcask.
- [Chương 31: Cơ chế lưu trữ đĩa cứng & Thao tác vào ra tệp nhị phân (Disk Storage & File I/O Mechanics)](chuong_31.md)
- [Chương 32: Kiến trúc trang Slotted-Page & Quản lý bộ nhớ đệm Buffer Pool (Slotted-Page Architecture & Buffer Pool Management)](chuong_32.md)
- [Chương 33: Chỉ mục hiệu năng cao B-Tree & B+ Tree (High-Performance B-Tree & B+ Tree Indexing)](chuong_33.md)
- [Chương 34: Nhật ký ghi trước WAL & Động cơ lưu trữ hiện đại LSM-Tree (Write-Ahead Logging & LSM-Tree Engine)](chuong_34.md)
- [Chương 35: Giao dịch, Đảm bảo ACID & Kiểm soát đồng thời MVCC (Transactions, ACID Guarantees & MVCC Concurrency Control)](chuong_35.md)
- [Chương 36: Dự án lớn: Xây dựng động cơ lưu trữ Mini-Bitcask Key-Value bền vững (Capstone Project: Building a Persistent Mini-Bitcask Key-Value Engine)](chuong_36.md)

---

## Chủ đề 7: An toàn thông tin & Kỹ thuật tấn công/phòng thủ OSCP (Cyber Security) — Chương 37 đến 42
Tư duy bảo mật thâm nhập thực chiến: bản đồ bộ nhớ ảo, cơ chế khai thác lỗ hổng kinh điển (Buffer Overflow, UAF, Format String), Unsafe Rust và xây dựng công cụ quét mạng đa luồng siêu tốc.
- [Chương 37: Bản đồ bộ nhớ & Không gian địa chỉ ảo (Virtual Address Space & Memory Layout)](chuong_37.md)
- [Chương 38: Tam đại hiểm họa tham nhũng bộ nhớ: Buffer Overflow, Use-After-Free & Format Strings (Memory Corruption: Buffer Overflow, UAF & Format Strings)](chuong_38.md)
- [Chương 39: Kiểm chứng an toàn bộ nhớ Rust vs Unsafe Rust & FFI (Rust Memory Safety Verification vs Unsafe Rust & FFI)](chuong_39.md)
- [Chương 40: Tự chế công cụ quét cổng mạng đa luồng siêu tốc (High-Speed Concurrent Network Port Scanner Tool)](chuong_40.md)
- [Chương 41: Phân tích gói tin mạng không sao chép & Giải mã tệp nhị phân ELF/PE (Zero-Copy Network Packet Inspection & ELF/PE Parsing)](chuong_41.md)
- [Chương 42: Tư duy tấn công thực chiến OSCP, Mô hình hóa mối đe dọa & Gia cố bảo mật hệ thống (OSCP Offensive Mindset, Threat Modeling & Hardening)](chuong_42.md)

---

## Chủ đề 8: Lập trình hiện đại cùng AI (Vibe Coding) — Chương 43 đến 47
Phương pháp luận lập trình thời đại mới: trở thành tổng đạo diễn kiến trúc hệ thống, kiểm soát cửa sổ ngữ cảnh, phát triển theo đặc tả SDD, lấy trình biên dịch làm trọng tài tối cao và hoàn thiện công cụ CLI chuẩn sản xuất.
- [Chương 43: Tư Duy Vibe Coding: Từ Thợ Gõ Cú Pháp Thành Tổng Đạo Diễn Kiến Trúc (The Vibe Coding Paradigm: System Architect vs Syntax Typist)](chuong_43.md)
- [Chương 44: Kỹ Nghệ Prompt Kỹ Thuật Hệ Thống & Quản Lý Cửa Sổ Ngữ Cảnh (Systems Prompt Engineering & Context Management)](chuong_44.md)
- [Chương 45: Quy Trình Phát Triển Dựa Trên Đặc Tả & TDD Cùng AI (Spec-Driven Development SDD & AI-Assisted TDD)](chuong_45.md)
- [Chương 46: Trình Biên Dịch Là Trọng Tài Tối Cao: Tự Sửa Lỗi Cùng AI (Compiler as Supreme Arbiter: AI Self-Correction & Refactoring)](chuong_46.md)
- [Chương 47: Dự Án Thực Chiến: Xây Dựng Công Cụ CLI Chuẩn Sản Xuất Bằng Vibe Coding (Capstone Project: AI-Assisted Production CLI Tool)](chuong_47.md)

---

## Chủ đề 9: Thiết kế hệ thống phân tán & hiệu năng cao (System Design) — Chương 48 đến 54
Đỉnh cao kỹ nghệ phần mềm hệ thống: lập trình bất đồng bộ Tokio Runtime và Epoll, mô hình Actor giao tiếp qua kênh Channel, REST API với Axum, gRPC với Tonic, Redis Caching, Định lý CAP và thuật toán đồng thuận phân tán Raft.
- [Chương 48: Kiến trúc hệ thống: Từ khối đơn Monolith đến Microservices phân tán hiệu năng cao (Monolithic vs High-Performance Microservices)](chuong_48.md)
- [Chương 49: Động cơ bất đồng bộ Tokio Runtime, Vòng lặp sự kiện & Cơ chế Epoll (Asynchronous Tokio Runtime, Event Loops & Epoll)](chuong_49.md)
- [Chương 50: Mô hình Actor & Giao tiếp hộp thư đa luồng qua Channel (Actor Model & Thread-Safe Channels mpsc/oneshot)](chuong_50.md)
- [Chương 51: Dịch vụ REST & gRPC thông lượng cao với Axum & Tonic (High-Throughput REST & gRPC Services with Axum & Tonic)](chuong_51.md)
- [Chương 52: Tầng lưu trữ đệm phân tán Redis & Hàng đợi thông điệp (Distributed Caching with Redis & Message Queuing)](chuong_52.md)
- [Chương 53: Nền tảng hệ phân tán, Định lý CAP & Thuật toán đồng thuận Raft (Distributed Consensus, CAP Theorem & Raft Protocol)](chuong_53.md)
- [Chương 54: Đại dự án tốt nghiệp: Xây dựng Động cơ Xử lý Đơn hàng Phân tán (Capstone Project: Distributed Order Processing Engine)](chuong_54.md)

---

## Chủ đề 10: Kiểm thử & Đảm bảo chất lượng (Testing & Quality) — Chương 55
Kim tự tháp kiểm thử và toàn bộ phương pháp: unit, integration, E2E, TDD, BDD, property-based, doctest, mocking và fuzzing — tất cả bằng công cụ tích hợp sẵn của Rust.
- [Chương 55: Kim tự tháp Kiểm thử — Unit, Integration, E2E, TDD, BDD, Property & Doctest (The Testing Pyramid)](chuong_55.md)

---

## Chủ đề 11: Kỹ nghệ Tác tử AI (Agentic AI Engineering) — Chương 56
Bốn tầng kỹ nghệ đứng sau mọi ứng dụng AI thực chiến, vượt xa Prompt Engineering: quản trị ngân sách ngữ cảnh, thiết kế bộ khung công cụ, vòng lặp tác tử có điều kiện dừng, và đồ thị tri thức cho truy xuất nhiều bước.
- [Chương 56: Kỹ nghệ Ngữ cảnh, Bộ khung, Vòng lặp và Đồ thị cho Tác tử AI (Context, Harness, Loop & Graph Engineering)](chuong_56.md)

---

## Chủ đề 12: Bảo mật Ứng dụng Web — OSWE (Web Application Security) — Chương 57
Mười lỗ hổng web kinh điển theo tinh thần OSWE, mỗi lỗ hổng có bản dính lỗi và bản sửa kèm test: SQLi, XSS, IDOR, SSRF, xác thực, path traversal và Top 10 OWASP dưới góc nhìn Rust.
- [Chương 57: Bảo mật ứng dụng Web — OSWE: SQLi, XSS, IDOR, SSRF, Xác thực & Path Traversal (Web Application Security)](chuong_57.md)

---

## Chủ đề 13: Kỹ nghệ Dữ liệu & Phân tích (Data Engineering & Analytics) — Chương 58
Xây mini-DataFrame dạng cột từ đầu để hiểu cơ chế Polars/Arrow: đường ống ETL, group-by, window function, join — tất cả trên iterator và closure.
- [Chương 58: Kỹ nghệ Dữ liệu & Phân tích bằng Rust — ETL, DataFrame dạng cột, Group-By, Window & Join (Data Engineering & Analytics)](chuong_58.md)

---

## Chủ đề 14: Thiết kế hệ thống mở rộng (Scaling Patterns) — Chương 59
Bốn mẫu mở rộng ngang bổ sung cho Chủ đề 9: cân bằng tải, băm nhất quán, giới hạn tần suất và back-pressure.
- [Chương 59: Thiết kế hệ thống mở rộng — Cân bằng tải, Băm nhất quán, Giới hạn tần suất & Back-Pressure (Scaling Patterns)](chuong_59.md)


---

## Chủ đề 15: Khoa học Máy tính & Thuật toán nâng cao (Algorithm Design) — Chương 60
Bốn mô thức thiết kế thuật toán theo tinh thần TheAlgorithms/Rust và Rusty-CS: quy hoạch động, quay lui, tham lam và lý thuyết số — giải các bài LeetCode kinh điển.
- [Chương 60: Khoa học máy tính — Quy hoạch động, Quay lui, Tham lam & Lý thuyết số (Algorithm Design Paradigms)](chuong_60.md)

---

## Chủ đề 16: Phát triển Web — Backend & Frontend (Web Development) — Chương 61–62
Xây dịch vụ web với Axum và giao diện với Rust+WASM/Leptos: định tuyến, trích xuất, trạng thái, hệ phản ứng và Virtual DOM.
- [Chương 61: Phát triển Backend Web với Axum — Định tuyến, Bộ trích xuất, Trạng thái & Xử lý lỗi (Backend Web Development)](chuong_61.md)
- [Chương 62: Phát triển Frontend với Rust & WebAssembly — Hệ phản ứng & Virtual DOM (Frontend Development)](chuong_62.md)

---

## Chủ đề 17: Ứng dụng Desktop & Đa nền tảng (Desktop & Cross-Platform) — Chương 63
Xây ứng dụng desktop chạy mọi hệ điều hành: kiến trúc trạng thái Elm, cầu IPC, và ba con đường Tauri 2.0 + Svelte, gpui, wgpu.
- [Chương 63: Ứng dụng Desktop & Đa nền tảng — Tauri 2.0, gpui & wgpu (Desktop & Cross-Platform Apps)](chuong_63.md)

---

## Chủ đề 18: Hệ điều hành & Mạng máy tính (Operating Systems & Networking) — Chương 64–65
Hai tầng nền mà mọi chương trình đều đứng lên: hệ điều hành phân phối CPU và bộ nhớ, mạng máy tính chuyển bit qua dây. Lập lịch, phân trang, bế tắc; đóng gói theo tầng, máy trạng thái TCP, điều khiển tắc nghẽn, CIDR và DNS.
- [Chương 64: Hệ điều hành từ bên trong — Lập lịch CPU, Bộ nhớ ảo & Bế tắc (Operating Systems Internals)](chuong_64.md)
- [Chương 65: Mạng máy tính & Giao thức — Từ Bit Trên Dây Tới HTTP (Computer Networking & Protocols)](chuong_65.md)

---

## Chủ đề 19: Hệ thống nhúng & Phần cứng số (Embedded & Digital Hardware) — Chương 66–67
Rust ở nơi không có hệ điều hành, không có heap, không có tha thứ. Thanh ghi ánh xạ bộ nhớ, typestate cho chân GPIO, số dấu phẩy tĩnh — rồi đi xa hơn một bước: tự thiết kế mạch số bằng cổng logic, flip-flop và đường ống.
- [Chương 66: Lập trình nhúng & `no_std` — Rust Trên Con Chip 32 KB RAM (Embedded Rust)](chuong_66.md)
- [Chương 67: FPGA & Thiết kế phần cứng số — Khi Chương Trình Trở Thành Mạch Điện (Digital Hardware Design in Rust)](chuong_67.md)

---

## Chủ đề 20: Lập trình Game (Game Development) — Chương 68
Vòng lặp game bước cố định, hai bộ tích phân Euler, phát hiện va chạm và băm không gian, kiến trúc ECS hướng dữ liệu — toàn bộ lõi thuần túy, kiểm thử tất định.
- [Chương 68: Lập trình Game — Vòng Lặp, Vật Lý, Va Chạm & ECS (Game Development in Rust)](chuong_68.md)

---

## Chủ đề 21: Hệ thống giao dịch thuật toán (Algorithmic Trading Systems) — Chương 69
Bài tổng hợp cuối khóa: sổ lệnh ưu tiên giá–thời gian, động cơ khớp lệnh, cổng rủi ro bằng typestate, vị thế như một vị nhóm, và bộ kiểm định chiến lược không nhìn trộm tương lai.
- [Chương 69: Hệ thống giao dịch thuật toán — Sổ Lệnh, Khớp Lệnh & Kiểm Định Chiến Lược (Algorithmic Trading Systems)](chuong_69.md)

---

## Chủ đề 22: Blockchain & Web3 với Rust (Blockchain from Scratch) — Chương 70 đến 73
Dựng blockchain từ số không rồi mới dùng công cụ có sẵn: tự cài SHA-256 đối chiếu vector FIPS, cây Merkle có bằng chứng gộp, UTXO và bằng chứng công việc; mạng ngang hàng Kademlia, gossip và ngưỡng Byzantine đúng công thức; hợp đồng thông minh CosmWasm và Solana; và toàn bộ lớp mã hoá Ethereum — Keccak-256 kiểm chứng bằng chữ ký hàm ERC-20 công khai, ABI, RLP, EIP-1559.
- [Chương 70: Blockchain từ đầu — SHA-256, Cây Merkle, UTXO & Bằng chứng công việc (Building a Blockchain from Scratch)](chuong_70.md)
- [Chương 71: Mạng ngang hàng — Kademlia, Gossip & Đồng thuận Byzantine (P2P Networking)](chuong_71.md)
- [Chương 72: Hợp đồng thông minh với Rust — CosmWasm & Solana (Smart Contracts in Rust)](chuong_72.md)
- [Chương 73: Ethereum với Rust — Keccak-256, ABI, RLP & Alloy (Ethereum Tooling in Rust)](chuong_73.md)

---

## Chủ đề 23: Hệ sinh thái giao dịch tần suất cao (HFT Ecosystem) — Chương 74 đến 78, và 85
Năm chương dựng một hệ sinh thái HFT hoàn chỉnh, ở mức tương đương những gì Jane Street làm bằng OCaml: đo độ trễ theo phân vị và vòng Disruptor không cấp phát; giao thức nhị phân, phát hiện khe và sổ lệnh L2/L3; môi trường phục dựng phiên giao dịch thật qua ghi–phát lại có đồng hồ ảo và mô hình độ trễ; cổng rủi ro trước lệnh cùng định cỡ vị thế; và cuối cùng là thị trường blockchain với AMM, MEV và chênh lệch CEX–DEX.
- [Chương 74: Nền tảng HFT — Đo độ trễ, Vòng Disruptor & Bố cục bộ nhớ (HFT Foundations)](chuong_74.md)
- [Chương 75: Dữ liệu thị trường — Giao thức nhị phân, Phát hiện khe & Sổ lệnh (Market Data Pipeline)](chuong_75.md)
- [Chương 76: Phục dựng phiên giao dịch — Ghi phiên, Đồng hồ ảo & Phát lại (Session Capture & Replay)](chuong_76.md)
- [Chương 77: Chiến lược & Quản trị rủi ro — Cổng rủi ro, Tín hiệu & Định cỡ vị thế (Strategy & Risk Management)](chuong_77.md)
- [Chương 78: Thị trường blockchain — AMM, MEV & Chênh lệch giá CEX–DEX (Blockchain Market Microstructure)](chuong_78.md)
- [Chương 85: Hệ sinh thái HFT tích hợp — Nối mọi mảnh thành một hệ chạy được (Integrated HFT Ecosystem)](chuong_85.md) — **đọc sau cùng trong nhóm này**

---

## Chủ đề 24: Hiệu năng cấp phần cứng (Hardware-Level Performance) — Chương 79 đến 81
Ba chương về nơi phần mềm chạm trần vật lý. Tư duy FPGA cho giao dịch — tương ứng Rust của Hardcaml, tức là RHDL — nơi `if` trở thành bộ chọn và thời gian là hằng số tuyệt đối. Rồi hiệu năng CPU sâu theo tinh thần leetcpu.com: cache, dự đoán rẽ nhánh, ILP, SIMD. Và mô hình lập trình GPU theo tinh thần leetgpu.com: SIMT, phân kỳ warp, gộp truy cập, xung đột ngân hàng.
- [Chương 79: Tăng tốc phần cứng — Tư duy FPGA cho giao dịch (Hardcaml / RHDL)](chuong_79.md)
- [Chương 80: Hiệu năng CPU sâu — Cache, Dự đoán rẽ nhánh, ILP & SIMD (LeetCPU)](chuong_80.md)
- [Chương 81: Mô hình lập trình GPU — SIMT, Gộp truy cập & Bờ ngân hàng (LeetGPU)](chuong_81.md)

---

## Chủ đề 25: Tài chính định lượng bằng Rust (Quantitative Finance — OpenAlgo) — Chương 82 đến 84
Toàn bộ phần lập trình được của giáo trình OpenAlgo, cài lại từ đầu bằng Rust không thư viện ngoài: phân tích kỹ thuật với bất biến chống nhìn trộm tương lai được kiểm thử; định giá quyền chọn Black-Scholes, Greeks và biến động ngụ ý kèm giới hạn số học của nó; và chênh lệch thống kê với đồng liên kết, bộ lọc Kalman, kiểm định tiến và bằng chứng thực nghiệm về quá khớp.
- [Chương 82: Phân tích kỹ thuật bằng Rust — Nến, Chỉ báo & Bẫy nhìn trộm tương lai (Technical Analysis)](chuong_82.md)
- [Chương 83: Quyền chọn & Greeks bằng Rust — Black-Scholes, Biến động ngụ ý (Options & Greeks)](chuong_83.md)
- [Chương 84: Định lượng & Chênh lệch thống kê — Đồng liên kết, Kalman & Kiểm định tiến (Quant & Statistical Arbitrage)](chuong_84.md)

---

## Phụ lục

Tài liệu tra cứu, đọc sau khi đã hoàn thành các chủ đề tương ứng.
- [Lộ trình học tập: đồ thị phụ thuộc, bốn nhánh, bản đồ phủ nguồn tham khảo](ROADMAP.md) — đọc TRƯỚC khi bắt đầu
- [Phụ lục A: Bản đồ đầy đủ 24 Cấu trúc Đại số của Fantasy Land trong Rust](PHU_LUC_A_FANTASY_LAND.md) — đọc sau Chương 18–20
- [Bảng thuật ngữ Việt–Anh](THUAT_NGU.md) — tra cứu bất cứ lúc nào

---

*Chúc bạn có những trải nghiệm học tập tuyệt vời và vững vàng trên con đường trở thành một Kỹ sư Hệ thống Rust xuất sắc!*
