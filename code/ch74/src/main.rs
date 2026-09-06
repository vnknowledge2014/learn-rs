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
pub struct LatencyHistogram {
    /// xo[i] đếm các giá trị trong [2^(i-1), 2^i)
    xor: Vec<u64>,
    pub tong_mau: u64,
    pub min: u64,
    pub max: u64,
    total_value: u128,
}

impl LatencyHistogram {
    pub fn new() -> Self {
        LatencyHistogram { xor: vec![0; 65], tong_mau: 0, min: u64::MAX,
                    max: 0, total_value: 0 }
    }

    #[inline]
    pub fn record(&mut self, ns: u64) {
        let i = if ns == 0 { 0 } else { 64 - ns.leading_zeros() as usize };
        self.xor[i] += 1;
        self.tong_mau += 1;
        self.total_value += ns as u128;
        if ns < self.min { self.min = ns; }
        if ns > self.max { self.max = ns; }
    }

    pub fn mean(&self) -> f64 {
        if self.tong_mau == 0 { 0.0 } else { self.total_value as f64 / self.tong_mau as f64 }
    }

    /// Cận TRÊN của xô chứa phân vị. Với thang log, sai số tương đối bị chặn
    /// trong mỗi xô — đủ tốt để phát hiện đuôi dài, vốn là mục đích chính.
    pub fn percentile(&self, p: f64) -> u64 {
        if self.tong_mau == 0 { return 0; }
        let threshold = (self.tong_mau as f64 * p).ceil().max(1.0) as u64;
        let mut accumulate = 0u64;
        for (i, &c) in self.xor.iter().enumerate() {
            accumulate += c;
            if accumulate >= threshold {
                return if i == 0 { 0 } else { (1u64 << (i - 1)) * 2 - 1 };
            }
        }
        self.max
    }

    /// Bản tóm tắt mà một kỹ sư độ trễ thật sự nhìn vào.
    pub fn tom_tat(&self) -> String {
        format!("n={} min={} p50={} p99={} p99.9={} max={} (tb={:.0})",
                self.tong_mau, self.min, self.percentile(0.50),
                self.percentile(0.99), self.percentile(0.999), self.max, self.mean())
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
pub struct SharedBuffer { pub a: AtomicUsize, pub b: AtomicUsize }

/// Đệm cho mỗi bộ đếm chiếm trọn một dòng cache riêng.
#[repr(C, align(64))]
pub struct CountHasCount { pub value: AtomicUsize, _count: [u8; DONG_CACHE - 8] }

impl CountHasCount {
    pub fn new() -> Self { CountHasCount { value: AtomicUsize::new(0), _count: [0; DONG_CACHE - 8] } }
}

#[repr(C)]
pub struct BufferSplitClose { pub a: CountHasCount, pub b: CountHasCount }

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
    o: UnsafeCell<[Option<T>; N]>,
    _dem1: [u8; DONG_CACHE],
    pos_value_record: AtomicUsize,
    _dem2: [u8; DONG_CACHE - 8],
    pos_value_read: AtomicUsize,
    _dem3: [u8; DONG_CACHE - 8],
}

// An toàn: mỗi con trỏ chỉ có ĐÚNG MỘT bên ghi vào.
unsafe impl<T: Send, const N: usize> Sync for DisruptorRing<T, N> {}
unsafe impl<T: Send, const N: usize> Send for DisruptorRing<T, N> {}

impl<T, const N: usize> DisruptorRing<T, N> {
    pub fn new() -> Self {
        assert!(N.is_power_of_two(), "sức chứa phải là luỹ thừa của 2");
        DisruptorRing {
            o: UnsafeCell::new(std::array::from_fn(|_| None)),
            _dem1: [0; DONG_CACHE],
            pos_value_record: AtomicUsize::new(0), _dem2: [0; DONG_CACHE - 8],
            pos_value_read: AtomicUsize::new(0), _dem3: [0; DONG_CACHE - 8],
        }
    }

    #[inline]
    fn chi_so(v: usize) -> usize { v & (N - 1) } // thay cho v % N

