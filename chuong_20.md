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
fn gui_thu(dia_chi: String) { }
// Người gọi có thể truyền vào "", "abc", "  " — hàm phải tự kiểm tra lại.

// ✅ Kiểu có bằng chứng: chữ ký là một hợp đồng
fn gui_thu(dia_chi: Email) { }
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
│   │  Email::phan_tich  │               │      │ ← chỉ CỬA RA MÁY BAY nhận        │
│   │  - có @ không?     │               │      ▼                                  │
│   │  - có tên miền?    │               │  [Đã lên máy bay]                       │
│   └─────────┬──────────┘               │                                         │
│      ┌──────┴───────┐                  │  KHÔNG AI cầm "vé đã đặt" mà bước       │
│      ▼              ▼                  │  thẳng vào cửa ra máy bay được —        │
│  [TỪ CHỐI]   ┌──────────────┐          │  vì tờ giấy trên tay SAI LOẠI.          │
│   Err(...)   │ Email  ĐÃ    │          │                                         │
│              │ ĐÓNG DẤU ĐỎ  │          │  Trong Rust: DonHang<Nhap> và           │
│              └──────────────┘          │  DonHang<DaThanhToan> là HAI KIỂU       │
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

Đó chính xác là Typestate: `DonHang<Nhap>` và `DonHang<DaThanhToan>` là **hai kiểu khác nhau**, nên hàm `giao_hang` chỉ nhận loại thứ hai. Việc "quên thanh toán" không còn là một lỗi lúc chạy — nó là **lỗi biên dịch**.

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
struct DonHangXau {
    da_thanh_toan: bool,
    ma_giao_dich: Option<String>,
}
// Tổ hợp 1: da_thanh_toan = true,  ma_giao_dich = None      → Đã trả tiền mà không có mã?!
// Tổ hợp 2: da_thanh_toan = false, ma_giao_dich = Some(...) → Chưa trả mà có mã giao dịch?!
// Hệ quả: mọi hàm đọc struct này phải viết `if` phòng thủ cho hai trường hợp không thể xảy ra.

// ✅ Kiểu TỔNG: 1 + n = KHÔNG CÒN tổ hợp vô nghĩa nào
enum TrangThaiThanhToan {
    ChuaTra,
    DaTra { ma_giao_dich: String },
}
// Trình biên dịch bảo đảm: có mã giao dịch ⟺ đã trả tiền. Không cần `if` phòng thủ nào cả.
```

### 2. Kiểu bọc (Newtype) + Hàm khởi tạo có kiểm chứng (Smart Constructor)

Ba thành phần bắt buộc, thiếu một là hỏng cả:

```rust
mod mien {
    // (1) Kiểu bọc với trường RIÊNG TƯ — không có `pub` trước `String`
    #[derive(Debug, Clone, PartialEq, Eq)]
    pub struct Email(String);

