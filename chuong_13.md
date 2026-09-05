# Chương 13: Lập trình hàm là gì? Bất biến, Minh bạch tham chiếu và Hàm toàn phần (Introduction to FP: Immutability, Referential Transparency & Total Functions)

## Giới thiệu & Mục tiêu học tập

Chào mừng bạn bước sang **Chủ đề 3: Lập trình hàm (Functional Programming - FP)** của khóa học Rust Masterclass! Trong 12 chương đầu tiên, bạn đã xây dựng nền móng vững chắc về tư duy hệ thống: từ cách các bóng bán dẫn và thanh RAM lưu trữ byte dữ liệu, cho đến ba trụ cột độc nhất vô nhị của Rust: quyền sở hữu (ownership), vay mượn (borrow), và thời gian sống (lifetime). Bạn cũng đã biết cách dùng `struct`, `enum`, và `trait` để đóng gói dữ liệu và giao ước hành vi.

Tuy nhiên, khi bắt tay vào xử lý dữ liệu phức tạp trong thế giới thực — chẳng hạn như lọc danh sách hàng hóa trong kho, tính toán tiền thanh toán đơn hàng, chuẩn hóa chuỗi ký tự hay tổng hợp báo cáo doanh thu — bạn sẽ nhận thấy lối viết mã truyền thống (lập trình mệnh lệnh) thường đi kèm với:
1. Quá nhiều biến số tạm thời (`mut`) thay đổi liên tục, làm tăng nguy cơ phát sinh lỗi ngầm (bugs) do quên khởi tạo lại giá trị hoặc gán đè sai vị trí.
2. Vòng lặp lồng nhau sâu hoắm (`for`, `while`) khiến người đọc mất phương hướng khi lần theo dấu vết logic.
3. Mã nguồn trở nên dài dòng, cồng kềnh và khó kiểm thử độc lập.

Lập trình hàm (Functional Programming) là một trường phái tư duy lập trình đưa chúng ta tiếp cận bài toán theo một góc nhìn hoàn toàn mới: **Xem chương trình như một chuỗi các phép biến đổi toán học thuần túy trên dữ liệu bất biến (immutable data)**, thay vì chuỗi các mệnh lệnh xáo trộn trạng thái bộ nhớ. Rust không phải là một ngôn ngữ lập trình hàm thuần túy như Haskell, nhưng Rust được thiết kế để kế thừa những compute hoa tuyệt vời nhất của lập trình hàm, kết hợp hài hòa với tốc độ thực thi thần tốc và quyền kiểm soát tài nguyên phần cứng trực tiếp.

Mục tiêu học tập của chương này:
- Nắm rõ sự khác biệt cốt lõi giữa **Lập trình mệnh lệnh (Imperative Programming)** và **Lập trình khai báo (Declarative Programming)**.
- Thấu hiểu khái niệm **Hàm thuần túy (Pure Functions)** và tại sao việc triệt tiêu tác dụng phụ (Side-Effects) lại giúp mã nguồn ổn định tuyệt đối.
- Hiểu sâu sắc lý do Rust chọn triết lý **Bất biến mặc định (Immutability by Default)** và cách nó bảo vệ an toàn luồng dữ liệu.
- Phân biệt sự ưu việt của **Biểu thức (Expressions)** so với Câu lệnh (Statements).
- Làm quen với mô hình **Đường ống biến đổi dữ liệu (Data Pipelines)** thông qua các phương thức biến đổi ban đầu.

---

## Hình tượng hóa đời sống (Intuitive Everyday Analogy)

Để những khái niệm trên không còn là lý thuyết trừu tượng, chúng ta hãy cùng quan sát hai hình ảnh vô cùng gần gũi trong đời sống hằng ngày:

```
┌──────────────────────────────────────────────────────────────────────────────────┐
│              SO SÁNH HAI TRƯỜNG PHÁI: MỆNH LỆNH VS KHAI BÁO ĐƯỜNG ỐNG             │
├────────────────────────────────────────┬─────────────────────────────────────────┤
│    CÁCH MỆNH LỆNH (IMPERATIVE)         │       CÁCH KHAI BÁO ĐƯỜNG ỐNG (FP)      │
│     "Tự tay vào bếp làm bánh mì"      │          "Dây chuyền lọc nước RO"       │
│                                        │                                         │
│ 1. Lấy thau nhôm từ ngăn dưới tủ       │ [Nguồn nước giếng khoan thô]            │
│ 2. Đong 500g bột mì đổ vào thau        │     │                                   │
│ 3. Đập 2 quả trứng gà, quấy 40 vòng    │     ▼ [Lõi lọc 1: Lọc cặn bẩn thô]      │
│ 4. Bật lò nướng ở 180 độ C             │     │ (Loại bỏ bùn cát)                 │
│ 5. Đút thau bột vào lò, hẹn 30 phút    │     ▼ [Lõi lọc 2: Lõi than hoạt tính]   │
│ 6. Ngồi canh chừng lò nướng            │     │ (Khử clo và mùi hôi)              │
│ -> Tự tay làm từng bước thủ công.      │     ▼ [Lõi lọc 3: Màng thẩm thấu RO]    │
│    Sai một thao tác nhỏ là hỏng bánh!  │     │ (Chỉ cho phân tử nước đi qua)     │
│                                        │     ▼                                   │
│                                        │ [Ly nước khoáng compute khiết uống ngay]   │
│                                        │ -> Nước chảy liên tục qua đường ống.    │
│                                        │    Không làm bẩn phòng, không biến tạm! │
└────────────────────────────────────────┴─────────────────────────────────────────┘
```

### 1. Tự làm bánh mì thủ công vs Thực khách gọi món tại nhà hàng
- **Phong cách Mệnh lệnh (Imperative - Làm thế nào?)**: Giống như việc bạn đích thân bước vào bếp làm bánh:
  - Bạn phải tự tay lấy bát, tự tay đong từng gam bột, tự tay bật que đánh trứng, liên tục nhìn đồng hồ canh lửa lò nướng.
  - Bạn phải tự quản lý từng chiếc tô, dọn rửa từng chiếc thìa (tương đương với các biến tạm `mut` trong bộ nhớ RAM). Nếu lơ là quên tắt lửa hoặc bỏ nhầm gia vị, toàn bộ chiếc bánh sẽ bị hỏng.
- **Phong cách Khai báo (Declarative - Cần kết quả gì?)**: Giống như khi bạn là thực khách bước vào một nhà hàng sang trọng:
  - Bạn ngồi vào bàn, mở thực đơn và gọi: *"Cho tôi một phần cá hồi nướng bơ tỏi ăn kèm salad dầu giấm"*.
  - Bạn không cần quan tâm đầu bếp đứng ở góc nào, chảo rán lật mấy lần, lửa vặn bao nhiêu độ. Bạn chỉ tuyên bố **kết quả cuối cùng mong muốn**, và nhà bếp chuyên nghiệp sẽ đảm bảo đưa ra món ăn đúng chuẩn.

### 2. Dây chuyền hệ thống máy lọc nước gia đình (Data Pipeline)
Hãy quan sát cách chiếc máy lọc nước RO trong ngôi nhà bạn vận hành:
- Nước giếng khoan ban đầu đi vào từ đầu ống. Nó chảy qua **Lõi 1 (Lọc chặn thô)** để giữ lại cát sỏi. Nước chảy tiếp qua **Lõi 2 (Than hoạt tính)** để hút sạch hóa chất độc hại. Nước chảy tiếp qua **Lõi 3 (Màng siêu lọc RO)** để loại bỏ vi khuẩn. Cuối cùng, nước chảy ra vòi là nước khoáng compute khiết.
- Điểm đặc biệt của dây chuyền này là:
  - Nguồn nước chảy tuần tự qua từng trạm xử lý chuyên biệt (tương đương các hàm trong chuỗi `pipeline`).
  - Mỗi lõi lọc chỉ tập trung làm đúng một việc duy nhất và không làm ảnh hưởng đến lõi lọc khác.
  - Bạn không cần phải múc nước ra xô, rồi lại đổ từ xô này sang chậu khác (không cần vùng đệm `buffer` tạm thời dư thừa làm tốn ô nhớ).

---

## Khái niệm & Cơ chế kỹ thuật chuyên sâu (In-Depth Technical Mechanics)

### 1. Khác biệt kỹ thuật giữa Lập trình Mệnh lệnh và Lập trình Khai báo

Trong lập trình mệnh lệnh truyền thống (như C, C++, Python cơ bản), lập trình viên tập trung vào việc **thay đổi trạng thái bộ nhớ theo thời gian**:
```rust
// Phong cách mệnh lệnh (Imperative)
let mut tong = 0;
let mut i = 0;
while i < list.len() {
    if list[i] > 10 {
        tong += list[i];
    }
    i += 1;
}
```
Ở đoạn mã trên, có đến 2 biến mang cờ `mut`: `tong` và `i`. Bộ não của lập trình viên phải hoạt động như một bộ vi xử lý CPU thu nhỏ để theo dõi: "Ở vòng 1, `i` là mấy? `tong` là bao nhiêu? Có bị tràn chỉ mục ngoài mảng (index out of bounds) hay không?".

