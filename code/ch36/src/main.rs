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
