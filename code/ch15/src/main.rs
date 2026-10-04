// Tệp: src/main.rs
// Chương trình thực chiến làm chủ Closures: Fn, FnMut, và FnOnce trong Rust

// ============================================================================
// CÁC HÀM NHẬN CLOSURE LÀM THAM SỐ VỚI RÀNG BUỘC TRAIT (TRAIT BOUNDS)
// ============================================================================

/// Hàm 1: Nhận closure thực hiện giao ước Fn (Chỉ đọc môi trường)
/// Có thể gọi closure này nhiều lần liên tiếp một cách an toàn tuyệt đối
pub fn exec_read<F>(task_name: &str, action: F)
where
    F: Fn(),
{
    println!("--- BẮT ĐẦU TÁC VỤ CHỈ ĐỌC: [{}] ---", task_name);
    action(); // Gọi lần 1
    action(); // Gọi lần 2
    println!("--- HOÀN THÀNH TÁC VỤ CHỈ ĐỌC ---");
}

/// Hàm 2: Nhận closure thực hiện giao ước FnMut (Sửa đổi môi trường)
/// Bắt buộc tham số action phải mang từ khóa mut vì trạng thái nội bộ thay đổi
pub fn exec_mutate<F>(task_name: &str, mut action: F, iterations: usize)
where
    F: FnMut(usize),
{
    println!(
        "\n--- BẮT ĐẦU TÁC VỤ SỬA ĐỔI TRẠNG THÁI: [{}] ---",
        task_name
    );
    for step in 1..=iterations {
        action(step); // Gọi nhiều lần, mỗi lần biến nội bộ bên ngoài sẽ biến đổi
    }
    println!("--- HOÀN THÀNH TÁC VỤ SỬA ĐỔI TRẠNG THÁI ---");
}

/// Hàm 3: Nhận closure thực hiện giao ước FnOnce (Tiêu thụ tài nguyên)
/// Closure này tự hủy ngay sau khi được gọi vì quyền sở hữu đã bị đoạt lấy
pub fn exec_consume<F>(task_name: &str, action: F)
where
    F: FnOnce() -> String,
{
    println!("\n--- BẮT ĐẦU TÁC VỤ TIÊU THỤ MỘT LẦN: [{}] ---", task_name);
    let result = action(); // Gọi DUY NHẤT một lần tại đây
    // action(); // Nếu bỏ dấu chú thích dòng này, rustc sẽ chặn ngay lập tức!
    println!("Kết quả nhận được sau khi tiêu thụ: {}", result);
    println!("--- TÀI NGUYÊN ĐÃ ĐƯỢC GIẢI PHÓNG TOÀN DIỆN ---");
}

// ============================================================================
// CHƯƠNG TRÌNH ĐIỀU HÀNH CHÍNH
// ============================================================================

fn main() {
    println!("============================================================");
    println!("      HỆ THỐNG ĐIỀU PHỐI TÁC VỤ SỰ KIỆN: FN, FNMUT, FNONCE  ");
    println!("============================================================");

    // ------------------------------------------------------------------------
    // TÌNH HUỐNG 1: Giao ước Fn - Bắt giữ tham chiếu chỉ đọc (&T)
    // ------------------------------------------------------------------------
    let system_info = String::from("Máy chủ Cổng thanh toán (Gateway-01)");

    // Closure print_info chỉ mượn đọc system_info
    let print_info = || {
        println!("[GIÁM SÁT] Trạng thái hiện tại của: {}", system_info);
    };

    // Truyền closure vào hàm exec_read (chứng minh gọi được nhiều lần)
    exec_read("Kiểm tra sức khỏe định kỳ", print_info);
    // Biến system_info vẫn hoàn toàn nguyên vẹn ở phạm vi ngoài:
    println!(
        "Biến gốc bên ngoài vẫn truy cập bình thường: {}",
        system_info
    );

    // ------------------------------------------------------------------------
    // TÌNH HUỐNG 2: Giao ước FnMut - Bắt giữ tham chiếu sửa đổi (&mut T)
    // ------------------------------------------------------------------------
    let mut total_traffic: usize = 0;
    let mut activity_log: Vec<String> = Vec::new();

    // Closure record_view mượn sửa đổi biến total_traffic và activity_log
    let record_view = |round: usize| {
        total_traffic += 10;
        activity_log.push(format!("Đợt ghi nhận #{}: +10 yêu cầu", round));
        println!(
            "  -> Đang tích lũy... Tổng lưu lượng hiện tại: {}",
            total_traffic
        );
    };

    // Thực thi 3 vòng lặp tích lũy
    exec_mutate("Bộ đếm lưu lượng mạng", record_view, 3);
    println!("Kết quả sau khi kết thúc FnMut:");
    println!("- Tổng lưu lượng cuối cùng: {}", total_traffic);
    println!("- Chi tiết nhật ký: {:?}", activity_log);

    // ------------------------------------------------------------------------
    // TÌNH HUỐNG 3: Giao ước FnOnce - Đoạt quyền sở hữu (Move)
    // ------------------------------------------------------------------------
    // Giả lập một khóa bảo mật phiên đăng nhập chỉ dùng một lần (One-Time Token)
    let secret_token = String::from("SEC-TOKEN-XYZ-9999-SECRET");

    // Dùng từ khóa move để ép closure chiếm trọn quyền sở hữu của secret_token
    let end_session = move || {
        // Biến secret_token bị di chuyển vào đây và tiêu thụ
        let report = format!("Khóa [{}] đã bị thu hồi vĩnh viễn.", secret_token);
        report // Trả về chuỗi thông báo, secret_token bị Drop tại đây
    };

    exec_consume("Tiêu hủy phiên bảo mật", end_session);
    // println!("{}", secret_token); // LỖI E0382: borrow of moved value!

    // ------------------------------------------------------------------------
    // TÌNH HUỐNG 4: Lưu trữ danh sách Closure trong Vector với Box<dyn Fn()>
    // ------------------------------------------------------------------------
    println!("\n--- QUẢN LÝ DANH SÁCH BỘ ĐIỀU HƯỚNG VỚI BOX<DYN FN()> ---");
    let mut events: Vec<Box<dyn Fn()>> = Vec::new();

    events.push(Box::new(|| println!("Sự kiện A: Khởi động quạt làm mát")));
    events.push(Box::new(|| println!("Sự kiện B: Đèn LED chuyển màu xanh")));

    for (index, event) in events.iter().enumerate() {
        print!("Kích hoạt sự kiện #{}: ", index + 1);
        event(); // Gọi từng closure qua con trỏ Trait Object
    }

    println!("\n============================================================");
    println!("     HOÀN TẤT XÁC THỰC CƠ CHẾ BẮT GIỮ MÔI TRƯỜNG CỦA RUST   ");
    println!("============================================================");
}
