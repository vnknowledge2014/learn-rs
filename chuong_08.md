# Chương 08: Vòng đời dữ liệu: Hiểu đúng mà không cần đau đầu (Lifetimes: Mental Models & Safe References)

## Giới thiệu & Mục tiêu học tập

Nếu bạn dạo quanh các diễn đàn công nghệ quốc tế và hỏi: *"Điều gì khiến những người mới học Rust cảm thấy sợ hãi nhất?"*, chắc chắn câu trả lời phổ biến nhất bạn nhận được sẽ là **Vòng đời dữ liệu (Lifetimes)** với những ký hiệu kỳ lạ như `'a`, `'b`.

Nhiều người bỏ cuộc vì nghĩ rằng Lifetime là một khái niệm toán học hàn lâm bí hiểm. Nhưng sự thật hoàn toàn ngược lại! Lifetime thực chất chỉ là một **bản cam kết thời gian mượn đồ rất đỗi đời thường**. Nếu bạn hiểu được cách một tấm vé vào cổng khu du lịch hoạt động ra sao, bạn sẽ làm chủ hoàn toàn Lifetime trong Rust chỉ sau vài trang sách.

Mục tiêu học tập của chương này:
- Hiểu mục đích tối thượng duy nhất của Lifetime: **Ngăn chặn triệt để hiện tượng Tham chiếu lơ lửng (Dangling Reference)** — tình trạng con trỏ trỏ vào vùng nhớ ma đã bị xóa bỏ.
- Xóa bỏ nỗi sợ hãi về các ký hiệu chú thích `'a`: Hiểu rằng `'a` không kéo dài tuổi thọ của biến, mà chỉ là lời mô tả mối liên hệ sống còn giữa các dữ liệu.
- Nằm lòng 3 Quy tắc suy luận vòng đời tự động (**Lifetime Elision Rules**) giúp bạn hiểu vì sao 90% trường hợp bạn không cần phải tự tay viết ký hiệu `'a`.
- Biết cách thiết kế một cấu trúc dữ liệu (`struct`) chứa tham chiếu mượn mà vẫn an toàn tuyệt đối.
- Hiểu rõ bản chất của vòng đời vĩnh cửu: `'static`.

---

## Hình tượng hóa đời sống (Intuitive Everyday Analogy)

Chúng ta hãy hữu hình hóa khái niệm Lifetime qua hai câu chuyện đời thường vô cùng dễ hình dung:

```
┌──────────────────────────────────────────────────────────────────────────────────┐
│                   HÌNH TƯỢNG ĐỜI SỐNG VỀ VÒNG ĐỜI (LIFETIMES)                    │
├─────────────────────────┬───────────────────────────────┬────────────────────────┤
│  TẤM VÉ KHU DU LỊCH CÒN │    ANH BẢO VỆ SOÁT VÉ KHẮT KHE│   CHỮ KHẮC TRÊN BIA ĐÁ │
│  HẠN HOẠT ĐỘNG?         │          (Borrow Checker)     │         ('static)      │
│                         │                               │                        │
│ - Khu du lịch mở cửa    │ - Kiểm tra: Thời hạn của vé   │ - Khắc sâu vào núi đá  │
│   (Dữ liệu gốc còn sống)│   phải NGẮN HƠN thời gian hoạt│ - Công viên có thể đóng│
│ - Tấm vé mới có giá trị │   động của khu nhà            │ - Người có thể đổi dời │
│ - Nếu khu đất bị giải tỏa│ - Vé có hạn lâu hơn nhà ->   │ - Dòng chữ trên vách đá│
│   mà vẫn cầm vé bước vào│   Chặn ngay từ cổng soát vé   │   sống mãi cùng ngọn   │
│   -> Rơi xuống vực sâu! │   (Từ chối biên dịch mã lỗi!) │   núi (toàn chương trình)│
└─────────────────────────┴───────────────────────────────┴────────────────────────┘
```

### 1. Tấm vé tham quan và Khu du lịch (Bản chất của Tham chiếu và Vòng đời)
Hãy tưởng tượng bạn mua một tấm vé vào cổng một khu triển lãm hoa:
- **Khu triển lãm (Dữ liệu gốc)**: Chỉ thuê mặt bằng và mở cửa đón khách từ 8h sáng đến 5h chiều (phạm vi sống `{ ... }`). Sau 5h chiều, toàn bộ nhà rạp bị tháo dỡ, mặt bằng bị san phẳng thành bãi đất trống (`Drop`).
- **Tấm vé trên tay bạn (Tham chiếu `&`)**: Tấm vé chỉ có giá trị khi khu triển lãm còn đang thực sự tồn tại!
- Điều gì xảy ra nếu 7h tối bạn vẫn cầm tấm vé đó mở cửa bước vào? Bạn sẽ bước hụt chân vào một bãi đất trống tan hoang nguy hiểm. Hiện tượng này trong khoa học máy tính gọi là **Tham chiếu lơ lửng (Dangling Reference)**.

