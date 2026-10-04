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
