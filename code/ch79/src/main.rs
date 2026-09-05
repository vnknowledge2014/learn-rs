#![allow(dead_code)]
//! Chương 79 — FPGA cho giao dịch: bộ xử lý luồng dữ liệu bằng phần cứng, sổ
//! lệnh trên thanh ghi, đường ống kiểm tra rủi ro, và ngân sách tick-to-trade
//! tính bằng CHU KỲ thay vì micro-giây.
//!
//! Nối hai mạch của giáo trình: Chương 67 (thiết kế phần cứng số) gặp Chương
//! 74–77 (hệ sinh thái HFT). Đây chính là chỗ `hardcaml` của Jane Street và
//! `rhdl` trong hệ sinh thái Rust nhắm tới: mô tả phần cứng bằng một ngôn ngữ
//! có hệ thống kiểu mạnh, mô phỏng ngay trong bộ kiểm thử, rồi mới sinh Verilog.

// ============================================================================
// 1. VÌ SAO GIAO DỊCH DÙNG FPGA
// ============================================================================
// Phần mềm giỏi nhất đạt tick-to-trade khoảng 1–5 µs, nhưng có ĐUÔI DÀI: hệ
// điều hành xen vào, trượt cache, một cú dừng bất chợt. FPGA đạt 20–100 ns và
// quan trọng hơn — độ trễ gần như KHÔNG DAO ĐỘNG. Trong đấu giá theo thứ tự
// tới, người ổn định thắng người nhanh-nhưng-thất-thường.

/// Chu kỳ xung nhịp của FPGA giao dịch điển hình: 250 MHz → 4 ns mỗi chu kỳ.
pub const NS_MOI_CHU_KY: f64 = 4.0;

pub fn chu_ky_sang_ns(chu_ky: u32) -> f64 { chu_ky as f64 * NS_MOI_CHU_KY }

// ============================================================================
// 2. TÁCH TRƯỜNG SONG SONG — điều phần mềm không làm được
// ============================================================================
// Phần mềm đọc từng trường một: đọc offset 0, rồi 8, rồi 16… Mỗi lần là một
// lệnh CPU. Phần cứng nối THẲNG dây từ mọi vị trí byte tới mọi thanh ghi đích,
// nên TẤT CẢ trường được tách trong CÙNG MỘT chu kỳ.

#[derive(Debug, Clone, Copy, PartialEq, Eq, Default)]
pub struct TruongGoiTin {
    pub loai: u8,
    pub ma_ck: u32,
    pub gia: i64,
    pub so_luong: u32,
    pub hop_le: bool,
}

/// Bố cục gói tin cố định 20 byte:
/// `[loại 1B][mã ck 4B][giá 8B][số lượng 4B][tổng kiểm tra 3B]`
pub const DAI_GOI: usize = 20;

#[derive(Debug, Default)]
pub struct BoTachTruong {
    pub so_goi_da_tach: u64,
    pub so_goi_hong: u64,
}

impl BoTachTruong {
    /// Tách toàn bộ trường trong ĐÚNG MỘT chu kỳ. Trong Rust ta viết tuần tự,
    /// nhưng khi tổng hợp ra mạch thì các phép gán này là dây nối song song —
    /// không có "trước" và "sau", tất cả xảy ra cùng lúc.
    pub fn tach(&mut self, goi: &[u8]) -> Option<TruongGoiTin> {
        if goi.len() < DAI_GOI { self.so_goi_hong += 1; return None; }

        let t = TruongGoiTin {
            loai: goi[0],
            ma_ck: u32::from_be_bytes([goi[1], goi[2], goi[3], goi[4]]),
            gia: i64::from_be_bytes([goi[5], goi[6], goi[7], goi[8],
                                     goi[9], goi[10], goi[11], goi[12]]),
            so_luong: u32::from_be_bytes([goi[13], goi[14], goi[15], goi[16]]),
            hop_le: true,
        };

        // Tổng kiểm tra cũng tính SONG SONG bằng cây XOR — độ sâu log(n)
        // thay vì n bước cộng dồn như phần mềm.
        let tk = cay_xor(&goi[..17]) & 0x00FF_FFFF;
        let mong_doi = u32::from_be_bytes([0, goi[17], goi[18], goi[19]]);
        if tk != mong_doi {
            self.so_goi_hong += 1;
            return Some(TruongGoiTin { hop_le: false, ..t });
        }
        self.so_goi_da_tach += 1;
        Some(t)
    }

    /// Số chu kỳ để tách một gói. Phần cứng: LUÔN LUÔN 1.
    pub fn chu_ky_tach(&self) -> u32 { 1 }
}

/// Cây XOR: gộp từng cặp, độ sâu ⌈log₂(n)⌉ tầng cổng thay vì n tầng.
/// Đây là mẫu "rút gọn song song" — nền của mọi phép gộp trên phần cứng và GPU.
pub fn cay_xor(du_lieu: &[u8]) -> u32 {
    let mut tang: Vec<u32> = du_lieu.iter().map(|&b| b as u32).collect();
    while tang.len() > 1 {
        let mut tren = Vec::with_capacity(tang.len().div_ceil(2));
        for cap in tang.chunks(2) {
            tren.push(cap[0] ^ cap.get(1).copied().unwrap_or(0));
        }
        tang = tren;
    }
    tang.first().copied().unwrap_or(0)
}

