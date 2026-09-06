# Chương 06: Trọng tâm Rust: Quy tắc Sở hữu & Cơ chế Di chuyển (The Core of Rust: Ownership Rules & Move Semantics)

## Giới thiệu & Mục tiêu học tập

Chào mừng bạn đến với chương quan trọng nhất trong toàn bộ cuốn sách này. Khái niệm mà bạn chuẩn bị khám phá chính là "trái tim và linh hồn", là phát minh vĩ đại nhất đã đưa ngôn ngữ Rust lên đỉnh cao của ngành công nghệ: **Quy tắc Sở hữu (Ownership)**.

Trong suốt nhiều thập kỷ qua, các kỹ sư phần mềm luôn bị mắc kẹt giữa hai lựa chọn đau đớn:
1. Hoặc chọn các ngôn ngữ như **C / C++**: Tự tay xin cấp phát bộ nhớ và tự tay giải phóng. Hậu quả là sinh ra hàng ngàn thảm họa bảo mật nổi tiếng thế giới (như rò rỉ bộ nhớ, lỗi giải phóng hai lần, hay con trỏ lơ lửng).
2. Hoặc chọn các ngôn ngữ có Bộ gom rác như **Java, C#, Go, Python**: Máy tính phải dành riêng một tiến trình chạy ngầm để liên tục quét và dọn rác bộ nhớ, khiến chương trình bị khựng, tiêu tốn nhiều RAM và không thể ứng dụng trong các hệ thống thời gian thực siêu tốc độ.

Rust xuất hiện và giải quyết triệt để bài toán hóc búa này bằng một giải pháp thứ ba: **Quản lý bộ nhớ thông qua Hệ thống Sở hữu được kiểm tra chặt chẽ ngay tại thời điểm biên dịch**. Không có bộ gom rác, không tốn dù chỉ một nano giây khi chạy thực tế!

Mục tiêu học tập của chương này:
- Nằm lòng 3 Quy tắc vàng của Quyền sở hữu (The 3 Rules of Ownership).
- Hiểu sâu sắc Cơ chế Di chuyển (**Move Semantics**) khi chuyển giao dữ liệu cấp phát trên Heap.
- Phân biệt sự khác nhau giữa kiểu tự động sao chép trên Stack (**Copy**) và kiểu chuyển giao sở hữu (**Move**).
- Biết khi nào nên dùng phương thức nhân bản sâu `.clone()` và hiểu rõ cái giá về mặt hiệu năng.
- Hiểu cơ chế dọn dẹp tự động khi biến ra khỏi phạm vi hoạt động (**Drop Trait**).

---

## Hình tượng hóa đời sống (Intuitive Everyday Analogy)

Để làm chủ Quyền sở hữu mà không cần bất kỳ kiến thức máy tính hàn lâm nào, hãy cùng quan sát 4 hình ảnh vô cùng sống động sau:

```
┌──────────────────────────────────────────────────────────────────────────────────┐
│                   HÌNH TƯỢNG ĐỜI SỐNG VỀ QUYỀN SỞ HỮU TRONG RUST                 │
├─────────────────────────┬───────────────────────────────┬────────────────────────┤
│   SỔ ĐỎ NHÀ ĐẤT CHÍNH CHỦ│     BÀN GIAO & SANG TÊN XE MÁY│   KHÁCH TRẢ PHÒNG KHÁCH│
│       (Quy tắc Sở hữu)  │       (Cơ chế Move Semantics) │         SẠN (Hàm Drop) │
│                         │                               │                        │
│ - Mảnh đất chỉ có 1 sổ đỏ│ - Bạn có xe SH, trao chìa khóa│ - Khách hết hạn thuê,  │
│ - Đúng 1 người đứng tên │   và sang tên cho em trai     │   bước ra khỏi sảnh    │
│ - Không thể 2 người xa lạ│ - Em trai là chủ sở hữu mới   │ - Nhân viên tự động vào│
│   cùng nhận quyền sở hữu│ - Bạn mất quyền dắt xe đi chơi│   lau dọn sạch sẽ      │
│   độc lập tại một lúc   │   (Biến cũ bị vô hiệu hóa)    │ - Không cần khách giặt │
└─────────────────────────┴───────────────────────────────┴────────────────────────┘
```

