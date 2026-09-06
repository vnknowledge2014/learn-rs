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
/// có thể khiến ngoại vi bị giao HAI lần.
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
// 5. BỘ ĐỆM VÒNG KHÔNG CẤP PHÁT — `heapless` thu nhỏ
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
