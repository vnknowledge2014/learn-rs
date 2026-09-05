# Chương 21: Khái niệm Siêu lập trình: Khi code tự động viết code (Declarative Macros: macro_rules! & Syntax Matchers)

## Giới thiệu & Mục tiêu học tập

Chào mừng bạn bước vào **Chủ đề 4: Siêu lập trình (Meta Programming)** — cảnh giới đỉnh cao của nghệ thuật lập trình trong Rust! Cho đến lúc này, bạn đã quen thuộc với việc viết mã nguồn để ra lệnh cho máy tính xử lý các con số, chuỗi ký tự, cấu trúc dữ liệu và các giao ước trait. Tất cả những thao tác đó đều xoay quanh việc: **Code xử lý Dữ liệu (Code processes Data)**.

Tuy nhiên, có bao giờ bạn tự hỏi:
- *Làm sao macro `println!("Xin chào {}", ten)` có thể nhận số lượng tham số tùy ý (1 tham số, 3 tham số hay 10 tham số đều được), trong khi hàm thông thường `fn` trong Rust luôn bắt buộc số lượng tham số cố định?*
- *Làm sao macro `vec![1, 2, 3]` có thể tự động tạo ra một `Vec` đã được nạp sẵn các giá trị ban đầu chỉ bằng một dòng lệnh ngắn ngủi?*
- *Liệu chúng ta có thể viết ra những đoạn mã có khả năng... **tự động viết ra mã nguồn khác** để giải phóng lập trình viên khỏi hàng trăm dòng code lặp đi lặp lại nhàm chán (boilerplate code)?*

Câu trả lời nằm ở **Siêu lập trình (Metaprogramming)** và vũ khí cốt lõi đầu tiên của nó: **Macro khai báo (`macro_rules!`)**. Trong Rust, macro không phải là công cụ tìm-và-thay-thế chuỗi thô sơ như tiền xử lý `#define` của C/C++ (vốn rất dễ gây lỗi tràn số và xung đột tên biến). Macro trong Rust là một phần chính thức của trình biên dịch `rustc`, hoạt động trực tiếp trên các thẻ bài cú pháp (syntax tokens) và cây cú pháp trừu tượng (AST), đảm bảo an toàn tuyệt đối về mặt kiểu dữ liệu và bộ nhớ.

Mục tiêu học tập của chương này:
- Hiểu rõ bản chất **Siêu lập trình (Metaprogramming)**: Viết mã nguồn để sinh ra mã nguồn tại thời điểm biên dịch (Compile-time).
- Phân biệt sự khác nhau cốt lõi giữa **Hàm (`fn`)** thực thi lúc chạy (Runtime) và **Macro (`!`)** mở rộng lúc biên dịch (Compile-time).
- Nhận biết hai phân loại Macro lớn trong Rust: **Macro khai báo (`macro_rules!`)** và **Macro thủ tục (Procedural Macros)**.
- Làm chủ cú pháp định nghĩa `macro_rules!` và các **Bộ khớp cú pháp (Syntax Matchers / Designators)**: `$expr`, `$ident`, `$ty`, `$stmt`, `$path`, `$block`, `$literal`.
- Sử dụng các macro tích hợp sẵn hỗ trợ gỡ lỗi: `stringify!`, `file!`, `line!`.
- Biết cách dùng công cụ `cargo expand` để soi tận mắt đoạn mã máy tính thực sự nhìn thấy sau khi macro bung ra.

---

## Hình tượng hóa đời sống (Intuitive Everyday Analogy)

Hãy cùng giải mã khái niệm Siêu lập trình qua hai hình ảnh vô cùng sinh động trong đời thực:

```
┌──────────────────────────────────────────────────────────────────────────────────┐
│              HÌNH TƯỢNG ĐỜI SỐNG: NGƯỜI THỢ THỦ CÔNG VS MÁY IN KHUÔN ĐÚC         │
├────────────────────────────────────────┬─────────────────────────────────────────┤
│        HÀM THÔNG THƯỜNG (FN)           │          MACRO TRONG RUST (!)           │
│     "Người thợ làm bánh thủ công"      │      "Chiếc máy dập khuôn tự động"      │
│                                        │                                         │
│ - Ngồi trực tiếp tại quầy lúc nửa đêm  │ - Trước khi mở cửa tiệm (Compile-time): │
│ - Khi có khách gọi món (Runtime):      │   Chiếc máy dập tự động dập sẵn 1.000   │
│   Mới bắt đầu đập trứng, nhào bột,     │   vỏ bánh hoàn hảo y như đúc!           │
│   nướng bánh thủ công từng chiếc một.  │ - Khi khách đến mua lúc nửa đêm:        │
│ - Tốn công sức và thời gian lúc chạy!  │   Chỉ việc lấy trao tay ngay lập tức!   │
│                                        │ - Tốc độ ánh sáng, không trễ 1 giây nào!│
└────────────────────────────────────────┴─────────────────────────────────────────┘
```

### 1. Thợ làm bánh thủ công vs Máy dập khuôn công nghiệp
- **Hàm thông thường (`fn`)**: Giống như người thợ làm bánh thủ công ngồi trực trong tiệm bánh:
  - Khi chương trình đang chạy trên máy chủ (`Runtime`), có yêu cầu gửi tới, CPU mới bắt đầu nhảy vào hàm, thực hiện tuần tự từng phép tính, tiêu tốn xung nhịp CPU và chiếm dụng ngăn xếp bộ nhớ (Stack).
- **Macro (`macro_rules!`)**: Giống như chiếc máy dập khuôn cơ khí được lập trình sẵn trước giờ mở cửa:
  - Tại thời điểm biên dịch (`Compile-time`), chiếc máy dập đọc bản vẽ thiết kế của bạn và **dập sẵn toàn bộ mã nguồn cần thiết ra văn bản**.
  - Khi chương trình thực sự chạy, toàn bộ mã đó đã được biên dịch thành mã máy tối ưu hóa cao nhất. Không có chi phí gọi hàm, không có độ trễ lúc chạy!

### 2. Trò chơi Điền từ vào chỗ trống (Mad Libs)
Hãy nhớ lại trò chơi điền từ vào chỗ trống thời thơ ấu:
- Bạn có một câu chuyện mẫu: *"Hôm nay, bạn `[TÊN]` đã đi đến `[ĐỊA_DANH]` để mua `[SỐ_LƯỢNG]` quả `[TRÁI_CÂY]`."*
- Các ô vuông `[TÊN]`, `[ĐỊA_DANH]`, `[SỐ_LƯỢNG]` chính là các **Bộ khớp cú pháp (Matchers)** trong Macro của Rust!
- Khi bạn điền từ vào:
  - Điền: An, Chợ Bến Thành, 5, Xoài.
  - Macro lập tức ráp nối thành một câu hoàn chỉnh: *"Hôm nay, bạn An đã đi đến Chợ Bến Thành để mua 5 quả Xoài."*

---

## Khái niệm & Cơ chế kỹ thuật chuyên sâu (In-Depth Technical Mechanics)

### 1. Vòng đời Biên dịch: Macro diễn ra ở đâu trong `rustc`?

Để hiểu tại sao Macro lại có sức mạnh phi thường, hãy nhìn vào quy trình làm việc của trình biên dịch `rustc`:

```
Mã nguồn thô (.rs)
      │
      ▼
[1. Bộ phân tích từ vựng (Lexer)] ──► Biến văn bản thành dòng Thẻ bài (TokenStream)
      │
      ▼
[2. Bộ phân tích cú pháp (Parser)] ──► Dựng Cây cú pháp trừu tượng (AST)
      │
      ▼
[3. Mở rộng Macro (Macro Expansion)] ◄─── MACRO HOẠT ĐỘNG TẠI ĐÂY!
      │                                   (Bung code mới ngay trên AST,
      │                                    trước khi kiểm tra kiểu)
      ▼
      │
      ▼
[4. Kiểm tra kiểu & Vay mượn (Type & Borrow Checker)] ──► Kiểm tra quyền sở hữu (ownership)
      │
      ▼
[5. Trình sinh mã LLVM] ──► Tệp thực thi nhị phân (.exe / ELF)
```

