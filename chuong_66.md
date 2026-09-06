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
│                                    │                                          │
│  RAM: 16 GB (16 000 000 000 byte)  │  RAM: 32 KB (32 000 byte)               │
│                                    │  → ít hơn 500 000 LẦN                   │
│                                    │                                          │
│  Chạy 8 tiếng rồi tắt máy          │  Chạy 10 NĂM không tắt                  │
│  → rò rỉ nhỏ không sao             │  → rò rỉ 1 byte/giờ = chết sau 4 năm    │
│                                    │                                          │
├──────────────────────────────────────────────────────────────────────────────┤
│    THANH GHI ÁNH XẠ BỘ NHỚ = CÔNG TẮC ĐIỆN TRÔNG NHƯ Ô GHI CHÚ               │
│                                                                              │
│    Địa chỉ 0x4002_0014 trông y hệt một biến bình thường.                    │
│    Nhưng GHI vào nó = BẬT MỘT BÓNG ĐÈN THẬT trên bảng mạch.                 │
│                                                                              │
│    ⚠ NGUY HIỂM: trình tối ưu hóa thấy bạn "ghi rồi không đọc lại"           │
│      → nó XÓA LỆNH GHI đi cho nhanh → đèn không bao giờ sáng.                │
│      Từ khóa `volatile` nghĩa là: "ĐỪNG THÔNG MINH. GHI THẬT ĐI."           │
│                                                                              │
├──────────────────────────────────────────────────────────────────────────────┤
│    TYPESTATE CHO CHÂN GPIO = Ổ CẮM CÓ HÌNH DẠNG KHÁC NHAU                    │
│                                                                              │
│    Phích cắm 2 chân KHÔNG cắm vừa ổ 3 chân — không phải nhờ cảnh báo,       │
│    mà nhờ HÌNH DẠNG VẬT LÝ. Bạn không thể cắm sai kể cả khi cố tình.        │
│                                                                              │
│    Chan<Output> có .bat() và .tat()      ← điều khiển đèn                    │
│    Chan<Input> có .doc()               ← đọc nút bấm                       │
│                                                                              │
│    nut.bat()  →  ❌ E0599: không có phương thức `bat`                        │
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
*(0x4002_0014 as *mut u32) = 1;   // bật đèn
*(0x4002_0014 as *mut u32) = 0;   // tắt đèn
```

Trình tối ưu hóa lý luận: "ghi 1 rồi ghi 0 vào cùng chỗ mà không đọc ở giữa — lệnh đầu vô nghĩa, xóa đi." Kết quả: đèn không bao giờ nhấp nháy. Tệ hơn, nếu ta ghi trong vòng lặp mà không đọc, cả vòng lặp có thể bị xóa sạch.

`read_volatile`/`write_volatile` nói với trình biên dịch: **mỗi** thao tác đều có tác dụng phụ ngoài tầm hiểu biết của mày, đừng gộp, đừng xóa, đừng đảo thứ tự. Trong chương này, `IntoRecordPrice` đếm số lần đọc/ghi để bạn *thấy* được điều đó trong bài kiểm thử.

### 3. Đọc-Sửa-Ghi và cái bẫy ngắt

Muốn bật bit 3 mà không đụng các bit khác, phải làm ba bước: **đọc** giá trị hiện tại, **sửa** bit, **ghi** lại. Nhưng nếu một ngắt xảy ra *giữa* bước đọc và bước ghi, và ngắt đó cũng sửa cùng thanh ghi, thay đổi của nó sẽ bị ghi đè mất.

Ba cách xử lý:
1. **Vùng găng** (critical section): tắt ngắt trong lúc đọc-sửa-ghi. Đơn giản nhưng làm tăng độ trễ ngắt.
2. **Thanh ghi bit-band**: nhiều vi điều khiển ARM cung cấp vùng địa chỉ mà mỗi bit có địa chỉ riêng — ghi một bit thành một lệnh duy nhất, không thể bị cắt ngang.
3. **Thanh ghi set/clear riêng**: STM32 có `BSRR` — ghi 1 vào bit `n` thì bật chân `n`, ghi 1 vào bit `n+16` thì tắt chân `n`. Không cần đọc trước.

### 4. Typestate: chuyển trạng thái phải **tiêu thụ** giá trị cũ

Điểm mấu chốt của typestate là chữ ký hàm:

```rust
pub fn into_output(self, tg: &IntoRecordPrice) -> Block<Output>
//                  ^^^^ nhận `self` theo GIÁ TRỊ, không phải `&self`
```

Vì nhận `self`, chân cũ bị **di chuyển** và không dùng lại được. Nhờ vậy không bao giờ tồn tại đồng thời hai cách nhìn về cùng một chân phần cứng. Nếu dùng `&self`, bạn có thể tạo `Chan<Output>` mà vẫn giữ `Chan<Input>` cũ — và trình biên dịch sẽ vui vẻ cho phép bạn vừa đọc vừa ghi cùng một chân.

Chi phí lúc chạy: **bằng không**. `PhantomData<Output>` không chiếm byte nào; `size_of::<Chan<Output>>() == size_of::<u8>()`. Toàn bộ kiểm tra biến mất sau khi biên dịch.

### 5. Số dấu phẩy tĩnh Q16.16

Cortex-M0/M0+ không có bộ xử lý dấu phẩy động. Mọi phép `f32` bị mô phỏng bằng phần mềm — chậm hơn hàng chục lần và tốn hàng KB flash.

Giải pháp: đặt dấu phẩy ở một vị trí **cố định** trong số nguyên. Q16.16 dùng 16 bit cho phần nguyên, 16 bit cho phần thập phân, tất cả trong một `i32`:

```
   Giá trị thực 3.5  →  3.5 × 65536  =  229376  =  0x0003_8000
                          ▲                          ▲▲▲▲ ▲▲▲▲
                          hằng số 2^16               nguyên│thập phân
