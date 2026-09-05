#![allow(dead_code, unused_variables)]
//! Chương 55 — Kim tự tháp Kiểm thử: Unit, Integration, E2E, TDD, BDD, Property, Doctest.

// ============================================================================
// PHẦN 1: MIỀN NGHIỆP VỤ ĐƯỢC PHÁT TRIỂN THEO TDD (Red → Green → Refactor)
// ============================================================================

/// Giỏ hàng — ta sẽ "viết test trước, code sau" cho từng hành vi.
#[derive(Debug, Clone, PartialEq)]
pub struct GioHang {
    mat_hang: Vec<(String, u64, u32)>, // (tên, đơn giá, số lượng)
}

#[derive(Debug, PartialEq, Eq)]
pub enum LoiGio {
    SoLuongBangKhong,
    KhongTonTai,
}

impl GioHang {
    pub fn moi() -> Self {
        GioHang { mat_hang: Vec::new() }
    }

    /// Thêm mặt hàng. Số lượng 0 là lỗi nghiệp vụ (không phải panic).
    pub fn them(&mut self, ten: &str, don_gia: u64, so_luong: u32) -> Result<(), LoiGio> {
        if so_luong == 0 {
            return Err(LoiGio::SoLuongBangKhong);
        }
        // Nếu đã có, cộng dồn số lượng thay vì tạo dòng mới
        if let Some(dong) = self.mat_hang.iter_mut().find(|(t, _, _)| t == ten) {
            dong.2 += so_luong;
        } else {
            self.mat_hang.push((ten.to_string(), don_gia, so_luong));
        }
        Ok(())
    }

    /// Tổng tiền, tính bằng đơn vị nhỏ nhất (đồng) — KHÔNG dùng f64.
    ///
    /// # Ví dụ (đây cũng là một DOCTEST — chạy khi `cargo test`)
    /// ```
    /// # use ch55::GioHang;
    /// let mut gio = GioHang::moi();
    /// gio.them("Sách", 45_000, 2).unwrap();
    /// gio.them("Bút", 5_000, 3).unwrap();
    /// assert_eq!(gio.tong_tien(), 105_000);
    /// ```
    pub fn tong_tien(&self) -> u64 {
        self.mat_hang.iter().map(|(_, gia, sl)| gia * *sl as u64).sum()
    }

    pub fn so_dong(&self) -> usize {
        self.mat_hang.len()
    }

    /// Áp mã giảm giá phần trăm (0..=100).
    pub fn sau_giam_gia(&self, phan_tram: u32) -> u64 {
        let tong = self.tong_tien();
        let pt = phan_tram.min(100) as u64;
        tong - tong * pt / 100
    }
}

// ============================================================================
// PHẦN 2: TEST DOUBLE (MOCK/FAKE) BẰNG TRAIT — không cần thư viện ngoài
// ============================================================================

/// Cổng thanh toán là một PHỤ THUỘC. Trong test ta thay nó bằng bản giả.
pub trait CongThanhToan {
    fn tru_tien(&self, so_tien: u64) -> Result<String, String>;
}

/// Bản thật (chỉ mô phỏng, không gọi mạng thật ở đây).
pub struct CongThat;
impl CongThanhToan for CongThat {
    fn tru_tien(&self, so_tien: u64) -> Result<String, String> {
        Ok(format!("TXN-THAT-{}", so_tien))
    }
}

/// Hàm nghiệp vụ nhận phụ thuộc qua trait (tiêm phụ thuộc, Chương 14).
pub fn thanh_toan_gio(
    gio: &GioHang,
    cong: &dyn CongThanhToan,
    giam_gia: u32,
) -> Result<String, String> {
    let so_tien = gio.sau_giam_gia(giam_gia);
    if so_tien == 0 {
        return Err("Giỏ rỗng hoặc miễn phí, không cần thanh toán".to_string());
    }
    cong.tru_tien(so_tien)
}

// ============================================================================
// PHẦN 3: MÁY TRẠNG THÁI ĐỂ DEMO KIỂM THỬ THEO TÍNH CHẤT (PROPERTY-BASED)
// ============================================================================

/// Bộ sinh giả ngẫu nhiên tất định (LCG) — giống Chương 18, không cần crate.
pub struct BoSinh(u64);
impl BoSinh {
    pub fn moi(hat: u64) -> Self { BoSinh(hat) }
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
    fn gio_moi_thi_rong() {
        let gio = GioHang::moi();
        assert_eq!(gio.so_dong(), 0);
        assert_eq!(gio.tong_tien(), 0);
    }

    #[test]
    fn them_mat_hang_tinh_dung_tong() {
        let mut gio = GioHang::moi();
        gio.them("A", 10_000, 3).unwrap();
        assert_eq!(gio.tong_tien(), 30_000);
    }

    #[test]
    fn them_cung_ten_thi_cong_don_so_luong() {
        let mut gio = GioHang::moi();
        gio.them("A", 10_000, 1).unwrap();
        gio.them("A", 10_000, 2).unwrap();
        assert_eq!(gio.so_dong(), 1, "phải gộp thành 1 dòng");
        assert_eq!(gio.tong_tien(), 30_000);
    }

    #[test]
    fn so_luong_bang_khong_la_loi_khong_phai_panic() {
        let mut gio = GioHang::moi();
        assert_eq!(gio.them("A", 10_000, 0), Err(LoiGio::SoLuongBangKhong));
        assert_eq!(gio.so_dong(), 0); // không thêm gì
    }

