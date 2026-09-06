# Chương 11: Xử lý lỗi chuyên nghiệp: Panic! vs Result<T, E> và Toán tử `?` (Error Handling: Panic vs Result and the `?` Operator)

## Giới thiệu & Mục tiêu học tập

Trong lập trình thực tế, không có một hệ thống phần mềm nào có thể chạy trơn tru mãi mãi mà không bao giờ gặp sự cố. Người dùng có thể vô tình gõ sai định dạng email, đường truyền mạng Wifi có thể bị chập chờn ngắt kết nối giữa chừng, hoặc chiếc ổ cứng có thể bị đầy dung lượng khi đang lưu dữ liệu.

Sự khác biệt giữa một lập trình viên nghiệp dư và một kỹ sư phần mềm cao cấp nằm ở chỗ: **Họ ứng phó như thế nào khi sự cố xảy ra?** Một ứng dụng tồi sẽ lập tức "đóng băng" hoặc văng ra màn hình desktop khiến người dùng mất trắng dữ liệu. Một ứng dụng Rust chuẩn mực sẽ lường trước mọi rủi ro và xử lý lỗi một cách lịch sự, kiên cố.

Mục tiêu học tập của chương này:
- Nắm vững hai triết lý xử lý sự cố trong Rust:
  - Lỗi không thể phục hồi (**Unrecoverable Errors** - dùng `panic!`).
  - Lỗi có thể phục hồi (**Recoverable Errors** - dùng kiểu `Result<T, E>`).
- Nhận thức sâu sắc về hiểm họa của việc lạm dụng `.unwrap()` trong mã nguồn thực tế ("Trò chơi cò quay Nga").
- Sử dụng thành thạo các công cụ xử lý lỗi an toàn: `.expect()`, `.unwrap_or()`, và `.unwrap_or_else()`.
- Làm chủ toán tử lan truyền lỗi thần thánh **`?` (The `?` Operator)** giúp mã nguồn ngắn gọn và sáng sủa.
- Tự tay thiết kế kiểu lỗi tùy biến chuyên nghiệp (**Custom Error Types**) bằng `enum`.

---

## Hình tượng hóa đời sống (Intuitive Everyday Analogy)

Hãy cùng hình dung cách ứng phó sự cố của Rust qua 3 hình tượng đời sống vô cùng trực quan:

```
┌──────────────────────────────────────────────────────────────────────────────────┐
│                    HÌNH TƯỢNG ĐỜI SỐNG VỀ XỬ LÝ LỖI TRONG RUST                   │
├─────────────────────────┬───────────────────────────────┬────────────────────────┤
│   CẦU ĐƯỜNG SẮT BỊ SẬP  │      CÂY RÚT TIỀN ATM NGÂN    │     NGƯỜI TRỢ LÝ VĂN   │
│         (Macro panic!)  │      HÀNG (Kiểu Result<T, E>) │     THƯ (Toán tử '?')  │
│                         │                               │                        │
│ - Tàu đang chạy 300km/h │ - Khách bấm rút 5 triệu       │ - Mở phong bì giấy tờ  │
│ - Cầu phía trước bị gãy │ - Đủ tiền: Nhả Ok(5 triệu)    │ - Nếu thiếu giấy: Cầm  │
│ - KHÔNG THỂ ĐI TIẾP!    │ - Hết tiền: Nhả thẻ ra và in  │   ngay báo cáo lỗi lên │
│ - Giật phanh khẩn cấp,  │   biên lai Err("Không đủ tiền")│   bàn sếp xin ý kiến   │
│   dừng toàn bộ đoàn tàu │ - Máy tuyệt đối KHÔNG BỐC     │ - Nếu đủ: Bóc sẵn đặt  │
│   để bảo toàn tính mạng!│   KHÓI NỔ TUNG SẬP NGUỒN!     │   lên bàn cho sếp ký   │
└─────────────────────────┴───────────────────────────────┴────────────────────────┘
```

