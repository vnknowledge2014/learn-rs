# Chương 74: Nền tảng HFT — Đo độ trễ, Vòng Disruptor & Bố cục bộ nhớ (HFT Foundations)

## Giới thiệu & Mục tiêu học tập

Giao dịch tần suất cao (HFT) là ngành duy nhất mà **nanosecond có giá bằng tiền mặt**. Ở Jane Street, Optiver hay Jump Trading, người ta viết lại toàn bộ ngăn xếp phần mềm chỉ để bớt vài trăm nanosecond.

Điều làm HFT khác biệt không phải là "code nhanh". Đó là ba nguyên tắc:

| Nguyên tắc | Nội dung |
|---|---|
| Đo bằng phân vị, không bằng trung bình | Trung bình giấu đi chính thứ giết bạn: cái đuôi |
| Không cấp phát trên đường nóng | `malloc` là bất định; bất định là kẻ thù |
| Bố cục bộ nhớ quan trọng hơn thuật toán | Một lần trượt cache xuống RAM ≈ 100 ns ≈ vài trăm chu kỳ — đủ cho hàng trăm phép cộng |

> Một câu nói lưu truyền trong ngành: *"Chúng tôi không tối ưu tốc độ trung bình. Chúng tôi tối ưu trường hợp tệ nhất, vì trường hợp tệ nhất là lúc thị trường đang biến động — tức là đúng lúc quan trọng nhất."*

Mục tiêu học tập:
- Đo độ trễ đúng cách bằng **biểu đồ phân vị**, và hiểu vì sao trung bình là con số vô dụng.
- Cài **vòng Disruptor** SPSC không khoá, không cấp phát.
- Hiểu **chia sẻ giả (false sharing)** và cách đệm theo dòng cache.
- So sánh **AoS và SoA** trên dữ liệu thật.
- Lập **ngân sách độ trễ** từ dây tới lệnh.

---

## Hình tượng hóa đời sống

```
┌──────────────────────────────────────────────────────────────────────────────┐
│  VÌ SAO TRUNG BÌNH LÀ CON SỐ VÔ DỤNG                                        │
│                                                                              │
│    Hai hệ thống, cùng độ trễ trung bình 500 ns:                             │
│                                                                              │
│    Hệ A: ████████████████████ đều đặn 480–520 ns                            │
│    Hệ B: ██████████ 300 ns (99,9%)  ▏  ███ 200 000 ns (0,1%)               │
│                                                                              │
│    Hệ B nhanh hơn ở 999/1000 lệnh. Nhưng lệnh thứ 1000 mất 200 µs —         │
│    và trong 200 µs đó thị trường đã chạy mất.                               │
│    Bạn thua đúng ở lúc đáng lẽ phải thắng.                                  │
│                                                                              │
│  CHIA SẺ GIẢ = HAI NGƯỜI GIÀNH MỘT CUỐN SỔ                                  │
│                                                                              │
│    Dòng cache (64 byte)                                                      │
│    ┌────────────────────────────────────────────┐                           │
│    │ bộ_đếm_A │ bộ_đếm_B │ ... còn trống ...    │                           │
│    └────────────────────────────────────────────┘                           │
│      ▲ lõi 0 ghi   ▲ lõi 1 ghi                                              │
│                                                                              │
│    Hai lõi ghi hai biến KHÁC NHAU. Nhưng chúng ở cùng một dòng cache,       │
│    nên mỗi lần ghi làm mất hiệu lực bản sao của lõi kia.                    │
│    Kết quả: chậm 5–10 lần mà không có bất kỳ tranh chấp logic nào.          │
│                                                                              │
│    Cách chữa: đệm mỗi biến đủ 64 byte → mỗi lõi một dòng riêng.             │
│                                                                              │
│  VÒNG DISRUPTOR = BĂNG CHUYỀN CÓ Ô CỐ ĐỊNH                                  │
│                                                                              │
│      ghi ──►┌───┬───┬───┬───┬───┬───┬───┬───┐                              │
│             │ 0 │ 1 │ 2 │ 3 │ 4 │ 5 │ 6 │ 7 │──┐                           │
│             └───┴───┴───┴───┴───┴───┴───┴───┘  │ vòng lại                  │
│                       ▲                         │                           │
│                      đọc ◄──────────────────────┘                           │
│                                                                              │
│    Bộ nhớ cấp phát MỘT LẦN lúc khởi động, rồi tái dùng mãi mãi.             │
│    Kích thước là luỹ thừa 2 → chỉ mục = con_trỏ & (N−1), không cần chia.    │
│    Con trỏ ĐƠN ĐIỆU TĂNG → phân biệt được "rỗng" với "đầy" mà không cờ.     │
│                                                                              │
│  AoS vs SoA                                                                 │
│    AoS: [giá,sl,mã][giá,sl,mã][giá,sl,mã]  ← đọc 1 lệnh: 1 dòng cache      │
│    SoA: [giá,giá,giá][sl,sl,sl][mã,mã,mã]  ← quét mọi giá: cực nhanh       │
│    Không có bên nào luôn thắng. Chọn theo CÁCH BẠN TRUY CẬP.               │
└──────────────────────────────────────────────────────────────────────────────┘
```

---

## Khái niệm & Cơ chế kỹ thuật chuyên sâu

### 1. Biểu đồ phân vị: đo cái đuôi

Trong HFT, các con số đáng quan tâm là p50, p99, p99.9 và **max**. Trung bình chỉ hữu ích để phát hiện là mình đã đo sai.

Cấu trúc phù hợp là biểu đồ kiểu HDR: các thùng có độ rộng tăng dần (logarit), nên lưu được dải từ 1 ns tới 1 phút mà chỉ tốn vài kilobyte, với sai số tương đối cố định. Bản trong chương này rút gọn tối đa — mỗi thùng là một luỹ thừa của 2, nên sai số có thể tới 2×; HDR thật chia mỗi luỹ thừa của 2 thành nhiều thùng con để sai số chỉ còn vài phần nghìn.

Bài kiểm thử của chương này minh hoạ chính xác cái bẫy: một phân phối gồm 99% mẫu ở 200–300 ns, 0,9% ở khoảng 2 µs và 0,1% ở khoảng 50 µs. p50 và p99 đều nhanh, p99.9 mới bắt đầu lộ, còn **max lớn hơn trung bình hơn 100 lần**. Nếu chỉ nhìn trung bình, bạn sẽ tuyên bố hệ thống "trung bình 330 ns" trong khi thực tế nó thỉnh thoảng đứng hình 50–60 µs.

### 2. Vì sao Disruptor không dùng khoá

Một mutex có ba vấn đề trên đường nóng:
- **Đảo ngược ưu tiên**: luồng giữ khoá bị hệ điều hành cho ra rìa, luồng quan trọng phải chờ.
- **Chuyển ngữ cảnh (Context switch)**: khi tranh chấp, chi phí là hàng microsecond — gấp hàng nghìn lần công việc thật.
- **Bất định**: bạn không biết trước lần này có tranh chấp hay không.

