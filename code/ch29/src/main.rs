#![allow(dead_code, unused_variables, unused_imports)]
/// Cấu trúc một nút bên trong Cây nhị phân tìm kiếm
#[derive(Debug)]
pub struct NutCay<T> {
    pub gia_tri: T,
    pub trai: Option<Box<NutCay<T>>>,
    pub phai: Option<Box<NutCay<T>>>,
}

impl<T> NutCay<T> {
    pub fn new(gia_tri: T) -> Self {
        NutCay {
            gia_tri,
            trai: None,
            phai: None,
        }
    }
}

/// Cấu trúc Cây nhị phân tìm kiếm hoàn chỉnh
#[derive(Debug)]
pub struct CayNhiPhanTimKiem<T: Ord> {
    goc: Option<Box<NutCay<T>>>,
    so_luong: usize,
}

impl<T: Ord> CayNhiPhanTimKiem<T> {
    /// Khởi tạo một cây BST rỗng
    pub fn new() -> Self {
        CayNhiPhanTimKiem {
            goc: None,
            so_luong: 0,
        }
    }

    /// Thêm một phần tử vào cây - Duy trì tính chất BST
    pub fn them(&mut self, gia_tri: T) {
        if Self::them_de_quy(&mut self.goc, gia_tri) {
            self.so_luong += 1;
        }
    }

    fn them_de_quy(nut: &mut Option<Box<NutCay<T>>>, gia_tri: T) -> bool {
        match nut {
            // Khi tìm thấy vị trí lá trống thích hợp: Tạo Box mới
            None => {
                *nut = Some(Box::new(NutCay::new(gia_tri)));
                true
            }
            Some(hien_tai) => {
                if gia_tri < hien_tai.gia_tri {
                    Self::them_de_quy(&mut hien_tai.trai, gia_tri)
                } else if gia_tri > hien_tai.gia_tri {
                    Self::them_de_quy(&mut hien_tai.phai, gia_tri)
                } else {
                    // Giá trị đã tồn tại trong cây (không cho phép trùng lặp)
                    false
                }
            }
        }
    }

    /// Tìm kiếm một giá trị trong cây - Tốc độ O(log N)
    pub fn chua_khoa(&self, gia_tri: &T) -> bool {
        let mut con_tro = &self.goc;
        while let Some(nut) = con_tro {
            if gia_tri == &nut.gia_tri {
                return true;
            } else if gia_tri < &nut.gia_tri {
                con_tro = &nut.trai;
            } else {
                con_tro = &nut.phai;
            }
        }
        false
    }

    /// Duyệt cây theo Trung thứ tự (In-order: Trái -> Gốc -> Phải)
    /// Trả về một Vector chứa các tham chiếu mượn được sắp xếp tăng dần!
    pub fn duyet_in_order(&self) -> Vec<&T> {
        let mut ket_qua = Vec::new();
        Self::thu_thap_in_order(&self.goc, &mut ket_qua);
        ket_qua
    }

    fn thu_thap_in_order<'a>(nut: &'a Option<Box<NutCay<T>>>, ket_qua: &mut Vec<&'a T>) {
        if let Some(hien_tai) = nut {
            // 1. Duyệt toàn bộ cây con bên trái
            Self::thu_thap_in_order(&hien_tai.trai, ket_qua);
            // 2. Thu thập nút hiện tại
            ket_qua.push(&hien_tai.gia_tri);
            // 3. Duyệt toàn bộ cây con bên phải
            Self::thu_thap_in_order(&hien_tai.phai, ket_qua);
        }
    }

    /// Tính chiều cao của cây (Độ sâu tối đa từ gốc đến lá xa nhất)
    pub fn tinh_chieu_cao(&self) -> usize {
        Self::chieu_cao_de_quy(&self.goc)
    }

    fn chieu_cao_de_quy(nut: &Option<Box<NutCay<T>>>) -> usize {
        match nut {
            None => 0,
            Some(hien_tai) => {
                let cao_trai = Self::chieu_cao_de_quy(&hien_tai.trai);
                let cao_phai = Self::chieu_cao_de_quy(&hien_tai.phai);
                1 + cao_trai.max(cao_phai)
            }
        }
    }

    pub fn len(&self) -> usize {
        self.so_luong
    }

    pub fn is_empty(&self) -> bool {
        self.so_luong == 0
    }
}

impl<T: Ord> Default for CayNhiPhanTimKiem<T> {
    fn default() -> Self {
        Self::new()
    }
}

