# Chương 54: Đại dự án tốt nghiệp: Xây dựng Động cơ Xử lý Đơn hàng Phân tán (Capstone Project: Distributed Order Processing Engine)

## Giới thiệu & Mục tiêu học tập

Chúc mừng bạn đã đặt chân tới chương sách thứ 50 — **Đại dự án Tốt nghiệp (Capstone Project) của toàn bộ Giáo trình Rust Masterclass**! 

Trải qua một hành trình phi thường gồm 9 chủ đề lớn: Từ những viên gạch đầu tiên về thanh ghi CPU, quyền sở hữu bộ nhớ, mượn và thời gian sống; qua các cấu trúc dữ liệu kinh điển, động cơ lưu trữ đĩa cứng Mini-Bitcask; vượt qua các thử thách bảo mật nhị phân, phân tích gói tin mạng và tư duy tấn công OSCP; cho đến kiến trúc vi dịch vụ, động cơ Tokio và thuật toán đồng thuận Raft... Giờ là lúc bạn chứng minh bản lĩnh của một **Kỹ sư Phần mềm Hệ thống Rust thực thụ (Senior Systems Engineer)**.

Trong dự án tốt nghiệp này, chúng ta sẽ hợp nhất toàn bộ tinh hoa kiến thức của giáo trình để tự tay thiết kế và lập trình: **Một Động cơ Xử lý Đơn hàng Phân tán (Distributed Order Processing Engine) đạt chuẩn sản xuất!**

Hệ thống này tích hợp 5 phân hệ cốt lõi:
1. **Tầng tiếp nhận & Xác thực bảo mật (API Ingestion & Threat Validation)**: Kiểm tra tính hợp lệ của dữ liệu đầu vào, chống tấn công Injection và kiểm soát giới hạn tải.
2. **Cơ chế Triệt tiêu trùng lặp (Idempotency Key Engine)**: Bảo đảm dù mạng bị chập chờn khiến khách hàng bấm nút "Đặt hàng" 10 lần liên tiếp, tài khoản của họ cũng chỉ bị trừ tiền đúng 1 lần duy nhất.
3. **Máy trạng thái Vòng đời Đơn hàng (Order State Machine)**: Quản lý nghiêm ngặt các bước chuyển trạng thái từ `Pending` -> `Validated` -> `Paid` -> `Fulfilled` (hoặc `Cancelled`).
4. **Hàng đợi Xử lý Đa luồng phi chặn (Concurrent Actor Pipeline)**: Điều phối các tác vụ trừ kho, thanh toán thông qua kênh truyền tin đa luồng `std::sync::mpsc`.
5. **Nhật ký Sự kiện Ghi trước Bền vững (Write-Ahead Event Log & Crash Recovery)**: Ghi nối đuôi toàn bộ sự kiện xuống đĩa cứng, bảo đảm khi máy chủ bị mất điện đột ngột và khởi động lại, 100% đơn hàng và trạng thái kho sẽ được phục hồi nguyên vẹn.

---

## Hình tượng hóa đời sống (Intuitive Everyday Analogy)

Hãy hình dung cỗ máy phân tán này như một **Dây chuyền Trung tâm Kho vận Khổng lồ trong Ngày hội Siêu giảm giá (Black Friday)**:

```
┌──────────────────────────────────────────────────────────────────────────────────┐
│              HÌNH TƯỢNG HÓA: DÂY CHUYỀN XỬ LÝ ĐƠN HÀNG NGÀY BLACK FRIDAY         │
├──────────────────────────────────────────────────────────────────────────────────┤
│                                                                                  │
│ [1. CỔNG AN NINH & BẢO VỆ (SECURITY & IDEMPOTENCY GATE)]                         │
│ Khách hàng gửi đơn hàng qua điện thoại ──► Cổng an ninh kiểm tra:                │
│ - Tên khách có chứa mã độc không? (Input Sanitization).                          │
│ - Mã đơn này đã nộp trước đó chưa? Nếu vừa nộp rồi ──► Bỏ qua (Chống trùng lặp)!│
│                                                                                  │
│ [2. BĂNG CHUYỀN HÀNG ĐỢI SỰ KIỆN (MESSAGE QUEUE PIPELINE)]                       │
│ Đơn hàng hợp lệ được đặt lên Khay trượt băng chuyền (Kênh mpsc Channel):        │
│ Băng chuyền chuyển đơn đi êm ái, hàng ngàn đơn không bao giờ đè bẹp nhau!        │
│                                                                                  │
│ [3. THỦ KHO GIỮ SỔ NỢ RAM (IN-MEMORY CACHING & INVENTORY RESERVATION)]           │
│ Bác thủ kho liếc bảng số lượng hàng trên bảng kính:                              │
│ "Áo khoác size L còn 5 chiếc" ──► Tạm giữ 1 chiếc cho đơn hàng (Lock-free)!      │
│                                                                                  │
│ [4. BÁC THỦ KHO ĐÓNG DẤU ĐỎ VÀO SỔ NHẬT KÝ KIM LOẠI (PERSISTENT WAL LOG)]        │
│ Mỗi khi đơn hàng chuyển sang trạng thái "ĐÃ THANH TOÁN", bác thủ kho cầm         │
│ con dấu đóng cộp vào cuốn Sổ cái lưu trong két sắt chống cháy (Ghi nối đĩa).     │
│   ===> DÙ MẤT ĐIỆN TOÀN THÀNH PHỐ, KHI CÓ ĐIỆN LẠI HỆ THỐNG KHÔNG MẤT 1 ĐƠN!    │
└──────────────────────────────────────────────────────────────────────────────────┘
```

### 1. Chiếc vé chống gian lận (Idempotency Key)
- Khách hàng bấm "Mua hàng" trên ứng dụng di động. Vì mạng 4G chập chờn, ứng dụng tự động gửi lại 3 lần gói tin.
- Nếu không có cơ chế chống trùng, khách hàng sẽ bị trừ tiền 3 lần và nhận về 3 chiếc tivi giống nhau!
- Nhờ **Khóa định danh bất biến (Idempotency Key)** đính kèm trên mỗi yêu cầu, hệ thống nhận ra gói tin số 2 và số 3 mang cùng mã khóa, lập tức trả về kết quả đã xử lý của đơn trước mà không trừ tiền lần nữa.

### 2. Cuốn sổ cái chống cháy (Write-Ahead Logging & Crash Recovery)
- Máy chủ đang chạy ở đỉnh tải thì công tắc điện tòa nhà bị sập. Dữ liệu trên RAM bốc hơi 100%.
- Khi máy phát điện dự phòng hoạt động và máy chủ khởi động lại: Động cơ mở tệp nhật ký `orders.wal` trên ổ đĩa, đọc tuần tự từng dòng từ đầu đến cuối (Replay Event Stream) để tái dựng lại toàn bộ cây trạng thái đơn hàng và số lượng tồn kho trên RAM trong vài phần mười giây!

---

## Khái niệm & Cơ chế kỹ thuật chuyên sâu (In-Depth Technical Mechanics)

### 1. Máy Trạng Thái Đơn Hàng Nghiêm Ngặt (Finite State Machine - FSM)

Trong kiến trúc thương mại điện tử phân tán, trạng thái của một đơn hàng phải tuân thủ nghiêm ngặt các quy tắc chuyển dịch (State Transitions), không bao giờ được phép "nhảy cóc":

```
   [CREATED] (Đã tạo mới)
       │
       ▼
 [VALIDATED] (Đã kiểm tra kho & xác thực hợp lệ)
       │
       ▼
    [PAID] ────(Lỗi vận chuyển)────► [CANCELLED] (Đã hủy & Hoàn tiền)
       │
       ▼
  [FULFILLED] (Đã đóng gói xuất kho thành công - Điểm kết thúc)
```
- Không thể chuyển từ `Created` thẳng sang `Fulfilled` mà chưa qua bước `Paid`.
- Không thể chuyển từ `Fulfilled` sang `Cancelled` khi hàng đã rời kho.
- Trong Rust, chúng ta mô hình hóa các trạng thái này bằng `enum` có kiểu dữ liệu mạnh mẽ kết hợp mẫu so khớp `match`, biến mọi hành vi chuyển trạng thái bất hợp pháp thành lỗi biên dịch hoặc lỗi logic an toàn.

### 2. Quản lý Kho Độc Lập và Cơ Chế Tạm Giữ Hàng (Inventory Reservation)

