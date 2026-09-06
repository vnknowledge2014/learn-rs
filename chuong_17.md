# Chương 17: Hàm bậc cao & Mẫu thiết kế lập trình hàm trong Rust (Higher-Order Functions & Functional Design Patterns)

## Giới thiệu & Mục tiêu học tập

Sau khi đã làm chủ Hàm ẩn danh (Closures) ở Chương 15 và Bộ lặp (Iterators) ở Chương 16, chúng ta đã thu thập đủ các mảnh ghép nền tảng của Lập trình hàm trong Rust. Giờ là lúc nâng tầm tư duy kiến trúc phần mềm bằng cách kết hợp chúng lại thành một bức tranh hoàn chỉnh thông qua **Hàm bậc cao (Higher-Order Functions - HOFs)** và các **Mẫu thiết kế hàm (Functional Design Patterns)**.

Trong các ngôn ngữ lập trình truyền thống, hàm thường chỉ được xem như một "khối mã cố định" nhận dữ liệu (số nguyên, chuỗi ký tự) và trả về dữ liệu. Nhưng trong Rust và trường phái Lập trình hàm, **Hàm được coi là một công dân hạng nhất (First-Class Citizens)**:
- Hàm có thể được truyền vào như một tham số của một hàm khác.
- Hàm có thể được sinh ra và trả về từ một hàm khác.
- Hàm có thể được đóng gói vào các cấu trúc dữ liệu hoặc con trỏ thông minh (smart pointer).

Khả năng này mở ra cánh cửa cho phong cách thiết kế **Bộ kết hợp (Combinators)** trên `Option` và `Result`. Thay vì phải viết các cấu trúc `match` hoặc `if let` lồng nhau 5-7 tầng (thường được gọi là "Kim tự tháp tử thần" - *Pyramid of Doom*), bạn có thể xâu chuỗi toàn bộ logic xử lý và kiểm tra lỗi thành một đường ống (pipeline) mạch lạc, phẳng phiu và an toàn tuyệt đối.

Mục tiêu học tập của chương này:
- Nắm vững khái niệm **Hàm bậc cao (Higher-Order Functions)**: Nhận hàm làm tham số hoặc trả về hàm mới.
- Phân biệt sự khác nhau giữa **Con trỏ hàm thuần túy (`fn`)** và **Giao ước Closure (`Fn`/`FnMut`/`FnOnce`)**.
- Làm chủ kỹ thuật trả về closure từ hàm thông qua **`impl Fn(...)`** (phân phối tĩnh - static dispatch) và con trỏ thông minh (smart pointer) **`Box<dyn Fn(...)>`** (phân phối động - dynamic dispatch).
- Xóa bỏ "Kim tự tháp tử thần" (*Pyramid of Doom*) bằng các phương thức Combinator trên `Option` và `Result`: **`map`**, **`and_then`**, **`or_else`**, **`unwrap_or_else`**.
- Xây dựng các mẫu thiết kế thực chiến: Mẫu đóng gói đo lường (Decorator/Wrapper Pattern), Mẫu xưởng chế tạo bộ kiểm tra (Factory Pattern), và Chuỗi biến đổi xác thực (Validation Pipeline).

---

## Hình tượng hóa đời sống (Intuitive Everyday Analogy)

Để thấu cảm vẻ đẹp của Hàm bậc cao và Bộ kết hợp dữ liệu, hãy cùng quan sát hai hình tượng sống động trong thực tế:

```
┌──────────────────────────────────────────────────────────────────────────────────┐
│              HÌNH TƯỢNG ĐỜI SỐNG: HÀM BẬC CAO VÀ DÂY CHUYỀN DOMINO               │
├────────────────────────────────────────┬─────────────────────────────────────────┤
│    XƯỞNG LẮP RÁP CÁNH TAY ROBOT        │        HIỆU ỨNG DOMINO TỰ PHỤC HỒI      │
│       (Higher-Order Functions)         │         (Combinators: and_then)         │
│                                        │                                         │
│ - Nhà máy không đóng đinh 1 chức năng! │ ┌───┐  ┌───┐  ┌───┐  ┌───┐              │
│ - Đầu chuyền là khung gầm xe rỗng      │ │ 1 │─►│ 2 │─►│ 3 │─►│ 4 │ (Đích đến)   │
│ - Quản đốc lắp "Cánh tay robot hàn"    │ └───┘  └───┘  └───┘  └───┘              │
│   -> Nhà máy biến thành xưởng hàn!     │                                         │
│ - Hôm sau đổi thành "Robot phun sơn"   │ * QUY TẮC DÂY CHUYỀN AN TOÀN:           │
│   -> Nhà máy biến thành xưởng sơn!     │ - Nếu quân số 2 bị đổ hỏng (None/Err):   │
│ - Hàm bậc cao chính là nhà máy:        │   Quân số 3 và 4 lập tức đứng yên!      │
│   Hành vi thực sự do công cụ đưa vào!  │ - Toàn bộ chuỗi dừng lại êm đẹp,        │
│                                        │   không làm đổ vỡ đồ đạc xung quanh!    │
└────────────────────────────────────────┴─────────────────────────────────────────┘
```

