# Chương 24: Chế tạo Macro: Custom Derive, Thuộc tính và Macro dạng hàm (Custom Derive, Attribute & Function-like Macros)

## Giới thiệu & Mục tiêu học tập

Chúc mừng bạn đã tiến bước đến chương đỉnh cao của Chủ đề 4: Siêu lập trình (Meta Programming)! Ở Chương 23, bạn đã làm quen với các khái niệm nền tảng của Macro thủ tục: Cây cú pháp trừu tượng (AST), kính hiển vi bóc tách `syn`, và cây bút ma thuật sinh mã `quote`.

Trong thế giới Rust thực chiến, Macro thủ tục không chỉ gói gọn trong một hình thức duy nhất mà được chia thành **Ba nhánh sức mạnh tối thượng (The Trinity of Procedural Macros)**:
1. **Custom Derive Macro (`#[derive(TenTrait)]`)**: Tự động sinh mã triển khai một Trait cho struct hoặc enum mà không làm biến đổi mã gốc.
2. **Attribute-like Macro (`#[ten_thuoc_tinh]`)**: Gắn lên đầu hàm, struct hay mô-đun để can thiệp, biến đổi hoặc bọc lớp vỏ bảo vệ quanh đối tượng (như cách `#[tokio::main]` biến một hàm bất đồng bộ thành luồng chạy thực tế).
3. **Function-like Macro (`ten_macro!(...)`)**: Nhận một ngôn ngữ đặc thù (DSL) tùy ý bên trong dấu ngoặc và dịch nó thành mã Rust chuẩn mực (như cách `sqlx::query!` kiểm tra cú pháp câu lệnh SQL ngay lúc biên dịch).

Mục tiêu học tập của chương này:
- Làm chủ sự khác biệt về chữ ký hàm, quyền hạn và phạm vi ứng dụng của **Cả 3 loại Macro thủ tục**.
- Xây dựng một **Custom Derive Macro** hoàn chỉnh với thuộc tính bổ trợ (**Helper Attributes**).
- Thấu hiểu cơ chế biến đổi mã nguồn của **Attribute-like Macro** (`attr: TokenStream, item: TokenStream`).
- Nắm vững cách xây dựng **Function-like Procedural Macro** để sáng tạo ngôn ngữ miền chuyên biệt (DSL).
- Áp dụng các kỹ thuật gỡ lỗi siêu lập trình chuyên nghiệp với `eprintln!` lúc biên dịch và công cụ `cargo expand`.

---

## Hình tượng hóa đời sống (Intuitive Everyday Analogy)

Hãy cùng hình dung ba loại Macro thủ tục qua ba hình ảnh quen thuộc trong quy trình quản lý chất lượng hàng hóa:

```
┌──────────────────────────────────────────────────────────────────────────────────┐
│              HÌNH TƯỢNG ĐỜI SỐNG: BA CÔNG CỤ QUẢN LÝ CHẤT LƯỢNG HÀNG HÓA         │
├─────────────────────────┬───────────────────────────────┬────────────────────────┤
│    CON DẤU KIỂM DỊCH    │     TEM CẢM BIẾN NHIỆT ĐỘ     │     MÁY DỊCH NGOẠI TỆ  │
│      (Custom Derive)    │     (Attribute-like Macro)    │     (Function-like)    │
│                         │                               │                        │
│ - Đóng dấu cộp lên thùng│ - Dán chiếc tem thông minh lên│ - Cho vào máy một tờ   │
│   nông sản xuất khẩu    │   thân thùng hàng đông lạnh   │   tiền giấy nước ngoài │
│ - Thùng hàng gốc nguyên │ - Chiếc tem tự động đo nhiệt, │ - Máy tự động kiểm tra │
│   vẹn, không bị đập vỡ  │   bật còi hú nếu nhiệt độ cao │   tiền giả và đổi ra   │
│ - Thùng hàng được cấp   │ - Biến đổi hoàn toàn cách thức│   tiền nội tệ tương ứng│
│   thêm quyền thông quan!│   bảo quản của kiện hàng!     │ - Tạo cú pháp hoàn toàn│
│ -> Chỉ bổ sung tính năng│ -> Biến đổi / Bọc hành vi gốc │   mới cho ngôn ngữ!    │
└─────────────────────────┴───────────────────────────────┴────────────────────────┘
```