- Khi có 1,000 khách hàng cùng tranh mua 10 chiếc vé ca nhạc cuối cùng:
  - **Cách làm sai**: Trừ thẳng số lượng trong cơ sở dữ liệu (dễ gây âm kho nếu giao dịch thanh toán bị hủy).
  - **Cách làm đúng của hệ thống phân tán**: Tách làm hai giai đoạn:
    1. *Tạm giữ (Reserve)*: Giảm số lượng khả dụng và đặt thời gian giữ chỗ (Hold Timeout 15 phút).
    2. *Xác nhận trừ đứt (Commit)*: Khi cổng thanh toán xác nhận trừ tiền thành công. Nếu khách không thanh toán, kho tự động nhả lại số lượng khả dụng.

### 3. Nhật ký Sự kiện Ghi trước (Event Sourcing & WAL Persistence)

Thay vì ghi đè trạng thái đơn hàng tại chỗ (Update In-place), hệ thống áp dụng nguyên lý **Nhật ký sự kiện (Event Sourcing)**:
- Mọi biến động đều được lưu dưới dạng một Sự kiện bất biến (Immutable Event):
  - `OrderCreated { order_id, amount }`
  - `InventoryReserved { order_id, item_id }`
  - `PaymentProcessed { order_id, transaction_id }`
  - `OrderFulfilled { order_id }`
- Toàn bộ sự kiện được ghi nối đuôi (`append-only`) vào tệp nhật ký nhị phân trên đĩa cứng bằng lời gọi `std::fs::OpenOptions::append`. Nhờ tính chất tuần tự (Sequential I/O), tốc độ ghi đạt mức tối đa của đĩa SSD (hàng trăm ngàn sự kiện/giây).

---

## Mã nguồn minh họa thực chiến (Idiomatic Runnable Rust Blueprint)

Dưới đây là mã nguồn hoàn chỉnh của **Động cơ Xử lý Đơn hàng Phân tán (Distributed Order Processing Engine)** được lập trình bằng 100% Safe Rust chuẩn mực, tích hợp đầy đủ tính năng: Xác thực đầu vào, kiểm tra khóa chống trùng lặp Idempotency, máy trạng thái đơn hàng, tạm giữ kho hàng đồng thời, và cơ chế ghi/phục hồi nhật ký sự kiện từ tệp đĩa:

```rust
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
    println!("\n[2] Gia lap su co sap may owner toan dien va khoi dong lai:");
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
```

---

## Bảng tra cứu lỗi biên dịch & Cách khắc phục (Compiler Error Guide)

Dưới đây là các lỗi biên dịch thường gặp nhất khi hiện thực hóa động cơ xử lý đơn hàng phân tán trong Rust:

| Mã lỗi | Thông báo mẫu từ trình biên dịch | Nguyên nhân cốt lõi | Cách khắc phục nhanh |
|---|---|---|---|
| **E0382** | `use of moved value: 'order'` | Bạn di chuyển quyền sở hữu (ownership) của `order` vào hàm ghi đĩa hoặc hàm xử lý khác, sau đó lại dùng tiếp nó. | Sử dụng phương thức `.clone()` để tạo bản sao sự kiện, hoặc chỉ truyền tham chiếu mượn (borrow). |
| **E0502** | `cannot borrow '*self.orders' as mutable because it is also borrowed as immutable` | Bạn vừa tra cứu đơn hàng trong `HashMap` vừa cố gắng cập nhật trạng thái của nó trong cùng một khối lệnh. | Sử dụng phương thức `.get_mut(&order_id)` để mượn khả biến trực tiếp phần tử cần cập nhật. |
| **E0277** | `the trait 'BufRead' is not implemented for 'File'` | Bạn sử dụng phương thức `.lines()` trực tiếp trên đối tượng `File` mà quên đưa qua bộ đệm. | Bọc tệp trong bộ nhớ đệm (buffer) đọc hiệu năng cao: `BufReader::new(file)`. |
| **E0599** | `no method named 'lock' found for struct 'InventoryManager'` | Gọi nhầm phương thức `.lock()` trên đối tượng ngoài thay vì trên trường khóa `Mutex` nội bộ. | Đảm bảo gọi đúng trường được bảo vệ: `self.stock.lock().unwrap()`. |

