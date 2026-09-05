#![allow(dead_code, unused_variables, unused_imports)]
// Tệp: src/main.rs
// Chương trình thực chiến làm chủ Custom Derive, Attribute và Function-like Macros trong Rust

use std::collections::HashMap;

// ============================================================================
// 1. GIAO ƯỚC VÀ CÁC THỰC THỂ ĐƯỢC TỰ ĐỘNG SINH MÃ BỞI DERIVE MACRO
// ============================================================================

/// Trait mà Derive Macro #[derive(KiemToanBaoMat)] sẽ tự động sinh mã
pub trait KiemToanBaoMat {
    fn xuat_thong_tin_an_toan(&self) -> Vec<(&'static str, String)>;
    fn ma_phan_loai() -> &'static str;
}

pub struct TaiKhoanNganHang {
    pub so_tai_khoan: String,
    pub chu_tai_khoan: String,
    pub ma_pin_bi_mat: String, // Trường nhạy cảm: không được xuất ra nhật ký!
}

// Đoạn mã mà Custom Derive Macro tự động sinh ra cho TaiKhoanNganHang:
impl KiemToanBaoMat for TaiKhoanNganHang {
    fn xuat_thong_tin_an_toan(&self) -> Vec<(&'static str, String)> {
        // Macro thông minh tự động lọc bỏ trường nhạy cảm có gắn nhãn helper attribute
        vec![
            ("so_tai_khoan", self.so_tai_khoan.clone()),
            ("chu_tai_khoan", self.chu_tai_khoan.clone()),
            ("ma_pin_bi_mat", String::from("***ĐÃ_ẨN_BẢO_MẬT***")),
        ]
    }

    fn ma_phan_loai() -> &'static str {
        "TAI_KHOAN_NGAN_HANG_V1"
    }
}

// ============================================================================
// 2. MÔ HÌNH HÓA KẾT QUẢ CỦA ATTRIBUTE MACRO: #[kiem_soat_truy_cap]
// ============================================================================

/// Hàm mô phỏng mã sau khi được Attribute Macro bọc lớp vỏ bảo vệ
pub fn chuyen_khoan_an_toan(
    nguoi_gui: &str,
    nguoi_nhan: &str,
    so_tien: f64,
    vai_tro_nguoi_thuc_hien: &str,
) -> Result<String, &'static str> {
    // [MÃ DO ATTRIBUTE MACRO TỰ ĐỘNG CHÈN VÀO ĐẦU HÀM]:
    println!("[BẢO VỆ ATTRIBUTE] Đang xác thực quyền hạn của vai trò: '{}'", vai_tro_nguoi_thuc_hien);
    if vai_tro_nguoi_thuc_hien != "QuanTriVien" && vai_tro_nguoi_thuc_hien != "ChuTaiKhoan" {
        return Err("Từ chối truy cập: Bạn không có quyền thực hiện giao dịch này!");
    }

    // [THÂN HÀM NGUYÊN BẢN CỦA LẬP TRÌNH VIÊN]:
    println!("  -> Đang thực hiện chuyển {:.2} đồng từ {} sang {}", so_tien, nguoi_gui, nguoi_nhan);
    let ma_giao_dich = "GD-99882233";

    // [MÃ DO ATTRIBUTE MACRO TỰ ĐỘNG CHÈN VÀO CUỐI HÀM]:
    println!("[BẢO VỆ ATTRIBUTE] Giao dịch hoàn tất thành công. Mã định danh: {}", ma_giao_dich);
    Ok(format!("Chuyển tiền thành công! Mã giao dịch: {}", ma_giao_dich))
}

// ============================================================================
// 3. MÔ HÌNH HÓA FUNCTION-LIKE MACRO PHÂN TÍCH DSL CẤU HÌNH
// ============================================================================

/// Macro dạng hàm phân tích chuỗi cấu hình dạng "KEY=VALUE;KEY=VALUE" lúc biên dịch
macro_rules! phan_tich_cau_hinh {
    ( $( $khoa:ident = $gia_tri:expr );* $(;)? ) => {
        {
            let mut ban_do = HashMap::new();
            $(
                ban_do.insert(stringify!($khoa), $gia_tri);
            )*
            ban_do
        }
    };
}

// ============================================================================
// CHƯƠNG TRÌNH THỰC THI CHÍNH
// ============================================================================

fn main() {
    println!("============================================================");
    println!("     CHẾ TẠO VÀ ỨNG DỤNG BỘ BA PROCEDURAL MACROS TRONG RUST ");
    println!("============================================================");

    // ------------------------------------------------------------------------
    // 1. Kiểm chứng Custom Derive Macro với Helper Attribute
    // ------------------------------------------------------------------------
    println!("\n1. Ứng dụng Custom Derive Macro [KiemToanBaoMat]:");
    let tai_khoan = TaiKhoanNganHang {
        so_tai_khoan: String::from("1900-8888-9999"),
        chu_tai_khoan: String::from("Nguyễn Văn An"),
        ma_pin_bi_mat: String::from("SecretPin1234"),
    };

    println!("Mã phân loại thực thể: {}", TaiKhoanNganHang::ma_phan_loai());
    println!("Danh sách trường được xuất ra an toàn:");
    for (ten_truong, gia_tri) in tai_khoan.xuat_thong_tin_an_toan() {
        println!("  - {}: {}", ten_truong, gia_tri);
    }

    // ------------------------------------------------------------------------
    // 2. Kiểm chứng Attribute-like Macro bọc lớp bảo vệ
    // ------------------------------------------------------------------------
    println!("\n2. Ứng dụng Attribute Macro kiểm soát quyền truy cập:");
    
    // Thử nghiệm gọi với quyền hợp lệ
    let ket_qua_hop_le = chuyen_khoan_an_toan(
        "NguyenVanA", 
        "TranThiB", 
        5000.0, 
        "ChuTaiKhoan"
    );
    match ket_qua_hop_le {
        Ok(msg) => println!("  [OK] {}", msg),
        Err(e) => println!("  [LỖI] {}", e),
    }

    // Thử nghiệm gọi với quyền trái phép (Bị chặn ngay ở cổng)
    let ket_qua_vi_pham = chuyen_khoan_an_toan(
        "NguyenVanA", 
        "KeXau", 
        999999.0, 
        "KhachLa"
    );
    match ket_qua_vi_pham {
        Ok(msg) => println!("  [NGUY HIỂM] Lọt qua kiểm duyệt: {}", msg),
        Err(ly_do) => println!("  [CHẶN THÀNH CÔNG] {}", ly_do),
    }

    // ------------------------------------------------------------------------
    // 3. Ứng dụng Function-like Macro xử lý DSL tùy biến
    // ------------------------------------------------------------------------
    println!("\n3. Ứng dụng Function-like Macro khởi tạo cấu hình bảo mật:");
    let cau_hinh = phan_tich_cau_hinh! {
        TIMEOUT = 30;
        MAX_RETRY = 3;
        PORT = 8443;
    };

    for (k, v) in &cau_hinh {
        println!("  Tham số hệ thống `{}` được nạp với giá trị: {}", k, v);
    }

    println!("\n============================================================");
    println!("     HOÀN TẤT CHƯƠNG TRÌNH LÀM CHỦ BỘ BA PROCEDURAL MACROS  ");
    println!("============================================================");
}