    pub fn quantity(&self) -> usize {
        self.pos_value_record.load(Ordering::Acquire) - self.pos_value_read.load(Ordering::Acquire)
    }
    pub fn rong(&self) -> bool { self.quantity() == 0 }
    pub fn day(&self) -> bool { self.quantity() == N }
    pub fn capacity(&self) -> usize { N }

    /// Gọi từ luồng SẢN XUẤT. Trả `Err` khi đầy — không bao giờ chặn,
    /// vì chặn trên đường nóng là điều cấm kỵ.
    pub fn push(&self, gt: T) -> Result<(), T> {
        let record = self.pos_value_record.load(Ordering::Relaxed); // ta là bên duy nhất ghi nó
        let doc = self.pos_value_read.load(Ordering::Acquire);
        if record - doc == N { return Err(gt); }
        unsafe { (*self.o.get())[Self::chi_so(record)] = Some(gt); }
        // Release: bảo đảm dữ liệu ghi xong TRƯỚC khi bên đọc thấy con trỏ mới
        self.pos_value_record.store(record + 1, Ordering::Release);
        Ok(())
    }

    /// Gọi từ luồng TIÊU THỤ.
    pub fn take(&self) -> Option<T> {
        let doc = self.pos_value_read.load(Ordering::Relaxed);
        let record = self.pos_value_record.load(Ordering::Acquire);
        if doc == record { return None; }
        let gt = unsafe { (*self.o.get())[Self::chi_so(doc)].take() };
        self.pos_value_read.store(doc + 1, Ordering::Release);
        gt
    }

    /// Lấy cả LÔ — mấu chốt của thông lượng cao: một lần đồng bộ cho nhiều
    /// phần tử, nên chi phí hàng rào bộ nhớ được chia đều cho cả lô.
    pub fn lay_lo(&self, toi_da: usize, ra: &mut Vec<T>) -> usize {
        let doc = self.pos_value_read.load(Ordering::Relaxed);
        let record = self.pos_value_record.load(Ordering::Acquire);
        let n = (record - doc).min(toi_da);
        for i in 0..n {
            if let Some(x) = unsafe { (*self.o.get())[Self::chi_so(doc + i)].take() } {
                ra.push(x);
            }
        }
        if n > 0 { self.pos_value_read.store(doc + n, Ordering::Release); }
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
    pub id_chain: u32,
    pub side: u8,
    pub count: [u8; 32],
}

pub struct ObjectPool<T> {
    ranh: Vec<usize>,
    o: Vec<T>,
    pub count_borrow: u64,
    pub so_lan_het_be: u64,
}

impl<T: Default + Clone> ObjectPool<T> {
    pub fn new(capacity: usize) -> Self {
        ObjectPool {
            ranh: (0..capacity).rev().collect(),
            o: vec![T::default(); capacity],
            count_borrow: 0, so_lan_het_be: 0,
        }
    }
    pub fn con_ranh(&self) -> usize { self.ranh.len() }

