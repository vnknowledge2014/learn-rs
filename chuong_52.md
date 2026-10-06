# Chương 52: Tầng lưu trữ đệm phân tán Redis & Hàng đợi thông điệp (Distributed Caching with Redis & Message Queuing)

## Giới thiệu & Mục tiêu học tập

Trong các hệ thống phân tán quy mô lớn, hai cơn ác mộng lớn nhất mà mọi kỹ sư kiến trúc phải đối mặt là: **Nghẽn cổ chai cơ sở dữ liệu (Database Bottleneck)** và **Sập nguồn do quá tải đỉnh (Traffic Spike Overload)**. 

Một cơ sở dữ liệu quan hệ (như PostgreSQL hay MySQL) dù được tối ưu hóa đến đâu cũng có một trần thông lượng (thường cỡ vài nghìn đến vài chục nghìn giao dịch ghi/giây tùy phần cứng và khối lượng) trước khi đĩa cứng và khóa giao dịch bị nghẽn. Để giải cứu cơ sở dữ liệu và giữ cho hệ thống luôn phản hồi trong vài mili-giây khi có hàng triệu người dùng cùng lúc, chúng ta cần hai trụ cột phòng thủ vững chắc:
1. **Tầng lưu trữ đệm phân tán (Distributed Caching với Redis)**: Đưa dữ liệu nóng (Hot Data) lên thanh RAM để phục vụ các yêu cầu đọc với độ trễ dưới 1 mili-giây.
2. **Hàng đợi thông điệp phân tán (Message Queuing / Event Streams)**: Đóng vai trò "đập thủy điện" san phẳng các đợt sóng tải đột biến (Traffic Smoothing), tách rời các dịch vụ (Decoupling) và — khi được cấu hình ghi bền vững kèm xác nhận (ack) — giữ cho công việc không bị rơi rớt khi một tiến trình xử lý gặp sự cố.

Mục tiêu học tập của bạn:
- Nắm vững các mô thức bộ đệm kinh điển: **Cache-Aside**, **Write-Through**, và **Write-Behind**.
- Mổ xẻ và khắc chế "Tam đại hiểm họa Bộ đệm": **Cache Stampede** (Đàn bò giẫm đạp), **Cache Penetration** (Thủng đệm), và **Cache Avalanche** (Tuyết lở bộ đệm).
- Hiểu sâu sắc kiến trúc Hàng đợi thông điệp: Mô hình Nhà sản xuất - Người tiêu thụ (Producer-Consumer), Cơ chế xác nhận hoàn tất (Ack / Nack), và Hàng đợi thư chết (Dead-Letter Queue - DLQ).
- Tự tay lập trình một hệ thống Lưu trữ đệm kèm Hàng đợi sự kiện phân tán bằng Rust chuẩn mực, an toàn đa luồng, có chống Cache Stampede.

---

## Hình tượng hóa đời sống (Intuitive Everyday Analogy)

Hãy cùng quan sát hai câu chuyện đời thường để hiểu rõ sức mạnh giải cứu của Caching và Message Queue:

```
┌──────────────────────────────────────────────────────────────────────────────────┐
│         HÌNH TƯỢNG HÓA: TỦ LẠNH GIA ĐÌNH VS HÀNG RÀO XẾP HÀNG LÀM HỘ CHIẾU       │
├──────────────────────────────────────────────────────────────────────────────────┤
│                                                                                  │
│ [1. TỦ LẠNH GIA ĐÌNH (CACHE-ASIDE) VS SIÊU THỊ ĐẦU MỐI (DATABASE)]               │
│ ┌──────────────────────────────────────────────────────────────────────┐         │
│ │ Bạn khát nước ──► Mở tủ lạnh ngay trong bếp (Cache Hit: Tốn 2 giây)! │         │
│ ├──────────────────────────────────────────────────────────────────────┤         │
│ │ Tủ lạnh hết nước ngọt (Cache Miss):                                  │         │
│ │ 1. Bạn lấy xe máy chạy ra Siêu thị cách 3km mua nước (Truy vấn DB).  │         │
│ │ 2. Uống 1 lon giải khát.                                             │         │
│ │ 3. Tiện tay cất ngay 2 lon vào tủ lạnh kèm nhãn hạn dùng (Lưu Cache)!│         │
│ └──────────────────────────────────────────────────────────────────────┘         │
│   ===> 99% số lần uống nước bạn chỉ tốn 2 giây mở tủ lạnh ở nhà!                 │
│                                                                                  │
│ [2. HÀNG RÀO DÍCH DẮC CẤP SỐ (MESSAGE QUEUE) TRƯỚC PHÒNG QUẢN LÝ XUẤT NHẬP CẢNH] │
│ ┌──────────────────────────────────────────────────────────────────────┐         │
│ │ Nếu 2,000 người cùng lúc ào vào phòng làm việc của 3 cán bộ công an: │         │
│ │   Phòng sẽ vỡ trận, giẫm đạp, tài liệu bay tứ tung (SẬP MÁY CHỦ)!   │         │
│ ├──────────────────────────────────────────────────────────────────────┤         │
│ │ Giải pháp Hàng đợi Message Queue:                                    │         │
│ │ 1. Người dân đến nơi được phát số thứ tự, đứng xếp hàng trật tự ngoài│         │
│ │    sân (Đẩy tác vụ vào Queue an toàn).                               │         │
│ │ 2. Cán bộ bên trong ung dung bấm chuông gọi từng người vào làm việc  │         │
│ │    theo đúng tốc độ xử lý ổn định của mình (Consumer pull).          │         │
│ └──────────────────────────────────────────────────────────────────────┘         │
│   ===> DÙ BÊN NGOÀI ĐÔNG ĐẾN ĐÂU, BÊN TRONG VẪN VẬN HÀNH ÊM ÁI HOÀN HẢO!         │
└──────────────────────────────────────────────────────────────────────────────────┘
```

### 1. Tủ lạnh gia đình (Distributed Caching)
- Cơ sở dữ liệu chính (PostgreSQL) giống như siêu thị Metro cách nhà bạn 3km: Rất to lớn, chứa được hàng triệu món đồ, nhưng muốn lấy món gì bạn phải nổ xe máy chạy đi mua, gửi xe, xếp hàng thanh toán (tốn thời gian I/O đĩa cứng).
- Bộ nhớ Cache (Redis) giống như chiếc tủ lạnh mini đặt ngay cạnh bàn làm việc của bạn: Nó không thể chứa cả siêu thị, nhưng nó chứa những lon nước bạn hay uống nhất trong ngày. 99% các lần khát nước bạn mở tủ lạnh lấy uống ngay trong nháy mắt.

### 2. Hàng rào dích dắc ngoài sân (Message Queuing)
- Khi có chương trình "Săn vé máy bay 0 đồng", 100,000 người cùng bấm nút "Đặt vé" trong 1 giây. Nếu gửi thẳng 100,000 giao dịch này vào Database, máy chủ sẽ bốc khói và sập ngay lập tức.
- Hàng đợi Message Queue (như Kafka, RabbitMQ, hay Redis Streams) đóng vai trò như chiếc rào chắn: Mỗi yêu cầu chỉ cần được ghi nhận vào hàng đợi (thao tác rẻ hơn rất nhiều so với một giao dịch CSDL) rồi báo ngay cho khách hàng: *"Yêu cầu của bạn đã được tiếp nhận, vui lòng chờ xử lý"*.
- Đằng sau hàng rào, một đội ngũ gồm 10 tiến trình công nhân (Workers / Consumers) cần mẫn rút từng đơn hàng ra xử lý theo tốc độ của mình: khi tải dồn, hàng đợi dài ra thay vì CSDL sập.

---

## Khái niệm & Cơ chế kỹ thuật chuyên sâu (In-Depth Technical Mechanics)

### 1. Ba Mô thức Thiết kế Bộ đệm (Caching Patterns)

1. **Cache-Aside (Đọc lười - Lazy Loading)**:
   - Ứng dụng kiểm tra trong Cache trước:
     - Nếu có (**Cache Hit**): Trả về dữ liệu ngay lập tức.
     - Nếu không có (**Cache Miss**): Ứng dụng đọc từ Database -> Ghi ngược lại vào Cache kèm thời gian hết hạn (`TTL - Time to Live`) -> Trả về kết quả.
   - **Ưu điểm**: Chỉ những dữ liệu thực sự được người dùng yêu cầu mới chiếm bộ nhớ RAM.
