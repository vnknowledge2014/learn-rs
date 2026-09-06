# Chương 30: Bảng băm, Đồ thị & Các thuật toán tìm kiếm, sắp xếp cốt lõi (Hash Tables, Graphs & Core Search/Sort Algorithms)

## Giới thiệu & Mục tiêu học tập

Chào mừng bạn đến với chương kết thúc của **Chủ đề 5: Cấu trúc dữ liệu & Giải thuật trong Rust**! Đến thời điểm này, bạn đã nắm vững từ các cấu trúc tuyến tính (Mảng, Vector, Danh sách liên kết (Linked list), Ngăn xếp, Hàng đợi) đến các cấu trúc phân cấp cây nhị phân. Trong chương này, chúng ta sẽ làm chủ hai cấu trúc dữ liệu và giải thuật tối thượng của ngành khoa học máy tính: **Bảng băm (Hash Table)** và **Đồ thị (Graph)**, cùng hai thuật toán kinh điển đi kèm là **Tìm kiếm theo chiều rộng (BFS)** và **Sắp xếp nhanh (Quicksort)**.

Nếu như Mảng cho phép truy cập $O(1)$ nhưng phải thông qua số thứ tự, thì Bảng băm (`HashMap`) mang lại phép màu: **Tra cứu dữ liệu bất kỳ bằng từ khóa (Key) bằng chữ trong thời gian tức thì $O(1)$**! Bảng băm là trái tim của mọi hệ thống bộ nhớ đệm (buffer cache), hệ thống từ điển, và cơ sở dữ liệu khóa-giá trị (Key-Value Store).

Trong khi đó, Đồ thị (Graph) là mô hình mạnh mẽ nhất để biểu diễn các mối quan hệ đa chiều trong thế giới thực: Mạng xã hội kết nối bạn bè, bản đồ giao thông đường bộ, mạng lưới các máy chủ Internet, hay chuỗi phụ thuộc giữa các gói thư viện (crate dependencies) trong Cargo. Chúng ta sẽ khám phá cách biểu diễn đồ thị cực kỳ thanh lịch và an toàn bằng Rust mà không sợ vướng vào "cuộc chiến" với trình kiểm tra mượn (Borrow Checker).

Mục tiêu học tập của chương này:
- Nắm vững cơ chế vận hành của **Bảng băm (Hash Table)**: Hàm băm (Hash function), phân phối xô ô nhớ (Buckets), và nghệ thuật sử dụng Entry API (`.entry().or_insert()`).
- Hiểu cấu trúc **Đồ thị (Graph)**: Đỉnh (Vertex/Node), Cạnh (Edge), Đồ thị có hướng vs Vô hướng.
- Làm chủ kỹ thuật biểu diễn Đồ thị an toàn 100% bằng **Danh sách kề dùng chỉ số (Index-based Adjacency List)** thay vì con trỏ chéo.
- Cài đặt và ứng dụng thuật toán **Tìm kiếm theo chiều rộng (Breadth-First Search - BFS)** để tìm đường đi ngắn nhất giữa hai nút.
- Cài đặt thuật toán **Sắp xếp nhanh (Quicksort)** tại chỗ (in-place) trên lát cắt mượn `&mut [T]` với độ phức tạp $O(N \log N)$.

---

## Hình tượng hóa đời sống (Intuitive Everyday Analogy)

Hãy quan sát hai hình ảnh vô cùng sinh động trong đời sống thực tế:

```
┌──────────────────────────────────────────────────────────────────────────────────┐
│              HÌNH TƯỢNG HÓA BẢNG BĂM (HASH TABLE) VÀ ĐỒ THỊ (GRAPH)              │
├──────────────────────────────────────────────────────────────────────────────────┤
│                                                                                  │
│ [1. BẢNG BĂM: HÒM THƯ BƯU ĐIỆN VÀ ĐẦU ĐỌC QUÉT MÃ VẠCH]                         │
│                                                                                  │
│ Tên người nhận: "Nguyễn Văn An"                                                  │
│        │                                                                         │
│        ▼ [Máy quét mã vạch (Hàm băm Hash)]                                       │
│    Mã số tính ra: Hộp #42                                                        │
│        │                                                                         │
│        ▼                                                                         │
│ ┌─────────┬─────────┬─────────┬─────────┬─────────┐                              │
│ │ Hộp #40 │ Hộp #41 │ Hộp #42 │ Hộp #43 │ Hộp #44 │ -> Mở đúng hộp #42 tốn 1 giây│
│ │ [Trống] │ [Trống] │ [Thư từ]│ [Trống] │ [Trống] │    Bất kể bưu điện có vạn hộp│
│ └─────────┴─────────┴─────────┴─────────┴─────────┘                              │
│                                                                                  │
│ [2. ĐỒ THỊ: MẠNG LƯỚI BẢN ĐỒ TÀU ĐIỆN NGẦM ĐÔ THỊ]                               │
│                                                                                  │
│      [Trạm Bến Thành (0)] ══════════════ [Trạm Nhà Hát (1)]                      │
│               ║                                 ║                                │
│               ║ Tuyến 1                         ║ Tuyến 2                        │
│               ║                                 ║                                │
│      [Trạm Chợ Lớn (2)]   ══════════════ [Trạm Tân Bình (3)]                     │
│                                                                                  │
│ - Mỗi trạm dừng chân là một ĐỈNH (Vertex).                                       │
│ - Mỗi đường ray kết nối giữa 2 trạm là một CẠNH (Edge).                          │
│ - Muốn đi từ Bến Thành đến Tân Bình nhanh nhất? Dùng thuật toán BFS!             │
└──────────────────────────────────────────────────────────────────────────────────┘
```

### 1. Bảng băm — Hệ thống phân loại thư tại bưu điện
- Khi người đưa thư cầm một lá thư gửi tới cho "Nguyễn Văn An":
  - Thay vì đi gõ cửa từng căn nhà trong thành phố để hỏi ($O(N)$ tìm kiếm tuần tự), người đưa thư đưa bức thư qua một **Máy quét mã bưu chính (Hàm băm - Hash Function)**.
  - Chiếc máy lập tức tính toán ra một con số toán học: Ví dụ số **42**.
  - Người đưa thư chỉ việc bước thẳng tới **Hộc tủ số 42** và nhét bức thư vào đó.
  - Khi người nhận tới lấy thư, họ cũng đưa căn cước quét qua máy, máy báo hộc số 42, mở tủ lấy thư mất đúng 1 giây ($O(1)$).
  - Dù bưu điện có 100 hộc tủ hay 1 triệu hộc tủ, thời gian lấy thư vẫn không hề thay đổi!

### 2. Đồ thị — Bản đồ mạng lưới xe buýt / Tàu điện ngầm
- Hãy nhìn vào bản đồ giao thông của một thành phố:
  - Các nhà ga, bến xe, hoặc các nút giao là các **Đỉnh (Vertices)**.
  - Các đoạn đường nối giữa hai địa điểm là các **Cạnh (Edges)**.
- Khác với Cây (nơi chỉ có quan hệ cha-con một chiều và không có vòng lặp), Đồ thị cho phép các con đường đan xen chằng chịt, có thể quay vòng lại điểm xuất phát (chu trình - Cycle).
- **Thuật toán BFS (Tìm kiếm theo chiều rộng)** giống như việc bạn ném một viên sỏi xuống mặt hồ nước phẳng lặng: Sóng nước sẽ lan tỏa đều ra xung quanh theo từng vòng tròn đồng tâm: Đầu tiên là các trạm cách bạn 1 chặng đi, sau đó là các trạm cách 2 chặng, rồi 3 chặng... Nhờ cơ chế lan tỏa từng lớp này, lần đầu tiên bạn chạm tới điểm đích cũng chính là con đường ngắn nhất!

---

