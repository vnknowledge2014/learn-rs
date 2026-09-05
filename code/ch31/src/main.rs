#![allow(dead_code, unused_variables, unused_imports)]
use std::convert::TryInto;
use std::fs::{File, OpenOptions};
use std::io::{self, Read, Seek, SeekFrom, Write};
use std::path::Path;

/// Cấu trúc bản ghi người dùng trong cơ sở dữ liệu
#[derive(Debug, PartialEq, Clone)]
pub struct BanGhiNguoiDung {
    pub id: u32,       // 4 bytes cố định
    pub tuoi: u8,      // 1 byte cố định
    pub ho_ten: String,// Độ dài biến thiên
}

impl BanGhiNguoiDung {
    pub fn new(id: u32, tuoi: u8, ho_ten: &str) -> Self {
        Self {
            id,
            tuoi,
            ho_ten: ho_ten.to_string(),
        }
    }

    /// CHUYỂN ĐỔI THÀNH BYTE (Serialization)
    /// Cấu trúc nhị phân đóng gói:
    /// [ID: 4B] + [Tuổi: 1B] + [Độ dài tên: 2B] + [Dữ liệu chuỗi tên: NB]
    pub fn serialize(&self) -> Vec<u8> {
        let ten_bytes = self.ho_ten.as_bytes();
        let do_dai_ten = ten_bytes.len() as u16;

        // Ước tính trước kích thước để cấp phát bộ nhớ một lần duy nhất
        let mut bo_dem_byte = Vec::with_capacity(4 + 1 + 2 + ten_bytes.len());

        // 1. Ghi ID (4 bytes Little-Endian)
        bo_dem_byte.extend_from_slice(&self.id.to_le_bytes());
        // 2. Ghi Tuổi (1 byte)
        bo_dem_byte.push(self.tuoi);
        // 3. Ghi Độ dài chuỗi tên (2 bytes Little-Endian)
        bo_dem_byte.extend_from_slice(&do_dai_ten.to_le_bytes());
        // 4. Ghi Chuỗi byte nội dung tên UTF-8
        bo_dem_byte.extend_from_slice(ten_bytes);

        bo_dem_byte
    }

    /// GIẢI MÃ TỪ BYTE (Deserialization)
    pub fn deserialize(du_lieu: &[u8]) -> io::Result<(Self, usize)> {
        // Kích thước tối thiểu phần đầu (Header): 4 + 1 + 2 = 7 bytes
        if du_lieu.len() < 7 {
            return Err(io::Error::new(
                io::ErrorKind::UnexpectedEof,
                "Dữ liệu byte quá ngắn, không đủ đọc Header",
            ));
        }

        // Đọc ID
        let id_bytes: [u8; 4] = du_lieu[0..4].try_into().map_err(|_| {
            io::Error::new(io::ErrorKind::InvalidData, "Lỗi giải mã ID")
        })?;
        let id = u32::from_le_bytes(id_bytes);

        // Đọc Tuổi
        let tuoi = du_lieu[4];

        // Đọc Độ dài tên
        let len_bytes: [u8; 2] = du_lieu[5..7].try_into().map_err(|_| {
            io::Error::new(io::ErrorKind::InvalidData, "Lỗi giải mã độ dài chuỗi")
        })?;
        let do_dai_ten = u16::from_le_bytes(len_bytes) as usize;

        let tong_kich_thuoc = 7 + do_dai_ten;
        if du_lieu.len() < tong_kich_thuoc {
            return Err(io::Error::new(
                io::ErrorKind::UnexpectedEof,
                "Dữ liệu không đủ độ dài chuỗi tên như khai báo",
            ));
        }

        // Đọc chuỗi tên UTF-8
        let ho_ten = String::from_utf8(du_lieu[7..tong_kich_thuoc].to_vec()).map_err(|e| {
            io::Error::new(io::ErrorKind::InvalidData, e.to_string())
        })?;

        Ok((BanGhiNguoiDung { id, tuoi, ho_ten }, tong_kich_thuoc))
    }
}

/// Động cơ tệp nhị phân đơn giản lưu trữ các bản ghi xuống đĩa cứng
pub struct KhoLuuTruNhiPhan {
    tep: File,
}

impl KhoLuuTruNhiPhan {
    /// Mở hoặc tạo mới tệp lưu trữ dữ liệu
    pub fn open<P: AsRef<Path>>(duong_dan: P) -> io::Result<Self> {
        let tep = OpenOptions::new()
            .read(true)
            .write(true)
            .create(true)
            .open(duong_dan)?;
        Ok(Self { tep })
    }

    /// Ghi thêm bản ghi vào cuối tệp - Trả về tọa độ byte (Offset) bắt đầu của bản ghi
    pub fn ghi_ban_ghi(&mut self, ban_ghi: &BanGhiNguoiDung) -> io::Result<u64> {
        // Nhảy đến cuối tệp để ghi nối đuôi tuần tự (Sequential Append)
        let vi_tri_offset = self.tep.seek(SeekFrom::End(0))?;
        let bytes_can_ghi = ban_ghi.serialize();
        self.tep.write_all(&bytes_can_ghi)?;
        // Ép dữ liệu từ bộ nhớ đệm hệ điều hành xuống đĩa vật lý
        self.tep.flush()?;
        Ok(vi_tri_offset)
    }

