# Chương 54: Đại dự án tốt nghiệp: Xây dựng Động cơ Xử lý Đơn hàng Phân tán (Capstone Project: Distributed Order Processing Engine)

## Giới thiệu & Mục tiêu học tập

Chúc mừng bạn đã đặt chân tới Chương 54 — **Đại dự án Tốt nghiệp (Capstone Project) của chủ đề Hệ thống phân tán trong Giáo trình Rust Masterclass**! 

Trải qua một hành trình phi thường gồm 9 chủ đề lớn: Từ những viên gạch đầu tiên về thanh ghi CPU, quyền sở hữu bộ nhớ, mượn và thời gian sống; qua các cấu trúc dữ liệu kinh điển, động cơ lưu trữ đĩa cứng Mini-Bitcask; vượt qua các thử thách bảo mật nhị phân, phân tích gói tin mạng và tư duy tấn công OSCP; cho đến kiến trúc vi dịch vụ, động cơ Tokio và thuật toán đồng thuận Raft... Giờ là lúc bạn chứng minh bản lĩnh của một **Kỹ sư Phần mềm Hệ thống Rust thực thụ (Senior Systems Engineer)**.

Trong dự án tốt nghiệp này, chúng ta sẽ hợp nhất toàn bộ tinh hoa kiến thức của giáo trình để tự tay thiết kế và lập trình: **Một Động cơ Xử lý Đơn hàng Phân tán (Distributed Order Processing Engine) đạt chuẩn sản xuất!**

Hệ thống này tích hợp 5 phân hệ cốt lõi:
1. **Tầng tiếp nhận & Xác thực bảo mật (API Ingestion & Threat Validation)**: Kiểm tra tính hợp lệ của dữ liệu đầu vào, chống tấn công Injection và kiểm soát giới hạn tải.
2. **Cơ chế Triệt tiêu trùng lặp (Idempotency Key Engine)**: Bảo đảm dù mạng bị chập chờn khiến khách hàng bấm nút "Đặt hàng" 10 lần liên tiếp, tài khoản của họ cũng chỉ bị trừ tiền đúng 1 lần duy nhất.
3. **Máy trạng thái Vòng đời Đơn hàng (Order State Machine)**: Quản lý nghiêm ngặt các bước chuyển trạng thái từ `Pending` -> `Validated` -> `Paid` -> `Fulfilled` (hoặc `Cancelled`).
4. **Xử lý Đồng thời An toàn (Concurrent Processing)**: Động cơ được chia sẻ qua `Arc`, nhiều luồng gọi `submit_order` cùng lúc; trạng thái dùng chung được bảo vệ bằng `Mutex` với thứ tự khóa cố định. (Biến nó thành một pipeline actor với `std::sync::mpsc` như Chương 50 là một hướng mở rộng hay.)
5. **Nhật ký Sự kiện Ghi trước Bền vững (Write-Ahead Event Log & Crash Recovery)**: Ghi nối đuôi toàn bộ sự kiện xuống đĩa cứng và `fsync` trước khi xác nhận, bảo đảm khi máy chủ bị mất điện đột ngột và khởi động lại, mọi đơn hàng đã được xác nhận cho khách — cùng tồn kho và khóa chống trùng lặp suy ra từ chúng — sẽ được phục hồi nguyên vẹn.

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
│ Băng chuyền chuyển đơn đi êm ái, hàng ngàn đơn không đè bẹp nhau!                │
│                                                                                  │
│ [3. THỦ KHO GIỮ SỔ NỢ RAM (IN-MEMORY CACHING & INVENTORY RESERVATION)]           │
│ Bác thủ kho liếc bảng số lượng hàng trên bảng kính:                              │
│ "Áo khoác size L còn 5 chiếc" ──► Tạm giữ 1 chiếc cho đơn hàng (khóa ngắn)!     │
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
- Trong Rust, chúng ta mô hình hóa các trạng thái này bằng `enum` có kiểu dữ liệu mạnh mẽ kết hợp mẫu so khớp `match`, biến mọi hành vi chuyển trạng thái bất hợp pháp thành lỗi được kiểm soát (`Err`) thay vì dữ liệu hỏng âm thầm. (Muốn biến chúng thành *lỗi biên dịch* thì dùng mẫu typestate — mỗi trạng thái một kiểu riêng.)

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
- Toàn bộ sự kiện được ghi nối đuôi (`append-only`) vào tệp nhật ký trên đĩa cứng mở bằng `std::fs::OpenOptions::append`. Nhờ tính chất tuần tự (Sequential I/O), thông lượng ghi rất cao.
- **Lưu ý sống còn về độ bền**: `write_all` + `flush()` chỉ đẩy dữ liệu vào bộ đệm trang (page cache) của hệ điều hành — mất điện lúc đó vẫn mất. Phải gọi `sync_data()`/`sync_all()` (fsync) và chờ nó xong rồi mới báo "thành công" cho khách. Mỗi lần fsync tốn từ vài chục micro-giây (SSD có tụ bảo vệ) tới vài mili-giây, nên hệ thống thật gộp nhiều sự kiện vào một lần fsync (*group commit*) để giữ thông lượng cao.
- Khi khởi động lại, ta phát lại WAL để dựng lại **mọi** trạng thái suy ra được: đơn hàng, khóa chống trùng lặp (nếu không, khách gửi lại yêu cầu cũ sau khi máy chủ khởi động lại sẽ bị trừ tiền lần hai), và tồn kho (kho ban đầu trừ đi các đơn đang giữ hàng).

