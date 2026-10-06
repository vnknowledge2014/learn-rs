use std::collections::{HashMap, VecDeque};
use std::hash::Hash;
use std::sync::{Arc, Mutex};
use std::time::{Duration, Instant};

/// Một mục lưu trữ trong bộ đệm kèm thời gian sống (TTL)
#[derive(Clone, Debug)]
struct CacheEntry<V> {
    value: V,
    expires_at: Instant,
}

/// Động cơ Lưu trữ đệm an toàn đa luồng hỗ trợ TTL (In-Memory Cache Engine)
/// kèm cơ chế chống Cache Stampede kiểu "single-flight".
pub struct SafeCacheEngine<K, V> {
    storage: Mutex<HashMap<K, CacheEntry<V>>>,
    // Mỗi khóa đang được nạp có một "ổ khóa nạp" riêng: chỉ MỘT luồng đi truy vấn
    // nguồn dữ liệu, các luồng khác cùng khóa xếp hàng chờ rồi đọc lại từ Cache.
    loading: Mutex<HashMap<K, Arc<Mutex<()>>>>,
}

impl<K: Hash + Eq + Clone, V: Clone> Default for SafeCacheEngine<K, V> {
    fn default() -> Self {
        Self::new()
    }
}

impl<K: Hash + Eq + Clone, V: Clone> SafeCacheEngine<K, V> {
    pub fn new() -> Self {
        Self {
            storage: Mutex::new(HashMap::new()),
            loading: Mutex::new(HashMap::new()),
        }
    }

    /// Lưu dữ liệu vào Cache kèm thời gian sống TTL
    pub fn set(&self, key: K, value: V, ttl: Duration) {
        let mut store = self.storage.lock().unwrap();
        let entry = CacheEntry {
            value,
            expires_at: Instant::now() + ttl,
        };
        store.insert(key, entry);
    }

    /// Lấy dữ liệu từ Cache (Tự động bỏ qua nếu dữ liệu đã hết hạn)
    pub fn get(&self, key: &K) -> Option<V> {
        let mut store = self.storage.lock().unwrap();

        if let Some(entry) = store.get(key) {
            if Instant::now() < entry.expires_at {
                // Cache Hit: Dữ liệu còn hạn sử dụng!
                return Some(entry.value.clone());
            }
            // Đã hết hạn: dọn dẹp mục cũ
            store.remove(key);
        }
        None
    }

    /// Cache-Aside có chống Stampede: khi Cache Miss, chỉ một luồng gọi `loader`
    /// cho mỗi khóa. Trả về (giá trị, có phải chính luồng này đã nạp không).
    pub fn get_or_load<F>(&self, key: &K, ttl: Duration, loader: F) -> (V, bool)
    where
        F: FnOnce() -> V,
    {
        if let Some(v) = self.get(key) {
            return (v, false);
        }

        // Lấy (hoặc tạo) ổ khóa nạp riêng của khóa này
        let key_lock = {
            let mut loading = self.loading.lock().unwrap();
            Arc::clone(loading.entry(key.clone()).or_default())
        };
        let _guard = key_lock.lock().unwrap();

        // Kiểm tra lại (double-checked): trong lúc ta chờ, luồng khác có thể đã nạp xong
        if let Some(v) = self.get(key) {
            return (v, false);
        }

        let value = loader();
        self.set(key.clone(), value.clone(), ttl);
        self.loading.lock().unwrap().remove(key);
        (value, true)
    }

    pub fn total_entries(&self) -> usize {
        self.storage.lock().unwrap().len()
    }
}

/// Hàng đợi thông điệp an toàn đa luồng có giới hạn dung lượng (chạy trong một
/// tiến trình — Kafka/RabbitMQ/Redis Streams làm điều này qua mạng và ghi đĩa).
pub struct DistributedMessageQueue<T> {
    // VecDeque: lấy ở đầu O(1). Dùng `Vec::remove(0)` sẽ phải dịch toàn bộ phần tử: O(n).
    queue: Mutex<VecDeque<T>>,
    capacity: usize,
}

