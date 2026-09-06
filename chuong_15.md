# Chương 15: Hàm ẩn danh: Các chế độ bắt giữ giá trị Fn, FnMut, FnOnce (Closures & Capturing Traits: Fn, FnMut, FnOnce)

## Giới thiệu & Mục tiêu học tập

Trong Chương 13, chúng ta đã tiếp cận tư duy đường ống (pipeline) của lập trình hàm (functional programming) và thấy được sự thanh thoát khi loại bỏ các biến tạm thay đổi liên tục. Bạn đã thấy những biểu thức ngắn gọn như `|hang| to_money(hang)` xuất hiện bên trong các phương thức `.map()` hay `.filter()`. Đó chính là **Hàm ẩn danh (Closures)** — một trong những vũ khí lợi hại bậc nhất của Rust.

Trong các ngôn ngữ có bộ gom rác (Garbage Collector) như JavaScript hay Python, bạn có thể tạo một hàm ẩn danh ở bất kỳ đâu và thoải mái dùng chung biến số mà không cần bận tâm biến đó được lưu trữ ở đâu trên thanh RAM hay sống được bao lâu. Nhưng trong Rust, với các nguyên tắc sắt đá về quyền sở hữu (ownership), vay mượn (borrow), và thời gian sống (lifetime), một câu hỏi hóc búa được đặt ra:
- *Khi một hàm ẩn danh sử dụng các biến ở môi trường xung quanh, nó đang mượn đọc, mượn sửa, hay đoạt đứt quyền sở hữu của biến đó?*
- *Làm sao trình biên dịch đảm bảo hàm ẩn danh không vô tình dùng một biến đã bị giải phóng khỏi bộ nhớ?*

Rust giải quyết bài toán này một cách tuyệt mỹ thông qua bộ ba Trait bắt giữ môi trường: **`Fn`**, **`FnMut`**, và **`FnOnce`**. Đây là chìa khóa then chốt giúp bạn viết mã nguồn linh hoạt nhưng vẫn an toàn tuyệt đối ở tốc độ phần cứng cao nhất.

Mục tiêu học tập của chương này:
- Nắm vững cú pháp khai báo **Closure (`|param| { than_ham }`)** và khả năng tự động suy luận kiểu dữ liệu của `rustc`.
- Thấu hiểu cơ chế **Đóng gói môi trường (Environment Capturing)**: Bản chất Closure trong Rust là một struct vô danh tự động sinh ra trên bộ nhớ ngăn xếp (stack) hoặc vùng nhớ tự do (heap).
- Phân biệt rạch ròi 3 cấp độ bắt giữ môi trường:
  - **`Fn`**: Bắt giữ bằng tham chiếu đọc bất biến (`&T`).
  - **`FnMut`**: Bắt giữ bằng tham chiếu sửa đổi khả biến (`&mut T`).
  - **`FnOnce`**: Đoạt quyền sở hữu giá trị (`T`), tiêu thụ môi trường và chỉ gọi được đúng một lần duy nhất.
- Làm chủ từ khóa **`move`** để cưỡng chế chuyển quyền sở hữu vào trong closure.
- Biết cách truyền closure vào hàm thông qua Ràng buộc Trait (Trait Bounds) hoặc con trỏ thông minh (smart pointer) `Box<dyn Fn()>`.

---

## Hình tượng hóa đời sống (Intuitive Everyday Analogy)

Để xóa tan sự khó hiểu về ba cái tên `Fn`, `FnMut`, và `FnOnce`, hãy tưởng tượng bạn là một giám đốc bận rộn và bạn thuê **3 người trợ lý bỏ túi** với 3 tấm thẻ quyền hạn khác nhau:

