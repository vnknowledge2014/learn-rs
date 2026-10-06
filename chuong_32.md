# Chương 32: Kiến trúc trang Slotted-Page & Quản lý bộ nhớ đệm Buffer Pool (Slotted-Page Architecture & Buffer Pool Management)

## Giới thiệu & Mục tiêu học tập

Trong chương trước, chúng ta đã hiểu nguyên lý lưu trữ đĩa cứng và kỹ thuật đóng gói nhị phân. Tuy nhiên, nếu mỗi lần thêm một bản ghi mới cơ sở dữ liệu lại thực hiện một thao tác ghi đĩa riêng lẻ, hệ thống sẽ sụp đổ hiệu năng vì nghẽn cổ chai I/O. Hơn nữa, trong thực tế, các bản ghi luôn có kích thước thay đổi (biến thiên): Người có tên 5 ký tự, người có tên 50 ký tự; khi một người dùng xóa tài khoản, khoảng trống ô nhớ bỏ lại sẽ bị phân mảnh nếu không có cách tổ chức khoa học.

Mọi hệ quản trị cơ sở dữ liệu hàng đầu thế giới (như PostgreSQL, MySQL InnoDB, SQLite) giải quyết bài toán này thông qua hai trụ cột kiến trúc cốt lõi:
1. **Kiến trúc trang phân khe (Slotted-Page Architecture)**: Phân chia tệp dữ liệu trên đĩa thành các khối có kích thước cố định (SQLite mặc định 4KB, PostgreSQL 8KB, MySQL InnoDB 16KB — chương này dùng **4KB = 4096 bytes**), cho phép chứa các bản ghi có độ dài co giãn linh hoạt mà không sợ phân mảnh ô nhớ.
2. **Bộ quản lý bộ nhớ đệm (Buffer Pool Manager)**: Một vùng đệm RAM trung gian giữ các trang dữ liệu nóng (hot pages) để người dùng đọc/ghi tức thì, kết hợp thuật toán **LRU (Least Recently Used)** để tự động trục xuất (evict) các trang cũ về đĩa cứng khi bộ nhớ đệm (buffer) bị đầy.

Mục tiêu học tập của chương này:
- Nắm vững cấu tạo vật lý của một **Trang dữ liệu kích thước cố định (Fixed-size Page)** và lý do kích thước trang thường là bội số của 4KB — đơn vị mà phần cứng SSD và hệ điều hành làm việc.
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

### 1. Tại sao trang có kích thước cố định — và vì sao thường là bội số của 4KB?

Kích thước trang **không** phải lúc nào cũng là 4KB: SQLite mặc định 4096 bytes, PostgreSQL dùng 8KB, MySQL InnoDB dùng 16KB, nhiều hệ cho phép cấu hình. Điểm chung là chúng đều **cố định** và là **bội số của 4KB**, vì:
1. **Kiến trúc phần cứng SSD/HDD**: Các khối khu vực (sectors) vật lý của đĩa hiện đại (Advanced Format) có kích thước 4096 bytes; trang SSD bên trong còn lớn hơn nữa.
2. **Bộ nhớ ảo (Virtual Memory)**: Đơn vị quản lý bộ nhớ mặc định của nhân Linux/Windows trên x86-64 là trang 4KB (Apple Silicon dùng 16KB).
3. Khi trang cơ sở dữ liệu là bội số của đơn vị phần cứng và được căn đúng ranh giới, một thao tác đọc/ghi trang không bao giờ phải chạm "nửa sector" — tránh được chu trình đọc-sửa-ghi lãng phí. Trang lớn hơn (8KB, 16KB) đổi lại chứa được nhiều khoá hơn mỗi lần đọc, rất có lợi cho B+Tree (Chương 33).

Chương này dùng 4KB cho đơn giản; đổi hằng số `PAGE_SIZE` là đổi được kích thước (miễn còn vừa trong `u16`, tối đa 32KB với bố cục hiện tại).

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

