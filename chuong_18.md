# Chương 18: Cấu trúc đại số & Luật: Nửa nhóm, Vị nhóm và cách kiểm chứng (Semigroup, Monoid & Verifying Laws)

## Giới thiệu & Mục tiêu học tập

Đọc đến đây bạn đã viết được rất nhiều đường ống dữ liệu. Nhưng hãy thử nhìn lại bốn đoạn mã sau và tìm điểm chung:

```rust
let tong: i64        = so.iter().sum();                             // gộp các số
let cau: String      = tu.concat();                                 // gộp các chuỗi
let tat_ca: Vec<i32> = nhieu_mang.into_iter().flatten().collect();  // gộp các danh sách
let deu_dat: bool    = diem.iter().all(|d| *d >= 5.0);              // gộp các giá trị đúng/sai
```

Bốn dòng này trông khác hẳn nhau. Nhưng thực ra chúng là **cùng một phép toán**, chỉ khác kiểu dữ liệu:

> *Lấy một đống thứ cùng loại, có một cách "gộp hai cái thành một", và một "giá trị rỗng" để bắt đầu. Cứ thế gộp dần cho tới khi còn đúng một kết quả.*

Cái khuôn mẫu đó có tên riêng trong toán học: **Vị nhóm (Monoid)**. Và khi bạn nhận ra nó, bạn có thể viết **một hàm gộp duy nhất** dùng được cho *mọi* kiểu dữ liệu — thay vì viết đi viết lại hàng chục hàm `tinh_tong`, `noi_chuoi`, `gop_danh_sach`.

Nhưng chương này còn dạy một thứ quan trọng hơn cả nội dung: khái niệm **Luật (Law)**.

> **Một trừu tượng không phải là một cái tên. Nó là cái tên CỘNG VỚI những đẳng thức luôn luôn phải đúng.**

Nếu bạn chỉ đặt tên "Vị nhóm" cho kiểu dữ liệu của mình mà phép gộp không tuân luật, mọi tối ưu hóa dựa trên trừu tượng đó (gộp song song, chia nhỏ, đảo thứ tự) sẽ cho kết quả sai. Vì vậy chương này cũng là chương đầu tiên trong giáo trình mà **luật được biến thành bài kiểm thử chạy được** — nền móng cho toàn bộ Chương 19.

Mục tiêu học tập của chương này:
- Nắm được thang bậc **Magma → Nửa nhóm → Vị nhóm → Nhóm** và biết mỗi bậc đòi hỏi thêm điều gì.
- Viết được `trait NuaNhom` và `trait ViNhom` trong Rust, cài đặt cho `String`, `Vec<T>`, số, giá trị logic và các kiểu bọc (newtype).
- Hiểu **vì sao `i64` có tận HAI vị nhóm** (`Tong` và `Tich`) và vì sao Rust buộc phải dùng kiểu bọc để phân biệt.
- Nhận ra các vị nhóm **đã có sẵn trong thư viện chuẩn Rust**: `Default`, `Sum`, `Product`, `Extend`, `Ordering::then`, `Option::or`.
- Hiểu **luật phản xạ** và lý do sâu xa vì sao `f64` chỉ có `PartialEq` chứ không có `Eq` — bài học sống động nhất về "luật có thật".
- Biến luật thành **kiểm thử theo tính chất (property-based testing)** chạy được bằng `cargo test`.

---

## Hình tượng hóa đời sống (Intuitive Everyday Analogy)

```
┌──────────────────────────────────────────────────────────────────────────────────┐
│           HÌNH TƯỢNG ĐỜI SỐNG: XẾP CHỒNG HỘP CARTON TRONG KHO HÀNG               │
├──────────────────────────────────────────────────────────────────────────────────┤
│                                                                                  │
│  Quy tắc của kho: hai hộp bất kỳ luôn CHỒNG ĐƯỢC lên nhau thành MỘT chồng mới.   │
│                                                                                  │
│   ┌───┐   ┌───┐          ┌───┐                                                   │
│   │ A │ ⊕ │ B │   ==>    │ A │   → Đây là NỬA NHÓM (Semigroup):                  │
│   └───┘   └───┘          ├───┤     "luôn gộp được 2 thành 1"                     │
│                          │ B │                                                   │
│                          └───┘                                                   │
│                                                                                  │
│  LUẬT KẾT HỢP: xếp (A trên B) rồi đặt lên C  ==  xếp A lên (B trên C)            │
│  ┌───┐ ┌───┐ ┌───┐        Chồng cuối cùng GIỐNG HỆT NHAU, chỉ khác thứ tự thao   │
│  │ A │ │ B │ │ C │        tác. → Nhờ luật này, 100 nhân viên có thể chia nhau    │
│  └───┘ └───┘ └───┘        xếp từng đoạn rồi ghép lại — SONG SONG HÓA ĐƯỢC!       │
│                                                                                  │
│  ┌ ─ ─ ─ ┐                                                                       │
│  │ HỘP   │  ⊕ bất kỳ chồng nào  ==  chính chồng đó (không đổi gì cả)             │
│  │ RỖNG  │  → Đây là PHẦN TỬ ĐƠN VỊ. Có nó thì Nửa nhóm lên hạng VỊ NHÓM.        │
│  └ ─ ─ ─ ┘     Hộp rỗng cực hữu ích: nó là "điểm bắt đầu" khi kho TRỐNG TRƠN.    │
│                                                                                  │
│  ┌───┐ ┌───┐                                                                     │
│  │ A │⊕│A⁻¹│ == HỘP RỖNG   → Mỗi hộp có một "hộp nghịch đảo" hủy được nó:        │
│  └───┘ └───┘                  Đây là NHÓM (Group). Ví dụ: +5 và −5.              │
└──────────────────────────────────────────────────────────────────────────────────┘
```

### 1. Chồng hộp được = Nửa nhóm (Semigroup)

Trong kho hàng, hai chồng carton bất kỳ luôn chồng lên nhau được thành một chồng mới. Phép "chồng lên" này có ba đặc điểm:
- **Đóng kín**: kết quả vẫn là một chồng carton, không biến thành cái xe đạp. (Trong lập trình: `ghep(T, T) -> T`.)
- **Kết hợp**: chồng A lên B rồi đặt cả hai lên C, hay chồng B lên C rồi đặt A lên trên — kết quả cuối cùng y hệt.
- **Không nhất thiết đổi chỗ được**: A trên B khác B trên A (nếu A nặng và B mỏng thì đổi chỗ là bẹp!).

### 2. Hộp rỗng = Phần tử đơn vị (Identity), nâng cấp thành Vị nhóm (Monoid)

