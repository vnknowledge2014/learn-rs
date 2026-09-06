# Chương 29: Cây, Cây nhị phân tìm kiếm & Duyệt đệ quy an toàn (Trees, Binary Search Trees & Safe Recursive Traversals)

## Giới thiệu & Mục tiêu học tập

Sau khi đã làm quen với các cấu trúc dữ liệu dạng tuyến tính (Linear Data Structures) như Mảng, Danh sách liên kết (Linked list), Ngăn xếp và Hàng đợi, chúng ta chính thức bước vào thế giới của các cấu trúc dữ liệu dạng phân cấp (Hierarchical Data Structures): **Cấu trúc Cây (Trees)** và đỉnh cao ứng dụng là **Cây nhị phân tìm kiếm (Binary Search Tree - BST)**.

Trong thực tế công nghiệp phần mềm, cấu trúc Cây hiện diện ở khắp mọi nơi:
- Hệ thống tệp tin và thư mục trên ổ đĩa máy tính (thư mục gốc `root`, thư mục con, tệp tin lá).
- Cây cú pháp trừu tượng (Abstract Syntax Tree - AST) mà trình biên dịch `rustc` phân tích mã nguồn.
- Cây DOM trong trình duyệt web đại diện cho cấu trúc trang HTML.
- Các chỉ mục dữ liệu siêu tốc của mọi hệ quản trị cơ sở dữ liệu hiện đại (B-Tree, B+ Tree, LSM-Tree).

Tuy nhiên, cấu trúc Cây trong Rust mang đến một vẻ đẹp kiến trúc độc đáo: Làm thế nào để một nút cha có thể sở hữu hai nút con độc lập, và làm thế nào để thực hiện các phép duyệt đệ quy (Recursive Traversals) an toàn mà không làm rò rỉ bộ nhớ hay vi phạm các quy tắc khắt khe của quyền sở hữu (ownership) và vay mượn (borrow)?

Mục tiêu học tập của chương này:
- Nắm vững các khái niệm hình học và thuật ngữ cốt lõi của cấu trúc Cây: Gốc (Root), Nút cha (Parent), Nút con (Child), Lá (Leaf), Bậc, và Chiều cao cây.
- Hiểu sâu sắc định nghĩa toán học và tính chất kỳ diệu của **Cây nhị phân tìm kiếm (BST)**: Khóa nhánh trái luôn nhỏ hơn khóa nút cha, khóa nhánh phải luôn lớn hơn khóa nút cha.
- Tự tay cài đặt cấu trúc `BinarySearchTree` an toàn 100% bằng Rust sử dụng con trỏ thông minh (smart pointer) `Box<T>`.
- Làm chủ ba phương pháp duyệt cây đệ quy kinh điển: **Tiền thứ tự (Pre-order)**, **Trung thứ tự (In-order)**, và **Hậu thứ tự (Post-order)**; giải thích vì sao duyệt In-order luôn trả về danh sách được sắp xếp tăng dần hoàn hảo.
- Hiểu được hiện tượng cây suy biến (Degenerate Tree) và lý do các hệ cơ sở dữ liệu chuyển sang dùng Cây cân bằng nhiều nhánh.

---

## Hình tượng hóa đời sống (Intuitive Everyday Analogy)

Hãy cùng hình dung cấu trúc Cây qua hai hình ảnh vô cùng quen thuộc và sinh động:

```
┌──────────────────────────────────────────────────────────────────────────────────┐
│                   HÌNH TƯỢNG HÓA CẤU TRÚC CÂY & CÂY NHỊ PHÂN                      │
├──────────────────────────────────────────────────────────────────────────────────┤
│                                                                                  │
│ [1. CÂY PHẢ HỆ GIA ĐÌNH]                                                         │
│                                [Cụ Tổ (Root)]                                    │
│                                      │                                           │
│                      ┌───────────────┴───────────────┐                           │
│                      ▼                               ▼                           │
│                 [Bác Cả]                         [Bác Hai]                       │
│                    │                                 │                           │
│              ┌─────┴─────┐                     ┌─────┴─────┐                     │
│              ▼           ▼                     ▼           ▼                     │
│           [Cháu A]    [Cháu B]              [Cháu C]    [Cháu D]                 │
│          (Nút Lá)    (Nút Lá)              (Nút Lá)    (Nút Lá)                  │
│                                                                                  │
│ [2. TỦ HỒ SƠ PHÂN LOẠI THÔNG MINH CỦA THƯ VIỆN (BST)]                            │
│                                                                                  │
│                          ┌───────────────────────┐                               │
│                          │  Ngăn trung tâm: #50  │                               │
│                          └───────────┬───────────┘                               │
│                   Nhỏ hơn 50         │         Lớn hơn 50                        │
│             ┌────────────────────────┴────────────────────────┐                  │
│             ▼                                                 ▼                  │
│   ┌───────────────────┐                             ┌───────────────────┐        │
│   │ Ngăn trái: #30    │                             │ Ngăn phải: #70    │        │
│   └─────────┬─────────┘                             └─────────┬─────────┘        │
│     < 30    │    > 30                                 < 70    │    > 70          │
│    ┌────────┴────────┐                               ┌────────┴────────┐         │
│    ▼                 ▼                               ▼                 ▼         │
│ [Ngăn #20]       [Ngăn #40]                      [Ngăn #60]        [Ngăn #80]    │
└──────────────────────────────────────────────────────────────────────────────────┘
```

### 1. Cây phả hệ dòng họ — Mối quan hệ phân cấp tự nhiên
- Hãy nhìn vào cuốn gia phả của dòng họ:
  - **Nút gốc (Root)**: Cụ tổ của dòng họ — người không có cha mẹ trong cây phả hệ này, đứng ở vị trí cao nhất.
  - **Nút nhánh (Internal Node)**: Những người con của cụ tổ, vừa là con của cụ, vừa là cha mẹ của thế hệ tiếp theo.
  - **Nút lá (Leaf)**: Thế hệ con cháu mới sinh ra chưa lập gia đình, nằm ở tận cùng của các nhánh cây và không có con cái nối dõi (`None`).
- Trong cây phả hệ, quyền sở hữu (ownership) chảy một chiều từ trên xuống dưới: Cụ tổ truyền lại huyết thống và tài sản cho con cháu; con cháu không thể đồng thời làm cha mẹ của cụ tổ (không có chu trình lặp kín - Acyclic).

### 2. Tủ hồ sơ phân loại thông minh (Cây nhị phân tìm kiếm - BST)
- Hãy tưởng tượng bạn là người thủ thư quản lý hàng vạn tập hồ sơ bệnh án.
- Tại cửa phòng, bạn đặt một ngăn bàn số 50.
- Bạn đặt ra một quy tắc sắt đá:
  - Bất kỳ hồ sơ nào có mã số **nhỏ hơn 50** thì bắt buộc phải chuyển sang **cánh tủ bên trái**.
  - Bất kỳ hồ sơ nào có mã số **lớn hơn 50** thì bắt buộc phải chuyển sang **cánh tủ bên phải**.
- Khi một bác sĩ bước vào hỏi: *"Tìm giúp tôi hồ sơ số 40!"*:
  1. Bạn đứng ở ngăn 50. Vì $40 < 50$, bạn lập tức bước sang cánh tủ bên trái (ngăn 30). Chỉ bằng 1 bước đi, bạn đã loại bỏ hoàn toàn toàn bộ cánh tủ bên phải chứa hàng ngàn hồ sơ lớn hơn 50!
  2. Tại ngăn 30: Vì $40 > 30$, bạn rẽ phải và tìm thấy ngay hồ sơ 40.
- Thay vì phải đi lật từng tập hồ sơ một từ đầu đến cuối ($O(N)$), bạn chỉ cần 2 bước rẽ ($O(\log N)$) là tìm ra chính xác kết quả!

---

## Khái niệm & Cơ chế kỹ thuật chuyên sâu (In-Depth Technical Mechanics)

### 1. Bố cục cấu trúc dữ liệu Cây trong Rust