```
┌──────────────────────────────────────────────────────────────────────────────────┐
│             HÌNH TƯỢNG ĐỜI SỐNG: BA NGƯỜI TRỢ LÝ BỎ TÚI (Fn, FnMut, FnOnce)      │
├─────────────────────────┬───────────────────────────────┬────────────────────────┤
│    TRỢ LÝ 1: THẺ VÀNG   │      TRỢ LÝ 2: THẺ XANH       │   TRỢ LÝ 3: THẺ ĐỎ     │
│          (Fn)           │           (FnMut)             │       (FnOnce)         │
│                         │                               │                        │
│ - Chỉ được ngắm nhìn đồ │ - Cầm bút chì ghi chép thêm   │ - Đóng gói toàn bộ đồ  │
│   vật trong văn phòng   │   vào cuốn sổ tay công tác    │   đạc trên bàn vào hộp │
│ - Không xê dịch, không  │ - Làm thay đổi dữ liệu trong  │ - Đem gửi xe tải đi    │
│   sửa chữa bất cứ thứ gì│   sổ sau mỗi lần gọi          │ - Bàn làm việc sạch trơn│
│ - Có thể gọi ngắm đi    │ - Có thể gọi ghi chép thêm    │ - Chỉ làm được ĐÚNG    │
│   ngắm lại 1.000 lần!   │   vô số lần liên tục!         │   1 LẦN DUY NHẤT!      │
└─────────────────────────┴───────────────────────────────┴────────────────────────┘
```

### 1. Trợ lý Thẻ Vàng (Giao ước `Fn` - Chỉ đọc)
- Người trợ lý này bước vào phòng làm việc của bạn. Họ chỉ dùng mắt để quan sát bức tranh phong cảnh treo tường và đọc bảng số liệu dự án dán trên bảng thông báo (`&T`).
- Họ không chạm tay vào hiện vật, không xóa sửa bất cứ chữ nào.
- Vì đồ đạc trong phòng vẫn nguyên vẹn 100%, bạn có thể bấm chuông gọi người trợ lý này bước vào đọc báo cáo bao nhiêu lần tùy thích mà không sợ hỏng phòng.

### 2. Trợ lý Thẻ Xanh (Giao ước `FnMut` - Đọc và Sửa đổi)
- Người trợ lý này được cấp thêm một cây bút chì và chiếc thước kẻ (`&mut T`). Họ bước vào phòng để ghi thêm số đếm khách hàng vào cuốn sổ tay tích lũy đặt trên bàn làm việc của bạn.
- Mỗi lần người trợ lý này được gọi (`invoke`), con số trong cuốn sổ tay lại tăng thêm một nấc. Trạng thái căn phòng bị thay đổi!
- Tuy vậy, cuốn sổ tay vẫn còn nằm nguyên trên bàn của bạn. Bạn vẫn có thể gọi người trợ lý này nhiều lần tiếp theo để ghi chép bổ sung.

### 3. Trợ lý Thẻ Đỏ (Giao ước `FnOnce` - Tiêu thụ và Đoạt quyền)
- Người trợ lý này mang theo băng keo và thùng carton niêm phong. Khi bạn gọi, họ thu gom chiếc chìa khóa két sắt quý giá trên bàn, bỏ vào thùng và chuyển phát nhanh sang chi nhánh nước ngoài (`move / T`).
- Sau khi hành động đó diễn ra, chiếc chìa khóa trên bàn làm việc của bạn đã biến mất hoàn toàn!
- Do vật phẩm đã bị "tiêu thụ" (consumed), bạn **không thể yêu cầu người trợ lý làm lại hành động đó lần thứ hai**. Lệnh này chỉ thực hiện được duy nhất một lần trong đời (`FnOnce`).

---

## Khái niệm & Cơ chế kỹ thuật chuyên sâu (In-Depth Technical Mechanics)

### 1. Cú pháp Closure và Khả năng Tự suy luận kiểu (Type Inference)

Hàm thông thường (`fn`) trong Rust luôn bắt buộc bạn phải chú thích kiểu tường minh cho tất cả tham số và giá trị trả về. Ngược lại, Closure (`|...|`) được thiết kế cho các tác vụ cục bộ ngắn gọn nên trình biên dịch `rustc` có khả năng **tự suy luận kiểu dữ liệu cực kỳ mạnh mẽ**:

```rust
// 1. Hàm thông thường: Bắt buộc kiểu tường minh
fn add_one_v1(x: i32) -> i32 { x + 1 }

// 2. Closure đầy đủ chú thích kiểu
let add_one_v2 = |x: i32| -> i32 { x + 1 };

// 3. Closure rút gọn: rustc tự suy luận kiểu dựa trên ngữ cảnh gọi đầu tiên
let add_one_v3 = |x| x + 1;
```

*Lưu ý quan trọng*: Một closure chỉ có thể suy luận kiểu duy nhất một lần. Nếu dòng đầu tiên bạn gọi `cong_mot_v3(5)` (truyền số `i32`), thì closure đó vĩnh viễn khóa cứng với kiểu `i32`. Nếu dòng tiếp theo bạn gọi `cong_mot_v3(5.5)` (số thực `f64`), trình biên dịch sẽ báo lỗi bất đồng kiểu dữ liệu ngay lập tức!

### 2. Bản chất bên dưới nắp ca-pô: Closure thực chất là một Struct vô danh!

Khi bạn viết một closure bắt giữ các biến xung quanh, trình biên dịch Rust thực sự làm gì?
Rust **không hề dùng con trỏ hàm chậm chạp hay bộ nhớ động dư thừa**. Thay vào đó, `rustc` tự động tạo ra một `struct` ẩn giấu với tên gọi nội bộ duy nhất (ví dụ: `Closure$1234`), trong đó các trường (fields) chính là các biến được bắt giữ:

```rust
let name = String::from("Rust");
let in_ten = || println!("{}", name);
```

Bên dưới tầng mã máy, Rust chuyển đoạn mã trên thành cấu trúc tương đương:
```rust
// [Mã do rustc tự sinh ngầm bên dưới]
struct EnvFrame<'a> {
    name: &'a String, // Trường dữ liệu mượn đọc
}

impl<'a> Fn<()> for EnvFrame<'a> {
    extern "rust-call" fn call(&self, _args: ()) {
        println!("{}", *self.name);
    }
}
```
> **⚠️ Lưu ý quan trọng — đoạn mã trên là MÔ PHỎNG KHÁI NIỆM, không phải mã hợp lệ.**
> Trait `Fn` cùng cú pháp `Fn<Args>` và ABI `extern "rust-call"` đều là tính năng **chưa ổn định** của Rust
> (`unboxed_closures`, `fn_traits`). Bạn **không thể** tự tay `impl Fn for` một struct trên Rust bản ổn định —
> gõ thử sẽ nhận lỗi `E0658: the extern "rust-call" ABI is experimental`.
> Chỉ có bản thân trình biên dịch mới sinh ra được các cài đặt đó. Đoạn mã trên chỉ nhằm cho bạn thấy
> *hình dung* về thứ `rustc` tạo ra sau lưng bạn.

Nhờ cơ chế biến closure thành struct này, Rust đạt được hiệu năng **Trừu tượng hóa không chi phí (Zero-Cost Abstraction)**: Closure được cấp phát trực tiếp trên Stack, không tốn một byte rác nào trên Heap, và có thể được mở rộng nội tuyến (inline) thẳng vào mã máy CPU!

### 3. Phân cấp Kế thừa giữa 3 Trait: `FnOnce` là Gốc rễ

Trong thư viện chuẩn của Rust, 3 trait này có mối quan hệ phụ thuộc chặt chẽ (Sub-traits):

```
       FnOnce (Đoạt quyền sở hữu, gọi ít nhất 1 lần)
          ▲
          │  (Mọi FnMut đều tự động là FnOnce)
        FnMut (Sửa đổi trạng thái, gọi nhiều lần)
          ▲
          │  (Mọi Fn đều tự động là FnMut)
          Fn (Chỉ đọc, gọi nhiều lần vô hạn)
```

