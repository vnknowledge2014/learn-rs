# Chương 71: Mạng ngang hàng — Kademlia, Gossip & Đồng thuận Byzantine (P2P Networking)

## Giới thiệu & Mục tiêu học tập

Chương 70 dựng được một blockchain chạy trên **một máy**. Nhưng blockchain một máy thì chỉ là cơ sở dữ liệu có băm. Giá trị thật nằm ở chỗ **hàng nghìn máy không tin nhau vẫn đồng ý được với nhau**.

Chương này dựng ba lớp làm nên điều đó:

| Lớp | Câu hỏi nó trả lời | Ý tưởng cốt lõi |
|---|---|---|
| Kademlia | "Ai đang giữ dữ liệu X?" | Khoảng cách XOR + bảng định tuyến log(n) |
| Gossip | "Làm sao tin lan tới mọi người?" | Mỗi nút kể cho vài hàng xóm, lặp lại |
| Byzantine | "Tin ai khi có kẻ nói dối?" | Ngưỡng quorum, đa số áp đảo |

Điểm nhấn của chương: một **lỗi công thức phổ biến**. Sách vở hay viết quorum = `2f+1`. Công thức đó **chỉ đúng khi n = 3f+1**. Với n = 5, nó cho quorum bằng 3 — và 3 là **không an toàn**. Chúng ta sẽ dựng công thức tổng quát và kiểm chứng cả tính an toàn lẫn tính sống của nó.

---

## Hình tượng hóa đời sống

```
┌──────────────────────────────────────────────────────────────────────────────┐
│  KHOẢNG CÁCH XOR = "SỐ NHÀ KHÁC NHAU TỪ CHỮ SỐ NÀO?"                        │
│                                                                              │
│   Nút A = 1011 0110      Nút B = 1011 0010                                  │
│   XOR   = 0000 0100  → khác nhau bắt đầu từ bit thứ 5 → RẤT GẦN            │
│                                                                              │
│   Đây KHÔNG phải khoảng cách địa lý. Hai nút "gần" nhau trong Kademlia      │
│   có thể ở hai châu lục. Gần nghĩa là ID giống nhau ở nhiều bit đầu.        │
│                                                                              │
│  BẢNG ĐỊNH TUYẾN = "TÔI QUEN NHIỀU NGƯỜI GẦN, ÍT NGƯỜI XA"                 │
│                                                                              │
│   thùng 0 (nửa mạng ở xa)    : nhớ 20 người   ← xa, nhớ thưa                │
│   thùng 1 (1/4 mạng)         : nhớ 20 người                                 │
│   ...                                                                       │
│   thùng 255 (1 người)        : nhớ 20 người   ← gần, nhớ dày                │
│                                                                              │
│   Mỗi bước hỏi cắt đôi không gian tìm kiếm → tìm bất kỳ ai trong log₂(n)   │
│   bước. Mạng 1 triệu nút? 20 bước. Mạng 1 tỷ nút? 30 bước.                  │
│                                                                              │
│  GOSSIP = TIN ĐỒN TRONG LÀNG                                                │
│   Vòng 0:  ●                          1 người biết                          │
│   Vòng 1:  ● → ● ● ●                  4 người biết                          │
│   Vòng 2:  mỗi người kể cho 3 người   13 người biết                         │
│   Vòng 3:  ...                        40 người biết                         │
│   Số người biết tăng theo cấp SỐ NHÂN → phủ mạng trong O(log n) vòng.       │
│   Không có nút trung tâm nào để đánh sập.                                   │
│                                                                              │
│  BYZANTINE = HỘI ĐỒNG CÓ NGƯỜI NÓI DỐI                                      │
│   7 tướng, tối đa 2 kẻ phản bội. Cần bao nhiêu phiếu để chắc chắn?          │
│   Hai quorum BẮT BUỘC phải giao nhau ở ít nhất 1 người TRUNG THỰC.          │
│   → q > (n+f)/2. Với n=7, f=2: q > 4.5 → q = 5.                            │
│   ⚠ Công thức "2f+1" cho q=5 ở đây là đúng, NHƯNG với n=5, f=1 nó cho      │
│     q=3, mà 2·3−5 = 1 KHÔNG > f=1 → hai quorum có thể chỉ giao ở kẻ dối.   │
└──────────────────────────────────────────────────────────────────────────────┘
```

---

## Khái niệm & Cơ chế kỹ thuật chuyên sâu

### 1. Vì sao XOR mà không phải phép trừ?

Kademlia dùng `d(a,b) = a ⊕ b`. Lựa chọn này không tuỳ tiện — XOR có ba tính chất mà phép trừ không có đủ:

- **Đối xứng**: `d(a,b) = d(b,a)`. Nghĩa là nếu A thấy B gần thì B cũng thấy A gần. Nhờ vậy, mỗi truy vấn đi qua đều **đồng thời** làm giàu bảng định tuyến của cả hai bên — mạng tự học từ lưu lượng bình thường.
- **Đơn hướng**: với mỗi khoảng cách d và điểm a, có **đúng một** b thoả `d(a,b) = d`. Điều này khiến mọi truy vấn cho cùng một khoá đều hội tụ theo cùng một đường, nên bộ nhớ đệm dọc đường có tác dụng.
- **Bất đẳng thức tam giác** dạng chặt: thực ra XOR có tính chất **mạnh hơn** tam giác, đó là đẳng thức chính xác `d(a,c) = d(a,b) ⊕ d(b,c)`. Chương này kiểm thử cả hai.

Một cái bẫy khi cài: `d(a,b) + d(b,c)` có thể **tràn u64**. Bài kiểm thử phải ép sang `u128` — nếu không, bất đẳng thức tam giác "thất bại" một cách giả tạo.

### 2. Vì sao k-bucket đuổi nút MỚI chứ không đuổi nút CŨ?

Đây là chi tiết phản trực giác nhất của Kademlia. Khi một thùng đầy, nút mới bị **từ chối**, còn nút cũ vẫn ở lại.

Lý do dựa trên một quan sát thực nghiệm: trong mạng ngang hàng, **nút càng sống lâu thì xác suất tiếp tục sống càng cao**. Ưu tiên nút cũ khiến bảng định tuyến ổn định, và quan trọng hơn — nó **chống tấn công tràn ngập**: kẻ tấn công không thể bơm hàng nghìn nút mới để chiếm bảng định tuyến của người khác.

### 3. Gossip: đánh đổi giữa fanout và băng thông

Với fanout = 3, tin lan tới toàn mạng 10 000 nút trong khoảng 9 vòng. Tăng fanout lên 10 thì còn 5 vòng — nhưng lưu lượng mạng tăng hơn ba lần. Đây là đánh đổi trực tiếp giữa **độ trễ** và **băng thông**, và không có đáp án đúng chung: Bitcoin chọn fanout thấp (tiết kiệm băng thông), còn các mạng cần chốt nhanh chọn fanout cao.

Một tính chất quý: gossip **chịu được mất mát**. Ngay cả khi 20% thông điệp rơi, tin vẫn tới đích — chỉ chậm hơn vài vòng. Đó là vì mỗi nút nhận tin từ nhiều nguồn dư thừa.

### 4. Công thức quorum đúng

Ta cần hai điều kiện đồng thời:

- **An toàn**: hai quorum bất kỳ phải giao nhau ở ít nhất một nút trung thực → `2q − n > f`.
- **Sống**: quorum phải đạt được ngay cả khi f nút im lặng → `q ≤ n − f`.

Giải ra: `q = ⌊(n+f)/2⌋ + 1`. Với `f = ⌊(n−1)/3⌋`, cả hai điều kiện luôn thoả.

| n | f | q đúng | "2f+1" | 2q−n > f? |
|---|---|---|---|---|
| 4 | 1 | 3 | 3 | 2 > 1 ✓ |
| **5** | **1** | **4** | **3** ✗ | với q=3: 1 > 1 **sai** |
| 7 | 2 | 5 | 5 | 3 > 2 ✓ |
| **8** | **2** | **6** | **5** ✗ | với q=5: 2 > 2 **sai** |
| 10 | 3 | 7 | 7 | 4 > 3 ✓ |

Công thức `2f+1` chỉ trùng khớp khi `n = 3f+1`. Đó chính là lý do các hệ thống thật (Tendermint, HotStuff) luôn phát biểu ngưỡng theo **tỉ lệ quyền biểu quyết** (`> 2/3`) chứ không theo đếm nút.

---

## Mã nguồn minh họa thực chiến

Chạy bằng `cargo run -p ch71`, kiểm thử bằng `cargo test -p ch71`.