---

## Mã nguồn minh họa thực chiến (Idiomatic Runnable Rust Blueprint)

Dưới đây là mã nguồn hoàn chỉnh của **Động cơ Xử lý Đơn hàng Phân tán (Distributed Order Processing Engine)** được lập trình bằng 100% Safe Rust chuẩn mực, tích hợp đầy đủ tính năng: Xác thực đầu vào (kể cả chặn ký tự phân cách trong khóa để không "tiêm" được dòng giả vào WAL), khóa chống trùng lặp Idempotency có trạng thái "đang xử lý" (chỉ đánh dấu "đã xong" khi đơn hàng thực sự được ghi bền — thất bại thì nhả khóa để khách gửi lại được), máy trạng thái đơn hàng, tạm giữ kho hàng đồng thời, và cơ chế ghi (`sync_data`)/phục hồi nhật ký sự kiện từ tệp đĩa:

```rust
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
        let mut reader = BufReader::new(File::open(&self.wal_path)?);

        let orders = self.orders.get_mut().unwrap();
        let keys = self.idempotency_keys.get_mut().unwrap();
        let mut skipped = 0;
        // Số byte của phần nhật ký còn nguyên vẹn (chỉ tính dòng có '\n' và đọc được)
        let mut valid_len: u64 = 0;
        let mut line = String::new();
        loop {
            line.clear();
            let n = reader.read_line(&mut line)?;
            if n == 0 {
                break;
            }
            let parsed = if line.ends_with('\n') {
                OrderEntity::from_wal_line(line.trim_end())
            } else {
                None
            };
            match parsed {
                Some(order) => {
                    keys.insert(
                        order.idempotency_key.clone(),
                        IdempotencyState::Completed(order.order_id),
                    );
                    orders.insert(order.order_id, order);
                    valid_len += n as u64;
                }
                // Dòng ghi dở khi mất điện: KHÔNG đoán bừa giá trị 0, và dừng ở đây —
                // mọi thứ phía sau một dòng hỏng đều không đáng tin.
                None => {
                    skipped += 1;
                    break;
                }
            }
        }

        // CẮT BỎ đuôi rách khỏi tệp. Nếu chỉ bỏ qua khi đọc mà để nguyên trên đĩa, bản ghi
        // kế tiếp (mở ở chế độ append) sẽ dính liền vào rác thành MỘT dòng hỏng — và một
        // đơn hàng đã fsync, đã trả Ok cho khách sẽ biến mất ở lần khôi phục sau.
        if skipped > 0 {
            let file = self.wal_file.get_mut().unwrap();
            file.set_len(valid_len)?;
            file.sync_all()?;
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
    fn order_written_after_torn_tail_survives_next_restart() {
        let path = temp_wal("torn_then_write");
        std::fs::write(&path, "1|1|7|10|PAID|A\n2|1|7|1").unwrap();
        {
            let inventory = Arc::new(InventoryManager::with_stock(&[(7, 5)]));
            let engine = DistributedOrderEngine::open(&path, inventory).unwrap();
            // Ghi một đơn mới SAU đuôi rách và nhận Ok (đã fsync)
            assert!(engine.submit_order("C", 3, 1, 7, 10).is_ok());
        }
        // Khởi động lại lần nữa: đơn #3 đã được xác nhận phải còn nguyên
        let inventory = Arc::new(InventoryManager::with_stock(&[(7, 5)]));
        let engine = DistributedOrderEngine::open(&path, Arc::clone(&inventory)).unwrap();
        assert_eq!(engine.total_orders(), 2);
        assert_eq!(inventory.get_available_stock(7), 3);
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
```