### 1. Cầu đường sắt bị sập nhịp xuống vực (`panic!`)
Hãy tưởng tượng một đoàn tàu chở khách đang lao đi với tốc độ cao:
- Bác lái tàu phát hiện cây cầu sắt bắc qua vực sâu phía trước đã bị lũ cuốn trôi hoàn toàn.
- Trong tình huống này, **không có bất kỳ cách nào để tiếp tục chuyến đi một cách an toàn**. Nếu cố tình chạy tiếp, đoàn tàu sẽ lao xuống vực thẳm.
- Bác lái tàu lập tức giật mạnh chiếc cần gạt phanh khẩn cấp (`panic!`). Còi báo động hú vang, bánh tàu nghiến chặt xuống đường ray, đoàn tàu dừng lại ngay lập tức để bảo vệ an toàn tính mạng cho toàn bộ hành khách.

### 2. Cây rút tiền ATM ngân hàng (Kiểu `Result<T, E>`)
Bạn đưa thẻ ngân hàng vào cây ATM và bấm rút 5.000.000 VND:
- **Trường hợp thành công (`Ok`)**: Máy đếm tiền xoẹt xoẹt và nhả ra phong bì tiền mặt `Ok(5_000_000 VND)`.
- **Trường hợp sự cố (`Err`)**: Tài khoản của bạn chỉ còn 100.000 VND. Cây ATM **tuyệt đối không được phát nổ hay bốc khói đen sập nguồn**! Thay vào đó, cây ATM sẽ đẩy chiếc thẻ nhựa trả lại vào tay bạn kèm một thông báo lịch sự trên màn hình: `Err("Số dư không đủ để thực hiện giao dịch")`. Bạn hoàn toàn có thể bấm chọn rút một số tiền nhỏ hơn.

### 3. Người trợ lý văn thư mẫn cán (Toán tử `?`)
Bạn giao cho người trợ lý một quy trình xử lý hồ sơ gồm 3 bước liên hoàn: Mở phong bì thư -> Kiểm tra hợp đồng -> Đóng dấu giáp lai.
- Nếu bạn tự làm thủ công, bạn sẽ phải viết: "Nếu bước 1 hỏng thì dừng lại báo lỗi; nếu bước 1 xong thì làm bước 2; nếu bước 2 hỏng thì dừng lại báo lỗi...". Rất dài dòng và mệt mỏi!
- Với toán tử `?`, người trợ lý sẽ làm việc thay bạn:
  - Người trợ lý mở phong bì thư. Nếu phong bì trống rỗng hoặc bị rách nát (`Err`), người trợ lý **lập tức cầm tờ giấy báo lỗi chạy lên bàn sếp nộp ngay** và dừng quy trình lại.
  - Nếu phong bì có đầy đủ hợp đồng hợp lệ (`Ok`), người trợ lý sẽ tự động bóc mở sẵn tập tài liệu đặt ngay ngắn lên bàn để bạn chỉ việc cầm bút ký bước tiếp theo!

---

## Khái niệm & Cơ chế kỹ thuật chuyên sâu (In-Depth Technical Mechanics)

### 1. Phẫu thuật `panic!` và Quy trình Xổ cuộn Ngăn xếp (Stack Unwinding)

Khi macro `panic!("Thông điệp lỗi")` được gọi, Rust không đơn thuần là "tắt phụt" chương trình như câu lệnh `exit(1)` trong ngôn ngữ C. Mặc định, Rust sẽ kích hoạt quy trình **Xổ cuộn ngăn xếp (Stack Unwinding)**:
- Hệ thống sẽ đi ngược từ hàm bị lỗi quay trở lại hàm gọi nó, đi dần xuống đáy Stack.
- Tại mỗi khung hàm (Stack Frame) đi qua, Rust sẽ **gọi hàm `Drop` để giải phóng sạch sẽ ô nhớ, đóng các tệp tin đang mở và ngắt kết nối mạng an toàn**.
- Sau khi dọn dẹp sạch sẽ tài nguyên, chương trình mới chính thức kết thúc và in ra thông báo lỗi.

