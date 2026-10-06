#![allow(dead_code)]
//! Chương 81 — Lập trình GPU: mô hình thực thi SIMT, phân kỳ warp, gộp truy
//! cập bộ nhớ, bộ nhớ chia sẻ và xung đột ngân hàng, rút gọn song song, và
//! nhân ma trận theo lát.
//!
//! Theo phân loại bài tập của [LeetGPU](https://leetgpu.com/) — 99 bài chia
//! ba mức, từ cộng vector tới khối transformer. Ở đây ta ĐẾM số giao dịch bộ
//! nhớ và số lần thực thi bị tuần tự hoá bằng mô phỏng tất định, thay vì đo
//! đồng hồ — nhờ vậy kiểm thử được mà không cần GPU.
//!
//! Rust chạm tới GPU qua `wgpu` (đa nền tảng, dùng WGSL — xem Chương 63),
//! `cudarc`/`cust` (ràng buộc CUDA), hoặc `rust-gpu` (biên dịch Rust sang SPIR-V).

// ============================================================================
// 1. MÔ HÌNH THỰC THI SIMT
// ============================================================================
// CPU: vài lõi mạnh, mỗi lõi chạy một luồng khác nhau, có dự đoán nhánh xịn.
// GPU: hàng nghìn lõi yếu, gom thành nhóm 32 luồng gọi là WARP. Cả warp thực
// thi CÙNG MỘT lệnh trên dữ liệu khác nhau — "một lệnh, nhiều luồng" (SIMT).

pub const THREADS_PER_WARP: usize = 32;

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct LaunchConfig {
    pub num_blocks: usize,
    pub threads_per_block: usize,
}

impl LaunchConfig {
    /// Cách phát chuẩn: đủ luồng để phủ hết `n` phần tử, làm tròn LÊN.
    pub fn for_n_items(n: usize, threads_per_block: usize) -> Self {
        let l = threads_per_block.max(1);
        LaunchConfig {
            num_blocks: n.div_ceil(l),
            threads_per_block: l,
        }
    }
    pub fn total_threads(&self) -> usize {
        self.num_blocks * self.threads_per_block
    }

    /// Số warp mỗi khối. Nếu `threads_per_block` không chia hết 32 thì warp cuối
    /// chạy thiếu luồng — phần cứng vẫn tốn nguyên một warp cho nó.
    pub fn warps_per_block(&self) -> usize {
        self.threads_per_block.div_ceil(THREADS_PER_WARP)
    }

    /// Số làn bị lãng phí ở warp cuối của mỗi khối.
    pub fn wasted_per_block(&self) -> usize {
        self.warps_per_block() * THREADS_PER_WARP - self.threads_per_block
    }

    /// Số luồng chạy nhưng không có việc (vì `total_amount` > n).
    pub fn idle_threads(&self, n: usize) -> usize {
        self.total_threads().saturating_sub(n)
    }
}

// ============================================================================
// 2. PHÂN KỲ WARP — cái bẫy lớn nhất của người mới
// ============================================================================
// Cả warp chạy CÙNG một lệnh. Gặp `if` mà 32 luồng chia hai phe, phần cứng
// buộc phải chạy nhánh `then` (tắt phe kia), rồi chạy nhánh `else` (tắt phe
// này). Hai nhánh chạy TUẦN TỰ — warp mất gấp đôi thời gian.
//
// Điểm mấu chốt: phân kỳ chỉ tính TRONG một warp. Nếu warp 0 toàn đi nhánh A
// và warp 1 toàn đi nhánh B thì KHÔNG có phân kỳ nào cả.

#[derive(Debug, PartialEq)]
pub struct DivergenceAnalysis {
    pub num_warps: usize,
    pub divergent_warps: usize,
    /// Tổng "lượt thực thi nhánh" — warp không phân kỳ tốn 1, phân kỳ tốn 2.
    pub execution_pass: usize,
    pub slowdown: f64,
}

/// `dieu_kien[i]` là kết quả `if` của luồng thứ `i`.
pub fn divergence_analysis(conditions: &[bool]) -> DivergenceAnalysis {
    let num_warps = conditions.len().div_ceil(THREADS_PER_WARP);
    let mut divergence = 0;
    let mut passes = 0;
    for w in conditions.chunks(THREADS_PER_WARP) {
        let any_true = w.iter().any(|&x| x);
        let any_false = w.iter().any(|&x| !x);
        if any_true && any_false {
            divergence += 1;
            passes += 2;
        } else {
            passes += 1;
        }
    }
    DivergenceAnalysis {
        num_warps,
        divergent_warps: divergence,
        execution_pass: passes,
        slowdown: if num_warps == 0 {
            1.0
        } else {
            passes as f64 / num_warps as f64
        },
    }
}

