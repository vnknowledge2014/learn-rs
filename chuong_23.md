# Chương 23: Macro thủ tục: syn, quote & Khám phá Cây cú pháp trừu tượng (Procedural Macros: syn, quote & AST Traversal)

## Giới thiệu & Mục tiêu học tập

Trong Chương 21 và 22, chúng ta đã khám phá Macro khai báo (`macro_rules!`) và thấy được sức mạnh của việc so khớp khuôn mẫu (pattern matching) để sinh mã nguồn tự động. Tuy nhiên, khi xây dựng các ứng dụng quy mô công nghiệp — chẳng hạn như tự động chuyển đổi struct thành chuỗi JSON trong `serde`, hay tự động sinh mã kết nối cơ sở dữ liệu trong `sqlx` — bạn sẽ sớm chạm tới bức tường giới hạn của `macro_rules!`:
- *`macro_rules!` không thể nhìn sâu vào cấu trúc bên trong của một `struct`*: Bạn không thể yêu cầu nó: "Hãy duyệt qua tất cả các trường dữ liệu (fields) của struct này, lấy tên của từng trường và kiểu dữ liệu tương ứng của nó để sinh mã in ấn".
- *`macro_rules!` không chạy được mã Rust tuỳ ý*: Về lý thuyết, các macro TT Muncher đệ quy có thể mô phỏng mọi phép tính, nhưng trong thực tế bạn không thể gọi hàm, dùng `String`, `HashMap`, hay kiểm tra logic nghiệp vụ phức tạp trong quá trình sinh mã.

Để vượt qua giới hạn này, Rust cung cấp vũ khí tối thượng của nghệ thuật siêu lập trình: **Macro thủ tục (Procedural Macros - Proc Macros)**. Thay vì so khớp khuôn mẫu thô sơ, Macro thủ tục thực chất là **những hàm Rust bình thường chạy trực tiếp trong quá trình biên dịch (Compile-time)**. Hàm này nhận đầu vào là một dòng thẻ bài mã nguồn (`TokenStream`), phân tích nó thành **Cây cú pháp trừu tượng (Abstract Syntax Tree - AST)** thông qua thư viện `syn`, tính toán xử lý tùy ý, và dùng thư viện `quote` để xuất ra một dòng thẻ bài mã nguồn mới toanh gắn vào chương trình của bạn!

Mục tiêu học tập của chương này:
- Thấu hiểu bản chất **Macro thủ tục (Procedural Macros)**: Hàm biến đổi `TokenStream -> TokenStream` lúc biên dịch.
- Nắm vững kiến trúc dự án bắt buộc: Crate thư viện riêng biệt với cờ cấu hình **`proc-macro = true`** trong `Cargo.toml`.
- Khám phá khái niệm **Cây cú pháp trừu tượng (Abstract Syntax Tree - AST)** và cách trình biên dịch `rustc` hiểu mã nguồn.
- Làm chủ thư viện **`syn`**: Kỹ thuật phân tích cú pháp từ Token thô sang các cấu trúc Rust có kiểu cụ thể (`DeriveInput`, `DataStruct`, `FieldsNamed`).
- Làm chủ thư viện **`quote`**: Sử dụng macro `quote!` và cơ chế nội suy thẻ bài `#var` để dập khuôn sinh mã.
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
│   + Đây là tế bào 1: `id` kiểu `u64`   │   `impl DetailedDescription for User`   │
│   + Đây là tế bào 2: `name` kiểu `str` │ - Chuẩn xác từng dấu chấm, dấu phẩy!    │
│ -> Bóc tách cấu trúc vi mô tường minh! │ -> Sinh mã thần tốc không tốn công sức! │
└────────────────────────────────────────┴─────────────────────────────────────────┘
```

### 1. Kính hiển vi phẫu thuật y khoa (Thư viện `syn` - AST Inspection)
- Hãy tưởng tượng bạn gửi một mẫu hồ sơ struct qua cửa sổ phòng khám:
  - Ở trạng thái bình thường, đối với máy tính, đoạn mã `struct Employee { name: String, age: u32 }` chỉ là một dãy các ký tự vô hồn hoặc dòng thẻ bài thô.
  - Thư viện **`syn`** đóng vai trò chiếc kính hiển vi điện tử: Nó phân tích mẫu vật thành một cái cây có cấu trúc rõ ràng:
    - Gốc cây: Đây là một cấu trúc dữ liệu loại `Struct`.
    - Thân cây: Tên của struct là định danh `Employee`.
    - Các cành cây: Có 2 nhánh trường dữ liệu (fields), nhánh 1 tên là `name` có kiểu `String`, nhánh 2 tên là `age` có kiểu `u32`.
  - Nhờ có kính hiển vi `syn`, bạn có thể duyệt qua từng cành cây để đọc dữ liệu một cách có trật tự!

### 2. Cây bút lông ma thuật (Thư viện `quote` - Code Generation)
- Sau khi bác sĩ đã ghi nhận các nhánh cây từ kính hiển vi:
  - Thay vì phải tự tay ghép từng chuỗi ký tự rời rạc rất dễ thiếu dấu ngoặc, bạn cầm cây bút lông ma thuật **`quote`**.
  - Bạn viết một đoạn mã mẫu: *"Với mỗi cành cây `#field_name`, hãy in ra dòng chữ: Trường `#field_name` có giá trị là `{}`"*.
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
edition = "2024"

