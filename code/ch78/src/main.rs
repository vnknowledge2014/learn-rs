#![allow(dead_code)]
//! Chương 78 — Thị trường blockchain: bể thanh khoản tích không đổi, trượt giá,
//! tổn thất tạm thời, tấn công kẹp (sandwich), và arbitrage giữa sàn tập trung
//! với sàn phi tập trung.
//!
//! Khác biệt cốt lõi so với thị trường truyền thống (Chương 75–77): ở đây
//! **mọi giao dịch đều công khai TRƯỚC khi được thực thi**. Ai cũng đọc được
//! hàng chờ, và ai trả phí cao hơn thì được xếp trước. Đó là mảnh đất của MEV.
//!
//! ⚠️ Đây là tài liệu KỸ THUẬT nhằm giúp người đọc TỰ BẢO VỆ và hiểu rủi ro,
//! không phải hướng dẫn khai thác người dùng khác.

// ============================================================================
// 1. BỂ THANH KHOẢN TÍCH KHÔNG ĐỔI
// ============================================================================
// Toàn bộ Uniswap v2 gói gọn trong một bất biến: x · y = k.
// Không sổ lệnh, không người khớp lệnh, không ai phải chờ đối tác.
// Giá được suy ra từ tỉ lệ dự trữ, và tự động điều chỉnh sau mỗi giao dịch.

pub type SoLuong = u128;

#[derive(Debug, Clone, Copy, PartialEq)]
pub struct BeThanhKhoan {
    pub du_tru_x: SoLuong,
    pub du_tru_y: SoLuong,
    /// Phí tính theo phần vạn: 30 = 0,30%
    pub phi_phan_van: u32,
}

#[derive(Debug, PartialEq)]
pub enum LoiHoanDoi {
    DauVaoBangKhong,
    BeRong,
    ThanhKhoanKhongDu,
    /// Người dùng đặt sàn nhận tối thiểu, mà kết quả thấp hơn → huỷ giao dịch.
    TruotGiaVuotChoPhep { nhan_duoc: SoLuong, toi_thieu: SoLuong },
}

impl BeThanhKhoan {
    pub fn moi(x: SoLuong, y: SoLuong, phi_phan_van: u32) -> Self {
        BeThanhKhoan { du_tru_x: x, du_tru_y: y, phi_phan_van }
    }

    /// Hằng số bất biến. Nó chỉ được TĂNG (nhờ phí), không bao giờ giảm.
    pub fn k(&self) -> u128 { self.du_tru_x * self.du_tru_y }

    /// Giá hiện thời của X tính theo Y, dạng số thực (chỉ để hiển thị).
    pub fn gia_x(&self) -> f64 {
        if self.du_tru_x == 0 { return 0.0; }
        self.du_tru_y as f64 / self.du_tru_x as f64
    }

    /// Tính lượng Y nhận được khi đưa vào `vao_x`, KHÔNG thay đổi bể.
    ///
    /// Công thức: dy = (y · dx') / (x + dx') với dx' = dx · (1 − phí).
    /// Toàn bộ tính bằng số nguyên — tiền không bao giờ dùng dấu phẩy động.
    pub fn thu_hoan_doi_x_lay_y(&self, vao_x: SoLuong) -> Result<SoLuong, LoiHoanDoi> {
        if vao_x == 0 { return Err(LoiHoanDoi::DauVaoBangKhong); }
        if self.du_tru_x == 0 || self.du_tru_y == 0 { return Err(LoiHoanDoi::BeRong); }
        let sau_phi = vao_x * (10_000 - self.phi_phan_van as u128);
        let tu = self.du_tru_y * sau_phi;
        let mau = self.du_tru_x * 10_000 + sau_phi;
        let ra = tu / mau;
        if ra == 0 || ra >= self.du_tru_y { return Err(LoiHoanDoi::ThanhKhoanKhongDu); }
        Ok(ra)
    }

    /// Thực hiện hoán đổi, có kiểm tra sàn nhận tối thiểu.
    /// `toi_thieu_y` chính là "bảo vệ trượt giá" mà ví hiển thị cho bạn.
    pub fn hoan_doi_x_lay_y(&mut self, vao_x: SoLuong, toi_thieu_y: SoLuong)
        -> Result<SoLuong, LoiHoanDoi>
    {
        let ra = self.thu_hoan_doi_x_lay_y(vao_x)?;
        if ra < toi_thieu_y {
            return Err(LoiHoanDoi::TruotGiaVuotChoPhep { nhan_duoc: ra, toi_thieu: toi_thieu_y });
        }
        self.du_tru_x += vao_x;
        self.du_tru_y -= ra;
        Ok(ra)
    }

    pub fn thu_hoan_doi_y_lay_x(&self, vao_y: SoLuong) -> Result<SoLuong, LoiHoanDoi> {
        let dao = BeThanhKhoan { du_tru_x: self.du_tru_y, du_tru_y: self.du_tru_x,
                                 phi_phan_van: self.phi_phan_van };
        dao.thu_hoan_doi_x_lay_y(vao_y)
    }

