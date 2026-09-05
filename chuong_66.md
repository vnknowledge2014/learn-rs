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
│    Chan<DauRa> có .bat() và .tat()      ← điều khiển đèn                    │
│    Chan<DauVao> có .doc()               ← đọc nút bấm                       │
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

`read_volatile`/`write_volatile` nói với trình biên dịch: **mỗi** thao tác đều có tác dụng phụ ngoài tầm hiểu biết của mày, đừng gộp, đừng xóa, đừng đảo thứ tự. Trong chương này, `ThanhGhiGia` đếm số lần đọc/ghi để bạn *thấy* được điều đó trong bài kiểm thử.

### 3. Đọc-Sửa-Ghi và cái bẫy ngắt

Muốn bật bit 3 mà không đụng các bit khác, phải làm ba bước: **đọc** giá trị hiện tại, **sửa** bit, **ghi** lại. Nhưng nếu một ngắt xảy ra *giữa* bước đọc và bước ghi, và ngắt đó cũng sửa cùng thanh ghi, thay đổi của nó sẽ bị ghi đè mất.

Ba cách xử lý:
1. **Vùng găng** (critical section): tắt ngắt trong lúc đọc-sửa-ghi. Đơn giản nhưng làm tăng độ trễ ngắt.
2. **Thanh ghi bit-band**: nhiều vi điều khiển ARM cung cấp vùng địa chỉ mà mỗi bit có địa chỉ riêng — ghi một bit thành một lệnh duy nhất, không thể bị cắt ngang.
3. **Thanh ghi set/clear riêng**: STM32 có `BSRR` — ghi 1 vào bit `n` thì bật chân `n`, ghi 1 vào bit `n+16` thì tắt chân `n`. Không cần đọc trước.

### 4. Typestate: chuyển trạng thái phải **tiêu thụ** giá trị cũ

Điểm mấu chốt của typestate là chữ ký hàm:

```rust
pub fn thanh_dau_ra(self, tg: &ThanhGhiGia) -> Chan<DauRa>
//                  ^^^^ nhận `self` theo GIÁ TRỊ, không phải `&self`
```

Vì nhận `self`, chân cũ bị **di chuyển** và không dùng lại được. Nhờ vậy không bao giờ tồn tại đồng thời hai cách nhìn về cùng một chân phần cứng. Nếu dùng `&self`, bạn có thể tạo `Chan<DauRa>` mà vẫn giữ `Chan<DauVao>` cũ — và trình biên dịch sẽ vui vẻ cho phép bạn vừa đọc vừa ghi cùng một chân.

Chi phí lúc chạy: **bằng không**. `PhantomData<DauRa>` không chiếm byte nào; `size_of::<Chan<DauRa>>() == size_of::<u8>()`. Toàn bộ kiểm tra biến mất sau khi biên dịch.

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
//         ▲ một ngắt chen vào ĐÂY sẽ khiến ngoại vi bị giao HAI lần
```

`AtomicBool::swap` làm cả hai việc trong **một** thao tác không thể bị cắt ngang: đặt giá trị mới *và* trả về giá trị cũ. Nếu giá trị cũ là `true`, ta biết chắc có người lấy trước — không có khe hở nào.

---

## Mã nguồn minh họa thực chiến (Idiomatic Runnable Rust Blueprint)

Mã dưới đây chạy được trên máy tính để bàn (để kiểm thử được). Trên vi điều khiển thật, bạn thêm `#![no_std]` + `#![no_main]`, thay `ThanhGhiGia` bằng `read_volatile`/`write_volatile` trên địa chỉ thật, và dùng crate HAL của dòng chip (`stm32f4xx-hal`, `rp2040-hal`, `esp-hal`…).

Chạy bằng `cargo run -p ch66`, kiểm thử bằng `cargo test -p ch66`.

```rust
#![allow(dead_code)]
//! Chương 66 — Lập trình nhúng & `no_std`: thanh ghi ánh xạ bộ nhớ, mẫu Singleton
//! cho ngoại vi, typestate cho chân GPIO, số dấu phẩy tĩnh, và bộ đệm vòng không cấp phát.
//!
//! Ghi chú: tệp này chạy trên máy tính để bàn để KIỂM THỬ ĐƯỢC. Trên vi điều khiển
//! thật, bạn thêm `#![no_std]` + `#![no_main]` và thay `ThanhGhiGia` bằng địa chỉ thật.

use core::marker::PhantomData;
use std::cell::Cell;
use std::sync::atomic::{AtomicBool, Ordering};

// ============================================================================
// 1. THANH GHI ÁNH XẠ BỘ NHỚ (MMIO) — phần cứng trông như biến
// ============================================================================

