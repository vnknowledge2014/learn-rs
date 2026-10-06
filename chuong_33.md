# Chương 33: Chỉ mục hiệu năng cao B-Tree & B+ Tree (High-Performance B-Tree & B+ Tree Indexing)

## Giới thiệu & Mục tiêu học tập

Hãy tưởng tượng một bảng cơ sở dữ liệu lưu trữ thông tin của **50 triệu công dân**. Nếu không có cấu trúc hỗ trợ tìm kiếm, mỗi khi cảnh sát muốn tra cứu thông tin của một người mang số căn cước "079...", hệ thống cơ sở dữ liệu sẽ phải quét lần lượt từ bản ghi số 1 đến bản ghi số 50 triệu (**Quét toàn bộ bảng - Full Table Scan**). Với hàng triệu lần truy cập đĩa cứng SSD/HDD, câu lệnh truy vấn có thể mất hàng chục phút để hoàn thành — một điều không thể chấp nhận được trong thế giới thực!

Để biến thời gian chờ đợi hàng chục phút thành **vài mili-giây (thậm chí micro-giây)**, các nhà khoa học máy tính đã phát minh ra **Cấu trúc Chỉ mục (Database Indexing)**, và "vị vua không ngai" ngự trị trên hầu hết các công cụ cơ sở dữ liệu quan hệ (RDBMS) suốt nửa thế kỷ qua chính là: **Cây B-Tree và Cây B+ Tree**.

Tại sao chúng ta không dùng Cây nhị phân tìm kiếm (BST, Red-Black Tree hay AVL) đã học ở Chủ đề 5 mà lại phải phát minh ra B-Tree và B+ Tree? Câu trả lời nằm ở sự khác biệt giữa **Bộ nhớ RAM** và **Khối trang 4KB trên Đĩa cứng**. Cây nhị phân chỉ có 2 nhánh con khiến cây mọc rất cao (với 50 triệu phần tử, chiều cao lên tới gần 30 tầng, tương đương 30 lần đọc đĩa chậm chạp). Ngược lại, Cây B+ Tree có hàng trăm nhánh con trên mỗi nút, nén chiều cao của cây xuống chỉ còn **3 đến 4 tầng**, khớp hoàn hảo với cấu trúc trang 4KB của bộ nhớ đệm (buffer pool)!

Mục tiêu học tập của chương này:
- Hiểu rõ vì sao Cây nhị phân thất bại trên đĩa cứng và lý do Cây nhiều nhánh (Multi-way Search Tree) như B-Tree và B+ Tree thống trị kiến trúc cơ sở dữ liệu.
- Phân biệt cấu tạo cốt lõi giữa **B-Tree** và **B+ Tree**: Vì sao B+ Tree tách biệt hoàn toàn nút trong (Internal Node) và nút lá (Leaf Node).
- Thấu hiểu sức mạnh của **Danh sách liên kết ngang giữa các nút lá** trong việc xử lý truy vấn quét dải dữ liệu (Range Queries).
- Nắm vững cơ chế tự cân bằng và thuật toán phân tách nút (Node Splitting) khi một trang dữ liệu bị đầy.
- Tự tay hiện thực một B+ Tree trong Rust: chèn có tách nút, tìm kiếm nhị phân trong nút, và quét dải dữ liệu theo chuỗi lá `next_leaf`.

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
2. Khi nút lá bị tràn (ví dụ sức chứa tối đa là 4 khoá nhưng khoá thứ 5 được thêm vào):
   - Nút lá bị chẻ đôi: 2 khoá ở lại nút cũ, 3 khoá sang nút lá mới bên phải.
   - Khoá **đầu tiên của lá mới** được **CHÉP** lên (copy-up) làm khoá dẫn đường cho nút cha. Nó vẫn nằm nguyên trong lá — ở B+ Tree, mọi bản ghi luôn ở tầng lá, nên tách nút không làm mất bản ghi nào.
   - Nối con trỏ `next_leaf`: lá cũ → lá mới → lá kế tiếp trước đây của lá cũ, để duy trì chuỗi quét dải liên tục.
