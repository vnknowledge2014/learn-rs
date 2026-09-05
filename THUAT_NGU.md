# Bảng Thuật Ngữ Việt – Anh (Vietnamese–English Glossary)

Tài liệu này chốt cách dịch thuật ngữ được dùng **nhất quán trong toàn bộ 84 chương**. Mục đích không chỉ là tra cứu: nó còn là **chiếc cầu bắc sang tài liệu tiếng Anh**. Khi bạn đọc xong giáo trình này và mở tài liệu chính thức của Rust hay một cuốn sách quốc tế, những từ bên cột phải sẽ không còn xa lạ.

**Quy ước dùng trong sách**: lần đầu một thuật ngữ xuất hiện, chúng tôi luôn viết dạng *tiếng Việt (tiếng Anh)*, ví dụ: "quyền sở hữu (ownership)". Những lần sau chỉ dùng bản tiếng Việt.

---

## 1. Nền tảng máy tính & Ngôn ngữ (Chương 01–05)

| Tiếng Việt | English | Ghi chú |
|---|---|---|
| Bóng bán dẫn | Transistor | |
| Nhịp xung nhịp | Clock cycle | Khác với *chu trình lệnh* (instruction cycle) |
| Chu trình Tìm nạp – Giải mã – Thực thi | Fetch–Decode–Execute cycle | |
| Địa chỉ ô nhớ | Memory address | |
| Bộ nhớ ngăn xếp | Stack | Cấp phát tự động, LIFO |
| Vùng nhớ tự do | Heap | Cấp phát động |
| Khung ngăn xếp | Stack frame | |
| Trình biên dịch | Compiler | `rustc` |
| Biểu thức / Câu lệnh | Expression / Statement | Biểu thức sinh giá trị, câu lệnh thì không |
| Trả về ngầm định | Implicit return | Dòng cuối không có `;` |
| Bất biến / Khả biến | Immutable / Mutable | |
| Con trỏ | Pointer | |
| Con trỏ béo | Fat pointer | 16 byte: địa chỉ + độ dài |
| Lát cắt | Slice | `&[T]`, `&str` |

## 2. Hệ thống sở hữu của Rust (Chương 06–12)

| Tiếng Việt | English | Ghi chú |
|---|---|---|
| Quyền sở hữu | Ownership | |
| Di chuyển | Move | Chuyển quyền sở hữu |
| Sao chép | Copy / Clone | `Copy` ngầm, `Clone` tường minh |
| Vay mượn | Borrow | `&T`, `&mut T` |
| Trình kiểm tra mượn | Borrow checker | |
| Độc quyền truy cập | Aliasing XOR Mutability | Nhiều `&T` HOẶC một `&mut T` |
| Thời gian sống / Vòng đời | Lifetime | Ký hiệu `'a` |
| Vòng đời không từ vựng | Non-Lexical Lifetimes (NLL) | |
| Quy tắc suy luận ngầm vòng đời | Lifetime elision rules | |
| Giải phóng hai lần | Double free | Lỗi kinh điển của C/C++ |
| Con trỏ lơ lửng | Dangling pointer | |
| Con trỏ thông minh | Smart pointer | `Box`, `Rc`, `Arc` |
| Khả biến nội tại | Interior mutability | `Cell`, `RefCell` |
| Kiểu dữ liệu đại số | Algebraic Data Type (ADT) | `struct` + `enum` |
| So khớp mẫu | Pattern matching | `match`, `if let` |
| Vét cạn | Exhaustiveness | `match` phải phủ hết mọi nhánh |
| Tối ưu hóa con trỏ rỗng | Null Pointer Optimization (NPO) | `Option<&T>` = 8 byte |
| Xổ cuộn ngăn xếp | Stack unwinding | Sau `panic!` |
| Lan truyền lỗi | Error propagation | Toán tử `?` |
| Giao ước hành vi | Trait | |
| Siêu trait | Supertrait | `ViNhom: NuaNhom` |
| Ràng buộc trait | Trait bound | `T: Display + Clone` |
| Đơn hình hóa | Monomorphization | Nguồn gốc của zero-cost |
| Đối tượng trait | Trait object | `dyn Trait` |
| Phân phối tĩnh / động | Static / Dynamic dispatch | `impl Trait` vs `Box<dyn Trait>` |
| Kiểu liên kết | Associated type | `type Item` |
| Quy tắc mồ côi | Orphan rule | Lý do tồn tại của kiểu bọc |
| Tính nhất quán cài đặt | Coherence | |

## 3. Lập trình hàm (Chương 13–20)

