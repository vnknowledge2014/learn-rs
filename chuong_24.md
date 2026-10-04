# Chương 24: Chế tạo Macro: Custom Derive, Thuộc tính và Macro dạng hàm (Custom Derive, Attribute & Function-like Macros)

## Giới thiệu & Mục tiêu học tập

Chúc mừng bạn đã tiến bước đến chương đỉnh cao của Chủ đề 4: Siêu lập trình (Meta Programming)! Ở Chương 23, bạn đã làm quen với các khái niệm nền tảng của Macro thủ tục: Cây cú pháp trừu tượng (Abstract Syntax Tree — AST), kính hiển vi bóc tách `syn`, và cây bút ma thuật sinh mã `quote`.

Trong thế giới Rust thực chiến, Macro thủ tục không chỉ gói gọn trong một hình thức duy nhất mà được chia thành **Ba nhánh sức mạnh tối thượng (The Trinity of Procedural Macros)**:
1. **Custom Derive Macro (`#[derive(MyTrait)]`)**: Tự động sinh mã triển khai một Trait cho struct hoặc enum mà không làm biến đổi mã gốc.
2. **Attribute-like Macro (`#[my_attribute]`)**: Gắn lên đầu hàm, struct hay mô-đun để can thiệp, biến đổi hoặc bọc lớp vỏ bảo vệ quanh đối tượng (như cách `#[tokio::main]` biến một hàm bất đồng bộ thành luồng chạy thực tế).
3. **Function-like Macro (`my_macro!(...)`)**: Nhận một ngôn ngữ đặc thù (DSL) tùy ý bên trong dấu ngoặc và dịch nó thành mã Rust chuẩn mực (như cách `sqlx::query!` kiểm tra cú pháp câu lệnh SQL ngay lúc biên dịch).

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
- Khi bạn có một thùng hàng táo tươi xuất khẩu (`struct FreshApples`):
  - Viên chức kiểm định đóng một con dấu đỏ "Đạt chuẩn an toàn thực phẩm" lên vỏ thùng (`#[derive(Inspected)]`).
  - Thùng táo bên trong không hề bị băm nhỏ hay sửa đổi. Nhưng nhờ con dấu đó, thùng táo tự động được cấp thêm một tập hồ sơ pháp lý cho phép nó thông quan qua cảng biển (`impl Export for FreshApples`).
  - **Quy tắc**: Derive macro **không bao giờ sửa đổi mã gốc**, nó chỉ sinh thêm mã triển khai Trait bên cạnh mã gốc.

### 2. Chiếc tem cảm biến nhiệt độ thông minh (Attribute-like Macro)
- Khi bạn dán một chiếc tem điện tử thông minh lên thùng vắc-xin (`#[temperature_guard]`):
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
#[derive(SecurityAudit)]
struct User {
    pub full_name: String,
    #[audit(skip)] // Thuộc tính bổ trợ: không in trường mật khẩu này!
    pub password: String,
}
```
Để trình biên dịch không báo lỗi *"cannot find attribute `audit` in this scope"*, bạn phải đăng ký tên thuộc tính này trong khai báo macro bằng tham số `attributes(...)`:

```rust
#[proc_macro_derive(SecurityAudit, attributes(audit))]
pub fn security_audit_derive(input: TokenStream) -> TokenStream {
    // rustc sẽ cho phép #[audit(...)] xuất hiện bên trong struct;
    // đọc nội dung của nó bằng attr.parse_nested_meta(...) (xem chương trình bên dưới)
    todo!()
}
```

### 3. Giải phẫu Attribute-like Macro: Cơ chế "Đóng gói bọc ngoài" (Decorator)

Attribute macro nhận vào hai dòng thẻ bài:
1. `attr`: Phần tham số nằm bên trong ngoặc vuông của thuộc tính (ví dụ: `#[check(role = "admin")]` thì `attr` chứa `role = "admin"`).
2. `item`: Toàn bộ đoạn mã của đối tượng bên dưới thuộc tính (ví dụ toàn bộ phần thân của hàm `fn handle() { ... }`).

```rust
#[proc_macro_attribute]
pub fn log_it(attr: TokenStream, item: TokenStream) -> TokenStream {
    let original_fn = parse_macro_input!(item as ItemFn);
    let fn_name = &original_fn.sig.ident;
    let body = &original_fn.block;
    let sig = &original_fn.sig;

    let expanded = quote! {
        #sig {
            println!(">>> [NHẬT KÝ] Bắt đầu gọi hàm: {}", stringify!(#fn_name));
            // Bọc thân gốc trong closure để `return` bên trong nó vẫn quay về đây
            let result = (|| #body )();
            println!(">>> [NHẬT KÝ] Kết thúc gọi hàm: {}", stringify!(#fn_name));
            result
        }
    };

    TokenStream::from(expanded)
}
```

