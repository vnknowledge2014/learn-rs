// File: src/main.rs
// Ứng dụng thực chiến làm chủ Vòng đời (Lifetimes) trong Rust

// 1. Hàm so sánh hai chuỗi và trả về chuỗi dài hơn
// Ký hiệu <'a> tuyên bố: Chuỗi trả về có vòng đời an toàn bằng khoảng giao nhau giữa x và y
fn pick_longer_message<'a>(x: &'a str, y: &'a str) -> &'a str {
    if x.len() > y.len() { x } else { y }
}

// 2. Struct nắm giữ tham chiếu mượn dữ liệu nguồn (&'a str)
// Giúp đọc và trích xuất cấu hình mà KHÔNG tốn dù chỉ 1 byte để sao chép chuỗi mới trên Heap!
struct SystemConfig<'a> {
    app_name: &'a str,
    monthly_fee: f64,
}

impl<'a> SystemConfig<'a> {
    // Phương thức đọc: Tận dụng Quy tắc suy luận ngầm số 3 (Lifetime Elision)
    // Không cần viết 'a ở kiểu trả về vì Rust tự lấy vòng đời của &self!
    fn name(&self) -> &str {
        self.app_name
    }

    fn print_info(&self) {
        println!(
            "- Ứng dụng: '{}' | Phí duy trì: {:.2} USD/tháng",
            self.app_name, self.monthly_fee
        );
    }
}

fn main() {
    println!("============================================================");
    println!("      BỘ PHÂN TÍCH CẤU HÌNH SIÊU TỐC - ZERO-COPY PARSER     ");
    println!("============================================================");

    // --- PHẦN 1: HÀM CÓ CHÚ THÍCH VÒNG ĐỜI 'a ---
    println!("\n1. So sánh hai thông điệp có vòng đời hợp lệ:");
    let message_1 = String::from("Hệ thống khởi động thành công");
    let message_2 = String::from("Cảnh báo pin yếu");

    // Cả message_1 và message_2 đều đang sống trong cùng phạm vi main
    let longer_message = pick_longer_message(message_1.as_str(), message_2.as_str());
    println!("- Thông điệp dài hơn được chọn: '{}'", longer_message);

    // --- PHẦN 2: CHỨNG MINH TÍNH AN TOÀN TRƯỚC VÒNG ĐỜI NGẮN HƠN ---
    println!("\n2. Kiểm soát phạm vi sống lồng nhau an toàn:");
    let parent_string = String::from("Dữ liệu bền vững của công ty");
    {
        let child_string = String::from("Dữ liệu tạm");
        let temp_result = pick_longer_message(parent_string.as_str(), child_string.as_str());
        println!(
            "- [Bên trong phạm vi con]: Kết quả chọn là: '{}'",
            temp_result
        );
        // temp_result chỉ được phép dùng bên trong dấu ngoặc nhọn này!
        // Nếu cố tình mang temp_result ra ngoài phạm vi con, compiler sẽ chặn đứng ngay!
    }

    // --- PHẦN 3: STRUCT CHỨA THAM CHIẾU (ZERO-COPY) ---
    println!("\n3. Khởi tạo Struct chứa tham chiếu mượn không tốn RAM:");
    let config_file = String::from("AppName: RustCloudServer, Fee: 49.99");

    // Lát cắt trích xuất tên ứng dụng trực tiếp từ chuỗi nguồn (byte 9 đến trước 24):
    let app_name_slice = &config_file[9..24];

    let config = SystemConfig {
        app_name: app_name_slice,
        monthly_fee: 49.99,
    };

    config.print_info();
    println!("- Tên ứng dụng trích xuất qua getter: '{}'", config.name());

    // --- PHẦN 4: VÒNG ĐỜI VĨNH CỬU 'static ---
    println!("\n4. Sử dụng hằng số có vòng đời vĩnh cửu ('static):");
    let eternal_message: &'static str = "PHẦN MỀM ĐÃ ĐƯỢC CHỨNG NHẬN AN TOÀN TUYỆT ĐỐI";
    println!("- Dòng chữ trên bia đá vĩnh cửu: '{}'", eternal_message);
}
