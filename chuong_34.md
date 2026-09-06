# Chương 34: Nhật ký ghi trước WAL & Động cơ lưu trữ hiện đại LSM-Tree (Write-Ahead Logging & LSM-Tree Engine)

## Giới thiệu & Mục tiêu học tập

Trong các chương trước, bạn đã thấy cấu trúc B+ Tree hoạt động xuất sắc như thế nào trong việc đọc dữ liệu với độ trễ thấp. Tuy nhiên, khi đối mặt với các hệ thống hiện đại có khối lượng ghi dữ liệu khổng lồ (hàng trăm ngàn đến hàng triệu thao tác ghi mỗi giây, như hệ thống tin nhắn mạng xã hội, nhật ký máy chủ viễn thông, hay cảm biến IoT), kiến trúc B+ Tree bộc lộ một điểm yếu chí mạng: **Nó phải thực hiện ghi ngẫu nhiên (Random I/O) xuống các khối trang 4KB trên đĩa cứng**. Tệ hơn nữa, nếu cỗ máy tính bất ngờ bị rút phích cắm điện (sập nguồn - crash) ngay giữa lúc đang ghi dở dang một trang dữ liệu, trang đó sẽ bị biến thành dữ liệu rác không thể cứu vãn (hiện tượng Torn Page)!

Làm thế nào để các kỹ sư hệ thống vừa đạt được tốc độ ghi dữ liệu thần tốc, vừa đảm bảo dữ liệu không bao giờ bị mất mát dù máy chủ có nổ cầu chì?

Giải pháp kinh điển mang tính cách mạng gồm hai thành phần:
1. **Nhật ký ghi trước (Write-Ahead Logging - WAL)**: Một nguyên tắc bất di bất dịch: *"Luôn luôn ghi chép nối đuôi tuần tự hành động xuống đĩa trước khi dám sửa bất kỳ byte nào trên thanh RAM"*. Nhờ đó, việc phục hồi (crash recovery) sau tai nạn trở nên dễ dàng tuyệt đối.
2. **Động cơ cây sáp nhập có cấu trúc nhật ký (Log-Structured Merge-Tree - LSM-Tree)**: Kiến trúc đứng sau sự thành công của Google Bigtable, Apache Cassandra, RocksDB, và TiKV, biến 100% thao tác ghi thành ghi tuần tự (Sequential I/O) thông qua bộ đôi **MemTable** (trên RAM) và **SSTable** (trên đĩa).

Mục tiêu học tập của chương này:
- Nắm vững nguyên lý hoạt động và tầm quan trọng sống còn của **Nhật ký ghi trước (WAL - Write-Ahead Logging)** trong việc bảo đảm an toàn dữ liệu và phục hồi sau sự cố.
- Hiểu cấu trúc 4 tầng của động cơ **LSM-Tree**: `MemTable`, `WAL`, `SSTable` (Sorted String Table), và tiến trình nén gộp ngầm (`Compaction`).
- Giải thích vì sao việc biến ghi ngẫu nhiên thành ghi tuần tự giúp LSM-Tree đạt tốc độ ghi nhanh gấp 10 lần so với B+ Tree truyền thống.
- Nhận diện cơ chế xóa mềm thông qua "Bia mộ" (**Tombstone**) trong các tệp dữ liệu bất biến.
- Tự tay lập trình một động cơ Mini-LSM trong Rust có ghi WAL, cập nhật MemTable, và có khả năng phục hồi dữ liệu 100% sau khi giả lập sập nguồn hệ thống.

---

## Hình tượng hóa đời sống (Intuitive Everyday Analogy)

Hãy quan sát hai câu chuyện đời thực vô cùng gần gũi để hình dung cách vận hành của WAL và LSM-Tree:

```
┌──────────────────────────────────────────────────────────────────────────────────┐
│                   HÌNH TƯỢNG HÓA WAL VÀ ĐỘNG CƠ HIỆN ĐẠI LSM-TREE                │
├──────────────────────────────────────────────────────────────────────────────────┤
│                                                                                  │
│ [1. NGUYÊN LÝ WAL: CUỐN SỔ TAY GHI NỢ CÀI THẮT LƯNG BÁC BÁN PHỞ]                 │
│                                                                                  │
│ Khách vào ăn phở ồ ạt: "Cho bát tái nạm, ghi nợ nhé!"                            │
│        │                                                                         │
│        ▼                                                                         │
│ Bác rút cây bút chì ngoáy nhanh vào sổ cài thắt lưng (WAL):                      │
│        ┌────────────────────────────────────────────────────────┐                │
│        │ [08:00] Bàn 1 nợ 50k                                   │                │
│        │ [08:02] Bàn 4 nợ 70k  ◄── Ghi nối đuôi (Append-only)   │                │
│        │ [08:05] Bàn 2 nợ 45k      Cực nhanh, bút không rời giấy│                │
│        └────────────────────────────────────────────────────────┘                │
│        │                                                                         │
│        ▼                                                                         │
│ SẬP NGUỒN! Cả khu phố mất điện tối om!                                           │
│ -> Bác mở cuốn sổ tay ra: Toàn bộ lịch sử nợ nần vẫn còn nguyên, không mất 1 xu!│
│                                                                                  │
│ [2. LSM-TREE: GIẤY NHỚ DÁN BÀN VÀ THÙNG HỒ SƠ LƯU TRỮ TRONG KHO]                 │
│                                                                                  │
│ ┌───────────────────────────────────────┐                                        │
│ │ 1. MEMTABLE (Giấy nhớ dán trên bàn)   │ -> Viết và sắp xếp A-Z trên RAM        │
│ └───────────────────┬───────────────────┘                                        │
│                     │ Khi giấy nhớ đầy bàn (Flush)                               │
│                     ▼                                                            │
│ ┌───────────────────────────────────────┐                                        │
│ │ 2. SSTABLE (Thùng sắt lưu trữ ở kho)  │ -> Đã cất vào kho thì KHÔNG BAO GIỜ    │
│ │ [A-E: Thùng 1]  [F-M: Thùng 2]        │    mở ra sửa (Bất biến - Immutable)    │
│ └───────────────────┬───────────────────┘                                        │
│                     │ Định kỳ cuối tuần (Compaction)                             │
│                     ▼                                                            │
│ ┌───────────────────────────────────────┐                                        │
│ │ 3. COMPACTION (Dọn dẹp gộp kho)       │ -> Gộp nhiều thùng nhỏ thành thùng to, │
│ │                                       │    ném bỏ các giấy nợ đã được trả tiền │
│ └───────────────────────────────────────┘                                        │
└──────────────────────────────────────────────────────────────────────────────────┘
```

### 1. Cuốn sổ tay cài thắt lưng của bác bán phở (WAL)
- Vào 8 giờ sáng, khách hàng đổ xô vào quán phở. Khách hô: *"Cho bát tái chín, tí tính tiền sau nhé!"*.
- Bác bán phở không thể chạy ngay vào bàn làm việc, bật máy tính, mở phần mềm kế toán để gõ từng dòng (khách sẽ nổi giận vì chờ phở quá lâu).
- Bác làm một thao tác cực nhanh: Rút mẩu bút chì cài ở thắt lưng, ghi ngoáy vào cuốn sổ tay con: `[08:05 Bàn 3 nợ 50k]`.
- Cuốn sổ tay con này chính là **WAL (Write-Ahead Log)**:
  - Nó chỉ ghi nối tiếp vào cuối trang (Append-only), bút không rời giấy, tốc độ tức thời!
  - Giả sử quán phở bất ngờ bị cúp điện toàn phần (Crash), máy tính bị tắt phụt. Bác chủ quán không hề hoang mang: Chỉ cần mở cuốn sổ tay cài thắt lưng ra, bác có thể đọc lại từng dòng và **phục hồi (recover)** lại chính xác 100% doanh thu của quán!

### 2. Giấy nhớ và Thùng sắt lưu trữ (LSM-Tree)
- Khi có thông tin mới, bạn viết nhanh vào các tờ giấy nhớ dán trên mặt bàn làm việc (Bộ nhớ đệm **MemTable** trên RAM). Trên mặt bàn, bạn dễ dàng xếp các tờ giấy nhớ theo thứ tự chữ cái A-Z.
- Khi giấy nhớ dán kín mặt bàn: Bạn gom toàn bộ giấy nhớ lại, đóng thành một tập hồ sơ ngăn nắp rồi đem cất vào thùng sắt dưới tầng hầm (**SSTable** trên đĩa cứng).
- **Quy tắc vàng của SSTable**: Một khi thùng sắt đã đóng nắp cất vào kho, bạn **không bao giờ mở ra tẩy xóa hay sửa đổi** (Dữ liệu bất biến - Immutable). Nếu khách hàng muốn đổi số điện thoại, bạn viết một tờ giấy nhớ mới ghi đè lên ở trên bàn.
- **Tiến trình nén gộp (Compaction)**: Định kỳ vào cuối tháng, người thủ kho đem 5 thùng sắt nhỏ ra, gộp lại thành 1 thùng sắt lớn, đồng thời xé bỏ các tờ giấy nợ cũ đã được thanh toán hoặc đã bị hủy (Tombstone). Kho tài liệu lại trở nên gọn gàng tinh tươm!