2. **Write-Through (Ghi đồng thời)**:
   - Khi có dữ liệu mới, ứng dụng ghi đồng thời vào Cache và Database trước khi trả về thành công. Đảm bảo dữ liệu trong Cache luôn mới nhất, nhưng tăng độ trễ ghi.
3. **Write-Behind / Write-Back (Ghi trì hoãn)**:
   - Ứng dụng ghi thẳng vào Cache siêu tốc rồi trả về thành công ngay. Một tiến trình nền sau đó sẽ gom các bản ghi và ghi xuống Database theo lô (Batch). Tốc độ ghi cực nhanh nhưng có rủi ro mất dữ liệu nếu máy chủ Cache mất điện đột ngột.

### 2. Tam Đại Hiểm Họa Bộ đệm & Biện pháp Hóa giải

```
[Hiểm họa 1: Cache Stampede]     ──► Giải pháp: Khóa Mutex phân tán / Gia hạn sớm
[Hiểm họa 2: Cache Penetration]    ──► Giải pháp: Bộ lọc Bloom Filter / Cache giá trị rỗng
[Hiểm họa 3: Cache Avalanche]      ──► Giải pháp: Thêm độ lệch ngẫu nhiên (TTL Jitter)
```

1. **Đàn bò giẫm đạp (Cache Stampede / Thundering Herd)**:
   - Xảy ra khi một khóa "cực nóng" (ví dụ thông tin sản phẩm iPhone mới giảm giá) vừa hết hạn TTL.
   - Ngay trong giây đó, 50,000 yêu cầu cùng lúc nhận thấy Cache Miss và cùng lúc xông thẳng vào Database để truy vấn, làm Database sập nguồn ngay tức khắc.
   - **Hóa giải**: Khi bị Cache Miss, chỉ cho phép duy nhất 1 luồng được cấp khóa đi truy vấn Database, các luồng còn lại phải chờ luồng này nạp lại Cache.
2. **Thủng bộ đệm (Cache Penetration)**:
   - Kẻ tấn công cố tình gửi liên tục hàng triệu yêu cầu truy vấn các ID không hề tồn tại (ví dụ `user_id = -999999`).
   - Cache không có dữ liệu này -> Yêu cầu xuyên thủng qua Cache lao thẳng vào Database.
   - **Hóa giải**: Lưu cả giá trị rỗng (`None`) vào Cache với TTL ngắn (30 giây), hoặc sử dụng cấu trúc dữ liệu xác suất **Bộ lọc Bloom (Bloom Filter)** ở cổng vào để từ chối ngay lập tức các khóa không tồn tại.
3. **Tuyết lở bộ đệm (Cache Avalanche)**:
   - Do lập trình viên đặt cùng một mốc thời gian hết hạn cố định (ví dụ tất cả các khóa đều có `TTL = 3600 giây`). Đúng 1 tiếng sau, toàn bộ dữ liệu trong Cache đồng loạt bốc hơi, dồn toàn bộ tải sang Database.
   - **Hóa giải**: Luôn bổ sung một khoảng thời gian ngẫu nhiên (Jitter) vào TTL, ví dụ: `TTL = 3600 + rand(1..300) giây`.

---

## Mã nguồn minh họa thực chiến (Idiomatic Runnable Rust Blueprint)

Dưới đây là mã nguồn Rust hoàn chỉnh hiện thực hóa một **Tầng lưu trữ đệm kèm Hàng đợi thông điệp phân tán (In-Memory Cache-Aside & Message Queue)**: Tự tay cài đặt cơ chế hết hạn TTL, hàng đợi Producer-Consumer an toàn đa luồng (dùng `VecDeque` để lấy đầu hàng O(1) — nhớ bài học Chương 28: `Vec::remove(0)` phải dịch toàn bộ phần tử), và cơ chế phòng chống Cache Stampede kiểu "single-flight" (`get_or_load`: mỗi khóa chỉ một luồng đi truy vấn nguồn, các luồng khác chờ rồi đọc lại Cache). Đào thải LRU khi đầy dung lượng được để dành cho Bài tập 1:

```rust
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
```

---

## Bảng tra cứu lỗi biên dịch & Cách khắc phục (Compiler Error Guide)