3. Nếu nút cha (nút trong) cũng bị tràn, nó cũng bị chẻ đôi — nhưng lần này khoá ở giữa được **CHUYỂN** lên (move-up): nút trong chỉ chứa khoá chỉ dẫn, nên giữ một bản sao ở dưới là thừa. Quá trình lan truyền ngược lên trên; nếu nút gốc bị phân tách, một nút gốc mới được sinh ra và chiều cao của toàn bộ cây tăng lên 1 tầng — cây chỉ cao lên ở gốc, nên mọi lá luôn cùng độ sâu.

---

## Mã nguồn minh họa thực chiến (Idiomatic Runnable Rust Blueprint)

Dưới đây là một bản thiết kế mã nguồn Rust hoàn chỉnh, độc lập và mang tính thực chiến cao. Chương trình cài đặt một Cây B+ Tree trong bộ nhớ (In-Memory B+ Tree) đầy đủ các thao tác cốt lõi: **chèn có tách nút lá và nút trong**, tìm kiếm điểm (Point Query), và quét dải dữ liệu (Range Scan) bằng cách **men theo chuỗi `next_leaf`**. Các nút được lưu trong một `Vec` và trỏ tới nhau bằng chỉ số `NodeId` — đúng như trang trên đĩa trỏ tới nhau bằng `page_id`, và cũng là cách duy nhất để lá trỏ sang lá kế bên mà vẫn 100% Safe Rust, không cần `Rc<RefCell<...>>`:

```rust
/// Số khoá tối đa của một nút trước khi bị phân tách
/// (rất nhỏ để thấy được việc tách nút; cơ sở dữ liệu thật dùng hàng trăm)
pub const NODE_CAPACITY: usize = 3;

/// Mã định danh của một nút — đóng vai trò `page_id` của trang trên đĩa.
/// Các nút nằm trong một "kho trang" `Vec`, trỏ tới nhau bằng chỉ số
/// (giống đồ thị dùng chỉ số ở Chương 30), nhờ vậy nút lá trỏ được sang
/// lá kế bên mà không cần `Rc<RefCell<...>>`.
pub type NodeId = usize;

/// Cấu tạo của một Nút trong cây B+ Tree
#[derive(Debug, Clone)]
pub enum BPlusNode<K: Ord + Copy, V: Clone> {
    /// NÚT TRONG (Internal Node): Chỉ chứa Khóa chỉ dẫn và mã các nút con.
    /// Con thứ i quản lý các khoá k với keys[i-1] <= k < keys[i].
    Internal { keys: Vec<K>, children: Vec<NodeId> },
    /// NÚT LÁ (Leaf Node): Chứa Khóa, Dữ liệu thực tế và mã của lá kế tiếp
    Leaf {
        keys: Vec<K>,
        values: Vec<V>,
        next_leaf: Option<NodeId>,
    },
}

/// Cấu trúc cây B+ Tree hoàn chỉnh
pub struct BPlusTree<K: Ord + Copy, V: Clone> {
    nodes: Vec<BPlusNode<K, V>>,
    root: NodeId,
    len: usize,
}

impl<K: Ord + Copy, V: Clone> BPlusTree<K, V> {
    /// Cây rỗng: chỉ có một nút lá rỗng làm gốc
    pub fn new() -> Self {
        Self {
            nodes: vec![BPlusNode::Leaf {
                keys: Vec::new(),
                values: Vec::new(),
                next_leaf: None,
            }],
            root: 0,
            len: 0,
        }
    }

    pub fn len(&self) -> usize {
        self.len
    }

    pub fn is_empty(&self) -> bool {
        self.len == 0
    }

    pub fn root_id(&self) -> NodeId {
        self.root
    }

    pub fn node(&self, id: NodeId) -> &BPlusNode<K, V> {
        &self.nodes[id]
    }

    /// Chiều cao cây (cây chỉ có một lá có chiều cao 1)
    pub fn height(&self) -> usize {
        let mut h = 1;
        let mut id = self.root;
        while let BPlusNode::Internal { children, .. } = &self.nodes[id] {
            id = children[0];
            h += 1;
        }
        h
    }

    /// Đi từ gốc xuống nút lá có thể chứa `key` — mỗi tầng là một lần "đọc trang"
    fn find_leaf(&self, key: &K) -> NodeId {
        let mut id = self.root;
        while let BPlusNode::Internal { keys, children } = &self.nodes[id] {
            // Số khoá chỉ dẫn <= key chính là chỉ số nhánh con cần đi xuống
            let idx = keys.partition_point(|k| k <= key);
            id = children[idx];
        }
        id
    }

    /// Tìm kiếm điểm (Point Query)
    pub fn get(&self, key: &K) -> Option<&V> {
        match &self.nodes[self.find_leaf(key)] {
            BPlusNode::Leaf { keys, values, .. } => {
                // Tại nút lá: tìm kiếm nhị phân trên mảng khoá đã sắp xếp
                keys.binary_search(key).ok().map(|idx| &values[idx])
            }
            BPlusNode::Internal { .. } => unreachable!("find_leaf luôn dừng ở nút lá"),
        }
    }

    /// Thêm (hoặc cập nhật) một cặp khoá–giá trị. Trả về giá trị cũ nếu khoá đã có.
    pub fn insert(&mut self, key: K, value: V) -> Option<V> {
        let (old, split) = self.insert_rec(self.root, key, value);
        if let Some((separator, right)) = split {
            // Gốc bị tách -> sinh gốc mới, cây cao thêm đúng một tầng
            self.nodes.push(BPlusNode::Internal {
                keys: vec![separator],
                children: vec![self.root, right],
            });
            self.root = self.nodes.len() - 1;
        }
        if old.is_none() {
            self.len += 1;
        }
        old
    }

    /// Chèn đệ quy. Nếu nút `id` bị tách, trả về (khoá phân tách, mã nút mới bên phải)
    /// để nút cha chèn vào chính nó.
    fn insert_rec(&mut self, id: NodeId, key: K, value: V) -> (Option<V>, Option<(K, NodeId)>) {
        match &mut self.nodes[id] {
            BPlusNode::Leaf { keys, values, .. } => {
                match keys.binary_search(&key) {
                    // Khoá đã tồn tại -> cập nhật đè, không tách gì cả
                    Ok(idx) => return (Some(std::mem::replace(&mut values[idx], value)), None),
                    // Chèn vào đúng vị trí để duy trì thứ tự tăng dần — O(M)
                    Err(idx) => {
                        keys.insert(idx, key);
                        values.insert(idx, value);
                    }
                }
                if keys.len() > NODE_CAPACITY {
                    (None, Some(self.split_leaf(id)))
                } else {
                    (None, None)
                }
            }
            BPlusNode::Internal { keys, children } => {
                let idx = keys.partition_point(|k| *k <= key);
                let child = children[idx];
                let (old, split) = self.insert_rec(child, key, value);
                let Some((separator, right)) = split else {
                    return (old, None);
                };
                let BPlusNode::Internal { keys, children } = &mut self.nodes[id] else {
                    unreachable!()
                };
                keys.insert(idx, separator);
                children.insert(idx + 1, right);
                if keys.len() > NODE_CAPACITY {
                    (old, Some(self.split_internal(id)))
                } else {
                    (old, None)
                }
            }
        }
    }

    /// Tách nút lá: nửa sau sang lá mới. Khoá đầu của lá mới được CHÉP lên cha
    /// (nó vẫn nằm trong lá — dữ liệu không bao giờ rời tầng lá).
    fn split_leaf(&mut self, id: NodeId) -> (K, NodeId) {
        let new_id = self.nodes.len();
        let BPlusNode::Leaf {
            keys,
            values,
            next_leaf,
        } = &mut self.nodes[id]
        else {
            unreachable!()
        };
        let mid = keys.len() / 2;
        let right_keys = keys.split_off(mid);
        let right_values = values.split_off(mid);
        let separator = right_keys[0];
        // Nối xích: lá cũ -> lá mới -> lá kế tiếp cũ
        let old_next = next_leaf.replace(new_id);
        self.nodes.push(BPlusNode::Leaf {
            keys: right_keys,
            values: right_values,
            next_leaf: old_next,
        });
        (separator, new_id)
    }

    /// Tách nút trong: khoá ở giữa được CHUYỂN lên cha (không ở lại nút nào bên dưới).
    fn split_internal(&mut self, id: NodeId) -> (K, NodeId) {
        let new_id = self.nodes.len();
        let BPlusNode::Internal { keys, children } = &mut self.nodes[id] else {
            unreachable!()
        };
        let mid = keys.len() / 2;
        let mut right_keys = keys.split_off(mid);
        let separator = right_keys.remove(0);
        let right_children = children.split_off(mid + 1);
        self.nodes.push(BPlusNode::Internal {
            keys: right_keys,
            children: right_children,
        });
        (separator, new_id)
    }

    /// Quét dải [min_key, max_key]: đi xuống đúng MỘT lá, rồi men theo
    /// chuỗi `next_leaf` sang phải — không bao giờ phải quay lại gốc.
    pub fn range(&self, min_key: K, max_key: K) -> Vec<(K, V)> {
        let mut out = Vec::new();
        let mut current = Some(self.find_leaf(&min_key));
        while let Some(id) = current {
            let BPlusNode::Leaf {
                keys,
                values,
                next_leaf,
            } = &self.nodes[id]
            else {
                unreachable!()
            };
            for (k, v) in keys.iter().zip(values) {
                if *k > max_key {
                    return out; // đã vượt cận trên -> dừng sớm
                }
                if *k >= min_key {
                    out.push((*k, v.clone()));
                }
            }
            current = *next_leaf;
        }
        out
    }

    /// Mã của các nút lá theo thứ tự chuỗi `next_leaf`, bắt đầu từ lá trái nhất
    pub fn leaf_chain(&self) -> Vec<NodeId> {
        let mut id = self.root;
        while let BPlusNode::Internal { children, .. } = &self.nodes[id] {
            id = children[0];
        }
        let mut chain = vec![id];
        while let BPlusNode::Leaf {
            next_leaf: Some(next),
            ..
        } = &self.nodes[id]
        {
            chain.push(*next);
            id = *next;
        }
        chain
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

    // Mỗi nút chứa tối đa NODE_CAPACITY = 3 khoá, nên chỉ vài lần chèn là cây phải tách nút
    let mut b_tree: BPlusTree<i32, &str> = BPlusTree::new();
    let people = [
        (50, "Cường (TP.HCM)"),
        (10, "Alice (Hà Nội)"),
        (90, "Emmy (Hải Phòng)"),
        (30, "Bình (Đà Nẵng)"),
        (70, "Dũng (Cần Thơ)"),
        (20, "Giang (Huế)"),
        (80, "Hà (Vinh)"),
        (40, "Khánh (Quy Nhơn)"),
        (60, "Lan (Nha Trang)"),
    ];

    println!("[0] Chèn {} bản ghi theo thứ tự lộn xộn:", people.len());
    for (key, name) in people {
        b_tree.insert(key, name);
        println!(
            "    - Chèn khoá {:2} -> chiều cao cây = {}, số lá = {}",
            key,
            b_tree.height(),
            b_tree.leaf_chain().len()
        );
    }
    assert_eq!(b_tree.len(), 9);
    assert_eq!(b_tree.height(), 2);

    // In chuỗi lá: các lá được nối với nhau theo thứ tự khoá tăng dần
    print!("    - Chuỗi lá (next_leaf):");
    for id in b_tree.leaf_chain() {
        if let BPlusNode::Leaf { keys, .. } = b_tree.node(id) {
            print!(" {:?} ->", keys);
        }
    }
    println!(" None");

    println!("\n[1] Kiểm tra tính năng tìm kiếm điểm (Point Search):");
    let result_30 = b_tree.get(&30);
    println!("    - Tra cứu khóa 30: {:?}", result_30);
    assert_eq!(result_30, Some(&"Bình (Đà Nẵng)"));

    let result_70 = b_tree.get(&70);
    println!("    - Tra cứu khóa 70: {:?}", result_70);
    assert_eq!(result_70, Some(&"Dũng (Cần Thơ)"));

    let result_99 = b_tree.get(&99);
    println!("    - Tra cứu khóa 99 (không tồn tại): {:?}", result_99);
    assert_eq!(result_99, None);

    println!("\n[2] Kiểm tra tính năng quét dải dữ liệu (Range Scan) qua chuỗi lá:");
    println!("    - Tìm kiếm các bản ghi có khóa từ 25 đến 75:");
    let range_entries = b_tree.range(25, 75);
    for (k, v) in &range_entries {
        println!("      -> Khóa {}: {}", k, v);
    }

    // Kết quả kỳ vọng: Khóa 30, 40, 50, 60, 70
    let found: Vec<i32> = range_entries.iter().map(|(k, _)| *k).collect();
    assert_eq!(found, vec![30, 40, 50, 60, 70]);
    println!("    => Quét dải dữ liệu hoàn tất: chỉ đi xuống một lần, rồi men theo next_leaf!");

    println!("============================================================");
    println!("               HOÀN TẤT THỰC NGHIỆM CHƯƠNG 33               ");
    println!("============================================================");
}
```

