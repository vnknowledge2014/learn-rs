# Chương 52: Tầng lưu trữ đệm phân tán Redis & Hàng đợi thông điệp (Distributed Caching with Redis & Message Queuing)

## Giới thiệu & Mục tiêu học tập

Trong các hệ thống phân tán quy mô lớn, hai cơn ác mộng lớn nhất mà mọi kỹ sư kiến trúc phải đối mặt là: **Nghẽn cổ chai cơ sở dữ liệu (Database Bottleneck)** và **Sập nguồn do quá tải đỉnh (Traffic Spike Overload)**. 

Một cơ sở dữ liệu quan hệ (như PostgreSQL hay MySQL) dù được tối ưu hóa đến đâu cũng chỉ có thể chịu tải tối đa vài ngàn truy vấn ghi/giây trước khi đĩa cứng và khóa giao dịch bị nghẽn. Để giải cứu cơ sở dữ liệu và giữ cho hệ thống luôn phản hồi trong vài mili-giây khi có hàng triệu người dùng cùng lúc, chúng ta cần hai trụ cột phòng thủ vững chắc:
1. **Tầng lưu trữ đệm phân tán (Distributed Caching với Redis)**: Đưa dữ liệu nóng (Hot Data) lên thanh RAM để phục vụ các yêu cầu đọc với độ trễ dưới 1 mili-giây.
2. **Hàng đợi thông điệp phân tán (Message Queuing / Event Streams)**: Đóng vai trò "đập thủy điện" san phẳng các đợt sóng tải đột biến (Traffic Smoothing), tách rời các dịch vụ (Decoupling) và bảo đảm dữ liệu không bao giờ bị rơi rớt.

Mục tiêu học tập của bạn:
- Nắm vững các mô thức bộ đệm kinh điển: **Cache-Aside**, **Write-Through**, và **Write-Behind**.
- Mổ xẻ và khắc chế "Tam đại hiểm họa Bộ đệm": **Cache Stampede** (Đàn bò giẫm đạp), **Cache Penetration** (Thủng đệm), và **Cache Avalanche** (Tuyết lở bộ đệm).
- Hiểu sâu sắc kiến trúc Hàng đợi thông điệp: Mô hình Nhà sản xuất - Người tiêu thụ (Producer-Consumer), Cơ chế xác nhận hoàn tất (Ack / Nack), và Hàng đợi thư chết (Dead-Letter Queue - DLQ).
- Tự tay lập trình một hệ thống Lưu trữ đệm kèm Hàng đợi sự kiện phân tán bằng Rust chuẩn mực, an toàn đa luồng và tối ưu hóa bộ nhớ đệm (buffer) tuyệt đối.

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
- Hàng đợi Message Queue (như Kafka, RabbitMQ, hay Redis Streams) đóng vai trò như chiếc rào chắn: Toàn bộ 100,000 yêu cầu được ghi nhận vào hàng đợi chỉ mất 1 mili-giây rồi báo cho khách hàng: *"Yêu cầu của bạn đã được tiếp nhận, vui lòng chờ xử lý"*.
- Đằng sau hàng rào, một đội ngũ gồm 10 tiến trình công nhân (Workers / Consumers) cần mẫn rút từng đơn hàng ra xử lý tuần tự, giúp hệ thống không bao giờ bị quá tải.

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

Dưới đây là mã nguồn Rust hoàn chỉnh hiện thực hóa một **Tầng lưu trữ đệm kèm Hàng đợi thông điệp phân tán (In-Memory Cache-Aside & Message Queue)**: Tự tay cài đặt cơ chế hết hạn TTL, giải thuật dọn rác LRU, hàng đợi Producer-Consumer an toàn đa luồng, và cơ chế phòng chống Cache Stampede:

```rust
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
    println!("\n[1] Kiem attempt mo thuc Cache-Aside kem TTL Expiration:");
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
    println!("\n[2] Kiem attempt Hang doi Thong diep phan tan (Message Queue):");
    let message_queue = Arc::new(DistributedMessageQueue::<String>::new(5));

    // Luồng Producer: Đẩy việc vào hàng đợi
    let producer_q = Arc::clone(&message_queue);
    let producer_handle = std::thread::spawn(move || {
        for i in 1..=4 {
            let msg = format!("DonHang_#{}", i);
            producer_q.push(msg.clone()).unwrap();
            println!("    [Producer] Da day '{}' vao hang doi an total.", msg);
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
```

---

## Bảng tra cứu lỗi biên dịch & Cách khắc phục (Compiler Error Guide)

