#![allow(dead_code, unused_variables, unused_imports)]
/// Cấu trúc một nút bên trong Cây nhị phân tìm kiếm
#[derive(Debug)]
pub struct NutCay<T> {
    pub value: T,
    pub left: Option<Box<NutCay<T>>>,
    pub must: Option<Box<NutCay<T>>>,
}

impl<T> NutCay<T> {
    pub fn new(value: T) -> Self {
        NutCay {
            value,
            left: None,
            must: None,
        }
    }
}

/// Cấu trúc Cây nhị phân tìm kiếm hoàn chỉnh
#[derive(Debug)]
pub struct BinarySearchTree<T: Ord> {
    root: Option<Box<NutCay<T>>>,
    quantity: usize,
}

impl<T: Ord> BinarySearchTree<T> {
    /// Khởi tạo một cây BST rỗng
    pub fn new() -> Self {
        BinarySearchTree {
            root: None,
            quantity: 0,
        }
    }

    /// Thêm một phần tử vào cây - Duy trì tính chất BST
    pub fn them(&mut self, value: T) {
        if Self::insert_recursive(&mut self.root, value) {
            self.quantity += 1;
        }
    }

    fn insert_recursive(nut: &mut Option<Box<NutCay<T>>>, value: T) -> bool {
        match nut {
            // Khi tìm thấy vị trí lá trống thích hợp: Tạo Box mới
            None => {
                *nut = Some(Box::new(NutCay::new(value)));
                true
            }
            Some(current) => {
                if value < current.value {
                    Self::insert_recursive(&mut current.left, value)
                } else if value > current.value {
                    Self::insert_recursive(&mut current.must, value)
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
        while let Some(nut) = pointer {
            if value == &nut.value {
                return true;
            } else if value < &nut.value {
                pointer = &nut.left;
            } else {
                pointer = &nut.must;
            }
        }
        false
    }

    /// Duyệt cây theo Trung thứ tự (In-order: Trái -> Gốc -> Phải)
    /// Trả về một Vector chứa các tham chiếu mượn được sắp xếp tăng dần!
    pub fn in_order_walk(&self) -> Vec<&T> {
        let mut ket_qua = Vec::new();
        Self::collect_in_order(&self.root, &mut ket_qua);
        ket_qua
    }

    fn collect_in_order<'a>(nut: &'a Option<Box<NutCay<T>>>, ket_qua: &mut Vec<&'a T>) {
        if let Some(current) = nut {
            // 1. Duyệt toàn bộ cây con bên trái
            Self::collect_in_order(&current.left, ket_qua);
            // 2. Thu thập nút hiện tại
            ket_qua.push(&current.value);
            // 3. Duyệt toàn bộ cây con bên phải
            Self::collect_in_order(&current.must, ket_qua);
        }
    }

    /// Tính chiều high của cây (Độ sâu tối đa từ gốc đến lá xa nhất)
    pub fn height(&self) -> usize {
        Self::recursive_height(&self.root)
    }

    fn recursive_height(nut: &Option<Box<NutCay<T>>>) -> usize {
        match nut {
            None => 0,
            Some(current) => {
                let high_left = Self::recursive_height(&current.left);
                let high_must = Self::recursive_height(&current.must);
                1 + high_left.max(high_must)
            }
        }
    }

    pub fn len(&self) -> usize {
        self.quantity
    }

    pub fn is_empty(&self) -> bool {
        self.quantity == 0
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

    let mut cay_bst: BinarySearchTree<i32> = BinarySearchTree::new();

    // 1. Thêm các phần tử vào cây
    // Cấu trúc dự kiến:
    //          50
    //        /    \
    //       30     70
    //      /  \   /  \
    //     20  40 60  80
    println!("[1] Nạp các giá trị vào Cây nhị phân tìm kiếm:");
    let all_num = [50, 30, 70, 20, 40, 60, 80];
    for &so in &all_num {
        cay_bst.them(so);
        print!("{} ", so);
    }
    println!("\n    - Tổng số nút trong cây: {}", cay_bst.len());
    assert_eq!(cay_bst.len(), 7);

    // 2. Kiểm tra chiều high của cây
    let height = cay_bst.height();
    println!("\n[2] Chiều high của cây: {}", height);
    assert_eq!(height, 3); // 3 tầng: 50 -> (30,70) -> (20,40,60,80)

    // 3. Kiểm tra tính năng tìm kiếm O(log N)
    println!("\n[3] Kiểm tra tính năng tìm kiếm nhị phân:");
    println!("    - Tìm số 40: {}", cay_bst.contains_key(&40));
    println!("    - Tìm số 99: {}", cay_bst.contains_key(&99));
    assert!(cay_bst.contains_key(&40));
    assert!(!cay_bst.contains_key(&99));

    // 4. Duyệt In-order xác nhận dãy số tăng dần hoàn hảo
    println!("\n[4] Duyệt cây In-order (Trái -> Gốc -> Phải):");
    let list_up_derive = cay_bst.in_order_walk();
    print!("    - Kết quả in: ");
    for &value in &list_up_derive {
        print!("{} ", value);
    }
    println!();

    let expectation = vec![&20, &30, &40, &50, &60, &70, &80];
    assert_eq!(list_up_derive, expectation);
    println!("    => Dãy số được sắp xếp tăng dần hoàn hảo đúng theo lý thuyết BST!");

    println!("============================================================");
    println!("               HOÀN TẤT THỰC NGHIỆM CHƯƠNG 25               ");
    println!("============================================================");
}


#[cfg(test)]
mod tests {
    use super::*;

    fn cay_mau() -> BinarySearchTree<i32> {
        let mut c = BinarySearchTree::new();
        for x in [50, 30, 70, 20, 40, 60, 80] {
            c.them(x);
        }
        c
    }

    #[test]
    fn in_order_walk_is_sorted() {
        let c = cay_mau();
        let so: Vec<i32> = c.in_order_walk().into_iter().copied().collect();
        assert_eq!(so, vec![20, 30, 40, 50, 60, 70, 80]); // BST in-order = sắp xếp
    }

    #[test]
    fn contains_key() {
        let c = cay_mau();
        assert!(c.contains_key(&40));
        assert!(c.contains_key(&80));
        assert!(!c.contains_key(&99));
        assert!(!c.contains_key(&35));
    }

    #[test]
    fn no_duplicate_inserts() {
        let mut c = BinarySearchTree::new();
        c.them(5);
        c.them(5); // giá trị trùng bị bỏ qua
        c.them(5);
        assert_eq!(c.len(), 1);
    }

    #[test]
    fn balanced_tree_is_shallower_than_degenerate() {
        let mut suy_bien = BinarySearchTree::new();
        for x in 1..=7 {
            suy_bien.them(x); // chèn tuần tự -> suy biến thành danh sách
        }
        assert_eq!(suy_bien.height(), 7);
        assert_eq!(cay_mau().height(), 3); // cân đối -> ~log N
    }

    #[test]
    fn cay_rong() {
        let c: BinarySearchTree<i32> = BinarySearchTree::new();
        assert!(c.is_empty());
        assert_eq!(c.height(), 0);
        assert_eq!(c.in_order_walk().len(), 0);
    }
}