| Tiếng Việt | English | Ghi chú |
|---|---|---|
| Lập trình hàm | Functional Programming (FP) | |
| Lập trình mệnh lệnh / khai báo | Imperative / Declarative | |
| Hàm thuần túy | Pure function | Tất định + không tác dụng phụ |
| Tác dụng phụ | Side effect | |
| **Minh bạch tham chiếu** | **Referential transparency** | Trụ cột 1 của FP |
| Suy luận bằng đẳng thức | Equational reasoning | |
| **Hàm toàn phần / Hàm bộ phận** | **Total / Partial function** | `.unwrap()` biến toàn phần thành bộ phận |
| Ngôi (số tham số) | Arity | |
| Tính lũy đẳng | Idempotence | `f(f(x)) = f(x)` |
| Vị từ | Predicate | Hàm trả `bool` |
| **Phép ghép hàm** | **Function composition** | Trụ cột 2 của FP |
| Curry hóa | Currying | |
| Áp dụng từng phần | Partial application | Nền tảng của tiêm phụ thuộc |
| Lối viết không nêu tham số | Point-free style | `.map(str::trim)` |
| Bộ kết hợp | Combinator | `identity`, `flip`, `const` |
| Phép tiếp nối | Continuation (CPS) | |
| Hàm ẩn danh / Closure | Closure | Cú pháp `\|x\| x + 1` |
| Bắt giữ môi trường | Environment capturing | `Fn`, `FnMut`, `FnOnce` |
| Hàm bậc cao | Higher-order function (HOF) | |
| Đánh giá lười biếng | Lazy evaluation | Iterator, `Future` |
| Bộ lặp / Bộ điều hợp / Hàm tiêu thụ | Iterator / Adapter / Consumer | |
| Gấp trái / Gấp phải | fold / rfold | |
| Tính kết hợp | Associativity | Cho phép song song hóa |
| Tính giao hoán | Commutativity | Cho phép đảo thứ tự |
| Magma | Magma | Phép toán hai ngôi đóng kín |
| **Nửa nhóm** | **Semigroup** | Magma + kết hợp |
| **Vị nhóm** | **Monoid** | Nửa nhóm + phần tử đơn vị |
| Nhóm / Nhóm giao hoán | Group / Abelian group | |
| Nửa vành / Vành | Semiring / Ring | |
| Phần tử đơn vị | Identity element | `0` cho cộng, `1` cho nhân, `""` cho chuỗi |
| Kiểu có quan hệ bằng | Setoid | Trong Rust: `PartialEq` / `Eq` |
| Luật | Law | Đẳng thức mà một trừu tượng phải luôn thỏa |
| Kiểm thử theo tính chất | Property-based testing | `proptest`, `quickcheck` |
| **Hàm tử** | **Functor** | Có `map`, tuân 2 luật |
| Hàm tử hai ngôi | Bifunctor | `Result`: `map` + `map_err` |
| Hàm tử nghịch biến | Contravariant functor | |
| Profunctor | Profunctor | |
| **Hàm tử áp dụng** | **Applicative functor** | Gộp ngữ cảnh độc lập, tích lũy lỗi |
| **Đơn nguyên** | **Monad** | `and_then` chính là `bind` |
| Phép buộc | bind / chain / flatMap | Trong Rust: `and_then` |
| Phép làm phẳng | join / flatten | |
| Phép ghép Kleisli | Kleisli composition | |
| Đối đơn nguyên | Comonad | |
| Kiểu duyệt được | Traversable | `collect::<Result<Vec<_>,E>>()` |
| Phép đảo ngữ cảnh | sequence / traverse | `Vec<Result>` → `Result<Vec>` |
| Kiểu gấp được | Foldable | |
| Kiểu có lựa chọn thay thế | Alternative | `Option::or` |
| Kiểu bậc cao | Higher-Kinded Type (HKT) | Rust chưa hỗ trợ trực tiếp |
| Đẳng cấu / Đồng cấu | Isomorphism / Homomorphism | |
| Phép gấp / Phép mở | Catamorphism / Anamorphism | |
| Phạm trù | Category | |
| Kiểu tích / Kiểu tổng | Product type / Sum type | `struct` / `enum` |
| Lực lượng của kiểu | Cardinality | Số trạng thái biểu diễn được |
| **Kiểu bọc** | **Newtype** | `struct Email(String)` |
| **Hàm khởi tạo có kiểm chứng** | **Smart constructor** | `Email::phan_tich` |
| Phân tích, đừng xác thực | Parse, don't validate | |
| Biến trạng thái sai thành không biểu diễn được | Make illegal states unrepresentable | |
| Trạng thái ghi trong kiểu | Typestate | `DonHang<DaThanhToan>` |
| Lập trình hai đường ray | Railway-Oriented Programming (ROP) | |
| Lõi thuần túy – vỏ mệnh lệnh | Functional core, imperative shell | |
| Kiểu truyền tải | Data Transfer Object (DTO) | Khác kiểu miền |
| Tiêm phụ thuộc | Dependency Injection | Trong FP: áp dụng từng phần |
| Đệ quy đuôi | Tail recursion | Rust **không** bảo đảm tối ưu hóa |
| Ghi nhớ kết quả | Memoization | |
| Sao chép khi ghi | Copy-on-write | `Cow<'_, str>` |
| Cấu trúc dữ liệu bền vững | Persistent data structure | crate `im`, `rpds` |
| Chia sẻ cấu trúc | Structural sharing | Nhờ `Rc` / `Arc` |
| Thấu kính | Lens / Optics | |

## 4. Siêu lập trình (Chương 21–24)

| Tiếng Việt | English |
|---|---|
| Siêu lập trình | Metaprogramming |
| Macro khai báo / thủ tục | Declarative / Procedural macro |
| Dòng thẻ bài | TokenStream |
| Cây cú pháp trừu tượng | Abstract Syntax Tree (AST) |
| Tính vệ sinh | Hygiene |
| Ngữ cảnh cú pháp | SyntaxContext |
| Bộ chỉ định cú pháp | Syntax designator (`expr`, `ident`, `ty`…) |
| Bộ nhai thẻ bài | TT Muncher |
| Thuộc tính bổ trợ | Helper attribute |

## 5. Cấu trúc dữ liệu & Thuật toán (Chương 25–30)

| Tiếng Việt | English |
|---|---|
| Độ phức tạp tính toán | Computational complexity |
| Độ phức tạp thời gian / không gian | Time / Space complexity |
| Thời gian khấu hao | Amortized time |
| Danh sách liên kết | Linked list |
| Ngăn xếp / Hàng đợi / Hàng đợi hai đầu | Stack / Queue / Deque |
| Vòng đệm tròn | Circular buffer |
| Cây nhị phân tìm kiếm | Binary Search Tree (BST) |
| Cây suy biến | Degenerate tree |
| Duyệt trung / tiền / hậu thứ tự | In-order / Pre-order / Post-order traversal |
| Bảng băm | Hash table |
| Trượt bộ nhớ đệm | Cache miss |
| Danh sách kề | Adjacency list |

