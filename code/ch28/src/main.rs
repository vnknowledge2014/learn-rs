#![allow(dead_code, unused_variables, unused_imports)]
use std::collections::VecDeque;

/// ỨNG DỤNG 1 CỦA STACK: Kiểm tra dấu ngoặc hợp lệ
/// Thuật toán sử dụng Ngăn xếp (LIFO):
/// - Gặp dấu mở '(', '[', '{': Đẩy vào đỉnh ngăn xếp.
/// - Gặp dấu đóng ')', ']', '}': Rút phần tử trên đỉnh ra so khớp.
///   Nếu không khớp hoặc ngăn xếp rỗng -> Biểu thức sai cú pháp!
/// - Kết thúc chuỗi, nếu ngăn xếp rỗng -> Biểu thức hợp lệ.
pub fn kiem_tra_ngoac_hop_le(bieu_thuc: &str) -> bool {
    let mut ngan_xep: Vec<char> = Vec::new();

    for ky_tu in bieu_thuc.chars() {
        match ky_tu {
            '(' | '[' | '{' => {
                ngan_xep.push(ky_tu);
            }
            ')' => {
                if ngan_xep.pop() != Some('(') {
                    return false;
                }
            }
            ']' => {
                if ngan_xep.pop() != Some('[') {
                    return false;
                }
            }
            '}' => {
                if ngan_xep.pop() != Some('{') {
                    return false;
                }
            }
            // Bỏ qua các ký tự chữ cái, số, hoặc khoảng trắng
            _ => {}
        }
    }

    // Biểu thức chỉ đúng khi mọi dấu ngoặc mở đều đã được đóng khớp hết
    ngan_xep.is_empty()
}

/// Mô hình Đơn hàng trong hệ thống thương mại điện tử
#[derive(Debug, PartialEq, Clone)]
pub struct DonHang {
    pub ma_don: u32,
    pub ten_khach: String,
    pub tong_tien: f64,
}

/// ỨNG DỤNG 2 CỦA QUEUE: Hệ thống quản lý hàng đợi đơn hàng chuẩn FIFO
pub struct HangDoiDonHang {
    danh_sach: VecDeque<DonHang>,
}

impl HangDoiDonHang {
    pub fn new() -> Self {
        Self {
            danh_sach: VecDeque::new(),
        }
    }

    /// Khách đặt hàng: Xếp vào cuối hàng đợi - O(1)
    pub fn them_don(&mut self, don: DonHang) {
        self.danh_sach.push_back(don);
    }

    /// Đơn hàng VIP (Ưu tiên khẩn cấp): Chèn thẳng vào đầu hàng đợi - O(1)
    pub fn them_don_vip(&mut self, don: DonHang) {
        self.danh_sach.push_front(don);
    }

    /// Nhà bếp / Kho xuất hàng: Phục vụ đơn đến trước - O(1)
    pub fn xu_ly_don_ke_tiep(&mut self) -> Option<DonHang> {
        self.danh_sach.pop_front()
    }

    /// Xem trước đơn sắp được phục vụ mà không xóa khỏi hàng đợi
    pub fn xem_don_dau(&self) -> Option<&DonHang> {
        self.danh_sach.front()
    }

    pub fn so_don_dang_cho(&self) -> usize {
        self.danh_sach.len()
    }
}

impl Default for HangDoiDonHang {
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

    println!("    - Biểu thức 1 '{}': {}", bieu_thuc_1, kiem_tra_ngoac_hop_le(bieu_thuc_1));
    println!("    - Biểu thức 2 '{}': {}", bieu_thuc_2, kiem_tra_ngoac_hop_le(bieu_thuc_2));
    println!("    - Biểu thức 3 '{}': {}", bieu_thuc_3, kiem_tra_ngoac_hop_le(bieu_thuc_3));

    assert!(kiem_tra_ngoac_hop_le(bieu_thuc_1));
    assert!(!kiem_tra_ngoac_hop_le(bieu_thuc_2));
    assert!(!kiem_tra_ngoac_hop_le(bieu_thuc_3));

