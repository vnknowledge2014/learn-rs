#![allow(dead_code, unused_variables)]
//! Chương 56 — Kỹ nghệ Ngữ cảnh & Tác tử: Context, Harness, Loop, Graph Engineering.
//! Toàn bộ chạy offline: mô hình ngôn ngữ được thay bằng một bản giả tất định,
//! đúng tinh thần "test double" ở Chương 55 — nhờ vậy mọi thứ kiểm thử được.

use std::collections::{HashMap, HashSet, VecDeque};

// ============================================================================
// PHẦN 1: NGÂN SÁCH NGỮ CẢNH — CONTEXT ENGINEERING
// ============================================================================

/// Một mẩu ngữ cảnh có thể nạp vào cửa sổ của mô hình.
#[derive(Debug, Clone, PartialEq)]
pub struct MauNguCanh {
    pub nhan: String,
    pub noi_dung: String,
    pub token: usize,
    /// Điểm liên quan tới truy vấn hiện tại (0.0 – 1.0).
    pub lien_quan: f64,
    /// Ghim cứng: luôn nạp bất kể ngân sách (ví dụ: quy tắc an toàn).
    pub ghim: bool,
}

/// Kết quả sau khi cắt gọt theo ngân sách.
#[derive(Debug, PartialEq)]
pub struct GoiNguCanh {
    pub cac_mau: Vec<MauNguCanh>,
    pub tong_token: usize,
    pub so_mau_bi_loai: usize,
}

/// CONTEXT ENGINEERING: chọn tập con ngữ cảnh tốt nhất trong ngân sách token.
/// Đây là bài toán xếp ba lô (knapsack) đơn giản hóa: ưu tiên điểm liên quan
/// trên mỗi token, và luôn giữ các mẩu bị ghim.
pub fn dong_goi_ngu_canh(mut mau: Vec<MauNguCanh>, ngan_sach: usize) -> GoiNguCanh {
    let tong_ban_dau = mau.len();

    // 1. Tách phần ghim cứng — luôn được nạp trước
    let (ghim, mut tuy_chon): (Vec<_>, Vec<_>) = mau.drain(..).partition(|m| m.ghim);
    let mut da_dung: usize = ghim.iter().map(|m| m.token).sum();
    let mut chon: Vec<MauNguCanh> = ghim;

    // 2. Xếp phần còn lại theo MẬT ĐỘ giá trị (liên quan / token) giảm dần
    tuy_chon.sort_by(|a, b| {
        let ma = a.lien_quan / a.token.max(1) as f64;
        let mb = b.lien_quan / b.token.max(1) as f64;
        mb.partial_cmp(&ma).unwrap_or(std::cmp::Ordering::Equal)
            .then(a.nhan.cmp(&b.nhan)) // phá hòa tất định
    });

    // 3. Nhồi vào cho tới khi hết ngân sách
    for m in tuy_chon {
        if da_dung + m.token <= ngan_sach {
            da_dung += m.token;
            chon.push(m);
        }
    }

    // 4. "Lost in the middle": đặt mẩu quan trọng nhất ở ĐẦU và CUỐI
    chon = sap_xep_chong_lang_quen(chon);

    GoiNguCanh {
        tong_token: da_dung,
        so_mau_bi_loai: tong_ban_dau - chon.len(),
        cac_mau: chon,
    }
}

/// Chống hiện tượng "Lost in the Middle": mô hình nhớ tốt phần đầu và phần cuối,
/// hay quên phần giữa. Vậy hãy đẩy thứ quan trọng nhất ra hai đầu.
pub fn sap_xep_chong_lang_quen(mut mau: Vec<MauNguCanh>) -> Vec<MauNguCanh> {
    mau.sort_by(|a, b| {
        b.lien_quan.partial_cmp(&a.lien_quan).unwrap_or(std::cmp::Ordering::Equal)
            .then(a.nhan.cmp(&b.nhan))
    });
    let mut dau: Vec<MauNguCanh> = Vec::new();
    let mut cuoi: Vec<MauNguCanh> = Vec::new();
    for (i, m) in mau.into_iter().enumerate() {
        if i % 2 == 0 { dau.push(m) } else { cuoi.push(m) }
    }
    cuoi.reverse();
    dau.extend(cuoi);
    dau
}

// ============================================================================
// PHẦN 2: HARNESS ENGINEERING — ĐỊNH NGHĨA KHÔNG GIAN HÀNH ĐỘNG CỦA TÁC TỬ
// ============================================================================