## 6. Cơ sở dữ liệu (Chương 31–36)

| Tiếng Việt | English |
|---|---|
| Thứ tự byte nhỏ / lớn trước | Little-Endian / Big-Endian |
| Trang có khe | Slotted page |
| Bể đệm | Buffer pool |
| Cờ bẩn | Dirty flag |
| Định danh bản ghi | Tuple ID / RID |
| Hệ số phân nhánh | Branching factor / Fan-out |
| Phân tách nút | Node splitting |
| Nhật ký ghi trước | Write-Ahead Log (WAL) |
| Bia mộ | Tombstone |
| Nén gộp | Compaction |
| Giao dịch | Transaction |
| Nguyên tử / Nhất quán / Cô lập / Bền vững | Atomicity / Consistency / Isolation / Durability |
| Đọc rác / Đọc không lặp lại / Đọc bóng ma | Dirty / Non-repeatable / Phantom read |
| Kiểm soát đồng thời đa phiên bản | MVCC |

## 7. An toàn thông tin (Chương 37–42)

| Tiếng Việt | English |
|---|---|
| Không gian địa chỉ ảo | Virtual address space |
| Tràn bộ đệm | Buffer overflow |
| Dùng sau khi giải phóng | Use-After-Free (UAF) |
| Lỗ hổng chuỗi định dạng | Format string vulnerability |
| Ngẫu nhiên hóa bố cục địa chỉ | ASLR |
| Chống thực thi vùng dữ liệu | DEP / NX |
| Kim tuyến ngăn xếp | Stack canary |
| Hành vi bất định | Undefined Behavior (UB) |
| Tấn công kênh kề theo thời gian | Timing attack |
| So sánh thời gian bất biến | Constant-time comparison |
| Mô hình hóa mối đe dọa | Threat modeling (STRIDE) |

## 8. Lập trình cùng AI (Chương 43–47)

| Tiếng Việt | English |
|---|---|
| Cửa sổ ngữ cảnh | Context window |
| Đơn vị token | Token |
| Bị lãng quên ở giữa | Lost in the middle |
| Phát triển dựa trên đặc tả | Spec-Driven Development (SDD) |
| Vòng lặp tự sửa lỗi | Self-correction loop |
| Phát triển hướng kiểm thử | Test-Driven Development (TDD) |

## 9. Hệ phân tán (Chương 48–54)

| Tiếng Việt | English |
|---|---|
| Khối đơn / Khối đơn hướng module | Monolith / Modular monolith |
| Vi dịch vụ | Microservice |
| Ngữ cảnh giới hạn | Bounded context |
| Ngôn ngữ chung | Ubiquitous language |
| Ngắt mạch tự động | Circuit breaker |
| Phân vùng chống tràn | Bulkhead |
| Bất đồng bộ | Asynchronous |
| Đa dồn kênh sự kiện | Event multiplexing (epoll / kqueue) |
| Máy trạng thái lười | Lazy state machine |
| Thuật toán cắp việc | Work-stealing |
| Đa nhiệm cộng tác | Cooperative multitasking |
| Mô hình Actor | Actor model |
| Hòm thư | Mailbox |
| Đàn bò giẫm đạp | Cache stampede |
| Thủng bộ đệm | Cache penetration |
| Tuyết lở bộ đệm | Cache avalanche |
| Định lý CAP | CAP theorem |
| Đồng thuận | Consensus |
| Nhiệm kỳ / Sao chép nhật ký | Term / Log replication |
| Đa số phiếu | Quorum |
| Nhật ký sự kiện | Event sourcing |
| Máy trạng thái hữu hạn | Finite State Machine (FSM) |

---

## 10. Kiểm thử, Tác tử AI, Bảo mật Web, Dữ liệu & Web/Desktop (Chương 55–63)