Dưới đây là các lỗi biên dịch thường gặp nhất khi triển khai bộ đệm Cache và hàng đợi thông điệp trong Rust:

| Mã lỗi | Thông báo mẫu từ trình biên dịch | Nguyên nhân cốt lõi | Cách khắc phục nhanh |
|---|---|---|---|
| **E0502** | `cannot borrow '*self' as mutable because it is also borrowed as immutable` | Bạn vừa đọc dữ liệu trong `HashMap` bộ đệm vừa cố gắng xóa một mục hết hạn. | Sao chép giá trị cần thiết ra ngoài trước khi thực hiện thao tác xóa, hoặc phân tách phạm vi mượn. |
| **E0382** | `` borrow of moved value: `message_queue` `` | Di chuyển quyền sở hữu (ownership) của hàng đợi vào luồng con mà quên bọc trong con trỏ đếm tham chiếu `Arc`. | Bọc cấu trúc trong `Arc::new(...)` và tạo bản sao `Arc::clone(&queue)` cho mỗi luồng. |
| **E0277** | `` the trait bound `MyKey: Eq` is not satisfied `` (và `MyKey: Hash`) | Khóa của bảng băm `HashMap` bắt buộc phải triển khai trait `Hash` và `Eq`. | Bổ sung derive tự động: `#[derive(Hash, PartialEq, Eq, Clone)]` lên trên kiểu khóa. |
| **E0596** | `` cannot borrow data in an `Arc` as mutable `` | Gọi phương thức nhận `&mut self` (ví dụ `fn pop(&mut self)`) qua con trỏ thông minh `Arc`: `Arc` tự Deref ra `&T` nhưng không bao giờ cho `&mut T` khi đang chia sẻ. | Cho phương thức nhận `&self` và đặt dữ liệu trong `Mutex`/`RwLock` bên trong (như `DistributedMessageQueue`), hoặc bọc cả struct: `Arc<Mutex<T>>`. |

### Ví dụ phân tích lỗi `E0502` khi vừa duyệt vừa xóa mục Cache hết hạn:

```rust
use std::collections::HashMap;

// Đoạn mã lỗi minh họa E0502:
fn delete_broken(map: &mut HashMap<String, u64>) {
    // for (k, &v) in map.iter() {
    //     if v == 0 {
    //         map.remove(k); // LỖI E0502: Không thể sửa map khi đang mượn bất biến để duyệt!
    //     }
    // }
}

// Cách sửa chữa đúng chuẩn: Thu thập danh sách khóa cần xóa trước
fn delete_correct(map: &mut HashMap<String, u64>) {
    let expired_keys: Vec<String> = map
        .iter()
        .filter(|&(_, &v)| v == 0)
        .map(|(k, _)| k.clone())
        .collect();

    for k in expired_keys {
        map.remove(&k);
    }
}
```

---

## Tóm tắt chương & Bài tập rèn luyện (Summary & Exercises)

### 4 Điểm cốt lõi cần ghi nhớ:
1. **Bảo vệ Cơ sở dữ liệu**: Tầng lưu trữ đệm phân tán Redis giải cứu Database khỏi nghẽn cổ chai, giảm độ trễ đọc dữ liệu nóng từ hàng chục mili-giây xuống cỡ dưới 1 mili-giây.
2. **Khắc chế 3 Hiểm họa Cache**: Chặn Cache Stampede bằng khóa nạp (single-flight / khóa phân tán), chống Cache Penetration bằng Bloom Filter, và chống Cache Avalanche bằng khoảng lệch thời gian ngẫu nhiên (TTL Jitter).
3. **Sức mạnh của Hàng đợi Thông điệp**: Đóng vai trò đập thủy điện san phẳng các đợt bùng nổ lưu lượng, tách rời các dịch vụ và bảo đảm độ tin cậy của luồng xử lý.
4. **An toàn Đa luồng Không Rò rỉ**: Vận dụng chuẩn mực quyền sở hữu (ownership), mượn (borrow), thời gian sống (lifetime), con trỏ thông minh (smart pointer) và bộ nhớ đệm (buffer) để bảo đảm các tiến trình đọc/ghi song song luôn đạt thông lượng cao nhất.

