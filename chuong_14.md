# Chương 14: Ghép hàm, Curry hóa và Áp dụng từng phần (Function Composition, Currying & Partial Application)

## Giới thiệu & Mục tiêu học tập

Ở Chương 13, bạn đã học được rằng lập trình hàm nhìn chương trình như một **chuỗi biến đổi dữ liệu** thay vì một chuỗi mệnh lệnh xáo trộn bộ nhớ. Nhưng chúng ta mới chỉ nói tới cái *kết quả* — những đường ống `.filter().map().sum()` đẹp mắt — mà chưa trả lời câu hỏi nền tảng nhất:

> **Vì sao các hàm lại ghép nối được với nhau? Và ghép bằng cách nào?**

Đây không phải câu hỏi phụ. Trong cộng đồng lập trình hàm quốc tế, người ta xem toàn bộ trường phái này đứng trên đúng **hai trụ cột**:
1. **Minh bạch tham chiếu (Referential Transparency)** — đã học ở Chương 13.
2. **Phép ghép hàm (Composition)** — chính là nội dung của chương này.

Nếu thiếu trụ cột thứ hai, bạn chỉ đang "dùng cú pháp của lập trình hàm" chứ chưa thực sự **tư duy** theo lập trình hàm. Bạn sẽ viết được `.map()` nhưng không tự xây được công cụ mới; bạn sẽ dùng được closure nhưng không biết cách biến một hàm 3 tham số thành một "nhà máy" sinh ra vô số hàm chuyên dụng.

Chương này cũng mở khóa một kỹ thuật cực kỳ thực dụng mà các hệ thống Rust lớn dùng hằng ngày: **tiêm phụ thuộc (Dependency Injection) bằng áp dụng từng phần**, giúp bạn viết mã kiểm thử được mà không cần bất kỳ thư viện giả lập (mocking framework) nào.

Mục tiêu học tập của chương này:
- Đọc được **chữ ký hàm** như đọc một hợp đồng: `A -> B` nghĩa là gì, và vì sao hai hợp đồng `A -> B` và `B -> C` lại "khớp nối" được thành `A -> C`.
- Tự tay xây dựng hàm **`compose` (ghép)** và kiểm chứng **luật kết hợp** của phép ghép hàm.
- Nắm vững **Curry hóa (Currying)**: biến hàm nhiều tham số thành chuỗi hàm một tham số.
- Làm chủ **Áp dụng từng phần (Partial Application)** và ứng dụng trực tiếp của nó: nhà máy sinh hàm và tiêm phụ thuộc.
- Hiểu **Lối viết không nêu tham số (Point-free style)**, biết khi nào nên dùng và khi nào nó làm mã khó đọc hơn.
- Bổ sung vào kho vũ khí 3 **bộ kết hợp nền tảng (combinators)**: `identity`, `flip`, `const`.

---

## Hình tượng hóa đời sống (Intuitive Everyday Analogy)

```
┌──────────────────────────────────────────────────────────────────────────────────┐
│        HÌNH TƯỢNG ĐỜI SỐNG: ỐNG NƯỚC LẮP REN VÀ MÁY PHA CÀ PHÊ CÀI SẴN           │
├────────────────────────────────────────┬─────────────────────────────────────────┤
│      GHÉP HÀM = LẮP REN ỐNG NƯỚC       │   CURRY HÓA = MÁY PHA CÀ PHÊ CÀI SẴN    │
│                                        │                                         │
│  Ống A→B          Ống B→C              │  Máy pha đầy đủ 3 núm vặn:              │
│  ┌────────┐      ┌────────┐            │  pha(loại_hạt, độ_đường, cỡ_ly)         │
│  │ Nước   │      │ Nước   │            │                                         │
│  │ giếng  │─┐  ┌─│ đã lọc │─┐          │  Quán quen của bạn CÀI SẴN 2 núm:       │
│  │  → lọc │ │  │ │  → đun │ │          │  ┌────────────────────────────────┐     │
│  └────────┘ │  │ └────────┘ │          │  │ loại_hạt = "Robusta"  [KHÓA]   │     │
│    ren cái ─┘  └─ ren đực   │          │  │ độ_đường = "ít"       [KHÓA]   │     │
│         ▼ VẶN KHỚP ▼        ▼          │  │ cỡ_ly    = ??? (còn trống)     │     │
│  ┌──────────────────────────────┐      │  └────────────────────────────────┘     │
│  │  Ống ghép A→C: giếng → sôi   │      │  Giờ bạn chỉ cần hô "LY LỚN!"           │
│  └──────────────────────────────┘      │  → Ra đúng ly cà phê quen thuộc.        │
│                                        │                                         │
│  Điều kiện DUY NHẤT để vặn được:       │  "Cài sẵn một phần các núm vặn"         │
│  đầu RA của ống 1 phải cùng cỡ ren     │  chính là ÁP DỤNG TỪNG PHẦN             │
│  với đầu VÀO của ống 2  (B khớp B).    │  (Partial Application)                  │
└────────────────────────────────────────┴─────────────────────────────────────────┘
```

### 1. Ống nước lắp ren (Phép ghép hàm)

Trong nhà bạn có hai đoạn ống lọc nước rời:
- Ống thứ nhất: đầu vào là **nước giếng**, đầu ra là **nước đã lọc cặn**.
- Ống thứ hai: đầu vào là **nước đã lọc cặn**, đầu ra là **nước sôi tiệt trùng**.

Bạn có thể vặn hai ống này vào nhau để tạo thành **một ống duy nhất**: đầu vào nước giếng, đầu ra nước sôi. Điều kỳ diệu là sau khi vặn xong, người dùng ống mới **không cần biết** bên trong có mấy đoạn — với họ đó chỉ là "một cái ống".

