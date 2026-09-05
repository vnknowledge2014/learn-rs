#![allow(dead_code, unused_variables, unused_imports)]
// Tệp: src/main.rs
// Chương trình thực chiến làm chủ Higher-Order Functions & Functional Patterns trong Rust

use std::time::Instant;

// ============================================================================
// ĐỊNH NGHĨA DỮ LIỆU ĐẦU VÀO VÀ ĐẦU RA
// ============================================================================

#[derive(Debug, Clone, PartialEq)]
pub struct HoSoTho {
    pub ten_dang_nhap: Option<String>,
    pub email: Option<String>,
    pub tuoi_chuoi: Option<String>,
}

#[derive(Debug, Clone, PartialEq)]
pub struct HoSoHopLe {
    pub ten_dang_nhap: String,
    pub email: String,
    pub tuoi: u32,
}

// ============================================================================
// 1. HÀM BẬC CAO: ĐO LƯỜNG THỜI GIAN VÀ GHI NHẬT KÝ KIỂM TOÁN (WRAPPER PATTERN)
// ============================================================================

/// Hàm bậc cao nhận vào tên tác vụ và một hành động F bất kỳ
/// Thực hiện đo thời gian thực thi của hành động đó và trả về kết quả nguyên bản
///
/// LƯU Ý VỀ TÍNH THUẦN TÚY: bản thân hàm này KHÔNG thuần túy — nó đọc đồng hồ
/// hệ thống (`Instant::now()`) và in ra màn hình, nên gọi hai lần cho hai kết quả
/// khác nhau. Đó là chủ ý: đo lường và ghi nhật ký là tác dụng phụ chính đáng,
/// nhưng chúng phải nằm ở TẦNG VỎ, bao bên ngoài phần lõi thuần túy.
/// Đây chính là kiến trúc "lõi thuần túy - vỏ mệnh lệnh" sẽ học kỹ ở Chương 20.
pub fn do_thoi_gian_thuc_thi<F, T>(ten_tac_vu: &str, hanh_dong: F) -> T
where
    F: FnOnce() -> T,
{
    println!(">>> [KIỂM TOÁN] Bắt đầu thực thi: {}", ten_tac_vu);
    let thoi_diem_bat_dau = Instant::now();
    
    // Gọi hàm/closure được truyền vào
    let ket_qua = hanh_dong();
    
    let khoang_thoi_gian = thoi_diem_bat_dau.elapsed();
    println!(">>> [KIỂM TOÁN] Hoàn thành '{}' trong: {:?}", ten_tac_vu, khoang_thoi_gian);
    ket_qua
}

// ============================================================================
// 2. HÀM XƯỞNG SẢN XUẤT CLOSURE (FACTORY PATTERN)
// ============================================================================

/// Tạo ra một closure kiểm tra xem một chuỗi có chứa từ cấm hay không
/// Sử dụng `move` để đóng gói danh sách từ cấm vào struct vô danh của closure
pub fn tao_bo_loc_tu_cam(danh_sach_tu_cam: Vec<&'static str>) -> impl Fn(&str) -> bool {
    move |van_ban: &str| {
        let chu_thuong = van_ban.to_lowercase();
        // Trả về true nếu KHÔNG chứa bất kỳ từ cấm nào
        !danh_sach_tu_cam.iter().any(|&tu| chu_thuong.contains(tu))
    }
}

/// Tạo ra một closure kiểm tra độ dài tối thiểu và tối đa của chuỗi
pub fn tao_bo_kiem_tra_do_dai(min: usize, max: usize) -> impl Fn(&str) -> bool {
    move |van_ban: &str| {
        let do_dai = van_ban.trim().chars().count();
        do_dai >= min && do_dai <= max
    }
}

// ============================================================================
// 3. ĐƯỜNG ỐNG XÁC THỰC BẰNG BỘ KẾT HỢP COMBINATORS (PIPELINE PATTERN)
// ============================================================================

