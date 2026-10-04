#![allow(dead_code, unused_variables, unused_imports)]
use std::collections::{HashMap, VecDeque};

/// PHẦN 1: THỐNG KÊ TẦN SUẤT TỪ VỚI BẢNG BĂM HASHMAP
pub fn word_frequencies(text: &str) -> HashMap<String, usize> {
    let mut counts = HashMap::new();
    for word in text.split_whitespace() {
        // Chuẩn hóa từ về chữ thường
        let normalized = word.to_lowercase();
        // Entry API: Tra cứu một lần, nếu chưa có thì khởi tạo giá trị 0, sau đó tăng 1
        let count = counts.entry(normalized).or_insert(0);
        *count += 1;
    }
    counts
}

/// PHẦN 2: CẤU TRÚC ĐỒ THỊ AN TOÀN VÀ THUẬT TOÁN BFS
pub struct Graph {
    adjacency_list: Vec<Vec<usize>>,
    vertex_names: Vec<String>,
}

impl Graph {
    pub fn new() -> Self {
        Graph {
            adjacency_list: Vec::new(),
            vertex_names: Vec::new(),
        }
    }

    /// Thêm một đỉnh mới vào đồ thị và trả về chỉ số của đỉnh đó
    pub fn add_vertex(&mut self, name: &str) -> usize {
        let index = self.vertex_names.len();
        self.vertex_names.push(name.to_string());
        self.adjacency_list.push(Vec::new());
        index
    }

    /// Thêm một cạnh nối hai chiều giữa hai đỉnh u và v
    pub fn add_edge(&mut self, u: usize, v: usize) {
        if u < self.adjacency_list.len() && v < self.adjacency_list.len() {
            self.adjacency_list[u].push(v);
            self.adjacency_list[v].push(u); // Đồ thị vô hướng 2 chiều
        }
    }

    /// Thuật toán BFS tìm đường đi ngắn nhất (Số chặng) giữa hai đỉnh
    pub fn bfs_shortest_distance(&self, start: usize, target: usize) -> Option<usize> {
        if start >= self.adjacency_list.len() || target >= self.adjacency_list.len() {
            return None;
        }

        // Mảng đánh dấu các đỉnh đã thăm để tránh chu trình lặp vô tận
        let mut visited = vec![false; self.adjacency_list.len()];
        // Hàng đợi lưu cặp (chỉ_số_đỉnh, khoảng_cách)
        let mut queue: VecDeque<(usize, usize)> = VecDeque::new();

        visited[start] = true;
        queue.push_back((start, 0));

        while let Some((current, distance)) = queue.pop_front() {
            if current == target {
                return Some(distance); // Tìm thấy đích đến!
            }

            for &neighbor in &self.adjacency_list[current] {
                if !visited[neighbor] {
                    visited[neighbor] = true;
                    queue.push_back((neighbor, distance + 1));
                }
            }
        }

        None // Không có đường đi kết nối giữa hai đỉnh này
    }

