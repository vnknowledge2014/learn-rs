#![allow(dead_code)]
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
