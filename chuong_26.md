# Chương 26: Bảng băm, Đồ thị & Các thuật toán tìm kiếm, sắp xếp cốt lõi (Hash Tables, Graphs & Core Search/Sort Algorithms)

## Giới thiệu & Mục tiêu học tập

Chào mừng bạn đến với chương kết thúc của **Chủ đề 5: Cấu trúc dữ liệu & Giải thuật trong Rust**! Đến thời điểm này, bạn đã nắm vững từ các cấu trúc tuyến tính (Mảng, Vector, Danh sách liên kết, Ngăn xếp, Hàng đợi) đến các cấu trúc phân cấp cây nhị phân. Trong chương này, chúng ta sẽ làm chủ hai cấu trúc dữ liệu và giải thuật tối thượng của ngành khoa học máy tính: **Bảng băm (Hash Table)** và **Đồ thị (Graph)**, cùng hai thuật toán kinh điển đi kèm là **Tìm kiếm theo chiều rộng (BFS)** và **Sắp xếp nhanh (Quicksort)**.

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
   let mut dem_tu = std::collections::HashMap::new();
   let tu = "rust";
   // Đếm số lần xuất hiện của từ chỉ với 1 lần tính băm duy nhất!
   *dem_tu.entry(tu).or_insert(0) += 1;
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

### 3. Thuật toán Sắp xếp nhanh (Quicksort - $O(N \log N)$)

Quicksort là một trong những thuật toán sắp xếp thực chiến hiệu quả nhất lịch sử:
1. **Chọn phần tử chốt (Pivot)**: Chọn một phần tử bất kỳ (ví dụ phần tử cuối cùng của mảng).
2. **Phân vùng (Partitioning)**: Duyệt qua mảng và dồn tất cả các phần tử nhỏ hơn chốt về bên trái, các phần tử lớn hơn chốt về bên phải. Đặt phần tử chốt vào đúng vị trí ranh giới chính giữa.
3. **Đệ quy**: Lặp lại quy trình trên cho hai nửa mảng bên trái và bên phải cho đến khi toàn bộ mảng được sắp xếp hoàn tất.

---

## Mã nguồn minh họa thực chiến (Idiomatic Runnable Rust Blueprint)

Dưới đây là một chương trình Rust hoàn chỉnh và độc lập, minh họa trọn vẹn ba nội dung cốt lõi:
1. Ứng dụng Bảng băm (`HashMap`) thống kê tần suất từ vựng với Entry API.
2. Cài đặt Đồ thị bằng danh sách kề chỉ số và chạy thuật toán BFS tìm khoảng cách ngắn nhất.
3. Cài đặt thuật toán Sắp xếp nhanh (Quicksort) in-place trên lát cắt mượn `&mut [T]`:

```rust
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
```

---

## Bảng tra cứu lỗi biên dịch & Cách khắc phục (Compiler Error Guide)

Dưới đây là các lỗi biên dịch điển hình nhất khi lập trình Bảng băm và Đồ thị trong Rust:

| Mã lỗi | Thông báo mẫu từ trình biên dịch | Nguyên nhân cốt lõi | Cách khắc phục nhanh |
|---|---|---|---|
| **E0277** | `the trait bound 'K: Hash' is not satisfied` | Bạn sử dụng một kiểu dữ liệu tự định nghĩa làm Khóa (Key) cho `HashMap` nhưng kiểu đó chưa cài đặt trait `Hash` và `Eq`. | Thêm chỉ dẫn derive tự động: `#[derive(Hash, PartialEq, Eq)]` phía trên khai báo struct. |
| **E0502** | `cannot borrow '...' as mutable because it is also borrowed as immutable` | Bạn đang lặp qua danh sách láng giềng mượn bất biến `&graph.danh_sach_ke[u]` nhưng bên trong thân vòng lặp lại gọi `graph.them_canh()` làm thay đổi đồ thị. | Thu thập các chỉ số cần biến đổi vào một vector tạm trước khi thực hiện ghi đè. |
| **E0382** | `use of moved value: 'tu'` | Bạn gọi `bang_dem.insert(tu, 1)` khiến chuỗi `tu` bị di chuyển quyền sở hữu (ownership), sau đó lại dùng lại `tu` ở dòng lệnh tiếp theo. | Dùng phương thức `.clone()` tạo bản sao độc lập, hoặc lưu tham chiếu mượn chuỗi `&str` nếu chuỗi có thời gian sống (lifetime) dài hơn bảng băm. |
| **E0308** | `mismatched types: expected '&str', found 'String'` | Bạn truyền một giá trị sở hữu `String` vào phương thức tra cứu `.get()` của HashMap vốn chỉ đòi hỏi một lát cắt tham chiếu `&str`. | Thêm dấu `&` phía trước biến chuỗi: `bang_dem.get(&tu)`. |