### 2. Anh bảo vệ cổng soát vé khó tính (Trình biên dịch Rust)
Người gác cổng **Borrow Checker** của Rust có một nhiệm vụ tối thượng:
- Trước khi cho phép chương trình chạy, anh bảo vệ sẽ đặt chiếc thước đo thời gian cạnh nhau:
  - Thước đo 1: Tuổi thọ của ngôi nhà (Dữ liệu gốc).
  - Thước đo 2: Hạn sử dụng của tấm vé (Tham chiếu).
- **Quy tắc an toàn**: Hạn của tấm vé **bắt buộc phải ngắn hơn hoặc bằng** thời gian tồn tại của ngôi nhà! Nếu tấm vé đòi sống lâu hơn ngôi nhà, anh bảo vệ sẽ dứt khoát giơ biển đỏ từ chối cấp phép.

### 3. Dòng chữ tạc trên vách đá hoa cương (`'static`)
Trong công viên có một phiến đá hoa cương ngàn năm tuổi khắc dòng chữ: *"Chân lý không bao giờ đổi thay"*.
- Dù các sự kiện hội chợ có mở ra rồi dọn dẹp hàng ngàn lần, phiến đá vẫn sừng sững ở đó chừng nào cả quả núi còn tồn tại.
- Trong Rust, những chuỗi ký tự cố định được nhúng thẳng vào tệp nhị phân của chương trình (như `"Xin chào"`) có vòng đời mang tên `'static` — chúng tồn tại suốt từ giây phút chương trình khởi động cho đến khi tắt máy tính.

---

## Khái niệm & Cơ chế kỹ thuật chuyên sâu (In-Depth Technical Mechanics)

### 1. Tác hại chí mạng của Lỗi trả về tham chiếu biến cục bộ

Hãy xem xét đoạn mã bị cấm sau đây để hiểu vì sao Rust lại cần cơ chế Lifetime:
```rust
// RUST CHẶN ĐỨNG HÀM NÀY NGAY TỪ BƯỚC KIỂM TRA KIỂU VỚI LỖI E0106:
fn make_greeting_unsafe() -> &String {
    let s = String::from("Chào bạn"); // s sinh ra trên Stack Frame của hàm này
    &s // Cố tình trả về địa chỉ của biến cục bộ s
} // HÀM KẾT THÚC: Stack Frame bị xóa sổ! Biến s bị attempt hồi!
```
Trong các ngôn ngữ như C/C++, trình biên dịch vẫn để bạn chạy đoạn mã trên, dẫn đến con trỏ trỏ vào vùng nhớ rác (Dangling Pointer) gây sập chương trình ngẫu nhiên.
Rust bảo vệ bạn bằng **hệ thống phòng thủ hai lớp kiên cố**:
1. **Lớp 1 (Kiểm tra kiểu & Quy tắc lược bỏ vòng đời - Lỗi E0106)**:
   Khi nhìn vào `fn tao_loi_chao_nguy_hiem() -> &String`, Rust thấy hàm trả về một tham chiếu mượn nhưng lại không hề có bất kỳ tham số đầu vào nào để mượn từ đó. Rust chặn lại ngay với mã:
   `error[E0106]: missing lifetime specifier (thiếu chỉ định vòng đời)` kèm lời nhắc: *"Hàm này trả về dữ liệu mượn, nhưng không có dữ liệu nguồn nào để mượn!"*.
2. **Lớp 2 (Trình kiểm tra mượn Borrow Checker - Lỗi E0515)**:
   Nếu bạn cố tình thêm một tham số đầu vào (ví dụ `ten: &str`) để đánh lừa Lớp 1, Trình kiểm tra mượn sẽ lập tức quét sâu vào thân hàm và chặn đứng bằng mã:
   `error[E0515]: cannot return reference to local variable 's'` (*không thể trả về tham chiếu trỏ tới biến cục bộ*).

### 2. Giải mã ký hiệu chú thích vòng đời `'a`