/// Cách viết TỆ: rẽ nhánh theo tính chẵn lẻ của chỉ số luồng.
/// Trong mỗi warp có 16 luồng chẵn và 16 luồng lẻ → phân kỳ 100%.
pub fn branch_on_parity(n: usize) -> Vec<bool> {
    (0..n).map(|i| i.is_multiple_of(2)).collect()
}

/// Cách viết TỐT: rẽ nhánh theo chỉ số WARP. Mỗi warp đi trọn một nhánh
/// → không warp nào phân kỳ, dù tỉ lệ hai nhánh vẫn là 50/50.
pub fn branch_on_warp(n: usize) -> Vec<bool> {
    (0..n)
        .map(|i| (i / THREADS_PER_WARP).is_multiple_of(2))
        .collect()
}

// ============================================================================
// 3. GỘP TRUY CẬP BỘ NHỚ
// ============================================================================
// Bộ nhớ toàn cục của GPU phục vụ theo GIAO DỊCH 128 byte. Nếu 32 luồng trong
// warp đọc 32 số f32 LIỀN NHAU, cả warp gói gọn trong 1 giao dịch. Nếu chúng
// đọc cách quãng, mỗi luồng có thể tốn một giao dịch riêng — chậm gấp 32 lần
// dù đọc cùng số byte có ích.

pub const TRANSACTION_BYTES: usize = 128;

#[derive(Debug, PartialEq)]
pub struct CoalescingAnalysis {
    pub quantity: usize,
    pub num_transactions: usize,
    pub useful_bytes: usize,
    pub bytes_transferred: usize,
    /// Tỉ lệ băng thông thực sự dùng được. 1.0 = hoàn hảo.
    pub efficiency: f64,
}

/// Đếm số giao dịch bộ nhớ cho một warp truy cập theo `buoc_nhay`.
pub fn coalescing_analysis(
    quantity: usize,
    bytes_per_element: usize,
    stride: usize,
) -> CoalescingAnalysis {
    let mut segments = std::collections::HashSet::new();
    for i in 0..quantity {
        let address = i * stride * bytes_per_element;
        segments.insert(address / TRANSACTION_BYTES);
    }
    let num_transactions = segments.len();
    let useful = quantity * bytes_per_element;
    let transferred = num_transactions * TRANSACTION_BYTES;
    CoalescingAnalysis {
        quantity,
        num_transactions,
        useful_bytes: useful,
        bytes_transferred: transferred,
        efficiency: if transferred == 0 {
            0.0
        } else {
            useful as f64 / transferred as f64
        },
    }
}

// ============================================================================
// 4. BỘ NHỚ CHIA SẺ & XUNG ĐỘT NGÂN HÀNG
// ============================================================================
// Bộ nhớ chia sẻ nhanh gần bằng thanh ghi, nhưng chia thành 32 NGÂN HÀNG.
// Hai luồng cùng warp chạm hai địa chỉ khác nhau trên CÙNG một ngân hàng thì
// phải xếp hàng. Ngân hàng = chỉ_số % 32 (với phần tử 4 byte).

pub const NUM_BANKS: usize = 32;

#[derive(Debug, PartialEq)]
pub struct BankAnalysis {
    /// Số luồng nhiều nhất dồn vào một ngân hàng — đúng bằng số lượt xếp hàng.
    pub conflict_degree: usize,
    pub has_conflict: bool,
}

pub fn bank_analysis(word_indices: &[usize]) -> BankAnalysis {
    let mut count = [0usize; NUM_BANKS];
    for &i in word_indices {
        count[i % NUM_BANKS] += 1;
    }
    let degree = count.iter().copied().max().unwrap_or(0);
    BankAnalysis {
        conflict_degree: degree,
        has_conflict: degree > 1,
    }
}

/// Truy cập lát ma trận theo CỘT với bề rộng 32: mọi luồng rơi vào CÙNG một
/// ngân hàng → xung đột 32 lối, chậm gấp 32 lần.
pub fn tile_column_access(width: usize) -> Vec<usize> {
    (0..THREADS_PER_WARP).map(|i| i * width).collect()
}