---

## Khái niệm & Cơ chế kỹ thuật chuyên sâu (In-Depth Technical Mechanics)

### 1. Nguyên lý vàng của WAL: Độ bền vững trước khi biến đổi

Trong mọi hệ thống cơ sở dữ liệu quan hệ và phi quan hệ, quy tắc bất biến là:
$$\text{Ghi WAL xuống đĩa cứng} \longrightarrow \text{Ép đĩa (flush/fsync)} \longrightarrow \text{Mới được phép cập nhật RAM}$$

Cấu trúc của một bản ghi trong tệp WAL:
```
┌───────────────┬────────────────┬───────────────┬─────────────┬──────────────┐
│ Mã CRC32 (4B) │ Chiều dài (4B) │ Kiểu lệnh(1B) │ Khóa (Key)  │ Giá trị (Val)│
└───────────────┴────────────────┴───────────────┴─────────────┴──────────────┘
```
- **Mã kiểm tra toàn vẹn CRC32 (4 bytes)**: Ngăn chặn lỗi khi máy tính sập nguồn giữa lúc đang ghi dở một dòng nhật ký. Khi khởi động lại, nếu mã CRC32 không khớp, hệ thống biết ngay dòng nhật ký đó bị rách (corrupted) và an toàn cắt bỏ nó.
- **Hàm `fsync` / `flush`**: Hệ điều hành thường giữ dữ liệu trong bộ nhớ đệm (buffer cache) của kernel. Hàm `flush()` trong Rust ép buộc dữ liệu phải rời khỏi RAM và ghi thực sự vào các chip nhớ vật lý của đĩa SSD.

### 2. Giải phẫu kiến trúc 4 tầng của LSM-Tree

LSM-Tree phân tách rạch ròi quy trình xử lý dữ liệu theo thời gian:

1. **Tầng 1: MemTable (Memory Table)**:
   - Nằm trên RAM, duy trì dữ liệu luôn luôn được sắp xếp theo thứ tự khóa tăng dần.
   - Trong Rust, `std::collections::BTreeMap` là sự lựa chọn hoàn hảo nhất cho MemTable nhờ tính chất tự sắp xếp $O(\log N)$ và thân thiện với bộ nhớ đệm (buffer) của CPU.
2. **Tầng 2: Write-Ahead Log (WAL)**:
   - Nằm trên đĩa SSD, nhận các bản ghi tuần tự song song với MemTable để phòng ngừa rủi ro mất điện.
3. **Tầng 3: SSTable (Sorted String Table)**:
   - Khi MemTable đạt ngưỡng kích thước (ví dụ 64MB), nó bị đóng băng (Frozen MemTable) và một tiến trình chạy ngầm sẽ xả (Flush) toàn bộ dữ liệu xuống đĩa thành một tệp SSTable mới.
   - SSTable gồm 2 phần: Dữ liệu đã sắp xếp và **Chỉ mục thưa (Sparse Index)** giúp nhảy cóc tìm nhanh dữ liệu trên đĩa.
4. **Tầng 4: Tiến trình nén gộp (Compaction)**:
   - Sử dụng thuật toán sáp nhập nhiều danh sách (K-way Merge Sort) tương tự hàm merge của Merge Sort.
   - Đọc tuần tự các tệp SSTable cũ, loại bỏ các bản ghi bị ghi đè nhiều lần hoặc các bản ghi bị đánh dấu cờ xóa (**Tombstone - Bia mộ**), và sinh ra tệp SSTable tầng cao hơn hoàn toàn tinh gọn.

---

## Mã nguồn minh họa thực chiến (Idiomatic Runnable Rust Blueprint)

Dưới đây là một chương trình Rust hoàn chỉnh và độc lập, mô phỏng một động cơ lưu trữ Mini-LSM Engine với cơ chế ghi trước WAL tuần tự, cập nhật MemTable trên RAM, và quy trình phục hồi sau sự cố (Crash Recovery):

