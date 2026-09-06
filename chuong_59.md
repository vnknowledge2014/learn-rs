# Chương 59: Thiết kế hệ thống mở rộng — Cân bằng tải, Băm nhất quán, Giới hạn tần suất & Back-Pressure (Scaling Patterns)

## Giới thiệu & Mục tiêu học tập

Chủ đề 9 (Chương 48–54) đã dạy nền tảng hệ phân tán: Tokio, Actor, REST/gRPC, Redis, CAP, Raft. Nhưng khi đối chiếu với [system-design-primer](https://github.com/donnemartin/system-design-primer), còn một nhóm mẫu thiết kế **mở rộng theo chiều ngang (horizontal scaling)** chưa được nói tới — mà đây lại là những thứ được hỏi nhiều nhất trong phỏng vấn thiết kế hệ thống và dùng nhiều nhất trong thực tế.

Chương này bổ sung bốn mẫu cốt lõi, mỗi mẫu có mã Rust chạy được và test:
- **Cân bằng tải** — chia lưu lượng cho nhiều máy chủ (round-robin, ít-kết-nối, trọng số).
- **Băm nhất quán** — thêm/bớt máy chủ mà không xáo trộn toàn bộ dữ liệu (nền tảng của mọi cache và cơ sở dữ liệu phân tán).
- **Giới hạn tần suất** — bảo vệ hệ thống khỏi quá tải và lạm dụng (token bucket).
- **Back-pressure** — nghệ thuật biết nói "không" khi quá tải, thay vì sập âm thầm.

Một chủ đề xuyên suốt: **mở rộng ngang không chỉ là "thêm máy chủ"**, mà là *phân tán thông minh* và *biết từ chối đúng lúc*.

Mục tiêu học tập:
- So sánh **mở rộng dọc (vertical) và ngang (horizontal)** và biết khi nào dùng cái nào.
- Cài ba chiến lược **cân bằng tải** bằng trait (Chương 12) — đổi chiến lược không đổi mã gọi.
- Hiểu vì sao **băm thường (`hash % N`) là thảm họa** khi số máy chủ thay đổi, và **băm nhất quán** giải nó thế nào.
- Nhận ra **chất lượng hàm băm quyết định phân bố tải** — một bài học được chứng minh bằng test.
- Cài **token bucket** cho giới hạn tần suất và **hàng đợi giới hạn** cho back-pressure.
- Điền vào bản đồ các mẫu còn thiếu so với system-design-primer.

---

## Hình tượng hóa đời sống (Intuitive Everyday Analogy)

```
┌──────────────────────────────────────────────────────────────────────────────────┐
│         HÌNH TƯỢNG: MỘT CHUỖI NHÀ HÀNG ĐANG PHÁT TRIỂN NÓNG                       │
├──────────────────────────────────────────────────────────────────────────────────┤
│                                                                                  │
│  CÂN BẰNG TẢI = NGƯỜI XẾP BÀN Ở CỬA                                              │
│     "Round-robin": lần lượt bàn 1, 2, 3, 1, 2, 3...                              │
│     "Ít kết nối" : dẫn khách tới người phục vụ đang RẢNH nhất.                    │
│     "Trọng số"   : phục vụ giỏi (bàn to) nhận nhiều khách hơn theo tỷ lệ.        │
│                                                                                  │
│  BĂM NHẤT QUÁN = CÁCH CHIA KHÁCH QUEN CHO TỪNG PHỤC VỤ                           │
│     Băm thường (khách_id % số_phục_vụ): 1 người nghỉ việc → PHẢI CHIA LẠI        │
│        gần như TOÀN BỘ khách quen. Hỗn loạn!                                     │
│     Băm nhất quán: 1 người nghỉ → chỉ khách CỦA RIÊNG người đó cần chia lại.     │
│        Mọi khách quen khác giữ nguyên phục vụ. Êm đẹp.                           │
│                                                                                  │
│  GIỚI HẠN TẦN SUẤT = "MỖI KHÁCH TỐI ĐA 3 MÓN/PHÚT"                               │
│     Xô token: có sẵn 3 phiếu, mỗi món tốn 1 phiếu, mỗi phút phát lại 1 phiếu.    │
│     Cho phép gọi dồn 3 món một lúc, nhưng không thể gọi 100 món/phút.            │
│                                                                                  │
│  BACK-PRESSURE = "BẾP ĐÃ ĐẦY ĐƠN, TẠM NGƯNG NHẬN"                                │
│     Bếp làm không kịp → treo biển "hết chỗ" ở cửa, thay vì nhận vô hạn đơn       │
│     rồi để khách chờ 3 tiếng (và bếp cháy).                                      │
└──────────────────────────────────────────────────────────────────────────────────┘
```

---

## Khái niệm & Cơ chế kỹ thuật chuyên sâu (In-Depth Technical Mechanics)

### 1. Mở rộng dọc vs ngang

| | Dọc (Vertical) | Ngang (Horizontal) |
|---|---|---|
| Cách làm | Máy mạnh hơn (nhiều CPU/RAM) | Nhiều máy hơn |
| Giới hạn | Có trần vật lý, giá tăng phi tuyến | Gần như vô hạn |
| Điểm hỏng | Một máy sập = sập hết | Một máy sập = phần còn lại vẫn chạy |
| Độ phức tạp | Đơn giản | Cần cân bằng tải, băm nhất quán, đồng thuận (Raft, Ch53) |

Rust tỏa sáng ở mở rộng ngang: một dịch vụ Rust chỉ tốn ~15MB RAM (Chương 48), nên nhồi được 200–400 tiến trình trên một máy chủ Kubernetes, thay vì 10–20 như Java/Node.

### 2. Cân bằng tải — ba chiến lược, một trait

Ba chiến lược khác nhau về *cách chọn máy chủ tiếp theo*:
- **Round-robin**: đơn giản, công bằng khi các máy đồng đều và các yêu cầu tốn công như nhau.
- **Ít kết nối nhất**: tốt khi các yêu cầu tốn công không đều (một số kéo dài lâu).
- **Trọng số**: khi các máy chủ mạnh yếu khác nhau.

Trong Rust, cả ba cài chung một `trait StrategyCanTable` (Chương 12), nên đổi chiến lược không cần sửa mã gọi — đúng tinh thần đa hình và tiêm phụ thuộc.

### 3. Băm nhất quán — vì sao `hash % N` là thảm họa

Giả sử bạn có 4 máy cache và phân khóa bằng `hash(khóa) % 4`. Thêm một máy thứ 5 → công thức thành `% 5`. Kết quả: **gần như MỌI khóa đổi máy chủ**, vì `x % 4` và `x % 5` cho kết quả khác nhau với hầu hết `x`. Cache lạnh toàn bộ, cơ sở dữ liệu bị dồn tải đột ngột — có thể sập cả hệ thống chỉ vì thêm một máy.

**Băm nhất quán** đặt cả máy chủ lẫn khóa lên một *vòng tròn băm*. Khóa đi theo chiều kim đồng hồ tới máy chủ gần nhất. Khi thêm/bớt một máy, **chỉ những khóa trong một cung nhỏ** cần di chuyển — trung bình `1/N` số khóa, thay vì gần như tất cả.

Test `consistent_hash_minimizes_remapping` trong chương chứng minh: bỏ 1 trong 4 máy chỉ làm ~25% khóa di chuyển (giữ nguyên >60%, thực tế thường ~75%), và **0 khóa "bất thường"** — chỉ khóa của máy bị bỏ mới di chuyển.

### 4. Chất lượng hàm băm quyết định phân bố tải

Đây là bài học mà tôi học được *ngay trong lúc viết chương này*. Phiên bản đầu dùng FNV-1a trần, và test thất bại: một máy chủ ôm **590/1000 khóa** trong khi máy khác chỉ 10. Nguyên nhân: các điểm ảo có tên gần giống nhau (`"A#0"`, `"A#1"`...) cho giá trị băm gần nhau, nên chúng cụm lại một chỗ trên vòng thay vì rải đều.

Lời giải: thêm một **bộ trộn bit cuối (splitmix64 finalizer)** để đạt *hiệu ứng tuyết lở* — đổi 1 bit đầu vào làm đổi ~một nửa số bit đầu ra:

```rust
h ^= h >> 30; h = h.wrapping_mul(0xbf58476d1ce4e5b9);
h ^= h >> 27; h = h.wrapping_mul(0x94d049bb133111eb);
h ^= h >> 31;
```

Bài học tổng quát: **một cấu trúc dữ liệu đúng về mặt logic vẫn có thể hỏng vì hàm băm kém**. Đây cũng là lý do `HashMap` của Rust dùng SipHash (Chương 30) — vừa phân bố đều vừa chống tấn công HashDoS. Số điểm ảo cũng quan trọng: càng nhiều điểm ảo mỗi máy, phân bố càng đều (thường 100–200 điểm/máy).

### 5. Token Bucket và Back-Pressure

**Token bucket** cho phép *bùng nổ có kiểm soát*: bình thường tích lũy token, khi cần có thể tiêu dồn, nhưng tốc độ trung bình bị giới hạn bởi tốc độ đổ token. Đây là thuật toán giới hạn tần suất phổ biến nhất (dùng ở API gateway, Redis Ch52).

**Back-pressure** là triết lý: *khi quá tải, hãy TỪ CHỐI rõ ràng thay vì âm thầm chất đống*. Một hàng đợi không giới hạn là một quả bom hẹn giờ — nó phình đến khi hết RAM rồi giết cả tiến trình. Hàng đợi *có giới hạn* từ chối khi đầy, buộc nguồn gửi phải chậm lại. Trong Tokio (Chương 49), điều này được cài sẵn qua kênh `mpsc` có sức chứa (bounded channel): `send().await` sẽ *chờ* khi kênh đầy, tự động lan truyền áp lực ngược về nguồn.

### 6. Bản đồ mẫu thiết kế so với system-design-primer

| Mẫu | Chương |
|---|---|
| Monolith vs Microservices, Circuit Breaker | 48 |
| Async, Event Loop, Epoll | 49 |
| Actor, Channels | 50 |
| REST, gRPC, HTTP/2 | 51 |
| Caching (cache-aside, write-through), Redis, hiểm họa cache | 52 |
| CAP, Raft, đồng thuận, replication | 53 |
| Event Sourcing, State Machine, idempotency | 54 |
| **Cân bằng tải, Băm nhất quán, Rate limiting, Back-pressure** | **59 (chương này)** |
| CDN, DNS, Reverse Proxy | *xem ghi chú cuối chương* |

---

## Mã nguồn minh họa thực chiến (Idiomatic Runnable Rust Blueprint)

```bash
cd code
cargo run  -p ch59
cargo test -p ch59
```

```rust
//! Chương 59 — Thiết kế hệ thống mở rộng: cân bằng tải, băm nhất quán,
//! giới hạn tần suất, back-pressure. Bổ sung cho Chương 48–54.

use std::collections::{BTreeMap, HashMap, VecDeque};

// ============================================================================
// 1. CÂN BẰNG TẢI (Load Balancing) — ba chiến lược
// ============================================================================

#[derive(Debug, Clone)]
pub struct Server {
    pub name: String,
    pub current_connect: u32,
    pub weight: u32, // máy mạnh hơn có trọng số high hơn
}

pub trait StrategyCanTable {
    fn pick<'a>(&mut self, server: &'a [Server]) -> Option<&'a Server>;
}

/// Xoay vòng (Round-Robin): lần lượt từng máy.
pub struct RoundRobin { pos_value: usize }
impl RoundRobin { pub fn new() -> Self { RoundRobin { pos_value: 0 } } }
impl StrategyCanTable for RoundRobin {
    fn pick<'a>(&mut self, server: &'a [Server]) -> Option<&'a Server> {
        if server.is_empty() { return None; }
        let m = &server[self.pos_value % server.len()];
        self.pos_value += 1;
        Some(m)
    }
}

/// Ít kết nối nhất (Least-Connections): gửi tới máy đang rảnh nhất.
pub struct FewConnect;
impl StrategyCanTable for FewConnect {
    fn pick<'a>(&mut self, server: &'a [Server]) -> Option<&'a Server> {
        server.iter().min_by_key(|m| m.current_connect)
    }
}

/// Xoay vòng có trọng số (Weighted): máy mạnh nhận nhiều hơn theo tỷ lệ trọng số.
pub struct WeightedRoundRobin { count: u32 }
impl WeightedRoundRobin { pub fn new() -> Self { WeightedRoundRobin { count: 0 } } }
impl StrategyCanTable for WeightedRoundRobin {
    fn pick<'a>(&mut self, server: &'a [Server]) -> Option<&'a Server> {
        if server.is_empty() { return None; }
        let tong: u32 = server.iter().map(|m| m.weight).sum();
        if tong == 0 { return server.first(); }
        let level = self.count % tong;
        self.count += 1;
        let mut accumulate = 0;
        for m in server {
            accumulate += m.weight;
            if level < accumulate { return Some(m); }
        }
        server.last()
    }
}

// ============================================================================
// 2. BĂM NHẤT QUÁN (Consistent Hashing) — thêm/bớt máy chủ không xáo trộn toàn bộ
// ============================================================================

/// Băm đơn giản, tất định (FNV-1a) — đủ cho minh họa.
pub fn bam(key: &str) -> u64 {
    // FNV-1a để trộn từng byte...
    let mut h: u64 = 0xcbf29ce484222325;
    for b in key.bytes() {
        h ^= b as u64;
        h = h.wrapping_mul(0x100000001b3);
    }
    // ...rồi bộ trộn bit cuối (splitmix64 finalizer) để đạt "hiệu ứng tuyết lở":
    // đổi 1 bit đầu vào -> đổi ~1/2 số bit đầu ra. Thiếu bước này, các chuỗi
    // gần giống nhau ("A#0", "A#1") cho hash gần nhau -> vòng băm phân bố LỆCH.
    h ^= h >> 30;
    h = h.wrapping_mul(0xbf58476d1ce4e5b9);
    h ^= h >> 27;
    h = h.wrapping_mul(0x94d049bb133111eb);
    h ^= h >> 31;
    h
}

/// Vòng băm nhất quán. Mỗi máy chủ được đặt tại NHIỀU điểm ảo trên vòng,
/// để phân bố đều. Khóa đi theo chiều kim đồng hồ tới máy chủ gần nhất.
pub struct ConsistentHashRing {
    round: BTreeMap<u64, String>, // điểm trên vòng -> tên máy chủ
    so_diem_ao: u32,
}

impl ConsistentHashRing {
    pub fn new(so_diem_ao: u32) -> Self {
        ConsistentHashRing { round: BTreeMap::new(), so_diem_ao }
    }
    pub fn add_server(&mut self, name: &str) {
        for i in 0..self.so_diem_ao {
            self.round.insert(bam(&format!("{}#{}", name, i)), name.to_string());
        }
    }
    pub fn unit_server(&mut self, name: &str) {
        self.round.retain(|_, v| v != name);
    }
    /// Tìm máy chủ chịu trách nhiệm cho một khóa: điểm đầu tiên >= hash(khóa),
    /// hoặc quay vòng về đầu (vòng tròn).
    pub fn find_server(&self, key: &str) -> Option<&str> {
        if self.round.is_empty() { return None; }
        let h = bam(key);
        self.round.range(h..).next()
            .or_else(|| self.round.iter().next()) // quay vòng
            .map(|(_, v)| v.as_str())
    }
}

// ============================================================================
// 3. GIỚI HẠN TẦN SUẤT (Rate Limiting) — thuật toán Token Bucket
// ============================================================================

/// Xô token: mỗi yêu cầu tốn 1 token; token được đổ lại theo thời gian.
/// Cho phép "bùng nổ" ngắn (dùng token tích lũy) nhưng giới hạn tốc độ trung bình.
pub struct TokenBucket {
    capacity: f64,
    token: f64,
    measured_rate: f64, // token/giây
}

impl TokenBucket {
    pub fn new(capacity: f64, measured_rate: f64) -> Self {
        TokenBucket { capacity, token: capacity, measured_rate }
    }
    /// Nạp token theo thời gian trôi qua (giây), rồi thử tiêu 1 token.
    pub fn wait_op(&mut self, thoi_gian_troi: f64) -> bool {
        self.token = (self.token + thoi_gian_troi * self.measured_rate).min(self.capacity);
        if self.token >= 1.0 {
            self.token -= 1.0;
            true
        } else {
            false
        }
    }
    pub fn token_con(&self) -> f64 { self.token }
}

// ============================================================================
// 4. BACK-PRESSURE — hàng đợi có giới hạn, từ chối khi đầy
// ============================================================================

#[derive(Debug, PartialEq)]
pub enum KetQuaNhan {
    DaNhan,
    RejectReason, // hàng đầy — báo ngược lên nguồn để nó chậm lại (back-pressure)
}

/// Hàng đợi có giới hạn: khi đầy, TỪ CHỐI thay vì phình vô hạn.
/// Đây là cốt lõi của back-pressure: hệ thống chậm phải BÁO cho hệ thống fast
/// biết mà giảm tốc, thay vì âm thầm chất đống đến khi hết RAM.
pub struct QueueLimit<T> {
    queue: VecDeque<T>,
    capacity: usize,
    da_reject: u64,
}

impl<T> QueueLimit<T> {
    pub fn new(capacity: usize) -> Self {
        QueueLimit { queue: VecDeque::new(), capacity, da_reject: 0 }
    }
    pub fn send(&mut self, viec: T) -> KetQuaNhan {
        if self.queue.len() >= self.capacity {
            self.da_reject += 1;
            KetQuaNhan::RejectReason
        } else {
            self.queue.push_back(viec);
            KetQuaNhan::DaNhan
        }
    }
    pub fn nhan(&mut self) -> Option<T> { self.queue.pop_front() }
    pub fn so_cho(&self) -> usize { self.queue.len() }
    pub fn num_da_reject(&self) -> u64 { self.da_reject }
}

fn main() {
    println!("═══════════════════════════════════════════════════════════════");
    println!("   THIẾT KẾ HỆ THỐNG MỞ RỘNG: CÂN BẰNG TẢI · BĂM NHẤT QUÁN     ");
    println!("═══════════════════════════════════════════════════════════════");

    let may = vec![
        Server { name: "web-1".into(), current_connect: 5, weight: 1 },
        Server { name: "web-2".into(), current_connect: 2, weight: 3 },
        Server { name: "web-3".into(), current_connect: 8, weight: 1 },
    ];

    println!("\n1. CÂN BẰNG TẢI");
    let mut xv = RoundRobin::new();
    let series: Vec<&str> = (0..5).filter_map(|_| xv.pick(&may).map(|m| m.name.as_str())).collect();
    println!("   Xoay vòng     : {:?}", series);
    println!("   Ít kết nối    : {:?}", FewConnect.pick(&may).map(|m| &m.name)); // web-2 (2 kết nối)
    let mut wt = WeightedRoundRobin::new();
    let ws: Vec<&str> = (0..5).filter_map(|_| wt.pick(&may).map(|m| m.name.as_str())).collect();
    println!("   Trọng số      : {:?} (web-2 xuất hiện nhiều nhất)", ws);

    println!("\n2. BĂM NHẤT QUÁN — thêm/bớt máy chủ ít xáo trộn");
    let mut round = ConsistentHashRing::new(100);
    for m in ["cache-A", "cache-B", "cache-C"] { round.add_server(m); }
    let key = ["user:1", "user:2", "user:3", "user:4", "user:5"];
    let prev: HashMap<&str, String> = key.iter()
        .map(|k| (*k, round.find_server(k).unwrap().to_string())).collect();
    println!("   Trước khi bỏ cache-B: {:?}", prev);
    round.unit_server("cache-B");
    let mut giu_nguyen = 0;
    for k in &key {
        let next = round.find_server(k).unwrap();
        if next == prev[k] { giu_nguyen += 1; }
    }
    println!("   Sau khi bỏ cache-B: {}/{} khóa GIỮ NGUYÊN máy chủ", giu_nguyen, key.len());
    println!("   → Băm thường (hash % N) sẽ xáo trộn GẦN NHƯ TẤT CẢ khóa!");

    println!("\n3. GIỚI HẠN TẦN SUẤT (Token Bucket: 3 token, đổ 1/giây)");
    let mut xor = TokenBucket::new(3.0, 1.0);
    for i in 1..=5 {
        print!("   Yêu cầu {} (tức thì): {} | ", i, if xor.wait_op(0.0) { "CHO" } else { "CHẶN" });
    }
    println!();
    println!("   Chờ 2 giây rồi thử lại: {}", if xor.wait_op(2.0) { "CHO" } else { "CHẶN" });

    println!("\n4. BACK-PRESSURE (hàng đợi sức chứa 3)");
    let mut hq: QueueLimit<u32> = QueueLimit::new(3);
    for i in 1..=5 {
        println!("   Gửi việc {}: {:?}", i, hq.send(i));
    }
    println!("   → 2 việc bị TỪ CHỐI. Nguồn gửi phải chậm lại, không được ép thêm.");

    println!("\n═══════════════════════════════════════════════════════════════");
    println!("   MỞ RỘNG NGANG = PHÂN TÁN THÔNG MINH + BIẾT NÓI \"KHÔNG\"        ");
    println!("═══════════════════════════════════════════════════════════════");
}

#[cfg(test)]
mod tests {
    use super::*;

    fn server3() -> Vec<Server> {
        vec![
            Server { name: "a".into(), current_connect: 5, weight: 1 },
            Server { name: "b".into(), current_connect: 2, weight: 3 },
            Server { name: "c".into(), current_connect: 8, weight: 1 },
        ]
    }

    #[test]
    fn round_robin_is_even_and_wraps() {
        let m = server3();
        let mut xv = RoundRobin::new();
        let name: Vec<&str> = (0..6).map(|_| xv.pick(&m).unwrap().name.as_str()).collect();
        assert_eq!(name, vec!["a", "b", "c", "a", "b", "c"]);
    }

    #[test]
    fn least_connections_picks_idlest() {
        assert_eq!(FewConnect.pick(&server3()).unwrap().name, "b"); // b có 2 kết nối
    }

    #[test]
    fn weights_distribute_proportionally() {
        let m = server3(); // trọng số a=1, b=3, c=1 -> tổng 5
        let mut wt = WeightedRoundRobin::new();
        let mut count: HashMap<String, u32> = HashMap::new();
        for _ in 0..5 { *count.entry(wt.pick(&m).unwrap().name.clone()).or_insert(0) += 1; }
        assert_eq!(count["b"], 3); // b nhận 3/5
        assert_eq!(count["a"], 1);
        assert_eq!(count["c"], 1);
    }

    #[test]
    fn consistent_hash_minimizes_remapping() {
        let mut round = ConsistentHashRing::new(150);
        for m in ["A", "B", "C", "D"] { round.add_server(m); }
        let key: Vec<String> = (0..1000).map(|i| format!("k{}", i)).collect();
        let prev: HashMap<&String, String> =
            key.iter().map(|k| (k, round.find_server(k).unwrap().to_string())).collect();

        round.unit_server("B"); // bỏ 1 trong 4 máy

        let giu = key.iter().filter(|k| round.find_server(k).unwrap() == prev[*k]).count();
        // Lý thuyết: chỉ ~1/4 khóa (thuộc B) phải di chuyển. Giữ nguyên phải > 60%.
        assert!(giu as f64 / 1000.0 > 0.6, "chỉ giữ {} khóa — xáo trộn quá nhiều", giu);
    }

    #[test]
    fn consistent_hash_keys_are_stable() {
        let mut round = ConsistentHashRing::new(50);
        round.add_server("X");
        round.add_server("Y");
        // Cùng một khóa luôn cho cùng một máy chủ
        let a = round.find_server("user:42").unwrap().to_string();
        let b = round.find_server("user:42").unwrap().to_string();
        assert_eq!(a, b);
    }

    #[test]
    fn token_bucket_limits_and_refills() {
        let mut xor = TokenBucket::new(3.0, 1.0);
        // 3 token đầu -> cho; token thứ 4 tức thì -> chặn
        assert!(xor.wait_op(0.0));
        assert!(xor.wait_op(0.0));
        assert!(xor.wait_op(0.0));
        assert!(!xor.wait_op(0.0));
        // Chờ 1 giây -> đổ lại 1 token -> cho đúng 1 lần
        assert!(xor.wait_op(1.0));
        assert!(!xor.wait_op(0.0));
    }

    #[test]
    fn token_bucket_never_exceeds_capacity() {
        let mut xor = TokenBucket::new(2.0, 100.0);
        // chờ rất lâu nhưng token bị GHIM ở dung lượng, không tràn
        xor.wait_op(1000.0);
        assert!(xor.token_con() <= 2.0);
    }

    #[test]
    fn back_pressure_rejects_when_full() {
        let mut hq: QueueLimit<u32> = QueueLimit::new(2);
        assert_eq!(hq.send(1), KetQuaNhan::DaNhan);
        assert_eq!(hq.send(2), KetQuaNhan::DaNhan);
        assert_eq!(hq.send(3), KetQuaNhan::RejectReason); // đầy!
        assert_eq!(hq.num_da_reject(), 1);
        // Lấy ra 1 -> có chỗ -> nhận lại được
        assert_eq!(hq.nhan(), Some(1));
        assert_eq!(hq.send(3), KetQuaNhan::DaNhan);
    }
}
```

---

## CDN, DNS và Reverse Proxy — mô hình tinh thần

Ba thành phần này thường là *dịch vụ hạ tầng* bạn cấu hình chứ không tự viết, nhưng phải hiểu để thiết kế đúng:

- **DNS** (Domain Name System): "danh bạ" của Internet, dịch `congty.vn` → địa chỉ IP. Một mẹo mở rộng: DNS có thể trả về *nhiều* IP (DNS round-robin) — một tầng cân bằng tải thô sơ ngay trước khi request tới máy chủ.
- **CDN** (Content Delivery Network): đặt bản sao nội dung tĩnh (ảnh, JS, CSS) ở hàng trăm điểm gần người dùng. Về bản chất đây là **cache-aside phân tán theo địa lý** (Chương 52) — giảm độ trễ và gánh nặng cho máy chủ gốc.
- **Reverse Proxy** (nginx, Caddy): đứng trước các máy chủ ứng dụng, làm cửa ngõ duy nhất. Nó thường kiêm luôn: cân bằng tải (mục 1), kết thúc TLS, giới hạn tần suất (mục 5), và bộ đệm. Trong Rust, bạn có thể tự viết reverse proxy bằng `tokio` + `hyper` — nhưng thường dùng công cụ có sẵn.

> **Nguyên tắc thiết kế**: đẩy càng nhiều việc ra *rìa* (edge) càng tốt. CDN xử lý nội dung tĩnh, reverse proxy xử lý TLS và rate limit, để máy chủ ứng dụng chỉ tập trung vào logic nghiệp vụ — đúng tinh thần "lõi thuần túy, vỏ mệnh lệnh" ở Chương 20, nhưng ở quy mô hạ tầng.

---

## Bảng tra cứu lỗi biên dịch thường gặp

| Lỗi | Nguyên nhân trong chương này | Cách sửa |
|---|---|---|
| `E0038: the trait cannot be made into an object` | `Box<dyn StrategyCanTable>` mà trait có phương thức generic | Bỏ generic, hoặc dùng enum thay trait object |
| `E0106: missing lifetime specifier` | Trả `Option<&Server>` từ lát cắt truyền vào | Ràng buộc vòng đời tường minh: `fn pick<'a>(&mut self, s: &'a [Server]) -> Option<&'a Server>` |
| `E0502: cannot borrow as mutable` | `self.pos` đổi trong khi còn mượn `&self.servers` | Đọc chỉ số ra biến trước, tăng `self.pos` sau |
| `attempt to subtract with overflow` | Trừ dấu thời gian `u64` khi cửa sổ chưa đầy | `saturating_sub` — thời gian có thể chưa trôi đủ |
| Băm nhất quán phân bố lệch nặng | Quá ít điểm ảo, hoặc hàm băm trộn bit kém | Tăng số điểm ảo lên hàng trăm và dùng bộ trộn có hiệu ứng tuyết lở |

---

## Tóm tắt chương & Bài tập rèn luyện (Summary & Exercises)

### 4 Điểm cốt lõi cần ghi nhớ:
1. **Mở rộng ngang = phân tán thông minh + biết từ chối.** Thêm máy chủ chỉ là bước đầu; phân tải và xử lý quá tải mới là phần khó.
2. **Băm nhất quán thay cho `hash % N`.** Thêm/bớt máy chỉ di chuyển ~1/N khóa thay vì gần như tất cả — điều kiện sống còn cho cache và cơ sở dữ liệu phân tán.
3. **Chất lượng hàm băm quyết định phân bố tải.** Một vòng băm đúng logic vẫn lệch nặng nếu hàm băm trộn bit kém. Dùng bộ trộn có hiệu ứng tuyết lở và đủ điểm ảo.
4. **Back-pressure: từ chối rõ ràng khi quá tải.** Hàng đợi không giới hạn là quả bom RAM. Token bucket giới hạn tần suất; hàng đợi có giới hạn lan truyền áp lực ngược.

### Bài tập rèn luyện tự giải:

**Bài tập 1 (Cân bằng tải kèm kiểm tra sức khỏe)**
Thêm trường `khoe_manh: bool` vào `Server` và một chiến lược `XoayVongBoQuaChet` chỉ chọn máy đang khỏe. Test rằng máy chết không bao giờ được chọn.

<details>
<summary><b>Lời giải</b></summary>

```rust
/// Máy chủ có thêm cờ sức khoẻ. Bài tập yêu cầu thêm trường `healthy`
/// vào `Server`; ở đây tách thành kiểu riêng để lời giải biên dịch được
/// độc lập mà không phải sửa kiểu gốc của chương.
#[derive(Debug, Clone)]
pub struct HealthyServer {
    pub name: String,
    pub current_connect: u32,
    pub weight: u32,
    /// Do luồng kiểm tra sức khoẻ nền cập nhật.
    pub healthy: bool,
}

/// Xoay vòng nhưng BỎ QUA máy đã chết.
#[derive(Default)]
pub struct RoundRobinSkipDead { pos: usize }

impl RoundRobinSkipDead {
    pub fn new() -> Self { RoundRobinSkipDead { pos: 0 } }

    pub fn pick<'a>(&mut self, servers: &'a [HealthyServer]) -> Option<&'a HealthyServer> {
        // Lọc trước rồi mới xoay vòng: nếu xoay vòng trên cả danh sách rồi
        // mới bỏ máy chết, con trỏ sẽ nhảy cóc và phân tải mất đều.
        let alive: Vec<&HealthyServer> = servers.iter().filter(|s| s.healthy).collect();
        if alive.is_empty() { return None; }
        let s = alive[self.pos % alive.len()];
        self.pos += 1;
        Some(s)
    }
}

#[test]
fn dead_servers_are_never_picked() {
    let servers = vec![
        HealthyServer { name: "a".into(), current_connect: 0, weight: 1, healthy: true },
        HealthyServer { name: "b".into(), current_connect: 0, weight: 1, healthy: false },
        HealthyServer { name: "c".into(), current_connect: 0, weight: 1, healthy: true },
    ];
    let mut lb = RoundRobinSkipDead::new();
    let picked: Vec<&str> = (0..6).filter_map(|_| lb.pick(&servers)).map(|s| s.name.as_str()).collect();
    assert!(!picked.contains(&"b"), "máy chết không bao giờ được chọn");
    assert_eq!(picked, ["a", "c", "a", "c", "a", "c"], "vẫn xoay vòng đều giữa các máy sống");
}

#[test]
fn all_dead_returns_none() {
    let servers = vec![
        HealthyServer { name: "a".into(), current_connect: 0, weight: 1, healthy: false },
    ];
    // Không có máy nào sống thì phải TRẢ VỀ None, không phải chọn bừa.
    assert!(RoundRobinSkipDead::new().pick(&servers).is_none());
}
```

Trong thực tế, "kiểm tra sức khỏe" là một luồng nền định kỳ ping từng máy; máy không phản hồi bị đánh dấu chết và loại khỏi vòng cho tới khi hồi phục — đúng mẫu Circuit Breaker ở Chương 48.
</details>

**Bài tập 2 (Sliding window rate limiter)**
Token bucket cho phép bùng nổ. Đôi khi ta muốn giới hạn *chặt* "tối đa N yêu cầu trong 60 giây gần nhất". Viết `CuaSoTruot` lưu dấu thời gian các yêu cầu và loại bỏ cái quá cũ. (Truyền thời gian vào làm tham số để test tất định — bài học Chương 55.)

<details>
<summary><b>Gợi ý</b></summary>

Dùng `VecDeque<u64>` chứa dấu thời gian. Mỗi yêu cầu ở thời điểm `t`: loại mọi dấu `< t - 60`, rồi nếu số còn lại `< N` thì cho và ghi `t`. Đừng gọi `Instant::now()` bên trong — nhận `t` làm tham số để test được (Chương 55, bài tập 2).
</details>

<details>
<summary><b>Lời giải</b></summary>

```rust
use std::collections::VecDeque;
pub struct SlidingWindow { dau_thoi_gian: VecDeque<u64>, limit: usize, cua_so_giay: u64 }
impl SlidingWindow {
    pub fn new(limit: usize, cua_so_giay: u64) -> Self {
        SlidingWindow { dau_thoi_gian: VecDeque::new(), limit, cua_so_giay }
    }
    pub fn wait_op(&mut self, now: u64) -> bool {
        while let Some(&cu) = self.dau_thoi_gian.front() {
            if cu + self.cua_so_giay <= now { self.dau_thoi_gian.pop_front(); } else { break; }
        }
        if self.dau_thoi_gian.len() < self.limit {
            self.dau_thoi_gian.push_back(now);
            true
        } else { false }
    }
}
#[cfg(test)]
mod bt2 {
    use super::*;
    #[test]
    fn limits_three_requests_per_ten_seconds() {
        let mut cs = SlidingWindow::new(3, 10);
        assert!(cs.wait_op(0)); assert!(cs.wait_op(1)); assert!(cs.wait_op(2));
        assert!(!cs.wait_op(3));      // đã đủ 3 trong cửa sổ
        assert!(cs.wait_op(11));      // cái ở t=0 đã hết hạn (11 >= 0+10)
    }
}
```
</details>

**Bài tập 3 (Tư duy: chọn mẫu mở rộng)**
Với mỗi vấn đề, chọn mẫu phù hợp:
1. Một máy chủ web quá tải vào giờ cao điểm.
2. Cụm 10 máy cache Redis, cần thêm máy thứ 11 mà không làm lạnh cache.
3. Một API công khai bị một client gọi 10.000 lần/giây.
4. Dịch vụ gửi email chậm, các dịch vụ khác dồn hàng triệu email vào làm hết RAM.

<details>
<summary><b>Lời giải tham khảo</b></summary>

1. **Cân bằng tải + mở rộng ngang.** Thêm máy chủ, đặt sau reverse proxy chia tải.
2. **Băm nhất quán.** Thêm máy thứ 11 chỉ di chuyển ~1/11 khóa; 10/11 cache vẫn nóng.
3. **Giới hạn tần suất (token bucket / sliding window)** theo từng client, ở tầng API gateway.
4. **Back-pressure** (hàng đợi có giới hạn) + hàng đợi thông điệp (Chương 52). Dịch vụ email từ chối/đệm có kiểm soát, các nguồn buộc phải chậm lại.

Nguyên tắc: hỏi *"nút thắt cổ chai ở đâu, và điều gì xảy ra khi nó quá tải?"* — câu trả lời chỉ ra ngay mẫu cần dùng.
</details>