/// Thủ thuật kinh điển: ĐỆM lát thêm một cột. Bề rộng 33 làm chỉ số lệch dần
/// nên 32 luồng rơi vào 32 ngân hàng khác nhau. Tốn thêm 1/32 bộ nhớ để đổi
/// lấy tốc độ gấp 32 lần.
pub fn padded_tile_column_access(width: usize) -> Vec<usize> {
    (0..THREADS_PER_WARP).map(|i| i * (width + 1)).collect()
}

// ============================================================================
// 5. RÚT GỌN SONG SONG
// ============================================================================
// Cộng n số: CPU tuần tự mất n bước. GPU gộp theo CÂY — mỗi bước một nửa số
// luồng cộng cặp của mình → log₂(n) bước. Đây là bài tập nền của LeetGPU.

#[derive(Debug, PartialEq)]
pub struct ReductionResult {
    pub sum: i64,
    pub num_steps: usize,
    /// Tổng số phép cộng thực hiện (bằng nhau ở cả hai cách).
    pub add_op_count: usize,
    /// Số luồng còn hoạt động ở bước cuối — đo mức lãng phí.
    pub active_lanes_last_step: usize,
}

/// Rút gọn theo cây, mô phỏng đúng cách GPU làm.
pub fn parallel_reduce(data: &[i64]) -> ReductionResult {
    if data.is_empty() {
        return ReductionResult {
            sum: 0,
            num_steps: 0,
            add_op_count: 0,
            active_lanes_last_step: 0,
        };
    }
    let mut level: Vec<i64> = data.to_vec();
    let mut num_steps = 0;
    let mut add_op_count = 0;
    let mut last = level.len();
    while level.len() > 1 {
        let mut next_level = Vec::with_capacity(level.len().div_ceil(2));
        for pair in level.chunks(2) {
            if pair.len() == 2 {
                add_op_count += 1;
            }
            next_level.push(pair[0] + pair.get(1).copied().unwrap_or(0));
        }
        last = level.len() / 2;
        level = next_level;
        num_steps += 1;
    }
    ReductionResult {
        sum: level[0],
        num_steps,
        add_op_count,
        active_lanes_last_step: last.max(1),
    }
}

pub fn sequential_reduce(data: &[i64]) -> i64 {
    data.iter().sum()
}

/// Số bước lý thuyết của rút gọn cây.
pub fn reduce_steps(n: usize) -> usize {
    if n <= 1 {
        return 0;
    }
    (n as f64).log2().ceil() as usize
}

// ============================================================================
// 6. NHÂN MA TRẬN THEO LÁT
// ============================================================================
// Bản ngây thơ: mỗi luồng đọc cả một hàng và một cột từ bộ nhớ toàn cục —
// mỗi phần tử bị đọc lại n lần. Bản theo lát: cả khối cùng nạp một lát vào
// bộ nhớ chia sẻ, rồi mọi luồng dùng chung. Số lần đọc toàn cục giảm `lat` lần.

#[derive(Debug, PartialEq)]
pub struct GemmAnalysis {
    pub n: usize,
    pub read_global: u64,
    pub read_shared: u64,
    pub mul_op_count: u64,
    /// Số phép tính trên mỗi byte đọc từ bộ nhớ toàn cục. Càng cao càng tốt —
    /// đây là con số quyết định bài toán bị chặn bởi TÍNH hay bởi BỘ NHỚ.
    pub arithmetic_intensity: f64,
}

pub fn gemm_naive(n: usize) -> GemmAnalysis {
    let n64 = n as u64;
    // Mỗi phần tử kết quả cần đọc n phần tử của A và n của B, tất cả từ toàn cục
    let reads = 2 * n64 * n64 * n64;
    GemmAnalysis {
        n,
        read_global: reads,
        read_shared: 0,
        mul_op_count: n64 * n64 * n64,
        arithmetic_intensity: n64.pow(3) as f64 / (reads * 4) as f64, // 4 byte mỗi f32
    }
}

pub fn tiled_gemm(n: usize, tile: usize) -> GemmAnalysis {
    let n64 = n as u64;
    let l = tile.max(1) as u64;
    // Mỗi lát được nạp một lần rồi dùng lại `lat` lần bởi cả khối
    let read_global = 2 * n64 * n64 * n64 / l;
    let read_shared = 2 * n64 * n64 * n64;
    GemmAnalysis {
        n,
        read_global,
        read_shared,
        mul_op_count: n64 * n64 * n64,
        arithmetic_intensity: n64.pow(3) as f64 / (read_global * 4) as f64,
    }
}