Ngược lại, trong phong cách lập trình hàm (khai báo):
```rust
// Phong cách khai báo đường ống (Declarative)
let tong: i32 = list.iter()
    .filter(|&&x| x > 10)
    .sum();
```
Mã nguồn giờ đây là một lời tuyên bố rõ ràng: "Hãy lọc lấy các phần tử lớn hơn 10, sau đó tính tổng của chúng". Không có biến `mut`, không có chỉ mục `i`, không có bất kỳ rủi ro nào về việc truy cập bộ nhớ ngoài biên!

### 2. Hàm thuần túy (Pure Functions) và Tác dụng phụ (Side-Effects)

Một hàm được gọi là **Hàm thuần túy (Pure Function)** nếu nó thỏa mãn đồng thời hai điều kiện khắt khe:
1. **Tính tất định (Deterministic)**: Với cùng một dữ liệu đầu vào, hàm luôn luôn trả về cùng một kết quả duy nhất, dù bạn gọi nó một lần hay một triệu lần.
2. **Không có tác dụng phụ (No Side-Effects)**: Hàm không âm thầm thay đổi bất kỳ trạng thái nào bên ngoài phạm vi cục bộ của nó. Nó không sửa biến toàn cục, không ghi đè vào tham chiếu mượn (borrow), không tự ý ghi tệp ra ổ đĩa, và không in dữ liệu lung tung ra màn hình nếu không được yêu cầu.

```
┌────────────────────────────────────────────────────────────────────────┐
│                   HÀM THUẦN TÚY (PURE FUNCTION)                        │
│                                                                        │
│   Đầu vào (Inputs) ────────► [ HỘP XỬ LÝ KHÉP KÍN ] ────────► Đầu ra   │
│   (Không đổi)                (Không chạm vào RAM,                     │
│                               Không sửa môi trường ngoài)              │
└────────────────────────────────────────────────────────────────────────┘
```

Tại sao Rust lại yêu thích hàm thuần túy?
- **Khả năng kiểm thử (Testability)**: Bạn có thể viết kiểm thử đơn vị (unit test) cực kỳ đơn giản vì không cần chuẩn bị môi trường giả lập (mocking).
- **An toàn song song (Thread Safety)**: Các hàm thuần túy chỉ đọc dữ liệu và trả về giá trị mới, không tranh chấp tài nguyên, giúp việc xử lý đa luồng diễn ra an toàn tuyệt đối mà không cần dùng khóa mutex cồng kềnh.
  > Đây không phải lời hứa suông. Ở **Chương 16** bạn sẽ thấy: với một đường ống toàn hàm thuần túy, chỉ cần đổi `.iter()` thành `.par_iter()` là chương trình chạy song song trên toàn bộ nhân CPU — và trình biên dịch bảo đảm không có tranh chấp dữ liệu. Đổi đúng **một** từ. Đó là phần thưởng cụ thể nhất mà tính thuần túy mang lại.

### 3. Tính bất biến (Immutability by Default) trong Rust

Trong nhiều ngôn ngữ khác, biến số mặc định có thể bị sửa đổi tùy tiện. Một hàm ở tầng sâu có thể nhận danh sách của bạn và âm thầm xóa bớt một phần tử, gây ra lỗi nghiêm trọng ở một mô-đun hoàn toàn khác.

Trong Rust, mọi biến khai báo bằng `let` đều **mặc định là bất biến**. Trình biên dịch `rustc` đóng vai trò người gác cổng:
- Nếu bạn muốn sửa đổi, bạn phải chủ động gắn từ khóa `mut`.
- Nếu bạn truyền một tham chiếu đọc `&T`, bạn trao quyền xem mà không trao quyền sửa.
- Nhờ vậy, khi một biến đi vào đường ống xử lý của lập trình hàm, bạn có sự đảm bảo chắc chắn rằng dữ liệu gốc ban đầu vẫn nguyên vẹn 100%.

### 4. Minh bạch tham chiếu (Referential Transparency) — trụ cột thứ nhất

Đây là cái tên chính thức của tính chất mà hàm thuần túy mang lại, và là **một trong hai trụ cột** mà cộng đồng lập trình hàm quốc tế đặt ở đầu mọi giáo trình (trụ cột thứ hai là *phép ghép hàm*, học ở Chương 14).

> **Định nghĩa**: một biểu thức là *minh bạch tham chiếu* nếu bạn có thể **thay nó bằng chính giá trị nó trả về** mà chương trình không đổi nghĩa.

```rust
let x = to_money(&queue);   // giả sử trả về 90.0
let tong = x + x;                 // 180.0

// Nếu `to_money` là hàm thuần túy, ba dòng dưới đây HOÀN TOÀN tương đương:
let tong = to_money(&queue) + to_money(&queue);
let tong = 90.0 + 90.0;
let tong = 180.0;
```

