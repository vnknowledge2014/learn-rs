#![allow(dead_code, unused_variables)]
//! Chương 55 — Kim tự tháp Kiểm thử: Unit, Integration, E2E, TDD, BDD, Property, Doctest.

// ============================================================================
// PHẦN 1: MIỀN NGHIỆP VỤ ĐƯỢC PHÁT TRIỂN THEO TDD (Red → Green → Refactor)
// ============================================================================

/// Giỏ hàng — ta sẽ "viết test trước, code sau" cho từng hành vi.
#[derive(Debug, Clone, PartialEq)]
pub struct Cart {
    mat_queue: Vec<(String, u64, u32)>, // (tên, đơn giá, số lượng)
}

#[derive(Debug, PartialEq, Eq)]
pub enum CartError {
    SoLuongBangKhong,
    KhongTonTai,
}

impl Cart {
    pub fn new() -> Self {
        Cart { mat_queue: Vec::new() }
    }

    /// Thêm mặt hàng. Số lượng 0 là lỗi nghiệp vụ (không phải panic).
    pub fn them(&mut self, name: &str, don_price: u64, quantity: u32) -> Result<(), CartError> {
        if quantity == 0 {
            return Err(CartError::SoLuongBangKhong);
        }
        // Nếu đã có, cộng dồn số lượng thay vì tạo dòng mới
        if let Some(dong) = self.mat_queue.iter_mut().find(|(t, _, _)| t == name) {
            dong.2 += quantity;
        } else {
            self.mat_queue.push((name.to_string(), don_price, quantity));
        }
        Ok(())
    }

    /// Tổng tiền, tính bằng đơn vị nhỏ nhất (đồng) — KHÔNG dùng f64.
    ///
    /// # Ví dụ (đây cũng là một DOCTEST — chạy khi `cargo test`)
    /// ```
    /// # use ch55::Cart;
    /// let mut gio = Cart::new();
    /// gio.them("Sách", 45_000, 2).unwrap();
    /// gio.them("Bút", 5_000, 3).unwrap();
    /// assert_eq!(gio.tong_tien(), 105_000);
    /// ```
    pub fn tong_tien(&self) -> u64 {
        self.mat_queue.iter().map(|(_, price, sl)| price * *sl as u64).sum()
    }

    pub fn so_dong(&self) -> usize {
        self.mat_queue.len()
    }

    /// Áp mã giảm giá phần trăm (0..=100).
    pub fn after_discount(&self, percent: u32) -> u64 {
        let tong = self.tong_tien();
        let pt = percent.min(100) as u64;
        tong - tong * pt / 100
    }
}

// ============================================================================
// PHẦN 2: TEST DOUBLE (MOCK/FAKE) BẰNG TRAIT — không cần thư viện ngoài
// ============================================================================

/// Cổng thanh toán là một PHỤ THUỘC. Trong test ta thay nó bằng bản giả.
pub trait PaymentGateway {
    fn debit(&self, so_tien: u64) -> Result<String, String>;
}

/// Bản thật (chỉ mô phỏng, không gọi mạng thật ở đây).
pub struct RealGateway;
impl PaymentGateway for RealGateway {
    fn debit(&self, so_tien: u64) -> Result<String, String> {
        Ok(format!("TXN-THAT-{}", so_tien))
    }
}

/// Hàm nghiệp vụ nhận phụ thuộc qua trait (tiêm phụ thuộc, Chương 14).
pub fn checkout(
    gio: &Cart,
    gate: &dyn PaymentGateway,
    discount: u32,
) -> Result<String, String> {
    let so_tien = gio.after_discount(discount);
    if so_tien == 0 {
        return Err("Giỏ rỗng hoặc miễn phí, không cần thanh toán".to_string());
    }
    gate.debit(so_tien)
}

// ============================================================================
// PHẦN 3: MÁY TRẠNG THÁI ĐỂ DEMO KIỂM THỬ THEO TÍNH CHẤT (PROPERTY-BASED)
// ============================================================================