    /// Trả về CHỈ SỐ chứ không phải con trỏ — tránh hẳn vấn đề vòng đời.
    pub fn borrow(&mut self) -> Option<usize> {
        self.count_borrow += 1;
        match self.ranh.pop() {
            Some(i) => Some(i),
            None => { self.so_lan_het_be += 1; None }
        }
    }
    pub fn tra(&mut self, i: usize) { self.ranh.push(i); }
    pub fn view(&self, i: usize) -> &T { &self.o[i] }
    pub fn fix(&mut self, i: usize) -> &mut T { &mut self.o[i] }
}

// ============================================================================
// 5. BỐ TRÍ BỘ NHỚ — mảng-của-struct vs struct-của-mảng
// ============================================================================

/// Mảng-của-struct (AoS): mỗi bản ghi liền mạch. Tốt khi đọc TẤT CẢ trường.
/// Trường được xếp theo kích thước GIẢM DẦN để trình biên dịch không phải đệm.
#[derive(Clone, Copy, Default)]
pub struct QuoteAoS {
    pub price_buy: i64,
    pub price_sell: i64,
    pub timestamp: u64,
    pub co: u64,
    pub id_chain: u32,
    pub qty_buy: u32,
    pub qty_sell: u32,
    pub count: u32,
}

/// Struct-của-mảng (SoA): mỗi trường một mảng riêng. Tốt khi chỉ đọc MỘT
/// trường trên nhiều bản ghi — CPU nạp một dòng cache 64 byte là được 8 giá
/// trị đều có ích, thay vì 8 byte có ích trên 40 byte rác.
#[derive(Default)]
pub struct QuoteTableSoA {
    pub id_chain: Vec<u32>,
    pub price_buy: Vec<i64>,
    pub price_sell: Vec<i64>,
    pub qty_buy: Vec<u32>,
    pub qty_sell: Vec<u32>,
    pub timestamp: Vec<u64>,
}

impl QuoteTableSoA {
    pub fn new(n: usize) -> Self {
        QuoteTableSoA {
            id_chain: vec![0; n], price_buy: vec![0; n], price_sell: vec![0; n],
            qty_buy: vec![0; n], qty_sell: vec![0; n], timestamp: vec![0; n],
        }
    }
    pub fn quantity(&self) -> usize { self.id_chain.len() }

    /// Quét chỉ trường `price_buy` — đây là chỗ SoA thắng đậm.
    pub fn total_price_buy(&self) -> i128 { self.price_buy.iter().map(|&x| x as i128).sum() }

