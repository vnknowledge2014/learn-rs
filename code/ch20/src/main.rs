// Tệp: src/main.rs
// Chương trình thực chiến: Kiểu bọc, Hàm khởi tạo có kiểm chứng và Typestate

use std::convert::TryFrom;
use std::marker::PhantomData;

// ============================================================================
// PHẦN 1: MÔ-ĐUN MIỀN NGHIỆP VỤ
// Đặt trong `mod` để tính RIÊNG TƯ của các trường thực sự có hiệu lực.
// ============================================================================
pub mod domain {
    use std::fmt;

    #[derive(Debug, Clone, PartialEq, Eq)]
    pub enum DomainError {
        InvalidEmail(String),
        InvalidProductName(String),
        InvalidQuantity(String),
        EmptyOrder,
        OrderTooLarge { line_count: usize, max: usize },
    }

    impl fmt::Display for DomainError {
        fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
            match self {
                DomainError::InvalidEmail(s) => write!(f, "Email không hợp lệ: {}", s),
                DomainError::InvalidProductName(s) => write!(f, "Tên sản phẩm không hợp lệ: {}", s),
                DomainError::InvalidQuantity(s) => write!(f, "Số lượng không hợp lệ: {}", s),
                DomainError::EmptyOrder => write!(f, "Đơn hàng phải có ít nhất 1 dòng hàng"),
                DomainError::OrderTooLarge { line_count, max } => {
                    write!(f, "Đơn có {} dòng, vượt giới hạn {} dòng", line_count, max)
                }
            }
        }
    }

    // ---------------------------------------------------------------------
    // KIỂU BỌC 1: Email — trường riêng tư, chỉ tạo được qua `parse`
    // ---------------------------------------------------------------------
    #[derive(Debug, Clone, PartialEq, Eq)]
    pub struct Email(String); // KHÔNG có `pub` trước String → đây là con dấu

    impl Email {
        pub fn parse(raw: &str) -> Result<Self, DomainError> {
            let s = raw.trim().to_lowercase();
            if s.is_empty() {
                return Err(DomainError::InvalidEmail("chuỗi rỗng".to_string()));
            }
            let parts: Vec<&str> = s.split('@').collect();
            if parts.len() != 2 || parts[0].is_empty() || !parts[1].contains('.') {
                return Err(DomainError::InvalidEmail(s));
            }
            Ok(Email(s))
        }
        pub fn as_str(&self) -> &str {
            &self.0
        }
    }

    impl fmt::Display for Email {
        fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
            write!(f, "{}", self.0)
        }
    }

    // ---------------------------------------------------------------------
    // KIỂU BỌC 2: ProductName — chuỗi có giới hạn độ dài
    // ---------------------------------------------------------------------
    #[derive(Debug, Clone, PartialEq, Eq)]
    pub struct ProductName(String);

    impl ProductName {
        pub const MAX: usize = 50;

        pub fn parse(raw: &str) -> Result<Self, DomainError> {
            let s = raw.trim();
            let char_count = s.chars().count(); // đếm CHỮ CÁI, không đếm byte (Chương 05)
            if char_count == 0 {
                Err(DomainError::InvalidProductName("chuỗi rỗng".to_string()))
            } else if char_count > Self::MAX {
                Err(DomainError::InvalidProductName(format!(
                    "dài {} ký tự, tối đa {}",
                    char_count,
                    Self::MAX
                )))
            } else {
                Ok(ProductName(s.to_string()))
            }
        }
        pub fn as_str(&self) -> &str {
            &self.0
        }
    }

    // ---------------------------------------------------------------------
    // KIỂU BỌC 3: Quantity — số nguyên dương trong khoảng cho phép
    // ---------------------------------------------------------------------
    #[derive(Debug, Clone, Copy, PartialEq, Eq)]
    pub struct Quantity(u32);

    impl Quantity {
        pub const MAX: u32 = 1000;

        pub fn parse(n: u32) -> Result<Self, DomainError> {
            if n == 0 {
                Err(DomainError::InvalidQuantity("phải lớn hơn 0".to_string()))
            } else if n > Self::MAX {
                Err(DomainError::InvalidQuantity(format!(
                    "{} vượt quá {}",
                    n,
                    Self::MAX
                )))
            } else {
                Ok(Quantity(n))
            }
        }
        pub fn value(&self) -> u32 {
            self.0
        }
    }

    // ---------------------------------------------------------------------
    // KIỂU BỌC 4: Money — tính bằng ĐƠN VỊ NHỎ NHẤT (đồng), dùng u64.
    // KHÔNG BAO GIỜ dùng f64 cho tiền tệ (xem cảnh báo ở Chương 03)!
    // ---------------------------------------------------------------------
    #[derive(Debug, Clone, Copy, PartialEq, Eq, PartialOrd, Ord)]
    pub struct Money(u64);

    impl Money {
        pub fn vnd(n: u64) -> Self {
            Money(n)
        }
        pub fn value(&self) -> u64 {
            self.0
        }
        pub fn plus(self, other: Money) -> Money {
            Money(self.0 + other.0) // đây là một VỊ NHÓM (Chương 18)!
        }
        pub fn subtract(self, other: Money) -> Money {
            Money(self.0.saturating_sub(other.0))
        }
        pub fn times(self, factor: u32) -> Money {
            Money(self.0 * factor as u64)
        }
    }

    impl fmt::Display for Money {
        fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
            write!(f, "{} đ", self.0)
        }
    }

    // ---------------------------------------------------------------------
    // KIỂU TỔNG: cách thanh toán — KHÔNG CÒN tổ hợp vô nghĩa
    // ---------------------------------------------------------------------
    #[derive(Debug, Clone, PartialEq, Eq)]
    pub enum PaymentMethod {
        Cash,
        Transfer { transaction_id: String },
        Card { last_four: String },
    }

    // ---------------------------------------------------------------------
    // Dòng hàng: một kiểu TÍCH gồm toàn kiểu đã được công chứng
    // ---------------------------------------------------------------------
    #[derive(Debug, Clone, PartialEq, Eq)]
    pub struct OrderLine {
        pub name: ProductName,
        pub quantity: Quantity,
        pub unit_price: Money,
    }

    impl OrderLine {
        pub fn line_total(&self) -> Money {
            self.unit_price.times(self.quantity.value())
        }
    }
}

