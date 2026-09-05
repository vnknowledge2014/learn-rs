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
let mut ngan_xep = Vec::new();
ngan_xep.push(10); // Đẩy vào đỉnh Stack: [10]
ngan_xep.push(20); // Đẩy vào đỉnh Stack: [10, 20]
let dinh = ngan_xep.pop(); // Lấy từ đỉnh Stack: Some(20), còn lại [10]
```

### 2. Thảm họa hiệu năng khi dùng `Vec::remove(0)` làm Hàng đợi

Giả sử bạn có một `Vec` chứa 1.000.000 phần tử và muốn lấy phần tử đầu tiên ra:
```rust
// CẢNH BÁO HIỆU NĂNG THẢM HỌA: O(N)
let mut danh_sach = vec![1, 2, 3, 4, 5];
let phan_tu_dau = danh_sach.remove(0); // Buộc CPU phải dời toàn bộ các phần tử phía sau!
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
pub fn kiem_tra_ngoac_hop_le(bieu_thuc: &str) -> bool {
    let mut ngan_xep: Vec<char> = Vec::new();

    for ky_tu in bieu_thuc.chars() {
        match ky_tu {
            '(' | '[' | '{' => {
                ngan_xep.push(ky_tu);
            }
            ')' => {
                if ngan_xep.pop() != Some('(') {
                    return false;
                }
            }
            ']' => {
                if ngan_xep.pop() != Some('[') {
                    return false;
                }
            }
            '}' => {
                if ngan_xep.pop() != Some('{') {
                    return false;
                }
            }
            // Bỏ qua các ký tự chữ cái, số, hoặc khoảng trắng
            _ => {}
        }
    }

    // Biểu thức chỉ đúng khi mọi dấu ngoặc mở đều đã được đóng khớp hết
    ngan_xep.is_empty()
}

/// Mô hình Đơn hàng trong hệ thống thương mại điện tử
#[derive(Debug, PartialEq, Clone)]
pub struct DonHang {
    pub ma_don: u32,
    pub ten_khach: String,
    pub tong_tien: f64,
}

/// ỨNG DỤNG 2 CỦA QUEUE: Hệ thống quản lý hàng đợi đơn hàng chuẩn FIFO
pub struct HangDoiDonHang {
    danh_sach: VecDeque<DonHang>,
}

impl HangDoiDonHang {
    pub fn new() -> Self {
        Self {
            danh_sach: VecDeque::new(),
        }
    }

    /// Khách đặt hàng: Xếp vào cuối hàng đợi - O(1)
    pub fn them_don(&mut self, don: DonHang) {
        self.danh_sach.push_back(don);
    }

    /// Đơn hàng VIP (Ưu tiên khẩn cấp): Chèn thẳng vào đầu hàng đợi - O(1)
    pub fn them_don_vip(&mut self, don: DonHang) {
        self.danh_sach.push_front(don);
    }

    /// Nhà bếp / Kho xuất hàng: Phục vụ đơn đến trước - O(1)
    pub fn xu_ly_don_ke_tiep(&mut self) -> Option<DonHang> {
        self.danh_sach.pop_front()
    }

    /// Xem trước đơn sắp được phục vụ mà không xóa khỏi hàng đợi
    pub fn xem_don_dau(&self) -> Option<&DonHang> {
        self.danh_sach.front()
    }

    pub fn so_don_dang_cho(&self) -> usize {
        self.danh_sach.len()
    }
}

impl Default for HangDoiDonHang {
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
    let bieu_thuc_1 = "{ a + [ b * ( c + d ) ] }";
    let bieu_thuc_2 = "( a + b ]";
    let bieu_thuc_3 = "{ [ ( ] ) }"; // Đóng sai thứ tự lồng nhau

    println!("    - Biểu thức 1 '{}': {}", bieu_thuc_1, kiem_tra_ngoac_hop_le(bieu_thuc_1));
    println!("    - Biểu thức 2 '{}': {}", bieu_thuc_2, kiem_tra_ngoac_hop_le(bieu_thuc_2));
    println!("    - Biểu thức 3 '{}': {}", bieu_thuc_3, kiem_tra_ngoac_hop_le(bieu_thuc_3));

