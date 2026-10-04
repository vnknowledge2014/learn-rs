# Chương 48: Kiến trúc hệ thống: Từ khối đơn Monolith đến Microservices phân tán hiệu năng cao (Monolithic vs High-Performance Microservices)

## Giới thiệu & Mục tiêu học tập

Chào mừng bạn đến với đỉnh cao của giáo trình Rust Masterclass: **Chủ đề 9: Thiết kế Hệ thống phân tán & Hiệu năng cao (System Design & High-Performance Distributed Systems)**! Nếu như ở các chủ đề trước bạn đã làm chủ từng viên gạch, thanh thép của ngôn ngữ — từ cú pháp, quản lý bộ nhớ, kiểm thử an ninh, đến lập trình mạng cấp thấp — thì trong chủ đề này, bạn sẽ khoác lên mình chiếc áo của một **Tổng công trình sư kiến trúc hệ thống (Lead System Architect)**.

Một câu hỏi mang tính sống còn mà mọi tập đoàn công nghệ lớn (như Amazon, Netflix, Discord, Cloudflare) đều phải giải quyết khi mở rộng quy mô phục vụ hàng trăm triệu người dùng là: **Lựa chọn kiến trúc nào giữa Khối đơn thống nhất (Monolithic) và Hệ thống Vi dịch vụ phân tán (Microservices)? Và tại sao việc chuyển đổi các dịch vụ lõi sang Rust lại tạo nên một cuộc cách mạng về hiệu năng và tiết kiệm hàng triệu USD chi phí hạ tầng đám mây?**

Trong chương mở đầu của Topic 9, chúng ta sẽ phân tích:
- Sự tiến hóa của kiến trúc phần mềm: Từ Khối đơn (Monolith), Khối đơn hướng module (Modular Monolith), đến Hệ thống vi dịch vụ phân tán (Microservices).
- Ranh giới nghiệp vụ (Bounded Contexts) theo phương pháp Domain-Driven Design (DDD): Khi nào nên tách dịch vụ và khi nào tách dịch vụ là một thảm họa tự sát.
- Phân tích bài toán kinh tế hạ tầng đám mây: Đối chiếu mức tiêu thụ tài nguyên thực tế giữa một Microservice viết bằng Java Spring Boot / Node.js (ngốn 500MB-1GB RAM) với cùng chức năng viết bằng Rust (chỉ tốn 15MB RAM và khởi động trong 5 mili-giây).
- Ngân sách độ trễ mạng (Latency Budget) và chi phí chuyển đổi dữ liệu (Serialization Overhead) trong môi trường phân tán.
- Các mô thức phòng vệ chống sập dây chuyền: Ngắt mạch tự động (Circuit Breaker) và Phân vùng chống tràn (Bulkhead).

---

## Hình tượng hóa đời sống (Intuitive Everyday Analogy)

Để hiểu thấu đáo bản chất của Monolith và Microservices mà không bị rối loạn bởi thuật ngữ kỹ thuật, hãy quan sát hai mô hình kinh doanh quen thuộc trong đời sống:

```
┌──────────────────────────────────────────────────────────────────────────────────┐
│         HÌNH TƯỢNG HÓA: ĐẠI SIÊU THỊ ĐA NĂNG VS TUYẾN PHỐ CHUYÊN DOANH           │
├──────────────────────────────────────────────────────────────────────────────────┤
│                                                                                  │
│ [1. KIẾN TRÚC KHỐI ĐƠN MONOLITH: ĐẠI SIÊU THỊ BÁCH HÓA TẬP TRUNG]                │
│ ┌──────────────────────────────────────────────────────────────────────┐         │
│ │ Tòa nhà siêu thị 5 tầng:                                             │         │
│ │ Tầng 1: Thực phẩm & Rau củ (User Service)                            │         │
│ │ Tầng 2: Quần áo thời trang (Catalog Service)                         │         │
│ │ Tầng 3: Thiết bị điện máy  (Order Service)                           │         │
│ │ Tầng 4: Rạp chiếu phim     (Payment Service)                         │         │
│ ├──────────────────────────────────────────────────────────────────────┤         │
│ │ Ưu điểm: Đi lại giữa các tầng rất nhanh bằng thang cuốn (In-Memory). │         │
│ │ Nhược điểm: Nếu chập điện cháy tầng 1, CẢ SIÊU THỊ BẮT BUỘC ĐÓNG CỬA!│         │
│ └──────────────────────────────────────────────────────────────────────┘         │
│                                                                                  │
│ [2. KIẾN TRÚC VI DỊCH VỤ PHÂN TÁN MICROSERVICES: TUYẾN PHỐ CHUYÊN DOANH]         │
│ Tuyến phố dài có các cửa hàng độc lập:                                           │
│ ┌────────────────┐ ┌────────────────┐ ┌────────────────┐ ┌────────────────┐     │
│ │ Tiệm Bánh Mì   │ │ Tiệm Thuốc Tây │ │ Tiệm Quần Áo   │ │ Quầy Thu Ngân  │     │
│ │ (Validation Service) │ │ (Order Service)│ │(Product Service│ │(Payment Service│     │
│ └────────────────┘ └────────────────┘ └────────────────┘ └────────────────┘     │
│ Ưu điểm: Nếu Tiệm Bánh Mì mất điện, Tiệm Thuốc vẫn mở cửa bán bình thường!      │
│ Nhược điểm: Khách muốn mua cả bánh và thuốc phải đi bộ qua lại (Độ trễ mạng)!    │
└──────────────────────────────────────────────────────────────────────────────────┘
```