Điều kiện duy nhất để vặn được: **cỡ ren phải khớp**. Nếu ống thứ hai đòi đầu vào là *khí gas* trong khi ống thứ nhất nhả ra *nước*, hai ống không thể lắp vào nhau. Trong lập trình, "cỡ ren" chính là **kiểu dữ liệu**, và người kiểm tra ren chính là **trình biên dịch `rustc`**.

### 2. Máy pha cà phê cài sẵn công thức (Curry hóa & Áp dụng từng phần)

Chiếc máy pha cà phê ở quán có 3 núm vặn: *loại hạt*, *độ đường*, *cỡ ly*. Mỗi lần pha, nhân viên phải vặn cả 3 núm.

Nhưng bạn là khách quen, ngày nào cũng uống "Robusta, ít đường". Chủ quán bèn làm một việc rất thông minh: **vặn sẵn 2 núm rồi khóa lại**, dán nhãn "Máy của anh Nam". Từ đó, bạn chỉ cần nói cỡ ly.

Chiếc "máy đã khóa 2 núm" đó chính là một **hàm mới** được sinh ra từ hàm gốc bằng cách cố định trước một phần tham số. Đây là **Áp dụng từng phần** — kỹ thuật nền tảng để "tiêm phụ thuộc" trong lập trình hàm.

---

## Khái niệm & Cơ chế kỹ thuật chuyên sâu (In-Depth Technical Mechanics)

### 1. Đọc chữ ký hàm như đọc một hợp đồng

Trước khi ghép, phải biết đọc. Trong ký hiệu của lập trình hàm, một hàm được viết là:

```
length_of :: String -> usize
```

Đọc là: *"hàm `length_of` nhận một `String` và trả về một `usize`"*. Trong Rust ta viết:

```rust
fn length_of(s: String) -> usize { s.len() }
```

Chữ ký hàm (function signature) là **hợp đồng đầy đủ** của một hàm thuần túy. Nếu hàm là thuần túy (Chương 13), chữ ký cho bạn biết *gần như mọi thứ* cần biết:

| Chữ ký | Hàm này có thể làm được gì? |
|---|---|
| `fn f<T>(x: T) -> T` | **Chỉ có đúng một cách cài đặt** (nếu bỏ qua panic hay vòng lặp vô tận): trả về chính `x`! Vì `T` là kiểu tùy ý, hàm không biết gì về nó nên không thể tự tạo ra một giá trị `T` mới. Đây chính là hàm `identity`. |
| `fn f<T>(x: T) -> usize` | Không thể phụ thuộc vào *nội dung* của `x` — chỉ có thể trả về một hằng số (hoặc một giá trị chỉ phụ thuộc vào *kiểu* `T`, như `std::mem::size_of::<T>()`). |
| `fn f(x: &str) -> String` | Có thể cắt, nối, viết hoa... vô số khả năng, vì `&str` và `String` là kiểu cụ thể. |
| `fn f(x: i32) -> Result<u32, NegativeError>` | Có thể **thất bại**. Chữ ký đã tự thú nhận điều đó. |

> **Kỹ năng cần rèn**: khi nhìn một hàm lạ trong tài liệu Rust, hãy đọc chữ ký trước, đoán xem nó làm gì, rồi mới đọc phần mô tả. Đây là cách nhanh nhất để làm chủ thư viện chuẩn.

### 2. Phép ghép hàm (Function Composition)

Cho hai hàm:
- `f: A -> B`
- `g: B -> C`

Ta luôn tạo được hàm thứ ba `g ∘ f : A -> C` (đọc là "g sau f"), định nghĩa bằng `(g ∘ f)(x) = g(f(x))`.

Rust không có sẵn toán tử `∘`, nhưng ta tự viết được trong đúng 3 dòng:

> **Xem trước Chương 15**: từ đây chương này dùng **closure** (hàm vô danh `|x| ...`), từ khóa **`move`**, và kiểu **`impl Fn(A) -> B`** trước khi chúng được giảng kỹ ở Chương 15. Bạn chỉ cần trực giác sau là đủ đọc tiếp:
> - Closure là một hàm viết tại chỗ, có thể "bắt" (dùng) các biến xung quanh nơi nó được tạo.
> - `move` buộc closure **sở hữu** các biến nó bắt, để nó sống được cả sau khi hàm tạo ra nó đã kết thúc (quên `move` là lỗi `E0373`).
> - `impl Fn(A) -> B` nghĩa là "một kiểu nào đó gọi được như một hàm `A -> B`, gọi bao nhiêu lần cũng được"; còn `FnMut` là loại closure được phép sửa trạng thái nó bắt giữ (nhầm lẫn hai loại là lỗi `E0525`).
>
> Chương 15 sẽ mổ xẻ cả ba trait `Fn`/`FnMut`/`FnOnce` và các lỗi này một cách chi tiết.

```rust
pub fn compose<A, B, C>(f: impl Fn(A) -> B, g: impl Fn(B) -> C) -> impl Fn(A) -> C {
    move |x| g(f(x))
}
```

Hãy đọc kỹ chữ ký này — nó chính là định nghĩa toán học viết bằng cú pháp Rust:
- Nhận vào một hàm `A -> B` và một hàm `B -> C`.
- Trả về một hàm `A -> C`.
- Từ khóa `move` là **bắt buộc**: closure trả về phải *sở hữu* `f` và `g`, nếu không chúng sẽ chết ngay khi hàm `compose` kết thúc.

**Ba tính chất phải nhớ về phép ghép:**

