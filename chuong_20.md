# Chương 20: Mô hình hóa nghiệp vụ bằng kiểu: Kiểu bọc, Hàm khởi tạo có kiểm chứng và Typestate (Domain Modeling with Types)

## Giới thiệu & Mục tiêu học tập

Bảy chương vừa qua đã trang bị cho bạn toàn bộ công cụ của lập trình hàm: hàm thuần túy, phép ghép, closure, iterator, bộ kết hợp, đại số, hàm tử và đơn nguyên. Chương này trả lời câu hỏi cuối cùng và quan trọng nhất:

> **Dùng tất cả những thứ đó để làm gì trong một dự án thật?**

Câu trả lời nằm ở một ý tưởng đơn giản đến mức gây sốc:

> **Nếu một trạng thái sai không thể *biểu diễn được* trong hệ thống kiểu, thì nó không thể xảy ra lúc chạy.**

Đây là tinh thần cốt lõi của cuốn *Domain Modeling Made Functional*. Thay vì viết hàng trăm câu lệnh `if` để kiểm tra dữ liệu ở mọi tầng, bạn **kiểm tra đúng một lần ở cổng vào**, rồi để hệ thống kiểu mang theo bằng chứng hợp lệ đó đi khắp chương trình.

Hãy so sánh hai cách viết cùng một hàm:

```rust
// ❌ Kiểu "thùng rỗng": chữ ký không nói gì cả
fn send_mail(address: String) { }
// Người gọi có thể truyền vào "", "abc", "  " — hàm phải tự kiểm tra lại.

// ✅ Kiểu có bằng chứng: chữ ký là một hợp đồng
fn send_mail(address: Email) { }
// KHÔNG THỂ tạo ra một `Email` không hợp lệ. Hàm này không cần kiểm tra gì nữa.
```

Sự khác biệt không phải ở lượng mã, mà ở chỗ **ai chịu trách nhiệm**. Ở cách thứ hai, trách nhiệm được đẩy về **trình biên dịch**.

Chương này cũng dạy một kỹ thuật mà Rust làm được còn tốt hơn cả F# và Haskell: **Typestate** — mã hóa *trạng thái của quy trình* vào trong kiểu, để trình biên dịch từ chối biên dịch một đơn hàng chưa thanh toán mà đã đòi giao.

