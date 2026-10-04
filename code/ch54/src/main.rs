use std::collections::HashMap;
use std::fs::{File, OpenOptions};
use std::io::{self, BufRead, BufReader, Write};
use std::path::{Path, PathBuf};
use std::str::FromStr;
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

    /// Trạng thái này có đang chiếm một đơn vị hàng trong kho không
    pub fn holds_stock(&self) -> bool {
        matches!(self, OrderStatus::Paid | OrderStatus::Fulfilled)
    }
}

impl FromStr for OrderStatus {
    type Err = ();

    fn from_str(s: &str) -> Result<Self, ()> {
        match s {
            "PENDING" => Ok(OrderStatus::Pending),
            "VALIDATED" => Ok(OrderStatus::Validated),
            "PAID" => Ok(OrderStatus::Paid),
            "FULFILLED" => Ok(OrderStatus::Fulfilled),
            "CANCELLED" => Ok(OrderStatus::Cancelled),
            _ => Err(()),
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
    pub idempotency_key: String,
}

impl OrderEntity {
    /// Một dòng WAL: order_id|customer_id|item_id|amount_cents|STATUS|idempotency_key
    fn to_wal_line(&self) -> String {
        format!(
            "{}|{}|{}|{}|{}|{}\n",
            self.order_id,
            self.customer_id,
            self.item_id,
            self.amount_cents,
            self.status.as_str(),
            self.idempotency_key
        )
    }

    /// Phân tích một dòng WAL; dòng hỏng (ví dụ ghi dở khi mất điện) trả None
    fn from_wal_line(line: &str) -> Option<Self> {
        let parts: Vec<&str> = line.split('|').collect();
        if parts.len() != 6 || parts[5].is_empty() {
            return None;
        }
        Some(OrderEntity {
            order_id: parts[0].parse().ok()?,
            customer_id: parts[1].parse().ok()?,
            item_id: parts[2].parse().ok()?,
            amount_cents: parts[3].parse().ok()?,
            status: parts[4].parse().ok()?,
            idempotency_key: parts[5].to_string(),
        })
    }
}

/// Động cơ Lưu trữ và Quản lý Kho hàng đồng thời (Thread-Safe Inventory Store)
pub struct InventoryManager {
    stock: Mutex<HashMap<u32, u32>>,
}

impl Default for InventoryManager {
    fn default() -> Self {
        Self::new()
    }
}

impl InventoryManager {
    /// Kho mặc định của bản demo: sản phẩm 101 có 10 chiếc, 102 có 2 chiếc
    pub fn new() -> Self {
        Self::with_stock(&[(101, 10), (102, 2)])
    }

    pub fn with_stock(items: &[(u32, u32)]) -> Self {
        Self {
            stock: Mutex::new(items.iter().copied().collect()),
        }
    }

    /// Tạm giữ 1 đơn vị sản phẩm trong kho
    pub fn reserve_item(&self, item_id: u32) -> Result<(), &'static str> {
        let mut guard = self.stock.lock().unwrap();
        match guard.get_mut(&item_id) {
            Some(count) if *count > 0 => {
                *count -= 1;
                Ok(())
            }
            _ => Err("Sản phẩm đã hết hàng trong kho!"),
        }
    }

    /// Trả lại 1 đơn vị sản phẩm (hủy đơn, hoặc hoàn tác khi bước sau thất bại)
    pub fn release_item(&self, item_id: u32) {
        *self.stock.lock().unwrap().entry(item_id).or_insert(0) += 1;
    }

    pub fn get_available_stock(&self, item_id: u32) -> u32 {
        let guard = self.stock.lock().unwrap();
        guard.get(&item_id).copied().unwrap_or(0)
    }
}

/// Trạng thái của một khóa chống trùng lặp
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
enum IdempotencyState {
    /// Đang có một yêu cầu cùng khóa được xử lý
    InProgress,
    /// Đã xử lý THÀNH CÔNG, tạo ra đơn hàng này
    Completed(u64),
}

/// Động cơ Xử lý Đơn hàng Phân tán Hợp nhất (Distributed Order Engine)
pub struct DistributedOrderEngine {
    orders: Mutex<HashMap<u64, OrderEntity>>,
    idempotency_keys: Mutex<HashMap<String, IdempotencyState>>,
    inventory: Arc<InventoryManager>,
    wal_file: Mutex<File>,
    wal_path: PathBuf,
}

impl DistributedOrderEngine {
    /// Mở hoặc tạo mới động cơ xử lý đơn hàng với tệp nhật ký WAL chỉ định.
    /// `inventory` là kho ở trạng thái BAN ĐẦU (trước mọi đơn hàng): việc phát lại
    /// WAL sẽ trừ lại các đơn vị hàng mà những đơn đã thanh toán đang giữ.
    pub fn open<P: AsRef<Path>>(wal_path: P, inventory: Arc<InventoryManager>) -> io::Result<Self> {
        let file = OpenOptions::new()
            .read(true)
            .create(true)
            .append(true)
            .open(&wal_path)?;

        let mut engine = Self {
            orders: Mutex::new(HashMap::new()),
            idempotency_keys: Mutex::new(HashMap::new()),
            inventory,
            wal_file: Mutex::new(file),
            wal_path: wal_path.as_ref().to_path_buf(),
        };

        // Tự động khôi phục dữ liệu từ tệp WAL nếu tệp đã tồn tại từ phiên trước
        engine.recover_from_wal()?;
        Ok(engine)
    }

