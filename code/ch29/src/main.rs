#![allow(dead_code, unused_variables, unused_imports)]
/// Cấu trúc một nút bên trong Cây nhị phân tìm kiếm
#[derive(Debug)]
pub struct TreeNode<T> {
    pub value: T,
    pub left: Option<Box<TreeNode<T>>>,
    pub right: Option<Box<TreeNode<T>>>,
}

impl<T> TreeNode<T> {
    pub fn new(value: T) -> Self {
        TreeNode {
            value,
            left: None,
            right: None,
        }
    }
}

/// Cấu trúc Cây nhị phân tìm kiếm hoàn chỉnh
#[derive(Debug)]
pub struct BinarySearchTree<T: Ord> {
    root: Option<Box<TreeNode<T>>>,
    size: usize,
}

impl<T: Ord> BinarySearchTree<T> {
    /// Khởi tạo một cây BST rỗng
    pub fn new() -> Self {
        BinarySearchTree {
            root: None,
            size: 0,
        }
    }

    /// Thêm một phần tử vào cây - Duy trì tính chất BST
    pub fn insert(&mut self, value: T) {
        if Self::insert_recursive(&mut self.root, value) {
            self.size += 1;
        }
    }

    fn insert_recursive(node: &mut Option<Box<TreeNode<T>>>, value: T) -> bool {
        match node {
            // Khi tìm thấy vị trí lá trống thích hợp: Tạo Box mới
            None => {
                *node = Some(Box::new(TreeNode::new(value)));
                true
            }
            Some(current) => {
                if value < current.value {
                    Self::insert_recursive(&mut current.left, value)
                } else if value > current.value {
                    Self::insert_recursive(&mut current.right, value)
                } else {
                    // Giá trị đã tồn tại trong cây (không cho phép trùng lặp)
                    false
                }
            }
        }
    }

    /// Tìm kiếm một giá trị trong cây - Tốc độ O(log N)
    pub fn contains_key(&self, value: &T) -> bool {
        let mut pointer = &self.root;
        while let Some(node) = pointer {
            if value == &node.value {
                return true;
            } else if value < &node.value {
                pointer = &node.left;
            } else {
                pointer = &node.right;
            }
        }
        false
    }

    /// Duyệt cây theo Trung thứ tự (In-order: Trái -> Gốc -> Phải)
    /// Trả về một Vector chứa các tham chiếu mượn được sắp xếp tăng dần!
    pub fn in_order_walk(&self) -> Vec<&T> {
        let mut out = Vec::new();
        Self::collect_in_order(&self.root, &mut out);
        out
    }

    fn collect_in_order<'a>(node: &'a Option<Box<TreeNode<T>>>, out: &mut Vec<&'a T>) {
        if let Some(current) = node {
            // 1. Duyệt toàn bộ cây con bên trái
            Self::collect_in_order(&current.left, out);
            // 2. Thu thập nút hiện tại
            out.push(&current.value);
            // 3. Duyệt toàn bộ cây con bên phải
            Self::collect_in_order(&current.right, out);
        }
    }

    /// Tính chiều cao của cây (Độ sâu tối đa từ gốc đến lá xa nhất)
    pub fn height(&self) -> usize {
        Self::recursive_height(&self.root)
    }

    fn recursive_height(node: &Option<Box<TreeNode<T>>>) -> usize {
        match node {
            None => 0,
            Some(current) => {
                let left_height = Self::recursive_height(&current.left);
                let right_height = Self::recursive_height(&current.right);
                1 + left_height.max(right_height)
            }
        }
    }

    pub fn len(&self) -> usize {
        self.size
    }

    pub fn is_empty(&self) -> bool {
        self.size == 0
    }
}

impl<T: Ord> Default for BinarySearchTree<T> {
    fn default() -> Self {
        Self::new()
    }
}