### 1. Con dấu kiểm dịch xuất khẩu (Custom Derive Macro)
- Khi bạn có một thùng hàng táo tươi xuất khẩu (`struct TaoTuoi`):
  - Viên chức kiểm định đóng một con dấu đỏ "Đạt chuẩn an toàn thực phẩm" lên vỏ thùng (`#[derive(KiemDinh)]`).
  - Thùng táo bên trong không hề bị băm nhỏ hay sửa đổi. Nhưng nhờ con dấu đó, thùng táo tự động được cấp thêm một tập hồ sơ pháp lý cho phép nó thông quan qua cảng biển (`impl XuatKhau for TaoTuoi`).
  - **Quy tắc**: Derive macro **không bao giờ sửa đổi mã gốc**, nó chỉ sinh thêm mã triển khai Trait bên cạnh mã gốc.

### 2. Chiếc tem cảm biến nhiệt độ thông minh (Attribute-like Macro)
- Khi bạn dán một chiếc tem điện tử thông minh lên thùng vắc-xin (`#[bao_ve_nhiet_do]`):
  - Chiếc tem này can thiệp trực tiếp vào quy trình: Bất cứ ai mở thùng hàng ra, chiếc tem sẽ tự động ghi lại nhật ký thời gian và nhiệt độ môi trường.
  - Mã nguồn ban đầu của hàm bị "bọc" lại trong một lớp vỏ bảo vệ mới.
  - **Quy tắc**: Attribute macro **có toàn quyền viết lại hoặc thay thế hoàn toàn đối tượng gốc** mà nó gắn lên!

### 3. Máy đổi ngoại tệ tự động (Function-like Macro)
- Bạn bước đến một cây ATM đổi tiền tự động tại sân bay:
  - Bạn đút vào một tờ tiền giấy ngoại tệ xa lạ (`sql!("SELECT * FROM users")`).
  - Cây ATM kiểm tra hoa văn, mệnh giá và nhả ra số tiền nội tệ tương đương để bạn tiêu dùng.
  - **Quy tắc**: Function-like macro cho phép bạn tạo ra một ngôn ngữ riêng, nhận bất kỳ chuỗi cú pháp nào và chuyển đổi nó thành mã máy Rust hợp lệ!

---

## Khái niệm & Cơ chế kỹ thuật chuyên sâu (In-Depth Technical Mechanics)

### 1. Bảng so sánh Ba loại Macro thủ tục

| Đặc điểm | Custom Derive | Attribute-like | Function-like |
|---|---|---|---|
| **Cú pháp sử dụng** | `#[derive(MyTrait)]` | `#[my_attribute]` | `my_macro!(...)` |
| **Khai báo trong `lib.rs`** | `#[proc_macro_derive(Name)]` | `#[proc_macro_attribute]` | `#[proc_macro]` |
| **Tham số hàm** | `(item: TokenStream)` | `(attr: TokenStream, item: TokenStream)` | `(input: TokenStream)` |
| **Khả năng sửa mã gốc** | ❌ Không (Chỉ sinh mã thêm) | ✅ Có (Có thể thay đổi/bọc mã gốc) | ✅ Có (Sinh mã hoàn toàn mới) |
| **Vị trí áp dụng** | Chỉ gắn trên `struct`, `enum`, `union` | Gắn trên hàm, struct, enum, mô-đun... | Bất kỳ vị trí nào cho phép biểu thức/câu lệnh |

### 2. Thuộc tính Bổ trợ (Helper Attributes) trong Custom Derive

Khi viết một Derive macro, bạn thường muốn cho phép người dùng tùy biến hành vi của từng trường dữ liệu:
```rust
#[derive(XuatDuLieu)]
struct NguoiDung {
    pub ho_ten: String,
    #[bo_qua] // Thuộc tính bổ trợ: không in trường mật khẩu này!
    pub mat_khau: String,
}
```
Để trình biên dịch không báo lỗi *"unknown attribute `bo_qua`"*, bạn phải đăng ký tên thuộc tính này trong khai báo macro bằng tham số `attributes(...)`:

```rust
#[proc_macro_derive(XuatDuLieu, attributes(bo_qua))]
pub fn xuat_du_lieu_derive(input: TokenStream) -> TokenStream {
    // rustc sẽ cho phép #[bo_qua] xuất hiện bên trong struct
}
```

### 3. Giải phẫu Attribute-like Macro: Cơ chế "Đóng gói bọc ngoài" (Decorator)

Attribute macro nhận vào hai dòng thẻ bài:
1. `attr`: Phần tham số nằm bên trong ngoặc vuông của thuộc tính (ví dụ: `#[kiem_tra(quyen = "admin")]` thì `attr` chứa `quyen = "admin"`).
2. `item`: Toàn bộ đoạn mã của đối tượng bên dưới thuộc tính (ví dụ toàn bộ phần thân của hàm `fn xu_ly() { ... }`).

```rust
#[proc_macro_attribute]
pub fn ghi_nhat_ky(attr: TokenStream, item: TokenStream) -> TokenStream {
    let ham_goc = parse_macro_input!(item as ItemFn);
    let ten_ham = &ham_goc.sig.ident;
    let than_ham = &ham_goc.block;
    let chu_ky = &ham_goc.sig;

    let ma_moi = quote! {
        #chu_ky {
            println!(">>> [NHẬT KÝ] Bắt đầu gọi hàm: {}", stringify!(#ten_ham));
            let ket_qua = (|| #than_ham )();
            println!(">>> [NHẬT KÝ] Kết thúc gọi hàm: {}", stringify!(#ten_ham));
            ket_qua
        }
    };

    TokenStream::from(ma_moi)
}
```

### 4. Kỹ thuật gỡ lỗi Macro thủ tục với `eprintln!`

Vì Macro thủ tục chạy trực tiếp trong lúc bạn gõ lệnh `cargo build`, bạn có thể dùng lệnh `eprintln!` ngay trong thân hàm proc-macro để in thông tin phân tích AST ra màn hình Terminal:
```rust
#[proc_macro_derive(KiemTra)]
pub fn kiem_tra_derive(input: TokenStream) -> TokenStream {
    let ast = parse_macro_input!(input as DeriveInput);
    eprintln!("DEBUG AST: {:#?}", ast); // In cây cú pháp ra terminal lúc build!
    TokenStream::new()
}
```

---

## Mã nguồn minh họa thực chiến (Idiomatic Runnable Rust Blueprint)

Dưới đây là một chương trình hoàn chỉnh, minh họa mô hình hoạt động và kết quả của cả ba loại Macro thủ tục trong một ứng dụng thực tế: **Hệ thống Quản lý Tài khoản & Kiểm toán Bảo mật**:
1. **Mô hình Custom Derive**: Tự động triển khai Trait `KiemToanBaoMat` có kiểm tra thuộc tính bổ trợ ẩn danh.
2. **Mô hình Attribute Macro**: Bọc hàm thực thi để tự động đo đạc hiệu năng và kiểm soát quyền truy cập.
3. **Mô hình Function-like Macro**: Phân tích cú pháp chuỗi cấu hình DSL dạng thẻ bài.