### Ví dụ phân tích lỗi `E0277` khi dùng struct làm khóa cho `HashMap`:

```rust
// Struct chưa thỏa mãn trait Hash và Eq
struct NguoiDungLoi {
    id: u32,
}

fn thu_nghiem_loi_hash() {
    let mut bang_hash = std::collections::HashMap::new();
    // bang_hash.insert(NguoiDungLoi { id: 1 }, "Admin"); // LỖI E0277!
}

// Cách sửa chữa đúng chuẩn: Derive đầy đủ PartialEq, Eq, Hash
#[derive(Hash, PartialEq, Eq, Debug)]
struct NguoiDungChuan {
    id: u32,
}

fn thu_nghiem_dung_hash() {
    let mut bang_hash = std::collections::HashMap::new();
    bang_hash.insert(NguoiDungChuan { id: 1 }, "Admin");
    println!("Tra cứu khóa người dùng thành công: {:?}", bang_hash.get(&NguoiDungChuan { id: 1 }));
}
```

---

## Tóm tắt chương & Bài tập rèn luyện (Summary & Exercises)

### 4 Điểm cốt lõi cần ghi nhớ:
1. **Sức mạnh $O(1)$ của Bảng băm**: `HashMap` mang lại khả năng tra cứu khóa-giá trị tức thời nhờ thuật toán băm phân phối vào các xô ô nhớ (buckets).
2. **Kỹ thuật Entry API**: Giúp tra cứu, khởi tạo mặc định và cập nhật giá trị chỉ với một lần tính toán băm duy nhất, tối ưu hóa tối đa chu kỳ CPU.
3. **Đồ thị dùng chỉ số**: Biểu diễn Đồ thị bằng danh sách kề `Vec<Vec<usize>>` là phương pháp chuẩn mực trong Rust để đạt 100% Safe Rust và giải phóng lập trình viên khỏi gánh nặng con trỏ.
4. **BFS tìm đường ngắn nhất**: Thuật toán Tìm kiếm theo chiều rộng (BFS) kết hợp với Hàng đợi FIFO (`VecDeque`) là công cụ hoàn hảo để tìm khoảng cách chặng ngắn nhất trong đồ thị không trọng số.

### Bài tập rèn luyện tự giải:
1. **Bài tập 1 (Tìm kiếm phần tử xuất hiện nhiều nhất)**:  
   Sử dụng `HashMap`, hãy viết một hàm `fn tim_phan_tu_pho_bien_nhat(ds: &[i32]) -> Option<i32>` tìm số nguyên có tần suất xuất hiện nhiều nhất trong mảng trong thời gian $O(N)$.
2. **Bài tập 2 (Phát hiện đỉnh cô lập trong đồ thị)**:  
   Viết phương thức `fn tim_dinh_co_lap(&self) -> Vec<usize>` cho cấu trúc `DoThi` để liệt kê tất cả các đỉnh không có bất kỳ cạnh kết nối nào với các đỉnh khác trong mạng lưới (`danh_sach_ke[i].is_empty()`).
3. **Bài tập 3 (Thuật toán DFS - Tìm kiếm theo chiều sâu)**:  
   Dựa trên cấu trúc `DoThi` đã học, hãy viết một hàm `fn dfs_kiem_tra_ket_noi(&self, u: usize, v: usize) -> bool` sử dụng đệ quy để kiểm tra xem có tồn tại bất kỳ con đường nào nối giữa hai đỉnh `u` và `v` hay không (không bắt buộc phải là con đường ngắn nhất).