Khi một hàm nhận vào **từ hai tham chiếu trở lên** và trả về một tham chiếu, trình biên dịch sẽ bối rối:
```rust
fn longer_of(x: &str, y: &str) -> &str {
    if x.len() > y.len() { x } else { y }
}
```
Trình biên dịch không thể biết trước lúc chạy xem hàm sẽ trả về `x` hay `y`. Do đó nó không biết tham chiếu trả về phụ thuộc vào tuổi thọ của ai để kiểm tra an toàn!

Chúng ta giải quyết bằng cách thêm chú thích vòng đời `'a`:
```rust
fn longer_of<'a>(x: &'a str, y: &'a str) -> &'a str {
    if x.len() > y.len() { x } else { y }
}
```
> **Bản chất kỹ thuật của `'a`**:
> - Ký hiệu `'a` **không làm biến nào sống lâu hơn hay chết sớm hơn**.
> - Nó chỉ là một **mệnh đề logic** tuyên bố với trình biên dịch rằng: *"Tham chiếu trả về sẽ có tuổi thọ bằng với khoảng thời gian sống chung (phần giao nhau nhỏ nhất) giữa `x` và `y`"*. Trình biên dịch sẽ dựa vào cam kết này để đảm bảo người nhận kết quả không sử dụng nó sau khi một trong hai biến `x` hoặc `y` đã qua đời!

### 3. Ba quy tắc suy luận ngầm (Lifetime Elision Rules)

Bạn không cần phải viết `'a` ở khắp mọi nơi, vì đội ngũ phát triển Rust đã tích hợp sẵn 3 quy tắc tự động suy luận sau vào trình biên dịch:

1. **Quy tắc 1 (Tham số đầu vào)**: Mỗi tham chiếu đầu vào của hàm sẽ được tự động gán cho một vòng đời độc lập riêng biệt (ví dụ: `fn foo(x: &i32, y: &i32)` được ngầm hiểu là `fn foo<'a, 'b>(x: &'a i32, y: &'b i32)`).
2. **Quy tắc 2 (Một đầu vào duy nhất)**: Nếu hàm chỉ có đúng **một** tham chiếu đầu vào, vòng đời của tham chiếu đó sẽ tự động được gán cho tất cả các tham chiếu trả về (ví dụ: `fn tach(s: &str) -> &str` được ngầm hiểu là `fn tach<'a>(s: &'a str) -> &'a str`).
3. **Quy tắc 3 (Phương thức có `&self`)**: Nếu hàm là một phương thức của Struct có tham số đầu vào là `&self` hoặc `&mut self`, thì vòng đời của `self` sẽ tự động được gán cho tất cả các tham chiếu trả về.

Chỉ khi nào hàm có **nhiều tham chiếu đầu vào** và **trả về một tham chiếu** mà không có `self`, Rust mới yêu cầu bạn phải tự tay viết ký hiệu `'a`.

### 4. Struct chứa Tham chiếu

Nếu bạn muốn tạo một `struct` không tự sở hữu dữ liệu mà chỉ mượn một phần dữ liệu từ nơi khác, bạn bắt buộc phải khai báo vòng đời cho struct đó:
```rust
struct Parser<'a> {
    du_lieu_nguon: &'a str, // Struct này cam kết không sống lâu hơn du_lieu_nguon
}
```

---

## Mã nguồn minh họa thực chiến (Idiomatic Runnable Rust Blueprint)

Chương trình dưới đây là một "Bộ phân tích cấu hình hệ thống siêu tốc không tốn RAM" (**Zero-Copy Config Parser**), minh họa toàn bộ các khía cạnh của Lifetime từ hàm so sánh đến struct chứa tham chiếu:

```rust
// File: src/main.rs
// Ứng dụng thực chiến làm chủ Vòng đời (Lifetimes) trong Rust

// 1. Hàm so sánh hai chuỗi và trả về chuỗi dài hơn
// Ký hiệu <'a> tuyên bố: Chuỗi trả về có vòng đời an toàn bằng khoảng deliver nhau giữa x và y
fn pick_longer_message<'a>(x: &'a str, y: &'a str) -> &'a str {
    if x.len() > y.len() {
        x
    } else {
        y
    }
}

// 2. Struct nắm giữ tham chiếu mượn dữ liệu nguồn (&'a str)
// Giúp đọc và trích xuất cấu hình mà KHÔNG tốn dù chỉ 1 byte để sao chép chuỗi mới trên Heap!
struct SystemConfig<'a> {
    name_resp_use: &'a str,
    phi_dich_vu: f64,
}

impl<'a> SystemConfig<'a> {
    // Phương thức đọc: Tận dụng Quy tắc suy luận ngầm số 3 (Lifetime Elision)
    // Không cần viết 'a ở kiểu trả về vì Rust tự lấy vòng đời của &self!
    fn lay_ten(&self) -> &str {
        self.name_resp_use
    }

    fn print_info(&self) {
        println!("- Ứng dụng: '{}' | Phí duy trì: {:.2} USD/tháng", 
                 self.name_resp_use, self.phi_dich_vu);
    }
}

fn main() {
    println!("============================================================");
    println!("      BỘ PHÂN TÍCH CẤU HÌNH SIÊU TỐC - ZERO-COPY PARSER     ");
    println!("============================================================");

    // --- PHẦN 1: HÀM CÓ CHÚ THÍCH VÒNG ĐỜI 'a ---
    println!("\n1. So sánh hai thông điệp có vòng đời hợp lệ:");
    let thong_message_1 = String::from("Hệ thống khởi động thành công");
    let thong_message_2 = String::from("Cảnh báo pin yếu");

    // Cả thong_message_1 và thong_message_2 đều đang sống trong cùng phạm vi main
    let thong_message_main = pick_longer_message(
        thong_message_1.as_str(), 
        thong_message_2.as_str()
    );
    println!("- Thông điệp dài hơn được chọn: '{}'", thong_message_main);

    // --- PHẦN 2: CHỨNG MINH TÍNH AN TOÀN TRƯỚC VÒNG ĐỜI NGẮN HƠN ---
    println!("\n2. Kiểm soát phạm vi sống lồng nhau an toàn:");
    let parent_string = String::from("Dữ liệu bền vững của công ty");
    {
        let series_con = String::from("Dữ liệu tạm");
        let ket_qua_tam = pick_longer_message(parent_string.as_str(), series_con.as_str());
        println!("- [Bên trong phạm vi con]: Kết quả chọn là: '{}'", ket_qua_tam);
        // ket_qua_tam chỉ được phép dùng bên trong dấu ngoặc nhọn này!
        // Nếu cố tình mang ket_qua_tam ra ngoài phạm vi con, compiler sẽ chặn đứng ngay!
    }

    // --- PHẦN 3: STRUCT CHỨA THAM CHIẾU (ZERO-COPY) ---
    println!("\n3. Khởi tạo Struct chứa tham chiếu mượn không tốn RAM:");
    let config_file = String::from("TenUngDung: RustCloudServer, Phi: 49.99");

    // Lát cắt trích xuất tên ứng dụng trực tiếp từ chuỗi nguồn:
    let name_cut_can = &config_file[12..27];

    let config = SystemConfig {
        name_resp_use: name_cut_can,
        phi_dich_vu: 49.99,
    };

    config.print_info();
    println!("- Tên ứng dụng trích xuất qua getter: '{}'", config.lay_ten());

    // --- PHẦN 4: VÒNG ĐỜI VĨNH CỬU 'static ---
    println!("\n4. Sử dụng hằng số có vòng đời vĩnh cửu ('static):");
    let eternal_message: &'static str = "PHẦN MỀM ĐÃ ĐƯỢC CHỨNG NHẬN AN TOÀN TUYỆT ĐỐI";
    println!("- Dòng chữ trên bia đá vĩnh cửu: '{}'", eternal_message);
}
```

---

## Bảng tra cứu lỗi biên dịch & Cách khắc phục (Compiler Error Guide)

Dưới đây là các thông báo lỗi kinh điển về Lifetimes và cách khắc phục chính xác:

| Mã lỗi | Thông báo mẫu từ trình biên dịch | Nguyên nhân cốt lõi | Cách khắc phục nhanh |
|---|---|---|---|
| **E0106** | `missing lifetime specifier: expected named lifetime parameter` | Hàm có từ 2 tham chiếu đầu vào trở lên và trả về 1 tham chiếu, nhưng không có chú thích `'a`. | Thêm khai báo `<'a>` sau tên hàm và gán `'a` vào các tham chiếu đầu vào và đầu ra tương ứng. |
| **E0597** | `borrowed value does not live long enough` | Bạn tạo một biến tạm thời trong khối ngoặc nhọn con, mượn địa chỉ của nó, nhưng lại cố sử dụng tham chiếu đó ở phạm vi bên ngoài sau khi biến tạm đã qua đời. | Kéo dài tuổi thọ của biến gốc bằng cách khai báo nó ở phạm vi bên ngoài, hoặc chuyển sang trả về giá trị sở hữu (Owned Type như `String` thay vì `&str`). |
| **E0515** | `cannot return reference to local variable` | Cố tình trả về con trỏ trỏ vào một biến được tạo ra bên trong chính hàm đó. | Thay đổi kiểu trả về của hàm từ dạng mượn (`&String`) sang dạng sở hữu (`String`) để Move dữ liệu ra ngoài cho người gọi. |