/// Trên vi điều khiển, ghi vào địa chỉ 0x4002_0014 sẽ BẬT một chân đèn.
/// Không có `volatile`, trình tối ưu hóa có quyền xóa lệnh ghi đó — vì theo
/// nó, ghi vào bộ nhớ rồi không đọc lại là việc vô nghĩa.
pub struct ThanhGhiGia {
    o_nho: Cell<u32>,
    pub so_lan_ghi: Cell<u32>,
    pub so_lan_doc: Cell<u32>,
}

impl ThanhGhiGia {
    pub fn moi(gia_tri: u32) -> Self {
        ThanhGhiGia { o_nho: Cell::new(gia_tri), so_lan_ghi: Cell::new(0), so_lan_doc: Cell::new(0) }
    }
    /// Tương ứng `core::ptr::write_volatile` — MỖI lệnh ghi đều phải xảy ra thật.
    pub fn ghi(&self, v: u32) { self.o_nho.set(v); self.so_lan_ghi.set(self.so_lan_ghi.get() + 1); }
    /// Tương ứng `core::ptr::read_volatile` — không được lưu vào thanh ghi CPU dùng lại.
    pub fn doc(&self) -> u32 { self.so_lan_doc.set(self.so_lan_doc.get() + 1); self.o_nho.get() }

    /// Đọc-Sửa-Ghi: mẫu thao tác bit chuẩn của lập trình nhúng.
    pub fn dat_bit(&self, bit: u8) { self.ghi(self.doc() | (1 << bit)); }
    pub fn xoa_bit(&self, bit: u8) { self.ghi(self.doc() & !(1 << bit)); }
    pub fn dao_bit(&self, bit: u8) { self.ghi(self.doc() ^ (1 << bit)); }
    pub fn thu_bit(&self, bit: u8) -> bool { self.doc() & (1 << bit) != 0 }

    /// Ghi một trường nhiều bit mà KHÔNG đụng các bit khác.
    pub fn ghi_truong(&self, lech: u8, rong: u8, gia_tri: u32) {
        let mat_na = ((1u32 << rong) - 1) << lech;
        self.ghi((self.doc() & !mat_na) | ((gia_tri << lech) & mat_na));
    }
    pub fn doc_truong(&self, lech: u8, rong: u8) -> u32 {
        (self.doc() >> lech) & ((1u32 << rong) - 1)
    }
}

// ============================================================================
// 2. TYPESTATE CHO CHÂN GPIO — cấu hình sai KHÔNG BIÊN DỊCH ĐƯỢC
// ============================================================================
// Đây là Chương 20 (Typestate) áp dụng vào phần cứng: trạng thái của chân
// nằm trong KIỂU, nên trình biên dịch chặn "đọc từ chân đang ở chế độ ra".

pub struct ChuaCauHinh;
pub struct DauVao;
pub struct DauRa;
pub struct TuongTu;   // analog — cho ADC

pub struct Chan<CheDo> {
    so_hieu: u8,
    _che_do: PhantomData<CheDo>,
}

impl Chan<ChuaCauHinh> {
    /// `unsafe` vì tạo hai `Chan` cùng số hiệu sẽ phá vỡ độc quyền phần cứng.
    /// Trong thực tế bạn chỉ gọi nó qua Singleton ở mục 3.
    pub unsafe fn moi(so_hieu: u8) -> Self { Chan { so_hieu, _che_do: PhantomData } }
}

impl<CheDo> Chan<CheDo> {
    pub fn so_hieu(&self) -> u8 { self.so_hieu }
    /// Chuyển chế độ TIÊU THỤ chân cũ (`self`) và trả về chân kiểu mới.
    /// Nhờ vậy không tồn tại đồng thời hai cách nhìn về cùng một chân.
    pub fn thanh_dau_ra(self, tg: &ThanhGhiGia) -> Chan<DauRa> {
        tg.ghi_truong(self.so_hieu * 2, 2, 0b01); // MODER = 01 (output)
        Chan { so_hieu: self.so_hieu, _che_do: PhantomData }
    }
    pub fn thanh_dau_vao(self, tg: &ThanhGhiGia) -> Chan<DauVao> {
        tg.ghi_truong(self.so_hieu * 2, 2, 0b00); // MODER = 00 (input)
        Chan { so_hieu: self.so_hieu, _che_do: PhantomData }
    }
    pub fn thanh_tuong_tu(self, tg: &ThanhGhiGia) -> Chan<TuongTu> {
        tg.ghi_truong(self.so_hieu * 2, 2, 0b11); // MODER = 11 (analog)
        Chan { so_hieu: self.so_hieu, _che_do: PhantomData }
    }
}

// CHỈ chân đầu ra mới có `bat`/`tat` — gọi trên chân đầu vào là lỗi biên dịch.
impl Chan<DauRa> {
    pub fn bat(&mut self, du_lieu: &ThanhGhiGia) { du_lieu.dat_bit(self.so_hieu); }
    pub fn tat(&mut self, du_lieu: &ThanhGhiGia) { du_lieu.xoa_bit(self.so_hieu); }
    pub fn dao(&mut self, du_lieu: &ThanhGhiGia) { du_lieu.dao_bit(self.so_hieu); }
}

