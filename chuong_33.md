# Chương 33: Chỉ mục hiệu năng cao B-Tree & B+ Tree (High-Performance B-Tree & B+ Tree Indexing)

## Giới thiệu & Mục tiêu học tập

Hãy tưởng tượng một bảng cơ sở dữ liệu lưu trữ thông tin của **50 triệu công dân**. Nếu không có cấu trúc hỗ trợ tìm kiếm, mỗi khi cảnh sát muốn tra cứu thông tin của một người mang số căn cước "079...", hệ thống cơ sở dữ liệu sẽ phải quét lần lượt từ bản ghi số 1 đến bản ghi số 50 triệu (**Quét toàn bộ bảng - Full Table Scan**). Với hàng triệu lần truy cập đĩa cứng SSD/HDD, câu lệnh truy vấn có thể mất hàng chục phút để hoàn thành — một điều không thể chấp nhận được trong thế giới thực!

Để biến thời gian chờ đợi hàng chục phút thành **vài mili-giây (thậm chí micro-giây)**, các nhà khoa học máy tính đã phát minh ra **Cấu trúc Chỉ mục (Database Indexing)**, và "vị vua không ngai" ngự trị trên hầu hết các công cụ cơ sở dữ liệu quan hệ (RDBMS) suốt nửa thế kỷ qua chính là: **Cây B-Tree và Cây B+ Tree**.

Tại sao chúng ta không dùng Cây nhị phân tìm kiếm (BST, Red-Black Tree hay AVL) đã học ở Topic 5 mà lại phải phát minh ra B-Tree và B+ Tree? Câu trả lời nằm ở sự khác biệt giữa **Bộ nhớ RAM** và **Khối trang 4KB trên Đĩa cứng**. Cây nhị phân chỉ có 2 nhánh con khiến cây mọc rất cao (với 50 triệu phần tử, chiều cao lên tới gần 30 tầng, tương đương 30 lần đọc đĩa chậm chạp). Ngược lại, Cây B+ Tree có hàng trăm nhánh con trên mỗi nút, nén chiều cao của cây xuống chỉ còn **3 đến 4 tầng**, khớp hoàn hảo với cấu trúc trang 4KB của bộ nhớ đệm (buffer pool)!

Mục tiêu học tập của chương này:
- Hiểu rõ vì sao Cây nhị phân thất bại trên đĩa cứng và lý do Cây nhiều nhánh (Multi-way Search Tree) như B-Tree và B+ Tree thống trị kiến trúc cơ sở dữ liệu.
- Phân biệt cấu tạo cốt lõi giữa **B-Tree** và **B+ Tree**: Vì sao B+ Tree tách biệt hoàn toàn nút trong (Internal Node) và nút lá (Leaf Node).
- Thấu hiểu sức mạnh của **Danh sách liên kết ngang giữa các nút lá** trong việc xử lý truy vấn quét dải dữ liệu (Range Queries).
- Nắm vững cơ chế tự cân bằng và thuật toán phân tách nút (Node Splitting) khi một trang dữ liệu bị đầy.
- Tự tay hiện thực mô hình cấu trúc nút B+ Tree trong Rust hỗ trợ tìm kiếm nhị phân và quét dải dữ liệu.

---

## Hình tượng hóa đời sống (Intuitive Everyday Analogy)

Hãy cùng quan sát hệ thống biển báo trên mạng lưới đường cao tốc liên tỉnh để hình dung cách B+ Tree chỉ đường cho cơ sở dữ liệu:

```
┌──────────────────────────────────────────────────────────────────────────────────┐
│                   HÌNH TƯỢNG HÓA CHỈ MỤC B+ TREE TRÊN ĐƯỜNG CAO TỐC              │
├──────────────────────────────────────────────────────────────────────────────────┤
│                                                                                  │
│ [NÚT GỐC - TẦNG 1: BIỂN BÁO LỚN TRÊN TRỜI]                                      │
│                ┌──────────────────────────────────────────────┐                  │
│                │ [Khóa rẽ: 100]           [Khóa rẽ: 200]      │                  │
│                └───────┬──────────────────────┬───────────────┘                  │
│     Đi lối < 100       │      Từ 100 đến 200  │        Đi lối > 200              │
│       ┌────────────────┴───────┐              └────────────────┐                 │
│       ▼                                                        ▼                 │
│ [NÚT TRONG - TẦNG 2]                                   [NÚT TRONG - TẦNG 2]      │
│ ┌────────────────────────┐                             ┌───────────────────────┐ │
│ │ [Khóa: 30]  [Khóa: 70] │                             │ [Khóa: 240] [Khóa:280]│ │
│ └─────┬───────────┬──────┘                             └─────┬───────────┬─────┘ │
│       ▼           ▼                                          ▼           ▼       │
│ [NÚT LÁ - TẦNG 3: BÃI ĐỖ XE CHỨA DỮ LIỆU THỰC TẾ]                                │
│ ┌──────────────┐      ┌──────────────┐      ┌──────────────┐      ┌────────────┐ │
│ │ [10] [20]    │ ═══► │ [30] [50]    │ ═══► │ [70] [90]    │ ═══► │ [100] [150]│ │
│ └──────────────┘      └──────────────┘      └──────────────┘      └────────────┘ │
│  ◄──────────────── ĐƯỜNG HẦM LIÊN KẾT XÍCH QUÉT DẢI (RANGE SCAN) ──────────────►  │
└──────────────────────────────────────────────────────────────────────────────────┘
```

### 1. Hệ thống biển báo nhiều tầng trên đường cao tốc
- Giả sử bạn lái xe tìm địa chỉ số **85 trên đại lộ**:
  - **Tầng 1 (Nút gốc)**: Biển báo chỉ dẫn ghi hai cột mốc lớn: `[100]` và `[200]`. Vì $85 < 100$, bạn rẽ ngay vào lối đi bên trái. Bạn không cần bận tâm đến hàng triệu ngôi nhà ở lối đi giữa và lối đi bên phải!
  - **Tầng 2 (Nút trong)**: Biển chỉ đường ghi: `[30]` và `[70]`. Vì $85 > 70$, bạn rẽ tiếp vào ngã ba bên phải.
  - **Tầng 3 (Nút lá)**: Bạn đỗ xe vào đúng bãi đỗ xe chứa các số từ 70 đến 99. Tại đây, bạn nhìn thấy ngay ngôi nhà số 85!
- Chỉ qua đúng **3 lần nhìn biển báo**, bạn đã tìm thấy mục tiêu giữa hàng chục triệu số nhà. Mỗi lần nhìn biển báo tương đương đúng 1 lần đọc trang 4KB từ đĩa vào RAM!

### 2. Sự khác biệt tinh tế giữa B-Tree và B+ Tree
- **Trong Cây B-Tree truyền thống**: Mỗi biển báo trên cao (nút trong) lại cõng theo một thùng hàng nặng trịch (dữ liệu bản ghi thực tế). Điều này khiến tấm biển báo trở nên cồng kềnh, một trang 4KB chỉ chứa được vài ba biển báo, làm cây mọc cao lên.
- **Trong Cây B+ Tree hiện đại**: 
  - Các nút trên cao chỉ chứa duy nhất các con số chỉ hướng (Khóa - Key) và địa chỉ trang con (`page_id`), cực kỳ thanh thoát và nhẹ nhàng. Một trang 4KB có thể nhồi nhét tới hàng trăm khóa chỉ hướng!
  - Toàn bộ dữ liệu thực tế (Payload/Value) đều được đưa hết xuống các **Nút lá (Leaf Nodes)** ở tầng trệt.
  - Đặc biệt nhất: Các nút lá này được nối xích với nhau bằng một **Đường hầm liên kết (Linked List)**. Khi bạn muốn tìm tất cả những người từ 20 đến 80 tuổi, bạn chỉ cần dùng cây tìm ra người 20 tuổi ở nút lá đầu tiên, sau đó cứ thế đi bộ men theo đường hầm từ lá này sang lá khác để lấy hết kết quả mà không cần phải leo ngược lên các tầng trên!