[lib]
proc-macro = true # ĐÁNH DẤU ĐÂY LÀ CRATE MACRO THỦ TỤC

[dependencies]
syn = "3"          # thêm features = ["full"] nếu cần phân tích hàm, khối lệnh...
quote = "1"
proc-macro2 = "1"
```

### 2. Cây cú pháp trừu tượng AST và Kiểu dữ liệu `DeriveInput` trong `syn`

Khi người dùng đánh dấu `#[derive(DetailedDescription)]` lên một struct:
```rust
#[derive(DetailedDescription)]
struct Student {
    full_name: String,
    score: f64,
}
```
Thư viện `syn` sẽ phân tích đoạn mã trên thành một struct mang tên `syn::DeriveInput`:

```rust
pub struct DeriveInput {
    pub attrs: Vec<Attribute>, // Danh sách thuộc tính #[...]
    pub vis: Visibility,        // pub hay private
    pub ident: Ident,           // Tên của kiểu dữ liệu (ở đây là "Student")
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
- **`#( #list ),*`**: Cơ chế lặp của `quote!`. Tự động lặp qua một danh sách và ngăn cách các phần tử bởi dấu phẩy!

```rust
let name = &ast.ident;
let generated = quote! {
    impl #name {
        pub fn print_type_name(&self) {
            println!("Tôi là thực thể của: {}", stringify!(#name));
        }
    }
};
```

### 4. Báo lỗi chuẩn mực với `syn::Error`

Nếu người dùng áp dụng macro của bạn lên một `enum` trong khi macro chỉ hỗ trợ `struct`, bạn không nên làm chương trình bị sập bằng `panic!`. Thay vào đó, hãy trả về một lỗi biên dịch chuẩn được gắn cờ đỏ trực tiếp tại vị trí vi phạm:

```rust
return syn::Error::new_spanned(
    &ast.ident,
    "DetailedDescription chỉ hỗ trợ struct, không hỗ trợ enum/union",
)
.to_compile_error()
.into();
```

Trình biên dịch `rustc` sẽ hiển thị thông báo lỗi màu đỏ đẹp mắt trỏ thẳng vào tên của `enum` đó trên màn hình Terminal của người dùng!

---

## Mã nguồn minh họa thực chiến (Idiomatic Runnable Rust Blueprint)

Dưới đây là một workspace thật gồm **hai crate**, biên dịch và chạy được bằng `cargo run -p ch23` (bài kiểm thử: `cargo test -p ch23`):
1. **`ch23_macros`** — crate thư viện `proc-macro = true`, chứa derive macro `#[derive(DetailedDescription)]` viết bằng `syn` + `quote`.
2. **`ch23`** — crate nhị phân định nghĩa trait `DetailedDescription`, **dùng** derive macro ở trên, và dùng thêm `syn::parse_str` để "soi" tận mắt cây AST mà macro nhận được.

**Bước 1 — Crate macro.** Tệp `ch23_macros/Cargo.toml`:

```toml
[package]
name = "ch23_macros"
version = "0.1.0"
edition = "2024"

[lib]
proc-macro = true

[dependencies]
syn = "3"
quote = "1"
proc-macro2 = "1"
```

Tệp `ch23_macros/src/lib.rs`:

```rust
// Tệp: ch23_macros/src/lib.rs
//! Crate macro thủ tục của Chương 23.
//!
//! Một crate `proc-macro = true` CHỈ được xuất ra macro (không xuất được trait,
//! struct hay hàm thường). Vì vậy trait `DetailedDescription` nằm ở crate dùng
//! macro (`ch23`), còn crate này chỉ sinh ra khối `impl` cho trait đó.

use proc_macro::TokenStream;
use quote::quote;
use syn::{Data, DeriveInput, Fields, parse_macro_input};

/// `#[derive(DetailedDescription)]` — sinh `impl DetailedDescription for <Kiểu>`.
///
/// Mã sinh ra gọi tên trait KHÔNG kèm đường dẫn, nên ở chỗ dùng trait phải nằm
/// trong phạm vi (giống ví dụ `HelloMacro` trong The Rust Book).
#[proc_macro_derive(DetailedDescription)]
pub fn detailed_description_derive(input: TokenStream) -> TokenStream {
    // 1. Phân tích TokenStream thành cây cú pháp AST bằng syn
    let ast = parse_macro_input!(input as DeriveInput);
    let name = &ast.ident;

    // 2. Kiểm tra an toàn: chỉ hỗ trợ struct có trường đặt tên.
    //    Sai thì trả về LỖI BIÊN DỊCH trỏ đúng vào tên kiểu, KHÔNG panic.
    let fields = match &ast.data {
        Data::Struct(s) => match &s.fields {
            Fields::Named(named) => &named.named,
            _ => {
                return syn::Error::new_spanned(
                    name,
                    "DetailedDescription chỉ hỗ trợ struct có trường đặt tên",
                )
                .to_compile_error()
                .into();
            }
        },
        _ => {
            return syn::Error::new_spanned(
                name,
                "DetailedDescription chỉ hỗ trợ struct, không hỗ trợ enum/union",
            )
            .to_compile_error()
            .into();
        }
    };

    // 3. Trích xuất tên các trường (Fields::Named nên `ident` luôn là Some)
    let field_idents: Vec<_> = fields.iter().filter_map(|f| f.ident.as_ref()).collect();
    let field_names: Vec<String> = field_idents.iter().map(|i| i.to_string()).collect();
    let count = field_idents.len();

    // 4. Giữ nguyên generics của struct gốc để struct generic vẫn dùng được
    let (impl_generics, ty_generics, where_clause) = ast.generics.split_for_impl();

    // 5. Dùng quote! để sinh mã Rust mới; `#( ... )*` lặp qua từng trường
    let expanded = quote! {
        impl #impl_generics DetailedDescription for #name #ty_generics #where_clause {
            fn describe(&self) -> String {
                let mut out = format!("THÔNG TIN THỰC THỂ: [{}]", stringify!(#name));
                #(
                    out.push_str(&format!(
                        "\n  - Trường `{}`: {:?}",
                        #field_names,
                        self.#field_idents
                    ));
                )*
                out
            }

            fn field_names() -> &'static [&'static str] {
                &[ #( #field_names ),* ]
            }

            fn field_count() -> usize {
                #count
            }
        }
    };

    // 6. Chuyển proc_macro2::TokenStream thành proc_macro::TokenStream trả cho rustc
    TokenStream::from(expanded)
}
```

> **Về phiên bản `syn`:** chương này dùng `syn` 3.x. Với những gì một derive macro cơ bản cần (`DeriveInput`, `Data`, `Fields`, `syn::Error`, `split_for_impl`), API giống hệt `syn` 2.x; điểm khác dễ thấy nhất là `syn::Field` có thêm hai trường `modifiers` và `default`, nên đừng dựng `Field` bằng cú pháp struct literal trong mã của bạn.

**Bước 2 — Crate dùng macro.** Tệp `ch23/Cargo.toml`:

```toml
[package]
name = "ch23"
version = "0.1.0"
edition = "2024"

