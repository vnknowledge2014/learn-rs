# Chương 80: Hiệu năng CPU sâu — Cache, Dự đoán rẽ nhánh, ILP & SIMD (LeetCPU)

## Giới thiệu & Mục tiêu học tập

Chương này lấy cảm hứng từ thể loại bài tập của **leetcpu.com**: những bài toán mà đáp án đúng phụ thuộc vào việc bạn hiểu CPU thật hoạt động thế nào, chứ không phải độ phức tạp big-O.

Điểm khởi đầu là một sự thật gây sốc:

> **Một lần truy cập RAM tốn khoảng 300 owner kỳ. Trong 300 owner kỳ đó, CPU có thể làm hơn 1000 phép cộng.**

Nghĩa là **bố cục bộ nhớ quan trọng hơn số phép tính**. Một thuật toán "kém hơn" về big-O nhưng thân thiện với cache thường nhanh hơn nhiều lần trong thực tế.

| Chủ đề | Bài học cốt lõi |
|---|---|
| Phân cấp bộ nhớ | L1 ~4 owner kỳ, RAM ~300 owner kỳ — chênh 75 lần |
| Cục bộ | Duyệt theo hàng nhanh hơn theo cột hàng chục lần |
| Chia khối | Cùng số phép tính, ít trượt cache hơn hàng chục lần |
| Dự đoán rẽ nhánh | Dữ liệu đã sắp xếp chạy nhanh hơn dữ liệu ngẫu nhiên |
| ILP | Nhiều biến tích luỹ phá chuỗi phụ thuộc |
| SIMD | Một lệnh, nhiều dữ liệu — nhưng phải xử lý phần dư |

---

## Hình tượng hóa đời sống

```
┌──────────────────────────────────────────────────────────────────────────────┐
│  PHÂN CẤP BỘ NHỚ = TỦ SÁCH, GIÁ SÁCH, THƯ VIỆN, KHO LƯU TRỮ                │
│                                                                              │
│    Thanh ghi   1 owner kỳ    ~1 KB     trong tầm tay                          │
│    L1          4 owner kỳ    32 KB     trên bàn                               │
│    L2         12 owner kỳ   256 KB     kệ sau lưng                            │
│    L3         40 owner kỳ    16 MB     phòng bên cạnh                         │
│    RAM       300 owner kỳ    32 GB     ĐI THƯ VIỆN THÀNH PHỐ                  │
│                                                                              │
│   Tỉ lệ L1 : RAM = 1 : 75.                                                  │
│   Nếu L1 là "với tay lấy" (1 giây) thì RAM là "đi bộ 75 giây".              │
│                                                                              │
│  DUYỆT HÀNG vs DUYỆT CỘT (id trận 1024×1024 f64)                           │
│                                                                              │
│    Theo hàng:  ████████░░░░░░░░  nạp 1 dòng cache = dùng được 8 phần tử     │
│                → 1 lần trượt cho mỗi 8 phần tử                             │
│                                                                              │
│    Theo cột:   █░░░░░░░ █░░░░░░░  nạp 1 dòng cache = dùng được 1 phần tử    │
│                → 1 lần trượt cho MỖI phần tử → chậm gấp ~8 lần trở lên     │
│                                                                              │
│   Cùng số phép tính. Cùng độ phức tạp. Khác nhau một bậc về tốc độ.         │
│                                                                              │
│  DỰ ĐOÁN RẼ NHÁNH = ĐOÁN TRƯỚC XE SẼ RẼ HƯỚNG NÀO                          │
│                                                                              │
│    Mảng ĐÃ SẮP XẾP:  0 0 0 0 0 1 1 1 1 1  → đoán đúng ~99%                │
│    Mảng NGẪU NHIÊN:  0 1 1 0 1 0 0 1 0 1  → đoán đúng ~50%                │
│                                                                              │
│    Mỗi lần đoán sai = xả toàn bộ đường ống = ~15 owner kỳ mất trắng.          │
│    → Sắp xếp mảng trước rồi lọc có thể NHANH HƠN lọc trực tiếp,            │
│      dù sắp xếp tốn O(n log n).                                            │
│                                                                              │
│  ILP = CHUỖI PHỤ THUỘC LÀ KẺ THÙ                                            │
│                                                                              │
│    1 biến tích luỹ:  s += a[0]; s += a[1]; ...  ← mỗi phép PHẢI chờ phép   │
│                                                    trước → 4 owner kỳ/phần tử │
│                                                                              │
│    4 biến tích luỹ:  s0 += a[0]; s1 += a[1];   ← 4 chuỗi ĐỘC LẬP           │
│                      s2 += a[2]; s3 += a[3];      chạy song song            │
│                                                → ~1 owner kỳ/phần tử         │
└──────────────────────────────────────────────────────────────────────────────┘
```

---

## Khái niệm & Cơ chế kỹ thuật chuyên sâu

### 1. Dòng cache là đơn vị thật của bộ nhớ

CPU không bao giờ nạp một byte. Nó nạp **một dòng cache 64 byte**. Nghĩa là đọc một `f64` (8 byte) thì 56 byte hàng xóm cũng được nạp theo — miễn phí.

Toàn bộ nghệ thuật tối ưu bộ nhớ nằm ở việc **dùng hết 56 byte đó**. Duyệt theo hàng thì dùng hết. Duyệt theo cột thì vứt đi.

Đây cũng là lý do `Vec<Struct>` và `Struct<Vec>` (AoS vs SoA ở chương 74) khác nhau nhiều đến vậy: chúng quyết định 56 byte kia chứa gì.

### 2. Cache tập hợp liên kết và vì sao có "xung đột"

Cache không phải hoàn toàn liên kết — nó chia thành các **tập** (set), và mỗi địa chỉ chỉ vào được đúng một tập. Với cache 8-way, mỗi tập chứa 8 dòng.

Hệ quả: nếu bạn truy cập nhiều địa chỉ cùng ánh xạ vào một tập, chúng đá nhau ra dù cache còn trống chỗ khác. Đây là **trượt do xung đột**, và nó xuất hiện đúng khi bước nhảy là luỹ thừa của 2 — tình huống rất hay gặp với id trận vuông.

Cách chữa kinh điển: **đệm** thêm một phần tử vào mỗi hàng, biến bước nhảy 1024 thành 1025. Một phần tử thừa, hết xung đột.

### 3. Chia khối: cùng phép tính, ít trượt hơn

Nhân id trận ngây thơ có ba vòng lặp lồng nhau. Với id trận lớn hơn cache, id trận B bị nạp lại **hoàn toàn** cho mỗi hàng của A.

Chia khối chia bài toán thành các khối con vừa với L1. Mỗi khối được nạp một lần rồi dùng hết trước khi bị đẩy ra.

Con số quan trọng: cùng `2n³` phép tính, nhưng số lần trượt cache giảm từ `O(n³)` xuống `O(n³/√M)` với M là kích thước cache. Với id trận 512×512, đó là chênh lệch 5–10 lần thời gian chạy.

### 4. Dự đoán rẽ nhánh và mã không rẽ nhánh

Bộ dự đoán 2-bit bão hoà có bốn trạng thái: chắc chắn không nhảy → có thể không → có thể nhảy → chắc chắn nhảy. Cần **hai** lần sai liên tiếp mới đổi hướng dự đoán, nên nó chịu được nhiễu tốt.

Nhưng với dữ liệu ngẫu nhiên, không bộ dự đoán nào cứu được — tỉ lệ đúng về 50%, và mỗi lần sai mất khoảng 15 owner kỳ.

