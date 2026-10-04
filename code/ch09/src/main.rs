// File: src/main.rs
// Chương trình làm chủ Structs, Tuples & Phương thức trong Rust

// 1. Tuple Struct: Biểu diễn tọa độ GPS của trụ sở ngân hàng (Vĩ độ, Kinh độ)
struct GpsCoord(f64, f64);

// 2. Unit-like Struct: Đóng vai trò như một nhãn chứng thực bảo mật giao dịch
struct SecurityAttestation;

// 3. Classic Struct: Định nghĩa cấu trúc tài khoản ngân hàng hoàn chỉnh
// Số dư lưu bằng số nguyên u64 (đơn vị: đồng), KHÔNG dùng f64 — nhớ lại cảnh báo
// ở Chương 03: số thực có sai số làm tròn, không bao giờ dùng để tính tiền.
struct BankAccount {
    account_number: String,
    account_owner: String,
    balance: u64,
    is_active: bool,
}

// Khối hiện thực các phương thức và hàm liên kết cho BankAccount
impl BankAccount {
    // A. HÀM LIÊN KẾT (Associated Function) - Khởi tạo tài khoản mới chuẩn mực
    fn open_account(account_number: String, account_owner: String, initial_balance: u64) -> Self {
        println!("-> Đang mở tài khoản mới cho khách hàng: {}", account_owner);
        // Field Init Shorthand: tên tham số trùng tên trường nên không cần viết `x: x`
        Self {
            account_number,
            account_owner,
            balance: initial_balance,
            is_active: true,
        }
    }

    // B. PHƯƠNG THỨC MƯỢN ĐỌC (&self): Tra cứu thông tin số dư an toàn
    fn show_info(&self) {
        println!("------------------------------------------------------------");
        println!("Số tài khoản : {}", self.account_number);
        println!("Chủ tài khoản: {}", self.account_owner);
        println!("Số dư hiện có: {} VND", self.balance);
        println!(
            "Trạng thái   : {}",
            if self.is_active {
                "Hoạt động"
            } else {
                "Đã khóa"
            }
        );
        println!("------------------------------------------------------------");
    }

    // C. PHƯƠNG THỨC MƯỢN SỬA (&mut self): Nạp tiền vào tài khoản
    fn deposit(&mut self, amount: u64) {
        if amount == 0 {
            println!("[!] Lỗi: Số tiền nạp phải lớn hơn 0!");
            return;
        }
        self.balance += amount;
        println!(
            "-> Nạp thành công {} VND vào tài khoản {}",
            amount, self.account_number
        );
    }

    // D. PHƯƠNG THỨC MƯỢN SỬA (&mut self): Rút tiền có kiểm tra số dư
    fn withdraw(&mut self, amount: u64) -> bool {
        if amount > self.balance {
            println!(
                "[!] Giao dịch thất bại: Số dư không đủ để rút {} VND!",
                amount
            );
            false
        } else {
            self.balance -= amount;
            println!(
                "-> Rút thành công {} VND. Số dư còn lại: {} VND",
                amount, self.balance
            );
            true
        }
    }

    // E. PHƯƠNG THỨC TIÊU THỤ SỞ HỮU (self): Đóng tài khoản vĩnh viễn
    fn close_account(self) {
        println!("\n*** TIẾN HÀNH TẤT TOÁN VÀ HỦY TÀI KHOẢN ***");
        println!(
            "- Hoàn trả toàn bộ số dư cuối cùng: {} VND cho ông/bà {}",
            self.balance, self.account_owner
        );
        println!(
            "- Tài khoản số {} đã bị đóng và giải phóng khỏi hệ thống.",
            self.account_number
        );
        // Khi hàm này kết thúc, self bị Drop ngay tại đây!
    }
}

fn main() {
    println!("============================================================");
    println!("     HỆ THỐNG QUẢN LÝ TÀI KHOẢN NGÂN HÀNG ĐIỆN TỬ RUST      ");
    println!("============================================================");

    // Sử dụng Tuple Struct để lưu tọa độ chi nhánh ngân hàng
    let hanoi_branch = GpsCoord(21.0285, 105.8542);
    println!(
        "Tọa độ chi nhánh giao dịch: Vĩ độ {}, Kinh độ {}",
        hanoi_branch.0, hanoi_branch.1
    );

    // Khởi tạo Unit-like Struct làm chứng thực an toàn cho phiên làm việc
    let _auth_session = SecurityAttestation;
    println!("Chứng thực bảo mật hệ thống: Đã kích hoạt tem xác thực điện tử.");

    // Mở một tài khoản ngân hàng mới thông qua hàm liên kết open_account
    let mut main_account = BankAccount::open_account(
        String::from("1900-123-456"),
        String::from("Nguyễn Văn An"),
        1_000_000,
    );

    // Tra cứu thông tin (gọi phương thức &self)
    main_account.show_info();

    // Thực hiện các giao dịch làm biến đổi số dư (gọi phương thức &mut self)
    main_account.deposit(500_000);
    main_account.withdraw(200_000);
    main_account.withdraw(2_000_000); // Thử rút vượt số dư

    // Tra cứu lại thông tin sau giao dịch
    main_account.show_info();

    // Minh họa Cú pháp cập nhật Struct (Struct Update Syntax ..)
    let savings_account = BankAccount {
        account_number: String::from("1900-999-888"),
        balance: 50_000,
        ..BankAccount::open_account(
            String::from("TEMP"),
            String::from("Nguyễn Văn An (Tài khoản tiết kiệm)"),
            0,
        )
    };
    println!("\nTài khoản phụ được tạo tự động:");
    savings_account.show_info();

    // Đóng tài khoản chính (gọi phương thức tiêu thụ self)
    main_account.close_account();

    // NẾU BẠN BỎ CHÚ THÍCH DÒNG SAU, RUSTC SẼ BÁO LỖI E0382 NGAY:
    // main_account.show_info(); // LỖI: Giá trị main_account đã bị tiêu thụ khi đóng sổ!
}

#[cfg(test)]
mod tests {
    use super::*;

    fn sample() -> BankAccount {
        BankAccount::open_account(String::from("1"), String::from("An"), 1_000)
    }

    #[test]
    fn deposit_and_withdraw_are_exact() {
        let mut account = sample();
        account.deposit(500);
        assert!(account.withdraw(200));
        assert_eq!(account.balance, 1_300); // số nguyên: không có sai số làm tròn
    }

    #[test]
    fn overdraw_is_rejected_and_balance_unchanged() {
        let mut account = sample();
        assert!(!account.withdraw(2_000));
        assert_eq!(account.balance, 1_000);
    }

    #[test]
    fn zero_deposit_is_ignored() {
        let mut account = sample();
        account.deposit(0);
        assert_eq!(account.balance, 1_000);
    }
}
