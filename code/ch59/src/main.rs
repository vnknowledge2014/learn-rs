#![allow(dead_code, unused_variables)]
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
    pub weight: u32, // máy mạnh hơn có trọng số cao hơn
}

pub trait StrategyCanTable {
    fn pick<'a>(&mut self, may_chu: &'a [Server]) -> Option<&'a Server>;
}

/// Xoay vòng (Round-Robin): lần lượt từng máy.
pub struct RoundRobin { pos_value: usize }
impl RoundRobin { pub fn new() -> Self { RoundRobin { pos_value: 0 } } }
impl StrategyCanTable for RoundRobin {
    fn pick<'a>(&mut self, may_chu: &'a [Server]) -> Option<&'a Server> {
        if may_chu.is_empty() { return None; }
        let m = &may_chu[self.pos_value % may_chu.len()];
        self.pos_value += 1;
        Some(m)
    }
}

/// Ít kết nối nhất (Least-Connections): gửi tới máy đang rảnh nhất.
pub struct FewConnect;
impl StrategyCanTable for FewConnect {
    fn pick<'a>(&mut self, may_chu: &'a [Server]) -> Option<&'a Server> {
        may_chu.iter().min_by_key(|m| m.current_connect)
    }
}

/// Xoay vòng có trọng số (Weighted): máy mạnh nhận nhiều hơn theo tỷ lệ trọng số.
pub struct WeightedRoundRobin { count: u32 }
impl WeightedRoundRobin { pub fn new() -> Self { WeightedRoundRobin { count: 0 } } }
impl StrategyCanTable for WeightedRoundRobin {
    fn pick<'a>(&mut self, may_chu: &'a [Server]) -> Option<&'a Server> {
        if may_chu.is_empty() { return None; }
        let tong: u32 = may_chu.iter().map(|m| m.weight).sum();
        if tong == 0 { return may_chu.first(); }
        let level = self.count % tong;
        self.count += 1;
        let mut accumulate = 0;
        for m in may_chu {
            accumulate += m.weight;
            if level < accumulate { return Some(m); }
        }
        may_chu.last()
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
/// Đây là cốt lõi của back-pressure: hệ thống chậm phải BÁO cho hệ thống nhanh
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
