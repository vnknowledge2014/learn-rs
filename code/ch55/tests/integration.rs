//! KIỂM THỬ TÍCH HỢP (Integration Test).
//! Tệp trong thư mục `tests/` được biên dịch thành MỘT CRATE RIÊNG, chỉ nhìn thấy
//! API CÔNG KHAI của `ch55` — đúng như một người dùng thật. Đây là điểm khác biệt
//! cốt lõi so với unit test (nằm trong lib, thấy được cả hàm riêng tư).

use ch55::{Cart, PaymentGateway, checkout};

/// Cổng giả cấp module test tích hợp (không truy cập được nội bộ crate).
struct FakeGateway;
impl PaymentGateway for FakeGateway {
    fn debit(&self, amount: u64) -> Result<String, String> {
        Ok(format!("INTEGRATION-{}", amount))
    }
}

#[test]
fn full_purchase_flow_via_public_api() {
    // Dựng giỏ, cộng dồn, giảm giá, thanh toán — toàn bộ qua API công khai
    let mut cart = Cart::new();
    cart.add("Màn hình", 5_000_000, 1).unwrap();
    cart.add("Cáp", 150_000, 2).unwrap();
    cart.add("Màn hình", 5_000_000, 1).unwrap(); // gộp dòng

    assert_eq!(cart.line_count(), 2);
    assert_eq!(cart.total(), 10_300_000);

    let id = checkout(&cart, &FakeGateway, 10).unwrap();
    assert_eq!(id, "INTEGRATION-9270000"); // 10.300.000 - 10%
}

#[test]
fn invariant_holds_across_operations() {
    let mut cart = Cart::new();
    for i in 0..20 {
        cart.add(&format!("SP{}", i % 5), 1000, 1).unwrap(); // 5 tên, mỗi tên 4 lần
    }
    assert_eq!(cart.line_count(), 5, "20 lần thêm 5 tên -> đúng 5 dòng");
    assert_eq!(cart.total(), 20_000);
}