// CHỈ chân đầu vào mới có `doc`.
impl Chan<DauVao> {
    pub fn doc(&self, du_lieu: &ThanhGhiGia) -> bool { du_lieu.thu_bit(self.so_hieu) }
}

// ============================================================================
// 3. SINGLETON NGOẠI VI — "chỉ có MỘT bộ ngoại vi trên con chip này"
// ============================================================================

/// Gói TẤT CẢ ngoại vi của con chip. Ai cầm được nó là chủ duy nhất của phần cứng.
pub struct BoNgoaiVi {
    pub cong_a: Chan<ChuaCauHinh>,
    pub cong_b: Chan<ChuaCauHinh>,
}

/// Cờ nguyên tử thay cho `static mut`: an toàn cả khi có ngắt xen giữa.
/// `swap` là thao tác ĐỌC-VÀ-ĐẶT không thể bị cắt ngang — nếu dùng
/// `if !DA_LAY { DA_LAY = true }` thì một ngắt chen vào giữa hai câu lệnh
/// có thể khiến ngoại vi bị giao HAI lần.
static DA_LAY: AtomicBool = AtomicBool::new(false);

impl BoNgoaiVi {
    /// Trả `Some` đúng MỘT lần trong suốt vòng đời chương trình.
    /// Lần thứ hai trả `None` — không thể có hai chủ sở hữu cùng điều khiển chip.
    pub fn lay() -> Option<BoNgoaiVi> {
        if DA_LAY.swap(true, Ordering::SeqCst) {
            return None; // đã có người lấy trước
        }
        // An toàn: cờ trên bảo đảm đoạn này chạy đúng một lần.
        Some(unsafe { BoNgoaiVi { cong_a: Chan::moi(5), cong_b: Chan::moi(13) } })
    }
    #[doc(hidden)]
    pub fn dat_lai_cho_kiem_thu() { DA_LAY.store(false, Ordering::SeqCst); }
}

// ============================================================================
// 4. SỐ DẤU PHẨY TĨNH — vì phần lớn vi điều khiển KHÔNG có FPU
// ============================================================================

/// Q16.16: 16 bit phần nguyên, 16 bit phần thập phân, đựng trong một `i32`.
/// Nhân/chia bằng số nguyên → nhanh gấp hàng chục lần mô phỏng dấu phẩy động.
#[derive(Debug, Clone, Copy, PartialEq, Eq, PartialOrd, Ord)]
pub struct Q16(pub i32);

impl Q16 {
    pub const MOT: Q16 = Q16(1 << 16);
    pub fn tu_nguyen(n: i16) -> Q16 { Q16((n as i32) << 16) }
    /// Chỉ dùng khi biên dịch trên máy có dấu phẩy động (lúc thiết kế hằng số).
    pub fn tu_thuc(x: f64) -> Q16 { Q16((x * 65536.0).round() as i32) }
    pub fn thanh_thuc(self) -> f64 { self.0 as f64 / 65536.0 }
    pub fn cong(self, k: Q16) -> Q16 { Q16(self.0.wrapping_add(k.0)) }
    pub fn tru(self, k: Q16) -> Q16 { Q16(self.0.wrapping_sub(k.0)) }
    /// Nhân phải qua i64 rồi dịch phải 16 — nếu không sẽ tràn ngay.
    pub fn nhan(self, k: Q16) -> Q16 { Q16(((self.0 as i64 * k.0 as i64) >> 16) as i32) }
    pub fn chia(self, k: Q16) -> Q16 { Q16((((self.0 as i64) << 16) / k.0 as i64) as i32) }
}

/// Chuyển giá trị ADC 12-bit (0..4095) sang nhiệt độ °C, toàn số nguyên.
/// Cảm biến giả định: 0 → -40 °C, 4095 → 125 °C (tuyến tính).
pub fn adc_sang_nhiet_do(adc: u16) -> Q16 {
    let ti_le = Q16::tu_thuc(165.0 / 4095.0);
    Q16::tu_nguyen(adc as i16).nhan(ti_le).tru(Q16::tu_nguyen(40))
}

// ============================================================================
// 5. BỘ ĐỆM VÒNG KHÔNG CẤP PHÁT — `heapless` thu nhỏ
// ============================================================================

/// Không `Vec`, không `Box`, không heap. Bộ nhớ nằm gọn trong struct,
/// kích thước biết trước lúc biên dịch. Đây là kiểu dữ liệu chủ lực của
/// ngắt UART: ISR đẩy byte vào, vòng lặp chính lấy ra.
pub struct DemVong<const N: usize> {
    o: [u8; N],
    dau: usize,
    duoi: usize,
    so_luong: usize,
}