Giải pháp là **mã không rẽ nhánh**: thay `if x > t { s += x }` bằng `s += x * (x > t) as i64`. Không có nhánh nên không có dự đoán sai. Với dữ liệu ngẫu nhiên, phiên bản không nhánh nhanh hơn rõ rệt; với dữ liệu đã sắp xếp thì phiên bản có nhánh lại thắng, vì dự đoán gần như luôn đúng.

Bài học: **không có phiên bản nào luôn tốt hơn**. Phải biết dữ liệu của mình.

### 5. ILP: vì sao 4 biến tích luỹ nhanh hơn 1

CPU hiện đại thực thi ngoài thứ tự và có nhiều đơn vị tính toán. Nhưng chúng không thể phá vỡ **chuỗi phụ thuộc dữ liệu**: nếu `s += a[i]` phải chờ `s` từ lần trước, thì tốc độ bị chặn bởi độ trễ của phép cộng (khoảng 4 owner kỳ cho số thực).

Chia thành 4 biến tích luỹ tạo ra 4 chuỗi độc lập. CPU chạy cả 4 song song, và thông lượng tăng gần 4 lần.

Lưu ý quan trọng với số thực: cộng dấu phẩy động **không có tính kết hợp**, nên `(a+b)+c ≠ a+(b+c)`. Kết quả của phiên bản 4 biến sẽ khác một chút. Đó là lý do trình biên dịch **không tự làm** phép biến đổi này trừ khi bạn cho phép rõ ràng.

### 6. SIMD và phần dư

SIMD xử lý nhiều phần tử bằng một lệnh: AVX2 làm 4 `f64` cùng lúc, AVX-512 làm 8.

Ba điều kiện để SIMD thực sự nhanh:
- **Căn chỉnh**: dữ liệu nên căn theo 32 byte (AVX2).
- **Liên tục**: SIMD đọc khối liền mạch, không nhảy cóc.
- **Không phụ thuộc**: mỗi phần tử tính độc lập.

Và luôn có **phần dư**: mảng 1000 phần tử với vector 4 phần tử cho 250 vector chẵn; mảng 1001 phần tử cho 250 vector và 1 phần tử lẻ phải xử lý riêng. Quên phần dư là lỗi phổ biến nhất khi viết mã SIMD bằng tay.


### Bản đồ 22 bài của LeetCPU sang chương này

Danh sách dưới đây được **lấy trực tiếp từ leetcpu.com** (thu thập ngày 05/09/2026). Nền tảng đó chạy mã C của bạn trên **ChampSim** — bộ mô phỏng vi kiến trúc chính xác theo owner kỳ, 200 triệu lệnh — rồi trả về IPC, MPKI và số liệu bộ dự đoán rẽ nhánh. Chuỗi công cụ của họ là `gcc` → `objdump` → vết Intel PIN → ChampSim → bảng chỉ số.

Chúng ta không mô phỏng vi kiến trúc ở đây; chúng ta cài **cùng những kỹ thuật đó bằng Rust** và đo bằng đồng hồ thật. Bốn nhóm của họ ánh xạ đúng vào bốn phần của chương này.

| # | Bài | Mức | Nhóm | Kỹ thuật tương ứng trong chương |
|---|---|---|---|---|
| 1 | Stable Partition for Predictable Branches | Dễ | Dự đoán rẽ nhánh | Sắp xếp trước để dự đoán đúng |
| 10 | Score Window — Remove Unpredictable Branches | Dễ | Dự đoán rẽ nhánh | Mã không rẽ nhánh |
| 20 | Masked SAXPY — Remove Branches | Vừa | Dự đoán rẽ nhánh | Mặt nạ + SIMD |
| 22 | Grade Bands — Replace Nested Branches with a LUT | Dễ | Dự đoán rẽ nhánh | Bảng tra thay chuỗi `if` |
| 2 | Matrix Multiply — Cache Tiling | Dễ | Cục bộ cache | `blocked_matmul` |
| 8 | Particle Score — Repack Structs (AoS→SoA) | Vừa | Cục bộ cache | AoS vs SoA (ch74) |
| 9 | Image Blur — Tile the Working Set | Vừa | Cục bộ cache | Chia khối cho stencil |
| 11 | ECE Lab — 2D Jacobi Stencil Cache Blocking | Vừa | Cục bộ cache | Chia khối; xem thêm ch81 |
| 14 | 3D Array — Fix Loop Order | Dễ | Cục bộ cache | Hoán vị vòng lặp |
| 15 | Binary Search — Eytzinger Layout | Khó | Cục bộ cache | Bố cục BFS thân thiện cache |
| 17 | 2D Grid Sum — Fix Column-First Access | Vừa | Cục bộ cache | `row_major_scan` vs `col_major_scan` |
| 19 | Gather Reordering — Cluster by Cache Region | Vừa | Cục bộ cache | Gom truy cập theo vùng |
| 3 | Reduction Tree — Break Dependency Chains | Vừa | ILP | Nhiều biến tích luỹ |
| 7 | Bitset Scan — Scalar Loops to Throughput | Dễ | ILP | `popcount`, thông lượng |
| 13 | Histogram — Break Write Dependency Chains | Vừa | ILP | Xen kẽ ô đếm |
| 16 | Streaming Computation — Multiple Accumulators | Dễ | ILP | `coalescing_analysis` |
| 18 | SAXPY — Unlock Auto-Vectorization | Dễ | ILP | `simd_analysis` |
| 21 | Dot Product — Eight Accumulators | Dễ | ILP | Nhiều biến tích luỹ |
| 4 | Pointer Chasing — Recover Memory Parallelism | Vừa | Song song bộ nhớ | Phá chuỗi đuổi con trỏ |
| 6 | Strided Sum — Hide Latency with Prefetch | Vừa | Song song bộ nhớ | Nạp trước thủ công |
| 12 | Irregular Gather — Prefetch Ahead | Vừa | Song song bộ nhớ | Nạp trước cho truy cập ngẫu nhiên |
| 5 | Bottleneck Triage — Diagnose and Fix | Khó | Chẩn đoán | Bài tổng hợp: đo trước, sửa sau |

Phân bố nhóm: cục bộ cache 8 bài, ILP 6, dự đoán rẽ nhánh 4, song song bộ nhớ 3, chẩn đoán 1. Nói cách khác, **hơn một phần ba bài tập của một nền tảng luyện hiệu năng CPU là về bố cục bộ nhớ** — đúng như luận điểm mở đầu chương này.

---

## Mã nguồn minh họa thực chiến

Chạy bằng `cargo run -p ch80`, kiểm thử bằng `cargo test -p ch80`.

