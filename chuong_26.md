# Chương 26: Lưu trữ vùng nhớ liền kề: Mảng cố định, Vector động và Lát cắt (Contiguous Memory: Arrays, Vectors & Slices)

## Giới thiệu & Mục tiêu học tập

Trong lập trình hệ thống hiệu năng cao, cách dữ liệu được sắp đặt vật lý trên các thanh RAM đóng vai trò quyết định đến tốc độ của toàn bộ ứng dụng. Một thuật toán dù có độ phức tạp lý thuyết là $O(N)$ nhưng nếu dữ liệu bị xé nhỏ và ném rải rác khắp nơi trong bộ nhớ sẽ chạy chậm hơn gấp hàng chục lần so với một thuật toán cũng $O(N)$ nhưng dữ liệu nằm san sát nhau trên cùng một dải ô nhớ liên tục. Hiện tượng này bắt nguồn từ tính chất phần cứng vi xử lý CPU: **Tính cục bộ không gian (Spatial Locality)** và cơ chế nạp trước của bộ nhớ đệm (cache prefetching).

Để khai thác tối đa sức mạnh phần cứng mà vẫn đảm bảo 100% an toàn bộ nhớ (không lo tràn bộ nhớ đệm hay truy cập ngoài biên), Rust cung cấp ba cấu trúc dữ liệu lưu trữ liền kề cốt lõi:
1. **Mảng cố định (Array - `[T; N]`)**: Kích thước cố định từ lúc biên dịch, nằm trực tiếp trên Ngăn xếp (Stack).
2. **Mảng động (Vector - `Vec<T>`)**: Tự động co giãn kích thước linh hoạt, quản lý vùng nhớ trên Vùng nhớ tự do (Heap).
3. **Lát cắt (Slice - `&[T]` và `&mut [T]`)**: Cửa sổ góc nhìn (View) trỏ vào một phần của mảng hoặc vector mà không tốn chi phí sao chép dữ liệu.

Mục tiêu học tập của chương này:
- Thấu hiểu cơ chế tổ chức vật lý của **Vùng nhớ liền kề (Contiguous Memory)** và lý do tại sao nó lại thân thiện tuyệt đối với bộ nhớ đệm (cache) của CPU.
- Phân biệt rạch ròi sự khác nhau giữa `Array`, `Vec`, và `Slice` về vị trí bộ nhớ (Stack vs Heap) và chi phí vận hành.
- Nắm vững cơ chế tăng trưởng tự động của `Vec` (Độ dài `len` vs Sức chứa `capacity`, chiến lược nhân đôi dung lượng và chi phí khấu hao amortized $O(1)$).
- Làm chủ kỹ thuật sử dụng `with_capacity` để triệt tiêu các lần tái cấp phát bộ nhớ lãng phí.
- Sử dụng thành thạo lát cắt (`&[T]`) như một cầu nối trừu tượng giúp hàm chấp nhận mọi dạng tập hợp liền kề mà không cần chuyển giao quyền sở hữu (ownership).

---

## Hình tượng hóa đời sống (Intuitive Everyday Analogy)

Hãy tưởng tượng bạn bước vào phòng thay đồ của một phòng tập Gym hiện đại để cất giữ đồ đạc:

```
┌──────────────────────────────────────────────────────────────────────────────────┐
│              HÌNH TƯỢNG HÓA VÙNG NHỚ LIỀN KỀ: ARRAY, VECTOR VÀ SLICE              │
├──────────────────────────────────────────────────────────────────────────────────┤
│                                                                                  │
│ [ARRAY: DÃY TỦ KHÓA CỐ ĐỊNH TẠI PHÒNG GYM]                                       │
│ ┌─────────┬─────────┬─────────┬─────────┬─────────┐                              │
│ │ Ngăn 0  │ Ngăn 1  │ Ngăn 2  │ Ngăn 3  │ Ngăn 4  │ -> Hàn chết vào tường (Stack)│
│ │ [Áo thun]│ [Giày]  │ [Bình]  │ [Khăn]  │ [Ví]    │    Không thêm/bớt ngăn tủ    │
│ └─────────┴─────────┴─────────┴─────────┴─────────┘                              │
│                                                                                  │
│ [VECTOR: CHIẾC VALI DU LỊCH THÔNG MINH CO GIÃN TRÊN HEAP]                        │
│ ┌───────────────────────┬─────────────────────────┐                              │
│ │ Đang dùng: len = 3    │ Còn trống: cap = 6      │ -> Để dưới sàn kho (Heap)    │
│ │ [Đồ 1] [Đồ 2] [Đồ 3]  │ [Trống] [Trống] [Trống] │    Đầy thì đổi vali to gấp đôi│
│ └───────────────────────┴─────────────────────────┘                              │
│                                                                                  │
│ [SLICE: KHUNG KÍNH SOI MỘT ĐOẠN NGĂN TỦ]                                         │
│                 │◄─── lát cắt &[1..4] ───►│                                      │
│                 ┌─────────┬─────────┬─────┴───┐                                  │
│                 │ Ngăn 1  │ Ngăn 2  │ Ngăn 3  │   -> Chỉ là thước đo góc nhìn    │
│                 │ [Giày]  │ [Bình]  │ [Khăn]  │      Không tốn tủ mới!           │
│                 └─────────┴─────────┴─────────┘                                  │
└──────────────────────────────────────────────────────────────────────────────────┘
```