### 1. Nhà máy lắp ráp với các cánh tay robot tháo rời (Hàm bậc cao)
- Hãy tưởng tượng một nhà máy chế tạo hiện đại:
  - Thay vì xây dựng 10 nhà máy riêng biệt: một nhà máy chỉ chuyên hàn, một nhà máy chỉ chuyên bắt ốc, một nhà máy chỉ chuyên dán nhãn...
  - Chủ nhà máy xây dựng một **Khung chuyền vạn năng** (Hàm bậc cao). Khung chuyền này có một ổ cắm tiêu chuẩn.
  - Khi cần hàn khung xe, người kỹ sư gắn chiếc đầu "Robot hàn" vào ổ cắm.
  - Khi cần sơn bóng, người kỹ sư tháo đầu hàn ra và gắn đầu "Robot phun sơn" vào.
- Trong lập trình, hàm bậc cao chính là chiếc khung chuyền vạn năng đó! Nó không tự quyết định làm gì cụ thể với dữ liệu, mà nó để **bạn truyền hành vi (hàm/closure con) vào** tại thời điểm gọi.

### 2. Trò chơi Domino dây chuyền tự phục hồi (Bộ kết hợp Combinators)
- Bạn xếp một dãy 10 quân cờ Domino nối tiếp nhau trên sàn:
  - Quân thứ nhất ngã sẽ chạm vào quân thứ hai, quân thứ hai chạm vào quân thứ ba (`.and_then()`).
  - Nếu ở bước thứ hai, quân cờ bị gãy hoặc mất tích (đại diện cho trạng thái `None` hoặc lỗi `Err`), dây chuyền domino tự động dừng lại một cách văn minh.
  - Bạn không cần phải cử một người đứng canh ở từng quân cờ để hô: "Nếu quân 1 ngã thì chạy sang quân 2, nếu quân 2 ngã thì chạy sang quân 3" (đó là cách viết `match` lồng nhau nhức đầu). Toàn bộ chuỗi tự động điều phối dòng chảy một cách trơn tru!

---

## Khái niệm & Cơ chế kỹ thuật chuyên sâu (In-Depth Technical Mechanics)

### 1. Phân biệt Con trỏ hàm (`fn`) và Giao ước Closure (`Fn`/`FnMut`/`FnOnce`)

Trong Rust, có sự khác biệt tinh tế giữa con trỏ hàm thuần túy và closure:

1. **Con trỏ hàm (`fn`)**:
   - Viết bằng chữ thường `fn(...) -> ...`.
   - Là một địa chỉ ô nhớ trỏ trực tiếp đến đoạn mã máy của một hàm độc lập khai báo bằng `fn`.
   - **Không thể bắt giữ môi trường xung quanh!** Kích thước của nó trên thanh RAM luôn bằng đúng kích thước của một con trỏ (8 bytes trên hệ điều hành 64-bit).
2. **Giao ước Closure (`Fn`, `FnMut`, `FnOnce`)**:
   - Viết hoa chữ cái đầu: `F: Fn(...) -> ...`.
   - Cho phép bắt giữ các biến xung quanh vào một `struct` nội bộ tự sinh.
   - Bất kỳ con trỏ hàm `fn` nào cũng tự động thỏa mãn giao ước `Fn`, `FnMut`, và `FnOnce` (vì nó không bắt giữ gì, coi như struct rỗng).

```rust
fn doubled(x: i32) -> i32 { x * 2 }

// Hàm bậc high nhận con trỏ hàm fn thuần túy
fn ap_dung_fn(pointer: fn(i32) -> i32, value: i32) -> i32 {
    pointer(value)
}

// Hàm bậc high nhận Trait Bound tổng quát (chấp nhận CẢ fn VÀ closure)
fn ap_dung_generic<F: Fn(i32) -> i32>(hanh_dong: F, value: i32) -> i32 {
    hanh_dong(value)
}
```

*Quy tắc thực chiến*: Luôn ưu tiên viết hàm nhận `F: Fn(...)` (hoặc `FnMut`, `FnOnce`) vì nó linh hoạt gấp bội: Người dùng có thể truyền vào một hàm thông thường hoặc một closure đều được!

### 2. Kỹ thuật trả về Closure từ Hàm

Đôi khi bạn muốn viết một hàm đóng vai trò như một "nhà máy" sản xuất ra các closure theo yêu cầu (Factory Pattern):

#### Cách 1: Phân phối tĩnh với `impl Fn(...)` (Static Dispatch - Không tốn chi phí)
Nếu hàm của bạn chỉ trả về một loại closure duy nhất:
```rust
// Trả về một closure cụ thể, rustc tự xác định kiểu và kích thước trên Stack
fn tao_bo_nhan(he_so: i32) -> impl Fn(i32) -> i32 {
    move |x| x * he_so // Bắt buộc dùng move để đóng gói he_so vào closure
}
```

