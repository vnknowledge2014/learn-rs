#![allow(dead_code, unused_variables, unused_imports)]
use std::convert::TryInto;
use std::collections::HashMap;

/// Kích thước trang chuẩn của cơ sở dữ liệu (4KB)
pub const PAGE_SIZE: usize = 4096;

/// Cấu trúc khe lưu trữ trong thư mục Slotted-Page (4 bytes)
#[derive(Debug, Clone, Copy)]
pub struct KheBanGhi {
    pub offset: u16,
    pub length: u16,
}

/// Cấu trúc Trang dữ liệu phân khe chuẩn 4KB (Slotted-Page)
pub struct SlottedPage {
    pub page_id: u32,
    pub du_lieu: [u8; PAGE_SIZE],
}

impl SlottedPage {
    /// Khởi tạo một trang mới tinh kích thước 4096 bytes
    pub fn new(page_id: u32) -> Self {
        let mut trang = Self {
            page_id,
            du_lieu: [0u8; PAGE_SIZE],
        };
        // Ghi Header ban đầu:
        // Byte 0..4: page_id
        trang.du_lieu[0..4].copy_from_slice(&page_id.to_le_bytes());
        // Byte 4..6: slot_count = 0
        trang.du_lieu[4..6].copy_from_slice(&0u16.to_le_bytes());
        // Byte 6..8: free_space_pointer = 4096 (đáy trang)
        trang.du_lieu[6..8].copy_from_slice(&(PAGE_SIZE as u16).to_le_bytes());
        trang
    }

    pub fn lay_so_khe(&self) -> u16 {
        u16::from_le_bytes(self.du_lieu[4..6].try_into().unwrap())
    }

    fn gan_so_khe(&mut self, count: u16) {
        self.du_lieu[4..6].copy_from_slice(&count.to_le_bytes());
    }

    pub fn lay_con_tro_day(&self) -> u16 {
        u16::from_le_bytes(self.du_lieu[6..8].try_into().unwrap())
    }

    fn gan_con_tro_day(&mut self, ptr: u16) {
        self.du_lieu[6..8].copy_from_slice(&ptr.to_le_bytes());
    }

    /// Thêm một bản ghi nhị phân vào trang - Trả về slot_id (chỉ số khe)
    pub fn them_ban_ghi(&mut self, bytes_ban_ghi: &[u8]) -> Option<u16> {
        let so_khe_hien_tai = self.lay_so_khe();
        let con_tro_day = self.lay_con_tro_day();
        let do_dai_ghi = bytes_ban_ghi.len() as u16;

        // Tính toán vị trí tiêu tốn của Slot Directory ở trên đầu trang:
        // Header: 8 bytes. Mỗi khe: 4 bytes.
        let vi_tri_khe_moi = 8 + (so_khe_hien_tai as usize * 4);
        let dung_luong_con_lai = con_tro_day as usize - (vi_tri_khe_moi + 4);

        // Kiểm tra xem trang còn đủ chỗ cho cả Slot mới lẫn thân dữ liệu không
        if do_dai_ghi as usize > dung_luong_con_lai {
            return None; // Trang đã đầy (Page Full)!
        }

        // 1. Tính tọa độ đáy mới và ghi dữ liệu từ đáy trang ngược lên
        let offset_day_moi = con_tro_day - do_dai_ghi;
        let bat_dau = offset_day_moi as usize;
        let ket_thuc = con_tro_day as usize;
        self.du_lieu[bat_dau..ket_thuc].copy_from_slice(bytes_ban_ghi);

        // 2. Ghi thông tin Khe vào Slot Directory ở đầu trang
        self.du_lieu[vi_tri_khe_moi..vi_tri_khe_moi + 2].copy_from_slice(&offset_day_moi.to_le_bytes());
        self.du_lieu[vi_tri_khe_moi + 2..vi_tri_khe_moi + 4].copy_from_slice(&do_dai_ghi.to_le_bytes());

        // 3. Cập nhật Header
        self.gan_so_khe(so_khe_hien_tai + 1);
        self.gan_con_tro_day(offset_day_moi);

        Some(so_khe_hien_tai)
    }