// ============================================================================
// 7. MỨC CHIẾM DỤNG
// ============================================================================
// Mỗi bộ xử lý đa luồng có giới hạn: số thanh ghi, dung lượng bộ nhớ chia sẻ,
// số warp đồng thời. Dùng quá nhiều thanh ghi cho mỗi luồng → ít warp cùng
// chạy → không đủ việc để che độ trễ bộ nhớ.

#[derive(Debug, PartialEq)]
pub struct Occupancy {
    pub concurrent_warps: usize,
    pub max_warps: usize,
    pub ratio: f64,
    pub blocked_by: &'static str,
}

pub fn occupancy(
    threads_per_block: usize,
    regs_per_thread: usize,
    shared_bytes_per_block: usize,
) -> Occupancy {
    const MAX_WARPS: usize = 64;
    const REGS_PER_SM: usize = 65_536;
    const SHARED_PER_SM: usize = 65_536;
    const MAX_BLOCKS: usize = 32;

    let l = threads_per_block.max(1);
    let warps_per_block = l.div_ceil(THREADS_PER_WARP);

    let blocks_by_regs = if regs_per_thread == 0 {
        MAX_BLOCKS
    } else {
        REGS_PER_SM / (l * regs_per_thread).max(1)
    };
    // Không dùng bộ nhớ chia sẻ (chia cho 0) thì ràng buộc này không áp dụng.
    let blocks_by_shared = SHARED_PER_SM
        .checked_div(shared_bytes_per_block)
        .unwrap_or(MAX_BLOCKS);
    let blocks_by_warps = MAX_WARPS / warps_per_block.max(1);

    let (num_blocks, limiter) = [
        (blocks_by_regs, "thanh ghi"),
        (blocks_by_shared, "bộ nhớ chia sẻ"),
        (blocks_by_warps, "số warp"),
        (MAX_BLOCKS, "số khối"),
    ]
    .into_iter()
    .min_by_key(|(v, _)| *v)
    .unwrap();

    let concurrent_warps = (num_blocks * warps_per_block).min(MAX_WARPS);
    Occupancy {
        concurrent_warps,
        max_warps: MAX_WARPS,
        ratio: concurrent_warps as f64 / MAX_WARPS as f64,
        blocked_by: limiter,
    }
}

