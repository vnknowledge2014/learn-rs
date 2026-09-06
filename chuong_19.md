# Chương 19: Hàm tử, Hàm tử áp dụng và Đơn nguyên — bản đồ sang thư viện chuẩn Rust (Functor, Applicative & Monad)

## Giới thiệu & Mục tiêu học tập

Có một sự thật thú vị: **bạn đã dùng Monad từ Chương 11 rồi mà không hề hay biết.**

Mỗi lần bạn viết `balance.and_then(|x| rut_tien(x))`, bạn đang gọi phép toán mà cả thế giới lập trình hàm gọi là **bind** — trái tim của Monad. Mỗi lần bạn viết `.map()`, bạn đang dùng **Functor**. Mỗi lần bạn gõ toán tử `?`, bạn đang dùng thứ mà Haskell gọi là **do-notation**.

Vậy tại sao phải học tên gọi của những thứ mình đã biết làm? Ba lý do rất cụ thể:

1. **Nhìn ra điểm chung.** `Option::map`, `Result::map`, `Iterator::map` trông giống nhau không phải do trùng hợp — chúng là *cùng một khuôn mẫu*. Khi bạn thấy khuôn mẫu, bạn đoán được API mà không cần tra tài liệu.
2. **Mở khóa những công cụ bạn chưa biết là mình cần.** Chương này giới thiệu `collect::<Result<Vec<_>, E>>()` và `transpose()` — hai công cụ mà hầu như mọi chương trình Rust đọc dữ liệu ngoài đều cần, nhưng người tự học thường mất vài năm mới tình cờ gặp.
3. **Đọc được tài liệu và mã nguồn quốc tế.** Khi đọc một crate Rust hay một bài viết tiếng Anh, các từ *functor*, *applicative*, *monadic* sẽ không còn là bức tường.

Chương này cũng trả lời câu hỏi mà mọi người học Rust nghiêm túc sớm muộn cũng gặp: **"Rust có Monad không?"** — và câu trả lời trung thực hơn bạn tưởng.

Mục tiêu học tập của chương này:
- Hiểu **Hàm tử (Functor)** là gì, hai **luật Functor**, và bản đồ của nó sang `Option`, `Result`, `Vec`, `Iterator`.
- Biết **Hàm tử hai ngôi (Bifunctor)** và vì sao `Result::map_err` là "chân còn lại" của `Result::map`.
- Nắm **Hàm tử áp dụng (Applicative)** và ứng dụng đắt giá nhất của nó: **xác thực tích lũy lỗi** — báo *tất cả* lỗi của biểu mẫu thay vì chỉ lỗi đầu tiên.
- Làm chủ **Traversable**: `collect::<Result<Vec<_>, E>>()` và `Option::transpose()` — biến `Vec<Result<T,E>>` thành `Result<Vec<T>,E>`.
- Hiểu **Đơn nguyên (Monad)**: `and_then` chính là `bind`, `flatten` chính là `join`, toán tử `?` chính là do-notation; kèm **ba luật Monad**.
- Trả lời được câu hỏi **"Vì sao Rust chưa có Monad tổng quát?"** — khái niệm **Kiểu bậc cao (Higher-Kinded Type)** và cách thư viện `fp-core.rs` mô phỏng nó.

---

## Hình tượng hóa đời sống (Intuitive Everyday Analogy)

```
┌──────────────────────────────────────────────────────────────────────────────────┐
│         HÌNH TƯỢNG ĐỜI SỐNG: HỘP QUÀ NIÊM PHONG VÀ BA CÁCH LÀM VIỆC VỚI NÓ       │
├──────────────────────────────────────────────────────────────────────────────────┤
│                                                                                  │
│  Một "ngữ cảnh" (context) là chiếc HỘP có thể chứa hoặc không chứa món quà:      │
│     Option<T>   = hộp có thể RỖNG                                                │
│     Result<T,E> = hộp có thể chứa PHIẾU BÁO LỖI thay vì quà                      │
│     Vec<T>      = hộp chứa NHIỀU món cùng lúc                                    │
│                                                                                  │
│  ┌──────────────────────────────────────────────────────────────────────────┐   │
│  │ 1. FUNCTOR (map)  — "SƠN LẠI MÓN QUÀ MÀ KHÔNG MỞ HỘP"                    │   │
│  │    Bạn đưa cho nhân viên bưu điện một cây cọ. Họ luồn tay vào sơn món     │   │
│  │    quà rồi niêm phong lại. Hộp vẫn là hộp. Rỗng thì vẫn rỗng.             │   │
│  │       [quà] --map(sơn)--> [quà đã sơn]      [rỗng] --map--> [rỗng]        │   │
│  └──────────────────────────────────────────────────────────────────────────┘   │
│                                                                                  │
│  ┌──────────────────────────────────────────────────────────────────────────┐   │
│  │ 2. APPLICATIVE (zip) — "GỘP NHIỀU HỘP ĐỘC LẬP THÀNH MỘT"                 │   │
│  │    Ba hộp gửi song song từ ba nơi. Chỉ khi CẢ BA cùng tới thì mới ráp     │   │
│  │    thành bộ quà. Nếu thiếu, bạn biết được TẤT CẢ hộp nào thiếu.           │   │
│  │       [A] [B] [C]        --zip-->  [A+B+C]                                │   │
│  │       [A] [∅] [∅]        --zip-->  báo: "thiếu hộp 2 VÀ hộp 3"            │   │
│  └──────────────────────────────────────────────────────────────────────────┘   │
│                                                                                  │
│  ┌──────────────────────────────────────────────────────────────────────────┐   │
│  │ 3. MONAD (and_then) — "MỞ HỘP RA XEM RỒI MỚI QUYẾT ĐỊNH GỬI HỘP TIẾP"    │   │
│  │    Bước sau PHỤ THUỘC vào nội dung bước trước. Mở hộp thấy "mã đơn hàng"  │   │
│  │    thì mới gọi được cửa hàng để lấy hộp tiếp theo. Không mở thì không     │   │
│  │    biết phải làm gì. → Đây là khác biệt cốt lõi so với Applicative!       │   │
│  │       [quà] --and_then(mở ra, dựa vào đó tạo hộp mới)--> [hộp mới] / [∅]  │   │
│  └──────────────────────────────────────────────────────────────────────────┘   │
└──────────────────────────────────────────────────────────────────────────────────┘
```

### 1. Functor — cây cọ luồn vào hộp

Bạn có một chiếc hộp niêm phong. Bạn muốn sơn món quà bên trong nhưng **không được phép mở hộp**. Giải pháp: đưa cây cọ (một hàm `A -> B`) cho hộp, hộp tự sơn bên trong rồi trả lại chính nó.