Kho có một chiếc "hộp rỗng" đặc biệt: chồng nó lên bất cứ thứ gì cũng không làm thay đổi gì cả.

Nghe có vẻ vô dụng, nhưng nó giải quyết một tình huống rất thực tế: **kho trống trơn thì trả về cái gì?** Nếu bạn phải tính tổng của một danh sách rỗng, câu trả lời phải là `0`. Tính tích của danh sách rỗng thì phải là `1`. Nối một danh sách chuỗi rỗng thì ra `""`. Ba con số ấy chính là ba "hộp rỗng" của ba vị nhóm khác nhau.

### 3. Vì sao phải quan tâm? — Vì luật kết hợp cho phép SONG SONG HÓA

Đây là lý do thực dụng nhất, không phải lý do toán học:

```
Tính tổng 8 con số theo kiểu tuần tự  : ((((((a+b)+c)+d)+e)+f)+g)+h   → 7 bước NỐI TIẾP
Tính tổng 8 con số theo kiểu chia đôi : ((a+b)+(c+d)) + ((e+f)+(g+h)) → 3 tầng, mỗi tầng SONG SONG
```

Cả hai cho **cùng một kết quả** — và điều đó được bảo đảm bởi *luật kết hợp*, không phải bởi may mắn. Đây chính là nền tảng để thư viện `rayon` chia dữ liệu ra nhiều nhân CPU rồi ghép lại. Nếu phép gộp của bạn không kết hợp, chương trình song song sẽ cho kết quả sai **một cách ngẫu nhiên** — loại lỗi khó tìm nhất trên đời.

---

## Khái niệm & Cơ chế kỹ thuật chuyên sâu (In-Depth Technical Mechanics)

### 1. Thang bậc các cấu trúc đại số

Mỗi bậc chỉ thêm đúng **một** đòi hỏi so với bậc dưới:

| Bậc | Đòi hỏi | Ví dụ trong Rust |
|---|---|---|
| **Magma** | Có phép `⊕: (T, T) -> T` đóng kín. Chấm hết. | Phép trừ trên `i64` (không kết hợp!) |
| **Nửa nhóm (Semigroup)** | Magma + **luật kết hợp** | `String` nối chuỗi, `Vec` nối mảng, `max`, `min` |
| **Vị nhóm (Monoid)** | Nửa nhóm + **phần tử đơn vị** `e` với `e ⊕ a = a ⊕ e = a` | `String` với `""`, `i64` cộng với `0` |
| **Nhóm (Group)** | Vị nhóm + **phần tử nghịch đảo** `a⁻¹` với `a ⊕ a⁻¹ = e` | `i64` cộng với phép đổi dấu |
| **Nhóm giao hoán (Abelian)** | Nhóm + **luật giao hoán** `a ⊕ b = b ⊕ a` | `i64` cộng (nối chuỗi thì KHÔNG) |

Viết thành các luật hình thức:

```
(L1) Kết hợp   : (a ⊕ b) ⊕ c  =  a ⊕ (b ⊕ c)
(L2) Đơn vị    : e ⊕ a  =  a  =  a ⊕ e
(L3) Nghịch đảo: a ⊕ a⁻¹  =  e  =  a⁻¹ ⊕ a
(L4) Giao hoán : a ⊕ b  =  b ⊕ a
```

> **Ví dụ phản chứng đáng nhớ**: phép trừ trên số nguyên là Magma nhưng **không** phải nửa nhóm, vì
> `(10 − 3) − 2 = 5` trong khi `10 − (3 − 2) = 9`. Đây là lý do bạn không bao giờ được chia nhỏ một phép trừ dài ra nhiều luồng!

### 2. Định nghĩa trait trong Rust

```rust
/// Nửa nhóm: bất kỳ kiểu nào có phép gộp hai thành một, tuân luật kết hợp.
pub trait NuaNhom {
    fn ghep(self, khac: Self) -> Self;
}

/// Vị nhóm: nửa nhóm có thêm một "phần tử rỗng".
pub trait ViNhom: NuaNhom + Sized {
    fn don_vi() -> Self;
}
```

Chú ý hai chi tiết thiết kế rất "Rust":
- `fn ghep(self, khac: Self) -> Self` nhận `self` **theo giá trị**, không phải `&self`. Nhờ vậy, khi gộp hai `String` ta có thể *tái sử dụng* bộ đệm của chuỗi thứ nhất thay vì cấp phát mới — đúng tinh thần zero-cost.
- `ViNhom: NuaNhom` là quan hệ **siêu trait (supertrait)**: mọi vị nhóm bắt buộc trước hết phải là một nửa nhóm. Đây chính là cách Rust biểu diễn quan hệ "kế thừa" giữa các cấu trúc đại số. (Bạn đã gặp mẫu này ở Chương 15 với `Fn: FnMut: FnOnce`.)

Có hai trait đó rồi, ta viết được **một hàm gộp duy nhất dùng chung cho mọi kiểu**:

```rust
pub fn gop_tat_ca<M: ViNhom>(danh_sach: impl IntoIterator<Item = M>) -> M {
    danh_sach.into_iter().fold(M::don_vi(), |tich_luy, x| tich_luy.ghep(x))
}
```

Hàm 3 dòng này thay thế được `tinh_tong`, `noi_chuoi`, `gop_mang`, `tim_max`… và mọi hàm gộp mà bạn sẽ cần trong tương lai. Đó là sức mạnh của việc gọi đúng tên một trừu tượng.

### 3. Vì sao số nguyên cần KIỂU BỌC (newtype)?

Kiểu `i64` có tận **hai** cấu trúc vị nhóm hoàn toàn hợp lệ:

| Vị nhóm | Phép gộp | Phần tử đơn vị |
|---|---|---|
| Cộng | `a + b` | `0` |
| Nhân | `a * b` | `1` |

Rust không cho phép viết `impl ViNhom for i64` hai lần (lỗi **E0119: conflicting implementations**). Cách giải quyết chuẩn mực của cả Rust lẫn Haskell là **bọc số vào một kiểu mới**:

```rust
pub struct Tong(pub i64);   // đại diện vị nhóm cộng
pub struct Tich(pub i64);   // đại diện vị nhóm nhân
```

Đây là lần đầu tiên trong giáo trình bạn gặp **mẫu kiểu bọc (newtype pattern)** — một kỹ thuật cực kỳ quan trọng mà chúng ta sẽ khai thác triệt để ở Chương 20 để mô hình hóa nghiệp vụ. Ghi nhớ: *kiểu bọc là cách bạn nói với trình biên dịch rằng "cùng một con số nhưng mang ý nghĩa khác nhau"*.

### 4. Bạn đã dùng vị nhóm mà không biết — bản đồ sang thư viện chuẩn Rust