    /// Nhảy đến vị trí Offset chính xác và đọc một bản ghi lên RAM - O(1) Disk Seek
    pub fn doc_ban_ghi_tai_offset(&mut self, offset: u64) -> io::Result<BanGhiNguoiDung> {
        self.tep.seek(SeekFrom::Start(offset))?;
        
        // Đọc trước 7 bytes phần đầu để biết độ dài chuỗi tên
        let mut header = [0u8; 7];
        self.tep.read_exact(&mut header)?;

        let len_bytes: [u8; 2] = header[5..7].try_into().unwrap();
        let do_dai_ten = u16::from_le_bytes(len_bytes) as usize;

        // Đọc tiếp phần thân chuỗi tên
        let mut ten_buffer = vec![0u8; do_dai_ten];
        self.tep.read_exact(&mut ten_buffer)?;

        // Ghép toàn bộ byte lại và giải mã
        let mut toan_bo_byte = Vec::with_capacity(7 + do_dai_ten);
        toan_bo_byte.extend_from_slice(&header);
        toan_bo_byte.extend_from_slice(&ten_buffer);

        let (ban_ghi, _) = BanGhiNguoiDung::deserialize(&toan_bo_byte)?;
        Ok(ban_ghi)
    }
}

fn main() -> io::Result<()> {
    println!("============================================================");
    println!("     CƠ CHẾ LƯU TRỮ ĐĨA CỨNG & TỆP NHỊ PHÂN TRONG RUST      ");
    println!("============================================================");

    // Sử dụng tệp tạm thời trong thư mục làm việc
    let duong_dan_tep = "kho_du_lieu_tam.bin";

    // 1. Khởi tạo kho lưu trữ
    let mut kho = KhoLuuTruNhiPhan::open(duong_dan_tep)?;
    println!("[1] Đã mở tệp lưu trữ nhị phân: '{}'", duong_dan_tep);

    // 2. Chuẩn bị dữ liệu và tuần tự hóa thành chuỗi byte
    let nguoi_1 = BanGhiNguoiDung::new(101, 24, "Nguyễn Văn An");
    let nguoi_2 = BanGhiNguoiDung::new(102, 30, "Trần Thị Bình");
    let nguoi_3 = BanGhiNguoiDung::new(103, 19, "Lê Hoàng Cường");

    println!("\n[2] Ghi tuần tự các bản ghi xuống đĩa:");
    let offset_1 = kho.ghi_ban_ghi(&nguoi_1)?;
    println!("    - Ghi bản ghi 101 ({}): Tọa độ byte = {}", nguoi_1.ho_ten, offset_1);

    let offset_2 = kho.ghi_ban_ghi(&nguoi_2)?;
    println!("    - Ghi bản ghi 102 ({}): Tọa độ byte = {}", nguoi_2.ho_ten, offset_2);

    let offset_3 = kho.ghi_ban_ghi(&nguoi_3)?;
    println!("    - Ghi bản ghi 103 ({}): Tọa độ byte = {}", nguoi_3.ho_ten, offset_3);

    // 3. Nhảy cóc ngẫu nhiên (Seek) đọc bản ghi bất kỳ mà không cần đọc từ đầu tệp!
    println!("\n[3] Đọc ngẫu nhiên bản ghi theo tọa độ byte (Offset):");
    let doc_lai_2 = kho.doc_ban_ghi_tai_offset(offset_2)?;
    println!("    - Nhảy tới offset {} đọc được: ID={}, Tuổi={}, Tên={}", 
        offset_2, doc_lai_2.id, doc_lai_2.tuoi, doc_lai_2.ho_ten);
    assert_eq!(doc_lai_2, nguoi_2);

    let doc_lai_1 = kho.doc_ban_ghi_tai_offset(offset_1)?;
    println!("    - Nhảy tới offset {} đọc được: ID={}, Tuổi={}, Tên={}", 
        offset_1, doc_lai_1.id, doc_lai_1.tuoi, doc_lai_1.ho_ten);
    assert_eq!(doc_lai_1, nguoi_1);

    let doc_lai_3 = kho.doc_ban_ghi_tai_offset(offset_3)?;
    println!("    - Nhảy tới offset {} đọc được: ID={}, Tuổi={}, Tên={}", 
        offset_3, doc_lai_3.id, doc_lai_3.tuoi, doc_lai_3.ho_ten);
    assert_eq!(doc_lai_3, nguoi_3);

    // 4. Dọn dẹp tệp thử nghiệm
    drop(kho); // Đóng tệp tin an toàn
    let _ = std::fs::remove_file(duong_dan_tep);
    println!("\n[4] Dọn dẹp tệp dữ liệu thử nghiệm thành công.");

    println!("============================================================");
    println!("               HOÀN TẤT THỰC NGHIỆM CHƯƠNG 27               ");
    println!("============================================================");
    Ok(())
}