```

- **Cộng/trừ**: cộng trừ số nguyên bình thường. Chính xác tuyệt đối.
- **Nhân**: phải qua `i64` rồi dịch phải 16, nếu không tràn ngay với số lớn hơn 1.
- **Chia**: dịch trái 16 **trước** khi chia, nếu không mất hết phần thập phân.

Sai số tối đa là `1/65536 ≈ 0.0000153` — thừa đủ cho cảm biến nhiệt độ, điều khiển động cơ, hay bộ lọc âm thanh.

### 6. `AtomicBool` thay cho `static mut`

Mẫu Singleton cần một cờ toàn cục "đã giao ngoại vi chưa". Viết bằng `static mut` là sai:

```rust
// ❌ SAI — có cửa sổ đua
if !DA_LAY { DA_LAY = true; giao_ngoai_vi() }
//         ▲ một ngắt chen vào ĐÂY sẽ khiến ngoại vi bị deliver HAI lần
```

`AtomicBool::swap` làm cả hai việc trong **một** thao tác không thể bị cắt ngang: đặt giá trị mới *và* trả về giá trị cũ. Nếu giá trị cũ là `true`, ta biết chắc có người lấy trước — không có khe hở nào.

---

## Mã nguồn minh họa thực chiến (Idiomatic Runnable Rust Blueprint)

Mã dưới đây chạy được trên máy tính để bàn (để kiểm thử được). Trên vi điều khiển thật, bạn thêm `#![no_std]` + `#![no_main]`, thay `IntoRecordPrice` bằng `read_volatile`/`write_volatile` trên địa chỉ thật, và dùng crate HAL của dòng chip (`stm32f4xx-hal`, `rp2040-hal`, `esp-hal`…).

Chạy bằng `cargo run -p ch66`, kiểm thử bằng `cargo test -p ch66`.

```rust
#![allow(dead_code)]
//! Chương 66 — Lập trình nhúng & `no_std`: thanh ghi ánh xạ bộ nhớ, mẫu Singleton
//! cho ngoại vi, typestate cho chân GPIO, số dấu phẩy tĩnh, và bộ đệm vòng không cấp phát.
//!
//! Ghi chú: tệp này chạy trên máy tính để bàn để KIỂM THỬ ĐƯỢC. Trên vi điều khiển
//! thật, bạn thêm `#![no_std]` + `#![no_main]` và thay `IntoRecordPrice` bằng địa chỉ thật.

use core::marker::PhantomData;
use std::cell::Cell;
use std::sync::atomic::{AtomicBool, Ordering};

// ============================================================================
// 1. THANH GHI ÁNH XẠ BỘ NHỚ (MMIO) — phần cứng trông như biến
// ============================================================================

/// Trên vi điều khiển, ghi vào địa chỉ 0x4002_0014 sẽ BẬT một chân đèn.
/// Không có `volatile`, trình tối ưu hóa có quyền xóa lệnh ghi đó — vì theo
/// nó, ghi vào bộ nhớ rồi không đọc lại là việc vô nghĩa.
pub struct IntoRecordPrice {
    small_cell: Cell<u32>,
    pub count_record: Cell<u32>,
    pub so_lan_doc: Cell<u32>,
}

impl IntoRecordPrice {
    pub fn new(value: u32) -> Self {
        IntoRecordPrice { small_cell: Cell::new(value), count_record: Cell::new(0), so_lan_doc: Cell::new(0) }
    }
    /// Tương ứng `core::ptr::write_volatile` — MỖI lệnh ghi đều phải xảy ra thật.
    pub fn record(&self, v: u32) { self.small_cell.set(v); self.count_record.set(self.count_record.get() + 1); }
    /// Tương ứng `core::ptr::read_volatile` — không được lưu vào thanh ghi CPU dùng lại.
    pub fn doc(&self) -> u32 { self.so_lan_doc.set(self.so_lan_doc.get() + 1); self.small_cell.get() }