### 1. Đại siêu thị tập trung (Monolithic Architecture)
- Hãy tưởng tượng bạn bước vào một Trung tâm thương mại 5 tầng đồ sộ: Mọi thứ từ quầy rau, tiệm bánh, cửa hàng quần áo, đến rạp chiếu phim đều nằm chung dưới một mái nhà.
- **Ưu điểm**: Mọi thứ kết nối cực kỳ nhanh. Nhân viên giao hàng chỉ cần đi thang máy từ tầng 1 lên tầng 3 (gọi hàm trực tiếp trên bộ nhớ RAM tốn vài nano-giây). Quản lý, tuyển dụng nhân sự tập trung dễ dàng.
- **Nhược điểm**: Toàn bộ tòa nhà dùng chung một hệ thống đường điện và máy bơm nước (dùng chung một Database). Nếu đường ống nước tầng 1 bị vỡ, ban quản lý buộc phải ngắt nước toàn bộ tòa nhà, khiến rạp chiếu phim tầng 4 cũng phải dừng chiếu.

### 2. Tuyến phố chuyên doanh độc lập (Microservices Architecture)
- Bây giờ, thay vì nhét tất cả vào một tòa nhà, người ta quy hoạch một khu phố gồm các ngôi nhà riêng biệt: Nhà làm bánh mì riêng, nhà bán thuốc tây riêng, nhà sửa xe riêng.
- **Ưu điểm**: Mỗi chủ tiệm tự trang bị máy phát điện và bể nước riêng (Database per Service). Nếu tiệm bánh mì bị sự cố hết bột, tiệm thuốc tây vẫn mở cửa đón khách bình thường mà không hề hay biết. Tiệm nào đông khách (ví dụ mùa dịch tiệm thuốc đông) có thể xây thêm tầng cơi nới mà không ảnh hưởng tới các nhà bên cạnh (Scale độc lập).
- **Nhược điểm**: Khách hàng muốn mua bánh mì xong mua thuốc tây thì phải ra đường đi bộ qua lại giữa trời mưa nắng. Đây chính là **Độ trễ mạng (Network Latency)** và chi phí đóng gói thông điệp qua dây cáp.

---

## Khái niệm & Cơ chế kỹ thuật chuyên sâu (In-Depth Technical Mechanics)

### 1. So sánh Ba Hình thái Kiến trúc Cốt lõi

```
[Monolith Đơn thuần]  ──►  [Modular Monolith]  ──►  [Distributed Microservices]
(Tất cả trộn lẫn)           (Mã tách module rõ,      (Mỗi dịch vụ là tiến trình
                             chạy chung tiến trình)   riêng, kết nối qua mạng)
```

1. **Khối đơn truyền thống (Classic Monolith)**:
   - Toàn bộ giao diện (UI), logic nghiệp vụ (Business Logic), và truy cập cơ sở dữ liệu được đóng gói thành một tệp nhị phân duy nhất.
   - **Thách thức**: Khi nhóm kỹ sư tăng lên 50 người, việc commit mã nguồn thường xuyên gây xung đột (Merge Conflicts), một lập trình viên thực tập sửa lỗi nhỏ có thể làm sập toàn bộ hệ thống sản xuất.
2. **Khối đơn hướng Module (Modular Monolith)**:
   - Vẫn biên dịch thành 1 tệp nhị phân duy nhất chạy trên máy chủ, nhưng mã nguồn được phân chia thành các crate hoặc module Rust độc lập với ranh giới giao tiếp công khai (Public Trait APIs) rõ ràng.
   - **Đây là điểm khởi đầu lý tưởng nhất**: Tận dụng tốc độ gọi hàm trực tiếp trong bộ nhớ (In-memory zero-cost abstraction) mà vẫn sẵn sàng tách thành Microservice bất kỳ lúc nào!
3. **Vi dịch vụ phân tán (Microservices)**:
   - Mỗi dịch vụ chạy như một tiến trình mạng độc lập (Network Process), có cơ sở dữ liệu riêng, giao tiếp với nhau qua HTTP REST API (Axum) hoặc gRPC (Tonic).

### 2. Cuộc cách mạng Rust trong Kinh tế học Đám mây (Cloud Economics)

Trong kỷ nguyên điện toán đám mây (AWS, Google Cloud, Kubernetes), chi phí hạ tầng máy chủ tỷ lệ thuận với lượng RAM và CPU mà ứng dụng tiêu thụ:

| Tiêu chí so sánh | Java Spring Boot / Node.js | Rust Microservice | Lợi thế vượt trội của Rust |
|---|---|---|---|
| **Bộ nhớ RAM khi khởi động** | 350MB – 800MB | 8MB – 15MB | **Ít hơn hàng chục lần** |
| **Thời gian khởi động lạnh (Cold Start)**| 5 – 20 giây | 2 – 5 mili-giây | Hoàn hảo cho Serverless & Auto-scaling |
| **Dừng hệ thống do dọn rác (GC Pause)** | vài ms – hàng trăm ms (tùy GC và cấu hình) | **Không có khoảng dừng GC** | Độ trễ đuôi $p99$ ổn định hơn (vẫn chịu ảnh hưởng của I/O, cấp phát, khóa) |
| **Mật độ Pod trên 1 máy chủ Kubernetes**| 10 – 20 pods | 200 – 400 pods | Tăng mật độ đáng kể, giảm chi phí máy chủ |

