# Chương 32: Kiến trúc trang Slotted-Page & Quản lý bộ nhớ đệm Buffer Pool (Slotted-Page Architecture & Buffer Pool Management)

## Giới thiệu & Mục tiêu học tập

Trong chương trước, chúng ta đã hiểu nguyên lý lưu trữ đĩa cứng và kỹ thuật đóng gói nhị phân. Tuy nhiên, nếu mỗi lần thêm một bản ghi mới cơ sở dữ liệu lại thực hiện một thao tác ghi đĩa riêng lẻ, hệ thống sẽ sụp đổ hiệu năng vì nghẽn cổ chai I/O. Hơn nữa, trong thực tế, các bản ghi luôn có kích thước thay đổi (biến thiên): Người có tên 5 ký tự, người có tên 50 ký tự; khi một người dùng xóa tài khoản, khoảng trống ô nhớ bỏ lại sẽ bị phân mảnh nếu không có cách tổ chức khoa học.

Mọi hệ quản trị cơ sở dữ liệu hàng đầu thế giới (như PostgreSQL, MySQL InnoDB, SQLite) giải quyết bài toán này thông qua hai trụ cột kiến trúc cốt lõi:
1. **Kiến trúc trang phân khe (Slotted-Page Architecture)**: Phân chia tệp dữ liệu trên đĩa thành các khối có kích thước cố định chuẩn mực là **4KB (4096 bytes)**, cho phép chứa các bản ghi có độ dài co giãn linh hoạt mà không sợ phân mảnh ô nhớ.
2. **Bộ quản lý bộ nhớ đệm (Buffer Pool Manager)**: Một vùng đệm RAM trung gian giữ các trang dữ liệu nóng (hot pages) để người dùng đọc/ghi tức thì, kết hợp thuật toán **LRU (Least Recently Used)** để tự động trục xuất (evict) các trang cũ về đĩa cứng khi bộ nhớ đệm (buffer) bị đầy.

Mục tiêu học tập của chương này:
- Nắm vững cấu tạo vật lý của một **Trang dữ liệu cố định 4KB (Fixed 4KB Page)** và lý do kích thước 4KB tương thích hoàn hảo với phần cứng SSD và hệ điều hành.
- Thấu hiểu cơ chế "hai đầu tiến vào giữa" của kiến trúc **Slotted-Page**: Thư mục khe (Slot Directory) tiến từ trên xuống, dữ liệu bản ghi ghi từ dưới đáy lên.
- Định vị bản ghi toàn cục trong cơ sở dữ liệu thông qua bộ đôi định danh **Tuple ID / RID `(page_id, slot_id)`**.
- Xây dựng mô hình **Buffer Pool** với cơ chế quản lý khung trang (Frames), cờ bẩn (Dirty Flag), và số đếm giữ trang (Pin Count).
- Cài đặt thuật toán loại trừ trang **LRU Page Eviction Policy** an toàn 100% bằng Rust.

---

## Hình tượng hóa đời sống (Intuitive Everyday Analogy)

Hãy quan sát hai hình ảnh đời sống trực quan dưới đây để hiểu thấu kiến trúc Slotted-Page và Buffer Pool:

```
┌──────────────────────────────────────────────────────────────────────────────────┐
│                   HÌNH TƯỢNG HÓA SLOTTED-PAGE VÀ BUFFER POOL                     │
├──────────────────────────────────────────────────────────────────────────────────┤
│                                                                                  │
│ [1. KIẾN TRÚC SLOTTED-PAGE: TRANG SỔ TAY 4KB GHI TỪ HAI ĐẦU]                     │
│ ┌──────────────────────────────────────────────────────────────────────┐         │
│ │ Header: [Mã trang: #1] [Số khe: 3] [Con trỏ đáy tự do: 3950]         │ 0 bytes │
│ ├──────────────────────────────────────────────────────────────────────┤         │
│ │ Khe 0 (Slot 0): Tọa độ = 4050, Dài = 46 bytes                        │         │
│ │ Khe 1 (Slot 1): Tọa độ = 4000, Dài = 50 bytes    ▼ Tiến dần xuống    │         │
│ │ Khe 2 (Slot 2): Tọa độ = 3950, Dài = 50 bytes                        │         │
│ ├ - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - -┤         │
│ │                                                                      │         │
│ │                 VÙNG NHỚ TRỐNG TỰ DO Ở GIỮA (FREE SPACE)             │         │
│ │                                                                      │         │
│ ├ - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - -┤         │
│ │ Bản ghi 2: [ID: 103, Tên: "Lê Văn C"] (Dài 50B)  ▲ Tiến ngược lên   │ 3950 B  │
│ │ Bản ghi 1: [ID: 102, Tên: "Trần Thị B"] (Dài 50B)                    │ 4000 B  │
│ │ Bản ghi 0: [ID: 101, Tên: "Nguyễn Văn A"] (Dài 46B)                  │ 4050 B  │
│ └──────────────────────────────────────────────────────────────────────┘ 4096 B  │
│                                                                                  │
│ [2. BUFFER POOL: BÀN HỌC THƯ VIỆN CÓ ĐÚNG 3 CHỖ ĐỂ SÁCH]                         │
│                                                                                  │
│ Thư viện có 10.000 cuốn sách (Ổ đĩa đĩa cứng SSD)                                 │
│ Mặt bàn bạn chỉ để được tối đa 3 cuốn sách (Buffer Pool RAM)                     │
│                                                                                  │
│ Muốn đọc cuốn thứ 4?                                                             │
│ -> Tìm cuốn sách mà bạn LÂU NHẤT KHÔNG ĐỌC (LRU)                                 │
│ -> Cất cuốn sách đó về giá sách (Evict) để nhường chỗ trống trên bàn!           │
└──────────────────────────────────────────────────────────────────────────────────┘
```

### 1. Trang sổ tay Slotted-Page ghi từ hai đầu
- Hãy tưởng tượng một trang giấy học sinh:
  - Ở **dòng trên cùng**, bạn ghi "Bảng mục lục": Dòng 1 nằm ở đâu, dài bao nhiêu chữ; Dòng 2 nằm ở đâu, dài bao nhiêu chữ. Mỗi khi có bài viết mới, bạn ghi thêm một dòng vào mục lục này, tiến dần từ trên xuống.
  - Nhưng nội dung các bài viết (ngắn dài tùy ý) thì bạn lại bắt đầu chép từ **dòng cuối cùng của trang giấy chép ngược dần lên trên**.
  - Khi mục lục ở trên đầu và bài viết ở dưới đáy chạm nhau, trang giấy đó chính thức hết chỗ (Full Page).
- **Lợi ích vĩ đại**: Nếu bài viết ở Khe 1 bị xóa, ta chỉ cần đánh dấu khe đó là rỗng trong bảng mục lục ở trên đầu. Tọa độ của các khe khác không bị ảnh hưởng, và người ngoài muốn tìm bài viết chỉ cần nhìn vào số khe mà không cần biết bài viết nằm ở tọa độ byte cụ thể nào!

### 2. Buffer Pool và Bàn học thư viện (LRU Eviction)
- Thư viện có hàng vạn cuốn sách nằm trên các giá sách khổng lồ dưới tầng hầm (Ổ đĩa).
- Bàn học của bạn chỉ có diện tích đủ để mở **3 cuốn sách** cùng lúc (Buffer Pool trên RAM).
- Khi bạn cần nghiên cứu cuốn sách thứ 4:
  - Bạn không thể nhét thêm vào bàn vì sẽ làm đổ đồ.
  - Bạn quan sát xem trong 3 cuốn đang để trên bàn, cuốn nào đã để yên lâu nhất mà bạn không lật trang (Thuật toán **LRU - Least Recently Used**).
  - Bạn đem cuốn sách đó cất trở lại vào giá sách thư viện (trục xuất - **evict**). Nếu cuốn sách đó bạn có ghi chú thêm vào các trang sách (trang bẩn - **dirty page**), bạn phải chép lại cẩn thận xuống đĩa rồi mới cất đi!

---

## Khái niệm & Cơ chế kỹ thuật chuyên sâu (In-Depth Technical Mechanics)

### 1. Tại sao kích thước trang cố định luôn là 4KB?