    /// Phát lại toàn bộ tệp nhật ký WAL để phục hồi trạng thái sau sự cố (Crash Recovery):
    /// đơn hàng (bản ghi sau đè bản ghi trước), khóa chống trùng lặp, và tồn kho.
    fn recover_from_wal(&mut self) -> io::Result<()> {
        let reader = BufReader::new(File::open(&self.wal_path)?);

        let orders = self.orders.get_mut().unwrap();
        let keys = self.idempotency_keys.get_mut().unwrap();
        let mut skipped = 0;
        for line in reader.lines() {
            match OrderEntity::from_wal_line(&line?) {
                Some(order) => {
                    keys.insert(
                        order.idempotency_key.clone(),
                        IdempotencyState::Completed(order.order_id),
                    );
                    orders.insert(order.order_id, order);
                }
                // Dòng cuối ghi dở khi mất điện: bỏ qua, KHÔNG đoán bừa giá trị 0
                None => skipped += 1,
            }
        }

        // Tồn kho không được lưu riêng: suy ra từ các đơn đang giữ hàng
        for order in orders.values().filter(|o| o.status.holds_stock()) {
            if self.inventory.reserve_item(order.item_id).is_err() {
                println!(
                    "    [Crash Recovery] CẢNH BÁO: kho ban đầu không đủ cho đơn #{}",
                    order.order_id
                );
            }
        }

        if !orders.is_empty() || skipped > 0 {
            println!(
                "    [Crash Recovery] Đã phục hồi {} đơn hàng từ WAL (bỏ qua {} dòng hỏng)",
                orders.len(),
                skipped
            );
        }
        Ok(())
    }

    /// Ghi sự kiện đơn hàng nối đuôi vào đĩa cứng (Write-Ahead Log Append).
    /// `flush()` chỉ đẩy dữ liệu từ bộ đệm của chương trình xuống hệ điều hành (page
    /// cache) — mất điện vẫn mất. `sync_data()` (fsync/fdatasync) mới chờ tới khi dữ
    /// liệu thực sự nằm trên đĩa; chỉ sau đó mới được báo "thành công" cho khách.
    fn log_event_to_disk(&self, order: &OrderEntity) -> io::Result<()> {
        let mut file_guard = self.wal_file.lock().unwrap();
        file_guard.write_all(order.to_wal_line().as_bytes())?;
        file_guard.sync_data()?;
        Ok(())
    }