Một nút cây nhị phân cần lưu trữ giá trị của chính nó và hai liên kết trỏ tới cây con bên trái và cây con bên phải. Vì kích thước của một cây con là đệ quy và co giãn động trên Heap, chúng ta sử dụng con trỏ thông minh (smart pointer) `Box<T>`:

```rust
pub struct NutCay<T> {
    pub value: T,
    pub left: Option<Box<NutCay<T>>>,
    pub right: Option<Box<NutCay<T>>>,
}
```

Trên Ngăn xếp (Stack), một `Option<Box<NutCay<T>>>` chỉ chiếm đúng **8 bytes** (nhờ kỹ thuật tối ưu hóa con trỏ rỗng Null Pointer Optimization của Rust: `None` tương đương giá trị nhị phân `0`, và `Some(box)` là địa chỉ con trỏ hợp lệ khác `0`). Dữ liệu thực tế của các nút được phân bổ linh hoạt trên Vùng nhớ tự do (Heap).

```
          [Root: 50] (Heap 0x1000)
         /                        \
 [Left: 30] (Heap 0x2000)     [Right: 70] (Heap 0x3000)
     /           \
[Leaf: 20]   [Leaf: 40]
```

### 2. Ba phương pháp duyệt cây đệ quy (Recursive Traversals)

Thứ tự xử lý một nút cha so với các nút con của nó quyết định chiến lược duyệt cây:

1. **Duyệt Trung thứ tự (In-order: Trái ➔ Gốc ➔ Phải)**:
   - Đi hết sang nhánh bên trái -> Xử lý nút hiện tại -> Đi sang nhánh bên phải.
   - **Đặc tính thần kỳ**: Đối với Cây nhị phân tìm kiếm (BST), duyệt In-order luôn luôn thăm các phần tử theo thứ tự **tăng dần hoàn hảo** ($20 \rightarrow 30 \rightarrow 40 \rightarrow 50 \rightarrow 70$).
2. **Duyệt Tiền thứ tự (Pre-order: Gốc ➔ Trái ➔ Phải)**:
   - Xử lý nút hiện tại trước tiên -> Đi sang nhánh trái -> Đi sang nhánh phải.
   - Ứng dụng: Sao chép nguyên vẹn cấu trúc cây, hoặc lưu cây xuống đĩa (Serialize).
3. **Duyệt Hậu thứ tự (Post-order: Trái ➔ Phải ➔ Gốc)**:
   - Đi hết nhánh trái -> Đi hết nhánh phải -> Mới xử lý nút hiện tại sau cùng.
   - Ứng dụng: Tính toán dung lượng thư mục (muốn biết thư mục cha nặng bao nhiêu phải cộng tổng dung lượng các tệp con trước), hoặc giải phóng bộ nhớ từ dưới lên.

### 3. Gấp một cái cây: `fold` không chỉ dành cho danh sách

Ở Chương 16 bạn đã dùng `fold` để cô đặc một danh sách thành một giá trị. Nhưng ý tưởng "gấp" tổng quát hơn thế nhiều: **bất kỳ cấu trúc dữ liệu nào duyệt được đều gấp được** — kể cả cây.

```rust
impl<T: Copy> NutCay<T> {
    /// Gấp cây theo thứ tự trung thứ tự (trái → gốc → phải).
    /// `f` nhận (giá trị tích lũy, giá trị nút) và trả về giá trị tích lũy mới.
    pub fn gap<A>(&self, block_make: A, f: &impl Fn(A, T) -> A) -> A {
        let mut acc = block_make;
        if let Some(left) = &self.left {
            acc = left.gap(acc, f);
        }
        acc = f(acc, self.value);
        if let Some(right) = &self.right {
            acc = right.gap(acc, f);
        }
        acc
    }
}
```

Một hàm `gap` duy nhất giờ đây thay thế cho hàng loạt hàm chuyên biệt:

```rust
let tong  = cay.gap(0i64, &|a, x| a + x);            // tính tổng
let count   = cay.gap(0usize, &|a, _| a + 1);          // đếm số nút
let large   = cay.gap(i64::MIN, &|a, x| a.max(x));     // tìm giá trị lớn nhất
let list    = cay.gap(Vec::new(), &|mut a, x| { a.push(x); a }); // xuất ra danh sách đã sắp xếp
```