> **Mẹo kỹ thuật chuyên sâu**: Khi chương trình bị panic, bạn có thể chạy ứng dụng với biến môi trường:
> `RUST_BACKTRACE=1 cargo run`
> **Chương trình đang chạy** (chứ không phải trình biên dịch) sẽ in ra một bản đồ chi tiết từng lời gọi hàm, từng tệp mã nguồn từ khi chương trình khởi động đến đúng nơi phát sinh sự cố, giúp bạn tìm ra nguyên nhân gốc rễ chỉ trong vài giây! (`RUST_BACKTRACE` là biến môi trường lúc chạy, không liên quan gì tới lúc biên dịch.)

### 2. Định nghĩa kiểu `Result<T, E>` và Cảnh báo `#[must_use]`

Kiểu `Result` trong thư viện chuẩn của Rust là một Enum gồm hai nhánh:
```rust
enum Result<T, E> {
    Ok(T),  // T là kiểu dữ liệu trả về khi thành công
    Err(E), // E là kiểu dữ liệu mô tả lỗi khi thất bại
}
```

Điểm đặc biệt là kiểu `Result` được gắn một thuộc tính đặc biệt mang tên `#[must_use]`:
- Nếu một hàm trả về `Result` (ví dụ hàm ghi tệp tin) mà bạn gọi hàm đó nhưng **không hứng kết quả hoặc không kiểm tra lỗi**, trình biên dịch Rust sẽ lập tức phát ra cảnh báo màu vàng:
  `warning: unused Result that must be used`
- Rust không cho phép bạn "lờ đi" các khả năng thất bại của phần mềm!

### 3. Hiểm họa khôn lường của `.unwrap()`

Phương thức `.unwrap()` hoạt động theo nguyên tắc:
- Nếu là `Ok(giá_trị)`: bóc hộp lấy giá trị ra.
- Nếu là `Err(lỗi)`: **KÍCH HOẠT `panic!` LẬP TỨC VÀ ĐÁNH SẬP CHƯƠNG TRÌNH!**

Việc lạm dụng `.unwrap()` giống như bạn đang chơi trò cò quay Nga với ứng dụng của mình. Một ứng dụng thương mại chuyên nghiệp không bao giờ được phép dùng bừa bãi `.unwrap()`. Thay vào đó, hãy dùng:
- **`.expect("Mô tả ngữ cảnh vì sao mong đợi có dữ liệu")`**: Nếu có sập thì cũng in ra lý do rõ ràng.
- **`.unwrap_or(giá_trị_mặc_định)`**: Nếu lỗi thì tự động lấy giá trị thay thế an toàn.
- **Toán tử `?`**: Lan truyền lỗi (Error propagation) ngược lên cho hàm cha cấp cao hơn xử lý.

### 4. Cơ chế hoạt động của Toán tử Lan truyền Lỗi `?`

Dòng lệnh:
```rust
let data = doc_du_lieu_tu_mang()?;
```
Được trình biên dịch Rust tự động mở rộng tương đương với khối mã sau:
```rust
let data = match doc_du_lieu_tu_mang() {
    Ok(val) => val,
    Err(err) => return Err(From::from(err)), // Thoát hàm ngay lập tức và trả về Err!
};
```
Toán tử `?` chỉ được phép sử dụng bên trong một hàm có kiểu trả về là `Result` hoặc `Option`.