| Ý tưởng đại số | Nó nằm ở đâu trong Rust chuẩn |
|---|---|
| Phần tử đơn vị | Trait **`Default`** — `String::default()` là `""`, `i32::default()` là `0`, `Vec::default()` là `[]` |
| Vị nhóm cộng | Trait **`Sum`** đứng sau phương thức `.sum()` mà bạn đã dùng ở Chương 16 |
| Vị nhóm nhân | Trait **`Product`** đứng sau `.product()` |
| Gộp tập hợp | Trait **`Extend`** đứng sau `.extend()` |
| Vị nhóm "so sánh nhiều tiêu chí" | **`Ordering::then`** / `then_with` — gộp nhiều kết quả so sánh, đơn vị là `Ordering::Equal` |
| Vị nhóm "lấy cái đầu tiên có" | **`Option::or`** — đơn vị là `None` |

Ví dụ đẹp nhất là sắp xếp theo nhiều tiêu chí. Nó chính là phép gộp của một vị nhóm:

```rust
nhan_vien.sort_by(|a, b| {
    a.phong_ban.cmp(&b.phong_ban)            // tiêu chí 1
        .then(b.tham_nien.cmp(&a.tham_nien)) // ⊕ tiêu chí 2 (giảm dần)
        .then(a.ho_ten.cmp(&b.ho_ten))       // ⊕ tiêu chí 3
});
```

`Ordering::Equal` chính là "hộp rỗng": gộp nó vào không làm đổi kết quả — đúng nghĩa "hai người bằng nhau ở tiêu chí này, xét tiếp tiêu chí sau".

### 5. Luật có thật: câu chuyện `f64`, `NaN` và trait `Eq`

Đây là bằng chứng sống động nhất cho thấy luật **không phải chuyện lý thuyết suông** — chúng được mã hóa thẳng vào thư viện chuẩn Rust.

Rust có hai trait so sánh bằng:
- **`PartialEq`**: chỉ đòi hỏi phép `==` tồn tại.
- **`Eq`**: đòi hỏi thêm rằng `==` là một **quan hệ tương đương**, tức phải thỏa mãn ba luật:
  - *Phản xạ (reflexive)*: `a == a` luôn đúng.
  - *Đối xứng (symmetric)*: `a == b` thì `b == a`.
  - *Bắc cầu (transitive)*: `a == b` và `b == c` thì `a == c`.

Bây giờ hãy thử với số thực:

```rust
let nan = f64::NAN;
println!("{}", nan == nan);   // in ra: false  (!!)
```

`NaN` ("Not a Number", kết quả của `0.0/0.0`) **không bằng chính nó** theo chuẩn IEEE 754. Luật phản xạ bị phá vỡ. Vì vậy Rust **từ chối** cài đặt `Eq` cho `f64` — và hệ quả dây chuyền rất cụ thể:

```rust
use std::collections::HashSet;
// let tap: HashSet<f64> = HashSet::new();  // LỖI: f64 không có Eq + Hash
let tap: HashSet<i64> = HashSet::new();     // OK
```

Bạn không thể dùng `f64` làm khóa `HashMap` hay phần tử `HashSet`. Đây không phải Rust khó tính vô cớ — nếu cho phép, bảng băm sẽ chứa một phần tử mà bạn **vĩnh viễn không tra cứu lại được**, vì phép so sánh khóa luôn trả `false`.

> **Bài học rút ra**: khi bạn tuyên bố một kiểu thỏa mãn một trừu tượng, bạn đang **hứa** với mọi đoạn mã sử dụng nó. Trình biên dịch kiểm tra được chữ ký, nhưng chỉ **bài kiểm thử** mới kiểm tra được lời hứa.

### 6. Kiểm thử theo tính chất (Property-Based Testing)

Kiểm thử thông thường kiểm tra **một ví dụ cụ thể**:

```rust
assert_eq!(Tong(2).ghep(Tong(3)), Tong(5));   // đúng với 2 và 3... còn các số khác?
```

Kiểm thử theo tính chất kiểm tra **một đẳng thức đúng với mọi đầu vào**:

```rust
// Với MỌI a, b, c: (a ⊕ b) ⊕ c == a ⊕ (b ⊕ c)
for (a, b, c) in cac_bo_ba_mau {
    assert_eq!(a.ghep(b).ghep(c), a.ghep(b.ghep(c)));
}
```

Trong dự án thực tế, bạn dùng crate `proptest` hoặc `quickcheck` để sinh hàng nghìn bộ dữ liệu ngẫu nhiên và **tự động thu nhỏ (shrink)** phản ví dụ khi tìm thấy lỗi. Ở giáo trình này, để không phụ thuộc thư viện ngoài, chúng ta tự viết một bộ sinh số giả ngẫu nhiên (thuật toán *đồng dư tuyến tính* — LCG) chỉ trong 6 dòng. Cách làm giống hệt nhau, chỉ khác quy mô.

---

## Mã nguồn minh họa thực chiến (Idiomatic Runnable Rust Blueprint)

Chương trình dưới đây xây dựng một **Bộ thống kê nhật ký máy chủ (Server Log Aggregator)**. Điểm đắt giá: nó tính **bốn chỉ số khác nhau chỉ trong MỘT lượt duyệt duy nhất**, nhờ ghép bốn vị nhóm lại thành một *vị nhóm tích*.