| Tiếng Việt | English | Ghi chú |
|---|---|---|
| Kim tự tháp kiểm thử | Testing pyramid | Nhiều unit, ít E2E — ngược lại là "nón kem kiểm thử" |
| Phát triển hướng kiểm thử | Test-Driven Development (TDD) | Đỏ → Xanh → Tái cấu trúc |
| Phát triển hướng hành vi | Behavior-Driven Development (BDD) | Cho trước – Khi – Thì |
| Kiểm thử tích hợp | Integration test | Thư mục `tests/` là một crate RIÊNG, chỉ thấy API công khai |
| Kiểm thử đầu-cuối | End-to-End (E2E) test | Chậm và giòn — dùng ít, chọn kỹ |
| Kiểm thử trong tài liệu | Doctest | Ví dụ trong `///` được `cargo test` chạy thật |
| Đóng thế kiểm thử | Test double | Nhóm chung của stub, spy, mock, fake |
| Kiểm thử theo tính chất | Property-based testing | Kiểm chứng luật trên hàng nghìn đầu vào sinh tự động |
| Kiểm thử mờ | Fuzzing | Ném dữ liệu hỗn loạn để tìm panic |
| Kỹ nghệ ngữ cảnh | Context engineering | Ngân sách ngữ cảnh như bài toán cái túi |
| Lạc giữa dòng | Lost in the middle | Mô hình nhớ đầu và cuối tốt hơn giữa |
| Kỹ nghệ bộ khung | Harness engineering | Công cụ như hợp đồng kiểu, có danh sách cho phép |
| Kỹ nghệ vòng lặp | Loop engineering | Ba cái phanh: hoàn thành, hết ngân sách, phát hiện lặp |
| Truy xuất lan tỏa | Multi-hop retrieval | Nền của GraphRAG |
| Chèn câu lệnh SQL | SQL Injection | Chữa bằng tham số hóa, không phải bằng lọc chuỗi |
| Kịch bản chéo trang | Cross-Site Scripting (XSS) | Thoát ký tự **theo ngữ cảnh** đích |
| Tham chiếu đối tượng trực tiếp không an toàn | IDOR | Luôn kiểm tra quyền sở hữu, không chỉ đăng nhập |
| Giả mạo yêu cầu phía máy chủ | SSRF | Chặn cả `169.254.169.254` — cổng siêu dữ liệu đám mây |
| Duyệt đường dẫn | Path traversal | Chuẩn hóa đường dẫn **trước** khi kiểm tra |
| So sánh bất biến thời gian | Constant-time comparison | Chống tấn công qua kênh thời gian |
| Lưu trữ theo cột | Columnar storage | Nền của Parquet, Arrow, Polars |
| Trích xuất–Biến đổi–Nạp | ETL (Extract-Transform-Load) | Mỗi bước là một hàm thuần túy |
| Hàm cửa sổ | Window function | Tính trên một khung trượt quanh mỗi dòng |
| Phép nối băm | Hash join | Dựng bảng băm từ bên nhỏ, quét bên lớn |
| Thống kê bền vững | Robust statistics | Trung vị + MAD thay trung bình + độ lệch chuẩn |
| Cân bằng tải | Load balancing | Xoay vòng, ít kết nối, theo trọng số |
| Băm nhất quán | Consistent hashing | Thêm/bớt máy chủ chỉ xáo trộn 1/n khóa |
| Nút ảo | Virtual node | Làm phẳng phân bố tải trên vòng băm |
| Xô token | Token bucket | Giới hạn tần suất mà vẫn cho phép bùng phát ngắn |
| Áp lực ngược | Back-pressure | Hàng đợi có giới hạn — từ chối còn hơn sập |
| Quy hoạch động | Dynamic programming | Cần bài toán con chồng lặp + cấu trúc con tối ưu |
| Quay lui | Backtracking | Thử → đệ quy → lùi lại |
| Tham lam | Greedy | Nhanh, nhưng phải chứng minh mới dám tin |
| Bộ trích xuất | Extractor | Cách Axum biến phần của yêu cầu thành tham số hàm |
| Tín hiệu | Signal | Đơn vị phản ứng của Leptos/SolidJS |
| DOM ảo | Virtual DOM | So sánh cây rồi chỉ vá phần khác |
| Kiến trúc Elm | The Elm Architecture | Mô hình – Thông điệp – `update` |
| Giao tiếp liên tiến trình | IPC | Cầu nối giữa mặt tiền web và lõi Rust trong Tauri |

---

## 11. Hệ điều hành & Mạng máy tính (Chương 64–65)

| Tiếng Việt | English | Ghi chú |
|---|---|---|
| Khối điều khiển tiến trình | Process Control Block (PCB) | Thứ nhân hệ điều hành lưu cho mỗi tiến trình |
| Chuyển ngữ cảnh | Context switch | Chi phí gián tiếp (mất cache) lớn hơn chi phí trực tiếp |
| Lượng tử thời gian | Time quantum / time slice | Đơn vị CPU cấp cho mỗi lượt Round-Robin |
| Hiệu ứng đoàn xe | Convoy effect | Nhược điểm kinh điển của FCFS |
| Đói (tài nguyên) | Starvation | Rủi ro cố hữu của lập lịch theo ưu tiên |
| Tiếm quyền | Preemption | Giành CPU khỏi tiến trình đang chạy |
| Lỗi trang | Page fault | Chậm hơn truy cập RAM khoảng 100 000 lần |
| Khung nhớ | Page frame | Ô chứa một trang trong RAM vật lý |
| Thay trang | Page replacement | FIFO, LRU, Clock, Tối ưu (Bélády) |
| Nghịch lý Bélády | Bélády's anomaly | Thêm khung nhớ mà lỗi trang lại tăng — chỉ xảy ra với FIFO |
| Thuật toán ngăn xếp | Stack algorithm | Lớp thuật toán miễn nhiễm nghịch lý Bélády, gồm LRU |
| Nguyên lý cục bộ | Principle of locality | Nền tảng của mọi bộ nhớ đệm |
| Bế tắc | Deadlock | Bốn điều kiện Coffman phải cùng đúng |
| Đồ thị chờ đợi | Wait-for graph | Có chu trình = có bế tắc |
| Vùng găng | Critical section | Đoạn mã không được để ngắt xen vào |
| Đóng gói (theo tầng) | Encapsulation | Mỗi tầng bọc dữ liệu tầng trên bằng phần đầu của mình |
| Phần đầu | Header | Phần siêu dữ liệu đứng trước tải trọng |
| Tải trọng | Payload | Dữ liệu thật, phân biệt với bao bì |
| Bắt tay ba bước | Three-way handshake | SYN → SYN+ACK → ACK |
| Cửa sổ tắc nghẽn | Congestion window (cwnd) | Số gói được phép bay chưa xác nhận |
| Khởi động chậm | Slow start | Tên gây hiểu lầm: thực ra tăng theo cấp số nhân |
| Tránh tắc nghẽn | Congestion avoidance | Pha tăng tuyến tính sau khi chạm ngưỡng |
| Tăng cộng, giảm nhân | AIMD | Nguồn gốc của tính công bằng trên Internet |
| Tổng kiểm tra | Checksum | Bù-1 16-bit; nhanh nhưng không bắt được hoán vị |
| Khớp tiền tố dài nhất | Longest prefix match | Quy tắc cốt lõi của mọi bộ định tuyến |
| Địa chỉ quảng bá | Broadcast address | Địa chỉ cuối của một mạng con |
| Cửa sổ trượt | Sliding window | Nền của truyền tin cậy |
| Quay lại N | Go-Back-N | Gửi lại từ gói mất trở đi — đơn giản, tốn băng thông |
| Lặp lại chọn lọc | Selective Repeat | Chỉ gửi lại gói mất; nền của tùy chọn SACK |

---

## 12. Hệ thống nhúng & Phần cứng số (Chương 66–67)

