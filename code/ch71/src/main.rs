#![allow(dead_code)]
//! Chương 71 — Mạng ngang hàng & Đồng thuận: khoảng cách XOR và bảng định tuyến
//! Kademlia, tra cứu lặp, lan truyền gossip, và đồng thuận chịu lỗi Byzantine.
//!
//! Đây là lõi khái niệm của `rust-libp2p` — thư viện mạng của IPFS, Polkadot,
//! Ethereum (phần discovery) và Filecoin.

use std::collections::{BTreeMap, HashMap, HashSet, VecDeque};

// ============================================================================
// 1. ĐỊNH DANH NÚT & KHOẢNG CÁCH XOR
// ============================================================================

/// Trong mạng P2P không có máy chủ trung tâm, nên "ai giữ dữ liệu gì" phải
/// suy ra được từ chính định danh. Kademlia dùng phép XOR làm khoảng cách.
#[derive(Clone, Copy, PartialEq, Eq, Hash, PartialOrd, Ord, Debug)]
pub struct MaNut(pub u64);

impl MaNut {
    /// XOR là một METRIC thật sự: đối xứng, thoả bất đẳng thức tam giác, và
    /// d(x,x)=0. Nhờ đối xứng mà mỗi lần A tra cứu B, B cũng học được về A —
    /// bảng định tuyến tự bồi đắp từ chính lưu lượng bình thường.
    pub fn khoang_cach(self, khac: MaNut) -> u64 { self.0 ^ khac.0 }

    /// Chỉ số "xô" = vị trí bit khác nhau cao nhất. Nút càng gần thì xô càng nhỏ.
    pub fn chi_so_xo(self, khac: MaNut) -> Option<u32> {
        let d = self.khoang_cach(khac);
        if d == 0 { None } else { Some(63 - d.leading_zeros()) }
    }
}

// ============================================================================
// 2. BẢNG ĐỊNH TUYẾN KADEMLIA — biết "log n" nút là đủ tìm ra cả mạng
// ============================================================================

pub const K: usize = 4; // số nút giữ trong mỗi xô (Kademlia thật dùng 20)

/// 64 xô, xô thứ `i` giữ các nút cách ta khoảng 2^i tới 2^(i+1).
/// Ta biết RẤT NHIỀU nút ở gần và RẤT ÍT nút ở xa — nhưng vẫn đủ để tới
/// bất kỳ đâu trong log₂(n) bước. Đây là "thế giới nhỏ" có cấu trúc.
pub struct BangDinhTuyen {
    pub toi: MaNut,
    pub xo: Vec<VecDeque<MaNut>>,
}

impl BangDinhTuyen {
    pub fn moi(toi: MaNut) -> Self {
        BangDinhTuyen { toi, xo: (0..64).map(|_| VecDeque::new()).collect() }
    }

    /// Trả `true` nếu nút được thêm mới. Nút đã biết được đẩy lên cuối hàng —
    /// Kademlia ưu tiên giữ nút CŨ, vì nút sống lâu có xác suất sống tiếp cao hơn.
    /// Đây cũng là biện pháp chống tấn công Sybil: kẻ tấn công không thể tràn
    /// bảng định tuyến bằng cách bơm nút mới.
    pub fn them(&mut self, nut: MaNut) -> bool {
        let i = match self.toi.chi_so_xo(nut) { Some(i) => i as usize, None => return false };
        if let Some(vt) = self.xo[i].iter().position(|&n| n == nut) {
            let n = self.xo[i].remove(vt).unwrap();
            self.xo[i].push_back(n);
            return false;
        }
        if self.xo[i].len() < K {
            self.xo[i].push_back(nut);
            true
        } else {
            false // xô đầy: giữ nút cũ, bỏ nút mới
        }
    }

    pub fn tong_so_nut(&self) -> usize { self.xo.iter().map(|x| x.len()).sum() }

    /// `so_luong` nút gần `dich` nhất mà ta biết.
    pub fn gan_nhat(&self, dich: MaNut, so_luong: usize) -> Vec<MaNut> {
        let mut v: Vec<MaNut> = self.xo.iter().flatten().copied().collect();
        v.sort_by_key(|n| n.khoang_cach(dich));
        v.truncate(so_luong);
        v
    }
}

// ============================================================================
// 3. TRA CỨU LẶP — tìm nút gần đích nhất trong O(log n) vòng
// ============================================================================

#[derive(Debug, PartialEq)]
pub struct KetQuaTraCuu {
    pub gan_nhat: Vec<MaNut>,
    pub so_vong: usize,
    pub so_nut_da_hoi: usize,
}

/// Mạng mô phỏng: mỗi nút có bảng định tuyến riêng.
pub struct MangMoPhong { pub nut: BTreeMap<MaNut, BangDinhTuyen> }

