#![allow(dead_code, unused_variables, unused_imports)]
// ============================================================================
// CHƯƠNG 39: MINH HỌA TƯ DUY VIBE CODING & KIẾN TRÚC HỢP ĐỒNG GIAO ƯỚC (CONTRACT)
// Tác giả: Tổng Đạo Diễn Kiến Trúc Rust (System Architect)
// ============================================================================

// 1. ĐỊNH NGHĨA KIỂU DỮ LIỆU NGHIỆP VỤ (DOMAIN MODELING)
// Sử dụng Enum để quản lý các trạng thái đơn hàng một cách tường minh.
#[derive(Debug, Clone, PartialEq, Eq)]
pub enum OrderStatus {
    Created,
    Paid { transaction_id: String },
    Failed { reason: String },
}

// Cấu trúc đơn hàng với nguyên tắc: Số tiền luôn dùng số nguyên u64 (cents/xu)
// để loại bỏ triệt để sai số làm tròn số thực của máy tính.
#[derive(Debug, Clone)]
pub struct Order {
    pub id: u64,
    pub customer_name: String,
    pub amount_cents: u64,
    pub status: OrderStatus,
}

impl Order {
    pub fn new(id: u64, customer_name: &str, amount_cents: u64) -> Self {
        Self {
            id,
            customer_name: customer_name.to_string(),
            amount_cents,
            status: OrderStatus::Created,
        }
    }
}

// 2. ĐỊNH NGHĨA TRẠNG THÁI LỖI TƯỜNG MINH (ERROR DOMAIN)
// Không bao giờ dùng chuỗi thô để mô tả lỗi; phân loại rõ ràng giúp hệ thống tự phục hồi.
#[derive(Debug, Clone, PartialEq, Eq)]
pub enum PaymentError {
    InsufficientFunds { available: u64, required: u64 },
    NetworkTimeout(String),
    InvalidCurrency(String),
    CardExpired,
}

// 3. GIAO ƯỚC HÀNH VI (CONTRACT TRAIT)
// Kiến trúc sư định nghĩa bản thiết kế: Bất kỳ cổng thanh toán nào cũng phải tuân thủ trait này.
pub trait PaymentGateway {
    fn process_payment(&self, account_id: &str, amount_cents: u64) -> Result<String, PaymentError>;
}

// 4. HIỆN THỰC HÓA BỞI AI: CỔNG THANH TOÁN THỬ NGHIỆM (MOCK PAYMENT GATEWAY)
// Phần mã chi tiết này do trợ lý AI sinh ra dựa trên bản thiết kế Trait ở trên.
pub struct MockBankingGateway {
    pub mock_balance_cents: u64,
}

impl PaymentGateway for MockBankingGateway {
    fn process_payment(&self, account_id: &str, amount_cents: u64) -> Result<String, PaymentError> {
        // Kiểm tra dữ liệu đầu vào: tài khoản không được để trống
        if account_id.is_empty() {
            return Err(PaymentError::NetworkTimeout("Mã định danh tài khoản không hợp lệ".to_string()));
        }

        // Kiểm tra số dư khả dụng
        if self.mock_balance_cents < amount_cents {
            return Err(PaymentError::InsufficientFunds {
                available: self.mock_balance_cents,
                required: amount_cents,
            });
        }

        // Sinh mã giao dịch thành công duy nhất
        let tx_id = format!("TXN-{}-OK", amount_cents);
        Ok(tx_id)
    }
}

// 5. BỘ ĐIỀU PHỐI ĐƠN HÀNG (ORDER PROCESSOR)
// Kiến trúc sư thiết kế bộ điều phối nhận vào một tham chiếu mượn (borrow) cổng thanh toán,
// tuân thủ nghiêm ngặt quyền sở hữu (ownership) và không làm sao chép dữ liệu thừa.
pub struct OrderProcessor<'a, G: PaymentGateway> {
    gateway: &'a G,
}

impl<'a, G: PaymentGateway> OrderProcessor<'a, G> {
    pub fn new(gateway: &'a G) -> Self {
        Self { gateway }
    }

    // Xử lý đơn hàng: Mượn khả biến (&mut) đơn hàng để cập nhật trạng thái
    pub fn checkout(&self, order: &mut Order, account_id: &str) -> Result<(), PaymentError> {
        println!("[Hệ thống] Bắt đầu thanh toán đơn hàng #{} cho khách hàng: {}", order.id, order.customer_name);

        match self.gateway.process_payment(account_id, order.amount_cents) {
            Ok(tx_id) => {
                println!("[Hệ thống] Thanh toán thành công! Mã giao dịch: {}", tx_id);
                order.status = OrderStatus::Paid { transaction_id: tx_id };
                Ok(())
            }
            Err(err) => {
                println!("[Cảnh báo] Thanh toán thất bại: {:?}", err);
                order.status = OrderStatus::Failed {
                    reason: format!("{:?}", err),
                };
                Err(err)
            }
        }
    }
}

// 6. HÀM MAIN KIỂM CHỨNG TOÀN BỘ LUỒNG HOẠT ĐỘNG
fn main() {
    println!("=== DEMO VIBE CODING PARADIGM: KIẾN TRÚC SƯ & HỆ THỐNG GIAO ƯỚC ===");

    // Tạo cổng thanh toán giả lập với số dư 50,000 xu (500 USD)
    let mock_gateway = MockBankingGateway {
        mock_balance_cents: 50_000,
    };

    // Khởi tạo bộ xử lý đơn hàng
    let processor = OrderProcessor::new(&mock_gateway);

    // Kịch bản 1: Đơn hàng hợp lệ (30,000 xu <= 50,000 xu)
    let mut order_1 = Order::new(101, "Nguyễn Văn An", 30_000);
    println!("Trạng thái ban đầu đơn #101: {:?}", order_1.status);
    let result_1 = processor.checkout(&mut order_1, "ACC-USER-888");
    assert!(result_1.is_ok());
    println!("Trạng thái sau thanh toán đơn #101: {:?}\n", order_1.status);

    // Kịch bản 2: Đơn hàng vượt hạn mức (80,000 xu > 50,000 xu)
    let mut order_2 = Order::new(102, "Trần Thị Bình", 80_000);
    println!("Trạng thái ban đầu đơn #102: {:?}", order_2.status);
    let result_2 = processor.checkout(&mut order_2, "ACC-USER-999");
    assert!(result_2.is_err());
    println!("Trạng thái sau thanh toán đơn #102: {:?}", order_2.status);

    println!("\n[Tổng kết] Toàn bộ kịch bản nghiệp vụ hoạt động chính xác 100% theo bản vẽ kiến trúc!");
}
