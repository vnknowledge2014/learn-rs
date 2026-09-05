#![allow(dead_code, unused_variables, unused_imports)]
// Tệp: src/main.rs
// Chương trình minh họa tư duy Lập trình hàm và Xây dựng Đường ống (Data Pipelines) trong Rust

#[derive(Debug, Clone, PartialEq)]
pub struct MatHang {
    pub ma_san_pham: String,
    pub ten_hang: String,
    pub don_gia: f64,
    pub so_luong: u32,
    pub da_thanh_toan: bool,
}

// ============================================================================
// HÀM THUẦN TÚY (PURE FUNCTIONS) - KHÔNG TÁC DỤNG PHỤ
// ============================================================================

/// Hàm thuần túy: Tính thành tiền của một mặt hàng
/// Nhận dữ liệu đầu vào và trả về giá trị mới, không thay đổi bất kỳ trạng thái nào
pub fn tinh_thanh_tien(hang: &MatHang) -> f64 {
    hang.don_gia * (hang.so_luong as f64)
}

/// Hàm thuần túy: Áp dụng phiếu giảm giá tỷ lệ phần trăm
pub fn ap_dung_giam_gia(tien_goc: f64, phan_tram_giam: f64) -> f64 {
    if phan_tram_giam <= 0.0 {
        tien_goc
    } else if phan_tram_giam >= 100.0 {
        0.0
    } else {
        tien_goc * (1.0 - (phan_tram_giam / 100.0))
    }
}

// ============================================================================
// SO SÁNH HAI CÁCH TIẾP CẬN TRÊN DỮ LIỆU
// ============================================================================

/// CÁCH 1: Phong cách Mệnh lệnh (Imperative)
/// Dùng vòng lặp thủ công, biến cờ mut tạm thời, dễ xảy ra lỗi ngoài ý muốn
pub fn xu_ly_menh_lenh(danh_sach: &[MatHang]) -> (f64, Vec<String>) {
    let mut tong_doanh_thu: f64 = 0.0;
    let mut danh_sach_ten: Vec<String> = Vec::new();

    // Vòng lặp thủ công với nhiều bước điều kiện lồng nhau
    for i in 0..danh_sach.len() {
        let hang = &danh_sach[i];
        // Chỉ xử lý các đơn hàng đã thanh toán và có giá trị trên 50.0
        if hang.da_thanh_toan {
            let thanh_tien = tinh_thanh_tien(hang);
            if thanh_tien >= 50.0 {
                tong_doanh_thu += thanh_tien;
                danh_sach_ten.push(hang.ten_hang.clone());
            }
        }
    }

    (tong_doanh_thu, danh_sach_ten)
}

/// CÁCH 2: Phong cách Lập trình Hàm Khai báo (Declarative Pipeline)
/// Dữ liệu chảy qua chuỗi lọc và ánh xạ, không dùng biến mut nào trong quá trình xử lý!
pub fn xu_ly_khai_bao(danh_sach: &[MatHang]) -> (f64, Vec<String>) {
    // 1. Nhánh tính tổng doanh thu thông qua đường ống (Pipeline)
    let tong_doanh_thu: f64 = danh_sach
        .iter()
        .filter(|hang| hang.da_thanh_toan)             // Bước 1: Lọc hàng đã trả tiền
        .map(|hang| tinh_thanh_tien(hang))             // Bước 2: Chuyển đổi thành tiền
        .filter(|&tien| tien >= 50.0)                  // Bước 3: Chỉ lấy món từ 50k trở lên
        .sum();                                        // Bước 4: Gom tụ tính tổng

    // 2. Nhánh trích xuất danh sách tên mặt hàng
    let danh_sach_ten: Vec<String> = danh_sach
        .iter()
        .filter(|hang| hang.da_thanh_toan && tinh_thanh_tien(hang) >= 50.0)
        .map(|hang| hang.ten_hang.clone())             // Ánh xạ sang chuỗi tên
        .collect();                                    // Gom vào vector mới

    (tong_doanh_thu, danh_sach_ten)
}

fn main() {
    println!("============================================================");
    println!("  HỆ THỐNG XỬ LÝ HÓA ĐƠN: LẬP TRÌNH MỆNH LỆNH VS ĐƯỜNG ỐNG  ");
    println!("============================================================");

    // Khởi tạo tập dữ liệu ban đầu bất biến
    let gio_hang: Vec<MatHang> = vec![
        MatHang {
            ma_san_pham: String::from("SP-01"),
            ten_hang: String::from("Sổ tay Lập trình Rust"),
            don_gia: 45.0,
            so_luong: 2,
            da_thanh_toan: true, // Thành tiền = 90.0 (Thỏa mãn >= 50)
        },
        MatHang {
            ma_san_pham: String::from("SP-02"),
            ten_hang: String::from("Bút bi kỹ thuật"),
            don_gia: 15.0,
            so_luong: 1,
            da_thanh_toan: true, // Thành tiền = 15.0 (Bị loại do < 50)
        },
        MatHang {
            ma_san_pham: String::from("SP-03"),
            ten_hang: String::from("Bàn phím cơ không dây"),
            don_gia: 120.0,
            so_luong: 1,
            da_thanh_toan: false, // Chưa thanh toán (Bị loại)
        },
        MatHang {
            ma_san_pham: String::from("SP-04"),
            ten_hang: String::from("Chuột công thái học"),
            don_gia: 75.0,
            so_luong: 1,
            da_thanh_toan: true, // Thành tiền = 75.0 (Thỏa mãn >= 50)
        },
    ];

    println!("Tổng số mặt hàng đưa vào xử lý: {}", gio_hang.len());

    // 1. Chạy theo phong cách mệnh lệnh
    let (doanh_thu_1, ten_1) = xu_ly_menh_lenh(&gio_hang);
    println!("\n[Kết quả Mệnh lệnh]:");
    println!("- Tổng doanh thu đạt chuẩn : {:.2} nghìn đồng", doanh_thu_1);
    println!("- Danh sách mặt hàng hợp lệ: {:?}", ten_1);

    // 2. Chạy theo phong cách khai báo đường ống
    let (doanh_thu_2, ten_2) = xu_ly_khai_bao(&gio_hang);
    println!("\n[Kết quả Khai báo Đường ống]:");
    println!("- Tổng doanh thu đạt chuẩn : {:.2} nghìn đồng", doanh_thu_2);
    println!("- Danh sách mặt hàng hợp lệ: {:?}", ten_2);

    // Xác thực hai cách tiếp cận cho ra cùng một kết quả nhất quán
    assert_eq!(doanh_thu_1, doanh_thu_2);
    assert_eq!(ten_1, ten_2);

    // Minh họa hàm thuần túy tính chiết khấu khuyến mãi độc lập
    let tong_sau_giam = ap_dung_giam_gia(doanh_thu_2, 10.0); // Giảm giá 10%
    println!("\n-> Doanh thu sau khi áp dụng phiếu giảm giá 10%: {:.2} nghìn đồng", tong_sau_giam);
    println!("============================================================");
}