use domain::*;

// ============================================================================
// PHẦN 2: TYPESTATE — MÁY TRẠNG THÁI ĐƯỢC MÃ HÓA VÀO KIỂU
// ============================================================================

/// Bốn "thẻ đánh dấu" trạng thái. Chúng chiếm 0 byte và biến mất khi biên dịch.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct Draft;
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct Validated;
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct Paid;
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct Delivered;

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct Order<TT> {
    id: String,
    customer: Email,
    lines: Vec<OrderLine>,
    payment: Option<PaymentMethod>,
    _state: PhantomData<TT>,
}

/// Các phương thức dùng chung cho MỌI trạng thái.
impl<TT> Order<TT> {
    pub fn id(&self) -> &str {
        &self.id
    }
    pub fn customer(&self) -> &Email {
        &self.customer
    }
    pub fn line_count(&self) -> usize {
        self.lines.len()
    }
    /// Tổng tiền = gộp các thành tiền bằng phép cộng của vị nhóm Money.
    pub fn total(&self) -> Money {
        self.lines
            .iter()
            .map(|d| d.line_total())
            .fold(Money::vnd(0), |a, b| a.plus(b))
    }
}

pub const MAX_LINES: usize = 20;

/// Trạng thái NHẬP: chỉ có đúng một hành động hợp lệ — xác thực.
impl Order<Draft> {
    pub fn new(id: &str, customer: Email, lines: Vec<OrderLine>) -> Self {
        Order {
            id: id.to_string(),
            customer,
            lines,
            payment: None,
            _state: PhantomData,
        }
    }

    pub fn validate(self) -> Result<Order<Validated>, DomainError> {
        if self.lines.is_empty() {
            return Err(DomainError::EmptyOrder);
        }
        if self.lines.len() > MAX_LINES {
            return Err(DomainError::OrderTooLarge {
                line_count: self.lines.len(),
                max: MAX_LINES,
            });
        }
        Ok(Order {
            id: self.id,
            customer: self.customer,
            lines: self.lines,
            payment: None,
            _state: PhantomData,
        })
    }
}

/// Trạng thái ĐÃ XÁC THỰC: chỉ có thể thanh toán.
impl Order<Validated> {
    pub fn pay(self, method: PaymentMethod) -> Order<Paid> {
        Order {
            id: self.id,
            customer: self.customer,
            lines: self.lines,
            payment: Some(method),
            _state: PhantomData,
        }
    }
}

