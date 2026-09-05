# Chương 35: Kiểm chứng an toàn bộ nhớ Rust vs Unsafe Rust & FFI (Rust Memory Safety Verification vs Unsafe Rust & FFI)

## Giới thiệu & Mục tiêu học tập

Ở hai chương trước, chúng ta đã chứng kiến cách kiến trúc không gian địa chỉ ảo vận hành và cách Rust dựng nên một bức tường thép bảo vệ hệ thống khỏi Tam đại hiểm họa tham nhũng bộ nhớ. Một câu hỏi tự nhiên xuất hiện trong tâm trí của mọi kỹ sư: **Nếu Rust an toàn tuyệt đối như vậy, làm thế nào nó có thể giao tiếp trực tiếp với vi mạch phần cứng, điều khiển thanh ghi CPU, hoặc tích hợp với hàng tỷ dòng mã nguồn C/C++ đang vận hành trong nhân hệ điều hành Linux, Windows và macOS?**

Câu trả lời nằm ở cánh cổng bí mật của ngôn ngữ: **`unsafe` Rust và Giao diện giao tiếp hàm ngoại lai (Foreign Function Interface - FFI)**. Từ khóa `unsafe` trong Rust không phải là một "lỗ hổng", mà là một công cụ có chủ đích, một hợp đồng phân định trách nhiệm rõ ràng giữa con người và trình biên dịch.

Trong chương này, chúng ta sẽ làm sáng tỏ:
- Sự khác biệt giữa kiểm chứng tĩnh tự động (Static Verification qua Borrow Checker) và sự can thiệp thủ công có kiểm soát của lập trình viên.
- **Năm siêu năng lực duy nhất của `unsafe`**: Những điều mà Safe Rust từ chối thực hiện nhưng Unsafe Rust cho phép.
- Khái niệm **Bất biến an toàn (Safety Invariants)** và nguyên tắc thiết kế **Bao bọc an toàn (Safe Abstraction Wrapper)**: Cách các thư viện cốt lõi (`Vec`, `String`, `Box`) biến mã nguồn cấp thấp thành các API an toàn 100%.
- Cách thức hoạt động của FFI: Trao đổi dữ liệu hai chiều với ngôn ngữ C thông qua quy ước nhị phân `extern "C"` và định dạng tương thích bộ nhớ `#[repr(C)]`.
- Các hành vi bất định (Undefined Behavior - UB) nguy hiểm nhất và cách sử dụng công cụ kiểm định Miri để rà soát lỗi bộ nhớ.

---

## Hình tượng hóa đời sống (Intuitive Everyday Analogy)

Để thấu suốt ranh giới giữa Safe Rust, Unsafe Rust và FFI, hãy hình dung hai bức tranh đời sống sau:

```
┌──────────────────────────────────────────────────────────────────────────────────┐
│              HÌNH TƯỢNG HÓA: PHÒNG ĐIỆN CAO THẾ & CỬA KHẨU QUỐC TẾ               │
├──────────────────────────────────────────────────────────────────────────────────┤
│                                                                                  │
│ [1. PHÒNG BIẾN ÁP CAO THẾ (UNSAFE RUST) VỚI LỚP VỎ CÁCH ĐIỆN (SAFE WRAPPER)]     │
│ ┌──────────────────────────────────────────────────────────────────────┐         │
│ │ [Bên Ngoài: Khu dân cư an toàn (Safe Rust)]                          │         │
│ │ Người dân bật công tắc đèn, cắm phích sạc điện thoại thoải mái       │         │
│ │ mà không bao giờ sợ bị điện giật.                                    │         │
│ ├──────────────────────────────────────────────────────────────────────┤         │
│ │ [Cánh Cửa Khóa Cẩn Mật: Khối lệnh unsafe { ... }]                     │         │
│ │ Chỉ kỹ sư có chứng chỉ, đeo găng tay cách điện chuyên dụng mới vào.  │         │
│ ├──────────────────────────────────────────────────────────────────────┤         │
│ │ [Bên Trong: Lõi dây đồng 220,000 Volts (Con trỏ thô Raw Pointers)]   │         │
│ │ Chạm tay trần vào đây là nổ tung lập tức (Undefined Behavior)!       │         │
│ └──────────────────────────────────────────────────────────────────────┘         │
│                                                                                  │
│ [2. CỬA KHẨU HẢI QUAN BIÊN GIỚI (FOREIGN FUNCTION INTERFACE - FFI)]             │
│   Nước Rust (Kỷ luật nghiêm ngặt)  ◄───────►  Nước C (Vùng đất tự do hoang dã)  │
│                 │                                     │                          │
│                 ▼                                     ▼                          │
│   Hàng hóa kiểm tra quét X-quang        Thương lái mang hàng qua lại             │
│   (#[repr(C)], CString, Pointer check)  (extern "C", Raw pointers, libc)         │
└──────────────────────────────────────────────────────────────────────────────────┘
```