---

## Khái niệm & Cơ chế kỹ thuật chuyên sâu (In-Depth Technical Mechanics)

### 1. Hệ số phân nhánh (Branching Factor) và Chiều cao của cây

Giả sử một trang dữ liệu có kích thước chuẩn $4096 \text{ bytes}$:
- Mỗi khóa tìm kiếm (ví dụ ID `u64`) chiếm $8 \text{ bytes}$.
- Mỗi con trỏ trang con (`page_id`) chiếm $4 \text{ bytes}$.
- Một cặp `(Key, Pointer)` chiếm khoảng $12 \text{ bytes}$.
- Một nút trong của B+ Tree có thể chứa tới: $\frac{4096}{12} \approx 340 \text{ nhánh con}$!

Với hệ số phân nhánh (Fan-out) là $M = 300$:
- **Tầng 1 (Gốc)**: 1 nút -> Quản lý 300 nút con.
- **Tầng 2**: 300 nút -> Quản lý $300 \times 300 = 90.000$ nút con.
- **Tầng 3**: $90.000$ nút -> Quản lý $90.000 \times 300 = 27.000.000$ (27 triệu bản ghi)!
- **Tầng 4**: Quản lý tới **8,1 tỷ bản ghi** (vượt quá dân số toàn cầu)!

> **Kết luận sống còn**: Với 50 triệu bản ghi, cây B+ Tree chỉ có chiều cao đúng **3 hoặc 4 tầng**. Nút gốc luôn luôn được ghim chặt trong bộ nhớ đệm (buffer pool) trên RAM. Do đó, để tìm kiếm bất kỳ bản ghi nào trong 50 triệu dòng, hệ thống chỉ tốn tối đa **2 đến 3 lần đọc đĩa SSD**!

### 2. Phép phân tách nút (Node Splitting) khi cây đầy

Cây B+ Tree là một cây tự cân bằng hoàn hảo từ dưới lên (Bottom-up growth):
1. Khi chèn một bản ghi mới vào nút lá, nếu nút lá chưa đầy sức chứa tối đa, ta chèn khóa vào đúng vị trí theo thứ tự tăng dần ($O(M)$).
2. Khi nút lá bị tràn (ví dụ sức chứa tối đa là 4 phần tử nhưng phần tử thứ 5 được thêm vào):
   - Nút lá bị chẻ đôi làm hai nửa: 2 phần tử ở lại nút cũ, 2 phần tử sang nút mới.
   - Phần tử ở giữa được đẩy lên (Promote) làm khóa dẫn đường cho nút cha ở tầng trên.
   - Nối con trỏ `next_leaf` từ nút cũ sang nút mới để duy trì chuỗi quét dải liên tục.
3. Nếu nút cha cũng bị tràn, quá trình phân tách tiếp tục lan truyền ngược lên trên. Nếu nút gốc bị phân tách, một nút gốc mới được sinh ra và chiều cao của toàn bộ cây tăng lên 1 tầng.

---

## Mã nguồn minh họa thực chiến (Idiomatic Runnable Rust Blueprint)

Dưới đây là một bản thiết kế mã nguồn Rust hoàn chỉnh, độc lập và mang tính thực chiến cao. Chương trình cài đặt cấu trúc Cây B+ Tree đơn giản hóa (In-Memory B+ Tree Node Model), hỗ trợ thao tác tìm kiếm điểm (Point Query) và quét dải dữ liệu (Range Scan):

