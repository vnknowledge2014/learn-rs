#![allow(dead_code, unused_variables, unused_imports)]
use std::cmp::Reverse;
use std::collections::{BinaryHeap, HashMap, VecDeque};

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

/// PHẦN 4: UNION-FIND (DISJOINT SET UNION) VỚI NÉN ĐƯỜNG VÀ HỢP THEO HẠNG
///
/// Cấu trúc dữ liệu quản lý các tập hợp rời rạc, hỗ trợ hai thao tác chính:
/// - `find`: tìm đại diện (gốc) của tập chứa phần tử x.
/// - `union`: hợp nhất hai tập hợp chứa x và y.
///
/// Nhờ kỹ thuật nén đường (path compression) và hợp theo hạng (union by rank),
/// mỗi thao tác có độ phức tạp gần như O(α(N)) — hằng số trong thực tế.
///
/// Ứng dụng thực chiến: phát hiện chu trình, thuật toán Kruskal MST,
/// bài toán kết nối mạng, phân nhóm (clustering).
pub struct UnionFind {
    /// Mảng cha — nếu parent[i] == i thì i là gốc.
    parent: Vec<usize>,
    /// Hạng (rank) để giữ cây phẳng khi hợp nhất.
    rank: Vec<usize>,
}

impl UnionFind {
    /// Khởi tạo n tập rời rạc, mỗi phần tử là gốc của chính nó.
    pub fn new(n: usize) -> Self {
        UnionFind {
            parent: (0..n).collect(),
            rank: vec![0; n],
        }
    }

    /// Tìm gốc đại diện của tập chứa x, đồng thời nén đường
    /// để mọi nút trên đường đi trỏ thẳng về gốc.
    pub fn find(&mut self, x: usize) -> usize {
        if self.parent[x] != x {
            self.parent[x] = self.find(self.parent[x]);
        }
        self.parent[x]
    }

    /// Hợp nhất hai tập chứa x và y. Trả về `false` nếu x, y đã cùng tập.
    /// Dùng hạng (rank) để luôn gắn cây thấp vào cây cao, giữ cây phẳng.
    pub fn union(&mut self, x: usize, y: usize) -> bool {
        let root_x = self.find(x);
        let root_y = self.find(y);
        if root_x == root_y {
            return false;
        }
        match self.rank[root_x].cmp(&self.rank[root_y]) {
            std::cmp::Ordering::Less => self.parent[root_x] = root_y,
            std::cmp::Ordering::Greater => self.parent[root_y] = root_x,
            std::cmp::Ordering::Equal => {
                self.parent[root_y] = root_x;
                self.rank[root_x] += 1;
            }
        }
        true
    }

    /// Kiểm tra x và y có cùng tập hợp hay không.
    pub fn connected(&mut self, x: usize, y: usize) -> bool {
        self.find(x) == self.find(y)
    }
}

/// PHẦN 5: ĐỒ THỊ CÓ TRỌNG SỐ VÀ THUẬT TOÁN DIJKSTRA TÌM ĐƯỜNG NGẮN NHẤT
///
/// Sử dụng BinaryHeap với Reverse để tạo min-heap, mô phỏng hàng đợi ưu tiên.
/// Độ phức tạp: O((V + E) log V) — hiệu quả cho đồ thị thưa.
///
/// Ứng dụng: tìm đường đi ngắn nhất trong bản đồ giao thông, mạng máy tính,
/// định tuyến gói tin (routing).
pub struct WeightedGraph {
    /// Danh sách kề có trọng số: adj[u] = vec![(v, weight), ...]
    adj: Vec<Vec<(usize, u64)>>,
}

impl WeightedGraph {
    /// Khởi tạo đồ thị có trọng số với n đỉnh, chưa có cạnh nào.
    pub fn new(n: usize) -> Self {
        WeightedGraph {
            adj: vec![Vec::new(); n],
        }
    }

    /// Thêm cạnh có hướng từ u đến v với trọng số w.
    pub fn add_edge(&mut self, u: usize, v: usize, w: u64) {
        self.adj[u].push((v, w));
    }