impl<const N: usize> DemVong<N> {
    pub const fn moi() -> Self { DemVong { o: [0; N], dau: 0, duoi: 0, so_luong: 0 } }
    pub fn suc_chua(&self) -> usize { N }
    pub fn so_luong(&self) -> usize { self.so_luong }
    pub fn rong(&self) -> bool { self.so_luong == 0 }
    pub fn day(&self) -> bool { self.so_luong == N }

    /// Trả `Err` thay vì cấp phát thêm — hệ nhúng KHÔNG được phép "cứ lớn dần".
    pub fn day_vao(&mut self, b: u8) -> Result<(), u8> {
        if self.day() { return Err(b); }
        self.o[self.duoi] = b;
        self.duoi = (self.duoi + 1) % N;
        self.so_luong += 1;
        Ok(())
    }
    pub fn lay_ra(&mut self) -> Option<u8> {
        if self.rong() { return None; }
        let b = self.o[self.dau];
        self.dau = (self.dau + 1) % N;
        self.so_luong -= 1;
        Some(b)
    }
    /// Ghi đè phần tử cũ nhất khi đầy — dùng cho nhật ký sự cố (black box).
    pub fn day_ghi_de(&mut self, b: u8) -> Option<u8> {
        let bi_mat = if self.day() { self.lay_ra() } else { None };
        let _ = self.day_vao(b);
        bi_mat
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
    on_dinh: bool,
    dem: u8,
    nguong: u8,
}

impl ChongRung {
    pub fn moi(nguong: u8) -> Self { ChongRung { on_dinh: false, dem: 0, nguong } }
    /// Trả `Some(trạng thái mới)` chỉ tại đúng khoảnh khắc chuyển.
    pub fn cap_nhat(&mut self, mau_tho: bool) -> Option<bool> {
        if mau_tho == self.on_dinh {
            self.dem = 0;
            return None;
        }
        self.dem += 1;
        if self.dem >= self.nguong {
            self.on_dinh = mau_tho;
            self.dem = 0;
            return Some(self.on_dinh);
        }
        None
    }
    pub fn trang_thai(&self) -> bool { self.on_dinh }
}

fn main() {
    println!("═══════════════════════════════════════════════════════════");
    println!("   LẬP TRÌNH NHÚNG: MMIO · TYPESTATE GPIO · Q16.16 · no_std ");
    println!("═══════════════════════════════════════════════════════════");

    println!("\n1. THANH GHI ÁNH XẠ BỘ NHỚ");
    let moder = ThanhGhiGia::moi(0);
    let odr = ThanhGhiGia::moi(0);
    moder.ghi_truong(10, 2, 0b01);
    println!("   MODER sau khi đặt chân 5 thành output: 0b{:032b}", moder.doc());
    println!("   Số lệnh ghi thực sự chạm phần cứng   : {}", moder.so_lan_ghi.get());

    println!("\n2. TYPESTATE GPIO — sai kiểu là không biên dịch được");
    let bo = BoNgoaiVi::lay().expect("lần đầu phải lấy được");
    println!("   BoNgoaiVi::lay() lần hai → {:?}", BoNgoaiVi::lay().is_none());
    let mut den = bo.cong_a.thanh_dau_ra(&moder);
    let nut = bo.cong_b.thanh_dau_vao(&moder);
    den.bat(&odr);
    println!("   Bật đèn chân {} → ODR = 0b{:016b}", den.so_hieu(), odr.doc());
    println!("   Đọc nút chân {}  → {}", nut.so_hieu(), nut.doc(&odr));
    println!("   ❌ nut.bat(&odr)   → E0599: không có phương thức `bat` cho Chan<DauVao>");

    println!("\n3. SỐ DẤU PHẨY TĨNH Q16.16 (không cần FPU)");
    for adc in [0u16, 1024, 2048, 4095] {
        let t = adc_sang_nhiet_do(adc);
        println!("   ADC {:>4} → {:>8.3} °C (bên trong chỉ là i32 = {})", adc, t.thanh_thuc(), t.0);
    }
    let a = Q16::tu_thuc(3.5);
    let b = Q16::tu_thuc(2.0);
    println!("   3.5 × 2.0 = {} · 3.5 ÷ 2.0 = {}", a.nhan(b).thanh_thuc(), a.chia(b).thanh_thuc());

    println!("\n4. BỘ ĐỆM VÒNG KHÔNG CẤP PHÁT (4 byte)");
    let mut dem: DemVong<4> = DemVong::moi();
    for b in b"RUST" { dem.day_vao(*b).unwrap(); }
    println!("   Đầy: {} | đẩy thêm 'X' → {:?}", dem.day(), dem.day_vao(b'X').unwrap_err() as char);
    println!("   Ghi đè 'X' → mất byte {:?}", dem.day_ghi_de(b'X').map(|b| b as char));
    let con: Vec<char> = std::iter::from_fn(|| dem.lay_ra()).map(|b| b as char).collect();
    println!("   Nội dung còn lại: {:?}", con);

    println!("\n5. CHỐNG RUNG PHÍM (ngưỡng 3 mẫu)");
    let mut cr = ChongRung::moi(3);
    let mau = [false, true, false, true, true, true, true, false, true, false, false, false];
    let mut ket_qua = Vec::new();
    for (i, &m) in mau.iter().enumerate() {
        if let Some(moi) = cr.cap_nhat(m) { ket_qua.push((i, moi)); }
    }
    println!("   12 mẫu nhiễu → chỉ {} sự kiện thật: {:?}", ket_qua.len(), ket_qua);

    println!("\n═══════════════════════════════════════════════════════════");
    println!("   NHÚNG = KHÔNG HỆ ĐIỀU HÀNH, KHÔNG HEAP, KHÔNG THA THỨ     ");
    println!("═══════════════════════════════════════════════════════════");
}

#[cfg(test)]
mod kiem_thu {
    use super::*;

