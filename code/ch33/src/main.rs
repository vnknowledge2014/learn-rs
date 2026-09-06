#![allow(dead_code, unused_variables, unused_imports)]
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
                    // Tối ưu hóa: Chỉ đi xuống nhánh con nếu khoảng khóa có giao thoa
                    let gioi_han_duoi_thoa = if i == 0 { true } else { keys[i - 1] <= *max_key };
                    let gioi_han_tren_thoa = if i == keys.len() { true } else { keys[i] >= *min_key };
                    if gioi_han_duoi_thoa && gioi_han_tren_thoa {
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