- **`FnOnce`**: Trait rộng nhất. Mọi closure trong Rust đều tự động triển khai `FnOnce`, bởi vì nếu bạn có thể gọi một hàm nhiều lần, bạn chắc chắn có thể gọi nó ít nhất một lần. Phương thức của nó nhận `self` theo giá trị:
  ```rust
  pub trait FnOnce<Args> {
      type Output;
      extern "rust-call" fn call_once(self, args: Args) -> Self::Output;
  }
  ```
- **`FnMut`**: Đòi hỏi quyền mượn sửa `&mut self`. Cho phép closure thay đổi các trường dữ liệu nội bộ được đóng gói:
  ```rust
  pub trait FnMut<Args>: FnOnce<Args> {
      extern "rust-call" fn call_mut(&mut self, args: Args) -> Self::Output;
  }
  ```
- **`Fn`**: Đòi hỏi quyền mượn đọc bất biến `&self`. Tuyệt đối an toàn để chia sẻ giữa nhiều luồng hoặc gọi liên tục:
  ```rust
  pub trait Fn<Args>: FnMut<Args> {
      extern "rust-call" fn call(&self, args: Args) -> Self::Output;
  }
  ```

### 4. Từ khóa `move` và Cưỡng chế Quyền sở hữu

Mặc định, Rust sẽ tự động chọn chế độ bắt giữ "nhẹ nhàng nhất có thể" (ưu tiên mượn đọc `&T`, rồi đến mượn sửa `&mut T`, cuối cùng mới đến lấy giá trị `T`).
Tuy nhiên, khi bạn muốn chuyển một closure sang một luồng tiến trình độc lập (thread) hoặc lưu trữ nó trong một cấu trúc dữ liệu sống lâu hơn hàm hiện tại, bạn phải thêm từ khóa **`move`**:

```rust
let greeting = String::from("Xin chào");
// move ép buộc closure đoạt quyền sở hữu greeting vào struct nội bộ của nó
let closure_moves = move || {
    println!("{}", greeting);
};
// greeting không còn sử dụng được ở đây nữa vì đã bị move!
```

---

## Mã nguồn minh họa thực chiến (Idiomatic Runnable Rust Blueprint)

Chương trình hoàn chỉnh dưới đây xây dựng một **Hệ thống Quản lý Tác vụ Sự kiện (Event-Driven Task Dispatcher)**, minh họa chi tiết cả ba chế độ bắt giữ `Fn`, `FnMut`, và `FnOnce`, cũng như kỹ thuật truyền closure vào hàm thông qua Generics và con trỏ thông minh (smart pointer) `Box<dyn Fn()>`.