---

## Bảng tra cứu lỗi biên dịch & Cách khắc phục (Compiler Error Guide)

Dưới đây là các lỗi biên dịch thường gặp nhất khi hiện thực hóa động cơ xử lý đơn hàng phân tán trong Rust:

| Mã lỗi | Thông báo mẫu từ trình biên dịch | Nguyên nhân cốt lõi | Cách khắc phục nhanh |
|---|---|---|---|
| **E0382** | `` borrow of moved value: `order` `` | Bạn di chuyển quyền sở hữu (ownership) của `order` vào hàm ghi đĩa hoặc hàm xử lý khác, sau đó lại dùng tiếp nó. | Sử dụng phương thức `.clone()` để tạo bản sao sự kiện, hoặc chỉ truyền tham chiếu mượn (borrow). |
| **E0502** | `` cannot borrow `orders` as mutable because it is also borrowed as immutable `` | Bạn vừa tra cứu đơn hàng trong `HashMap` vừa cố gắng cập nhật trạng thái của nó trong cùng một khối lệnh. | Sử dụng phương thức `.get_mut(&order_id)` để mượn khả biến trực tiếp phần tử cần cập nhật. |
| **E0599** | `` no method named `lines` found for struct `File` in the current scope `` | Bạn sử dụng phương thức `.lines()` trực tiếp trên đối tượng `File` mà quên đưa qua bộ đệm. | Bọc tệp trong bộ nhớ đệm (buffer) đọc hiệu năng cao: `BufReader::new(file)`. |
| **E0599** | `no method named 'lock' found for struct 'InventoryManager'` | Gọi nhầm phương thức `.lock()` trên đối tượng ngoài thay vì trên trường khóa `Mutex` nội bộ. | Đảm bảo gọi đúng trường được bảo vệ: `self.stock.lock().unwrap()`. |

### Ví dụ phân tích lỗi `E0382` khi ghi nhật ký sự kiện:

```rust
#[derive(Debug, Clone)]
struct Order {
    id: u64,
}

fn log_it(order: Order) {
    println!("Ghi nhật ký: {:?}", order);
}

// Đoạn mã lỗi minh họa E0382:
fn handle_broken(order: Order) {
    // log_it(order); // Di chuyển quyền sở hữu order
    // println!("Đơn hàng đã xử lý: {:?}", order); // LỖI E0382: order đã bị di chuyển!
}

// Cách sửa chữa đúng chuẩn: clone (hoặc tốt hơn: cho log_it nhận &Order)
fn handle_fixed(order: Order) {
    log_it(order.clone()); // Tạo bản sao độc lập
    println!("Đơn hàng an toàn: {:?}", order); // order ban đầu vẫn còn nguyên vẹn!
}
```

---

## Tóm tắt chương & Bài tập rèn luyện (Summary & Exercises)

### 4 Điểm cốt lõi cần ghi nhớ:
1. **Khóa Định danh Bất biến (Idempotency Key)**: Tấm khiên thần kỳ bảo vệ các dịch vụ thanh toán và đơn hàng khỏi các yêu cầu gửi lặp lại do chập chờn mạng.
2. **Máy Trạng Thái Hữu Hạn (FSM)**: Kiểm soát chặt chẽ từng bước chuyển dịch vòng đời của đơn hàng, loại bỏ triệt để các trạng thái mâu thuẫn nghiệp vụ.
3. **Bảo toàn Dữ liệu bằng WAL**: Ghi nối đuôi sự kiện tuần tự xuống đĩa cứng và `fsync` trước khi xác nhận; khi khởi động lại, phát lại WAL để dựng lại đơn hàng, tồn kho và khóa chống trùng lặp (Crash Recovery).
4. **Đỉnh cao Kỹ nghệ Hệ thống Rust**: Sự phối hợp nhuần nhuyễn giữa quyền sở hữu (ownership), mượn (borrow), thời gian sống (lifetime), con trỏ thông minh (smart pointer) và bộ nhớ đệm (buffer) tạo nên một cỗ máy vừa an toàn đa luồng vừa bền vững trước sự cố.

