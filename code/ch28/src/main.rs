#![allow(dead_code, unused_variables, unused_imports)]
use std::collections::VecDeque;

/// ỨNG DỤNG 1 CỦA STACK: Kiểm tra dấu ngoặc hợp lệ
/// Thuật toán sử dụng Ngăn xếp (LIFO):
/// - Gặp dấu mở '(', '[', '{': Đẩy vào đỉnh ngăn xếp.
/// - Gặp dấu đóng ')', ']', '}': Rút phần tử trên đỉnh ra so khớp.
///   Nếu không khớp hoặc ngăn xếp rỗng -> Biểu thức sai cú pháp!
/// - Kết thúc chuỗi, nếu ngăn xếp rỗng -> Biểu thức hợp lệ.
pub fn is_balanced_brackets(bieu_thuc: &str) -> bool {
    let mut stack: Vec<char> = Vec::new();

    for ky_tu in bieu_thuc.chars() {
        match ky_tu {
            '(' | '[' | '{' => {
                stack.push(ky_tu);
            }
            ')' => {
                if stack.pop() != Some('(') {
                    return false;
                }
            }
            ']' => {
                if stack.pop() != Some('[') {
                    return false;
                }
            }
            '}' => {
                if stack.pop() != Some('{') {
                    return false;
                }
            }
            // Bỏ qua các ký tự chữ cái, số, hoặc khoảng trắng
            _ => {}
        }
    }

    // Biểu thức chỉ đúng khi mọi dấu ngoặc mở đều đã được đóng khớp hết
    stack.is_empty()
}

/// Mô hình Đơn hàng trong hệ thống thương mại điện tử
#[derive(Debug, PartialEq, Clone)]
pub struct DonQueue {
    pub order_code: u32,
    pub customer_name: String,
    pub tong_tien: f64,
}

/// ỨNG DỤNG 2 CỦA QUEUE: Hệ thống quản lý hàng đợi đơn hàng chuẩn FIFO
pub struct QueueDonQueue {
    list: VecDeque<DonQueue>,
}

impl QueueDonQueue {
    pub fn new() -> Self {
        Self {
            list: VecDeque::new(),
        }
    }

    /// Khách đặt hàng: Xếp vào cuối hàng đợi - O(1)
    pub fn them_don(&mut self, don: DonQueue) {
        self.list.push_back(don);
    }

    /// Đơn hàng VIP (Ưu tiên khẩn cấp): Chèn thẳng vào đầu hàng đợi - O(1)
    pub fn them_don_vip(&mut self, don: DonQueue) {
        self.list.push_front(don);
    }

    /// Nhà bếp / Kho xuất hàng: Phục vụ đơn đến trước - O(1)
    pub fn handle_don_ke_cont(&mut self) -> Option<DonQueue> {
        self.list.pop_front()
    }

    /// Xem trước đơn sắp được phục vụ mà không xóa khỏi hàng đợi
    pub fn first_view_don(&self) -> Option<&DonQueue> {
        self.list.front()
    }

    pub fn so_don_dang_cho(&self) -> usize {
        self.list.len()
    }
}

impl Default for QueueDonQueue {
    fn default() -> Self {
        Self::new()
    }
}

