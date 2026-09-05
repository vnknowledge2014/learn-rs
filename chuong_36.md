# Chương 36: Dự án lớn: Xây dựng động cơ lưu trữ Mini-Bitcask Key-Value bền vững (Capstone Project: Building a Persistent Mini-Bitcask Key-Value Engine)

## Giới thiệu & Mục tiêu học tập

Chúc mừng bạn đã tiến tới chặng đường cuối cùng của **Chủ đề 6: Kiến trúc & Thiết kế Cơ sở Dữ liệu**! Trong suốt 11 chương vừa qua của hai chủ đề DSA và Database Internals, bạn đã trang bị cho mình một kho tàng kiến thức đồ sộ: Từ độ phức tạp Big-O, mảng liền kề, danh sách liên kết, cây nhị phân, bảng băm, đến thao tác nhị phân trên đĩa cứng, kiến trúc Slotted-Page, bộ đệm Buffer Pool, cây B+ Tree, nhật ký WAL, LSM-Tree và giao dịch MVCC.

Giờ là lúc chúng ta ghép nối tất cả những mảnh ghép rời rạc đó thành một cỗ máy hoàn chỉnh mang tính sản xuất: **Tự tay xây dựng một Động cơ Cơ sở Dữ liệu Khóa-Giá trị Bền vững (Persistent Key-Value Store) mang tên Mini-Bitcask từ con số không!**

Mô hình **Bitcask** là kiến trúc động cơ lưu trữ lừng danh được sáng chế bởi hãng Basho Technologies (được sử dụng làm trái tim cho cơ sở dữ liệu phân tán quy mô lớn Riak). Bitcask sở hữu một triết lý thiết kế thanh lịch đến kinh ngạc:
- **Tốc độ ghi siêu khủng**: Toàn bộ thao tác ghi chỉ là nối đuôi tuần tự vào cuối tệp tin trên đĩa cứng (Append-only Log).
- **Tốc độ đọc tức thì**: Tra cứu vị trí trên RAM thông qua Bảng băm mục lục (**KeyDir**) chỉ mất $O(1)$, sau đó nhảy thẳng tới đúng tọa độ byte trên đĩa để đọc giá trị chỉ với **duy nhất 1 lần đọc đĩa (Single Disk Seek)**!

Mục tiêu học tập của chương dự án lớn này:
- Nắm vững kiến trúc lai (Hybrid Architecture) kết hợp giữa Bảng băm trên RAM (`KeyDir`) và Tệp dữ liệu ghi nối đuôi trên Đĩa cứng (Append-only Data File).
- Tự tay lập trình đầy đủ các thao tác cơ bản: `set(key, value)`, `get(key)`, và `delete(key)` với định dạng đóng gói nhị phân tùy chỉnh.
- Hiện thực hóa cơ chế **Khởi động và Phục hồi sau sự cố (Crash Recovery & Startup Index Rebuild)**: Tự động quét lại tệp dữ liệu để dựng lại chỉ mục RAM khi máy chủ khởi động lại.
- Xây dựng tiến trình **Nén gộp và Dọn dẹp dữ liệu (Compaction & Merge)** giúp loại bỏ các bản ghi cũ bị ghi đè hoặc bị xóa, thu nhỏ dung lượng tệp đĩa tối đa.
- Rèn luyện kỹ năng viết mã nguồn Rust hướng module chuyên nghiệp, xử lý lỗi an toàn với `Result<T, io::Error>`, và kiểm thử tự động (Integration Testing).

---

## Hình tượng hóa đời sống (Intuitive Everyday Analogy)

Hãy cùng quan sát cách người chủ tiệm tạp hóa quản lý sổ sách nợ nần để hiểu thấu kiến trúc Bitcask:

```
┌──────────────────────────────────────────────────────────────────────────────────┐
│              HÌNH TƯỢNG HÓA KIẾN TRÚC ĐỘNG CƠ LƯU TRỮ BITCASK                    │
├──────────────────────────────────────────────────────────────────────────────────┤
│                                                                                  │
│ [1. TRÊN RAM: BẢNG MỤC LỤC DÁN NGOÀI BÌA SỔ (KEYDIR INDEX)]                      │
│ ┌──────────────────────┬────────────────────────┬──────────────────────┐         │
│ │ Tên khách hàng (Key) │ Tọa độ trang (Offset)  │ Độ dài chữ (ValSize) │         │
│ ├──────────────────────┼────────────────────────┼──────────────────────┤         │
│ │ "Bác Ba"             │ Byte #450              │ 12 bytes             │         │
│ │ "Chị Năm"            │ Byte #780              │ 15 bytes             │         │
│ │ "Chú Bảy"            │ Byte #920              │ 10 bytes             │         │
│ └──────────────────────┴────────────────────────┴──────────────────────┘         │
│            │                                                                     │
│            │ Tra cứu trên bìa sổ mất 1 tích tắc O(1)!                            │
│            ▼                                                                     │
│ [2. DƯỚI ĐĨA CỨNG: CUỐN SỔ CÁI GHI NỐI ĐUÔI (APPEND-ONLY DATA FILE)]             │
│ ┌──────────────────────────────────────────────────────────────────────┐         │
│ │ Byte 0: [Khởi tạo sổ ngày 01/01]                                     │         │
│ │ Byte 200: Giao dịch cũ: "Bác Ba nợ 20k" (Bị ghi đè)                  │         │
│ │ Byte 450: Giao dịch mới: "Bác Ba nợ 50k" ◄── Nhảy thẳng tới đọc 1 lần│         │
│ │ Byte 780: Giao dịch: "Chị Năm nợ 100k"                               │         │
│ │ Byte 920: Giao dịch: "Chú Bảy nợ 30k"                                │         │
│ │ Byte 1100: [Ghi tiếp vào đuôi sổ...] ◄── Ghi mới không cần sửa trang cũ│        │
│ └──────────────────────────────────────────────────────────────────────┘         │
└──────────────────────────────────────────────────────────────────────────────────┘
```

### 1. Cuốn sổ cái ghi nợ kèm Bảng mục lục dán ngoài bìa
- **Dưới đĩa cứng (Cuốn sổ cái)**:
  - Mọi giao dịch phát sinh trong ngày đều được ghi nối tiếp vào các dòng trống tiếp theo ở cuối cuốn sổ (thao tác `append`).
  - Bạn không bao giờ lấy tẩy xóa dòng cũ (vì việc tẩy xóa làm bẩn giấy và mất thời gian). Nếu Bác Ba nợ thêm tiền hoặc trả bớt nợ, bạn chỉ việc ghi một dòng mới toanh ở cuối sổ: *"Hôm nay Bác Ba nợ 50k"*.
- **Trên RAM (Bảng mục lục dán ngoài bìa sổ - KeyDir)**:
  - Để không phải lật từng trang sổ tìm tên Bác Ba, bạn dán một tờ giấy nhớ ở ngoài bìa cuốn sổ.
  - Trên tờ giấy nhớ ghi rõ: `"Bác Ba -> Xem trang 45 dòng 3 (Byte #450), đọc đúng 12 chữ"`.
  - Mỗi khi ghi một dòng mới vào cuối sổ, bạn chỉ cần lấy bút gạch số trang cũ trên tờ giấy nhớ và ghi đè số trang mới vào.
- **Tốc độ đọc**: Khi khách hỏi nợ, bạn liếc tờ giấy nhớ ngoài bìa (RAM tốn $O(1)$), biết ngay trang 45, bạn lật phắt một cái đến đúng trang 45 đọc to số nợ (**đúng 1 lần lật sổ - 1 Disk Seek**).
- **Tốc độ ghi**: Viết tiếp vào cuối sổ trong 1 giây mà không làm phiền bất kỳ trang sổ nào trước đó.

### 2. Dọn dẹp sổ nợ (Compaction & Merge)
- Sau 6 tháng, cuốn sổ cái dày cộm lên hàng ngàn trang, trong đó chứa rất nhiều dòng nợ cũ đã lỗi thời của Bác Ba và Chị Năm.
- Cuối năm, bác chủ tiệm mua một cuốn sổ mới tinh, mở tờ giấy mục lục ngoài bìa ra và chỉ chép lại các số nợ mới nhất còn hiệu lực sang cuốn sổ mới, vứt bỏ toàn bộ các trang giấy nợ cũ đã bị hủy. Cuốn sổ lại trở nên mỏng nhẹ tinh tươm!

