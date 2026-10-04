#![allow(dead_code, unused_variables, unused_imports)]
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