Disruptor thay bằng hai con trỏ nguyên tử **đơn điệu tăng**. Người ghi chỉ chờ khi vòng đầy; người đọc chỉ chờ khi vòng rỗng. Không có khoá nào cả.

Hai chi tiết cài đặt đáng nhớ:
- Kích thước **luỹ thừa của 2** để `chỉ_mục = con_trỏ & (N−1)` — phép AND một chu kỳ, thay cho phép chia hàng chục chu kỳ.
- Con trỏ **không bao giờ quay vòng** (`usize` 64 bit tăng mãi — ở 1 tỉ phần tử/giây phải mất hơn 500 năm mới tràn). Nhờ vậy `ghi − đọc` cho biết chính xác số phần tử, phân biệt được rỗng và đầy — điều mà con trỏ quay vòng không làm được nếu không hy sinh một ô.

### 3. Bể đối tượng: cấp phát trước, tái dùng mãi

Chiến lược của HFT là: **cấp phát toàn bộ lúc khởi động, không cấp phát gì nữa khi chạy**. Một `Vec::push` có thể tái cấp phát và sao chép — chi phí đó không dự đoán được.

Bể đối tượng (Object pool) giữ một danh sách chỉ số rỗi. `acquire()` là pop, `release()` là push. Cả hai đều O(1) và không chạm tới bộ cấp phát của hệ thống. Khi bể cạn, `acquire()` trả `None` — và đó là **hành vi đúng**: hệ thống từ chối tải mới thay vì tự làm chậm mình một cách bất định.

### 4. Ngân sách độ trễ: nơi thời gian thực sự đi

Một ngân sách "dây tới lệnh" điển hình của hệ thống dùng phần mềm và kernel bypass:

| Giai đoạn | ns |
|---|---|
| Card mạng nhận, DMA | ~250 |
| Phân tích gói tin | ~40 |
| Cập nhật sổ lệnh | ~80 |
| Tín hiệu chiến lược | ~120 |
| Kiểm soát rủi ro | ~30 |
| Đóng gói lệnh | ~35 |
| Card mạng gửi | ~250 |

Nhìn bảng này, ta thấy ngay: **hơn nửa thời gian nằm ở card mạng**, không nằm trong logic. Đó là lý do các hãng HFT chuyển sang FPGA (chương 79) — không phải vì code chậm, mà vì lớp mạng là trần cứng.

Đây cũng là **định luật Amdahl** áp dụng thẳng: tối ưu tín hiệu chiến lược nhanh gấp đôi chỉ cải thiện tổng thể khoảng 7%.

Lưu ý: `main` của chương dùng một ngân sách **khác** (`sample_budget`, tổng 3 400 ns) — của hệ thống phần mềm **không** kernel bypass, nên nút thắt ở đó là lời gọi hệ thống gửi lệnh (44%) chứ không phải card mạng. Hai bảng minh hoạ cùng một bài học: tìm chặng đắt nhất trước, rồi mới tối ưu.

### 5. Bố cục cấu trúc phải là quyết định có chủ đích

Chương này định nghĩa `OrderPacket` chiếm **đúng 64 byte** — một dòng cache. Không phải ngẫu nhiên: một lệnh đọc lên là đúng một lần nạp cache. Lưu ý: 64 byte chỉ bảo đảm *kích thước*; để không phần tử nào vắt qua hai dòng, mảng còn phải bắt đầu ở địa chỉ chia hết cho 64 — muốn chắc chắn thì thêm `#[repr(align(64))]`.

Một lưu ý Rust: mảng chỉ cài `Default` tới kích thước 32 (vẫn đúng ở Rust 1.97). Với `[u8; 64]` bạn phải tự cài `Default`, hoặc — như chương này làm — thiết kế một struct có ý nghĩa với phần dự trữ `[u8; 32]` thay vì mảng byte trần.

---

## Mã nguồn minh họa thực chiến

Chạy bằng `cargo run -p ch74`, kiểm thử bằng `cargo test -p ch74`.