1. **Luật kết hợp (Associativity)**: `h ∘ (g ∘ f) = (h ∘ g) ∘ f`.
   Nghĩa là bạn ghép ống theo thứ tự nào cũng cho kết quả y hệt — miễn là **thứ tự các ống trên đường ống không đổi**. Nhờ luật này, ta viết `compose(compose(f, g), h)` hay `compose(f, compose(g, h))` tùy thích.
2. **Phần tử đơn vị (Identity element)**: hàm `identity(x) = x` đóng vai trò "đoạn ống thẳng không làm gì". Ghép nó vào đầu hay cuối đều không đổi kết quả: `f ∘ id = id ∘ f = f`.
3. **Phép ghép KHÔNG giao hoán**: `g ∘ f` khác `f ∘ g`. "Rửa rau rồi thái" khác hẳn "thái rau rồi rửa"!

> Hai tính chất 1 và 2 nghe có vẻ hiển nhiên, nhưng chúng chính là định nghĩa của một cấu trúc toán học tên là **Phạm trù (Category)** — nền móng của toàn bộ lý thuyết ta sẽ gặp lại ở Chương 18 và Chương 19. Hãy nhớ tên hai luật này.

### 3. Bạn đã dùng phép ghép hàng ngày mà không biết

Nhìn lại đường ống quen thuộc:

```rust
let result: Vec<String> = list
    .iter()
    .map(normalize)  // &str -> String
    .map(add_prefix) // String -> String
    .collect();
```

Hai lần `.map()` liên tiếp **chính là một phép ghép hàm**. Và trình biên dịch biết điều đó: nó gộp hai lần `map` thành một vòng lặp duy nhất chạy qua dữ liệu **đúng một lần**, không tạo mảng trung gian. Nói cách khác:

```
list.map(f).map(g)   ≡   list.map(compose(f, g))
```

Đẳng thức này có tên chính thức: **luật ghép của Functor (Functor composition law)** — chúng ta sẽ chứng minh nó ở Chương 19.

### 4. Curry hóa (Currying)

**Curry hóa** là kỹ thuật biến một hàm nhận `n` tham số thành một chuỗi `n` hàm, mỗi hàm nhận đúng **1** tham số:

```
Hàm gốc      : add(a, b) -> i64          (nhận 2 tham số cùng lúc)
Hàm curry hóa: add(a) -> (b -> i64)      (nhận 1 tham số, trả về MỘT HÀM MỚI)
```

Trong Rust:

```rust
// Dạng thông thường
fn add(a: i64, b: i64) -> i64 { a + b }

// Dạng đã curry hóa
fn add_curried(a: i64) -> impl Fn(i64) -> i64 {
    move |b| a + b
}

let add_ten = add_curried(10); // Chưa tính gì cả! Ta vừa tạo ra một HÀM MỚI.
assert_eq!(add_ten(5), 15);
assert_eq!(add_ten(7), 17);   // Dùng lại được vô số lần
```

*Lưu ý về Rust*: các ngôn ngữ như Haskell hay PureScript **tự động curry hóa** mọi hàm. Rust thì không — bạn phải viết tay như trên. Đó là một đánh đổi có chủ đích: Rust ưu tiên hiệu năng dự đoán được và chữ ký hàm rõ ràng hơn là sự tiện lợi cú pháp.

### 5. Áp dụng từng phần (Partial Application) — Vũ khí thực chiến

**Áp dụng từng phần** là *hệ quả trực tiếp* của curry hóa: cung cấp trước một phần tham số, giữ lại phần còn lại cho sau.

Đây chính là chỗ lý thuyết biến thành lợi ích cực kỳ cụ thể. Xét bài toán quen thuộc: hàm gửi email cần biết địa chỉ máy chủ SMTP.

```rust
// ❌ Cách làm quen thuộc: hàm tự đi tìm phụ thuộc của mình
fn send_email(recipient: &str, content: &str) -> Result<(), String> {
    let server = read_server_from_env(); // Phụ thuộc ẩn, không thấy trong chữ ký!
    // ... Muốn kiểm thử hàm này thì phải dựng cả biến môi trường.
}

// ✅ Cách của lập trình hàm: nhận phụ thuộc làm tham số, rồi khóa nó lại
fn make_email_sender(server: String) -> impl Fn(&str, &str) -> Result<(), String> {
    move |recipient, content| {
        // ... dùng biến server đã bị khóa vào closure
        Ok(())
    }
}

// Lúc khởi động chương trình (tầng vỏ):
let send = make_email_sender("smtp.congty.vn".to_string());
// Lúc kiểm thử: chỉ cần khóa vào một máy chủ giả!
let send_test = make_email_sender("localhost:1025".to_string());
```

> **Đây chính là Tiêm phụ thuộc (Dependency Injection)** — không cần framework, không cần container, không cần thư viện giả lập. Nó chỉ là *áp dụng từng phần*. Cuốn *Domain Modeling Made Functional* dành hẳn một mục cho kỹ thuật này, và chúng ta sẽ dùng lại nó ở Chương 20.

### 6. Lối viết không nêu tham số (Point-free style)

So sánh hai cách viết cùng một ý:

```rust
// Có nêu tham số (pointful): ta phải đặt tên cho biến trung gian `s`
let lengths: Vec<usize> = names.iter().map(|s| s.len()).collect();

// Không nêu tham số (point-free): chỉ nói TÊN HÀM cần áp dụng
let lengths: Vec<usize> = names.iter().map(String::len).collect();
```

Ưu điểm: ngắn hơn, và quan trọng hơn là **loại bỏ cơ hội gõ nhầm tên biến**. Công cụ `clippy` thậm chí có một lint tên `clippy::redundant_closure` chuyên nhắc bạn rút gọn `|x| f(x)` thành `f`.

