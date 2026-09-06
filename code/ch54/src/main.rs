#![allow(dead_code, unused_variables, unused_imports)]
use std::collections::HashMap;
use std::fs::{File, OpenOptions};
use std::io::{self, BufRead, BufReader, Write};
use std::path::Path;
use std::sync::{Arc, Mutex};

/// Các trạng thái vòng đời của một Đơn hàng trong hệ thống phân tán
#[derive(Debug, PartialEq, Eq, Clone, Copy)]
pub enum OrderStatus {
    Pending,
    Validated,
    Paid,
    Fulfilled,
    Cancelled,
}

impl OrderStatus {
    pub fn as_str(&self) -> &'static str {
        match self {
            OrderStatus::Pending => "PENDING",
            OrderStatus::Validated => "VALIDATED",
            OrderStatus::Paid => "PAID",
            OrderStatus::Fulfilled => "FULFILLED",
            OrderStatus::Cancelled => "CANCELLED",
        }
    }

    pub fn from_str(s: &str) -> Option<Self> {
        match s {
            "PENDING" => Some(OrderStatus::Pending),
            "VALIDATED" => Some(OrderStatus::Validated),
            "PAID" => Some(OrderStatus::Paid),
            "FULFILLED" => Some(OrderStatus::Fulfilled),
            "CANCELLED" => Some(OrderStatus::Cancelled),
            _ => None,
        }
    }
}

/// Mô hình Đơn hàng đầy đủ trong hệ thống
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct OrderEntity {
    pub order_id: u64,
    pub customer_id: u64,
    pub item_id: u32,
    pub amount_cents: u64,
    pub status: OrderStatus,
}

/// Sự kiện biến động đơn hàng để ghi vào tệp nhật ký WAL
#[derive(Debug, Clone)]
pub struct OrderEvent {
    pub order_id: u64,
    pub new_status: OrderStatus,
    pub timestamp_ms: u64,
}

/// Động cơ Lưu trữ và Quản lý Kho hàng đồng thời (Thread-Safe Inventory Store)
pub struct InventoryManager {
    stock: Mutex<HashMap<u32, u32>>,
}

impl InventoryManager {
    pub fn new() -> Self {
        let mut stock = HashMap::new();
        stock.insert(101, 10); // Sản phẩm 101 có sẵn 10 chiếc
        stock.insert(102, 2);  // Sản phẩm 102 chỉ có sẵn 2 chiếc
        Self {
            stock: Mutex::new(stock),
        }
    }

    /// Tạm giữ 1 đơn vị sản phẩm trong kho
    pub fn reserve_item(&self, item_id: u32) -> Result<(), &'static str> {
        let mut guard = self.stock.lock().unwrap();
        if let Some(count) = guard.get_mut(&item_id) {
            if *count > 0 {
                *count -= 1;
                return Ok(());
            }
        }
        Err("Sản phẩm đã hết hàng trong kho phân tán!")
    }

    pub fn get_available_stock(&self, item_id: u32) -> u32 {
        let guard = self.stock.lock().unwrap();
        guard.get(&item_id).copied().unwrap_or(0)
    }
}

/// Động cơ Xử lý Đơn hàng Phân tán Hợp nhất (Distributed Order Engine)
pub struct DistributedOrderEngine {
    orders: Mutex<HashMap<u64, OrderEntity>>,
    idempotency_keys: Mutex<HashMap<String, u64>>,
    inventory: Arc<InventoryManager>,
    wal_file: Mutex<File>,
    wal_path: String,
}

impl DistributedOrderEngine {
    /// Mở hoặc tạo mới động cơ xử lý đơn hàng với tệp nhật ký WAL chỉ định
    pub fn open<P: AsRef<Path>>(wal_path: P, inventory: Arc<InventoryManager>) -> io::Result<Self> {
        let path_str = wal_path.as_ref().to_str().unwrap().to_string();

        let file = OpenOptions::new()
            .read(true)
            .write(true)
            .create(true)
            .append(true)
            .open(&wal_path)?;

        let mut engine = Self {
            orders: Mutex::new(HashMap::new()),
            idempotency_keys: Mutex::new(HashMap::new()),
            inventory,
            wal_file: Mutex::new(file),
            wal_path: path_str,
        };

        // Tự động khôi phục dữ liệu từ tệp WAL nếu tệp đã tồn tại từ phiên trước
        engine.recover_from_wal()?;
        Ok(engine)
    }