### 1. Phòng biến áp cao thế và Ổ cắm an toàn (Unsafe vs Safe Wrapper)
- **Safe Rust giống như hệ thống điện dân dụng trong nhà bạn**: Tất cả dây dẫn đều được bọc nhựa cách điện, ổ cắm có nắp che an toàn. Trẻ em có thể cắm sạc điện thoại thoại mái mà không thể bị điện giật.
- **Unsafe Rust giống như phòng trạm biến áp cao thế `220,000 Volts`**: Để cấp điện cho cả thành phố, bắt buộc phải có những thanh đồng trần mang dòng điện cực lớn.
- Kỹ sư điện bước vào phòng biến áp phải mặc đồ bảo hộ chuyên dụng (từ khóa `unsafe`). Họ phải tự chịu trách nhiệm 100% về mạng sống của mình.
- Sau khi đấu nối xong, họ đóng cửa phòng trạm, khóa van bảo vệ lại. Bên ngoài chỉ để lộ ra một chiếc công tắc bật/tắt đơn giản (**Safe Abstraction Wrapper**). Người dân chỉ cần dùng công tắc đó một cách an toàn mà không cần biết bên trong chứa dây điện cao thế nguy hiểm ra sao!

### 2. Trạm kiểm soát hải quan tại cửa khẩu (FFI)
- Hãy tưởng tượng Safe Rust là một quốc gia có luật lệ giao thông cực kỳ nghiêm ngặt: Mọi người dân đều thắt dây an toàn, đi đúng làn đường.
- C/C++ là quốc gia láng giềng tự do: Không có biển báo giao thông, xe máy và ô tô có thể chạy bất kỳ tốc độ nào.
- Khi muốn giao thương giữa hai nước (FFI - Foreign Function Interface), ta phải đặt một **Trạm hải quan quốc tế** (`extern "C"`).
- Xe chở hàng từ nước C muốn sang nước Rust phải được kiểm tra giấy tờ, cân tải trọng (`#[repr(C)]`), đóng gói quy chuẩn trước khi lăn bánh vào lãnh thổ Rust.

---

## Khái niệm & Cơ chế kỹ thuật chuyên sâu (In-Depth Technical Mechanics)

### 1. Năm Siêu năng lực của Unsafe Rust (The 5 Unsafe Superpowers)

Trình biên dịch `rustc` có một "cảnh sát bộ nhớ" là Borrow Checker. Khi bạn viết từ khóa `unsafe`, bạn **không** làm tắt đi trình biên dịch và cũng **không** làm mất đi hệ thống kiểm tra kiểu dữ liệu. Bạn chỉ mở khóa đúng **5 hành động đặc quyền** sau:

1. **Giải tham chiếu con trỏ thô (Dereferencing raw pointers)**:
   - Trong Safe Rust, bạn chỉ có tham chiếu an toàn `&T` hoặc `&mut T` (không bao giờ null, luôn trỏ vào ô nhớ hợp lệ).
   - Trong Unsafe Rust, bạn có con trỏ thô: `*const T` (con trỏ hằng) và `*mut T` (con trỏ khả biến). Chúng có thể trỏ vào bất kỳ địa chỉ số nguyên nào, có thể là `null`, hoặc trỏ vào vùng nhớ đã bị thu hồi. Chỉ khi bạn dùng toán tử `*ptr` để đọc/ghi thì mới bắt buộc phải nằm trong khối `unsafe`.
2. **Gọi một hàm hoặc phương thức không an toàn (Calling an unsafe function or method)**:
   - Các hàm có từ khóa `unsafe fn` (ví dụ các hàm cấp phát bộ nhớ cấp thấp, thao tác SIMD, hoặc các hàm gọi qua FFI).