#[derive(Debug, Clone, PartialEq)]
pub enum KetQuaCongCu {
    Xong(String),
    Loi(String),
}

/// Một CÔNG CỤ mà tác tử được phép gọi. Đây chính là "harness":
/// bạn định nghĩa tác tử ĐƯỢC LÀM GÌ, và mọi thứ khác đều bị cấm.
pub trait CongCu {
    fn ten(&self) -> &str;
    fn mo_ta(&self) -> &str;
    fn chay(&self, tham_so: &str) -> KetQuaCongCu;
}

pub struct CongCuTinhToan;
impl CongCu for CongCuTinhToan {
    fn ten(&self) -> &str { "tinh_tong" }
    fn mo_ta(&self) -> &str { "Cộng các số cách nhau bởi dấu phẩy. Ví dụ: \"3,4,5\"" }
    fn chay(&self, tham_so: &str) -> KetQuaCongCu {
        let mut tong: i64 = 0;
        for phan in tham_so.split(',') {
            match phan.trim().parse::<i64>() {
                Ok(n) => tong += n,
                Err(_) => return KetQuaCongCu::Loi(format!("{:?} không phải số", phan.trim())),
            }
        }
        KetQuaCongCu::Xong(tong.to_string())
    }
}

pub struct CongCuTraCuu {
    pub kho: HashMap<String, String>,
}
impl CongCu for CongCuTraCuu {
    fn ten(&self) -> &str { "tra_cuu" }
    fn mo_ta(&self) -> &str { "Tra cứu định nghĩa một thuật ngữ trong kho tri thức." }
    fn chay(&self, tham_so: &str) -> KetQuaCongCu {
        match self.kho.get(tham_so.trim()) {
            Some(v) => KetQuaCongCu::Xong(v.clone()),
            None => KetQuaCongCu::Loi(format!("Không tìm thấy {:?}", tham_so.trim())),
        }
    }
}

/// Bộ khung (harness) giữ danh mục công cụ và ÁP ĐẶT GIỚI HẠN.
pub struct BoKhung {
    cong_cu: Vec<Box<dyn CongCu>>,
    pub so_lan_goi_toi_da: usize,
}

impl BoKhung {
    pub fn moi(so_lan_goi_toi_da: usize) -> Self {
        BoKhung { cong_cu: Vec::new(), so_lan_goi_toi_da }
    }
    pub fn dang_ky(mut self, cc: Box<dyn CongCu>) -> Self {
        self.cong_cu.push(cc);
        self
    }
    /// Bản mô tả công cụ để nhét vào ngữ cảnh — đây là "giao diện" tác tử nhìn thấy.
    pub fn mo_ta_cong_cu(&self) -> String {
        self.cong_cu.iter()
            .map(|c| format!("- {}: {}", c.ten(), c.mo_ta()))
            .collect::<Vec<_>>().join("\n")
    }
    pub fn goi(&self, ten: &str, tham_so: &str) -> KetQuaCongCu {
        match self.cong_cu.iter().find(|c| c.ten() == ten) {
            Some(c) => c.chay(tham_so),
            None => KetQuaCongCu::Loi(format!("Công cụ {:?} không tồn tại trong bộ khung", ten)),
        }
    }
}

// ============================================================================
// PHẦN 3: LOOP ENGINEERING — VÒNG LẶP TÁC TỬ CÓ ĐIỀU KIỆN DỪNG
// ============================================================================

#[derive(Debug, Clone, PartialEq)]
pub enum HanhDong {
    GoiCongCu { ten: String, tham_so: String },
    TraLoi(String),
}

/// Bộ não của tác tử. Trong thực tế đây là lời gọi tới mô hình ngôn ngữ;
/// ở đây ta dùng một bản GIẢ TẤT ĐỊNH để chương trình kiểm thử được.
pub trait BoNao {
    fn quyet_dinh(&self, nhiem_vu: &str, lich_su: &[String]) -> HanhDong;
}

#[derive(Debug, PartialEq)]
pub enum LyDoDung {
    HoanThanh,
    HetLuotGoi,
    LapVoHan,
}

#[derive(Debug, PartialEq)]
pub struct KetQuaVongLap {
    pub tra_loi: Option<String>,
    pub so_buoc: usize,
    pub ly_do_dung: LyDoDung,
    pub nhat_ky: Vec<String>,
}