impl MangMoPhong {
    /// Dựng mạng và cho các nút "gặp nhau" theo kiểu bootstrap thật:
    /// mỗi nút mới tự tra cứu chính mình qua một nút đã có sẵn.
    pub fn dung(cac_ma: &[u64]) -> MangMoPhong {
        let mut m = MangMoPhong { nut: BTreeMap::new() };
        for &x in cac_ma {
            let ma = MaNut(x);
            m.nut.insert(ma, BangDinhTuyen::moi(ma));
        }
        // Vài vòng trao đổi để bảng định tuyến hội tụ
        let tat_ca: Vec<MaNut> = m.nut.keys().copied().collect();
        for _ in 0..3 {
            for &a in &tat_ca {
                for &b in &tat_ca {
                    if a != b { m.nut.get_mut(&a).unwrap().them(b); }
                }
            }
        }
        m
    }

    /// Tra cứu lặp: hỏi α nút gần nhất đã biết, chúng trả về nút chúng biết,
    /// lặp lại cho tới khi không tiến gần hơn được nữa.
    pub fn tra_cuu(&self, tu: MaNut, dich: MaNut, alpha: usize) -> KetQuaTraCuu {
        let mut ung_vien: Vec<MaNut> = self.nut[&tu].gan_nhat(dich, K);
        let mut da_hoi: HashSet<MaNut> = HashSet::new();
        let mut so_vong = 0;

        loop {
            let hoi: Vec<MaNut> = ung_vien.iter().copied()
                .filter(|n| !da_hoi.contains(n)).take(alpha).collect();
            if hoi.is_empty() { break; }
            so_vong += 1;
            let mut moi = Vec::new();
            for n in hoi {
                da_hoi.insert(n);
                if let Some(b) = self.nut.get(&n) { moi.extend(b.gan_nhat(dich, K)); }
            }
            let truoc = ung_vien.first().map(|n| n.khoang_cach(dich));
            ung_vien.extend(moi);
            ung_vien.sort_by_key(|n| n.khoang_cach(dich));
            ung_vien.dedup();
            ung_vien.truncate(K);
            // Không tiến gần hơn → dừng. Đây là điều kiện hội tụ của Kademlia.
            if ung_vien.first().map(|n| n.khoang_cach(dich)) == truoc && so_vong > 1 { break; }
            if so_vong > 64 { break; } // chặn an toàn
        }
        KetQuaTraCuu { gan_nhat: ung_vien, so_vong, so_nut_da_hoi: da_hoi.len() }
    }
}

// ============================================================================
// 4. GOSSIP — lan truyền kiểu dịch bệnh
// ============================================================================

#[derive(Debug, PartialEq)]
pub struct KetQuaLanTruyen {
    pub so_vong: usize,
    pub so_nut_nhan: usize,
    /// Tổng số bản tin đã gửi — thước đo chi phí băng thông.
    pub so_ban_tin: usize,
    pub phu_song_hoan_toan: bool,
}

/// Mỗi nút chuyển tiếp bản tin cho `bac` hàng xóm, nhưng CHỈ LẦN ĐẦU thấy nó.
/// Không có bộ nhớ chống trùng thì mạng sẽ bão bản tin và tự sập.
pub fn lan_truyen_gossip(
    lang_gieng: &HashMap<MaNut, Vec<MaNut>>,
    nguon: MaNut,
    bac: usize,
    so_vong_toi_da: usize,
) -> KetQuaLanTruyen {
    let mut da_thay: HashSet<MaNut> = HashSet::new();
    da_thay.insert(nguon);
    let mut dang_lan = vec![nguon];
    let mut so_ban_tin = 0;
    let mut so_vong = 0;

    while !dang_lan.is_empty() && so_vong < so_vong_toi_da {
        so_vong += 1;
        let mut ke_tiep = Vec::new();
        for n in &dang_lan {
            let lg = match lang_gieng.get(n) { Some(l) => l, None => continue };
            // Chọn `bac` hàng xóm một cách TẤT ĐỊNH (thật thì chọn ngẫu nhiên)
            for &m in lg.iter().take(bac) {
                so_ban_tin += 1;
                if da_thay.insert(m) { ke_tiep.push(m); }
            }
        }
        dang_lan = ke_tiep;
    }
    KetQuaLanTruyen {
        so_vong,
        so_nut_nhan: da_thay.len(),
        so_ban_tin,
        phu_song_hoan_toan: da_thay.len() == lang_gieng.len(),
    }
}

// ============================================================================
// 5. ĐỒNG THUẬN CHỊU LỖI BYZANTINE
// ============================================================================

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum HanhVi { TrungThuc, Im, HaiMat }

#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
pub enum LaPhieu { Thuan(u32), Chong }

