#![allow(dead_code)]
//! Chương 69 — Hệ thống giao dịch thuật toán: sổ lệnh, động cơ khớp lệnh,
//! quản trị rủi ro bằng kiểu, và bộ kiểm định chiến lược trên dữ liệu quá khứ.
//!
//! Đây là phần LÕI của một nền tảng kiểu OpenAlgo — nhưng viết bằng Rust, nơi
//! không có bộ dọn rác nên độ trễ có TRẦN xác định, chứ không phải trung bình đẹp
//! kèm những cú khựng bất chợt.
//!
//! ⚠️ Đây là tài liệu KỸ THUẬT, không phải lời khuyên đầu tư. Mọi số liệu đều
//! là dữ liệu giả lập tất định.

use std::collections::{BTreeMap, VecDeque};
use std::marker::PhantomData;

// ============================================================================
// 1. TIỀN LÀ SỐ NGUYÊN — sai lầm đắt giá nhất của người mới
// ============================================================================

/// KHÔNG BAO GIỜ dùng `f64` cho tiền. `0.1 + 0.2 != 0.3` trong nhị phân, và
/// sai số một xu nhân với triệu lệnh là một vụ kiện. Ngành tài chính dùng
/// SỐ NGUYÊN đơn vị nhỏ nhất — ở đây là "tick", 1 tick = 0,01 đơn vị tiền.
pub type Gia = i64;      // tính bằng tick
pub type SoLuong = i64;
pub type MaLenh = u64;

