#![allow(dead_code, unused_variables, unused_imports)]
use std::hint::black_box;

/// Hàm so sánh mảng byte theo kiểu thời gian bất biến (Constant-Time Comparison)
/// Không kết thúc sớm khi gặp byte sai, nên thời gian chạy không phụ thuộc VỊ TRÍ byte sai.
///
/// GIỚI HẠN QUAN TRỌNG: đây là mã minh hoạ. Rust/LLVM KHÔNG hứa sinh mã máy hằng thời gian
/// (`black_box` chỉ là gợi ý "đừng tối ưu", không phải bảo đảm). Mã thật hãy dùng crate
/// chuyên dụng như `subtle` (`ConstantTimeEq`), được viết và kiểm định cho đúng mục đích này.
pub fn constant_time_compare(a: &[u8], b: &[u8]) -> bool {
    // Độ dài khác nhau thì trả về ngay: điều này tiết lộ ĐỘ DÀI bí mật (thường chấp nhận
    // được vì độ dài token là công khai), nhưng không tiết lộ nội dung.
    if a.len() != b.len() {
        return false;
    }

    let mut difference_accumulator: u8 = 0;

    // Duyệt qua toàn bộ các phần tử mà không dùng lệnh 'break' hay 'return' sớm
    for (byte_a, byte_b) in a.iter().zip(b.iter()) {
        // Phép XOR: Nếu hai byte giống nhau thì kết quả bằng 0, khác nhau thì khác 0
        difference_accumulator |= byte_a ^ byte_b;
    }

    // Gợi ý trình biên dịch đừng "thông minh" biến vòng lặp thành so sánh thoát sớm
    // (chỉ là nỗ lực tốt nhất — xem giới hạn ở chú thích đầu hàm)
    black_box(difference_accumulator) == 0
}

/// Các mức phân quyền người dùng trong mô hình bảo mật
#[derive(Debug, PartialEq, Eq, PartialOrd, Ord, Clone, Copy)]
pub enum UserRole {
    Guest = 0,
    Member = 1,
    Auditor = 2,
    Administrator = 3,
}

/// Động cơ xác thực và lọc mối đe dọa an ninh theo mô hình STRIDE
pub struct SecurityGateEngine {
    secret_master_token: Vec<u8>,
}

impl SecurityGateEngine {
    pub fn new(master_token: &[u8]) -> Self {
        Self {
            secret_master_token: master_token.to_vec(),
        }
    }

    /// Làm sạch dữ liệu đầu vào (Input Sanitization) theo nguyên tắc Whitelist
    /// Ngăn chặn Tampering và Injection
    pub fn sanitize_command_input(&self, raw_input: &str) -> Result<String, &'static str> {
        if raw_input.is_empty() {
            return Err("Đầu vào trống: Từ chối xử lý!");
        }

        if raw_input.len() > 64 {
            return Err("Đầu vào quá dài: Nguy cơ DoS bị chặn đứng!");
        }

        // Nguyên tắc Whitelist: Chỉ cho phép chữ cái/chữ số ASCII, gạch dưới, gạch ngang
        // và khoảng trắng. Dùng is_ascii_alphanumeric (KHÔNG phải is_alphanumeric):
        // danh sách trắng phải hẹp và rõ ràng — is_alphanumeric chấp nhận hàng chục
        // nghìn ký tự Unicode, gồm cả các ký tự trông giống hệt chữ Latinh.
        let is_safe = raw_input
            .chars()
            .all(|c| c.is_ascii_alphanumeric() || c == '_' || c == '-' || c == ' ');

        if !is_safe {
            return Err("Phát hiện ký tự nguy hiểm (đã chặn SQL/Shell Injection)!");
        }

        Ok(raw_input.trim().to_string())
    }

    /// Xác thực khóa bí mật với cơ chế chống Timing Attack
    pub fn authenticate_token(&self, provided_token: &[u8]) -> bool {
        constant_time_compare(&self.secret_master_token, provided_token)
    }

    /// Kiểm tra phân quyền truy cập theo nguyên tắc quyền tối thiểu (Least Privilege)
    pub fn verify_permission(
        &self,
        current_role: UserRole,
        required_role: UserRole,
    ) -> Result<(), &'static str> {
        if current_role >= required_role {
            Ok(())
        } else {
            Err("Từ chối truy cập: Không đủ đặc quyền (đã chặn Elevation of Privilege)!")
        }
    }
}

