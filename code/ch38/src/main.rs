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

impl Default for SafeBufferManager {
    fn default() -> Self {
        Self::new()
    }
}

impl SafeBufferManager {
    pub fn new() -> Self {
        Self { buffer: [0u8; 16] }
    }

    /// Ghi dữ liệu vào bộ đệm với cơ chế kiểm tra biên chặt chẽ
    pub fn safe_write(&mut self, input_data: &[u8]) -> Result<usize, &'static str> {
        if input_data.len() > self.buffer.len() {
            // Ngăn chặn tràn bộ đệm: Từ chối ghi đè khi dữ liệu quá lớn
            return Err("Kích thước dữ liệu vượt quá giới hạn bộ đệm (đã chặn Buffer Overflow)!");
        }

        // Sao chép an toàn đúng số lượng byte hợp lệ.
        // (Kể cả nếu quên kiểm tra ở trên, copy_from_slice cũng panic khi độ dài lệch
        // chứ không bao giờ ghi lấn ra ngoài mảng như memcpy của C.)
        self.buffer[..input_data.len()].copy_from_slice(input_data);

        Ok(input_data.len())
    }

    /// Đọc một byte tại chỉ số xác định mà không gây panic sập chương trình
    pub fn safe_read(&self, index: usize) -> Option<u8> {
        self.buffer.get(index).copied()
    }
}

fn main() {
    println!("==================================================================");
    println!("   KIỂM CHỨNG AN TOÀN BỘ NHỚ RUST: TRIỆT TIÊU MEMORY CORRUPTION   ");
    println!("==================================================================");

    // -------------------------------------------------------------
    // 1. KIỂM THỬ PHÒNG CHỐNG TRÀN BỘ ĐỆM (BUFFER OVERFLOW)
    // -------------------------------------------------------------
    println!("\n[1] Thử nghiệm phòng chống Tràn bộ đệm (Buffer Overflow):");
    let mut manager = SafeBufferManager::new();

    let safe_payload = b"MatKhauAnToan"; // 13 bytes (< 16 bytes)
    match manager.safe_write(safe_payload) {
        Ok(bytes_written) => println!(
            "    - Ghi payload hợp lệ thành công: {} bytes",
            bytes_written
        ),
        Err(err) => println!("    - Thất bại: {}", err),
    }

    let exploit_payload = b"ChuoiPayloadRatDaiCoTinhLamTranBoNhoDeChiChiemThanhGhiRIP"; // 57 bytes
    println!(
        "    - Thử gửi payload tấn công có độ dài {} bytes...",
        exploit_payload.len()
    );
    match manager.safe_write(exploit_payload) {
        Ok(_) => println!("    - [NGUY HIỂM] Payload đã ghi đè thành công!"),
        Err(err) => println!("    - [CHẶN ĐỨNG AN TOÀN] Trình quản lý từ chối: '{}'", err),
    }

    // Đọc ngoài biên an toàn qua Option
    println!("    - Thử đọc ký tự tại chỉ số index = 99:");
    match manager.safe_read(99) {
        Some(val) => println!("    - Giá trị: {}", val),
        None => println!("    - [SAFE BOUNDS] Trả về None: Chỉ số ngoài biên được xử lý an toàn!"),
    }

    // -------------------------------------------------------------
    // 2. KIỂM THỬ PHÒNG CHỐNG USE-AFTER-FREE (UAF)
    // -------------------------------------------------------------
    println!("\n[2] Thử nghiệm phòng chống Use-After-Free (UAF):");
    {
        let session = Box::new(SafeUserSession::new("Chuyên gia bảo mật", false));
        println!(
            "    - Khởi tạo phiên làm việc tại Heap: {:p}",
            session.as_ref()
        );
        println!(
            "    - Người dùng: {}, Admin: {}",
            session.username, session.is_admin
        );

        // Trong Rust, khi session ra khỏi khối lệnh này, trait Drop sẽ tự động
        // giải phóng vùng nhớ một cách sạch sẽ. Trình biên dịch Rust tuyệt đối
        // CẤM mọi hành vi giữ lại tham chiếu đến session sau khi nó đã chết!
    }
    println!("    - [UAF ELIMINATED] Vùng nhớ đã được thu hồi tự động.");
    println!("    - Trình biên dịch đảm bảo không còn tham chiếu lơ lửng nào tồn tại!");

    // -------------------------------------------------------------
    // 3. KIỂM THỬ PHÒNG CHỐNG LỖ HỔNG FORMAT STRING
    // -------------------------------------------------------------
    println!("\n[3] Thử nghiệm phòng chống Lỗ hổng Chuỗi định dạng (Format String):");
    // Giả sử kẻ tấn công cố tình nhập vào chuỗi chứa các mã ma thuật độc hại của C
    let malicious_user_input = "%x %x %s %p %n Chiếm đoạt bộ nhớ";
    println!(
        "    - Chuỗi đầu vào từ người dùng: '{}'",
        malicious_user_input
    );

    // Trong C: printf(malicious_user_input) sẽ làm rò rỉ nội dung Stack.
    // Trong Rust: chuỗi người dùng chỉ là dữ liệu (data) truyền qua placeholder `{}`
    println!(
        "    - Kết quả in qua Rust format: \"{}\"",
        malicious_user_input
    );
    println!("    - [FORMAT STRING SECURE] Rust coi chuỗi người dùng là chuỗi thuần túy,");
    println!("      không bao giờ phân tích các ký tự '%' thành lệnh thực thi!");

    println!("\n==================================================================");
    println!("   KẾT LUẬN: RUST AN TOÀN LOẠI BỎ CẢ LỚP LỖI GÂY RA ~70% CVE!   ");
    println!("==================================================================");
}
