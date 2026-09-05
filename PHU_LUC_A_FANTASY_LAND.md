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
        Monoid                                        │
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

                                    Foldable ──► Traversable

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
| 20 | **Monad** | Đơn nguyên | Applicative + Chain | đơn vị trái · đơn vị phải | `Option`, `Result`, `Iterator`, `Future` |
| 21 | **Extend** | Mở rộng | `extend` | kết hợp | Không có sẵn |
| 22 | **Comonad** | Đối đơn nguyên | `extend` + `extract` | đơn vị trái · đơn vị phải | Không có sẵn |
| 23 | **Bifunctor** | Hàm tử hai ngôi | `bimap` | đơn vị · ghép | **`Result::map` + `map_err`** |
| 24 | **Profunctor** | Profunctor | `promap` | đơn vị · ghép | Không có sẵn — tự viết cho `Fn(A) -> B` |

**Đọc bảng này thế nào**: cột cuối cho thấy Rust **đã có sẵn 15/24** dưới dạng phương thức thư viện chuẩn — bạn dùng chúng hằng ngày mà không biết tên gọi chung. Bảy cái còn lại phải tự viết, và mã trong phụ lục này làm đúng việc đó.

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

Ở Chương 14 bạn đã viết hàm `ghep` và kiểm chứng hai luật *kết hợp* và *đơn vị*. Hai luật đó chính là định nghĩa của **Category**:

- **Semigroupoid** = có phép ghép hai "mũi tên" khớp đầu nối đuôi (`A → B` ghép `B → C` thành `A → C`), tuân luật kết hợp.
- **Category** = Semigroupoid **cộng thêm** một "mũi tên đơn vị" `id: A → A` cho mọi kiểu.

Cái tên nghe ghê gớm nhưng nội dung thì bạn đã dùng từ Chương 14. Điều đáng nhớ: **hàm trong Rust tạo thành một phạm trù**, và mọi thứ ở nhánh 3 (Functor, Monad…) đều được định nghĩa *dựa trên* phạm trù này.

### 3.3. Contravariant — hàm tử "đi ngược"

`Functor` cho phép đắp thêm việc vào **đầu ra**. `Contravariant` cho phép đắp thêm vào **đầu vào**.

Ví dụ kinh điển là **vị từ** (`predicate`): bạn có `ViTu<i64>` biết kiểm tra "số này có chẵn không". Bạn muốn có `ViTu<String>` kiểm tra "chuỗi này có độ dài chẵn không". Bạn không thể `map` — vì `ViTu` *tiêu thụ* giá trị chứ không *sản xuất* ra nó. Thứ bạn cần là hàm đi **ngược chiều**: `String -> i64`.

```
Functor      :  F<A>  +  (A -> B)  =  F<B>       ← hàm cùng chiều
Contravariant:  F<A>  +  (B -> A)  =  F<B>       ← hàm NGƯỢC chiều
```

Trong Rust bạn gặp mẫu này ở mọi hàm so sánh và mọi bộ lọc tùy biến: `sort_by_key(|x| x.tuoi)` chính là contramap trên bộ so sánh.

### 3.4. Profunctor — vừa nghịch biến vừa hiệp biến

Một hàm `A -> B` có **hai chỗ** để đắp thêm: đầu vào (nghịch biến) và đầu ra (hiệp biến). Kiểu nào đắp được cả hai gọi là **Profunctor**, với phép `promap`:

```
promap :  (C -> A)  +  Ham<A, B>  +  (B -> D)  =  Ham<C, D>
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
let cong = tu_dong_lenh.or(tu_bien_moi_truong).or(tu_tep_cau_hinh).unwrap_or(8080);
```

### 3.7. ChainRec — câu trả lời cho vấn đề tràn ngăn xếp

Đây là cấu trúc **thực dụng nhất trong nhóm chưa được dạy**, và nó liên quan trực tiếp tới cảnh báo ở Chương 16 rằng **Rust không bảo đảm tối ưu hóa lời gọi đuôi**.

Vấn đề: một vòng lặp đơn nguyên viết bằng đệ quy sẽ làm phình ngăn xếp:

```rust
// ❌ Đệ quy đơn nguyên — 1.000.000 vòng sẽ TRÀN NGĂN XẾP
fn dem_nguoc(n: u32) -> Option<u32> {
    if n == 0 { Some(0) } else { dem_nguoc(n - 1) }
}
```

`ChainRec` giải quyết bằng cách bắt hàm bước trả về một **thẻ báo hiệu** thay vì tự gọi lại chính nó:

```rust
enum BuocTiep<A, B> { TiepTuc(A), Xong(B) }
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

Đây chính là mô hình tính toán của: bộ lọc ảnh (mỗi điểm ảnh cần biết các điểm lân cận), trung bình trượt trên chuỗi thời gian, và trò chơi Life của Conway. Mã dưới đây cài đặt `CuaSo<T>` và dùng nó tính tổng ba ô lân cận cho mỗi vị trí — chỉ bằng một lời gọi `mo_rong`.

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
    fn bang(&self, khac: &Self) -> bool;
}

/// 2. ORD — Setoid có thêm quan hệ thứ tự toàn phần.
pub trait ThuTu: Setoid {
    fn so_sanh(&self, khac: &Self) -> Ordering;
    fn nho_hon_hoac_bang(&self, khac: &Self) -> bool {
        self.so_sanh(khac) != Ordering::Greater
    }
}

/// 5. SEMIGROUP — phép gộp hai thành một, tuân luật kết hợp.
pub trait NuaNhom {
    fn ghep(self, khac: Self) -> Self;
}

/// 6. MONOID — nửa nhóm có phần tử đơn vị.
pub trait ViNhom: NuaNhom + Sized {
    fn don_vi() -> Self;
}

/// 7. GROUP — vị nhóm có phần tử nghịch đảo.
pub trait Nhom: ViNhom {
    fn nghich_dao(self) -> Self;
}

// ---- Instance: Tong (vị nhóm cộng) là một NHÓM đầy đủ ----
#[derive(Debug, Clone, Copy, PartialEq)]
pub struct Tong(pub i64);
impl Setoid for Tong {
    fn bang(&self, k: &Self) -> bool { self.0 == k.0 }
}
impl ThuTu for Tong {
    fn so_sanh(&self, k: &Self) -> Ordering { self.0.cmp(&k.0) }
}
impl NuaNhom for Tong {
    fn ghep(self, k: Self) -> Self { Tong(self.0.wrapping_add(k.0)) }
}
impl ViNhom for Tong {
    fn don_vi() -> Self { Tong(0) }
}
impl Nhom for Tong {
    fn nghich_dao(self) -> Self { Tong(-self.0) }
}

// ---- Instance: Mod4 — nhóm cộng modulo 4 (hữu hạn, dễ kiểm chứng vét cạn) ----
#[derive(Debug, Clone, Copy, PartialEq)]
pub struct Mod4(pub u8);
impl Setoid for Mod4 {
    fn bang(&self, k: &Self) -> bool { self.0 % 4 == k.0 % 4 }
}
impl NuaNhom for Mod4 {
    fn ghep(self, k: Self) -> Self { Mod4((self.0 + k.0) % 4) }
}
impl ViNhom for Mod4 {
    fn don_vi() -> Self { Mod4(0) }
}
impl Nhom for Mod4 {
    fn nghich_dao(self) -> Self { Mod4((4 - self.0 % 4) % 4) }
}

// ---- Instance: String là nửa nhóm + vị nhóm, nhưng KHÔNG phải nhóm ----
impl NuaNhom for String {
    fn ghep(self, k: Self) -> Self { self + &k }
}
impl ViNhom for String {
    fn don_vi() -> Self { String::new() }
}

/// Gộp vạn năng cho mọi vị nhóm.
pub fn gop_tat_ca<M: ViNhom>(ds: impl IntoIterator<Item = M>) -> M {
    ds.into_iter().fold(M::don_vi(), |a, x| a.ghep(x))
}

// ══════════════════════════════════════════════════════════════════════════
// NHÓM B — ĐẠI SỐ TRÊN HÀM (Semigroupoid, Category, Profunctor)
// ══════════════════════════════════════════════════════════════════════════

/// Bọc một hàm thành giá trị để có thể cài trait lên nó (vượt quy tắc mồ côi).
pub struct Ham<A, B>(Box<dyn Fn(A) -> B>);

impl<A, B> Ham<A, B> {
    pub fn moi(f: impl Fn(A) -> B + 'static) -> Self { Ham(Box::new(f)) }
    pub fn chay(&self, a: A) -> B { (self.0)(a) }
}

/// 3. SEMIGROUPOID — có phép ghép hai "mũi tên" khớp đầu nối đuôi.
impl<A: 'static, B: 'static> Ham<A, B> {
    pub fn ghep_voi<C: 'static>(self, sau: Ham<B, C>) -> Ham<A, C> {
        Ham::moi(move |a| sau.chay(self.chay(a)))
    }
}

/// 4. CATEGORY — Semigroupoid có thêm "mũi tên đơn vị".
pub fn dong_nhat<A>() -> Ham<A, A> {
    Ham::moi(|a| a)
}

/// 24. PROFUNCTOR — nghịch biến ở đầu vào, hiệp biến ở đầu ra.
impl<A: 'static, B: 'static> Ham<A, B> {
    pub fn promap<C: 'static, D: 'static>(
        self,
        truoc: impl Fn(C) -> A + 'static,  // NGHỊCH biến: đắp thêm vào ĐẦU VÀO
        sau: impl Fn(B) -> D + 'static,    // HIỆP biến : đắp thêm vào ĐẦU RA
    ) -> Ham<C, D> {
        Ham::moi(move |c| sau(self.chay(truoc(c))))
    }
}

/// 10. CONTRAVARIANT — chỉ có đầu vào để đắp thêm. Ví dụ kinh điển: vị từ.
pub struct ViTu<A>(Box<dyn Fn(&A) -> bool>);

impl<A: 'static> ViTu<A> {
    pub fn moi(f: impl Fn(&A) -> bool + 'static) -> Self { ViTu(Box::new(f)) }
    pub fn kiem(&self, a: &A) -> bool { (self.0)(a) }

    /// contramap: từ vị từ trên A, tạo ra vị từ trên B nhờ hàm B -> A.
    pub fn contramap<B: 'static>(self, f: impl Fn(&B) -> A + 'static) -> ViTu<B> {
        ViTu::moi(move |b| self.kiem(&f(b)))
    }
}

// ══════════════════════════════════════════════════════════════════════════
// NHÓM C — ĐẠI SỐ TRÊN NGỮ CẢNH (cần HKT, ta mô phỏng bằng kiểu liên kết)
// ══════════════════════════════════════════════════════════════════════════

pub trait HKT<U> {
    type HienTai;
    type DichDen;
}
impl<T, U> HKT<U> for Option<T> { type HienTai = T; type DichDen = Option<U>; }
impl<T, U> HKT<U> for Vec<T> { type HienTai = T; type DichDen = Vec<U>; }
impl<T, U, E> HKT<U> for Result<T, E> { type HienTai = T; type DichDen = Result<U, E>; }

/// 9. FUNCTOR
pub trait HamTu<U>: HKT<U> {
    fn anh_xa<F: FnMut(Self::HienTai) -> U>(self, f: F) -> Self::DichDen;
}
impl<T, U> HamTu<U> for Option<T> {
    fn anh_xa<F: FnMut(T) -> U>(self, f: F) -> Option<U> { self.map(f) }
}
impl<T, U> HamTu<U> for Vec<T> {
    fn anh_xa<F: FnMut(T) -> U>(self, f: F) -> Vec<U> { self.into_iter().map(f).collect() }
}
impl<T, U, E> HamTu<U> for Result<T, E> {
    fn anh_xa<F: FnMut(T) -> U>(self, f: F) -> Result<U, E> { self.map(f) }
}

/// 8. FILTERABLE — lọc và biến đổi cùng lúc bằng A -> Option<B>.
pub trait LocDuoc<U>: HKT<U> {
    fn loc_anh_xa<F: FnMut(Self::HienTai) -> Option<U>>(self, f: F) -> Self::DichDen;
}
impl<T, U> LocDuoc<U> for Vec<T> {
    fn loc_anh_xa<F: FnMut(T) -> Option<U>>(self, f: F) -> Vec<U> {
        self.into_iter().filter_map(f).collect()
    }
}
impl<T, U> LocDuoc<U> for Option<T> {
    fn loc_anh_xa<F: FnMut(T) -> Option<U>>(self, mut f: F) -> Option<U> {
        self.and_then(|x| f(x))
    }
}

/// 23. BIFUNCTOR — hai chân, đắp thêm được vào cả hai.
pub trait HamTuHaiNgoi<C, D> {
    type Ra;
    fn bimap(self, f: impl FnOnce(Self::Trai) -> C, g: impl FnOnce(Self::Phai) -> D) -> Self::Ra;
    type Trai;
    type Phai;
}
impl<A, B, C, D> HamTuHaiNgoi<C, D> for Result<A, B> {
    type Trai = A;
    type Phai = B;
    type Ra = Result<C, D>;
    fn bimap(self, f: impl FnOnce(A) -> C, g: impl FnOnce(B) -> D) -> Result<C, D> {
        match self {
            Ok(a) => Ok(f(a)),
            Err(b) => Err(g(b)),
        }
    }
}
impl<A, B, C, D> HamTuHaiNgoi<C, D> for (A, B) {
    type Trai = A;
    type Phai = B;
    type Ra = (C, D);
    fn bimap(self, f: impl FnOnce(A) -> C, g: impl FnOnce(B) -> D) -> (C, D) {
        (f(self.0), g(self.1))
    }
}

// ---- 11. APPLY & 12. APPLICATIVE (bản cụ thể cho Option / Result / Vec) ----

/// APPLY: ngữ cảnh chứa HÀM áp vào ngữ cảnh chứa GIÁ TRỊ.
pub fn ap_option<A, B>(ham: Option<Box<dyn Fn(A) -> B>>, gt: Option<A>) -> Option<B> {
    match (ham, gt) {
        (Some(f), Some(a)) => Some(f(a)),
        _ => None,
    }
}
pub fn ap_result<A, B, E>(ham: Result<Box<dyn Fn(A) -> B>, E>, gt: Result<A, E>) -> Result<B, E> {
    match (ham, gt) {
        (Ok(f), Ok(a)) => Ok(f(a)),
        (Err(e), _) => Err(e),
        (_, Err(e)) => Err(e),
    }
}
/// APPLICATIVE: `of` — nhấc một giá trị trần vào ngữ cảnh.
pub fn of_option<A>(a: A) -> Option<A> { Some(a) }
pub fn of_result<A, E>(a: A) -> Result<A, E> { Ok(a) }
pub fn of_vec<A>(a: A) -> Vec<A> { vec![a] }

/// APPLICATIVE tích lũy lỗi — biến thể `Validation` (không phải Monad!).
#[derive(Debug, Clone, PartialEq)]
pub enum XacThuc<T> {
    Dat(T),
    Hong(Vec<String>),
}
impl<T> XacThuc<T> {
    pub fn anh_xa<U>(self, f: impl FnOnce(T) -> U) -> XacThuc<U> {
        match self {
            XacThuc::Dat(x) => XacThuc::Dat(f(x)),
            XacThuc::Hong(e) => XacThuc::Hong(e),
        }
    }
}
pub fn ap_xac_thuc<A, B>(ham: XacThuc<Box<dyn Fn(A) -> B>>, gt: XacThuc<A>) -> XacThuc<B> {
    match (ham, gt) {
        (XacThuc::Dat(f), XacThuc::Dat(a)) => XacThuc::Dat(f(a)),
        (XacThuc::Hong(mut e1), XacThuc::Hong(e2)) => { e1.extend(e2); XacThuc::Hong(e1) }
        (XacThuc::Hong(e), _) => XacThuc::Hong(e),
        (_, XacThuc::Hong(e)) => XacThuc::Hong(e),
    }
}

// ---- 13. ALT · 14. PLUS · 15. ALTERNATIVE ----

/// ALT — "hoặc cái này hoặc cái kia", giữ nguyên kiểu.
pub trait Alt {
    fn alt(self, khac: Self) -> Self;
}
/// PLUS — Alt có thêm phần tử "rỗng".
pub trait Plus: Alt + Sized {
    fn rong() -> Self;
}
impl<T> Alt for Option<T> {
    fn alt(self, khac: Self) -> Self { self.or(khac) }
}
impl<T> Plus for Option<T> {
    fn rong() -> Self { None }
}
impl<T> Alt for Vec<T> {
    fn alt(mut self, mut khac: Self) -> Self { self.append(&mut khac); self }
}
impl<T> Plus for Vec<T> {
    fn rong() -> Self { Vec::new() }
}
/// ALTERNATIVE = Applicative + Plus. Trong Rust: đánh dấu bằng siêu trait.
pub trait LuaChonThayThe: Plus {}
impl<T> LuaChonThayThe for Option<T> {}
impl<T> LuaChonThayThe for Vec<T> {}

// ---- 16. FOLDABLE · 17. TRAVERSABLE ----

/// FOLDABLE — gấp một cấu trúc về một giá trị.
pub trait GapDuoc {
    type Phan;
    fn gap<B>(self, khoi_tao: B, f: impl FnMut(B, Self::Phan) -> B) -> B;
}
#[derive(Debug, Clone, PartialEq)]
pub enum Cay<T> {
    La,
    Nut(Box<Cay<T>>, T, Box<Cay<T>>),
}
impl<T> GapDuoc for Cay<T> {
    type Phan = T;
    fn gap<B>(self, khoi_tao: B, mut f: impl FnMut(B, T) -> B) -> B {
        fn di<T, B>(c: Cay<T>, acc: B, f: &mut impl FnMut(B, T) -> B) -> B {
            match c {
                Cay::La => acc,
                Cay::Nut(t, v, p) => {
                    let acc = di(*t, acc, f);
                    let acc = f(acc, v);
                    di(*p, acc, f)
                }
            }
        }
        di(self, khoi_tao, &mut f)
    }
}

/// TRAVERSABLE — đảo ngữ cảnh từ trong ra ngoài.
pub fn duyet_vec_result<A, B, E>(
    ds: Vec<A>,
    f: impl FnMut(A) -> Result<B, E>,
) -> Result<Vec<B>, E> {
    ds.into_iter().map(f).collect()
}
pub fn duyet_vec_option<A, B>(ds: Vec<A>, f: impl FnMut(A) -> Option<B>) -> Option<Vec<B>> {
    ds.into_iter().map(f).collect()
}

// ---- 18. CHAIN · 19. CHAINREC · 20. MONAD ----

/// CHAIN — phép `bind`: A -> F<B>.
pub trait Noi<U>: HKT<U> {
    fn noi<F: FnMut(Self::HienTai) -> Self::DichDen>(self, f: F) -> Self::DichDen;
}
impl<T, U> Noi<U> for Option<T> {
    fn noi<F: FnMut(T) -> Option<U>>(self, mut f: F) -> Option<U> { self.and_then(|x| f(x)) }
}
impl<T, U, E> Noi<U> for Result<T, E> {
    fn noi<F: FnMut(T) -> Result<U, E>>(self, mut f: F) -> Result<U, E> { self.and_then(|x| f(x)) }
}
impl<T, U> Noi<U> for Vec<T> {
    fn noi<F: FnMut(T) -> Vec<U>>(self, f: F) -> Vec<U> { self.into_iter().flat_map(f).collect() }
}

/// MONAD = Applicative + Chain. Trong Rust: siêu trait đánh dấu.
pub trait DonNguyen<U>: Noi<U> + HamTu<U> {}
impl<T, U> DonNguyen<U> for Option<T> {}
impl<T, U, E> DonNguyen<U> for Result<T, E> {}
impl<T, U> DonNguyen<U> for Vec<T> {}

/// CHAINREC — lặp đơn nguyên với NGĂN XẾP KHÔNG PHÌNH TO.
/// Đây là câu trả lời của Fantasy Land cho việc Rust không tối ưu hóa lời gọi đuôi.
#[derive(Debug, Clone, PartialEq)]
pub enum BuocTiep<A, B> {
    TiepTuc(A),
    Xong(B),
}
pub fn chain_rec_option<A, B>(
    khoi_dau: A,
    mut buoc: impl FnMut(A) -> Option<BuocTiep<A, B>>,
) -> Option<B> {
    let mut hien_tai = khoi_dau;
    loop {
        match buoc(hien_tai)? {
            BuocTiep::TiepTuc(a) => hien_tai = a, // vòng lặp, KHÔNG đệ quy
            BuocTiep::Xong(b) => return Some(b),
        }
    }
}

// ---- 21. EXTEND · 22. COMONAD ----

/// EXTEND — đối ngẫu của Chain: F<A> -> (F<A> -> B) -> F<B>.
pub trait MoRong<U>: HKT<U> + Sized {
    fn mo_rong<F: FnMut(&Self) -> U>(self, f: F) -> Self::DichDen;
}
/// COMONAD — Extend có thêm `extract`: F<A> -> A (đối ngẫu của `of`).
/// 22a. `extract` được tách riêng, đúng như đặc tả Fantasy Land: nó KHÔNG phụ
/// thuộc kiểu đích U, nên không được đặt trong một trait generic theo U.
pub trait TrichXuat {
    type Ruot;
    fn trich_xuat(&self) -> &Self::Ruot;
}

/// 22b. COMONAD = Extend + extract (đối ngẫu của Monad = Chain + of).
pub trait DoiDonNguyen<U>: MoRong<U> + TrichXuat {}

/// Ví dụ kinh điển: con trỏ trượt trên dãy (Zipper) — luôn có "tiêu điểm".
#[derive(Debug, Clone, PartialEq)]
pub struct CuaSo<T> {
    pub truoc: Vec<T>,
    pub tieu_diem: T,
    pub sau: Vec<T>,
}
impl<T, U> HKT<U> for CuaSo<T> {
    type HienTai = T;
    type DichDen = CuaSo<U>;
}
impl<T: Clone, U> MoRong<U> for CuaSo<T> {
    fn mo_rong<F: FnMut(&Self) -> U>(self, mut f: F) -> CuaSo<U> {
        let n = self.truoc.len();
        let tat_ca: Vec<T> = self
            .truoc
            .iter()
            .cloned()
            .chain(std::iter::once(self.tieu_diem.clone()))
            .chain(self.sau.iter().cloned())
            .collect();
        let tai = |i: usize| CuaSo {
            truoc: tat_ca[..i].to_vec(),
            tieu_diem: tat_ca[i].clone(),
            sau: tat_ca[i + 1..].to_vec(),
        };
        CuaSo {
            truoc: (0..n).map(|i| f(&tai(i))).collect(),
            tieu_diem: f(&tai(n)),
            sau: ((n + 1)..tat_ca.len()).map(|i| f(&tai(i))).collect(),
        }
    }
}
impl<T> TrichXuat for CuaSo<T> {
    type Ruot = T;
    fn trich_xuat(&self) -> &T { &self.tieu_diem }
}
impl<T: Clone, U> DoiDonNguyen<U> for CuaSo<T> {}

// ══════════════════════════════════════════════════════════════════════════
// CHƯƠNG TRÌNH DEMO
// ══════════════════════════════════════════════════════════════════════════

fn main() {
    println!("═══════════════════════════════════════════════════════════════");
    println!("   24 CẤU TRÚC ĐẠI SỐ FANTASY LAND — HIỆN THỰC HÓA BẰNG RUST   ");
    println!("═══════════════════════════════════════════════════════════════");

    println!("\n── NHÓM A: ĐẠI SỐ TRÊN MỘT KIỂU ──");
    println!(" 1. Setoid     Tong(5).bang(&Tong(5))      = {}", Tong(5).bang(&Tong(5)));
    println!(" 2. Ord        Tong(3).so_sanh(&Tong(9))   = {:?}", Tong(3).so_sanh(&Tong(9)));
    println!(" 5. Semigroup  Tong(3).ghep(Tong(4))       = {:?}", Tong(3).ghep(Tong(4)));
    println!(" 6. Monoid     don_vi()                    = {:?}", Tong::don_vi());
    println!(" 7. Group      Tong(7).nghich_dao()        = {:?}", Tong(7).nghich_dao());
    println!("               Mod4(3).ghep(nghich_dao)    = {:?}", Mod4(3).ghep(Mod4(3).nghich_dao()));
    println!("    (String là Monoid nhưng KHÔNG phải Group: không có \"chuỗi âm\")");

    println!("\n── NHÓM B: ĐẠI SỐ TRÊN HÀM ──");
    let nhan2 = Ham::moi(|x: i64| x * 2);
    let cong3 = Ham::moi(|x: i64| x + 3);
    let ghep = nhan2.ghep_voi(cong3);
    println!(" 3. Semigroupoid  (nhân2 rồi cộng3)(10)    = {}", ghep.chay(10));
    println!(" 4. Category      identity(42)             = {}", dong_nhat::<i64>().chay(42));
    let do_dai = Ham::moi(|s: String| s.chars().count());
    let pro = do_dai.promap(|n: i64| format!("số {}", n), |u: usize| u * 100);
    println!("24. Profunctor    promap(i64 -> usize)(7)  = {}", pro.chay(7));
    let la_chan = ViTu::moi(|n: &i64| n % 2 == 0);
    let ten_chan = la_chan.contramap(|s: &String| s.chars().count() as i64);
    println!("10. Contravariant \"Rust\" có độ dài chẵn?   = {}", ten_chan.kiem(&"Rust".to_string()));

    println!("\n── NHÓM C: ĐẠI SỐ TRÊN NGỮ CẢNH ──");
    println!(" 9. Functor      Some(5).anh_xa(+1)        = {:?}", Some(5i32).anh_xa(|x| x + 1));
    println!(" 8. Filterable   lọc số phân tích được     = {:?}",
             vec!["1", "x", "3"].loc_anh_xa(|s: &str| s.parse::<i32>().ok()));
    println!("23. Bifunctor    Err(2).bimap(+1, *10)     = {:?}",
             (Err(2i32) as Result<i32, i32>).bimap(|a| a + 1, |b| b * 10));
    let f: Option<Box<dyn Fn(i32) -> i32>> = Some(Box::new(|x| x * 3));
    println!("11. Apply        ap(Some(*3), Some(7))     = {:?}", ap_option(f, Some(7)));
    println!("12. Applicative  of(9)                     = {:?}", of_option(9));
    println!("13. Alt          None.alt(Some(2))         = {:?}", None.alt(Some(2)));
    println!("14. Plus         Option::rong()            = {:?}", <Option<i32> as Plus>::rong());
    println!("15. Alternative  = Applicative + Plus (siêu trait đánh dấu)");

    let cay = Cay::Nut(
        Box::new(Cay::Nut(Box::new(Cay::La), 20i64, Box::new(Cay::La))),
        50,
        Box::new(Cay::Nut(Box::new(Cay::La), 70, Box::new(Cay::La))),
    );
    println!("16. Foldable     gấp cây [20,50,70] -> tổng= {}", cay.clone().gap(0i64, |a, x| a + x));
    println!("17. Traversable  Vec<Result> -> Result<Vec>= {:?}",
             duyet_vec_result(vec!["1", "2"], |s: &str| s.parse::<i32>()));
    println!("18. Chain        Some(4).noi(|x| Some(x*5))= {:?}", Some(4i32).noi(|x| Some(x * 5)));
    println!("20. Monad        = Applicative + Chain (siêu trait đánh dấu)");

    let luy_thua = chain_rec_option(( 1u64, 20u32), |(acc, con_lai)| {
        Some(if con_lai == 0 { BuocTiep::Xong(acc) } else { BuocTiep::TiepTuc((acc * 2, con_lai - 1)) })
    });
    println!("19. ChainRec     2^20 bằng vòng lặp        = {:?}", luy_thua);

    let cs = CuaSo { truoc: vec![1i64, 2], tieu_diem: 3, sau: vec![4, 5] };
    println!("22. Comonad      trích xuất tiêu điểm      = {}", cs.trich_xuat());
    let tong_lan_can = cs.clone().mo_rong(|w: &CuaSo<i64>| {
        w.truoc.last().copied().unwrap_or(0) + w.tieu_diem + w.sau.first().copied().unwrap_or(0)
    });
    println!("21. Extend       tổng 3 ô lân cận mỗi vị trí= {:?}",
             [tong_lan_can.truoc.clone(), vec![tong_lan_can.tieu_diem], tong_lan_can.sau.clone()].concat());

    println!("\n═══════════════════════════════════════════════════════════════");
    println!("   24/24 CẤU TRÚC — MỖI CÁI MỘT ĐỊNH NGHĨA, MỘT LUẬT, MỘT MÃ    ");
    println!("═══════════════════════════════════════════════════════════════");
}

// ══════════════════════════════════════════════════════════════════════════
// KIỂM CHỨNG LUẬT — MỖI ĐẠI SỐ MỘT BÀI TEST
// ══════════════════════════════════════════════════════════════════════════

#[cfg(test)]
mod luat {
    use super::*;

    fn mau4() -> Vec<Mod4> { (0..4).map(Mod4).collect() }

    #[test] // 1. SETOID: phản xạ, đối xứng, bắc cầu
    fn setoid() {
        for a in mau4() { assert!(a.bang(&a)); }                                  // phản xạ
        for a in mau4() { for b in mau4() { assert_eq!(a.bang(&b), b.bang(&a)); } } // đối xứng
        for a in mau4() { for b in mau4() { for c in mau4() {
            if a.bang(&b) && b.bang(&c) { assert!(a.bang(&c)); }                   // bắc cầu
        }}}
    }

    #[test] // 2. ORD: toàn phần, phản đối xứng, bắc cầu
    fn ord() {
        let m: Vec<Tong> = (-3..4).map(Tong).collect();
        for a in &m { for b in &m {
            assert!(a.nho_hon_hoac_bang(b) || b.nho_hon_hoac_bang(a));            // toàn phần
            if a.nho_hon_hoac_bang(b) && b.nho_hon_hoac_bang(a) { assert!(a.bang(b)); }
        }}
    }

    #[test] // 3. SEMIGROUPOID: (f ∘ g) ∘ h == f ∘ (g ∘ h)
    fn semigroupoid_ket_hop() {
        for x in [-5i64, 0, 7, 100] {
            let trai = Ham::moi(|a: i64| a + 1).ghep_voi(Ham::moi(|a: i64| a * 2))
                          .ghep_voi(Ham::moi(|a: i64| a - 3));
            let phai = Ham::moi(|a: i64| a + 1)
                          .ghep_voi(Ham::moi(|a: i64| a * 2).ghep_voi(Ham::moi(|a: i64| a - 3)));
            assert_eq!(trai.chay(x), phai.chay(x));
        }
    }

    #[test] // 4. CATEGORY: id ∘ f == f == f ∘ id
    fn category_don_vi() {
        for x in [-5i64, 0, 42] {
            let f = |a: i64| a * 3 + 1;
            assert_eq!(dong_nhat::<i64>().ghep_voi(Ham::moi(f)).chay(x), f(x));
            assert_eq!(Ham::moi(f).ghep_voi(dong_nhat::<i64>()).chay(x), f(x));
        }
    }

    #[test] // 5. SEMIGROUP: (a ⊕ b) ⊕ c == a ⊕ (b ⊕ c)
    fn semigroup_ket_hop() {
        for a in mau4() { for b in mau4() { for c in mau4() {
            assert!(a.ghep(b).ghep(c).bang(&a.ghep(b.ghep(c))));
        }}}
        let s = ["a".to_string(), "bc".to_string(), "d".to_string()];
        assert_eq!(s[0].clone().ghep(s[1].clone()).ghep(s[2].clone()),
                   s[0].clone().ghep(s[1].clone().ghep(s[2].clone())));
    }

    #[test] // 6. MONOID: e ⊕ a == a == a ⊕ e
    fn monoid_don_vi() {
        for a in mau4() {
            assert!(Mod4::don_vi().ghep(a).bang(&a));
            assert!(a.ghep(Mod4::don_vi()).bang(&a));
        }
        let rong: Vec<Tong> = Vec::new();
        assert_eq!(gop_tat_ca(rong), Tong(0));
    }

    #[test] // 7. GROUP: a ⊕ a⁻¹ == e
    fn group_nghich_dao() {
        for a in mau4() {
            assert!(a.ghep(a.nghich_dao()).bang(&Mod4::don_vi()));
            assert!(a.nghich_dao().ghep(a).bang(&Mod4::don_vi()));
        }
        for n in [-9i64, 0, 33] {
            assert_eq!(Tong(n).ghep(Tong(n).nghich_dao()), Tong::don_vi());
        }
    }

    #[test] // 8. FILTERABLE: lọc bằng Some == identity; lọc bằng None == rỗng
    fn filterable() {
        let v = vec![1i32, 2, 3];
        assert_eq!(v.clone().loc_anh_xa(Some), v);
        assert_eq!(v.clone().loc_anh_xa(|_: i32| None::<i32>), Vec::<i32>::new());
        // luật phân phối: lọc rồi lọc == lọc bằng hàm ghép
        let f = |x: i32| if x % 2 == 0 { Some(x) } else { None };
        let g = |x: i32| if x > 2 { Some(x * 10) } else { None };
        assert_eq!(v.clone().loc_anh_xa(f).loc_anh_xa(g),
                   v.clone().loc_anh_xa(|x| f(x).and_then(g)));
    }

    #[test] // 9. FUNCTOR: identity và composition
    fn functor() {
        for x in [Some(3i32), None] {
            assert_eq!(x.anh_xa(|a| a), x);
            let (f, g) = (|a: i32| a + 2, |a: i32| a * 5);
            assert_eq!(x.anh_xa(f).anh_xa(g), x.anh_xa(|a| g(f(a))));
        }
        let v = vec![1i32, 2, 3];
        assert_eq!(v.clone().anh_xa(|a| a), v);
    }

    #[test] // 10. CONTRAVARIANT: contramap(id) == id
    fn contravariant() {
        let goc = ViTu::moi(|n: &i64| *n > 10);
        let qua_contramap = ViTu::moi(|n: &i64| *n > 10).contramap(|n: &i64| *n);
        for n in [-5i64, 10, 11, 99] {
            assert_eq!(goc.kiem(&n), qua_contramap.kiem(&n));
        }
    }

    #[test] // 11-12. APPLY / APPLICATIVE: luật đồng nhất  ap(of(id), v) == v
    fn applicative_dong_nhat() {
        for v in [Some(7i32), None] {
            let id: Option<Box<dyn Fn(i32) -> i32>> = of_option(Box::new(|x: i32| x) as Box<dyn Fn(i32) -> i32>);
            assert_eq!(ap_option(id, v), v);
        }
        // Đồng cấu: ap(of(f), of(x)) == of(f(x))
        let f = |x: i32| x * 4;
        let bao: Option<Box<dyn Fn(i32) -> i32>> = of_option(Box::new(f) as Box<dyn Fn(i32) -> i32>);
        assert_eq!(ap_option(bao, of_option(5)), of_option(f(5)));
    }

    #[test] // 12b. APPLICATIVE tích lũy lỗi: gom ĐỦ lỗi, khác hẳn Monad
    fn applicative_tich_luy_loi() {
        let ham: XacThuc<Box<dyn Fn(i32) -> i32>> = XacThuc::Hong(vec!["lỗi A".into()]);
        let gt: XacThuc<i32> = XacThuc::Hong(vec!["lỗi B".into()]);
        match ap_xac_thuc(ham, gt) {
            XacThuc::Hong(e) => assert_eq!(e.len(), 2, "phải gom CẢ HAI lỗi"),
            _ => panic!("phải hỏng"),
        }
    }

    #[test] // 13-14. ALT kết hợp · PLUS đơn vị & triệt tiêu
    fn alt_plus() {
        for a in [Some(1i32), None] { for b in [Some(2i32), None] { for c in [Some(3i32), None] {
            assert_eq!(a.alt(b).alt(c), a.alt(b.alt(c)));           // Alt kết hợp
        }}}
        for a in [Some(1i32), None] {
            assert_eq!(<Option<i32> as Plus>::rong().alt(a), a);     // đơn vị trái
            assert_eq!(a.alt(<Option<i32> as Plus>::rong()), a);     // đơn vị phải
        }
    }

    #[test] // 16. FOLDABLE: gấp cây tương đương gấp danh sách các phần tử
    fn foldable() {
        let cay = Cay::Nut(
            Box::new(Cay::Nut(Box::new(Cay::La), 20i64, Box::new(Cay::La))),
            50,
            Box::new(Cay::Nut(Box::new(Cay::La), 70, Box::new(Cay::La))),
        );
        assert_eq!(cay.clone().gap(0i64, |a, x| a + x), 140);
        assert_eq!(cay.clone().gap(Vec::new(), |mut a, x| { a.push(x); a }), vec![20, 50, 70]);
        assert_eq!(Cay::<i64>::La.gap(0i64, |a, x| a + x), 0); // cây rỗng -> phần tử đơn vị
    }

    #[test] // 17. TRAVERSABLE: đảo ngữ cảnh, ngắn mạch ở phần tử hỏng đầu tiên
    fn traversable() {
        assert_eq!(duyet_vec_result(vec!["1", "2"], |s: &str| s.parse::<i32>()), Ok(vec![1, 2]));
        assert!(duyet_vec_result(vec!["1", "x"], |s: &str| s.parse::<i32>()).is_err());
        assert_eq!(duyet_vec_option(vec![1i32, 2], |x| Some(x * 2)), Some(vec![2, 4]));
        assert_eq!(duyet_vec_option(vec![1i32, 2], |x| if x > 1 { None } else { Some(x) }), None);
    }

    #[test] // 18. CHAIN: (m >>= f) >>= g  ==  m >>= (x -> f(x) >>= g)
    fn chain_ket_hop() {
        let f = |x: i32| if x >= 0 { Some(x + 1) } else { None };
        let g = |x: i32| if x % 2 == 0 { Some(x / 2) } else { None };
        for m in [Some(-3i32), Some(0), Some(3), Some(7), None] {
            assert_eq!(m.noi(f).noi(g), m.noi(|x| f(x).noi(g)));
        }
    }

    #[test] // 20. MONAD: đơn vị trái & đơn vị phải
    fn monad_don_vi() {
        let f = |x: i32| if x > 0 { Some(x * 2) } else { None };
        for a in [-1i32, 0, 5] { assert_eq!(of_option(a).noi(f), f(a)); }   // trái
        for m in [Some(4i32), None] { assert_eq!(m.noi(of_option), m); }    // phải
    }

    #[test] // 19. CHAINREC: chạy 1 TRIỆU vòng mà KHÔNG tràn ngăn xếp
    fn chainrec_khong_tran_ngan_xep() {
        let kq = chain_rec_option((0u64, 1_000_000u32), |(acc, con_lai)| {
            Some(if con_lai == 0 { BuocTiep::Xong(acc) }
                 else { BuocTiep::TiepTuc((acc + 1, con_lai - 1)) })
        });
        assert_eq!(kq, Some(1_000_000));
    }

    #[test] // 21-22. EXTEND & COMONAD: extract(extend(w, f)) == f(w)
    fn comonad() {
        let w = CuaSo { truoc: vec![1i64, 2], tieu_diem: 3, sau: vec![4, 5] };
        let f = |c: &CuaSo<i64>| c.tieu_diem * 10;
        assert_eq!(*w.clone().mo_rong(f).trich_xuat(), f(&w));   // đơn vị trái
        // extend(w, extract) == w   (đơn vị phải)
        let lai: CuaSo<i64> = w.clone().mo_rong(|c: &CuaSo<i64>| *c.trich_xuat());
        assert_eq!(lai, w);
    }

    #[test] // 23. BIFUNCTOR: bimap(id, id) == id
    fn bifunctor() {
        let ok: Result<i32, String> = Ok(5);
        let er: Result<i32, String> = Err("hỏng".into());
        assert_eq!(ok.clone().bimap(|a| a, |b| b), ok);
        assert_eq!(er.clone().bimap(|a| a, |b| b), er);
        assert_eq!(er.bimap(|a| a + 1, |b| format!("[{}]", b)), Err("[hỏng]".to_string()));
        assert_eq!((1i32, "x").bimap(|a| a * 2, |b: &str| b.len()), (2, 1));
    }

    #[test] // 24. PROFUNCTOR: promap(id, id) == id
    fn profunctor() {
        for x in [-4i64, 0, 9] {
            let goc = |a: i64| a * 3;
            let qua = Ham::moi(goc).promap(|a: i64| a, |b: i64| b);
            assert_eq!(qua.chay(x), goc(x));
        }
    }
}
```