| Tiếng Việt | English | Ghi chú |
|---|---|---|
| Không thư viện chuẩn | `no_std` | Mất `std`, giữ nguyên `core` và toàn bộ hệ thống kiểu |
| Thanh ghi ánh xạ bộ nhớ | Memory-mapped register (MMIO) | Ghi vào địa chỉ = điều khiển phần cứng thật |
| Dễ biến động | Volatile | Cấm trình tối ưu hóa gộp hoặc xóa lệnh truy cập |
| Đọc-Sửa-Ghi | Read-Modify-Write | Không nguyên tử — dễ hỏng nếu ngắt xen vào |
| Trạng thái trong kiểu | Typestate | Trạng thái nằm trong kiểu, chi phí lúc chạy bằng không |
| Kiểu rỗng / nhãn kiểu | Zero-sized type, marker type | `PhantomData` không chiếm byte nào |
| Chống rung phím | Debounce | Lọc nhiễu cơ khí của nút bấm |
| Số dấu phẩy tĩnh | Fixed-point arithmetic | Q16.16 — thay dấu phẩy động khi chip không có FPU |
| Bộ đệm vòng | Ring buffer / circular buffer | Kiểu dữ liệu chủ lực của ngắt UART |
| Một-sản-xuất-một-tiêu-thụ | SPSC queue | Không cần khóa nếu mỗi con trỏ chỉ một bên ghi |
| Điện trở kéo lên / kéo xuống | Pull-up / pull-down resistor | Chân thả nổi cho giá trị đọc ngẫu nhiên |
| Bảng tra | Look-Up Table (LUT) | Khối logic vạn năng của FPGA |
| Mạch tổ hợp | Combinational logic | Đầu ra chỉ phụ thuộc đầu vào hiện tại |
| Mạch tuần tự | Sequential logic | Có trí nhớ, cập nhật theo xung nhịp |
| Cổng phổ dụng | Universal gate | NAND — dựng được mọi hàm logic |
| Giá trị điều khiển | Controlling value | 0 với AND, 1 với OR; XOR không có |
| Nhớ nối tiếp | Ripple-carry | Độ trễ tỉ lệ thuận số bit |
| Nhìn trước nhớ | Carry-lookahead | Đổi diện tích lấy tốc độ |
| Sinh nhớ / truyền nhớ | Generate / Propagate | Hai tín hiệu nền của carry-lookahead |
| Sườn lên | Rising edge | Khoảnh khắc flip-flop chốt giá trị |
| Thanh ghi dịch | Shift register | Nền của SPI, UART, CRC |
| Đường ống | Pipeline | Tăng thông lượng, **không** giảm độ trễ |
| Độ trễ | Latency | Thời gian cho **một** phần tử đi hết |
| Thông lượng | Throughput | Số phần tử hoàn thành mỗi đơn vị thời gian |
| Danh sách nối | Netlist | Mô tả mạch dưới dạng đồ thị các cổng |
| Đường tới hạn | Critical path | Chuỗi cổng dài nhất — quyết định tần số tối đa |
| Vòng lặp tổ hợp | Combinational loop | Mạch không bao giờ ổn định — lỗi thiết kế |
| Tổng hợp mạch | Synthesis | Biên dịch HDL thành netlist |

---

## 13. Game & Giao dịch thuật toán (Chương 68–69)

| Tiếng Việt | English | Ghi chú |
|---|---|---|
| Bước cố định | Fixed timestep | Vật lý phải độc lập với tốc độ khung hình |
| Bộ tích lũy thời gian | Time accumulator | Dùng số nguyên để tránh trôi sai số |
| Xoắn ốc tử thần | Spiral of death | Nợ thời gian chồng chất làm game treo |
| Nội suy | Interpolation | Làm mượt hình ảnh giữa hai bước vật lý |
| Euler tường minh | Explicit / forward Euler | Bơm năng lượng — hệ dao động sẽ nổ |
| Euler nửa ẩn | Semi-implicit / symplectic Euler | Ổn định năng lượng; mặc định của mọi game engine |
| Hộp bao thẳng trục | AABB (Axis-Aligned Bounding Box) | Kiểm tra giao nhau chỉ cần 2 phép so |
| Định lý trục tách | Separating Axis Theorem | Nền lý thuyết của phát hiện va chạm hình lồi |
| Vector đẩy tối thiểu | Minimum Translation Vector | Đẩy theo trục chồng lấn ít nhất |
| Băm không gian | Spatial hashing | Cắt O(n²) xuống gần O(n) |
| Lọc thô | Broad phase | Bước loại nhanh trước khi kiểm tra chính xác |
| Thực thể–Thành phần–Hệ thống | Entity-Component-System (ECS) | Thay kế thừa bằng mảng dữ liệu phẳng |
| Thiết kế hướng dữ liệu | Data-oriented design | Tối ưu cho cache CPU, không cho mô hình khái niệm |
| Nguyên mẫu | Archetype | Nhóm thực thể cùng tập thành phần vào một khối liên tục |
| Thế hệ (mã thực thể) | Generation | Chống tham chiếu treo khi tái dùng mã số |
| Sổ lệnh giới hạn | Limit order book | Trái tim của mọi sàn giao dịch |
| Ưu tiên giá–thời gian | Price-time priority | Giá tốt hơn thắng; cùng giá thì ai trước thắng |
| Chênh lệch mua-bán | Bid-ask spread | Chi phí ẩn của mọi giao dịch |
| Giá giữa | Mid price | Ước lượng giá trị thật tốt hơn giá khớp gần nhất |
| Cải thiện giá | Price improvement | Khớp ở giá của lệnh đã nằm sẵn trong sổ |
| Động cơ khớp lệnh | Matching engine | Bất biến sống còn: khối lượng được bảo toàn |
| Khớp ngay hoặc hủy | Immediate-or-Cancel (IOC) | Khớp được bao nhiêu hay bấy nhiêu |
| Khớp toàn bộ hoặc hủy | Fill-or-Kill (FOK) | Không khớp đủ thì không khớp gì |
| Trượt giá | Slippage | Luôn mua đắt hơn, bán rẻ hơn giá lý thuyết |
| Kiểm định trên quá khứ | Backtesting | Chỉ đáng tin khi không nhìn trộm tương lai |
| Nhìn trộm tương lai | Look-ahead bias | Lỗi khiến mọi chiến lược trông như in tiền |
| Sụt giảm tối đa | Maximum drawdown | Quan trọng hơn lợi nhuận: quyết định bạn có trụ nổi không |
| Khớp quá mức | Overfitting | Tối ưu vào nhiễu của một bộ dữ liệu cụ thể |
| Nguồn sự kiện | Event sourcing | Nhật ký là chân lý; trạng thái chỉ là kết quả phát lại |

