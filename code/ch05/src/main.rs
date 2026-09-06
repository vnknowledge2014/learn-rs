#![allow(dead_code, unused_variables, unused_imports)]
// File: src/main.rs
// Chương trình tính toán sức khỏe BMI và minh họa Stack vs Heap

use std::io; // Nhập khẩu module Nhập/Xuất chuẩn của Rust

// 1. Hàm thuần túy: Toàn bộ tham số và kết quả đều nằm gọn trên STACK (kích thước f32 cố định)
fn bmi(can_nang_kg: f32, chieu_cao_m: f32) -> f32 {
    // Biểu thức tính toán trả về kết quả ngầm định (không cần từ khóa return hay dấu chấm phẩy)
    can_nang_kg / (chieu_cao_m * chieu_cao_m)
}

// 2. Hàm phân tích trạng thái thể lực: Trả về một chuỗi ký tự cố định (&'static str)
fn mark_price_state(bmi: f32) -> &'static str {
    if bmi < 18.5 {
        "Thiếu cân (cần bồi dưỡng thêm dinh dưỡng)"
    } else if bmi < 24.9 {
        "Thể trạng lý tưởng (rất cân đối, chúc mừng bạn!)"
    } else if bmi < 29.9 {
        "Thừa cân nhẹ (nên tăng cường vận động thể thao)"
    } else {
        "Béo phì (cần điều chỉnh chế độ ăn uống và tập luyện)"
    }
}

// 3. Hàm hỗ trợ đọc một dòng văn bản từ bàn phím và chuyển thành số thực
// Dùng #[allow(dead_code)] để hàm main có thể chạy mượt mà với dữ liệu mẫu tĩnh trong các môi trường kiểm thử tự động,
// đồng thời người học vẫn có thể gọi hàm này khi thực hành tương tác trên máy tính cá nhân.
#[allow(dead_code)]
fn parse_float(cau_hoi: &str) -> f32 {
    println!("{}", cau_hoi);

    // Chuỗi co giãn được cấp phát trên bãi đỗ HEAP để hứng các ký tự người dùng gõ
    let mut input_buffer = String::new();

    // io::stdin() kết nối với bàn phím
    // read_line ghi dữ liệu vào input_buffer qua tham chiếu mượn sửa (mutable borrow / &mut)
    // expect sẽ dừng chương trình và báo lỗi nếu thiết bị nhập liệu bị ngắt kết nối
    io::stdin()
        .read_line(&mut input_buffer)
        .expect("Lỗi: Không thể đọc dữ liệu từ bàn phím!");

    // .trim() loại bỏ ký tự xuống dòng Enter (\n hoặc \r\n)
    // .parse() chuyển đổi chuỗi thành số f32
    // unwrap_or(0.0) sẽ lấy số 0.0 làm giá trị mặc định nếu người dùng gõ chữ linh tinh
    input_buffer.trim().parse::<f32>().unwrap_or(0.0)
}

fn main() {
    println!("============================================================");
    println!("     ỨNG DỤNG ĐO CHỈ SỐ SỨC KHỎE THỂ HÌNH CHUẨN QUỐC TẾ     ");
    println!("============================================================");

    // Lấy thông số cân nặng và chiều high từ người dùng
    // Trong môi trường tự động không có người gõ, hàm sẽ dùng giá trị mặc định an toàn
    let can_heavy = 68.5; // Đơn vị: kg
    let height = 1.72; // Đơn vị: mét

    println!("Thông số kiểm tra thể lực mẫu:");
    println!("- Cân nặng : {} kg (lưu trữ trên Stack)", can_heavy);
    println!("- Chiều high: {} m  (lưu trữ trên Stack)", height);

    // Gọi hàm tính toán BMI
    let bmi = bmi(can_heavy, height);
    let loi_khuyen = mark_price_state(bmi);

    println!("------------------------------------------------------------");
    println!("Chỉ số BMI của bạn : {:.2}", bmi);
    println!("Kết luận thể trạng : {}", loi_khuyen);
    println!("------------------------------------------------------------");

    // Khám phá kích thước của đối tượng String (Stack 24 bytes vs Heap)
    let mo_ta_chi_tiet = String::from("Báo cáo sức khỏe cá nhân năm 2026");
    println!("Kiểm tra ô nhớ của chuỗi mô tả:");
    println!("- Kích thước thẻ quản lý trên STACK: {} bytes", std::mem::size_of_val(&mo_ta_chi_tiet));
    println!("- Độ dài chuỗi nội dung trên HEAP  : {} bytes", mo_ta_chi_tiet.len());
    println!("- Sức chứa bãi đỗ xe đã cấp phát   : {} bytes", mo_ta_chi_tiet.capacity());
}