    /// Kiểm tra khóa chống trùng lặp: không rỗng, không chứa ký tự phân cách/điều khiển
    /// (chống "tiêm" dòng giả vào WAL — một dạng Injection).
    fn validate_key(key: &str) -> Result<(), &'static str> {
        if key.is_empty() || key.len() > 64 || key.chars().any(|c| c == '|' || c.is_control()) {
            return Err("Khóa Idempotency không hợp lệ!");
        }
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
        Self::validate_key(idempotency_key)?;

        // 2. Giành quyền xử lý khóa (Idempotency Key Check)
        {
            let mut keys = self.idempotency_keys.lock().unwrap();
            match keys.get(idempotency_key).copied() {
                Some(IdempotencyState::Completed(existing_order_id)) => {
                    drop(keys);
                    println!(
                        "    [Idempotency] Phát hiện yêu cầu trùng lặp (Key: '{}')! Trả về đơn hàng cũ #{}",
                        idempotency_key, existing_order_id
                    );
                    return self
                        .orders
                        .lock()
                        .unwrap()
                        .get(&existing_order_id)
                        .cloned()
                        .ok_or("Không tìm thấy đơn hàng ứng với khóa Idempotency");
                }
                Some(IdempotencyState::InProgress) => {
                    return Err("Yêu cầu cùng khóa đang được xử lý, vui lòng thử lại sau!");
                }
                None => {
                    keys.insert(idempotency_key.to_string(), IdempotencyState::InProgress);
                }
            }
        }

        // 3. Xử lý; chỉ đánh dấu khóa "đã xong" khi đơn hàng THỰC SỰ tồn tại.
        //    Thất bại -> xóa khóa để khách gửi lại được (thay vì kẹt khóa mãi mãi).
        let result = self.process_new_order(
            idempotency_key,
            order_id,
            customer_id,
            item_id,
            amount_cents,
        );
        let mut keys = self.idempotency_keys.lock().unwrap();
        match &result {
            Ok(order) => {
                keys.insert(
                    idempotency_key.to_string(),
                    IdempotencyState::Completed(order.order_id),
                );
            }
            Err(_) => {
                keys.remove(idempotency_key);
            }
        }
        result
    }

    fn process_new_order(
        &self,
        idempotency_key: &str,
        order_id: u64,
        customer_id: u64,
        item_id: u32,
        amount_cents: u64,
    ) -> Result<OrderEntity, &'static str> {
        // Giữ khóa `orders` suốt từ kiểm tra tới khi ghi WAL + cập nhật RAM (thứ tự khóa
        // luôn là orders -> wal_file, giống `fulfill_order`): WAL và RAM không bao giờ lệch
        // nhau, kể cả khi có tiến trình nén WAL chạy song song. Cái giá là các lần ghi bị
        // tuần tự hóa qua fsync — hệ thống thật dùng "group commit" để gộp nhiều sự kiện
        // vào một lần fsync.
        let mut orders = self.orders.lock().unwrap();
        if orders.contains_key(&order_id) {
            return Err("Mã đơn hàng đã tồn tại!");
        }

        // Khởi tạo đơn hàng ở trạng thái PENDING
        let mut order = OrderEntity {
            order_id,
            customer_id,
            item_id,
            amount_cents,
            status: OrderStatus::Pending,
            idempotency_key: idempotency_key.to_string(),
        };

        // Kiểm tra và tạm giữ kho hàng
        self.inventory.reserve_item(item_id)?;
        order.status = OrderStatus::Validated;

        // Mô phỏng xử lý thanh toán thành công
        order.status = OrderStatus::Paid;

        // Ghi bền vững trạng thái xuống đĩa cứng (WAL) TRƯỚC khi xác nhận
        if self.log_event_to_disk(&order).is_err() {
            self.inventory.release_item(item_id); // hoàn tác phần đã giữ kho
            return Err("Lỗi ghi đĩa nhật ký WAL");
        }

