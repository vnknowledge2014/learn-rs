#![allow(dead_code, unused_variables, unused_imports)]
/// Cấu trúc nút bên trong danh sách liên kết
struct Nut<T> {
    gia_tri: T,
    ke_tiep: Option<Box<Nut<T>>>,
}

/// Cấu trúc Danh sách liên kết đơn (Singly Linked List)
pub struct DanhSachLienKet<T> {
    dinh: Option<Box<Nut<T>>>,
    do_dai: usize,
}

impl<T> DanhSachLienKet<T> {
    /// Khởi tạo một danh sách liên kết rỗng
    pub fn new() -> Self {
        DanhSachLienKet {
            dinh: None,
            do_dai: 0,
        }
    }

    /// Thêm một phần tử mới vào đầu danh sách - Độ phức tạp O(1)
    pub fn push_dau(&mut self, gia_tri: T) {
        // Tạo nút mới trên Heap thông qua con trỏ thông minh Box
        // Sử dụng self.dinh.take() để lấy quyền sở hữu đỉnh cũ mà không vi phạm quy tắc mượn
        let nut_moi = Box::new(Nut {
            gia_tri,
            ke_tiep: self.dinh.take(),
        });

        // Gán đỉnh mới cho danh sách
        self.dinh = Some(nut_moi);
        self.do_dai += 1;
    }

    /// Lấy phần tử ở đầu danh sách ra và trả về giá trị - Độ phức tạp O(1)
    pub fn pop_dau(&mut self) -> Option<T> {
        // .take() thay thế đỉnh bằng None và trả về Some(nut_cu)
        self.dinh.take().map(|nut_cu| {
            // Đưa nút kế tiếp lên làm đỉnh mới
            self.dinh = nut_cu.ke_tiep;
            self.do_dai -= 1;
            // Trả về giá trị của nút vừa lấy ra
            nut_cu.gia_tri
        })
    }

    /// Xem giá trị phần tử ở đầu danh sách mà không đoạt quyền sở hữu - Trả về tham chiếu mượn
    pub fn peek_dau(&self) -> Option<&T> {
        self.dinh.as_ref().map(|nut| &nut.gia_tri)
    }

    /// Kiểm tra số lượng phần tử hiện tại trong danh sách
    pub fn len(&self) -> usize {
        self.do_dai
    }

    /// Kiểm tra danh sách có đang rỗng hay không
    pub fn is_empty(&self) -> bool {
        self.do_dai == 0
    }
}

/// Cài đặt hàm hủy bộ nhớ an toàn (Safe Drop)
/// Sử dụng vòng lặp tuần tự thay vì đệ quy để triệt tiêu nguy cơ tràn ngăn xếp (Stack Overflow)
impl<T> Drop for DanhSachLienKet<T> {
    fn drop(&mut self) {
        let mut nut_hien_tai = self.dinh.take();
        // Lặp tuần tự gỡ từng Box trên Heap đưa vào biến cục bộ rồi giải phóng
        while let Some(mut nut) = nut_hien_tai {
            nut_hien_tai = nut.ke_tiep.take();
            // nut tự động được giải phóng tại đây mà không cần gọi đệ quy sâu!
        }
    }
}

// Cài đặt Default trait chuẩn phong cách Rust
impl<T> Default for DanhSachLienKet<T> {
    fn default() -> Self {
        Self::new()
    }
}