---

## 5. Ba điều Rust làm khác — và vì sao

### 5.1. Không có Kiểu bậc cao (HKT), nên không có `trait Monad` tổng quát

Trong đặc tả Fantasy Land, `Monad` là một giao diện mà **bất kỳ** kiểu chứa nào cũng cài được. Trong Rust, bạn không viết được `trait Monad { fn chain<A,B>(self: Self<A>, ...) -> Self<B> }` vì `Self<A>` là cú pháp không tồn tại.

Cách vòng tránh dùng trong mã trên (và trong thư viện [`fp-core.rs`](https://github.com/JasonShin/fp-core.rs)) là **mô phỏng HKT bằng kiểu liên kết**: thay vì nói `Self<U>`, ta nói `Self::DichDen` và để mỗi kiểu tự khai báo "đích đến" của mình. Nó hoạt động, nhưng chữ ký dài dòng hơn và không tổng quát bằng.

Tin vui: **GAT (Generic Associated Types)** đã ổn định từ Rust 1.65 và thu hẹp đáng kể khoảng cách này.

### 5.2. Quy tắc mồ côi buộc phải dùng kiểu bọc

Muốn cài `NuaNhom` cho `i64` theo *hai* cách (cộng và nhân)? Không được — lỗi `E0119`. Muốn cài trait của thư viện khác cho kiểu của thư viện khác? Không được — lỗi `E0117`.

Lối thoát duy nhất là **kiểu bọc**: `Tong(i64)`, `Tich(i64)`, `Ham<A,B>`, `ViTu<A>`. Bạn thấy mẫu này khắp mã nguồn trên. Đây không phải hạn chế vô cớ: nó bảo đảm **tính nhất quán cài đặt** — cả chương trình luôn thống nhất về việc `a.ghep(b)` nghĩa là gì.

### 5.3. Quyền sở hữu làm thay đổi hình dạng chữ ký

Fantasy Land viết cho JavaScript, nơi mọi giá trị đều chia sẻ thoải mái. Trong Rust bạn phải quyết định:

| Chữ ký | Ý nghĩa | Dùng khi |
|---|---|---|
| `fn ghep(self, khac: Self) -> Self` | **tiêu thụ** cả hai | phép gộp — tái dùng được bộ đệm, nhanh nhất |
| `fn bang(&self, khac: &Self) -> bool` | chỉ **đọc** | phép so sánh — không cần sở hữu |
| `fn trich_xuat(&self) -> &Self::Ruot` | trả **tham chiếu** | `extract` của Comonad — tránh sao chép |

Chính vì vậy `NuaNhom::ghep` trong mã trên nhận `self` theo giá trị: gộp hai `String` thì tái sử dụng luôn bộ đệm của chuỗi thứ nhất, thay vì cấp phát chuỗi thứ ba. Đây là chỗ Rust **nhanh hơn** bản JavaScript của cùng một trừu tượng.

---

## 6. Đọc tiếp

- [fantasyland/fantasy-land](https://github.com/fantasyland/fantasy-land) — bản đặc tả gốc, có đầy đủ luật dạng hình thức và phần *Derivations* (cách suy ra phép này từ phép kia).
- [JasonShin/fp-core.rs](https://github.com/JasonShin/fp-core.rs) — thư viện Rust cài đặt các trait này, đáng đọc phần `src/hkt.rs`.
- [enricopolanski/functional-programming](https://github.com/enricopolanski/functional-programming) — giáo trình dẫn dắt từ Magma tới Monad bằng TypeScript, rất gần với cách trình bày của Chương 18–19.
- *Functional Programming Made Easier* (Charles Scalfani) — nguồn của lộ trình Typeclass → Đại số → Fold → Functor → Applicative → Monad.

---

*Quay lại [Mục lục](./SUMMARY.md) · Xem [Bảng thuật ngữ](./THUAT_NGU.md) · Ôn lại [Chương 18](./chuong_18.md), [Chương 19](./chuong_19.md), [Chương 20](./chuong_20.md)*