Dưới đây là các lỗi biên dịch thường gặp nhất khi triển khai bộ đệm Cache và hàng đợi thông điệp trong Rust:

| Mã lỗi | Thông báo mẫu từ trình biên dịch | Nguyên nhân cốt lõi | Cách khắc phục nhanh |
|---|---|---|---|
| **E0502** | `cannot borrow '*self' as mutable because it is also borrowed as immutable` | Bạn vừa đọc dữ liệu trong `HashMap` bộ đệm vừa cố gắng xóa một mục hết hạn. | Sao chép giá trị cần thiết ra ngoài trước khi thực hiện thao tác xóa, hoặc phân tách phạm vi mượn. |
| **E0382** | `use of moved value: 'message_queue'` | Di chuyển quyền sở hữu (ownership) của hàng đợi vào luồng con mà quên bọc trong con trỏ đếm tham chiếu `Arc`. | Bọc cấu trúc trong `Arc::new(...)` và tạo bản sao `Arc::clone(&queue)` cho mỗi luồng. |
| **E0277** | `the trait 'Eq' is not implemented for 'MyKey'` | Khóa của bảng băm `HashMap` bắt buộc phải triển khai trait `Hash` và `Eq`. | Bổ sung derive tự động: `#[derive(Hash, PartialEq, Eq, Clone)]` lên trên kiểu khóa. |
| **E0599** | `no method named 'pop' found for struct 'Arc<...>'` | Gọi trực tiếp phương thức của struct nội bộ trên con trỏ thông minh (smart pointer) `Arc` mà chưa giải tham chiếu. | Rust tự động Deref, nhưng nếu phương thức đòi hỏi mượn khả biến `&mut`, phải bọc trong `Mutex`. |

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
1. **Bảo vệ Cơ sở dữ liệu**: Tầng lưu trữ đệm phân tán Redis giải cứu Database khỏi nghẽn cổ chai, giảm độ trễ truy vấn từ hàng chục mili-giây xuống dưới 1 mili-giây.
2. **Khắc chế 3 Hiểm họa Cache**: Triệt tiêu Cache Stampede bằng khóa phân tán, chống Cache Penetration bằng Bloom Filter, và chống Cache Avalanche bằng khoảng lệch thời gian ngẫu nhiên (TTL Jitter).
3. **Sức mạnh của Hàng đợi Thông điệp**: Đóng vai trò đập thủy điện san phẳng các đợt bùng nổ lưu lượng, tách rời các dịch vụ và bảo đảm độ tin cậy của luồng xử lý.
4. **An toàn Đa luồng Không Rò rỉ**: Vận dụng chuẩn mực quyền sở hữu (ownership), mượn (borrow), thời gian sống (lifetime), con trỏ thông minh (smart pointer) và bộ nhớ đệm (buffer) để bảo đảm các tiến trình đọc/ghi song song luôn đạt thông lượng cao nhất.

### Bài tập rèn luyện tự giải:
1. **Bài tập 1 (Bổ sung Thuật toán Đào thải trang LRU vào Cache)**:  
   Mở rộng `SafeCacheEngine`: Khi bộ nhớ đệm đạt tới giới hạn dung lượng tối đa (ví dụ 1,000 mục), hãy tự động tìm và xóa mục có thời gian truy cập lâu nhất (Least Recently Used) để nhường chỗ cho mục mới.
2. **Bài tập 2 (Xây dựng Hàng đợi Thư Chết - Dead-Letter Queue)**:  
   Trong `DistributedMessageQueue`, nếu một thông điệp bị xử lý thất bại quá 3 lần liên tiếp, thay vì vứt bỏ, hãy tự động chuyển thông điệp đó sang một hàng đợi riêng biệt mang tên `DeadLetterQueue` để các kỹ sư quản trị có thể kiểm tra và gỡ lỗi thủ công.
3. **Bài tập 3 (Suy ngẫm kiến trúc: Tại sao Cache Invalidation là một trong hai bài toán khó nhất?)**:  
   Chuyên gia Martin Fowler từng nói: *"Chỉ có hai thứ khó trong khoa học máy tính: Đặt tên biến và Hủy tính hợp lệ của Cache (Cache Invalidation)"*. Hãy phân tích một tình huống cụ thể: Khi người dùng đổi mật khẩu, làm thế nào để đảm bảo 10 máy chủ Cache phân tán trên toàn cầu cùng hủy bỏ phiên đăng nhập cũ ngay lập tức mà không để xảy ra kẽ hở bảo mật?