```rust
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
    pub fn distance(self, other: MaNut) -> u64 { self.0 ^ other.0 }

    /// Chỉ số "xô" = vị trí bit khác nhau cao nhất. Nút càng gần thì xô càng nhỏ.
    pub fn leading_bit_diff(self, other: MaNut) -> Option<u32> {
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
    pub toi: MaNut,
    pub xor: Vec<VecDeque<MaNut>>,
}

impl RoutingTable {
    pub fn new(toi: MaNut) -> Self {
        RoutingTable { toi, xor: (0..64).map(|_| VecDeque::new()).collect() }
    }

    /// Trả `true` nếu nút được thêm mới. Nút đã biết được đẩy lên cuối hàng —
    /// Kademlia ưu tiên giữ nút CŨ, vì nút sống lâu có xác suất sống tiếp cao hơn.
    /// Đây cũng là biện pháp chống tấn công Sybil: kẻ tấn công không thể tràn
    /// bảng định tuyến bằng cách bơm nút mới.
    pub fn them(&mut self, nut: MaNut) -> bool {
        let i = match self.toi.leading_bit_diff(nut) { Some(i) => i as usize, None => return false };
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
    pub fn nearest(&self, dich: MaNut, quantity: usize) -> Vec<MaNut> {
        let mut v: Vec<MaNut> = self.xor.iter().flatten().copied().collect();
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
    pub nearest: Vec<MaNut>,
    pub num_round: usize,
    pub so_nut_da_hoi: usize,
}

/// Mạng mô phỏng: mỗi nút có bảng định tuyến riêng.
pub struct BucketArray { pub nut: BTreeMap<MaNut, RoutingTable> }

impl BucketArray {
    /// Dựng mạng và cho các nút "gặp nhau" theo kiểu bootstrap thật:
    /// mỗi nút mới tự tra cứu chính mình qua một nút đã có sẵn.
    pub fn dung(ids: &[u64]) -> BucketArray {
        let mut m = BucketArray { nut: BTreeMap::new() };
        for &x in ids {
            let id = MaNut(x);
            m.nut.insert(id, RoutingTable::new(id));
        }
        // Vài vòng trao đổi để bảng định tuyến hội tụ
        let all: Vec<MaNut> = m.nut.keys().copied().collect();
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
    pub fn tra_cuu(&self, tu: MaNut, dich: MaNut, alpha: usize) -> KetQuaTraCuu {
        let mut candidates: Vec<MaNut> = self.nut[&tu].nearest(dich, K);
        let mut da_hoi: HashSet<MaNut> = HashSet::new();
        let mut num_round = 0;

        loop {
            let hoi: Vec<MaNut> = candidates.iter().copied()
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
    neighbors: &HashMap<MaNut, Vec<MaNut>>,
    nguon: MaNut,
    bac: usize,
    max_num_round: usize,
) -> ResultPropagate {
    let mut seen: HashSet<MaNut> = HashSet::new();
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
pub enum ExecPos { Honest, Im, HaiMat }

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
///   an toàn : hai quorum bất kỳ phải deliver nhau ở nhiều hơn f nút
///             ⟹ 2q − n > f  ⟺  q > (n+f)/2
///   sống còn: phải gom đủ phiếu dù f nút im lặng  ⟹  q ≤ n − f
///
/// Ví dụ n = 5, f = 1: công thức "2f+1" cho q = 3. Nhưng hai quorum 3 trên 5
/// chỉ deliver nhau ĐÚNG MỘT nút — và nút đó có thể chính là kẻ phản bội. Khi ấy
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
            ExecPos::Honest => *thung.entry(Ballot::Thuan(gia_tri_de_xuat)).or_insert(0) += 1,
            ExecPos::Im => {}  // không gửi gì — lỗi "dừng", dạng nhẹ nhất
            ExecPos::HaiMat => {
                // Nút phản bội gửi giá trị KHÁC NHAU cho các nhóm khác nhau.
                // Đây là lỗi Byzantine thực thụ, khó hơn hẳn lỗi "im lặng".
                *thung.entry(Ballot::Thuan(gia_tri_de_xuat.wrapping_add(i as u32 + 1)))
                    .or_insert(0) += 1;
            }
        }
    }
    let best = thung.iter().max_by_key(|(_, &c)| c);
    let (decide, so_phieu) = match best {
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
    pub store: HashMap<MaNut, HashMap<u64, String>>,
    pub he_so_nhan_ban: usize,
}

impl HashMapPartTan {
    pub fn new(ids: &[u64], he_so_nhan_ban: usize) -> Self {
        let mang = BucketArray::dung(ids);
        let store = ids.iter().map(|&x| (MaNut(x), HashMap::new())).collect();
        HashMapPartTan { mang, store, he_so_nhan_ban }
    }

    /// Ghi vào `r` nút gần khoá nhất. Nhân bản là cách DHT chịu được việc
    /// nút rời mạng bất cứ lúc nào — điều xảy ra liên tục trong mạng thật.
    pub fn set(&mut self, tu: MaNut, key: u64, value: &str) -> usize {
        let kq = self.mang.tra_cuu(tu, MaNut(key), 3);
        let mut dich: Vec<MaNut> = kq.nearest;
        dich.truncate(self.he_so_nhan_ban);
        for n in &dich {
            self.store.get_mut(n).unwrap().insert(key, value.to_string());
        }
        dich.len()
    }

    pub fn lay(&self, tu: MaNut, key: u64) -> Option<String> {
        let kq = self.mang.tra_cuu(tu, MaNut(key), 3);
        for n in kq.nearest {
            if let Some(v) = self.store.get(&n).and_then(|k| k.get(&key)) {
                return Some(v.clone());
            }
        }
        None
    }

    /// Mô phỏng nút rời mạng — xoá cả dữ liệu nó giữ.
    pub fn nut_roi_mang(&mut self, nut: MaNut) {
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
    let (a, b, c) = (MaNut(0b1010), MaNut(0b1100), MaNut(0b0001));
    println!("   d(a,b) = {} · d(b,a) = {} → đối xứng", a.distance(b), b.distance(a));
    println!("   d(a,c) = {} ≤ d(a,b) + d(b,c) = {} → bất đẳng thức tam giác",
             a.distance(c), a.distance(b) + b.distance(c));
    println!("   d(a,a) = {}", a.distance(a));

    println!("\n2. BẢNG ĐỊNH TUYẾN — biết ít mà tới được mọi nơi");
    let id: Vec<u64> = (0..64u64).map(|i| i.wrapping_mul(0x9E3779B97F4A7C15)).collect();
    let mang = BucketArray::dung(&id);
    let toi = MaNut(id[0]);
    let b0 = &mang.nut[&toi];
    println!("   Mạng {} nút · nút này chỉ lưu {} địa chỉ ({} xô không rỗng)",
             id.len(), b0.tong_so_nut(), b0.xor.iter().filter(|x| !x.is_empty()).count());

    println!("\n3. TRA CỨU LẶP");
    let dich = MaNut(id[50]);
    let kq = mang.tra_cuu(toi, dich, 3);
    println!("   Tìm {:x} → {} vòng, hỏi {} nút", dich.0, kq.num_round, kq.so_nut_da_hoi);
    println!("   Tìm thấy đúng đích: {}", kq.nearest.contains(&dich));

    println!("\n4. GOSSIP — đánh đổi tốc độ lấy băng thông");
    let mut lg: HashMap<MaNut, Vec<MaNut>> = HashMap::new();
    for (i, &x) in id.iter().enumerate() {
        // vòng tròn + vài dây cung → đồ thị "thế giới nhỏ"
        let l: Vec<MaNut> = [1, 2, 7, 19, 31].iter()
            .map(|d| MaNut(id[(i + d) % id.len()])).collect();
        lg.insert(MaNut(x), l);
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
    let hv = vec![ExecPos::Honest; 10];
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
    println!("   Đọc lại: {:?}", dht.lay(MaNut(id[30]), 0xDEADBEEF));
    let giu: Vec<MaNut> = dht.store.iter()
        .filter(|(_, k)| k.contains_key(&0xDEADBEEF)).map(|(n, _)| *n).collect();
    dht.nut_roi_mang(giu[0]);
    println!("   Sau khi 1 nút giữ dữ liệu rời mạng: {:?}", dht.lay(MaNut(id[30]), 0xDEADBEEF));

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
    fn xor_satisfies_metric_axioms() {
        let id = color_code(24);
        for &x in &id {
            let a = MaNut(x);
            assert_eq!(a.distance(a), 0, "d(x,x) = 0");
            for &y in &id {
                let b = MaNut(y);
                assert_eq!(a.distance(b), b.distance(a), "đối xứng");
                for &z in &id {
                    let c = MaNut(z);
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
    fn xor_gives_triangle_equality_not_just_inequality() {
        // Tính chất ĐẶC BIỆT của metric XOR, mạnh hơn hẳn bất đẳng thức thường:
        //   d(a,c) = d(a,b) ⊕ d(b,c)   — ĐẲNG THỨC, không phải "≤"
        // vì (a⊕b) ⊕ (b⊕c) = a⊕c. Nhờ nó, khoảng cách tính được theo từng chặng
        // mà không tích luỹ sai số, và không bao giờ tràn số.
        let id = color_code(20);
        for &x in &id { for &y in &id { for &z in &id {
            let (a, b, c) = (MaNut(x), MaNut(y), MaNut(z));
            assert_eq!(a.distance(c), a.distance(b) ^ b.distance(c));
        }}}
    }

    #[test]
    fn xor_distance_is_zero_only_for_identical_ids() {
        let a = MaNut(12345);
        assert_eq!(a.distance(a), 0);
        assert_ne!(a.distance(MaNut(12346)), 0);
        assert_eq!(a.leading_bit_diff(a), None, "khoảng cách 0 không thuộc xô nào");
    }

    #[test]
    fn bucket_index_matches_highest_differing_bit() {
        let a = MaNut(0b0000);
        assert_eq!(a.leading_bit_diff(MaNut(0b0001)), Some(0));
        assert_eq!(a.leading_bit_diff(MaNut(0b0010)), Some(1));
        assert_eq!(a.leading_bit_diff(MaNut(0b1000)), Some(3));
        assert_eq!(a.leading_bit_diff(MaNut(0b1001)), Some(3), "lấy bit CAO nhất khác nhau");
    }

    // ---------- Bảng định tuyến ----------
    #[test]
    fn bucket_never_exceeds_k() {
        let mut b = RoutingTable::new(MaNut(0));
        for i in 1..500u64 { b.them(MaNut(i)); }
        for (i, x) in b.xor.iter().enumerate() {
            assert!(x.len() <= K, "xô {} có {} nút, vượt K={}", i, x.len(), K);
        }
    }

    #[test]
    fn full_bucket_keeps_old_nodes() {
        // Chống Sybil: kẻ tấn công bơm nút mới KHÔNG đẩy được nút cũ ra.
        let mut b = RoutingTable::new(MaNut(0));
        // các nút 8..11 đều thuộc xô 3
        for i in 8..8 + K as u64 { assert!(b.them(MaNut(i))); }
        assert_eq!(b.xor[3].len(), K);
        let cu: Vec<MaNut> = b.xor[3].iter().copied().collect();
        assert!(!b.them(MaNut(15)), "xô đầy → từ chối nút mới");
        assert_eq!(b.xor[3].iter().copied().collect::<Vec<_>>(), cu, "nút cũ nguyên vẹn");
    }

    #[test]
    fn seeing_an_old_node_again_moves_it_to_the_tail() {
        let mut b = RoutingTable::new(MaNut(0));
        for i in 8..12u64 { b.them(MaNut(i)); }
        assert_eq!(*b.xor[3].front().unwrap(), MaNut(8));
        assert!(!b.them(MaNut(8)), "gặp lại không tính là thêm mới");
        assert_eq!(*b.xor[3].back().unwrap(), MaNut(8), "nút vừa liên lạc lên cuối hàng");
    }

    #[test]
    fn node_never_adds_itself() {
        let mut b = RoutingTable::new(MaNut(42));
        assert!(!b.them(MaNut(42)));
        assert_eq!(b.tong_so_nut(), 0);
    }

    #[test]
    fn closest_is_sorted_by_distance() {
        let mut b = RoutingTable::new(MaNut(0));
        for i in 1..100u64 { b.them(MaNut(i)); }
        let dich = MaNut(50);
        let g = b.nearest(dich, 5);
        for w in g.windows(2) {
            assert!(w[0].distance(dich) <= w[1].distance(dich));
        }
    }

    #[test]
    fn routing_table_is_far_smaller_than_the_network() {
        let id = color_code(256);
        let m = BucketArray::dung(&id);
        let b = &m.nut[&MaNut(id[0])];
        assert!(b.tong_so_nut() < id.len(),
                "biết {} trong tổng {} nút — đó là ý nghĩa của định tuyến log n",
                b.tong_so_nut(), id.len());
    }

    // ---------- Tra cứu ----------
    #[test]
    fn lookup_finds_the_target_node() {
        let id = color_code(128);
        let m = BucketArray::dung(&id);
        let tu = MaNut(id[0]);
        for &x in id.iter().skip(1).take(20) {
            let kq = m.tra_cuu(tu, MaNut(x), 3);
            assert!(kq.nearest.contains(&MaNut(x)), "không tìm được nút {:x}", x);
        }
    }

    #[test]
    fn lookup_queries_far_fewer_than_all_nodes() {
        let id = color_code(256);
        let m = BucketArray::dung(&id);
        let kq = m.tra_cuu(MaNut(id[0]), MaNut(id[200]), 3);
        assert!(kq.so_nut_da_hoi < id.len() / 2,
                "hỏi {} nút trên tổng {} — tra cứu phải RẺ", kq.so_nut_da_hoi, id.len());
        assert!(kq.num_round <= 64, "phải hội tụ, không lặp vô hạn");
    }

    #[test]
    fn lookup_works_even_for_unowned_keys() {
        let id = color_code(64);
        let m = BucketArray::dung(&id);
        let key = MaNut(0x1234_5678_9ABC_DEF0);
        let kq = m.tra_cuu(MaNut(id[0]), key, 3);
        assert!(!kq.nearest.is_empty(), "vẫn phải trả về nút gần nhất");
        // kết quả phải thật sự là gần nhất trong toàn mạng
        let truly_nearest = id.iter().map(|&x| MaNut(x))
            .min_by_key(|n| n.distance(key)).unwrap();
        assert!(kq.nearest.contains(&truly_nearest),
                "tra cứu phải hội tụ về nút gần nhất thật sự");
    }

    // ---------- Gossip ----------
    #[test]
    fn gossip_covers_a_connected_graph() {
        let id = color_code(50);
        let mut lg = HashMap::new();
        for (i, &x) in id.iter().enumerate() {
            lg.insert(MaNut(x), vec![MaNut(id[(i + 1) % id.len()])]); // vòng tròn
        }
        let r = gossip_propagate(&lg, MaNut(id[0]), 1, 100);
        assert!(r.fully_parallel);
        assert_eq!(r.so_nut_nhan, 50);
    }

    #[test]
    fn higher_fanout_covers_faster() {
        let id = color_code(64);
        let mut lg = HashMap::new();
        for (i, &x) in id.iter().enumerate() {
            lg.insert(MaNut(x), [1, 2, 7, 19, 31].iter()
                .map(|d| MaNut(id[(i + d) % id.len()])).collect());
        }
        let it = gossip_propagate(&lg, MaNut(id[0]), 1, 100);
        let many = gossip_propagate(&lg, MaNut(id[0]), 4, 100);
        assert!(many.num_round < it.num_round, "bậc cao phải phủ nhanh hơn");
        assert!(many.so_ban_tin > it.so_ban_tin, "và tốn nhiều băng thông hơn");
    }

    #[test]
    fn dedup_prevents_message_storms() {
        // Không có `seen` thì mỗi nút chuyển tiếp mãi mãi và mạng sập.
        let id = color_code(30);
        let mut lg = HashMap::new();
        for (i, &x) in id.iter().enumerate() {
            lg.insert(MaNut(x), (1..=5).map(|d| MaNut(id[(i + d) % id.len()])).collect());
        }
        let r = gossip_propagate(&lg, MaNut(id[0]), 5, 100);
        assert!(r.so_ban_tin <= id.len() * 5,
                "mỗi nút chỉ được chuyển tiếp MỘT lần: {} bản tin", r.so_ban_tin);
    }

    #[test]
    fn gossip_cannot_reach_a_partition() {
        let id = color_code(20);
        let mut lg = HashMap::new();
        // hai cụm rời nhau hoàn toàn
        for i in 0..10 { lg.insert(MaNut(id[i]), vec![MaNut(id[(i + 1) % 10])]); }
        for i in 10..20 { lg.insert(MaNut(id[i]), vec![MaNut(id[10 + (i + 1) % 10])]); }
        let r = gossip_propagate(&lg, MaNut(id[0]), 1, 100);
        assert_eq!(r.so_nut_nhan, 10, "chỉ phủ được cụm của mình");
        assert!(!r.fully_parallel, "phân mảnh mạng là rủi ro có thật");
    }

    // ---------- Đồng thuận ----------
    #[test]
    fn quorum_satisfies_safety_and_liveness_for_all_n() {
        for n in 4..500usize {
            let f = fault_tolerance(n);
            let q = quorum_threshold(n);
            assert!(3 * f + 1 <= n, "n={} phải chứa nổi 3f+1 với f={}", n, f);

            // AN TOÀN: hai quorum deliver nhau ở nhiều hơn f nút, nên luôn có ít
            // nhất một nút TRUNG THỰC nằm trong cả hai → không thể chốt hai
            // giá trị mâu thuẫn.
            let deliver = 2 * q as i64 - n as i64;
            assert!(deliver > f as i64,
                    "n={}: hai quorum deliver {} nút, phải nhiều hơn f={}", n, deliver, f);

            // SỐNG CÒN: gom đủ q phiếu ngay cả khi f nút im lặng hoàn toàn.
            assert!(q <= n - f, "n={}: cần {} phiếu nhưng chỉ chắc chắn có {}", n, q, n - f);
        }
    }

    #[test]
    fn the_2f_plus_1_formula_only_holds_when_n_is_3f_plus_1() {
        // Trường hợp "đẹp": n = 3f+1 → công thức phổ biến 2f+1 đúng
        for f in 1..50usize {
            let n = 3 * f + 1;
            assert_eq!(quorum_threshold(n), 2 * f + 1, "n=3f+1 thì phải khớp 2f+1");
        }
        // Trường hợp "xấu": n = 5, f = 1 → 2f+1 = 3 là KHÔNG AN TOÀN
        assert_eq!(fault_tolerance(5), 1);
        assert_eq!(quorum_threshold(5), 4, "phải là 4, không phải 3");
        assert!(2 * 3 - 5 <= 1, "quorum 3 chỉ deliver 1 nút — có thể chính là kẻ gian");
        assert!(2 * 4 - 5 > 1, "quorum 4 deliver 3 nút — chắc chắn có nút trung thực");
    }

    #[test]
    fn consensus_succeeds_with_enough_honest_nodes() {
        let n = 10;
        let f = fault_tolerance(n); // 3
        for so_gian in 0..=f {
            let mut h = vec![ExecPos::Honest; n];
            for i in 0..so_gian { h[i] = ExecPos::HaiMat; }
            let r = consensus_round(&h, 42);
            assert_eq!(r.decide, Some(42),
                       "{} kẻ gian (<= f={}) vẫn phải chốt được", so_gian, f);
        }
    }

    #[test]
    fn consensus_fails_beyond_the_threshold() {
        let n = 10;
        let f = fault_tolerance(n);
        let mut h = vec![ExecPos::Honest; n];
        for i in 0..=f + 1 { h[i] = ExecPos::HaiMat; }
        let r = consensus_round(&h, 42);
        assert_eq!(r.decide, None, "quá f kẻ gian → THÀ DỪNG còn hơn chốt sai");
    }

    #[test]
    fn silent_nodes_are_easier_than_equivocating_ones() {
        // Lỗi "dừng" nhẹ hơn lỗi Byzantine: nút im chỉ không đóng góp,
        // còn nút hai mặt vừa không đóng góp vừa gây nhiễu phiếu.
        let n = 10;
        let mut im = vec![ExecPos::Honest; n];
        let mut time = vec![ExecPos::Honest; n];
        for i in 0..3 { im[i] = ExecPos::Im; time[i] = ExecPos::HaiMat; }
        assert_eq!(consensus_round(&im, 42).decide, Some(42));
        assert_eq!(consensus_round(&time, 42).decide, Some(42));
        // Cùng 7 phiếu thật; khác nhau ở chỗ nút hai mặt còn tạo thêm phiếu rác
        assert_eq!(consensus_round(&im, 42).votes_received, 7);
        assert_eq!(consensus_round(&time, 42).votes_received, 7);
    }

    #[test]
    fn four_nodes_tolerate_exactly_one_traitor() {
        assert_eq!(fault_tolerance(4), 1);
        assert_eq!(quorum_threshold(4), 3);
        let r = consensus_round(&[ExecPos::Honest, ExecPos::Honest,
                                  ExecPos::Honest, ExecPos::HaiMat], 7);
        assert_eq!(r.decide, Some(7));
        let r2 = consensus_round(&[ExecPos::Honest, ExecPos::Honest,
                                   ExecPos::HaiMat, ExecPos::HaiMat], 7);
        assert_eq!(r2.decide, None, "2 kẻ gian trên 4 nút là quá ngưỡng");
    }

    // ---------- DHT ----------
    #[test]
    fn dht_value_readable_from_any_node() {
        let id = color_code(64);
        let mut d = HashMapPartTan::new(&id, 3);
        d.set(MaNut(id[0]), 999, "gia tri");
        for &x in id.iter().take(10) {
            assert_eq!(d.lay(MaNut(x), 999), Some("gia tri".to_string()),
                       "mọi nút đều phải tìm ra dữ liệu");
        }
    }

    #[test]
    fn dht_replicates_the_right_number_of_copies() {
        let id = color_code(64);
        let mut d = HashMapPartTan::new(&id, 3);
        assert_eq!(d.set(MaNut(id[0]), 555, "x"), 3);
        let giu = d.store.values().filter(|k| k.contains_key(&555)).count();
        assert_eq!(giu, 3);
    }

    #[test]
    fn dht_survives_losing_one_replica() {
        let id = color_code(64);
        let mut d = HashMapPartTan::new(&id, 3);
        d.set(MaNut(id[0]), 777, "ben bi");
        let giu: Vec<MaNut> = d.store.iter()
            .filter(|(_, k)| k.contains_key(&777)).map(|(n, _)| *n).collect();
        d.nut_roi_mang(giu[0]);
        assert_eq!(d.lay(MaNut(id[40]), 777), Some("ben bi".to_string()),
                   "nhân bản 3 lần thì mất 1 vẫn đọc được");
    }

    #[test]
    fn dht_returns_none_for_unknown_key() {
        let id = color_code(32);
        let d = HashMapPartTan::new(&id, 3);
        assert_eq!(d.lay(MaNut(id[0]), 12345), None);
    }
}
```