Nhược điểm: khi lạm dụng, mã trở nên khó đọc kinh khủng (cộng đồng gọi vui là *pointless style* — "lối viết vô nghĩa"). **Quy tắc thực chiến**: dùng point-free khi nó làm mã *rõ hơn*, đừng dùng chỉ vì nó ngắn hơn.

### 7. Ba bộ kết hợp nền tảng (Combinators)

Trong từ điển thuật ngữ lập trình hàm, **bộ kết hợp (combinator)** là một hàm thuần túy không phụ thuộc vào bất kỳ biến nào bên ngoài. Ba bộ kết hợp cơ bản nhất:

| Tên | Chữ ký | Ý nghĩa | Có sẵn trong Rust? |
|---|---|---|---|
| **`identity`** | `T -> T` | Trả về chính đầu vào. Là "phần tử đơn vị" của phép ghép. | ✅ `std::convert::identity` |
| **`const`** | `A -> (B -> A)` | Nuốt tham số thứ hai, luôn trả về giá trị đã khóa sẵn. | ❌ tự viết (trong mã dưới đây đặt tên `constant`, vì `const` là từ khóa của Rust) |
| **`flip`** | `(A, B) -> C` thành `(B, A) -> C` | Đảo thứ tự hai tham số. | ❌ tự viết |

`identity` nghe vô dụng nhưng cực kỳ hữu ích trong thực tế: `data.into_iter().flat_map(identity)` sẽ lọc bỏ toàn bộ `None` khỏi một danh sách `Option` (thư viện chuẩn có sẵn lối tắt cho đúng việc này: `.flatten()`).

---

## Mã nguồn minh họa thực chiến (Idiomatic Runnable Rust Blueprint)

Chương trình dưới đây xây dựng một **Hệ thống Chuẩn hóa & Kiểm duyệt Bình luận (Comment Sanitization Engine)**. Toàn bộ logic được lắp ghép từ những hàm nhỏ xíu, mỗi hàm làm đúng một việc — đúng triết lý "ống nước lắp ren".

