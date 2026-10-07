# Chương 80: Hiệu năng CPU sâu — Cache, Dự đoán rẽ nhánh, ILP & SIMD (LeetCPU)

## Giới thiệu & Mục tiêu học tập

Chương này lấy cảm hứng từ thể loại bài tập của **leetcpu.com**: những bài toán mà đáp án đúng phụ thuộc vào việc bạn hiểu CPU thật hoạt động thế nào, chứ không phải độ phức tạp big-O.

Điểm khởi đầu là một sự thật gây sốc:

> **Một lần truy cập RAM tốn khoảng 300 chu kỳ. Trong 300 chu kỳ đó, CPU có thể làm hơn 1000 phép cộng.**

Nghĩa là **bố cục bộ nhớ quan trọng hơn số phép tính**. Một thuật toán "kém hơn" về big-O nhưng thân thiện với cache thường nhanh hơn nhiều lần trong thực tế.

| Chủ đề | Bài học cốt lõi |
|---|---|
| Phân cấp bộ nhớ | L1 ~4 chu kỳ, RAM ~300 chu kỳ — chênh 75 lần |
| Cục bộ | Duyệt theo hàng ít trượt cache hơn theo cột ~8 lần (với `f64`) |
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
│    Thanh ghi   1 chu kỳ    ~1 KB     trong tầm tay                          │
│    L1          4 chu kỳ    32 KB     trên bàn                               │
│    L2         12 chu kỳ   256 KB     kệ sau lưng                            │
│    L3         40 chu kỳ    16 MB     phòng bên cạnh                         │
│    RAM       300 chu kỳ    32 GB     ĐI THƯ VIỆN THÀNH PHỐ                  │
│                                                                              │
│   Tỉ lệ L1 : RAM = 1 : 75.                                                  │
│   Nếu L1 là "với tay lấy" (1 giây) thì RAM là "đi bộ 75 giây".              │
│                                                                              │
│  DUYỆT HÀNG vs DUYỆT CỘT (ma trận 1024×1024 f64)                           │
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
│    Mỗi lần đoán sai = xả toàn bộ đường ống = ~15 chu kỳ mất trắng.          │
│    → Sắp xếp mảng trước rồi lọc có thể NHANH HƠN lọc trực tiếp,            │
│      dù sắp xếp tốn O(n log n).                                            │
│                                                                              │
│  ILP = CHUỖI PHỤ THUỘC LÀ KẺ THÙ                                            │
│                                                                              │
│    1 biến tích luỹ:  s += a[0]; s += a[1]; ...  ← mỗi phép PHẢI chờ phép   │
│                                                    trước → 4 chu kỳ/phần tử │
│                                                                              │
│    4 biến tích luỹ:  s0 += a[0]; s1 += a[1];   ← 4 chuỗi ĐỘC LẬP           │
│                      s2 += a[2]; s3 += a[3];      chạy song song            │
│                                                → ~1 chu kỳ/phần tử         │
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

Hệ quả: nếu bạn truy cập nhiều địa chỉ cùng ánh xạ vào một tập, chúng đá nhau ra dù cache còn trống chỗ khác. Đây là **trượt do xung đột**, và nó xuất hiện đúng khi bước nhảy là luỹ thừa của 2 — tình huống rất hay gặp với ma trận vuông.

Cách chữa kinh điển: **đệm** thêm một phần tử vào mỗi hàng, biến bước nhảy 1024 thành 1025. Một phần tử thừa, hết xung đột.

### 3. Chia khối: cùng phép tính, ít trượt hơn

Nhân ma trận ngây thơ có ba vòng lặp lồng nhau. Với ma trận lớn hơn cache, ma trận B bị nạp lại **hoàn toàn** cho mỗi hàng của A.

Chia khối chia bài toán thành các khối con vừa với L1. Mỗi khối được nạp một lần rồi dùng hết trước khi bị đẩy ra.

Con số quan trọng: cùng `2n³` phép tính, nhưng số lần trượt cache giảm từ `O(n³)` xuống `O(n³/√M)` với M là kích thước cache. Với ma trận 512×512, đó là chênh lệch 5–10 lần thời gian chạy.

### 4. Dự đoán rẽ nhánh và mã không rẽ nhánh

Bộ dự đoán 2-bit bão hoà có bốn trạng thái: chắc chắn không nhảy → có thể không → có thể nhảy → chắc chắn nhảy. Từ trạng thái "chắc chắn", cần **hai** lần sai liên tiếp mới đổi hướng dự đoán, nên nó chịu được nhiễu tốt.

Nhưng với dữ liệu ngẫu nhiên, không bộ dự đoán nào cứu được — tỉ lệ đúng về 50%, và mỗi lần sai mất khoảng 15 chu kỳ.

Giải pháp là **mã không rẽ nhánh**: thay `if x > t { s += x }` bằng `s += x * (x > t) as i64`. Không có nhánh nên không có dự đoán sai. Với dữ liệu ngẫu nhiên, phiên bản không nhánh nhanh hơn rõ rệt; với dữ liệu đã sắp xếp thì phiên bản có nhánh lại thắng, vì dự đoán gần như luôn đúng.

Bài học: **không có phiên bản nào luôn tốt hơn**. Phải biết dữ liệu của mình.

### 5. ILP: vì sao 4 biến tích luỹ nhanh hơn 1

CPU hiện đại thực thi ngoài thứ tự và có nhiều đơn vị tính toán. Nhưng chúng không thể phá vỡ **chuỗi phụ thuộc dữ liệu**: nếu `s += a[i]` phải chờ `s` từ lần trước, thì tốc độ bị chặn bởi độ trễ của phép cộng (khoảng 4 chu kỳ cho số thực).

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

Danh sách dưới đây được **lấy trực tiếp từ leetcpu.com** (thu thập ngày 05/09/2026). Nền tảng đó chạy mã C của bạn trên **ChampSim** — bộ mô phỏng vi kiến trúc chính xác theo chu kỳ, 200 triệu lệnh — rồi trả về IPC, MPKI và số liệu bộ dự đoán rẽ nhánh. Chuỗi công cụ của họ là `gcc` → `objdump` → vết Intel PIN → ChampSim → bảng chỉ số.