---

## Khái niệm & Cơ chế kỹ thuật chuyên sâu (In-Depth Technical Mechanics)

### 1. Định dạng bản ghi nhị phân trên Đĩa cứng (On-Disk Record Format)

Mỗi bản ghi được lưu xuống tệp dữ liệu tuân theo cấu trúc nhị phân chuẩn hóa sau:

```
┌───────────────┬────────────────┬────────────────┬────────────────┬──────────────┬────────────────┐
│ Dấu mốc (8B)  │ Cờ xóa (1B)    │ Dài Khóa (4B)  │ Dài Giá trị(4B)│ Khóa (Key)   │ Giá trị (Val)  │
│ timestamp u64 │ is_deleted u8  │ key_len u32    │ val_len u32    │ [u8; key_len]│ [u8; val_len]  │
└───────────────┴────────────────┴────────────────┴────────────────┴──────────────┴────────────────┘
```
- **`timestamp` (8 bytes)**: Dấu mốc thời gian Unix Epoch (nanoseconds) ghi nhận thời điểm bản ghi được tạo ra.
- **`is_deleted` (1 byte)**: Cờ đánh dấu bản ghi đã bị xóa (Tombstone). Giá trị `0` là hợp lệ, `1` là đã bị xóa.
- **`key_len` (4 bytes)** và **`val_len` (4 bytes)**: Kích thước của khóa và giá trị theo định dạng Little-Endian.
- **Tổng kích thước Header cố định**: $8 + 1 + 4 + 4 = 17 \text{ bytes}$.

### 2. Cấu trúc Chỉ mục bộ nhớ trong (`KeyDir`)

Trên thanh RAM, `KeyDir` là một bảng băm ánh xạ từ Khóa sang cấu trúc chỉ dẫn vị trí ô nhớ:
```rust
pub struct KeyDirEntry {
    pub file_offset: u64,    // Tọa độ byte bắt đầu của phần thân giá trị
    pub value_size: usize,   // Độ dài byte của giá trị để đọc đúng số lượng
    pub timestamp: u64,      // Dấu mốc thời gian của bản ghi
}
```
Khi người dùng gọi `get("user:101")`:
1. Tra cứu `"user:101"` trong `HashMap` trên RAM -> Nhận về `KeyDirEntry { file_offset: 1024, value_size: 40 }`.
2. Gọi lệnh `file.seek(SeekFrom::Start(1024))` để nhảy đầu đọc đĩa tới byte thứ 1024.
3. Gọi lệnh `file.read_exact(&mut buffer)` đọc đúng 40 bytes dữ liệu.
4. Trả về kết quả tức thì. Toàn bộ thao tác chỉ tiêu tốn đúng **1 lần I/O đĩa**!

### 3. Quy trình Nén gộp và Hợp nhất (Compaction & Merge)

Khi số lượng thao tác cập nhật và xóa tăng cao, tệp dữ liệu sẽ bị phình to bởi các bản ghi "rác" (bản ghi cũ đã bị ghi đè hoặc bị đánh dấu cờ xóa `is_deleted = 1`).

Thuật toán Compaction diễn ra tuần tự như sau:
1. Tạo một tệp dữ liệu mới tạm thời: `data.db.compact`.
2. Duyệt qua từng cặp `(key, entry)` hiện có trong bảng mục lục `KeyDir` trên RAM (đây là những bản ghi mới nhất và còn hiệu lực sống).
3. Đọc dữ liệu từ tệp cũ tại `entry.file_offset` và ghi nối tiếp sang tệp mới `data.db.compact`.
4. Cập nhật lại `file_offset` trong `KeyDir` trỏ sang tọa độ mới trong tệp nén.
5. Đóng tệp, thực hiện hoán đổi nguyên tử (Atomic Rename): Đổi tên `data.db.compact` đè lên tệp `data.db` ban đầu. Dung lượng đĩa được giải phóng hoàn toàn!

---

