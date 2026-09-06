#![allow(dead_code, unused_variables, unused_imports)]
use std::collections::BTreeMap;
use std::fs::{File, OpenOptions};
use std::io::{self, BufRead, BufReader, Seek, SeekFrom, Write};
use std::path::Path;

/// ĐỘNG CƠ MINI LSM-TREE KẾT HỢP GHI NHẬT KÝ WAL
pub struct MiniLsmEngine {
    memtable: BTreeMap<String, String>, // Bộ nhớ đệm RAM tự sắp xếp
    wal_file: File,                     // Tệp nhật ký an toàn trên đĩa
    wal_path: String,
}

impl MiniLsmEngine {
    /// Khởi động động cơ: Mở tệp WAL và tự động phục hồi nếu tệp đã tồn tại
    pub fn open(wal_path: &str) -> io::Result<Self> {
        let mut memtable = BTreeMap::new();

        // 1. TIẾN TRÌNH PHỤC HỒI SAU SỰ CỐ (Crash Recovery):
        // Nếu tệp WAL đã có sẵn từ phiên chạy trước, đọc lại toàn bộ nhật ký
        if Path::new(wal_path).exists() {
            let file_doc = File::open(wal_path)?;
            let reader = BufReader::new(file_doc);
            for line_res in reader.lines() {
                let line = line_res?;
                if let Some((order, phan_con_lai)) = line.split_once(':') {
                    if order == "SET" {
                        if let Some((k, v)) = phan_con_lai.split_once('=') {
                            memtable.insert(k.to_string(), v.to_string());
                        }
                    } else if order == "DEL" {
                        memtable.remove(phan_con_lai);
                    }
                }
            }
            println!("    [RECOVERY]: Đã phục hồi thành công {} khóa từ tệp WAL!", memtable.len());
        }

        // 2. Mở tệp WAL ở chế độ ghi chèn (Append-only)
        let wal_file = OpenOptions::new()
            .create(true)
            .write(true)
            .append(true)
            .open(wal_path)?;

        Ok(Self {
            memtable,
            wal_file,
            wal_path: wal_path.to_string(),
        })
    }

    /// Thao tác Ghi: BẮT BUỘC ghi WAL trước, sau đó mới cập nhật MemTable
    pub fn set(&mut self, key: &str, value: &str) -> io::Result<()> {
        // BƯỚC 1: Ghi tuần tự vào WAL (Write-Ahead)
        let close_log = format!("SET:{}={}\n", key, value);
        self.wal_file.write_all(close_log.as_bytes())?;
        // Ép dữ liệu từ bộ đệm phần mềm xuống phần cứng đĩa
        self.wal_file.flush()?;

        // BƯỚC 2: Cập nhật MemTable trên RAM
        self.memtable.insert(key.to_string(), value.to_string());
        Ok(())
    }

    /// Thao tác Xóa: Ghi nhận Tombstone vào WAL và xóa khỏi MemTable
    pub fn delete(&mut self, key: &str) -> io::Result<bool> {
        if self.memtable.contains_key(key) {
            // Ghi nhận bia mộ (Tombstone) vào WAL
            let close_log = format!("DEL:{}\n", key);
            self.wal_file.write_all(close_log.as_bytes())?;
            self.wal_file.flush()?;

            self.memtable.remove(key);
            Ok(true)
        } else {
            Ok(false)
        }
    }

    /// Thao tác Đọc: Đọc siêu tốc từ MemTable trên RAM - O(log N)
    pub fn get(&self, key: &str) -> Option<&String> {
        self.memtable.get(key)
    }

    pub fn total_keys(&self) -> usize {
        self.memtable.len()
    }
}

fn main() -> io::Result<()> {
    println!("============================================================");
    println!("   NHẬT KÝ GHI TRƯỚC WAL & ĐỘNG CƠ LƯU TRỮ HIỆN ĐẠI LSM-TREE ");
    println!("============================================================");

    let duong_dan_wal = "mini_engine.wal";

    // Đảm bảo dọn dẹp tệp cũ trước khi bắt đầu thử nghiệm
    let _ = std::fs::remove_file(duong_dan_wal);

    // GIAI ĐOẠN 1: Khởi động động cơ và ghi chép dữ liệu
    println!("[1] Khởi động động cơ MiniLsmEngine lần đầu:");
    {
        let mut engine = MiniLsmEngine::open(duong_dan_wal)?;
        
        println!("    - Ghi khóa 'user:1' -> 'Alice'");
        engine.set("user:1", "Alice")?;
        
        println!("    - Ghi khóa 'user:2' -> 'Bob'");
        engine.set("user:2", "Bob")?;
        
        println!("    - Ghi đè khóa 'user:1' -> 'Alice Nguyen'");
        engine.set("user:1", "Alice Nguyen")?;
        
        println!("    - Ghi khóa 'user:3' -> 'Charlie'");
        engine.set("user:3", "Charlie")?;
        
        println!("    - Xóa khóa 'user:2' (Ghi Tombstone vào WAL)");
        engine.delete("user:2")?;

        println!("    - Tổng số khóa hợp lệ trên RAM: {}", engine.total_keys());
        assert_eq!(engine.get("user:1"), Some(&"Alice Nguyen".to_string()));
        assert_eq!(engine.get("user:2"), None);
        assert_eq!(engine.get("user:3"), Some(&"Charlie".to_string()));

        println!("\n    => ĐỘT NGỘT SẬP NGUỒN! (Cỗ máy tính bị ngắt điện)");
        // engine bị drop tại đây, tương đương tiến trình bị tắt đột ngột
    }

    // GIAI ĐOẠN 2: Khởi động lại sau sự cố và kiểm tra tính năng phục hồi
    println!("\n[2] Bật lại máy chủ và khởi động lại MiniLsmEngine:");
    {
        let recovered_engine = MiniLsmEngine::open(duong_dan_wal)?;
        
        println!("    - Kiểm tra dữ liệu sau phục hồi:");
        println!("      + 'user:1' = {:?}", recovered_engine.get("user:1"));
        println!("      + 'user:2' = {:?}", recovered_engine.get("user:2"));
        println!("      + 'user:3' = {:?}", recovered_engine.get("user:3"));

        // Xác nhận dữ liệu được phục hồi chuẩn xác 100%
        assert_eq!(recovered_engine.get("user:1"), Some(&"Alice Nguyen".to_string()));
        assert_eq!(recovered_engine.get("user:2"), None);
        assert_eq!(recovered_engine.get("user:3"), Some(&"Charlie".to_string()));
        assert_eq!(recovered_engine.total_keys(), 2);
        
        println!("    => Toàn bộ trạng thái dữ liệu đã được phục hồi hoàn hảo nhờ WAL!");
    }

    // Dọn dẹp tệp thử nghiệm
    let _ = std::fs::remove_file(duong_dan_wal);

    println!("============================================================");
    println!("               HOÀN TẤT THỰC NGHIỆM CHƯƠNG 30               ");
    println!("============================================================");
    Ok(())
}
