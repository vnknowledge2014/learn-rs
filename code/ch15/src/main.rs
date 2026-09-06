#![allow(dead_code, unused_variables, unused_imports)]
// Tệp: src/main.rs
// Chương trình thực chiến làm chủ Closures: Fn, FnMut, và FnOnce trong Rust

// ============================================================================
// CÁC HÀM NHẬN CLOSURE LÀM THAM SỐ VỚI RÀNG BUỘC TRAIT (TRAIT BOUNDS)
// ============================================================================

/// Hàm 1: Nhận closure thực hiện giao ước Fn (Chỉ đọc môi trường)
/// Có thể gọi closure này nhiều lần liên tiếp một cách an toàn tuyệt đối
pub fn exec_read<F>(ten_tac_vu: &str, hanh_dong: F)
where
    F: Fn(),
{
    println!("--- BẮT ĐẦU TÁC VỤ CHỈ ĐỌC: [{}] ---", ten_tac_vu);
    hanh_dong(); // Gọi lần 1
    hanh_dong(); // Gọi lần 2
    println!("--- HOÀN THÀNH TÁC VỤ CHỈ ĐỌC ---");
}

/// Hàm 2: Nhận closure thực hiện giao ước FnMut (Sửa đổi môi trường)
/// Bắt buộc tham số hanh_dong phải mang từ khóa mut vì trạng thái nội bộ thay đổi
pub fn exec_swap<F>(ten_tac_vu: &str, mut hanh_dong: F, so_vong_lap: usize)
where
    F: FnMut(usize),
{
    println!("\n--- BẮT ĐẦU TÁC VỤ SỬA ĐỔI TRẠNG THÁI: [{}] ---", ten_tac_vu);
    for step in 1..=so_vong_lap {
        hanh_dong(step); // Gọi nhiều lần, mỗi lần biến nội bộ bên ngoài sẽ biến đổi
    }
    println!("--- HOÀN THÀNH TÁC VỤ SỬA ĐỔI TRẠNG THÁI ---");
}

/// Hàm 3: Nhận closure thực hiện giao ước FnOnce (Tiêu thụ tài nguyên)
/// Closure này tự hủy ngay sau khi được gọi vì quyền sở hữu đã bị đoạt lấy
pub fn exec_consume<F>(ten_tac_vu: &str, hanh_dong: F)
where
    F: FnOnce() -> String,
{
    println!("\n--- BẮT ĐẦU TÁC VỤ TIÊU THỤ MỘT LẦN: [{}] ---", ten_tac_vu);
    let ket_qua = hanh_dong(); // Gọi DUY NHẤT một lần tại đây
    // hanh_dong(); // Nếu bỏ dấu chú thích dòng này, rustc sẽ chặn ngay lập tức!
    println!("Kết quả nhận được sau khi tiêu thụ: {}", ket_qua);
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
    let thong_tin_he_thong = String::from("Máy chủ Cổng thanh toán (Gateway-01)");
    
    // Closure print_info chỉ mượn đọc thong_tin_he_thong
    let print_info = || {
        println!("[GIÁM SÁT] Trạng thái hiện tại của: {}", thong_tin_he_thong);
    };

    // Truyền closure vào hàm exec_read (chứng minh gọi được nhiều lần)
    exec_read("Kiểm tra sức khỏe định kỳ", print_info);
    // Biến thong_tin_he_thong vẫn hoàn toàn nguyên vẹn ở phạm vi ngoài:
    println!("Biến gốc bên ngoài vẫn truy cập bình thường: {}", thong_tin_he_thong);

    // ------------------------------------------------------------------------
    // TÌNH HUỐNG 2: Giao ước FnMut - Bắt giữ tham chiếu sửa đổi (&mut T)
    // ------------------------------------------------------------------------
    let mut total_amount_access_cap: usize = 0;
    let mut nhat_ky_hoat_dong: Vec<String> = Vec::new();

    // Closure tang_truy_cap mượn sửa đổi biến total_amount_access_cap và nhat_ky_hoat_dong
    let ghi_nhan_luot_xem = |lan_lap: usize| {
        total_amount_access_cap += 10;
        nhat_ky_hoat_dong.push(format!("Đợt ghi nhận #{}: +10 yêu cầu", lan_lap));
        println!("  -> Đang tích lũy... Tổng lưu lượng hiện tại: {}", total_amount_access_cap);
    };

    // Thực thi 3 vòng lặp tích lũy
    exec_swap("Bộ đếm lưu lượng mạng", ghi_nhan_luot_xem, 3);
    println!("Kết quả sau khi kết thúc FnMut:");
    println!("- Tổng lưu lượng cuối cùng: {}", total_amount_access_cap);
    println!("- Chi tiết nhật ký: {:?}", nhat_ky_hoat_dong);

    // ------------------------------------------------------------------------
    // TÌNH HUỐNG 3: Giao ước FnOnce - Đoạt quyền sở hữu (Move)
    // ------------------------------------------------------------------------
    // Giả lập một khóa bảo mật phiên đăng nhập chỉ dùng một lần (One-Time Token)
    let secret_token = String::from("SEC-TOKEN-XYZ-9999-SECRET");

    // Dùng từ khóa move để ép closure chiếm trọn quyền sở hữu của secret_token
    let huy_phien_lam_viec = move || {
        // Biến secret_token bị di chuyển vào đây và tiêu thụ
        let thong_report = format!("Khóa [{}] đã bị thu hồi vĩnh viễn.", secret_token);
        thong_report // Trả về chuỗi thông báo, secret_token bị Drop tại đây
    };

    exec_consume("Tiêu hủy phiên bảo mật", huy_phien_lam_viec);
    // println!("{}", secret_token); // LỖI: value borrowed here after move!

    // ------------------------------------------------------------------------
    // TÌNH HUỐNG 4: Lưu trữ danh sách Closure trong Vector với Box<dyn Fn()>
    // ------------------------------------------------------------------------
    println!("\n--- QUẢN LÝ DANH SÁCH BỘ ĐIỀU HƯỚNG VỚI BOX<DYN FN()> ---");
    let mut list_event: Vec<Box<dyn Fn()>> = Vec::new();

    list_event.push(Box::new(|| println!("Sự kiện A: Khởi động quạt làm mát")));
    list_event.push(Box::new(|| println!("Sự kiện B: Đèn LED chuyển màu xanh")));

    for (stt, event) in list_event.iter().enumerate() {
        print!("Kích hoạt sự kiện #{}: ", stt + 1);
        event(); // Gọi từng closure qua con trỏ Trait Object
    }

    println!("\n============================================================");
    println!("     HOÀN TẤT XÁC THỰC CƠ CHẾ BẮT GIỮ MÔI TRƯỜNG CỦA RUST   ");
    println!("============================================================");
}