pub fn do_sau_cay_xor(n: usize) -> u32 {
    if n <= 1 { return 0; }
    (n as f64).log2().ceil() as u32
}

/// Cách phần mềm làm: cộng dồn tuần tự, n bước phụ thuộc nhau.
pub fn xor_tuan_tu(du_lieu: &[u8]) -> u32 {
    du_lieu.iter().fold(0u32, |a, &b| a ^ b as u32)
}

// ============================================================================
// 3. SỔ LỆNH TRÊN THANH GHI
// ============================================================================
// Phần mềm dùng BTreeMap: O(log n) nhưng có nhảy con trỏ và trượt cache.
// Phần cứng giữ N mức giá tốt nhất trong THANH GHI và so sánh TẤT CẢ cùng lúc
// bằng một mạng so sánh. Tìm giá tốt nhất tốn đúng 1 chu kỳ, bất kể N.

pub const SO_MUC_PHAN_CUNG: usize = 8;

#[derive(Debug, Clone, Copy, PartialEq, Eq, Default)]
pub struct MucGiaPC { pub gia: i64, pub khoi_luong: u32 }

/// Sổ lệnh "nông nhưng nhanh": chỉ giữ 8 mức tốt nhất mỗi bên. Đủ cho gần
/// như mọi chiến lược, và vừa trọn trong thanh ghi FPGA.
#[derive(Debug, Clone, Copy)]
pub struct SoLenhPhanCung {
    pub mua: [MucGiaPC; SO_MUC_PHAN_CUNG],
    pub ban: [MucGiaPC; SO_MUC_PHAN_CUNG],
}

impl Default for SoLenhPhanCung {
    fn default() -> Self {
        SoLenhPhanCung { mua: [MucGiaPC::default(); SO_MUC_PHAN_CUNG],
                         ban: [MucGiaPC::default(); SO_MUC_PHAN_CUNG] }
    }
}

impl SoLenhPhanCung {
    /// Bộ mã hoá ưu tiên: tìm mức mua có giá CAO nhất. Trên phần cứng đây là
    /// một cây so sánh độ sâu log₂(8) = 3 tầng, chạy trong MỘT chu kỳ.
    /// Phần mềm phải duyệt 8 phần tử — 8 lần so sánh phụ thuộc nhau.
    pub fn mua_tot_nhat(&self) -> Option<MucGiaPC> {
        self.mua.iter().filter(|m| m.khoi_luong > 0).max_by_key(|m| m.gia).copied()
    }
    pub fn ban_tot_nhat(&self) -> Option<MucGiaPC> {
        self.ban.iter().filter(|m| m.khoi_luong > 0).min_by_key(|m| m.gia).copied()
    }
    pub fn chenh_lech(&self) -> Option<i64> {
        Some(self.ban_tot_nhat()?.gia - self.mua_tot_nhat()?.gia)
    }

    /// Cập nhật một mức giá. Mọi ô so sánh SONG SONG với giá đầu vào, nên
    /// dù có 8 hay 64 mức thì vẫn tốn đúng một chu kỳ.
    pub fn cap_nhat(&mut self, la_mua: bool, gia: i64, khoi_luong: u32) {
        let o = if la_mua { &mut self.mua } else { &mut self.ban };
        // Đã có mức giá này chưa?
        if let Some(m) = o.iter_mut().find(|m| m.gia == gia && m.khoi_luong > 0) {
            m.khoi_luong = khoi_luong;
            if khoi_luong == 0 { m.gia = 0; }
            return;
        }
        if khoi_luong == 0 { return; }
        // Ô trống?
        if let Some(m) = o.iter_mut().find(|m| m.khoi_luong == 0) {
            *m = MucGiaPC { gia, khoi_luong };
            return;
        }
        // Đầy: thay mức TỆ NHẤT nếu mức mới tốt hơn
        let te_nhat = if la_mua {
            o.iter_mut().min_by_key(|m| m.gia).unwrap()
        } else {
            o.iter_mut().max_by_key(|m| m.gia).unwrap()
        };
        let tot_hon = if la_mua { gia > te_nhat.gia } else { gia < te_nhat.gia };
        if tot_hon { *te_nhat = MucGiaPC { gia, khoi_luong }; }
    }

    /// Độ sâu cây so sánh — quyết định tần số tối đa của mạch.
    pub fn do_sau_so_sanh() -> u32 { do_sau_cay_xor(SO_MUC_PHAN_CUNG) }

    pub fn so_muc_dang_dung(&self, la_mua: bool) -> usize {
        let o = if la_mua { &self.mua } else { &self.ban };
        o.iter().filter(|m| m.khoi_luong > 0).count()
    }
}

// ============================================================================
// 4. MẠCH KIỂM TRA RỦI RO — tổ hợp thuần tuý, 1 chu kỳ
// ============================================================================
// Toàn bộ cổng rủi ro của Chương 77 nén thành logic tổ hợp: mọi điều kiện
// được tính SONG SONG rồi OR lại. Không có `if` tuần tự, không có nhánh dự
// đoán sai — thời gian luôn bằng nhau, kể cả khi lệnh bị từ chối.

