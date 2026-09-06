#![allow(dead_code, unused_variables, unused_imports)]
use std::collections::{HashMap, VecDeque};

/// PHẦN 1: THỐNG KÊ TẦN SUẤT TỪ VỚI BẢNG BĂM HASHMAP
pub fn thong_ke_from_region(van_ban: &str) -> HashMap<String, usize> {
    let mut table_count = HashMap::new();
    for tu in van_ban.split_whitespace() {
        // Chuẩn hóa từ về chữ thường
        let from_standard = tu.to_lowercase();
        // Entry API: Tra cứu một lần, nếu chưa có thì khởi tạo giá trị 0, sau đó tăng 1
        let count = table_count.entry(from_standard).or_insert(0);
        *count += 1;
    }
    table_count
}

/// PHẦN 2: CẤU TRÚC ĐỒ THỊ AN TOÀN VÀ THUẬT TOÁN BFS
pub struct Graph {
    adjacency_list: Vec<Vec<usize>>,
    name_all_peak: Vec<String>,
}

impl Graph {
    pub fn new() -> Self {
        Graph {
            adjacency_list: Vec::new(),
            name_all_peak: Vec::new(),
        }
    }

    /// Thêm một đỉnh mới vào đồ thị và trả về chỉ số của đỉnh đó
    pub fn add_peak(&mut self, name: &str) -> usize {
        let chi_so = self.name_all_peak.len();
        self.name_all_peak.push(name.to_string());
        self.adjacency_list.push(Vec::new());
        chi_so
    }

    /// Thêm một cạnh nối hai chiều giữa hai đỉnh u và v
    pub fn add_edge(&mut self, u: usize, v: usize) {
        if u < self.adjacency_list.len() && v < self.adjacency_list.len() {
            self.adjacency_list[u].push(v);
            self.adjacency_list[v].push(u); // Đồ thị vô hướng 2 chiều
        }
    }

    /// Thuật toán BFS tìm đường đi ngắn nhất (Số chặng) giữa hai đỉnh
    pub fn bfs_shortest_distance(&self, diem_dau: usize, diem_dich: usize) -> Option<usize> {
        if diem_dau >= self.adjacency_list.len() || diem_dich >= self.adjacency_list.len() {
            return None;
        }

        // Mảng đánh dấu các đỉnh đã thăm để tránh chu trình lặp vô tận
        let mut da_tham = vec![false; self.adjacency_list.len()];
        // Hàng đợi lưu cặp (chỉ_số_đỉnh, khoảng_cách)
        let mut queue: VecDeque<(usize, usize)> = VecDeque::new();

        da_tham[diem_dau] = true;
        queue.push_back((diem_dau, 0));

        while let Some((current, distance)) = queue.pop_front() {
            if current == diem_dich {
                return Some(distance); // Tìm thấy đích đến!
            }

            for &ke in &self.adjacency_list[current] {
                if !da_tham[ke] {
                    da_tham[ke] = true;
                    queue.push_back((ke, distance + 1));
                }
            }
        }

        None // Không có đường đi kết nối giữa hai đỉnh này
    }

    pub fn lay_ten(&self, chi_so: usize) -> &str {
        &self.name_all_peak[chi_so]
    }
}

impl Default for Graph {
    fn default() -> Self {
        Self::new()
    }
}

/// PHẦN 3: THUẬT TOÁN SẮP XẾP NHANH (QUICKSORT) TẠI CHỖ
pub fn quicksort<T: Ord>(data: &mut [T]) {
    if data.len() <= 1 {
        return;
    }
    let pivot_pos = part_region(data);
    // Chia đôi mảng và đệ quy sắp xếp hai nửa
    quicksort(&mut data[0..pivot_pos]);
    quicksort(&mut data[pivot_pos + 1..]);
}

fn part_region<T: Ord>(data: &mut [T]) -> usize {
    let length = data.len();
    let pivot_index = length - 1;
    let mut i = 0;

    for j in 0..pivot_index {
        if data[j] <= data[pivot_index] {
            data.swap(i, j);
            i += 1;
        }
    }
    data.swap(i, pivot_index);
    i
}

