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
