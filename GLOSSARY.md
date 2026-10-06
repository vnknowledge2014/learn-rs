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
| Ảnh chụp (giao dịch) | Snapshot | Tập giao dịch đã commit mà một giao dịch nhìn thấy |
| Người commit trước thắng | First-committer-wins | Luật chống mất cập nhật của Snapshot Isolation |
| Lệch ghi | Write skew | Dị thường Snapshot Isolation không chặn được |
| Chép khoá lên / chuyển khoá lên | Copy-up / push-up | Tách nút lá B+Tree chép khoá; tách nút trong chuyển khoá |

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
| Ngang giá quyền chọn mua–bán | Put-call parity | Chênh lệch giá, **không** phải mô hình |
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
| Chống tự khớp | Self-trade prevention | Lệnh của ta không được khớp với nhau |
| Iron condor | Iron condor | Giữ tên tiếng Anh |

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
| `ArrayOpenPhong` | `SimNetwork` |
| `Ban` | `Put` |
| `BanGhi` | `DnsRecord` |
| `BanGhiCamBien` | `SensorRecord` |
| `BanGhiNguoiDung` | `UserRecord` |
| `BanGhiNhatKy` | `LogRecord` |
| `BanGhiPhienBan` | `VersionedRecord` |
| `BanGhiTruyCap` | `AccessRecord` |
| `BanVa` | `Patch` |
| `BangBamPhanTan` | `DistributedHashTable` |
| `BangBaoGiaSoA` | `QuoteTableSoA` |
| `BangDinhTuyen` | `RoutingTable` |
| `BaoGiaAoS` | `QuoteAoS` |
| `BeDoiTuong` | `ObjectPool` |
| `BeRong` | `EmptyPool` |
| `BeThanhKhoan` | `Pool` |
| `BieuDoTre` | `LatencyHistogram` |
| `BoDemChungDong` | `SameLineCounters` |
| `BoDemTachDong` | `SplitLineCounters` |
| `BoDieuKhienDen` | `TrafficController` |
| `BoDinhTuyen` | `Router` |
| `BoDoLuong` | `Metrics` |
| `BoGhiPhien` | `SessionRecorder` |
| `BoKhung` | `Harness` |
| `BoLoc` | `Filter` |
| `BoNao` | `Brain` |
| `BoNaoGia` | `FakeBrain` |
| `BoNgoaiVi` | `Peripherals` |
| `BoPhatHienKhe` | `GapDetector` |
| `BoPhatLai` | `Replayer` |
| `BoSinh` | `Generator` |
| `BoTacNghen` | `CongestionControl` |
| `BoTachTruong` | `FieldExtractor` |
| `BoTichLuy` | `Accumulator` |
| `BoTichLuyNguyen` | `IntegerAccumulator` |
| `BoXuLy` | `Handler` |
| `BufferChungClose` | `SameLineCounters` |
| `BuocChungMinh` | `ProofStep` |
| `BuocTiep` | `Step` |
| `CallNguEdge` | `ContextPack` |
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
| `Chan` | `Pin` |
| `ChangDoTre` | `LatencyStage` |
| `ChenhLechHaiSan` | `CrossVenueArb` |
| `ChiDanDauRa` | `OutPoint` |
| `ChienLuoc` | `Strategy` |
| `ChienLuocCanBang` | `BalancingStrategy` |
| `ChienLuocPhatLai` | `ReplayStrategy` |
| `ChienLuocQuyen` | `OptionStrategy` |
| `Chieu` | `Side` |
| `ChuaCauHinh` | `Unconfigured` |
| `ChucHeavy` | `Feature` |
| `ChucNang` | `Feature` |
| `ChungThucBaoMat` | `SecurityAttestation` |
| `ChungThucReportMat` | `SecurityAttestation` |
| `ChuoiKhoi` | `Chain` |
| `ChuongTrinhDem` | `CounterProgram` |
| `CoHoiArb` | `ArbOpportunity` |
| `CoIt` | `Any` |
| `CoTuChoi` | `RejectFlags` |
| `CongCu` | `Tool` |
| `CongCuTinhToan` | `CalculatorTool` |
| `CongCuTraCuu` | `LookupTool` |
| `CongGianDiep` | `SpyGateway` |
| `CongLuonHong` | `AlwaysFailGateway` |
| `CongOk` | `OkGateway` |
| `CongOldCompute` | `CalculatorTool` |
| `CongRuiRo` | `RiskGate` |
| `CongThanhToan` | `PaymentGateway` |
| `CongThat` | `RealGateway` |
| `CongViec` | `Task` |
| `CongWork` | `Task` |
| `CuaSo` | `Window` |
| `CuaSoThongKe` | `StatsWindow` |
| `DaAuth` | `Validated` |
| `DaCheckRisk` | `RiskChecked` |
| `DaGiao` | `Delivered` |
| `DaGui` | `Sent` |
| `DaIntoToan` | `Paid` |
| `DaKhop` | `Traded` |
| `DaKiemTraRuiRo` | `RiskChecked` |
| `DaNgatKhanCap` | `KillSwitchOn` |
| `DaSend` | `Sent` |
| `DaThanhToan` | `Paid` |
| `DaXacThuc` | `Validated` |
| `DaiBollinger` | `BollingerBands` |
| `DanXuat` | `Derived` |
| `DanhSachLienKet` | `LinkedList` |
| `DatCoPhongVe` | `PlaceHedged` |
| `DatLenh` | `Place` |
| `DauRa` | `Output` |
| `DauTien` | `First` |
| `DauVao` | `Input` |
| `DauVaoBangKhong` | `ZeroInput` |
| `DemCoDem` | `PaddedCounter` |
| `DemNguoc` | `Countdown` |
| `DemVong` | `RingBuffer` |
| `DenGiaoThong` | `TrafficLight` |
| `DiaChi` | `Address` |
| `DoThi` | `Graph` |
| `DoThiCho` | `WaitForGraph` |
| `DoThiTriThuc` | `KnowledgeGraph` |
| `DoThiValueThuc` | `KnowledgeGraph` |
| `DoThiWait` | `WaitForGraph` |
| `DoanKiemDinh` | `TestSegment` |
| `DoanTest` | `TestSegment` |
| `DoiDonNguyen` | `Comonad` |
| `DonHang` | `Order` |
| `DonHangDto` | `OrderDto` |
| `DonNguyen` | `Monad` |
| `DongHang` | `OrderLine` |
| `DongHangDto` | `OrderLineDto` |
| `DongHoAo` | `VirtualClock` |
| `DuDoanNhanh` | `BranchPredictor` |
| `DungNgoai` | `SitOut` |
| `DuongOngPhanCung` | `HwPipeline` |
| `ErrorIntoToan` | `PaymentError` |
| `FillCuaTa` | `OurFill` |
| `GapDuoc` | `Foldable` |
| `Gia` | `Price` |
| `GiaNgoaiBien` | `PriceOutOfBand` |
| `GiaTri` | `Value` |
| `GiaTriAbi` | `AbiValue` |
| `GiaTriLenhQuaLon` | `OrderValueTooLarge` |
| `GiaTriMacd` | `MacdValue` |
| `GiaoCatTrungBinh` | `MeanCross` |
| `GiaoCutMean` | `MeanCross` |
| `GiaoDich` | `Transaction` |
| `GiaoDich1559` | `Tx1559` |
| `GiaoDichCho` | `PendingTx` |
| `GioHang` | `Cart` |
| `GoiLenh` | `OrderPacket` |
| `GoiNguCanh` | `ContextPack` |
| `GoiTinTruong` | `FeedPacket` |
| `HamTu` | `Functor` |
| `HamTuHaiNgoi` | `Bifunctor` |
| `HanMuc` | `Limit` |
| `HanMucRuiRo` | `RiskLimits` |
| `HangDoiDonHang` | `OrderQueue` |
| `HangDoiGioiHan` | `BoundedQueue` |
| `HanhDong` | `Action` |
| `HanhVi` | `Behavior` |
| `HeSinhThai` | `Ecosystem` |
| `HienTai` | `Current` |
| `HoSoHopLe` | `ValidProfile` |
| `HoSoTho` | `RawProfile` |
| `HoaDon` | `Invoice` |
| `HoanDoiTrenBe` | `PoolSwap` |
| `HopBao` | `Aabb` |
| `HuyLenh` | `CancelOrder` |
| `IntoToan` | `PaymentMethod` |
| `ItKetNoi` | `LeastConnections` |
| `KetQuaCong` | `AdderResult` |
| `KetQuaCongCu` | `ToolResult` |
| `KetQuaHoiQuy` | `RegressionResult` |
| `KetQuaKiemDinh` | `BacktestResult` |
| `KetQuaKiemDinhTien` | `WalkForwardResult` |
| `KetQuaLanTruyen` | `PropagationResult` |
| `KetQuaLenh` | `CommandResult` |
| `KetQuaNhom` | `GroupResult` |
| `KetQuaOng` | `PipelineResult` |
| `KetQuaPhatLai` | `ReplayResult` |
| `KetQuaThayTrang` | `ReplacementResult` |
| `KetQuaTruyen` | `TransferResult` |
| `KetQuaVong` | `RoundResult` |
| `KetQuaVongLap` | `LoopResult` |
| `KheBanGhi` | `RecordSlot` |
| `KheSellRecord` | `RecordSlot` |
| `Kho` | `Store` |
| `KhoLuuTruNhiPhan` | `BinaryStore` |
| `Khoi` | `Pin` |
| `KhoiLuongQuaLon` | `QuantityTooLarge` |
| `KhongDatToiThieu` | `BelowMinOut` |
| `KhongLamGi` | `NoOp` |
| `KhopCuaTa` | `OurFill` |
| `KhopLenh` | `Fill` |
| `KhungGhi` | `RecordedFrame` |
| `KiemToanBaoMat` | `SecurityAudit` |
| `KiemToanReportMat` | `SecurityAudit` |
| `KiemTraTaiKhoan` | `CheckAccount` |
| `KyQuy` | `Escrow` |
| `Lenh` | `Order` |
| `LenhBackend` | `BackendCommand` |
| `LenhCuaTa` | `OurOrder` |
| `LenhDangBay` | `InFlightOrder` |
| `LenhL3` | `L3Order` |
| `LenhLuuTep` | `SaveFileCommand` |
| `LenhThongTinHeThong` | `SystemInfoCommand` |
| `LenhToken` | `TokenMsg` |
| `LoaiCauPhan` | `LegKind` |
| `LoaiQuyen` | `OptionKind` |
| `LoaiSuKien` | `EventKind` |
| `LocDuoc` | `Filterable` |
| `LocKalman` | `KalmanFilter` |
| `LoiDoc` | `ReadError` |
| `LoiGiaoDich` | `TransactionError` |
| `LoiGio` | `CartError` |
| `LoiHoanDoi` | `SwapError` |
| `LoiHopDong` | `ContractError` |
| `LoiKhoi` | `BlockError` |
| `LoiMatKhau` | `PasswordError` |
| `LoiMien` | `DomainError` |
| `LoiPhanTich` | `ParseError` |
| `LoiRuiRo` | `RiskError` |
| `LoiSolana` | `SolanaError` |
| `LoiThanhToan` | `PaymentError` |
| `LoiTruyCap` | `AccessError` |
| `LoiUrl` | `UrlError` |
| `LonNhat` | `Max` |
| `LuaChonThayThe` | `Alternative` |
| `LuonMua` | `AlwaysBuy` |
| `LyDoDung` | `StopReason` |
| `MaLenh` | `OrderId` |
| `MachDien` | `Circuit` |
| `MachElec` | `Circuit` |
| `MachRuiRo` | `RiskCircuit` |
| `MangMoPhong` | `SimNetwork` |
| `MatHang` | `Item` |
| `MauHinh` | `Pattern` |
| `MauNguCanh` | `ContextChunk` |
| `MauNguEdge` | `ContextChunk` |
| `MayChu` | `Server` |
| `MayChuDns` | `DnsServer` |
| `MoHinh` | `Model` |
| `MoHinhDoTre` | `LatencyModel` |
| `MoPhongCache` | `CacheSim` |
| `MoRong` | `Extend` |
| `MoTaChiTiet` | `DetailedDescription` |
| `MoiTruong` | `Env` |
| `Mua` | `Call` |
| `MucChiemDung` | `Occupancy` |
| `MucGia` | `PriceLevel` |
| `MucGiaPC` | `HwPriceLevel` |
| `Nano` | `Nanos` |
| `Nen` | `Candle` |
| `NewTruong` | `Env` |
| `NganSachDoTre` | `LatencyBudget` |
| `NguoiDung` | `User` |
| `Nhanh` | `Fast` |
| `Nhap` | `Draft` |
| `NhipKhung` | `FrameClock` |
| `NhoNhat` | `Min` |
| `Nhom` | `Group` |
| `Noi` | `Chain` |
| `NoiThucThi` | `ExecutionUnit` |
| `NuaGroup` | `Semigroup` |
| `NuaNhom` | `Semigroup` |
| `NutAo` | `VirtualNode` |
| `OldCong` | `Tool` |
| `OldResultCong` | `ToolResult` |
| `OpenHinh` | `Model` |
| `OrderL3` | `L3Order` |
| `OrderThongInfoHeThong` | `SystemInfoCommand` |
| `OwnedDoRisk` | `RiskMetrics` |
| `PacketTruong` | `FeedPacket` |
| `PhaTacNghen` | `CongestionPhase` |
| `Phai` | `Right` |
| `Phan` | `Item` |
| `PhanDauKhoi` | `BlockHeader` |
| `PhanHoi` | `Response` |
| `PhanTichGemm` | `GemmAnalysis` |
| `PhanTichGop` | `CoalescingAnalysis` |
| `PhanTichIlp` | `IlpAnalysis` |
| `PhanTichNganHang` | `BankAnalysis` |
| `PhanTichPhanKy` | `DivergenceAnalysis` |
| `PhanTichSimd` | `SimdAnalysis` |
| `PhanTichSongSong` | `ParallelAnalysis` |
| `PhienDaGhi` | `RecordedSession` |
| `PhuongThuc` | `Method` |
| `PositionGioi` | `World` |
| `ProxyNumHopLe` | `ValidProfile` |
| `ResultCong` | `AdderResult` |
| `ResultOng` | `PipelineResult` |
| `ResultThayState` | `ReplacementResult` |
| `ResultTruyen` | `TransferResult` |
| `RoundHashNhatQuan` | `ConsistentHashRing` |
| `San` | `Venue` |
| `SanChuoiKhoi` | `ChainVenue` |
| `SanTruyenThong` | `LitVenue` |
| `SoLenh` | `OrderBook` |
| `SoLenhL2` | `L2Book` |
| `SoLenhL3` | `L3Book` |
| `SoLenhPhanCung` | `HwOrderBook` |
| `SoLuong` | `Quantity` |
| `SoRutGon` | `ReducedBook` |
| `SoTien` | `Money` |
| `StoreSaveTruNhiPart` | `BinaryStore` |
| `SuKien` | `Event` |
| `SuKienPhien` | `SessionEvent` |
| `SuKienTcp` | `TcpEvent` |
| `SuKienThiTruong` | `MarketEvent` |
| `TaiKhoan` | `Account` |
| `TaiKhoanNganHang` | `BankAccount` |
| `TangBoNho` | `MemoryTier` |
| `TangOng` | `PipelineStage` |
| `TaoLapCoKiemSoat` | `ManagedMaker` |
| `TaoLapDonGian` | `NaiveMaker` |
| `ThamSoQuyen` | `OptionParams` |
| `ThanhGhiDich` | `ShiftRegister` |
| `ThanhGhiGia` | `FakeRegisters` |
| `ThanhToan` | `PaymentMethod` |
| `TheGioi` | `World` |
| `TheVatLy` | `PhysicsBody` |
| `ThemLenh` | `AddOrder` |
| `ThietBiMang` | `NetworkDevice` |
| `ThoiGianThuc` | `RealTime` |
| `ThongBaoNguyHiem` | `DangerAlert` |
| `ThongDiep` | `Message` |
| `ThongKeCache` | `CacheStats` |
| `ThongKeDanhMuc` | `PortfolioStats` |
| `ThuFrom` | `Ord` |
| `ThuTu` | `Ord` |
| `ThucPosition` | `Entity` |
| `ThucThe` | `Entity` |
| `ThuocDoRuiRo` | `RiskMetrics` |
| `Tich` | `Product` |
| `TienTrinh` | `Process` |
| `TinHieu` | `Signal` |
| `TinHieuCap` | `PairSignal` |
| `ToGiaoThong` | `TrafficLight` |
| `ToaDoGps` | `GpsCoord` |
| `TocDoPhat` | `ReplaySpeed` |
| `Trai` | `Left` |
| `TrangThai` | `State` |
| `TrangThaiDem` | `CounterState` |
| `TrangThaiDonHang` | `OrderStatus` |
| `TrangThaiKyQuy` | `EscrowState` |
| `TrangThaiTcp` | `TcpState` |
| `TrangThaiTienTrinh` | `ProcessState` |
| `TrichXuat` | `Extract` |
| `TruongGoiTin` | `PacketField` |
| `TruongPacket` | `PacketField` |
| `TruyenThong` | `Lit` |
| `TuChoi` | `RejectReason` |
| `TuongFrom` | `Analog` |
| `TuongTu` | `Analog` |
| `Tuyen` | `Route` |
| `UnitPeakTuyen` | `Router` |
| `UnitTichAccum` | `Accumulator` |
| `UpOng` | `PipelineStage` |
| `ValueAbi` | `AbiValue` |
| `ViNhom` | `Monoid` |
| `ViThe` | `Position` |
| `ViTu` | `Predicate` |
| `VoHan` | `Unbounded` |
| `VongBamNhatQuan` | `ConsistentHashRing` |
| `VongDisruptor` | `DisruptorRing` |
| `VuotHanMucLo` | `LossLimit` |
| `VuotHanMucViThe` | `PositionLimit` |
| `VuotTanSuat` | `RateLimit` |
| `WindowThongKe` | `StatsWindow` |
| `XacThuc` | `Validation` |
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
| `anh_remote` | `fmap` |
| `anh_xa` | `fmap` |
| `ap_dung` | `apply` |
| `ap_dung_khop` | `apply_fill` |
| `ap_suat_bar` | `pressure_bar` |
| `array_tinh` | `static_array` |
| `balance_doan_sai` | `mispredictions` |
| `bam64` | `hash64` |
| `bam_khoi_truoc` | `prev_block_hash` |
| `ban_ghi` | `record` / `records` |
| `ban_tot_nhat` | `best_ask` |
| `bat_dau` | `start` |
| `bay_ban` | `in_flight_ask` |
| `bay_gio` | `now` |
| `bay_mua` | `in_flight_bid` |
| `be_mau` | `sample_pool` |
| `ben_mua` | `buy_side` / `is_buy` |
| `bfs_khoang_cach_ngan_nhat` | `bfs_shortest_distance` |
| `bi_bo` | `dropped` |
| `bi_bo_buoc` | `steps_dropped` |
| `bi_chan` | `is_even` / `is_blocked` |
| `bi_chan_boi` | `blocked_by` |
| `bi_chan_boi_bao_ve` | `blocked_by_guard` |
| `bi_cheo` | `is_crossed` |
| `bien_dong_ngu_y` | `implied_volatility` |
| `bieu_do` | `histogram` |
| `bo_dem` | `counter` |
| `bo_loc` | `filter` |
| `bo_tai_khoan` | `account_set` |
| `bong_duoi` | `lower_wick` |
| `bong_tren` | `upper_wick` |
| `buoc_co_dinh` | `fixed_step` |
| `buoc_euler_nua_an` | `semi_implicit_euler_step` |
| `buoc_euler_tuong_minh` | `explicit_euler_step` |
| `buoc_ns` | `step_nanos` |
| `buy_good_nhat` | `best_bid` |
| `byte_da_chuyen` | `bytes_transferred` |
| `byte_moi_o` | `bytes_per_cell` |
| `cac_doan` | `segments` |
| `cac_khoi` | `blocks` |
| `cac_khop` | `fills` |
| `cac_mau` | `chunks` |
| `cac_su_kien` | `events` |
| `cac_tang` | `layers` |
| `can_duoi_chau_au` | `european_lower_bound` |
| `cap_nhat` | `update` |
| `cap_nhat_tien_trinh` | `update_progress` |
| `cat_khoang_trang` | `trim_whitespace` |
| `cau_phan` | `legs` |
| `cay_xor` | `xor_tree` |
| `chay_kiem_dinh` | `run_backtest` |
| `chay_phien` | `run_session` |
| `chay_vong_lap` | `run_agent_loop` |
| `chenh_lech` | `spread` |
| `chi_phi_ban_dau` | `initial_cost` |
| `chiet_khau` | `discount` |
| `chieu` | `side` |
| `chieu_cao` | `height` |
| `chieu_cao_dinh` | `tip_height` |
| `chieu_chu_dong` | `side_aggressive` |
| `cho_bao_lau` | `wall_delay` |
| `cho_phep` | `allowed_hosts` / `try_acquire` / `approve` / `slippage_bps` |
| `cho_trung_binh` | `avg_wait` |
| `chon_tuyen` | `match_route` |
| `chu_account` | `account_owner` |
| `chu_dong` | `aggressive` |
| `chu_ky` | `signature` |
| `chu_ky_ham` | `selector` |
| `chu_ky_khoi_dau` | `initiation_interval` |
| `chu_ky_phi` | `wasted_cycles` |
| `chu_ky_sang_ns` | `cycles_to_ns` |
| `chu_ky_uoc_tinh` | `estimated_cycles` |
| `chu_so_huu` | `owner` |
| `chu_tai_khoan` | `account_owner` |
| `chua_key` | `contains_key` |
| `chua_khoa` | `contains_key` |
| `chuan_hoa` | `normalize` |
| `chung_minh` | `prove` |
| `chuoi` | `text` / `strings` / `as_str` / `refs` |
| `chuoi_atr` | `atr_series` |
| `chuoi_co_tien` | `chain_with_funds` |
| `chuoi_ema` | `ema_series` |
| `chuoi_macd` | `macd_series` |
| `chuoi_rsi` | `rsi_series` |
| `chuoi_sma` | `sma_series` |
| `chuyen` | `transition` |
| `ck_du_tru_x` | `chain_reserve_x` |
| `ck_du_tru_y` | `chain_reserve_y` |
| `ck_gia` | `chain_price` |
| `close_call_ngu_edge` | `pack_context` |
| `close_nhat` | `identity` |
| `co_be_tac` | `has_deadlock` |
| `co_co_hoi` | `has_opportunity` |
| `co_lenh` | `order_size` |
| `co_so` | `base` |
| `co_theo_bien_dong` | `size_by_volatility` |
| `co_xung_dot` | `has_conflict` |
| `con_lai` | `remaining` |
| `con_tro` | `pointer` |
| `con_tro_day` | `free_space_pointer` |
| `cong_and` | `and_gate` |
| `cong_cu` | `tools` |
| `cong_don` | `accumulate` |
| `cong_hoac` | `or_gate` |
| `cong_khong` | `not_gate` |
| `cong_mau` | `sample_gate` |
| `cong_nhin_truoc_8bit` | `lookahead_adder_8bit` |
| `cong_no` | `not_gate` |
| `cong_noi_tiep_8bit` | `ripple_adder_8bit` |
| `cong_tac_all` | `kill_switch` |
| `cong_tac_tat` | `kill_switch` |
| `cong_va` | `and_gate` |
| `cong_viec` | `tasks` / `work` |
| `cong_work` | `tasks` / `work` |
| `cong_xor` | `xor_gate` |
| `count_has_nhanh` | `branch_taken_count` |
| `cua_so` | `window` |
| `cua_so_lenh` | `order_times` |
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
| `da_tieu` | `spent` |
| `da_vao` | `admitted` |
| `dan_xuat_pda` | `derive_pda` |
| `dang_bay` | `in_flight` |
| `dang_khoi_phuc` | `is_recovering` |
| `dang_ky` | `register` |
| `dang_ky_ngan_mach` | `register_short_circuit` |
| `dang_ky_tich_accum` | `register_accumulate` |
| `dang_ky_tich_luy` | `register_accumulate` |
| `dang_mo` | `open_orders` / `open_signal` |
| `danh_gia` | `evaluate` |
| `danh_sach` | `list` |
| `danh_sach_cho_phep` | `allowed_symbols` |
| `danh_sach_dai` | `range_entries` |
| `danh_sach_ke` | `adjacency_list` |
| `danh_sach_phien_ban` | `versions` |
| `danh_sach_ten` | `names` |
| `dao_dong_ns` | `jitter_ns` |
| `dao_khoi_moi` | `mine_new_block` |
| `dao_khoi_tren` | `mine_on` |
| `dao_nguoc_tai_cho` | `reverse_in_place` |
| `dat_lai` | `reset` |
| `dat_lenh_cua_ta` | `place_our_order` |
| `dat_thue_rieng` | `colocated` |
| `data_doan` | `predict` |
| `data_kien` | `amount_in` |
| `dau_ra` | `output` |
| `dau_vao` | `input` |
| `day_con_chung_dai_nhat` | `longest_common_subsequence` |
| `day_vao` | `push` |
| `dem_co_nhanh` | `branch_taken_count` |
| `dem_lai` | `buffered` |
| `dem_trong_cua_so` | `window_count` |
| `den_luc` | `arrives_at` |
| `dia_chi` | `address` |
| `dia_chi_bo_dem` | `counter_address` |
| `dia_chi_hien_tai` | `current_address` |
| `dia_chi_mang` | `network_address` |
| `dia_chi_truoc` | `prev_address` |
| `dia_chi_tu_hex` | `address_from_hex` |
| `dien_hinh` | `typical` |
| `dinh_an` | `an_tip` |
| `dinh_so` | `top_levels` |
| `do_dai` | `length` |
| `do_dai_ten` | `name_len` |
| `do_kho` | `difficulty` |
| `do_lech_chuan` | `stddev` |
| `do_lech_tick` | `tick_offset` |
| `do_long` | `length` |
| `do_luong` | `metrics` |
| `do_next_cong` | `gate_depth` |
| `do_next_xor_tree` | `xor_tree_depth` |
| `do_risk` | `measure_risk` |
| `do_rui_ro` | `measure_risk` |
| `do_sau_cong` | `gate_depth` |
| `do_tre` | `latency` |
| `do_tre_chu_ky` | `latency_cycles` |
| `do_tre_ns` | `latency_nanos` |
| `doc_toan_cuc` | `read_global` |
| `doi_tien` | `coin_change` |
| `doi_ung` | `contra_side` |
| `doi_ung_la_ban` | `contra_is_sell` |
| `don_gia` | `unit_price` |
| `don_nhap` | `draft_order` |
| `don_vi` | `unit` / `empty` / `lot_size` |
| `dong_goi_ngu_canh` | `pack_context` |
| `dong_ho` | `clock` |
| `dong_nhat` | `identity` |
| `du_doan` | `predict` |
| `du_lieu` | `data` |
| `du_lieu_sai` | `unprocessable` |
| `du_lieu_tho` | `raw_data` |
| `du_tru_x` | `reserve_x` |
| `du_tru_y` | `reserve_y` |
| `dung_luong` | `capacity` |
| `duoc_ghi` | `is_writable` |
| `duong_dan` | `path` |
| `duong_dan_an_toan` | `safe_path` |
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
| `gd_mau` | `sample_tx` |
| `gemm_ngay_tho` | `gemm_naive` |
| `gemm_theo_lat` | `tiled_gemm` |
| `gen_error_suat` | `gen_returns` |
| `get_num_khe` | `slot_count` |
| `ghep_voi` | `compose_with` |
| `ghi_nhan` | `record` |
| `ghi_nhan_khop` | `record_fill` |
| `ghi_truong` | `write_field` |
| `ghi_tu_choi` | `record_reject` |
| `gia_ban` | `ask_price` / `put_strike` |
| `gia_ban_tot_nhat` | `best_ask` |
| `gia_co_so` | `spot` |
| `gia_cuoi` | `last_price` |
| `gia_dex_sau` | `dex_price_after` |
| `gia_dong` | `close_price` |
| `gia_giua` | `mid` |
| `gia_hien` | `current_price` |
| `gia_khong` | `invalid_price` |
| `gia_mua` | `bid_price` / `call_strike` |
| `gia_mua_tot_nhat` | `best_bid` |
| `gia_tham_chieu` | `reference_price` |
| `gia_thuc` | `exec_price` |
| `gia_thuc_hien` | `strike` |
| `gia_thuc_hien_chiet_khau` | `discounted_strike` |
| `gia_tri` | `value` |
| `gia_tri_chiu_rui_ro` | `value_at_risk` |
| `gia_tri_cuoi` | `last_value` |
| `gia_tri_lenh_toi_da` | `max_order_value` |
| `gia_tri_mau` | `sample_value` |
| `gia_tri_noi_tai` | `intrinsic_value` |
| `gia_tri_rong` | `net_value` |
| `gia_tri_tam` | `temp_value` |
| `gia_tri_thoi_gian` | `time_value` |
| `gia_truoc` | `prev_price` |
| `gia_vi_mo` | `micro_price` |
| `gia_von` | `cost_basis` |
| `gia_x` | `price_x` |
| `giai_ngan` | `release` |
| `giam_gia` | `discount` |
| `giam_sat_thong_so` | `monitor_metrics` |
| `giam_tb` | `down_avg` |
| `giao_dich` | `transactions` |
| `giao_each` | `intersects` |
| `giao_nhau` | `intersects` |
| `gioi_han_gas` | `gas_limit` |
| `goc_merkle` | `merkle_root` |
| `goc_merkle_tinh_lai` | `recompute_merkle_root` |
| `goi_hop_le` | `valid_packet` |
| `good_nhat` | `best` |
| `gop_tat_ca` | `combine_all` |
| `hai_ma` | `two_assets` |
| `han_chot` | `deadline` |
| `han_muc` | `limit` |
| `han_muc_ton_kho` | `inventory_limit` |
| `handle_has_ong` | `run_pipelined` |
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
| `hoan_pos` | `permutations` |
| `hoan_tien` | `refund` |
| `hoan_vi` | `permutations` |
| `hoi_quy` | `regression` |
| `hop_le` | `is_valid` |
| `id_ke_tiep` | `next_id` |
| `in_thong_tin` | `print_info` |
| `into_thuc` | `into_real` |
| `into_toan` | `payment` |
| `is_thuc_thi` | `is_executable` |
| `ke_cont` | `next` |
| `ke_tan_cong_lai` | `attacker_profit` |
| `ke_tiep` | `next` |
| `ket_noi_hien_tai` | `active_connections` |
| `ket_qua_dem` | `frequencies` |
| `ket_thuc` | `end` |
| `khach` | `customer` |
| `khe_dang_cho` | `pending_gap` |
| `khi_co_su_kien` | `on_event` |
| `khoang_cach` | `distance` |
| `khoi_luong` | `quantity` |
| `khoi_luong_dung_truoc` | `queue_ahead` |
| `khoi_luong_khop` | `filled_qty` |
| `khoi_luong_tai` | `qty_at` |
| `khoi_luong_toi_uu` | `optimal_quantity` |
| `khoi_luong_truoc` | `queue_ahead` |
| `khoi_luong_truoc_mat` | `queue_ahead` |
| `khoi_tao` | `create` / `init` |
| `khong` | `no` |
| `khong_do_tre` | `no_latency` |
| `khu_hoi_ns` | `round_trip_ns` |
| `khung` | `frame` |
| `khung_moi` | `advance` |
| `kich_ban` | `scenarios` |
| `kich_thuoc` | `size` |
| `kich_thuoc_o` | `cell_size` |
| `kiem_chung` | `verify` |
| `kiem_chung_don_vi` | `verify_identity` |
| `kiem_chung_ket_hop` | `verify_associativity` |
| `kiem_dinh_dong_lien_ket` | `cointegration_test` |
| `kiem_dinh_tien` | `walk_forward` |
| `kiem_thu` | `tests` |
| `kiem_tra` | `check` |
| `kiem_tra_do_manh` | `check_strength` |
| `kiem_tra_hop_le` | `verify_checksum` |
| `kiem_tra_ngoac_hop_le` | `is_balanced_brackets` |
| `kiem_tra_ten` | `check_name` |
| `kiem_tra_tu_cam` | `check_banned_words` |
| `kiem_tra_url_an_toan` | `is_safe_url` |
| `kiem_traverse` | `validator` |
| `kl_ban` | `ask_qty` |
| `kl_mua` | `bid_qty` |
| `ky_round` | `expected_seq` |
| `ky_vong` | `expected_seq` |
| `la_chan` | `is_even` / `is_blocked` |
| `la_ky` | `is_signer` |
| `la_thuc_thi` | `is_executable` |
| `lai_khi` | `profit_at` |
| `lai_lo` | `pnl` |
| `lai_lo_da_chot` | `realized_pnl` |
| `lai_suat` | `rate` |
| `lai_uoc_tinh` | `estimated_return` |
| `lan_truyen_gossip` | `gossip_propagate` |
| `lang_gieng` | `neighbors` |
| `lanh_manh` | `is_healthy` |
| `lap_khe` | `fill_gap` |
| `lay_con_tro_day` | `free_space_pointer` |
| `lay_ra` | `take` |
| `lay_so_khe` | `slot_count` |
| `lenh_cho` | `resting_orders` |
| `lenh_cua_ta` | `our_orders` |
| `lenh_da_gui` | `sent_order` |
| `lenh_ra_ns` | `outbound_ns` |
| `lenh_thi_truong` | `market_orders` |
| `lenh_thu_dong` | `passive_order` |
| `lich_su` | `history` |
| `list_ke` | `adjacency_list` |
| `lo_trong_ngay_toi_da` | `max_daily_loss` |
| `loc_anh_xa` | `filter_map` |
| `lon_nhat` | `max` |
| `loop_khe` | `fill_gap` |
| `lru_danh_sach` | `lru_list` |
| `luong` | `amount` |
| `luong_lang_phi_moi_khoi` | `wasted_per_block` |
| `luong_moi_khoi` | `threads_per_block` |
| `luy_ke` | `cumulative` |
| `luy_thua_mod` | `mod_pow` |
| `ly_do_dung` | `stop_reason` |
| `ma_chuoi` | `chain_id` |
| `ma_ck` | `symbol` |
| `ma_cu` | `old_id` |
| `ma_don` | `order_code` |
| `ma_giao_dich` | `transaction_id` |
| `ma_hoa` | `encode` |
| `ma_hoa_abi` | `abi_encode` |
| `ma_ke` | `next_id` |
| `ma_ke_tiep` | `next_id` |
| `ma_lenh` | `order_id` |
| `ma_mau` | `sample_ids` |
| `ma_trang_thai` | `status_code` |
| `mang_xa_hoi` | `social_network` |
| `mat_can_bang` | `imbalance` |
| `mat_do_chuan` | `normal_pdf` |
| `mat_hang` | `items` |
| `max_lo_in_day` | `max_daily_loss` |
| `mean_truot` | `moving_average` |
| `merkle_root_tinh_lai` | `recompute_merkle_root` |
| `mo_phong_kep` | `simulate_sandwich` |
| `mo_ta` | `description` |
| `moi_n_su_kien` | `every_n_events` |
| `moi_truong` | `environment` |
| `mua_tot_nhat` | `best_bid` |
| `muc_sut_giam` | `degradation` |
| `muc_tieu` | `target` |
| `muc_xung_dot` | `conflict_degree` |
| `n_chuan` | `norm_cdf` |
| `nam_tren_duong_nong` | `on_hot_path` |
| `name_khach` | `customer_name` |
| `name_truong` | `field_name` |
| `near_nhat` | `nearest` |
| `nen_don` | `simple_candle` |
| `new_pos_value_khe` | `new_slot_index` |
| `ngan_xep` | `stack` |
| `ngoai` | `outside` |
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
| `nguong_vao` | `entry_threshold` |
| `nhan_dien` | `detect_pattern` |
| `nhan_khi_bi_kep` | `receive_when_sandwiched` |
| `nhan_neu_khong_bi_kep` | `receive_if_not_sandwiched` |
| `nhat_ky` | `log` |
| `nhiet_do_c` | `temp_c` |
| `nhieu` | `noise` |
| `nhieu_tat_dinh` | `deterministic_noise` |
| `nho_hon_hoac_bang` | `less_or_equal` |
| `nho_nhat` | `min` |
| `noi_dung` | `content` |
| `noi_use` | `content` |
| `nua_chu_ky` | `half_life` |
| `num_duong` | `positive_count` |
| `num_hieu` | `number` |
| `num_khe` | `slot_count` |
| `num_nhanh` | `branch_count` |
| `num_op_cong` | `add_op_count` |
| `num_truot` | `miss_count` |
| `old_cong` | `tools` |
| `on_dinh` | `is_stable` |
| `only_num_xor` | `leading_bit_diff` |
| `order_da_send` | `orders_sent` |
| `owned_tinh` | `attrs` |
| `part_cong` | `assign_unit` |
| `peek_dau` | `peek_front` |
| `phan_cong` | `assign_unit` |
| `phan_dau` | `header` |
| `phan_du` | `residuals` |
| `phan_giai` | `resolve` |
| `phan_thuong` | `block_reward` |
| `phan_tich` | `analyze` |
| `phan_tich_gop` | `coalescing_analysis` |
| `phan_tich_ngan_hang` | `bank_analysis` |
| `phan_tich_phan_ky` | `divergence_analysis` |
| `phan_tich_simd` | `simd_analysis` |
| `phan_tich_tong_nhieu_bien` | `analyze_multi_accumulator` |
| `phan_tram` | `percent` |
| `phan_vi` | `percentile` |
| `phat_hien_bat_thuong` | `detect_anomalies` |
| `phat_luc` | `sent_at` |
| `phat_show_enable_normal` | `detect_anomalies` |
| `phi_phan_van` | `fee_bps` |
| `phi_quyen` | `premium` |
| `phi_thuc_te` | `effective_gas_price` |
| `phi_toi_da` | `max_fee` |
| `phi_uu_tien` | `priority_fee` |
| `phi_uu_tien_toi_da` | `max_priority_fee` |
| `phien` | `session` |
| `phien_ban` | `version` |
| `phong_ve_tren` | `hedge_on` |
| `phuong_sai` | `variance` |
| `phuong_sai_uoc_luong` | `estimated_variance` |
| `phuong_thuc` | `method` |
| `pick_tuyen` | `match_route` |
| `point_num_goc` | `base_score` |
| `pop_dau` | `pop_front` |
| `pos_value_khe` | `slot_offset` |
| `push_dau` | `push_front` |
| `quy_open` | `scale` |
| `quyet_dinh` | `decide` |
| `r_binh_phuong` | `r_squared` |
| `ra_ns` | `out_nanos` |
| `ra_truoc` | `front_run_out` |
| `record_truong` | `write_field` |
| `rut_lien_mach` | `drain` |
| `san_ck` | `venue_chain` |
| `san_nhan_toi_thieu` | `min_amount_out` |
| `san_tt` | `venue_lit` |
| `sat_thuong_cham` | `contact_damage` |
| `sau_giam_gia` | `after_discount` |
| `sau_lenh` | `position_after` |
| `sau_phi` | `after_fee` |
| `schedule_su` | `history` |
| `sell_good_nhat` | `best_ask` |
| `show_thi` | `display` |
| `sinh_cap_gia` | `gen_price_pair` |
| `sinh_du_lieu` | `gen_data` |
| `sinh_loi_suat` | `gen_returns` |
| `sinh_mau_do_tre` | `gen_latency_samples` |
| `sinh_nen` | `gen_candle` |
| `sinh_phien` | `generate_session` |
| `sinh_phien_ghi` | `gen_recorded_session` |
| `so_bit_khong_dau` | `leading_zero_bits` |
| `so_buoc` | `num_steps` |
| `so_buoc_vat_ly` | `physics_steps` |
| `so_chu_ky` | `num_cycles` |
| `so_du` | `balance` |
| `so_du_ban_dau` | `initial_balance` |
| `so_du_doan_sai` | `mispredictions` |
| `so_du_moi` | `new_balance` |
| `so_duong` | `positive_count` |
| `so_gd` | `num_trades` / `num_transactions` |
| `so_giao_dich` | `num_trades` / `num_transactions` |
| `so_hang` | `num_rows` / `row_index` |
| `so_hieu` | `number` |
| `so_khe` | `slot_count` |
| `so_khoi` | `num_blocks` |
| `so_khop` | `fill_count` |
| `so_khung` | `num_frames` |
| `so_lan_ghi` | `write_count` |
| `so_lan_gui` | `send_count` |
| `so_lan_phong_ve` | `hedge_count` |
| `so_lan_tu_choi` | `reject_counts` |
| `so_lenh_bi_chan` | `orders_blocked` |
| `so_lenh_dang_mo` | `open_orders` |
| `so_lenh_gui` | `orders_sent` |
| `so_lenh_khop` | `filled_orders` |
| `so_lenh_qua` | `orders_passed` |
| `so_loi_chiu_duoc` | `fault_tolerance` |
| `so_loi_trang` | `page_faults` |
| `so_luong` | `quantity` |
| `so_luong_khong` | `invalid_quantity` |
| `so_luong_ve` | `ticket_count` |
| `so_mau` | `samples` |
| `so_may_chu` | `usable_hosts` |
| `so_muc` | `num_levels` |
| `so_muc_dang_dung` | `levels_in_use` |
| `so_nhanh` | `branch_count` |
| `so_phan_tu` | `num_elements` |
| `so_phan_tu_du` | `remainder_elements` |
| `so_phep_cong` | `add_op_count` |
| `so_phep_nhan` | `mul_op_count` |
| `so_phien_lai` | `winning_sessions` |
| `so_phien_lo` | `losing_sessions` |
| `so_phieu_thu_duoc` | `votes_received` |
| `so_su_kien` | `event_count` |
| `so_tai_khoan` | `account_number` |
| `so_thu_tu` | `seq` |
| `so_tiep` | `next_number` |
| `so_trung` | `hit_count` |
| `so_truot` | `miss_count` |
| `so_truy_cap` | `access_count` |
| `so_vong` | `num_rounds` |
| `so_warp_phan_ky` | `divergent_warps` |
| `so_y_dinh` | `intent_count` |
| `standard_hoa` | `normalize` |
| `su_kien` | `event` |
| `suc_chua` | `capacity` |
| `sut_giam_toi_da` | `max_drawdown` |
| `suy_kieu` | `infer_type` |
| `tai_in_ky` | `signing_payload` |
| `tai_khoan` | `account` |
| `tai_trong_ky` | `signing_payload` |
| `tam_tinh` | `subtotal` |
| `tan_suat_doi` | `change_frequency` |
| `tang_tb` | `up_avg` |
| `tang_toc_ly_thuyet` | `theoretical_speedup` |
| `tang_toc_toi_da_neu_xoa_nut` | `max_speedup_if_bottleneck_removed` |
| `tao_bo_loc_tu_cam` | `make_ban_filter` |
| `tat_ca` | `all` |
| `temp_tinh` | `subtotal` |
| `ten_cac_dinh` | `vertex_names` |
| `ten_dang_nhap` | `username` |
| `ten_hang` | `product_name` |
| `ten_khach` | `customer_name` |
| `ten_truong` | `field_name` |
| `tham_num` | `param` |
| `tham_num_path` | `path_params` |
| `tham_so` | `param` |
| `tham_so_duong_dan` | `path_params` |
| `thanh_bool` | `to_bool` |
| `thanh_dau_ra` | `into_output` |
| `thanh_html` | `to_html` |
| `thanh_thong_ke` | `to_stats` |
| `thanh_thuc` | `into_real` |
| `thanh_tien` | `line_total` |
| `thanh_toan` | `payment` |
| `thanh_toan_gio` | `checkout` |
| `thay_trang_fifo` | `fifo_replace` |
| `thay_trang_lru` | `lru_replace` |
| `them_canh` | `add_edge` |
| `them_dinh` | `add_vertex` |
| `them_hang` | `add_row` |
| `them_may_chu` | `add_server` |
| `them_quan_he` | `add_relation` |
| `them_thuc_the` | `add_entity` |
| `theo_khoi` | `chunked` / `blocked` |
| `thieu_hut_ky_vong` | `expected_shortfall` |
| `thoat_html` | `escape_html` |
| `thoi_diem` | `timestamp` |
| `thoi_diem_den` | `arrives_at` |
| `thoi_diem_ns` | `timestamp_nanos` |
| `thoi_diem_vao` | `entered_at` |
| `thoi_gian_ao_ns` | `virtual_time_nanos` |
| `thoi_gian_can` | `time_needed` |
| `thoi_gian_cho_thuc_ns` | `real_wait_nanos` |
| `thoi_gian_ms` | `time_ms` |
| `thoi_gian_nam` | `years` |
| `thong_ke_danh_muc` | `portfolio_stats` |
| `thu_gon_khoang_trang` | `collapse_whitespace` |
| `thu_gon_range_state` | `collapse_whitespace` |
| `thu_hoan_doi` | `try_swap` |
| `thu_hoan_doi_x_lay_y` | `try_swap_x_for_y` |
| `thu_hoan_doi_y_lay_x` | `try_swap_y_for_x` |
| `thu_swap` | `try_swap` |
| `thuc_hien_giao_dich` | `withdraw` |
| `thuc_show_trade` | `withdraw` |
| `thuc_te` | `actual` |
| `thuc_thi` | `execute` |
| `thuoc_tinh` | `attrs` |
| `tich_accum` | `accumulate` |
| `tich_accum_nanos` | `accumulated_nanos` |
| `tich_luy` | `accumulate` |
| `tich_luy_ns` | `accumulated_nanos` |
| `tick_sang_chuoi` | `tick_to_string` |
| `tien_to` | `prefix` |
| `tien_toi` | `advance` |
| `tien_trinh` | `process` |
| `tieu_de` | `title` |
| `tieu_diem` | `focus` |
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
| `tinh_tong_lat_cat` | `sum_slice` |
| `tinh_total_lat_cut` | `sum_slice` |
| `tk_an` | `main_account` |
| `toc_do` | `speed` |
| `toi_da_buoc_mot_khung` | `max_steps_per_frame` |
| `toi_thieu` | `min` |
| `toi_thieu_y` | `min_y` |
| `ton_that_tam_thoi` | `impermanent_loss` |
| `tong_binh_phuong` | `sum_of_squares` |
| `tong_chu_ky_cho` | `total_cycles_pipelined` |
| `tong_chu_ky_khong_ong` | `total_cycles_no_pipeline` |
| `tong_cung` | `total_supply` |
| `tong_khoi_luong` | `total_qty` |
| `tong_kiem_tra` | `checksum` |
| `tong_lai_lo` | `total_pnl` |
| `tong_luong` | `total_amount` |
| `tong_luong_truy_cap` | `total_traffic` |
| `tong_vao` | `total_in` |
| `total_binh_phuong` | `sum_of_squares` |
| `total_period_no_ong` | `total_cycles_no_pipeline` |
| `tra_loi` | `answer` |
| `trang` | `page` |
| `trang_1` | `page_1` |
| `trang_thai` | `page` |
| `tre_lenh` | `order_latency` |
| `treo_ban` | `resting_ask` |
| `treo_mua` | `resting_bid` |
| `trich_xuat` | `extract` |
| `trong` | `in` |
| `trong_so` | `weight` |
| `trong_tai` | `arbiter` |
| `tru_tien` | `debit` |
| `trung_binh` | `mean` |
| `trung_binh_ngoai_mau` | `out_of_sample_mean` |
| `trung_binh_trong_mau` | `in_sample_mean` |
| `trung_binh_truot` | `moving_average` |
| `truoc` | `prev` |
| `truot_bat_buoc` | `compulsory_miss` |
| `truot_do_capacity` | `capacity_miss` |
| `truot_do_dung_luong` | `capacity_miss` |
| `truot_enable_step` | `compulsory_miss` |
| `truot_gia` | `slippage` |
| `truy_cap` | `access` |
| `truy_cap_cot_lat` | `tile_column_access` |
| `truy_xuat_lan_toa` | `spread_retrieve` |
| `tt_ban` | `lit_ask` |
| `tt_mat_can_bang` | `lit_imbalance` |
| `tt_mua` | `lit_bid` |
| `tt_vi_gia` | `lit_micro_price` |
| `tu_bool` | `from_bool` |
| `tu_bool_` | `from_bool` |
| `tu_khop` | `from_fill` |
| `tu_so` | `numerator` |
| `tu_tam` | `from_center` |
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
| `vi_tri` | `location` / `index` / `position` |
| `vi_tri_doc` | `read_pos` |
| `vi_tri_ghi` | `write_pos` |
| `vi_tri_phan_tram` | `percent_b` |
| `vi_tri_trong_hang` | `queue_position` |
| `viet_hoa_chu_dau` | `capitalize_first` |
| `view_dem` | `counter_view` |
| `view_hoa_don_safe` | `view_invoice_safe` |
| `vong_dong_thuan` | `consensus_round` |
| `vuot_gia_tri` | `exceed_value` |
| `vuot_vi_the` | `exceed_position` |
| `warp_dong_thoi` | `concurrent_warps` |
| `xac_thuc` | `auth` |
| `xem_hoa_don_an_toan` | `view_invoice_safe` |
| `xep_lich_song_song` | `schedule_parallel` |
| `xu_ly` | `handle` |
| `xu_ly_co_ong` | `run_pipelined` |
| `xu_ly_don_ke_tiep` | `process_next_order` |
| `y_dinh` | `intent` |