```rust
// Tệp: src/main.rs
// Chương trình thực chiến làm chủ Custom Derive, Attribute và Function-like Macros trong Rust

use std::collections::HashMap;

// ============================================================================
// 1. GIAO ƯỚC VÀ CÁC THỰC THỂ ĐƯỢC TỰ ĐỘNG SINH MÃ BỞI DERIVE MACRO
// ============================================================================

/// Trait mà Derive Macro #[derive(KiemToanBaoMat)] sẽ tự động sinh mã
pub trait KiemToanBaoMat {
    fn xuat_thong_tin_an_toan(&self) -> Vec<(&'static str, String)>;
    fn ma_phan_loai() -> &'static str;
}

pub struct TaiKhoanNganHang {
    pub so_tai_khoan: String,
    pub chu_tai_khoan: String,
    pub ma_pin_bi_mat: String, // Trường nhạy cảm: không được xuất ra nhật ký!
}

// Đoạn mã mà Custom Derive Macro tự động sinh ra cho TaiKhoanNganHang:
impl KiemToanBaoMat for TaiKhoanNganHang {
    fn xuat_thong_tin_an_toan(&self) -> Vec<(&'static str, String)> {
        // Macro thông minh tự động lọc bỏ trường nhạy cảm có gắn nhãn helper attribute
        vec![
            ("so_tai_khoan", self.so_tai_khoan.clone()),
            ("chu_tai_khoan", self.chu_tai_khoan.clone()),
            ("ma_pin_bi_mat", String::from("***ĐÃ_ẨN_BẢO_MẬT***")),
        ]
    }

    fn ma_phan_loai() -> &'static str {
        "TAI_KHOAN_NGAN_HANG_V1"
    }
}

// ============================================================================
// 2. MÔ HÌNH HÓA KẾT QUẢ CỦA ATTRIBUTE MACRO: #[kiem_soat_truy_cap]
// ============================================================================

/// Hàm mô phỏng mã sau khi được Attribute Macro bọc lớp vỏ bảo vệ
pub fn chuyen_khoan_an_toan(
    nguoi_gui: &str,
    nguoi_nhan: &str,
    so_tien: f64,
    vai_tro_nguoi_thuc_hien: &str,
) -> Result<String, &'static str> {
    // [MÃ DO ATTRIBUTE MACRO TỰ ĐỘNG CHÈN VÀO ĐẦU HÀM]:
    println!("[BẢO VỆ ATTRIBUTE] Đang xác thực quyền hạn của vai trò: '{}'", vai_tro_nguoi_thuc_hien);
    if vai_tro_nguoi_thuc_hien != "QuanTriVien" && vai_tro_nguoi_thuc_hien != "ChuTaiKhoan" {
        return Err("Từ chối truy cập: Bạn không có quyền thực hiện giao dịch này!");
    }

    // [THÂN HÀM NGUYÊN BẢN CỦA LẬP TRÌNH VIÊN]:
    println!("  -> Đang thực hiện chuyển {:.2} đồng từ {} sang {}", so_tien, nguoi_gui, nguoi_nhan);
    let ma_giao_dich = "GD-99882233";

    // [MÃ DO ATTRIBUTE MACRO TỰ ĐỘNG CHÈN VÀO CUỐI HÀM]:
    println!("[BẢO VỆ ATTRIBUTE] Giao dịch hoàn tất thành công. Mã định danh: {}", ma_giao_dich);
    Ok(format!("Chuyển tiền thành công! Mã giao dịch: {}", ma_giao_dich))
}

// ============================================================================
// 3. MÔ HÌNH HÓA FUNCTION-LIKE MACRO PHÂN TÍCH DSL CẤU HÌNH
// ============================================================================

/// Macro dạng hàm phân tích chuỗi cấu hình dạng "KEY=VALUE;KEY=VALUE" lúc biên dịch
macro_rules! phan_tich_cau_hinh {
    ( $( $khoa:ident = $gia_tri:expr );* $(;)? ) => {
        {
            let mut ban_do = HashMap::new();
            $(
                ban_do.insert(stringify!($khoa), $gia_tri);
            )*
            ban_do
        }
    };
}

// ============================================================================
// CHƯƠNG TRÌNH THỰC THI CHÍNH
// ============================================================================

fn main() {
    println!("============================================================");
    println!("     CHẾ TẠO VÀ ỨNG DỤNG BỘ BA PROCEDURAL MACROS TRONG RUST ");
    println!("============================================================");

    // ------------------------------------------------------------------------
    // 1. Kiểm chứng Custom Derive Macro với Helper Attribute
    // ------------------------------------------------------------------------
    println!("\n1. Ứng dụng Custom Derive Macro [KiemToanBaoMat]:");
    let tai_khoan = TaiKhoanNganHang {
        so_tai_khoan: String::from("1900-8888-9999"),
        chu_tai_khoan: String::from("Nguyễn Văn An"),
        ma_pin_bi_mat: String::from("SecretPin1234"),
    };

    println!("Mã phân loại thực thể: {}", TaiKhoanNganHang::ma_phan_loai());
    println!("Danh sách trường được xuất ra an toàn:");
    for (ten_truong, gia_tri) in tai_khoan.xuat_thong_tin_an_toan() {
        println!("  - {}: {}", ten_truong, gia_tri);
    }

    // ------------------------------------------------------------------------
    // 2. Kiểm chứng Attribute-like Macro bọc lớp bảo vệ
    // ------------------------------------------------------------------------
    println!("\n2. Ứng dụng Attribute Macro kiểm soát quyền truy cập:");
    
    // Thử nghiệm gọi với quyền hợp lệ
    let ket_qua_hop_le = chuyen_khoan_an_toan(
        "NguyenVanA", 
        "TranThiB", 
        5000.0, 
        "ChuTaiKhoan"
    );
    match ket_qua_hop_le {
        Ok(msg) => println!("  [OK] {}", msg),
        Err(e) => println!("  [LỖI] {}", e),
    }

    // Thử nghiệm gọi với quyền trái phép (Bị chặn ngay ở cổng)
    let ket_qua_vi_pham = chuyen_khoan_an_toan(
        "NguyenVanA", 
        "KeXau", 
        999999.0, 
        "KhachLa"
    );
    match ket_qua_vi_pham {
        Ok(msg) => println!("  [NGUY HIỂM] Lọt qua kiểm duyệt: {}", msg),
        Err(ly_do) => println!("  [CHẶN THÀNH CÔNG] {}", ly_do),
    }

    // ------------------------------------------------------------------------
    // 3. Ứng dụng Function-like Macro xử lý DSL tùy biến
    // ------------------------------------------------------------------------
    println!("\n3. Ứng dụng Function-like Macro khởi tạo cấu hình bảo mật:");
    let cau_hinh = phan_tich_cau_hinh! {
        TIMEOUT = 30;
        MAX_RETRY = 3;
        PORT = 8443;
    };

    for (k, v) in &cau_hinh {
        println!("  Tham số hệ thống `{}` được nạp với giá trị: {}", k, v);
    }

    println!("\n============================================================");
    println!("     HOÀN TẤT CHƯƠNG TRÌNH LÀM CHỦ BỘ BA PROCEDURAL MACROS  ");
    println!("============================================================");
}
```