### Bài tập rèn luyện tự giải:
1. **Bài tập 1 (Bổ sung Kênh Hủy đơn hàng và Hoàn trả Kho - Order Cancellation & Stock Rollback)**:  
   Mở rộng động cơ với phương thức `cancel_order(&self, order_id: u64)`. Khi một đơn hàng ở trạng thái `Paid` bị hủy, hệ thống sẽ tự động hoàn trả lại +1 đơn vị sản phẩm vào kho phân tán `InventoryManager` và ghi sự kiện `CANCELLED` xuống tệp WAL.
2. **Bài tập 2 (Xây dựng Tiến trình Dọn dẹp và Nén Nhật ký WAL - Log Compaction)**:  
   Sau khi hệ thống ghi nhận 10,000 sự kiện, tệp `orders.wal` sẽ phình to. Hãy viết hàm `compact_wal_log(&self)` chỉ giữ lại trạng thái cuối cùng mới nhất của từng đơn hàng và ghi sang tệp mới gọn gàng, giải phóng dung lượng đĩa tương tự như kiến trúc Bitcask đã học ở Chương 36.
3. **Bài tập 3 (Suy ngẫm đỉnh cao: Thiết kế Kiến trúc Thanh toán Saga Phân tán)**:  
   Khi Dịch vụ Đơn hàng (Order Service), Dịch vụ Kho (Inventory Service) và Dịch vụ Cổng Thanh toán (Payment Service) nằm trên 3 máy chủ phân tán khác nhau ở 3 quốc gia, việc sử dụng giao dịch phân tán 2PC (Two-Phase Commit) sẽ gây nghẽn mạng nghiêm trọng. Hãy trình bày cách áp dụng **Mô hình Saga điều phối qua Sự kiện (Event-Driven Choreography Saga)** để đảm bảo tính nhất quán cuối cùng (Eventual Consistency) nếu bước thanh toán bất ngờ bị ngân hàng từ chối.

---

### Gợi ý & Lời giải

<details>
<summary><b>Bài tập 1 — Gợi ý</b></summary>

Hủy đơn = giao dịch bù (compensating transaction): đảo ngược hiệu ứng của đơn đã trả tiền — ghi sự kiện CANCELLED xuống WAL rồi hoàn +1 vào kho (`InventoryManager::release_item`). Chỉ hủy được đơn ở đúng trạng thái Paid. Nhớ kiểm tra rằng việc phục hồi từ WAL cũng ra đúng tồn kho.
</details>

<details>
<summary><b>Bài tập 1 — Lời giải</b></summary>