Vì sao điều này quan trọng đến vậy? Vì nó chính là **giấy phép để bạn tái cấu trúc mã nguồn**. Mỗi khi bạn tách một hàm dài thành hai hàm nhỏ, gộp hai biến thành một, hay để trình biên dịch nhớ tạm một kết quả — bạn đang dựa vào tính minh bạch tham chiếu mà có thể chưa biết tên nó. Cách suy luận bằng cách thay thế biểu thức như trên gọi là **suy luận bằng đẳng thức (equational reasoning)**.

Ngược lại, những thứ sau **phá vỡ** tính chất này: đọc đồng hồ hệ thống (`Instant::now()`), sinh số ngẫu nhiên, đọc biến toàn cục có thể thay đổi, đọc tệp. Gọi hai lần cho hai kết quả khác nhau, nên **không thể thay bằng giá trị**.

> **Liên hệ với Rust**: từ khóa `const fn` đánh dấu những hàm thuần túy đến mức trình biên dịch tính được kết quả ngay lúc biên dịch. Còn thuộc tính `#[must_use]` là lời nhắc rằng "hàm này thuần túy — nếu bạn vứt kết quả đi thì nó chẳng làm gì cả".

### 5. Hàm toàn phần và Hàm bộ phận (Total vs Partial Functions)

Đây là khái niệm có giá trị thực dụng cao nhất trong chương, và nó giải thích **vì sao Rust ép bạn dùng `Option` và `Result`**.

- **Hàm toàn phần (total function)**: với **mọi** giá trị đầu vào hợp lệ về kiểu, hàm đều trả về một kết quả. Không sập, không treo.
- **Hàm bộ phận (partial function)**: có những đầu vào mà hàm **không có câu trả lời** — nó sập, panic, hoặc trả về giá trị vô nghĩa.

Chữ ký hàm thường **nói dối** về điều này:

```rust
fn chia(a: i32, b: i32) -> i32 { a / b }
```

Chữ ký hứa hẹn: "đưa tôi hai `i32` bất kỳ, tôi trả về một `i32`". Nhưng `chia(10, 0)` thì sập chương trình. Đây là **hàm bộ phận đội lốt hàm toàn phần** — loại hàm nguy hiểm nhất.

Có đúng **hai cách** biến một hàm bộ phận thành hàm toàn phần:

| Cách | Ý tưởng | Ví dụ trong Rust |
|---|---|---|
| **Mở rộng đầu ra** | Thêm một "chỗ" cho trường hợp không có câu trả lời | `fn chia(a: i32, b: i32) -> Option<i32>` |
| **Thu hẹp đầu vào** | Làm cho đầu vào xấu không thể biểu diễn được | `fn chia(a: i32, b: NonZeroI32) -> i32` |

Cách thứ hai mạnh hơn nhiều — và nó chính là chủ đề trung tâm của Chương 20.

Bảng dưới đây liệt kê các "hàm bộ phận đội lốt" phổ biến nhất trong Rust và bản toàn phần tương ứng:

| Hàm bộ phận (có thể sập) | Bản toàn phần (an toàn) |
|---|---|
| `v[i]` | `v.get(i) -> Option<&T>` |
| `.unwrap()` / `.expect()` | `match`, `unwrap_or`, `unwrap_or_else`, toán tử `?` |
| `a / b`, `a % b` | `a.checked_div(b) -> Option<i32>` |
| `a + b` (tràn số ở bản release) | `a.checked_add(b)`, `saturating_add`, `wrapping_add` |
| `s[0..5]` trên chuỗi UTF-8 | `s.get(0..5) -> Option<&str>` |

> **Quy tắc thực chiến**: mỗi lần bạn gõ `.unwrap()`, bạn đang biến một hàm toàn phần thành hàm bộ phận. Hãy tự hỏi: *"tôi có chứng minh được trường hợp này không bao giờ xảy ra không?"* Nếu không chứng minh được, đừng dùng `.unwrap()`.

### 6. Triết lý "Mọi thứ đều là Biểu thức" (Expression-Oriented)

Rust là ngôn ngữ định hướng biểu thức (Expression-Oriented Language):
- **Câu lệnh (Statement)**: Là hành động kết thúc bằng dấu chấm phẩy `;`, thực hiện một tác vụ nhưng không sinh ra giá trị (trả về kiểu rỗng unit `()`).
- **Biểu thức (Expression)**: Tính toán và trực tiếp sinh ra một giá trị trả về. Không kết thúc bằng dấu chấm phẩy ở dòng cuối cùng của khối lệnh `{}`.

