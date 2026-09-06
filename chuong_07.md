# Chương 07: Vay mượn & Tham chiếu: Chia sẻ an toàn tuyệt đối (Borrowing & References: Sharing Safely)

## Giới thiệu & Mục tiêu học tập

Ở chương trước, bạn đã hiểu về Quyền sở hữu (Ownership). Nhưng bạn có nhận thấy một điều khá bất tiện không? Mỗi lần chúng ta truyền một biến vào hàm để tính toán (ví dụ: chỉ để đo độ dài của một chuỗi), quyền sở hữu lại bị "chuyển giao" (Move) luôn vào hàm đó. Để tiếp tục sử dụng chuỗi ở các dòng lệnh tiếp theo, hàm lại phải nhọc công trả ngược biến đó về qua một bộ đôi Tuple rườm rà.

Trong đời thực, nếu bạn muốn bạn bè xem một tấm ảnh trên điện thoại, bạn đâu cần phải sang tên tặng luôn chiếc điện thoại cho họ, đúng không? Bạn chỉ cần **cho họ mượn xem một lát**, sau khi xem xong họ trả lại điện thoại cho bạn.

Rust hiện thực hóa chính xác tư duy thực tế đó qua cơ chế **Vay mượn (Borrowing)** và **Tham chiếu (References)**.

Mục tiêu học tập của chương này:
- Nắm vững khái niệm Tham chiếu (Reference) bằng dấu `&` và cơ chế Vay mượn (Borrowing).
- Phân biệt rạch ròi giữa Tham chiếu bất biến (`&T` - Mượn chỉ để đọc) và Tham chiếu khả biến (`&mut T` - Mượn để chỉnh sửa).
- NẰM LÒNG HAI QUY TẮC SẮT ĐÁ của người gác cổng Borrow Checker nhằm triệt tiêu hoàn toàn lỗi Xung đột dữ liệu (**Data Race**).
- Hiểu cơ chế Vòng đời không từ vựng (**Non-Lexical Lifetimes - NLL**) giúp việc viết mã trở nên linh hoạt và tự nhiên.
- Làm quen với khái niệm Lát cắt chuỗi (**String Slices - `&str`**) để xử lý văn bản với hiệu năng cực đại mà không tốn thêm bộ nhớ.

---

## Hình tượng hóa đời sống (Intuitive Everyday Analogy)

Để các quy tắc vay mượn của Rust trở nên thân thuộc như hơi thở, hãy cùng ghi nhớ 3 hình tượng đời sống sau:

```
┌──────────────────────────────────────────────────────────────────────────────────┐
│                   HÌNH TƯỢNG ĐỜI SỐNG VỀ VAY MƯỢN TRONG RUST                     │
├─────────────────────────┬───────────────────────────────┬────────────────────────┤
│   PHÒNG ĐỌC THƯ VIỆN MỞ │    THỢ SỬA XE ĐỘC QUYỀN GARA  │   VỪA ĐỌC VỪA SỬA LÀ CẤM!│
│     (Tham chiếu đọc &T) │    (Tham chiếu sửa &mut T)    │   (Triệt tiêu Data Race)│
│                         │                               │                        │
│ - Cuốn bách khoa quý giá│ - Xe đưa vào gara sửa máy     │ - Không thể vừa mở máy │
│ - 10 độc giả cùng ngồi  │ - Chỉ ĐÚNG 1 thợ được can thiệp│   thay dầu nhớt        │
│   quanh bàn cùng đọc    │ - Lúc này chủ xe và bạn bè    │ - Lại vừa cho người    │
│ - Miễn là KHÔNG AI cầm  │   không được trèo lên xe lái  │   khác trèo lên nổ máy │
│   bút vẽ bậy lên sách   │   thử hay đọc đồng hồ         │   chạy thử -> TAI NẠN! │
└─────────────────────────┴───────────────────────────────┴────────────────────────┘
```