Buffer Pool là một hệ thống đệm tinh vi gồm:
1. **Bảng khung trang (Frame Table)**: Mảng các ô nhớ, mỗi ô vừa đúng một trang, trên RAM (`Page Frames`).
2. **Bảng tra cứu trang (Page Table)**: Bảng băm ánh xạ từ `page_id` trên đĩa sang số thứ tự khung trang `frame_id` trên RAM.
3. **Cờ bẩn (Dirty Flag)**: Một bit đánh dấu trang đã bị sửa đổi dữ liệu hay chưa. Nếu cờ bẩn bật (`is_dirty == true`), khi trục xuất trang về đĩa cứng, hệ thống bắt buộc phải ghi nội dung đệm xuống đĩa. Nếu chưa bị sửa (`is_dirty == false`), chỉ cần hủy bỏ khỏi RAM mà không tốn lệnh ghi đĩa nào. Quy tắc quan trọng: cờ bẩn chỉ được **bật lên** khi trang bị sửa và chỉ được **xoá** khi trang đã thực sự được ghi xuống đĩa — một lần truy cập "chỉ đọc" sau đó không được phép xoá nó.
4. **Hàng đợi LRU (LRU List)**: Quản lý thứ tự thời gian sử dụng của các trang. Mỗi lần một trang được đọc hoặc ghi, nó được chuyển xuống cuối danh sách (vừa sử dụng gần nhất). Trang ở đầu danh sách là trang "nguội" nhất, sẽ là nạn nhân đầu tiên bị trục xuất khi bộ nhớ đầy.

---

## Mã nguồn minh họa thực chiến (Idiomatic Runnable Rust Blueprint)

Dưới đây là một chương trình Rust hoàn chỉnh và độc lập, cài đặt cả hai cấu trúc:
1. Cấu trúc `SlottedPage` chuẩn 4096 bytes với khả năng thêm bản ghi, đọc bản ghi qua `slot_id`.
2. Hệ thống `BufferPool` hoàn chỉnh với dung lượng giới hạn, bảng băm tra cứu, cờ bẩn `is_dirty`, và cơ chế trục xuất trang theo thuật toán `LRU`:

```rust
use std::collections::{HashMap, VecDeque};
use std::convert::TryInto;

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
    /// Khởi tạo một trang mới tinh kích thước 4096 bytes
    pub fn new(page_id: u32) -> Self {
        let mut page = Self {
            page_id,
            data: [0u8; PAGE_SIZE],
        };
        // Ghi Header ban đầu:
        // Byte 0..4: page_id
        page.data[0..4].copy_from_slice(&page_id.to_le_bytes());
        // Byte 4..6: slot_count = 0
        page.data[4..6].copy_from_slice(&0u16.to_le_bytes());
        // Byte 6..8: free_space_pointer = 4096 (đáy trang)
        page.data[6..8].copy_from_slice(&(PAGE_SIZE as u16).to_le_bytes());
        page
    }

    pub fn slot_count(&self) -> u16 {
        u16::from_le_bytes(self.data[4..6].try_into().unwrap())
    }

    fn set_slot_count(&mut self, count: u16) {
        self.data[4..6].copy_from_slice(&count.to_le_bytes());
    }

    pub fn free_space_pointer(&self) -> u16 {
        u16::from_le_bytes(self.data[6..8].try_into().unwrap())
    }

    fn set_free_space_pointer(&mut self, ptr: u16) {
        self.data[6..8].copy_from_slice(&ptr.to_le_bytes());
    }

    /// Thêm một bản ghi nhị phân vào trang - Trả về slot_id (chỉ số khe)
    pub fn insert_record(&mut self, record: &[u8]) -> Option<u16> {
        let current_slot_count = self.slot_count();
        let free_space_pointer = self.free_space_pointer();

        // Tính toán vị trí tiêu tốn của Slot Directory ở trên đầu trang:
        // Header: 8 bytes. Mỗi khe: 4 bytes.
        let new_slot_index = 8 + (current_slot_count as usize * 4);
        // checked_sub: khi bảng khe đã chạm vùng dữ liệu, phép trừ sẽ âm —
        // trừ thẳng trên usize sẽ panic (bản debug) hoặc quay vòng (bản release).
        let Some(free_space) = (free_space_pointer as usize).checked_sub(new_slot_index + 4) else {
            return None; // Không còn chỗ cho cả một khe mới
        };

        // Kiểm tra xem trang còn đủ chỗ cho cả Slot mới lẫn thân dữ liệu không
        // (so sánh trên usize TRƯỚC khi ép về u16 để không bị cắt cụt)
        if record.len() > free_space {
            return None; // Trang đã đầy (Page Full)!
        }
        let record_len = record.len() as u16;

        // 1. Tính tọa độ đáy mới và ghi dữ liệu từ đáy trang ngược lên
        let new_offset = free_space_pointer - record_len;
        let start = new_offset as usize;
        let end = free_space_pointer as usize;
        self.data[start..end].copy_from_slice(record);

        // 2. Ghi thông tin Khe vào Slot Directory ở đầu trang
        self.data[new_slot_index..new_slot_index + 2].copy_from_slice(&new_offset.to_le_bytes());
        self.data[new_slot_index + 2..new_slot_index + 4]
            .copy_from_slice(&record_len.to_le_bytes());

        // 3. Cập nhật Header
        self.set_slot_count(current_slot_count + 1);
        self.set_free_space_pointer(new_offset);

        Some(current_slot_count)
    }

    /// Đọc bản ghi qua slot_id - O(1)
    pub fn read_record(&self, slot_id: u16) -> Option<&[u8]> {
        let slot_count = self.slot_count();
        if slot_id >= slot_count {
            return None;
        }

        let slot_offset = 8 + (slot_id as usize * 4);
        let offset = u16::from_le_bytes(self.data[slot_offset..slot_offset + 2].try_into().unwrap())
            as usize;
        let length = u16::from_le_bytes(
            self.data[slot_offset + 2..slot_offset + 4]
                .try_into()
                .unwrap(),
        ) as usize;

        Some(&self.data[offset..offset + length])
    }
}

/// Khung trang quản lý bên trong Buffer Pool
pub struct Frame {
    pub page: SlottedPage,
    pub is_dirty: bool,
}

/// Hệ thống quản lý bộ nhớ đệm Buffer Pool với thuật toán LRU Eviction
pub struct BufferPool {
    capacity: usize,
    frames: HashMap<u32, Frame>,
    lru_list: VecDeque<u32>, // Quản lý thứ tự: Đầu danh sách là nguội nhất (LRU)
}

impl BufferPool {
    pub fn new(capacity: usize) -> Self {
        Self {
            capacity,
            frames: HashMap::new(),
            lru_list: VecDeque::new(),
        }
    }

    /// Cập nhật trang vừa được truy cập xuống cuối danh sách LRU
    fn touch_lru(&mut self, page_id: u32) {
        self.lru_list.retain(|&id| id != page_id);
        self.lru_list.push_back(page_id);
    }

    /// Lấy trang từ bộ nhớ đệm (nếu có)
    pub fn get_page(&mut self, page_id: u32) -> Option<&SlottedPage> {
        if self.frames.contains_key(&page_id) {
            self.touch_lru(page_id);
            return self.frames.get(&page_id).map(|f| &f.page);
        }
        None
    }

    /// Đưa trang vào Buffer Pool - Nếu đầy, tự động trục xuất (evict) trang cũ nhất
    pub fn put_page(&mut self, page: SlottedPage, is_dirty: bool) {
        let id = page.page_id;

        // Nếu trang chưa có trong buffer và buffer đã đầy sức chứa
        if !self.frames.contains_key(&id) && self.frames.len() >= self.capacity {
            // Trục xuất trang ở đầu danh sách LRU (nguội nhất)
            // (VecDeque::pop_front là O(1) — xem Chương 28 về Vec::remove(0))
            if let Some(evict_id) = self.lru_list.pop_front()
                && let Some(evicted) = self.frames.remove(&evict_id)
            {
                if evicted.is_dirty {
                    println!(
                        "    [EVICT]: Trang #{} có cờ bẩn (is_dirty=true) -> Đang ghi đè xuống đĩa SSD...",
                        evict_id
                    );
                } else {
                    println!(
                        "    [EVICT]: Trang #{} sạch (chưa sửa) -> Hủy khỏi RAM tức thì mà không cần ghi đĩa.",
                        evict_id
                    );
                }
            }
        }

        // Nếu trang ĐÃ có trong buffer và đang bẩn, việc đưa lại một bản "sạch"
        // KHÔNG được xoá cờ bẩn: những thay đổi trước đó vẫn chưa được ghi xuống đĩa.
        // Cờ bẩn chỉ được xoá khi trang thực sự được ghi ra đĩa.
        let was_dirty = self.frames.get(&id).is_some_and(|f| f.is_dirty);
        self.frames.insert(
            id,
            Frame {
                page,
                is_dirty: was_dirty || is_dirty,
            },
        );
        self.touch_lru(id);
    }

    /// Trang có đang nằm trong buffer và có cờ bẩn không? (None nếu không có trong buffer)
    pub fn is_dirty(&self, page_id: u32) -> Option<bool> {
        self.frames.get(&page_id).map(|f| f.is_dirty)
    }

    pub fn page_count(&self) -> usize {
        self.frames.len()
    }
}

fn main() {
    println!("============================================================");
    println!("  KIẾN TRÚC SLOTTED-PAGE 4KB & QUẢN LÝ BỘ NHỚ ĐỆM BUFFER POOL");
    println!("============================================================");

    // 1. Khảo sát cấu trúc trang SlottedPage kích thước 4KB
    println!("[1] Thao tác trên Trang phân khe Slotted-Page (4096 bytes):");
    let mut page_1 = SlottedPage::new(1);
    println!(
        "    - Khởi tạo Trang #1. Kích thước bộ đệm vật lý: {} bytes",
        page_1.data.len()
    );
    println!(
        "    - Con trỏ đáy tự do ban đầu: {} (Đáy trang)",
        page_1.free_space_pointer()
    );

    // Nạp các bản ghi có kích thước chuỗi thay đổi
    let record_a = "Người dùng: Nguyễn Văn An - Hà Nội".as_bytes();
    let record_b = "Người dùng: Trần Thị Bình - TP Hồ Chí Minh (Thành viên VIP)".as_bytes();
    let record_c = "Người dùng: Lê Hoàng Cường - Đà Nẵng".as_bytes();

    let slot_a = page_1.insert_record(record_a).expect("Lỗi chèn khe A");
    let slot_b = page_1.insert_record(record_b).expect("Lỗi chèn khe B");
    let slot_c = page_1.insert_record(record_c).expect("Lỗi chèn khe C");

    println!(
        "    - Đã chèn Bản ghi A -> Được cấp Tuple ID: (Page: 1, Slot: {})",
        slot_a
    );
    println!(
        "    - Đã chèn Bản ghi B -> Được cấp Tuple ID: (Page: 1, Slot: {})",
        slot_b
    );
    println!(
        "    - Đã chèn Bản ghi C -> Được cấp Tuple ID: (Page: 1, Slot: {})",
        slot_c
    );
    println!(
        "    - Tổng số khe: {}, Con trỏ đáy hiện tại: {}",
        page_1.slot_count(),
        page_1.free_space_pointer()
    );

    // Đọc lại nội dung qua Slot ID
    let read_b = page_1.read_record(slot_b).unwrap();
    println!(
        "    - Đọc nội dung qua Slot ID {}: '{}'",
        slot_b,
        String::from_utf8_lossy(read_b)
    );
    assert_eq!(read_b, record_b);

    // 2. Khảo sát hệ thống Buffer Pool và thuật toán trục xuất LRU Eviction
    println!("\n[2] Vận hành Buffer Pool với sức chứa tối đa 2 trang:");
    let mut buffer_pool = BufferPool::new(2);

    // Đưa Trang 1 và Trang 2 vào Buffer Pool
    println!("    - Nạp Trang #1 (đã sửa đổi -> dirty=true) vào Buffer Pool");
    buffer_pool.put_page(page_1, true);

    let page_2 = SlottedPage::new(2);
    println!("    - Nạp Trang #2 (chỉ đọc -> dirty=false) vào Buffer Pool");
    buffer_pool.put_page(page_2, false);

    println!(
        "    - Số trang hiện có trong Buffer: {}",
        buffer_pool.page_count()
    );
    assert_eq!(buffer_pool.page_count(), 2);

    // Người dùng truy cập lại Trang 1 -> Trang 1 trở thành trang dùng gần nhất
    println!("\n    - Người dùng đọc Trang #1 -> Cập nhật thứ tự ưu tiên LRU cho Trang #1!");
    assert!(buffer_pool.get_page(1).is_some());

    // Giờ đây, Trang #2 là trang "nguội nhất" (lâu nhất không dùng).
    // Khi nạp thêm Trang #3 vào, Buffer Pool sẽ kích hoạt trục xuất (evict) Trang #2!
    println!("\n    - Nạp Trang #3 mới tinh vào (Vượt quá sức chứa 2 trang):");
    let page_3 = SlottedPage::new(3);
    buffer_pool.put_page(page_3, false);

    // Kiểm tra: Trang 2 đã bị loại bỏ, Trang 1 và Trang 3 vẫn nằm trong Buffer Pool
    assert!(buffer_pool.get_page(2).is_none());
    assert!(buffer_pool.get_page(1).is_some());
    assert!(buffer_pool.get_page(3).is_some());
    println!("    => Thuật toán LRU Eviction vận hành chuẩn xác 100%!");

    println!("============================================================");
    println!("               HOÀN TẤT THỰC NGHIỆM CHƯƠNG 32               ");
    println!("============================================================");
}
```

