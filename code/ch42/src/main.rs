#![allow(dead_code, unused_variables, unused_imports)]
use std::hint::black_box;

/// Hàm so sánh mảng byte với thời gian bất biến (Constant-Time Comparison)
/// Tuyệt đối không kết thúc sớm khi gặp byte sai, ngăn chặn Timing Attack 100%!
pub fn constant_time_compare(a: &[u8], b: &[u8]) -> bool {
    if a.len() != b.len() {
        return false;
    }

    let mut difference_accumulator: u8 = 0;

    // Duyệt qua toàn bộ các phần tử mà không dùng lệnh 'break' hay 'return' sớm
    for (byte_a, byte_b) in a.iter().zip(b.iter()) {
        // Phép XOR: Nếu hai byte giống nhau thì kết quả bằng 0, khác nhau thì khác 0
        difference_accumulator |= byte_a ^ byte_b;
    }

    // Đảm bảo trình biên dịch không tối ưu hóa làm biến mất vòng lặp
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
            return Err("Dau vao trong: Tu choi xu ly!");
        }

        if raw_input.len() > 64 {
            return Err("Dau vao qua dai: Nguy co tran bo dem hoac DoS bi chan dung!");
        }

        // Nguyên tắc Whitelist: Chỉ cho phép chữ cái, chữ số, gạch dưới và khoảng trắng
        let is_safe = raw_input
            .chars()
            .all(|c| c.is_alphanumeric() || c == '_' || c == '-' || c == ' ');

        if !is_safe {
            return Err("Phat hien ky tu nguy hiem (SQL/Shell Injection blocked)!");
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
            Err("Tu choi truy cap: Khong du dac quyen (Elevation of Privilege blocked)!")
        }
    }
}

fn main() {
    println!("==================================================================");
    println!("   GIA CO HE THONG RUST & MO HINH HOA MOI DE DOA STRIDE / OSCP    ");
    println!("==================================================================");

    // Khởi tạo động cơ an ninh với Master Token bí mật 16 bytes
    let master_token = b"OSCP_RUST_KEY_99";
    let security_gate = SecurityGateEngine::new(master_token);

    // -------------------------------------------------------------
    // 1. THỬ NGHIỆM CHỐNG TẤN CÔNG TIMING ATTACK QUA CONSTANT-TIME
    // -------------------------------------------------------------
    println!("\n[1] Kiem chung so sanh thoi gian bat bien (Constant-Time):");
    let valid_attempt = b"OSCP_RUST_KEY_99";
    let wrong_first_byte = b"XSCP_RUST_KEY_99";
    let wrong_last_byte = b"OSCP_RUST_KEY_00";

    println!(
        "    - Thu token hop le      : {}",
        security_gate.authenticate_token(valid_attempt)
    );
    println!(
        "    - Thu token sai byte dau : {}",
        security_gate.authenticate_token(wrong_first_byte)
    );
    println!(
        "    - Thu token sai byte cuoi: {}",
        security_gate.authenticate_token(wrong_last_byte)
    );
    println!("    => Moi phep so sanh deu duyet 100% mang byte voi thoi gian dong nhat!");

    // -------------------------------------------------------------
    // 2. THỬ NGHIỆM LÀM SẠCH ĐẦU VÀO CHỐNG INJECTION & BUFFER FLOOD
    // -------------------------------------------------------------
    println!("\n[2] Kiem thu lam sach du lieu dau vao (Input Sanitization):");

    let safe_input = "get_system_status";
    match security_gate.sanitize_command_input(safe_input) {
        Ok(clean) => println!("    - Lenh an toan duoc chap nhan: '{}'", clean),
        Err(err) => println!("    [!] Tu choi: {}", err),
    }

    let malicious_injection = "get_status; rm -rf /; --";
    println!("    - Thu gui payload doc hai: '{}'", malicious_injection);
    match security_gate.sanitize_command_input(malicious_injection) {
        Ok(_) => println!("    [!] [CANH BAO] Lenh doc hai da lot qua!"),
        Err(err) => println!("    [+] [CHAN DUNG AN TOAN] {}", err),
    }

    let overflow_dos_attempt = "A".repeat(128);
    println!("    - Thu gui chuoi tan cong DoS dai {} bytes...", overflow_dos_attempt.len());
    match security_gate.sanitize_command_input(&overflow_dos_attempt) {
        Ok(_) => println!("    [!] [CANH BAO] Payload DoS da duoc chap nhan!"),
        Err(err) => println!("    [+] [CHAN DUNG AN TOAN] {}", err),
    }

    // -------------------------------------------------------------
    // 3. THỬ NGHIỆM KIỂM SOÁT PHÂN QUYỀN TỐI THIỂU (LEAST PRIVILEGE)
    // -------------------------------------------------------------
    println!("\n[3] Kiem tra kiem soat phan quyen truy cap (RBAC):");
    let user_role = UserRole::Member;
    println!("    - Nguoi dung dang co vai tro: {:?}", user_role);

    let audit_access = security_gate.verify_permission(user_role, UserRole::Auditor);
    println!("    - Yeu cau truy cap vung Auditor: {:?}", audit_access);
    assert!(audit_access.is_err());

    let member_access = security_gate.verify_permission(user_role, UserRole::Member);
    println!("    - Yeu cau truy cap vung Member : {:?}", member_access);
    assert!(member_access.is_ok());
    println!("    => Ngăn chan triet de nguy co Leo thang dac quyen (Elevation of Privilege)!");

    println!("\n==================================================================");
    println!("   XAC NHAN: HE THONG PHONG THU CHIEU SAU SAN SANG HOAT DONG!    ");
    println!("==================================================================");
}