/// Vì sao cần 3f+1 nút để chịu được f nút phản bội?
///
/// Ta phải quyết định dù f nút không trả lời, nên chỉ chờ được n−f phiếu.
/// Trong n−f phiếu đó có thể có tới f phiếu gian, còn lại n−2f là thật.
/// Muốn phe thật áp đảo phe gian: n−2f > f  ⟺  n > 3f.
/// Vậy n = 3f+1 là con số NHỎ NHẤT dùng được — không phải quy ước tuỳ tiện.
pub fn so_loi_chiu_duoc(n: usize) -> usize { (n - 1) / 3 }

/// ⚠️ CẨN THẬN VỚI CÔNG THỨC "2f+1" ĐƯỢC TRÍCH DẪN KHẮP NƠI.
///
/// Nó chỉ đúng khi n ĐÚNG BẰNG 3f+1. Với n bất kỳ, quy tắc tổng quát là:
///
///   an toàn : hai quorum bất kỳ phải giao nhau ở nhiều hơn f nút
///             ⟹ 2q − n > f  ⟺  q > (n+f)/2
///   sống còn: phải gom đủ phiếu dù f nút im lặng  ⟹  q ≤ n − f
///
/// Ví dụ n = 5, f = 1: công thức "2f+1" cho q = 3. Nhưng hai quorum 3 trên 5
/// chỉ giao nhau ĐÚNG MỘT nút — và nút đó có thể chính là kẻ phản bội. Khi ấy
/// hai nhóm chốt hai giá trị khác nhau: chuỗi rẽ đôi. Đáp số đúng là q = 4.
pub fn nguong_quorum(n: usize) -> usize {
    let f = so_loi_chiu_duoc(n);
    (n + f) / 2 + 1
}

#[derive(Debug, PartialEq)]
pub struct KetQuaVong {
    pub quyet_dinh: Option<u32>,
    pub so_phieu_thu_duoc: usize,
    pub nguong_can: usize,
}

/// Một vòng đồng thuận kiểu Tendermint/PBFT rút gọn: nút đề xuất phát giá trị,
/// các nút bỏ phiếu, đạt quorum thì chốt.
pub fn vong_dong_thuan(hanh_vi: &[HanhVi], gia_tri_de_xuat: u32) -> KetQuaVong {
    let n = hanh_vi.len();
    let nguong = nguong_quorum(n);
    let mut thung: HashMap<LaPhieu, usize> = HashMap::new();

    for (i, &h) in hanh_vi.iter().enumerate() {
        match h {
            HanhVi::TrungThuc => *thung.entry(LaPhieu::Thuan(gia_tri_de_xuat)).or_insert(0) += 1,
            HanhVi::Im => {}  // không gửi gì — lỗi "dừng", dạng nhẹ nhất
            HanhVi::HaiMat => {
                // Nút phản bội gửi giá trị KHÁC NHAU cho các nhóm khác nhau.
                // Đây là lỗi Byzantine thực thụ, khó hơn hẳn lỗi "im lặng".
                *thung.entry(LaPhieu::Thuan(gia_tri_de_xuat.wrapping_add(i as u32 + 1)))
                    .or_insert(0) += 1;
            }
        }
    }
    let tot_nhat = thung.iter().max_by_key(|(_, &c)| c);
    let (quyet_dinh, so_phieu) = match tot_nhat {
        Some((LaPhieu::Thuan(v), &c)) if c >= nguong => (Some(*v), c),
        Some((_, &c)) => (None, c),
        None => (None, 0),
    };
    KetQuaVong { quyet_dinh, so_phieu_thu_duoc: so_phieu, nguong_can: nguong }
}

// ============================================================================
// 6. BẢNG BĂM PHÂN TÁN — lưu và tìm dữ liệu không cần máy chủ
// ============================================================================

pub struct BangBamPhanTan {
    pub mang: MangMoPhong,
    /// Mỗi nút giữ một phần kho. Dữ liệu nằm ở `r` nút gần khoá nhất.
    pub kho: HashMap<MaNut, HashMap<u64, String>>,
    pub he_so_nhan_ban: usize,
}

impl BangBamPhanTan {
    pub fn moi(cac_ma: &[u64], he_so_nhan_ban: usize) -> Self {
        let mang = MangMoPhong::dung(cac_ma);
        let kho = cac_ma.iter().map(|&x| (MaNut(x), HashMap::new())).collect();
        BangBamPhanTan { mang, kho, he_so_nhan_ban }
    }