---

## 14. Blockchain & Mạng ngang hàng (Chương 70–73)

| Tiếng Việt | English | Ghi chú |
|---|---|---|
| Hàm băm mật mã | Cryptographic hash function | SHA-256, Keccak-256 |
| Đệm (thông điệp) | Padding | Quy tắc FIPS 180-4 |
| Cây Merkle | Merkle tree | |
| Gốc Merkle | Merkle root | Nằm trong phần đầu khối |
| Bằng chứng gộp | Inclusion proof / Merkle proof | Chỉ cần log₂(n) giá trị băm |
| Đầu ra chưa tiêu | UTXO — Unspent Transaction Output | Mô hình của Bitcoin |
| Bằng chứng công việc | Proof of Work | |
| Độ khó | Difficulty | Số bit 0 dẫn đầu |
| Số dùng một lần | Nonce | |
| Tái tổ chức chuỗi | Chain reorganization / reorg | Chọn theo **tổng công việc**, không theo chiều dài |
| Tiêu hai lần | Double spend | |
| Ví nhẹ | Light client / SPV wallet | Chỉ tải phần đầu khối |
| Khoảng cách XOR | XOR metric | Nền của Kademlia |
| Thùng k | k-bucket | Ưu tiên giữ nút cũ |
| Lan truyền tin đồn | Gossip protocol | |
| Hệ số phát tán | Fanout | Đánh đổi độ trễ ↔ băng thông |
| Chống entropy | Anti-entropy | Bảo đảm hội tụ chắc chắn |
| Lỗi Byzantine | Byzantine fault | Nút nói dối, không chỉ nút chết |
| Ngưỡng quorum | Quorum threshold | Đúng: `⌊(n+f)/2⌋+1`, **không** phải `2f+1` |
| Tính an toàn / tính sống | Safety / Liveness | Hai điều kiện phải kiểm cùng lúc |
| Nói đôi mặt | Equivocation | Bằng chứng tự chứng minh, nền của phạt cắt cọc |
| Phạt cắt cọc | Slashing | |
| Hợp đồng thông minh | Smart contract | |
| Địa chỉ suy ra từ chương trình | PDA — Program Derived Address | Nằm **ngoài** đường cong ed25519 |
| Bump chuẩn | Canonical bump | Luôn dùng bump lớn nhất hợp lệ |
| Nhầm lẫn kiểu tài khoản | Account type confusion | Chữa bằng byte định danh |
| Định danh | Discriminator | Anchor thêm 8 byte |
| Kiểm tra – tác động – tương tác | Checks-Effects-Interactions | Mẫu chống tái nhập |
| Tái nhập | Reentrancy | |
| Mã hoá ABI | ABI encoding | Mọi thứ là từ 32 byte |
| Chữ ký hàm | Function selector | 4 byte đầu của `keccak(chữ_ký)` |
| Tiền tố độ dài đệ quy | RLP — Recursive Length Prefix | Đòi mã hoá tối giản |
| Phí cơ sở | Base fee | Bị **đốt**, không về tay người xác thực |
| Tiền bo | Priority fee / tip | |
| Hàng đợi giao dịch | Mempool | |
| Địa chỉ có mã kiểm | Checksummed address | EIP-55 |

---

## 15. Giao dịch tần suất cao (Chương 74–78)