*Các con số trên là cỡ độ điển hình cho dịch vụ nhỏ, chỉ để định hướng — JVM hiện đại (GraalVM native image, CRaC) khởi động nhanh hơn nhiều, và con số thật phụ thuộc khối lượng công việc. Hãy tự đo trước khi ra quyết định.*

### 3. Ngân sách Độ trễ mạng (Latency Budget) & Serialization Overhead

- Khi gọi một hàm nội bộ trên RAM: Tốn khoảng **10 nano-giây**.
- Khi gọi qua mạng nội bộ Datacenter (RPC Call): Tốn khoảng **1 đến 5 mili-giây** (chậm hơn **100,000 lần**!).
- Do đó, nếu một yêu cầu của khách hàng phải nhảy qua 10 microservices liên tiếp, tổng độ trễ đã là 50ms chỉ riêng thời gian di chuyển trên dây mạng.
- Sử dụng các định dạng tuần tự hóa nhị phân tốc độ cao (như Protocol Buffers trong gRPC hoặc MessagePack) thay vì JSON cồng kềnh giúp thu nhỏ kích thước gói tin và giảm đáng kể gánh nặng CPU khi chuyển đổi chuỗi.

### 4. Mẫu Thiết kế Chống sập dây chuyền (Circuit Breaker Pattern)

Trong hệ thống phân tán, sự cố mạng là điều chắc chắn sẽ xảy ra. Nếu Dịch vụ B bị đơ phản hồi, Dịch vụ A tiếp tục gửi hàng ngàn yêu cầu sẽ dẫn tới cạn kiệt luồng và sập lan truyền (Cascading Failure):
- **Trạng thái Closed (Đóng)**: Hệ thống hoạt động bình thường, các yêu cầu được chuyển qua mạng.
- **Trạng thái Open (Mở / Ngắt mạch)**: Khi tỷ lệ lỗi vượt quá ngưỡng (ví dụ 50% lỗi trong 10 giây qua), ngắt mạch lập tức chặn đứng mọi yêu cầu mới, trả về lỗi ngay tức thì hoặc dữ liệu mặc định (Fallback) mà không gửi qua mạng nữa, giúp dịch vụ đích có thời gian phục hồi.
- **Trạng thái Half-Open (Nửa mở)**: Sau một khoảng thời gian chờ (ví dụ 30 giây), ngắt mạch cho phép một (hoặc vài) yêu cầu thử nghiệm đi qua để kiểm tra xem dịch vụ đích đã hồi phục hay chưa. Thăm dò thành công → Closed; thất bại → Open lại ngay.
- **Chỉ đếm sự cố hạ tầng**: hết thời gian chờ, mất kết nối, lỗi 5xx mới là "thất bại" của ngắt mạch. Câu trả lời hợp lệ kiểu "không tìm thấy người dùng" (404) chứng tỏ dịch vụ vẫn khỏe — nếu đếm nó, vài khách gõ sai mã sẽ làm ngắt mạch chặn *mọi* khách hàng.

---

## Mã nguồn minh họa thực chiến (Idiomatic Runnable Rust Blueprint)

Dưới đây là mã nguồn hoàn chỉnh của một kiến trúc **Modular Monolith sẵn sàng chuyển dịch sang Microservices phân tán**: Minh họa sự trừu tượng hóa ranh giới nghiệp vụ qua Trait `UserService` và bộ điều phối `OrderCoordinatorService`, cùng cơ chế phòng thủ **Ngắt mạch chống sập dây chuyền (Circuit Breaker)**. Lưu ý chi tiết: `Mutex` của ngắt mạch chỉ được giữ trong lúc kiểm tra/ghi nhận trạng thái, **không** giữ trong lúc gọi dịch vụ — nếu không, mọi yêu cầu đồng thời sẽ bị xếp hàng nối đuôi sau một cuộc gọi mạng chậm:

```rust
use std::collections::HashMap;
use std::sync::atomic::{AtomicBool, Ordering};
use std::sync::{Arc, Mutex};
use std::time::{Duration, Instant};

/// Mô hình Dữ liệu Người dùng
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct UserProfile {
    pub user_id: u64,
    pub username: String,
    pub email: String,
}

/// Mô hình Dữ liệu Đơn hàng
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct OrderRecord {
    pub order_id: u64,
    pub user_id: u64,
    pub item_name: String,
    pub price_cents: u64,
}

/// Lỗi khi gọi dịch vụ. Phân biệt hai loại là then chốt cho Circuit Breaker:
/// - `NotFound`: dịch vụ VẪN KHỎE, nó trả lời đúng rằng không có dữ liệu -> không tính là sự cố.
/// - `Unavailable`: dịch vụ sập/quá tải/hết thời gian chờ -> đây mới là thứ ngắt mạch cần đếm.
#[derive(Debug, Clone, PartialEq, Eq)]
pub enum ServiceError {
    NotFound,
    Unavailable,
    CircuitOpen,
}

/// Giao diện Hợp đồng Dịch vụ Người dùng (Domain Service Interface)
pub trait UserService: Send + Sync {
    fn get_user(&self, user_id: u64) -> Result<UserProfile, ServiceError>;
}

/// Trạng thái hoạt động của Ngắt mạch (Circuit Breaker States)
#[derive(Debug, PartialEq, Eq, Clone, Copy)]
pub enum CircuitState {
    Closed,   // Bình thường: Cho phép yêu cầu đi qua
    Open,     // Ngắt mạch: Từ chối ngay lập tức để bảo vệ hệ thống
    HalfOpen, // Nửa mở: Cho đúng MỘT yêu cầu thăm dò đi qua
}

/// Bộ ngắt mạch chống sập lan truyền cho các cuộc gọi mạng phân tán
pub struct CircuitBreaker {
    state: CircuitState,
    failure_count: usize,
    failure_threshold: usize,
    last_state_change: Instant,
    cooldown_duration: Duration,
    probe_in_flight: bool,
}

impl CircuitBreaker {
    pub fn new(failure_threshold: usize, cooldown_ms: u64) -> Self {
        Self {
            state: CircuitState::Closed,
            failure_count: 0,
            failure_threshold,
            last_state_change: Instant::now(),
            cooldown_duration: Duration::from_millis(cooldown_ms),
            probe_in_flight: false,
        }
    }

    pub fn state(&self) -> CircuitState {
        self.state
    }

    /// Kiểm tra xem yêu cầu có được phép thực thi hay không
    pub fn allow_request(&mut self) -> bool {
        match self.state {
            CircuitState::Closed => true,
            CircuitState::Open => {
                // Kiểm tra xem đã hết thời gian hồi sức (Cooldown) chưa
                if self.last_state_change.elapsed() >= self.cooldown_duration {
                    println!(
                        "    [CircuitBreaker] Hết thời gian chờ: Chuyển sang HALF-OPEN để thử nghiệm!"
                    );
                    self.state = CircuitState::HalfOpen;
                    self.last_state_change = Instant::now();
                    self.probe_in_flight = true;
                    true
                } else {
                    false // Vẫn ngắt mạch, từ chối cuộc gọi mạng
                }
            }
            // Chỉ một yêu cầu thăm dò tại một thời điểm; các yêu cầu khác vẫn bị chặn
            CircuitState::HalfOpen => {
                if self.probe_in_flight {
                    false
                } else {
                    self.probe_in_flight = true;
                    true
                }
            }
        }
    }

    /// Báo cáo cuộc gọi mạng thành công
    pub fn record_success(&mut self) {
        if self.state == CircuitState::HalfOpen {
            println!(
                "    [CircuitBreaker] Yêu cầu thử nghiệm thành công: Phục hồi trạng thái CLOSED!"
            );
        }
        self.state = CircuitState::Closed;
        self.failure_count = 0;
        self.probe_in_flight = false;
    }

    /// Báo cáo cuộc gọi mạng thất bại
    pub fn record_failure(&mut self) {
        self.failure_count += 1;
        self.probe_in_flight = false;
        println!(
            "    [CircuitBreaker] Ghi nhận thất bại #{}",
            self.failure_count
        );

        // Ở HALF-OPEN, chỉ một lần thăm dò thất bại là ngắt mạch lại ngay
        if self.state == CircuitState::HalfOpen || self.failure_count >= self.failure_threshold {
            println!("    [!] [CẢNH BÁO] Số lỗi vượt ngưỡng: KÍCH HOẠT NGẮT MẠCH (OPEN)!");
            self.state = CircuitState::Open;
            self.last_state_change = Instant::now();
        }
    }
}

/// Hiện thực hóa Dịch vụ Người dùng chạy trong bộ nhớ (In-Memory Modular Implementation).
/// Cờ `down` mô phỏng dịch vụ bị sập (trong thực tế: hết thời gian chờ, mất kết nối).
pub struct InMemoryUserService {
    users: HashMap<u64, UserProfile>,
    down: AtomicBool,
}

impl Default for InMemoryUserService {
    fn default() -> Self {
        Self::new()
    }
}

impl InMemoryUserService {
    pub fn new() -> Self {
        let mut users = HashMap::new();
        users.insert(
            1,
            UserProfile {
                user_id: 1,
                username: "nguyen_van_a".to_string(),
                email: "a@masterclass.vn".to_string(),
            },
        );
        Self {
            users,
            down: AtomicBool::new(false),
        }
    }

    pub fn set_down(&self, down: bool) {
        self.down.store(down, Ordering::SeqCst);
    }
}

impl UserService for InMemoryUserService {
    fn get_user(&self, user_id: u64) -> Result<UserProfile, ServiceError> {
        if self.down.load(Ordering::SeqCst) {
            return Err(ServiceError::Unavailable);
        }
        self.users
            .get(&user_id)
            .cloned()
            .ok_or(ServiceError::NotFound)
    }
}

/// Dịch vụ Điều phối Đơn hàng phân tán kết nối với Dịch vụ Người dùng
pub struct OrderCoordinatorService {
    user_service: Arc<dyn UserService>,
    circuit_breaker: Mutex<CircuitBreaker>,
}

impl OrderCoordinatorService {
    pub fn new(user_service: Arc<dyn UserService>) -> Self {
        Self {
            user_service,
            circuit_breaker: Mutex::new(CircuitBreaker::new(3, 200)), // Ngưỡng 3 lỗi, cooldown 200ms
        }
    }

    pub fn breaker_state(&self) -> CircuitState {
        self.circuit_breaker.lock().unwrap().state()
    }

    /// Tạo đơn hàng mới với sự bảo vệ của Circuit Breaker
    pub fn create_order(
        &self,
        order_id: u64,
        user_id: u64,
        item_name: &str,
        price_cents: u64,
    ) -> Result<OrderRecord, ServiceError> {
        // 1. Kiểm tra Circuit Breaker. Khóa chỉ giữ trong khối này: KHÔNG giữ Mutex
        //    trong lúc gọi mạng, nếu không mọi yêu cầu sẽ bị xếp hàng nối đuôi nhau.
        if !self.circuit_breaker.lock().unwrap().allow_request() {
            return Err(ServiceError::CircuitOpen);
        }

        // 2. Gọi sang dịch vụ người dùng để xác thực (không giữ khóa)
        let result = self.user_service.get_user(user_id);

        // 3. Báo kết quả cho Circuit Breaker
        let mut breaker = self.circuit_breaker.lock().unwrap();
        match result {
            Ok(user) => {
                breaker.record_success();
                println!(
                    "    [OrderService] Xác thực thành công khách hàng: {}",
                    user.username
                );
                Ok(OrderRecord {
                    order_id,
                    user_id: user.user_id,
                    item_name: item_name.to_string(),
                    price_cents,
                })
            }
            // Dịch vụ trả lời "không có" -> dịch vụ vẫn khỏe, không tính là sự cố
            Err(ServiceError::NotFound) => {
                breaker.record_success();
                Err(ServiceError::NotFound)
            }
            Err(err) => {
                breaker.record_failure();
                Err(err)
            }
        }
    }
}

fn main() {
    println!("==================================================================");
    println!("   KIẾN TRÚC PHÂN TÁN: MODULAR MONOLITH & CIRCUIT BREAKER RUST    ");
    println!("==================================================================");

    // Khởi tạo Dịch vụ Người dùng
    let user_service = Arc::new(InMemoryUserService::new());

    // Khởi tạo Dịch vụ Đơn hàng liên kết (Arc<InMemoryUserService> -> Arc<dyn UserService>)
    let order_service = OrderCoordinatorService::new(user_service.clone());

    // 1. Thử nghiệm tạo đơn hàng hợp lệ
    println!("\n[1] Thử nghiệm tạo đơn hàng cho khách hàng hợp lệ (ID = 1):");
    match order_service.create_order(101, 1, "Sách Rust Masterclass Chuyên Sâu", 450000) {
        Ok(order) => println!(
            "    [+] Đơn hàng tạo thành công: ID #{} - Sản phẩm: {}",
            order.order_id, order.item_name
        ),
        Err(err) => println!("    [!] Thất bại: {:?}", err),
    }

    // 2. Khách không tồn tại: lỗi nghiệp vụ, KHÔNG được làm ngắt mạch
    println!("\n[2] Khách hàng không tồn tại (ID = 999) — lỗi nghiệp vụ, mạch vẫn CLOSED:");
    let not_found = order_service.create_order(150, 999, "Vật phẩm", 10000);
    println!("    - Kết quả: {:?}", not_found);
    assert_eq!(not_found, Err(ServiceError::NotFound));
    assert_eq!(order_service.breaker_state(), CircuitState::Closed);

    // 3. Dịch vụ người dùng sập: gửi liên tiếp để kích hoạt ngắt mạch
    println!("\n[3] Dịch vụ Người dùng sập — gửi liên tiếp các yêu cầu:");
    user_service.set_down(true);
    for i in 1..=4 {
        let result = order_service.create_order(200 + i, 1, "Vật phẩm ảo", 10000);
        println!("    --> Yêu cầu #{}: {:?}", i, result);
    }
    // Yêu cầu thứ 4 đã bị chặn ngay tại chỗ, không tốn một cuộc gọi mạng nào
    assert_eq!(order_service.breaker_state(), CircuitState::Open);

    // 4. Dịch vụ hồi phục; sau thời gian cooldown, yêu cầu thăm dò khép mạch lại
    println!("\n[4] Dịch vụ hồi phục, chờ hết cooldown 200ms rồi gửi yêu cầu thăm dò:");
    user_service.set_down(false);
    std::thread::sleep(Duration::from_millis(250));
    let probe = order_service.create_order(301, 1, "Mặt hàng mới", 50000);
    println!("    - Kết quả cuộc gọi: {:?}", probe);
    assert!(probe.is_ok());
    assert_eq!(order_service.breaker_state(), CircuitState::Closed);

    println!("\n==================================================================");
    println!("   XÁC NHẬN: NGẮT MẠCH CHẶN SẬP DÂY CHUYỀN VÀ TỰ PHỤC HỒI         ");
    println!("==================================================================");
}

#[cfg(test)]
mod tests {
    use super::*;

    fn setup() -> (Arc<InMemoryUserService>, OrderCoordinatorService) {
        let svc = Arc::new(InMemoryUserService::new());
        let orders = OrderCoordinatorService::new(svc.clone());
        (svc, orders)
    }

    #[test]
    fn not_found_does_not_trip_breaker() {
        let (_svc, orders) = setup();
        for i in 0..10 {
            assert_eq!(
                orders.create_order(i, 999, "x", 1),
                Err(ServiceError::NotFound)
            );
        }
        assert_eq!(orders.breaker_state(), CircuitState::Closed);
        assert!(orders.create_order(99, 1, "x", 1).is_ok());
    }

    #[test]
    fn unavailable_trips_breaker_then_blocks() {
        let (svc, orders) = setup();
        svc.set_down(true);
        for i in 0..3 {
            assert_eq!(
                orders.create_order(i, 1, "x", 1),
                Err(ServiceError::Unavailable)
            );
        }
        assert_eq!(orders.breaker_state(), CircuitState::Open);
        svc.set_down(false); // dù dịch vụ đã khỏe, mạch vẫn chặn cho tới hết cooldown
        assert_eq!(
            orders.create_order(9, 1, "x", 1),
            Err(ServiceError::CircuitOpen)
        );
    }

    #[test]
    fn half_open_allows_single_probe_and_failed_probe_reopens() {
        let mut cb = CircuitBreaker::new(1, 0);
        assert!(cb.allow_request());
        cb.record_failure();
        assert_eq!(cb.state(), CircuitState::Open);
        assert!(cb.allow_request()); // cooldown 0 -> HALF-OPEN, đây là yêu cầu thăm dò
        assert_eq!(cb.state(), CircuitState::HalfOpen);
        assert!(!cb.allow_request()); // yêu cầu thứ hai bị chặn khi thăm dò đang chạy
        cb.record_failure();
        assert_eq!(cb.state(), CircuitState::Open);
    }
}
```