```rust
#![allow(dead_code)]
//! Chương 74 — Kỹ nghệ độ trễ thấp: đo phân vị thay vì trung bình, vòng đệm
//! không khoá kiểu Disruptor, chia sẻ giả, bố trí bộ nhớ, và đường nóng không cấp phát.
//!
//! Đây là nền móng của mọi hệ thống HFT. Triết lý giống hệt cách Jane Street
//! làm với OCaml: đẩy mọi thứ có thể ra khỏi đường nóng, và ĐO thay vì đoán.

use std::cell::UnsafeCell;
use std::sync::atomic::{AtomicUsize, Ordering};

// ============================================================================
// 1. ĐO ĐỘ TRỄ — vì sao trung bình là con số vô dụng
// ============================================================================

/// Biểu đồ tần suất kiểu HDR (rút gọn): chia thang log thành các "xô" luỹ
/// thừa của 2 để giữ độ chính xác tương đối ở mọi bậc độ lớn, mà chỉ tốn vài
/// trăm byte. Bản rút gọn này sai số tới 2× trong một xô; HDR thật chia mỗi
/// luỹ thừa của 2 thành nhiều xô con để sai số chỉ còn vài phần nghìn.
///
/// Ghi một mẫu là O(1) và KHÔNG cấp phát — bắt buộc, vì bản thân việc đo
/// không được làm nhiễu thứ đang đo.
pub struct LatencyHistogram {
    /// buckets[0] đếm giá trị 0; buckets[i] (i ≥ 1) đếm các giá trị trong [2^(i-1), 2^i)
    buckets: Vec<u64>,
    pub count: u64,
    pub min: u64,
    pub max: u64,
    sum: u128,
}

impl Default for LatencyHistogram {
    fn default() -> Self {
        Self::new()
    }
}

impl LatencyHistogram {
    pub fn new() -> Self {
        LatencyHistogram {
            buckets: vec![0; 65],
            count: 0,
            min: u64::MAX,
            max: 0,
            sum: 0,
        }
    }

    #[inline]
    pub fn record(&mut self, ns: u64) {
        let i = if ns == 0 {
            0
        } else {
            64 - ns.leading_zeros() as usize
        };
        self.buckets[i] += 1;
        self.count += 1;
        self.sum += ns as u128;
        if ns < self.min {
            self.min = ns;
        }
        if ns > self.max {
            self.max = ns;
        }
    }

    pub fn mean(&self) -> f64 {
        if self.count == 0 {
            0.0
        } else {
            self.sum as f64 / self.count as f64
        }
    }

    /// Cận TRÊN của xô chứa phân vị (nhưng không vượt `max` đã thấy). Với
    /// thang log, sai số tương đối bị chặn trong mỗi xô — đủ tốt để phát hiện
    /// đuôi dài, vốn là mục đích chính.
    pub fn percentile(&self, p: f64) -> u64 {
        if self.count == 0 {
            return 0;
        }
        let threshold = (self.count as f64 * p).ceil().max(1.0) as u64;
        let mut accumulate = 0u64;
        for (i, &c) in self.buckets.iter().enumerate() {
            accumulate += c;
            if accumulate >= threshold {
                // Cận trên của xô i là 2^i − 1. Viết `(1 << (i-1)) * 2 - 1` sẽ
                // tràn số khi i = 64 (mẫu ≥ 2^63) — dịch phải từ u64::MAX thì không.
                let upper = if i == 0 { 0 } else { u64::MAX >> (64 - i) };
                return upper.min(self.max);
            }
        }
        self.max
    }

    /// Bản tóm tắt mà một kỹ sư độ trễ thật sự nhìn vào.
    pub fn summary(&self) -> String {
        format!(
            "n={} min={} p50={} p99={} p99.9={} max={} (tb={:.0})",
            self.count,
            self.min,
            self.percentile(0.50),
            self.percentile(0.99),
            self.percentile(0.999),
            self.max,
            self.mean()
        )
    }
}

// ============================================================================
// 2. CHIA SẺ GIẢ — hai biến cạnh nhau giết chết hiệu năng đa luồng
// ============================================================================

pub const CACHE_LINE: usize = 64;

/// Hai bộ đếm nằm CÙNG một dòng cache. Hai lõi ghi vào hai biến khác nhau,
/// nhưng phần cứng chỉ biết tới dòng cache — nên chúng giành nhau quyền sở
/// hữu dòng đó, ping-pong qua lại. Chậm đi nhiều lần mà nhìn mã không thấy.
#[repr(C)]
pub struct SameLineCounters {
    pub a: AtomicUsize,
    pub b: AtomicUsize,
}

/// Đệm cho mỗi bộ đếm chiếm trọn một dòng cache riêng.
#[repr(C, align(64))]
pub struct PaddedCounter {
    pub value: AtomicUsize,
    _pad: [u8; CACHE_LINE - 8],
}

impl Default for PaddedCounter {
    fn default() -> Self {
        Self::new()
    }
}

impl PaddedCounter {
    pub fn new() -> Self {
        PaddedCounter {
            value: AtomicUsize::new(0),
            _pad: [0; CACHE_LINE - 8],
        }
    }
}

#[repr(C)]
pub struct SplitLineCounters {
    pub a: PaddedCounter,
    pub b: PaddedCounter,
}

// ============================================================================
// 3. VÒNG ĐỆM KHÔNG KHOÁ KIỂU DISRUPTOR
// ============================================================================

/// Một-ghi-một-đọc, không khoá, không cấp phát, sức chứa là luỹ thừa của 2.
///
/// Ba quyết định thiết kế đáng chú ý:
/// 1. Sức chứa 2^n → thay `%` (phép chia, ~20–40 chu kỳ) bằng `&` (1 chu kỳ).
/// 2. Con trỏ đọc/ghi nằm ở hai dòng cache RIÊNG → không chia sẻ giả.
/// 3. Con trỏ TĂNG MÃI, không quấn vòng → phân biệt được "rỗng" và "đầy"
///    mà không phải hy sinh một ô như hàng đợi vòng thông thường.
#[repr(C, align(64))]
pub struct DisruptorRing<T, const N: usize> {
    slots: UnsafeCell<[Option<T>; N]>,
    _pad1: [u8; CACHE_LINE],
    write_pos: AtomicUsize,
    _pad2: [u8; CACHE_LINE - 8],
    read_pos: AtomicUsize,
    _pad3: [u8; CACHE_LINE - 8],
}

// An toàn: mỗi con trỏ chỉ có ĐÚNG MỘT bên ghi vào.
unsafe impl<T: Send, const N: usize> Sync for DisruptorRing<T, N> {}
unsafe impl<T: Send, const N: usize> Send for DisruptorRing<T, N> {}

impl<T, const N: usize> Default for DisruptorRing<T, N> {
    fn default() -> Self {
        Self::new()
    }
}

impl<T, const N: usize> DisruptorRing<T, N> {
    pub fn new() -> Self {
        assert!(N.is_power_of_two(), "sức chứa phải là luỹ thừa của 2");
        DisruptorRing {
            slots: UnsafeCell::new(std::array::from_fn(|_| None)),
            _pad1: [0; CACHE_LINE],
            write_pos: AtomicUsize::new(0),
            _pad2: [0; CACHE_LINE - 8],
            read_pos: AtomicUsize::new(0),
            _pad3: [0; CACHE_LINE - 8],
        }
    }

    #[inline]
    fn slot(v: usize) -> usize {
        v & (N - 1)
    } // thay cho v % N

    /// Số phần tử đang chờ. Đọc `write_pos` TRƯỚC rồi mới đọc `read_pos`;
    /// nếu gọi từ luồng thứ ba, `read_pos` có thể đã vượt ảnh chụp cũ của
    /// `write_pos`, nên dùng `saturating_sub` thay vì để phép trừ tràn.
    pub fn len(&self) -> usize {
        let write = self.write_pos.load(Ordering::Acquire);
        let read = self.read_pos.load(Ordering::Acquire);
        write.saturating_sub(read)
    }
    pub fn is_empty(&self) -> bool {
        self.len() == 0
    }
    pub fn is_full(&self) -> bool {
        self.len() == N
    }
    pub fn capacity(&self) -> usize {
        N
    }

    /// Gọi từ luồng SẢN XUẤT. Trả `Err` khi đầy — không bao giờ chặn,
    /// vì chặn trên đường nóng là điều cấm kỵ.
    pub fn push(&self, value: T) -> Result<(), T> {
        let write = self.write_pos.load(Ordering::Relaxed); // ta là bên duy nhất ghi nó
        let read = self.read_pos.load(Ordering::Acquire);
        if write - read == N {
            return Err(value);
        }
        unsafe {
            (*self.slots.get())[Self::slot(write)] = Some(value);
        }
        // Release: bảo đảm dữ liệu ghi xong TRƯỚC khi bên đọc thấy con trỏ mới
        self.write_pos.store(write + 1, Ordering::Release);
        Ok(())
    }

    /// Gọi từ luồng TIÊU THỤ.
    pub fn take(&self) -> Option<T> {
        let read = self.read_pos.load(Ordering::Relaxed);
        let write = self.write_pos.load(Ordering::Acquire);
        if read == write {
            return None;
        }
        let value = unsafe { (*self.slots.get())[Self::slot(read)].take() };
        self.read_pos.store(read + 1, Ordering::Release);
        value
    }

    /// Lấy cả LÔ — mấu chốt của thông lượng cao: một lần đồng bộ cho nhiều
    /// phần tử, nên chi phí hàng rào bộ nhớ được chia đều cho cả lô.
    pub fn take_batch(&self, max: usize, out: &mut Vec<T>) -> usize {
        let read = self.read_pos.load(Ordering::Relaxed);
        let write = self.write_pos.load(Ordering::Acquire);
        let n = (write - read).min(max);
        for i in 0..n {
            if let Some(x) = unsafe { (*self.slots.get())[Self::slot(read + i)].take() } {
                out.push(x);
            }
        }
        if n > 0 {
            self.read_pos.store(read + n, Ordering::Release);
        }
        n
    }
}

// ============================================================================
// 4. BỂ ĐỐI TƯỢNG — đường nóng không được cấp phát
// ============================================================================
// Một lần cấp phát heap tốn 50–200 ns và có ĐUÔI DÀI không đoán trước: nó có
// thể gọi xuống hệ điều hành xin thêm trang nhớ. Trên đường nóng, ta cấp phát
// TRƯỚC toàn bộ rồi tái sử dụng.

/// Bản ghi lệnh cấp phát sẵn — thứ ta thật sự tái sử dụng trên đường nóng.
/// Cỡ vừa đúng một dòng cache để mỗi lần chạm chỉ tốn một lần nạp.
#[derive(Clone, Default, PartialEq, Debug)]
pub struct OrderPacket {
    pub order_id: u64,
    pub price: i64,
    pub quantity: i64,
    pub symbol_id: u32,
    pub side: u8,
    pub _reserved: [u8; 32], // 61 byte dữ liệu + 3 byte đệm = 64
}

pub struct ObjectPool<T> {
    free: Vec<usize>,
    slots: Vec<T>,
    pub acquire_count: u64,
    pub exhausted_count: u64,
}

impl<T: Default + Clone> ObjectPool<T> {
    pub fn new(capacity: usize) -> Self {
        ObjectPool {
            free: (0..capacity).rev().collect(),
            slots: vec![T::default(); capacity],
            acquire_count: 0,
            exhausted_count: 0,
        }
    }
    pub fn available(&self) -> usize {
        self.free.len()
    }

    /// Trả về CHỈ SỐ chứ không phải con trỏ — tránh hẳn vấn đề vòng đời.
    pub fn acquire(&mut self) -> Option<usize> {
        self.acquire_count += 1;
        match self.free.pop() {
            Some(i) => Some(i),
            None => {
                self.exhausted_count += 1;
                None
            }
        }
    }
    pub fn release(&mut self, i: usize) {
        self.free.push(i);
    }
    pub fn get(&self, i: usize) -> &T {
        &self.slots[i]
    }
    pub fn get_mut(&mut self, i: usize) -> &mut T {
        &mut self.slots[i]
    }
}

// ============================================================================
// 5. BỐ TRÍ BỘ NHỚ — mảng-của-struct vs struct-của-mảng
// ============================================================================

/// Mảng-của-struct (AoS): mỗi bản ghi liền mạch. Tốt khi đọc TẤT CẢ trường.
/// Trường được xếp theo kích thước GIẢM DẦN để trình biên dịch không phải đệm.
#[derive(Clone, Copy, Default)]
pub struct QuoteAoS {
    pub bid_price: i64,
    pub ask_price: i64,
    pub timestamp: u64,
    pub flags: u64,
    pub symbol_id: u32,
    pub bid_qty: u32,
    pub ask_qty: u32,
    pub _reserved: u32,
}

/// Struct-của-mảng (SoA): mỗi trường một mảng riêng. Tốt khi chỉ đọc MỘT
/// trường trên nhiều bản ghi — CPU nạp một dòng cache 64 byte là được 8 giá
/// trị đều có ích, thay vì 8 byte có ích trong mỗi bản ghi 48 byte.
#[derive(Default)]
pub struct QuoteTableSoA {
    pub symbol_id: Vec<u32>,
    pub bid_price: Vec<i64>,
    pub ask_price: Vec<i64>,
    pub bid_qty: Vec<u32>,
    pub ask_qty: Vec<u32>,
    pub timestamp: Vec<u64>,
}

impl QuoteTableSoA {
    pub fn new(n: usize) -> Self {
        QuoteTableSoA {
            symbol_id: vec![0; n],
            bid_price: vec![0; n],
            ask_price: vec![0; n],
            bid_qty: vec![0; n],
            ask_qty: vec![0; n],
            timestamp: vec![0; n],
        }
    }
    pub fn len(&self) -> usize {
        self.symbol_id.len()
    }
    pub fn is_empty(&self) -> bool {
        self.symbol_id.is_empty()
    }

    /// Quét chỉ trường `bid_price` — đây là chỗ SoA thắng đậm.
    pub fn total_bid_price(&self) -> i128 {
        self.bid_price.iter().map(|&x| x as i128).sum()
    }

    /// Số byte thực sự phải kéo từ RAM để quét một trường 8 byte.
    pub fn bytes_to_read_one_field(&self) -> usize {
        self.len() * 8
    }
}

pub fn bytes_to_read_one_field_aos(n: usize) -> usize {
    // Phải kéo cả bản ghi dù chỉ cần 8 byte
    n * std::mem::size_of::<QuoteAoS>()
}

// ============================================================================
// 6. NGÂN SÁCH ĐỘ TRỄ — chia nhỏ "tick-to-trade"
// ============================================================================

#[derive(Debug, Clone, PartialEq)]
pub struct LatencyStage {
    pub name: String,
    pub ns: u64,
}

#[derive(Debug, PartialEq)]
pub struct LatencyBudget {
    pub stages: Vec<LatencyStage>,
    pub budget_ns: u64,
}

impl LatencyBudget {
    pub fn total(&self) -> u64 {
        self.stages.iter().map(|c| c.ns).sum()
    }
    pub fn within_budget(&self) -> bool {
        self.total() <= self.budget_ns
    }
    /// Chặng tốn nhất — nơi DUY NHẤT đáng bỏ công tối ưu.
    pub fn bottleneck(&self) -> Option<&LatencyStage> {
        self.stages.iter().max_by_key(|c| c.ns)
    }
    /// Định luật Amdahl: tăng tốc tối đa nếu chặng nghẽn cổ chai thành 0.
    pub fn max_speedup_if_bottleneck_removed(&self) -> f64 {
        match self.bottleneck() {
            Some(n) if self.total() > n.ns => self.total() as f64 / (self.total() - n.ns) as f64,
            _ => f64::INFINITY,
        }
    }
}

/// Sinh mẫu độ trễ tất định có ĐUÔI DÀI — giống hệt hệ thống thật:
/// phần lớn nhanh, thỉnh thoảng một cú chậm gấp hàng trăm lần.
pub fn gen_latency_samples(n: usize, seed: u64) -> Vec<u64> {
    let mut s = seed;
    (0..n)
        .map(|_| {
            s = s
                .wrapping_mul(6364136223846793005)
                .wrapping_add(1442695040888963407);
            let r = (s >> 33) % 10_000;
            match r {
                0..=9_899 => 200 + r % 100,       // 99%  : 200–300 ns
                9_900..=9_989 => 2_000 + r % 500, // 0.9% : ~2 µs (trượt cache)
                _ => 50_000 + r % 10_000,         // 0.1% : ~50 µs (hệ điều hành xen vào)
            }
        })
        .collect()
}

fn main() {
    println!("═══════════════════════════════════════════════════════════");
    println!("   KỸ NGHỆ ĐỘ TRỄ THẤP: ĐO · KHÔNG KHOÁ · KHÔNG CẤP PHÁT   ");
    println!("═══════════════════════════════════════════════════════════");

    println!("\n1. VÌ SAO TRUNG BÌNH LÀ CON SỐ VÔ DỤNG");
    let mut hist = LatencyHistogram::new();
    for x in gen_latency_samples(1_000_000, 42) {
        hist.record(x);
    }
    println!("   {}", hist.summary());
    println!("   Trung bình {:.0} ns nghe rất đẹp…", hist.mean());
    println!(
        "   …nhưng 1 trên 1000 lệnh rơi vào dải tới {} ns, và cú chậm nhất là {} ns",
        hist.percentile(0.999),
        hist.max
    );
    println!(
        "   — gấp {:.0} lần trung bình. (Phân vị là cận TRÊN của xô log.)",
        hist.max as f64 / hist.mean()
    );
    println!("   Trong giao dịch, chính CÁI ĐUÔI đó là lúc bạn mất tiền.");

    println!("\n2. CHIA SẺ GIẢ — kích thước quyết định tốc độ");
    println!(
        "   SameLineCounters : {} byte (hai bộ đếm CÙNG một dòng cache)",
        std::mem::size_of::<SameLineCounters>()
    );
    println!(
        "   SplitLineCounters: {} byte (mỗi bộ đếm một dòng riêng)",
        std::mem::size_of::<SplitLineCounters>()
    );
    println!(
        "   → Tốn thêm {} byte để tránh ping-pong dòng cache giữa hai lõi.",
        std::mem::size_of::<SplitLineCounters>() - std::mem::size_of::<SameLineCounters>()
    );

    println!("\n3. VÒNG ĐỆM DISRUPTOR");
    let v: DisruptorRing<u64, 1024> = DisruptorRing::new();
    for i in 0..1024 {
        v.push(i).unwrap();
    }
    println!(
        "   Đẩy 1024 phần tử → đầy: {} · đẩy thêm → bị từ chối: {}",
        v.is_full(),
        v.push(9999).is_err()
    );
    let mut batch = Vec::new();
    let n = v.take_batch(256, &mut batch);
    println!(
        "   Lấy một lô 256 → được {} phần tử, còn lại {}",
        n,
        v.len()
    );
    println!(
        "   Chỉ số dùng phép AND: 1030 & 1023 = {} (thay cho phép chia)",
        1030usize & 1023
    );

    println!("\n4. BỂ ĐỐI TƯỢNG");
    let mut pool: ObjectPool<OrderPacket> = ObjectPool::new(4);
    let slots: Vec<usize> = (0..4).filter_map(|_| pool.acquire()).collect();
    println!(
        "   Mượn 4/4 → còn rảnh {} · mượn thêm → {:?}",
        pool.available(),
        pool.acquire()
    );
    pool.release(slots[0]);
    println!(
        "   Trả 1 lại → còn rảnh {} · số lần hết bể = {}",
        pool.available(),
        pool.exhausted_count
    );
    println!(
        "   Một OrderPacket = {} byte — vừa đúng một dòng cache",
        std::mem::size_of::<OrderPacket>()
    );

    println!("\n5. BỐ TRÍ BỘ NHỚ — AoS vs SoA khi quét MỘT trường");
    let n = 100_000;
    println!(
        "   Một bản ghi AoS = {} byte",
        std::mem::size_of::<QuoteAoS>()
    );
    println!("   Quét {} bản ghi chỉ để lấy `bid_price`:", n);
    println!(
        "     AoS phải kéo {:>9} byte từ RAM",
        bytes_to_read_one_field_aos(n)
    );
    println!(
        "     SoA chỉ kéo  {:>9} byte",
        QuoteTableSoA::new(n).bytes_to_read_one_field()
    );
    println!(
        "   → SoA đọc ít hơn {:.1}× — và đó là băng thông RAM, thứ đắt nhất.",
        bytes_to_read_one_field_aos(n) as f64 / (n * 8) as f64
    );

    println!("\n6. NGÂN SÁCH ĐỘ TRỄ TICK-TO-TRADE");
    let ns = LatencyBudget {
        budget_ns: 5_000,
        stages: vec![
            LatencyStage {
                name: "Card mạng → bộ nhớ".into(),
                ns: 800,
            },
            LatencyStage {
                name: "Phân tích gói tin".into(),
                ns: 150,
            },
            LatencyStage {
                name: "Cập nhật sổ lệnh".into(),
                ns: 400,
            },
            LatencyStage {
                name: "Chiến lược quyết định".into(),
                ns: 250,
            },
            LatencyStage {
                name: "Kiểm tra rủi ro".into(),
                ns: 120,
            },
            LatencyStage {
                name: "Tuần tự hoá lệnh".into(),
                ns: 180,
            },
            LatencyStage {
                name: "Gọi hệ thống gửi".into(),
                ns: 1_500,
            },
        ],
    };
    for c in &ns.stages {
        let percent = c.ns as f64 * 100.0 / ns.total() as f64;
        println!(
            "   {:<26} {:>5} ns  {:>5.1}%  {}",
            c.name,
            c.ns,
            percent,
            "#".repeat((percent / 2.0) as usize)
        );
    }
    println!(
        "   Tổng {} ns / trần {} ns → {}",
        ns.total(),
        ns.budget_ns,
        if ns.within_budget() {
            "ĐẠT"
        } else {
            "TRƯỢT"
        }
    );
    println!(
        "   Nút thắt: {} · xoá hẳn nó cũng chỉ nhanh được {:.2}×",
        ns.bottleneck().unwrap().name,
        ns.max_speedup_if_bottleneck_removed()
    );
    println!("   → Đó là lý do HFT thật dùng kernel bypass: gọi hệ thống là chặng đắt nhất.");

    println!("\n═══════════════════════════════════════════════════════════");
    println!("   ĐO PHÂN VỊ, ĐỪNG ĐO TRUNG BÌNH. TỐI ƯU NÚT, ĐỪNG TỐI ƯU BỪA");
    println!("═══════════════════════════════════════════════════════════");
}

#[cfg(test)]
mod tests {
    use super::*;

    // ---------- Biểu đồ độ trễ ----------
    #[test]
    fn empty_histogram_does_not_panic() {
        let b = LatencyHistogram::new();
        assert_eq!(b.count, 0);
        assert_eq!(b.mean(), 0.0);
        assert_eq!(b.percentile(0.99), 0);
    }

    #[test]
    fn histogram_tracks_min_max_and_mean() {
        let mut b = LatencyHistogram::new();
        for x in [10u64, 20, 30, 40] {
            b.record(x);
        }
        assert_eq!(b.min, 10);
        assert_eq!(b.max, 40);
        assert_eq!(b.mean(), 25.0);
        assert_eq!(b.count, 4);
    }

    #[test]
    fn percentiles_are_monotonic() {
        let mut b = LatencyHistogram::new();
        for x in gen_latency_samples(10_000, 7) {
            b.record(x);
        }
        let (p50, p90, p99, p999) = (
            b.percentile(0.5),
            b.percentile(0.9),
            b.percentile(0.99),
            b.percentile(0.999),
        );
        assert!(
            p50 <= p90 && p90 <= p99 && p99 <= p999,
            "phân vị phải tăng dần: {} {} {} {}",
            p50,
            p90,
            p99,
            p999
        );
        assert!(p999 <= b.max);
    }

    #[test]
    fn percentiles_bracket_the_true_value() {
        // Cận trên của xô phải THỰC SỰ là cận trên: không được báo thấp hơn
        // giá trị thật, nếu không ta sẽ tưởng hệ thống nhanh hơn thực tế.
        let mut b = LatencyHistogram::new();
        for x in [1u64, 2, 3, 100, 1000] {
            b.record(x);
        }
        assert!(b.percentile(1.0) >= 1000);
        assert!(b.percentile(0.8) >= 100, "80% mẫu ≤ 100, cận phải ≥ 100");
    }

    #[test]
    fn a_long_tail_makes_the_mean_lie() {
        // Đây là bài học trung tâm của chương: 99% mẫu ở 200–300 ns, nhưng
        // 0.1% ở 50 µs kéo trung bình lên và che mất phân bố thật.
        let mut b = LatencyHistogram::new();
        for x in gen_latency_samples(100_000, 42) {
            b.record(x);
        }

        // Phân bố thật: p50 ≈ 250 ns, p99 ≈ 299 ns, p99.9 ≈ 2.5 µs, max ≈ 60 µs.
        // Chú ý p99 vẫn NHANH — phải soi tới p99.9 mới thấy dấu vết đuôi,
        // và tới giá trị lớn nhất mới thấy hết mức độ.
        assert!(b.percentile(0.5) < 512, "phân vị 50 phải nằm ở vùng nhanh");
        assert!(
            b.percentile(0.99) < 512,
            "ngay cả p99 vẫn nhanh — đuôi còn ẩn kỹ hơn thế"
        );
        assert!(
            b.percentile(0.999) > 2_000,
            "tới p99.9 mới lộ ra đuôi, thực tế {}",
            b.percentile(0.999)
        );
        assert!(b.max > 50_000, "giá trị lớn nhất mới cho thấy hết mức độ");

        // Đây là con số đắt giá nhất: trung bình ~326 ns che mất một cú
        // gần 60 µs, tức chậm gấp gần 200 lần.
        assert!(
            b.max as f64 > b.mean() * 100.0,
            "max {} so với trung bình {:.0} — trung bình che giấu đúng thứ giết bạn",
            b.max,
            b.mean()
        );
    }

    #[test]
    fn recording_zero_and_max_is_safe() {
        let mut b = LatencyHistogram::new();
        b.record(0);
        b.record(u64::MAX);
        assert_eq!(b.count, 2);
        assert_eq!(b.min, 0);
        assert_eq!(b.max, u64::MAX);
        // Trước đây `(1 << 63) * 2` tràn số ở đây và làm panic bản debug.
        assert_eq!(b.percentile(1.0), u64::MAX);
        assert_eq!(b.percentile(0.5), 0);
    }

    #[test]
    fn percentile_never_exceeds_observed_max() {
        // Cận trên của xô [32768, 65535] là 65535, nhưng nếu mẫu lớn nhất chỉ
        // là 50 000 thì báo 65 535 là bịa ra một con số chưa từng xảy ra.
        let mut b = LatencyHistogram::new();
        for x in [100u64, 200, 50_000] {
            b.record(x);
        }
        assert_eq!(b.percentile(1.0), 50_000);
        assert!(b.percentile(0.999) <= b.max);
    }

    // ---------- Chia sẻ giả ----------
    #[test]
    fn padded_counter_owns_a_whole_cache_line() {
        assert_eq!(std::mem::size_of::<PaddedCounter>(), CACHE_LINE);
        assert_eq!(
            std::mem::align_of::<PaddedCounter>(),
            CACHE_LINE,
            "phải căn theo dòng cache, không chỉ đủ kích thước"
        );
    }

    #[test]
    fn padded_counters_never_share_a_line() {
        let b = SplitLineCounters {
            a: PaddedCounter::new(),
            b: PaddedCounter::new(),
        };
        let addr_a = &b.a as *const _ as usize;
        let addr_b = &b.b as *const _ as usize;
        assert!(
            addr_b - addr_a >= CACHE_LINE,
            "hai bộ đếm cách nhau {} byte, phải ít nhất {}",
            addr_b - addr_a,
            CACHE_LINE
        );
        // Ngược lại, phiên bản không đệm thì chúng nằm sát nhau
        let c = SameLineCounters {
            a: AtomicUsize::new(0),
            b: AtomicUsize::new(0),
        };
        let ca = &c.a as *const _ as usize;
        let cb = &c.b as *const _ as usize;
        assert!(
            cb - ca < CACHE_LINE,
            "đây chính là chia sẻ giả: cách nhau chỉ {} byte",
            cb - ca
        );
    }

    // ---------- Vòng Disruptor ----------
    #[test]
    fn ring_is_fifo() {
        let v: DisruptorRing<u32, 8> = DisruptorRing::new();
        for i in 0..5 {
            v.push(i).unwrap();
        }
        for i in 0..5 {
            assert_eq!(v.take(), Some(i));
        }
        assert_eq!(v.take(), None);
    }

    #[test]
    fn ring_uses_full_capacity_without_wasting_a_slot() {
        // Hàng đợi vòng thường phải bỏ một ô để phân biệt rỗng/đầy.
        // Con trỏ tăng mãi giúp ta dùng trọn N ô.
        let v: DisruptorRing<u32, 8> = DisruptorRing::new();
        for i in 0..8 {
            assert!(v.push(i).is_ok(), "phải nhận đủ 8 phần tử");
        }
        assert!(v.is_full());
        assert_eq!(v.len(), 8);
        assert_eq!(v.push(99), Err(99));
    }

    #[test]
    fn ring_wraps_correctly_over_many_laps() {
        let v: DisruptorRing<u64, 4> = DisruptorRing::new();
        for i in 0..1000u64 {
            v.push(i).unwrap();
            assert_eq!(v.take(), Some(i), "chỉ số phải quấn đúng qua biên mảng");
        }
        assert!(v.is_empty());
    }

    #[test]
    fn empty_ring_returns_none_safely() {
        let v: DisruptorRing<u8, 16> = DisruptorRing::new();
        assert_eq!(v.take(), None);
        assert!(v.is_empty() && !v.is_full());
        assert_eq!(v.len(), 0);
    }

    #[test]
    fn batch_take_returns_the_requested_count() {
        let v: DisruptorRing<u32, 64> = DisruptorRing::new();
        for i in 0..50 {
            v.push(i).unwrap();
        }
        let mut out = Vec::new();
        assert_eq!(v.take_batch(20, &mut out), 20);
        assert_eq!(out, (0..20).collect::<Vec<u32>>());
        assert_eq!(v.len(), 30);
        // Xin nhiều hơn số có thì chỉ lấy được số có
        let mut out2 = Vec::new();
        assert_eq!(v.take_batch(1000, &mut out2), 30);
        assert!(v.is_empty());
    }

    #[test]
    fn batch_take_on_empty_ring_returns_zero() {
        let v: DisruptorRing<u32, 8> = DisruptorRing::new();
        let mut out = Vec::new();
        assert_eq!(v.take_batch(10, &mut out), 0);
        assert!(out.is_empty());
    }

    #[test]
    fn and_replaces_modulo_for_power_of_two_capacity() {
        for n in [8usize, 16, 64, 1024, 4096] {
            for v in [0usize, 1, 7, 1030, 99999] {
                assert_eq!(v & (n - 1), v % n, "AND phải cho cùng kết quả với MOD");
            }
        }
    }

    #[test]
    #[should_panic(expected = "luỹ thừa của 2")]
    fn non_power_of_two_capacity_is_rejected() {
        let _: DisruptorRing<u8, 100> = DisruptorRing::new();
    }

    // ---------- Bể đối tượng ----------
    #[test]
    fn pool_recycles_objects() {
        let mut b: ObjectPool<u64> = ObjectPool::new(3);
        assert_eq!(b.available(), 3);
        let a = b.acquire().unwrap();
        let c = b.acquire().unwrap();
        assert_ne!(a, c, "hai lần mượn phải ra hai ô khác nhau");
        assert_eq!(b.available(), 1);
        b.release(a);
        assert_eq!(b.available(), 2);
    }

    #[test]
    fn exhausted_pool_returns_none_instead_of_allocating() {
        // Điểm mấu chốt: thà từ chối còn hơn cấp phát heap trên đường nóng.
        let mut b: ObjectPool<u32> = ObjectPool::new(2);
        assert!(b.acquire().is_some());
        assert!(b.acquire().is_some());
        assert!(b.acquire().is_none());
        assert_eq!(
            b.exhausted_count, 1,
            "phải ĐẾM số lần hết bể để còn chỉnh kích thước"
        );
        assert_eq!(b.acquire_count, 3);
    }

    #[test]
    fn order_packet_is_exactly_one_cache_line() {
        assert_eq!(
            std::mem::size_of::<OrderPacket>(),
            CACHE_LINE,
            "bản ghi trên đường nóng nên vừa một dòng cache, không hơn"
        );
    }

    #[test]
    fn a_returned_slot_is_reusable_immediately() {
        let mut b: ObjectPool<u64> = ObjectPool::new(2);
        let i = b.acquire().unwrap();
        *b.get_mut(i) = 12345;
        assert_eq!(*b.get(i), 12345);
        b.release(i);
        let j = b.acquire().unwrap();
        assert_eq!(
            i, j,
            "ô vừa trả phải được tái dùng ngay — nó còn NÓNG trong cache"
        );
    }

    // ---------- Bố trí bộ nhớ ----------
    #[test]
    fn soa_reads_far_fewer_bytes_when_scanning_one_field() {
        let n = 10_000;
        let aos = bytes_to_read_one_field_aos(n);
        let soa = QuoteTableSoA::new(n).bytes_to_read_one_field();
        assert!(aos > soa * 4, "AoS đọc {} byte, SoA chỉ {} byte", aos, soa);
    }

    #[test]
    fn soa_computes_the_correct_sum() {
        let mut t = QuoteTableSoA::new(5);
        for i in 0..5 {
            t.bid_price[i] = (i as i64 + 1) * 100;
        }
        assert_eq!(t.total_bid_price(), 100 + 200 + 300 + 400 + 500);
    }

    #[test]
    fn aos_quote_has_no_surprise_padding() {
        // Nếu kích thước lệch so với tổng các trường thì có đệm ẩn — điều
        // cần biết khi tính băng thông bộ nhớ. Xếp trường theo kích thước
        // giảm dần là cách đơn giản nhất để tránh đệm.
        let total_fields = 8 + 8 + 8 + 8 + 4 + 4 + 4 + 4;
        assert_eq!(std::mem::size_of::<QuoteAoS>(), total_fields);
    }

    // ---------- Ngân sách độ trễ ----------
    fn sample_budget() -> LatencyBudget {
        LatencyBudget {
            budget_ns: 5_000,
            stages: vec![
                LatencyStage {
                    name: "network".into(),
                    ns: 800,
                },
                LatencyStage {
                    name: "parse".into(),
                    ns: 150,
                },
                LatencyStage {
                    name: "syscall".into(),
                    ns: 1_500,
                },
            ],
        }
    }

    #[test]
    fn budget_computes_total_and_bottleneck() {
        let ns = sample_budget();
        assert_eq!(ns.total(), 2_450);
        assert!(ns.within_budget());
        assert_eq!(ns.bottleneck().unwrap().name, "syscall");
    }

    #[test]
    fn amdahl_bounds_the_speedup() {
        let ns = sample_budget();
        // Xoá hẳn chặng 1500 ns khỏi tổng 2450 ns → còn 950 ns
        let expected = 2_450.0 / 950.0;
        assert!((ns.max_speedup_if_bottleneck_removed() - expected).abs() < 1e-9);
        assert!(
            ns.max_speedup_if_bottleneck_removed() < 3.0,
            "kể cả xoá sạch nút thắt cũng chỉ nhanh được ~2.6× — đó là định luật Amdahl"
        );
    }

    #[test]
    fn over_budget_is_reported_as_a_miss() {
        let ns = LatencyBudget {
            budget_ns: 1_000,
            stages: vec![LatencyStage {
                name: "slow".into(),
                ns: 9_999,
            }],
        };
        assert!(!ns.within_budget());
    }

    #[test]
    fn a_single_stage_budget_allows_unbounded_speedup() {
        let ns = LatencyBudget {
            budget_ns: 100,
            stages: vec![LatencyStage {
                name: "everything".into(),
                ns: 500,
            }],
        };
        assert!(
            ns.max_speedup_if_bottleneck_removed().is_infinite(),
            "xoá chặng duy nhất thì thời gian còn 0"
        );
    }

    // ---------- Sinh mẫu ----------
    #[test]
    fn sample_generation_is_deterministic() {
        assert_eq!(gen_latency_samples(100, 5), gen_latency_samples(100, 5));
        assert_ne!(gen_latency_samples(100, 5), gen_latency_samples(100, 6));
    }

    #[test]
    fn samples_span_exactly_three_latency_bands() {
        let m = gen_latency_samples(100_000, 1);
        let fast = m.iter().filter(|&&x| x < 1_000).count();
        let medium = m.iter().filter(|&&x| (1_000..10_000).contains(&x)).count();
        let slow = m.iter().filter(|&&x| x >= 10_000).count();
        assert!(fast > 95_000, "~99% phải nhanh, thực tế {}", fast);
        assert!(medium > 0 && slow > 0, "phải có cả đuôi vừa và đuôi dài");
        assert_eq!(fast + medium + slow, m.len());
    }
}
```

