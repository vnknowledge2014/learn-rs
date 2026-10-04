// File: src/main.rs
// Chương trình thực chiến làm chủ Kỹ thuật Xử lý Lỗi Chuyên Nghiệp trong Rust
// (Số tiền dùng u64 — đơn vị đồng — chứ không dùng f64, theo cảnh báo ở Chương 03.)

// 1. Tự định nghĩa kiểu Lỗi Nghiệp Vụ Tùy Biến (Custom Error Type) bằng Enum
#[derive(Debug, PartialEq)]
enum PaymentError {
    InvalidAmount(String),
    AccountLocked,
    InsufficientBalance { balance: u64, requested: u64 },
}

// Cài đặt khả năng in ấn đẹp mắt cho kiểu lỗi của chúng ta
impl std::fmt::Display for PaymentError {
    fn fmt(&self, f: &mut std::fmt::Formatter) -> std::fmt::Result {
        match self {
            PaymentError::InvalidAmount(msg) => write!(f, "Số tiền không hợp lệ: {}", msg),
            PaymentError::AccountLocked => write!(f, "Tài khoản đang bị khóa do vi phạm an ninh!"),
            PaymentError::InsufficientBalance { balance, requested } => {
                write!(
                    f,
                    "Số dư không đủ (Hiện có: {}, Yêu cầu rút: {})",
                    balance, requested
                )
            }
        }
    }
}

// 2. Hàm kiểm tra tính hợp lệ của số tiền nhập vào
fn parse_amount(input: &str) -> Result<u64, PaymentError> {
    let amount: u64 = input.trim().parse().map_err(|_| {
        PaymentError::InvalidAmount(String::from("Vui lòng chỉ nhập các chữ số hợp lệ!"))
    })?;

    if amount == 0 {
        return Err(PaymentError::InvalidAmount(String::from(
            "Số tiền phải lớn hơn 0!",
        )));
    }

    Ok(amount)
}

// 3. Hàm thực hiện giao dịch rút tiền: Tận dụng toán tử '?' để lan truyền lỗi siêu gọn
fn withdraw(
    input: &str,
    current_balance: u64,
    is_account_active: bool,
) -> Result<u64, PaymentError> {
    // Bước 1: Kiểm tra trạng thái tài khoản
    if !is_account_active {
        return Err(PaymentError::AccountLocked);
    }

    // Bước 2: Phân tích số tiền bằng toán tử '?'
    // Nếu parse_amount trả về Err, hàm lập tức return Err ngay tại dòng này!
    let amount = parse_amount(input)?;

    // Bước 3: Kiểm tra hạn mức số dư
    if amount > current_balance {
        return Err(PaymentError::InsufficientBalance {
            balance: current_balance,
            requested: amount,
        });
    }

    // Bước 4: Trừ tiền thành công, trả về số dư mới bọc trong Ok
    Ok(current_balance - amount)
}

// 4. Giả lập một lời gọi mạng luôn thất bại (để minh hoạ unwrap_or)
fn fetch_balance_from_server() -> Result<u64, &'static str> {
    Err("Mất kết nối máy chủ")
}

fn main() {
    println!("============================================================");
    println!("     CỔNG THANH TOÁN TÀI CHÍNH AN TOÀN - RUST BANKING       ");
    println!("============================================================");

    let initial_balance: u64 = 5_000_000;

    // --- KỊCH BẢN 1: GIAO DỊCH THÀNH CÔNG HỢP LỆ ---
    println!("\n[Kịch bản 1] Rút 1.500.000 VND hợp lệ:");
    match withdraw("1500000", initial_balance, true) {
        Ok(new_balance) => println!(
            "-> Giao dịch THÀNH CÔNG! Số dư còn lại: {} VND",
            new_balance
        ),
        Err(e) => println!("-> Giao dịch THẤT BẠI: {}", e),
    }

    // --- KỊCH BẢN 2: LỖI NHẬP LIỆU KHÔNG PHẢI CHỮ SỐ ---
    println!("\n[Kịch bản 2] Người dùng nhập chữ linh tinh:");
    match withdraw("một triệu", initial_balance, true) {
        Ok(new_balance) => println!("-> Thành công: {} VND", new_balance),
        Err(e) => println!("-> Hệ thống xử lý êm dịu: [{}]", e),
    }

    // --- KỊCH BẢN 3: LỖI SỐ DƯ KHÔNG ĐỦ ĐỂ RÚT ---
    println!("\n[Kịch bản 3] Rút số tiền vượt hạn mức số dư:");
    match withdraw("10000000", initial_balance, true) {
        Ok(new_balance) => println!("-> Thành công: {} VND", new_balance),
        Err(e) => println!("-> Báo cáo lỗi chính xác: [{}]", e),
    }

    // --- KỊCH BẢN 4: LỖI TÀI KHOẢN BỊ KHÓA AN NINH ---
    println!("\n[Kịch bản 4] Tài khoản bị phong tỏa:");
    match withdraw("500000", initial_balance, false) {
        Ok(new_balance) => println!("-> Thành công: {} VND", new_balance),
        Err(e) => println!("-> Từ chối truy cập: [{}]", e),
    }

    // --- KỊCH BẢN 5: CÁC PHƯƠNG THỨC XỬ LÝ DỰ PHÒNG AN TOÀN ---
    println!("\n[Kịch bản 5] Sử dụng unwrap_or để lấy giá trị mặc định an toàn:");
    let fallback_amount = fetch_balance_from_server().unwrap_or(0);
    println!(
        "- Giá trị an toàn thu được: {} VND (không hề bị sập ứng dụng!)",
        fallback_amount
    );
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn successful_withdrawal_returns_new_balance() {
        assert_eq!(withdraw("1500000", 5_000_000, true), Ok(3_500_000));
    }

    #[test]
    fn non_numeric_and_zero_amounts_are_invalid() {
        assert!(matches!(
            withdraw("một triệu", 5_000_000, true),
            Err(PaymentError::InvalidAmount(_))
        ));
        assert!(matches!(
            withdraw("0", 5_000_000, true),
            Err(PaymentError::InvalidAmount(_))
        ));
        // Số âm cũng không parse được thành u64 -> InvalidAmount
        assert!(matches!(
            withdraw("-5", 5_000_000, true),
            Err(PaymentError::InvalidAmount(_))
        ));
    }

    #[test]
    fn overdraw_and_locked_account_are_rejected() {
        assert_eq!(
            withdraw("10000000", 5_000_000, true),
            Err(PaymentError::InsufficientBalance {
                balance: 5_000_000,
                requested: 10_000_000
            })
        );
        assert_eq!(
            withdraw("500000", 5_000_000, false),
            Err(PaymentError::AccountLocked)
        );
    }
}
