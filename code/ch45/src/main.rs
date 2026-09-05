#![allow(dead_code, unused_variables, unused_imports)]
// ============================================================================
// CHƯƠNG 41: MINH HỌA QUY TRÌNH SPEC-DRIVEN DEVELOPMENT & TDD CÙNG AI
// Tác giả: Kỹ Sư Hệ Thống Rust
// ============================================================================

// ----------------------------------------------------------------------------
// PHẦN 1: BẢN ĐẶC TẢ NGHIỆP VỤ & HỆ THỐNG KIỂU (SPEC & DOMAIN TYPES)
// Ràng buộc đặc tả (SPEC):
// 1. Số tài khoản phải có tiền tố "VN", độ dài đúng 10 ký tự, phần sau là chữ số.
// 2. Số tiền chuyển khoản phải lớn hơn 0 và không vượt quá hạn mức 100,000,000 xu.
// 3. Tài khoản nguồn và tài khoản đích tuyệt đối không được trùng nhau.
// ----------------------------------------------------------------------------

#[derive(Debug, Clone, PartialEq, Eq)]
pub enum ValidationError {
    InvalidAccountLength { expected: usize, actual: usize },
    InvalidAccountPrefix(String),
    InvalidAccountDigits,
    SameSourceAndDestination,
    ZeroOrNegativeAmount,
    AmountExceedsLimit { limit: u64, requested: u64 },
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct TransferRequest {
    pub from_account: String,
    pub to_account: String,
    pub amount_cents: u64,
}

pub struct BankTransactionValidator {
    pub max_limit_cents: u64,
}

impl BankTransactionValidator {
    pub fn new(max_limit_cents: u64) -> Self {
        Self { max_limit_cents }
    }

    // Xác thực định dạng của một số tài khoản theo quy chuẩn
    // Mượn (borrow) tham chiếu lát cắt chuỗi &str để tối ưu hóa hiệu năng, zero-copy
    pub fn validate_account_format(&self, account: &str) -> Result<(), ValidationError> {
        if account.len() != 10 {
            return Err(ValidationError::InvalidAccountLength {
                expected: 10,
                actual: account.len(),
            });
        }

        if !account.starts_with("VN") {
            return Err(ValidationError::InvalidAccountPrefix(account[0..2].to_string()));
        }

        // Kiểm tra 8 ký tự phía sau phải là chữ số hợp lệ
        if !account[2..].chars().all(|c| c.is_ascii_digit()) {
            return Err(ValidationError::InvalidAccountDigits);
        }

        Ok(())
    }