---

### Bổ sung bản sửa tháng 10/2026

Đợt rà soát toàn bộ đã dịch nốt các định danh tiếng Việt còn sót và sửa các bản dịch máy sai nghĩa. Bảng dưới liệt kê theo phạm vi chương (biến cục bộ không liệt kê).

#### Chương 01–14

| Định danh trước bản sửa | Định danh hiện tại |
|---|---|
| `bmi` (ch05) | `compute_bmi` |
| `mark_price_state` (ch05) | `body_status` |
| `parse_float` (ch05) | `read_f32` |
| `consume_series` (ch06) | `consume_string` |
| `series_length` (ch07) | `string_length` |
| `SystemConfig::name_resp_use` (ch08) | `app_name` |
| `SystemConfig::phi_dich_vu` (ch08) | `monthly_fee` |
| `SystemConfig::lay_ten` (ch08) | `name` |
| `BankAccount::activate` (ch09) | `is_active` |
| `BankAccount::tra_cuu_thong_tin` (ch09) | `show_info` |
| `BankAccount::nap_tien` (ch09) | `deposit` |
| `BankAccount::rut_tien` (ch09) | `withdraw` |
| `BankAccount::all_math_and_round` (ch09) | `close_account` |
| `OrderStatus::DangDongGoi { store_export_queue }` (ch10) | `OrderStatus::Packing { warehouse }` |
| `OrderStatus::InTransit { ma_van_don, ten_tai_xe }` (ch10) | `{ tracking_code, driver_name }` |
| `OrderStatus::Delivered { time_time_recv }` (ch10) | `{ received_at }` |
| `PaymentError::InsufficientBalance { can_rut }` (ch11) | `{ requested }` |
| `check_num_tien` (ch11) | `parse_amount` |
| `execute_trade` (ch11) | `withdraw` |
| `mod thiet_bi_thong_minh` (ch12) | `mod smart_devices` |
| `mod trung_tam_dieu_khien` (ch12) | `mod control_center` |
| `Sensor::don_pos_do` (ch12) | `Sensor::unit` |
| `Sensor::check_computed_state` (ch12) | `Sensor::report_status` |
| `TempSensor::do_c` (ch12) | `celsius` |
| `SmokeSensor::khu_vuc` / `mat_do_khoi_ppm` (ch12) | `area` / `smoke_ppm` |
| `Item::ma_san_pham` (ch13) | `sku` |
| `to_money` (ch13) | `subtotal` |
| `apply_down_price` (ch13) | `apply_discount` |
| `xu_ly_menh_lenh` (ch13) | `process_imperative` |
| `handle_declaration` (ch13) | `process_declarative` |
| `ghep3` (ch14) | `compose3` |
| `queue_num` (ch14, bộ kết hợp const) | `constant` |
| `cat_bot` / `cat_bot_curry` (ch14) | `truncate` / `truncate_curried` |
| `tao_bo_che_tu_cam` (ch14) | `make_censor` |
| `LogRecord::ma_binh_luan` / `ket_luan` (ch14) | `comment_id` / `verdict` |
| `xep_loai` (ch04) | `classify` |
| `tong_so_chan` (ch04) | `sum_of_evens` |
| `tinh_chu_vi_dien_tich_hcn` (ch05) | `rectangle_perimeter_area` |
| `c_sang_f` (ch05) | `celsius_to_fahrenheit` |
| `tinh_do_dai` (ch06) | `calculate_length` |
| `them_loi_chuc` (ch07) | `add_wish` |
| `chon_chuoi_ngan_hon` (ch08) | `shorter` |
| `Parser::du_lieu_nguon` (ch08) | `source` |
| `HinhChuNhat { chieu_dai, chieu_rong }` (ch09) | `Rectangle { length, width }` |
| `tao_moi` / `tinh_dien_tich` / `tinh_chu_vi` / `co_phai_hinh_vuong` (ch09) | `new` / `area` / `perimeter` / `is_square` |
| `QueDiem { con_dau }`, `dot` (ch09) | `Match { has_head }`, `strike` |
| `Order::tinh_tong_thanh_toan` (ch09) | `Order::total` |
| `MauSac`, `Diem` (ch09) | `Color`, `Point` |
| `PaymentState::{ChuaTra, DaTra}` (ch10) | `{Unpaid, Paid}` |
| `Gender::{Nam, Nu, Khac}` (ch10) | `{Male, Female, Other}` |
| `PhepTinh::{Cong, Tru, Nhan, Chia}`, `tinh_toan` (ch10) | `Operation::{Add, Sub, Mul, Div}`, `calculate` |
| `doc_so_tu_chuoi` (ch11) | `parse_non_negative` |
| `AppError::{DocTep, PhanTichSo}` (ch11) | `AppError::{ReadFile, ParseNumber}` |
| `CoDienTich::tinh_dien_tich` (ch12) | `HasArea::area` |
| `HinhTron { ban_kinh }`, `HinhVuong { canh }`, `in_dien_tich` (ch12) | `Circle { radius }`, `Square { side }`, `print_area` |
| `mod quan_ly_kho`, `HangHoa { ten, gia }`, `moi` (ch12) | `mod inventory`, `Product { name, price }`, `new` |
| `mod ban_hang`, `xuat_hoa_don` (ch12) | `mod sales`, `make_invoice` |
| `in_du_lieu`, `ma_thiet_bi`, `so_sanh_he_thong` (ch12) | `print_value`, `device_id`, `compare_devices` |
| `chuan_hoa_ten` (ch13, đề bài) | `normalize_name` (khớp lời giải) |
| `chia` (ch13, ch14) | `divide` |
| `ghep` / `ghep4` (ch14 văn) | `compose` / `compose4` |
| `tao_kiem_tra_khoang`, `kiem_tra_diem` (ch14, đề bài) | `make_range_check`, `check_score` (khớp lời giải) |
| `tao_bo_nhan_sai` / `tao_bo_nhan_dung` (ch14) | `make_multiplier_wrong` / `make_multiplier` |
| `BO_DEM`, `tang_va_lay` (ch14) | `COUNTER`, `increment_and_get` |