/// Bộ sinh giả ngẫu nhiên tất định (LCG) — giống Chương 18, không cần crate.
pub struct Generator(u64);
impl Generator {
    pub fn new(hat: u64) -> Self { Generator(hat) }
    pub fn so(&mut self, tran: u32) -> u32 {
        self.0 = self.0.wrapping_mul(6364136223846793005).wrapping_add(1442695040888963407);
        ((self.0 >> 33) as u32) % tran
    }
}


// ============================================================================
// TẦNG 1 — UNIT TESTS: nhanh, nhiều, kiểm một đơn vị biệt lập
// ============================================================================

#[cfg(test)]
mod unit {
    use super::*;

    // --- Phong cách TDD: mỗi test mô tả MỘT hành vi mong muốn ---

    #[test]
    fn new_cart_is_empty() {
        let gio = Cart::new();
        assert_eq!(gio.so_dong(), 0);
        assert_eq!(gio.tong_tien(), 0);
    }

    #[test]
    fn add_item_totals_correctly() {
        let mut gio = Cart::new();
        gio.them("A", 10_000, 3).unwrap();
        assert_eq!(gio.tong_tien(), 30_000);
    }

    #[test]
    fn same_name_merges_quantity() {
        let mut gio = Cart::new();
        gio.them("A", 10_000, 1).unwrap();
        gio.them("A", 10_000, 2).unwrap();
        assert_eq!(gio.so_dong(), 1, "phải gộp thành 1 dòng");
        assert_eq!(gio.tong_tien(), 30_000);
    }

    #[test]
    fn zero_quantity_is_error_not_panic() {
        let mut gio = Cart::new();
        assert_eq!(gio.them("A", 10_000, 0), Err(CartError::SoLuongBangKhong));
        assert_eq!(gio.so_dong(), 0); // không thêm gì
    }

    #[test]
    fn discount_clamped_at_100() {
        let mut gio = Cart::new();
        gio.them("A", 100_000, 1).unwrap();
        assert_eq!(gio.after_discount(200), 0); // ghim ở 100%, không âm
    }
}

// ============================================================================
// TẦNG 2 — TEST VỚI TEST DOUBLE (MOCK): thay phụ thuộc bằng bản giả
// ============================================================================

#[cfg(test)]
mod test_double {
    use super::*;
    use std::cell::RefCell;

    /// SPY: cổng giả ghi lại nó được gọi với số tiền bao nhiêu.
    struct SpyGateway {
        called_with: RefCell<Vec<u64>>,
    }
    impl PaymentGateway for SpyGateway {
        fn debit(&self, so_tien: u64) -> Result<String, String> {
            self.called_with.borrow_mut().push(so_tien);
            Ok("TXN-GIA".to_string())
        }
    }

    /// STUB: cổng giả luôn báo lỗi, để test nhánh thất bại.
    struct AlwaysFailGateway;
    impl PaymentGateway for AlwaysFailGateway {
        fn debit(&self, _: u64) -> Result<String, String> {
            Err("Thẻ bị từ chối".to_string())
        }
    }

    #[test]
    fn checkout_charges_discounted_total() {
        let mut gio = Cart::new();
        gio.them("A", 100_000, 1).unwrap();
        let spy = SpyGateway { called_with: RefCell::new(vec![]) };

        checkout(&gio, &spy, 20).unwrap(); // giảm 20% -> 80.000

        assert_eq!(*spy.called_with.borrow(), vec![80_000], "phải trừ đúng số sau giảm giá");
    }

    #[test]
    fn checkout_propagates_gateway_error() {
        let mut gio = Cart::new();
        gio.them("A", 100_000, 1).unwrap();
        assert_eq!(checkout(&gio, &AlwaysFailGateway, 0), Err("Thẻ bị từ chối".to_string()));
    }

    #[test]
    fn empty_cart_skips_gateway() {
        let gio = Cart::new();
        let spy = SpyGateway { called_with: RefCell::new(vec![]) };
        let kq = checkout(&gio, &spy, 0);
        assert!(kq.is_err());
        assert!(spy.called_with.borrow().is_empty(), "cổng KHÔNG được gọi khi giỏ rỗng");
    }
}

