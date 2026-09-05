# Chương 74: Nền tảng HFT — Đo độ trễ, Vòng Disruptor & Bố cục bộ nhớ

## Giới thiệu & Mục tiêu học tập

Giao dịch tần suất cao (HFT) là ngành duy nhất mà **nanosecond có giá bằng tiền mặt**. Ở Jane Street, Optiver hay Jump Trading, người ta viết lại toàn bộ ngăn xếp phần mềm chỉ để bớt vài trăm nanosecond.

Điều làm HFT khác biệt không phải là "code nhanh". Đó là ba nguyên tắc:

| Nguyên tắc | Nội dung |
|---|---|
| Đo bằng phân vị, không bằng trung bình | Trung bình giấu đi chính thứ giết bạn: cái đuôi |
| Không cấp phát trên đường nóng | `malloc` là bất định; bất định là kẻ thù |
| Bố cục bộ nhớ quan trọng hơn thuật toán | Một lần trượt cache = 300 chu kỳ = 100 phép cộng |

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
│    AoS: [gia,sl,ma][gia,sl,ma][gia,sl,ma]  ← đọc 1 lệnh: 1 dòng cache      │
│    SoA: [gia,gia,gia][sl,sl,sl][ma,ma,ma]  ← quét mọi giá: cực nhanh       │
│    Không có bên nào luôn thắng. Chọn theo CÁCH BẠN TRUY CẬP.               │
└──────────────────────────────────────────────────────────────────────────────┘
```

---

## Khái niệm & Cơ chế kỹ thuật chuyên sâu

### 1. Biểu đồ phân vị: đo cái đuôi

Trong HFT, các con số đáng quan tâm là p50, p99, p99.9 và **max**. Trung bình chỉ hữu ích để phát hiện là mình đã đo sai.

Cấu trúc phù hợp là biểu đồ kiểu HDR: các thùng có độ rộng tăng dần (logarit), nên lưu được dải từ 1 ns tới 1 phút mà chỉ tốn vài kilobyte, với sai số tương đối cố định.

Bài kiểm thử của chương này minh hoạ chính xác cái bẫy: một phân phối gồm 99,9% mẫu ở khoảng 300 ns và 0,1% ở khoảng 50 µs. p50 và p99 đều nhanh, p99.9 mới bắt đầu lộ, còn **max lớn hơn trung bình hơn 100 lần**. Nếu chỉ nhìn trung bình, bạn sẽ tuyên bố hệ thống "nhanh 500 ns" trong khi thực tế nó thỉnh thoảng đứng hình 50 µs.

### 2. Vì sao Disruptor không dùng khoá

Một mutex có ba vấn đề trên đường nóng:
- **Đảo ngược ưu tiên**: luồng giữ khoá bị hệ điều hành cho ra rìa, luồng quan trọng phải chờ.
- **Chuyển ngữ cảnh**: khi tranh chấp, chi phí là hàng microsecond — gấp hàng nghìn lần công việc thật.
- **Bất định**: bạn không biết trước lần này có tranh chấp hay không.

Disruptor thay bằng hai con trỏ nguyên tử **đơn điệu tăng**. Người ghi chỉ chờ khi vòng đầy; người đọc chỉ chờ khi vòng rỗng. Không có khoá nào cả.

Hai chi tiết cài đặt đáng nhớ:
- Kích thước **luỹ thừa của 2** để `chỉ_mục = con_trỏ & (N−1)` — phép AND một chu kỳ, thay cho phép chia hàng chục chu kỳ.
- Con trỏ **không bao giờ quay vòng** (u64 tăng mãi). Nhờ vậy `ghi − đọc` cho biết chính xác số phần tử, phân biệt được rỗng và đầy — điều mà con trỏ quay vòng không làm được nếu không hy sinh một ô.

### 3. Bể đối tượng: cấp phát trước, tái dùng mãi

Chiến lược của HFT là: **cấp phát toàn bộ lúc khởi động, không cấp phát gì nữa khi chạy**. Một `Vec::push` có thể tái cấp phát và sao chép — chi phí đó không dự đoán được.

Bể đối tượng giữ một danh sách chỉ số rỗi. `lay()` là pop, `tra()` là push. Cả hai đều O(1) và không chạm tới bộ cấp phát của hệ thống. Khi bể cạn, `lay()` trả `None` — và đó là **hành vi đúng**: hệ thống từ chối tải mới thay vì tự làm chậm mình một cách bất định.

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

### 5. Bố cục cấu trúc phải là quyết định có chủ đích

Chương này định nghĩa `GoiLenh` chiếm **đúng 64 byte** — một dòng cache. Không phải ngẫu nhiên: một lệnh đọc lên là đúng một lần nạp cache, và một mảng lệnh sẽ không bao giờ có phần tử nào nằm vắt qua hai dòng.

Một lưu ý Rust: mảng chỉ cài `Default` tới kích thước 32. Với `[u8; 64]` bạn phải tự cài `Default`, hoặc — như chương này làm — thiết kế một struct có ý nghĩa thay vì mảng byte trần.

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

/// Biểu đồ tần suất kiểu HDR: chia thang log thành các "xô" để giữ độ chính
/// xác tương đối ở mọi bậc độ lớn, mà chỉ tốn vài trăm byte.
///
/// Ghi một mẫu là O(1) và KHÔNG cấp phát — bắt buộc, vì bản thân việc đo
/// không được làm nhiễu thứ đang đo.
pub struct BieuDoTre {
    /// xo[i] đếm các giá trị trong [2^(i-1), 2^i)
    xo: Vec<u64>,
    pub tong_mau: u64,
    pub nho_nhat: u64,
    pub lon_nhat: u64,
    tong_gia_tri: u128,
}

impl BieuDoTre {
    pub fn moi() -> Self {
        BieuDoTre { xo: vec![0; 65], tong_mau: 0, nho_nhat: u64::MAX,
                    lon_nhat: 0, tong_gia_tri: 0 }
    }

    #[inline]
    pub fn ghi(&mut self, ns: u64) {
        let i = if ns == 0 { 0 } else { 64 - ns.leading_zeros() as usize };
        self.xo[i] += 1;
        self.tong_mau += 1;
        self.tong_gia_tri += ns as u128;
        if ns < self.nho_nhat { self.nho_nhat = ns; }
        if ns > self.lon_nhat { self.lon_nhat = ns; }
    }

    pub fn trung_binh(&self) -> f64 {
        if self.tong_mau == 0 { 0.0 } else { self.tong_gia_tri as f64 / self.tong_mau as f64 }
    }

    /// Cận TRÊN của xô chứa phân vị. Với thang log, sai số tương đối bị chặn
    /// trong mỗi xô — đủ tốt để phát hiện đuôi dài, vốn là mục đích chính.
    pub fn phan_vi(&self, p: f64) -> u64 {
        if self.tong_mau == 0 { return 0; }
        let nguong = (self.tong_mau as f64 * p).ceil().max(1.0) as u64;
        let mut cong_don = 0u64;
        for (i, &c) in self.xo.iter().enumerate() {
            cong_don += c;
            if cong_don >= nguong {
                return if i == 0 { 0 } else { (1u64 << (i - 1)) * 2 - 1 };
            }
        }
        self.lon_nhat
    }

    /// Bản tóm tắt mà một kỹ sư độ trễ thật sự nhìn vào.
    pub fn tom_tat(&self) -> String {
        format!("n={} min={} p50={} p99={} p99.9={} max={} (tb={:.0})",
                self.tong_mau, self.nho_nhat, self.phan_vi(0.50),
                self.phan_vi(0.99), self.phan_vi(0.999), self.lon_nhat, self.trung_binh())
    }
}

// ============================================================================
// 2. CHIA SẺ GIẢ — hai biến cạnh nhau giết chết hiệu năng đa luồng
// ============================================================================

pub const DONG_CACHE: usize = 64;

/// Hai bộ đếm nằm CÙNG một dòng cache. Hai lõi ghi vào hai biến khác nhau,
/// nhưng phần cứng chỉ biết tới dòng cache — nên chúng giành nhau quyền sở
/// hữu dòng đó, ping-pong qua lại. Chậm hơn hàng chục lần mà nhìn mã không thấy.
#[repr(C)]
pub struct BoDemChungDong { pub a: AtomicUsize, pub b: AtomicUsize }

/// Đệm cho mỗi bộ đếm chiếm trọn một dòng cache riêng.
#[repr(C, align(64))]
pub struct DemCoDem { pub gia_tri: AtomicUsize, _dem: [u8; DONG_CACHE - 8] }

impl DemCoDem {
    pub fn moi() -> Self { DemCoDem { gia_tri: AtomicUsize::new(0), _dem: [0; DONG_CACHE - 8] } }
}

#[repr(C)]
pub struct BoDemTachDong { pub a: DemCoDem, pub b: DemCoDem }

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
pub struct VongDisruptor<T, const N: usize> {
    o: UnsafeCell<[Option<T>; N]>,
    _dem1: [u8; DONG_CACHE],
    vi_tri_ghi: AtomicUsize,
    _dem2: [u8; DONG_CACHE - 8],
    vi_tri_doc: AtomicUsize,
    _dem3: [u8; DONG_CACHE - 8],
}

// An toàn: mỗi con trỏ chỉ có ĐÚNG MỘT bên ghi vào.
unsafe impl<T: Send, const N: usize> Sync for VongDisruptor<T, N> {}
unsafe impl<T: Send, const N: usize> Send for VongDisruptor<T, N> {}

impl<T, const N: usize> VongDisruptor<T, N> {
    pub fn moi() -> Self {
        assert!(N.is_power_of_two(), "sức chứa phải là luỹ thừa của 2");
        VongDisruptor {
            o: UnsafeCell::new(std::array::from_fn(|_| None)),
            _dem1: [0; DONG_CACHE],
            vi_tri_ghi: AtomicUsize::new(0), _dem2: [0; DONG_CACHE - 8],
            vi_tri_doc: AtomicUsize::new(0), _dem3: [0; DONG_CACHE - 8],
        }
    }

    #[inline]
    fn chi_so(v: usize) -> usize { v & (N - 1) } // thay cho v % N

    pub fn so_luong(&self) -> usize {
        self.vi_tri_ghi.load(Ordering::Acquire) - self.vi_tri_doc.load(Ordering::Acquire)
    }
    pub fn rong(&self) -> bool { self.so_luong() == 0 }
    pub fn day(&self) -> bool { self.so_luong() == N }
    pub fn suc_chua(&self) -> usize { N }

    /// Gọi từ luồng SẢN XUẤT. Trả `Err` khi đầy — không bao giờ chặn,
    /// vì chặn trên đường nóng là điều cấm kỵ.
    pub fn day_vao(&self, gt: T) -> Result<(), T> {
        let ghi = self.vi_tri_ghi.load(Ordering::Relaxed); // ta là bên duy nhất ghi nó
        let doc = self.vi_tri_doc.load(Ordering::Acquire);
        if ghi - doc == N { return Err(gt); }
        unsafe { (*self.o.get())[Self::chi_so(ghi)] = Some(gt); }
        // Release: bảo đảm dữ liệu ghi xong TRƯỚC khi bên đọc thấy con trỏ mới
        self.vi_tri_ghi.store(ghi + 1, Ordering::Release);
        Ok(())
    }

    /// Gọi từ luồng TIÊU THỤ.
    pub fn lay_ra(&self) -> Option<T> {
        let doc = self.vi_tri_doc.load(Ordering::Relaxed);
        let ghi = self.vi_tri_ghi.load(Ordering::Acquire);
        if doc == ghi { return None; }
        let gt = unsafe { (*self.o.get())[Self::chi_so(doc)].take() };
        self.vi_tri_doc.store(doc + 1, Ordering::Release);
        gt
    }

    /// Lấy cả LÔ — mấu chốt của thông lượng cao: một lần đồng bộ cho nhiều
    /// phần tử, nên chi phí hàng rào bộ nhớ được chia đều cho cả lô.
    pub fn lay_lo(&self, toi_da: usize, ra: &mut Vec<T>) -> usize {
        let doc = self.vi_tri_doc.load(Ordering::Relaxed);
        let ghi = self.vi_tri_ghi.load(Ordering::Acquire);
        let n = (ghi - doc).min(toi_da);
        for i in 0..n {
            if let Some(x) = unsafe { (*self.o.get())[Self::chi_so(doc + i)].take() } {
                ra.push(x);
            }
        }
        if n > 0 { self.vi_tri_doc.store(doc + n, Ordering::Release); }
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
pub struct GoiLenh {
    pub ma_lenh: u64,
    pub gia: i64,
    pub so_luong: i64,
    pub ma_ck: u32,
    pub chieu: u8,
    pub dem: [u8; 32],
}

pub struct BeDoiTuong<T> {
    ranh: Vec<usize>,
    o: Vec<T>,
    pub so_lan_muon: u64,
    pub so_lan_het_be: u64,
}

impl<T: Default + Clone> BeDoiTuong<T> {
    pub fn moi(suc_chua: usize) -> Self {
        BeDoiTuong {
            ranh: (0..suc_chua).rev().collect(),
            o: vec![T::default(); suc_chua],
            so_lan_muon: 0, so_lan_het_be: 0,
        }
    }
    pub fn con_ranh(&self) -> usize { self.ranh.len() }

    /// Trả về CHỈ SỐ chứ không phải con trỏ — tránh hẳn vấn đề vòng đời.
    pub fn muon(&mut self) -> Option<usize> {
        self.so_lan_muon += 1;
        match self.ranh.pop() {
            Some(i) => Some(i),
            None => { self.so_lan_het_be += 1; None }
        }
    }
    pub fn tra(&mut self, i: usize) { self.ranh.push(i); }
    pub fn xem(&self, i: usize) -> &T { &self.o[i] }
    pub fn sua(&mut self, i: usize) -> &mut T { &mut self.o[i] }
}

// ============================================================================
// 5. BỐ TRÍ BỘ NHỚ — mảng-của-struct vs struct-của-mảng
// ============================================================================

/// Mảng-của-struct (AoS): mỗi bản ghi liền mạch. Tốt khi đọc TẤT CẢ trường.
/// Trường được xếp theo kích thước GIẢM DẦN để trình biên dịch không phải đệm.
#[derive(Clone, Copy, Default)]
pub struct BaoGiaAoS {
    pub gia_mua: i64,
    pub gia_ban: i64,
    pub thoi_diem: u64,
    pub co: u64,
    pub ma_ck: u32,
    pub kl_mua: u32,
    pub kl_ban: u32,
    pub dem: u32,
}

/// Struct-của-mảng (SoA): mỗi trường một mảng riêng. Tốt khi chỉ đọc MỘT
/// trường trên nhiều bản ghi — CPU nạp một dòng cache 64 byte là được 8 giá
/// trị đều có ích, thay vì 8 byte có ích trên 40 byte rác.
#[derive(Default)]
pub struct BangBaoGiaSoA {
    pub ma_ck: Vec<u32>,
    pub gia_mua: Vec<i64>,
    pub gia_ban: Vec<i64>,
    pub kl_mua: Vec<u32>,
    pub kl_ban: Vec<u32>,
    pub thoi_diem: Vec<u64>,
}

impl BangBaoGiaSoA {
    pub fn moi(n: usize) -> Self {
        BangBaoGiaSoA {
            ma_ck: vec![0; n], gia_mua: vec![0; n], gia_ban: vec![0; n],
            kl_mua: vec![0; n], kl_ban: vec![0; n], thoi_diem: vec![0; n],
        }
    }
    pub fn so_luong(&self) -> usize { self.ma_ck.len() }

    /// Quét chỉ trường `gia_mua` — đây là chỗ SoA thắng đậm.
    pub fn tong_gia_mua(&self) -> i128 { self.gia_mua.iter().map(|&x| x as i128).sum() }

    /// Số byte thực sự phải kéo từ RAM để quét một trường 8 byte.
    pub fn byte_can_doc_mot_truong(&self) -> usize { self.so_luong() * 8 }
}

pub fn byte_can_doc_mot_truong_aos(n: usize) -> usize {
    // Phải kéo cả bản ghi dù chỉ cần 8 byte
    n * std::mem::size_of::<BaoGiaAoS>()
}

// ============================================================================
// 6. NGÂN SÁCH ĐỘ TRỄ — chia nhỏ "tick-to-trade"
// ============================================================================

#[derive(Debug, Clone, PartialEq)]
pub struct ChangDoTre { pub ten: String, pub ns: u64 }

#[derive(Debug, PartialEq)]
pub struct NganSachDoTre { pub chang: Vec<ChangDoTre>, pub tran_ns: u64 }

impl NganSachDoTre {
    pub fn tong(&self) -> u64 { self.chang.iter().map(|c| c.ns).sum() }
    pub fn dat_muc_tieu(&self) -> bool { self.tong() <= self.tran_ns }
    /// Chặng tốn nhất — nơi DUY NHẤT đáng bỏ công tối ưu.
    pub fn nut_that_co_chai(&self) -> Option<&ChangDoTre> {
        self.chang.iter().max_by_key(|c| c.ns)
    }
    /// Định luật Amdahl: tăng tốc tối đa nếu chặng nghẽn cổ chai thành 0.
    pub fn tang_toc_toi_da_neu_xoa_nut(&self) -> f64 {
        match self.nut_that_co_chai() {
            Some(n) if self.tong() > n.ns => self.tong() as f64 / (self.tong() - n.ns) as f64,
            _ => f64::INFINITY,
        }
    }
}

/// Sinh mẫu độ trễ tất định có ĐUÔI DÀI — giống hệt hệ thống thật:
/// phần lớn nhanh, thỉnh thoảng một cú chậm gấp hàng trăm lần.
pub fn sinh_mau_do_tre(n: usize, hat_giong: u64) -> Vec<u64> {
    let mut s = hat_giong;
    (0..n).map(|_| {
        s = s.wrapping_mul(6364136223846793005).wrapping_add(1442695040888963407);
        let r = (s >> 33) % 10_000;
        match r {
            0..=9_899 => 200 + r % 100,       // 99%  : 200–300 ns
            9_900..=9_989 => 2_000 + r % 500, // 0.9% : ~2 µs (trượt cache)
            _ => 50_000 + r % 10_000,         // 0.1% : ~50 µs (hệ điều hành xen vào)
        }
    }).collect()
}

fn main() {
    println!("═══════════════════════════════════════════════════════════");
    println!("   KỸ NGHỆ ĐỘ TRỄ THẤP: ĐO · KHÔNG KHOÁ · KHÔNG CẤP PHÁT   ");
    println!("═══════════════════════════════════════════════════════════");

    println!("\n1. VÌ SAO TRUNG BÌNH LÀ CON SỐ VÔ DỤNG");
    let mut bd = BieuDoTre::moi();
    for x in sinh_mau_do_tre(1_000_000, 42) { bd.ghi(x); }
    println!("   {}", bd.tom_tat());
    println!("   Trung bình {:.0} ns nghe rất đẹp…", bd.trung_binh());
    println!("   …nhưng 1 trên 1000 lệnh rơi vào dải tới {} ns, và cú chậm nhất là {} ns",
             bd.phan_vi(0.999), bd.lon_nhat);
    println!("   — gấp {:.0} lần trung bình. (Phân vị là cận TRÊN của xô log.)",
             bd.lon_nhat as f64 / bd.trung_binh());
    println!("   Trong giao dịch, chính CÁI ĐUÔI đó là lúc bạn mất tiền.");

    println!("\n2. CHIA SẺ GIẢ — kích thước quyết định tốc độ");
    println!("   BoDemChungDong: {} byte (hai bộ đếm CÙNG một dòng cache)",
             std::mem::size_of::<BoDemChungDong>());
    println!("   BoDemTachDong : {} byte (mỗi bộ đếm một dòng riêng)",
             std::mem::size_of::<BoDemTachDong>());
    println!("   → Tốn thêm {} byte để tránh ping-pong dòng cache giữa hai lõi.",
             std::mem::size_of::<BoDemTachDong>() - std::mem::size_of::<BoDemChungDong>());

    println!("\n3. VÒNG ĐỆM DISRUPTOR");
    let v: VongDisruptor<u64, 1024> = VongDisruptor::moi();
    for i in 0..1024 { v.day_vao(i).unwrap(); }
    println!("   Đẩy 1024 phần tử → đầy: {} · đẩy thêm → bị từ chối: {}",
             v.day(), v.day_vao(9999).is_err());
    let mut lo = Vec::new();
    let n = v.lay_lo(256, &mut lo);
    println!("   Lấy một lô 256 → được {} phần tử, còn lại {}", n, v.so_luong());
    println!("   Chỉ số dùng phép AND: 1030 & 1023 = {} (thay cho phép chia)", 1030usize & 1023);

    println!("\n4. BỂ ĐỐI TƯỢNG");
    let mut be: BeDoiTuong<GoiLenh> = BeDoiTuong::moi(4);
    let cac_i: Vec<usize> = (0..4).filter_map(|_| be.muon()).collect();
    println!("   Mượn 4/4 → còn rảnh {} · mượn thêm → {:?}", be.con_ranh(), be.muon());
    be.tra(cac_i[0]);
    println!("   Trả 1 lại → còn rảnh {} · số lần hết bể = {}", be.con_ranh(), be.so_lan_het_be);
    println!("   Một GoiLenh = {} byte — vừa đúng một dòng cache",
             std::mem::size_of::<GoiLenh>());

    println!("\n5. BỐ TRÍ BỘ NHỚ — AoS vs SoA khi quét MỘT trường");
    let n = 100_000;
    println!("   Một bản ghi AoS = {} byte", std::mem::size_of::<BaoGiaAoS>());
    println!("   Quét {} bản ghi chỉ để lấy `gia_mua`:", n);
    println!("     AoS phải kéo {:>9} byte từ RAM", byte_can_doc_mot_truong_aos(n));
    println!("     SoA chỉ kéo  {:>9} byte", BangBaoGiaSoA::moi(n).byte_can_doc_mot_truong());
    println!("   → SoA đọc ít hơn {:.1}× — và đó là băng thông RAM, thứ đắt nhất.",
             byte_can_doc_mot_truong_aos(n) as f64 / (n * 8) as f64);

    println!("\n6. NGÂN SÁCH ĐỘ TRỄ TICK-TO-TRADE");
    let ns = NganSachDoTre {
        tran_ns: 5_000,
        chang: vec![
            ChangDoTre { ten: "Card mạng → bộ nhớ".into(), ns: 800 },
            ChangDoTre { ten: "Phân tích gói tin".into(), ns: 150 },
            ChangDoTre { ten: "Cập nhật sổ lệnh".into(), ns: 400 },
            ChangDoTre { ten: "Chiến lược quyết định".into(), ns: 250 },
            ChangDoTre { ten: "Kiểm tra rủi ro".into(), ns: 120 },
            ChangDoTre { ten: "Tuần tự hoá lệnh".into(), ns: 180 },
            ChangDoTre { ten: "Gọi hệ thống gửi".into(), ns: 1_500 },
        ],
    };
    for c in &ns.chang {
        let phan_tram = c.ns as f64 * 100.0 / ns.tong() as f64;
        println!("   {:<26} {:>5} ns  {:>5.1}%  {}",
                 c.ten, c.ns, phan_tram, "#".repeat((phan_tram / 2.0) as usize));
    }
    println!("   Tổng {} ns / trần {} ns → {}",
             ns.tong(), ns.tran_ns, if ns.dat_muc_tieu() { "ĐẠT" } else { "TRƯỢT" });
    println!("   Nút thắt: {} · xoá hẳn nó cũng chỉ nhanh được {:.2}×",
             ns.nut_that_co_chai().unwrap().ten, ns.tang_toc_toi_da_neu_xoa_nut());
    println!("   → Đó là lý do HFT thật dùng kernel bypass: gọi hệ thống là chặng đắt nhất.");

    println!("\n═══════════════════════════════════════════════════════════");
    println!("   ĐO PHÂN VỊ, ĐỪNG ĐO TRUNG BÌNH. TỐI ƯU NÚT, ĐỪNG TỐI ƯU BỪA");
    println!("═══════════════════════════════════════════════════════════");
}

#[cfg(test)]
mod kiem_thu {
    use super::*;

    // ---------- Biểu đồ độ trễ ----------
    #[test]
    fn bieu_do_rong_khong_panic() {
        let b = BieuDoTre::moi();
        assert_eq!(b.tong_mau, 0);
        assert_eq!(b.trung_binh(), 0.0);
        assert_eq!(b.phan_vi(0.99), 0);
    }

    #[test]
    fn bieu_do_ghi_dung_min_max_va_trung_binh() {
        let mut b = BieuDoTre::moi();
        for x in [10u64, 20, 30, 40] { b.ghi(x); }
        assert_eq!(b.nho_nhat, 10);
        assert_eq!(b.lon_nhat, 40);
        assert_eq!(b.trung_binh(), 25.0);
        assert_eq!(b.tong_mau, 4);
    }

    #[test]
    fn phan_vi_tang_don_dieu() {
        let mut b = BieuDoTre::moi();
        for x in sinh_mau_do_tre(10_000, 7) { b.ghi(x); }
        let (p50, p90, p99, p999) = (b.phan_vi(0.5), b.phan_vi(0.9),
                                     b.phan_vi(0.99), b.phan_vi(0.999));
        assert!(p50 <= p90 && p90 <= p99 && p99 <= p999,
                "phân vị phải tăng dần: {} {} {} {}", p50, p90, p99, p999);
        assert!(p999 <= b.lon_nhat);
    }

    #[test]
    fn phan_vi_bao_gio_cung_bao_phu_gia_tri_that() {
        // Cận trên của xô phải THỰC SỰ là cận trên: không được báo thấp hơn
        // giá trị thật, nếu không ta sẽ tưởng hệ thống nhanh hơn thực tế.
        let mut b = BieuDoTre::moi();
        for x in [1u64, 2, 3, 100, 1000] { b.ghi(x); }
        assert!(b.phan_vi(1.0) >= 1000);
        assert!(b.phan_vi(0.8) >= 100, "80% mẫu ≤ 100, cận phải ≥ 100");
    }

    #[test]
    fn duoi_dai_lam_trung_binh_noi_doi() {
        // Đây là bài học trung tâm của chương: 99% mẫu ở 200–300 ns, nhưng
        // 0.1% ở 50 µs kéo trung bình lên và che mất phân bố thật.
        let mut b = BieuDoTre::moi();
        for x in sinh_mau_do_tre(100_000, 42) { b.ghi(x); }

        // Phân bố thật: p50 ≈ 250 ns, p99 ≈ 299 ns, p99.9 ≈ 2.5 µs, max ≈ 60 µs.
        // Chú ý p99 vẫn NHANH — phải soi tới p99.9 mới thấy dấu vết đuôi,
        // và tới giá trị lớn nhất mới thấy hết mức độ.
        assert!(b.phan_vi(0.5) < 512, "phân vị 50 phải nằm ở vùng nhanh");
        assert!(b.phan_vi(0.99) < 512, "ngay cả p99 vẫn nhanh — đuôi còn ẩn kỹ hơn thế");
        assert!(b.phan_vi(0.999) > 2_000,
                "tới p99.9 mới lộ ra đuôi, thực tế {}", b.phan_vi(0.999));
        assert!(b.lon_nhat > 50_000, "giá trị lớn nhất mới cho thấy hết mức độ");

        // Đây là con số đắt giá nhất: trung bình ~326 ns che mất một cú
        // gần 60 µs, tức chậm gấp gần 200 lần.
        assert!(b.lon_nhat as f64 > b.trung_binh() * 100.0,
                "max {} so với trung bình {:.0} — trung bình che giấu đúng thứ giết bạn",
                b.lon_nhat, b.trung_binh());
    }

    #[test]
    fn ghi_gia_tri_khong_va_gia_tri_lon_nhat_deu_an_toan() {
        let mut b = BieuDoTre::moi();
        b.ghi(0);
        b.ghi(u64::MAX);
        assert_eq!(b.tong_mau, 2);
        assert_eq!(b.nho_nhat, 0);
        assert_eq!(b.lon_nhat, u64::MAX);
    }

    // ---------- Chia sẻ giả ----------
    #[test]
    fn dem_co_dem_chiem_tron_mot_dong_cache() {
        assert_eq!(std::mem::size_of::<DemCoDem>(), DONG_CACHE);
        assert_eq!(std::mem::align_of::<DemCoDem>(), DONG_CACHE,
                   "phải căn theo dòng cache, không chỉ đủ kích thước");
    }

    #[test]
    fn hai_bo_dem_tach_dong_khong_the_chung_dong_cache() {
        let b = BoDemTachDong { a: DemCoDem::moi(), b: DemCoDem::moi() };
        let dc_a = &b.a as *const _ as usize;
        let dc_b = &b.b as *const _ as usize;
        assert!(dc_b - dc_a >= DONG_CACHE,
                "hai bộ đếm cách nhau {} byte, phải ít nhất {}", dc_b - dc_a, DONG_CACHE);
        // Ngược lại, phiên bản không đệm thì chúng nằm sát nhau
        let c = BoDemChungDong { a: AtomicUsize::new(0), b: AtomicUsize::new(0) };
        let ca = &c.a as *const _ as usize;
        let cb = &c.b as *const _ as usize;
        assert!(cb - ca < DONG_CACHE,
                "đây chính là chia sẻ giả: cách nhau chỉ {} byte", cb - ca);
    }

    // ---------- Vòng Disruptor ----------
    #[test]
    fn vong_vao_truoc_ra_truoc() {
        let v: VongDisruptor<u32, 8> = VongDisruptor::moi();
        for i in 0..5 { v.day_vao(i).unwrap(); }
        for i in 0..5 { assert_eq!(v.lay_ra(), Some(i)); }
        assert_eq!(v.lay_ra(), None);
    }

    #[test]
    fn vong_dung_het_suc_chua_khong_hy_sinh_o_nao() {
        // Hàng đợi vòng thường phải bỏ một ô để phân biệt rỗng/đầy.
        // Con trỏ tăng mãi giúp ta dùng trọn N ô.
        let v: VongDisruptor<u32, 8> = VongDisruptor::moi();
        for i in 0..8 { assert!(v.day_vao(i).is_ok(), "phải nhận đủ 8 phần tử"); }
        assert!(v.day());
        assert_eq!(v.so_luong(), 8);
        assert_eq!(v.day_vao(99), Err(99));
    }

    #[test]
    fn vong_quay_dung_qua_nhieu_luot() {
        let v: VongDisruptor<u64, 4> = VongDisruptor::moi();
        for i in 0..1000u64 {
            v.day_vao(i).unwrap();
            assert_eq!(v.lay_ra(), Some(i), "chỉ số phải quấn đúng qua biên mảng");
        }
        assert!(v.rong());
    }

    #[test]
    fn vong_rong_tra_none_va_khong_panic() {
        let v: VongDisruptor<u8, 16> = VongDisruptor::moi();
        assert_eq!(v.lay_ra(), None);
        assert!(v.rong() && !v.day());
        assert_eq!(v.so_luong(), 0);
    }

    #[test]
    fn lay_lo_lay_dung_so_luong_va_dung_thu_tu() {
        let v: VongDisruptor<u32, 64> = VongDisruptor::moi();
        for i in 0..50 { v.day_vao(i).unwrap(); }
        let mut ra = Vec::new();
        assert_eq!(v.lay_lo(20, &mut ra), 20);
        assert_eq!(ra, (0..20).collect::<Vec<u32>>());
        assert_eq!(v.so_luong(), 30);
        // Xin nhiều hơn số có thì chỉ lấy được số có
        let mut ra2 = Vec::new();
        assert_eq!(v.lay_lo(1000, &mut ra2), 30);
        assert!(v.rong());
    }

    #[test]
    fn lay_lo_tren_vong_rong_tra_ve_khong() {
        let v: VongDisruptor<u32, 8> = VongDisruptor::moi();
        let mut ra = Vec::new();
        assert_eq!(v.lay_lo(10, &mut ra), 0);
        assert!(ra.is_empty());
    }

    #[test]
    fn phep_and_thay_duoc_phep_chia_khi_suc_chua_la_luy_thua_hai() {
        for n in [8usize, 16, 64, 1024, 4096] {
            for v in [0usize, 1, 7, 1030, 99999] {
                assert_eq!(v & (n - 1), v % n, "AND phải cho cùng kết quả với MOD");
            }
        }
    }

    #[test]
    #[should_panic(expected = "luỹ thừa của 2")]
    fn suc_chua_khong_phai_luy_thua_hai_bi_tu_choi() {
        let _: VongDisruptor<u8, 100> = VongDisruptor::moi();
    }

    // ---------- Bể đối tượng ----------
    #[test]
    fn be_cap_phat_va_tra_lai_dung() {
        let mut b: BeDoiTuong<u64> = BeDoiTuong::moi(3);
        assert_eq!(b.con_ranh(), 3);
        let a = b.muon().unwrap();
        let c = b.muon().unwrap();
        assert_ne!(a, c, "hai lần mượn phải ra hai ô khác nhau");
        assert_eq!(b.con_ranh(), 1);
        b.tra(a);
        assert_eq!(b.con_ranh(), 2);
    }

    #[test]
    fn be_het_thi_bao_none_chu_khong_cap_phat_them() {
        // Điểm mấu chốt: thà từ chối còn hơn cấp phát heap trên đường nóng.
        let mut b: BeDoiTuong<u32> = BeDoiTuong::moi(2);
        assert!(b.muon().is_some());
        assert!(b.muon().is_some());
        assert!(b.muon().is_none());
        assert_eq!(b.so_lan_het_be, 1, "phải ĐẾM số lần hết bể để còn chỉnh kích thước");
        assert_eq!(b.so_lan_muon, 3);
    }

    #[test]
    fn goi_lenh_vua_dung_mot_dong_cache() {
        assert_eq!(std::mem::size_of::<GoiLenh>(), DONG_CACHE,
                   "bản ghi trên đường nóng nên vừa một dòng cache, không hơn");
    }

    #[test]
    fn o_vua_tra_duoc_tai_dung_ngay() {
        let mut b: BeDoiTuong<u64> = BeDoiTuong::moi(2);
        let i = b.muon().unwrap();
        *b.sua(i) = 12345;
        assert_eq!(*b.xem(i), 12345);
        b.tra(i);
        let j = b.muon().unwrap();
        assert_eq!(i, j, "ô vừa trả phải được tái dùng ngay — nó còn NÓNG trong cache");
    }

    // ---------- Bố trí bộ nhớ ----------
    #[test]
    fn soa_doc_it_byte_hon_han_aos_khi_quet_mot_truong() {
        let n = 10_000;
        let aos = byte_can_doc_mot_truong_aos(n);
        let soa = BangBaoGiaSoA::moi(n).byte_can_doc_mot_truong();
        assert!(aos > soa * 4, "AoS đọc {} byte, SoA chỉ {} byte", aos, soa);
    }

    #[test]
    fn soa_tinh_dung_tong() {
        let mut t = BangBaoGiaSoA::moi(5);
        for i in 0..5 { t.gia_mua[i] = (i as i64 + 1) * 100; }
        assert_eq!(t.tong_gia_mua(), 100 + 200 + 300 + 400 + 500);
    }

    #[test]
    fn bao_gia_aos_khong_bi_don_dem_bat_ngo() {
        // Nếu kích thước lệch so với tổng các trường thì có đệm ẩn — điều
        // cần biết khi tính băng thông bộ nhớ. Xếp trường theo kích thước
        // giảm dần là cách đơn giản nhất để tránh đệm.
        let tong_truong = 8 + 8 + 8 + 8 + 4 + 4 + 4 + 4;
        assert_eq!(std::mem::size_of::<BaoGiaAoS>(), tong_truong);
    }

    // ---------- Ngân sách độ trễ ----------
    fn ns_mau() -> NganSachDoTre {
        NganSachDoTre {
            tran_ns: 5_000,
            chang: vec![
                ChangDoTre { ten: "mang".into(), ns: 800 },
                ChangDoTre { ten: "phan_tich".into(), ns: 150 },
                ChangDoTre { ten: "goi_he_thong".into(), ns: 1_500 },
            ],
        }
    }

    #[test]
    fn ngan_sach_tinh_dung_tong_va_nut_that() {
        let ns = ns_mau();
        assert_eq!(ns.tong(), 2_450);
        assert!(ns.dat_muc_tieu());
        assert_eq!(ns.nut_that_co_chai().unwrap().ten, "goi_he_thong");
    }

    #[test]
    fn amdahl_tinh_dung_gioi_han_tang_toc() {
        let ns = ns_mau();
        // Xoá hẳn chặng 1500 ns khỏi tổng 2450 ns → còn 950 ns
        let mong_doi = 2_450.0 / 950.0;
        assert!((ns.tang_toc_toi_da_neu_xoa_nut() - mong_doi).abs() < 1e-9);
        assert!(ns.tang_toc_toi_da_neu_xoa_nut() < 3.0,
                "kể cả xoá sạch nút thắt cũng chỉ nhanh được ~2.6× — đó là định luật Amdahl");
    }

    #[test]
    fn ngan_sach_vuot_tran_bi_bao_truot() {
        let ns = NganSachDoTre {
            tran_ns: 1_000,
            chang: vec![ChangDoTre { ten: "cham".into(), ns: 9_999 }],
        };
        assert!(!ns.dat_muc_tieu());
    }

    #[test]
    fn ngan_sach_mot_chang_duy_nhat_cho_tang_toc_vo_han() {
        let ns = NganSachDoTre {
            tran_ns: 100,
            chang: vec![ChangDoTre { ten: "tat_ca".into(), ns: 500 }],
        };
        assert!(ns.tang_toc_toi_da_neu_xoa_nut().is_infinite(),
                "xoá chặng duy nhất thì thời gian còn 0");
    }

    // ---------- Sinh mẫu ----------
    #[test]
    fn sinh_mau_tat_dinh() {
        assert_eq!(sinh_mau_do_tre(100, 5), sinh_mau_do_tre(100, 5));
        assert_ne!(sinh_mau_do_tre(100, 5), sinh_mau_do_tre(100, 6));
    }

    #[test]
    fn sinh_mau_co_dung_ba_vung_do_tre() {
        let m = sinh_mau_do_tre(100_000, 1);
        let nhanh = m.iter().filter(|&&x| x < 1_000).count();
        let vua = m.iter().filter(|&&x| (1_000..10_000).contains(&x)).count();
        let cham = m.iter().filter(|&&x| x >= 10_000).count();
        assert!(nhanh > 95_000, "~99% phải nhanh, thực tế {}", nhanh);
        assert!(vua > 0 && cham > 0, "phải có cả đuôi vừa và đuôi dài");
        assert_eq!(nhanh + vua + cham, m.len());
    }
}
```