    pub fn hoan_doi_y_lay_x(&mut self, vao_y: SoLuong, toi_thieu_x: SoLuong)
        -> Result<SoLuong, LoiHoanDoi>
    {
        let ra = self.thu_hoan_doi_y_lay_x(vao_y)?;
        if ra < toi_thieu_x {
            return Err(LoiHoanDoi::TruotGiaVuotChoPhep { nhan_duoc: ra, toi_thieu: toi_thieu_x });
        }
        self.du_tru_y += vao_y;
        self.du_tru_x -= ra;
        Ok(ra)
    }

    /// Trượt giá: chênh lệch giữa giá thực nhận và giá niêm yết trước giao dịch.
    /// Đây KHÔNG phải phí — nó là hệ quả toán học của đường cong x·y = k,
    /// và nó lớn dần theo quy mô giao dịch so với bể.
    pub fn truot_gia(&self, vao_x: SoLuong) -> Option<f64> {
        let ra = self.thu_hoan_doi_x_lay_y(vao_x).ok()?;
        let gia_niem_yet = self.gia_x();
        let gia_thuc = ra as f64 / vao_x as f64;
        Some((gia_niem_yet - gia_thuc) / gia_niem_yet)
    }
}

// ============================================================================
// 2. TỔN THẤT TẠM THỜI — cái giá của việc làm nhà cung cấp thanh khoản
// ============================================================================

/// Khi giá đổi theo hệ số `r`, giá trị phần vốn góp so với việc CHỈ NẮM GIỮ là:
///
///     2·√r / (1 + r) − 1
///
/// Luôn ≤ 0, và bằng 0 chỉ khi r = 1 (giá không đổi). Nghĩa là: giá càng
/// biến động, người góp vốn càng thiệt so với người chỉ ngồi im — và phí thu
/// được phải bù nổi khoản đó thì góp vốn mới có lãi.
///
/// Chữ "tạm thời" gây hiểu lầm: nó chỉ tạm thời nếu giá QUAY VỀ mức cũ.
/// Không quay về thì nó vĩnh viễn.
pub fn ton_that_tam_thoi(ty_le_gia: f64) -> f64 {
    if ty_le_gia <= 0.0 { return 0.0; }
    2.0 * ty_le_gia.sqrt() / (1.0 + ty_le_gia) - 1.0
}

// ============================================================================
// 3. HÀNG CHỜ CÔNG KHAI & TẤN CÔNG KẸP
// ============================================================================
// Trên blockchain, giao dịch nằm trong hàng chờ CÔNG KHAI trước khi vào khối,
// và người xây khối sắp xếp theo phí ưu tiên. Ai trả cao hơn được xếp trước.
// Hệ quả: bất kỳ ai cũng thấy trước bạn định làm gì, và chen lên trước được.

#[derive(Debug, Clone, PartialEq)]
pub struct GiaoDichCho {
    pub nguoi_gui: String,
    pub vao_x: SoLuong,
    pub toi_thieu_y: SoLuong,
    /// Phí ưu tiên — con số quyết định thứ tự trong khối.
    pub phi_uu_tien: u64,
}

/// Người xây khối sắp xếp theo phí ưu tiên GIẢM DẦN. Đây là toàn bộ cơ chế
/// khiến MEV tồn tại: thứ tự không theo thời gian tới, mà theo số tiền trả.
pub fn sap_xep_khoi(mut cho: Vec<GiaoDichCho>) -> Vec<GiaoDichCho> {
    // `sort_by` của Rust là sắp xếp ỔN ĐỊNH → phí bằng nhau thì giữ nguyên
    // thứ tự, nên kết quả tất định và kiểm thử được.
    cho.sort_by(|a, b| b.phi_uu_tien.cmp(&a.phi_uu_tien));
    cho
}

#[derive(Debug, PartialEq)]
pub struct KetQuaKep {
    /// Nạn nhân nhận được bao nhiêu khi KHÔNG bị kẹp.
    pub nhan_neu_khong_bi_kep: SoLuong,
    /// Nạn nhân nhận được bao nhiêu khi BỊ kẹp.
    pub nhan_khi_bi_kep: SoLuong,
    pub ke_tan_cong_lai: i128,
    /// Giao dịch của nạn nhân có bị chặn nhờ sàn nhận tối thiểu không.
    pub bi_chan_boi_bao_ve: bool,
}