    /// Số byte thực sự phải kéo từ RAM để quét một trường 8 byte.
    pub fn bytes_to_read_one_field(&self) -> usize { self.quantity() * 8 }
}

pub fn bytes_to_read_one_field_aos(n: usize) -> usize {
    // Phải kéo cả bản ghi dù chỉ cần 8 byte
    n * std::mem::size_of::<QuoteAoS>()
}

// ============================================================================
// 6. NGÂN SÁCH ĐỘ TRỄ — chia nhỏ "tick-to-trade"
// ============================================================================

#[derive(Debug, Clone, PartialEq)]
pub struct LatencyStage { pub name: String, pub ns: u64 }

#[derive(Debug, PartialEq)]
pub struct LatencyBudget { pub chang: Vec<LatencyStage>, pub tran_ns: u64 }

impl LatencyBudget {
    pub fn tong(&self) -> u64 { self.chang.iter().map(|c| c.ns).sum() }
    pub fn set_level_spend(&self) -> bool { self.tong() <= self.tran_ns }
    /// Chặng tốn nhất — nơi DUY NHẤT đáng bỏ công tối ưu.
    pub fn nut_that_co_chai(&self) -> Option<&LatencyStage> {
        self.chang.iter().max_by_key(|c| c.ns)
    }
    /// Định luật Amdahl: tăng tốc tối đa nếu chặng nghẽn cổ chai thành 0.
    pub fn max_speedup_if_node_removed(&self) -> f64 {
        match self.nut_that_co_chai() {
            Some(n) if self.tong() > n.ns => self.tong() as f64 / (self.tong() - n.ns) as f64,
            _ => f64::INFINITY,
        }
    }
}

/// Sinh mẫu độ trễ tất định có ĐUÔI DÀI — giống hệt hệ thống thật:
/// phần lớn nhanh, thỉnh thoảng một cú chậm gấp hàng trăm lần.
pub fn gen_mau_latency(n: usize, hat_giong: u64) -> Vec<u64> {
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
    let mut bd = LatencyHistogram::new();
    for x in gen_mau_latency(1_000_000, 42) { bd.record(x); }
    println!("   {}", bd.tom_tat());
    println!("   Trung bình {:.0} ns nghe rất đẹp…", bd.mean());
    println!("   …nhưng 1 trên 1000 lệnh rơi vào dải tới {} ns, và cú chậm nhất là {} ns",
             bd.percentile(0.999), bd.max);
    println!("   — gấp {:.0} lần trung bình. (Phân vị là cận TRÊN của xô log.)",
             bd.max as f64 / bd.mean());
    println!("   Trong giao dịch, chính CÁI ĐUÔI đó là lúc bạn mất tiền.");

    println!("\n2. CHIA SẺ GIẢ — kích thước quyết định tốc độ");
    println!("   BoDemChungDong: {} byte (hai bộ đếm CÙNG một dòng cache)",
             std::mem::size_of::<SharedBuffer>());
    println!("   BoDemTachDong : {} byte (mỗi bộ đếm một dòng riêng)",
             std::mem::size_of::<BufferSplitClose>());
    println!("   → Tốn thêm {} byte để tránh ping-pong dòng cache giữa hai lõi.",
             std::mem::size_of::<BufferSplitClose>() - std::mem::size_of::<SharedBuffer>());

    println!("\n3. VÒNG ĐỆM DISRUPTOR");
    let v: DisruptorRing<u64, 1024> = DisruptorRing::new();
    for i in 0..1024 { v.push(i).unwrap(); }
    println!("   Đẩy 1024 phần tử → đầy: {} · đẩy thêm → bị từ chối: {}",
             v.day(), v.push(9999).is_err());
    let mut lo = Vec::new();
    let n = v.lay_lo(256, &mut lo);
    println!("   Lấy một lô 256 → được {} phần tử, còn lại {}", n, v.quantity());
    println!("   Chỉ số dùng phép AND: 1030 & 1023 = {} (thay cho phép chia)", 1030usize & 1023);

    println!("\n4. BỂ ĐỐI TƯỢNG");
    let mut be: ObjectPool<OrderPacket> = ObjectPool::new(4);
    let cac_i: Vec<usize> = (0..4).filter_map(|_| be.borrow()).collect();
    println!("   Mượn 4/4 → còn rảnh {} · mượn thêm → {:?}", be.con_ranh(), be.borrow());
    be.tra(cac_i[0]);
    println!("   Trả 1 lại → còn rảnh {} · số lần hết bể = {}", be.con_ranh(), be.so_lan_het_be);
    println!("   Một GoiLenh = {} byte — vừa đúng một dòng cache",
             std::mem::size_of::<OrderPacket>());

    println!("\n5. BỐ TRÍ BỘ NHỚ — AoS vs SoA khi quét MỘT trường");
    let n = 100_000;
    println!("   Một bản ghi AoS = {} byte", std::mem::size_of::<QuoteAoS>());
    println!("   Quét {} bản ghi chỉ để lấy `gia_mua`:", n);
    println!("     AoS phải kéo {:>9} byte từ RAM", bytes_to_read_one_field_aos(n));
    println!("     SoA chỉ kéo  {:>9} byte", QuoteTableSoA::new(n).bytes_to_read_one_field());
    println!("   → SoA đọc ít hơn {:.1}× — và đó là băng thông RAM, thứ đắt nhất.",
             bytes_to_read_one_field_aos(n) as f64 / (n * 8) as f64);

    println!("\n6. NGÂN SÁCH ĐỘ TRỄ TICK-TO-TRADE");
    let ns = LatencyBudget {
        tran_ns: 5_000,
        chang: vec![
            LatencyStage { name: "Card mạng → bộ nhớ".into(), ns: 800 },
            LatencyStage { name: "Phân tích gói tin".into(), ns: 150 },
            LatencyStage { name: "Cập nhật sổ lệnh".into(), ns: 400 },
            LatencyStage { name: "Chiến lược quyết định".into(), ns: 250 },
            LatencyStage { name: "Kiểm tra rủi ro".into(), ns: 120 },
            LatencyStage { name: "Tuần tự hoá lệnh".into(), ns: 180 },
            LatencyStage { name: "Gọi hệ thống gửi".into(), ns: 1_500 },
        ],
    };
    for c in &ns.chang {
        let percent = c.ns as f64 * 100.0 / ns.tong() as f64;
        println!("   {:<26} {:>5} ns  {:>5.1}%  {}",
                 c.name, c.ns, percent, "#".repeat((percent / 2.0) as usize));
    }
    println!("   Tổng {} ns / trần {} ns → {}",
             ns.tong(), ns.tran_ns, if ns.set_level_spend() { "ĐẠT" } else { "TRƯỢT" });
    println!("   Nút thắt: {} · xoá hẳn nó cũng chỉ nhanh được {:.2}×",
             ns.nut_that_co_chai().unwrap().name, ns.max_speedup_if_node_removed());
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
        assert_eq!(b.tong_mau, 0);
        assert_eq!(b.mean(), 0.0);
        assert_eq!(b.percentile(0.99), 0);
    }

