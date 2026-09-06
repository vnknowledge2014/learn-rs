#![allow(dead_code, unused_variables, unused_imports)]
// Tệp: src/main.rs
// Chương trình thực chiến làm chủ Higher-Order Functions & Functional Patterns trong Rust

use std::time::Instant;

// ============================================================================
// ĐỊNH NGHĨA DỮ LIỆU ĐẦU VÀO VÀ ĐẦU RA
// ============================================================================

#[derive(Debug, Clone, PartialEq)]
pub struct RawProfile {
    pub name_dang_import: Option<String>,
    pub email: Option<String>,
    pub age_series: Option<String>,
}

#[derive(Debug, Clone, PartialEq)]
pub struct ValidProxy {
    pub name_dang_import: String,
    pub email: String,
    pub age: u32,
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
pub fn measure_exec_time<F, T>(ten_tac_vu: &str, hanh_dong: F) -> T
where
    F: FnOnce() -> T,
{
    println!(">>> [KIỂM TOÁN] Bắt đầu thực thi: {}", ten_tac_vu);
    let timestamp_start = Instant::now();
    
    // Gọi hàm/closure được truyền vào
    let ket_qua = hanh_dong();
    
    let range_time_time = timestamp_start.elapsed();
    println!(">>> [KIỂM TOÁN] Hoàn thành '{}' trong: {:?}", ten_tac_vu, range_time_time);
    ket_qua
}

// ============================================================================
// 2. HÀM XƯỞNG SẢN XUẤT CLOSURE (FACTORY PATTERN)
// ============================================================================

/// Tạo ra một closure kiểm tra xem một chuỗi có chứa từ cấm hay không
/// Sử dụng `move` để đóng gói danh sách từ cấm vào struct vô danh của closure
pub fn make_ban_filter(danh_sach_tu_cam: Vec<&'static str>) -> impl Fn(&str) -> bool {
    move |van_ban: &str| {
        let lowercase = van_ban.to_lowercase();
        // Trả về true nếu KHÔNG chứa bất kỳ từ cấm nào
        !danh_sach_tu_cam.iter().any(|&tu| lowercase.contains(tu))
    }
}

/// Tạo ra một closure kiểm tra độ dài tối thiểu và tối đa của chuỗi
pub fn make_unit_check_do_long(min: usize, max: usize) -> impl Fn(&str) -> bool {
    move |van_ban: &str| {
        let length = van_ban.trim().chars().count();
        length >= min && length <= max
    }
}

// ============================================================================
// 3. ĐƯỜNG ỐNG XÁC THỰC BẰNG BỘ KẾT HỢP COMBINATORS (PIPELINE PATTERN)
// ============================================================================

pub fn auth_proxy_num(
    profile: &RawProfile,
    check_name: &impl Fn(&str) -> bool,
    check_banned_words: &impl Fn(&str) -> bool,
) -> Result<ValidProxy, &'static str> {
    // 1. Xác thực và chuẩn hóa Tên đăng nhập bằng chuỗi combinators
    let name_hop_le = profile
        .name_dang_import
        .as_deref()                                   // Option<String> -> Option<&str>
        .map(|s| s.trim())                            // Cắt khoảng trắng
        .filter(|s| check_name(s))                  // Kiểm tra độ dài hợp lệ
        .filter(|s| check_banned_words(s))              // Kiểm tra từ cấm
        .map(|s| s.to_string())
        .ok_or("Tên đăng nhập không hợp lệ hoặc chứa từ cấm!")?; // Lan truyền lỗi phẳng phiu

    // 2. Xác thực và chuẩn hóa Email
    let email_hop_le = profile
        .email
        .as_deref()
        .map(|s| s.trim())
        .filter(|s| s.contains('@') && s.contains('.')) // Điều kiện email cơ bản
        .map(|s| s.to_lowercase())                      // Viết thường toàn bộ email
        .ok_or("Địa chỉ Email sai định dạng!")?;

    // 3. Xác thực và chuẩn hóa Tuổi
    let age_hop_le = profile
        .age_series
        .as_deref()
        .map(|s| s.trim())
        .and_then(|s| s.parse::<u32>().ok())           // Phân tích chuỗi sang u32
        .filter(|&age| (16..=100).contains(&age))    // Giới hạn độ tuổi từ 16 đến 100
        .ok_or("Độ tuổi phải là số nguyên từ 16 đến 100!")?;

    // Trả về cấu trúc hồ sơ đã được tinh chế sạch sẽ
    Ok(ValidProxy {
        name_dang_import: name_hop_le,
        email: email_hop_le,
        age: age_hop_le,
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
    let check_do_long_name = make_unit_check_do_long(4, 15);
    let check_banned_words = make_ban_filter(vec!["admin", "root", "lua_dao"]);

    // Dữ liệu mẫu 1: Hồ sơ chuẩn mực hoàn hảo
    let proxy_num_standard = RawProfile {
        name_dang_import: Some(String::from("  nguyen_an  ")),
        email: Some(String::from("An.Nguyen@EXAMPLE.COM  ")),
        age_series: Some(String::from("  22  ")),
    };

    // Dữ liệu mẫu 2: Hồ sơ lỗi chứa từ cấm và email hỏng
    let proxy_num_error = RawProfile {
        name_dang_import: Some(String::from("super_admin")), // Chứa từ cấm 'admin'
        email: Some(String::from("email_khong_hop_le")),
        age_series: Some(String::from("12")),             // Dưới 16 tuổi
    };

    // 1. Kiểm tra hồ sơ chuẩn với hàm bậc cao đo thời gian
    println!("\n--- TIẾN HÀNH XỬ LÝ HỒ SƠ THỨ NHẤT ---");
    let ket_qua_1 = measure_exec_time("Xử lý Hồ sơ Hợp lệ", || {
        auth_proxy_num(&proxy_num_standard, &check_do_long_name, &check_banned_words)
    });

    match ket_qua_1 {
        Ok(profile) => {
            println!("[THÀNH CÔNG] Dữ liệu sau khi làm sạch:");
            println!("  - Tên đăng nhập: {}", profile.name_dang_import);
            println!("  - Email hợp chuẩn: {}", profile.email);
            println!("  - Tuổi: {}", profile.age);
        }
        Err(error) => println!("[THẤT BẠI] Lỗi: {}", error),
    }

    // 2. Kiểm tra hồ sơ lỗi
    println!("\n--- TIẾN HÀNH XỬ LÝ HỒ SƠ THỨ HAI (CÓ LỖI) ---");
    let ket_qua_2 = measure_exec_time("Xử lý Hồ sơ Vi phạm", || {
        auth_proxy_num(&proxy_num_error, &check_do_long_name, &check_banned_words)
    });

    match ket_qua_2 {
        Ok(_) => println!("[LỖI KHÔNG MONG MUỐN] Hồ sơ vi phạm lại lọt qua!"),
        Err(ly_do) => println!("[CHẶN THÀNH CÔNG] Hệ thống từ chối vì: '{}'", ly_do),
    }

    println!("\n============================================================");
    println!("     XÂY DỰNG PIPELINE HÀM BẬC CAO HOÀN THÀNH XUẤT SẮC      ");
    println!("============================================================");
}
