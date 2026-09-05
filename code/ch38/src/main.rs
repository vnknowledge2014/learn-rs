#![allow(dead_code, unused_variables, unused_imports)]
/// Cấu trúc mô phỏng một phiên đăng nhập người dùng an toàn
#[derive(Debug, Clone)]
pub struct SafeUserSession {
    pub username: String,
    pub is_admin: bool,
}

impl SafeUserSession {
    pub fn new(username: &str, is_admin: bool) -> Self {
        Self {
            username: username.to_string(),
            is_admin,
        }
    }
}

/// Trình xử lý bộ đệm an toàn tuyệt đối chống Buffer Overflow
pub struct SafeBufferManager {
    buffer: [u8; 16], // Bộ đệm cố định 16 bytes
}

impl SafeBufferManager {
    pub fn new() -> Self {
        Self { buffer: [0u8; 16] }
    }

    /// Ghi dữ liệu vào bộ đệm với cơ chế kiểm tra biên chặt chẽ
    pub fn safe_write(&mut self, input_data: &[u8]) -> Result<usize, &'static str> {
        if input_data.len() > self.buffer.len() {
            // Ngăn chặn tràn bộ đệm: Từ chối ghi đè khi dữ liệu quá lớn
            return Err("Kich thuoc du lieu vuot qua gioi han bo dem (Buffer Overflow prevented)!");
        }

        // Sao chép an toàn đúng số lượng byte hợp lệ
        for (idx, &byte) in input_data.iter().enumerate() {
            self.buffer[idx] = byte;
        }

        Ok(input_data.len())
    }

    /// Đọc một byte tại chỉ số xác định mà không gây panic sập chương trình
    pub fn safe_read(&self, index: usize) -> Option<u8> {
        self.buffer.get(index).copied()
    }
}

fn main() {
    println!("==================================================================");
    println!("   KIEM CHUNG AN TOAN BO NHO RUST: TRIET TIEU MEMORY CORRUPTION   ");
    println!("==================================================================");

    // -------------------------------------------------------------
    // 1. KIỂM THỬ PHÒNG CHỐNG TRÀN BỘ ĐỆM (BUFFER OVERFLOW)
    // -------------------------------------------------------------
    println!("\n[1] Thu nghiem phong chong Tran bo dem (Buffer Overflow):");
    let mut manager = SafeBufferManager::new();

    let safe_payload = b"MatKhauAnToan"; // 13 bytes (< 16 bytes)
    match manager.safe_write(safe_payload) {
        Ok(bytes_written) => println!("    - Ghi payload hop le thanh cong: {} bytes", bytes_written),
        Err(err) => println!("    - Loi: {}", err),
    }

    let exploit_payload = b"ChuoiPayloadRatDaiCoTinhLamTranBoNhoDeChiChiemThanhGhiRIP"; // 55 bytes
    println!("    - Thu gui payload tan cong co do dai {} bytes...", exploit_payload.len());
    match manager.safe_write(exploit_payload) {
        Ok(_) => println!("    - [NGUY HIEM] Payload da ghi de thanh cong!"),
        Err(err) => println!("    - [CHẶN ĐỨNG AN TOÀN] Trinh quan ly tu choi: '{}'", err),
    }

    // Đọc ngoài biên an toàn qua Option
    println!("    - Thu doc ky tu tai chi so index = 99:");
    match manager.safe_read(99) {
        Some(val) => println!("    - Gia tri: {}", val),
        None => println!("    - [SAFE BOUNDS] Tra ve None: Chi so ngoai bien duoc xu ly an toan!"),
    }

    // -------------------------------------------------------------
    // 2. KIỂM THỬ PHÒNG CHỐNG USE-AFTER-FREE (UAF)
    // -------------------------------------------------------------
    println!("\n[2] Thu nghiem phong chong Use-After-Free (UAF):");
    {
        let session = Box::new(SafeUserSession::new("ChuyenGiaBaoMat", false));
        println!("    - Khoi tao phien lam viec tai Heap: {:p}", session.as_ref());
        println!("    - Nguoi dung: {}, Admin: {}", session.username, session.is_admin);

        // Trong Rust, khi session ra khoi khoi lenh nay, trait Drop se tu dong
        // giai phong vung nho mot cach sach se. Trinh bien dich Rust tuyet doi
        // CAM moi hanh vi giu lai con tro tham chieu den session sau khi no da chet!
    }
    println!("    - [UAF ELIMINATED] Vung nho da duoc thu hoi tu dong.");
    println!("    - Trinh bien dich dam bao 100% khong con con tro lo lung ton tai!");

    // -------------------------------------------------------------
    // 3. KIỂM THỬ PHÒNG CHỐNG LỖ HỔNG FORMAT STRING
    // -------------------------------------------------------------
    println!("\n[3] Thu nghiem phong chong Lo hong Chuoi dinh dang (Format String):");
    // Giả sử kẻ tấn công cố tình nhập vào chuỗi chứa các mã ma thuật độc hại của C
    let malicious_user_input = "%x %x %s %p %n ChiemDoatBoNho";
    println!("    - Chuoi dau vao tu nguoi dung: '{}'", malicious_user_input);

    // Trong C: printf(malicious_user_input) se lam ro ri toan bo Stack.
    // Trong Rust: Chuoi nguoi dung chi la du lieu (data) truyen qua placeholder `{}`
    println!("    - Ket qua in qua Rust format: \"{}\"", malicious_user_input);
    println!("    - [FORMAT STRING SECURE] Rust coi chuoi nguoi dung la chuoi thuan túy,");
    println!("      khong bao gio phan tich cac ky tu '%' thanh lenh thuc thi!");

    println!("\n==================================================================");
    println!("   KET LUAN: RUST LOAI BO HOAN TOAN 70% NGUON GOC LO HONG CVE!   ");
    println!("==================================================================");
}
