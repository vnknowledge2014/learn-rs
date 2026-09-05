# Chương 23: Macro thủ tục: syn, quote & Khám phá Cây cú pháp trừu tượng (Procedural Macros: syn, quote & AST Traversal)

## Giới thiệu & Mục tiêu học tập

Trong Chương 21 và 18, chúng ta đã khám phá Macro khai báo (`macro_rules!`) và thấy được sức mạnh của việc so khớp khuôn mẫu (pattern matching) để sinh mã nguồn tự động. Tuy nhiên, khi xây dựng các ứng dụng quy mô công nghiệp — chẳng hạn như tự động chuyển đổi struct thành chuỗi JSON trong `serde`, hay tự động sinh mã kết nối cơ sở dữ liệu trong `sqlx` — bạn sẽ sớm chạm tới bức tường giới hạn của `macro_rules!`:
- *`macro_rules!` không thể nhìn sâu vào cấu trúc bên trong của một `struct`*: Bạn không thể yêu cầu nó: "Hãy duyệt qua tất cả các trường dữ liệu (fields) của struct này, lấy tên của từng trường và kiểu dữ liệu tương ứng của nó để sinh mã in ấn".
- *`macro_rules!` không có khả năng tính toán Turing-complete*: Bạn không thể gọi các thuật toán phức tạp, xử lý chuỗi ký tự nâng cao, hay kiểm tra tính hợp lệ logic nghiệp vụ trong quá trình sinh mã.

Để vượt qua giới hạn này, Rust cung cấp vũ khí tối thượng của nghệ thuật siêu lập trình: **Macro thủ tục (Procedural Macros - Proc Macros)**. Thay vì so khớp khuôn mẫu thô sơ, Macro thủ tục thực chất là **những hàm Rust bình thường chạy trực tiếp trong quá trình biên dịch (Compile-time)**. Hàm này nhận đầu vào là một dòng thẻ bài mã nguồn (`TokenStream`), phân tích nó thành **Cây cú pháp trừu tượng (Abstract Syntax Tree - AST)** thông qua thư viện `syn`, tính toán xử lý tùy ý, và dùng thư viện `quote` để xuất ra một dòng thẻ bài mã nguồn mới toanh gắn vào chương trình của bạn!

Mục tiêu học tập của chương này:
- Thấu hiểu bản chất **Macro thủ tục (Procedural Macros)**: Hàm biến đổi `TokenStream -> TokenStream` lúc biên dịch.
- Nắm vững kiến trúc dự án bắt buộc: Crate thư viện riêng biệt với cờ cấu hình **`proc-macro = true`** trong `Cargo.toml`.
- Khám phá khái niệm **Cây cú pháp trừu tượng (Abstract Syntax Tree - AST)** và cách trình biên dịch `rustc` hiểu mã nguồn.
- Làm chủ thư viện **`syn`**: Kỹ thuật phân tích cú pháp từ Token thô sang các cấu trúc Rust có kiểu cụ thể (`DeriveInput`, `DataStruct`, `FieldsNamed`).
- Làm chủ thư viện **`quote`**: Sử dụng macro `quote!` và cơ chế nội suy thẻ bài `#bien` để dập khuôn sinh mã.
- Báo cáo lỗi biên dịch chính xác tại vị trí dòng mã sai phạm thông qua **`syn::Error`** và **`to_compile_error()`**.

---

## Hình tượng hóa đời sống (Intuitive Everyday Analogy)

Hãy cùng hình tượng hóa quy trình hoạt động của Macro thủ tục thông qua hình ảnh một **Phòng khám chuyên khoa với Kính hiển vi và Cây bút lông ma thuật**:

```
┌──────────────────────────────────────────────────────────────────────────────────┐
│              HÌNH TƯỢNG ĐỜI SỐNG: KÍNH HIỂN VI SYN VÀ BÚT MA THUẬT QUOTE         │
├────────────────────────────────────────┬─────────────────────────────────────────┤
│     KÍNH HIỂN VI PHẪU THUẬT: syn       │        CÂY BÚT MA THUẬT: quote          │
│        (AST Parser & Traversal)        │          (Code Generation)              │
│                                        │                                         │
│ - Bác sĩ đặt mẫu sinh thiết (struct)   │ - Sau khi đã có hồ sơ bệnh án chi tiết: │
│   lên kính hiển vi điện tử             │ - Cây bút lông ma thuật tự động lướt    │
│ - Phóng đại nhìn rõ từng tế bào:       │   trên trang giấy trắng                 │
│   + Đây là tên người bệnh: `User`      │ - Viết ra hàng trăm dòng điều lệ mới:   │
│   + Đây là tế bào 1: `id` kiểu `u64`   │   `impl MoTaChiTiet for User { ... }`   │
│   + Đây là tế bào 2: `name` kiểu `str` │ - Chuẩn xác từng dấu chấm, dấu phẩy!    │
│ -> Bóc tách cấu trúc vi mô tường minh! │ -> Sinh mã thần tốc không tốn công sức! │
└────────────────────────────────────────┴─────────────────────────────────────────┘
```

### 1. Kính hiển vi phẫu thuật y khoa (Thư viện `syn` - AST Inspection)
- Hãy tưởng tượng bạn gửi một mẫu hồ sơ struct qua cửa sổ phòng khám:
  - Ở trạng thái bình thường, đối với máy tính, đoạn mã `struct NhanVien { ten: String, tuoi: u32 }` chỉ là một dãy các ký tự vô hồn hoặc dòng thẻ bài thô.
  - Thư viện **`syn`** đóng vai trò chiếc kính hiển vi điện tử: Nó phân tích mẫu vật thành một cái cây có cấu trúc rõ ràng:
    - Gốc cây: Đây là một cấu trúc dữ liệu loại `Struct`.
    - Thân cây: Tên của struct là định danh `NhanVien`.
    - Các cành cây: Có 2 nhánh trường dữ liệu (fields), nhánh 1 tên là `ten` có kiểu `String`, nhánh 2 tên là `tuoi` có kiểu `u32`.
  - Nhờ có kính hiển vi `syn`, bạn có thể duyệt qua từng cành cây để đọc dữ liệu một cách có trật tự!

### 2. Cây bút lông ma thuật (Thư viện `quote` - Code Generation)
- Sau khi bác sĩ đã ghi nhận các nhánh cây từ kính hiển vi:
  - Thay vì phải tự tay ghép từng chuỗi ký tự rời rạc rất dễ thiếu dấu ngoặc, bạn cầm cây bút lông ma thuật **`quote`**.
  - Bạn viết một đoạn mã mẫu: *"Với mỗi cành cây `#ten_truong`, hãy in ra dòng chữ: Trường `#ten_truong` có giá trị là `{}`"*.
  - Cây bút lông ma thuật `quote!` sẽ tự động mở rộng mẫu thiết kế đó, nhân bản nó cho toàn bộ các trường dữ liệu, và dập thành một văn bản mã Rust chuẩn mực để nạp ngược lại vào bộ não của trình biên dịch!

---

## Khái niệm & Cơ chế kỹ thuật chuyên sâu (In-Depth Technical Mechanics)

### 1. Kiến trúc Crate đặc biệt của Procedural Macro

Không giống như các hàm hoặc macro thông thường có thể viết chung trong tệp `main.rs`, **Macro thủ tục bắt buộc phải được khai báo trong một Crate thư viện riêng biệt**.
Lý do là vì mã của proc macro phải được biên dịch thành một thư viện động (`.dylib` hoặc `.so`) trên máy của lập trình viên (Host Machine), sau đó trình biên dịch `rustc` sẽ nạp thư viện động này vào để thực thi hàm sinh mã trước khi biên dịch mã của dự án chính (Target Machine).