/// Mô phỏng một cú kẹp để thấy **vì sao phải đặt sàn nhận tối thiểu chặt**.
///
/// Kịch bản: kẻ tấn công thấy giao dịch của nạn nhân trong hàng chờ, trả phí
/// cao hơn để mua TRƯỚC (đẩy giá lên), để nạn nhân mua ở giá xấu, rồi bán
/// NGAY SAU đó ăn chênh lệch.
pub fn mo_phong_kep(be: &BeThanhKhoan, nan_nhan: &GiaoDichCho, von_tan_cong: SoLuong)
    -> KetQuaKep
{
    // (a) Nếu không ai chen ngang
    let sach = be.thu_hoan_doi_x_lay_y(nan_nhan.vao_x).unwrap_or(0);

    // (b) Có kẻ chen ngang, mua trước để đẩy giá
    let mut b = *be;
    let ra_truoc = b.hoan_doi_x_lay_y(von_tan_cong, 0).unwrap_or(0);

    let nhan_khi_bi_kep = b.thu_hoan_doi_x_lay_y(nan_nhan.vao_x).unwrap_or(0);
    // ĐÂY là chỗ sàn nhận tối thiểu cứu nạn nhân: giao dịch bị huỷ, không mất vốn
    let bi_chan = nhan_khi_bi_kep < nan_nhan.toi_thieu_y;
    if !bi_chan {
        let _ = b.hoan_doi_x_lay_y(nan_nhan.vao_x, nan_nhan.toi_thieu_y);
    }

    // (c) Kẻ tấn công bán lại phần vừa mua
    let thu_ve = if bi_chan { 0 } else { b.thu_hoan_doi_y_lay_x(ra_truoc).unwrap_or(0) };
    let lai = if bi_chan { 0 } else { thu_ve as i128 - von_tan_cong as i128 };

    KetQuaKep {
        nhan_neu_khong_bi_kep: sach,
        nhan_khi_bi_kep: if bi_chan { 0 } else { nhan_khi_bi_kep },
        ke_tan_cong_lai: lai,
        bi_chan_boi_bao_ve: bi_chan,
    }
}

/// Tính sàn nhận tối thiểu từ mức trượt giá chấp nhận được (phần vạn).
/// Đặt 5% "cho chắc ăn" chính là mời kẻ tấn công lấy đúng 5% đó.
pub fn san_nhan_toi_thieu(du_kien: SoLuong, cho_phep_phan_van: u32) -> SoLuong {
    du_kien * (10_000 - cho_phep_phan_van as u128) / 10_000
}

// ============================================================================
// 4. ARBITRAGE GIỮA SÀN TẬP TRUNG VÀ SÀN PHI TẬP TRUNG
// ============================================================================

#[derive(Debug, PartialEq)]
pub struct CoHoiArb {
    pub co_co_hoi: bool,
    pub khoi_luong_toi_uu: SoLuong,
    pub lai_uoc_tinh: i128,
    pub gia_dex_truoc: f64,
    pub gia_dex_sau: f64,
}

/// Tìm khối lượng hoán đổi tối ưu để kéo giá sàn phi tập trung về sát giá sàn
/// tập trung, và tính lãi ước tính sau phí.
///
/// Dùng tìm kiếm tam phân trên hàm lãi thay vì giải công thức đóng: đường cong
/// có phí và có làm tròn số nguyên, nên công thức đóng lệch với thực tế. Tìm
/// kiếm trên chính hàm sẽ thực thi thì luôn khớp với những gì xảy ra trên chuỗi.
pub fn tim_co_hoi_arb(be: &BeThanhKhoan, gia_cex: f64, von_toi_da: SoLuong) -> CoHoiArb {
    let gia_truoc = be.gia_x();
    let khong_co = CoHoiArb { co_co_hoi: false, khoi_luong_toi_uu: 0, lai_uoc_tinh: 0,
                              gia_dex_truoc: gia_truoc, gia_dex_sau: gia_truoc };
    // Chỉ xét chiều: mua X trên DEX (đưa Y vào) khi X trên DEX RẺ hơn CEX
    if gia_truoc >= gia_cex || von_toi_da == 0 { return khong_co; }

    let lai_khi = |vao_y: SoLuong| -> i128 {
        match be.thu_hoan_doi_y_lay_x(vao_y) {
            // Nhận `ra_x` đơn vị X, bán trên CEX được ra_x · gia_cex đơn vị Y
            Ok(ra_x) => (ra_x as f64 * gia_cex) as i128 - vao_y as i128,
            Err(_) => i128::MIN,
        }
    };

    // Hàm lãi lõm theo khối lượng → tìm kiếm tam phân
    let (mut lo, mut hi) = (1u128, von_toi_da);
    for _ in 0..200 {
        if hi <= lo + 2 { break; }
        let m1 = lo + (hi - lo) / 3;
        let m2 = hi - (hi - lo) / 3;
        if lai_khi(m1) < lai_khi(m2) { lo = m1 + 1; } else { hi = m2 - 1; }
    }
    let mut tot_nhat = (lo, lai_khi(lo));
    let mut v = lo;
    while v <= hi && v <= lo + 8 {
        let l = lai_khi(v);
        if l > tot_nhat.1 { tot_nhat = (v, l); }
        v += 1;
    }

    if tot_nhat.1 <= 0 { return khong_co; }
    let mut sau = *be;
    let _ = sau.hoan_doi_y_lay_x(tot_nhat.0, 0);
    CoHoiArb {
        co_co_hoi: true,
        khoi_luong_toi_uu: tot_nhat.0,
        lai_uoc_tinh: tot_nhat.1,
        gia_dex_truoc: gia_truoc,
        gia_dex_sau: sau.gia_x(),
    }
}

