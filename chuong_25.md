# Chương 25: Độ phức tạp tính toán & Trực quan hóa Big-O (Computational Complexity & Big-O Visualized)

## Giới thiệu & Mục tiêu học tập

Chào mừng bạn bước vào **Chủ đề 5: Cấu trúc dữ liệu & Giải thuật trong Rust (Data Structures & Algorithms - DSA)**! Ở các chủ đề trước, bạn đã làm chủ cú pháp ngôn ngữ, cơ chế an toàn bộ nhớ độc nhất vô nhị của Rust với quyền sở hữu (ownership), vay mượn (borrow), thời gian sống (lifetime), cũng như sức mạnh của lập trình hàm và siêu lập trình (macro). 

Tuy nhiên, viết được một đoạn mã chạy đúng mới chỉ là bước khởi đầu. Trong thực tế phát triển phần mềm hệ thống, câu hỏi sống còn đặt ra là: **"Đoạn mã của bạn sẽ chạy nhanh hay chậm khi lượng dữ liệu phình to gấp 10 lần, 1.000 lần, hay 1.000.000 lần?"** Một thuật toán chạy mượt mà trên máy tính cá nhân với 10 dòng dữ liệu mẫu có thể khiến toàn bộ máy chủ sập nguồn hoặc bị treo đơ vĩnh viễn khi ứng dụng đón nhận 1 triệu người dùng thực tế.

Để đo lường, so sánh và dự đoán hiệu năng của các giải thuật mà không cần phải chạy thử trên từng cỗ máy cụ thể, các nhà khoa học máy tính sử dụng một công cụ tư duy trực quan mang tên: **Ký hiệu Big-O (Big-O Notation)**. 

Mục tiêu học tập của chương này:
- Nắm vững bản chất của **Độ phức tạp tính toán (Computational Complexity)** bao gồm **Độ phức tạp thời gian (Time Complexity)** và **Độ phức tạp không gian (Space Complexity)** mà không cần bất kỳ công thức toán giải tích hay vi phân nào.
- Hiểu rõ ký hiệu Big-O như một thước đo "độ tốn công" khi quy mô công việc $N$ bùng nổ.
- Nhận diện trực quan các cấp bậc Big-O phổ biến: $O(1)$, $O(\log N)$, $O(N)$, $O(N \log N)$, $O(N^2)$, và $O(2^N)$.
- Biết cách sử dụng `std::time::Instant` trong Rust để thực nghiệm đo đạc thời gian thực thi của mã nguồn.
- Rèn luyện phản xạ phát hiện các "nút thắt cổ chai" (bottlenecks) làm tiêu hao bộ nhớ đệm (buffer) và chu kỳ vi xử lý CPU.

---

## Hình tượng hóa đời sống (Intuitive Everyday Analogy)

Hãy quên đi các định nghĩa toán học khô khan. Để thấu hiểu Big-O, chúng ta hãy bước vào một căn bếp quen thuộc và so sánh: **Nấu bữa tối cho 1 người ăn vs Chuẩn bị đại tiệc cưới cho 500 thực khách**.

```
┌──────────────────────────────────────────────────────────────────────────────────┐
│                   HÌNH TƯỢNG HÓA CÁC CẤP ĐỘ PHỨC TẠP BIG-O                      │
├─────────────┬───────────────────────────────────────┬────────────────────────────┤
│   KÝ HIỆU   │    HÌNH ẢNH ẨN DỤ TRONG ĐỜI SỐNG      │   KHI N TĂNG TỪ 10 LÊN 1000│
├─────────────┼───────────────────────────────────────┼────────────────────────────┤
│   O(1)      │ Bật công tắc đèn / Rót 1 ly nước lọc  │ Vẫn mất 1 giây duy nhất    │
│   O(log N)  │ Chặt đôi danh bạ tìm tên A-Z          │ Từ 3 lần lật lên 10 lần lật│
│   O(N)      │ Rửa từng chiếc bát đĩa sau bữa tiệc   │ Từ 10 phút lên 16 tiếng!   │
│   O(N log N)│ Chia bát đĩa theo bàn rồi rửa theo ca │ Từ 30 giây lên 10 giây x 10│
│   O(N^2)    │ Mọi khách mời lần lượt bắt tay nhau   │ Từ 100 cái lên 1.000.000!  │
└─────────────┴───────────────────────────────────────┴────────────────────────────┘
```