### 1. Mảng tĩnh (Array) — Dãy tủ khóa cố định gắn chặt vào tường
- Dãy tủ gồm đúng 5 ngăn được đúc bằng thép nguyên khối và hàn chặt vào tường phòng thay đồ (Stack).
- Kích thước này là vĩnh cửu: Bạn không thể nào gắn thêm ngăn thứ 6, cũng không thể tháo bớt đi ngăn nào.
- Đổi lại, vì vị trí các ngăn tủ được đánh số thứ tự liền kề nhau (`0, 1, 2, 3, 4`), bạn chỉ cần bước tới ngăn số 0 là có thể với tay mở ngăn số 1 hay ngăn số 2 ngay lập tức mà không cần đi tìm kiếm quanh phòng.

### 2. Mảng động (Vector) — Chiếc vali du lịch có khóa kéo mở rộng
- Khi chuẩn bị đi du lịch dài ngày, bạn không biết trước mình sẽ mua thêm bao nhiêu món quà lưu niệm. Bạn chọn dùng một chiếc **vali du lịch co giãn** để dưới kho hành lý (Heap).
- Ban đầu, chiếc vali có sức chứa 4 ngăn đồ (`capacity = 4`). Bạn bỏ vào 3 bộ quần áo (`len = 3`).
- Khi bạn mua thêm món đồ thứ 5 vượt quá sức chứa, điều gì xảy ra? Chiếc vali thông minh sẽ:
  1. Yêu cầu khách sạn cấp một chiếc vali mới to gấp đôi (`capacity = 8`).
  2. Bốc toàn bộ 4 món đồ cũ từ vali cũ chuyển sang vali mới.
  3. Bỏ món đồ thứ 5 vào, và vứt bỏ chiếc vali cũ vào thùng tái chế.
- Thao tác chuyển nhà này có tốn công không? Có chứ! Nhưng vì mỗi lần đổi vali kích thước lại nhân đôi (4 -> 8 -> 16 -> 32...), số lần chuyển nhà diễn ra ngày càng thưa thớt, giúp cho chi phí trung bình (Amortized Cost) để thêm một món đồ vẫn đạt $O(1)$.

### 3. Lát cắt (Slice) — Khung kính ngắm một đoạn ngăn tủ
- Giả sử bạn muốn nhờ người bạn thân: *"Hãy kiểm tra giúp tôi các món đồ từ ngăn số 1 đến ngăn số 3 xem đã giặt sạch chưa"*.
- Bạn không cần phải cưa đứt dãy tủ khóa mang về nhà bạn ấy, cũng không cần photocopy lại các món đồ.
- Bạn chỉ cần đưa cho người bạn một mẩu giấy ghi: **"Bắt đầu từ ngăn số 1, đếm đúng 3 ngăn tiếp theo"**. Đó chính là một lát cắt `Slice` — chỉ gồm một con trỏ địa chỉ và một độ dài, cực kỳ nhẹ nhàng và không tiêu tốn thêm một byte bộ nhớ dữ liệu nào.

---

## Khái niệm & Cơ chế kỹ thuật chuyên sâu (In-Depth Technical Mechanics)

### 1. Bố cục bộ nhớ của Array `[T; N]` trên Stack

Trong Rust, mảng tĩnh được khai báo với cú pháp `[T; N]`, trong đó `T` là kiểu dữ liệu và `N` là số lượng phần tử cố định được biết trước ngay từ lúc biên dịch:

```rust
let array: [i32; 4] = [10, 20, 30, 40];
```

Trên thanh RAM (ngăn xếp Stack), mảng này chiếm đúng $4 \times 4 = 16$ bytes liên tục:
```
Địa chỉ ô nhớ: 0x1000   0x1004   0x1008   0x100C
Nội dung     : [  10  ] [  20  ] [  30  ] [  40  ]
Chỉ số index :    0        1        2        3
```
Công thức tính địa chỉ của phần tử thứ `i` vô cùng đơn giản:
$$\text{Địa chỉ}(i) = \text{Địa chỉ gốc} + i \times \text{kích thước kiểu } T$$
Nhờ công thức này, CPU chỉ cần 1 phép nhân và 1 phép cộng số học để nhảy tới bất kỳ phần tử nào trong thời gian hằng số $O(1)$.

### 2. Cấu tạo bên trong của `Vec<T>` (Cơ chế 3 từ máy)

Một biến `Vec<T>` khi nằm trên Stack thực chất chỉ chiếm đúng **3 từ máy (3 usize words = 24 bytes trên hệ điều hành 64-bit)**:
1. **Con trỏ `ptr` (Pointer)**: Địa chỉ 8-byte trỏ tới vùng nhớ thực tế chứa các phần tử trên Heap.
2. **Độ dài `len` (Length)**: Số lượng phần tử thực tế hiện đang có trong vector.
3. **Sức chứa `capacity` (Capacity)**: Tổng số phần tử mà vùng nhớ Heap hiện tại có thể chứa trước khi bắt buộc phải tái cấp phát (reallocate).

```
STACK (24 bytes)                      HEAP (Vùng nhớ tự do)
┌──────────────┬───────┐              ┌────┬────┬────┬────────┬────────┐
│ Con trỏ ptr  │ 0x5000├─────────────►│ 10 │ 20 │ 30 │ [Trống]│ [Trống]│
├──────────────┼───────┤              └────┴────┴────┴────────┴────────┘
│ Chiều dài len│   3   │              Địa chỉ: 0x5000
├──────────────┼───────┤
│ Sức chứa cap │   5   │
└──────────────┴───────┘
```

> **Tối ưu hóa với `Vec::with_capacity(n)`**:  
> Nếu bạn biết trước mình cần nạp 10.000 phần tử, việc gọi `Vec::new()` rồi `push` liên tục sẽ khiến vector phải xin cấp phát và sao chép lại toàn bộ dữ liệu khoảng **13 lần** (với kiểu `i32`, sức chứa đi theo dãy $4 \to 8 \to 16 \to 32 \to \dots \to 16.384$ — Rust khởi đầu ở 4 phần tử chứ không phải 1, rồi nhân đôi mỗi lần đầy). Thay vào đó, khởi tạo ngay `Vec::with_capacity(10_000)` sẽ cấp phát đúng 1 lần duy nhất trên Heap, tăng tốc chương trình lên gấp nhiều lần!

### 3. Con trỏ béo (Fat Pointer) của Lát cắt `&[T]`

Một lát cắt (Slice) là một kiểu dữ liệu có kích thước không cố định (Dynamically Sized Type - DST). Do đó, bạn không bao giờ có thể lưu trực tiếp `[T]` vào một biến, mà luôn phải đứng sau một con trỏ: thường gặp nhất là tham chiếu mượn (borrow) `&[T]` hoặc `&mut [T]`, ngoài ra còn có `Box<[T]>`, `Rc<[T]>`.

Một tham chiếu lát cắt `&[T]` là một **Con trỏ béo (Fat Pointer)** chiếm đúng 16 bytes trên Stack:
- **Địa chỉ con trỏ dữ liệu (8 bytes)**: Trỏ tới phần tử bắt đầu của lát cắt (có thể nằm trên Stack nếu cắt từ Array, hoặc nằm trên Heap nếu cắt từ Vec).
- **Độ dài lát cắt (8 bytes)**: Số lượng phần tử nằm trong phạm vi của lát cắt.

Vì mang theo độ dài bên mình, mỗi khi bạn truy cập `slice[i]`, Rust sẽ thực hiện phép **Kiểm tra biên an toàn (Bounds Check)** lúc chạy. Nếu `i >= len`, chương trình sẽ báo lỗi hoảng loạn (panic) an toàn thay vì đọc lén bộ nhớ rác như C/C++.

---

## Mã nguồn minh họa thực chiến (Idiomatic Runnable Rust Blueprint)