```rust
#![allow(dead_code)]
//! Chương 80 — Kỹ nghệ hiệu năng CPU: phân cấp bộ nhớ, cục bộ cache, dự đoán
//! nhánh, song song mức lệnh, và mã không nhánh.
//!
//! Theo compute thần các bài tập của [LeetCPU](https://www.leetcpu.com/) — nền
//! tảng luyện hiệu năng CPU có mô phỏng vi kiến trúc phản hồi. Ở đây ta ĐẾM
//! số lần trượt cache và dự đoán sai bằng mô phỏng tất định, thay vì đo đồng
//! hồ treo tường — nhờ vậy kết quả tái lập được và kiểm thử được.

use std::collections::HashMap;

// ============================================================================
// 1. PHÂN CẤP BỘ NHỚ — những con số cần thuộc lòng
// ============================================================================

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum UpMemory { ThanhGhi, L1, L2, L3, Ram, SsdNvme, DiaQuay }

impl UpMemory {
    /// Độ trễ tính bằng CHU KỲ CPU. Cách nhìn này quan trọng hơn nano-giây:
    /// nó cho biết CPU phải ngồi chơi bao nhiêu nhịp.
    pub fn period(self) -> u64 {
        match self {
            UpMemory::ThanhGhi => 1,
            UpMemory::L1 => 4,
            UpMemory::L2 => 12,
            UpMemory::L3 => 40,
            UpMemory::Ram => 200,
            UpMemory::SsdNvme => 200_000,
            UpMemory::DiaQuay => 20_000_000,
        }
    }
    pub fn name(self) -> &'static str {
        match self {
            UpMemory::ThanhGhi => "Thanh ghi", UpMemory::L1 => "Cache L1",
            UpMemory::L2 => "Cache L2", UpMemory::L3 => "Cache L3",
            UpMemory::Ram => "RAM", UpMemory::SsdNvme => "SSD NVMe",
            UpMemory::DiaQuay => "Đĩa quay",
        }
    }
    pub fn all() -> [UpMemory; 7] {
        [UpMemory::ThanhGhi, UpMemory::L1, UpMemory::L2, UpMemory::L3,
         UpMemory::Ram, UpMemory::SsdNvme, UpMemory::DiaQuay]
    }
}

pub const BYTE_MOI_DONG_CACHE: usize = 64;

// ============================================================================
// 2. MÔ PHỎNG CACHE LIÊN KẾT TẬP HỢP
// ============================================================================
// Cache thật không phải "có hay không có" — nó chia thành TẬP HỢP, mỗi tập
// chứa vài ĐƯỜNG. Địa chỉ quyết định tập nào; trong tập thì thay theo LRU.

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct CacheStats {
    pub num_access_cap: u64,
    pub num_duplicate: u64,
    pub slip_count: u64,
    /// Trượt vì lần đầu chạm tới — không tránh được.
    pub compulsory_miss: u64,
    /// Trượt vì cache quá nhỏ hoặc bị đá ra — CÓ THỂ tránh được.
    pub capacity_miss: u64,
}

impl CacheStats {
    pub fn ratio_duplicate(&self) -> f64 {
        if self.num_access_cap == 0 { 0.0 } else { self.num_duplicate as f64 / self.num_access_cap as f64 }
    }
    /// Tổng owner kỳ phải trả — thước đo thật sự, không phải số lần trượt.
    pub fn total_period(&self) -> u64 {
        self.num_duplicate * UpMemory::L1.period() + self.slip_count * UpMemory::Ram.period()
    }
}

pub struct CacheSim {
    pub so_tap: usize,
    pub positive_count: usize,
    /// tập → danh sách (thẻ, dấu thời gian dùng gần nhất), dài tối đa `positive_count`
    tap: Vec<Vec<(u64, u64)>>,
    seen: std::collections::HashSet<u64>,
    clock: u64,
    pub account: CacheStats,
}

impl CacheSim {
    /// `kich_thuoc_byte` là tổng dung lượng; `positive_count` là số đường mỗi tập.
    pub fn new(kich_thuoc_byte: usize, positive_count: usize) -> Self {
        let so_dong = kich_thuoc_byte / BYTE_MOI_DONG_CACHE;
        let so_tap = (so_dong / positive_count).max(1);
        CacheSim {
            so_tap, positive_count,
            tap: vec![Vec::with_capacity(positive_count); so_tap],
            seen: std::collections::HashSet::new(),
            clock: 0,
            account: CacheStats { num_access_cap: 0, num_duplicate: 0, slip_count: 0,
                               compulsory_miss: 0, capacity_miss: 0 },
        }
    }

    /// Truy cập một địa chỉ byte. Trả `true` nếu trúng cache.
    pub fn access_cap(&mut self, address: usize) -> bool {
        self.clock += 1;
        self.account.num_access_cap += 1;
        let so_dong = (address / BYTE_MOI_DONG_CACHE) as u64;
        let chi_so_tap = (so_dong as usize) % self.so_tap;
        let the = so_dong;

        let dh = self.clock;
        let t = &mut self.tap[chi_so_tap];
        if let Some(e) = t.iter_mut().find(|(x, _)| *x == the) {
            e.1 = dh;
            self.account.num_duplicate += 1;
            return true;
        }
        // Trượt
        self.account.slip_count += 1;
        if self.seen.insert(the) {
            self.account.compulsory_miss += 1;
        } else {
            self.account.capacity_miss += 1;
        }
        if t.len() == self.positive_count {
            // Đá ra đường LÂU NHẤT KHÔNG DÙNG
            let vt = t.iter().enumerate().min_by_key(|(_, (_, d))| *d).map(|(i, _)| i).unwrap();
            t.swap_remove(vt);
        }
        t.push((the, dh));
        false
    }

    pub fn set_lai(&mut self) {
        for t in self.tap.iter_mut() { t.clear(); }
        self.seen.clear();
        self.clock = 0;
        self.account = CacheStats { num_access_cap: 0, num_duplicate: 0, slip_count: 0,
                                 compulsory_miss: 0, capacity_miss: 0 };
    }
}

// ============================================================================
// 3. CỤC BỘ CACHE — cùng phép tính, hai cách duyệt
// ============================================================================

/// Duyệt id trận THEO HÀNG. Rust lưu mảng theo hàng, nên hai phần tử kề nhau
/// trong hàng cũng kề nhau trong bộ nhớ → mỗi dòng cache 64 byte nạp về được
/// dùng cho 8 phần tử `f64`.
pub fn row_major_scan(mp: &mut CacheSim, n: usize, bytes_per_cell: usize) -> u64 {
    mp.set_lai();
    for i in 0..n {
        for j in 0..n {
            mp.access_cap((i * n + j) * bytes_per_cell);
        }
    }
    mp.account.slip_count
}

/// Duyệt THEO CỘT. Hai phần tử liên tiếp cách nhau `n` ô → mỗi lần chạm là
/// một dòng cache mới. Nạp 64 byte về chỉ để dùng 8 byte, phí 87,5%.
pub fn col_major_scan(mp: &mut CacheSim, n: usize, bytes_per_cell: usize) -> u64 {
    mp.set_lai();
    for j in 0..n {
        for i in 0..n {
            mp.access_cap((i * n + j) * bytes_per_cell);
        }
    }
    mp.account.slip_count
}

/// Nhân id trận ngây thơ: vòng lặp i-j-k. Vòng trong quét CỘT của id trận B.
pub fn matmul_naive(mp: &mut CacheSim, n: usize, bytes_per_cell: usize) -> u64 {
    mp.set_lai();
    let goc_a = 0usize;
    let goc_b = n * n * bytes_per_cell;
    for i in 0..n {
        for j in 0..n {
            for k in 0..n {
                mp.access_cap(goc_a + (i * n + k) * bytes_per_cell);
                mp.access_cap(goc_b + (k * n + j) * bytes_per_cell); // quét cột!
            }
        }
    }
    mp.account.slip_count
}

/// Nhân id trận theo KHỐI: chia thành các khối vừa lọt cache, làm xong khối
/// này mới sang khối khác. Cùng số phép nhân, nhưng dữ liệu được TÁI SỬ DỤNG
/// khi còn nóng trong cache.
pub fn blocked_matmul(mp: &mut CacheSim, n: usize, khoi: usize,
                         bytes_per_cell: usize) -> u64 {
    mp.set_lai();
    let goc_a = 0usize;
    let goc_b = n * n * bytes_per_cell;
    for ii in (0..n).step_by(khoi) {
        for jj in (0..n).step_by(khoi) {
            for kk in (0..n).step_by(khoi) {
                for i in ii..(ii + khoi).min(n) {
                    for j in jj..(jj + khoi).min(n) {
                        for k in kk..(kk + khoi).min(n) {
                            mp.access_cap(goc_a + (i * n + k) * bytes_per_cell);
                            mp.access_cap(goc_b + (k * n + j) * bytes_per_cell);
                        }
                    }
                }
            }
        }
    }
    mp.account.slip_count
}

// ============================================================================
// 4. DỰ ĐOÁN NHÁNH
// ============================================================================
// CPU hiện đại có đường ống 15–20 tầng. Gặp một `if`, nó ĐOÁN kết quả và chạy
// tiếp. Đoán đúng: không mất gì. Đoán sai: xả sạch đường ống, mất 15–20 owner kỳ.

pub const PHAT_DU_DOAN_SAI: u64 = 18;

/// Bộ đếm bão hoà 2 bit — bộ dự đoán nhánh kinh điển.
/// Trạng thái: 0 = chắc chắn không, 1 = có lẽ không, 2 = có lẽ có, 3 = chắc có.
/// Cần SAI HAI LẦN liên tiếp mới đổi ý → chống nhiễu cho vòng lặp.
#[derive(Debug, Clone)]
pub struct BranchPredictor {
    state: HashMap<usize, u8>,
    pub branch_count: u64,
    pub wrong_guess_balance: u64,
}

impl BranchPredictor {
    pub fn new() -> Self {
        BranchPredictor { state: HashMap::new(), branch_count: 0, wrong_guess_balance: 0 }
    }

    /// `id_nhanh` là vị trí lệnh nhánh; `actual` là kết quả thật.
    pub fn segment_data(&mut self, id_nhanh: usize, actual: bool) -> bool {
        self.branch_count += 1;
        let tt = self.state.entry(id_nhanh).or_insert(1);
        let doan = *tt >= 2;
        if doan != actual { self.wrong_guess_balance += 1; }
        // Bão hoà: 3 không lên nữa, 0 không xuống nữa
        if actual { *tt = (*tt + 1).min(3); } else { *tt = tt.saturating_sub(1); }
        doan == actual
    }

    pub fn ratio_sai(&self) -> f64 {
        if self.branch_count == 0 { 0.0 } else { self.wrong_guess_balance as f64 / self.branch_count as f64 }
    }
    /// Số owner kỳ mất trắng vì đoán sai.
    pub fn period_phi(&self) -> u64 { self.wrong_guess_balance * PHAT_DU_DOAN_SAI }
}

/// Đếm phần tử lớn hơn ngưỡng, CÓ nhánh. Trên dữ liệu ĐÃ SẮP XẾP, nhánh cực
/// dễ đoán (một chuỗi dài "không" rồi một chuỗi dài "có"). Trên dữ liệu lộn
/// xộn, nó gần như tung đồng xu.
pub fn branch_taken_count(data: &[i32], threshold: i32, dd: &mut BranchPredictor) -> (usize, u64) {
    let mut count = 0;
    for &x in data {
        let dieu_kien = x >= threshold;
        dd.segment_data(0xB1, dieu_kien); // một vị trí nhánh duy nhất
        if dieu_kien { count += 1; }
    }
    (count, dd.wrong_guess_balance)
}

/// Cùng phép tính nhưng KHÔNG có nhánh: biến điều kiện thành số học.
/// CPU không phải đoán gì cả → không bao giờ đoán sai.
pub fn branch_not_taken_count(data: &[i32], threshold: i32) -> usize {
    data.iter().map(|&x| (x >= threshold) as usize).sum()
}

// ============================================================================
// 5. SONG SONG MỨC LỆNH
// ============================================================================
// CPU hiện đại chạy 4–6 lệnh mỗi owner kỳ — NẾU chúng độc lập. Một chuỗi phụ
// thuộc (mỗi lệnh cần kết quả lệnh trước) làm mọi cổng thực thi khác ngồi chơi.

#[derive(Debug, PartialEq)]
pub struct IlpAnalysis {
    pub compute_op_count: u64,
    /// Chuỗi phụ thuộc dài nhất — cận dưới của số owner kỳ, bất kể CPU rộng bao nhiêu.
    pub critical_path: u64,
    pub ilp: f64,
    /// Số owner kỳ ước tính trên CPU rộng `do_rong` lệnh/owner kỳ.
    pub estimated_cycles: u64,
}

/// Cộng dồn vào MỘT biến: mỗi phép cộng phải chờ phép trước.
/// Đường tới hạn = n. CPU rộng 4 cũng vô dụng.
pub fn analyze_total_one_bien(n: u64, _do_rong: u64) -> IlpAnalysis {
    IlpAnalysis {
        compute_op_count: n,
        critical_path: n,
        ilp: 1.0,
        estimated_cycles: n.max(1), // bị chặn bởi chuỗi phụ thuộc, không bởi độ rộng
    }
}

/// Cộng dồn vào `k` biến rồi gộp cuối: `k` chuỗi độc lập chạy song song.
/// Đây là "bung vòng lặp có nhiều bộ tích luỹ" — thủ thuật hiệu năng cổ điển.
pub fn analyze_total_many_bien(n: u64, k: u64, do_rong: u64) -> IlpAnalysis {
    let k = k.max(1);
    // Mỗi chuỗi dài n/k, cộng thêm log2(k) bước gộp các bộ tích luỹ lại
    let critical_path = n / k + k.next_power_of_two().trailing_zeros() as u64;
    IlpAnalysis {
        compute_op_count: n,
        critical_path,
        ilp: n as f64 / critical_path.max(1) as f64,
        estimated_cycles: critical_path.max(n / do_rong.max(1)),
    }
}

/// Kiểm chứng: nhiều bộ tích luỹ phải cho CÙNG kết quả với một bộ.
pub fn tong_mot_bien(data: &[i64]) -> i64 { data.iter().sum() }

pub fn total_many_bien(data: &[i64], k: usize) -> i64 {
    let k = k.max(1);
    let mut acc = vec![0i64; k];
    for (i, &x) in data.iter().enumerate() { acc[i % k] += x; }
    acc.iter().sum()
}

// ============================================================================
// 6. SIMD — một lệnh, nhiều dữ liệu
// ============================================================================
// Thanh ghi vector 256 bit chứa 4 số `f64` hoặc 8 số `f32`. Một lệnh cộng
// vector làm 4 phép cộng cùng lúc. Trình biên dịch TỰ vector hoá được vòng
// lặp đơn giản, nhưng chỉ khi không có phụ thuộc và không có nhánh bên trong.

#[derive(Debug, PartialEq)]
pub struct SimdAnalysis {
    pub num_part_from: usize,
    pub be_rong_vector: usize,
    pub so_lenh_vector: usize,
    pub num_part_from_data: usize,
    pub theoretical_speedup: f64,
}

pub fn simd_analysis(num_part_from: usize, be_rong_vector: usize) -> SimdAnalysis {
    let w = be_rong_vector.max(1);
    let du = num_part_from % w;
    let so_lenh_vector = num_part_from / w;
    // Phần dư phải xử lý từng phần tử một — đó là cái giá của mảng không chia hết
    let total_order = so_lenh_vector + du;
    SimdAnalysis {
        num_part_from,
        be_rong_vector: w,
        so_lenh_vector,
        num_part_from_data: du,
        theoretical_speedup: if total_order == 0 { 1.0 }
                            else { num_part_from as f64 / total_order as f64 },
    }
}

/// Cộng hai mảng theo lô `w` phần tử — mô phỏng cách trình biên dịch vector hoá.
pub fn batch_add_array(a: &[f64], b: &[f64], w: usize) -> Vec<f64> {
    let n = a.len().min(b.len());
    let mut ra = vec![0.0; n];
    let w = w.max(1);
    let het_lo = n - n % w;
    for i in (0..het_lo).step_by(w) {
        for j in 0..w { ra[i + j] = a[i + j] + b[i + j]; }
    }
    for i in het_lo..n { ra[i] = a[i] + b[i]; }
    ra
}

// ============================================================================
// 7. SINH DỮ LIỆU TẤT ĐỊNH
// ============================================================================

pub fn gen_data(n: usize, hat_giong: u64) -> Vec<i32> {
    let mut s = hat_giong;
    (0..n).map(|_| {
        s = s.wrapping_mul(6364136223846793005).wrapping_add(1442695040888963407);
        ((s >> 33) % 256) as i32
    }).collect()
}

fn main() {
    println!("═══════════════════════════════════════════════════════════");
    println!("   KỸ NGHỆ HIỆU NĂNG CPU: CACHE · NHÁNH · ILP · SIMD       ");
    println!("═══════════════════════════════════════════════════════════");

    println!("\n1. PHÂN CẤP BỘ NHỚ — những con số cần thuộc");
    println!("   {:<12} {:>14} {:>16}", "tầng", "owner kỳ", "so với L1");
    for t in UpMemory::all() {
        println!("   {:<12} {:>14} {:>15.0}x",
                 t.name(), t.period(), t.period() as f64 / UpMemory::L1.period() as f64);
    }
    println!("   → Một lần trượt xuống RAM tốn bằng 50 lần chạm L1.");

    println!("\n2. CỤC BỘ CACHE — cùng phép duyệt, khác thứ tự");
    let mut mp = CacheSim::new(32 * 1024, 8); // L1 32 KB, 8 đường
    let n = 256;
    let theo_queue = row_major_scan(&mut mp, n, 8);
    let chain_queue = mp.account.total_period();
    let theo_cot = col_major_scan(&mut mp, n, 8);
    let chain_col = mp.account.total_period();
    println!("   Ma trận {}x{} f64 ({} KB):", n, n, n * n * 8 / 1024);
    println!("   Theo hàng: {:>8} lần trượt · {:>10} owner kỳ", theo_queue, chain_queue);
    println!("   Theo cột : {:>8} lần trượt · {:>10} owner kỳ", theo_cot, chain_col);
    println!("   → Cùng {} phép truy cập, chỉ khác thứ tự, chậm gấp {:.1} lần.",
             n * n, chain_col as f64 / chain_queue as f64);

    println!("\n3. NHÂN MA TRẬN — chia khối để tái dùng dữ liệu nóng");
    let n = 96;
    let mut mp = CacheSim::new(32 * 1024, 8);
    let ngay_tho = matmul_naive(&mut mp, n, 8);
    println!("   Ngây thơ (i-j-k): {:>9} lần trượt", ngay_tho);
    for k in [8usize, 16, 32] {
        let mut mp2 = CacheSim::new(32 * 1024, 8);
        let theo_block = blocked_matmul(&mut mp2, n, k, 8);
        println!("   Chia khối {:>2}x{:<2}   : {:>9} lần trượt → giảm {:.0}%",
                 k, k, theo_block, (1.0 - theo_block as f64 / ngay_tho as f64) * 100.0);
    }
    println!("   → CÙNG số phép nhân. Chỉ đổi thứ tự truy cập bộ nhớ.");

    println!("\n4. DỰ ĐOÁN NHÁNH — vì sao sắp xếp trước lại nhanh hơn");
    let lon_xon = gen_data(100_000, 42);
    let mut da_sap = lon_xon.clone();
    da_sap.sort_unstable();
    for (name, d) in [("lộn xộn ", &lon_xon), ("đã sắp  ", &da_sap)] {
        let mut dd = BranchPredictor::new();
        let (count, sai) = branch_taken_count(d, 128, &mut dd);
        println!("   {} → {} phần tử · {:>6} lần đoán sai ({:>5.1}%) · phí {:>8} owner kỳ",
                 name, count, sai, dd.ratio_sai() * 100.0, dd.period_phi());
    }
    println!("   Bản KHÔNG NHÁNH: {} phần tử · 0 lần đoán sai · 0 owner kỳ phí",
             branch_not_taken_count(&da_sap, 128));
    println!("   → Sắp xếp trước không làm phép đếm nhanh hơn; nó làm CPU ĐOÁN ĐÚNG hơn.");

    println!("\n5. SONG SONG MỨC LỆNH");
    let n = 1_000_000u64;
    println!("   {:<22} {:>14} {:>8} {:>16}",
             "cách viết", "đường tới hạn", "ILP", "owner kỳ ước tính");
    let a = analyze_total_one_bien(n, 4);
    println!("   {:<22} {:>14} {:>8.1} {:>16}",
             "1 bộ tích luỹ", a.critical_path, a.ilp, a.estimated_cycles);
    for k in [2u64, 4, 8] {
        let b = analyze_total_many_bien(n, k, 4);
        println!("   {:<22} {:>14} {:>8.1} {:>16}",
                 format!("{} bộ tích luỹ", k), b.critical_path, b.ilp, b.estimated_cycles);
    }
    let d: Vec<i64> = (1..=1000).collect();
    println!("   Kết quả vẫn giống hệt nhau: {}",
             tong_mot_bien(&d) == total_many_bien(&d, 4));

    println!("\n6. SIMD");
    println!("   {:>10} {:>10} {:>14} {:>10} {:>12}",
             "phần tử", "bề rộng", "lệnh vector", "phần dư", "tăng tốc");
    for (n, w) in [(1024usize, 4usize), (1024, 8), (1001, 8), (7, 8)] {
        let p = simd_analysis(n, w);
        println!("   {:>10} {:>10} {:>14} {:>10} {:>11.2}x",
                 p.num_part_from, p.be_rong_vector, p.so_lenh_vector,
                 p.num_part_from_data, p.theoretical_speedup);
    }
    println!("   → Mảng 7 phần tử với vector 8 làn: KHÔNG tăng tốc chút nào.");
    println!("     Đó là lý do người ta đệm mảng cho tròn bội số bề rộng vector.");

    println!("\n═══════════════════════════════════════════════════════════");
    println!("   CÙNG THUẬT TOÁN, KHÁC CÁCH CHẠM BỘ NHỚ, KHÁC HÀNG CHỤC LẦN");
    println!("═══════════════════════════════════════════════════════════");
}

#[cfg(test)]
mod tests {
    use super::*;

    // ---------- Phân cấp bộ nhớ ----------
    #[test]
    fn latency_up_derive_theo_distance() {
        let t = UpMemory::all();
        for w in t.windows(2) {
            assert!(w[0].period() < w[1].period(),
                    "{} phải nhanh hơn {}", w[0].name(), w[1].name());
        }
    }

    #[test]
    fn khoang_cach_giua_cac_tang_dung_bac_do_lon() {
        assert_eq!(UpMemory::Ram.period() / UpMemory::L1.period(), 50,
                   "trượt xuống RAM tốn bằng 50 lần chạm L1");
        assert!(UpMemory::SsdNvme.period() > UpMemory::Ram.period() * 500,
                "SSD chậm hơn RAM cả ba bậc độ lớn");
    }

    // ---------- Mô phỏng cache ----------
    #[test]
    fn lan_first_slow_always_truot_lan_next_duplicate() {
        let mut mp = CacheSim::new(32 * 1024, 8);
        assert!(!mp.access_cap(0), "lần đầu phải trượt");
        assert!(mp.access_cap(0), "lần hai phải trúng");
        assert_eq!(mp.account.compulsory_miss, 1);
        assert_eq!(mp.account.capacity_miss, 0);
    }

    #[test]
    fn ca_dong_cache_duoc_nap_ve_cung_luc() {
        // Chạm byte 0 thì byte 1..63 cũng vào cache theo — đó chính là lý do
        // duyệt tuần tự nhanh hơn duyệt nhảy cóc.
        let mut mp = CacheSim::new(32 * 1024, 8);
        mp.access_cap(0);
        for b in 1..BYTE_MOI_DONG_CACHE {
            assert!(mp.access_cap(b), "byte {} phải nằm cùng dòng cache với byte 0", b);
        }
        assert_eq!(mp.account.slip_count, 1, "64 byte chỉ tốn MỘT lần trượt");
    }

    #[test]
    fn buoc_nhay_bang_dong_cache_thi_lan_nao_cung_truot() {
        let mut mp = CacheSim::new(32 * 1024, 8);
        for i in 0..100 { mp.access_cap(i * BYTE_MOI_DONG_CACHE); }
        assert_eq!(mp.account.slip_count, 100, "mỗi lần chạm một dòng mới");
        assert_eq!(mp.account.ratio_duplicate(), 0.0);
    }

    #[test]
    fn data_exceed_cache_thi_is_da_out() {
        // Cache 1 KB = 16 dòng. Quét vòng qua 64 dòng thì lần nào cũng trượt.
        let mut mp = CacheSim::new(1024, 4);
        for _ in 0..3 {
            for i in 0..64 { mp.access_cap(i * BYTE_MOI_DONG_CACHE); }
        }
        assert!(mp.account.capacity_miss > 0, "phải có trượt do bị đá ra");
        assert!(mp.account.ratio_duplicate() < 0.1, "quét vòng lớn hơn cache → gần như trượt hết");
    }

    #[test]
    fn du_lieu_vua_cache_thi_lan_hai_trung_het() {
        let mut mp = CacheSim::new(32 * 1024, 8); // 512 dòng
        for _ in 0..5 {
            for i in 0..100 { mp.access_cap(i * BYTE_MOI_DONG_CACHE); }
        }
        assert_eq!(mp.account.slip_count, 100, "chỉ 100 lần trượt bắt buộc, sau đó trúng hết");
        assert_eq!(mp.account.capacity_miss, 0);
        assert!(mp.account.ratio_duplicate() > 0.79);
    }

    #[test]
    fn thong_ke_always_can_table() {
        let mut mp = CacheSim::new(4096, 4);
        for i in 0..1000 { mp.access_cap(i * 7); }
        assert_eq!(mp.account.num_duplicate + mp.account.slip_count, mp.account.num_access_cap);
        assert_eq!(mp.account.compulsory_miss + mp.account.capacity_miss, mp.account.slip_count);
    }

    // ---------- Cục bộ ----------
    #[test]
    fn duyet_theo_hang_it_truot_hon_han_theo_cot() {
        // Đây là bài học trung tâm của chương.
        let mut mp = CacheSim::new(32 * 1024, 8);
        let n = 256;
        let queue = row_major_scan(&mut mp, n, 8);
        let cot = col_major_scan(&mut mp, n, 8);
        assert!(cot > queue * 5,
                "theo cột {} lần trượt phải nhiều hơn hẳn theo hàng {}", cot, queue);
        // Theo hàng: mỗi dòng cache 64 byte phục vụ 8 phần tử f64
        assert_eq!(queue, (n * n / 8) as u64, "đúng bằng số dòng cache của cả id trận");
    }

    #[test]
    fn traverse_two_kind_slow_use_same_num_cell_nho() {
        let mut mp = CacheSim::new(32 * 1024, 8);
        let n = 64;
        row_major_scan(&mut mp, n, 8);
        let a = mp.account.num_access_cap;
        col_major_scan(&mut mp, n, 8);
        assert_eq!(a, mp.account.num_access_cap, "cùng khối lượng việc, chỉ khác thứ tự");
    }

    #[test]
    fn o_nho_nho_hon_thi_moi_dong_cache_phuc_vu_nhieu_phan_tu_hon() {
        let mut mp = CacheSim::new(32 * 1024, 8);
        let n = 128;
        let f64_ = row_major_scan(&mut mp, n, 8);
        let f32_ = row_major_scan(&mut mp, n, 4);
        assert!(f32_ < f64_, "dùng f32 thay f64 giảm một nửa số lần trượt");
        assert_eq!(f64_, f32_ * 2);
    }

    // ---------- Nhân id trận ----------
    #[test]
    fn chia_khoi_giam_so_lan_truot() {
        let n = 96;
        let mut a = CacheSim::new(32 * 1024, 8);
        let ngay_tho = matmul_naive(&mut a, n, 8);
        let mut b = CacheSim::new(32 * 1024, 8);
        let theo_block = blocked_matmul(&mut b, n, 16, 8);
        assert!(theo_block < ngay_tho,
                "chia khối {} phải ít trượt hơn ngây thơ {}", theo_block, ngay_tho);
    }

    #[test]
    fn chia_khoi_lam_dung_so_phep_truy_cap() {
        // Bất biến: tối ưu không được đổi KHỐI LƯỢNG VIỆC, chỉ đổi thứ tự.
        let n = 48;
        let mut a = CacheSim::new(32 * 1024, 8);
        matmul_naive(&mut a, n, 8);
        let mut b = CacheSim::new(32 * 1024, 8);
        blocked_matmul(&mut b, n, 16, 8);
        assert_eq!(a.account.num_access_cap, b.account.num_access_cap,
                   "cùng 2·n³ phép truy cập, chỉ khác thứ tự");
        assert_eq!(a.account.num_access_cap, 2 * (n * n * n) as u64);
    }

    // ---------- Dự đoán nhánh ----------
    #[test]
    fn buffer_report_hoa_can_sai_two_lan_new_swap_y() {
        // Đây là lý do bộ đếm 2 bit tốt hơn 1 bit: một lần chệch không làm
        // nó đổi ý, nên vòng lặp dài không bị phạt ở lần lặp bất thường.
        let mut d = BranchPredictor::new();
        for _ in 0..10 { d.segment_data(1, true); } // học "luôn đúng"
        let prev_sai = d.wrong_guess_balance;
        d.segment_data(1, false); // một lần chệch
        assert_eq!(d.wrong_guess_balance, prev_sai + 1);
        assert!(d.segment_data(1, true), "một lần chệch KHÔNG làm nó đổi ý");
    }

    #[test]
    fn nhanh_luon_dung_thi_gan_nhu_khong_doan_sai() {
        let mut d = BranchPredictor::new();
        for _ in 0..10_000 { d.segment_data(1, true); }
        assert!(d.wrong_guess_balance <= 2, "chỉ sai vài lần lúc học, thực tế {}", d.wrong_guess_balance);
        assert!(d.ratio_sai() < 0.001);
    }

    #[test]
    fn nhanh_lat_lien_tuc_thi_doan_sai_gan_het() {
        // Trường hợp tệ nhất của bộ đếm 2 bit: mẫu luân phiên.
        let mut d = BranchPredictor::new();
        for i in 0..10_000 { d.segment_data(1, i % 2 == 0); }
        assert!(d.ratio_sai() > 0.4, "mẫu luân phiên phải làm nó sai rất nhiều");
    }

    #[test]
    fn du_lieu_da_sap_xep_it_doan_sai_hon_han() {
        // Câu hỏi phỏng vấn kinh điển: "vì sao sắp xếp mảng trước lại làm
        // vòng lặp đếm chạy nhanh hơn?" — không phải vì phép đếm nhanh hơn,
        // mà vì CPU đoán nhánh đúng hơn.
        let lon_xon = gen_data(50_000, 42);
        let mut da_sap = lon_xon.clone();
        da_sap.sort_unstable();

        let mut d1 = BranchPredictor::new();
        let (a, sai_lon_xon) = branch_taken_count(&lon_xon, 128, &mut d1);
        let mut d2 = BranchPredictor::new();
        let (b, sai_da_sap) = branch_taken_count(&da_sap, 128, &mut d2);

        assert_eq!(a, b, "kết quả phải giống hệt — chỉ hiệu năng khác");
        assert!(sai_da_sap * 20 < sai_lon_xon,
                "đã sắp: {} lần sai, lộn xộn: {} lần sai", sai_da_sap, sai_lon_xon);
        assert!(d1.period_phi() > d2.period_phi() * 20);
    }

    #[test]
    fn sell_no_nhanh_wait_same_result() {
        for hat in [1u64, 42, 2024] {
            let d = gen_data(10_000, hat);
            let mut dd = BranchPredictor::new();
            let (a, _) = branch_taken_count(&d, 128, &mut dd);
            assert_eq!(a, branch_not_taken_count(&d, 128),
                       "mã không nhánh phải cho cùng đáp số");
        }
    }

    #[test]
    fn ban_khong_nhanh_khong_bao_gio_doan_sai() {
        // Không có nhánh thì không có gì để đoán — và không có gì để đoán sai.
        // Đây cũng là nền của mã mật mã chạy thời gian không đổi (Chương 57).
        let lon_xon = gen_data(10_000, 7);
        let dd = BranchPredictor::new();
        branch_not_taken_count(&lon_xon, 128);
        assert_eq!(dd.wrong_guess_balance, 0);
        assert_eq!(dd.period_phi(), 0);
    }

    // ---------- ILP ----------
    #[test]
    fn mot_bo_tich_luy_bi_chan_boi_chuoi_phu_thuoc() {
        let a = analyze_total_one_bien(1_000_000, 4);
        assert_eq!(a.ilp, 1.0, "chuỗi phụ thuộc thuần → không song song được gì");
        assert_eq!(a.estimated_cycles, 1_000_000,
                   "CPU rộng 4 lệnh/owner kỳ cũng không giúp được gì");
    }

    #[test]
    fn nhieu_bo_tich_luy_tang_ilp() {
        let mut ilp_truoc = 0.0;
        for k in [1u64, 2, 4, 8] {
            let b = analyze_total_many_bien(1_000_000, k, 4);
            assert!(b.ilp > ilp_truoc, "k={} phải cho ILP cao hơn", k);
            ilp_truoc = b.ilp;
        }
        let b4 = analyze_total_many_bien(1_000_000, 4, 4);
        assert!(b4.ilp > 3.9, "4 bộ tích luỹ phải đạt ILP gần 4, thực tế {:.2}", b4.ilp);
    }

    #[test]
    fn do_rong_cpu_chan_tren_toc_do() {
        // Dù có 64 bộ tích luỹ, CPU rộng 4 vẫn chỉ chạy 4 lệnh mỗi owner kỳ.
        let b = analyze_total_many_bien(1_000_000, 64, 4);
        assert!(b.estimated_cycles >= 1_000_000 / 4,
                "không thể nhanh hơn giới hạn độ rộng CPU");
    }

    #[test]
    fn many_unit_tich_accum_wait_same_result() {
        // Cộng số nguyên có tính kết hợp nên đổi thứ tự vẫn đúng.
        // (Với f64 thì KHÔNG — đó là lý do trình biên dịch không tự làm việc
        // này cho số thực trừ khi bạn cho phép nới lỏng ngữ nghĩa dấu phẩy động.)
        let d: Vec<i64> = (1..=10_000).collect();
        let mong_doi = tong_mot_bien(&d);
        for k in [1usize, 2, 3, 4, 8, 16] {
            assert_eq!(total_many_bien(&d, k), mong_doi, "k={}", k);
        }
    }

    #[test]
    fn total_array_empty_table_no() {
        assert_eq!(tong_mot_bien(&[]), 0);
        assert_eq!(total_many_bien(&[], 4), 0);
    }

    // ---------- SIMD ----------
    #[test]
    fn simd_tang_toc_dung_be_rong_khi_chia_het() {
        let p = simd_analysis(1024, 4);
        assert_eq!(p.so_lenh_vector, 256);
        assert_eq!(p.num_part_from_data, 0);
        assert!((p.theoretical_speedup - 4.0).abs() < 1e-9);
    }

    #[test]
    fn phan_du_lam_giam_tang_toc() {
        let chia_het = simd_analysis(1024, 8);
        assert_eq!(chia_het.num_part_from_data, 0);
        let le = simd_analysis(1001, 8);
        assert_eq!(le.num_part_from_data, 1);
        assert!(le.theoretical_speedup < chia_het.theoretical_speedup);
    }

    #[test]
    fn mang_qua_ngan_thi_simd_vo_dung() {
        // 7 phần tử với vector 8 làn: không lô nào đầy, mọi phần tử xử lý lẻ.
        let p = simd_analysis(7, 8);
        assert_eq!(p.so_lenh_vector, 0);
        assert_eq!(p.num_part_from_data, 7);
        assert!((p.theoretical_speedup - 1.0).abs() < 1e-9, "không tăng tốc chút nào");
    }

    #[test]
    fn simd_be_rong_bat_thuong_khong_lam_hong_gi() {
        let p = simd_analysis(100, 1);
        assert!((p.theoretical_speedup - 1.0).abs() < 1e-9);
        let p0 = simd_analysis(100, 0);
        assert_eq!(p0.be_rong_vector, 1, "bề rộng 0 phải được chặn thành 1");
        let rong = simd_analysis(0, 8);
        assert!((rong.theoretical_speedup - 1.0).abs() < 1e-9, "mảng rỗng không panic");
    }

    #[test]
    fn cong_array_theo_lo_wait_same_result_with_new_be_empty() {
        let a: Vec<f64> = (0..103).map(|i| i as f64).collect();
        let b: Vec<f64> = (0..103).map(|i| (i * 2) as f64).collect();
        let mong_doi: Vec<f64> = a.iter().zip(b.iter()).map(|(x, y)| x + y).collect();
        for w in [1usize, 2, 4, 8, 16] {
            assert_eq!(batch_add_array(&a, &b, w), mong_doi,
                       "vector hoá bề rộng {} phải cho cùng kết quả", w);
        }
    }

    #[test]
    fn cong_array_do_long_other_each_get_part_chung() {
        let a = vec![1.0, 2.0, 3.0];
        let b = vec![10.0, 20.0];
        assert_eq!(batch_add_array(&a, &b, 4), vec![11.0, 22.0]);
    }

    // ---------- Sinh dữ liệu ----------
    #[test]
    fn gen_data_all_peak() {
        assert_eq!(gen_data(100, 5), gen_data(100, 5));
        assert_ne!(gen_data(100, 5), gen_data(100, 6));
    }

    #[test]
    fn du_lieu_sinh_ra_trai_deu_hai_phia_nguong() {
        let d = gen_data(100_000, 42);
        let above = d.iter().filter(|&&x| x >= 128).count();
        assert!((above as f64 / d.len() as f64 - 0.5).abs() < 0.05,
                "phải chia đôi quanh ngưỡng để nhánh thật sự khó đoán");
    }
}
```

