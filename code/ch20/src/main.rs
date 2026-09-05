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
    pub enum LoiMien {
        EmailSai(String),
        TenSanPhamSai(String),
        SoLuongSai(String),
        DonRong,
        DonQuaLon { so_dong: usize, toi_da: usize },
    }

    impl fmt::Display for LoiMien {
        fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
            match self {
                LoiMien::EmailSai(s) => write!(f, "Email không hợp lệ: {}", s),
                LoiMien::TenSanPhamSai(s) => write!(f, "Tên sản phẩm không hợp lệ: {}", s),
                LoiMien::SoLuongSai(s) => write!(f, "Số lượng không hợp lệ: {}", s),
                LoiMien::DonRong => write!(f, "Đơn hàng phải có ít nhất 1 dòng hàng"),
                LoiMien::DonQuaLon { so_dong, toi_da } => {
                    write!(f, "Đơn có {} dòng, vượt giới hạn {} dòng", so_dong, toi_da)
                }
            }
        }
    }

    // ---------------------------------------------------------------------
    // KIỂU BỌC 1: Email — trường riêng tư, chỉ tạo được qua `phan_tich`
    // ---------------------------------------------------------------------
    #[derive(Debug, Clone, PartialEq, Eq)]
    pub struct Email(String); // KHÔNG có `pub` trước String → đây là con dấu

    impl Email {
        pub fn phan_tich(tho: &str) -> Result<Self, LoiMien> {
            let s = tho.trim().to_lowercase();
            if s.is_empty() {
                return Err(LoiMien::EmailSai("chuỗi rỗng".to_string()));
            }
            let phan: Vec<&str> = s.split('@').collect();
            if phan.len() != 2 || phan[0].is_empty() || !phan[1].contains('.') {
                return Err(LoiMien::EmailSai(s));
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

        pub fn phan_tich(tho: &str) -> Result<Self, LoiMien> {
            let s = tho.trim();
            let so_ky_tu = s.chars().count(); // đếm CHỮ CÁI, không đếm byte (Chương 05)
            if so_ky_tu == 0 {
                Err(LoiMien::TenSanPhamSai("chuỗi rỗng".to_string()))
            } else if so_ky_tu > Self::TOI_DA {
                Err(LoiMien::TenSanPhamSai(format!(
                    "dài {} ký tự, tối đa {}",
                    so_ky_tu,
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
    // KIỂU BỌC 3: SoLuong — số nguyên dương trong khoảng cho phép
    // ---------------------------------------------------------------------
    #[derive(Debug, Clone, Copy, PartialEq, Eq)]
    pub struct SoLuong(u32);

    impl SoLuong {
        pub const TOI_DA: u32 = 1000;

        pub fn phan_tich(n: u32) -> Result<Self, LoiMien> {
            if n == 0 {
                Err(LoiMien::SoLuongSai("phải lớn hơn 0".to_string()))
            } else if n > Self::TOI_DA {
                Err(LoiMien::SoLuongSai(format!("{} vượt quá {}", n, Self::TOI_DA)))
            } else {
                Ok(SoLuong(n))
            }
        }
        pub fn gia_tri(&self) -> u32 {
            self.0
        }
    }

    // ---------------------------------------------------------------------
    // KIỂU BỌC 4: SoTien — tính bằng ĐƠN VỊ NHỎ NHẤT (đồng), dùng u64.
    // KHÔNG BAO GIỜ dùng f64 cho tiền tệ (xem cảnh báo ở Chương 03)!
    // ---------------------------------------------------------------------
    #[derive(Debug, Clone, Copy, PartialEq, Eq, PartialOrd, Ord)]
    pub struct SoTien(u64);

    impl SoTien {
        pub fn dong(n: u64) -> Self {
            SoTien(n)
        }
        pub fn gia_tri(&self) -> u64 {
            self.0
        }
        pub fn cong(self, khac: SoTien) -> SoTien {
            SoTien(self.0 + khac.0) // đây là một VỊ NHÓM (Chương 18)!
        }
        pub fn tru(self, khac: SoTien) -> SoTien {
            SoTien(self.0.saturating_sub(khac.0))
        }
        pub fn nhan(self, he_so: u32) -> SoTien {
            SoTien(self.0 * he_so as u64)
        }
    }

    impl fmt::Display for SoTien {
        fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
            write!(f, "{} đ", self.0)
        }
    }

    // ---------------------------------------------------------------------
    // KIỂU TỔNG: cách thanh toán — KHÔNG CÒN tổ hợp vô nghĩa
    // ---------------------------------------------------------------------
    #[derive(Debug, Clone, PartialEq, Eq)]
    pub enum ThanhToan {
        TienMat,
        ChuyenKhoan { ma_giao_dich: String },
        The { bon_so_cuoi: String },
    }

    // ---------------------------------------------------------------------
    // Dòng hàng: một kiểu TÍCH gồm toàn kiểu đã được công chứng
    // ---------------------------------------------------------------------
    #[derive(Debug, Clone, PartialEq, Eq)]
    pub struct DongHang {
        pub ten: TenSanPham,
        pub so_luong: SoLuong,
        pub don_gia: SoTien,
    }

    impl DongHang {
        pub fn thanh_tien(&self) -> SoTien {
            self.don_gia.nhan(self.so_luong.gia_tri())
        }
    }
}

use mien::*;

// ============================================================================
// PHẦN 2: TYPESTATE — MÁY TRẠNG THÁI ĐƯỢC MÃ HÓA VÀO KIỂU
// ============================================================================

/// Bốn "thẻ đánh dấu" trạng thái. Chúng chiếm 0 byte và biến mất khi biên dịch.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct Nhap;
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct DaXacThuc;
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct DaThanhToan;
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct DaGiao;

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct DonHang<TT> {
    ma: String,
    khach: Email,
    dong: Vec<DongHang>,
    thanh_toan: Option<ThanhToan>,
    _trang_thai: PhantomData<TT>,
}

/// Các phương thức dùng chung cho MỌI trạng thái.
impl<TT> DonHang<TT> {
    pub fn ma(&self) -> &str {
        &self.ma
    }
    pub fn khach(&self) -> &Email {
        &self.khach
    }
    pub fn so_dong(&self) -> usize {
        self.dong.len()
    }
    /// Tổng tiền = gộp các thành tiền bằng phép cộng của vị nhóm SoTien.
    pub fn tong_tien(&self) -> SoTien {
        self.dong
            .iter()
            .map(|d| d.thanh_tien())
            .fold(SoTien::dong(0), |a, b| a.cong(b))
    }
}

pub const SO_DONG_TOI_DA: usize = 20;

/// Trạng thái NHẬP: chỉ có đúng một hành động hợp lệ — xác thực.
impl DonHang<Nhap> {
    pub fn moi(ma: &str, khach: Email, dong: Vec<DongHang>) -> Self {
        DonHang {
            ma: ma.to_string(),
            khach,
            dong,
            thanh_toan: None,
            _trang_thai: PhantomData,
        }
    }

    pub fn xac_thuc(self) -> Result<DonHang<DaXacThuc>, LoiMien> {
        if self.dong.is_empty() {
            return Err(LoiMien::DonRong);
        }
        if self.dong.len() > SO_DONG_TOI_DA {
            return Err(LoiMien::DonQuaLon {
                so_dong: self.dong.len(),
                toi_da: SO_DONG_TOI_DA,
            });
        }
        Ok(DonHang {
            ma: self.ma,
            khach: self.khach,
            dong: self.dong,
            thanh_toan: None,
            _trang_thai: PhantomData,
        })
    }
}

/// Trạng thái ĐÃ XÁC THỰC: chỉ có thể thanh toán.
impl DonHang<DaXacThuc> {
    pub fn thanh_toan(self, cach: ThanhToan) -> DonHang<DaThanhToan> {
        DonHang {
            ma: self.ma,
            khach: self.khach,
            dong: self.dong,
            thanh_toan: Some(cach),
            _trang_thai: PhantomData,
        }
    }
}

/// Trạng thái ĐÃ THANH TOÁN: chỉ có thể giao hàng.
impl DonHang<DaThanhToan> {
    pub fn cach_thanh_toan(&self) -> &ThanhToan {
        // An toàn tuyệt đối: chỉ trạng thái này mới tồn tại, và nó LUÔN có thanh toán.
        self.thanh_toan
            .as_ref()
            .expect("bất biến của DonHang<DaThanhToan>: luôn có thông tin thanh toán")
    }

    pub fn giao_hang(self, ma_van_don: &str) -> DonHang<DaGiao> {
        println!(
            "   [VỎ MỆNH LỆNH] Gửi email tới {} về vận đơn {}",
            self.khach, ma_van_don
        );
        DonHang {
            ma: self.ma,
            khach: self.khach,
            dong: self.dong,
            thanh_toan: self.thanh_toan,
            _trang_thai: PhantomData,
        }
    }
}

// ============================================================================
// PHẦN 3: BIÊN HỆ THỐNG — DTO VÀ CỔNG CÔNG CHỨNG `TryFrom`
// ============================================================================

/// Kiểu TRUYỀN TẢI: khoan dung, phẳng, toàn chuỗi — đúng như JSON gửi tới.
#[derive(Debug, Clone)]
pub struct DonHangDto {
    pub ma: String,
    pub email: String,
    pub dong: Vec<DongHangDto>,
}

#[derive(Debug, Clone)]
pub struct DongHangDto {
    pub ten: String,
    pub so_luong: u32,
    pub don_gia: u64,
}

impl TryFrom<DonHangDto> for DonHang<Nhap> {
    /// Trả về TẤT CẢ lỗi cùng lúc — đúng tinh thần Applicative ở Chương 19.
    type Error = Vec<LoiMien>;

    fn try_from(dto: DonHangDto) -> Result<Self, Self::Error> {
        let mut loi: Vec<LoiMien> = Vec::new();

        let khach = match Email::phan_tich(&dto.email) {
            Ok(e) => Some(e),
            Err(e) => {
                loi.push(e);
                None
            }
        };

        let mut dong: Vec<DongHang> = Vec::new();
        for d in &dto.dong {
            let ten = TenSanPham::phan_tich(&d.ten);
            let sl = SoLuong::phan_tich(d.so_luong);
            match (ten, sl) {
                (Ok(t), Ok(s)) => dong.push(DongHang {
                    ten: t,
                    so_luong: s,
                    don_gia: SoTien::dong(d.don_gia),
                }),
                (t, s) => {
                    if let Err(e) = t {
                        loi.push(e);
                    }
                    if let Err(e) = s {
                        loi.push(e);
                    }
                }
            }
        }

        match khach {
            Some(k) if loi.is_empty() => Ok(DonHang::moi(&dto.ma, k, dong)),
            _ => Err(loi),
        }
    }
}

// ============================================================================
// PHẦN 4: LÕI THUẦN TÚY — QUY TẮC NGHIỆP VỤ, KHÔNG CÓ MỘT DÒNG I/O NÀO
// ============================================================================

/// Tính phí vận chuyển theo tổng tiền. Hàm thuần túy 100%: dễ kiểm thử tuyệt đối.
pub fn tinh_phi_van_chuyen(tong: SoTien) -> SoTien {
    if tong.gia_tri() >= 500_000 {
        SoTien::dong(0) // miễn phí cho đơn từ 500k
    } else {
        SoTien::dong(30_000)
    }
}

/// Tính chiết khấu theo số dòng hàng. Cũng thuần túy 100%.
pub fn tinh_chiet_khau(tong: SoTien, so_dong: usize) -> SoTien {
    let phan_tram = if so_dong >= 10 {
        10
    } else if so_dong >= 5 {
        5
    } else {
        0
    };
    SoTien::dong(tong.gia_tri() * phan_tram / 100)
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct HoaDon {
    pub tam_tinh: SoTien,
    pub chiet_khau: SoTien,
    pub phi_van_chuyen: SoTien,
    pub tong_thanh_toan: SoTien,
}

/// Toàn bộ phép tính hóa đơn — vẫn hoàn toàn thuần túy.
pub fn lap_hoa_don(don: &DonHang<DaXacThuc>) -> HoaDon {
    let tam_tinh = don.tong_tien();
    let chiet_khau = tinh_chiet_khau(tam_tinh, don.so_dong());
    let sau_chiet_khau = tam_tinh.tru(chiet_khau);
    let phi = tinh_phi_van_chuyen(sau_chiet_khau);
    HoaDon {
        tam_tinh,
        chiet_khau,
        phi_van_chuyen: phi,
        tong_thanh_toan: sau_chiet_khau.cong(phi),
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
        match Email::phan_tich(tho) {
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
    let dto_hong = DonHangDto {
        ma: "ORD-0001".to_string(),
        email: "sai-email".to_string(),
        dong: vec![
            DongHangDto { ten: "".to_string(), so_luong: 0, don_gia: 100 },
            DongHangDto { ten: "Bàn phím cơ".to_string(), so_luong: 2, don_gia: 1_200_000 },
        ],
    };
    match DonHang::try_from(dto_hong) {
        Ok(_) => println!("   (không tới đây)"),
        Err(loi) => {
            println!("   Từ chối đơn hàng với {} lỗi:", loi.len());
            for (i, l) in loi.iter().enumerate() {
                println!("     {}. {}", i + 1, l);
            }
        }
    }

    // ------------------------------------------------------------------
    // 4. ĐƠN HỢP LỆ ĐI QUA TOÀN BỘ MÁY TRẠNG THÁI
    // ------------------------------------------------------------------
    println!("\n4. TYPESTATE — QUY TRÌNH ĐƠN HÀNG");
    let dto_tot = DonHangDto {
        ma: "ORD-0002".to_string(),
        email: "  Khach.Hang@Shop.VN  ".to_string(),
        dong: vec![
            DongHangDto { ten: "Bàn phím cơ không dây".to_string(), so_luong: 2, don_gia: 1_200_000 },
            DongHangDto { ten: "Chuột công thái học".to_string(), so_luong: 1, don_gia: 750_000 },
            DongHangDto { ten: "Lót chuột cỡ lớn".to_string(), so_luong: 3, don_gia: 150_000 },
        ],
    };

    let don_nhap: DonHang<Nhap> = DonHang::try_from(dto_tot).expect("đơn này phải hợp lệ");
    println!(
        "   [Nhập]          mã={} khách={} số dòng={}",
        don_nhap.ma(),
        don_nhap.khach(),
        don_nhap.so_dong()
    );

    let don_xac_thuc: DonHang<DaXacThuc> = don_nhap.xac_thuc().expect("đơn có 3 dòng, hợp lệ");
    println!("   [Đã xác thực]   tổng hàng = {}", don_xac_thuc.tong_tien());

    // ---- LÕI THUẦN TÚY: lập hóa đơn (không I/O, kiểm thử được ngay) ----
    let hoa_don = lap_hoa_don(&don_xac_thuc);
    println!("   ┌─ HÓA ĐƠN (tính bởi LÕI THUẦN TÚY) ─────────────");
    println!("   │ Tạm tính        : {}", hoa_don.tam_tinh);
    println!("   │ Chiết khấu      : {}", hoa_don.chiet_khau);
    println!("   │ Phí vận chuyển  : {}", hoa_don.phi_van_chuyen);
    println!("   │ TỔNG THANH TOÁN : {}", hoa_don.tong_thanh_toan);
    println!("   └────────────────────────────────────────────────");

    let don_da_tra: DonHang<DaThanhToan> = don_xac_thuc.thanh_toan(ThanhToan::ChuyenKhoan {
        ma_giao_dich: "VCB-99881234".to_string(),
    });
    println!("   [Đã thanh toán] cách trả = {:?}", don_da_tra.cach_thanh_toan());

    let _don_da_giao: DonHang<DaGiao> = don_da_tra.giao_hang("VN-EXP-77213");
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
    let email = Email::phan_tich("test@shop.vn").unwrap();
    let don_rong: DonHang<Nhap> = DonHang::moi("ORD-0003", email, vec![]);
    match don_rong.xac_thuc() {
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
mod kiem_thu {
    use super::*;

    fn don_mau() -> DonHang<DaXacThuc> {
        let email = Email::phan_tich("khach@shop.vn").unwrap();
        let dong = vec![
            DongHang {
                ten: TenSanPham::phan_tich("Bàn phím").unwrap(),
                so_luong: SoLuong::phan_tich(2).unwrap(),
                don_gia: SoTien::dong(100_000),
            },
            DongHang {
                ten: TenSanPham::phan_tich("Chuột").unwrap(),
                so_luong: SoLuong::phan_tich(1).unwrap(),
                don_gia: SoTien::dong(50_000),
            },
        ];
        DonHang::moi("ORD-TEST", email, dong).xac_thuc().unwrap()
    }

    #[test]
    fn email_chap_nhan_dia_chi_hop_le() {
        let e = Email::phan_tich("  An.Nguyen@Example.COM ").unwrap();
        assert_eq!(e.as_str(), "an.nguyen@example.com"); // đã chuẩn hóa
    }

    #[test]
    fn email_tu_choi_dia_chi_sai() {
        for xau in ["", "   ", "khong-co-a-cong", "@thieu-ten.vn", "a@b@c.vn", "a@khongcocham"] {
            assert!(Email::phan_tich(xau).is_err(), "phải từ chối {:?}", xau);
        }
    }

    #[test]
    fn so_luong_phai_duong_va_trong_gioi_han() {
        assert!(SoLuong::phan_tich(0).is_err());
        assert!(SoLuong::phan_tich(1001).is_err());
        assert_eq!(SoLuong::phan_tich(5).unwrap().gia_tri(), 5);
    }

    #[test]
    fn ten_san_pham_dem_ky_tu_khong_dem_byte() {
        // 50 chữ cái tiếng Việt có dấu = nhiều hơn 50 BYTE, nhưng vẫn hợp lệ.
        let ten_dai: String = "ế".repeat(50);
        assert!(TenSanPham::phan_tich(&ten_dai).is_ok());
        let qua_dai: String = "ế".repeat(51);
        assert!(TenSanPham::phan_tich(&qua_dai).is_err());
    }

    #[test]
    fn don_rong_bi_tu_choi() {
        let email = Email::phan_tich("a@b.vn").unwrap();
        let don = DonHang::moi("X", email, vec![]);
        assert_eq!(don.xac_thuc().unwrap_err(), LoiMien::DonRong);
    }

    #[test]
    fn dto_gom_tat_ca_loi_cung_luc() {
        let dto = DonHangDto {
            ma: "X".to_string(),
            email: "sai".to_string(),
            dong: vec![DongHangDto { ten: "".to_string(), so_luong: 0, don_gia: 1 }],
        };
        let loi = DonHang::try_from(dto).unwrap_err();
        assert_eq!(loi.len(), 3, "phải gom đủ 3 lỗi, nhận được {:?}", loi);
    }

    // ---- Kiểm thử LÕI THUẦN TÚY: không cần CSDL, không cần mạng ----

    #[test]
    fn tong_tien_cong_dung_thanh_tien_tung_dong() {
        let don = don_mau();
        // 2 × 100.000 + 1 × 50.000 = 250.000
        assert_eq!(don.tong_tien(), SoTien::dong(250_000));
    }

    #[test]
    fn phi_van_chuyen_mien_phi_tu_500k() {
        assert_eq!(tinh_phi_van_chuyen(SoTien::dong(499_999)), SoTien::dong(30_000));
        assert_eq!(tinh_phi_van_chuyen(SoTien::dong(500_000)), SoTien::dong(0));
    }

    #[test]
    fn chiet_khau_theo_bac_so_dong() {
        let tong = SoTien::dong(1_000_000);
        assert_eq!(tinh_chiet_khau(tong, 3), SoTien::dong(0));
        assert_eq!(tinh_chiet_khau(tong, 5), SoTien::dong(50_000));
        assert_eq!(tinh_chiet_khau(tong, 12), SoTien::dong(100_000));
    }

    #[test]
    fn hoa_don_tinh_dung_toan_bo() {
        let don = don_mau(); // tạm tính 250.000, 2 dòng -> không chiết khấu
        let hd = lap_hoa_don(&don);
        assert_eq!(hd.tam_tinh, SoTien::dong(250_000));
        assert_eq!(hd.chiet_khau, SoTien::dong(0));
        assert_eq!(hd.phi_van_chuyen, SoTien::dong(30_000));
        assert_eq!(hd.tong_thanh_toan, SoTien::dong(280_000));
    }

    #[test]
    fn quy_trinh_typestate_chay_het_bon_buoc() {
        let don = don_mau();
        let da_tra = don.thanh_toan(ThanhToan::TienMat);
        assert_eq!(da_tra.cach_thanh_toan(), &ThanhToan::TienMat);
        let da_giao = da_tra.giao_hang("VD-001");
        assert_eq!(da_giao.ma(), "ORD-TEST");
    }

    #[test]
    fn typestate_khong_ton_bo_nho_luc_chay() {
        use std::mem::size_of;
        // PhantomData chiếm 0 byte: DonHang<Nhap> và DonHang<DaGiao> có cùng kích thước.
        assert_eq!(size_of::<DonHang<Nhap>>(), size_of::<DonHang<DaGiao>>());
        assert_eq!(size_of::<Nhap>(), 0);
        assert_eq!(size_of::<PhantomData<DaGiao>>(), 0);
    }
}
