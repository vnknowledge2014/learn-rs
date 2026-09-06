#![allow(dead_code, unused_variables, unused_imports)]
// File: src/main.rs
// Chương trình thực hành làm chủ Biến và Kiểu dữ liệu nguyên bản

fn main() {
    println!("=== 1. KHÁM PHÁ TÍNH BẤT BIẾN (IMMUTABILITY) ===");
    let founding_year = 2006; // Biến bất biến: không thể sửa
    println!("Năm ngôn ngữ Rust bắt đầu được thai nghén: {}", founding_year);
    // Nếu bạn bỏ chú thích dòng dưới, compiler sẽ lập tức báo lỗi E0384:
    // founding_year = 2010;

    println!("\n=== 2. KHÁM PHÁ BIẾN KHẢ BIẾN VỚI TỪ KHÓA 'mut' ===");
    let mut rust_version = 1.0; // Chiếc bảng phấn: cho phép xóa đi viết lại
    println!("Phiên bản Rust ban đầu: {}", rust_version);
    
    rust_version = 1.85; // Cập nhật giá trị mới hợp lệ
    println!("Phiên bản Rust hiện đại : {}", rust_version);

    println!("\n=== 3. KỸ THUẬT CHE KHUẤT BIẾN (SHADOWING) ===");
    // Giả sử nhận được dữ liệu dạng chuỗi văn bản từ người dùng nhập
    let quantity_ve = "5"; 
    println!("Dữ liệu người dùng nhập (chuỗi): {}", quantity_ve);

    // Dán đè một biến mới cùng tên nhưng đổi kiểu dữ liệu sang số nguyên:
    let quantity_ve: u32 = quantity_ve.parse().expect("Không phải con số hợp lệ!");
    let tong_tien = quantity_ve * 100_000; // Rust cho phép dùng dấu gạch dưới _ để số dễ đọc hơn
    println!("Số vé sau khi chuyển đổi: {} vé", quantity_ve);
    println!("Tổng tiền cần thanh toán : {} VND", tong_tien);

    println!("\n=== 4. CÁC KIỂU DỮ LIỆU SỐ HỌC NGUYÊN BẢN ===");
    let age: u8 = 25;                       // Số nguyên không dấu 8-bit (0..255)
    let nhiet_do: i16 = -15;                  // Số nguyên có dấu 16-bit
    let derive_num_write_nam: u32 = 100_000_000;   // Số nguyên không dấu 32-bit
    let pos_value_distance: f64 = 384_400.5; // Khoảng cách tới Mặt Trăng (km)
    
    println!("Tuổi học viên   : {} tuổi (chiếm {} byte)", age, std::mem::size_of_val(&age));
    println!("Nhiệt độ mùa đông: {}°C (chiếm {} bytes)", nhiet_do, std::mem::size_of_val(&nhiet_do));
    println!("Dân số Việt Nam : {} người (chiếm {} bytes)", derive_num_write_nam, std::mem::size_of_val(&derive_num_write_nam));
    println!("Khoảng cách trăng: {} km (chiếm {} bytes)", pos_value_distance, std::mem::size_of_val(&pos_value_distance));

    println!("\n=== 5. KIỂU LOGIC VÀ KÝ TỰ UNICODE ===");
    let dang_hoc_rust: bool = true;
    let bieu_cam: char = '🎯'; // Ký tự Unicode chiếm trọn vẹn 4 bytes
    let ky_tu_tieng_viet: char = 'Đ';

    println!("Đang say mê học Rust? {}", dang_hoc_rust);
    println!("Mục tiêu học tập    : {}", bieu_cam);
    println!("Chữ cái tiếng Việt  : {}", ky_tu_tieng_viet);
    println!("Kích thước char trên RAM: {} bytes", std::mem::size_of::<char>());

    println!("\n=== 6. ÉP KIỂU AN TOÀN VỚI TỪ KHÓA 'as' ===");
    let point_transfer_can: u8 = 9;
    let exam_score: f32 = 8.5;
    // Để cộng số nguyên với số thực, ta phải chủ động ép kiểu (explicit casting)
    let diem_tong_ket = (point_transfer_can as f32 * 0.3) + (exam_score * 0.7);
    println!("Điểm tổng kết môn học: {:.2}", diem_tong_ket);
}