Khả năng "gấp được" này có tên chính thức là **Foldable**, và giá trị được tính ra bằng cách gấp một cấu trúc đệ quy gọi là một **catamorphism** (phép gấp). Bạn sẽ gặp lại toàn bộ nhóm khái niệm này ở Chương 18 và 19.

> **Ghi nhớ thiết kế**: khi bạn thấy mình sắp viết hàm thứ tư kiểu `tinh_tong_cay`, `dem_nut_cay`, `tim_max_cay`… hãy dừng lại và viết **một** hàm `gap` duy nhất. Ba hàm kia sẽ tự sinh ra từ nó.

### 4. Vấn đề Cây suy biến (Degenerate Tree)

Nếu bạn thêm các số vào BST theo thứ tự đã được sắp xếp sẵn: `[10, 20, 30, 40, 50]`:
- 20 lớn hơn 10 -> nằm bên phải 10.
- 30 lớn hơn 20 -> nằm bên phải 20.
- Chiếc cây bị "lệch hẳn về một bên", biến tướng thành một Danh sách liên kết thẳng đuột!
- Lúc này, chiều cao cây bằng $N$, và tốc độ tìm kiếm bị tụt dốc thảm hại từ $O(\log N)$ về lại $O(N)$.
- Đây chính là lý do vì sao trong các chương sau về Cơ sở dữ liệu (Topic 6), chúng ta sẽ khám phá **Cây B-Tree và B+ Tree** — những cấu trúc cây tự động tái cân bằng (Self-balancing) để luôn giữ vững tốc độ tìm kiếm siêu tốc.

---

## Mã nguồn minh họa thực chiến (Idiomatic Runnable Rust Blueprint)

Dưới đây là một chương trình Rust hoàn chỉnh cài đặt cấu trúc Cây nhị phân tìm kiếm (Binary Search Tree - BST), hỗ trợ thêm phần tử mới, tìm kiếm theo khóa, duyệt In-order tăng dần, và tính chiều cao của cây:

```rust
/// Cấu trúc một nút bên trong Cây nhị phân tìm kiếm
#[derive(Debug)]
pub struct NutCay<T> {
    pub value: T,
    pub left: Option<Box<NutCay<T>>>,
    pub right: Option<Box<NutCay<T>>>,
}

impl<T> NutCay<T> {
    pub fn new(value: T) -> Self {
        NutCay {
            value,
            left: None,
            right: None,
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
        while let Some(nut) = pointer {
            if value == &nut.value {
                return true;
            } else if value < &nut.value {
                pointer = &nut.left;
            } else {
                pointer = &nut.right;
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
            Self::collect_in_order(&current.right, ket_qua);
        }
    }

    /// Tính chiều cao của cây (Độ sâu tối đa từ gốc đến lá xa nhất)
    pub fn height(&self) -> usize {
        Self::recursive_height(&self.root)
    }

    fn recursive_height(nut: &Option<Box<NutCay<T>>>) -> usize {
        match nut {
            None => 0,
            Some(current) => {
                let high_left = Self::recursive_height(&current.left);
                let high_must = Self::recursive_height(&current.right);
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

    // 2. Kiểm tra chiều cao của cây
    let height = cay_bst.height();
    println!("\n[2] Chiều cao của cây: {}", height);
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
```

---

## Bảng tra cứu lỗi biên dịch & Cách khắc phục (Compiler Error Guide)

Khi làm việc với các cấu trúc cây đệ quy trong Rust, lập trình viên thường xuyên đối mặt với các lỗi liên quan đến trait bounds và mượn lồng nhau:

| Mã lỗi | Thông báo mẫu từ trình biên dịch | Nguyên nhân cốt lõi | Cách khắc phục nhanh |
|---|---|---|---|
| **E0277** | `the trait bound 'T: Ord' is not satisfied` | Cây nhị phân tìm kiếm bắt buộc các phần tử phải so sánh được với nhau (`<`, `>`, `==`). Nếu kiểu `T` không thỏa mãn trait `Ord`, phép so sánh sẽ bị cấm. | Bổ sung ràng buộc trait: `impl<T: Ord> BinarySearchTree<T>`. Nếu là struct tự tạo, thêm `#[derive(Ord, PartialOrd, Eq, PartialEq)]`. |
| **E0502** | `cannot borrow 'current.trai' as mutable more than once at a time` | Trong thân hàm đệ quy, bạn vừa mượn nhánh trái làm mutable, vừa cố mượn cả nút cha hoặc nhánh phải trong cùng một biểu thức. | Tách rời các bước rẽ nhánh điều kiện `if/else` để mỗi nhánh mượn nằm trong một khối lệnh độc lập. |
| **E0106** | `missing lifetime specifier` | Khi viết hàm duyệt cây trả về mảng tham chiếu mượn `Vec<&T>`, bạn quên gắn nhãn thời gian sống (lifetime) liên kết giữa cây mượn và danh sách trả về. | Khai báo thời gian sống rõ ràng: `fn thu_thap<'a>(nut: &'a Option<Box<NutCay<T>>>, ket_qua: &mut Vec<&'a T>)`. |
| **E0382** | `use of moved value: 'value'` | Trong hàm đệ quy, bạn truyền `value` bằng giá trị (by value) vào nhánh trái, sau đó lại dùng lại nó trong nhánh phải. | Nếu kiểu `T` không phải là `Copy`, hãy chỉ di chuyển `value` khi chắc chắn rẽ vào nhánh đó, hoặc truyền mượn tham chiếu `&T` khi tìm kiếm. |

### Ví dụ phân tích lỗi `E0277` và cách gắn ràng buộc `Ord`:

```rust
// Định nghĩa một kiểu dữ liệu tùy chỉnh chưa có khả năng so sánh
struct ToaDo {
    x: i32,
    y: i32,
}

// Đoạn mã lỗi minh họa: Cố tạo BST cho kiểu ToaDo
fn broken_bst() {
    // let mut cay = BinarySearchTree::new();
    // cay.them(ToaDo { x: 1, y: 2 }); // LỖI E0277: ToaDo không thỏa mãn trait Ord!
}

// Cách sửa chữa đúng chuẩn: Derive các trait so sánh cần thiết
#[derive(Debug, PartialEq, Eq, PartialOrd, Ord)]
struct CoordIdiomatic {
    x: i32,
    y: i32,
}

fn correct_bst() {
    let mut cay = BinarySearchTree::new();
    cay.them(CoordIdiomatic { x: 10, y: 20 });
    cay.them(CoordIdiomatic { x: 5, y: 15 });
    println!("Cây BST chứa tọa độ hoạt động mượt mà! Số nút = {}", cay.len());
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
```

## Tóm tắt chương & Bài tập rèn luyện (Summary & Exercises)

### 4 Điểm cốt lõi cần ghi nhớ:
1. **Bản chất của BST**: Mọi phần tử nhánh trái đều nhỏ hơn cha, mọi phần tử nhánh phải đều lớn hơn cha. Nhờ đó, thao tác tìm kiếm đạt hiệu năng kỳ diệu $O(\log N)$.
2. **Con trỏ thông minh `Box`**: Là chìa khóa giải quyết bài toán kích thước đệ quy vô hạn của các nút cây trên Ngăn xếp (Stack).
3. **Duyệt In-order thần kỳ**: Thăm cây theo thứ tự Trái -> Gốc -> Phải luôn mang lại danh sách các phần tử được sắp xếp tăng dần hoàn hảo.
4. **Thời gian sống trong duyệt cây**: Khi trả về danh sách các tham chiếu mượn từ cây (`Vec<&T>`), thời gian sống (lifetime) của các tham chiếu bị ràng buộc chặt chẽ vào thời gian sống của bản thân chiếc cây đó.