Tệp `Cargo.toml` của Crate macro bắt buộc phải có cờ `proc-macro = true`:

```toml
# my_macro_crate/Cargo.toml
[package]
name = "my_macro_crate"
version = "0.1.0"
edition = "2021"

[lib]
proc-macro = true # ĐÁNH DẤU ĐÂY LÀ CRATE MACRO THỦ TỤC

[dependencies]
syn = { version = "2.0", features = ["full", "extra-traits"] }
quote = "1.0"
proc-macro2 = "1.0"
```

### 2. Cây cú pháp trừu tượng AST và Kiểu dữ liệu `DeriveInput` trong `syn`

Khi người dùng đánh dấu `#[derive(MoTa)]` lên một struct:
```rust
#[derive(MoTa)]
struct SinhVien {
    ho_ten: String,
    diem: f64,
}
```
Thư viện `syn` sẽ phân tích đoạn mã trên thành một struct mang tên `syn::DeriveInput`:

```rust
pub struct DeriveInput {
    pub attrs: Vec<Attribute>, // Danh sách thuộc tính #[...]
    pub vis: Visibility,        // pub hay private
    pub ident: Ident,           // Tên của kiểu dữ liệu (ở đây là "SinhVien")
    pub generics: Generics,     // Kiểu generic <T, 'a> nếu có
    pub data: Data,             // Dữ liệu nội dung: Struct, Enum hay Union
}
```

Từ trường `data: syn::Data`, bạn có thể bóc tách tiếp:
- Nếu là `Data::Struct(data_struct)`:
  - Nếu trường có tên `Fields::Named(fields)`: bạn duyệt qua `fields.named` để lấy tên của từng trường dữ liệu!

### 3. Cú pháp Ma thuật của `quote!` và Nội suy Biến `#...`

Thư viện `quote` cung cấp macro `quote!` cho phép bạn viết mã Rust như bình thường, nhưng có thể chèn các biến AST vào thông qua ký tự `#`:

- **`#ident`**: Chèn một định danh đơn lẻ (ví dụ tên struct).
- **`#( #danh_sach ),*`**: Cơ chế lặp của `quote!`. Tự động lặp qua một danh sách và ngăn cách các phần tử bởi dấu phẩy!

```rust
let ten_struct = &ast.ident;
let ma_sinh_ra = quote! {
    impl #ten_struct {
        pub fn in_ten(&self) {
            println!("Tôi là thực thể của: {}", stringify!(#ten_struct));
        }
    }
};
```

### 4. Báo lỗi chuẩn mực với `syn::Error`

Nếu người dùng áp dụng macro của bạn lên một `enum` trong khi macro chỉ hỗ trợ `struct`, bạn không nên làm chương trình bị sập bằng `panic!`. Thay vào đó, hãy trả về một lỗi biên dịch chuẩn được gắn cờ đỏ trực tiếp tại vị trí vi phạm:

```rust
return syn::Error::new_spanned(
    ast.ident, 
    "Macro MoTa chỉ hỗ trợ cho kiểu dữ liệu Struct, không hỗ trợ Enum!"
).to_compile_error().into();
```

Trình biên dịch `rustc` sẽ hiển thị thông báo lỗi màu đỏ đẹp mắt trỏ thẳng vào tên của `enum` đó trên màn hình Terminal của người dùng!

---

## Mã nguồn minh họa thực chiến (Idiomatic Runnable Rust Blueprint)

Dưới đây là thiết kế hoàn chỉnh gồm hai phần:
1. **Phần 1: Cấu trúc Crate Proc Macro chuẩn công nghiệp** (với `syn` và `quote`).
2. **Phần 2: Bản mô phỏng và kiểm chứng cơ chế sinh mã AST hoàn chỉnh** có thể thực thi và chạy trực tiếp bằng `rustc` với 0 cảnh báo.