Điều quan trọng: **hình dạng chiếc hộp không đổi**. Hộp rỗng vẫn rỗng, hộp lỗi vẫn lỗi, hộp 5 món vẫn 5 món. Chỉ *nội dung* thay đổi.

### 2. Applicative — ba hộp gửi song song

Bạn đặt ba món hàng từ ba cửa hàng khác nhau, **hoàn toàn độc lập**. Đơn nào cũng đã gửi đi rồi. Khi cả ba tới nơi, bạn ráp thành một bộ quà.

Điểm mấu chốt: vì ba đơn độc lập, nếu có sự cố bạn biết được **tất cả** đơn nào hỏng cùng lúc. Đây chính là nền tảng của kỹ thuật **xác thực tích lũy lỗi** — điều mà toán tử `?` (vốn dừng ngay ở lỗi đầu tiên) không làm được.

### 3. Monad — mở hộp rồi mới biết bước tiếp theo

Lần này khác: bạn mở hộp thứ nhất, thấy bên trong là *"mã đơn hàng ORD-8891"*. Chỉ **sau khi biết mã đó**, bạn mới gọi được cửa hàng để hỏi hộp thứ hai.

Bước sau **phụ thuộc vào nội dung** bước trước. Không thể gửi song song. Đây chính là ranh giới phân biệt Monad với Applicative, và cũng là lý do vì sao chuỗi `and_then` phải chạy tuần tự.

---

## Khái niệm & Cơ chế kỹ thuật chuyên sâu (In-Depth Technical Mechanics)

### 1. Hàm tử (Functor) và hai luật của nó

Một kiểu `F<T>` là **Hàm tử** nếu nó có phép `map` với chữ ký:

```
map : F<A> -> (A -> B) -> F<B>
```

và tuân đúng **hai luật**:

```
(F1) Luật đơn vị: x.map(|a| a)      ==  x
(F2) Luật ghép   : x.map(f).map(g)  ==  x.map(|a| g(f(a)))
```

Luật F2 chính là **phép ghép hàm ở Chương 14** được nâng lên cấp ngữ cảnh. Và nó không chỉ là lý thuyết: đó là **giấy phép để trình biên dịch gộp hai vòng `map` thành một** — kỹ thuật *loop fusion* giúp iterator của Rust nhanh ngang vòng lặp C viết tay (Chương 16).

Bản đồ sang Rust:

| Hàm tử | `map` | Ý nghĩa "hộp" |
|---|---|---|
| `Option<T>` | `Option::map` | Có thể rỗng |
| `Result<T, E>` | `Result::map` | Có thể lỗi |
| `Vec<T>` | `.iter().map(...).collect()` | Chứa nhiều giá trị |
| `Iterator` | `Iterator::map` | Dòng chảy lười biếng |
| `Future` | (qua `.await` và các tổ hợp tử) | Giá trị sẽ có trong tương lai (Chương 49) |

> **Mẹo nhận diện**: nếu một kiểu có phương thức tên `map` nhận closure `A -> B` và trả về "cùng loại hộp nhưng chứa B", gần như chắc chắn đó là một Hàm tử.

### 2. Hàm tử hai ngôi (Bifunctor): `Result` có HAI chân

`Result<T, E>` chứa dữ liệu ở **hai vị trí**, nên nó có hai phép `map`:

```rust
let a: Result<i32, String> = Ok(5);
a.map(|x| x * 2);          // sơn "chân thành công" -> Ok(10)

let b: Result<i32, String> = Err("hỏng".into());
b.map_err(|e| format!("Lỗi hệ thống: {}", e));  // sơn "chân lỗi"
```

Một kiểu có `map` cho **cả hai** vị trí gọi là **Bifunctor**. Trong thực chiến, `map_err` là công cụ quan trọng bậc nhất để chuyển đổi lỗi giữa các tầng:

```rust
// Tầng dưới trả lỗi kỹ thuật, tầng trên cần lỗi nghiệp vụ
doc_tep(path)
    .map_err(|e| LoiNghiepVu::KhongDocDuocCauHinh(e.to_string()))?;
```

### 3. Hàm tử áp dụng (Applicative) và bài toán "báo hết lỗi một lần"

Toán tử `?` mà bạn học ở Chương 11 có một đặc tính: **ngắn mạch tại lỗi đầu tiên**.

```rust
fn register(form: &Form) -> Result<User, String> {
    let name = check_name(&form.name)?;      // Hỏng ở đây thì...
    let mail = kiem_tra_mail(&form.mail)?;   // ...dòng này không bao giờ chạy
    let age = check_age(&form.age)?;   // ...và dòng này cũng vậy
    Ok(User { name, mail, age })
}
```

Với một máy chủ nội bộ thì không sao. Nhưng với **biểu mẫu đăng ký của người dùng thật**, hành vi này rất tệ: người dùng sửa lỗi tên, bấm gửi, lại báo lỗi email; sửa email, bấm gửi, lại báo lỗi tuổi. Ba vòng qua lại chỉ vì ta báo lỗi từng cái một.

Applicative giải quyết đúng vấn đề này. Vì ba phép kiểm tra **độc lập** với nhau (không cái nào cần kết quả của cái nào), ta có thể chạy cả ba rồi **gom hết lỗi lại**:

```
Kết quả: Hong(["Tên quá ngắn (cần ít nhất 4 ký tự)",
               "Email thiếu ký tự @",
               "Tuổi không phải số nguyên"])
```

| | Applicative | Monad |
|---|---|---|
| Các bước có phụ thuộc nhau? | **Không** — độc lập | **Có** — bước sau cần kết quả bước trước |
| Xử lý lỗi | Gom **tất cả** lỗi | Dừng ở lỗi **đầu tiên** |
| Chạy song song được? | Được | Không |
| Trong Rust | `Option::zip`, kiểu `Auth` tự viết | `and_then`, toán tử `?` |

> **Quy tắc chọn**: các trường của một biểu mẫu độc lập nhau → dùng Applicative để báo hết lỗi. Các bước của một quy trình nghiệp vụ nối tiếp nhau → dùng `?` để dừng sớm. Đây là quyết định thiết kế, không phải sở thích.

### 4. Traversable: đảo ngược ngữ cảnh — công cụ bị bỏ quên nhất trong Rust

Bạn có một danh sách chuỗi cần chuyển thành số. Kết quả tự nhiên là `Vec<Result<i32, E>>` — một danh sách các kết quả. Nhưng thứ bạn *thật sự muốn* thường là `Result<Vec<i32>, E>` — "hoặc là cả danh sách đều tốt, hoặc là báo lỗi".