/// Trạng thái ĐÃ THANH TOÁN: chỉ có thể giao hàng.
impl Order<Paid> {
    pub fn payment_method(&self) -> &PaymentMethod {
        // An toàn tuyệt đối: chỉ trạng thái này mới tồn tại, và nó LUÔN có thanh toán.
        self.payment
            .as_ref()
            .expect("bất biến của Order<Paid>: luôn có thông tin thanh toán")
    }

    pub fn ship(self, tracking_code: &str) -> Order<Delivered> {
        println!(
            "   [VỎ MỆNH LỆNH] Gửi email tới {} về vận đơn {}",
            self.customer, tracking_code
        );
        Order {
            id: self.id,
            customer: self.customer,
            lines: self.lines,
            payment: self.payment,
            _state: PhantomData,
        }
    }
}

// ============================================================================
// PHẦN 3: BIÊN HỆ THỐNG — DTO VÀ CỔNG CÔNG CHỨNG `TryFrom`
// ============================================================================

/// Kiểu TRUYỀN TẢI: khoan dung, phẳng, toàn chuỗi — đúng như JSON gửi tới.
#[derive(Debug, Clone)]
pub struct OrderDto {
    pub id: String,
    pub email: String,
    pub lines: Vec<OrderLineDto>,
}

#[derive(Debug, Clone)]
pub struct OrderLineDto {
    pub name: String,
    pub quantity: u32,
    pub unit_price: u64,
}

impl TryFrom<OrderDto> for Order<Draft> {
    /// Trả về TẤT CẢ lỗi cùng lúc — đúng tinh thần Applicative ở Chương 19.
    type Error = Vec<DomainError>;

    fn try_from(dto: OrderDto) -> Result<Self, Self::Error> {
        let mut errors: Vec<DomainError> = Vec::new();

        let customer = match Email::parse(&dto.email) {
            Ok(e) => Some(e),
            Err(e) => {
                errors.push(e);
                None
            }
        };

        let mut lines: Vec<OrderLine> = Vec::new();
        for d in &dto.lines {
            let name = ProductName::parse(&d.name);
            let qty = Quantity::parse(d.quantity);
            match (name, qty) {
                (Ok(t), Ok(s)) => lines.push(OrderLine {
                    name: t,
                    quantity: s,
                    unit_price: Money::vnd(d.unit_price),
                }),
                (t, s) => {
                    if let Err(e) = t {
                        errors.push(e);
                    }
                    if let Err(e) = s {
                        errors.push(e);
                    }
                }
            }
        }

        match customer {
            Some(customer) if errors.is_empty() => Ok(Order::new(&dto.id, customer, lines)),
            _ => Err(errors),
        }
    }
}

// ============================================================================
// PHẦN 4: LÕI THUẦN TÚY — QUY TẮC NGHIỆP VỤ, KHÔNG CÓ MỘT DÒNG I/O NÀO
// ============================================================================

/// Tính phí vận chuyển theo tổng tiền. Hàm thuần túy 100%: dễ kiểm thử tuyệt đối.
pub fn shipping_fee(total: Money) -> Money {
    if total.value() >= 500_000 {
        Money::vnd(0) // miễn phí cho đơn từ 500k
    } else {
        Money::vnd(30_000)
    }
}