```rust
// Tệp: src/main.rs
// Chương trình thực chiến: Nửa nhóm, Vị nhóm và Kiểm thử theo tính chất

use std::cmp::Ordering;
use std::fmt::Debug;

// ============================================================================
// PHẦN 1: HAI TRAIT NỀN TẢNG
// ============================================================================

/// Nửa nhóm (Semigroup): có phép gộp hai thành một, tuân LUẬT KẾT HỢP.
pub trait NuaNhom {
    fn ghep(self, khac: Self) -> Self;
}

/// Vị nhóm (Monoid): nửa nhóm có thêm PHẦN TỬ ĐƠN VỊ.
pub trait ViNhom: NuaNhom + Sized {
    fn don_vi() -> Self;
}

/// Hàm gộp vạn năng: dùng được cho MỌI vị nhóm.
/// Nó thay thế cho tinh_tong, noi_chuoi, gop_mang, tim_max... tất cả.
pub fn gop_tat_ca<M: ViNhom>(danh_sach: impl IntoIterator<Item = M>) -> M {
    danh_sach
        .into_iter()
        .fold(M::don_vi(), |tich_luy, x| tich_luy.ghep(x))
}

// ============================================================================
// PHẦN 2: CÁC KIỂU CÓ SẴN CŨNG LÀ VỊ NHÓM
// ============================================================================

impl NuaNhom for String {
    fn ghep(self, khac: Self) -> Self {
        self + &khac // tái sử dụng bộ đệm của chuỗi thứ nhất
    }
}
impl ViNhom for String {
    fn don_vi() -> Self {
        String::new()
    }
}

impl<T> NuaNhom for Vec<T> {
    fn ghep(mut self, mut khac: Self) -> Self {
        self.append(&mut khac);
        self
    }
}
impl<T> ViNhom for Vec<T> {
    fn don_vi() -> Self {
        Vec::new()
    }
}

// ============================================================================
// PHẦN 3: KIỂU BỌC (NEWTYPE) — VÌ SỐ NGUYÊN CÓ NHIỀU VỊ NHÓM
// ============================================================================

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct Tong(pub i64);
impl NuaNhom for Tong {
    fn ghep(self, k: Self) -> Self {
        Tong(self.0 + k.0)
    }
}
impl ViNhom for Tong {
    fn don_vi() -> Self {
        Tong(0)
    }
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct Tich(pub i64);
impl NuaNhom for Tich {
    fn ghep(self, k: Self) -> Self {
        Tich(self.0.wrapping_mul(k.0))
    }
}
impl ViNhom for Tich {
    fn don_vi() -> Self {
        Tich(1) // Chú ý: đơn vị của phép nhân là 1, KHÔNG phải 0!
    }
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct LonNhat(pub i64);
impl NuaNhom for LonNhat {
    fn ghep(self, k: Self) -> Self {
        LonNhat(self.0.max(k.0))
    }
}
impl ViNhom for LonNhat {
    fn don_vi() -> Self {
        LonNhat(i64::MIN) // "âm vô cực": gộp với gì cũng thua
    }
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct NhoNhat(pub i64);
impl NuaNhom for NhoNhat {
    fn ghep(self, k: Self) -> Self {
        NhoNhat(self.0.min(k.0))
    }
}
impl ViNhom for NhoNhat {
    fn don_vi() -> Self {
        NhoNhat(i64::MAX)
    }
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct MoiDeu(pub bool); // "tất cả đều đúng" — tương ứng .all()
impl NuaNhom for MoiDeu {
    fn ghep(self, k: Self) -> Self {
        MoiDeu(self.0 && k.0)
    }
}
impl ViNhom for MoiDeu {
    fn don_vi() -> Self {
        MoiDeu(true)
    }
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct CoIt(pub bool); // "có ít nhất một cái đúng" — tương ứng .any()
impl NuaNhom for CoIt {
    fn ghep(self, k: Self) -> Self {
        CoIt(self.0 || k.0)
    }
}
impl ViNhom for CoIt {
    fn don_vi() -> Self {
        CoIt(false)
    }
}

/// Vị nhóm "lấy cái đầu tiên có giá trị" — chính là ý tưởng của `Option::or`.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct DauTien<T>(pub Option<T>);
impl<T> NuaNhom for DauTien<T> {
    fn ghep(self, k: Self) -> Self {
        if self.0.is_some() {
            self
        } else {
            k
        }
    }
}
impl<T> ViNhom for DauTien<T> {
    fn don_vi() -> Self {
        DauTien(None)
    }
}

// ============================================================================
// PHẦN 4: VỊ NHÓM TÍCH — GHÉP NHIỀU VỊ NHÓM THÀNH MỘT
// ============================================================================
// Mấu chốt: nếu A và B đều là vị nhóm thì cặp (A, B) cũng là vị nhóm.
// Nhờ vậy ta tính được NHIỀU chỉ số chỉ trong MỘT lượt duyệt dữ liệu.

impl<A: NuaNhom, B: NuaNhom> NuaNhom for (A, B) {
    fn ghep(self, k: Self) -> Self {
        (self.0.ghep(k.0), self.1.ghep(k.1))
    }
}
impl<A: ViNhom, B: ViNhom> ViNhom for (A, B) {
    fn don_vi() -> Self {
        (A::don_vi(), B::don_vi())
    }
}

impl<A: NuaNhom, B: NuaNhom, C: NuaNhom, D: NuaNhom> NuaNhom for (A, B, C, D) {
    fn ghep(self, k: Self) -> Self {
        (
            self.0.ghep(k.0),
            self.1.ghep(k.1),
            self.2.ghep(k.2),
            self.3.ghep(k.3),
        )
    }
}
impl<A: ViNhom, B: ViNhom, C: ViNhom, D: ViNhom> ViNhom for (A, B, C, D) {
    fn don_vi() -> Self {
        (A::don_vi(), B::don_vi(), C::don_vi(), D::don_vi())
    }
}

// ============================================================================
// PHẦN 5: ỨNG DỤNG THẬT — THỐNG KÊ NHẬT KÝ MÁY CHỦ
// ============================================================================

#[derive(Debug, Clone, PartialEq)]
pub struct BanGhiTruyCap {
    pub duong_dan: String,
    pub ma_trang_thai: u16,
    pub thoi_gian_ms: i64,
}

/// Bốn chỉ số cần tính, gói trong một vị nhóm tích 4 thành phần.
pub type ThongKe = (Tong, LonNhat, NhoNhat, CoIt);

/// Biến một bản ghi thành "đóng góp" của nó vào thống kê tổng.
pub fn thanh_thong_ke(bg: &BanGhiTruyCap) -> ThongKe {
    (
        Tong(bg.thoi_gian_ms),
        LonNhat(bg.thoi_gian_ms),
        NhoNhat(bg.thoi_gian_ms),
        CoIt(bg.ma_trang_thai >= 500),
    )
}

// ============================================================================
// PHẦN 6: BỘ SINH SỐ GIẢ NGẪU NHIÊN CHO KIỂM THỬ THEO TÍNH CHẤT
// ============================================================================

/// Bộ sinh đồng dư tuyến tính (LCG) — tất định nên kiểm thử luôn lặp lại được.
pub struct BoSinh(u64);
impl BoSinh {
    pub fn moi(hat_giong: u64) -> Self {
        BoSinh(hat_giong)
    }
    pub fn so_tiep(&mut self) -> i64 {
        // Hằng số của cuốn Numerical Recipes
        self.0 = self
            .0
            .wrapping_mul(6364136223846793005)
            .wrapping_add(1442695040888963407);
        ((self.0 >> 33) as i64) % 1000 - 500 // dải [-500, 499]
    }
}

/// Kiểm chứng LUẬT KẾT HỢP trên nhiều mẫu giả ngẫu nhiên.
pub fn kiem_chung_ket_hop<M, F>(ten: &str, tao: F, so_mau: usize) -> bool
where
    M: NuaNhom + Clone + PartialEq + Debug,
    F: Fn(i64) -> M,
{
    let mut sinh = BoSinh::moi(2026);
    for _ in 0..so_mau {
        let a = tao(sinh.so_tiep());
        let b = tao(sinh.so_tiep());
        let c = tao(sinh.so_tiep());
        let trai = a.clone().ghep(b.clone()).ghep(c.clone());
        let phai = a.clone().ghep(b.clone().ghep(c.clone()));
        if trai != phai {
            println!("  ✗ {} VI PHẠM luật kết hợp: {:?} vs {:?}", ten, trai, phai);
            return false;
        }
    }
    println!("  ✓ {}: luật kết hợp đúng trên {} bộ mẫu", ten, so_mau);
    true
}

/// Kiểm chứng LUẬT ĐƠN VỊ trên nhiều mẫu giả ngẫu nhiên.
pub fn kiem_chung_don_vi<M, F>(ten: &str, tao: F, so_mau: usize) -> bool
where
    M: ViNhom + Clone + PartialEq + Debug,
    F: Fn(i64) -> M,
{
    let mut sinh = BoSinh::moi(777);
    for _ in 0..so_mau {
        let a = tao(sinh.so_tiep());
        if M::don_vi().ghep(a.clone()) != a || a.clone().ghep(M::don_vi()) != a {
            println!("  ✗ {} VI PHẠM luật đơn vị với {:?}", ten, a);
            return false;
        }
    }
    println!("  ✓ {}: luật đơn vị đúng trên {} bộ mẫu", ten, so_mau);
    true
}

// ============================================================================
// CHƯƠNG TRÌNH ĐIỀU HÀNH CHÍNH
// ============================================================================

fn main() {
    println!("============================================================");
    println!("    CẤU TRÚC ĐẠI SỐ: NỬA NHÓM, VỊ NHÓM VÀ LUẬT             ");
    println!("============================================================");

    // ------------------------------------------------------------------
    // 1. MỘT HÀM GỘP DUY NHẤT DÙNG CHO MỌI KIỂU
    // ------------------------------------------------------------------
    println!("\n1. HÀM `gop_tat_ca` VẠN NĂNG");
    let so = vec![Tong(3), Tong(8), Tong(-2), Tong(11)];
    println!("   Tổng các số       : {:?}", gop_tat_ca(so));

    let tich = vec![Tich(2), Tich(3), Tich(7)];
    println!("   Tích các số       : {:?}", gop_tat_ca(tich));

    let chuoi = vec![
        String::from("Rust "),
        String::from("thật "),
        String::from("tuyệt!"),
    ];
    println!("   Nối chuỗi         : {:?}", gop_tat_ca(chuoi));

    let mang = vec![vec![1, 2], vec![3], vec![4, 5, 6]];
    println!("   Gộp danh sách     : {:?}", gop_tat_ca(mang));

    let dat = vec![MoiDeu(true), MoiDeu(true), MoiDeu(false)];
    println!("   Tất cả đều đạt?   : {:?}", gop_tat_ca(dat));

    let cau_hinh: Vec<DauTien<&str>> = vec![
        DauTien(None),                // biến môi trường: không có
        DauTien(Some("config.toml")), // tệp cấu hình: có!
        DauTien(Some("mac_dinh")),    // giá trị mặc định (không dùng tới)
    ];
    println!("   Nguồn cấu hình đầu: {:?}", gop_tat_ca(cau_hinh));

    // ------------------------------------------------------------------
    // 2. DANH SÁCH RỖNG — GIÁ TRỊ CỦA "HỘP RỖNG"
    // ------------------------------------------------------------------
    println!("\n2. VÌ SAO CẦN PHẦN TỬ ĐƠN VỊ?");
    let rong_cong: Vec<Tong> = Vec::new();
    let rong_nhan: Vec<Tich> = Vec::new();
    println!("   Tổng của danh sách RỖNG: {:?}  (đúng: 0)", gop_tat_ca(rong_cong));
    println!(
        "   Tích của danh sách RỖNG: {:?}  (đúng: 1, KHÔNG phải 0!)",
        gop_tat_ca(rong_nhan)
    );

    // ------------------------------------------------------------------
    // 3. VỊ NHÓM TÍCH: 4 CHỈ SỐ TRONG 1 LƯỢT DUYỆT
    // ------------------------------------------------------------------
    println!("\n3. VỊ NHÓM TÍCH — 4 CHỈ SỐ, 1 LƯỢT DUYỆT");
    let nhat_ky = vec![
        BanGhiTruyCap { duong_dan: "/api/don-hang".into(), ma_trang_thai: 200, thoi_gian_ms: 42 },
        BanGhiTruyCap { duong_dan: "/api/thanh-toan".into(), ma_trang_thai: 500, thoi_gian_ms: 1350 },
        BanGhiTruyCap { duong_dan: "/api/san-pham".into(), ma_trang_thai: 200, thoi_gian_ms: 17 },
        BanGhiTruyCap { duong_dan: "/api/kho".into(), ma_trang_thai: 404, thoi_gian_ms: 8 },
        BanGhiTruyCap { duong_dan: "/api/don-hang".into(), ma_trang_thai: 200, thoi_gian_ms: 63 },
    ];

    let (tong, cham_nhat, nhanh_nhat, co_loi_may_chu): ThongKe =
        gop_tat_ca(nhat_ky.iter().map(thanh_thong_ke));

    println!("   Số bản ghi          : {}", nhat_ky.len());
    println!("   Tổng thời gian      : {} ms", tong.0);
    println!("   Trung bình          : {} ms", tong.0 / nhat_ky.len() as i64);
    println!("   Chậm nhất           : {} ms", cham_nhat.0);
    println!("   Nhanh nhất          : {} ms", nhanh_nhat.0);
    println!("   Có lỗi máy chủ 5xx? : {}", co_loi_may_chu.0);

    // ------------------------------------------------------------------
    // 4. LUẬT KẾT HỢP CHO PHÉP CHIA NHỎ & SONG SONG HÓA
    // ------------------------------------------------------------------
    println!("\n4. CHIA NHỎ RỒI GHÉP LẠI CHO CÙNG KẾT QUẢ");
    let tat_ca: ThongKe = gop_tat_ca(nhat_ky.iter().map(thanh_thong_ke));
    let (nua_dau, nua_sau) = nhat_ky.split_at(2);
    let phan_1: ThongKe = gop_tat_ca(nua_dau.iter().map(thanh_thong_ke));
    let phan_2: ThongKe = gop_tat_ca(nua_sau.iter().map(thanh_thong_ke));
    let ghep_lai = phan_1.ghep(phan_2);
    assert_eq!(tat_ca, ghep_lai);
    println!("   Gộp 1 lượt     : {:?}", tat_ca);
    println!("   Chia 2 rồi ghép: {:?}", ghep_lai);
    println!("   → GIỐNG NHAU ✓ Đây chính là cơ sở để chạy song song trên nhiều nhân CPU.");

    // ------------------------------------------------------------------
    // 5. KIỂM CHỨNG LUẬT BẰNG KIỂM THỬ THEO TÍNH CHẤT
    // ------------------------------------------------------------------
    println!("\n5. KIỂM THỬ THEO TÍNH CHẤT (1.000 bộ mẫu mỗi luật)");
    kiem_chung_ket_hop("Tong   ", Tong, 1000);
    kiem_chung_ket_hop("Tich   ", Tich, 1000);
    kiem_chung_ket_hop("LonNhat", LonNhat, 1000);
    kiem_chung_ket_hop("String ", |n: i64| n.to_string(), 1000);
    kiem_chung_don_vi("Tong   ", Tong, 1000);
    kiem_chung_don_vi("Tich   ", Tich, 1000);
    kiem_chung_don_vi("LonNhat", LonNhat, 1000);

    // ------------------------------------------------------------------
    // 6. PHẢN VÍ DỤ: PHÉP TRỪ KHÔNG PHẢI NỬA NHÓM
    // ------------------------------------------------------------------
    println!("\n6. PHẢN VÍ DỤ — PHÉP TRỪ VI PHẠM LUẬT KẾT HỢP");
    let (a, b, c) = (10i64, 3i64, 2i64);
    println!("   (10 - 3) - 2 = {}", (a - b) - c);
    println!("   10 - (3 - 2) = {}", a - (b - c));
    println!("   → KHÁC NHAU! Nên KHÔNG BAO GIỜ được chia nhỏ phép trừ ra nhiều luồng.");

    // ------------------------------------------------------------------
    // 7. VỊ NHÓM CÓ SẴN TRONG THƯ VIỆN CHUẨN: Ordering::then
    // ------------------------------------------------------------------
    println!("\n7. VỊ NHÓM `Ordering` — SẮP XẾP THEO NHIỀU TIÊU CHÍ");
    let mut nhan_vien = vec![
        ("Kỹ thuật", 3u32, "An"),
        ("Kinh doanh", 5, "Bình"),
        ("Kỹ thuật", 5, "Cường"),
        ("Kỹ thuật", 5, "Anh"),
    ];
    nhan_vien.sort_by(|x, y| {
        x.0.cmp(y.0) // 1. phòng ban tăng dần
            .then(y.1.cmp(&x.1)) // ⊕ 2. thâm niên giảm dần
            .then(x.2.cmp(y.2)) // ⊕ 3. họ tên tăng dần
    });
    for nv in &nhan_vien {
        println!("   {:<12} {} năm  {}", nv.0, nv.1, nv.2);
    }
    println!("   (Ordering::Equal chính là \"hộp rỗng\": bằng nhau thì xét tiêu chí sau)");

    // ------------------------------------------------------------------
    // 8. LUẬT PHẢN XẠ VÀ CÂU CHUYỆN f64 / NaN
    // ------------------------------------------------------------------
    println!("\n8. LUẬT CÓ THẬT: f64 KHÔNG CÓ TRAIT `Eq`");
    let nan = f64::NAN;
    println!("   f64::NAN == f64::NAN  ->  {}", nan == nan);
    println!("   → Luật phản xạ (a == a) bị phá vỡ, nên Rust TỪ CHỐI cài `Eq` cho f64.");
    println!("   → Hệ quả: không thể dùng f64 làm khóa HashMap / phần tử HashSet.");
    let so_sanh: Ordering = 3i64.cmp(&5i64);
    println!("   (Còn i64 thì có đủ Eq + Ord: 3.cmp(&5) = {:?})", so_sanh);

    println!("\n============================================================");
    println!("   MỘT TRỪU TƯỢNG = MỘT CÁI TÊN + NHỮNG LUẬT LUÔN ĐÚNG      ");
    println!("============================================================");
}

// ============================================================================
// KIỂM THỬ TỰ ĐỘNG: LUẬT TRỞ THÀNH TEST CHẠY ĐƯỢC BẰNG `cargo test`
// ============================================================================

#[cfg(test)]
mod kiem_thu {
    use super::*;

    #[test]
    fn tong_tuan_thu_luat_ket_hop() {
        assert!(kiem_chung_ket_hop("Tong", Tong, 500));
    }

    #[test]
    fn tong_tuan_thu_luat_don_vi() {
        assert!(kiem_chung_don_vi("Tong", Tong, 500));
    }

    #[test]
    fn tich_tuan_thu_ca_hai_luat() {
        assert!(kiem_chung_ket_hop("Tich", Tich, 500));
        assert!(kiem_chung_don_vi("Tich", Tich, 500));
    }

    #[test]
    fn chuoi_tuan_thu_luat_ket_hop() {
        assert!(kiem_chung_ket_hop("String", |n: i64| n.to_string(), 500));
    }

    #[test]
    fn danh_sach_rong_tra_ve_phan_tu_don_vi() {
        let rong_cong: Vec<Tong> = Vec::new();
        let rong_nhan: Vec<Tich> = Vec::new();
        let rong_max: Vec<LonNhat> = Vec::new();
        assert_eq!(gop_tat_ca(rong_cong), Tong(0));
        assert_eq!(gop_tat_ca(rong_nhan), Tich(1));
        assert_eq!(gop_tat_ca(rong_max), LonNhat(i64::MIN));
    }

    #[test]
    fn vi_nhom_tich_gop_dung_bon_chi_so() {
        let nhat_ky = vec![
            BanGhiTruyCap { duong_dan: "/a".into(), ma_trang_thai: 200, thoi_gian_ms: 10 },
            BanGhiTruyCap { duong_dan: "/b".into(), ma_trang_thai: 503, thoi_gian_ms: 40 },
            BanGhiTruyCap { duong_dan: "/c".into(), ma_trang_thai: 200, thoi_gian_ms: 25 },
        ];
        let (tong, max, min, loi): ThongKe = gop_tat_ca(nhat_ky.iter().map(thanh_thong_ke));
        assert_eq!(tong, Tong(75));
        assert_eq!(max, LonNhat(40));
        assert_eq!(min, NhoNhat(10));
        assert_eq!(loi, CoIt(true));
    }

    /// Đây là bài test QUAN TRỌNG NHẤT chương: nó chứng minh rằng
    /// chia nhỏ dữ liệu rồi ghép lại luôn cho cùng kết quả —
    /// tức là thuật toán này SONG SONG HÓA ĐƯỢC một cách an toàn.
    #[test]
    fn chia_nho_roi_ghep_lai_cho_cung_ket_qua() {
        let mut sinh = BoSinh::moi(12345);
        let du_lieu: Vec<Tong> = (0..100).map(|_| Tong(sinh.so_tiep())).collect();

        let mot_luot = gop_tat_ca(du_lieu.clone());
        for diem_cat in [0usize, 1, 37, 50, 99, 100] {
            let (trai, phai) = du_lieu.split_at(diem_cat);
            let ghep = gop_tat_ca(trai.to_vec()).ghep(gop_tat_ca(phai.to_vec()));
            assert_eq!(mot_luot, ghep, "Sai khi cắt tại vị trí {}", diem_cat);
        }
    }

    #[test]
    fn phep_tru_khong_phai_nua_nhom() {
        // Phản ví dụ: chứng minh phép trừ VI PHẠM luật kết hợp.
        assert_ne!((10i64 - 3) - 2, 10i64 - (3 - 2));
    }

    #[test]
    fn nan_pha_vo_luat_phan_xa() {
        let nan = f64::NAN;
        assert!(!(nan == nan), "NaN phải KHÔNG bằng chính nó theo IEEE 754");
        // Còn số nguyên thì luôn thỏa luật phản xạ:
        for i in -5i64..5 {
            assert!(i == i);
        }
    }
}
```