#### Cách 2: Phân phối động với Con trỏ thông minh `Box<dyn Fn(...)>` (Dynamic Dispatch)
Khi hàm của bạn có thể trả về **hai closure khác nhau** tùy theo điều kiện `if/else`:
Vì mỗi closure trong Rust có một kiểu ẩn danh duy nhất không trùng lặp, bạn không thể dùng `impl Fn` trong hai nhánh `if/else` khác nhau. Lúc này, ta phải đóng gói chúng vào con trỏ thông minh (smart pointer) `Box` trên vùng nhớ Heap:
```rust
fn make_converter(la_viet_hoa: bool) -> Box<dyn Fn(&str) -> String> {
    if la_viet_hoa {
        Box::new(|s| s.to_uppercase())
    } else {
        Box::new(|s| s.to_lowercase())
    }
}
```

### 3. Phá vỡ "Kim tự tháp tử thần" với Bộ kết hợp (Combinators)

Hãy quan sát đoạn mã xử lý dữ liệu người dùng khi viết bằng `match` lồng nhau truyền thống:

```rust
// ❌ CÁCH VIẾT CỒNG KỀNH (Pyramid of Doom):
fn parse_age_imperative(series: Option<&str>) -> Option<u32> {
    match series {
        Some(s) => {
            let cut_range_state = s.trim();
            if !cut_range_state.is_empty() {
                match cut_range_state.parse::<u32>() {
                    Ok(age) => {
                        if age >= 18 { Some(age) } else { None }
                    },
                    Err(_) => None,
                }
            } else {
                None
            }
        },
        None => None,
    }
}
```
Khối mã trên lồng nhau 4 cấp, đọc rất mỏi mắt và rất dễ bỏ sót nhánh rẽ.

Giờ hãy chiêm ngưỡng vẻ đẹp của **Bộ kết hợp Combinators** trong lập trình hàm:

```rust
// ✅ CÁCH VIẾT PHẲNG PHIU THEO PHONG CÁCH ĐƯỜNG ỐNG (FP Combinators):
fn parse_age_idiomatic(series: Option<&str>) -> Option<u32> {
    series
        .map(|s| s.trim())                      // 1. Cắt tỉa khoảng trắng
        .filter(|s| !s.is_empty())              // 2. Lọc chuỗi không rỗng
        .and_then(|s| s.parse::<u32>().ok())   // 3. Phân tích chuỗi thành số (bỏ qua lỗi)
        .filter(|&age| age >= 18)             // 4. Chỉ nhận người từ 18 tuổi trở lên
}
```
Mã nguồn giờ đây chảy thẳng từ trên xuống dưới như một dòng suối tự nhiên. Mọi trường hợp `None` hay lỗi ngầm đều được Rust tự động xử lý và lan truyền ngắn mạch (short-circuiting)!

---

### 4. Đặt đúng tên cho `and_then`: đó là phép `bind`

Trước khi đi tiếp, hãy ghi nhớ một điều sẽ được khai triển đầy đủ ở **Chương 19**:

> Phương thức `and_then` mà bạn vừa dùng để "phá kim tự tháp tử thần" có một cái tên chính thức trong toàn bộ thế giới lập trình hàm: **`bind`**. Và `map` cộng `bind` chính là định nghĩa của một **Đơn nguyên (Monad)**.

Quy tắc phân biệt chỉ gồm một câu:

| Closure bạn truyền vào trả về gì? | Dùng cái gì |
|---|---|
| Giá trị **trần** (`A -> B`) | **`map`** |
| Một **chiếc hộp mới** (`A -> Option<B>` hoặc `A -> Result<B,E>`) | **`and_then`** |

Nếu chọn nhầm, trình biên dịch báo `Option<Option<T>>` — hộp lồng trong hộp. Đó là tín hiệu rõ ràng nhất rằng bạn cần `and_then` chứ không phải `map`.

### 5. Lập trình hai đường ray (Railway-Oriented Programming)

Đây là mô hình tinh thần giúp gắn tất cả các bộ kết hợp bạn vừa học thành **một bức tranh duy nhất**. Nó đến từ cuốn *Domain Modeling Made Functional*, và một khi đã thấy, bạn sẽ không bao giờ nhìn `Result` như cũ nữa.

Hãy hình dung chương trình của bạn là một tuyến **đường sắt hai ray**:

```
                    ĐƯỜNG RAY THÀNH CÔNG (Ok)
   ──────────●───────────────●───────────────●──────────────►  Ok(kết quả)
             │ ↘             │ ↘             │ ↘
             │  ↘ (rẽ ghi)   │  ↘            │  ↘
             ▼   ↘           ▼   ↘           ▼   ↘
   ──────────────────────────────────────────────────────────►  Err(lỗi)
                    ĐƯỜNG RAY THẤT BẠI (Err)

   · Đoàn tàu khởi hành trên ray THÀNH CÔNG.
   · Mỗi bước xử lý là một "ghi tàu": hoặc đi thẳng, hoặc bẻ lái sang ray THẤT BẠI.
   · Một khi đã sang ray thất bại, tàu CHẠY THẲNG tới đích, KHÔNG BƯỚC NÀO CÒN CHẠY NỮA.
   · Đó chính xác là hành vi của toán tử `?` và của `and_then`.
```

Vấn đề thực tế: các hàm bạn có trong tay **không cùng một hình dạng**. Muốn nối chúng lên đường ray, phải "nâng cấp" từng loại. DMMF phân chúng thành bốn nhóm, và Rust có sẵn công cụ tương ứng cho mỗi nhóm:

| Nhóm hàm | Hình dạng | Ví dụ | Công cụ để nối lên đường ray |
|---|---|---|---|
| **Hàm ghi tàu** (switch) | `A -> Result<B, E>` | `validate_email` | **`.and_then(f)`** — nối thẳng, đây là dạng chuẩn |
| **Hàm một ray** (one-track) | `A -> B` | `s.to_uppercase()` | **`.map(f)`** — nâng lên ray thành công |
| **Hàm cụt** (dead-end) | `&A -> ()` | `ghi_nhat_ky(&don)` | **`.inspect(f)`** — chạy tác dụng phụ rồi trả nguyên giá trị |
| **Hàm có thể panic** | `A -> B` (nhưng sập được) | thư viện C qua FFI | `std::panic::catch_unwind` rồi `.map_err(...)` |

Và hai công cụ nữa để làm việc với **ray thất bại**:
- **`.map_err(f)`** — đổi *kiểu* lỗi khi đi từ tầng dưới lên tầng trên (đây là "chân thứ hai" của Bifunctor, Chương 19).
- **`.or_else(f)`** — thử **quay lại ray thành công**: dùng cho cơ chế dự phòng (đọc bộ nhớ đệm hỏng thì đọc cơ sở dữ liệu).

Ví dụ hoàn chỉnh — một đường ray đọc và xử lý cấu hình:

```rust
fn handle(tho: &str) -> Result<u16, LoiCauHinh> {
    read_value(tho)                                    // A -> Result<B,E>  : and_then dạng gốc
        .map(|s| s.trim().to_string())                  // hàm MỘT RAY       : map
        .and_then(|s| phan_tich_cong(&s))               // hàm GHI TÀU       : and_then
        .inspect(|gate| println!("Cổng hợp lệ: {}", gate)) // hàm CỤT        : inspect
        .map_err(LoiCauHinh::tu_loi_doc)                // đổi kiểu lỗi      : map_err
        .or_else(|_| Ok(8080))                          // phương án dự phòng: or_else
}
```

Toàn bộ đường ống này **phẳng phiu, đọc từ trên xuống**, không một tầng lồng nhau nào — trong khi vẫn xử lý đầy đủ mọi nhánh lỗi.

> **Vì sao mô hình này quan trọng?** Vì nó cho bạn một câu hỏi duy nhất để tự hỏi mỗi khi bí: *"Hàm tiếp theo của tôi thuộc nhóm nào trong bốn nhóm trên?"* Trả lời được câu đó là biết ngay phải dùng `map`, `and_then`, `inspect` hay `map_err`. Đây là chỗ lý thuyết ở Chương 19 gặp thực chiến ở Chương 20.

---

## Mã nguồn minh họa thực chiến (Idiomatic Runnable Rust Blueprint)

Dưới đây là một chương trình hoàn chỉnh xây dựng **Hệ thống Xác thực & Kiểm tra Hồ sơ Người Dùng (User Profile Sanitization & Validation Engine)**. Chương trình kết hợp:
1. Hàm bậc cao đo lường thời gian thực thi (Decorator Pattern).
2. Hàm xưởng sản xuất bộ lọc tùy biến (Closure Factory).
3. Chuỗi xử lý đường ống combinators trên `Option` và `Result` phẳng phiu, an toàn tuyệt đối.