    #[test]
    fn giam_gia_vuot_100_bi_ghim_ve_100() {
        let mut gio = GioHang::moi();
        gio.them("A", 100_000, 1).unwrap();
        assert_eq!(gio.sau_giam_gia(200), 0); // ghim ở 100%, không âm
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
    struct CongGianDiep {
        da_goi_voi: RefCell<Vec<u64>>,
    }
    impl CongThanhToan for CongGianDiep {
        fn tru_tien(&self, so_tien: u64) -> Result<String, String> {
            self.da_goi_voi.borrow_mut().push(so_tien);
            Ok("TXN-GIA".to_string())
        }
    }

    /// STUB: cổng giả luôn báo lỗi, để test nhánh thất bại.
    struct CongLuonHong;
    impl CongThanhToan for CongLuonHong {
        fn tru_tien(&self, _: u64) -> Result<String, String> {
            Err("Thẻ bị từ chối".to_string())
        }
    }

    #[test]
    fn thanh_toan_goi_cong_dung_so_tien_sau_giam_gia() {
        let mut gio = GioHang::moi();
        gio.them("A", 100_000, 1).unwrap();
        let spy = CongGianDiep { da_goi_voi: RefCell::new(vec![]) };

        thanh_toan_gio(&gio, &spy, 20).unwrap(); // giảm 20% -> 80.000

        assert_eq!(*spy.da_goi_voi.borrow(), vec![80_000], "phải trừ đúng số sau giảm giá");
    }

    #[test]
    fn thanh_toan_lan_truyen_loi_tu_cong() {
        let mut gio = GioHang::moi();
        gio.them("A", 100_000, 1).unwrap();
        assert_eq!(thanh_toan_gio(&gio, &CongLuonHong, 0), Err("Thẻ bị từ chối".to_string()));
    }

    #[test]
    fn gio_rong_khong_goi_cong_thanh_toan() {
        let gio = GioHang::moi();
        let spy = CongGianDiep { da_goi_voi: RefCell::new(vec![]) };
        let kq = thanh_toan_gio(&gio, &spy, 0);
        assert!(kq.is_err());
        assert!(spy.da_goi_voi.borrow().is_empty(), "cổng KHÔNG được gọi khi giỏ rỗng");
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
    fn giam_gia_luon_nam_trong_khoang_0_va_tong() {
        let mut sinh = BoSinh::moi(2026);
        for _ in 0..2000 {
            let mut gio = GioHang::moi();
            let so_mat_hang = sinh.so(5) + 1;
            for i in 0..so_mat_hang {
                let _ = gio.them(&format!("SP{}", i), (sinh.so(100_000) + 1) as u64, sinh.so(5) + 1);
            }
            let pt = sinh.so(150); // cố tình cho vượt 100
            let sau = gio.sau_giam_gia(pt);
            // TÍNH CHẤT: giá sau giảm luôn trong [0, tổng]
            assert!(sau <= gio.tong_tien(), "giảm giá không được làm TĂNG tiền");
        }
    }

    #[test]
    fn giam_0_phan_tram_bang_dung_tong() {
        let mut sinh = BoSinh::moi(7);
        for _ in 0..1000 {
            let mut gio = GioHang::moi();
            gio.them("X", (sinh.so(50_000) + 1) as u64, sinh.so(9) + 1).unwrap();
            // TÍNH CHẤT: giảm 0% là phép đồng nhất
            assert_eq!(gio.sau_giam_gia(0), gio.tong_tien());
        }
    }

    #[test]
    fn them_roi_lai_bang_tong_cac_phan() {
        let mut sinh = BoSinh::moi(99);
        for _ in 0..1000 {
            let (g1, sl1) = ((sinh.so(1000) + 1) as u64, sinh.so(9) + 1);
            let (g2, sl2) = ((sinh.so(1000) + 1) as u64, sinh.so(9) + 1);
            let mut gio = GioHang::moi();
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

    struct CongOk(RefCell<Vec<u64>>);
    impl CongThanhToan for CongOk {
        fn tru_tien(&self, s: u64) -> Result<String, String> { self.0.borrow_mut().push(s); Ok("OK".into()) }
    }

    /// Kịch bản: "Khách VIP mua hàng và được giảm 15%".
    #[test]
    fn khach_vip_duoc_giam_15_phan_tram() {
        // GIVEN — một giỏ hàng trị giá 1.000.000đ và một cổng thanh toán
        let mut gio = GioHang::moi();
        gio.them("Tai nghe", 1_000_000, 1).unwrap();
        let cong = CongOk(RefCell::new(vec![]));

        // WHEN — khách VIP (giảm 15%) thanh toán
        let ket_qua = thanh_toan_gio(&gio, &cong, 15);

        // THEN — thanh toán thành công và số tiền bị trừ đúng 850.000đ
        assert!(ket_qua.is_ok());
        assert_eq!(*cong.0.borrow(), vec![850_000]);
    }

    /// Kịch bản: "Không thể thanh toán một giỏ hàng rỗng".
    #[test]
    fn khong_the_thanh_toan_gio_rong() {
        // GIVEN — một giỏ hàng rỗng
        let gio = GioHang::moi();
        let cong = CongOk(RefCell::new(vec![]));

        // WHEN — cố gắng thanh toán
        let ket_qua = thanh_toan_gio(&gio, &cong, 0);

        // THEN — hệ thống từ chối và không gọi cổng thanh toán
        assert!(ket_qua.is_err());
        assert!(cong.0.borrow().is_empty());
    }
}