    /// Ghi vào `r` nút gần khoá nhất. Nhân bản là cách DHT chịu được việc
    /// nút rời mạng bất cứ lúc nào — điều xảy ra liên tục trong mạng thật.
    pub fn dat(&mut self, tu: MaNut, khoa: u64, gia_tri: &str) -> usize {
        let kq = self.mang.tra_cuu(tu, MaNut(khoa), 3);
        let mut dich: Vec<MaNut> = kq.gan_nhat;
        dich.truncate(self.he_so_nhan_ban);
        for n in &dich {
            self.kho.get_mut(n).unwrap().insert(khoa, gia_tri.to_string());
        }
        dich.len()
    }

    pub fn lay(&self, tu: MaNut, khoa: u64) -> Option<String> {
        let kq = self.mang.tra_cuu(tu, MaNut(khoa), 3);
        for n in kq.gan_nhat {
            if let Some(v) = self.kho.get(&n).and_then(|k| k.get(&khoa)) {
                return Some(v.clone());
            }
        }
        None
    }

    /// Mô phỏng nút rời mạng — xoá cả dữ liệu nó giữ.
    pub fn nut_roi_mang(&mut self, nut: MaNut) {
        self.kho.remove(&nut);
        self.mang.nut.remove(&nut);
        for (_, b) in self.mang.nut.iter_mut() {
            for x in b.xo.iter_mut() { x.retain(|&n| n != nut); }
        }
    }
}

fn main() {
    println!("═══════════════════════════════════════════════════════════");
    println!("   MẠNG NGANG HÀNG: KADEMLIA · GOSSIP · ĐỒNG THUẬN BFT     ");
    println!("═══════════════════════════════════════════════════════════");

    println!("\n1. KHOẢNG CÁCH XOR LÀ MỘT METRIC THẬT");
    let (a, b, c) = (MaNut(0b1010), MaNut(0b1100), MaNut(0b0001));
    println!("   d(a,b) = {} · d(b,a) = {} → đối xứng", a.khoang_cach(b), b.khoang_cach(a));
    println!("   d(a,c) = {} ≤ d(a,b) + d(b,c) = {} → bất đẳng thức tam giác",
             a.khoang_cach(c), a.khoang_cach(b) + b.khoang_cach(c));
    println!("   d(a,a) = {}", a.khoang_cach(a));

    println!("\n2. BẢNG ĐỊNH TUYẾN — biết ít mà tới được mọi nơi");
    let ma: Vec<u64> = (0..64u64).map(|i| i.wrapping_mul(0x9E3779B97F4A7C15)).collect();
    let mang = MangMoPhong::dung(&ma);
    let toi = MaNut(ma[0]);
    let b0 = &mang.nut[&toi];
    println!("   Mạng {} nút · nút này chỉ lưu {} địa chỉ ({} xô không rỗng)",
             ma.len(), b0.tong_so_nut(), b0.xo.iter().filter(|x| !x.is_empty()).count());

    println!("\n3. TRA CỨU LẶP");
    let dich = MaNut(ma[50]);
    let kq = mang.tra_cuu(toi, dich, 3);
    println!("   Tìm {:x} → {} vòng, hỏi {} nút", dich.0, kq.so_vong, kq.so_nut_da_hoi);
    println!("   Tìm thấy đúng đích: {}", kq.gan_nhat.contains(&dich));

    println!("\n4. GOSSIP — đánh đổi tốc độ lấy băng thông");
    let mut lg: HashMap<MaNut, Vec<MaNut>> = HashMap::new();
    for (i, &x) in ma.iter().enumerate() {
        // vòng tròn + vài dây cung → đồ thị "thế giới nhỏ"
        let l: Vec<MaNut> = [1, 2, 7, 19, 31].iter()
            .map(|d| MaNut(ma[(i + d) % ma.len()])).collect();
        lg.insert(MaNut(x), l);
    }
    for bac in [1usize, 2, 3, 5] {
        let r = lan_truyen_gossip(&lg, toi, bac, 50);
        println!("   bậc {} → {:>2} vòng · phủ {:>2}/{} nút · {:>3} bản tin",
                 bac, r.so_vong, r.so_nut_nhan, ma.len(), r.so_ban_tin);
    }
    println!("   → Bậc cao phủ nhanh hơn nhưng tốn băng thông theo cấp số nhân.");

    println!("\n5. ĐỒNG THUẬN BYZANTINE — vì sao là 3f+1");
    for n in [4usize, 7, 10, 13, 100] {
        println!("   {:>3} nút → chịu được {:>2} nút phản bội · cần {:>2} phiếu",
                 n, so_loi_chiu_duoc(n), nguong_quorum(n));
    }
    let hv = vec![HanhVi::TrungThuc; 10];
    println!("\n   10 nút, tăng dần số kẻ phản bội:");
    for so_gian in 0..5 {
        let mut h = hv.clone();
        for i in 0..so_gian { h[i] = HanhVi::HaiMat; }
        let r = vong_dong_thuan(&h, 42);
        println!("   {} kẻ gian → {:?} ({}/{} phiếu){}",
                 so_gian, r.quyet_dinh, r.so_phieu_thu_duoc, r.nguong_can,
                 if so_gian > so_loi_chiu_duoc(10) { "  ← vượt ngưỡng an toàn" } else { "" });
    }

    println!("\n6. BẢNG BĂM PHÂN TÁN — chịu được nút rời mạng");
    let mut dht = BangBamPhanTan::moi(&ma, 3);
    let n = dht.dat(toi, 0xDEADBEEF, "xin chao P2P");
    println!("   Ghi khoá 0xDEADBEEF vào {} nút gần nhất", n);
    println!("   Đọc lại: {:?}", dht.lay(MaNut(ma[30]), 0xDEADBEEF));
    let giu: Vec<MaNut> = dht.kho.iter()
        .filter(|(_, k)| k.contains_key(&0xDEADBEEF)).map(|(n, _)| *n).collect();
    dht.nut_roi_mang(giu[0]);
    println!("   Sau khi 1 nút giữ dữ liệu rời mạng: {:?}", dht.lay(MaNut(ma[30]), 0xDEADBEEF));

    println!("\n═══════════════════════════════════════════════════════════");
    println!("   KHÔNG MÁY CHỦ, KHÔNG TIN NHAU, VẪN THỐNG NHẤT ĐƯỢC       ");
    println!("═══════════════════════════════════════════════════════════");
}