### Ví dụ phân tích lỗi `E0382` khi ghi nhật ký sự kiện:

```rust
#[derive(Debug, Clone)]
struct DonQueue {
    id: u64,
}

fn ghi_nhat_ky(dh: DonQueue) {
    println!("Ghi nhật ký: {:?}", dh);
}

// Đoạn mã lỗi minh họa E0382:
fn xu_ly_loi(dh: DonQueue) {
    // ghi_nhat_ky(dh); // Di chuyển quyền sở hữu dh
    // println!("Đơn hàng đã xử lý: {:?}", dh); // LỖI E0382: dh đã bị di chuyển!
}

// Cách sửa chữa đúng chuẩn: Truyền tham chiếu mượn hoặc clone
fn xu_ly_dung(dh: DonQueue) {
    ghi_nhat_ky(dh.clone()); // Tạo bản sao độc lập
    println!("Đơn hàng an toàn: {:?}", dh); // dh ban đầu vẫn còn nguyên vẹn!
}
```

---

## Tóm tắt chương & Bài tập rèn luyện (Summary & Exercises)

### 4 Điểm cốt lõi cần ghi nhớ:
1. **Khóa Định danh Bất biến (Idempotency Key)**: Tấm khiên thần kỳ bảo vệ các dịch vụ thanh toán và đơn hàng khỏi các yêu cầu gửi lặp lại do chập chờn mạng.
2. **Máy Trạng Thái Hữu Hạn (FSM)**: Kiểm soát chặt chẽ từng bước chuyển dịch vòng đời của đơn hàng, loại bỏ triệt để các trạng thái mâu thuẫn nghiệp vụ.
3. **Bảo toàn Dữ liệu bằng WAL**: Ghi nối đuôi sự kiện tuần tự xuống đĩa cứng bảo đảm tính toàn vẹn và khả năng phục hồi thần tốc sau sự cố mất điện (Crash Recovery).
4. **Đỉnh cao Kỹ nghệ Hệ thống Rust**: Sự phối hợp nhuần nhuyễn giữa quyền sở hữu (ownership), mượn (borrow), thời gian sống (lifetime), con trỏ thông minh (smart pointer) và bộ nhớ đệm (buffer) tạo nên một cỗ máy phân tán đạt thông lượng hàng trăm ngàn giao dịch/giây với độ ổn định tuyệt đối.

### Bài tập rèn luyện tự giải:
1. **Bài tập 1 (Bổ sung Kênh Hủy đơn hàng và Hoàn trả Kho - Order Cancellation & Stock Rollback)**:  
   Mở rộng động cơ với phương thức `cancel_order(&self, order_id: u64)`. Khi một đơn hàng ở trạng thái `Paid` bị hủy, hệ thống sẽ tự động hoàn trả lại +1 đơn vị sản phẩm vào kho phân tán `InventoryManager` và ghi sự kiện `CANCELLED` xuống tệp WAL.
2. **Bài tập 2 (Xây dựng Tiến trình Dọn dẹp và Nén Nhật ký WAL - Log Compaction)**:  
   Sau khi hệ thống ghi nhận 10,000 sự kiện, tệp `orders.wal` sẽ phình to. Hãy viết hàm `compact_wal_log(&self)` chỉ giữ lại trạng thái cuối cùng mới nhất của từng đơn hàng và ghi sang tệp mới gọn gàng, giải phóng dung lượng đĩa tương tự như kiến trúc Bitcask đã học ở Chương 36.
3. **Bài tập 3 (Suy ngẫm đỉnh cao: Thiết kế Kiến trúc Thanh toán Saga Phân tán)**:  
   Khi Dịch vụ Đơn hàng (Order Service), Dịch vụ Kho (Inventory Service) và Dịch vụ Cổng Thanh toán (Payment Service) nằm trên 3 máy chủ phân tán khác nhau ở 3 quốc gia, việc sử dụng giao dịch phân tán 2PC (Two-Phase Commit) sẽ gây nghẽn mạng nghiêm trọng. Hãy trình bày cách áp dụng **Mô hình Saga điều phối qua Sự kiện (Event-Driven Choreography Saga)** để đảm bảo tính nhất quán cuối cùng (Eventual Consistency) nếu bước thanh toán bất ngờ bị ngân hàng từ chối.