| Tiếng Việt | English | Ghi chú |
|---|---|---|
| Giao dịch tần suất cao | HFT — High-Frequency Trading | |
| Độ trễ dây tới lệnh | Tick-to-trade latency | Thước đo trung tâm |
| Phân vị | Percentile | p50, p99, p99.9 — **không** dùng trung bình |
| Độ trễ đuôi | Tail latency | Thứ thực sự giết chiến lược |
| Biểu đồ phân vị | Latency histogram | Kiểu HDR, thùng logarit |
| Chia sẻ giả | False sharing | Chậm 5–10× mà không tranh chấp logic |
| Dòng cache | Cache line | 64 byte |
| Đệm theo dòng cache | Cache-line padding | |
| Vòng Disruptor | Disruptor ring buffer | SPSC, không khoá, không cấp phát |
| Bể đối tượng | Object pool | Cấp phát trước, tái dùng |
| Đường nóng | Hot path | Nơi cấm mọi cấp phát |
| Mảng cấu trúc / Cấu trúc mảng | AoS / SoA | GPU ưa SoA, CPU tuỳ cách truy cập |
| Ngân sách độ trễ | Latency budget | Chỉ ra nên tối ưu ở đâu |
| Giao thức nhị phân | Binary protocol | ITCH, trường độ dài cố định |
| Thứ tự byte mạng | Network byte order | Big-endian |
| Phát hiện khe | Gap detection | Yêu cầu phát lại **đúng một lần** |
| Đang chờ khôi phục | Pending recovery | Trạng thái chống bão yêu cầu |
| Sổ lệnh | Order book | |
| Mức L2 / L3 | Level 2 / Level 3 | L3 cho biết vị trí xếp hàng |
| Vị trí xếp hàng | Queue position | Quyết định lãi lỗ nhà tạo lập |
| Ưu tiên giá–thời gian | Price-time priority | |
| Chọn lọc bất lợi | Adverse selection | Được khớp đúng lúc không nên khớp |
| Ghi phiên & phát lại | Capture & replay | |
| Đồng hồ ảo | Virtual clock | Nguồn thời gian **duy nhất** |
| Đẩy tốc độ phát | Replay speed scaling | ×1, ×1000, vô hạn |
| Mô hình độ trễ | Latency model | Có cả jitter, không chỉ hằng số |
| Nhìn trộm tương lai | Look-ahead bias | Bỏ qua độ trễ là dạng tinh vi nhất |
| Tính tất định | Determinism | `BTreeMap`, không `HashMap` |
| Tác động thị trường | Market impact | Quy luật căn bậc hai |
| Cổng rủi ro trước lệnh | Pre-trade risk gate | Không được có đường vòng |
| Công tắc ngắt | Kill switch | |
| Giá vốn trung bình | Average cost basis | Phải xử lý riêng ca đảo chiều |
| Mất cân bằng sổ lệnh | Order book imbalance | |
| Vi giá | Micro-price | Trọng số **ngược** với khối lượng |
| Công thức Kelly | Kelly criterion | Thực tế dùng ¼–½ Kelly |
| Sụt giảm tối đa | Maximum drawdown | Quan trọng hơn lợi nhuận |
| Tỉ số Sharpe | Sharpe ratio | Phạt cả biến động tăng |
| Nhà tạo lập thị trường | Market maker | |
| Tạo lập tự động | AMM — Automated Market Maker | `x·y = k` |
| Trượt giá | Slippage | |
| Tổn thất tạm thời | Impermanent loss | `2√r/(1+r) − 1` |
| Giá trị moi được tối đa | MEV — Maximal Extractable Value | |
| Kẹp lệnh | Sandwich attack | Chống bằng số nhận tối thiểu chặt |
| Số nhận tối thiểu | Minimum amount out | Phòng vệ **duy nhất** có hiệu lực |
| Đấu giá theo lô | Batch auction | CoW Swap — mọi lệnh cùng giá |
| Trùng hợp mong muốn | Coincidence of wants | Khớp trực tiếp, không qua bể |
| Tìm kiếm tam phân | Ternary search | Cho hàm lợi nhuận lõm |

---

## 16. Hiệu năng cấp phần cứng (Chương 79–81)

| Tiếng Việt | English | Ghi chú |
|---|---|---|
| Mảng cổng lập trình được | FPGA | |
| Bảng tra cứu | LUT — Look-Up Table | LUT-6 = 64 bit SRAM |
| Bộ nhớ lật | Flip-flop | Một bit trạng thái |
| Bộ chọn | Multiplexer | `if` trên FPGA trở thành cái này |
| Logic tổ hợp | Combinational logic | Không có trạng thái |
| Độ sâu tổ hợp | Combinational depth | Quyết định tần số tối đa |
| Đường tới hạn | Critical path | |
| Cây rút gọn | Reduction tree | Độ sâu `log₂(n)` |
| Đường ống | Pipeline | Đánh đổi độ trễ ↔ thông lượng |
| Thời gian hằng số | Constant-time | Không dự đoán sai, không xả ống |
| Tổng hợp | Synthesis | Mất hàng giờ — đừng đặt logic hay sửa lên FPGA |
| Phân cấp bộ nhớ | Memory hierarchy | L1 ~4, RAM ~300 chu kỳ |
| Trượt cache | Cache miss | |
| Tính cục bộ | Locality | Không gian và thời gian |
| Cache tập hợp liên kết | Set-associative cache | Nguồn của **trượt do xung đột** |
| Trượt do xung đột | Conflict miss | Chữa bằng đệm một phần tử |
| Chia khối | Blocking / Tiling | Cùng phép tính, ít trượt hơn một bậc |
| Dự đoán rẽ nhánh | Branch prediction | 2-bit bão hoà |
| Dự đoán sai | Branch misprediction | ~15 chu kỳ |
| Mã không rẽ nhánh | Branchless code | Thắng với dữ liệu ngẫu nhiên |
| Song song mức lệnh | ILP — Instruction-Level Parallelism | |
| Chuỗi phụ thuộc | Dependency chain | Kẻ thù của ILP |
| Nhiều biến tích luỹ | Multiple accumulators | Phá chuỗi phụ thuộc |
| Bộ nạp trước | Prefetcher | Có thể làm hỏng phép đo |
| Một lệnh nhiều dữ liệu | SIMD | AVX2 = 4×f64 |
| Phần dư | Remainder / tail | Lỗi phổ biến nhất khi viết SIMD tay |
| Một lệnh nhiều luồng | SIMT | Mô hình của GPU |
| Bó luồng | Warp | 32 luồng, chạy đồng bộ tuyệt đối |
| Phân kỳ bó luồng | Warp divergence | Chi phí = số nhánh khác nhau |
| Gộp truy cập | Memory coalescing | Yếu tố hiệu năng số một |
| Giao dịch bộ nhớ | Memory transaction | 128 byte |
| Ngân hàng bộ nhớ | Memory bank | 32 ngân hàng ở bộ nhớ chia sẻ |
| Xung đột ngân hàng | Bank conflict | Chữa bằng mẹo đệm `+1` |
| Bộ nhớ chia sẻ | Shared memory | |
| Rào chắn đồng bộ | Synchronization barrier | `__syncthreads()` |
| Trao đổi trong bó | Warp shuffle | Không cần rào chắn |
| Rút gọn song song | Parallel reduction | Cây, không phải vòng lặp |
| Cường độ số học | Arithmetic intensity | Phép tính trên mỗi byte đọc |
| Mức chiếm dụng | Occupancy | Cao không phải lúc nào cũng tốt |
| Bị chặn bởi bộ nhớ / sức tính | Memory-bound / Compute-bound | Mục tiêu của chia lát là chuyển từ trái sang phải |