---

## Bảng tra cứu lỗi biên dịch & Cách khắc phục (Compiler Error Guide)

Dưới đây là các lỗi biên dịch thường gặp nhất khi thiết kế kiến trúc phân tán hướng Trait trong Rust:

| Mã lỗi | Thông báo mẫu từ trình biên dịch | Nguyên nhân cốt lõi | Cách khắc phục nhanh |
|---|---|---|---|
| **E0277** | `` `dyn UserService` cannot be sent between threads safely `` (và `cannot be shared between threads safely`) | Khi chia sẻ một đối tượng Trait qua các luồng bằng `Arc<dyn UserService>`, Trait đó bắt buộc phải có ràng buộc `Send + Sync`. | Định nghĩa Trait với ràng buộc luồng: `pub trait UserService: Send + Sync { ... }`. |
| **E0038** | `` the trait `UserService` is not dyn compatible `` (rustc cũ ghi: `cannot be made into an object`) | Trait chứa phương thức generic, phương thức không có receiver `self`, hoặc trả về `Self` — vi phạm quy tắc tương thích `dyn` (dyn compatibility, trước gọi là object safety). Riêng `fn f(self)` khi `Self: Sized` không tự động gây lỗi này. | Bỏ generic khỏi phương thức của trait (nhận kiểu cụ thể / `&dyn Trait`), hoặc gắn `where Self: Sized` cho phương thức không cần gọi qua `dyn`. |
| **E0599** | `no method named 'clone' found for struct 'OrderRecord'` | Bạn gọi `.clone()` trên một cấu trúc dữ liệu domain mà quên khai báo derive tự động. | Thêm macro derive: `#[derive(Clone, Debug)]` trên cấu trúc dữ liệu. |
| **E0382** | `use of moved value: 'user_service'` | Di chuyển quyền sở hữu (ownership) của dịch vụ vào một luồng khác mà không bọc trong con trỏ thông minh (smart pointer) chia sẻ. | Sử dụng con trỏ đếm tham chiếu đa luồng: `Arc::clone(&user_service)`. |