### 1. Cuốn bách khoa trong phòng đọc thư viện mở (Tham chiếu đọc `&T`)
Hãy tưởng tượng một cuốn đại bách khoa toàn thư được đặt ở giữa bàn trong phòng đọc:
- Vì cuốn sách được quy ước là "chỉ đọc" (bất biến), nên cùng một lúc **5, 10 hay thậm chí 100 độc giả** có thể cùng quây quần bên chiếc bàn để cùng đọc các trang sách.
- Mọi chuyện diễn ra cực kỳ hòa bình và an toàn, bởi vì **không một ai được phép cầm bút hay kéo để chỉnh sửa** nội dung của cuốn sách đó.

### 2. Người thợ sửa xe độc quyền trong gara (Tham chiếu sửa đổi `&mut T`)
Khi bạn dắt chiếc xe máy vào gara và nhờ bác thợ tháo tung lốc máy ra để đại tu:
- Trong suốt khoảng thời gian xe đang được sửa chữa, bác thợ là người **duy nhất có quyền can thiệp độc quyền** vào chiếc xe máy.
- Bác thợ không thể để thêm một người thứ hai cùng cầm mỏ lết chọc vào cùng một bánh răng máy tính, vì hai người sẽ va chạm vào nhau làm hỏng máy.

### 3. Quy tắc an toàn tối thượng: Không bao giờ vừa đọc vừa sửa đồng thời!
Điều gì sẽ xảy ra nếu chiếc xe máy vừa đang được thợ tháo ốc vít sửa phanh, lại vừa có một người bạn nhảy lên xe vặn ga phóng thử? Một vụ tai nạn kinh hoàng chắc chắn sẽ xảy ra!

Trong lập trình, hiện tượng này được gọi là **Xung đột dữ liệu (Data Race)**: Một luồng thì đang sửa dữ liệu, một luồng khác thì đang đọc. Người đọc sẽ đọc phải những mảnh dữ liệu rác, chắp vá dở dang khiến ứng dụng sập tan tành.
Rust bảo vệ bạn bằng một lời răn đe bất biến: **ĐÃ CÓ NGƯỜI SỬA THÌ CẤM AI ĐƯỢC ĐỌC; ĐÃ CÓ NGƯỜI ĐỌC THÌ CẤM AI ĐƯỢC PHÉP SỬA!**

---

## Khái niệm & Cơ chế kỹ thuật chuyên sâu (In-Depth Technical Mechanics)

### 1. Bản chất phần cứng của một Tham chiếu (Reference)

Dưới góc nhìn ô nhớ, một tham chiếu `&s` thực chất là gì?
- Nó là một **Con trỏ (Pointer)** có kích thước cố định bằng 8 bytes (trên hệ điều hành 64-bit).
- Con trỏ này nằm trên Stack và lưu trữ **địa chỉ số nhà** của biến gốc mà nó đang trỏ tới.
- Điểm khác biệt sống còn với ngôn ngữ C/C++: trong **Rust an toàn (Safe Rust)**, mọi tham chiếu `&T` / `&mut T` đều được trình biên dịch đảm bảo **chắc chắn 100% là đang trỏ vào một ô nhớ còn sống và hợp lệ**, tuyệt đối không bao giờ có chuyện trỏ vào khoảng không hư vô (Null Pointer hay Dangling Pointer)! (Rust vẫn có *con trỏ thô* `*const T` / `*mut T` có thể null hoặc lơ lửng, nhưng chúng chỉ được phép giải tham chiếu bên trong khối `unsafe` — chủ đề của Chương 39.)

```
      STACK (Biến tham chiếu)                  STACK (Biến gốc)
   ┌────────────────┬──────────┐            ┌────────────────┬──────────┐
   │ Tên: ptr_ref   │  0x1000  │ ─────────> │ Địa chỉ ô nhớ  │  0x1000  │
   │ Kích thước     │  8 bytes │            │ Tên biến gốc   │    s1    │
   └────────────────┴──────────┘            └────────────────┴──────────┘
```

### 2. Hai quy tắc vàng của Borrow Checker (The Rules of Borrowing)

Trình biên dịch Rust áp dụng nguyên tắc kiểm tra tương đương với nguyên lý Độc quyền truy cập (**Aliasing XOR Mutability**):

> **Tại một thời điểm bất kỳ đối với một vùng dữ liệu:**
> 1. Bạn có thể có **vô số tham chiếu bất biến** (`&T`),
> 2. **HOẶC** bạn chỉ có thể có **duy nhất MỘT tham chiếu khả biến** (`&mut T`).
> 3. Không bao giờ được phép tồn tại cả hai loại tham chiếu này cùng lúc!