Chúng ta không mô phỏng vi kiến trúc chính xác theo chu kỳ như ChampSim; chúng ta cài **cùng những kỹ thuật đó bằng Rust** và **đếm** số lần trượt cache, số lần đoán sai bằng các mô hình đơn giản, tất định (`CacheSim`, `BranchPredictor`) — thay vì đo đồng hồ — để kết quả tái lập được và kiểm thử được. Năm nhóm của họ (bốn nhóm kỹ thuật cộng một bài chẩn đoán) đều có kỹ thuật tương ứng trong chương này.

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
| 16 | Streaming Computation — Multiple Accumulators | Dễ | ILP | `analyze_multi_accumulator` |
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
//! Theo tinh thần các bài tập của [LeetCPU](https://www.leetcpu.com/) — nền
//! tảng luyện hiệu năng CPU có mô phỏng vi kiến trúc phản hồi. Ở đây ta ĐẾM
//! số lần trượt cache và dự đoán sai bằng mô phỏng tất định, thay vì đo đồng
//! hồ treo tường — nhờ vậy kết quả tái lập được và kiểm thử được.

use std::collections::HashMap;

// ============================================================================
// 1. PHÂN CẤP BỘ NHỚ — những con số cần thuộc lòng
// ============================================================================

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum MemoryTier {
    Register,
    L1,
    L2,
    L3,
    Ram,
    SsdNvme,
    SpinningDisk,
}

impl MemoryTier {
    /// Độ trễ tính bằng CHU KỲ CPU. Cách nhìn này quan trọng hơn nano-giây:
    /// nó cho biết CPU phải ngồi chơi bao nhiêu nhịp.
    pub fn cycles(self) -> u64 {
        match self {
            MemoryTier::Register => 1,
            MemoryTier::L1 => 4,
            MemoryTier::L2 => 12,
            MemoryTier::L3 => 40,
            MemoryTier::Ram => 300,
            MemoryTier::SsdNvme => 200_000,
            MemoryTier::SpinningDisk => 20_000_000,
        }
    }
    pub fn name(self) -> &'static str {
        match self {
            MemoryTier::Register => "Thanh ghi",
            MemoryTier::L1 => "Cache L1",
            MemoryTier::L2 => "Cache L2",
            MemoryTier::L3 => "Cache L3",
            MemoryTier::Ram => "RAM",
            MemoryTier::SsdNvme => "SSD NVMe",
            MemoryTier::SpinningDisk => "Đĩa quay",
        }
    }
    pub fn all() -> [MemoryTier; 7] {
        [
            MemoryTier::Register,
            MemoryTier::L1,
            MemoryTier::L2,
            MemoryTier::L3,
            MemoryTier::Ram,
            MemoryTier::SsdNvme,
            MemoryTier::SpinningDisk,
        ]
    }
}

pub const CACHE_LINE_BYTES: usize = 64;

// ============================================================================
// 2. MÔ PHỎNG CACHE LIÊN KẾT TẬP HỢP
// ============================================================================
// Cache thật không phải "có hay không có" — nó chia thành TẬP HỢP, mỗi tập
// chứa vài ĐƯỜNG. Địa chỉ quyết định tập nào; trong tập thì thay theo LRU.

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct CacheStats {
    pub access_count: u64,
    pub hit_count: u64,
    pub miss_count: u64,
    /// Trượt vì lần đầu chạm tới — không tránh được.
    pub compulsory_miss: u64,
    /// Trượt vì cache quá nhỏ hoặc bị đá ra (gồm cả trượt do XUNG ĐỘT trong
    /// một tập) — CÓ THỂ tránh được.
    pub capacity_miss: u64,
}

impl CacheStats {
    pub fn hit_ratio(&self) -> f64 {
        if self.access_count == 0 {
            0.0
        } else {
            self.hit_count as f64 / self.access_count as f64
        }
    }
    /// Tổng chu kỳ phải trả — thước đo thật sự, không phải số lần trượt.
    pub fn total_cycles(&self) -> u64 {
        self.hit_count * MemoryTier::L1.cycles() + self.miss_count * MemoryTier::Ram.cycles()
    }
}

pub struct CacheSim {
    pub num_sets: usize,
    pub ways: usize,
    /// tập → danh sách (thẻ, dấu thời gian dùng gần nhất), dài tối đa `ways`
    sets: Vec<Vec<(u64, u64)>>,
    seen: std::collections::HashSet<u64>,
    clock: u64,
    pub stats: CacheStats,
}

impl CacheSim {
    /// `size_bytes` là tổng dung lượng; `ways` là số đường mỗi tập.
    pub fn new(size_bytes: usize, ways: usize) -> Self {
        let num_lines = size_bytes / CACHE_LINE_BYTES;
        let num_sets = (num_lines / ways).max(1);
        CacheSim {
            num_sets,
            ways,
            sets: vec![Vec::with_capacity(ways); num_sets],
            seen: std::collections::HashSet::new(),
            clock: 0,
            stats: CacheStats {
                access_count: 0,
                hit_count: 0,
                miss_count: 0,
                compulsory_miss: 0,
                capacity_miss: 0,
            },
        }
    }

    /// Truy cập một địa chỉ byte. Trả `true` nếu trúng cache.
    pub fn access(&mut self, address: usize) -> bool {
        self.clock += 1;
        self.stats.access_count += 1;
        let line = (address / CACHE_LINE_BYTES) as u64;
        let set_index = (line as usize) % self.num_sets;
        let tag = line;

        let now = self.clock;
        let t = &mut self.sets[set_index];
        if let Some(e) = t.iter_mut().find(|(x, _)| *x == tag) {
            e.1 = now;
            self.stats.hit_count += 1;
            return true;
        }
        // Trượt
        self.stats.miss_count += 1;
        if self.seen.insert(tag) {
            self.stats.compulsory_miss += 1;
        } else {
            self.stats.capacity_miss += 1;
        }
        if t.len() == self.ways {
            // Đá ra đường LÂU NHẤT KHÔNG DÙNG
            let victim = t
                .iter()
                .enumerate()
                .min_by_key(|(_, (_, d))| *d)
                .map(|(i, _)| i)
                .unwrap();
            t.swap_remove(victim);
        }
        t.push((tag, now));
        false
    }

    pub fn reset(&mut self) {
        for t in self.sets.iter_mut() {
            t.clear();
        }
        self.seen.clear();
        self.clock = 0;
        self.stats = CacheStats {
            access_count: 0,
            hit_count: 0,
            miss_count: 0,
            compulsory_miss: 0,
            capacity_miss: 0,
        };
    }
}