```rust
// Tệp: src/main.rs
// Chương trình thực chiến: Ghép hàm, Curry hóa và Áp dụng từng phần trong Rust

use std::collections::HashMap;

// ============================================================================
// PHẦN 1: BỘ CÔNG CỤ GHÉP HÀM (COMPOSITION TOOLKIT)
// ============================================================================

/// Ghép 2 hàm: (A -> B) và (B -> C) thành (A -> C).
/// Đây chính là phép toán `g ∘ f` viết bằng cú pháp Rust.
pub fn compose<A, B, C>(f: impl Fn(A) -> B, g: impl Fn(B) -> C) -> impl Fn(A) -> C {
    move |x| g(f(x))
}

/// Ghép 3 hàm liên tiếp cho tiện dùng.
pub fn compose3<A, B, C, D>(
    f: impl Fn(A) -> B,
    g: impl Fn(B) -> C,
    h: impl Fn(C) -> D,
) -> impl Fn(A) -> D {
    move |x| h(g(f(x)))
}

/// Bộ kết hợp `identity`: phần tử đơn vị của phép ghép hàm.
pub fn identity<T>(x: T) -> T {
    x
}

/// Bộ kết hợp `const`: nuốt tham số, luôn trả về giá trị đã khóa sẵn.
pub fn constant<A: Clone, B>(value: A) -> impl Fn(B) -> A {
    move |_ignored| value.clone()
}

/// Bộ kết hợp `flip`: đảo thứ tự hai tham số của một hàm.
pub fn flip_args<A, B, C>(f: impl Fn(A, B) -> C) -> impl Fn(B, A) -> C {
    move |b, a| f(a, b)
}

// ============================================================================
// PHẦN 2: CÁC HÀM NHỎ THUẦN TÚY — TỪNG "ĐOẠN ỐNG" RIÊNG LẺ
// ============================================================================

/// Cắt bỏ khoảng trắng thừa ở hai đầu.
pub fn trim_whitespace(s: &str) -> String {
    s.trim().to_string()
}

/// Thu gọn nhiều khoảng trắng liên tiếp thành một khoảng trắng duy nhất.
pub fn collapse_whitespace(s: String) -> String {
    s.split_whitespace().collect::<Vec<&str>>().join(" ")
}

/// Viết hoa chữ cái đầu tiên của câu (an toàn với tiếng Việt có dấu).
pub fn capitalize_first(s: String) -> String {
    let mut chars = s.chars();
    match chars.next() {
        None => String::new(),
        Some(first) => first.to_uppercase().collect::<String>() + chars.as_str(),
    }
}

// ============================================================================
// PHẦN 3: CURRY HÓA & ÁP DỤNG TỪNG PHẦN — CÁC "NHÀ MÁY" SINH HÀM
// ============================================================================

/// Dạng thông thường: nhận đủ 2 tham số cùng lúc.
pub fn truncate(limit: usize, s: &str) -> String {
    if s.chars().count() <= limit {
        s.to_string()
    } else {
        let header: String = s.chars().take(limit).collect();
        format!("{}…", header)
    }
}

/// Dạng đã curry hóa: khóa trước `limit`, sinh ra một hàm chuyên dụng.
pub fn truncate_curried(limit: usize) -> impl Fn(&str) -> String {
    move |s: &str| truncate(limit, s)
}

/// Nhà máy sinh bộ lọc từ cấm: khóa sẵn danh sách từ, trả về một vị từ (predicate).
pub fn make_ban_filter(banned_words: Vec<String>) -> impl Fn(&str) -> bool {
    move |text: &str| {
        let lowercase = text.to_lowercase();
        !banned_words
            .iter()
            .any(|word| lowercase.contains(word.as_str()))
    }
}

/// Hạ một ký tự về chữ thường (với tiếng Việt, mỗi chữ hoa ứng đúng một chữ thường).
fn lower_char(c: char) -> char {
    c.to_lowercase().next().unwrap_or(c)
}

/// Nhà máy sinh bộ che từ cấm bằng dấu sao.
/// Không phân biệt hoa/thường — khớp với `make_ban_filter`, để "SPAM" cũng bị che.
pub fn make_censor(banned_words: Vec<String>) -> impl Fn(String) -> String {
    // Chuẩn bị MỘT LẦN lúc tạo bộ che: mỗi từ cấm thành dãy ký tự chữ thường.
    let banned: Vec<Vec<char>> = banned_words
        .iter()
        .map(|word| word.chars().map(lower_char).collect())
        .filter(|word: &Vec<char>| !word.is_empty())
        .collect();
    move |text: String| {
        let original: Vec<char> = text.chars().collect();
        // So khớp trên bản chữ thường, nhưng che trên bản gốc (cùng chỉ số ký tự).
        let lower: Vec<char> = original.iter().copied().map(lower_char).collect();
        let mut masked = original;
        for word in &banned {
            for start in 0..lower.len() {
                if lower[start..].starts_with(word) {
                    masked[start..start + word.len()].fill('*');
                }
            }
        }
        masked.into_iter().collect()
    }
}

// ============================================================================
// PHẦN 4: TIÊM PHỤ THUỘC BẰNG ÁP DỤNG TỪNG PHẦN
// ============================================================================

/// Bản ghi nhật ký kiểm duyệt (thay cho việc ghi ra tệp thật).
#[derive(Debug, Clone, PartialEq)]
pub struct LogRecord {
    pub comment_id: u32,
    pub verdict: String,
}

/// "Phụ thuộc" ở đây là hàm ghi nhật ký. Ta KHÓA nó vào trong bộ kiểm duyệt
/// bằng áp dụng từng phần, thay vì để bộ kiểm duyệt tự đi tìm.
/// `log` phải là `FnMut` vì nó ghi thêm vào sổ sau mỗi lần gọi.
pub fn make_validator<L>(
    check_clean: impl Fn(&str) -> bool,
    sanitize: impl Fn(String) -> String,
    mut log: L,
) -> impl FnMut(u32, &str) -> String
where
    L: FnMut(LogRecord),
{
    move |id: u32, raw: &str| {
        let trimmed = trim_whitespace(raw);
        // Kiểm tra TRƯỚC khi che — nếu che trước thì từ cấm biến mất
        // và bộ kiểm tra sẽ luôn báo "hợp lệ". Thứ tự các bước rất quan trọng!
        let verdict = if check_clean(&trimmed) {
            "HỢP LỆ"
        } else {
            "CHỨA TỪ CẤM — ĐÃ CHE"
        };
        let cleaned = sanitize(trimmed);
        log(LogRecord {
            comment_id: id,
            verdict: verdict.to_string(),
        });
        cleaned
    }
}

// ============================================================================
// CHƯƠNG TRÌNH ĐIỀU HÀNH CHÍNH
// ============================================================================

fn main() {
    println!("============================================================");
    println!("   GHÉP HÀM, CURRY HÓA & ÁP DỤNG TỪNG PHẦN TRONG RUST      ");
    println!("============================================================");

    // ------------------------------------------------------------------
    // 1. LẮP REN ỐNG NƯỚC: ghép 3 hàm nhỏ thành 1 đường ống chuẩn hóa
    // ------------------------------------------------------------------
    let normalize = compose3(trim_whitespace, collapse_whitespace, capitalize_first);

    let raw = "   xin    chào     các bạn  ";
    println!("\n1. GHÉP HÀM (Composition)");
    println!("   Đầu vào thô  : {:?}", raw);
    println!("   Sau đường ống: {:?}", normalize(raw));

    // ------------------------------------------------------------------
    // 2. KIỂM CHỨNG LUẬT KẾT HỢP: h ∘ (g ∘ f) == (h ∘ g) ∘ f
    // ------------------------------------------------------------------
    let way_a = compose(
        compose(trim_whitespace, collapse_whitespace),
        capitalize_first,
    );
    let way_b = compose(
        trim_whitespace,
        compose(collapse_whitespace, capitalize_first),
    );
    assert_eq!(way_a(raw), way_b(raw));
    println!("\n2. LUẬT KẾT HỢP");
    println!("   h∘(g∘f) và (h∘g)∘f cho cùng kết quả: {:?} ✓", way_a(raw));

    // ------------------------------------------------------------------
    // 3. LUẬT ĐƠN VỊ: ghép với `identity` không làm thay đổi gì
    // ------------------------------------------------------------------
    let with_identity = compose(identity::<&str>, &normalize);
    assert_eq!(with_identity(raw), normalize(raw));
    println!("\n3. LUẬT ĐƠN VỊ");
    println!("   identity ∘ f == f  ✓ (kết quả không đổi)");

    // ------------------------------------------------------------------
    // 4. CURRY HÓA: một hàm gốc sinh ra nhiều hàm chuyên dụng
    // ------------------------------------------------------------------
    println!("\n4. CURRY HÓA & ÁP DỤNG TỪNG PHẦN");
    let truncate_10 = truncate_curried(10); // Máy đã khóa núm "10 ký tự"
    let truncate_25 = truncate_curried(25); // Máy đã khóa núm "25 ký tự"

    let sentence = "Rust là ngôn ngữ lập trình hệ thống hiện đại";
    println!("   Bản gốc   : {}", sentence);
    println!("   Cắt còn 10: {}", truncate_10(sentence));
    println!("   Cắt còn 25: {}", truncate_25(sentence));

    // ------------------------------------------------------------------
    // 5. NHÀ MÁY SINH HÀM: cùng một danh sách từ cấm, hai công cụ khác nhau
    // ------------------------------------------------------------------
    let banned_words: Vec<String> = vec!["lừa đảo".to_string(), "spam".to_string()];
    let is_clean = make_ban_filter(banned_words.clone());
    let censor = make_censor(banned_words.clone());

    println!("\n5. NHÀ MÁY SINH HÀM (Closure Factory)");
    let dirty_comment = "Đây là tin spam lừa đảo";
    println!(
        "   {:?} có sạch không? {}",
        dirty_comment,
        is_clean(dirty_comment)
    );
    println!("   Sau khi che: {}", censor(dirty_comment.to_string()));

    // ------------------------------------------------------------------
    // 6. TIÊM PHỤ THUỘC: khóa "bộ ghi nhật ký" vào bộ kiểm duyệt
    // ------------------------------------------------------------------
    println!("\n6. TIÊM PHỤ THUỘC BẰNG ÁP DỤNG TỪNG PHẦN");
    let mut log_book: Vec<LogRecord> = Vec::new();

    {
        // Phụ thuộc thật: ghi vào sổ nhật ký trong bộ nhớ.
        let write_to_log = |record: LogRecord| log_book.push(record);
        let mut validator = make_validator(&is_clean, &censor, write_to_log);

        println!("   #101 -> {}", validator(101, "  Bài viết rất hay!  "));
        println!(
            "   #102 -> {}",
            validator(102, "  Cẩn thận kẻo bị lừa đảo  ")
        );
    }

    println!("   Nhật ký thu được ({} dòng):", log_book.len());
    for record in &log_book {
        println!(
            "     - Bình luận #{}: {}",
            record.comment_id, record.verdict
        );
    }

    // ------------------------------------------------------------------
    // 7. BỘ KẾT HỢP `flip` VÀ `const`
    // ------------------------------------------------------------------
    println!("\n7. BỘ KẾT HỢP flip & const");
    let divide = |a: f64, b: f64| a / b;
    let divide_flipped = flip_args(divide);
    println!("   divide(10, 2)       = {}", divide(10.0, 2.0));
    println!("   flip(divide)(10, 2) = {}", divide_flipped(10.0, 2.0)); // = divide(2, 10)

    let always_zero = constant::<i32, &str>(0);
    println!("   const(0)(\"bất kỳ\") = {}", always_zero("bất kỳ"));

    // ------------------------------------------------------------------
    // 8. `identity` GIÚP LỌC BỎ None — ỨNG DỤNG THỰC TẾ
    // ------------------------------------------------------------------
    let raw_data: Vec<Option<i32>> = vec![Some(1), None, Some(3), None, Some(5)];
    let clean: Vec<i32> = raw_data.into_iter().flat_map(identity).collect();
    println!("\n8. identity LỌC BỎ None: {:?}", clean);
    assert_eq!(clean, vec![1, 3, 5]);

    // ------------------------------------------------------------------
    // 9. GHÉP HÀM QUY MÔ LỚN: xử lý cả một danh sách bình luận
    // ------------------------------------------------------------------
    println!("\n9. ÁP DỤNG ĐƯỜNG ỐNG LÊN TOÀN BỘ DỮ LIỆU");
    let raw_comments = [
        "   rust rất   thú vị  ",
        " cẩn thận trò spam này ",
        "   giáo trình  hay quá   ",
    ];

    let stats: HashMap<bool, usize> =
        raw_comments
            .iter()
            .map(|c| normalize(c))
            .fold(HashMap::new(), |mut table, sentence| {
                *table.entry(is_clean(&sentence)).or_insert(0) += 1;
                table
            });

    for c in raw_comments.iter() {
        println!("   {:?} -> {:?}", c, normalize(c));
    }
    println!("   Thống kê [sạch = true/false]: {:?}", stats);

    println!("\n============================================================");
    println!("      HOÀN TẤT: TỪ HÀM NHỎ LẮP THÀNH HỆ THỐNG LỚN          ");
    println!("============================================================");
}

// ============================================================================
// KIỂM THỬ: BIẾN "LUẬT" THÀNH BÀI TEST CHẠY ĐƯỢC
// ============================================================================

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn composition_is_associative() {
        let samples = ["  a   b ", "Xin   chào", "   rust  "];
        for s in samples {
            let a = compose(
                compose(trim_whitespace, collapse_whitespace),
                capitalize_first,
            );
            let b = compose(
                trim_whitespace,
                compose(collapse_whitespace, capitalize_first),
            );
            assert_eq!(a(s), b(s), "Luật kết hợp bị vi phạm với đầu vào {:?}", s);
        }
    }

    #[test]
    fn composition_has_identity() {
        let f = compose(trim_whitespace, capitalize_first);
        let left = compose(identity::<&str>, &f);
        for s in ["  xin chào ", "rust"] {
            assert_eq!(left(s), f(s));
        }
    }

    #[test]
    fn curried_matches_original() {
        let truncate_15 = truncate_curried(15);
        let sentence = "Rust là ngôn ngữ tuyệt vời";
        assert_eq!(truncate_15(sentence), truncate(15, sentence));
    }

    #[test]
    fn flip_swaps_argument_order() {
        let subtract = |a: i32, b: i32| a - b;
        let flipped_subtract = flip_args(subtract);
        assert_eq!(subtract(10, 3), 7);
        assert_eq!(flipped_subtract(10, 3), -7); // = subtract(3, 10)
    }

    #[test]
    fn generated_closures_are_independent() {
        let filter = make_ban_filter(vec!["spam".to_string()]);
        assert!(filter("bài viết hay"));
        assert!(!filter("đây là SPAM"));
    }

    #[test]
    fn censor_is_case_insensitive_like_the_filter() {
        // Trước đây bộ lọc không phân biệt hoa/thường nhưng bộ che thì có,
        // nên "SPAM" bị báo là từ cấm mà vẫn không bị che.
        let censor = make_censor(vec!["spam".to_string(), "lừa đảo".to_string()]);
        assert_eq!(censor("đây là SPAM".to_string()), "đây là ****");
        assert_eq!(censor("Lừa Đảo nè".to_string()), "******* nè");
        assert_eq!(censor("bài viết hay".to_string()), "bài viết hay");
    }
}
```