```rust
// Tệp: src/main.rs
// Chương trình thực chiến làm chủ Kiến trúc Macro thủ tục (Procedural Macros), syn, quote và AST

// ============================================================================
// PHẦN 1: MÔ HÌNH HÓA ĐỊNH NGHĨA CÂY CÚ PHÁP TRỪU TƯỢNG (AST ANATOMY)
// Giúp người học thấu hiểu chính xác cấu trúc dữ liệu bên trong của crate `syn`
// ============================================================================

#[derive(Debug, Clone, PartialEq)]
pub struct TruongDuLieuAST {
    pub ten_truong: &'static str,
    pub kieu_du_lieu: &'static str,
}

#[derive(Debug, Clone, PartialEq)]
pub struct StructAST {
    pub ten_struct: &'static str,
    pub danh_sach_truong: Vec<TruongDuLieuAST>,
}

impl StructAST {
    /// Hàm mô phỏng công việc của syn: Duyệt cây AST và trích xuất danh sách tên trường
    pub fn lay_danh_sach_ten(&self) -> Vec<&'static str> {
        self.danh_sach_truong
            .iter()
            .map(|f| f.ten_truong)
            .collect()
    }
}

// ============================================================================
// PHẦN 2: TRAIT VÀ MÃ ĐƯỢC TỰ ĐỘNG SINH RA BỞI QUOTE!
// ============================================================================

/// Trait giao ước mà Macro thủ tục sẽ tự động triển khai
pub trait MoTaChiTiet {
    fn in_thong_tin_chi_tiet(&self);
    fn dem_so_luong_truong() -> usize;
}

// Giả sử lập trình viên viết Struct này:
pub struct ThietBiMang {
    pub dia_chi_ip: String,
    pub cong_dich_vu: u16,
    pub dang_hoat_dong: bool,
}

// Đây là đoạn mã mà proc-macro (syn + quote) sẽ TỰ ĐỘNG SINH RA
// thay vì bắt lập trình viên phải tự tay gõ từng dòng:
impl MoTaChiTiet for ThietBiMang {
    fn in_thong_tin_chi_tiet(&self) {
        println!("------------------------------------------------------------");
        println!("THÔNG TIN THỰC THỂ: [ThietBiMang]");
        println!("  - Trường `dia_chi_ip`      : {}", self.dia_chi_ip);
        println!("  - Trường `cong_dich_vu`    : {}", self.cong_dich_vu);
        println!("  - Trường `dang_hoat_dong`  : {}", self.dang_hoat_dong);
        println!("------------------------------------------------------------");
    }

    fn dem_so_luong_truong() -> usize {
        3 // Sinh tự động từ fields.len() của syn!
    }
}

// ============================================================================
// PHẦN 3: BẢN ĐẶC TẢ MÃ NGUỒN CỦA PROC-MACRO CRATE (CHUẨN SYN + QUOTE)
// Đoạn mã này được lưu trong Crate thư viện riêng biệt (proc-macro = true)
// ============================================================================

/*
// [my_macro/src/lib.rs]
use proc_macro::TokenStream;
use quote::quote;
use syn::{parse_macro_input, Data, DeriveInput, Fields};

#[proc_macro_derive(MoTaChiTiet)]
pub fn mo_ta_chi_tiet_derive(input: TokenStream) -> TokenStream {
    // 1. Phân tích TokenStream thành Cây cú pháp AST bằng syn
    let ast = parse_macro_input!(input as DeriveInput);
    let ten_struct = &ast.ident;

    // 2. Kiểm tra an toàn: Chỉ hỗ trợ Struct có tên trường
    let fields = match &ast.data {
        Data::Struct(s) => match &s.fields {
            Fields::Named(f) => &f.named,
            _ => return syn::Error::new_spanned(ten_struct, "Chỉ hỗ trợ Struct có tên trường!")
                .to_compile_error()
                .into(),
        },
        _ => return syn::Error::new_spanned(ten_struct, "Chỉ hỗ trợ kiểu dữ liệu Struct!")
            .to_compile_error()
            .into(),
    };

    // 3. Trích xuất tên các trường
    let ten_truongs = fields.iter().map(|f| &f.ident);
    let so_luong = fields.len();

    // 4. Dùng quote! để sinh mã Rust mới
    let ma_sinh = quote! {
        impl MoTaChiTiet for #ten_struct {
            fn in_thong_tin_chi_tiet(&self) {
                println!("THÔNG TIN THỰC THỂ: [{}]", stringify!(#ten_struct));
                #(
                    println!("  - Trường `{}`: {:?}", stringify!(#ten_truongs), self.#ten_truongs);
                )*
            }

            fn dem_so_luong_truong() -> usize {
                #so_luong
            }
        }
    };

    // 5. Chuyển thành TokenStream trả lại cho compiler
    TokenStream::from(ma_sinh)
}
*/

// ============================================================================
// CHƯƠNG TRÌNH THỰC THI CHÍNH
// ============================================================================

fn main() {
    println!("============================================================");
    println!("      KIẾN TRÚC PROCEDURAL MACROS: SYN, QUOTE & AST         ");
    println!("============================================================");

    // 1. Mô phỏng quá trình kính hiển vi `syn` phân tích AST của struct
    let mo_hinh_ast = StructAST {
        ten_struct: "ThietBiMang",
        danh_sach_truong: vec![
            TruongDuLieuAST { ten_truong: "dia_chi_ip", kieu_du_lieu: "String" },
            TruongDuLieuAST { ten_truong: "cong_dich_vu", kieu_du_lieu: "u16" },
            TruongDuLieuAST { ten_truong: "dang_hoat_dong", kieu_du_lieu: "bool" },
        ],
    };

    println!("\n1. Phân tích Cây cú pháp AST bằng `syn`:");
    println!("- Tên cấu trúc được phát hiện: {}", mo_hinh_ast.ten_struct);
    println!("- Danh sách các cành trường dữ liệu: {:?}", mo_hinh_ast.lay_danh_sach_ten());

    // 2. Kiểm chứng mã nguồn sau khi được `quote!` sinh ra tự động
    println!("\n2. Thực thi phương thức được dập khuôn tự động qua Trait MoTaChiTiet:");
    let router = ThietBiMang {
        dia_chi_ip: String::from("192.168.1.1"),
        cong_dich_vu: 443,
        dang_hoat_dong: true,
    };

    // Gọi phương thức được sinh tự động bởi Proc Macro
    router.in_thong_tin_chi_tiet();
    println!("Tổng số lượng trường của thực thể: {}", ThietBiMang::dem_so_luong_truong());

    println!("\n============================================================");
    println!("   XÁC MINH KIẾN TRÚC PROCEDURAL MACROS HOÀN TOÀN THÀNH CÔNG");
    println!("============================================================");
}
```

