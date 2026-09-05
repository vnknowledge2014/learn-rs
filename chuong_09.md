# Chương 09: Cấu trúc dữ liệu tự tạo và Phương thức (Structs, Tuples & Associated Functions)

## Giới thiệu & Mục tiêu học tập

Trong các chương trước, bạn đã làm việc với các kiểu dữ liệu đơn lẻ như số nguyên `i32`, số thực `f64`, hay chuỗi ký tự `String`. Tuy nhiên, trong thế giới thực, dữ liệu không bao giờ tồn tại một cách đơn độc. Một "Tài khoản ngân hàng" bao gồm số tài khoản, tên chủ thẻ và số dư. Một "Ngôi nhà" bao gồm địa chỉ, số tầng và diện tích.

Để mô hình hóa các thực thể đời sống một cách mạch lạc và chuyên nghiệp, Rust cung cấp cho chúng ta công cụ mạnh mẽ mang tên **Cấu trúc dữ liệu (Struct)**. Kết hợp với khối hiện thực hành vi **`impl`**, bạn sẽ có thể tạo ra các kiểu dữ liệu hoàn chỉnh vừa nắm giữ thông tin, vừa có những hành động cụ thể.

Mục tiêu học tập của chương này:
- Làm chủ 3 dạng Struct trong Rust: Struct có tên trường (Classic Struct), Struct dạng bộ giá trị (Tuple Struct), và Struct rỗng đánh dấu (Unit-like Struct).
- Hiểu cách tổ chức mã nguồn hướng dữ liệu bằng khối hiện thực hành vi `impl`.
- Phân biệt sâu sắc 3 sắc thái của tham số `self` trong phương thức:
  - `&self`: Mượn chỉ để xem thông tin.
  - `&mut self`: Mượn để cập nhật và sửa đổi trạng thái.
  - `self`: Tiêu thụ hoàn toàn quyền sở hữu để kết thúc vòng đời của đối tượng.
- Nắm vững khái niệm **Hàm liên kết (Associated Functions)** và quy ước thiết kế hàm khởi tạo chuẩn `new()`.
- Sử dụng cú pháp khởi tạo rút gọn (**Field Init Shorthand**) và cú pháp kế thừa cập nhật (**Struct Update Syntax**).

---

## Hình tượng hóa đời sống (Intuitive Everyday Analogy)

Hãy cùng khám phá Struct và Phương thức qua 3 hình ảnh đời thường:

```
┌──────────────────────────────────────────────────────────────────────────────────┐
│                     HÌNH TƯỢNG ĐỜI SỐNG VỀ STRUCT VÀ PHƯƠNG THỨC                 │
├─────────────────────────┬───────────────────────────────┬────────────────────────┤
│   THẺ CĂN CƯỚC CÔNG DÂN │      CHIẾC THẺ ATM NGÂN HÀNG  │    3 CÁCH SỬ DỤNG THẺ  │
│        (Classic Struct) │         (Khối hành vi impl)   │        (&self, &mut, self)│
│                         │                               │                        │
│ - Tên trường rõ ràng    │ - Dữ liệu: Số thẻ, Số dư      │ - Xem số dư: &self     │
│ - Tên: Nguyễn Văn A     │ - Hành vi đi kèm:             │   (Chỉ đọc màn hình)   │
│ - Năm sinh: 2000        │   + Xem số dư                 │ - Nạp/Rút tiền: &mut   │
│ - Không thể nhầm lẫn    │   + Nạp tiền, Rút tiền        │   (Thay đổi số dư)     │
│   giữa tên và năm sinh  │   + Hủy thẻ vĩnh viễn         │ - Cắt hủy thẻ: self    │
│                         │                               │   (Tiêu hủy đối tượng) │
└─────────────────────────┴───────────────────────────────┴────────────────────────┘
```

### 1. Thẻ Căn cước công dân gắn chip (Struct có tên trường)
Khi bạn cầm trên tay chiếc thẻ Căn cước công dân:
- Trên mặt thẻ có các ô thông tin được in tiêu đề rõ ràng: "Họ và tên", "Ngày tháng năm sinh", "Quê quán", "Số căn cước".
- Nhờ có tên trường rõ ràng, bạn không bao giờ bị nhầm lẫn giữa năm sinh và số căn cước dù cả hai đều có thể biểu diễn bằng những con số.