---

## Bảng tra cứu lỗi biên dịch thường gặp

| Lỗi | Nguyên nhân trong chương này | Cách sửa |
|---|---|---|
| `E0277: [u8; 64]: Default is not satisfied` | Mảng chỉ cài `Default` tới 32 phần tử | Tự cài `Default`, hoặc thiết kế struct có nghĩa |
| `E0507: cannot move out of index` | Lấy `T` ra khỏi `Vec` trong vòng | `Option<T>` + `.take()`, hoặc `std::mem::replace` |
| `E0596: cannot borrow data in an Arc as mutable` | Viết `push(&mut self)` rồi chia sẻ vòng giữa hai luồng qua `Arc` | Dùng `&self` + `UnsafeCell` + con trỏ nguyên tử như `DisruptorRing` |
| `attempt to multiply with overflow` (panic lúc chạy, bản debug) | Tính cận trên xô bằng `(1u64 << (i - 1)) * 2 - 1` khi i = 64 | `u64::MAX >> (64 - i)` — chính là cách `percentile` làm |
| Đo được 0 ns | Trình tối ưu xoá vòng lặp trống | `std::hint::black_box` để giữ lại phép tính |

---

## Tóm tắt chương & Bài tập rèn luyện

### 5 điểm cốt lõi

1. **Đo phân vị, không đo trung bình.** Cái đuôi mới là thứ giết chiến lược, và nó xuất hiện đúng lúc thị trường biến động.
2. **Không cấp phát trên đường nóng.** Cấp phát trước, tái dùng, và từ chối tải khi bể cạn.
3. **Chia sẻ giả gây chậm 5–10 lần mà không có tranh chấp logic nào.** Đệm theo dòng cache (Cache-line padding).
4. **Disruptor thắng nhờ ba thứ**: bộ nhớ cấp phát sẵn, chỉ mục bằng phép AND, con trỏ đơn điệu.
5. **Ngân sách độ trễ (Latency budget) chỉ ra nên tối ưu ở đâu.** Nếu hơn nửa thời gian ở card mạng, tối ưu code là công cốc — Amdahl đã nói vậy.