3. **Hiện thực hóa một Trait không an toàn (Implementing an unsafe trait)**:
   - Các trait như `Send` và `Sync`. Khi bạn cam kết với trình biên dịch rằng cấu trúc dữ liệu của bạn an toàn khi chuyển qua các luồng (threads), bạn phải chịu trách nhiệm đảm bảo không có Data Race.
4. **Thay đổi giá trị của một biến tĩnh khả biến (`static mut`)**:
   - Biến toàn cục khả biến có thể bị đọc/ghi đồng thời bởi nhiều luồng khác nhau mà không có khóa đồng bộ, gây ra xung đột dữ liệu nguy hiểm.
5. **Truy cập các trường của một `union`**:
   - `union` chia sẻ chung một vùng nhớ vật lý cho nhiều kiểu dữ liệu khác nhau (thường dùng khi tương thích với mã C). Rust không thể xác minh kiểu dữ liệu nào đang thực sự nằm trong ô nhớ.

### 2. Nguyên tắc Đóng gói Bao bọc An toàn (Safe Abstraction Invariants)

Hãy nhìn vào cách thư viện chuẩn của Rust (`std`) hiện thực hóa kiểu `Vec<T>`:
- Bản chất `Vec<T>` chứa một con trỏ thô `ptr: *mut T`, sức chứa `cap: usize`, và độ dài `len: usize`.
- Việc cấp phát ô nhớ và mở rộng dung lượng đều sử dụng mã `unsafe`.
- Nhưng người dùng bình thường gọi `vec.push(42)` hay `vec[0]` hoàn toàn trong Safe Rust!
- Tại sao? Bởi vì các kỹ sư Rust đã thiết lập các **Bất biến an toàn (Invariants)**:
  1. `ptr` luôn trỏ vào vùng nhớ có dung lượng ít nhất `cap * size_of::<T>()`.
  2. `len` luôn nhỏ hơn hoặc bằng `cap`.
  3. Mọi phần tử từ chỉ số `0` đến `len - 1` đều đã được khởi tạo hợp lệ.
  4. Khi `Vec` bị tiêu hủy, phương thức `drop()` sẽ giải phóng chính xác vùng nhớ đó đúng 1 lần duy nhất.

### 3. Giao diện Giao tiếp Hàm Ngoại lai (FFI - Foreign Function Interface)

Khi gọi một hàm viết bằng ngôn ngữ C từ Rust:
1. **Quy ước gọi hàm C (`extern "C"`)**: Đảm bảo thanh ghi CPU và ngăn xếp tuân thủ đúng chuẩn C ABI (Application Binary Interface) của hệ điều hành.
2. **Bố cục bộ nhớ tương thích (`#[repr(C)]`)**: Mặc định, trình biên dịch Rust có quyền sắp xếp lại thứ tự các trường trong `struct` để tối ưu hóa bộ nhớ đệm (buffer) (cache). Thuộc tính `#[repr(C)]` buộc Rust phải sắp xếp các trường y hệt như trình biên dịch C (GCC/Clang).
3. **Xử lý chuỗi ký tự**: Chuỗi trong C kết thúc bằng byte số không (`\0` - Null-terminated string). Trong Rust, chuỗi `&str` và `String` lưu kèm độ dài và không bắt buộc có byte `\0`. Rust cung cấp `std::ffi::CString` (sở hữu vùng nhớ kết thúc bằng `\0`) và `std::ffi::CStr` (tham chiếu mượn (borrow) chuỗi C) để chuyển đổi an toàn tuyệt đối.

### 4. Khái niệm Undefined Behavior (UB) & Công cụ Miri

Hành vi bất định (Undefined Behavior) là cơn ác mộng lớn nhất trong lập trình cấp thấp. Khi chương trình chạm vào UB, trình biên dịch được phép giả định điều đó không bao giờ xảy ra, dẫn tới việc tối ưu hóa sai lệch, sinh ra mã máy kỳ dị hoặc tạo ra lỗ hổng bảo mật.
- Một số ví dụ về UB trong Rust:
  - Giải tham chiếu con trỏ thô `null` hoặc con trỏ lơ lửng (dangling).
  - Vi phạm quy tắc mượn (borrow): Tạo ra hai tham chiếu `&mut` tới cùng một ô nhớ trong cùng một thời điểm.
  - Ép kiểu một số nguyên thành kiểu `bool` có giá trị khác `0` hoặc `1`.