### 1. Cấp độ $O(1)$ — Thời gian hằng số (Làm ngay tức khắc)
Hãy tưởng tượng bạn bước vào phòng bếp và **bật công tắc đèn**, hoặc **rót một ly nước lọc** cho vị khách đầu tiên.
- Bữa tiệc có 1 khách: Bạn bấm công tắc mất 1 giây.
- Bữa tiệc có 500 khách: Bạn vẫn chỉ cần bấm công tắc đèn đó đúng 1 giây.
- Dù số lượng khách $N$ có tăng lên hàng triệu người, việc bật đèn không tốn thêm bất kỳ một tích tắc nào. Trong lập trình, đây là thao tác $O(1)$ — tốc độ lý tưởng nhất của mọi thuật toán.

### 2. Cấp độ $O(N)$ — Thời gian tuyến tính (Lần lượt từng việc)
Bữa tiệc kết thúc, đến công đoạn **rửa bát đĩa**:
- Nếu tiệc có $N = 10$ người: Bạn phải rửa 10 chiếc bát, tốn khoảng 5 phút.
- Nếu tiệc có $N = 500$ người: Bạn phải rửa 500 chiếc bát, tốn khoảng 250 phút (hơn 4 tiếng đồng hồ!).
- Số lượng công việc tăng tỷ lệ thuận trực tiếp với số lượng khách $N$. Nếu số bát tăng gấp 10 lần, thời gian rửa tăng đúng 10 lần. Đây chính là $O(N)$ — tương đương với một vòng lặp duyệt qua từng phần tử của danh sách.

### 3. Cấp độ $O(\log N)$ — Chặt đôi chia để trị (Siêu tốc độ)
Hãy tưởng tượng bạn cầm trên tay cuốn **danh bạ điện thoại dày 1.000 trang** đã được sắp xếp theo thứ tự bảng chữ cái từ A đến Z, và cần tìm tên "Nguyễn Văn An":
- Bạn mở ngay chính giữa cuốn sách (trang 500). Bạn thấy chữ cái "L". Vì "N" đứng sau "L", bạn lập tức loại bỏ được 500 trang đầu tiên!
- Bạn tiếp tục mở đôi 500 trang còn lại (trang 750). Cứ mỗi lần lật sách, bạn **chia đôi phạm vi tìm kiếm** làm hai nửa.
- Với 1.000 trang, bạn chỉ cần tối đa khoảng 10 lần lật sách là tìm thấy người cần tìm! Kể cả danh bạ dày 1.000.000 trang, bạn cũng chỉ tốn đúng 20 lần lật sách. Đó chính là sức mạnh kỳ diệu của $O(\log N)$.

### 4. Cấp độ $O(N^2)$ — Bùng nổ thảm họa (Vòng lặp lồng nhau)
Trong đám cưới, người dẫn chương trình yêu cầu: **"Tất cả các vị khách có mặt trong hội trường phải lần lượt đến từng chiếc bàn để bắt tay và chào hỏi từng vị khách khác!"**
- Nếu hội trường chỉ có $N = 5$ người bạn thân: Tổng số lượt bắt tay là $5 \times 4 / 2 = 10$ cái bắt tay, diễn ra vui vẻ trong 1 phút.
- Nhưng nếu hội trường có $N = 500$ khách: Mỗi người phải đi bắt tay 499 người còn lại! Tổng số cái bắt tay là xấp xỉ $\frac{500 \times 500}{2} = 125.000$ cái bắt tay! Toàn bộ buổi tiệc sẽ biến thành một mớ hỗn loạn kiệt sức, không ai kịp ăn uống gì. Trong mã nguồn, việc lồng hai vòng lặp `for` lặp lại trên cùng một tập dữ liệu sẽ biến hệ thống của bạn thành thảm họa $O(N^2)$.

---

## Khái niệm & Cơ chế kỹ thuật chuyên sâu (In-Depth Technical Mechanics)

### 1. Bản chất phần cứng: Vì sao CPU quan tâm đến Big-O?

Bộ vi xử lý trung tâm CPU của máy tính hiện đại hoạt động theo các chu kỳ xung nhịp (clock cycles). Một vi xử lý tốc độ 3.0 GHz có thể thực hiện khoảng 3 tỷ chu kỳ mỗi giây. 

