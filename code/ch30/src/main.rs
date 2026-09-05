#![allow(dead_code, unused_variables, unused_imports)]
use std::collections::{HashMap, VecDeque};

/// PHẦN 1: THỐNG KÊ TẦN SUẤT TỪ VỚI BẢNG BĂM HASHMAP
pub fn thong_ke_tu_vung(van_ban: &str) -> HashMap<String, usize> {
    let mut bang_dem = HashMap::new();
    for tu in van_ban.split_whitespace() {
        // Chuẩn hóa từ về chữ thường
        let tu_chuan = tu.to_lowercase();
        // Entry API: Tra cứu một lần, nếu chưa có thì khởi tạo giá trị 0, sau đó tăng 1
        let dem = bang_dem.entry(tu_chuan).or_insert(0);
        *dem += 1;
    }
    bang_dem
}

/// PHẦN 2: CẤU TRÚC ĐỒ THỊ AN TOÀN VÀ THUẬT TOÁN BFS
pub struct DoThi {
    danh_sach_ke: Vec<Vec<usize>>,
    ten_cac_dinh: Vec<String>,
}

impl DoThi {
    pub fn new() -> Self {
        DoThi {
            danh_sach_ke: Vec::new(),
            ten_cac_dinh: Vec::new(),
        }
    }

    /// Thêm một đỉnh mới vào đồ thị và trả về chỉ số của đỉnh đó
    pub fn them_dinh(&mut self, ten: &str) -> usize {
        let chi_so = self.ten_cac_dinh.len();
        self.ten_cac_dinh.push(ten.to_string());
        self.danh_sach_ke.push(Vec::new());
        chi_so
    }

    /// Thêm một cạnh nối hai chiều giữa hai đỉnh u và v
    pub fn them_canh(&mut self, u: usize, v: usize) {
        if u < self.danh_sach_ke.len() && v < self.danh_sach_ke.len() {
            self.danh_sach_ke[u].push(v);
            self.danh_sach_ke[v].push(u); // Đồ thị vô hướng 2 chiều
        }
    }

    /// Thuật toán BFS tìm đường đi ngắn nhất (Số chặng) giữa hai đỉnh
    pub fn bfs_khoang_cach_ngan_nhat(&self, diem_dau: usize, diem_dich: usize) -> Option<usize> {
        if diem_dau >= self.danh_sach_ke.len() || diem_dich >= self.danh_sach_ke.len() {
            return None;
        }

        // Mảng đánh dấu các đỉnh đã thăm để tránh chu trình lặp vô tận
        let mut da_tham = vec![false; self.danh_sach_ke.len()];
        // Hàng đợi lưu cặp (chỉ_số_đỉnh, khoảng_cách)
        let mut hang_doi: VecDeque<(usize, usize)> = VecDeque::new();

        da_tham[diem_dau] = true;
        hang_doi.push_back((diem_dau, 0));

        while let Some((hien_tai, khoang_cach)) = hang_doi.pop_front() {
            if hien_tai == diem_dich {
                return Some(khoang_cach); // Tìm thấy đích đến!
            }

            for &ke in &self.danh_sach_ke[hien_tai] {
                if !da_tham[ke] {
                    da_tham[ke] = true;
                    hang_doi.push_back((ke, khoang_cach + 1));
                }
            }
        }

        None // Không có đường đi kết nối giữa hai đỉnh này
    }

    pub fn lay_ten(&self, chi_so: usize) -> &str {
        &self.ten_cac_dinh[chi_so]
    }
}

impl Default for DoThi {
    fn default() -> Self {
        Self::new()
    }
}

/// PHẦN 3: THUẬT TOÁN SẮP XẾP NHANH (QUICKSORT) TẠI CHỖ
pub fn quicksort<T: Ord>(du_lieu: &mut [T]) {
    if du_lieu.len() <= 1 {
        return;
    }
    let vi_tri_chot = phan_vung(du_lieu);
    // Chia đôi mảng và đệ quy sắp xếp hai nửa
    quicksort(&mut du_lieu[0..vi_tri_chot]);
    quicksort(&mut du_lieu[vi_tri_chot + 1..]);
}

fn phan_vung<T: Ord>(du_lieu: &mut [T]) -> usize {
    let do_dai = du_lieu.len();
    let chi_so_chot = do_dai - 1;
    let mut i = 0;

    for j in 0..chi_so_chot {
        if du_lieu[j] <= du_lieu[chi_so_chot] {
            du_lieu.swap(i, j);
            i += 1;
        }
    }
    du_lieu.swap(i, chi_so_chot);
    i
}