### 3. Vòng đời không từ vựng (Non-Lexical Lifetimes - NLL)

Ở các phiên bản Rust xa xưa, một biến mượn sẽ giữ quyền mượn cho đến tận dấu ngoặc nhọn đóng `}` của khối mã. Nhưng từ phiên bản Rust 2018 trở đi, trình biên dịch được nâng cấp cực kỳ thông minh với tính năng **Non-Lexical Lifetimes (NLL)**:
- Phạm vi mượn của một tham chiếu sẽ **kết thúc ngay tại dòng lệnh cuối cùng mà nó được sử dụng thực tế**, chứ không cần đợi đến hết dấu ngoặc nhọn!

Ví dụ minh họa:
```rust
let mut s = String::from("Xin chào");

let r1 = &s; // Mượn đọc
let r2 = &s; // Mượn đọc thêm người thứ hai
println!("{} và {}", r1, r2); 
// ---> Kể từ dòng này trở đi, r1 và r2 KHÔNG CÒN ĐƯỢC DÙNG NỮA! Quyền mượn đọc đã tự động kết thúc.

let r3 = &mut s; // HOÀN TOÀN HỢP LỆ! Vì các lệnh đọc trước đó đã xong xuôi.
r3.push_str(" Việt Nam");
println!("{}", r3);
```

### 4. Lát cắt chuỗi (String Slices - `&str`)

Khi bạn muốn trích xuất một từ trong một câu văn dài mà không muốn cấp phát thêm bộ nhớ Heap mới để sao chép từ đó, Rust cung cấp kiểu **Lát cắt chuỗi (`&str`)**:
- Lát cắt thực chất là một tham chiếu trỏ vào một đoạn liên tiếp của chuỗi ban đầu.
- Nó chỉ chiếm đúng **16 bytes trên Stack** (gồm 8 bytes con trỏ trỏ tới byte bắt đầu, và 8 bytes lưu **độ dài tính bằng byte** của lát cắt — *không phải* số chữ cái, xem cảnh báo UTF-8 ở cuối mục này).

### 5. Toán tử Giải tham chiếu (Dereference Operator - `*`)

Khi bạn nắm giữ một tham chiếu `so: &mut i32`, bạn thực chất chỉ đang cầm một chiếc "thẻ ghi số nhà" (con trỏ lưu địa chỉ 8 bytes trên Stack).
- Nếu bạn muốn trực tiếp mở cửa bước vào căn nhà đó để đọc hoặc sửa đổi giá trị thực tế bên trong ô nhớ, bạn phải thực hiện thao tác **Giải tham chiếu (Dereferencing)** bằng cách đặt dấu sao `*` ngay phía trước tên biến:
```rust
let mut x = 10;
let r = &mut x; // r là tham chiếu mượn sửa trỏ tới x
*r = *r * 2;    // Đi theo địa chỉ con trỏ để nhân đôi giá trị gốc của x lên 20!
```
- **Lưu ý thực chiến**: Với các kiểu dữ liệu phức tạp như `String`, Rust tự động kích hoạt cơ chế Ép kiểu giải tham chiếu (**Deref Coercion**) khi bạn gọi phương thức (ví dụ: `chuoi.push_str(...)`), giúp bạn không cần phải viết dấu `*` thủ công. Nhưng đối với các kiểu dữ liệu nguyên bản (`i32`, `f64`, `bool`), dấu `*` là công cụ tường minh và bắt buộc khi thao tác qua tham chiếu!

> **CẢNH BÁO QUAN TRỌNG VỀ LÁT CẮT VÀ CHUỖI TIẾNG VIỆT (UTF-8):**
> Trong Rust, chỉ số cắt lát `[start..end]` luôn tính theo **đơn vị Byte**, tuyệt đối **KHÔNG PHẢI số thứ tự chữ cái**!
> Các chữ cái tiếng Việt có dấu (như `'à'`, `'é'`, `'ộ'`) là các ký tự Unicode nhiều bytes (thường chiếm 2 đến 3 bytes trong UTF-8). Ví dụ: từ `"an toàn"` kéo dài từ byte số 5 đến byte số 12 (chữ `'à'` chiếm 2 bytes số 10 và 11; chữ `'n'` nằm ở byte số 12). Để lấy trọn vẹn chữ `'n'`, bạn phải cắt theo dải nửa mở đến trước byte số 13: `&cau_noi[5..13]`. Nếu bạn cắt nhầm vào giữa byte của ký tự Unicode (ví dụ `[5..11]`), Rust sẽ dừng chương trình ngay lập tức (**panic**) để ngăn ngừa hỏng dữ liệu!

