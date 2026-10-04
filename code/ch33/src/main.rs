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