---

## Tóm tắt chương & Bài tập rèn luyện (Summary & Exercises)

### 4 Điểm cốt lõi cần ghi nhớ:
1. **Sứ mệnh của Lifetime**: Đảm bảo mọi con trỏ và tham chiếu luôn trỏ vào một vùng ô nhớ còn sống và hợp lệ; loại trừ vĩnh viễn lỗi Tham chiếu lơ lửng (Dangling Reference).
2. **Bản chất của `'a`**: Là bản hợp đồng mô tả mối liên hệ sống còn giữa các tham chiếu, không làm kéo dài hay rút ngắn tuổi thọ tự nhiên của bất kỳ biến nào.
3. **Quy tắc lược bỏ (Elision Rules)**: Trình biên dịch tự động suy luận vòng đời cho hàm có 1 tham chiếu đầu vào hoặc phương thức có `&self`, bạn chỉ cần can thiệp khi có sự mập mờ từ nhiều nguồn tham chiếu.
4. **Vòng đời `'static`**: Tồn tại suốt toàn bộ thời gian chạy của chương trình, đại diện cho các chuỗi văn bản cố định được nhúng sẵn trong mã nhị phân.

### Bài tập rèn luyện tự giải:
1. **Bài tập suy luận (Elision practice)**: Hãy cho biết trong các hàm sau đây, hàm nào cần tự tay viết chú thích `'a`, hàm nào được Rust tự động suy luận:
   - `fn in_loi_chao(ten: &str);`
   - `fn lay_ky_tu_dau(van_ban: &str) -> &str;`
   - `fn ghep_ten(ho: &str, ten: &str) -> &str;`
2. **Bài tập thực hành 2**: Viết một hàm mang tên `chon_chuoi_ngan_hon<'a>(s1: &'a str, s2: &'a str) -> &'a str` nhận vào hai tham chiếu lát cắt chuỗi (`&str`) và trả về tham chiếu của chuỗi có độ dài ngắn hơn (sử dụng phương thức `.len()`). Trong hàm `main`: tạo hai biến `String` có độ dài khác nhau, gọi hàm và in chuỗi ngắn hơn ra màn hình. Thử giải thích tại sao tham số vòng đời `<'a>` là bắt buộc trong chữ ký hàm này.
3. **Bài tập sửa lỗi (Compiler fix)**: Đoạn mã sau bị lỗi biên dịch:
   ```rust
   fn make_string() -> &str {
       let s = "Rustacean".to_string();
       &s
   }
   ```
   Hãy giải thích tại sao đoạn mã trên bị lỗi và đưa ra cách sửa tối ưu nhất.

---

### Gợi ý & Lời giải

<details>
<summary><b>Bài tập 1 — Gợi ý</b></summary>

Quy tắc rút gọn vòng đời (lifetime elision) tự lo được khi **chỉ có một tham chiếu đầu vào**. Rắc rối xuất hiện khi có **nhiều tham chiếu đầu vào mà lại trả về tham chiếu** — Rust không đoán được trả về mượn từ cái nào.
</details>

<details>
<summary><b>Bài tập 1 — Lời giải</b></summary>

Xét từng hàm:

**`fn in_loi_chao(ten: &str);` — Rust tự lo, KHÔNG cần viết `'a`.**
Chỉ nhận một tham chiếu và *không trả về* tham chiếu nào. Không có gì để gắn vòng đời đầu ra vào, nên chẳng có mơ hồ.

**`fn lay_ky_tu_dau(van_ban: &str) -> &str;` — Rust tự lo, KHÔNG cần viết `'a`.**
Đúng một tham chiếu vào, một tham chiếu ra. Quy tắc rút gọn nói: đầu ra *phải* mượn từ đầu vào duy nhất đó. Không mơ hồ, Rust tự điền `'a` ngầm.

