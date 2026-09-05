#![allow(dead_code, unused_variables, unused_imports)]
// Tệp: src/main.rs
// Chương trình thực chiến: Kiểu bọc, Hàm khởi tạo có kiểm chứng và Typestate

use std::convert::TryFrom;
use std::marker::PhantomData;

// ============================================================================
// PHẦN 1: MÔ-ĐUN MIỀN NGHIỆP VỤ
// Đặt trong `mod` để tính RIÊNG TƯ của các trường thực sự có hiệu lực.
// ============================================================================
pub mod mien {
    use std::fmt;

    #[derive(Debug, Clone, PartialEq, Eq)]
    pub enum DomainError {
        EmailSai(String),
        TenSanPhamSai(String),
        SoLuongSai(String),
        DonRong,
        DonQuaLon { so_dong: usize, toi_da: usize },
    }

    impl fmt::Display for DomainError {
        fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
            match self {
                DomainError::EmailSai(s) => write!(f, "Email không hợp lệ: {}", s),
                DomainError::TenSanPhamSai(s) => write!(f, "Tên sản phẩm không hợp lệ: {}", s),
                DomainError::SoLuongSai(s) => write!(f, "Số lượng không hợp lệ: {}", s),
                DomainError::DonRong => write!(f, "Đơn hàng phải có ít nhất 1 dòng hàng"),
                DomainError::DonQuaLon { so_dong, toi_da } => {
                    write!(f, "Đơn có {} dòng, vượt giới hạn {} dòng", so_dong, toi_da)
                }
            }
        }
    }

    // ---------------------------------------------------------------------
    // KIỂU BỌC 1: Email — trường riêng tư, chỉ tạo được qua `analyze`
    // ---------------------------------------------------------------------
    #[derive(Debug, Clone, PartialEq, Eq)]
    pub struct Email(String); // KHÔNG có `pub` trước String → đây là con dấu

    impl Email {
        pub fn analyze(tho: &str) -> Result<Self, DomainError> {
            let s = tho.trim().to_lowercase();
            if s.is_empty() {
                return Err(DomainError::EmailSai("chuỗi rỗng".to_string()));
            }
            let part: Vec<&str> = s.split('@').collect();
            if part.len() != 2 || part[0].is_empty() || !part[1].contains('.') {
                return Err(DomainError::EmailSai(s));
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
    // KIỂU BỌC 2: TenSanPham — chuỗi có giới hạn độ dài
    // ---------------------------------------------------------------------
    #[derive(Debug, Clone, PartialEq, Eq)]
    pub struct TenSanPham(String);

    impl TenSanPham {
        pub const TOI_DA: usize = 50;

        pub fn analyze(tho: &str) -> Result<Self, DomainError> {
            let s = tho.trim();
            let num_ky_from = s.chars().count(); // đếm CHỮ CÁI, không đếm byte (Chương 05)
            if num_ky_from == 0 {
                Err(DomainError::TenSanPhamSai("chuỗi rỗng".to_string()))
            } else if num_ky_from > Self::TOI_DA {
                Err(DomainError::TenSanPhamSai(format!(
                    "dài {} ký tự, tối đa {}",
                    num_ky_from,
                    Self::TOI_DA
                )))
            } else {
                Ok(TenSanPham(s.to_string()))
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
        pub const TOI_DA: u32 = 1000;

        pub fn analyze(n: u32) -> Result<Self, DomainError> {
            if n == 0 {
                Err(DomainError::SoLuongSai("phải lớn hơn 0".to_string()))
            } else if n > Self::TOI_DA {
                Err(DomainError::SoLuongSai(format!("{} vượt quá {}", n, Self::TOI_DA)))
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
        pub fn dong(n: u64) -> Self {
            Money(n)
        }
        pub fn value(&self) -> u64 {
            self.0
        }
        pub fn gate(self, other: Money) -> Money {
            Money(self.0 + other.0) // đây là một VỊ NHÓM (Chương 18)!
        }
        pub fn subtract(self, other: Money) -> Money {
            Money(self.0.saturating_sub(other.0))
        }
        pub fn nhan(self, he_so: u32) -> Money {
            Money(self.0 * he_so as u64)
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
    pub enum MathOp {
        TienMat,
        ChuyenKhoan { id_trade: String },
        The { bon_so_cuoi: String },
    }

    // ---------------------------------------------------------------------
    // Dòng hàng: một kiểu TÍCH gồm toàn kiểu đã được công chứng
    // ---------------------------------------------------------------------
    #[derive(Debug, Clone, PartialEq, Eq)]
    pub struct CloseQueue {
        pub name: TenSanPham,
        pub quantity: Quantity,
        pub don_price: Money,
    }

    impl CloseQueue {
        pub fn into_tien(&self) -> Money {
            self.don_price.nhan(self.quantity.value())
        }
    }
}

use mien::*;

// ============================================================================
// PHẦN 2: TYPESTATE — MÁY TRẠNG THÁI ĐƯỢC MÃ HÓA VÀO KIỂU
// ============================================================================

/// Bốn "thẻ đánh dấu" trạng thái. Chúng chiếm 0 byte và biến mất khi biên dịch.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct Import;
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct Authenticated;
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct MathDone;
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct Delivered;

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct DonQueue<TT> {
    id: String,
    customer: Email,
    dong: Vec<CloseQueue>,
    payment: Option<MathOp>,
    _state: PhantomData<TT>,
}

/// Các phương thức dùng chung cho MỌI trạng thái.
impl<TT> DonQueue<TT> {
    pub fn id(&self) -> &str {
        &self.id
    }
    pub fn customer(&self) -> &Email {
        &self.customer
    }
    pub fn so_dong(&self) -> usize {
        self.dong.len()
    }
    /// Tổng tiền = gộp các thành tiền bằng phép cộng của vị nhóm Money.
    pub fn tong_tien(&self) -> Money {
        self.dong
            .iter()
            .map(|d| d.into_tien())
            .fold(Money::dong(0), |a, b| a.gate(b))
    }
}

pub const SO_DONG_TOI_DA: usize = 20;

/// Trạng thái NHẬP: chỉ có đúng một hành động hợp lệ — xác thực.
impl DonQueue<Import> {
    pub fn new(id: &str, customer: Email, dong: Vec<CloseQueue>) -> Self {
        DonQueue {
            id: id.to_string(),
            customer,
            dong,
            payment: None,
            _state: PhantomData,
        }
    }

    pub fn auth(self) -> Result<DonQueue<Authenticated>, DomainError> {
        if self.dong.is_empty() {
            return Err(DomainError::DonRong);
        }
        if self.dong.len() > SO_DONG_TOI_DA {
            return Err(DomainError::DonQuaLon {
                so_dong: self.dong.len(),
                toi_da: SO_DONG_TOI_DA,
            });
        }
        Ok(DonQueue {
            id: self.id,
            customer: self.customer,
            dong: self.dong,
            payment: None,
            _state: PhantomData,
        })
    }
}

/// Trạng thái ĐÃ XÁC THỰC: chỉ có thể thanh toán.
impl DonQueue<Authenticated> {
    pub fn payment(self, cach: MathOp) -> DonQueue<MathDone> {
        DonQueue {
            id: self.id,
            customer: self.customer,
            dong: self.dong,
            payment: Some(cach),
            _state: PhantomData,
        }
    }
}

/// Trạng thái ĐÃ THANH TOÁN: chỉ có thể giao hàng.
impl DonQueue<MathDone> {
    pub fn payment_method(&self) -> &MathOp {
        // An toàn tuyệt đối: chỉ trạng thái này mới tồn tại, và nó LUÔN có thanh toán.
        self.payment
            .as_ref()
            .expect("bất biến của DonHang<DaThanhToan>: luôn có thông tin thanh toán")
    }

    pub fn delivery_queue(self, ma_van_don: &str) -> DonQueue<Delivered> {
        println!(
            "   [VỎ MỆNH LỆNH] Gửi email tới {} về vận đơn {}",
            self.customer, ma_van_don
        );
        DonQueue {
            id: self.id,
            customer: self.customer,
            dong: self.dong,
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
    pub dong: Vec<OrderLineDto>,
}

#[derive(Debug, Clone)]
pub struct OrderLineDto {
    pub name: String,
    pub quantity: u32,
    pub don_price: u64,
}

impl TryFrom<OrderDto> for DonQueue<Import> {
    /// Trả về TẤT CẢ lỗi cùng lúc — đúng compute thần Applicative ở Chương 19.
    type Error = Vec<DomainError>;

    fn try_from(dto: OrderDto) -> Result<Self, Self::Error> {
        let mut error: Vec<DomainError> = Vec::new();

        let customer = match Email::analyze(&dto.email) {
            Ok(e) => Some(e),
            Err(e) => {
                error.push(e);
                None
            }
        };

        let mut dong: Vec<CloseQueue> = Vec::new();
        for d in &dto.dong {
            let name = TenSanPham::analyze(&d.name);
            let sl = Quantity::analyze(d.quantity);
            match (name, sl) {
                (Ok(t), Ok(s)) => dong.push(CloseQueue {
                    name: t,
                    quantity: s,
                    don_price: Money::dong(d.don_price),
                }),
                (t, s) => {
                    if let Err(e) = t {
                        error.push(e);
                    }
                    if let Err(e) = s {
                        error.push(e);
                    }
                }
            }
        }

        match customer {
            Some(k) if error.is_empty() => Ok(DonQueue::new(&dto.id, k, dong)),
            _ => Err(error),
        }
    }
}

// ============================================================================
// PHẦN 4: LÕI THUẦN TÚY — QUY TẮC NGHIỆP VỤ, KHÔNG CÓ MỘT DÒNG I/O NÀO
// ============================================================================

/// Tính phí vận chuyển theo tổng tiền. Hàm thuần túy 100%: dễ kiểm thử tuyệt đối.
pub fn shipping_fee(tong: Money) -> Money {
    if tong.value() >= 500_000 {
        Money::dong(0) // miễn phí cho đơn từ 500k
    } else {
        Money::dong(30_000)
    }
}

/// Tính chiết khấu theo số dòng hàng. Cũng thuần túy 100%.
pub fn apply_discount(tong: Money, so_dong: usize) -> Money {
    let percent = if so_dong >= 10 {
        10
    } else if so_dong >= 5 {
        5
    } else {
        0
    };
    Money::dong(tong.value() * percent / 100)
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct Invoice {
    pub computed_temp: Money,
    pub discount: Money,
    pub phi_van_transfer: Money,
    pub total_payable: Money,
}

/// Toàn bộ phép tính hóa đơn — vẫn hoàn toàn thuần túy.
pub fn invoice_loop(don: &DonQueue<Authenticated>) -> Invoice {
    let computed_temp = don.tong_tien();
    let discount = apply_discount(computed_temp, don.so_dong());
    let sau_chiet_khau = computed_temp.subtract(discount);
    let phi = shipping_fee(sau_chiet_khau);
    Invoice {
        computed_temp,
        discount,
        phi_van_transfer: phi,
        total_payable: sau_chiet_khau.gate(phi),
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
    for tho in ["  An.Nguyen@Example.COM ", "khong-co-a-cong", "@thieu-ten.vn", ""] {
        match Email::analyze(tho) {
            Ok(e) => println!("   {:>28} -> ✓ đóng dấu: {}", format!("{:?}", tho), e),
            Err(l) => println!("   {:>28} -> ✗ từ chối: {}", format!("{:?}", tho), l),
        }
    }
    println!("   → Không có cách nào tạo ra một `Email` sai. Trường bên trong là riêng tư.");

    // ------------------------------------------------------------------
    // 2. ĐẠI SỐ CỦA KIỂU — ĐẾM SỐ TRẠNG THÁI
    // ------------------------------------------------------------------
    println!("\n2. ĐẠI SỐ CỦA KIỂU");
    println!("   struct (bool, bool)        -> kiểu TÍCH: 2 × 2 = 4 trạng thái");
    println!("   enum {{ TienMat, CK, The }}  -> kiểu TỔNG: 1 + 1 + 1 = 3 trạng thái");
    println!("   Cách SAI : struct {{ da_tra: bool, ma_gd: Option<String> }}");
    println!("              -> có 2 tổ hợp VÔ NGHĨA (đã trả mà không mã / chưa trả mà có mã)");
    println!("   Cách ĐÚNG: enum {{ ChuaTra, DaTra {{ ma_gd }} }} -> 0 tổ hợp vô nghĩa ✓");

    // ------------------------------------------------------------------
    // 3. CỔNG BIÊN HỆ THỐNG: DTO -> KIỂU MIỀN, GOM HẾT LỖI
    // ------------------------------------------------------------------
    println!("\n3. CỔNG BIÊN HỆ THỐNG (DTO -> Miền), gom TẤT CẢ lỗi");
    let dto_hong = OrderDto {
        id: "ORD-0001".to_string(),
        email: "sai-email".to_string(),
        dong: vec![
            OrderLineDto { name: "".to_string(), quantity: 0, don_price: 100 },
            OrderLineDto { name: "Bàn phím cơ".to_string(), quantity: 2, don_price: 1_200_000 },
        ],
    };
    match DonQueue::try_from(dto_hong) {
        Ok(_) => println!("   (không tới đây)"),
        Err(error) => {
            println!("   Từ chối đơn hàng với {} lỗi:", error.len());
            for (i, l) in error.iter().enumerate() {
                println!("     {}. {}", i + 1, l);
            }
        }
    }

    // ------------------------------------------------------------------
    // 4. ĐƠN HỢP LỆ ĐI QUA TOÀN BỘ MÁY TRẠNG THÁI
    // ------------------------------------------------------------------
    println!("\n4. TYPESTATE — QUY TRÌNH ĐƠN HÀNG");
    let dto_tot = OrderDto {
        id: "ORD-0002".to_string(),
        email: "  Khach.Hang@Shop.VN  ".to_string(),
        dong: vec![
            OrderLineDto { name: "Bàn phím cơ không dây".to_string(), quantity: 2, don_price: 1_200_000 },
            OrderLineDto { name: "Chuột công thái học".to_string(), quantity: 1, don_price: 750_000 },
            OrderLineDto { name: "Lót chuột cỡ lớn".to_string(), quantity: 3, don_price: 150_000 },
        ],
    };

    let don_import: DonQueue<Import> = DonQueue::try_from(dto_tot).expect("đơn này phải hợp lệ");
    println!(
        "   [Nhập]          mã={} khách={} số dòng={}",
        don_import.id(),
        don_import.customer(),
        don_import.so_dong()
    );

    let don_auth: DonQueue<Authenticated> = don_import.auth().expect("đơn có 3 dòng, hợp lệ");
    println!("   [Đã xác thực]   tổng hàng = {}", don_auth.tong_tien());

    // ---- LÕI THUẦN TÚY: lập hóa đơn (không I/O, kiểm thử được ngay) ----
    let hoa_don = invoice_loop(&don_auth);
    println!("   ┌─ HÓA ĐƠN (tính bởi LÕI THUẦN TÚY) ─────────────");
    println!("   │ Tạm tính        : {}", hoa_don.computed_temp);
    println!("   │ Chiết khấu      : {}", hoa_don.discount);
    println!("   │ Phí vận chuyển  : {}", hoa_don.phi_van_transfer);
    println!("   │ TỔNG THANH TOÁN : {}", hoa_don.total_payable);
    println!("   └────────────────────────────────────────────────");

    let don_da_tra: DonQueue<MathDone> = don_auth.payment(MathOp::ChuyenKhoan {
        id_trade: "VCB-99881234".to_string(),
    });
    println!("   [Đã thanh toán] cách trả = {:?}", don_da_tra.payment_method());

    let _don_da_giao: DonQueue<Delivered> = don_da_tra.delivery_queue("VN-EXP-77213");
    println!("   [Đã giao]       hoàn tất quy trình ✓");

    // ------------------------------------------------------------------
    // 5. NHỮNG GÌ TRÌNH BIÊN DỊCH TỪ CHỐI
    // ------------------------------------------------------------------
    println!("\n5. TRÌNH BIÊN DỊCH LÀ NHÂN VIÊN SOÁT VÉ KHÔNG BAO GIỜ NGỦ GẬT");
    println!("   Các dòng sau KHÔNG BIÊN DỊCH ĐƯỢC (đã đóng chú thích trong mã nguồn):");
    println!("     · don_nhap.giao_hang(...)  -> E0599: DonHang<Nhap> không có `giao_hang`");
    println!("     · mien::Email(\"rác\".into()) -> E0603: trường riêng tư, không dựng được");
    println!("     · don_xac_thuc.tong_tien() -> E0382: đơn đã bị `thanh_toan` tiêu thụ");
    println!("   → Ba lớp lỗi nghiệp vụ bị xóa sổ TRƯỚC khi chương trình kịp chạy.");

    // ------------------------------------------------------------------
    // 6. ĐƠN VI PHẠM QUY TẮC NGHIỆP VỤ
    // ------------------------------------------------------------------
    println!("\n6. XÁC THỰC QUY TẮC NGHIỆP VỤ");
    let email = Email::analyze("test@shop.vn").unwrap();
    let don_rong: DonQueue<Import> = DonQueue::new("ORD-0003", email, vec![]);
    match don_rong.auth() {
        Ok(_) => println!("   (không tới đây)"),
        Err(l) => println!("   Đơn rỗng bị chặn: {}", l),
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

    fn don_mau() -> DonQueue<Authenticated> {
        let email = Email::analyze("customer@shop.vn").unwrap();
        let dong = vec![
            CloseQueue {
                name: TenSanPham::analyze("Bàn phím").unwrap(),
                quantity: Quantity::analyze(2).unwrap(),
                don_price: Money::dong(100_000),
            },
            CloseQueue {
                name: TenSanPham::analyze("Chuột").unwrap(),
                quantity: Quantity::analyze(1).unwrap(),
                don_price: Money::dong(50_000),
            },
        ];
        DonQueue::new("ORD-TEST", email, dong).auth().unwrap()
    }

    #[test]
    fn email_chap_nhan_dia_chi_hop_le() {
        let e = Email::analyze("  An.Nguyen@Example.COM ").unwrap();
        assert_eq!(e.as_str(), "an.nguyen@example.com"); // đã chuẩn hóa
    }

    #[test]
    fn email_tu_choi_dia_chi_sai() {
        for xau in ["", "   ", "khong-co-a-cong", "@thieu-ten.vn", "a@b@c.vn", "a@khongcocham"] {
            assert!(Email::analyze(xau).is_err(), "phải từ chối {:?}", xau);
        }
    }

    #[test]
    fn quantity_must_duong_and_in_limit() {
        assert!(Quantity::analyze(0).is_err());
        assert!(Quantity::analyze(1001).is_err());
        assert_eq!(Quantity::analyze(5).unwrap().value(), 5);
    }

    #[test]
    fn ten_san_pham_dem_ky_tu_khong_dem_byte() {
        // 50 chữ cái tiếng Việt có dấu = nhiều hơn 50 BYTE, nhưng vẫn hợp lệ.
        let name_long: String = "ế".repeat(50);
        assert!(TenSanPham::analyze(&name_long).is_ok());
        let qua_long: String = "ế".repeat(51);
        assert!(TenSanPham::analyze(&qua_long).is_err());
    }

    #[test]
    fn don_empty_is_reject() {
        let email = Email::analyze("a@b.vn").unwrap();
        let don = DonQueue::new("X", email, vec![]);
        assert_eq!(don.auth().unwrap_err(), DomainError::DonRong);
    }

    #[test]
    fn dto_gom_tat_ca_loi_cung_luc() {
        let dto = OrderDto {
            id: "X".to_string(),
            email: "sai".to_string(),
            dong: vec![OrderLineDto { name: "".to_string(), quantity: 0, don_price: 1 }],
        };
        let error = DonQueue::try_from(dto).unwrap_err();
        assert_eq!(error.len(), 3, "phải gom đủ 3 lỗi, nhận được {:?}", error);
    }

    // ---- Kiểm thử LÕI THUẦN TÚY: không cần CSDL, không cần mạng ----

    #[test]
    fn tong_tien_cong_dung_thanh_tien_tung_dong() {
        let don = don_mau();
        // 2 × 100.000 + 1 × 50.000 = 250.000
        assert_eq!(don.tong_tien(), Money::dong(250_000));
    }

    #[test]
    fn phi_van_chuyen_mien_phi_tu_500k() {
        assert_eq!(shipping_fee(Money::dong(499_999)), Money::dong(30_000));
        assert_eq!(shipping_fee(Money::dong(500_000)), Money::dong(0));
    }

    #[test]
    fn chiet_khau_theo_bac_so_dong() {
        let tong = Money::dong(1_000_000);
        assert_eq!(apply_discount(tong, 3), Money::dong(0));
        assert_eq!(apply_discount(tong, 5), Money::dong(50_000));
        assert_eq!(apply_discount(tong, 12), Money::dong(100_000));
    }

    #[test]
    fn hoa_don_tinh_use_toan_unit() {
        let don = don_mau(); // tạm tính 250.000, 2 dòng -> không chiết khấu
        let hd = invoice_loop(&don);
        assert_eq!(hd.computed_temp, Money::dong(250_000));
        assert_eq!(hd.discount, Money::dong(0));
        assert_eq!(hd.phi_van_transfer, Money::dong(30_000));
        assert_eq!(hd.total_payable, Money::dong(280_000));
    }

    #[test]
    fn quy_trinh_typestate_chay_het_bon_buoc() {
        let don = don_mau();
        let da_tra = don.payment(MathOp::TienMat);
        assert_eq!(da_tra.payment_method(), &MathOp::TienMat);
        let da_giao = da_tra.delivery_queue("VD-001");
        assert_eq!(da_giao.id(), "ORD-TEST");
    }

    #[test]
    fn typestate_khong_ton_bo_nho_luc_chay() {
        use std::mem::size_of;
        // PhantomData chiếm 0 byte: DonQueue<Nhap> và DonQueue<Delivered> có cùng kích thước.
        assert_eq!(size_of::<DonQueue<Import>>(), size_of::<DonQueue<Delivered>>());
        assert_eq!(size_of::<Import>(), 0);
        assert_eq!(size_of::<PhantomData<Delivered>>(), 0);
    }
}