Hầu hết các hệ quản trị cơ sở dữ liệu và hệ điều hành đều chuẩn hóa kích thước trang là **4096 bytes (4KB)**:
1. **Kiến trúc phần cứng SSD/HDD**: Các khối khu vực (sectors) vật lý của đĩa hiện đại (Advanced Format) có kích thước 4096 bytes.
2. **Bộ nhớ ảo (Virtual Memory)**: Đơn vị quản lý bộ nhớ của nhân hệ điều hành Linux/macOS/Windows cũng là các trang 4KB.
3. Khi kích thước trang của cơ sở dữ liệu khớp hoàn hảo với 4KB của hệ điều hành và phần cứng, một thao tác đọc/ghi trang sẽ diễn ra trong **1 lệnh I/O duy nhất**, triệt tiêu hiện tượng đọc ghi phân mảnh lãng phí.

### 2. Bóc tách giải phẫu một trang Slotted-Page

Trong một mảng byte `[u8; 4096]`:
- **Phần đầu trang (Page Header - 8 bytes)**:
  - `page_id (u32)`: Số thứ tự trang trong tệp cơ sở dữ liệu.
  - `slot_count (u16)`: Số lượng khe bản ghi hiện đang có.
  - `free_space_pointer (u16)`: Tọa độ byte đáy trống kế tiếp (ban đầu là 4096).
- **Thư mục khe (Slot Directory - Mỗi khe chiếm 4 bytes)**:
  - `offset (u16)`: Tọa độ byte bắt đầu của bản ghi.
  - `length (u16)`: Độ dài của bản ghi.
- **Định danh bản ghi toàn cục (Tuple ID / RID)**:
  - Một bản ghi trong cơ sở dữ liệu được định danh duy nhất bởi cặp số: `RID = (page_id, slot_id)`.
  - Dù bản ghi có bị dịch chuyển vị trí bên trong trang (chẳng hạn khi dọn rác dồn ô nhớ), giá trị `RID` của nó đối với các bảng chỉ mục bên ngoài vẫn hoàn toàn không thay đổi!

### 3. Cấu trúc và Vòng đời của Buffer Pool

Buffer Pool là một hệ thống đệm compute vi gồm:
1. **Bảng khung trang (Frame Table)**: Mảng các ô nhớ kích thước 4KB trên RAM (`Page Frames`).
2. **Bảng tra cứu trang (Page Table)**: Bảng băm ánh xạ từ `page_id` trên đĩa sang số thứ tự khung trang `frame_id` trên RAM.
3. **Cờ bẩn (Dirty Flag)**: Một bit đánh dấu trang đã bị sửa đổi dữ liệu hay chưa. Nếu cờ bẩn bật (`is_dirty == true`), khi trục xuất trang về đĩa cứng, hệ thống bắt buộc phải ghi nội dung đệm xuống đĩa. Nếu chưa bị sửa (`is_dirty == false`), chỉ cần hủy bỏ khỏi RAM mà không tốn lệnh ghi đĩa nào.
4. **Hàng đợi LRU (LRU List)**: Quản lý thứ tự thời gian sử dụng của các trang. Mỗi lần một trang được đọc hoặc ghi, nó được chuyển xuống cuối danh sách (vừa sử dụng gần nhất). Trang ở đầu danh sách là trang "nguội" nhất, sẽ là nạn nhân đầu tiên bị trục xuất khi bộ nhớ đầy.

---

## Mã nguồn minh họa thực chiến (Idiomatic Runnable Rust Blueprint)

Dưới đây là một chương trình Rust hoàn chỉnh và độc lập, cài đặt cả hai cấu trúc:
1. Cấu trúc `SlottedPage` chuẩn 4096 bytes với khả năng thêm bản ghi, đọc bản ghi qua `slot_id`.
2. Hệ thống `BufferPool` hoàn chỉnh với dung lượng giới hạn, bảng băm tra cứu, cờ bẩn `is_dirty`, và cơ chế trục xuất trang theo thuật toán `LRU`:

```rust
use std::convert::TryInto;
use std::collections::HashMap;

/// Kích thước trang chuẩn của cơ sở dữ liệu (4KB)
pub const PAGE_SIZE: usize = 4096;

/// Cấu trúc khe lưu trữ trong thư mục Slotted-Page (4 bytes)
#[derive(Debug, Clone, Copy)]
pub struct RecordSlot {
    pub offset: u16,
    pub length: u16,
}

/// Cấu trúc Trang dữ liệu phân khe chuẩn 4KB (Slotted-Page)
pub struct SlottedPage {
    pub page_id: u32,
    pub data: [u8; PAGE_SIZE],
}

impl SlottedPage {
    /// Khởi tạo một trang mới compute kích thước 4096 bytes
    pub fn new(page_id: u32) -> Self {
        let mut state = Self {
            page_id,
            data: [0u8; PAGE_SIZE],
        };
        // Ghi Header ban đầu:
        // Byte 0..4: page_id
        state.data[0..4].copy_from_slice(&page_id.to_le_bytes());
        // Byte 4..6: slot_count = 0
        state.data[4..6].copy_from_slice(&0u16.to_le_bytes());
        // Byte 6..8: free_space_pointer = 4096 (đáy trang)
        state.data[6..8].copy_from_slice(&(PAGE_SIZE as u16).to_le_bytes());
        state
    }

    pub fn slot_count(&self) -> u16 {
        u16::from_le_bytes(self.data[4..6].try_into().unwrap())
    }

    fn nearest_slot(&mut self, count: u16) {
        self.data[4..6].copy_from_slice(&count.to_le_bytes());
    }

    pub fn tail_pointer(&self) -> u16 {
        u16::from_le_bytes(self.data[6..8].try_into().unwrap())
    }

    fn set_tail_pointer(&mut self, ptr: u16) {
        self.data[6..8].copy_from_slice(&ptr.to_le_bytes());
    }

    /// Thêm một bản ghi nhị phân vào trang - Trả về slot_id (chỉ số khe)
    pub fn add_sell_record(&mut self, bytes_ban_ghi: &[u8]) -> Option<u16> {
        let current_num_khe = self.slot_count();
        let con_tro_day = self.tail_pointer();
        let do_long_record = bytes_ban_ghi.len() as u16;

        // Tính toán vị trí tiêu tốn của Slot Directory ở trên đầu trang:
        // Header: 8 bytes. Mỗi khe: 4 bytes.
        let new_pos_value_khe = 8 + (current_num_khe as usize * 4);
        let capacity_remaining = con_tro_day as usize - (new_pos_value_khe + 4);

        // Kiểm tra xem trang còn đủ chỗ cho cả Slot mới lẫn thân dữ liệu không
        if do_long_record as usize > capacity_remaining {
            return None; // Trang đã đầy (Page Full)!
        }

        // 1. Tính tọa độ đáy mới và ghi dữ liệu từ đáy trang ngược lên
        let offset_day_moi = con_tro_day - do_long_record;
        let start = offset_day_moi as usize;
        let end = con_tro_day as usize;
        self.data[start..end].copy_from_slice(bytes_ban_ghi);

        // 2. Ghi thông tin Khe vào Slot Directory ở đầu trang
        self.data[new_pos_value_khe..new_pos_value_khe + 2].copy_from_slice(&offset_day_moi.to_le_bytes());
        self.data[new_pos_value_khe + 2..new_pos_value_khe + 4].copy_from_slice(&do_long_record.to_le_bytes());

        // 3. Cập nhật Header
        self.nearest_slot(current_num_khe + 1);
        self.set_tail_pointer(offset_day_moi);

        Some(current_num_khe)
    }

    /// Đọc bản ghi qua slot_id - O(1)
    pub fn read_sell_record(&self, slot_id: u16) -> Option<&[u8]> {
        let slot_count = self.slot_count();
        if slot_id >= slot_count {
            return None;
        }

        let pos_value_khe = 8 + (slot_id as usize * 4);
        let offset = u16::from_le_bytes(self.data[pos_value_khe..pos_value_khe + 2].try_into().unwrap()) as usize;
        let length = u16::from_le_bytes(self.data[pos_value_khe + 2..pos_value_khe + 4].try_into().unwrap()) as usize;

        Some(&self.data[offset..offset + length])
    }
}

/// Khung trang quản lý bên trong Buffer Pool
pub struct Frame {
    pub state: SlottedPage,
    pub is_dirty: bool,
}

/// Hệ thống quản lý bộ nhớ đệm Buffer Pool với thuật toán LRU Eviction
pub struct BufferPool {
    capacity: usize,
    frames: HashMap<u32, Frame>,
    lru_list: Vec<u32>, // Quản lý thứ tự: Đầu danh sách là nguội nhất (LRU)
}

impl BufferPool {
    pub fn new(capacity: usize) -> Self {
        Self {
            capacity,
            frames: HashMap::new(),
            lru_list: Vec::new(),
        }
    }

    /// Cập nhật trang vừa được truy cập xuống cuối danh sách LRU
    fn touch_lru(&mut self, page_id: u32) {
        self.lru_list.retain(|&id| id != page_id);
        self.lru_list.push(page_id);
    }

    /// Lấy trang từ bộ nhớ đệm (nếu có)
    pub fn get_page(&mut self, page_id: u32) -> Option<&SlottedPage> {
        if self.frames.contains_key(&page_id) {
            self.touch_lru(page_id);
            return self.frames.get(&page_id).map(|f| &f.state);
        }
        None
    }

    /// Đưa trang vào Buffer Pool - Nếu đầy, tự động trục xuất (evict) trang cũ nhất
    pub fn put_page(&mut self, state: SlottedPage, is_dirty: bool) {
        let id = state.page_id;

        // Nếu trang chưa có trong buffer và buffer đã đầy sức chứa
        if !self.frames.contains_key(&id) && self.frames.len() >= self.capacity {
            // Trục xuất trang ở đầu danh sách LRU (nguội nhất)
            let evict_id = self.lru_list.remove(0);
            if let Some(khung_cu) = self.frames.remove(&evict_id) {
                if khung_cu.is_dirty {
                    println!("    [EVICT]: Trang #{} có cờ bẩn (is_dirty=true) -> Đang ghi đè xuống đĩa SSD...", evict_id);
                } else {
                    println!("    [EVICT]: Trang #{} sạch (chưa sửa) -> Hủy khỏi RAM tức thì mà không cần ghi đĩa.", evict_id);
                }
            }
        }

        self.frames.insert(id, Frame { state, is_dirty });
        self.touch_lru(id);
    }

    pub fn num_state_show_has(&self) -> usize {
        self.frames.len()
    }
}

fn main() {
    println!("============================================================");
    println!("  KIẾN TRÚC SLOTTED-PAGE 4KB & QUẢN LÝ BỘ NHỚ ĐỆM BUFFER POOL");
    println!("============================================================");

    // 1. Khảo sát cấu trúc trang SlottedPage kích thước 4KB
    println!("[1] Thao tác trên Trang phân khe Slotted-Page (4096 bytes):");
    let mut state_1 = SlottedPage::new(1);
    println!("    - Khởi tạo Trang #1. Kích thước bộ đệm vật lý: {} bytes", state_1.data.len());
    println!("    - Con trỏ đáy tự do ban đầu: {} (Đáy trang)", state_1.tail_pointer());

    // Nạp các bản ghi có kích thước chuỗi thay đổi
    let ban_ghi_a = b"NguoiDung: Nguyen Van An - Ha Chain";
    let ban_ghi_b = b"NguoiDung: Tran Thi Binh - TP Ho Chi Minh (VIP Member)";
    let ban_ghi_c = b"NguoiDung: Le Hoang Cuong - Da Nang";

    let slot_a = state_1.add_sell_record(ban_ghi_a).expect("Lỗi chèn khe A");
    let slot_b = state_1.add_sell_record(ban_ghi_b).expect("Lỗi chèn khe B");
    let slot_c = state_1.add_sell_record(ban_ghi_c).expect("Lỗi chèn khe C");

    println!("    - Đã chèn Bản ghi A -> Được cấp Tuple ID: (Page: 1, Slot: {})", slot_a);
    println!("    - Đã chèn Bản ghi B -> Được cấp Tuple ID: (Page: 1, Slot: {})", slot_b);
    println!("    - Đã chèn Bản ghi C -> Được cấp Tuple ID: (Page: 1, Slot: {})", slot_c);
    println!("    - Tổng số khe: {}, Con trỏ đáy hiện tại: {}", state_1.slot_count(), state_1.tail_pointer());

    // Đọc lại nội dung qua Slot ID
    let doc_b = state_1.read_sell_record(slot_b).unwrap();
    println!("    - Đọc nội dung qua Slot ID {}: '{}'", slot_b, String::from_utf8_lossy(doc_b));
    assert_eq!(doc_b, ban_ghi_b);

    // 2. Khảo sát hệ thống Buffer Pool và thuật toán trục xuất LRU Eviction
    println!("\n[2] Vận hành Buffer Pool với sức chứa tối đa 2 trang:");
    let mut buffer_pool = BufferPool::new(2);

    // Đưa Trang 1 và Trang 2 vào Buffer Pool
    println!("    - Nạp Trang #1 (đã sửa đổi -> dirty=true) vào Buffer Pool");
    buffer_pool.put_page(state_1, true);

    let state_2 = SlottedPage::new(2);
    println!("    - Nạp Trang #2 (chỉ đọc -> dirty=false) vào Buffer Pool");
    buffer_pool.put_page(state_2, false);

    println!("    - Số trang hiện có trong Buffer: {}", buffer_pool.num_state_show_has());
    assert_eq!(buffer_pool.num_state_show_has(), 2);

    // Người dùng truy cập lại Trang 1 -> Trang 1 trở thành trang dùng gần nhất
    println!("\n    - Người dùng đọc Trang #1 -> Cập nhật thứ tự ưu tiên LRU cho Trang #1!");
    assert!(buffer_pool.get_page(1).is_some());

    // Giờ đây, Trang #2 là trang "nguội nhất" (lâu nhất không dùng).
    // Khi nạp thêm Trang #3 vào, Buffer Pool sẽ kích hoạt trục xuất (evict) Trang #2!
    println!("\n    - Nạp Trang #3 mới compute vào (Vượt quá sức chứa 2 trang):");
    let state_3 = SlottedPage::new(3);
    buffer_pool.put_page(state_3, false);

    // Kiểm tra: Trang 2 đã bị loại bỏ, Trang 1 và Trang 3 vẫn nằm trong Buffer Pool
    assert!(buffer_pool.get_page(2).is_none());
    assert!(buffer_pool.get_page(1).is_some());
    assert!(buffer_pool.get_page(3).is_some());
    println!("    => Thuật toán LRU Eviction vận hành chuẩn xác 100%!");

    println!("============================================================");
    println!("               HOÀN TẤT THỰC NGHIỆM CHƯƠNG 28               ");
    println!("============================================================");
}
```