fn main() {
    println!("═══════════════════════════════════════════════════════════");
    println!("   LẬP TRÌNH GPU: SIMT · PHÂN KỲ · GỘP · LÁT · CHIẾM DỤNG  ");
    println!("═══════════════════════════════════════════════════════════");

    println!("\n1. CẤU HÌNH PHÁT");
    for (n, l) in [
        (1_000_000usize, 256usize),
        (1_000_000, 128),
        (1_000, 256),
        (100, 256),
    ] {
        let c = LaunchConfig::for_n_items(n, l);
        println!(
            "   n={:>9} khối {:>3} luồng → {:>5} khối · {:>9} luồng · thừa {:>5}",
            n,
            l,
            c.num_blocks,
            c.total_threads(),
            c.idle_threads(n)
        );
    }
    let ragged = LaunchConfig {
        num_blocks: 10,
        threads_per_block: 100,
    };
    println!(
        "   Khối 100 luồng → {} warp, lãng phí {} làn ở warp cuối",
        ragged.warps_per_block(),
        ragged.wasted_per_block()
    );
    println!(
        "   → Luôn chọn số luồng mỗi khối là bội số của {}.",
        THREADS_PER_WARP
    );

    println!("\n2. PHÂN KỲ WARP — cùng tỉ lệ 50/50, khác hẳn tốc độ");
    let n = 1024;
    for (name, conds) in [
        ("rẽ theo chẵn/lẻ", branch_on_parity(n)),
        ("rẽ theo warp   ", branch_on_warp(n)),
    ] {
        let p = divergence_analysis(&conds);
        let true_ratio = conds.iter().filter(|&&x| x).count() as f64 / n as f64;
        println!(
            "   {} → {:>2}/{} warp phân kỳ · chậm {:.1}x (tỉ lệ nhánh đúng {:.0}%)",
            name,
            p.divergent_warps,
            p.num_warps,
            p.slowdown,
            true_ratio * 100.0
        );
    }
    println!("   → Cùng 50% luồng đi mỗi nhánh. Chỉ khác CÁCH NHÓM chúng.");

    println!("\n3. GỘP TRUY CẬP BỘ NHỚ (một warp đọc f32)");
    println!(
        "   {:>10} {:>14} {:>14} {:>12}",
        "bước nhảy", "giao dịch", "byte chuyển", "hiệu suất"
    );
    for b in [1usize, 2, 4, 8, 32] {
        let p = coalescing_analysis(THREADS_PER_WARP, 4, b);
        println!(
            "   {:>10} {:>14} {:>14} {:>11.1}%",
            b,
            p.num_transactions,
            p.bytes_transferred,
            p.efficiency * 100.0
        );
    }
    println!(
        "   → Bước nhảy 32 tốn {} giao dịch cho cùng {} byte có ích.",
        coalescing_analysis(32, 4, 32).num_transactions,
        32 * 4
    );

    println!("\n4. XUNG ĐỘT NGÂN HÀNG BỘ NHỚ CHIA SẺ");
    let a = bank_analysis(&tile_column_access(32));
    let b = bank_analysis(&padded_tile_column_access(32));
    println!(
        "   Lát 32x32, đọc theo cột  → xung đột {} lối",
        a.conflict_degree
    );
    println!(
        "   Lát 32x33 (đệm 1 cột)    → xung đột {} lối",
        b.conflict_degree
    );
    println!(
        "   → Thêm 1/32 bộ nhớ, nhanh gấp {} lần. Thủ thuật rẻ nhất trong GPU.",
        a.conflict_degree / b.conflict_degree.max(1)
    );

    println!("\n5. RÚT GỌN SONG SONG");
    println!(
        "   {:>10} {:>14} {:>16} {:>16}",
        "phần tử", "bước (cây)", "bước (tuần tự)", "phép cộng"
    );
    for n in [16usize, 1024, 1_048_576] {
        let d: Vec<i64> = (1..=n as i64).collect();
        let r = parallel_reduce(&d);
        println!(
            "   {:>10} {:>14} {:>16} {:>16}",
            n, r.num_steps, n, r.add_op_count
        );
    }
    let d: Vec<i64> = (1..=1000).collect();
    println!(
        "   Cùng kết quả với cách tuần tự: {}",
        parallel_reduce(&d).sum == sequential_reduce(&d)
    );
    println!("   → Cùng số phép cộng, nhưng 20 bước thay vì một triệu bước.");

    println!("\n6. NHÂN MA TRẬN THEO LÁT (n = 1024)");
    let n = 1024;
    let naive = gemm_naive(n);
    println!(
        "   {:<18} {:>18} {:>24}",
        "cách làm", "đọc toàn cục", "cường độ tính toán"
    );
    println!(
        "   {:<18} {:>18} {:>21.2} FLOP/B",
        "ngây thơ", naive.read_global, naive.arithmetic_intensity
    );
    for tile in [8usize, 16, 32] {
        let g = tiled_gemm(n, tile);
        println!(
            "   {:<18} {:>18} {:>21.2} FLOP/B",
            format!("lát {}x{}", tile, tile),
            g.read_global,
            g.arithmetic_intensity
        );
    }
    println!(
        "   → Cùng {} phép nhân. Lát 32 đọc ít hơn 32 lần từ bộ nhớ toàn cục.",
        naive.mul_op_count
    );

    println!("\n7. MỨC CHIẾM DỤNG");
    println!(
        "   {:>8} {:>10} {:>14} {:>12} {:>18}",
        "luồng", "thanh ghi", "chia sẻ (B)", "chiếm dụng", "bị chặn bởi"
    );
    for (l, regs, shared) in [
        (256usize, 32usize, 0usize),
        (256, 64, 0),
        (256, 128, 0),
        (256, 32, 16_384),
        (1024, 32, 0),
    ] {
        let m = occupancy(l, regs, shared);
        println!(
            "   {:>8} {:>10} {:>14} {:>11.0}% {:>18}",
            l,
            regs,
            shared,
            m.ratio * 100.0,
            m.blocked_by
        );
    }
    println!("   → Dùng nhiều thanh ghi cho mỗi luồng thì ít warp cùng chạy,");
    println!("     và GPU không còn đủ việc để che độ trễ bộ nhớ.");

    println!("\n═══════════════════════════════════════════════════════════");
    println!("   GPU KHÔNG NHANH HƠN — NÓ RỘNG HƠN. PHẢI CHO NÓ ĐỦ VIỆC.  ");
    println!("═══════════════════════════════════════════════════════════");
}