#[derive(Debug, Clone, Copy, PartialEq, Eq, Default)]
pub struct CoTuChoi {
    pub so_luong_khong: bool,
    pub gia_khong: bool,
    pub vuot_gia_tri: bool,
    pub vuot_vi_the: bool,
    pub cong_tac_tat: bool,
}

impl CoTuChoi {
    /// Gộp mọi cờ bằng OR — trên phần cứng là một cổng OR nhiều đầu vào,
    /// độ sâu log₂(số cờ).
    pub fn bi_chan(&self) -> bool {
        self.so_luong_khong || self.gia_khong || self.vuot_gia_tri
            || self.vuot_vi_the || self.cong_tac_tat
    }
    pub fn so_co_bat(&self) -> u32 {
        [self.so_luong_khong, self.gia_khong, self.vuot_gia_tri,
         self.vuot_vi_the, self.cong_tac_tat].iter().filter(|&&x| x).count() as u32
    }
}

#[derive(Debug, Clone, Copy)]
pub struct MachRuiRo {
    pub gia_tri_toi_da: i64,
    pub vi_the_toi_da: i64,
    pub vi_the: i64,
    pub cong_tac_tat: bool,
}

impl MachRuiRo {
    /// TẤT CẢ điều kiện tính song song. Đây là điểm khác biệt cốt lõi so với
    /// phần mềm: dù lệnh hợp lệ hay bị chặn, mạch vẫn tốn đúng một chu kỳ.
    /// Không có "đường nhanh" và "đường chậm" → độ trễ không dao động, và
    /// thời gian phản hồi không tiết lộ điều gì về nội dung lệnh.
    pub fn kiem_tra(&self, la_mua: bool, gia: i64, so_luong: i64) -> CoTuChoi {
        let dau = if la_mua { 1i64 } else { -1 };
        CoTuChoi {
            so_luong_khong: so_luong <= 0,
            gia_khong: gia <= 0,
            vuot_gia_tri: gia.saturating_mul(so_luong) > self.gia_tri_toi_da,
            vuot_vi_the: self.vi_the.saturating_add(dau.saturating_mul(so_luong))
                             .saturating_abs() > self.vi_the_toi_da,
            cong_tac_tat: self.cong_tac_tat,
        }
    }
    pub fn chu_ky_kiem_tra(&self) -> u32 { 1 }
}

// ============================================================================
// 5. ĐƯỜNG ỐNG TICK-TO-TRADE
// ============================================================================

#[derive(Debug, Clone, PartialEq)]
pub struct TangOng { pub ten: String, pub chu_ky: u32 }

#[derive(Debug, PartialEq)]
pub struct DuongOngPhanCung { pub tang: Vec<TangOng> }

impl DuongOngPhanCung {
    /// Đường ống điển hình của một hệ thống giao dịch trên FPGA.
    pub fn dien_hinh() -> Self {
        DuongOngPhanCung {
            tang: vec![
                TangOng { ten: "MAC/PHY nhận khung".into(), chu_ky: 3 },
                TangOng { ten: "Tách trường song song".into(), chu_ky: 1 },
                TangOng { ten: "Cập nhật sổ lệnh".into(), chu_ky: 1 },
                TangOng { ten: "Tính tín hiệu".into(), chu_ky: 2 },
                TangOng { ten: "Kiểm tra rủi ro".into(), chu_ky: 1 },
                TangOng { ten: "Dựng gói lệnh".into(), chu_ky: 2 },
                TangOng { ten: "MAC/PHY phát khung".into(), chu_ky: 3 },
            ],
        }
    }

    /// ĐỘ TRỄ: một gói tin đi hết đường ống mất bao nhiêu chu kỳ.
    pub fn do_tre_chu_ky(&self) -> u32 { self.tang.iter().map(|t| t.chu_ky).sum() }
    pub fn do_tre_ns(&self) -> f64 { chu_ky_sang_ns(self.do_tre_chu_ky()) }

    /// THÔNG LƯỢNG: sau khi ống đầy, cứ mỗi `chu_ky_khoi_dau` là một gói xong.
    /// Bằng chu kỳ của tầng CHẬM NHẤT — không phải tổng các tầng.
    pub fn chu_ky_khoi_dau(&self) -> u32 {
        self.tang.iter().map(|t| t.chu_ky).max().unwrap_or(1)
    }
    pub fn thong_luong_goi_moi_giay(&self) -> f64 {
        1e9 / chu_ky_sang_ns(self.chu_ky_khoi_dau())
    }

    /// Xử lý `n` gói mất bao nhiêu chu kỳ (có đường ống).
    pub fn tong_chu_ky_cho(&self, n: u32) -> u32 {
        if n == 0 { return 0; }
        self.do_tre_chu_ky() + (n - 1) * self.chu_ky_khoi_dau()
    }