    /// Thêm cạnh vô hướng giữa u và v với trọng số w.
    pub fn add_undirected_edge(&mut self, u: usize, v: usize, w: u64) {
        self.adj[u].push((v, w));
        self.adj[v].push((u, w));
    }

    /// Thuật toán Dijkstra tìm khoảng cách ngắn nhất từ đỉnh `start`
    /// đến tất cả các đỉnh còn lại.
    ///
    /// Trả về Vec<Option<u64>>: phần tử thứ i là khoảng cách ngắn nhất
    /// từ start đến i, hoặc None nếu không thể đến được.
    ///
    /// Dùng BinaryHeap<Reverse<(dist, node)>> làm min-heap.
    /// Độ phức tạp: O((V + E) log V).
    pub fn dijkstra(&self, start: usize) -> Vec<Option<u64>> {
        let n = self.adj.len();
        let mut dist: Vec<Option<u64>> = vec![None; n];
        dist[start] = Some(0);

        // Min-heap: phần tử nhỏ nhất (khoảng cách ngắn nhất) được lấy ra trước.
        let mut heap: BinaryHeap<Reverse<(u64, usize)>> = BinaryHeap::new();
        heap.push(Reverse((0, start)));

        while let Some(Reverse((cost, u))) = heap.pop() {
            // Nếu đã tìm được đường ngắn hơn rồi thì bỏ qua.
            if let Some(d) = dist[u]
                && cost > d
            {
                continue;
            }

            for &(v, w) in &self.adj[u] {
                let new_dist = cost + w;
                let is_shorter = match dist[v] {
                    None => true,
                    Some(old) => new_dist < old,
                };
                if is_shorter {
                    dist[v] = Some(new_dist);
                    heap.push(Reverse((new_dist, v)));
                }
            }
        }

        dist
    }
}

/// PHẦN 6: CẠNH CÓ TRỌNG SỐ VÀ THUẬT TOÁN KRUSKAL TÌM CÂY KHUNG NHỎ NHẤT (MST)
///
/// Thuật toán Kruskal:
/// 1. Sắp xếp tất cả các cạnh theo trọng số tăng dần.
/// 2. Duyệt từng cạnh, nếu hai đầu chưa cùng tập (dùng UnionFind) thì thêm
///    cạnh đó vào MST.
///
/// Độ phức tạp: O(E log E) do bước sắp xếp.
///
/// Ứng dụng: thiết kế mạng cáp quang chi phí thấp nhất, phân cụm (clustering),
/// xấp xỉ bài toán người du lịch (TSP).
pub struct Edge {
    /// Đỉnh nguồn.
    pub u: usize,
    /// Đỉnh đích.
    pub v: usize,
    /// Trọng số cạnh.
    pub weight: u64,
}