    /// Quét lại toàn bộ tệp nhật ký WAL để phục hồi trạng thái sau sự cố (Crash Recovery)
    fn recover_from_wal(&mut self) -> io::Result<()> {
        let file = File::open(&self.wal_path)?;
        let reader = BufReader::new(file);

        let mut recovered_count = 0;
        for line in reader.lines() {
            let record = line?;
            let parts: Vec<&str> = record.split('|').collect();
            if parts.len() == 5 {
                let order_id = parts[0].parse::<u64>().unwrap_or(0);
                let customer_id = parts[1].parse::<u64>().unwrap_or(0);
                let item_id = parts[2].parse::<u32>().unwrap_or(0);
                let amount_cents = parts[3].parse::<u64>().unwrap_or(0);
                let status = OrderStatus::from_str(parts[4]).unwrap_or(OrderStatus::Pending);

                let order = OrderEntity {
                    order_id,
                    customer_id,
                    item_id,
                    amount_cents,
                    status,
                };

                self.orders.lock().unwrap().insert(order_id, order);
                recovered_count += 1;
            }
        }

        if recovered_count > 0 {
            println!("    [Crash Recovery] Đã phục hồi thành công {} đơn hàng từ tệp nhật ký WAL!", recovered_count);
        }
        Ok(())
    }

    /// Ghi sự kiện đơn hàng nối đuôi vào đĩa cứng (Write-Ahead Log Append)
    fn log_event_to_disk(&self, order: &OrderEntity) -> io::Result<()> {
        let mut file_guard = self.wal_file.lock().unwrap();
        let log_line = format!(
            "{}|{}|{}|{}|{}\n",
            order.order_id,
            order.customer_id,
            order.item_id,
            order.amount_cents,
            order.status.as_str()
        );
        file_guard.write_all(log_line.as_bytes())?;
        file_guard.flush()?;
        Ok(())
    }

    /// Tiếp nhận và xử lý đơn hàng mới kèm cơ chế chống trùng lặp Idempotency
    pub fn submit_order(
        &self,
        idempotency_key: &str,
        order_id: u64,
        customer_id: u64,
        item_id: u32,
        amount_cents: u64,
    ) -> Result<OrderEntity, &'static str> {
        // 1. Kiểm tra an toàn dữ liệu đầu vào (Input Sanitization)
        if amount_cents == 0 {
            return Err("Giá trị đơn hàng không hợp lệ (Phải lớn hơn 0)!");
        }

        // 2. Kiểm tra khóa chống trùng lặp (Idempotency Key Check)
        {
            let mut idemp_guard = self.idempotency_keys.lock().unwrap();
            if let Some(&existing_order_id) = idemp_guard.get(idempotency_key) {
                println!(
                    "    [Idempotency] Phát hiện yêu cầu trùng lặp (Key: '{}')! Trả về đơn hàng cũ #{}",
                    idempotency_key, existing_order_id
                );
                let orders_guard = self.orders.lock().unwrap();
                return Ok(orders_guard.get(&existing_order_id).unwrap().clone());
            }
            idemp_guard.insert(idempotency_key.to_string(), order_id);
        }

        // 3. Khởi tạo đơn hàng ở trạng thái PENDING
        let mut order = OrderEntity {
            order_id,
            customer_id,
            item_id,
            amount_cents,
            status: OrderStatus::Pending,
        };

        // 4. Kiểm tra và tạm giữ kho hàng
        self.inventory.reserve_item(item_id)?;
        order.status = OrderStatus::Validated;

        // 5. Mô phỏng xử lý thanh toán thành công
        order.status = OrderStatus::Paid;

        // 6. Ghi bền vững trạng thái xuống đĩa cứng (WAL)
        self.log_event_to_disk(&order).map_err(|_| "Lỗi ghi đĩa nhật ký WAL")?;

        // 7. Lưu trữ trạng thái đơn hàng trên bộ nhớ RAM
        {
            let mut orders_guard = self.orders.lock().unwrap();
            orders_guard.insert(order_id, order.clone());
        }

        println!(
            "    [OrderEngine] Đơn hàng #{} đã được xử lý phân tán an toàn: Trạng thái {}",
            order_id,
            order.status.as_str()
        );

        Ok(order)
    }

    /// Hoàn tất xuất kho đơn hàng (Fulfill Order)
    pub fn fulfill_order(&self, order_id: u64) -> Result<(), &'static str> {
        let mut orders_guard = self.orders.lock().unwrap();
        if let Some(order) = orders_guard.get_mut(&order_id) {
            if order.status == OrderStatus::Paid {
                order.status = OrderStatus::Fulfilled;
                let _ = self.log_event_to_disk(order);
                println!("    [OrderEngine] Đơn hàng #{} đã XUẤT KHO THÀNH CÔNG (Fulfilled)!", order_id);
                return Ok(());
            } else {
                return Err("Đơn hàng chưa thanh toán, không thể xuất kho!");
            }
        }
        Err("Không tìm thấy đơn hàng chỉ định")
    }

    pub fn total_orders(&self) -> usize {
        self.orders.lock().unwrap().len()
    }
}