---

## 17. Tài chính định lượng (Chương 82–84)

| Tiếng Việt | English | Ghi chú |
|---|---|---|
| Nến | Candlestick | Mở, cao, thấp, đóng |
| Thân nến / bóng nến | Body / Wick (shadow) | |
| Trung bình trượt giản đơn | SMA — Simple Moving Average | Trọng số phẳng |
| Trung bình trượt luỹ thừa | EMA — Exponential Moving Average | Cách khởi tạo **đổi kết quả** |
| Chỉ số sức mạnh tương đối | RSI | "Quá mua" ≠ "nên bán" |
| Phân kỳ hội tụ trung bình trượt | MACD | EMA của EMA — độ trễ cộng dồn |
| Dải Bollinger | Bollinger Bands | Giả định phân phối chuẩn mà lợi suất thì không |
| Khoảng thật | True Range | Ba vế vì có khoảng nhảy giá |
| Khoảng thật trung bình | ATR — Average True Range | Tốt nhất để định cỡ dừng lỗ |
| Khoảng nhảy giá | Gap | |
| Giá bình quân theo khối lượng | VWAP | Chuẩn đánh giá thực thi |
| Phân kỳ | Divergence | Xác nhận muộn `nhin_lai` phiên |
| Đuôi béo | Fat tail | Cực đoan xảy ra thường hơn mô hình |
| Quyền chọn mua / bán | Call / Put option | |
| Giá thực hiện | Strike price | |
| Trong tiền / ngoài tiền | In-the-money / Out-of-the-money | |
| Giá trị nội tại | Intrinsic value | Quyền bán châu Âu **có thể** rẻ hơn |
| Giá trị thời gian | Time value | Giảm theo `√T` |
| Giá trị thực thi sớm | Early exercise premium | Chênh lệch Mỹ − châu Âu |
| Cân bằng quyền mua–bán | Put-call parity | Chênh lệch giá, **không** phải mô hình |
| Phân phối chuẩn tích luỹ | Cumulative normal distribution | Xấp xỉ Abramowitz–Stegun |
| Độ nhạy | Greeks | Delta, Gamma, Vega, Theta, Rho |
| Biến động ngụ ý | Implied volatility | Đảo ngược công thức bằng chia đôi |
| Nụ cười / nghiêng biến động | Volatility smile / skew | Cách thị trường sửa mô hình |
| Cố định gamma | Gamma pinning | |
| Trung tính delta | Delta-neutral | Phòng vệ lại tốn phí |
| Cây nhị thức | Binomial tree | Định giá được quyền chọn Mỹ |
| Hồi quy tuyến tính | Linear regression | |
| Tương quan | Correlation | **Không đủ** cho giao dịch cặp |
| Đồng liên kết | Cointegration | Điều kiện đúng — chênh lệch kéo về |
| Tính dừng | Stationarity | |
| Kéo về trung bình | Mean reversion | |
| Nửa chu kỳ | Half-life | `ln(2)/|λ|` — vốn bị kẹt bao lâu |
| Tỉ lệ phòng vệ | Hedge ratio | |
| Điểm z | Z-score | Ngưỡng vào/ra lệnh |
| Bộ lọc Kalman | Kalman filter | Tỉ lệ phòng vệ thích ứng có nguyên tắc |
| Nhiễu quá trình / nhiễu đo | Process noise / Measurement noise | Tỉ lệ `Q/R` quyết định tốc độ thích ứng |
| Giá trị chịu rủi ro | VaR — Value at Risk | Vi phạm tính dưới cộng tính |
| Thiếu hụt kỳ vọng | Expected Shortfall / CVaR | Thước đo nhất quán — Basel III dùng |
| Thước đo rủi ro nhất quán | Coherent risk measure | |
| Quá khớp | Overfitting | Là toán học, không phải xui xẻo |
| Kiểm định tiến | Walk-forward validation | Phòng vệ mạnh nhất |
| Ngoài mẫu | Out-of-sample | |
| Danh mục hiệu quả | Efficient frontier | Nổi tiếng bất ổn |
| Co ma trận hiệp phương sai | Covariance shrinkage | Ledoit–Wolf |
| Ngang bằng rủi ro | Risk parity | Cân bằng theo rủi ro, không theo vốn |
| Đóng góp rủi ro | Risk contribution | |

---

## Ghi chú về cách dịch

Một số thuật ngữ **không nên dịch** — hãy giữ nguyên tiếng Anh vì cộng đồng Việt Nam đã dùng quen và bản dịch sẽ gây khó hiểu hơn:

`trait`, `struct`, `enum`, `closure`, `iterator`, `borrow checker`, `panic`, `crate`, `commit`, `rollback`, `cache`, `token`, `endpoint`, `serialize`, `deserialize`, `deploy`.

Ngược lại, những thuật ngữ **nên dịch** vì bản dịch làm rõ nghĩa cho người mới: *quyền sở hữu, vay mượn, thời gian sống, hàm thuần túy, bất biến, minh bạch tham chiếu, hàm toàn phần, kiểu bọc, phép ghép hàm*.

Với nhóm thuật ngữ toán học của lập trình hàm (*Functor, Monad, Semigroup, Monoid*), giáo trình dùng **song ngữ**: nêu bản dịch tiếng Việt để hiểu, nhưng luôn kèm từ tiếng Anh trong ngoặc — vì đó là từ bạn sẽ gặp trong mọi tài liệu, tên hàm và tên crate ngoài đời thực.
