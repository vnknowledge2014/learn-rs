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
pub struct NodeId(pub u64);

impl NodeId {
    /// XOR là một METRIC thật sự: đối xứng, thoả bất đẳng thức tam giác, và
    /// d(x,x)=0. Nhờ đối xứng mà mỗi lần A tra cứu B, B cũng học được về A —
    /// bảng định tuyến tự bồi đắp từ chính lưu lượng bình thường.
    pub fn distance(self, other: NodeId) -> u64 { self.0 ^ other.0 }

    /// Chỉ số "xô" = vị trí bit khác nhau cao nhất. Nút càng gần thì xô càng nhỏ.
    pub fn only_num_xor(self, other: NodeId) -> Option<u32> {
        let d = self.distance(other);
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
pub struct RoutingTable {
    pub toi: NodeId,
    pub xor: Vec<VecDeque<NodeId>>,
}

impl RoutingTable {
    pub fn new(toi: NodeId) -> Self {
        RoutingTable { toi, xor: (0..64).map(|_| VecDeque::new()).collect() }
    }

    /// Trả `true` nếu nút được thêm mới. Nút đã biết được đẩy lên cuối hàng —
    /// Kademlia ưu tiên giữ nút CŨ, vì nút sống lâu có xác suất sống tiếp cao hơn.
    /// Đây cũng là biện pháp chống tấn công Sybil: kẻ tấn công không thể tràn
    /// bảng định tuyến bằng cách bơm nút mới.
    pub fn them(&mut self, nut: NodeId) -> bool {
        let i = match self.toi.only_num_xor(nut) { Some(i) => i as usize, None => return false };
        if let Some(vt) = self.xor[i].iter().position(|&n| n == nut) {
            let n = self.xor[i].remove(vt).unwrap();
            self.xor[i].push_back(n);
            return false;
        }
        if self.xor[i].len() < K {
            self.xor[i].push_back(nut);
            true
        } else {
            false // xô đầy: giữ nút cũ, bỏ nút mới
        }
    }

    pub fn tong_so_nut(&self) -> usize { self.xor.iter().map(|x| x.len()).sum() }

    /// `quantity` nút gần `dich` nhất mà ta biết.
    pub fn nearest(&self, dich: NodeId, quantity: usize) -> Vec<NodeId> {
        let mut v: Vec<NodeId> = self.xor.iter().flatten().copied().collect();
        v.sort_by_key(|n| n.distance(dich));
        v.truncate(quantity);
        v
    }
}

// ============================================================================
// 3. TRA CỨU LẶP — tìm nút gần đích nhất trong O(log n) vòng
// ============================================================================

#[derive(Debug, PartialEq)]
pub struct KetQuaTraCuu {
    pub nearest: Vec<NodeId>,
    pub num_round: usize,
    pub so_nut_da_hoi: usize,
}

/// Mạng mô phỏng: mỗi nút có bảng định tuyến riêng.
pub struct BucketArray { pub nut: BTreeMap<NodeId, RoutingTable> }

impl BucketArray {
    /// Dựng mạng và cho các nút "gặp nhau" theo kiểu bootstrap thật:
    /// mỗi nút mới tự tra cứu chính mình qua một nút đã có sẵn.
    pub fn dung(cac_ma: &[u64]) -> BucketArray {
        let mut m = BucketArray { nut: BTreeMap::new() };
        for &x in cac_ma {
            let id = NodeId(x);
            m.nut.insert(id, RoutingTable::new(id));
        }
        // Vài vòng trao đổi để bảng định tuyến hội tụ
        let all: Vec<NodeId> = m.nut.keys().copied().collect();
        for _ in 0..3 {
            for &a in &all {
                for &b in &all {
                    if a != b { m.nut.get_mut(&a).unwrap().them(b); }
                }
            }
        }
        m
    }

    /// Tra cứu lặp: hỏi α nút gần nhất đã biết, chúng trả về nút chúng biết,
    /// lặp lại cho tới khi không tiến gần hơn được nữa.
    pub fn tra_cuu(&self, tu: NodeId, dich: NodeId, alpha: usize) -> KetQuaTraCuu {
        let mut candidates: Vec<NodeId> = self.nut[&tu].nearest(dich, K);
        let mut da_hoi: HashSet<NodeId> = HashSet::new();
        let mut num_round = 0;

        loop {
            let hoi: Vec<NodeId> = candidates.iter().copied()
                .filter(|n| !da_hoi.contains(n)).take(alpha).collect();
            if hoi.is_empty() { break; }
            num_round += 1;
            let mut new = Vec::new();
            for n in hoi {
                da_hoi.insert(n);
                if let Some(b) = self.nut.get(&n) { new.extend(b.nearest(dich, K)); }
            }
            let prev = candidates.first().map(|n| n.distance(dich));
            candidates.extend(new);
            candidates.sort_by_key(|n| n.distance(dich));
            candidates.dedup();
            candidates.truncate(K);
            // Không tiến gần hơn → dừng. Đây là điều kiện hội tụ của Kademlia.
            if candidates.first().map(|n| n.distance(dich)) == prev && num_round > 1 { break; }
            if num_round > 64 { break; } // chặn an toàn
        }
        KetQuaTraCuu { nearest: candidates, num_round, so_nut_da_hoi: da_hoi.len() }
    }
}

// ============================================================================
// 4. GOSSIP — lan truyền kiểu dịch bệnh
// ============================================================================

#[derive(Debug, PartialEq)]
pub struct ResultPropagate {
    pub num_round: usize,
    pub so_nut_nhan: usize,
    /// Tổng số bản tin đã gửi — thước đo chi phí băng thông.
    pub so_ban_tin: usize,
    pub fully_parallel: bool,
}

/// Mỗi nút chuyển tiếp bản tin cho `bac` hàng xóm, nhưng CHỈ LẦN ĐẦU thấy nó.
/// Không có bộ nhớ chống trùng thì mạng sẽ bão bản tin và tự sập.
pub fn gossip_propagate(
    neighbors: &HashMap<NodeId, Vec<NodeId>>,
    nguon: NodeId,
    bac: usize,
    max_num_round: usize,
) -> ResultPropagate {
    let mut seen: HashSet<NodeId> = HashSet::new();
    seen.insert(nguon);
    let mut dang_lan = vec![nguon];
    let mut so_ban_tin = 0;
    let mut num_round = 0;

    while !dang_lan.is_empty() && num_round < max_num_round {
        num_round += 1;
        let mut next = Vec::new();
        for n in &dang_lan {
            let lg = match neighbors.get(n) { Some(l) => l, None => continue };
            // Chọn `bac` hàng xóm một cách TẤT ĐỊNH (thật thì chọn ngẫu nhiên)
            for &m in lg.iter().take(bac) {
                so_ban_tin += 1;
                if seen.insert(m) { next.push(m); }
            }
        }
        dang_lan = next;
    }
    ResultPropagate {
        num_round,
        so_nut_nhan: seen.len(),
        so_ban_tin,
        fully_parallel: seen.len() == neighbors.len(),
    }
}

// ============================================================================
// 5. ĐỒNG THUẬN CHỊU LỖI BYZANTINE
// ============================================================================

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum ExecPos { TrungThuc, Im, HaiMat }

#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
pub enum Ballot { Thuan(u32), Chong }

/// Vì sao cần 3f+1 nút để chịu được f nút phản bội?
///
/// Ta phải quyết định dù f nút không trả lời, nên chỉ chờ được n−f phiếu.
/// Trong n−f phiếu đó có thể có tới f phiếu gian, còn lại n−2f là thật.
/// Muốn phe thật áp đảo phe gian: n−2f > f  ⟺  n > 3f.
/// Vậy n = 3f+1 là con số NHỎ NHẤT dùng được — không phải quy ước tuỳ tiện.
pub fn fault_tolerance(n: usize) -> usize { (n - 1) / 3 }

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
pub fn quorum_threshold(n: usize) -> usize {
    let f = fault_tolerance(n);
    (n + f) / 2 + 1
}

#[derive(Debug, PartialEq)]
pub struct ResultRound {
    pub decide: Option<u32>,
    pub votes_received: usize,
    pub threshold_can: usize,
}

/// Một vòng đồng thuận kiểu Tendermint/PBFT rút gọn: nút đề xuất phát giá trị,
/// các nút bỏ phiếu, đạt quorum thì chốt.
pub fn consensus_round(hanh_vi: &[ExecPos], gia_tri_de_xuat: u32) -> ResultRound {
    let n = hanh_vi.len();
    let threshold = quorum_threshold(n);
    let mut thung: HashMap<Ballot, usize> = HashMap::new();

    for (i, &h) in hanh_vi.iter().enumerate() {
        match h {
            ExecPos::TrungThuc => *thung.entry(Ballot::Thuan(gia_tri_de_xuat)).or_insert(0) += 1,
            ExecPos::Im => {}  // không gửi gì — lỗi "dừng", dạng nhẹ nhất
            ExecPos::HaiMat => {
                // Nút phản bội gửi giá trị KHÁC NHAU cho các nhóm khác nhau.
                // Đây là lỗi Byzantine thực thụ, khó hơn hẳn lỗi "im lặng".
                *thung.entry(Ballot::Thuan(gia_tri_de_xuat.wrapping_add(i as u32 + 1)))
                    .or_insert(0) += 1;
            }
        }
    }
    let good_nhat = thung.iter().max_by_key(|(_, &c)| c);
    let (decide, so_phieu) = match good_nhat {
        Some((Ballot::Thuan(v), &c)) if c >= threshold => (Some(*v), c),
        Some((_, &c)) => (None, c),
        None => (None, 0),
    };
    ResultRound { decide, votes_received: so_phieu, threshold_can: threshold }
}

// ============================================================================
// 6. BẢNG BĂM PHÂN TÁN — lưu và tìm dữ liệu không cần máy chủ
// ============================================================================

pub struct HashMapPartTan {
    pub mang: BucketArray,
    /// Mỗi nút giữ một phần kho. Dữ liệu nằm ở `r` nút gần khoá nhất.
    pub store: HashMap<NodeId, HashMap<u64, String>>,
    pub he_so_nhan_ban: usize,
}

impl HashMapPartTan {
    pub fn new(cac_ma: &[u64], he_so_nhan_ban: usize) -> Self {
        let mang = BucketArray::dung(cac_ma);
        let store = cac_ma.iter().map(|&x| (NodeId(x), HashMap::new())).collect();
        HashMapPartTan { mang, store, he_so_nhan_ban }
    }

    /// Ghi vào `r` nút gần khoá nhất. Nhân bản là cách DHT chịu được việc
    /// nút rời mạng bất cứ lúc nào — điều xảy ra liên tục trong mạng thật.
    pub fn set(&mut self, tu: NodeId, key: u64, value: &str) -> usize {
        let kq = self.mang.tra_cuu(tu, NodeId(key), 3);
        let mut dich: Vec<NodeId> = kq.nearest;
        dich.truncate(self.he_so_nhan_ban);
        for n in &dich {
            self.store.get_mut(n).unwrap().insert(key, value.to_string());
        }
        dich.len()
    }

    pub fn lay(&self, tu: NodeId, key: u64) -> Option<String> {
        let kq = self.mang.tra_cuu(tu, NodeId(key), 3);
        for n in kq.nearest {
            if let Some(v) = self.store.get(&n).and_then(|k| k.get(&key)) {
                return Some(v.clone());
            }
        }
        None
    }

    /// Mô phỏng nút rời mạng — xoá cả dữ liệu nó giữ.
    pub fn nut_roi_mang(&mut self, nut: NodeId) {
        self.store.remove(&nut);
        self.mang.nut.remove(&nut);
        for (_, b) in self.mang.nut.iter_mut() {
            for x in b.xor.iter_mut() { x.retain(|&n| n != nut); }
        }
    }
}

fn main() {
    println!("═══════════════════════════════════════════════════════════");
    println!("   MẠNG NGANG HÀNG: KADEMLIA · GOSSIP · ĐỒNG THUẬN BFT     ");
    println!("═══════════════════════════════════════════════════════════");

    println!("\n1. KHOẢNG CÁCH XOR LÀ MỘT METRIC THẬT");
    let (a, b, c) = (NodeId(0b1010), NodeId(0b1100), NodeId(0b0001));
    println!("   d(a,b) = {} · d(b,a) = {} → đối xứng", a.distance(b), b.distance(a));
    println!("   d(a,c) = {} ≤ d(a,b) + d(b,c) = {} → bất đẳng thức tam giác",
             a.distance(c), a.distance(b) + b.distance(c));
    println!("   d(a,a) = {}", a.distance(a));

    println!("\n2. BẢNG ĐỊNH TUYẾN — biết ít mà tới được mọi nơi");
    let id: Vec<u64> = (0..64u64).map(|i| i.wrapping_mul(0x9E3779B97F4A7C15)).collect();
    let mang = BucketArray::dung(&id);
    let toi = NodeId(id[0]);
    let b0 = &mang.nut[&toi];
    println!("   Mạng {} nút · nút này chỉ lưu {} địa chỉ ({} xô không rỗng)",
             id.len(), b0.tong_so_nut(), b0.xor.iter().filter(|x| !x.is_empty()).count());

    println!("\n3. TRA CỨU LẶP");
    let dich = NodeId(id[50]);
    let kq = mang.tra_cuu(toi, dich, 3);
    println!("   Tìm {:x} → {} vòng, hỏi {} nút", dich.0, kq.num_round, kq.so_nut_da_hoi);
    println!("   Tìm thấy đúng đích: {}", kq.nearest.contains(&dich));

    println!("\n4. GOSSIP — đánh đổi tốc độ lấy băng thông");
    let mut lg: HashMap<NodeId, Vec<NodeId>> = HashMap::new();
    for (i, &x) in id.iter().enumerate() {
        // vòng tròn + vài dây cung → đồ thị "thế giới nhỏ"
        let l: Vec<NodeId> = [1, 2, 7, 19, 31].iter()
            .map(|d| NodeId(id[(i + d) % id.len()])).collect();
        lg.insert(NodeId(x), l);
    }
    for bac in [1usize, 2, 3, 5] {
        let r = gossip_propagate(&lg, toi, bac, 50);
        println!("   bậc {} → {:>2} vòng · phủ {:>2}/{} nút · {:>3} bản tin",
                 bac, r.num_round, r.so_nut_nhan, id.len(), r.so_ban_tin);
    }
    println!("   → Bậc cao phủ nhanh hơn nhưng tốn băng thông theo cấp số nhân.");

    println!("\n5. ĐỒNG THUẬN BYZANTINE — vì sao là 3f+1");
    for n in [4usize, 7, 10, 13, 100] {
        println!("   {:>3} nút → chịu được {:>2} nút phản bội · cần {:>2} phiếu",
                 n, fault_tolerance(n), quorum_threshold(n));
    }
    let hv = vec![ExecPos::TrungThuc; 10];
    println!("\n   10 nút, tăng dần số kẻ phản bội:");
    for so_gian in 0..5 {
        let mut h = hv.clone();
        for i in 0..so_gian { h[i] = ExecPos::HaiMat; }
        let r = consensus_round(&h, 42);
        println!("   {} kẻ gian → {:?} ({}/{} phiếu){}",
                 so_gian, r.decide, r.votes_received, r.threshold_can,
                 if so_gian > fault_tolerance(10) { "  ← vượt ngưỡng an toàn" } else { "" });
    }

    println!("\n6. BẢNG BĂM PHÂN TÁN — chịu được nút rời mạng");
    let mut dht = HashMapPartTan::new(&id, 3);
    let n = dht.set(toi, 0xDEADBEEF, "xin chao P2P");
    println!("   Ghi khoá 0xDEADBEEF vào {} nút gần nhất", n);
    println!("   Đọc lại: {:?}", dht.lay(NodeId(id[30]), 0xDEADBEEF));
    let giu: Vec<NodeId> = dht.store.iter()
        .filter(|(_, k)| k.contains_key(&0xDEADBEEF)).map(|(n, _)| *n).collect();
    dht.nut_roi_mang(giu[0]);
    println!("   Sau khi 1 nút giữ dữ liệu rời mạng: {:?}", dht.lay(NodeId(id[30]), 0xDEADBEEF));

    println!("\n═══════════════════════════════════════════════════════════");
    println!("   KHÔNG MÁY CHỦ, KHÔNG TIN NHAU, VẪN THỐNG NHẤT ĐƯỢC       ");
    println!("═══════════════════════════════════════════════════════════");
}

#[cfg(test)]
mod tests {
    use super::*;

    fn color_code(n: usize) -> Vec<u64> {
        (0..n as u64).map(|i| i.wrapping_mul(0x9E3779B97F4A7C15)).collect()
    }

    // ---------- Khoảng cách XOR ----------
    #[test]
    fn xor_thoa_ba_tinh_chat_cua_metric() {
        let id = color_code(24);
        for &x in &id {
            let a = NodeId(x);
            assert_eq!(a.distance(a), 0, "d(x,x) = 0");
            for &y in &id {
                let b = NodeId(y);
                assert_eq!(a.distance(b), b.distance(a), "đối xứng");
                for &z in &id {
                    let c = NodeId(z);
                    // Cộng trong u128: với hai giá trị 64-bit, tổng của chúng
                    // TRÀN u64. Đây là cái bẫy thật khi kiểm chứng metric XOR.
                    assert!(a.distance(c) as u128
                            <= a.distance(b) as u128 + b.distance(c) as u128,
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
        let id = color_code(20);
        for &x in &id { for &y in &id { for &z in &id {
            let (a, b, c) = (NodeId(x), NodeId(y), NodeId(z));
            assert_eq!(a.distance(c), a.distance(b) ^ b.distance(c));
        }}}
    }

    #[test]
    fn xor_duy_nhat_khoang_cach_bang_khong_khi_trung_nhau() {
        let a = NodeId(12345);
        assert_eq!(a.distance(a), 0);
        assert_ne!(a.distance(NodeId(12346)), 0);
        assert_eq!(a.only_num_xor(a), None, "khoảng cách 0 không thuộc xô nào");
    }

    #[test]
    fn chi_so_xo_khop_bit_khac_cao_nhat() {
        let a = NodeId(0b0000);
        assert_eq!(a.only_num_xor(NodeId(0b0001)), Some(0));
        assert_eq!(a.only_num_xor(NodeId(0b0010)), Some(1));
        assert_eq!(a.only_num_xor(NodeId(0b1000)), Some(3));
        assert_eq!(a.only_num_xor(NodeId(0b1001)), Some(3), "lấy bit CAO nhất khác nhau");
    }

    // ---------- Bảng định tuyến ----------
    #[test]
    fn xo_khong_bao_gio_vuot_qua_k() {
        let mut b = RoutingTable::new(NodeId(0));
        for i in 1..500u64 { b.them(NodeId(i)); }
        for (i, x) in b.xor.iter().enumerate() {
            assert!(x.len() <= K, "xô {} có {} nút, vượt K={}", i, x.len(), K);
        }
    }

    #[test]
    fn bang_dinh_tuyen_giu_nut_cu_khi_xo_day() {
        // Chống Sybil: kẻ tấn công bơm nút mới KHÔNG đẩy được nút cũ ra.
        let mut b = RoutingTable::new(NodeId(0));
        // các nút 8..11 đều thuộc xô 3
        for i in 8..8 + K as u64 { assert!(b.them(NodeId(i))); }
        assert_eq!(b.xor[3].len(), K);
        let cu: Vec<NodeId> = b.xor[3].iter().copied().collect();
        assert!(!b.them(NodeId(15)), "xô đầy → từ chối nút mới");
        assert_eq!(b.xor[3].iter().copied().collect::<Vec<_>>(), cu, "nút cũ nguyên vẹn");
    }

    #[test]
    fn gap_lai_nut_cu_day_no_len_cuoi_hang() {
        let mut b = RoutingTable::new(NodeId(0));
        for i in 8..12u64 { b.them(NodeId(i)); }
        assert_eq!(*b.xor[3].front().unwrap(), NodeId(8));
        assert!(!b.them(NodeId(8)), "gặp lại không tính là thêm mới");
        assert_eq!(*b.xor[3].back().unwrap(), NodeId(8), "nút vừa liên lạc lên cuối hàng");
    }

    #[test]
    fn no_from_add_main_minh() {
        let mut b = RoutingTable::new(NodeId(42));
        assert!(!b.them(NodeId(42)));
        assert_eq!(b.tong_so_nut(), 0);
    }

    #[test]
    fn near_nhat_sort_use_theo_distance() {
        let mut b = RoutingTable::new(NodeId(0));
        for i in 1..100u64 { b.them(NodeId(i)); }
        let dich = NodeId(50);
        let g = b.nearest(dich, 5);
        for w in g.windows(2) {
            assert!(w[0].distance(dich) <= w[1].distance(dich));
        }
    }

    #[test]
    fn bang_dinh_tuyen_nho_hon_nhieu_so_voi_ca_mang() {
        let id = color_code(256);
        let m = BucketArray::dung(&id);
        let b = &m.nut[&NodeId(id[0])];
        assert!(b.tong_so_nut() < id.len(),
                "biết {} trong tổng {} nút — đó là ý nghĩa của định tuyến log n",
                b.tong_so_nut(), id.len());
    }

    // ---------- Tra cứu ----------
    #[test]
    fn tra_cuu_tim_duoc_nut_dich() {
        let id = color_code(128);
        let m = BucketArray::dung(&id);
        let tu = NodeId(id[0]);
        for &x in id.iter().skip(1).take(20) {
            let kq = m.tra_cuu(tu, NodeId(x), 3);
            assert!(kq.nearest.contains(&NodeId(x)), "không tìm được nút {:x}", x);
        }
    }

    #[test]
    fn tra_cuu_hoi_it_hon_nhieu_so_voi_ca_mang() {
        let id = color_code(256);
        let m = BucketArray::dung(&id);
        let kq = m.tra_cuu(NodeId(id[0]), NodeId(id[200]), 3);
        assert!(kq.so_nut_da_hoi < id.len() / 2,
                "hỏi {} nút trên tổng {} — tra cứu phải RẺ", kq.so_nut_da_hoi, id.len());
        assert!(kq.num_round <= 64, "phải hội tụ, không lặp vô hạn");
    }

    #[test]
    fn tra_cuu_luon_dung_ke_ca_khoa_khong_ung_voi_nut_nao() {
        let id = color_code(64);
        let m = BucketArray::dung(&id);
        let key = NodeId(0x1234_5678_9ABC_DEF0);
        let kq = m.tra_cuu(NodeId(id[0]), key, 3);
        assert!(!kq.nearest.is_empty(), "vẫn phải trả về nút gần nhất");
        // kết quả phải thật sự là gần nhất trong toàn mạng
        let true_su_near_nhat = id.iter().map(|&x| NodeId(x))
            .min_by_key(|n| n.distance(key)).unwrap();
        assert!(kq.nearest.contains(&true_su_near_nhat),
                "tra cứu phải hội tụ về nút gần nhất thật sự");
    }

    // ---------- Gossip ----------
    #[test]
    fn gossip_phu_song_toan_mang_neu_do_thi_lien_thong() {
        let id = color_code(50);
        let mut lg = HashMap::new();
        for (i, &x) in id.iter().enumerate() {
            lg.insert(NodeId(x), vec![NodeId(id[(i + 1) % id.len()])]); // vòng tròn
        }
        let r = gossip_propagate(&lg, NodeId(id[0]), 1, 100);
        assert!(r.fully_parallel);
        assert_eq!(r.so_nut_nhan, 50);
    }

    #[test]
    fn bac_cao_hon_phu_song_nhanh_hon() {
        let id = color_code(64);
        let mut lg = HashMap::new();
        for (i, &x) in id.iter().enumerate() {
            lg.insert(NodeId(x), [1, 2, 7, 19, 31].iter()
                .map(|d| NodeId(id[(i + d) % id.len()])).collect());
        }
        let it = gossip_propagate(&lg, NodeId(id[0]), 1, 100);
        let many = gossip_propagate(&lg, NodeId(id[0]), 4, 100);
        assert!(many.num_round < it.num_round, "bậc cao phải phủ nhanh hơn");
        assert!(many.so_ban_tin > it.so_ban_tin, "và tốn nhiều băng thông hơn");
    }

    #[test]
    fn gossip_khong_bao_ban_tin_nho_chong_trung() {
        // Không có `seen` thì mỗi nút chuyển tiếp mãi mãi và mạng sập.
        let id = color_code(30);
        let mut lg = HashMap::new();
        for (i, &x) in id.iter().enumerate() {
            lg.insert(NodeId(x), (1..=5).map(|d| NodeId(id[(i + d) % id.len()])).collect());
        }
        let r = gossip_propagate(&lg, NodeId(id[0]), 5, 100);
        assert!(r.so_ban_tin <= id.len() * 5,
                "mỗi nút chỉ được chuyển tiếp MỘT lần: {} bản tin", r.so_ban_tin);
    }

    #[test]
    fn gossip_khong_toi_duoc_phan_mang_bi_co_lap() {
        let id = color_code(20);
        let mut lg = HashMap::new();
        // hai cụm rời nhau hoàn toàn
        for i in 0..10 { lg.insert(NodeId(id[i]), vec![NodeId(id[(i + 1) % 10])]); }
        for i in 10..20 { lg.insert(NodeId(id[i]), vec![NodeId(id[10 + (i + 1) % 10])]); }
        let r = gossip_propagate(&lg, NodeId(id[0]), 1, 100);
        assert_eq!(r.so_nut_nhan, 10, "chỉ phủ được cụm của mình");
        assert!(!r.fully_parallel, "phân mảnh mạng là rủi ro có thật");
    }

    // ---------- Đồng thuận ----------
    #[test]
    fn quorum_thoa_ca_an_toan_lan_song_con_voi_moi_n() {
        for n in 4..500usize {
            let f = fault_tolerance(n);
            let q = quorum_threshold(n);
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
            assert_eq!(quorum_threshold(n), 2 * f + 1, "n=3f+1 thì phải khớp 2f+1");
        }
        // Trường hợp "xấu": n = 5, f = 1 → 2f+1 = 3 là KHÔNG AN TOÀN
        assert_eq!(fault_tolerance(5), 1);
        assert_eq!(quorum_threshold(5), 4, "phải là 4, không phải 3");
        assert!(2 * 3 - 5 <= 1, "quorum 3 chỉ giao 1 nút — có thể chính là kẻ gian");
        assert!(2 * 4 - 5 > 1, "quorum 4 giao 3 nút — chắc chắn có nút trung thực");
    }

    #[test]
    fn dong_thuan_thanh_cong_khi_du_nut_trung_thuc() {
        let n = 10;
        let f = fault_tolerance(n); // 3
        for so_gian in 0..=f {
            let mut h = vec![ExecPos::TrungThuc; n];
            for i in 0..so_gian { h[i] = ExecPos::HaiMat; }
            let r = consensus_round(&h, 42);
            assert_eq!(r.decide, Some(42),
                       "{} kẻ gian (<= f={}) vẫn phải chốt được", so_gian, f);
        }
    }

    #[test]
    fn dong_thuan_that_bai_khi_vuot_nguong() {
        let n = 10;
        let f = fault_tolerance(n);
        let mut h = vec![ExecPos::TrungThuc; n];
        for i in 0..=f + 1 { h[i] = ExecPos::HaiMat; }
        let r = consensus_round(&h, 42);
        assert_eq!(r.decide, None, "quá f kẻ gian → THÀ DỪNG còn hơn chốt sai");
    }

    #[test]
    fn nut_im_lang_de_chiu_hon_nut_hai_mat() {
        // Lỗi "dừng" nhẹ hơn lỗi Byzantine: nút im chỉ không đóng góp,
        // còn nút hai mặt vừa không đóng góp vừa gây nhiễu phiếu.
        let n = 10;
        let mut im = vec![ExecPos::TrungThuc; n];
        let mut time = vec![ExecPos::TrungThuc; n];
        for i in 0..3 { im[i] = ExecPos::Im; time[i] = ExecPos::HaiMat; }
        assert_eq!(consensus_round(&im, 42).decide, Some(42));
        assert_eq!(consensus_round(&time, 42).decide, Some(42));
        // Cùng 7 phiếu thật; khác nhau ở chỗ nút hai mặt còn tạo thêm phiếu rác
        assert_eq!(consensus_round(&im, 42).votes_received, 7);
        assert_eq!(consensus_round(&time, 42).votes_received, 7);
    }

    #[test]
    fn mang_bon_nut_chiu_duoc_dung_mot_ke_phan_boi() {
        assert_eq!(fault_tolerance(4), 1);
        assert_eq!(quorum_threshold(4), 3);
        let r = consensus_round(&[ExecPos::TrungThuc, ExecPos::TrungThuc,
                                  ExecPos::TrungThuc, ExecPos::HaiMat], 7);
        assert_eq!(r.decide, Some(7));
        let r2 = consensus_round(&[ExecPos::TrungThuc, ExecPos::TrungThuc,
                                   ExecPos::HaiMat, ExecPos::HaiMat], 7);
        assert_eq!(r2.decide, None, "2 kẻ gian trên 4 nút là quá ngưỡng");
    }

    // ---------- DHT ----------
    #[test]
    fn dht_ghi_roi_doc_lai_duoc_tu_nut_bat_ky() {
        let id = color_code(64);
        let mut d = HashMapPartTan::new(&id, 3);
        d.set(NodeId(id[0]), 999, "gia tri");
        for &x in id.iter().take(10) {
            assert_eq!(d.lay(NodeId(x), 999), Some("gia tri".to_string()),
                       "mọi nút đều phải tìm ra dữ liệu");
        }
    }

    #[test]
    fn dht_nhan_ban_dung_so_luong() {
        let id = color_code(64);
        let mut d = HashMapPartTan::new(&id, 3);
        assert_eq!(d.set(NodeId(id[0]), 555, "x"), 3);
        let giu = d.store.values().filter(|k| k.contains_key(&555)).count();
        assert_eq!(giu, 3);
    }

    #[test]
    fn dht_song_sot_khi_mot_ban_sao_roi_mang() {
        let id = color_code(64);
        let mut d = HashMapPartTan::new(&id, 3);
        d.set(NodeId(id[0]), 777, "ben bi");
        let giu: Vec<NodeId> = d.store.iter()
            .filter(|(_, k)| k.contains_key(&777)).map(|(n, _)| *n).collect();
        d.nut_roi_mang(giu[0]);
        assert_eq!(d.lay(NodeId(id[40]), 777), Some("ben bi".to_string()),
                   "nhân bản 3 lần thì mất 1 vẫn đọc được");
    }

    #[test]
    fn dht_tra_none_cho_khoa_chua_tung_ghi() {
        let id = color_code(32);
        let d = HashMapPartTan::new(&id, 3);
        assert_eq!(d.lay(NodeId(id[0]), 12345), None);
    }
}