fn main() -> io::Result<()> {
    println!("==================================================================");
    println!("   ĐẠI DỰ ÁN TỐT NGHIỆP: ĐỘNG CƠ XỬ LÝ ĐƠN HÀNG PHÂN TÁN RUST    ");
    println!("==================================================================");

    let wal_file_path = "capstone_orders.wal";
    let _ = std::fs::remove_file(wal_file_path); // Dọn dẹp tệp thử nghiệm cũ

    let inventory = Arc::new(InventoryManager::new());

    // -------------------------------------------------------------
    // GIAI ĐOẠN 1: TIẾP NHẬN ĐƠN HÀNG VÀ CHỐNG TRÙNG LẶP IDEMPOTENCY
    // -------------------------------------------------------------
    println!("\n[1] Khoi tao dong co va tiep nhan don hang dau tien:");
    {
        let engine = DistributedOrderEngine::open(wal_file_path, Arc::clone(&inventory))?;

        println!("    - So luong ton kho San pham #101 ban dau: {} chiec", inventory.get_available_stock(101));

        // Khách hàng đặt hàng với Idempotency Key
        let idemp_key = "CLIENT_REQ_UUID_001";
        let order1 = engine.submit_order(idemp_key, 1001, 888, 101, 750_000).unwrap();
        println!("    - Don hang #{} da tao thanh cong!", order1.order_id);

        println!("    - So luong ton kho San pham #101 sau khi dat: {} chiec", inventory.get_available_stock(101));
        assert_eq!(inventory.get_available_stock(101), 9);

        // Khách hàng bị lag mạng và gửi lại chính xác Idempotency Key đó
        println!("\n    - Thu gui lai chinh xac yeu cau voi Idempotency Key '{}':", idemp_key);
        let duplicate_order = engine.submit_order(idemp_key, 1001, 888, 101, 750_000).unwrap();
        assert_eq!(duplicate_order.order_id, 1001);
        assert_eq!(inventory.get_available_stock(101), 9); // Kho KHÔNG bị trừ lần 2!
        println!("    => Idempotency Engine da chan dung viec tru tien va tru kho trung lap!");

        // Tiến hành xuất kho
        engine.fulfill_order(1001).unwrap();

        // Đặt thêm đơn hàng thứ 2
        engine.submit_order("CLIENT_REQ_UUID_002", 1002, 999, 101, 1_200_000).unwrap();
        assert_eq!(engine.total_orders(), 2);
    } // engine đóng tệp an toàn tại đây

    // -------------------------------------------------------------
    // GIAI ĐOẠN 2: KIỂM THỬ PHỤC HỒI SAU SỰ CỐ SẬP MÁY CHỦ (CRASH RECOVERY)
    // -------------------------------------------------------------
    println!("\n[2] Gia lap su co sap may chu toan dien va khoi dong lai:");
    {
        // Mở lại động cơ từ chính tệp nhật ký WAL
        let recovered_engine = DistributedOrderEngine::open(wal_file_path, Arc::clone(&inventory))?;

        println!("    - Tong so don hang phuc hoi tren RAM: {}", recovered_engine.total_orders());
        assert_eq!(recovered_engine.total_orders(), 2);

        // Kiểm tra chi tiết đơn hàng đã phục hồi
        let orders_guard = recovered_engine.orders.lock().unwrap();
        let restored_order_1 = orders_guard.get(&1001).unwrap();
        let restored_order_2 = orders_guard.get(&1002).unwrap();

        println!("    - Kiem tra Don #1001 sau phuc hoi: {:?}", restored_order_1.status);
        println!("    - Kiem tra Don #1002 sau phuc hoi: {:?}", restored_order_2.status);

        assert_eq!(restored_order_1.status, OrderStatus::Fulfilled);
        assert_eq!(restored_order_2.status, OrderStatus::Paid);
        println!("    => Phuc hoi toan ven trang thai may trang thai tu tep WAL thanh cong 100%!");
    }

    // Dọn dẹp tệp thử nghiệm
    let _ = std::fs::remove_file(wal_file_path);

    println!("\n==================================================================");
    println!("   CHUC MUNG BAN DA HOAN THANH XUAT SAC TOAN BO 50 CHUONG HOC!   ");
    println!("==================================================================");
    Ok(())
}