### 4. Kỹ thuật gỡ lỗi Macro thủ tục với `eprintln!`

Vì Macro thủ tục chạy trực tiếp trong lúc bạn gõ lệnh `cargo build`, bạn có thể dùng lệnh `eprintln!` ngay trong thân hàm proc-macro để in thông tin phân tích AST ra màn hình Terminal. Lưu ý: các kiểu AST của `syn` chỉ cài `Debug` khi bật feature **`extra-traits`** (`syn = { version = "3", features = ["extra-traits"] }`), nếu không `{:#?}` sẽ báo `E0277: 'DeriveInput' doesn't implement 'Debug'`:
```rust
#[proc_macro_derive(Inspect)]
pub fn inspect_derive(input: TokenStream) -> TokenStream {
    let ast = parse_macro_input!(input as DeriveInput);
    eprintln!("DEBUG AST: {:#?}", ast); // In cây cú pháp ra terminal lúc build!
    TokenStream::new()
}
```

---

## Mã nguồn minh họa thực chiến (Idiomatic Runnable Rust Blueprint)

Dưới đây là một workspace thật gồm **hai crate**, cài đặt đủ **cả ba loại Macro thủ tục** cho một **Hệ thống Quản lý Tài khoản & Kiểm toán Bảo mật** (chạy: `cargo run -p ch24`, kiểm thử: `cargo test -p ch24`):
1. **Custom Derive** `#[derive(SecurityAudit)]` với thuộc tính bổ trợ `#[audit(sensitive)]` / `#[audit(skip)]` — tự động sinh danh sách trường an toàn để ghi nhật ký.
2. **Attribute-like** `#[require_role("admin", "owner")]` — bọc hàm, chèn đoạn kiểm soát quyền truy cập vào đầu thân hàm.
3. **Function-like** `config! { KEY = 123; ... }` — phân tích một DSL cấu hình và **kiểm tra lúc biên dịch** (khoá phải viết hoa, không được trùng).

**Bước 1 — Crate macro.** Tệp `ch24_macros/Cargo.toml`:

```toml
[package]
name = "ch24_macros"
version = "0.1.0"
edition = "2024"

[lib]
proc-macro = true

[dependencies]
# "full" để phân tích được cả hàm (ItemFn) cho macro thuộc tính
syn = { version = "3", features = ["full"] }
quote = "1"
proc-macro2 = "1"
```

Tệp `ch24_macros/src/lib.rs`:

```rust
// Tệp: ch24_macros/src/lib.rs
//! Crate macro thủ tục của Chương 24 — đủ BỘ BA macro thủ tục:
//!
//! 1. `#[derive(SecurityAudit)]` + thuộc tính bổ trợ `#[audit(...)]`  (Custom Derive)
//! 2. `#[require_role("admin", ...)]`                                (Attribute-like)
//! 3. `config! { KEY = 123; ... }`                                   (Function-like)

use std::collections::HashSet;

use proc_macro::TokenStream;
use quote::quote;
use syn::parse::{Parse, ParseStream};
use syn::punctuated::Punctuated;
use syn::{
    Data, DeriveInput, Fields, FnArg, Ident, ItemFn, LitInt, LitStr, Pat, Token, parse_macro_input,
    parse_quote,
};

// ============================================================================
// 1. CUSTOM DERIVE: #[derive(SecurityAudit)] VỚI HELPER ATTRIBUTE #[audit(...)]
// ============================================================================

/// Cách xử lý một trường khi xuất nhật ký kiểm toán.
enum FieldMode {
    /// In giá trị bình thường
    Plain,
    /// `#[audit(sensitive)]` — in tên trường nhưng che giá trị
    Sensitive,
    /// `#[audit(skip)]` — bỏ hẳn trường khỏi nhật ký
    Skip,
}

