#![allow(dead_code, unused_variables, unused_imports)]
// File: src/main.rs
// Chương trình minh họa việc in ấn thông tin và kiểm tra công cụ Rust

fn main() {
    // 1. In một dòng chữ đơn giản kèm ký tự xuống dòng tự động
    println!("Xin chào! Chào mừng bạn đến với thế giới lập trình Rust!");

    // 2. Sử dụng dấu ngoặc nhọn {} làm "vị trí giữ chỗ định dạng" (Format slot)
    let ten_khoa_hoc = "Rust Masterclass Toàn Diện";
    let so_chuong = 12;
    println!("Bạn đang tham gia khóa học: {}", ten_khoa_hoc);
    println!("Giai đoạn nền tảng bao gồm: {} chương chuyên sâu.", so_chuong);

    // 3. Truyền nhiều giá trị vào cùng một câu thông báo
    let nguoi_hoc = "Lập trình viên tương lai";
    let muc_tieu = "Làm chủ bộ nhớ và hệ thống";
    println!("Học viên [{}] đặt mục tiêu: [{}]", nguoi_hoc, muc_tieu);

    // 4. Các kỹ thuật định dạng văn bản nâng cao với println!
    // In số với khoảng cách căn lề cố định (rất hữu ích khi in bảng biểu dữ liệu)
    println!("------------------------------------------------------------");
    println!("| {:<15} | {:<20} | {:>10} |", "MÃ CHƯƠNG", "CHỦ ĐỀ HỌC", "TRẠNG THÁI");
    println!("------------------------------------------------------------");
    println!("| {:<15} | {:<20} | {:>10} |", "Chương 01", "Phần cứng & CPU", "Hoàn thành");
    println!("| {:<15} | {:<20} | {:>10} |", "Chương 02", "Rust & Cargo", "Đang học");
    println!("| {:<15} | {:<20} | {:>10} |", "Chương 03", "Biến & Kiểu dữ liệu", "Sắp tới");
    println!("------------------------------------------------------------");

    // 5. In biểu diễn số ở các hệ cơ số khác nhau mà không cần tính toán thủ công
    let gia_tri_mau = 255;
    println!("Con số {} trong các hệ đếm máy tính:", gia_tri_mau);
    println!("- Hệ thập phân (Decimal)     : {}", gia_tri_mau);
    println!("- Hệ nhị phân (Binary)       : {:08b}", gia_tri_mau);
    println!("- Hệ thập lục phân (Hex)     : 0x{:X}", gia_tri_mau);
    println!("- Hệ bát phân (Octal)        : 0o{:o}", gia_tri_mau);

    // 6. Thông điệp khích lệ kết thúc chương
    println!("\nChúc mừng! Bạn đã biên dịch và thực thi thành công chương trình thứ hai!");
}