fn main() {
    println!("═══════════════════════════════════════════════════════════");
    println!("   THỊ TRƯỜNG BLOCKCHAIN: AMM · TRƯỢT GIÁ · MEV            ");
    println!("═══════════════════════════════════════════════════════════");

    println!("\n1. BỂ THANH KHOẢN TÍCH KHÔNG ĐỔI");
    let be = BeThanhKhoan::moi(1_000_000, 2_000_000_000, 30);
    println!("   Dự trữ: {} X · {} Y", be.du_tru_x, be.du_tru_y);
    println!("   Giá niêm yết: 1 X = {:.2} Y · k = {}", be.gia_x(), be.k());

    println!("\n2. TRƯỢT GIÁ TĂNG THEO QUY MÔ GIAO DỊCH");
    println!("   {:>10} {:>16} {:>14} {:>10}", "đưa vào X", "nhận được Y", "giá thực", "trượt giá");
    for vao in [100u128, 1_000, 10_000, 100_000, 500_000] {
        let ra = be.thu_hoan_doi_x_lay_y(vao).unwrap();
        println!("   {:>10} {:>16} {:>14.2} {:>9.2}%",
                 vao, ra, ra as f64 / vao as f64, be.truot_gia(vao).unwrap() * 100.0);
    }
    println!("   → Giao dịch bằng 50% bể mất tới {:.0}% giá trị. Đây KHÔNG phải phí,",
             be.truot_gia(500_000).unwrap() * 100.0);
    println!("     mà là hình dạng của chính đường cong x·y = k.");

    println!("\n3. PHÍ LÀM HẰNG SỐ k LỚN DẦN — đó là lãi của người góp vốn");
    let mut b2 = be;
    let k_dau = b2.k();
    for _ in 0..10 { b2.hoan_doi_x_lay_y(10_000, 0).unwrap(); }
    println!("   k trước: {} · sau 10 lần hoán đổi: {}", k_dau, b2.k());
    println!("   k tăng {:.4}% — phần đó thuộc về người góp vốn.",
             (b2.k() as f64 / k_dau as f64 - 1.0) * 100.0);

    println!("\n4. TỔN THẤT TẠM THỜI");
    println!("   {:>14} {:>18}", "giá đổi", "so với chỉ nắm giữ");
    for r in [0.25f64, 0.5, 0.8, 1.0, 1.25, 2.0, 4.0, 10.0] {
        println!("   {:>13.2}x {:>17.2}%", r, ton_that_tam_thoi(r) * 100.0);
    }
    println!("   → Luôn ≤ 0, chỉ bằng 0 khi giá không đổi. Phí thu được phải bù nổi");
    println!("     khoản này thì góp vốn mới thật sự có lãi.");

    println!("\n5. HÀNG CHỜ CÔNG KHAI — phí quyết định thứ tự, không phải thời gian tới");
    let cho = vec![
        GiaoDichCho { nguoi_gui: "nguoi-dung-thuong".into(), vao_x: 50_000,
                      toi_thieu_y: 0, phi_uu_tien: 2 },
        GiaoDichCho { nguoi_gui: "bot-chen-truoc".into(), vao_x: 30_000,
                      toi_thieu_y: 0, phi_uu_tien: 500 },
        GiaoDichCho { nguoi_gui: "nguoi-kien-nhan".into(), vao_x: 1_000,
                      toi_thieu_y: 0, phi_uu_tien: 1 },
    ];
    for (i, g) in sap_xep_khoi(cho).iter().enumerate() {
        println!("   #{} {:<20} phí ưu tiên {}", i + 1, g.nguoi_gui, g.phi_uu_tien);
    }

    println!("\n6. TẤN CÔNG KẸP — và cách sàn nhận tối thiểu cứu bạn");
    let du_kien = be.thu_hoan_doi_x_lay_y(50_000).unwrap();
    println!("   Nạn nhân định đổi 50 000 X, dự kiến nhận {} Y", du_kien);
    for cho_phep in [5_000u32, 1_000, 100, 50] {
        let sn = san_nhan_toi_thieu(du_kien, cho_phep);
        let nn = GiaoDichCho { nguoi_gui: "nan-nhan".into(), vao_x: 50_000,
                               toi_thieu_y: sn, phi_uu_tien: 1 };
        let kq = mo_phong_kep(&be, &nn, 200_000);
        if kq.bi_chan_boi_bao_ve {
            println!("   cho phép trượt {:>4.1}% → GIAO DỊCH BỊ HUỶ, nạn nhân không mất vốn",
                     cho_phep as f64 / 100.0);
        } else {
            let mat = kq.nhan_neu_khong_bi_kep - kq.nhan_khi_bi_kep;
            println!("   cho phép trượt {:>4.1}% → nạn nhân mất {:>8} Y · kẻ tấn công lãi {:>8}",
                     cho_phep as f64 / 100.0, mat, kq.ke_tan_cong_lai);
        }
    }
    println!("   → Đặt 5% \"cho chắc ăn\" chính là công khai mời người khác lấy 5% đó.");

    println!("\n7. ARBITRAGE CEX ↔ DEX");
    let lech = BeThanhKhoan::moi(1_000_000, 1_900_000_000, 30); // DEX rẻ hơn
    let gia_cex = 2_000.0;
    println!("   Giá DEX {:.2} · giá CEX {:.2} → lệch {:.2}%",
             lech.gia_x(), gia_cex, (gia_cex / lech.gia_x() - 1.0) * 100.0);
    let ch = tim_co_hoi_arb(&lech, gia_cex, 500_000_000);
    if ch.co_co_hoi {
        println!("   Khối lượng tối ưu: {} Y → lãi ước tính {} Y",
                 ch.khoi_luong_toi_uu, ch.lai_uoc_tinh);
        println!("   Giá DEX sau giao dịch: {:.2} (đã kéo về gần CEX)", ch.gia_dex_sau);
    }
    println!("   → Chính đội arbitrage giữ cho giá DEX bám sát thị trường.");
    println!("     Họ không làm từ thiện — họ được trả công bằng khoảng lệch đó.");

    println!("\n═══════════════════════════════════════════════════════════");
    println!("   HÀNG CHỜ CÔNG KHAI = MỌI Ý ĐỊNH ĐỀU BỊ ĐỌC TRƯỚC         ");
    println!("═══════════════════════════════════════════════════════════");
}