```rust
// Tệp: src/main.rs
// Chương trình thực chiến làm chủ Higher-Order Functions & Functional Patterns trong Rust

use std::time::Instant;

// ============================================================================
// ĐỊNH NGHĨA DỮ LIỆU ĐẦU VÀO VÀ ĐẦU RA
// ============================================================================

#[derive(Debug, Clone, PartialEq)]
pub struct RawProfile {
    pub name_dang_import: Option<String>,
    pub email: Option<String>,
    pub age_series: Option<String>,
}

#[derive(Debug, Clone, PartialEq)]
pub struct ValidProxy {
    pub name_dang_import: String,
    pub email: String,
    pub age: u32,
}

// ============================================================================
// 1. HÀM BẬC CAO: ĐO LƯỜNG THỜI GIAN VÀ GHI NHẬT KÝ KIỂM TOÁN (WRAPPER PATTERN)
// ============================================================================

/// Hàm bậc high nhận vào tên tác vụ và một hành động F bất kỳ
/// Thực hiện đo thời gian thực thi của hành động đó và trả về kết quả nguyên bản
///
/// LƯU Ý VỀ TÍNH THUẦN TÚY: bản thân hàm này KHÔNG thuần túy — nó đọc đồng hồ
/// hệ thống (`Instant::now()`) và in ra màn hình, nên gọi hai lần cho hai kết quả
/// khác nhau. Đó là chủ ý: đo lường và ghi nhật ký là tác dụng phụ chính đáng,
/// nhưng chúng phải nằm ở TẦNG VỎ, bao bên ngoài phần lõi thuần túy.
/// Đây chính là kiến trúc "lõi thuần túy - vỏ mệnh lệnh" sẽ học kỹ ở Chương 20.
pub fn measure_exec_time<F, T>(ten_tac_vu: &str, hanh_dong: F) -> T
where
    F: FnOnce() -> T,
{
    println!(">>> [KIỂM TOÁN] Bắt đầu thực thi: {}", ten_tac_vu);
    let timestamp_start = Instant::now();
    
    // Gọi hàm/closure được truyền vào
    let ket_qua = hanh_dong();
    
    let range_time_time = timestamp_start.elapsed();
    println!(">>> [KIỂM TOÁN] Hoàn thành '{}' trong: {:?}", ten_tac_vu, range_time_time);
    ket_qua
}

// ============================================================================
// 2. HÀM XƯỞNG SẢN XUẤT CLOSURE (FACTORY PATTERN)
// ============================================================================

/// Tạo ra một closure kiểm tra xem một chuỗi có chứa từ cấm hay không
/// Sử dụng `move` để đóng gói danh sách từ cấm vào struct vô danh của closure
pub fn make_ban_filter(danh_sach_tu_cam: Vec<&'static str>) -> impl Fn(&str) -> bool {
    move |van_ban: &str| {
        let lowercase = van_ban.to_lowercase();
        // Trả về true nếu KHÔNG chứa bất kỳ từ cấm nào
        !danh_sach_tu_cam.iter().any(|&tu| lowercase.contains(tu))
    }
}

/// Tạo ra một closure kiểm tra độ dài tối thiểu và tối đa của chuỗi
pub fn make_unit_check_do_long(min: usize, max: usize) -> impl Fn(&str) -> bool {
    move |van_ban: &str| {
        let length = van_ban.trim().chars().count();
        length >= min && length <= max
    }
}

// ============================================================================
// 3. ĐƯỜNG ỐNG XÁC THỰC BẰNG BỘ KẾT HỢP COMBINATORS (PIPELINE PATTERN)
// ============================================================================

pub fn auth_proxy_num(
    profile: &RawProfile,
    check_name: &impl Fn(&str) -> bool,
    check_banned_words: &impl Fn(&str) -> bool,
) -> Result<ValidProxy, &'static str> {
    // 1. Xác thực và chuẩn hóa Tên đăng nhập bằng chuỗi combinators
    let name_hop_le = profile
        .name_dang_import
        .as_deref()                                   // Option<String> -> Option<&str>
        .map(|s| s.trim())                            // Cắt khoảng trắng
        .filter(|s| check_name(s))                  // Kiểm tra độ dài hợp lệ
        .filter(|s| check_banned_words(s))              // Kiểm tra từ cấm
        .map(|s| s.to_string())
        .ok_or("Tên đăng nhập không hợp lệ hoặc chứa từ cấm!")?; // Lan truyền lỗi phẳng phiu

    // 2. Xác thực và chuẩn hóa Email
    let email_hop_le = profile
        .email
        .as_deref()
        .map(|s| s.trim())
        .filter(|s| s.contains('@') && s.contains('.')) // Điều kiện email cơ bản
        .map(|s| s.to_lowercase())                      // Viết thường toàn bộ email
        .ok_or("Địa chỉ Email sai định dạng!")?;

    // 3. Xác thực và chuẩn hóa Tuổi
    let age_hop_le = profile
        .age_series
        .as_deref()
        .map(|s| s.trim())
        .and_then(|s| s.parse::<u32>().ok())           // Phân tích chuỗi sang u32
        .filter(|&age| (16..=100).contains(&age))    // Giới hạn độ tuổi từ 16 đến 100
        .ok_or("Độ tuổi phải là số nguyên từ 16 đến 100!")?;

    // Trả về cấu trúc hồ sơ đã được tinh chế sạch sẽ
    Ok(ValidProxy {
        name_dang_import: name_hop_le,
        email: email_hop_le,
        age: age_hop_le,
    })
}

// ============================================================================
// CHƯƠNG TRÌNH THỰC THI CHÍNH
// ============================================================================

fn main() {
    println!("============================================================");
    println!("   HỆ THỐNG XÁC THỰC HỒ SƠ: HÀM BẬC CAO & COMBINATORS FP    ");
    println!("============================================================");

    // Khởi tạo các cỗ máy kiểm tra từ xưởng Factory
    let check_do_long_name = make_unit_check_do_long(4, 15);
    let check_banned_words = make_ban_filter(vec!["admin", "root", "lua_dao"]);

    // Dữ liệu mẫu 1: Hồ sơ chuẩn mực hoàn hảo
    let proxy_num_standard = RawProfile {
        name_dang_import: Some(String::from("  nguyen_an  ")),
        email: Some(String::from("An.Nguyen@EXAMPLE.COM  ")),
        age_series: Some(String::from("  22  ")),
    };

    // Dữ liệu mẫu 2: Hồ sơ lỗi chứa từ cấm và email hỏng
    let proxy_num_error = RawProfile {
        name_dang_import: Some(String::from("super_admin")), // Chứa từ cấm 'admin'
        email: Some(String::from("email_khong_hop_le")),
        age_series: Some(String::from("12")),             // Dưới 16 tuổi
    };

    // 1. Kiểm tra hồ sơ chuẩn với hàm bậc high đo thời gian
    println!("\n--- TIẾN HÀNH XỬ LÝ HỒ SƠ THỨ NHẤT ---");
    let ket_qua_1 = measure_exec_time("Xử lý Hồ sơ Hợp lệ", || {
        auth_proxy_num(&proxy_num_standard, &check_do_long_name, &check_banned_words)
    });

    match ket_qua_1 {
        Ok(profile) => {
            println!("[THÀNH CÔNG] Dữ liệu sau khi làm sạch:");
            println!("  - Tên đăng nhập: {}", profile.name_dang_import);
            println!("  - Email hợp chuẩn: {}", profile.email);
            println!("  - Tuổi: {}", profile.age);
        }
        Err(error) => println!("[THẤT BẠI] Lỗi: {}", error),
    }

    // 2. Kiểm tra hồ sơ lỗi
    println!("\n--- TIẾN HÀNH XỬ LÝ HỒ SƠ THỨ HAI (CÓ LỖI) ---");
    let ket_qua_2 = measure_exec_time("Xử lý Hồ sơ Vi phạm", || {
        auth_proxy_num(&proxy_num_error, &check_do_long_name, &check_banned_words)
    });

    match ket_qua_2 {
        Ok(_) => println!("[LỖI KHÔNG MONG MUỐN] Hồ sơ vi phạm lại lọt qua!"),
        Err(ly_do) => println!("[CHẶN THÀNH CÔNG] Hệ thống từ chối vì: '{}'", ly_do),
    }

    println!("\n============================================================");
    println!("     XÂY DỰNG PIPELINE HÀM BẬC CAO HOÀN THÀNH XUẤT SẮC      ");
    println!("============================================================");
}
```