### Bài tập rèn luyện tự giải:
1. **Bài tập 1 (Bổ sung Thuật toán Đào thải trang LRU vào Cache)**:  
   Mở rộng `SafeCacheEngine`: Khi bộ nhớ đệm đạt tới giới hạn dung lượng tối đa (ví dụ 1,000 mục), hãy tự động tìm và xóa mục có thời gian truy cập lâu nhất (Least Recently Used) để nhường chỗ cho mục mới.
2. **Bài tập 2 (Xây dựng Hàng đợi Thư Chết - Dead-Letter Queue)**:  
   Trong `DistributedMessageQueue`, nếu một thông điệp bị xử lý thất bại quá 3 lần liên tiếp, thay vì vứt bỏ, hãy tự động chuyển thông điệp đó sang một hàng đợi riêng biệt mang tên `DeadLetterQueue` để các kỹ sư quản trị có thể kiểm tra và gỡ lỗi thủ công.
3. **Bài tập 3 (Suy ngẫm kiến trúc: Tại sao Cache Invalidation là một trong hai bài toán khó nhất?)**:  
   Phil Karlton từng nói (Martin Fowler trích lại và làm nó nổi tiếng): *"Chỉ có hai thứ khó trong khoa học máy tính: Hủy tính hợp lệ của Cache (Cache Invalidation) và Đặt tên"*. Hãy phân tích một tình huống cụ thể: Khi người dùng đổi mật khẩu, làm thế nào để đảm bảo 10 máy chủ Cache phân tán trên toàn cầu cùng hủy bỏ phiên đăng nhập cũ ngay lập tức mà không để xảy ra kẽ hở bảo mật?

---

### Gợi ý & Lời giải

<details>
<summary><b>Bài tập 1 — Gợi ý</b></summary>

LRU đào thải mục 'lâu nhất chưa dùng'. Cần theo dõi thứ tự truy cập: mỗi lần get/set một khóa, đánh dấu nó vừa được dùng. Khi đầy, bỏ mục có dấu thời gian cũ nhất.
</details>

<details>
<summary><b>Bài tập 1 — Lời giải</b></summary>

```rust
use std::collections::HashMap;

/// Cache có đào thải LRU (Least Recently Used): đầy thì bỏ mục lâu nhất chưa dùng.
pub struct LruCache<K: std::hash::Hash + Eq + Clone, V: Clone> {
    map: HashMap<K, (V, u64)>, // khóa -> (giá trị, dấu thời gian truy cập gần nhất)
    capacity: usize,
    clock: u64,                // bộ đếm logic, tăng mỗi lần truy cập
}

impl<K: std::hash::Hash + Eq + Clone, V: Clone> LruCache<K, V> {
    pub fn new(capacity: usize) -> Self {
        Self { map: HashMap::new(), capacity, clock: 0 }
    }

    fn tick(&mut self) -> u64 { self.clock += 1; self.clock }

    pub fn get(&mut self, key: &K) -> Option<V> {
        let now = self.tick();
        if let Some(e) = self.map.get_mut(key) {
            e.1 = now;              // đánh dấu VỪA DÙNG -> lùi thời điểm bị đào thải
            Some(e.0.clone())
        } else { None }
    }

    pub fn set(&mut self, key: K, value: V) {
        let now = self.tick();
        if !self.map.contains_key(&key) && self.map.len() >= self.capacity {
            // Đầy chỗ -> tìm khóa có dấu thời gian NHỎ NHẤT (lâu nhất chưa dùng) và bỏ.
            if let Some(lru_key) = self.map.iter()
                .min_by_key(|(_, (_, ts))| *ts)
                .map(|(k, _)| k.clone())
            {
                self.map.remove(&lru_key);
            }
        }
        self.map.insert(key, (value, now));
    }

    pub fn len(&self) -> usize { self.map.len() }
}

#[test]
fn evicts_least_recently_used() {
    let mut c = LruCache::new(2);
    c.set("a", 1);
    c.set("b", 2);
    // Truy cập "a" -> "a" thành mới dùng, "b" thành cũ nhất.
    assert_eq!(c.get(&"a"), Some(1));
    // Thêm "c" khi đã đầy -> "b" (cũ nhất chưa dùng) bị đào thải, KHÔNG phải "a".
    c.set("c", 3);
    assert_eq!(c.len(), 2);
    assert_eq!(c.get(&"b"), None);      // b đã bị bỏ
    assert_eq!(c.get(&"a"), Some(1));   // a còn vì vừa được dùng
    assert_eq!(c.get(&"c"), Some(3));
}
```