```rust
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

    let wal_path = "mini_engine.wal";

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
        let recovered_engine = MiniLsmEngine::open(wal_path)?;
        
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
    let _ = std::fs::remove_file(wal_path);

    println!("============================================================");
    println!("               HOÀN TẤT THỰC NGHIỆM CHƯƠNG 30               ");
    println!("============================================================");
    Ok(())
}
```

---

## Bảng tra cứu lỗi biên dịch & Cách khắc phục (Compiler Error Guide)

Dưới đây là các lỗi biên dịch thường gặp khi cài đặt WAL và động cơ LSM-Tree trong Rust:

| Mã lỗi | Thông báo mẫu từ trình biên dịch | Nguyên nhân cốt lõi | Cách khắc phục nhanh |
|---|---|---|---|
| **E0599** | `no method named 'lines' found for struct 'BufReader<File>'` | Bạn gọi phương thức `.lines()` để đọc từng dòng của tệp WAL nhưng chưa đưa trait `BufRead` vào phạm vi. | Thêm khai báo: `use std::io::BufRead;` ở đầu tệp mã nguồn. |
| **E0382** | `use of moved value: 'wal_file'` | Bạn truyền đối tượng `wal_file` vào một hàm phụ trợ khiến quyền sở hữu (ownership) bị chuyển đi, sau đó cố sử dụng lại trong struct. | Truyền tham chiếu mượn `&mut wal_file` vào hàm phụ trợ. |
| **E0596** | `cannot borrow 'engine' as mutable, as it is not declared as mutable` | Phương thức `.set()` đòi hỏi ghi vào tệp WAL và sửa đổi `MemTable`, bắt buộc đối tượng phải là `&mut self`. | Khai báo biến với `let mut engine = ...`. |
| **E0716** | `temporary value dropped while borrowed` | Bạn tạo một chuỗi định dạng tạm thời `format!(...).as_bytes()` và gán vào một biến mượn có thời gian sống (lifetime) dài hơn biểu thức tạm. | Tách riêng việc lưu trữ chuỗi `String` vào một biến cục bộ trước khi lấy mảng byte của nó. |

### Ví dụ phân tích lỗi `E0599` và cách khắc phục:

```rust
use std::fs::File;
use std::io::BufReader;
// Thiếu use std::io::BufRead;

// Đoạn mã lỗi minh họa E0599: Quên import trait BufRead
fn read_record_broken(f: File) {
    let reader = BufReader::new(f);
    // for line in reader.lines() { ... } // LỖI E0599: no method named `lines`!
}

// Cách sửa chữa đúng chuẩn: Import trait BufRead
use std::io::BufRead;

fn doc_dong_dung(f: File) -> std::io::Result<()> {
    let reader = BufReader::new(f);
    for line in reader.lines() {
        println!("Dòng nhật ký: {}", line?);
    }
    Ok(())
}
```

---

## Tóm tắt chương & Bài tập rèn luyện (Summary & Exercises)

### 4 Điểm cốt lõi cần ghi nhớ:
1. **Nguyên lý WAL tối thượng**: Không bao giờ được phép sửa dữ liệu trên RAM trước khi ghi nhận thành công thao tác vào tệp nhật ký nối đuôi (Append-only) trên đĩa cứng.
2. **Sức mạnh ghi tuần tự của LSM-Tree**: Bằng cách tiếp nhận dữ liệu trên RAM (`MemTable`) và ghi tuần tự (`WAL`), LSM-Tree đạt thông lượng ghi vượt trội hàng chục lần so với B+ Tree.
3. **Tính chất bất biến của SSTable**: Các tệp trên đĩa không bao giờ bị ghi đè; các bản ghi cập nhật hoặc bị xóa được đánh dấu bằng khóa mới hoặc cờ Tombstone.
4. **Tiến trình Compaction**: Đóng vai trò như người dọn dẹp vệ sinh chạy ngầm, sáp nhập nhiều tệp SSTable nhỏ thành tệp lớn và loại bỏ rác để giải phóng dung lượng đĩa.

### Bài tập rèn luyện tự giải:
1. **Bài tập 1 (Phục hồi sau sự cố)**:  
   Giả sử tệp WAL có nội dung sau:
   ```
   SET:a=10
   SET:b=20
   SET:a=30
   DEL:b
   SET:c=40
   ```
   Hãy cho biết sau khi chạy quy trình phục hồi `open()`, trong `MemTable` sẽ còn lại những khóa nào và giá trị tương ứng của chúng là bao nhiêu?