## Mã nguồn minh họa thực chiến (Idiomatic Runnable Rust Blueprint)

Dưới đây là mã nguồn hoàn chỉnh của động cơ lưu trữ **Mini-Bitcask Engine** được viết bằng Safe Rust 100%, hỗ trợ đầy đủ các tính năng: Ghi nối đuôi nhị phân, tra cứu RAM 1 lần đọc đĩa, xóa mềm Tombstone, phục hồi tự động khi mở tệp, và nén gộp dọn rác Compaction:

```rust
use std::convert::TryInto;
use std::collections::HashMap;
use std::fs::{File, OpenOptions};
use std::io::{self, Read, Seek, SeekFrom, Write};
use std::path::Path;
use std::time::{SystemTime, UNIX_EPOCH};

/// Cấu trúc mục lục chỉ dẫn vị trí bản ghi nằm trên RAM
#[derive(Debug, Clone, PartialEq)]
pub struct KeyDirEntry {
    pub file_offset: u64,  // Tọa độ byte bắt đầu của phần Giá trị (Value) trên đĩa
    pub value_size: usize, // Độ dài byte của Giá trị
    pub timestamp: u64,    // Thời điểm ghi nhận
}

/// Động cơ lưu trữ Mini-Bitcask Engine
pub struct MiniBitcask {
    file: File,
    keydir: HashMap<String, KeyDirEntry>,
    file_path: String,
    current_offset: u64,
}

impl MiniBitcask {
    /// Mở hoặc tạo mới cơ sở dữ liệu Bitcask tại đường dẫn chỉ định
    pub fn open<P: AsRef<Path>>(path: P) -> io::Result<Self> {
        let path_str = path.as_ref().to_str().unwrap().to_string();
        let mut file = OpenOptions::new()
            .read(true)
            .write(true)
            .create(true)
            .open(&path)?;

        let file_len = file.seek(SeekFrom::End(0))?;
        let mut bitcask = Self {
            file,
            keydir: HashMap::new(),
            file_path: path_str,
            current_offset: file_len,
        };

        // Khôi phục lại toàn bộ chỉ mục KeyDir trên RAM từ tệp đĩa
        bitcask.rebuild_keydir()?;
        Ok(bitcask)
    }

    /// Quét tuần tự toàn bộ tệp từ byte 0 để dựng lại chỉ mục RAM (Startup Recovery)
    fn rebuild_keydir(&mut self) -> io::Result<()> {
        let file_len = self.file.seek(SeekFrom::End(0))?;
        if file_len == 0 {
            return Ok(());
        }

        self.file.seek(SeekFrom::Start(0))?;
        let mut con_tro: u64 = 0;

        // Header: [Timestamp: 8B] [is_deleted: 1B] [k_len: 4B] [v_len: 4B] = 17 bytes
        while con_tro < file_len {
            let mut header = [0u8; 17];
            if let Err(e) = self.file.read_exact(&mut header) {
                if e.kind() == io::ErrorKind::UnexpectedEof {
                    break; // Hết tệp
                }
                return Err(e);
            }

            let timestamp = u64::from_le_bytes(header[0..8].try_into().unwrap());
            let is_deleted = header[8];
            let k_len = u32::from_le_bytes(header[9..13].try_into().unwrap()) as usize;
            let v_len = u32::from_le_bytes(header[13..17].try_into().unwrap()) as usize;

            // Đọc khóa (Key)
            let mut k_buf = vec![0u8; k_len];
            self.file.read_exact(&mut k_buf)?;
            let key = String::from_utf8_lossy(&k_buf).to_string();

            // Tọa độ bắt đầu của phần Value trên đĩa
            let value_offset = con_tro + 17 + k_len as u64;

            // Nhảy cóc qua phần Value để đến bản ghi tiếp theo
            self.file.seek(SeekFrom::Current(v_len as i64))?;
            con_tro = value_offset + v_len as u64;

            // Cập nhật KeyDir trên RAM
            if is_deleted == 1 {
                self.keydir.remove(&key);
            } else {
                self.keydir.insert(
                    key,
                    KeyDirEntry {
                        file_offset: value_offset,
                        value_size: v_len,
                        timestamp,
                    },
                );
            }
        }

        self.current_offset = file_len;
        println!("    [REBUILD]: Đã phục hồi thành công {} khóa hợp lệ vào RAM!", self.keydir.len());
        Ok(())
    }

    /// THAO TÁC GHI (Set): Ghi nối đuôi vào tệp và cập nhật RAM
    pub fn set(&mut self, key: &str, value: &str) -> io::Result<()> {
        let now = SystemTime::now()
            .duration_since(UNIX_EPOCH)
            .unwrap()
            .as_nanos() as u64;

        let k_bytes = key.as_bytes();
        let v_bytes = value.as_bytes();
        let k_len = k_bytes.len() as u32;
        let v_len = v_bytes.len() as u32;

        // Đóng gói bản ghi nhị phân
        let mut buffer = Vec::with_capacity(17 + k_bytes.len() + v_bytes.len());
        buffer.extend_from_slice(&now.to_le_bytes()); // Timestamp (8B)
        buffer.push(0);                               // is_deleted = 0 (1B)
        buffer.extend_from_slice(&k_len.to_le_bytes());// Key length (4B)
        buffer.extend_from_slice(&v_len.to_le_bytes());// Val length (4B)
        buffer.extend_from_slice(k_bytes);            // Key
        buffer.extend_from_slice(v_bytes);            // Value

        // 1. Nhảy đến cuối tệp để ghi nối đuôi (Append-only)
        self.file.seek(SeekFrom::End(0))?;
        let record_offset = self.current_offset;
        self.file.write_all(&buffer)?;
        self.file.flush()?;

        let value_offset = record_offset + 17 + k_bytes.len() as u64;
        self.current_offset += buffer.len() as u64;

        // 2. Cập nhật mục lục KeyDir trên RAM
        self.keydir.insert(
            key.to_string(),
            KeyDirEntry {
                file_offset: value_offset,
                value_size: v_bytes.len(),
                timestamp: now,
            },
        );

        Ok(())
    }

    /// THAO TÁC ĐỌC (Get): Tra cứu RAM và nhảy đúng 1 lần đọc đĩa
    pub fn get(&mut self, key: &str) -> io::Result<Option<String>> {
        if let Some(entry) = self.keydir.get(key).cloned() {
            // Nhảy thẳng tới tọa độ byte của Value trên đĩa
            self.file.seek(SeekFrom::Start(entry.file_offset))?;
            let mut v_buf = vec![0u8; entry.value_size];
            self.file.read_exact(&mut v_buf)?;
            let val_str = String::from_utf8(v_buf).map_err(|e| {
                io::Error::new(io::ErrorKind::InvalidData, e.to_string())
            })?;
            Ok(Some(val_str))
        } else {
            Ok(None)
        }
    }

    /// THAO TÁC XÓA (Delete): Ghi bản ghi Tombstone xuống tệp và xóa khỏi RAM
    pub fn delete(&mut self, key: &str) -> io::Result<bool> {
        if !self.keydir.contains_key(key) {
            return Ok(false);
        }

        let now = SystemTime::now()
            .duration_since(UNIX_EPOCH)
            .unwrap()
            .as_nanos() as u64;

        let k_bytes = key.as_bytes();
        let k_len = k_bytes.len() as u32;

        let mut buffer = Vec::with_capacity(17 + k_bytes.len());
        buffer.extend_from_slice(&now.to_le_bytes()); // Timestamp (8B)
        buffer.push(1);                               // is_deleted = 1 (Tombstone!)
        buffer.extend_from_slice(&k_len.to_le_bytes());// Key length (4B)
        buffer.extend_from_slice(&0u32.to_le_bytes()); // Val length = 0 (4B)
        buffer.extend_from_slice(k_bytes);

        self.file.seek(SeekFrom::End(0))?;
        self.file.write_all(&buffer)?;
        self.file.flush()?;

        self.current_offset += buffer.len() as u64;
        self.keydir.remove(key);

        Ok(true)
    }

    /// TIẾN TRÌNH NÉN GỘP VÀ DỌN RÁC (Compaction & Merge)
    pub fn compact(&mut self) -> io::Result<()> {
        let compact_path = format!("{}.compact", self.file_path);
        {
            let mut new_file = OpenOptions::new()
                .read(true)
                .write(true)
                .create(true)
                .truncate(true)
                .open(&compact_path)?;

            let mut new_keydir = HashMap::new();
            let mut new_offset: u64 = 0;

            // Đọc các bản ghi còn hiệu lực từ tệp cũ và ghi sang tệp mới
            for (key, entry) in &self.keydir {
                self.file.seek(SeekFrom::Start(entry.file_offset))?;
                let mut v_buf = vec![0u8; entry.value_size];
                self.file.read_exact(&mut v_buf)?;

                let k_bytes = key.as_bytes();
                let k_len = k_bytes.len() as u32;
                let v_len = v_buf.len() as u32;

                let mut buffer = Vec::with_capacity(17 + k_bytes.len() + v_buf.len());
                buffer.extend_from_slice(&entry.timestamp.to_le_bytes());
                buffer.push(0); // Không bị xóa
                buffer.extend_from_slice(&k_len.to_le_bytes());
                buffer.extend_from_slice(&v_len.to_le_bytes());
                buffer.extend_from_slice(k_bytes);
                buffer.extend_from_slice(&v_buf);

                new_file.write_all(&buffer)?;
                let new_val_offset = new_offset + 17 + k_bytes.len() as u64;
                new_offset += buffer.len() as u64;

                new_keydir.insert(
                    key.clone(),
                    KeyDirEntry {
                        file_offset: new_val_offset,
                        value_size: v_buf.len(),
                        timestamp: entry.timestamp,
                    },
                );
            }
            new_file.flush()?;
        }

        // Hoán đổi tệp nén mới đè lên tệp cũ
        std::fs::rename(&compact_path, &self.file_path)?;

        // Mở lại tệp dữ liệu đã nén
        self.file = OpenOptions::new()
            .read(true)
            .write(true)
            .open(&self.file_path)?;
        let new_len = self.file.seek(SeekFrom::End(0))?;
        self.current_offset = new_len;

        // Dựng lại chỉ mục từ tệp mới
        self.keydir.clear();
        self.rebuild_keydir()?;
        Ok(())
    }

    pub fn total_keys(&self) -> usize {
        self.keydir.len()
    }

    pub fn file_size(&self) -> u64 {
        self.current_offset
    }
}

fn main() -> io::Result<()> {
    println!("============================================================");
    println!("  DỰ ÁN LỚN: ĐỘNG CƠ LƯU TRỮ PERSISTENT MINI-BITCASK TRONG RUST");
    println!("============================================================");

    let db_path = "mini_bitcask_test.db";
    let _ = std::fs::remove_file(db_path);

    // GIAI ĐOẠN 1: Khởi tạo cơ sở dữ liệu và thực hiện ghi chép
    println!("[1] Mở MiniBitcask và thêm các cặp khóa - giá trị:");
    {
        let mut db = MiniBitcask::open(db_path)?;

        db.set("user:101", "Alice - Ha Noi")?;
        db.set("user:102", "Bob - Da Nang")?;
        db.set("user:103", "Charlie - TP Ho Chi Minh")?;

        // Ghi đè cập nhật giá trị (tạo ra dữ liệu cũ trên đĩa)
        db.set("user:101", "Alice Nguyen - Ha Noi (Updated)")?;

        // Xóa một khóa (tạo Tombstone trên đĩa)
        db.delete("user:102")?;

        println!("    - Kích thước tệp đĩa hiện tại: {} bytes", db.file_size());
        println!("    - Tổng số khóa hợp lệ trên RAM: {}", db.total_keys());

        // Kiểm tra đọc dữ liệu qua 1 lần Disk Seek
        assert_eq!(db.get("user:101")?, Some("Alice Nguyen - Ha Noi (Updated)".to_string()));
        assert_eq!(db.get("user:102")?, None);
        assert_eq!(db.get("user:103")?, Some("Charlie - TP Ho Chi Minh".to_string()));
        println!("    => Các thao tác CRUD ban đầu hoạt động hoàn hảo!");
    } // db đóng tệp an toàn tại đây

    // GIAI ĐOẠN 2: Kiểm thử tính năng phục hồi sau sự cố (Crash Recovery)
    println!("\n[2] Kiểm tra phục hồi dữ liệu khi khởi động lại ứng dụng:");
    {
        let mut db_recovered = MiniBitcask::open(db_path)?;
        println!("    - Đã mở lại tệp '{}'", db_path);
        println!("    - Kiểm tra dữ liệu sau phục hồi:");
        println!("      + 'user:101' = {:?}", db_recovered.get("user:101")?);
        println!("      + 'user:102' = {:?}", db_recovered.get("user:102")?);
        println!("      + 'user:103' = {:?}", db_recovered.get("user:103")?);

        assert_eq!(db_recovered.get("user:101")?, Some("Alice Nguyen - Ha Noi (Updated)".to_string()));
        assert_eq!(db_recovered.get("user:102")?, None);
        assert_eq!(db_recovered.get("user:103")?, Some("Charlie - TP Ho Chi Minh".to_string()));
        assert_eq!(db_recovered.total_keys(), 2);
        println!("    => Khôi phục chỉ mục KeyDir trên RAM từ đĩa thành công 100%!");

        // GIAI ĐOẠN 3: Kiểm thử tiến trình nén gộp dọn rác (Compaction & Merge)
        println!("\n[3] Thực thi tiến trình nén gộp dọn rác Compaction:");
        let dung_luong_truoc = db_recovered.file_size();
        db_recovered.compact()?;
        let dung_luong_sau = db_recovered.file_size();

        println!("    - Dung lượng tệp TRƯỚC nén gộp: {} bytes", dung_luong_truoc);
        println!("    - Dung lượng tệp SAU nén gộp   : {} bytes", dung_luong_sau);
        assert!(dung_luong_sau < dung_luong_truoc);

        // Kiểm tra dữ liệu sau nén gộp vẫn còn nguyên vẹn
        assert_eq!(db_recovered.get("user:101")?, Some("Alice Nguyen - Ha Noi (Updated)".to_string()));
        assert_eq!(db_recovered.get("user:103")?, Some("Charlie - TP Ho Chi Minh".to_string()));
        println!("    => Tiến trình Compaction đã dọn sạch toàn bộ rác thừa trên đĩa!");
    }

    // Dọn dẹp tệp thử nghiệm
    let _ = std::fs::remove_file(db_path);

    println!("============================================================");
    println!("     CHÚC MỪNG BẠN ĐÃ HOÀN THÀNH XUẤT SẮC DỰ ÁN LỚN 6!     ");
    println!("============================================================");
    Ok(())
}
```