---

## Mã nguồn minh họa thực chiến (Idiomatic Runnable Rust Blueprint)

Chương trình dưới đây minh họa toàn bộ các khía cạnh: mượn chỉ đọc, mượn sửa đổi, toán tử giải tham chiếu, cơ chế NLL, và lát cắt chuỗi:

```rust
// File: src/main.rs
// Chương trình thực hành chuyên sâu về Vay mượn (Borrowing) và Tham chiếu (References)

// 1. Hàm mượn chỉ đọc (&String): Nhận dữ liệu để tính toán nhưng KHÔNG cướp quyền sở hữu
fn series_length(series: &String) -> usize {
    // chuoi là một tham chiếu chỉ đọc, ta chỉ có thể xem nội dung qua .len()
    series.len()
}

// 2. Hàm mượn sửa đổi (&mut String): Cho phép thay đổi trực tiếp nội dung biến gốc
fn add_greeting(chuoi_goc: &mut String) {
    // Phương thức .push_str() ghi thêm ký tự vào bãi đỗ Heap của biến gốc
    chuoi_goc.push_str(" - Chúc bạn một ngày tràn đầy năng lượng!");
}

// 3. Hàm minh họa toán tử giải tham chiếu (Dereferencing '*') với số nguyên
fn double(so: &mut i32) {
    // Dấu * dùng để đi theo địa chỉ con trỏ và can thiệp thẳng vào giá trị thực bên trong ô nhớ
    *so = *so * 2;
}

fn main() {
    println!("============================================================");
    println!("     CHƯƠNG TRÌNH LÀM CHỦ VAY MƯỢN & THAM CHIẾU TRONG RUST  ");
    println!("============================================================");

    // --- PHẦN 1: THAM CHIẾU BẤT BIẾN (&T - MƯỢN ĐỂ ĐỌC) ---
    println!("\n1. Minh họa mượn dữ liệu chỉ để đọc:");
    let thong_tin_xe = String::from("Xe máy Honda SH 150i");

    // Truyền &thong_tin_xe: Ta chỉ đưa "tấm ảnh chụp" địa chỉ ô nhớ cho hàm mượn
    let length = series_length(&thong_tin_xe);
    
    // Biến thong_tin_xe vẫn còn nguyên quyền sở hữu thuộc về hàm main!
    println!("- Xe máy: '{}'", thong_tin_xe);
    println!("- Số lượng ký tự trong chuỗi thông tin: {}", length);

    // Nhiều người có thể cùng mượn đọc đồng thời một lúc:
    let reader_1 = &thong_tin_xe;
    let reader_2 = &thong_tin_xe;
    println!("- Độc giả 1 đọc: {}", reader_1);
    println!("- Độc giả 2 đọc: {}", reader_2);

    // --- PHẦN 2: THAM CHIẾU KHẢ BIẾN (&mut T - MƯỢN ĐỂ SỬA) ---
    println!("\n2. Minh họa mượn dữ liệu để sửa đổi trực tiếp:");
    let mut letter = String::from("Xin chào bạn thân mến");
    println!("- Bức thư ban đầu: '{}'", letter);

    // Mượn để chỉnh sửa nội dung thông qua &mut
    add_greeting(&mut letter);
    println!("- Bức thư sau khi sửa: '{}'", letter);

    // --- PHẦN 3: GIẢI THAM CHIẾU VỚI TOÁN TỬ '*' TRÊN SỐ NGUYÊN ---
    println!("\n3. Thao tác ô nhớ số nguyên với toán tử giải tham chiếu (*):");
    let mut account_xu = 500;
    println!("- Số xu trước khi nhân đôi: {}", account_xu);

    double(&mut account_xu);
    println!("- Số xu sau khi nhân đôi  : {}", account_xu);

    // --- PHẦN 4: LÁT CẮT CHUỖI (STRING SLICES - &str) ---
    println!("\n4. Trích xuất văn bản bằng Lát cắt chuỗi (String Slices):");
    let sentence = String::from("Rust an toàn tuyệt đối");

    // Lát cắt trỏ vào một phần ô nhớ của chuỗi mà không tạo dữ liệu mới:
    let first_from: &str = &sentence[0..4];    // Cắt từ chỉ số byte 0 đến trước 4 ("Rust")
    let from_two: &str = &sentence[5..13];   // Cắt từ chỉ số byte 5 đến trước 13 ("an toàn")

    println!("- Câu nói gốc: '{}'", sentence);
    println!("- Từ thứ nhất : '{}' (chiếm {} bytes trên Stack)", first_from, std::mem::size_of_val(&first_from));
    println!("- Từ thứ hai  : '{}'", from_two);

    // --- PHẦN 5: CHỨNG MINH TÍNH LINH HOẠT CỦA NLL (NON-LEXICAL LIFETIMES) ---
    println!("\n5. Kiểm tra cơ chế Vòng đời không từ vựng (NLL):");
    let mut order_log = String::from("Nhật ký ngày 01");

    let read_log = &order_log; // Bắt đầu mượn đọc
    println!("- Đọc nhật ký: {}", read_log);
    // Sau dòng print trên, read_log không còn được dùng nữa -> Hết hiệu lực mượn!

    let fix_log = &mut order_log; // Được phép mượn sửa ngay lập tức mà không xung đột!
    fix_log.push_str(" - Đã ghi thêm sự kiện mới");
    println!("- Nội dung sau cập nhật: {}", fix_log);
}
```