---

## Bảng tra cứu lỗi biên dịch & Cách khắc phục (Compiler Error Guide)

Dưới đây là các lỗi biên dịch điển hình khi thiết kế Slotted-Page và hệ thống Buffer Pool trong Rust:

| Mã lỗi | Thông báo mẫu từ trình biên dịch | Nguyên nhân cốt lõi | Cách khắc phục nhanh |
|---|---|---|---|
| **E0382** | `use of moved value: 'page'` | Bạn truyền `page` vào hàm `put_page()` khiến quyền sở hữu (ownership) bị chuyển giao, sau đó lại dùng lại biến `page` ở dòng dưới. | Gọi hàm đọc thông qua Buffer Pool `buffer_pool.get_page(id)` thay vì sử dụng trực tiếp biến cũ đã bị di chuyển. |
| **E0499** | `cannot borrow '*pool' as mutable more than once at a time` | Bạn giữ kết quả `let p = pool.get_page(1);` rồi gọi `pool.put_page(...)` trong cùng phạm vi. Vì `get_page` nhận `&mut self` (để cập nhật thứ tự LRU), tham chiếu `p` trả về kéo dài một lần mượn **khả biến** — nên lỗi là "mượn khả biến hai lần", không phải E0502. | Giới hạn phạm vi mượn đọc hoặc sao chép dữ liệu cần thiết ra trước khi thực hiện thêm trang mới. |
| **E0277** | `the trait bound '[u8; 4096]: Default' is not satisfied` | Bạn viết `#[derive(Default)]` cho struct chứa `data: [u8; 4096]`. Dù hầu hết trait của mảng đã được cài cho mọi độ dài nhờ const generics, `Default` cho mảng **đến nay vẫn chỉ có tới độ dài 32**. Bản thân biểu thức `[0u8; PAGE_SIZE]` thì hoàn toàn hợp lệ. | Khởi tạo mảng tường minh: `[0u8; PAGE_SIZE]`. |
| **E0308** | `mismatched types: expected 'u16', found 'usize'` | Các chỉ số trong Header của Slotted-Page dùng `u16` để tiết kiệm byte đĩa, trong khi độ dài của mảng trên RAM là `usize`. | Thực hiện ép kiểu tường minh an toàn: `len as u16` sau khi kiểm tra không vượt quá 4096 bytes. |