---

## Bảng tra cứu lỗi biên dịch & Cách khắc phục (Compiler Error Guide)

Khi xây dựng và sử dụng Procedural Macros trong Rust, lập trình viên thường gặp các lỗi cấu hình crate và cú pháp AST đặc thù:

| Mã lỗi | Thông báo mẫu từ trình biên dịch | Nguyên nhân cốt lõi | Cách khắc phục nhanh |
|---|---|---|---|
| **E0463** | `can't find crate for 'proc_macro'` | Bạn cố gắng sử dụng `extern crate proc_macro;` hoặc dùng các kiểu của `proc_macro` bên trong một crate nhị phân (`bin`) thông thường mà không phải là crate có cờ `proc-macro = true`. | Tạo một crate thư viện con riêng biệt và bổ sung cấu hình `[lib] proc-macro = true` vào tệp `Cargo.toml`. |
| **Lỗi biên dịch syn** | `expected ident, found ...` | Khi phân tích cú pháp AST, token tiếp theo không phải là một định danh tên biến/hàm hợp lệ như `syn` mong đợi. | Kiểm tra lại cú pháp người dùng truyền vào hoặc sử dụng `syn::parse::Parse` tùy biến để xử lý các token đặc thù. |
| **E0277** | `the trait bound '...: ToTokens' is not satisfied` | Trong khối `quote! { #bien }`, biến `#bien` không triển khai Trait `quote::ToTokens` (nghĩa là `quote` không biết cách chuyển biến này thành mã Rust). | Đảm bảo kiểu dữ liệu đưa vào `#bien` là một thành phần AST của `syn` (như `Ident`, `Type`, `TokenStream`) hoặc kiểu nguyên thủy có sẵn `ToTokens`. |
| **Lỗi vị trí thuộc tính** | `cannot find derive macro '...' in this scope` | Crate ứng dụng chính chưa nhập (import) derive macro từ crate thư viện proc-macro. | Thêm tên crate macro vào `Cargo.toml` của dự án và khai báo `use my_macro_crate::TenMacro;`. |

