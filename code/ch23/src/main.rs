#![allow(dead_code, unused_variables, unused_imports)]
// Tệp: src/main.rs
// Chương trình thực chiến làm chủ Kiến trúc Macro thủ tục (Procedural Macros), syn, quote và AST

// ============================================================================
// PHẦN 1: MÔ HÌNH HÓA ĐỊNH NGHĨA CÂY CÚ PHÁP TRỪU TƯỢNG (AST ANATOMY)
// Giúp người học thấu hiểu chính xác cấu trúc dữ liệu bên trong của crate `syn`
// ============================================================================

#[derive(Debug, Clone, PartialEq)]
pub struct AstDataField {
    pub field_name: &'static str,
    pub kind_data: &'static str,
}

#[derive(Debug, Clone, PartialEq)]
pub struct StructAST {
    pub ten_struct: &'static str,
    pub field_list: Vec<AstDataField>,
}

impl StructAST {
    /// Hàm mô phỏng công việc của syn: Duyệt cây AST và trích xuất danh sách tên trường
    pub fn get_list_name(&self) -> Vec<&'static str> {
        self.field_list
            .iter()
            .map(|f| f.field_name)
            .collect()
    }
}

// ============================================================================
// PHẦN 2: TRAIT VÀ MÃ ĐƯỢC TỰ ĐỘNG SINH RA BỞI QUOTE!
// ============================================================================

/// Trait giao ước mà Macro thủ tục sẽ tự động triển khai
pub trait DetailedDescription {
    fn in_thong_tin_chi_tiet(&self);
    fn field_count() -> usize;
}

// Giả sử lập trình viên viết Struct này:
pub struct NetworkDevice {
    pub ip_address: String,
    pub service_port: u16,
    pub dang_hoat_dong: bool,
}

// Đây là đoạn mã mà proc-macro (syn + quote) sẽ TỰ ĐỘNG SINH RA
// thay vì bắt lập trình viên phải tự tay gõ từng dòng:
impl DetailedDescription for NetworkDevice {
    fn in_thong_tin_chi_tiet(&self) {
        println!("------------------------------------------------------------");
        println!("THÔNG TIN THỰC THỂ: [NetworkDevice]");
        println!("  - Trường `ip_address`      : {}", self.ip_address);
        println!("  - Trường `service_port`    : {}", self.service_port);
        println!("  - Trường `dang_hoat_dong`  : {}", self.dang_hoat_dong);
        println!("------------------------------------------------------------");
    }

    fn field_count() -> usize {
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

#[proc_macro_derive(DetailedDescription)]
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
    let quantity = fields.len();

    // 4. Dùng quote! để sinh mã Rust mới
    let ma_sinh = quote! {
        impl DetailedDescription for #ten_struct {
            fn in_thong_tin_chi_tiet(&self) {
                println!("THÔNG TIN THỰC THỂ: [{}]", stringify!(#ten_struct));
                #(
                    println!("  - Trường `{}`: {:?}", stringify!(#ten_truongs), self.#ten_truongs);
                )*
            }

            fn field_count() -> usize {
                #quantity
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
        ten_struct: "NetworkDevice",
        field_list: vec![
            AstDataField { field_name: "ip_address", kind_data: "String" },
            AstDataField { field_name: "service_port", kind_data: "u16" },
            AstDataField { field_name: "dang_hoat_dong", kind_data: "bool" },
        ],
    };

    println!("\n1. Phân tích Cây cú pháp AST bằng `syn`:");
    println!("- Tên cấu trúc được phát hiện: {}", mo_hinh_ast.ten_struct);
    println!("- Danh sách các cành trường dữ liệu: {:?}", mo_hinh_ast.get_list_name());

    // 2. Kiểm chứng mã nguồn sau khi được `quote!` sinh ra tự động
    println!("\n2. Thực thi phương thức được dập khuôn tự động qua Trait DetailedDescription:");
    let router = NetworkDevice {
        ip_address: String::from("192.168.1.1"),
        service_port: 443,
        dang_hoat_dong: true,
    };

    // Gọi phương thức được sinh tự động bởi Proc Macro
    router.in_thong_tin_chi_tiet();
    println!("Tổng số lượng trường của thực thể: {}", NetworkDevice::field_count());

    println!("\n============================================================");
    println!("   XÁC MINH KIẾN TRÚC PROCEDURAL MACROS HOÀN TOÀN THÀNH CÔNG");
    println!("============================================================");
}