    // ---------- MMIO ----------
    #[test]
    fn thao_tac_bit_khong_dung_bit_khac() {
        let tg = ThanhGhiGia::moi(0b1010_0000);
        tg.dat_bit(0);
        assert_eq!(tg.doc(), 0b1010_0001, "đặt bit 0 phải giữ nguyên bit 5 và 7");
        tg.xoa_bit(7);
        assert_eq!(tg.doc(), 0b0010_0001);
        tg.dao_bit(5);
        assert_eq!(tg.doc(), 0b0000_0001);
    }

    #[test]
    fn ghi_truong_chi_dung_dung_so_bit() {
        let tg = ThanhGhiGia::moi(0xFFFF_FFFF);
        tg.ghi_truong(4, 3, 0b010); // đặt 3 bit tại vị trí 4
        assert_eq!(tg.doc_truong(4, 3), 0b010);
        assert_eq!(tg.doc(), 0xFFFF_FFAF, "mọi bit ngoài trường phải nguyên vẹn");
    }

    #[test]
    fn gia_tri_tran_bi_cat_theo_do_rong_truong() {
        let tg = ThanhGhiGia::moi(0);
        tg.ghi_truong(0, 2, 0b1111); // chỉ 2 bit chứa được
        assert_eq!(tg.doc(), 0b11, "phần thừa bị mặt nạ chặn, không tràn sang bit 2");
    }

    #[test]
    fn doc_sua_ghi_ton_dung_mot_lenh_ghi() {
        let tg = ThanhGhiGia::moi(0);
        tg.dat_bit(3);
        assert_eq!(tg.so_lan_ghi.get(), 1);
        assert_eq!(tg.so_lan_doc.get(), 1);
    }

    // ---------- Typestate GPIO ----------
    #[test]
    fn chuyen_che_do_ghi_dung_ma_moder() {
        let moder = ThanhGhiGia::moi(0);
        let c = unsafe { Chan::moi(5) };
        let _ra = c.thanh_dau_ra(&moder);
        assert_eq!(moder.doc_truong(10, 2), 0b01, "chân 5 → bit 10-11 = 01 (output)");
    }

    #[test]
    fn dau_ra_bat_tat_dung_chan() {
        let moder = ThanhGhiGia::moi(0);
        let odr = ThanhGhiGia::moi(0);
        let mut c = unsafe { Chan::moi(3) }.thanh_dau_ra(&moder);
        c.bat(&odr);
        assert_eq!(odr.doc(), 0b1000);
        c.dao(&odr);
        assert_eq!(odr.doc(), 0);
    }

    #[test]
    fn vong_doi_chan_di_qua_nhieu_che_do() {
        let moder = ThanhGhiGia::moi(0);
        let c = unsafe { Chan::moi(2) };
        let ra = c.thanh_dau_ra(&moder);
        let vao = ra.thanh_dau_vao(&moder);      // tiêu thụ chân đầu ra
        let tt = vao.thanh_tuong_tu(&moder);      // rồi thành analog
        assert_eq!(tt.so_hieu(), 2, "số hiệu chân theo suốt mọi lần đổi kiểu");
        assert_eq!(moder.doc_truong(4, 2), 0b11);
    }

    #[test]
    fn singleton_chi_giao_ngoai_vi_dung_mot_lan() {
        BoNgoaiVi::dat_lai_cho_kiem_thu();
        assert!(BoNgoaiVi::lay().is_some(), "lần đầu phải thành công");
        assert!(BoNgoaiVi::lay().is_none(), "lần hai phải bị từ chối");
        assert!(BoNgoaiVi::lay().is_none());
        BoNgoaiVi::dat_lai_cho_kiem_thu();
    }

