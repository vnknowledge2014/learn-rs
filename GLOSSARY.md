# Bảng Thuật Ngữ Việt – Anh (Vietnamese–English Glossary)

Tài liệu này chốt cách dịch thuật ngữ được dùng **nhất quán trong toàn bộ 85 chương**. Mục đích không chỉ là tra cứu: nó còn là **chiếc cầu bắc sang tài liệu tiếng Anh**. Khi bạn đọc xong giáo trình này và mở tài liệu chính thức của Rust hay một cuốn sách quốc tế, những từ bên cột phải sẽ không còn xa lạ.

## Quy ước xử lý thuật ngữ kỹ thuật

Giáo trình dùng **ba tầng**, chọn theo mức độ mà bản dịch tiếng Việt có giúp hiểu hay không.

| Tầng | Khi nào dùng | Cách viết | Ví dụ |
|---|---|---|---|
| **1. Dịch, kèm tiếng Anh** | Bản dịch tự nó đã gợi đúng nghĩa | *tiếng Việt (English)* ở **lần đầu mỗi chương**, sau đó chỉ dùng tiếng Việt | quyền sở hữu (ownership) · vị nhóm (Monoid) · nghịch lý Bélády (Bélády's anomaly) |
| **2. Giữ tiếng Anh, chú nghĩa** | Dịch ra sẽ xa lạ hơn bản gốc, hoặc cộng đồng Việt đã dùng quen từ tiếng Anh | `English` + một câu cắt nghĩa ngay cạnh | `trait` · `closure` · `borrow checker` · `crate` · `panic` |
| **3. Đưa vào bảng tra** | Thuật ngữ hiếm gặp, hoặc chỉ xuất hiện một lần | chỉ dùng tiếng Việt trong bài, tra nghĩa ở bảng dưới | các mục ở phần 1–18 của tài liệu này |

Có viết tắt thông dụng thì ghi cả hai: *cây cú pháp trừu tượng (Abstract Syntax Tree — AST)*.

**Vì sao không dịch hết sang tiếng Việt.** Mục đích cuối là bạn đọc được tài liệu Rust thật, mà tài liệu đó bằng tiếng Anh. Một thuật ngữ chỉ tồn tại trong sách này thì không bắc được cầu sang đó — nên tầng 1 luôn kèm bản tiếng Anh ở lần đầu, và tầng 2 giữ nguyên từ gốc.

**Vì sao không giữ hết tiếng Anh.** Người mới học phải xử lý đồng thời khái niệm mới *và* ngoại ngữ thì gấp đôi tải nhận thức. Bản dịch tiếng Việt gánh phần khái niệm, từ tiếng Anh gánh phần tra cứu.

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
| Đọc rác / Đọc không lặp lại / Đọc bóng id | Dirty / Non-repeatable / Phantom read |
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
| Nhìn trộm tương lai | Look-ahead bias | Bỏ qua độ trễ là dạng compute vi nhất |
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
| Co id trận hiệp phương sai | Covariance shrinkage | Ledoit–Wolf |
| Ngang bằng rủi ro | Risk parity | Cân bằng theo rủi ro, không theo vốn |
| Đóng góp rủi ro | Risk contribution | |

---

## 18. Hệ sinh thái HFT tích hợp (Chương 85)

| Tiếng Việt | English | Ghi chú |
|---|---|---|
| Ảnh chụp thị trường | Market snapshot | Thứ DUY NHẤT chiến lược được nhìn |
| Bộ điều phối | Orchestrator | Nối phát lại → sàn → chiến lược → rủi ro → OMS |
| Hệ thống quản lý lệnh | OMS — Order Management System | |
| Lệnh đang bay | In-flight order | Đã phát, chưa tới sàn |
| Đặt chỗ phơi nhiễm | Exposure reservation | Đặt chỗ lúc **phát**, không lúc giao |
| Phơi nhiễm ba tầng | vị thế + đang treo + đang bay | Thiếu tầng nào cũng vỡ hạn mức |
| Rủi ro chân lẻ | Leg risk | Một chân qua, chân kia bị chặn |
| Bất đối xứng khớp | Fill asymmetry | AMM luôn khớp đủ, sổ lệnh thì không |
| Phòng vệ theo khối lượng đã khớp | Hedge-on-fill | Chạy chân không chắc trước |
| Chân không chắc / chân chắc chắn | Uncertain leg / certain leg | Thứ tự thực thi quyết định vị thế ròng |
| Nghịch đảo hoán đổi | Swap inverse | Cần bỏ vào bao nhiêu để nhận đúng ngần này |
| Tỉ lệ thụ động | Passive fill ratio | < 50% nghĩa là MM đang cắt qua sổ |
| Kẹp giá báo | Quote clamping | Không bao giờ cắt qua bên kia |
| Rút báo giá quá tuổi | Stale quote cancellation | Không có nó, hệ thống tự bóp cổ mình |
| Đường ưu tiên | Priority path | Huỷ lệnh đi thẳng, không xếp hàng |
| Giám sát sức khoẻ | Health monitoring | Tỉ lệ, không phải số tuyệt đối |
| Tính nhân quả | Causality | Không thấy dữ liệu tương lai |
| Bất biến hệ thống | System invariant | Thứ đáng tin, khác với lãi lỗ |
| Sản phẩm phụ của mô hình | Model artifact | Kết quả đúng cơ học mà sai kinh tế |

---

## 19. Tra cứu định danh mã nguồn (Vietnamese → English)

Từ bản 85 chương, **mọi định danh trong mã nguồn đều bằng tiếng Anh** — kể cả tên
hàm kiểm thử; phần giảng nghĩa nằm ở comment tiếng Việt ngay trên định danh đó.
Lý do: học viên đọc `OrderBook` sẽ nhận ra ngay khi mở tài liệu của một crate thật,
còn `SoLenh` thì không — nó là vốn từ chỉ tồn tại trong sách này.

Tên hàm kiểm thử được đặt lại bằng tay chứ không dịch máy, vì chúng là **câu mô tả**:
`phan_vi_bat_duoc_duoi_ma_trung_binh_bo_lo` thành `percentiles_catch_tail_that_mean_hides`.

Bảng dưới đối chiếu tên cũ với tên mới, dành cho ai đã đọc bản trước.

### Kiểu dữ liệu (struct, enum, trait, type)

| Định danh cũ (tiếng Việt) | Định danh mới (tiếng Anh) |
|---|---|
| `AnalyzeGemm` | `GemmAnalysis` |
| `AnhChupThiTruong` | `MarketSnapshot` |
| `ArrayOpenPhong` | `BucketArray` |
| `Ban` | `Sell` |
| `BanGhi` | `SellRecord` |
| `BanGhiCamBien` | `SensorRecord` |
| `BanGhiNguoiDung` | `SellRecordUser` |
| `BanGhiNhatKy` | `SellRecordLog` |
| `BanGhiPhienBan` | `SellRecordSessionSell` |
| `BanGhiTruyCap` | `SellRecordAccessCap` |
| `BanVa` | `SellAnd` |
| `BangBamPhanTan` | `HashMapPartTan` |
| `BangBaoGiaSoA` | `QuoteTableSoA` |
| `BangDinhTuyen` | `RoutingTable` |
| `BaoGiaAoS` | `QuoteAoS` |
| `BeDoiTuong` | `ObjectPool` |
| `BeRong` | `EmptyPool` |
| `BeThanhKhoan` | `Pool` |
| `BieuDoTre` | `LatencyHistogram` |
| `BoDemChungDong` | `SharedBuffer` |
| `BoDemTachDong` | `BufferSplitClose` |
| `BoDieuKhienDen` | `LedController` |
| `BoDinhTuyen` | `RouteMatcher` |
| `BoDoLuong` | `Metrics` |
| `BoGhiPhien` | `SessionRecorder` |
| `BoKhung` | `UnitFrame` |
| `BoLoc` | `Filter` |
| `BoNao` | `UnitWhich` |
| `BoNaoGia` | `UnitWhichPrice` |
| `BoNgoaiVi` | `UnitOutPos` |
| `BoPhatHienKhe` | `GapDetector` |
| `BoPhatLai` | `Replayer` |
| `BoSinh` | `Generator` |
| `BoTacNghen` | `CongestionControl` |
| `BoTachTruong` | `FieldExtractor` |
| `BoTichLuy` | `AccumulatorUnit` |
| `BoTichLuyNguyen` | `IntegerAccumulator` |
| `BoXuLy` | `UnitHandle` |
| `BufferChungClose` | `SharedBuffer` |
| `BuocChungMinh` | `ProofStep` |
| `BuocTiep` | `StepCont` |
| `CallNguEdge` | `EdgeCall` |
| `CamBien` | `Sensor` |
| `CamBienKhoi` | `SmokeSensor` |
| `CamBienNhietDo` | `TempSensor` |
| `CauHinhHeThong` | `SystemConfig` |
| `CauHinhPhat` | `LaunchConfig` |
| `CauIPC` | `IpcBridge` |
| `CauPhan` | `Leg` |
| `CauSqlAnToan` | `SafeSql` |
| `CayMerkle` | `MerkleTree` |
| `CayNhiPartSearch` | `BinarySearchTree` |
| `CayNhiPhanTimKiem` | `BinarySearchTree` |
| `Chan` | `Block` |
| `ChangDoTre` | `LatencyStage` |
| `ChenhLechHaiSan` | `CrossVenueArb` |
| `ChiDanDauRa` | `OnlyDeriveOutput` |
| `ChienLuoc` | `Strategy` |
| `ChienLuocCanBang` | `StrategyCanTable` |
| `ChienLuocPhatLai` | `StrategyReplay` |
| `ChienLuocQuyen` | `OptionStrategy` |
| `Chieu` | `Side` |
| `ChuaCauHinh` | `Unconfigured` |
| `ChucHeavy` | `HeavyStage` |
| `ChucNang` | `HeavyStage` |
| `ChungThucBaoMat` | `LostReport` |
| `ChungThucReportMat` | `LostReport` |
| `ChuoiKhoi` | `Chain` |
| `ChuongTrinhDem` | `CounterProgram` |
| `CoHoiArb` | `ArbOpportunity` |
| `CoIt` | `HasFew` |
| `CoTuChoi` | `HasReject` |
| `CongCu` | `LegacyTool` |
| `CongCuTinhToan` | `LegacyComputeTool` |
| `CongCuTraCuu` | `LookupTool` |
| `CongGianDiep` | `SpyGateway` |
| `CongLuonHong` | `AlwaysFailGateway` |
| `CongOk` | `OkGateway` |
| `CongOldCompute` | `LegacyComputeTool` |
| `CongRuiRo` | `RiskGate` |
| `CongThanhToan` | `PaymentGateway` |
| `CongThat` | `RealGateway` |
| `CongViec` | `WorkPort` |
| `CongWork` | `WorkPort` |
| `CuaSo` | `Window` |
| `CuaSoThongKe` | `StatsWindow` |
| `DaAuth` | `Authenticated` |
| `DaCheckRisk` | `RiskChecked` |
| `DaGiao` | `Delivered` |
| `DaGui` | `Sent` |
| `DaIntoToan` | `MathDone` |
| `DaKhop` | `Traded` |
| `DaKiemTraRuiRo` | `RiskChecked` |
| `DaNgatKhanCap` | `KillSwitchOn` |
| `DaSend` | `Sent` |
| `DaThanhToan` | `MathDone` |
| `DaXacThuc` | `Authenticated` |
| `DaiBollinger` | `BollingerBands` |
| `DanXuat` | `DeriveExport` |
| `DanhSachLienKet` | `ListLienLink` |
| `DatCoPhongVe` | `PlaceHedged` |
| `DatLenh` | `Place` |
| `DauRa` | `Output` |
| `DauTien` | `FirstTien` |
| `DauVao` | `Input` |
| `DauVaoBangKhong` | `ZeroInput` |
| `DemCoDem` | `CountHasCount` |
| `DemNguoc` | `CountInverse` |
| `DemVong` | `CountRound` |
| `DenGiaoThong` | `TrafficLight` |
| `DiaChi` | `Address` |
| `DoThi` | `Graph` |
| `DoThiCho` | `WaitForGraph` |
| `DoThiTriThuc` | `RealValueGraph` |
| `DoThiValueThuc` | `RealValueGraph` |
| `DoThiWait` | `WaitForGraph` |
| `DoanKiemDinh` | `TestSegment` |
| `DoanTest` | `TestSegment` |
| `DoiDonNguyen` | `MonoidHomomorphism` |
| `DonHang` | `DonQueue` |
| `DonHangDto` | `OrderDto` |
| `DonNguyen` | `Monoid` |
| `DongHang` | `CloseQueue` |
| `DongHangDto` | `OrderLineDto` |
| `DongHoAo` | `VirtualClock` |
| `DuDoanNhanh` | `BranchPredictor` |
| `DungNgoai` | `UseOut` |
| `DuongOngPhanCung` | `HwPipeline` |
| `ErrorIntoToan` | `MathError` |
| `FillCuaTa` | `OurFill` |
| `GapDuoc` | `Traversable` |
| `Gia` | `Price` |
| `GiaNgoaiBien` | `PriceOutOfBand` |
| `GiaTri` | `Value` |
| `GiaTriAbi` | `AbiValue` |
| `GiaTriLenhQuaLon` | `OrderValueTooLarge` |
| `GiaTriMacd` | `MacdValue` |
| `GiaoCatTrungBinh` | `MeanCross` |
| `GiaoCutMean` | `MeanCross` |
| `GiaoDich` | `Trade` |
| `GiaoDich1559` | `Tx1559` |
| `GiaoDichCho` | `TradeWait` |
| `GioHang` | `Cart` |
| `GoiLenh` | `OrderPacket` |
| `GoiNguCanh` | `EdgeCall` |
| `GoiTinTruong` | `FieldPacket` |
| `HamTu` | `Functor` |
| `HamTuHaiNgoi` | `Bifunctor` |
| `HanMuc` | `Limit` |
| `HanMucRuiRo` | `LimitRisk` |
| `HangDoiDonHang` | `QueueDonQueue` |
| `HangDoiGioiHan` | `QueueLimit` |
| `HanhDong` | `ExecClose` |
| `HanhVi` | `ExecPos` |
| `HeSinhThai` | `Ecosystem` |
| `HienTai` | `Current` |
| `HoSoHopLe` | `ValidProxy` |
| `HoSoTho` | `RawProfile` |
| `HoaDon` | `Invoice` |
| `HoanDoiTrenBe` | `PoolSwap` |
| `HopBao` | `HopReport` |
| `HuyLenh` | `CancelOrder` |
| `IntoToan` | `MathOp` |
| `ItKetNoi` | `FewConnect` |
| `KetQuaCong` | `GateResult` |
| `KetQuaCongCu` | `LegacyToolResult` |
| `KetQuaHoiQuy` | `ResultRegression` |
| `KetQuaKiemDinh` | `ResultTest` |
| `KetQuaKiemDinhTien` | `ResultWalkForward` |
| `KetQuaLanTruyen` | `ResultPropagate` |
| `KetQuaLenh` | `ResultOrder` |
| `KetQuaNhom` | `ResultGroup` |
| `KetQuaOng` | `PipelineResult` |
| `KetQuaPhatLai` | `ResultReplay` |
| `KetQuaThayTrang` | `StateChange` |
| `KetQuaTruyen` | `TransferResult` |
| `KetQuaVong` | `ResultRound` |
| `KetQuaVongLap` | `ResultRoundLoop` |
| `KheBanGhi` | `RecordSlot` |
| `KheSellRecord` | `RecordSlot` |
| `Kho` | `Store` |
| `KhoLuuTruNhiPhan` | `BinaryPageStore` |
| `Khoi` | `Block` |
| `KhoiLuongQuaLon` | `QuantityTooLarge` |
| `KhongDatToiThieu` | `BelowMinOut` |
| `KhongLamGi` | `NoOp` |
| `KhopCuaTa` | `OurFill` |
| `KhopLenh` | `Fill` |
| `KhungGhi` | `FrameRecord` |
| `KiemToanBaoMat` | `AuditLostReport` |
| `KiemToanReportMat` | `AuditLostReport` |
| `KiemTraTaiKhoan` | `CheckAccount` |
| `KyQuy` | `Escrow` |
| `LaPhieu` | `Ballot` |
| `Lenh` | `Order` |
| `LenhBackend` | `BackendCommand` |
| `LenhCuaTa` | `OurOrder` |
| `LenhDangBay` | `InFlightOrder` |
| `LenhL3` | `L3Order` |
| `LenhLuuTep` | `OrderSaveFile` |
| `LenhThongTinHeThong` | `SystemInfoRequest` |
| `LenhToken` | `TokenMsg` |
| `LoaiCauPhan` | `KindLeg` |
| `LoaiQuyen` | `OptionKind` |
| `LoaiSuKien` | `EventKind` |
| `LocDuoc` | `FilterCan` |
| `LocKalman` | `KalmanFilter` |
| `LoiDoc` | `ErrorRead` |
| `LoiGiaoDich` | `ErrorTrade` |
| `LoiGio` | `CartError` |
| `LoiHoanDoi` | `SwapError` |
| `LoiHopDong` | `ContractError` |
| `LoiKhoi` | `ErrorBlock` |
| `LoiMatKhau` | `ErrorPassword` |
| `LoiMien` | `DomainError` |
| `LoiPhanTich` | `ErrorAnalyze` |
| `LoiRuiRo` | `ErrorRisk` |
| `LoiSolana` | `SolanaError` |
| `LoiThanhToan` | `MathError` |
| `LoiTruyCap` | `ErrorAccessCap` |
| `LoiUrl` | `UrlError` |
| `LonNhat` | `Max` |
| `LuaChonThayThe` | `Alternative` |
| `LuonMua` | `AlwaysBuy` |
| `LyDoDung` | `StopReason` |
| `MaLenh` | `OrderId` |
| `MachDien` | `Circuit` |
| `MachElec` | `Circuit` |
| `MachRuiRo` | `RiskCircuit` |
| `MangMoPhong` | `BucketArray` |
| `MatHang` | `MatQueue` |
| `MauHinh` | `Pattern` |
| `MauNguCanh` | `EdgePattern` |
| `MauNguEdge` | `EdgePattern` |
| `MayChu` | `Server` |
| `MayChuDns` | `DnsServer` |
| `MoHinh` | `OpenImage` |
| `MoHinhDoTre` | `LatencyModel` |
| `MoPhongCache` | `CacheSim` |
| `MoRong` | `Extend` |
| `MoTaChiTiet` | `DetailedDescription` |
| `MoiTruong` | `NewField` |
| `Mua` | `Buy` |
| `MucChiemDung` | `Occupancy` |
| `MucGia` | `PriceLevel` |
| `MucGiaPC` | `HwPriceLevel` |
| `Nano` | `Nanos` |
| `Nen` | `Candle` |
| `NewTruong` | `NewField` |
| `NganSachDoTre` | `LatencyBudget` |
| `NguoiDung` | `User` |
| `Nhanh` | `Fast` |
| `Nhap` | `Import` |
| `NhipKhung` | `FrameClock` |
| `NhoNhat` | `Min` |
| `Nhom` | `Group` |
| `Noi` | `Chain` |
| `NoiThucThi` | `ExecutionUnit` |
| `NuaGroup` | `Semigroup` |
| `NuaNhom` | `Semigroup` |
| `NutAo` | `VirtualNode` |
| `OldCong` | `LegacyTool` |
| `OldResultCong` | `LegacyToolResult` |
| `OpenHinh` | `OpenImage` |
| `OrderL3` | `L3Order` |
| `OrderThongInfoHeThong` | `SystemInfoRequest` |
| `OwnedDoRisk` | `RiskOwned` |
| `PacketTruong` | `FieldPacket` |
| `PhaTacNghen` | `CongestionPhase` |
| `Phai` | `Must` |
| `Phan` | `Part` |
| `PhanDauKhoi` | `BlockHeader` |
| `PhanHoi` | `Response` |
| `PhanTichGemm` | `GemmAnalysis` |
| `PhanTichGop` | `CoalescingAnalysis` |
| `PhanTichIlp` | `IlpAnalysis` |
| `PhanTichNganHang` | `BankAnalysis` |
| `PhanTichPhanKy` | `DivergenceAnalysis` |
| `PhanTichSimd` | `SimdAnalysis` |
| `PhanTichSongSong` | `AnalyzeParallel` |
| `PhienDaGhi` | `RecordedSession` |
| `PhuongThuc` | `Method` |
| `PositionGioi` | `BoundedPos` |
| `ProxyNumHopLe` | `ValidProxy` |
| `ResultCong` | `GateResult` |
| `ResultOng` | `PipelineResult` |
| `ResultThayState` | `StateChange` |
| `ResultTruyen` | `TransferResult` |
| `RoundHashNhatQuan` | `ConsistentHashRing` |
| `San` | `Venue` |
| `SanChuoiKhoi` | `ChainVenue` |
| `SanTruyenThong` | `LitVenue` |
| `SoLenh` | `OrderBook` |
| `SoLenhL2` | `L2Book` |
| `SoLenhL3` | `L3Book` |
| `SoLenhPhanCung` | `OrderBookHardware` |
| `SoLuong` | `Quantity` |
| `SoRutGon` | `ReducedBook` |
| `SoTien` | `Money` |
| `StoreSaveTruNhiPart` | `BinaryPageStore` |
| `SuKien` | `Event` |
| `SuKienPhien` | `SessionEvent` |
| `SuKienTcp` | `TcpEvent` |
| `SuKienThiTruong` | `EventMarket` |
| `TaiKhoan` | `Account` |
| `TaiKhoanNganHang` | `AccountBank` |
| `TangBoNho` | `UpMemory` |
| `TangOng` | `PipelineStage` |
| `TaoLapCoKiemSoat` | `ManagedMaker` |
| `TaoLapDonGian` | `NaiveMaker` |
| `ThamSoQuyen` | `OptionParams` |
| `ThanhGhiDich` | `IntoRecordDich` |
| `ThanhGhiGia` | `IntoRecordPrice` |
| `ThanhToan` | `MathOp` |
| `TheGioi` | `BoundedPos` |
| `TheVatLy` | `PhysicsBody` |
| `ThemLenh` | `AddOrder` |
| `ThietBiMang` | `NetworkDevice` |
| `ThoiGianThuc` | `RealTime` |
| `ThongBaoNguyHiem` | `ThongReportUnsafe` |
| `ThongDiep` | `ThongMessage` |
| `ThongKeCache` | `CacheStats` |
| `ThongKeDanhMuc` | `PortfolioStats` |
| `ThuFrom` | `Foldable` |
| `ThuTu` | `Foldable` |
| `ThucPosition` | `RealPosition` |
| `ThucThe` | `RealPosition` |
| `ThuocDoRuiRo` | `RiskOwned` |
| `Tich` | `Product` |
| `TienTrinh` | `Process` |
| `TinHieu` | `Signal` |
| `TinHieuCap` | `SignalCap` |
| `ToGiaoThong` | `TrafficLight` |
| `ToaDoGps` | `GpsCoord` |
| `TocDoPhat` | `ReplaySpeed` |
| `Trai` | `Left` |
| `TrangThai` | `State` |
| `TrangThaiDem` | `StateCount` |
| `TrangThaiDonHang` | `StateDonQueue` |
| `TrangThaiKyQuy` | `StateEscrow` |
| `TrangThaiTcp` | `TcpState` |
| `TrangThaiTienTrinh` | `StateProcess` |
| `TrichXuat` | `Extract` |
| `TruongDuLieuAST` | `AstDataField` |
| `TruongGoiTin` | `PacketField` |
| `TruongPacket` | `PacketField` |
| `TruyenThong` | `Lit` |
| `TuChoi` | `RejectReason` |
| `TuongFrom` | `Wall` |
| `TuongTu` | `Wall` |
| `Tuyen` | `Route` |
| `UnitPeakTuyen` | `RouteMatcher` |
| `UnitTichAccum` | `AccumulatorUnit` |
| `UpOng` | `PipelineStage` |
| `ValueAbi` | `AbiValue` |
| `ViNhom` | `PosGroup` |
| `ViThe` | `Position` |
| `ViTu` | `PosFrom` |
| `VoHan` | `Unbounded` |
| `VongBamNhatQuan` | `ConsistentHashRing` |
| `VongDisruptor` | `DisruptorRing` |
| `VuotHanMucLo` | `LossLimit` |
| `VuotHanMucViThe` | `PositionLimit` |
| `VuotTanSuat` | `RateLimit` |
| `WindowThongKe` | `StatsWindow` |
| `XacThuc` | `Auth` |
| `XoToken` | `TokenBucket` |
| `XoayVong` | `RoundRobin` |
| `XoayVongTrongSo` | `WeightedRoundRobin` |
| `YDinh` | `Intent` |
| `YeuCau` | `Request` |

### Hàm và trường thường gặp

| Định danh cũ (tiếng Việt) | Định danh mới (tiếng Anh) |
|---|---|
| `add_thuc_position` | `add_entity` |
| `all_doan` | `segments` |
| `amount_lang_phi_new_block` | `wasted_per_block` |
| `anh_remote` | `mapping` |
| `anh_xa` | `mapping` |
| `ap_dung` | `apply` |
| `ap_dung_khop` | `apply_fill` |
| `ap_suat_bar` | `pressure_bar` |
| `array_tinh` | `computed_array` |
| `balance_doan_sai` | `wrong_guess_balance` |
| `bam64` | `hash64` |
| `bam_khoi_truoc` | `prev_hash_block` |
| `ban_ghi` | `sell_record` |
| `ban_tot_nhat` | `best_ask` |
| `bat_dau` | `start` |
| `bay_ban` | `in_flight_ask` |
| `bay_gio` | `now` |
| `bay_mua` | `in_flight_bid` |
| `be_mau` | `sample_pool` |
| `ben_mua` | `side_buy` |
| `bfs_khoang_cach_ngan_nhat` | `bfs_shortest_distance` |
| `bi_bo` | `is_unit` |
| `bi_bo_buoc` | `is_unit_step` |
| `bi_chan` | `is_block` |
| `bi_chan_boi` | `blocked_by` |
| `bi_chan_boi_bao_ve` | `blocked_by_guard` |
| `bi_cheo` | `is_crossed` |
| `bien_dong_ngu_y` | `implied_volatility` |
| `bieu_do` | `histogram` |
| `bo_dem` | `buffer` |
| `bo_loc` | `filter` |
| `bo_tai_khoan` | `account_set` |
| `bong_duoi` | `lower_wick` |
| `bong_tren` | `upper_wick` |
| `buoc_co_dinh` | `step_has_peak` |
| `buoc_euler_nua_an` | `semi_implicit_euler_step` |
| `buoc_euler_tuong_minh` | `explicit_euler_step` |
| `buoc_ns` | `step_nanos` |
| `buy_good_nhat` | `best_bid` |
| `byte_da_chuyen` | `bytes_transferred` |
| `byte_moi_o` | `bytes_per_cell` |
| `cac_doan` | `segments` |
| `cac_khoi` | `all_block` |
| `cac_khop` | `all_fill` |
| `cac_mau` | `all_mau` |
| `cac_su_kien` | `all_event` |
| `cac_tang` | `all_up` |
| `can_duoi_chau_au` | `european_lower_bound` |
| `cap_nhat` | `update` |
| `cap_nhat_tien_trinh` | `update_process` |
| `cat_khoang_trang` | `cut_range_state` |
| `cau_phan` | `leg` |
| `cay_xor` | `xor_tree` |
| `chay_kiem_dinh` | `run_test` |
| `chay_phien` | `run_session` |
| `chay_vong_lap` | `run_round_loop` |
| `chenh_lech` | `spread` |
| `chi_phi_ban_dau` | `first_only_phi_sell` |
| `chiet_khau` | `discount` |
| `chieu` | `side` |
| `chieu_cao` | `height` |
| `chieu_cao_dinh` | `height_peak` |
| `chieu_chu_dong` | `side_aggressive` |
| `cho_bao_lau` | `wall_delay` |
| `cho_phep` | `wait_op` |
| `cho_trung_binh` | `wait_mean` |
| `chon_tuyen` | `match_route` |
| `chu_account` | `account_owner` |
| `chu_dong` | `aggressive` |
| `chu_ky` | `period` |
| `chu_ky_ham` | `selector` |
| `chu_ky_khoi_dau` | `first_period_block` |
| `chu_ky_phi` | `period_phi` |
| `chu_ky_sang_ns` | `cycles_to_ns` |
| `chu_ky_uoc_tinh` | `estimated_cycles` |
| `chu_so_huu` | `owner` |
| `chu_tai_khoan` | `account_owner` |
| `chua_key` | `contains_key` |
| `chua_khoa` | `contains_key` |
| `chuan_hoa` | `normalize` |
| `chung_minh` | `prove` |
| `chuoi` | `series` |
| `chuoi_atr` | `atr_series` |
| `chuoi_co_tien` | `utxo_has_funds` |
| `chuoi_ema` | `ema_series` |
| `chuoi_macd` | `macd_series` |
| `chuoi_rsi` | `rsi_series` |
| `chuoi_sma` | `sma_series` |
| `chuyen` | `transfer` |
| `ck_du_tru_x` | `chain_reserve_x` |
| `ck_du_tru_y` | `chain_reserve_y` |
| `ck_gia` | `chain_price` |
| `close_call_ngu_edge` | `close_edge_call` |
| `close_nhat` | `closest` |
| `co_be_tac` | `has_deadlock` |
| `co_co_hoi` | `has_has_hoi` |
| `co_lenh` | `has_order` |
| `co_so` | `has_num` |
| `co_theo_bien_dong` | `has_theo_volatility` |
| `co_xung_dot` | `has_conflict` |
| `con_lai` | `remaining` |
| `con_tro` | `pointer` |
| `con_tro_day` | `tail_pointer` |
| `cong_and` | `and_gate` |
| `cong_cu` | `legacy_tool` |
| `cong_don` | `accumulate` |
| `cong_hoac` | `or_gate` |
| `cong_khong` | `nor_gate` |
| `cong_mau` | `sample_gate` |
| `cong_nhin_truoc_8bit` | `lookahead_adder_8bit` |
| `cong_no` | `nor_gate` |
| `cong_noi_tiep_8bit` | `ripple_adder_8bit` |
| `cong_tac_all` | `switch_all` |
| `cong_tac_tat` | `switch_all` |
| `cong_va` | `and_gate` |
| `cong_viec` | `job` |
| `cong_work` | `job` |
| `cong_xor` | `xor_gate` |
| `count_has_nhanh` | `branch_taken_count` |
| `cua_so` | `window` |
| `cua_so_lenh` | `window_order` |
| `cuong_do_tinh_toan` | `arithmetic_intensity` |
| `current_ap_suat` | `current_pressure` |
| `cut_ngan` | `truncate` |
| `da_fill` | `filled` |
| `da_goi_voi` | `called_with` |
| `da_into_toan` | `is_paid` |
| `da_khop` | `filled` |
| `da_ngat` | `kill_switch_on` |
| `da_thanh_toan` | `is_paid` |
| `da_thay` | `seen` |
| `da_tieu` | `da_spend` |
| `da_vao` | `da_in` |
| `dan_xuat_pda` | `derive_pda` |
| `dang_bay` | `in_flight` |
| `dang_khoi_phuc` | `dang_recovery` |
| `dang_ky` | `register` |
| `dang_ky_ngan_mach` | `short_circuit_register` |
| `dang_ky_tich_accum` | `accumulator_register` |
| `dang_ky_tich_luy` | `accumulator_register` |
| `dang_mo` | `is_open` |
| `danh_gia` | `evaluate` |
| `danh_sach` | `list` |
| `danh_sach_cho_phep` | `list_wait_op` |
| `danh_sach_dai` | `list_long` |
| `danh_sach_ke` | `adjacency_list` |
| `danh_sach_phien_ban` | `list_session_sell` |
| `danh_sach_ten` | `list_name` |
| `dao_dong_ns` | `jitter_ns` |
| `dao_khoi_moi` | `new_mine_block` |
| `dao_khoi_tren` | `mine_block_above` |
| `dao_nguoc_tai_cho` | `reverse_inverse_tai_wait` |
| `dat_lai` | `set_lai` |
| `dat_lenh_cua_ta` | `place_our_order` |
| `dat_thue_rieng` | `set_custom_tax` |
| `data_doan` | `segment_data` |
| `data_kien` | `amount_in` |
| `dau_ra` | `output` |
| `dau_vao` | `input` |
| `day_con_chung_dai_nhat` | `longest_common_subsequence` |
| `day_vao` | `push` |
| `dem_co_nhanh` | `branch_taken_count` |
| `dem_lai` | `count_lai` |
| `dem_trong_cua_so` | `window_count` |
| `den_luc` | `arrives_at` |
| `dia_chi` | `address` |
| `dia_chi_bo_dem` | `address_buffer` |
| `dia_chi_hien_tai` | `current_address` |
| `dia_chi_mang` | `address_array` |
| `dia_chi_truoc` | `prev_address` |
| `dia_chi_tu_hex` | `address_from_hex` |
| `dien_hinh` | `typical` |
| `dinh_an` | `peak_hidden` |
| `dinh_so` | `peak_num` |
| `do_dai` | `length` |
| `do_dai_ten` | `do_long_name` |
| `do_kho` | `difficulty` |
| `do_lech_chuan` | `stddev` |
| `do_lech_tick` | `tick_offset` |
| `do_long` | `length` |
| `do_luong` | `metrics` |
| `do_next_cong` | `gate_depth` |
| `do_next_xor_tree` | `xor_tree_depth` |
| `do_risk` | `risk_level` |
| `do_rui_ro` | `risk_level` |
| `do_sau_cong` | `gate_depth` |
| `do_tre` | `latency` |
| `do_tre_chu_ky` | `latency_period` |
| `do_tre_ns` | `latency_nanos` |
| `doc_toan_cuc` | `read_global` |
| `doi_tien` | `swap_tien` |
| `doi_ung` | `swap_resp` |
| `doi_ung_la_ban` | `swap_resp_is_sell` |
| `don_gia` | `don_price` |
| `don_nhap` | `don_import` |
| `don_vi` | `don_pos` |
| `dong_goi_ngu_canh` | `close_edge_call` |
| `dong_ho` | `clock` |
| `dong_nhat` | `closest` |
| `du_doan` | `segment_data` |
| `du_lieu` | `data` |
| `du_lieu_sai` | `data_sai` |
| `du_lieu_tho` | `raw_data` |
| `du_tru_x` | `reserve_x` |
| `du_tru_y` | `reserve_y` |
| `dung_luong` | `capacity` |
| `duoc_ghi` | `is_writable` |
| `duong_dan` | `path` |
| `duong_dan_an_toan` | `path_safe` |
| `duong_thoi_gian` | `timeline` |
| `duong_time_time` | `timeline` |
| `duong_toi_han` | `critical_path` |
| `duong_von` | `equity_curve` |
| `duyet_theo_hang` | `row_major_scan` |
| `engine_phuc_hoi` | `recovered_engine` |
| `env_mau` | `sample_env` |
| `filter_anh_remote` | `filter_map` |
| `first_write_hoa_chu` | `capitalize_first` |
| `from_thuc` | `from_real` |
| `gan_nhat` | `nearest` |
| `gd_mau` | `trade_mau` |
| `gemm_ngay_tho` | `gemm_naive` |
| `gemm_theo_lat` | `tiled_gemm` |
| `gen_error_suat` | `gen_returns` |
| `get_num_khe` | `slot_count` |
| `ghep_voi` | `compose_with` |
| `ghi_nhan` | `record` |
| `ghi_nhan_khop` | `record_recv_fill` |
| `ghi_truong` | `record_field` |
| `ghi_tu_choi` | `record_reject` |
| `gia_ban` | `price_sell` |
| `gia_ban_tot_nhat` | `best_ask` |
| `gia_co_so` | `spot` |
| `gia_cuoi` | `last_price` |
| `gia_dex_sau` | `dex_price_after` |
| `gia_dong` | `price_close` |
| `gia_giua` | `mid` |
| `gia_hien` | `price_show` |
| `gia_khong` | `price_no` |
| `gia_mua` | `price_buy` |
| `gia_mua_tot_nhat` | `best_bid` |
| `gia_tham_chieu` | `reference_price` |
| `gia_thuc` | `exec_price` |
| `gia_thuc_hien` | `strike` |
| `gia_thuc_hien_chiet_khau` | `discounted_strike` |
| `gia_tri` | `value` |
| `gia_tri_chiu_rui_ro` | `value_at_risk` |
| `gia_tri_cuoi` | `last_value` |
| `gia_tri_lenh_toi_da` | `max_order_value` |
| `gia_tri_mau` | `value_mau` |
| `gia_tri_noi_tai` | `intrinsic_value` |
| `gia_tri_rong` | `value_empty` |
| `gia_tri_tam` | `value_temp` |
| `gia_tri_thoi_gian` | `value_time_time` |
| `gia_truoc` | `prev_price` |
| `gia_vi_mo` | `price_pos_open` |
| `gia_von` | `cost_basis` |
| `gia_x` | `price_x` |
| `giai_ngan` | `release` |
| `giam_gia` | `discount` |
| `giam_sat_thong_so` | `monitor_metrics` |
| `giam_tb` | `down_avg` |
| `giao_dich` | `trade` |
| `giao_each` | `intersect` |
| `giao_nhau` | `intersect` |
| `gioi_han_gas` | `gas_limit` |
| `goc_merkle` | `merkle_root` |
| `goc_merkle_tinh_lai` | `recompute_merkle_root` |
| `goi_hop_le` | `call_hop_le` |
| `good_nhat` | `best` |
| `gop_tat_ca` | `coalesce_all_all` |
| `hai_ma` | `two_id` |
| `han_chot` | `deadline` |
| `han_muc` | `limit` |
| `han_muc_ton_kho` | `inventory_limit` |
| `handle_has_ong` | `handle_with_pipeline` |
| `hang_doi` | `queue` |
| `hang_thi_truong` | `market_queues` |
| `has_be_tac` | `has_deadlock` |
| `he_moi` | `new_ecosystem` |
| `he_so_keo_ve` | `reversion_coef` |
| `he_so_noi_suy` | `lerp_factor` |
| `hien_tai` | `current` |
| `hien_thi` | `display` |
| `hiep_phuong_sai` | `covariance` |
| `hieu_suat` | `efficiency` |
| `ho_so` | `profile` |
| `ho_ten` | `full_name` |
| `hoa_don` | `invoice` |
| `hoan_doi` | `swap` |
| `hoan_doi_x_lay_y` | `swap_x_for_y` |
| `hoan_pos` | `swap_pos` |
| `hoan_tien` | `refund` |
| `hoan_vi` | `swap_pos` |
| `hoi_quy` | `regression` |
| `hop_le` | `is_valid` |
| `id_ke_tiep` | `next_id` |
| `in_thong_tin` | `print_info` |
| `into_thuc` | `into_real` |
| `into_toan` | `payment` |
| `is_thuc_thi` | `is_executable` |
| `ke_cont` | `next` |
| `ke_tan_cong_lai` | `ke_attack_lai` |
| `ke_tiep` | `next` |
| `ket_noi_hien_tai` | `current_connect` |
| `ket_qua_dem` | `result_count` |
| `ket_thuc` | `end` |
| `khach` | `customer` |
| `khe_dang_cho` | `pending_gap` |
| `khi_co_su_kien` | `on_event` |
| `khoang_cach` | `distance` |
| `khoi_luong` | `quantity` |
| `khoi_luong_dung_truoc` | `queue_ahead` |
| `khoi_luong_khop` | `filled_qty` |
| `khoi_luong_tai` | `qty_at` |
| `khoi_luong_toi_uu` | `quantity_toi_uu` |
| `khoi_luong_truoc` | `prev_quantity` |
| `khoi_luong_truoc_mat` | `quantity_prev_mat` |
| `khoi_tao` | `block_make` |
| `khong` | `no` |
| `khong_do_tre` | `no_latency` |
| `khu_hoi_ns` | `round_trip_ns` |
| `khung` | `frame` |
| `khung_moi` | `new_frame` |
| `kich_ban` | `size_sell` |
| `kich_thuoc` | `size` |
| `kich_thuoc_o` | `size_cell` |
| `kiem_chung` | `verify` |
| `kiem_chung_don_vi` | `verify_don_pos` |
| `kiem_chung_ket_hop` | `verify_link_hop` |
| `kiem_dinh_dong_lien_ket` | `cointegration_test` |
| `kiem_dinh_tien` | `walk_forward` |
| `kiem_thu` | `tests` |
| `kiem_tra` | `check` |
| `kiem_tra_do_manh` | `check_do_strong` |
| `kiem_tra_hop_le` | `check_hop_le` |
| `kiem_tra_ngoac_hop_le` | `is_balanced_brackets` |
| `kiem_tra_ten` | `check_name` |
| `kiem_tra_tu_cam` | `check_banned_words` |
| `kiem_tra_url_an_toan` | `is_safe_url` |
| `kiem_traverse` | `validator` |
| `kl_ban` | `qty_sell` |
| `kl_mua` | `qty_buy` |
| `ky_round` | `expectation` |
| `ky_vong` | `expectation` |
| `la_chan` | `is_block` |
| `la_ky` | `is_signer` |
| `la_phai` | `is_must` |
| `la_thuc_thi` | `is_executable` |
| `lai_khi` | `lai_when` |
| `lai_lo` | `pnl` |
| `lai_lo_da_chot` | `realized_pnl` |
| `lai_suat` | `rate` |
| `lai_uoc_tinh` | `estimated_return` |
| `lan_truyen_gossip` | `gossip_propagate` |
| `lang_gieng` | `neighbors` |
| `lanh_manh` | `is_healthy` |
| `lap_khe` | `slot_loop` |
| `lay_con_tro_day` | `tail_pointer` |
| `lay_ra` | `take` |
| `lay_so_khe` | `slot_count` |
| `lenh_cho` | `order_wait` |
| `lenh_cua_ta` | `our_orders` |
| `lenh_da_gui` | `order_sent` |
| `lenh_ra_ns` | `outbound_ns` |
| `lenh_thi_truong` | `market_orders` |
| `lenh_thu_dong` | `order_passive` |
| `lich_su` | `history` |
| `list_ke` | `adjacency_list` |
| `lo_trong_ngay_toi_da` | `max_daily_loss` |
| `loc_anh_xa` | `filter_map` |
| `lon_nhat` | `max` |
| `loop_khe` | `slot_loop` |
| `lru_danh_sach` | `lru_list` |
| `luong` | `amount` |
| `luong_lang_phi_moi_khoi` | `wasted_per_block` |
| `luong_moi_khoi` | `amount_new_block` |
| `luy_ke` | `accum_ke` |
| `luy_thua_mod` | `mod_pow` |
| `ly_do_dung` | `stop_reason` |
| `ma_chuoi` | `chain_id` |
| `ma_ck` | `id_chain` |
| `ma_cu` | `old_id` |
| `ma_don` | `order_code` |
| `ma_giao_dich` | `id_trade` |
| `ma_hoa` | `encode` |
| `ma_hoa_abi` | `abi_encode` |
| `ma_ke` | `id_ke` |
| `ma_ke_tiep` | `next_id` |
| `ma_lenh` | `order_id` |
| `ma_mau` | `color_code` |
| `ma_trang_thai` | `id_state` |
| `mang_xa_hoi` | `array_remote_hoi` |
| `mat_can_bang` | `imbalance` |
| `mat_do_chuan` | `mat_do_standard` |
| `mat_hang` | `mat_queue` |
| `max_lo_in_day` | `max_daily_loss` |
| `mean_truot` | `moving_average` |
| `merkle_root_tinh_lai` | `recompute_merkle_root` |
| `mo_phong_kep` | `simulate_sandwich` |
| `mo_ta` | `description` |
| `moi_n_su_kien` | `every_n_events` |
| `moi_truong` | `environment` |
| `mua_tot_nhat` | `best_bid` |
| `muc_sut_giam` | `level_drawdown` |
| `muc_tieu` | `level_spend` |
| `muc_xung_dot` | `level_conflict` |
| `n_chuan` | `norm_cdf` |
| `nam_tren_duong_nong` | `on_hot_path` |
| `name_khach` | `customer_name` |
| `name_truong` | `field_name` |
| `near_nhat` | `nearest` |
| `nen_don` | `simple_candle` |
| `new_pos_value_khe` | `new_slot_index` |
| `ngan_xep` | `stack` |
| `ngoai` | `out` |
| `nguoc` | `inverse` |
| `nguoi_ban` | `seller` |
| `nguoi_buy` | `buyer` |
| `nguoi_gui` | `sender` |
| `nguoi_mua` | `buyer` |
| `nguoi_nhan` | `recipient` |
| `nguoi_recv` | `recipient` |
| `nguoi_sell` | `seller` |
| `nguong` | `threshold` |
| `nguong_ngon_tay_beo` | `fat_finger_threshold` |
| `nguong_quorum` | `quorum_threshold` |
| `nguong_vao` | `threshold_in` |
| `nhan_dien` | `recv_elec` |
| `nhan_khi_bi_kep` | `receive_when_sandwiched` |
| `nhan_neu_khong_bi_kep` | `receive_if_not_sandwiched` |
| `nhat_ky` | `order_log` |
| `nhiet_do_c` | `temp_c` |
| `nhieu` | `many` |
| `nhieu_tat_dinh` | `deterministic_noise` |
| `nho_hon_hoac_bang` | `less_or_equal` |
| `nho_nhat` | `min` |
| `noi_dung` | `content` |
| `noi_use` | `content` |
| `nua_chu_ky` | `half_life` |
| `num_duong` | `positive_count` |
| `num_hieu` | `serial` |
| `num_khe` | `slot_count` |
| `num_nhanh` | `branch_count` |
| `num_op_cong` | `add_op_count` |
| `num_truot` | `slip_count` |
| `old_cong` | `legacy_tool` |
| `on_dinh` | `is_stable` |
| `only_num_xor` | `leading_bit_diff` |
| `order_da_send` | `orders_sent` |
| `owned_tinh` | `attribute` |
| `part_cong` | `partial_sum` |
| `peek_dau` | `peek_front` |
| `phan_cong` | `partial_sum` |
| `phan_dau` | `header` |
| `phan_du` | `part_data` |
| `phan_giai` | `part_solve` |
| `phan_thuong` | `part_normal` |
| `phan_tich` | `analyze` |
| `phan_tich_gop` | `coalescing_analysis` |
| `phan_tich_ngan_hang` | `bank_analysis` |
| `phan_tich_phan_ky` | `divergence_analysis` |
| `phan_tich_simd` | `simd_analysis` |
| `phan_tich_tong_nhieu_bien` | `analyze_total_many_bien` |
| `phan_tram` | `percent` |
| `phan_vi` | `percentile` |
| `phat_hien_bat_thuong` | `emit_normal` |
| `phat_luc` | `sent_at` |
| `phat_show_enable_normal` | `emit_normal` |
| `phi_phan_van` | `fee_bps` |
| `phi_quyen` | `premium` |
| `phi_thuc_te` | `effective_fee` |
| `phi_toi_da` | `max_fee` |
| `phi_uu_tien` | `priority_fee` |
| `phi_uu_tien_toi_da` | `max_priority_fee` |
| `phien` | `session` |
| `phien_ban` | `session_sell` |
| `phong_ve_tren` | `hedge_on` |
| `phuong_sai` | `variance` |
| `phuong_sai_uoc_luong` | `estimated_variance` |
| `phuong_thuc` | `method` |
| `pick_tuyen` | `match_route` |
| `point_num_goc` | `base_score` |
| `pop_dau` | `pop_front` |
| `pos_value_khe` | `slot_pos` |
| `push_dau` | `push_front` |
| `quy_open` | `scale` |
| `quyet_dinh` | `decide` |
| `r_binh_phuong` | `r_squared` |
| `ra_ns` | `out_nanos` |
| `ra_truoc` | `prev_out` |
| `record_truong` | `record_field` |
| `rut_lien_mach` | `drain` |
| `san_ck` | `venue_chain` |
| `san_nhan_toi_thieu` | `min_venue_recv` |
| `san_tt` | `venue_lit` |
| `sat_thuong_cham` | `contact_damage` |
| `sau_giam_gia` | `after_discount` |
| `sau_lenh` | `next_order` |
| `sau_phi` | `next_phi` |
| `schedule_su` | `history` |
| `sell_good_nhat` | `best_ask` |
| `show_thi` | `display` |
| `sinh_cap_gia` | `gen_cap_price` |
| `sinh_du_lieu` | `gen_data` |
| `sinh_loi_suat` | `gen_returns` |
| `sinh_mau_do_tre` | `gen_mau_latency` |
| `sinh_nen` | `gen_candle` |
| `sinh_phien` | `generate_session` |
| `sinh_phien_ghi` | `gen_session_record` |
| `so_bit_khong_dau` | `unsigned_bits` |
| `so_buoc` | `num_step` |
| `so_buoc_vat_ly` | `physics_steps` |
| `so_chu_ky` | `num_period` |
| `so_du` | `balance` |
| `so_du_ban_dau` | `first_balance_sell` |
| `so_du_doan_sai` | `wrong_guess_balance` |
| `so_du_moi` | `new_balance` |
| `so_duong` | `positive_count` |
| `so_gd` | `num_trade` |
| `so_giao_dich` | `num_trade` |
| `so_hang` | `num_queue` |
| `so_hieu` | `serial` |
| `so_khe` | `slot_count` |
| `so_khoi` | `num_block` |
| `so_khop` | `fill_count` |
| `so_khung` | `num_frame` |
| `so_lan_ghi` | `count_record` |
| `so_lan_gui` | `count_send` |
| `so_lan_phong_ve` | `hedge_count` |
| `so_lan_tu_choi` | `reject_counts` |
| `so_lenh_bi_chan` | `orders_blocked` |
| `so_lenh_dang_mo` | `order_book_dang_open` |
| `so_lenh_gui` | `orders_sent` |
| `so_lenh_khop` | `order_book_fill` |
| `so_lenh_qua` | `order_book_qua` |
| `so_loi_chiu_duoc` | `fault_tolerance` |
| `so_loi_trang` | `num_error_state` |
| `so_luong` | `quantity` |
| `so_luong_khong` | `quantity_no` |
| `so_luong_ve` | `quantity_ve` |
| `so_mau` | `samples` |
| `so_may_chu` | `num_server` |
| `so_muc` | `num_level` |
| `so_muc_dang_dung` | `num_level_dang_use` |
| `so_nhanh` | `branch_count` |
| `so_phan_tu` | `num_part_from` |
| `so_phan_tu_du` | `num_part_from_data` |
| `so_phep_cong` | `add_op_count` |
| `so_phep_nhan` | `num_op_recv` |
| `so_phien_lai` | `num_session_lai` |
| `so_phien_lo` | `num_session_lo` |
| `so_phieu_thu_duoc` | `votes_received` |
| `so_su_kien` | `event_count` |
| `so_tai_khoan` | `num_account` |
| `so_thu_tu` | `nonce` |
| `so_tiep` | `num_cont` |
| `so_trung` | `num_duplicate` |
| `so_truot` | `slip_count` |
| `so_truy_cap` | `num_access_cap` |
| `so_vong` | `num_round` |
| `so_warp_phan_ky` | `divergent_warps` |
| `so_y_dinh` | `intents` |
| `standard_hoa` | `normalize` |
| `su_kien` | `event` |
| `suc_chua` | `capacity` |
| `sut_giam_toi_da` | `max_drawdown` |
| `suy_kieu` | `infer_type` |
| `tai_in_ky` | `load_in_period` |
| `tai_khoan` | `account` |
| `tai_trong_ky` | `load_in_period` |
| `tam_tinh` | `computed_temp` |
| `tan_suat_doi` | `rate_swap` |
| `tang_tb` | `up_avg` |
| `tang_toc_ly_thuyet` | `theoretical_speedup` |
| `tang_toc_toi_da_neu_xoa_nut` | `max_speedup_if_node_removed` |
| `tao_bo_loc_tu_cam` | `make_ban_filter` |
| `tat_ca` | `all` |
| `temp_tinh` | `computed_temp` |
| `ten_cac_dinh` | `name_all_peak` |
| `ten_dang_nhap` | `name_dang_import` |
| `ten_hang` | `name_queue` |
| `ten_khach` | `customer_name` |
| `ten_truong` | `field_name` |
| `tham_num` | `param` |
| `tham_num_path` | `path_param` |
| `tham_so` | `param` |
| `tham_so_duong_dan` | `path_param` |
| `thanh_bool` | `to_bool` |
| `thanh_dau_ra` | `into_output` |
| `thanh_html` | `to_html` |
| `thanh_thong_ke` | `into_thong_ke` |
| `thanh_thuc` | `into_real` |
| `thanh_tien` | `into_tien` |
| `thanh_toan` | `payment` |
| `thanh_toan_gio` | `checkout` |
| `thay_trang_fifo` | `fifo_replace` |
| `thay_trang_lru` | `lru_replace` |
| `them_canh` | `add_edge` |
| `them_dinh` | `add_peak` |
| `them_hang` | `add_queue` |
| `them_may_chu` | `add_server` |
| `them_quan_he` | `add_relation` |
| `them_thuc_the` | `add_entity` |
| `theo_khoi` | `theo_block` |
| `thieu_hut_ky_vong` | `expected_shortfall` |
| `thoat_html` | `escape_html` |
| `thoi_diem` | `timestamp` |
| `thoi_diem_den` | `arrives_at` |
| `thoi_diem_ns` | `timestamp_nanos` |
| `thoi_diem_vao` | `entered_at` |
| `thoi_gian_ao_ns` | `time_time_ao_nanos` |
| `thoi_gian_can` | `time_time_can` |
| `thoi_gian_cho_thuc_ns` | `real_wait_nanos` |
| `thoi_gian_ms` | `time_ms` |
| `thoi_gian_nam` | `years` |
| `thong_ke_danh_muc` | `portfolio_stats` |
| `thu_gon_khoang_trang` | `reduce_range` |
| `thu_gon_range_state` | `reduce_range` |
| `thu_hoan_doi` | `try_swap` |
| `thu_hoan_doi_x_lay_y` | `try_swap_x_for_y` |
| `thu_hoan_doi_y_lay_x` | `try_swap_y_for_x` |
| `thu_swap` | `try_swap` |
| `thuc_hien_giao_dich` | `display_trade` |
| `thuc_show_trade` | `display_trade` |
| `thuc_te` | `actual` |
| `thuc_thi` | `execute` |
| `thuoc_tinh` | `attribute` |
| `tich_accum` | `accumulate` |
| `tich_accum_nanos` | `accumulated_nanos` |
| `tich_luy` | `accumulate` |
| `tich_luy_ns` | `accumulated_nanos` |
| `tick_sang_chuoi` | `tick_to_string` |
| `tien_to` | `prefix` |
| `tien_toi` | `advance` |
| `tien_trinh` | `process` |
| `tieu_de` | `title` |
| `tieu_diem` | `spend_point` |
| `tim_co_hoi_arb` | `find_arb` |
| `tim_kiem_nhi_phan_ologn` | `binary_search_ologn` |
| `tim_kiem_tuyen_tinh_on` | `linear_search_on` |
| `tim_may_chu` | `find_server` |
| `time_time_wait_thuc_nanos` | `real_wait_nanos` |
| `timestamp_to` | `arrives_at` |
| `tin_hieu` | `signal` |
| `tinh_chi_so_bmi` | `bmi` |
| `tinh_chiet_khau` | `apply_discount` |
| `tinh_chieu_cao` | `height` |
| `tinh_greeks` | `greeks` |
| `tinh_height` | `height` |
| `tinh_muc_chiem_dung` | `occupancy` |
| `tinh_tong_lat_cat` | `total_tile_latency` |
| `tinh_total_lat_cut` | `total_tile_latency` |
| `tk_an` | `account_hidden` |
| `toc_do` | `speed` |
| `toi_da_buoc_mot_khung` | `max_step_one_frame` |
| `toi_thieu` | `min` |
| `toi_thieu_y` | `min_y` |
| `ton_that_tam_thoi` | `impermanent_loss` |
| `tong_binh_phuong` | `sum_of_squares` |
| `tong_chu_ky_cho` | `total_period_wait` |
| `tong_chu_ky_khong_ong` | `total_cycles_no_pipeline` |
| `tong_cung` | `total_supply` |
| `tong_khoi_luong` | `total_qty` |
| `tong_kiem_tra` | `total_check` |
| `tong_lai_lo` | `total_pnl` |
| `tong_luong` | `total_amount` |
| `tong_luong_truy_cap` | `total_amount_access_cap` |
| `tong_vao` | `total_in` |
| `total_binh_phuong` | `sum_of_squares` |
| `total_period_no_ong` | `total_cycles_no_pipeline` |
| `tra_loi` | `return_error` |
| `trang` | `state` |
| `trang_1` | `state_1` |
| `trang_thai` | `state` |
| `tre_lenh` | `order_latency` |
| `treo_ban` | `resting_ask` |
| `treo_mua` | `resting_bid` |
| `trich_xuat` | `extract` |
| `trong` | `in` |
| `trong_so` | `weight` |
| `trong_tai` | `in_tai` |
| `tru_tien` | `debit` |
| `trung_binh` | `mean` |
| `trung_binh_ngoai_mau` | `mean_out_mau` |
| `trung_binh_trong_mau` | `mean_in_mau` |
| `trung_binh_truot` | `moving_average` |
| `truoc` | `prev` |
| `truot_bat_buoc` | `compulsory_miss` |
| `truot_do_capacity` | `capacity_miss` |
| `truot_do_dung_luong` | `capacity_miss` |
| `truot_enable_step` | `compulsory_miss` |
| `truot_gia` | `slippage` |
| `truy_cap` | `access_cap` |
| `truy_cap_cot_lat` | `access_cap_col_lat` |
| `truy_xuat_lan_toa` | `broadcast_access` |
| `tt_ban` | `lit_sell` |
| `tt_mat_can_bang` | `lit_imbalance` |
| `tt_mua` | `lit_buy` |
| `tt_vi_gia` | `lit_micro_price` |
| `tu_bool` | `from_bool` |
| `tu_bool_` | `from_bool` |
| `tu_khop` | `from_fill` |
| `tu_so` | `numerator` |
| `tu_tam` | `self_centered` |
| `tu_thuc` | `from_real` |
| `tuan_tu_hoa` | `serialize` |
| `tuong_quan` | `correlation` |
| `tuyen` | `route` |
| `ty_le` | `ratio` |
| `ty_le_kelly` | `kelly_fraction` |
| `ty_so_sharpe` | `sharpe_ratio` |
| `ung_vien` | `candidates` |
| `use_thu_from` | `try_fold` |
| `uy_quyen` | `allowance` |
| `van_toc` | `velocity` |
| `vao_luc` | `entered_at` |
| `vao_ns` | `in_nanos` |
| `vao_x` | `x_in` |
| `vi_the` | `position` |
| `vi_the_cuoi` | `last_position` |
| `vi_the_toi_da` | `max_position` |
| `vi_tri` | `pos_value` |
| `vi_tri_doc` | `pos_value_read` |
| `vi_tri_ghi` | `pos_value_record` |
| `vi_tri_phan_tram` | `pos_value_percent` |
| `vi_tri_trong_hang` | `queue_position` |
| `viet_hoa_chu_dau` | `capitalize_first` |
| `view_dem` | `counter_view` |
| `view_hoa_don_safe` | `invoice_view_safe` |
| `vong_dong_thuan` | `consensus_round` |
| `vuot_gia_tri` | `exceed_value` |
| `vuot_vi_the` | `exceed_position` |
| `warp_dong_thoi` | `concurrent_warps` |
| `xac_thuc` | `auth` |
| `xem_hoa_don_an_toan` | `invoice_view_safe` |
| `xep_lich_song_song` | `arrange_schedule_parallel` |
| `xu_ly` | `handle` |
| `xu_ly_co_ong` | `handle_with_pipeline` |
| `xu_ly_don_ke_tiep` | `handle_don_ke_cont` |
| `y_dinh` | `intent` |

---

## Ghi chú về cách dịch