[dependencies]
# Crate macro thủ tục do chính chương này viết (proc-macro = true)
ch23_macros = { path = "../ch23_macros" }
# Dùng syn ở chế độ thường (không trong macro) để "soi" cây AST lúc chạy
syn = "3"
quote = "1"
```

Tệp `ch23/src/main.rs`:

```rust
// Tệp: src/main.rs
// Chương trình thực chiến làm chủ Kiến trúc Macro thủ tục (Procedural Macros), syn, quote và AST
//
// Cấu trúc workspace:
//   ch23_macros/  (lib, proc-macro = true) — chứa #[proc_macro_derive(DetailedDescription)]
//   ch23/         (bin)                    — định nghĩa trait và DÙNG derive macro

use std::fmt::Debug;

use ch23_macros::DetailedDescription;
use quote::ToTokens;
use syn::{Data, DeriveInput, Fields, Visibility};

// ============================================================================
// PHẦN 1: TRAIT MÀ DERIVE MACRO SẼ TỰ ĐỘNG CÀI ĐẶT
// ============================================================================
// Crate proc-macro không xuất được trait, nên trait nằm ở đây. (Trait và derive
// macro trùng tên `DetailedDescription` không xung đột: macro sống ở không gian
// tên macro, trait ở không gian tên kiểu — y hệt `Debug` của thư viện chuẩn.)

pub trait DetailedDescription {
    /// Mô tả từng trường kèm giá trị (sinh tự động bởi quote!)
    fn describe(&self) -> String;
    /// Danh sách tên trường (sinh tự động từ Fields::Named của syn)
    fn field_names() -> &'static [&'static str];
    /// Số trường (sinh tự động từ fields.len())
    fn field_count() -> usize;

    /// Phương thức mặc định — KHÔNG do macro sinh ra
    fn print_details(&self) {
        println!("------------------------------------------------------------");
        println!("{}", self.describe());
        println!("------------------------------------------------------------");
    }
}

// ============================================================================
// PHẦN 2: STRUCT DÙNG DERIVE MACRO THẬT
// ============================================================================

/// Lập trình viên chỉ viết đúng một dòng #[derive(...)].
/// Toàn bộ khối `impl DetailedDescription for NetworkDevice` do ch23_macros sinh ra.
#[derive(Debug, DetailedDescription)]
pub struct NetworkDevice {
    pub ip_address: String,
    pub service_port: u16,
    pub is_active: bool,
}

/// Struct generic: macro dùng `split_for_impl()` để giữ nguyên `<T>` và `where`.
#[derive(DetailedDescription)]
pub struct Pair<T: Debug> {
    pub left: T,
    pub right: T,
}

// ============================================================================
// PHẦN 3: "KÍNH HIỂN VI" SYN — PHÂN TÍCH AST NGAY LÚC CHẠY
// ============================================================================
// syn không chỉ chạy trong macro: `syn::parse_str` phân tích được mã nguồn
// dạng chuỗi ở chương trình thường. Ta dùng nó để NHÌN TẬN MẮT cây DeriveInput
// mà derive macro nhận được.

/// Thông tin của một trường, rút từ cây AST.
#[derive(Debug, Clone, PartialEq)]
pub struct FieldInfo {
    /// `None` với struct dạng tuple — các trường không có tên
    pub name: Option<String>,
    /// Kiểu ở dạng CÚ PHÁP (chưa được phân giải), in lại thành chuỗi
    pub ty: String,
    pub is_pub: bool,
}

/// Tóm tắt cây AST của một khai báo kiểu.
#[derive(Debug, Clone, PartialEq)]
pub struct AstSummary {
    pub type_name: String,
    /// "struct", "enum" hoặc "union"
    pub kind: &'static str,
    pub fields: Vec<FieldInfo>,
}