### 1. Sổ đỏ nhà đất chính chủ (Quy tắc Sở hữu cốt lõi)
Hãy tưởng tượng một căn nhà trên một mảnh đất:
- Trên giấy chứng nhận quyền sở hữu nhà đất (Sổ đỏ), tại một thời điểm chỉ có duy nhất **MỘT chủ sở hữu hợp pháp** đứng tên.
- Không bao giờ có chuyện hai người hoàn toàn xa lạ, không liên quan gì đến nhau, lại cùng cầm trong tay hai cuốn sổ đỏ chính chủ độc lập cho cùng một mảnh đất duy nhất đó.

### 2. Sang tên đổi chủ xe máy (Cơ chế Di chuyển - Move Semantics)
Giả sử bạn đang sở hữu một chiếc xe máy SH rất đẹp:
- Bạn làm thủ tục sang tên đổi chủ, trao toàn bộ giấy tờ đăng ký xe và chùm chìa khóa cho người em trai của mình (`let em_trai = xe_cua_ban;`).
- Kể từ giây phút chiếc chìa khóa rời khỏi tay bạn: **Người em trai là chủ nhân hợp pháp duy nhất của chiếc xe**.
- Bạn không còn chìa khóa xe trong túi nữa. Nếu bạn cố tình ra cổng tìm chiếc xe để phóng đi chơi, người nhà sẽ chặn bạn lại ngay: *"Anh đã sang tên xe cho em trai rồi, anh không còn quyền sử dụng chiếc xe đó nữa!"*.
Đây chính là cách Rust ngăn chặn lỗi nguy hiểm: **Khi quyền sở hữu đã bị chuyển đi (Move), biến ban đầu sẽ lập tức bị khóa lại và không thể truy cập được nữa!**

### 3. Tờ rơi quảng cáo in sẵn (Kiểu tự sao chép - Copy Trait)
Đối với các kiểu dữ liệu siêu nhỏ nằm gọn trên Stack (như con số nguyên `i32` hay `bool`):
- Chúng giống như những tờ rơi quảng cáo được in hàng loạt.
- Khi bạn muốn chia sẻ con số `42` cho bạn của mình (`let y = x;`), bạn không cần sang tên đổi chủ chiếc xe cồng kềnh. Bạn chỉ việc tiện tay photocopy thêm một tờ giấy mới và đưa cho người bạn đó.
- Bạn vẫn giữ tờ rơi của mình, người bạn có tờ rơi của họ. Cả hai người đều có bản sao riêng biệt trên Stack và dùng độc lập không ảnh hưởng gì tới nhau.

### 4. Khách trả phòng khách sạn (Cơ chế tự động dọn dẹp - Drop)
Khi bạn đi du lịch và thuê một phòng khách sạn:
- Trong suốt thời gian bạn còn ở trong phòng (`{ ... }`), căn phòng thuộc quyền sử dụng của bạn.
- Khi hết giờ thuê và bạn bước chân ra khỏi cửa sảnh khách sạn (`}`), nhân viên dọn phòng sẽ tự động bước vào thu dọn rác, thay ga trải giường và khóa phòng lại.
- Bạn không cần phải tự tay mang chổi quét nhà hay giặt khăn tắm trước khi về. Trong Rust, cơ chế dọn dẹp tự động siêu tốc này được gọi là **`Drop`**.

---

## Khái niệm & Cơ chế kỹ thuật chuyên sâu (In-Depth Technical Mechanics)

Bây giờ chúng ta sẽ soi chiếu cơ chế này dưới góc độ các ô nhớ vật lý trên thanh RAM máy tính.

### 1. Ba quy tắc vàng của Quyền sở hữu (The 3 Rules of Ownership)

Mọi dòng mã Rust bạn viết ra đều bị trình biên dịch giám sát dựa trên 3 quy tắc bất di bất dịch sau:
1. **Mỗi giá trị trong Rust đều có một Chủ sở hữu duy nhất (được đại diện bởi một biến).**
2. **Tại một thời điểm bất kỳ, chỉ có duy nhất một chủ sở hữu hợp pháp.**
3. **Khi chủ sở hữu đi ra khỏi phạm vi sống (Scope - biểu thị bởi dấu ngoặc nhọn đóng `}`), giá trị đó sẽ tự động bị tiêu hủy và trả lại bộ nhớ cho hệ điều hành ngay lập tức.**

### 2. Mổ xẻ thảm họa "Giải phóng hai lần" (Double Free) trong C/C++ và cách Rust khắc phục

Hãy xem điều gì sẽ xảy ra ở cấp độ bộ nhớ khi bạn gán một biến chuỗi sang biến khác:
```rust
let s1 = String::from("Xin chào");
let s2 = s1;
```