// ============================================================================
// TẦNG 3 — KIỂM THỬ THEO TÍNH CHẤT (PROPERTY-BASED)
// Không kiểm một ví dụ, mà kiểm một ĐẲNG THỨC đúng với mọi đầu vào.
// ============================================================================

#[cfg(test)]
mod property {
    use super::*;

    #[test]
    fn discount_within_bounds() {
        let mut sinh = Generator::new(2026);
        for _ in 0..2000 {
            let mut gio = Cart::new();
            let item_count = sinh.so(5) + 1;
            for i in 0..item_count {
                let _ = gio.them(&format!("SP{}", i), (sinh.so(100_000) + 1) as u64, sinh.so(5) + 1);
            }
            let pt = sinh.so(150); // cố tình cho vượt 100
            let next = gio.after_discount(pt);
            // TÍNH CHẤT: giá sau giảm luôn trong [0, tổng]
            assert!(next <= gio.tong_tien(), "giảm giá không được làm TĂNG tiền");
        }
    }

    #[test]
    fn zero_discount_keeps_total() {
        let mut sinh = Generator::new(7);
        for _ in 0..1000 {
            let mut gio = Cart::new();
            gio.them("X", (sinh.so(50_000) + 1) as u64, sinh.so(9) + 1).unwrap();
            // TÍNH CHẤT: giảm 0% là phép đồng nhất
            assert_eq!(gio.after_discount(0), gio.tong_tien());
        }
    }

    #[test]
    fn sum_equals_parts() {
        let mut sinh = Generator::new(99);
        for _ in 0..1000 {
            let (g1, sl1) = ((sinh.so(1000) + 1) as u64, sinh.so(9) + 1);
            let (g2, sl2) = ((sinh.so(1000) + 1) as u64, sinh.so(9) + 1);
            let mut gio = Cart::new();
            gio.them("A", g1, sl1).unwrap();
            gio.them("B", g2, sl2).unwrap();
            // TÍNH CHẤT: tổng = tổng thành tiền từng dòng
            assert_eq!(gio.tong_tien(), g1 * sl1 as u64 + g2 * sl2 as u64);
        }
    }
}

// ============================================================================
// TẦNG 4 — BDD: cấu trúc GIVEN / WHEN / THEN (Behaviour-Driven Development)
// Không cần cucumber-rs: chỉ cần đặt tên và bố cục test theo ngôn ngữ nghiệp vụ.
// ============================================================================

#[cfg(test)]
mod bdd {
    use super::*;
    use std::cell::RefCell;

    struct OkGateway(RefCell<Vec<u64>>);
    impl PaymentGateway for OkGateway {
        fn debit(&self, s: u64) -> Result<String, String> { self.0.borrow_mut().push(s); Ok("OK".into()) }
    }

    /// Kịch bản: "Khách VIP mua hàng và được giảm 15%".
    #[test]
    fn vip_gets_15_percent_off() {
        // GIVEN — một giỏ hàng trị giá 1.000.000đ và một cổng thanh toán
        let mut gio = Cart::new();
        gio.them("Tai nghe", 1_000_000, 1).unwrap();
        let gate = OkGateway(RefCell::new(vec![]));

        // WHEN — khách VIP (giảm 15%) thanh toán
        let ket_qua = checkout(&gio, &gate, 15);

        // THEN — thanh toán thành công và số tiền bị trừ đúng 850.000đ
        assert!(ket_qua.is_ok());
        assert_eq!(*gate.0.borrow(), vec![850_000]);
    }

    /// Kịch bản: "Không thể thanh toán một giỏ hàng rỗng".
    #[test]
    fn cannot_checkout_empty_cart() {
        // GIVEN — một giỏ hàng rỗng
        let gio = Cart::new();
        let gate = OkGateway(RefCell::new(vec![]));

        // WHEN — cố gắng thanh toán
        let ket_qua = checkout(&gio, &gate, 0);

        // THEN — hệ thống từ chối và không gọi cổng thanh toán
        assert!(ket_qua.is_err());
        assert!(gate.0.borrow().is_empty());
    }
}
