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
4. **`sync_all()` tệp mới** — ép toàn bộ nội dung của nó xuống đĩa.
5. Thực hiện hoán đổi nguyên tử (Atomic Rename): Đổi tên `data.db.compact` đè lên tệp `data.db` ban đầu.
6. **fsync thư mục chứa tệp** — vì tên tệp là một mục trong thư mục, phép đổi tên chỉ bền vững khi chính thư mục được đồng bộ.
7. Mở lại tệp và dựng lại `KeyDir` trỏ sang tọa độ mới. Dung lượng đĩa được giải phóng hoàn toàn!

> **Vì sao bước 4 và 6 là sống còn:** `rename` là nguyên tử đối với *các tiến trình đang chạy*, nhưng không tự đảm bảo gì khi **mất điện**. Nếu bỏ bước 4, hệ điều hành có thể ghi phép đổi tên xuống đĩa *trước* nội dung tệp mới (nhiều hệ tệp làm vậy). Mất điện đúng lúc đó, bạn còn lại một `data.db` rỗng hoặc dở dang — trong khi bản đầy đủ cũ đã bị thay thế: **mất toàn bộ cơ sở dữ liệu**. Bỏ bước 6 thì sau khi khởi động lại, có thể tên `data.db` vẫn trỏ về tệp cũ — đỡ hơn, nhưng các bước sau (ví dụ xoá tệp cũ) sẽ sai. Thứ tự đúng luôn là: *ghi dữ liệu → fsync tệp → rename → fsync thư mục*.

---

## Mã nguồn minh họa thực chiến (Idiomatic Runnable Rust Blueprint)

Dưới đây là mã nguồn hoàn chỉnh của động cơ lưu trữ **Mini-Bitcask Engine** được viết bằng Safe Rust 100%, hỗ trợ đầy đủ các tính năng: Ghi nối đuôi nhị phân, tra cứu RAM 1 lần đọc đĩa, xóa mềm Tombstone, phục hồi tự động khi mở tệp, và nén gộp dọn rác Compaction:

```rust
use std::collections::HashMap;
use std::fs::{File, OpenOptions};
use std::io::{self, Read, Seek, SeekFrom, Write};
use std::path::{Path, PathBuf};
use std::time::{SystemTime, UNIX_EPOCH};

/// Kích thước phần đầu cố định của một bản ghi:
/// [Timestamp: 8B] [is_deleted: 1B] [k_len: 4B] [v_len: 4B] = 17 bytes
const HEADER_SIZE: u64 = 17;

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
    file_path: PathBuf,
    current_offset: u64,
}

/// Đóng gói một bản ghi nhị phân: header 17 byte + khoá + giá trị
fn encode_record(timestamp: u64, is_deleted: bool, key: &[u8], value: &[u8]) -> Vec<u8> {
    let mut buffer = Vec::with_capacity(HEADER_SIZE as usize + key.len() + value.len());
    buffer.extend_from_slice(&timestamp.to_le_bytes()); // Timestamp (8B)
    buffer.push(is_deleted as u8); // is_deleted (1B): 1 = Tombstone
    buffer.extend_from_slice(&(key.len() as u32).to_le_bytes()); // Key length (4B)
    buffer.extend_from_slice(&(value.len() as u32).to_le_bytes()); // Val length (4B)
    buffer.extend_from_slice(key); // Key
    buffer.extend_from_slice(value); // Value
    buffer
}

fn now_nanos() -> u64 {
    SystemTime::now()
        .duration_since(UNIX_EPOCH)
        .unwrap()
        .as_nanos() as u64
}

/// fsync thư mục chứa `path`: một tệp mới tạo hay vừa đổi tên chỉ thực sự "tồn tại"
/// sau mất điện khi MỤC của nó trong thư mục cũng đã nằm trên đĩa.
fn sync_parent_dir(path: &Path) -> io::Result<()> {
    #[cfg(unix)]
    {
        let parent = match path.parent() {
            Some(p) if !p.as_os_str().is_empty() => p,
            _ => Path::new("."),
        };
        File::open(parent)?.sync_all()?;
    }
    #[cfg(not(unix))]
    let _ = path; // Windows không cho mở thư mục như tệp; NTFS tự ghi nhật ký siêu dữ liệu
    Ok(())
}

impl MiniBitcask {
    /// Mở hoặc tạo mới cơ sở dữ liệu Bitcask tại đường dẫn chỉ định
    pub fn open<P: AsRef<Path>>(path: P) -> io::Result<Self> {
        let file = OpenOptions::new()
            .read(true)
            .write(true)
            .create(true)
            // Mở lại tệp cũ thì phải GIỮ NGUYÊN dữ liệu
            .truncate(false)
            .open(&path)?;

        let mut bitcask = Self {
            file,
            keydir: HashMap::new(),
            file_path: path.as_ref().to_path_buf(),
            current_offset: 0,
        };

        // Khôi phục lại toàn bộ chỉ mục KeyDir trên RAM từ tệp đĩa
        bitcask.rebuild_keydir()?;
        Ok(bitcask)
    }

    /// Quét tuần tự toàn bộ tệp từ byte 0 để dựng lại chỉ mục RAM (Startup Recovery)
    fn rebuild_keydir(&mut self) -> io::Result<()> {
        let file_len = self.file.seek(SeekFrom::End(0))?;
        self.file.seek(SeekFrom::Start(0))?;
        let mut pointer: u64 = 0;

        while pointer < file_len {
            // Bản ghi cuối bị cắt cụt (sập nguồn giữa lúc ghi) -> dừng tại đây
            if pointer + HEADER_SIZE > file_len {
                break;
            }
            let mut header = [0u8; HEADER_SIZE as usize];
            self.file.read_exact(&mut header)?;

            let timestamp = u64::from_le_bytes(header[0..8].try_into().unwrap());
            let is_deleted = header[8];
            let k_len = u32::from_le_bytes(header[9..13].try_into().unwrap()) as u64;
            let v_len = u32::from_le_bytes(header[13..17].try_into().unwrap()) as u64;

            let record_end = pointer + HEADER_SIZE + k_len + v_len;
            if record_end > file_len {
                break; // phần thân bị rách: không được nạp nửa bản ghi
            }

            // Đọc khóa (Key)
            let mut k_buf = vec![0u8; k_len as usize];
            self.file.read_exact(&mut k_buf)?;
            let key = String::from_utf8_lossy(&k_buf).to_string();

            // Tọa độ bắt đầu của phần Value trên đĩa
            let value_offset = pointer + HEADER_SIZE + k_len;

            // Nhảy cóc qua phần Value để đến bản ghi tiếp theo
            self.file.seek(SeekFrom::Current(v_len as i64))?;
            pointer = record_end;

            // Cập nhật KeyDir trên RAM
            if is_deleted == 1 {
                self.keydir.remove(&key);
            } else {
                self.keydir.insert(
                    key,
                    KeyDirEntry {
                        file_offset: value_offset,
                        value_size: v_len as usize,
                        timestamp,
                    },
                );
            }
        }

        if pointer < file_len {
            // Cắt bỏ đuôi rách để bản ghi mới không bị nối vào sau rác
            println!(
                "    [REBUILD]: Bỏ {} byte cuối bị rách do ghi dở",
                file_len - pointer
            );
            self.file.set_len(pointer)?;
            self.file.sync_all()?;
        }
        self.current_offset = pointer;
        println!(
            "    [REBUILD]: Đã phục hồi thành công {} khóa hợp lệ vào RAM!",
            self.keydir.len()
        );
        Ok(())
    }

    /// Ghi nối đuôi một bản ghi và ÉP xuống đĩa; trả về tọa độ bắt đầu của bản ghi
    fn append(&mut self, record: &[u8]) -> io::Result<u64> {
        let record_offset = self.current_offset;
        self.file.seek(SeekFrom::Start(record_offset))?;
        self.file.write_all(record)?;
        // flush() KHÔNG đưa dữ liệu xuống đĩa; sync_data() (fdatasync) mới làm điều đó
        self.file.sync_data()?;
        self.current_offset += record.len() as u64;
        Ok(record_offset)
    }

    /// THAO TÁC GHI (Set): Ghi nối đuôi vào tệp và cập nhật RAM
    pub fn set(&mut self, key: &str, value: &str) -> io::Result<()> {
        let now = now_nanos();
        let record = encode_record(now, false, key.as_bytes(), value.as_bytes());

        // 1. Ghi nối đuôi (Append-only) và đồng bộ xuống đĩa
        let record_offset = self.append(&record)?;
        let value_offset = record_offset + HEADER_SIZE + key.len() as u64;

        // 2. Chỉ SAU KHI dữ liệu đã bền vững mới cập nhật mục lục KeyDir trên RAM
        self.keydir.insert(
            key.to_string(),
            KeyDirEntry {
                file_offset: value_offset,
                value_size: value.len(),
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
            let val_str = String::from_utf8(v_buf)
                .map_err(|e| io::Error::new(io::ErrorKind::InvalidData, e.to_string()))?;
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

        let record = encode_record(now_nanos(), true, key.as_bytes(), &[]);
        self.append(&record)?;
        self.keydir.remove(key);

        Ok(true)
    }

    /// TIẾN TRÌNH NÉN GỘP VÀ DỌN RÁC (Compaction & Merge)
    pub fn compact(&mut self) -> io::Result<()> {
        let mut compact_name = self.file_path.clone().into_os_string();
        compact_name.push(".compact");
        let compact_path = PathBuf::from(compact_name);
        {
            let mut new_file = OpenOptions::new()
                .write(true)
                .create(true)
                .truncate(true)
                .open(&compact_path)?;

            // Đọc các bản ghi còn hiệu lực từ tệp cũ và ghi sang tệp mới
            let entries: Vec<(String, KeyDirEntry)> = self
                .keydir
                .iter()
                .map(|(k, e)| (k.clone(), e.clone()))
                .collect();
            for (key, entry) in entries {
                self.file.seek(SeekFrom::Start(entry.file_offset))?;
                let mut v_buf = vec![0u8; entry.value_size];
                self.file.read_exact(&mut v_buf)?;
                new_file.write_all(&encode_record(
                    entry.timestamp,
                    false,
                    key.as_bytes(),
                    &v_buf,
                ))?;
            }
            // BƯỚC QUAN TRỌNG 1: ép NỘI DUNG tệp mới xuống đĩa TRƯỚC khi đổi tên.
            // Nếu không, sau rename + mất điện ta có thể còn lại một tệp data.db rỗng/dở dang
            // trong khi tệp cũ (bản đầy đủ) đã bị thay thế -> mất dữ liệu.
            new_file.sync_all()?;
        }

        // BƯỚC 2: Hoán đổi nguyên tử tệp nén mới đè lên tệp cũ
        std::fs::rename(&compact_path, &self.file_path)?;
        // BƯỚC 3: fsync thư mục cha để chính việc đổi tên cũng bền vững
        sync_parent_dir(&self.file_path)?;

        // Mở lại tệp dữ liệu đã nén
        self.file = OpenOptions::new()
            .read(true)
            .write(true)
            .open(&self.file_path)?;

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

    let db_path = std::env::temp_dir().join("mini_bitcask_demo.db");
    let _ = std::fs::remove_file(&db_path);

    // GIAI ĐOẠN 1: Khởi tạo cơ sở dữ liệu và thực hiện ghi chép
    println!("[1] Mở MiniBitcask và thêm các cặp khóa - giá trị:");
    {
        let mut db = MiniBitcask::open(&db_path)?;

        db.set("user:101", "Alice - Hà Nội")?;
        db.set("user:102", "Bob - Đà Nẵng")?;
        db.set("user:103", "Charlie - TP Hồ Chí Minh")?;

        // Ghi đè cập nhật giá trị (tạo ra dữ liệu cũ trên đĩa)
        db.set("user:101", "Alice Nguyễn - Hà Nội (đã cập nhật)")?;

        // Xóa một khóa (tạo Tombstone trên đĩa)
        db.delete("user:102")?;

        println!(
            "    - Kích thước tệp đĩa hiện tại: {} bytes",
            db.file_size()
        );
        println!("    - Tổng số khóa hợp lệ trên RAM: {}", db.total_keys());

        // Kiểm tra đọc dữ liệu qua 1 lần Disk Seek
        assert_eq!(
            db.get("user:101")?,
            Some("Alice Nguyễn - Hà Nội (đã cập nhật)".to_string())
        );
        assert_eq!(db.get("user:102")?, None);
        assert_eq!(
            db.get("user:103")?,
            Some("Charlie - TP Hồ Chí Minh".to_string())
        );
        println!("    => Các thao tác CRUD ban đầu hoạt động hoàn hảo!");
    } // db đóng tệp an toàn tại đây

    // GIAI ĐOẠN 2: Kiểm thử tính năng phục hồi sau sự cố (Crash Recovery)
    println!("\n[2] Kiểm tra phục hồi dữ liệu khi khởi động lại ứng dụng:");
    {
        let mut db_recovered = MiniBitcask::open(&db_path)?;
        println!("    - Đã mở lại tệp '{}'", db_path.display());
        println!("    - Kiểm tra dữ liệu sau phục hồi:");
        println!("      + 'user:101' = {:?}", db_recovered.get("user:101")?);
        println!("      + 'user:102' = {:?}", db_recovered.get("user:102")?);
        println!("      + 'user:103' = {:?}", db_recovered.get("user:103")?);

        assert_eq!(
            db_recovered.get("user:101")?,
            Some("Alice Nguyễn - Hà Nội (đã cập nhật)".to_string())
        );
        assert_eq!(db_recovered.get("user:102")?, None);
        assert_eq!(
            db_recovered.get("user:103")?,
            Some("Charlie - TP Hồ Chí Minh".to_string())
        );
        assert_eq!(db_recovered.total_keys(), 2);
        println!("    => Khôi phục chỉ mục KeyDir trên RAM từ đĩa thành công 100%!");

        // GIAI ĐOẠN 3: Kiểm thử tiến trình nén gộp dọn rác (Compaction & Merge)
        println!("\n[3] Thực thi tiến trình nén gộp dọn rác Compaction:");
        let size_before = db_recovered.file_size();
        db_recovered.compact()?;
        let size_after = db_recovered.file_size();

        println!("    - Dung lượng tệp TRƯỚC nén gộp: {} bytes", size_before);
        println!("    - Dung lượng tệp SAU nén gộp   : {} bytes", size_after);
        assert!(size_after < size_before);

        // Kiểm tra dữ liệu sau nén gộp vẫn còn nguyên vẹn
        assert_eq!(
            db_recovered.get("user:101")?,
            Some("Alice Nguyễn - Hà Nội (đã cập nhật)".to_string())
        );
        assert_eq!(
            db_recovered.get("user:103")?,
            Some("Charlie - TP Hồ Chí Minh".to_string())
        );
        println!("    => Tiến trình Compaction đã dọn sạch toàn bộ rác thừa trên đĩa!");
    }

    // Dọn dẹp tệp thử nghiệm
    let _ = std::fs::remove_file(&db_path);

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
| **E0502** | `cannot borrow '*self' as mutable because it is also borrowed as immutable` | Bạn giữ tham chiếu `entry` lấy từ `self.keydir.get(key)` rồi gọi một **phương thức** nhận `&mut self` (ví dụ `self.read_at(...)`), sau đó còn dùng lại `entry`. Lưu ý: gọi thẳng `self.file.seek(...)` thì *không* lỗi — trình biên dịch hiểu `self.keydir` và `self.file` là hai trường tách biệt; chỉ lời gọi phương thức `&mut self` mới mượn khả biến *toàn bộ* struct. | Sử dụng `.cloned()` để sao chép cấu trúc nhẹ `KeyDirEntry` ra biến độc lập trên Stack trước khi thao tác với tệp tin. |
| **E0599** | `no method named 'seek' found for struct 'File'` | Bạn sử dụng phương thức `.seek()` để nhảy tọa độ byte trên đĩa nhưng quên đưa trait `Seek` vào phạm vi hoạt động. | Thêm dòng khai báo: `use std::io::Seek;` ở đầu tệp mã nguồn. |
| **E0382** | `borrow of moved value: 'compact_path'` | Bạn truyền `compact_path` (kiểu sở hữu `String`/`PathBuf`) vào hàm `rename()` khiến nó bị di chuyển, sau đó lại dùng lại nó ở dòng lệnh kế tiếp. | Truyền mượn tham chiếu `&compact_path` vào hàm `std::fs::rename`. |
| **E0061** | `this method takes 1 argument but 0 arguments were supplied` | Gọi `SystemTime::now().duration_since()` mà quên truyền mốc so sánh `UNIX_EPOCH`. | Dùng chuẩn: `SystemTime::now().duration_since(UNIX_EPOCH).unwrap().as_nanos()`. |

### Ví dụ phân tích lỗi `E0502` khi vừa tra cứu KeyDir vừa đọc File:

```rust
use std::fs::File;
use std::collections::HashMap;