pub fn tick_sang_chuoi(t: Gia) -> String {
    format!("{}.{:02}", t / 100, (t % 100).abs())
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
pub enum Chieu { Mua, Ban }

impl Chieu {
    pub fn nguoc_lai(self) -> Chieu {
        match self { Chieu::Mua => Chieu::Ban, Chieu::Ban => Chieu::Mua }
    }
    /// Dấu của vị thế: mua làm vị thế tăng, bán làm giảm.
    pub fn dau(self) -> i64 { match self { Chieu::Mua => 1, Chieu::Ban => -1 } }
}

// ============================================================================
// 2. VÒNG ĐỜI LỆNH BẰNG TYPESTATE — trạng thái nằm trong KIỂU
// ============================================================================
// Áp dụng Chương 20 vào nghiệp vụ thật: gửi hai lần cùng một lệnh, hoặc hủy
// một lệnh đã khớp hết, là những lỗi tốn tiền. Ở đây chúng KHÔNG BIÊN DỊCH ĐƯỢC.

// Ba nhãn trạng thái. Chúng là kiểu RỖNG — không chiếm một byte nào lúc chạy;
// toàn bộ tác dụng của chúng diễn ra trong trình biên dịch.
#[derive(Debug, Clone, Copy)] pub struct DangSoan;
#[derive(Debug, Clone, Copy)] pub struct DaKiemTraRuiRo;
#[derive(Debug, Clone, Copy)] pub struct DaGui;

#[derive(Debug, Clone)]
pub struct Lenh<TrangThai> {
    pub ma: MaLenh,
    pub ma_ck: String,
    pub chieu: Chieu,
    pub gia: Gia,
    pub so_luong: SoLuong,
    pub da_khop: SoLuong,
    _tt: PhantomData<TrangThai>,
}

impl Lenh<DangSoan> {
    pub fn moi(ma: MaLenh, ma_ck: &str, chieu: Chieu, gia: Gia, so_luong: SoLuong) -> Self {
        Lenh { ma, ma_ck: ma_ck.to_string(), chieu, gia, so_luong, da_khop: 0, _tt: PhantomData }
    }
}

impl<TT> Lenh<TT> {
    pub fn con_lai(&self) -> SoLuong { self.so_luong - self.da_khop }
    fn chuyen<Moi>(self) -> Lenh<Moi> {
        Lenh { ma: self.ma, ma_ck: self.ma_ck, chieu: self.chieu, gia: self.gia,
               so_luong: self.so_luong, da_khop: self.da_khop, _tt: PhantomData }
    }
}

// Chỉ lệnh ĐÃ QUA kiểm tra rủi ro mới gửi được vào sổ lệnh.
impl Lenh<DaKiemTraRuiRo> {
    pub fn gui(self) -> Lenh<DaGui> { self.chuyen() }
}

// ============================================================================
// 3. KIỂM TRA RỦI RO — cổng bắt buộc trước khi lệnh ra thị trường
// ============================================================================

#[derive(Debug, Clone, PartialEq)]
pub enum LoiRuiRo {
    SoLuongKhongDuong(SoLuong),
    GiaKhongDuong(Gia),
    VuotGiaTriToiDa { gia_tri: i64, tran: i64 },
    VuotViTheToiDa { sau_lenh: i64, tran: i64 },
    MaChungKhoanLa(String),
}

pub struct HanMuc {
    pub gia_tri_lenh_toi_da: i64,
    pub vi_the_toi_da: i64,
    pub danh_sach_cho_phep: Vec<String>,
}

impl HanMuc {
    /// Trả `Result` chứ không panic: từ chối lệnh là chuyện BÌNH THƯỜNG,
    /// không phải lỗi lập trình. Đây là ranh giới "parse, đừng validate".
    pub fn kiem_tra(&self, l: Lenh<DangSoan>, vi_the_hien_tai: i64)
        -> Result<Lenh<DaKiemTraRuiRo>, LoiRuiRo>
    {
        if l.so_luong <= 0 { return Err(LoiRuiRo::SoLuongKhongDuong(l.so_luong)); }
        if l.gia <= 0 { return Err(LoiRuiRo::GiaKhongDuong(l.gia)); }
        if !self.danh_sach_cho_phep.iter().any(|m| *m == l.ma_ck) {
            return Err(LoiRuiRo::MaChungKhoanLa(l.ma_ck.clone()));
        }
        let gia_tri = l.gia * l.so_luong;
        if gia_tri > self.gia_tri_lenh_toi_da {
            return Err(LoiRuiRo::VuotGiaTriToiDa { gia_tri, tran: self.gia_tri_lenh_toi_da });
        }
        let sau_lenh = vi_the_hien_tai + l.chieu.dau() * l.so_luong;
        if sau_lenh.abs() > self.vi_the_toi_da {
            return Err(LoiRuiRo::VuotViTheToiDa { sau_lenh, tran: self.vi_the_toi_da });
        }
        Ok(l.chuyen())
    }
}

// ============================================================================
// 4. SỔ LỆNH & ĐỘNG CƠ KHỚP LỆNH
// ============================================================================

#[derive(Debug, Clone, PartialEq)]
pub struct KhopLenh {
    pub lenh_chu_dong: MaLenh,
    pub lenh_thu_dong: MaLenh,
    pub gia: Gia,
    pub so_luong: SoLuong,
}

/// Sổ lệnh giới hạn. `BTreeMap` cho phép lấy giá tốt nhất trong O(log n) và
/// duyệt các mức giá theo THỨ TỰ — đúng thứ động cơ khớp lệnh cần.
/// `VecDeque` ở mỗi mức giá giữ ưu tiên THỜI GIAN: ai đặt trước khớp trước.
pub struct SoLenh {
    /// Bên mua: khóa là giá ÂM để `BTreeMap` (vốn tăng dần) trả giá CAO nhất trước.
    ben_mua: BTreeMap<Gia, VecDeque<Lenh<DaGui>>>,
    ben_ban: BTreeMap<Gia, VecDeque<Lenh<DaGui>>>,
}

impl SoLenh {
    pub fn moi() -> Self { SoLenh { ben_mua: BTreeMap::new(), ben_ban: BTreeMap::new() } }

    /// Giá mua cao nhất — cái giá tốt nhất mà người bán có thể nhận ngay.
    pub fn gia_mua_tot_nhat(&self) -> Option<Gia> {
        self.ben_mua.keys().next().map(|k| -k)
    }
    /// Giá bán thấp nhất.
    pub fn gia_ban_tot_nhat(&self) -> Option<Gia> {
        self.ben_ban.keys().next().copied()
    }
    /// Chênh lệch mua-bán: chi phí ẩn của mọi giao dịch.
    pub fn chenh_lech(&self) -> Option<Gia> {
        Some(self.gia_ban_tot_nhat()? - self.gia_mua_tot_nhat()?)
    }
    /// Giá giữa — ước lượng "giá trị thật" tốt hơn giá khớp gần nhất.
    pub fn gia_giua(&self) -> Option<Gia> {
        Some((self.gia_ban_tot_nhat()? + self.gia_mua_tot_nhat()?) / 2)
    }
    pub fn khoi_luong_tai(&self, chieu: Chieu, gia: Gia) -> SoLuong {
        let ban = match chieu { Chieu::Mua => &self.ben_mua, Chieu::Ban => &self.ben_ban };
        let khoa = match chieu { Chieu::Mua => -gia, Chieu::Ban => gia };
        ban.get(&khoa).map_or(0, |q| q.iter().map(|l| l.con_lai()).sum())
    }
    pub fn tong_so_lenh(&self) -> usize {
        self.ben_mua.values().map(|q| q.len()).sum::<usize>()
            + self.ben_ban.values().map(|q| q.len()).sum::<usize>()
    }

    /// Nạp lệnh và khớp ngay phần khớp được; phần dư nằm lại sổ.
    /// Đây là trái tim của sàn: ƯU TIÊN GIÁ trước, rồi ƯU TIÊN THỜI GIAN.
    pub fn nap(&mut self, mut lenh: Lenh<DaGui>) -> Vec<KhopLenh> {
        let mut cac_khop = Vec::new();
        let doi_ung_la_ban = lenh.chieu == Chieu::Mua;

        loop {
            if lenh.con_lai() == 0 { break; }
            // Mức giá đối ứng tốt nhất còn khớp được với giá giới hạn của ta?
            let khoa_tot = {
                let doi_ung = if doi_ung_la_ban { &self.ben_ban } else { &self.ben_mua };
                match doi_ung.keys().next().copied() {
                    Some(k) => {
                        let gia_that = if doi_ung_la_ban { k } else { -k };
                        let khop_duoc = if doi_ung_la_ban { gia_that <= lenh.gia }
                                        else { gia_that >= lenh.gia };
                        if khop_duoc { Some((k, gia_that)) } else { None }
                    }
                    None => None,
                }
            };
            let (khoa, gia_khop) = match khoa_tot { Some(x) => x, None => break };

            let doi_ung = if doi_ung_la_ban { &mut self.ben_ban } else { &mut self.ben_mua };
            let hang = doi_ung.get_mut(&khoa).unwrap();
            while lenh.con_lai() > 0 {
                let doi_tac = match hang.front_mut() { Some(d) => d, None => break };
                let luong = lenh.con_lai().min(doi_tac.con_lai());
                lenh.da_khop += luong;
                doi_tac.da_khop += luong;
                cac_khop.push(KhopLenh {
                    lenh_chu_dong: lenh.ma,
                    lenh_thu_dong: doi_tac.ma,
                    // Giá khớp là giá của lệnh ĐÃ NẰM SẴN trong sổ — người
                    // đến sau được hưởng giá tốt hơn nếu có. Đây là quy tắc
                    // "cải thiện giá" của mọi sàn nghiêm túc.
                    gia: gia_khop,
                    so_luong: luong,
                });
                if doi_tac.con_lai() == 0 { hang.pop_front(); }
            }
            if hang.is_empty() { doi_ung.remove(&khoa); }
        }

        if lenh.con_lai() > 0 {
            let khoa = if lenh.chieu == Chieu::Mua { -lenh.gia } else { lenh.gia };
            let ban = if lenh.chieu == Chieu::Mua { &mut self.ben_mua } else { &mut self.ben_ban };
            ban.entry(khoa).or_default().push_back(lenh);
        }
        cac_khop
    }

    pub fn huy(&mut self, ma: MaLenh) -> bool {
        for ban in [&mut self.ben_mua, &mut self.ben_ban] {
            let mut rong = None;
            for (khoa, hang) in ban.iter_mut() {
                if let Some(i) = hang.iter().position(|l| l.ma == ma) {
                    hang.remove(i);
                    if hang.is_empty() { rong = Some(*khoa); }
                    if let Some(k) = rong { ban.remove(&k); }
                    return true;
                }
            }
        }
        false
    }
}

// ============================================================================
// 5. VỊ THẾ & LÃI/LỖ — một VỊ NHÓM (Chương 18) trá hình
// ============================================================================

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct ViThe {
    pub so_luong: i64,
    /// Tiền mặt tính bằng tick. Mua làm tiền giảm, bán làm tiền tăng.
    pub tien_mat: i64,
}

impl ViThe {
    pub const RONG: ViThe = ViThe { so_luong: 0, tien_mat: 0 };

    /// Phép `ghep` này KẾT HỢP và có ĐƠN VỊ `RONG` → đúng định nghĩa vị nhóm.
    /// Nhờ vậy có thể gộp lãi/lỗ song song bằng `rayon` mà kết quả không đổi.
    pub fn ghep(self, k: ViThe) -> ViThe {
        ViThe { so_luong: self.so_luong + k.so_luong, tien_mat: self.tien_mat + k.tien_mat }
    }
    pub fn tu_khop(chieu: Chieu, gia: Gia, so_luong: SoLuong) -> ViThe {
        ViThe {
            so_luong: chieu.dau() * so_luong,
            tien_mat: -chieu.dau() * gia * so_luong,
        }
    }
    /// Giá trị ròng khi định giá lại theo giá thị trường hiện tại.
    pub fn gia_tri_rong(&self, gia_thi_truong: Gia) -> i64 {
        self.tien_mat + self.so_luong * gia_thi_truong
    }
}

// ============================================================================
// 6. BỘ KIỂM ĐỊNH CHIẾN LƯỢC (backtest) — hàm thuần túy trên lịch sử
// ============================================================================

#[derive(Debug, Clone, Copy, PartialEq)]
pub struct Nen { pub thoi_diem: u64, pub mo: Gia, pub cao: Gia, pub thap: Gia, pub dong: Gia }

#[derive(Debug, Clone, Copy, PartialEq)]
pub enum TinHieu { Mua(SoLuong), Ban(SoLuong), Giu }

/// Chiến lược là một HÀM THUẦN TÚY: cùng lịch sử → cùng tín hiệu, luôn luôn.
/// Nhờ tính chất này mà kết quả kiểm định tái lập được 100%.
pub trait ChienLuoc {
    fn ten(&self) -> &str;
    fn quyet_dinh(&mut self, lich_su: &[Nen], vi_the: &ViThe) -> TinHieu;
}

/// Giao cắt trung bình động: kinh điển, dễ hiểu, và cố tình đơn giản.
pub struct GiaoCatTrungBinh { pub nhanh: usize, pub cham: usize, pub don_vi: SoLuong }

fn trung_binh(nen: &[Nen], n: usize) -> Option<Gia> {
    if nen.len() < n { return None; }
    Some(nen[nen.len() - n..].iter().map(|c| c.dong).sum::<Gia>() / n as Gia)
}

impl ChienLuoc for GiaoCatTrungBinh {
    fn ten(&self) -> &str { "Giao cắt trung bình động" }
    fn quyet_dinh(&mut self, lich_su: &[Nen], vi_the: &ViThe) -> TinHieu {
        let (nhanh, cham) = match (trung_binh(lich_su, self.nhanh), trung_binh(lich_su, self.cham)) {
            (Some(a), Some(b)) => (a, b),
            _ => return TinHieu::Giu, // chưa đủ dữ liệu — KHÔNG đoán mò
        };
        if nhanh > cham && vi_the.so_luong <= 0 { TinHieu::Mua(self.don_vi) }
        else if nhanh < cham && vi_the.so_luong > 0 { TinHieu::Ban(vi_the.so_luong) }
        else { TinHieu::Giu }
    }
}

#[derive(Debug, PartialEq)]
pub struct KetQuaKiemDinh {
    pub vi_the_cuoi: ViThe,
    pub gia_tri_cuoi: i64,
    pub so_giao_dich: usize,
    /// Mức sụt giảm sâu nhất từ đỉnh — con số quan trọng hơn cả lợi nhuận,
    /// vì nó quyết định bạn có chịu nổi để đi hết chiến lược hay không.
    pub sut_giam_toi_da: i64,
    pub duong_von: Vec<i64>,
}

/// Chạy kiểm định. Có mô hình TRƯỢT GIÁ và PHÍ — bỏ hai thứ này là cách
/// nhanh nhất để tự lừa mình bằng một đường vốn đẹp nhưng không có thật.
pub fn chay_kiem_dinh(
    du_lieu: &[Nen],
    chien_luoc: &mut dyn ChienLuoc,
    truot_gia_tick: Gia,
    phi_moi_don_vi: i64,
) -> KetQuaKiemDinh {
    let mut vi_the = ViThe::RONG;
    let mut so_gd = 0;
    let mut duong_von = Vec::with_capacity(du_lieu.len());
    let mut dinh = i64::MIN;
    let mut sut_toi_da = 0;

    for i in 0..du_lieu.len() {
        let lich_su = &du_lieu[..=i];
        // Quyết định dựa trên nến ĐÃ ĐÓNG, khớp ở nến KẾ TIẾP.
        // Bỏ qua chi tiết này = "nhìn trộm tương lai", lỗi kinh điển
        // khiến mọi chiến lược trông như in tiền.
        let tin_hieu = chien_luoc.quyet_dinh(lich_su, &vi_the);
        if let Some(nen_sau) = du_lieu.get(i + 1) {
            let (chieu, luong) = match tin_hieu {
                TinHieu::Mua(q) => (Chieu::Mua, q),
                TinHieu::Ban(q) => (Chieu::Ban, q),
                TinHieu::Giu => { duong_von.push(vi_the.gia_tri_rong(du_lieu[i].dong)); continue; }
            };
            if luong > 0 {
                // Trượt giá: ta luôn mua đắt hơn và bán rẻ hơn giá lý thuyết.
                let gia = nen_sau.mo + chieu.dau() * truot_gia_tick;
                vi_the = vi_the.ghep(ViThe::tu_khop(chieu, gia, luong));
                vi_the.tien_mat -= phi_moi_don_vi * luong;
                so_gd += 1;
            }
        }
        let gt = vi_the.gia_tri_rong(du_lieu[i].dong);
        duong_von.push(gt);
        dinh = dinh.max(gt);
        sut_toi_da = sut_toi_da.max(dinh - gt);
    }

    let gia_cuoi = du_lieu.last().map_or(0, |n| n.dong);
    KetQuaKiemDinh {
        gia_tri_cuoi: vi_the.gia_tri_rong(gia_cuoi),
        vi_the_cuoi: vi_the,
        so_giao_dich: so_gd,
        sut_giam_toi_da: sut_toi_da,
        duong_von,
    }
}

/// Sinh dữ liệu giá tất định (bước ngẫu nhiên có hạt giống cố định).
/// Tất định là điều kiện BẮT BUỘC để kiểm thử hồi quy có ý nghĩa.
pub fn sinh_du_lieu(so_nen: usize, gia_dau: Gia, hat_giong: u64) -> Vec<Nen> {
    let mut s = hat_giong;
    let mut gia = gia_dau;
    (0..so_nen).map(|i| {
        s = s.wrapping_mul(6364136223846793005).wrapping_add(1442695040888963407);
        let buoc = ((s >> 33) % 41) as i64 - 20; // -20..+20 tick
        let mo = gia;
        gia = (gia + buoc).max(1);
        Nen {
            thoi_diem: i as u64,
            mo,
            cao: mo.max(gia) + 5,
            thap: (mo.min(gia) - 5).max(1),
            dong: gia,
        }
    }).collect()
}

fn main() {
    println!("═══════════════════════════════════════════════════════════");
    println!("   HỆ THỐNG GIAO DỊCH: SỔ LỆNH · KHỚP LỆNH · KIỂM ĐỊNH     ");
    println!("═══════════════════════════════════════════════════════════");

    println!("\n1. VÌ SAO TIỀN PHẢI LÀ SỐ NGUYÊN");
    let sai: f64 = (0..10).map(|_| 0.1f64).sum();
    println!("   Cộng 0.1 mười lần bằng f64 → {:.20}", sai);
    println!("   Bằng nhau với 1.0?          → {}", sai == 1.0);
    println!("   Bằng số nguyên tick         → {} tick = {}", 100, tick_sang_chuoi(100));

    println!("\n2. CỔNG RỦI RO");
    let hm = HanMuc { gia_tri_lenh_toi_da: 1_000_000, vi_the_toi_da: 500,
                      danh_sach_cho_phep: vec!["VNM".into(), "FPT".into()] };
    for (mo_ta, l) in [
        ("hợp lệ         ", Lenh::moi(1, "VNM", Chieu::Mua, 8_500, 100)),
        ("mã lạ          ", Lenh::moi(2, "XYZ", Chieu::Mua, 8_500, 100)),
        ("quá to         ", Lenh::moi(3, "VNM", Chieu::Mua, 8_500, 1_000)),
        ("số lượng âm    ", Lenh::moi(4, "VNM", Chieu::Mua, 8_500, -5)),
    ] {
        match hm.kiem_tra(l, 0) {
            Ok(_) => println!("   {} → CHO QUA", mo_ta),
            Err(e) => println!("   {} → CHẶN: {:?}", mo_ta, e),
        }
    }

    println!("\n3. SỔ LỆNH & ƯU TIÊN GIÁ–THỜI GIAN");
    let mut so = SoLenh::moi();
    let gui = |ma, chieu, gia, sl| {
        Lenh::<DangSoan>::moi(ma, "VNM", chieu, gia, sl)
            .chuyen::<DaKiemTraRuiRo>().gui()
    };
    for (ma, gia, sl) in [(10u64, 8_400i64, 100i64), (11, 8_400, 200), (12, 8_390, 500)] {
        so.nap(gui(ma, Chieu::Mua, gia, sl));
    }
    for (ma, gia, sl) in [(20u64, 8_420i64, 150i64), (21, 8_430, 300)] {
        so.nap(gui(ma, Chieu::Ban, gia, sl));
    }
    println!("   Mua tốt nhất {} · Bán tốt nhất {} · Chênh lệch {} tick",
             tick_sang_chuoi(so.gia_mua_tot_nhat().unwrap()),
             tick_sang_chuoi(so.gia_ban_tot_nhat().unwrap()),
             so.chenh_lech().unwrap());
    println!("   Khối lượng chờ mua ở {}: {}", tick_sang_chuoi(8_400), so.khoi_luong_tai(Chieu::Mua, 8_400));

    println!("\n4. KHỚP LỆNH — lệnh bán 250 quét qua bên mua");
    let khop = so.nap(gui(30, Chieu::Ban, 8_390, 250));
    for k in &khop {
        println!("   {} đơn vị @ {} (đối tác lệnh #{})",
                 k.so_luong, tick_sang_chuoi(k.gia), k.lenh_thu_dong);
    }
    println!("   → Lệnh #10 (đặt trước) khớp hết TRƯỚC lệnh #11, dù cùng giá.");
    println!("   → Khớp ở giá {} chứ không phải {} — người đến sau được cải thiện giá.",
             tick_sang_chuoi(8_400), tick_sang_chuoi(8_390));

    println!("\n5. VỊ THẾ LÀ MỘT VỊ NHÓM");
    let a = ViThe::tu_khop(Chieu::Mua, 8_400, 100);
    let b = ViThe::tu_khop(Chieu::Ban, 8_500, 60);
    println!("   Mua 100@84.00 rồi bán 60@85.00 → {:?}", a.ghep(b));
    println!("   Kết hợp: (a·b)·c == a·(b·c) → {}",
             a.ghep(b).ghep(ViThe::RONG) == a.ghep(b.ghep(ViThe::RONG)));

    println!("\n6. KIỂM ĐỊNH CHIẾN LƯỢC — 500 nến, có phí và trượt giá");
    let du_lieu = sinh_du_lieu(500, 8_000, 42);
    for (truot, phi) in [(0i64, 0i64), (2, 3)] {
        let mut cl = GiaoCatTrungBinh { nhanh: 5, cham: 20, don_vi: 100 };
        let kq = chay_kiem_dinh(&du_lieu, &mut cl, truot, phi);
        println!("   trượt {} tick, phí {}/đv → lãi {:>8} tick · {} lệnh · sụt sâu nhất {} tick",
                 truot, phi, kq.gia_tri_cuoi, kq.so_giao_dich, kq.sut_giam_toi_da);
    }
    println!("   → Cùng một chiến lược: bỏ qua phí và trượt giá là tự lừa mình.");

    println!("\n═══════════════════════════════════════════════════════════");
    println!("   ĐỘ TRỄ CÓ TRẦN XÁC ĐỊNH — LÝ DO NGÀNH NÀY CHỌN RUST      ");
    println!("═══════════════════════════════════════════════════════════");
}

#[cfg(test)]
mod kiem_thu {
    use super::*;

    fn lenh_da_gui(ma: MaLenh, chieu: Chieu, gia: Gia, sl: SoLuong) -> Lenh<DaGui> {
        Lenh::<DangSoan>::moi(ma, "VNM", chieu, gia, sl).chuyen::<DaKiemTraRuiRo>().gui()
    }

    // ---------- Tiền & kiểu ----------
    #[test]
    fn tien_so_nguyen_khong_co_sai_so_tich_luy() {
        let f64_tong: f64 = (0..1000).map(|_| 0.01f64).sum();
        assert_ne!(f64_tong, 10.0, "f64 KHÔNG cộng đúng — đây là lý do không dùng nó cho tiền");
        let tick_tong: i64 = (0..1000).map(|_| 1i64).sum();
        assert_eq!(tick_tong, 1000, "số nguyên thì chính xác tuyệt đối");
    }

    #[test]
    fn hien_thi_tick_dung_ca_so_am() {
        assert_eq!(tick_sang_chuoi(8_450), "84.50");
        assert_eq!(tick_sang_chuoi(5), "0.05");
        assert_eq!(tick_sang_chuoi(-8_450), "-84.50");
    }

    // ---------- Rủi ro ----------
    #[test]
    fn cong_rui_ro_chan_dung_tung_loai_vi_pham() {
        let hm = HanMuc { gia_tri_lenh_toi_da: 1_000_000, vi_the_toi_da: 500,
                          danh_sach_cho_phep: vec!["VNM".into()] };
        // Dùng `unwrap_err()` chứ không `assert_eq!` cả `Result`: `Lenh` không
        // cài `PartialEq` (so sánh hai lệnh theo giá trị là vô nghĩa — mỗi lệnh
        // có danh tính riêng qua `ma`).
        assert!(hm.kiem_tra(Lenh::moi(1, "VNM", Chieu::Mua, 8_500, 100), 0).is_ok());
        assert_eq!(hm.kiem_tra(Lenh::moi(2, "VNM", Chieu::Mua, 8_500, 0), 0).unwrap_err(),
                   LoiRuiRo::SoLuongKhongDuong(0));
        assert_eq!(hm.kiem_tra(Lenh::moi(3, "VNM", Chieu::Mua, 0, 10), 0).unwrap_err(),
                   LoiRuiRo::GiaKhongDuong(0));
        assert_eq!(hm.kiem_tra(Lenh::moi(4, "XYZ", Chieu::Mua, 100, 10), 0).unwrap_err(),
                   LoiRuiRo::MaChungKhoanLa("XYZ".into()));
        assert!(matches!(hm.kiem_tra(Lenh::moi(5, "VNM", Chieu::Mua, 8_500, 1_000), 0).unwrap_err(),
                         LoiRuiRo::VuotGiaTriToiDa { .. }));
    }

    #[test]
    fn han_muc_vi_the_tinh_ca_chieu_ban_khong() {
        let hm = HanMuc { gia_tri_lenh_toi_da: i64::MAX, vi_the_toi_da: 100,
                          danh_sach_cho_phep: vec!["VNM".into()] };
        // bán khống 150 khi đang giữ 0 → vị thế -150, vượt trần 100
        assert_eq!(hm.kiem_tra(Lenh::moi(1, "VNM", Chieu::Ban, 100, 150), 0).unwrap_err(),
                   LoiRuiRo::VuotViTheToiDa { sau_lenh: -150, tran: 100 });
        // nhưng bán 150 khi đang giữ 100 → còn -50, hợp lệ
        assert!(hm.kiem_tra(Lenh::moi(2, "VNM", Chieu::Ban, 100, 150), 100).is_ok());
    }

    // ---------- Sổ lệnh ----------
    #[test]
    fn so_lenh_tra_dung_gia_tot_nhat_hai_ben() {
        let mut s = SoLenh::moi();
        s.nap(lenh_da_gui(1, Chieu::Mua, 100, 10));
        s.nap(lenh_da_gui(2, Chieu::Mua, 105, 10)); // giá cao hơn = tốt hơn cho bên mua
        s.nap(lenh_da_gui(3, Chieu::Ban, 120, 10));
        s.nap(lenh_da_gui(4, Chieu::Ban, 110, 10)); // giá thấp hơn = tốt hơn cho bên bán
        assert_eq!(s.gia_mua_tot_nhat(), Some(105));
        assert_eq!(s.gia_ban_tot_nhat(), Some(110));
        assert_eq!(s.chenh_lech(), Some(5));
        assert_eq!(s.gia_giua(), Some(107));
    }

    #[test]
    fn lenh_khong_giao_nhau_thi_nam_lai_so() {
        let mut s = SoLenh::moi();
        assert!(s.nap(lenh_da_gui(1, Chieu::Mua, 100, 10)).is_empty());
        assert!(s.nap(lenh_da_gui(2, Chieu::Ban, 110, 10)).is_empty());
        assert_eq!(s.tong_so_lenh(), 2);
    }

    #[test]
    fn uu_tien_thoi_gian_o_cung_muc_gia() {
        let mut s = SoLenh::moi();
        s.nap(lenh_da_gui(1, Chieu::Mua, 100, 50));  // đến TRƯỚC
        s.nap(lenh_da_gui(2, Chieu::Mua, 100, 50));  // đến SAU
        let khop = s.nap(lenh_da_gui(3, Chieu::Ban, 100, 60));
        assert_eq!(khop.len(), 2);
        assert_eq!(khop[0].lenh_thu_dong, 1, "lệnh đến trước phải khớp trước");
        assert_eq!(khop[0].so_luong, 50);
        assert_eq!(khop[1].lenh_thu_dong, 2);
        assert_eq!(khop[1].so_luong, 10);
    }

    #[test]
    fn uu_tien_gia_thang_uu_tien_thoi_gian() {
        let mut s = SoLenh::moi();
        s.nap(lenh_da_gui(1, Chieu::Mua, 100, 50));  // đến trước, giá THẤP hơn
        s.nap(lenh_da_gui(2, Chieu::Mua, 105, 50));  // đến sau, giá CAO hơn
        let khop = s.nap(lenh_da_gui(3, Chieu::Ban, 100, 10));
        assert_eq!(khop[0].lenh_thu_dong, 2, "giá tốt hơn thắng, dù đến sau");
        assert_eq!(khop[0].gia, 105);
    }

    #[test]
    fn nguoi_den_sau_duoc_cai_thien_gia() {
        let mut s = SoLenh::moi();
        s.nap(lenh_da_gui(1, Chieu::Ban, 100, 10)); // ai đó chào bán rẻ
        // ta sẵn sàng mua tới 120, nhưng chỉ phải trả 100
        let khop = s.nap(lenh_da_gui(2, Chieu::Mua, 120, 10));
        assert_eq!(khop[0].gia, 100, "khớp ở giá của lệnh nằm sẵn trong sổ");
    }

    #[test]
    fn lenh_lon_quet_qua_nhieu_muc_gia() {
        let mut s = SoLenh::moi();
        s.nap(lenh_da_gui(1, Chieu::Ban, 100, 10));
        s.nap(lenh_da_gui(2, Chieu::Ban, 101, 10));
        s.nap(lenh_da_gui(3, Chieu::Ban, 102, 10));
        let khop = s.nap(lenh_da_gui(4, Chieu::Mua, 102, 25));
        assert_eq!(khop.len(), 3);
        assert_eq!(khop.iter().map(|k| k.gia).collect::<Vec<_>>(), vec![100, 101, 102],
                   "phải ăn từ giá tốt nhất trở đi");
        assert_eq!(khop.iter().map(|k| k.so_luong).sum::<i64>(), 25);
        assert_eq!(s.tong_so_lenh(), 1, "mức 102 còn dư 5 đơn vị");
    }

    #[test]
    fn phan_du_cua_lenh_chu_dong_nam_lai_so() {
        let mut s = SoLenh::moi();
        s.nap(lenh_da_gui(1, Chieu::Ban, 100, 10));
        let khop = s.nap(lenh_da_gui(2, Chieu::Mua, 100, 30));
        assert_eq!(khop.iter().map(|k| k.so_luong).sum::<i64>(), 10);
        assert_eq!(s.gia_mua_tot_nhat(), Some(100), "20 đơn vị còn lại thành lệnh chờ mua");
        assert_eq!(s.khoi_luong_tai(Chieu::Mua, 100), 20);
    }

    #[test]
    fn bao_toan_khoi_luong_qua_moi_lan_khop() {
        // BẤT BIẾN SỐNG CÒN của mọi sàn: không đơn vị nào được sinh ra
        // hay biến mất trong quá trình khớp.
        let mut s = SoLenh::moi();
        let mut da_nap = 0i64;
        let mut da_khop = 0i64;
        for i in 0..60u64 {
            let chieu = if i % 2 == 0 { Chieu::Mua } else { Chieu::Ban };
            let gia = 100 + ((i * 7) % 11) as i64 - 5;
            let sl = 10 + (i % 13) as i64;
            da_nap += sl;
            da_khop += s.nap(lenh_da_gui(i, chieu, gia, sl))
                        .iter().map(|k| k.so_luong).sum::<i64>();
        }
        let con_trong_so: i64 = [Chieu::Mua, Chieu::Ban].iter()
            .flat_map(|&c| (80..=120).map(move |g| (c, g)))
            .map(|(c, g)| s.khoi_luong_tai(c, g)).sum();
        // Mỗi lần khớp tiêu thụ khối lượng từ CẢ HAI phía
        assert_eq!(da_nap - 2 * da_khop, con_trong_so,
                   "khối lượng phải cân bằng tuyệt đối");
    }

    #[test]
    fn huy_lenh_go_dung_lenh_va_don_muc_gia_rong() {
        let mut s = SoLenh::moi();
        s.nap(lenh_da_gui(1, Chieu::Mua, 100, 10));
        s.nap(lenh_da_gui(2, Chieu::Mua, 100, 20));
        assert!(s.huy(1));
        assert_eq!(s.khoi_luong_tai(Chieu::Mua, 100), 20);
        assert!(s.huy(2));
        assert_eq!(s.gia_mua_tot_nhat(), None, "mức giá rỗng phải bị xóa khỏi sổ");
        assert!(!s.huy(999), "hủy lệnh không tồn tại phải trả false");
    }

    #[test]
    fn so_rong_khong_co_gia_va_khong_panic() {
        let s = SoLenh::moi();
        assert_eq!(s.gia_mua_tot_nhat(), None);
        assert_eq!(s.chenh_lech(), None);
        assert_eq!(s.gia_giua(), None);
        assert_eq!(s.tong_so_lenh(), 0);
    }

    // ---------- Vị thế ----------
    #[test]
    fn vi_the_thoa_luat_vi_nhom() {
        let a = ViThe::tu_khop(Chieu::Mua, 100, 10);
        let b = ViThe::tu_khop(Chieu::Ban, 110, 5);
        let c = ViThe::tu_khop(Chieu::Mua, 90, 3);
        assert_eq!(a.ghep(b).ghep(c), a.ghep(b.ghep(c)), "luật kết hợp");
        assert_eq!(a.ghep(ViThe::RONG), a, "luật đơn vị phải");
        assert_eq!(ViThe::RONG.ghep(a), a, "luật đơn vị trái");
    }

    #[test]
    fn gop_vi_the_theo_khoi_cho_cung_ket_qua() {
        // Vì là vị nhóm, chia nhỏ rồi gộp lại (như khi dùng rayon) cho kết quả
        // Y HỆT tính tuần tự. Đây là bảo chứng toán học, không phải may mắn.
        let khop: Vec<ViThe> = (0..100).map(|i| {
            let chieu = if i % 3 == 0 { Chieu::Ban } else { Chieu::Mua };
            ViThe::tu_khop(chieu, 100 + i % 7, 1 + i % 5)
        }).collect();
        let tuan_tu = khop.iter().fold(ViThe::RONG, |a, &b| a.ghep(b));
        let theo_khoi = khop.chunks(7)
            .map(|k| k.iter().fold(ViThe::RONG, |a, &b| a.ghep(b)))
            .fold(ViThe::RONG, |a, b| a.ghep(b));
        assert_eq!(tuan_tu, theo_khoi);
    }

    #[test]
    fn mua_roi_ban_cao_hon_thi_co_lai() {
        let v = ViThe::tu_khop(Chieu::Mua, 8_000, 100)
            .ghep(ViThe::tu_khop(Chieu::Ban, 8_500, 100));
        assert_eq!(v.so_luong, 0, "đã đóng hết vị thế");
        assert_eq!(v.gia_tri_rong(0), 50_000, "(8500-8000) × 100 tick");
    }

    #[test]
    fn vi_the_mo_duoc_dinh_gia_lai_theo_thi_truong() {
        let v = ViThe::tu_khop(Chieu::Mua, 8_000, 100);
        assert_eq!(v.gia_tri_rong(8_000), 0, "vừa mua xong thì hòa vốn");
        assert_eq!(v.gia_tri_rong(8_100), 10_000, "giá lên 100 tick → lãi 10 000");
        assert_eq!(v.gia_tri_rong(7_900), -10_000, "giá xuống thì lỗ đối xứng");
    }

    // ---------- Kiểm định ----------
    #[test]
    fn sinh_du_lieu_tat_dinh_theo_hat_giong() {
        assert_eq!(sinh_du_lieu(50, 8_000, 7), sinh_du_lieu(50, 8_000, 7));
        assert_ne!(sinh_du_lieu(50, 8_000, 7), sinh_du_lieu(50, 8_000, 8));
    }

    #[test]
    fn du_lieu_sinh_ra_luon_hop_le() {
        for nen in sinh_du_lieu(500, 8_000, 99) {
            assert!(nen.cao >= nen.mo && nen.cao >= nen.dong, "đỉnh phải cao nhất");
            assert!(nen.thap <= nen.mo && nen.thap <= nen.dong, "đáy phải thấp nhất");
            assert!(nen.thap > 0, "giá không bao giờ âm");
        }
    }

    #[test]
    fn chien_luoc_giu_im_khi_chua_du_du_lieu() {
        let mut cl = GiaoCatTrungBinh { nhanh: 5, cham: 20, don_vi: 100 };
        let it_nen = sinh_du_lieu(10, 8_000, 1);
        assert_eq!(cl.quyet_dinh(&it_nen, &ViThe::RONG), TinHieu::Giu,
                   "chưa đủ 20 nến thì KHÔNG được đoán mò");
    }

    #[test]
    fn kiem_dinh_tai_lap_duoc_hoan_toan() {
        let du_lieu = sinh_du_lieu(300, 8_000, 42);
        let chay = || {
            let mut cl = GiaoCatTrungBinh { nhanh: 5, cham: 20, don_vi: 100 };
            chay_kiem_dinh(&du_lieu, &mut cl, 2, 3)
        };
        assert_eq!(chay(), chay(), "cùng dữ liệu + cùng chiến lược = cùng kết quả, luôn luôn");
    }

    #[test]
    fn phi_va_truot_gia_luon_lam_ket_qua_xau_di() {
        let du_lieu = sinh_du_lieu(400, 8_000, 2024);
        let mut cl1 = GiaoCatTrungBinh { nhanh: 5, cham: 20, don_vi: 100 };
        let ly_tuong = chay_kiem_dinh(&du_lieu, &mut cl1, 0, 0);
        let mut cl2 = GiaoCatTrungBinh { nhanh: 5, cham: 20, don_vi: 100 };
        let thuc_te = chay_kiem_dinh(&du_lieu, &mut cl2, 2, 3);
        assert_eq!(ly_tuong.so_giao_dich, thuc_te.so_giao_dich, "cùng số lệnh");
        assert!(thuc_te.gia_tri_cuoi < ly_tuong.gia_tri_cuoi,
                "chi phí giao dịch luôn ăn vào lợi nhuận: {} so với {}",
                thuc_te.gia_tri_cuoi, ly_tuong.gia_tri_cuoi);
    }

    #[test]
    fn sut_giam_toi_da_khong_bao_gio_am() {
        for hat in [1u64, 7, 42, 2024, 31337] {
            let du_lieu = sinh_du_lieu(200, 8_000, hat);
            let mut cl = GiaoCatTrungBinh { nhanh: 3, cham: 10, don_vi: 50 };
            let kq = chay_kiem_dinh(&du_lieu, &mut cl, 1, 1);
            assert!(kq.sut_giam_toi_da >= 0, "sụt giảm là khoảng cách, không thể âm");
            assert_eq!(kq.duong_von.len(), du_lieu.len());
        }
    }

    #[test]
    fn khong_giao_dich_thi_khong_lai_khong_lo() {
        struct KhongLamGi;
        impl ChienLuoc for KhongLamGi {
            fn ten(&self) -> &str { "đứng ngoài" }
            fn quyet_dinh(&mut self, _: &[Nen], _: &ViThe) -> TinHieu { TinHieu::Giu }
        }
        let du_lieu = sinh_du_lieu(200, 8_000, 5);
        let kq = chay_kiem_dinh(&du_lieu, &mut KhongLamGi, 5, 10);
        assert_eq!(kq.so_giao_dich, 0);
        assert_eq!(kq.gia_tri_cuoi, 0, "không vào lệnh thì không thể mất tiền");
        assert_eq!(kq.sut_giam_toi_da, 0);
    }

    #[test]
    fn chien_luoc_khong_duoc_nhin_trom_tuong_lai() {
        // Nếu bộ kiểm định khớp ở giá ĐÓNG của chính cây nến ra tín hiệu,
        // ta đã dùng thông tin chưa tồn tại. Ở đây khớp ở giá MỞ của nến kế
        // tiếp, nên nến CUỐI CÙNG không thể sinh giao dịch nào.
        let du_lieu = sinh_du_lieu(30, 8_000, 3);
        struct LuonMua;
        impl ChienLuoc for LuonMua {
            fn ten(&self) -> &str { "luôn mua" }
            fn quyet_dinh(&mut self, _: &[Nen], _: &ViThe) -> TinHieu { TinHieu::Mua(1) }
        }
        let kq = chay_kiem_dinh(&du_lieu, &mut LuonMua, 0, 0);
        assert_eq!(kq.so_giao_dich, du_lieu.len() - 1,
                   "nến cuối không có nến kế tiếp để khớp — không được bịa ra giao dịch");
    }
}