Đoạn mã dưới đây là một chương trình độc lập, minh họa toàn diện từ cách cấp phát, quan sát địa chỉ ô nhớ liền kề, theo dõi chu kỳ tăng trưởng dung lượng của `Vec`, đến kỹ thuật trích xuất lát cắt an toàn:

```rust
/// Hàm tính tổng các phần tử sử dụng lát cắt mượn &[i32]
/// Hàm này có tính tổng quát cực cao: Nó chấp nhận cả mảng tĩnh [i32; N],
/// một phần mảng, hoặc toàn bộ Vector động Vec<i32> mà không cần sao chép dữ liệu!
pub fn sum_slice(data: &[i32]) -> i64 {
    let mut sum: i64 = 0;
    for &value in data {
        sum += value as i64;
    }
    sum
}

/// Hàm đảo ngược các phần tử tại chỗ trên một lát cắt khả biến &mut [i32]
pub fn reverse_in_place(data: &mut [i32]) {
    if data.is_empty() {
        return;
    }
    let mut left = 0;
    let mut right = data.len() - 1;
    while left < right {
        data.swap(left, right);
        left += 1;
        right -= 1;
    }
}

fn main() {
    println!("============================================================");
    println!("     KHẢO SÁT VÙNG NHỚ LIỀN KỀ: ARRAY, VECTOR VÀ SLICE      ");
    println!("============================================================");

    // 1. Khảo sát Mảng tĩnh [T; N] cố định trên Stack
    let static_array: [i32; 5] = [10, 20, 30, 40, 50];
    println!("[1] Mảng tĩnh trên Stack:");
    println!(
        "    - Kích thước vật lý : {} bytes",
        std::mem::size_of_val(&static_array)
    );
    println!("    - Số lượng phần tử  : {}", static_array.len());

    // Kiểm chứng tính chất liền kề của các địa chỉ ô nhớ
    print!("    - Địa chỉ ô nhớ từng phần tử: ");
    for (i, element) in static_array.iter().enumerate() {
        let address = element as *const i32 as usize;
        print!("[Phần tử {}: đuôi ...{:x}] ", i, address % 0x1000);
    }
    println!("\n    => Mỗi ô nhớ cách nhau đúng 4 bytes (kích thước i32)!");

    // 2. Khảo sát Vector động Vec<T> và chu kỳ co giãn dung lượng
    println!("\n[2] Vòng đời co giãn của Vector động (Heap Allocation):");
    let mut growing_vec: Vec<i32> = Vec::new();
    println!(
        "    Ban đầu khi mới tạo: len = {}, cap = {}",
        growing_vec.len(),
        growing_vec.capacity()
    );

    let mut prev_address: usize = 0;
    for i in 1..=9 {
        growing_vec.push(i * 10);
        let current_address = growing_vec.as_ptr() as usize;

        // Phát hiện thời điểm vector đổi nhà sang vùng nhớ mới.
        // Lưu ý: bộ cấp phát có thể nới rộng vùng nhớ TẠI CHỖ (realloc), khi đó
        // cap tăng mà địa chỉ không đổi — vì vậy hãy nhìn cột cap là chính.
        let relocation_note = if current_address != prev_address && prev_address != 0 {
            prev_address = current_address;
            " -> [ĐỔI NHÀ MỚI TRÊN HEAP!]"
        } else {
            prev_address = current_address;
            ""
        };

        println!(
            "    - Thêm {:2}: len = {}, cap = {:2}, ptr = {:x}{}",
            i * 10,
            growing_vec.len(),
            growing_vec.capacity(),
            current_address % 0x10000,
            relocation_note
        );
    }

    // 3. Tối ưu hóa trước với with_capacity
    println!("\n[3] Tối ưu hóa Vector với with_capacity(100):");
    let mut presized_vec: Vec<i32> = Vec::with_capacity(100);
    let ptr_before = presized_vec.as_ptr() as usize;
    for i in 0..100 {
        presized_vec.push(i);
    }
    let ptr_after = presized_vec.as_ptr() as usize;
    println!(
        "    - Sau khi nạp 100 phần tử: len = {}, cap = {}",
        presized_vec.len(),
        presized_vec.capacity()
    );
    println!(
        "    - Địa chỉ vùng nhớ có đổi không? {}",
        if ptr_before == ptr_after {
            "KHÔNG ĐỔI (Cực kỳ tối ưu!)"
        } else {
            "CÓ ĐỔI"
        }
    );
    assert_eq!(ptr_before, ptr_after);

    // 4. Khảo sát Lát cắt (Slice) - Cửa sổ góc nhìn không tốn phí sao chép
    println!("\n[4] Ứng dụng Lát cắt (Slice) linh hoạt:");
    // Lấy lát cắt từ mảng tĩnh
    let array_slice = &static_array[1..4]; // Lấy phần tử chỉ số 1, 2, 3 -> [20, 30, 40]
    println!("    - Lát cắt từ mảng tĩnh [1..4]: {:?}", array_slice);
    let array_sum = sum_slice(array_slice);
    println!("    - Tổng tính từ lát cắt mảng  : {}", array_sum);
    assert_eq!(array_sum, 90);

    // Lấy lát cắt từ vector động
    let vec_slice = &growing_vec[0..5]; // Lấy 5 phần tử đầu tiên
    println!("    - Lát cắt từ vector [0..5]   : {:?}", vec_slice);
    let vec_sum = sum_slice(vec_slice);
    println!("    - Tổng tính từ lát cắt vector: {}", vec_sum);
    assert_eq!(vec_sum, 150);

    // 5. Thao tác trên lát cắt khả biến &mut [T]
    let mut to_reverse = [1, 2, 3, 4, 5, 6];
    println!("\n[5] Đảo ngược tại chỗ trên lát cắt khả biến:");
    println!("    - Mảng ban đầu : {:?}", to_reverse);
    // Đảo ngược chỉ một đoạn ở giữa: từ chỉ số 1 đến 4 (các số 2, 3, 4, 5)
    reverse_in_place(&mut to_reverse[1..5]);
    println!("    - Sau khi đảo đoạn [1..5]: {:?}", to_reverse);
    assert_eq!(to_reverse, [1, 5, 4, 3, 2, 6]);

    println!("============================================================");
    println!("               HOÀN TẤT THỰC NGHIỆM CHƯƠNG 26               ");
    println!("============================================================");
}
```

