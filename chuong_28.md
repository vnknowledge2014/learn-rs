# Chương 28: Ngăn xếp, Hàng đợi & Hàng đợi hai đầu: Triển khai an toàn và Ứng dụng thực tế (Stacks, Queues & VecDeque)

## Giới thiệu & Mục tiêu học tập

Sau khi đã nắm vững các vùng nhớ liền kề (Array, Vector) và danh sách liên kết, chúng ta bước sang hai cấu trúc dữ liệu kinh điển và phổ biến bậc nhất trong khoa học máy tính: **Ngăn xếp (Stack)** và **Hàng đợi (Queue)**. 

Cả Ngăn xếp và Hàng đợi đều là các cấu trúc dữ liệu dạng tuyến tính, nhưng chúng có các quy tắc nghiêm ngặt về thứ tự thêm vào và lấy ra của các phần tử:
- **Ngăn xếp (Stack)** tuân theo nguyên lý **LIFO (Last-In, First-Out - Vào sau ra trước)**: Phần tử nào được thêm vào cuối cùng sẽ là phần tử đầu tiên được lấy ra.
- **Hàng đợi (Queue)** tuân theo nguyên lý **FIFO (First-In, First-Out - Vào trước ra trước)**: Phần tử nào đến trước sẽ được phục vụ và rời đi trước.

Trong Rust, người mới bắt đầu thường mắc một sai lầm chết người về mặt hiệu năng: Dùng `Vec` để làm hàng đợi bằng cách gọi `vec.remove(0)`. Thao tác này buộc CPU phải dời toàn bộ hàng triệu phần tử còn lại về phía trước, biến một thao tác lẽ ra phải tốn $O(1)$ thành thảm họa $O(N)$. Để giải quyết triệt để vấn đề này, thư viện chuẩn của Rust cung cấp một vũ khí siêu hạng: **`VecDeque<T>` (Double-Ended Queue)** dựa trên kiến trúc vòng đệm tròn (Circular Ring Buffer).

Mục tiêu học tập của chương này:
- Nắm vững nguyên lý hoạt động của LIFO (Ngăn xếp) và FIFO (Hàng đợi) thông qua các hình ảnh đời sống trực quan.
- Nhận biết các bài toán thực tế bắt buộc phải dùng Ngăn xếp (như tính năng Undo/Redo trong trình soạn thảo, kiểm tra dấu ngoặc hợp lệ, hoặc ngăn xếp cuộc gọi Call Stack).
- Hiểu rõ vì sao `Vec::remove(0)` làm hàng đợi lại gây suy giảm hiệu năng nghiêm trọng và cách `VecDeque<T>` tối ưu hóa bằng con trỏ vòng đệm tròn (Ring Buffer) đạt $O(1)$.
- Tự tay xây dựng và kiểm thử các cấu trúc Stack và Queue an toàn bằng Rust với quyền sở hữu (ownership) và vay mượn (borrow) chặt chẽ.
- Làm quen với Hàng đợi ưu tiên (Priority Queue - `BinaryHeap`) trong Rust.

---

## Hình tượng hóa đời sống (Intuitive Everyday Analogy)

Hãy quan sát hai khung cảnh vô cùng quen thuộc trong một quán ăn đông đúc vào giờ cao điểm:

```
┌──────────────────────────────────────────────────────────────────────────────────┐
│             HÌNH TƯỢNG HÓA NGĂN XẾP (STACK) VS HÀNG ĐỢI (QUEUE)                  │
├──────────────────────────────────────────────────────────────────────────────────┤
│                                                                                  │
│ [NGĂN XẾP - STACK (LIFO: VÀO SAU RA TRƯỚC)]                                      │
│                                                                                  │
│      Lấy ra / Thêm vào (Chỉ ở 1 đầu duy nhất)                                    │
│             ▲                                                                    │
│             │                                                                    │
│        ┌────┴────┐                                                               │
│        │ Đĩa #3  │ ◄── Vừa mới rửa xong, úp lên trên cùng (Vào sau cùng)         │
│        ├─────────┤                                                               │
│        │ Đĩa #2  │                                                               │
│        ├─────────┤                                                               │
│        │ Đĩa #1  │ ◄── Nằm ở đáy chồng đĩa từ sáng (Lấy ra sau cùng)             │
│        └─────────┘                                                               │
│                                                                                  │
│ [HÀNG ĐỢI - QUEUE (FIFO: VÀO TRƯỚC RA TRƯỚC)]                                    │
│                                                                                  │
│  Ra về (Phục vụ)                           Xếp hàng vào (Chờ đợi)                │
│       ▲                                             ▲                            │
│       │                                             │                            │
│   ┌───┴───┐         ┌───────┐         ┌───────┐ ┌───┴───┐                        │
│   │Khách 1│ ◄────── │Khách 2│ ◄────── │Khách 3│ │Khách 4│                        │
│   └───────┘         └───────┘         └───────┘ └───────┘                        │
│  (Đến sớm nhất)                                (Vừa mới tới)                     │
│                                                                                  │
│ [VECDEQUE - BĂNG CHUYỀN SUSHI XOAY VÒNG (RING BUFFER)]                           │
│   - Đầu bếp có thể đặt đĩa mới vào cả đầu trái hoặc đầu phải.                    │
│   - Thực khách có thể nhấc đĩa ra ở cả hai đầu mà không cần ai phải xê dịch ghế! │
└──────────────────────────────────────────────────────────────────────────────────┘
```

### 1. Ngăn xếp (Stack) — Chồng đĩa ăn trong nhà hàng tiệc cưới
- Sau khi rửa sạch một chiếc đĩa, người phục vụ úp chiếc đĩa đó lên **trên cùng của chồng đĩa** (thao tác `push`).
- Khi người dọn bàn cần lấy đĩa ra tiếp khách, họ sẽ nhấc ngay chiếc đĩa nằm ở **đỉnh trên cùng** (thao tác `pop`).
- Chiếc đĩa nào được đặt vào sau cùng (`Last-In`) sẽ là chiếc đĩa đầu tiên được mang đi sử dụng (`First-Out`). Bạn không thể nào rút chiếc đĩa ở đáy chồng đĩa ra trước vì sẽ làm đổ vỡ toàn bộ chồng đĩa!

### 2. Hàng đợi (Queue) — Hàng người xếp hàng mua bánh mì buổi sáng
- Khách hàng tới mua bánh mì phải xếp hàng:
  - Người đến đầu tiên đứng ở đầu hàng (`Front`), được bán bánh mì và ra về đầu tiên.
  - Người đến sau phải đứng vào cuối hàng (`Back/Rear`) và kiên nhẫn chờ đến lượt mình.
- Ai vào trước thì ra trước (`First-In, First-Out`). Đây là quy tắc văn minh và công bằng nhất trong đời sống cũng như trong việc xếp hàng xử lý tin nhắn mạng hoặc lệnh in tài liệu.

### 3. Hàng đợi hai đầu (VecDeque) — Băng chuyền sushi xoay vòng
- Trong nhà hàng Nhật Bản, băng chuyền sushi chạy theo một vòng tròn khép kín.
- Người đầu bếp có thể đặt đĩa sushi vào bất kỳ khoảng trống nào phía trước hoặc phía sau.
- Khách hàng có thể lấy đĩa ra ở cả hai đầu mà không làm gián đoạn chuyển động của băng chuyền. Nhờ cơ chế vòng tròn này, băng chuyền không bao giờ bị "kẹt đuôi", và việc thêm/bớt ở cả hai đầu diễn ra êm ru trong thời gian $O(1)$.

---

## Khái niệm & Cơ chế kỹ thuật chuyên sâu (In-Depth Technical Mechanics)

### 1. Triển khai Ngăn xếp bằng `Vec<T>` đạt $O(1)$

