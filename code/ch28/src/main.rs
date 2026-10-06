#![allow(dead_code, unused_variables, unused_imports)]
use std::collections::VecDeque;

/// ỨNG DỤNG 1 CỦA STACK: Kiểm tra dấu ngoặc hợp lệ
/// Thuật toán sử dụng Ngăn xếp (LIFO):
/// - Gặp dấu mở '(', '[', '{': Đẩy vào đỉnh ngăn xếp.
/// - Gặp dấu đóng ')', ']', '}': Rút phần tử trên đỉnh ra so khớp.
///   Nếu không khớp hoặc ngăn xếp rỗng -> Biểu thức sai cú pháp!
/// - Kết thúc chuỗi, nếu ngăn xếp rỗng -> Biểu thức hợp lệ.
pub fn is_balanced_brackets(expr: &str) -> bool {
    let mut stack: Vec<char> = Vec::new();

    for ch in expr.chars() {
        match ch {
            '(' | '[' | '{' => {
                stack.push(ch);
            }
            ')' | ']' | '}' => {
                // Dấu đóng phải khớp với dấu mở nằm trên đỉnh ngăn xếp
                let expected = match ch {
                    ')' => '(',
                    ']' => '[',
                    _ => '{',
                };
                if stack.pop() != Some(expected) {
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
pub struct Order {
    pub order_code: u32,
    pub customer_name: String,
    pub total_amount: f64,
}

/// ỨNG DỤNG 2 CỦA QUEUE: Hệ thống quản lý hàng đợi đơn hàng chuẩn FIFO
pub struct OrderQueue {
    list: VecDeque<Order>,
}

impl OrderQueue {
    pub fn new() -> Self {
        Self {
            list: VecDeque::new(),
        }
    }

    /// Khách đặt hàng: Xếp vào cuối hàng đợi - O(1)
    pub fn add_order(&mut self, order: Order) {
        self.list.push_back(order);
    }

    /// Đơn hàng VIP (Ưu tiên khẩn cấp): Chèn thẳng vào đầu hàng đợi - O(1)
    pub fn add_vip_order(&mut self, order: Order) {
        self.list.push_front(order);
    }

    /// Nhà bếp / Kho xuất hàng: Phục vụ đơn đến trước - O(1)
    pub fn process_next_order(&mut self) -> Option<Order> {
        self.list.pop_front()
    }

    /// Xem trước đơn sắp được phục vụ mà không xóa khỏi hàng đợi
    pub fn peek_next_order(&self) -> Option<&Order> {
        self.list.front()
    }

    pub fn pending_count(&self) -> usize {
        self.list.len()
    }
}

impl Default for OrderQueue {
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
    let expr_1 = "{ a + [ b * ( c + d ) ] }";
    let expr_2 = "( a + b ]";
    let expr_3 = "{ [ ( ] ) }"; // Đóng sai thứ tự lồng nhau

    println!(
        "    - Biểu thức 1 '{}': {}",
        expr_1,
        is_balanced_brackets(expr_1)
    );
    println!(
        "    - Biểu thức 2 '{}': {}",
        expr_2,
        is_balanced_brackets(expr_2)
    );
    println!(
        "    - Biểu thức 3 '{}': {}",
        expr_3,
        is_balanced_brackets(expr_3)
    );

    assert!(is_balanced_brackets(expr_1));
    assert!(!is_balanced_brackets(expr_2));
    assert!(!is_balanced_brackets(expr_3));

    // 2. Kiểm thử Hệ thống Hàng đợi đơn hàng với VecDeque
    println!("\n[2] Vận hành hệ thống xử lý đơn hàng FIFO bằng VecDeque:");
    let mut queue = OrderQueue::new();

    // Khách hàng thông thường đặt hàng lần lượt
    queue.add_order(Order {
        order_code: 101,
        customer_name: String::from("Nguyễn Văn A"),
        total_amount: 150.0,
    });
    queue.add_order(Order {
        order_code: 102,
        customer_name: String::from("Trần Thị B"),
        total_amount: 80.0,
    });

    println!(
        "    - Đã nhận 2 đơn hàng thông thường. Số đơn chờ: {}",
        queue.pending_count()
    );

    // Đơn hàng hỏa tốc VIP xuất hiện! Đưa thẳng vào đầu hàng đợi
    queue.add_vip_order(Order {
        order_code: 999,
        customer_name: String::from("Khách VIP Kim Cương"),
        total_amount: 500.0,
    });
    println!("    - Nhận đơn hỏa tốc VIP 999 (chen lên đầu hàng)!");

    // Xem trước đơn hàng kế tiếp
    if let Some(next_order) = queue.peek_next_order() {
        println!(
            "    - Đơn hàng chuẩn bị xử lý tiếp theo là: Mã #{} ({})",
            next_order.order_code, next_order.customer_name
        );
        assert_eq!(next_order.order_code, 999);
    }

    // Tiến hành xuất kho lần lượt theo đúng thứ tự ưu tiên
    println!("\n    Bắt đầu xuất kho theo thứ tự FIFO:");
    let mut processed = Vec::new();
    while let Some(order) = queue.process_next_order() {
        println!(
            "    -> Đang đóng gói đơn #{}: Khách {} - {:.2}k",
            order.order_code, order.customer_name, order.total_amount
        );
        processed.push(order.order_code);
    }

    // Xác nhận thứ tự xử lý: Đơn VIP 999 trước, sau đó là 101, rồi đến 102
    assert_eq!(processed, vec![999, 101, 102]);
    assert_eq!(queue.pending_count(), 0);
    println!("    => Toàn bộ hàng đợi đã được xử lý sạch sẽ!");

    println!("============================================================");
    println!("               HOÀN TẤT THỰC NGHIỆM CHƯƠNG 28               ");
    println!("============================================================");
}

#[cfg(test)]
mod tests {
    use super::*;

    fn order(id: u32, name: &str) -> Order {
        Order {
            order_code: id,
            customer_name: name.into(),
            total_amount: 100.0,
        }
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
        let mut queue = OrderQueue::new();
        queue.add_order(order(1, "A"));
        queue.add_order(order(2, "B"));
        queue.add_vip_order(order(9, "VIP")); // chen lên đầu
        assert_eq!(queue.pending_count(), 3);
        assert_eq!(queue.peek_next_order().map(|d| d.order_code), Some(9));

        // VIP ra trước, phần còn lại giữ đúng thứ tự FIFO
        assert_eq!(queue.process_next_order().map(|d| d.order_code), Some(9));
        assert_eq!(queue.process_next_order().map(|d| d.order_code), Some(1));
        assert_eq!(queue.process_next_order().map(|d| d.order_code), Some(2));
        assert_eq!(queue.process_next_order().map(|d| d.order_code), None);
    }
}