/// Đọc các thuộc tính `#[audit(...)]` trên một trường.
/// Tuỳ chọn lạ (gõ nhầm) phải là LỖI BIÊN DỊCH, không được âm thầm bỏ qua.
fn field_mode(field: &syn::Field) -> syn::Result<FieldMode> {
    let mut mode = FieldMode::Plain;
    for attr in &field.attrs {
        if !attr.path().is_ident("audit") {
            continue; // thuộc tính của người khác (vd #[allow]) — không phải việc của ta
        }
        attr.parse_nested_meta(|meta| {
            if meta.path.is_ident("sensitive") {
                mode = FieldMode::Sensitive;
                Ok(())
            } else if meta.path.is_ident("skip") {
                mode = FieldMode::Skip;
                Ok(())
            } else {
                Err(meta.error("tuỳ chọn không hợp lệ; chỉ chấp nhận `sensitive` hoặc `skip`"))
            }
        })?;
    }
    Ok(mode)
}

/// `attributes(audit)` đăng ký thuộc tính bổ trợ: nhờ nó rustc chấp nhận
/// `#[audit(...)]` xuất hiện trên các trường của struct.
#[proc_macro_derive(SecurityAudit, attributes(audit))]
pub fn security_audit_derive(input: TokenStream) -> TokenStream {
    let ast = parse_macro_input!(input as DeriveInput);
    match expand_security_audit(&ast) {
        Ok(tokens) => tokens.into(),
        Err(err) => err.to_compile_error().into(),
    }
}