- **Miri**: Trình thông dịch trung gian chính thức của Rust (`cargo miri run`/`cargo miri test`), có khả năng phát hiện các hành vi rò rỉ bộ nhớ, Use-After-Free, và vi phạm quyền mượn (borrow) (Stacked Borrows) ngay khi chạy kiểm thử!

---

## Mã nguồn minh họa thực chiến (Idiomatic Runnable Rust Blueprint)

Dưới đây là mã nguồn Rust hoàn chỉnh thể hiện trọn vẹn triết lý: Tự tay xây dựng một cấu trúc **Bộ nhớ đệm (buffer) an toàn** mang tên `SafeRawBuffer` bọc kín mã `unsafe` bên trong, tuân thủ nghiêm ngặt các bất biến an toàn, kết hợp với gọi hàm chuẩn C thông qua FFI:

```rust
use std::alloc::{alloc, dealloc, Layout};
use std::ffi::CStr;
use std::os::raw::c_char;

/// Cấu trúc dữ liệu tương thích 100% với định dạng bộ nhớ C ABI
#[repr(C)]
#[derive(Debug, Clone, Copy)]
pub struct NativePoint {
    pub x: i32,
    pub y: i32,
}

/// Một cấu trúc bao bọc an toàn (Safe Abstraction Wrapper)
/// tự quản lý con trỏ thô cấp thấp trên Heap mà không gây rò rỉ bộ nhớ
pub struct SafeRawBuffer {
    ptr: *mut u8,
    capacity: usize,
    layout: Layout,
}

impl SafeRawBuffer {
    /// Khởi tạo bộ đệm với dung lượng chỉ định (Cấp phát thô an toàn)
    pub fn with_capacity(capacity: usize) -> Result<Self, &'static str> {
        if capacity == 0 {
            return Err("Dung lượng bộ đệm phải lớn hơn 0");
        }

        // Tạo bố cục bộ nhớ (Memory Layout) với căn lề 8 bytes
        let layout = Layout::array::<u8>(capacity)
            .map_err(|_| "Lỗi tính toán kích thước bố cục bộ nhớ")?;

        // Thao tác cấp phát thô nằm trong khối unsafe
        let raw_ptr = unsafe { alloc(layout) };

        if raw_ptr.is_null() {
            return Err("Hệ thống cạn kiệt bộ nhớ: Cấp phát con trỏ thô thất bại!");
        }

        // Khởi tạo các byte về 0 để tránh đọc dữ liệu rác
        unsafe {
            std::ptr::write_bytes(raw_ptr, 0, capacity);
        }

        Ok(Self {
            ptr: raw_ptr,
            capacity,
            layout,
        })
    }

    /// Ghi dữ liệu vào vị trí offset với kiểm tra biên tuyệt đối
    pub fn write_byte(&mut self, offset: usize, value: u8) -> Result<(), &'static str> {
        if offset >= self.capacity {
            return Err("Chỉ số vượt quá giới hạn dung lượng bộ đệm!");
        }

        // Thao tác unsafe được kiểm chứng an toàn 100% bởi ranh giới offset < capacity
        unsafe {
            let target_ptr = self.ptr.add(offset);
            *target_ptr = value;
        }

        Ok(())
    }

    /// Đọc dữ liệu tại vị trí offset an toàn
    pub fn read_byte(&self, offset: usize) -> Option<u8> {
        if offset >= self.capacity {
            return None;
        }

        unsafe {
            let target_ptr = self.ptr.add(offset);
            Some(*target_ptr)
        }
    }

    pub fn capacity(&self) -> usize {
        self.capacity
    }
}

// Tự động giải phóng con trỏ thô khi cấu trúc ra khỏi phạm vi (RAII Pattern)
impl Drop for SafeRawBuffer {
    fn drop(&mut self) {
        if !self.ptr.is_null() {
            println!("    [Drop] Đang giải phóng con trỏ thô tại địa chỉ {:p}...", self.ptr);
            unsafe {
                dealloc(self.ptr, self.layout);
            }
            self.ptr = std::ptr::null_mut();
        }
    }
}

// Giả lập khai báo hàm FFI tương thích chuẩn C
extern "C" {
    // Gọi hàm đo độ dài chuỗi kinh điển strlen trong thư viện C chuẩn (libc)
    fn strlen(s: *const c_char) -> usize;
}

fn main() {
    println!("==================================================================");
    println!("   KIEM CHUNG AN TOAN BO NHO: UNSAFE RUST & FFI DONG GOI CHUAN   ");
    println!("==================================================================");

    // -------------------------------------------------------------
    // 1. THỬ NGHIỆM BỘ ĐỆM CẤP THẤP ĐÓNG GÓI AN TOÀN (SAFE WRAPPER)
    // -------------------------------------------------------------
    println!("\n[1] Khoi tao SafeRawBuffer dong goi con tro tho Heap:");
    {
        let mut my_buffer = SafeRawBuffer::with_capacity(32).expect("Khoi tao that bai");
        println!("    - Khoi tao thanh cong bo dem dung luong: {} bytes", my_buffer.capacity());

        // Ghi dữ liệu an toàn
        my_buffer.write_byte(0, 0xDE).unwrap();
        my_buffer.write_byte(1, 0xAD).unwrap();
        my_buffer.write_byte(2, 0xBE).unwrap();
        my_buffer.write_byte(3, 0xEF).unwrap();

        println!("    - Doc byte tai index 0: 0x{:02X}", my_buffer.read_byte(0).unwrap());
        println!("    - Doc byte tai index 1: 0x{:02X}", my_buffer.read_byte(1).unwrap());

        // Thử nghiệm truy cập ngoài biên an toàn
        let out_of_bounds = my_buffer.write_byte(100, 0xFF);
        println!("    - Thu ghi vao index = 100: {:?}", out_of_bounds);
        assert!(out_of_bounds.is_err());
        println!("    => Lop vo Safe Wrapper da chan dung hanh vi vi pham bien!");
    } // my_buffer tự động được giải phóng an toàn tại đây thông qua drop()!

    // -------------------------------------------------------------
    // 2. THỬ NGHIỆM GIAO TIẾP HÀM NGOẠI LAI (FFI VỚI C ABI)
    // -------------------------------------------------------------
    println!("\n[2] Thu nghiem Foreign Function Interface (FFI) voi C Library:");

    // Tạo chuỗi an toàn tương thích C kết thúc bằng byte \0
    let c_greeting = std::ffi::CString::new("Hello from Rust via C ABI!").unwrap();

    // Gọi hàm strlen của C bên trong khối unsafe có kiểm soát
    let length_from_c = unsafe {
        let raw_c_ptr = c_greeting.as_ptr();
        strlen(raw_c_ptr)
    };

    println!("    - Chuoi gui sang C : {:?}", c_greeting);
    println!("    - Do dai do boi C strlen: {} bytes", length_from_c);
    assert_eq!(length_from_c, 26);

    // -------------------------------------------------------------
    // 3. THỬ NGHIỆM CẤU TRÚC ĐỊNH DẠNG TƯƠNG THÍCH #[repr(C)]
    // -------------------------------------------------------------
    println!("\n[3] Kiem tra tuong thich bo cuc bo nho #[repr(C)]:");
    let pt = NativePoint { x: 100, y: 200 };
    println!("    - Toa do diem C-compatible: x = {}, y = {}", pt.x, pt.y);
    println!("    - Kich thuoc struct NativePoint: {} bytes (dung bang 2 * i32)", std::mem::size_of::<NativePoint>());
    assert_eq!(std::mem::size_of::<NativePoint>(), 8);

    println!("\n==================================================================");
    println!("   XAC NHAN: UNSAFE & FFI HOAT DONG AN TOAN DUNG QUY CHUAN!      ");
    println!("==================================================================");
}
```

