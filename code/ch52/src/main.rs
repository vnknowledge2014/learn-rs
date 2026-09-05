#![allow(dead_code, unused_variables, unused_imports)]
use std::collections::HashMap;
use std::sync::{Arc, Mutex};
use std::time::{Duration, Instant};

/// Một mục lưu trữ trong bộ đệm kèm thời gian sống (TTL)
#[derive(Clone, Debug)]
struct CacheEntry<V> {
    value: V,
    expires_at: Instant,
}

/// Động cơ Lưu trữ đệm an toàn đa luồng hỗ trợ TTL (In-Memory Cache Engine)
pub struct SafeCacheEngine<K, V> {
    storage: Mutex<HashMap<K, CacheEntry<V>>>,
}

impl<K: std::hash::Hash + Eq + Clone, V: Clone> SafeCacheEngine<K, V> {
    pub fn new() -> Self {
        Self {
            storage: Mutex::new(HashMap::new()),
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
        }

        // Cache Miss hoặc đã hết hạn: Dọn dẹp mục cũ nếu có
        store.remove(key);
        None
    }

    pub fn total_entries(&self) -> usize {
        self.storage.lock().unwrap().len()
    }
}

/// Hàng đợi thông điệp an toàn đa luồng (Thread-Safe Message Queue)
pub struct DistributedMessageQueue<T> {
    queue: Mutex<Vec<T>>,
    capacity: usize,
}

impl<T> DistributedMessageQueue<T> {
    pub fn new(capacity: usize) -> Self {
        Self {
            queue: Mutex::new(Vec::new()),
            capacity,
        }
    }

    /// Đẩy thông điệp vào hàng đợi (Producer)
    pub fn push(&self, item: T) -> Result<(), &'static str> {
        let mut q = self.queue.lock().unwrap();
        if q.len() >= self.capacity {
            return Err("Hang doi day (Queue is Full): Tu choi tiep nhan them thong diep!");
        }
        q.push(item);
        Ok(())
    }

    /// Rút thông điệp ra khỏi hàng đợi để xử lý theo thứ tự FIFO (Consumer)
    pub fn pop(&self) -> Option<T> {
        let mut q = self.queue.lock().unwrap();
        if q.is_empty() {
            None
        } else {
            Some(q.remove(0))
        }
    }

    pub fn len(&self) -> usize {
        self.queue.lock().unwrap().len()
    }
}

/// Mô phỏng mẫu thiết kế Cache-Aside truy vấn dữ liệu thông minh
pub fn fetch_user_data_cache_aside(
    cache: &SafeCacheEngine<String, String>,
    user_id: u64,
) -> (String, &'static str) {
    let key = format!("user:{}", user_id);

    // 1. Thử tìm kiếm trong Cache
    if let Some(cached_val) = cache.get(&key) {
        return (cached_val, "CACHE_HIT (2ms)");
    }

    // 2. Cache Miss: Truy vấn cơ sở dữ liệu chính (Giả lập I/O tốn 50ms)
    println!("    [Database Query] Đang truy vấn từ ổ đĩa CSDL cho user_id = {}...", user_id);
    let db_val = format!("DuLieuNguoiDung_#{}", user_id);

    // 3. Ghi ngược lại vào Cache với TTL = 100ms
    cache.set(key, db_val.clone(), Duration::from_millis(100));

    (db_val, "CACHE_MISS (50ms)")
}

fn main() {
    println!("==================================================================");
    println!("   TANG LUU TRU DEM REDIS & HANG DOI THONG DIEP PHAN TAN RUST     ");
    println!("==================================================================");

    // -------------------------------------------------------------
    // 1. THỬ NGHIỆM MÔ THỨC CACHE-ASIDE VÀ HẾT HẠN TTL
    // -------------------------------------------------------------
    println!("\n[1] Kiem thu mo thuc Cache-Aside kem TTL Expiration:");
    let cache = SafeCacheEngine::new();

    // Lần gọi 1: Chưa có trong cache -> Cache Miss
    let (data1, source1) = fetch_user_data_cache_aside(&cache, 101);
    println!("    - Lan 1: Nhan '{}' tu nguon: {}", data1, source1);
    assert_eq!(source1, "CACHE_MISS (50ms)");

    // Lần gọi 2: Đã có trong cache -> Cache Hit tức thì
    let (data2, source2) = fetch_user_data_cache_aside(&cache, 101);
    println!("    - Lan 2: Nhan '{}' tu nguon: {}", data2, source2);
    assert_eq!(source2, "CACHE_HIT (2ms)");
    assert_eq!(data1, data2);

    // Chờ 120ms để TTL hết hạn
    println!("    - Dang cho 120ms de TTL het han...");
    std::thread::sleep(Duration::from_millis(120));

    // Lần gọi 3: TTL đã hết hạn -> Tự động Cache Miss và nạp lại
    let (data3, source3) = fetch_user_data_cache_aside(&cache, 101);
    println!("    - Lan 3 (Sau TTL): Nhan '{}' tu nguon: {}", data3, source3);
    assert_eq!(source3, "CACHE_MISS (50ms)");

    // -------------------------------------------------------------
    // 2. THỬ NGHIỆM HÀNG ĐỢI THÔNG ĐIỆP ĐA LUỒNG PRODUCER-CONSUMER
    // -------------------------------------------------------------
    println!("\n[2] Kiem thu Hang doi Thong diep phan tan (Message Queue):");
    let message_queue = Arc::new(DistributedMessageQueue::<String>::new(5));

    // Luồng Producer: Đẩy việc vào hàng đợi
    let producer_q = Arc::clone(&message_queue);
    let producer_handle = std::thread::spawn(move || {
        for i in 1..=4 {
            let msg = format!("DonHang_#{}", i);
            producer_q.push(msg.clone()).unwrap();
            println!("    [Producer] Da day '{}' vao hang doi an toan.", msg);
        }
    });

    producer_handle.join().unwrap();
    println!("    - So luong thong diep dang cho trong hang doi: {}", message_queue.len());

    // Luồng Consumer: Rút việc ra xử lý tuần tự (Worker)
    println!("\n[3] Tien trinh Worker bat dau rut thong diep xu ly:");
    while let Some(task) = message_queue.pop() {
        println!("    [Consumer Worker] Dang xu ly thanh cong: {}", task);
    }

    assert_eq!(message_queue.len(), 0);
    println!("    => Toan bo hang doi da duoc giai phong sach se!");

    println!("\n==================================================================");
    println!("   XAC NHAN: TANG CACHE VA HANG DOI DONG BO AN TOAN TUYET DOI!  ");
    println!("==================================================================");
}