Ý tưởng LRU: **thứ lâu nhất không được đụng tới là thứ đáng bỏ nhất** — vì theo *tính cục bộ thời gian* (temporal locality), dữ liệu vừa dùng có xác suất cao sẽ được dùng lại sớm. Mấu chốt là mỗi lần `get` cũng phải **cập nhật dấu thời gian** (không chỉ `set`), nếu không "vừa dùng" sẽ không được ghi nhận và bạn có thể đào thải nhầm mục đang nóng. Bản ở đây dùng quét tuyến tính tìm mục cũ nhất (`O(n)` mỗi lần đào thải) cho dễ hiểu; cache thật dùng danh sách liên kết đôi + HashMap để đào thải trong `O(1)` — đó chính là cấu trúc bên trong `lru` crate hay `LinkedHashMap`.
</details>

<details>
<summary><b>Bài tập 2 — Gợi ý</b></summary>

Hàng đợi thư chết (Dead-Letter Queue): thông điệp thất bại quá 3 lần không vứt đi mà chuyển sang một hàng riêng để người quản trị kiểm tra. Cần theo dõi số lần thử mỗi thông điệp.
</details>

<details>
<summary><b>Bài tập 2 — Lời giải</b></summary>

```rust
use std::collections::VecDeque;

/// Thông điệp kèm bộ đếm số lần đã thử xử lý.
pub struct Message { pub payload: String, pub attempts: u32 }

/// Hàng đợi có DLQ: thất bại đủ 3 lần thì chuyển sang hàng thư chết thay vì vứt bỏ.
pub struct QueueWithDlq {
    main: VecDeque<Message>,
    pub dead_letter: Vec<Message>, // nơi kỹ sư kiểm tra thủ công về sau
}

impl QueueWithDlq {
    pub fn new() -> Self { Self { main: VecDeque::new(), dead_letter: Vec::new() } }

    pub fn push(&mut self, payload: &str) {
        self.main.push_back(Message { payload: payload.to_string(), attempts: 0 });
    }

    /// Xử lý thông điệp kế tiếp. `succeeded` mô phỏng kết quả xử lý.
    /// Nếu thất bại: tăng bộ đếm, đưa lại hàng chính; đủ 3 lần -> chuyển DLQ.
    pub fn process_next(&mut self, succeeded: bool) {
        if let Some(mut msg) = self.main.pop_front() {
            if succeeded { return; } // xử lý xong, biến mất khỏi hàng
            msg.attempts += 1;
            if msg.attempts >= 3 {
                // Thất bại 3 lần liên tiếp -> KHÔNG vứt, chuyển sang hàng thư chết.
                self.dead_letter.push(msg);
            } else {
                self.main.push_back(msg); // thử lại sau
            }
        }
    }

    pub fn main_len(&self) -> usize { self.main.len() }
}

#[test]
fn moves_to_dlq_after_3_failures() {
    let mut q = QueueWithDlq::new();
    q.push("gửi email lỗi");
    // Ba lần xử lý đều thất bại.
    q.process_next(false); // lần 1 -> đưa lại hàng
    q.process_next(false); // lần 2 -> đưa lại hàng
    q.process_next(false); // lần 3 -> chuyển DLQ
    assert_eq!(q.main_len(), 0);              // không còn ở hàng chính
    assert_eq!(q.dead_letter.len(), 1);       // đã vào hàng thư chết
    assert_eq!(q.dead_letter[0].attempts, 3);
}
```

Hàng đợi thư chết giải quyết một tình thế tiến thoái lưỡng nan: một thông điệp *cứ thất bại mãi* (dữ liệu dị dạng, dịch vụ đích hỏng vĩnh viễn) thì làm gì? **Thử lại vô hạn** làm nghẽn hàng đợi, chặn mọi thông điệp tốt phía sau — gọi là "thông điệp độc" (poison message). **Vứt bỏ luôn** thì mất dữ liệu âm thầm, không ai biết. DLQ là lối thoát thứ ba: sau vài lần thử, **tách riêng thông điệp hỏng ra một chỗ an toàn** để hàng chính chảy tiếp, đồng thời *giữ lại* thông điệp đó cho kỹ sư điều tra nguyên nhân. Đây là thành phần chuẩn của mọi hệ thống hàng đợi nghiêm túc (RabbitMQ, AWS SQS, Kafka) — một van an toàn giữa "thử mãi" và "mất dữ liệu".
</details>