Phép **đảo ngữ cảnh** đó gọi là `sequence` / `traverse`. Trong Rust nó được cài sẵn ngay trong `collect()`:

```rust
let tho = vec!["10", "20", "30"];
let so: Result<Vec<i32>, _> = tho.iter().map(|s| s.parse::<i32>()).collect();
assert_eq!(so, Ok(vec![10, 20, 30]));

let hong = vec!["10", "hai muoi", "30"];
let so: Result<Vec<i32>, _> = hong.iter().map(|s| s.parse::<i32>()).collect();
assert!(so.is_err());   // Cả danh sách hỏng vì MỘT phần tử hỏng
```

Đây là một trong những dòng mã hữu ích nhất trong toàn bộ thư viện chuẩn Rust, và nó hoạt động vì `Result` cài đặt trait `FromIterator`. Cùng họ với nó:

| Bạn có | Bạn muốn | Dùng |
|---|---|---|
| `Vec<Result<T, E>>` | `Result<Vec<T>, E>` | `.collect::<Result<Vec<_>, _>>()` |
| `Vec<Option<T>>` | `Option<Vec<T>>` | `.collect::<Option<Vec<_>>>()` |
| `Option<Result<T, E>>` | `Result<Option<T>, E>` | `.transpose()` |
| `Result<Option<T>, E>` | `Option<Result<T, E>>` | `.transpose()` |

### 5. Đơn nguyên (Monad): `and_then` chính là `bind`

Một Hàm tử `F<T>` là **Đơn nguyên** nếu ngoài `map` nó còn có:

```
bind (còn gọi là flatMap, chain, and_then) :  F<A> -> (A -> F<B>) -> F<B>
```

Hãy để ý điểm khác biệt then chốt so với `map`:

```
map  nhận hàm  A -> B      (trả về giá trị TRẦN)
bind nhận hàm  A -> F<B>   (trả về một HỘP MỚI)
```

Nếu bạn dùng nhầm `map` ở chỗ cần `bind`, bạn sẽ nhận về **hộp lồng trong hộp**: `Option<Option<T>>`. Đó chính là lúc `flatten` (tên toán học: `join`) xuất hiện:

```rust
let long: Option<Option<i32>> = Some(Some(5));
assert_eq!(long.flatten(), Some(5));

// Và đây là đẳng thức định nghĩa:  bind(x, f)  ==  x.map(f).flatten()
let x = Some(4);
let f = |n: i32| if n > 0 { Some(n * 10) } else { None };
assert_eq!(x.and_then(f), x.map(f).flatten());
```

**Ba luật Monad:**

```
(M1) Đơn vị trái : Some(a).and_then(f)        ==  f(a)
(M2) Đơn vị phải : m.and_then(Some)           ==  m
(M3) Kết hợp     : m.and_then(f).and_then(g)  ==  m.and_then(|x| f(x).and_then(g))
```

Luật M3 nói rằng bạn có thể **nhóm lại các bước trong một chuỗi xử lý mà không đổi kết quả** — chính là thứ cho phép bạn tách một hàm dài thành nhiều hàm nhỏ rồi ghép lại một cách an toàn.

### 6. Toán tử `?` chính là do-notation của Rust

Trong Haskell, viết chuỗi bind lồng nhau rất khó đọc nên người ta phát minh ra cú pháp `do`. Rust giải quyết đúng vấn đề đó bằng toán tử `?`:

```rust
// Viết bằng bind tường minh — "kim tự tháp"
fn xu_ly_a(s: &str) -> Option<u64> {
    doc_ma_don(s).and_then(|id| return_price(id).and_then(|price| ap_thue(price)))
}

// Viết bằng `?` — phẳng phiu, đọc từ trên xuống
fn xu_ly_b(s: &str) -> Option<u64> {
    let id = doc_ma_don(s)?;
    let price = return_price(id)?;
    let last = ap_thue(price)?;
    Some(last)
}
```

Hai hàm này **hoàn toàn tương đương**. `?` không phải phép màu — nó chỉ là đường cú pháp cho `bind`.

Điều này cũng giải thích một giới hạn mà người học hay thắc mắc: **vì sao không trộn được `Option` và `Result` trong cùng một hàm với `?`?** Vì mỗi hàm chỉ "ở trong" đúng một đơn nguyên tại một thời điểm. Muốn chuyển giữa hai thế giới, phải nói rõ:

```rust
let x = tim_nguoi_dung(id).ok_or("Không tìm thấy người dùng")?;  // Option -> Result
let y = doc_so(s).ok();                                          // Result -> Option
```

Và một chi tiết ít người biết: toán tử `?` **tự động gọi `From::from` trên kiểu lỗi**. Đó là lý do bạn có thể trả về nhiều loại lỗi khác nhau từ cùng một hàm, miễn là chúng đều `impl From<...> for LoiCuaBan`. (Xem lại Chương 11 và Chương 12.)

### 7. "Rust có Monad không?" — Câu chuyện Kiểu bậc cao (HKT)

Câu trả lời trung thực: **Rust có rất nhiều monad cụ thể, nhưng chưa có trait `Monad` tổng quát.**

`Option`, `Result`, `Iterator`, `Future` đều là monad — chúng đều có `map` và `and_then`. Nhưng bạn **không thể** viết một hàm dùng chung cho tất cả:

```rust
// Đoạn mã này KHÔNG biên dịch được trong Rust:
// trait Monoid {
//     fn bind<A, B>(self: Self<A>, f: impl Fn(A) -> Self<B>) -> Self<B>;
// }
```

Vấn đề nằm ở `Self<A>`. Rust cho phép generic trên **kiểu** (`T`), nhưng chưa cho phép generic trên **bộ tạo kiểu** (`Option` khi chưa điền `T` vào). Khả năng đó gọi là **Kiểu bậc cao (Higher-Kinded Type — HKT)**, và Rust chưa hỗ trợ.

Cộng đồng có một cách vòng tránh khéo léo, được thư viện **`fp-core.rs`** dùng: mô phỏng HKT bằng **kiểu liên kết (associated types)**.

```rust
pub trait HKT<U> {
    type Current;  // kiểu đang chứa bên trong, ví dụ T của Option<T>
    type DichDen;  // "cùng cái hộp đó nhưng chứa U", ví dụ Option<U>
}

impl<T, U> HKT<U> for Option<T> {
    type Current = T;
    type DichDen = Option<U>;
}

pub trait Functor<U>: HKT<U> {
    fn mapping<F>(self, f: F) -> Self::DichDen
    where
        F: FnMut(Self::Current) -> U;
}
```