Trong Rust, cấu trúc `Vec<T>` bản thân nó đã là một Ngăn xếp hoàn hảo:
- Phương thức `vec.push(x)`: Thêm phần tử vào cuối vector với thời gian khấu hao amortized $O(1)$.
- Phương thức `vec.pop()`: Rút phần tử cuối cùng ra và trả về `Option<T>` với thời gian $O(1)$ tuyệt đối, vì không có bất kỳ phần tử nào khác bị xê dịch vị trí ô nhớ!

```rust
let mut stack = Vec::new();
stack.push(10); // Đẩy vào đỉnh Stack: [10]
stack.push(20); // Đẩy vào đỉnh Stack: [10, 20]
let peak = stack.pop(); // Lấy từ đỉnh Stack: Some(20), còn lại [10]
```

### 2. Thảm họa hiệu năng khi dùng `Vec::remove(0)` làm Hàng đợi

Giả sử bạn có một `Vec` chứa 1.000.000 phần tử và muốn lấy phần tử đầu tiên ra:
```rust
// CẢNH BÁO HIỆU NĂNG THẢM HỌA: O(N)
let mut list = vec![1, 2, 3, 4, 5];
let front_item = list.remove(0); // Buộc CPU phải dời toàn bộ các phần tử phía sau!
```
Điều gì diễn ra bên dưới thanh RAM?
1. Rust lấy phần tử tại ô nhớ chỉ số 0.
2. Để giữ cho mảng luôn liền kề không bị thủng lỗ, CPU buộc phải thực hiện lệnh sao chép dịch chuyển: phần tử 1 dời về 0, phần tử 2 dời về 1, ..., phần tử 999.999 dời về 999.998.
3. Tổng cộng **999.999 lượt ghi nhớ** bị kích hoạt! Nếu bạn làm điều này 1.000 lần, chương trình của bạn sẽ bị đơ cứng hoàn toàn.

### 3. Bí mật bên trong của `VecDeque<T>` (Vòng đệm tròn - Circular Buffer)

`VecDeque<T>` giải quyết triệt để bài toán trên bằng cách biến một mảng phẳng thành một **vòng tròn khép kín** sử dụng hai con trỏ chỉ số: `head` (đầu) và `tail` (đuôi):

```
       Chỉ số:   0      1      2      3      4      5      6      7
               ┌──────┬──────┬──────┬──────┬──────┬──────┬──────┬──────┐
Mảng vật lý:   │  D   │  E   │Trống │Trống │Trống │  A   │  B   │  C   │
               └──────┴──────┴──────┴──────┴──────┴──────┴──────┴──────┘
                              ▲                    ▲
                              │                    │
                             tail                 head
```
- Khi bạn gọi `pop_front()`: `VecDeque` không hề dịch chuyển mảng! Nó chỉ đơn giản tăng con trỏ `head` lên 1 nấc: `head = (head + 1) % capacity`. Thời gian tốn đúng $O(1)$!
- Khi bạn gọi `push_front()`: Con trỏ `head` lùi về 1 nấc theo vòng tròn: `head = (head - 1 + capacity) % capacity`.
- Dữ liệu không bao giờ bị dời chỗ. Toàn bộ chi phí chỉ là một vài phép tính số học trên chỉ số!

---

## Mã nguồn minh họa thực chiến (Idiomatic Runnable Rust Blueprint)

Dưới đây là một chương trình hoàn chỉnh, minh họa cả hai cấu trúc:
1. Sử dụng Ngăn xếp (`Vec`) để giải quyết bài toán kinh điển: **Kiểm tra tính hợp lệ của các dấu ngoặc đóng mở trong chuỗi biểu thức**.
2. Xây dựng một **Hệ thống hàng đợi in ấn / xử lý đơn hàng** theo thời gian thực bằng `VecDeque<T>` đạt chuẩn $O(1)$:

```rust
use std::collections::VecDeque;

/// ỨNG DỤNG 1 CỦA STACK: Kiểm tra dấu ngoặc hợp lệ
/// Thuật toán sử dụng Ngăn xếp (LIFO):
/// - Gặp dấu mở '(', '[', '{': Đẩy vào đỉnh ngăn xếp.
/// - Gặp dấu đóng ')', ']', '}': Rút phần tử trên đỉnh ra so khớp.
///   Nếu không khớp hoặc ngăn xếp rỗng -> Biểu thức sai cú pháp!
/// - Kết thúc chuỗi, nếu ngăn xếp rỗng -> Biểu thức hợp lệ.
pub fn is_balanced_brackets(bieu_thuc: &str) -> bool {
    let mut stack: Vec<char> = Vec::new();

    for ky_tu in bieu_thuc.chars() {
        match ky_tu {
            '(' | '[' | '{' => {
                stack.push(ky_tu);
            }
            ')' => {
                if stack.pop() != Some('(') {
                    return false;
                }
            }
            ']' => {
                if stack.pop() != Some('[') {
                    return false;
                }
            }
            '}' => {
                if stack.pop() != Some('{') {
                    return false;
                }
            }
            // Bỏ qua các ký tự chữ cái, số, hoặc khoảng trắng
            _ => {}
        }
    }

    // Biểu thức chỉ đúng khi mọi dấu ngoặc mở đều đã được đóng khớp hết
    stack.is_empty()
}

/// Mô hình Đơn hàng trong hệ thống thương mại điện tử
#[derive(Debug, PartialEq, Clone)]
pub struct DonQueue {
    pub order_code: u32,
    pub customer_name: String,
    pub tong_tien: f64,
}

/// ỨNG DỤNG 2 CỦA QUEUE: Hệ thống quản lý hàng đợi đơn hàng chuẩn FIFO
pub struct QueueDonQueue {
    list: VecDeque<DonQueue>,
}

impl QueueDonQueue {
    pub fn new() -> Self {
        Self {
            list: VecDeque::new(),
        }
    }

    /// Khách đặt hàng: Xếp vào cuối hàng đợi - O(1)
    pub fn them_don(&mut self, don: DonQueue) {
        self.list.push_back(don);
    }

    /// Đơn hàng VIP (Ưu tiên khẩn cấp): Chèn thẳng vào đầu hàng đợi - O(1)
    pub fn them_don_vip(&mut self, don: DonQueue) {
        self.list.push_front(don);
    }

    /// Nhà bếp / Kho xuất hàng: Phục vụ đơn đến trước - O(1)
    pub fn handle_don_ke_cont(&mut self) -> Option<DonQueue> {
        self.list.pop_front()
    }

    /// Xem trước đơn sắp được phục vụ mà không xóa khỏi hàng đợi
    pub fn first_view_don(&self) -> Option<&DonQueue> {
        self.list.front()
    }

    pub fn so_don_dang_cho(&self) -> usize {
        self.list.len()
    }
}

impl Default for QueueDonQueue {
    fn default() -> Self {
        Self::new()
    }
}

fn main() {
    println!("============================================================");
    println!("   ỨNG DỤNG THỰC CHIẾN CỦA NGĂN XẾP (STACK) & HÀNG ĐỢI (QUEUE)");
    println!("============================================================");

    // 1. Kiểm thử thuật toán kiểm tra dấu ngoặc với Stack
    println!("[1] Kiểm tra tính hợp lệ của biểu thức toán học:");
    let expr_1 = "{ a + [ b * ( c + d ) ] }";
    let expr_2 = "( a + b ]";
    let expr_3 = "{ [ ( ] ) }"; // Đóng sai thứ tự lồng nhau

    println!("    - Biểu thức 1 '{}': {}", expr_1, is_balanced_brackets(expr_1));
    println!("    - Biểu thức 2 '{}': {}", expr_2, is_balanced_brackets(expr_2));
    println!("    - Biểu thức 3 '{}': {}", expr_3, is_balanced_brackets(expr_3));

    assert!(is_balanced_brackets(expr_1));
    assert!(!is_balanced_brackets(expr_2));
    assert!(!is_balanced_brackets(expr_3));

    // 2. Kiểm thử Hệ thống Hàng đợi đơn hàng với VecDeque
    println!("\n[2] Vận hành hệ thống xử lý đơn hàng FIFO bằng VecDeque:");
    let mut he_thong = QueueDonQueue::new();

    // Khách hàng thông thường đặt hàng lần lượt
    he_thong.them_don(DonQueue {
        order_code: 101,
        customer_name: String::from("Nguyễn Văn A"),
        tong_tien: 150.0,
    });
    he_thong.them_don(DonQueue {
        order_code: 102,
        customer_name: String::from("Trần Thị B"),
        tong_tien: 80.0,
    });

    println!("    - Đã nhận 2 đơn hàng thông thường. Số đơn chờ: {}", he_thong.so_don_dang_cho());

    // Đơn hàng hỏa tốc VIP xuất hiện! Đưa thẳng vào đầu hàng đợi
    he_thong.them_don_vip(DonQueue {
        order_code: 999,
        customer_name: String::from("Khách VIP Kim Cương"),
        tong_tien: 500.0,
    });
    println!("    - Nhận đơn hỏa tốc VIP 999 (chen lên đầu hàng)!");

    // Xem trước đơn hàng kế tiếp
    if let Some(don_dau) = he_thong.first_view_don() {
        println!("    - Đơn hàng chuẩn bị xử lý tiếp theo là: Mã #{} ({})", don_dau.order_code, don_dau.customer_name);
        assert_eq!(don_dau.order_code, 999);
    }

    // Tiến hành xuất kho lần lượt theo đúng thứ tự ưu tiên
    println!("\n    Bắt đầu xuất kho theo thứ tự FIFO:");
    let mut handles = Vec::new();
    while let Some(don) = he_thong.handle_don_ke_cont() {
        println!("    -> Đang đóng gói đơn #{}: Khách {} - {:.2}k", don.order_code, don.customer_name, don.tong_tien);
        handles.push(don.order_code);
    }

    // Xác nhận thứ tự xử lý: Đơn VIP 999 trước, sau đó là 101, rồi đến 102
    assert_eq!(handles, vec![999, 101, 102]);
    assert_eq!(he_thong.so_don_dang_cho(), 0);
    println!("    => Toàn bộ hàng đợi đã được xử lý sạch sẽ!");

    println!("============================================================");
    println!("               HOÀN TẤT THỰC NGHIỆM CHƯƠNG 24               ");
    println!("============================================================");
}
```