#### Chương 15–24, Phụ lục A

| Định danh trước bản sửa | Định danh hiện tại |
|---|---|
| `exec_swap` | `exec_mutate` |
| `SensorRecord.ma_cam_bien` | `SensorRecord.sensor_id` |
| `DangerAlert.fold_records` | `DangerAlert.position` |
| `DangerAlert.level_do` | `DangerAlert.severity` |
| `Trade.khu_vuc` | `Trade.region` |
| `Trade.so_tien` | `Trade.amount` |
| `analyze_close` | `parse_trade` |
| `tong` / `tong_duoi` / `tong_lap` (snippet đệ quy) | `sum_rec` / `sum_tail` / `sum_loop` |
| `RawProfile.age_series` | `RawProfile.age_text` |
| `make_ban_filter` | `make_banned_word_filter` |
| `make_unit_check_do_long` | `make_length_checker` |
| `auth_proxy_num` | `validate_profile` |
| `ap_dung_fn` / `ap_dung_generic` / `tao_bo_nhan` | `apply_fn` / `apply_generic` / `make_multiplier` |
| `LoiCauHinh`, `phan_tich_cong` | `ConfigError`, `parse_port` |
| `Tong` | `Sum` |
| `MoiDeu` | `All` |
| `ThongKe` | `Stats` |
| `CuoiCung` (bài tập) | `LastWins` |
| `BangDem` (bài tập) | `CountTable` |
| `TrungBinh` (bài tập) | `NaiveMean` |
| `RunningMean.tong` / `.quantity` | `RunningMean.sum` / `.count` |
| `HKT::DichDen` | `HKT::Target` |
| `Validation::Set` / `Validation::Hong` | `Validation::Valid` / `Validation::Invalid` |
| `Validation::tu_ket_qua` / `is_set` | `Validation::from_result` / `is_valid` |
| `ghep2` / `ghep3` | `zip2` / `zip3` |
| `DonTho` | `RawForm` |
| `doc_ma_don` / `return_price` / `ap_thue` | `parse_order_id` / `lookup_price` / `apply_tax` |
| `doc_cau_hinh` (bài tập) | `read_config` |
| `bind_bang_map_roi_flatten` (test) | `bind_equals_map_then_flatten` |
| `mod mien` | `mod domain` |
| `DomainError::EmailSai` / `TenSanPhamSai` / `BadQuantity` / `DonRong` | `InvalidEmail` / `InvalidProductName` / `InvalidQuantity` / `EmptyOrder` |
| `OrderTooLarge { so_dong, toi_da }` | `OrderTooLarge { line_count, max }` |
| `TenSanPham` | `ProductName` |
| `Email::analyze` / `ProductName::analyze` / `Quantity::analyze` | `::parse` |
| `TOI_DA` / `SO_DONG_TOI_DA` | `MAX` / `MAX_LINES` |
| `Money::dong` / `Money::gate` / `Money::nhan` | `Money::vnd` / `Money::plus` / `Money::times` |
| `PaymentMethod::TienMat` / `PaymentMethod::The` | `PaymentMethod::Cash` / `PaymentMethod::Card` |
| `Authenticated` (typestate) | `Validated` |
| `Order::auth` / `Order::payment` / `Order::delivery_queue` | `Order::validate` / `Order::pay` / `Order::ship` |
| `Order.dong` / `so_dong()` / `tong_tien()` | `Order.lines` / `line_count()` / `total()` |
| `OrderDto.dong` | `OrderDto.lines` |
| `apply_discount` | `discount_for` |
| `Invoice.phi_van_transfer` | `Invoice.shipping` |
| `invoice_loop` | `build_invoice` |
| `PaymentState::ChuaTra` / `DaTra` | `Unpaid` / `Paid` |
| `mod lien_lac`, `VnPhone::analyze` (bài tập) | `mod contact`, `VnPhone::parse` |
| `Account::ChoKichHoat` / `DangHoatDong` / `BiKhoa`, `Account::key` | `PendingActivation` / `Active` / `Locked`, `Account::lock` |
| `ChuaKetNoi` / `DaKetNoi` / `TrongGiaoDich` | `Disconnected` / `Connected` / `InTransaction` |
| `start_trade` / `truy_van` / `order_log` | `begin_transaction` / `query` / `statement_log` |
| `tao_ban_do!` / `kiem_toan_bien!` / `do_luong_thoi_gian!` | `hash_map!` / `inspect_var!` / `measure_time!` |
| `in_gap_doi!` (ví dụ lỗi) | `print_double!` |
| `phep_tinh_noi_bo!` / `tao_ma_tran!` / `tinh_bieu_thuc_chuoi!` | `internal_calc!` / `matrix!` / `eval_chain!` |
| `dem_phan_tu!` / `in_tung_the!` / `in_sai_lap!` / `in_dung_lap!` | `count_tts!` / `print_each!` / `print_wrong!` / `print_right!` |
| `DetailedDescription::in_thong_tin_chi_tiet` | `describe` (+ `field_names`, `print_details`) |
| `NetworkDevice.dang_hoat_dong` | `NetworkDevice.is_active` |
| `#[derive(MoTa)]`, `XuatFileJson` (văn) | `#[derive(DetailedDescription)]`, `ToJson` |
| `SecurityAudit::export_thong_info_safe` / `id_part_kind` | `audit_fields` / `entity_kind` |
| `BankAccount.ma_pin_bi_mat` | `BankAccount.pin_code` |
| `#[bo_qua]` | `#[audit(skip)]`, `#[audit(sensitive)]` |
| `#[kiem_soat_truy_cap]` | `#[require_role(...)]` |
| `phan_tich_cau_hinh!` (macro_rules) | `config!` (proc macro thật) |
| `XuatDuLieu`, `KiemTra`, `InThongTin` (văn) | `SecurityAudit`, `Inspect` |
| `Setoid::bang` / `Ord::so_sanh` / `Group::nghich_dao` | `equals` / `compare` / `invert` |
| `Tong` / `Ham` | `Sum` / `Func` |
| `HKT::DichDen` / `Bifunctor::Ra` / `Extract::Ruot` | `Target` / `Output` / `Inner` |
| `Plus::rong` / `Foldable::gap` / `Chain::concat` / `Extend::mo_rong` | `zero` / `fold` / `chain` / `extend` |
| `Cay` / `Cay::La` / `Cay::Nut` | `Tree` / `Tree::Leaf` / `Tree::Node` |
| `ap_auth` / `Validation::Set` / `Validation::Hong` | `ap_validation` / `Valid` / `Invalid` |
| `mod luat` (tests) | `mod laws` |