---

## Bảng tra cứu lỗi biên dịch & Cách khắc phục (Compiler Error Guide)

| Mã lỗi | Thông báo mẫu từ trình biên dịch | Nguyên nhân cốt lõi | Cách khắc phục nhanh |
|---|---|---|---|
| **E0119** | `conflicting implementations of trait 'ViNhom' for type 'i64'` | Bạn cố cài đặt cùng một trait hai lần cho một kiểu (ví dụ `i64` vừa là vị nhóm cộng vừa là vị nhóm nhân). | Dùng **kiểu bọc (newtype)**: `struct Tong(i64)` và `struct Tich(i64)` — mỗi kiểu bọc mang đúng một ý nghĩa. |
| **E0117** | `only traits defined in the current crate can be implemented for types defined outside of the crate` | **Quy tắc mồ côi (orphan rule)**: bạn không được cài trait của người khác cho kiểu của người khác. | Hoặc trait phải là của bạn (như `NuaNhom` trong chương này), hoặc kiểu phải là của bạn — lại là kiểu bọc! |
| **E0277** | `the trait bound 'X: ViNhom' is not satisfied` | Bạn gọi `gop_tat_ca` với một kiểu chưa cài `ViNhom`, hoặc quên cài `NuaNhom` (siêu trait bắt buộc). | Cài đủ **cả hai** trait. Nhớ rằng `ViNhom: NuaNhom` nghĩa là muốn có vị nhóm thì phải có nửa nhóm trước. |
| **E0507** | `cannot move out of ... which is behind a shared reference` | `fn ghep(self, ...)` nhận `self` theo giá trị, nhưng bạn đang cầm `&M`. | Gọi `.clone()` trước khi gộp, hoặc dùng `.iter().map(...)` để tạo giá trị mới thay vì mượn. |
| **E0382** | `use of moved value` | Trong vòng lặp kiểm chứng luật, bạn dùng lại `a` sau khi nó đã bị `ghep` tiêu thụ. | Thêm ràng buộc `M: Clone` và gọi `a.clone()` như trong hàm `kiem_chung_ket_hop` ở trên. |