Khối `if/else`, khối so khớp `match`, và thậm chí khối mã `{ ... }` trong Rust đều là biểu thức. Điều này cho phép lập trình viên gắn kết trực tiếp kết quả vào một biến bất biến mà không cần tạo biến rỗng rồi gán giá trị sau đó:

```rust
// Khởi tạo biến bất biến trực tiếp từ biểu thức rẽ nhánh
let state = if diem_so >= 50 { "Đạt" } else { "Thi lại" };
```

---

## Mã nguồn minh họa thực chiến (Idiomatic Runnable Rust Blueprint)

Dưới đây là một chương trình hoàn chỉnh, minh họa bài toán thực tế: **Hệ thống Xử lý Hóa đơn Bán lẻ (Retail Invoice Processing Pipeline)**. Chương trình so sánh trực diện giữa cách viết mệnh lệnh truyền thống và cách viết theo phong cách lập trình hàm khai báo đường ống trong Rust.

```rust
// Tệp: src/main.rs
// Chương trình minh họa tư duy Lập trình hàm và Xây dựng Đường ống (Data Pipelines) trong Rust

#[derive(Debug, Clone, PartialEq)]
pub struct MatQueue {
    pub ma_san_pham: String,
    pub name_queue: String,
    pub don_price: f64,
    pub quantity: u32,
    pub is_paid: bool,
}

// ============================================================================
// HÀM THUẦN TÚY (PURE FUNCTIONS) - KHÔNG TÁC DỤNG PHỤ
// ============================================================================

/// Hàm thuần túy: Tính thành tiền của một mặt hàng
/// Nhận dữ liệu đầu vào và trả về giá trị mới, không thay đổi bất kỳ trạng thái nào
pub fn to_money(queue: &MatQueue) -> f64 {
    queue.don_price * (queue.quantity as f64)
}

/// Hàm thuần túy: Áp dụng phiếu giảm giá tỷ lệ phần trăm
pub fn apply_down_price(tien_goc: f64, phan_tram_giam: f64) -> f64 {
    if phan_tram_giam <= 0.0 {
        tien_goc
    } else if phan_tram_giam >= 100.0 {
        0.0
    } else {
        tien_goc * (1.0 - (phan_tram_giam / 100.0))
    }
}

// ============================================================================
// SO SÁNH HAI CÁCH TIẾP CẬN TRÊN DỮ LIỆU
// ============================================================================

/// CÁCH 1: Phong cách Mệnh lệnh (Imperative)
/// Dùng vòng lặp thủ công, biến cờ mut tạm thời, dễ xảy ra lỗi ngoài ý muốn
pub fn xu_ly_menh_lenh(list: &[MatQueue]) -> (f64, Vec<String>) {
    let mut tong_doanh_thu: f64 = 0.0;
    let mut list_name: Vec<String> = Vec::new();

    // Vòng lặp thủ công với nhiều bước điều kiện lồng nhau
    for i in 0..list.len() {
        let queue = &list[i];
        // Chỉ xử lý các đơn hàng đã thanh toán và có giá trị trên 50.0
        if queue.is_paid {
            let into_tien = to_money(queue);
            if into_tien >= 50.0 {
                tong_doanh_thu += into_tien;
                list_name.push(queue.name_queue.clone());
            }
        }
    }

    (tong_doanh_thu, list_name)
}

/// CÁCH 2: Phong cách Lập trình Hàm Khai báo (Declarative Pipeline)
/// Dữ liệu chảy qua chuỗi lọc và ánh xạ, không dùng biến mut nào trong quá trình xử lý!
pub fn handle_declaration(list: &[MatQueue]) -> (f64, Vec<String>) {
    // 1. Nhánh tính tổng doanh thu thông qua đường ống (Pipeline)
    let tong_doanh_thu: f64 = list
        .iter()
        .filter(|queue| queue.is_paid)             // Bước 1: Lọc hàng đã trả tiền
        .map(|queue| to_money(queue))             // Bước 2: Chuyển đổi thành tiền
        .filter(|&tien| tien >= 50.0)                  // Bước 3: Chỉ lấy món từ 50k trở lên
        .sum();                                        // Bước 4: Gom tụ tính tổng

    // 2. Nhánh trích xuất danh sách tên mặt hàng
    let list_name: Vec<String> = list
        .iter()
        .filter(|queue| queue.is_paid && to_money(queue) >= 50.0)
        .map(|queue| queue.name_queue.clone())             // Ánh xạ sang chuỗi tên
        .collect();                                    // Gom vào vector mới

    (tong_doanh_thu, list_name)
}

fn main() {
    println!("============================================================");
    println!("  HỆ THỐNG XỬ LÝ HÓA ĐƠN: LẬP TRÌNH MỆNH LỆNH VS ĐƯỜNG ỐNG  ");
    println!("============================================================");

    // Khởi tạo tập dữ liệu ban đầu bất biến
    let gio_hang: Vec<MatQueue> = vec![
        MatQueue {
            ma_san_pham: String::from("SP-01"),
            name_queue: String::from("Sổ tay Lập trình Rust"),
            don_price: 45.0,
            quantity: 2,
            is_paid: true, // Thành tiền = 90.0 (Thỏa mãn >= 50)
        },
        MatQueue {
            ma_san_pham: String::from("SP-02"),
            name_queue: String::from("Bút bi kỹ thuật"),
            don_price: 15.0,
            quantity: 1,
            is_paid: true, // Thành tiền = 15.0 (Bị loại do < 50)
        },
        MatQueue {
            ma_san_pham: String::from("SP-03"),
            name_queue: String::from("Bàn phím cơ không dây"),
            don_price: 120.0,
            quantity: 1,
            is_paid: false, // Chưa thanh toán (Bị loại)
        },
        MatQueue {
            ma_san_pham: String::from("SP-04"),
            name_queue: String::from("Chuột công thái học"),
            don_price: 75.0,
            quantity: 1,
            is_paid: true, // Thành tiền = 75.0 (Thỏa mãn >= 50)
        },
    ];

    println!("Tổng số mặt hàng đưa vào xử lý: {}", gio_hang.len());

    // 1. Chạy theo phong cách mệnh lệnh
    let (doanh_thu_1, ten_1) = xu_ly_menh_lenh(&gio_hang);
    println!("\n[Kết quả Mệnh lệnh]:");
    println!("- Tổng doanh thu đạt chuẩn : {:.2} nghìn đồng", doanh_thu_1);
    println!("- Danh sách mặt hàng hợp lệ: {:?}", ten_1);

    // 2. Chạy theo phong cách khai báo đường ống
    let (doanh_thu_2, ten_2) = handle_declaration(&gio_hang);
    println!("\n[Kết quả Khai báo Đường ống]:");
    println!("- Tổng doanh thu đạt chuẩn : {:.2} nghìn đồng", doanh_thu_2);
    println!("- Danh sách mặt hàng hợp lệ: {:?}", ten_2);

    // Xác thực hai cách tiếp cận cho ra cùng một kết quả nhất quán
    assert_eq!(doanh_thu_1, doanh_thu_2);
    assert_eq!(ten_1, ten_2);

    // Minh họa hàm thuần túy tính chiết khấu khuyến mãi độc lập
    let total_next_down = apply_down_price(doanh_thu_2, 10.0); // Giảm giá 10%
    println!("\n-> Doanh thu sau khi áp dụng phiếu giảm giá 10%: {:.2} nghìn đồng", total_next_down);
    println!("============================================================");
}
```

