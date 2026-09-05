#![allow(dead_code, unused_variables, unused_imports)]
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