---

## Bảng tra cứu lỗi biên dịch & Cách khắc phục (Compiler Error Guide)

Dưới đây là các lỗi biên dịch phổ biến nhất liên quan đến quyền sở hữu (ownership), vay mượn (borrow), và kích thước kiểu dữ liệu khi làm việc với Array, Vec và Slice:

| Mã lỗi | Thông báo mẫu từ trình biên dịch | Nguyên nhân cốt lõi | Cách khắc phục nhanh |
|---|---|---|---|
| **E0277** | `the size for values of type '[i32]' cannot be known at compilation time` | Bạn cố gắng truyền một mảng chưa rõ kích thước bằng giá trị `fn handle(arr: [i32])`. Kiểu `[T]` là kiểu kích thước động (DST), không thể nằm trực tiếp trên Stack mà không có con trỏ. | Đổi tham số sang tham chiếu lát cắt `&[i32]` hoặc mảng kích thước cố định `[i32; 10]`. |
| **E0502** | `cannot borrow 'vec' as mutable because it is also borrowed as immutable` | Bạn tạo một lát cắt `let s = &vec[0..2];` rồi sau đó gọi `vec.push(10);` trong khi `s` vẫn đang được sử dụng. Phép `push` có thể khiến vector đổi nhà trên Heap, biến con trỏ `s` thành con trỏ lơ lửng (Dangling Pointer)! Rust ngăn chặn triệt để điều này. | Kết thúc việc sử dụng lát cắt `s` trước khi gọi các hàm làm biến đổi vector như `.push()`, hoặc sao chép dữ liệu ra nếu cần. |
| **E0308** | `mismatched types` (kèm `expected an array with a size of 4, found one with a size of 5`) | Trong Rust, độ dài của mảng tĩnh là một phần của hệ thống kiểu dữ liệu! Mảng 4 phần tử có kiểu dữ liệu hoàn toàn khác mảng 5 phần tử. | Nếu hàm cần nhận mảng có độ dài bất kỳ, hãy đổi kiểu tham số sang lát cắt `&[i32]`. |
| **E0596** | `cannot borrow '...' as mutable, as it is not declared as mutable` | Bạn cố gắng tạo một lát cắt khả biến `&mut arr[..]` từ một mảng hoặc vector khai báo bằng `let` bất biến. | Thêm từ khóa `mut` khi khai báo biến: `let mut arr = ...;`. |