---

## Bảng tra cứu lỗi biên dịch & Cách khắc phục (Compiler Error Guide)

Dưới đây là các lỗi thường gặp nhất khi triển khai và sử dụng Custom Derive và Attribute Macros:

| Mã lỗi | Thông báo mẫu từ trình biên dịch | Nguyên nhân cốt lõi | Cách khắc phục nhanh |
|---|---|---|---|
| **E0412** | `cannot find type '...' in this scope` | Mã do macro sinh ra tham chiếu đến một kiểu dữ liệu hoặc Trait nhưng người dùng chưa `use` nó vào phạm vi gọi. | Sử dụng đường dẫn tuyệt đối đầy đủ trong thân `quote!` (ví dụ: `::core::fmt::Display` hoặc `$crate::MyTrait`). |
| **Lỗi thuộc tính** | `cannot find attribute '...' in this scope` | Bạn dùng thuộc tính phụ `#[bo_qua]` trên trường struct nhưng chưa đăng ký nó trong khai báo `#[proc_macro_derive(TenTrait, attributes(bo_qua))]`. | Bổ sung tên thuộc tính phụ vào danh sách `attributes(...)` của proc macro derive. |
| **Lỗi chữ ký hàm** | `proc-macro attribute functions must have signature fn(TokenStream, TokenStream) -> TokenStream` | Bạn khai báo một Attribute Macro nhưng chỉ truyền vào một tham số `TokenStream` thay vì hai tham số (`attr` và `item`). | Sửa chữ ký hàm proc macro attribute thành `fn ten_macro(attr: TokenStream, item: TokenStream) -> TokenStream`. |
| **Lỗi phân tích cú pháp** | `proc macro panicked: unexpected token` | Dữ liệu đầu vào người dùng truyền vào macro không khớp với cấu trúc mà `syn` mong đợi (ví dụ truyền chuỗi sai quy cách). | Thay vì gọi `.unwrap()`, sử dụng `syn::parse::Parse` kết hợp với `syn::Error` để trả về thông báo lỗi êm đẹp cho người dùng. |