        // Lưu trữ trạng thái đơn hàng trên bộ nhớ RAM
        orders.insert(order_id, order.clone());

        println!(
            "    [OrderEngine] Đơn hàng #{} đã được xử lý an toàn: Trạng thái {}",
            order_id,
            order.status.as_str()
        );

        Ok(order)
    }

    /// Hoàn tất xuất kho đơn hàng (Fulfill Order)
    pub fn fulfill_order(&self, order_id: u64) -> Result<(), &'static str> {
        let mut orders_guard = self.orders.lock().unwrap();
        let order = orders_guard
            .get_mut(&order_id)
            .ok_or("Không tìm thấy đơn hàng chỉ định")?;
        if order.status != OrderStatus::Paid {
            return Err("Đơn hàng chưa thanh toán, không thể xuất kho!");
        }

        // Ghi WAL trước, thành công mới đổi trạng thái trên RAM
        let mut updated = order.clone();
        updated.status = OrderStatus::Fulfilled;
        self.log_event_to_disk(&updated)
            .map_err(|_| "Lỗi ghi đĩa nhật ký WAL")?;
        *order = updated;
        println!(
            "    [OrderEngine] Đơn hàng #{} đã XUẤT KHO THÀNH CÔNG (Fulfilled)!",
            order_id
        );
        Ok(())
    }

    pub fn get_order(&self, order_id: u64) -> Option<OrderEntity> {
        self.orders.lock().unwrap().get(&order_id).cloned()
    }

    pub fn total_orders(&self) -> usize {
        self.orders.lock().unwrap().len()
    }
}

