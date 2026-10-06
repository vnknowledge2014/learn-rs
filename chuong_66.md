# Chương 66: Lập trình nhúng & `no_std` — Rust Trên Con Chip 32 KB RAM (Embedded Rust)

## Giới thiệu & Mục tiêu học tập

Mọi chương trước đều ngầm giả định ba thứ: có hệ điều hành, có bộ nhớ heap, và có ai đó dọn dẹp khi chương trình kết thúc. Trên vi điều khiển, **không có thứ nào trong ba thứ đó**.

Không có `println!` — không có màn hình. Không có `Vec` — không có bộ cấp phát. Không có `panic!` in ra thông báo — không có nơi để in. Chương trình của bạn là thứ **duy nhất** chạy trên con chip, và nó phải chạy liên tục nhiều năm không được khởi động lại.

Đây cũng là nơi Rust tỏa sáng nhất. Ngành nhúng vốn là lãnh địa của C, nơi một con trỏ sai làm rơi máy bay không người lái. Rust mang tới điều C không thể: **an toàn bộ nhớ mà không cần bộ dọn rác, không cần runtime**.

Chương này dựa trên [The Embedded Rust Book](https://github.com/rust-embedded/book) — tài liệu chính thức của Nhóm làm việc Nhúng.

Mục tiêu học tập:
- Hiểu `no_std` nghĩa là gì và **mất những gì**.
- Thao tác **thanh ghi ánh xạ bộ nhớ** (MMIO) và hiểu vì sao `volatile` là bắt buộc.
- Áp dụng **typestate** (Chương 20) cho chân GPIO: đọc từ chân đầu ra thành **lỗi biên dịch**.
- Cài mẫu **Singleton ngoại vi** — "chỉ có một bộ ngoại vi trên chip này".
- Tính toán bằng **số dấu phẩy tĩnh Q16.16** vì phần lớn vi điều khiển không có FPU.
- Viết **bộ đệm vòng không cấp phát** và **bộ chống rung phím** — hai kiểu dữ liệu chủ lực của lập trình nhúng.

---

## Hình tượng hóa đời sống (Intuitive Everyday Analogy)

```
┌──────────────────────────────────────────────────────────────────────────────┐
│    HÌNH TƯỢNG: LẬP TRÌNH MÁY TÍNH vs LẬP TRÌNH LÒ VI SÓNG                    │
├──────────────────────────────────────────────────────────────────────────────┤
│                                                                              │
│  MÁY TÍNH (std)                    │  LÒ VI SÓNG (no_std)                    │
│  ─────────────────                 │  ────────────────────                   │
│  Có quản gia (hệ điều hành):       │  Bạn là TẤT CẢ. Không ai giúp.          │
│    - dọn dẹp khi bạn quên          │    - quên tắt = chập điện               │
│    - cấp thêm phòng khi cần        │    - hết chỗ = HỎNG, không xin thêm     │
│    - báo lỗi ra màn hình           │    - lỗi = đèn nhấp nháy, hoặc treo im  │
│                                    │                                         │
│  RAM: 16 GB (16 000 000 000 byte)  │  RAM: 32 KB (32 000 byte)               │
│                                    │  → ít hơn 500 000 LẦN                   │
│                                    │                                         │
│  Chạy 8 tiếng rồi tắt máy          │  Chạy 10 NĂM không tắt                  │
│  → rò rỉ nhỏ không sao             │  → rò rỉ 1 byte/giờ = chết sau 4 năm    │
│                                    │                                         │
├──────────────────────────────────────────────────────────────────────────────┤
│    THANH GHI ÁNH XẠ BỘ NHỚ = CÔNG TẮC ĐIỆN TRÔNG NHƯ Ô GHI CHÚ               │
│                                                                              │
│    Địa chỉ 0x4002_0014 trông y hệt một biến bình thường.                     │
│    Nhưng GHI vào nó = BẬT MỘT BÓNG ĐÈN THẬT trên bảng mạch.                  │
│                                                                              │
│    ⚠ NGUY HIỂM: trình tối ưu hóa thấy bạn "ghi rồi không đọc lại"            │
│      → nó XÓA LỆNH GHI đi cho nhanh → đèn không bao giờ sáng.                │
│      Từ khóa `volatile` nghĩa là: "ĐỪNG THÔNG MINH. GHI THẬT ĐI."            │
│                                                                              │
├──────────────────────────────────────────────────────────────────────────────┤
│    TYPESTATE CHO CHÂN GPIO = Ổ CẮM CÓ HÌNH DẠNG KHÁC NHAU                    │
│                                                                              │
│    Phích cắm 2 chân KHÔNG cắm vừa ổ 3 chân — không phải nhờ cảnh báo,        │
│    mà nhờ HÌNH DẠNG VẬT LÝ. Bạn không thể cắm sai kể cả khi cố tình.         │
│                                                                              │
│    Pin<Output> có .set_high() và .set_low()   ← điều khiển đèn               │
│    Pin<Input>  có .is_high()                  ← đọc nút bấm                  │
│                                                                              │
│    button.set_high()  →  ❌ E0599: không có phương thức `set_high`           │
│    Lỗi bị bắt lúc BIÊN DỊCH, không phải lúc thiết bị đã nằm trong tay khách. │
└──────────────────────────────────────────────────────────────────────────────┘
```

---

## Khái niệm & Cơ chế kỹ thuật chuyên sâu (In-Depth Technical Mechanics)

### 1. `no_std` — bạn mất gì và giữ gì

Thêm `#![no_std]` vào đầu crate là cắt bỏ thư viện chuẩn, chỉ giữ lại `core`:

| Mất (`std`) | Giữ (`core`) |
|---|---|
| `Vec`, `String`, `HashMap`, `Box` | mảng, lát cắt, `&str`, tuple |
| `println!`, `File`, `TcpStream` | `Option`, `Result`, iterator |
| `std::thread`, `Mutex` | trait, generic, macro, closure |
| Bộ cấp phát heap | **Toàn bộ hệ thống kiểu và borrow checker** |

Điều quan trọng: bạn **không mất** thứ làm nên Rust. Quyền sở hữu, vòng đời, trait, iterator, `Option`/`Result`, khớp mẫu — tất cả nằm trong `core`, đều dùng được.

Nếu con chip có đủ RAM, bạn còn có thể thêm crate `alloc` để lấy lại `Vec` và `String` với một bộ cấp phát tự viết. Nhưng phần lớn mã nhúng nghiêm túc **cố tình** không dùng heap: cấp phát động có thời gian thực thi không đoán trước được, và phân mảnh heap sau vài năm chạy là án tử.

### 2. Vì sao `volatile` là bắt buộc

Xét đoạn mã bật đèn rồi tắt:

```rust
// ❌ SAI — không có volatile
unsafe { *(0x4002_0014 as *mut u32) = 1; }   // bật đèn
unsafe { *(0x4002_0014 as *mut u32) = 0; }   // tắt đèn

// ✅ ĐÚNG — mỗi lệnh ghi đều phải xảy ra thật
use core::ptr::write_volatile;
unsafe { write_volatile(0x4002_0014 as *mut u32, 1); }
unsafe { write_volatile(0x4002_0014 as *mut u32, 0); }
```

Trình tối ưu hóa lý luận: "ghi 1 rồi ghi 0 vào cùng chỗ mà không đọc ở giữa — lệnh đầu vô nghĩa, xóa đi." Kết quả: đèn không bao giờ nhấp nháy. Tệ hơn, nếu ta ghi trong vòng lặp mà không đọc, cả vòng lặp có thể bị xóa sạch.

`read_volatile`/`write_volatile` nói với trình biên dịch: **mỗi** thao tác đều có tác dụng phụ ngoài tầm hiểu biết của mày, đừng gộp, đừng xóa, đừng đảo thứ tự. Trong chương này, `FakeRegisters` đếm số lần đọc/ghi để bạn *thấy* được điều đó trong bài kiểm thử.

### 3. Đọc-Sửa-Ghi và cái bẫy ngắt

Muốn bật bit 3 mà không đụng các bit khác, phải làm ba bước: **đọc** giá trị hiện tại, **sửa** bit, **ghi** lại. Nhưng nếu một ngắt xảy ra *giữa* bước đọc và bước ghi, và ngắt đó cũng sửa cùng thanh ghi, thay đổi của nó sẽ bị ghi đè mất.

Ba cách xử lý:
1. **Vùng găng** (critical section): tắt ngắt trong lúc đọc-sửa-ghi. Đơn giản nhưng làm tăng độ trễ ngắt.
2. **Thanh ghi bit-band**: nhiều vi điều khiển ARM cung cấp vùng địa chỉ mà mỗi bit có địa chỉ riêng — ghi một bit thành một lệnh duy nhất, không thể bị cắt ngang.
3. **Thanh ghi set/clear riêng**: STM32 có `BSRR` — ghi 1 vào bit `n` thì bật chân `n`, ghi 1 vào bit `n+16` thì tắt chân `n`. Không cần đọc trước.

### 4. Typestate: chuyển trạng thái phải **tiêu thụ** giá trị cũ

Điểm mấu chốt của typestate là chữ ký hàm:

```rust
pub fn into_output(self, moder: &FakeRegisters) -> Pin<Output>
//                  ^^^^ nhận `self` theo GIÁ TRỊ, không phải `&self`
```

Vì nhận `self`, chân cũ bị **di chuyển** và không dùng lại được. Nhờ vậy không bao giờ tồn tại đồng thời hai cách nhìn về cùng một chân phần cứng. Nếu dùng `&self`, bạn có thể tạo `Pin<Output>` mà vẫn giữ `Pin<Input>` cũ — và trình biên dịch sẽ vui vẻ cho phép bạn vừa đọc vừa ghi cùng một chân.

Chi phí lúc chạy: **bằng không**. `PhantomData<Output>` không chiếm byte nào; `size_of::<Pin<Output>>() == size_of::<u8>()` (chỉ còn trường số hiệu chân). Toàn bộ kiểm tra biến mất sau khi biên dịch.

### 5. Số dấu phẩy tĩnh Q16.16

Cortex-M0/M0+ không có bộ xử lý dấu phẩy động. Mọi phép `f32` bị mô phỏng bằng phần mềm — chậm hơn hàng chục lần và tốn hàng KB flash.

Giải pháp: đặt dấu phẩy ở một vị trí **cố định** trong số nguyên. Q16.16 dùng 16 bit cho phần nguyên, 16 bit cho phần thập phân, tất cả trong một `i32`:

```
   Giá trị thực 3.5  →  3.5 × 65536  =  229376  =  0x0003_8000
                          ▲                          ▲▲▲▲ ▲▲▲▲
                          hằng số 2^16               nguyên│thập phân
```

- **Cộng/trừ**: cộng trừ số nguyên bình thường. Chính xác tuyệt đối.
- **Nhân**: phải qua `i64` rồi dịch phải 16 — nếu nhân thẳng hai `i32`, ngay cả `1.0 × 1.0` (giá trị thô `2^16 × 2^16 = 2^32`) cũng tràn.
- **Chia**: dịch trái 16 **trước** khi chia, nếu không mất hết phần thập phân.

Trong mã, `Q16` cài các trait `core::ops::{Add, Sub, Mul, Div}` nên viết được `a * b - c` như số thường; còn `Q16::from_real` là `const fn`, nên hằng số như `165.0 / 4095.0` được tính **lúc biên dịch** — trên chip chỉ còn một số nguyên.

Độ phân giải là `1/65536 ≈ 0.0000153` (mỗi phép nhân/chia làm tròn sai tối đa một đơn vị cuối) — thừa đủ cho cảm biến nhiệt độ, điều khiển động cơ, hay bộ lọc âm thanh.

### 6. `AtomicBool` thay cho `static mut`

Mẫu Singleton cần một cờ toàn cục "đã giao ngoại vi chưa". Viết bằng `static mut` là sai:

```rust
// ❌ SAI — có cửa sổ đua
if !TAKEN { TAKEN = true; hand_out_peripherals() }
//         ▲ một ngắt chen vào ĐÂY sẽ khiến ngoại vi bị giao HAI lần
```

`AtomicBool::swap` làm cả hai việc trong **một** thao tác không thể bị cắt ngang: đặt giá trị mới *và* trả về giá trị cũ. Nếu giá trị cũ là `true`, ta biết chắc có người lấy trước — không có khe hở nào.

---

## Mã nguồn minh họa thực chiến (Idiomatic Runnable Rust Blueprint)

Mã dưới đây chạy được trên máy tính để bàn (để kiểm thử được). Trên vi điều khiển thật, bạn thêm `#![no_std]` + `#![no_main]`, thay `FakeRegisters` bằng `read_volatile`/`write_volatile` trên địa chỉ thật, và dùng crate HAL của dòng chip (`stm32f4xx-hal`, `rp2040-hal`, `esp-hal`…). Tên phương thức `set_high`/`set_low`/`is_high` cố ý trùng với trait `OutputPin`/`InputPin` của `embedded-hal` 1.0 — giao diện chung mà mọi HAL đều cài.

Chạy bằng `cargo run -p ch66`, kiểm thử bằng `cargo test -p ch66`.

```rust
//! Chương 66 — Lập trình nhúng & `no_std`: thanh ghi ánh xạ bộ nhớ, mẫu Singleton
//! cho ngoại vi, typestate cho chân GPIO, số dấu phẩy tĩnh, và bộ đệm vòng không cấp phát.
//!
//! Ghi chú: tệp này chạy trên máy tính để bàn để KIỂM THỬ ĐƯỢC. Trên vi điều khiển
//! thật, bạn thêm `#![no_std]` + `#![no_main]` và thay `FakeRegisters` bằng địa chỉ thật.

use core::marker::PhantomData;
use std::cell::Cell;
use std::sync::atomic::{AtomicBool, Ordering};

// ============================================================================
// 1. THANH GHI ÁNH XẠ BỘ NHỚ (MMIO) — phần cứng trông như biến
// ============================================================================

/// Trên vi điều khiển, ghi vào địa chỉ 0x4002_0014 sẽ BẬT một chân đèn.
/// Không có `volatile`, trình tối ưu hóa có quyền xóa lệnh ghi đó — vì theo
/// nó, ghi vào bộ nhớ rồi không đọc lại là việc vô nghĩa.
pub struct FakeRegisters {
    value: Cell<u32>,
    pub write_count: Cell<u32>,
    pub read_count: Cell<u32>,
}

impl FakeRegisters {
    pub fn new(value: u32) -> Self {
        FakeRegisters {
            value: Cell::new(value),
            write_count: Cell::new(0),
            read_count: Cell::new(0),
        }
    }
    /// Tương ứng `core::ptr::write_volatile` — MỖI lệnh ghi đều phải xảy ra thật.
    pub fn write(&self, v: u32) {
        self.value.set(v);
        self.write_count.set(self.write_count.get() + 1);
    }
    /// Tương ứng `core::ptr::read_volatile` — không được lưu vào thanh ghi CPU dùng lại.
    pub fn read(&self) -> u32 {
        self.read_count.set(self.read_count.get() + 1);
        self.value.get()
    }

    /// Đọc-Sửa-Ghi: mẫu thao tác bit chuẩn của lập trình nhúng.
    pub fn set_bit(&self, bit: u8) {
        self.write(self.read() | (1 << bit));
    }
    pub fn clear_bit(&self, bit: u8) {
        self.write(self.read() & !(1 << bit));
    }
    pub fn toggle_bit(&self, bit: u8) {
        self.write(self.read() ^ (1 << bit));
    }
    pub fn test_bit(&self, bit: u8) -> bool {
        self.read() & (1 << bit) != 0
    }

    /// Ghi một trường nhiều bit mà KHÔNG đụng các bit khác.
    pub fn write_field(&self, offset: u8, width: u8, value: u32) {
        let mask = ((1u32 << width) - 1) << offset;
        self.write((self.read() & !mask) | ((value << offset) & mask));
    }
    pub fn read_field(&self, offset: u8, width: u8) -> u32 {
        (self.read() >> offset) & ((1u32 << width) - 1)
    }
}

// ============================================================================
// 2. TYPESTATE CHO CHÂN GPIO — cấu hình sai KHÔNG BIÊN DỊCH ĐƯỢC
// ============================================================================
// Đây là Chương 20 (Typestate) áp dụng vào phần cứng: trạng thái của chân
// nằm trong KIỂU, nên trình biên dịch chặn "đọc từ chân đang ở chế độ ra".

pub struct Unconfigured;
pub struct Input;
pub struct Output;
pub struct Analog; // analog — cho ADC

pub struct Pin<Mode> {
    number: u8,
    _mode: PhantomData<Mode>,
}

impl Pin<Unconfigured> {
    /// Tạo một chân chưa cấu hình.
    ///
    /// # Safety
    /// Người gọi phải bảo đảm không có `Pin` nào khác cùng số hiệu tồn tại:
    /// hai `Pin` cùng số hiệu sẽ phá vỡ độc quyền phần cứng. Trong thực tế
    /// bạn chỉ gọi nó qua Singleton ở mục 3.
    pub unsafe fn new(number: u8) -> Self {
        Pin {
            number,
            _mode: PhantomData,
        }
    }
}

impl<Mode> Pin<Mode> {
    pub fn number(&self) -> u8 {
        self.number
    }
    /// Chuyển chế độ TIÊU THỤ chân cũ (`self`) và trả về chân kiểu mới.
    /// Nhờ vậy không tồn tại đồng thời hai cách nhìn về cùng một chân.
    pub fn into_output(self, moder: &FakeRegisters) -> Pin<Output> {
        moder.write_field(self.number * 2, 2, 0b01); // MODER = 01 (output)
        Pin {
            number: self.number,
            _mode: PhantomData,
        }
    }
    pub fn into_input(self, moder: &FakeRegisters) -> Pin<Input> {
        moder.write_field(self.number * 2, 2, 0b00); // MODER = 00 (input)
        Pin {
            number: self.number,
            _mode: PhantomData,
        }
    }
    pub fn into_analog(self, moder: &FakeRegisters) -> Pin<Analog> {
        moder.write_field(self.number * 2, 2, 0b11); // MODER = 11 (analog)
        Pin {
            number: self.number,
            _mode: PhantomData,
        }
    }
}

// CHỈ chân đầu ra mới có `set_high`/`set_low` — gọi trên chân đầu vào là lỗi biên dịch.
impl Pin<Output> {
    pub fn set_high(&mut self, odr: &FakeRegisters) {
        odr.set_bit(self.number);
    }
    pub fn set_low(&mut self, odr: &FakeRegisters) {
        odr.clear_bit(self.number);
    }
    pub fn toggle(&mut self, odr: &FakeRegisters) {
        odr.toggle_bit(self.number);
    }
}

// CHỈ chân đầu vào mới có `is_high`.
impl Pin<Input> {
    pub fn is_high(&self, idr: &FakeRegisters) -> bool {
        idr.test_bit(self.number)
    }
}

// ============================================================================
// 3. SINGLETON NGOẠI VI — "chỉ có MỘT bộ ngoại vi trên con chip này"
// ============================================================================

/// Gói TẤT CẢ ngoại vi của con chip. Ai cầm được nó là chủ duy nhất của phần cứng.
pub struct Peripherals {
    pub pa5: Pin<Unconfigured>,  // chân LED trên nhiều bo Nucleo
    pub pc13: Pin<Unconfigured>, // chân nút bấm
}

/// Cờ nguyên tử thay cho `static mut`: an toàn cả khi có ngắt xen giữa.
/// `swap` là thao tác ĐỌC-VÀ-ĐẶT không thể bị cắt ngang — nếu dùng
/// `if !TAKEN { TAKEN = true }` thì một ngắt chen vào giữa hai câu lệnh
/// có thể khiến ngoại vi bị giao HAI lần.
static TAKEN: AtomicBool = AtomicBool::new(false);

impl Peripherals {
    /// Trả `Some` đúng MỘT lần trong suốt vòng đời chương trình.
    /// Lần thứ hai trả `None` — không thể có hai chủ sở hữu cùng điều khiển chip.
    pub fn take() -> Option<Peripherals> {
        if TAKEN.swap(true, Ordering::SeqCst) {
            return None; // đã có người lấy trước
        }
        // An toàn: cờ trên bảo đảm đoạn này chạy đúng một lần.
        Some(unsafe {
            Peripherals {
                pa5: Pin::new(5),
                pc13: Pin::new(13),
            }
        })
    }
    #[doc(hidden)]
    pub fn reset_for_test() {
        TAKEN.store(false, Ordering::SeqCst);
    }
}

// ============================================================================
// 4. SỐ DẤU PHẨY TĨNH — vì phần lớn vi điều khiển KHÔNG có FPU
// ============================================================================

/// Q16.16: 16 bit phần nguyên, 16 bit phần thập phân, đựng trong một `i32`.
/// Nhân/chia bằng số nguyên → nhanh gấp hàng chục lần mô phỏng dấu phẩy động.
#[derive(Debug, Clone, Copy, PartialEq, Eq, PartialOrd, Ord)]
pub struct Q16(pub i32);

impl Q16 {
    pub const ONE: Q16 = Q16(1 << 16);
    pub const fn from_int(n: i16) -> Q16 {
        Q16((n as i32) << 16)
    }
    /// `const fn`: dùng để tính HẰNG SỐ lúc biên dịch — chip không cần FPU,
    /// vì phép tính dấu phẩy động chạy trên máy biên dịch chứ không trên chip.
    pub const fn from_real(x: f64) -> Q16 {
        Q16((x * 65536.0).round() as i32)
    }
    pub fn into_real(self) -> f64 {
        self.0 as f64 / 65536.0
    }
}

// Cài các trait toán tử của `core::ops` để viết `a + b`, `a * b` như số thường.
// Cộng/trừ: cộng số nguyên bình thường, chính xác tuyệt đối.
impl core::ops::Add for Q16 {
    type Output = Q16;
    fn add(self, k: Q16) -> Q16 {
        Q16(self.0.wrapping_add(k.0))
    }
}
impl core::ops::Sub for Q16 {
    type Output = Q16;
    fn sub(self, k: Q16) -> Q16 {
        Q16(self.0.wrapping_sub(k.0))
    }
}
/// Nhân phải qua i64 rồi dịch phải 16 — nếu không, ngay cả 1.0 × 1.0 (= 2^32) cũng tràn i32.
impl core::ops::Mul for Q16 {
    type Output = Q16;
    fn mul(self, k: Q16) -> Q16 {
        Q16(((self.0 as i64 * k.0 as i64) >> 16) as i32)
    }
}
/// Chia: dịch trái 16 TRƯỚC khi chia, nếu không mất hết phần thập phân.
impl core::ops::Div for Q16 {
    type Output = Q16;
    fn div(self, k: Q16) -> Q16 {
        Q16((((self.0 as i64) << 16) / k.0 as i64) as i32)
    }
}

/// Chuyển giá trị ADC 12-bit (0..4095) sang nhiệt độ °C, toàn số nguyên.
/// Cảm biến giả định: 0 → -40 °C, 4095 → 125 °C (tuyến tính).
pub fn adc_to_celsius(adc: u16) -> Q16 {
    // Tính sẵn lúc biên dịch: trong tệp nhị phân chỉ còn một hằng số i32.
    const SCALE: Q16 = Q16::from_real(165.0 / 4095.0);
    Q16::from_int(adc as i16) * SCALE - Q16::from_int(40)
}

// ============================================================================
// 5. BỘ ĐỆM VÒNG KHÔNG CẤP PHÁT — `heapless` thu nhỏ
// ============================================================================

/// Không `Vec`, không `Box`, không heap. Bộ nhớ nằm gọn trong struct,
/// kích thước biết trước lúc biên dịch. Đây là kiểu dữ liệu chủ lực của
/// ngắt UART: ISR đẩy byte vào, vòng lặp chính lấy ra.
pub struct RingBuffer<const N: usize> {
    buf: [u8; N],
    /// Vị trí ĐỌC kế tiếp.
    head: usize,
    /// Vị trí GHI kế tiếp.
    tail: usize,
    len: usize,
}

impl<const N: usize> Default for RingBuffer<N> {
    fn default() -> Self {
        Self::new()
    }
}

impl<const N: usize> RingBuffer<N> {
    pub const fn new() -> Self {
        RingBuffer {
            buf: [0; N],
            head: 0,
            tail: 0,
            len: 0,
        }
    }
    pub fn capacity(&self) -> usize {
        N
    }
    pub fn len(&self) -> usize {
        self.len
    }
    pub fn is_empty(&self) -> bool {
        self.len == 0
    }
    pub fn is_full(&self) -> bool {
        self.len == N
    }

    /// Trả `Err` thay vì cấp phát thêm — hệ nhúng KHÔNG được phép "cứ lớn dần".
    pub fn push(&mut self, b: u8) -> Result<(), u8> {
        if self.is_full() {
            return Err(b);
        }
        self.buf[self.tail] = b;
        self.tail = (self.tail + 1) % N;
        self.len += 1;
        Ok(())
    }
    pub fn take(&mut self) -> Option<u8> {
        if self.is_empty() {
            return None;
        }
        let b = self.buf[self.head];
        self.head = (self.head + 1) % N;
        self.len -= 1;
        Some(b)
    }
    /// Ghi đè phần tử cũ nhất khi đầy — dùng cho nhật ký sự cố (black box).
    pub fn push_overwrite(&mut self, b: u8) -> Option<u8> {
        let dropped = if self.is_full() { self.take() } else { None };
        let _ = self.push(b);
        dropped
    }
}

// ============================================================================
// 6. MÁY TRẠNG THÁI KHÔNG CẤP PHÁT — bộ chống rung phím (debounce)
// ============================================================================

/// Nút bấm cơ khí "nảy" hàng chục lần trong vài mili-giây. Không lọc thì
/// một cú bấm thành 20 sự kiện. Bộ lọc: chỉ đổi trạng thái khi đọc được
/// `threshold` mẫu GIỐNG NHAU liên tiếp.
#[derive(Debug, Clone, Copy, PartialEq)]
pub struct Debouncer {
    is_stable: bool,
    count: u8,
    threshold: u8,
}

impl Debouncer {
    pub fn new(threshold: u8) -> Self {
        Debouncer {
            is_stable: false,
            count: 0,
            threshold,
        }
    }
    /// Trả `Some(trạng thái mới)` chỉ tại đúng khoảnh khắc chuyển.
    pub fn update(&mut self, raw_sample: bool) -> Option<bool> {
        if raw_sample == self.is_stable {
            self.count = 0;
            return None;
        }
        self.count += 1;
        if self.count >= self.threshold {
            self.is_stable = raw_sample;
            self.count = 0;
            return Some(self.is_stable);
        }
        None
    }
    pub fn state(&self) -> bool {
        self.is_stable
    }
}

fn main() {
    println!("═══════════════════════════════════════════════════════════");
    println!("   LẬP TRÌNH NHÚNG: MMIO · TYPESTATE GPIO · Q16.16 · no_std ");
    println!("═══════════════════════════════════════════════════════════");

    println!("\n1. THANH GHI ÁNH XẠ BỘ NHỚ");
    let moder = FakeRegisters::new(0);
    let odr = FakeRegisters::new(0);
    moder.write_field(10, 2, 0b01);
    println!(
        "   MODER sau khi đặt chân 5 thành output: 0b{:032b}",
        moder.read()
    );
    println!(
        "   Số lệnh ghi thực sự chạm phần cứng   : {}",
        moder.write_count.get()
    );

    println!("\n2. TYPESTATE GPIO — sai kiểu là không biên dịch được");
    let p = Peripherals::take().expect("lần đầu phải lấy được");
    println!(
        "   Peripherals::take() lần hai trả None? {}",
        Peripherals::take().is_none()
    );
    let mut led = p.pa5.into_output(&moder);
    let button = p.pc13.into_input(&moder);
    led.set_high(&odr);
    println!(
        "   Bật đèn chân {} → ODR = 0b{:016b}",
        led.number(),
        odr.read()
    );
    println!(
        "   Đọc nút chân {} → {}",
        button.number(),
        button.is_high(&odr)
    );
    println!("   ❌ button.set_high(&odr) → E0599: không có phương thức `set_high` cho Pin<Input>");

    println!("\n3. SỐ DẤU PHẨY TĨNH Q16.16 (không cần FPU)");
    for adc in [0u16, 1024, 2048, 4095] {
        let t = adc_to_celsius(adc);
        println!(
            "   ADC {:>4} → {:>8.3} °C (bên trong chỉ là i32 = {})",
            adc,
            t.into_real(),
            t.0
        );
    }
    let a = Q16::from_real(3.5);
    let b = Q16::from_real(2.0);
    println!(
        "   3.5 × 2.0 = {} · 3.5 ÷ 2.0 = {}",
        (a * b).into_real(),
        (a / b).into_real()
    );

    println!("\n4. BỘ ĐỆM VÒNG KHÔNG CẤP PHÁT (4 byte)");
    let mut ring: RingBuffer<4> = RingBuffer::new();
    for b in b"RUST" {
        ring.push(*b).unwrap();
    }
    println!(
        "   Đầy: {} | đẩy thêm 'X' → {:?}",
        ring.is_full(),
        ring.push(b'X').unwrap_err() as char
    );
    println!(
        "   Ghi đè 'X' → mất byte {:?}",
        ring.push_overwrite(b'X').map(|b| b as char)
    );
    let rest: Vec<char> = std::iter::from_fn(|| ring.take())
        .map(|b| b as char)
        .collect();
    println!("   Nội dung còn lại: {:?}", rest);

    println!("\n5. CHỐNG RUNG PHÍM (ngưỡng 3 mẫu)");
    let mut debouncer = Debouncer::new(3);
    let samples = [
        false, true, false, true, true, true, true, false, true, false, false, false,
    ];
    let mut events = Vec::new();
    for (i, &s) in samples.iter().enumerate() {
        if let Some(new) = debouncer.update(s) {
            events.push((i, new));
        }
    }
    println!(
        "   12 mẫu nhiễu → chỉ {} sự kiện thật: {:?}",
        events.len(),
        events
    );

    println!("\n═══════════════════════════════════════════════════════════");
    println!("   NHÚNG = KHÔNG HỆ ĐIỀU HÀNH, KHÔNG HEAP, KHÔNG THA THỨ     ");
    println!("═══════════════════════════════════════════════════════════");
}

#[cfg(test)]
mod tests {
    use super::*;

    // ---------- MMIO ----------
    #[test]
    fn bit_ops_leave_other_bits_alone() {
        let reg = FakeRegisters::new(0b1010_0000);
        reg.set_bit(0);
        assert_eq!(
            reg.read(),
            0b1010_0001,
            "đặt bit 0 phải giữ nguyên bit 5 và 7"
        );
        reg.clear_bit(7);
        assert_eq!(reg.read(), 0b0010_0001);
        reg.toggle_bit(5);
        assert_eq!(reg.read(), 0b0000_0001);
    }

    #[test]
    fn field_write_uses_exactly_its_width() {
        let reg = FakeRegisters::new(0xFFFF_FFFF);
        reg.write_field(4, 3, 0b010); // đặt 3 bit tại vị trí 4
        assert_eq!(reg.read_field(4, 3), 0b010);
        assert_eq!(
            reg.read(),
            0xFFFF_FFAF,
            "mọi bit ngoài trường phải nguyên vẹn"
        );
    }

    #[test]
    fn values_are_truncated_to_the_field_width() {
        let reg = FakeRegisters::new(0);
        reg.write_field(0, 2, 0b1111); // chỉ 2 bit chứa được
        assert_eq!(
            reg.read(),
            0b11,
            "phần thừa bị mặt nạ chặn, không tràn sang bit 2"
        );
    }

    #[test]
    fn read_modify_write_issues_one_store() {
        let reg = FakeRegisters::new(0);
        reg.set_bit(3);
        assert_eq!(reg.write_count.get(), 1);
        assert_eq!(reg.read_count.get(), 1);
    }

    // ---------- Typestate GPIO ----------
    #[test]
    fn mode_switch_writes_correct_moder_bits() {
        let moder = FakeRegisters::new(0);
        let c = unsafe { Pin::new(5) };
        let _out = c.into_output(&moder);
        assert_eq!(
            moder.read_field(10, 2),
            0b01,
            "chân 5 → bit 10-11 = 01 (output)"
        );
    }

    #[test]
    fn output_toggles_the_right_pin() {
        let moder = FakeRegisters::new(0);
        let odr = FakeRegisters::new(0);
        let mut c = unsafe { Pin::new(3) }.into_output(&moder);
        c.set_high(&odr);
        assert_eq!(odr.read(), 0b1000);
        c.toggle(&odr);
        assert_eq!(odr.read(), 0);
    }

    #[test]
    fn pin_lifecycle_moves_through_modes() {
        let moder = FakeRegisters::new(0);
        let c = unsafe { Pin::new(2) };
        let out = c.into_output(&moder);
        let input_pin = out.into_input(&moder); // tiêu thụ chân đầu ra
        let analog = input_pin.into_analog(&moder); // rồi thành analog
        assert_eq!(
            analog.number(),
            2,
            "số hiệu chân theo suốt mọi lần đổi kiểu"
        );
        assert_eq!(moder.read_field(4, 2), 0b11);
    }

    #[test]
    fn singleton_hands_out_peripheral_once() {
        Peripherals::reset_for_test();
        assert!(Peripherals::take().is_some(), "lần đầu phải thành công");
        assert!(Peripherals::take().is_none(), "lần hai phải bị từ chối");
        assert!(Peripherals::take().is_none());
        Peripherals::reset_for_test();
    }

    // ---------- Q16.16 ----------
    #[test]
    fn q16_add_sub_is_exact() {
        let a = Q16::from_int(7);
        let b = Q16::from_int(3);
        assert_eq!(a + b, Q16::from_int(10));
        assert_eq!(a - b, Q16::from_int(4));
    }

    #[test]
    fn q16_mul_div_error_below_one_lsb() {
        let a = Q16::from_real(3.5);
        let b = Q16::from_real(2.25);
        assert!(((a * b).into_real() - 7.875).abs() < 1.0 / 65536.0);
        assert!(((a / b).into_real() - 3.5 / 2.25).abs() < 1.0 / 65536.0);
    }

    #[test]
    fn q16_multiply_by_one_is_identity() {
        for x in [0.0, 1.5, -3.25, 100.125] {
            let q = Q16::from_real(x);
            assert_eq!(q * Q16::ONE, q, "nhân với 1 phải trả lại chính nó");
        }
    }

    #[test]
    fn adc_to_temp_is_exact_at_both_ends() {
        assert!((adc_to_celsius(0).into_real() - (-40.0)).abs() < 0.01);
        assert!((adc_to_celsius(4095).into_real() - 125.0).abs() < 0.05);
        // và đơn điệu tăng
        let mut prev = adc_to_celsius(0);
        for adc in (100..4096).step_by(100) {
            let now = adc_to_celsius(adc as u16);
            assert!(now > prev, "nhiệt độ phải tăng đơn điệu theo ADC");
            prev = now;
        }
    }

    // ---------- Bộ đệm vòng ----------
    #[test]
    fn ring_buffer_is_fifo() {
        let mut d: RingBuffer<4> = RingBuffer::new();
        for b in [1u8, 2, 3] {
            d.push(b).unwrap();
        }
        assert_eq!(d.take(), Some(1));
        assert_eq!(d.take(), Some(2));
        assert_eq!(d.len(), 1);
    }

    #[test]
    fn ring_buffer_errors_instead_of_allocating() {
        let mut d: RingBuffer<2> = RingBuffer::new();
        d.push(1).unwrap();
        d.push(2).unwrap();
        assert_eq!(
            d.push(3),
            Err(3),
            "đầy thì TRẢ LẠI byte, không được lớn thêm"
        );
        assert_eq!(d.capacity(), 2, "sức chứa cố định lúc biên dịch");
    }

    #[test]
    fn ring_buffer_wraps_correctly() {
        let mut d: RingBuffer<3> = RingBuffer::new();
        for i in 0..30u8 {
            d.push(i).unwrap();
            assert_eq!(
                d.take(),
                Some(i),
                "chỉ số phải quay vòng đúng qua biên mảng"
            );
        }
        assert!(d.is_empty());
    }

    #[test]
    fn overwrite_mode_drops_oldest() {
        let mut d: RingBuffer<3> = RingBuffer::new();
        for b in [1u8, 2, 3] {
            d.push(b).unwrap();
        }
        assert_eq!(d.push_overwrite(4), Some(1), "phần tử CŨ NHẤT bị hy sinh");
        let rest: Vec<u8> = std::iter::from_fn(|| d.take()).collect();
        assert_eq!(rest, vec![2, 3, 4]);
    }

    #[test]
    fn empty_ring_returns_none() {
        let mut d: RingBuffer<4> = RingBuffer::new();
        assert_eq!(d.take(), None);
        assert!(d.is_empty() && !d.is_full());
    }

    // ---------- Chống rung ----------
    #[test]
    fn debounce_ignores_short_noise() {
        let mut c = Debouncer::new(3);
        // nhiễu: bật-tắt liên tục, không mẫu nào đủ 3 lần liên tiếp
        for m in [true, false, true, false, true, false] {
            assert_eq!(c.update(m), None, "nhiễu không được sinh sự kiện");
        }
        assert!(!c.state());
    }

    #[test]
    fn debounce_accepts_stable_signal() {
        let mut c = Debouncer::new(3);
        assert_eq!(c.update(true), None);
        assert_eq!(c.update(true), None);
        assert_eq!(c.update(true), Some(true), "đủ 3 mẫu → chuyển trạng thái");
        assert_eq!(
            c.update(true),
            None,
            "giữ nguyên thì không phát lại sự kiện"
        );
    }

    #[test]
    fn debounce_emits_one_event_per_press() {
        let mut c = Debouncer::new(2);
        let samples = [false, true, false, true, true, true, true, true];
        let event_count = samples.iter().filter(|&&m| c.update(m).is_some()).count();
        assert_eq!(event_count, 1, "một cú bấm nảy = đúng một sự kiện");
    }
}
```

---

## Từ mô phỏng tới phần cứng thật

Đây là cách đoạn mã trên biến thành chương trình chạy trên vi điều khiển thật. Bản tối giản dưới đây ghi thẳng thanh ghi bằng `read_volatile`/`write_volatile` — đúng những gì `FakeRegisters` mô phỏng; dự án thật sẽ dùng crate HAL để có typestate như mục 2:

```rust
#![no_std] // không có thư viện chuẩn
#![no_main] // không có hàm main() do hệ điều hành gọi

// Cargo.toml: cortex-m = "0.7", cortex-m-rt = "0.7", panic-halt = "1"
// Mã minh hoạ cho STM32F4 (LED ở PA5) — cần dự án nhúng riêng, target thumbv7em-none-eabihf.
use core::ptr::{read_volatile, write_volatile};
use cortex_m_rt::entry;
use panic_halt as _; // panic = dừng CPU (bản phát hành dùng panic-reset)

const RCC_AHB1ENR: *mut u32 = 0x4002_3830 as *mut u32; // bật clock cho cổng GPIO
const GPIOA_MODER: *mut u32 = 0x4002_0000 as *mut u32;
const GPIOA_ODR: *mut u32 = 0x4002_0014 as *mut u32;

#[entry]
fn start() -> ! {
    // trả về `!` — hàm này KHÔNG BAO GIỜ kết thúc
    unsafe {
        write_volatile(RCC_AHB1ENR, read_volatile(RCC_AHB1ENR) | 1); // GPIOAEN
        let moder = read_volatile(GPIOA_MODER);
        write_volatile(GPIOA_MODER, (moder & !(0b11 << 10)) | (0b01 << 10)); // PA5 = output
    }
    loop {
        // đọc-sửa-ghi: đảo bit 5 của ODR
        unsafe { write_volatile(GPIOA_ODR, read_volatile(GPIOA_ODR) ^ (1 << 5)) };
        cortex_m::asm::delay(8_000_000); // chờ ~0,5 giây ở 16 MHz
    }
}
```

Ba điểm khác biệt đáng chú ý:

1. **`fn start() -> !`** — kiểu trả về `!` (never type) nói rằng hàm này không bao giờ trả về. Đúng vậy: không có hệ điều hành nào để trả về *cho*.
2. **`panic_halt as _`** — phải khai báo *hành vi khi panic*, vì không có `std` để in thông báo. Bản phát hành thường dùng `panic-reset` (khởi động lại chip) hoặc ghi vào bộ nhớ không mất điện để gỡ lỗi sau.
3. **Không có `println!`** — gỡ lỗi qua semihosting (`hprintln!`, chậm), qua UART, hoặc qua RTT (Real-Time Transfer, nhanh).

Bộ công cụ: `cargo install probe-rs-tools`, rồi `cargo embed` để nạp chương trình và xem log. Muốn thử mà chưa có phần cứng? `qemu-system-arm` mô phỏng được bo mạch STM32 ngay trên máy tính.

---

## Bảng tra cứu lỗi biên dịch thường gặp

| Lỗi | Nguyên nhân trong chương này | Cách sửa |
|---|---|---|
| `E0599`: no method named `set_high` found for struct `Pin<Input>` | **Đây là tính năng!** Bạn đang cố ghi vào chân cấu hình làm đầu vào | Gọi `.into_output(&moder)` trước |
| `E0382`: borrow of moved value: `pin` | Dùng lại chân sau khi đã đổi chế độ (`pin.into_output(..)` rồi `pin.number()`) | Đúng như thiết kế — dùng giá trị **trả về** của `into_output` |
| `error: creating a shared reference to mutable static` (lint `static_mut_refs`; là lỗi mặc định từ Edition 2024, cảnh báo ở 2021) | Lấy tham chiếu tới `static mut` (`&TAKEN`) | Dùng `AtomicBool`/`AtomicUsize`, hoặc `critical_section::Mutex<RefCell<T>>` |
| `E0015`: cannot call non-const associated function `Q16::from_real` in constants | Tính hằng số Q16 lúc biên dịch (`const SCALE: Q16 = Q16::from_real(..)`) mà hàm không phải `const fn` | Khai báo `pub const fn from_real` — phép toán `f64` (kể cả `round`) dùng được trong `const fn` ở Rust hiện nay |
| `` error: `#[panic_handler]` function required, but not found `` | Thêm `#![no_std]` mà quên khai báo trình xử lý panic (trên máy tính còn kèm `unwinding panics are not supported without std` nếu không đặt `panic = "abort"`) | `use panic_halt as _;` (hoặc tự viết `#[panic_handler]`) |
| `` error: using `fn main` requires the standard library `` | Có `#![no_std]` nhưng quên `#![no_main]` | Thêm `#![no_main]` và đánh dấu hàm khởi động bằng `#[entry]`; hàm entry phải trả về `!` |

---

## Tóm tắt chương & Bài tập rèn luyện (Summary & Exercises)

### 5 điểm cốt lõi cần ghi nhớ

1. **`no_std` cắt thư viện, không cắt ngôn ngữ.** Quyền sở hữu, trait, iterator, `Result` — tất cả vẫn còn nguyên. Đó là lý do Rust hợp với nhúng đến vậy.
2. **`volatile` không phải tùy chọn.** Thiếu nó, trình tối ưu hóa sẽ xóa mất chính những lệnh điều khiển phần cứng của bạn.
3. **Typestate biến lỗi phần cứng thành lỗi biên dịch, với chi phí lúc chạy bằng không.** Đây là ứng dụng thực tế nhất của Chương 20 trong cả giáo trình.
4. **Không heap là một lựa chọn thiết kế, không phải hạn chế.** Bộ nhớ tĩnh cho thời gian thực thi đoán trước được — điều bắt buộc với hệ thống thời gian thực.
5. **Số dấu phẩy tĩnh (Fixed-point arithmetic) là bạn của vi điều khiển.** Q16.16 cho sai số dưới 0,00002 mà chỉ dùng phép toán số nguyên.

### Bài tập rèn luyện tự giải

**Bài 1.** Cài **bộ lọc trung bình trượt** cho dữ liệu cảm biến, dùng `RingBuffer` và số Q16.16, **không cấp phát**.

<details>
<summary><b>Gợi ý</b></summary>

Giữ một mảng `N` mẫu **và** một biến `sum` chạy. Khi ghi mẫu mới đè lên mẫu cũ nhất, trừ mẫu bị đuổi ra khỏi `sum` rồi cộng mẫu mới vào. Nhờ vậy tính trung bình là O(1) thay vì O(N).

Cẩn thận với tràn số: `sum` phải đủ rộng để chứa `N` mẫu Q16.16 cộng lại.
</details>

<details>
<summary><b>Lời giải</b></summary>

```rust
pub struct MovingAverage<const N: usize> {
    samples: [Q16; N],
    next: usize,  // ô sẽ bị ghi đè tiếp theo
    filled: usize, // số mẫu đã có (≤ N)
    sum: i64,     // i64 để chắc chắn không tràn khi cộng N mẫu i32
}

impl<const N: usize> MovingAverage<N> {
    pub const fn new() -> Self {
        MovingAverage {
            samples: [Q16(0); N],
            next: 0,
            filled: 0,
            sum: 0,
        }
    }
    /// O(1): trừ mẫu cũ, cộng mẫu mới — không duyệt lại cả mảng.
    pub fn add_sample(&mut self, x: Q16) -> Q16 {
        self.sum -= self.samples[self.next].0 as i64; // bỏ mẫu bị ghi đè
        self.samples[self.next] = x;
        self.sum += x.0 as i64;
        self.next = (self.next + 1) % N;
        if self.filled < N {
            self.filled += 1;
        }
        Q16((self.sum / self.filled as i64) as i32)
    }
}

#[test]
fn moving_average_warms_up_correctly() {
    let mut avg: MovingAverage<4> = MovingAverage::new();
    assert_eq!(avg.add_sample(Q16::from_int(10)), Q16::from_int(10)); // chưa đầy: chia cho 1
    assert_eq!(avg.add_sample(Q16::from_int(20)), Q16::from_int(15));
    for _ in 0..4 {
        avg.add_sample(Q16::from_int(2));
    }
    assert_eq!(avg.add_sample(Q16::from_int(6)), Q16::from_int(3)); // (2+2+2+6)/4
}
```

Chú ý `self.filled` thay vì `N` ở mẫu số: trong `N` lần gọi đầu tiên bộ đệm chưa đầy, chia cho `N` sẽ cho kết quả nhỏ hơn thực tế — một lỗi khởi động kinh điển khiến cảm biến báo sai trong vài giây đầu.
</details>

**Bài 2.** Mở rộng typestate để phân biệt chân đầu vào **kéo lên** (pull-up), **kéo xuống** (pull-down) và **thả nổi** (floating), sao cho việc đọc một chân thả nổi phải sinh cảnh báo.

<details>
<summary><b>Gợi ý</b></summary>

Dùng typestate **hai tầng**: `Pin<InputWith<PullUp>>`. Cài `is_high()` cho `Pin<InputWith<PullUp>>` và `Pin<InputWith<PullDown>>`, nhưng đặt tên phương thức của `Pin<InputWith<Floating>>` là `read_unchecked()` — người đọc mã sẽ tự thấy vấn đề. (Rust không có cơ chế "cảnh báo khi gọi hàm" cho mã của bạn ngoài `#[deprecated]`; một cái tên tự tố cáo là cách thực tế.)

Vì sao chân thả nổi nguy hiểm? Nó không nối với nguồn cũng không nối với đất, nên điện áp trôi theo nhiễu môi trường. Đọc nó cho kết quả ngẫu nhiên — và tệ hơn, kết quả *có vẻ ổn định* trong phòng thí nghiệm rồi hỏng ngoài thực địa.
</details>

<details>
<summary><b>Lời giải</b></summary>

```rust
pub struct PullUp;
pub struct PullDown;
pub struct Floating;

pub struct InputWith<Pull>(PhantomData<Pull>);

impl<Pull> Pin<InputWith<Pull>> {
    fn read_raw(&self, idr: &FakeRegisters) -> bool {
        idr.test_bit(self.number())
    }
}

// Chỉ chân có điện trở kéo mới có `is_high()` — trạng thái nghỉ xác định.
impl Pin<InputWith<PullUp>> {
    /// Nút chưa bấm = mức CAO (bị điện trở kéo lên). Bấm = nối đất = THẤP.
    pub fn is_high(&self, idr: &FakeRegisters) -> bool {
        self.read_raw(idr)
    }
}
impl Pin<InputWith<PullDown>> {
    pub fn is_high(&self, idr: &FakeRegisters) -> bool {
        self.read_raw(idr)
    }
}

impl Pin<InputWith<Floating>> {
    /// Tên dài và xấu là CỐ Ý: chân thả nổi không có mức nghỉ xác định.
    /// Chỉ dùng khi mạch ngoài đã tự có điện trở kéo.
    pub fn read_unchecked(&self, idr: &FakeRegisters) -> bool {
        self.read_raw(idr)
    }
}

// Chuyển chế độ — PUPDR (thanh ghi kéo lên/xuống) dùng 2 bit mỗi chân: 00 thả nổi, 01 kéo lên, 10 kéo xuống.
impl<Mode> Pin<Mode> {
    /// Tiêu thụ chân cũ (`self`), đặt MODER = 00 (vào) và PUPDR = 01 (kéo lên).
    pub fn into_pull_up_input(
        self,
        moder: &FakeRegisters,
        pupdr: &FakeRegisters,
    ) -> Pin<InputWith<PullUp>> {
        moder.write_field(self.number * 2, 2, 0b00);
        pupdr.write_field(self.number * 2, 2, 0b01);
        Pin {
            number: self.number,
            _mode: PhantomData,
        }
    }
}

#[test]
fn pull_up_pin_reads_high_at_rest() {
    let (moder, pupdr, idr) = (FakeRegisters::new(0), FakeRegisters::new(0), FakeRegisters::new(1 << 13));
    let button = unsafe { Pin::new(13) }.into_pull_up_input(&moder, &pupdr);
    assert!(button.is_high(&idr));
    assert_eq!(pupdr.read_field(26, 2), 0b01);
}
```

Đây là kỹ thuật thiết kế API quan trọng: **làm cho việc đúng dễ làm, việc nguy hiểm khó gõ**. Không cấm hẳn (đôi khi thả nổi là đúng), nhưng buộc người viết phải gõ ra một cái tên tự tố cáo.
</details>

**Bài 3.** Cài **hàng đợi một-nhà-sản-xuất-một-người-tiêu-thụ** (SPSC) an toàn giữa ngắt và vòng lặp chính, không dùng khóa.

<details>
<summary><b>Gợi ý</b></summary>

Đây là bài toán kinh điển: ngắt UART đẩy byte vào, vòng lặp chính lấy ra. Vì chỉ có **một** bên ghi `tail` và **một** bên ghi `head`, ta không cần khóa — chỉ cần hai `AtomicUsize` với thứ tự bộ nhớ đúng.

Người sản xuất: đọc `head` (Acquire), ghi dữ liệu, rồi ghi `tail` (Release).
Người tiêu thụ: đọc `tail` (Acquire), đọc dữ liệu, rồi ghi `head` (Release).

Cặp Release/Acquire bảo đảm: khi bên kia *thấy* con trỏ mới, nó cũng thấy dữ liệu đã ghi xong.
</details>

<details>
<summary><b>Lời giải</b></summary>

```rust
use core::cell::UnsafeCell;
use core::sync::atomic::AtomicUsize;   // `Ordering` chương đã nhập ở trên

pub struct SpscQueue<const N: usize> {
    o: UnsafeCell<[u8; N]>,
    head: AtomicUsize,   // CHỈ người tiêu thụ ghi — vị trí ĐỌC
    tail: AtomicUsize,  // CHỈ người sản xuất ghi — vị trí GHI
}

// An toàn: mỗi con trỏ chỉ có ĐÚNG MỘT bên ghi, nên không có cuộc đua ghi-ghi.
unsafe impl<const N: usize> Sync for SpscQueue<N> {}

impl<const N: usize> SpscQueue<N> {
    pub const fn new() -> Self {
        SpscQueue {
            o: UnsafeCell::new([0; N]),
            head: AtomicUsize::new(0),
            tail: AtomicUsize::new(0),
        }
    }

    /// Gọi TỪ NGẮT. Trả `Err` nếu đầy — không bao giờ chặn.
    pub fn push(&self, b: u8) -> Result<(), u8> {
        let tail = self.tail.load(Ordering::Relaxed);   // ta là bên duy nhất ghi nó
        let tail_next = (tail + 1) % N;
        if tail_next == self.head.load(Ordering::Acquire) {
            return Err(b); // đầy — hy sinh byte còn hơn chặn ngắt
        }
        unsafe { (*self.o.get())[tail] = b; }
        // Release: bảo đảm lệnh ghi dữ liệu ở trên HOÀN TẤT trước khi
        // người tiêu thụ nhìn thấy con trỏ mới.
        self.tail.store(tail_next, Ordering::Release);
        Ok(())
    }

    /// Gọi từ VÒNG LẶP CHÍNH.
    pub fn take(&self) -> Option<u8> {
        let head = self.head.load(Ordering::Relaxed);
        if head == self.tail.load(Ordering::Acquire) {
            return None; // rỗng
        }
        let b = unsafe { (*self.o.get())[head] };
        self.head.store((head + 1) % N, Ordering::Release);
        Some(b)
    }
}

#[test]
fn spsc_holds_n_minus_one() {
    let q: SpscQueue<4> = SpscQueue::new();
    for b in [1, 2, 3] {
        q.push(b).unwrap();
    }
    assert_eq!(q.push(4), Err(4)); // 4 ô nhưng chỉ chứa được 3
    assert_eq!(q.take(), Some(1));
    q.push(4).unwrap(); // con trỏ quay vòng qua biên mảng
    assert_eq!([q.take(), q.take(), q.take(), q.take()], [Some(2), Some(3), Some(4), None]);
}
```

Điểm tinh tế nhất là **hy sinh một ô nhớ**: hàng đợi `N` ô chỉ chứa được `N-1` phần tử, vì `head == tail` phải chỉ nghĩa "rỗng". Nếu cho phép chứa đủ `N`, trạng thái đầy và rỗng trông giống hệt nhau và không cách nào phân biệt mà không thêm biến đếm — mà thêm biến đếm thì lại cần cả hai bên cùng ghi, phá vỡ tính không-khóa.
</details>