#[cfg(test)]
mod kiem_thu {
    use super::*;

    fn ma_mau(n: usize) -> Vec<u64> {
        (0..n as u64).map(|i| i.wrapping_mul(0x9E3779B97F4A7C15)).collect()
    }

    // ---------- Khoảng cách XOR ----------
    #[test]
    fn xor_thoa_ba_tinh_chat_cua_metric() {
        let ma = ma_mau(24);
        for &x in &ma {
            let a = MaNut(x);
            assert_eq!(a.khoang_cach(a), 0, "d(x,x) = 0");
            for &y in &ma {
                let b = MaNut(y);
                assert_eq!(a.khoang_cach(b), b.khoang_cach(a), "đối xứng");
                for &z in &ma {
                    let c = MaNut(z);
                    // Cộng trong u128: với hai giá trị 64-bit, tổng của chúng
                    // TRÀN u64. Đây là cái bẫy thật khi kiểm chứng metric XOR.
                    assert!(a.khoang_cach(c) as u128
                            <= a.khoang_cach(b) as u128 + b.khoang_cach(c) as u128,
                            "bất đẳng thức tam giác");
                }
            }
        }
    }

    #[test]
    fn xor_thoa_dang_thuc_tam_giac_chu_khong_chi_bat_dang_thuc() {
        // Tính chất ĐẶC BIỆT của metric XOR, mạnh hơn hẳn bất đẳng thức thường:
        //   d(a,c) = d(a,b) ⊕ d(b,c)   — ĐẲNG THỨC, không phải "≤"
        // vì (a⊕b) ⊕ (b⊕c) = a⊕c. Nhờ nó, khoảng cách tính được theo từng chặng
        // mà không tích luỹ sai số, và không bao giờ tràn số.
        let ma = ma_mau(20);
        for &x in &ma { for &y in &ma { for &z in &ma {
            let (a, b, c) = (MaNut(x), MaNut(y), MaNut(z));
            assert_eq!(a.khoang_cach(c), a.khoang_cach(b) ^ b.khoang_cach(c));
        }}}
    }

    #[test]
    fn xor_duy_nhat_khoang_cach_bang_khong_khi_trung_nhau() {
        let a = MaNut(12345);
        assert_eq!(a.khoang_cach(a), 0);
        assert_ne!(a.khoang_cach(MaNut(12346)), 0);
        assert_eq!(a.chi_so_xo(a), None, "khoảng cách 0 không thuộc xô nào");
    }

    #[test]
    fn chi_so_xo_khop_bit_khac_cao_nhat() {
        let a = MaNut(0b0000);
        assert_eq!(a.chi_so_xo(MaNut(0b0001)), Some(0));
        assert_eq!(a.chi_so_xo(MaNut(0b0010)), Some(1));
        assert_eq!(a.chi_so_xo(MaNut(0b1000)), Some(3));
        assert_eq!(a.chi_so_xo(MaNut(0b1001)), Some(3), "lấy bit CAO nhất khác nhau");
    }

    // ---------- Bảng định tuyến ----------
    #[test]
    fn xo_khong_bao_gio_vuot_qua_k() {
        let mut b = BangDinhTuyen::moi(MaNut(0));
        for i in 1..500u64 { b.them(MaNut(i)); }
        for (i, x) in b.xo.iter().enumerate() {
            assert!(x.len() <= K, "xô {} có {} nút, vượt K={}", i, x.len(), K);
        }
    }

