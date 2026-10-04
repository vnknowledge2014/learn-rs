// Tệp: src/main.rs
// Chương trình minh họa tư duy Lập trình hàm và Xây dựng Đường ống (Data Pipelines) trong Rust
// (Tiền tính bằng u64 — đơn vị đồng — chứ không dùng f64, theo cảnh báo ở Chương 03.)

#[derive(Debug, Clone, PartialEq)]
pub struct Item {
    pub sku: String,
    pub product_name: String,
    pub unit_price: u64,
    pub quantity: u32,
    pub is_paid: bool,
}

// Ngưỡng giá trị tối thiểu của một dòng hàng để được tính vào báo cáo
const MIN_LINE_TOTAL: u64 = 50_000;

// ============================================================================
// HÀM THUẦN TÚY (PURE FUNCTIONS) - KHÔNG TÁC DỤNG PHỤ
// ============================================================================

/// Hàm thuần túy: Tính thành tiền của một mặt hàng
/// Nhận dữ liệu đầu vào và trả về giá trị mới, không thay đổi bất kỳ trạng thái nào
pub fn subtotal(item: &Item) -> u64 {
    item.unit_price * item.quantity as u64
}

/// Hàm thuần túy: Áp dụng phiếu giảm giá theo tỷ lệ phần trăm (0..=100)
pub fn apply_discount(base_price: u64, discount_percent: u64) -> u64 {
    if discount_percent >= 100 {
        0
    } else {
        base_price * (100 - discount_percent) / 100
    }
}

// ============================================================================
// SO SÁNH HAI CÁCH TIẾP CẬN TRÊN DỮ LIỆU
// ============================================================================

/// CÁCH 1: Phong cách Mệnh lệnh (Imperative)
/// Dùng vòng lặp thủ công, biến cờ mut tạm thời, dễ xảy ra lỗi ngoài ý muốn
// Vòng lặp theo chỉ số được giữ CỐ Ý để làm đối chứng với cách 2,
// nên tắt lint `needless_range_loop` của Clippy cho riêng hàm này.
#[allow(clippy::needless_range_loop)]
pub fn process_imperative(list: &[Item]) -> (u64, Vec<String>) {
    let mut total_revenue: u64 = 0;
    let mut names: Vec<String> = Vec::new();

    // Vòng lặp thủ công với nhiều bước điều kiện lồng nhau
    for i in 0..list.len() {
        let item = &list[i];
        // Chỉ xử lý các mặt hàng đã thanh toán và có thành tiền từ 50.000đ trở lên
        if item.is_paid {
            let line_total = subtotal(item);
            if line_total >= MIN_LINE_TOTAL {
                total_revenue += line_total;
                names.push(item.product_name.clone());
            }
        }
    }

    (total_revenue, names)
}

/// CÁCH 2: Phong cách Lập trình Hàm Khai báo (Declarative Pipeline)
/// Dữ liệu chảy qua chuỗi lọc và ánh xạ, không dùng biến mut nào trong quá trình xử lý!
pub fn process_declarative(list: &[Item]) -> (u64, Vec<String>) {
    // 1. Nhánh tính tổng doanh thu thông qua đường ống (Pipeline)
    let total_revenue: u64 = list
        .iter()
        .filter(|item| item.is_paid) // Bước 1: Lọc hàng đã trả tiền
        .map(subtotal) // Bước 2: Chuyển đổi thành tiền
        .filter(|&amount| amount >= MIN_LINE_TOTAL) // Bước 3: Chỉ lấy món từ 50k trở lên
        .sum(); // Bước 4: Gom tụ tính tổng

    // 2. Nhánh trích xuất danh sách tên mặt hàng
    let names: Vec<String> = list
        .iter()
        .filter(|item| item.is_paid && subtotal(item) >= MIN_LINE_TOTAL)
        .map(|item| item.product_name.clone()) // Ánh xạ sang chuỗi tên
        .collect(); // Gom vào vector mới

    (total_revenue, names)
}

fn sample_cart() -> Vec<Item> {
    vec![
        Item {
            sku: String::from("SP-01"),
            product_name: String::from("Sổ tay Lập trình Rust"),
            unit_price: 45_000,
            quantity: 2,
            is_paid: true, // Thành tiền = 90.000 (Thỏa mãn >= 50.000)
        },
        Item {
            sku: String::from("SP-02"),
            product_name: String::from("Bút bi kỹ thuật"),
            unit_price: 15_000,
            quantity: 1,
            is_paid: true, // Thành tiền = 15.000 (Bị loại do < 50.000)
        },
        Item {
            sku: String::from("SP-03"),
            product_name: String::from("Bàn phím cơ không dây"),
            unit_price: 120_000,
            quantity: 1,
            is_paid: false, // Chưa thanh toán (Bị loại)
        },
        Item {
            sku: String::from("SP-04"),
            product_name: String::from("Chuột công thái học"),
            unit_price: 75_000,
            quantity: 1,
            is_paid: true, // Thành tiền = 75.000 (Thỏa mãn >= 50.000)
        },
    ]
}

fn main() {
    println!("============================================================");
    println!("  HỆ THỐNG XỬ LÝ HÓA ĐƠN: LẬP TRÌNH MỆNH LỆNH VS ĐƯỜNG ỐNG  ");
    println!("============================================================");

    // Khởi tạo tập dữ liệu ban đầu bất biến
    let cart: Vec<Item> = sample_cart();

    println!("Tổng số mặt hàng đưa vào xử lý: {}", cart.len());

    // 1. Chạy theo phong cách mệnh lệnh
    let (revenue_1, names_1) = process_imperative(&cart);
    println!("\n[Kết quả Mệnh lệnh]:");
    println!("- Tổng doanh thu đạt chuẩn : {} đồng", revenue_1);
    println!("- Danh sách mặt hàng hợp lệ: {:?}", names_1);

    // 2. Chạy theo phong cách khai báo đường ống
    let (revenue_2, names_2) = process_declarative(&cart);
    println!("\n[Kết quả Khai báo Đường ống]:");
    println!("- Tổng doanh thu đạt chuẩn : {} đồng", revenue_2);
    println!("- Danh sách mặt hàng hợp lệ: {:?}", names_2);

    // Xác thực hai cách tiếp cận cho ra cùng một kết quả nhất quán
    assert_eq!(revenue_1, revenue_2);
    assert_eq!(names_1, names_2);

    // Minh họa hàm thuần túy tính chiết khấu khuyến mãi độc lập
    let discounted_total = apply_discount(revenue_2, 10); // Giảm giá 10%
    println!(
        "\n-> Doanh thu sau khi áp dụng phiếu giảm giá 10%: {} đồng",
        discounted_total
    );
    println!("============================================================");
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn both_styles_agree() {
        let cart = sample_cart();
        let expected = (
            165_000,
            vec![
                String::from("Sổ tay Lập trình Rust"),
                String::from("Chuột công thái học"),
            ],
        );
        assert_eq!(process_imperative(&cart), expected);
        assert_eq!(process_declarative(&cart), expected);
    }

    #[test]
    fn discount_is_exact_and_clamped() {
        assert_eq!(apply_discount(165_000, 10), 148_500);
        assert_eq!(apply_discount(165_000, 0), 165_000);
        assert_eq!(apply_discount(165_000, 150), 0);
    }
}