---

## Bảng tra cứu lỗi biên dịch & Cách khắc phục (Compiler Error Guide)

Dưới đây là các lỗi biên dịch điển hình khi xây dựng cấu trúc cây B-Tree và B+ Tree trong Rust:

| Mã lỗi | Thông báo mẫu từ trình biên dịch | Nguyên nhân cốt lõi | Cách khắc phục nhanh |
|---|---|---|---|
| **E0507** | `cannot move out of index of 'Vec<Box<DemoNode>>'` | Bạn cố lấy một nút con `Box` ra khỏi `children[0]` bằng cú pháp gán trực tiếp trong khi chỉ có tham chiếu mượn (xem ví dụ dưới; cây trong chương này dùng `NodeId` là `Copy` nên không gặp lỗi này). | Dùng tham chiếu mượn `&children[idx]` khi duyệt, hoặc dùng `.remove()` nếu có quyền khả biến `&mut`. |
| **E0277** | `the trait bound 'K: Ord' is not satisfied` | B+ Tree bắt buộc các khóa phải có thứ tự sắp xếp tuyệt đối để chia nhánh nhị phân. Nếu kiểu khóa `K` chưa cài trait `Ord`, trình biên dịch sẽ chặn lại. | Bổ sung ràng buộc trait: `impl<K: Ord, V> ...`. |
| **E0596** | `cannot borrow '...' as mutable, as it is not declared as mutable` | Bạn cố chèn thêm khóa vào nút lá trong khi biến cây hoặc nút được khai báo bằng `let` bất biến. | Khai báo với từ khóa `let mut`. |
| **E0004** | `non-exhaustive patterns: '&BPlusNode::Internal { .. }' not covered` | Bạn dùng khối lệnh `match` trên enum `BPlusNode` nhưng chỉ xử lý trường hợp `Leaf` mà bỏ sót trường hợp `Internal`. | Bổ sung đầy đủ các nhánh `match` cho cả hai biến thể của enum. |