---

## Bảng tra cứu lỗi biên dịch & Cách khắc phục (Compiler Error Guide)

Dưới đây là các lỗi biên dịch điển hình khi thiết kế Slotted-Page và hệ thống Buffer Pool trong Rust:

| Mã lỗi | Thông báo mẫu từ trình biên dịch | Nguyên nhân cốt lõi | Cách khắc phục nhanh |
|---|---|---|---|
| **E0382** | `use of moved value: 'trang'` | Bạn truyền `trang` vào hàm `put_page()` khiến quyền sở hữu (ownership) bị chuyển giao, sau đó lại dùng lại biến `trang` ở dòng dưới. | Gọi hàm đọc thông qua Buffer Pool `buffer_pool.get_page(id)` thay vì sử dụng trực tiếp biến cũ đã bị di chuyển. |
| **E0502** | `cannot borrow 'buffer_pool' as mutable because it is also borrowed as immutable` | Bạn vừa mượn bất biến một trang `let p = pool.get_page(1);`, vừa gọi hàm làm thay đổi bộ nhớ đệm `pool.put_page(...)` trong cùng phạm vi. | Giới hạn phạm vi mượn đọc hoặc sao chép dữ liệu cần thiết ra trước khi thực hiện thêm trang mới. |
| **E0277** | `the trait bound '[u8; 4096]: Default' is not satisfied` | Trong các phiên bản Rust rất cũ, mảng kích thước lớn hơn 32 không tự động derive một số trait. Trong Rust hiện đại (const generics), `[0u8; PAGE_SIZE]` hoàn toàn hợp lệ. | Khởi tạo mảng tường minh: `[0u8; PAGE_SIZE]`. |
| **E0308** | `mismatched types: expected 'u16', found 'usize'` | Các chỉ số trong Header của Slotted-Page dùng `u16` để tiết kiệm byte đĩa, trong khi độ dài của mảng trên RAM là `usize`. | Thực hiện ép kiểu tường minh an toàn: `len as u16` sau khi kiểm tra không vượt quá 4096 bytes. |

