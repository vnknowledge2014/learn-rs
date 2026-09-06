# Chương 31: Cơ chế lưu trữ đĩa cứng & Thao tác vào ra tệp nhị phân (Disk Storage & File I/O Mechanics)

## Giới thiệu & Mục tiêu học tập

Chào mừng bạn bước sang **Chủ đề 6: Kiến trúc & Thiết kế Cơ sở Dữ liệu trong Rust (Database Internals & Design)**! Trong 5 chủ đề trước, mọi cấu trúc dữ liệu mà bạn đã học — từ Mảng, Vector, Danh sách liên kết (Linked list), Cây nhị phân đến Bảng băm — đều tồn tại trên bộ nhớ truy cập ngẫu nhiên RAM. RAM có tốc độ xử lý nhanh như tia chớp (tính bằng nano-giây), nhưng nó có một điểm yếu chí mạng: **Dữ liệu sẽ bốc hơi hoàn toàn ngay khi máy tính bị ngắt nguồn điện (Volatile Memory)**.

Để xây dựng các hệ thống lưu trữ bền vững (Persistent Systems) như PostgreSQL, MySQL, SQLite, hay Redis, các kỹ sư phần mềm phải đối mặt với bài toán cốt lõi: **"Làm thế nào để đưa dữ liệu từ RAM xuống đĩa cứng (SSD/HDD) một cách an toàn, tin cậy, và đạt tốc độ cao nhất?"**

Ở cấp độ này, chúng ta không thể tiếp tục lưu trữ dữ liệu dưới dạng các tệp văn bản thuần túy như JSON hay CSV vì chúng cồng kềnh, tốn kém tài nguyên CPU để phân tích cú pháp chuỗi ký tự (string parsing), và không thể định vị nhanh bản ghi. Thay vào đó, chúng ta phải làm việc trực tiếp với **chuỗi byte nhị phân thô (`[u8]`)**, cơ chế căn chỉnh ô nhớ, và các thao tác vào/ra tệp tin (File I/O) cấp thấp trong Rust.

Mục tiêu học tập của chương này:
- Hiểu rõ sự khác biệt vật lý và độ trễ thời gian giữa RAM và Ổ đĩa lưu trữ (SSD/HDD).
- Phân biệt sâu sắc giữa **Ghi tuần tự (Sequential I/O)** và **Ghi ngẫu nhiên (Random I/O)**; giải thích vì sao ghi tuần tự luôn nhanh hơn hàng chục lần.
- Nắm vững kỹ thuật chuyển đổi (Serialization & Deserialization) giữa cấu trúc dữ liệu trong bộ nhớ và mảng byte nhị phân thô (`[u8]`) bằng Little-Endian.
- Làm chủ bộ công cụ thao tác tệp tin của Rust: `std::fs::File`, `OpenOptions`, `std::io::Seek`, và bộ nhớ đệm (buffer) với `BufReader` / `BufWriter`.
- Tự tay xây dựng một động cơ lưu trữ bản ghi nhị phân độc lập, có khả năng ghi nối đuôi và nhảy đến vị trí chính xác trên đĩa để đọc dữ liệu.

---

## Hình tượng hóa đời sống (Intuitive Everyday Analogy)

Hãy cùng quan sát cách làm việc tại một văn phòng lưu trữ hồ sơ để hình dung cơ chế hoạt động của RAM và Ổ đĩa cứng:

```
┌──────────────────────────────────────────────────────────────────────────────────┐
│              HÌNH TƯỢNG HÓA LƯU TRỮ: MẶT BÀN RAM VS KHO TẦNG HẦM ĐĨA CỨNG        │
├──────────────────────────────────────────────────────────────────────────────────┤
│                                                                                  │
│ [RAM: MẶT BÀN LÀM VIỆC NGAY TRƯỚC MẮT]                                           │
│   - Tốc độ: Lấy bút, viết sổ chỉ mất 1 giây (Nanoseconds).                       │
│   - Điểm yếu: Cuối ngày lao công lau dọn sạch trơn ném sọt rác (Mất điện).       │
│                                                                                  │
│ [Ổ CỨNG SSD/HDD: KHO LƯU TRỮ DƯỚI TẦNG HẦM]                                      │
│   - Tốc độ: Phải đi thang máy xuống mở khóa cửa kho (Chậm hơn hàng ngàn lần).    │
│   - Ưu điểm: Đóng thùng sắt khóa lại thì 10 năm sau quay lại đồ vẫn còn nguyên! │
│                                                                                  │
│ [SO SÁNH THAO TÁC VÀO RA ĐĨA: TUẦN TỰ VS NGẪU NHIÊN]                            │
│                                                                                  │
│ 1. Ghi tuần tự (Sequential I/O - Chép bài vào vở học sinh):                      │
│    ┌─────────┬─────────┬─────────┬─────────┐                                     │
│    │ Dòng 1  │ Dòng 2  │ Dòng 3  │ Dòng 4  │ -> Viết liên tục từ đầu đến cuối    │
│    └─────────┴─────────┴─────────┴─────────┘    Bút không rời giấy, siêu nhanh!  │
│                                                                                  │
│ 2. Ghi ngẫu nhiên (Random I/O - Nhảy trang lung tung):                           │
│    ┌─────────┐      ┌─────────┐      ┌─────────┐                                 │
│    │ Trang 1 │ ───► │Trang 50 │ ───► │Trang 12 │ -> Mất cả ngày chỉ để lật đi    │
│    │ (1 chữ) │      │ (1 chữ) │      │ (1 chữ) │    lật lại các trang sách!      │
│    └─────────┘      └─────────┘      └─────────┘                                 │
└──────────────────────────────────────────────────────────────────────────────────┘
```

### 1. Mặt bàn làm việc (RAM) vs Kho lưu trữ dưới tầng hầm (Ổ đĩa)
- **Mặt bàn làm việc (RAM)**: Bạn ngồi ngay tại bàn, với tay lấy chiếc bút hay tập giấy nháp chỉ mất 1 tích tắc. Bạn có thể ghi chép, tính toán cực nhanh. Nhưng khi hết giờ làm việc và tắt cầu dao điện văn phòng, nhân viên vệ sinh sẽ dọn sạch bóng mặt bàn, mọi thứ chưa kịp cất sẽ biến mất.
- **Kho lưu trữ tầng hầm (Ổ đĩa SSD/HDD)**: Khi muốn cất một tập hồ sơ quan trọng để lưu giữ qua nhiều năm, bạn phải bỏ hồ sơ vào cặp, đi thang máy xuống tầng hầm, mở cánh cửa sắt nặng nề và cất vào kệ tủ. Quá trình này tốn nhiều công sức hơn mặt bàn hàng ngàn lần, nhưng đổi lại, tài liệu nằm an toàn tuyệt đối qua năm tháng.

### 2. Chép bài tuần tự vs Lật sách ngẫu nhiên
- **Ghi tuần tự (Sequential I/O)**: Giống như bạn chép bài giảng vào cuốn sổ tay. Ngòi bút lia liên tục từ dòng 1 sang dòng 2, rồi sang dòng 3. Đầu ghi của đĩa cứng chỉ việc quay hoặc xả dòng điện liên tục vào các ô nhớ kế tiếp nhau, đạt tốc độ hàng trăm Megabytes đến Gigabytes mỗi giây.
- **Ghi ngẫu nhiên (Random I/O)**: Giống như việc bạn viết 1 chữ ở trang 1, sau đó bị bắt lật sang trang 50 viết 1 chữ, rồi lại lật ngược về trang 12 viết 1 chữ. Cần đọc cơ học của ổ cứng HDD phải di chuyển liên tục, còn chip nhớ SSD phải kích hoạt các khối nhớ rải rác, khiến tốc độ sụt giảm nghiêm trọng. Mọi hệ thống cơ sở dữ liệu hiện đại đều tìm mọi cách biến các thao tác ghi ngẫu nhiên thành ghi tuần tự!

---

## Khái niệm & Cơ chế kỹ thuật chuyên sâu (In-Depth Technical Mechanics)

### 1. Tại sao cơ sở dữ liệu không lưu trữ bằng tệp JSON hay CSV?

Hãy so sánh việc lưu một bản ghi người dùng gồm 3 trường: ID (số nguyên), Tuổi (số nguyên), Tên (chuỗi ký tự):
- **Định dạng JSON văn bản**:
  ```json
  {"id": 1001, "age": 25, "name": "Alice"}
  ```
  Chuỗi văn bản này tốn **40 bytes**. Khi cơ sở dữ liệu muốn đọc tuổi của Alice, CPU phải quét từ đầu chuỗi qua từng ký tự dấu ngoặc kép, dấu hai chấm, chuyển đổi ký tự `'2'` và `'5'` từ mã ASCII thành số nhị phân. Thao tác này cực kỳ chậm chạp!