```rust
/// Dung lượng tối đa của một nút trước khi bị phân tách (đơn giản hóa để minh họa)
pub const SUC_CHUA_NUT: usize = 3;

/// Cấu tạo của một Nút trong cây B+ Tree
#[derive(Debug, Clone)]
pub enum BPlusNode<K: Ord + Copy, V: Clone> {
    /// NÚT TRONG (Internal Node): Chỉ chứa Khóa chỉ dẫn và con trỏ tới các nút con
    Internal {
        keys: Vec<K>,
        children: Vec<Box<BPlusNode<K, V>>>,
    },
    /// NÚT LÁ (Leaf Node): Chứa Khóa và Dữ liệu thực tế
    Leaf {
        keys: Vec<K>,
        values: Vec<V>,
    },
}

impl<K: Ord + Copy, V: Clone> BPlusNode<K, V> {
    /// Tạo một nút lá mới tinh
    pub fn new_leaf() -> Self {
        BPlusNode::Leaf {
            keys: Vec::new(),
            values: Vec::new(),
        }
    }

    /// Tìm kiếm giá trị theo khóa trong cây con bắt đầu từ nút này
    pub fn search(&self, key: &K) -> Option<&V> {
        match self {
            BPlusNode::Leaf { keys, values } => {
                // Tại nút lá: Dùng tìm kiếm nhị phân trên mảng khóa đã sắp xếp
                match keys.binary_search(key) {
                    Ok(idx) => Some(&values[idx]),
                    Err(_) => None,
                }
            }
            BPlusNode::Internal { keys, children } => {
                // Tại nút trong: Tìm nhánh con thích hợp để đi xuống
                // Nhánh con thứ i quản lý các khóa nhỏ hơn keys[i]
                let mut idx = 0;
                while idx < keys.len() && *key >= keys[idx] {
                    idx += 1;
                }
                children[idx].search(key)
            }
        }
    }

    /// Quét dải dữ liệu: Thu thập tất cả các giá trị có khóa trong khoảng [min_key, max_key]
    pub fn range_scan(&self, min_key: &K, max_key: &K, ket_qua: &mut Vec<(K, V)>) {
        match self {
            BPlusNode::Leaf { keys, values } => {
                for (i, &k) in keys.iter().enumerate() {
                    if k >= *min_key && k <= *max_key {
                        ket_qua.push((k, values[i].clone()));
                    }
                }
            }
            BPlusNode::Internal { keys, children } => {
                for (i, child) in children.iter().enumerate() {
                    // Tối ưu hóa: Chỉ đi xuống nhánh con nếu khoảng khóa có deliver thoa
                    let lower_bound_ok = if i == 0 { true } else { keys[i - 1] <= *max_key };
                    let upper_bound_ok = if i == keys.len() { true } else { keys[i] >= *min_key };
                    if lower_bound_ok && upper_bound_ok {
                        child.range_scan(min_key, max_key, ket_qua);
                    }
                }
            }
        }
    }

    /// Thêm một cặp (key, value) vào nút lá đơn giản hóa
    pub fn insert_non_full_leaf(&mut self, key: K, value: V) -> bool {
        match self {
            BPlusNode::Leaf { keys, values } => {
                match keys.binary_search(&key) {
                    Ok(idx) => {
                        // Khóa đã tồn tại -> Cập nhật đè giá trị mới
                        values[idx] = value;
                        false
                    }
                    Err(idx) => {
                        // Chèn vào đúng vị trí để duy trì thứ tự sắp xếp
                        keys.insert(idx, key);
                        values.insert(idx, value);
                        true
                    }
                }
            }
            _ => panic!("Chỉ được gọi trên nút lá"),
        }
    }
}

/// Cấu trúc cây B+ Tree hoàn chỉnh
pub struct BPlusTree<K: Ord + Copy, V: Clone> {
    pub root: Box<BPlusNode<K, V>>,
    pub total_records: usize,
}

impl<K: Ord + Copy, V: Clone> BPlusTree<K, V> {
    pub fn new() -> Self {
        Self {
            root: Box::new(BPlusNode::new_leaf()),
            total_records: 0,
        }
    }

    /// Tìm kiếm một khóa bất kỳ
    pub fn get(&self, key: &K) -> Option<&V> {
        self.root.search(key)
    }

    /// Quét các bản ghi trong khoảng [min_key, max_key]
    pub fn get_range(&self, min_key: K, max_key: K) -> Vec<(K, V)> {
        let mut ket_qua = Vec::new();
        self.root.range_scan(&min_key, &max_key, &mut ket_qua);
        ket_qua
    }
}

impl<K: Ord + Copy, V: Clone> Default for BPlusTree<K, V> {
    fn default() -> Self {
        Self::new()
    }
}

fn main() {
    println!("============================================================");
    println!("     MÔ HÌNH CHỈ MỤC HIỆU NĂNG CAO B-TREE & B+ TREE         ");
    println!("============================================================");

    // Xây dựng một mô hình B+ Tree thủ công với 1 nút gốc (Internal) và 2 nút lá (Leaf)
    // Cấu trúc:
    //                    [Gốc: Khóa rẽ = 50]
    //                   /                   \
    //   [Lá 1: (10, "A"), (30, "B")]     [Lá 2: (50, "C"), (70, "D"), (90, "E")]
    let mut is_left = BPlusNode::new_leaf();
    is_left.insert_non_full_leaf(10, "Alice (Hà Nội)");
    is_left.insert_non_full_leaf(30, "Bình (Đà Nẵng)");

    let mut is_must = BPlusNode::new_leaf();
    is_must.insert_non_full_leaf(50, "Cường (TP.HCM)");
    is_must.insert_non_full_leaf(70, "Dũng (Cần Thơ)");
    is_must.insert_non_full_leaf(90, "Emmy (Hải Phòng)");

    let root_node = BPlusNode::Internal {
        keys: vec![50],
        children: vec![Box::new(is_left), Box::new(is_must)],
    };

    let b_tree = BPlusTree {
        root: Box::new(root_node),
        total_records: 5,
    };

    println!("[1] Kiểm tra tính năng tìm kiếm điểm (Point Search):");
    let ket_qua_30 = b_tree.get(&30);
    println!("    - Tra cứu khóa 30: {:?}", ket_qua_30);
    assert_eq!(ket_qua_30, Some(&"Bình (Đà Nẵng)"));

    let ket_qua_70 = b_tree.get(&70);
    println!("    - Tra cứu khóa 70: {:?}", ket_qua_70);
    assert_eq!(ket_qua_70, Some(&"Dũng (Cần Thơ)"));

    let ket_qua_99 = b_tree.get(&99);
    println!("    - Tra cứu khóa 99 (không tồn tại): {:?}", ket_qua_99);
    assert_eq!(ket_qua_99, None);

    println!("\n[2] Kiểm tra tính năng quét dải dữ liệu (Range Scan):");
    println!("    - Tìm kiếm các bản ghi có khóa từ 25 đến 75:");
    let list_long = b_tree.get_range(25, 75);
    for (k, v) in &list_long {
        println!("      -> Khóa {}: {}", k, v);
    }

    // Kết quả kỳ vọng: Khóa 30, 50, 70
    assert_eq!(list_long.len(), 3);
    assert_eq!(list_long[0].0, 30);
    assert_eq!(list_long[1].0, 50);
    assert_eq!(list_long[2].0, 70);
    println!("    => Quét dải dữ liệu hoàn tất thành công vượt trội!");

    println!("============================================================");
    println!("               HOÀN TẤT THỰC NGHIỆM CHƯƠNG 29               ");
    println!("============================================================");
}
```

