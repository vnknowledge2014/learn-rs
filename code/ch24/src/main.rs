// Tệp: src/main.rs
// Chương trình thực chiến làm chủ Custom Derive, Attribute và Function-like Macros trong Rust
//
// Ba macro dùng ở đây đều là macro thủ tục THẬT, viết trong crate `ch24_macros`
// (proc-macro = true) bằng syn + quote.

use ch24_macros::{SecurityAudit, config, require_role};

// ============================================================================
// 1. CUSTOM DERIVE + HELPER ATTRIBUTE: #[derive(SecurityAudit)] / #[audit(...)]
// ============================================================================

/// Trait mà `#[derive(SecurityAudit)]` tự động cài đặt.
/// (Crate proc-macro không xuất được trait, nên trait sống ở crate này.)
pub trait SecurityAudit {
    /// Danh sách (tên trường, giá trị) AN TOÀN để ghi nhật ký
    fn audit_fields(&self) -> Vec<(&'static str, String)>;
    /// Tên loại thực thể (sinh từ tên struct)
    fn entity_kind() -> &'static str;
}

#[derive(SecurityAudit)]
pub struct BankAccount {
    pub account_number: String,
    pub account_owner: String,
    #[audit(sensitive)] // Trường nhạy cảm: hiện tên, che giá trị
    pub pin_code: String,
    #[audit(skip)] // Bỏ hẳn khỏi nhật ký
    pub internal_note: String,
}

// ============================================================================
// 2. ATTRIBUTE-LIKE MACRO: #[require_role(...)] BỌC HÀM BẰNG LỚP KIỂM QUYỀN
// ============================================================================

/// Lập trình viên chỉ viết phần thân nghiệp vụ. Macro chèn đoạn kiểm tra
/// `role` vào ĐẦU thân hàm; vai trò không nằm trong danh sách sẽ bị trả `Err`
/// trước khi dòng nghiệp vụ nào kịp chạy.
#[require_role("admin", "owner")]
pub fn safe_transfer(
    sender: &str,
    recipient: &str,
    amount_vnd: u64, // tiền tệ: số nguyên đơn vị nhỏ nhất, KHÔNG dùng f64
    role: &str,
) -> Result<String, &'static str> {
    println!(
        "  -> Đang chuyển {} đồng từ {} sang {}",
        amount_vnd, sender, recipient
    );
    let transaction_id = "GD-99882233";
    Ok(format!(
        "Chuyển tiền thành công! Mã giao dịch: {}",
        transaction_id
    ))
}

// ============================================================================
// 3. FUNCTION-LIKE MACRO: config! { ... } — DSL CẤU HÌNH KIỂM TRA LÚC BIÊN DỊCH
// ============================================================================

/// Viết `MAX_RETRY = 3;` hai lần, hay `timeout = 30;` (chữ thường), thì chương
/// trình KHÔNG biên dịch được — lỗi trỏ đúng vào khoá sai.
fn load_config() -> std::collections::HashMap<&'static str, i64> {
    config! {
        TIMEOUT = 30;
        MAX_RETRY = 3;
        PORT = 8443;
    }
}

// ============================================================================
// CHƯƠNG TRÌNH THỰC THI CHÍNH
// ============================================================================

fn main() {
    println!("============================================================");
    println!("     CHẾ TẠO VÀ ỨNG DỤNG BỘ BA PROCEDURAL MACROS TRONG RUST ");
    println!("============================================================");

    // ------------------------------------------------------------------------
    // 1. Custom Derive Macro với Helper Attribute
    // ------------------------------------------------------------------------
    println!("\n1. Ứng dụng Custom Derive Macro [SecurityAudit]:");
    let account = BankAccount {
        account_number: String::from("1900-8888-9999"),
        account_owner: String::from("Nguyễn Văn An"),
        pin_code: String::from("SecretPin1234"),
        internal_note: String::from("khách VIP"),
    };

    println!("Loại thực thể: {}", BankAccount::entity_kind());
    println!("Danh sách trường được xuất ra an toàn:");
    for (field_name, value) in account.audit_fields() {
        println!("  - {}: {}", field_name, value);
    }

    // ------------------------------------------------------------------------
    // 2. Attribute-like Macro bọc lớp bảo vệ
    // ------------------------------------------------------------------------
    println!("\n2. Ứng dụng Attribute Macro kiểm soát quyền truy cập:");

    // Thử nghiệm gọi với quyền hợp lệ
    match safe_transfer("NguyenVanA", "TranThiB", 5_000_000, "owner") {
        Ok(msg) => println!("  [OK] {}", msg),
        Err(e) => println!("  [LỖI] {}", e),
    }

    // Thử nghiệm gọi với quyền trái phép (Bị chặn ngay ở cổng)
    match safe_transfer("NguyenVanA", "KeXau", 999_999_000, "guest") {
        Ok(msg) => println!("  [NGUY HIỂM] Lọt qua kiểm duyệt: {}", msg),
        Err(reason) => println!("  [CHẶN THÀNH CÔNG] {}", reason),
    }

    // ------------------------------------------------------------------------
    // 3. Function-like Macro xử lý DSL tùy biến
    // ------------------------------------------------------------------------
    println!("\n3. Ứng dụng Function-like Macro khởi tạo cấu hình bảo mật:");
    let config = load_config();
    let mut keys: Vec<_> = config.keys().collect();
    keys.sort();
    for k in keys {
        println!(
            "  Tham số hệ thống `{}` được nạp với giá trị: {}",
            k, config[k]
        );
    }

    println!("\n============================================================");
    println!("     HOÀN TẤT CHƯƠNG TRÌNH LÀM CHỦ BỘ BA PROCEDURAL MACROS  ");
    println!("============================================================");
}

// ============================================================================
// KIỂM THỬ: KIỂM CHỨNG MÃ DO CẢ BA MACRO SINH RA
// ============================================================================

#[cfg(test)]
mod tests {
    use super::*;

    fn sample() -> BankAccount {
        BankAccount {
            account_number: "123".into(),
            account_owner: "An".into(),
            pin_code: "0000".into(),
            internal_note: "bí mật".into(),
        }
    }

    #[test]
    fn derive_masks_sensitive_and_drops_skipped_fields() {
        let fields = sample().audit_fields();
        assert_eq!(
            fields,
            vec![
                ("account_number", "123".to_string()),
                ("account_owner", "An".to_string()),
                ("pin_code", "***ĐÃ ẨN***".to_string()),
            ]
        );
        // Giá trị thật của PIN và ghi chú KHÔNG bao giờ lọt vào nhật ký
        assert!(fields.iter().all(|(_, v)| v != "0000" && v != "bí mật"));
    }

    #[test]
    fn derive_generates_entity_kind() {
        assert_eq!(BankAccount::entity_kind(), "BankAccount");
    }

    #[test]
    fn attribute_allows_listed_roles() {
        assert!(safe_transfer("A", "B", 1, "admin").is_ok());
        assert!(safe_transfer("A", "B", 1, "owner").is_ok());
    }

    #[test]
    fn attribute_blocks_other_roles_before_body_runs() {
        let err = safe_transfer("A", "B", 1, "guest").unwrap_err();
        assert!(err.starts_with("Từ chối truy cập"));
    }

    #[test]
    fn function_like_macro_builds_map() {
        let c = load_config();
        assert_eq!(c.len(), 3);
        assert_eq!(c["PORT"], 8443);
        assert_eq!(c["MAX_RETRY"], 3);
    }
}