fn main() {
    println!("============================================================");
    println!("    BẢNG BĂM, ĐỒ THỊ VÀ CÁC THUẬT TOÁN CỐT LÕI TRONG RUST   ");
    println!("============================================================");

    // 1. Kiểm thử Bảng băm đếm tần suất từ
    println!("[1] Thống kê tần suất từ vựng bằng HashMap Entry API:");
    let van_ban = "học rust thật vui học lập trình rust thật tuyệt vời";
    let ket_qua_dem = thong_ke_tu_vung(van_ban);
    for (tu, so_lan) in &ket_qua_dem {
        println!("    - Từ '{:8}': xuất hiện {} lần", tu, so_lan);
    }
    assert_eq!(ket_qua_dem.get("rust"), Some(&2));
    assert_eq!(ket_qua_dem.get("học"), Some(&2));
    assert_eq!(ket_qua_dem.get("vui"), Some(&1));

    // 2. Kiểm thử Mạng lưới Đồ thị và Thuật toán BFS
    println!("\n[2] Mô phỏng mạng xã hội kết nối bạn bè bằng Đồ thị & BFS:");
    let mut mang_xa_hoi = DoThi::new();
    let an = mang_xa_hoi.them_dinh("An");       // Đỉnh 0
    let binh = mang_xa_hoi.them_dinh("Bình");   // Đỉnh 1
    let chi = mang_xa_hoi.them_dinh("Chi");     // Đỉnh 2
    let dung = mang_xa_hoi.them_dinh("Dũng");   // Đỉnh 3
    let hoa = mang_xa_hoi.them_dinh("Hoa");     // Đỉnh 4 (ở xa)

    // Thiết lập các mối quan hệ bạn bè (Cạnh)
    // An quen Bình, Bình quen Chi, Chi quen Dũng, An quen Dũng (lối tắt)
    mang_xa_hoi.them_canh(an, binh);
    mang_xa_hoi.them_canh(binh, chi);
    mang_xa_hoi.them_canh(chi, dung);
    mang_xa_hoi.them_canh(an, dung); // Lối tắt trực tiếp từ An đến Dũng!

    println!("    - Tìm khoảng cách kết nối giữa '{}' và '{}':", mang_xa_hoi.lay_ten(an), mang_xa_hoi.lay_ten(chi));
    let khoang_cach_an_chi = mang_xa_hoi.bfs_khoang_cach_ngan_nhat(an, chi);
    println!("      => Khoảng cách ngắn nhất: {:?} chặng", khoang_cach_an_chi);
    assert_eq!(khoang_cach_an_chi, Some(2)); // An -> Bình -> Chi hoặc An -> Dũng -> Chi

    println!("    - Tìm khoảng cách kết nối giữa '{}' và '{}':", mang_xa_hoi.lay_ten(an), mang_xa_hoi.lay_ten(dung));
    let khoang_cach_an_dung = mang_xa_hoi.bfs_khoang_cach_ngan_nhat(an, dung);
    println!("      => Khoảng cách ngắn nhất: {:?} chặng (nhờ lối tắt trực tiếp!)", khoang_cach_an_dung);
    assert_eq!(khoang_cach_an_dung, Some(1));

    println!("    - Tìm khoảng cách đến '{}' (Chưa có kết nối):", mang_xa_hoi.lay_ten(hoa));
    let khoang_cach_hoa = mang_xa_hoi.bfs_khoang_cach_ngan_nhat(an, hoa);
    println!("      => Kết quả: {:?} (Không có đường đi)", khoang_cach_hoa);
    assert_eq!(khoang_cach_hoa, None);

    // 3. Kiểm thử Thuật toán Sắp xếp nhanh Quicksort
    println!("\n[3] Kiểm thử Thuật toán Sắp xếp nhanh Quicksort tại chỗ:");
    let mut mang_so = [42, 12, 88, 5, 63, 19, 77, 3];
    println!("    - Mảng trước khi sắp xếp: {:?}", mang_so);
    quicksort(&mut mang_so);
    println!("    - Mảng sau khi sắp xếp   : {:?}", mang_so);
    assert_eq!(mang_so, [3, 5, 12, 19, 42, 63, 77, 88]);
    println!("    => Quicksort O(N log N) hoàn tất thành công!");

    println!("============================================================");
    println!("               HOÀN TẤT THỰC NGHIỆM CHƯƠNG 26               ");
    println!("============================================================");
}


#[cfg(test)]
mod kiem_thu {
    use super::*;

    #[test]
    fn dem_tan_suat_tu() {
        let bang = thong_ke_tu_vung("rust rust an toan rust");
        assert_eq!(bang.get("rust"), Some(&3));
        assert_eq!(bang.get("an"), Some(&1));
        assert_eq!(bang.get("khong-co"), None);
    }

    #[test]
    fn quicksort_khop_voi_sort_chuan() {
        let mut a = vec![5, 2, 9, 1, 5, 6, 3, 3, 8];
        let mut b = a.clone();
        quicksort(&mut a);
        b.sort();
        assert_eq!(a, b); // kiểm chứng chéo với thư viện chuẩn
    }

    #[test]
    fn quicksort_truong_hop_bien() {
        let mut rong: Vec<i32> = vec![];
        quicksort(&mut rong);
        assert!(rong.is_empty());

        let mut mot = vec![42];
        quicksort(&mut mot);
        assert_eq!(mot, vec![42]);

        // Trường hợp XẤU NHẤT O(N^2): mảng đã sắp xếp sẵn — vẫn phải đúng
        let mut da_sap: Vec<i32> = (1..=100).collect();
        quicksort(&mut da_sap);
        assert_eq!(da_sap, (1..=100).collect::<Vec<i32>>());
    }

    #[test]
    fn bfs_tim_duong_ngan_nhat() {
        let mut g = DoThi::new();
        let a = g.them_dinh("A");
        let b = g.them_dinh("B");
        let c = g.them_dinh("C");
        let d = g.them_dinh("D");
        g.them_canh(a, b);
        g.them_canh(b, c);
        g.them_canh(a, d);
        g.them_canh(d, c);
        assert_eq!(g.bfs_khoang_cach_ngan_nhat(a, c), Some(2));
        assert_eq!(g.bfs_khoang_cach_ngan_nhat(a, a), Some(0));
    }

    #[test]
    fn bfs_khong_co_duong_di() {
        let mut g = DoThi::new();
        let a = g.them_dinh("A");
        let b = g.them_dinh("B"); // cô lập
        assert_eq!(g.bfs_khoang_cach_ngan_nhat(a, b), None);
    }
}
