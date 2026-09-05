# Chương 18: Tính vệ sinh trong Macro, Mẫu lặp lại & Các trường hợp biên (Macro Hygiene, Repetition Patterns & Edge Cases)

## Giới thiệu & Mục tiêu học tập

Ở Chương 17, bạn đã bước đầu khám phá thế giới siêu lập trình với `macro_rules!` và các bộ khớp cú pháp cơ bản. Bạn đã thấy macro có thể tạo ra các từ điển `HashMap` tiện lợi chỉ bằng vài dòng mã ngắn gọn. Nhưng khi xây dựng các thư viện lớn hoặc các công cụ tự động hóa phức tạp, bạn sẽ ngay lập tức đối mặt với những bài toán hóc búa hơn:
- *Làm sao để một macro có thể nhận danh sách tham số lặp đi lặp lại vô số lần, hỗ trợ cả dấu phẩy ở phần tử cuối cùng (`trailing comma`) giống hệt như các cấu trúc chuẩn của Rust?*
- *Làm sao để tạo các ma trận đa chiều 2D, 3D bằng các mẫu lặp lồng nhau?*
- *Điều gì sẽ xảy ra nếu bên trong macro bạn khai báo một biến tạm mang tên `let x = 10;`, và người lập trình bên ngoài cũng đang có một biến `let x = 999;`? Liệu biến của macro có vô tình "đè bẹp" hoặc làm sai lệch giá trị của biến bên ngoài hay không?*

Trong các ngôn ngữ như C hay C++, các tiền xử lý macro (`#define`) khét tiếng vì sự nguy hiểm: Chúng hoạt động như công cụ thay thế chuỗi mù quáng, thường xuyên gây ra lỗi xung đột tên biến ngầm và lỗ hổng bảo mật rò rỉ bộ nhớ đệm (buffer overflow). Nhưng trong Rust, các kỹ sư thiết kế ngôn ngữ đã trang bị một cơ chế bảo vệ tối tân mang tên: **Tính vệ sinh trong Macro (Macro Hygiene)**. Nhờ tính vệ sinh, các biến bên trong macro được cách ly hoàn toàn với thế giới bên ngoài.

Mục tiêu học tập của chương này:
- Hiểu sâu sắc khái niệm **Tính vệ sinh (Macro Hygiene)** và cơ chế gán nhãn ngữ cảnh cú pháp (Syntax Context) của trình biên dịch `rustc`.
- Phân biệt phạm vi vệ sinh: Những gì được bảo vệ (biến cục bộ) và những gì cần cẩn trọng (tên struct, tên trait, đường dẫn thư viện `$crate`).
- Làm chủ toàn diện các toán tử lặp:
  - **`$(...)*`**: Lặp lại từ 0 đến vô số lần.
  - **`$(...),+`**: Lặp lại từ 1 đến vô số lần (ít nhất một phần tử).
  - **`$(...)?`**: Tùy chọn (xuất hiện 0 hoặc 1 lần), đặc biệt là kỹ thuật xử lý dấu phẩy cuối dòng `$(,)?`.
- Thiết kế các cấu trúc lặp lồng nhau (**Nested Repetitions**) cho dữ liệu bảng biểu và ma trận đa chiều.
- Khám phá mẫu thiết kế đệ quy nâng cao: **Bộ nhai thẻ bài (TT Muncher - Token Tree Muncher)**.
- Xử lý các trường hợp biên (Edge Cases): Giới hạn đệ quy `#![recursion_limit]` và quy tắc xuất bản macro qua nhiều crate với `#[macro_export]`.

---

## Hình tượng hóa đời sống (Intuitive Everyday Analogy)

Hãy cùng giải mã Tính vệ sinh và Mẫu lặp lại qua hai hình tượng gần gũi trong đời sống:

```
┌──────────────────────────────────────────────────────────────────────────────────┐
│              HÌNH TƯỢNG ĐỜI SỐNG: PHÒNG THÍ NGHIỆM VÔ TRÙNG VÀ ĐÀN XẾP           │
├────────────────────────────────────────┬─────────────────────────────────────────┤
│     PHÒNG CÁCH LY VÔ TRÙNG             │         CHIẾC ĐÀN XẾP ACCORDION         │
│          (Macro Hygiene)               │           (Repetition Patterns)         │
│                                        │                                         │
│ - Các bác sĩ mặc đồ bảo hộ kín mít     │ - Chiếc đàn có các nếp gấp co giãn      │
│ - Mọi dao mổ, ống nghiệm mang nhãn     │ - Bạn kéo nhẹ 1 nhịp:                   │
│   riêng bên trong phòng thí nghiệm     │   Đàn phát ra 1 nốt nhạc ($(..),+)      │
│ - Tuyệt đối không làm lây nhiễm vi     │ - Bạn kéo rộng 10 nhịp:                 │
│   khuẩn ra hành lang bên ngoài!        │   Đàn phát ra 10 nốt nhạc!              │
│ - Khách đi ngoài hành lang không sợ    │ - Bạn buông tay không kéo:              │
│   bị dụng cụ trong phòng đâm trúng!    │   Đàn êm ru, 0 nốt nhạc ($(..)*)        │
│ -> Tên biến trong macro được vô trùng! │ -> Co giãn linh hoạt theo dữ liệu nạp!  │
└────────────────────────────────────────┴─────────────────────────────────────────┘
```

### 1. Phòng thí nghiệm y tế vô trùng (Tính vệ sinh - Macro Hygiene)
- Hãy quan sát phòng cách ly vô trùng (Cleanroom) của một bệnh viện:
  - Bác sĩ phẫu thuật bên trong phòng có một chiếc khay nhôm đựng dao mổ dán nhãn `khay_tam`.
  - Ở ngoài quầy lễ tân bệnh viện, cô y tá tiếp đón cũng có một chiếc khay đựng hồ sơ bệnh án dán nhãn `khay_tam`.
  - Mặc dù hai chiếc khay có cùng tên gọi là `khay_tam`, nhưng chúng nằm ở hai không gian tách biệt hoàn toàn bởi lớp cửa kính cách ly. Bác sĩ dùng chiếc khay trong phòng mổ không bao giờ làm đổ hay xáo trộn tài liệu trên chiếc khay ngoài quầy lễ tân!
- Đây chính là **Tính vệ sinh trong Rust**: Trình biên dịch tự động "nhuộm màu" (Syntax Context) các biến bên trong macro. Một biến `x` do macro sinh ra và một biến `x` của bạn ngoài hàm `main` là hai thực thể hoàn toàn độc lập, không thể xung đột!

### 2. Chiếc đàn xếp Accordion (Mẫu lặp lại - Repetitions)
- Hãy quan sát người nghệ sĩ chơi chiếc đàn xếp accordion:
  - Dây đàn và hộp hơi có thể ép sát lại không còn khoảng trống (lặp 0 lần - `*`).
  - Khi nghệ sĩ kéo dãn đàn ra, hàng chục nếp gấp mở rộng linh hoạt theo từng giai điệu (lặp nhiều lần - `+`).
  - Cho dù bản nhạc có 1 nốt, 5 nốt hay 50 nốt, thân đàn vẫn co giãn hoàn hảo để đáp ứng. Cú pháp lặp `$(...)*` trong Rust hoạt động y hệt như những nếp gấp đàn xếp đó!

---

## Khái niệm & Cơ chế kỹ thuật chuyên sâu (In-Depth Technical Mechanics)

### 1. Cơ chế Tính vệ sinh (Hygiene Mechanics) & Nhãn Ngữ cảnh Cú pháp

Làm thế nào mà `rustc` bảo vệ các biến không bị xung đột tên?
Khi trình biên dịch mở rộng một macro, nó không đơn thuần ghép chuỗi ký tự. Nó gán cho mỗi định danh (Identifier) một chiếc thẻ bài định danh nội bộ gọi là **SyntaxContext (Ngữ cảnh cú pháp)**:

```rust
// Mã người dùng viết:
let ket_qua = 100;

macro_rules! tinh_toan {
    () => {
        let ket_qua = 999; // Biến này mang SyntaxContext của Macro!
        println!("Trong macro: {}", ket_qua);
    };
}

tinh_toan!();
println!("Ngoài hàm main: {}", ket_qua);
```

Khi chạy chương trình:
- Dòng in trong macro xuất hiện: `Trong macro: 999`
- Dòng in ngoài hàm main xuất hiện: `Ngoài hàm main: 100`
Biến `ket_qua` bên ngoài hàm `main` vẫn giữ nguyên giá trị `100` trọn vẹn! Trình biên dịch xem chúng là `ket_qua#Context1` và `ket_qua#Context2`.

*Lưu ý về giới hạn của tính vệ sinh*: Tính vệ sinh trong `macro_rules!` bảo vệ biến cục bộ, nhưng **không tự động bảo vệ đường dẫn mô-đun hoặc tên kiểu dữ liệu**. Vì vậy, khi viết thư viện, luôn dùng từ khóa `$crate::` để tham chiếu đến các mục bên trong chính crate của bạn (ví dụ: `$crate::collections::HashMap` thay vì viết trần `HashMap`), phòng trường hợp người dùng quên `use std::collections::HashMap;`.

### 2. Cú pháp Lặp lại Toàn diện: `*`, `+`, và `?`

Cú pháp lặp trong `macro_rules!` có dạng chuẩn:
`$( <khuôn_mẫu> )<ký_tự_ngăn_cách><toán_tử_lặp>`

```
       ┌─── Bắt đầu khối lặp
       │   ┌─── Khuôn mẫu cần lặp
       │   │           ┌─── Đóng khối lặp
       │   │           │ ┌─── Ký tự ngăn cách (dấu phẩy, chấm phẩy...)
       │   │           │ │ ┌─── Toán tử lặp (*, +, ?)
       ▼   ▼           ▼ ▼ ▼
       $( $item:expr ) , *
```

1. **`$( $x:expr ),*`**: Nhận danh sách các biểu thức cách nhau bởi dấu phẩy, có thể rỗng (0 phần tử, 1 phần tử, hoặc nhiều phần tử).
2. **`$( $x:expr ),+`**: Đòi hỏi **ít nhất 1 phần tử** trở lên. Nếu truyền rỗng `()`, macro sẽ báo lỗi biên dịch ngay.
3. **`$(,)?`**: Kỹ thuật kinh điển để xử lý dấu phẩy tùy chọn ở phần tử cuối cùng:
   ```rust
   ( $( $x:expr ),* $(,)? )
   ```
   Nhờ đoạn `$(,)?` này, người dùng có thể viết `my_macro!(1, 2, 3)` hoặc `my_macro!(1, 2, 3,)` (có dấu phẩy cuối) đều được chấp nhận hợp lệ!

### 3. Mẫu lặp lồng nhau (Nested Repetitions)

Khi bạn muốn biểu diễn các cấu trúc đa chiều như ma trận hàng và cột (2D Grid) hoặc danh sách các bảng dữ liệu:

```rust
macro_rules! tao_ma_tran {
    ( $( [ $( $gia_tri:expr ),* $(,)? ] ),* $(,)? ) => {
        vec![
            $(
                vec![ $( $gia_tri ),* ],
            )*
        ]
    };
}
```
Ở đây có hai cấp lặp:
- Cấp ngoài: lặp qua từng hàng `[ ... ]`.
- Cấp trong: lặp qua từng phần tử `$gia_tri` trong hàng đó.

### 4. Mẫu thiết kế Đệ quy: "Bộ nhai thẻ bài" (TT Muncher)

**TT Muncher (Token Tree Muncher)** là mẫu thiết kế đỉnh cao của Macro khai báo. Nó hoạt động tương tự như thuật toán đệ quy trên danh sách liên kết:
- Nhánh cơ sở (Base case): Khi danh sách thẻ bài rỗng -> Dừng đệ quy và trả về kết quả.
- Nhánh đệ quy (Recursive step): Nhặt lấy một thẻ bài ở đầu hàng (`head`), xử lý nó, sau đó gọi lại chính macro đó với phần còn lại ở đuôi (`tail`).