---

## Bảng tra cứu lỗi biên dịch & Cách khắc phục (Compiler Error Guide)

Dưới đây là các lỗi biên dịch thường gặp nhất khi hiện thực hóa động cơ Bitcask trong Rust:

| Mã lỗi | Thông báo mẫu từ trình biên dịch | Nguyên nhân cốt lõi | Cách khắc phục nhanh |
|---|---|---|---|
| **E0502** | `cannot borrow '*self' as mutable because it is also borrowed as immutable` | Bạn gọi `self.keydir.get(key)` trả về tham chiếu mượn, sau đó gọi tiếp `self.file.seek(...)` làm mượn khả biến toàn bộ struct `self`. | Sử dụng `.cloned()` để sao chép cấu trúc nhẹ `KeyDirEntry` ra biến độc lập trên Stack trước khi thao tác với tệp tin. |
| **E0599** | `no method named 'seek' found for struct 'File'` | Bạn sử dụng phương thức `.seek()` để nhảy tọa độ byte trên đĩa nhưng quên đưa trait `Seek` vào phạm vi hoạt động. | Thêm dòng khai báo: `use std::io::Seek;` ở đầu tệp mã nguồn. |
| **E0382** | `use of moved value: 'compact_path'` | Bạn truyền `compact_path` vào hàm `rename()` khiến chuỗi bị di chuyển, sau đó lại dùng lại nó ở dòng lệnh kế tiếp. | Truyền mượn tham chiếu `&compact_path` vào hàm `std::fs::rename`. |
| **E0061** | `this function takes 1 argument but 0 arguments were supplied` | Gọi hàm `UNIX_EPOCH` hoặc `SystemTime::now()` sai cú pháp. | Dùng chuẩn: `SystemTime::now().duration_since(UNIX_EPOCH).unwrap().as_nanos()`. |

