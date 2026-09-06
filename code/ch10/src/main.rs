#![allow(dead_code, unused_variables, unused_imports)]
// File: src/main.rs
// Chương trình thực chiến làm chủ Enums, Option & So khớp mẫu (Pattern Matching)

// 1. Enum biểu diễn các trạng thái đa dạng của một đơn hàng trực tuyến
// Mỗi nhánh có thể cõng theo những thông tin hoàn toàn khác nhau!
enum StateDonQueue {
    ChoThanhToan,
    DangDongGoi { store_export_queue: String },
    DangVanChuyen { ma_van_don: String, ten_tai_xe: String },
    GiaoThanhCong { recipient: String, time_time_recv: String },
    DaHuy(String), // Cõng theo một chuỗi String chứa lý do hủy đơn
}

// 2. Hàm chia kẹo an toàn: Trả về Option<u32> để ngăn chặn lỗi chia cho 0
fn safe_divide(so_keo: u32, so_tre_em: u32) -> Option<u32> {
    if so_tre_em == 0 {
        // Không thể chia cho 0 em bé: Trả về None báo hiệu không có kết quả
        None
    } else {
        // Chia thành công: Bọc kết quả vào trong hộp Some
        Some(so_keo / so_tre_em)
    }
}

// 3. Hàm xử lý trạng thái đơn hàng bằng cấu trúc so khớp mẫu 'match' toàn diện
fn update_process(don_hang: &StateDonQueue) {
    println!("------------------------------------------------------------");
    match don_hang {
        StateDonQueue::ChoThanhToan => {
            println!("[TRẠNG THÁI] Đơn hàng đang chờ khách thanh toán qua thẻ...");
        }
        StateDonQueue::DangDongGoi { store_export_queue } => {
            println!("[TRẠNG THÁI] Đơn hàng đang được đóng gói tại kho: {}", store_export_queue);
        }
        // Bóc tách cả 2 trường dữ liệu từ nhánh DangVanChuyen
        StateDonQueue::DangVanChuyen { ma_van_don, ten_tai_xe } => {
            println!("[VẬN CHUYỂN] Đơn đang trên đường giao!");
            println!("  + Mã vận đơn : {}", ma_van_don);
            println!("  + Shipper    : {}", ten_tai_xe);
        }
        StateDonQueue::GiaoThanhCong { recipient, time_time_recv } => {
            println!("[THÀNH CÔNG] Đơn hàng đã giao thành công!");
            println!("  + Người ký nhận: {}", recipient);
            println!("  + Thời điểm    : {}", time_time_recv);
        }
        StateDonQueue::DaHuy(ly_do) => {
            println!("[HỦY BỎ] Đơn hàng đã bị hủy. Lý do ghi nhận: '{}'", ly_do);
        }
    }
}

fn main() {
    println!("============================================================");
    println!("    HỆ THỐNG QUẢN LÝ ĐƠN HÀNG & MÔ HÌNH DỮ LIỆU AN TOÀN     ");
    println!("============================================================");

    // --- PHẦN 1: SO KHỚP MẪU VỚI ENUM CHỨA DỮ LIỆU ---
    let don_cho = StateDonQueue::ChoThanhToan;
    let don_dong_goi = StateDonQueue::DangDongGoi {
        store_export_queue: String::from("Kho Tổng Cầu Giấy, Hà Nội"),
    };
    let don_van_transfer = StateDonQueue::DangVanChuyen {
        ma_van_don: String::from("SPX-987654321"),
        ten_tai_xe: String::from("Bác Ba Giao Hàng"),
    };
    let order_delivered = StateDonQueue::GiaoThanhCong {
        recipient: String::from("Trần Thị Bình"),
        time_time_recv: String::from("14:30 ngày 05/09/2026"),
    };
    let don_cancel = StateDonQueue::DaHuy(String::from("Khách hàng đổi ý muốn chọn màu khác"));

    update_process(&don_cho);
    update_process(&don_dong_goi);
    update_process(&don_van_transfer);
    update_process(&order_delivered);
    update_process(&don_cancel);

    // --- PHẦN 2: LÀM VIỆC VỚI OPTION<T> VÀ TRIỆT TIÊU NULL ---
    println!("\n=== KIỂM THỬ TÍNH TOÁN AN TOÀN VỚI OPTION ===");
    let result_hop_le = safe_divide(20, 4);
    let result_error = safe_divide(20, 0);

    // Dùng match để mở hộp quà Option
    match result_hop_le {
        Some(keo) => println!("- Chia 20 kẹo cho 4 bé: Mỗi bé được {} cái kẹo.", keo),
        None => println!("- Lỗi: Số trẻ em không thể bằng 0!"),
    }

    match result_error {
        Some(keo) => println!("- Mỗi bé được: {} cái kẹo.", keo),
        None => println!("- [Được bảo vệ an toàn] Không thể chia cho 0 bé! Hệ thống không bị sập!"),
    }

    // --- PHẦN 3: MATCH GUARDS (ĐIỀU KIỆN BẢO VỆ PHỤ) VÀ KHOẢNG GIÁ TRỊ ---
    println!("\n=== PHÂN LOẠI TUỔI KHÁCH HÀNG VỚI MATCH GUARDS ===");
    let age = 17;
    let co_the_can_cuoc = true;

    match age {
        0..=12 => println!("Khách hàng thuộc lứa tuổi Thiếu nhi"),
        13..=17 if co_the_can_cuoc => println!("Lứa tuổi vị thành niên (ĐÃ có thẻ CCCD hợp lệ)"),
        13..=17 => println!("Lứa tuổi vị thành niên (chưa làm thẻ CCCD)"),
        18..=60 => println!("Khách hàng trong độ tuổi lao động trưởng thành"),
        _ => println!("Khách hàng cao tuổi ưu tiên"),
    }

    // --- PHẦN 4: CÚ PHÁP RÚT GỌN 'if let' ---
    println!("\n=== DÙNG 'if let' KHI CHỈ QUAN TÂM 1 TRƯỜNG HỢP ===");
    let info_recv_send_to: Option<&str> = Some("Xin chào, bạn có nhà không?");

    // Thay vì viết match dài dòng với cả nhánh None, ta chỉ bắt nhánh Some:
    if let Some(content) = info_recv_send_to {
        println!("Tin nhắn mới nhận được: '{}'", content);
    }
}