### Ví dụ phân tích lỗi `E0502` và cơ chế bảo vệ của Rust:

```rust
// Đoạn mã lỗi minh họa: Vi phạm an toàn bộ nhớ do vector tái cấp phát
fn e0502_broken() {
    let mut list = vec![1, 2, 3];
    // Tham chiếu first_ref đang giữ con trỏ trỏ vào vùng nhớ Heap hiện tại của vector
    // let first_ref = &list[0]; 
    
    // Thao tác push có thể kích hoạt cấp phát vùng nhớ mới to hơn và hủy vùng nhớ cũ!
    // list.push(4); 
    
    // Nếu dòng này được phép chạy, first_ref sẽ đọc vào vùng nhớ rác đã bị giải phóng!
    // println!("Phần tử đầu: {}", first_ref); // LỖI E0502!
}

// Cách sửa chữa đúng chuẩn: Sử dụng xong lát cắt trước khi biến đổi
fn e0502_correct() {
    let mut list = vec![1, 2, 3];
    
    // Bước 1: Đọc giá trị và sao chép (copy) ra biến độc lập trên Stack
    let first_value = list[0];
    
    // Bước 2: Tự do biến đổi vector mà không lo xung đột con trỏ
    list.push(4);
    
    println!("Phần tử đầu đã sao chép an toàn: {}", first_value);
    println!("Danh sách sau khi thêm mới: {:?}", list);
}
```

---

## Kiểm thử tự động (Automated Tests)

Cấu trúc dữ liệu và thuật toán là nơi kiểm thử tỏ ra hữu ích nhất: một lỗi ở biên (mảng rỗng, một phần tử, giá trị trùng, trường hợp xấu nhất) thường ẩn rất kỹ. Thêm module `#[cfg(test)]` dưới đây vào cuối tệp `main.rs`, rồi chạy `cargo test`. Hãy để ý cách các test nhắm thẳng vào trường hợp biên và vào **bất biến** của cấu trúc dữ liệu, thay vì chỉ thử một ví dụ "đẹp".

```rust
#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn sum_of_slices() {
        assert_eq!(sum_slice(&[10, 20, 30]), 60);
        assert_eq!(sum_slice(&[]), 0);
    }

    #[test]
    fn reverse_in_place_without_allocating() {
        let mut v = vec![1, 2, 3, 4, 5];
        reverse_in_place(&mut v);
        assert_eq!(v, vec![5, 4, 3, 2, 1]);
    }

    #[test]
    fn reverse_twice_is_identity() {
        let original = vec![7, 3, 9, 1];
        let mut v = original.clone();
        reverse_in_place(&mut v);
        reverse_in_place(&mut v);
        assert_eq!(v, original); // đảo hai lần = phép đồng nhất
    }

    #[test]
    fn odd_length_reverse_keeps_middle() {
        let mut v = vec![1, 2, 3];
        reverse_in_place(&mut v);
        assert_eq!(v, vec![3, 2, 1]);
        let mut r = vec![42];
        reverse_in_place(&mut r);
        assert_eq!(r, vec![42]);
    }
}
```

## Tóm tắt chương & Bài tập rèn luyện (Summary & Exercises)

### 4 Điểm cốt lõi cần ghi nhớ:
1. **Lợi thế vùng nhớ liền kề**: Dữ liệu nằm liên tục giúp CPU tải trước dữ liệu vào bộ nhớ đệm (cache) siêu tốc, giảm thiểu tối đa hiện tượng trượt cache (Cache Miss).
2. **Array vs Vector**: Dùng `Array` khi biết trước số lượng phần tử cố định và muốn tiết kiệm tối đa tài nguyên trên Stack; Dùng `Vec` khi dữ liệu co giãn kích thước linh hoạt trên Heap.
3. **Cơ chế nhân đôi dung lượng**: `Vec` tự động nhân đôi sức chứa khi đầy. Hãy tận dụng `Vec::with_capacity(n)` bất cứ khi nào ước tính được quy mô dữ liệu để tránh tái cấp phát nhiều lần.
4. **Sức mạnh của Lát cắt (`&[T]`)**: Luôn ưu tiên nhận tham số hàm dưới dạng `&[T]` thay vì `&Vec<T>`, vì `&[T]` có thể nhận cả Array, Vector, lẫn các lát cắt con mà không đòi hỏi cấp phát hay chuyển quyền sở hữu (ownership).

