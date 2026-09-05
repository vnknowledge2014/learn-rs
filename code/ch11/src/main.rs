#![allow(dead_code, unused_variables, unused_imports)]
// File: src/main.rs
// Chương trình thực chiến làm chủ Kỹ thuật Xử lý Lỗi Chuyên Nghiệp trong Rust

// 1. Tự định nghĩa kiểu Lỗi Nghiệp Vụ Tùy Biến (Custom Error Type) bằng Enum
#[derive(Debug)]
enum LoiThanhToan {
    SoTienKhongHopLe(String),
    TaiKhoanBiKhoa,
    SoDuKhongDu { so_du: f64, can_rut: f64 },
}

// Cài đặt khả năng in ấn đẹp mắt cho kiểu lỗi của chúng ta
impl std::fmt::Display for LoiThanhToan {
    fn fmt(&self, f: &mut std::fmt::Formatter) -> std::fmt::Result {
        match self {
            LoiThanhToan::SoTienKhongHopLe(msg) => write!(f, "Số tiền không hợp lệ: {}", msg),
            LoiThanhToan::TaiKhoanBiKhoa => write!(f, "Tài khoản đang bị khóa do vi phạm an ninh!"),
            LoiThanhToan::SoDuKhongDu { so_du, can_rut } => {
                write!(f, "Số dư không đủ (Hiện có: {:.2}, Yêu cầu rút: {:.2})", so_du, can_rut)
            }
        }
    }
}

// 2. Hàm kiểm tra tính hợp lệ của số tiền nhập vào
fn kiem_tra_so_tien(chuoi_nhap: &str) -> Result<f64, LoiThanhToan> {
    let so_tien: f64 = chuoi_nhap.trim().parse().map_err(|_| {
        LoiThanhToan::SoTienKhongHopLe(String::from("Vui lòng chỉ nhập các chữ số hợp lệ!"))
    })?;

    if so_tien <= 0.0 {
        return Err(LoiThanhToan::SoTienKhongHopLe(String::from("Số tiền phải lớn hơn 0!")));
    }

    Ok(so_tien)
}

// 3. Hàm thực hiện giao dịch: Tận dụng toán tử '?' để lan truyền lỗi siêu gọn
fn thuc_hien_giao_dich(
    chuoi_nhap: &str, 
    mut so_du_hien_tai: f64, 
    tai_khoan_hoat_dong: bool
) -> Result<f64, LoiThanhToan> {
    // Bước 1: Kiểm tra trạng thái tài khoản
    if !tai_khoan_hoat_dong {
        return Err(LoiThanhToan::TaiKhoanBiKhoa);
    }

    // Bước 2: Phân tích số tiền bằng toán tử '?'
    // Nếu kiem_tra_so_tien trả về Err, hàm lập tức return Err ngay tại dòng này!
    let so_tien_can_rut = kiem_tra_so_tien(chuoi_nhap)?;

    // Bước 3: Kiểm tra hạn mức số dư
    if so_tien_can_rut > so_du_hien_tai {
        return Err(LoiThanhToan::SoDuKhongDu {
            so_du: so_du_hien_tai,
            can_rut: so_tien_can_rut,
        });
    }

    // Bước 4: Trừ tiền thành công
    so_du_hien_tai -= so_tien_can_rut;
    Ok(so_du_hien_tai) // Trả về số dư mới bọc trong Ok
}

fn main() {
    println!("============================================================");
    println!("     CỔNG THANH TOÁN TÀI CHÍNH AN TOÀN - RUST BANKING       ");
    println!("============================================================");

    let so_du_ban_dau = 5_000_000.0;

    // --- KỊCH BẢN 1: GIAO DỊCH THÀNH CÔNG HỢP LỆ ---
    println!("\n[Kịch bản 1] Rút 1.500.000 VND hợp lệ:");
    match thuc_hien_giao_dich("1500000", so_du_ban_dau, true) {
        Ok(so_du_moi) => println!("-> Giao dịch THÀNH CÔNG! Số dư còn lại: {:.2} VND", so_du_moi),
        Err(e) => println!("-> Giao dịch THẤT BẠI: {}", e),
    }

    // --- KỊCH BẢN 2: LỖI NHẬP LIỆU KHÔNG PHẢI CHỮ SỐ ---
    println!("\n[Kịch bản 2] Người dùng nhập chữ linh tinh:");
    match thuc_hien_giao_dich("mot_trieu", so_du_ban_dau, true) {
        Ok(so_du_moi) => println!("-> Thành công: {:.2} VND", so_du_moi),
        Err(e) => println!("-> Hệ thống xử lý êm dịu: [{}]", e),
    }

    // --- KỊCH BẢN 3: LỖI SỐ DƯ KHÔNG ĐỦ ĐỂ RÚT ---
    println!("\n[Kịch bản 3] Rút số tiền vượt hạn mức số dư:");
    match thuc_hien_giao_dich("10000000", so_du_ban_dau, true) {
        Ok(so_du_moi) => println!("-> Thành công: {:.2} VND", so_du_moi),
        Err(e) => println!("-> Báo cáo lỗi chính xác: [{}]", e),
    }

    // --- KỊCH BẢN 4: LỖI TÀI KHOẢN BỊ KHÓA AN NINH ---
    println!("\n[Kịch bản 4] Tài khoản bị phong tỏa:");
    match thuc_hien_giao_dich("500000", so_du_ban_dau, false) {
        Ok(so_du_moi) => println!("-> Thành công: {:.2} VND", so_du_moi),
        Err(e) => println!("-> Từ chối truy cập: [{}]", e),
    }

    // --- KỊCH BẢN 5: CÁC PHƯƠNG THỨC XỬ LÝ DỰ PHÒNG AN TOÀN ---
    println!("\n[Kịch bản 5] Sử dụng unwrap_or để lấy giá trị mặc định an toàn:");
    let ket_qua_loi: Result<f64, &str> = Err("Mất kết nối máy chủ");
    let so_tien_cuoi_cung = ket_qua_loi.unwrap_or(0.0);
    println!("- Giá trị an toàn thu được: {:.2} VND (không hề bị sập ứng dụng!)", so_tien_cuoi_cung);
}