**`fn ghep_ten(ho: &str, ten: &str) -> &str;` — BẮT BUỘC tự viết `'a`.**
Hai tham chiếu vào, một tham chiếu ra. Rust không biết kết quả mượn từ `ho` hay từ `ten`, nên **từ chối đoán** và báo lỗi. Bạn phải nói rõ, ví dụ `fn ghep_ten<'a>(ho: &'a str, ten: &'a str) -> &'a str` — buộc cả hai đầu vào và đầu ra sống cùng một vòng đời.

Nguyên tắc gọn: **rút gọn vòng đời chỉ hoạt động khi không có mơ hồ.** Một tham chiếu vào thì đầu ra chỉ có thể mượn từ nó; nhiều tham chiếu vào thì bạn phải tự chỉ định.
</details>

<details>
<summary><b>Bài tập 2 — Gợi ý</b></summary>

Vì trả về tham chiếu mượn từ *một trong hai* đầu vào, Rust cần bạn hứa cả hai đầu vào và đầu ra sống đủ lâu như nhau — đó là việc của `<'a>`.
</details>

<details>
<summary><b>Bài tập 2 — Lời giải</b></summary>

```rust
// <'a> hứa: kết quả sống không lâu hơn ĐẦU VÀO NGẮN TUỔI NHẤT trong s1, s2.
// Bắt buộc phải có, vì Rust không tự biết kết quả mượn từ s1 hay s2.
fn chon_chuoi_ngan_hon<'a>(s1: &'a str, s2: &'a str) -> &'a str {
    if s1.len() <= s2.len() { s1 } else { s2 }
}

fn main() {
    let a = String::from("Rust");
    let b = String::from("Ngôn ngữ lập trình");
    println!("Ngắn hơn: {}", chon_chuoi_ngan_hon(&a, &b));
}

#[test]
fn chon_dung_chuoi_ngan() {
    assert_eq!(chon_chuoi_ngan_hon("Rust", "Programming"), "Rust");
    assert_eq!(chon_chuoi_ngan_hon("abcdef", "xy"), "xy");
}
```

**Vì sao `<'a>` bắt buộc ở đây:** hàm trả về một tham chiếu, nhưng nó có thể là `s1` *hoặc* `s2` — quyết định lúc chạy, tùy độ dài. Trình biên dịch cần một lời hứa ở *biên dịch* rằng tham chiếu trả về không sống lâu hơn dữ liệu nó trỏ tới. `<'a>` buộc cả hai đầu vào và đầu ra chung một vòng đời, nên kết quả bị ràng buộc sống không quá cái đầu vào chết sớm hơn. Không có nó, bạn có thể trả về tham chiếu tới một chuỗi đã bị hủy — đúng loại lỗi mà chương này dạy cách chặn.
</details>

<details>
<summary><b>Bài tập 3 — Gợi ý</b></summary>

Lỗi kinh điển: trả về tham chiếu tới một biến **cục bộ** sẽ bị hủy ngay khi hàm kết thúc. Sửa bằng cách **trả về giá trị sở hữu** (`String`) thay vì tham chiếu.
</details>

<details>
<summary><b>Bài tập 3 — Lời giải</b></summary>

**Vì sao lỗi:** `s` là biến cục bộ, bị **hủy khi hàm kết thúc**. Trả về `&s` là trả về tham chiếu tới vùng nhớ vừa bị giải phóng — một con trỏ treo (dangling reference). Rust chặn ngay với lỗi `cannot return reference to local variable `s``.

```text
fn make_string() -> &str {        // trả về &str, nhưng mượn từ đâu?
    let s = "Rustacean".to_string();  // s sinh ra trong hàm...
    &s                                 // ...và CHẾT ở dấu } cuối hàm -> treo
}
```

**Cách sửa tối ưu — trả về `String` sở hữu, không phải tham chiếu:**
```rust
fn make_string() -> String {
    "Rustacean".to_string()   // chuyển quyền sở hữu RA NGOÀI cho người gọi
}
```

Giờ chuỗi không chết cùng hàm — **quyền sở hữu của nó được chuyển ra** cho người gọi, và nó sống tiếp bao lâu người gọi cần. Đây là hướng giải quyết đúng: khi dữ liệu được *tạo ra bên trong hàm*, hàm nên **trả nó đi** (sở hữu), chứ không cho mượn thứ mình sắp hủy. Chỉ trả về tham chiếu khi dữ liệu *đã tồn tại từ trước* ở một trong các đầu vào.
</details>