    #[test]
    fn bang_dinh_tuyen_giu_nut_cu_khi_xo_day() {
        // Chống Sybil: kẻ tấn công bơm nút mới KHÔNG đẩy được nút cũ ra.
        let mut b = BangDinhTuyen::moi(MaNut(0));
        // các nút 8..11 đều thuộc xô 3
        for i in 8..8 + K as u64 { assert!(b.them(MaNut(i))); }
        assert_eq!(b.xo[3].len(), K);
        let cu: Vec<MaNut> = b.xo[3].iter().copied().collect();
        assert!(!b.them(MaNut(15)), "xô đầy → từ chối nút mới");
        assert_eq!(b.xo[3].iter().copied().collect::<Vec<_>>(), cu, "nút cũ nguyên vẹn");
    }

    #[test]
    fn gap_lai_nut_cu_day_no_len_cuoi_hang() {
        let mut b = BangDinhTuyen::moi(MaNut(0));
        for i in 8..12u64 { b.them(MaNut(i)); }
        assert_eq!(*b.xo[3].front().unwrap(), MaNut(8));
        assert!(!b.them(MaNut(8)), "gặp lại không tính là thêm mới");
        assert_eq!(*b.xo[3].back().unwrap(), MaNut(8), "nút vừa liên lạc lên cuối hàng");
    }

    #[test]
    fn khong_tu_them_chinh_minh() {
        let mut b = BangDinhTuyen::moi(MaNut(42));
        assert!(!b.them(MaNut(42)));
        assert_eq!(b.tong_so_nut(), 0);
    }

    #[test]
    fn gan_nhat_sap_dung_theo_khoang_cach() {
        let mut b = BangDinhTuyen::moi(MaNut(0));
        for i in 1..100u64 { b.them(MaNut(i)); }
        let dich = MaNut(50);
        let g = b.gan_nhat(dich, 5);
        for w in g.windows(2) {
            assert!(w[0].khoang_cach(dich) <= w[1].khoang_cach(dich));
        }
    }

    #[test]
    fn bang_dinh_tuyen_nho_hon_nhieu_so_voi_ca_mang() {
        let ma = ma_mau(256);
        let m = MangMoPhong::dung(&ma);
        let b = &m.nut[&MaNut(ma[0])];
        assert!(b.tong_so_nut() < ma.len(),
                "biết {} trong tổng {} nút — đó là ý nghĩa của định tuyến log n",
                b.tong_so_nut(), ma.len());
    }

    // ---------- Tra cứu ----------
    #[test]
    fn tra_cuu_tim_duoc_nut_dich() {
        let ma = ma_mau(128);
        let m = MangMoPhong::dung(&ma);
        let tu = MaNut(ma[0]);
        for &x in ma.iter().skip(1).take(20) {
            let kq = m.tra_cuu(tu, MaNut(x), 3);
            assert!(kq.gan_nhat.contains(&MaNut(x)), "không tìm được nút {:x}", x);
        }
    }

    #[test]
    fn tra_cuu_hoi_it_hon_nhieu_so_voi_ca_mang() {
        let ma = ma_mau(256);
        let m = MangMoPhong::dung(&ma);
        let kq = m.tra_cuu(MaNut(ma[0]), MaNut(ma[200]), 3);
        assert!(kq.so_nut_da_hoi < ma.len() / 2,
                "hỏi {} nút trên tổng {} — tra cứu phải RẺ", kq.so_nut_da_hoi, ma.len());
        assert!(kq.so_vong <= 64, "phải hội tụ, không lặp vô hạn");
    }

    #[test]
    fn tra_cuu_luon_dung_ke_ca_khoa_khong_ung_voi_nut_nao() {
        let ma = ma_mau(64);
        let m = MangMoPhong::dung(&ma);
        let khoa = MaNut(0x1234_5678_9ABC_DEF0);
        let kq = m.tra_cuu(MaNut(ma[0]), khoa, 3);
        assert!(!kq.gan_nhat.is_empty(), "vẫn phải trả về nút gần nhất");
        // kết quả phải thật sự là gần nhất trong toàn mạng
        let that_su_gan_nhat = ma.iter().map(|&x| MaNut(x))
            .min_by_key(|n| n.khoang_cach(khoa)).unwrap();
        assert!(kq.gan_nhat.contains(&that_su_gan_nhat),
                "tra cứu phải hội tụ về nút gần nhất thật sự");
    }

