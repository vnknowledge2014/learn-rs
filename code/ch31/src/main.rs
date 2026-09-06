#![allow(dead_code, unused_variables, unused_imports)]
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
        let mut bo_dem_byte = Vec::with_capacity(4 + 1 + 2 + ten_bytes.len());

        // 1. Ghi ID (4 bytes Little-Endian)
        bo_dem_byte.extend_from_slice(&self.id.to_le_bytes());
        // 2. Ghi Tuổi (1 byte)
        bo_dem_byte.push(self.age);
        // 3. Ghi Độ dài chuỗi tên (2 bytes Little-Endian)
        bo_dem_byte.extend_from_slice(&do_long_name.to_le_bytes());
        // 4. Ghi Chuỗi byte nội dung tên UTF-8
        bo_dem_byte.extend_from_slice(ten_bytes);

        bo_dem_byte
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
        let mut toan_bo_byte = Vec::with_capacity(7 + do_long_name);
        toan_bo_byte.extend_from_slice(&header);
        toan_bo_byte.extend_from_slice(&ten_buffer);

        let (sell_record, _) = SellRecordUser::deserialize(&toan_bo_byte)?;
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
    let nguoi_1 = SellRecordUser::new(101, 24, "Nguyễn Văn An");
    let nguoi_2 = SellRecordUser::new(102, 30, "Trần Thị Bình");
    let nguoi_3 = SellRecordUser::new(103, 19, "Lê Hoàng Cường");

    println!("\n[2] Ghi tuần tự các bản ghi xuống đĩa:");
    let offset_1 = store.record_sell_record(&nguoi_1)?;
    println!("    - Ghi bản ghi 101 ({}): Tọa độ byte = {}", nguoi_1.full_name, offset_1);

    let offset_2 = store.record_sell_record(&nguoi_2)?;
    println!("    - Ghi bản ghi 102 ({}): Tọa độ byte = {}", nguoi_2.full_name, offset_2);

    let offset_3 = store.record_sell_record(&nguoi_3)?;
    println!("    - Ghi bản ghi 103 ({}): Tọa độ byte = {}", nguoi_3.full_name, offset_3);

    // 3. Nhảy cóc ngẫu nhiên (Seek) đọc bản ghi bất kỳ mà không cần đọc từ đầu tệp!
    println!("\n[3] Đọc ngẫu nhiên bản ghi theo tọa độ byte (Offset):");
    let doc_lai_2 = store.read_record_at(offset_2)?;
    println!("    - Nhảy tới offset {} đọc được: ID={}, Tuổi={}, Tên={}", 
        offset_2, doc_lai_2.id, doc_lai_2.age, doc_lai_2.full_name);
    assert_eq!(doc_lai_2, nguoi_2);

    let doc_lai_1 = store.read_record_at(offset_1)?;
    println!("    - Nhảy tới offset {} đọc được: ID={}, Tuổi={}, Tên={}", 
        offset_1, doc_lai_1.id, doc_lai_1.age, doc_lai_1.full_name);
    assert_eq!(doc_lai_1, nguoi_1);

    let doc_lai_3 = store.read_record_at(offset_3)?;
    println!("    - Nhảy tới offset {} đọc được: ID={}, Tuổi={}, Tên={}", 
        offset_3, doc_lai_3.id, doc_lai_3.age, doc_lai_3.full_name);
    assert_eq!(doc_lai_3, nguoi_3);

    // 4. Dọn dẹp tệp thử nghiệm
    drop(store); // Đóng tệp tin an toàn
    let _ = std::fs::remove_file(path_file);
    println!("\n[4] Dọn dẹp tệp dữ liệu thử nghiệm thành công.");

    println!("============================================================");
    println!("               HOÀN TẤT THỰC NGHIỆM CHƯƠNG 27               ");
    println!("============================================================");
    Ok(())
}