### Phân tích lỗi thực tế `E0119` (vì sao bắt buộc phải dùng newtype):

```rust
// ❌ Đoạn mã lỗi minh họa (đã đóng chú thích để tệp vẫn biên dịch được):
// impl NuaNhom for i64 { fn ghep(self, k: Self) -> Self { self + k } }
// impl NuaNhom for i64 { fn ghep(self, k: Self) -> Self { self * k } }
// LỖI E0119: conflicting implementations of trait `NuaNhom` for type `i64`
//
// Trình biên dịch hỏi rất hợp lý: "Khi ai đó viết a.ghep(b) trên hai số i64,
// tôi phải cộng hay phải nhân?" — Không có câu trả lời, nên nó từ chối.

// ✅ Cách sửa: mỗi ý nghĩa một kiểu bọc riêng
pub struct TongSo(pub i64);
pub struct TichSo(pub i64);
// Giờ `TongSo(2).ghep(TongSo(3))` và `TichSo(2).ghep(TichSo(3))` là hai chuyện khác nhau.
```

---

## Tóm tắt chương & Bài tập rèn luyện (Summary & Exercises)

### 4 Điểm cốt lõi cần ghi nhớ:
1. **Vị nhóm = phép gộp + phần tử rỗng + luật**. Nhận ra khuôn mẫu này cho phép bạn viết **một** hàm `gop_tat_ca` thay cho hàng chục hàm gộp chuyên biệt.
2. **Luật kết hợp là giấy phép song song hóa**. Vì `(a⊕b)⊕c = a⊕(b⊕c)`, bạn có thể chia dữ liệu ra nhiều nhân CPU rồi ghép lại mà chắc chắn không sai. Phép trừ vi phạm luật này, nên không bao giờ được chia nhỏ.
3. **Kiểu bọc (newtype) giải quyết bài toán "một kiểu, nhiều ý nghĩa"** và cũng là lối thoát khỏi quy tắc mồ côi. Đây là mẫu thiết kế sẽ trở thành trung tâm ở Chương 20.
4. **Luật phải được kiểm chứng bằng test, không phải bằng niềm tin.** Câu chuyện `f64::NAN != f64::NAN` cho thấy thư viện chuẩn Rust mã hóa luật ngay vào hệ thống trait: phá luật thì mất trait, mất trait thì mất luôn `HashMap`.