    impl Email {
        // (2) Cửa duy nhất để tạo ra giá trị: hàm khởi tạo có kiểm chứng
        pub fn phan_tich(tho: &str) -> Result<Self, String> {
            let s = tho.trim().to_lowercase();
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
| Chữ ký | `fn kiem_tra(s: &str) -> bool` | `fn phan_tich(s: &str) -> Result<Email, Loi>` |
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
fn giao_hang(don: &mut DonHang) -> Result<(), Loi> {
    if don.trang_thai != TrangThai::DaThanhToan {
        return Err(Loi::ChuaThanhToan);  // ← lỗi này chỉ lộ ra khi chạy tới
    }
    Ok(())
}
```

Typestate đẩy phép kiểm tra đó lên **lúc biên dịch**, bằng cách gắn trạng thái vào *tham số kiểu*:

```rust
use std::marker::PhantomData;

pub struct Nhap;          // Các kiểu "thẻ đánh dấu" — không chứa dữ liệu,
pub struct DaXacThuc;     // chiếm 0 byte bộ nhớ, chỉ tồn tại lúc biên dịch.
pub struct DaThanhToan;

pub struct DonHang<TT> {
    ma: String,
    dong: Vec<DongHang>,
    _trang_thai: PhantomData<TT>,   // "tôi mang thẻ TT" — 0 byte
}

impl DonHang<Nhap> {
    pub fn xac_thuc(self) -> Result<DonHang<DaXacThuc>, LoiMien> { /* ... */ }
}
impl DonHang<DaThanhToan> {
    pub fn giao_hang(self) -> PhieuGiaoHang { /* ... */ }   // ← chỉ tồn tại cho trạng thái này!
}
```

Bây giờ đoạn mã sau **không biên dịch được**:

```rust
let don = DonHang::moi(/* ... */);   // DonHang<Nhap>
don.giao_hang();
// LỖI E0599: no method named `giao_hang` found for struct `DonHang<Nhap>`
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
| Ví dụ | `struct DonHangDto { email: String }` | `struct DonHang { khach: Email }` |

Cầu nối giữa hai thế giới chính là trait **`TryFrom`**:

```rust
impl TryFrom<DonHangDto> for DonHang<Nhap> {
    type Error = Vec<LoiMien>;   // trả về TẤT CẢ lỗi — Applicative ở Chương 19!
    fn try_from(dto: DonHangDto) -> Result<Self, Self::Error> { /* ... */ }
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
pub mod mien {
    use std::fmt;

    #[derive(Debug, Clone, PartialEq, Eq)]
    pub enum LoiMien {
        EmailSai(String),
        TenSanPhamSai(String),
        SoLuongSai(String),
        DonRong,
        DonQuaLon { so_dong: usize, toi_da: usize },
    }

    impl fmt::Display for LoiMien {
        fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
            match self {
                LoiMien::EmailSai(s) => write!(f, "Email không hợp lệ: {}", s),
                LoiMien::TenSanPhamSai(s) => write!(f, "Tên sản phẩm không hợp lệ: {}", s),
                LoiMien::SoLuongSai(s) => write!(f, "Số lượng không hợp lệ: {}", s),
                LoiMien::DonRong => write!(f, "Đơn hàng phải có ít nhất 1 dòng hàng"),
                LoiMien::DonQuaLon { so_dong, toi_da } => {
                    write!(f, "Đơn có {} dòng, vượt giới hạn {} dòng", so_dong, toi_da)
                }
            }
        }
    }

    // ---------------------------------------------------------------------
    // KIỂU BỌC 1: Email — trường riêng tư, chỉ tạo được qua `phan_tich`
    // ---------------------------------------------------------------------
    #[derive(Debug, Clone, PartialEq, Eq)]
    pub struct Email(String); // KHÔNG có `pub` trước String → đây là con dấu

    impl Email {
        pub fn phan_tich(tho: &str) -> Result<Self, LoiMien> {
            let s = tho.trim().to_lowercase();
            if s.is_empty() {
                return Err(LoiMien::EmailSai("chuỗi rỗng".to_string()));
            }
            let phan: Vec<&str> = s.split('@').collect();
            if phan.len() != 2 || phan[0].is_empty() || !phan[1].contains('.') {
                return Err(LoiMien::EmailSai(s));
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
    // KIỂU BỌC 2: TenSanPham — chuỗi có giới hạn độ dài
    // ---------------------------------------------------------------------
    #[derive(Debug, Clone, PartialEq, Eq)]
    pub struct TenSanPham(String);

    impl TenSanPham {
        pub const TOI_DA: usize = 50;

        pub fn phan_tich(tho: &str) -> Result<Self, LoiMien> {
            let s = tho.trim();
            let so_ky_tu = s.chars().count(); // đếm CHỮ CÁI, không đếm byte (Chương 05)
            if so_ky_tu == 0 {
                Err(LoiMien::TenSanPhamSai("chuỗi rỗng".to_string()))
            } else if so_ky_tu > Self::TOI_DA {
                Err(LoiMien::TenSanPhamSai(format!(
                    "dài {} ký tự, tối đa {}",
                    so_ky_tu,
                    Self::TOI_DA
                )))
            } else {
                Ok(TenSanPham(s.to_string()))
            }
        }
        pub fn as_str(&self) -> &str {
            &self.0
        }
    }

    // ---------------------------------------------------------------------
    // KIỂU BỌC 3: SoLuong — số nguyên dương trong khoảng cho phép
    // ---------------------------------------------------------------------
    #[derive(Debug, Clone, Copy, PartialEq, Eq)]
    pub struct SoLuong(u32);

    impl SoLuong {
        pub const TOI_DA: u32 = 1000;

        pub fn phan_tich(n: u32) -> Result<Self, LoiMien> {
            if n == 0 {
                Err(LoiMien::SoLuongSai("phải lớn hơn 0".to_string()))
            } else if n > Self::TOI_DA {
                Err(LoiMien::SoLuongSai(format!("{} vượt quá {}", n, Self::TOI_DA)))
            } else {
                Ok(SoLuong(n))
            }
        }
        pub fn gia_tri(&self) -> u32 {
            self.0
        }
    }

    // ---------------------------------------------------------------------
    // KIỂU BỌC 4: SoTien — tính bằng ĐƠN VỊ NHỎ NHẤT (đồng), dùng u64.
    // KHÔNG BAO GIỜ dùng f64 cho tiền tệ (xem cảnh báo ở Chương 03)!
    // ---------------------------------------------------------------------
    #[derive(Debug, Clone, Copy, PartialEq, Eq, PartialOrd, Ord)]
    pub struct SoTien(u64);

    impl SoTien {
        pub fn dong(n: u64) -> Self {
            SoTien(n)
        }
        pub fn gia_tri(&self) -> u64 {
            self.0
        }
        pub fn cong(self, khac: SoTien) -> SoTien {
            SoTien(self.0 + khac.0) // đây là một VỊ NHÓM (Chương 18)!
        }
        pub fn tru(self, khac: SoTien) -> SoTien {
            SoTien(self.0.saturating_sub(khac.0))
        }
        pub fn nhan(self, he_so: u32) -> SoTien {
            SoTien(self.0 * he_so as u64)
        }
    }

    impl fmt::Display for SoTien {
        fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
            write!(f, "{} đ", self.0)
        }
    }

    // ---------------------------------------------------------------------
    // KIỂU TỔNG: cách thanh toán — KHÔNG CÒN tổ hợp vô nghĩa
    // ---------------------------------------------------------------------
    #[derive(Debug, Clone, PartialEq, Eq)]
    pub enum ThanhToan {
        TienMat,
        ChuyenKhoan { ma_giao_dich: String },
        The { bon_so_cuoi: String },
    }

    // ---------------------------------------------------------------------
    // Dòng hàng: một kiểu TÍCH gồm toàn kiểu đã được công chứng
    // ---------------------------------------------------------------------
    #[derive(Debug, Clone, PartialEq, Eq)]
    pub struct DongHang {
        pub ten: TenSanPham,
        pub so_luong: SoLuong,
        pub don_gia: SoTien,
    }

    impl DongHang {
        pub fn thanh_tien(&self) -> SoTien {
            self.don_gia.nhan(self.so_luong.gia_tri())
        }
    }
}

use mien::*;

// ============================================================================
// PHẦN 2: TYPESTATE — MÁY TRẠNG THÁI ĐƯỢC MÃ HÓA VÀO KIỂU
// ============================================================================

/// Bốn "thẻ đánh dấu" trạng thái. Chúng chiếm 0 byte và biến mất khi biên dịch.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct Nhap;
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct DaXacThuc;
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct DaThanhToan;
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct DaGiao;

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct DonHang<TT> {
    ma: String,
    khach: Email,
    dong: Vec<DongHang>,
    thanh_toan: Option<ThanhToan>,
    _trang_thai: PhantomData<TT>,
}

/// Các phương thức dùng chung cho MỌI trạng thái.
impl<TT> DonHang<TT> {
    pub fn ma(&self) -> &str {
        &self.ma
    }
    pub fn khach(&self) -> &Email {
        &self.khach
    }
    pub fn so_dong(&self) -> usize {
        self.dong.len()
    }
    /// Tổng tiền = gộp các thành tiền bằng phép cộng của vị nhóm SoTien.
    pub fn tong_tien(&self) -> SoTien {
        self.dong
            .iter()
            .map(|d| d.thanh_tien())
            .fold(SoTien::dong(0), |a, b| a.cong(b))
    }
}

pub const SO_DONG_TOI_DA: usize = 20;

/// Trạng thái NHẬP: chỉ có đúng một hành động hợp lệ — xác thực.
impl DonHang<Nhap> {
    pub fn moi(ma: &str, khach: Email, dong: Vec<DongHang>) -> Self {
        DonHang {
            ma: ma.to_string(),
            khach,
            dong,
            thanh_toan: None,
            _trang_thai: PhantomData,
        }
    }

    pub fn xac_thuc(self) -> Result<DonHang<DaXacThuc>, LoiMien> {
        if self.dong.is_empty() {
            return Err(LoiMien::DonRong);
        }
        if self.dong.len() > SO_DONG_TOI_DA {
            return Err(LoiMien::DonQuaLon {
                so_dong: self.dong.len(),
                toi_da: SO_DONG_TOI_DA,
            });
        }
        Ok(DonHang {
            ma: self.ma,
            khach: self.khach,
            dong: self.dong,
            thanh_toan: None,
            _trang_thai: PhantomData,
        })
    }
}

/// Trạng thái ĐÃ XÁC THỰC: chỉ có thể thanh toán.
impl DonHang<DaXacThuc> {
    pub fn thanh_toan(self, cach: ThanhToan) -> DonHang<DaThanhToan> {
        DonHang {
            ma: self.ma,
            khach: self.khach,
            dong: self.dong,
            thanh_toan: Some(cach),
            _trang_thai: PhantomData,
        }
    }
}

/// Trạng thái ĐÃ THANH TOÁN: chỉ có thể giao hàng.
impl DonHang<DaThanhToan> {
    pub fn cach_thanh_toan(&self) -> &ThanhToan {
        // An toàn tuyệt đối: chỉ trạng thái này mới tồn tại, và nó LUÔN có thanh toán.
        self.thanh_toan
            .as_ref()
            .expect("bất biến của DonHang<DaThanhToan>: luôn có thông tin thanh toán")
    }

    pub fn giao_hang(self, ma_van_don: &str) -> DonHang<DaGiao> {
        println!(
            "   [VỎ MỆNH LỆNH] Gửi email tới {} về vận đơn {}",
            self.khach, ma_van_don
        );
        DonHang {
            ma: self.ma,
            khach: self.khach,
            dong: self.dong,
            thanh_toan: self.thanh_toan,
            _trang_thai: PhantomData,
        }
    }
}

// ============================================================================
// PHẦN 3: BIÊN HỆ THỐNG — DTO VÀ CỔNG CÔNG CHỨNG `TryFrom`
// ============================================================================

/// Kiểu TRUYỀN TẢI: khoan dung, phẳng, toàn chuỗi — đúng như JSON gửi tới.
#[derive(Debug, Clone)]
pub struct DonHangDto {
    pub ma: String,
    pub email: String,
    pub dong: Vec<DongHangDto>,
}

#[derive(Debug, Clone)]
pub struct DongHangDto {
    pub ten: String,
    pub so_luong: u32,
    pub don_gia: u64,
}

impl TryFrom<DonHangDto> for DonHang<Nhap> {
    /// Trả về TẤT CẢ lỗi cùng lúc — đúng tinh thần Applicative ở Chương 19.
    type Error = Vec<LoiMien>;

    fn try_from(dto: DonHangDto) -> Result<Self, Self::Error> {
        let mut loi: Vec<LoiMien> = Vec::new();

        let khach = match Email::phan_tich(&dto.email) {
            Ok(e) => Some(e),
            Err(e) => {
                loi.push(e);
                None
            }
        };

        let mut dong: Vec<DongHang> = Vec::new();
        for d in &dto.dong {
            let ten = TenSanPham::phan_tich(&d.ten);
            let sl = SoLuong::phan_tich(d.so_luong);
            match (ten, sl) {
                (Ok(t), Ok(s)) => dong.push(DongHang {
                    ten: t,
                    so_luong: s,
                    don_gia: SoTien::dong(d.don_gia),
                }),
                (t, s) => {
                    if let Err(e) = t {
                        loi.push(e);
                    }
                    if let Err(e) = s {
                        loi.push(e);
                    }
                }
            }
        }

        match khach {
            Some(k) if loi.is_empty() => Ok(DonHang::moi(&dto.ma, k, dong)),
            _ => Err(loi),
        }
    }
}

// ============================================================================
// PHẦN 4: LÕI THUẦN TÚY — QUY TẮC NGHIỆP VỤ, KHÔNG CÓ MỘT DÒNG I/O NÀO
// ============================================================================

/// Tính phí vận chuyển theo tổng tiền. Hàm thuần túy 100%: dễ kiểm thử tuyệt đối.
pub fn tinh_phi_van_chuyen(tong: SoTien) -> SoTien {
    if tong.gia_tri() >= 500_000 {
        SoTien::dong(0) // miễn phí cho đơn từ 500k
    } else {
        SoTien::dong(30_000)
    }
}

/// Tính chiết khấu theo số dòng hàng. Cũng thuần túy 100%.
pub fn tinh_chiet_khau(tong: SoTien, so_dong: usize) -> SoTien {
    let phan_tram = if so_dong >= 10 {
        10
    } else if so_dong >= 5 {
        5
    } else {
        0
    };
    SoTien::dong(tong.gia_tri() * phan_tram / 100)
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct HoaDon {
    pub tam_tinh: SoTien,
    pub chiet_khau: SoTien,
    pub phi_van_chuyen: SoTien,
    pub tong_thanh_toan: SoTien,
}

/// Toàn bộ phép tính hóa đơn — vẫn hoàn toàn thuần túy.
pub fn lap_hoa_don(don: &DonHang<DaXacThuc>) -> HoaDon {
    let tam_tinh = don.tong_tien();
    let chiet_khau = tinh_chiet_khau(tam_tinh, don.so_dong());
    let sau_chiet_khau = tam_tinh.tru(chiet_khau);
    let phi = tinh_phi_van_chuyen(sau_chiet_khau);
    HoaDon {
        tam_tinh,
        chiet_khau,
        phi_van_chuyen: phi,
        tong_thanh_toan: sau_chiet_khau.cong(phi),
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
    for tho in ["  An.Nguyen@Example.COM ", "khong-co-a-cong", "@thieu-ten.vn", ""] {
        match Email::phan_tich(tho) {
            Ok(e) => println!("   {:>28} -> ✓ đóng dấu: {}", format!("{:?}", tho), e),
            Err(l) => println!("   {:>28} -> ✗ từ chối: {}", format!("{:?}", tho), l),
        }
    }
    println!("   → Không có cách nào tạo ra một `Email` sai. Trường bên trong là riêng tư.");

    // ------------------------------------------------------------------
    // 2. ĐẠI SỐ CỦA KIỂU — ĐẾM SỐ TRẠNG THÁI
    // ------------------------------------------------------------------
    println!("\n2. ĐẠI SỐ CỦA KIỂU");
    println!("   struct (bool, bool)        -> kiểu TÍCH: 2 × 2 = 4 trạng thái");
    println!("   enum {{ TienMat, CK, The }}  -> kiểu TỔNG: 1 + 1 + 1 = 3 trạng thái");
    println!("   Cách SAI : struct {{ da_tra: bool, ma_gd: Option<String> }}");
    println!("              -> có 2 tổ hợp VÔ NGHĨA (đã trả mà không mã / chưa trả mà có mã)");
    println!("   Cách ĐÚNG: enum {{ ChuaTra, DaTra {{ ma_gd }} }} -> 0 tổ hợp vô nghĩa ✓");

    // ------------------------------------------------------------------
    // 3. CỔNG BIÊN HỆ THỐNG: DTO -> KIỂU MIỀN, GOM HẾT LỖI
    // ------------------------------------------------------------------
    println!("\n3. CỔNG BIÊN HỆ THỐNG (DTO -> Miền), gom TẤT CẢ lỗi");
    let dto_hong = DonHangDto {
        ma: "ORD-0001".to_string(),
        email: "sai-email".to_string(),
        dong: vec![
            DongHangDto { ten: "".to_string(), so_luong: 0, don_gia: 100 },
            DongHangDto { ten: "Bàn phím cơ".to_string(), so_luong: 2, don_gia: 1_200_000 },
        ],
    };
    match DonHang::try_from(dto_hong) {
        Ok(_) => println!("   (không tới đây)"),
        Err(loi) => {
            println!("   Từ chối đơn hàng với {} lỗi:", loi.len());
            for (i, l) in loi.iter().enumerate() {
                println!("     {}. {}", i + 1, l);
            }
        }
    }

    // ------------------------------------------------------------------
    // 4. ĐƠN HỢP LỆ ĐI QUA TOÀN BỘ MÁY TRẠNG THÁI
    // ------------------------------------------------------------------
    println!("\n4. TYPESTATE — QUY TRÌNH ĐƠN HÀNG");
    let dto_tot = DonHangDto {
        ma: "ORD-0002".to_string(),
        email: "  Khach.Hang@Shop.VN  ".to_string(),
        dong: vec![
            DongHangDto { ten: "Bàn phím cơ không dây".to_string(), so_luong: 2, don_gia: 1_200_000 },
            DongHangDto { ten: "Chuột công thái học".to_string(), so_luong: 1, don_gia: 750_000 },
            DongHangDto { ten: "Lót chuột cỡ lớn".to_string(), so_luong: 3, don_gia: 150_000 },
        ],
    };

    let don_nhap: DonHang<Nhap> = DonHang::try_from(dto_tot).expect("đơn này phải hợp lệ");
    println!(
        "   [Nhập]          mã={} khách={} số dòng={}",
        don_nhap.ma(),
        don_nhap.khach(),
        don_nhap.so_dong()
    );

    let don_xac_thuc: DonHang<DaXacThuc> = don_nhap.xac_thuc().expect("đơn có 3 dòng, hợp lệ");
    println!("   [Đã xác thực]   tổng hàng = {}", don_xac_thuc.tong_tien());

    // ---- LÕI THUẦN TÚY: lập hóa đơn (không I/O, kiểm thử được ngay) ----
    let hoa_don = lap_hoa_don(&don_xac_thuc);
    println!("   ┌─ HÓA ĐƠN (tính bởi LÕI THUẦN TÚY) ─────────────");
    println!("   │ Tạm tính        : {}", hoa_don.tam_tinh);
    println!("   │ Chiết khấu      : {}", hoa_don.chiet_khau);
    println!("   │ Phí vận chuyển  : {}", hoa_don.phi_van_chuyen);
    println!("   │ TỔNG THANH TOÁN : {}", hoa_don.tong_thanh_toan);
    println!("   └────────────────────────────────────────────────");

    let don_da_tra: DonHang<DaThanhToan> = don_xac_thuc.thanh_toan(ThanhToan::ChuyenKhoan {
        ma_giao_dich: "VCB-99881234".to_string(),
    });
    println!("   [Đã thanh toán] cách trả = {:?}", don_da_tra.cach_thanh_toan());

    let _don_da_giao: DonHang<DaGiao> = don_da_tra.giao_hang("VN-EXP-77213");
    println!("   [Đã giao]       hoàn tất quy trình ✓");

    // ------------------------------------------------------------------
    // 5. NHỮNG GÌ TRÌNH BIÊN DỊCH TỪ CHỐI
    // ------------------------------------------------------------------
    println!("\n5. TRÌNH BIÊN DỊCH LÀ NHÂN VIÊN SOÁT VÉ KHÔNG BAO GIỜ NGỦ GẬT");
    println!("   Các dòng sau KHÔNG BIÊN DỊCH ĐƯỢC (đã đóng chú thích trong mã nguồn):");
    println!("     · don_nhap.giao_hang(...)  -> E0599: DonHang<Nhap> không có `giao_hang`");
    println!("     · mien::Email(\"rác\".into()) -> E0603: trường riêng tư, không dựng được");
    println!("     · don_xac_thuc.tong_tien() -> E0382: đơn đã bị `thanh_toan` tiêu thụ");
    println!("   → Ba lớp lỗi nghiệp vụ bị xóa sổ TRƯỚC khi chương trình kịp chạy.");

    // ------------------------------------------------------------------
    // 6. ĐƠN VI PHẠM QUY TẮC NGHIỆP VỤ
    // ------------------------------------------------------------------
    println!("\n6. XÁC THỰC QUY TẮC NGHIỆP VỤ");
    let email = Email::phan_tich("test@shop.vn").unwrap();
    let don_rong: DonHang<Nhap> = DonHang::moi("ORD-0003", email, vec![]);
    match don_rong.xac_thuc() {
        Ok(_) => println!("   (không tới đây)"),
        Err(l) => println!("   Đơn rỗng bị chặn: {}", l),
    }

    println!("\n============================================================");
    println!("  TRẠNG THÁI SAI KHÔNG BIỂU DIỄN ĐƯỢC = LỖI KHÔNG XẢY RA    ");
    println!("============================================================");
}

// ============================================================================
// KIỂM THỬ: LÕI THUẦN TÚY KIỂM THỬ ĐƯỢC MÀ KHÔNG CẦN CSDL, MẠNG HAY MOCK
// ============================================================================

#[cfg(test)]
mod kiem_thu {
    use super::*;

    fn don_mau() -> DonHang<DaXacThuc> {
        let email = Email::phan_tich("khach@shop.vn").unwrap();
        let dong = vec![
            DongHang {
                ten: TenSanPham::phan_tich("Bàn phím").unwrap(),
                so_luong: SoLuong::phan_tich(2).unwrap(),
                don_gia: SoTien::dong(100_000),
            },
            DongHang {
                ten: TenSanPham::phan_tich("Chuột").unwrap(),
                so_luong: SoLuong::phan_tich(1).unwrap(),
                don_gia: SoTien::dong(50_000),
            },
        ];
        DonHang::moi("ORD-TEST", email, dong).xac_thuc().unwrap()
    }

    #[test]
    fn email_chap_nhan_dia_chi_hop_le() {
        let e = Email::phan_tich("  An.Nguyen@Example.COM ").unwrap();
        assert_eq!(e.as_str(), "an.nguyen@example.com"); // đã chuẩn hóa
    }

    #[test]
    fn email_tu_choi_dia_chi_sai() {
        for xau in ["", "   ", "khong-co-a-cong", "@thieu-ten.vn", "a@b@c.vn", "a@khongcocham"] {
            assert!(Email::phan_tich(xau).is_err(), "phải từ chối {:?}", xau);
        }
    }

    #[test]
    fn so_luong_phai_duong_va_trong_gioi_han() {
        assert!(SoLuong::phan_tich(0).is_err());
        assert!(SoLuong::phan_tich(1001).is_err());
        assert_eq!(SoLuong::phan_tich(5).unwrap().gia_tri(), 5);
    }

    #[test]
    fn ten_san_pham_dem_ky_tu_khong_dem_byte() {
        // 50 chữ cái tiếng Việt có dấu = nhiều hơn 50 BYTE, nhưng vẫn hợp lệ.
        let ten_dai: String = "ế".repeat(50);
        assert!(TenSanPham::phan_tich(&ten_dai).is_ok());
        let qua_dai: String = "ế".repeat(51);
        assert!(TenSanPham::phan_tich(&qua_dai).is_err());
    }

    #[test]
    fn don_rong_bi_tu_choi() {
        let email = Email::phan_tich("a@b.vn").unwrap();
        let don = DonHang::moi("X", email, vec![]);
        assert_eq!(don.xac_thuc().unwrap_err(), LoiMien::DonRong);
    }

    #[test]
    fn dto_gom_tat_ca_loi_cung_luc() {
        let dto = DonHangDto {
            ma: "X".to_string(),
            email: "sai".to_string(),
            dong: vec![DongHangDto { ten: "".to_string(), so_luong: 0, don_gia: 1 }],
        };
        let loi = DonHang::try_from(dto).unwrap_err();
        assert_eq!(loi.len(), 3, "phải gom đủ 3 lỗi, nhận được {:?}", loi);
    }

    // ---- Kiểm thử LÕI THUẦN TÚY: không cần CSDL, không cần mạng ----

    #[test]
    fn tong_tien_cong_dung_thanh_tien_tung_dong() {
        let don = don_mau();
        // 2 × 100.000 + 1 × 50.000 = 250.000
        assert_eq!(don.tong_tien(), SoTien::dong(250_000));
    }

    #[test]
    fn phi_van_chuyen_mien_phi_tu_500k() {
        assert_eq!(tinh_phi_van_chuyen(SoTien::dong(499_999)), SoTien::dong(30_000));
        assert_eq!(tinh_phi_van_chuyen(SoTien::dong(500_000)), SoTien::dong(0));
    }

    #[test]
    fn chiet_khau_theo_bac_so_dong() {
        let tong = SoTien::dong(1_000_000);
        assert_eq!(tinh_chiet_khau(tong, 3), SoTien::dong(0));
        assert_eq!(tinh_chiet_khau(tong, 5), SoTien::dong(50_000));
        assert_eq!(tinh_chiet_khau(tong, 12), SoTien::dong(100_000));
    }

    #[test]
    fn hoa_don_tinh_dung_toan_bo() {
        let don = don_mau(); // tạm tính 250.000, 2 dòng -> không chiết khấu
        let hd = lap_hoa_don(&don);
        assert_eq!(hd.tam_tinh, SoTien::dong(250_000));
        assert_eq!(hd.chiet_khau, SoTien::dong(0));
        assert_eq!(hd.phi_van_chuyen, SoTien::dong(30_000));
        assert_eq!(hd.tong_thanh_toan, SoTien::dong(280_000));
    }

    #[test]
    fn quy_trinh_typestate_chay_het_bon_buoc() {
        let don = don_mau();
        let da_tra = don.thanh_toan(ThanhToan::TienMat);
        assert_eq!(da_tra.cach_thanh_toan(), &ThanhToan::TienMat);
        let da_giao = da_tra.giao_hang("VD-001");
        assert_eq!(da_giao.ma(), "ORD-TEST");
    }

    #[test]
    fn typestate_khong_ton_bo_nho_luc_chay() {
        use std::mem::size_of;
        // PhantomData chiếm 0 byte: DonHang<Nhap> và DonHang<DaGiao> có cùng kích thước.
        assert_eq!(size_of::<DonHang<Nhap>>(), size_of::<DonHang<DaGiao>>());
        assert_eq!(size_of::<Nhap>(), 0);
        assert_eq!(size_of::<PhantomData<DaGiao>>(), 0);
    }
}
```

---

## Bảng tra cứu lỗi biên dịch & Cách khắc phục (Compiler Error Guide)

| Mã lỗi | Thông báo mẫu từ trình biên dịch | Nguyên nhân cốt lõi | Cách khắc phục nhanh |
|---|---|---|---|
| **E0603** | `tuple struct constructor 'Email' is private` | **Đây là lỗi TỐT!** Nó chứng minh kiểu bọc đang bảo vệ bạn: ai đó cố tạo `Email` mà không đi qua phòng công chứng. | Gọi `Email::phan_tich(...)` thay vì `Email(...)`. Đừng bao giờ "sửa" bằng cách thêm `pub` vào trường. |
| **E0599** | `no method named 'giao_hang' found for struct 'DonHang<Nhap>'` | **Cũng là lỗi TỐT!** Typestate đang chặn một bước nhảy cóc trong quy trình. | Đi đúng thứ tự: `.xac_thuc()?` rồi `.thanh_toan(...)` rồi mới `.giao_hang(...)`. |
| **E0382** | `use of moved value: 'don'` | Mỗi phép chuyển trạng thái **tiêu thụ** `self`, nên đơn hàng ở trạng thái cũ không còn tồn tại. | Đó là chủ ý thiết kế. Dùng biến mới cho mỗi trạng thái, hoặc `#[derive(Clone)]` nếu thật sự cần bản sao. |
| **E0392** | `parameter 'TT' is never used` | Bạn khai báo `struct DonHang<TT>` mà không dùng `TT` trong bất kỳ trường nào. | Thêm trường `_trang_thai: PhantomData<TT>` — đây chính là lý do `PhantomData` tồn tại. |
| **E0277** | `the trait bound 'DonHang<Nhap>: Debug' is not satisfied` | `#[derive(Debug)]` trên kiểu generic sinh ra ràng buộc `TT: Debug`. | Thêm `#[derive(Debug)]` cho các kiểu thẻ đánh dấu (`Nhap`, `DaGiao`…), hoặc tự viết `impl Debug`. |

### Phân tích lỗi thực tế `E0603` — khi lỗi biên dịch là dấu hiệu thành công:

```rust
// ❌ Đoạn mã lỗi (đã đóng chú thích để tệp vẫn biên dịch được):
// let e = mien::Email("đây không phải email".to_string());
// LỖI E0603: tuple struct constructor `Email` is private
//
// Đây KHÔNG phải sự cố cần khắc phục — đây là bằng chứng thiết kế đang hoạt động!
// Nếu đoạn mã trên biên dịch được, mọi bảo đảm của kiểu `Email` đều vô nghĩa.

// ✅ Cách duy nhất được phép:
// let e = mien::Email::phan_tich("kh@shop.vn").expect("địa chỉ hợp lệ");
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

**Bài tập 1 (Kiểu bọc `SoDienThoaiVN`)**
Viết kiểu bọc `SoDienThoaiVN` với hàm khởi tạo có kiểm chứng: chấp nhận chuỗi chỉ gồm chữ số (cho phép có khoảng trắng và dấu chấm ở giữa), độ dài sau khi làm sạch là 10 chữ số và bắt đầu bằng `0`. Chuẩn hóa kết quả về dạng không dấu cách. Viết ít nhất 4 bài kiểm thử.

<details>
<summary><b>Gợi ý</b></summary>

Dùng `chars().filter(|c| c.is_ascii_digit()).collect::<String>()` để làm sạch. Nhớ đặt kiểu trong một `mod` và **không** đánh `pub` cho trường bên trong.
</details>

<details>
<summary><b>Lời giải</b></summary>

```rust
pub mod lien_lac {
    #[derive(Debug, Clone, PartialEq, Eq)]
    pub struct SoDienThoaiVN(String);

    impl SoDienThoaiVN {
        pub fn phan_tich(tho: &str) -> Result<Self, String> {
            let sach: String = tho.chars().filter(|c| c.is_ascii_digit()).collect();
            if sach.len() != 10 {
                return Err(format!("Cần đúng 10 chữ số, nhận được {}", sach.len()));
            }
            if !sach.starts_with('0') {
                return Err("Số điện thoại phải bắt đầu bằng 0".to_string());
            }
            Ok(SoDienThoaiVN(sach))
        }
        pub fn as_str(&self) -> &str { &self.0 }
    }
}

#[cfg(test)]
mod t {
    use super::lien_lac::SoDienThoaiVN as SDT;

    #[test] fn chap_nhan_so_hop_le() {
        assert_eq!(SDT::phan_tich("0912 345 678").unwrap().as_str(), "0912345678");
    }
    #[test] fn chap_nhan_dinh_dang_co_dau_cham() {
        assert_eq!(SDT::phan_tich("098.765.4321").unwrap().as_str(), "0987654321");
    }
    #[test] fn tu_choi_thieu_chu_so() { assert!(SDT::phan_tich("0912345").is_err()); }
    #[test] fn tu_choi_khong_bat_dau_bang_0() { assert!(SDT::phan_tich("1912345678").is_err()); }
}
```
</details>

**Bài tập 2 (Xóa trạng thái vô nghĩa)**
Cho struct sau đây, hãy đếm số trạng thái nó biểu diễn được, chỉ ra những tổ hợp vô nghĩa, rồi thiết kế lại bằng `enum` sao cho **không còn tổ hợp vô nghĩa nào**:

```rust
struct TaiKhoan {
    da_kich_hoat: bool,
    ngay_kich_hoat: Option<String>,
    ly_do_khoa: Option<String>,
}
```

<details>
<summary><b>Gợi ý</b></summary>

Liệt kê các trạng thái nghiệp vụ *thật sự* tồn tại của một tài khoản: chờ kích hoạt, đang hoạt động, bị khóa. Mỗi trạng thái cần mang theo *đúng* dữ liệu nào?
</details>

<details>
<summary><b>Lời giải</b></summary>

**Đếm trạng thái**: `2 × (1 + n) × (1 + m)` — với `n`, `m` là số chuỗi có thể. Ngay cả khi rút gọn `Option` thành "có/không", ta đã có `2 × 2 × 2 = 8` tổ hợp, trong khi nghiệp vụ chỉ có **3** trạng thái thật. Năm tổ hợp vô nghĩa, ví dụ:
- `da_kich_hoat = true` nhưng `ngay_kich_hoat = None` → hoạt động mà không rõ từ bao giờ?
- `da_kich_hoat = true` và `ly_do_khoa = Some(...)` → vừa hoạt động vừa bị khóa?
- `da_kich_hoat = false`, `ngay_kich_hoat = Some(...)` → đã kích hoạt rồi mà lại chưa kích hoạt?

**Thiết kế lại**:

```rust
#[derive(Debug, Clone, PartialEq, Eq)]
pub enum TaiKhoan {
    ChoKichHoat,
    DangHoatDong { ngay_kich_hoat: String },
    BiKhoa { ngay_kich_hoat: String, ly_do: String },
}

impl TaiKhoan {
    pub fn kich_hoat(self, ngay: String) -> Result<Self, &'static str> {
        match self {
            TaiKhoan::ChoKichHoat => Ok(TaiKhoan::DangHoatDong { ngay_kich_hoat: ngay }),
            _ => Err("Tài khoản đã được kích hoạt trước đó"),
        }
    }
    pub fn khoa(self, ly_do: String) -> Result<Self, &'static str> {
        match self {
            TaiKhoan::DangHoatDong { ngay_kich_hoat } =>
                Ok(TaiKhoan::BiKhoa { ngay_kich_hoat, ly_do }),
            _ => Err("Chỉ khóa được tài khoản đang hoạt động"),
        }
    }
}
```

Đúng 3 trạng thái, 0 tổ hợp vô nghĩa. Và nhờ `match` vét cạn, khi bạn thêm trạng thái thứ tư sau này, trình biên dịch sẽ chỉ ra **chính xác** mọi nơi cần cập nhật.
</details>

**Bài tập 3 (Typestate cho kết nối cơ sở dữ liệu)**
Thiết kế typestate cho một kết nối cơ sở dữ liệu với ba trạng thái: `ChuaKetNoi` → `DaKetNoi` → `TrongGiaoDich`. Yêu cầu:
- Chỉ `DaKetNoi` mới có phương thức `bat_dau_giao_dich()`.
- Chỉ `TrongGiaoDich` mới có `truy_van()`, `commit()` và `rollback()`.
- `commit()` và `rollback()` đưa kết nối trở về trạng thái `DaKetNoi`.

<details>
<summary><b>Gợi ý</b></summary>

Mẫu giống hệt `DonHang<TT>`. Điểm mới: `commit` và `rollback` đi **ngược** về `KetNoi<DaKetNoi>` — điều đó hoàn toàn hợp lệ, vì typestate không bắt buộc phải là đường một chiều.
</details>

<details>
<summary><b>Lời giải</b></summary>

```rust
use std::marker::PhantomData;