/// LOOP ENGINEERING: vòng lặp tác tử với BA điều kiện dừng bắt buộc.
/// Một vòng lặp thiếu điều kiện dừng là một hóa đơn API không giới hạn.
pub fn chay_vong_lap(nhiem_vu: &str, nao: &dyn BoNao, khung: &BoKhung) -> KetQuaVongLap {
    let mut lich_su: Vec<String> = Vec::new();
    let mut da_thay: HashSet<String> = HashSet::new();

    for buoc in 1..=khung.so_lan_goi_toi_da {
        match nao.quyet_dinh(nhiem_vu, &lich_su) {
            HanhDong::TraLoi(t) => {
                lich_su.push(format!("[{}] TRẢ LỜI: {}", buoc, t));
                return KetQuaVongLap {
                    tra_loi: Some(t), so_buoc: buoc,
                    ly_do_dung: LyDoDung::HoanThanh, nhat_ky: lich_su,
                };
            }
            HanhDong::GoiCongCu { ten, tham_so } => {
                // DỪNG #3: phát hiện lặp vô hạn (gọi y hệt lần trước)
                let dau_van_tay = format!("{}::{}", ten, tham_so);
                if !da_thay.insert(dau_van_tay.clone()) {
                    lich_su.push(format!("[{}] PHÁT HIỆN LẶP: {}", buoc, dau_van_tay));
                    return KetQuaVongLap {
                        tra_loi: None, so_buoc: buoc,
                        ly_do_dung: LyDoDung::LapVoHan, nhat_ky: lich_su,
                    };
                }
                let kq = khung.goi(&ten, &tham_so);
                lich_su.push(match kq {
                    KetQuaCongCu::Xong(v) => format!("[{}] {}({}) -> {}", buoc, ten, tham_so, v),
                    KetQuaCongCu::Loi(e) => format!("[{}] {}({}) -> LỖI: {}", buoc, ten, tham_so, e),
                });
            }
        }
    }
    // DỪNG #2: hết ngân sách lượt gọi
    KetQuaVongLap {
        tra_loi: None, so_buoc: khung.so_lan_goi_toi_da,
        ly_do_dung: LyDoDung::HetLuotGoi, nhat_ky: lich_su,
    }
}

// ============================================================================
// PHẦN 4: GRAPH ENGINEERING — ĐỒ THỊ TRI THỨC & TRUY XUẤT NHIỀU BƯỚC
// ============================================================================

/// Đồ thị tri thức: các thực thể nối với nhau bằng quan hệ có nhãn.
/// Đây là nền của GraphRAG — truy xuất theo QUAN HỆ, không chỉ theo từ khóa.
pub struct DoThiTriThuc {
    canh: HashMap<String, Vec<(String, String)>>, // đỉnh -> [(nhãn quan hệ, đỉnh đích)]
    mo_ta: HashMap<String, String>,
}

impl DoThiTriThuc {
    pub fn moi() -> Self {
        DoThiTriThuc { canh: HashMap::new(), mo_ta: HashMap::new() }
    }
    pub fn them_thuc_the(&mut self, ten: &str, mo_ta: &str) {
        self.mo_ta.insert(ten.to_string(), mo_ta.to_string());
        self.canh.entry(ten.to_string()).or_default();
    }
    pub fn them_quan_he(&mut self, tu: &str, nhan: &str, den: &str) {
        self.canh.entry(tu.to_string()).or_default()
            .push((nhan.to_string(), den.to_string()));
    }

    /// Truy xuất nhiều bước: từ một điểm xuất phát, đi tối đa `do_sau` bước
    /// để gom ngữ cảnh liên quan. Đây là điểm khác biệt so với tìm kiếm phẳng.
    pub fn truy_xuat_lan_toa(&self, bat_dau: &str, do_sau: usize) -> Vec<String> {
        let mut ket_qua = Vec::new();
        let mut da_tham: HashSet<String> = HashSet::new();
        let mut hang_doi: VecDeque<(String, usize)> = VecDeque::new();

        hang_doi.push_back((bat_dau.to_string(), 0));
        da_tham.insert(bat_dau.to_string());

        while let Some((dinh, sau)) = hang_doi.pop_front() {
            if let Some(m) = self.mo_ta.get(&dinh) {
                ket_qua.push(format!("{}: {}", dinh, m));
            }
            if sau >= do_sau { continue; }
            if let Some(lang_gieng) = self.canh.get(&dinh) {
                let mut sx = lang_gieng.clone();
                sx.sort(); // tất định
                for (nhan, den) in sx {
                    if da_tham.insert(den.clone()) {
                        ket_qua.push(format!("  ({} --{}--> {})", dinh, nhan, den));
                        hang_doi.push_back((den, sau + 1));
                    }
                }
            }
        }
        ket_qua
    }
}