---

## Bảng tra cứu lỗi biên dịch & Cách khắc phục (Compiler Error Guide)

Dưới đây là các lỗi biên dịch thường gặp nhất khi làm việc với `unsafe` và FFI trong Rust:

| Mã lỗi | Thông báo mẫu từ trình biên dịch | Nguyên nhân cốt lõi | Cách khắc phục nhanh |
|---|---|---|---|
| **E0133** | `call to unsafe function requires unsafe function or block` | Bạn gọi một hàm ngoại lai `extern "C"` hoặc giải tham chiếu con trỏ thô mà quên đặt trong khối `unsafe { ... }`. | Bọc dòng lệnh đó vào bên trong một khối lệnh `unsafe { ... }` và bổ sung chú thích lý do an toàn. |
| **E0606** | `cannot cast '&T' as '*mut T'` | Bạn cố gắng ép kiểu một tham chiếu mượn (borrow) bất biến trực tiếp sang một con trỏ thô khả biến. | Ép kiểu qua con trỏ hằng trước: `&val as *const T as *mut T`, hoặc dùng tham chiếu khả biến `&mut val as *mut T`. |
| **E0277** | `the trait 'Send' is not implemented for '*const u8'` | Con trỏ thô mặc định không tự động triển khai trait `Send` và `Sync` để ngăn chặn việc truyền dữ liệu bất cẩn qua các luồng. | Đóng gói con trỏ thô bên trong một `struct` và tự triển khai `unsafe impl Send for MyWrapper {}` nếu cam kết đồng bộ an toàn. |
| **E0507** | `cannot move out of a raw pointer` | Cố gắng lấy quyền sở hữu (ownership) của một giá trị nằm sau con trỏ thô mà không sao chép dữ liệu. | Sử dụng hàm `std::ptr::read(raw_ptr)` để sao chép dữ liệu ra ngoài một cách có ý thức. |