Vì Macro được mở rộng ở **Giai đoạn 3** — sau khi mã nguồn đã được dựng thành cây cú pháp, nhưng **trước** khi trình biên dịch kiểm tra kiểu dữ liệu và quyền sở hữu:
1. Macro có thể sinh ra các định nghĩa hàm mới, struct mới, hoặc cài đặt trait mới mà các dòng mã phía sau có thể sử dụng bình thường.
2. Macro không bị ràng buộc bởi kiểu dữ liệu tại thời điểm viết, cho phép bạn truyền vào tên biến, tên kiểu, hoặc cả một khối lệnh.
3. Toàn bộ mã do macro sinh ra vẫn phải bước qua bước kiểm tra nghiêm ngặt của Bộ kiểm tra mượn (Borrow Checker) ở Giai đoạn 4, đảm bảo an toàn tuyệt đối, không thể gây lỗi tràn bộ nhớ đệm (buffer) hay rò rỉ ô nhớ!

### 2. Cấu trúc Cú pháp của `macro_rules!`

Một macro khai báo được định nghĩa bằng cú pháp so khớp khuôn mẫu (Pattern Matching) tương tự như lệnh `match`:

```rust
macro_rules! ten_macro {
    // Nhánh 1: So khớp với khuôn mẫu A
    ( khuôn_mau_A ) => {
        // Đoạn mã sinh ra tương ứng
    };
    // Nhánh 2: So khớp với khuôn mẫu B
    ( khuôn_mau_B ) => {
        // Đoạn mã sinh ra tương ứng
    };
}
```

### 3. Bảng tra cứu các Bộ khớp cú pháp (Syntax Designators)

Mỗi vị trí điền dữ liệu trong macro đều bắt đầu bằng dấu đô la `$`, theo sau là tên định danh và **Bộ chỉ định cú pháp (Designator)**:

| Bộ chỉ định | Tên tiếng Anh | Ý nghĩa trong cú pháp Rust | Ví dụ thực tế |
|---|---|---|---|
| **`$e:expr`** | Expression | Bất kỳ biểu thức nào sinh ra giá trị | `1 + 2`, `x * 5`, `String::new()` |
| **`$i:ident`** | Identifier | Tên định danh (tên biến, tên hàm, tên struct) | `so_luong`, `NguoiDung`, `tinh_tong` |
| **`$t:ty`** | Type | Một kiểu dữ liệu hợp lệ trong Rust | `i32`, `String`, `Vec<u8>`, `&str` |
| **`$s:stmt`** | Statement | Một câu lệnh (thường kết thúc bằng dấu `;`) | `let x = 10;`, `dem += 1;` |
| **`$p:path`** | Path | Đường dẫn định danh mô-đun hoặc kiểu dữ liệu | `std::collections::HashMap`, `crate::api` |
| **`$b:block`** | Block | Một khối mã được bao bọc bởi cặp ngoặc `{}` | `{ let a = 1; a + 2 }` |
| **`$lit:literal`** | Literal | Một hằng số nguyên bản | `42`, `"Xin chào"`, `true`, `'🦀'` |
| **`$tt:tt`** | Token Tree | Một thẻ bài đơn lẻ hoặc một nhóm ngoặc `()` `[]` `{}` | Bất kỳ ký tự cú pháp hợp lệ nào |

### 4. Công cụ Ma thuật: `stringify!` và `cargo expand`

- **`stringify!($e)`**: Chuyển đổi trực tiếp đoạn mã nguồn thành một chuỗi ký tự `&str` ngay lúc biên dịch mà không cần tính toán giá trị của nó.
  - Ví dụ: `stringify!(1 + 1)` sẽ sinh ra chuỗi `"1 + 1"`, chứ không phải `"2"`!