    assert!(kiem_tra_ngoac_hop_le(bieu_thuc_1));
    assert!(!kiem_tra_ngoac_hop_le(bieu_thuc_2));
    assert!(!kiem_tra_ngoac_hop_le(bieu_thuc_3));

    // 2. Kiểm thử Hệ thống Hàng đợi đơn hàng với VecDeque
    println!("\n[2] Vận hành hệ thống xử lý đơn hàng FIFO bằng VecDeque:");
    let mut he_thong = HangDoiDonHang::new();

    // Khách hàng thông thường đặt hàng lần lượt
    he_thong.them_don(DonHang {
        ma_don: 101,
        ten_khach: String::from("Nguyễn Văn A"),
        tong_tien: 150.0,
    });
    he_thong.them_don(DonHang {
        ma_don: 102,
        ten_khach: String::from("Trần Thị B"),
        tong_tien: 80.0,
    });

    println!("    - Đã nhận 2 đơn hàng thông thường. Số đơn chờ: {}", he_thong.so_don_dang_cho());

    // Đơn hàng hỏa tốc VIP xuất hiện! Đưa thẳng vào đầu hàng đợi
    he_thong.them_don_vip(DonHang {
        ma_don: 999,
        ten_khach: String::from("Khách VIP Kim Cương"),
        tong_tien: 500.0,
    });
    println!("    - Nhận đơn hỏa tốc VIP 999 (chen lên đầu hàng)!");

    // Xem trước đơn hàng kế tiếp
    if let Some(don_dau) = he_thong.xem_don_dau() {
        println!("    - Đơn hàng chuẩn bị xử lý tiếp theo là: Mã #{} ({})", don_dau.ma_don, don_dau.ten_khach);
        assert_eq!(don_dau.ma_don, 999);
    }

    // Tiến hành xuất kho lần lượt theo đúng thứ tự ưu tiên
    println!("\n    Bắt đầu xuất kho theo thứ tự FIFO:");
    let mut thu_tu_xu_ly = Vec::new();
    while let Some(don) = he_thong.xu_ly_don_ke_tiep() {
        println!("    -> Đang đóng gói đơn #{}: Khách {} - {:.2}k", don.ma_don, don.ten_khach, don.tong_tien);
        thu_tu_xu_ly.push(don.ma_don);
    }