Tuy nhiên, tài nguyên phần cứng không phải là vô tận:
1. **Truy xuất bộ nhớ (Memory Access Latency)**: CPU truy xuất thanh ghi (registers) tốn dưới 1 nanosecond, nhưng truy xuất thanh RAM chính tốn tới 50-100 nanoseconds (chậm hơn hàng trăm lần).
2. **Bộ nhớ đệm (buffer) Cache L1/L2/L3**: Khi dữ liệu nằm gọn trong cache CPU, thuật toán chạy cực nhanh. Nhưng khi thuật toán bắt CPU nhảy cóc lung tung qua hàng triệu ô nhớ rải rác trên RAM, hiện tượng trượt cache (Cache Miss) xảy ra liên tục, kéo tụt hiệu năng xuống đáy vực.

Ký hiệu Big-O không đo lường bằng giây hay mili-giây tuyệt đối (vì mỗi cỗ máy tính có phần cứng mạnh yếu khác nhau), mà đo lường **xu hướng tăng trưởng số lượng chỉ thị lệnh của CPU** khi kích thước đầu vào $N$ tiến tới vô cực:

```
Số phép tính
 ▲
 │                                                 O(N^2) [Thảm họa]
 │                                          . '
 │                                      . '
 │                                   . '
 │                                . '       O(N log N) [Chấp nhận được]
 │                             . '     . - '
 │                         . '   . - '      O(N) [Tuyến tính]
 │                     . ' . - '
 │                 . - '                O(log N) [Rất tốt]
 │            . - '             . - - - - - - - -
 │     . - - '      . - - - - -
 │────────────────────────────────────────── O(1) [Lý tưởng tuyệt đối]
 └────────────────────────────────────────────────────────► Quy mô dữ liệu (N)
```

### 2. So sánh cụ thể số bước thực thi giữa các cấp Big-O

Bảng dưới đây minh họa số phép toán mà CPU cần thực hiện khi quy mô dữ liệu $N$ tăng dần:

| Ký hiệu Big-O | $N = 10$ | $N = 100$ | $N = 1.000$ | $N = 1.000.000$ (1 triệu) | Đánh giá trực quan |
|---|---|---|---|---|---|
| **$O(1)$** | 1 | 1 | 1 | 1 | Tức thì, tối ưu nhất |
| **$O(\log N)$** | ~3 | ~7 | ~10 | ~20 | Siêu tốc (chia để trị) |
| **$O(N)$** | 10 | 100 | 1.000 | 1.000.000 | Tốt, chấp nhận được |
| **$O(N \log N)$** | ~33 | ~664 | ~9.965 | ~20.000.000 | Chuẩn mực của sắp xếp |
| **$O(N^2)$** | 100 | 10.000 | 1.000.000 | 1.000.000.000.000 ($10^{12}$) | Nguy hiểm, đơ máy |
| **$O(2^N)$** | 1.024 | $1.26 \times 10^{30}$ | Vô tận | Không thể tính toán | Bùng nổ hàm mũ |

> **Quy tắc bỏ qua hằng số**: Trong Big-O, chúng ta chỉ quan tâm đến tốc độ tăng trưởng bậc cao nhất. Ví dụ thuật toán tốn $2N + 100$ bước tính vẫn được quy về $O(N)$, và thuật toán tốn $0.5N^2 + 3N$ bước tính sẽ được quy về $O(N^2)$. Khi $N$ lên tới 1 tỷ, con số cộng thêm 100 hay nhân 2 trở nên hoàn toàn không đáng kể so với sức ảnh hưởng của $N^2$.

### 3. Độ phức tạp không gian (Space Complexity)

Bên cạnh thời gian chạy, thuật toán còn tiêu tốn bộ nhớ RAM để lưu trữ biến số, cấu trúc dữ liệu phụ trợ hoặc các khung ngăn xếp gọi hàm (call stack frames):
- **$O(1)$ Space**: Thuật toán chỉ sử dụng một vài biến đơn lẻ cố định (`let mut tong = 0;`), không xin thêm bất kỳ ô nhớ nào dù $N$ có lớn bao nhiêu.
- **$O(N)$ Space**: Thuật toán tạo ra một mảng phụ sao chép toàn bộ $N$ phần tử, hoặc đệ quy sâu $N$ tầng khiến ngăn xếp Stack phình to theo tỷ lệ thuận.