### Bài tập rèn luyện tự giải:
1. **Bài tập 1 (Tìm giá trị nhỏ nhất và lớn nhất)**:  
   Viết hai phương thức cho `BinarySearchTree`:
   - `fn min(&self) -> Option<&T>`: Lần theo nhánh trái tận cùng để tìm giá trị nhỏ nhất.
   - `fn max(&self) -> Option<&T>`: Lần theo nhánh phải tận cùng để tìm giá trị lớn nhất.  
   *(Giải thích: Tại sao hai thao tác này chỉ tốn thời gian tương đương chiều cao của cây?)*
2. **Bài tập 2 (Đếm số nút lá)**:  
   Viết phương thức `fn leaf_count(&self) -> usize` đếm số lượng nút trong cây không có bất kỳ nút con nào (`left == None && right == None`).
3. **Bài tập 3 (Tư duy mở rộng)**:  
   Điều gì sẽ xảy ra nếu bạn nạp lần lượt các số `[1, 2, 3, 4, 5, 6, 7]` vào cây BST này? Chiều cao của cây sẽ là bao nhiêu? Làm thế nào để cấu trúc B-Tree trong hệ quản trị cơ sở dữ liệu ngăn chặn được hiện tượng suy biến này?

---

### Gợi ý & Lời giải

<details>
<summary><b>Bài tập 1 — Gợi ý</b></summary>

Giá trị nhỏ nhất nằm ở nhánh **trái tận cùng**, lớn nhất ở nhánh **phải tận cùng**. Đây là hệ quả trực tiếp của bất biến BST, không cần so sánh gì thêm.
</details>

<details>
<summary><b>Bài tập 1 — Lời giải</b></summary>

```rust
impl<T: Ord> BinarySearchTree<T> {
    /// Nhỏ nhất = đi trái tới khi không đi được nữa.
    pub fn min(&self) -> Option<&T> {
        let mut hien_tai = self.root.as_ref()?;
        while let Some(t) = hien_tai.left.as_ref() { hien_tai = t; }
        Some(&hien_tai.value)
    }

    /// Lớn nhất = đi phải tới khi không đi được nữa.
    pub fn max(&self) -> Option<&T> {
        let mut hien_tai = self.root.as_ref()?;
        while let Some(p) = hien_tai.right.as_ref() { hien_tai = p; }
        Some(&hien_tai.value)
    }
}

#[test]
fn min_max_khop_voi_duyet_in_order() {
    let mut cay = BinarySearchTree::new();
    for x in [50, 30, 70, 20, 40, 60, 80] { cay.them(x); }

    assert_eq!(cay.min(), Some(&20));
    assert_eq!(cay.max(), Some(&80));

    // Đối chiếu độc lập: duyệt in-order cho dãy TĂNG DẦN,
    // nên đầu dãy là min và cuối dãy là max.
    let day = cay.in_order_walk();
    assert_eq!(cay.min(), day.first().copied());
    assert_eq!(cay.max(), day.last().copied());

    let rong: BinarySearchTree<i32> = BinarySearchTree::new();
    assert_eq!(rong.min(), None);
    assert_eq!(rong.max(), None);
}
```

Cả hai đều là **O(h)** với `h` là chiều cao — không phải O(log n). Với cây cân bằng thì hai con số trùng nhau, với cây suy biến thì `h = n` và bạn quay về tìm kiếm tuyến tính. Bài tập 3 nói kỹ chuyện đó.
</details>

<details>
<summary><b>Bài tập 2 — Gợi ý</b></summary>

Nút lá là nút **không có con nào**. Đệ quy: lá đếm 1, nút trong đếm bằng tổng số lá của hai nhánh.
</details>

<details>
<summary><b>Bài tập 2 — Lời giải</b></summary>