### Ví dụ phân tích lỗi `E0382` khi quản lý quyền sở hữu trang:

```rust
// Đoạn mã lỗi minh họa E0382: Di chuyển quyền sở hữu trang vào Buffer Pool
fn broken_page(mut pool: BufferPool, page: SlottedPage) {
    // pool.put_page(page, false); // Quyền sở hữu trang bị chuyển vào HashMap!
    // println!("Mã trang: {}", page.page_id); // LỖI E0382: page đã bị move!
}

// Cách sửa chữa đúng chuẩn: Lấy mã ID ra trước hoặc truy cập qua Pool
fn correct_page(mut pool: BufferPool, page: SlottedPage) {
    let id = page.page_id;
    pool.put_page(page, false);
    println!("Mã trang vừa nạp: {}", id);
    if let Some(p) = pool.get_page(id) {
        println!("Đọc lại trang từ bộ nhớ đệm thành công: #{}", p.page_id);
    }
}
```

---

## Kiểm thử tự động (Automated Tests)

Module kiểm thử dưới đây (đặt ở cuối `main.rs`, chạy bằng `cargo test`) khoá lại các hành vi quan trọng nhất của trang và Buffer Pool — trong đó có hai lỗi biên rất dễ mắc: phép trừ `usize` bị tràn khi bảng khe chạm vùng dữ liệu, và việc đưa lại một trang "sạch" vô tình xoá mất cờ bẩn của bản đang nằm trong buffer (khiến thay đổi chưa ghi bị vứt bỏ lúc trục xuất).