---

## Bảng tra cứu lỗi biên dịch & Cách khắc phục (Compiler Error Guide)

Khi thiết kế các hàm bậc cao và chuỗi combinators trong Rust, lập trình viên thường bắt gặp các lỗi biên dịch về kiểu dữ liệu và kích thước bộ nhớ:

| Mã lỗi | Thông báo mẫu từ trình biên dịch | Nguyên nhân cốt lõi | Cách khắc phục nhanh |
|---|---|---|---|
| **E0308** | `mismatched types: expected closure, found a different closure` | Trong Rust, mỗi closure có một kiểu vô danh độc nhất vô nhị. Dù hai closure có cùng chữ ký `|x| x + 1`, chúng vẫn là 2 kiểu khác nhau. Bạn không thể gán chúng cho cùng một biến mà không dùng Box. | Sử dụng con trỏ thông minh (smart pointer) `Box<dyn Fn(...)>` nếu cần chứa các closure khác nhau vào cùng một tập hợp hoặc nhánh rẽ `if/else`. |
| **E0277** | `the size for values of type 'dyn Fn()' cannot be known at compilation time` | Bạn cố gắng trả về `dyn Fn()` trực tiếp hoặc lưu nó trên Stack. Kiểu Trait Object không có kích thước cố định lúc biên dịch. | Bọc Trait Object vào con trỏ thông minh: `Box<dyn Fn()>` hoặc dùng tham chiếu `&dyn Fn()`. |
| **E0562** | `'impl Trait' is not allowed in this position` | Bạn cố tình dùng cú pháp `impl Fn(...)` làm trường dữ liệu (field) của một `struct` hoặc bí danh kiểu (`type`). `impl Trait` chỉ được hỗ trợ ở vị trí tham số hàm và kiểu trả về của hàm. | Chuyển sang sử dụng tham số Generic trên struct (`struct MyStruct<F: Fn()> { f: F }`) hoặc dùng `Box<dyn Fn()>`. |
| **E0599** | `no method named 'and_then' found for type 'Option<...>'` | Bạn gọi `.and_then(...)` nhưng closure bên trong lại trả về một giá trị trần `T` thay vì bọc trong `Option<T>` (hoặc ngược lại với `.map()`). | Nếu closure trả về giá trị trần, dùng `.map()`. Nếu closure trả về một `Option` mới, dùng `.and_then()`. |