fn expand_security_audit(ast: &DeriveInput) -> syn::Result<proc_macro2::TokenStream> {
    let name = &ast.ident;
    let fields = match &ast.data {
        Data::Struct(s) => match &s.fields {
            Fields::Named(named) => &named.named,
            _ => {
                return Err(syn::Error::new_spanned(
                    name,
                    "SecurityAudit chỉ hỗ trợ struct có trường đặt tên",
                ));
            }
        },
        _ => {
            return Err(syn::Error::new_spanned(
                name,
                "SecurityAudit chỉ hỗ trợ struct",
            ));
        }
    };

    let mut entries = Vec::new();
    for field in fields {
        let ident = field.ident.as_ref().expect("Fields::Named luôn có tên");
        let label = ident.to_string();
        match field_mode(field)? {
            FieldMode::Plain => {
                entries.push(quote! { (#label, ::std::string::ToString::to_string(&self.#ident)) })
            }
            FieldMode::Sensitive => entries.push(quote! {
                (#label, ::std::string::String::from("***ĐÃ ẨN***"))
            }),
            FieldMode::Skip => {}
        }
    }

    let (impl_generics, ty_generics, where_clause) = ast.generics.split_for_impl();
    // Tên trait để KHÔNG kèm đường dẫn vì trait nằm ở crate người dùng (ch24).
    // Thư viện thật sẽ sinh `::my_lib::SecurityAudit` (xem bảng lỗi E0405).
    Ok(quote! {
        impl #impl_generics SecurityAudit for #name #ty_generics #where_clause {
            fn audit_fields(&self) -> ::std::vec::Vec<(&'static str, ::std::string::String)> {
                ::std::vec![ #( #entries ),* ]
            }
            fn entity_kind() -> &'static str {
                stringify!(#name)
            }
        }
    })
}

// ============================================================================
// 2. ATTRIBUTE-LIKE: #[require_role("admin", "owner")]
// ============================================================================

/// Bọc một hàm: chèn đoạn KIỂM TRA VAI TRÒ vào đầu thân hàm.
///
/// Quy ước: hàm được bọc phải có tham số tên `role` (kiểu `&str`) và trả về
/// `Result<_, E>` với `E: From<&'static str>`.
#[proc_macro_attribute]
pub fn require_role(attr: TokenStream, item: TokenStream) -> TokenStream {
    // `attr` = phần trong ngoặc của thuộc tính: "admin", "owner"
    let roles = parse_macro_input!(attr with Punctuated::<LitStr, Token![,]>::parse_terminated);
    // `item` = toàn bộ hàm bên dưới thuộc tính
    let mut func = parse_macro_input!(item as ItemFn);

    if roles.is_empty() {
        return syn::Error::new_spanned(
            &func.sig.ident,
            "require_role cần ít nhất một vai trò, vd #[require_role(\"admin\")]",
        )
        .to_compile_error()
        .into();
    }

    // Tìm tham số tên `role` trong chữ ký hàm
    let has_role_arg = func.sig.inputs.iter().any(|arg| match arg {
        FnArg::Typed(pt) => matches!(&*pt.pat, Pat::Ident(p) if p.ident == "role"),
        FnArg::Receiver(_) => false,
    });
    if !has_role_arg {
        return syn::Error::new_spanned(
            &func.sig,
            "hàm dùng #[require_role] phải có tham số `role: &str`",
        )
        .to_compile_error()
        .into();
    }

    let fn_name = func.sig.ident.to_string();
    let roles: Vec<&LitStr> = roles.iter().collect();
    let original = &func.block;

    // Thân hàm mới = [kiểm tra vai trò] + [thân hàm gốc, giữ nguyên]
    let new_block: syn::Block = parse_quote! {{
        println!("[BẢO VỆ] `{}` kiểm tra vai trò '{}'", #fn_name, role);
        if ![ #( #roles ),* ].contains(&role) {
            return ::core::result::Result::Err(::core::convert::From::from(
                "Từ chối truy cập: vai trò không được phép thực hiện thao tác này",
            ));
        }
        #original
    }};
    *func.block = new_block;

    // In lại TOÀN BỘ hàm (thuộc tính khác, pub, chữ ký... giữ nguyên)
    quote!(#func).into()
}

// ============================================================================
// 3. FUNCTION-LIKE: config! { TIMEOUT = 30; MAX_RETRY = 3; }
// ============================================================================

/// Một dòng `KEY = 123;` của DSL cấu hình.
struct ConfigEntry {
    key: Ident,
    value: LitInt,
}

/// Toàn bộ đầu vào của `config!`.
struct ConfigInput {
    entries: Vec<ConfigEntry>,
}

/// Tự viết bộ phân tích cú pháp cho DSL bằng trait `syn::parse::Parse`.
impl Parse for ConfigInput {
    fn parse(input: ParseStream) -> syn::Result<Self> {
        let mut entries = Vec::new();
        while !input.is_empty() {
            let key: Ident = input.parse()?;
            input.parse::<Token![=]>()?;
            let value: LitInt = input.parse()?;
            input.parse::<Token![;]>()?;
            entries.push(ConfigEntry { key, value });
        }
        Ok(ConfigInput { entries })
    }
}

/// Dựng `HashMap<&'static str, i64>` từ DSL, và KIỂM TRA LÚC BIÊN DỊCH:
/// khoá phải VIẾT_HOA, không được trùng, giá trị phải vừa `i64`.
/// (Phát hiện khoá trùng là việc `macro_rules!` không làm được.)
#[proc_macro]
pub fn config(input: TokenStream) -> TokenStream {
    let parsed = parse_macro_input!(input as ConfigInput);
    match expand_config(parsed) {
        Ok(tokens) => tokens.into(),
        Err(err) => err.to_compile_error().into(),
    }
}

fn expand_config(parsed: ConfigInput) -> syn::Result<proc_macro2::TokenStream> {
    let mut seen = HashSet::new();
    let mut inserts = Vec::new();
    for ConfigEntry { key, value } in &parsed.entries {
        let name = key.to_string();
        if name.chars().any(|c| c.is_ascii_lowercase()) {
            return Err(syn::Error::new_spanned(
                key,
                format!("khoá cấu hình phải VIẾT_HOA: `{}`", name),
            ));
        }
        if !seen.insert(name.clone()) {
            return Err(syn::Error::new_spanned(
                key,
                format!("khoá cấu hình `{}` bị khai báo hai lần", name),
            ));
        }
        let number: i64 = value.base10_parse()?;
        inserts.push(quote! { map.insert(#name, #number); });
    }
    Ok(quote! {{
        let mut map: ::std::collections::HashMap<&'static str, i64> =
            ::std::collections::HashMap::new();
        #( #inserts )*
        map
    }})
}
```

**Bước 2 — Crate dùng macro.** Tệp `ch24/Cargo.toml`:

```toml
[package]
name = "ch24"
version = "0.1.0"
edition = "2024"

[dependencies]
# Crate macro thủ tục do chính chương này viết (proc-macro = true)
ch24_macros = { path = "../ch24_macros" }
```

Tệp `ch24/src/main.rs`:

```rust
// Tệp: src/main.rs
// Chương trình thực chiến làm chủ Custom Derive, Attribute và Function-like Macros trong Rust
//
// Ba macro dùng ở đây đều là macro thủ tục THẬT, viết trong crate `ch24_macros`
// (proc-macro = true) bằng syn + quote.

use ch24_macros::{SecurityAudit, config, require_role};

// ============================================================================
// 1. CUSTOM DERIVE + HELPER ATTRIBUTE: #[derive(SecurityAudit)] / #[audit(...)]
// ============================================================================

/// Trait mà `#[derive(SecurityAudit)]` tự động cài đặt.
/// (Crate proc-macro không xuất được trait, nên trait sống ở crate này.)
pub trait SecurityAudit {
    /// Danh sách (tên trường, giá trị) AN TOÀN để ghi nhật ký
    fn audit_fields(&self) -> Vec<(&'static str, String)>;
    /// Tên loại thực thể (sinh từ tên struct)
    fn entity_kind() -> &'static str;
}

#[derive(SecurityAudit)]
pub struct BankAccount {
    pub account_number: String,
    pub account_owner: String,
    #[audit(sensitive)] // Trường nhạy cảm: hiện tên, che giá trị
    pub pin_code: String,
    #[audit(skip)] // Bỏ hẳn khỏi nhật ký
    pub internal_note: String,
}

// ============================================================================
// 2. ATTRIBUTE-LIKE MACRO: #[require_role(...)] BỌC HÀM BẰNG LỚP KIỂM QUYỀN
// ============================================================================

/// Lập trình viên chỉ viết phần thân nghiệp vụ. Macro chèn đoạn kiểm tra
/// `role` vào ĐẦU thân hàm; vai trò không nằm trong danh sách sẽ bị trả `Err`
/// trước khi dòng nghiệp vụ nào kịp chạy.
#[require_role("admin", "owner")]
pub fn safe_transfer(
    sender: &str,
    recipient: &str,
    amount_vnd: u64, // tiền tệ: số nguyên đơn vị nhỏ nhất, KHÔNG dùng f64
    role: &str,
) -> Result<String, &'static str> {
    println!(
        "  -> Đang chuyển {} đồng từ {} sang {}",
        amount_vnd, sender, recipient
    );
    let transaction_id = "GD-99882233";
    Ok(format!(
        "Chuyển tiền thành công! Mã giao dịch: {}",
        transaction_id
    ))
}

// ============================================================================
// 3. FUNCTION-LIKE MACRO: config! { ... } — DSL CẤU HÌNH KIỂM TRA LÚC BIÊN DỊCH
// ============================================================================

/// Viết `MAX_RETRY = 3;` hai lần, hay `timeout = 30;` (chữ thường), thì chương
/// trình KHÔNG biên dịch được — lỗi trỏ đúng vào khoá sai.
fn load_config() -> std::collections::HashMap<&'static str, i64> {
    config! {
        TIMEOUT = 30;
        MAX_RETRY = 3;
        PORT = 8443;
    }
}

// ============================================================================
// CHƯƠNG TRÌNH THỰC THI CHÍNH
// ============================================================================

fn main() {
    println!("============================================================");
    println!("     CHẾ TẠO VÀ ỨNG DỤNG BỘ BA PROCEDURAL MACROS TRONG RUST ");
    println!("============================================================");

    // ------------------------------------------------------------------------
    // 1. Custom Derive Macro với Helper Attribute
    // ------------------------------------------------------------------------
    println!("\n1. Ứng dụng Custom Derive Macro [SecurityAudit]:");
    let account = BankAccount {
        account_number: String::from("1900-8888-9999"),
        account_owner: String::from("Nguyễn Văn An"),
        pin_code: String::from("SecretPin1234"),
        internal_note: String::from("khách VIP"),
    };

    println!("Loại thực thể: {}", BankAccount::entity_kind());
    println!("Danh sách trường được xuất ra an toàn:");
    for (field_name, value) in account.audit_fields() {
        println!("  - {}: {}", field_name, value);
    }

    // ------------------------------------------------------------------------
    // 2. Attribute-like Macro bọc lớp bảo vệ
    // ------------------------------------------------------------------------
    println!("\n2. Ứng dụng Attribute Macro kiểm soát quyền truy cập:");

    // Thử nghiệm gọi với quyền hợp lệ
    match safe_transfer("NguyenVanA", "TranThiB", 5_000_000, "owner") {
        Ok(msg) => println!("  [OK] {}", msg),
        Err(e) => println!("  [LỖI] {}", e),
    }

    // Thử nghiệm gọi với quyền trái phép (Bị chặn ngay ở cổng)
    match safe_transfer("NguyenVanA", "KeXau", 999_999_000, "guest") {
        Ok(msg) => println!("  [NGUY HIỂM] Lọt qua kiểm duyệt: {}", msg),
        Err(reason) => println!("  [CHẶN THÀNH CÔNG] {}", reason),
    }

    // ------------------------------------------------------------------------
    // 3. Function-like Macro xử lý DSL tùy biến
    // ------------------------------------------------------------------------
    println!("\n3. Ứng dụng Function-like Macro khởi tạo cấu hình bảo mật:");
    let config = load_config();
    let mut keys: Vec<_> = config.keys().collect();
    keys.sort();
    for k in keys {
        println!(
            "  Tham số hệ thống `{}` được nạp với giá trị: {}",
            k, config[k]
        );
    }

    println!("\n============================================================");
    println!("     HOÀN TẤT CHƯƠNG TRÌNH LÀM CHỦ BỘ BA PROCEDURAL MACROS  ");
    println!("============================================================");
}

// ============================================================================
// KIỂM THỬ: KIỂM CHỨNG MÃ DO CẢ BA MACRO SINH RA
// ============================================================================

#[cfg(test)]
mod tests {
    use super::*;

    fn sample() -> BankAccount {
        BankAccount {
            account_number: "123".into(),
            account_owner: "An".into(),
            pin_code: "0000".into(),
            internal_note: "bí mật".into(),
        }
    }

    #[test]
    fn derive_masks_sensitive_and_drops_skipped_fields() {
        let fields = sample().audit_fields();
        assert_eq!(
            fields,
            vec![
                ("account_number", "123".to_string()),
                ("account_owner", "An".to_string()),
                ("pin_code", "***ĐÃ ẨN***".to_string()),
            ]
        );
        // Giá trị thật của PIN và ghi chú KHÔNG bao giờ lọt vào nhật ký
        assert!(fields.iter().all(|(_, v)| v != "0000" && v != "bí mật"));
    }

    #[test]
    fn derive_generates_entity_kind() {
        assert_eq!(BankAccount::entity_kind(), "BankAccount");
    }

    #[test]
    fn attribute_allows_listed_roles() {
        assert!(safe_transfer("A", "B", 1, "admin").is_ok());
        assert!(safe_transfer("A", "B", 1, "owner").is_ok());
    }

    #[test]
    fn attribute_blocks_other_roles_before_body_runs() {
        let err = safe_transfer("A", "B", 1, "guest").unwrap_err();
        assert!(err.starts_with("Từ chối truy cập"));
    }

    #[test]
    fn function_like_macro_builds_map() {
        let c = load_config();
        assert_eq!(c.len(), 3);
        assert_eq!(c["PORT"], 8443);
        assert_eq!(c["MAX_RETRY"], 3);
    }
}
```

> **Thử làm hỏng để thấy lỗi biên dịch "có tâm":** đổi `#[audit(sensitive)]` thành `#[audit(sensitiv)]`, hoặc viết `MAX_RETRY = 3;` hai lần trong `config!`, hoặc đổi tên tham số `role` của `safe_transfer` — trình biên dịch sẽ báo đúng thông điệp tiếng Việt mà macro tạo bằng `syn::Error::new_spanned`, gạch chân đúng token sai.

---

## Bảng tra cứu lỗi biên dịch & Cách khắc phục (Compiler Error Guide)

Dưới đây là các lỗi thường gặp nhất khi triển khai và sử dụng Custom Derive và Attribute Macros:

| Mã lỗi | Thông báo mẫu từ trình biên dịch | Nguyên nhân cốt lõi | Cách khắc phục nhanh |
|---|---|---|---|
| **E0405** / **E0412** | `cannot find trait 'MyTrait' in this scope` / `cannot find type '...' in this scope` | Mã do macro sinh ra tham chiếu đến một trait hoặc kiểu dữ liệu nhưng ở chỗ dùng macro, người dùng chưa `use` nó vào phạm vi. | Sinh đường dẫn tuyệt đối trong thân `quote!` (ví dụ: `::core::fmt::Display`, `::std::string::String`, `::my_crate::MyTrait`). Lưu ý: `$crate` **chỉ có trong `macro_rules!`**, macro thủ tục không dùng được — vì vậy các thư viện như `serde` tách thành crate "mặt tiền" (`serde`) re-export cả trait lẫn macro từ crate `serde_derive`. |
| **Lỗi phân giải tên** (không có mã) | `cannot find attribute 'audit' in this scope` | Bạn dùng thuộc tính phụ `#[audit(...)]` trên trường struct nhưng chưa đăng ký nó trong khai báo `#[proc_macro_derive(SecurityAudit, attributes(audit))]` (hoặc struct không hề derive macro đó). | Bổ sung tên thuộc tính phụ vào danh sách `attributes(...)` của proc macro derive. |
| **Lỗi chữ ký hàm** (không có mã) | `attribute proc macro has incorrect signature` (kèm `expected signature 'fn(TokenStream, TokenStream) -> TokenStream'`) | Bạn khai báo một Attribute Macro nhưng chỉ nhận một tham số `TokenStream` thay vì hai tham số (`attr` và `item`). | Sửa chữ ký thành `fn my_macro(attr: TokenStream, item: TokenStream) -> TokenStream`. |
| **Macro panic** (không có mã) | `proc-macro derive panicked` (kèm `help: message: called 'Result::unwrap()' on an 'Err' value: Error("expected 'fn'")`) | Macro gọi `.unwrap()` trên kết quả phân tích của `syn`; dữ liệu người dùng không khớp cấu trúc mong đợi nên macro panic giữa lúc biên dịch, và thông báo trỏ vào cả lời gọi macro thay vì token sai. | Thay vì `.unwrap()`, dùng `parse_macro_input!` / `syn::parse::Parse` và trả `syn::Error` bằng `.to_compile_error()` — như cả ba macro của `ch24_macros`. |
| **Lỗi do macro tự báo** (không có mã) | `tuỳ chọn không hợp lệ; chỉ chấp nhận 'sensitive' hoặc 'skip'` | Đây là lỗi **chính macro của bạn** tạo ra bằng `meta.error(...)` / `syn::Error::new_spanned` khi gặp `#[audit(sensitiv)]`, `config! { timeout = 1; }`… | Sửa đầu vào theo đúng thông điệp. Với tác giả macro: đây là cách báo lỗi đúng đắn — gạch chân đúng token sai, nói rõ cần gì. |

### Phân tích lỗi thực tế: Quên khai báo Helper Attribute

```text
// Đoạn mã lỗi minh họa (trong Crate thư viện proc-macro):
// Khai báo thiếu attributes(audit):
// #[proc_macro_derive(SecurityAudit)]
// pub fn security_audit_derive(input: TokenStream) -> TokenStream { ... }

// Khi người dùng áp dụng:
// #[derive(SecurityAudit)]
// struct User {
//     #[audit(skip)] // LỖI: cannot find attribute `audit` in this scope
//     password: String,
// }

// Cách khắc phục chuẩn xác:
// #[proc_macro_derive(SecurityAudit, attributes(audit))]
// pub fn security_audit_derive(input: TokenStream) -> TokenStream { ... }
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
   - Tự động sinh phương thức `fn default_init() -> Self` cho 20 struct khác nhau trong dự án.
   - Viết một bộ định tuyến cho Web Server kiểm tra quyền hạn `#[require_admin]` trước khi thực thi hàm xử lý.
   - Viết một tiện ích `make_list!(1, 2, 3)` nhận số lượng tham số tùy ý.

2. **Bài tập 2 (Thiết kế Helper Attribute)**:  
   Nếu bạn viết một Derive Macro mang tên `#[derive(Validation)]`, bạn sẽ thiết kế những thuộc tính phụ (Helper Attributes) nào trên các trường dữ liệu (ví dụ: kiểm tra độ dài chuỗi, kiểm tra số dương...)? Hãy mô tả cú pháp bạn mong muốn người dùng sử dụng.

3. **Bài tập 3 (Tổng kết Tư duy Siêu lập trình)**:  
   Siêu lập trình là một công cụ cực kỳ mạnh mẽ, nhưng tại sao các chuyên gia Rust luôn khuyên: *"Nếu bài toán có thể giải quyết được bằng Hàm (fn) hoặc Kiểu tổng quát (Generics), đừng bao giờ vội vàng viết Macro"*? Hãy nêu 3 nhược điểm lớn của việc lạm dụng macro trong dự án.

---

### Gợi ý & Lời giải

<details>
<summary><b>Bài tập 1 — Gợi ý</b></summary>

Ba câu hỏi phân loại: cần sinh `impl` cho nhiều kiểu → Derive; cần *bọc* một hàm có sẵn → Attribute-like; cần số tham số tuỳ ý → khai báo hoặc Function-like.
</details>

<details>
<summary><b>Bài tập 1 — Lời giải</b></summary>

| Bài toán | Loại macro | Vì sao |
|---|---|---|
| Sinh `fn default_init() -> Self` cho 20 struct | **Custom Derive** | Derive sinh ra một khối `impl` mới mà **không sửa** định nghĩa gốc. Đúng 20 lần viết `#[derive(...)]` là xong |
| `#[require_admin]` trước hàm xử lý | **Attribute-like** | Đây là *bọc*: macro nhận cả thân hàm, chèn phần kiểm quyền vào đầu, rồi trả lại hàm đã sửa. Derive **không làm được** vì nó chỉ thêm, không sửa |
| `make_list!(1, 2, 3)` | **`macro_rules!`** | Số tham số tuỳ ý, không cần phân tích cú pháp phức tạp. Function-like thủ tục cũng làm được nhưng thừa: nó kéo theo `syn` + `quote` + một crate riêng |

**Quy tắc rút ra:** Derive khi bạn *thêm* hành vi cho một kiểu; Attribute khi bạn *biến đổi* một khối mã có sẵn; `macro_rules!` khi bạn chỉ cần *thay thế mẫu* và cú pháp đơn giản.
</details>

<details>
<summary><b>Bài tập 2 — Gợi ý</b></summary>

Thuộc tính phụ chỉ có nghĩa khi đi kèm derive khai báo chúng. Hãy thiết kế sao cho **đọc lên hiểu ngay** và **quên là biên dịch báo lỗi**, chứ không âm thầm bỏ qua.
</details>

<details>
<summary><b>Bài tập 2 — Lời giải</b></summary>

```text
#[derive(Validation)]
struct SignUp {
    #[validate(length(min = 3, max = 32))]
    username: String,

    #[validate(length(min = 12), has_uppercase, has_digit, has_special_char)]
    password: String,

    #[validate(positive, max = 150)]
    age: u32,

    #[validate(format = "email")]
    email: String,

    // Không có #[validate(...)] -> KHÔNG kiểm gì. Im lặng là một lựa chọn,
    // nhưng phải là lựa chọn CÓ Ý THỨC.
    note: Option<String>,
}
```

Macro sinh ra `fn validate(&self) -> Result<(), Vec<ValidationError>>` — trả **mọi** lỗi cùng lúc chứ không dừng ở lỗi đầu, đúng như `applicative_collects_all_three_errors` ở Chương 19.

**Ba quyết định thiết kế đáng bàn:**

- **Gom dưới một tiền tố `validate`** thay vì rải `#[min_len]`, `#[positive]`. Tiền tố tránh đụng tên với derive khác, và cho người đọc biết ngay ai là chủ của thuộc tính đó.
- **Thuộc tính lạ phải báo lỗi, không được bỏ qua.** Gõ nhầm `#[validate(lenght(min=3))]` mà macro im lặng thì trường đó **không được kiểm** — một lỗ hổng bảo mật sinh ra từ một chữ cái thừa.
- **Không kiểm được kiểu lúc mở rộng.** `#[validate(positive)]` trên một `String` chỉ vỡ khi mã sinh ra được biên dịch. Thông báo lỗi khi đó trỏ vào *mã sinh ra*, khó đọc — nên macro tốt dùng `syn::Error::new_spanned` để **trỏ ngược về đúng dòng người dùng viết**.
</details>

<details>
<summary><b>Bài tập 3 — Gợi ý</b></summary>

Ba nhược điểm nằm ở ba chỗ khác nhau: lúc gõ mã, lúc biên dịch, và lúc gỡ lỗi.
</details>

<details>
<summary><b>Bài tập 3 — Lời giải</b></summary>

**1. Công cụ mất tác dụng.** Trong thân `macro_rules!`, IDE không gợi ý được, không nhảy tới định nghĩa được, không đổi tên hàng loạt được — với nó đó chỉ là các thẻ bài chưa có nghĩa. Với hàm thường, mọi công cụ đều hiểu. Với macro thủ tục còn tệ hơn: `rust-analyzer` phải *chạy* macro mới biết nó sinh ra gì.

**2. Thông báo lỗi trỏ nhầm chỗ.** Lỗi kiểu bên trong mã sinh ra thường hiện ở vị trí *lời gọi macro*, kèm một đoạn mã người dùng chưa từng viết. Một dòng `E0308` trong hàm thường mất mười giây để hiểu; cùng lỗi đó qua ba tầng macro có thể mất nửa buổi.

**3. Biên dịch chậm và không kiểm tra kiểu trước.** Macro chạy **trước** khi kiểm tra kiểu, nên nó không thể tận dụng thông tin kiểu; mọi phép kiểm phải tự viết bằng cách soi cú pháp. Macro thủ tục còn kéo theo `syn`/`quote` — thường là vài giây biên dịch cộng thêm cho mỗi lần build sạch.

**Còn một nhược điểm ít được nhắc:** macro **không tổ hợp được**. Hai hàm ghép lại thành hàm thứ ba. Hai macro thì không — bạn phải viết macro thứ ba biết về cả hai. Đó là lý do `Generics` gần như luôn là lựa chọn tốt hơn khi nó đủ dùng: hàm generic vừa được kiểm kiểu, vừa ghép được, vừa có công cụ hỗ trợ.
</details>