fn main() {
    println!("============================================================");
    println!("    BẢNG BĂM, ĐỒ THỊ VÀ CÁC THUẬT TOÁN CỐT LÕI TRONG RUST   ");
    println!("============================================================");

    // 1. Kiểm thử Bảng băm đếm tần suất từ
    println!("[1] Thống kê tần suất từ vựng bằng HashMap Entry API:");
    let van_ban = "học rust thật vui học lập trình rust thật tuyệt vời";
    let result_count = thong_ke_from_region(van_ban);
    for (tu, so_lan) in &result_count {
        println!("    - Từ '{:8}': xuất hiện {} lần", tu, so_lan);
    }
    assert_eq!(result_count.get("rust"), Some(&2));
    assert_eq!(result_count.get("học"), Some(&2));
    assert_eq!(result_count.get("vui"), Some(&1));

    // 2. Kiểm thử Mạng lưới Đồ thị và Thuật toán BFS
    println!("\n[2] Mô phỏng mạng xã hội kết nối bạn bè bằng Đồ thị & BFS:");
    let mut array_remote_hoi = Graph::new();
    let an = array_remote_hoi.add_peak("An");       // Đỉnh 0
    let binh = array_remote_hoi.add_peak("Bình");   // Đỉnh 1
    let chi = array_remote_hoi.add_peak("Chi");     // Đỉnh 2
    let dung = array_remote_hoi.add_peak("Dũng");   // Đỉnh 3
    let hoa = array_remote_hoi.add_peak("Hoa");     // Đỉnh 4 (ở xa)

    // Thiết lập các mối quan hệ bạn bè (Cạnh)
    // An quen Bình, Bình quen Chi, Chi quen Dũng, An quen Dũng (lối tắt)
    array_remote_hoi.add_edge(an, binh);
    array_remote_hoi.add_edge(binh, chi);
    array_remote_hoi.add_edge(chi, dung);
    array_remote_hoi.add_edge(an, dung); // Lối tắt trực tiếp từ An đến Dũng!

    println!("    - Tìm khoảng cách kết nối giữa '{}' và '{}':", array_remote_hoi.lay_ten(an), array_remote_hoi.lay_ten(chi));
    let distance_hidden_only = array_remote_hoi.bfs_shortest_distance(an, chi);
    println!("      => Khoảng cách ngắn nhất: {:?} chặng", distance_hidden_only);
    assert_eq!(distance_hidden_only, Some(2)); // An -> Bình -> Chi hoặc An -> Dũng -> Chi

    println!("    - Tìm khoảng cách kết nối giữa '{}' và '{}':", array_remote_hoi.lay_ten(an), array_remote_hoi.lay_ten(dung));
    let distance_hidden_use = array_remote_hoi.bfs_shortest_distance(an, dung);
    println!("      => Khoảng cách ngắn nhất: {:?} chặng (nhờ lối tắt trực tiếp!)", distance_hidden_use);
    assert_eq!(distance_hidden_use, Some(1));

    println!("    - Tìm khoảng cách đến '{}' (Chưa có kết nối):", array_remote_hoi.lay_ten(hoa));
    let distance_to_c = array_remote_hoi.bfs_shortest_distance(an, hoa);
    println!("      => Kết quả: {:?} (Không có đường đi)", distance_to_c);
    assert_eq!(distance_to_c, None);

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
mod tests {
    use super::*;

    #[test]
    fn word_frequency_count() {
        let bang = thong_ke_from_region("rust rust an toan rust");
        assert_eq!(bang.get("rust"), Some(&3));
        assert_eq!(bang.get("an"), Some(&1));
        assert_eq!(bang.get("khong-co"), None);
    }

    #[test]
    fn quicksort_matches_std_sort() {
        let mut a = vec![5, 2, 9, 1, 5, 6, 3, 3, 8];
        let mut b = a.clone();
        quicksort(&mut a);
        b.sort();
        assert_eq!(a, b); // kiểm chứng chéo với thư viện chuẩn
    }

    #[test]
    fn quicksort_edge_cases() {
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
    fn bfs_finds_shortest_path() {
        let mut g = Graph::new();
        let a = g.add_peak("A");
        let b = g.add_peak("B");
        let c = g.add_peak("C");
        let d = g.add_peak("D");
        g.add_edge(a, b);
        g.add_edge(b, c);
        g.add_edge(a, d);
        g.add_edge(d, c);
        assert_eq!(g.bfs_shortest_distance(a, c), Some(2));
        assert_eq!(g.bfs_shortest_distance(a, a), Some(0));
    }

    #[test]
    fn bfs_reports_no_path() {
        let mut g = Graph::new();
        let a = g.add_peak("A");
        let b = g.add_peak("B"); // cô lập
        assert_eq!(g.bfs_shortest_distance(a, b), None);
    }
}