// ============================================================================
// PHẦN 5: BỘ NÃO GIẢ TẤT ĐỊNH (test double cho mô hình ngôn ngữ)
// ============================================================================

/// Bộ não giả: quyết định dựa trên luật cố định, nên chương trình TẤT ĐỊNH
/// và kiểm thử được — không cần khóa API, không cần mạng.
pub struct BoNaoGia {
    pub kich_ban: Vec<HanhDong>,
}
impl BoNao for BoNaoGia {
    fn quyet_dinh(&self, _nhiem_vu: &str, lich_su: &[String]) -> HanhDong {
        self.kich_ban
            .get(lich_su.len())
            .cloned()
            .unwrap_or_else(|| HanhDong::TraLoi("Hết kịch bản".to_string()))
    }
}

fn main() {
    println!("═══════════════════════════════════════════════════════════════");
    println!("   KỸ NGHỆ NGỮ CẢNH · BỘ KHUNG · VÒNG LẶP · ĐỒ THỊ TRI THỨC    ");
    println!("═══════════════════════════════════════════════════════════════");

    // ---- 1. CONTEXT ENGINEERING ----
    println!("\n1. KỸ NGHỆ NGỮ CẢNH — nhồi 4000 token vào cửa sổ 1000 token");
    let mau = vec![
        MauNguCanh { nhan: "quy_tac_an_toan".into(), noi_dung: "Không tiết lộ khóa bí mật".into(), token: 50, lien_quan: 0.3, ghim: true },
        MauNguCanh { nhan: "tai_lieu_A".into(), noi_dung: "...".into(), token: 800, lien_quan: 0.9, ghim: false },
        MauNguCanh { nhan: "tai_lieu_B".into(), noi_dung: "...".into(), token: 200, lien_quan: 0.85, ghim: false },
        MauNguCanh { nhan: "tai_lieu_C".into(), noi_dung: "...".into(), token: 2000, lien_quan: 0.95, ghim: false },
        MauNguCanh { nhan: "lich_su_chat_cu".into(), noi_dung: "...".into(), token: 900, lien_quan: 0.1, ghim: false },
    ];
    let goi = dong_goi_ngu_canh(mau, 1000);
    println!("   Dùng {} / 1000 token, loại bỏ {} mẩu", goi.tong_token, goi.so_mau_bi_loai);
    for m in &goi.cac_mau {
        println!("     [{:>4} tok · lq {:.2}{}] {}", m.token, m.lien_quan,
                 if m.ghim { " · GHIM" } else { "" }, m.nhan);
    }
    println!("   → tai_lieu_C (2000 tok) bị loại dù liên quan cao nhất: KHÔNG VỪA ngân sách.");
    println!("   → Thứ tự đã đảo để mẩu quan trọng nằm ở ĐẦU và CUỐI (chống Lost-in-the-Middle).");

    // ---- 2 & 3. HARNESS + LOOP ----
    println!("\n2-3. BỘ KHUNG & VÒNG LẶP TÁC TỬ");
    let mut kho = HashMap::new();
    kho.insert("Rust".to_string(), "Ngôn ngữ hệ thống an toàn bộ nhớ".to_string());
    let khung = BoKhung::moi(5)
        .dang_ky(Box::new(CongCuTinhToan))
        .dang_ky(Box::new(CongCuTraCuu { kho }));
    println!("   Công cụ tác tử được phép dùng:\n{}", khung.mo_ta_cong_cu());

    let nao = BoNaoGia { kich_ban: vec![
        HanhDong::GoiCongCu { ten: "tra_cuu".into(), tham_so: "Rust".into() },
        HanhDong::GoiCongCu { ten: "tinh_tong".into(), tham_so: "10,20,12".into() },
        HanhDong::TraLoi("Rust là ngôn ngữ hệ thống; tổng là 42.".into()),
    ]};
    let kq = chay_vong_lap("Tra cứu Rust rồi cộng 10+20+12", &nao, &khung);
    for d in &kq.nhat_ky { println!("   {}", d); }
    println!("   Dừng vì: {:?} sau {} bước", kq.ly_do_dung, kq.so_buoc);

    // Vòng lặp hỏng: tác tử lặp mãi một lời gọi
    let nao_ket = BoNaoGia { kich_ban: vec![
        HanhDong::GoiCongCu { ten: "tra_cuu".into(), tham_so: "X".into() },
        HanhDong::GoiCongCu { ten: "tra_cuu".into(), tham_so: "X".into() },
    ]};
    let kq2 = chay_vong_lap("nhiệm vụ hỏng", &nao_ket, &khung);
    println!("   [Tác tử kẹt] dừng vì: {:?} sau {} bước", kq2.ly_do_dung, kq2.so_buoc);

    // ---- 4. GRAPH ENGINEERING ----
    println!("\n4. ĐỒ THỊ TRI THỨC — truy xuất lan tỏa 2 bước");
    let mut g = DoThiTriThuc::moi();
    g.them_thuc_the("DonHang", "Đơn hàng của khách");
    g.them_thuc_the("KhachHang", "Người mua");
    g.them_thuc_the("ThanhToan", "Giao dịch trừ tiền");
    g.them_thuc_the("VanDon", "Phiếu giao hàng");
    g.them_thuc_the("Kho", "Kho hàng vật lý");
    g.them_quan_he("DonHang", "thuoc_ve", "KhachHang");
    g.them_quan_he("DonHang", "duoc_tra_boi", "ThanhToan");
    g.them_quan_he("DonHang", "sinh_ra", "VanDon");
    g.them_quan_he("VanDon", "xuat_tu", "Kho");
    for dong in g.truy_xuat_lan_toa("DonHang", 2) {
        println!("   {}", dong);
    }
    println!("   → Tìm kiếm từ khóa thường sẽ BỎ SÓT \"Kho\" vì nó cách 2 bước.");

    println!("\n═══════════════════════════════════════════════════════════════");
    println!("  NGỮ CẢNH LÀ TÀI NGUYÊN · VÒNG LẶP PHẢI CÓ PHANH · CÔNG CỤ LÀ HỢP ĐỒNG ");
    println!("═══════════════════════════════════════════════════════════════");
}