#[cfg(test)]
mod kiem_thu {
    use super::*;

    fn be_mau() -> BeThanhKhoan { BeThanhKhoan::moi(1_000_000, 2_000_000_000, 30) }

    // ---------- Bể thanh khoản ----------
    #[test]
    fn gia_suy_ra_tu_ty_le_du_tru() {
        let b = be_mau();
        assert!((b.gia_x() - 2_000.0).abs() < 1e-9);
        assert_eq!(BeThanhKhoan::moi(0, 100, 30).gia_x(), 0.0, "bể rỗng không chia cho 0");
    }

    #[test]
    fn hoan_doi_lam_hang_so_k_tang_chu_khong_giam() {
        // Bất biến sống còn của AMM: phí làm k lớn dần, và đó chính là
        // phần lãi thuộc về người góp vốn.
        let mut b = be_mau();
        let mut k = b.k();
        for _ in 0..50 {
            b.hoan_doi_x_lay_y(10_000, 0).unwrap();
            let k_moi = b.k();
            assert!(k_moi >= k, "k giảm từ {} xuống {} — bể bị rút ruột", k, k_moi);
            k = k_moi;
        }
        assert!(b.k() > be_mau().k(), "sau 50 lần hoán đổi k phải lớn hơn hẳn");
    }

    #[test]
    fn khong_phi_thi_k_gan_nhu_khong_doi() {
        let mut b = BeThanhKhoan::moi(1_000_000, 2_000_000_000, 0);
        let k_dau = b.k();
        b.hoan_doi_x_lay_y(10_000, 0).unwrap();
        // Chỉ lệch do làm tròn số nguyên, không phải do phí
        let lech = (b.k() as f64 / k_dau as f64 - 1.0).abs();
        assert!(lech < 1e-4, "không phí thì k gần như đứng yên, lệch {:.6}", lech);
    }

    #[test]
    fn dua_vao_cang_nhieu_thi_nhan_cang_nhieu_nhung_kem_hieu_qua() {
        let b = be_mau();
        let mut gia_thuc_truoc = f64::MAX;
        let mut ra_truoc = 0u128;
        for vao in [100u128, 1_000, 10_000, 100_000] {
            let ra = b.thu_hoan_doi_x_lay_y(vao).unwrap();
            assert!(ra > ra_truoc, "đưa vào nhiều hơn phải nhận nhiều hơn");
            let gia_thuc = ra as f64 / vao as f64;
            assert!(gia_thuc < gia_thuc_truoc, "nhưng giá mỗi đơn vị phải TỆ dần");
            ra_truoc = ra;
            gia_thuc_truoc = gia_thuc;
        }
    }

    #[test]
    fn truot_gia_luon_duong_va_tang_theo_quy_mo() {
        let b = be_mau();
        let mut truoc = 0.0;
        for vao in [100u128, 1_000, 10_000, 100_000, 500_000] {
            let t = b.truot_gia(vao).unwrap();
            assert!(t > 0.0, "trượt giá luôn dương — bạn luôn nhận ít hơn giá niêm yết");
            assert!(t > truoc, "và tăng dần theo quy mô");
            truoc = t;
        }
        assert!(b.truot_gia(500_000).unwrap() > 0.3,
                "giao dịch bằng nửa bể phải mất hơn 30%");
    }