/// Duyệt cây `DeriveInput` giống hệt việc một derive macro sẽ làm.
pub fn inspect_ast(source: &str) -> syn::Result<AstSummary> {
    let ast: DeriveInput = syn::parse_str(source)?;
    let (kind, fields) = match &ast.data {
        Data::Struct(s) => {
            let fields = match &s.fields {
                Fields::Named(named) => named.named.iter().collect(),
                Fields::Unnamed(unnamed) => unnamed.unnamed.iter().collect(),
                Fields::Unit => Vec::new(),
            };
            ("struct", fields)
        }
        Data::Enum(_) => ("enum", Vec::new()),
        Data::Union(_) => ("union", Vec::new()),
    };
    let fields = fields
        .into_iter()
        .map(|f| FieldInfo {
            name: f.ident.as_ref().map(|i| i.to_string()),
            ty: f.ty.to_token_stream().to_string(),
            is_pub: matches!(f.vis, Visibility::Public(_)),
        })
        .collect();
    Ok(AstSummary {
        type_name: ast.ident.to_string(),
        kind,
        fields,
    })
}

// ============================================================================
// CHƯƠNG TRÌNH THỰC THI CHÍNH
// ============================================================================

fn main() {
    println!("============================================================");
    println!("      KIẾN TRÚC PROCEDURAL MACROS: SYN, QUOTE & AST         ");
    println!("============================================================");

    // 1. Soi cây AST bằng `syn` — đúng thứ mà derive macro nhận được
    println!("\n1. Phân tích Cây cú pháp AST bằng `syn`:");
    let source = "pub struct NetworkDevice { pub ip_address: String, pub service_port: u16, is_active: bool }";
    match inspect_ast(source) {
        Ok(summary) => {
            println!(
                "- Phát hiện một {} tên `{}`",
                summary.kind, summary.type_name
            );
            for f in &summary.fields {
                println!(
                    "  + trường {:<14} kiểu {:<8} {}",
                    f.name.as_deref().unwrap_or("<không tên>"),
                    f.ty,
                    if f.is_pub { "(pub)" } else { "(riêng tư)" }
                );
            }
        }
        Err(e) => println!("- syn báo lỗi cú pháp: {}", e),
    }

    // syn::Error cũng là thứ derive macro trả về để báo lỗi đúng vị trí
    match inspect_ast("struct Broken { x: }") {
        Ok(_) => println!("(không tới đây)"),
        Err(e) => println!("- Mã nguồn hỏng -> syn::Error: {}", e),
    }

    // 2. Gọi các phương thức do derive macro THẬT sinh ra lúc biên dịch
    println!("\n2. Thực thi phương thức được `quote!` sinh tự động:");
    let router = NetworkDevice {
        ip_address: String::from("192.168.1.1"),
        service_port: 443,
        is_active: true,
    };
    router.print_details();
    println!(
        "Tên các trường: {:?} (tổng {} trường)",
        NetworkDevice::field_names(),
        NetworkDevice::field_count()
    );

    // 3. Struct generic vẫn được hỗ trợ nhờ split_for_impl()
    println!("\n3. Derive trên struct generic Pair<T>:");
    let pair = Pair {
        left: "trái",
        right: "phải",
    };
    println!("{}", pair.describe());

    println!("\n============================================================");
    println!("   XÁC MINH KIẾN TRÚC PROCEDURAL MACROS HOÀN TOÀN THÀNH CÔNG");
    println!("============================================================");
}

