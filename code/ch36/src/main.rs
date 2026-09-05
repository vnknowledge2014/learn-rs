#![allow(dead_code, unused_variables, unused_imports)]
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