    /// Đọc bản ghi qua slot_id - O(1)
    pub fn doc_ban_ghi(&self, slot_id: u16) -> Option<&[u8]> {
        let so_khe = self.lay_so_khe();
        if slot_id >= so_khe {
            return None;
        }

        let vi_tri_khe = 8 + (slot_id as usize * 4);
        let offset = u16::from_le_bytes(self.du_lieu[vi_tri_khe..vi_tri_khe + 2].try_into().unwrap()) as usize;
        let length = u16::from_le_bytes(self.du_lieu[vi_tri_khe + 2..vi_tri_khe + 4].try_into().unwrap()) as usize;

        Some(&self.du_lieu[offset..offset + length])
    }
}

/// Khung trang quản lý bên trong Buffer Pool
pub struct Frame {
    pub trang: SlottedPage,
    pub is_dirty: bool,
}

/// Hệ thống quản lý bộ nhớ đệm Buffer Pool với thuật toán LRU Eviction
pub struct BufferPool {
    suc_chua: usize,
    frames: HashMap<u32, Frame>,
    lru_danh_sach: Vec<u32>, // Quản lý thứ tự: Đầu danh sách là nguội nhất (LRU)
}

impl BufferPool {
    pub fn new(suc_chua: usize) -> Self {
        Self {
            suc_chua,
            frames: HashMap::new(),
            lru_danh_sach: Vec::new(),
        }
    }

    /// Cập nhật trang vừa được truy cập xuống cuối danh sách LRU
    fn cap_nhat_lru(&mut self, page_id: u32) {
        self.lru_danh_sach.retain(|&id| id != page_id);
        self.lru_danh_sach.push(page_id);
    }

    /// Lấy trang từ bộ nhớ đệm (nếu có)
    pub fn get_page(&mut self, page_id: u32) -> Option<&SlottedPage> {
        if self.frames.contains_key(&page_id) {
            self.cap_nhat_lru(page_id);
            return self.frames.get(&page_id).map(|f| &f.trang);
        }
        None
    }

    /// Đưa trang vào Buffer Pool - Nếu đầy, tự động trục xuất (evict) trang cũ nhất
    pub fn put_page(&mut self, trang: SlottedPage, is_dirty: bool) {
        let id = trang.page_id;

        // Nếu trang chưa có trong buffer và buffer đã đầy sức chứa
        if !self.frames.contains_key(&id) && self.frames.len() >= self.suc_chua {
            // Trục xuất trang ở đầu danh sách LRU (nguội nhất)
            let evict_id = self.lru_danh_sach.remove(0);
            if let Some(khung_cu) = self.frames.remove(&evict_id) {
                if khung_cu.is_dirty {
                    println!("    [EVICT]: Trang #{} có cờ bẩn (is_dirty=true) -> Đang ghi đè xuống đĩa SSD...", evict_id);
                } else {
                    println!("    [EVICT]: Trang #{} sạch (chưa sửa) -> Hủy khỏi RAM tức thì mà không cần ghi đĩa.", evict_id);
                }
            }
        }

        self.frames.insert(id, Frame { trang, is_dirty });
        self.cap_nhat_lru(id);
    }

    pub fn so_trang_hien_co(&self) -> usize {
        self.frames.len()
    }
}