## Khái niệm & Cơ chế kỹ thuật chuyên sâu (In-Depth Technical Mechanics)

### 1. Bản chất của Bảng băm (`HashMap`) trong Rust

Trong Rust, `HashMap<K, V>` được xây dựng dựa trên thuật toán **SwissTable** (nằm trong thư viện nổi tiếng `hashbrown` được tích hợp thẳng vào thư viện chuẩn `std::collections`):
1. **Hàm băm (Hash Function)**: Mặc định Rust sử dụng thuật toán `SipHash 1-3`, một hàm băm mật mã học được thiết kế đặc biệt để ngăn chặn các cuộc tấn công từ chối dịch vụ **HashDoS** (khi kẻ tấn công cố tình tạo ra hàng triệu khóa có cùng giá trị băm để làm bảng băm suy biến về danh sách liên kết $O(N)$).
2. **Kiểm soát nhóm xô (Group of Buckets & SIMD Control Bytes)**: SwissTable sử dụng các byte điều khiển và các lệnh vi xử lý song song SIMD để kiểm tra cùng lúc 16 xô ô nhớ trong 1 chu kỳ CPU, mang lại tốc độ tra cứu khủng khiếp.
3. **Tuyệt chiêu Entry API**: Thay vì kiểm tra xem khóa có tồn tại rồi mới chèn (tốn 2 lần băm dữ liệu), Rust cung cấp cú pháp `entry(key)`:
   ```rust
   let mut word_count = std::collections::HashMap::new();
   let tu = "rust";
   // Đếm số lần xuất hiện của từ chỉ với 1 lần tính băm duy nhất!
   *word_count.entry(tu).or_insert(0) += 1;
   ```

### 2. Giải mã bí mật: Biểu diễn Đồ thị không sợ Borrow Checker

Nếu bạn cố gắng tạo một nút đồ thị chứa các con trỏ trỏ trực tiếp sang các nút khác trong Rust (`struct Node { neighbors: Vec<Rc<RefCell<Node>>> }`), bạn sẽ sớm rơi vào "địa ngục con trỏ": Mã nguồn trở nên rối rắm, bộ nhớ bị rò rỉ do các liên kết vòng (Reference Cycles) ngăn cản cơ chế giải phóng tự động.

**Phương pháp chuẩn công nghiệp trong Rust: Danh sách kề dùng chỉ số (Index-based Adjacency List)**:
- Toàn bộ các đỉnh trong đồ thị được đánh số thứ tự từ `0, 1, 2, ...` và lưu trong một `Vec`.
- Mỗi đỉnh chỉ cần lưu một danh sách các số nguyên đại diện cho các đỉnh láng giềng: `Vec<Vec<usize>>`.
```
Đỉnh 0 (Bến Thành) -> Kề với: [1, 2]
Đỉnh 1 (Nhà Hát)   -> Kề với: [0, 3]
Đỉnh 2 (Chợ Lớn)   -> Kề với: [0, 3]
Đỉnh 3 (Tân Bình)  -> Kề với: [1, 2]
```
Mã nguồn giờ đây là **Safe Rust 100%**: Không có con trỏ, không có `unsafe`, không sợ Borrow Checker, và tốc độ truy cập đạt đỉnh cao nhờ tính liên tục của bộ nhớ RAM!

### 3. Thuật toán Sắp xếp nhanh (Quicksort - trung bình $O(N \log N)$, xấu nhất $O(N^2)$)

Quicksort là một trong những thuật toán sắp xếp thực chiến hiệu quả nhất lịch sử:
1. **Chọn phần tử chốt (Pivot)**: Chọn một phần tử bất kỳ (ví dụ phần tử cuối cùng của mảng).
2. **Phân vùng (Partitioning)**: Duyệt qua mảng và dồn tất cả các phần tử nhỏ hơn chốt về bên trái, các phần tử lớn hơn chốt về bên phải. Đặt phần tử chốt vào đúng vị trí ranh giới chính giữa.
3. **Đệ quy**: Lặp lại quy trình trên cho hai nửa mảng bên trái và bên phải cho đến khi toàn bộ mảng được sắp xếp hoàn tất.

