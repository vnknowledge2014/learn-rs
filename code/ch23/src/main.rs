#![allow(dead_code, unused_variables, unused_imports)]
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