### 2. Chiếc hộp 3 thỏi màu vẽ tranh (Tuple Struct `MauSac(u8, u8, u8)`)
Khi bạn mua một hộp bút vẽ cơ bản gồm 3 màu: Đỏ (Red), Lục (Green), Lam (Blue):
- Bạn không cần dán nhãn dài dòng lên từng ngăn hộp. Mọi người đều ngầm hiểu ngăn số 0 là màu Đỏ, ngăn số 1 là màu Lục, ngăn số 2 là màu Lam.
- Trong Rust, khi bạn cần gom 2-3 đại lượng liên quan mật thiết (như tọa độ không gian `Diem(x, y)` hay mã màu RGB) mà không cần đặt tên cho từng trường, bạn sử dụng **Tuple Struct**.

### 3. Thẻ ATM và 3 cách sử dụng tại cây rút tiền (`&self`, `&mut self`, `self`)
Chiếc thẻ ATM của bạn lưu giữ số dư tiền bạc. Khối `impl` cung cấp cho bạn 3 thao tác:
- **Xem số dư (`&self`)**: Bạn đưa thẻ vào máy ATM, bấm nút "Truy vấn số dư". Máy đọc thông tin trên chip và hiện số tiền lên màn hình. Số dư trong tài khoản không thay đổi, thẻ của bạn vẫn còn nguyên vẹn trong tay.
- **Rút tiền (`&mut self`)**: Bạn rút 500.000 VND. Máy ATM cập nhật số dư mới vào hệ thống. Thẻ vẫn thuộc quyền sở hữu của bạn, nhưng dữ liệu bên trong đã bị biến đổi.
- **Tất toán đóng tài khoản vĩnh viễn (`self`)**: Bạn yêu cầu hủy dịch vụ. Nhân viên ngân hàng cầm chiếc kéo cắt đôi chiếc thẻ nhựa trước mắt bạn và ném vào sọt rác. Chiếc thẻ bị tiêu hủy hoàn toàn, bạn không thể mang chiếc thẻ đó ra cây ATM quẹt thêm lần nào nữa!

---

## Khái niệm & Cơ chế kỹ thuật chuyên sâu (In-Depth Technical Mechanics)

### 1. Bố cục ô nhớ và Sự sắp xếp tối ưu của Trình biên dịch (Memory Layout & Alignment)

Khi bạn định nghĩa một Struct:
```rust
struct GoiHang {
    dang_giao: bool, // 1 byte
    khoi_luong: f64, // 8 bytes
    ma_so: u8,       // 1 byte
}
```
CPU của máy tính hiện đại không đọc từng byte đơn lẻ từ RAM một cách chậm chạp. Nó đọc theo từng khối **4 bytes hoặc 8 bytes** (gọi là Memory Word) để đạt tốc độ tối đa.

Nếu các trường dữ liệu nằm lệch nhịp bộ nhớ, CPU sẽ phải mất 2 lần đọc và ghép nối, làm giảm tốc độ thực thi. Để khắc phục điều này:
- Trình biên dịch `rustc` tự động tính toán khoảng đệm (**Data Padding**).
- Thậm chí, Rust còn tự động **đảo lại thứ tự các trường trên RAM** (Field Reordering) để gom các trường nhỏ cạnh nhau mà bạn không cần bận tâm, giúp struct chiếm ít dung lượng RAM nhất có thể!

### 2. Ba sắc thái của tham số `self` trong khối `impl`

Trong khối `impl TenStruct`, các hàm có tham số đầu tiên là `self` được gọi là **Phương thức (Methods)**:

```rust
impl TaiKhoan {
    // 1. Tham chiếu bất biến: Chỉ đọc dữ liệu (Borrow immutable)
    fn xem_so_du(&self) -> f64 { self.so_du }

    // 2. Tham chiếu khả biến: Cho phép chỉnh sửa trạng thái (Borrow mutable)
    fn nap_tien(&mut self, tien: f64) { self.so_du += tien; }

    // 3. Quyền sở hữu độc quyền: Tiêu thụ và hủy đối tượng (Take ownership & Drop)
    fn dong_tai_khoan(self) {
        println!("Tài khoản của {} đã chính thức bị đóng vĩnh viễn!", self.ten);
        // Khi hàm này kết thúc, self đi ra khỏi scope và bị giải phóng!
    }
}
```