---

## Bảng tra cứu lỗi biên dịch thường gặp

| Lỗi | Nguyên nhân trong chương này | Cách sửa |
|---|---|---|
| `attempt to add with overflow` | `d(a,b) + d(b,c)` tràn `u64` khi kiểm bất đẳng thức tam giác | Ép sang `u128` trước khi cộng |
| `E0502: cannot borrow as mutable` | Vừa duyệt `self.cac_thung` vừa muốn `push` | Tính chỉ số thùng ra biến trước, rồi mượn `&mut` một lần |
| `E0507: cannot move out of index` | Lấy `HashSet` ra khỏi map bằng cách gán | Dùng `.clone()` hoặc thao tác qua tham chiếu |
| Bất đẳng thức tam giác "sai" | Quên rằng XOR có **đẳng thức** chặt hơn | Kiểm `d(a,c) == d(a,b) ^ d(b,c)` — luôn đúng |
| `leading_zeros` cho 256 | XOR của một nút với chính nó bằng 0 | Xử lý riêng trường hợp `distance == 0` |

---

## Tóm tắt chương & Bài tập rèn luyện

### 5 điểm cốt lõi

1. **XOR không phải là lựa chọn ngẫu nhiên** — tính đối xứng khiến mạng tự học bảng định tuyến từ lưu lượng thường.
2. **k-bucket ưu tiên nút cũ** vì tuổi thọ dự đoán tuổi thọ, và vì nó chặn tấn công tràn ngập.
3. **Gossip lan theo cấp số nhân** và chịu được mất mát — đánh đổi là băng thông dư thừa.
4. **Quorum đúng là `⌊(n+f)/2⌋+1`, không phải `2f+1`.** Công thức phổ biến chỉ đúng khi `n = 3f+1`.
5. **An toàn và sống luôn phải kiểm cùng lúc.** Quorum quá nhỏ → mất an toàn; quá lớn → hệ thống đứng khi có nút chết.