// ============================================================================
// 3. CỤC BỘ CACHE — cùng phép tính, hai cách duyệt
// ============================================================================

/// Duyệt ma trận THEO HÀNG. Rust lưu mảng theo hàng, nên hai phần tử kề nhau
/// trong hàng cũng kề nhau trong bộ nhớ → mỗi dòng cache 64 byte nạp về được
/// dùng cho 8 phần tử `f64`.
pub fn row_major_scan(cache: &mut CacheSim, n: usize, bytes_per_cell: usize) -> u64 {
    cache.reset();
    for i in 0..n {
        for j in 0..n {
            cache.access((i * n + j) * bytes_per_cell);
        }
    }
    cache.stats.miss_count
}

/// Duyệt THEO CỘT. Hai phần tử liên tiếp cách nhau `n` ô → mỗi lần chạm là
/// một dòng cache mới. Nạp 64 byte về chỉ để dùng 8 byte, phí 87,5%.
pub fn col_major_scan(cache: &mut CacheSim, n: usize, bytes_per_cell: usize) -> u64 {
    cache.reset();
    for j in 0..n {
        for i in 0..n {
            cache.access((i * n + j) * bytes_per_cell);
        }
    }
    cache.stats.miss_count
}

/// Nhân ma trận ngây thơ: vòng lặp i-j-k. Vòng trong quét CỘT của ma trận B.
pub fn matmul_naive(cache: &mut CacheSim, n: usize, bytes_per_cell: usize) -> u64 {
    cache.reset();
    let base_a = 0usize;
    let base_b = n * n * bytes_per_cell;
    for i in 0..n {
        for j in 0..n {
            for k in 0..n {
                cache.access(base_a + (i * n + k) * bytes_per_cell);
                cache.access(base_b + (k * n + j) * bytes_per_cell); // quét cột!
            }
        }
    }
    cache.stats.miss_count
}

/// Nhân ma trận theo KHỐI: chia thành các khối vừa lọt cache, làm xong khối
/// này mới sang khối khác. Cùng số phép nhân, nhưng dữ liệu được TÁI SỬ DỤNG
/// khi còn nóng trong cache.
pub fn blocked_matmul(cache: &mut CacheSim, n: usize, block: usize, bytes_per_cell: usize) -> u64 {
    cache.reset();
    let base_a = 0usize;
    let base_b = n * n * bytes_per_cell;
    for ii in (0..n).step_by(block) {
        for jj in (0..n).step_by(block) {
            for kk in (0..n).step_by(block) {
                for i in ii..(ii + block).min(n) {
                    for j in jj..(jj + block).min(n) {
                        for k in kk..(kk + block).min(n) {
                            cache.access(base_a + (i * n + k) * bytes_per_cell);
                            cache.access(base_b + (k * n + j) * bytes_per_cell);
                        }
                    }
                }
            }
        }
    }
    cache.stats.miss_count
}

// ============================================================================
// 4. DỰ ĐOÁN NHÁNH
// ============================================================================
// CPU hiện đại có đường ống 15–20 tầng. Gặp một `if`, nó ĐOÁN kết quả và chạy
// tiếp. Đoán đúng: không mất gì. Đoán sai: xả sạch đường ống, mất 15–20 chu kỳ.

pub const MISPREDICT_PENALTY: u64 = 18;

/// Bộ đếm bão hoà 2 bit — bộ dự đoán nhánh kinh điển.
/// Trạng thái: 0 = chắc chắn không, 1 = có lẽ không, 2 = có lẽ có, 3 = chắc có.
/// Cần SAI HAI LẦN liên tiếp mới đổi ý → chống nhiễu cho vòng lặp.
#[derive(Debug, Clone)]
pub struct BranchPredictor {
    state: HashMap<usize, u8>,
    pub branch_count: u64,
    pub mispredictions: u64,
}

impl Default for BranchPredictor {
    fn default() -> Self {
        Self::new()
    }
}

impl BranchPredictor {
    pub fn new() -> Self {
        BranchPredictor {
            state: HashMap::new(),
            branch_count: 0,
            mispredictions: 0,
        }
    }

    /// `branch_id` là vị trí lệnh nhánh; `actual` là kết quả thật.
    pub fn predict(&mut self, branch_id: usize, actual: bool) -> bool {
        self.branch_count += 1;
        let state = self.state.entry(branch_id).or_insert(1);
        let guess = *state >= 2;
        if guess != actual {
            self.mispredictions += 1;
        }
        // Bão hoà: 3 không lên nữa, 0 không xuống nữa
        if actual {
            *state = (*state + 1).min(3);
        } else {
            *state = state.saturating_sub(1);
        }
        guess == actual
    }

    pub fn mispredict_ratio(&self) -> f64 {
        if self.branch_count == 0 {
            0.0
        } else {
            self.mispredictions as f64 / self.branch_count as f64
        }
    }
    /// Số chu kỳ mất trắng vì đoán sai.
    pub fn wasted_cycles(&self) -> u64 {
        self.mispredictions * MISPREDICT_PENALTY
    }
}

/// Đếm phần tử lớn hơn ngưỡng, CÓ nhánh. Trên dữ liệu ĐÃ SẮP XẾP, nhánh cực
/// dễ đoán (một chuỗi dài "không" rồi một chuỗi dài "có"). Trên dữ liệu lộn
/// xộn, nó gần như tung đồng xu.
pub fn branch_taken_count(
    data: &[i32],
    threshold: i32,
    predictor: &mut BranchPredictor,
) -> (usize, u64) {
    let mut count = 0;
    for &x in data {
        let cond = x >= threshold;
        predictor.predict(0xB1, cond); // một vị trí nhánh duy nhất
        if cond {
            count += 1;
        }
    }
    (count, predictor.mispredictions)
}

/// Cùng phép tính nhưng KHÔNG có nhánh: biến điều kiện thành số học.
/// CPU không phải đoán gì cả → không bao giờ đoán sai.
pub fn branch_not_taken_count(data: &[i32], threshold: i32) -> usize {
    data.iter().map(|&x| (x >= threshold) as usize).sum()
}

// ============================================================================
// 5. SONG SONG MỨC LỆNH
// ============================================================================
// CPU hiện đại chạy 4–6 lệnh mỗi chu kỳ — NẾU chúng độc lập. Một chuỗi phụ
// thuộc (mỗi lệnh cần kết quả lệnh trước) làm mọi cổng thực thi khác ngồi chơi.