    // Xác nhận thứ tự xử lý: Đơn VIP 999 trước, sau đó là 101, rồi đến 102
    assert_eq!(thu_tu_xu_ly, vec![999, 101, 102]);
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
| **E0596** | `cannot borrow '...' as mutable, as it is not declared as mutable` | Bạn cố gọi `.push()` trên `Vec` hoặc `.push_back()` trên `VecDeque` nhưng biến tập hợp được khai báo bằng `let` bất biến. | Thêm từ khóa `mut`: `let mut hang_doi = VecDeque::new();`. |
| **E0308** | `mismatched types: expected 'char', found 'Option<char>'` | Bạn gán trực tiếp kết quả trả về của `ngan_xep.pop()` vào một biến kiểu `char` mà quên rằng `pop()` trả về `Option<T>` (vì ngăn xếp có thể rỗng). | Sử dụng `match`, `if let Some(x)`, hoặc so sánh với `Some(...)`. |
| **E0502** | `cannot borrow '...' as mutable because it is also borrowed as immutable` | Bạn đang giữ tham chiếu mượn bất biến xem phần tử đầu `front()` nhưng lại gọi hàm ghi chèn `push_back()` trong cùng phạm vi. | Kết thúc phạm vi tham chiếu đọc trước khi thực hiện thao tác thay đổi hàng đợi. |
| **E0432** | `unresolved import 'std::collections::Queue'` | Trong thư viện chuẩn của Rust không có kiểu tên là `Queue`. Rust dùng `VecDeque` làm cấu trúc hàng đợi chuẩn. | Sửa dòng khai báo thư viện thành: `use std::collections::VecDeque;`. |

### Ví dụ phân tích lỗi `E0308` khi xử lý giá trị trả về từ `pop()`:

```rust
// Đoạn mã lỗi minh họa: Quên xử lý trường hợp ngăn xếp bị rỗng
fn lay_dinh_loi(mut stack: Vec<i32>) {
    // let gia_tri: i32 = stack.pop(); // LỖI E0308: pop() trả về Option<i32>, không phải i32!
}

// Cách sửa chữa đúng chuẩn: Xử lý an toàn với Option
fn lay_dinh_dung(mut stack: Vec<i32>) {
    match stack.pop() {
        Some(gia_tri) => println!("Đã lấy được giá trị: {}", gia_tri),
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
mod kiem_thu {
    use super::*;

    fn don(ma: u32, ten: &str) -> DonHang {
        DonHang { ma_don: ma, ten_khach: ten.into(), tong_tien: 100.0 }
    }

    #[test]
    fn kiem_tra_ngoac() {
        assert!(kiem_tra_ngoac_hop_le("(a[b]{c})"));
        assert!(kiem_tra_ngoac_hop_le(""));
        assert!(!kiem_tra_ngoac_hop_le("(a]"));
        assert!(!kiem_tra_ngoac_hop_le("((("));
        assert!(!kiem_tra_ngoac_hop_le(")("));
    }

    #[test]
    fn hang_doi_fifo_va_uu_tien_vip() {
        let mut hd = HangDoiDonHang::new();
        hd.them_don(don(1, "A"));
        hd.them_don(don(2, "B"));
        hd.them_don_vip(don(9, "VIP")); // chen lên đầu
        assert_eq!(hd.so_don_dang_cho(), 3);
        assert_eq!(hd.xem_don_dau().map(|d| d.ma_don), Some(9));

        // VIP ra trước, phần còn lại giữ đúng thứ tự FIFO
        assert_eq!(hd.xu_ly_don_ke_tiep().map(|d| d.ma_don), Some(9));
        assert_eq!(hd.xu_ly_don_ke_tiep().map(|d| d.ma_don), Some(1));
        assert_eq!(hd.xu_ly_don_ke_tiep().map(|d| d.ma_don), Some(2));
        assert_eq!(hd.xu_ly_don_ke_tiep().map(|d| d.ma_don), None);
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
   Áp dụng nguyên lý Ngăn xếp (LIFO), hãy viết một hàm `fn doi_thap_phan_sang_nhi_phan(mut so: u32) -> String`:
   - Liên tục chia `so` cho 2, lấy phần dư đẩy vào một Stack.
   - Khi `so == 0`, lần lượt rút (`pop`) các phần dư ra khỏi Stack và ghép thành chuỗi kết quả.
   *(Giải thích: Tại sao cơ chế LIFO của Stack lại đảo ngược chính xác các số dư thành chuỗi nhị phân chuẩn?)*
2. **Bài tập 2 (Mô phỏng bộ đệm bàn phím)**:  
   Sử dụng `VecDeque<char>` để viết cấu trúc `BoDemPhim` có sức chứa tối đa 10 ký tự. Khi người dùng gõ ký tự thứ 11, ký tự cũ nhất ở đầu hàng đợi sẽ tự động bị loại bỏ (`pop_front`) để nhường chỗ cho ký tự mới ở cuối hàng đợi (`push_back`).
3. **Bài tập 3 (Tư duy thiết kế: Hàng đợi bằng 2 Ngăn xếp)**:  
   Làm thế nào để bạn có thể giả lập một Hàng đợi (Queue - FIFO) chỉ bằng cách sử dụng **hai Ngăn xếp (Stack 1 và Stack 2)**? Hãy mô tả quy trình nạp dữ liệu vào Stack 1 và đổ ngược dữ liệu sang Stack 2 khi cần lấy ra.
