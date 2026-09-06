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
struct Parcel {
    in_transit: bool, // 1 byte
    quantity: f64, // 8 bytes
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
impl Account {
    // 1. Tham chiếu bất biến: Chỉ đọc dữ liệu (Borrow immutable)
    fn show_balance(&self) -> f64 { self.balance }

    // 2. Tham chiếu khả biến: Cho phép chỉnh sửa trạng thái (Borrow mutable)
    fn nap_tien(&mut self, tien: f64) { self.balance += tien; }

    // 3. Quyền sở hữu độc quyền: Tiêu thụ và hủy đối tượng (Take ownership & Drop)
    fn close_account(self) {
        println!("Tài khoản của {} đã chính thức bị đóng vĩnh viễn!", self.name);
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
  let name = String::from("An");
  let account = Account { name, balance: 100.0 }; // Thay vì phải viết ten: ten
  ```
- **Cập nhật Struct (Struct Update Syntax `..`)**: Tạo một struct mới dựa trên struct cũ và chỉ thay đổi một vài trường:
  ```rust
  let tk2 = Account {
      balance: 500.0,
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
struct GpsCoord(f64, f64);

// 2. Unit-like Struct: Đóng vai trò như một nhãn chứng thực bảo mật deliver dịch
struct LostReport;

// 3. Classic Struct: Định nghĩa cấu trúc tài khoản ngân hàng hoàn chỉnh
struct AccountBank {
    num_account: String,
    account_owner: String,
    balance: f64,
    activate: bool,
}

// Khối hiện thực các phương thức và hàm liên kết cho AccountBank
impl AccountBank {
    // A. HÀM LIÊN KẾT (Associated Function) - Khởi tạo tài khoản mới chuẩn mực
    fn open_account(so_tk: String, chu_tk: String, so_du_dau: f64) -> Self {
        println!("-> Đang mở tài khoản mới cho khách hàng: {}", chu_tk);
        Self {
            num_account: so_tk,
            account_owner: chu_tk,
            balance: so_du_dau,
            activate: true,
        }
    }

    // B. PHƯƠNG THỨC MƯỢN ĐỌC (&self): Tra cứu thông tin số dư an toàn
    fn tra_cuu_thong_tin(&self) {
        println!("------------------------------------------------------------");
        println!("Số tài khoản : {}", self.num_account);
        println!("Chủ tài khoản: {}", self.account_owner);
        println!("Số dư hiện có: {:.2} VND", self.balance);
        println!("Trạng thái   : {}", if self.activate { "Hoạt động" } else { "Đã khóa" });
        println!("------------------------------------------------------------");
    }

    // C. PHƯƠNG THỨC MƯỢN SỬA (&mut self): Nạp tiền vào tài khoản
    fn nap_tien(&mut self, so_tien: f64) {
        if so_tien <= 0.0 {
            println!("[!] Lỗi: Số tiền nạp phải lớn hơn 0!");
            return;
        }
        self.balance += so_tien;
        println!("-> Nạp thành công {:.2} VND vào tài khoản {}", so_tien, self.num_account);
    }

    // D. PHƯƠNG THỨC MƯỢN SỬA (&mut self): Rút tiền có kiểm tra số dư
    fn rut_tien(&mut self, so_tien: f64) -> bool {
        if so_tien > self.balance {
            println!("[!] Giao dịch thất bại: Số dư không đủ để rút {:.2} VND!", so_tien);
            false
        } else {
            self.balance -= so_tien;
            println!("-> Rút thành công {:.2} VND. Số dư còn lại: {:.2} VND", so_tien, self.balance);
            true
        }
    }

    // E. PHƯƠNG THỨC TIÊU THỤ SỞ HỮU (self): Đóng tài khoản vĩnh viễn
    fn all_math_and_round(self) {
        println!("\n*** TIẾN HÀNH TẤT TOÁN VÀ HỦY TÀI KHOẢN ***");
        println!("- Hoàn trả toàn bộ số dư cuối cùng: {:.2} VND cho ông/bà {}", 
                 self.balance, self.account_owner);
        println!("- Tài khoản số {} đã bị đóng và giải phóng khỏi hệ thống.", self.num_account);
        // Khi hàm này kết thúc, self bị Drop ngay tại đây!
    }
}

fn main() {
    println!("============================================================");
    println!("     HỆ THỐNG QUẢN LÝ TÀI KHOẢN NGÂN HÀNG ĐIỆN TỬ RUST      ");
    println!("============================================================");

    // Sử dụng Tuple Struct để lưu tọa độ chi nhánh ngân hàng
    let hanoi_branch = GpsCoord(21.0285, 105.8542);
    println!("Tọa độ chi nhánh deliver dịch: Vĩ độ {}, Kinh độ {}", 
             hanoi_branch.0, hanoi_branch.1);

    // Khởi tạo Unit-like Struct làm chứng thực an toàn cho phiên làm việc
    let _auth_session = LostReport;
    println!("Chứng thực bảo mật hệ thống: Đã kích hoạt tem xác thực điện tử.");

    // Mở một tài khoản ngân hàng mới thông qua hàm liên kết open_account
    let mut account_hidden = AccountBank::open_account(
        String::from("1900-123-456"),
        String::from("Nguyễn Văn An"),
        1_000_000.0,
    );

    // Tra cứu thông tin (gọi phương thức &self)
    account_hidden.tra_cuu_thong_tin();

    // Thực hiện các deliver dịch làm biến đổi số dư (gọi phương thức &mut self)
    account_hidden.nap_tien(500_000.0);
    account_hidden.rut_tien(200_000.0);
    account_hidden.rut_tien(2_000_000.0); // Thử rút vượt số dư

    // Tra cứu lại thông tin sau deliver dịch
    account_hidden.tra_cuu_thong_tin();

    // Minh họa Cú pháp cập nhật Struct (Struct Update Syntax ..)
    let account_aux = AccountBank {
        num_account: String::from("1900-999-888"),
        balance: 50_000.0,
        ..AccountBank::open_account(
            String::from("TEMP"),
            String::from("Nguyễn Văn An (Tài khoản tiết kiệm)"),
            0.0
        )
    };
    println!("\nTài khoản phụ được tạo tự động:");
    account_aux.tra_cuu_thong_tin();

    // Đóng tài khoản chính (gọi phương thức tiêu thụ self)
    account_hidden.all_math_and_round();

    // NẾU BẠN BỎ CHÚ THÍCH DÒNG SAU, RUSTC SẼ BÁO LỖI E0382 NGAY:
    // account_hidden.tra_cuu_thong_tin(); // LỖI: Giá trị account_hidden đã bị tiêu thụ khi đóng sổ!
}
```

---

## Bảng tra cứu lỗi biên dịch & Cách khắc phục (Compiler Error Guide)

Dưới đây là các lỗi thường gặp khi làm việc với Structs và Phương thức:

| Mã lỗi | Thông báo mẫu từ trình biên dịch | Nguyên nhân cốt lõi | Cách khắc phục nhanh |
|---|---|---|---|
| **E0599** | `no method named 'rut_tien' found for struct 'Account' in the current scope` | Bạn gọi một phương thức chưa được khai báo trong khối `impl`, hoặc gõ sai chính tả tên hàm. | Kiểm tra lại tên phương thức trong khối `impl` và đảm bảo kiểu dữ liệu gọi phương thức là chính xác. |
| **E0596** | `cannot borrow 'tk' as mutable, as it is not declared as mutable` | Bạn gọi phương thức đòi hỏi `&mut self` (như `nap_tien`) trên một đối tượng struct khai báo bất biến (`let tk = ...`). | Thêm từ khóa `mut` khi tạo biến: `let mut tk = ...`. |
| **E0382** | `use of moved value: 'tk'` | Bạn gọi một phương thức nhận `self` (tiêu thụ đối tượng), sau đó lại cố sử dụng tiếp biến đó ở các dòng sau. | Đổi tham số phương thức thành `&self` hoặc `&mut self` nếu không muốn hủy đối tượng, hoặc tạo bản sao trước khi tiêu thụ. |
| **E0063** | `missing field 'activate' in initializer of 'AccountBank'` | Bạn khởi tạo Struct nhưng quên chưa điền giá trị cho một trong các trường dữ liệu. | Điền đầy đủ tất cả các trường, hoặc sử dụng cú pháp cập nhật `..struct_cu` để lấy giá trị mặc định cho các trường còn lại. |

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
3. **Bài tập Tuple Struct 3**: Định nghĩa một Tuple Struct mang tên `DonQueue(u64, u64, u64)` đại diện cho 3 thành phần chi phí của một đơn hàng mua sắm: (tiền hàng, phí giao hàng, phụ phí đóng gói). Trong khối `impl`, viết phương thức `tinh_tong_thanh_toan(&self) -> u64` cộng tổng cả 3 khoản chi phí lại (truy xuất qua chỉ số `.0`, `.1`, `.2`). Trong hàm `main`, hãy khởi tạo một đơn hàng mẫu (ví dụ: tiền hàng 250.000đ, phí ship 30.000đ, đóng gói 10.000đ) và in ra tổng số tiền thực tế khách cần thanh toán.

---

### Gợi ý & Lời giải

<details>
<summary><b>Bài tập 1 — Gợi ý</b></summary>

`impl` gom các phương thức của struct. Hàm liên kết `tao_moi` không có `self` (gọi qua `HinhChuNhat::tao_moi`), các phương thức còn lại nhận `&self` để đọc dữ liệu.
</details>

<details>
<summary><b>Bài tập 1 — Lời giải</b></summary>

```rust
struct HinhChuNhat {
    chieu_dai: f64,
    chieu_rong: f64,
}

impl HinhChuNhat {
    // Hàm LIÊN KẾT: không có self, đóng vai trò "nhà xây dựng".
    fn tao_moi(dai: f64, rong: f64) -> Self {
        Self { chieu_dai: dai, chieu_rong: rong }
    }
    // Các PHƯƠNG THỨC: nhận &self để đọc dữ liệu mà không lấy quyền sở hữu.
    fn tinh_dien_tich(&self) -> f64 {
        self.chieu_dai * self.chieu_rong
    }
    fn tinh_chu_vi(&self) -> f64 {
        2.0 * (self.chieu_dai + self.chieu_rong)
    }
    fn co_phai_hinh_vuong(&self) -> bool {
        self.chieu_dai == self.chieu_rong
    }
}

fn main() {
    let hcn = HinhChuNhat::tao_moi(5.0, 3.0);
    println!("Diện tích {}, chu vi {}", hcn.tinh_dien_tich(), hcn.tinh_chu_vi());
    println!("Là hình vuông? {}", hcn.co_phai_hinh_vuong());
}

#[test]
fn tinh_toan_hinh_chu_nhat() {
    let hcn = HinhChuNhat::tao_moi(5.0, 3.0);
    assert_eq!(hcn.tinh_dien_tich(), 15.0);
    assert_eq!(hcn.tinh_chu_vi(), 16.0);
    assert!(!hcn.co_phai_hinh_vuong());
    assert!(HinhChuNhat::tao_moi(4.0, 4.0).co_phai_hinh_vuong());
}
```

Điểm phân biệt cốt lõi: **hàm liên kết** (`tao_moi`, không có `self`) gọi qua `TênKiểu::ham()` và thường dùng làm nhà xây dựng; **phương thức** (có `&self`) gọi qua `bien.phuong_thuc()`. Cả bốn dùng `&self` (mượn đọc) là đúng — chúng chỉ cần *đọc* kích thước để tính, không cần sửa và cũng không nên nuốt mất đối tượng.
</details>

<details>
<summary><b>Bài tập 2 — Gợi ý</b></summary>

Phương thức nhận `self` (không phải `&self`) **nuốt** đối tượng — sau khi gọi, biến cũ không dùng lại được. Nghĩ tới hành động **chỉ làm được một lần** rồi vật thể không còn nguyên vẹn.
</details>

<details>
<summary><b>Bài tập 2 — Lời giải</b></summary>

Rust hỗ trợ phương thức tiêu thụ `self` để mô hình hóa những hành động **dùng một lần rồi hết** — biến quy tắc nghiệp vụ thành thứ trình biên dịch ép được.

**Ví dụ que diêm:**
```rust
struct QueDiem { con_dau: bool }
impl QueDiem {
    fn dot(self) -> String {   // self, KHÔNG phải &self -> nuốt luôn que diêm
        String::from("Bùng cháy!")
    }
}
let que = QueDiem { con_dau: true };
let lua = que.dot();
// que.dot();   // <- LỖI BIÊN DỊCH: que đã bị nuốt ở lần đốt trước
```

Sau `que.dot()`, biến `que` bị **di chuyển vào hàm và hủy** — dòng đốt lần hai *không biên dịch được*. Đây đúng bản chất thực tế: một que diêm cháy rồi thì không đốt lại được. Cũng vậy với **gửi thư điện tử**: `fn gui(self)` nuốt đối tượng thư, nên bạn không thể vô tình `gui()` hai lần cùng một bức thư — tránh gửi trùng.

Giá trị nằm ở chỗ: quy tắc "đối tượng này chỉ dùng được một lần" thường chỉ nằm trong tài liệu hoặc đầu người lập trình. Phương thức `self` **nâng nó thành ràng buộc kiểu**: mọi lần dùng lại đều bị chặn ngay lúc biên dịch, không đợi tới lúc chạy mới phát hiện.
</details>

<details>
<summary><b>Bài tập 3 — Gợi ý</b></summary>

Tuple struct đặt tên cho một bộ giá trị nhưng truy xuất qua chỉ số `.0`, `.1`, `.2` thay vì tên trường.
</details>

<details>
<summary><b>Bài tập 3 — Lời giải</b></summary>

```rust
// Tuple struct: có tên kiểu (DonQueue) nhưng trường truy xuất bằng chỉ số.
struct DonQueue(u64, u64, u64); // (tiền hàng, phí giao, phụ phí đóng gói)

impl DonQueue {
    fn tinh_tong_thanh_toan(&self) -> u64 {
        self.0 + self.1 + self.2   // truy xuất qua .0 .1 .2, không có tên trường
    }
}

fn main() {
    let don = DonQueue(250_000, 30_000, 10_000);
    println!("Tổng thanh toán: {}đ", don.tinh_tong_thanh_toan());
}

#[test]
fn tong_ba_khoan_chi_phi() {
    let don = DonQueue(250_000, 30_000, 10_000);
    assert_eq!(don.tinh_tong_thanh_toan(), 290_000);
}
```

Tuple struct hợp khi bạn muốn một **kiểu riêng có tên** (để trình biên dịch phân biệt `DonQueue` với một `(u64,u64,u64)` bất kỳ) nhưng bản thân các trường đã rõ nghĩa theo thứ tự, không cần đặt tên. Đánh đổi: gọn hơn struct thường, nhưng `.0/.1/.2` kém tự mô tả — nhầm thứ tự tiền hàng và phí ship là lỗi âm thầm. Quy tắc thực dụng: ít trường và thứ tự hiển nhiên thì dùng tuple struct; nhiều trường hoặc dễ lẫn thì đặt tên trường.
</details>