### Bài tập rèn luyện

**Bài 1.** Mở rộng vòng Disruptor thành **nhiều người tiêu thụ** — mỗi người đọc toàn bộ dòng dữ liệu với tốc độ riêng.

<details>
<summary><b>Gợi ý</b></summary>

Đây là mẫu "phát tán" (fan-out): sổ lệnh, ghi nhật ký, và giám sát rủi ro cùng đọc một dòng sự kiện thị trường. Người ghi chỉ được phép đè lên ô mà **người đọc chậm nhất** đã đi qua.
</details>

<details>
<summary><b>Lời giải</b></summary>

```rust
pub struct MultiReaderRing<T, const N: usize> {
    slots: Vec<Option<T>>,
    write_pos: u64,
    /// Mỗi người tiêu thụ giữ con trỏ đọc riêng.
    read_pos: Vec<u64>,
}

impl<T: Clone, const N: usize> MultiReaderRing<T, N> {
    pub fn new(readers: usize) -> Self {
        assert!(N.is_power_of_two(), "N phải là luỹ thừa của 2");
        Self {
            slots: (0..N).map(|_| None).collect(),
            write_pos: 0,
            read_pos: vec![0; readers],
        }
    }

    /// Rào chắn: người ghi bị chặn bởi NGƯỜI ĐỌC CHẬM NHẤT.
    fn slowest(&self) -> u64 {
        self.read_pos.iter().copied().min().unwrap_or(self.write_pos)
    }

    pub fn publish(&mut self, value: T) -> bool {
        if self.write_pos - self.slowest() >= N as u64 {
            return false; // đầy
        }
        let i = (self.write_pos as usize) & (N - 1);
        self.slots[i] = Some(value);
        self.write_pos += 1;
        true
    }

    pub fn read(&mut self, reader: usize) -> Option<T> {
        if self.read_pos[reader] >= self.write_pos {
            return None; // rỗng
        }
        let i = (self.read_pos[reader] as usize) & (N - 1);
        let v = self.slots[i].clone();
        self.read_pos[reader] += 1;
        v
    }

    /// Mỗi người đọc đang tụt lại bao xa — dấu hiệu cảnh báo sớm.
    pub fn lags(&self) -> Vec<u64> {
        self.read_pos.iter().map(|&r| self.write_pos - r).collect()
    }
}
```