```rust
// Tệp: src/main.rs
// Chương trình thực chiến làm chủ Closures: Fn, FnMut, và FnOnce trong Rust

// ============================================================================
// CÁC HÀM NHẬN CLOSURE LÀM THAM SỐ VỚI RÀNG BUỘC TRAIT (TRAIT BOUNDS)
// ============================================================================

/// Hàm 1: Nhận closure thực hiện deliver ước Fn (Chỉ đọc môi trường)
/// Có thể gọi closure này nhiều lần liên tiếp một cách an toàn tuyệt đối
pub fn exec_read<F>(ten_tac_vu: &str, hanh_dong: F)
where
    F: Fn(),
{
    println!("--- BẮT ĐẦU TÁC VỤ CHỈ ĐỌC: [{}] ---", ten_tac_vu);
    hanh_dong(); // Gọi lần 1
    hanh_dong(); // Gọi lần 2
    println!("--- HOÀN THÀNH TÁC VỤ CHỈ ĐỌC ---");
}

/// Hàm 2: Nhận closure thực hiện deliver ước FnMut (Sửa đổi môi trường)
/// Bắt buộc tham số hanh_dong phải mang từ khóa mut vì trạng thái nội bộ thay đổi
pub fn exec_swap<F>(ten_tac_vu: &str, mut hanh_dong: F, so_vong_lap: usize)
where
    F: FnMut(usize),
{
    println!("\n--- BẮT ĐẦU TÁC VỤ SỬA ĐỔI TRẠNG THÁI: [{}] ---", ten_tac_vu);
    for step in 1..=so_vong_lap {
        hanh_dong(step); // Gọi nhiều lần, mỗi lần biến nội bộ bên ngoài sẽ biến đổi
    }
    println!("--- HOÀN THÀNH TÁC VỤ SỬA ĐỔI TRẠNG THÁI ---");
}

/// Hàm 3: Nhận closure thực hiện deliver ước FnOnce (Tiêu thụ tài nguyên)
/// Closure này tự hủy ngay sau khi được gọi vì quyền sở hữu đã bị đoạt lấy
pub fn exec_consume<F>(ten_tac_vu: &str, hanh_dong: F)
where
    F: FnOnce() -> String,
{
    println!("\n--- BẮT ĐẦU TÁC VỤ TIÊU THỤ MỘT LẦN: [{}] ---", ten_tac_vu);
    let ket_qua = hanh_dong(); // Gọi DUY NHẤT một lần tại đây
    // hanh_dong(); // Nếu bỏ dấu chú thích dòng này, rustc sẽ chặn ngay lập tức!
    println!("Kết quả nhận được sau khi tiêu thụ: {}", ket_qua);
    println!("--- TÀI NGUYÊN ĐÃ ĐƯỢC GIẢI PHÓNG TOÀN DIỆN ---");
}

// ============================================================================
// CHƯƠNG TRÌNH ĐIỀU HÀNH CHÍNH
// ============================================================================

fn main() {
    println!("============================================================");
    println!("      HỆ THỐNG ĐIỀU PHỐI TÁC VỤ SỰ KIỆN: FN, FNMUT, FNONCE  ");
    println!("============================================================");

    // ------------------------------------------------------------------------
    // TÌNH HUỐNG 1: Giao ước Fn - Bắt giữ tham chiếu chỉ đọc (&T)
    // ------------------------------------------------------------------------
    let thong_tin_he_thong = String::from("Máy chủ Cổng thanh toán (Gateway-01)");
    
    // Closure print_info chỉ mượn đọc thong_tin_he_thong
    let print_info = || {
        println!("[GIÁM SÁT] Trạng thái hiện tại của: {}", thong_tin_he_thong);
    };

    // Truyền closure vào hàm exec_read (chứng minh gọi được nhiều lần)
    exec_read("Kiểm tra sức khỏe định kỳ", print_info);
    // Biến thong_tin_he_thong vẫn hoàn toàn nguyên vẹn ở phạm vi ngoài:
    println!("Biến gốc bên ngoài vẫn truy cập bình thường: {}", thong_tin_he_thong);

    // ------------------------------------------------------------------------
    // TÌNH HUỐNG 2: Giao ước FnMut - Bắt giữ tham chiếu sửa đổi (&mut T)
    // ------------------------------------------------------------------------
    let mut total_amount_access_cap: usize = 0;
    let mut activity_log: Vec<String> = Vec::new();

    // Closure tang_truy_cap mượn sửa đổi biến total_amount_access_cap và activity_log
    let record_view = |lan_lap: usize| {
        total_amount_access_cap += 10;
        activity_log.push(format!("Đợt ghi nhận #{}: +10 yêu cầu", lan_lap));
        println!("  -> Đang tích lũy... Tổng lưu lượng hiện tại: {}", total_amount_access_cap);
    };

    // Thực thi 3 vòng lặp tích lũy
    exec_swap("Bộ đếm lưu lượng mạng", record_view, 3);
    println!("Kết quả sau khi kết thúc FnMut:");
    println!("- Tổng lưu lượng cuối cùng: {}", total_amount_access_cap);
    println!("- Chi tiết nhật ký: {:?}", activity_log);

    // ------------------------------------------------------------------------
    // TÌNH HUỐNG 3: Giao ước FnOnce - Đoạt quyền sở hữu (Move)
    // ------------------------------------------------------------------------
    // Giả lập một khóa bảo mật phiên đăng nhập chỉ dùng một lần (One-Time Token)
    let secret_token = String::from("SEC-TOKEN-XYZ-9999-SECRET");

    // Dùng từ khóa move để ép closure chiếm trọn quyền sở hữu của secret_token
    let end_session = move || {
        // Biến secret_token bị di chuyển vào đây và tiêu thụ
        let thong_report = format!("Khóa [{}] đã bị attempt hồi vĩnh viễn.", secret_token);
        thong_report // Trả về chuỗi thông báo, secret_token bị Drop tại đây
    };

    exec_consume("Tiêu hủy phiên bảo mật", end_session);
    // println!("{}", secret_token); // LỖI: value borrowed here after move!

    // ------------------------------------------------------------------------
    // TÌNH HUỐNG 4: Lưu trữ danh sách Closure trong Vector với Box<dyn Fn()>
    // ------------------------------------------------------------------------
    println!("\n--- QUẢN LÝ DANH SÁCH BỘ ĐIỀU HƯỚNG VỚI BOX<DYN FN()> ---");
    let mut list_event: Vec<Box<dyn Fn()>> = Vec::new();

    list_event.push(Box::new(|| println!("Sự kiện A: Khởi động quạt làm mát")));
    list_event.push(Box::new(|| println!("Sự kiện B: Đèn LED chuyển màu xanh")));

    for (stt, event) in list_event.iter().enumerate() {
        print!("Kích hoạt sự kiện #{}: ", stt + 1);
        event(); // Gọi từng closure qua con trỏ Trait Object
    }

    println!("\n============================================================");
    println!("     HOÀN TẤT XÁC THỰC CƠ CHẾ BẮT GIỮ MÔI TRƯỜNG CỦA RUST   ");
    println!("============================================================");
}
```