---

## Bảng tra cứu lỗi biên dịch thường gặp

| Lỗi | Nguyên nhân trong chương này | Cách sửa |
|---|---|---|
| Kết quả đo bằng 0 | Trình tối ưu xoá vòng lặp không dùng kết quả | `std::hint::black_box` quanh giá trị |
| Kết quả 4 biến tích luỹ khác 1 biến | Cộng số thực không có tính kết hợp | Đúng như dự kiến; kiểm bằng sai số tương đối, không bằng `==` |
| `E0308: expected usize, found u64` | Trộn chỉ số với địa chỉ | Ép kiểu tường minh ở biên |
| SIMD bỏ sót phần tử cuối | Quên xử lý phần dư | `num_part_from % chieu_rong` luôn phải có nhánh xử lý |
| Đo cache ra kết quả vô nghĩa | Bộ nạp trước (prefetcher) đoán đúng mẫu truy cập | Dùng bước nhảy không đều để đánh lừa nó |

---

## Tóm tắt chương & Bài tập rèn luyện

### 5 điểm cốt lõi

1. **RAM chậm hơn L1 khoảng 75 lần.** Bố cục bộ nhớ thường quan trọng hơn số phép tính.
2. **Dòng cache 64 byte là đơn vị thật.** Duyệt theo hàng dùng hết; duyệt theo cột vứt đi 87,5%.
3. **Chia khối giữ nguyên số phép tính nhưng giảm trượt cache một bậc.**
4. **Mã không rẽ nhánh thắng với dữ liệu ngẫu nhiên, thua với dữ liệu đã sắp xếp.** Phải biết dữ liệu.
5. **Chuỗi phụ thuộc chặn ILP.** Nhiều biến tích luỹ phá chuỗi và tăng thông lượng gần tuyến tính.