Mẹo ở đây: thay vì nói `Self<U>` (không viết được), ta nói `Self::DichDen` và để mỗi kiểu tự khai báo "đích đến" của mình là gì. Chương trình minh họa bên dưới cài đặt đầy đủ mẫu này cho `Option`, `Result` và `Vec`.

> **Tin vui**: tính năng **Generic Associated Types (GAT)** đã ổn định từ Rust 1.65 và thu hẹp đáng kể khoảng cách này. Nhiều thư viện hiện đại đã dùng GAT để biểu diễn những trừu tượng trước đây phải chờ HKT.

---

## Mã nguồn minh họa thực chiến (Idiomatic Runnable Rust Blueprint)

Chương trình dưới đây xây dựng **Cổng tiếp nhận Đơn đăng ký (Registration Intake Gateway)**, so sánh trực diện hai chiến lược xử lý lỗi: `?` ngắn mạch (Monad) và tích lũy toàn bộ lỗi (Applicative).

```rust
// Tệp: src/main.rs
// Chương trình thực chiến: Functor, Applicative, Monad và bản đồ sang Rust

// ============================================================================
// PHẦN 1: MÔ PHỎNG KIỂU BẬC CAO (HKT) THEO CÁCH CỦA fp-core.rs
// ============================================================================

/// `HKT<U>` trả lời câu hỏi: "cái hộp này đang chứa gì, và nếu đổi ruột
/// sang kiểu U thì nó trở thành kiểu gì?"
pub trait HKT<U> {
    type Current; // T trong Option<T>
    type DichDen; // Option<U>
}

impl<T, U> HKT<U> for Option<T> {
    type Current = T;
    type DichDen = Option<U>;
}
impl<T, U> HKT<U> for Vec<T> {
    type Current = T;
    type DichDen = Vec<U>;
}
impl<T, U, E> HKT<U> for Result<T, E> {
    type Current = T;
    type DichDen = Result<U, E>;
}

/// HÀM TỬ tổng quát: nhờ HKT, một trait duy nhất dùng chung cho Option, Result và Vec.
pub trait Functor<U>: HKT<U> {
    fn mapping<F>(self, f: F) -> Self::DichDen
    where
        F: FnMut(Self::Current) -> U;
}

impl<T, U> Functor<U> for Option<T> {
    fn mapping<F>(self, f: F) -> Option<U>
    where
        F: FnMut(T) -> U,
    {
        self.map(f)
    }
}
impl<T, U> Functor<U> for Vec<T> {
    fn mapping<F>(self, f: F) -> Vec<U>
    where
        F: FnMut(T) -> U,
    {
        self.into_iter().map(f).collect()
    }
}
impl<T, U, E> Functor<U> for Result<T, E> {
    fn mapping<F>(self, f: F) -> Result<U, E>
    where
        F: FnMut(T) -> U,
    {
        self.map(f)
    }
}

// ============================================================================
// PHẦN 2: KIỂU XÁC THỰC TÍCH LŨY LỖI (APPLICATIVE VALIDATION)
// ============================================================================

/// Khác `Result`: khi hỏng, `Auth` giữ lại TOÀN BỘ danh sách lỗi.
#[derive(Debug, Clone, PartialEq)]
pub enum Auth<T> {
    Dat(T),
    Hong(Vec<String>),
}

impl<T> Auth<T> {
    /// FUNCTOR: sơn lại giá trị bên trong mà không đụng tới danh sách lỗi.
    pub fn mapping<U>(self, f: impl FnOnce(T) -> U) -> Auth<U> {
        match self {
            Auth::Dat(x) => Auth::Dat(f(x)),
            Auth::Hong(error) => Auth::Hong(error),
        }
    }

    /// Chuyển từ Result sang Auth để bắt đầu tích lũy lỗi.
    pub fn tu_ket_qua(kq: Result<T, String>) -> Self {
        match kq {
            Ok(x) => Auth::Dat(x),
            Err(e) => Auth::Hong(vec![e]),
        }
    }

    pub fn is_set(&self) -> bool {
        matches!(self, Auth::Dat(_))
    }
}

/// APPLICATIVE: gộp 2 kết quả ĐỘC LẬP. Nếu cả hai hỏng, giữ lại CẢ HAI lỗi.
pub fn ghep2<A, B>(a: Auth<A>, b: Auth<B>) -> Auth<(A, B)> {
    match (a, b) {
        (Auth::Dat(x), Auth::Dat(y)) => Auth::Dat((x, y)),
        (Auth::Hong(mut e1), Auth::Hong(e2)) => {
            e1.extend(e2); // ← đây chính là chỗ LỖI ĐƯỢC TÍCH LŨY
            Auth::Hong(e1)
        }
        (Auth::Hong(e), _) => Auth::Hong(e),
        (_, Auth::Hong(e)) => Auth::Hong(e),
    }
}

/// Gộp 3 kết quả độc lập — xây trên `ghep2`, đúng tinh thần ghép hàm ở Chương 14.
pub fn ghep3<A, B, C>(a: Auth<A>, b: Auth<B>, c: Auth<C>) -> Auth<(A, B, C)> {
    ghep2(ghep2(a, b), c).mapping(|((x, y), z)| (x, y, z))
}

// ============================================================================
// PHẦN 3: MIỀN NGHIỆP VỤ — ĐƠN ĐĂNG KÝ
// ============================================================================

#[derive(Debug, Clone, PartialEq)]
pub struct DonTho {
    pub name: String,
    pub email: String,
    pub age: String,
}

#[derive(Debug, Clone, PartialEq)]
pub struct User {
    pub name: String,
    pub email: String,
    pub age: u32,
}

pub fn check_name(tho: &str) -> Result<String, String> {
    let s = tho.trim();
    if s.chars().count() < 4 {
        Err(format!("Tên {:?} quá ngắn (cần ít nhất 4 ký tự)", s))
    } else if s.chars().count() > 30 {
        Err("Tên quá dài (tối đa 30 ký tự)".to_string())
    } else {
        Ok(s.to_string())
    }
}

pub fn validate_email(tho: &str) -> Result<String, String> {
    let s = tho.trim().to_lowercase();
    if !s.contains('@') {
        Err(format!("Email {:?} thiếu ký tự @", s))
    } else if !s.contains('.') {
        Err(format!("Email {:?} thiếu tên miền hợp lệ", s))
    } else {
        Ok(s)
    }
}

pub fn check_age(tho: &str) -> Result<u32, String> {
    let s = tho.trim();
    match s.parse::<u32>() {
        Err(_) => Err(format!("Tuổi {:?} không phải số nguyên", s)),
        Ok(n) if !(16..=100).contains(&n) => {
            Err(format!("Tuổi {} nằm ngoài khoảng cho phép 16-100", n))
        }
        Ok(n) => Ok(n),
    }
}

// ---------------------------------------------------------------------------
// CHIẾN LƯỢC A — MONAD: toán tử `?` dừng ngay ở lỗi ĐẦU TIÊN
// ---------------------------------------------------------------------------
pub fn short_circuit_register(don: &DonTho) -> Result<User, String> {
    let name = check_name(&don.name)?;
    let email = validate_email(&don.email)?;
    let age = check_age(&don.age)?;
    Ok(User { name, email, age })
}

// ---------------------------------------------------------------------------
// CHIẾN LƯỢC B — APPLICATIVE: chạy cả ba, gom TẤT CẢ lỗi
// ---------------------------------------------------------------------------
pub fn accumulator_register(don: &DonTho) -> Auth<User> {
    let name = Auth::tu_ket_qua(check_name(&don.name));
    let email = Auth::tu_ket_qua(validate_email(&don.email));
    let age = Auth::tu_ket_qua(check_age(&don.age));

    ghep3(name, email, age).mapping(|(name, email, age)| User { name, email, age })
}

// ============================================================================
// PHẦN 4: HÀM PHỤ TRỢ CHO PHẦN MONAD TUẦN TỰ
// ============================================================================

pub fn doc_ma_don(s: &str) -> Option<u32> {
    s.strip_prefix("ORD-")?.parse::<u32>().ok()
}

pub fn return_price(id: u32) -> Option<u64> {
    match id {
        8891 => Some(250_000),
        8892 => Some(1_200_000),
        _ => None,
    }
}

pub fn ap_thue(price: u64) -> Option<u64> {
    price.checked_mul(110)?.checked_div(100)
}

// ============================================================================
// CHƯƠNG TRÌNH ĐIỀU HÀNH CHÍNH
// ============================================================================

fn main() {
    println!("============================================================");
    println!("     HÀM TỬ, HÀM TỬ ÁP DỤNG VÀ ĐƠN NGUYÊN TRONG RUST       ");
    println!("============================================================");

    // ------------------------------------------------------------------
    // 1. FUNCTOR: cùng một `map` cho ba chiếc hộp khác nhau
    // ------------------------------------------------------------------
    println!("\n1. HÀM TỬ (Functor) — MỘT `map`, BA CHIẾC HỘP");
    let hop_option: Option<i32> = Some(21);
    let hop_result: Result<i32, String> = Ok(21);
    let hop_vec: Vec<i32> = vec![1, 2, 3];

    println!("   Option : {:?} -> {:?}", hop_option, hop_option.map(|x| x * 2));
    println!("   Result : {:?} -> {:?}", hop_result.clone(), hop_result.map(|x| x * 2));
    println!(
        "   Vec    : {:?} -> {:?}",
        hop_vec.clone(),
        hop_vec.iter().map(|x| x * 2).collect::<Vec<_>>()
    );

    let hop_rong: Option<i32> = None;
    println!("   Hộp rỗng vẫn rỗng: {:?} -> {:?}", hop_rong, hop_rong.map(|x| x * 2));

    // Dùng trait Functor tổng quát tự viết (mô phỏng HKT)
    println!("\n   Qua trait `HamTu` tổng quát (mô phỏng HKT):");
    println!("   Option: {:?}", Some(5i32).mapping(|x| x + 1));
    println!("   Vec   : {:?}", vec![1i32, 2, 3].mapping(|x| x * 10));
    let r: Result<i32, String> = Ok(7);
    println!("   Result: {:?}", r.mapping(|x| x - 7));

    // ------------------------------------------------------------------
    // 2. HAI LUẬT FUNCTOR
    // ------------------------------------------------------------------
    println!("\n2. HAI LUẬT FUNCTOR");
    let x = Some(10i32);
    assert_eq!(x.map(|a| a), x);
    println!("   (F1) x.map(identity) == x  ✓");

    let f = |a: i32| a + 3;
    let g = |a: i32| a * 2;
    assert_eq!(x.map(f).map(g), x.map(|a| g(f(a))));
    println!("   (F2) x.map(f).map(g) == x.map(g∘f)  ✓");
    println!("        → Đây là lý do trình biên dịch gộp được 2 vòng map thành 1!");

    // ------------------------------------------------------------------
    // 3. BIFUNCTOR: Result có hai chân
    // ------------------------------------------------------------------
    println!("\n3. BIFUNCTOR — `Result` CÓ HAI CHÂN");
    let into_sum: Result<i32, String> = Ok(5);
    let that_bai: Result<i32, String> = Err("mất kết nối".into());
    println!("   map     (chân Ok) : {:?}", into_sum.map(|v| v * 100));
    println!(
        "   map_err (chân Err): {:?}",
        that_bai.map_err(|e| format!("[HỆ THỐNG] {}", e))
    );

    // ------------------------------------------------------------------
    // 4. MONAD: `and_then` chính là `bind`
    // ------------------------------------------------------------------
    println!("\n4. ĐƠN NGUYÊN — `and_then` CHÍNH LÀ `bind`");
    for id in ["ORD-8891", "ORD-9999", "SAI-DINH-DANG"] {
        let ket_qua = doc_ma_don(id).and_then(return_price).and_then(ap_thue);
        println!("   {:>14} -> {:?}", id, ket_qua);
    }

    println!("\n   Đẳng thức định nghĩa: bind(x,f) == x.map(f).flatten()");
    let x = Some(4i32);
    let f = |n: i32| if n > 0 { Some(n * 10) } else { None };
    assert_eq!(x.and_then(f), x.map(f).flatten());
    println!("   {:?} == {:?}  ✓", x.and_then(f), x.map(f).flatten());

    // ------------------------------------------------------------------
    // 5. BA LUẬT MONAD
    // ------------------------------------------------------------------
    println!("\n5. BA LUẬT MONAD");
    let a = 5i32;
    let m = Some(a);
    let f = |n: i32| Some(n + 1);
    let g = |n: i32| if n % 2 == 0 { Some(n / 2) } else { None };

    assert_eq!(Some(a).and_then(f), f(a));
    println!("   (M1) Đơn vị trái : Some(a).and_then(f) == f(a)  ✓");
    assert_eq!(m.and_then(Some), m);
    println!("   (M2) Đơn vị phải : m.and_then(Some) == m  ✓");
    assert_eq!(m.and_then(f).and_then(g), m.and_then(|x| f(x).and_then(g)));
    println!("   (M3) Kết hợp     : (m>>=f)>>=g == m>>=(x -> f(x)>>=g)  ✓");

    // ------------------------------------------------------------------
    // 6. TRAVERSABLE: đảo ngữ cảnh Vec<Result> -> Result<Vec>
    // ------------------------------------------------------------------
    println!("\n6. TRAVERSABLE — CÔNG CỤ BỊ BỎ QUÊN NHẤT CỦA RUST");
    let tot = vec!["10", "20", "30"];
    let hong = vec!["10", "hai mươi", "30"];

    let result_good: Result<Vec<i32>, _> = tot.iter().map(|s| s.parse::<i32>()).collect();
    let result_hong: Result<Vec<i32>, _> = hong.iter().map(|s| s.parse::<i32>()).collect();
    println!("   Vec<Result> -> Result<Vec> (tốt) : {:?}", result_good);
    println!("   Vec<Result> -> Result<Vec> (hỏng): có lỗi = {:?}", result_hong.is_err());

    let has_empty: Option<Vec<i32>> = vec![Some(1), None, Some(3)].into_iter().collect();
    let no_empty: Option<Vec<i32>> = vec![Some(1), Some(2)].into_iter().collect();
    println!("   Vec<Option> -> Option<Vec> (có None): {:?}", has_empty);
    println!("   Vec<Option> -> Option<Vec> (đủ)     : {:?}", no_empty);

    let lat: Option<Result<i32, String>> = Some(Ok(9));
    println!("   Option<Result> --transpose--> Result<Option>: {:?}", lat.transpose());

    // ------------------------------------------------------------------
    // 7. ALTERNATIVE: chuỗi phương án dự phòng
    // ------------------------------------------------------------------
    println!("\n7. ALTERNATIVE — CHUỖI PHƯƠNG ÁN DỰ PHÒNG");
    let missing_field: Option<&str> = None;
    let from_config_file: Option<&str> = Some("8080");
    let gate = missing_field.or(from_config_file).unwrap_or("3000");
    println!("   Cổng dùng: {} (biến môi trường -> tệp cấu hình -> mặc định)", gate);

    // ------------------------------------------------------------------
    // 8. SO SÁNH TRỰC DIỆN: MONAD NGẮN MẠCH vs APPLICATIVE TÍCH LŨY
    // ------------------------------------------------------------------
    println!("\n8. NGẮN MẠCH (Monad) vs TÍCH LŨY LỖI (Applicative)");
    let don_hong = DonTho {
        name: "An".into(),             // quá ngắn
        email: "an-tai-gmail".into(), // thiếu @
        age: "mười tám".into(),      // không phải số
    };

    println!("\n   [A] Dùng toán tử `?` (Monad — dừng ở lỗi đầu tiên):");
    match short_circuit_register(&don_hong) {
        Ok(nd) => println!("       Thành công: {:?}", nd),
        Err(e) => println!("       Báo về 1 lỗi duy nhất: {}", e),
    }

    println!("\n   [B] Dùng `XacThuc` (Applicative — gom hết lỗi):");
    match accumulator_register(&don_hong) {
        Auth::Dat(nd) => println!("       Thành công: {:?}", nd),
        Auth::Hong(error) => {
            println!("       Báo về {} lỗi cùng lúc:", error.len());
            for (i, l) in error.iter().enumerate() {
                println!("         {}. {}", i + 1, l);
            }
        }
    }

    println!("\n   [C] Đơn hợp lệ đi qua cả hai chiến lược:");
    let don_tot = DonTho {
        name: "Nguyễn Văn An".into(),
        email: "  An.Nguyen@Example.COM ".into(),
        age: " 28 ".into(),
    };
    println!("       Ngắn mạch: {:?}", short_circuit_register(&don_tot));
    println!("       Tích lũy : hợp lệ = {}", accumulator_register(&don_tot).is_set());

    println!("\n============================================================");
    println!("  map = SƠN TRONG HỘP · zip = GỘP HỘP · and_then = MỞ HỘP   ");
    println!("============================================================");
}

// ============================================================================
// KIỂM THỬ: BIẾN LUẬT FUNCTOR VÀ MONAD THÀNH TEST CHẠY ĐƯỢC
// ============================================================================

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn functor_identity_law() {
        for x in [Some(1i32), Some(-7), None] {
            assert_eq!(x.map(|a| a), x);
        }
    }

    #[test]
    fn functor_composition_law() {
        let f = |a: i32| a + 3;
        let g = |a: i32| a * 2;
        for x in [Some(0i32), Some(10), Some(-4), None] {
            assert_eq!(x.map(f).map(g), x.map(|a| g(f(a))));
        }
    }

    #[test]
    fn monad_left_and_right_identity() {
        let f = |n: i32| if n > 0 { Some(n * 2) } else { None };
        for a in [-3i32, 0, 5, 100] {
            assert_eq!(Some(a).and_then(f), f(a)); // M1
        }
        for m in [Some(1i32), None] {
            assert_eq!(m.and_then(Some), m); // M2
        }
    }

    #[test]
    fn monad_associativity() {
        let f = |n: i32| if n >= 0 { Some(n + 1) } else { None };
        let g = |n: i32| if n % 2 == 0 { Some(n / 2) } else { None };
        for m in [Some(-5i32), Some(0), Some(3), Some(8), None] {
            assert_eq!(
                m.and_then(f).and_then(g),
                m.and_then(|x| f(x).and_then(g)) // M3
            );
        }
    }

    #[test]
    fn bind_bang_map_roi_flatten() {
        let f = |n: i32| if n > 0 { Some(n * 10) } else { None };
        for x in [Some(4i32), Some(-1), None] {
            assert_eq!(x.and_then(f), x.map(f).flatten());
        }
    }

    #[test]
    fn traversable_swaps_contexts() {
        let tot: Result<Vec<i32>, _> = ["1", "2", "3"].iter().map(|s| s.parse::<i32>()).collect();
        assert_eq!(tot, Ok(vec![1, 2, 3]));

        let hong: Result<Vec<i32>, _> = ["1", "x", "3"].iter().map(|s| s.parse::<i32>()).collect();
        assert!(hong.is_err());

        let co_none: Option<Vec<i32>> = vec![Some(1), None].into_iter().collect();
        assert_eq!(co_none, None);

        let lat: Option<Result<i32, String>> = Some(Ok(9));
        assert_eq!(lat.transpose(), Ok(Some(9)));
    }

    #[test]
    fn applicative_collects_all_three_errors() {
        let don = DonTho {
            name: "An".into(),
            email: "khong-co-a-cong".into(),
            age: "abc".into(),
        };
        match accumulator_register(&don) {
            Auth::Hong(error) => {
                assert_eq!(error.len(), 3, "Phải gom đủ 3 lỗi, nhận được {:?}", error)
            }
            Auth::Dat(_) => panic!("Đơn hỏng mà lại được chấp nhận!"),
        }
    }

    #[test]
    fn monad_reports_only_first_error() {
        let don = DonTho {
            name: "An".into(),
            email: "khong-co-a-cong".into(),
            age: "abc".into(),
        };
        // Toán tử `?` dừng ngay ở lỗi đầu tiên: chỉ nhận được 1 thông báo.
        let error = short_circuit_register(&don).unwrap_err();
        assert!(error.contains("quá ngắn"), "Phải là lỗi ĐẦU TIÊN, nhận: {}", error);
    }

    #[test]
    fn valid_order_passes_both_strategies() {
        let don = DonTho {
            name: "Nguyễn Văn An".into(),
            email: " An.Nguyen@Example.COM ".into(),
            age: " 28 ".into(),
        };
        let mong_doi = User {
            name: "Nguyễn Văn An".to_string(),
            email: "an.nguyen@example.com".to_string(),
            age: 28,
        };
        assert_eq!(short_circuit_register(&don), Ok(mong_doi.clone()));
        assert_eq!(accumulator_register(&don), Auth::Dat(mong_doi));
    }

    #[test]
    fn the_generic_functor_works_for_three_types() {
        assert_eq!(Some(5i32).mapping(|x| x + 1), Some(6));
        assert_eq!(vec![1i32, 2, 3].mapping(|x| x * 10), vec![10, 20, 30]);
        let r: Result<i32, String> = Ok(7);
        assert_eq!(r.mapping(|x| x - 7), Ok(0));
    }
}
```