fn main() {
    println!("============================================================");
    println!("   ỨNG DỤNG THỰC CHIẾN CỦA NGĂN XẾP (STACK) & HÀNG ĐỢI (QUEUE)");
    println!("============================================================");

    // 1. Kiểm thử thuật toán kiểm tra dấu ngoặc với Stack
    println!("[1] Kiểm tra tính hợp lệ của biểu thức toán học:");
    let bieu_thuc_1 = "{ a + [ b * ( c + d ) ] }";
    let bieu_thuc_2 = "( a + b ]";
    let bieu_thuc_3 = "{ [ ( ] ) }"; // Đóng sai thứ tự lồng nhau

    println!("    - Biểu thức 1 '{}': {}", bieu_thuc_1, is_balanced_brackets(bieu_thuc_1));
    println!("    - Biểu thức 2 '{}': {}", bieu_thuc_2, is_balanced_brackets(bieu_thuc_2));
    println!("    - Biểu thức 3 '{}': {}", bieu_thuc_3, is_balanced_brackets(bieu_thuc_3));

    assert!(is_balanced_brackets(bieu_thuc_1));
    assert!(!is_balanced_brackets(bieu_thuc_2));
    assert!(!is_balanced_brackets(bieu_thuc_3));

    // 2. Kiểm thử Hệ thống Hàng đợi đơn hàng với VecDeque
    println!("\n[2] Vận hành hệ thống xử lý đơn hàng FIFO bằng VecDeque:");
    let mut he_thong = QueueDonQueue::new();

    // Khách hàng thông thường đặt hàng lần lượt
    he_thong.them_don(DonQueue {
        order_code: 101,
        customer_name: String::from("Nguyễn Văn A"),
        tong_tien: 150.0,
    });
    he_thong.them_don(DonQueue {
        order_code: 102,
        customer_name: String::from("Trần Thị B"),
        tong_tien: 80.0,
    });

    println!("    - Đã nhận 2 đơn hàng thông thường. Số đơn chờ: {}", he_thong.so_don_dang_cho());

    // Đơn hàng hỏa tốc VIP xuất hiện! Đưa thẳng vào đầu hàng đợi
    he_thong.them_don_vip(DonQueue {
        order_code: 999,
        customer_name: String::from("Khách VIP Kim Cương"),
        tong_tien: 500.0,
    });
    println!("    - Nhận đơn hỏa tốc VIP 999 (chen lên đầu hàng)!");

    // Xem trước đơn hàng kế tiếp
    if let Some(don_dau) = he_thong.first_view_don() {
        println!("    - Đơn hàng chuẩn bị xử lý tiếp theo là: Mã #{} ({})", don_dau.order_code, don_dau.customer_name);
        assert_eq!(don_dau.order_code, 999);
    }

    // Tiến hành xuất kho lần lượt theo đúng thứ tự ưu tiên
    println!("\n    Bắt đầu xuất kho theo thứ tự FIFO:");
    let mut handles = Vec::new();
    while let Some(don) = he_thong.handle_don_ke_cont() {
        println!("    -> Đang đóng gói đơn #{}: Khách {} - {:.2}k", don.order_code, don.customer_name, don.tong_tien);
        handles.push(don.order_code);
    }

    // Xác nhận thứ tự xử lý: Đơn VIP 999 trước, sau đó là 101, rồi đến 102
    assert_eq!(handles, vec![999, 101, 102]);
    assert_eq!(he_thong.so_don_dang_cho(), 0);
    println!("    => Toàn bộ hàng đợi đã được xử lý sạch sẽ!");

    println!("============================================================");
    println!("               HOÀN TẤT THỰC NGHIỆM CHƯƠNG 24               ");
    println!("============================================================");
}


#[cfg(test)]
mod tests {
    use super::*;

    fn don(id: u32, name: &str) -> DonQueue {
        DonQueue { order_code: id, customer_name: name.into(), tong_tien: 100.0 }
    }

    #[test]
    fn bracket_matching() {
        assert!(is_balanced_brackets("(a[b]{c})"));
        assert!(is_balanced_brackets(""));
        assert!(!is_balanced_brackets("(a]"));
        assert!(!is_balanced_brackets("((("));
        assert!(!is_balanced_brackets(")("));
    }

    #[test]
    fn fifo_queue_and_vip_priority() {
        let mut hd = QueueDonQueue::new();
        hd.them_don(don(1, "A"));
        hd.them_don(don(2, "B"));
        hd.them_don_vip(don(9, "VIP")); // chen lên đầu
        assert_eq!(hd.so_don_dang_cho(), 3);
        assert_eq!(hd.first_view_don().map(|d| d.order_code), Some(9));

        // VIP ra trước, phần còn lại giữ đúng thứ tự FIFO
        assert_eq!(hd.handle_don_ke_cont().map(|d| d.order_code), Some(9));
        assert_eq!(hd.handle_don_ke_cont().map(|d| d.order_code), Some(1));
        assert_eq!(hd.handle_don_ke_cont().map(|d| d.order_code), Some(2));
        assert_eq!(hd.handle_don_ke_cont().map(|d| d.order_code), None);
    }
}