### Bài tập rèn luyện

**Bài 1.** Cài **gossip chống entropy (anti-entropy)**: định kỳ hai nút so danh sách thông điệp và trao đổi phần thiếu.

<details>
<summary><b>Gợi ý</b></summary>

Gossip đẩy (push) lan nhanh lúc đầu nhưng "đuôi" rất chậm — vài nút cuối cùng có thể không bao giờ nhận được. Anti-entropy sửa đúng chỗ đó: định kỳ đồng bộ toàn bộ với một nút ngẫu nhiên, đảm bảo hội tụ chắc chắn.
</details>

<details>
<summary><b>Lời giải</b></summary>

```rust
/// Trạng thái gossip của cả mạng: mỗi nút giữ tập thông điệp nó đã thấy.
pub struct MangChongEntropy {
    /// `BTreeMap` (không phải `HashMap`) để thứ tự duyệt tất định.
    pub seen: BTreeMap<MaNut, std::collections::BTreeSet<u64>>,
}

impl MangChongEntropy {
    pub fn new(nodes: &[MaNut]) -> Self {
        MangChongEntropy {
            seen: nodes.iter().map(|&n| (n, Default::default())).collect(),
        }
    }

    pub fn gieo(&mut self, nut: MaNut, greeting: u64) {
        self.seen.entry(nut).or_default().insert(greeting);
    }

    /// Đồng bộ hai chiều giữa hai nút. Trả về số thông điệp đã trao đổi.
    pub fn chong_entropy(&mut self, a: MaNut, b: MaNut) -> usize {
        if a == b { return 0; }
        let (ca, cb) = match (self.seen.get(&a), self.seen.get(&b)) {
            (Some(x), Some(y)) => (x.clone(), y.clone()),
            _ => return 0,
        };
        let mut exchanged = 0;
        for m in &ca { if self.seen.get_mut(&b).unwrap().insert(*m) { exchanged += 1; } }
        for m in &cb { if self.seen.get_mut(&a).unwrap().insert(*m) { exchanged += 1; } }
        exchanged
    }

    /// Chạy anti-entropy tới khi mọi nút hội tụ, hoặc hết `toi_da` vòng.
    pub fn hoi_tu(&mut self, toi_da: usize) -> Option<usize> {
        let nodes: Vec<MaNut> = self.seen.keys().copied().collect();
        let n = nodes.len();
        if n == 0 { return Some(0); }
        for round in 1..=toi_da {
            // Ghép cặp TẤT ĐỊNH: nút i đồng bộ với nút (i + vòng) mod n.
            // Cách ghép này bảo đảm mọi cặp gặp nhau trong nhiều nhất n−1 vòng,
            // nên hội tụ là CHẮC CHẮN, không phải xác suất.
            for i in 0..n {
                let (a, b) = (nodes[i], nodes[(i + round) % n]);
                self.chong_entropy(a, b);
            }
            let dich = self.seen[&nodes[0]].len();
            if self.seen.values().all(|x| x.len() == dich) { return Some(round); }
        }
        None
    }
}
```