---

## Bảng tra cứu lỗi biên dịch & Cách khắc phục (Compiler Error Guide)

Dưới đây là các lỗi biên dịch điển hình khi xây dựng cấu trúc cây B-Tree và B+ Tree trong Rust:

| Mã lỗi | Thông báo mẫu từ trình biên dịch | Nguyên nhân cốt lõi | Cách khắc phục nhanh |
|---|---|---|---|
| **E0507** | `cannot move out of a shared reference` | Bạn cố lấy một nút con ra khỏi `children[idx]` bằng cú pháp gán trực tiếp trong khi chỉ có tham chiếu mượn `&self`. | Dùng tham chiếu mượn `&children[idx]` khi duyệt, hoặc dùng `.remove()` nếu hàm có quyền sở hữu (ownership) khả biến `&mut self`. |
| **E0277** | `the trait bound 'K: Ord' is not satisfied` | B+ Tree bắt buộc các khóa phải có thứ tự sắp xếp tuyệt đối để chia nhánh nhị phân. Nếu kiểu khóa `K` chưa cài trait `Ord`, trình biên dịch sẽ chặn lại. | Bổ sung ràng buộc trait: `impl<K: Ord, V> ...`. |
| **E0596** | `cannot borrow '...' as mutable, as it is not declared as mutable` | Bạn cố chèn thêm khóa vào nút lá trong khi biến cây hoặc nút được khai báo bằng `let` bất biến. | Khai báo với từ khóa `let mut`. |
| **E0004** | `non-exhaustive patterns: 'Internal { .. }' not covered` | Bạn dùng khối lệnh `match` trên enum `BPlusNode` nhưng chỉ xử lý trường hợp `Leaf` mà bỏ sót trường hợp `Internal`. | Bổ sung đầy đủ các nhánh `match` cho cả hai biến thể của enum. |