- **Định dạng nhị phân thuần túy (Binary Serialization)**:
  - `id`: số nguyên 4 bytes (`u32`) -> `[0xE9, 0x03, 0x00, 0x00]`
  - `age`: số nguyên 1 byte (`u8`) -> `[0x19]` (số 25)
  - `name_len`: độ dài tên 2 bytes (`u16`) -> `[0x05, 0x00]`
  - `name`: chuỗi byte UTF-8 của "Alice" -> `[0x41, 0x6C, 0x69, 0x63, 0x65]`
  Tổng cộng chỉ tốn đúng **12 bytes** (tiết kiệm hơn 70% dung lượng đĩa), và CPU có thể đọc trực tiếp vào các thanh ghi mà không cần phân tích cú pháp!

### 2. Thứ tự byte: Little-Endian vs Big-Endian

Khi một số nguyên có kích thước lớn hơn 1 byte (như `u32` 4 bytes hoặc `u64` 8 bytes), làm sao sắp xếp các byte của nó xuống đĩa?
- **Little-Endian (Tiêu chuẩn x86/ARM hiện đại)**: Byte có giá trị nhỏ nhất (Least Significant Byte) được lưu ở địa chỉ ô nhớ đầu tiên.
  - Ví dụ số `u32` giá trị `1` sẽ lưu thành: `[1, 0, 0, 0]`.
- **Big-Endian (Tiêu chuẩn truyền thông mạng Network Order)**: Byte có giá trị lớn nhất được lưu đầu tiên: `[0, 0, 0, 1]`.

Trong Rust, chúng ta sử dụng hai phương thức chuẩn hóa:
- `so.to_le_bytes()`: Chuyển số nguyên thành mảng byte Little-Endian.
- `u32::from_le_bytes(bytes)`: Khôi phục mảng byte Little-Endian trở lại số nguyên.

### 3. Con trỏ dịch chuyển trên tệp: Trait `Seek`

Một tệp tin trên đĩa được hệ điều hành xem như một mảng byte khổng lồ có chỉ số từ `0` đến `capacity - 1`. Con trỏ đọc/ghi (Cursor/Offset) xác định vị trí mà lệnh đọc hoặc ghi tiếp theo sẽ diễn ra.

Rust cung cấp trait `std::io::Seek` với enum `SeekFrom`:
- `SeekFrom::Start(n)`: Nhảy con trỏ tới vị trí byte thứ `n` tính từ đầu tệp.
- `SeekFrom::Current(n)`: Dịch chuyển con trỏ thêm `n` bytes so với vị trí hiện tại.
- `SeekFrom::End(n)`: Nhảy con trỏ tới vị trí tính từ cuối tệp (dùng `SeekFrom::End(0)` để nhảy đến đuôi tệp chuẩn bị ghi chèn).

---

## Mã nguồn minh họa thực chiến (Idiomatic Runnable Rust Blueprint)

Dưới đây là một chương trình hoàn chỉnh cài đặt một hệ thống lưu trữ bản ghi nhị phân bền vững (Persistent Binary Record Store). Hệ thống hỗ trợ đóng gói bản ghi thành byte nhị phân, ghi tuần tự xuống đĩa cứng, nhảy con trỏ tìm kiếm bản ghi theo tọa độ byte (Offset), và khôi phục toàn bộ bản ghi:

```rust
use std::convert::TryInto;
use std::fs::{File, OpenOptions};
use std::io::{self, Read, Seek, SeekFrom, Write};
use std::path::Path;

/// Cấu trúc bản ghi người dùng trong cơ sở dữ liệu
#[derive(Debug, PartialEq, Clone)]
pub struct SellRecordUser {
    pub id: u32,       // 4 bytes cố định
    pub age: u8,      // 1 byte cố định
    pub full_name: String,// Độ dài biến thiên
}

impl SellRecordUser {
    pub fn new(id: u32, age: u8, full_name: &str) -> Self {
        Self {
            id,
            age,
            full_name: full_name.to_string(),
        }
    }

    /// CHUYỂN ĐỔI THÀNH BYTE (Serialization)
    /// Cấu trúc nhị phân đóng gói:
    /// [ID: 4B] + [Tuổi: 1B] + [Độ dài tên: 2B] + [Dữ liệu chuỗi tên: NB]
    pub fn serialize(&self) -> Vec<u8> {
        let ten_bytes = self.full_name.as_bytes();
        let do_long_name = ten_bytes.len() as u16;

        // Ước tính trước kích thước để cấp phát bộ nhớ một lần duy nhất
        let mut byte_buffer = Vec::with_capacity(4 + 1 + 2 + ten_bytes.len());

        // 1. Ghi ID (4 bytes Little-Endian)
        byte_buffer.extend_from_slice(&self.id.to_le_bytes());
        // 2. Ghi Tuổi (1 byte)
        byte_buffer.push(self.age);
        // 3. Ghi Độ dài chuỗi tên (2 bytes Little-Endian)
        byte_buffer.extend_from_slice(&do_long_name.to_le_bytes());
        // 4. Ghi Chuỗi byte nội dung tên UTF-8
        byte_buffer.extend_from_slice(ten_bytes);

        byte_buffer
    }

    /// GIẢI MÃ TỪ BYTE (Deserialization)
    pub fn deserialize(data: &[u8]) -> io::Result<(Self, usize)> {
        // Kích thước tối thiểu phần đầu (Header): 4 + 1 + 2 = 7 bytes
        if data.len() < 7 {
            return Err(io::Error::new(
                io::ErrorKind::UnexpectedEof,
                "Dữ liệu byte quá ngắn, không đủ đọc Header",
            ));
        }

        // Đọc ID
        let id_bytes: [u8; 4] = data[0..4].try_into().map_err(|_| {
            io::Error::new(io::ErrorKind::InvalidData, "Lỗi giải mã ID")
        })?;
        let id = u32::from_le_bytes(id_bytes);

        // Đọc Tuổi
        let age = data[4];

        // Đọc Độ dài tên
        let len_bytes: [u8; 2] = data[5..7].try_into().map_err(|_| {
            io::Error::new(io::ErrorKind::InvalidData, "Lỗi giải mã độ dài chuỗi")
        })?;
        let do_long_name = u16::from_le_bytes(len_bytes) as usize;

        let total_size = 7 + do_long_name;
        if data.len() < total_size {
            return Err(io::Error::new(
                io::ErrorKind::UnexpectedEof,
                "Dữ liệu không đủ độ dài chuỗi tên như khai báo",
            ));
        }

        // Đọc chuỗi tên UTF-8
        let full_name = String::from_utf8(data[7..total_size].to_vec()).map_err(|e| {
            io::Error::new(io::ErrorKind::InvalidData, e.to_string())
        })?;

        Ok((SellRecordUser { id, age, full_name }, total_size))
    }
}

/// Động cơ tệp nhị phân đơn giản lưu trữ các bản ghi xuống đĩa cứng
pub struct BinaryPageStore {
    file: File,
}

impl BinaryPageStore {
    /// Mở hoặc tạo mới tệp lưu trữ dữ liệu
    pub fn open<P: AsRef<Path>>(path: P) -> io::Result<Self> {
        let file = OpenOptions::new()
            .read(true)
            .write(true)
            .create(true)
            .open(path)?;
        Ok(Self { file })
    }

    /// Ghi thêm bản ghi vào cuối tệp - Trả về tọa độ byte (Offset) bắt đầu của bản ghi
    pub fn record_sell_record(&mut self, sell_record: &SellRecordUser) -> io::Result<u64> {
        // Nhảy đến cuối tệp để ghi nối đuôi tuần tự (Sequential Append)
        let vi_tri_offset = self.file.seek(SeekFrom::End(0))?;
        let bytes_to_write = sell_record.serialize();
        self.file.write_all(&bytes_to_write)?;
        // Ép dữ liệu từ bộ nhớ đệm hệ điều hành xuống đĩa vật lý
        self.file.flush()?;
        Ok(vi_tri_offset)
    }

    /// Nhảy đến vị trí Offset chính xác và đọc một bản ghi lên RAM - O(1) Disk Seek
    pub fn read_record_at(&mut self, offset: u64) -> io::Result<SellRecordUser> {
        self.file.seek(SeekFrom::Start(offset))?;
        
        // Đọc trước 7 bytes phần đầu để biết độ dài chuỗi tên
        let mut header = [0u8; 7];
        self.file.read_exact(&mut header)?;

        let len_bytes: [u8; 2] = header[5..7].try_into().unwrap();
        let do_long_name = u16::from_le_bytes(len_bytes) as usize;

        // Đọc tiếp phần thân chuỗi tên
        let mut ten_buffer = vec![0u8; do_long_name];
        self.file.read_exact(&mut ten_buffer)?;

        // Ghép toàn bộ byte lại và giải mã
        let mut all_bytes = Vec::with_capacity(7 + do_long_name);
        all_bytes.extend_from_slice(&header);
        all_bytes.extend_from_slice(&ten_buffer);

        let (sell_record, _) = SellRecordUser::deserialize(&all_bytes)?;
        Ok(sell_record)
    }
}

fn main() -> io::Result<()> {
    println!("============================================================");
    println!("     CƠ CHẾ LƯU TRỮ ĐĨA CỨNG & TỆP NHỊ PHÂN TRONG RUST      ");
    println!("============================================================");

    // Sử dụng tệp tạm thời trong thư mục làm việc
    let path_file = "kho_du_lieu_tam.bin";

    // 1. Khởi tạo kho lưu trữ
    let mut store = BinaryPageStore::open(path_file)?;
    println!("[1] Đã mở tệp lưu trữ nhị phân: '{}'", path_file);

    // 2. Chuẩn bị dữ liệu và tuần tự hóa thành chuỗi byte
    let person_1 = SellRecordUser::new(101, 24, "Nguyễn Văn An");
    let person_2 = SellRecordUser::new(102, 30, "Trần Thị Bình");
    let person_3 = SellRecordUser::new(103, 19, "Lê Hoàng Cường");

    println!("\n[2] Ghi tuần tự các bản ghi xuống đĩa:");
    let offset_1 = store.record_sell_record(&person_1)?;
    println!("    - Ghi bản ghi 101 ({}): Tọa độ byte = {}", person_1.full_name, offset_1);

    let offset_2 = store.record_sell_record(&person_2)?;
    println!("    - Ghi bản ghi 102 ({}): Tọa độ byte = {}", person_2.full_name, offset_2);

    let offset_3 = store.record_sell_record(&person_3)?;
    println!("    - Ghi bản ghi 103 ({}): Tọa độ byte = {}", person_3.full_name, offset_3);

    // 3. Nhảy cóc ngẫu nhiên (Seek) đọc bản ghi bất kỳ mà không cần đọc từ đầu tệp!
    println!("\n[3] Đọc ngẫu nhiên bản ghi theo tọa độ byte (Offset):");
    let doc_lai_2 = store.read_record_at(offset_2)?;
    println!("    - Nhảy tới offset {} đọc được: ID={}, Tuổi={}, Tên={}", 
        offset_2, doc_lai_2.id, doc_lai_2.age, doc_lai_2.full_name);
    assert_eq!(doc_lai_2, person_2);

    let doc_lai_1 = store.read_record_at(offset_1)?;
    println!("    - Nhảy tới offset {} đọc được: ID={}, Tuổi={}, Tên={}", 
        offset_1, doc_lai_1.id, doc_lai_1.age, doc_lai_1.full_name);
    assert_eq!(doc_lai_1, person_1);

    let doc_lai_3 = store.read_record_at(offset_3)?;
    println!("    - Nhảy tới offset {} đọc được: ID={}, Tuổi={}, Tên={}", 
        offset_3, doc_lai_3.id, doc_lai_3.age, doc_lai_3.full_name);
    assert_eq!(doc_lai_3, person_3);

    // 4. Dọn dẹp tệp thử nghiệm
    drop(store); // Đóng tệp tin an toàn
    let _ = std::fs::remove_file(path_file);
    println!("\n[4] Dọn dẹp tệp dữ liệu thử nghiệm thành công.");

    println!("============================================================");
    println!("               HOÀN TẤT THỰC NGHIỆM CHƯƠNG 27               ");
    println!("============================================================");
    Ok(())
}
```