---

## Bảng tra cứu lỗi biên dịch & Cách khắc phục (Compiler Error Guide)

| Mã lỗi | Thông báo mẫu từ trình biên dịch | Nguyên nhân cốt lõi | Cách khắc phục nhanh |
|---|---|---|---|
| **E0373** | `closure may outlive the current function, but it borrows '...'` | Bạn trả về một closure từ hàm nhưng closure đó chỉ *mượn* các biến cục bộ. Khi hàm kết thúc, biến chết, closure trở thành con trỏ lơ lửng. | Thêm từ khóa `move` trước dấu `\|`. Đây là lỗi số 1 khi tự viết hàm `compose`. |
| **E0631** | `type mismatch in function arguments` (kèm `expected function signature 'fn(String) -> _', found 'fn(i32) -> _'`) | "Cỡ ren không khớp": đầu ra của hàm thứ nhất không cùng kiểu với đầu vào của hàm thứ hai (với closure, thông báo là `type mismatch in closure arguments`). | Kiểm tra lại chữ ký hai hàm. Chèn một hàm chuyển đổi (`.to_string()`, `.as_str()`, `From::from`) vào giữa để khớp ren. |
| **E0507** | `cannot move out of '...', a captured variable in an 'Fn' closure` | Closure trả về từ nhà máy được đánh dấu `Fn` (gọi nhiều lần) nhưng bên trong bạn lại chuyển quyền sở hữu biến đã bắt giữ ra ngoài — lần gọi thứ hai sẽ không còn gì. | Dùng `.clone()` bên trong closure (như hàm `constant` ở trên), hoặc chỉ mượn tham chiếu `&`. |
| **E0525** | `expected a closure that implements the 'Fn' trait, but this closure only implements 'FnMut'` | Closure của bạn thay đổi trạng thái bên ngoài (ví dụ ghi vào sổ nhật ký) nên nó là `FnMut`, không phải `Fn`. | Đổi ràng buộc thành `FnMut` và đánh dấu biến closure là `mut` — xem hàm `make_validator` ở trên. |
| **E0562** | `'impl Trait' is not allowed in field types` | Bạn viết `impl Fn(...)` làm kiểu của một trường `struct`. | Dùng tham số generic `struct S<F: Fn()> { f: F }`, hoặc `Box<dyn Fn(...)>`. |
| **E0658** | `'impl Trait' in type aliases is unstable` | Bạn viết `type F = impl Fn(...);` — tính năng này (TAIT) chưa ổn định trên Rust stable. | Như trên: dùng tham số generic hoặc `Box<dyn Fn(...)>`; hoặc viết thẳng `impl Fn(...)` ở vị trí tham số/kiểu trả về của hàm. |
| **E0282** | `type annotations needed` | Gọi hàm generic như `identity` hoặc `constant` mà trình biên dịch không suy ra được kiểu (ví dụ `let z = constant(0);` — không ai cho biết `B` là gì). | Chỉ định tường minh bằng cú pháp cá voi (turbofish): `identity::<&str>`, `constant::<i32, &str>(0)`. |