impl<T> DistributedMessageQueue<T> {
    pub fn new(capacity: usize) -> Self {
        Self {
            queue: Mutex::new(VecDeque::with_capacity(capacity)),
            capacity,
        }
    }

    /// Đẩy thông điệp vào hàng đợi (Producer). Đầy thì từ chối (backpressure).
    pub fn push(&self, item: T) -> Result<(), &'static str> {
        let mut q = self.queue.lock().unwrap();
        if q.len() >= self.capacity {
            return Err("Hàng đợi đầy (Queue is Full): Từ chối tiếp nhận thêm thông điệp!");
        }
        q.push_back(item);
        Ok(())
    }

    /// Rút thông điệp ra khỏi hàng đợi để xử lý theo thứ tự FIFO (Consumer)
    pub fn pop(&self) -> Option<T> {
        self.queue.lock().unwrap().pop_front()
    }

    pub fn len(&self) -> usize {
        self.queue.lock().unwrap().len()
    }

    pub fn is_empty(&self) -> bool {
        self.queue.lock().unwrap().is_empty()
    }
}

/// Mô phỏng mẫu thiết kế Cache-Aside truy vấn dữ liệu
pub fn fetch_user_data_cache_aside(
    cache: &SafeCacheEngine<String, String>,
    user_id: u64,
) -> (String, &'static str) {
    let key = format!("user:{}", user_id);

    let (value, loaded_from_db) = cache.get_or_load(&key, Duration::from_millis(100), || {
        // Cache Miss: Truy vấn cơ sở dữ liệu chính (giả lập)
        println!(
            "    [Database Query] Đang truy vấn từ ổ đĩa CSDL cho user_id = {}...",
            user_id
        );
        format!("UserData_#{}", user_id)
    });

    if loaded_from_db {
        (value, "CACHE_MISS (truy vấn CSDL)")
    } else {
        (value, "CACHE_HIT (đọc từ RAM)")
    }
}