    #[test]
    fn histogram_tracks_min_max_and_mean() {
        let mut b = LatencyHistogram::new();
        for x in [10u64, 20, 30, 40] { b.record(x); }
        assert_eq!(b.min, 10);
        assert_eq!(b.max, 40);
        assert_eq!(b.mean(), 25.0);
        assert_eq!(b.tong_mau, 4);
    }

    #[test]
    fn percentiles_are_monotonic() {
        let mut b = LatencyHistogram::new();
        for x in gen_mau_latency(10_000, 7) { b.record(x); }
        let (p50, p90, p99, p999) = (b.percentile(0.5), b.percentile(0.9),
                                     b.percentile(0.99), b.percentile(0.999));
        assert!(p50 <= p90 && p90 <= p99 && p99 <= p999,
                "phân vị phải tăng dần: {} {} {} {}", p50, p90, p99, p999);
        assert!(p999 <= b.max);
    }

    #[test]
    fn percentiles_bracket_the_true_value() {
        // Cận trên của xô phải THỰC SỰ là cận trên: không được báo thấp hơn
        // giá trị thật, nếu không ta sẽ tưởng hệ thống nhanh hơn thực tế.
        let mut b = LatencyHistogram::new();
        for x in [1u64, 2, 3, 100, 1000] { b.record(x); }
        assert!(b.percentile(1.0) >= 1000);
        assert!(b.percentile(0.8) >= 100, "80% mẫu ≤ 100, cận phải ≥ 100");
    }

    #[test]
    fn a_long_tail_makes_the_mean_lie() {
        // Đây là bài học trung tâm của chương: 99% mẫu ở 200–300 ns, nhưng
        // 0.1% ở 50 µs kéo trung bình lên và che mất phân bố thật.
        let mut b = LatencyHistogram::new();
        for x in gen_mau_latency(100_000, 42) { b.record(x); }

        // Phân bố thật: p50 ≈ 250 ns, p99 ≈ 299 ns, p99.9 ≈ 2.5 µs, max ≈ 60 µs.
        // Chú ý p99 vẫn NHANH — phải soi tới p99.9 mới thấy dấu vết đuôi,
        // và tới giá trị lớn nhất mới thấy hết mức độ.
        assert!(b.percentile(0.5) < 512, "phân vị 50 phải nằm ở vùng nhanh");
        assert!(b.percentile(0.99) < 512, "ngay cả p99 vẫn nhanh — đuôi còn ẩn kỹ hơn thế");
        assert!(b.percentile(0.999) > 2_000,
                "tới p99.9 mới lộ ra đuôi, thực tế {}", b.percentile(0.999));
        assert!(b.max > 50_000, "giá trị lớn nhất mới cho thấy hết mức độ");

        // Đây là con số đắt giá nhất: trung bình ~326 ns che mất một cú
        // gần 60 µs, tức chậm gấp gần 200 lần.
        assert!(b.max as f64 > b.mean() * 100.0,
                "max {} so với trung bình {:.0} — trung bình che giấu đúng thứ giết bạn",
                b.max, b.mean());
    }

    #[test]
    fn recording_zero_and_max_is_safe() {
        let mut b = LatencyHistogram::new();
        b.record(0);
        b.record(u64::MAX);
        assert_eq!(b.tong_mau, 2);
        assert_eq!(b.min, 0);
        assert_eq!(b.max, u64::MAX);
    }

    // ---------- Chia sẻ giả ----------
    #[test]
    fn padded_counter_owns_a_whole_cache_line() {
        assert_eq!(std::mem::size_of::<CountHasCount>(), DONG_CACHE);
        assert_eq!(std::mem::align_of::<CountHasCount>(), DONG_CACHE,
                   "phải căn theo dòng cache, không chỉ đủ kích thước");
    }