---

## Mã nguồn minh họa thực chiến (Idiomatic Runnable Rust Blueprint)

Dưới đây là một chương trình Rust hoàn chỉnh, độc lập và có thể chạy ngay bằng `cargo run` hoặc `rustc`. Chương trình minh họa và đo đạc thời gian thực tế giữa ba cấp độ giải thuật cốt lõi: $O(1)$ truy cập chỉ số, $O(N)$ tìm kiếm tuần tự (Linear Search), và $O(\log N)$ tìm kiếm nhị phân (Binary Search), kèm theo phân tích độ phức tạp không gian $O(1)$ vs $O(N)$:

```rust
use std::time::Instant;

/// Minh họa giải thuật O(1) - Truy cập phần tử qua chỉ số mảng
/// Bất kể danh sách có 10 phần tử hay 10 triệu phần tử,
/// CPU chỉ cần 1 phép tính cộng địa chỉ bộ nhớ là lấy được giá trị ngay!
pub fn index_access_o1(list: &[i32], chi_so: usize) -> Option<i32> {
    // Thao tác kiểm tra biên giới và đọc ô nhớ diễn ra trong thời gian hằng số O(1)
    list.get(chi_so).copied()
}

/// Minh họa giải thuật O(N) - Tìm kiếm tuyến tính (Linear Search)
/// Trong trường hợp xấu nhất (Worst-case), phần tử cần tìm nằm ở cuối danh sách
/// hoặc không tồn tại, hàm bắt buộc phải duyệt qua toàn bộ N phần tử.
pub fn linear_search_on(list: &[i32], level_spend: i32) -> Option<usize> {
    for (pos_value, &value) in list.iter().enumerate() {
        if value == level_spend {
            return Some(pos_value); // Tìm thấy tại vị trí pos_value
        }
    }
    None // Không tìm thấy sau khi duyệt hết N phần tử
}

/// Minh họa giải thuật O(log N) - Tìm kiếm nhị phân (Binary Search)
/// Điều kiện tiên quyết: Mảng đầu vào PHẢI được sắp xếp tăng dần từ trước.
/// Tại mỗi bước, ta so sánh mục tiêu với phần tử ở giữa và loại bỏ 50% phạm vi tìm kiếm.
pub fn binary_search_ologn(list: &[i32], level_spend: i32) -> Option<usize> {
    if list.is_empty() {
        return None;
    }

    let mut left: usize = 0;
    let mut right: usize = list.len() - 1;

    while left <= right {
        // Tính vị trí ở giữa an toàn để tránh nguy cơ tràn số (integer overflow)
        let mid = left + (right - left) / 2;
        let value_mid = list[mid];

        if value_mid == level_spend {
            return Some(mid);
        } else if value_mid < level_spend {
            // Mục tiêu nằm ở nửa bên phải, dời biên trái lên
            left = mid + 1;
        } else {
            // Mục tiêu nằm ở nửa bên trái, dời biên phải xuống
            if mid == 0 {
                break; // Ngăn chặn tràn số usize khi trừ về dưới 0
            }
            right = mid - 1;
        }
    }

    None
}

/// Minh họa độ phức tạp không gian O(1) vs O(N)
/// Hàm 1: Tính tổng tích lũy tại chỗ - Tiêu tốn O(1) bộ nhớ phụ
pub fn sum_in_place_o1(list: &[i32]) -> i64 {
    let mut tong: i64 = 0; // Biến duy nhất trên Stack, không tốn thêm Heap
    for &so in list {
        tong += so as i64;
    }
    tong
}

/// Hàm 2: Tạo mảng nhân đôi - Tiêu tốn O(N) bộ nhớ phụ trên Heap
pub fn grow_doubling(list: &[i32]) -> Vec<i32> {
    let mut ket_qua = Vec::with_capacity(list.len());
    for &so in list {
        ket_qua.push(so * 2);
    }
    ket_qua
}

fn main() {
    println!("============================================================");
    println!("   THỰC NGHIỆM ĐO ĐẠC ĐỘ PHỨC TẠP TÍNH TOÁN VỚI BIG-O       ");
    println!("============================================================");

    // Chuẩn bị tập dữ liệu lớn gồm 1.000.000 (1 triệu) số nguyên đã sắp xếp
    let scale: usize = 1_000_000;
    println!("Khởi tạo danh sách gồm {} phần tử...", scale);
    let list: Vec<i32> = (0..scale as i32).collect();

    let level_spend: i32 = 999_999; // Phần tử nằm ở cuối cùng (trường hợp xấu nhất)

    // 1. Thực nghiệm O(1) - Truy cập trực tiếp qua chỉ số
    let start_o1 = Instant::now();
    let ket_qua_o1 = index_access_o1(&list, scale - 1);
    let elapsed_o1 = start_o1.elapsed();
    println!("\n[1] Thao tác O(1) - Truy cập chỉ số:");
    println!("    - Giá trị tìm được: {:?}", ket_qua_o1);
    println!("    - Thời gian thực thi: {:?}", elapsed_o1);

    // 2. Thực nghiệm O(N) - Tìm kiếm tuyến tính duyệt từ đầu đến cuối
    let start_on = Instant::now();
    let ket_qua_on = linear_search_on(&list, level_spend);
    let elapsed_on = start_on.elapsed();
    println!("\n[2] Thao tác O(N) - Tìm kiếm tuyến tính (Duyệt 1 triệu phần tử):");
    println!("    - Vị trí tìm được: {:?}", ket_qua_on);
    println!("    - Thời gian thực thi: {:?}", elapsed_on);

    // 3. Thực nghiệm O(log N) - Tìm kiếm nhị phân (Chặt đôi chia để trị)
    let start_ologn = Instant::now();
    let ket_qua_ologn = binary_search_ologn(&list, level_spend);
    let elapsed_ologn = start_ologn.elapsed();
    println!("\n[3] Thao tác O(log N) - Tìm kiếm nhị phân (Chỉ tốn ~20 phép chia):");
    println!("    - Vị trí tìm được: {:?}", ket_qua_ologn);
    println!("    - Thời gian thực thi: {:?}", elapsed_ologn);

    // Xác nhận tính nhất quán của kết quả
    assert_eq!(ket_qua_on, Some(scale - 1));
    assert_eq!(ket_qua_ologn, Some(scale - 1));

    // 4. So sánh tỷ lệ chênh lệch thời gian giữa O(log N) và O(N)
    if elapsed_ologn.as_nanos() > 0 {
        let ti_le = elapsed_on.as_nanos() as f64 / elapsed_ologn.as_nanos() as f64;
        println!("\n=> ĐÁNH GIÁ: O(log N) chạy nhanh gấp xấp xỉ {:.1} lần so với O(N)!", ti_le);
    }

    // 5. Kiểm tra tính năng tiêu thụ bộ nhớ không gian
    let tong_o1 = sum_in_place_o1(&list[0..100]);
    let mang_on = grow_doubling(&list[0..100]);
    println!("\n[4] Không gian bộ nhớ:");
    println!("    - Tổng O(1) Space: {}", tong_o1);
    println!("    - Kích thước mảng phụ O(N) Space: {} phần tử", mang_on.len());
    println!("============================================================");
}
```

