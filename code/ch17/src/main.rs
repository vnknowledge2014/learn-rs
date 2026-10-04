// Tệp: src/main.rs
// Chương trình thực chiến làm chủ Higher-Order Functions & Functional Patterns trong Rust

use std::time::Instant;

// ============================================================================
// ĐỊNH NGHĨA DỮ LIỆU ĐẦU VÀO VÀ ĐẦU RA
// ============================================================================

#[derive(Debug, Clone, PartialEq)]
pub struct RawProfile {
    pub username: Option<String>,
    pub email: Option<String>,
    pub age_text: Option<String>,
}

#[derive(Debug, Clone, PartialEq)]
pub struct ValidProfile {
    pub username: String,
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
pub fn measure_exec_time<F, T>(task_name: &str, action: F) -> T
where
    F: FnOnce() -> T,
{
    println!(">>> [KIỂM TOÁN] Bắt đầu thực thi: {}", task_name);
    let start = Instant::now();

    // Gọi hàm/closure được truyền vào
    let result = action();

    let elapsed = start.elapsed();
    println!(
        ">>> [KIỂM TOÁN] Hoàn thành '{}' trong: {:?}",
        task_name, elapsed
    );
    result
}

// ============================================================================
// 2. HÀM XƯỞNG SẢN XUẤT CLOSURE (FACTORY PATTERN)
// ============================================================================

/// Tạo ra một closure kiểm tra xem một chuỗi có chứa từ cấm hay không
/// Sử dụng `move` để đóng gói danh sách từ cấm vào struct vô danh của closure
pub fn make_banned_word_filter(banned_words: Vec<&'static str>) -> impl Fn(&str) -> bool {
    move |text: &str| {
        let lowercase = text.to_lowercase();
        // Trả về true nếu KHÔNG chứa bất kỳ từ cấm nào
        !banned_words.iter().any(|&word| lowercase.contains(word))
    }
}

/// Tạo ra một closure kiểm tra độ dài tối thiểu và tối đa của chuỗi
pub fn make_length_checker(min: usize, max: usize) -> impl Fn(&str) -> bool {
    move |text: &str| {
        let length = text.trim().chars().count();
        length >= min && length <= max
    }
}

// ============================================================================
// 3. ĐƯỜNG ỐNG XÁC THỰC BẰNG BỘ KẾT HỢP COMBINATORS (PIPELINE PATTERN)
// ============================================================================

pub fn validate_profile(
    profile: &RawProfile,
    check_name: &impl Fn(&str) -> bool,
    check_banned_words: &impl Fn(&str) -> bool,
) -> Result<ValidProfile, &'static str> {
    // 1. Xác thực và chuẩn hóa Tên đăng nhập bằng chuỗi combinators
    let valid_name = profile
        .username
        .as_deref() // Option<String> -> Option<&str>
        .map(|s| s.trim()) // Cắt khoảng trắng
        .filter(|s| check_name(s)) // Kiểm tra độ dài hợp lệ
        .filter(|s| check_banned_words(s)) // Kiểm tra từ cấm
        .map(|s| s.to_string())
        .ok_or("Tên đăng nhập không hợp lệ hoặc chứa từ cấm!")?; // Lan truyền lỗi phẳng phiu

    // 2. Xác thực và chuẩn hóa Email
    let valid_email = profile
        .email
        .as_deref()
        .map(|s| s.trim())
        .filter(|s| s.contains('@') && s.contains('.')) // Điều kiện email cơ bản
        .map(|s| s.to_lowercase()) // Viết thường toàn bộ email
        .ok_or("Địa chỉ Email sai định dạng!")?;

    // 3. Xác thực và chuẩn hóa Tuổi
    let valid_age = profile
        .age_text
        .as_deref()
        .map(|s| s.trim())
        .and_then(|s| s.parse::<u32>().ok()) // Phân tích chuỗi sang u32
        .filter(|&age| (16..=100).contains(&age)) // Giới hạn độ tuổi từ 16 đến 100
        .ok_or("Độ tuổi phải là số nguyên từ 16 đến 100!")?;

