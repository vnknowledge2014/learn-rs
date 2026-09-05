#![allow(dead_code, unused_variables, unused_imports)]
/// Cấu trúc nút bên trong danh sách liên kết
struct Nut<T> {
    value: T,
    next: Option<Box<Nut<T>>>,
}

/// Cấu trúc Danh sách liên kết đơn (Singly Linked List)
pub struct ListLienLink<T> {
    peak: Option<Box<Nut<T>>>,
    length: usize,
}

impl<T> ListLienLink<T> {
    /// Khởi tạo một danh sách liên kết rỗng
    pub fn new() -> Self {
        ListLienLink {
            peak: None,
            length: 0,
        }
    }

    /// Thêm một phần tử mới vào đầu danh sách - Độ phức tạp O(1)
    pub fn push_front(&mut self, value: T) {
        // Tạo nút mới trên Heap thông qua con trỏ thông minh Box
        // Sử dụng self.dinh.take() để lấy quyền sở hữu đỉnh cũ mà không vi phạm quy tắc mượn
        let nut_moi = Box::new(Nut {
            value,
            next: self.peak.take(),
        });

        // Gán đỉnh mới cho danh sách
        self.peak = Some(nut_moi);
        self.length += 1;
    }

    /// Lấy phần tử ở đầu danh sách ra và trả về giá trị - Độ phức tạp O(1)
    pub fn pop_front(&mut self) -> Option<T> {
        // .take() thay thế đỉnh bằng None và trả về Some(nut_cu)
        self.peak.take().map(|nut_cu| {
            // Đưa nút kế tiếp lên làm đỉnh mới
            self.peak = nut_cu.next;
            self.length -= 1;
            // Trả về giá trị của nút vừa lấy ra
            nut_cu.value
        })
    }

    /// Xem giá trị phần tử ở đầu danh sách mà không đoạt quyền sở hữu - Trả về tham chiếu mượn
    pub fn peek_front(&self) -> Option<&T> {
        self.peak.as_ref().map(|nut| &nut.value)
    }

    /// Kiểm tra số lượng phần tử hiện tại trong danh sách
    pub fn len(&self) -> usize {
        self.length
    }

    /// Kiểm tra danh sách có đang rỗng hay không
    pub fn is_empty(&self) -> bool {
        self.length == 0
    }
}

/// Cài đặt hàm hủy bộ nhớ an toàn (Safe Drop)
/// Sử dụng vòng lặp tuần tự thay vì đệ quy để triệt tiêu nguy cơ tràn ngăn xếp (Stack Overflow)
impl<T> Drop for ListLienLink<T> {
    fn drop(&mut self) {
        let mut current_node = self.peak.take();
        // Lặp tuần tự gỡ từng Box trên Heap đưa vào biến cục bộ rồi giải phóng
        while let Some(mut nut) = current_node {
            current_node = nut.next.take();
            // nut tự động được giải phóng tại đây mà không cần gọi đệ quy sâu!
        }
    }
}

// Cài đặt Default trait chuẩn phong cách Rust
impl<T> Default for ListLienLink<T> {
    fn default() -> Self {
        Self::new()
    }
}

fn main() {
    println!("============================================================");
    println!("     HIỆN THỰC DANH SÁCH LIÊN KẾT & SMART POINTERS TRONG RUST");
    println!("============================================================");

    let mut list: ListLienLink<i32> = ListLienLink::new();
    println!("Khởi tạo danh sách rỗng: len = {}", list.len());
    assert!(list.is_empty());

    // 1. Thao tác thêm vào đầu danh sách (Push)
    println!("\n[1] Thêm các phần tử vào đầu danh sách:");
    list.push_front(10);
    println!("    - Đã thêm 10. Đỉnh hiện tại: {:?}", list.peek_front());
    list.push_front(20);
    println!("    - Đã thêm 20. Đỉnh hiện tại: {:?}", list.peek_front());
    list.push_front(30);
    println!("    - Đã thêm 30. Đỉnh hiện tại: {:?}", list.peek_front());
    
    println!("    => Tổng số phần tử: {}", list.len());
    assert_eq!(list.len(), 3);
    assert_eq!(list.peek_front(), Some(&30));

    // 2. Thao tác lấy phần tử ra khỏi danh sách (Pop)
    println!("\n[2] Lấy các phần tử ra lần lượt (LIFO):");
    let p1 = list.pop_front();
    println!("    - Lấy ra lần 1: {:?} (Kỳ vọng: Some(30))", p1);
    assert_eq!(p1, Some(30));

    let p2 = list.pop_front();
    println!("    - Lấy ra lần 2: {:?} (Kỳ vọng: Some(20))", p2);
    assert_eq!(p2, Some(20));

    let p3 = list.pop_front();
    println!("    - Lấy ra lần 3: {:?} (Kỳ vọng: Some(10))", p3);
    assert_eq!(p3, Some(10));

    let p4 = list.pop_front();
    println!("    - Lấy ra khi danh sách rỗng: {:?} (Kỳ vọng: None)", p4);
    assert_eq!(p4, None);
    assert!(list.is_empty());

    // 3. Kiểm thử khả năng chịu tải chống tràn ngăn xếp (Drop 100.000 phần tử)
    println!("\n[3] Kiểm thử độ bền của hàm hủy Drop an toàn:");
    {
        let mut list_lon = ListLienLink::new();
        for i in 0..100_000 {
            list_lon.push_front(i);
        }
        println!("    - Đã nạp thành công 100.000 phần tử vào danh sách liên kết.");
        println!("    - Bắt đầu giải phóng bộ nhớ khi ra khỏi khối ngoặc nhọn...");
    } // list_lon bị Drop tại đây. Nhờ vòng lặp tuần tự, không bị tràn Stack!
    println!("    => Giải phóng 100.000 nút bộ nhớ thành công tuyệt đối!");

    println!("============================================================");
    println!("               HOÀN TẤT THỰC NGHIỆM CHƯƠNG 23               ");
    println!("============================================================");
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn push_pop_theo_thu_tu_lifo_o_dau() {
        let mut list: ListLienLink<i32> = ListLienLink::new();
        assert!(list.is_empty());
        list.push_front(1);
        list.push_front(2);
        list.push_front(3);
        assert_eq!(list.len(), 3);
        assert_eq!(list.peek_front(), Some(&3));
        assert_eq!(list.pop_front(), Some(3));
        assert_eq!(list.pop_front(), Some(2));
        assert_eq!(list.pop_front(), Some(1));
        assert_eq!(list.pop_front(), None);
        assert!(list.is_empty());
    }

    #[test]
    fn list_new_thi_empty() {
        let list: ListLienLink<String> = ListLienLink::new();
        assert_eq!(list.len(), 0);
        assert!(list.is_empty());
        assert_eq!(list.peek_front(), None);
    }

    #[test]
    fn cancel_list_lon_no_cap_stack() {
        // Bằng chứng cho mục "Drop lặp thay vì đệ quy": 1 triệu nút không sập.
        let mut list: ListLienLink<u32> = ListLienLink::new();
        for i in 0..1_000_000 {
            list.push_front(i);
        }
        assert_eq!(list.len(), 1_000_000);
        drop(list); // nếu Drop đệ quy, dòng này sẽ tràn ngăn xếp
    }
}