---

## Bảng tra cứu lỗi biên dịch & Cách khắc phục (Compiler Error Guide)

Dưới đây là các lỗi biên dịch điển hình nhất mà người học thường gặp phải khi chuyển từ tư duy lập trình mệnh lệnh sang lập trình hàm trong Rust:

| Mã lỗi | Thông báo mẫu từ trình biên dịch | Nguyên nhân cốt lõi | Cách khắc phục nhanh |
|---|---|---|---|
| **E0384** | `cannot assign twice to immutable variable` | Bạn cố tình gán đè giá trị mới lên một biến bất biến khai báo bằng `let`. Tư duy biến đổi trạng thái của lập trình mệnh lệnh đang chi phối. | Cân nhắc chuyển sang phong cách biểu thức hoặc trả về giá trị mới. Nếu bắt buộc phải thay đổi trạng thái, thêm từ khóa `mut` (`let mut ...`). |
| **E0596** | `cannot borrow '...' as mutable, as it is not declared as mutable` | Bạn cố gọi một phương thức sửa đổi dữ liệu (như `.push()`) trên một tập hợp bất biến trong thân hàm. | Khai báo lại biến với `let mut` hoặc chuyển sang sử dụng các hàm bộ điều hợp không làm biến đổi dữ liệu gốc như `.map()` hay `.filter()`. |
| **E0308** | `mismatched types: expected 'bool', found '()'` | Bạn đưa một câu lệnh kết thúc bằng dấu chấm phẩy `;` vào vị trí đòi hỏi biểu thức điều kiện (ví dụ trong closure của `.filter()`). | Bỏ dấu chấm phẩy `;` ở cuối mệnh đề để khối mã trả về giá trị `bool` thực sự cho bộ lọc. |
| **E0507** | `cannot move out of '...' which is behind a shared reference` | Bạn cố lấy quyền sở hữu (ownership) của một phần tử trong danh sách khi đang duyệt qua tham chiếu mượn (`.iter()`). | Sử dụng tham chiếu `&` thay vì đoạt quyền sở hữu, hoặc gọi phương thức `.clone()` nếu thực sự cần một bản sao độc lập. |