---

## Bảng tra cứu lỗi biên dịch & Cách khắc phục (Compiler Error Guide)

Khi làm việc với Closure trong Rust, người lập trình thường vấp phải các thông báo lỗi đặc trưng liên quan đến việc mượn quyền và quyền sở hữu (ownership):

| Mã lỗi | Thông báo mẫu từ trình biên dịch | Nguyên nhân cốt lõi | Cách khắc phục nhanh |
|---|---|---|---|
| **E0525** | `expected a closure that implements the 'Fn' trait, but this closure only implements 'FnMut'` | Hàm của bạn đòi hỏi tham số dạng `F: Fn()` (chỉ đọc), nhưng bên trong thân closure bạn lại thay đổi một biến môi trường, khiến nó bị hạ cấp xuống thành `FnMut`. | Đổi ràng buộc của hàm nhận tham số thành `F: FnMut()`, hoặc tái cấu trúc logic bên trong closure để không thay đổi biến ngoài. |
| **E0507** | `cannot move out of '...', a captured variable in an 'FnMut' closure` | Trong closure dạng `FnMut` (được gọi nhiều lần), bạn lại thực hiện hành động chuyển quyền sở hữu (move) một giá trị ra ngoài. Vì hàm chạy nhiều lần, lần chạy thứ hai biến đó đã mất! | Mượn tham chiếu `&` thay vì lấy quyền sở hữu, hoặc nhân bản giá trị bằng `.clone()` trước khi di chuyển. |
| **E0382** | `use of moved value: '...'` | Bạn đã dùng từ khóa `move ||` khiến biến bị đoạt quyền sở hữu vào trong closure, sau đó bạn lại cố dùng tiếp biến đó ở phạm vi bên ngoài. | Không dùng từ khóa `move` nếu chỉ cần mượn đọc, hoặc gọi phương thức `.clone()` để tạo bản sao trước khi chuyển vào closure. |
| **E0596** | `cannot borrow '...' as mutable, as it is not declared as mutable` | Bạn gọi một closure có tính chất `FnMut` nhưng biến lưu closure đó không được đánh dấu bằng từ khóa `mut`. | Thêm từ khóa `mut` vào biến lưu closure: `let mut my_closure = ...;`. |