### Ví dụ phân tích lỗi `E0038` khi thiết kế Trait Object cho Microservice:

```rust
// Đoạn mã lỗi minh họa E0038: Trait không thỏa mãn Object Safety
trait FailingService {
    // Lỗi: Hàm generic không thể gọi qua Trait Object động (không có vtable cho mọi T)
    fn process_generic<T>(&self, data: T);
}

// fn call_service(svc: &dyn FailingService) {} // LỖI E0038: not dyn compatible!

// Cách sửa chữa đúng chuẩn: Dùng kiểu cụ thể hoặc lát cắt byte
trait CorrectService: Send + Sync {
    fn handle_idiomatic(&self, data: &[u8]) -> Result<(), &'static str>;
}

fn call_correct_service(svc: &dyn CorrectService) {
    let _ = svc.handle_idiomatic(b"data");
}
```

---

## Tóm tắt chương & Bài tập rèn luyện (Summary & Exercises)

### 4 Điểm cốt lõi cần ghi nhớ:
1. **Tiến trình kiến trúc tự nhiên**: Hãy bắt đầu với một Modular Monolith chặt chẽ trước khi quyết định xé nhỏ thành các Microservice phân tán.
2. **Kinh tế học Rust trên Đám mây**: Nhờ mức tiêu thụ RAM cực thấp (~15MB), không có độ trễ GC, và thời gian khởi động tính bằng mili-giây, Rust giúp doanh nghiệp cắt giảm đáng kể hóa đơn máy chủ (mức cụ thể phải đo trên khối lượng thật).
3. **Chi phí Độ trễ mạng**: Gọi hàm nội bộ trên RAM nhanh gấp 100,000 lần gọi qua mạng. Tận dụng định dạng nhị phân tốc độ cao để giảm thiểu chi phí chuyển đổi dữ liệu.
4. **Phòng chống sập lan truyền**: Luôn trang bị mô hình Ngắt mạch (Circuit Breaker) và Phân vùng chống tràn (Bulkhead) cho mọi điểm giao tiếp mạng, kết hợp cơ chế quyền sở hữu (ownership), mượn (borrow), thời gian sống (lifetime), con trỏ thông minh (smart pointer) và bộ nhớ đệm (buffer) để bảo vệ toàn vẹn hệ thống.

### Bài tập rèn luyện tự giải:
1. **Bài tập 1 (Bổ sung cơ chế Fallback Cache)**:  
   Mở rộng `OrderCoordinatorService`: Khi Circuit Breaker ở trạng thái `Open`, thay vì trả về lỗi ngay lập tức, hãy cho dịch vụ tra cứu thông tin khách hàng từ một bảng băm bộ đệm cục bộ (Local Cache) đã lưu từ trước.
2. **Bài tập 2 (Hiện thực hóa Bộ giới hạn số lượng cuộc gọi đồng thời - Bulkhead)**:  
   Viết một cấu trúc `BulkheadSemaphore` giới hạn tối đa chỉ cho phép 10 yêu cầu gọi mạng chạy đồng thời cùng lúc. Nếu có yêu cầu thứ 11 ập vào trong khi 10 yêu cầu trước chưa hoàn thành, lập tức xếp vào hàng đợi chờ hoặc từ chối để chống tràn tài nguyên máy chủ.