### Ví dụ phân tích lỗi `E0507` khi truy xuất nút con trong B+ Tree:

```rust
enum DemoNode {
    Leaf(Vec<i32>),
    Internal(Vec<Box<DemoNode>>),
}

// Đoạn mã lỗi minh họa E0507: Cố đoạt quyền sở hữu con trỏ Box từ tham chiếu mượn
fn child_broken(node: &DemoNode) {
    match node {
        DemoNode::Internal(children) => {
            // let first_child = children[0]; // LỖI E0507: cannot move out of index of `Vec<Box<DemoNode>>`!
        }
        _ => {}
    }
}

// Cách sửa chữa đúng chuẩn: Mượn tham chiếu &Box hoặc mượn trực tiếp &NutDemo
fn child_correct(node: &DemoNode) {
    match node {
        DemoNode::Internal(children) => {
            let first_child: &DemoNode = &children[0]; // Chỉ mượn, không di chuyển quyền sở hữu!
            println!("Đã mượn nút con thành công.");
        }
        _ => {}
    }
}
```

---

## Kiểm thử tự động (Automated Tests)

Cây B+ Tree có rất nhiều bất biến dễ vỡ khi tách nút: khoá trong mỗi nút tăng dần, nút trong có đúng `n + 1` con cho `n` khoá, mọi khoá trong một nhánh con nằm đúng trong dải mà khoá chỉ dẫn của cha cho phép, và **mọi lá cùng độ sâu**. Module kiểm thử dưới đây kiểm tất cả các bất biến đó sau 2.000 lần chèn ngẫu nhiên, và **kiểm chứng chéo** từng phép tìm kiếm và quét dải với `std::collections::BTreeMap` của thư viện chuẩn.

