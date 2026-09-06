# Chương 10: Kiểu liệt kê, Option và So khớp mẫu (Enums, Option, and Pattern Matching)

## Giới thiệu & Mục tiêu học tập

Trong lập trình thực tế, rất nhiều đại lượng chỉ có thể nhận một trong số các trạng thái hữu hạn xác định: Đèn giao thông chỉ có thể là Xanh, Đỏ hoặc Vàng. Một đơn hàng trên Shopee chỉ có thể là Chờ thanh toán, Đang vận chuyển, Giao thành công, hoặc Đã hủy.

Để mô hình hóa những trạng thái này, hầu hết các ngôn ngữ đều có khái niệm **Kiểu liệt kê (Enum)**. Tuy nhiên, trong Rust, Enum không đơn thuần là một danh sách các con số nguyên đơn điệu như trong C hay Java. Enum của Rust là một cỗ máy kỳ diệu mang tên **Kiểu dữ liệu đại số (Algebraic Data Types)** — nơi mỗi nhánh liệt kê có thể cõng theo những khối dữ liệu phong phú với kích thước và kiểu dáng hoàn toàn khác nhau!

Đặc biệt hơn, chương này sẽ giới thiệu cho bạn phát minh giúp Rust loại bỏ vĩnh viễn "Sai lầm tỷ đô" của ngành công nghệ thông tin: kiểu **`Option<T>`**, kết hợp với vũ khí sắc bén **So khớp mẫu (`match`)**.

Mục tiêu học tập của chương này:
- Làm chủ sức mạnh của `enum` đa năng trong Rust: gán dữ liệu cụ thể vào từng nhánh liệt kê.
- Hiểu nguồn gốc của "Sai lầm tỷ đô" (The Billion-Dollar Mistake - `null`) và cách kiểu `Option<T>` (`Some` và `None`) xóa sổ hoàn toàn lỗi sập ứng dụng do con trỏ rỗng.
- Thành thạo cấu trúc so khớp mẫu `match` với tính chất **Bắt buộc Vét cạn (Exhaustiveness)** 100%.
- Sử dụng các mẫu nâng cao: Mẫu đại diện `_`, Mẫu kết hợp `|`, Mẫu khoảng giá trị `1..=5`, và Mệnh đề bảo vệ mẫu (**Match Guards `if`**).
- Viết mã ngắn gọn và thanh lịch với cú pháp `if let` và `let else`.

---

## Hình tượng hóa đời sống (Intuitive Everyday Analogy)

Hãy cùng hình tượng hóa các khái niệm này qua 3 câu chuyện đời thường vô cùng gần gũi:

```
┌──────────────────────────────────────────────────────────────────────────────────┐
│                     HÌNH TƯỢNG ĐỜI SỐNG VỀ ENUMS, OPTION VÀ MATCH                │
├─────────────────────────┬───────────────────────────────┬────────────────────────┤
│   Ổ CẮM ĐIỆN ĐA NĂNG    │      HỘP QUÀ SINH NHẬT BÍ MẬT │    BÁC BƯU TÁ PHÂN LOẠI│
│           (Rust Enum)   │                 (Option<T>)   │                 (match)│
│                         │                               │                        │
│ - Hoặc cắm 2 chấu tròn  │ - Nhận hộp quà thắt nơ        │ - Có 10 sọt thư ứng với│
│ - Hoặc cắm 3 chấu dẹt   │ - Chỉ có đúng 2 trạng thái:   │   10 quận trong thành  │
│ - Hoặc cắm cổng USB-C   │   + Some: Có món quà bên trong│   phố                  │
│ - Không bao giờ có trạng│   + None: Hộp trống rỗng      │ - Thả đúng từng lá thư │
│   thái nửa nọ nửa kia   │ - Phải MỞ NẮP ra kiểm tra     │ - Không được bỏ sót dù │
│   gây chập cháy nổ điện │   mới được dùng quà!          │   chỉ 1 lá ngoài sân!  │
└─────────────────────────┴───────────────────────────────┴────────────────────────┘
```

