//! Chương 59 — Thiết kế hệ thống mở rộng: cân bằng tải, băm nhất quán,
//! giới hạn tần suất, back-pressure. Bổ sung cho Chương 48–54.

use std::collections::{BTreeMap, HashMap, VecDeque};

// ============================================================================
// 1. CÂN BẰNG TẢI (Load Balancing) — ba chiến lược
// ============================================================================

#[derive(Debug, Clone)]
pub struct Server {
    pub name: String,
    pub active_connections: u32,
    pub weight: u32, // máy mạnh hơn có trọng số cao hơn
}

pub trait BalancingStrategy {
    fn pick<'a>(&mut self, servers: &'a [Server]) -> Option<&'a Server>;
}

/// Xoay vòng (Round-Robin): lần lượt từng máy.
#[derive(Default)]
pub struct RoundRobin {
    index: usize,
}
impl RoundRobin {
    pub fn new() -> Self {
        Self::default()
    }
}
impl BalancingStrategy for RoundRobin {
    fn pick<'a>(&mut self, servers: &'a [Server]) -> Option<&'a Server> {
        if servers.is_empty() {
            return None;
        }
        let m = &servers[self.index % servers.len()];
        self.index += 1;
        Some(m)
    }
}

/// Ít kết nối nhất (Least-Connections): gửi tới máy đang rảnh nhất.
pub struct LeastConnections;
impl BalancingStrategy for LeastConnections {
    fn pick<'a>(&mut self, servers: &'a [Server]) -> Option<&'a Server> {
        servers.iter().min_by_key(|m| m.active_connections)
    }
}

/// Xoay vòng có trọng số (Weighted): máy mạnh nhận nhiều hơn theo tỷ lệ trọng số.
/// Bản đơn giản này dồn các lượt của một máy liền nhau (a, b, b, b, c); nginx dùng
/// "smooth weighted round-robin" để xen kẽ (b, a, b, c, b) cho tải mượt hơn.
#[derive(Default)]
pub struct WeightedRoundRobin {
    count: u32,
}
impl WeightedRoundRobin {
    pub fn new() -> Self {
        Self::default()
    }
}
impl BalancingStrategy for WeightedRoundRobin {
    fn pick<'a>(&mut self, servers: &'a [Server]) -> Option<&'a Server> {
        if servers.is_empty() {
            return None;
        }
        let total: u32 = servers.iter().map(|m| m.weight).sum();
        if total == 0 {
            return servers.first();
        }
        let slot = self.count % total;
        self.count = self.count.wrapping_add(1); // không panic tràn số sau ~4 tỷ lượt
        let mut cumulative = 0;
        for m in servers {
            cumulative += m.weight;
            if slot < cumulative {
                return Some(m);
            }
        }
        servers.last()
    }
}

// ============================================================================
// 2. BĂM NHẤT QUÁN (Consistent Hashing) — thêm/bớt máy chủ không xáo trộn toàn bộ
// ============================================================================

/// Băm đơn giản, tất định (FNV-1a) — đủ cho minh họa.
pub fn hash_key(key: &str) -> u64 {
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
    ring: BTreeMap<u64, String>, // điểm trên vòng -> tên máy chủ
    virtual_nodes: u32,
}