<details>
<summary><b>Bài tập 3 — Gợi ý</b></summary>

Tình huống đổi mật khẩu: phiên cũ trên 10 cache toàn cầu phải bị vô hiệu NGAY, nếu không kẻ trộm mật khẩu cũ vẫn vào được. Vấn đề là độ trễ lan truyền giữa các cache.
</details>

<details>
<summary><b>Bài tập 3 — Lời giải</b></summary>

**Vì sao hủy hiệu lực cache (cache invalidation) khó — qua tình huống đổi mật khẩu:**

Khi người dùng đổi mật khẩu, mục đích an ninh là: **mọi phiên đăng nhập cũ phải chết ngay lập tức** — để nếu kẻ tấn công đã đánh cắp mật khẩu cũ (hoặc token phiên cũ), chúng bị đá ra tức thì. Nhưng phiên được lưu đệm trên **10 máy chủ cache rải khắp toàn cầu** để đăng nhập nhanh. Đó là nơi cái khó lộ ra.

**Kẽ hở bảo mật cốt lõi — cửa sổ không nhất quán:**
```text
t=0:   Người dùng đổi mật khẩu ở máy chủ khu vực Mỹ.
t=0:   Cache Mỹ hủy phiên cũ ngay.       -> Mỹ: phiên cũ đã chết
t=0:   Nhưng lệnh hủy còn ĐANG LAN sang cache Âu, Á (mất 50-500ms).
t=0.2: Kẻ tấn công dùng token phiên CŨ, kết nối vào cache CHÂU Á.
       -> Châu Á CHƯA nhận lệnh hủy -> phiên cũ vẫn còn hiệu lực
       -> KẺ TẤN CÔNG VÀO ĐƯỢC trong cửa sổ 0.2 giây đó
```
Chính khoảng trễ lan truyền giữa các cache tạo ra một **cửa sổ tấn công**. Với dữ liệu thường (tên hiển thị cũ vài trăm mili-giây), cửa sổ này vô hại. Với **phiên đăng nhập**, nó là lỗ hổng thật.

**Các hướng xử lý — và đánh đổi của từng hướng:**

| Chiến lược | Cách làm | Đánh đổi |
|---|---|---|
| **Hủy chủ động (push/fan-out)** | Máy đổi mật khẩu chủ động phát lệnh xóa tới cả 10 cache và *chờ xác nhận* | Chậm hơn (chờ nơi xa nhất), nhưng đóng cửa sổ |
| **TTL ngắn** | Phiên trong cache tự hết hạn sau 30-60 giây, buộc kiểm lại nguồn thật | Đơn giản, nhưng vẫn có cửa sổ = TTL; tăng tải nguồn thật |
| **Danh sách thu hồi tập trung** | Mỗi yêu cầu kiểm nhanh một "sổ đen" phiên bị thu hồi (nguồn duy nhất, sao chép nhanh) | Thêm một lần tra mỗi yêu cầu; nhưng nhất quán tức thì |
| **Cặp token + phiên bản mật khẩu** | Token nhúng "phiên bản mật khẩu"; đổi mật khẩu tăng phiên bản -> mọi token cũ tự sai | Sạch, nhưng cần thiết kế token từ đầu |

Trong thực tế, hệ thống nghiêm túc **kết hợp**: TTL ngắn cho phiên + một sổ đen thu hồi tập trung cho các sự kiện nhạy cảm (đổi mật khẩu, đăng xuất). Câu nói của Phil Karlton đúng ở chỗ: **không có lời giải hoàn hảo** — mọi phương án đều đánh đổi giữa *tốc độ* (đệm để nhanh), *tính nhất quán* (dữ liệu mới nhất ở mọi nơi), và *tải hệ thống*. Chọn điểm cân bằng nào là tùy dữ liệu: "hơi cũ" chấp nhận được thì ưu tiên tốc độ; dữ liệu an ninh thì phải trả giá tốc độ để đóng cửa sổ tấn công.
</details>