> **Chi tiết ít người để ý nhưng cực kỳ quan trọng — `?` tự động gọi `From::from`:**
> Hãy nhìn kỹ dòng `return Err(From::from(err))` ở trên. Toán tử `?` **không** chỉ trả lỗi ra ngoài — nó còn *chuyển đổi kiểu lỗi* trên đường đi.
>
> Nhờ vậy, một hàm có thể gọi nhiều thư viện khác nhau (mỗi thư viện một kiểu lỗi riêng) mà vẫn gom hết về **một kiểu lỗi thống nhất** của bạn:
>
> ```rust
> #[derive(Debug)]
> enum LoiUngDung {
>     DocTep(std::io::Error),
>     PhanTichSo(std::num::ParseIntError),
> }
>
> // Hai cây cầu cho `?` đi qua:
> impl From<std::io::Error> for LoiUngDung {
>     fn from(e: std::io::Error) -> Self { LoiUngDung::DocTep(e) }
> }
> impl From<std::num::ParseIntError> for LoiUngDung {
>     fn from(e: std::num::ParseIntError) -> Self { LoiUngDung::PhanTichSo(e) }
> }
>
> fn doc_cau_hinh(path: &str) -> Result<u16, LoiUngDung> {
>     let content = std::fs::read_to_string(path)?;  // io::Error  -> LoiUngDung
>     let gate: u16 = content.trim().parse()?;            // ParseIntError -> LoiUngDung
>     Ok(gate)
> }
> ```
>
> Nếu chưa có `impl From<...>`, trình biên dịch sẽ báo lỗi *"`?` couldn't convert the error"*. Khi đó bạn có hai lựa chọn: cài `From`, hoặc chuyển thủ công ngay tại chỗ bằng **`.map_err(...)`** trước dấu `?`:
>
> ```rust
> let gate: u16 = content.trim().parse().map_err(LoiUngDung::PhanTichSo)?;
> ```
>
> Chúng ta sẽ gặp lại `map_err` ở **Chương 17** dưới cái tên "bẻ ghi sang đường ray thất bại", và ở **Chương 19** với tên chính thức của nó: *Bifunctor*.

---

## Mã nguồn minh họa thực chiến (Idiomatic Runnable Rust Blueprint)

Chương trình dưới đây mô phỏng một hệ thống đọc tệp cấu hình thanh toán tài chính, kết hợp tự tạo kiểu lỗi tùy chỉnh bằng Enum, sử dụng toán tử `?` để lan truyền lỗi, và xử lý êm đẹp các tình huống thất bại:

```rust
// File: src/main.rs
// Chương trình thực chiến làm chủ Kỹ thuật Xử lý Lỗi Chuyên Nghiệp trong Rust

// 1. Tự định nghĩa kiểu Lỗi Nghiệp Vụ Tùy Biến (Custom Error Type) bằng Enum
#[derive(Debug)]
enum MathError {
    SoTienKhongHopLe(String),
    TaiKhoanBiKhoa,
    SoDuKhongDu { balance: f64, can_rut: f64 },
}

// Cài đặt khả năng in ấn đẹp mắt cho kiểu lỗi của chúng ta
impl std::fmt::Display for MathError {
    fn fmt(&self, f: &mut std::fmt::Formatter) -> std::fmt::Result {
        match self {
            MathError::SoTienKhongHopLe(msg) => write!(f, "Số tiền không hợp lệ: {}", msg),
            MathError::TaiKhoanBiKhoa => write!(f, "Tài khoản đang bị khóa do vi phạm an ninh!"),
            MathError::SoDuKhongDu { balance, can_rut } => {
                write!(f, "Số dư không đủ (Hiện có: {:.2}, Yêu cầu rút: {:.2})", balance, can_rut)
            }
        }
    }
}

// 2. Hàm kiểm tra tính hợp lệ của số tiền nhập vào
fn check_num_tien(input_buffer: &str) -> Result<f64, MathError> {
    let so_tien: f64 = input_buffer.trim().parse().map_err(|_| {
        MathError::SoTienKhongHopLe(String::from("Vui lòng chỉ nhập các chữ số hợp lệ!"))
    })?;

    if so_tien <= 0.0 {
        return Err(MathError::SoTienKhongHopLe(String::from("Số tiền phải lớn hơn 0!")));
    }

    Ok(so_tien)
}

// 3. Hàm thực hiện giao dịch: Tận dụng toán tử '?' để lan truyền lỗi siêu gọn
fn display_trade(
    input_buffer: &str, 
    mut so_du_hien_tai: f64, 
    is_account_active: bool
) -> Result<f64, MathError> {
    // Bước 1: Kiểm tra trạng thái tài khoản
    if !is_account_active {
        return Err(MathError::TaiKhoanBiKhoa);
    }

    // Bước 2: Phân tích số tiền bằng toán tử '?'
    // Nếu check_num_tien trả về Err, hàm lập tức return Err ngay tại dòng này!
    let so_tien_can_rut = check_num_tien(input_buffer)?;

    // Bước 3: Kiểm tra hạn mức số dư
    if so_tien_can_rut > so_du_hien_tai {
        return Err(MathError::SoDuKhongDu {
            balance: so_du_hien_tai,
            can_rut: so_tien_can_rut,
        });
    }

    // Bước 4: Trừ tiền thành công
    so_du_hien_tai -= so_tien_can_rut;
    Ok(so_du_hien_tai) // Trả về số dư mới bọc trong Ok
}

fn main() {
    println!("============================================================");
    println!("     CỔNG THANH TOÁN TÀI CHÍNH AN TOÀN - RUST BANKING       ");
    println!("============================================================");

    let first_balance_sell = 5_000_000.0;

    // --- KỊCH BẢN 1: GIAO DỊCH THÀNH CÔNG HỢP LỆ ---
    println!("\n[Kịch bản 1] Rút 1.500.000 VND hợp lệ:");
    match display_trade("1500000", first_balance_sell, true) {
        Ok(new_balance) => println!("-> Giao dịch THÀNH CÔNG! Số dư còn lại: {:.2} VND", new_balance),
        Err(e) => println!("-> Giao dịch THẤT BẠI: {}", e),
    }

    // --- KỊCH BẢN 2: LỖI NHẬP LIỆU KHÔNG PHẢI CHỮ SỐ ---
    println!("\n[Kịch bản 2] Người dùng nhập chữ linh tinh:");
    match display_trade("mot_trieu", first_balance_sell, true) {
        Ok(new_balance) => println!("-> Thành công: {:.2} VND", new_balance),
        Err(e) => println!("-> Hệ thống xử lý êm dịu: [{}]", e),
    }

    // --- KỊCH BẢN 3: LỖI SỐ DƯ KHÔNG ĐỦ ĐỂ RÚT ---
    println!("\n[Kịch bản 3] Rút số tiền vượt hạn mức số dư:");
    match display_trade("10000000", first_balance_sell, true) {
        Ok(new_balance) => println!("-> Thành công: {:.2} VND", new_balance),
        Err(e) => println!("-> Báo cáo lỗi chính xác: [{}]", e),
    }

    // --- KỊCH BẢN 4: LỖI TÀI KHOẢN BỊ KHÓA AN NINH ---
    println!("\n[Kịch bản 4] Tài khoản bị phong tỏa:");
    match display_trade("500000", first_balance_sell, false) {
        Ok(new_balance) => println!("-> Thành công: {:.2} VND", new_balance),
        Err(e) => println!("-> Từ chối truy cập: [{}]", e),
    }

    // --- KỊCH BẢN 5: CÁC PHƯƠNG THỨC XỬ LÝ DỰ PHÒNG AN TOÀN ---
    println!("\n[Kịch bản 5] Sử dụng unwrap_or để lấy giá trị mặc định an toàn:");
    let result_error: Result<f64, &str> = Err("Mất kết nối máy chủ");
    let num_tien_last_same = result_error.unwrap_or(0.0);
    println!("- Giá trị an toàn thu được: {:.2} VND (không hề bị sập ứng dụng!)", num_tien_last_same);
}
```

---

## Bảng tra cứu lỗi biên dịch & Cách khắc phục (Compiler Error Guide)

Dưới đây là các lỗi kinh điển khi sử dụng cơ chế xử lý lỗi trong Rust:

| Mã lỗi | Thông báo mẫu từ trình biên dịch | Nguyên nhân cốt lõi | Cách khắc phục nhanh |
|---|---|---|---|
| **E0277** | `the '?' operator can only be used in a function that returns 'Result' or 'Option'` | Bạn sử dụng dấu hỏi chấm `?` bên trong một hàm không có kiểu trả về `Result` (ví dụ hàm `fn main()` thông thường mặc định trả về kiểu rỗng `()`). | Sửa kiểu trả về của hàm đang chứa `?` thành `Result<T, E>`. Nếu muốn dùng `?` ngay trong hàm `main`, hãy đổi kiểu trả về của hàm `main` thành `fn main() -> Result<(), std::io::Error>` (khi đọc/ghi dữ liệu I/O) hoặc `fn main() -> Result<(), String>` / `Result<(), MathError>`. |
| **E0308** | `mismatched types: expected 'f64', found 'Result<f64, _>'` | Bạn gọi một hàm trả về `Result` và cố tình gán thẳng vào một biến số thực mà chưa mở hộp `Ok` hay dùng toán tử `?`. | Thêm toán tử `?` ở cuối lời gọi hàm (nếu đang ở trong hàm trả về Result), hoặc dùng `match` / `.unwrap_or(...)`. |
| **Cảnh báo `unused`** | `warning: unused 'Result' that must be used` | Gọi một thao tác có nguy cơ thất bại (như ghi file) nhưng không gán kết quả cho biến nào và không kiểm tra lỗi. | Thêm `let _ = ...` nếu cố ý bỏ qua, hoặc dùng `?` để kiểm tra lỗi đúng quy chuẩn. |
| **E0599** | `no method named 'unwrap' found for type ...` | Bạn gọi `.unwrap()` trên một biến không phải là `Option` hay `Result`. | Kiểm tra lại kiểu dữ liệu của biến trước khi mở gói. |

---

## Tóm tắt chương & Bài tập rèn luyện (Summary & Exercises)

### 4 Điểm cốt lõi cần ghi nhớ:
1. **Hai loại lỗi**: Dùng `panic!` cho các sự cố thảm họa không thể cứu vãn; Dùng `Result<T, E>` cho các tình huống dự đoán trước có thể phục hồi và khắc phục.
2. **Quy trình Stack Unwinding**: Khi panic xảy ra, Rust tự động đi ngược ngăn xếp và gọi `Drop` để dọn sạch sẽ tài nguyên bộ nhớ trước khi thoát.
3. **Nói không với `.unwrap()` bừa bãi**: Trong mã nguồn sản phẩm, thay thế `.unwrap()` bằng `.expect()`, `.unwrap_or()`, hoặc toán tử `?`.
4. **Toán tử `?` diệu kỳ**: Giúp tự động kiểm tra `Err`, return sớm ngay khi gặp sự cố, và bóc tách giá trị `Ok` thành công chỉ trong 1 ký tự duy nhất.

### Bài tập rèn luyện tự giải:
1. **Bài tập thực hành 1**: Viết một hàm `doc_so_tu_chuoi(s: &str) -> Result<i32, String>` nhận vào một chuỗi. Nếu chuỗi có thể chuyển đổi thành số nguyên dương thì trả về `Ok(số)`; nếu số âm hoặc không phải chữ số thì trả về `Err("Số không hợp lệ")`.
2. **Bài tập tái cấu trúc (Refactoring)**: Đoạn mã sau đây đang lạm dụng `.unwrap()` nguy hiểm:
   ```rust
   let s = "42";
   let so: i32 = s.parse().unwrap();
   ```
   Hãy viết lại đoạn mã trên theo 2 cách: Cách 1 dùng `unwrap_or`, Cách 2 dùng cấu trúc `match` để in ra câu thông báo thân thiện nếu người dùng nhập sai.
3. **Bài tập tư duy 3**: Trong Rust, hàm `main` không chỉ trả về kiểu rỗng `()` mà còn có thể trả về một `Result`, ví dụ: `fn main() -> Result<(), std::io::Error>` hoặc `fn main() -> Result<(), String>`. Hãy trả lời hai câu hỏi sau:
   a) Lợi ích của việc cho phép hàm `main` trả về một `Result` là gì? (Gợi ý: điều này giúp chúng ta dùng toán tử `?` ngay trong `main` như thế nào thay vì phải gọi `.expect()` hay `match` liên tục?).
   b) Khi hàm `main` trả về một biến thể `Err(...)`, chương trình Rust sẽ xử lý và hiển thị thông báo lỗi ra màn hình như thế nào so với khi xảy ra `panic!`?