2. **Bài tập 2 (Xả dữ liệu Flush MemTable)**:  
   Viết phương thức `fn flush_to_sstable(&mut self, sstable_path: &str) -> io::Result<()>`: Khi số lượng khóa trong `memtable` vượt quá 5 phần tử, ghi toàn bộ các cặp khóa-giá trị đã được sắp xếp từ `memtable` ra một tệp văn bản mới, sau đó xóa sạch `memtable` và tạo lại tệp WAL mới rỗng.
3. **Bài tập 3 (Tư duy thiết kế)**:  
   Tại sao các hệ thống Big Data phân tán như Apache Cassandra hay Google Bigtable lại chọn kiến trúc LSM-Tree làm động cơ lưu trữ chính thay vì dùng B+ Tree? Trong trường hợp đọc dữ liệu ngẫu nhiên (Random Read), LSM-Tree có nhược điểm gì so với B+ Tree và kỹ thuật Bộ lọc Bloom (Bloom Filter) giúp khắc phục nhược điểm đó như thế nào?

---

### Gợi ý & Lời giải

<details>
<summary><b>Bài tập 1 — Gợi ý</b></summary>

Phát lại WAL theo **đúng thứ tự**, mỗi dòng ghi đè lên trạng thái trước. `DEL` bỏ khoá khỏi bảng. Dòng sau luôn thắng dòng trước.
</details>

<details>
<summary><b>Bài tập 1 — Lời giải</b></summary>

Phát lại từng dòng, trạng thái `MemTable` sau mỗi bước:

| Dòng WAL | Thao tác | MemTable sau đó |
|---|---|---|
| `SET:a=10` | thêm a | `{a: 10}` |
| `SET:b=20` | thêm b | `{a: 10, b: 20}` |
| `SET:a=30` | **ghi đè** a | `{a: 30, b: 20}` |
| `DEL:b` | bỏ b | `{a: 30}` |
| `SET:c=40` | thêm c | `{a: 30, c: 40}` |

**Kết quả cuối: `a = 30`, `c = 40`. Khoá `b` không còn.**

Ba điều bài này dạy:

1. **WAL là nhật ký ý định, không phải ảnh chụp trạng thái.** Nó ghi *bạn đã làm gì*, không phải *kết quả ra sao*. Trạng thái được dựng lại bằng cách phát lại theo thứ tự.
2. **Thứ tự là tất cả.** Đảo hai dòng `SET:a=10` và `SET:a=30` là ra kết quả khác. Vì vậy WAL bắt buộc ghi **tuần tự, chỉ nối thêm** — không bao giờ sửa dòng cũ.
3. **Bỏ khoá cũng là một bản ghi.** `DEL:b` không gỡ dòng `SET:b=20` khỏi tệp — nó *thêm* một dòng mới nói rằng b đã chết. Đây chính là **bia mộ** giống hệt Chương 32, và cũng để lại rác cần dọn về sau.
</details>

<details>
<summary><b>Bài tập 2 — Gợi ý</b></summary>

`BTreeMap` đã sắp xếp sẵn nên chỉ việc duyệt và ghi ra. Thứ tự hai việc — ghi SSTable rồi mới làm mới WAL — là vấn đề an toàn dữ liệu, không phải phong cách.
</details>

<details>
<summary><b>Bài tập 2 — Lời giải</b></summary>

```rust
use std::io::Write;

impl MiniLsmEngine {
    pub fn flush_to_sstable(&mut self, sstable_path: &str) -> io::Result<()> {
        if self.memtable.len() <= 5 { return Ok(()); }

        // 1. Ghi ra SSTable và ÉP XUỐNG ĐĨA trước.
        //    BTreeMap đã sắp xếp sẵn -> SSTable ra đời đã có thứ tự,
        //    nên về sau tra cứu bằng tìm kiếm nhị phân được.
        {
            let mut f = std::fs::File::create(sstable_path)?;
            for (k, v) in &self.memtable {
                writeln!(f, "{k}={v}")?;
            }
            f.sync_all()?;   // BẮT BUỘC: chưa sync thì dữ liệu mới ở cache HĐH
        }

        // 2. Chỉ SAU KHI SSTable nằm chắc trên đĩa mới được làm mới WAL.
        //    Làm ngược lại rồi mất điện -> mất trắng: WAL đã rỗng mà
        //    SSTable thì chưa kịp ghi.
        self.memtable.clear();
        self.wal_file = std::fs::File::options()
            .create(true).write(true).truncate(true).read(true)
            .open(&self.wal_path)?;
        Ok(())
    }
}

#[test]
fn xa_khi_vuot_nguong_va_lam_moi_wal() -> io::Result<()> {
    let tmp = std::env::temp_dir();
    let wal = tmp.join("ch34_flush_test.wal");
    let sst = tmp.join("ch34_flush_test.sst");

    let mut e = MiniLsmEngine::open(wal.to_str().unwrap())?;
    for i in 0..6 { e.set(&format!("k{i}"), &format!("v{i}"))?; }
    assert_eq!(e.total_keys(), 6);

    e.flush_to_sstable(sst.to_str().unwrap())?;
    assert_eq!(e.total_keys(), 0, "memtable phải rỗng sau khi xả");

    // SSTable phải ĐÃ SẮP XẾP — đó là điều làm nó tra cứu nhanh được.
    let noi_dung = std::fs::read_to_string(&sst)?;
    let khoa: Vec<&str> = noi_dung.lines().map(|l| l.split('=').next().unwrap()).collect();
    let mut sap = khoa.clone(); sap.sort();
    assert_eq!(khoa, sap, "SSTable phải sắp xếp theo khoá");
    Ok(())
}
```