- **`cargo expand`**: Công cụ dòng lệnh cài đặt qua `cargo install cargo-expand`. Nó cho phép bạn in toàn bộ mã nguồn của dự án ra màn hình sau khi tất cả các macro đã được bung ra hoàn toàn. Đây là chiếc "kính lúp soi macro" không thể thiếu của mọi lập trình viên Rust chuyên nghiệp.

---

## Mã nguồn minh họa thực chiến (Idiomatic Runnable Rust Blueprint)

Chương trình hoàn chỉnh dưới đây xây dựng một **Bộ công cụ Siêu lập trình Tiện ích (Metaprogramming Utility Toolkit)** gồm 3 macro thực chiến:
1. `tao_ban_do!`: Macro tạo nhanh `HashMap` theo phong cách từ điển JSON trực quan.
2. `kiem_toan_bien!`: Macro soi sáng thông tin nội bộ của biến (tên biến, giá trị, tệp tin, dòng mã).
3. `do_luong_thoi_gian!`: Macro bọc một khối lệnh bất kỳ để đo thời gian thực thi của nó.

```rust
// Tệp: src/main.rs
// Chương trình thực chiến làm chủ macro_rules! và Bộ khớp cú pháp trong Rust

use std::collections::HashMap;
use std::time::Instant;

// ============================================================================
// 1. MACRO TẠO NHANH HASHMAP VỚI CÚ PHÁP TỪ ĐIỂN: tao_ban_do!
// ============================================================================

/// Macro nhận vào các cặp $khoa => $gia_tri cách nhau bởi dấu phẩy
/// Hỗ trợ dấu phẩy tùy chọn ở cuối cùng $(,)?
macro_rules! tao_ban_do {
    // Nhánh xử lý: $( $khoa:expr => $gia_tri:expr ),*
    ( $( $khoa:expr => $gia_tri:expr ),* $(,)? ) => {
        {
            let mut ban_do = HashMap::new();
            $(
                ban_do.insert($khoa, $gia_tri);
            )*
            ban_do
        }
    };
}

// ============================================================================
// 2. MACRO SOI SÁNG VÀ KIỂM TOÁN BIẾN: kiem_toan_bien!
// ============================================================================

/// Macro sử dụng $i:ident và $e:expr kết hợp với stringify!, file!, line!
/// Giúp lập trình viên gỡ lỗi với thông tin vị trí mã nguồn cực kỳ chi tiết
macro_rules! kiem_toan_bien {
    ( $ten_bien:ident ) => {
        println!(
            "[KIỂM TOÁN] Biến `{}` = {:?} (Tại tệp: {}, Dòng: {})",
            stringify!($ten_bien),
            $ten_bien,
            file!(),
            line!()
        );
    };
    ( $nhan_dan:expr, $bieu_thuc:expr ) => {
        println!(
            "[KIỂM TOÁN: {}] Biểu thức `{}` có giá trị = {:?} (Dòng: {})",
            $nhan_dan,
            stringify!($bieu_thuc),
            $bieu_thuc,
            line!()
        );
    };
}

// ============================================================================
// 3. MACRO ĐO THỜI GIAN KHỐI LỆNH: do_luong_thoi_gian!
// ============================================================================

/// Macro nhận một nhãn mô tả $ten:expr và một khối mã $khoi:block
/// Trả về trực tiếp kết quả của khối mã đó!
macro_rules! do_luong_thoi_gian {
    ( $ten:expr, $khoi:block ) => {
        {
            println!(">>> [BẮT ĐẦU ĐO] {}", $ten);
            let bat_dau = Instant::now();
            let ket_qua = $khoi; // Thực thi khối lệnh
            let thoi_gian = bat_dau.elapsed();
            println!(">>> [KẾT THÚC] {} hoàn thành trong: {:?}", $ten, thoi_gian);
            ket_qua // Trả kết quả của khối lệnh về phía người gọi
        }
    };
}

// ============================================================================
// CHƯƠNG TRÌNH THỰC THI CHÍNH
// ============================================================================

fn main() {
    println!("============================================================");
    println!("     BỘ CÔNG CỤ SIÊU LẬP TRÌNH: DECLARATIVE MACRO RULES     ");
    println!("============================================================");

    // ------------------------------------------------------------------------
    // TÌNH HUỐNG 1: Sử dụng macro tao_ban_do! tạo cấu hình hệ thống
    // ------------------------------------------------------------------------
    println!("\n1. Khởi tạo Bản đồ thông số máy chủ bằng cú pháp trực quan:");
    let thong_so_may_chu = tao_ban_do! {
        "cong_mang" => "8080",
        "dia_chi_ip" => "192.168.1.100",
        "moi_truong" => "SanXuat",
        "trang_thai" => "KichHoat", // Hỗ trợ dấu phẩy ở phần tử cuối cùng!
    };

    for (khoa, gia_tri) in &thong_so_may_chu {
        println!("  - Tham số `{}`: {}", khoa, gia_tri);
    }

    // ------------------------------------------------------------------------
    // TÌNH HUỐNG 2: Sử dụng macro kiem_toan_bien! để soi dữ liệu
    // ------------------------------------------------------------------------
    println!("\n2. Soi sáng biến số và biểu thức bằng siêu lập trình:");
    let diem_trung_binh = 8.75;
    let danh_sach_lop = vec!["An", "Bình", "Cường"];

    // Gỡ lỗi biến đơn lẻ qua $ident
    kiem_toan_bien!(diem_trung_binh);
    kiem_toan_bien!(danh_sach_lop);

    // Gỡ lỗi biểu thức phức tạp qua $expr
    kiem_toan_bien!("Tính toán điểm cộng", diem_trung_binh + 1.25);

    // ------------------------------------------------------------------------
    // TÌNH HUỐNG 3: Đo lường khối lệnh tính toán qua do_luong_thoi_gian!
    // ------------------------------------------------------------------------
    println!("\n3. Đo lường hiệu năng của một khối thuật toán:");
    
    let tong_tich_luy = do_luong_thoi_gian!("Tính tổng dãy 1 triệu số", {
        let mut tong: u64 = 0;
        for i in 1..=1_000_000 {
            tong += i;
        }
        tong // Giá trị trả về từ khối block
    });

    println!("-> Kết quả tính được từ khối mã: {}", tong_tich_luy);

    println!("\n============================================================");
    println!("     XÁC THỰC CÁC MACRO KHAI BÁO HOÀN THÀNH AN TOÀN TUYỆT ĐỐI");
    println!("============================================================");
}
```