---

## Bảng tra cứu lỗi biên dịch & Cách khắc phục (Compiler Error Guide)

| Mã lỗi | Thông báo mẫu từ trình biên dịch | Nguyên nhân cốt lõi | Cách khắc phục nhanh |
|---|---|---|---|
| **E0308** | `expected 'Option<i32>', found 'Option<Option<i32>>'` | Bạn dùng `map` ở chỗ cần `and_then`: closure trả về một *hộp mới* nên hộp bị lồng nhau. | Đổi `.map(f)` thành `.and_then(f)`, hoặc giữ `.map(f)` rồi thêm `.flatten()`. |
| **E0277** | `the '?' operator can only be used in a function that returns 'Result' or 'Option'` | Bạn dùng `?` trong hàm trả về kiểu trần. Toán tử `?` là do-notation, nó phải "ở trong" một đơn nguyên. | Đổi kiểu trả về thành `Result<_, _>` / `Option<_>`, hoặc xử lý bằng `match` / `unwrap_or`. |
| **E0277** | `'?' couldn't convert the error to 'LoiCuaBan'` | `?` tự gọi `From::from` trên kiểu lỗi, nhưng bạn chưa cài `impl From<LoiGoc> for LoiCuaBan`. | Cài `impl From<...>`, hoặc chuyển thủ công bằng `.map_err(...)` ngay trước dấu `?`. |
| **E0282** | `type annotations needed` khi gọi `.collect()` | Trình biên dịch không biết bạn muốn `Vec<Result<T,E>>` hay `Result<Vec<T>,E>` — cả hai đều hợp lệ! | Ghi rõ kiểu: `let x: Result<Vec<i32>, _> = ...` hoặc dùng turbofish `.collect::<Result<Vec<_>, _>>()`. |
| **E0599** | `no method named 'flatten' found` | `flatten` có trên `Option<Option<T>>` và `Iterator`; với `Result` thì hai kiểu lỗi phải trùng nhau. | Thống nhất kiểu lỗi trước, hoặc dùng `.and_then(|x| x)`. |