### Ví dụ phân tích lỗi `E0507` khi truy xuất nút con trong B+ Tree:

```rust
enum NutDemo {
    Leaf(Vec<i32>),
    Internal(Vec<Box<NutDemo>>),
}

// Đoạn mã lỗi minh họa E0507: Cố đoạt quyền sở hữu con trỏ Box từ tham chiếu mượn
fn child_broken(nut: &NutDemo) {
    match nut {
        NutDemo::Internal(children) => {
            // let first_child = children[0]; // LỖI E0507: cannot move out of indexed content!
        }
        _ => {}
    }
}

// Cách sửa chữa đúng chuẩn: Mượn tham chiếu &Box hoặc mượn trực tiếp &NutDemo
fn lay_con_dung(nut: &NutDemo) {
    match nut {
        NutDemo::Internal(children) => {
            let first_child: &NutDemo = &children[0]; // Chỉ mượn, không di chuyển quyền sở hữu!
            println!("Đã mượn nút con thành công.");
        }
        _ => {}
    }
}
```

---

## Tóm tắt chương & Bài tập rèn luyện (Summary & Exercises)

### 4 Điểm cốt lõi cần ghi nhớ:
1. **Khắc phục điểm yếu đĩa cứng**: B-Tree và B+ Tree có hệ số phân nhánh lớn (hàng trăm nhánh), nén chiều cao của cây xuống chỉ còn 3-4 tầng, giảm số lần truy xuất đĩa xuống mức tối thiểu.
2. **B-Tree vs B+ Tree**: B+ Tree chỉ lưu khóa ở các nút trong và dồn toàn bộ dữ liệu xuống nút lá, giúp các nút trong chứa được nhiều khóa hơn và các nút lá có thể nối xích với nhau.
3. **Thần tốc quét dải (Range Scan)**: Nhờ danh sách liên kết ngang giữa các nút lá, câu lệnh `BETWEEN A AND B` diễn ra cực nhanh bằng cách duyệt tuần tự trên các lá mà không cần quay lại nút gốc.
4. **Phân tách nút tự cân bằng**: Cây B+ Tree luôn phát triển từ dưới lên thông qua cơ chế chẻ đôi nút khi đầy, đảm bảo mọi nút lá luôn nằm trên cùng một độ sâu (cân bằng 100%).

### Bài tập rèn luyện tự giải:
1. **Bài tập 1 (Tính số lần I/O đĩa)**:  
   Một bảng cơ sở dữ liệu có 100.000.000 (100 triệu) bản ghi. Sử dụng chỉ mục B+ Tree với hệ số phân nhánh trung bình $M = 200$. Hãy tính xem cây B+ Tree này có chiều cao bao nhiêu tầng? Nếu nút gốc đã được nạp sẵn vào RAM, ta cần đọc đĩa tối đa bao nhiêu lần để tìm thấy một bản ghi?