    /// Nếu KHÔNG có đường ống: gói sau phải chờ gói trước ra hẳn.
    pub fn tong_chu_ky_khong_ong(&self, n: u32) -> u32 { n * self.do_tre_chu_ky() }
}

/// Ngân sách phần mềm tương ứng, lấy từ Chương 74 (đơn vị nano-giây).
pub fn do_tre_phan_mem_ns() -> f64 { 3_400.0 }

// ============================================================================
// 6. VÌ SAO VẪN CẦN PHẦN MỀM — kiến trúc lai
// ============================================================================
// FPGA rất nhanh nhưng rất khó sửa: một thay đổi nhỏ tốn hàng chục phút tổng
// hợp mạch. Thực tế người ta chia đôi: đường CỰC NÓNG nằm trên FPGA, còn
// logic hay đổi thì nằm trên CPU.

#[derive(Debug, Clone, Copy, PartialEq)]
pub enum NoiThucThi { PhanCung, PhanMem }

#[derive(Debug, Clone, PartialEq)]
pub struct ChucNang {
    pub ten: String,
    pub tan_suat_doi: u32, // số lần sửa mỗi năm
    pub nam_tren_duong_nong: bool,
}

/// Quy tắc chia việc: nằm trên đường nóng VÀ ít thay đổi thì đưa xuống phần
/// cứng. Hay đổi thì giữ trên phần mềm, dù có nóng — vì mỗi lần sửa mạch tốn
/// hàng chục phút, và một chiến lược không thử nghiệm được là chiến lược chết.
pub fn phan_cong(c: &ChucNang) -> NoiThucThi {
    if c.nam_tren_duong_nong && c.tan_suat_doi <= 4 {
        NoiThucThi::PhanCung
    } else {
        NoiThucThi::PhanMem
    }
}