> **Cạm bẫy phải biết — Quicksort KHÔNG phải lúc nào cũng $O(N \log N)$:**
> Con số $O(N \log N)$ chỉ đúng ở **trường hợp trung bình**, khi phần tử chốt chia mảng thành hai nửa tương đối cân bằng.
> Nếu bạn luôn chọn phần tử cuối làm chốt và đưa vào một mảng **đã được sắp xếp sẵn**, mỗi lần phân vùng chỉ tách ra được 1 phần tử —
> cây đệ quy suy biến thành một chuỗi thẳng đúng như hiện tượng *Cây suy biến* ở Chương 29, và độ phức tạp tụt xuống **$O(N^2)$**.
> Cách hóa giải trong thực chiến: chọn chốt ngẫu nhiên, hoặc dùng kỹ thuật "trung vị của ba" (median-of-three).
> Đây cũng là lý do `slice::sort()` của thư viện chuẩn Rust dùng thuật toán lai **Timsort** (ổn định, $O(N \log N)$ ở mọi trường hợp),
> còn `slice::sort_unstable()` dùng **pattern-defeating quicksort** — một biến thể tự động phát hiện và thoát khỏi trường hợp xấu nhất.

---

## Mã nguồn minh họa thực chiến (Idiomatic Runnable Rust Blueprint)

Dưới đây là một chương trình Rust hoàn chỉnh và độc lập, minh họa trọn vẹn ba nội dung cốt lõi:
1. Ứng dụng Bảng băm (`HashMap`) thống kê tần suất từ vựng với Entry API.
2. Cài đặt Đồ thị bằng danh sách kề chỉ số và chạy thuật toán BFS tìm khoảng cách ngắn nhất.
3. Cài đặt thuật toán Sắp xếp nhanh (Quicksort) in-place trên lát cắt mượn `&mut [T]`:

```rust
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
        let mut visited = vec![false; self.adjacency_list.len()];
        // Hàng đợi lưu cặp (chỉ_số_đỉnh, khoảng_cách)
        let mut queue: VecDeque<(usize, usize)> = VecDeque::new();

        visited[diem_dau] = true;
        queue.push_back((diem_dau, 0));

        while let Some((current, distance)) = queue.pop_front() {
            if current == diem_dich {
                return Some(distance); // Tìm thấy đích đến!
            }

            for &ke in &self.adjacency_list[current] {
                if !visited[ke] {
                    visited[ke] = true;
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
    let uppercase = array_remote_hoi.add_peak("Hoa");     // Đỉnh 4 (ở xa)

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

    println!("    - Tìm khoảng cách đến '{}' (Chưa có kết nối):", array_remote_hoi.lay_ten(uppercase));
    let distance_to_c = array_remote_hoi.bfs_shortest_distance(an, uppercase);
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
```

---

## Bảng tra cứu lỗi biên dịch & Cách khắc phục (Compiler Error Guide)

Dưới đây là các lỗi biên dịch điển hình nhất khi lập trình Bảng băm và Đồ thị trong Rust:

| Mã lỗi | Thông báo mẫu từ trình biên dịch | Nguyên nhân cốt lõi | Cách khắc phục nhanh |
|---|---|---|---|
| **E0277** | `the trait bound 'K: Hash' is not satisfied` | Bạn sử dụng một kiểu dữ liệu tự định nghĩa làm Khóa (Key) cho `HashMap` nhưng kiểu đó chưa cài đặt trait `Hash` và `Eq`. | Thêm chỉ dẫn derive tự động: `#[derive(Hash, PartialEq, Eq)]` phía trên khai báo struct. |
| **E0502** | `cannot borrow '...' as mutable because it is also borrowed as immutable` | Bạn đang lặp qua danh sách láng giềng mượn bất biến `&graph.adjacency_list[u]` nhưng bên trong thân vòng lặp lại gọi `graph.add_edge()` làm thay đổi đồ thị. | Thu thập các chỉ số cần biến đổi vào một vector tạm trước khi thực hiện ghi đè. |
| **E0382** | `use of moved value: 'tu'` | Bạn gọi `table_count.insert(tu, 1)` khiến chuỗi `tu` bị di chuyển quyền sở hữu (ownership), sau đó lại dùng lại `tu` ở dòng lệnh tiếp theo. | Dùng phương thức `.clone()` tạo bản sao độc lập, hoặc lưu tham chiếu mượn chuỗi `&str` nếu chuỗi có thời gian sống (lifetime) dài hơn bảng băm. |
| **E0308** | `mismatched types: expected '&str', found 'String'` | Bạn truyền một giá trị sở hữu `String` vào phương thức tra cứu `.get()` của HashMap vốn chỉ đòi hỏi một lát cắt tham chiếu `&str`. | Thêm dấu `&` phía trước biến chuỗi: `table_count.get(&tu)`. |

### Ví dụ phân tích lỗi `E0277` khi dùng struct làm khóa cho `HashMap`:

```rust
// Struct chưa thỏa mãn trait Hash và Eq
struct UserBroken {
    id: u32,
}

fn broken_hash() {
    let mut bang_hash = std::collections::HashMap::new();
    // bang_hash.insert(UserBroken { id: 1 }, "Admin"); // LỖI E0277!
}

// Cách sửa chữa đúng chuẩn: Derive đầy đủ PartialEq, Eq, Hash
#[derive(Hash, PartialEq, Eq, Debug)]
struct UserIdiomatic {
    id: u32,
}

fn correct_hash() {
    let mut bang_hash = std::collections::HashMap::new();
    bang_hash.insert(UserIdiomatic { id: 1 }, "Admin");
    println!("Tra cứu khóa người dùng thành công: {:?}", bang_hash.get(&UserIdiomatic { id: 1 }));
}
```

---



---

## Kiểm thử tự động (Automated Tests)

Cấu trúc dữ liệu và thuật toán là nơi kiểm thử tỏ ra hữu ích nhất: một lỗi ở biên (mảng rỗng, một phần tử, giá trị trùng, trường hợp xấu nhất) thường ẩn rất kỹ. Thêm module `#[cfg(test)]` dưới đây vào cuối tệp `main.rs`, rồi chạy `cargo test`. Một mẫu rất mạnh xuất hiện ở đây: **kiểm chứng chéo** — so kết quả thuật toán tự viết với hàm chuẩn của Rust (`quicksort` đối chiếu `slice::sort`, tìm kiếm nhị phân đối chiếu tìm tuyến tính).

```rust
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
```

## Tóm tắt chương & Bài tập rèn luyện (Summary & Exercises)

### 4 Điểm cốt lõi cần ghi nhớ:
1. **Sức mạnh $O(1)$ của Bảng băm**: `HashMap` mang lại khả năng tra cứu khóa-giá trị tức thời nhờ thuật toán băm phân phối vào các xô ô nhớ (buckets).
2. **Kỹ thuật Entry API**: Giúp tra cứu, khởi tạo mặc định và cập nhật giá trị chỉ với một lần tính toán băm duy nhất, tối ưu hóa tối đa chu kỳ CPU.
3. **Đồ thị dùng chỉ số**: Biểu diễn Đồ thị bằng danh sách kề `Vec<Vec<usize>>` là phương pháp chuẩn mực trong Rust để đạt 100% Safe Rust và giải phóng lập trình viên khỏi gánh nặng con trỏ.
4. **BFS tìm đường ngắn nhất**: Thuật toán Tìm kiếm theo chiều rộng (BFS) kết hợp với Hàng đợi FIFO (`VecDeque`) là công cụ hoàn hảo để tìm khoảng cách chặng ngắn nhất trong đồ thị không trọng số.

