//! Chương 55 — Kim tự tháp Kiểm thử: Unit, Integration, E2E, TDD, BDD, Property, Doctest.

// ============================================================================
// PHẦN 1: MIỀN NGHIỆP VỤ ĐƯỢC PHÁT TRIỂN THEO TDD (Red → Green → Refactor)
// ============================================================================

/// Giỏ hàng — ta sẽ "viết test trước, code sau" cho từng hành vi.
#[derive(Debug, Clone, PartialEq)]
pub struct Cart {
    items: Vec<(String, u64, u32)>, // (tên, đơn giá, số lượng)
}

#[derive(Debug, PartialEq, Eq)]
pub enum CartError {
    ZeroQuantity,
    NotFound,
}

impl Default for Cart {
    fn default() -> Self {
        Self::new()
    }
}

impl Cart {
    pub fn new() -> Self {
        Cart { items: Vec::new() }
    }

    /// Thêm mặt hàng. Số lượng 0 là lỗi nghiệp vụ (không phải panic).
    pub fn add(&mut self, name: &str, unit_price: u64, quantity: u32) -> Result<(), CartError> {
        if quantity == 0 {
            return Err(CartError::ZeroQuantity);
        }
        // Nếu đã có, cộng dồn số lượng thay vì tạo dòng mới
        if let Some(line) = self.items.iter_mut().find(|(t, _, _)| t == name) {
            line.2 += quantity;
        } else {
            self.items.push((name.to_string(), unit_price, quantity));
        }
        Ok(())
    }

    /// Tổng tiền, tính bằng đơn vị nhỏ nhất (đồng) — KHÔNG dùng f64.
    ///
    /// # Ví dụ (đây cũng là một DOCTEST — chạy khi `cargo test`)
    /// ```
    /// # use ch55::Cart;
    /// let mut cart = Cart::new();
    /// cart.add("Sách", 45_000, 2).unwrap();
    /// cart.add("Bút", 5_000, 3).unwrap();
    /// assert_eq!(cart.total(), 105_000);
    /// ```
    pub fn total(&self) -> u64 {
        self.items
            .iter()
            .map(|(_, price, qty)| price * *qty as u64)
            .sum()
    }

    pub fn line_count(&self) -> usize {
        self.items.len()
    }

    /// Áp mã giảm giá phần trăm (vượt 100 thì ghim ở 100).
    pub fn after_discount(&self, percent: u32) -> u64 {
        let total = self.total();
        let pct = percent.min(100) as u64;
        total - total * pct / 100
    }
}

// ============================================================================
// PHẦN 2: TEST DOUBLE (MOCK/FAKE) BẰNG TRAIT — không cần thư viện ngoài
// ============================================================================

/// Cổng thanh toán là một PHỤ THUỘC. Trong test ta thay nó bằng bản giả.
pub trait PaymentGateway {
    fn debit(&self, amount: u64) -> Result<String, String>;
}

/// Bản thật (chỉ mô phỏng, không gọi mạng thật ở đây).
pub struct RealGateway;
impl PaymentGateway for RealGateway {
    fn debit(&self, amount: u64) -> Result<String, String> {
        Ok(format!("TXN-REAL-{}", amount))
    }
}

/// Hàm nghiệp vụ nhận phụ thuộc qua trait (tiêm phụ thuộc, Chương 14).
pub fn checkout(
    cart: &Cart,
    gateway: &dyn PaymentGateway,
    discount: u32,
) -> Result<String, String> {
    let amount = cart.after_discount(discount);
    if amount == 0 {
        return Err("Giỏ rỗng hoặc miễn phí, không cần thanh toán".to_string());
    }
    gateway.debit(amount)
}

// ============================================================================
// PHẦN 3: BỘ SINH DỮ LIỆU CHO KIỂM THỬ THEO TÍNH CHẤT (PROPERTY-BASED)
// ============================================================================

/// Bộ rng giả ngẫu nhiên tất định (LCG) — giống Chương 18, không cần crate.
pub struct Generator(u64);
impl Generator {
    pub fn new(seed: u64) -> Self {
        Generator(seed)
    }
    pub fn below(&mut self, bound: u32) -> u32 {
        self.0 = self
            .0
            .wrapping_mul(6364136223846793005)
            .wrapping_add(1442695040888963407);
        ((self.0 >> 33) as u32) % bound
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
        let cart = Cart::new();
        assert_eq!(cart.line_count(), 0);
        assert_eq!(cart.total(), 0);
    }

    #[test]
    fn add_item_totals_correctly() {
        let mut cart = Cart::new();
        cart.add("A", 10_000, 3).unwrap();
        assert_eq!(cart.total(), 30_000);
    }

    #[test]
    fn same_name_merges_quantity() {
        let mut cart = Cart::new();
        cart.add("A", 10_000, 1).unwrap();
        cart.add("A", 10_000, 2).unwrap();
        assert_eq!(cart.line_count(), 1, "phải gộp thành 1 dòng");
        assert_eq!(cart.total(), 30_000);
    }