    // ---------- Gossip ----------
    #[test]
    fn gossip_phu_song_toan_mang_neu_do_thi_lien_thong() {
        let ma = ma_mau(50);
        let mut lg = HashMap::new();
        for (i, &x) in ma.iter().enumerate() {
            lg.insert(MaNut(x), vec![MaNut(ma[(i + 1) % ma.len()])]); // vòng tròn
        }
        let r = lan_truyen_gossip(&lg, MaNut(ma[0]), 1, 100);
        assert!(r.phu_song_hoan_toan);
        assert_eq!(r.so_nut_nhan, 50);
    }

    #[test]
    fn bac_cao_hon_phu_song_nhanh_hon() {
        let ma = ma_mau(64);
        let mut lg = HashMap::new();
        for (i, &x) in ma.iter().enumerate() {
            lg.insert(MaNut(x), [1, 2, 7, 19, 31].iter()
                .map(|d| MaNut(ma[(i + d) % ma.len()])).collect());
        }
        let it = lan_truyen_gossip(&lg, MaNut(ma[0]), 1, 100);
        let nhieu = lan_truyen_gossip(&lg, MaNut(ma[0]), 4, 100);
        assert!(nhieu.so_vong < it.so_vong, "bậc cao phải phủ nhanh hơn");
        assert!(nhieu.so_ban_tin > it.so_ban_tin, "và tốn nhiều băng thông hơn");
    }

    #[test]
    fn gossip_khong_bao_ban_tin_nho_chong_trung() {
        // Không có `da_thay` thì mỗi nút chuyển tiếp mãi mãi và mạng sập.
        let ma = ma_mau(30);
        let mut lg = HashMap::new();
        for (i, &x) in ma.iter().enumerate() {
            lg.insert(MaNut(x), (1..=5).map(|d| MaNut(ma[(i + d) % ma.len()])).collect());
        }
        let r = lan_truyen_gossip(&lg, MaNut(ma[0]), 5, 100);
        assert!(r.so_ban_tin <= ma.len() * 5,
                "mỗi nút chỉ được chuyển tiếp MỘT lần: {} bản tin", r.so_ban_tin);
    }

    #[test]
    fn gossip_khong_toi_duoc_phan_mang_bi_co_lap() {
        let ma = ma_mau(20);
        let mut lg = HashMap::new();
        // hai cụm rời nhau hoàn toàn
        for i in 0..10 { lg.insert(MaNut(ma[i]), vec![MaNut(ma[(i + 1) % 10])]); }
        for i in 10..20 { lg.insert(MaNut(ma[i]), vec![MaNut(ma[10 + (i + 1) % 10])]); }
        let r = lan_truyen_gossip(&lg, MaNut(ma[0]), 1, 100);
        assert_eq!(r.so_nut_nhan, 10, "chỉ phủ được cụm của mình");
        assert!(!r.phu_song_hoan_toan, "phân mảnh mạng là rủi ro có thật");
    }

    // ---------- Đồng thuận ----------
    #[test]
    fn quorum_thoa_ca_an_toan_lan_song_con_voi_moi_n() {
        for n in 4..500usize {
            let f = so_loi_chiu_duoc(n);
            let q = nguong_quorum(n);
            assert!(3 * f + 1 <= n, "n={} phải chứa nổi 3f+1 với f={}", n, f);

            // AN TOÀN: hai quorum giao nhau ở nhiều hơn f nút, nên luôn có ít
            // nhất một nút TRUNG THỰC nằm trong cả hai → không thể chốt hai
            // giá trị mâu thuẫn.
            let giao = 2 * q as i64 - n as i64;
            assert!(giao > f as i64,
                    "n={}: hai quorum giao {} nút, phải nhiều hơn f={}", n, giao, f);

            // SỐNG CÒN: gom đủ q phiếu ngay cả khi f nút im lặng hoàn toàn.
            assert!(q <= n - f, "n={}: cần {} phiếu nhưng chỉ chắc chắn có {}", n, q, n - f);
        }
    }

    #[test]
    fn cong_thuc_2f_cong_1_chi_dung_khi_n_bang_3f_cong_1() {
        // Trường hợp "đẹp": n = 3f+1 → công thức phổ biến 2f+1 đúng
        for f in 1..50usize {
            let n = 3 * f + 1;
            assert_eq!(nguong_quorum(n), 2 * f + 1, "n=3f+1 thì phải khớp 2f+1");
        }
        // Trường hợp "xấu": n = 5, f = 1 → 2f+1 = 3 là KHÔNG AN TOÀN
        assert_eq!(so_loi_chiu_duoc(5), 1);
        assert_eq!(nguong_quorum(5), 4, "phải là 4, không phải 3");
        assert!(2 * 3 - 5 <= 1, "quorum 3 chỉ giao 1 nút — có thể chính là kẻ gian");
        assert!(2 * 4 - 5 > 1, "quorum 4 giao 3 nút — chắc chắn có nút trung thực");
    }

