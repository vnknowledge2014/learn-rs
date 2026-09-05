#![allow(dead_code, unused_variables, unused_imports)]
use std::collections::HashMap;
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

/// Giao diện Hợp đồng Dịch vụ Người dùng (Domain Service Interface)
pub trait UserService: Send + Sync {
    fn get_user(&self, user_id: u64) -> Result<UserProfile, &'static str>;
}

/// Trạng thái hoạt động của Ngắt mạch (Circuit Breaker States)
#[derive(Debug, PartialEq, Eq, Clone, Copy)]
pub enum CircuitState {
    Closed,   // Bình thường: Cho phép yêu cầu đi qua
    Open,     // Ngắt mạch: Từ chối ngay lập tức để bảo vệ hệ thống
    HalfOpen, // Nửa mở: Cho phép thử nghiệm vài yêu cầu
}

/// Bộ ngắt mạch chống sập lan truyền cho các cuộc gọi mạng phân tán
pub struct CircuitBreaker {
    state: CircuitState,
    failure_count: usize,
    failure_threshold: usize,
    last_state_change: Instant,
    cooldown_duration: Duration,
}

impl CircuitBreaker {
    pub fn new(failure_threshold: usize, cooldown_ms: u64) -> Self {
        Self {
            state: CircuitState::Closed,
            failure_count: 0,
            failure_threshold,
            last_state_change: Instant::now(),
            cooldown_duration: Duration::from_millis(cooldown_ms),
        }
    }

    /// Kiểm tra xem yêu cầu có được phép thực thi hay không
    pub fn allow_request(&mut self) -> bool {
        match self.state {
            CircuitState::Closed => true,
            CircuitState::Open => {
                // Kiểm tra xem đã hết thời gian hồi sức (Cooldown) chưa
                if self.last_state_change.elapsed() >= self.cooldown_duration {
                    println!("    [CircuitBreaker] Hết thời gian chờ: Chuyển sang HALF-OPEN để thử nghiệm!");
                    self.state = CircuitState::HalfOpen;
                    self.last_state_change = Instant::now();
                    true
                } else {
                    false // Vẫn ngắt mạch, từ chối cuộc gọi mạng
                }
            }
            CircuitState::HalfOpen => true,
        }
    }

    /// Báo cáo cuộc gọi mạng thành công
    pub fn record_success(&mut self) {
        if self.state == CircuitState::HalfOpen {
            println!("    [CircuitBreaker] Yêu cầu thử nghiệm thành công: Phục hồi trạng thái CLOSED!");
        }
        self.state = CircuitState::Closed;
        self.failure_count = 0;
    }

    /// Báo cáo cuộc gọi mạng thất bại
    pub fn record_failure(&mut self) {
        self.failure_count += 1;
        println!("    [CircuitBreaker] Ghi nhận thất bại #{}", self.failure_count);

        if self.failure_count >= self.failure_threshold {
            println!("    [!] [CẢNH BÁO] Số lỗi vượt ngưỡng: KÍCH HOẠT NGẮT MẠCH (OPEN)!");
            self.state = CircuitState::Open;
            self.last_state_change = Instant::now();
        }
    }
}

/// Hiện thực hóa Dịch vụ Người dùng chạy trong bộ nhớ (In-Memory Modular Implementation)
pub struct InMemoryUserService {
    users: HashMap<u64, UserProfile>,
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
        Self { users }
    }
}

impl UserService for InMemoryUserService {
    fn get_user(&self, user_id: u64) -> Result<UserProfile, &'static str> {
        self.users
            .get(&user_id)
            .cloned()
            .ok_or("Không tìm thấy thông tin người dùng")
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

    /// Tạo đơn hàng mới với sự bảo vệ của Circuit Breaker
    pub fn create_order(
        &self,
        order_id: u64,
        user_id: u64,
        item_name: &str,
        price_cents: u64,
    ) -> Result<OrderRecord, &'static str> {
        let mut breaker = self.circuit_breaker.lock().unwrap();

        // 1. Kiểm tra Circuit Breaker trước khi thực hiện cuộc gọi liên dịch vụ
        if !breaker.allow_request() {
            return Err("Dịch vụ Người dùng đang gặp sự cố: Circuit Breaker đang ngắt mạch để tự bảo vệ!");
        }

        // 2. Gọi sang dịch vụ người dùng để xác thực
        match self.user_service.get_user(user_id) {
            Ok(user) => {
                breaker.record_success();
                println!("    [OrderService] Xác thực thành công khách hàng: {}", user.username);
                Ok(OrderRecord {
                    order_id,
                    user_id: user.user_id,
                    item_name: item_name.to_string(),
                    price_cents,
                })
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
    println!("   KIEN TRUC PHAN TAN: MODULAR MONOLITH & CIRCUIT BREAKER RUST    ");
    println!("==================================================================");

    // Khởi tạo Dịch vụ Người dùng
    let user_service = Arc::new(InMemoryUserService::new());

    // Khởi tạo Dịch vụ Đơn hàng liên kết
    let order_service = OrderCoordinatorService::new(user_service);

    // 1. Thử nghiệm tạo đơn hàng hợp lệ
    println!("\n[1] Thử nghiệm tạo đơn hàng cho khách hàng hợp lệ (ID = 1):");
    match order_service.create_order(101, 1, "Sách Rust Masterclass Chuyên Sâu", 450000) {
        Ok(order) => println!("    [+] Đơn hàng tạo thành công: ID #{} - Sản phẩm: {}", order.order_id, order.item_name),
        Err(err) => println!("    [!] Thất bại: {}", err),
    }

    // 2. Thử nghiệm kích hoạt ngắt mạch Circuit Breaker bằng cách gọi liên tục ID không tồn tại
    println!("\n[2] Gửi liên tiếp các yêu cầu lỗi để kích hoạt Circuit Breaker:");
    for i in 1..=4 {
        println!("    --> Gửi yêu cầu #{} với user_id không tồn tại (ID = 999)...", i);
        let result = order_service.create_order(200 + i, 999, "Vật phẩm ảo", 10000);
        match result {
            Ok(_) => println!("        Thành công!"),
            Err(e) => println!("        Thất bại: {}", e),
        }
    }

    // 3. Yêu cầu thứ 5 bị chặn đứng ngay từ vòng gửi xe bởi Circuit Breaker
    println!("\n[3] Gửi yêu cầu tiếp theo khi ngắt mạch đang OPEN:");
    let blocked_call = order_service.create_order(301, 1, "Mặt hàng mới", 50000);
    println!("    - Kết quả cuộc gọi: {:?}", blocked_call);
    assert!(blocked_call.is_err());
    println!("    => Circuit Breaker đã chặn đứng cuộc gọi mạng, bảo vệ hệ thống tuyệt đối!");

    println!("\n==================================================================");
    println!("   XÁC NHẬN: KIẾN TRÚC PHÂN TÁN AN TOÀN - CHỐNG SẬP DÂY CHUYỀN!   ");
    println!("==================================================================");
}