fn main() {
    println!("============================================================");
    println!("  KIẾN TRÚC SLOTTED-PAGE 4KB & QUẢN LÝ BỘ NHỚ ĐỆM BUFFER POOL");
    println!("============================================================");

    // 1. Khảo sát cấu trúc trang SlottedPage kích thước 4KB
    println!("[1] Thao tác trên Trang phân khe Slotted-Page (4096 bytes):");
    let mut trang_1 = SlottedPage::new(1);
    println!("    - Khởi tạo Trang #1. Kích thước bộ đệm vật lý: {} bytes", trang_1.du_lieu.len());
    println!("    - Con trỏ đáy tự do ban đầu: {} (Đáy trang)", trang_1.lay_con_tro_day());

    // Nạp các bản ghi có kích thước chuỗi thay đổi
    let ban_ghi_a = b"NguoiDung: Nguyen Van An - Ha Noi";
    let ban_ghi_b = b"NguoiDung: Tran Thi Binh - TP Ho Chi Minh (VIP Member)";
    let ban_ghi_c = b"NguoiDung: Le Hoang Cuong - Da Nang";

    let slot_a = trang_1.them_ban_ghi(ban_ghi_a).expect("Lỗi chèn khe A");
    let slot_b = trang_1.them_ban_ghi(ban_ghi_b).expect("Lỗi chèn khe B");
    let slot_c = trang_1.them_ban_ghi(ban_ghi_c).expect("Lỗi chèn khe C");

    println!("    - Đã chèn Bản ghi A -> Được cấp Tuple ID: (Page: 1, Slot: {})", slot_a);
    println!("    - Đã chèn Bản ghi B -> Được cấp Tuple ID: (Page: 1, Slot: {})", slot_b);
    println!("    - Đã chèn Bản ghi C -> Được cấp Tuple ID: (Page: 1, Slot: {})", slot_c);
    println!("    - Tổng số khe: {}, Con trỏ đáy hiện tại: {}", trang_1.lay_so_khe(), trang_1.lay_con_tro_day());

    // Đọc lại nội dung qua Slot ID
    let doc_b = trang_1.doc_ban_ghi(slot_b).unwrap();
    println!("    - Đọc nội dung qua Slot ID {}: '{}'", slot_b, String::from_utf8_lossy(doc_b));
    assert_eq!(doc_b, ban_ghi_b);

    // 2. Khảo sát hệ thống Buffer Pool và thuật toán trục xuất LRU Eviction
    println!("\n[2] Vận hành Buffer Pool với sức chứa tối đa 2 trang:");
    let mut buffer_pool = BufferPool::new(2);

    // Đưa Trang 1 và Trang 2 vào Buffer Pool
    println!("    - Nạp Trang #1 (đã sửa đổi -> dirty=true) vào Buffer Pool");
    buffer_pool.put_page(trang_1, true);

    let trang_2 = SlottedPage::new(2);
    println!("    - Nạp Trang #2 (chỉ đọc -> dirty=false) vào Buffer Pool");
    buffer_pool.put_page(trang_2, false);

    println!("    - Số trang hiện có trong Buffer: {}", buffer_pool.so_trang_hien_co());
    assert_eq!(buffer_pool.so_trang_hien_co(), 2);

    // Người dùng truy cập lại Trang 1 -> Trang 1 trở thành trang dùng gần nhất
    println!("\n    - Người dùng đọc Trang #1 -> Cập nhật thứ tự ưu tiên LRU cho Trang #1!");
    assert!(buffer_pool.get_page(1).is_some());

    // Giờ đây, Trang #2 là trang "nguội nhất" (lâu nhất không dùng).
    // Khi nạp thêm Trang #3 vào, Buffer Pool sẽ kích hoạt trục xuất (evict) Trang #2!
    println!("\n    - Nạp Trang #3 mới tinh vào (Vượt quá sức chứa 2 trang):");
    let trang_3 = SlottedPage::new(3);
    buffer_pool.put_page(trang_3, false);

    // Kiểm tra: Trang 2 đã bị loại bỏ, Trang 1 và Trang 3 vẫn nằm trong Buffer Pool
    assert!(buffer_pool.get_page(2).is_none());
    assert!(buffer_pool.get_page(1).is_some());
    assert!(buffer_pool.get_page(3).is_some());
    println!("    => Thuật toán LRU Eviction vận hành chuẩn xác 100%!");

    println!("============================================================");
    println!("               HOÀN TẤT THỰC NGHIỆM CHƯƠNG 28               ");
    println!("============================================================");
}