---

## Bảng tra cứu lỗi biên dịch & Cách khắc phục (Compiler Error Guide)

Dưới đây là các lỗi biên dịch thường gặp nhất khi thao tác với Stack, Queue và `VecDeque`:

| Mã lỗi | Thông báo mẫu từ trình biên dịch | Nguyên nhân cốt lõi | Cách khắc phục nhanh |
|---|---|---|---|
| **E0596** | `cannot borrow '...' as mutable, as it is not declared as mutable` | Bạn cố gọi `.push()` trên `Vec` hoặc `.push_back()` trên `VecDeque` nhưng biến tập hợp được khai báo bằng `let` bất biến. | Thêm từ khóa `mut`: `let mut queue = VecDeque::new();`. |
| **E0308** | `mismatched types: expected 'char', found 'Option<char>'` | Bạn gán trực tiếp kết quả trả về của `stack.pop()` vào một biến kiểu `char` mà quên rằng `pop()` trả về `Option<T>` (vì ngăn xếp có thể rỗng). | Sử dụng `match`, `if let Some(x)`, hoặc so sánh với `Some(...)`. |
| **E0502** | `cannot borrow '...' as mutable because it is also borrowed as immutable` | Bạn đang giữ tham chiếu mượn bất biến xem phần tử đầu `front()` nhưng lại gọi hàm ghi chèn `push_back()` trong cùng phạm vi. | Kết thúc phạm vi tham chiếu đọc trước khi thực hiện thao tác thay đổi hàng đợi. |
| **E0432** | `unresolved import 'std::collections::Queue'` | Trong thư viện chuẩn của Rust không có kiểu tên là `Queue`. Rust dùng `VecDeque` làm cấu trúc hàng đợi chuẩn. | Sửa dòng khai báo thư viện thành: `use std::collections::VecDeque;`. |