---

## Bảng tra cứu lỗi biên dịch thường gặp

| Lỗi | Nguyên nhân trong chương này | Cách sửa |
|---|---|---|
| `E0277: [u8; 64]: Default is not satisfied` | Mảng chỉ cài `Default` tới 32 phần tử | Tự cài `Default`, hoặc thiết kế struct có nghĩa |
| `E0507: cannot move out of index` | Lấy `T` ra khỏi `Vec` trong vòng | `Option<T>` + `.take()`, hoặc `std::mem::replace` |
| `E0596: cannot borrow as mutable` | Vòng Disruptor cần `&mut` cho `ghi` | Trong bản thật dùng `UnsafeCell` + nguyên tử; bản dạy dùng `&mut` cho rõ |
| `attempt to subtract with overflow` | `ghi - doc` khi biểu đồ chưa có mẫu | Kiểm `so_mau == 0` trước khi tính phân vị |
| Đo được 0 ns | Trình tối ưu xoá vòng lặp trống | `std::hint::black_box` để giữ lại phép tính |

---

## Tóm tắt chương & Bài tập rèn luyện

### 5 điểm cốt lõi

1. **Đo phân vị, không đo trung bình.** Cái đuôi mới là thứ giết chiến lược, và nó xuất hiện đúng lúc thị trường biến động.
2. **Không cấp phát trên đường nóng.** Cấp phát trước, tái dùng, và từ chối tải khi bể cạn.
3. **Chia sẻ giả gây chậm 5–10 lần mà không có tranh chấp logic nào.** Đệm theo dòng cache.
4. **Disruptor thắng nhờ ba thứ**: bộ nhớ cấp phát sẵn, chỉ mục bằng phép AND, con trỏ đơn điệu.
5. **Ngân sách độ trễ chỉ ra nên tối ưu ở đâu.** Nếu hơn nửa thời gian ở card mạng, tối ưu code là công cốc — Amdahl đã nói vậy.