```rust
// Thêm vào cùng mô-đun với `DistributedOrderEngine` của chương này.
impl DistributedOrderEngine {
    /// Hủy đơn: CHỈ khi đang ở trạng thái Paid. Ghi CANCELLED xuống WAL trước,
    /// thành công rồi mới đổi trạng thái trên RAM và hoàn +1 vào kho.
    pub fn cancel_order(&self, order_id: u64) -> Result<(), &'static str> {
        let mut orders = self.orders.lock().unwrap();
        let order = orders.get_mut(&order_id).ok_or("Không tìm thấy đơn hàng")?;
        match order.status {
            OrderStatus::Paid => {}
            OrderStatus::Fulfilled => return Err("Đơn đã xuất kho, không hủy được"),
            _ => return Err("Chỉ hủy được đơn ở trạng thái Paid"),
        }

        let mut cancelled = order.clone();
        cancelled.status = OrderStatus::Cancelled;
        self.log_event_to_disk(&cancelled)
            .map_err(|_| "Lỗi ghi đĩa nhật ký WAL")?; // 1. Ghi sự kiện bền vững
        *order = cancelled; // 2. Đổi trạng thái
        self.inventory.release_item(order.item_id); // 3. Giao dịch bù: hoàn kho
        Ok(())
    }
}

#[test]
fn cancel_paid_order_restores_stock_and_logs() {
    let path = std::env::temp_dir().join(format!("ch54_cancel_{}.wal", std::process::id()));
    let _ = std::fs::remove_file(&path);
    {
        let inventory = Arc::new(InventoryManager::with_stock(&[(7, 1)]));
        let engine = DistributedOrderEngine::open(&path, Arc::clone(&inventory)).unwrap();
        engine.submit_order("K", 100, 1, 7, 500).unwrap();
        assert_eq!(inventory.get_available_stock(7), 0); // đơn đang giữ chiếc duy nhất

        assert_eq!(engine.cancel_order(100), Ok(()));
        assert_eq!(inventory.get_available_stock(7), 1); // đã hoàn +1
        assert_eq!(engine.get_order(100).unwrap().status, OrderStatus::Cancelled);
        assert!(engine.cancel_order(100).is_err()); // không hủy hai lần
        assert!(engine.cancel_order(999).is_err()); // đơn không tồn tại
    }
    // Sau "sập máy", WAL cho biết đơn đã hủy -> phục hồi KHÔNG trừ kho cho nó
    let inventory = Arc::new(InventoryManager::with_stock(&[(7, 1)]));
    let engine = DistributedOrderEngine::open(&path, Arc::clone(&inventory)).unwrap();
    assert_eq!(engine.get_order(100).unwrap().status, OrderStatus::Cancelled);
    assert_eq!(inventory.get_available_stock(7), 1);
    let _ = std::fs::remove_file(&path);
}
```

Điểm cốt lõi: hủy một đơn *đã trả tiền* không phải là "xóa" nó — mà là chạy một **giao dịch bù (compensating transaction)** đảo ngược mọi hiệu ứng đã gây ra: đã trừ kho thì hoàn kho, và **ghi lại sự kiện `CANCELLED` xuống WAL** (trước khi đổi trạng thái trên RAM) thay vì sửa bản ghi cũ. Vì `OrderStatus::holds_stock()` trả `false` cho `Cancelled`, việc phát lại WAL sau sự cố tự động không trừ kho cho đơn đã hủy. Đây là tư duy **nguồn sự kiện (event sourcing)**: nhật ký chỉ-nối-thêm giữ *toàn bộ lịch sử* — đơn được tạo, trả tiền, rồi hủy — chứ không ghi đè trạng thái. Nhờ vậy bạn luôn dựng lại được trạng thái tại bất kỳ thời điểm nào, và có dấu vết kiểm toán đầy đủ. Ràng buộc "chỉ hủy được đơn ở trạng thái `Paid`" là **bảo vệ bất biến trạng thái**: không cho hủy đơn chưa trả (chẳng có gì để hoàn) hay đơn đã giao (hàng đã ra khỏi kho, hoàn kho là sai).
</details>

<details>
<summary><b>Bài tập 2 — Gợi ý</b></summary>

Nén nhật ký (giống Bitcask ch36): WAL ghi nối-thêm nên phình to với nhiều sự kiện cho cùng một đơn. Nén = chỉ giữ trạng thái CUỐI CÙNG của mỗi đơn (bảng `orders` trên RAM chính là kết quả đó), ghi sang tệp mới, `sync_all`, `rename` đè tệp cũ, rồi `fsync` thư mục.
</details>

<details>
<summary><b>Bài tập 2 — Lời giải</b></summary>

