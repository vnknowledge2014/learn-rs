#![allow(dead_code, unused_variables, unused_imports)]
use std::collections::BTreeMap;
use std::fs::{File, OpenOptions};
use std::io::{self, BufRead, BufReader, Write};
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
        // Số byte hợp lệ của WAL (kết thúc ở dấu '\n' cuối cùng)
        let mut valid_len: u64 = 0;

        // 1. TIẾN TRÌNH PHỤC HỒI SAU SỰ CỐ (Crash Recovery):
        // Nếu tệp WAL đã có sẵn từ phiên chạy trước, đọc lại toàn bộ nhật ký
        if Path::new(wal_path).exists() {
            let mut reader = BufReader::new(File::open(wal_path)?);
            let mut line = Vec::new();
            loop {
                line.clear();
                let n = reader.read_until(b'\n', &mut line)?;
                // Dòng cuối không có '\n' = dòng bị RÁCH do sập nguồn giữa lúc ghi:
                // bỏ qua, không được áp dụng một nửa bản ghi.
                if n == 0 || line.last() != Some(&b'\n') {
                    break;
                }
                valid_len += n as u64;
                let text = String::from_utf8_lossy(&line[..n - 1]);
                if let Some((op, rest)) = text.split_once(':') {
                    if op == "SET" {
                        if let Some((k, v)) = rest.split_once('=') {
                            memtable.insert(k.to_string(), v.to_string());
                        }
                    } else if op == "DEL" {
                        memtable.remove(rest);
                    }
                }
            }
            println!(
                "    [RECOVERY]: Đã phục hồi thành công {} khóa từ tệp WAL!",
                memtable.len()
            );
        }

        // 2. Mở tệp WAL ở chế độ ghi nối đuôi (Append-only).
        //    (append(true) đã bao hàm quyền ghi, không cần write(true))
        let wal_file = OpenOptions::new()
            .create(true)
            .append(true)
            .open(wal_path)?;
        // Cắt bỏ phần đuôi bị rách (nếu có), nếu không bản ghi mới sẽ dính
        // vào sau mảnh rác và chính nó cũng bị hỏng ở lần phục hồi sau.
        if wal_file.metadata()?.len() > valid_len {
            wal_file.set_len(valid_len)?;
            wal_file.sync_all()?;
        }

        Ok(Self {
            memtable,
            wal_file,
            wal_path: wal_path.to_string(),
        })
    }

    /// Ghi một dòng vào WAL và ÉP xuống đĩa trước khi trả về.
    fn append_to_wal(&mut self, log_line: &str) -> io::Result<()> {
        self.wal_file.write_all(log_line.as_bytes())?;
        // write_all chỉ đưa dữ liệu vào page cache của hệ điều hành (vẫn là RAM!).
        // `File::flush()` không làm gì cả; sync_data() (fdatasync) mới thực sự
        // chờ dữ liệu nằm trên thiết bị lưu trữ.
        self.wal_file.sync_data()
    }

    /// Thao tác Ghi: BẮT BUỘC ghi WAL (và đồng bộ xuống đĩa) trước, sau đó mới cập nhật MemTable.
    /// Giới hạn của định dạng văn bản đơn giản: khoá không được chứa '=' hay '\n',
    /// giá trị không được chứa '\n'.
    pub fn set(&mut self, key: &str, value: &str) -> io::Result<()> {
        // BƯỚC 1: Ghi tuần tự vào WAL (Write-Ahead)
        self.append_to_wal(&format!("SET:{}={}\n", key, value))?;

        // BƯỚC 2: Cập nhật MemTable trên RAM
        self.memtable.insert(key.to_string(), value.to_string());
        Ok(())
    }

    /// Thao tác Xóa: Ghi nhận Tombstone vào WAL và xóa khỏi MemTable
    pub fn delete(&mut self, key: &str) -> io::Result<bool> {
        if self.memtable.contains_key(key) {
            // Ghi nhận bia mộ (Tombstone) vào WAL
            self.append_to_wal(&format!("DEL:{}\n", key))?;

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

    let wal_path_buf = std::env::temp_dir().join("ch34_mini_engine.wal");
    let wal_path = wal_path_buf.to_str().expect("đường dẫn tạm phải là UTF-8");

    // Đảm bảo dọn dẹp tệp cũ trước khi bắt đầu thử nghiệm
    let _ = std::fs::remove_file(wal_path);

    // GIAI ĐOẠN 1: Khởi động động cơ và ghi chép dữ liệu
    println!("[1] Khởi động động cơ MiniLsmEngine lần đầu:");
    {
        let mut engine = MiniLsmEngine::open(wal_path)?;

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

        println!(
            "    - Tổng số khóa hợp lệ trên RAM: {}",
            engine.total_keys()
        );
        assert_eq!(engine.get("user:1"), Some(&"Alice Nguyen".to_string()));
        assert_eq!(engine.get("user:2"), None);
        assert_eq!(engine.get("user:3"), Some(&"Charlie".to_string()));

        println!("\n    => ĐỘT NGỘT SẬP NGUỒN! (Cỗ máy tính bị ngắt điện)");
        // engine bị drop tại đây, tương đương tiến trình bị tắt đột ngột
    }

    // GIAI ĐOẠN 2: Khởi động lại sau sự cố và kiểm tra tính năng phục hồi
    println!("\n[2] Bật lại máy chủ và khởi động lại MiniLsmEngine:");
    {
        let recovered_engine = MiniLsmEngine::open(wal_path)?;

        println!("    - Kiểm tra dữ liệu sau phục hồi:");
        println!("      + 'user:1' = {:?}", recovered_engine.get("user:1"));
        println!("      + 'user:2' = {:?}", recovered_engine.get("user:2"));
        println!("      + 'user:3' = {:?}", recovered_engine.get("user:3"));

        // Xác nhận dữ liệu được phục hồi chuẩn xác 100%
        assert_eq!(
            recovered_engine.get("user:1"),
            Some(&"Alice Nguyen".to_string())
        );
        assert_eq!(recovered_engine.get("user:2"), None);
        assert_eq!(recovered_engine.get("user:3"), Some(&"Charlie".to_string()));
        assert_eq!(recovered_engine.total_keys(), 2);

        println!("    => Toàn bộ trạng thái dữ liệu đã được phục hồi hoàn hảo nhờ WAL!");
    }

    // Dọn dẹp tệp thử nghiệm
    let _ = std::fs::remove_file(wal_path);

    println!("============================================================");
    println!("               HOÀN TẤT THỰC NGHIỆM CHƯƠNG 34               ");
    println!("============================================================");
    Ok(())
}

#[cfg(test)]
mod tests {
    use super::*;

    fn temp_wal(name: &str) -> String {
        let p = std::env::temp_dir().join(format!("ch34_test_{name}.wal"));
        let _ = std::fs::remove_file(&p);
        p.to_str().unwrap().to_string()
    }

    #[test]
    fn recovery_replays_sets_and_tombstones() -> io::Result<()> {
        let path = temp_wal("replay");
        {
            let mut e = MiniLsmEngine::open(&path)?;
            e.set("a", "10")?;
            e.set("b", "20")?;
            e.set("a", "30")?;
            assert!(e.delete("b")?);
            e.set("c", "40")?;
        }
        let e = MiniLsmEngine::open(&path)?;
        assert_eq!(e.get("a"), Some(&"30".to_string()));
        assert_eq!(e.get("b"), None);
        assert_eq!(e.get("c"), Some(&"40".to_string()));
        assert_eq!(e.total_keys(), 2);
        std::fs::remove_file(&path)
    }

    #[test]
    fn torn_last_line_is_ignored_and_truncated() -> io::Result<()> {
        let path = temp_wal("torn");
        // Giả lập sập nguồn giữa lúc ghi: dòng cuối thiếu '\n'
        std::fs::write(&path, "SET:a=1\nSET:b=2\nSET:c=12")?;
        {
            let mut e = MiniLsmEngine::open(&path)?;
            assert_eq!(e.get("c"), None, "không được áp dụng nửa bản ghi");
            assert_eq!(e.total_keys(), 2);
            // Bản ghi mới phải nằm trên một dòng sạch, không dính vào mảnh rác
            e.set("d", "4")?;
        }
        assert_eq!(
            std::fs::read_to_string(&path)?,
            "SET:a=1\nSET:b=2\nSET:d=4\n"
        );
        let e = MiniLsmEngine::open(&path)?;
        assert_eq!(e.get("d"), Some(&"4".to_string()));
        std::fs::remove_file(&path)
    }

    #[test]
    fn deleting_missing_key_writes_nothing() -> io::Result<()> {
        let path = temp_wal("missing");
        let mut e = MiniLsmEngine::open(&path)?;
        assert!(!e.delete("ghost")?);
        assert_eq!(std::fs::metadata(&path)?.len(), 0);
        std::fs::remove_file(&path)
    }
}