fn main() -> io::Result<()> {
    println!("==================================================================");
    println!("   ĐẠI DỰ ÁN TỐT NGHIỆP: ĐỘNG CƠ XỬ LÝ ĐƠN HÀNG PHÂN TÁN RUST    ");
    println!("==================================================================");

    let wal_file_path = std::env::temp_dir().join("capstone_orders.wal");
    let _ = std::fs::remove_file(&wal_file_path); // Dọn dẹp tệp thử nghiệm cũ

    // -------------------------------------------------------------
    // GIAI ĐOẠN 1: TIẾP NHẬN ĐƠN HÀNG VÀ CHỐNG TRÙNG LẶP IDEMPOTENCY
    // -------------------------------------------------------------
    println!("\n[1] Khởi tạo động cơ và tiếp nhận đơn hàng đầu tiên:");
    {
        let inventory = Arc::new(InventoryManager::new());
        let engine = DistributedOrderEngine::open(&wal_file_path, Arc::clone(&inventory))?;

        println!(
            "    - Số lượng tồn kho Sản phẩm #101 ban đầu: {} chiếc",
            inventory.get_available_stock(101)
        );

        // Khách hàng đặt hàng với Idempotency Key
        let idemp_key = "CLIENT_REQ_UUID_001";
        let order1 = engine
            .submit_order(idemp_key, 1001, 888, 101, 750_000)
            .unwrap();
        println!("    - Đơn hàng #{} đã tạo thành công!", order1.order_id);
        assert_eq!(inventory.get_available_stock(101), 9);

        // Khách hàng bị lag mạng và gửi lại chính xác Idempotency Key đó
        println!(
            "\n    - Thử gửi lại chính xác yêu cầu với Idempotency Key '{}':",
            idemp_key
        );
        let duplicate_order = engine
            .submit_order(idemp_key, 1001, 888, 101, 750_000)
            .unwrap();
        assert_eq!(duplicate_order.order_id, 1001);
        assert_eq!(inventory.get_available_stock(101), 9); // Kho KHÔNG bị trừ lần 2!
        println!("    => Idempotency Engine đã chặn đứng việc trừ tiền và trừ kho trùng lặp!");

        // Đơn cho sản phẩm hết hàng thất bại; gửi lại cùng khóa cũng chỉ báo lỗi, không panic
        println!("\n    - Đặt sản phẩm #999 (không có trong kho), rồi gửi lại cùng khóa:");
        for _ in 0..2 {
            let result = engine.submit_order("CLIENT_REQ_UUID_003", 1003, 777, 999, 99_000);
            println!("      -> {:?}", result.map(|o| o.order_id));
        }

        // Tiến hành xuất kho
        engine.fulfill_order(1001).unwrap();

        // Đặt thêm đơn hàng thứ 2
        engine
            .submit_order("CLIENT_REQ_UUID_002", 1002, 999, 101, 1_200_000)
            .unwrap();
        assert_eq!(engine.total_orders(), 2);
        assert_eq!(inventory.get_available_stock(101), 8);
    } // engine và kho trên RAM "chết" tại đây

    // -------------------------------------------------------------
    // GIAI ĐOẠN 2: KIỂM THỬ PHỤC HỒI SAU SỰ CỐ SẬP MÁY CHỦ (CRASH RECOVERY)
    // -------------------------------------------------------------
    println!("\n[2] Giả lập sự cố sập máy chủ toàn diện và khởi động lại:");
    {
        // RAM đã mất: kho khởi động lại từ số lượng BAN ĐẦU, WAL phải dựng lại phần còn lại
        let inventory = Arc::new(InventoryManager::new());
        let recovered_engine =
            DistributedOrderEngine::open(&wal_file_path, Arc::clone(&inventory))?;

        println!(
            "    - Tổng số đơn hàng phục hồi trên RAM: {}",
            recovered_engine.total_orders()
        );
        assert_eq!(recovered_engine.total_orders(), 2);

        let restored_order_1 = recovered_engine.get_order(1001).unwrap();
        let restored_order_2 = recovered_engine.get_order(1002).unwrap();
        println!(
            "    - Kiểm tra Đơn #1001 sau phục hồi: {:?}",
            restored_order_1.status
        );
        println!(
            "    - Kiểm tra Đơn #1002 sau phục hồi: {:?}",
            restored_order_2.status
        );
        assert_eq!(restored_order_1.status, OrderStatus::Fulfilled);
        assert_eq!(restored_order_2.status, OrderStatus::Paid);

        println!(
            "    - Tồn kho #101 sau phục hồi: {} chiếc",
            inventory.get_available_stock(101)
        );
        assert_eq!(inventory.get_available_stock(101), 8);

        // Khóa Idempotency cũng được phục hồi: gửi lại yêu cầu cũ không tạo đơn mới
        let replay = recovered_engine
            .submit_order("CLIENT_REQ_UUID_001", 1001, 888, 101, 750_000)
            .unwrap();
        assert_eq!(replay.order_id, 1001);
        assert_eq!(inventory.get_available_stock(101), 8);
        println!("    => Đơn hàng, tồn kho và khóa chống trùng lặp đều được phục hồi từ WAL!");
    }

    // Dọn dẹp tệp thử nghiệm
    let _ = std::fs::remove_file(&wal_file_path);

    println!("\n==================================================================");
    println!("   CHÚC MỪNG BẠN ĐÃ HOÀN THÀNH ĐẠI DỰ ÁN CHỦ ĐỀ HỆ THỐNG PHÂN TÁN! ");
    println!("==================================================================");
    Ok(())
}

#[cfg(test)]
mod tests {
    use super::*;
    use std::sync::atomic::{AtomicUsize, Ordering};

    fn temp_wal(name: &str) -> PathBuf {
        static COUNTER: AtomicUsize = AtomicUsize::new(0);
        let n = COUNTER.fetch_add(1, Ordering::SeqCst);
        let path =
            std::env::temp_dir().join(format!("ch54_{}_{}_{}.wal", name, std::process::id(), n));
        let _ = std::fs::remove_file(&path);
        path
    }