```rust
impl<T: Ord> BinarySearchTree<T> {
    pub fn leaf_count(&self) -> usize {
        fn dem<T>(nut: &Option<Box<NutCay<T>>>) -> usize {
            match nut {
                None => 0,
                Some(n) if n.left.is_none() && n.right.is_none() => 1,   // là LÁ
                Some(n) => dem(&n.left) + dem(&n.right),                 // nút trong
            }
        }
        dem(&self.root)
    }
}

#[test]
fn dem_dung_so_nut_la() {
    let rong: BinarySearchTree<i32> = BinarySearchTree::new();
    assert_eq!(rong.leaf_count(), 0);

    let mut mot = BinarySearchTree::new();
    mot.them(42);
    assert_eq!(mot.leaf_count(), 1, "gốc không con thì chính nó là lá");

    //        50
    //      /    \
    //    30      70
    //   /  \    /  \
    //  20  40  60  80      -> 4 lá
    let mut cay = BinarySearchTree::new();
    for x in [50, 30, 70, 20, 40, 60, 80] { cay.them(x); }
    assert_eq!(cay.leaf_count(), 4);

    // Cây suy biến: mọi nút chỉ có một con -> đúng MỘT lá.
    let mut suy_bien = BinarySearchTree::new();
    for x in 1..=7 { suy_bien.them(x); }
    assert_eq!(suy_bien.leaf_count(), 1);
}
```

Thứ tự nhánh `match` quan trọng: nhánh `Some(n) if ...is_none()` phải đứng **trước** nhánh `Some(n)` tổng quát, nếu không nó không bao giờ được chạm tới. Rust cảnh báo trường hợp nhánh không thể đạt tới, nhưng ở đây hai nhánh cùng khớp `Some`, phân biệt bằng điều kiện — nên thứ tự là do bạn chịu trách nhiệm.
</details>

<details>
<summary><b>Bài tập 3 — Gợi ý</b></summary>

Nạp dãy đã sắp xếp thì mọi giá trị đều lớn hơn nút hiện tại, nên luôn rẽ phải. Cây thành cái gì? Và B-Tree làm gì khác đi?
</details>

<details>
<summary><b>Bài tập 3 — Lời giải</b></summary>

**Nạp `[1,2,3,4,5,6,7]` cho ra một danh sách liên kết đội lốt cây:**

```text
1
 \
  2
   \
    3
     \
      4          chiều cao = 7 (đúng bằng n)
       \        tìm kiếm = O(n), không phải O(log n)
        5        chỉ có 1 lá
         \
          6
           \
            7
```

Mọi giá trị mới đều lớn hơn nút đang đứng nên luôn rẽ phải. Cây **suy biến**. Điều trớ trêu: dữ liệu *đã sắp xếp sẵn* — thứ trông như đầu vào lý tưởng — lại là trường hợp tệ nhất. Và đó là đầu vào rất hay gặp trong thực tế: nạp từ tệp CSV đã sắp, hoặc nạp theo mã tăng dần từ cơ sở dữ liệu.

**B-Tree ngăn chuyện đó bằng cách đảo ngược hướng lớn lên:**

| | BST | B-Tree |
|---|---|---|
| Mỗi nút chứa | 1 khoá | hàng chục tới hàng trăm khoá |
| Cây lớn lên bằng cách | thêm nút ở **đáy** | tách nút, đẩy khoá **lên trên** |
| Chiều cao | không có gì bảo đảm, tệ nhất `n` | luôn `O(log_B n)`, B là bậc |
| Cân bằng | không tự có | **bất biến cấu trúc**, luôn đúng |

Điểm mấu chốt: B-Tree không *sửa* mất cân bằng sau khi xảy ra — nó khiến mất cân bằng **không thể xảy ra**. Nút đầy thì tách làm đôi và đẩy khoá giữa lên cha; cây chỉ cao thêm khi *gốc* tách, nên mọi lá luôn ở **cùng một độ sâu**.

Với B = 100, một cây ba tầng chứa được khoảng một triệu khoá. Đó là lý do cơ sở dữ liệu chọn B-Tree: chiều cao 3 nghĩa là **3 lần đọc đĩa**, và đọc đĩa mới là thứ đắt — xem Chương 33.

Với dữ liệu trong bộ nhớ, lối thoát khác là **cây tự cân bằng** (AVL, đỏ-đen). `BTreeMap` của Rust dùng B-Tree; `HashMap` thì tránh vấn đề này hoàn toàn bằng cách không giữ thứ tự.
</details>