fn main() {
    println!("═══════════════════════════════════════════════════════════");
    println!("   FPGA CHO GIAO DỊCH: TICK-TO-TRADE TÍNH BẰNG CHU KỲ      ");
    println!("═══════════════════════════════════════════════════════════");

    println!("\n1. TÁCH TRƯỜNG SONG SONG");
    let mut bt = BoTachTruong::default();
    let mut goi = vec![b'A'];
    goi.extend_from_slice(&7u32.to_be_bytes());
    goi.extend_from_slice(&8_450i64.to_be_bytes());
    goi.extend_from_slice(&100u32.to_be_bytes());
    let tk = cay_xor(&goi) & 0x00FF_FFFF;
    goi.extend_from_slice(&tk.to_be_bytes()[1..]);
    let t = bt.tach(&goi).unwrap();
    println!("   Gói {} byte → loại {:?} · mã ck {} · giá {} · số lượng {} · hợp lệ {}",
             goi.len(), t.loai as char, t.ma_ck, t.gia, t.so_luong, t.hop_le);
    println!("   Phần cứng tách TẤT CẢ trường trong {} chu kỳ = {} ns",
             bt.chu_ky_tach(), chu_ky_sang_ns(bt.chu_ky_tach()));

    println!("\n2. CÂY XOR — rút gọn song song");
    println!("   {:>8} {:>18} {:>18}", "số byte", "cây (log n tầng)", "tuần tự (n tầng)");
    for n in [4usize, 16, 64, 256, 1024] {
        println!("   {:>8} {:>18} {:>18}", n, do_sau_cay_xor(n), n);
    }
    let d: Vec<u8> = (0..=255).collect();
    println!("   Cùng kết quả với cách tuần tự: {}", cay_xor(&d) == xor_tuan_tu(&d));

    println!("\n3. SỔ LỆNH TRÊN THANH GHI");
    let mut so = SoLenhPhanCung::default();
    for (g, kl) in [(8_400i64, 500u32), (8_390, 300), (8_380, 200)] {
        so.cap_nhat(true, g, kl);
    }
    for (g, kl) in [(8_410i64, 400u32), (8_420, 250)] { so.cap_nhat(false, g, kl); }
    println!("   Mua tốt nhất {:?} · bán tốt nhất {:?}",
             so.mua_tot_nhat().unwrap(), so.ban_tot_nhat().unwrap());
    println!("   Chênh lệch {} tick · tìm giá tốt nhất tốn {} tầng so sánh = 1 chu kỳ",
             so.chenh_lech().unwrap(), SoLenhPhanCung::do_sau_so_sanh());

    println!("\n4. MẠCH KIỂM TRA RỦI RO — thời gian KHÔNG đổi");
    let m = MachRuiRo { gia_tri_toi_da: 1_000_000, vi_the_toi_da: 500,
                        vi_the: 0, cong_tac_tat: false };
    for (mo_ta, gia, sl) in [("hợp lệ        ", 8_400i64, 100i64),
                             ("số lượng âm   ", 8_400, -5),
                             ("giá trị quá to", 8_400, 1_000),
                             ("cả hai lỗi    ", 0, -1)] {
        let c = m.kiem_tra(true, gia, sl);
        println!("   {} → chặn {:<5} ({} cờ bật) · luôn {} chu kỳ",
                 mo_ta, c.bi_chan(), c.so_co_bat(), m.chu_ky_kiem_tra());
    }
    println!("   → Hợp lệ hay không cũng tốn đúng một chu kỳ: độ trễ không dao động,");
    println!("     và thời gian phản hồi không tiết lộ gì về nội dung lệnh.");

    println!("\n5. ĐƯỜNG ỐNG TICK-TO-TRADE");
    let ong = DuongOngPhanCung::dien_hinh();
    for t in &ong.tang {
        println!("   {:<26} {} chu kỳ = {:>4.0} ns", t.ten, t.chu_ky, chu_ky_sang_ns(t.chu_ky));
    }
    println!("   ─────────────────────────────────────────");
    println!("   Độ trễ     : {} chu kỳ = {:.0} ns", ong.do_tre_chu_ky(), ong.do_tre_ns());
    println!("   Thông lượng: 1 gói mỗi {} chu kỳ = {:.0} triệu gói/giây",
             ong.chu_ky_khoi_dau(), ong.thong_luong_goi_moi_giay() / 1e6);
    println!("   So với phần mềm ({} ns) → nhanh gấp {:.0} lần",
             do_tre_phan_mem_ns(), do_tre_phan_mem_ns() / ong.do_tre_ns());

    println!("\n6. ĐƯỜNG ỐNG SO VỚI KHÔNG ĐƯỜNG ỐNG (1000 gói)");
    println!("   Có ống   : {:>7} chu kỳ", ong.tong_chu_ky_cho(1_000));
    println!("   Không ống: {:>7} chu kỳ", ong.tong_chu_ky_khong_ong(1_000));
    println!("   → Nhanh gấp {:.1} lần về THÔNG LƯỢNG, nhưng ĐỘ TRỄ vẫn y nguyên {} ns.",
             ong.tong_chu_ky_khong_ong(1_000) as f64 / ong.tong_chu_ky_cho(1_000) as f64,
             ong.do_tre_ns());

    println!("\n7. CHIA VIỆC GIỮA PHẦN CỨNG VÀ PHẦN MỀM");
    let cn = vec![
        ChucNang { ten: "Tách gói tin".into(), tan_suat_doi: 1, nam_tren_duong_nong: true },
        ChucNang { ten: "Cập nhật sổ lệnh".into(), tan_suat_doi: 2, nam_tren_duong_nong: true },
        ChucNang { ten: "Kiểm tra rủi ro cứng".into(), tan_suat_doi: 3, nam_tren_duong_nong: true },
        ChucNang { ten: "Logic chiến lược".into(), tan_suat_doi: 200, nam_tren_duong_nong: true },
        ChucNang { ten: "Báo cáo cuối ngày".into(), tan_suat_doi: 12, nam_tren_duong_nong: false },
        ChucNang { ten: "Hiệu chỉnh tham số".into(), tan_suat_doi: 500, nam_tren_duong_nong: true },
    ];
    for c in &cn {
        println!("   {:<24} đổi {:>3} lần/năm · nóng {:<5} → {:?}",
                 c.ten, c.tan_suat_doi, c.nam_tren_duong_nong, phan_cong(c));
    }
    println!("   → Chiến lược ở lại phần mềm dù rất nóng: một chiến lược không");
    println!("     thử nghiệm được là chiến lược chết, dù nó nhanh tới đâu.");

    println!("\n═══════════════════════════════════════════════════════════");
    println!("   PHẦN CỨNG THẮNG Ở SỰ ỔN ĐỊNH, KHÔNG CHỈ Ở TỐC ĐỘ         ");
    println!("═══════════════════════════════════════════════════════════");
}

#[cfg(test)]
mod kiem_thu {
    use super::*;

    fn goi_hop_le(loai: u8, ma_ck: u32, gia: i64, sl: u32) -> Vec<u8> {
        let mut g = vec![loai];
        g.extend_from_slice(&ma_ck.to_be_bytes());
        g.extend_from_slice(&gia.to_be_bytes());
        g.extend_from_slice(&sl.to_be_bytes());
        let tk = cay_xor(&g) & 0x00FF_FFFF;
        g.extend_from_slice(&tk.to_be_bytes()[1..]);
        g
    }

    // ---------- Cây XOR ----------
    #[test]
    fn cay_xor_cho_cung_ket_qua_voi_tuan_tu() {
        // Bất biến: song song hoá KHÔNG được đổi kết quả. XOR có tính kết hợp
        // và giao hoán nên gộp theo cây hay theo chuỗi đều như nhau.
        for n in [0usize, 1, 2, 3, 4, 7, 16, 17, 64, 255, 256] {
            let d: Vec<u8> = (0..n).map(|i| (i * 37 % 251) as u8).collect();
            assert_eq!(cay_xor(&d), xor_tuan_tu(&d), "n={}", n);
        }
    }

    #[test]
    fn do_sau_cay_la_log_chu_khong_tuyen_tinh() {
        assert_eq!(do_sau_cay_xor(1), 0);
        assert_eq!(do_sau_cay_xor(2), 1);
        assert_eq!(do_sau_cay_xor(4), 2);
        assert_eq!(do_sau_cay_xor(256), 8);
        assert_eq!(do_sau_cay_xor(1024), 10, "1024 byte chỉ cần 10 tầng, không phải 1024");
    }