### Bài tập rèn luyện tự giải:
1. **Bài tập 1 (Tìm kiếm phần tử xuất hiện nhiều nhất)**:  
   Sử dụng `HashMap`, hãy viết một hàm `fn most_common(list: &[i32]) -> Option<i32>` tìm số nguyên có tần suất xuất hiện nhiều nhất trong mảng trong thời gian $O(N)$.
2. **Bài tập 2 (Phát hiện đỉnh cô lập trong đồ thị)**:  
   Viết phương thức `fn isolated_vertices(&self) -> Vec<usize>` cho cấu trúc `Graph` để liệt kê tất cả các đỉnh không có bất kỳ cạnh kết nối nào với các đỉnh khác trong mạng lưới (`adjacency_list[i].is_empty()`).
3. **Bài tập 3 (Thuật toán DFS - Tìm kiếm theo chiều sâu)**:  
   Dựa trên cấu trúc `Graph` đã học, hãy viết một hàm `fn dfs_connected(&self, u: usize, v: usize) -> bool` sử dụng đệ quy để kiểm tra xem có tồn tại bất kỳ con đường nào nối giữa hai đỉnh `u` và `v` hay không (không bắt buộc phải là con đường ngắn nhất).

---

### Gợi ý & Lời giải

<details>
<summary><b>Bài tập 1 — Gợi ý</b></summary>

Một lượt duyệt để đếm tần suất vào `HashMap`, một lượt nữa để tìm khoá có tần suất cao nhất. Hai lượt O(N) vẫn là O(N).
</details>

<details>
<summary><b>Bài tập 1 — Lời giải</b></summary>

```rust
use std::collections::HashMap;

/// Tìm số xuất hiện nhiều nhất trong O(N).
pub fn most_common(list: &[i32]) -> Option<i32> {
    if list.is_empty() { return None; }

    let mut dem: HashMap<i32, usize> = HashMap::new();
    for &x in list {
        *dem.entry(x).or_insert(0) += 1;     // O(1) khấu hao mỗi phần tử
    }
    // max_by_key trả phần tử CUỐI khi hoà; thêm khoá vào tiêu chí
    // so sánh để kết quả TẤT ĐỊNH thay vì phụ thuộc thứ tự duyệt HashMap.
    dem.into_iter().max_by_key(|&(gt, n)| (n, gt)).map(|(gt, _)| gt)
}

#[test]
fn tim_dung_phan_tu_pho_bien() {
    assert_eq!(most_common(&[1, 3, 3, 2, 3, 1]), Some(3));
    assert_eq!(most_common(&[]), None);
    assert_eq!(most_common(&[7]), Some(7));
    // Hoà nhau -> luôn cho cùng kết quả, không phụ thuộc thứ tự HashMap.
    let a = most_common(&[1, 1, 2, 2]);
    let b = most_common(&[2, 2, 1, 1]);
    assert_eq!(a, b);
}
```

Chi tiết dễ bỏ qua: `max_by_key(|&(gt, n)| (n, gt))` chứ không phải `(n)`. Thứ tự duyệt `HashMap` **không xác định**, nên khi hai giá trị hoà tần suất, chỉ so `n` sẽ cho kết quả khác nhau giữa các lần chạy. Thêm khoá vào tiêu chí khiến kết quả tất định — đúng nguyên tắc mà Chương 76 phải trả giá mới học được.
</details>

<details>
<summary><b>Bài tập 2 — Gợi ý</b></summary>

Đỉnh cô lập là đỉnh có danh sách kề rỗng. Chú ý: đồ thị ở đây **vô hướng**, nên nếu `add_edge` thêm cả hai chiều thì một lần kiểm là đủ.
</details>

<details>
<summary><b>Bài tập 2 — Lời giải</b></summary>