---

## Bảng tra cứu lỗi biên dịch & Cách khắc phục (Compiler Error Guide)

Đây là những lỗi biên dịch kinh điển liên quan đến Borrow Checker mà bạn chắc chắn sẽ gặp:

| Mã lỗi | Thông báo mẫu từ trình biên dịch | Nguyên nhân cốt lõi | Cách khắc phục nhanh |
|---|---|---|---|
| **E0499** | `cannot borrow 'x' as mutable more than once at a time` | Bạn cố tình tạo từ hai tham chiếu khả biến (`&mut`) trở lên cho cùng một biến trong cùng một phạm vi thời gian. | Đảm bảo người thợ thứ nhất hoàn thành công việc sửa đổi xong xuôi rồi mới tạo tham chiếu `&mut` thứ hai. |
| **E0502** | `cannot borrow 'x' as mutable because it is also borrowed as immutable` | Bạn vừa có người đang mượn đọc (`&`), lại vừa tạo một tham chiếu mượn sửa (`&mut`) đè lên cùng lúc. | Di chuyển dòng lệnh đọc lên trước và kết thúc việc đọc trước khi thực hiện mượn sửa, hoặc tách phạm vi bằng khối ngoặc nhọn `{}`. |
| **E0596** | `cannot borrow 'x' as mutable, as it is not declared as mutable` | Bạn cố mượn sửa `&mut x` nhưng khi khai báo biến ban đầu lại quên không viết từ khóa `mut`. | Thêm từ khóa `mut` vào biến gốc ban đầu: `let mut x = ...`. |
| **E0106** | `missing lifetime specifier` | Trả về một tham chiếu từ hàm mà không rõ tham chiếu đó trỏ vào dữ liệu nào (sẽ giải quyết triệt để ở Chương 08). | Đảm bảo hàm trả về giá trị sở hữu hoặc thêm chú thích vòng đời phù hợp. |

---

## Tóm tắt chương & Bài tập rèn luyện (Summary & Exercises)