#[cfg(test)]
mod kiem_thu {
    use super::*;

    fn mau(nhan: &str, token: usize, lq: f64, ghim: bool) -> MauNguCanh {
        MauNguCanh { nhan: nhan.into(), noi_dung: "x".into(), token, lien_quan: lq, ghim }
    }

    #[test]
    fn ngu_canh_khong_bao_gio_vuot_ngan_sach() {
        let ds = vec![mau("a", 400, 0.9, false), mau("b", 400, 0.8, false), mau("c", 400, 0.7, false)];
        let g = dong_goi_ngu_canh(ds, 1000);
        assert!(g.tong_token <= 1000, "vượt ngân sách: {}", g.tong_token);
        assert_eq!(g.cac_mau.len(), 2);
    }

    #[test]
    fn mau_ghim_luon_duoc_giu() {
        let ds = vec![
            mau("quy_tac", 100, 0.01, true),  // liên quan cực thấp nhưng GHIM
            mau("to", 900, 0.99, false),
        ];
        let g = dong_goi_ngu_canh(ds, 1000);
        assert!(g.cac_mau.iter().any(|m| m.nhan == "quy_tac"), "mẩu ghim bị loại!");
    }

    #[test]
    fn uu_tien_mat_do_gia_tri_chu_khong_phai_diem_tho() {
        // "nho" có điểm thấp hơn nhưng mật độ (lq/token) cao hơn nhiều
        let ds = vec![mau("to", 900, 0.9, false), mau("nho", 90, 0.5, false)];
        let g = dong_goi_ngu_canh(ds, 500);
        assert_eq!(g.cac_mau.len(), 1);
        assert_eq!(g.cac_mau[0].nhan, "nho");
    }

    #[test]
    fn chong_lang_quen_dat_quan_trong_o_hai_dau() {
        let ds = vec![mau("a", 1, 0.9, false), mau("b", 1, 0.5, false), mau("c", 1, 0.8, false)];
        let sx = sap_xep_chong_lang_quen(ds);
        // xếp giảm dần: a(.9) c(.8) b(.5) -> chẵn ra đầu, lẻ ra cuối (đảo): a, b, c
        assert_eq!(sx.first().unwrap().nhan, "a");
        assert_eq!(sx.last().unwrap().nhan, "c");
    }

    #[test]
    fn cong_cu_tra_loi_dung_va_bao_loi_ro_rang() {
        let cc = CongCuTinhToan;
        assert_eq!(cc.chay("1,2,3"), KetQuaCongCu::Xong("6".into()));
        assert!(matches!(cc.chay("1,x"), KetQuaCongCu::Loi(_)));
    }