fn main() {
    println!("============================================================");
    println!("    HIỆN THỰC CÂY NHỊ PHÂN TÌM KIẾM (BST) AN TOÀN TRONG RUST");
    println!("============================================================");

    let mut bst: BinarySearchTree<i32> = BinarySearchTree::new();

    // 1. Thêm các phần tử vào cây
    // Cấu trúc dự kiến:
    //          50
    //        /    \
    //       30     70
    //      /  \   /  \
    //     20  40 60  80
    println!("[1] Nạp các giá trị vào Cây nhị phân tìm kiếm:");
    let values = [50, 30, 70, 20, 40, 60, 80];
    for &value in &values {
        bst.insert(value);
        print!("{} ", value);
    }
    println!("\n    - Tổng số nút trong cây: {}", bst.len());
    assert_eq!(bst.len(), 7);

    // 2. Kiểm tra chiều cao của cây
    let height = bst.height();
    println!("\n[2] Chiều cao của cây: {}", height);
    assert_eq!(height, 3); // 3 tầng: 50 -> (30,70) -> (20,40,60,80)

    // 3. Kiểm tra tính năng tìm kiếm O(log N)
    println!("\n[3] Kiểm tra tính năng tìm kiếm nhị phân:");
    println!("    - Tìm số 40: {}", bst.contains_key(&40));
    println!("    - Tìm số 99: {}", bst.contains_key(&99));
    assert!(bst.contains_key(&40));
    assert!(!bst.contains_key(&99));

    // 4. Duyệt In-order xác nhận dãy số tăng dần hoàn hảo
    println!("\n[4] Duyệt cây In-order (Trái -> Gốc -> Phải):");
    let sorted = bst.in_order_walk();
    print!("    - Kết quả in: ");
    for &value in &sorted {
        print!("{} ", value);
    }
    println!();

    let expected = vec![&20, &30, &40, &50, &60, &70, &80];
    assert_eq!(sorted, expected);
    println!("    => Dãy số được sắp xếp tăng dần hoàn hảo đúng theo lý thuyết BST!");

    println!("============================================================");
    println!("               HOÀN TẤT THỰC NGHIỆM CHƯƠNG 29               ");
    println!("============================================================");
}

#[cfg(test)]
mod tests {
    use super::*;

    fn sample_tree() -> BinarySearchTree<i32> {
        let mut c = BinarySearchTree::new();
        for x in [50, 30, 70, 20, 40, 60, 80] {
            c.insert(x);
        }
        c
    }

    #[test]
    fn in_order_walk_is_sorted() {
        let c = sample_tree();
        let values: Vec<i32> = c.in_order_walk().into_iter().copied().collect();
        assert_eq!(values, vec![20, 30, 40, 50, 60, 70, 80]); // BST in-order = sắp xếp
    }

    #[test]
    fn contains_key() {
        let c = sample_tree();
        assert!(c.contains_key(&40));
        assert!(c.contains_key(&80));
        assert!(!c.contains_key(&99));
        assert!(!c.contains_key(&35));
    }

    #[test]
    fn no_duplicate_inserts() {
        let mut c = BinarySearchTree::new();
        c.insert(5);
        c.insert(5); // giá trị trùng bị bỏ qua
        c.insert(5);
        assert_eq!(c.len(), 1);
    }

    #[test]
    fn balanced_tree_is_shallower_than_degenerate() {
        let mut degenerate = BinarySearchTree::new();
        for x in 1..=7 {
            degenerate.insert(x); // chèn tuần tự -> suy biến thành danh sách
        }
        assert_eq!(degenerate.height(), 7);
        assert_eq!(sample_tree().height(), 3); // cân đối -> ~log N
    }

    #[test]
    fn empty_tree() {
        let c: BinarySearchTree<i32> = BinarySearchTree::new();
        assert!(c.is_empty());
        assert_eq!(c.height(), 0);
        assert_eq!(c.in_order_walk().len(), 0);
    }
}