Như chúng ta đã biết ở Chương 05, một chuỗi `String` gồm 2 phần:
- Phần 24 bytes trên **Stack**: chứa con trỏ `ptr`, độ dài `len`, và sức chứa `capacity`.
- Dãy ký tự thực tế trên **Heap**: chứa chuỗi byte `"Xin chào"`.

```
      STACK                          HEAP
  ┌───────────┬──────────┐         ┌────────────────────────┐
  │ s1 (cũ)   │ 0x00A1   │ ──────> │  "Xin chào" (9 bytes)  │
  ├───────────┼──────────┤         └────────────────────────┘
  │ s2 (mới)  │ 0x00A1   │ ──────>              ▲
  └───────────┴──────────┘                      │
                                (CÙNG TRỎ VÀO MỘT VÙNG HEAP!)
```

Nếu hệ thống cho phép cả `s1` và `s2` cùng tồn tại:
- Khi hàm kết thúc, `s2` đi ra khỏi scope -> Hệ thống dọn dẹp vùng nhớ Heap tại địa chỉ `0x00A1`.
- Tiếp theo, `s1` cũng đi ra khỏi scope -> Hệ thống lại cố gắng giải phóng vùng nhớ `0x00A1` một lần nữa!
- Hiện tượng này gọi là **Double Free (Giải phóng hai lần)** — một trong những lỗi kinh hoàng nhất có thể phá nát bảng phân phối bộ nhớ của hệ điều hành và tạo kẽ hở cho tin tặc tấn công.

**Giải pháp kỳ tài của Rust**:
Thay vì sao chép dữ liệu tốn kém trên Heap, Rust sao chép 24 bytes trên Stack từ `s1` sang `s2`, nhưng ngay lập tức **HỦY BỎ TÍNH HỢP LỆ CỦA `s1`**.
Kể từ dòng lệnh `let s2 = s1;`, biến `s1` coi như đã "chết". Nếu bạn cố tình đọc biến `s1`, trình biên dịch Rust sẽ báo lỗi `E0382` ngay lập tức!
Vì `s1` đã bị vô hiệu hóa, khi ra khỏi scope, **chỉ có duy nhất `s2` đứng ra giải phóng vùng Heap đó**. Bài toán Double Free được giải quyết sạch sẽ 100%!

### 3. Phân biệt: Kiểu Move vs Kiểu Copy

Làm thế nào để biết khi gán biến thì Rust sẽ **Move (chuyển giao)** hay **Copy (sao chép)**?

- **Kiểu Copy**:
  - Dành riêng cho các kiểu dữ liệu có kích thước cố định, nằm trọn vẹn 100% trên Stack.
  - Chi phí sao chép một vài byte trên Stack là siêu rẻ (chỉ mất 1 chu kỳ CPU).
  - Bao gồm: tất cả các kiểu số nguyên (`i32`, `u64`,...), số thực (`f32`, `f64`), kiểu logic (`bool`), ký tự (`char`), và Tuple (nếu tất cả phần tử bên trong nó đều là kiểu Copy).
  - Khi gán `let y = x;`, biến `x` vẫn hoàn toàn nguyên vẹn và dùng bình thường.
- **Kiểu Move**:
  - Bất kỳ kiểu dữ liệu nào có nắm giữ tài nguyên cấp phát động bên ngoài Stack (như vùng nhớ Heap của `String`, `Vec<T>`, tệp tin đang mở, hoặc kết nối mạng).
  - Khi gán hoặc truyền vào hàm, quyền sở hữu sẽ bị di chuyển (Move). Biến cũ bị vô hiệu hóa.

### 4. Phương thức `.clone()` — Khi bạn thực sự muốn nhân bản Heap

Nếu bạn thực sự muốn cả hai biến đều có dữ liệu riêng biệt trên Heap, bạn phải gọi phương thức `.clone()` một cách rõ ràng:
```rust
let s1 = String::from("Bí mật");
let s2 = s1.clone(); // Cấp phát thêm một ô nhớ Heap mới toanh và sao chép toàn bộ nội dung sang!
```
Khi dùng `.clone()`, máy tính sẽ phải chạy ra bãi đỗ xe Heap, tìm một ô đất trống mới, sao chép từng ký tự sang ô mới đó. Quá trình này tiêu tốn nhiều thời gian và bộ nhớ hơn. Rust ép bạn phải tự tay gõ `.clone()` để bạn luôn có ý thức về chi phí tài nguyên mà mình đang tiêu hao.

---

## Mã nguồn minh họa thực chiến (Idiomatic Runnable Rust Blueprint)