### Phân tích lỗi thực tế `E0308` (dùng nhầm `map` thay cho `and_then`):

```rust
fn tim_tuoi(s: &str) -> Option<u32> {
    s.trim().parse::<u32>().ok()
}

// ❌ Sai: closure trả về Option nên kết quả bị LỒNG hai lớp
// fn sai(input: Option<&str>) -> Option<u32> {
//     input.map(|s| tim_tuoi(s))
//     // LỖI E0308: expected `Option<u32>`, found `Option<Option<u32>>`
// }

// ✅ Cách 1: dùng and_then (bind) — closure trả về hộp thì dùng bind
fn dung_1(input: Option<&str>) -> Option<u32> {
    input.and_then(tim_tuoi)
}

// ✅ Cách 2: giữ map rồi flatten (join) — hoàn toàn tương đương
fn dung_2(input: Option<&str>) -> Option<u32> {
    input.map(tim_tuoi).flatten()
}
```

**Quy tắc nhớ đời**: nhìn vào closure bạn truyền vào.
- Closure trả về **giá trị trần** (`A -> B`) → dùng **`map`**.
- Closure trả về **một chiếc hộp** (`A -> Option<B>` / `A -> Result<B,E>`) → dùng **`and_then`**.

---

## Tóm tắt chương & Bài tập rèn luyện (Summary & Exercises)

