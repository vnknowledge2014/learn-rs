#![allow(dead_code, unused_variables, unused_imports)]
use std::convert::TryInto;
use std::fs::{File, OpenOptions};
use std::io::{self, Read, Seek, SeekFrom, Write};
use std::path::Path;

/// Cấu trúc bản ghi người dùng trong cơ sở dữ liệu
#[derive(Debug, PartialEq, Clone)]
pub struct UserRecord {
    pub id: u32,           // 4 bytes cố định
    pub age: u8,           // 1 byte cố định
    pub full_name: String, // Độ dài biến thiên
}

impl UserRecord {
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
        let name_bytes = self.full_name.as_bytes();
        let name_len = name_bytes.len() as u16;

        // Ước tính trước kích thước để cấp phát bộ nhớ một lần duy nhất
        let mut byte_buffer = Vec::with_capacity(4 + 1 + 2 + name_bytes.len());

        // 1. Ghi ID (4 bytes Little-Endian)
        byte_buffer.extend_from_slice(&self.id.to_le_bytes());
        // 2. Ghi Tuổi (1 byte)
        byte_buffer.push(self.age);
        // 3. Ghi Độ dài chuỗi tên (2 bytes Little-Endian)
        byte_buffer.extend_from_slice(&name_len.to_le_bytes());
        // 4. Ghi Chuỗi byte nội dung tên UTF-8
        byte_buffer.extend_from_slice(name_bytes);

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
        let id_bytes: [u8; 4] = data[0..4]
            .try_into()
            .map_err(|_| io::Error::new(io::ErrorKind::InvalidData, "Lỗi giải mã ID"))?;
        let id = u32::from_le_bytes(id_bytes);

        // Đọc Tuổi
        let age = data[4];

        // Đọc Độ dài tên
        let len_bytes: [u8; 2] = data[5..7]
            .try_into()
            .map_err(|_| io::Error::new(io::ErrorKind::InvalidData, "Lỗi giải mã độ dài chuỗi"))?;
        let name_len = u16::from_le_bytes(len_bytes) as usize;

        let total_size = 7 + name_len;
        if data.len() < total_size {
            return Err(io::Error::new(
                io::ErrorKind::UnexpectedEof,
                "Dữ liệu không đủ độ dài chuỗi tên như khai báo",
            ));
        }

        // Đọc chuỗi tên UTF-8
        let full_name = String::from_utf8(data[7..total_size].to_vec())
            .map_err(|e| io::Error::new(io::ErrorKind::InvalidData, e.to_string()))?;

        Ok((UserRecord { id, age, full_name }, total_size))
    }
}

/// Động cơ tệp nhị phân đơn giản lưu trữ các bản ghi xuống đĩa cứng
pub struct BinaryStore {
    file: File,
}

impl BinaryStore {
    /// Mở hoặc tạo mới tệp lưu trữ dữ liệu
    pub fn open<P: AsRef<Path>>(path: P) -> io::Result<Self> {
        let file = OpenOptions::new()
            .read(true)
            .write(true)
            .create(true)
            // Không xoá dữ liệu cũ khi mở lại: kho chỉ ghi nối đuôi
            .truncate(false)
            .open(path)?;
        Ok(Self { file })
    }

    /// Ghi thêm bản ghi vào cuối tệp - Trả về tọa độ byte (Offset) bắt đầu của bản ghi
    pub fn append_record(&mut self, record: &UserRecord) -> io::Result<u64> {
        // Nhảy đến cuối tệp để ghi nối đuôi tuần tự (Sequential Append)
        let offset = self.file.seek(SeekFrom::End(0))?;
        let bytes_to_write = record.serialize();
        self.file.write_all(&bytes_to_write)?;
        // write_all chỉ chép dữ liệu vào bộ nhớ đệm trang (page cache) của hệ điều hành.
        // `File::flush()` KHÔNG giúp gì (File không có bộ đệm phía người dùng);
        // phải gọi sync_data() (fdatasync) để ép dữ liệu xuống đĩa vật lý.
        self.file.sync_data()?;
        Ok(offset)
    }