    // ---------- Tách trường ----------
    #[test]
    fn tach_dung_moi_truong() {
        let mut bt = BoTachTruong::default();
        let g = goi_hop_le(b'A', 12_345, 8_450, 100);
        let t = bt.tach(&g).unwrap();
        assert_eq!(t.loai, b'A');
        assert_eq!(t.ma_ck, 12_345);
        assert_eq!(t.gia, 8_450);
        assert_eq!(t.so_luong, 100);
        assert!(t.hop_le);
        assert_eq!(bt.so_goi_da_tach, 1);
        assert_eq!(bt.so_goi_hong, 0);
    }

    #[test]
    fn tach_dung_ca_gia_am_va_gia_tri_bien() {
        let mut bt = BoTachTruong::default();
        for (gia, sl) in [(-1i64, 0u32), (i64::MIN, u32::MAX), (i64::MAX, 1)] {
            let g = goi_hop_le(b'X', 0, gia, sl);
            let t = bt.tach(&g).unwrap();
            assert_eq!(t.gia, gia, "giá {} phải tách đúng", gia);
            assert_eq!(t.so_luong, sl);
        }
    }

    #[test]
    fn goi_qua_ngan_bi_tu_choi() {
        let mut bt = BoTachTruong::default();
        for n in 0..DAI_GOI {
            assert_eq!(bt.tach(&vec![0u8; n]), None, "gói {} byte phải bị từ chối", n);
        }
        assert_eq!(bt.so_goi_hong, DAI_GOI as u64);
    }

    #[test]
    fn tong_kiem_tra_sai_bi_danh_dau_khong_hop_le() {
        let mut bt = BoTachTruong::default();
        let mut g = goi_hop_le(b'A', 1, 100, 10);
        g[19] ^= 0xFF; // phá tổng kiểm tra
        let t = bt.tach(&g).unwrap();
        assert!(!t.hop_le, "gói hỏng phải bị đánh dấu, KHÔNG được im lặng cho qua");
        assert_eq!(bt.so_goi_hong, 1);
        assert_eq!(bt.so_goi_da_tach, 0);
    }

    #[test]
    fn lat_mot_bit_trong_than_goi_bi_bat() {
        let mut bt = BoTachTruong::default();
        for vi_tri in 0..17usize {
            let mut g = goi_hop_le(b'A', 999, 8_400, 500);
            g[vi_tri] ^= 1;
            let t = bt.tach(&g).unwrap();
            assert!(!t.hop_le, "lật bit ở byte {} mà không bị phát hiện", vi_tri);
        }
    }

    #[test]
    fn tach_luon_ton_dung_mot_chu_ky() {
        let bt = BoTachTruong::default();
        assert_eq!(bt.chu_ky_tach(), 1, "phần cứng tách mọi trường song song");
    }

    // ---------- Sổ lệnh phần cứng ----------
    #[test]
    fn so_rong_khong_co_gia_tot_nhat() {
        let s = SoLenhPhanCung::default();
        assert_eq!(s.mua_tot_nhat(), None);
        assert_eq!(s.ban_tot_nhat(), None);
        assert_eq!(s.chenh_lech(), None);
    }

    #[test]
    fn tra_dung_gia_tot_nhat_hai_ben() {
        let mut s = SoLenhPhanCung::default();
        for (g, kl) in [(8_380i64, 100u32), (8_400, 500), (8_390, 300)] {
            s.cap_nhat(true, g, kl);
        }
        for (g, kl) in [(8_430i64, 100u32), (8_410, 400), (8_420, 250)] {
            s.cap_nhat(false, g, kl);
        }
        assert_eq!(s.mua_tot_nhat().unwrap().gia, 8_400, "bên mua lấy giá CAO nhất");
        assert_eq!(s.ban_tot_nhat().unwrap().gia, 8_410, "bên bán lấy giá THẤP nhất");
        assert_eq!(s.chenh_lech(), Some(10));
    }

    #[test]
    fn cap_nhat_muc_da_co_thi_ghi_de_khoi_luong() {
        let mut s = SoLenhPhanCung::default();
        s.cap_nhat(true, 8_400, 500);
        s.cap_nhat(true, 8_400, 700);
        assert_eq!(s.so_muc_dang_dung(true), 1, "không được tạo mức trùng");
        assert_eq!(s.mua_tot_nhat().unwrap().khoi_luong, 700);
    }

    #[test]
    fn khoi_luong_ve_khong_thi_muc_bien_mat() {
        let mut s = SoLenhPhanCung::default();
        s.cap_nhat(true, 8_400, 500);
        s.cap_nhat(true, 8_390, 300);
        s.cap_nhat(true, 8_400, 0);
        assert_eq!(s.mua_tot_nhat().unwrap().gia, 8_390, "đỉnh phải tụt xuống mức kế");
        assert_eq!(s.so_muc_dang_dung(true), 1);
    }