### Phân tích lỗi thực tế: Quên khai báo Helper Attribute

```rust
// Đoạn mã lỗi minh họa (trong Crate thư viện proc-macro):
// Khai báo thiếu attributes(bo_qua):
// #[proc_macro_derive(InThongTin)] 
// pub fn in_thong_tin(input: TokenStream) -> TokenStream { ... }

// Khi người dùng áp dụng:
// #[derive(InThongTin)]
// struct NguoiDung {
//     #[bo_qua] // LỖI: cannot find attribute `bo_qua` in this scope!
//     mat_khau: String,
// }

// Cách khắc phục chuẩn xác:
// #[proc_macro_derive(InThongTin, attributes(bo_qua))]
// pub fn in_thong_tin(input: TokenStream) -> TokenStream { ... }
```

---

## Tóm tắt chương & Bài tập rèn luyện (Summary & Exercises)

### 4 Điểm cốt lõi cần ghi nhớ:
1. **Bộ ba Macro thủ tục**:
   - *Custom Derive*: Tự động triển khai Trait, không sửa mã gốc.
   - *Attribute-like*: Bọc và biến đổi mã gốc tùy ý (Decorator Pattern).
   - *Function-like*: Xây dựng cú pháp ngôn ngữ riêng (DSL).
2. **Helper Attributes**: Bổ sung các nhãn chỉ dẫn tùy biến trên từng trường của struct để điều khiển hành vi sinh mã.
3. **An toàn kiểu với Đường dẫn Tuyệt đối**: Trong khối mã `quote!`, luôn dùng đường dẫn tuyệt đối (như `::std::string::String`) để tránh phụ thuộc vào các câu lệnh `use` của người dùng.
4. **Công cụ Soi mã `cargo expand`**: Luôn sử dụng `cargo expand` để kiểm tra mã nguồn thực tế sinh ra trước khi phát hành thư viện ra cộng đồng.

### Bài tập rèn luyện tự giải:
1. **Bài tập 1 (Phân loại Macro phù hợp)**:  
   Trong các bài toán sau, hãy chọn loại macro phù hợp nhất (Macro khai báo `macro_rules!`, Custom Derive, Attribute-like, hay Function-like):
   - Tự động sinh phương thức `fn khoi_tao_mac_dinh() -> Self` cho 20 struct khác nhau trong dự án.
   - Viết một bộ định tuyến cho Web Server kiểm tra quyền hạn `#[kiem_tra_admin]` trước khi thực thi hàm xử lý.
   - Viết một tiện ích `tao_danh_sach!(1, 2, 3)` nhận số lượng tham số tùy ý.

2. **Bài tập 2 (Thiết kế Helper Attribute)**:  
   Nếu bạn viết một Derive Macro mang tên `#[derive(XacThuc)]`, bạn sẽ thiết kế những thuộc tính phụ (Helper Attributes) nào trên các trường dữ liệu (ví dụ: kiểm tra độ dài chuỗi, kiểm tra số dương...)? Hãy mô tả cú pháp bạn mong muốn người dùng sử dụng.

3. **Bài tập 3 (Tổng kết Tư duy Siêu lập trình)**:  
   Siêu lập trình là một công cụ cực kỳ mạnh mẽ, nhưng tại sao các chuyên gia Rust luôn khuyên: *"Nếu bài toán có thể giải quyết được bằng Hàm (fn) hoặc Kiểu tổng quát (Generics), đừng bao giờ vội vàng viết Macro"*? Hãy nêu 3 nhược điểm lớn của việc lạm dụng macro trong dự án.