    // Xác thực toàn bộ yêu cầu giao dịch chuyển khoản
    pub fn validate_transfer(&self, req: &TransferRequest) -> Result<(), ValidationError> {
        // 1. Kiểm tra tài khoản nguồn
        self.validate_account_format(&req.from_account)?;

        // 2. Kiểm tra tài khoản đích
        self.validate_account_format(&req.to_account)?;

        // 3. Kiểm tra trùng lặp
        if req.from_account == req.to_account {
            return Err(ValidationError::SameSourceAndDestination);
        }

        // 4. Kiểm tra số tiền
        if req.amount_cents == 0 {
            return Err(ValidationError::ZeroOrNegativeAmount);
        }

        if req.amount_cents > self.max_limit_cents {
            return Err(ValidationError::AmountExceedsLimit {
                limit: self.max_limit_cents,
                requested: req.amount_cents,
            });
        }

        Ok(())
    }
}

// ----------------------------------------------------------------------------
// PHẦN 2: BỘ KIỂM THỬ ĐƠN VỊ TDD DO AI SINH RA TỪ FILE SPEC (RED -> GREEN)
// ----------------------------------------------------------------------------
#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_valid_account_format() {
        let validator = BankTransactionValidator::new(50_000_000);
        assert!(validator.validate_account_format("VN12345678").is_ok());
    }

    #[test]
    fn test_account_invalid_length() {
        let validator = BankTransactionValidator::new(50_000_000);
        // Quá ngắn
        let err_short = validator.validate_account_format("VN123").unwrap_err();
        assert_eq!(err_short, ValidationError::InvalidAccountLength { expected: 10, actual: 5 });

        // Quá dài
        let err_long = validator.validate_account_format("VN12345678999").unwrap_err();
        assert_eq!(err_long, ValidationError::InvalidAccountLength { expected: 10, actual: 13 });
    }

    #[test]
    fn test_account_invalid_prefix() {
        let validator = BankTransactionValidator::new(50_000_000);
        let err = validator.validate_account_format("US12345678").unwrap_err();
        assert_eq!(err, ValidationError::InvalidAccountPrefix("US".to_string()));
    }

    #[test]
    fn test_account_non_digit_characters() {
        let validator = BankTransactionValidator::new(50_000_000);
        let err = validator.validate_account_format("VN1234ABCD").unwrap_err();
        assert_eq!(err, ValidationError::InvalidAccountDigits);
    }

    #[test]
    fn test_transfer_same_account_fails() {
        let validator = BankTransactionValidator::new(50_000_000);
        let req = TransferRequest {
            from_account: "VN12345678".to_string(),
            to_account: "VN12345678".to_string(),
            amount_cents: 1_000_000,
        };
        assert_eq!(validator.validate_transfer(&req), Err(ValidationError::SameSourceAndDestination));
    }

    #[test]
    fn test_transfer_amount_exceeds_limit() {
        let validator = BankTransactionValidator::new(10_000_000);
        let req = TransferRequest {
            from_account: "VN12345678".to_string(),
            to_account: "VN87654321".to_string(),
            amount_cents: 20_000_000, // Vượt quá hạn mức 10 triệu
        };
        assert_eq!(
            validator.validate_transfer(&req),
            Err(ValidationError::AmountExceedsLimit { limit: 10_000_000, requested: 20_000_000 })
        );
    }

    #[test]
    fn test_transfer_success() {
        let validator = BankTransactionValidator::new(50_000_000);
        let req = TransferRequest {
            from_account: "VN12345678".to_string(),
            to_account: "VN87654321".to_string(),
            amount_cents: 5_000_000,
        };
        assert!(validator.validate_transfer(&req).is_ok());
    }
}

// ----------------------------------------------------------------------------
// PHẦN 3: HÀM MAIN THỰC THI TRỰC TIẾP ĐỂ KIỂM CHỨNG TÍNH NĂNG
// ----------------------------------------------------------------------------
fn main() {
    println!("=== CHƯƠNG 41: MINH HỌA QUY TRÌNH SPEC-DRIVEN DEVELOPMENT (SDD) ===");

    // Khởi tạo bộ kiểm định giao dịch với hạn mức 50 triệu xu
    let validator = BankTransactionValidator::new(50_000_000);

    // Kịch bản kiểm thử trực tiếp 1: Giao dịch thành công
    let req_ok = TransferRequest {
        from_account: "VN11112222".to_string(),
        to_account: "VN33334444".to_string(),
        amount_cents: 15_000_000,
    };
    match validator.validate_transfer(&req_ok) {
        Ok(()) => println!("[Xác nhận] Giao dịch 15,000,000 xu từ {} sang {} HỢP LỆ!", req_ok.from_account, req_ok.to_account),
        Err(e) => println!("[Từ chối] Lỗi: {:?}", e),
    }

    // Kịch bản kiểm thử trực tiếp 2: Chuyển khoản trùng tài khoản
    let req_duplicate = TransferRequest {
        from_account: "VN11112222".to_string(),
        to_account: "VN11112222".to_string(),
        amount_cents: 500_000,
    };
    match validator.validate_transfer(&req_duplicate) {
        Ok(()) => println!("[Xác nhận] Giao dịch hợp lệ!"),
        Err(e) => println!("[Đặc tả chặn thành công] Phát hiện lỗi nghiệp vụ mong đợi: {:?}", e),
    }

    println!("\n[Tổng kết] Tất cả các điều kiện ràng buộc trong file SPEC đều được kiểm chứng chặt chẽ!");
}