Mục tiêu học tập của chương này:
- Hiểu **đại số của kiểu**: vì sao `struct` gọi là *kiểu tích*, `enum` gọi là *kiểu tổng*, và cách **đếm số trạng thái** một kiểu có thể mang.
- Làm chủ **kiểu bọc (newtype)** kết hợp **hàm khởi tạo có kiểm chứng (smart constructor)**, và nguyên tắc **"phân tích, đừng xác thực" (parse, don't validate)**.
- Áp dụng **"biến trạng thái sai thành không biểu diễn được"** để loại bỏ cả một lớp lỗi khỏi chương trình.
- Xây dựng **Typestate** bằng generic và `PhantomData` để mã hóa máy trạng thái vào kiểu.
- Thiết lập **biên hệ thống**: kiểu truyền tải (DTO) khác kiểu miền, chuyển đổi bằng `TryFrom`.
- Nắm kiến trúc **"lõi thuần túy — vỏ mệnh lệnh" (functional core, imperative shell)** — cách gói toàn bộ giáo trình lập trình hàm vào một hình dạng kiến trúc duy nhất.

---

## Hình tượng hóa đời sống (Intuitive Everyday Analogy)

```
┌──────────────────────────────────────────────────────────────────────────────────┐
│      HÌNH TƯỢNG ĐỜI SỐNG: PHÒNG CÔNG CHỨNG VÀ CỬA KIỂM TRA SÂN BAY               │
├────────────────────────────────────────┬─────────────────────────────────────────┤
│  KIỂU BỌC + HÀM KHỞI TẠO CÓ KIỂM CHỨNG │        TYPESTATE = CỬA SÂN BAY          │
│         = PHÒNG CÔNG CHỨNG             │                                         │
│                                        │  [Vé đã đặt]                            │
│   Tờ giấy viết tay bất kỳ              │      │ ← chỉ cửa CHECK-IN nhận vé này   │
│   "nguyenvana@gmail.com"               │      ▼                                  │
│           │                            │  [Thẻ lên máy bay]                      │
│           ▼                            │      │ ← chỉ cửa AN NINH nhận thẻ này   │
│   ┌────────────────────┐               │      ▼                                  │
│   │  PHÒNG CÔNG CHỨNG  │               │  [Đã qua an ninh]                       │
│   │  Email::parse      │               │      │ ← chỉ CỬA RA MÁY BAY nhận        │
│   │  - có @ không?     │               │      ▼                                  │
│   │  - có tên miền?    │               │  [Đã lên máy bay]                       │
│   └─────────┬──────────┘               │                                         │
│      ┌──────┴───────┐                  │  KHÔNG AI cầm "vé đã đặt" mà bước       │
│      ▼              ▼                  │  thẳng vào cửa ra máy bay được —        │
│  [TỪ CHỐI]   ┌──────────────┐          │  vì tờ giấy trên tay SAI LOẠI.          │
│   Err(...)   │ Email  ĐÃ    │          │                                         │
│              │ ĐÓNG DẤU ĐỎ  │          │  Trong Rust: Order<Draft> và            │
│              └──────────────┘          │  Order<Paid> là HAI KIỂU                │
│                                        │  KHÁC NHAU. Trình biên dịch chính là    │
│  Từ đây trở đi, MỌI phòng ban trong    │  nhân viên soát vé — và anh ta KHÔNG    │
│  công ty tin tưởng tuyệt đối tờ giấy   │  BAO GIỜ ngủ gật.                       │
│  có dấu đỏ. Không ai kiểm tra lại!     │                                         │
└────────────────────────────────────────┴─────────────────────────────────────────┘
```

### 1. Phòng công chứng (Kiểu bọc + Hàm khởi tạo có kiểm chứng)

Bạn cầm một mẩu giấy viết tay đến phòng công chứng. Nhân viên kiểm tra kỹ, nếu hợp lệ thì **đóng dấu đỏ** lên và trả lại. Kể từ giây phút đó, mọi phòng ban khác trong công ty nhìn thấy con dấu là tin tưởng ngay — **không ai kiểm tra lại nữa**.

Điều quan trọng: **không ai có thể tự đóng dấu đỏ ở nhà**. Con dấu chỉ nằm trong phòng công chứng. Trong Rust, "con dấu" chính là trường dữ liệu riêng tư của kiểu bọc, và "phòng công chứng" là hàm khởi tạo duy nhất được công khai.

### 2. Cửa kiểm tra sân bay (Typestate)

Ở sân bay, mỗi cửa chỉ nhận đúng **một loại giấy tờ**:
- Cửa check-in nhận *mã đặt chỗ*, trả ra *thẻ lên máy bay*.
- Cửa an ninh nhận *thẻ lên máy bay*, đóng dấu *đã qua kiểm tra*.
- Cửa ra máy bay chỉ nhận thẻ *đã qua kiểm tra*.

Bạn không thể cầm mã đặt chỗ mà đi thẳng ra cửa máy bay — không phải vì có ai chặn bạn lại, mà vì **tờ giấy trên tay bạn sai loại**.

Đó chính xác là Typestate: `Order<Draft>` và `Order<Paid>` là **hai kiểu khác nhau**, nên phương thức `ship` chỉ có trên loại thứ hai. Việc "quên thanh toán" không còn là một lỗi lúc chạy — nó là **lỗi biên dịch**.

---

## Khái niệm & Cơ chế kỹ thuật chuyên sâu (In-Depth Technical Mechanics)

### 1. Đại số của kiểu: vì sao gọi là "tích" và "tổng"

Ở Chương 10 bạn đã nghe cụm từ **Kiểu dữ liệu đại số (Algebraic Data Type)** nhưng chưa ai giải thích chữ "đại số". Bây giờ là lúc.

**Đếm số trạng thái mà một kiểu có thể mang** (gọi là *lực lượng* của kiểu):

| Kiểu | Số giá trị có thể | Vì sao |
|---|---|---|
| `bool` | 2 | `true`, `false` |
| `()` (unit) | 1 | chỉ có đúng một giá trị |
| `Option<bool>` | 3 | `None`, `Some(true)`, `Some(false)` |
| `(bool, bool)` — **struct** | **2 × 2 = 4** | mỗi tổ hợp một trạng thái → **NHÂN** |
| `enum { A, B, C }` — **enum** | **1 + 1 + 1 = 3** | chọn đúng một nhánh → **CỘNG** |
| `Result<bool, ()>` | 2 + 1 = 3 | `Ok(true)`, `Ok(false)`, `Err(())` |

Vậy đó:
- **`struct` là kiểu TÍCH** vì số trạng thái của nó là *tích* các trường: `|A × B| = |A| · |B|`.
- **`enum` là kiểu TỔNG** vì số trạng thái là *tổng* các nhánh: `|A + B| = |A| + |B|`.

Đây không phải trò chơi chữ — nó là **công cụ thiết kế sắc bén nhất trong chương này**. Quy tắc:

> **Kiểu tốt nhất là kiểu có số trạng thái biểu diễn được ĐÚNG BẰNG số trạng thái hợp lệ trong nghiệp vụ. Mỗi trạng thái dư ra là một lỗi đang chờ xảy ra.**

Ví dụ kinh điển:

```rust
// ❌ Kiểu TÍCH: 2 × (1 + n) = có 2 tổ hợp VÔ NGHĨA
struct MalformedOrder {
    is_paid: bool,
    transaction_id: Option<String>,
}
// Tổ hợp 1: is_paid = true,  transaction_id = None      → Đã trả tiền mà không có mã?!
// Tổ hợp 2: is_paid = false, transaction_id = Some(...) → Chưa trả mà có mã giao dịch?!
// Hệ quả: mọi hàm đọc struct này phải viết `if` phòng thủ cho hai trường hợp không thể xảy ra.

// ✅ Kiểu TỔNG: 1 + n = KHÔNG CÒN tổ hợp vô nghĩa nào
enum PaymentState {
    Unpaid,
    Paid { transaction_id: String },
}
// Trình biên dịch bảo đảm: có mã giao dịch ⟺ đã trả tiền. Không cần `if` phòng thủ nào cả.
```

### 2. Kiểu bọc (Newtype) + Hàm khởi tạo có kiểm chứng (Smart Constructor)

Ba thành phần bắt buộc, thiếu một là hỏng cả:

```rust
mod domain {
    // (1) Kiểu bọc với trường RIÊNG TƯ — không có `pub` trước `String`
    #[derive(Debug, Clone, PartialEq, Eq)]
    pub struct Email(String);

    impl Email {
        // (2) Cửa duy nhất để tạo ra giá trị: hàm khởi tạo có kiểm chứng
        pub fn parse(raw: &str) -> Result<Self, String> {
            let s = raw.trim().to_lowercase();
            if !s.contains('@') {
                return Err(format!("Email {:?} thiếu ký tự @", s));
            }
            Ok(Email(s))
        }
        // (3) Cửa để đọc ra (chỉ đọc, không cho sửa)
        pub fn as_str(&self) -> &str { &self.0 }
    }
}
```

Ba điểm cần khắc ghi:

1. **Trường phải riêng tư.** Nếu viết `pub struct Email(pub String)` thì bất kỳ ai cũng gõ được `Email("rác".into())` và toàn bộ bảo đảm sụp đổ. Tính riêng tư **chỉ có hiệu lực qua ranh giới mô-đun** — đây là lý do phải đặt kiểu miền vào một `mod` riêng.
2. **Chi phí lúc chạy bằng không.** `Email` chiếm đúng số byte như `String` bên trong. Đây là *trừu tượng hóa không chi phí* — bạn chỉ trả bằng công gõ phím, không trả bằng hiệu năng.
3. **Trả về `Result`, không `panic`.** Hàm khởi tạo là một *hàm toàn phần* (Chương 13): mọi đầu vào đều có câu trả lời, kể cả đầu vào rác.

### 3. "Phân tích, đừng xác thực" (Parse, don't validate)

Đây là câu khẩu hiệu tóm gọn cả chương, và nó chỉ ra một khác biệt rất tinh tế:

| | Xác thực (Validate) | Phân tích (Parse) |
|---|---|---|
| Chữ ký | `fn check(s: &str) -> bool` | `fn parse(s: &str) -> Result<Email, Error>` |
| Sau khi gọi, bạn có gì? | Một `bool` **rồi vứt đi** | Một **giá trị mang bằng chứng** |
| Ở tầng sau | Vẫn cầm `String` → **phải kiểm tra lại** | Cầm `Email` → khỏi kiểm tra |
| Nguy cơ | Quên gọi hàm kiểm tra ở một nhánh nào đó | Không thể quên — không có `Email` thì không gọi được hàm |

Hãy nhìn lại mã ở Chương 17: chúng ta xác thực email bằng `.filter(|s| s.contains('@'))` rồi trả về một `String` trần. Chuỗi đó **đánh mất toàn bộ bằng chứng hợp lệ ngay khi rời khỏi hàm**. Tầng sau nhận `String` và không có cách nào biết nó đã được kiểm tra hay chưa. Chương 20 sửa đúng điểm này.

### 4. Typestate: mã hóa máy trạng thái vào kiểu

Nghiệp vụ đơn hàng có một máy trạng thái nghiêm ngặt:

```
[Nhập] ──xác thực──► [Đã xác thực] ──thanh toán──► [Đã thanh toán] ──giao hàng──► [Đã giao]
```

Cách thông thường là dùng một `enum` trạng thái rồi kiểm tra lúc chạy:

```rust
// Cách thường: kiểm tra LÚC CHẠY
fn ship(order: &mut Order) -> Result<(), ShipError> {
    if order.state != State::Paid {
        return Err(ShipError::NotPaid);  // ← lỗi này chỉ lộ ra khi chạy tới
    }
    Ok(())
}
```

Typestate đẩy phép kiểm tra đó lên **lúc biên dịch**, bằng cách gắn trạng thái vào *tham số kiểu*:

```rust
use std::marker::PhantomData;

pub struct Draft;          // Các kiểu "thẻ đánh dấu" — không chứa dữ liệu,
pub struct Validated;      // chiếm 0 byte bộ nhớ, chỉ tồn tại lúc biên dịch.
pub struct Paid;
pub struct Delivered;

pub struct Order<TT> {
    id: String,
    lines: Vec<OrderLine>,
    _state: PhantomData<TT>,   // "tôi mang thẻ TT" — 0 byte
}

impl Order<Draft> {
    pub fn validate(self) -> Result<Order<Validated>, DomainError> { /* ... */ }
}
impl Order<Paid> {
    pub fn ship(self, tracking_code: &str) -> Order<Delivered> { /* ... */ }   // ← chỉ tồn tại cho trạng thái này!
}
```

Bây giờ đoạn mã sau **không biên dịch được**:

```rust
let order = Order::new(/* ... */);   // Order<Draft>
order.ship("VN-EXP-1");
// LỖI E0599: no method named `ship` found for struct `Order<Draft>` in the current scope
```

Ba điều đáng chú ý:
- Mỗi phép chuyển trạng thái **tiêu thụ** `self` và trả về kiểu mới → không thể dùng lại đơn hàng ở trạng thái cũ (quyền sở hữu ở Chương 06 đang làm việc cho bạn).
- `PhantomData<TT>` chiếm **0 byte**. Toàn bộ máy trạng thái này biến mất hoàn toàn khi biên dịch.
- Nếu nghiệp vụ thay đổi (thêm bước "chờ duyệt"), trình biên dịch sẽ **liệt kê chính xác mọi chỗ cần sửa**.

> **Đây là chỗ Rust vượt cả F#**: nhờ hệ thống quyền sở hữu, Rust bảo đảm được rằng đơn hàng ở trạng thái cũ **không thể còn tồn tại** sau khi chuyển trạng thái — điều mà ngôn ngữ có bộ gom rác không làm được.

### 5. Biên hệ thống: kiểu truyền tải (DTO) khác kiểu miền

Có một cám dỗ rất lớn: dùng luôn kiểu miền để nhận dữ liệu JSON từ mạng. **Đừng làm vậy.** Hai kiểu này có hai mục đích trái ngược nhau:

| | Kiểu truyền tải (DTO) | Kiểu miền (Domain) |
|---|---|---|
| Mục đích | Nhận **mọi thứ** người ta gửi tới | Chỉ chứa dữ liệu **đã hợp lệ** |
| Cấu trúc | Phẳng, toàn `String` và `Option` | Lồng nhau, dùng kiểu bọc |
| Thái độ | Khoan dung | Nghiêm ngặt |
| Ví dụ | `struct OrderDto { email: String }` | `struct Order { customer: Email }` |

Cầu nối giữa hai thế giới chính là trait **`TryFrom`**:

```rust
impl TryFrom<OrderDto> for Order<Draft> {
    type Error = Vec<DomainError>;   // trả về TẤT CẢ lỗi — Applicative ở Chương 19!
    fn try_from(dto: OrderDto) -> Result<Self, Self::Error> { /* ... */ }
}
```

Đây chính là **cổng công chứng của cả hệ thống**. Mọi dữ liệu từ bên ngoài (HTTP, tệp, cơ sở dữ liệu) đều phải đi qua cửa này. Sau cửa đó, phần còn lại của chương trình sống trong một thế giới nơi mọi dữ liệu đều hợp lệ.

### 6. Kiến trúc "Lõi thuần túy — Vỏ mệnh lệnh"

Đây là hình dạng kiến trúc gói trọn tám chương lập trình hàm:

```
        ┌───────────────────────────────────────────────────────────┐
        │  VỎ MỆNH LỆNH (Imperative Shell) — mỏng, khó kiểm thử     │
        │  · Đọc HTTP / tệp / CSDL / đồng hồ / số ngẫu nhiên         │
        │  · Ghi log, gửi email, in màn hình                        │
        │                                                           │
        │      ┌─────────────────────────────────────────────┐      │
        │      │  LÕI THUẦN TÚY (Functional Core) — dày,     │      │
        │      │  100% hàm thuần túy, kiểm thử cực dễ        │      │
        │      │  · Kiểu miền + hàm khởi tạo có kiểm chứng   │      │
        │      │  · Quy tắc nghiệp vụ, tính giá, chuyển trạng│      │
        │      │  · KHÔNG có I/O, KHÔNG đọc đồng hồ          │      │
        │      └─────────────────────────────────────────────┘      │
        └───────────────────────────────────────────────────────────┘
```

Nguyên tắc: **đẩy mọi tác dụng phụ ra sát rìa**. Vỏ đọc dữ liệu → chuyển thành kiểu miền → gọi lõi thuần túy → nhận kết quả → vỏ ghi kết quả ra ngoài.

Lợi ích cụ thể: lõi thuần túy kiểm thử được **không cần cơ sở dữ liệu, không cần mạng, không cần thư viện giả lập** — vì nó chỉ là hàm nhận vào giá trị và trả ra giá trị. Đó cũng chính là lý do bạn học tiêm phụ thuộc bằng áp dụng từng phần ở Chương 14.

---

## Mã nguồn minh họa thực chiến (Idiomatic Runnable Rust Blueprint)

Chương trình dưới đây mô hình hóa **Quy trình Tiếp nhận Đơn hàng (Order-Taking Workflow)** — chính là miền nghiệp vụ được dùng xuyên suốt cuốn *Domain Modeling Made Functional*, viết lại bằng Rust.

```rust
// Tệp: src/main.rs
// Chương trình thực chiến: Kiểu bọc, Hàm khởi tạo có kiểm chứng và Typestate

use std::convert::TryFrom;
use std::marker::PhantomData;

// ============================================================================
// PHẦN 1: MÔ-ĐUN MIỀN NGHIỆP VỤ
// Đặt trong `mod` để tính RIÊNG TƯ của các trường thực sự có hiệu lực.
// ============================================================================
pub mod domain {
    use std::fmt;

    #[derive(Debug, Clone, PartialEq, Eq)]
    pub enum DomainError {
        InvalidEmail(String),
        InvalidProductName(String),
        InvalidQuantity(String),
        EmptyOrder,
        OrderTooLarge { line_count: usize, max: usize },
    }

    impl fmt::Display for DomainError {
        fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
            match self {
                DomainError::InvalidEmail(s) => write!(f, "Email không hợp lệ: {}", s),
                DomainError::InvalidProductName(s) => write!(f, "Tên sản phẩm không hợp lệ: {}", s),
                DomainError::InvalidQuantity(s) => write!(f, "Số lượng không hợp lệ: {}", s),
                DomainError::EmptyOrder => write!(f, "Đơn hàng phải có ít nhất 1 dòng hàng"),
                DomainError::OrderTooLarge { line_count, max } => {
                    write!(f, "Đơn có {} dòng, vượt giới hạn {} dòng", line_count, max)
                }
            }
        }
    }

    // ---------------------------------------------------------------------
    // KIỂU BỌC 1: Email — trường riêng tư, chỉ tạo được qua `parse`
    // ---------------------------------------------------------------------
    #[derive(Debug, Clone, PartialEq, Eq)]
    pub struct Email(String); // KHÔNG có `pub` trước String → đây là con dấu

    impl Email {
        pub fn parse(raw: &str) -> Result<Self, DomainError> {
            let s = raw.trim().to_lowercase();
            if s.is_empty() {
                return Err(DomainError::InvalidEmail("chuỗi rỗng".to_string()));
            }
            let parts: Vec<&str> = s.split('@').collect();
            if parts.len() != 2 || parts[0].is_empty() || !parts[1].contains('.') {
                return Err(DomainError::InvalidEmail(s));
            }
            Ok(Email(s))
        }
        pub fn as_str(&self) -> &str {
            &self.0
        }
    }

    impl fmt::Display for Email {
        fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
            write!(f, "{}", self.0)
        }
    }

    // ---------------------------------------------------------------------
    // KIỂU BỌC 2: ProductName — chuỗi có giới hạn độ dài
    // ---------------------------------------------------------------------
    #[derive(Debug, Clone, PartialEq, Eq)]
    pub struct ProductName(String);

    impl ProductName {
        pub const MAX: usize = 50;

        pub fn parse(raw: &str) -> Result<Self, DomainError> {
            let s = raw.trim();
            let char_count = s.chars().count(); // đếm CHỮ CÁI, không đếm byte (Chương 05)
            if char_count == 0 {
                Err(DomainError::InvalidProductName("chuỗi rỗng".to_string()))
            } else if char_count > Self::MAX {
                Err(DomainError::InvalidProductName(format!(
                    "dài {} ký tự, tối đa {}",
                    char_count,
                    Self::MAX
                )))
            } else {
                Ok(ProductName(s.to_string()))
            }
        }
        pub fn as_str(&self) -> &str {
            &self.0
        }
    }

    // ---------------------------------------------------------------------
    // KIỂU BỌC 3: Quantity — số nguyên dương trong khoảng cho phép
    // ---------------------------------------------------------------------
    #[derive(Debug, Clone, Copy, PartialEq, Eq)]
    pub struct Quantity(u32);

    impl Quantity {
        pub const MAX: u32 = 1000;

        pub fn parse(n: u32) -> Result<Self, DomainError> {
            if n == 0 {
                Err(DomainError::InvalidQuantity("phải lớn hơn 0".to_string()))
            } else if n > Self::MAX {
                Err(DomainError::InvalidQuantity(format!(
                    "{} vượt quá {}",
                    n,
                    Self::MAX
                )))
            } else {
                Ok(Quantity(n))
            }
        }
        pub fn value(&self) -> u32 {
            self.0
        }
    }

    // ---------------------------------------------------------------------
    // KIỂU BỌC 4: Money — tính bằng ĐƠN VỊ NHỎ NHẤT (đồng), dùng u64.
    // KHÔNG BAO GIỜ dùng f64 cho tiền tệ (xem cảnh báo ở Chương 03)!
    // ---------------------------------------------------------------------
    #[derive(Debug, Clone, Copy, PartialEq, Eq, PartialOrd, Ord)]
    pub struct Money(u64);

    impl Money {
        pub fn vnd(n: u64) -> Self {
            Money(n)
        }
        pub fn value(&self) -> u64 {
            self.0
        }
        pub fn plus(self, other: Money) -> Money {
            Money(self.0 + other.0) // đây là một VỊ NHÓM (Chương 18)!
        }
        pub fn subtract(self, other: Money) -> Money {
            Money(self.0.saturating_sub(other.0))
        }
        pub fn times(self, factor: u32) -> Money {
            Money(self.0 * factor as u64)
        }
    }

    impl fmt::Display for Money {
        fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
            write!(f, "{} đ", self.0)
        }
    }

    // ---------------------------------------------------------------------
    // KIỂU TỔNG: cách thanh toán — KHÔNG CÒN tổ hợp vô nghĩa
    // ---------------------------------------------------------------------
    #[derive(Debug, Clone, PartialEq, Eq)]
    pub enum PaymentMethod {
        Cash,
        Transfer { transaction_id: String },
        Card { last_four: String },
    }

    // ---------------------------------------------------------------------
    // Dòng hàng: một kiểu TÍCH gồm toàn kiểu đã được công chứng
    // ---------------------------------------------------------------------
    #[derive(Debug, Clone, PartialEq, Eq)]
    pub struct OrderLine {
        pub name: ProductName,
        pub quantity: Quantity,
        pub unit_price: Money,
    }

    impl OrderLine {
        pub fn line_total(&self) -> Money {
            self.unit_price.times(self.quantity.value())
        }
    }
}

use domain::*;

// ============================================================================
// PHẦN 2: TYPESTATE — MÁY TRẠNG THÁI ĐƯỢC MÃ HÓA VÀO KIỂU
// ============================================================================

/// Bốn "thẻ đánh dấu" trạng thái. Chúng chiếm 0 byte và biến mất khi biên dịch.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct Draft;
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct Validated;
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct Paid;
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct Delivered;

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct Order<TT> {
    id: String,
    customer: Email,
    lines: Vec<OrderLine>,
    payment: Option<PaymentMethod>,
    _state: PhantomData<TT>,
}

/// Các phương thức dùng chung cho MỌI trạng thái.
impl<TT> Order<TT> {
    pub fn id(&self) -> &str {
        &self.id
    }
    pub fn customer(&self) -> &Email {
        &self.customer
    }
    pub fn line_count(&self) -> usize {
        self.lines.len()
    }
    /// Tổng tiền = gộp các thành tiền bằng phép cộng của vị nhóm Money.
    pub fn total(&self) -> Money {
        self.lines
            .iter()
            .map(|d| d.line_total())
            .fold(Money::vnd(0), |a, b| a.plus(b))
    }
}

pub const MAX_LINES: usize = 20;

/// Trạng thái NHẬP: chỉ có đúng một hành động hợp lệ — xác thực.
impl Order<Draft> {
    pub fn new(id: &str, customer: Email, lines: Vec<OrderLine>) -> Self {
        Order {
            id: id.to_string(),
            customer,
            lines,
            payment: None,
            _state: PhantomData,
        }
    }

    pub fn validate(self) -> Result<Order<Validated>, DomainError> {
        if self.lines.is_empty() {
            return Err(DomainError::EmptyOrder);
        }
        if self.lines.len() > MAX_LINES {
            return Err(DomainError::OrderTooLarge {
                line_count: self.lines.len(),
                max: MAX_LINES,
            });
        }
        Ok(Order {
            id: self.id,
            customer: self.customer,
            lines: self.lines,
            payment: None,
            _state: PhantomData,
        })
    }
}

/// Trạng thái ĐÃ XÁC THỰC: chỉ có thể thanh toán.
impl Order<Validated> {
    pub fn pay(self, method: PaymentMethod) -> Order<Paid> {
        Order {
            id: self.id,
            customer: self.customer,
            lines: self.lines,
            payment: Some(method),
            _state: PhantomData,
        }
    }
}

/// Trạng thái ĐÃ THANH TOÁN: chỉ có thể giao hàng.
impl Order<Paid> {
    pub fn payment_method(&self) -> &PaymentMethod {
        // An toàn tuyệt đối: chỉ trạng thái này mới tồn tại, và nó LUÔN có thanh toán.
        self.payment
            .as_ref()
            .expect("bất biến của Order<Paid>: luôn có thông tin thanh toán")
    }

    pub fn ship(self, tracking_code: &str) -> Order<Delivered> {
        println!(
            "   [VỎ MỆNH LỆNH] Gửi email tới {} về vận đơn {}",
            self.customer, tracking_code
        );
        Order {
            id: self.id,
            customer: self.customer,
            lines: self.lines,
            payment: self.payment,
            _state: PhantomData,
        }
    }
}

// ============================================================================
// PHẦN 3: BIÊN HỆ THỐNG — DTO VÀ CỔNG CÔNG CHỨNG `TryFrom`
// ============================================================================

/// Kiểu TRUYỀN TẢI: khoan dung, phẳng, toàn chuỗi — đúng như JSON gửi tới.
#[derive(Debug, Clone)]
pub struct OrderDto {
    pub id: String,
    pub email: String,
    pub lines: Vec<OrderLineDto>,
}

#[derive(Debug, Clone)]
pub struct OrderLineDto {
    pub name: String,
    pub quantity: u32,
    pub unit_price: u64,
}

impl TryFrom<OrderDto> for Order<Draft> {
    /// Trả về TẤT CẢ lỗi cùng lúc — đúng tinh thần Applicative ở Chương 19.
    type Error = Vec<DomainError>;

    fn try_from(dto: OrderDto) -> Result<Self, Self::Error> {
        let mut errors: Vec<DomainError> = Vec::new();

        let customer = match Email::parse(&dto.email) {
            Ok(e) => Some(e),
            Err(e) => {
                errors.push(e);
                None
            }
        };

        let mut lines: Vec<OrderLine> = Vec::new();
        for d in &dto.lines {
            let name = ProductName::parse(&d.name);
            let qty = Quantity::parse(d.quantity);
            match (name, qty) {
                (Ok(t), Ok(s)) => lines.push(OrderLine {
                    name: t,
                    quantity: s,
                    unit_price: Money::vnd(d.unit_price),
                }),
                (t, s) => {
                    if let Err(e) = t {
                        errors.push(e);
                    }
                    if let Err(e) = s {
                        errors.push(e);
                    }
                }
            }
        }

        match customer {
            Some(customer) if errors.is_empty() => Ok(Order::new(&dto.id, customer, lines)),
            _ => Err(errors),
        }
    }
}

// ============================================================================
// PHẦN 4: LÕI THUẦN TÚY — QUY TẮC NGHIỆP VỤ, KHÔNG CÓ MỘT DÒNG I/O NÀO
// ============================================================================

/// Tính phí vận chuyển theo tổng tiền. Hàm thuần túy 100%: dễ kiểm thử tuyệt đối.
pub fn shipping_fee(total: Money) -> Money {
    if total.value() >= 500_000 {
        Money::vnd(0) // miễn phí cho đơn từ 500k
    } else {
        Money::vnd(30_000)
    }
}

/// Tính chiết khấu theo số dòng hàng. Cũng thuần túy 100%.
pub fn discount_for(total: Money, line_count: usize) -> Money {
    let percent = if line_count >= 10 {
        10
    } else if line_count >= 5 {
        5
    } else {
        0
    };
    Money::vnd(total.value() * percent / 100)
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct Invoice {
    pub subtotal: Money,
    pub discount: Money,
    pub shipping: Money,
    pub total_payable: Money,
}

/// Toàn bộ phép tính hóa đơn — vẫn hoàn toàn thuần túy.
pub fn build_invoice(order: &Order<Validated>) -> Invoice {
    let subtotal = order.total();
    let discount = discount_for(subtotal, order.line_count());
    let after_discount = subtotal.subtract(discount);
    let fee = shipping_fee(after_discount);
    Invoice {
        subtotal,
        discount,
        shipping: fee,
        total_payable: after_discount.plus(fee),
    }
}

// ============================================================================
// CHƯƠNG TRÌNH ĐIỀU HÀNH CHÍNH (VỎ MỆNH LỆNH)
// ============================================================================

fn main() {
    println!("============================================================");
    println!("   MÔ HÌNH HÓA NGHIỆP VỤ BẰNG KIỂU: NEWTYPE & TYPESTATE    ");
    println!("============================================================");

    // ------------------------------------------------------------------
    // 1. HÀM KHỞI TẠO CÓ KIỂM CHỨNG — PHÒNG CÔNG CHỨNG
    // ------------------------------------------------------------------
    println!("\n1. PHÒNG CÔNG CHỨNG (Smart Constructor)");
    for raw in [
        "  An.Nguyen@Example.COM ",
        "khong-co-a-cong",
        "@thieu-ten.vn",
        "",
    ] {
        match Email::parse(raw) {
            Ok(e) => println!("   {:>28} -> ✓ đóng dấu: {}", format!("{:?}", raw), e),
            Err(err) => println!("   {:>28} -> ✗ từ chối: {}", format!("{:?}", raw), err),
        }
    }
    println!("   → Không có cách nào tạo ra một `Email` sai. Trường bên trong là riêng tư.");

    // ------------------------------------------------------------------
    // 2. ĐẠI SỐ CỦA KIỂU — ĐẾM SỐ TRẠNG THÁI
    // ------------------------------------------------------------------
    println!("\n2. ĐẠI SỐ CỦA KIỂU");
    println!("   struct (bool, bool)        -> kiểu TÍCH: 2 × 2 = 4 trạng thái");
    println!("   enum {{ Cash, Transfer, Card }} -> kiểu TỔNG: 1 + 1 + 1 = 3 trạng thái");
    println!("   Cách SAI : struct {{ is_paid: bool, transaction_id: Option<String> }}");
    println!("              -> có 2 tổ hợp VÔ NGHĨA (đã trả mà không mã / chưa trả mà có mã)");
    println!("   Cách ĐÚNG: enum {{ Unpaid, Paid {{ transaction_id }} }} -> 0 tổ hợp vô nghĩa ✓");

    // ------------------------------------------------------------------
    // 3. CỔNG BIÊN HỆ THỐNG: DTO -> KIỂU MIỀN, GOM HẾT LỖI
    // ------------------------------------------------------------------
    println!("\n3. CỔNG BIÊN HỆ THỐNG (DTO -> Miền), gom TẤT CẢ lỗi");
    let bad_dto = OrderDto {
        id: "ORD-0001".to_string(),
        email: "sai-email".to_string(),
        lines: vec![
            OrderLineDto {
                name: "".to_string(),
                quantity: 0,
                unit_price: 100,
            },
            OrderLineDto {
                name: "Bàn phím cơ".to_string(),
                quantity: 2,
                unit_price: 1_200_000,
            },
        ],
    };
    match Order::try_from(bad_dto) {
        Ok(_) => println!("   (không tới đây)"),
        Err(errors) => {
            println!("   Từ chối đơn hàng với {} lỗi:", errors.len());
            for (i, err) in errors.iter().enumerate() {
                println!("     {}. {}", i + 1, err);
            }
        }
    }

    // ------------------------------------------------------------------
    // 4. ĐƠN HỢP LỆ ĐI QUA TOÀN BỘ MÁY TRẠNG THÁI
    // ------------------------------------------------------------------
    println!("\n4. TYPESTATE — QUY TRÌNH ĐƠN HÀNG");
    let good_dto = OrderDto {
        id: "ORD-0002".to_string(),
        email: "  Khach.Hang@Shop.VN  ".to_string(),
        lines: vec![
            OrderLineDto {
                name: "Bàn phím cơ không dây".to_string(),
                quantity: 2,
                unit_price: 1_200_000,
            },
            OrderLineDto {
                name: "Chuột công thái học".to_string(),
                quantity: 1,
                unit_price: 750_000,
            },
            OrderLineDto {
                name: "Lót chuột cỡ lớn".to_string(),
                quantity: 3,
                unit_price: 150_000,
            },
        ],
    };

    let draft_order: Order<Draft> = Order::try_from(good_dto).expect("đơn này phải hợp lệ");
    println!(
        "   [Nhập]          mã={} khách={} số dòng={}",
        draft_order.id(),
        draft_order.customer(),
        draft_order.line_count()
    );

    let validated_order: Order<Validated> = draft_order.validate().expect("đơn có 3 dòng, hợp lệ");
    println!("   [Đã xác thực]   tổng hàng = {}", validated_order.total());

    // ---- LÕI THUẦN TÚY: lập hóa đơn (không I/O, kiểm thử được ngay) ----
    let invoice = build_invoice(&validated_order);
    println!("   ┌─ HÓA ĐƠN (tính bởi LÕI THUẦN TÚY) ─────────────");
    println!("   │ Tạm tính        : {}", invoice.subtotal);
    println!("   │ Chiết khấu      : {}", invoice.discount);
    println!("   │ Phí vận chuyển  : {}", invoice.shipping);
    println!("   │ TỔNG THANH TOÁN : {}", invoice.total_payable);
    println!("   └────────────────────────────────────────────────");

    let paid_order: Order<Paid> = validated_order.pay(PaymentMethod::Transfer {
        transaction_id: "VCB-99881234".to_string(),
    });
    println!(
        "   [Đã thanh toán] cách trả = {:?}",
        paid_order.payment_method()
    );

    let _delivered_order: Order<Delivered> = paid_order.ship("VN-EXP-77213");
    println!("   [Đã giao]       hoàn tất quy trình ✓");

    // ------------------------------------------------------------------
    // 5. NHỮNG GÌ TRÌNH BIÊN DỊCH TỪ CHỐI
    // ------------------------------------------------------------------
    println!("\n5. TRÌNH BIÊN DỊCH LÀ NHÂN VIÊN SOÁT VÉ KHÔNG BAO GIỜ NGỦ GẬT");
    // Bỏ chú thích từng dòng dưới đây để tận mắt thấy lỗi biên dịch:
    // let draft = Order::new("X", Email::parse("a@b.vn").unwrap(), vec![]);
    // draft.ship("VD-1");                 // E0599: Order<Draft> không có `ship`
    // let fake = domain::Email("rác".into()); // E0603: hàm dựng của Email là riêng tư
    // validated_order.total();            // E0382: validated_order đã bị `pay` tiêu thụ
    println!("   Các dòng sau KHÔNG BIÊN DỊCH ĐƯỢC (đã đóng chú thích trong mã nguồn):");
    println!("     · draft.ship(...)           -> E0599: Order<Draft> không có `ship`");
    println!("     · domain::Email(\"rác\".into()) -> E0603: hàm dựng riêng tư, không dựng được");
    println!("     · validated_order.total()   -> E0382: đơn đã bị `pay` tiêu thụ");
    println!("   → Ba lớp lỗi nghiệp vụ bị xóa sổ TRƯỚC khi chương trình kịp chạy.");

    // ------------------------------------------------------------------
    // 6. ĐƠN VI PHẠM QUY TẮC NGHIỆP VỤ
    // ------------------------------------------------------------------
    println!("\n6. XÁC THỰC QUY TẮC NGHIỆP VỤ");
    let email = Email::parse("test@shop.vn").unwrap();
    let empty_order: Order<Draft> = Order::new("ORD-0003", email, vec![]);
    match empty_order.validate() {
        Ok(_) => println!("   (không tới đây)"),
        Err(err) => println!("   Đơn rỗng bị chặn: {}", err),
    }

    println!("\n============================================================");
    println!("  TRẠNG THÁI SAI KHÔNG BIỂU DIỄN ĐƯỢC = LỖI KHÔNG XẢY RA    ");
    println!("============================================================");
}

// ============================================================================
// KIỂM THỬ: LÕI THUẦN TÚY KIỂM THỬ ĐƯỢC MÀ KHÔNG CẦN CSDL, MẠNG HAY MOCK
// ============================================================================

#[cfg(test)]
mod tests {
    use super::*;

    fn sample_order() -> Order<Validated> {
        let email = Email::parse("khach@shop.vn").unwrap();
        let lines = vec![
            OrderLine {
                name: ProductName::parse("Bàn phím").unwrap(),
                quantity: Quantity::parse(2).unwrap(),
                unit_price: Money::vnd(100_000),
            },
            OrderLine {
                name: ProductName::parse("Chuột").unwrap(),
                quantity: Quantity::parse(1).unwrap(),
                unit_price: Money::vnd(50_000),
            },
        ];
        Order::new("ORD-TEST", email, lines).validate().unwrap()
    }

    #[test]
    fn email_accepts_valid_address() {
        let e = Email::parse("  An.Nguyen@Example.COM ").unwrap();
        assert_eq!(e.as_str(), "an.nguyen@example.com"); // đã chuẩn hóa
    }

    #[test]
    fn email_rejects_invalid_address() {
        for bad in [
            "",
            "   ",
            "khong-co-a-cong",
            "@thieu-ten.vn",
            "a@b@c.vn",
            "a@khongcocham",
        ] {
            assert!(Email::parse(bad).is_err(), "phải từ chối {:?}", bad);
        }
    }

    #[test]
    fn quantity_must_be_positive_and_bounded() {
        assert!(Quantity::parse(0).is_err());
        assert!(Quantity::parse(1001).is_err());
        assert_eq!(Quantity::parse(5).unwrap().value(), 5);
    }

    #[test]
    fn product_name_counts_chars_not_bytes() {
        // 50 chữ cái tiếng Việt có dấu = nhiều hơn 50 BYTE, nhưng vẫn hợp lệ.
        let name_50: String = "ế".repeat(50);
        assert!(ProductName::parse(&name_50).is_ok());
        let name_51: String = "ế".repeat(51);
        assert!(ProductName::parse(&name_51).is_err());
    }

    #[test]
    fn empty_order_is_rejected() {
        let email = Email::parse("a@b.vn").unwrap();
        let order = Order::new("X", email, vec![]);
        assert_eq!(order.validate().unwrap_err(), DomainError::EmptyOrder);
    }

    #[test]
    fn dto_collects_all_errors_at_once() {
        let dto = OrderDto {
            id: "X".to_string(),
            email: "sai".to_string(),
            lines: vec![OrderLineDto {
                name: "".to_string(),
                quantity: 0,
                unit_price: 1,
            }],
        };
        let errors = Order::try_from(dto).unwrap_err();
        assert_eq!(errors.len(), 3, "phải gom đủ 3 lỗi, nhận được {:?}", errors);
    }

    // ---- Kiểm thử LÕI THUẦN TÚY: không cần CSDL, không cần mạng ----

    #[test]
    fn total_sums_line_amounts() {
        let order = sample_order();
        // 2 × 100.000 + 1 × 50.000 = 250.000
        assert_eq!(order.total(), Money::vnd(250_000));
    }

    #[test]
    fn shipping_is_free_above_threshold() {
        assert_eq!(shipping_fee(Money::vnd(499_999)), Money::vnd(30_000));
        assert_eq!(shipping_fee(Money::vnd(500_000)), Money::vnd(0));
    }

    #[test]
    fn discount_tiers_by_line_count() {
        let total = Money::vnd(1_000_000);
        assert_eq!(discount_for(total, 3), Money::vnd(0));
        assert_eq!(discount_for(total, 5), Money::vnd(50_000));
        assert_eq!(discount_for(total, 12), Money::vnd(100_000));
    }

    #[test]
    fn invoice_totals_are_correct() {
        let order = sample_order(); // tạm tính 250.000, 2 dòng -> không chiết khấu
        let invoice = build_invoice(&order);
        assert_eq!(invoice.subtotal, Money::vnd(250_000));
        assert_eq!(invoice.discount, Money::vnd(0));
        assert_eq!(invoice.shipping, Money::vnd(30_000));
        assert_eq!(invoice.total_payable, Money::vnd(280_000));
    }

    #[test]
    fn typestate_flow_runs_all_four_steps() {
        let order = sample_order();
        let paid = order.pay(PaymentMethod::Cash);
        assert_eq!(paid.payment_method(), &PaymentMethod::Cash);
        let delivered = paid.ship("VD-001");
        assert_eq!(delivered.id(), "ORD-TEST");
    }

    #[test]
    fn typestate_is_zero_cost_at_runtime() {
        use std::mem::size_of;
        // PhantomData chiếm 0 byte: Order<Draft> và Order<Delivered> có cùng kích thước.
        assert_eq!(size_of::<Order<Draft>>(), size_of::<Order<Delivered>>());
        assert_eq!(size_of::<Draft>(), 0);
        assert_eq!(size_of::<PhantomData<Delivered>>(), 0);
    }
}
```

---

## Bảng tra cứu lỗi biên dịch & Cách khắc phục (Compiler Error Guide)

| Mã lỗi | Thông báo mẫu từ trình biên dịch | Nguyên nhân cốt lõi | Cách khắc phục nhanh |
|---|---|---|---|
| **E0603** | `tuple struct constructor 'Email' is private` | **Đây là lỗi TỐT!** Nó chứng minh kiểu bọc đang bảo vệ bạn: ai đó cố tạo `Email` mà không đi qua phòng công chứng. | Gọi `Email::parse(...)` thay vì `Email(...)`. Đừng bao giờ "sửa" bằng cách thêm `pub` vào trường. |
| **E0599** | `no method named 'ship' found for struct 'Order<Draft>'` | **Cũng là lỗi TỐT!** Typestate đang chặn một bước nhảy cóc trong quy trình. | Đi đúng thứ tự: `.validate()?` rồi `.pay(...)` rồi mới `.ship(...)`. |
| **E0382** | `borrow of moved value: 'validated_order'` (hoặc `use of moved value`) | Mỗi phép chuyển trạng thái **tiêu thụ** `self`, nên đơn hàng ở trạng thái cũ không còn tồn tại. | Đó là chủ ý thiết kế. Dùng biến mới cho mỗi trạng thái, hoặc `#[derive(Clone)]` nếu thật sự cần bản sao. |
| **E0392** | `type parameter 'TT' is never used` | Bạn khai báo `struct Order<TT>` mà không dùng `TT` trong bất kỳ trường nào. | Thêm trường `_state: PhantomData<TT>` — đây chính là lý do `PhantomData` tồn tại. |
| **E0277** | `` `Draft` doesn't implement `Debug` `` | `#[derive(Debug)]` trên kiểu generic sinh ra ràng buộc `TT: Debug`, nên in `Order<Draft>` bằng `{:?}` đòi `Draft: Debug`. | Thêm `#[derive(Debug)]` cho các kiểu thẻ đánh dấu (`Draft`, `Delivered`…), hoặc tự viết `impl Debug`. |

### Phân tích lỗi thực tế `E0603` — khi lỗi biên dịch là dấu hiệu thành công:

```rust
// ❌ Đoạn mã lỗi (đã đóng chú thích để tệp vẫn biên dịch được):
// let e = domain::Email("đây không phải email".to_string());
// LỖI E0603: tuple struct constructor `Email` is private
//
// Đây KHÔNG phải sự cố cần khắc phục — đây là bằng chứng thiết kế đang hoạt động!
// Nếu đoạn mã trên biên dịch được, mọi bảo đảm của kiểu `Email` đều vô nghĩa.

// ✅ Cách duy nhất được phép:
// let e = domain::Email::parse("kh@shop.vn").expect("địa chỉ hợp lệ");
```

> **Nguyên tắc vàng khi gặp E0603 với kiểu bọc**: đừng bao giờ thêm `pub` vào trường để "cho nhanh". Cái `pub` đó xóa sổ toàn bộ lợi ích của chương này.

---

## Tóm tắt chương & Bài tập rèn luyện (Summary & Exercises)

### 4 Điểm cốt lõi cần ghi nhớ:
1. **Đếm số trạng thái trước khi thiết kế kiểu.** `struct` là kiểu tích (nhân), `enum` là kiểu tổng (cộng). Mỗi tổ hợp dư ra là một lỗi đang chờ xảy ra — hãy chọn kiểu có đúng số trạng thái hợp lệ.
2. **Phân tích, đừng xác thực.** Kiểm tra một lần ở cổng vào rồi trả về một *kiểu mang bằng chứng*. Ba thành phần bắt buộc: trường riêng tư + hàm khởi tạo trả `Result` + kiểu nằm trong `mod` riêng.
3. **Typestate biến lỗi quy trình thành lỗi biên dịch.** `PhantomData` chiếm 0 byte, nên toàn bộ máy trạng thái biến mất khi biên dịch — bạn được an toàn hoàn toàn miễn phí.
4. **Lõi thuần túy — vỏ mệnh lệnh.** Đẩy mọi I/O ra sát rìa; lõi chỉ nhận giá trị và trả giá trị. Nhờ vậy toàn bộ quy tắc nghiệp vụ kiểm thử được mà không cần cơ sở dữ liệu, mạng hay thư viện giả lập nào.

### Bài tập rèn luyện tự giải:

**Bài tập 1 (Kiểu bọc `VnPhone`)**
Viết kiểu bọc `VnPhone` (số điện thoại Việt Nam) với hàm khởi tạo có kiểm chứng: chấp nhận chuỗi chỉ gồm chữ số (cho phép có khoảng trắng và dấu chấm ở giữa), độ dài sau khi làm sạch là 10 chữ số và bắt đầu bằng `0`. Chuẩn hóa kết quả về dạng không dấu cách. Viết ít nhất 4 bài kiểm thử.

<details>
<summary><b>Gợi ý</b></summary>

Dùng `chars().filter(|c| c.is_ascii_digit()).collect::<String>()` để làm sạch. Nhớ đặt kiểu trong một `mod` và **không** đánh `pub` cho trường bên trong.
</details>

<details>
<summary><b>Lời giải</b></summary>

```rust
pub mod contact {
    #[derive(Debug, Clone, PartialEq, Eq)]
    pub struct VnPhone(String);

    impl VnPhone {
        pub fn parse(raw: &str) -> Result<Self, String> {
            let clean: String = raw.chars().filter(|c| c.is_ascii_digit()).collect();
            if clean.len() != 10 {
                return Err(format!("Cần đúng 10 chữ số, nhận được {}", clean.len()));
            }
            if !clean.starts_with('0') {
                return Err("Số điện thoại phải bắt đầu bằng 0".to_string());
            }
            Ok(VnPhone(clean))
        }
        pub fn as_str(&self) -> &str { &self.0 }
    }
}

#[cfg(test)]
mod t {
    use super::contact::VnPhone;

    #[test] fn accepts_valid_number() {
        assert_eq!(VnPhone::parse("0912 345 678").unwrap().as_str(), "0912345678");
    }
    #[test] fn accepts_dotted_format() {
        assert_eq!(VnPhone::parse("098.765.4321").unwrap().as_str(), "0987654321");
    }
    #[test] fn rejects_too_few_digits() { assert!(VnPhone::parse("0912345").is_err()); }
    #[test] fn rejects_missing_leading_zero() { assert!(VnPhone::parse("1912345678").is_err()); }
}
```
</details>

**Bài tập 2 (Xóa trạng thái vô nghĩa)**
Cho struct sau đây, hãy đếm số trạng thái nó biểu diễn được, chỉ ra những tổ hợp vô nghĩa, rồi thiết kế lại bằng `enum` sao cho **không còn tổ hợp vô nghĩa nào**:

```rust
struct Account {
    activated: bool,
    activated_on: Option<String>,
    lock_reason: Option<String>,
}
```

<details>
<summary><b>Gợi ý</b></summary>

Liệt kê các trạng thái nghiệp vụ *thật sự* tồn tại của một tài khoản: chờ kích hoạt, đang hoạt động, bị khóa. Mỗi trạng thái cần mang theo *đúng* dữ liệu nào?
</details>

<details>
<summary><b>Lời giải</b></summary>

**Đếm trạng thái**: `2 × (1 + n) × (1 + m)` — với `n`, `m` là số chuỗi có thể. Ngay cả khi rút gọn `Option` thành "có/không", ta đã có `2 × 2 × 2 = 8` tổ hợp, trong khi nghiệp vụ chỉ có **3** trạng thái thật. Năm tổ hợp vô nghĩa, ví dụ:
- `activated = true` nhưng `activated_on = None` → hoạt động mà không rõ từ bao giờ?
- `activated = true` và `lock_reason = Some(...)` → vừa hoạt động vừa bị khóa?
- `activated = false`, `activated_on = Some(...)` → đã kích hoạt rồi mà lại chưa kích hoạt?

**Thiết kế lại**:

```rust
#[derive(Debug, Clone, PartialEq, Eq)]
pub enum Account {
    PendingActivation,
    Active { activated_on: String },
    Locked { activated_on: String, reason: String },
}

impl Account {
    pub fn activate(self, date: String) -> Result<Self, &'static str> {
        match self {
            Account::PendingActivation => Ok(Account::Active { activated_on: date }),
            _ => Err("Tài khoản đã được kích hoạt trước đó"),
        }
    }
    pub fn lock(self, reason: String) -> Result<Self, &'static str> {
        match self {
            Account::Active { activated_on } =>
                Ok(Account::Locked { activated_on, reason }),
            _ => Err("Chỉ khóa được tài khoản đang hoạt động"),
        }
    }
}
```

Đúng 3 trạng thái, 0 tổ hợp vô nghĩa. Và nhờ `match` vét cạn, khi bạn thêm trạng thái thứ tư sau này, trình biên dịch sẽ chỉ ra **chính xác** mọi nơi cần cập nhật.
</details>

**Bài tập 3 (Typestate cho kết nối cơ sở dữ liệu)**
Thiết kế typestate cho một kết nối cơ sở dữ liệu với ba trạng thái: `Disconnected` (chưa kết nối) → `Connected` (đã kết nối) → `InTransaction` (trong giao dịch). Yêu cầu:
- Chỉ `Connected` mới có phương thức `begin_transaction()`.
- Chỉ `InTransaction` mới có `query()`, `commit()` và `rollback()`.
- `commit()` và `rollback()` đưa kết nối trở về trạng thái `Connected`.

<details>
<summary><b>Gợi ý</b></summary>

Mẫu giống hệt `Order<TT>`. Điểm mới: `commit` và `rollback` đi **ngược** về `Connection<Connected>` — điều đó hoàn toàn hợp lệ, vì typestate không bắt buộc phải là đường một chiều.
</details>

<details>
<summary><b>Lời giải</b></summary>

```rust
use std::marker::PhantomData;

pub struct Disconnected;
pub struct Connected;
pub struct InTransaction;

pub struct Connection<TT> {
    connection_string: String,
    statement_log: Vec<String>,
    _tt: PhantomData<TT>,
}

impl Connection<Disconnected> {
    pub fn new(conn_str: &str) -> Self {
        Connection { connection_string: conn_str.to_string(), statement_log: Vec::new(), _tt: PhantomData }
    }
    pub fn connect(self) -> Result<Connection<Connected>, String> {
        if self.connection_string.is_empty() {
            return Err("Chuỗi kết nối rỗng".to_string());
        }
        Ok(Connection { connection_string: self.connection_string, statement_log: self.statement_log, _tt: PhantomData })
    }
}

impl Connection<Connected> {
    pub fn begin_transaction(self) -> Connection<InTransaction> {
        Connection { connection_string: self.connection_string, statement_log: self.statement_log, _tt: PhantomData }
    }
}

impl Connection<InTransaction> {
    pub fn query(mut self, sql: &str) -> Self {
        self.statement_log.push(sql.to_string());
        self
    }
    pub fn commit(self) -> Connection<Connected> {
        println!("COMMIT {} câu lệnh", self.statement_log.len());
        Connection { connection_string: self.connection_string, statement_log: Vec::new(), _tt: PhantomData }
    }
    pub fn rollback(self) -> Connection<Connected> {
        println!("ROLLBACK, hủy {} câu lệnh", self.statement_log.len());
        Connection { connection_string: self.connection_string, statement_log: Vec::new(), _tt: PhantomData }
    }
}

fn main() {
    let conn = Connection::new("postgres://localhost/shop").connect().unwrap();
    let conn = conn.begin_transaction()
        .query("UPDATE inventory SET quantity = quantity - 1 WHERE id = 7")
        .query("INSERT INTO orders VALUES (7, 1)")
        .commit();
    let _ = conn.begin_transaction().query("DELETE FROM temp").rollback();

    // Các dòng sau KHÔNG biên dịch được — và đó chính là mục đích:
    // Connection::new("...").query("SELECT 1");  // E0599: chưa kết nối
    // conn.commit();                            // E0599: không ở trong giao dịch
}
```

Lưu ý điểm tinh tế: `query` nhận `mut self` và trả về `Self`, cho phép xâu chuỗi phương thức mà vẫn giữ nguyên tắc "mỗi thao tác tiêu thụ giá trị cũ". Đây là mẫu *builder* kết hợp typestate — rất phổ biến trong các thư viện Rust chất lượng cao.
</details>