#[derive(Debug, PartialEq)]
pub struct IlpAnalysis {
    pub compute_op_count: u64,
    /// Chuỗi phụ thuộc dài nhất — cận dưới của số chu kỳ, bất kể CPU rộng bao nhiêu.
    pub critical_path: u64,
    pub ilp: f64,
    /// Số chu kỳ ước tính trên CPU rộng `width` lệnh/chu kỳ.
    pub estimated_cycles: u64,
}

/// Cộng dồn vào MỘT biến: mỗi phép cộng phải chờ phép trước.
/// Đường tới hạn = n. CPU rộng 4 cũng vô dụng.
pub fn analyze_single_accumulator(n: u64, _width: u64) -> IlpAnalysis {
    IlpAnalysis {
        compute_op_count: n,
        critical_path: n,
        ilp: 1.0,
        estimated_cycles: n.max(1), // bị chặn bởi chuỗi phụ thuộc, không bởi độ rộng
    }
}

/// Cộng dồn vào `k` biến rồi gộp cuối: `k` chuỗi độc lập chạy song song.
/// Đây là "bung vòng lặp có nhiều bộ tích luỹ" — thủ thuật hiệu năng cổ điển.
pub fn analyze_multi_accumulator(n: u64, k: u64, width: u64) -> IlpAnalysis {
    let k = k.max(1);
    // Mỗi chuỗi dài n/k, cộng thêm log2(k) bước gộp các bộ tích luỹ lại
    let critical_path = n / k + k.next_power_of_two().trailing_zeros() as u64;
    IlpAnalysis {
        compute_op_count: n,
        critical_path,
        ilp: n as f64 / critical_path.max(1) as f64,
        estimated_cycles: critical_path.max(n / width.max(1)),
    }
}

/// Kiểm chứng: nhiều bộ tích luỹ phải cho CÙNG kết quả với một bộ.
pub fn sum_single(data: &[i64]) -> i64 {
    data.iter().sum()
}

pub fn sum_multi(data: &[i64], k: usize) -> i64 {
    let k = k.max(1);
    let mut acc = vec![0i64; k];
    for (i, &x) in data.iter().enumerate() {
        acc[i % k] += x;
    }
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
    pub num_elements: usize,
    pub vector_width: usize,
    pub vector_ops: usize,
    pub remainder_elements: usize,
    pub theoretical_speedup: f64,
}

pub fn simd_analysis(num_elements: usize, vector_width: usize) -> SimdAnalysis {
    let w = vector_width.max(1);
    let remainder = num_elements % w;
    let vector_ops = num_elements / w;
    // Phần dư phải xử lý từng phần tử một — đó là cái giá của mảng không chia hết
    let total_ops = vector_ops + remainder;
    SimdAnalysis {
        num_elements,
        vector_width: w,
        vector_ops,
        remainder_elements: remainder,
        theoretical_speedup: if total_ops == 0 {
            1.0
        } else {
            num_elements as f64 / total_ops as f64
        },
    }
}

/// Cộng hai mảng theo lô `w` phần tử — mô phỏng cách trình biên dịch vector hoá.
pub fn batch_add_array(a: &[f64], b: &[f64], w: usize) -> Vec<f64> {
    let n = a.len().min(b.len());
    let mut out = vec![0.0; n];
    let w = w.max(1);
    let full_end = n - n % w;
    for i in (0..full_end).step_by(w) {
        for j in 0..w {
            out[i + j] = a[i + j] + b[i + j];
        }
    }
    for i in full_end..n {
        out[i] = a[i] + b[i];
    }
    out
}

// ============================================================================
// 7. SINH DỮ LIỆU TẤT ĐỊNH
// ============================================================================

pub fn gen_data(n: usize, seed: u64) -> Vec<i32> {
    let mut s = seed;
    (0..n)
        .map(|_| {
            s = s
                .wrapping_mul(6364136223846793005)
                .wrapping_add(1442695040888963407);
            ((s >> 33) % 256) as i32
        })
        .collect()
}