Chương trình dưới đây minh họa toàn bộ các sắc thái của Quy tắc Sở hữu, cơ chế Move, cơ chế Copy, và cách chuyển giao quyền sở hữu qua các lời gọi hàm:

```rust
// File: src/main.rs
// Chương trình thực chiến làm chủ Quy tắc Sở hữu & Cơ chế Di chuyển (Move Semantics)

// 1. Hàm tiếp nhận quyền sở hữu: Biến truyền vào sẽ bị "nuốt chửng" tại đây!
fn consume_series(chuoi_nhan_vao: String) {
    println!("-> [Trong hàm tieu_thu_chuoi]: Đã nhận được: '{}'", chuoi_nhan_vao);
    // Khi hàm này kết thúc tại dấu ngoặc nhọn dưới, chuoi_nhan_vao đi ra khỏi scope
    // Bộ nhớ Heap của chuỗi này sẽ tự động bị giải phóng (DROP) ngay lập tức!
}

// 2. Hàm tiếp nhận và trả lại quyền sở hữu cho người gọi
fn append_suffix(mut series: String) -> String {
    series.push_str(" (Đã được kiểm định)");
    series // Trả lại quyền sở hữu chuỗi mới về cho nơi gọi hàm
}

// 3. Hàm nhận kiểu Copy trên Stack: Không ảnh hưởng gì đến biến gốc
fn print_int(so: i32) {
    println!("-> [Trong hàm in_so_nguyen]: Giá trị số là: {}", so);
}

fn main() {
    println!("============================================================");
    println!("     KHÁM PHÁ QUY TẮC SỞ HỮU & CƠ CHẾ DI CHUYỂN TRONG RUST  ");
    println!("============================================================");

    // --- PHẦN 1: CƠ CHẾ SAO CHÉP TRÊN STACK (COPY TRAIT) ---
    println!("\n1. Kiểm tra kiểu dữ liệu Copy trên Stack:");
    let base_score = 100;
    let point_num_copy = base_score; // Tự động nhân bản trên Stack

    println!("- Điểm gốc: {}, Điểm sao chép: {}", base_score, point_num_copy);
    print_int(base_score);
    // Biến base_score vẫn sử dụng hoàn toàn bình thường sau khi truyền vào hàm!
    println!("- Sau khi gọi hàm, điểm gốc vẫn còn nguyên: {}", base_score);

    // --- PHẦN 2: CƠ CHẾ DI CHUYỂN TRÊN HEAP (MOVE SEMANTICS) ---
    println!("\n2. Kiểm tra cơ chế Di chuyển quyền sở hữu (Move):");
    let security_certificate = String::from("CHUNG_THU_BAO_MAT_2026");
    println!("- Biến 'chung_thu_so' đang là chủ sở hữu hợp pháp duy nhất.");

    // Chuyển deliver quyền sở hữu từ security_certificate sang new_owner:
    let new_owner = security_certificate;
    println!("- Đã sang tên đổi chủ thành công cho: {}", new_owner);

    // NẾU BẠN BỎ CHÚ THÍCH DÒNG LỆNH SAU, RUSTC SẼ BÁO LỖI E0382 NGAY:
    // println!("Thử dùng lại biến cũ: {}", security_certificate);

    // --- PHẦN 3: DI CHUYỂN VÀO HÀM VÀ MẤT QUYỀN SỞ HỮU ---
    println!("\n3. Chuyển quyền sở hữu vào một hàm con:");
    let greeting = String::from("Xin chào từ Hà Nội");
    
    // Khi gọi hàm này, greeting bị Move vào hàm con và biến mất khỏi main!
    consume_series(greeting);

    // Dòng sau cũng bị lỗi E0382 vì greeting đã bị Drop bên trong hàm con:
    // println!("Thử in lại thông điệp: {}", greeting);

    // --- PHẦN 4: LẤY LẠI QUYỀN SỞ HỮU THÔNG QUA GIÁ TRỊ TRẢ VỀ ---
    println!("\n4. Chuyển deliver đi và nhận lại quyền sở hữu qua return:");
    let profile = String::from("Hồ sơ ứng viên Nguyễn Văn A");
    let decorated_profile = append_suffix(profile);
    // Lúc này 'profile' đã bị move, nhưng 'decorated_profile' là chủ nhân mới nắm giữ kết quả!
    println!("- Kết quả hồ sơ sau khi xử lý: {}", decorated_profile);

    // --- PHẦN 5: NHÂN BẢN SÂU BẰNG .clone() KHI CẦN THIẾT ---
    println!("\n5. Nhân bản sâu toàn diện bằng phương thức .clone():");
    let original_data = String::from("Bản quyền sở hữu trí tuệ");
    let tai_lieu_nhan_ban = original_data.clone(); // Cấp phát thêm một vùng nhớ Heap mới

    println!("- Bản gốc     : {}", original_data);
    println!("- Bản nhân bản: {}", tai_lieu_nhan_ban);
    println!("=> Cả hai biến đều cùng tồn tại và hoạt động độc lập trên 2 vùng Heap riêng biệt!");
}
```