    #[test]
    fn padded_counters_never_share_a_line() {
        let b = BufferSplitClose { a: CountHasCount::new(), b: CountHasCount::new() };
        let dc_a = &b.a as *const _ as usize;
        let dc_b = &b.b as *const _ as usize;
        assert!(dc_b - dc_a >= DONG_CACHE,
                "hai bộ đếm cách nhau {} byte, phải ít nhất {}", dc_b - dc_a, DONG_CACHE);
        // Ngược lại, phiên bản không đệm thì chúng nằm sát nhau
        let c = SharedBuffer { a: AtomicUsize::new(0), b: AtomicUsize::new(0) };
        let ca = &c.a as *const _ as usize;
        let cb = &c.b as *const _ as usize;
        assert!(cb - ca < DONG_CACHE,
                "đây chính là chia sẻ giả: cách nhau chỉ {} byte", cb - ca);
    }

    // ---------- Vòng Disruptor ----------
    #[test]
    fn ring_is_fifo() {
        let v: DisruptorRing<u32, 8> = DisruptorRing::new();
        for i in 0..5 { v.push(i).unwrap(); }
        for i in 0..5 { assert_eq!(v.take(), Some(i)); }
        assert_eq!(v.take(), None);
    }

    #[test]
    fn ring_uses_full_capacity_without_wasting_a_slot() {
        // Hàng đợi vòng thường phải bỏ một ô để phân biệt rỗng/đầy.
        // Con trỏ tăng mãi giúp ta dùng trọn N ô.
        let v: DisruptorRing<u32, 8> = DisruptorRing::new();
        for i in 0..8 { assert!(v.push(i).is_ok(), "phải nhận đủ 8 phần tử"); }
        assert!(v.day());
        assert_eq!(v.quantity(), 8);
        assert_eq!(v.push(99), Err(99));
    }

    #[test]
    fn ring_wraps_correctly_over_many_laps() {
        let v: DisruptorRing<u64, 4> = DisruptorRing::new();
        for i in 0..1000u64 {
            v.push(i).unwrap();
            assert_eq!(v.take(), Some(i), "chỉ số phải quấn đúng qua biên mảng");
        }
        assert!(v.rong());
    }

    #[test]
    fn empty_ring_returns_none_safely() {
        let v: DisruptorRing<u8, 16> = DisruptorRing::new();
        assert_eq!(v.take(), None);
        assert!(v.rong() && !v.day());
        assert_eq!(v.quantity(), 0);
    }

    #[test]
    fn batch_take_returns_the_requested_count() {
        let v: DisruptorRing<u32, 64> = DisruptorRing::new();
        for i in 0..50 { v.push(i).unwrap(); }
        let mut ra = Vec::new();
        assert_eq!(v.lay_lo(20, &mut ra), 20);
        assert_eq!(ra, (0..20).collect::<Vec<u32>>());
        assert_eq!(v.quantity(), 30);
        // Xin nhiều hơn số có thì chỉ lấy được số có
        let mut ra2 = Vec::new();
        assert_eq!(v.lay_lo(1000, &mut ra2), 30);
        assert!(v.rong());
    }

    #[test]
    fn batch_take_on_empty_ring_returns_zero() {
        let v: DisruptorRing<u32, 8> = DisruptorRing::new();
        let mut ra = Vec::new();
        assert_eq!(v.lay_lo(10, &mut ra), 0);
        assert!(ra.is_empty());
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
        assert_eq!(b.con_ranh(), 3);
        let a = b.borrow().unwrap();
        let c = b.borrow().unwrap();
        assert_ne!(a, c, "hai lần mượn phải ra hai ô khác nhau");
        assert_eq!(b.con_ranh(), 1);
        b.tra(a);
        assert_eq!(b.con_ranh(), 2);
    }