```rust
#[cfg(test)]
mod tests {
    use super::*;
    use std::collections::BTreeMap;

    /// Kiểm mọi bất biến cấu trúc; trả về độ sâu của các lá (phải bằng nhau)
    fn check_node<K: Ord + Copy + std::fmt::Debug, V: Clone>(
        tree: &BPlusTree<K, V>,
        id: NodeId,
        lower: Option<K>,
        upper: Option<K>,
    ) -> usize {
        let in_bounds = |k: &K| lower.is_none_or(|lo| *k >= lo) && upper.is_none_or(|hi| *k < hi);
        match tree.node(id) {
            BPlusNode::Leaf { keys, values, .. } => {
                assert_eq!(keys.len(), values.len());
                assert!(keys.len() <= NODE_CAPACITY);
                assert!(keys.windows(2).all(|w| w[0] < w[1]));
                assert!(keys.iter().all(in_bounds), "khoá lá nằm ngoài dải của cha");
                1
            }
            BPlusNode::Internal { keys, children } => {
                assert_eq!(children.len(), keys.len() + 1);
                assert!(keys.len() <= NODE_CAPACITY);
                assert!(keys.windows(2).all(|w| w[0] < w[1]));
                let depths: Vec<usize> = (0..children.len())
                    .map(|i| {
                        let lo = if i == 0 { lower } else { Some(keys[i - 1]) };
                        let hi = if i == keys.len() {
                            upper
                        } else {
                            Some(keys[i])
                        };
                        check_node(tree, children[i], lo, hi)
                    })
                    .collect();
                assert!(
                    depths.windows(2).all(|w| w[0] == w[1]),
                    "lá không cùng độ sâu"
                );
                depths[0] + 1
            }
        }
    }

    fn pseudo_random_keys(n: usize) -> Vec<i64> {
        // Bộ sinh đồng dư tuyến tính đơn giản — tất định, không cần crate ngoài
        let mut x: u64 = 0x2545_F491_4F6C_DD1D;
        let mut keys = Vec::with_capacity(n);
        for _ in 0..n {
            x = x
                .wrapping_mul(6364136223846793005)
                .wrapping_add(1442695040888963407);
            keys.push((x >> 33) as i64 % 10_000);
        }
        keys
    }

    #[test]
    fn insert_get_and_invariants_match_btreemap() {
        let mut tree = BPlusTree::new();
        let mut model = BTreeMap::new();
        for (i, k) in pseudo_random_keys(2_000).into_iter().enumerate() {
            assert_eq!(
                tree.insert(k, i),
                model.insert(k, i),
                "giá trị cũ khác ở khoá {k}"
            );
        }
        assert_eq!(tree.len(), model.len());
        let depth = check_node(&tree, tree.root_id(), None, None);
        assert_eq!(depth, tree.height());
        for (k, v) in &model {
            assert_eq!(tree.get(k), Some(v));
        }
        assert_eq!(tree.get(&-1), None);
    }

    #[test]
    fn range_scan_matches_btreemap_range() {
        let mut tree = BPlusTree::new();
        let mut model = BTreeMap::new();
        for k in pseudo_random_keys(1_000) {
            tree.insert(k, k * 10);
            model.insert(k, k * 10);
        }
        for (lo, hi) in [
            (0, 10_000),
            (123, 456),
            (5_000, 5_000),
            (9_990, 20_000),
            (-50, 3),
        ] {
            let expected: Vec<(i64, i64)> = model.range(lo..=hi).map(|(k, v)| (*k, *v)).collect();
            assert_eq!(tree.range(lo, hi), expected, "dải [{lo}, {hi}]");
        }
        assert!(tree.range(10, 5).is_empty(), "dải ngược phải rỗng");
    }

    #[test]
    fn leaf_chain_visits_every_key_in_order() {
        let mut tree = BPlusTree::new();
        for k in (0..100).rev() {
            tree.insert(k, ());
        }
        let mut all = Vec::new();
        for id in tree.leaf_chain() {
            let BPlusNode::Leaf { keys, .. } = tree.node(id) else {
                panic!("chuỗi lá chứa nút trong")
            };
            all.extend_from_slice(keys);
        }
        assert_eq!(all, (0..100).collect::<Vec<_>>());
    }

    #[test]
    fn leaf_split_copies_separator_up() {
        // 4 khoá với sức chứa 3 -> lá tách thành [1, 2] và [3, 4], khoá 3 được CHÉP lên gốc
        let mut tree = BPlusTree::new();
        for k in [1, 2, 3, 4] {
            tree.insert(k, k);
        }
        let BPlusNode::Internal { keys, children } = tree.node(tree.root_id()) else {
            panic!("gốc phải là nút trong sau khi tách")
        };
        assert_eq!(keys, &vec![3]);
        let BPlusNode::Leaf { keys: right, .. } = tree.node(children[1]) else {
            panic!()
        };
        assert_eq!(right, &vec![3, 4], "khoá phân tách vẫn nằm trong lá phải");
        assert_eq!(tree.len(), 4, "không mất bản ghi nào khi tách");
        for k in 1..=4 {
            assert_eq!(tree.get(&k), Some(&k));
        }
    }

    #[test]
    fn updating_existing_key_does_not_grow_tree() {
        let mut tree = BPlusTree::new();
        assert_eq!(tree.insert(7, "a"), None);
        assert_eq!(tree.insert(7, "b"), Some("a"));
        assert_eq!(tree.len(), 1);
        assert_eq!(tree.get(&7), Some(&"b"));
    }

    #[test]
    fn empty_tree() {
        let tree: BPlusTree<i32, i32> = BPlusTree::new();
        assert!(tree.is_empty());
        assert_eq!(tree.height(), 1);
        assert_eq!(tree.get(&1), None);
        assert!(tree.range(0, 100).is_empty());
    }
}
```

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
/// Bất biến sống còn của B+ Tree: khoá trong MỌI nút phải tăng ngặt.
/// Nếu sai thì `get` bằng tìm kiếm nhị phân sẽ trả kết quả sai — âm thầm.
fn keys_strictly_increasing<K: Ord + Copy, V: Clone>(tree: &BPlusTree<K, V>, id: NodeId) -> bool {
    match tree.node(id) {
        BPlusNode::Leaf { keys, values, .. } => {
            assert_eq!(keys.len(), values.len(), "mỗi khoá phải có đúng một giá trị");
            keys.windows(2).all(|w| w[0] < w[1])
        }
        BPlusNode::Internal { keys, children } => {
            assert_eq!(
                children.len(),
                keys.len() + 1,
                "nút trong luôn có nhiều hơn khoá đúng một con"
            );
            keys.windows(2).all(|w| w[0] < w[1])
                && children.iter().all(|&c| keys_strictly_increasing(tree, c))
        }
    }
}