### Ví dụ phân tích lỗi `E0382` khi quản lý quyền sở hữu trang:

```rust
// Đoạn mã lỗi minh họa E0382: Di chuyển quyền sở hữu trang vào Buffer Pool
fn thu_nghiem_loi_trang(mut pool: BufferPool, state: SlottedPage) {
    // pool.put_page(trang, false); // Quyền sở hữu trang bị chuyển vào HashMap!
    // println!("Mã trang: {}", trang.page_id); // LỖI E0382: trang đã bị moved!
}

// Cách sửa chữa đúng chuẩn: Lấy mã ID ra trước hoặc truy cập qua Pool
fn thu_nghiem_dung_trang(mut pool: BufferPool, state: SlottedPage) {
    let id = state.page_id;
    pool.put_page(state, false);
    println!("Mã trang vừa nạp: {}", id);
    if let Some(p) = pool.get_page(id) {
        println!("Đọc lại trang từ bộ nhớ đệm thành công: #{}", p.page_id);
    }
}
```

---

## Tóm tắt chương & Bài tập rèn luyện (Summary & Exercises)

### 4 Điểm cốt lõi cần ghi nhớ:
1. **Trang chuẩn 4KB**: Là viên gạch nền tảng của mọi hệ thống cơ sở dữ liệu, đồng bộ hoàn hảo với khối khu vực ổ đĩa SSD và trang bộ nhớ ảo của hệ điều hành.
2. **Kiến trúc Slotted-Page**: Thư mục khe (Slot Directory) tiến từ đầu trang xuống, dữ liệu tiến từ đáy trang lên. Giải quyết triệt để vấn đề bản ghi có độ dài biến thiên mà không gây phân mảnh ô nhớ.
3. **Định danh Tuple ID `(page_id, slot_id)`**: Cho phép các bảng chỉ mục trỏ chính xác tới bản ghi mà không phụ thuộc vào vị trí byte vật lý bên trong trang.
4. **Buffer Pool và LRU**: Giữ các trang nóng trên RAM để tăng tốc x1000 lần; tự động chọn trang nguội nhất để trục xuất (evict) về đĩa cứng khi bộ nhớ đệm (buffer) đầy.

### Bài tập rèn luyện tự giải:
1. **Bài tập 1 (Tính toán dung lượng Slotted-Page)**:  
   Giả sử mỗi bản ghi có kích thước trung bình là 100 bytes. Hãy tính xem một trang Slotted-Page 4096 bytes (với Header 8 bytes và mỗi khe Slot chiếm 4 bytes) có thể chứa tối đa bao nhiêu bản ghi?
2. **Bài tập 2 (Xóa bản ghi trong Slotted-Page)**:  
   Hãy viết thêm phương thức `fn xoa_ban_ghi(&mut self, slot_id: u16) -> bool` cho `SlottedPage`. Để xóa bản ghi, ta chỉ cần gán độ dài khe `length = 0` trong Slot Directory (đánh dấu Tombstone) mà không cần phải dời dữ liệu bên dưới đáy.
3. **Bài tập 3 (Cơ chế Pin Count)**:  
   Tại sao trong các hệ quản trị cơ sở dữ liệu thực tế, Buffer Pool phải có thêm trường `pin_count: usize` (số luồng đang đọc trang)? Nếu một trang có `pin_count > 0` thì thuật toán LRU có được phép trục xuất (evict) trang đó không? Vì sao?