---

## Bảng tra cứu lỗi biên dịch & Cách khắc phục (Compiler Error Guide)

Khi lập trình các thuật toán tìm kiếm và đo đạc độ phức tạp tính toán trong Rust, người học thường đối mặt với các lỗi biên dịch điển hình liên quan đến việc mượn (borrow) và di chuyển quyền sở hữu (ownership):

| Mã lỗi | Thông báo mẫu từ trình biên dịch | Nguyên nhân cốt lõi | Cách khắc phục nhanh |
|---|---|---|---|
| **E0382** | `use of moved value: '...'` | Bạn truyền một `Vec` lớn vào hàm giải thuật bằng giá trị (by value) thay vì mượn tham chiếu `&[T]`. Quyền sở hữu đã bị chuyển đi, khiến biến gốc không dùng lại được. | Đổi chữ ký hàm nhận lát cắt tham chiếu `&[T]` thay vì sở hữu `Vec<T>`. |
| **E0596** | `cannot borrow '...' as mutable, as it is not declared as mutable` | Bạn cố gắng thay đổi các biến chỉ số biên (`left`, `right`) trong thuật toán tìm kiếm mà quên khai báo từ khóa `mut`. | Thêm từ khóa `mut` khi khai báo biến: `let mut left = 0;`. |
| **E0308** | `mismatched types: expected 'usize', found 'i32'` | Chỉ số mảng trong Rust luôn mang kiểu số nguyên không dấu `usize`. Việc dùng kiểu `i32` làm chỉ số truy cập sẽ bị trình biên dịch từ chối ngay lập tức. | Chuyển đổi kiểu tường minh bằng từ khóa `as usize` hoặc khai báo biến chỉ số ngay từ đầu là `usize`. |
| **E0502** | `cannot borrow '...' as mutable because it is also borrowed as immutable` | Bạn vừa mượn bất biến `&list` để lặp, vừa gọi phương thức làm biến đổi danh sách (như `.push()`) trong cùng một phạm vi. | Tách rời thao tác đọc và thao tác ghi thành hai bước độc lập để tôn trọng quy tắc mượn của Rust. |