    /// Nhảy đến vị trí Offset chính xác và đọc một bản ghi lên RAM - O(1) Disk Seek
    pub fn read_record_at(&mut self, offset: u64) -> io::Result<UserRecord> {
        self.file.seek(SeekFrom::Start(offset))?;

        // Đọc trước 7 bytes phần đầu để biết độ dài chuỗi tên
        let mut header = [0u8; 7];
        self.file.read_exact(&mut header)?;

        let len_bytes: [u8; 2] = header[5..7].try_into().unwrap();
        let name_len = u16::from_le_bytes(len_bytes) as usize;

        // Đọc tiếp phần thân chuỗi tên
        let mut name_buf = vec![0u8; name_len];
        self.file.read_exact(&mut name_buf)?;

        // Ghép toàn bộ byte lại và giải mã
        let mut all_bytes = Vec::with_capacity(7 + name_len);
        all_bytes.extend_from_slice(&header);
        all_bytes.extend_from_slice(&name_buf);

        let (record, _) = UserRecord::deserialize(&all_bytes)?;
        Ok(record)
    }
}

fn main() -> io::Result<()> {
    println!("============================================================");
    println!("     CƠ CHẾ LƯU TRỮ ĐĨA CỨNG & TỆP NHỊ PHÂN TRONG RUST      ");
    println!("============================================================");

    // Sử dụng tệp trong thư mục tạm của hệ điều hành
    let path = std::env::temp_dir().join("ch31_records.bin");

    // 1. Khởi tạo kho lưu trữ
    let _ = std::fs::remove_file(&path); // xoá dấu vết của lần chạy trước (nếu có)
    let mut store = BinaryStore::open(&path)?;
    println!("[1] Đã mở tệp lưu trữ nhị phân: '{}'", path.display());

    // 2. Chuẩn bị dữ liệu và tuần tự hóa thành chuỗi byte
    let person_1 = UserRecord::new(101, 24, "Nguyễn Văn An");
    let person_2 = UserRecord::new(102, 30, "Trần Thị Bình");
    let person_3 = UserRecord::new(103, 19, "Lê Hoàng Cường");

    println!("\n[2] Ghi tuần tự các bản ghi xuống đĩa:");
    let offset_1 = store.append_record(&person_1)?;
    println!(
        "    - Ghi bản ghi 101 ({}): Tọa độ byte = {}",
        person_1.full_name, offset_1
    );

    let offset_2 = store.append_record(&person_2)?;
    println!(
        "    - Ghi bản ghi 102 ({}): Tọa độ byte = {}",
        person_2.full_name, offset_2
    );

    let offset_3 = store.append_record(&person_3)?;
    println!(
        "    - Ghi bản ghi 103 ({}): Tọa độ byte = {}",
        person_3.full_name, offset_3
    );

    // 3. Nhảy cóc ngẫu nhiên (Seek) đọc bản ghi bất kỳ mà không cần đọc từ đầu tệp!
    println!("\n[3] Đọc ngẫu nhiên bản ghi theo tọa độ byte (Offset):");
    let read_back_2 = store.read_record_at(offset_2)?;
    println!(
        "    - Nhảy tới offset {} đọc được: ID={}, Tuổi={}, Tên={}",
        offset_2, read_back_2.id, read_back_2.age, read_back_2.full_name
    );
    assert_eq!(read_back_2, person_2);

    let read_back_1 = store.read_record_at(offset_1)?;
    println!(
        "    - Nhảy tới offset {} đọc được: ID={}, Tuổi={}, Tên={}",
        offset_1, read_back_1.id, read_back_1.age, read_back_1.full_name
    );
    assert_eq!(read_back_1, person_1);

    let read_back_3 = store.read_record_at(offset_3)?;
    println!(
        "    - Nhảy tới offset {} đọc được: ID={}, Tuổi={}, Tên={}",
        offset_3, read_back_3.id, read_back_3.age, read_back_3.full_name
    );
    assert_eq!(read_back_3, person_3);

    // 4. Dọn dẹp tệp thử nghiệm
    drop(store); // Đóng tệp tin an toàn
    let _ = std::fs::remove_file(&path);
    println!("\n[4] Dọn dẹp tệp dữ liệu thử nghiệm thành công.");

    println!("============================================================");
    println!("               HOÀN TẤT THỰC NGHIỆM CHƯƠNG 31               ");
    println!("============================================================");
    Ok(())
}