```rust
#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn insert_and_read_back() {
        let mut page = SlottedPage::new(7);
        let a = page.insert_record(b"alpha").unwrap();
        let b = page.insert_record(b"beta").unwrap();
        assert_eq!(page.read_record(a), Some(&b"alpha"[..]));
        assert_eq!(page.read_record(b), Some(&b"beta"[..]));
        assert_eq!(page.read_record(2), None);
        assert_eq!(page.free_space_pointer() as usize, PAGE_SIZE - 5 - 4);
    }

    #[test]
    fn exactly_39_records_of_100_bytes_fit() {
        // Đối chiếu với lời giải Bài tập 1: (4096 - 8) / (100 + 4) = 39
        let mut page = SlottedPage::new(1);
        let record = [0xABu8; 100];
        for i in 0..39 {
            assert_eq!(page.insert_record(&record), Some(i));
        }
        assert_eq!(page.insert_record(&record), None);
    }

    #[test]
    fn full_page_returns_none_instead_of_panicking() {
        // Lấp đầy trang sao cho vùng trống còn lại đúng bằng 1 khe (4 byte),
        // rồi chèn bản ghi rỗng: bản cũ trừ usize bị tràn và panic ở đây.
        let mut page = SlottedPage::new(1);
        let fill = PAGE_SIZE - 8 - 4 - 4; // chừa đúng 4 byte cho khe thứ hai
        assert!(page.insert_record(&vec![1u8; fill]).is_some());
        assert!(page.insert_record(&[]).is_some()); // khe thứ hai vừa khít
        assert_eq!(page.insert_record(&[]), None); // không còn chỗ cho khe thứ ba
        assert_eq!(page.insert_record(b"x"), None);
    }

    #[test]
    fn oversized_record_is_rejected_not_truncated() {
        let mut page = SlottedPage::new(1);
        // 65_540 as u16 == 4: nếu ép kiểu trước khi so sánh, bản ghi này sẽ "lọt" qua
        assert_eq!(page.insert_record(&vec![0u8; 65_540]), None);
        assert_eq!(page.slot_count(), 0);
    }

    #[test]
    fn lru_evicts_least_recently_used() {
        let mut pool = BufferPool::new(2);
        pool.put_page(SlottedPage::new(1), false);
        pool.put_page(SlottedPage::new(2), false);
        assert!(pool.get_page(1).is_some()); // 1 vừa dùng -> 2 thành nguội nhất
        pool.put_page(SlottedPage::new(3), false);
        assert!(pool.get_page(2).is_none());
        assert!(pool.get_page(1).is_some());
        assert!(pool.get_page(3).is_some());
        assert_eq!(pool.page_count(), 2);
    }

    #[test]
    fn re_putting_clean_page_keeps_dirty_flag() {
        // Lỗi cũ: put lại một trang đang bẩn với is_dirty=false xoá mất cờ bẩn,
        // nên lúc trục xuất trang sẽ bị vứt đi mà không ghi xuống đĩa.
        let mut pool = BufferPool::new(2);
        pool.put_page(SlottedPage::new(1), true);
        assert_eq!(pool.is_dirty(1), Some(true));
        pool.put_page(SlottedPage::new(1), false);
        assert_eq!(pool.is_dirty(1), Some(true));
        assert_eq!(pool.page_count(), 1);
    }
}
```

## Tóm tắt chương & Bài tập rèn luyện (Summary & Exercises)

### 4 Điểm cốt lõi cần ghi nhớ:
1. **Trang kích thước cố định**: Là viên gạch nền tảng của mọi hệ thống cơ sở dữ liệu (4KB, 8KB hay 16KB tuỳ hệ), luôn là bội số của khối ổ đĩa và trang bộ nhớ ảo 4KB của hệ điều hành.
2. **Kiến trúc Slotted-Page**: Thư mục khe (Slot Directory) tiến từ đầu trang xuống, dữ liệu tiến từ đáy trang lên. Giải quyết triệt để vấn đề bản ghi có độ dài biến thiên mà không gây phân mảnh ô nhớ.
3. **Định danh Tuple ID `(page_id, slot_id)`**: Cho phép các bảng chỉ mục trỏ chính xác tới bản ghi mà không phụ thuộc vào vị trí byte vật lý bên trong trang.
4. **Buffer Pool và LRU**: Giữ các trang nóng trên RAM để tăng tốc x1000 lần; tự động chọn trang nguội nhất để trục xuất (evict) về đĩa cứng khi bộ nhớ đệm (buffer) đầy.

### Bài tập rèn luyện tự giải:
1. **Bài tập 1 (Tính toán dung lượng Slotted-Page)**:  
   Giả sử mỗi bản ghi có kích thước trung bình là 100 bytes. Hãy tính xem một trang Slotted-Page 4096 bytes (với Header 8 bytes và mỗi khe Slot chiếm 4 bytes) có thể chứa tối đa bao nhiêu bản ghi?
