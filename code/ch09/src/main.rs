#![allow(dead_code, unused_variables, unused_imports)]
// File: src/main.rs
// Chương trình làm chủ Structs, Tuples & Phương thức trong Rust

// 1. Tuple Struct: Biểu diễn tọa độ GPS của trụ sở ngân hàng (Kinh độ, Vĩ độ)
struct ToaDoGps(f64, f64);

// 2. Unit-like Struct: Đóng vai trò như một nhãn chứng thực bảo mật giao dịch
struct ChungThucBaoMat;

// 3. Classic Struct: Định nghĩa cấu trúc tài khoản ngân hàng hoàn chỉnh
struct TaiKhoanNganHang {
    so_tai_khoan: String,
    chu_tai_khoan: String,
    so_du: f64,
    kich_hoat: bool,
}

// Khối hiện thực các phương thức và hàm liên kết cho TaiKhoanNganHang
impl TaiKhoanNganHang {
    // A. HÀM LIÊN KẾT (Associated Function) - Khởi tạo tài khoản mới chuẩn mực
    fn mo_tai_khoan(so_tk: String, chu_tk: String, so_du_dau: f64) -> Self {
        println!("-> Đang mở tài khoản mới cho khách hàng: {}", chu_tk);
        Self {
            so_tai_khoan: so_tk,
            chu_tai_khoan: chu_tk,
            so_du: so_du_dau,
            kich_hoat: true,
        }
    }

    // B. PHƯƠNG THỨC MƯỢN ĐỌC (&self): Tra cứu thông tin số dư an toàn
    fn tra_cuu_thong_tin(&self) {
        println!("------------------------------------------------------------");
        println!("Số tài khoản : {}", self.so_tai_khoan);
        println!("Chủ tài khoản: {}", self.chu_tai_khoan);
        println!("Số dư hiện có: {:.2} VND", self.so_du);
        println!("Trạng thái   : {}", if self.kich_hoat { "Hoạt động" } else { "Đã khóa" });
        println!("------------------------------------------------------------");
    }

    // C. PHƯƠNG THỨC MƯỢN SỬA (&mut self): Nạp tiền vào tài khoản
    fn nap_tien(&mut self, so_tien: f64) {
        if so_tien <= 0.0 {
            println!("[!] Lỗi: Số tiền nạp phải lớn hơn 0!");
            return;
        }
        self.so_du += so_tien;
        println!("-> Nạp thành công {:.2} VND vào tài khoản {}", so_tien, self.so_tai_khoan);
    }

    // D. PHƯƠNG THỨC MƯỢN SỬA (&mut self): Rút tiền có kiểm tra số dư
    fn rut_tien(&mut self, so_tien: f64) -> bool {
        if so_tien > self.so_du {
            println!("[!] Giao dịch thất bại: Số dư không đủ để rút {:.2} VND!", so_tien);
            false
        } else {
            self.so_du -= so_tien;
            println!("-> Rút thành công {:.2} VND. Số dư còn lại: {:.2} VND", so_tien, self.so_du);
            true
        }
    }

    // E. PHƯƠNG THỨC TIÊU THỤ SỞ HỮU (self): Đóng tài khoản vĩnh viễn
    fn tat_toan_va_dong_so(self) {
        println!("\n*** TIẾN HÀNH TẤT TOÁN VÀ HỦY TÀI KHOẢN ***");
        println!("- Hoàn trả toàn bộ số dư cuối cùng: {:.2} VND cho ông/bà {}", 
                 self.so_du, self.chu_tai_khoan);
        println!("- Tài khoản số {} đã bị đóng và giải phóng khỏi hệ thống.", self.so_tai_khoan);
        // Khi hàm này kết thúc, self bị Drop ngay tại đây!
    }
}

fn main() {
    println!("============================================================");
    println!("     HỆ THỐNG QUẢN LÝ TÀI KHOẢN NGÂN HÀNG ĐIỆN TỬ RUST      ");
    println!("============================================================");

    // Sử dụng Tuple Struct để lưu tọa độ chi nhánh ngân hàng
    let chi_nhanh_ha_noi = ToaDoGps(21.0285, 105.8542);
    println!("Tọa độ chi nhánh giao dịch: Vĩ độ {}, Kinh độ {}", 
             chi_nhanh_ha_noi.0, chi_nhanh_ha_noi.1);

    // Khởi tạo Unit-like Struct làm chứng thực an toàn cho phiên làm việc
    let _chung_thuc_phien = ChungThucBaoMat;
    println!("Chứng thực bảo mật hệ thống: Đã kích hoạt tem xác thực điện tử.");

    // Mở một tài khoản ngân hàng mới thông qua hàm liên kết mo_tai_khoan
    let mut tk_an = TaiKhoanNganHang::mo_tai_khoan(
        String::from("1900-123-456"),
        String::from("Nguyễn Văn An"),
        1_000_000.0,
    );

    // Tra cứu thông tin (gọi phương thức &self)
    tk_an.tra_cuu_thong_tin();

    // Thực hiện các giao dịch làm biến đổi số dư (gọi phương thức &mut self)
    tk_an.nap_tien(500_000.0);
    tk_an.rut_tien(200_000.0);
    tk_an.rut_tien(2_000_000.0); // Thử rút vượt số dư

    // Tra cứu lại thông tin sau giao dịch
    tk_an.tra_cuu_thong_tin();

    // Minh họa Cú pháp cập nhật Struct (Struct Update Syntax ..)
    let tk_phu = TaiKhoanNganHang {
        so_tai_khoan: String::from("1900-999-888"),
        so_du: 50_000.0,
        ..TaiKhoanNganHang::mo_tai_khoan(
            String::from("TEMP"),
            String::from("Nguyễn Văn An (Tài khoản tiết kiệm)"),
            0.0
        )
    };
    println!("\nTài khoản phụ được tạo tự động:");
    tk_phu.tra_cuu_thong_tin();

    // Đóng tài khoản chính (gọi phương thức tiêu thụ self)
    tk_an.tat_toan_va_dong_so();

    // NẾU BẠN BỎ CHÚ THÍCH DÒNG SAU, RUSTC SẼ BÁO LỖI E0382 NGAY:
    // tk_an.tra_cuu_thong_tin(); // LỖI: Giá trị tk_an đã bị tiêu thụ khi đóng sổ!
}