    // ---------- Q16.16 ----------
    #[test]
    fn q16_cong_tru_chinh_xac_tuyet_doi() {
        let a = Q16::tu_nguyen(7);
        let b = Q16::tu_nguyen(3);
        assert_eq!(a.cong(b), Q16::tu_nguyen(10));
        assert_eq!(a.tru(b), Q16::tu_nguyen(4));
    }

    #[test]
    fn q16_nhan_chia_sai_so_duoi_mot_phan_65536() {
        let a = Q16::tu_thuc(3.5);
        let b = Q16::tu_thuc(2.25);
        assert!((a.nhan(b).thanh_thuc() - 7.875).abs() < 1.0 / 65536.0);
        assert!((a.chia(b).thanh_thuc() - 3.5 / 2.25).abs() < 1.0 / 65536.0);
    }

    #[test]
    fn q16_nhan_voi_mot_la_phep_dong_nhat() {
        for x in [0.0, 1.5, -3.25, 100.125] {
            let q = Q16::tu_thuc(x);
            assert_eq!(q.nhan(Q16::MOT), q, "nhân với 1 phải trả lại chính nó");
        }
    }

    #[test]
    fn adc_sang_nhiet_do_dung_hai_dau_thang_do() {
        assert!((adc_sang_nhiet_do(0).thanh_thuc() - (-40.0)).abs() < 0.01);
        assert!((adc_sang_nhiet_do(4095).thanh_thuc() - 125.0).abs() < 0.05);
        // và đơn điệu tăng
        let mut truoc = adc_sang_nhiet_do(0);
        for adc in (100..4096).step_by(100) {
            let nay = adc_sang_nhiet_do(adc as u16);
            assert!(nay > truoc, "nhiệt độ phải tăng đơn điệu theo ADC");
            truoc = nay;
        }
    }

    // ---------- Bộ đệm vòng ----------
    #[test]
    fn dem_vong_vao_truoc_ra_truoc() {
        let mut d: DemVong<4> = DemVong::moi();
        for b in [1u8, 2, 3] { d.day_vao(b).unwrap(); }
        assert_eq!(d.lay_ra(), Some(1));
        assert_eq!(d.lay_ra(), Some(2));
        assert_eq!(d.so_luong(), 1);
    }

    #[test]
    fn dem_vong_bao_loi_thay_vi_cap_phat_them() {
        let mut d: DemVong<2> = DemVong::moi();
        d.day_vao(1).unwrap();
        d.day_vao(2).unwrap();
        assert_eq!(d.day_vao(3), Err(3), "đầy thì TRẢ LẠI byte, không được lớn thêm");
        assert_eq!(d.suc_chua(), 2, "sức chứa cố định lúc biên dịch");
    }

    #[test]
    fn dem_vong_quay_vong_dung_sau_nhieu_luot() {
        let mut d: DemVong<3> = DemVong::moi();
        for i in 0..30u8 {
            d.day_vao(i).unwrap();
            assert_eq!(d.lay_ra(), Some(i), "chỉ số phải quay vòng đúng qua biên mảng");
        }
        assert!(d.rong());
    }

    #[test]
    fn dem_vong_ghi_de_bo_phan_tu_cu_nhat() {
        let mut d: DemVong<3> = DemVong::moi();
        for b in [1u8, 2, 3] { d.day_vao(b).unwrap(); }
        assert_eq!(d.day_ghi_de(4), Some(1), "phần tử CŨ NHẤT bị hy sinh");
        let con: Vec<u8> = std::iter::from_fn(|| d.lay_ra()).collect();
        assert_eq!(con, vec![2, 3, 4]);
    }

    #[test]
    fn dem_vong_rong_tra_none() {
        let mut d: DemVong<4> = DemVong::moi();
        assert_eq!(d.lay_ra(), None);
        assert!(d.rong() && !d.day());
    }

    // ---------- Chống rung ----------
    #[test]
    fn chong_rung_bo_qua_nhieu_ngan() {
        let mut c = ChongRung::moi(3);
        // nhiễu: bật-tắt liên tục, không mẫu nào đủ 3 lần liên tiếp
        for m in [true, false, true, false, true, false] {
            assert_eq!(c.cap_nhat(m), None, "nhiễu không được sinh sự kiện");
        }
        assert!(!c.trang_thai());
    }

    #[test]
    fn chong_rung_chap_nhan_tin_hieu_on_dinh() {
        let mut c = ChongRung::moi(3);
        assert_eq!(c.cap_nhat(true), None);
        assert_eq!(c.cap_nhat(true), None);
        assert_eq!(c.cap_nhat(true), Some(true), "đủ 3 mẫu → chuyển trạng thái");
        assert_eq!(c.cap_nhat(true), None, "giữ nguyên thì không phát lại sự kiện");
    }