    // Trả về cấu trúc hồ sơ đã được tinh chế sạch sẽ
    Ok(ValidProfile {
        username: valid_name,
        email: valid_email,
        age: valid_age,
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
    let check_name_length = make_length_checker(4, 15);
    let check_banned_words = make_banned_word_filter(vec!["admin", "root", "lua_dao"]);

    // Dữ liệu mẫu 1: Hồ sơ chuẩn mực hoàn hảo
    let raw_good = RawProfile {
        username: Some(String::from("  nguyen_an  ")),
        email: Some(String::from("An.Nguyen@EXAMPLE.COM  ")),
        age_text: Some(String::from("  22  ")),
    };

    // Dữ liệu mẫu 2: Hồ sơ lỗi chứa từ cấm và email hỏng
    let raw_bad = RawProfile {
        username: Some(String::from("super_admin")), // Chứa từ cấm 'admin'
        email: Some(String::from("email_khong_hop_le")),
        age_text: Some(String::from("12")), // Dưới 16 tuổi
    };

    // 1. Kiểm tra hồ sơ chuẩn với hàm bậc cao đo thời gian
    println!("\n--- TIẾN HÀNH XỬ LÝ HỒ SƠ THỨ NHẤT ---");
    let result_1 = measure_exec_time("Xử lý Hồ sơ Hợp lệ", || {
        validate_profile(&raw_good, &check_name_length, &check_banned_words)
    });

    match result_1 {
        Ok(profile) => {
            println!("[THÀNH CÔNG] Dữ liệu sau khi làm sạch:");
            println!("  - Tên đăng nhập: {}", profile.username);
            println!("  - Email hợp chuẩn: {}", profile.email);
            println!("  - Tuổi: {}", profile.age);
        }
        Err(error) => println!("[THẤT BẠI] Lỗi: {}", error),
    }

    // 2. Kiểm tra hồ sơ lỗi
    println!("\n--- TIẾN HÀNH XỬ LÝ HỒ SƠ THỨ HAI (CÓ LỖI) ---");
    let result_2 = measure_exec_time("Xử lý Hồ sơ Vi phạm", || {
        validate_profile(&raw_bad, &check_name_length, &check_banned_words)
    });

    match result_2 {
        Ok(_) => println!("[LỖI KHÔNG MONG MUỐN] Hồ sơ vi phạm lại lọt qua!"),
        Err(reason) => println!("[CHẶN THÀNH CÔNG] Hệ thống từ chối vì: '{}'", reason),
    }

    println!("\n============================================================");
    println!("     XÂY DỰNG PIPELINE HÀM BẬC CAO HOÀN THÀNH XUẤT SẮC      ");
    println!("============================================================");
}

// ============================================================================
// KIỂM THỬ
// ============================================================================

#[cfg(test)]
mod tests {
    use super::*;

    fn raw(name: &str, email: &str, age: &str) -> RawProfile {
        RawProfile {
            username: Some(name.to_string()),
            email: Some(email.to_string()),
            age_text: Some(age.to_string()),
        }
    }

    #[test]
    fn valid_profile_is_normalized() {
        let check_name = make_length_checker(4, 15);
        let check_words = make_banned_word_filter(vec!["admin"]);
        let profile = validate_profile(
            &raw("  nguyen_an ", "A@B.COM ", " 22 "),
            &check_name,
            &check_words,
        );
        assert_eq!(
            profile,
            Ok(ValidProfile {
                username: "nguyen_an".to_string(),
                email: "a@b.com".to_string(),
                age: 22,
            })
        );
    }

    #[test]
    fn first_failing_step_short_circuits() {
        let check_name = make_length_checker(4, 15);
        let check_words = make_banned_word_filter(vec!["admin"]);
        // Tên chứa từ cấm -> dừng ngay ở bước 1, dù email và tuổi cũng hỏng
        let result = validate_profile(&raw("super_admin", "sai", "12"), &check_name, &check_words);
        assert_eq!(result, Err("Tên đăng nhập không hợp lệ hoặc chứa từ cấm!"));
        // Tên ổn, tuổi ngoài khoảng 16..=100
        let result = validate_profile(&raw("binh", "b@c.vn", "12"), &check_name, &check_words);
        assert_eq!(result, Err("Độ tuổi phải là số nguyên từ 16 đến 100!"));
    }
}