### 1. Ổ cắm điện đa năng trên tường (Sức mạnh của Rust Enum)
Hãy tưởng tượng một ổ cắm điện âm tường hiện đại:
- Thiết bị cắm vào hoặc là một chiếc phích cắm 2 chấu tròn (chỉ mang điện áp 220V), hoặc là phích 3 chấu dẹt (có thêm dây tiếp đất an toàn), hoặc là một sợi cáp USB-C (mang dòng điện 5V sạc điện thoại).
- Tại một thời điểm, chỉ có **một loại phích cắm duy nhất** được gắn vào ổ. Mỗi loại phích cắm mang theo những thông số kỹ thuật (dữ liệu) hoàn toàn khác nhau, và hệ thống điện luôn nhận biết chính xác loại nào đang được kết nối.

### 2. Chiếc hộp quà sinh nhật bí mật (Bản chất của `Option<T>`)
Vào ngày sinh nhật, bạn nhận được một chiếc hộp quà xinh xắn thắt nơ từ một người bạn:
- Khi nhìn từ bên ngoài, bạn **chưa thể biết** bên trong có gì.
- Chỉ có đúng 2 khả năng vật lý xảy ra khi bạn mở nắp hộp:
  1. Mở nắp ra và thấy một chiếc đồng hồ đeo tay: đó là **`Some(đồng_hồ)`** (Có dữ liệu).
  2. Mở nắp ra và thấy chiếc hộp hoàn toàn trống rỗng (một trò đùa vui): đó là **`None`** (Không có dữ liệu).
- Trong Rust, bạn **không bao giờ được phép nhắm mắt đưa tay lên cổ tay giả vờ xem giờ** khi chưa mở hộp! Bạn bắt buộc phải mở hộp ra, kiểm tra xem có quà hay không. Nếu có quà thì mới đeo, nếu rỗng thì mỉm cười cho qua. Nhờ vậy, bạn không bao giờ bị "hớ"!

### 3. Bác nhân viên bưu điện phân loại thư từ (So khớp mẫu `match` vét cạn)
Bác bưu tá ngồi trước 10 chiếc sọt đựng thư tương ứng với 10 quận huyện:
- Bác cầm từng phong bì thư lên, liếc mắt nhìn địa chỉ (`match thư`).
- Nếu ghi "Quận 1" -> Thả vào sọt 1. Nếu ghi "Quận 3" -> Thả vào sọt 3.
- **Quy tắc trách nhiệm tối cao (Exhaustiveness)**: Bác bưu tá không được phép bỏ quên bất kỳ lá thư nào nằm bơ vơ trên mặt đất. Mọi lá thư đều phải có một chiếc sọt đích đến rõ ràng. Nếu có một lá thư gửi đi tỉnh xa ngoài danh sách, bác phải chuẩn bị sẵn một chiếc sọt phụ dán nhãn "Các nơi khác" (`_`) để hứng nó!

---

## Khái niệm & Cơ chế kỹ thuật chuyên sâu (In-Depth Technical Mechanics)

### 1. Xóa sổ "Sai lầm tỷ đô" (The Billion-Dollar Mistake)

Năm 1965, nhà khoa học máy tính lỗi lạc Sir Tony Hoare phát minh ra giá trị `null` (con trỏ trỏ vào số không) cho ngôn ngữ ALGOL W. Nhiều năm sau, chính ông đã công khai lên tiếng xin lỗi toàn nhân loại:

> *"Tôi gọi đó là sai lầm tỷ đô của đời mình... Nó đã dẫn đến vô số lỗi, lỗ hổng bảo mật và sự cố sập hệ thống phần mềm, gây thiệt hại hàng tỷ đô la trong suốt 50 năm qua."*