### Phân tích lỗi thực tế `E0373` (quên `move` khi trả về closure):

```rust
// ❌ Đoạn mã lỗi minh họa (đã đóng chú thích để tệp vẫn biên dịch được):
// fn make_multiplier_wrong(factor: i32) -> impl Fn(i32) -> i32 {
//     |x| x * factor
//     // LỖI E0373: closure may outlive the current function,
//     //            but it borrows `factor`, which is owned by the current function
// }

// ✅ Cách sửa: thêm `move` để closure ĐOẠT quyền sở hữu `factor`
fn make_multiplier(factor: i32) -> impl Fn(i32) -> i32 {
    move |x| x * factor
}
```

---

## Tóm tắt chương & Bài tập rèn luyện (Summary & Exercises)

### 4 Điểm cốt lõi cần ghi nhớ:
1. **Ghép hàm là trụ cột thứ hai của FP**: hai hàm `A -> B` và `B -> C` luôn ghép được thành `A -> C`. Điều kiện duy nhất là "cỡ ren" (kiểu dữ liệu) phải khớp — và `rustc` là người kiểm tra ren.
2. **Phép ghép tuân hai luật**: *kết hợp* (`h∘(g∘f) = (h∘g)∘f`) và *đơn vị* (`f∘id = id∘f = f`). Hai luật này chính là định nghĩa của một **Phạm trù** — nền móng cho Chương 18 và 19.
3. **Curry hóa biến hàm nhiều tham số thành nhà máy sinh hàm**. Rust không tự động curry hóa như Haskell; bạn viết tay bằng closure trả về closure kèm `move`.
4. **Áp dụng từng phần chính là Tiêm phụ thuộc**: khóa sẵn phụ thuộc (máy chủ, bộ ghi nhật ký, cấu hình) vào closure, giữ lại tham số nghiệp vụ. Nhờ vậy kiểm thử không cần bất kỳ framework giả lập nào.