```rust
macro_rules! dem_phan_tu {
    // Nhánh cơ sở: Hết phần tử -> trả về 0
    () => { 0usize };
    // Nhánh đệ quy: Nhặt 1 phần tử $dau, gọi đệ quy cho phần $duoi
    ( $dau:tt $( $duoi:tt )* ) => {
        1usize + dem_phan_tu!( $( $duoi )* )
    };
}
```

---

## Mã nguồn minh họa thực chiến (Idiomatic Runnable Rust Blueprint)

Dưới đây là một chương trình hoàn chỉnh minh họa toàn diện:
1. Chứng minh **Tính vệ sinh (Macro Hygiene)** bảo vệ biến an toàn tuyệt đối.
2. Macro xây dựng **Ma trận dữ liệu 2D (`tao_ma_tran!`)** với mẫu lặp lồng nhau và xử lý dấu phẩy cuối.
3. Macro đệ quy **TT Muncher (`tinh_bieu_thuc_chuoi!`)** tính toán chuỗi phép toán từ trái sang phải.

```rust
// Tệp: src/main.rs
// Chương trình thực chiến làm chủ Tính vệ sinh, Mẫu lặp lại & Đệ quy Macro trong Rust

// ============================================================================
// 1. MACRO CHỨNG MINH TÍNH VỆ SINH (MACRO HYGIENE)
// ============================================================================

macro_rules! phep_tinh_noi_bo {
    ( $dau_vao:expr ) => {
        {
            // Khai báo biến tạm mang tên 'gia_tri_tam' bên trong macro
            let gia_tri_tam = $dau_vao * 2;
            println!("  [Trong Macro] gia_tri_tam = {}", gia_tri_tam);
            gia_tri_tam + 5
        }
    };
}

// ============================================================================
// 2. MACRO MA TRẬN 2D VỚI CÚ PHÁP LẶP LỒNG NHAU: tao_ma_tran!
// ============================================================================

/// Macro tạo Vector lồng nhau (Ma trận 2 chiều) hỗ trợ dấu phẩy tùy chọn ở mọi cấp
macro_rules! tao_ma_tran {
    (
        $(
            [ $( $phan_tu:expr ),* $(,)? ]
        ),*
        $(,)?
    ) => {
        vec![
            $(
                vec![ $( $phan_tu ),* ],
            )*
        ]
    };
}

// ============================================================================
// 3. MACRO ĐỆ QUY TT MUNCHER: tinh_bieu_thuc_chuoi!
// ============================================================================

/// Macro đệ quy phân tích chuỗi phép toán từ trái sang phải
macro_rules! tinh_bieu_thuc_chuoi {
    // Nhánh dừng cơ sở: Chỉ còn lại duy nhất một giá trị
    ( $gia_tri:expr ) => {
        $gia_tri
    };

    // Nhánh đệ quy phép cộng: (x + y + rest...) -> tinh_bieu_thuc_chuoi!((x + y) + rest...)
    ( $x:expr, +, $y:expr $(, $duoi:tt )* ) => {
        tinh_bieu_thuc_chuoi!( ($x + $y) $(, $duoi )* )
    };

    // Nhánh đệ quy phép nhân: (x * y * rest...)
    ( $x:expr, *, $y:expr $(, $duoi:tt )* ) => {
        tinh_bieu_thuc_chuoi!( ($x * $y) $(, $duoi )* )
    };

    // Nhánh đệ quy phép trừ: (x - y - rest...)
    ( $x:expr, -, $y:expr $(, $duoi:tt )* ) => {
        tinh_bieu_thuc_chuoi!( ($x - $y) $(, $duoi )* )
    };
}

// ============================================================================
// CHƯƠNG TRÌNH THỰC THI CHÍNH
// ============================================================================

fn main() {
    println!("============================================================");
    println!("     NÂNG CAO METAPROGRAMMING: HYGIENE, REPETITIONS & TT    ");
    println!("============================================================");

    // ------------------------------------------------------------------------
    // TÌNH HUỐNG 1: Kiểm chứng Tính vệ sinh không làm ô nhiễm biến ngoài
    // ------------------------------------------------------------------------
    println!("\n1. Kiểm chứng Tính vệ sinh của Macro (Macro Hygiene):");
    let gia_tri_tam = 7777; // Biến trùng tên ở phạm vi hàm main
    println!("Trước khi gọi macro: gia_tri_tam = {}", gia_tri_tam);

    let ket_qua_macro = phep_tinh_noi_bo!(10);
    println!("Kết quả trả về từ macro: {}", ket_qua_macro);

    // Xác nhận biến gia_tri_tam ngoài hàm main KHÔNG HỀ BỊ THAY ĐỔI!
    println!("Sau khi gọi macro: gia_tri_tam = {}", gia_tri_tam);
    assert_eq!(gia_tri_tam, 7777);
    println!("-> KẾT LUẬN: Biến trong macro được cách ly vô trùng tuyệt đối!");

    // ------------------------------------------------------------------------
    // TÌNH HUỐNG 2: Xây dựng Ma trận dữ liệu 2D với Mẫu lặp lồng nhau
    // ------------------------------------------------------------------------
    println!("\n2. Khởi tạo Bảng dữ liệu ma trận 2D qua macro lồng nhau:");
    let ma_tran_diem = tao_ma_tran![
        [10, 20, 30,], // Dấu phẩy ở cuối hàng hợp lệ
        [40, 50, 60],
        [70, 80, 90],  // Dấu phẩy ở cuối khối ma trận hợp lệ
    ];

    for (so_hang, hang) in ma_tran_diem.iter().enumerate() {
        println!("  Hàng #{}: {:?}", so_hang + 1, hang);
    }
    assert_eq!(ma_tran_diem[1][1], 50);

    // ------------------------------------------------------------------------
    // TÌNH HUỐNG 3: Vận hành TT Muncher phân tích chuỗi phép tính đệ quy
    // ------------------------------------------------------------------------
    println!("\n3. Vận hành Bộ nhai thẻ bài TT Muncher đệ quy:");
    // Tính toán: (((10 + 5) * 2) - 6) = 15 * 2 - 6 = 30 - 6 = 24
    let ket_qua_tinh = tinh_bieu_thuc_chuoi!(10, +, 5, *, 2, -, 6);
    println!("Kết quả phân tích đệ quy (10 + 5) * 2 - 6 = {}", ket_qua_tinh);
    assert_eq!(ket_qua_tinh, 24);

    println!("\n============================================================");
    println!("     XÁC THỰC CÁC MẪU MACRO NÂNG CAO HOÀN THÀNH THÀNH CÔNG  ");
    println!("============================================================");
}
```