### Ví dụ phân tích lỗi `E0382` và cách khắc phục:

```rust
// Đoạn mã lỗi minh họa E0382: Di chuyển quyền sở hữu vector vào hàm đo thời gian
fn count_broken(list: Vec<i32>) -> usize {
    list.len() // Hàm đoạt lấy quyền sở hữu và giải phóng bộ nhớ khi kết thúc
}

fn broken_example() {
    let data = vec![1, 2, 3, 4, 5];
    // let n = count_broken(data); 
    // println!("Dữ liệu có: {}", data.len()); // LỖI E0382: data đã bị di chuyển!
}

// Cách sửa chữa đúng chuẩn: Mượn lát cắt (Slice) tham chiếu &[i32]
fn count_idiomatic(list: &[i32]) -> usize {
    list.len() // Chỉ mượn tham chiếu, không đoạt quyền sở hữu
}

fn correct_example() {
    let data = vec![1, 2, 3, 4, 5];
    let n = count_idiomatic(&data);
    println!("Dữ liệu mượn hợp lệ, vẫn còn sử dụng được: độ dài = {}", n);
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

    #[test]
    fn index_access_is_o1_and_bounds_checked() {
        let list = [10, 20, 30];
        assert_eq!(index_access_o1(&list, 1), Some(20));
        assert_eq!(index_access_o1(&list, 5), None); // an toàn, không panic
    }

    #[test]
    fn linear_search() {
        let list = [4, 8, 15, 16, 23, 42];
        assert_eq!(linear_search_on(&list, 15), Some(2));
        assert_eq!(linear_search_on(&list, 99), None);
    }

    #[test]
    fn binary_search_matches_linear() {
        let list: Vec<i32> = (0..1000).map(|x| x * 3).collect();
        for &level_spend in &[0, 297, 1500, 2997, 1, 2998] {
            // hai thuật toán phải cho CÙNG kết luận có/không
            assert_eq!(
                binary_search_ologn(&list, level_spend).is_some(),
                linear_search_on(&list, level_spend).is_some(),
                "bất đồng ở {}", level_spend
            );
        }
        assert_eq!(binary_search_ologn(&list, 297), Some(99));
    }

    #[test]
    fn binary_search_on_empty_and_single() {
        assert_eq!(binary_search_ologn(&[], 5), None);
        assert_eq!(binary_search_ologn(&[5], 5), Some(0));
        assert_eq!(binary_search_ologn(&[5], 3), None);
    }

    #[test]
    fn sum_uses_o1_space() {
        assert_eq!(sum_in_place_o1(&[1, 2, 3, 4]), 10);
        assert_eq!(sum_in_place_o1(&[]), 0);
    }

    #[test]
    fn doubling_grows_in_on_space() {
        assert_eq!(grow_doubling(&[1, 2, 3]), vec![2, 4, 6]);
    }
}
```

## Tóm tắt chương & Bài tập rèn luyện (Summary & Exercises)