### Bài tập rèn luyện tự giải:
1. **Bài tập 1 (Phân loại dữ liệu)**:  
   Trong các tình huống sau, cấu trúc dữ liệu nào (`[T; N]`, `Vec<T>`, hay `&[T]`) là lựa chọn tối ưu nhất?
   - a) Lưu trữ tọa độ 3 chiều $(x, y, z)$ của một hạt bụi trong không gian trò chơi vật lý.
   - b) Lưu danh sách các bình luận của người dùng trên một bài đăng mạng xã hội (số lượng bình luận tăng dần theo thời gian).
   - c) Viết hàm kiểm tra một chuỗi số có phải là chuỗi đối xứng (Palindrome) hay không mà không cần nhân bản dữ liệu.
2. **Bài tập 2 (Tìm phần tử lớn nhất bằng Lát cắt)**:  
   Hãy viết một hàm `fn max_of(data: &[i32]) -> Option<i32>` trả về giá trị lớn nhất trong lát cắt. Viết hàm kiểm thử gọi `max_of` lần lượt với một mảng tĩnh `[10, 50, 30]`, một `Vec` động, và một lát cắt rỗng `&[]` để đảm bảo hàm xử lý an toàn không bị hoảng loạn (panic).
3. **Bài tập 3 (Tối ưu hóa dung lượng)**:  
   Viết một đoạn mã tạo một vector chứa các số chẵn từ 2 đến 2000. Đếm số lần vector phải tái cấp phát (sức chứa `capacity()` thay đổi) và số lần địa chỉ con trỏ `as_ptr()` thay đổi trong hai trường hợp:
   - Trường hợp A: Sử dụng `Vec::new()` thông thường.
   - Trường hợp B: Sử dụng `Vec::with_capacity(1000)`.  
   Quan sát và đưa ra nhận xét về hiệu quả bảo toàn vùng nhớ.

---

### Gợi ý & Lời giải

<details>
<summary><b>Bài tập 1 — Gợi ý</b></summary>

Ba câu hỏi: kích thước có biết trước lúc biên dịch không? Có cần lớn lên không? Hàm có cần *sở hữu* dữ liệu không?
</details>

<details>
<summary><b>Bài tập 1 — Lời giải</b></summary>

| | Tình huống | Lựa chọn | Vì sao |
|---|---|---|---|
| a | Toạ độ 3 chiều `(x, y, z)` | **`[f32; 3]`** | Đúng 3 phần tử, biết chắc lúc biên dịch. Nằm trọn trên ngăn xếp, không cấp phát, `Copy` được |
| b | Danh sách bình luận, tăng dần | **`Vec<String>`** | Số lượng không biết trước và lớn lên theo thời gian. Chỉ `Vec` mới `push` được |
| c | Hàm kiểm tra chuỗi đối xứng | **`&[T]`** | Hàm chỉ *đọc*, không cần sở hữu. Nhận lát cắt thì gọi được với **cả** `[T; N]` lẫn `Vec<T>` mà không nhân bản |

**Quy tắc rút ra cho tham số hàm:** nhận `&[T]` chứ đừng nhận `&Vec<T>`. Lát cắt tổng quát hơn — nó nhận được mảng tĩnh, `Vec`, và cả một phần của chúng (`&v[2..5]`). Nhận `&Vec<T>` là tự bó hẹp mình mà không được lợi gì. Clippy có hẳn một cảnh báo cho lỗi này.
</details>

<details>
<summary><b>Bài tập 2 — Gợi ý</b></summary>

`Option` là câu trả lời đúng cho lát cắt có thể rỗng. Thư viện chuẩn đã có `.iter().max()` trả sẵn `Option` — dùng nó thay vì tự viết vòng lặp.
</details>

<details>
<summary><b>Bài tập 2 — Lời giải</b></summary>

```rust
/// Trả `None` cho lát cắt rỗng — KHÔNG panic.
/// Nhận `&[i32]` nên gọi được với mảng tĩnh, Vec, hay một phần của chúng.
pub fn max_of(data: &[i32]) -> Option<i32> {
    data.iter().copied().max()
}

#[test]
fn max_of_works_for_every_source() {
    let array = [10, 50, 30];
    assert_eq!(max_of(&array), Some(50));          // mảng tĩnh

    let v = vec![-5, -1, -99];
    assert_eq!(max_of(&v), Some(-1));             // Vec động

    assert_eq!(max_of(&[]), None);                // RỖNG -> None, không panic
    assert_eq!(max_of(&v[1..]), Some(-1));        // một phần của Vec
}
```