struct DemoStore {
    file: File,
    index: HashMap<String, (u64, usize)>, // khoá -> (offset, độ dài)
}

impl DemoStore {
    /// Đọc `len` byte tại `offset` — nhận &mut self vì phải seek trên tệp
    fn read_at(&mut self, offset: u64, len: usize) -> Vec<u8> {
        let _ = (&mut self.file, offset);
        vec![0; len]
    }

    // Đoạn mã lỗi minh họa E0502: giữ tham chiếu vào self.index qua một lời gọi &mut self
    fn read_broken(&mut self, key: &str) {
        // if let Some(entry) = self.index.get(key) {         // Mượn bất biến *self (qua self.index)
        //     let data = self.read_at(entry.0, entry.1);     // LỖI E0502: mượn khả biến *self!
        //     println!("{:?} (offset {})", data, entry.0);   // ...trong khi `entry` còn được dùng
        // }
        let _ = key;
    }

    // Cách sửa chữa đúng chuẩn: Sao chép (copy) giá trị nhỏ ra trước
    fn read_correct(&mut self, key: &str) {
        let entry = self.index.get(key).copied(); // entry là biến độc lập trên Stack
        if let Some((offset, len)) = entry {
            let data = self.read_at(offset, len);
            println!("Đã đọc {} byte tại tọa độ {}", data.len(), offset);
        }
    }
}
```

---

## Kiểm thử tự động (Automated Tests)

Một động cơ lưu trữ chỉ đáng tin khi nó được kiểm thử ở những chỗ *khó chịu* nhất: mở lại sau khi đóng, nén gộp rồi ghi tiếp, và — quan trọng nhất — tệp bị **cắt cụt giữa một bản ghi** như khi mất điện lúc đang ghi. Test cuối cùng giả lập đúng tình huống đó bằng `File::set_len`, và khẳng định động cơ vẫn mở được, bỏ bản ghi rách, và bản ghi mới không bị nối vào sau rác.

```rust
#[cfg(test)]
mod tests {
    use super::*;