### Ví dụ phân tích lỗi `E0502` khi vừa tra cứu KeyDir vừa đọc File:

```rust
use std::fs::File;
use std::collections::HashMap;

struct DemoStore {
    file: File,
    index: HashMap<String, u64>,
}

// Đoạn mã lỗi minh họa E0502: Mượn lồng nhau gây xung đột
impl DemoStore {
    fn doc_loi(&mut self, key: &str) {
        // let offset = self.index.get(key); // Mượn bất biến self.index
        // self.file.set_len(100).unwrap();  // LỖI E0502: Mượn khả biến self.file!
        // println!("Offset: {:?}", offset);
    }

    // Cách sửa chữa đúng chuẩn: Sao chép (copy) giá trị số nguyên ra trước
    fn doc_dung(&mut self, key: &str) {
        let offset = self.index.get(key).copied(); // offset là biến độc lập trên Stack
        if let Some(pos) = offset {
            println!("Đã lấy được tọa độ an toàn: {}", pos);
        }
    }
}
```

---

## Tóm tắt chương & Bài tập rèn luyện (Summary & Exercises)

### 4 Điểm cốt lõi cần ghi nhớ:
1. **Thiết kế lai hoàn hảo**: Bitcask kết hợp tinh hoa của Bảng băm trên RAM (`KeyDir`) cho tốc độ tra cứu $O(1)$ và Tệp ghi nối đuôi (Append-only) trên đĩa cho tốc độ ghi tối đa.
2. **Đúng 1 lần đọc đĩa (Single Disk Seek)**: Nhờ biết chính xác tọa độ byte (`offset`) và độ dài (`value_size`) từ RAM, thao tác đọc dữ liệu bỏ qua mọi tầng trung gian, nhảy thẳng tới vị trí đĩa cần đọc.
3. **Cơ chế Tombstone**: Thay vì tìm xóa tại chỗ trên đĩa (gây phân mảnh và chậm chạp), Bitcask ghi một bản ghi đánh dấu xóa (Tombstone) vào cuối tệp và xóa khỏi RAM.
4. **Nén gộp Compaction**: Tiến trình dọn dẹp định kỳ đọc lại các khóa còn sống và ghi sang tệp mới, giữ cho cơ sở dữ liệu luôn nhỏ gọn và loại bỏ hoàn toàn các phiên bản dữ liệu cũ.