```rust
impl Graph {
    /// Đỉnh không nối với ai. Với đồ thị VÔ HƯỚNG, danh sách kề rỗng
    /// là đủ để kết luận — vì mọi cạnh đều được ghi ở cả hai đầu.
    pub fn isolated_vertices(&self) -> Vec<usize> {
        self.adjacency_list.iter().enumerate()
            .filter(|(_, ke)| ke.is_empty())
            .map(|(i, _)| i)
            .collect()
    }
}

#[test]
fn tim_dung_dinh_co_lap() {
    let mut g = Graph::new();
    let a = g.add_peak("A");
    let b = g.add_peak("B");
    let c = g.add_peak("C");     // C không nối với ai
    g.add_edge(a, b);

    assert_eq!(g.isolated_vertices(), vec![c]);

    // Nối C vào -> không còn đỉnh cô lập nào.
    g.add_edge(b, c);
    assert!(g.isolated_vertices().is_empty());
}
```

**Cảnh báo cho đồ thị CÓ HƯỚNG:** danh sách kề rỗng chỉ nghĩa là "không có cạnh **đi ra**". Một đỉnh có thể nhận rất nhiều cạnh đi vào mà vẫn có danh sách kề rỗng — nó không cô lập chút nào. Muốn tìm đỉnh thực sự cô lập trong đồ thị có hướng, phải kiểm cả bậc vào, tức là quét toàn bộ danh sách kề của mọi đỉnh khác.
</details>

<details>
<summary><b>Bài tập 3 — Gợi ý</b></summary>

DFS đệ quy cần một tập `da_tham` để không lặp vô tận khi đồ thị có chu trình. Đây chính là chỗ hầu hết mọi người quên.
</details>

<details>
<summary><b>Bài tập 3 — Lời giải</b></summary>

```rust
use std::collections::HashSet;

impl Graph {
    /// Có đường đi nào giữa u và v không? Không cần ngắn nhất.
    pub fn dfs_connected(&self, u: usize, v: usize) -> bool {
        if u >= self.adjacency_list.len() || v >= self.adjacency_list.len() {
            return false;
        }
        let mut da_tham = HashSet::new();
        self.dfs(u, v, &mut da_tham)
    }

    fn dfs(&self, hien_tai: usize, dich: usize, da_tham: &mut HashSet<usize>) -> bool {
        if hien_tai == dich { return true; }
        // `insert` trả false nếu đã có -> chặn lặp vô tận khi có chu trình.
        if !da_tham.insert(hien_tai) { return false; }
        self.adjacency_list[hien_tai].iter()
            .any(|&ke| self.dfs(ke, dich, da_tham))
    }
}

#[test]
fn dfs_tim_duoc_duong_va_bao_dung_khi_khong_co() {
    let mut g = Graph::new();
    let (a, b, c, d) = (g.add_peak("A"), g.add_peak("B"),
                        g.add_peak("C"), g.add_peak("D"));
    g.add_edge(a, b);
    g.add_edge(b, c);
    // D tách rời

    assert!(g.dfs_connected(a, c), "A-B-C có đường");
    assert!(g.dfs_connected(a, a), "chính nó luôn tới được");
    assert!(!g.dfs_connected(a, d), "D tách rời");
    assert!(!g.dfs_connected(a, 99), "đỉnh không tồn tại");

    // Có CHU TRÌNH -> phải dừng, không lặp vô tận.
    g.add_edge(c, a);
    assert!(g.dfs_connected(a, c));
    assert!(!g.dfs_connected(a, d));
}
```

`da_tham.insert(x)` trả `false` nếu `x` đã có — dùng luôn giá trị trả về làm điều kiện dừng, gọn hơn `if da_tham.contains(&x) { return ... }` rồi mới `insert`.

**DFS khác BFS ở đâu:** DFS trả lời "có đường không?" và tiết kiệm bộ nhớ hơn (chỉ giữ một nhánh trong ngăn xếp), nhưng đường nó tìm ra **không nhất thiết ngắn nhất**. Cần ngắn nhất thì phải dùng BFS, như `bfs_shortest_distance` ở phần trên chương.
</details>