fn main() {
    println!("═══════════════════════════════════════════════════════════");
    println!("   KỸ NGHỆ HIỆU NĂNG CPU: CACHE · NHÁNH · ILP · SIMD       ");
    println!("═══════════════════════════════════════════════════════════");

    println!("\n1. PHÂN CẤP BỘ NHỚ — những con số cần thuộc");
    println!("   {:<12} {:>14} {:>16}", "tầng", "chu kỳ", "so với L1");
    for t in MemoryTier::all() {
        println!(
            "   {:<12} {:>14} {:>15.2}x",
            t.name(),
            t.cycles(),
            t.cycles() as f64 / MemoryTier::L1.cycles() as f64
        );
    }
    println!("   → Một lần trượt xuống RAM tốn bằng 75 lần chạm L1.");

    println!("\n2. CỤC BỘ CACHE — cùng phép duyệt, khác thứ tự");
    let mut cache = CacheSim::new(32 * 1024, 8); // L1 32 KB, 8 đường
    let n = 256;
    let row_misses = row_major_scan(&mut cache, n, 8);
    let row_cycles = cache.stats.total_cycles();
    let col_misses = col_major_scan(&mut cache, n, 8);
    let col_cycles = cache.stats.total_cycles();
    println!("   Ma trận {}x{} f64 ({} KB):", n, n, n * n * 8 / 1024);
    println!(
        "   Theo hàng: {:>8} lần trượt · {:>10} chu kỳ",
        row_misses, row_cycles
    );
    println!(
        "   Theo cột : {:>8} lần trượt · {:>10} chu kỳ",
        col_misses, col_cycles
    );
    println!(
        "   → Cùng {} phép truy cập, chỉ khác thứ tự, chậm gấp {:.1} lần.",
        n * n,
        col_cycles as f64 / row_cycles as f64
    );

    println!("\n3. NHÂN MA TRẬN — chia khối để tái dùng dữ liệu nóng");
    let n = 96;
    let mut cache = CacheSim::new(32 * 1024, 8);
    let naive = matmul_naive(&mut cache, n, 8);
    println!("   Ngây thơ (i-j-k): {:>9} lần trượt", naive);
    for k in [8usize, 16, 32] {
        let mut cache2 = CacheSim::new(32 * 1024, 8);
        let blocked = blocked_matmul(&mut cache2, n, k, 8);
        println!(
            "   Chia khối {:>2}x{:<2}   : {:>9} lần trượt → giảm {:.0}%",
            k,
            k,
            blocked,
            (1.0 - blocked as f64 / naive as f64) * 100.0
        );
    }
    println!("   → CÙNG số phép nhân. Chỉ đổi thứ tự truy cập bộ nhớ.");

    println!("\n4. DỰ ĐOÁN NHÁNH — vì sao sắp xếp trước lại nhanh hơn");
    let shuffled = gen_data(100_000, 42);
    let mut sorted = shuffled.clone();
    sorted.sort_unstable();
    for (name, d) in [("lộn xộn ", &shuffled), ("đã sắp  ", &sorted)] {
        let mut predictor = BranchPredictor::new();
        let (count, mispredicts) = branch_taken_count(d, 128, &mut predictor);
        println!(
            "   {} → {} phần tử · {:>6} lần đoán sai ({:>5.1}%) · phí {:>8} chu kỳ",
            name,
            count,
            mispredicts,
            predictor.mispredict_ratio() * 100.0,
            predictor.wasted_cycles()
        );
    }
    println!(
        "   Bản KHÔNG NHÁNH: {} phần tử · 0 lần đoán sai · 0 chu kỳ phí",
        branch_not_taken_count(&sorted, 128)
    );
    println!("   → Sắp xếp trước không làm phép đếm nhanh hơn; nó làm CPU ĐOÁN ĐÚNG hơn.");

    println!("\n5. SONG SONG MỨC LỆNH");
    let n = 1_000_000u64;
    println!(
        "   {:<22} {:>14} {:>8} {:>16}",
        "cách viết", "đường tới hạn", "ILP", "chu kỳ ước tính"
    );
    let a = analyze_single_accumulator(n, 4);
    println!(
        "   {:<22} {:>14} {:>8.1} {:>16}",
        "1 bộ tích luỹ", a.critical_path, a.ilp, a.estimated_cycles
    );
    for k in [2u64, 4, 8] {
        let b = analyze_multi_accumulator(n, k, 4);
        println!(
            "   {:<22} {:>14} {:>8.1} {:>16}",
            format!("{} bộ tích luỹ", k),
            b.critical_path,
            b.ilp,
            b.estimated_cycles
        );
    }
    let d: Vec<i64> = (1..=1000).collect();
    println!(
        "   Kết quả vẫn giống hệt nhau: {}",
        sum_single(&d) == sum_multi(&d, 4)
    );

    println!("\n6. SIMD");
    println!(
        "   {:>10} {:>10} {:>14} {:>10} {:>12}",
        "phần tử", "bề rộng", "lệnh vector", "phần dư", "tăng tốc"
    );
    for (n, w) in [(1024usize, 4usize), (1024, 8), (1001, 8), (7, 8)] {
        let p = simd_analysis(n, w);
        println!(
            "   {:>10} {:>10} {:>14} {:>10} {:>11.2}x",
            p.num_elements,
            p.vector_width,
            p.vector_ops,
            p.remainder_elements,
            p.theoretical_speedup
        );
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
    fn latency_grows_with_distance() {
        let t = MemoryTier::all();
        for w in t.windows(2) {
            assert!(
                w[0].cycles() < w[1].cycles(),
                "{} phải nhanh hơn {}",
                w[0].name(),
                w[1].name()
            );
        }
    }

    #[test]
    fn the_gap_between_levels_is_an_order_of_magnitude() {
        assert_eq!(
            MemoryTier::Ram.cycles() / MemoryTier::L1.cycles(),
            75,
            "trượt xuống RAM tốn bằng 75 lần chạm L1"
        );
        assert!(
            MemoryTier::SsdNvme.cycles() > MemoryTier::Ram.cycles() * 500,
            "SSD chậm hơn RAM cả ba bậc độ lớn"
        );
    }

    // ---------- Mô phỏng cache ----------
    #[test]
    fn first_touch_misses_then_hits() {
        let mut cache = CacheSim::new(32 * 1024, 8);
        assert!(!cache.access(0), "lần đầu phải trượt");
        assert!(cache.access(0), "lần hai phải trúng");
        assert_eq!(cache.stats.compulsory_miss, 1);
        assert_eq!(cache.stats.capacity_miss, 0);
    }

    #[test]
    fn a_whole_cache_line_arrives_at_once() {
        // Chạm byte 0 thì byte 1..63 cũng vào cache theo — đó chính là lý do
        // duyệt tuần tự nhanh hơn duyệt nhảy cóc.
        let mut cache = CacheSim::new(32 * 1024, 8);
        cache.access(0);
        for b in 1..CACHE_LINE_BYTES {
            assert!(
                cache.access(b),
                "byte {} phải nằm cùng dòng cache với byte 0",
                b
            );
        }
        assert_eq!(cache.stats.miss_count, 1, "64 byte chỉ tốn MỘT lần trượt");
    }

    #[test]
    fn a_cache_line_stride_misses_every_time() {
        let mut cache = CacheSim::new(32 * 1024, 8);
        for i in 0..100 {
            cache.access(i * CACHE_LINE_BYTES);
        }
        assert_eq!(cache.stats.miss_count, 100, "mỗi lần chạm một dòng mới");
        assert_eq!(cache.stats.hit_ratio(), 0.0);
    }

    #[test]
    fn data_beyond_capacity_gets_evicted() {
        // Cache 1 KB = 16 dòng. Quét vòng qua 64 dòng thì lần nào cũng trượt.
        let mut cache = CacheSim::new(1024, 4);
        for _ in 0..3 {
            for i in 0..64 {
                cache.access(i * CACHE_LINE_BYTES);
            }
        }
        assert!(cache.stats.capacity_miss > 0, "phải có trượt do bị đá ra");
        assert!(
            cache.stats.hit_ratio() < 0.1,
            "quét vòng lớn hơn cache → gần như trượt hết"
        );
    }

    #[test]
    fn data_that_fits_hits_on_the_second_pass() {
        let mut cache = CacheSim::new(32 * 1024, 8); // 512 dòng
        for _ in 0..5 {
            for i in 0..100 {
                cache.access(i * CACHE_LINE_BYTES);
            }
        }
        assert_eq!(
            cache.stats.miss_count, 100,
            "chỉ 100 lần trượt bắt buộc, sau đó trúng hết"
        );
        assert_eq!(cache.stats.capacity_miss, 0);
        assert!(cache.stats.hit_ratio() > 0.79);
    }

    #[test]
    fn the_counters_always_balance() {
        let mut cache = CacheSim::new(4096, 4);
        for i in 0..1000 {
            cache.access(i * 7);
        }
        assert_eq!(
            cache.stats.hit_count + cache.stats.miss_count,
            cache.stats.access_count
        );
        assert_eq!(
            cache.stats.compulsory_miss + cache.stats.capacity_miss,
            cache.stats.miss_count
        );
    }

    // ---------- Cục bộ ----------
    #[test]
    fn row_major_misses_far_less_than_column_major() {
        // Đây là bài học trung tâm của chương.
        let mut cache = CacheSim::new(32 * 1024, 8);
        let n = 256;
        let row = row_major_scan(&mut cache, n, 8);
        let col = col_major_scan(&mut cache, n, 8);
        assert!(
            col > row * 5,
            "theo cột {} lần trượt phải nhiều hơn hẳn theo hàng {}",
            col,
            row
        );
        // Theo hàng: mỗi dòng cache 64 byte phục vụ 8 phần tử f64
        assert_eq!(
            row,
            (n * n / 8) as u64,
            "đúng bằng số dòng cache của cả ma trận"
        );
    }

    #[test]
    fn both_orders_touch_the_same_number_of_cells() {
        let mut cache = CacheSim::new(32 * 1024, 8);
        let n = 64;
        row_major_scan(&mut cache, n, 8);
        let a = cache.stats.access_count;
        col_major_scan(&mut cache, n, 8);
        assert_eq!(
            a, cache.stats.access_count,
            "cùng khối lượng việc, chỉ khác thứ tự"
        );
    }

    #[test]
    fn smaller_elements_pack_more_per_line() {
        let mut cache = CacheSim::new(32 * 1024, 8);
        let n = 128;
        let f64_ = row_major_scan(&mut cache, n, 8);
        let f32_ = row_major_scan(&mut cache, n, 4);
        assert!(f32_ < f64_, "dùng f32 thay f64 giảm một nửa số lần trượt");
        assert_eq!(f64_, f32_ * 2);
    }

    // ---------- Nhân ma trận ----------
    #[test]
    fn blocking_cuts_the_miss_count() {
        let n = 96;
        let mut a = CacheSim::new(32 * 1024, 8);
        let naive = matmul_naive(&mut a, n, 8);
        let mut b = CacheSim::new(32 * 1024, 8);
        let blocked = blocked_matmul(&mut b, n, 16, 8);
        assert!(
            blocked < naive,
            "chia khối {} phải ít trượt hơn ngây thơ {}",
            blocked,
            naive
        );
    }

    #[test]
    fn blocking_performs_the_same_number_of_accesses() {
        // Bất biến: tối ưu không được đổi KHỐI LƯỢNG VIỆC, chỉ đổi thứ tự.
        let n = 48;
        let mut a = CacheSim::new(32 * 1024, 8);
        matmul_naive(&mut a, n, 8);
        let mut b = CacheSim::new(32 * 1024, 8);
        blocked_matmul(&mut b, n, 16, 8);
        assert_eq!(
            a.stats.access_count, b.stats.access_count,
            "cùng 2·n³ phép truy cập, chỉ khác thứ tự"
        );
        assert_eq!(a.stats.access_count, 2 * (n * n * n) as u64);
    }

    // ---------- Dự đoán nhánh ----------
    #[test]
    fn the_saturating_counter_needs_two_misses_to_flip() {
        // Đây là lý do bộ đếm 2 bit tốt hơn 1 bit: một lần chệch không làm
        // nó đổi ý, nên vòng lặp dài không bị phạt ở lần lặp bất thường.
        let mut d = BranchPredictor::new();
        for _ in 0..10 {
            d.predict(1, true);
        } // học "luôn đúng"
        let prev_mispredicts = d.mispredictions;
        d.predict(1, false); // một lần chệch
        assert_eq!(d.mispredictions, prev_mispredicts + 1);
        assert!(d.predict(1, true), "một lần chệch KHÔNG làm nó đổi ý");
    }

    #[test]
    fn an_always_taken_branch_almost_never_mispredicts() {
        let mut d = BranchPredictor::new();
        for _ in 0..10_000 {
            d.predict(1, true);
        }
        assert!(
            d.mispredictions <= 2,
            "chỉ sai vài lần lúc học, thực tế {}",
            d.mispredictions
        );
        assert!(d.mispredict_ratio() < 0.001);
    }

    #[test]
    fn an_alternating_branch_mispredicts_almost_always() {
        // Trường hợp tệ nhất của bộ đếm 2 bit: mẫu luân phiên.
        let mut d = BranchPredictor::new();
        for i in 0..10_000 {
            d.predict(1, i % 2 == 0);
        }
        assert!(
            d.mispredict_ratio() > 0.4,
            "mẫu luân phiên phải làm nó sai rất nhiều"
        );
    }

    #[test]
    fn sorted_data_mispredicts_far_less() {
        // Câu hỏi phỏng vấn kinh điển: "vì sao sắp xếp mảng trước lại làm
        // vòng lặp đếm chạy nhanh hơn?" — không phải vì phép đếm nhanh hơn,
        // mà vì CPU đoán nhánh đúng hơn.
        let shuffled = gen_data(50_000, 42);
        let mut sorted = shuffled.clone();
        sorted.sort_unstable();

        let mut d1 = BranchPredictor::new();
        let (a, shuffled_mispredicts) = branch_taken_count(&shuffled, 128, &mut d1);
        let mut d2 = BranchPredictor::new();
        let (b, sorted_mispredicts) = branch_taken_count(&sorted, 128, &mut d2);

        assert_eq!(a, b, "kết quả phải giống hệt — chỉ hiệu năng khác");
        assert!(
            sorted_mispredicts * 20 < shuffled_mispredicts,
            "đã sắp: {} lần sai, lộn xộn: {} lần sai",
            sorted_mispredicts,
            shuffled_mispredicts
        );
        assert!(d1.wasted_cycles() > d2.wasted_cycles() * 20);
    }

    #[test]
    fn the_branchless_version_gives_the_same_result() {
        for seed in [1u64, 42, 2024] {
            let d = gen_data(10_000, seed);
            let mut predictor = BranchPredictor::new();
            let (a, _) = branch_taken_count(&d, 128, &mut predictor);
            assert_eq!(
                a,
                branch_not_taken_count(&d, 128),
                "mã không nhánh phải cho cùng đáp số"
            );
        }
    }

    #[test]
    fn the_branchless_version_never_mispredicts() {
        // Không có nhánh thì không có gì để đoán — và không có gì để đoán sai.
        // Đây cũng là nền của mã mật mã chạy thời gian không đổi (Chương 57).
        let shuffled = gen_data(10_000, 7);
        let predictor = BranchPredictor::new();
        branch_not_taken_count(&shuffled, 128);
        assert_eq!(predictor.mispredictions, 0);
        assert_eq!(predictor.wasted_cycles(), 0);
    }

    // ---------- ILP ----------
    #[test]
    fn one_accumulator_is_bound_by_the_dependency_chain() {
        let a = analyze_single_accumulator(1_000_000, 4);
        assert_eq!(
            a.ilp, 1.0,
            "chuỗi phụ thuộc thuần → không song song được gì"
        );
        assert_eq!(
            a.estimated_cycles, 1_000_000,
            "CPU rộng 4 lệnh/chu kỳ cũng không giúp được gì"
        );
    }

    #[test]
    fn multiple_accumulators_raise_ilp() {
        let mut prev_ilp = 0.0;
        for k in [1u64, 2, 4, 8] {
            let b = analyze_multi_accumulator(1_000_000, k, 4);
            assert!(b.ilp > prev_ilp, "k={} phải cho ILP cao hơn", k);
            prev_ilp = b.ilp;
        }
        let b4 = analyze_multi_accumulator(1_000_000, 4, 4);
        assert!(
            b4.ilp > 3.9,
            "4 bộ tích luỹ phải đạt ILP gần 4, thực tế {:.2}",
            b4.ilp
        );
    }

    #[test]
    fn cpu_width_caps_the_speedup() {
        // Dù có 64 bộ tích luỹ, CPU rộng 4 vẫn chỉ chạy 4 lệnh mỗi chu kỳ.
        let b = analyze_multi_accumulator(1_000_000, 64, 4);
        assert!(
            b.estimated_cycles >= 1_000_000 / 4,
            "không thể nhanh hơn giới hạn độ rộng CPU"
        );
    }

    #[test]
    fn multiple_accumulators_give_the_same_result() {
        // Cộng số nguyên có tính kết hợp nên đổi thứ tự vẫn đúng.
        // (Với f64 thì KHÔNG — đó là lý do trình biên dịch không tự làm việc
        // này cho số thực trừ khi bạn cho phép nới lỏng ngữ nghĩa dấu phẩy động.)
        let d: Vec<i64> = (1..=10_000).collect();
        let expected = sum_single(&d);
        for k in [1usize, 2, 3, 4, 8, 16] {
            assert_eq!(sum_multi(&d, k), expected, "k={}", k);
        }
    }

    #[test]
    fn the_sum_of_an_empty_slice_is_zero() {
        assert_eq!(sum_single(&[]), 0);
        assert_eq!(sum_multi(&[], 4), 0);
    }

    // ---------- SIMD ----------
    #[test]
    fn simd_speedup_equals_the_width_when_it_divides_evenly() {
        let p = simd_analysis(1024, 4);
        assert_eq!(p.vector_ops, 256);
        assert_eq!(p.remainder_elements, 0);
        assert!((p.theoretical_speedup - 4.0).abs() < 1e-9);
    }

    #[test]
    fn the_remainder_erodes_the_speedup() {
        let even = simd_analysis(1024, 8);
        assert_eq!(even.remainder_elements, 0);
        let odd = simd_analysis(1001, 8);
        assert_eq!(odd.remainder_elements, 1);
        assert!(odd.theoretical_speedup < even.theoretical_speedup);
    }

    #[test]
    fn simd_is_useless_on_a_tiny_slice() {
        // 7 phần tử với vector 8 làn: không lô nào đầy, mọi phần tử xử lý lẻ.
        let p = simd_analysis(7, 8);
        assert_eq!(p.vector_ops, 0);
        assert_eq!(p.remainder_elements, 7);
        assert!(
            (p.theoretical_speedup - 1.0).abs() < 1e-9,
            "không tăng tốc chút nào"
        );
    }

    #[test]
    fn an_unusual_simd_width_breaks_nothing() {
        let p = simd_analysis(100, 1);
        assert!((p.theoretical_speedup - 1.0).abs() < 1e-9);
        let p0 = simd_analysis(100, 0);
        assert_eq!(p0.vector_width, 1, "bề rộng 0 phải được chặn thành 1");
        let empty = simd_analysis(0, 8);
        assert!(
            (empty.theoretical_speedup - 1.0).abs() < 1e-9,
            "mảng rỗng không panic"
        );
    }

    #[test]
    fn batch_add_matches_scalar_add() {
        let a: Vec<f64> = (0..103).map(|i| i as f64).collect();
        let b: Vec<f64> = (0..103).map(|i| (i * 2) as f64).collect();
        let expected: Vec<f64> = a.iter().zip(b.iter()).map(|(x, y)| x + y).collect();
        for w in [1usize, 2, 4, 8, 16] {
            assert_eq!(
                batch_add_array(&a, &b, w),
                expected,
                "vector hoá bề rộng {} phải cho cùng kết quả",
                w
            );
        }
    }

    #[test]
    fn adding_unequal_slices_uses_the_common_prefix() {
        let a = vec![1.0, 2.0, 3.0];
        let b = vec![10.0, 20.0];
        assert_eq!(batch_add_array(&a, &b, 4), vec![11.0, 22.0]);
    }

    // ---------- Sinh dữ liệu ----------
    #[test]
    fn data_generation_is_deterministic() {
        assert_eq!(gen_data(100, 5), gen_data(100, 5));
        assert_ne!(gen_data(100, 5), gen_data(100, 6));
    }

    #[test]
    fn generated_data_straddles_the_threshold_evenly() {
        let d = gen_data(100_000, 42);
        let above = d.iter().filter(|&&x| x >= 128).count();
        assert!(
            (above as f64 / d.len() as f64 - 0.5).abs() < 0.05,
            "phải chia đôi quanh ngưỡng để nhánh thật sự khó đoán"
        );
    }
}
```

---

## Bảng tra cứu lỗi biên dịch thường gặp

| Lỗi | Nguyên nhân trong chương này | Cách sửa |
|---|---|---|
| Kết quả đo bằng 0 | Trình tối ưu xoá vòng lặp không dùng kết quả | `std::hint::black_box` quanh giá trị |
| Kết quả 4 biến tích luỹ khác 1 biến | Cộng số thực không có tính kết hợp | Đúng như dự kiến; kiểm bằng sai số tương đối, không bằng `==` |
| `E0308: mismatched types` (expected `usize`, found `u64`) | Trộn chỉ số với địa chỉ, vd `let i: usize = addr;` | Ép kiểu tường minh ở biên |
| SIMD bỏ sót phần tử cuối | Quên xử lý phần dư | `num_elements % vector_width` luôn phải có nhánh xử lý (xem `batch_add_array`) |
| Đo cache ra kết quả vô nghĩa | Bộ nạp trước (prefetcher) đoán đúng mẫu truy cập | Dùng bước nhảy không đều để đánh lừa nó |

---

## Tóm tắt chương & Bài tập rèn luyện

### 5 điểm cốt lõi

1. **RAM chậm hơn L1 khoảng 75 lần.** Bố cục bộ nhớ thường quan trọng hơn số phép tính.
2. **Dòng cache (Cache line) 64 byte là đơn vị thật.** Duyệt theo hàng dùng hết; duyệt theo cột vứt đi 87,5%.
3. **Chia khối giữ nguyên số phép tính nhưng giảm trượt cache một bậc.**
4. **Mã không rẽ nhánh (Branchless code) thắng với dữ liệu ngẫu nhiên, thua với dữ liệu đã sắp xếp.** Phải biết dữ liệu.
5. **Chuỗi phụ thuộc (Dependency chain) chặn ILP.** Nhiều biến tích luỹ phá chuỗi và tăng thông lượng gần tuyến tính.

### Bài tập rèn luyện

**Bài 1.** Cài **chuyển vị ma trận thân thiện cache** và so với bản ngây thơ.

<details>
<summary><b>Gợi ý</b></summary>

Chuyển vị ngây thơ luôn có một phía truy cập theo cột — bất kể bạn xoay vòng lặp thế nào. Chia khối giải quyết cả hai phía cùng lúc: mỗi khối nhỏ vừa L1, nên cả đọc lẫn ghi đều nằm trong cache.
</details>

<details>
<summary><b>Lời giải</b></summary>

```rust
pub fn transpose_naive(a: &[f64], n: usize) -> Vec<f64> {
    let mut r = vec![0.0; n * n];
    for i in 0..n {
        for j in 0..n {
            r[j * n + i] = a[i * n + j]; // ghi theo cột → trượt mỗi phần tử
        }
    }
    r
}

pub fn transpose_blocked(a: &[f64], n: usize, block: usize) -> Vec<f64> {
    let mut r = vec![0.0; n * n];
    for ii in (0..n).step_by(block) {
        for jj in (0..n).step_by(block) {
            let end_i = (ii + block).min(n);
            let end_j = (jj + block).min(n);
            // Cả khối đọc lẫn khối ghi đều vừa L1 → không trượt bên trong
            for i in ii..end_i {
                for j in jj..end_j {
                    r[j * n + i] = a[i * n + j];
                }
            }
        }
    }
    r
}

/// Kích thước khối tối ưu: hai khối (đọc + ghi) phải vừa L1.
pub fn optimal_block(l1_bytes: usize) -> usize {
    // 2 khối × B² × 8 byte ≤ L1  →  B ≤ √(L1 / 16)
    ((l1_bytes as f64 / 16.0).sqrt() as usize).max(8)
}
```

Với L1 32 KB, `optimal_block` cho B = 45, nên thực tế dùng 32 hoặc 64 (luỹ thừa 2 cho phép tính chỉ số rẻ hơn). Với ma trận 1024×1024, chuyển vị theo khối thường nhanh hơn 3–5 lần.
</details>

**Bài 2.** Cài **tổng tiền tố (prefix sum) SIMD** — bài toán tưởng như không song song hoá được.

<details>
<summary><b>Gợi ý</b></summary>

Tổng tiền tố có phụ thuộc tuần tự: `s[i] = s[i-1] + a[i]`. Nhưng nó song song hoá được bằng thuật toán **quét Hillis–Steele**: dịch và cộng với bước nhảy 1, 2, 4, 8… Tổng cộng `log₂(n)` bước thay vì `n` bước.
</details>

<details>
<summary><b>Lời giải</b></summary>

```rust
pub fn prefix_sum_sequential(a: &[f64]) -> Vec<f64> {
    let mut r = Vec::with_capacity(a.len());
    let mut s = 0.0;
    for x in a {
        s += x;
        r.push(s);
    }
    r
}

/// Quét Hillis–Steele: log₂(n) bước, mỗi bước song song hoàn toàn.
/// Tốn nhiều phép cộng hơn (n·log n so với n) nhưng ĐỘ SÂU ngắn hơn nhiều.
pub fn prefix_sum_scan(a: &[f64]) -> Vec<f64> {
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

/// Chia khối: quét trong từng khối (các khối độc lập → song song được), rồi
/// cộng dồn phần bù giữa các khối. Đây là cách các thư viện thật làm — cân
/// bằng giữa độ sâu và tổng công.
pub fn prefix_sum_blocked(a: &[f64], block: usize) -> Vec<f64> {
    let mut r = a.to_vec();
    for c in r.chunks_mut(block) {
        let mut s = 0.0;
        for x in c.iter_mut() {
            s += *x;
            *x = s;
        }
    }
    let mut carry = 0.0;
    for c in r.chunks_mut(block) {
        for x in c.iter_mut() {
            *x += carry;
        }
        carry = *c.last().unwrap();
    }
    r
}
```

Đây là ví dụ tổng quát của một nguyên tắc quan trọng: **thuật toán song song thường làm nhiều việc hơn nhưng có độ sâu ngắn hơn**. Quét Hillis–Steele tốn `n log n` phép cộng thay vì `n`, nhưng độ sâu chỉ `log n` thay vì `n` — và trên phần cứng song song, độ sâu mới là thứ quyết định.
</details>