### Bài tập rèn luyện

**Bài 1.** Cài **chuyển vị id trận thân thiện cache** và so với bản ngây thơ.

<details>
<summary><b>Gợi ý</b></summary>

Chuyển vị ngây thơ luôn có một phía truy cập theo cột — bất kể bạn xoay vòng lặp thế nào. Chia khối giải quyết cả hai phía cùng lúc: mỗi khối nhỏ vừa L1, nên cả đọc lẫn ghi đều nằm trong cache.
</details>

<details>
<summary><b>Lời giải</b></summary>

```rust
pub fn chuyen_vi_ngay_tho(a: &[f64], n: usize) -> Vec<f64> {
    let mut r = vec![0.0; n * n];
    for i in 0..n {
        for j in 0..n {
            r[j * n + i] = a[i * n + j];   // ghi theo cột → trượt mỗi phần tử
        }
    }
    r
}

pub fn chuyen_vi_theo_khoi(a: &[f64], n: usize, khoi: usize) -> Vec<f64> {
    let mut r = vec![0.0; n * n];
    for ii in (0..n).step_by(khoi) {
        for jj in (0..n).step_by(khoi) {
            let het_i = (ii + khoi).min(n);
            let het_j = (jj + khoi).min(n);
            // Cả khối đọc lẫn khối ghi đều vừa L1 → không trượt bên trong
            for i in ii..het_i {
                for j in jj..het_j {
                    r[j * n + i] = a[i * n + j];
                }
            }
        }
    }
    r
}

/// Kích thước khối tối ưu: hai khối (đọc + ghi) phải vừa L1.
pub fn khoi_toi_uu(kich_thuoc_l1_byte: usize) -> usize {
    // 2 khối × B² × 8 byte ≤ L1  →  B ≤ √(L1 / 16)
    ((kich_thuoc_l1_byte as f64 / 16.0).sqrt() as usize).max(8)
}
```