> **Muốn xem bức tranh đầy đủ?** Chương này mới dạy 4 bậc đầu của thang đại số (Magma → Nửa nhóm → Vị nhóm → Nhóm). Toàn bộ **24 cấu trúc** trong đặc tả Fantasy Land — kể cả Setoid, Ord, Semigroupoid, Category, Filterable, Contravariant, Alt, Plus, Alternative, Foldable, ChainRec, Extend, Comonad, Profunctor — được liệt kê đầy đủ kèm luật và mã Rust chạy được tại **[Phụ lục A](./PHU_LUC_A_FANTASY_LAND.md)**.

### Bài tập rèn luyện tự giải:

**Bài tập 1 (Vị nhóm `CuoiCung`)**
Trong chương ta đã có `DauTien` (giữ giá trị đầu tiên khác `None`). Hãy viết vị nhóm đối ngẫu `CuoiCung<T>` giữ **giá trị cuối cùng** khác `None`, rồi giải thích vì sao nó hữu ích khi đọc cấu hình theo thứ tự "mặc định → tệp cấu hình → biến môi trường → tham số dòng lệnh".

<details>
<summary><b>Gợi ý</b></summary>

`DauTien` giữ `self` nếu `self` có giá trị. `CuoiCung` thì làm ngược lại: giữ `khac` nếu `khac` có giá trị. Phần tử đơn vị vẫn là `None`.
</details>

