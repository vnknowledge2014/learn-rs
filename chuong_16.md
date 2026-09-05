# Chương 16: Hàm bậc cao & Mẫu thiết kế lập trình hàm trong Rust (Higher-Order Functions & Functional Design Patterns)

## Giới thiệu & Mục tiêu học tập

Sau khi đã làm chủ Hàm ẩn danh (Closures) ở Chương 14 và Bộ lặp (Iterators) ở Chương 15, chúng ta đã thu thập đủ các mảnh ghép nền tảng của Lập trình hàm trong Rust. Giờ là lúc nâng tầm tư duy kiến trúc phần mềm bằng cách kết hợp chúng lại thành một bức tranh hoàn chỉnh thông qua **Hàm bậc cao (Higher-Order Functions - HOFs)** và các **Mẫu thiết kế hàm (Functional Design Patterns)**.

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
fn nhan_doi(x: i32) -> i32 { x * 2 }

// Hàm bậc cao nhận con trỏ hàm fn thuần túy
fn ap_dung_fn(con_tro: fn(i32) -> i32, gia_tri: i32) -> i32 {
    con_tro(gia_tri)
}

// Hàm bậc cao nhận Trait Bound tổng quát (chấp nhận CẢ fn VÀ closure)
fn ap_dung_generic<F: Fn(i32) -> i32>(hanh_dong: F, gia_tri: i32) -> i32 {
    hanh_dong(gia_tri)
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
fn tao_bo_chuyen_doi(la_viet_hoa: bool) -> Box<dyn Fn(&str) -> String> {
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
fn phan_tich_tuoi_truyen_thong(chuoi: Option<&str>) -> Option<u32> {
    match chuoi {
        Some(s) => {
            let cat_khoang_trang = s.trim();
            if !cat_khoang_trang.is_empty() {
                match cat_khoang_trang.parse::<u32>() {
                    Ok(tuoi) => {
                        if tuoi >= 18 { Some(tuoi) } else { None }
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
fn phan_tich_tuoi_chuyen_nghiep(chuoi: Option<&str>) -> Option<u32> {
    chuoi
        .map(|s| s.trim())                      // 1. Cắt tỉa khoảng trắng
        .filter(|s| !s.is_empty())              // 2. Lọc chuỗi không rỗng
        .and_then(|s| s.parse::<u32>().ok())   // 3. Phân tích chuỗi thành số (bỏ qua lỗi)
        .filter(|&tuoi| tuoi >= 18)             // 4. Chỉ nhận người từ 18 tuổi trở lên
}
```
Mã nguồn giờ đây chảy thẳng từ trên xuống dưới như một dòng suối tự nhiên. Mọi trường hợp `None` hay lỗi ngầm đều được Rust tự động xử lý và lan truyền ngắn mạch (short-circuiting)!

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
pub struct HoSoTho {
    pub ten_dang_nhap: Option<String>,
    pub email: Option<String>,
    pub tuoi_chuoi: Option<String>,
}

#[derive(Debug, Clone, PartialEq)]
pub struct HoSoHopLe {
    pub ten_dang_nhap: String,
    pub email: String,
    pub tuoi: u32,
}

// ============================================================================
// 1. HÀM BẬC CAO: ĐO LƯỜNG THỜI GIAN VÀ GHI NHẬT KÝ KIỂM TOÁN (WRAPPER PATTERN)
// ============================================================================

/// Hàm bậc cao nhận vào tên tác vụ và một hành động F bất kỳ
/// Thực hiện đo thời gian thực thi của hành động đó và trả về kết quả nguyên bản
pub fn do_thoi_gian_thuc_thi<F, T>(ten_tac_vu: &str, hanh_dong: F) -> T
where
    F: FnOnce() -> T,
{
    println!(">>> [KIỂM TOÁN] Bắt đầu thực thi: {}", ten_tac_vu);
    let thoi_diem_bat_dau = Instant::now();
    
    // Gọi hàm/closure được truyền vào
    let ket_qua = hanh_dong();
    
    let khoang_thoi_gian = thoi_diem_bat_dau.elapsed();
    println!(">>> [KIỂM TOÁN] Hoàn thành '{}' trong: {:?}", ten_tac_vu, khoang_thoi_gian);
    ket_qua
}

// ============================================================================
// 2. HÀM XƯỞNG SẢN XUẤT CLOSURE (FACTORY PATTERN)
// ============================================================================

/// Tạo ra một closure kiểm tra xem một chuỗi có chứa từ cấm hay không
/// Sử dụng `move` để đóng gói danh sách từ cấm vào struct vô danh của closure
pub fn tao_bo_loc_tu_cam(danh_sach_tu_cam: Vec<&'static str>) -> impl Fn(&str) -> bool {
    move |van_ban: &str| {
        let chu_thuong = van_ban.to_lowercase();
        // Trả về true nếu KHÔNG chứa bất kỳ từ cấm nào
        !danh_sach_tu_cam.iter().any(|&tu| chu_thuong.contains(tu))
    }
}

/// Tạo ra một closure kiểm tra độ dài tối thiểu và tối đa của chuỗi
pub fn tao_bo_kiem_tra_do_dai(min: usize, max: usize) -> impl Fn(&str) -> bool {
    move |van_ban: &str| {
        let do_dai = van_ban.trim().chars().count();
        do_dai >= min && do_dai <= max
    }
}

// ============================================================================
// 3. ĐƯỜNG ỐNG XÁC THỰC BẰNG BỘ KẾT HỢP COMBINATORS (PIPELINE PATTERN)
// ============================================================================

pub fn xac_thuc_ho_so(
    ho_so: &HoSoTho,
    kiem_tra_ten: &impl Fn(&str) -> bool,
    kiem_tra_tu_cam: &impl Fn(&str) -> bool,
) -> Result<HoSoHopLe, &'static str> {
    // 1. Xác thực và chuẩn hóa Tên đăng nhập bằng chuỗi combinators
    let ten_hop_le = ho_so
        .ten_dang_nhap
        .as_deref()                                   // Option<String> -> Option<&str>
        .map(|s| s.trim())                            // Cắt khoảng trắng
        .filter(|s| kiem_tra_ten(s))                  // Kiểm tra độ dài hợp lệ
        .filter(|s| kiem_tra_tu_cam(s))              // Kiểm tra từ cấm
        .map(|s| s.to_string())
        .ok_or("Tên đăng nhập không hợp lệ hoặc chứa từ cấm!")?; // Lan truyền lỗi phẳng phiu

    // 2. Xác thực và chuẩn hóa Email
    let email_hop_le = ho_so
        .email
        .as_deref()
        .map(|s| s.trim())
        .filter(|s| s.contains('@') && s.contains('.')) // Điều kiện email cơ bản
        .map(|s| s.to_lowercase())                      // Viết thường toàn bộ email
        .ok_or("Địa chỉ Email sai định dạng!")?;

    // 3. Xác thực và chuẩn hóa Tuổi
    let tuoi_hop_le = ho_so
        .tuoi_chuoi
        .as_deref()
        .map(|s| s.trim())
        .and_then(|s| s.parse::<u32>().ok())           // Phân tích chuỗi sang u32
        .filter(|&tuoi| (16..=100).contains(&tuoi))    // Giới hạn độ tuổi từ 16 đến 100
        .ok_or("Độ tuổi phải là số nguyên từ 16 đến 100!")?;

    // Trả về cấu trúc hồ sơ đã được tinh chế sạch sẽ
    Ok(HoSoHopLe {
        ten_dang_nhap: ten_hop_le,
        email: email_hop_le,
        tuoi: tuoi_hop_le,
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
    let kiem_tra_do_dai_ten = tao_bo_kiem_tra_do_dai(4, 15);
    let kiem_tra_tu_cam = tao_bo_loc_tu_cam(vec!["admin", "root", "lua_dao"]);

    // Dữ liệu mẫu 1: Hồ sơ chuẩn mực hoàn hảo
    let ho_so_chuan = HoSoTho {
        ten_dang_nhap: Some(String::from("  nguyen_an  ")),
        email: Some(String::from("An.Nguyen@EXAMPLE.COM  ")),
        tuoi_chuoi: Some(String::from("  22  ")),
    };

    // Dữ liệu mẫu 2: Hồ sơ lỗi chứa từ cấm và email hỏng
    let ho_so_loi = HoSoTho {
        ten_dang_nhap: Some(String::from("super_admin")), // Chứa từ cấm 'admin'
        email: Some(String::from("email_khong_hop_le")),
        tuoi_chuoi: Some(String::from("12")),             // Dưới 16 tuổi
    };

    // 1. Kiểm tra hồ sơ chuẩn với hàm bậc cao đo thời gian
    println!("\n--- TIẾN HÀNH XỬ LÝ HỒ SƠ THỨ NHẤT ---");
    let ket_qua_1 = do_thoi_gian_thuc_thi("Xử lý Hồ sơ Hợp lệ", || {
        xac_thuc_ho_so(&ho_so_chuan, &kiem_tra_do_dai_ten, &kiem_tra_tu_cam)
    });

    match ket_qua_1 {
        Ok(ho_so) => {
            println!("[THÀNH CÔNG] Dữ liệu sau khi làm sạch:");
            println!("  - Tên đăng nhập: {}", ho_so.ten_dang_nhap);
            println!("  - Email hợp chuẩn: {}", ho_so.email);
            println!("  - Tuổi: {}", ho_so.tuoi);
        }
        Err(loi) => println!("[THẤT BẠI] Lỗi: {}", loi),
    }

    // 2. Kiểm tra hồ sơ lỗi
    println!("\n--- TIẾN HÀNH XỬ LÝ HỒ SƠ THỨ HAI (CÓ LỖI) ---");
    let ket_qua_2 = do_thoi_gian_thuc_thi("Xử lý Hồ sơ Vi phạm", || {
        xac_thuc_ho_so(&ho_so_loi, &kiem_tra_do_dai_ten, &kiem_tra_tu_cam)
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
fn thu_nghiem_loi_closure(dieu_kien: bool) {
    // LỖI E0308: Hai nhánh if và else trả về hai kiểu closure ẩn danh khác nhau!
    /*
    let bo_xu_ly = if dieu_kien {
        |x: i32| x + 1
    } else {
        |x: i32| x * 2
    };
    */
}

// Cách sửa chữa đúng chuẩn: Bọc qua Box<dyn Fn>
fn thu_nghiem_dung_closure(dieu_kien: bool) {
    let bo_xu_ly: Box<dyn Fn(i32) -> i32> = if dieu_kien {
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
4. **Lan truyền lỗi phẳng phiu**: Kết hợp toán tử `?` với `.ok_or()` để chuyển đổi tự nhiên giữa `Option` và `Result`, giữ cho luồng nghiệp vụ luôn trong sáng và gọn gàng.

### Bài tập rèn luyện tự giải:
1. **Bài tập 1 (Hàm bậc cao lọc mảng)**:  
   Viết một hàm bậc cao `dem_thoa_man<T, F>(danh_sach: &[T], dieu_kien: F) -> usize` với `F: Fn(&T) -> bool`. Dùng hàm này để đếm xem trong một danh sách chuỗi ký tự có bao nhiêu từ có độ dài lớn hơn 5 ký tự.

2. **Bài tập 2 (Xây dựng Bộ kết hợp tính toán an toàn)**:  
   Cho một chuỗi đầu vào tùy ý `let dau_vao: Option<&str> = Some(" 50 ");`.  
   Dùng chuỗi combinators (`map`, `and_then`) để:
   - Cắt tỉa khoảng trắng.
   - Chuyển thành số nguyên `i32`.
   - Nhân số đó với 2 nếu số đó dương.
   - Trả về giá trị mặc định là `0` nếu đầu vào là `None` hoặc chuỗi không thể phân tích thành số (sử dụng `.unwrap_or(...)`).

3. **Bài tập 3 (Tư duy kiến trúc)**:  
   Tại sao việc sử dụng con trỏ thông minh (smart pointer) `Box<dyn Fn()>` lại có chi phí thực thi nhỏ hơn một chút so với `impl Fn()`? Hãy liên hệ kiến thức về bảng tra cứu hàm ảo (vtable) và việc cấp phát bộ nhớ Heap.