#### Chương 25–42

| Định danh trước bản sửa | Định danh hiện tại |
|---|---|
| `chi_so` (tham số `index_access_o1`, ch25) | `index` |
| `hai_ban_cho_cung_ket_qua` (test, ch25 lời giải) | `both_versions_agree` |
| `tong_lat_cat` (test ch26) | `sum_of_slices` |
| `max_of_hoat_dong_voi_moi_nguon` (test ch26 lời giải) | `max_of_works_for_every_source` |
| `dem_lan_doi_dia_chi` (ch26 lời giải) | `count_reallocations` (viết lại) |
| `Nut` (ch27) | `Node` |
| `NutLoi` (ch27) | `BrokenNode` |
| `LinkedList::peak` (trường, ch27) | `head` |
| `dem_phan_tu_thu_cong` (ch27 lời giải) | `count_manually` |
| `chua_phan_tu` (ch27 lời giải) | `contains` |
| `dem_thu_cong_khop_voi_len`, `tim_thay_va_khong_tim_thay` (test ch27) | `manual_count_matches_len`, `found_and_not_found` |
| `peek_broken`/`peek_correct` (ch27 ví dụ E0507) | `take_broken`/`take_correct` |
| `Order::tong_tien` (ch28) | `total_amount` |
| `OrderQueue::them_don` / `them_don_vip` | `add_order` / `add_vip_order` |
| `OrderQueue::first_view_don` | `peek_next_order` |
| `OrderQueue::so_don_dang_cho` | `pending_count` |
| `KeyBuffer::go` / `noi_dung` (ch28 lời giải) | `type_key` / `contents` |
| `QueueTuHaiStack` (trường `vao`/`ra`) | `TwoStackQueue` (`inbox`/`outbox`) |
| `NutCay` (ch29) | `TreeNode` |
| `BinarySearchTree::them` | `insert` |
| `BinarySearchTree::quantity` (trường) | `size` |
| `TreeNode::gap` (tham số `block_make`) | `fold` (`init`) |
| `ToaDo` (ch29 ví dụ E0277) | `Coord` |
| `thong_ke_from_region` (ch30) | `word_frequencies` |
| `part_region` (ch30) | `partition` |
| `Graph::lay_ten` | `vertex_name` |
| `BinaryStore::record_sell_record` (ch31) | `append_record` |
| `SlottedPage::add_sell_record` (ch32) | `insert_record` |
| `SlottedPage::read_sell_record` / `read_sell_record_live` | `read_record` / `read_live_record` |
| `SlottedPage::tail_pointer` / `set_tail_pointer` | `free_space_pointer` / `set_free_space_pointer` |
| `SlottedPage::nearest_slot` | `set_slot_count` |
| `BufferPool::num_state_show_has` | `page_count` |
| `SUC_CHUA_NUT` (ch33) | `NODE_CAPACITY` |
| `BPlusNode::Internal.children: Vec<Box<BPlusNode>>` | `Vec<NodeId>` (thiết kế lại, thêm `Leaf.next_leaf`) |
| `BPlusTree::get_range` | `range` |
| `BPlusTree.total_records` (trường pub) | `len` (riêng tư, có `len()`) |
| `NutDemo`, `lay_con_dung` (ch33 ví dụ E0507) | `DemoNode`, `child_correct` |
| `kiem_khoa_tang_dan`, test `moi_nut_la_deu_giu_thu_tu_tang_dan` (ch33 BT2) | `keys_strictly_increasing`, `every_node_keeps_keys_sorted` |
| `doc_dong_dung` (ch34 ví dụ E0599) | `read_lines_correct` |
| `MvccStore::start_trade` (ch35) | `begin` (trả `Transaction`) |
| `MvccStore::record` / `doc` | `write` / `read` (nhận `&Transaction`) |
| `MvccStore::don_dep_rac(oldest_active_tx)` | `vacuum()` |
| `MiniBitcask.file_path: String` (ch36) | `PathBuf` |
| `doc_dung` (ch36 ví dụ E0502) | `read_correct` |
| `vi_du_dung_e0716` (ch37) | `e0716_correct` |
| `do_khung_ngan_xep` (ch37 BT1) | `measure_stack_frame` |
| `lay_4_byte` (ch38 BT2) | `take_4_bytes` |
| `SafeRingBuffer.o` / `.quantity` (ch38 BT1) | `buf` / `count` |
| `vi_du_dung_e0133` (ch39) | `e0133_correct` |
| `vi_du_dung_e0382` (ch40) | `e0382_correct` |
| `vi_du_dung_e0507` / `e0507_broken` (ch41) | `e0308_correct` / `e0308_broken` |
| `ErrorLevel` / `CapBacDung` (biến thể `NhanVien`, `GiamDoc`) (ch42 ví dụ lỗi) | `RankBroken` / `Rank` (`Staff`, `Director`) |
| `LoginRateLimiter::ghi_nhan_that_bai` / `dang_bi_khoa`, trường `theo_doi` (ch42 BT1) | `record_failure` / `is_locked`, `tracked` |
| `sinh_token_bi_mat` (ch42 BT2) | `generate_secret_token` |