pub struct ChuaKetNoi;
pub struct DaKetNoi;
pub struct TrongGiaoDich;

pub struct KetNoi<TT> {
    chuoi_ket_noi: String,
    nhat_ky: Vec<String>,
    _tt: PhantomData<TT>,
}

impl KetNoi<ChuaKetNoi> {
    pub fn moi(chuoi: &str) -> Self {
        KetNoi { chuoi_ket_noi: chuoi.to_string(), nhat_ky: Vec::new(), _tt: PhantomData }
    }
    pub fn ket_noi(self) -> Result<KetNoi<DaKetNoi>, String> {
        if self.chuoi_ket_noi.is_empty() {
            return Err("Chuỗi kết nối rỗng".to_string());
        }
        Ok(KetNoi { chuoi_ket_noi: self.chuoi_ket_noi, nhat_ky: self.nhat_ky, _tt: PhantomData })
    }
}

impl KetNoi<DaKetNoi> {
    pub fn bat_dau_giao_dich(self) -> KetNoi<TrongGiaoDich> {
        KetNoi { chuoi_ket_noi: self.chuoi_ket_noi, nhat_ky: self.nhat_ky, _tt: PhantomData }
    }
}

impl KetNoi<TrongGiaoDich> {
    pub fn truy_van(mut self, sql: &str) -> Self {
        self.nhat_ky.push(sql.to_string());
        self
    }
    pub fn commit(self) -> KetNoi<DaKetNoi> {
        println!("COMMIT {} câu lệnh", self.nhat_ky.len());
        KetNoi { chuoi_ket_noi: self.chuoi_ket_noi, nhat_ky: Vec::new(), _tt: PhantomData }
    }
    pub fn rollback(self) -> KetNoi<DaKetNoi> {
        println!("ROLLBACK, hủy {} câu lệnh", self.nhat_ky.len());
        KetNoi { chuoi_ket_noi: self.chuoi_ket_noi, nhat_ky: Vec::new(), _tt: PhantomData }
    }
}

fn main() {
    let kn = KetNoi::moi("postgres://localhost/shop").ket_noi().unwrap();
    let kn = kn.bat_dau_giao_dich()
        .truy_van("UPDATE kho SET so_luong = so_luong - 1 WHERE id = 7")
        .truy_van("INSERT INTO don_hang VALUES (7, 1)")
        .commit();
    let _ = kn.bat_dau_giao_dich().truy_van("DELETE FROM tam").rollback();

    // Các dòng sau KHÔNG biên dịch được — và đó chính là mục đích:
    // KetNoi::moi("...").truy_van("SELECT 1");  // chưa kết nối
    // kn.commit();                              // không ở trong giao dịch
}
```

Lưu ý điểm tinh tế: `truy_van` nhận `mut self` và trả về `Self`, cho phép xâu chuỗi phương thức mà vẫn giữ nguyên tắc "mỗi thao tác tiêu thụ giá trị cũ". Đây là mẫu *builder* kết hợp typestate — rất phổ biến trong các thư viện Rust chất lượng cao.
</details>