    #[test]
    fn chong_rung_phat_dung_mot_su_kien_cho_mot_cu_bam() {
        let mut c = ChongRung::moi(2);
        let mau = [false, true, false, true, true, true, true, true];
        let so_su_kien = mau.iter().filter(|&&m| c.cap_nhat(m).is_some()).count();
        assert_eq!(so_su_kien, 1, "một cú bấm nảy = đúng một sự kiện");
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
    let bo = BoNgoaiVi::lay().unwrap();
    let moder = unsafe { &*(0x4002_0000 as *const ThanhGhiThat) };
    let mut den = bo.cong_a.thanh_dau_ra(moder);

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
| `E0599: no method named 'bat' found for struct Chan<DauVao>` | **Đây là tính năng!** Bạn đang cố ghi vào chân cấu hình làm đầu vào | Gọi `.thanh_dau_ra(&moder)` trước |
| `E0382: use of moved value: 'chan'` | Dùng lại chân sau khi đã đổi chế độ | Đúng như thiết kế — dùng giá trị **trả về** của `thanh_dau_ra` |
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
5. **Số dấu phẩy tĩnh là bạn của vi điều khiển.** Q16.16 cho sai số dưới 0,00002 mà chỉ dùng phép toán số nguyên.

### Bài tập rèn luyện tự giải

**Bài 1.** Cài **bộ lọc trung bình trượt** cho dữ liệu cảm biến, dùng `DemVong` và số Q16.16, **không cấp phát**.

<details>
<summary><b>Gợi ý</b></summary>

Giữ một `DemVong<N>` các mẫu **và** một biến `tong` chạy. Khi đẩy mẫu mới vào bộ đệm đầy, trừ mẫu bị đuổi ra khỏi `tong` rồi cộng mẫu mới vào. Nhờ vậy tính trung bình là O(1) thay vì O(N).

Cẩn thận với tràn số: `tong` phải đủ rộng để chứa `N` mẫu Q16.16 cộng lại.
</details>

<details>
<summary><b>Lời giải</b></summary>

```rust
pub struct TrungBinhTruot<const N: usize> {
    mau: [Q16; N],
    chi_so: usize,
    so_mau: usize,
    tong: i64,      // i64 để chắc chắn không tràn khi cộng N mẫu i32
}

impl<const N: usize> TrungBinhTruot<N> {
    pub const fn moi() -> Self {
        TrungBinhTruot { mau: [Q16(0); N], chi_so: 0, so_mau: 0, tong: 0 }
    }
    /// O(1): trừ mẫu cũ, cộng mẫu mới — không duyệt lại cả mảng.
    pub fn them(&mut self, x: Q16) -> Q16 {
        self.tong -= self.mau[self.chi_so].0 as i64;   // bỏ mẫu bị ghi đè
        self.mau[self.chi_so] = x;
        self.tong += x.0 as i64;
        self.chi_so = (self.chi_so + 1) % N;
        if self.so_mau < N { self.so_mau += 1; }
        Q16((self.tong / self.so_mau as i64) as i32)
    }
}
```

Chú ý `self.so_mau` thay vì `N` ở mẫu số: trong `N` lần gọi đầu tiên bộ đệm chưa đầy, chia cho `N` sẽ cho kết quả nhỏ hơn thực tế — một lỗi khởi động kinh điển khiến cảm biến báo sai trong vài giây đầu.
</details>

**Bài 2.** Mở rộng typestate để phân biệt chân đầu vào **kéo lên** (pull-up), **kéo xuống** (pull-down) và **thả nổi** (floating), sao cho việc đọc một chân thả nổi phải sinh cảnh báo.

<details>
<summary><b>Gợi ý</b></summary>

Dùng typestate **hai tầng**: `Chan<DauVao<KeoLen>>`. Cài `doc()` cho `Chan<DauVao<KeoLen>>` và `Chan<DauVao<KeoXuong>>`, nhưng đặt tên phương thức của `Chan<DauVao<ThaNoi>>` là `doc_khong_dam_bao()` — người đọc mã sẽ tự thấy vấn đề.

Vì sao chân thả nổi nguy hiểm? Nó không nối với nguồn cũng không nối với đất, nên điện áp trôi theo nhiễu môi trường. Đọc nó cho kết quả ngẫu nhiên — và tệ hơn, kết quả *có vẻ ổn định* trong phòng thí nghiệm rồi hỏng ngoài thực địa.
</details>

<details>
<summary><b>Lời giải</b></summary>

```rust
pub struct KeoLen;
pub struct KeoXuong;
pub struct ThaNoi;

pub struct DauVaoVoi<Tro>(PhantomData<Tro>);

impl<Tro> Chan<DauVaoVoi<Tro>> {
    fn doc_tho(&self, du_lieu: &ThanhGhiGia) -> bool { du_lieu.thu_bit(self.so_hieu()) }
}

// Chỉ chân có điện trở kéo mới có `doc()` — trạng thái nghỉ xác định.
impl Chan<DauVaoVoi<KeoLen>> {
    /// Nút chưa bấm = mức CAO (bị điện trở kéo lên). Bấm = nối đất = THẤP.
    pub fn doc(&self, du_lieu: &ThanhGhiGia) -> bool { self.doc_tho(du_lieu) }
}
impl Chan<DauVaoVoi<KeoXuong>> {
    pub fn doc(&self, du_lieu: &ThanhGhiGia) -> bool { self.doc_tho(du_lieu) }
}

impl Chan<DauVaoVoi<ThaNoi>> {
    /// Tên dài và xấu là CỐ Ý: chân thả nổi không có mức nghỉ xác định.
    /// Chỉ dùng khi mạch ngoài đã tự có điện trở kéo.
    pub fn doc_khong_dam_bao(&self, du_lieu: &ThanhGhiGia) -> bool {
        self.doc_tho(du_lieu)
    }
}
```

Đây là kỹ thuật thiết kế API quan trọng: **làm cho việc đúng dễ làm, việc nguy hiểm khó gõ**. Không cấm hẳn (đôi khi thả nổi là đúng), nhưng buộc người viết phải gõ ra một cái tên tự tố cáo.
</details>

**Bài 3.** Cài **hàng đợi một-nhà-sản-xuất-một-người-tiêu-thụ** (SPSC) an toàn giữa ngắt và vòng lặp chính, không dùng khóa.

<details>
<summary><b>Gợi ý</b></summary>

Đây là bài toán kinh điển: ngắt UART đẩy byte vào, vòng lặp chính lấy ra. Vì chỉ có **một** bên ghi `duoi` và **một** bên ghi `dau`, ta không cần khóa — chỉ cần hai `AtomicUsize` với thứ tự bộ nhớ đúng.

Người sản xuất: đọc `dau` (Acquire), ghi dữ liệu, rồi ghi `duoi` (Release).
Người tiêu thụ: đọc `duoi` (Acquire), đọc dữ liệu, rồi ghi `dau` (Release).

Cặp Release/Acquire bảo đảm: khi bên kia *thấy* con trỏ mới, nó cũng thấy dữ liệu đã ghi xong.
</details>

<details>
<summary><b>Lời giải</b></summary>

```rust
use core::cell::UnsafeCell;
use core::sync::atomic::AtomicUsize;   // `Ordering` chương đã nhập ở trên

pub struct HangSpsc<const N: usize> {
    o: UnsafeCell<[u8; N]>,
    dau: AtomicUsize,   // CHỈ người tiêu thụ ghi
    duoi: AtomicUsize,  // CHỈ người sản xuất ghi
}

// An toàn: mỗi con trỏ chỉ có ĐÚNG MỘT bên ghi, nên không có cuộc đua ghi-ghi.
unsafe impl<const N: usize> Sync for HangSpsc<N> {}

impl<const N: usize> HangSpsc<N> {
    pub const fn moi() -> Self {
        HangSpsc {
            o: UnsafeCell::new([0; N]),
            dau: AtomicUsize::new(0),
            duoi: AtomicUsize::new(0),
        }
    }

    /// Gọi TỪ NGẮT. Trả `Err` nếu đầy — không bao giờ chặn.
    pub fn day_vao(&self, b: u8) -> Result<(), u8> {
        let duoi = self.duoi.load(Ordering::Relaxed);   // ta là bên duy nhất ghi nó
        let duoi_moi = (duoi + 1) % N;
        if duoi_moi == self.dau.load(Ordering::Acquire) {
            return Err(b); // đầy — hy sinh byte còn hơn chặn ngắt
        }
        unsafe { (*self.o.get())[duoi] = b; }
        // Release: bảo đảm lệnh ghi dữ liệu ở trên HOÀN TẤT trước khi
        // người tiêu thụ nhìn thấy con trỏ mới.
        self.duoi.store(duoi_moi, Ordering::Release);
        Ok(())
    }

    /// Gọi từ VÒNG LẶP CHÍNH.
    pub fn lay_ra(&self) -> Option<u8> {
        let dau = self.dau.load(Ordering::Relaxed);
        if dau == self.duoi.load(Ordering::Acquire) {
            return None; // rỗng
        }
        let b = unsafe { (*self.o.get())[dau] };
        self.dau.store((dau + 1) % N, Ordering::Release);
        Some(b)
    }
}
```

Điểm tinh tế nhất là **hy sinh một ô nhớ**: hàng đợi `N` ô chỉ chứa được `N-1` phần tử, vì `dau == duoi` phải chỉ nghĩa "rỗng". Nếu cho phép chứa đủ `N`, trạng thái đầy và rỗng trông giống hệt nhau và không cách nào phân biệt mà không thêm biến đếm — mà thêm biến đếm thì lại cần cả hai bên cùng ghi, phá vỡ tính không-khóa.
</details>
