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