### Ví dụ phân tích lỗi `E0384` thực tế:

```rust
// Đoạn mã lỗi minh họa: Cố tình gán lại giá trị cho biến bất biến
fn doan_ma_loi() {
    let tong_tien = 100;
    // tong_tien = tong_tien + 50; // LỖI E0384: cannot assign twice to immutable variable `tong_tien`
}

// Cách sửa chữa đúng chuẩn lập trình hàm:
fn doan_ma_dung() {
    let tien_goc = 100;
    let phu_phi = 50;
    // Tạo biến mới bằng một biểu thức tính toán rõ ràng
    let tong_tien = tien_goc + phu_phi; 
    println!("Tổng tiền: {}", tong_tien);
}
```

---

## Tóm tắt chương & Bài tập rèn luyện (Summary & Exercises)

### 4 Điểm cốt lõi cần ghi nhớ:
1. **Mệnh lệnh vs Khai báo**: Lập trình mệnh lệnh chỉ đạo CPU làm *như thế nào* bằng các bước thao tác vi mô; Lập trình khai báo tuyên bố *kết quả mong muốn là gì* thông qua chuỗi chuyển hóa dữ liệu.
2. **Hàm thuần túy (Pure Functions)**: Nhận đầu vào, trả về đầu ra, không tạo tác dụng phụ ra bên ngoài, mang lại sự tin cậy tuyệt đối và triệt tiêu lỗi ngầm.
3. **Bất biến mặc định**: Bảo vệ dữ liệu không bị sửa đổi ngoài ý muốn; dữ liệu qua đường ống luôn giữ trọn vẹn trạng thái gốc.
4. **Mọi thứ là Biểu thức**: Tận dụng triệt để biểu thức trả về giá trị để loại bỏ các biến tạm thời không cần thiết.
5. **Minh bạch tham chiếu**: thay được biểu thức bằng giá trị của nó mà chương trình không đổi nghĩa — đây chính là giấy phép để tái cấu trúc mã an toàn.
6. **Hàm toàn phần**: mọi đầu vào đều có câu trả lời. Hoặc *mở rộng đầu ra* bằng `Option`/`Result`, hoặc *thu hẹp đầu vào* bằng kiểu chặt hơn (Chương 20). Mỗi `.unwrap()` là một bước lùi khỏi tính toàn phần.

### Bài tập rèn luyện tự giải:
1. **Bài tập 1 (Chuyển đổi tư duy Mệnh lệnh sang Khai báo)**:  
   Cho một danh sách các số nguyên bất biến: `let list = vec![3, 8, 12, 5, 20, 7];`.  
   Hãy viết chương trình bằng phong cách đường ống (sử dụng `.iter()`, `.filter()`, `.map()`, `.sum()`):
   - Lọc ra các số lẻ.
   - Nhân đôi giá trị của từng số lẻ đó.
   - Tính tổng toàn bộ các số sau khi nhân đôi.  
   *(Yêu cầu: Không sử dụng bất kỳ biến `mut` nào).*

2. **Bài tập 2 (Xây dựng Hàm thuần túy)**:  
   Định nghĩa một hàm thuần túy `chuan_hoa_ten(full_name: &str) -> String` nhận vào một chuỗi họ tên bị thừa khoảng trắng ở hai đầu (ví dụ: `"   nguyễn văn an   "`), thực hiện cắt tỉa khoảng trắng thừa và viết in hoa toàn bộ chuỗi ký tự trả về (`"NGUYỄN VĂN AN"`). Kiểm tra tính thuần túy: gọi hàm này 3 lần liên tiếp với cùng tham số và xác nhận kết quả trả về luôn giống nhau.

3. **Bài tập 3 (Tư duy thiết kế)**:  
   Tại sao trong các hệ thống xử lý phân tán hoặc tài chính ngân hàng có tính chất quan trọng sống còn, các kiến trúc sư phần mềm luôn ưu tiên sử dụng lập trình hàm và dữ liệu bất biến thay vì cho phép các luồng tiến trình tự do sửa đổi một biến chung trên thanh RAM?

---

### Gợi ý & Lời giải

<details>
<summary><b>Bài tập 1 — Gợi ý</b></summary>