### 4 Điểm cốt lõi cần ghi nhớ:
1. **Big-O đo xu hướng tăng trưởng**: Không đo số giây cụ thể, Big-O phản ánh tốc độ phình to của khối lượng công việc khi kích thước dữ liệu $N$ tăng lên vô tận.
2. **Thứ bậc hiệu năng**: $O(1) < O(\log N) < O(N) < O(N \log N) < O(N^2) < O(2^N)$. Luôn tìm cách đưa giải thuật về $O(1)$ hoặc $O(\log N)$ nếu có thể.
3. **Độ phức tạp thời gian vs Không gian**: Nhanh hơn thường đòi hỏi tốn RAM hơn (đánh đổi Time-Space Trade-off). Một thuật toán tốt phải cân đối hài hòa cả hai yếu tố.
4. **Tham chiếu lát cắt trong Rust**: Truyền dữ liệu vào các hàm thuật toán dưới dạng lát cắt mượn `&[T]` để đạt hiệu năng $O(1)$ về chi phí truyền tham số và bảo toàn quyền sở hữu (ownership).

### Bài tập rèn luyện tự giải:
1. **Bài tập 1 (Nhận diện Big-O đời sống)**:  
   Trong các hoạt động sau đây, hoạt động nào có độ phức tạp $O(1)$, $O(N)$, và $O(N^2)$?
   - a) Tìm chìa khóa mở cửa nhà trong chùm chìa khóa có $N$ chiếc chìa không đánh dấu.
   - b) Tra cứu số phòng của một vị khách khi khách đưa thẻ căn cước có ghi rõ số phòng trên mặt thẻ.
   - c) So sánh từng bức ảnh trong bộ sưu tập $N$ bức ảnh với tất cả các bức ảnh còn lại để tìm ảnh trùng lặp.
2. **Bài tập 2 (Phát hiện nút thắt cổ chai)**:  
   Đoạn mã sau đây có độ phức tạp thời gian là bao nhiêu? Làm thế nào để cải tiến nó?
   ```rust
   // Đoạn mã kiểm tra xem mảng có chứa hai số trùng nhau hay không
   fn has_duplicate(list: &[i32]) -> bool {
       for i in 0..list.len() {
           for j in (i + 1)..list.len() {
               if list[i] == list[j] {
                   return true;
               }
           }
       }
       false
   }
   ```
3. **Bài tập 3 (Thực hành đo đạc)**:  
   Hãy viết một chương trình Rust sử dụng `std::time::Instant` để so sánh thời gian cộng dồn 1 triệu số nguyên từ 1 đến 1.000.000 bằng vòng lặp `for` (mất $O(N)$ bước) so với việc áp dụng công thức tính nhanh của nhà toán học Gauss: $S = \frac{N(N + 1)}{2}$ (chỉ mất $O(1)$ phép tính). Quan sát và in ra sự chênh lệch thời gian giữa hai phương pháp.

---

### Gợi ý & Lời giải

<details>
<summary><b>Bài tập 1 — Gợi ý</b></summary>

Hỏi một câu duy nhất cho mỗi tình huống: *khi số lượng tăng gấp đôi, công việc tăng bao nhiêu lần?*
</details>

<details>
<summary><b>Bài tập 1 — Lời giải</b></summary>

| | Tình huống | Độ phức tạp | Vì sao |
|---|---|---|---|
| a | Tìm chìa trong chùm N chiếc không đánh dấu | **O(N)** | Tệ nhất phải thử hết N chiếc. Gấp đôi số chìa thì gấp đôi số lần thử |
| b | Tra số phòng in sẵn trên thẻ | **O(1)** | Đọc một chỗ cố định. Khách sạn 10 phòng hay 10.000 phòng cũng vậy |
| c | So từng ảnh với mọi ảnh còn lại | **O(N²)** | Mỗi ảnh so với N−1 ảnh khác → N(N−1)/2 phép so. Gấp đôi số ảnh thì công việc gấp **bốn** |

Điều đáng nhớ ở (c): với 1.000 ảnh là ~500.000 phép so — máy tính làm trong nháy mắt. Với 100.000 ảnh là ~5 tỷ phép so — hàng giờ. Đó là lý do O(N²) không phải "hơi chậm" mà là **không dùng được** khi dữ liệu lớn lên.

Và (c) có cách hạ xuống O(N): băm từng ảnh rồi bỏ vào `HashSet`. Đổi thời gian lấy bộ nhớ — đúng cái đánh đổi mà chương này nói tới.
</details>

<details>
<summary><b>Bài tập 2 — Gợi ý</b></summary>

Hai vòng lặp lồng nhau, vòng trong chạy tới `list.len()` → O(N²). Muốn hạ xuống O(N) thì cần một cấu trúc tra cứu O(1).
</details>

