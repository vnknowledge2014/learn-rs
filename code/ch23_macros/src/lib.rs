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
