//! TẦNG 3 — KIỂM THỬ TÍCH HỢP (Integration Test).
//! Tệp trong thư mục `tests/` được biên dịch thành MỘT CRATE RIÊNG, chỉ nhìn thấy
//! API CÔNG KHAI của `ch55` — đúng như một người dùng thật. Đây là điểm khác biệt
//! cốt lõi so với unit test (nằm trong lib, thấy được cả hàm riêng tư).

use ch55::{checkout, PaymentGateway, Cart};

/// Cổng giả cấp module test tích hợp (không truy cập được nội bộ crate).
struct CongGia;
impl PaymentGateway for CongGia {
    fn debit(&self, so_tien: u64) -> Result<String, String> {
        Ok(format!("TICH-HOP-{}", so_tien))
    }
}

#[test]
fn luong_mua_hang_hoan_chinh_tu_ben_ngoai() {
    // Dựng giỏ, cộng dồn, giảm giá, thanh toán — toàn bộ qua API công khai
    let mut gio = Cart::new();
    gio.them("Màn hình", 5_000_000, 1).unwrap();
    gio.them("Cáp", 150_000, 2).unwrap();
    gio.them("Màn hình", 5_000_000, 1).unwrap(); // gộp dòng

    assert_eq!(gio.so_dong(), 2);
    assert_eq!(gio.tong_tien(), 10_300_000);

    let id = checkout(&gio, &CongGia, 10).unwrap();
    assert_eq!(id, "TICH-HOP-9270000"); // 10.300.000 - 10%
}

#[test]
fn giu_bat_bien_qua_nhieu_thao_tac() {
    let mut gio = Cart::new();
    for i in 0..20 {
        gio.them(&format!("SP{}", i % 5), 1000, 1).unwrap(); // 5 tên, mỗi tên 4 lần
    }
    assert_eq!(gio.so_dong(), 5, "20 lần thêm 5 tên -> đúng 5 dòng");
    assert_eq!(gio.tong_tien(), 20_000);
}