    #[test]
    fn khong_bao_gio_rut_can_duoc_be() {
        // Bất biến toán học của x·y = k: không lượng đầu vào hữu hạn nào lấy
        // hết được phía bên kia. Đây là điều khiến AMM không thể bị "vét sạch".
        let b = be_mau();
        for vao in [1_000_000u128, 10_000_000, 1_000_000_000] {
            match b.thu_hoan_doi_x_lay_y(vao) {
                Ok(ra) => assert!(ra < b.du_tru_y, "nhận {} mà bể chỉ có {}", ra, b.du_tru_y),
                Err(e) => assert_eq!(e, LoiHoanDoi::ThanhKhoanKhongDu),
            }
        }
    }

    #[test]
    fn hoan_doi_khong_hop_le_bi_tu_choi() {
        let b = be_mau();
        assert_eq!(b.thu_hoan_doi_x_lay_y(0), Err(LoiHoanDoi::DauVaoBangKhong));
        assert_eq!(BeThanhKhoan::moi(0, 0, 30).thu_hoan_doi_x_lay_y(100),
                   Err(LoiHoanDoi::BeRong));
    }

    #[test]
    fn du_tru_cap_nhat_dung_sau_hoan_doi() {
        let mut b = be_mau();
        let ra = b.hoan_doi_x_lay_y(10_000, 0).unwrap();
        assert_eq!(b.du_tru_x, 1_010_000, "X vào bể");
        assert_eq!(b.du_tru_y, 2_000_000_000 - ra, "Y ra khỏi bể");
    }

    #[test]
    fn hoan_doi_di_roi_ve_thi_lo_vi_tra_phi_hai_lan() {
        let mut b = be_mau();
        let y = b.hoan_doi_x_lay_y(10_000, 0).unwrap();
        assert!(y > 0);
        let x = b.hoan_doi_y_lay_x(y, 0).unwrap();
        assert!(x > 0);
        assert!(x < 10_000, "đổi đi rồi đổi lại phải LỖ, nhận về {} thay vì 10 000", x);
    }

    #[test]
    fn san_nhan_toi_thieu_chan_giao_dich_xau() {
        let mut b = be_mau();
        let du_kien = b.thu_hoan_doi_x_lay_y(10_000).unwrap();
        // Đòi nhiều hơn mức có thể → phải bị chặn, và bể KHÔNG được đổi
        let truoc = b;
        let e = b.hoan_doi_x_lay_y(10_000, du_kien + 1).unwrap_err();
        assert!(matches!(e, LoiHoanDoi::TruotGiaVuotChoPhep { .. }));
        assert_eq!(b, truoc, "giao dịch hỏng phải KHÔNG để lại thay đổi nào");
    }

    #[test]
    fn tinh_san_nhan_toi_thieu_dung() {
        assert_eq!(san_nhan_toi_thieu(1_000_000, 50), 995_000, "0,5%");
        assert_eq!(san_nhan_toi_thieu(1_000_000, 100), 990_000, "1%");
        assert_eq!(san_nhan_toi_thieu(1_000_000, 5_000), 500_000, "50% là quá lỏng");
        assert_eq!(san_nhan_toi_thieu(1_000_000, 0), 1_000_000);
    }

    // ---------- Tổn thất tạm thời ----------
    #[test]
    fn ton_that_bang_khong_khi_gia_khong_doi() {
        assert!(ton_that_tam_thoi(1.0).abs() < 1e-12);
    }

    #[test]
    fn ton_that_luon_khong_duong() {
        for r in [0.01f64, 0.1, 0.5, 0.9, 1.0, 1.1, 2.0, 5.0, 100.0] {
            assert!(ton_that_tam_thoi(r) <= 1e-12,
                    "r={} cho {} — không bao giờ được dương", r, ton_that_tam_thoi(r));
        }
    }

    #[test]
    fn ton_that_doi_xung_qua_phep_nghich_dao() {
        // Giá tăng gấp đôi hay giảm một nửa đều thiệt như nhau.
        for r in [2.0f64, 4.0, 10.0] {
            let a = ton_that_tam_thoi(r);
            let b = ton_that_tam_thoi(1.0 / r);
            assert!((a - b).abs() < 1e-12, "r={}: {} so với {}", r, a, b);
        }
    }

    #[test]
    fn ton_that_lon_dan_khi_gia_bien_dong_manh_hon() {
        let mut truoc = 0.0;
        for r in [1.1f64, 1.5, 2.0, 4.0, 10.0] {
            let t = ton_that_tam_thoi(r);
            assert!(t < truoc, "biến động mạnh hơn phải thiệt hơn");
            truoc = t;
        }
        // Con số hay được trích dẫn: giá gấp đôi → thiệt khoảng 5,7%
        assert!((ton_that_tam_thoi(2.0) + 0.0572).abs() < 0.001);
        assert!((ton_that_tam_thoi(4.0) + 0.20).abs() < 0.001, "gấp 4 → thiệt 20%");
    }