    pub fn vertex_name(&self, index: usize) -> &str {
        &self.vertex_names[index]
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
    let pivot_pos = partition(data);
    // Chia đôi mảng và đệ quy sắp xếp hai nửa
    quicksort(&mut data[0..pivot_pos]);
    quicksort(&mut data[pivot_pos + 1..]);
}

fn partition<T: Ord>(data: &mut [T]) -> usize {
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
    let text = "học rust thật vui học lập trình rust thật tuyệt vời";
    let frequencies = word_frequencies(text);
    for (word, count) in &frequencies {
        println!("    - Từ '{:8}': xuất hiện {} lần", word, count);
    }
    assert_eq!(frequencies.get("rust"), Some(&2));
    assert_eq!(frequencies.get("học"), Some(&2));
    assert_eq!(frequencies.get("vui"), Some(&1));

    // 2. Kiểm thử Mạng lưới Đồ thị và Thuật toán BFS
    println!("\n[2] Mô phỏng mạng xã hội kết nối bạn bè bằng Đồ thị & BFS:");
    let mut social_network = Graph::new();
    let an = social_network.add_vertex("An"); // Đỉnh 0
    let binh = social_network.add_vertex("Bình"); // Đỉnh 1
    let chi = social_network.add_vertex("Chi"); // Đỉnh 2
    let dung = social_network.add_vertex("Dũng"); // Đỉnh 3
    let hoa = social_network.add_vertex("Hoa"); // Đỉnh 4 (ở xa)

    // Thiết lập các mối quan hệ bạn bè (Cạnh)
    // An quen Bình, Bình quen Chi, Chi quen Dũng, An quen Dũng (lối tắt)
    social_network.add_edge(an, binh);
    social_network.add_edge(binh, chi);
    social_network.add_edge(chi, dung);
    social_network.add_edge(an, dung); // Lối tắt trực tiếp từ An đến Dũng!

    println!(
        "    - Tìm khoảng cách kết nối giữa '{}' và '{}':",
        social_network.vertex_name(an),
        social_network.vertex_name(chi)
    );
    let dist_an_chi = social_network.bfs_shortest_distance(an, chi);
    println!("      => Khoảng cách ngắn nhất: {:?} chặng", dist_an_chi);
    assert_eq!(dist_an_chi, Some(2)); // An -> Bình -> Chi hoặc An -> Dũng -> Chi

    println!(
        "    - Tìm khoảng cách kết nối giữa '{}' và '{}':",
        social_network.vertex_name(an),
        social_network.vertex_name(dung)
    );
    let dist_an_dung = social_network.bfs_shortest_distance(an, dung);
    println!(
        "      => Khoảng cách ngắn nhất: {:?} chặng (nhờ lối tắt trực tiếp!)",
        dist_an_dung
    );
    assert_eq!(dist_an_dung, Some(1));

    println!(
        "    - Tìm khoảng cách đến '{}' (Chưa có kết nối):",
        social_network.vertex_name(hoa)
    );
    let dist_an_hoa = social_network.bfs_shortest_distance(an, hoa);
    println!("      => Kết quả: {:?} (Không có đường đi)", dist_an_hoa);
    assert_eq!(dist_an_hoa, None);

    // 3. Kiểm thử Thuật toán Sắp xếp nhanh Quicksort
    println!("\n[3] Kiểm thử Thuật toán Sắp xếp nhanh Quicksort tại chỗ:");
    let mut numbers = [42, 12, 88, 5, 63, 19, 77, 3];
    println!("    - Mảng trước khi sắp xếp: {:?}", numbers);
    quicksort(&mut numbers);
    println!("    - Mảng sau khi sắp xếp   : {:?}", numbers);
    assert_eq!(numbers, [3, 5, 12, 19, 42, 63, 77, 88]);
    println!("    => Quicksort O(N log N) hoàn tất thành công!");

    println!("============================================================");
    println!("               HOÀN TẤT THỰC NGHIỆM CHƯƠNG 30               ");
    println!("============================================================");
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn word_frequency_count() {
        let freq = word_frequencies("rust rust an toàn rust");
        assert_eq!(freq.get("rust"), Some(&3));
        assert_eq!(freq.get("an"), Some(&1));
        assert_eq!(freq.get("không-có"), None);
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
        let mut empty: Vec<i32> = vec![];
        quicksort(&mut empty);
        assert!(empty.is_empty());

        let mut single = vec![42];
        quicksort(&mut single);
        assert_eq!(single, vec![42]);

        // Trường hợp XẤU NHẤT O(N^2): mảng đã sắp xếp sẵn — vẫn phải đúng
        let mut sorted: Vec<i32> = (1..=100).collect();
        quicksort(&mut sorted);
        assert_eq!(sorted, (1..=100).collect::<Vec<i32>>());
    }

    #[test]
    fn bfs_finds_shortest_path() {
        let mut g = Graph::new();
        let a = g.add_vertex("A");
        let b = g.add_vertex("B");
        let c = g.add_vertex("C");
        let d = g.add_vertex("D");
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
        let a = g.add_vertex("A");
        let b = g.add_vertex("B"); // cô lập
        assert_eq!(g.bfs_shortest_distance(a, b), None);
    }
}