---

## Bảng tra cứu lỗi biên dịch & Cách khắc phục (Compiler Error Guide)

Khi bắt đầu viết macro với `macro_rules!`, bạn sẽ gặp những lỗi biên dịch rất đặc thù của bộ phân tích cú pháp:

| Mã lỗi | Thông báo mẫu từ trình biên dịch | Nguyên nhân cốt lõi | Cách khắc phục nhanh |
|---|---|---|---|
| **E0423** | `expected value, found macro '...'` | Bạn gọi một macro nhưng quên viết dấu chấm than `!` ở phía sau tên macro (ví dụ viết `println("...")` thay vì `println!("...")`). | Thêm dấu chấm than `!` ngay sau tên macro: `ten_macro!(...)`. |
| **Lỗi cú pháp** | `no rules expected the token '...'` | Tham số bạn truyền vào khi gọi macro không khớp với bất kỳ nhánh so khớp nào đã được định nghĩa trong `macro_rules!`. | Kiểm tra lại khuôn mẫu ở vế trái: dấu ngăn cách (dấu phẩy, dấu mũi tên `=>`), kiểu dữ liệu của matcher, hoặc bổ sung thêm nhánh so khớp mới. |
| **Lỗi cú pháp** | `$e:expr is followed by '...', which is not allowed for expr fragments` | Quy tắc an toàn cú pháp của Rust: Sau một biểu thức `$e:expr`, bạn chỉ được phép đặt các ký tự phân cách an toàn như `,`, `;`, hoặc `=>`. Bạn không thể đặt ngay một định danh khác liền kề vì trình biên dịch sẽ bị nhập nhằng cú pháp. | Đặt dấu phẩy `,` hoặc dấu chấm phẩy `;` ngăn cách giữa các matcher. |
| **E0425** | `cannot find value '...' in this scope` | Mã bên trong thân macro tham chiếu đến một biến hoặc hàm mà ở phạm vi người gọi macro không tồn tại. | Đảm bảo các biến cần thiết được truyền trực tiếp vào macro qua tham số, hoặc sử dụng đường dẫn đầy đủ dạng `std::...` hoặc `$crate::...`. |