    #[test]
    fn so_day_thi_giu_lai_cac_muc_tot_nhat() {
        // Sổ phần cứng chỉ có 8 ô. Khi đầy, mức tệ nhất phải bị đẩy ra —
        // nếu không, ta sẽ giữ những mức giá vô dụng và bỏ mất mức tốt.
        let mut s = SoLenhPhanCung::default();
        for i in 0..SO_MUC_PHAN_CUNG as i64 {
            s.cap_nhat(true, 8_000 + i, 100);
        }
        assert_eq!(s.so_muc_dang_dung(true), SO_MUC_PHAN_CUNG);
        assert_eq!(s.mua_tot_nhat().unwrap().gia, 8_007);
        // Mức tốt hơn hẳn → phải chen vào được
        s.cap_nhat(true, 9_000, 100);
        assert_eq!(s.mua_tot_nhat().unwrap().gia, 9_000);
        assert_eq!(s.so_muc_dang_dung(true), SO_MUC_PHAN_CUNG, "vẫn đúng 8 ô");
        // Mức tệ hơn tất cả → phải bị bỏ qua
        s.cap_nhat(true, 1, 100);
        assert!(s.mua.iter().all(|m| m.gia != 1), "mức tệ không được chiếm chỗ");
    }

    #[test]
    fn ben_ban_cung_giu_lai_muc_tot_nhat() {
        let mut s = SoLenhPhanCung::default();
        for i in 0..SO_MUC_PHAN_CUNG as i64 {
            s.cap_nhat(false, 9_000 - i, 100);
        }
        assert_eq!(s.ban_tot_nhat().unwrap().gia, 8_993);
        s.cap_nhat(false, 8_000, 100); // rẻ hơn hẳn = tốt hơn cho bên bán
        assert_eq!(s.ban_tot_nhat().unwrap().gia, 8_000);
        s.cap_nhat(false, 99_999, 100); // đắt vô lý
        assert!(s.ban.iter().all(|m| m.gia != 99_999));
    }

    #[test]
    fn do_sau_so_sanh_la_log_so_muc() {
        assert_eq!(SoLenhPhanCung::do_sau_so_sanh(), 3, "8 mức → 3 tầng cây so sánh");
    }

    // ---------- Mạch rủi ro ----------
    fn mach() -> MachRuiRo {
        MachRuiRo { gia_tri_toi_da: 1_000_000, vi_the_toi_da: 500,
                    vi_the: 0, cong_tac_tat: false }
    }

    #[test]
    fn lenh_hop_le_khong_bat_co_nao() {
        let c = mach().kiem_tra(true, 8_400, 100);
        assert!(!c.bi_chan());
        assert_eq!(c.so_co_bat(), 0);
    }

    #[test]
    fn moi_dieu_kien_bat_dung_co_cua_no() {
        let m = mach();
        assert!(m.kiem_tra(true, 8_400, 0).so_luong_khong);
        assert!(m.kiem_tra(true, 0, 100).gia_khong);
        assert!(m.kiem_tra(true, 8_400, 1_000).vuot_gia_tri);
        assert!(m.kiem_tra(true, 100, 600).vuot_vi_the);
        let tat = MachRuiRo { cong_tac_tat: true, ..m };
        assert!(tat.kiem_tra(true, 8_400, 100).cong_tac_tat);
    }

    #[test]
    fn nhieu_loi_cung_luc_bat_nhieu_co_cung_luc() {
        // Đây là điểm khác biệt thật so với phần mềm: phần mềm `return` ở lỗi
        // ĐẦU TIÊN nên chỉ biết một lỗi; mạch tính song song nên thấy HẾT.
        let c = mach().kiem_tra(true, 0, -1);
        assert!(c.so_luong_khong && c.gia_khong);
        assert!(c.so_co_bat() >= 2, "phần cứng thấy mọi lỗi cùng lúc, không dừng ở lỗi đầu");
    }

    #[test]
    fn ban_khong_cung_bi_chan_boi_han_muc_vi_the() {
        let m = mach();
        assert!(m.kiem_tra(false, 100, 600).vuot_vi_the, "chiều bán cũng phải bị chặn");
    }

    #[test]
    fn vi_the_hien_tai_duoc_tinh_vao() {
        let m = MachRuiRo { vi_the: 450, ..mach() };
        assert!(!m.kiem_tra(true, 100, 50).vuot_vi_the, "450+50 = 500, vừa trần");
        assert!(m.kiem_tra(true, 100, 51).vuot_vi_the, "450+51 vượt trần");
        assert!(!m.kiem_tra(false, 100, 500).vuot_vi_the, "bán thì giảm vị thế");
    }

    #[test]
    fn phep_nhan_khong_bao_gio_tran_so() {
        let m = mach();
        // Toàn bộ dùng phép bão hoà: không được panic, và phải báo vượt hạn mức
        let c = m.kiem_tra(true, i64::MAX, i64::MAX);
        assert!(c.vuot_gia_tri);
        assert!(c.bi_chan());
        let c2 = m.kiem_tra(false, 1, i64::MAX);
        assert!(c2.bi_chan());
    }