Chi tiết quyết định đúng/sai: **`sync_all()` trước khi làm mới WAL**. Không có nó, dữ liệu mới còn nằm trong bộ đệm của hệ điều hành; mất điện lúc đó thì SSTable trống mà WAL cũng đã bị cắt. Đây là lý do mọi động cơ lưu trữ đều có một điểm `fsync` mà bạn không được phép bỏ qua vì lý do hiệu năng.
</details>

<details>
<summary><b>Bài tập 3 — Gợi ý</b></summary>

LSM-Tree tối ưu cho **ghi**, B+ Tree tối ưu cho **đọc**. Hãy nghĩ xem mỗi phép ghi trong hai kiến trúc phải chạm đĩa như thế nào.
</details>

<details>
<summary><b>Bài tập 3 — Lời giải</b></summary>

**Vì sao Cassandra và Bigtable chọn LSM-Tree:**

| | B+ Tree | LSM-Tree |
|---|---|---|
| Một phép ghi | tìm trang đúng, **đọc** nó, sửa, **ghi ngẫu nhiên** | nối vào WAL (**tuần tự**) + ghi vào RAM |
| Kiểu I/O khi ghi | ngẫu nhiên | tuần tự |
| Khuếch đại ghi | cao — ghi cả trang 4KB cho vài byte | thấp lúc ghi, dồn vào lúc nén |
| Thế mạnh | đọc | **ghi** |

Với tải ghi nặng — ghi log, dữ liệu chuỗi thời gian, bảng tin mạng xã hội — chênh lệch là hàng chục lần. LSM-Tree biến ghi ngẫu nhiên thành ghi tuần tự, đúng thủ thuật WAL dùng, nhưng nâng lên thành cả kiến trúc.

**Nhược điểm khi ĐỌC NGẪU NHIÊN:** dữ liệu của một khoá có thể ở MemTable, hoặc SSTable mới nhất, hoặc cũ hơn, hoặc cũ hơn nữa. Không thấy ở tầng này thì phải xuống tầng dưới. Đọc một khoá **không tồn tại** là tệ nhất — phải kiểm **mọi** SSTable rồi mới kết luận là không có:

```
get("khoa_khong_ton_tai"):
    MemTable   -> không thấy
    SSTable-1  -> đọc đĩa, không thấy
    SSTable-2  -> đọc đĩa, không thấy
    ...
    SSTable-N  -> đọc đĩa, không thấy
    => N lần đọc đĩa chỉ để trả lời "không có"
```

**Bộ lọc Bloom chữa đúng chỗ đó.** Mỗi SSTable kèm một cấu trúc bit nhỏ trả lời được *"khoá này CHẮC CHẮN không có trong tệp"* mà không chạm đĩa. Nó có thể báo nhầm "có thể có" (dương tính giả), nhưng **không bao giờ báo nhầm "không có"** — và tính bất đối xứng đó là toàn bộ giá trị:

```
get("khoa_khong_ton_tai") có Bloom filter:
    Bloom-1    -> "chắc chắn không có"  -> BỎ QUA, không đọc đĩa
    Bloom-2    -> "chắc chắn không có"  -> BỎ QUA
    => 0 lần đọc đĩa
```

Với tỉ lệ dương tính giả 1%, khoảng 10 bit mỗi khoá là đủ — vài megabyte RAM để tránh hàng triệu lần chạm đĩa. Một trong những đánh đổi bộ nhớ/tốc độ hời nhất trong ngành.
</details>