### Bài tập rèn luyện tự giải:

**Bài tập 1 (Tự xây `compose4`)**
Viết hàm `compose4` ghép bốn hàm liên tiếp `A -> B -> C -> D -> E`. Sau đó dùng nó để xây dựng đường ống xử lý mã sản phẩm: cắt khoảng trắng → viết hoa toàn bộ → thêm tiền tố `"SP-"` → cắt còn tối đa 12 ký tự.

<details>
<summary><b>Gợi ý</b></summary>

Bạn có hai lựa chọn: viết thẳng `move |x| k(h(g(f(x))))`, hoặc tận dụng những gì đã có — `compose4(f,g,h,k)` chính là `compose(compose3(f,g,h), k)`. Nhớ `move`, và nhớ rằng mỗi tham số `impl Fn(...)` là một kiểu generic ẩn danh riêng biệt.
</details>

<details>
<summary><b>Lời giải</b></summary>

```rust
pub fn compose4<A, B, C, D, E>(
    f: impl Fn(A) -> B,
    g: impl Fn(B) -> C,
    h: impl Fn(C) -> D,
    k: impl Fn(D) -> E,
) -> impl Fn(A) -> E {
    move |x| k(h(g(f(x))))
}

fn main() {
    let pipeline = compose4(
        |s: &str| s.trim().to_string(),
        |s: String| s.to_uppercase(),
        |s: String| format!("SP-{}", s),
        |s: String| s.chars().take(12).collect::<String>(),
    );
    assert_eq!(pipeline("  ban phim co  "), "SP-BAN PHIM ");
    println!("{:?}", pipeline("  ban phim co  "));
}
```
</details>

**Bài tập 2 (Nhà máy sinh bộ kiểm tra)**
Viết hàm `make_range_check(min: i64, max: i64) -> impl Fn(i64) -> Result<i64, String>` trả về một hàm kiểm tra số có nằm trong khoảng `[min, max]` hay không. Nếu hợp lệ trả `Ok(value)`, nếu không trả `Err` kèm thông báo tiếng Việt rõ ràng. Dùng nó để tạo hai bộ kiểm tra: `check_age` (0–120) và `check_score` (0–10).

<details>
<summary><b>Gợi ý</b></summary>

Đây là bài tập về *áp dụng từng phần*: `min` và `max` bị khóa vào closure bằng `move`, còn `value` là tham số để lại cho lúc gọi. Vì `i64` là kiểu `Copy` nên bạn không cần `.clone()`.
</details>

<details>
<summary><b>Lời giải</b></summary>

```rust
pub fn make_range_check(min: i64, max: i64) -> impl Fn(i64) -> Result<i64, String> {
    move |value: i64| {
        if (min..=max).contains(&value) {
            Ok(value)
        } else {
            Err(format!("Giá trị {} nằm ngoài khoảng cho phép [{}, {}]", value, min, max))
        }
    }
}

fn main() {
    let check_age = make_range_check(0, 120);
    let check_score = make_range_check(0, 10);

    assert_eq!(check_age(35), Ok(35));
    assert!(check_age(500).is_err());
    assert_eq!(check_score(9), Ok(9));
    println!("{:?}", check_score(11));
    // Err("Giá trị 11 nằm ngoài khoảng cho phép [0, 10]")
}
```
</details>

**Bài tập 3 (Tư duy thiết kế)**
Trong Chương 13 bạn đã học rằng hàm thuần túy luôn cho cùng kết quả với cùng đầu vào. Hãy giải thích: *vì sao chỉ có hàm thuần túy mới ghép nối được một cách an toàn?* Nêu một ví dụ cụ thể trong đó việc ghép hai hàm **không** thuần túy dẫn đến kết quả sai lệch.

<details>
<summary><b>Gợi ý</b></summary>

Hãy nghĩ tới hai hàm cùng đọc/ghi một biến toàn cục, hoặc một hàm đọc đồng hồ hệ thống. Khi đó `g(f(x))` chạy ở hai thời điểm khác nhau có thể cho kết quả khác nhau — chữ ký hàm không còn kể hết câu chuyện nữa.
</details>

<details>
<summary><b>Lời giải tham khảo</b></summary>

Phép ghép hàm dựa trên một giả định ngầm: **giá trị trả về của `f` là toàn bộ những gì `f` tạo ra**. Nếu `f` còn âm thầm sửa một biến toàn cục hay ghi tệp, thì `g(f(x))` mang theo một "kênh dữ liệu ẩn" mà chữ ký hàm không hề nói tới. Hệ quả:

- Bạn không thể thay `f(x)` bằng giá trị nó trả về (mất tính minh bạch tham chiếu ở Chương 13), nên không thể suy luận về đường ống bằng đẳng thức.
- Bạn không thể kiểm thử `compose(f, g)` một cách độc lập, vì kết quả phụ thuộc trạng thái ngoài.
- Bạn không thể chạy song song, vì hai luồng cùng đụng vào trạng thái ẩn đó.

Ví dụ cụ thể:

```rust
static mut COUNTER: i32 = 0;

fn increment_and_get(_: ()) -> i32 {
    unsafe { COUNTER += 1; COUNTER }   // KHÔNG thuần túy
}
```

Hàm `increment_and_get` gọi lần 1 trả `1`, lần 2 trả `2`. Một đường ống gọi nó hai lần sẽ cho kết quả khác nhau mỗi lần chạy, và nếu chạy đa luồng còn phát sinh tranh chấp dữ liệu (data race). Đây chính là lý do Rust bắt buộc phải dùng `unsafe` mới đụng được vào `static mut` — ngôn ngữ đang chủ động cản bạn phá vỡ tính ghép nối.
</details>