### Ví dụ phân tích lỗi `E0308` khi xử lý giá trị trả về từ `pop()`:

```rust
// Đoạn mã lỗi minh họa: Quên xử lý trường hợp ngăn xếp bị rỗng
fn peek_broken(mut stack: Vec<i32>) {
    // let value: i32 = stack.pop(); // LỖI E0308: pop() trả về Option<i32>, không phải i32!
}

// Cách sửa chữa đúng chuẩn: Xử lý an toàn với Option
fn peek_correct(mut stack: Vec<i32>) {
    match stack.pop() {
        Some(value) => println!("Đã lấy được giá trị: {}", value),
        None => println!("Ngăn xếp đang rỗng, không có gì để lấy!"),
    }
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

    fn don(id: u32, name: &str) -> DonQueue {
        DonQueue { order_code: id, customer_name: name.into(), tong_tien: 100.0 }
    }

    #[test]
    fn bracket_matching() {
        assert!(is_balanced_brackets("(a[b]{c})"));
        assert!(is_balanced_brackets(""));
        assert!(!is_balanced_brackets("(a]"));
        assert!(!is_balanced_brackets("((("));
        assert!(!is_balanced_brackets(")("));
    }

    #[test]
    fn fifo_queue_and_vip_priority() {
        let mut hd = QueueDonQueue::new();
        hd.them_don(don(1, "A"));
        hd.them_don(don(2, "B"));
        hd.them_don_vip(don(9, "VIP")); // chen lên đầu
        assert_eq!(hd.so_don_dang_cho(), 3);
        assert_eq!(hd.first_view_don().map(|d| d.order_code), Some(9));

        // VIP ra trước, phần còn lại giữ đúng thứ tự FIFO
        assert_eq!(hd.handle_don_ke_cont().map(|d| d.order_code), Some(9));
        assert_eq!(hd.handle_don_ke_cont().map(|d| d.order_code), Some(1));
        assert_eq!(hd.handle_don_ke_cont().map(|d| d.order_code), Some(2));
        assert_eq!(hd.handle_don_ke_cont().map(|d| d.order_code), None);
    }
}
```

## Tóm tắt chương & Bài tập rèn luyện (Summary & Exercises)

### 4 Điểm cốt lõi cần ghi nhớ:
1. **LIFO vs FIFO**: Ngăn xếp (Stack) lấy phần tử mới nhất ra trước (LIFO); Hàng đợi (Queue) lấy phần tử cũ nhất ra trước (FIFO).
2. **Dùng `Vec` cho Stack**: `Vec::push` và `Vec::pop` thao tác ở đuôi mảng với hiệu năng tuyệt hảo $O(1)$.
3. **Tuyệt đối tránh `Vec::remove(0)`**: Việc dời toàn bộ mảng gây thảm họa $O(N)$. Luôn sử dụng `VecDeque<T>` khi cần cấu trúc hàng đợi.
4. **Cơ chế Vòng đệm tròn**: `VecDeque` sử dụng con trỏ vòng đệm tròn để thêm và xóa ở cả hai đầu (`front` và `back`) trong thời gian hằng số $O(1)$ mà không cần di dời dữ liệu trong bộ nhớ đệm (buffer).

### Bài tập rèn luyện tự giải:
1. **Bài tập 1 (Bộ chuyển đổi cơ số 10 sang nhị phân)**:  
   Áp dụng nguyên lý Ngăn xếp (LIFO), hãy viết một hàm `fn to_binary(mut n: u32) -> String`:
   - Liên tục chia `so` cho 2, lấy phần dư đẩy vào một Stack.
   - Khi `so == 0`, lần lượt rút (`pop`) các phần dư ra khỏi Stack và ghép thành chuỗi kết quả.
   *(Giải thích: Tại sao cơ chế LIFO của Stack lại đảo ngược chính xác các số dư thành chuỗi nhị phân chuẩn?)*