2. **Bài tập 2 (Xóa bản ghi trong Slotted-Page)**:  
   Hãy viết thêm phương thức `fn delete_record(&mut self, slot_id: u16) -> bool` cho `SlottedPage`. Để xóa bản ghi, ta chỉ cần gán độ dài khe `length = 0` trong Slot Directory (đánh dấu Tombstone) mà không cần phải dời dữ liệu bên dưới đáy.
3. **Bài tập 3 (Cơ chế Pin Count)**:  
   Tại sao trong các hệ quản trị cơ sở dữ liệu thực tế, Buffer Pool phải có thêm trường `pin_count: usize` (số luồng đang đọc trang)? Nếu một trang có `pin_count > 0` thì thuật toán LRU có được phép trục xuất (evict) trang đó không? Vì sao?

---

### Gợi ý & Lời giải

<details>
<summary><b>Bài tập 1 — Gợi ý</b></summary>

Mỗi bản ghi tốn hai thứ: dữ liệu ở đáy trang, **và** một khe 4 byte trong bảng khe ở đầu trang. Đừng quên phần thứ hai.
</details>

<details>
<summary><b>Bài tập 1 — Lời giải</b></summary>

Mỗi bản ghi tiêu tốn **hai** vùng, không phải một:

```
chi phí mỗi bản ghi = 100 byte dữ liệu + 4 byte khe = 104 byte
không gian dùng được = 4096 − 8 (header) = 4088 byte
số bản ghi tối đa    = 4088 ÷ 104 = 39,3 → 39 bản ghi
```

Kiểm lại (test `exactly_39_records_of_100_bytes_fit` ở trên kiểm chứng bằng mã): 39 × 100 = 3900 byte dữ liệu, 39 × 4 = 156 byte bảng khe, cộng 8 byte header là **4064** — vừa trong 4096, còn thừa 32 byte.

Thử 40 bản ghi: 4000 + 160 + 8 = **4168** > 4096. Không đủ chỗ.

**Sai lầm hay gặp:** tính `4088 ÷ 100 = 40` rồi kết luận 40 bản ghi. Nó quên mất rằng bảng khe cũng lớn lên theo số bản ghi — hai đầu trang tiến về phía nhau, và chúng gặp nhau sớm hơn bạn tưởng. Đó chính là lý do `insert_record` phải kiểm tra **cả hai** con trỏ trước khi ghi.
</details>

<details>
<summary><b>Bài tập 2 — Gợi ý</b></summary>

Đánh dấu bia mộ nghĩa là **chỉ sửa bảng khe**, không đụng vào dữ liệu ở đáy. Xoá là O(1); phần dọn dẹp để dành cho lúc nén trang.
</details>

<details>
<summary><b>Bài tập 2 — Lời giải</b></summary>

```rust
impl SlottedPage {
    /// Xoá bằng BIA MỘ: chỉ ghi `length = 0` vào bảng khe ở ĐẦU trang.
    /// Dữ liệu ở đáy trang vẫn nằm nguyên đó — đó là CHỦ Ý.
    pub fn delete_record(&mut self, slot_id: u16) -> bool {
        if slot_id >= self.slot_count() { return false; }
        let slot_offset = 8 + (slot_id as usize * 4);
        let length = u16::from_le_bytes(
            self.data[slot_offset + 2..slot_offset + 4].try_into().unwrap());
        if length == 0 { return false; }              // đã là bia mộ rồi
        self.data[slot_offset + 2..slot_offset + 4].copy_from_slice(&0u16.to_le_bytes());
        true
    }

    /// Đọc có TÔN TRỌNG bia mộ. `read_record` gốc không kiểm
    /// `length == 0` nên nó trả về lát cắt RỖNG thay vì `None` —
    /// đúng cú pháp, nhưng che mất sự khác biệt giữa "bản ghi rỗng"
    /// và "bản ghi đã xoá".
    pub fn read_live_record(&self, slot_id: u16) -> Option<&[u8]> {
        if slot_id >= self.slot_count() { return None; }
        let slot_offset = 8 + (slot_id as usize * 4);
        let offset = u16::from_le_bytes(
            self.data[slot_offset..slot_offset + 2].try_into().unwrap()) as usize;
        let length = u16::from_le_bytes(
            self.data[slot_offset + 2..slot_offset + 4].try_into().unwrap()) as usize;
        if length == 0 { return None; }               // bia mộ
        Some(&self.data[offset..offset + length])
    }
}

#[test]
fn delete_by_tombstone() {
    let mut p = SlottedPage::new(1);
    let a = p.insert_record(b"record A").unwrap();
    let b = p.insert_record(b"record B").unwrap();

    assert!(p.delete_record(a));
    assert_eq!(p.read_live_record(a), None, "đã xoá thì đọc ra None");
    assert_eq!(p.read_live_record(b), Some(&b"record B"[..]),
               "B không bị ảnh hưởng");

    assert!(!p.delete_record(a), "xoá hai lần thì lần sau trả false");
    assert!(!p.delete_record(999), "khe không tồn tại");

    // Dữ liệu của A VẪN nằm ở đáy trang — chỉ khe bị đánh dấu.
    // Đó là lý do trang bị "phình xác" cho tới lần nén tiếp theo.
    assert_eq!(p.free_space_pointer() as usize, PAGE_SIZE - 8 - 8);
}
```