    #[test]
    fn exhausted_pool_returns_none_instead_of_allocating() {
        // Điểm mấu chốt: thà từ chối còn hơn cấp phát heap trên đường nóng.
        let mut b: ObjectPool<u32> = ObjectPool::new(2);
        assert!(b.borrow().is_some());
        assert!(b.borrow().is_some());
        assert!(b.borrow().is_none());
        assert_eq!(b.so_lan_het_be, 1, "phải ĐẾM số lần hết bể để còn chỉnh kích thước");
        assert_eq!(b.count_borrow, 3);
    }

    #[test]
    fn order_packet_is_exactly_one_cache_line() {
        assert_eq!(std::mem::size_of::<OrderPacket>(), DONG_CACHE,
                   "bản ghi trên đường nóng nên vừa một dòng cache, không hơn");
    }

    #[test]
    fn a_returned_slot_is_reusable_immediately() {
        let mut b: ObjectPool<u64> = ObjectPool::new(2);
        let i = b.borrow().unwrap();
        *b.fix(i) = 12345;
        assert_eq!(*b.view(i), 12345);
        b.tra(i);
        let j = b.borrow().unwrap();
        assert_eq!(i, j, "ô vừa trả phải được tái dùng ngay — nó còn NÓNG trong cache");
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
        for i in 0..5 { t.price_buy[i] = (i as i64 + 1) * 100; }
        assert_eq!(t.total_price_buy(), 100 + 200 + 300 + 400 + 500);
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
    fn nanos_mau() -> LatencyBudget {
        LatencyBudget {
            tran_ns: 5_000,
            chang: vec![
                LatencyStage { name: "mang".into(), ns: 800 },
                LatencyStage { name: "phan_tich".into(), ns: 150 },
                LatencyStage { name: "goi_he_thong".into(), ns: 1_500 },
            ],
        }
    }

    #[test]
    fn budget_computes_total_and_bottleneck() {
        let ns = nanos_mau();
        assert_eq!(ns.tong(), 2_450);
        assert!(ns.set_level_spend());
        assert_eq!(ns.nut_that_co_chai().unwrap().name, "goi_he_thong");
    }

    #[test]
    fn amdahl_bounds_the_speedup() {
        let ns = nanos_mau();
        // Xoá hẳn chặng 1500 ns khỏi tổng 2450 ns → còn 950 ns
        let expected = 2_450.0 / 950.0;
        assert!((ns.max_speedup_if_node_removed() - expected).abs() < 1e-9);
        assert!(ns.max_speedup_if_node_removed() < 3.0,
                "kể cả xoá sạch nút thắt cũng chỉ nhanh được ~2.6× — đó là định luật Amdahl");
    }

    #[test]
    fn over_budget_is_reported_as_a_miss() {
        let ns = LatencyBudget {
            tran_ns: 1_000,
            chang: vec![LatencyStage { name: "cham".into(), ns: 9_999 }],
        };
        assert!(!ns.set_level_spend());
    }

    #[test]
    fn a_single_stage_budget_allows_unbounded_speedup() {
        let ns = LatencyBudget {
            tran_ns: 100,
            chang: vec![LatencyStage { name: "tat_ca".into(), ns: 500 }],
        };
        assert!(ns.max_speedup_if_node_removed().is_infinite(),
                "xoá chặng duy nhất thì thời gian còn 0");
    }

    // ---------- Sinh mẫu ----------
    #[test]
    fn sample_generation_is_deterministic() {
        assert_eq!(gen_mau_latency(100, 5), gen_mau_latency(100, 5));
        assert_ne!(gen_mau_latency(100, 5), gen_mau_latency(100, 6));
    }

    #[test]
    fn samples_span_exactly_three_latency_bands() {
        let m = gen_mau_latency(100_000, 1);
        let fast = m.iter().filter(|&&x| x < 1_000).count();
        let vua = m.iter().filter(|&&x| (1_000..10_000).contains(&x)).count();
        let cham = m.iter().filter(|&&x| x >= 10_000).count();
        assert!(fast > 95_000, "~99% phải nhanh, thực tế {}", fast);
        assert!(vua > 0 && cham > 0, "phải có cả đuôi vừa và đuôi dài");
        assert_eq!(fast + vua + cham, m.len());
    }
}