---

## Bảng tra cứu lỗi biên dịch & Cách khắc phục (Compiler Error Guide)

Dưới đây là các lỗi biên dịch thường gặp nhất khi làm việc với tệp tin và mảng byte nhị phân trong Rust:

| Mã lỗi | Thông báo mẫu từ trình biên dịch | Nguyên nhân cốt lõi | Cách khắc phục nhanh |
|---|---|---|---|
| **E0599** | `no method named 'seek' found for struct 'File'` | Bạn gọi phương thức `.seek()` trên đối tượng `File` nhưng chưa đưa trait `Seek` vào phạm vi hoạt động. Trong Rust, muốn dùng phương thức của trait bắt buộc phải `use` trait đó. | Thêm dòng khai báo: `use std::io::Seek;` ở đầu tệp mã nguồn. |
| **E0599** | `no method named 'write_all' found for struct 'File'` | Tương tự lỗi trên, bạn gọi `.write_all()` mà quên đưa trait `Write` vào phạm vi. | Thêm dòng khai báo: `use std::io::Write;`. |
| **E0277** | `the trait bound '[u8]: Index<Range<usize>>' is not satisfied` | Bạn cố lấy lát cắt trên một con trỏ thô hoặc kiểu không hỗ trợ chỉ số mà quên chuyển đổi sang tham chiếu lát cắt `&[u8]`. | Đảm bảo biến mang kiểu tham chiếu lát cắt: `let slice = &buffer[start..end];`. |
| **E0308** | `mismatched types: expected '[u8; 4]', found '&[u8]'` | Hàm `from_le_bytes` đòi hỏi một mảng có kích thước cố định `[u8; 4]`, trong khi lát cắt `&bytes[0..4]` có kích thước động (`&[u8]`). | Sử dụng phương thức chuyển đổi an toàn: `bytes[0..4].try_into().unwrap()`. |