2. **Bài tập 2 (Xác thực tính chất B+ Tree)**:  
   Viết một hàm kiểm thử kiểm tra xem toàn bộ các khóa trong mảng `keys` của một nút lá có luôn luôn được sắp xếp theo thứ tự tăng dần hay không (`keys.windows(2).all(|w| w[0] < w[1])`).
3. **Bài tập 3 (Tư duy thiết kế)**:  
   Tại sao các cơ sở dữ liệu lại khuyên người dùng nên chọn Khóa chính (Primary Key) là số nguyên tự tăng (`AUTOINCREMENT` / `SERIAL` / `UUID v7`) thay vì một chuỗi ngẫu nhiên (`UUID v4`) khi sử dụng chỉ mục B+ Tree? Hiện tượng gì sẽ xảy ra với các nút lá nếu ta chèn các khóa ngẫu nhiên liên tục?

---

### Gợi ý & Lời giải

<details>
<summary><b>Bài tập 1 — Gợi ý</b></summary>

Mỗi tầng nhân số nút lên `M` lần. Hỏi: cần bao nhiêu tầng để `200^h ≥ 100.000.000`?
</details>

<details>
<summary><b>Bài tập 1 — Lời giải</b></summary>

**Chiều cao cây:**

```
M = 200 (hệ số phân nhánh), N = 100.000.000 bản ghi

tầng 1 (gốc)     : 200      khoá
tầng 2           : 200²     = 40.000
tầng 3           : 200³     = 8.000.000        < 100 triệu, chưa đủ
tầng 4           : 200⁴     = 1.600.000.000    ≥ 100 triệu  ✓

chiều cao = 4 tầng
```

Kiểm bằng công thức: `h = ⌈log₂₀₀(10⁸)⌉ = ⌈8 / log₁₀(200)⌉ = ⌈8 / 2,301⌉ = ⌈3,48⌉ = **4**`.

**Số lần đọc đĩa:** gốc đã nằm sẵn trong RAM, nên chỉ phải đọc 3 tầng còn lại → **tối đa 3 lần đọc đĩa** để tìm một bản ghi trong 100 triệu.

Con số này là toàn bộ lý do B+ Tree tồn tại. So sánh:

| Cấu trúc | Số lần đọc đĩa | Thời gian (SSD ~0,1 ms/lần) |
|---|---|---|
| B+ Tree, M=200 | 3 | ~0,3 ms |
| Cây nhị phân cân bằng | ⌈log₂ 10⁸⌉ = 27 | ~2,7 ms |
| Quét tuần tự | ~10⁸ / (số bản ghi mỗi trang) | hàng phút |

Cây nhị phân *cũng* là O(log N) — nhưng cơ số 2 thay vì 200. Cùng độ phức tạp tiệm cận, chênh nhau **chín lần** số lần chạm đĩa. Đó là lời nhắc rằng big-O giấu mất hằng số, và với I/O thì hằng số mới là thứ trả tiền.
</details>

<details>
<summary><b>Bài tập 2 — Gợi ý</b></summary>

Bất biến của nút lá: khoá luôn tăng dần. Có sẵn `windows(2)` để kiểm hai phần tử liền kề.
</details>

<details>
<summary><b>Bài tập 2 — Lời giải</b></summary>

```rust
/// Bất biến sống còn của B+ Tree: khoá trong MỌI nút lá phải tăng dần.
/// Nếu sai thì `search` bằng tìm kiếm nhị phân sẽ trả kết quả sai — âm thầm.
fn kiem_khoa_tang_dan<K: Ord + Copy + std::fmt::Debug, V: Clone>(
    nut: &BPlusNode<K, V>,
) -> bool {
    match nut {
        BPlusNode::Leaf { keys, values } => {
            assert_eq!(keys.len(), values.len(), "mỗi khoá phải có đúng một giá trị");
            keys.windows(2).all(|w| w[0] < w[1])
        }
        BPlusNode::Internal { keys, children } => {
            assert_eq!(children.len(), keys.len() + 1,
                       "nút trong luôn có nhiều hơn khoá đúng một con");
            keys.windows(2).all(|w| w[0] < w[1])
                && children.iter().all(|c| kiem_khoa_tang_dan(c))
        }
    }
}

#[test]
fn moi_nut_la_deu_giu_thu_tu_tang_dan() {
    let mut cay: BPlusTree<i32, String> = BPlusTree::new();
    // Nạp theo thứ tự LỘN XỘN — bất biến vẫn phải đúng.
    for k in [50, 10, 90, 30, 70, 20, 80, 40, 60] {
        cay.insert(k, format!("v{k}"));
    }
    assert!(kiem_khoa_tang_dan(&cay.root));

    // Cây rỗng cũng phải thoả (không có cặp nào để so).
    let rong: BPlusTree<i32, String> = BPlusTree::new();
    assert!(kiem_khoa_tang_dan(&rong.root));
}
```