    /// Đọc-Sửa-Ghi: mẫu thao tác bit chuẩn của lập trình nhúng.
    pub fn set_bit(&self, bit: u8) { self.record(self.doc() | (1 << bit)); }
    pub fn clear_bit(&self, bit: u8) { self.record(self.doc() & !(1 << bit)); }
    pub fn dao_bit(&self, bit: u8) { self.record(self.doc() ^ (1 << bit)); }
    pub fn test_bit(&self, bit: u8) -> bool { self.doc() & (1 << bit) != 0 }

    /// Ghi một trường nhiều bit mà KHÔNG đụng các bit khác.
    pub fn record_field(&self, lech: u8, rong: u8, value: u32) {
        let mat_na = ((1u32 << rong) - 1) << lech;
        self.record((self.doc() & !mat_na) | ((value << lech) & mat_na));
    }
    pub fn read_field(&self, lech: u8, rong: u8) -> u32 {
        (self.doc() >> lech) & ((1u32 << rong) - 1)
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
pub struct Wall;   // analog — cho ADC

pub struct Block<CheDo> {
    serial: u8,
    _che_do: PhantomData<CheDo>,
}

impl Block<Unconfigured> {
    /// `unsafe` vì tạo hai `Chan` cùng số hiệu sẽ phá vỡ độc quyền phần cứng.
    /// Trong thực tế bạn chỉ gọi nó qua Singleton ở mục 3.
    pub unsafe fn new(serial: u8) -> Self { Block { serial, _che_do: PhantomData } }
}

impl<CheDo> Block<CheDo> {
    pub fn serial(&self) -> u8 { self.serial }
    /// Chuyển chế độ TIÊU THỤ chân cũ (`self`) và trả về chân kiểu mới.
    /// Nhờ vậy không tồn tại đồng thời hai cách nhìn về cùng một chân.
    pub fn into_output(self, tg: &IntoRecordPrice) -> Block<Output> {
        tg.record_field(self.serial * 2, 2, 0b01); // MODER = 01 (output)
        Block { serial: self.serial, _che_do: PhantomData }
    }
    pub fn into_input(self, tg: &IntoRecordPrice) -> Block<Input> {
        tg.record_field(self.serial * 2, 2, 0b00); // MODER = 00 (input)
        Block { serial: self.serial, _che_do: PhantomData }
    }
    pub fn into_wall(self, tg: &IntoRecordPrice) -> Block<Wall> {
        tg.record_field(self.serial * 2, 2, 0b11); // MODER = 11 (analog)
        Block { serial: self.serial, _che_do: PhantomData }
    }
}

// CHỈ chân đầu ra mới có `bat`/`tat` — gọi trên chân đầu vào là lỗi biên dịch.
impl Block<Output> {
    pub fn bat(&mut self, data: &IntoRecordPrice) { data.set_bit(self.serial); }
    pub fn tat(&mut self, data: &IntoRecordPrice) { data.clear_bit(self.serial); }
    pub fn dao(&mut self, data: &IntoRecordPrice) { data.dao_bit(self.serial); }
}

// CHỈ chân đầu vào mới có `doc`.
impl Block<Input> {
    pub fn doc(&self, data: &IntoRecordPrice) -> bool { data.test_bit(self.serial) }
}

// ============================================================================
// 3. SINGLETON NGOẠI VI — "chỉ có MỘT bộ ngoại vi trên con chip này"
// ============================================================================

/// Gói TẤT CẢ ngoại vi của con chip. Ai cầm được nó là chủ duy nhất của phần cứng.
pub struct UnitOutPos {
    pub gate_a: Block<Unconfigured>,
    pub gate_b: Block<Unconfigured>,
}

/// Cờ nguyên tử thay cho `static mut`: an toàn cả khi có ngắt xen giữa.
/// `swap` là thao tác ĐỌC-VÀ-ĐẶT không thể bị cắt ngang — nếu dùng
/// `if !DA_LAY { DA_LAY = true }` thì một ngắt chen vào giữa hai câu lệnh
/// có thể khiến ngoại vi bị deliver HAI lần.
static DA_LAY: AtomicBool = AtomicBool::new(false);

impl UnitOutPos {
    /// Trả `Some` đúng MỘT lần trong suốt vòng đời chương trình.
    /// Lần thứ hai trả `None` — không thể có hai chủ sở hữu cùng điều khiển chip.
    pub fn lay() -> Option<UnitOutPos> {
        if DA_LAY.swap(true, Ordering::SeqCst) {
            return None; // đã có người lấy trước
        }
        // An toàn: cờ trên bảo đảm đoạn này chạy đúng một lần.
        Some(unsafe { UnitOutPos { gate_a: Block::new(5), gate_b: Block::new(13) } })
    }
    #[doc(hidden)]
    pub fn reset_for_test() { DA_LAY.store(false, Ordering::SeqCst); }
}

// ============================================================================
// 4. SỐ DẤU PHẨY TĨNH — vì phần lớn vi điều khiển KHÔNG có FPU
// ============================================================================

/// Q16.16: 16 bit phần nguyên, 16 bit phần thập phân, đựng trong một `i32`.
/// Nhân/chia bằng số nguyên → fast gấp hàng chục lần mô phỏng dấu phẩy động.
#[derive(Debug, Clone, Copy, PartialEq, Eq, PartialOrd, Ord)]
pub struct Q16(pub i32);

impl Q16 {
    pub const MOT: Q16 = Q16(1 << 16);
    pub fn tu_nguyen(n: i16) -> Q16 { Q16((n as i32) << 16) }
    /// Chỉ dùng khi biên dịch trên máy có dấu phẩy động (lúc thiết kế hằng số).
    pub fn from_real(x: f64) -> Q16 { Q16((x * 65536.0).round() as i32) }
    pub fn into_real(self) -> f64 { self.0 as f64 / 65536.0 }
    pub fn gate(self, k: Q16) -> Q16 { Q16(self.0.wrapping_add(k.0)) }
    pub fn subtract(self, k: Q16) -> Q16 { Q16(self.0.wrapping_sub(k.0)) }
    /// Nhân phải qua i64 rồi dịch phải 16 — nếu không sẽ tràn ngay.
    pub fn nhan(self, k: Q16) -> Q16 { Q16(((self.0 as i64 * k.0 as i64) >> 16) as i32) }
    pub fn chia(self, k: Q16) -> Q16 { Q16((((self.0 as i64) << 16) / k.0 as i64) as i32) }
}

/// Chuyển giá trị ADC 12-bit (0..4095) sang nhiệt độ °C, toàn số nguyên.
/// Cảm biến giả định: 0 → -40 °C, 4095 → 125 °C (tuyến tính).
pub fn adc_sang_nhiet_do(adc: u16) -> Q16 {
    let ti_le = Q16::from_real(165.0 / 4095.0);
    Q16::tu_nguyen(adc as i16).nhan(ti_le).subtract(Q16::tu_nguyen(40))
}

// ============================================================================
// 5. BỘ ĐỆM VÒNG KHÔNG CẤP PHÁT — `heapless` attempt nhỏ
// ============================================================================

/// Không `Vec`, không `Box`, không heap. Bộ nhớ nằm gọn trong struct,
/// kích thước biết trước lúc biên dịch. Đây là kiểu dữ liệu chủ lực của
/// ngắt UART: ISR đẩy byte vào, vòng lặp chính lấy ra.
pub struct CountRound<const N: usize> {
    o: [u8; N],
    /// Vị trí ĐỌC kế tiếp.
    head: usize,
    /// Vị trí GHI kế tiếp.
    tail: usize,
    quantity: usize,
}

impl<const N: usize> CountRound<N> {
    pub const fn new() -> Self { CountRound { o: [0; N], head: 0, tail: 0, quantity: 0 } }
    pub fn capacity(&self) -> usize { N }
    pub fn quantity(&self) -> usize { self.quantity }
    pub fn rong(&self) -> bool { self.quantity == 0 }
    pub fn day(&self) -> bool { self.quantity == N }

    /// Trả `Err` thay vì cấp phát thêm — hệ nhúng KHÔNG được phép "cứ lớn dần".
    pub fn push(&mut self, b: u8) -> Result<(), u8> {
        if self.day() { return Err(b); }
        self.o[self.tail] = b;
        self.tail = (self.tail + 1) % N;
        self.quantity += 1;
        Ok(())
    }
    pub fn take(&mut self) -> Option<u8> {
        if self.rong() { return None; }
        let b = self.o[self.head];
        self.head = (self.head + 1) % N;
        self.quantity -= 1;
        Some(b)
    }
    /// Ghi đè phần tử cũ nhất khi đầy — dùng cho nhật ký sự cố (black box).
    pub fn overwrite_buffer(&mut self, b: u8) -> Option<u8> {
        let is_mat = if self.day() { self.take() } else { None };
        let _ = self.push(b);
        is_mat
    }
}

// ============================================================================
// 6. MÁY TRẠNG THÁI KHÔNG CẤP PHÁT — bộ chống rung phím (debounce)
// ============================================================================

/// Nút bấm cơ khí "nảy" hàng chục lần trong vài mili-giây. Không lọc thì
/// một cú bấm thành 20 sự kiện. Bộ lọc: chỉ đổi trạng thái khi đọc được
/// `NGUONG` mẫu GIỐNG NHAU liên tiếp.
#[derive(Debug, Clone, Copy, PartialEq)]
pub struct ChongRung {
    is_stable: bool,
    count: u8,
    threshold: u8,
}

impl ChongRung {
    pub fn new(threshold: u8) -> Self { ChongRung { is_stable: false, count: 0, threshold } }
    /// Trả `Some(trạng thái mới)` chỉ tại đúng khoảnh khắc chuyển.
    pub fn update(&mut self, mau_tho: bool) -> Option<bool> {
        if mau_tho == self.is_stable {
            self.count = 0;
            return None;
        }
        self.count += 1;
        if self.count >= self.threshold {
            self.is_stable = mau_tho;
            self.count = 0;
            return Some(self.is_stable);
        }
        None
    }
    pub fn state(&self) -> bool { self.is_stable }
}

fn main() {
    println!("═══════════════════════════════════════════════════════════");
    println!("   LẬP TRÌNH NHÚNG: MMIO · TYPESTATE GPIO · Q16.16 · no_std ");
    println!("═══════════════════════════════════════════════════════════");

    println!("\n1. THANH GHI ÁNH XẠ BỘ NHỚ");
    let moder = IntoRecordPrice::new(0);
    let odr = IntoRecordPrice::new(0);
    moder.record_field(10, 2, 0b01);
    println!("   MODER sau khi đặt chân 5 thành output: 0b{:032b}", moder.doc());
    println!("   Số lệnh ghi thực sự chạm phần cứng   : {}", moder.count_record.get());

    println!("\n2. TYPESTATE GPIO — sai kiểu là không biên dịch được");
    let bo = UnitOutPos::lay().expect("lần đầu phải lấy được");
    println!("   BoNgoaiVi::lay() lần hai → {:?}", UnitOutPos::lay().is_none());
    let mut den = bo.gate_a.into_output(&moder);
    let nut = bo.gate_b.into_input(&moder);
    den.bat(&odr);
    println!("   Bật đèn chân {} → ODR = 0b{:016b}", den.serial(), odr.doc());
    println!("   Đọc nút chân {}  → {}", nut.serial(), nut.doc(&odr));
    println!("   ❌ nut.bat(&odr)   → E0599: không có phương thức `bat` cho Chan<DauVao>");

    println!("\n3. SỐ DẤU PHẨY TĨNH Q16.16 (không cần FPU)");
    for adc in [0u16, 1024, 2048, 4095] {
        let t = adc_sang_nhiet_do(adc);
        println!("   ADC {:>4} → {:>8.3} °C (bên trong chỉ là i32 = {})", adc, t.into_real(), t.0);
    }
    let a = Q16::from_real(3.5);
    let b = Q16::from_real(2.0);
    println!("   3.5 × 2.0 = {} · 3.5 ÷ 2.0 = {}", a.nhan(b).into_real(), a.chia(b).into_real());

    println!("\n4. BỘ ĐỆM VÒNG KHÔNG CẤP PHÁT (4 byte)");
    let mut count: CountRound<4> = CountRound::new();
    for b in b"RUST" { count.push(*b).unwrap(); }
    println!("   Đầy: {} | đẩy thêm 'X' → {:?}", count.day(), count.push(b'X').unwrap_err() as char);
    println!("   Ghi đè 'X' → mất byte {:?}", count.overwrite_buffer(b'X').map(|b| b as char));
    let con: Vec<char> = std::iter::from_fn(|| count.take()).map(|b| b as char).collect();
    println!("   Nội dung còn lại: {:?}", con);

    println!("\n5. CHỐNG RUNG PHÍM (ngưỡng 3 mẫu)");
    let mut cr = ChongRung::new(3);
    let mau = [false, true, false, true, true, true, true, false, true, false, false, false];
    let mut ket_qua = Vec::new();
    for (i, &m) in mau.iter().enumerate() {
        if let Some(new) = cr.update(m) { ket_qua.push((i, new)); }
    }
    println!("   12 mẫu nhiễu → chỉ {} sự kiện thật: {:?}", ket_qua.len(), ket_qua);

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
        let tg = IntoRecordPrice::new(0b1010_0000);
        tg.set_bit(0);
        assert_eq!(tg.doc(), 0b1010_0001, "đặt bit 0 phải giữ nguyên bit 5 và 7");
        tg.clear_bit(7);
        assert_eq!(tg.doc(), 0b0010_0001);
        tg.dao_bit(5);
        assert_eq!(tg.doc(), 0b0000_0001);
    }

    #[test]
    fn field_write_uses_exactly_its_width() {
        let tg = IntoRecordPrice::new(0xFFFF_FFFF);
        tg.record_field(4, 3, 0b010); // đặt 3 bit tại vị trí 4
        assert_eq!(tg.read_field(4, 3), 0b010);
        assert_eq!(tg.doc(), 0xFFFF_FFAF, "mọi bit ngoài trường phải nguyên vẹn");
    }

    #[test]
    fn values_are_truncated_to_the_field_width() {
        let tg = IntoRecordPrice::new(0);
        tg.record_field(0, 2, 0b1111); // chỉ 2 bit chứa được
        assert_eq!(tg.doc(), 0b11, "phần thừa bị mặt nạ chặn, không tràn sang bit 2");
    }

    #[test]
    fn read_modify_write_issues_one_store() {
        let tg = IntoRecordPrice::new(0);
        tg.set_bit(3);
        assert_eq!(tg.count_record.get(), 1);
        assert_eq!(tg.so_lan_doc.get(), 1);
    }

    // ---------- Typestate GPIO ----------
    #[test]
    fn mode_switch_writes_correct_moder_bits() {
        let moder = IntoRecordPrice::new(0);
        let c = unsafe { Block::new(5) };
        let _ra = c.into_output(&moder);
        assert_eq!(moder.read_field(10, 2), 0b01, "chân 5 → bit 10-11 = 01 (output)");
    }

    #[test]
    fn output_toggles_the_right_pin() {
        let moder = IntoRecordPrice::new(0);
        let odr = IntoRecordPrice::new(0);
        let mut c = unsafe { Block::new(3) }.into_output(&moder);
        c.bat(&odr);
        assert_eq!(odr.doc(), 0b1000);
        c.dao(&odr);
        assert_eq!(odr.doc(), 0);
    }

    #[test]
    fn pin_lifecycle_moves_through_modes() {
        let moder = IntoRecordPrice::new(0);
        let c = unsafe { Block::new(2) };
        let ra = c.into_output(&moder);
        let input_pin = ra.into_input(&moder);      // tiêu thụ chân đầu ra
        let tt = input_pin.into_wall(&moder);      // rồi thành analog
        assert_eq!(tt.serial(), 2, "số hiệu chân theo suốt mọi lần đổi kiểu");
        assert_eq!(moder.read_field(4, 2), 0b11);
    }

    #[test]
    fn singleton_hands_out_peripheral_once() {
        UnitOutPos::reset_for_test();
        assert!(UnitOutPos::lay().is_some(), "lần đầu phải thành công");
        assert!(UnitOutPos::lay().is_none(), "lần hai phải bị từ chối");
        assert!(UnitOutPos::lay().is_none());
        UnitOutPos::reset_for_test();
    }

    // ---------- Q16.16 ----------
    #[test]
    fn q16_add_sub_is_exact() {
        let a = Q16::tu_nguyen(7);
        let b = Q16::tu_nguyen(3);
        assert_eq!(a.gate(b), Q16::tu_nguyen(10));
        assert_eq!(a.subtract(b), Q16::tu_nguyen(4));
    }

    #[test]
    fn q16_mul_div_error_below_one_lsb() {
        let a = Q16::from_real(3.5);
        let b = Q16::from_real(2.25);
        assert!((a.nhan(b).into_real() - 7.875).abs() < 1.0 / 65536.0);
        assert!((a.chia(b).into_real() - 3.5 / 2.25).abs() < 1.0 / 65536.0);
    }

    #[test]
    fn q16_multiply_by_one_is_identity() {
        for x in [0.0, 1.5, -3.25, 100.125] {
            let q = Q16::from_real(x);
            assert_eq!(q.nhan(Q16::MOT), q, "nhân với 1 phải trả lại chính nó");
        }
    }

    #[test]
    fn adc_to_temp_is_exact_at_both_ends() {
        assert!((adc_sang_nhiet_do(0).into_real() - (-40.0)).abs() < 0.01);
        assert!((adc_sang_nhiet_do(4095).into_real() - 125.0).abs() < 0.05);
        // và đơn điệu tăng
        let mut prev = adc_sang_nhiet_do(0);
        for adc in (100..4096).step_by(100) {
            let nay = adc_sang_nhiet_do(adc as u16);
            assert!(nay > prev, "nhiệt độ phải tăng đơn điệu theo ADC");
            prev = nay;
        }
    }

    // ---------- Bộ đệm vòng ----------
    #[test]
    fn ring_buffer_is_fifo() {
        let mut d: CountRound<4> = CountRound::new();
        for b in [1u8, 2, 3] { d.push(b).unwrap(); }
        assert_eq!(d.take(), Some(1));
        assert_eq!(d.take(), Some(2));
        assert_eq!(d.quantity(), 1);
    }

    #[test]
    fn ring_buffer_errors_instead_of_allocating() {
        let mut d: CountRound<2> = CountRound::new();
        d.push(1).unwrap();
        d.push(2).unwrap();
        assert_eq!(d.push(3), Err(3), "đầy thì TRẢ LẠI byte, không được lớn thêm");
        assert_eq!(d.capacity(), 2, "sức chứa cố định lúc biên dịch");
    }

    #[test]
    fn ring_buffer_wraps_correctly() {
        let mut d: CountRound<3> = CountRound::new();
        for i in 0..30u8 {
            d.push(i).unwrap();
            assert_eq!(d.take(), Some(i), "chỉ số phải quay vòng đúng qua biên mảng");
        }
        assert!(d.rong());
    }

    #[test]
    fn overwrite_mode_drops_oldest() {
        let mut d: CountRound<3> = CountRound::new();
        for b in [1u8, 2, 3] { d.push(b).unwrap(); }
        assert_eq!(d.overwrite_buffer(4), Some(1), "phần tử CŨ NHẤT bị hy sinh");
        let con: Vec<u8> = std::iter::from_fn(|| d.take()).collect();
        assert_eq!(con, vec![2, 3, 4]);
    }

    #[test]
    fn empty_ring_returns_none() {
        let mut d: CountRound<4> = CountRound::new();
        assert_eq!(d.take(), None);
        assert!(d.rong() && !d.day());
    }

    // ---------- Chống rung ----------
    #[test]
    fn debounce_ignores_short_noise() {
        let mut c = ChongRung::new(3);
        // nhiễu: bật-tắt liên tục, không mẫu nào đủ 3 lần liên tiếp
        for m in [true, false, true, false, true, false] {
            assert_eq!(c.update(m), None, "nhiễu không được sinh sự kiện");
        }
        assert!(!c.state());
    }

    #[test]
    fn debounce_accepts_stable_signal() {
        let mut c = ChongRung::new(3);
        assert_eq!(c.update(true), None);
        assert_eq!(c.update(true), None);
        assert_eq!(c.update(true), Some(true), "đủ 3 mẫu → chuyển trạng thái");
        assert_eq!(c.update(true), None, "giữ nguyên thì không phát lại sự kiện");
    }

    #[test]
    fn debounce_emits_one_event_per_press() {
        let mut c = ChongRung::new(2);
        let mau = [false, true, false, true, true, true, true, true];
        let event_count = mau.iter().filter(|&&m| c.update(m).is_some()).count();
        assert_eq!(event_count, 1, "một cú bấm nảy = đúng một sự kiện");
    }
}
```

---

## Từ mô phỏng tới phần cứng thật

Đây là cách đoạn mã trên biến thành chương trình chạy trên vi điều khiển thật:

```rust
#![no_std]      // không có thư viện chuẩn
#![no_main]     // không có hàm main() do hệ điều hành gọi

use cortex_m_rt::entry;
use panic_halt as _;   // panic = dừng CPU (bản phát hành dùng panic-reset)

#[entry]
fn khoi_dong() -> ! {           // trả về `!` — hàm này KHÔNG BAO GIỜ kết thúc
    let bo = UnitOutPos::lay().unwrap();
    let moder = unsafe { &*(0x4002_0000 as *const ThanhGhiThat) };
    let mut den = bo.gate_a.into_output(moder);

    loop {
        den.bat(odr);
        cho_khoang(500_000);
        den.tat(odr);
        cho_khoang(500_000);
    }
}
```

Ba điểm khác biệt đáng chú ý:

1. **`fn khoi_dong() -> !`** — kiểu trả về `!` (never type) nói rằng hàm này không bao giờ trả về. Đúng vậy: không có hệ điều hành nào để trả về *cho*.
2. **`panic_halt as _`** — phải khai báo *hành vi khi panic*, vì không có `std` để in thông báo. Bản phát hành thường dùng `panic-reset` (khởi động lại chip) hoặc ghi vào bộ nhớ không mất điện để gỡ lỗi sau.
3. **Không có `println!`** — gỡ lỗi qua semihosting (`hprintln!`, chậm), qua UART, hoặc qua RTT (Real-Time Transfer, nhanh).

Bộ công cụ: `cargo install probe-rs-tools`, rồi `cargo embed` để nạp chương trình và xem log. Muốn thử mà chưa có phần cứng? `qemu-system-arm` mô phỏng được bo mạch STM32 ngay trên máy tính.

---

## Bảng tra cứu lỗi biên dịch thường gặp

| Lỗi | Nguyên nhân trong chương này | Cách sửa |
|---|---|---|
| `E0599: no method named 'bat' found for struct Chan<Input>` | **Đây là tính năng!** Bạn đang cố ghi vào chân cấu hình làm đầu vào | Gọi `.into_output(&moder)` trước |
| `E0382: use of moved value: 'chan'` | Dùng lại chân sau khi đã đổi chế độ | Đúng như thiết kế — dùng giá trị **trả về** của `into_output` |
| `static_mut_refs` (cảnh báo, sẽ thành lỗi ở Edition 2024) | Truy cập `static mut` | Dùng `AtomicBool`/`AtomicUsize`, hoặc `critical_section::Mutex<RefCell<T>>` |
| `E0658: use of unstable library feature` | Thử `impl Fn` hoặc `const fn` với tính năng chưa ổn định | Kiểm tra `rustup show`; nhiều tính năng nhúng cần bản nightly |
| `error: language item required, but not found: 'eh_personality'` | Thêm `#![no_std]` mà quên khai báo trình xử lý panic | `use panic_halt as _;` |
| `rust-lld: error: undefined symbol: main` | Quên `#![no_main]` hoặc `#[entry]` | Thêm cả hai; hàm entry phải trả về `!` |

---

## Tóm tắt chương & Bài tập rèn luyện (Summary & Exercises)

### 5 điểm cốt lõi cần ghi nhớ

1. **`no_std` cắt thư viện, không cắt ngôn ngữ.** Quyền sở hữu, trait, iterator, `Result` — tất cả vẫn còn nguyên. Đó là lý do Rust hợp với nhúng đến vậy.
2. **`volatile` không phải tùy chọn.** Thiếu nó, trình tối ưu hóa sẽ xóa mất chính những lệnh điều khiển phần cứng của bạn.
3. **Typestate biến lỗi phần cứng thành lỗi biên dịch, với chi phí lúc chạy bằng không.** Đây là ứng dụng thực tế nhất của Chương 20 trong cả giáo trình.
4. **Không heap là một lựa chọn thiết kế, không phải hạn chế.** Bộ nhớ tĩnh cho thời gian thực thi đoán trước được — điều bắt buộc với hệ thống thời gian thực.
5. **Số dấu phẩy tĩnh (Fixed-point arithmetic) là bạn của vi điều khiển.** Q16.16 cho sai số dưới 0,00002 mà chỉ dùng phép toán số nguyên.

### Bài tập rèn luyện tự giải

**Bài 1.** Cài **bộ lọc trung bình trượt** cho dữ liệu cảm biến, dùng `CountRound` và số Q16.16, **không cấp phát**.

<details>
<summary><b>Gợi ý</b></summary>

Giữ một `CountRound<N>` các mẫu **và** một biến `tong` chạy. Khi đẩy mẫu mới vào bộ đệm đầy, trừ mẫu bị đuổi ra khỏi `tong` rồi cộng mẫu mới vào. Nhờ vậy tính trung bình là O(1) thay vì O(N).

Cẩn thận với tràn số: `tong` phải đủ rộng để chứa `N` mẫu Q16.16 cộng lại.
</details>

<details>
<summary><b>Lời giải</b></summary>

```rust
pub struct MovingAverage<const N: usize> {
    mau: [Q16; N],
    chi_so: usize,
    samples: usize,
    tong: i64,      // i64 để chắc chắn không tràn khi cộng N mẫu i32
}

impl<const N: usize> MovingAverage<N> {
    pub const fn new() -> Self {
        MovingAverage { mau: [Q16(0); N], chi_so: 0, samples: 0, tong: 0 }
    }
    /// O(1): trừ mẫu cũ, cộng mẫu mới — không duyệt lại cả mảng.
    pub fn them(&mut self, x: Q16) -> Q16 {
        self.tong -= self.mau[self.chi_so].0 as i64;   // bỏ mẫu bị ghi đè
        self.mau[self.chi_so] = x;
        self.tong += x.0 as i64;
        self.chi_so = (self.chi_so + 1) % N;
        if self.samples < N { self.samples += 1; }
        Q16((self.tong / self.samples as i64) as i32)
    }
}
```

Chú ý `self.samples` thay vì `N` ở mẫu số: trong `N` lần gọi đầu tiên bộ đệm chưa đầy, chia cho `N` sẽ cho kết quả nhỏ hơn thực tế — một lỗi khởi động kinh điển khiến cảm biến báo sai trong vài giây đầu.
</details>

**Bài 2.** Mở rộng typestate để phân biệt chân đầu vào **kéo lên** (pull-up), **kéo xuống** (pull-down) và **thả nổi** (floating), sao cho việc đọc một chân thả nổi phải sinh cảnh báo.

<details>
<summary><b>Gợi ý</b></summary>

Dùng typestate **hai tầng**: `Chan<Input<KeoLen>>`. Cài `doc()` cho `Chan<Input<KeoLen>>` và `Chan<Input<KeoXuong>>`, nhưng đặt tên phương thức của `Chan<Input<ThaNoi>>` là `doc_khong_dam_bao()` — người đọc mã sẽ tự thấy vấn đề.

Vì sao chân thả nổi nguy hiểm? Nó không nối với nguồn cũng không nối với đất, nên điện áp trôi theo nhiễu môi trường. Đọc nó cho kết quả ngẫu nhiên — và tệ hơn, kết quả *có vẻ ổn định* trong phòng thí nghiệm rồi hỏng ngoài thực địa.
</details>

<details>
<summary><b>Lời giải</b></summary>

```rust
pub struct PullUp;
pub struct PullDown;
pub struct Floating;

pub struct InputWith<Tro>(PhantomData<Tro>);

impl<Tro> Block<InputWith<Tro>> {
    fn doc_tho(&self, data: &IntoRecordPrice) -> bool { data.test_bit(self.serial()) }
}

// Chỉ chân có điện trở kéo mới có `doc()` — trạng thái nghỉ xác định.
impl Block<InputWith<PullUp>> {
    /// Nút chưa bấm = mức CAO (bị điện trở kéo lên). Bấm = nối đất = THẤP.
    pub fn doc(&self, data: &IntoRecordPrice) -> bool { self.doc_tho(data) }
}
impl Block<InputWith<PullDown>> {
    pub fn doc(&self, data: &IntoRecordPrice) -> bool { self.doc_tho(data) }
}

impl Block<InputWith<Floating>> {
    /// Tên dài và xấu là CỐ Ý: chân thả nổi không có mức nghỉ xác định.
    /// Chỉ dùng khi mạch ngoài đã tự có điện trở kéo.
    pub fn read_unchecked(&self, data: &IntoRecordPrice) -> bool {
        self.doc_tho(data)
    }
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
```

Điểm tinh tế nhất là **hy sinh một ô nhớ**: hàng đợi `N` ô chỉ chứa được `N-1` phần tử, vì `dau == duoi` phải chỉ nghĩa "rỗng". Nếu cho phép chứa đủ `N`, trạng thái đầy và rỗng trông giống hệt nhau và không cách nào phân biệt mà không thêm biến đếm — mà thêm biến đếm thì lại cần cả hai bên cùng ghi, phá vỡ tính không-khóa.
</details>
