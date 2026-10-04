// File: src/main.rs
// Chương trình thực chiến làm chủ Enums, Option & So khớp mẫu (Pattern Matching)

// 1. Enum biểu diễn các trạng thái đa dạng của một đơn hàng trực tuyến
// Mỗi nhánh có thể cõng theo những thông tin hoàn toàn khác nhau!
enum OrderStatus {
    AwaitingPayment,
    Packing {
        warehouse: String,
    },
    InTransit {
        tracking_code: String,
        driver_name: String,
    },
    Delivered {
        recipient: String,
        received_at: String,
    },
    Cancelled(String), // Cõng theo một chuỗi String chứa lý do hủy đơn
}

// 2. Hàm chia kẹo an toàn: Trả về Option<u32> để ngăn chặn lỗi chia cho 0
// (Thư viện chuẩn đã có sẵn `candies.checked_div(children)` làm đúng việc này;
// ở đây ta tự viết tay để thấy rõ Option hoạt động thế nào, nên tắt lint gợi ý của Clippy.)
#[allow(clippy::manual_checked_ops)]
fn safe_divide(candies: u32, children: u32) -> Option<u32> {
    if children == 0 {
        // Không thể chia cho 0 em bé: Trả về None báo hiệu không có kết quả
        None
    } else {
        // Chia thành công: Bọc kết quả vào trong hộp Some
        Some(candies / children)
    }
}

// 3. Hàm xử lý trạng thái đơn hàng bằng cấu trúc so khớp mẫu 'match' toàn diện
fn update_progress(order: &OrderStatus) {
    println!("------------------------------------------------------------");
    match order {
        OrderStatus::AwaitingPayment => {
            println!("[TRẠNG THÁI] Đơn hàng đang chờ khách thanh toán qua thẻ...");
        }
        OrderStatus::Packing { warehouse } => {
            println!(
                "[TRẠNG THÁI] Đơn hàng đang được đóng gói tại kho: {}",
                warehouse
            );
        }
        // Bóc tách cả 2 trường dữ liệu từ nhánh InTransit
        OrderStatus::InTransit {
            tracking_code,
            driver_name,
        } => {
            println!("[VẬN CHUYỂN] Đơn đang trên đường giao!");
            println!("  + Mã vận đơn : {}", tracking_code);
            println!("  + Shipper    : {}", driver_name);
        }
        OrderStatus::Delivered {
            recipient,
            received_at,
        } => {
            println!("[THÀNH CÔNG] Đơn hàng đã giao thành công!");
            println!("  + Người ký nhận: {}", recipient);
            println!("  + Thời điểm    : {}", received_at);
        }
        OrderStatus::Cancelled(reason) => {
            println!("[HỦY BỎ] Đơn hàng đã bị hủy. Lý do ghi nhận: '{}'", reason);
        }
    }
}

fn main() {
    println!("============================================================");
    println!("    HỆ THỐNG QUẢN LÝ ĐƠN HÀNG & MÔ HÌNH DỮ LIỆU AN TOÀN     ");
    println!("============================================================");

    // --- PHẦN 1: SO KHỚP MẪU VỚI ENUM CHỨA DỮ LIỆU ---
    let awaiting = OrderStatus::AwaitingPayment;
    let packing = OrderStatus::Packing {
        warehouse: String::from("Kho Tổng Cầu Giấy, Hà Nội"),
    };
    let in_transit = OrderStatus::InTransit {
        tracking_code: String::from("SPX-987654321"),
        driver_name: String::from("Bác Ba Giao Hàng"),
    };
    let delivered = OrderStatus::Delivered {
        recipient: String::from("Trần Thị Bình"),
        received_at: String::from("14:30 ngày 05/09/2026"),
    };
    let cancelled = OrderStatus::Cancelled(String::from("Khách hàng đổi ý muốn chọn màu khác"));

    update_progress(&awaiting);
    update_progress(&packing);
    update_progress(&in_transit);
    update_progress(&delivered);
    update_progress(&cancelled);

    // --- PHẦN 2: LÀM VIỆC VỚI OPTION<T> VÀ TRIỆT TIÊU NULL ---
    println!("\n=== KIỂM THỬ TÍNH TOÁN AN TOÀN VỚI OPTION ===");
    let valid_result = safe_divide(20, 4);
    let zero_result = safe_divide(20, 0);

    // Dùng match để mở hộp quà Option
    match valid_result {
        Some(each) => println!("- Chia 20 kẹo cho 4 bé: Mỗi bé được {} cái kẹo.", each),
        None => println!("- Lỗi: Số trẻ em không thể bằng 0!"),
    }

    match zero_result {
        Some(each) => println!("- Mỗi bé được: {} cái kẹo.", each),
        None => println!("- [Được bảo vệ an toàn] Không thể chia cho 0 bé! Hệ thống không bị sập!"),
    }

    // --- PHẦN 3: MATCH GUARDS (ĐIỀU KIỆN BẢO VỆ PHỤ) VÀ KHOẢNG GIÁ TRỊ ---
    println!("\n=== PHÂN LOẠI TUỔI KHÁCH HÀNG VỚI MATCH GUARDS ===");
    let age = 17;
    let has_id_card = true;

    match age {
        0..=12 => println!("Khách hàng thuộc lứa tuổi Thiếu nhi"),
        13..=17 if has_id_card => println!("Lứa tuổi vị thành niên (ĐÃ có thẻ CCCD hợp lệ)"),
        13..=17 => println!("Lứa tuổi vị thành niên (chưa làm thẻ CCCD)"),
        18..=60 => println!("Khách hàng trong độ tuổi lao động trưởng thành"),
        _ => println!("Khách hàng cao tuổi ưu tiên"),
    }

    // --- PHẦN 4: CÚ PHÁP RÚT GỌN 'if let' ---
    println!("\n=== DÙNG 'if let' KHI CHỈ QUAN TÂM 1 TRƯỜNG HỢP ===");
    let incoming_message: Option<&str> = Some("Xin chào, bạn có nhà không?");

    // Thay vì viết match dài dòng với cả nhánh None, ta chỉ bắt nhánh Some:
    if let Some(content) = incoming_message {
        println!("Tin nhắn mới nhận được: '{}'", content);
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn safe_divide_handles_zero() {
        assert_eq!(safe_divide(20, 4), Some(5));
        assert_eq!(safe_divide(20, 0), None);
        // Cùng hành vi với hàm có sẵn của thư viện chuẩn:
        assert_eq!(safe_divide(7, 2), 7u32.checked_div(2));
    }
}