3. **Bài tập 3 (Suy ngẫm kiến trúc: Khi nào không nên dùng Microservices?)**:  
   Một công ty khởi nghiệp chỉ có 3 lập trình viên và 500 người dùng hoạt động mỗi ngày có nên chia hệ thống thành 15 microservices độc lập hay không? Rủi ro lớn nhất về mặt vận hành hạ tầng (DevOps, Giám sát hệ thống, Distributed Tracing) mà họ sẽ phải đối mặt là gì?

---

### Gợi ý & Lời giải

<details>
<summary><b>Bài tập 1 — Gợi ý</b></summary>

Khi mạch ngắt (Open), thay vì trả lỗi ngay, tra một bản đệm cục bộ đã lưu trước đó. Dữ liệu có thể cũ nhưng còn hơn không — đây là suy giảm duyên dáng (graceful degradation).
</details>

<details>
<summary><b>Bài tập 1 — Lời giải</b></summary>

```rust
use std::collections::HashMap;

/// Bộ đệm dự phòng: giữ bản sao dữ liệu khách hàng đã tra thành công trước đó.
/// Khi mạch Open, phục vụ từ đây thay vì trả lỗi ngay.
pub struct FallbackCache {
    snapshot: HashMap<u64, String>, // user_id -> tên (bản chụp gần nhất)
}

impl FallbackCache {
    pub fn new() -> Self { Self { snapshot: HashMap::new() } }

    /// Mỗi lần tra THÀNH CÔNG qua dịch vụ thật, lưu lại vào đệm.
    pub fn remember(&mut self, user_id: u64, name: &str) {
        self.snapshot.insert(user_id, name.to_string());
    }

    /// Khi mạch Open: trả bản đệm nếu có (có thể cũ), None nếu chưa từng thấy.
    pub fn lookup_fallback(&self, user_id: u64) -> Option<String> {
        self.snapshot.get(&user_id).cloned()
    }
}

#[test]
fn serves_from_cache_when_open() {
    let mut cache = FallbackCache::new();
    // Lúc mạch còn Closed và tra thành công -> ghi nhớ.
    cache.remember(42, "Nguyễn Văn A");
    // Sau đó dịch vụ sập, mạch chuyển Open -> phục vụ từ đệm thay vì lỗi.
    assert_eq!(cache.lookup_fallback(42), Some("Nguyễn Văn A".to_string()));
    // Khách chưa từng tra thành công -> không có gì để dự phòng.
    assert_eq!(cache.lookup_fallback(99), None);
}
```

**Cách gắn vào `OrderCoordinatorService`:** trong luồng `create_order`, khi `circuit_breaker.allow_request()` trả `false` (mạch Open), thay vì `return Err(...)` ngay, hãy gọi `fallback_cache.lookup_fallback(user_id)`. Có bản đệm thì dùng nó tiếp tục xử lý (đánh dấu "dữ liệu có thể cũ"); không có thì mới trả lỗi.

Đây là nguyên tắc **suy giảm duyên dáng (graceful degradation)**: khi một phụ thuộc sập, hệ thống không sập theo mà *lùi về mức phục vụ thấp hơn nhưng vẫn dùng được*. Đánh đổi phải nói rõ: dữ liệu đệm có thể **cũ** (khách vừa đổi tên xong chẳng hạn), nên chỉ hợp với dữ liệu mà "hơi cũ" là chấp nhận được (tên, hồ sơ) — *không* dùng cho dữ liệu phải luôn đúng khoảnh khắc (số dư tài khoản, tồn kho lúc thanh toán).
</details>

<details>
<summary><b>Bài tập 2 — Gợi ý</b></summary>

Semaphore đếm số 'giấy phép' còn lại. Mỗi cuộc gọi lấy một giấy phép trước khi chạy, trả lại khi xong. Hết giấy phép thì yêu cầu mới bị từ chối — chặn cạn kiệt tài nguyên.
</details>

<details>
<summary><b>Bài tập 2 — Lời giải</b></summary>

```rust
use std::sync::{Arc, Mutex};

/// Bulkhead (vách ngăn): giới hạn TỐI ĐA số cuộc gọi chạy đồng thời.
/// Như khoang kín trên tàu — một khoang ngập nước không làm chìm cả tàu.
pub struct BulkheadSemaphore {
    available_permits: Arc<Mutex<usize>>,
    max_permits: usize,
}

/// Chứng từ giữ chỗ: tự động TRẢ giấy phép khi ra khỏi phạm vi (RAII).
pub struct Permit {
    permits: Arc<Mutex<usize>>,
}
impl Drop for Permit {
    fn drop(&mut self) {
        *self.permits.lock().unwrap() += 1; // trả giấy phép khi cuộc gọi kết thúc
    }
}

impl BulkheadSemaphore {
    pub fn new(max_permits: usize) -> Self {
        Self { available_permits: Arc::new(Mutex::new(max_permits)), max_permits }
    }

    /// Thử lấy một giấy phép. Some(chứng từ) nếu còn chỗ, None nếu đã đầy -> từ chối.
    pub fn try_acquire(&self) -> Option<Permit> {
        let mut remaining = self.available_permits.lock().unwrap();
        if *remaining == 0 {
            return None; // đã đủ 10 cuộc gọi đồng thời -> từ chối yêu cầu thứ 11
        }
        *remaining -= 1;
        Some(Permit { permits: Arc::clone(&self.available_permits) })
    }

    pub fn available(&self) -> usize { *self.available_permits.lock().unwrap() }
}

#[test]
fn limits_concurrency_to_10() {
    let bulkhead = BulkheadSemaphore::new(10);
    let mut held = Vec::new();
    // 10 cuộc gọi đầu: đều lấy được giấy phép.
    for _ in 0..10 {
        let p = bulkhead.try_acquire();
        assert!(p.is_some());
        held.push(p);
    }
    // Cuộc gọi thứ 11 khi 10 cái trước chưa xong -> bị từ chối.
    assert!(bulkhead.try_acquire().is_none());
    assert_eq!(bulkhead.available(), 0);

    // Một cuộc gọi xong (chứng từ bị hủy) -> trả lại 1 giấy phép.
    held.pop();
    assert_eq!(bulkhead.available(), 1);
    assert!(bulkhead.try_acquire().is_some()); // giờ lại nhận được
}
```