---

## Bảng tra cứu lỗi biên dịch & Cách khắc phục (Compiler Error Guide)

Đây là những lỗi biên dịch kinh điển mà mọi lập trình viên Rust đều phải đối mặt khi mới làm quen với Ownership:

| Mã lỗi | Thông báo mẫu từ trình biên dịch | Nguyên nhân cốt lõi | Cách khắc phục nhanh |
|---|---|---|---|
| **E0382** | `error[E0382]: borrow of moved value: 's1'` | Bạn cố tình sử dụng lại một biến sau khi nó đã bị chuyển giao quyền sở hữu (Move) sang biến khác hoặc truyền vào hàm. | Có 3 cách khắc phục:<br>1. Dùng `.clone()` nếu muốn giữ lại bản sao.<br>2. Thay vì chuyển giao sở hữu, hãy dùng cơ chế **Vay mượn (Borrowing)** bằng dấu `&` (sẽ học ở Chương 07).<br>3. Cho hàm trả lại quyền sở hữu qua giá trị return. |
| **E0505** | `error[E0505]: cannot move out of 'x' because it is borrowed` | Bạn đang có một người khác mượn xem biến `x`, nhưng lại cố tình bán đứt (Move) biến `x` đi nơi khác. | Chờ cho người mượn dùng xong biến `x` rồi mới được phép di chuyển quyền sở hữu. |
| **E0384** | `cannot assign twice to immutable variable` | Cố tình gán lại giá trị cho biến bất biến sau khi đã Move. | Khai báo biến với từ khóa `mut` nếu bạn muốn tái sử dụng biến để chứa giá trị mới. |

---

## Tóm tắt chương & Bài tập rèn luyện (Summary & Exercises)

### 4 Điểm cốt lõi cần ghi nhớ:
1. **Triết lý 1 chủ**: Mỗi giá trị trên bộ nhớ chỉ có duy nhất một chủ sở hữu hợp pháp tại một thời điểm; khi chủ sở hữu ra khỏi scope, bộ nhớ tự động được dọn dẹp sạch sẽ bằng hàm `Drop`.
2. **Cơ chế Move**: Gán hoặc truyền một kiểu dữ liệu Heap (như `String`) sang biến/hàm khác sẽ chuyển giao quyền sở hữu và vô hiệu hóa vĩnh viễn biến ban đầu, ngăn chặn triệt để lỗi Double Free.
3. **Cơ chế Copy**: Các kiểu dữ liệu đơn giản nằm hoàn toàn trên Stack (`i32`, `f64`, `bool`, `char`) sẽ tự động sao chép giá trị mà không làm mất biến gốc.
4. **Chi phí của `.clone()`**: Dùng `.clone()` khi thực sự cần 2 bản sao độc lập trên Heap, nhưng cần cẩn trọng vì hành động này tiêu tốn thêm bộ nhớ và thời gian cấp phát.

### Bài tập rèn luyện tự giải:
1. **Bài tập phán đoán (Code inspection)**: Hãy quan sát đoạn mã sau và đoán xem dòng nào sẽ gây ra lỗi biên dịch:
   ```rust
   let a = String::from("Học Rust");
   let b = a;
   let c = 50;
   let d = c;
   println!("Giá trị: {} và {}", a, c);
   ```
   Hãy giải thích tại sao biến `c` in được mà biến `a` lại báo lỗi.
2. **Bài tập thực hành 2**: Viết một hàm `tinh_do_dai(s: String) -> (String, usize)` nhận vào một chuỗi, đo độ dài của chuỗi đó, sau đó trả về một bộ đôi Tuple chứa lại chính chuỗi đó và độ dài vừa đo được, để người gọi hàm không bị mất quyền sở hữu chuỗi.
3. **Bài tập tư duy 3**: Tại sao Rust không tự động thực hiện `.clone()` ngầm định cho chúng ta mỗi khi gán biến `String` giống như cách nó làm với số nguyên `i32`? Lợi ích về mặt hiệu năng hệ thống của quyết định thiết kế này là gì?