### Phân tích lỗi thực tế "No rules expected the token":

```rust
// Định nghĩa macro chỉ nhận 1 biểu thức:
macro_rules! in_gap_doi {
    ( $x:expr ) => {
        println!("{}", $x * 2);
    };
}

fn thu_nghiem_loi_macro() {
    in_gap_doi!(10); // Hợp lệ!

    // LỖI: no rules expected the token `,`
    // in_gap_doi!(10, 20); // Sai vì macro không có nhánh nhận 2 tham số!
}

// Cách sửa chữa: Bổ sung nhánh nhận 2 tham số hoặc dùng cú pháp lặp $(,)*
macro_rules! in_gap_doi_sua {
    ( $x:expr ) => { println!("{}", $x * 2); };
    ( $x:expr, $y:expr ) => { println!("{} và {}", $x * 2, $y * 2); };
}
```

---

## Tóm tắt chương & Bài tập rèn luyện (Summary & Exercises)

### 4 Điểm cốt lõi cần ghi nhớ:
1. **Bản chất Siêu lập trình**: Code sinh ra code tại thời điểm biên dịch (Compile-time), không gây ra bất kỳ chi phí trễ nào lúc chạy (Zero Runtime Overhead).
2. **Sức mạnh vượt trội so với Hàm**: Macro chấp nhận số lượng tham số linh hoạt, có thể nhận cả định danh và kiểu dữ liệu, cho phép sáng tạo cú pháp mới (DSL).
3. **Bộ khớp cú pháp (Matchers)**: Sử dụng các designator chuẩn xác (`$expr`, `$ident`, `$ty`, `$block`) để định hình khung mẫu tiếp nhận mã nguồn.
4. **Vũ khí gỡ lỗi**: Khai thác sức mạnh của `stringify!`, `file!`, `line!`, và công cụ `cargo expand` để thấu suốt bản chất mã nguồn sau khi được bung ra.

### Bài tập rèn luyện tự giải:
1. **Bài tập 1 (Macro Hoán đổi Biến)**:  
   Hãy viết một macro mang tên `hoan_doi!($a:ident, $b:ident)` nhận vào hai định danh biến và hoán đổi giá trị của chúng cho nhau bằng một biến tạm. Viết hàm `main` kiểm tra với hai biến số nguyên `mut x = 5; mut y = 10;`.

2. **Bài tập 2 (Macro So sánh Giá trị Lớn nhất)**:  
   Viết một macro `tim_max!($a:expr, $b:expr)` sử dụng biểu thức `if/else` để trả về giá trị lớn hơn giữa hai biểu thức. Đảm bảo kết quả của macro có thể được gán trực tiếp vào một biến bất biến: `let max = tim_max!(15, 27);`.

3. **Bài tập 3 (Tư duy thiết kế Macro vs Hàm)**:  
   Khi nào bạn nên viết một hàm bình thường `fn`, và khi nào bạn thực sự bắt buộc phải dùng `macro_rules!`? Hãy liệt kê 3 trường hợp mà hàm thông thường hoàn toàn bất lực không thể giải quyết được.