#### Chương 43–60

| Định danh trước bản sửa | Định danh hiện tại |
|---|---|
| `MemoryInventory::kho` (ch43 lời giải BT2) | `MemoryInventory::stock` |
| `hop_dong_kho_hoat_dong` (test, ch43) | `inventory_contract_works` |
| `ValidationError::ZeroOrNegativeAmount` (ch45) | `ValidationError::ZeroAmount` |
| `doc_danh_sach_cong` (ch44 prompt mẫu) | `read_port_list` |
| `HttpMethod::Other(String)` (ch47) | `HttpMethod::Other` |
| `LogAnalyzer::load_from_raw_text` (ch47) | `LogAnalyzer::load_from_reader` + `ingest_line` |
| `xuat_json_hop_le`, `phan_tich_an_toan_khong_sap` (ch47 test lời giải) | `json_output_is_valid`, `parse_is_safe_on_garbage` |
| `xu_ly_generic`, `goi_dich_vu`, `DichVuDung`, `goi_dich_vu_dung` (ch48) | `process_generic`, `call_service`, `CorrectService`, `call_correct_service` |
| `FallbackCache::ghi_nho`, `tra_du_phong` (ch48 BT1) | `remember`, `lookup_fallback` |
| `LaFuture` (ch49) | `RealFuture` |
| `tim_nguoi_dung`, `tim_don_hang`, `invoice_loop`, `LoiHeThong` (ch49 đoạn async) | `find_user`, `find_orders`, `build_invoice`, `SystemError` |
| `AsyncInterval` trường `con_lai`, `chu_ky`, `nhip_ke`, `da_phat` (ch49 BT1) | `remaining`, `period`, `next_tick`, `emitted` |
| `JoinTwo` trường `kq_a`, `kq_b` (ch49 BT2) | `out_a`, `out_b` |
| `tao_san_pham` (ch51) | `create_product` |
| `StdError { chi_tiet }`, `SystemError { greeting }` (ch51 ví dụ lỗi) | `SystemError { message }` + `impl IntoResponse` |
| `RateLimitLayer` trường `gioi_han`, `cua_so` (ch51 BT2) | `limit`, `window` |
| `RaftNode::check_and_commit` (ch53) | `record_replication` + `advance_commit_index` |
| `RaftNode::handle_request_vote(id, term)` (ch53) | `handle_request_vote(id, term, last_log_index, last_log_term)` |
| `ViDuNode` (ch53) | `DemoNode` |
| `co_du_quorum`, `so_phieu`, `tong_cum` (ch53 BT1) | `has_quorum`, `votes`, `cluster_size` |
| `apply_sync(.., tu_vi_tri, bu)` (ch53 BT2) | `apply_sync(.., prev_index, entries)` |
| `LruCache` trường `suc_chua`, `dong_ho` (ch52 BT1) | `capacity`, `clock` |
| `Message::so_lan_thu`, tham số `thanh_cong` (ch52 BT2) | `attempts`, `succeeded` |
| `OrderStatus::from_str` (ch54, phương thức riêng) | `impl FromStr for OrderStatus` |
| `xu_ly_dung`, tham số `dh` (ch54 ví dụ E0382) | `handle_fixed`, `order` |
| `huy_don_paid_hoan_kho_va_ghi_wal`, `nen_wal_giu_trang_thai_cuoi` (ch54 test lời giải) | `cancel_paid_order_restores_stock_and_logs`, `compaction_keeps_latest_state_only` |
| `Cart::them`, `tong_tien`, `so_dong` (ch55) | `Cart::add`, `total`, `line_count` |
| `Generator::moi`/`new(hat)`, `Generator::so(tran)` (ch55) | `Generator::new(seed)`, `Generator::below(bound)` |
| `CongGia` (ch55 tests/integration.rs) | `FakeGateway` |
| `DongHo`, `DongHoGia`, `ma_don_hang` (ch55 BT2) | `Clock`, `FakeClock` (+`SystemClock`), `order_code` |
| `Cart::xoa`, `CartError::KhongTonTai` (ch55 đề BT1) | `Cart::remove`, `CartError::NotFound` |
| `GiayPhep`, `giay_phep_con_lai`, `toi_da`, `thu_vao`, `cho_trong` (ch48 BT2) | `Permit`, `available_permits`, `max_permits`, `try_acquire`, `available` |