pub fn xac_thuc_ho_so(
    ho_so: &HoSoTho,
    kiem_tra_ten: &impl Fn(&str) -> bool,
    kiem_tra_tu_cam: &impl Fn(&str) -> bool,
) -> Result<HoSoHopLe, &'static str> {
    // 1. Xác thực và chuẩn hóa Tên đăng nhập bằng chuỗi combinators
    let ten_hop_le = ho_so
        .ten_dang_nhap
        .as_deref()                                   // Option<String> -> Option<&str>
        .map(|s| s.trim())                            // Cắt khoảng trắng
        .filter(|s| kiem_tra_ten(s))                  // Kiểm tra độ dài hợp lệ
        .filter(|s| kiem_tra_tu_cam(s))              // Kiểm tra từ cấm
        .map(|s| s.to_string())
        .ok_or("Tên đăng nhập không hợp lệ hoặc chứa từ cấm!")?; // Lan truyền lỗi phẳng phiu

    // 2. Xác thực và chuẩn hóa Email
    let email_hop_le = ho_so
        .email
        .as_deref()
        .map(|s| s.trim())
        .filter(|s| s.contains('@') && s.contains('.')) // Điều kiện email cơ bản
        .map(|s| s.to_lowercase())                      // Viết thường toàn bộ email
        .ok_or("Địa chỉ Email sai định dạng!")?;

    // 3. Xác thực và chuẩn hóa Tuổi
    let tuoi_hop_le = ho_so
        .tuoi_chuoi
        .as_deref()
        .map(|s| s.trim())
        .and_then(|s| s.parse::<u32>().ok())           // Phân tích chuỗi sang u32
        .filter(|&tuoi| (16..=100).contains(&tuoi))    // Giới hạn độ tuổi từ 16 đến 100
        .ok_or("Độ tuổi phải là số nguyên từ 16 đến 100!")?;

    // Trả về cấu trúc hồ sơ đã được tinh chế sạch sẽ
    Ok(HoSoHopLe {
        ten_dang_nhap: ten_hop_le,
        email: email_hop_le,
        tuoi: tuoi_hop_le,
    })
}

// ============================================================================
// CHƯƠNG TRÌNH THỰC THI CHÍNH
// ============================================================================

fn main() {
    println!("============================================================");
    println!("   HỆ THỐNG XÁC THỰC HỒ SƠ: HÀM BẬC CAO & COMBINATORS FP    ");
    println!("============================================================");

    // Khởi tạo các cỗ máy kiểm tra từ xưởng Factory
    let kiem_tra_do_dai_ten = tao_bo_kiem_tra_do_dai(4, 15);
    let kiem_tra_tu_cam = tao_bo_loc_tu_cam(vec!["admin", "root", "lua_dao"]);

    // Dữ liệu mẫu 1: Hồ sơ chuẩn mực hoàn hảo
    let ho_so_chuan = HoSoTho {
        ten_dang_nhap: Some(String::from("  nguyen_an  ")),
        email: Some(String::from("An.Nguyen@EXAMPLE.COM  ")),
        tuoi_chuoi: Some(String::from("  22  ")),
    };

    // Dữ liệu mẫu 2: Hồ sơ lỗi chứa từ cấm và email hỏng
    let ho_so_loi = HoSoTho {
        ten_dang_nhap: Some(String::from("super_admin")), // Chứa từ cấm 'admin'
        email: Some(String::from("email_khong_hop_le")),
        tuoi_chuoi: Some(String::from("12")),             // Dưới 16 tuổi
    };

    // 1. Kiểm tra hồ sơ chuẩn với hàm bậc cao đo thời gian
    println!("\n--- TIẾN HÀNH XỬ LÝ HỒ SƠ THỨ NHẤT ---");
    let ket_qua_1 = do_thoi_gian_thuc_thi("Xử lý Hồ sơ Hợp lệ", || {
        xac_thuc_ho_so(&ho_so_chuan, &kiem_tra_do_dai_ten, &kiem_tra_tu_cam)
    });

    match ket_qua_1 {
        Ok(ho_so) => {
            println!("[THÀNH CÔNG] Dữ liệu sau khi làm sạch:");
            println!("  - Tên đăng nhập: {}", ho_so.ten_dang_nhap);
            println!("  - Email hợp chuẩn: {}", ho_so.email);
            println!("  - Tuổi: {}", ho_so.tuoi);
        }
        Err(loi) => println!("[THẤT BẠI] Lỗi: {}", loi),
    }

    // 2. Kiểm tra hồ sơ lỗi
    println!("\n--- TIẾN HÀNH XỬ LÝ HỒ SƠ THỨ HAI (CÓ LỖI) ---");
    let ket_qua_2 = do_thoi_gian_thuc_thi("Xử lý Hồ sơ Vi phạm", || {
        xac_thuc_ho_so(&ho_so_loi, &kiem_tra_do_dai_ten, &kiem_tra_tu_cam)
    });

    match ket_qua_2 {
        Ok(_) => println!("[LỖI KHÔNG MONG MUỐN] Hồ sơ vi phạm lại lọt qua!"),
        Err(ly_do) => println!("[CHẶN THÀNH CÔNG] Hệ thống từ chối vì: '{}'", ly_do),
    }

    println!("\n============================================================");
    println!("     XÂY DỰNG PIPELINE HÀM BẬC CAO HOÀN THÀNH XUẤT SẮC      ");
    println!("============================================================");
}