2. **Bài tập 2 (Mô phỏng bộ đệm bàn phím)**:  
   Sử dụng `VecDeque<char>` để viết cấu trúc `KeyBuffer` có sức chứa tối đa 10 ký tự. Khi người dùng gõ ký tự thứ 11, ký tự cũ nhất ở đầu hàng đợi sẽ tự động bị loại bỏ (`pop_front`) để nhường chỗ cho ký tự mới ở cuối hàng đợi (`push_back`).
3. **Bài tập 3 (Tư duy thiết kế: Hàng đợi bằng 2 Ngăn xếp)**:  
   Làm thế nào để bạn có thể giả lập một Hàng đợi (Queue - FIFO) chỉ bằng cách sử dụng **hai Ngăn xếp (Stack 1 và Stack 2)**? Hãy mô tả quy trình nạp dữ liệu vào Stack 1 và đổ ngược dữ liệu sang Stack 2 khi cần lấy ra.

---

### Gợi ý & Lời giải

<details>
<summary><b>Bài tập 1 — Gợi ý</b></summary>

Chia liên tục cho 2, đẩy phần dư vào ngăn xếp. Phần dư ra **ngược** thứ tự cần in, và LIFO chính là thứ đảo nó lại giúp bạn.
</details>

<details>
<summary><b>Bài tập 1 — Lời giải</b></summary>

```rust
/// Đổi số thập phân sang chuỗi nhị phân bằng NGĂN XẾP.
pub fn to_binary(mut n: u32) -> String {
    if n == 0 { return "0".to_string(); }   // trường hợp biên, dễ quên nhất

    let mut stack = Vec::new();
    while n > 0 {
        stack.push(n % 2);   // phần dư ra theo thứ tự NGƯỢC với kết quả cần in
        n /= 2;
    }
    // Rút ra theo LIFO -> tự động đảo lại đúng thứ tự.
    let mut ra = String::with_capacity(stack.len());
    while let Some(bit) = stack.pop() {
        ra.push(if bit == 1 { '1' } else { '0' });
    }
    ra
}

#[test]
fn doi_nhi_phan_dung() {
    assert_eq!(to_binary(0), "0");
    assert_eq!(to_binary(1), "1");
    assert_eq!(to_binary(10), "1010");
    assert_eq!(to_binary(255), "11111111");
    // Đối chiếu với bộ định dạng của Rust — nguồn sự thật độc lập.
    for n in [0u32, 1, 7, 64, 1000, u32::MAX] {
        assert_eq!(to_binary(n), format!("{n:b}"));
    }
}
```

Bài này chọn ngăn xếp không phải vì nhanh hơn mà vì nó **diễn đạt đúng bài toán**: "tôi sinh ra kết quả theo thứ tự ngược, cần đảo lại". Bạn hoàn toàn có thể `insert(0, ...)` vào `String`, nhưng mỗi lần chèn đầu là O(N) — tổng thành O(N²) một cách vô ích.
</details>

<details>
<summary><b>Bài tập 2 — Gợi ý</b></summary>

`VecDeque` cho phép thêm ở cuối và bỏ ở đầu đều O(1). Kiểm sức chứa **trước** khi thêm, nếu đầy thì `pop_front()` một ký tự.
</details>

<details>
<summary><b>Bài tập 2 — Lời giải</b></summary>