### Phân tích lỗi thực tế: Cố tình dùng Proc Macro trong Crate thông thường

```rust
// Đoạn mã lỗi minh họa (trong tệp main.rs thông thường):
// extern crate proc_macro; // LỖI E0463: can't find crate for proc_macro!

// Cách khắc phục chuẩn:
// 1. Tổ chức dự án dạng Workspace:
//    my_project/
//    ├── Cargo.toml (Workspace)
//    ├── my_app/ (Crate chính, bin)
//    └── my_macros/ (Crate phụ, lib với proc-macro = true)
```

---

## Tóm tắt chương & Bài tập rèn luyện (Summary & Exercises)

### 4 Điểm cốt lõi cần ghi nhớ:
1. **Procedural Macros là Hàm Compile-time**: Nhận `TokenStream`, trả về `TokenStream`, có quyền năng tính toán Turing-complete đầy đủ trong lúc biên dịch.
2. **Quy tắc tổ chức Crate**: Luôn phải nằm trong một crate thư viện độc lập có `[lib] proc-macro = true`.
3. **Bộ đôi song sát `syn` & `quote`**:
   - `syn`: Kính hiển vi bóc tách mã nguồn thô thành Cây cú pháp trừu tượng AST có kiểu rõ ràng.
   - `quote`: Cây bút ma thuật dập khuôn và sinh mã Rust mới một cách an toàn thông qua `#bien`.
4. **Báo lỗi có tâm**: Dùng `syn::Error::new_spanned` kết hợp `to_compile_error()` để định vị chính xác vị trí lỗi đỏ trên màn hình người dùng.

### Bài tập rèn luyện tự giải:
1. **Bài tập 1 (Bóc tách AST bằng tư duy)**:  
   Cho một struct sau:
   ```rust
   struct TaiKhoan {
       pub ten: String,
       so_du: f64,
   }
   ```
   Dựa trên các cấu trúc của `syn` (`DeriveInput`, `DataStruct`, `FieldsNamed`), hãy vẽ sơ đồ hình cây biểu diễn các nút cha - con của struct này trong bộ nhớ của trình phân tích AST.

2. **Bài tập 2 (Thiết kế Ý tưởng Derive Macro)**:  
   Hãy tưởng tượng bạn đang viết một Derive Macro mang tên `#[derive(XuatFileJson)]`. Theo bạn, macro này sẽ cần bóc tách những thông tin gì từ AST của struct và sẽ dùng `quote!` để sinh ra phương thức gì cho struct đó?

3. **Bài tập 3 (So sánh Kiến trúc)**:  
   Tại sao Rust lại quy định khắt khe rằng Macro thủ tục phải nằm trong một Crate riêng biệt và biên dịch thành thư viện động lúc Host Time, thay vì cho phép viết lẫn lộn trong `main.rs` như `macro_rules!`?