#[cfg(test)]
mod tests {
    use super::*;

    // ---------- Cấu hình phát ----------
    #[test]
    fn the_launch_config_covers_every_element() {
        for (n, l) in [
            (1_000_000usize, 256usize),
            (1_000, 256),
            (1, 256),
            (257, 256),
        ] {
            let c = LaunchConfig::for_n_items(n, l);
            assert!(
                c.total_threads() >= n,
                "phải đủ luồng phủ hết {} phần tử",
                n
            );
            assert!(
                c.total_threads() < n + l,
                "nhưng không được thừa quá một khối"
            );
        }
    }

    #[test]
    fn zero_elements_needs_no_blocks() {
        let c = LaunchConfig::for_n_items(0, 256);
        assert_eq!(c.num_blocks, 0);
        assert_eq!(c.total_threads(), 0);
    }

    #[test]
    fn a_non_warp_multiple_block_wastes_lanes() {
        let even = LaunchConfig {
            num_blocks: 1,
            threads_per_block: 256,
        };
        assert_eq!(even.warps_per_block(), 8);
        assert_eq!(even.wasted_per_block(), 0);
        let ragged = LaunchConfig {
            num_blocks: 1,
            threads_per_block: 100,
        };
        assert_eq!(ragged.warps_per_block(), 4, "100 luồng vẫn tốn 4 warp");
        assert_eq!(ragged.wasted_per_block(), 28, "28 làn ngồi chơi");
    }

    #[test]
    fn any_multiple_of_32_wastes_nothing() {
        for l in [32usize, 64, 128, 256, 512, 1024] {
            let c = LaunchConfig {
                num_blocks: 1,
                threads_per_block: l,
            };
            assert_eq!(c.wasted_per_block(), 0, "khối {} luồng", l);
        }
    }

    // ---------- Phân kỳ warp ----------
    #[test]
    fn a_uniform_warp_does_not_diverge() {
        let all_true = vec![true; 256];
        let p = divergence_analysis(&all_true);
        assert_eq!(p.divergent_warps, 0);
        assert!((p.slowdown - 1.0).abs() < 1e-9);
        let all_false = vec![false; 256];
        assert_eq!(divergence_analysis(&all_false).divergent_warps, 0);
    }

    #[test]
    fn branching_on_parity_diverges_every_warp() {
        let p = divergence_analysis(&branch_on_parity(1024));
        assert_eq!(p.num_warps, 32);
        assert_eq!(
            p.divergent_warps, 32,
            "warp nào cũng có cả luồng chẵn lẫn lẻ"
        );
        assert!((p.slowdown - 2.0).abs() < 1e-9, "chậm gấp đôi");
    }

    #[test]
    fn branching_per_warp_never_diverges() {
        // Bài học trung tâm: cùng tỉ lệ 50/50, chỉ khác CÁCH NHÓM.
        let conds = branch_on_warp(1024);
        let p = divergence_analysis(&conds);
        assert_eq!(p.divergent_warps, 0);
        assert!((p.slowdown - 1.0).abs() < 1e-9);
        let taken = conds.iter().filter(|&&x| x).count();
        assert_eq!(taken, 512, "vẫn đúng một nửa số luồng đi nhánh đúng");
    }

    #[test]
    fn one_odd_lane_diverges_the_whole_warp() {
        // Đây là điều khiến phân kỳ nguy hiểm: một luồng đủ để phạt cả 32.
        let mut conds = vec![true; 32];
        conds[17] = false;
        let p = divergence_analysis(&conds);
        assert_eq!(p.divergent_warps, 1);
        assert!(
            (p.slowdown - 2.0).abs() < 1e-9,
            "một luồng lạc điệu → cả warp chậm gấp đôi"
        );
    }

    #[test]
    fn an_empty_list_does_not_panic() {
        let p = divergence_analysis(&[]);
        assert_eq!(p.num_warps, 0);
        assert_eq!(p.slowdown, 1.0);
    }

    // ---------- Gộp truy cập ----------
    #[test]
    fn contiguous_access_coalesces_into_the_fewest_transactions() {
        let p = coalescing_analysis(THREADS_PER_WARP, 4, 1);
        assert_eq!(
            p.num_transactions, 1,
            "32 luồng x 4 byte = 128 byte = đúng 1 giao dịch"
        );
        assert!(
            (p.efficiency - 1.0).abs() < 1e-9,
            "hiệu suất băng thông hoàn hảo"
        );
    }