#[test]
fn every_node_keeps_keys_sorted() {
    let mut tree: BPlusTree<i32, String> = BPlusTree::new();
    // Nạp theo thứ tự LỘN XỘN — bất biến vẫn phải đúng sau mọi lần tách nút.
    for k in [50, 10, 90, 30, 70, 20, 80, 40, 60] {
        tree.insert(k, format!("v{k}"));
    }
    assert!(keys_strictly_increasing(&tree, tree.root_id()));

    // Cây rỗng cũng phải thoả (không có cặp nào để so).
    let empty: BPlusTree<i32, String> = BPlusTree::new();
    assert!(keys_strictly_increasing(&empty, empty.root_id()));
}
```

`windows(2).all(...)` đúng ngay cả với 0 hoặc 1 phần tử — `windows` trả về iterator rỗng, và `all` trên iterator rỗng là `true`. Đó là hành vi đúng: một nút có một khoá thì hiển nhiên đã sắp xếp.

Hai `assert_eq!` bên trong bắt những bất biến *khác* mà đề bài không hỏi nhưng cũng quan trọng: số giá trị khớp số khoá ở lá, và nút trong luôn có `n+1` con cho `n` khoá. Bất biến vỡ ở đây thường là dấu hiệu tách nút sai. Hàm `check_node` trong module kiểm thử của chương đi xa hơn một bước: nó còn kiểm mọi khoá của nhánh con nằm đúng dải mà nút cha cho phép, và mọi lá cùng độ sâu.
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
  → nút cũ không bao giờ nhận thêm khoá nữa
  → ghi ra đĩa gần như TUẦN TỰ
```