### Ví dụ phân tích lỗi `E0599` và cách khắc phục:

```rust
use std::fs::File;
// Thiếu use std::io::Write;

// Đoạn mã lỗi minh họa: Quên import trait Write
fn write_file_broken(mut f: File) {
    // f.write_all(b"Hello Rust").unwrap(); // LỖI E0599: no method named `write_all`!
}

// Cách sửa chữa đúng chuẩn: Import trait Write và Seek
use std::io::Write;

fn write_file_correct(mut f: File) -> std::io::Result<()> {
    f.write_all(b"Hello Rust")?;
    Ok(())
}
```

---

## Tóm tắt chương & Bài tập rèn luyện (Summary & Exercises)

### 4 Điểm cốt lõi cần ghi nhớ:
1. **RAM vs Ổ đĩa cứng**: RAM cực nhanh nhưng mất điện là mất sạch dữ liệu; Đĩa cứng chậm hơn nhưng lưu trữ vĩnh cửu. Cơ sở dữ liệu phải dung hòa tốc độ của RAM và tính bền vững của Đĩa.
2. **Ghi tuần tự là Vua**: Luôn ưu tiên ghi nối đuôi tuần tự (Sequential Append) thay vì nhảy cóc ghi ngẫu nhiên (Random I/O) để tận dụng tối đa băng thông phần cứng đĩa.
3. **Đóng gói nhị phân**: Lưu trữ dữ liệu dưới dạng byte nhị phân (`[u8]`) giúp tiết kiệm hơn 70% dung lượng và loại bỏ chi phí phân tích cú pháp chuỗi so với JSON/CSV.
4. **Quy tắc Little-Endian**: Sử dụng nhất quán `.to_le_bytes()` và `from_le_bytes()` để đảm bảo tệp dữ liệu có thể đọc được chính xác trên mọi kiến trúc máy tính khác nhau.

### Bài tập rèn luyện tự giải:
1. **Bài tập 1 (Phân biệt I/O)**:  
   Trong hai tác vụ sau đây của hệ quản trị cơ sở dữ liệu, tác vụ nào là Ghi tuần tự và tác vụ nào là Ghi ngẫu nhiên?
   - a) Ghi chép nhật ký mọi giao dịch chuyển tiền ngân hàng vào cuối tệp nhật ký WAL (Write-Ahead Log).
   - b) Cập nhật số dư tài khoản của một khách hàng vào trang dữ liệu số 4096 nằm rải rác trên đĩa cứng.
2. **Bài tập 2 (Xây dựng bộ tuần tự hóa Sản phẩm)**:  
   Định nghĩa struct `SanPham { ma_sp: u64, gia_tien: f64, con_hang: bool }`. Hãy tự viết hai hàm `serialize(&self) -> Vec<u8>` và `deserialize(bytes: &[u8]) -> Option<Self>` sử dụng `to_le_bytes()` và `from_le_bytes()`. Tính xem một bản ghi sản phẩm tốn chính xác bao nhiêu bytes trên đĩa cứng.
3. **Bài tập 3 (Thao tác Seek)**:  
   Viết một chương trình tạo tệp `so_nguyen.bin`, ghi 10 số nguyên `u32` từ 10 đến 100 vào tệp. Sử dụng `SeekFrom::Start` để nhảy thẳng tới vị trí số thứ 5 và đọc giá trị của nó lên màn hình mà không được đọc 4 số đầu tiên.