// ============================================================================
// KIỂM THỬ: MÃ DO MACRO SINH RA ĐƯỢC KIỂM NHƯ MÃ VIẾT TAY
// ============================================================================

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn derive_generates_field_names_and_count() {
        assert_eq!(
            NetworkDevice::field_names(),
            &["ip_address", "service_port", "is_active"]
        );
        assert_eq!(NetworkDevice::field_count(), 3);
    }

    #[test]
    fn derive_generates_describe_with_values() {
        let d = NetworkDevice {
            ip_address: "10.0.0.1".into(),
            service_port: 22,
            is_active: false,
        };
        let text = d.describe();
        assert!(text.starts_with("THÔNG TIN THỰC THỂ: [NetworkDevice]"));
        assert!(text.contains("`service_port`: 22"));
        assert!(text.contains("`ip_address`: \"10.0.0.1\""));
    }

    #[test]
    fn derive_supports_generics() {
        assert_eq!(Pair::<i32>::field_count(), 2);
        let p = Pair { left: 1, right: 2 };
        assert!(p.describe().contains("`right`: 2"));
    }

    #[test]
    fn syn_sees_names_types_and_visibility() {
        let s = inspect_ast("struct Account { pub name: String, balance: f64 }").unwrap();
        assert_eq!(s.type_name, "Account");
        assert_eq!(s.kind, "struct");
        assert_eq!(
            s.fields,
            vec![
                FieldInfo {
                    name: Some("name".into()),
                    ty: "String".into(),
                    is_pub: true
                },
                FieldInfo {
                    name: Some("balance".into()),
                    ty: "f64".into(),
                    is_pub: false
                },
            ]
        );
    }

    #[test]
    fn tuple_struct_fields_have_no_name() {
        let s = inspect_ast("struct Point(f32, f32);").unwrap();
        assert_eq!(s.fields.len(), 2);
        assert!(s.fields.iter().all(|f| f.name.is_none()));
    }

    #[test]
    fn syn_reports_enum_and_syntax_errors() {
        assert_eq!(
            inspect_ast("enum Color { Red, Green }").unwrap().kind,
            "enum"
        );
        assert!(inspect_ast("struct Broken { x: }").is_err());
    }
}
```

---

## Bảng tra cứu lỗi biên dịch & Cách khắc phục (Compiler Error Guide)

Khi xây dựng và sử dụng Procedural Macros trong Rust, lập trình viên thường gặp các lỗi cấu hình crate và cú pháp AST đặc thù:

| Mã lỗi | Thông báo mẫu từ trình biên dịch | Nguyên nhân cốt lõi | Cách khắc phục nhanh |
|---|---|---|---|
| **Lỗi cấu hình crate** (không có mã) | ``the `#[proc_macro_derive]` attribute is only usable with crates of the `proc-macro` crate type`` | Bạn viết hàm `#[proc_macro_derive]` (hoặc `#[proc_macro]`, `#[proc_macro_attribute]`) ngay trong `main.rs` hay một crate thư viện thường. | Tạo một crate thư viện riêng và bổ sung `[lib] proc-macro = true` vào `Cargo.toml` của nó. |
| **Panic lúc chạy** (không phải lỗi biên dịch) | `procedural macro API is used outside of a procedural macro` | Bạn viết `extern crate proc_macro;` trong crate nhị phân — dòng này **biên dịch bình thường**, nhưng khi chương trình gọi tới API như `"...".parse::<proc_macro::TokenStream>()` thì panic, vì cầu nối tới trình biên dịch chỉ tồn tại khi mã chạy *bên trong* `rustc`. | Ngoài macro, dùng `proc_macro2::TokenStream` và `syn::parse_str` (chạy được ở mọi nơi, như hàm `inspect_ast` ở trên). |
| **Lỗi phân tích của syn** | `expected identifier` | Khi phân tích cú pháp AST, token tiếp theo không phải là một định danh hợp lệ như `syn` mong đợi (vd `syn::parse_str::<syn::Ident>("123")`). | Kiểm tra lại cú pháp người dùng truyền vào hoặc cài `syn::parse::Parse` tuỳ biến để xử lý các token đặc thù. |
| **E0277** | `the trait bound 'NotTokens: ToTokens' is not satisfied` | Trong khối `quote! { #var }`, biến `#var` không triển khai trait `quote::ToTokens` (nghĩa là `quote` không biết cách chuyển biến này thành mã Rust). | Đảm bảo kiểu dữ liệu đưa vào `#var` là một thành phần AST của `syn` (như `Ident`, `Type`), một `TokenStream`, hoặc kiểu nguyên thủy có sẵn `ToTokens` (số, `&str`, `String`…). |
| **Lỗi phân giải tên** (không có mã) | `cannot find derive macro 'Mystery' in this scope` | Crate ứng dụng chưa nhập (import) derive macro từ crate proc-macro, hoặc gõ sai tên. | Thêm crate macro vào `[dependencies]` và khai báo `use my_macro_crate::MacroName;`. |
| **E0404** | ``expected trait, found derive macro `DetailedDescription` `` | Mã sinh ra viết `impl DetailedDescription for ...` (không kèm đường dẫn), nhưng ở chỗ `#[derive]` trait cùng tên **không** nằm trong phạm vi — chỉ có derive macro. | Đưa trait vào phạm vi (`use crate::...::DetailedDescription;`), hoặc cho macro sinh đường dẫn đầy đủ tới trait. |