    fn temp_db(name: &str) -> PathBuf {
        let p = std::env::temp_dir().join(format!("ch36_test_{name}.db"));
        let _ = std::fs::remove_file(&p);
        p
    }

    #[test]
    fn crud_survives_reopen() -> io::Result<()> {
        let path = temp_db("crud");
        {
            let mut db = MiniBitcask::open(&path)?;
            db.set("a", "1")?;
            db.set("b", "2")?;
            db.set("a", "3")?;
            assert!(db.delete("b")?);
            assert!(!db.delete("missing")?);
        }
        let mut db = MiniBitcask::open(&path)?;
        assert_eq!(db.get("a")?, Some("3".into()));
        assert_eq!(db.get("b")?, None);
        assert_eq!(db.total_keys(), 1);
        std::fs::remove_file(&path)
    }

    #[test]
    fn compaction_keeps_live_data_and_cleans_up() -> io::Result<()> {
        let path = temp_db("compact");
        let mut db = MiniBitcask::open(&path)?;
        for i in 0..50 {
            db.set("hot", &format!("v{i}"))?;
        }
        db.set("cold", "x")?;
        db.set("gone", "y")?;
        db.delete("gone")?;
        let before = db.file_size();
        db.compact()?;
        assert!(db.file_size() < before);
        assert_eq!(db.get("hot")?, Some("v49".into()));
        assert_eq!(db.get("cold")?, Some("x".into()));
        assert_eq!(db.get("gone")?, None);
        // Không còn tệp tạm .compact sót lại
        let mut tmp = path.clone().into_os_string();
        tmp.push(".compact");
        assert!(!PathBuf::from(tmp).exists());
        // Ghi tiếp sau khi nén rồi mở lại: dữ liệu vẫn đúng
        db.set("after", "z")?;
        drop(db);
        let mut db = MiniBitcask::open(&path)?;
        assert_eq!(db.get("after")?, Some("z".into()));
        assert_eq!(db.get("hot")?, Some("v49".into()));
        assert_eq!(db.file_size(), std::fs::metadata(&path)?.len());
        std::fs::remove_file(&path)
    }