Hai chi tiết:

- **`.copied()` trước `.max()`**: `iter()` cho ra `&i32`, mà ta muốn trả `i32`. Không có `.copied()` thì kiểu trả về thành `Option<&i32>` — vẫn dùng được nhưng buộc người gọi phải bóc tham chiếu.
- **Không có `.unwrap()` nào.** Lát cắt rỗng là trạng thái *hợp lệ*, không phải lỗi lập trình — nên nó phải nằm trong kiểu trả về, đúng tinh thần hàm toàn phần của Chương 13.
</details>

<details>
<summary><b>Bài tập 3 — Gợi ý</b></summary>

Mỗi lần `Vec` hết chỗ, nó xin vùng nhớ lớn hơn — `capacity()` đổi. Đếm số lần `capacity()` đổi là đếm số lần tái cấp phát. `as_ptr()` cho địa chỉ vùng nhớ hiện tại; hãy so hai con số này với nhau — chúng không nhất thiết bằng nhau.
</details>

<details>
<summary><b>Bài tập 3 — Lời giải</b></summary>

```rust
/// Trả về (số lần sức chứa đổi, số lần địa chỉ đổi, sức chứa cuối).
fn count_reallocations(preallocate: bool) -> (usize, usize, usize) {
    let mut v: Vec<u32> = if preallocate {
        Vec::with_capacity(1000) // xin đủ chỗ NGAY TỪ ĐẦU
    } else {
        Vec::new() // để nó tự lớn dần
    };
    let mut old_cap = v.capacity();
    let mut old_ptr = v.as_ptr();
    let mut cap_changes = 0;
    let mut ptr_changes = 0;

    for x in (2..=2000).step_by(2) {
        v.push(x);
        if v.capacity() != old_cap {
            // sức chứa đổi = vừa (tái) cấp phát
            cap_changes += 1;
            old_cap = v.capacity();
        }
        if v.as_ptr() != old_ptr {
            // địa chỉ đổi = dữ liệu đã bị chuyển sang vùng nhớ mới
            ptr_changes += 1;
            old_ptr = v.as_ptr();
        }
    }
    (cap_changes, ptr_changes, v.capacity())
}

fn main() {
    let (a, pa, cap_a) = count_reallocations(false);
    let (b, pb, cap_b) = count_reallocations(true);

    println!("Vec::new()               : {a} lần tái cấp phát, {pa} lần đổi địa chỉ, capacity cuối {cap_a}");
    println!("Vec::with_capacity(1000) : {b} lần tái cấp phát, {pb} lần đổi địa chỉ, capacity cuối {cap_b}");

    assert!(a > b, "biết trước sức chứa thì tái cấp phát ít hơn hẳn");
    assert_eq!(b, 0, "đã đủ chỗ từ đầu nên không phải tái cấp phát lần nào");
    assert!(pa <= a, "địa chỉ chỉ có thể đổi khi có tái cấp phát");
}
```

**Con số thường thấy:** `Vec::new()` tái cấp phát **9 lần** (sức chứa đi 0→4→8→16→…→1024), còn `with_capacity(1000)` **0 lần** — vùng nhớ đã được xin sẵn lúc gọi `with_capacity`, mọi `push` sau đó chỉ ghi vào chỗ trống.

**Cái bẫy của việc đếm bằng `as_ptr()`:** số lần *đổi địa chỉ* thường ÍT hơn 9 — trên Linux/glibc có khi chỉ 1–2 lần. Lý do: `Vec` tái cấp phát bằng `realloc`, và khi ngay sau khối nhớ cũ còn trống, bộ cấp phát **nới rộng tại chỗ** mà không cần chép dữ liệu. Vì vậy đếm `capacity()` mới là cách đo chính xác số lần tái cấp phát; địa chỉ đổi chỉ cho biết lần nào thực sự phải sao chép.

Vì sao vẫn là O(1) khấu hao dù có tái cấp phát: mỗi lần nhân đôi, tổng số phần tử phải sao chép là 1+2+4+…+N < 2N. Chia cho N thao tác `push` ra một hằng số. Nhưng **khấu hao O(1) không có nghĩa là miễn phí** — mỗi lần tái cấp phát là một lần dừng để sao chép, và trong hệ thống độ trễ thấp (Chương 74) đó chính là cái đuôi độ trễ phải tiêu diệt bằng cách cấp phát trước.
</details>