### Bài tập rèn luyện

**Bài 1.** Mở rộng vòng Disruptor thành **nhiều người tiêu thụ** — mỗi người đọc toàn bộ dòng dữ liệu với tốc độ riêng.

<details>
<summary><b>Gợi ý</b></summary>

Đây là mẫu "phát tán" (fan-out): sổ lệnh, ghi nhật ký, và giám sát rủi ro cùng đọc một dòng sự kiện thị trường. Người ghi chỉ được phép đè lên ô mà **người đọc chậm nhất** đã đi qua.
</details>

<details>
<summary><b>Lời giải</b></summary>

```rust
pub struct VongNhieuDoc<T, const N: usize> {
    dem: Vec<Option<T>>,
    ghi: u64,
    /// Mỗi người tiêu thụ giữ con trỏ riêng.
    doc: Vec<u64>,
}

impl<T: Clone, const N: usize> VongNhieuDoc<T, N> {
    pub fn moi(so_nguoi_doc: usize) -> Self {
        assert!(N.is_power_of_two(), "N phải là luỹ thừa của 2");
        Self {
            dem: (0..N).map(|_| None).collect(),
            ghi: 0,
            doc: vec![0; so_nguoi_doc],
        }
    }

    /// Rào chắn: người ghi bị chặn bởi NGƯỜI ĐỌC CHẬM NHẤT.
    fn cham_nhat(&self) -> u64 { self.doc.iter().copied().min().unwrap_or(self.ghi) }

    pub fn ghi(&mut self, gia_tri: T) -> bool {
        if self.ghi - self.cham_nhat() >= N as u64 { return false; } // đầy
        let i = (self.ghi as usize) & (N - 1);
        self.dem[i] = Some(gia_tri);
        self.ghi += 1;
        true
    }

    pub fn doc(&mut self, nguoi: usize) -> Option<T> {
        if self.doc[nguoi] >= self.ghi { return None; }              // rỗng
        let i = (self.doc[nguoi] as usize) & (N - 1);
        let v = self.dem[i].clone();
        self.doc[nguoi] += 1;
        v
    }

    /// Người đọc nào đang tụt lại — dấu hiệu cảnh báo sớm.
    pub fn do_tut_hau(&self) -> Vec<u64> {
        self.doc.iter().map(|&d| self.ghi - d).collect()
    }
}
```