### Phân tích lỗi thực tế `E0525`:

```rust
// Đoạn mã lỗi minh họa E0525:
fn call_twice<F: Fn()>(f: F) {
    f();
    f();
}

fn broken_example() {
    let mut count = 0;
    // Closure này sửa biến dem nên nó là FnMut, không thỏa mãn Fn
    let closure_broken = || { 
        count += 1; 
    };
    // call_twice(closure_broken); // LỖI E0525: closure chỉ cài đặt FnMut, không phải Fn!
}

// Cách sửa chữa:
fn call_twice_fixed<F: FnMut()>(mut f: F) {
    f();
    f();
}
```

---

## Tóm tắt chương & Bài tập rèn luyện (Summary & Exercises)

### 4 Điểm cốt lõi cần ghi nhớ:
1. **Closure là Struct vô danh**: Trình biên dịch tự động tạo cấu trúc dữ liệu lưu trữ các biến được bắt giữ trên Stack, đem lại hiệu năng tối đa (Zero-Cost Abstraction).
2. **Ba cấp độ bắt giữ**:
   - `Fn`: Bắt giữ tham chiếu đọc `&T`, gọi nhiều lần, không làm biến đổi môi trường.
   - `FnMut`: Bắt giữ tham chiếu sửa đổi `&mut T`, gọi nhiều lần, thay đổi trạng thái nội bộ.
   - `FnOnce`: Đoạt quyền sở hữu `T`, tiêu thụ tài nguyên và chỉ gọi được đúng một lần duy nhất.
3. **Từ khóa `move`**: Ép buộc closure đoạt quyền sở hữu toàn bộ các biến môi trường được sử dụng, rất quan trọng khi truyền closure sang luồng mới hoặc trả về từ hàm.
4. **Linh hoạt đa hình**: Có thể truyền closure tĩnh thông qua Generics `<F: Fn()>` để tối ưu hóa mã máy, hoặc truyền động thông qua Trait Object `Box<dyn Fn()>`.

### Bài tập rèn luyện tự giải:
1. **Bài tập 1 (Phán đoán loại Trait)**:  
   Xem xét đoạn mã sau và phán đoán xem closure `handle` sẽ tự động thực hiện các Trait nào (`Fn`, `FnMut`, hay `FnOnce`):
   ```rust
   let list = vec![1, 2, 3];
   let handle = || {
       println!("Độ dài danh sách: {}", list.len());
   };
   ```
   Hãy viết mã nguồn kiểm chứng bằng cách truyền `handle` vào hàm đòi hỏi `Fn`.

2. **Bài tập 2 (Thiết kế Bộ lọc Tùy biến với Fn)**:  
   Viết một hàm `loc_du_lieu<F>(list: &[i32], dieu_kien: F) -> Vec<i32>` trong đó `dieu_kien` là một closure có chữ ký `Fn(&i32) -> bool`. Dùng hàm này để lọc ra các số chẵn lớn hơn 10 từ một mảng số nguyên bất kỳ.

3. **Bài tập 3 (Sử dụng FnMut làm Bộ tích lũy)**:  
   Viết một closure `accumulate` sử dụng tính chất `FnMut` để cộng dồn điểm số của học sinh qua từng môn học. Mỗi lần gọi `accumulate(diem)`, điểm số mới được cộng thêm và in ra màn hình điểm trung bình tạm thời sau mỗi môn thi.

---

### Gợi ý & Lời giải

<details>
<summary><b>Bài tập 1 — Gợi ý</b></summary>

Hãy hỏi: closure này **làm gì** với `list`? Nó gọi `.len()` — chỉ đọc. Không sửa, không tiêu thụ. Vậy chế độ bắt giữ "nhẹ nhàng nhất" mà Rust chọn là gì?
</details>

<details>
<summary><b>Bài tập 1 — Lời giải</b></summary>

Closure `handle` chỉ **mượn đọc** `list` (`&Vec<i32>`), nên nó cài đặt **cả ba** trait: `Fn`, `FnMut` và `FnOnce`. Nhớ phân cấp ở mục 3: `Fn` là hẹp nhất và tự động thỏa mãn hai trait còn lại.