#### Chương 61–73

| Định danh trước bản sửa | Định danh hiện tại |
|---|---|
| `Method::{GET,POST,PUT,DELETE}` | `Method::{Get,Post,Put,Delete}` |
| `Request.than` / `Response.than` | `body` |
| `Request.path_param` | `path_params` |
| `Response.id` | `status` |
| `Response::tao` | `Response::created` |
| `Response::not_seen` | `Response::not_found` |
| `Response::bad_data` | `Response::unprocessable` (+ mới `bad_request` 400, `method_not_allowed` 405) |
| `Route.mau` / `Route.handle` | `pattern` / `handler` |
| `Router.route` (trường) / `Router::them` | `routes` / `Router::route` |
| `Router::fill` | `Router::find` + `match_path` |
| `State` | `AppState` (tránh đè `axum::extract::State`) |
| `SanPham` | `Product` |
| `analyze_than` | `parse_body` |
| `xu_ly_liet_ke` / `handle_view_one` / `handle_make` / `handle_remove` | `handle_list` / `handle_get_one` / `handle_create` / `handle_delete` |
| `use_resp_use` | `build_router` |
| `yc` (hàm dựng request) | `request` |
| `Signal::lay` / `Derived::lay` | `get` |
| `VirtualNode::The {name, attribute, con}` | `VirtualNode::Element {tag, attrs, children}` |
| `VirtualNode::Van` | `VirtualNode::Text` |
| `VirtualNode::the` / `van` | `element` / `text` |
| `Patch::Replaced{nut_moi}` / `TextChanged{van_moi}` | `node` / `text` |
| `Patch::ThemCon{nut}` | `Patch::ChildAdded{node}` |
| `Patch::ChildRemoved{chi_so}` | `index` |
| `CounterState.so` | `count` |
| `Filter::{TatCa,ChuaXong,DaXong}` | `Filter::{All,Active,Completed}` |
| `Message::BatTat` | `Message::Toggle` |
| `Model::display` | `Model::visible` |
| `IpcBridge.order` | `commands` |
| `ProcessState::{Moi,SanSang,DangChay,Cho}` | `{New,Ready,Running,Waiting}` |
| `Process.uu_tien` | `priority` |
| `Process::time_time_wait` | `waiting_time` |
| `KetQuaLapLich` / `.process` | `ScheduleResult` / `.processes` |
| `tong_ket` | `summarize` |
| `lap_lich_fcfs` / `lap_lich_sjf` / `lap_lich_round_robin` | `schedule_fcfs` / `schedule_sjf` / `schedule_round_robin` |
| `ReplacementResult.series_frame` | `frame_history` |
| `WaitForGraph.edge` / `them_cho` | `edges` / `add_wait` |
| `Tang::{VatLy,LienKet,Mang,...}` | `Layer::{Physical,DataLink,Network,Transport,Application}` |
| `GoiTin{tang,header,tai}` / `boc` | `Packet{layer,header,payload}` / `wrap` |
| `dong_goi_xuong` | `encapsulate` |
| `TcpState::{Dong,Nghe,DaNhanSyn,DaThietLap,ChoDong1,ChoDong2}` | `{Closed,Listen,SynReceived,Established,FinWait1,FinWait2}`; **sửa bug**: `LastAck`/`TimeWait` bị gán nhãn đảo; thêm `Closing` |
| `TcpEvent::{NhanSyn,NhanSynAck,NhanAck,NhanFin,HetGio}` | `{RecvSyn,RecvSynAck,RecvAck,RecvFin,Timeout}` |
| `transfer_state` | `transition` |
| `CongestionPhase::{KhoiDongCham,TranhTacNghen}` | `{SlowStart,CongestionAvoidance}` |
| `CongestionControl.threshold` / `.pha` | `ssthresh` / `phase` |
| `nhan_ack` / `mat_call_light` / `het_gio` | `on_ack` / `on_triple_dup_ack` / `on_timeout` |
| `MangCon` / `analyze` / `mat_na` / `quang_ba` / `num_servers` | `Subnet` / `parse` / `mask` / `broadcast` / `usable_hosts` |
| `TransferResult.da_nhan` | `delivered` |
| `FakeRegisters::{record,doc,dao_bit,record_field}`, trường `so_lan_doc` | `write`, `read`, `toggle_bit`, `write_field`, `read_count` |
| `Pin<CheDo>{serial,_che_do}` / `serial()` | `Pin<Mode>{number,_mode}` / `number()` |
| `into_wall` | `into_analog` |
| `bat`/`tat`/`dao` (Output), `doc` (Input) | `set_high`/`set_low`/`toggle`, `is_high` (khớp embedded-hal 1.0) |
| `Peripherals{gate_a,gate_b}` / `lay()` / `DA_LAY` | `{pa5,pc13}` / `take()` / `TAKEN` |
| `Q16::{MOT,tu_nguyen,gate,subtract,nhan,chia}` | `ONE`, `from_int` (const), và trait `Add/Sub/Mul/Div`; `from_real` thành `const fn` |
| `adc_sang_nhiet_do` | `adc_to_celsius` |
| `RingBuffer::{quantity,rong,day,overwrite_buffer}` | `len`, `is_empty`, `is_full`, `push_overwrite` |
| `ChongRung` | `Debouncer` |
| `Signal::{Cao,KhongXacDinh}` | `Signal::{High,X}` |
| `unit_pick` | `mux2` |
| `AdderResult{tong,tran}` | `{sum,overflow}` |
| `suon_len` (FlipFlopD/ShiftRegister/đèn) | `rising_edge` |
| `ShiftRegister.o` / `doc` | `flops` / `contents` |
| `TrafficLight::{Do,DoVang,Xanh,Vang}` | `{Red,RedAmber,Green,Amber}` |
| `LedController{time_amount}` / `transfer_hop_le` | `TrafficController{durations}` / `is_legal_transition` |
| `handle_without_pipeline` / `handle_with_pipeline` | `run_unpipelined` / `run_pipelined` (độ trễ nay được ĐO, không gán cứng) |
| `Nut::{Low,Va,Hoac}` | `Node::{Not,And,Or}` |
| `Circuit.nut` / `them` / `open_bucket` | `nodes` / `add` / `simulate` |
| `Vec2::{KHONG,gate,subtract,nhan,part_remote}` | `ZERO`, trait `Add/Sub/Mul<f32>/Neg`, `reflect` |
| `Accumulator{accumulate,max_step_one_frame}::new_frame` | `{accumulated,max_steps_per_frame}::advance` (cả `IntegerAccumulator`) |
| `PhysicsBody.quantity` | `mass` |
| `Aabb::{tam,day_ra}` | `center`, `min_translation` |
| `intersect_merge` | `circles_intersect` |
| `LuoiBam{o}::{build_use,suspicious_pairs}` | `SpatialHash{cells}::{rebuild,candidate_pairs}` |
| `va_cham_vet_can` / `va_cham_qua_luoi` | `brute_force_pairs` / `grid_pairs` |
| `World{con_song,mau,ban_kinh}::{tao,cancel}` | `{alive,health,radius}::{spawn,despawn}` |
| `he_thong_move` | `movement_system` |
| `DangSoan` | `Draft` |
| `Order.id_chain` | `symbol` |
| `Side::{inverse_lai,first}` | `opposite`, `sign` |
| `RiskError::{ExceedsMaxValue,ExceedsMaxPosition}.tran` | `.limit` |
| `OrderBook.ben_ban` / `nap` | `sell_side` / `submit` |
| `Position.tien_mat` / `RONG` | `cash` / `EMPTY` |
| `Candle.{mo,dong}` | `open`, `close` |
| `Signal::Giu` | `Signal::Hold` |
| `MeanCross.cham` | `slow` |
| `HANG_SO_K` | `K` |
| `Bam` / `KHONG` / `rut_gon` | `Hash256` / `ZERO` / `short` |
| `OutPoint.chi_so` | `index` |
| `Transaction::{tao_tien,la_tao_tien}` | `coinbase`, `is_coinbase` (+ mới `spend`, trường `coinbase_height`) |
| `TapUtxo{o}` | `UtxoSet{outputs}` |
| `TransactionError::SpendsMoreThanReceives{ra}` | `{total_out}` |
| `BlockHeader.so_ngau_nhien` / `Block::dao` | `nonce` / `mine` |
| `BlockError::BelowDifficulty{set,can}` / `CoinbaseOverpays{lay,can}` | `{got,need}` / `{claimed,allowed→need}` |
| `Chain.peak` / `peak_height` / `utxo_tai` / `them` | `tip` / `tip_height` / `utxo_at` / `add_block` |
| `MaNut` | `NodeId` |
| `RoutingTable{toi,xor}::{them,tong_so_nut}` | `{own_id,buckets}::{insert,known_nodes}` |
| `KetQuaTraCuu{so_nut_da_hoi}` | `LookupResult{nodes_queried}` |
| `SimNetwork{nut}::{dung,tra_cuu}` | `{nodes}::{build,lookup}` |
| `PropagationResult{so_nut_nhan,so_ban_tin,fully_parallel}` | `{nodes_reached,messages_sent,full_coverage}` |
| `Behavior::{Im,HaiMat}` | `{Silent,Equivocating}` |
| `RoundResult.threshold_can` | `threshold` |
| `DistributedHashTable{mang,he_so_nhan_ban}::{lay,nut_roi_mang}` | `{network,replication}::{get,leave}` |
| `ThongTinGoi` | `MessageInfo` (khớp tên cosmwasm-std) |
| `Store{o}::lay` | `Store{kv}::get` |
| `ContractError::{InsufficientFunds{can,co},Forbidden{ai},TranSo}` | `{InsufficientFunds{needed,available},Forbidden{addr},Overflow}` |
| `Response{event,thong_message_cont}::send_cont` | `{events,messages}::add_message` |
| `TokenMsg::{Transfer{den},Dot,TransferFrom{tu,den}}` | `{Transfer{recipient},Burn,TransferFrom{owner,recipient}}` |
| `TokenCw20::{gate,subtract}` | `add_balance`, `sub_balance` |
| `EscrowState::{DangGiu,DaGiaiNgan,DaHoanTien}`, `Escrow.so_tien` | `{Holding,Released,Refunded}`, `amount` |
| `derive_pda(hat_giong, ma_chuong_trinh)` | `derive_pda(seeds, program_id)` |
| `CheckAccount::{must_ky,must_owned_own,must_record_can}` | `{must_be_signer,must_be_owned_by,must_be_writable}` |
| `MA_CHUONG_TRINH` / `CounterProgram::tang` | `COUNTER_PROGRAM_ID` / `increment` |
| `SolanaError::InsufficientLamports{can,co}` | `{needed,available}` |
| `ParallelAnalysis{so_lo_song_song,lo}` | `{batch_count,batches}` |
| `AbiValue::MangUint` / `la_dong` | `UintArray` / `is_dynamic` |
| `dung_calldata` / `doc_uint` | `build_calldata` / `read_uint` |
| `Rlp::{Text,DanhSach}` / `Rlp::numerator` | `Rlp::{Bytes,List}` / `Rlp::uint` |
| `Tx1559.den` | `to` |
| `Tx1559::{load_in_period,id_hash_ky,chi_phi_toi_da}` | `{signing_payload,signing_hash,max_cost}` |
| `Tx1559::effective_fee(u128) -> u128` | `effective_gas_price(u128) -> Option<u128>` (None khi base fee > max_fee — sửa bug vượt trần, có test) |
| `Erc20::{CK_CHUYEN,CK_SO_DU,CK_CHO_PHEP,SK_CHUYEN}` | `{TRANSFER_SIG,BALANCE_OF_SIG,APPROVE_SIG,TRANSFER_EVENT}` |