```rust
// Thêm vào cùng mô-đun với `DistributedOrderEngine` của chương này.
impl DistributedOrderEngine {
    /// Nén WAL: ghi trạng thái MỚI NHẤT của mỗi đơn (chính là bảng `orders` trên RAM,
    /// vốn đã là kết quả phát lại WAL) sang tệp mới, rồi thay thế tệp cũ an toàn.
    pub fn compact_wal_log(&self) -> io::Result<()> {
        // Khóa theo ĐÚNG thứ tự như `fulfill_order` (orders rồi wal_file) để tránh
        // deadlock; giữ khóa WAL suốt quá trình để không ai ghi chen vào tệp cũ.
        let orders = self.orders.lock().unwrap();
        let mut wal = self.wal_file.lock().unwrap();

        let tmp_path = self.wal_path.with_extension("wal.compact");
        {
            let mut tmp = File::create(&tmp_path)?;
            let mut ids: Vec<u64> = orders.keys().copied().collect();
            ids.sort_unstable();
            for id in ids {
                tmp.write_all(orders[&id].to_wal_line().as_bytes())?;
            }
            tmp.sync_all()?; // 1. Nội dung tệp mới chắc chắn nằm trên đĩa
        }
        std::fs::rename(&tmp_path, &self.wal_path)?; // 2. Thay thế nguyên tử
        // 3. fsync THƯ MỤC cha để chính việc đổi tên cũng bền vững (Linux/Unix)
        let dir = self
            .wal_path
            .parent()
            .filter(|p| !p.as_os_str().is_empty())
            .unwrap_or(Path::new("."));
        File::open(dir)?.sync_all()?;
        // 4. Mọi sự kiện sau đó ghi nối vào tệp MỚI
        *wal = OpenOptions::new().append(true).open(&self.wal_path)?;
        Ok(())
    }
}

#[test]
fn compaction_keeps_latest_state_only() {
    let path = std::env::temp_dir().join(format!("ch54_compact_{}.wal", std::process::id()));
    let _ = std::fs::remove_file(&path);
    let line_count = |p: &Path| std::fs::read_to_string(p).unwrap().lines().count();
    {
        let engine = DistributedOrderEngine::open(&path, Arc::new(InventoryManager::new())).unwrap();
        engine.submit_order("A", 1, 1, 101, 10).unwrap();
        engine.submit_order("B", 2, 1, 101, 10).unwrap();
        engine.fulfill_order(1).unwrap();
        assert_eq!(line_count(&path), 3); // đơn 1 có 2 dòng: PAID rồi FULFILLED

        engine.compact_wal_log().unwrap();
        assert_eq!(line_count(&path), 2); // mỗi đơn đúng 1 dòng, trạng thái cuối

        engine.submit_order("C", 3, 1, 101, 10).unwrap(); // vẫn ghi tiếp được
        assert_eq!(line_count(&path), 3);
    }
    // Phục hồi từ tệp đã nén cho ra đúng trạng thái
    let inventory = Arc::new(InventoryManager::new());
    let engine = DistributedOrderEngine::open(&path, Arc::clone(&inventory)).unwrap();
    assert_eq!(engine.get_order(1).unwrap().status, OrderStatus::Fulfilled);
    assert_eq!(engine.total_orders(), 3);
    assert_eq!(inventory.get_available_stock(101), 7);
    let _ = std::fs::remove_file(&path);
}
```

Nén nhật ký giải quyết điểm yếu cố hữu của mọi kiến trúc **ghi nối-thêm** (append-only): tệp *chỉ lớn lên*, không bao giờ nhỏ đi, vì mỗi thay đổi trạng thái là một dòng mới chứ không sửa dòng cũ. Sau 10.000 sự kiện, một đơn hàng đi qua 5 trạng thái để lại 5 dòng — nhưng để *phục hồi*, ta chỉ cần trạng thái **cuối cùng**. Nén làm đúng việc đó: **phát lại toàn bộ nhật ký, giữ lại trạng thái mới nhất của mỗi đơn, ghi sang tệp mới gọn gàng** rồi thay thế tệp cũ. Đây chính là cơ chế `compact()` của Bitcask ở Chương 36 (bỏ các bản ghi bị bia mộ che khuất) và của LSM-Tree ở Chương 34 (gộp SSTable). Điểm an toàn phải nhớ, theo đúng thứ tự trong mã: **(1) ghi tệp mới và `sync_all` nó, (2) `rename` đè tệp cũ (nguyên tử), (3) `fsync` thư mục cha** để chính việc đổi tên được ghi bền — bỏ bước 1 thì sau mất điện có thể thấy tệp WAL mới *rỗng hoặc dở dang* trong khi tệp cũ đã bị thay; bỏ bước 3 thì việc đổi tên có thể "quay ngược". Ngoài ra, nén phải giữ khóa theo đúng thứ tự `orders` → `wal_file` như các phương thức khác, nếu không sẽ có deadlock hoặc một sự kiện ghi chen vào tệp cũ rồi bị mất.
</details>

<details>
<summary><b>Bài tập 3 — Gợi ý</b></summary>