---

## Bảng tra cứu lỗi biên dịch & Cách khắc phục (Compiler Error Guide)

Các lỗi biên dịch thường gặp khi làm việc với mẫu lặp và đệ quy trong Macro:

| Mã lỗi | Thông báo mẫu từ trình biên dịch | Nguyên nhân cốt lõi | Cách khắc phục nhanh |
|---|---|---|---|
| **Giới hạn đệ quy** | `recursion limit reached while expanding the macro` | Macro đệ quy (như TT Muncher) gọi lồng nhau vượt quá giới hạn mặc định của Rust (thường là 128 tầng đệ quy). | Kiểm tra nhánh dừng cơ sở xem có bị thiếu không. Nếu logic đệ quy thực sự cần xử lý tập dữ liệu lớn, thêm thuộc tính `#![recursion_limit = "256"]` ở đầu tệp gốc crate (`main.rs` hoặc `lib.rs`). |
| **Lỗi lặp** | `variable '...' is still repeating at this depth` | Bạn khai báo một biến nằm trong khối lặp `$( $x:expr ),*`, nhưng khi bung mã ở vế phải bạn lại không đặt nó bên trong khối `$( ... )*` tương ứng. | Đảm bảo mọi biến bắt giữ trong khối lặp đều được mở rộng bên trong khối lặp ở thân macro. |
| **Lỗi cú pháp** | `unexpected end of macro invocation` | Macro đòi hỏi ít nhất một phần tử (`+`) hoặc một ký tự kết thúc, nhưng người gọi lại đóng ngoặc quá sớm. | Kiểm tra lại các điều kiện dừng hoặc chuyển từ toán tử `+` sang `*` nếu cho phép trường hợp rỗng. |
| **E0408** | `variable '...' from pattern #1 is not bound in pattern #2` | Khi viết macro có nhiều nhánh so khớp rẽ nhánh, một nhánh không gán giá trị cho định danh được yêu cầu. | Đảm bảo tính nhất quán của các định danh ở tất cả các nhánh tương đương. |