`do_tut_hau` là công cụ vận hành quan trọng: khi một người tiêu thụ bắt đầu tụt, nó sẽ **chặn cả người ghi**, làm chậm toàn hệ thống. Theo dõi số này giúp phát hiện vấn đề trước khi vòng đầy.
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

pub struct DongHoHieuChinh { chi_phi_ns: u64 }

impl DongHoHieuChinh {
    pub fn hieu_chinh(so_mau: usize) -> Self {
        let mut cac_mau = Vec::with_capacity(so_mau);
        for _ in 0..so_mau {
            let t0 = Instant::now();
            let t1 = Instant::now();                    // chỉ đo chi phí gọi
            cac_mau.push(t1.duration_since(t0).as_nanos() as u64);
        }
        cac_mau.sort_unstable();
        // Lấy TRUNG VỊ, không lấy trung bình: nhiễu hệ điều hành lệch phải mạnh.
        Self { chi_phi_ns: cac_mau[so_mau / 2] }
    }

    pub fn do_ns<F: FnOnce() -> R, R>(&self, f: F) -> (R, u64) {
        let t0 = Instant::now();
        let kq = std::hint::black_box(f());
        let tho = t0.elapsed().as_nanos() as u64;
        (kq, tho.saturating_sub(self.chi_phi_ns))
    }
}
```

Hai chi tiết: dùng **trung vị** chứ không trung bình khi hiệu chỉnh (nhiễu hệ điều hành lệch phải rất mạnh), và `saturating_sub` để tránh kết quả âm khi thao tác nhanh hơn cả chi phí đo — lúc đó bạn cần đo theo lô, không đo từng lần.
</details>