Với L1 32 KB, `khoi_toi_uu` cho B = 45, nên thực tế dùng 32 hoặc 64 (luỹ thừa 2 cho phép tính chỉ số rẻ hơn). Với id trận 1024×1024, chuyển vị theo khối thường nhanh hơn 3–5 lần.
</details>

**Bài 2.** Cài **tổng tiền tố (prefix sum) SIMD** — bài toán tưởng như không song song hoá được.

<details>
<summary><b>Gợi ý</b></summary>

Tổng tiền tố có phụ thuộc tuần tự: `s[i] = s[i-1] + a[i]`. Nhưng nó song song hoá được bằng thuật toán **quét Hillis–Steele**: dịch và cộng với bước nhảy 1, 2, 4, 8… Tổng cộng `log₂(n)` bước thay vì `n` bước.
</details>

<details>
<summary><b>Lời giải</b></summary>

```rust
pub fn tong_tien_to_tuan_tu(a: &[f64]) -> Vec<f64> {
    let mut r = Vec::with_capacity(a.len());
    let mut s = 0.0;
    for x in a { s += x; r.push(s); }
    r
}

/// Quét Hillis–Steele: log₂(n) bước, mỗi bước song song hoàn toàn.
/// Tốn nhiều phép cộng hơn (n·log n so với n) nhưng ĐỘ SÂU ngắn hơn nhiều.
pub fn tong_tien_to_quet(a: &[f64]) -> Vec<f64> {
    let mut r = a.to_vec();
    let n = r.len();
    let mut step = 1;
    while step < n {
        // Duyệt NGƯỢC để không ghi đè giá trị chưa dùng
        for i in (step..n).rev() {
            r[i] += r[i - step];
        }
        step *= 2;
    }
    r
}

/// Chia khối: quét song song trong khối, rồi cộng dồn offset giữa các khối.
/// Đây là cách các thư viện thật làm — cân bằng giữa độ sâu và tổng công.
pub fn tong_tien_to_theo_khoi(a: &[f64], khoi: usize) -> Vec<f64> {
    let mut r = a.to_vec();
    for c in r.chunks_mut(khoi) {
        let mut s = 0.0;
        for x in c.iter_mut() { s += *x; *x = s; }
    }
    let mut bu = 0.0;
    for k in 0..r.len().div_ceil(khoi) {
        let first = k * khoi;
        let last = (first + khoi).min(r.len());
        if k > 0 { for i in first..last { r[i] += bu; } }
        bu = r[last - 1];
    }
    r
}
```

Đây là ví dụ tổng quát của một nguyên tắc quan trọng: **thuật toán song song thường làm nhiều việc hơn nhưng có độ sâu ngắn hơn**. Quét Hillis–Steele tốn `n log n` phép cộng thay vì `n`, nhưng độ sâu chỉ `log n` thay vì `n` — và trên phần cứng song song, độ sâu mới là thứ quyết định.
</details>