`windows(2).all(...)` đúng ngay cả với 0 hoặc 1 phần tử — `windows` trả về iterator rỗng, và `all` trên iterator rỗng là `true`. Đó là hành vi đúng: một nút có một khoá thì hiển nhiên đã sắp xếp.

Hai `assert_eq!` bên trong bắt những bất biến *khác* mà đề bài không hỏi nhưng cũng quan trọng: số giá trị khớp số khoá ở lá, và nút trong luôn có `n+1` con cho `n` khoá. Bất biến vỡ ở đây thường là dấu hiệu tách nút sai.
</details>

<details>
<summary><b>Bài tập 3 — Gợi ý</b></summary>

Khoá tăng dần thì mọi lần chèn đều rơi vào **cùng một nút lá bên phải**. Khoá ngẫu nhiên thì rơi rải rác khắp cây. Nghĩ xem điều đó làm gì với bộ đệm và với việc tách nút.
</details>

<details>
<summary><b>Bài tập 3 — Lời giải</b></summary>

**Khoá tăng dần (`SERIAL`, `UUID v7`):**

```
Mọi lần chèn đều vào nút lá TẬN CÙNG BÊN PHẢI.
  → chỉ 1 nút lá "nóng", luôn nằm trong buffer pool
  → tách nút chỉ xảy ra ở rìa phải
  → nút cũ giữ nguyên, đầy khoảng 100%
  → ghi ra đĩa gần như TUẦN TỰ
```

**Khoá ngẫu nhiên (`UUID v4`):**

```
Mỗi lần chèn rơi vào một nút lá NGẪU NHIÊN trong hàng nghìn nút.
  → nút đích thường KHÔNG có trong buffer pool  → phải đọc đĩa trước khi ghi
  → tách nút xảy ra khắp nơi, mỗi lần tách để lại HAI nút đầy ~50%
  → ghi ra đĩa NGẪU NHIÊN
```

Ba hậu quả đo được:

| | Khoá tăng dần | Khoá ngẫu nhiên |
|---|---|---|
| Độ đầy nút lá | ~100% | **~50%** — cây to gấp đôi |
| Trang phải đọc trước khi ghi | gần như không | gần như mọi lần |
| Kiểu ghi đĩa | tuần tự | ngẫu nhiên |

**Hiện tượng "nút đầy 50%"** đáng nói riêng: khi một nút đầy bị tách, nó chia đôi thành hai nút mỗi nút đầy một nửa. Với khoá tăng dần, nút trái sau đó không bao giờ nhận thêm gì nữa nhưng nút phải tiếp tục được lấp đầy — nên trung bình vẫn cao. Với khoá ngẫu nhiên, mọi nút đều bị tách rồi bỏ đó ở mức nửa vời. Cây phình gấp đôi, chiều cao tăng thêm một tầng, và mỗi truy vấn tốn thêm một lần đọc đĩa.

**Vì sao UUID v7 giải quyết được:** nó đặt dấu thời gian mili giây vào 48 bit đầu, nên các UUID sinh gần nhau về thời gian thì cũng gần nhau về giá trị. Vẫn ngẫu nhiên đủ để không đoán được, nhưng **sắp xếp gần như tăng dần** — lấy được cả tính cục bộ của khoá tự tăng lẫn tính phân tán của UUID.
</details>