    #[test]
    fn torn_tail_record_is_dropped_on_recovery() -> io::Result<()> {
        // Lỗi cũ: bản ghi cuối bị cắt cụt làm open() thất bại (UnexpectedEof khi đọc khoá),
        // hoặc nạp vào KeyDir một mục trỏ ra ngoài cuối tệp.
        let path = temp_db("torn");
        {
            let mut db = MiniBitcask::open(&path)?;
            db.set("ok", "fine")?;
            db.set("torn", "this value will be cut")?;
        }
        let full = std::fs::metadata(&path)?.len();
        let f = OpenOptions::new().write(true).open(&path)?;
        f.set_len(full - 5)?; // giả lập sập nguồn giữa lúc ghi bản ghi thứ hai
        drop(f);

        let mut db = MiniBitcask::open(&path)?;
        assert_eq!(db.get("ok")?, Some("fine".into()));
        assert_eq!(db.get("torn")?, None);
        // Đuôi rách đã bị cắt, bản ghi mới nằm ngay sau bản ghi hợp lệ cuối cùng
        db.set("next", "clean")?;
        drop(db);
        let mut db = MiniBitcask::open(&path)?;
        assert_eq!(db.get("next")?, Some("clean".into()));
        assert_eq!(db.total_keys(), 2);
        std::fs::remove_file(&path)
    }
}
```

## Tóm tắt chương & Bài tập rèn luyện (Summary & Exercises)

### 4 Điểm cốt lõi cần ghi nhớ:
1. **Thiết kế lai hoàn hảo**: Bitcask kết hợp tinh hoa của Bảng băm trên RAM (`KeyDir`) cho tốc độ tra cứu $O(1)$ và Tệp ghi nối đuôi (Append-only) trên đĩa cho tốc độ ghi tối đa.
2. **Đúng 1 lần đọc đĩa (Single Disk Seek)**: Nhờ biết chính xác tọa độ byte (`offset`) và độ dài (`value_size`) từ RAM, thao tác đọc dữ liệu bỏ qua mọi tầng trung gian, nhảy thẳng tới vị trí đĩa cần đọc.
3. **Cơ chế Tombstone**: Thay vì tìm xóa tại chỗ trên đĩa (gây phân mảnh và chậm chạp), Bitcask ghi một bản ghi đánh dấu xóa (Tombstone) vào cuối tệp và xóa khỏi RAM.
4. **Nén gộp Compaction**: Tiến trình dọn dẹp định kỳ đọc lại các khóa còn sống và ghi sang tệp mới, giữ cho cơ sở dữ liệu luôn nhỏ gọn và loại bỏ hoàn toàn các phiên bản dữ liệu cũ. Thứ tự an toàn khi thay tệp: *ghi → `sync_all` tệp mới → `rename` → fsync thư mục*; và mỗi lần ghi bản ghi phải `sync_data()` (không phải `flush()`) trước khi coi là thành công.

### Bài tập rèn luyện tự giải:
1. **Bài tập 1 (Bổ sung mã kiểm tra toàn vẹn CRC32)**:  
   Mở rộng phần Header của bản ghi trong `MiniBitcask` thêm 4 bytes chứa mã kiểm tra toàn vẹn CRC32 (`crc32fast::Hasher` hoặc tự cài đặt thuật toán kiểm tra tổng kiểm tra checksum đơn giản). Khi đọc lại tệp trong hàm `rebuild_keydir`, tính toán lại mã CRC32 của bản ghi, nếu không khớp thì dừng lại và bỏ qua bản ghi bị lỗi.
2. **Bài tập 2 (Tạo tệp Hint File tối ưu hóa)**:  
   Sau khi tiến trình `compact()` hoàn tất, hãy cho ghi thêm một tệp gợi ý `data.db.hint` chỉ chứa các cặp `(key, KeyDirEntry)`. Khi hệ thống khởi động lại, thay vì phải quét toàn bộ tệp dữ liệu lớn, hệ thống chỉ cần đọc tệp Hint File nhỏ bé để khôi phục RAM trong vài mili-giây.
3. **Bài tập 3 (Giới hạn của Bitcask)**:  
   Điểm yếu lớn nhất của mô hình Bitcask là gì? Nếu cơ sở dữ liệu có 1 tỷ khóa khác nhau thì thanh RAM có thể chứa nổi `KeyDir` không? Trong trường hợp đó, người ta sẽ chuyển sang sử dụng mô hình nào (B+ Tree hay LSM-Tree)?

---

### Gợi ý & Lời giải

<details>
<summary><b>Bài tập 1 — Gợi ý</b></summary>

CRC32 quét từng byte, cập nhật một thanh ghi 32-bit qua bảng tra 256 mục. Chèn 4 byte CRC vào tiêu đề bản ghi; khi đọc lại thì tính lại và so — lệch là bản ghi hỏng.
</details>

<details>
<summary><b>Bài tập 1 — Lời giải</b></summary>

```rust
/// CRC32 (đa thức IEEE 0xEDB88320) — cùng thuật toán `crc32fast` dùng,
/// nhưng tự cài để thấy rõ cơ chế. Kiểm toàn vẹn: dữ liệu lệch 1 bit -> CRC đổi.
fn crc32(data: &[u8]) -> u32 {
    let mut crc: u32 = 0xFFFF_FFFF;
    for &byte in data {
        crc ^= byte as u32;
        // 8 vòng: mỗi bit hoặc dịch phải, hoặc dịch rồi XOR đa thức.
        for _ in 0..8 {
            let mask = (crc & 1).wrapping_neg(); // 0xFFFFFFFF nếu bit thấp =1, else 0
            crc = (crc >> 1) ^ (0xEDB8_8320 & mask);
        }
    }
    !crc // đảo bit cuối theo chuẩn
}