### Phân tích lỗi thực tế: Quên khối lặp ở thân mở rộng

```rust
// Đoạn mã lỗi minh họa:
macro_rules! in_sai_lap {
    ( $( $item:expr ),* ) => {
        // LỖI: variable 'item' is still repeating at this depth!
        // println!("{}", $item); 
    };
}

// Cách sửa chữa đúng: Đặt $item vào bên trong khối $( ... )*
macro_rules! in_dung_lap {
    ( $( $item:expr ),* ) => {
        $(
            println!("Phần tử: {}", $item);
        )*
    };
}
```

---

## Tóm tắt chương & Bài tập rèn luyện (Summary & Exercises)

### 4 Điểm cốt lõi cần ghi nhớ:
1. **Tính vệ sinh (Macro Hygiene)**: Tự động cô lập các biến cục bộ bên trong macro, loại bỏ hoàn toàn nguy cơ xung đột tên biến ngầm với người gọi.
2. **Bộ ba toán tử lặp**:
   - `*`: 0 hoặc nhiều lần.
   - `+`: Ít nhất 1 lần trở lên.
   - `?`: Tùy chọn 0 hoặc 1 lần (công thức vàng để hỗ trợ dấu phẩy cuối `$(,)?`).
3. **Mẫu lặp lồng nhau**: Cho phép mô hình hóa các cấu trúc dữ liệu nhiều chiều (Vector trong Vector) một cách tự nhiên và ngắn gọn.
4. **TT Muncher Đệ quy**: Biến `macro_rules!` thành một cỗ máy phân tích ngôn ngữ đặc thù (DSL) mạnh mẽ bằng cách duyệt qua từng thẻ bài cú pháp.

### Bài tập rèn luyện tự giải:
1. **Bài tập 1 (Macro Tính Tổng Đa năng)**:  
   Viết một macro mang tên `tong_cong!($( $x:expr ),* $(,)?)` có thể nhận số lượng tham số tùy ý cách nhau bằng dấu phẩy, hỗ trợ dấu phẩy ở cuối, và trả về tổng của tất cả các số đó. Kiểm tra với:
   - `tong_cong!()` (trả về 0)
   - `tong_cong!(5, 10, 15,)` (trả về 30).

2. **Bài tập 2 (Macro Đếm Số lượng Đối số)**:  
   Sử dụng cú pháp lặp `$(...)*` để tạo một macro `dem_so_luong!( $( $phan_tu:expr ),* )` trả về số lượng các tham số được truyền vào dưới dạng `usize` mà không cần duyệt mảng lúc chạy. *(Gợi ý: Mở rộng thành một mảng các số 1 `[ $( { let _ = &$phan_tu; 1usize } ),* ].len()`)*.

3. **Bài tập 3 (Xử lý Giới hạn Đệ quy)**:  
   Viết một macro TT Muncher in ra từng ký tự của một chuỗi thẻ bài. Điều gì xảy ra nếu bạn truyền vào một dãy 200 phần tử? Hãy thực hành thêm `#![recursion_limit = "256"]` để quan sát cách trình biên dịch mở rộng ngưỡng chịu tải.