Saga thay 2PC bằng một chuỗi giao dịch cục bộ + giao dịch bù khi hỏng. Mỗi dịch vụ tự commit phần của mình; nếu một bước sau thất bại, chạy ngược các bước bù để hoàn tác.
</details>

<details>
<summary><b>Bài tập 3 — Lời giải</b></summary>

**Vì sao 2PC gây nghẽn, và Saga giải quyết thế nào — thiết kế thanh toán phân tán qua 3 dịch vụ ở 3 quốc gia:**

**Vấn đề của 2PC (Two-Phase Commit):** 2PC đạt tính nguyên tử bằng cách **khóa tài nguyên ở cả 3 dịch vụ suốt toàn bộ giao dịch**, chờ một điều phối viên ra lệnh commit đồng loạt. Với 3 máy chủ ở 3 quốc gia, độ trễ mạng giữa các châu lục là hàng trăm mili-giây — nghĩa là khóa bị **giữ rất lâu** qua đường truyền chậm. Tệ hơn, nếu điều phối viên hoặc một dịch vụ chết giữa chừng, các dịch vụ kia **kẹt khóa chờ vô định** (blocking). Ở quy mô địa lý, 2PC bóp nghẹt thông lượng và tạo điểm chết đơn lẻ.

**Saga — chuỗi giao dịch cục bộ + bù trừ:** thay vì một giao dịch phân tán khổng lồ có khóa, Saga chia thành **một chuỗi giao dịch cục bộ độc lập**, mỗi dịch vụ **tự commit ngay phần của mình** (không giữ khóa qua mạng). Nếu một bước sau thất bại, Saga chạy các **giao dịch bù (compensating transactions)** để *hoàn tác* các bước đã commit trước đó.

```text
Luồng thành công (mỗi bước commit cục bộ NGAY, không giữ khóa xuyên quốc gia):
  [Order Service]     tạo đơn (Pending)        -> commit
  [Inventory Service] giữ 1 sản phẩm            -> commit
  [Payment Service]   trừ tiền                  -> commit
  [Order Service]     đánh dấu đơn Paid         -> commit   -> hoàn tất

Nếu Payment THẤT BẠI ở bước 3 -> chạy NGƯỢC các giao dịch bù:
  [Inventory Service] HOÀN 1 sản phẩm vào kho   (bù cho bước 2)
  [Order Service]     đánh dấu đơn Cancelled    (bù cho bước 1)
  -> hệ thống trở về trạng thái nhất quán, KHÔNG cần khóa toàn cục
```

**Đánh đổi phải nói thẳng — Saga hy sinh tính cô lập:**

| | 2PC | Saga |
|---|---|---|
| Khóa qua mạng | có, giữ suốt giao dịch | không — mỗi bước commit cục bộ ngay |
| Nguyên tử | thật (all-or-nothing tức thời) | *cuối cùng* (qua bù trừ) |
| **Tính cô lập** | có | **KHÔNG — có trạng thái trung gian lộ ra** |
| Chịu lỗi địa lý | kém (blocking) | tốt (không khóa chờ) |

Cái giá của Saga là **mất tính cô lập**: giữa lúc kho đã giữ hàng mà thanh toán chưa xong, tồn tại một *trạng thái trung gian nhìn thấy được* — một truy vấn khác có thể thấy "1 sản phẩm đang bị giữ" cho một đơn rồi sẽ bị hủy. Ứng dụng phải **tự thiết kế để chịu được trạng thái trung gian này** (ví dụ đánh dấu "đang xử lý", không cho thao tác khác đè lên). Và **giao dịch bù phải thật sự đảo ngược được** — hoàn tiền, hoàn kho — điều không phải lúc nào cũng làm được (đã gửi email cho khách thì không "thu hồi email" được).

Nguyên tắc chọn: **2PC** khi các bên ở gần (cùng trung tâm dữ liệu, độ trễ thấp) và cần cô lập chặt; **Saga** khi phân tán về địa lý, ưu tiên khả dụng và thông lượng, và chấp nhận nhất quán *cuối cùng* cùng với việc tự quản lý trạng thái trung gian. Hầu hết hệ thống thương mại điện tử quy mô lớn chọn Saga — vì một đơn hàng "đang xử lý" trong vài giây là chấp nhận được, còn khóa kho toàn cầu thì không.
</details>