    #[test]
    fn failed_order_does_not_burn_key_and_retry_does_not_panic() {
        let path = temp_wal("retry");
        let inventory = Arc::new(InventoryManager::with_stock(&[(5, 0)]));
        let engine = DistributedOrderEngine::open(&path, Arc::clone(&inventory)).unwrap();

        // Hết hàng: lỗi. Bản cũ đã ghi khóa trước -> lần gửi lại panic ở unwrap()
        assert!(engine.submit_order("K1", 1, 1, 5, 100).is_err());
        assert!(engine.submit_order("K1", 1, 1, 5, 100).is_err());

        // Có hàng trở lại: cùng khóa giờ phải xử lý được
        inventory.release_item(5);
        let order = engine.submit_order("K1", 1, 1, 5, 100).unwrap();
        assert_eq!(order.status, OrderStatus::Paid);
        assert_eq!(inventory.get_available_stock(5), 0);
        let _ = std::fs::remove_file(&path);
    }

    #[test]
    fn recovery_restores_keys_and_inventory() {
        let path = temp_wal("recover");
        {
            let inventory = Arc::new(InventoryManager::with_stock(&[(7, 3)]));
            let engine = DistributedOrderEngine::open(&path, inventory).unwrap();
            engine.submit_order("A", 1, 1, 7, 10).unwrap();
            engine.submit_order("B", 2, 1, 7, 10).unwrap();
            engine.fulfill_order(1).unwrap();
        }
        let inventory = Arc::new(InventoryManager::with_stock(&[(7, 3)]));
        let engine = DistributedOrderEngine::open(&path, Arc::clone(&inventory)).unwrap();
        assert_eq!(inventory.get_available_stock(7), 1);
        assert_eq!(engine.get_order(1).unwrap().status, OrderStatus::Fulfilled);
        // Khóa "A" đã phục hồi: không trừ kho lần nữa
        assert_eq!(engine.submit_order("A", 1, 1, 7, 10).unwrap().order_id, 1);
        assert_eq!(inventory.get_available_stock(7), 1);
        let _ = std::fs::remove_file(&path);
    }

    #[test]
    fn torn_last_line_is_skipped() {
        let path = temp_wal("torn");
        std::fs::write(&path, "1|1|7|10|PAID|A\n2|1|7|1").unwrap();
        let inventory = Arc::new(InventoryManager::with_stock(&[(7, 5)]));
        let engine = DistributedOrderEngine::open(&path, Arc::clone(&inventory)).unwrap();
        assert_eq!(engine.total_orders(), 1);
        assert_eq!(inventory.get_available_stock(7), 4);
        let _ = std::fs::remove_file(&path);
    }

    #[test]
    fn concurrent_duplicates_reserve_once() {
        let path = temp_wal("concurrent");
        let inventory = Arc::new(InventoryManager::with_stock(&[(9, 100)]));
        let engine = Arc::new(DistributedOrderEngine::open(&path, Arc::clone(&inventory)).unwrap());
        let handles: Vec<_> = (0..16)
            .map(|_| {
                let engine = Arc::clone(&engine);
                std::thread::spawn(move || engine.submit_order("SAME", 42, 1, 9, 10))
            })
            .collect();
        for h in handles {
            let _ = h.join().unwrap(); // Ok (đơn cũ) hoặc Err "đang xử lý", không panic
        }
        assert_eq!(inventory.get_available_stock(9), 99);
        assert_eq!(engine.total_orders(), 1);
        let _ = std::fs::remove_file(&path);
    }

    #[test]
    fn key_with_separator_is_rejected() {
        let path = temp_wal("inject");
        let engine =
            DistributedOrderEngine::open(&path, Arc::new(InventoryManager::new())).unwrap();
        assert!(engine.submit_order("A|B", 1, 1, 101, 10).is_err());
        assert!(
            engine
                .submit_order("A\n2|1|101|1|PAID|X", 1, 1, 101, 10)
                .is_err()
        );
        let _ = std::fs::remove_file(&path);
    }
}