    #[test]
    fn a_larger_stride_costs_more_transactions() {
        let mut prev = 0;
        for b in [1usize, 2, 4, 8, 16, 32] {
            let p = coalescing_analysis(THREADS_PER_WARP, 4, b);
            assert!(
                p.num_transactions >= prev,
                "bước {} phải tốn ít nhất bằng bước trước",
                b
            );
            prev = p.num_transactions;
        }
        assert_eq!(
            coalescing_analysis(THREADS_PER_WARP, 4, 32).num_transactions,
            32,
            "bước nhảy 32 → mỗi luồng một giao dịch riêng"
        );
    }

    #[test]
    fn useful_bytes_stay_constant_as_stride_changes() {
        // Cùng lượng dữ liệu CẦN, khác hẳn lượng dữ liệu PHẢI CHUYỂN.
        for b in [1usize, 4, 32] {
            let p = coalescing_analysis(THREADS_PER_WARP, 4, b);
            assert_eq!(p.useful_bytes, 128, "luôn cần đúng 128 byte");
        }
        assert!(
            coalescing_analysis(32, 4, 32).bytes_transferred
                > coalescing_analysis(32, 4, 1).bytes_transferred * 30
        );
    }

    #[test]
    fn efficiency_stays_within_zero_and_one() {
        for b in [1usize, 2, 3, 7, 16, 64, 128] {
            let p = coalescing_analysis(THREADS_PER_WARP, 4, b);
            assert!(
                (0.0..=1.0).contains(&p.efficiency),
                "bước {} cho hiệu suất {}",
                b,
                p.efficiency
            );
        }
    }

    // ---------- Xung đột ngân hàng ----------
    #[test]
    fn contiguous_access_has_no_bank_conflicts() {
        let indices: Vec<usize> = (0..THREADS_PER_WARP).collect();
        let p = bank_analysis(&indices);
        assert_eq!(p.conflict_degree, 1);
        assert!(!p.has_conflict, "32 luồng vào 32 ngân hàng khác nhau");
    }

    #[test]
    fn reading_a_column_of_a_32_wide_tile_conflicts_fully() {
        let p = bank_analysis(&tile_column_access(32));
        assert_eq!(
            p.conflict_degree, 32,
            "mọi luồng rơi vào CÙNG một ngân hàng"
        );
        assert!(p.has_conflict);
    }

    #[test]
    fn padding_by_one_column_removes_all_conflicts() {
        // Thủ thuật rẻ nhất trong lập trình GPU: tốn thêm 1/32 bộ nhớ,
        // đổi lấy tốc độ gấp 32 lần.
        let p = bank_analysis(&padded_tile_column_access(32));
        assert_eq!(p.conflict_degree, 1);
        assert!(!p.has_conflict);
    }

    #[test]
    fn conflicts_depend_on_the_gcd_with_the_bank_count() {
        // Bề rộng nguyên tố cùng nhau với 32 thì không xung đột.
        for width in [1usize, 3, 33, 65] {
            let p = bank_analysis(&tile_column_access(width));
            assert!(!p.has_conflict, "bề rộng {} không nên xung đột", width);
        }
        // Bề rộng chẵn có ước chung với 32 thì xung đột
        for width in [2usize, 4, 8, 16, 32] {
            assert!(
                bank_analysis(&tile_column_access(width)).has_conflict,
                "bề rộng {} phải xung đột",
                width
            );
        }
    }

    // ---------- Rút gọn ----------
    #[test]
    fn parallel_reduction_matches_sequential() {
        // Bất biến sống còn: song song hoá không được đổi kết quả.
        for n in [0usize, 1, 2, 3, 7, 16, 17, 1000, 4096] {
            let d: Vec<i64> = (1..=n as i64).collect();
            assert_eq!(parallel_reduce(&d).sum, sequential_reduce(&d), "n={}", n);
        }
    }

    #[test]
    fn the_step_count_is_logarithmic() {
        for n in [2usize, 4, 16, 1024, 1_048_576] {
            let d: Vec<i64> = vec![1; n];
            let r = parallel_reduce(&d);
            assert_eq!(r.num_steps, reduce_steps(n), "n={}", n);
            assert!(r.num_steps < 25, "một triệu phần tử chỉ tốn 20 bước");
        }
    }

    #[test]
    fn the_addition_count_is_still_n_minus_one() {
        // Song song hoá KHÔNG làm ít việc hơn — nó chỉ làm việc song song.
        for n in [2usize, 8, 100, 1024] {
            let d: Vec<i64> = vec![1; n];
            assert_eq!(parallel_reduce(&d).add_op_count, n - 1, "n={}", n);
        }
    }