Gossip đẩy lan rất nhanh lúc đầu nhưng "đuôi" chậm — vài nút cuối có thể không bao giờ nhận. Anti-entropy sửa đúng chỗ đó, và cách ghép cặp tất định biến hội tụ từ **xác suất** thành **chắc chắn**.
</details>

**Bài 2.** Cài **phát hiện nút đôi mặt (equivocation)**: kẻ Byzantine gửi giá trị khác nhau cho những người khác nhau.

<details>
<summary><b>Gợi ý</b></summary>

Nói dối nhất quán thì khó phát hiện. Nhưng nói **hai điều mâu thuẫn** cho hai người thì để lại bằng chứng: chỉ cần hai người so phiếu là lộ. Đây là nền của cơ chế "phạt cắt cọc" (slashing) trong proof-of-stake.
</details>

<details>
<summary><b>Lời giải</b></summary>

```rust
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct EquivocationProof {
    pub nut: usize,
    pub round: u64,
    pub gia_tri_a: u64,
    pub gia_tri_b: u64,
}

#[derive(Default)]
pub struct EquivocationDetector {
    /// (nút, vòng) → giá trị đã bỏ phiếu lần đầu
    seen: HashMap<(usize, u64), u64>,
    pub proofs: Vec<EquivocationProof>,
}

impl EquivocationDetector {
    pub fn quan_sat(&mut self, nut: usize, round: u64, value: u64) -> bool {
        match self.seen.get(&(nut, round)) {
            Some(&cu) if cu != value => {
                self.proofs.push(EquivocationProof {
                    nut, round, gia_tri_a: cu, gia_tri_b: value,
                });
                true // phát hiện đôi mặt
            }
            Some(_) => false,          // lặp lại đúng giá trị cũ — hợp lệ
            None => { self.seen.insert((nut, round), value); false }
        }
    }

    /// Danh sách nút đã bị chứng minh là Byzantine — đưa vào danh sách phạt.
    pub fn faulty_nodes(&self) -> Vec<usize> {
        let mut v: Vec<usize> = self.proofs.iter().map(|b| b.nut).collect();
        v.sort_unstable();
        v.dedup();
        v
    }
}
```

Điểm mấu chốt: bằng chứng đôi mặt **tự chứng minh** — bất kỳ ai cũng kiểm được, không cần tin người báo cáo. Trong proof-of-stake, đây chính là thứ cho phép tự động cắt cọc của kẻ gian.
</details>