impl ConsistentHashRing {
    pub fn new(virtual_nodes: u32) -> Self {
        ConsistentHashRing {
            ring: BTreeMap::new(),
            virtual_nodes,
        }
    }
    pub fn add_server(&mut self, name: &str) {
        for i in 0..self.virtual_nodes {
            self.ring
                .insert(hash_key(&format!("{}#{}", name, i)), name.to_string());
        }
    }
    pub fn remove_server(&mut self, name: &str) {
        self.ring.retain(|_, v| v != name);
    }
    /// Tìm máy chủ chịu trách nhiệm cho một khóa: điểm đầu tiên >= hash(khóa),
    /// hoặc quay vòng về đầu (vòng tròn).
    pub fn find_server(&self, key: &str) -> Option<&str> {
        if self.ring.is_empty() {
            return None;
        }
        let h = hash_key(key);
        self.ring
            .range(h..)
            .next()
            .or_else(|| self.ring.iter().next()) // quay vòng
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
    tokens: f64,
    refill_rate: f64, // token/giây
}

impl TokenBucket {
    pub fn new(capacity: f64, refill_rate: f64) -> Self {
        TokenBucket {
            capacity,
            tokens: capacity,
            refill_rate,
        }
    }
    /// Nạp token theo thời gian trôi qua (giây), rồi thử tiêu 1 token.
    pub fn try_acquire(&mut self, elapsed_secs: f64) -> bool {
        // Thời gian âm (đồng hồ lùi) không được "rút" token
        let elapsed_secs = elapsed_secs.max(0.0);
        self.tokens = (self.tokens + elapsed_secs * self.refill_rate).min(self.capacity);
        if self.tokens >= 1.0 {
            self.tokens -= 1.0;
            true
        } else {
            false
        }
    }
    pub fn tokens_left(&self) -> f64 {
        self.tokens
    }
}

// ============================================================================
// 4. BACK-PRESSURE — hàng đợi có giới hạn, từ chối khi đầy
// ============================================================================

#[derive(Debug, PartialEq)]
pub enum SendResult {
    Accepted,
    Rejected, // hàng đầy — báo ngược lên nguồn để nó chậm lại (back-pressure)
}

/// Hàng đợi có giới hạn: khi đầy, TỪ CHỐI thay vì phình vô hạn.
/// Đây là cốt lõi của back-pressure: hệ thống chậm phải BÁO cho hệ thống nhanh
/// biết mà giảm tốc, thay vì âm thầm chất đống đến khi hết RAM.
pub struct BoundedQueue<T> {
    queue: VecDeque<T>,
    capacity: usize,
    rejected: u64,
}

impl<T> BoundedQueue<T> {
    pub fn new(capacity: usize) -> Self {
        BoundedQueue {
            queue: VecDeque::new(),
            capacity,
            rejected: 0,
        }
    }
    pub fn send(&mut self, job: T) -> SendResult {
        if self.queue.len() >= self.capacity {
            self.rejected += 1;
            SendResult::Rejected
        } else {
            self.queue.push_back(job);
            SendResult::Accepted
        }
    }
    pub fn recv(&mut self) -> Option<T> {
        self.queue.pop_front()
    }
    pub fn pending(&self) -> usize {
        self.queue.len()
    }
    pub fn rejected_count(&self) -> u64 {
        self.rejected
    }
}

fn main() {
    println!("═══════════════════════════════════════════════════════════════");
    println!("   THIẾT KẾ HỆ THỐNG MỞ RỘNG: CÂN BẰNG TẢI · BĂM NHẤT QUÁN     ");
    println!("═══════════════════════════════════════════════════════════════");

    let servers = vec![
        Server {
            name: "web-1".into(),
            active_connections: 5,
            weight: 1,
        },
        Server {
            name: "web-2".into(),
            active_connections: 2,
            weight: 3,
        },
        Server {
            name: "web-3".into(),
            active_connections: 8,
            weight: 1,
        },
    ];

    println!("\n1. CÂN BẰNG TẢI");
    let mut rr = RoundRobin::new();
    let series: Vec<&str> = (0..5)
        .filter_map(|_| rr.pick(&servers).map(|m| m.name.as_str()))
        .collect();
    println!("   Xoay vòng     : {:?}", series);
    println!(
        "   Ít kết nối    : {:?}",
        LeastConnections.pick(&servers).map(|m| &m.name)
    ); // web-2 (2 kết nối)
    let mut wt = WeightedRoundRobin::new();
    let ws: Vec<&str> = (0..5)
        .filter_map(|_| wt.pick(&servers).map(|m| m.name.as_str()))
        .collect();
    println!("   Trọng số      : {:?} (web-2 xuất hiện nhiều nhất)", ws);

    println!("\n2. BĂM NHẤT QUÁN — thêm/bớt máy chủ ít xáo trộn");
    let mut ring = ConsistentHashRing::new(100);
    for m in ["cache-A", "cache-B", "cache-C"] {
        ring.add_server(m);
    }
    let keys = ["user:1", "user:2", "user:3", "user:4", "user:5"];
    let before: HashMap<&str, String> = keys
        .iter()
        .map(|k| (*k, ring.find_server(k).unwrap().to_string()))
        .collect();
    println!("   Trước khi bỏ cache-B: {:?}", before);
    ring.remove_server("cache-B");
    let mut unchanged = 0;
    for k in &keys {
        let after = ring.find_server(k).unwrap();
        if after == before[k] {
            unchanged += 1;
        }
    }
    println!(
        "   Sau khi bỏ cache-B: {}/{} khóa GIỮ NGUYÊN máy chủ",
        unchanged,
        keys.len()
    );
    println!("   → Băm thường (hash % N) sẽ xáo trộn GẦN NHƯ TẤT CẢ khóa!");

    println!("\n3. GIỚI HẠN TẦN SUẤT (Token Bucket: 3 token, đổ 1/giây)");
    let mut bucket = TokenBucket::new(3.0, 1.0);
    for i in 1..=5 {
        print!(
            "   Yêu cầu {} (tức thì): {} | ",
            i,
            if bucket.try_acquire(0.0) {
                "CHO"
            } else {
                "CHẶN"
            }
        );
    }
    println!();
    println!(
        "   Chờ 2 giây rồi thử lại: {}",
        if bucket.try_acquire(2.0) {
            "CHO"
        } else {
            "CHẶN"
        }
    );

    println!("\n4. BACK-PRESSURE (hàng đợi sức chứa 3)");
    let mut queue: BoundedQueue<u32> = BoundedQueue::new(3);
    for i in 1..=5 {
        println!("   Gửi việc {}: {:?}", i, queue.send(i));
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
            Server {
                name: "a".into(),
                active_connections: 5,
                weight: 1,
            },
            Server {
                name: "b".into(),
                active_connections: 2,
                weight: 3,
            },
            Server {
                name: "c".into(),
                active_connections: 8,
                weight: 1,
            },
        ]
    }