fn main() {
    println!("============================================================");
    println!("     HIỆN THỰC DANH SÁCH LIÊN KẾT & SMART POINTERS TRONG RUST");
    println!("============================================================");

    let mut danh_sach: DanhSachLienKet<i32> = DanhSachLienKet::new();
    println!("Khởi tạo danh sách rỗng: len = {}", danh_sach.len());
    assert!(danh_sach.is_empty());

    // 1. Thao tác thêm vào đầu danh sách (Push)
    println!("\n[1] Thêm các phần tử vào đầu danh sách:");
    danh_sach.push_dau(10);
    println!("    - Đã thêm 10. Đỉnh hiện tại: {:?}", danh_sach.peek_dau());
    danh_sach.push_dau(20);
    println!("    - Đã thêm 20. Đỉnh hiện tại: {:?}", danh_sach.peek_dau());
    danh_sach.push_dau(30);
    println!("    - Đã thêm 30. Đỉnh hiện tại: {:?}", danh_sach.peek_dau());
    
    println!("    => Tổng số phần tử: {}", danh_sach.len());
    assert_eq!(danh_sach.len(), 3);
    assert_eq!(danh_sach.peek_dau(), Some(&30));

    // 2. Thao tác lấy phần tử ra khỏi danh sách (Pop)
    println!("\n[2] Lấy các phần tử ra lần lượt (LIFO):");
    let p1 = danh_sach.pop_dau();
    println!("    - Lấy ra lần 1: {:?} (Kỳ vọng: Some(30))", p1);
    assert_eq!(p1, Some(30));

    let p2 = danh_sach.pop_dau();
    println!("    - Lấy ra lần 2: {:?} (Kỳ vọng: Some(20))", p2);
    assert_eq!(p2, Some(20));

    let p3 = danh_sach.pop_dau();
    println!("    - Lấy ra lần 3: {:?} (Kỳ vọng: Some(10))", p3);
    assert_eq!(p3, Some(10));

    let p4 = danh_sach.pop_dau();
    println!("    - Lấy ra khi danh sách rỗng: {:?} (Kỳ vọng: None)", p4);
    assert_eq!(p4, None);
    assert!(danh_sach.is_empty());

    // 3. Kiểm thử khả năng chịu tải chống tràn ngăn xếp (Drop 100.000 phần tử)
    println!("\n[3] Kiểm thử độ bền của hàm hủy Drop an toàn:");
    {
        let mut danh_sach_lon = DanhSachLienKet::new();
        for i in 0..100_000 {
            danh_sach_lon.push_dau(i);
        }
        println!("    - Đã nạp thành công 100.000 phần tử vào danh sách liên kết.");
        println!("    - Bắt đầu giải phóng bộ nhớ khi ra khỏi khối ngoặc nhọn...");
    } // danh_sach_lon bị Drop tại đây. Nhờ vòng lặp tuần tự, không bị tràn Stack!
    println!("    => Giải phóng 100.000 nút bộ nhớ thành công tuyệt đối!");

    println!("============================================================");
    println!("               HOÀN TẤT THỰC NGHIỆM CHƯƠNG 23               ");
    println!("============================================================");
}

#[cfg(test)]
mod kiem_thu {
    use super::*;

    #[test]
    fn push_pop_theo_thu_tu_lifo_o_dau() {
        let mut ds: DanhSachLienKet<i32> = DanhSachLienKet::new();
        assert!(ds.is_empty());
        ds.push_dau(1);
        ds.push_dau(2);
        ds.push_dau(3);
        assert_eq!(ds.len(), 3);
        assert_eq!(ds.peek_dau(), Some(&3));
        assert_eq!(ds.pop_dau(), Some(3));
        assert_eq!(ds.pop_dau(), Some(2));
        assert_eq!(ds.pop_dau(), Some(1));
        assert_eq!(ds.pop_dau(), None);
        assert!(ds.is_empty());
    }

    #[test]
    fn danh_sach_moi_thi_rong() {
        let ds: DanhSachLienKet<String> = DanhSachLienKet::new();
        assert_eq!(ds.len(), 0);
        assert!(ds.is_empty());
        assert_eq!(ds.peek_dau(), None);
    }

    #[test]
    fn huy_danh_sach_lon_khong_tran_ngan_xep() {
        // Bằng chứng cho mục "Drop lặp thay vì đệ quy": 1 triệu nút không sập.
        let mut ds: DanhSachLienKet<u32> = DanhSachLienKet::new();
        for i in 0..1_000_000 {
            ds.push_dau(i);
        }
        assert_eq!(ds.len(), 1_000_000);
        drop(ds); // nếu Drop đệ quy, dòng này sẽ tràn ngăn xếp
    }
}
