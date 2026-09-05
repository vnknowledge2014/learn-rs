#![allow(dead_code, unused_variables, unused_imports)]
// Tệp: src/main.rs
// Chương trình minh họa tư duy Lập trình hàm và Xây dựng Đường ống (Data Pipelines) trong Rust

#[derive(Debug, Clone, PartialEq)]
pub struct MatQueue {
    pub ma_san_pham: String,
    pub name_queue: String,
    pub don_price: f64,
    pub quantity: u32,
    pub is_paid: bool,
}

// ============================================================================
// HÀM THUẦN TÚY (PURE FUNCTIONS) - KHÔNG TÁC DỤNG PHỤ
// ============================================================================

/// Hàm thuần túy: Tính thành tiền của một mặt hàng
/// Nhận dữ liệu đầu vào và trả về giá trị mới, không thay đổi bất kỳ trạng thái nào
pub fn to_money(queue: &MatQueue) -> f64 {
    queue.don_price * (queue.quantity as f64)
}

/// Hàm thuần túy: Áp dụng phiếu giảm giá tỷ lệ phần trăm
pub fn apply_down_price(tien_goc: f64, phan_tram_giam: f64) -> f64 {
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
pub fn xu_ly_menh_lenh(list: &[MatQueue]) -> (f64, Vec<String>) {
    let mut tong_doanh_thu: f64 = 0.0;
    let mut list_name: Vec<String> = Vec::new();

    // Vòng lặp thủ công với nhiều bước điều kiện lồng nhau
    for i in 0..list.len() {
        let queue = &list[i];
        // Chỉ xử lý các đơn hàng đã thanh toán và có giá trị trên 50.0
        if queue.is_paid {
            let into_tien = to_money(queue);
            if into_tien >= 50.0 {
                tong_doanh_thu += into_tien;
                list_name.push(queue.name_queue.clone());
            }
        }
    }

    (tong_doanh_thu, list_name)
}

/// CÁCH 2: Phong cách Lập trình Hàm Khai báo (Declarative Pipeline)
/// Dữ liệu chảy qua chuỗi lọc và ánh xạ, không dùng biến mut nào trong quá trình xử lý!
pub fn handle_declaration(list: &[MatQueue]) -> (f64, Vec<String>) {
    // 1. Nhánh tính tổng doanh thu thông qua đường ống (Pipeline)
    let tong_doanh_thu: f64 = list
        .iter()
        .filter(|queue| queue.is_paid)             // Bước 1: Lọc hàng đã trả tiền
        .map(|queue| to_money(queue))             // Bước 2: Chuyển đổi thành tiền
        .filter(|&tien| tien >= 50.0)                  // Bước 3: Chỉ lấy món từ 50k trở lên
        .sum();                                        // Bước 4: Gom tụ tính tổng

    // 2. Nhánh trích xuất danh sách tên mặt hàng
    let list_name: Vec<String> = list
        .iter()
        .filter(|queue| queue.is_paid && to_money(queue) >= 50.0)
        .map(|queue| queue.name_queue.clone())             // Ánh xạ sang chuỗi tên
        .collect();                                    // Gom vào vector mới

    (tong_doanh_thu, list_name)
}

fn main() {
    println!("============================================================");
    println!("  HỆ THỐNG XỬ LÝ HÓA ĐƠN: LẬP TRÌNH MỆNH LỆNH VS ĐƯỜNG ỐNG  ");
    println!("============================================================");

    // Khởi tạo tập dữ liệu ban đầu bất biến
    let gio_hang: Vec<MatQueue> = vec![
        MatQueue {
            ma_san_pham: String::from("SP-01"),
            name_queue: String::from("Sổ tay Lập trình Rust"),
            don_price: 45.0,
            quantity: 2,
            is_paid: true, // Thành tiền = 90.0 (Thỏa mãn >= 50)
        },
        MatQueue {
            ma_san_pham: String::from("SP-02"),
            name_queue: String::from("Bút bi kỹ thuật"),
            don_price: 15.0,
            quantity: 1,
            is_paid: true, // Thành tiền = 15.0 (Bị loại do < 50)
        },
        MatQueue {
            ma_san_pham: String::from("SP-03"),
            name_queue: String::from("Bàn phím cơ không dây"),
            don_price: 120.0,
            quantity: 1,
            is_paid: false, // Chưa thanh toán (Bị loại)
        },
        MatQueue {
            ma_san_pham: String::from("SP-04"),
            name_queue: String::from("Chuột công thái học"),
            don_price: 75.0,
            quantity: 1,
            is_paid: true, // Thành tiền = 75.0 (Thỏa mãn >= 50)
        },
    ];

    println!("Tổng số mặt hàng đưa vào xử lý: {}", gio_hang.len());

    // 1. Chạy theo phong cách mệnh lệnh
    let (doanh_thu_1, ten_1) = xu_ly_menh_lenh(&gio_hang);
    println!("\n[Kết quả Mệnh lệnh]:");
    println!("- Tổng doanh thu đạt chuẩn : {:.2} nghìn đồng", doanh_thu_1);
    println!("- Danh sách mặt hàng hợp lệ: {:?}", ten_1);

    // 2. Chạy theo phong cách khai báo đường ống
    let (doanh_thu_2, ten_2) = handle_declaration(&gio_hang);
    println!("\n[Kết quả Khai báo Đường ống]:");
    println!("- Tổng doanh thu đạt chuẩn : {:.2} nghìn đồng", doanh_thu_2);
    println!("- Danh sách mặt hàng hợp lệ: {:?}", ten_2);

    // Xác thực hai cách tiếp cận cho ra cùng một kết quả nhất quán
    assert_eq!(doanh_thu_1, doanh_thu_2);
    assert_eq!(ten_1, ten_2);

    // Minh họa hàm thuần túy tính chiết khấu khuyến mãi độc lập
    let total_next_down = apply_down_price(doanh_thu_2, 10.0); // Giảm giá 10%
    println!("\n-> Doanh thu sau khi áp dụng phiếu giảm giá 10%: {:.2} nghìn đồng", total_next_down);
    println!("============================================================");
}