    #[test]
    fn dong_thuan_thanh_cong_khi_du_nut_trung_thuc() {
        let n = 10;
        let f = so_loi_chiu_duoc(n); // 3
        for so_gian in 0..=f {
            let mut h = vec![HanhVi::TrungThuc; n];
            for i in 0..so_gian { h[i] = HanhVi::HaiMat; }
            let r = vong_dong_thuan(&h, 42);
            assert_eq!(r.quyet_dinh, Some(42),
                       "{} kẻ gian (<= f={}) vẫn phải chốt được", so_gian, f);
        }
    }

    #[test]
    fn dong_thuan_that_bai_khi_vuot_nguong() {
        let n = 10;
        let f = so_loi_chiu_duoc(n);
        let mut h = vec![HanhVi::TrungThuc; n];
        for i in 0..=f + 1 { h[i] = HanhVi::HaiMat; }
        let r = vong_dong_thuan(&h, 42);
        assert_eq!(r.quyet_dinh, None, "quá f kẻ gian → THÀ DỪNG còn hơn chốt sai");
    }

    #[test]
    fn nut_im_lang_de_chiu_hon_nut_hai_mat() {
        // Lỗi "dừng" nhẹ hơn lỗi Byzantine: nút im chỉ không đóng góp,
        // còn nút hai mặt vừa không đóng góp vừa gây nhiễu phiếu.
        let n = 10;
        let mut im = vec![HanhVi::TrungThuc; n];
        let mut gian = vec![HanhVi::TrungThuc; n];
        for i in 0..3 { im[i] = HanhVi::Im; gian[i] = HanhVi::HaiMat; }
        assert_eq!(vong_dong_thuan(&im, 42).quyet_dinh, Some(42));
        assert_eq!(vong_dong_thuan(&gian, 42).quyet_dinh, Some(42));
        // Cùng 7 phiếu thật; khác nhau ở chỗ nút hai mặt còn tạo thêm phiếu rác
        assert_eq!(vong_dong_thuan(&im, 42).so_phieu_thu_duoc, 7);
        assert_eq!(vong_dong_thuan(&gian, 42).so_phieu_thu_duoc, 7);
    }

    #[test]
    fn mang_bon_nut_chiu_duoc_dung_mot_ke_phan_boi() {
        assert_eq!(so_loi_chiu_duoc(4), 1);
        assert_eq!(nguong_quorum(4), 3);
        let r = vong_dong_thuan(&[HanhVi::TrungThuc, HanhVi::TrungThuc,
                                  HanhVi::TrungThuc, HanhVi::HaiMat], 7);
        assert_eq!(r.quyet_dinh, Some(7));
        let r2 = vong_dong_thuan(&[HanhVi::TrungThuc, HanhVi::TrungThuc,
                                   HanhVi::HaiMat, HanhVi::HaiMat], 7);
        assert_eq!(r2.quyet_dinh, None, "2 kẻ gian trên 4 nút là quá ngưỡng");
    }

    // ---------- DHT ----------
    #[test]
    fn dht_ghi_roi_doc_lai_duoc_tu_nut_bat_ky() {
        let ma = ma_mau(64);
        let mut d = BangBamPhanTan::moi(&ma, 3);
        d.dat(MaNut(ma[0]), 999, "gia tri");
        for &x in ma.iter().take(10) {
            assert_eq!(d.lay(MaNut(x), 999), Some("gia tri".to_string()),
                       "mọi nút đều phải tìm ra dữ liệu");
        }
    }

    #[test]
    fn dht_nhan_ban_dung_so_luong() {
        let ma = ma_mau(64);
        let mut d = BangBamPhanTan::moi(&ma, 3);
        assert_eq!(d.dat(MaNut(ma[0]), 555, "x"), 3);
        let giu = d.kho.values().filter(|k| k.contains_key(&555)).count();
        assert_eq!(giu, 3);
    }

    #[test]
    fn dht_song_sot_khi_mot_ban_sao_roi_mang() {
        let ma = ma_mau(64);
        let mut d = BangBamPhanTan::moi(&ma, 3);
        d.dat(MaNut(ma[0]), 777, "ben bi");
        let giu: Vec<MaNut> = d.kho.iter()
            .filter(|(_, k)| k.contains_key(&777)).map(|(n, _)| *n).collect();
        d.nut_roi_mang(giu[0]);
        assert_eq!(d.lay(MaNut(ma[40]), 777), Some("ben bi".to_string()),
                   "nhân bản 3 lần thì mất 1 vẫn đọc được");
    }

    #[test]
    fn dht_tra_none_cho_khoa_chua_tung_ghi() {
        let ma = ma_mau(32);
        let d = BangBamPhanTan::moi(&ma, 3);
        assert_eq!(d.lay(MaNut(ma[0]), 12345), None);
    }
}