#### Chương 74–85

| Định danh trước bản sửa | Định danh hiện tại |
|---|---|
| `LatencyHistogram::tong_mau` | `count` |
| `LatencyHistogram::xor` (xô) | `buckets` |
| `LatencyHistogram::total_value` | `sum` |
| `tom_tat` | `summary` |
| `DONG_CACHE` | `CACHE_LINE` |
| `DisruptorRing::o` / `ObjectPool::o` | `slots` |
| `chi_so` | `slot` |
| `rong` / `day` / `quantity` (vòng) | `is_empty` / `is_full` / `len` |
| `lay_lo` | `take_batch` |
| `ObjectPool::{borrow, tra, view, fix}` | `acquire`, `release`, `get`, `get_mut` |
| `ObjectPool::{ranh, con_ranh, count_borrow, so_lan_het_be}` | `free`, `available`, `acquire_count`, `exhausted_count` |
| `QuoteAoS::co` / `count` | `flags` / `_reserved` |
| `OrderPacket::count` | `_reserved` |
| `total_price_buy` | `total_bid_price` |
| `LatencyBudget::{chang, tran_ns, tong, set_level_spend, nut_that_co_chai, max_speedup_if_node_removed}` | `stages`, `budget_ns`, `total`, `within_budget`, `bottleneck`, `max_speedup_if_bottleneck_removed` |
| `BanTin` / trường `ban_tin` | `Message` / `message` |
| `Message::Replaced::ma_moi` | `new_id` |
| `ParseError::TooShort { can, co }` | `TooShort { needed, got }` |
| `KetQuaNhan` | `SeqOutcome` |
| `MissingMessages { tu, den, so_ban_tin_mat }` | `MissingMessages { from, to, count }` |
| `GapDetector::{nhan, slot_loop, num_dang_count}` | `receive`, `fill_gap`, `buffered_len` |
| `GapDetector::{slot_count, num_duplicate_loop, tong_ban_tin_mat}` | `gap_count`, `duplicate_count`, `total_lost` |
| `PriceLevel::so_lenh` | `order_count` |
| `L2Book::{buy, ban}` | `bids`, `asks` |
| `L2Book::{them, bot, price_can_table, is_key}` | `add`, `reduce`, `micro_price`, `is_locked` |
| `L3Book::order` | `orders` |
| `Chieu2` (bỏ hẳn) | `Side` nay derive `Ord` |
| `ReadError::{DoDaiVoLy, MaSuKienLa}` | `InvalidLength`, `UnknownEventCode` |
| `SessionRecorder::{so_byte, doc_lai, time_amount_nanos}` | `byte_len`, `read_back`, `duration_nanos` |
| `VirtualClock::{bay_gio_ns, adder_gate}` | `now_ns`, `advance_by` |
| `ReplaySpeed::HeSo` | `ReplaySpeed::Factor` |
| `LatencyModel::qua_internet` | `over_internet` |
| `ReducedBook::{buy, ban, them, bot, quantity}` | `bids`, `asks`, `add`, `reduce`, `qty_at` |
| `OurOrder::{timestamp_toi_venue_nanos, fill_done}` | `arrives_at_venue_nanos`, `is_filled` |
| `Position::tien_mat` | `cash` |
| `ReplayStrategy::when_can_fill` | `on_fill` |
| `ManagedMaker::is_pending` | `pending_exposure` |
| `Side::first` | `Side::sign` |
| `RejectReason::NgonTayBeo { lech_percent }` | `FatFinger { deviation_pct }` |
| `RiskLimits::so_lenh_moi_giay_toi_da` | `max_orders_per_second` |
| `RiskGate::{da_tat, enable_all_switches, operator_flips_switch, check_join_unit}` | `is_killed`, `trip_kill_switch`, `operator_reset`, `check_inner` |
| `StatsWindow::{o, tong, them, quantity, day, diem_z}` | `values`, `total`, `push`, `len`, `is_full`, `z_score` |
| `PairSignal::{MoDaiB, Dong, KhongLam}` | `OpenLongB`, `Close`, `Hold` |
| `ArbCap` / `proxy_ratio`, `threshold_out`, `threshold_use` | `PairArb` / `hedge_ratio`, `exit_threshold`, `stop_threshold` |
| `sort_arrange_block` | `order_block` |
| `KetQuaKep` | `SandwichOutcome` |
| `NS_MOI_CHU_KY`, `DAI_GOI`, `SO_MUC_PHAN_CUNG` | `NS_PER_CYCLE`, `PACKET_LEN`, `HW_LEVELS` |
| `PacketField::id_chain` | `symbol_id` |
| `FieldExtractor::{tach, period_split, so_goi_da_tach, so_goi_hong}` | `extract`, `extract_cycles`, `packets_parsed`, `packets_rejected` |
| `xor_tuan_tu` | `xor_sequential` |
| `HwOrderBook::{buy, ban}` | `bids`, `asks` |
| `RejectFlags::num_has_enable` | `count_raised` |
| `RiskCircuit::period_check` | `check_cycles` |
| `PipelineStage::period` / `HwPipeline::tang` | `cycles` / `stages` |
| `HwPipeline::{latency_period, total_period_wait}` | `latency_cycles`, `total_cycles_pipelined` |
| `partial_sum` / `ExecutionUnit::PhanMem` | `assign_unit` / `ExecutionUnit::Software` |
| `MemoryTier::DiaQuay` / `period()` | `SpinningDisk` / `cycles()` |
| `BYTE_MOI_DONG_CACHE`, `PHAT_DU_DOAN_SAI` | `CACHE_LINE_BYTES`, `MISPREDICT_PENALTY` |
| `CacheStats::{slip_count, ratio_duplicate, total_period}` | `miss_count`, `hit_ratio`, `total_cycles` |
| `CacheSim::{so_tap, positive_count, tap, account, access_cap}` | `num_sets`, `ways`, `sets`, `stats`, `access` |
| `BranchPredictor::ratio_sai` | `mispredict_ratio` |
| `analyze_total_one_bien`, `tong_mot_bien`, `total_many_bien` | `analyze_single_accumulator`, `sum_single`, `sum_multi` |
| `SimdAnalysis::{be_rong_vector, so_lenh_vector}` | `vector_width`, `vector_ops` |
| `LUONG_MOI_WARP`, `BYTE_MOI_GIAO_DICH`, `SO_NGAN_HANG` | `THREADS_PER_WARP`, `TRANSACTION_BYTES`, `NUM_BANKS` |
| `LaunchConfig::{total_amount, warp_moi_khoi, excess_flow}` | `total_threads`, `warps_per_block`, `idle_threads` |
| `DivergenceAnalysis::{so_warp, he_so_cham}` | `num_warps`, `slowdown` |
| `CoalescingAnalysis::byte_co_ich` | `useful_bytes` |
| `access_cap_col_lat_has_count` | `padded_tile_column_access` |
| `KetQuaRutGon` / `tong` | `ReductionResult` / `sum` |
| `rut_gon_song_song`, `rut_gon_tuan_tu`, `num_step_reduce` | `parallel_reduce`, `sequential_reduce`, `reduce_steps` |
| `GemmAnalysis::doc_chia_se` | `read_shared` |
| `Occupancy::warp_toi_da` | `max_warps` |
| `Candle::{mo, dong, quantity}` | `open`, `close`, `volume` |
| `Candle::{than, bien_do, tang, down}` | `body`, `range`, `is_bullish`, `is_bearish` |
| `Pattern::{BuaTang, SaoBangGiam, NhanChimTang, NhanChimGiam, KhongCo}` | `Hammer`, `ShootingStar`, `BullishEngulfing`, `BearishEngulfing`, `NoPattern` |
| `la_doji`, `la_bua`, `la_sao_bang`, `la_nhan_chim_tang` | `is_doji`, `is_hammer`, `is_shooting_star`, `is_bullish_engulfing` |
| `BollingerBands::{above, mid, below, do_rong}` | `upper`, `middle`, `lower`, `bandwidth` |
| `bien_do_that` | `true_range` |
| `co_theo_atr` | `atr_position_size` |
| `OptionParams::bien_dong` | `vol` |
| `gia_black_scholes` | `black_scholes_price` |
| `LegKind::{QuyenBan, TaiSanCoSo}` | `Put`, `Underlying` |
| `spread_price_up` | `bull_call_spread` |
| `dieu_hau_sat` ("Điều hâu sắt") | `iron_condor` ("Iron condor") |
| `OptionStrategy::{lai_max_in_long, lo_max_in_long}` | `max_profit_in_range`, `max_loss_in_range` |
| `RegressionResult::{sai_num_standard, so_quan_sat}` | `residual_std`, `num_obs` |
| `KetQuaDongLienKet` | `CointegrationResult` |
| `TestSegment::{query_param, point_in_mau, point_out_mau}` | `chosen_param`, `in_sample_score`, `out_of_sample_score` |
| `sinh_cap_dong_lien_ket` | `gen_cointegrated_pair` |
| `gen_cap_price_cointegration` (tên sai nghĩa: cặp này KHÔNG đồng liên kết) | `gen_correlated_only_pair` |
| `Side::first` | `Side::sign` |
| `Intent::block_don` | `Intent::place` |
| `OurOrder::prev_quantity` | `queue_ahead` |
| `MarketSnapshot::{lit_buy, lit_sell, mid_price_traditional}` | `lit_bid`, `lit_ask`, `lit_mid` |
| `LitVenue::{buy, ban, ben, them, bot}` | `bids`, `asks`, `side_map`, `add`, `reduce` |
| `BE_KHOI_DAU` | `INITIAL_POOL` |

## Ghi chú về cách dịch