/// Tìm cây khung nhỏ nhất (MST) bằng thuật toán Kruskal.
///
/// Nhận số đỉnh và một slice cạnh có thể sắp xếp.
/// Trả về (tổng trọng số MST, danh sách tham chiếu các cạnh trong MST).
///
/// Nếu đồ thị không liên thông, trả về rừng khung nhỏ nhất (partial MST)
/// bao gồm các cây khung của từng thành phần liên thông.
pub fn kruskal_mst(num_vertices: usize, edges: &mut [Edge]) -> (u64, Vec<&Edge>) {
    // Sắp xếp cạnh theo trọng số tăng dần.
    edges.sort_by_key(|e| e.weight);

    let mut uf = UnionFind::new(num_vertices);
    let mut total_weight: u64 = 0;
    let mut mst_edges: Vec<&Edge> = Vec::new();

    for edge in edges.iter() {
        // Chỉ thêm cạnh nếu hai đầu chưa cùng thành phần liên thông.
        if uf.union(edge.u, edge.v) {
            total_weight += edge.weight;
            mst_edges.push(edge);
        }
    }

    (total_weight, mst_edges)
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

    // ========== UNION-FIND TESTS ==========

    #[test]
    fn test_union_find_basic() {
        // Tạo 5 phần tử, hợp nhất một số, kiểm tra tính liên thông.
        let mut uf = UnionFind::new(5);
        assert!(!uf.connected(0, 1));
        assert!(!uf.connected(2, 3));

        assert!(uf.union(0, 1)); // Hợp 0-1
        assert!(uf.union(2, 3)); // Hợp 2-3
        assert!(uf.connected(0, 1));
        assert!(uf.connected(2, 3));
        assert!(!uf.connected(0, 2));

        assert!(uf.union(1, 3)); // Hợp hai nhóm lại
        assert!(uf.connected(0, 3));
        assert!(uf.connected(1, 2));

        // Phần tử 4 vẫn cô lập
        assert!(!uf.connected(0, 4));
    }

    #[test]
    fn test_union_find_already_connected() {
        // Hợp nhất các phần tử đã cùng tập phải trả về false.
        let mut uf = UnionFind::new(3);
        assert!(uf.union(0, 1)); // Lần đầu: thành công
        assert!(!uf.union(0, 1)); // Lần hai: đã cùng tập
        assert!(!uf.union(1, 0)); // Thứ tự ngược cũng vậy

        assert!(uf.union(1, 2));
        assert!(!uf.union(0, 2)); // 0-1-2 đều cùng tập rồi
    }

    // ========== DIJKSTRA TESTS ==========

    #[test]
    fn test_dijkstra_simple() {
        // Đồ thị tam giác: 0 --1--> 1 --2--> 2, và 0 --10--> 2.
        // Đường ngắn nhất 0->2 là 0->1->2 = 3, không phải 0->2 = 10.
        let mut g = WeightedGraph::new(3);
        g.add_edge(0, 1, 1);
        g.add_edge(1, 2, 2);
        g.add_edge(0, 2, 10);

        let dist = g.dijkstra(0);
        assert_eq!(dist[0], Some(0));
        assert_eq!(dist[1], Some(1));
        assert_eq!(dist[2], Some(3)); // 0->1->2 = 1+2 = 3
    }

    #[test]
    fn test_dijkstra_unreachable() {
        // Đỉnh 2 không có cạnh nào nối tới từ 0 hoặc 1.
        let mut g = WeightedGraph::new(3);
        g.add_edge(0, 1, 5);

        let dist = g.dijkstra(0);
        assert_eq!(dist[0], Some(0));
        assert_eq!(dist[1], Some(5));
        assert_eq!(dist[2], None); // Không thể đến được
    }

    #[test]
    fn test_dijkstra_single_node() {
        // Đồ thị chỉ có 1 đỉnh, không có cạnh.
        let g = WeightedGraph::new(1);
        let dist = g.dijkstra(0);
        assert_eq!(dist.len(), 1);
        assert_eq!(dist[0], Some(0));
    }

    // ========== KRUSKAL MST TESTS ==========

    #[test]
    fn test_kruskal_simple() {
        //   0 ---1--- 1
        //   |         |
        //   4         2
        //   |         |
        //   3 ---3--- 2
        // MST nên chọn cạnh 0-1(1), 1-2(2), 2-3(3) => tổng = 6
        let mut edges = vec![
            Edge {
                u: 0,
                v: 1,
                weight: 1,
            },
            Edge {
                u: 1,
                v: 2,
                weight: 2,
            },
            Edge {
                u: 2,
                v: 3,
                weight: 3,
            },
            Edge {
                u: 0,
                v: 3,
                weight: 4,
            },
        ];

        let (total, mst_edges) = kruskal_mst(4, &mut edges);
        assert_eq!(total, 6);
        assert_eq!(mst_edges.len(), 3); // MST với 4 đỉnh có 3 cạnh
    }

    #[test]
    fn test_kruskal_disconnected() {
        // Hai thành phần liên thông tách rời: {0,1} và {2,3}.
        // Kruskal trả về rừng khung nhỏ nhất (2 cạnh thay vì 3).
        let mut edges = vec![
            Edge {
                u: 0,
                v: 1,
                weight: 5,
            },
            Edge {
                u: 2,
                v: 3,
                weight: 7,
            },
        ];

        let (total, mst_edges) = kruskal_mst(4, &mut edges);
        assert_eq!(total, 12); // 5 + 7
        assert_eq!(mst_edges.len(), 2); // Chỉ 2 cạnh (không đủ nối 4 đỉnh)
    }
}