### 4 Điểm cốt lõi cần ghi nhớ:
1. **Ba tầng trừu tượng, ba câu hỏi khác nhau**:
   - *Functor* (`map`): "sơn lại ruột hộp" — các bước không biết gì về nhau.
   - *Applicative* (`zip`, `Auth`): "gộp nhiều hộp độc lập" — gom được **tất cả** lỗi.
   - *Monad* (`and_then`, `?`): "mở hộp rồi mới quyết định bước sau" — dừng ở lỗi **đầu tiên**.
2. **`and_then` chính là `bind`, `flatten` chính là `join`, `?` chính là do-notation.** Bạn đã dùng monad từ Chương 11; chương này chỉ đặt đúng tên và chỉ ra các luật.
3. **`collect::<Result<Vec<_>, E>>()` và `transpose()` là hai công cụ đắt giá nhất chương.** Chúng biến `Vec<Result>` thành `Result<Vec>` — thao tác mà gần như mọi chương trình đọc dữ liệu ngoài đều cần.
4. **Rust có nhiều monad cụ thể nhưng chưa có trait `Monad` tổng quát**, vì thiếu Kiểu bậc cao (HKT). Thư viện `fp-core.rs` mô phỏng HKT bằng kiểu liên kết, và GAT (ổn định từ Rust 1.65) đang dần thu hẹp khoảng cách.