### 4 Điểm cốt lõi cần ghi nhớ:
1. **Vay mượn (Borrowing)**: Sử dụng dấu `&` để tạo tham chiếu trỏ vào dữ liệu mà không cướp quyền sở hữu gốc của biến.
2. **Nguyên tắc Aliasing XOR Mutability**: Tại một thời điểm, được phép có vô số người đọc (`&T`) HOẶC duy nhất một người sửa (`&mut T`), không bao giờ được phép vừa đọc vừa sửa đồng thời.
3. **Triệt tiêu Data Race**: Nhờ cơ chế kiểm tra vay mượn ngay lúc biên dịch, Rust đảm bảo mã nguồn đa luồng không bao giờ gặp lỗi xung đột dữ liệu lúc chạy.
4. **Lát cắt chuỗi (`&str`)**: Một con trỏ nhẹ nhàng 16-bit trỏ thẳng vào một đoạn của chuỗi văn bản, giúp thao tác trích xuất từ ngữ mà không cần sao chép bộ nhớ Heap.

### Bài tập rèn luyện tự giải:
1. **Bài tập thực hành 1**: Viết hàm `them_loi_chuc(loi_nhan: &mut String)` nhận vào một tham chiếu khả biến (`&mut String`). Hàm sẽ sử dụng phương thức `.push_str()` để nối thêm chuỗi `", chúc bạn học tốt Rust!"` vào trực tiếp cuối chuỗi gốc. Trong hàm `main`: khởi tạo một biến chuỗi có từ khóa `mut` (ví dụ: `"Chào bạn"`), truyền tham chiếu mượn sửa `&mut` vào hàm, sau đó in chuỗi ra màn hình để kiểm chứng dữ liệu đã được cập nhật thành công mà không làm mất quyền sở hữu gốc.
2. **Bài tập tìm lỗi (Borrow Checker audit)**: Hãy giải thích tại sao đoạn mã sau đây không thể biên dịch:
   ```rust
   let mut list = String::from("Táo, Cam");
   let doc = &list;
   list.push_str(", Xoài");
   println!("Danh sách quả: {}", doc);
   ```
   Hãy chỉ ra lỗi và viết lại đoạn mã để nó biên dịch thành công mà vẫn in ra được đầy đủ 3 loại quả.
3. **Bài tập tư duy 3**: Tại sao việc cấm "Vừa đọc vừa sửa" lại có thể ngăn chặn được lỗi sập chương trình khi một chuỗi `String` tự động phình to kích thước trên Heap? Hãy giải thích mối liên hệ giữa việc tái cấp phát Heap (Reallocation) và con trỏ đọc lơ lửng.

---

### Gợi ý & Lời giải

<details>
<summary><b>Bài tập 1 — Gợi ý</b></summary>

`&mut String` cho phép hàm sửa thẳng chuỗi gốc. `.push_str` nối thêm vào cuối. Người gọi phải khai báo biến bằng `mut` và truyền `&mut`.
</details>

<details>
<summary><b>Bài tập 1 — Lời giải</b></summary>

```rust
// Nhận tham chiếu KHẢ BIẾN: hàm sửa thẳng chuỗi gốc, không lấy quyền sở hữu.
fn them_loi_chuc(loi_nhan: &mut String) {
    loi_nhan.push_str(", chúc bạn học tốt Rust!");
}

fn main() {
    let mut loi = String::from("Chào bạn");   // mut vì chuỗi sẽ bị sửa
    them_loi_chuc(&mut loi);                    // cho mượn để sửa
    println!("{loi}");                          // chuỗi gốc đã được cập nhật
}

#[test]
fn noi_them_vao_chuoi_goc() {
    let mut loi = String::from("Chào bạn");
    them_loi_chuc(&mut loi);
    assert_eq!(loi, "Chào bạn, chúc bạn học tốt Rust!");
    // Sau khi hàm trả về, `loi` VẪN thuộc về main — chỉ được sửa, không bị nuốt.
}
```

Điểm cốt lõi: `&mut String` là **mượn để sửa**, khác hẳn `String` (lấy luôn quyền sở hữu) và `&String` (mượn chỉ để đọc). Nhờ mượn sửa, hàm thay đổi được dữ liệu tại chỗ mà người gọi *không mất* biến — sau lời gọi, `main` vẫn dùng `loi` bình thường. Đây là cách Rust cho phép "hàm sửa đối số" một cách an toàn và tường minh: phải viết rõ `&mut` ở cả nơi khai báo hàm lẫn nơi gọi, không có sửa lén.
</details>

<details>
<summary><b>Bài tập 2 — Gợi ý</b></summary>