    #[test]
    fn zero_quantity_is_error_not_panic() {
        let mut cart = Cart::new();
        assert_eq!(cart.add("A", 10_000, 0), Err(CartError::ZeroQuantity));
        assert_eq!(cart.line_count(), 0); // không thêm gì
    }

    #[test]
    fn discount_clamped_at_100() {
        let mut cart = Cart::new();
        cart.add("A", 100_000, 1).unwrap();
        assert_eq!(cart.after_discount(200), 0); // ghim ở 100%, không âm
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
        fn debit(&self, amount: u64) -> Result<String, String> {
            self.called_with.borrow_mut().push(amount);
            Ok("TXN-FAKE".to_string())
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
        let mut cart = Cart::new();
        cart.add("A", 100_000, 1).unwrap();
        let spy = SpyGateway {
            called_with: RefCell::new(vec![]),
        };

        checkout(&cart, &spy, 20).unwrap(); // giảm 20% -> 80.000

        assert_eq!(
            *spy.called_with.borrow(),
            vec![80_000],
            "phải trừ đúng số sau giảm giá"
        );
    }

    #[test]
    fn checkout_propagates_gateway_error() {
        let mut cart = Cart::new();
        cart.add("A", 100_000, 1).unwrap();
        assert_eq!(
            checkout(&cart, &AlwaysFailGateway, 0),
            Err("Thẻ bị từ chối".to_string())
        );
    }

    #[test]
    fn empty_cart_skips_gateway() {
        let cart = Cart::new();
        let spy = SpyGateway {
            called_with: RefCell::new(vec![]),
        };
        let result = checkout(&cart, &spy, 0);
        assert!(result.is_err());
        assert!(
            spy.called_with.borrow().is_empty(),
            "cổng KHÔNG được gọi khi giỏ rỗng"
        );
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
        let mut rng = Generator::new(2026);
        for _ in 0..2000 {
            let mut cart = Cart::new();
            let item_count = rng.below(5) + 1;
            for i in 0..item_count {
                let _ = cart.add(
                    &format!("SP{}", i),
                    (rng.below(100_000) + 1) as u64,
                    rng.below(5) + 1,
                );
            }
            let pct = rng.below(150); // cố tình cho vượt 100
            let discounted = cart.after_discount(pct);
            // TÍNH CHẤT: giá sau giảm luôn trong [0, tổng]
            assert!(
                discounted <= cart.total(),
                "giảm giá không được làm TĂNG tiền"
            );
        }
    }

    #[test]
    fn zero_discount_keeps_total() {
        let mut rng = Generator::new(7);
        for _ in 0..1000 {
            let mut cart = Cart::new();
            cart.add("X", (rng.below(50_000) + 1) as u64, rng.below(9) + 1)
                .unwrap();
            // TÍNH CHẤT: giảm 0% là phép đồng nhất
            assert_eq!(cart.after_discount(0), cart.total());
        }
    }

    #[test]
    fn sum_equals_parts() {
        let mut rng = Generator::new(99);
        for _ in 0..1000 {
            let (p1, q1) = ((rng.below(1000) + 1) as u64, rng.below(9) + 1);
            let (p2, q2) = ((rng.below(1000) + 1) as u64, rng.below(9) + 1);
            let mut cart = Cart::new();
            cart.add("A", p1, q1).unwrap();
            cart.add("B", p2, q2).unwrap();
            // TÍNH CHẤT: tổng = tổng thành tiền từng dòng
            assert_eq!(cart.total(), p1 * q1 as u64 + p2 * q2 as u64);
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
        fn debit(&self, s: u64) -> Result<String, String> {
            self.0.borrow_mut().push(s);
            Ok("OK".into())
        }
    }

    /// Kịch bản: "Khách VIP mua hàng và được giảm 15%".
    #[test]
    fn vip_gets_15_percent_off() {
        // GIVEN — một giỏ hàng trị giá 1.000.000đ và một cổng thanh toán
        let mut cart = Cart::new();
        cart.add("Tai nghe", 1_000_000, 1).unwrap();
        let gateway = OkGateway(RefCell::new(vec![]));

        // WHEN — khách VIP (giảm 15%) thanh toán
        let result = checkout(&cart, &gateway, 15);

        // THEN — thanh toán thành công và số tiền bị trừ đúng 850.000đ
        assert!(result.is_ok());
        assert_eq!(*gateway.0.borrow(), vec![850_000]);
    }

    /// Kịch bản: "Không thể thanh toán một giỏ hàng rỗng".
    #[test]
    fn cannot_checkout_empty_cart() {
        // GIVEN — một giỏ hàng rỗng
        let cart = Cart::new();
        let gateway = OkGateway(RefCell::new(vec![]));

        // WHEN — cố gắng thanh toán
        let result = checkout(&cart, &gateway, 0);

        // THEN — hệ thống từ chối và không gọi cổng thanh toán
        assert!(result.is_err());
        assert!(gateway.0.borrow().is_empty());
    }
}