> **Còn những cấu trúc nào nữa?** Chương này dạy Functor, Bifunctor, Apply/Applicative, Traversable, Chain/Monad và Alternative. Đặc tả Fantasy Land còn 11 cấu trúc khác — trong đó đáng chú ý nhất là **ChainRec** (vòng lặp đơn nguyên không tràn ngăn xếp), **Comonad** (đối ngẫu của Monad) và **Profunctor** (nền tảng của Lens). Tất cả có tại **[Phụ lục A](./PHU_LUC_A_FANTASY_LAND.md)**, kèm mã chạy được và bài kiểm chứng luật.

### Bài tập rèn luyện tự giải:

**Bài tập 1 (`map` hay `and_then`?)**
Cho ba hàm dưới đây, hãy xây một chuỗi xử lý từ `Option<&str>` ra `Option<String>` và giải thích tại sao mỗi bước bạn chọn `map` hoặc `and_then`:
```rust
fn cat(s: &str) -> String;              // luôn thành công
fn thanh_so(s: String) -> Option<u32>;  // có thể thất bại
fn dinh_dang(n: u32) -> String;         // luôn thành công
```

<details>
<summary><b>Gợi ý</b></summary>

Nhìn kiểu trả về của từng hàm: hàm nào trả `Option<...>` thì phải nối bằng `and_then`; hàm nào trả giá trị trần thì nối bằng `map`. Nếu bạn dùng nhầm, `rustc` sẽ báo `Option<Option<...>>`.
</details>

<details>
<summary><b>Lời giải</b></summary>

```rust
fn cat(s: &str) -> String { s.trim().to_string() }
fn thanh_so(s: String) -> Option<u32> { s.parse::<u32>().ok() }
fn dinh_dang(n: u32) -> String { format!("{} đồng", n) }

fn handle(input: Option<&str>) -> Option<String> {
    input
        .map(cat)           // cat trả String (trần)       -> map
        .and_then(thanh_so) // thanh_so trả Option (hộp)   -> and_then
        .map(dinh_dang)     // dinh_dang trả String (trần) -> map
}

fn main() {
    assert_eq!(handle(Some("  1500 ")), Some("1500 đồng".to_string()));
    assert_eq!(handle(Some("  abc ")), None);
    assert_eq!(handle(None), None);
}
```
</details>

**Bài tập 2 (Traversable trong thực chiến)**
Cho một lát cắt `&[&str]` chứa các dòng cấu hình dạng `"khoa=value"`. Viết hàm `doc_cau_hinh(dong: &[&str]) -> Result<HashMap<String, String>, String>` sao cho: nếu **mọi** dòng đều hợp lệ thì trả về bảng cấu hình; nếu **bất kỳ** dòng nào thiếu dấu `=` thì trả lỗi kèm nội dung dòng sai. Yêu cầu: dùng `collect()` chứ không dùng vòng lặp `for` với biến `mut`.

<details>
<summary><b>Gợi ý</b></summary>

`HashMap<K, V>` cài đặt `FromIterator<(K, V)>`, và `Result` cũng cài `FromIterator`. Vì vậy `Result<HashMap<_,_>, E>` thu được trực tiếp từ một iterator các `Result<(String, String), E>`. Dùng `split_once('=')` để tách.
</details>

<details>
<summary><b>Lời giải</b></summary>

```rust
use std::collections::HashMap;

fn doc_cau_hinh(dong: &[&str]) -> Result<HashMap<String, String>, String> {
    dong.iter()
        .map(|d| {
            d.split_once('=')
                .map(|(k, v)| (k.trim().to_string(), v.trim().to_string()))
                .ok_or_else(|| format!("Dòng cấu hình sai định dạng: {:?}", d))
        })
        .collect() // ← Traversable: Iterator<Result<(K,V),E>> -> Result<HashMap<K,V>, E>
}

fn main() {
    let tot = ["cong = 8080", "host=localhost"];
    let bang = doc_cau_hinh(&tot).unwrap();
    assert_eq!(bang.get("cong"), Some(&"8080".to_string()));

    let hong = ["cong = 8080", "dong sai khong co dau bang"];
    assert!(doc_cau_hinh(&hong).is_err());
    println!("{:?}", doc_cau_hinh(&hong));
}
```

Chỉ **một** lời gọi `collect()` đã làm cả ba việc: duyệt, đảo ngữ cảnh `Result` ra ngoài, và dựng `HashMap`. Đó là sức mạnh của Traversable kết hợp `FromIterator`.
</details>

**Bài tập 3 (Tư duy thiết kế: chọn Applicative hay Monad?)**
Với mỗi tình huống dưới đây, hãy quyết định nên dùng **Applicative** (gom hết lỗi) hay **Monad** (dừng sớm), và giải thích:
1. Xác thực 8 trường của biểu mẫu đăng ký người dùng.
2. Quy trình đặt hàng: kiểm tra tồn kho → trừ tiền → tạo vận đơn.
3. Đọc 5 tệp cấu hình độc lập lúc khởi động máy chủ.
4. Xác thực đăng nhập → lấy quyền hạn → kiểm tra quyền truy cập tài nguyên.

<details>
<summary><b>Gợi ý</b></summary>

Câu hỏi duy nhất cần trả lời cho mỗi tình huống: **bước sau có cần kết quả của bước trước không?** Nếu không cần → độc lập → Applicative. Nếu cần → tuần tự → Monad.
</details>

<details>
<summary><b>Lời giải tham khảo</b></summary>

1. **Applicative.** Tám trường độc lập hoàn toàn. Người dùng cần thấy tất cả lỗi trong một lần gửi, không phải sửa tám vòng.
2. **Monad.** Không được trừ tiền nếu chưa biết còn hàng; không được tạo vận đơn nếu chưa trừ tiền thành công. Bước sau phụ thuộc bước trước → dùng `?`. Hơn nữa, "gom hết lỗi" ở đây là **nguy hiểm**: nó ngụ ý bạn đã thực hiện các bước sau dù bước trước đã hỏng.
3. **Applicative.** Năm tệp độc lập. Báo hết một lượt "thiếu tệp A, tệp C sai cú pháp" giúp người vận hành sửa một lần rồi khởi động lại, thay vì lặp năm vòng.
4. **Monad.** Không có danh tính thì không có quyền hạn; không có quyền hạn thì không kiểm tra được truy cập. Ngoài ra, dừng sớm ở đây còn là yêu cầu **bảo mật**: đừng làm lộ thông tin về tài nguyên cho người chưa xác thực.

**Nguyên tắc tổng quát**: *Applicative cho dữ liệu (song song, gom lỗi) — Monad cho quy trình (tuần tự, dừng sớm).*
</details>