**Vì sao không dời dữ liệu ngay:**

1. **Xoá là O(1).** Dời dữ liệu là O(kích thước trang) và phải cập nhật mọi khe phía sau.
2. **Chỉ số khe giữ nguyên.** Nhiều thứ khác trỏ tới bản ghi bằng `(page_id, slot_id)` — chỉ mục, con trỏ từ trang khác. Dồn dữ liệu mà giữ nguyên `slot_id` được là nhờ khe chỉ chứa *offset*, nhưng dồn *khe* thì mọi tham chiếu bên ngoài hỏng hết.
3. **Không gian được thu hồi lúc nén trang**, khi trang đầy và cần chỗ. Gộp nhiều lần xoá vào một lần dọn thì rẻ hơn dọn từng lần.

Cái giá: trang bị **phình xác** (bloat). Postgres gọi đây là *dead tuple*, và tiến trình `VACUUM` chính là bước nén trang đó — Chương 35 nói tiếp.
</details>

<details>
<summary><b>Bài tập 3 — Gợi ý</b></summary>

Nếu một luồng đang *đọc* trang mà trang bị trục xuất, con trỏ nó đang cầm trỏ vào bộ nhớ đã bị tái sử dụng. Đó là dùng-sau-khi-giải-phóng, ở tầng cơ sở dữ liệu.
</details>

<details>
<summary><b>Bài tập 3 — Lời giải</b></summary>

`pin_count` đếm **số luồng đang thực sự dùng trang**, và LRU **tuyệt đối không được** trục xuất trang có `pin_count > 0`.

**Nếu trục xuất thì sao:** một luồng đang cầm `&[u8]` trỏ vào khung nhớ đó. Buffer Pool giải phóng khung, nạp trang khác vào cùng chỗ. Luồng kia đọc tiếp — và thấy dữ liệu của trang hoàn toàn khác. Đây đúng là **dùng-sau-khi-giải-phóng**, chỉ khác là nó xảy ra ở tầng cơ sở dữ liệu chứ không phải tầng cấp phát bộ nhớ. Không sập ngay, mà trả ra dữ liệu sai — dạng lỗi tệ hơn nhiều.

**Vì sao LRU một mình không đủ:** LRU trả lời "trang nào *lâu nhất chưa dùng*". Nhưng "lâu chưa dùng" và "đang được dùng" là hai chuyện khác nhau. Một luồng có thể ghim trang từ lâu rồi ngủ chờ I/O — trang đó vừa là ứng viên LRU tốt nhất, vừa là trang tuyệt đối không được đụng.

```
tìm trang để trục xuất:
    duyệt danh sách LRU từ cũ nhất
        nếu pin_count == 0  ->  chọn trang này
        ngược lại           ->  bỏ qua, xét trang kế
    nếu MỌI trang đều bị ghim -> KHÔNG trục xuất được
        -> trả lỗi "buffer pool đã cạn", không được chờ vô hạn
```

Trường hợp cuối là chuyện có thật: nếu mọi khung đều bị ghim và một luồng lại xin thêm trang, hệ thống **bế tắc**. Vì vậy cơ sở dữ liệu thật giới hạn số trang mỗi giao dịch được ghim cùng lúc, và trả lỗi rõ ràng thay vì treo — cùng nguyên tắc "từ chối rõ ràng khi quá tải" của back-pressure ở Chương 59.

Trong Rust, `pin_count` chính là thứ `Rc`/`Arc` làm tự động: đếm số người đang giữ tham chiếu, và chỉ giải phóng khi đếm về 0.
</details>