### 3. Hàm liên kết (Associated Functions) và Mẫu hàm khởi tạo `new()`

Nếu một hàm nằm trong khối `impl` nhưng **không có tham số `self`**, nó được gọi là một **Hàm liên kết (Associated Function)**:
- Nó không gắn liền với một đối tượng cụ thể nào, mà gắn liền với chính kiểu dữ liệu đó.
- Để gọi hàm này, chúng ta sử dụng dấu hai chấm kép `::` (ví dụ: `String::from("...")` chính là một hàm liên kết của kiểu `String`!).
- Quy ước chuẩn mực của Rust là dùng hàm liên kết `new()` hoặc `tao_moi()` để làm hàm khởi tạo (Constructor), kiểm tra tính hợp lệ của dữ liệu trước khi bàn giao đối tượng cho người dùng.

### 4. Cú pháp khởi tạo rút gọn và Cập nhật Struct

- **Khởi tạo rút gọn (Field Init Shorthand)**: Khi tên biến trùng với tên trường của struct:
  ```rust
  let ten = String::from("An");
  let tk = TaiKhoan { ten, so_du: 100.0 }; // Thay vì phải viết ten: ten
  ```
- **Cập nhật Struct (Struct Update Syntax `..`)**: Tạo một struct mới dựa trên struct cũ và chỉ thay đổi một vài trường:
  ```rust
  let tk2 = TaiKhoan {
      so_du: 500.0,
      ..tk1 // Tất cả các trường còn lại sao chép hoặc move từ tk1!
  };
  ```

---

## Mã nguồn minh họa thực chiến (Idiomatic Runnable Rust Blueprint)

Chương trình hoàn chỉnh dưới đây mô phỏng một hệ thống quản lý tài khoản ngân hàng điện tử, bao gồm Struct có tên trường, Tuple Struct, Unit Struct và đầy đủ các loại phương thức:

```rust
// File: src/main.rs
// Chương trình làm chủ Structs, Tuples & Phương thức trong Rust

// 1. Tuple Struct: Biểu diễn tọa độ GPS của trụ sở ngân hàng (Kinh độ, Vĩ độ)
struct ToaDoGps(f64, f64);

// 2. Unit-like Struct: Đóng vai trò như một nhãn chứng thực bảo mật giao dịch
struct ChungThucBaoMat;

// 3. Classic Struct: Định nghĩa cấu trúc tài khoản ngân hàng hoàn chỉnh
struct TaiKhoanNganHang {
    so_tai_khoan: String,
    chu_tai_khoan: String,
    so_du: f64,
    kich_hoat: bool,
}

// Khối hiện thực các phương thức và hàm liên kết cho TaiKhoanNganHang
impl TaiKhoanNganHang {
    // A. HÀM LIÊN KẾT (Associated Function) - Khởi tạo tài khoản mới chuẩn mực
    fn mo_tai_khoan(so_tk: String, chu_tk: String, so_du_dau: f64) -> Self {
        println!("-> Đang mở tài khoản mới cho khách hàng: {}", chu_tk);
        Self {
            so_tai_khoan: so_tk,
            chu_tai_khoan: chu_tk,
            so_du: so_du_dau,
            kich_hoat: true,
        }
    }

    // B. PHƯƠNG THỨC MƯỢN ĐỌC (&self): Tra cứu thông tin số dư an toàn
    fn tra_cuu_thong_tin(&self) {
        println!("------------------------------------------------------------");
        println!("Số tài khoản : {}", self.so_tai_khoan);
        println!("Chủ tài khoản: {}", self.chu_tai_khoan);
        println!("Số dư hiện có: {:.2} VND", self.so_du);
        println!("Trạng thái   : {}", if self.kich_hoat { "Hoạt động" } else { "Đã khóa" });
        println!("------------------------------------------------------------");
    }

    // C. PHƯƠNG THỨC MƯỢN SỬA (&mut self): Nạp tiền vào tài khoản
    fn nap_tien(&mut self, so_tien: f64) {
        if so_tien <= 0.0 {
            println!("[!] Lỗi: Số tiền nạp phải lớn hơn 0!");
            return;
        }
        self.so_du += so_tien;
        println!("-> Nạp thành công {:.2} VND vào tài khoản {}", so_tien, self.so_tai_khoan);
    }

    // D. PHƯƠNG THỨC MƯỢN SỬA (&mut self): Rút tiền có kiểm tra số dư
    fn rut_tien(&mut self, so_tien: f64) -> bool {
        if so_tien > self.so_du {
            println!("[!] Giao dịch thất bại: Số dư không đủ để rút {:.2} VND!", so_tien);
            false
        } else {
            self.so_du -= so_tien;
            println!("-> Rút thành công {:.2} VND. Số dư còn lại: {:.2} VND", so_tien, self.so_du);
            true
        }
    }

    // E. PHƯƠNG THỨC TIÊU THỤ SỞ HỮU (self): Đóng tài khoản vĩnh viễn
    fn tat_toan_va_dong_so(self) {
        println!("\n*** TIẾN HÀNH TẤT TOÁN VÀ HỦY TÀI KHOẢN ***");
        println!("- Hoàn trả toàn bộ số dư cuối cùng: {:.2} VND cho ông/bà {}", 
                 self.so_du, self.chu_tai_khoan);
        println!("- Tài khoản số {} đã bị đóng và giải phóng khỏi hệ thống.", self.so_tai_khoan);
        // Khi hàm này kết thúc, self bị Drop ngay tại đây!
    }
}

fn main() {
    println!("============================================================");
    println!("     HỆ THỐNG QUẢN LÝ TÀI KHOẢN NGÂN HÀNG ĐIỆN TỬ RUST      ");
    println!("============================================================");

    // Sử dụng Tuple Struct để lưu tọa độ chi nhánh ngân hàng
    let chi_nhanh_ha_noi = ToaDoGps(21.0285, 105.8542);
    println!("Tọa độ chi nhánh giao dịch: Vĩ độ {}, Kinh độ {}", 
             chi_nhanh_ha_noi.0, chi_nhanh_ha_noi.1);

    // Khởi tạo Unit-like Struct làm chứng thực an toàn cho phiên làm việc
    let _chung_thuc_phien = ChungThucBaoMat;
    println!("Chứng thực bảo mật hệ thống: Đã kích hoạt tem xác thực điện tử.");

    // Mở một tài khoản ngân hàng mới thông qua hàm liên kết mo_tai_khoan
    let mut tk_an = TaiKhoanNganHang::mo_tai_khoan(
        String::from("1900-123-456"),
        String::from("Nguyễn Văn An"),
        1_000_000.0,
    );

    // Tra cứu thông tin (gọi phương thức &self)
    tk_an.tra_cuu_thong_tin();

    // Thực hiện các giao dịch làm biến đổi số dư (gọi phương thức &mut self)
    tk_an.nap_tien(500_000.0);
    tk_an.rut_tien(200_000.0);
    tk_an.rut_tien(2_000_000.0); // Thử rút vượt số dư

    // Tra cứu lại thông tin sau giao dịch
    tk_an.tra_cuu_thong_tin();

    // Minh họa Cú pháp cập nhật Struct (Struct Update Syntax ..)
    let tk_phu = TaiKhoanNganHang {
        so_tai_khoan: String::from("1900-999-888"),
        so_du: 50_000.0,
        ..TaiKhoanNganHang::mo_tai_khoan(
            String::from("TEMP"),
            String::from("Nguyễn Văn An (Tài khoản tiết kiệm)"),
            0.0
        )
    };
    println!("\nTài khoản phụ được tạo tự động:");
    tk_phu.tra_cuu_thong_tin();

    // Đóng tài khoản chính (gọi phương thức tiêu thụ self)
    tk_an.tat_toan_va_dong_so();

    // NẾU BẠN BỎ CHÚ THÍCH DÒNG SAU, RUSTC SẼ BÁO LỖI E0382 NGAY:
    // tk_an.tra_cuu_thong_tin(); // LỖI: Giá trị tk_an đã bị tiêu thụ khi đóng sổ!
}
```

---

## Bảng tra cứu lỗi biên dịch & Cách khắc phục (Compiler Error Guide)

Dưới đây là các lỗi thường gặp khi làm việc với Structs và Phương thức:

| Mã lỗi | Thông báo mẫu từ trình biên dịch | Nguyên nhân cốt lõi | Cách khắc phục nhanh |
|---|---|---|---|
| **E0599** | `no method named 'rut_tien' found for struct 'TaiKhoan' in the current scope` | Bạn gọi một phương thức chưa được khai báo trong khối `impl`, hoặc gõ sai chính tả tên hàm. | Kiểm tra lại tên phương thức trong khối `impl` và đảm bảo kiểu dữ liệu gọi phương thức là chính xác. |
| **E0596** | `cannot borrow 'tk' as mutable, as it is not declared as mutable` | Bạn gọi phương thức đòi hỏi `&mut self` (như `nap_tien`) trên một đối tượng struct khai báo bất biến (`let tk = ...`). | Thêm từ khóa `mut` khi tạo biến: `let mut tk = ...`. |
| **E0382** | `use of moved value: 'tk'` | Bạn gọi một phương thức nhận `self` (tiêu thụ đối tượng), sau đó lại cố sử dụng tiếp biến đó ở các dòng sau. | Đổi tham số phương thức thành `&self` hoặc `&mut self` nếu không muốn hủy đối tượng, hoặc tạo bản sao trước khi tiêu thụ. |
| **E0063** | `missing field 'kich_hoat' in initializer of 'TaiKhoanNganHang'` | Bạn khởi tạo Struct nhưng quên chưa điền giá trị cho một trong các trường dữ liệu. | Điền đầy đủ tất cả các trường, hoặc sử dụng cú pháp cập nhật `..struct_cu` để lấy giá trị mặc định cho các trường còn lại. |

---

## Tóm tắt chương & Bài tập rèn luyện (Summary & Exercises)

### 4 Điểm cốt lõi cần ghi nhớ:
1. **Ba dạng Struct**: Classic Struct (có tên trường rõ ràng), Tuple Struct (định danh theo vị trí chỉ số `.0`, `.1`), và Unit Struct (không chứa dữ liệu, dùng làm cờ hiệu).
2. **Khối hiện thực `impl`**: Nơi gắn kết logic hành vi trực tiếp vào dữ liệu, tạo nên cấu trúc mã nguồn hướng dữ liệu chuẩn mực.
3. **Ba sắc thái của `self`**: `&self` để đọc dữ liệu không làm mất quyền sở hữu; `&mut self` để cập nhật trạng thái; `self` để tiêu thụ và kết thúc vòng đời của đối tượng.
4. **Hàm liên kết `new()`**: Hàm không có tham số `self`, được gọi qua dấu `::` và đóng vai trò như hàm khởi tạo kiểm tra tính hợp lệ của dữ liệu trước khi tạo lập Struct.

### Bài tập rèn luyện tự giải:
1. **Bài tập thực hành 1**: Tạo một `struct HinhChuNhat` gồm hai trường `chieu_dai: f64` và `chieu_rong: f64`. Trong khối `impl`, viết:
   - Hàm liên kết `tao_moi(dai: f64, rong: f64) -> Self`.
   - Phương thức `tinh_dien_tich(&self) -> f64`.
   - Phương thức `tinh_chu_vi(&self) -> f64`.
   - Phương thức `co_phai_hinh_vuong(&self) -> bool`.
2. **Bài tập tư duy 2**: Tại sao Rust lại hỗ trợ phương thức tiêu thụ `self` (chuyển giao quyền sở hữu)? Hãy nêu một tình huống thực tế (ví dụ: gửi một bức thư điện tử hoặc đốt một que diêm) mà phương thức `self` giúp ngăn chặn người dùng sử dụng lại đối tượng đã hết giá trị.
3. **Bài tập Tuple Struct 3**: Định nghĩa một Tuple Struct mang tên `DonHang(u64, u64, u64)` đại diện cho 3 thành phần chi phí của một đơn hàng mua sắm: (tiền hàng, phí giao hàng, phụ phí đóng gói). Trong khối `impl`, viết phương thức `tinh_tong_thanh_toan(&self) -> u64` cộng tổng cả 3 khoản chi phí lại (truy xuất qua chỉ số `.0`, `.1`, `.2`). Trong hàm `main`, hãy khởi tạo một đơn hàng mẫu (ví dụ: tiền hàng 250.000đ, phí ship 30.000đ, đóng gói 10.000đ) và in ra tổng số tiền thực tế khách cần thanh toán.