    #[test]
    fn kiem_tra_luon_ton_dung_mot_chu_ky() {
        // Bất biến quan trọng nhất của mạch rủi ro: thời gian KHÔNG phụ thuộc
        // dữ liệu. Nhờ vậy độ trễ không dao động và không rò rỉ thông tin.
        let m = mach();
        assert_eq!(m.chu_ky_kiem_tra(), 1);
        for (g, sl) in [(8_400i64, 100i64), (0, 0), (-1, -1), (i64::MAX, i64::MAX)] {
            let _ = m.kiem_tra(true, g, sl);
            assert_eq!(m.chu_ky_kiem_tra(), 1, "mọi đầu vào đều tốn đúng 1 chu kỳ");
        }
    }

    // ---------- Đường ống ----------
    #[test]
    fn do_tre_bang_tong_cac_tang() {
        let o = DuongOngPhanCung::dien_hinh();
        assert_eq!(o.do_tre_chu_ky(), 3 + 1 + 1 + 2 + 1 + 2 + 3);
        assert!((o.do_tre_ns() - 13.0 * NS_MOI_CHU_KY).abs() < 1e-9);
    }

    #[test]
    fn thong_luong_bang_tang_cham_nhat_khong_phai_tong() {
        // Nhầm hai đại lượng này là hiểu sai toàn bộ kiến trúc đường ống.
        let o = DuongOngPhanCung::dien_hinh();
        assert_eq!(o.chu_ky_khoi_dau(), 3, "tầng chậm nhất là 3 chu kỳ");
        assert!(o.chu_ky_khoi_dau() < o.do_tre_chu_ky());
    }

    #[test]
    fn duong_ong_tang_thong_luong_nhung_khong_giam_do_tre() {
        let o = DuongOngPhanCung::dien_hinh();
        // Một gói: y hệt nhau
        assert_eq!(o.tong_chu_ky_cho(1), o.do_tre_chu_ky());
        assert_eq!(o.tong_chu_ky_khong_ong(1), o.do_tre_chu_ky());
        // Nhiều gói: đường ống thắng đậm
        assert!(o.tong_chu_ky_cho(1_000) * 4 < o.tong_chu_ky_khong_ong(1_000));
        // Nhưng độ trễ của MỘT gói vẫn y nguyên
        assert_eq!(o.do_tre_chu_ky(), 13);
    }

    #[test]
    fn khong_goi_nao_thi_khong_ton_chu_ky_nao() {
        let o = DuongOngPhanCung::dien_hinh();
        assert_eq!(o.tong_chu_ky_cho(0), 0);
        assert_eq!(o.tong_chu_ky_khong_ong(0), 0);
    }

    #[test]
    fn phan_cung_nhanh_hon_phan_mem_hang_chuc_lan() {
        let o = DuongOngPhanCung::dien_hinh();
        let ty_le = do_tre_phan_mem_ns() / o.do_tre_ns();
        assert!(ty_le > 50.0, "phải nhanh hơn ít nhất 50 lần, thực tế {:.0}", ty_le);
        assert!(o.do_tre_ns() < 100.0, "tick-to-trade phải dưới 100 ns");
    }

    #[test]
    fn thong_luong_dat_hang_tram_trieu_goi_moi_giay() {
        let o = DuongOngPhanCung::dien_hinh();
        assert!(o.thong_luong_goi_moi_giay() > 50e6,
                "phải trên 50 triệu gói/giây, thực tế {:.0}", o.thong_luong_goi_moi_giay());
    }

    // ---------- Phân công phần cứng/phần mềm ----------
    #[test]
    fn viec_nong_va_it_doi_thi_xuong_phan_cung() {
        let c = ChucNang { ten: "tách gói".into(), tan_suat_doi: 1, nam_tren_duong_nong: true };
        assert_eq!(phan_cong(&c), NoiThucThi::PhanCung);
    }

    #[test]
    fn viec_hay_doi_thi_o_lai_phan_mem_du_rat_nong() {
        // Bài học kiến trúc quan trọng nhất của chương: tốc độ không đáng giá
        // bằng khả năng thay đổi. Chiến lược sửa 200 lần/năm mà nằm trên FPGA
        // thì mỗi lần thử nghiệm tốn hàng chục phút tổng hợp mạch.
        let c = ChucNang { ten: "chiến lược".into(), tan_suat_doi: 200,
                           nam_tren_duong_nong: true };
        assert_eq!(phan_cong(&c), NoiThucThi::PhanMem);
    }

    #[test]
    fn viec_khong_nong_thi_o_phan_mem_du_it_doi() {
        let c = ChucNang { ten: "báo cáo".into(), tan_suat_doi: 1,
                           nam_tren_duong_nong: false };
        assert_eq!(phan_cong(&c), NoiThucThi::PhanMem,
                   "không nằm trên đường nóng thì đưa xuống phần cứng là lãng phí");
    }

    #[test]
    fn quy_doi_chu_ky_sang_nano_giay_dung() {
        assert!((chu_ky_sang_ns(1) - 4.0).abs() < 1e-9);
        assert!((chu_ky_sang_ns(250) - 1_000.0).abs() < 1e-9, "250 chu kỳ ở 250 MHz = 1 µs");
        assert_eq!(chu_ky_sang_ns(0), 0.0);
    }
}