Ba bước của đề bài ứng đúng ba mắt xích: lọc số lẻ là `.filter()`, nhân đôi là `.map()`, tính tổng là `.sum()`. Chú ý `.iter()` trên `Vec<i32>` cho ra `&i32`, nên trong closure của `filter` bạn sẽ gặp `&&i32` — dùng mẫu `|&&x|` hoặc `|x| **x` để bóc hai lớp tham chiếu.
</details>

<details>
<summary><b>Bài tập 1 — Lời giải</b></summary>

```rust
fn main() {
    let list = vec![3, 8, 12, 5, 20, 7];

    let tong: i32 = list
        .iter()
        .filter(|&&x| x % 2 != 0)   // giữ số lẻ: 3, 5, 7
        .map(|&x| x * 2)            // nhân đôi : 6, 10, 14
        .sum();                     // cộng lại : 30

    assert_eq!(tong, 30);
    println!("Tổng sau khi lọc lẻ và nhân đôi: {}", tong);
}
```
Không một biến `mut` nào, không một chỉ số mảng nào — nên cũng không có cơ hội truy cập ngoài biên.
</details>

<details>
<summary><b>Bài tập 2 — Gợi ý</b></summary>

Hàm chỉ cần hai lời gọi có sẵn của `str`: `.trim()` và `.to_uppercase()`. Điều quan trọng của bài này không phải cách viết, mà là **vì sao nó thuần túy**: nó chỉ đọc tham số, không đụng vào bất cứ thứ gì bên ngoài, không in ra màn hình, không đọc đồng hồ.
</details>

<details>
<summary><b>Bài tập 2 — Lời giải</b></summary>

```rust
/// Hàm THUẦN TÚY: chỉ phụ thuộc tham số đầu vào, không tác dụng phụ.
pub fn chuan_hoa_ten(full_name: &str) -> String {
    full_name.trim().to_uppercase()
}

fn main() {
    let tho = "   nguyễn văn an   ";

    // Gọi 3 lần với cùng đầu vào -> luôn cùng kết quả (tính tất định)
    let a = chuan_hoa_ten(tho);
    let b = chuan_hoa_ten(tho);
    let c = chuan_hoa_ten(tho);
    assert_eq!(a, b);
    assert_eq!(b, c);
    assert_eq!(a, "NGUYỄN VĂN AN");

    // Đầu vào gốc KHÔNG hề bị thay đổi:
    assert_eq!(tho, "   nguyễn văn an   ");
    println!("{:?} -> {:?}", tho, a);
}
```

Lưu ý cách `to_uppercase()` xử lý đúng chữ tiếng Việt có dấu — vì Rust làm việc theo Unicode chứ không phải ASCII (Chương 03).
</details>

<details>
<summary><b>Bài tập 3 — Lời giải tham khảo</b></summary>

Có bốn lý do, xếp theo mức độ nghiêm trọng:

1. **Triệt tiêu tranh chấp dữ liệu (data race).** Nếu hai luồng cùng sửa một số dư tài khoản, kết quả phụ thuộc vào việc luồng nào chạy trước — một loại lỗi *không lặp lại được*, nên gần như không thể gỡ. Dữ liệu bất biến thì có bao nhiêu luồng đọc cũng không sao, vì không ai ghi cả.
2. **Kiểm toán được (auditability).** Ngành tài chính bắt buộc phải trả lời câu hỏi *"số dư này đến từ đâu?"*. Nếu mỗi giao dịch sinh ra một **giá trị mới** thay vì ghi đè giá trị cũ, bạn có sẵn toàn bộ lịch sử. Đây chính là nguyên lý *Nhật ký sự kiện (Event Sourcing)* mà Chương 54 sẽ xây dựng.
3. **Khôi phục sau sự cố.** Dữ liệu bất biến có thể phát lại (replay) từ đầu để dựng lại trạng thái sau khi máy chủ sập — đúng nguyên lý của WAL và LSM-Tree ở Chương 34.
4. **Suy luận và kiểm thử.** Với hàm thuần túy, muốn biết một hàm làm gì bạn chỉ cần đọc chữ ký và thân hàm, không phải truy vết xem có ai đó ở tệp khác đang sửa trộm biến toàn cục hay không (đây chính là *minh bạch tham chiếu* ở mục 4).

Và trong Rust, ba lý do đầu không phải lời khuyên suông: hệ thống quyền sở hữu **buộc** bạn tuân thủ — muốn chia sẻ dữ liệu khả biến giữa các luồng, bạn phải nói rõ ra bằng `Mutex` hoặc `Arc<Mutex<..>>`, và trình biên dịch sẽ từ chối nếu bạn quên.
</details>