    #[test]
    fn ton_that_dau_vao_xau_khong_panic() {
        assert_eq!(ton_that_tam_thoi(0.0), 0.0);
        assert_eq!(ton_that_tam_thoi(-1.0), 0.0);
    }

    // ---------- Hàng chờ & MEV ----------
    #[test]
    fn khoi_sap_xep_theo_phi_uu_tien_giam_dan() {
        let cho = vec![
            GiaoDichCho { nguoi_gui: "a".into(), vao_x: 1, toi_thieu_y: 0, phi_uu_tien: 2 },
            GiaoDichCho { nguoi_gui: "b".into(), vao_x: 1, toi_thieu_y: 0, phi_uu_tien: 500 },
            GiaoDichCho { nguoi_gui: "c".into(), vao_x: 1, toi_thieu_y: 0, phi_uu_tien: 1 },
        ];
        let sap = sap_xep_khoi(cho);
        assert_eq!(sap.iter().map(|g| g.nguoi_gui.as_str()).collect::<Vec<_>>(),
                   vec!["b", "a", "c"], "trả nhiều nhất được xếp đầu");
        for w in sap.windows(2) {
            assert!(w[0].phi_uu_tien >= w[1].phi_uu_tien);
        }
    }

    #[test]
    fn sap_xep_on_dinh_khi_phi_bang_nhau() {
        let cho: Vec<GiaoDichCho> = (0..5).map(|i| GiaoDichCho {
            nguoi_gui: format!("n{}", i), vao_x: 1, toi_thieu_y: 0, phi_uu_tien: 10,
        }).collect();
        let sap = sap_xep_khoi(cho);
        assert_eq!(sap.iter().map(|g| g.nguoi_gui.clone()).collect::<Vec<_>>(),
                   vec!["n0", "n1", "n2", "n3", "n4"], "phí bằng nhau thì giữ nguyên thứ tự");
    }

    #[test]
    fn khong_dat_san_nhan_thi_bi_kep_mat_tien() {
        // `toi_thieu_y = 0` nghĩa là "nhận bao nhiêu cũng được" — lời mời công khai.
        let b = be_mau();
        let nn = GiaoDichCho { nguoi_gui: "nan-nhan".into(), vao_x: 50_000,
                               toi_thieu_y: 0, phi_uu_tien: 1 };
        let kq = mo_phong_kep(&b, &nn, 200_000);
        assert!(!kq.bi_chan_boi_bao_ve, "không có bảo vệ thì không gì chặn được");
        assert!(kq.nhan_khi_bi_kep < kq.nhan_neu_khong_bi_kep,
                "bị kẹp thì nhận ít hơn: {} so với {}",
                kq.nhan_khi_bi_kep, kq.nhan_neu_khong_bi_kep);
        assert!(kq.ke_tan_cong_lai > 0, "và kẻ tấn công có lãi");
    }

    #[test]
    fn san_nhan_chat_thi_giao_dich_bi_huy_thay_vi_bi_boc_lot() {
        // Bị huỷ giao dịch là KẾT QUẢ TỐT: bạn chỉ mất phí gas, không mất vốn.
        let b = be_mau();
        let du_kien = b.thu_hoan_doi_x_lay_y(50_000).unwrap();
        let nn = GiaoDichCho { nguoi_gui: "can-than".into(), vao_x: 50_000,
                               toi_thieu_y: san_nhan_toi_thieu(du_kien, 50), // 0,5%
                               phi_uu_tien: 1 };
        let kq = mo_phong_kep(&b, &nn, 200_000);
        assert!(kq.bi_chan_boi_bao_ve, "sàn chặt phải chặn được cú kẹp");
        assert_eq!(kq.ke_tan_cong_lai, 0, "kẻ tấn công không ăn được gì");
    }

    #[test]
    fn san_nhan_cang_long_thi_thiet_hai_cang_lon() {
        let b = be_mau();
        let du_kien = b.thu_hoan_doi_x_lay_y(50_000).unwrap();
        let mut thiet_hai_truoc = 0u128;
        // Đi từ chặt tới lỏng
        for cho_phep in [50u32, 100, 500, 1_000, 5_000] {
            let nn = GiaoDichCho { nguoi_gui: "n".into(), vao_x: 50_000,
                                   toi_thieu_y: san_nhan_toi_thieu(du_kien, cho_phep),
                                   phi_uu_tien: 1 };
            let kq = mo_phong_kep(&b, &nn, 200_000);
            if !kq.bi_chan_boi_bao_ve {
                let thiet = kq.nhan_neu_khong_bi_kep - kq.nhan_khi_bi_kep;
                assert!(thiet >= thiet_hai_truoc,
                        "nới sàn nhận thì thiệt hại không được giảm");
                thiet_hai_truoc = thiet;
            }
        }
        assert!(thiet_hai_truoc > 0, "phải có ít nhất một mức bị bóc lột");
    }

