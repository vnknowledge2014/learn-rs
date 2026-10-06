// File: src/main.rs
// Chương trình tính toán sức khỏe BMI và minh họa Stack vs Heap

use std::io; // Nhập khẩu module Nhập/Xuất chuẩn của Rust

// 1. Hàm thuần túy: Toàn bộ tham số và kết quả đều nằm gọn trên STACK (kích thước f32 cố định)
fn compute_bmi(weight_kg: f32, height_m: f32) -> f32 {
    // Biểu thức tính toán trả về kết quả ngầm định (không cần từ khóa return hay dấu chấm phẩy)
    weight_kg / (height_m * height_m)
}

// 2. Hàm phân tích trạng thái thể lực: Trả về một chuỗi ký tự cố định (&'static str)
fn body_status(bmi: f32) -> &'static str {
    // Ngưỡng theo WHO: < 18.5 | 18.5–< 25 | 25–< 30 | ≥ 30
    if bmi < 18.5 {
        "Thiếu cân (cần bồi dưỡng thêm dinh dưỡng)"
    } else if bmi < 25.0 {
        "Thể trạng lý tưởng (rất cân đối, chúc mừng bạn!)"
    } else if bmi < 30.0 {
        "Thừa cân nhẹ (nên tăng cường vận động thể thao)"
    } else {
        "Béo phì (cần điều chỉnh chế độ ăn uống và tập luyện)"
    }
}

// 3. Hàm hỗ trợ đọc một dòng văn bản từ bàn phím và chuyển thành số thực
// Dùng #[allow(dead_code)] để hàm main có thể chạy mượt mà với dữ liệu mẫu tĩnh trong các môi trường kiểm thử tự động,
// đồng thời người học vẫn có thể gọi hàm này khi thực hành tương tác trên máy tính cá nhân.
#[allow(dead_code)]
fn read_f32(prompt: &str) -> f32 {
    println!("{}", prompt);

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

    // Dùng dữ liệu mẫu cố định để chương trình chạy được cả khi không có người gõ
    // (ví dụ trong CI). Muốn nhập thật từ bàn phím, thay bằng:
    //     let weight_kg = read_f32("Nhập cân nặng (kg):");
    let weight_kg = 68.5; // Đơn vị: kg
    let height_m = 1.72; // Đơn vị: mét

    println!("Thông số kiểm tra thể lực mẫu:");
    println!("- Cân nặng : {} kg (lưu trữ trên Stack)", weight_kg);
    println!("- Chiều cao: {} m  (lưu trữ trên Stack)", height_m);

    // Gọi hàm tính toán BMI
    let bmi = compute_bmi(weight_kg, height_m);
    let advice = body_status(bmi);

    println!("------------------------------------------------------------");
    println!("Chỉ số BMI của bạn : {:.2}", bmi);
    println!("Kết luận thể trạng : {}", advice);
    println!("------------------------------------------------------------");

    // Khám phá kích thước của đối tượng String (Stack 24 bytes vs Heap)
    let description = String::from("Báo cáo sức khỏe cá nhân năm 2026");
    println!("Kiểm tra ô nhớ của chuỗi mô tả:");
    println!(
        "- Kích thước thẻ quản lý trên STACK: {} bytes",
        std::mem::size_of_val(&description)
    );
    println!(
        "- Độ dài chuỗi nội dung trên HEAP  : {} bytes",
        description.len()
    );
    println!(
        "- Sức chứa bãi đỗ xe đã cấp phát   : {} bytes",
        description.capacity()
    );
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn bmi_formula() {
        // 68.5 / (1.72 * 1.72) ≈ 23.15
        let bmi = compute_bmi(68.5, 1.72);
        assert!((bmi - 23.15).abs() < 0.01);
    }

    #[test]
    fn body_status_thresholds_have_no_gaps() {
        // Các mốc 24.95 và 29.95 từng rơi nhầm nhóm khi ngưỡng viết là 24.9 / 29.9.
        assert!(body_status(18.4).starts_with("Thiếu cân"));
        assert!(body_status(18.5).starts_with("Thể trạng lý tưởng"));
        assert!(body_status(24.95).starts_with("Thể trạng lý tưởng"));
        assert!(body_status(25.0).starts_with("Thừa cân"));
        assert!(body_status(29.95).starts_with("Thừa cân"));
        assert!(body_status(30.0).starts_with("Béo phì"));
    }
}