    // 2. Kiểm thử Hệ thống Hàng đợi đơn hàng với VecDeque
    println!("\n[2] Vận hành hệ thống xử lý đơn hàng FIFO bằng VecDeque:");
    let mut he_thong = HangDoiDonHang::new();

    // Khách hàng thông thường đặt hàng lần lượt
    he_thong.them_don(DonHang {
        ma_don: 101,
        ten_khach: String::from("Nguyễn Văn A"),
        tong_tien: 150.0,
    });
    he_thong.them_don(DonHang {
        ma_don: 102,
        ten_khach: String::from("Trần Thị B"),
        tong_tien: 80.0,
    });

    println!("    - Đã nhận 2 đơn hàng thông thường. Số đơn chờ: {}", he_thong.so_don_dang_cho());

    // Đơn hàng hỏa tốc VIP xuất hiện! Đưa thẳng vào đầu hàng đợi
    he_thong.them_don_vip(DonHang {
        ma_don: 999,
        ten_khach: String::from("Khách VIP Kim Cương"),
        tong_tien: 500.0,
    });
    println!("    - Nhận đơn hỏa tốc VIP 999 (chen lên đầu hàng)!");

    // Xem trước đơn hàng kế tiếp
    if let Some(don_dau) = he_thong.xem_don_dau() {
        println!("    - Đơn hàng chuẩn bị xử lý tiếp theo là: Mã #{} ({})", don_dau.ma_don, don_dau.ten_khach);
        assert_eq!(don_dau.ma_don, 999);
    }

    // Tiến hành xuất kho lần lượt theo đúng thứ tự ưu tiên
    println!("\n    Bắt đầu xuất kho theo thứ tự FIFO:");
    let mut thu_tu_xu_ly = Vec::new();
    while let Some(don) = he_thong.xu_ly_don_ke_tiep() {
        println!("    -> Đang đóng gói đơn #{}: Khách {} - {:.2}k", don.ma_don, don.ten_khach, don.tong_tien);
        thu_tu_xu_ly.push(don.ma_don);
    }

    // Xác nhận thứ tự xử lý: Đơn VIP 999 trước, sau đó là 101, rồi đến 102
    assert_eq!(thu_tu_xu_ly, vec![999, 101, 102]);
    assert_eq!(he_thong.so_don_dang_cho(), 0);
    println!("    => Toàn bộ hàng đợi đã được xử lý sạch sẽ!");

    println!("============================================================");
    println!("               HOÀN TẤT THỰC NGHIỆM CHƯƠNG 24               ");
    println!("============================================================");
}


#[cfg(test)]
mod kiem_thu {
    use super::*;

    fn don(ma: u32, ten: &str) -> DonHang {
        DonHang { ma_don: ma, ten_khach: ten.into(), tong_tien: 100.0 }
    }

    #[test]
    fn kiem_tra_ngoac() {
        assert!(kiem_tra_ngoac_hop_le("(a[b]{c})"));
        assert!(kiem_tra_ngoac_hop_le(""));
        assert!(!kiem_tra_ngoac_hop_le("(a]"));
        assert!(!kiem_tra_ngoac_hop_le("((("));
        assert!(!kiem_tra_ngoac_hop_le(")("));
    }

    #[test]
    fn hang_doi_fifo_va_uu_tien_vip() {
        let mut hd = HangDoiDonHang::new();
        hd.them_don(don(1, "A"));
        hd.them_don(don(2, "B"));
        hd.them_don_vip(don(9, "VIP")); // chen lên đầu
        assert_eq!(hd.so_don_dang_cho(), 3);
        assert_eq!(hd.xem_don_dau().map(|d| d.ma_don), Some(9));

        // VIP ra trước, phần còn lại giữ đúng thứ tự FIFO
        assert_eq!(hd.xu_ly_don_ke_tiep().map(|d| d.ma_don), Some(9));
        assert_eq!(hd.xu_ly_don_ke_tiep().map(|d| d.ma_don), Some(1));
        assert_eq!(hd.xu_ly_don_ke_tiep().map(|d| d.ma_don), Some(2));
        assert_eq!(hd.xu_ly_don_ke_tiep().map(|d| d.ma_don), None);
    }
}