<details>
<summary><b>Bài tập 2 — Lời giải</b></summary>

Đoạn mã đã cho là **O(N²)**: với mỗi `i`, vòng trong chạy tới N lần, tổng cộng N(N−1)/2 phép so.

```rust
use std::collections::HashSet;

/// Bản O(N²) — trong đề bài. Không cấp phát thêm bộ nhớ.
pub fn has_duplicate_on2(list: &[i32]) -> bool {
    for i in 0..list.len() {
        for j in (i + 1)..list.len() {
            if list[i] == list[j] { return true; }
        }
    }
    false
}

/// Bản O(N) — đổi thời gian lấy BỘ NHỚ.
/// `insert` trả false nếu phần tử đã có -> phát hiện trùng ngay.
pub fn has_duplicate_on(list: &[i32]) -> bool {
    let mut da_thay = HashSet::with_capacity(list.len());
    !list.iter().all(|x| da_thay.insert(*x))
}

#[test]
fn hai_ban_cho_cung_ket_qua() {
    for mau in [vec![], vec![1], vec![1, 2, 3], vec![1, 2, 1], vec![5, 5]] {
        assert_eq!(has_duplicate_on2(&mau), has_duplicate_on(&mau), "{mau:?}");
    }
}
```

**Đánh đổi thật sự:** bản O(N) nhanh hơn rất nhiều với dữ liệu lớn, nhưng tốn O(N) bộ nhớ và đòi `T: Hash + Eq`. Bản O(N²) chạy được với mọi kiểu chỉ cần `PartialEq`, và **không cấp phát gì** — với mảng 10 phần tử nó còn nhanh hơn, vì chi phí dựng `HashSet` lớn hơn cả việc so 45 cặp.

Đó là lý do `slice::contains` của thư viện chuẩn vẫn là tuyến tính: với dữ liệu nhỏ, đơn giản thắng.
</details>

<details>
<summary><b>Bài tập 3 — Gợi ý</b></summary>

Dùng `std::time::Instant::now()` rồi `.elapsed()`. Nhớ `std::hint::black_box` để trình tối ưu không xoá mất vòng lặp mà bạn muốn đo.
</details>

<details>
<summary><b>Bài tập 3 — Lời giải</b></summary>

```rust
use std::time::Instant;
use std::hint::black_box;

fn main() {
    const N: u64 = 1_000_000;

    // O(N): cộng dồn từng số
    let t0 = Instant::now();
    let mut tong_lap: u64 = 0;
    for i in 1..=N { tong_lap += i; }
    let tg_lap = t0.elapsed();
    black_box(tong_lap);   // ngăn trình tối ưu xoá cả vòng lặp

    // O(1): công thức Gauss
    let t1 = Instant::now();
    let tong_gauss = N * (N + 1) / 2;
    let tg_gauss = t1.elapsed();
    black_box(tong_gauss);

    assert_eq!(tong_lap, tong_gauss, "hai cách phải ra cùng con số");

    println!("Vòng lặp O(N) : {:?}", tg_lap);
    println!("Gauss    O(1) : {:?}", tg_gauss);
    println!("Nhanh hơn     : {:.0} lần",
             tg_lap.as_nanos() as f64 / tg_gauss.as_nanos().max(1) as f64);
}
```

**Ba cái bẫy khi đo, cả ba đều thật:**

1. **Không có `black_box`, trình tối ưu xoá sạch vòng lặp.** Nó thấy `tong_lap` không được dùng và bỏ luôn — bạn đo được 0 nano giây và tưởng mình vừa phát minh ra thuật toán thần kỳ.
2. **Bản `-O` và bản gỡ lỗi khác nhau hàng chục lần.** Luôn đo bằng `cargo run --release`.
3. **Gauss có thể ra ~0 ns** vì trình biên dịch tính sẵn lúc biên dịch (`N` là hằng số). Muốn đo trung thực thì đọc `N` từ đầu vào lúc chạy.

Con số bạn nhìn thấy thường vào khoảng vài trăm micro giây so với vài nano giây — chênh khoảng **năm bậc**. Nhưng bài học quan trọng hơn con số: cả hai đều cho *cùng một kết quả*, chỉ khác cách đi tới nó.
</details>