```rust
fn goi_ba_lan<F: Fn()>(f: F) {
    f();
    f();
    f();   // gọi được nhiều lần -> chứng minh nó đúng là `Fn`
}

fn main() {
    let list = vec![1, 2, 3];
    let handle = || println!("Độ dài danh sách: {}", list.len());

    goi_ba_lan(handle);

    // list VẪN dùng được vì closure chỉ mượn đọc, không đoạt quyền sở hữu:
    println!("Danh sách gốc vẫn nguyên vẹn: {:?}", list);
}
```

Thử nghiệm đáng làm: thêm `list.push(4);` vào thân closure. Nó lập tức bị hạ cấp xuống `FnMut` và `goi_ba_lan` sẽ từ chối biên dịch với lỗi **E0525**.
</details>

<details>
<summary><b>Bài tập 2 — Gợi ý</b></summary>

Ràng buộc `F: Fn(&i32) -> bool` khớp chính xác với thứ mà `.filter()` cần. Vì `.iter()` cho ra `&i32` còn bạn muốn trả `Vec<i32>`, hãy dùng `.copied()` (hoặc `.cloned()`) để bóc một lớp tham chiếu trước khi `.collect()`.
</details>

<details>
<summary><b>Bài tập 2 — Lời giải</b></summary>

```rust
pub fn filter_data<F>(list: &[i32], condition: F) -> Vec<i32>
where
    F: Fn(&i32) -> bool,
{
    list.iter().filter(|x| condition(x)).copied().collect()
}

fn main() {
    let so = [4, 12, 7, 20, 30, 9, 16];

    let greater_than_ten = filter_data(&so, |&x| x % 2 == 0 && x > 10);
    assert_eq!(greater_than_ten, vec![12, 20, 30, 16]);

    // Cùng một hàm, đổi closure là đổi hẳn hành vi — đó là sức mạnh của hàm bậc high:
    let so_le = filter_data(&so, |&x| x % 2 != 0);
    println!("Chẵn > 10: {:?}\nLẻ      : {:?}", greater_than_ten, so_le);
}
```
</details>

<details>
<summary><b>Bài tập 3 — Gợi ý</b></summary>

Closure cần nhớ **hai** thứ giữa các lần gọi: tổng điểm và số môn đã thi. Cả hai đều bị bắt giữ theo `&mut`, nên closure sẽ là `FnMut` — và biến chứa nó bắt buộc phải khai báo `let mut`.
</details>

<details>
<summary><b>Bài tập 3 — Lời giải</b></summary>

```rust
fn main() {
    let mut tong: f64 = 0.0;
    let mut so_mon: u32 = 0;

    // Closure này SỬA hai biến ngoài -> nó là FnMut -> biến chứa nó phải `mut`.
    let mut accumulate = |diem: f64| {
        tong += diem;
        so_mon += 1;
        let mean = tong / so_mon as f64;
        println!("  Môn thứ {}: {:.1} điểm | Trung bình tạm thời: {:.2}",
                 so_mon, diem, mean);
        mean
    };

    println!("Bảng điểm học kỳ:");
    accumulate(8.0);
    accumulate(6.5);
    accumulate(9.0);
    let final = accumulate(7.5);

    // Closure phải kết thúc vòng đời (ra khỏi phạm vi mượn) thì mới đọc lại được biến gốc.
    drop(accumulate);
    println!("Điểm trung bình cuối: {:.2} trên {} môn", final, so_mon);
}
```

Hai điểm dễ sai:
- Quên `let mut accumulate` → lỗi **E0596** (`cannot borrow as mutable`).
- Cố đọc `tong` khi closure vẫn còn sống → lỗi **E0502**, vì closure đang giữ quyền mượn sửa. Gọi `drop(accumulate)` (hoặc đặt closure trong một khối `{ }`) để trả quyền mượn lại.
</details>