#[test]
fn crc32_matches_standard_vector() {
    // Vector kiểm nghiệm kinh điển: "123456789" -> 0xCBF43926
    assert_eq!(crc32(b"123456789"), 0xCBF4_3926);
    // Đổi đúng 1 byte -> CRC khác hẳn: đó là điều làm nó phát hiện hỏng.
    assert_ne!(crc32(b"123456780"), crc32(b"123456789"));
}
```

**Cách gắn vào MiniBitcask:** tiêu đề bản ghi hiện là 17 byte (timestamp 8 + is_deleted 1 + k_len 4 + v_len 4). Thêm **4 byte CRC32** đặt ngay đầu, tính trên *toàn bộ phần còn lại của bản ghi* (17 byte tiêu đề cũ + key + value) lúc `set`:
```text
[ CRC32 4B ][ timestamp 8B ][ is_deleted 1B ][ k_len 4B ][ v_len 4B ][ key ][ value ]
  \________ tính CRC trên vùng này ________________________________________________/
```
Trong `rebuild_keydir`, sau khi đọc một bản ghi, **tính lại CRC** trên vùng dữ liệu và so với 4 byte CRC đã lưu. Khớp thì nạp vào KeyDir; **lệch thì bỏ qua bản ghi hỏng** (và dừng nếu muốn chặt chẽ) — vì đĩa có thể hỏng bit, hoặc ghi dở khi mất điện giữa chừng.

Đây là lý do mọi định dạng lưu trữ nghiêm túc (SSTable, WAL, gói TCP) đều mang checksum: **đĩa, mạng và thời gian đều không tin được** — CRC là tấm lưới bắt lỗi âm thầm trước khi nó lan thành dữ liệu sai.
</details>

<details>
<summary><b>Bài tập 2 — Gợi ý</b></summary>

Tệp Hint chỉ chứa `(key, offset, size, timestamp)` — không chứa value. Nhỏ hơn tệp dữ liệu nhiều lần, nên nạp lại RAM nhanh hơn hẳn.
</details>

<details>
<summary><b>Bài tập 2 — Lời giải</b></summary>

**Ý tưởng:** khôi phục KeyDir lúc khởi động hiện phải **quét cả tệp dữ liệu lớn** (đọc qua từng value chỉ để lấy offset). Nhưng KeyDir chỉ cần `(key -> offset, size, timestamp)` — *không* cần value. Tệp Hint lưu đúng phần đó.

```rust
use std::io::Write;
/// Sau compact(), ghi tệp hint: mỗi dòng là metadata một khóa, KHÔNG có value.
/// Định dạng đơn giản: key_len(4) | key | offset(8) | size(4) | ts(8)
fn write_hint_file(path: &str, keydir: &std::collections::HashMap<String, (u64, u32, u64)>)
    -> std::io::Result<()>
{
    let mut f = std::fs::File::create(path)?;
    for (key, (offset, size, ts)) in keydir {
        let kb = key.as_bytes();
        f.write_all(&(kb.len() as u32).to_le_bytes())?;
        f.write_all(kb)?;
        f.write_all(&offset.to_le_bytes())?;
        f.write_all(&size.to_le_bytes())?;
        f.write_all(&ts.to_le_bytes())?;
    }
    f.sync_all()?; // ép xuống đĩa: hint chỉ hữu ích nếu nó sống sót qua mất điện
    Ok(())
}
```

**Vì sao nhanh hơn nhiều bậc:** giả sử mỗi value trung bình 1 KB còn metadata mỗi khóa ~30 byte. Với 1 triệu khóa, tệp dữ liệu ~1 GB, tệp hint chỉ ~30 MB — **nhỏ hơn ~33 lần**. Khởi động lại: thay vì đọc 1 GB, hệ thống đọc 30 MB và dựng lại KeyDir trong vài mili-giây.

Điểm tinh tế cần đúng: khi khởi động phải **ưu tiên đọc hint nếu có**, và chỉ quét tệp dữ liệu đầy đủ khi hint thiếu hoặc cũ hơn tệp dữ liệu (ví dụ đã có ghi mới sau lần compact cuối). Đây chính là cách Bitcask thật (của Riak) rút ngắn thời gian phục hồi — một ví dụ đẹp của việc *tách metadata khỏi dữ liệu* để tăng tốc.
</details>

<details>
<summary><b>Bài tập 3 — Gợi ý</b></summary>

Điểm yếu nằm ở chỗ **KeyDir sống hoàn toàn trên RAM** — mọi khóa đều phải có một mục trong RAM, bất kể dữ liệu lớn đến đâu.
</details>

<details>
<summary><b>Bài tập 3 — Lời giải</b></summary>

**Điểm yếu lớn nhất của Bitcask: toàn bộ tập khóa (KeyDir) phải nằm vừa trong RAM.**

Bitcask đánh đổi để đạt tốc độ: mỗi thao tác đọc chỉ tốn *đúng một* lần chạm đĩa, vì RAM giữ sẵn ánh xạ `key -> vị trí trên đĩa`. Cái giá là **RAM phải chứa được mọi khóa**.

**Với 1 tỷ khóa thì sao?** Ước lượng thô mỗi mục KeyDir:
```text
key trung bình      ~16 byte
offset (u64)          8 byte
size (u32/usize)      8 byte
timestamp (u64)       8 byte
chi phí HashMap      ~20 byte (con trỏ, băm, ô trống dự phòng)
-------------------------------
mỗi khóa           ~60 byte
× 1 tỷ khóa        = ~60 GB RAM  chỉ riêng cho MỤC LỤC
```
**60 GB RAM chỉ để chứa mục lục** — vượt xa RAM của gần như mọi máy chủ thông thường, và đó là còn *chưa* tính chỗ cho chính dữ liệu. Bitcask sụp đổ ở quy mô này.

**Chuyển sang mô hình nào:** khi tập khóa không còn vừa RAM, người ta dùng cấu trúc **giữ mục lục *trên đĩa*, chỉ nạp phần nóng vào RAM**:

| Mô hình | Ý tưởng | Ai dùng |
|---|---|---|
| **B+ Tree** | Cây cân bằng trên đĩa; mỗi tra cứu vài lần đọc đĩa (log n), nhưng mục lục *không* cần vừa RAM | Hầu hết CSDL quan hệ (PostgreSQL, MySQL/InnoDB) |
| **LSM-Tree** | Ghi vào RAM rồi xả xuống các SSTable đã sắp xếp trên đĩa; tra cứu qua nhiều tầng + bộ lọc Bloom | Cassandra, RocksDB, LevelDB |

Chọn giữa hai: **B+ Tree** khi đọc ngẫu nhiên nhiều (ưu tiên đọc nhanh, ổn định); **LSM-Tree** khi ghi rất nhiều (biến ghi ngẫu nhiên thành ghi tuần tự — xem lại Chương 34). Cả hai đều bỏ ràng buộc "mọi khóa phải vừa RAM" của Bitcask, đổi lấy mỗi lần đọc tốn hơn một lần chạm đĩa.
</details>