### Phân tích lỗi thực tế `E0308` (Bất đồng kiểu giữa hai Closure):

```rust
// Đoạn mã lỗi minh họa:
fn broken_closure(condition: bool) {
    // LỖI E0308: Hai nhánh if và else trả về hai kiểu closure ẩn danh khác nhau!
    /*
    let bo_xu_ly = if condition {
        |x: i32| x + 1
    } else {
        |x: i32| x * 2
    };
    */
}

// Cách sửa chữa đúng chuẩn: Bọc qua Box<dyn Fn>
fn correct_closure(condition: bool) {
    let bo_xu_ly: Box<dyn Fn(i32) -> i32> = if condition {
        Box::new(|x: i32| x + 1)
    } else {
        Box::new(|x: i32| x * 2)
    };
    println!("Kết quả: {}", bo_xu_ly(10));
}
```

---

## Tóm tắt chương & Bài tập rèn luyện (Summary & Exercises)

### 4 Điểm cốt lõi cần ghi nhớ:
1. **Hàm là Công dân hạng nhất**: Rust cho phép truyền hàm và closure như tham số, trả về hàm mới từ hàm khác, mở ra tư duy thiết kế kiến trúc dạng khối lắp ghép linh hoạt.
2. **`impl Fn` vs `Box<dyn Fn>`**:
   - `impl Fn`: Dành cho phân phối tĩnh, siêu tốc độ, không cấp phát bộ nhớ động trên Heap.
   - `Box<dyn Fn>`: Dành cho phân phối động, cho phép trả về nhiều closure khác nhau tùy điều kiện lúc chạy.
3. **Bộ kết hợp Combinators**: Thay thế triệt để các khối `match` lồng nhau sâu dòng bằng chuỗi đường ống (`.map()`, `.and_then()`, `.filter()`, `.or_else()`).
4. **Lan truyền lỗi (Error propagation) phẳng phiu**: Kết hợp toán tử `?` với `.ok_or()` để chuyển đổi tự nhiên giữa `Option` và `Result`, giữ cho luồng nghiệp vụ luôn trong sáng và gọn gàng.

### Bài tập rèn luyện tự giải:
1. **Bài tập 1 (Hàm bậc cao lọc mảng)**:  
   Viết một hàm bậc cao `dem_thoa_man<T, F>(list: &[T], dieu_kien: F) -> usize` với `F: Fn(&T) -> bool`. Dùng hàm này để đếm xem trong một danh sách chuỗi ký tự có bao nhiêu từ có độ dài lớn hơn 5 ký tự.

   <details>
   <summary><b>Gợi ý</b></summary>

   Thân hàm chỉ cần một đường ống `.iter().filter(...).count()`. Với chuỗi tiếng Việt, hãy dùng `.chars().count()` chứ **không** dùng `.len()` — `.len()` đếm byte, còn chữ có dấu chiếm 2–3 byte (xem Chương 05).
   </details>

   <details>
   <summary><b>Lời giải</b></summary>

   ```rust
   pub fn count_matching<T, F>(list: &[T], condition: F) -> usize
   where
       F: Fn(&T) -> bool,
   {
       list.iter().filter(|x| condition(x)).count()
   }

   fn main() {
       let tu = ["Rust", "an toàn", "fast", "đồng thời", "bộ nhớ"];

       // Đếm theo SỐ CHỮ CÁI, không phải số byte
       let long = count_matching(&tu, |s: &&str| s.chars().count() > 5);
       assert_eq!(long, 3); // "an toàn", "đồng thời", "bộ nhớ"

       // Cùng một hàm, đổi closure là đổi hẳn câu hỏi:
       let so_nguyen = [3, 8, 12, 5, 20];
       assert_eq!(count_matching(&so_nguyen, |&n| n > 6), 3);

       println!("Từ dài hơn 5 ký tự: {}", long);
   }
   ```

   Thử dùng `.len()` thay cho `.chars().count()` và bạn sẽ nhận kết quả `4` — sai, vì `"nhanh"` chỉ có 5 chữ nhưng `"bộ nhớ"` thì `.len()` đếm ra tận 9 byte. Đây là lỗi kinh điển khi xử lý tiếng Việt.
   </details>