<details>
<summary><b>Lời giải</b></summary>

```rust
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct CuoiCung<T>(pub Option<T>);

impl<T> NuaNhom for CuoiCung<T> {
    fn ghep(self, khac: Self) -> Self {
        if khac.0.is_some() { khac } else { self }
    }
}
impl<T> ViNhom for CuoiCung<T> {
    fn don_vi() -> Self { CuoiCung(None) }
}

fn main() {
    let nguon = vec![
        CuoiCung(Some("mac_dinh")),
        CuoiCung(Some("config.toml")),
        CuoiCung(None),                // biến môi trường không đặt
        CuoiCung(Some("--cong=8080")), // tham số dòng lệnh thắng
    ];
    assert_eq!(gop_tat_ca(nguon), CuoiCung(Some("--cong=8080")));
}
```

**Vì sao hữu ích**: quy tắc "nguồn cấu hình sau ghi đè nguồn trước" chính xác là phép gộp của vị nhóm `CuoiCung`. Bạn chỉ cần xếp các nguồn theo đúng thứ tự ưu tiên rồi `gop_tat_ca` — không cần một dãy `if let Some(...) else if ...` dài dằng dặc.
</details>

**Bài tập 2 (Vị nhóm đếm tần suất)**
Viết kiểu bọc `BangDem(pub std::collections::HashMap<String, u32>)` là một vị nhóm: phép gộp cộng dồn số đếm của các khóa trùng nhau, phần tử đơn vị là bảng rỗng. Dùng nó cùng `gop_tat_ca` để đếm số lượt truy cập theo từng đường dẫn trong danh sách `BanGhiTruyCap`.

<details>
<summary><b>Gợi ý</b></summary>

Trong hàm `ghep`, duyệt bảng thứ hai và với mỗi cặp `(khoa, so)` hãy dùng `*self.0.entry(khoa).or_insert(0) += so;`. Bạn đã học `entry` API ở Chương 30 (Bảng băm) — đây là chỗ dùng lại nó.
</details>

<details>
<summary><b>Lời giải</b></summary>

```rust
use std::collections::HashMap;

#[derive(Debug, Clone, PartialEq)]
pub struct BangDem(pub HashMap<String, u32>);

impl NuaNhom for BangDem {
    fn ghep(mut self, khac: Self) -> Self {
        for (khoa, so) in khac.0 {
            *self.0.entry(khoa).or_insert(0) += so;
        }
        self
    }
}
impl ViNhom for BangDem {
    fn don_vi() -> Self { BangDem(HashMap::new()) }
}

fn dem_mot(duong_dan: &str) -> BangDem {
    let mut b = HashMap::new();
    b.insert(duong_dan.to_string(), 1);
    BangDem(b)
}

fn main() {
    let duong_dan = ["/api/a", "/api/b", "/api/a", "/api/a"];
    let ket_qua = gop_tat_ca(duong_dan.iter().map(|d| dem_mot(d)));
    assert_eq!(ket_qua.0.get("/api/a"), Some(&3));
    assert_eq!(ket_qua.0.get("/api/b"), Some(&1));
    println!("{:?}", ket_qua);
}
```

Lưu ý: thứ tự chèn khóa vào `HashMap` có thể khác nhau, nhưng **giá trị** của bảng kết quả thì luôn giống nhau — và luật kết hợp nói về giá trị, nên `BangDem` là một vị nhóm hợp lệ.
</details>

**Bài tập 3 (Tư duy: tìm phản ví dụ)**
Xét kiểu bọc `TrungBinh(pub f64)` với phép gộp `(a + b) / 2.0` và ý định dùng nó để tính giá trị trung bình. Hãy chứng minh bằng một phản ví dụ cụ thể rằng đây **không** phải nửa nhóm, rồi đề xuất cách thiết kế đúng.

<details>
<summary><b>Gợi ý</b></summary>

Thử ba số `0, 0, 12`. Tính `(a⊕b)⊕c` rồi `a⊕(b⊕c)` và so sánh. Sau đó nghĩ xem: muốn tính trung bình một cách kết hợp được thì cần mang theo *thêm thông tin gì* trong lúc gộp?
</details>

<details>
<summary><b>Lời giải</b></summary>

**Phản ví dụ**: với `a=0, b=0, c=12`:
- `(a ⊕ b) ⊕ c = ((0+0)/2 + 12)/2 = (0 + 12)/2 = 6`
- `a ⊕ (b ⊕ c) = (0 + (0+12)/2)/2 = (0 + 6)/2 = 3`

`6 ≠ 3` → luật kết hợp bị phá vỡ, nên `TrungBinh` **không** phải nửa nhóm. Nếu bạn đem nó chạy song song, kết quả sẽ thay đổi tùy vào cách chia dữ liệu — một lỗi cực kỳ khó truy vết.

**Thiết kế đúng**: đừng gộp trực tiếp giá trị trung bình. Hãy gộp cặp *(tổng, số lượng)* — vốn là một vị nhóm tích hoàn hảo — rồi mới chia ở **bước cuối cùng**:

```rust
#[derive(Debug, Clone, Copy, PartialEq)]
pub struct TichLuyTB { pub tong: f64, pub so_luong: u64 }

impl NuaNhom for TichLuyTB {
    fn ghep(self, k: Self) -> Self {
        TichLuyTB { tong: self.tong + k.tong, so_luong: self.so_luong + k.so_luong }
    }
}
impl ViNhom for TichLuyTB {
    fn don_vi() -> Self { TichLuyTB { tong: 0.0, so_luong: 0 } }
}
impl TichLuyTB {
    pub fn trung_binh(&self) -> Option<f64> {
        if self.so_luong == 0 { None } else { Some(self.tong / self.so_luong as f64) }
    }
}
```

Đây là một bài học thiết kế tổng quát: **khi một phép gộp không kết hợp được, hãy tìm xem cần mang thêm thông tin gì để nó kết hợp được, rồi chỉ "kết sổ" ở bước cuối.** Trả về `Option<f64>` cũng giải quyết luôn câu hỏi hóc búa "trung bình của danh sách rỗng là gì?" — nó là `None`, chứ không phải `0`.
</details>
