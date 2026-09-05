#![allow(dead_code, unused_variables, unused_imports)]
// File: src/main.rs
// Ứng dụng thực chiến làm chủ Vòng đời (Lifetimes) trong Rust

// 1. Hàm so sánh hai chuỗi và trả về chuỗi dài hơn
// Ký hiệu <'a> tuyên bố: Chuỗi trả về có vòng đời an toàn bằng khoảng giao nhau giữa x và y
fn chon_thong_bao_dai_hon<'a>(x: &'a str, y: &'a str) -> &'a str {
    if x.len() > y.len() {
        x
    } else {
        y
    }
}

// 2. Struct nắm giữ tham chiếu mượn dữ liệu nguồn (&'a str)
// Giúp đọc và trích xuất cấu hình mà KHÔNG tốn dù chỉ 1 byte để sao chép chuỗi mới trên Heap!
struct CauHinhHeThong<'a> {
    ten_ung_dung: &'a str,
    phi_dich_vu: f64,
}

impl<'a> CauHinhHeThong<'a> {
    // Phương thức đọc: Tận dụng Quy tắc suy luận ngầm số 3 (Lifetime Elision)
    // Không cần viết 'a ở kiểu trả về vì Rust tự lấy vòng đời của &self!
    fn lay_ten(&self) -> &str {
        self.ten_ung_dung
    }

    fn in_thong_tin(&self) {
        println!("- Ứng dụng: '{}' | Phí duy trì: {:.2} USD/tháng", 
                 self.ten_ung_dung, self.phi_dich_vu);
    }
}

fn main() {
    println!("============================================================");
    println!("      BỘ PHÂN TÍCH CẤU HÌNH SIÊU TỐC - ZERO-COPY PARSER     ");
    println!("============================================================");

    // --- PHẦN 1: HÀM CÓ CHÚ THÍCH VÒNG ĐỜI 'a ---
    println!("\n1. So sánh hai thông điệp có vòng đời hợp lệ:");
    let thong_diep_1 = String::from("Hệ thống khởi động thành công");
    let thong_diep_2 = String::from("Cảnh báo pin yếu");

    // Cả thong_diep_1 và thong_diep_2 đều đang sống trong cùng phạm vi main
    let thong_diep_chinh = chon_thong_bao_dai_hon(
        thong_diep_1.as_str(), 
        thong_diep_2.as_str()
    );
    println!("- Thông điệp dài hơn được chọn: '{}'", thong_diep_chinh);

    // --- PHẦN 2: CHỨNG MINH TÍNH AN TOÀN TRƯỚC VÒNG ĐỜI NGẮN HƠN ---
    println!("\n2. Kiểm soát phạm vi sống lồng nhau an toàn:");
    let chuoi_me = String::from("Dữ liệu bền vững của công ty");
    {
        let chuoi_con = String::from("Dữ liệu tạm");
        let ket_qua_tam = chon_thong_bao_dai_hon(chuoi_me.as_str(), chuoi_con.as_str());
        println!("- [Bên trong phạm vi con]: Kết quả chọn là: '{}'", ket_qua_tam);
        // ket_qua_tam chỉ được phép dùng bên trong dấu ngoặc nhọn này!
        // Nếu cố tình mang ket_qua_tam ra ngoài phạm vi con, compiler sẽ chặn đứng ngay!
    }

    // --- PHẦN 3: STRUCT CHỨA THAM CHIẾU (ZERO-COPY) ---
    println!("\n3. Khởi tạo Struct chứa tham chiếu mượn không tốn RAM:");
    let tap_tin_cau_hinh = String::from("TenUngDung: RustCloudServer, Phi: 49.99");

    // Lát cắt trích xuất tên ứng dụng trực tiếp từ chuỗi nguồn:
    let ten_cat_duoc = &tap_tin_cau_hinh[12..27];

    let cau_hinh = CauHinhHeThong {
        ten_ung_dung: ten_cat_duoc,
        phi_dich_vu: 49.99,
    };

    cau_hinh.in_thong_tin();
    println!("- Tên ứng dụng trích xuất qua getter: '{}'", cau_hinh.lay_ten());

    // --- PHẦN 4: VÒNG ĐỜI VĨNH CỬU 'static ---
    println!("\n4. Sử dụng hằng số có vòng đời vĩnh cửu ('static):");
    let thong_diep_vinh_cuu: &'static str = "PHẦN MỀM ĐÃ ĐƯỢC CHỨNG NHẬN AN TOÀN TUYỆT ĐỐI";
    println!("- Dòng chữ trên bia đá vĩnh cửu: '{}'", thong_diep_vinh_cuu);
}