`lags` là công cụ vận hành quan trọng: khi một người tiêu thụ bắt đầu tụt, nó sẽ **chặn cả người ghi**, làm chậm toàn hệ thống. Theo dõi số này giúp phát hiện vấn đề trước khi vòng đầy.
</details>

**Bài 2.** Cài **bộ đo thời gian có xét chi phí đo**: trừ đi chính chi phí gọi đồng hồ.

<details>
<summary><b>Gợi ý</b></summary>

`Instant::now()` tự nó tốn 20–30 ns. Khi đo một thao tác chỉ mất 50 ns, chi phí đo chiếm hơn một phần ba kết quả. Cách chữa: hiệu chỉnh bằng cách đo hai lần gọi đồng hồ liền nhau, rồi trừ đi.
</details>

<details>
<summary><b>Lời giải</b></summary>

```rust
use std::time::Instant;

pub struct CalibratedClock {
    overhead_ns: u64,
}

impl CalibratedClock {
    pub fn calibrate(samples: usize) -> Self {
        assert!(samples > 0, "cần ít nhất một mẫu để hiệu chỉnh");
        let mut costs = Vec::with_capacity(samples);
        for _ in 0..samples {
            let t0 = Instant::now();
            let t1 = Instant::now(); // chỉ đo chi phí gọi
            costs.push(t1.duration_since(t0).as_nanos() as u64);
        }
        costs.sort_unstable();
        // Lấy TRUNG VỊ, không lấy trung bình: nhiễu hệ điều hành lệch phải mạnh.
        Self {
            overhead_ns: costs[samples / 2],
        }
    }

    pub fn measure_ns<F: FnOnce() -> R, R>(&self, f: F) -> (R, u64) {
        let t0 = Instant::now();
        let result = std::hint::black_box(f());
        let raw = t0.elapsed().as_nanos() as u64;
        (result, raw.saturating_sub(self.overhead_ns))
    }
}
```

Hai chi tiết: dùng **trung vị** chứ không trung bình khi hiệu chỉnh (nhiễu hệ điều hành lệch phải rất mạnh), và `saturating_sub` để tránh kết quả âm khi thao tác nhanh hơn cả chi phí đo — lúc đó bạn cần đo theo lô, không đo từng lần.
</details>
