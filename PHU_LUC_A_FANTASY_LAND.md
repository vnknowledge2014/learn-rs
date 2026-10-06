# Phụ lục A — Bản đồ đầy đủ 24 Cấu trúc Đại số của Fantasy Land trong Rust

## Vì sao có phụ lục này?

[**Fantasy Land**](https://github.com/fantasyland/fantasy-land) là bản đặc tả được cộng đồng JavaScript dùng làm chuẩn chung cho các cấu trúc đại số. Giá trị của nó không nằm ở JavaScript, mà ở chỗ: nó **liệt kê đầy đủ và định nghĩa chính xác bằng luật** 24 cấu trúc mà mọi ngôn ngữ hàm đều dùng — kể cả Rust, dù Rust không gọi tên chúng ra.

Chương 18, 19 và 20 đã dạy kỹ **sáu** cấu trúc quan trọng nhất trong thực chiến (Semigroup, Monoid, Functor, Applicative, Chain/Monad, Traversable). Phụ lục này làm nốt phần còn lại và, quan trọng hơn, đặt tất cả **vào một bức tranh duy nhất**: cái nào xây trên cái nào, cái nào Rust đã có sẵn, cái nào phải tự viết.

> **Cách dùng phụ lục**: đây là tài liệu **tra cứu**, không phải bài học tuần tự. Hãy đọc Chương 18–20 trước. Khi gặp một cái tên lạ trong tài liệu tiếng Anh (`Profunctor`, `ChainRec`, `Alt`…), quay lại đây tra.

---

## 1. Bản đồ phụ thuộc: cái nào xây trên cái nào

Mỗi mũi tên đọc là *"xây dựng trên"*. Đi từ trên xuống là đi từ ít đòi hỏi tới nhiều đòi hỏi.

```
  NHÁNH 1 — ĐẠI SỐ TRÊN MỘT KIỂU              NHÁNH 2 — ĐẠI SỐ TRÊN HÀM
  ────────────────────────────────            ─────────────────────────────
        Setoid  (bằng nhau)                        Semigroupoid  (ghép mũi tên)
           │                                              │
           ▼                                              ▼
         Ord  (thứ tự)                                 Category  (+ mũi tên đơn vị)

        Magma  (gộp được)
           │  + luật kết hợp
           ▼
       Semigroup                             NHÁNH 3 — ĐẠI SỐ TRÊN NGỮ CẢNH
           │  + phần tử đơn vị                ──────────────────────────────
           ▼                                       Functor  ◄── Filterable
        Monoid                                       │
           │  + phần tử nghịch đảo                    ├──────────► Bifunctor
           ▼                                          │            Profunctor
         Group                                        │            Contravariant
                                                      ▼
                                                    Apply
                                                   ╱     ╲
                                        Applicative       Chain ──► ChainRec
                                             │  ╲          ╱
                                             │   ╲        ╱
                                             │    ▼      ▼
                                             │     Monad
                                             ▼
                                    Alt ──► Plus ──► Alternative

                                    Ord ──► Foldable

                                     Extend ──► Comonad   (đối ngẫu của Chain/Monad)
```

**Ba nhánh, ba câu hỏi khác nhau:**
- **Nhánh 1** hỏi: *"hai giá trị cùng kiểu gộp/so sánh với nhau thế nào?"*
- **Nhánh 2** hỏi: *"hai hàm nối với nhau thế nào?"*
- **Nhánh 3** hỏi: *"làm việc với giá trị nằm TRONG một chiếc hộp thế nào?"*

---

## 2. Bảng tra cứu đầy đủ 24 cấu trúc

| # | Fantasy Land | Tiếng Việt | Phép toán cốt lõi | Luật bắt buộc | Trong Rust chuẩn |
|---|---|---|---|---|---|
| 1 | **Setoid** | Kiểu có quan hệ bằng | `equals` | phản xạ · đối xứng · bắc cầu | `PartialEq` / **`Eq`** |
| 2 | **Ord** | Thứ tự | `lte` | toàn phần · phản đối xứng · bắc cầu | `PartialOrd` / **`Ord`** |
| 3 | **Semigroupoid** | Nửa phạm trù | `compose` | kết hợp | Ghép closure (tự viết, Ch14) |
| 4 | **Category** | Phạm trù | `compose` + `id` | kết hợp · đơn vị | `std::convert::identity` + ghép hàm |
| 5 | **Semigroup** | Nửa nhóm | `concat` | kết hợp | `String`/`Vec` nối, `Ordering::then` |
| 6 | **Monoid** | Vị nhóm | `concat` + `empty` | kết hợp · đơn vị | **`Default`**, **`Sum`**, **`Product`** |
| 7 | **Group** | Nhóm | `+ invert` | nghịch đảo | `Neg` cho số; `String` **không** có |
| 8 | **Filterable** | Kiểu lọc được | `filter` | phân phối · đơn vị · vắng | **`filter_map`**, `Iterator::filter` |
| 9 | **Functor** | Hàm tử | `map` | đơn vị · ghép | **`Option::map`**, `Result::map`, `Iterator::map` |
| 10 | **Contravariant** | Hàm tử nghịch biến | `contramap` | đơn vị · ghép | Không có sẵn — tự viết cho vị từ/bộ so sánh |
| 11 | **Apply** | Áp dụng | `ap` | ghép | `Option::zip` (dạng gần đúng) |
| 12 | **Applicative** | Hàm tử áp dụng | `ap` + `of` | đồng nhất · đồng cấu · hoán vị | `Some`/`Ok` đóng vai `of` |
| 13 | **Alt** | Lựa chọn | `alt` | kết hợp · phân phối | **`Option::or`**, `Result::or` |
| 14 | **Plus** | Lựa chọn có rỗng | `alt` + `zero` | đơn vị · triệt tiêu | `None`, `Vec::new()` |
| 15 | **Alternative** | Applicative + Plus | — | phân phối · triệt tiêu | `Option` thỏa mãn cả hai |
| 16 | **Foldable** | Kiểu gấp được | `reduce` | tương đương với gấp danh sách | **`Iterator::fold`**, `IntoIterator` |
| 17 | **Traversable** | Kiểu duyệt được | `traverse` | tự nhiên · đơn vị · ghép | **`collect::<Result<Vec<_>,E>>()`**, `transpose` |
| 18 | **Chain** | Phép buộc | `chain` | kết hợp | **`and_then`**, `flat_map` |
| 19 | **ChainRec** | Buộc đệ quy | `chainRec` | tương đương · **ngăn xếp không phình** | `loop` + `ControlFlow` |
| 20 | **Monad** | Đơn nguyên | Applicative + Chain | đơn vị trái · đơn vị phải | `Option`, `Result` (qua `and_then`), `Vec`/`Iterator` (qua `flat_map`) |
| 21 | **Extend** | Mở rộng | `extend` | kết hợp | Không có sẵn |
| 22 | **Comonad** | Đối đơn nguyên | `extend` + `extract` | đơn vị trái · đơn vị phải | Không có sẵn |
| 23 | **Bifunctor** | Hàm tử hai ngôi | `bimap` | đơn vị · ghép | **`Result::map` + `map_err`** |
| 24 | **Profunctor** | Profunctor | `promap` | đơn vị · ghép | Không có sẵn — tự viết cho `Fn(A) -> B` |

**Đọc bảng này thế nào**: cột cuối cho thấy Rust **đã có sẵn 18/24** dưới dạng kiểu, trait hoặc phương thức thư viện chuẩn — bạn dùng chúng hằng ngày mà không biết tên gọi chung. Sáu cái còn lại — Semigroupoid và Category (thư viện chuẩn không có hàm ghép `compose`), Contravariant, Extend, Comonad và Profunctor — phải tự viết, và mã trong phụ lục này làm đúng việc đó.

---

## 3. Giải nghĩa những cấu trúc chưa xuất hiện ở Chương 18–20

Sáu cấu trúc cốt lõi đã học kỹ ở Chương 18, 19, 20. Mục này giải thích những cái còn lại.

### 3.1. Setoid và Ord — "bằng nhau" cũng phải tuân luật

Bạn đã gặp câu chuyện `f64::NAN != f64::NAN` ở Chương 18. Đó chính là **Setoid**: một kiểu không chỉ cần *có* phép `==`, mà phép `==` đó phải là **quan hệ tương đương** (phản xạ, đối xứng, bắc cầu).

**Ord** thêm một tầng: quan hệ thứ tự phải **toàn phần** (hai phần tử bất kỳ luôn so sánh được) và **phản đối xứng** (`a ≤ b` và `b ≤ a` thì `a = b`).

Đây chính là lý do Rust tách đôi:

| | Chỉ có phép toán | Có phép toán **và** tuân luật |
|---|---|---|
| Bằng nhau | `PartialEq` | **`Eq`** ← đây là Setoid |
| Thứ tự | `PartialOrd` | **`Ord`** ← đây là Ord của Fantasy Land |

Và hệ quả rất thật: `HashMap` đòi khóa phải có `Eq + Hash`; `BTreeMap` đòi khóa phải có `Ord`. Vì `f64` phá luật, nó không dùng làm khóa được.

### 3.2. Semigroupoid và Category — đại số của phép ghép hàm

Ở Chương 14 bạn đã viết hàm `compose` và kiểm chứng hai luật *kết hợp* và *đơn vị*. Hai luật đó chính là định nghĩa của **Category**:

- **Semigroupoid** = có phép ghép hai "mũi tên" khớp đầu nối đuôi (`A → B` ghép `B → C` thành `A → C`), tuân luật kết hợp.
- **Category** = Semigroupoid **cộng thêm** một "mũi tên đơn vị" `id: A → A` cho mọi kiểu.

Cái tên nghe ghê gớm nhưng nội dung thì bạn đã dùng từ Chương 14. Điều đáng nhớ: **hàm trong Rust tạo thành một phạm trù**, và mọi thứ ở nhánh 3 (Functor, Monad…) đều được định nghĩa *dựa trên* phạm trù này.

### 3.3. Contravariant — hàm tử "đi ngược"

`Functor` cho phép đắp thêm việc vào **đầu ra**. `Contravariant` cho phép đắp thêm vào **đầu vào**.

Ví dụ kinh điển là **vị từ** (`predicate`): bạn có `Predicate<i64>` biết kiểm tra "số này có chẵn không". Bạn muốn có `Predicate<String>` kiểm tra "chuỗi này có độ dài chẵn không". Bạn không thể `map` — vì `Predicate` *tiêu thụ* giá trị chứ không *sản xuất* ra nó. Thứ bạn cần là hàm đi **ngược chiều**: `String -> i64`.

```
Functor      :  F<A>  +  (A -> B)  =  F<B>       ← hàm cùng chiều
Contravariant:  F<A>  +  (B -> A)  =  F<B>       ← hàm NGƯỢC chiều
```

Trong Rust bạn gặp mẫu này ở mọi hàm so sánh và mọi bộ lọc tùy biến: `sort_by_key(|x| x.age)` chính là contramap trên bộ so sánh.

### 3.4. Profunctor — vừa nghịch biến vừa hiệp biến

Một hàm `A -> B` có **hai chỗ** để đắp thêm: đầu vào (nghịch biến) và đầu ra (hiệp biến). Kiểu nào đắp được cả hai gọi là **Profunctor**, với phép `promap`:

```
promap :  (C -> A)  +  Func<A, B>  +  (B -> D)  =  Func<C, D>
```

Đây là nền tảng lý thuyết của **Lens/Optics** — hệ thống truy cập và cập nhật dữ liệu lồng sâu theo kiểu hàm. Trong thực chiến Rust, bạn thường gặp nó dưới dạng "bộ chuyển đổi hai đầu": nhận dữ liệu ở định dạng ngoài, gọi lõi nghiệp vụ, rồi chuyển kết quả về định dạng ngoài — chính là **cổng biên hệ thống** ở Chương 20.

### 3.5. Filterable — lọc và biến đổi cùng lúc

Fantasy Land tách riêng `Filterable` vì không phải hàm tử nào cũng lọc được: bạn `map` được một cặp `(A, B)` nhưng không thể "lọc bớt" nó — cặp luôn có đúng hai phần.

Trong Rust, `Filterable` chính là **`filter_map`** mà Chương 16 đã dạy. Luật quan trọng nhất của nó là *phân phối*:

```
xs.filter_map(f).filter_map(g)  ==  xs.filter_map(|x| f(x).and_then(g))
```

Chính luật này cho phép trình biên dịch gộp hai vòng lọc thành một.

### 3.6. Alt, Plus, Alternative — đại số của "phương án dự phòng"

Ba tầng, mỗi tầng thêm một đòi hỏi:

| | Phép toán | Ý nghĩa | Rust |
|---|---|---|---|
| **Alt** | `alt(a, b)` | "lấy a, không có thì lấy b" | `Option::or`, `Result::or` |
| **Plus** | `+ zero()` | có một giá trị "rỗng" làm đơn vị | `None`, `Vec::new()` |
| **Alternative** | Plus + Applicative | vừa gộp được vừa nhấc giá trị vào được | `Option` |

Nhìn kỹ sẽ thấy: **Plus chính là Monoid, nhưng ở tầng ngữ cảnh** thay vì tầng giá trị. Đây là mẫu dùng hằng ngày để đọc cấu hình theo thứ tự ưu tiên:

```rust
let port = from_cli.or(from_env).or(from_config_file).unwrap_or(8080);
```

### 3.7. ChainRec — câu trả lời cho vấn đề tràn ngăn xếp

Đây là cấu trúc **thực dụng nhất trong nhóm chưa được dạy**, và nó liên quan trực tiếp tới cảnh báo ở Chương 16 rằng **Rust không bảo đảm tối ưu hóa lời gọi đuôi**.

Vấn đề: một vòng lặp đơn nguyên viết bằng đệ quy sẽ làm phình ngăn xếp:

```rust
// ❌ Đệ quy đơn nguyên — 1.000.000 vòng có thể TRÀN NGĂN XẾP (chắc chắn ở bản debug)
fn count_down(n: u32) -> Option<u32> {
    if n == 0 { Some(0) } else { count_down(n - 1) }
}
```

`ChainRec` giải quyết bằng cách bắt hàm bước trả về một **thẻ báo hiệu** thay vì tự gọi lại chính nó:

```rust
enum Step<A, B> { Continue(A), Finished(B) }
```

Người điều phối nhận thẻ đó và **lặp bằng vòng lặp**, nên ngăn xếp giữ nguyên độ sâu bất kể bao nhiêu vòng. Đây là kỹ thuật *trampoline* — và trong Rust nó tương ứng với `loop` kết hợp `std::ops::ControlFlow`. Bài kiểm thử trong mã dưới đây chạy **1.000.000 vòng** để chứng minh điều đó.

### 3.8. Extend và Comonad — đối ngẫu của Chain và Monad

Đây là cặp khái niệm đối xứng gương với Monad. Hãy đặt cạnh nhau:

| | Monad | Comonad |
|---|---|---|
| Nhấc vào / lấy ra | `of : A -> F<A>` | `extract : F<A> -> A` |
| Phép nối | `chain : F<A> -> (A -> F<B>) -> F<B>` | `extend : F<A> -> (F<A> -> B) -> F<B>` |
| Câu hỏi | *"từ một giá trị, tạo ra ngữ cảnh mới"* | *"từ toàn bộ ngữ cảnh, rút ra một giá trị"* |
| Dùng cho | tác dụng phụ, thất bại, bất đồng bộ | dữ liệu **luôn có** giá trị, phụ thuộc lân cận |

Ví dụ kinh điển của Comonad là **con trỏ trượt (Zipper)**: một dãy có "tiêu điểm" luôn tồn tại. `extract` lấy tiêu điểm; `extend` tính lại giá trị cho **mọi vị trí**, mỗi vị trí được nhìn thấy toàn bộ ngữ cảnh xung quanh mình.

Đây chính là mô hình tính toán của: bộ lọc ảnh (mỗi điểm ảnh cần biết các điểm lân cận), trung bình trượt trên chuỗi thời gian, và trò chơi Life của Conway. Mã dưới đây cài đặt `Window<T>` và dùng nó tính tổng ba ô lân cận cho mỗi vị trí — chỉ bằng một lời gọi `extend`.

---

## 4. Mã nguồn hoàn chỉnh: 24 cấu trúc, chạy được, có kiểm chứng luật

Chương trình dưới đây cài đặt **toàn bộ 24 cấu trúc** bằng Rust ổn định, không dùng thư viện ngoài, kèm **21 bài kiểm thử kiểm chứng luật** của từng cấu trúc.

Chạy thử:

```bash
cd code
cargo run  -p phu_luc_a      # xem cả 24 cấu trúc hoạt động
cargo test -p phu_luc_a      # kiểm chứng toàn bộ luật
```

```rust
use std::cmp::Ordering;
use std::fmt::Debug;

// ══════════════════════════════════════════════════════════════════════════
// NHÓM A — ĐẠI SỐ TRÊN MỘT KIỂU DỮ LIỆU (không cần HKT)
// ══════════════════════════════════════════════════════════════════════════

/// 1. SETOID — kiểu có quan hệ "bằng nhau" tuân luật tương đương.
pub trait Setoid {
    fn equals(&self, other: &Self) -> bool;
}

/// 2. ORD — Setoid có thêm quan hệ thứ tự toàn phần.
pub trait Ord: Setoid {
    fn compare(&self, other: &Self) -> Ordering;
    fn less_or_equal(&self, other: &Self) -> bool {
        self.compare(other) != Ordering::Greater
    }
}

/// 5. SEMIGROUP — phép gộp hai thành một, tuân luật kết hợp.
pub trait Semigroup {
    fn compose(self, other: Self) -> Self;
}

/// 6. MONOID — nửa nhóm có phần tử đơn vị.
pub trait Monoid: Semigroup + Sized {
    fn empty() -> Self;
}

/// 7. GROUP — vị nhóm có phần tử nghịch đảo.
pub trait Group: Monoid {
    fn invert(self) -> Self;
}

// ---- Instance: Sum (vị nhóm cộng) là một NHÓM đầy đủ ----
#[derive(Debug, Clone, Copy, PartialEq)]
pub struct Sum(pub i64);
impl Setoid for Sum {
    fn equals(&self, other: &Self) -> bool {
        self.0 == other.0
    }
}
impl Ord for Sum {
    fn compare(&self, other: &Self) -> Ordering {
        self.0.cmp(&other.0)
    }
}
impl Semigroup for Sum {
    fn compose(self, other: Self) -> Self {
        Sum(self.0.wrapping_add(other.0))
    }
}
impl Monoid for Sum {
    fn empty() -> Self {
        Sum(0)
    }
}
impl Group for Sum {
    fn invert(self) -> Self {
        Sum(-self.0)
    }
}

// ---- Instance: Mod4 — nhóm cộng modulo 4 (hữu hạn, dễ kiểm chứng vét cạn) ----
#[derive(Debug, Clone, Copy, PartialEq)]
pub struct Mod4(pub u8);
impl Setoid for Mod4 {
    fn equals(&self, other: &Self) -> bool {
        self.0 % 4 == other.0 % 4
    }
}
impl Semigroup for Mod4 {
    fn compose(self, other: Self) -> Self {
        Mod4((self.0 + other.0) % 4)
    }
}
impl Monoid for Mod4 {
    fn empty() -> Self {
        Mod4(0)
    }
}
impl Group for Mod4 {
    fn invert(self) -> Self {
        Mod4((4 - self.0 % 4) % 4)
    }
}

// ---- Instance: String là nửa nhóm + vị nhóm, nhưng KHÔNG phải nhóm ----
impl Semigroup for String {
    fn compose(self, other: Self) -> Self {
        self + &other
    }
}
impl Monoid for String {
    fn empty() -> Self {
        String::new()
    }
}

/// Gộp vạn năng cho mọi vị nhóm.
pub fn combine_all<M: Monoid>(list: impl IntoIterator<Item = M>) -> M {
    list.into_iter().fold(M::empty(), |a, x| a.compose(x))
}

// ══════════════════════════════════════════════════════════════════════════
// NHÓM B — ĐẠI SỐ TRÊN HÀM (Semigroupoid, Category, Profunctor)
// ══════════════════════════════════════════════════════════════════════════

/// Bọc một hàm thành giá trị để có thể cài trait lên nó (vượt quy tắc mồ côi).
pub struct Func<A, B>(Box<dyn Fn(A) -> B>);

impl<A, B> Func<A, B> {
    pub fn new(f: impl Fn(A) -> B + 'static) -> Self {
        Func(Box::new(f))
    }
    pub fn run(&self, a: A) -> B {
        (self.0)(a)
    }
}

/// 3. SEMIGROUPOID — có phép ghép hai "mũi tên" khớp đầu nối đuôi.
impl<A: 'static, B: 'static> Func<A, B> {
    pub fn compose_with<C: 'static>(self, next: Func<B, C>) -> Func<A, C> {
        Func::new(move |a| next.run(self.run(a)))
    }
}

/// 4. CATEGORY — Semigroupoid có thêm "mũi tên đơn vị".
pub fn identity<A>() -> Func<A, A> {
    Func::new(|a| a)
}

/// 24. PROFUNCTOR — nghịch biến ở đầu vào, hiệp biến ở đầu ra.
impl<A: 'static, B: 'static> Func<A, B> {
    pub fn promap<C: 'static, D: 'static>(
        self,
        prev: impl Fn(C) -> A + 'static, // NGHỊCH biến: đắp thêm vào ĐẦU VÀO
        next: impl Fn(B) -> D + 'static, // HIỆP biến : đắp thêm vào ĐẦU RA
    ) -> Func<C, D> {
        Func::new(move |c| next(self.run(prev(c))))
    }
}

/// 10. CONTRAVARIANT — chỉ có đầu vào để đắp thêm. Ví dụ kinh điển: vị từ.
pub struct Predicate<A>(Box<dyn Fn(&A) -> bool>);

impl<A: 'static> Predicate<A> {
    pub fn new(f: impl Fn(&A) -> bool + 'static) -> Self {
        Predicate(Box::new(f))
    }
    pub fn check(&self, a: &A) -> bool {
        (self.0)(a)
    }

    /// contramap: từ vị từ trên A, tạo ra vị từ trên B nhờ hàm B -> A.
    pub fn contramap<B: 'static>(self, f: impl Fn(&B) -> A + 'static) -> Predicate<B> {
        Predicate::new(move |b| self.check(&f(b)))
    }
}

// ══════════════════════════════════════════════════════════════════════════
// NHÓM C — ĐẠI SỐ TRÊN NGỮ CẢNH (cần HKT, ta mô phỏng bằng kiểu liên kết)
// ══════════════════════════════════════════════════════════════════════════

pub trait HKT<U> {
    type Current;
    type Target;
}
impl<T, U> HKT<U> for Option<T> {
    type Current = T;
    type Target = Option<U>;
}
impl<T, U> HKT<U> for Vec<T> {
    type Current = T;
    type Target = Vec<U>;
}
impl<T, U, E> HKT<U> for Result<T, E> {
    type Current = T;
    type Target = Result<U, E>;
}

/// 9. FUNCTOR
pub trait Functor<U>: HKT<U> {
    fn fmap<F: FnMut(Self::Current) -> U>(self, f: F) -> Self::Target;
}
impl<T, U> Functor<U> for Option<T> {
    fn fmap<F: FnMut(T) -> U>(self, f: F) -> Option<U> {
        self.map(f)
    }
}
impl<T, U> Functor<U> for Vec<T> {
    fn fmap<F: FnMut(T) -> U>(self, f: F) -> Vec<U> {
        self.into_iter().map(f).collect()
    }
}
impl<T, U, E> Functor<U> for Result<T, E> {
    fn fmap<F: FnMut(T) -> U>(self, f: F) -> Result<U, E> {
        self.map(f)
    }
}

/// 8. FILTERABLE — lọc và biến đổi cùng lúc bằng A -> Option<B>.
pub trait Filterable<U>: HKT<U> {
    fn filter_map<F: FnMut(Self::Current) -> Option<U>>(self, f: F) -> Self::Target;
}
impl<T, U> Filterable<U> for Vec<T> {
    fn filter_map<F: FnMut(T) -> Option<U>>(self, f: F) -> Vec<U> {
        self.into_iter().filter_map(f).collect()
    }
}
impl<T, U> Filterable<U> for Option<T> {
    fn filter_map<F: FnMut(T) -> Option<U>>(self, f: F) -> Option<U> {
        self.and_then(f)
    }
}

/// 23. BIFUNCTOR — hai chân, đắp thêm được vào cả hai.
pub trait Bifunctor<C, D> {
    type Output;
    fn bimap(
        self,
        f: impl FnOnce(Self::Left) -> C,
        g: impl FnOnce(Self::Right) -> D,
    ) -> Self::Output;
    type Left;
    type Right;
}
impl<A, B, C, D> Bifunctor<C, D> for Result<A, B> {
    type Left = A;
    type Right = B;
    type Output = Result<C, D>;
    fn bimap(self, f: impl FnOnce(A) -> C, g: impl FnOnce(B) -> D) -> Result<C, D> {
        match self {
            Ok(a) => Ok(f(a)),
            Err(b) => Err(g(b)),
        }
    }
}
impl<A, B, C, D> Bifunctor<C, D> for (A, B) {
    type Left = A;
    type Right = B;
    type Output = (C, D);
    fn bimap(self, f: impl FnOnce(A) -> C, g: impl FnOnce(B) -> D) -> (C, D) {
        (f(self.0), g(self.1))
    }
}

// ---- 11. APPLY & 12. APPLICATIVE (bản cụ thể cho Option / Result / Vec) ----

/// APPLY: ngữ cảnh chứa HÀM áp vào ngữ cảnh chứa GIÁ TRỊ.
pub fn ap_option<A, B>(func: Option<Box<dyn Fn(A) -> B>>, value: Option<A>) -> Option<B> {
    match (func, value) {
        (Some(f), Some(a)) => Some(f(a)),
        _ => None,
    }
}
pub fn ap_result<A, B, E>(
    func: Result<Box<dyn Fn(A) -> B>, E>,
    value: Result<A, E>,
) -> Result<B, E> {
    match (func, value) {
        (Ok(f), Ok(a)) => Ok(f(a)),
        (Err(e), _) => Err(e),
        (_, Err(e)) => Err(e),
    }
}
/// APPLICATIVE: `of` — nhấc một giá trị trần vào ngữ cảnh.
pub fn of_option<A>(a: A) -> Option<A> {
    Some(a)
}
pub fn of_result<A, E>(a: A) -> Result<A, E> {
    Ok(a)
}
pub fn of_vec<A>(a: A) -> Vec<A> {
    vec![a]
}

/// APPLICATIVE tích lũy lỗi — biến thể `Validation` (không phải Monad!).
#[derive(Debug, Clone, PartialEq)]
pub enum Validation<T> {
    Valid(T),
    Invalid(Vec<String>),
}
impl<T> Validation<T> {
    pub fn fmap<U>(self, f: impl FnOnce(T) -> U) -> Validation<U> {
        match self {
            Validation::Valid(x) => Validation::Valid(f(x)),
            Validation::Invalid(e) => Validation::Invalid(e),
        }
    }
}
pub fn ap_validation<A, B>(
    func: Validation<Box<dyn Fn(A) -> B>>,
    value: Validation<A>,
) -> Validation<B> {
    match (func, value) {
        (Validation::Valid(f), Validation::Valid(a)) => Validation::Valid(f(a)),
        (Validation::Invalid(mut e1), Validation::Invalid(e2)) => {
            e1.extend(e2);
            Validation::Invalid(e1)
        }
        (Validation::Invalid(e), _) => Validation::Invalid(e),
        (_, Validation::Invalid(e)) => Validation::Invalid(e),
    }
}

// ---- 13. ALT · 14. PLUS · 15. ALTERNATIVE ----

/// ALT — "hoặc cái này hoặc cái kia", giữ nguyên kiểu.
pub trait Alt {
    fn alt(self, other: Self) -> Self;
}
/// PLUS — Alt có thêm phần tử "rỗng".
pub trait Plus: Alt + Sized {
    fn zero() -> Self;
}
impl<T> Alt for Option<T> {
    fn alt(self, other: Self) -> Self {
        self.or(other)
    }
}
impl<T> Plus for Option<T> {
    fn zero() -> Self {
        None
    }
}
impl<T> Alt for Vec<T> {
    fn alt(mut self, mut other: Self) -> Self {
        self.append(&mut other);
        self
    }
}
impl<T> Plus for Vec<T> {
    fn zero() -> Self {
        Vec::new()
    }
}
/// ALTERNATIVE = Applicative + Plus. Trong Rust: đánh dấu bằng siêu trait.
pub trait Alternative: Plus {}
impl<T> Alternative for Option<T> {}
impl<T> Alternative for Vec<T> {}

// ---- 16. FOLDABLE · 17. TRAVERSABLE ----

/// FOLDABLE — gấp một cấu trúc về một giá trị.
pub trait Foldable {
    type Item;
    fn fold<B>(self, init: B, f: impl FnMut(B, Self::Item) -> B) -> B;
}
#[derive(Debug, Clone, PartialEq)]
pub enum Tree<T> {
    Leaf,
    Node(Box<Tree<T>>, T, Box<Tree<T>>),
}
impl<T> Foldable for Tree<T> {
    type Item = T;
    fn fold<B>(self, init: B, mut f: impl FnMut(B, T) -> B) -> B {
        fn walk<T, B>(c: Tree<T>, acc: B, f: &mut impl FnMut(B, T) -> B) -> B {
            match c {
                Tree::Leaf => acc,
                Tree::Node(t, v, p) => {
                    let acc = walk(*t, acc, f);
                    let acc = f(acc, v);
                    walk(*p, acc, f)
                }
            }
        }
        walk(self, init, &mut f)
    }
}

/// TRAVERSABLE — đảo ngữ cảnh từ trong ra ngoài.
pub fn traverse_vec_result<A, B, E>(
    list: Vec<A>,
    f: impl FnMut(A) -> Result<B, E>,
) -> Result<Vec<B>, E> {
    list.into_iter().map(f).collect()
}
pub fn traverse_vec_option<A, B>(list: Vec<A>, f: impl FnMut(A) -> Option<B>) -> Option<Vec<B>> {
    list.into_iter().map(f).collect()
}

// ---- 18. CHAIN · 19. CHAINREC · 20. MONAD ----

/// CHAIN — phép `bind`: A -> F<B>.
pub trait Chain<U>: HKT<U> {
    fn chain<F: FnMut(Self::Current) -> Self::Target>(self, f: F) -> Self::Target;
}
impl<T, U> Chain<U> for Option<T> {
    fn chain<F: FnMut(T) -> Option<U>>(self, f: F) -> Option<U> {
        self.and_then(f)
    }
}
impl<T, U, E> Chain<U> for Result<T, E> {
    fn chain<F: FnMut(T) -> Result<U, E>>(self, f: F) -> Result<U, E> {
        self.and_then(f)
    }
}
impl<T, U> Chain<U> for Vec<T> {
    fn chain<F: FnMut(T) -> Vec<U>>(self, f: F) -> Vec<U> {
        self.into_iter().flat_map(f).collect()
    }
}

/// MONAD = Applicative + Chain. Trong Rust: siêu trait đánh dấu.
pub trait Monad<U>: Chain<U> + Functor<U> {}
impl<T, U> Monad<U> for Option<T> {}
impl<T, U, E> Monad<U> for Result<T, E> {}
impl<T, U> Monad<U> for Vec<T> {}

/// CHAINREC — lặp đơn nguyên với NGĂN XẾP KHÔNG PHÌNH TO.
/// Đây là câu trả lời của Fantasy Land cho việc Rust không tối ưu hóa lời gọi đuôi.
#[derive(Debug, Clone, PartialEq)]
pub enum Step<A, B> {
    Continue(A),
    Finished(B),
}
pub fn chain_rec_option<A, B>(
    initial: A,
    mut step: impl FnMut(A) -> Option<Step<A, B>>,
) -> Option<B> {
    let mut current = initial;
    loop {
        match step(current)? {
            Step::Continue(a) => current = a, // vòng lặp, KHÔNG đệ quy
            Step::Finished(b) => return Some(b),
        }
    }
}

// ---- 21. EXTEND · 22. COMONAD ----

/// EXTEND — đối ngẫu của Chain: F<A> -> (F<A> -> B) -> F<B>.
pub trait Extend<U>: HKT<U> + Sized {
    fn extend<F: FnMut(&Self) -> U>(self, f: F) -> Self::Target;
}
/// COMONAD — Extend có thêm `extract`: F<A> -> A (đối ngẫu của `of`).
/// 22a. `extract` được tách riêng, đúng như đặc tả Fantasy Land: nó KHÔNG phụ
/// thuộc kiểu đích U, nên không được đặt trong một trait generic theo U.
pub trait Extract {
    type Inner;
    fn extract(&self) -> &Self::Inner;
}

/// 22b. COMONAD = Extend + extract (đối ngẫu của Monad = Chain + of).
pub trait Comonad<U>: Extend<U> + Extract {}

/// Ví dụ kinh điển: con trỏ trượt trên dãy (Zipper) — luôn có "tiêu điểm".
#[derive(Debug, Clone, PartialEq)]
pub struct Window<T> {
    pub prev: Vec<T>,
    pub focus: T,
    pub next: Vec<T>,
}
impl<T, U> HKT<U> for Window<T> {
    type Current = T;
    type Target = Window<U>;
}
impl<T: Clone, U> Extend<U> for Window<T> {
    fn extend<F: FnMut(&Self) -> U>(self, mut f: F) -> Window<U> {
        let n = self.prev.len();
        let all: Vec<T> = self
            .prev
            .iter()
            .cloned()
            .chain(std::iter::once(self.focus.clone()))
            .chain(self.next.iter().cloned())
            .collect();
        let at = |i: usize| Window {
            prev: all[..i].to_vec(),
            focus: all[i].clone(),
            next: all[i + 1..].to_vec(),
        };
        Window {
            prev: (0..n).map(|i| f(&at(i))).collect(),
            focus: f(&at(n)),
            next: ((n + 1)..all.len()).map(|i| f(&at(i))).collect(),
        }
    }
}
impl<T> Extract for Window<T> {
    type Inner = T;
    fn extract(&self) -> &T {
        &self.focus
    }
}
impl<T: Clone, U> Comonad<U> for Window<T> {}

// ══════════════════════════════════════════════════════════════════════════
// CHƯƠNG TRÌNH DEMO
// ══════════════════════════════════════════════════════════════════════════

fn main() {
    println!("═══════════════════════════════════════════════════════════════");
    println!("   24 CẤU TRÚC ĐẠI SỐ FANTASY LAND — HIỆN THỰC HÓA BẰNG RUST   ");
    println!("═══════════════════════════════════════════════════════════════");

    println!("\n── NHÓM A: ĐẠI SỐ TRÊN MỘT KIỂU ──");
    println!(
        " 1. Setoid     Sum(5).equals(&Sum(5))     = {}",
        Sum(5).equals(&Sum(5))
    );
    println!(
        " 2. Ord        Sum(3).compare(&Sum(9))    = {:?}",
        Sum(3).compare(&Sum(9))
    );
    println!(
        " 5. Semigroup  Sum(3).compose(Sum(4))     = {:?}",
        Sum(3).compose(Sum(4))
    );
    println!(
        " 6. Monoid     Sum::empty()               = {:?}",
        Sum::empty()
    );
    println!(
        " 7. Group      Sum(7).invert()            = {:?}",
        Sum(7).invert()
    );
    println!(
        "               Mod4(3).compose(invert)    = {:?}",
        Mod4(3).compose(Mod4(3).invert())
    );
    println!("    (String là Monoid nhưng KHÔNG phải Group: không có \"chuỗi âm\")");

    println!("\n── NHÓM B: ĐẠI SỐ TRÊN HÀM ──");
    let times2 = Func::new(|x: i64| x * 2);
    let plus3 = Func::new(|x: i64| x + 3);
    let composed = times2.compose_with(plus3);
    println!(
        " 3. Semigroupoid  (nhân 2 rồi cộng 3)(10)  = {}",
        composed.run(10)
    );
    println!(
        " 4. Category      identity(42)             = {}",
        identity::<i64>().run(42)
    );
    let length = Func::new(|s: String| s.chars().count());
    let pro = length.promap(|n: i64| format!("số {}", n), |u: usize| u * 100);
    println!(
        "24. Profunctor    promap(i64 -> usize)(7)  = {}",
        pro.run(7)
    );
    let is_even = Predicate::new(|n: &i64| n % 2 == 0);
    let even_length = is_even.contramap(|s: &String| s.chars().count() as i64);
    println!(
        "10. Contravariant \"Rust\" có độ dài chẵn?   = {}",
        even_length.check(&"Rust".to_string())
    );

    println!("\n── NHÓM C: ĐẠI SỐ TRÊN NGỮ CẢNH ──");
    println!(
        " 9. Functor      Some(5).fmap(+1)          = {:?}",
        Some(5i32).fmap(|x| x + 1)
    );
    println!(
        " 8. Filterable   lọc số phân tích được     = {:?}",
        vec!["1", "x", "3"].filter_map(|s: &str| s.parse::<i32>().ok())
    );
    println!(
        "23. Bifunctor    Err(2).bimap(+1, *10)     = {:?}",
        (Err(2i32) as Result<i32, i32>).bimap(|a| a + 1, |b| b * 10)
    );
    let f: Option<Box<dyn Fn(i32) -> i32>> = Some(Box::new(|x| x * 3));
    println!(
        "11. Apply        ap(Some(*3), Some(7))     = {:?}",
        ap_option(f, Some(7))
    );
    println!(
        "12. Applicative  of(9)                     = {:?}",
        of_option(9)
    );
    println!(
        "13. Alt          None.alt(Some(2))         = {:?}",
        None.alt(Some(2))
    );
    println!(
        "14. Plus         Option::zero()            = {:?}",
        <Option<i32> as Plus>::zero()
    );
    println!("15. Alternative  = Applicative + Plus (siêu trait đánh dấu)");

    let tree = Tree::Node(
        Box::new(Tree::Node(
            Box::new(Tree::Leaf),
            20i64,
            Box::new(Tree::Leaf),
        )),
        50,
        Box::new(Tree::Node(Box::new(Tree::Leaf), 70, Box::new(Tree::Leaf))),
    );
    println!(
        "16. Foldable     gấp cây [20,50,70] -> tổng = {}",
        tree.clone().fold(0i64, |a, x| a + x)
    );
    println!(
        "17. Traversable  Vec<Result> -> Result<Vec> = {:?}",
        traverse_vec_result(vec!["1", "2"], |s: &str| s.parse::<i32>())
    );
    println!(
        "18. Chain        Some(4).chain(|x| Some(x*5)) = {:?}",
        Some(4i32).chain(|x| Some(x * 5))
    );
    println!("20. Monad        = Applicative + Chain (siêu trait đánh dấu)");

    let power = chain_rec_option((1u64, 20u32), |(acc, remaining)| {
        Some(if remaining == 0 {
            Step::Finished(acc)
        } else {
            Step::Continue((acc * 2, remaining - 1))
        })
    });
    println!("19. ChainRec     2^20 bằng vòng lặp        = {:?}", power);

    let window = Window {
        prev: vec![1i64, 2],
        focus: 3,
        next: vec![4, 5],
    };
    println!(
        "22. Comonad      trích xuất tiêu điểm      = {}",
        window.extract()
    );
    let neighbor_sums = window.clone().extend(|w: &Window<i64>| {
        w.prev.last().copied().unwrap_or(0) + w.focus + w.next.first().copied().unwrap_or(0)
    });
    println!(
        "21. Extend       tổng 3 ô lân cận mỗi vị trí= {:?}",
        [
            neighbor_sums.prev.clone(),
            vec![neighbor_sums.focus],
            neighbor_sums.next.clone()
        ]
        .concat()
    );

    println!("\n═══════════════════════════════════════════════════════════════");
    println!("   24/24 CẤU TRÚC — MỖI CÁI MỘT ĐỊNH NGHĨA, MỘT LUẬT, MỘT MÃ    ");
    println!("═══════════════════════════════════════════════════════════════");
}

// ══════════════════════════════════════════════════════════════════════════
// KIỂM CHỨNG LUẬT — MỖI ĐẠI SỐ MỘT BÀI TEST
// ══════════════════════════════════════════════════════════════════════════

#[cfg(test)]
mod laws {
    use super::*;

    fn mod4_samples() -> Vec<Mod4> {
        (0..4).map(Mod4).collect()
    }

    #[test] // 1. SETOID: phản xạ, đối xứng, bắc cầu
    fn setoid() {
        for a in mod4_samples() {
            assert!(a.equals(&a));
        } // phản xạ
        for a in mod4_samples() {
            for b in mod4_samples() {
                assert_eq!(a.equals(&b), b.equals(&a));
            }
        } // đối xứng
        for a in mod4_samples() {
            for b in mod4_samples() {
                for c in mod4_samples() {
                    if a.equals(&b) && b.equals(&c) {
                        assert!(a.equals(&c));
                    } // bắc cầu
                }
            }
        }
    }

    #[test] // 2. ORD: toàn phần, phản đối xứng, bắc cầu
    fn ord() {
        let m: Vec<Sum> = (-3..4).map(Sum).collect();
        for a in &m {
            for b in &m {
                assert!(a.less_or_equal(b) || b.less_or_equal(a)); // toàn phần
                if a.less_or_equal(b) && b.less_or_equal(a) {
                    assert!(a.equals(b));
                }
            }
        }
    }

    #[test] // 3. SEMIGROUPOID: (f ∘ g) ∘ h == f ∘ (g ∘ h)
    fn semigroupoid_associative() {
        for x in [-5i64, 0, 7, 100] {
            let left = Func::new(|a: i64| a + 1)
                .compose_with(Func::new(|a: i64| a * 2))
                .compose_with(Func::new(|a: i64| a - 3));
            let right = Func::new(|a: i64| a + 1)
                .compose_with(Func::new(|a: i64| a * 2).compose_with(Func::new(|a: i64| a - 3)));
            assert_eq!(left.run(x), right.run(x));
        }
    }

    #[test] // 4. CATEGORY: id ∘ f == f == f ∘ id
    fn category_identity() {
        for x in [-5i64, 0, 42] {
            let f = |a: i64| a * 3 + 1;
            assert_eq!(identity::<i64>().compose_with(Func::new(f)).run(x), f(x));
            assert_eq!(Func::new(f).compose_with(identity::<i64>()).run(x), f(x));
        }
    }

    #[test] // 5. SEMIGROUP: (a ⊕ b) ⊕ c == a ⊕ (b ⊕ c)
    fn semigroup_associative() {
        for a in mod4_samples() {
            for b in mod4_samples() {
                for c in mod4_samples() {
                    assert!(a.compose(b).compose(c).equals(&a.compose(b.compose(c))));
                }
            }
        }
        let s = ["a".to_string(), "bc".to_string(), "d".to_string()];
        assert_eq!(
            s[0].clone().compose(s[1].clone()).compose(s[2].clone()),
            s[0].clone().compose(s[1].clone().compose(s[2].clone()))
        );
    }

    #[test] // 6. MONOID: e ⊕ a == a == a ⊕ e
    fn monoid_identity() {
        for a in mod4_samples() {
            assert!(Mod4::empty().compose(a).equals(&a));
            assert!(a.compose(Mod4::empty()).equals(&a));
        }
        let zero: Vec<Sum> = Vec::new();
        assert_eq!(combine_all(zero), Sum(0));
    }

    #[test] // 7. GROUP: a ⊕ a⁻¹ == e
    fn group_inverse() {
        for a in mod4_samples() {
            assert!(a.compose(a.invert()).equals(&Mod4::empty()));
            assert!(a.invert().compose(a).equals(&Mod4::empty()));
        }
        for n in [-9i64, 0, 33] {
            assert_eq!(Sum(n).compose(Sum(n).invert()), Sum::empty());
        }
    }

    #[test] // 8. FILTERABLE: lọc bằng Some == identity; lọc bằng None == rỗng
    fn filterable() {
        let v = vec![1i32, 2, 3];
        assert_eq!(v.clone().filter_map(Some), v);
        assert_eq!(
            v.clone().filter_map(|_: i32| None::<i32>),
            Vec::<i32>::new()
        );
        // luật phân phối: lọc rồi lọc == lọc bằng hàm ghép
        let f = |x: i32| if x % 2 == 0 { Some(x) } else { None };
        let g = |x: i32| if x > 2 { Some(x * 10) } else { None };
        assert_eq!(
            v.clone().filter_map(f).filter_map(g),
            v.clone().filter_map(|x| f(x).and_then(g))
        );
    }

    #[test] // 9. FUNCTOR: identity và composition
    fn functor() {
        for x in [Some(3i32), None] {
            assert_eq!(x.fmap(|a| a), x);
            let (f, g) = (|a: i32| a + 2, |a: i32| a * 5);
            assert_eq!(x.fmap(f).fmap(g), x.fmap(|a| g(f(a))));
        }
        let v = vec![1i32, 2, 3];
        assert_eq!(v.clone().fmap(|a| a), v);
    }

    #[test] // 10. CONTRAVARIANT: contramap(id) == id
    fn contravariant() {
        let original = Predicate::new(|n: &i64| *n > 10);
        let via_contramap = Predicate::new(|n: &i64| *n > 10).contramap(|n: &i64| *n);
        for n in [-5i64, 10, 11, 99] {
            assert_eq!(original.check(&n), via_contramap.check(&n));
        }
    }

    #[test] // 11-12. APPLY / APPLICATIVE: luật đồng nhất  ap(of(id), v) == v
    fn applicative_identity() {
        for v in [Some(7i32), None] {
            let id: Option<Box<dyn Fn(i32) -> i32>> =
                of_option(Box::new(|x: i32| x) as Box<dyn Fn(i32) -> i32>);
            assert_eq!(ap_option(id, v), v);
        }
        // Đồng cấu: ap(of(f), of(x)) == of(f(x))
        let f = |x: i32| x * 4;
        let wrapped: Option<Box<dyn Fn(i32) -> i32>> =
            of_option(Box::new(f) as Box<dyn Fn(i32) -> i32>);
        assert_eq!(ap_option(wrapped, of_option(5)), of_option(f(5)));
    }

    #[test] // 12b. APPLICATIVE tích lũy lỗi: gom ĐỦ lỗi, khác hẳn Monad
    fn applicative_accumulates_errors() {
        let func: Validation<Box<dyn Fn(i32) -> i32>> = Validation::Invalid(vec!["lỗi A".into()]);
        let value: Validation<i32> = Validation::Invalid(vec!["lỗi B".into()]);
        match ap_validation(func, value) {
            Validation::Invalid(e) => assert_eq!(e.len(), 2, "phải gom CẢ HAI lỗi"),
            _ => panic!("phải hỏng"),
        }
    }

    #[test] // 13-14. ALT kết hợp · PLUS đơn vị & triệt tiêu
    fn alt_plus() {
        for a in [Some(1i32), None] {
            for b in [Some(2i32), None] {
                for c in [Some(3i32), None] {
                    assert_eq!(a.alt(b).alt(c), a.alt(b.alt(c))); // Alt kết hợp
                }
            }
        }
        for a in [Some(1i32), None] {
            assert_eq!(<Option<i32> as Plus>::zero().alt(a), a); // đơn vị trái
            assert_eq!(a.alt(<Option<i32> as Plus>::zero()), a); // đơn vị phải
        }
    }

    #[test] // 16. FOLDABLE: gấp cây tương đương gấp danh sách các phần tử
    fn foldable() {
        let tree = Tree::Node(
            Box::new(Tree::Node(
                Box::new(Tree::Leaf),
                20i64,
                Box::new(Tree::Leaf),
            )),
            50,
            Box::new(Tree::Node(Box::new(Tree::Leaf), 70, Box::new(Tree::Leaf))),
        );
        assert_eq!(tree.clone().fold(0i64, |a, x| a + x), 140);
        assert_eq!(
            tree.clone().fold(Vec::new(), |mut a, x| {
                a.push(x);
                a
            }),
            vec![20, 50, 70]
        );
        assert_eq!(Tree::<i64>::Leaf.fold(0i64, |a, x| a + x), 0); // cây rỗng -> phần tử đơn vị
    }

    #[test] // 17. TRAVERSABLE: đảo ngữ cảnh, ngắn mạch ở phần tử hỏng đầu tiên
    fn traversable() {
        assert_eq!(
            traverse_vec_result(vec!["1", "2"], |s: &str| s.parse::<i32>()),
            Ok(vec![1, 2])
        );
        assert!(traverse_vec_result(vec!["1", "x"], |s: &str| s.parse::<i32>()).is_err());
        assert_eq!(
            traverse_vec_option(vec![1i32, 2], |x| Some(x * 2)),
            Some(vec![2, 4])
        );
        assert_eq!(
            traverse_vec_option(vec![1i32, 2], |x| if x > 1 { None } else { Some(x) }),
            None
        );
    }

    #[test] // 18. CHAIN: (m >>= f) >>= g  ==  m >>= (x -> f(x) >>= g)
    fn chain_associative() {
        let f = |x: i32| if x >= 0 { Some(x + 1) } else { None };
        let g = |x: i32| if x % 2 == 0 { Some(x / 2) } else { None };
        for m in [Some(-3i32), Some(0), Some(3), Some(7), None] {
            assert_eq!(m.chain(f).chain(g), m.chain(|x| f(x).chain(g)));
        }
    }

    #[test] // 20. MONAD: đơn vị trái & đơn vị phải
    fn monad_identity() {
        let f = |x: i32| if x > 0 { Some(x * 2) } else { None };
        for a in [-1i32, 0, 5] {
            assert_eq!(of_option(a).chain(f), f(a));
        } // trái
        for m in [Some(4i32), None] {
            assert_eq!(m.chain(of_option), m);
        } // phải
    }

    #[test] // 19. CHAINREC: chạy 1 TRIỆU vòng mà KHÔNG tràn ngăn xếp
    fn chainrec_does_not_overflow_the_stack() {
        let result = chain_rec_option((0u64, 1_000_000u32), |(acc, remaining)| {
            Some(if remaining == 0 {
                Step::Finished(acc)
            } else {
                Step::Continue((acc + 1, remaining - 1))
            })
        });
        assert_eq!(result, Some(1_000_000));
    }

    #[test] // 21-22. EXTEND & COMONAD: extract(extend(w, f)) == f(w)
    fn comonad() {
        let w = Window {
            prev: vec![1i64, 2],
            focus: 3,
            next: vec![4, 5],
        };
        let f = |c: &Window<i64>| c.focus * 10;
        assert_eq!(*w.clone().extend(f).extract(), f(&w)); // đơn vị trái
        // extend(w, extract) == w   (đơn vị phải)
        let back: Window<i64> = w.clone().extend(|c: &Window<i64>| *c.extract());
        assert_eq!(back, w);
    }

    #[test] // 23. BIFUNCTOR: bimap(id, id) == id
    fn bifunctor() {
        let ok: Result<i32, String> = Ok(5);
        let err: Result<i32, String> = Err("hỏng".into());
        assert_eq!(ok.clone().bimap(|a| a, |b| b), ok);
        assert_eq!(err.clone().bimap(|a| a, |b| b), err);
        assert_eq!(
            err.bimap(|a| a + 1, |b| format!("[{}]", b)),
            Err("[hỏng]".to_string())
        );
        assert_eq!((1i32, "x").bimap(|a| a * 2, |b: &str| b.len()), (2, 1));
    }

    #[test] // 24. PROFUNCTOR: promap(id, id) == id
    fn profunctor() {
        for x in [-4i64, 0, 9] {
            let original = |a: i64| a * 3;
            let via = Func::new(original).promap(|a: i64| a, |b: i64| b);
            assert_eq!(via.run(x), original(x));
        }
    }
}
```

---

## 5. Ba điều Rust làm khác — và vì sao

### 5.1. Không có Kiểu bậc cao (HKT), nên không có `trait Monad` tổng quát

Trong đặc tả Fantasy Land, `Monad` là một giao diện mà **bất kỳ** kiểu chứa nào cũng cài được. Trong Rust, bạn không viết được `trait Monad { fn chain<A,B>(self: Self<A>, ...) -> Self<B> }` vì `Self<A>` là cú pháp không tồn tại.

Cách vòng tránh dùng trong mã trên (và trong thư viện [`fp-core.rs`](https://github.com/JasonShin/fp-core.rs)) là **mô phỏng HKT bằng kiểu liên kết**: thay vì nói `Self<U>`, ta nói `Self::Target` và để mỗi kiểu tự khai báo "đích đến" của mình. Nó hoạt động, nhưng chữ ký dài dòng hơn và không tổng quát bằng.

Tin vui: **GAT (Generic Associated Types)** đã ổn định từ Rust 1.65 và thu hẹp đáng kể khoảng cách này.

### 5.2. Quy tắc mồ côi buộc phải dùng kiểu bọc

Muốn cài `Semigroup` cho `i64` theo *hai* cách (cộng và nhân)? Không được — lỗi `E0119`. Muốn cài trait của thư viện khác cho kiểu của thư viện khác? Không được — lỗi `E0117`.

Lối thoát duy nhất là **kiểu bọc**: `Sum(i64)`, `Product(i64)`, `Func<A,B>`, `Predicate<A>`. Bạn thấy mẫu này khắp mã nguồn trên. Đây không phải hạn chế vô cớ: nó bảo đảm **tính nhất quán cài đặt** — cả chương trình luôn thống nhất về việc `a.compose(b)` nghĩa là gì.

### 5.3. Quyền sở hữu làm thay đổi hình dạng chữ ký

Fantasy Land viết cho JavaScript, nơi mọi giá trị đều chia sẻ thoải mái. Trong Rust bạn phải quyết định:

| Chữ ký | Ý nghĩa | Dùng khi |
|---|---|---|
| `fn compose(self, other: Self) -> Self` | **tiêu thụ** cả hai | phép gộp — tái dùng được bộ đệm, nhanh nhất |
| `fn equals(&self, other: &Self) -> bool` | chỉ **đọc** | phép so sánh — không cần sở hữu |
| `fn extract(&self) -> &Self::Inner` | trả **tham chiếu** | `extract` của Comonad — tránh sao chép |

Chính vì vậy `Semigroup::compose` trong mã trên nhận `self` theo giá trị: gộp hai `String` thì tái sử dụng luôn bộ đệm của chuỗi thứ nhất, thay vì cấp phát chuỗi thứ ba. Đây là chỗ Rust **nhanh hơn** bản JavaScript của cùng một trừu tượng.

---

## 6. Đọc tiếp

- [fantasyland/fantasy-land](https://github.com/fantasyland/fantasy-land) — bản đặc tả gốc, có đầy đủ luật dạng hình thức và phần *Derivations* (cách suy ra phép này từ phép kia).
- [JasonShin/fp-core.rs](https://github.com/JasonShin/fp-core.rs) — thư viện Rust cài đặt các trait này, đáng đọc phần `src/hkt.rs`.
- [enricopolanski/functional-programming](https://github.com/enricopolanski/functional-programming) — giáo trình dẫn dắt từ Magma tới Monad bằng TypeScript, rất gần với cách trình bày của Chương 18–19.
- *Functional Programming Made Easier* (Charles Scalfani) — nguồn của lộ trình Typeclass → Đại số → Fold → Functor → Applicative → Monad.

---

*Quay lại [Mục lục](./SUMMARY.md) · Xem [Bảng thuật ngữ](./GLOSSARY.md) · Ôn lại [Chương 18](./chuong_18.md), [Chương 19](./chuong_19.md), [Chương 20](./chuong_20.md)*