    #[test]
    fn reduce_handles_empty_and_single_element() {
        assert_eq!(parallel_reduce(&[]).sum, 0);
        assert_eq!(parallel_reduce(&[]).num_steps, 0);
        assert_eq!(parallel_reduce(&[42]).sum, 42);
        assert_eq!(parallel_reduce(&[42]).num_steps, 0);
    }

    #[test]
    fn reduction_is_correct_for_odd_counts() {
        // Số lẻ phần tử là chỗ dễ sai nhất: phần tử cuối không có cặp.
        let d = vec![1i64, 2, 3, 4, 5, 6, 7];
        assert_eq!(parallel_reduce(&d).sum, 28);
    }

    // ---------- GEMM theo lát ----------
    #[test]
    fn tiling_cuts_global_reads() {
        let n = 1024;
        let naive = gemm_naive(n);
        let mut prev = naive.read_global;
        for tile in [8usize, 16, 32] {
            let g = tiled_gemm(n, tile);
            assert!(g.read_global < prev, "lát {} phải đọc ít hơn", tile);
            prev = g.read_global;
        }
        assert_eq!(tiled_gemm(n, 32).read_global, naive.read_global / 32);
    }

    #[test]
    fn tiling_does_not_change_the_multiply_count() {
        // Tối ưu không được đổi khối lượng TÍNH TOÁN, chỉ đổi cách chạm bộ nhớ.
        let n = 512;
        let naive = gemm_naive(n);
        for tile in [1usize, 8, 16, 32] {
            assert_eq!(tiled_gemm(n, tile).mul_op_count, naive.mul_op_count);
        }
    }

    #[test]
    fn arithmetic_intensity_grows_with_tile_size() {
        let n = 1024;
        let mut prev = gemm_naive(n).arithmetic_intensity;
        for tile in [8usize, 16, 32] {
            let c = tiled_gemm(n, tile).arithmetic_intensity;
            assert!(c > prev, "lát {} phải cho cường độ cao hơn", tile);
            prev = c;
        }
    }

    #[test]
    fn a_tile_of_one_is_no_better_than_naive() {
        let n = 256;
        assert_eq!(tiled_gemm(n, 1).read_global, gemm_naive(n).read_global);
        assert_eq!(
            tiled_gemm(n, 0).read_global,
            gemm_naive(n).read_global,
            "lát 0 phải được chặn thành 1, không chia cho 0"
        );
    }

    // ---------- Mức chiếm dụng ----------
    #[test]
    fn few_registers_gives_high_occupancy() {
        let m = occupancy(256, 32, 0);
        assert!(
            m.ratio > 0.9,
            "32 thanh ghi/luồng phải cho chiếm dụng cao, thực tế {:.2}",
            m.ratio
        );
    }

    #[test]
    fn many_registers_drops_occupancy() {
        let few = occupancy(256, 32, 0);
        let many = occupancy(256, 128, 0);
        assert!(
            many.ratio < few.ratio,
            "dùng 128 thanh ghi phải giảm chiếm dụng: {:.2} so với {:.2}",
            many.ratio,
            few.ratio
        );
        assert_eq!(many.blocked_by, "thanh ghi");
    }

    #[test]
    fn shared_memory_can_also_become_the_bottleneck() {
        let m = occupancy(256, 32, 32_768); // nửa dung lượng chia sẻ mỗi khối
        assert_eq!(m.blocked_by, "bộ nhớ chia sẻ");
        assert!(m.ratio < 0.5);
    }

    #[test]
    fn occupancy_stays_in_a_valid_range() {
        for l in [32usize, 128, 256, 512, 1024] {
            for regs in [16usize, 32, 64, 128, 255] {
                let m = occupancy(l, regs, 0);
                assert!(
                    (0.0..=1.0).contains(&m.ratio),
                    "luồng {} thanh ghi {} cho tỉ lệ {}",
                    l,
                    regs,
                    m.ratio
                );
                assert!(m.concurrent_warps <= m.max_warps);
            }
        }
    }

    #[test]
    fn with_no_register_or_shared_pressure_the_warp_cap_binds() {
        let m = occupancy(1024, 0, 0);
        assert_eq!(m.ratio, 1.0, "không có ràng buộc nào thì chiếm dụng tối đa");
    }
}