fn main() {
    println!("==================================================================");
    println!("   GIA CỐ HỆ THỐNG RUST & MÔ HÌNH HÓA MỐI ĐE DỌA STRIDE / OSCP    ");
    println!("==================================================================");

    // Khởi tạo động cơ an ninh với Master Token bí mật 16 bytes
    let master_token = b"OSCP_RUST_KEY_99";
    let security_gate = SecurityGateEngine::new(master_token);

    // -------------------------------------------------------------
    // 1. THỬ NGHIỆM CHỐNG TẤN CÔNG TIMING ATTACK QUA CONSTANT-TIME
    // -------------------------------------------------------------
    println!("\n[1] Kiểm chứng so sánh thời gian bất biến (Constant-Time):");
    let valid_attempt = b"OSCP_RUST_KEY_99";
    let wrong_first_byte = b"XSCP_RUST_KEY_99";
    let wrong_last_byte = b"OSCP_RUST_KEY_00";

    println!(
        "    - Thử token hợp lệ      : {}",
        security_gate.authenticate_token(valid_attempt)
    );
    println!(
        "    - Thử token sai byte đầu : {}",
        security_gate.authenticate_token(wrong_first_byte)
    );
    println!(
        "    - Thử token sai byte cuối: {}",
        security_gate.authenticate_token(wrong_last_byte)
    );
    println!("    => Mọi phép so sánh đều duyệt hết mảng byte, bất kể byte sai nằm ở đâu!");

    // -------------------------------------------------------------
    // 2. THỬ NGHIỆM LÀM SẠCH ĐẦU VÀO CHỐNG INJECTION & BUFFER FLOOD
    // -------------------------------------------------------------
    println!("\n[2] Kiểm thử làm sạch dữ liệu đầu vào (Input Sanitization):");

    let safe_input = "get_system_status";
    match security_gate.sanitize_command_input(safe_input) {
        Ok(clean) => println!("    - Lệnh an toàn được chấp nhận: '{}'", clean),
        Err(err) => println!("    [!] Từ chối: {}", err),
    }

    let malicious_injection = "get_status; rm -rf /; --";
    println!("    - Thử gửi payload độc hại: '{}'", malicious_injection);
    match security_gate.sanitize_command_input(malicious_injection) {
        Ok(_) => println!("    [!] [CẢNH BÁO] Lệnh độc hại đã lọt qua!"),
        Err(err) => println!("    [+] [CHẶN ĐỨNG AN TOÀN] {}", err),
    }

    let overflow_dos_attempt = "A".repeat(128);
    println!(
        "    - Thử gửi chuỗi tấn công DoS dài {} bytes...",
        overflow_dos_attempt.len()
    );
    match security_gate.sanitize_command_input(&overflow_dos_attempt) {
        Ok(_) => println!("    [!] [CẢNH BÁO] Payload DoS đã được chấp nhận!"),
        Err(err) => println!("    [+] [CHẶN ĐỨNG AN TOÀN] {}", err),
    }

    // -------------------------------------------------------------
    // 3. THỬ NGHIỆM KIỂM SOÁT PHÂN QUYỀN TỐI THIỂU (LEAST PRIVILEGE)
    // -------------------------------------------------------------
    println!("\n[3] Kiểm tra kiểm soát phân quyền truy cập (RBAC):");
    let user_role = UserRole::Member;
    println!("    - Người dùng đang có vai trò: {:?}", user_role);

    let audit_access = security_gate.verify_permission(user_role, UserRole::Auditor);
    println!("    - Yêu cầu truy cập vùng Auditor: {:?}", audit_access);
    assert!(audit_access.is_err());

    let member_access = security_gate.verify_permission(user_role, UserRole::Member);
    println!("    - Yêu cầu truy cập vùng Member : {:?}", member_access);
    assert!(member_access.is_ok());
    println!("    => Ngăn chặn triệt để nguy cơ Leo thang đặc quyền (Elevation of Privilege)!");

    println!("\n==================================================================");
    println!("   XÁC NHẬN: HỆ THỐNG PHÒNG THỦ CHIỀU SÂU SẴN SÀNG HOẠT ĐỘNG!    ");
    println!("==================================================================");
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn constant_time_compare_is_correct() {
        assert!(constant_time_compare(b"abc", b"abc"));
        assert!(!constant_time_compare(b"abc", b"abd"));
        assert!(!constant_time_compare(b"abc", b"xbc"));
        assert!(!constant_time_compare(b"abc", b"abcd"));
        assert!(constant_time_compare(b"", b""));
    }

    #[test]
    fn whitelist_is_ascii_only() {
        let gate = SecurityGateEngine::new(b"k");
        assert!(gate.sanitize_command_input("get_status-2 now").is_ok());
        assert!(gate.sanitize_command_input("rm -rf /").is_err());
        // Chữ 'а' Kirin (U+0430) trông giống hệt 'a' Latinh: is_alphanumeric chấp nhận nó,
        // danh sách trắng ASCII thì không.
        assert!(gate.sanitize_command_input("\u{0430}dmin").is_err());
        assert!(gate.sanitize_command_input("").is_err());
        assert!(gate.sanitize_command_input(&"A".repeat(65)).is_err());
    }

    #[test]
    fn least_privilege_ordering() {
        let gate = SecurityGateEngine::new(b"k");
        assert!(
            gate.verify_permission(UserRole::Administrator, UserRole::Auditor)
                .is_ok()
        );
        assert!(
            gate.verify_permission(UserRole::Guest, UserRole::Member)
                .is_err()
        );
    }
}
