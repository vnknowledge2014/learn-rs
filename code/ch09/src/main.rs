#![allow(dead_code, unused_variables, unused_imports)]
// File: src/main.rs
// Chương trình làm chủ Structs, Tuples & Phương thức trong Rust

// 1. Tuple Struct: Biểu diễn tọa độ GPS của trụ sở ngân hàng (Kinh độ, Vĩ độ)
struct GpsCoord(f64, f64);

// 2. Unit-like Struct: Đóng vai trò như một nhãn chứng thực bảo mật giao dịch
struct LostReport;

// 3. Classic Struct: Định nghĩa cấu trúc tài khoản ngân hàng hoàn chỉnh
struct AccountBank {
    num_account: String,
    account_owner: String,
    balance: f64,
    activate: bool,
}

// Khối hiện thực các phương thức và hàm liên kết cho AccountBank
impl AccountBank {
    // A. HÀM LIÊN KẾT (Associated Function) - Khởi tạo tài khoản mới chuẩn mực
    fn open_account(so_tk: String, chu_tk: String, so_du_dau: f64) -> Self {
        println!("-> Đang mở tài khoản mới cho khách hàng: {}", chu_tk);
        Self {
            num_account: so_tk,
            account_owner: chu_tk,
            balance: so_du_dau,
            activate: true,
        }
    }

    // B. PHƯƠNG THỨC MƯỢN ĐỌC (&self): Tra cứu thông tin số dư an toàn
    fn tra_cuu_thong_tin(&self) {
        println!("------------------------------------------------------------");
        println!("Số tài khoản : {}", self.num_account);
        println!("Chủ tài khoản: {}", self.account_owner);
        println!("Số dư hiện có: {:.2} VND", self.balance);
        println!("Trạng thái   : {}", if self.activate { "Hoạt động" } else { "Đã khóa" });
        println!("------------------------------------------------------------");
    }

    // C. PHƯƠNG THỨC MƯỢN SỬA (&mut self): Nạp tiền vào tài khoản
    fn nap_tien(&mut self, so_tien: f64) {
        if so_tien <= 0.0 {
            println!("[!] Lỗi: Số tiền nạp phải lớn hơn 0!");
            return;
        }
        self.balance += so_tien;
        println!("-> Nạp thành công {:.2} VND vào tài khoản {}", so_tien, self.num_account);
    }

    // D. PHƯƠNG THỨC MƯỢN SỬA (&mut self): Rút tiền có kiểm tra số dư
    fn rut_tien(&mut self, so_tien: f64) -> bool {
        if so_tien > self.balance {
            println!("[!] Giao dịch thất bại: Số dư không đủ để rút {:.2} VND!", so_tien);
            false
        } else {
            self.balance -= so_tien;
            println!("-> Rút thành công {:.2} VND. Số dư còn lại: {:.2} VND", so_tien, self.balance);
            true
        }
    }

    // E. PHƯƠNG THỨC TIÊU THỤ SỞ HỮU (self): Đóng tài khoản vĩnh viễn
    fn all_math_and_round(self) {
        println!("\n*** TIẾN HÀNH TẤT TOÁN VÀ HỦY TÀI KHOẢN ***");
        println!("- Hoàn trả toàn bộ số dư cuối cùng: {:.2} VND cho ông/bà {}", 
                 self.balance, self.account_owner);
        println!("- Tài khoản số {} đã bị đóng và giải phóng khỏi hệ thống.", self.num_account);
        // Khi hàm này kết thúc, self bị Drop ngay tại đây!
    }
}

fn main() {
    println!("============================================================");
    println!("     HỆ THỐNG QUẢN LÝ TÀI KHOẢN NGÂN HÀNG ĐIỆN TỬ RUST      ");
    println!("============================================================");

    // Sử dụng Tuple Struct để lưu tọa độ chi nhánh ngân hàng
    let chi_nhanh_ha_noi = GpsCoord(21.0285, 105.8542);
    println!("Tọa độ chi nhánh giao dịch: Vĩ độ {}, Kinh độ {}", 
             chi_nhanh_ha_noi.0, chi_nhanh_ha_noi.1);

    // Khởi tạo Unit-like Struct làm chứng thực an toàn cho phiên làm việc
    let _chung_thuc_session = LostReport;
    println!("Chứng thực bảo mật hệ thống: Đã kích hoạt tem xác thực điện tử.");

    // Mở một tài khoản ngân hàng mới thông qua hàm liên kết open_account
    let mut account_hidden = AccountBank::open_account(
        String::from("1900-123-456"),
        String::from("Nguyễn Văn An"),
        1_000_000.0,
    );

    // Tra cứu thông tin (gọi phương thức &self)
    account_hidden.tra_cuu_thong_tin();

    // Thực hiện các giao dịch làm biến đổi số dư (gọi phương thức &mut self)
    account_hidden.nap_tien(500_000.0);
    account_hidden.rut_tien(200_000.0);
    account_hidden.rut_tien(2_000_000.0); // Thử rút vượt số dư

    // Tra cứu lại thông tin sau giao dịch
    account_hidden.tra_cuu_thong_tin();

    // Minh họa Cú pháp cập nhật Struct (Struct Update Syntax ..)
    let account_aux = AccountBank {
        num_account: String::from("1900-999-888"),
        balance: 50_000.0,
        ..AccountBank::open_account(
            String::from("TEMP"),
            String::from("Nguyễn Văn An (Tài khoản tiết kiệm)"),
            0.0
        )
    };
    println!("\nTài khoản phụ được tạo tự động:");
    account_aux.tra_cuu_thong_tin();

    // Đóng tài khoản chính (gọi phương thức tiêu thụ self)
    account_hidden.all_math_and_round();

    // NẾU BẠN BỎ CHÚ THÍCH DÒNG SAU, RUSTC SẼ BÁO LỖI E0382 NGAY:
    // account_hidden.tra_cuu_thong_tin(); // LỖI: Giá trị account_hidden đã bị tiêu thụ khi đóng sổ!
}