Trong hầu hết các ngôn ngữ (C++, Java, Python, C#), bất kỳ biến đối tượng nào cũng có thể bí mật mang giá trị `null`. Nếu bạn viết `nguoi.gui_tin_nhan()` mà biến `nguoi` lại vô tình bị rỗng (`null`), cả hệ thống máy chủ sẽ lập tức lăn đùng ra chết với lỗi `NullPointerException`!

**Giải pháp triệt để của Rust**:
Rust **không hề có từ khóa `null`**!
Nếu một đại lượng có thể bị thiếu vắng dữ liệu trong thực tế (ví dụ: người dùng có thể chưa cập nhật số điện thoại), biến đó **bắt buộc phải mang kiểu `Option<T>`**:
```rust
enum Option<T> {
    Some(T), // Có giá trị T bên trong
    None,    // Không có giá trị nào cả
}
```
Vì kiểu `Option<String>` và kiểu `String` là hai kiểu dữ liệu hoàn toàn khác nhau, bạn không thể cộng chuỗi hay in ấn trực tiếp từ `Option<String>`. Trình biên dịch Rust ép buộc bạn 100% phải dùng `match` hoặc `if let` để bóc tách gói quà ra trước khi sử dụng. Nhờ vậy, lỗi Null Pointer bị tiêu diệt vĩnh viễn ngay từ khâu biên dịch!

### 2. Bố cục ô nhớ của Enum (Tag và Payload)

Dưới góc nhìn phần cứng, Rust bố trí một `enum` trên RAM như thế nào?
- **Thẻ định danh (Discriminant / Tag)**: 1 byte nhỏ dùng để đánh dấu xem nhánh nào đang hoạt động (nhánh 0, 1 hay 2).
- **Vùng chứa dữ liệu (Payload)**: Rust đo kích thước của nhánh lớn nhất trong enum rồi dành ra một vùng vừa đủ bằng nhánh lớn nhất đó. Vùng này nằm **ngay tại chỗ enum được lưu** (trên Stack nếu biến nằm trên Stack, bên trong `Box` nếu enum nằm trong `Box`) — bản thân `enum` **không bao giờ tự cấp phát thêm bộ nhớ Heap**.

> **Kỹ thuật tối ưu hóa con trỏ rỗng (Null Pointer Optimization - NPO)**:
> Khi bạn sử dụng `Option<&T>` hoặc `Option<Box<T>>` (con trỏ tham chiếu), Rust biết rằng một con trỏ hợp lệ trên RAM không bao giờ mang địa chỉ `0x0`.
> Do đó, Rust gán giá trị `None` chính là địa chỉ `0x0`, còn `Some(&T)` là địa chỉ thực tế của đối tượng! Kết quả là: `Option<&T>` chiếm đúng **8 bytes** trên RAM — **hoàn toàn không tốn thêm 1 bit nào** so với một con trỏ thông thường!

### 3. Vì sao gọi là "ĐẠI SỐ"? Kiểu tích và kiểu tổng

Cụm từ *Kiểu dữ liệu đại số* nghe rất kêu, nhưng ý nghĩa của nó thì đơn giản đến bất ngờ: **hãy đếm số trạng thái mà một kiểu có thể mang**.

| Kiểu | Số giá trị có thể | Phép toán |
|---|---|---|
| `bool` | 2 | |
| `()` | 1 | |
| `(bool, bool)` — **struct** | 2 × 2 = **4** | **NHÂN** → gọi là *kiểu tích* (product type) |
| `enum { A, B, C }` — **enum** | 1 + 1 + 1 = **3** | **CỘNG** → gọi là *kiểu tổng* (sum type) |
| `Option<bool>` | 1 + 2 = **3** | `None` + `Some(true)` + `Some(false)` |

- **`struct` là kiểu TÍCH**: nó chứa trường A **VÀ** trường B cùng lúc, nên số tổ hợp là tích: `|A × B| = |A| · |B|`.
- **`enum` là kiểu TỔNG**: nó là nhánh A **HOẶC** nhánh B, nên số tổ hợp là tổng: `|A + B| = |A| + |B|`.

Đây không phải trò chơi chữ — nó là **công cụ thiết kế**. Quy tắc vàng:

> Kiểu tốt nhất là kiểu biểu diễn được **đúng bằng** số trạng thái hợp lệ trong nghiệp vụ. Mỗi trạng thái dư ra là một lỗi đang chờ xảy ra.

```rust
// ❌ Kiểu TÍCH: có 2 tổ hợp VÔ NGHĨA
struct DonQueue { is_paid: bool, id_trade: Option<String> }
//   (true, None)      -> đã trả tiền mà không có mã deliver dịch?!
//   (false, Some(..)) -> chưa trả tiền mà đã có mã?!

// ✅ Kiểu TỔNG: KHÔNG CÒN tổ hợp vô nghĩa nào
enum PaymentState { ChuaTra, DaTra { id_trade: String } }
```

Chúng ta sẽ khai thác triệt để ý tưởng này ở **Chương 20** để loại bỏ cả một lớp lỗi khỏi chương trình.

### 4. Bộ công cụ so khớp mẫu đầy đủ

`match` của Rust mạnh hơn nhiều so với `switch` của các ngôn ngữ khác. Đây là những dạng mẫu bạn sẽ dùng thường xuyên:

```rust
let diem = 85;
let so = Some(7);
let mang = [1, 2, 3, 4, 5];

// 1) MẪU KHOẢNG (range pattern)
let grade = match diem {
    90..=100 => "Xuất sắc",
    80..=89  => "Giỏi",
    50..=79  => "Đạt",
    _        => "Chưa đạt",
};

// 2) MẪU HOẶC `|` — gộp nhiều nhánh
let is_weekend = match "Thứ 7" {
    "Thứ 7" | "Chủ nhật" => true,
    _ => false,
};

// 3) ĐIỀU KIỆN BẢO VỆ (match guard) — thêm `if` vào nhánh
let description = match so {
    Some(n) if n % 2 == 0 => "số chẵn",
    Some(n) if n > 5      => "số lẻ lớn",
    Some(_)               => "số lẻ nhỏ",
    None                  => "không có gì",
};

// 4) RÀNG BUỘC `@` — vừa kiểm tra vừa GIỮ LẠI giá trị
let thong_report = match so {
    Some(n @ 1..=9) => format!("Chữ số đơn: {}", n),  // n vẫn dùng được!
    Some(n)         => format!("Số lớn: {}", n),
    None            => "Rỗng".to_string(),
};

// 5) MẪU LÁT CẮT — bóc tách mảng
let tom_tat = match &mang[..] {
    []              => "rỗng".to_string(),
    [x]             => format!("một phần tử: {}", x),
    [first, .., last] => format!("từ {} đến {}", first, last),
};

// 6) `matches!` — kiểm tra nhanh, trả về bool
let co_gia_tri = matches!(so, Some(_));

// 7) `let ... else` — bóc tách hoặc THOÁT SỚM, giữ mã phẳng phiu
fn handle(input: Option<i32>) -> i32 {
    let Some(n) = input else {
        return 0;   // bắt buộc phải thoát khỏi phạm vi
    };
    n * 2           // từ đây trở đi, n dùng như biến bình thường
}
```

> **`let ... else` là công cụ chống thụt lề tuyệt vời**: thay vì bọc toàn bộ phần còn lại của hàm trong `if let Some(n) = ... { ... }`, bạn xử lý trường hợp xấu trước rồi thoát, giữ luồng chính luôn nằm ở mức thụt lề ngoài cùng.

### 5. Quy tắc bắt buộc Vét cạn của `match` (Exhaustiveness)

Khi bạn so khớp một biểu thức với `match`, Rust bắt buộc bạn phải liệt kê **đầy đủ tất cả các trường hợp có thể xảy ra**.
Nếu bạn quên một nhánh, trình biên dịch sẽ từ chối dịch mã với lỗi `E0004`:
```rust
enum Gender { Nam, Nu, Khac }

let gender = Gender::Nam;

// ĐOẠN MÃ NÀY BỊ LỖI E0004 VÌ QUÊN CHƯA XỬ LÝ NHÁNH 'Khac':
match gender {
    Gender::Nam => println!("Nam giới"),
    Gender::Nu => println!("Nữ giới"),
}
```
Khi biên dịch đoạn mã trên, trình biên dịch Rust sẽ từ chối dịch mã với thông báo:
```text
error[E0004]: non-exhaustive patterns: `GioiTinh::Khac` not covered
 --> src/main.rs:6:11
  |
6 |     match gioi_tinh {
  |           ^^^^^^^^^ pattern `GioiTinh::Khac` not covered
help: ensure that all possible cases are being handled by adding a match arm
```

---

## Mã nguồn minh họa thực chiến (Idiomatic Runnable Rust Blueprint)

Chương trình hoàn chỉnh dưới đây minh họa một hệ thống xử lý trạng thái Đơn hàng Thương mại Điện tử, kết hợp giữa Enum chứa dữ liệu phong phú, hàm an toàn trả về `Option`, cấu trúc `match` vét cạn có Match Guards, `if let` và `let else`:

```rust
// File: src/main.rs
// Chương trình thực chiến làm chủ Enums, Option & So khớp mẫu (Pattern Matching)

// 1. Enum biểu diễn các trạng thái đa dạng của một đơn hàng trực tuyến
// Mỗi nhánh có thể cõng theo những thông tin hoàn toàn khác nhau!
enum StateDonQueue {
    AwaitingPayment,
    DangDongGoi { store_export_queue: String },
    InTransit { ma_van_don: String, ten_tai_xe: String },
    Delivered { recipient: String, time_time_recv: String },
    Cancelled(String), // Cõng theo một chuỗi String chứa lý do hủy đơn
}

// 2. Hàm chia kẹo an toàn: Trả về Option<u32> để ngăn chặn lỗi chia cho 0
fn safe_divide(so_keo: u32, so_tre_em: u32) -> Option<u32> {
    if so_tre_em == 0 {
        // Không thể chia cho 0 em bé: Trả về None báo hiệu không có kết quả
        None
    } else {
        // Chia thành công: Bọc kết quả vào trong hộp Some
        Some(so_keo / so_tre_em)
    }
}

// 3. Hàm xử lý trạng thái đơn hàng bằng cấu trúc so khớp mẫu 'match' toàn diện
fn update_process(don_hang: &StateDonQueue) {
    println!("------------------------------------------------------------");
    match don_hang {
        StateDonQueue::AwaitingPayment => {
            println!("[TRẠNG THÁI] Đơn hàng đang chờ khách thanh toán qua thẻ...");
        }
        StateDonQueue::DangDongGoi { store_export_queue } => {
            println!("[TRẠNG THÁI] Đơn hàng đang được đóng gói tại kho: {}", store_export_queue);
        }
        // Bóc tách cả 2 trường dữ liệu từ nhánh InTransit
        StateDonQueue::InTransit { ma_van_don, ten_tai_xe } => {
            println!("[VẬN CHUYỂN] Đơn đang trên đường deliver!");
            println!("  + Mã vận đơn : {}", ma_van_don);
            println!("  + Shipper    : {}", ten_tai_xe);
        }
        StateDonQueue::Delivered { recipient, time_time_recv } => {
            println!("[THÀNH CÔNG] Đơn hàng đã deliver thành công!");
            println!("  + Người ký nhận: {}", recipient);
            println!("  + Thời điểm    : {}", time_time_recv);
        }
        StateDonQueue::Cancelled(ly_do) => {
            println!("[HỦY BỎ] Đơn hàng đã bị hủy. Lý do ghi nhận: '{}'", ly_do);
        }
    }
}

fn main() {
    println!("============================================================");
    println!("    HỆ THỐNG QUẢN LÝ ĐƠN HÀNG & MÔ HÌNH DỮ LIỆU AN TOÀN     ");
    println!("============================================================");

    // --- PHẦN 1: SO KHỚP MẪU VỚI ENUM CHỨA DỮ LIỆU ---
    let don_cho = StateDonQueue::AwaitingPayment;
    let don_dong_goi = StateDonQueue::DangDongGoi {
        store_export_queue: String::from("Kho Tổng Cầu Giấy, Hà Nội"),
    };
    let don_van_transfer = StateDonQueue::InTransit {
        ma_van_don: String::from("SPX-987654321"),
        ten_tai_xe: String::from("Bác Ba Giao Hàng"),
    };
    let order_delivered = StateDonQueue::Delivered {
        recipient: String::from("Trần Thị Bình"),
        time_time_recv: String::from("14:30 ngày 05/09/2026"),
    };
    let don_cancel = StateDonQueue::Cancelled(String::from("Khách hàng đổi ý muốn chọn màu khác"));

    update_process(&don_cho);
    update_process(&don_dong_goi);
    update_process(&don_van_transfer);
    update_process(&order_delivered);
    update_process(&don_cancel);

    // --- PHẦN 2: LÀM VIỆC VỚI OPTION<T> VÀ TRIỆT TIÊU NULL ---
    println!("\n=== KIỂM THỬ TÍNH TOÁN AN TOÀN VỚI OPTION ===");
    let result_hop_le = safe_divide(20, 4);
    let result_error = safe_divide(20, 0);

    // Dùng match để mở hộp quà Option
    match result_hop_le {
        Some(keo) => println!("- Chia 20 kẹo cho 4 bé: Mỗi bé được {} cái kẹo.", keo),
        None => println!("- Lỗi: Số trẻ em không thể bằng 0!"),
    }

    match result_error {
        Some(keo) => println!("- Mỗi bé được: {} cái kẹo.", keo),
        None => println!("- [Được bảo vệ an toàn] Không thể chia cho 0 bé! Hệ thống không bị sập!"),
    }

    // --- PHẦN 3: MATCH GUARDS (ĐIỀU KIỆN BẢO VỆ PHỤ) VÀ KHOẢNG GIÁ TRỊ ---
    println!("\n=== PHÂN LOẠI TUỔI KHÁCH HÀNG VỚI MATCH GUARDS ===");
    let age = 17;
    let co_the_can_cuoc = true;

    match age {
        0..=12 => println!("Khách hàng thuộc lứa tuổi Thiếu nhi"),
        13..=17 if co_the_can_cuoc => println!("Lứa tuổi vị thành niên (ĐÃ có thẻ CCCD hợp lệ)"),
        13..=17 => println!("Lứa tuổi vị thành niên (chưa làm thẻ CCCD)"),
        18..=60 => println!("Khách hàng trong độ tuổi lao động trưởng thành"),
        _ => println!("Khách hàng cao tuổi ưu tiên"),
    }

    // --- PHẦN 4: CÚ PHÁP RÚT GỌN 'if let' ---
    println!("\n=== DÙNG 'if let' KHI CHỈ QUAN TÂM 1 TRƯỜNG HỢP ===");
    let info_recv_send_to: Option<&str> = Some("Xin chào, bạn có nhà không?");

    // Thay vì viết match dài dòng với cả nhánh None, ta chỉ bắt nhánh Some:
    if let Some(content) = info_recv_send_to {
        println!("Tin nhắn mới nhận được: '{}'", content);
    }
}
```

---

## Bảng tra cứu lỗi biên dịch & Cách khắc phục (Compiler Error Guide)

Dưới đây là các lỗi kinh điển khi sử dụng Enum và Pattern Matching trong Rust:

| Mã lỗi | Thông báo mẫu từ trình biên dịch | Nguyên nhân cốt lõi | Cách khắc phục nhanh |
|---|---|---|---|
| **E0004** | `non-exhaustive patterns: 'None' not covered` | Bạn dùng `match` trên một biến `Option` hoặc `Enum` nhưng quên không viết nhánh xử lý cho một số trường hợp. | Bổ sung thêm các nhánh còn thiếu vào khối `match`, hoặc thêm nhánh đại diện `_ => ...` để bắt toàn bộ các trường hợp còn lại. |
| **E0308** | `mismatched types: expected integer, found 'Option<{integer}>'` | Bạn cố tình lấy một biến `Option<i32>` ra cộng trừ nhân chia trực tiếp với một số nguyên mà quên mở nắp hộp. | Dùng `match`, `if let`, hoặc phương thức `.unwrap_or(0)` để lấy giá trị số nguyên thực sự bên trong hộp ra trước khi tính toán. |
| **E0425** | `cannot find value 'ChoThanhToan' in this scope` | Bạn viết tên nhánh của Enum một cách cộc lốc mà không chỉ định tên Enum cha. | Thêm tiền tố tên Enum phía trước: `StateDonQueue::ChoThanhToan`. |
| **E0005** | `refutable pattern in local binding` | Bạn dùng `let Some(x) = bien_option;` để gán biến. Rust từ chối vì nếu `bien_option` là `None` thì lệnh gán sẽ thất bại. | Chuyển sang sử dụng cú pháp `if let Some(x) = ...` hoặc `let Some(x) = ... else { return; };`. |

---

## Tóm tắt chương & Bài tập rèn luyện (Summary & Exercises)

### 4 Điểm cốt lõi cần ghi nhớ:
1. **Enum đa năng (Algebraic Data Types)**: Trong Rust, mỗi nhánh của Enum có thể chứa các kiểu dữ liệu riêng biệt (từ số nguyên, struct đến tuple).
2. **Không có `null`**: Rust thay thế hoàn toàn con trỏ rỗng bằng kiểu `Option<T>` gồm `Some(T)` và `None`, buộc lập trình viên phải xử lý trường hợp thiếu dữ liệu một cách minh bạch.
3. **So khớp mẫu `match` vét cạn**: Không bao giờ bỏ sót bất kỳ trường hợp nào, giúp loại trừ hoàn toàn các nhánh logic bị lãng quên trong phần mềm.
4. **Cú pháp `if let`**: Lối viết ngắn gọn, tiện lợi khi bạn chỉ muốn thực hiện hành động cho một nhánh duy nhất mà bỏ qua các nhánh còn lại.

### Bài tập rèn luyện tự giải:
1. **Bài tập thực hành 1**: Định nghĩa một `enum PhepTinh` gồm 4 nhánh:
   - `Cong(f64, f64)`
   - `Tru(f64, f64)`
   - `Nhan(f64, f64)`
   - `Chia(f64, f64)`
   Viết hàm `tinh_toan(pt: PhepTinh) -> Option<f64>` sử dụng cấu trúc `match`. Lưu ý nhánh `Chia` nếu mẫu số bằng `0.0` thì phải trả về `None`, ngược lại trả về `Some(kết_quả)`.
2. **Bài tập tư duy 2**: Tại sao nói kiểu `Option<T>` giúp loại bỏ lỗi sập hệ thống tốt hơn việc hàm trả về một con số quy ước đặc biệt (ví dụ trả về số `-1` để báo lỗi)?
3. **Bài tập `if let` 3**: Cho biến `let diem_danh: Option<&str> = Some("Có mặt");`. Hãy dùng cú pháp `if let` để in ra dòng chữ `"Học viên: Có mặt"`, và thử đổi giá trị thành `None` để kiểm tra xem chương trình chạy êm đẹp ra sao.

---

### Gợi ý & Lời giải

<details>
<summary><b>Bài tập 1 — Gợi ý</b></summary>

`enum` gom các biến thể có mang dữ liệu; `match` bắt buộc xử lý đủ mọi nhánh. Nhánh `Chia` cho mẫu 0 trả `None` để báo phép tính vô nghĩa mà không sập.
</details>

<details>
<summary><b>Bài tập 1 — Lời giải</b></summary>

```rust
enum PhepTinh {
    Cong(f64, f64),
    Tru(f64, f64),
    Nhan(f64, f64),
    Chia(f64, f64),
}

// Trả Option: Some(kết quả) khi hợp lệ, None khi chia cho 0.
fn tinh_toan(pt: PhepTinh) -> Option<f64> {
    match pt {
        PhepTinh::Cong(a, b) => Some(a + b),
        PhepTinh::Tru(a, b) => Some(a - b),
        PhepTinh::Nhan(a, b) => Some(a * b),
        // Chia cho 0.0 là vô nghĩa -> None thay vì để sinh ra vô cực/NaN.
        PhepTinh::Chia(_, b) if b == 0.0 => None,
        PhepTinh::Chia(a, b) => Some(a / b),
    }
}

fn main() {
    println!("{:?}", tinh_toan(PhepTinh::Cong(2.0, 3.0)));   // Some(5.0)
    println!("{:?}", tinh_toan(PhepTinh::Chia(1.0, 0.0)));   // None
}

#[test]
fn phep_tinh_co_ban() {
    assert_eq!(tinh_toan(PhepTinh::Cong(2.0, 3.0)), Some(5.0));
    assert_eq!(tinh_toan(PhepTinh::Nhan(4.0, 5.0)), Some(20.0));
    assert_eq!(tinh_toan(PhepTinh::Chia(10.0, 2.0)), Some(5.0));
    assert_eq!(tinh_toan(PhepTinh::Chia(1.0, 0.0)), None);   // không sập, trả None
}
```

Hai điều đáng học: **`enum` mang dữ liệu** cho phép mỗi biến thể ôm theo toán hạng của nó — gọn hơn nhiều struct rời. Và **guard `if b == 0.0`** trong `match` tách nhánh chia-cho-0 ra xử lý riêng *trước* nhánh chia thường, biến một lỗi tiềm tàng thành một giá trị `None` tường minh mà người gọi buộc phải xử lý.
</details>

<details>
<summary><b>Bài tập 2 — Gợi ý</b></summary>

So sánh hai cách báo lỗi: trả `-1` (một con số trông như dữ liệu thật) so với trả `None` (một giá trị mà trình biên dịch *bắt* bạn kiểm tra).
</details>

<details>
<summary><b>Bài tập 2 — Lời giải</b></summary>

`Option<T>` thắng "quy ước trả -1" vì nó biến việc xử-lý-lỗi từ *kỷ luật tự giác* thành *ràng buộc trình biên dịch ép*.

**Vấn đề của quy ước `-1`:**
```text
fn tim_vi_tri(...) -> i32 { ... }   // trả -1 nếu không thấy
let vt = tim_vi_tri(...);
let ket_qua = mang[vt as usize];    // QUÊN kiểm tra -1 -> đọc mang[-1] -> sập/rác
```
Con số `-1` **trông y hệt một kết quả hợp lệ**. Không gì ngăn bạn quên kiểm tra; trình biên dịch cũng chẳng nhắc, vì `-1` vẫn là một `i32` đúng kiểu. Lỗi chỉ lộ ra lúc chạy, trên máy người dùng.

Tệ hơn, `-1` chỉ dùng được khi nó *không* phải giá trị hợp lệ. Nếu hàm có thể trả về số âm thật (ví dụ nhiệt độ, số dư tài khoản), thì `-1` vừa là "lỗi" vừa là dữ liệu thật — không cách nào phân biệt.

**Vì sao `Option<T>` chặn được lỗi sập:**
```text
fn tim_vi_tri(...) -> Option<usize> { ... }
let vt = tim_vi_tri(...);
// let x = mang[vt];   // KHÔNG biên dịch: vt là Option<usize>, không phải usize
match vt {
    Some(i) => mang[i],   // buộc phải mở hộp -> buộc phải nghĩ tới ca "không thấy"
    None => { /* xử lý đàng hoàng */ }
}
```
`Option` gói kết quả vào một **kiểu khác** với giá trị bên trong. Bạn *không thể* dùng nó như một số cho tới khi mở hộp, và mở hộp thì *buộc* phải viết nhánh `None`. Khả năng "quên kiểm tra" bị loại bỏ ngay từ hệ thống kiểu — lỗi vắng-giá-trị chuyển từ lúc-chạy sang lúc-biên-dịch. Đó là toàn bộ triết lý của Rust: **làm cho trạng thái sai không biểu diễn được**, thay vì trông cậy lập trình viên nhớ kiểm tra.
</details>

<details>
<summary><b>Bài tập 3 — Gợi ý</b></summary>

`if let` mở một biến thể cụ thể mà không cần `match` đầy đủ. Với `None`, thân `if let` đơn giản không chạy — chương trình đi tiếp êm đẹp.
</details>

<details>
<summary><b>Bài tập 3 — Lời giải</b></summary>

```rust
fn main() {
    let diem_danh: Option<&str> = Some("Có mặt");
    // if let: chỉ quan tâm ca Some, bỏ qua None gọn gàng.
    if let Some(trang_thai) = diem_danh {
        println!("Học viên: {trang_thai}");   // in "Học viên: Có mặt"
    }

    let vang: Option<&str> = None;
    if let Some(trang_thai) = vang {
        println!("Học viên: {trang_thai}");   // KHÔNG chạy vì là None
    }
    println!("Chương trình vẫn chạy tiếp bình thường.");
}

#[test]
fn if_let_bat_dung_some() {
    let mut ket_qua = String::new();
    let diem_danh: Option<&str> = Some("Có mặt");
    if let Some(t) = diem_danh {
        ket_qua = format!("Học viên: {t}");
    }
    assert_eq!(ket_qua, "Học viên: Có mặt");

    // Với None, thân if let không chạy -> ket_qua giữ nguyên, không sập.
    let mut kq2 = String::from("chưa gán");
    let vang: Option<&str> = None;
    if let Some(t) = vang { kq2 = t.to_string(); }
    assert_eq!(kq2, "chưa gán");
}
```

`if let` là **`match` rút gọn cho đúng một biến thể bạn quan tâm**. Khi chỉ cần xử lý ca `Some` và mặc kệ `None`, viết `if let` gọn hơn hẳn `match` hai nhánh. Điểm an toàn: kể cả khi giá trị là `None`, thân lệnh đơn giản *không chạy* — không có ngoại lệ, không sập, chương trình chảy tiếp. Đây là kiểu xử lý vắng-giá-trị nhẹ nhàng mà bạn dùng liên tục.
</details>