    #[test]
    fn be_cang_sau_thi_cang_kho_bi_kep() {
        // Thanh khoản dày là biện pháp phòng vệ tự nhiên: cùng một cú tấn công
        // đẩy giá được ít hơn hẳn.
        let nong = BeThanhKhoan::moi(100_000, 200_000_000, 30);
        let sau = BeThanhKhoan::moi(10_000_000, 20_000_000_000, 30);
        let thiet = |b: &BeThanhKhoan| {
            let nn = GiaoDichCho { nguoi_gui: "n".into(), vao_x: 10_000,
                                   toi_thieu_y: 0, phi_uu_tien: 1 };
            let kq = mo_phong_kep(b, &nn, 50_000);
            (kq.nhan_neu_khong_bi_kep - kq.nhan_khi_bi_kep) as f64
                / kq.nhan_neu_khong_bi_kep as f64
        };
        assert!(thiet(&sau) < thiet(&nong),
                "bể sâu thiệt {:.4} phải nhỏ hơn bể nông {:.4}", thiet(&sau), thiet(&nong));
    }

    // ---------- Arbitrage CEX-DEX ----------
    #[test]
    fn khong_bao_co_hoi_khi_gia_da_bang_nhau() {
        let b = be_mau(); // giá 2000
        let ch = tim_co_hoi_arb(&b, 2_000.0, 1_000_000_000);
        assert!(!ch.co_co_hoi, "giá bằng nhau thì không có gì để ăn");
        assert_eq!(ch.khoi_luong_toi_uu, 0);
    }

    #[test]
    fn khong_bao_co_hoi_khi_dex_dat_hon_cex() {
        let b = be_mau(); // DEX 2000
        let ch = tim_co_hoi_arb(&b, 1_900.0, 1_000_000_000);
        assert!(!ch.co_co_hoi, "chiều này không có lãi");
    }

    #[test]
    fn tim_duoc_co_hoi_khi_dex_re_hon_va_lai_duong() {
        let b = BeThanhKhoan::moi(1_000_000, 1_900_000_000, 30); // DEX = 1900
        let ch = tim_co_hoi_arb(&b, 2_000.0, 500_000_000);
        assert!(ch.co_co_hoi);
        assert!(ch.lai_uoc_tinh > 0, "lãi phải dương thì mới gọi là cơ hội");
        assert!(ch.khoi_luong_toi_uu > 0);
    }

    #[test]
    fn arbitrage_keo_gia_dex_ve_gan_cex() {
        // Đây là lý do arbitrage tồn tại và có ích: nó khiến giá hội tụ.
        let b = BeThanhKhoan::moi(1_000_000, 1_900_000_000, 30);
        let gia_cex = 2_000.0;
        let ch = tim_co_hoi_arb(&b, gia_cex, 500_000_000);
        assert!(ch.co_co_hoi);
        let lech_truoc = (gia_cex - ch.gia_dex_truoc).abs();
        let lech_sau = (gia_cex - ch.gia_dex_sau).abs();
        assert!(lech_sau < lech_truoc,
                "sau arbitrage giá phải gần nhau hơn: {:.2} so với {:.2}", lech_sau, lech_truoc);
    }

    #[test]
    fn khoi_luong_toi_uu_that_su_toi_uu() {
        // So với các khối lượng lân cận, khối lượng tìm được phải cho lãi cao nhất.
        let b = BeThanhKhoan::moi(1_000_000, 1_900_000_000, 30);
        let gia_cex = 2_000.0;
        let ch = tim_co_hoi_arb(&b, gia_cex, 500_000_000);
        let lai = |v: u128| -> i128 {
            match b.thu_hoan_doi_y_lay_x(v) {
                Ok(x) => (x as f64 * gia_cex) as i128 - v as i128,
                Err(_) => i128::MIN,
            }
        };
        let v = ch.khoi_luong_toi_uu;
        for khac in [v / 4, v / 2, v * 2, v * 4] {
            if khac > 0 && khac < 500_000_000 {
                assert!(lai(v) >= lai(khac),
                        "khối lượng {} cho lãi {} > {} tại {}", khac, lai(khac), lai(v), v);
            }
        }
    }

    #[test]
    fn von_bang_khong_thi_khong_co_co_hoi() {
        let b = BeThanhKhoan::moi(1_000_000, 1_900_000_000, 30);
        assert!(!tim_co_hoi_arb(&b, 2_000.0, 0).co_co_hoi);
    }

    #[test]
    fn lech_gia_cang_lon_thi_lai_arbitrage_cang_nhieu() {
        let mut truoc = 0i128;
        for gia_y in [1_950_000_000u128, 1_900_000_000, 1_800_000_000, 1_600_000_000] {
            let b = BeThanhKhoan::moi(1_000_000, gia_y, 30);
            let ch = tim_co_hoi_arb(&b, 2_000.0, 2_000_000_000);
            assert!(ch.co_co_hoi);
            assert!(ch.lai_uoc_tinh > truoc,
                    "lệch giá lớn hơn phải cho lãi lớn hơn: {} so với {}",
                    ch.lai_uoc_tinh, truoc);
            truoc = ch.lai_uoc_tinh;
        }
    }
}