### Bài tập rèn luyện tự giải:
1. **Bài tập 1 (Bổ sung mã kiểm tra toàn vẹn CRC32)**:  
   Mở rộng phần Header của bản ghi trong `MiniBitcask` thêm 4 bytes chứa mã kiểm tra toàn vẹn CRC32 (`crc32fast::Hasher` hoặc tự cài đặt thuật toán kiểm tra tổng kiểm tra checksum đơn giản). Khi đọc lại tệp trong hàm `rebuild_keydir`, tính toán lại mã CRC32 của bản ghi, nếu không khớp thì dừng lại và bỏ qua bản ghi bị lỗi.
2. **Bài tập 2 (Tạo tệp Hint File tối ưu hóa)**:  
   Sau khi tiến trình `compact()` hoàn tất, hãy cho ghi thêm một tệp gợi ý `data.db.hint` chỉ chứa các cặp `(key, KeyDirEntry)`. Khi hệ thống khởi động lại, thay vì phải quét toàn bộ tệp dữ liệu lớn, hệ thống chỉ cần đọc tệp Hint File nhỏ bé để khôi phục RAM trong vài mili-giây.
3. **Bài tập 3 (Giới hạn của Bitcask)**:  
   Điểm yếu lớn nhất của mô hình Bitcask là gì? Nếu cơ sở dữ liệu có 1 tỷ khóa khác nhau thì thanh RAM có thể chứa nổi `KeyDir` không? Trong trường hợp đó, người ta sẽ chuyển sang sử dụng mô hình nào (B+ Tree hay LSM-Tree)?