2. **Bài tập 2 (Xây dựng Bộ kết hợp tính toán an toàn)**:  
   Cho một chuỗi đầu vào tùy ý `let input: Option<&str> = Some(" 50 ");`.  
   Dùng chuỗi combinators (`map`, `and_then`) để:
   - Cắt tỉa khoảng trắng.
   - Chuyển thành số nguyên `i32`.
   - Nhân số đó với 2 nếu số đó dương.
   - Trả về giá trị mặc định là `0` nếu đầu vào là `None` hoặc chuỗi không thể phân tích thành số (sử dụng `.unwrap_or(...)`).

   <details>
   <summary><b>Gợi ý</b></summary>

   Áp dụng đúng quy tắc ở mục 4: closure trả về **giá trị trần** thì dùng `map`; closure trả về **một chiếc hộp** (`Option`) thì dùng `and_then`. Bước "chuyển thành số" và bước "chỉ nhân nếu dương" đều có thể thất bại — chúng thuộc nhóm nào?
   </details>

   <details>
   <summary><b>Lời giải</b></summary>

   ```rust
   fn handle(input: Option<&str>) -> i32 {
       input
           .map(|s| s.trim())                              // &str -> &str      : trần  -> map
           .and_then(|s| s.parse::<i32>().ok())            // &str -> Option<i32>: hộp  -> and_then
           .filter(|&n| n > 0)                             // chỉ giữ số dương
           .map(|n| n * 2)                                 // i32 -> i32        : trần  -> map
           .unwrap_or(0)                                   // giá trị dự phòng
   }

   fn main() {
       assert_eq!(handle(Some(" 50 ")), 100);
       assert_eq!(handle(Some(" -7 ")), 0);   // bị `filter` loại
       assert_eq!(handle(Some("abc")), 0);    // `parse` hỏng
       assert_eq!(handle(None), 0);           // không có gì để xử lý
       println!("{} {} {} {}", handle(Some(" 50 ")), handle(Some(" -7 ")),
                               handle(Some("abc")), handle(None));
   }
   ```

   Bốn tình huống hoàn toàn khác nhau, **một** đường ống phẳng phiu xử lý hết — không một dòng `match` lồng nhau nào. Đây chính là "đường ray thành công" ở mục 5: chỉ cần một bước bẻ ghi là tàu chạy thẳng tới `unwrap_or(0)`.
   </details>

3. **Bài tập 3 (Tư duy kiến trúc)**:  
   Tại sao việc sử dụng con trỏ thông minh (smart pointer) `Box<dyn Fn()>` lại có chi phí thực thi **cao hơn** so với `impl Fn()`? Hãy nêu **ba** nguồn chi phí cụ thể, liên hệ kiến thức về bảng tra cứu hàm ảo (vtable) và việc cấp phát bộ nhớ Heap. Sau đó chỉ ra **một tình huống** mà bạn vẫn buộc phải chọn `Box<dyn Fn()>` dù nó đắt hơn.

   <details>
   <summary><b>Gợi ý</b></summary>

   Hãy đặt câu hỏi cho từng giai đoạn: lúc *tạo* closure có xin thêm bộ nhớ không? Lúc *gọi* closure, CPU có biết trước địa chỉ hàm cần nhảy tới không? Và trình tối ưu hóa LLVM có nội tuyến hóa (inline) được một lời gọi mà nó không biết đích đến hay không?
   </details>

   <details>
   <summary><b>Lời giải tham khảo</b></summary>

   **Ba nguồn chi phí của `Box<dyn Fn()>`:**
   1. **Cấp phát Heap**: `Box::new(...)` xin một vùng nhớ trên Heap và giải phóng nó khi `Box` bị hủy. `impl Fn` thì nằm trọn trên Stack, không tốn lần cấp phát nào.
   2. **Gọi gián tiếp qua vtable**: `dyn Fn` là một *đối tượng trait*, được biểu diễn bằng **con trỏ béo** 16 byte (con trỏ dữ liệu + con trỏ vtable). Mỗi lần gọi, CPU phải đọc vtable để biết nhảy đi đâu — một phép truy cập bộ nhớ phụ và một lệnh nhảy gián tiếp mà bộ dự đoán rẽ nhánh khó đoán trước.
   3. **Mất khả năng nội tuyến hóa**: vì đích đến chỉ biết lúc chạy, LLVM không thể chèn thẳng thân closure vào chỗ gọi. Với `impl Fn`, nhờ cơ chế đơn hình hóa (Chương 12), lời gọi thường được inline hoàn toàn và chi phí trừu tượng về đúng bằng **không**.

   **Khi nào vẫn phải chọn `Box<dyn Fn()>`?**
   - Khi cần chứa **nhiều closure khác kiểu nhau trong cùng một tập hợp**: `Vec<Box<dyn Fn()>>` (mỗi closure có một kiểu ẩn danh riêng, `impl Fn` không làm được).
   - Khi hàm phải **trả về closure khác nhau tùy nhánh `if/else`**.
   - Khi closure được lưu làm **trường của struct** mà bạn không muốn struct đó trở thành generic lan khắp chương trình.

   Nói gọn: `impl Fn` cho tốc độ, `Box<dyn Fn>` cho tính linh hoạt. Hãy mặc định chọn `impl Fn` và chỉ chuyển sang `Box<dyn Fn>` khi trình biên dịch cho thấy bạn thực sự cần.
   </details>