Mẫu Bulkhead lấy tên từ **vách ngăn kín nước trên tàu thủy**: chia thân tàu thành nhiều khoang để một khoang thủng không làm chìm cả con tàu. Ở đây nó chặn một lỗi kinh điển: một phụ thuộc chậm (dịch vụ mạng treo) khiến *hàng nghìn* yêu cầu cùng chờ, ngốn sạch luồng/bộ nhớ/kết nối của máy chủ — rồi *toàn bộ* hệ thống sập, không chỉ phần gọi dịch vụ chậm đó. Giới hạn "tối đa 10 cuộc gọi đồng thời" cô lập thiệt hại: yêu cầu thứ 11 bị từ chối *nhanh* (fail nhanh) thay vì xếp hàng chờ vô tận. Chi tiết Rust đẹp ở đây là **RAII qua `Drop`**: chứng từ `Permit` tự trả giấy phép khi ra khỏi phạm vi, nên không bao giờ rò rỉ giấy phép kể cả khi cuộc gọi hoảng loạn giữa chừng.
</details>

<details>
<summary><b>Bài tập 3 — Gợi ý</b></summary>

Câu hỏi về đánh đổi: vi dịch vụ giải quyết vấn đề *quy mô tổ chức* (nhiều đội làm việc độc lập), nhưng đội nhỏ thì cái giá vận hành lại vượt xa lợi ích.
</details>

<details>
<summary><b>Bài tập 3 — Lời giải</b></summary>

**Một startup 3 lập trình viên, 500 người dùng/ngày KHÔNG nên chia thành 15 vi dịch vụ.** Đây gần như là một sai lầm kiến trúc kinh điển.

**Vì sao vi dịch vụ sai ở quy mô này:** vi dịch vụ giải quyết một vấn đề *tổ chức*, không phải vấn đề *kỹ thuật* — nó cho phép **nhiều đội độc lập** triển khai riêng rẽ mà không giẫm chân nhau. Với 3 người, bạn *không có* vấn đề đó. Bạn nhận về mọi cái giá của hệ phân tán mà chẳng hưởng lợi ích nào.

**Rủi ro vận hành lớn nhất họ sẽ đối mặt:**

| Lĩnh vực | Với 1 khối liền (monolith) | Với 15 vi dịch vụ |
|---|---|---|
| **Triển khai (DevOps)** | 1 tiến trình, 1 lần deploy | 15 tiến trình, 15 đường ống CI/CD, điều phối container (Kubernetes...) mà 3 người phải tự vận hành |
| **Giám sát** | 1 tập log, 1 bảng điều khiển | 15 nguồn log rời rạc; một yêu cầu hỏng phải ghép mảnh từ 15 nơi |
| **Truy vết phân tán** | không cần — mọi lời gọi trong cùng tiến trình, đọc thẳng ngăn xếp | BẮT BUỘC phải có (Jaeger, Zipkin...): một yêu cầu nhảy qua 6 dịch vụ, muốn gỡ lỗi phải lần theo dấu vết xuyên mạng |
| **Gỡ lỗi** | đặt điểm dừng, đọc ngăn xếp | lỗi ẩn trong *khoảng giữa* các dịch vụ: độ trễ mạng, thử lại, một phần thất bại |
| **Nhất quán dữ liệu** | một CSDL, một giao dịch ACID | dữ liệu rải nhiều CSDL, phải xử lý giao dịch phân tán / Saga (xem Chương 54) |

**Rủi ro nghiêm trọng nhất: truy vết phân tán và gỡ lỗi xuyên dịch vụ.** Trong khối liền, một yêu cầu hỏng để lại *một* dấu vết ngăn xếp bạn đọc thẳng. Trong 15 vi dịch vụ, cùng yêu cầu đó xuyên qua nhiều tiến trình qua mạng — không có ngăn xếp chung, và bạn *phải* dựng sẵn hạ tầng truy vết phân tán chỉ để trả lời "yêu cầu này chết ở đâu?". Ba lập trình viên sẽ tiêu phần lớn thời gian **vận hành hạ tầng** thay vì xây tính năng.

**Lời khuyên chuẩn của ngành:** *"Bắt đầu bằng khối liền, tách vi dịch vụ khi nỗi đau tổ chức xuất hiện."* Ngay cả những người đề xướng vi dịch vụ (Martin Fowler, Sam Newman) cũng khuyên **"monolith first"**. Khi startup này lớn tới mức có nhiều đội mà việc deploy giẫm chân nhau, *khi đó* mới tách — và tách theo đường biên nghiệp vụ thật, không phải chia bừa thành 15 mảnh.
</details>