### Phân tích lỗi thực tế: Cố tình viết Proc Macro trong Crate thông thường

```rust
// Đoạn mã lỗi minh họa (trong tệp main.rs thông thường):
// extern crate proc_macro;          // ← dòng này KHÔNG lỗi: crate proc_macro luôn có sẵn
// use proc_macro::TokenStream;
//
// #[proc_macro_derive(Foo)]         // ← LỖI ở đây:
// pub fn foo(input: TokenStream) -> TokenStream { input }
// error: the `#[proc_macro_derive]` attribute is only usable with crates
//        of the `proc-macro` crate type
//
// Và nếu bỏ hàm trên đi nhưng vẫn gọi API proc_macro lúc chạy:
// let ts: proc_macro::TokenStream = "struct A;".parse().unwrap();
// → biên dịch được, nhưng PANIC: "procedural macro API is used outside of a procedural macro"

// Cách khắc phục chuẩn:
// 1. Tổ chức dự án dạng Workspace (giống ch23 / ch23_macros ở trên):
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
   - `quote`: Cây bút ma thuật dập khuôn và sinh mã Rust mới một cách an toàn thông qua `#var`.
4. **Báo lỗi có tâm**: Dùng `syn::Error::new_spanned` kết hợp `to_compile_error()` để định vị chính xác vị trí lỗi đỏ trên màn hình người dùng.

### Bài tập rèn luyện tự giải:
1. **Bài tập 1 (Bóc tách AST bằng tư duy)**:  
   Cho một struct sau:
   ```rust
   struct Account {
       pub name: String,
       balance: f64,
   }
   ```
   Dựa trên các cấu trúc của `syn` (`DeriveInput`, `DataStruct`, `FieldsNamed`), hãy vẽ sơ đồ hình cây biểu diễn các nút cha - con của struct này trong bộ nhớ của trình phân tích AST.

2. **Bài tập 2 (Thiết kế Ý tưởng Derive Macro)**:  
   Hãy tưởng tượng bạn đang viết một Derive Macro mang tên `#[derive(ToJson)]` (xuất struct ra chuỗi JSON). Theo bạn, macro này sẽ cần bóc tách những thông tin gì từ AST của struct và sẽ dùng `quote!` để sinh ra phương thức gì cho struct đó?

3. **Bài tập 3 (So sánh Kiến trúc)**:  
   Tại sao Rust lại quy định khắt khe rằng Macro thủ tục phải nằm trong một Crate riêng biệt và biên dịch thành thư viện động lúc Host Time, thay vì cho phép viết lẫn lộn trong `main.rs` như `macro_rules!`?

---

### Gợi ý & Lời giải

<details>
<summary><b>Bài tập 1 — Gợi ý</b></summary>

Hình dung `syn` bóc struct thành các tầng: gốc là `DeriveInput`, dưới đó là `Data::Struct`, rồi tới `Fields::Named`, rồi từng `Field`. Mỗi `Field` lại có tên, kiểu và mức hiển thị.
</details>

<details>
<summary><b>Bài tập 1 — Lời giải</b></summary>

```text
DeriveInput
├── attrs: []                       (không có #[...] nào)
├── vis: Inherited                  (struct không có `pub`)
├── ident: "Account"                ← TÊN struct
├── generics: <>                    (không có tham số kiểu)
└── data: Data::Struct(DataStruct)
    └── fields: Fields::Named(FieldsNamed)
        ├── Field
        │   ├── vis:   Visibility::Public   ← `pub`
        │   ├── ident: Some("name")
        │   └── ty:    Type::Path("String")
        └── Field
            ├── vis:   Visibility::Inherited
            ├── ident: Some("balance")
            └── ty:    Type::Path("f64")
```

Ba điểm đáng nhớ khi đọc cây này:

- **`ident` là `Option`** vì struct dạng tuple (`struct Point(f32, f32)`) có trường *không tên*. Macro nào giả định `ident` luôn có sẽ vỡ ngay khi gặp tuple struct.
- **`vis` nằm ở từng trường**, không phải ở struct — nên macro sinh getter phải tự quyết định có tôn trọng `pub` hay không.
- **`ty` là `Type::Path`, không phải chuỗi.** `String` ở đây là một *đường dẫn cú pháp*, chưa được phân giải. Macro thủ tục chạy **trước** khi kiểm tra kiểu, nên nó không biết `String` là gì — nó chỉ thấy ba ký tự đó.
</details>