    #[test]
    fn round_robin_is_even_and_wraps() {
        let m = server3();
        let mut rr = RoundRobin::new();
        let names: Vec<&str> = (0..6).map(|_| rr.pick(&m).unwrap().name.as_str()).collect();
        assert_eq!(names, vec!["a", "b", "c", "a", "b", "c"]);
    }

    #[test]
    fn least_connections_picks_idlest() {
        assert_eq!(LeastConnections.pick(&server3()).unwrap().name, "b"); // b có 2 kết nối
    }

    #[test]
    fn weights_distribute_proportionally() {
        let m = server3(); // trọng số a=1, b=3, c=1 -> tổng 5
        let mut wt = WeightedRoundRobin::new();
        let mut count: HashMap<String, u32> = HashMap::new();
        for _ in 0..5 {
            *count.entry(wt.pick(&m).unwrap().name.clone()).or_insert(0) += 1;
        }
        assert_eq!(count["b"], 3); // b nhận 3/5
        assert_eq!(count["a"], 1);
        assert_eq!(count["c"], 1);
    }

    #[test]
    fn consistent_hash_minimizes_remapping() {
        let mut ring = ConsistentHashRing::new(150);
        for m in ["A", "B", "C", "D"] {
            ring.add_server(m);
        }
        let keys: Vec<String> = (0..1000).map(|i| format!("k{}", i)).collect();
        let before: HashMap<&String, String> = keys
            .iter()
            .map(|k| (k, ring.find_server(k).unwrap().to_string()))
            .collect();

        ring.remove_server("B"); // bỏ 1 trong 4 máy

        let kept = keys
            .iter()
            .filter(|k| ring.find_server(k).unwrap() == before[*k])
            .count();
        // Lý thuyết: chỉ ~1/4 khóa (thuộc B) phải di chuyển. Giữ nguyên phải > 60%.
        assert!(
            kept as f64 / 1000.0 > 0.6,
            "chỉ giữ {} khóa — xáo trộn quá nhiều",
            kept
        );
        // Và KHÔNG có khóa "bất thường": mọi khóa bị di chuyển đều từng thuộc B
        for k in &keys {
            let after = ring.find_server(k).unwrap();
            if after != before[k] {
                assert_eq!(
                    before[k], "B",
                    "khóa {} không thuộc B mà vẫn bị di chuyển",
                    k
                );
            }
        }
    }

    #[test]
    fn consistent_hash_keys_are_stable() {
        let mut ring = ConsistentHashRing::new(50);
        ring.add_server("X");
        ring.add_server("Y");
        // Cùng một khóa luôn cho cùng một máy chủ
        let a = ring.find_server("user:42").unwrap().to_string();
        let b = ring.find_server("user:42").unwrap().to_string();
        assert_eq!(a, b);
    }

    #[test]
    fn token_bucket_limits_and_refills() {
        let mut bucket = TokenBucket::new(3.0, 1.0);
        // 3 token đầu -> cho; token thứ 4 tức thì -> chặn
        assert!(bucket.try_acquire(0.0));
        assert!(bucket.try_acquire(0.0));
        assert!(bucket.try_acquire(0.0));
        assert!(!bucket.try_acquire(0.0));
        // Chờ 1 giây -> đổ lại 1 token -> cho đúng 1 lần
        assert!(bucket.try_acquire(1.0));
        assert!(!bucket.try_acquire(0.0));
    }

    #[test]
    fn token_bucket_never_exceeds_capacity() {
        let mut bucket = TokenBucket::new(2.0, 100.0);
        // chờ rất lâu nhưng token bị GHIM ở dung lượng, không tràn
        bucket.try_acquire(1000.0);
        assert!(bucket.tokens_left() <= 2.0);
    }

    #[test]
    fn back_pressure_rejects_when_full() {
        let mut queue: BoundedQueue<u32> = BoundedQueue::new(2);
        assert_eq!(queue.send(1), SendResult::Accepted);
        assert_eq!(queue.send(2), SendResult::Accepted);
        assert_eq!(queue.send(3), SendResult::Rejected); // đầy!
        assert_eq!(queue.rejected_count(), 1);
        // Lấy ra 1 -> có chỗ -> nhận lại được
        assert_eq!(queue.recv(), Some(1));
        assert_eq!(queue.send(3), SendResult::Accepted);
    }
}