fn main() {
    println!("==================================================================");
    println!("   TẦNG LƯU TRỮ ĐỆM REDIS & HÀNG ĐỢI THÔNG ĐIỆP PHÂN TÁN RUST     ");
    println!("==================================================================");

    // -------------------------------------------------------------
    // 1. THỬ NGHIỆM MÔ THỨC CACHE-ASIDE VÀ HẾT HẠN TTL
    // -------------------------------------------------------------
    println!("\n[1] Kiểm thử mô thức Cache-Aside kèm TTL Expiration:");
    let cache = SafeCacheEngine::new();

    // Lần gọi 1: Chưa có trong cache -> Cache Miss
    let (data1, source1) = fetch_user_data_cache_aside(&cache, 101);
    println!("    - Lần 1: Nhận '{}' từ nguồn: {}", data1, source1);
    assert_eq!(source1, "CACHE_MISS (truy vấn CSDL)");

    // Lần gọi 2: Đã có trong cache -> Cache Hit tức thì
    let (data2, source2) = fetch_user_data_cache_aside(&cache, 101);
    println!("    - Lần 2: Nhận '{}' từ nguồn: {}", data2, source2);
    assert_eq!(source2, "CACHE_HIT (đọc từ RAM)");
    assert_eq!(data1, data2);

    // Chờ 120ms để TTL hết hạn
    println!("    - Đang chờ 120ms để TTL hết hạn...");
    std::thread::sleep(Duration::from_millis(120));

    // Lần gọi 3: TTL đã hết hạn -> Tự động Cache Miss và nạp lại
    let (data3, source3) = fetch_user_data_cache_aside(&cache, 101);
    println!(
        "    - Lần 3 (Sau TTL): Nhận '{}' từ nguồn: {}",
        data3, source3
    );
    assert_eq!(source3, "CACHE_MISS (truy vấn CSDL)");

    // -------------------------------------------------------------
    // 2. CHỐNG CACHE STAMPEDE: 8 LUỒNG CÙNG MISS MỘT KHÓA NÓNG
    // -------------------------------------------------------------
    println!("\n[2] 8 luồng cùng đòi khóa nóng 'product:iphone' vừa hết hạn:");
    let hot_cache = Arc::new(SafeCacheEngine::<String, String>::new());
    let db_queries = Arc::new(Mutex::new(0u32));
    let workers: Vec<_> = (0..8)
        .map(|_| {
            let cache = Arc::clone(&hot_cache);
            let db_queries = Arc::clone(&db_queries);
            std::thread::spawn(move || {
                cache.get_or_load(
                    &"product:iphone".to_string(),
                    Duration::from_secs(60),
                    || {
                        *db_queries.lock().unwrap() += 1;
                        std::thread::sleep(Duration::from_millis(50)); // truy vấn chậm
                        "iPhone giảm giá".to_string()
                    },
                )
            })
        })
        .collect();
    for w in workers {
        w.join().unwrap();
    }
    let queries = *db_queries.lock().unwrap();
    println!(
        "    - Số truy vấn thực sự chạm CSDL: {} (thay vì 8)",
        queries
    );
    assert_eq!(queries, 1);

    // -------------------------------------------------------------
    // 3. THỬ NGHIỆM HÀNG ĐỢI THÔNG ĐIỆP ĐA LUỒNG PRODUCER-CONSUMER
    // -------------------------------------------------------------
    println!("\n[3] Kiểm thử Hàng đợi Thông điệp (Message Queue):");
    let message_queue = Arc::new(DistributedMessageQueue::<String>::new(5));

    // Luồng Producer: Đẩy việc vào hàng đợi
    let producer_q = Arc::clone(&message_queue);
    let producer_handle = std::thread::spawn(move || {
        for i in 1..=4 {
            let msg = format!("Đơn hàng #{}", i);
            producer_q.push(msg.clone()).unwrap();
            println!("    [Producer] Đã đẩy '{}' vào hàng đợi an toàn.", msg);
        }
    });

    producer_handle.join().unwrap();
    println!(
        "    - Số lượng thông điệp đang chờ trong hàng đợi: {}",
        message_queue.len()
    );

    // Luồng Consumer: Rút việc ra xử lý tuần tự (Worker)
    println!("\n[4] Tiến trình Worker bắt đầu rút thông điệp xử lý:");
    while let Some(task) = message_queue.pop() {
        println!("    [Consumer Worker] Đang xử lý thành công: {}", task);
    }

    assert!(message_queue.is_empty());
    println!("    => Toàn bộ hàng đợi đã được giải phóng sạch sẽ!");

    println!("\n==================================================================");
    println!("   XÁC NHẬN: TẦNG CACHE VÀ HÀNG ĐỢI HOẠT ĐỘNG ĐÚNG ĐA LUỒNG!     ");
    println!("==================================================================");
}

#[cfg(test)]
mod tests {
    use super::*;
    use std::sync::atomic::{AtomicU32, Ordering};

    #[test]
    fn stampede_loads_once() {
        let cache = Arc::new(SafeCacheEngine::<u32, u32>::new());
        let calls = Arc::new(AtomicU32::new(0));
        let handles: Vec<_> = (0..16)
            .map(|_| {
                let cache = Arc::clone(&cache);
                let calls = Arc::clone(&calls);
                std::thread::spawn(move || {
                    cache
                        .get_or_load(&7, Duration::from_secs(10), || {
                            calls.fetch_add(1, Ordering::SeqCst);
                            std::thread::sleep(Duration::from_millis(30));
                            42
                        })
                        .0
                })
            })
            .collect();
        for h in handles {
            assert_eq!(h.join().unwrap(), 42);
        }
        assert_eq!(calls.load(Ordering::SeqCst), 1);
    }

    #[test]
    fn expired_entry_is_removed() {
        let cache = SafeCacheEngine::new();
        cache.set("k", 1, Duration::from_millis(0));
        assert_eq!(cache.get(&"k"), None);
        assert_eq!(cache.total_entries(), 0);
    }

    #[test]
    fn queue_is_fifo_and_bounded() {
        let q = DistributedMessageQueue::new(2);
        q.push(1).unwrap();
        q.push(2).unwrap();
        assert!(q.push(3).is_err());
        assert_eq!(q.pop(), Some(1));
        assert_eq!(q.pop(), Some(2));
        assert_eq!(q.pop(), None);
    }
}