<details>
<summary><b>Bài tập 2 — Gợi ý</b></summary>

Muốn sinh ra `to_json()` thì cần đúng ba thứ: tên struct, danh sách tên trường, và cách gọi định dạng cho từng trường.
</details>

<details>
<summary><b>Bài tập 2 — Lời giải</b></summary>

**Cần bóc từ AST:**

| Thông tin | Lấy từ đâu | Dùng để làm gì |
|---|---|---|
| Tên struct | `input.ident` | Viết `impl ToJson for <tên>` |
| Danh sách trường | `Fields::Named` | Sinh một dòng JSON cho mỗi trường |
| Tên từng trường | `field.ident` | Làm **khoá** JSON |
| Kiểu từng trường | `field.ty` | Chọn cách định dạng (chuỗi cần dấu nháy, số thì không) |
| Generics | `input.generics` | Truyền lại vào `impl` để struct generic vẫn dùng được |

**Mã sinh ra bằng `quote!`:**

```text
// Phác hoạ phần thân macro thủ tục
let name = &input.ident;
let fields = /* trích từ Fields::Named */;
let entries: Vec<_> = fields.iter().map(|f| {
    let key = f.ident.as_ref().unwrap();
    let label = key.to_string();
    quote! { format!("\"{}\": {:?}", #label, self.#key) }
}).collect();

quote! {
    impl ToJson for #name {
        fn to_json(&self) -> String {
            let parts = vec![ #( #entries ),* ];
            format!("{{{}}}", parts.join(", "))
        }
    }
}
```

**Chỗ khó thật sự** không nằm ở việc sinh mã mà ở **kiểu lồng nhau**: `Vec<Account>` thì phải gọi `to_json()` trên từng phần tử, chứ `{:?}` sẽ ra sai. Vì macro chạy trước kiểm tra kiểu nên nó *không biết* `Vec<Account>` là gì — nó chỉ thấy một `Type::Path`. Đó là lý do `serde` không tự đoán mà **yêu cầu mọi kiểu lồng bên trong cũng phải cài `Serialize`**, rồi để trình biên dịch kiểm tra giúp.
</details>

<details>
<summary><b>Bài tập 3 — Gợi ý</b></summary>

Macro thủ tục là **chương trình chạy trong lòng trình biên dịch**. Hãy nghĩ xem điều đó kéo theo ràng buộc gì về thứ tự biên dịch.
</details>

<details>
<summary><b>Bài tập 3 — Lời giải</b></summary>

Vì macro thủ tục **được biên dịch trước, rồi chạy** trong tiến trình `rustc` khi biên dịch crate của bạn. Điều đó kéo theo ba ràng buộc:

**1. Bài toán con gà và quả trứng.** Nếu macro nằm trong `main.rs`, thì để biên dịch `main.rs`, `rustc` cần chạy macro; nhưng để chạy macro, nó phải biên dịch xong `main.rs`. Vòng lặp không lối thoát. Tách ra crate riêng phá vòng: crate macro biên dịch **xong hẳn** thành thư viện động, rồi crate của bạn nạp nó lên.

**2. Kiến trúc đích khác nhau.** Bạn có thể biên dịch chéo cho ARM, nhưng macro phải chạy trên máy **của bạn** (x86 chẳng hạn). Crate macro được biên dịch cho *host*, crate ứng dụng cho *target*. Trộn chung một crate thì không có cách nào diễn đạt hai đích khác nhau.

**3. Ranh giới an toàn.** Macro thủ tục là mã tuỳ ý chạy lúc biên dịch — nó đọc được tệp, mở được mạng. Bắt nó nằm trong crate khai báo `proc-macro = true` khiến điều đó **hiện rõ trong `Cargo.toml`**, thay vì lẩn khuất giữa mã ứng dụng.

`macro_rules!` không có vấn đề này vì nó không phải chương trình: nó là **quy tắc thay thế mẫu** mà chính `rustc` diễn giải, không cần biên dịch gì trước.
</details>