fn main() {
    println!("============================================================");
    println!("    HIỆN THỰC CÂY NHỊ PHÂN TÌM KIẾM (BST) AN TOÀN TRONG RUST");
    println!("============================================================");

    let mut cay_bst: CayNhiPhanTimKiem<i32> = CayNhiPhanTimKiem::new();

    // 1. Thêm các phần tử vào cây
    // Cấu trúc dự kiến:
    //          50
    //        /    \
    //       30     70
    //      /  \   /  \
    //     20  40 60  80
    println!("[1] Nạp các giá trị vào Cây nhị phân tìm kiếm:");
    let cac_so = [50, 30, 70, 20, 40, 60, 80];
    for &so in &cac_so {
        cay_bst.them(so);
        print!("{} ", so);
    }
    println!("\n    - Tổng số nút trong cây: {}", cay_bst.len());
    assert_eq!(cay_bst.len(), 7);

    // 2. Kiểm tra chiều cao của cây
    let chieu_cao = cay_bst.tinh_chieu_cao();
    println!("\n[2] Chiều cao của cây: {}", chieu_cao);
    assert_eq!(chieu_cao, 3); // 3 tầng: 50 -> (30,70) -> (20,40,60,80)

    // 3. Kiểm tra tính năng tìm kiếm O(log N)
    println!("\n[3] Kiểm tra tính năng tìm kiếm nhị phân:");
    println!("    - Tìm số 40: {}", cay_bst.chua_khoa(&40));
    println!("    - Tìm số 99: {}", cay_bst.chua_khoa(&99));
    assert!(cay_bst.chua_khoa(&40));
    assert!(!cay_bst.chua_khoa(&99));

    // 4. Duyệt In-order xác nhận dãy số tăng dần hoàn hảo
    println!("\n[4] Duyệt cây In-order (Trái -> Gốc -> Phải):");
    let danh_sach_tang_dan = cay_bst.duyet_in_order();
    print!("    - Kết quả in: ");
    for &gia_tri in &danh_sach_tang_dan {
        print!("{} ", gia_tri);
    }
    println!();

    let ky_vong = vec![&20, &30, &40, &50, &60, &70, &80];
    assert_eq!(danh_sach_tang_dan, ky_vong);
    println!("    => Dãy số được sắp xếp tăng dần hoàn hảo đúng theo lý thuyết BST!");

    println!("============================================================");
    println!("               HOÀN TẤT THỰC NGHIỆM CHƯƠNG 25               ");
    println!("============================================================");
}


#[cfg(test)]
mod kiem_thu {
    use super::*;

    fn cay_mau() -> CayNhiPhanTimKiem<i32> {
        let mut c = CayNhiPhanTimKiem::new();
        for x in [50, 30, 70, 20, 40, 60, 80] {
            c.them(x);
        }
        c
    }

    #[test]
    fn duyet_in_order_luon_tang_dan() {
        let c = cay_mau();
        let so: Vec<i32> = c.duyet_in_order().into_iter().copied().collect();
        assert_eq!(so, vec![20, 30, 40, 50, 60, 70, 80]); // BST in-order = sắp xếp
    }

    #[test]
    fn chua_khoa() {
        let c = cay_mau();
        assert!(c.chua_khoa(&40));
        assert!(c.chua_khoa(&80));
        assert!(!c.chua_khoa(&99));
        assert!(!c.chua_khoa(&35));
    }

    #[test]
    fn khong_chen_trung_lap() {
        let mut c = CayNhiPhanTimKiem::new();
        c.them(5);
        c.them(5); // giá trị trùng bị bỏ qua
        c.them(5);
        assert_eq!(c.len(), 1);
    }

    #[test]
    fn cay_can_bang_thap_hon_cay_suy_bien() {
        let mut suy_bien = CayNhiPhanTimKiem::new();
        for x in 1..=7 {
            suy_bien.them(x); // chèn tuần tự -> suy biến thành danh sách
        }
        assert_eq!(suy_bien.tinh_chieu_cao(), 7);
        assert_eq!(cay_mau().tinh_chieu_cao(), 3); // cân đối -> ~log N
    }

    #[test]
    fn cay_rong() {
        let c: CayNhiPhanTimKiem<i32> = CayNhiPhanTimKiem::new();
        assert!(c.is_empty());
        assert_eq!(c.tinh_chieu_cao(), 0);
        assert_eq!(c.duyet_in_order().len(), 0);
    }
}