    #[test]
    fn bo_khung_tu_choi_cong_cu_ngoai_danh_muc() {
        let khung = BoKhung::moi(3).dang_ky(Box::new(CongCuTinhToan));
        // Tác tử KHÔNG THỂ gọi thứ không được đăng ký — đây là ranh giới an toàn.
        assert!(matches!(khung.goi("xoa_o_cung", "/"), KetQuaCongCu::Loi(_)));
    }

    #[test]
    fn vong_lap_dung_khi_hoan_thanh() {
        let khung = BoKhung::moi(5).dang_ky(Box::new(CongCuTinhToan));
        let nao = BoNaoGia { kich_ban: vec![
            HanhDong::GoiCongCu { ten: "tinh_tong".into(), tham_so: "40,2".into() },
            HanhDong::TraLoi("42".into()),
        ]};
        let kq = chay_vong_lap("nv", &nao, &khung);
        assert_eq!(kq.ly_do_dung, LyDoDung::HoanThanh);
        assert_eq!(kq.tra_loi, Some("42".to_string()));
        assert_eq!(kq.so_buoc, 2);
    }

    #[test]
    fn vong_lap_dung_khi_het_luot_goi() {
        let khung = BoKhung::moi(3).dang_ky(Box::new(CongCuTinhToan));
        // Bộ não không bao giờ trả lời, chỉ gọi công cụ với tham số KHÁC nhau
        let nao = BoNaoGia { kich_ban: vec![
            HanhDong::GoiCongCu { ten: "tinh_tong".into(), tham_so: "1".into() },
            HanhDong::GoiCongCu { ten: "tinh_tong".into(), tham_so: "2".into() },
            HanhDong::GoiCongCu { ten: "tinh_tong".into(), tham_so: "3".into() },
            HanhDong::GoiCongCu { ten: "tinh_tong".into(), tham_so: "4".into() },
        ]};
        let kq = chay_vong_lap("nv", &nao, &khung);
        assert_eq!(kq.ly_do_dung, LyDoDung::HetLuotGoi);
        assert_eq!(kq.so_buoc, 3, "phải dừng đúng ở ngân sách 3 lượt");
    }

    #[test]
    fn vong_lap_phat_hien_tac_tu_bi_ket() {
        let khung = BoKhung::moi(50).dang_ky(Box::new(CongCuTinhToan));
        let nao = BoNaoGia { kich_ban: vec![
            HanhDong::GoiCongCu { ten: "tinh_tong".into(), tham_so: "1".into() },
            HanhDong::GoiCongCu { ten: "tinh_tong".into(), tham_so: "1".into() }, // y hệt
        ]};
        let kq = chay_vong_lap("nv", &nao, &khung);
        assert_eq!(kq.ly_do_dung, LyDoDung::LapVoHan);
        assert!(kq.so_buoc < 50, "phải dừng SỚM, không chạy hết 50 lượt");
    }

    #[test]
    fn do_thi_truy_xuat_dung_do_sau() {
        let mut g = DoThiTriThuc::moi();
        g.them_thuc_the("A", "a"); g.them_thuc_the("B", "b");
        g.them_thuc_the("C", "c"); g.them_thuc_the("D", "d");
        g.them_quan_he("A", "r1", "B");
        g.them_quan_he("B", "r2", "C");
        g.them_quan_he("C", "r3", "D");

        let sau1 = g.truy_xuat_lan_toa("A", 1);
        assert!(sau1.iter().any(|s| s.starts_with("B:")));
        assert!(!sau1.iter().any(|s| s.starts_with("C:")), "độ sâu 1 không được tới C");

        let sau2 = g.truy_xuat_lan_toa("A", 2);
        assert!(sau2.iter().any(|s| s.starts_with("C:")), "độ sâu 2 phải tới được C");
        assert!(!sau2.iter().any(|s| s.starts_with("D:")));
    }

    #[test]
    fn do_thi_khong_lap_vo_han_khi_co_chu_trinh() {
        let mut g = DoThiTriThuc::moi();
        g.them_thuc_the("A", "a"); g.them_thuc_the("B", "b");
        g.them_quan_he("A", "r", "B");
        g.them_quan_he("B", "r", "A"); // chu trình
        let kq = g.truy_xuat_lan_toa("A", 10);
        assert!(kq.len() < 10, "phải dừng nhờ tập đã thăm, không lặp vô hạn");
    }
}