Đây là quy tắc mượn cốt lõi: **không được vừa giữ một tham chiếu đọc (`&list`) vừa sửa (`push_str`)**. `doc` mượn bất biến, còn `push_str` cần mượn khả biến — hai thứ không sống chung.
</details>

<details>
<summary><b>Bài tập 2 — Lời giải</b></summary>

**Lỗi: `cannot borrow `list` as mutable because it is also borrowed as immutable`.**

Chuyện xảy ra theo dòng thời gian:
```text
let doc = &list;            // (1) mượn ĐỌC bắt đầu
list.push_str(", Xoài");    // (2) cần mượn SỬA -> ĐỤNG mượn đọc còn sống
println!("... {}", doc);    // (3) doc còn được dùng ở đây -> nên (1) chưa kết thúc
```

Vì `doc` còn được dùng ở dòng (3), phép mượn đọc ở (1) vẫn **còn sống** khi tới (2). Rust cấm mượn-sửa trong lúc một mượn-đọc đang sống, nên chặn ngay.

**Viết lại cho biên dịch được — dùng xong `doc` rồi mới sửa:**
```text
let mut list = String::from("Táo, Cam");
let doc = &list;
println!("Trước khi thêm: {doc}");   // dùng doc XONG ở đây
// Tới đây mượn đọc đã kết thúc -> tự do sửa:
list.push_str(", Xoài");
println!("Danh sách quả: {list}");   // Táo, Cam, Xoài
```

Mẹo: Rust dùng **NLL (non-lexical lifetimes)** — phép mượn kết thúc ngay tại *lần dùng cuối*, không phải ở cuối khối `{}`. Nên chỉ cần đưa mọi lần đọc `doc` lên *trước* lần sửa là xong. Đây không phải mẹo lách luật — nó phản ánh đúng ý định: đọc xong rồi mới đổi thì chẳng có mâu thuẫn nào.
</details>

<details>
<summary><b>Bài tập 3 — Gợi ý</b></summary>

Nối hai sự thật lại: (a) khi `String` đầy chỗ, nó **tái cấp phát** — dời toàn bộ dữ liệu sang vùng heap mới; (b) một tham chiếu đọc giữ **địa chỉ cũ**.
</details>

<details>
<summary><b>Bài tập 3 — Lời giải</b></summary>

Cấm "vừa đọc vừa sửa" chính là cấm một lỗi dùng-sau-khi-giải-phóng rất tinh vi. Cơ chế:

Khi bạn `push_str` mà chuỗi đã đầy sức chứa (capacity), `String` phải **tái cấp phát (reallocate)**:
```text
Trước:  Stack[con trỏ = 0xAAA] ──▶ Heap 0xAAA: "Táo, Cam"  (đầy chỗ)

push_str khiến hết chỗ -> cấp vùng MỚI lớn hơn, CHÉP sang, GIẢI PHÓNG vùng cũ:

Sau:    Stack[con trỏ = 0xBBB] ──▶ Heap 0xBBB: "Táo, Cam, Xoài"
                                    Heap 0xAAA: (ĐÃ GIẢI PHÓNG — rác)
```

Bây giờ giả sử Rust *cho phép* giữ một tham chiếu đọc `doc` xuyên qua thao tác này. `doc` đã chụp lại địa chỉ **0xAAA** — nhưng 0xAAA vừa bị giải phóng. Đọc `doc` giờ là đọc vùng nhớ đã trả lại hệ điều hành: **dùng-sau-khi-giải-phóng**, thứ gây ra sập chương trình hoặc lỗ hổng bảo mật kinh điển trong C/C++.

Quy tắc mượn chặn đúng chỗ đó: **một mượn-đọc còn sống thì cấm mọi mượn-sửa**. Nhờ vậy con trỏ mà `doc` giữ được bảo đảm vẫn trỏ tới vùng nhớ hợp lệ suốt thời gian nó sống — vì trong khoảng đó không thao tác nào được phép tái cấp phát. Đây là một trong những ví dụ đẹp nhất cho thấy borrow checker không phải luật lệ tùy tiện, mà là **định lý an toàn bộ nhớ được ép ngay lúc biên dịch**.
</details>