/// Tính chiết khấu theo số dòng hàng. Cũng thuần túy 100%.
pub fn discount_for(total: Money, line_count: usize) -> Money {
    let percent = if line_count >= 10 {
        10
    } else if line_count >= 5 {
        5
    } else {
        0
    };
    Money::vnd(total.value() * percent / 100)
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct Invoice {
    pub subtotal: Money,
    pub discount: Money,
    pub shipping: Money,
    pub total_payable: Money,
}

/// Toàn bộ phép tính hóa đơn — vẫn hoàn toàn thuần túy.
pub fn build_invoice(order: &Order<Validated>) -> Invoice {
    let subtotal = order.total();
    let discount = discount_for(subtotal, order.line_count());
    let after_discount = subtotal.subtract(discount);
    let fee = shipping_fee(after_discount);
    Invoice {
        subtotal,
        discount,
        shipping: fee,
        total_payable: after_discount.plus(fee),
    }
}

// ============================================================================
// CHƯƠNG TRÌNH ĐIỀU HÀNH CHÍNH (VỎ MỆNH LỆNH)
// ============================================================================

fn main() {
    println!("============================================================");
    println!("   MÔ HÌNH HÓA NGHIỆP VỤ BẰNG KIỂU: NEWTYPE & TYPESTATE    ");
    println!("============================================================");

    // ------------------------------------------------------------------
    // 1. HÀM KHỞI TẠO CÓ KIỂM CHỨNG — PHÒNG CÔNG CHỨNG
    // ------------------------------------------------------------------
    println!("\n1. PHÒNG CÔNG CHỨNG (Smart Constructor)");
    for raw in [
        "  An.Nguyen@Example.COM ",
        "khong-co-a-cong",
        "@thieu-ten.vn",
        "",
    ] {
        match Email::parse(raw) {
            Ok(e) => println!("   {:>28} -> ✓ đóng dấu: {}", format!("{:?}", raw), e),
            Err(err) => println!("   {:>28} -> ✗ từ chối: {}", format!("{:?}", raw), err),
        }
    }
    println!("   → Không có cách nào tạo ra một `Email` sai. Trường bên trong là riêng tư.");

    // ------------------------------------------------------------------
    // 2. ĐẠI SỐ CỦA KIỂU — ĐẾM SỐ TRẠNG THÁI
    // ------------------------------------------------------------------
    println!("\n2. ĐẠI SỐ CỦA KIỂU");
    println!("   struct (bool, bool)        -> kiểu TÍCH: 2 × 2 = 4 trạng thái");
    println!("   enum {{ Cash, Transfer, Card }} -> kiểu TỔNG: 1 + 1 + 1 = 3 trạng thái");
    println!("   Cách SAI : struct {{ is_paid: bool, transaction_id: Option<String> }}");
    println!("              -> có 2 tổ hợp VÔ NGHĨA (đã trả mà không mã / chưa trả mà có mã)");
    println!("   Cách ĐÚNG: enum {{ Unpaid, Paid {{ transaction_id }} }} -> 0 tổ hợp vô nghĩa ✓");

    // ------------------------------------------------------------------
    // 3. CỔNG BIÊN HỆ THỐNG: DTO -> KIỂU MIỀN, GOM HẾT LỖI
    // ------------------------------------------------------------------
    println!("\n3. CỔNG BIÊN HỆ THỐNG (DTO -> Miền), gom TẤT CẢ lỗi");
    let bad_dto = OrderDto {
        id: "ORD-0001".to_string(),
        email: "sai-email".to_string(),
        lines: vec![
            OrderLineDto {
                name: "".to_string(),
                quantity: 0,
                unit_price: 100,
            },
            OrderLineDto {
                name: "Bàn phím cơ".to_string(),
                quantity: 2,
                unit_price: 1_200_000,
            },
        ],
    };
    match Order::try_from(bad_dto) {
        Ok(_) => println!("   (không tới đây)"),
        Err(errors) => {
            println!("   Từ chối đơn hàng với {} lỗi:", errors.len());
            for (i, err) in errors.iter().enumerate() {
                println!("     {}. {}", i + 1, err);
            }
        }
    }

    // ------------------------------------------------------------------
    // 4. ĐƠN HỢP LỆ ĐI QUA TOÀN BỘ MÁY TRẠNG THÁI
    // ------------------------------------------------------------------
    println!("\n4. TYPESTATE — QUY TRÌNH ĐƠN HÀNG");
    let good_dto = OrderDto {
        id: "ORD-0002".to_string(),
        email: "  Khach.Hang@Shop.VN  ".to_string(),
        lines: vec![
            OrderLineDto {
                name: "Bàn phím cơ không dây".to_string(),
                quantity: 2,
                unit_price: 1_200_000,
            },
            OrderLineDto {
                name: "Chuột công thái học".to_string(),
                quantity: 1,
                unit_price: 750_000,
            },
            OrderLineDto {
                name: "Lót chuột cỡ lớn".to_string(),
                quantity: 3,
                unit_price: 150_000,
            },
        ],
    };

    let draft_order: Order<Draft> = Order::try_from(good_dto).expect("đơn này phải hợp lệ");
    println!(
        "   [Nhập]          mã={} khách={} số dòng={}",
        draft_order.id(),
        draft_order.customer(),
        draft_order.line_count()
    );

    let validated_order: Order<Validated> = draft_order.validate().expect("đơn có 3 dòng, hợp lệ");
    println!("   [Đã xác thực]   tổng hàng = {}", validated_order.total());

    // ---- LÕI THUẦN TÚY: lập hóa đơn (không I/O, kiểm thử được ngay) ----
    let invoice = build_invoice(&validated_order);
    println!("   ┌─ HÓA ĐƠN (tính bởi LÕI THUẦN TÚY) ─────────────");
    println!("   │ Tạm tính        : {}", invoice.subtotal);
    println!("   │ Chiết khấu      : {}", invoice.discount);
    println!("   │ Phí vận chuyển  : {}", invoice.shipping);
    println!("   │ TỔNG THANH TOÁN : {}", invoice.total_payable);
    println!("   └────────────────────────────────────────────────");

    let paid_order: Order<Paid> = validated_order.pay(PaymentMethod::Transfer {
        transaction_id: "VCB-99881234".to_string(),
    });
    println!(
        "   [Đã thanh toán] cách trả = {:?}",
        paid_order.payment_method()
    );

    let _delivered_order: Order<Delivered> = paid_order.ship("VN-EXP-77213");
    println!("   [Đã giao]       hoàn tất quy trình ✓");

    // ------------------------------------------------------------------
    // 5. NHỮNG GÌ TRÌNH BIÊN DỊCH TỪ CHỐI
    // ------------------------------------------------------------------
    println!("\n5. TRÌNH BIÊN DỊCH LÀ NHÂN VIÊN SOÁT VÉ KHÔNG BAO GIỜ NGỦ GẬT");
    // Bỏ chú thích từng dòng dưới đây để tận mắt thấy lỗi biên dịch:
    // let draft = Order::new("X", Email::parse("a@b.vn").unwrap(), vec![]);
    // draft.ship("VD-1");                 // E0599: Order<Draft> không có `ship`
    // let fake = domain::Email("rác".into()); // E0603: hàm dựng của Email là riêng tư
    // validated_order.total();            // E0382: validated_order đã bị `pay` tiêu thụ
    println!("   Các dòng sau KHÔNG BIÊN DỊCH ĐƯỢC (đã đóng chú thích trong mã nguồn):");
    println!("     · draft.ship(...)           -> E0599: Order<Draft> không có `ship`");
    println!("     · domain::Email(\"rác\".into()) -> E0603: hàm dựng riêng tư, không dựng được");
    println!("     · validated_order.total()   -> E0382: đơn đã bị `pay` tiêu thụ");
    println!("   → Ba lớp lỗi nghiệp vụ bị xóa sổ TRƯỚC khi chương trình kịp chạy.");

    // ------------------------------------------------------------------
    // 6. ĐƠN VI PHẠM QUY TẮC NGHIỆP VỤ
    // ------------------------------------------------------------------
    println!("\n6. XÁC THỰC QUY TẮC NGHIỆP VỤ");
    let email = Email::parse("test@shop.vn").unwrap();
    let empty_order: Order<Draft> = Order::new("ORD-0003", email, vec![]);
    match empty_order.validate() {
        Ok(_) => println!("   (không tới đây)"),
        Err(err) => println!("   Đơn rỗng bị chặn: {}", err),
    }

    println!("\n============================================================");
    println!("  TRẠNG THÁI SAI KHÔNG BIỂU DIỄN ĐƯỢC = LỖI KHÔNG XẢY RA    ");
    println!("============================================================");
}

// ============================================================================
// KIỂM THỬ: LÕI THUẦN TÚY KIỂM THỬ ĐƯỢC MÀ KHÔNG CẦN CSDL, MẠNG HAY MOCK
// ============================================================================

#[cfg(test)]
mod tests {
    use super::*;

    fn sample_order() -> Order<Validated> {
        let email = Email::parse("khach@shop.vn").unwrap();
        let lines = vec![
            OrderLine {
                name: ProductName::parse("Bàn phím").unwrap(),
                quantity: Quantity::parse(2).unwrap(),
                unit_price: Money::vnd(100_000),
            },
            OrderLine {
                name: ProductName::parse("Chuột").unwrap(),
                quantity: Quantity::parse(1).unwrap(),
                unit_price: Money::vnd(50_000),
            },
        ];
        Order::new("ORD-TEST", email, lines).validate().unwrap()
    }

    #[test]
    fn email_accepts_valid_address() {
        let e = Email::parse("  An.Nguyen@Example.COM ").unwrap();
        assert_eq!(e.as_str(), "an.nguyen@example.com"); // đã chuẩn hóa
    }

    #[test]
    fn email_rejects_invalid_address() {
        for bad in [
            "",
            "   ",
            "khong-co-a-cong",
            "@thieu-ten.vn",
            "a@b@c.vn",
            "a@khongcocham",
        ] {
            assert!(Email::parse(bad).is_err(), "phải từ chối {:?}", bad);
        }
    }

    #[test]
    fn quantity_must_be_positive_and_bounded() {
        assert!(Quantity::parse(0).is_err());
        assert!(Quantity::parse(1001).is_err());
        assert_eq!(Quantity::parse(5).unwrap().value(), 5);
    }

    #[test]
    fn product_name_counts_chars_not_bytes() {
        // 50 chữ cái tiếng Việt có dấu = nhiều hơn 50 BYTE, nhưng vẫn hợp lệ.
        let name_50: String = "ế".repeat(50);
        assert!(ProductName::parse(&name_50).is_ok());
        let name_51: String = "ế".repeat(51);
        assert!(ProductName::parse(&name_51).is_err());
    }

    #[test]
    fn empty_order_is_rejected() {
        let email = Email::parse("a@b.vn").unwrap();
        let order = Order::new("X", email, vec![]);
        assert_eq!(order.validate().unwrap_err(), DomainError::EmptyOrder);
    }

    #[test]
    fn dto_collects_all_errors_at_once() {
        let dto = OrderDto {
            id: "X".to_string(),
            email: "sai".to_string(),
            lines: vec![OrderLineDto {
                name: "".to_string(),
                quantity: 0,
                unit_price: 1,
            }],
        };
        let errors = Order::try_from(dto).unwrap_err();
        assert_eq!(errors.len(), 3, "phải gom đủ 3 lỗi, nhận được {:?}", errors);
    }

    // ---- Kiểm thử LÕI THUẦN TÚY: không cần CSDL, không cần mạng ----

    #[test]
    fn total_sums_line_amounts() {
        let order = sample_order();
        // 2 × 100.000 + 1 × 50.000 = 250.000
        assert_eq!(order.total(), Money::vnd(250_000));
    }

    #[test]
    fn shipping_is_free_above_threshold() {
        assert_eq!(shipping_fee(Money::vnd(499_999)), Money::vnd(30_000));
        assert_eq!(shipping_fee(Money::vnd(500_000)), Money::vnd(0));
    }

    #[test]
    fn discount_tiers_by_line_count() {
        let total = Money::vnd(1_000_000);
        assert_eq!(discount_for(total, 3), Money::vnd(0));
        assert_eq!(discount_for(total, 5), Money::vnd(50_000));
        assert_eq!(discount_for(total, 12), Money::vnd(100_000));
    }

    #[test]
    fn invoice_totals_are_correct() {
        let order = sample_order(); // tạm tính 250.000, 2 dòng -> không chiết khấu
        let invoice = build_invoice(&order);
        assert_eq!(invoice.subtotal, Money::vnd(250_000));
        assert_eq!(invoice.discount, Money::vnd(0));
        assert_eq!(invoice.shipping, Money::vnd(30_000));
        assert_eq!(invoice.total_payable, Money::vnd(280_000));
    }

    #[test]
    fn typestate_flow_runs_all_four_steps() {
        let order = sample_order();
        let paid = order.pay(PaymentMethod::Cash);
        assert_eq!(paid.payment_method(), &PaymentMethod::Cash);
        let delivered = paid.ship("VD-001");
        assert_eq!(delivered.id(), "ORD-TEST");
    }

    #[test]
    fn typestate_is_zero_cost_at_runtime() {
        use std::mem::size_of;
        // PhantomData chiếm 0 byte: Order<Draft> và Order<Delivered> có cùng kích thước.
        assert_eq!(size_of::<Order<Draft>>(), size_of::<Order<Delivered>>());
        assert_eq!(size_of::<Draft>(), 0);
        assert_eq!(size_of::<PhantomData<Delivered>>(), 0);
    }
}