```rust
use std::collections::VecDeque;

/// Bộ đệm bàn phím sức chứa cố định: đầy thì ký tự CŨ NHẤT bị đẩy ra.
pub struct KeyBuffer {
    buf: VecDeque<char>,
    capacity: usize,
}

impl KeyBuffer {
    pub fn new(capacity: usize) -> Self {
        KeyBuffer { buf: VecDeque::with_capacity(capacity), capacity }
    }

    /// Trả về ký tự bị đẩy ra (nếu có) — đừng nuốt mất thông tin đó.
    pub fn go(&mut self, c: char) -> Option<char> {
        let bi_bo = if self.buf.len() == self.capacity {
            self.buf.pop_front()
        } else { None };
        self.buf.push_back(c);
        bi_bo
    }

    pub fn noi_dung(&self) -> String { self.buf.iter().collect() }
    pub fn len(&self) -> usize { self.buf.len() }
    pub fn is_empty(&self) -> bool { self.buf.is_empty() }
}

#[test]
fn day_thi_day_ky_tu_cu_nhat_ra() {
    let mut kb = KeyBuffer::new(10);
    for c in "abcdefghij".chars() {
        assert_eq!(kb.go(c), None);          // chưa đầy -> không đẩy ai ra
    }
    assert_eq!(kb.noi_dung(), "abcdefghij");

    assert_eq!(kb.go('k'), Some('a'));       // ký tự thứ 11 -> 'a' bị đẩy ra
    assert_eq!(kb.noi_dung(), "bcdefghijk");
    assert_eq!(kb.len(), 10);                // sức chứa KHÔNG BAO GIỜ vượt
}
```

Chi tiết đáng học: `go` **trả về** ký tự bị loại thay vì lặng lẽ vứt đi. Trong một trình soạn thảo thật, đó là thứ bạn cần để ghi nhật ký hoặc hoàn tác. Hàm nuốt mất thông tin là hàm khó dùng lại.
</details>

<details>
<summary><b>Bài tập 3 — Gợi ý</b></summary>

Mấu chốt: chỉ đổ từ ngăn xếp Vào sang ngăn xếp Ra **khi Ra rỗng**. Đổ đúng lúc đó thì mỗi phần tử chỉ bị chuyển đúng một lần trong cả đời.
</details>

<details>
<summary><b>Bài tập 3 — Lời giải</b></summary>

```rust
/// Hàng đợi FIFO dựng từ hai ngăn xếp LIFO.
pub struct QueueTuHaiStack<T> {
    vao: Vec<T>,   // nơi nhận phần tử mới
    ra:  Vec<T>,   // nơi lấy phần tử ra, thứ tự đã ĐẢO sẵn
}

impl<T> QueueTuHaiStack<T> {
    pub fn new() -> Self { QueueTuHaiStack { vao: Vec::new(), ra: Vec::new() } }

    pub fn push(&mut self, x: T) { self.vao.push(x); }

    pub fn pop(&mut self) -> Option<T> {
        if self.ra.is_empty() {
            // CHỈ đổ khi `ra` đã cạn. Đổ sớm hơn là làm hỏng thứ tự.
            while let Some(x) = self.vao.pop() { self.ra.push(x); }
        }
        self.ra.pop()
    }

    pub fn len(&self) -> usize { self.vao.len() + self.ra.len() }
    pub fn is_empty(&self) -> bool { self.len() == 0 }
}

#[test]
fn hai_stack_cho_dung_thu_tu_fifo() {
    let mut q = QueueTuHaiStack::new();
    q.push(1); q.push(2); q.push(3);
    assert_eq!(q.pop(), Some(1));   // vào trước ra trước
    q.push(4);                      // thêm giữa chừng
    assert_eq!(q.pop(), Some(2));
    assert_eq!(q.pop(), Some(3));
    assert_eq!(q.pop(), Some(4));
    assert_eq!(q.pop(), None);
}
```

**Vì sao đây là O(1) khấu hao dù `pop` đôi khi tốn O(N):** mỗi phần tử được chuyển từ `vao` sang `ra` **đúng một lần** trong cả vòng đời của nó. Chia tổng chi phí cho tổng số thao tác ra một hằng số. Đây chính là *thời gian khấu hao* (amortized time) mà Chương 25 nói tới — cùng loại lập luận với việc `Vec` nhân đôi dung lượng.

Cái bẫy: nếu đổ mỗi lần `pop` (không kiểm `ra.is_empty()`), lập luận khấu hao sụp đổ và bạn có O(N) thật cho mỗi thao tác.
</details>