### Ví dụ phân tích lỗi `E0133` khi gọi hàm ngoại lai không có khối `unsafe`:

```rust
// Giả lập hàm cấp thấp nguy hiểm
unsafe fn xoa_o_dia_cap_thap() {
    println!("Thao tác cấp thấp nguy hiểm đã chạy!");
}

// Đoạn mã lỗi minh họa E0133:
fn vi_du_loi_e0133() {
    // xoa_o_dia_cap_thap(); // LỖI E0133: Trình biên dịch cấm gọi hàm unsafe trực tiếp!
}

// Cách sửa chữa đúng chuẩn:
fn vi_du_dung_e0133() {
    // Phải có khối lệnh unsafe thể hiện trách nhiệm của lập trình viên
    unsafe {
        xoa_o_dia_cap_thap();
    }
}
```

---

## Tóm tắt chương & Bài tập rèn luyện (Summary & Exercises)

### 4 Điểm cốt lõi cần ghi nhớ:
1. **Unsafe không phải là vô pháp**: Unsafe Rust chỉ mở khóa đúng 5 siêu năng lực cấp thấp. Toàn bộ các quy tắc về kiểu dữ liệu, thời gian sống (lifetime), và cú pháp vẫn được kiểm tra bình thường.
2. **Nguyên tắc Bao bọc an toàn (Safe Abstraction)**: Mã nguồn cấp thấp nguy hiểm được đóng kín bên trong cấu trúc dữ liệu, chỉ để lộ ra các phương thức công khai an toàn tuyệt đối cho người dùng.
3. **Cầu nối FFI với C**: Sử dụng `extern "C"`, thuộc tính căn lề bộ nhớ `#[repr(C)]`, cùng các con trỏ thông minh (smart pointer) như `Box` và chuỗi `CString` để giao tiếp mượt mà với thư viện C.
4. **Triệt tiêu Undefined Behavior**: Tôn trọng các bất biến an toàn và tận dụng công cụ kiểm định Miri để đảm bảo không bao giờ tồn tại lỗi vi phạm bộ nhớ ngầm.

### Bài tập rèn luyện tự giải:
1. **Bài tập 1 (Tự viết hàm hoán đổi Swap bằng con trỏ thô)**:  
   Viết một hàm `unsafe fn raw_swap<T>(a: *mut T, b: *mut T)`. Sử dụng các hàm thao tác con trỏ thô như `std::ptr::read` và `std::ptr::write` để tráo đổi giá trị giữa hai ô nhớ mà không làm hỏng dữ liệu. Hãy viết một hàm bọc an toàn `fn safe_swap<T>(a: &mut T, b: &mut T)` bên ngoài.
2. **Bài tập 2 (Gọi hàm toán học C qua FFI)**:  
   Khai báo hàm `sqrt` (tính căn bậc hai) từ thư viện toán học của C: `extern "C" { fn sqrt(x: f64) -> f64; }`. Viết một chương trình Rust gọi hàm này và so sánh kết quả với phương thức `.sqrt()` có sẵn của Rust.
3. **Bài tập 3 (Suy ngẫm kiến trúc: Tại sao `Send` và `Sync` lại là `unsafe trait`?)**:  
   Tại sao trình biên dịch Rust không tự động suy diễn trait `Send` cho các cấu trúc chứa con trỏ thô? Nếu một lập trình viên tự ý đánh dấu `unsafe impl Send` cho một đối tượng chứa con trỏ thô dùng chung mà không có cơ chế khóa bảo vệ (như Mutex), nguy cơ rủi ro nào sẽ xảy ra khi chạy đa luồng?
