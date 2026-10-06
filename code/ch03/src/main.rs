// File: src/main.rs
// Chương trình thực hành làm chủ Biến và Kiểu dữ liệu nguyên bản

fn main() {
    println!("=== 1. KHÁM PHÁ TÍNH BẤT BIẾN (IMMUTABILITY) ===");
    let founding_year = 2006; // Biến bất biến: không thể sửa
    println!(
        "Năm ngôn ngữ Rust bắt đầu được thai nghén: {}",
        founding_year
    );
    // Nếu bạn bỏ chú thích dòng dưới, compiler sẽ lập tức báo lỗi E0384:
    // founding_year = 2010;

    println!("\n=== 2. KHÁM PHÁ BIẾN KHẢ BIẾN VỚI TỪ KHÓA 'mut' ===");
    let mut rust_version = 1.0; // Chiếc bảng phấn: cho phép xóa đi viết lại
    println!("Phiên bản Rust ban đầu: {}", rust_version);

    rust_version = 1.97; // Cập nhật giá trị mới hợp lệ
    println!("Phiên bản Rust hiện đại : {}", rust_version);

    println!("\n=== 3. KỸ THUẬT CHE KHUẤT BIẾN (SHADOWING) ===");
    // Giả sử nhận được dữ liệu dạng chuỗi văn bản từ người dùng nhập
    let ticket_count = "5";
    println!("Dữ liệu người dùng nhập (chuỗi): {}", ticket_count);

    // Dán đè một biến mới cùng tên nhưng đổi kiểu dữ liệu sang số nguyên:
    let ticket_count: u32 = ticket_count.parse().expect("Không phải con số hợp lệ!");
    let total_price = ticket_count * 100_000; // Rust cho phép dùng dấu gạch dưới _ để số dễ đọc hơn
    println!("Số vé sau khi chuyển đổi: {} vé", ticket_count);
    println!("Tổng tiền cần thanh toán : {} VND", total_price);

    println!("\n=== 4. CÁC KIỂU DỮ LIỆU SỐ HỌC NGUYÊN BẢN ===");
    let age: u8 = 25; // Số nguyên không dấu 8-bit (0..255)
    let temperature: i16 = -15; // Số nguyên có dấu 16-bit
    let vietnam_population: u32 = 100_000_000; // Số nguyên không dấu 32-bit
    let moon_distance: f64 = 384_400.5; // Khoảng cách tới Mặt Trăng (km)

    println!(
        "Tuổi học viên   : {} tuổi (chiếm {} byte)",
        age,
        std::mem::size_of_val(&age)
    );
    println!(
        "Nhiệt độ mùa đông: {}°C (chiếm {} bytes)",
        temperature,
        std::mem::size_of_val(&temperature)
    );
    println!(
        "Dân số Việt Nam : {} người (chiếm {} bytes)",
        vietnam_population,
        std::mem::size_of_val(&vietnam_population)
    );
    println!(
        "Khoảng cách trăng: {} km (chiếm {} bytes)",
        moon_distance,
        std::mem::size_of_val(&moon_distance)
    );

    println!("\n=== 5. KIỂU LOGIC VÀ KÝ TỰ UNICODE ===");
    let learning_rust: bool = true;
    let emoji: char = '🎯'; // Ký tự Unicode chiếm trọn vẹn 4 bytes
    let vietnamese_char: char = 'Đ';

    println!("Đang say mê học Rust? {}", learning_rust);
    println!("Mục tiêu học tập    : {}", emoji);
    println!("Chữ cái tiếng Việt  : {}", vietnamese_char);
    println!(
        "Kích thước char trên RAM: {} bytes",
        std::mem::size_of::<char>()
    );

    println!("\n=== 6. ÉP KIỂU AN TOÀN VỚI TỪ KHÓA 'as' ===");
    let attendance_score: u8 = 9;
    let exam_score: f32 = 8.5;
    // Để cộng số nguyên với số thực, ta phải chủ động ép kiểu (explicit casting)
    let final_score = (attendance_score as f32 * 0.3) + (exam_score * 0.7);
    println!("Điểm tổng kết môn học: {:.2}", final_score);
}