**Khoá ngẫu nhiên (`UUID v4`):**

```
Mỗi lần chèn rơi vào một nút lá NGẪU NHIÊN trong hàng nghìn nút.
  → nút đích thường KHÔNG có trong buffer pool  → phải đọc đĩa trước khi ghi
  → tách nút xảy ra khắp nơi, mỗi lần tách để lại HAI nút đầy ~50%,
    rồi chúng được lấp dần một cách ngẫu nhiên
  → ghi ra đĩa NGẪU NHIÊN
```

Ba hậu quả đo được:

| | Khoá tăng dần | Khoá ngẫu nhiên |
|---|---|---|
| Độ đầy nút lá | ~100% (nếu tách lệch ở rìa phải) | **~69%** (ln 2) — cây to hơn ~45% |
| Trang phải đọc trước khi ghi | gần như không | gần như mọi lần |
| Kiểu ghi đĩa | tuần tự | ngẫu nhiên |

**Độ đầy của nút** đáng nói riêng: khi một nút đầy bị tách, nó chia đôi thành hai nút mỗi nút đầy một nửa. Với khoá ngẫu nhiên, các nút nửa đầy này được lấp dần rồi lại bị tách — phân tích kinh điển (Yao, 1978) cho thấy độ đầy trung bình ổn định quanh **ln 2 ≈ 69%**. Cây phình to thêm khoảng 45%, có thể cao thêm một tầng, và buffer pool chứa được ít dữ liệu hữu ích hơn.

Với khoá tăng dần thì khác: nút trái sau khi tách không bao giờ nhận thêm khoá nào nữa. Nếu tách **chia đôi** như cây trong chương này, mọi nút trái bị bỏ lại ở mức ~50% — còn tệ hơn khoá ngẫu nhiên! Vì vậy các hệ quản trị thật đều có tối ưu riêng cho chèn ở rìa phải: khi nút bị tràn là lá tận cùng bên phải, chúng tách **lệch** (để nút trái gần đầy, nút mới gần rỗng). Nhờ đó khoá tăng dần cho các nút đầy gần 100% — một chi tiết nhỏ nhưng là lý do thật đằng sau lời khuyên dùng khoá tự tăng.

**Vì sao UUID v7 giải quyết được:** nó đặt dấu thời gian mili giây vào 48 bit đầu, nên các UUID sinh gần nhau về thời gian thì cũng gần nhau về giá trị. Vẫn ngẫu nhiên đủ để không đoán được, nhưng **sắp xếp gần như tăng dần** — lấy được cả tính cục bộ của khoá tự tăng lẫn tính phân tán của UUID.
</details>
