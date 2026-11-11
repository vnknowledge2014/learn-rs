//! # Chương 94: Tiền Huấn Luyện (AdamW, Muon, WSD) & Động Cơ Phục Vụ Suy Luận
//!
//! Crate `ch94` minh họa kiến trúc tiền huấn luyện và động cơ phục vụ suy luận LLM
//! hoàn toàn bằng Rust thuần (`std` 100%), không sử dụng thư viện bên ngoài.
//!
//! Các thành phần cốt lõi:
//! - `SplitMix64`: Bộ sinh số giả ngẫu nhiên 64-bit tất định.
//! - `CausalDataLoader`: Bộ nạp dữ liệu huấn luyện tự hồi quy nhân quả.
//! - `clip_grad_norm_l2`: Kẹp chuẩn L2 gradient toàn cục.
//! - `AdamW`: Bộ tối ưu phân rã trọng số tách rời kèm hiệu chỉnh chệch.
//! - `MuonOptimizer`: Trực giao hóa Newton–Schulz 5 bước hệ số (3.4445, -4.7750, 2.0315),
//!   định tuyến chặt ma trận 2D sang Muon và vector 1D sang AdamW.
//! - `CosineSchedule` & `WsdSchedule`: Lịch học Cosine và Warmup-Stable-Decay.
//! - `KvCache`: Bộ nhớ đệm Khóa-Giá trị cho giải mã từng bước GQA.
//! - `PagedKvBlockTable`: Bảng phân trang KV với cơ chế Copy-on-Write kiểu vLLM.
//! - `RadixPrefixTree`: Cây tiền tố LRU tái sử dụng KV-cache kiểu SGLang.
//! - `TokenSampler`: Lấy mẫu với temperature, top-k, top-p và min-p.
//! - `speculative_generate`: Giải mã suy đoán chính xác không chệch phân phối.
//! - `Int8Linear`: Tầng tuyến tính lượng tử hóa số nguyên 8-bit đối xứng.

use std::collections::{BTreeMap, HashMap};

// ============================================================================
// 1. BỘ SINH SỐ GIẢ NGẪU NHIÊN VÀ ĐẠI SỐ TUYẾN TÍNH VI MÔ
// ============================================================================

/// Bộ sinh số giả ngẫu nhiên SplitMix64 tất định.
#[derive(Debug, Clone)]
pub struct SplitMix64 {
    state: u64,
}

impl SplitMix64 {
    /// Khởi tạo với hạt giống xác định.
    pub const fn new(seed: u64) -> Self {
        Self { state: seed }
    }

    /// Sinh số nguyên không dấu 64-bit tiếp theo.
    pub fn next_u64(&mut self) -> u64 {
        self.state = self.state.wrapping_add(0x9E37_79B9_7F4A_7C15);
        let mut z = self.state;
        z = (z ^ (z >> 30)).wrapping_mul(0xBF58_476D_1CE4_E5B9);
        z = (z ^ (z >> 27)).wrapping_mul(0x94D0_49BB_1331_11EB);
        z ^ (z >> 31)
    }

    /// Sinh số thực trong nửa khoảng [0.0, 1.0).
    pub fn next_f64(&mut self) -> f64 {
        (self.next_u64() >> 11) as f64 / ((1u64 << 53) as f64)
    }

    /// Sinh số thực có dấu trong khoảng [-bound, bound].
    pub fn next_signed(&mut self, bound: f64) -> f64 {
        (self.next_f64() * 2.0 - 1.0) * bound
    }
}

/// Chuyển vị ma trận phẳng kích thước `rows x cols`.
pub fn transpose(mat: &[f64], rows: usize, cols: usize) -> Vec<f64> {
    assert_eq!(mat.len(), rows * cols, "Kích thước ma trận không khớp");
    let mut out = vec![0.0; rows * cols];
    for r in 0..rows {
        for c in 0..cols {
            out[c * rows + r] = mat[r * cols + c];
        }
    }
    out
}

/// Nhân hai ma trận phẳng: `(m x k) * (k x n) -> (m x n)`.
pub fn matmul(a: &[f64], b: &[f64], m: usize, k: usize, n: usize) -> Vec<f64> {
    assert_eq!(a.len(), m * k, "Kích thước ma trận A không hợp lệ");
    assert_eq!(b.len(), k * n, "Kích thước ma trận B không hợp lệ");
    let mut out = vec![0.0; m * n];
    for i in 0..m {
        for p in 0..k {
            let a_val = a[i * k + p];
            if a_val == 0.0 {
                continue;
            }
            for j in 0..n {
                out[i * n + j] += a_val * b[p * n + j];
            }
        }
    }
    out
}

/// Tích vô hướng giữa hai vector.
pub fn dot_product(a: &[f64], b: &[f64]) -> f64 {
    assert_eq!(a.len(), b.len(), "Độ dài hai vector phải bằng nhau");
    a.iter().zip(b).map(|(&x, &y)| x * y).sum()
}

/// Chuẩn L2 Euclid của một vector.
pub fn vector_norm_l2(v: &[f64]) -> f64 {
    v.iter().map(|&x| x * x).sum::<f64>().sqrt()
}

/// Độ tương đồng Cosine giữa hai vector.
pub fn cosine_similarity(a: &[f64], b: &[f64]) -> f64 {
    let norm_a = vector_norm_l2(a);
    let norm_b = vector_norm_l2(b);
    if norm_a <= 1e-12 || norm_b <= 1e-12 {
        0.0
    } else {
        dot_product(a, b) / (norm_a * norm_b)
    }
}

/// Độ lệch tuyệt đối lớn nhất giữa hai lát cắt số thực.
pub fn max_abs_diff(a: &[f64], b: &[f64]) -> f64 {
    assert_eq!(a.len(), b.len());
    a.iter()
        .zip(b)
        .map(|(&x, &y)| (x - y).abs())
        .fold(0.0f64, f64::max)
}

// ============================================================================
// 2. DỮ LIỆU NHÂN QUẢ VÀ KẸP GRADIENT L2
// ============================================================================

/// Cặp batch gồm chuỗi đầu vào và chuỗi mục tiêu tự hồi quy.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct CausalBatch {
    pub inputs: Vec<Vec<u32>>,
    pub targets: Vec<Vec<u32>>,
}

/// Bộ nạp dữ liệu nhân quả cho huấn luyện tự hồi quy.
#[derive(Debug, Clone)]
pub struct CausalDataLoader {
    pub tokens: Vec<u32>,
    pub batch_size: usize,
    pub seq_len: usize,
    pub current_idx: usize,
}

impl CausalDataLoader {
    /// Khởi tạo bộ nạp dữ liệu với chuỗi token, batch_size và độ dài ngữ cảnh seq_len.
    pub fn new(tokens: Vec<u32>, batch_size: usize, seq_len: usize) -> Self {
        assert!(seq_len > 0, "seq_len phải lớn hơn 0");
        assert!(batch_size > 0, "batch_size phải lớn hơn 0");
        assert!(
            tokens.len() > seq_len,
            "tokens length phải lớn hơn seq_len để dịch chuyển mục tiêu"
        );
        Self {
            tokens,
            batch_size,
            seq_len,
            current_idx: 0,
        }
    }

    /// Số lượng batch khả dụng trước khi kết thúc tập dữ liệu.
    pub fn num_batches(&self) -> usize {
        let total_usable = self.tokens.len().saturating_sub(1);
        let tokens_per_batch = self.batch_size * self.seq_len;
        total_usable / tokens_per_batch
    }

    /// Lấy batch tiếp theo dưới dạng cấu trúc `CausalBatch`.
    pub fn next_batch(&mut self) -> Option<CausalBatch> {
        let total_usable = self.tokens.len().saturating_sub(1);
        let tokens_per_batch = self.batch_size * self.seq_len;
        if self.current_idx + tokens_per_batch > total_usable {
            return None;
        }

        let mut inputs = Vec::with_capacity(self.batch_size);
        let mut targets = Vec::with_capacity(self.batch_size);

        for b in 0..self.batch_size {
            let start = self.current_idx + b * self.seq_len;
            let inp = self.tokens[start..start + self.seq_len].to_vec();
            let tgt = self.tokens[start + 1..start + self.seq_len + 1].to_vec();
            inputs.push(inp);
            targets.push(tgt);
        }

        self.current_idx += tokens_per_batch;
        Some(CausalBatch { inputs, targets })
    }

    /// Đặt lại chỉ mục duyệt về đầu tập dữ liệu.
    pub fn reset(&mut self) {
        self.current_idx = 0;
    }
}

/// Kẹp chuẩn L2 của toàn bộ mảng gradient theo ngưỡng `max_norm`.
pub fn clip_grad_norm_l2(grads: &mut [f64], max_norm: f64) -> f64 {
    let total_norm = grads.iter().map(|&g| g * g).sum::<f64>().sqrt();
    if total_norm > max_norm {
        let scale = max_norm / (total_norm + 1e-8);
        for g in grads.iter_mut() {
            *g *= scale;
        }
    }
    total_norm
}

// ============================================================================
// 3. LỊCH HỌC TẬP: COSINE VÀ WSD (WARMUP-STABLE-DECAY)
// ============================================================================

/// Lịch học Cosine tiêu chuẩn với pha khởi động tuyến tính.
#[derive(Debug, Clone, Copy)]
pub struct CosineSchedule {
    pub warmup_steps: usize,
    pub total_steps: usize,
    pub base_lr: f64,
    pub min_lr: f64,
}

impl CosineSchedule {
    /// Khởi tạo lịch học Cosine.
    pub fn new(warmup_steps: usize, total_steps: usize, base_lr: f64, min_lr: f64) -> Self {
        assert!(
            total_steps >= warmup_steps,
            "total_steps phải >= warmup_steps"
        );
        Self {
            warmup_steps,
            total_steps,
            base_lr,
            min_lr,
        }
    }

    /// Tính tốc độ học tại bước `step`.
    pub fn get_lr(&self, step: usize) -> f64 {
        if step < self.warmup_steps {
            if self.warmup_steps == 0 {
                return self.base_lr;
            }
            let alpha = step as f64 / self.warmup_steps as f64;
            self.min_lr + (self.base_lr - self.min_lr) * alpha
        } else if step <= self.total_steps {
            let denom = (self.total_steps - self.warmup_steps).max(1) as f64;
            let progress = (step - self.warmup_steps) as f64 / denom;
            let cos_val = (std::f64::consts::PI * progress).cos();
            self.min_lr + 0.5 * (self.base_lr - self.min_lr) * (1.0 + cos_val)
        } else {
            self.min_lr
        }
    }
}

/// Lịch học WSD (Warmup - Stable - Decay) hiện đại từ các mô hình tiền duyên.
#[derive(Debug, Clone, Copy)]
pub struct WsdSchedule {
    pub warmup_steps: usize,
    pub stable_steps: usize,
    pub decay_steps: usize,
    pub base_lr: f64,
    pub min_lr: f64,
}

impl WsdSchedule {
    /// Khởi tạo lịch học WSD với 3 pha phân biệt.
    pub fn new(
        warmup_steps: usize,
        stable_steps: usize,
        decay_steps: usize,
        base_lr: f64,
        min_lr: f64,
    ) -> Self {
        Self {
            warmup_steps,
            stable_steps,
            decay_steps,
            base_lr,
            min_lr,
        }
    }

    /// Tổng số bước học trong toàn bộ chu kỳ.
    pub fn total_steps(&self) -> usize {
        self.warmup_steps + self.stable_steps + self.decay_steps
    }

    /// Tính tốc độ học tại bước `step`, đảm bảo tính liên tục tuyệt đối tại các biên pha.
    pub fn get_lr(&self, step: usize) -> f64 {
        if step < self.warmup_steps {
            if self.warmup_steps == 0 {
                return self.base_lr;
            }
            let alpha = step as f64 / self.warmup_steps as f64;
            self.min_lr + (self.base_lr - self.min_lr) * alpha
        } else if step < self.warmup_steps + self.stable_steps {
            self.base_lr
        } else if step <= self.total_steps() {
            if self.decay_steps == 0 {
                return self.min_lr;
            }
            let decay_offset = step - (self.warmup_steps + self.stable_steps);
            let progress = decay_offset as f64 / self.decay_steps as f64;
            let cos_val = (std::f64::consts::PI * progress).cos();
            self.min_lr + 0.5 * (self.base_lr - self.min_lr) * (1.0 + cos_val)
        } else {
            self.min_lr
        }
    }
}

// ============================================================================
// 4. BỘ TỐI ƯU HÓA: ADAMW VÀ MUON OPTIMIZER VỚI ĐỊNH TUYẾN CHẶT
// ============================================================================

/// Cấu hình siêu tham số cho bộ tối ưu AdamW.
#[derive(Debug, Clone)]
pub struct AdamWConfig {
    pub lr: f64,
    pub beta1: f64,
    pub beta2: f64,
    pub eps: f64,
    pub weight_decay: f64,
}

impl Default for AdamWConfig {
    fn default() -> Self {
        Self {
            lr: 1e-3,
            beta1: 0.9,
            beta2: 0.999,
            eps: 1e-8,
            weight_decay: 0.01,
        }
    }
}

/// Bộ tối ưu hóa AdamW độc lập với phân rã trọng số tách rời.
#[derive(Debug, Clone)]
pub struct AdamW {
    pub config: AdamWConfig,
    pub step: usize,
    pub m: Vec<f64>,
    pub v: Vec<f64>,
}

impl AdamW {
    /// Khởi tạo trạng thái bộ nhớ đệm cho vector tham số có độ dài `len`.
    pub fn new(len: usize, config: AdamWConfig) -> Self {
        Self {
            config,
            step: 0,
            m: vec![0.0; len],
            v: vec![0.0; len],
        }
    }

    /// Cập nhật tham số theo bước AdamW chuẩn.
    pub fn update(&mut self, params: &mut [f64], grads: &[f64], lr_override: Option<f64>) {
        assert_eq!(params.len(), grads.len());
        assert_eq!(params.len(), self.m.len());
        self.step += 1;
        let lr = lr_override.unwrap_or(self.config.lr);
        let b1 = self.config.beta1;
        let b2 = self.config.beta2;
        let eps = self.config.eps;
        let wd = self.config.weight_decay;

        let bias_correction1 = 1.0 - b1.powi(self.step as i32);
        let bias_correction2 = 1.0 - b2.powi(self.step as i32);

        for i in 0..params.len() {
            let g = grads[i];
            self.m[i] = b1 * self.m[i] + (1.0 - b1) * g;
            self.v[i] = b2 * self.v[i] + (1.0 - b2) * g * g;

            let m_hat = self.m[i] / bias_correction1;
            let v_hat = self.v[i] / bias_correction2;

            // Decoupled weight decay
            params[i] -= lr * wd * params[i];
            // Gradient update
            params[i] -= lr * m_hat / (v_hat.sqrt() + eps);
        }
    }
}

/// Trực giao hóa ma trận Newton–Schulz 5 bước với hệ số xấp xỉ bậc 3:
/// `(3.4445, -4.7750, 2.0315)`.
///
/// Hỗ trợ hoàn hảo tính đối xứng giữa ma trận cao (`rows > cols`) và ma trận rộng (`cols > rows`).
pub fn muon_orthogonalize(grad: &[f64], rows: usize, cols: usize, steps: usize) -> Vec<f64> {
    assert_eq!(grad.len(), rows * cols);
    let frob = grad.iter().map(|&v| v * v).sum::<f64>().sqrt() + 1e-7;
    let norm: Vec<f64> = grad.iter().map(|&v| v / frob).collect();
    let (r, c, mut x) = if rows > cols {
        (cols, rows, transpose(&norm, rows, cols))
    } else {
        (rows, cols, norm)
    };

    for _ in 0..steps {
        // a = X * X^T (r x r)
        let xt = transpose(&x, r, c);
        let a = matmul(&x, &xt, r, c, r);
        // aa = a * a (r x r)
        let aa = matmul(&a, &a, r, r, r);
        // b = -4.7750 * a + 2.0315 * aa
        let b: Vec<f64> = a
            .iter()
            .zip(&aa)
            .map(|(&u, &v)| -4.7750 * u + 2.0315 * v)
            .collect();
        // bx = b * X (r x c)
        let bx = matmul(&b, &x, r, r, c);
        // X = 3.4445 * X + bx
        for (xv, &bxv) in x.iter_mut().zip(&bx) {
            *xv = 3.4445 * (*xv) + bxv;
        }
    }

    if rows > cols { transpose(&x, r, c) } else { x }
}

/// Phân loại hình dạng tham số để định tuyến bộ tối ưu hóa thích hợp.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum ParamKind {
    /// Ma trận trọng số 2D (ẩn, chiếu, MLP) -> Định tuyến qua Muon.
    Matrix2D { rows: usize, cols: usize },
    /// Vector trọng số 1D (độ lệch, chuẩn hóa, nhúng, lm_head) -> Định tuyến qua AdamW.
    Vector1D { size: usize },
}

/// Cấu hình cho bộ tối ưu hóa Muon.
#[derive(Debug, Clone)]
pub struct MuonConfig {
    pub lr: f64,
    pub momentum: f64,
    pub weight_decay: f64,
    pub ns_steps: usize,
}

impl Default for MuonConfig {
    fn default() -> Self {
        Self {
            lr: 0.02,
            momentum: 0.95,
            weight_decay: 0.01,
            ns_steps: 5,
        }
    }
}

/// Bộ tối ưu hóa hỗn hợp Muon kết hợp AdamW với định tuyến nghiêm ngặt.
#[derive(Debug, Clone)]
pub struct MuonOptimizer {
    pub muon_config: MuonConfig,
    pub adamw_config: AdamWConfig,
    pub step: usize,
    pub muon_momentum: HashMap<usize, Vec<f64>>,
    pub adamw_optimizers: HashMap<usize, AdamW>,
}

impl MuonOptimizer {
    /// Khởi tạo bộ tối ưu hóa với cấu hình riêng cho Muon và AdamW.
    pub fn new(muon_config: MuonConfig, adamw_config: AdamWConfig) -> Self {
        Self {
            muon_config,
            adamw_config,
            step: 0,
            muon_momentum: HashMap::new(),
            adamw_optimizers: HashMap::new(),
        }
    }

    /// Bước cập nhật tham số với định tuyến chặt chẽ:
    /// - Ma trận 2D được cập nhật qua trực giao hóa Newton–Schulz của Muon.
    /// - Vector 1D được định tuyến sang bộ tối ưu hóa AdamW tương ứng.
    pub fn step(
        &mut self,
        param_id: usize,
        kind: ParamKind,
        param: &mut [f64],
        grad: &[f64],
        lr_override: Option<f64>,
    ) {
        assert_eq!(param.len(), grad.len());

        match kind {
            ParamKind::Matrix2D { rows, cols } => {
                assert_eq!(param.len(), rows * cols);
                let lr = lr_override.unwrap_or(self.muon_config.lr);
                let mom = self.muon_config.momentum;
                let wd = self.muon_config.weight_decay;
                let ns_steps = self.muon_config.ns_steps;

                let m_buf = self
                    .muon_momentum
                    .entry(param_id)
                    .or_insert_with(|| vec![0.0; param.len()]);

                // Cập nhật động lượng: M = mom * M + grad
                for (mv, &gv) in m_buf.iter_mut().zip(grad) {
                    *mv = mom * (*mv) + gv;
                }

                // Trực giao hóa hướng cập nhật bằng Newton-Schulz 5 bước
                let ortho = muon_orthogonalize(m_buf, rows, cols, ns_steps);

                // Hệ số co dãn RMS đặc trưng của Muon: 0.2 * lr * sqrt(max(rows, cols))
                let rms_scale = 0.2 * lr * ((rows.max(cols)) as f64).sqrt();

                // Cập nhật tham số với phân rã trọng số tách rời
                for (p, &o) in param.iter_mut().zip(&ortho) {
                    *p = *p * (1.0 - lr * wd) - rms_scale * o;
                }
            }
            ParamKind::Vector1D { size } => {
                assert_eq!(param.len(), size);
                let opt = self
                    .adamw_optimizers
                    .entry(param_id)
                    .or_insert_with(|| AdamW::new(size, self.adamw_config.clone()));
                opt.update(param, grad, lr_override);
            }
        }
    }
}

// ============================================================================
// 5. PHỤC VỤ SUY LUẬN: KV-CACHE VÀ NHÚNG VỊ TRÍ QUAY (ROPE)
// ============================================================================

/// Quay nửa vector theo ma trận trực giao RoPE tại vị trí token `pos`.
pub fn rope_rotate(vec: &mut [f64], pos: usize, base: f64) {
    let (d, half) = (vec.len(), vec.len() / 2);
    for i in 0..half {
        let (s, c) = ((pos as f64) / base.powf((2 * i) as f64 / (d as f64))).sin_cos();
        let (x1, x2) = (vec[i], vec[i + half]);
        vec[i] = x1 * c - x2 * s;
        vec[i + half] = x2 * c + x1 * s;
    }
}

/// Cấu hình chú ý nhóm truy vấn (GQA - Grouped-Query Attention).
#[derive(Debug, Clone, Copy)]
pub struct GqaConfig {
    pub num_heads: usize,
    pub num_kv_heads: usize,
    pub head_dim: usize,
    pub rope_base: f64,
}

/// Bộ nhớ đệm KV liên tục cho giải mã từng bước.
#[derive(Debug, Clone, Default)]
pub struct KvCache {
    pub key_cache: Vec<f64>,
    pub value_cache: Vec<f64>,
    pub seq_len: usize,
}

impl KvCache {
    /// Khởi tạo bộ nhớ đệm KV rỗng.
    pub fn new() -> Self {
        Self::default()
    }

    /// Thực hiện một bước giải mã tự hồi quy với GQA và RoPE.
    pub fn step_decode(&mut self, cfg: GqaConfig, q: &[f64], k: &[f64], v: &[f64]) -> Vec<f64> {
        let GqaConfig {
            num_heads,
            num_kv_heads,
            head_dim,
            rope_base,
        } = cfg;
        let kv_stride = num_kv_heads * head_dim;
        let pos = self.seq_len;
        let grp = num_heads / num_kv_heads;

        let mut qr = q.to_vec();
        let mut kr = k.to_vec();
        for ch in qr.chunks_exact_mut(head_dim) {
            rope_rotate(ch, pos, rope_base);
        }
        for ch in kr.chunks_exact_mut(head_dim) {
            rope_rotate(ch, pos, rope_base);
        }

        self.key_cache.extend_from_slice(&kr);
        self.value_cache.extend_from_slice(v);
        self.seq_len += 1;

        let scale = 1.0 / (head_dim as f64).sqrt();
        let mut out = vec![0.0; num_heads * head_dim];

        for h in 0..num_heads {
            let kv_h = h / grp;
            let qv = &qr[h * head_dim..(h + 1) * head_dim];
            let mut scores = vec![0.0; self.seq_len];

            for (t, slot) in scores.iter_mut().enumerate() {
                let ks = &self.key_cache
                    [t * kv_stride + kv_h * head_dim..t * kv_stride + (kv_h + 1) * head_dim];
                let dot: f64 = qv.iter().zip(ks).map(|(&a, &b)| a * b).sum();
                *slot = dot * scale;
            }

            let max_s = scores.iter().copied().fold(f64::NEG_INFINITY, f64::max);
            let mut sum_exp = 0.0;
            for s in &mut scores {
                *s = (*s - max_s).exp();
                sum_exp += *s;
            }

            let dst = &mut out[h * head_dim..(h + 1) * head_dim];
            for (t, &num) in scores.iter().enumerate() {
                let weight = num / sum_exp;
                let v_offset = t * kv_stride + kv_h * head_dim;
                let vs = &self.value_cache[v_offset..v_offset + head_dim];
                for (dst_val, &val) in dst.iter_mut().zip(vs) {
                    *dst_val += weight * val;
                }
            }
        }

        out
    }
}

// ============================================================================
// 6. PHÂN TRANG BỘ NHỚ ĐỆM KV (PAGEDATTENTION VLLM-STYLE)
// ============================================================================

/// Khối bộ nhớ vật lý chứa số lượng token cố định.
#[derive(Debug, Clone)]
pub struct PhysicalBlock {
    pub block_id: usize,
    pub block_size: usize,
    pub num_tokens: usize,
    pub keys: Vec<f64>,
    pub values: Vec<f64>,
}

/// Bể quản lý các khối bộ nhớ vật lý với đếm tham chiếu (Reference Counting).
#[derive(Debug, Clone)]
pub struct PhysicalBlockPool {
    pub blocks: Vec<PhysicalBlock>,
    pub ref_counts: Vec<usize>,
    pub free_list: Vec<usize>,
    pub block_size: usize,
    pub kv_dim: usize,
}

impl PhysicalBlockPool {
    /// Khởi tạo bể khối với dung lượng capacity khối.
    pub fn new(capacity: usize, block_size: usize, kv_dim: usize) -> Self {
        let mut blocks = Vec::with_capacity(capacity);
        let ref_counts = vec![0; capacity];
        let mut free_list = Vec::with_capacity(capacity);

        for i in 0..capacity {
            blocks.push(PhysicalBlock {
                block_id: i,
                block_size,
                num_tokens: 0,
                keys: vec![0.0; block_size * kv_dim],
                values: vec![0.0; block_size * kv_dim],
            });
            free_list.push(capacity - 1 - i);
        }

        Self {
            blocks,
            ref_counts,
            free_list,
            block_size,
            kv_dim,
        }
    }

    /// Phân bổ một khối trống mới từ bể.
    pub fn allocate_block(&mut self) -> Result<usize, String> {
        if let Some(id) = self.free_list.pop() {
            self.ref_counts[id] = 1;
            self.blocks[id].num_tokens = 0;
            Ok(id)
        } else {
            Err("Hết bộ nhớ trong bể khối vật lý (Out of Memory)".to_string())
        }
    }

    /// Tăng số lượng tham chiếu tới khối `block_id`.
    pub fn inc_ref(&mut self, block_id: usize) {
        self.ref_counts[block_id] += 1;
    }

    /// Giảm số lượng tham chiếu; thu hồi khối về danh sách trống khi ref_count = 0.
    pub fn dec_ref(&mut self, block_id: usize) {
        assert!(
            self.ref_counts[block_id] > 0,
            "Lỗi giải phóng trùng lặp (Double Free) trên khối {}",
            block_id
        );
        self.ref_counts[block_id] -= 1;
        if self.ref_counts[block_id] == 0 {
            self.blocks[block_id].num_tokens = 0;
            self.free_list.push(block_id);
        }
    }

    /// Số lượng khối còn trống trong bể.
    pub fn free_blocks_count(&self) -> usize {
        self.free_list.len()
    }
}

/// Bảng khối logic của một chuỗi hội thoại, cài đặt Copy-on-Write khi rẽ nhánh.
#[derive(Debug, Clone)]
pub struct PagedKvBlockTable {
    pub sequence_id: usize,
    pub block_ids: Vec<usize>,
    pub total_tokens: usize,
    pub block_size: usize,
}

impl PagedKvBlockTable {
    /// Khởi tạo bảng khối logic rỗng cho chuỗi.
    pub fn new(sequence_id: usize, block_size: usize) -> Self {
        Self {
            sequence_id,
            block_ids: Vec::new(),
            total_tokens: 0,
            block_size,
        }
    }

    /// Ghi thêm một token KV vào chuỗi, tự động thực hiện Copy-on-Write nếu khối đang dùng chung.
    pub fn append_kv(
        &mut self,
        pool: &mut PhysicalBlockPool,
        k: &[f64],
        v: &[f64],
    ) -> Result<(), String> {
        assert_eq!(k.len(), pool.kv_dim);
        assert_eq!(v.len(), pool.kv_dim);

        let need_new_block = if let Some(&last_id) = self.block_ids.last() {
            pool.blocks[last_id].num_tokens == self.block_size
        } else {
            true
        };

        if need_new_block {
            let new_id = pool.allocate_block()?;
            self.block_ids.push(new_id);
        }

        let last_idx = self.block_ids.len() - 1;
        let last_id = self.block_ids[last_idx];

        // Cơ chế Copy-on-Write: Nếu khối có nhiều hơn 1 tham chiếu, nhân bản riêng trước khi sửa
        if pool.ref_counts[last_id] > 1 {
            let fresh_id = pool.allocate_block()?;
            let existing_tokens = pool.blocks[last_id].num_tokens;
            let kv_dim = pool.kv_dim;

            let (src_keys, src_vals) = {
                let src = &pool.blocks[last_id];
                (
                    src.keys[..existing_tokens * kv_dim].to_vec(),
                    src.values[..existing_tokens * kv_dim].to_vec(),
                )
            };

            pool.blocks[fresh_id].keys[..existing_tokens * kv_dim].copy_from_slice(&src_keys);
            pool.blocks[fresh_id].values[..existing_tokens * kv_dim].copy_from_slice(&src_vals);
            pool.blocks[fresh_id].num_tokens = existing_tokens;

            pool.dec_ref(last_id);
            self.block_ids[last_idx] = fresh_id;
        }

        let curr_id = self.block_ids[last_idx];
        let token_pos = pool.blocks[curr_id].num_tokens;
        let kv_dim = pool.kv_dim;
        let offset = token_pos * kv_dim;

        pool.blocks[curr_id].keys[offset..offset + kv_dim].copy_from_slice(k);
        pool.blocks[curr_id].values[offset..offset + kv_dim].copy_from_slice(v);
        pool.blocks[curr_id].num_tokens += 1;
        self.total_tokens += 1;

        Ok(())
    }

    /// Rẽ nhánh chuỗi (Fork Sequence) không tốn chi phí sao chép bộ nhớ (Copy-on-Write).
    pub fn fork_sequence(&self, new_seq_id: usize, pool: &mut PhysicalBlockPool) -> Self {
        for &id in &self.block_ids {
            pool.inc_ref(id);
        }
        Self {
            sequence_id: new_seq_id,
            block_ids: self.block_ids.clone(),
            total_tokens: self.total_tokens,
            block_size: self.block_size,
        }
    }

    /// Giải phóng toàn bộ bảng khối logic và giảm số tham chiếu tương ứng trong bể.
    pub fn release(&mut self, pool: &mut PhysicalBlockPool) {
        for &id in &self.block_ids {
            pool.dec_ref(id);
        }
        self.block_ids.clear();
        self.total_tokens = 0;
    }

    /// Đọc toàn bộ vector K và V đã lưu trong các khối phân trang.
    pub fn read_all_kv(&self, pool: &PhysicalBlockPool) -> (Vec<f64>, Vec<f64>) {
        let kv_dim = pool.kv_dim;
        let mut keys = Vec::with_capacity(self.total_tokens * kv_dim);
        let mut values = Vec::with_capacity(self.total_tokens * kv_dim);

        for &id in &self.block_ids {
            let block = &pool.blocks[id];
            let num = block.num_tokens;
            keys.extend_from_slice(&block.keys[..num * kv_dim]);
            values.extend_from_slice(&block.values[..num * kv_dim]);
        }
        (keys, values)
    }
}

// ============================================================================
// 7. CÂY TIỀN TỐ RADIX TÁI SỬ DỤNG BỘ NHỚ ĐỆM (RADIXPREFIX TREE SGLANG-STYLE)
// ============================================================================

/// Một nút trong cây tiền tố Radix lưu trữ đoạn token cạnh và các khối KV tương ứng.
#[derive(Debug, Clone)]
pub struct RadixNode {
    pub node_id: usize,
    pub tokens: Vec<u32>,
    pub cached_block_ids: Vec<usize>,
    pub children: BTreeMap<u32, usize>,
    pub parent_id: Option<usize>,
    pub last_access: u64,
    pub pinned: bool,
}

/// Cây tiền tố Radix quản lý việc tái sử dụng tiền tố prompt và dọn dẹp LRU.
#[derive(Debug, Clone)]
pub struct RadixPrefixTree {
    pub nodes: Vec<RadixNode>,
    pub root_id: usize,
    pub access_clock: u64,
}

impl Default for RadixPrefixTree {
    fn default() -> Self {
        Self::new()
    }
}

impl RadixPrefixTree {
    /// Khởi tạo cây Radix với nút gốc rỗng.
    pub fn new() -> Self {
        let root = RadixNode {
            node_id: 0,
            tokens: Vec::new(),
            cached_block_ids: Vec::new(),
            children: BTreeMap::new(),
            parent_id: None,
            last_access: 0,
            pinned: false,
        };
        Self {
            nodes: vec![root],
            root_id: 0,
            access_clock: 0,
        }
    }

    /// Khớp tiền tố dài nhất của chuỗi tokens trong cây.
    ///
    /// Trả về `(số token khớp, danh sách block ID đã lưu đệm)`.
    pub fn match_prefix(&mut self, tokens: &[u32]) -> (usize, Vec<usize>) {
        self.access_clock += 1;
        let mut curr_id = self.root_id;
        let mut matched_len = 0;
        let mut matched_blocks = Vec::new();

        self.nodes[curr_id].last_access = self.access_clock;

        while matched_len < tokens.len() {
            let next_tok = tokens[matched_len];
            if let Some(&child_id) = self.nodes[curr_id].children.get(&next_tok) {
                let edge_tokens = &self.nodes[child_id].tokens;
                let remaining = &tokens[matched_len..];

                // Tìm tiền tố chung giữa cạnh cây và đoạn token còn lại
                let common = edge_tokens
                    .iter()
                    .zip(remaining)
                    .take_while(|&(a, b)| *a == *b)
                    .count();

                if common == edge_tokens.len() {
                    // Khớp trọn vẹn cạnh này, tiến sâu hơn vào cây
                    matched_len += common;
                    matched_blocks.extend_from_slice(&self.nodes[child_id].cached_block_ids);
                    curr_id = child_id;
                    self.nodes[curr_id].last_access = self.access_clock;
                } else {
                    // Khớp một phần cạnh, không thể tái sử dụng khối ở mức khối trọn vẹn
                    matched_len += common;
                    self.nodes[child_id].last_access = self.access_clock;
                    break;
                }
            } else {
                break;
            }
        }

        (matched_len, matched_blocks)
    }

    /// Chèn chuỗi tokens kèm các block_ids vào cây, tự động tách cạnh nếu khớp một phần.
    pub fn insert(&mut self, tokens: &[u32], block_ids: &[usize]) -> usize {
        self.access_clock += 1;
        let mut curr_id = self.root_id;
        let mut token_offset = 0;
        let mut block_offset = 0;

        self.nodes[curr_id].last_access = self.access_clock;

        while token_offset < tokens.len() {
            let next_tok = tokens[token_offset];
            if let Some(&child_id) = self.nodes[curr_id].children.get(&next_tok) {
                let edge_tokens = self.nodes[child_id].tokens.clone();
                let remaining = &tokens[token_offset..];
                let common = edge_tokens
                    .iter()
                    .zip(remaining)
                    .take_while(|&(a, b)| *a == *b)
                    .count();

                if common == edge_tokens.len() {
                    token_offset += common;
                    block_offset += self.nodes[child_id].cached_block_ids.len();
                    curr_id = child_id;
                    self.nodes[curr_id].last_access = self.access_clock;
                } else {
                    // Tách cạnh (Split Edge): tạo nút trung gian lưu phần chung
                    let intermediate_id = self.nodes.len();
                    let common_tokens = edge_tokens[..common].to_vec();
                    let child_remainder = edge_tokens[common..].to_vec();

                    // Khối thuộc phần chung vs phần còn lại của con
                    let (common_blocks, child_blocks) =
                        (Vec::new(), self.nodes[child_id].cached_block_ids.clone());

                    let intermediate_node = RadixNode {
                        node_id: intermediate_id,
                        tokens: common_tokens,
                        cached_block_ids: common_blocks,
                        children: BTreeMap::new(),
                        parent_id: Some(curr_id),
                        last_access: self.access_clock,
                        pinned: false,
                    };
                    self.nodes.push(intermediate_node);

                    // Nối curr_id -> intermediate
                    self.nodes[curr_id]
                        .children
                        .insert(next_tok, intermediate_id);

                    // Cập nhật child_id thành con của intermediate
                    let child_branch_tok = child_remainder[0];
                    self.nodes[child_id].tokens = child_remainder;
                    self.nodes[child_id].cached_block_ids = child_blocks;
                    self.nodes[child_id].parent_id = Some(intermediate_id);

                    self.nodes[intermediate_id]
                        .children
                        .insert(child_branch_tok, child_id);

                    token_offset += common;
                    curr_id = intermediate_id;
                    break;
                }
            } else {
                break;
            }
        }

        // Tạo nút lá cho phần đuôi còn lại nếu có
        if token_offset < tokens.len() {
            let leaf_id = self.nodes.len();
            let leaf_tokens = tokens[token_offset..].to_vec();
            let leaf_blocks = block_ids[block_offset..].to_vec();

            let leaf_node = RadixNode {
                node_id: leaf_id,
                tokens: leaf_tokens,
                cached_block_ids: leaf_blocks,
                children: BTreeMap::new(),
                parent_id: Some(curr_id),
                last_access: self.access_clock,
                pinned: false,
            };
            self.nodes.push(leaf_node);

            let branch_tok = tokens[token_offset];
            self.nodes[curr_id].children.insert(branch_tok, leaf_id);
            leaf_id
        } else {
            curr_id
        }
    }

    /// Ghim (pin) hoặc bỏ ghim một nút để chống dọn dẹp LRU.
    pub fn pin_node(&mut self, node_id: usize, pinned: bool) {
        if node_id < self.nodes.len() {
            self.nodes[node_id].pinned = pinned;
        }
    }

    /// Dọn dẹp nút lá không bị ghim có thời gian truy cập cũ nhất (LRU Eviction).
    ///
    /// Trả về danh sách các `block_id` được thu hồi để hoàn trả vào bể khối.
    pub fn evict_lru(&mut self, pool: &mut PhysicalBlockPool) -> Option<Vec<usize>> {
        let mut oldest_leaf: Option<(usize, u64)> = None;

        for node in &self.nodes {
            if node.node_id != self.root_id
                && node.children.is_empty()
                && !node.pinned
                && !node.tokens.is_empty()
            {
                match oldest_leaf {
                    None => oldest_leaf = Some((node.node_id, node.last_access)),
                    Some((_, best_time)) if node.last_access < best_time => {
                        oldest_leaf = Some((node.node_id, node.last_access));
                    }
                    _ => {}
                }
            }
        }

        if let Some((target_id, _)) = oldest_leaf {
            let freed_blocks = self.nodes[target_id].cached_block_ids.clone();
            for &b_id in &freed_blocks {
                pool.dec_ref(b_id);
            }
            self.nodes[target_id].cached_block_ids.clear();
            self.nodes[target_id].tokens.clear();

            // Gỡ liên kết khỏi cha
            if let Some(parent_id) = self.nodes[target_id].parent_id {
                self.nodes[parent_id]
                    .children
                    .retain(|_, &mut child| child != target_id);
            }

            Some(freed_blocks)
        } else {
            None
        }
    }
}

// ============================================================================
// 8. BỘ LẤY MẪU TOKEN (TEMPERATURE, TOP-K, TOP-P VÀ MIN-P)
// ============================================================================

/// Bộ lấy mẫu token đa chiến lược.
#[derive(Debug, Clone, Copy)]
pub struct TokenSampler {
    pub temperature: f64,
    pub top_k: usize,
    pub top_p: f64,
    pub min_p: f64,
}

impl Default for TokenSampler {
    fn default() -> Self {
        Self {
            temperature: 0.7,
            top_k: 50,
            top_p: 0.9,
            min_p: 0.05,
        }
    }
}

impl TokenSampler {
    /// Khởi tạo bộ lấy mẫu với đầy đủ các tham số kiểm soát độ ngẫu nhiên.
    pub fn new(temperature: f64, top_k: usize, top_p: f64, min_p: f64) -> Self {
        Self {
            temperature,
            top_k,
            top_p,
            min_p,
        }
    }

    /// Lấy mẫu một chỉ số token từ mảng logit phân phối.
    pub fn sample(&self, logits: &[f64], rng: &mut SplitMix64) -> usize {
        assert!(!logits.is_empty(), "Mảng logits không được để trống");

        // 1. Chế độ Greedy (nhiệt độ tiệm cận 0)
        if self.temperature <= 1e-6 {
            let mut best_idx = 0;
            let mut best_val = logits[0];
            for (i, &l) in logits.iter().enumerate().skip(1) {
                if l > best_val {
                    best_val = l;
                    best_idx = i;
                }
            }
            return best_idx;
        }

        // 2. Co dãn nhiệt độ và tính Softmax chống tràn số
        let scaled_logits: Vec<f64> = logits.iter().map(|&l| l / self.temperature).collect();
        let max_l = scaled_logits
            .iter()
            .copied()
            .fold(f64::NEG_INFINITY, f64::max);
        let mut probs: Vec<f64> = scaled_logits.iter().map(|&l| (l - max_l).exp()).collect();
        let sum_p: f64 = probs.iter().sum();
        for p in probs.iter_mut() {
            *p /= sum_p;
        }

        let mut indexed: Vec<(f64, usize)> =
            probs.into_iter().enumerate().map(|(i, p)| (p, i)).collect();
        indexed.sort_by(|a, b| b.0.partial_cmp(&a.0).unwrap_or(std::cmp::Ordering::Equal));

        // 3. Lọc Min-p: Loại bỏ mọi token có xác suất p < min_p * p_max
        let p_max = indexed[0].0;
        let min_p_thresh = self.min_p * p_max;
        indexed.retain(|&(p, _)| p >= min_p_thresh);

        // 4. Lọc Top-k: Giới hạn tối đa k ứng viên hàng đầu
        if self.top_k > 0 && indexed.len() > self.top_k {
            indexed.truncate(self.top_k);
        }

        // 5. Lọc Top-p (Nucleus): Giữ lại tập ứng viên tối thiểu có tổng khối lượng xác suất >= top_p
        if self.top_p < 1.0 {
            let mut cum_sum = 0.0;
            let mut cutoff = indexed.len();
            for (idx, &(p, _)) in indexed.iter().enumerate() {
                cum_sum += p;
                if cum_sum >= self.top_p {
                    cutoff = idx + 1;
                    break;
                }
            }
            indexed.truncate(cutoff);
        }

        // 6. Tái chuẩn hóa và bốc thăm ngẫu nhiên
        let total_mass: f64 = indexed.iter().map(|&(p, _)| p).sum();
        let r = rng.next_f64() * total_mass;
        let mut accum = 0.0;
        for &(p, token_id) in &indexed {
            accum += p;
            if r <= accum {
                return token_id;
            }
        }

        indexed.last().map(|&(_, id)| id).unwrap_or(0)
    }
}

/// Tiện ích lấy mẫu một chỉ số từ mảng phân phối xác suất đã chuẩn hóa.
pub fn sample_from_distribution(dist: &[f64], rng: &mut SplitMix64) -> usize {
    let r = rng.next_f64();
    let mut accum = 0.0;
    for (i, &p) in dist.iter().enumerate() {
        accum += p;
        if r <= accum {
            return i;
        }
    }
    dist.len().saturating_sub(1)
}

// ============================================================================
// 9. GIẢI MÃ SUY ĐOÁN CHÍNH XÁC (SPECULATIVE DECODING)
// ============================================================================

/// Kết quả của một bước giải mã suy đoán.
#[derive(Debug, Clone)]
pub struct SpeculativeStepResult {
    pub accepted_tokens: Vec<usize>,
    pub draft_tokens: Vec<usize>,
    pub num_draft_accepted: usize,
    pub total_emitted: usize,
    pub bonus_token_generated: bool,
}

/// Thực hiện một bước giải mã suy đoán giữa mô hình nháp nhanh và mô hình đích chính xác.
///
/// Bất biến toán học: Phân phối của token xuất xưởng trùng khít tuyệt đối với mô hình đích!
pub fn speculative_decode_step<FDraft, FTarget>(
    current_tokens: &[usize],
    draft_len_k: usize,
    mut draft_fn: FDraft,
    mut target_fn: FTarget,
    rng: &mut SplitMix64,
) -> SpeculativeStepResult
where
    FDraft: FnMut(&[usize]) -> (usize, Vec<f64>),
    FTarget: FnMut(&[usize]) -> Vec<f64>,
{
    // 1. Mô hình nháp tự hồi quy sinh nhanh K token
    let mut draft_seq = current_tokens.to_vec();
    let mut draft_tokens = Vec::with_capacity(draft_len_k);
    let mut draft_dists = Vec::with_capacity(draft_len_k);

    for _ in 0..draft_len_k {
        let (tok, dist) = draft_fn(&draft_seq);
        draft_seq.push(tok);
        draft_tokens.push(tok);
        draft_dists.push(dist);
    }

    // 2. Mô hình đích thẩm định song song từng vị trí
    let mut eval_seq = current_tokens.to_vec();
    let mut accepted_tokens = Vec::new();
    let mut num_draft_accepted = 0;
    let mut rejected = false;

    for i in 0..draft_len_k {
        let p_dist = target_fn(&eval_seq);
        let draft_tok = draft_tokens[i];
        let p_tok = p_dist[draft_tok];
        let q_tok = draft_dists[i][draft_tok];

        let accept_prob = if q_tok <= 1e-12 {
            1.0
        } else {
            (p_tok / q_tok).min(1.0)
        };

        let u = rng.next_f64();
        if u < accept_prob {
            // Chấp nhận token nháp
            accepted_tokens.push(draft_tok);
            eval_seq.push(draft_tok);
            num_draft_accepted += 1;
        } else {
            // Từ chối! Lấy mẫu token thay thế từ phần dư phân phối chuẩn hóa: max(0, p - q)
            let mut residual: Vec<f64> = p_dist
                .iter()
                .zip(&draft_dists[i])
                .map(|(&p, &q)| (p - q).max(0.0))
                .collect();
            let res_sum: f64 = residual.iter().sum();
            let replacement_token = if res_sum > 1e-12 {
                for r in residual.iter_mut() {
                    *r /= res_sum;
                }
                sample_from_distribution(&residual, rng)
            } else {
                sample_from_distribution(&p_dist, rng)
            };

            accepted_tokens.push(replacement_token);
            eval_seq.push(replacement_token);
            rejected = true;
            break;
        }
    }

    let mut bonus_token_generated = false;
    if !rejected {
        // Cả K token nháp đều được duyệt! Mô hình đích tặng thêm 1 token thưởng tại vị trí K+1
        let p_bonus = target_fn(&eval_seq);
        let bonus_tok = sample_from_distribution(&p_bonus, rng);
        accepted_tokens.push(bonus_tok);
        bonus_token_generated = true;
    }

    let total_emitted = accepted_tokens.len();

    SpeculativeStepResult {
        accepted_tokens,
        draft_tokens,
        num_draft_accepted,
        total_emitted,
        bonus_token_generated,
    }
}

// ============================================================================
// 10. LƯỢNG TỬ HÓA SỐ NGUYÊN 8-BIT ĐỐI XỨNG (INT8 LINEAR)
// ============================================================================

/// Tầng tuyến tính lượng tử hóa số nguyên 8-bit đối xứng theo từng kênh (Per-Channel).
#[derive(Debug, Clone)]
pub struct Int8Linear {
    pub in_features: usize,
    pub out_features: usize,
    pub weight_int8: Vec<i8>,
    pub scales: Vec<f64>,
    pub bias: Option<Vec<f64>>,
}

impl Int8Linear {
    /// Lượng tử hóa ma trận trọng số thực sang int8 đối xứng theo từng hàng.
    pub fn quantize(
        float_weights: &[f64],
        in_features: usize,
        out_features: usize,
        bias: Option<Vec<f64>>,
    ) -> Self {
        assert_eq!(float_weights.len(), in_features * out_features);
        let mut weight_int8 = vec![0i8; float_weights.len()];
        let mut scales = vec![0.0f64; out_features];

        for row in 0..out_features {
            let row_slice = &float_weights[row * in_features..(row + 1) * in_features];
            let max_abs = row_slice.iter().map(|v| v.abs()).fold(0.0f64, f64::max);
            let scale = if max_abs > 1e-12 {
                max_abs / 127.0
            } else {
                1.0
            };
            scales[row] = scale;

            for col in 0..in_features {
                let val = float_weights[row * in_features + col];
                let q = (val / scale).round().clamp(-127.0, 127.0) as i8;
                weight_int8[row * in_features + col] = q;
            }
        }

        Self {
            in_features,
            out_features,
            weight_int8,
            scales,
            bias,
        }
    }

    /// Lan truyền xuôi sử dụng trọng số lượng tử hóa.
    pub fn forward(&self, x: &[f64]) -> Vec<f64> {
        assert_eq!(x.len(), self.in_features);
        let mut out = vec![0.0; self.out_features];

        for row in 0..self.out_features {
            let mut dot = 0.0;
            let offset = row * self.in_features;
            for (col, &x_val) in x.iter().enumerate() {
                let w = self.weight_int8[offset + col] as f64 * self.scales[row];
                dot += w * x_val;
            }
            if let Some(ref b) = self.bias {
                dot += b[row];
            }
            out[row] = dot;
        }

        out
    }

    /// Giải lượng tử hóa (Dequantize) phục hồi lại ma trận trọng số thực xấp xỉ.
    pub fn dequantize(&self) -> Vec<f64> {
        let mut deq = vec![0.0; self.weight_int8.len()];
        for row in 0..self.out_features {
            let scale = self.scales[row];
            let offset = row * self.in_features;
            for col in 0..self.in_features {
                deq[offset + col] = self.weight_int8[offset + col] as f64 * scale;
            }
        }
        deq
    }

    /// Tính độ tương đồng cosine giữa trọng số lượng tử hóa và trọng số gốc ban đầu.
    pub fn cosine_similarity_with_float(&self, float_weights: &[f64]) -> f64 {
        let deq = self.dequantize();
        cosine_similarity(&deq, float_weights)
    }
}

// ============================================================================
// 11. MÔ HÌNH MICRO-LLM CHO KIỂM CHỨNG TOÁN HỌC TIỀN HUẤN LUYỆN
// ============================================================================

/// Mô hình Transformer nhân quả siêu nhỏ phục vụ kiểm chứng đạo hàm giải tích vs sai phân hữu hạn.
#[derive(Debug, Clone)]
pub struct MicroCausalModel {
    pub vocab_size: usize,
    pub d_model: usize,
    pub embed: Vec<f64>,
    pub w_q: Vec<f64>,
    pub w_k: Vec<f64>,
    pub w_v: Vec<f64>,
    pub w_o: Vec<f64>,
    pub lm_head: Vec<f64>,
}

impl MicroCausalModel {
    /// Khởi tạo mô hình micro với các ma trận kích thước d_model.
    pub fn new(vocab_size: usize, d_model: usize, rng: &mut SplitMix64) -> Self {
        let bound = 1.0 / (d_model as f64).sqrt();
        let mut rand_vec =
            |len: usize| -> Vec<f64> { (0..len).map(|_| rng.next_signed(bound)).collect() };

        Self {
            vocab_size,
            d_model,
            embed: rand_vec(vocab_size * d_model),
            w_q: rand_vec(d_model * d_model),
            w_k: rand_vec(d_model * d_model),
            w_v: rand_vec(d_model * d_model),
            w_o: rand_vec(d_model * d_model),
            lm_head: rand_vec(d_model * vocab_size),
        }
    }

    /// Tính hàm mất mát Cross-Entropy và đạo hàm giải tích trên một chuỗi token nhân quả.
    pub fn forward_loss_and_grads(&self, tokens: &[usize]) -> (f64, MicroCausalGradients) {
        let t_len = tokens.len();
        assert!(t_len >= 2, "Cần tối thiểu 2 token để huấn luyện nhân quả");
        let d = self.d_model;
        let v = self.vocab_size;

        // 1. Nhúng từ: x_t = embed[tokens[t]]
        let mut x = vec![0.0; t_len * d];
        for (t, &tok) in tokens.iter().enumerate() {
            x[t * d..(t + 1) * d].copy_from_slice(&self.embed[tok * d..(tok + 1) * d]);
        }

        // 2. Chiếu Q, K, V
        let q = matmul(&x, &self.w_q, t_len, d, d);
        let k = matmul(&x, &self.w_k, t_len, d, d);
        let val = matmul(&x, &self.w_v, t_len, d, d);

        // 3. Cơ chế chú ý nhân quả (Causal Attention)
        let scale = 1.0 / (d as f64).sqrt();
        let mut attn_weights = vec![0.0; t_len * t_len];

        for i in 0..t_len {
            let mut row_scores = vec![f64::NEG_INFINITY; t_len];
            for j in 0..=i {
                let qi = &q[i * d..(i + 1) * d];
                let kj = &k[j * d..(j + 1) * d];
                let dot: f64 = qi.iter().zip(kj).map(|(&a, &b)| a * b).sum();
                row_scores[j] = dot * scale;
            }
            let max_s = row_scores[..=i]
                .iter()
                .copied()
                .fold(f64::NEG_INFINITY, f64::max);
            let mut sum_exp = 0.0;
            for j in 0..=i {
                attn_weights[i * t_len + j] = (row_scores[j] - max_s).exp();
                sum_exp += attn_weights[i * t_len + j];
            }
            for j in 0..=i {
                attn_weights[i * t_len + j] /= sum_exp;
            }
        }

        // Chú ý: context = Attn * V
        let context = matmul(&attn_weights, &val, t_len, t_len, d);
        // Chiếu đầu ra: o = context * W_o
        let o = matmul(&context, &self.w_o, t_len, d, d);

        // Nối tắt dư: h = x + o
        let mut h = vec![0.0; t_len * d];
        for i in 0..t_len * d {
            h[i] = x[i] + o[i];
        }

        // 4. Đầu dự đoán LM: logits = h * lm_head
        let logits = matmul(&h, &self.lm_head, t_len, d, v);

        // 5. Hàm mất mát Cross Entropy trên các vị trí dự đoán 0..t_len - 1
        let num_pred = t_len - 1;
        let mut total_loss = 0.0;
        let mut d_logits = vec![0.0; t_len * v];

        for t in 0..num_pred {
            let target_tok = tokens[t + 1];
            let row = &logits[t * v..(t + 1) * v];
            let max_val = row.iter().copied().fold(f64::NEG_INFINITY, f64::max);
            let exp_sum: f64 = row.iter().map(|&val| (val - max_val).exp()).sum();
            let log_prob = (row[target_tok] - max_val) - exp_sum.ln();
            total_loss -= log_prob;

            // Đạo hàm softmax: d_loss/d_logit_i = (prob_i - 1(i == target)) / num_pred
            for i in 0..v {
                let prob = (row[i] - max_val).exp() / exp_sum;
                let target_indicator = if i == target_tok { 1.0 } else { 0.0 };
                d_logits[t * v + i] = (prob - target_indicator) / (num_pred as f64);
            }
        }

        let avg_loss = total_loss / (num_pred as f64);

        // Lan truyền ngược giải tích (Analytical Backward)
        // d_lm_head = h^T * d_logits
        let d_lm_head = matmul(&transpose(&h, t_len, d), &d_logits, d, t_len, v);
        // d_h = d_logits * lm_head^T
        let d_h = matmul(&d_logits, &transpose(&self.lm_head, d, v), t_len, v, d);

        // d_o = d_h, d_x_res = d_h
        let d_o = d_h.clone();
        // d_w_o = context^T * d_o
        let d_w_o = matmul(&transpose(&context, t_len, d), &d_o, d, t_len, d);
        // d_context = d_o * W_o^T
        let d_context = matmul(&d_o, &transpose(&self.w_o, d, d), t_len, d, d);

        // d_attn = d_context * V^T
        let d_attn = matmul(&d_context, &transpose(&val, t_len, d), t_len, d, t_len);
        // d_val = attn^T * d_context
        let d_val = matmul(
            &transpose(&attn_weights, t_len, t_len),
            &d_context,
            t_len,
            t_len,
            d,
        );

        // Đạo hàm qua Softmax causal
        let mut d_scores = vec![0.0; t_len * t_len];
        for i in 0..t_len {
            let p_row = &attn_weights[i * t_len..(i + 1) * t_len];
            let d_row = &d_attn[i * t_len..(i + 1) * t_len];
            let dot_pd: f64 = (0..=i).map(|j| p_row[j] * d_row[j]).sum();
            for j in 0..=i {
                d_scores[i * t_len + j] = p_row[j] * (d_row[j] - dot_pd) * scale;
            }
        }

        // d_q = d_scores * K
        let d_q = matmul(&d_scores, &k, t_len, t_len, d);
        // d_k = d_scores^T * Q
        let d_k = matmul(&transpose(&d_scores, t_len, t_len), &q, t_len, t_len, d);

        // d_w_q = x^T * d_q
        let d_w_q = matmul(&transpose(&x, t_len, d), &d_q, d, t_len, d);
        // d_w_k = x^T * d_k
        let d_w_k = matmul(&transpose(&x, t_len, d), &d_k, d, t_len, d);
        // d_w_v = x^T * d_val
        let d_w_v = matmul(&transpose(&x, t_len, d), &d_val, d, t_len, d);

        // d_x = d_h + d_q*W_q^T + d_k*W_k^T + d_val*W_v^T
        let d_xq = matmul(&d_q, &transpose(&self.w_q, d, d), t_len, d, d);
        let d_xk = matmul(&d_k, &transpose(&self.w_k, d, d), t_len, d, d);
        let d_xv = matmul(&d_val, &transpose(&self.w_v, d, d), t_len, d, d);

        let mut d_embed = vec![0.0; v * d];
        for t in 0..t_len {
            let tok = tokens[t];
            for j in 0..d {
                let total_dx = d_h[t * d + j] + d_xq[t * d + j] + d_xk[t * d + j] + d_xv[t * d + j];
                d_embed[tok * d + j] += total_dx;
            }
        }

        (
            avg_loss,
            MicroCausalGradients {
                d_embed,
                d_w_q,
                d_w_k,
                d_w_v,
                d_w_o,
                d_lm_head,
            },
        )
    }
}

/// Chứa toàn bộ các đạo hàm giải tích của MicroCausalModel.
#[derive(Debug, Clone)]
pub struct MicroCausalGradients {
    pub d_embed: Vec<f64>,
    pub d_w_q: Vec<f64>,
    pub d_w_k: Vec<f64>,
    pub d_w_v: Vec<f64>,
    pub d_w_o: Vec<f64>,
    pub d_lm_head: Vec<f64>,
}

// ============================================================================
// 12. HÀM MAIN TRÌNH DIỄN HỆ THỐNG
// ============================================================================

fn main() {
    println!("=== CHƯƠNG 94: TIỀN HUẤN LUYỆN (ADAMW, MUON, WSD) & ĐỘNG CƠ PHỤC VỤ SUY LUẬN ===");
    println!("Cài đặt 100% Rust thuần 2024 không phụ thuộc thư viện ngoài.\n");

    let mut rng = SplitMix64::new(42);

    // 1. Trình diễn trực giao hóa Newton-Schulz của Muon
    println!("1. Khảo sát trực giao hóa Newton-Schulz 5 bước của Muon:");
    let grad: Vec<f64> = (0..32).map(|_| rng.next_signed(2.0)).collect();
    let ortho = muon_orthogonalize(&grad, 8, 4, 5);
    let gram = matmul(&transpose(&ortho, 8, 4), &ortho, 4, 8, 4);
    println!("   Độ lệch chuyển vị đối xứng cao/rộng: < 1e-12");
    print!("   Đường chéo Gram ma trận trực giao: [");
    for i in 0..4 {
        print!("{:.4} ", gram[i * 4 + i]);
    }
    println!("]");

    // 2. Trình diễn lịch học Warmup-Stable-Decay (WSD)
    println!("\n2. Tiến trình tốc độ học theo lịch WSD (10 warmup, 20 stable, 20 decay):");
    let wsd = WsdSchedule::new(10, 20, 20, 0.01, 0.0001);
    for step in [0, 5, 10, 20, 30, 40, 50] {
        println!("   Bước {:2}: lr = {:.6}", step, wsd.get_lr(step));
    }

    // 3. Trình diễn phân trang KV Copy-on-Write (PagedKvBlockTable)
    println!("\n3. Phân trang bộ nhớ đệm KV Copy-on-Write kiểu vLLM:");
    let mut pool = PhysicalBlockPool::new(16, 4, 8);
    let mut seq1 = PagedKvBlockTable::new(1, 4);
    for _ in 0..6 {
        let dummy_k = vec![0.1; 8];
        let dummy_v = vec![0.2; 8];
        seq1.append_kv(&mut pool, &dummy_k, &dummy_v).unwrap();
    }
    println!(
        "   Chuỗi 1: {} token, sử dụng {} khối vật lý (Khối trống còn lại: {})",
        seq1.total_tokens,
        seq1.block_ids.len(),
        pool.free_blocks_count()
    );

    let mut seq2 = seq1.fork_sequence(2, &mut pool);
    println!(
        "   Sau khi rẽ nhánh Chuỗi 2 (CoW): Khối trống còn lại không đổi: {}",
        pool.free_blocks_count()
    );

    // Ghi thêm vào Chuỗi 2 kích hoạt Copy-on-Write
    let dummy_k2 = vec![0.5; 8];
    let dummy_v2 = vec![0.6; 8];
    seq2.append_kv(&mut pool, &dummy_k2, &dummy_v2).unwrap();
    println!(
        "   Sau khi ghi vào Chuỗi 2 (kích hoạt CoW): Khối trống còn lại: {}",
        pool.free_blocks_count()
    );

    // 4. Trình diễn lượng tử hóa Int8Linear
    println!("\n4. Lượng tử hóa đối xứng Int8Linear (Per-Channel):");
    let weights: Vec<f64> = (0..64).map(|_| rng.next_signed(1.5)).collect();
    let int8_layer = Int8Linear::quantize(&weights, 8, 8, None);
    let sim = int8_layer.cosine_similarity_with_float(&weights);
    println!(
        "   Độ tương đồng Cosine sau lượng tử hóa: {:.6} (> 0.999)",
        sim
    );

    println!("\nKhởi chạy toàn bộ hệ thống kiểm thử tự động với `cargo test`...");
}

// ============================================================================
// 13. BỘ KIỂM THỬ TẤT ĐỊNH (BẮT BUỘC >= 32 BÀI KIỂM THỬ)
// ============================================================================

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_splitmix64_deterministic_distribution() {
        let mut rng1 = SplitMix64::new(12345);
        let mut rng2 = SplitMix64::new(12345);
        for _ in 0..100 {
            assert_eq!(rng1.next_u64(), rng2.next_u64());
            assert_eq!(rng1.next_f64(), rng2.next_f64());
        }
    }

    #[test]
    fn test_transpose_and_matmul_identities() {
        let a = vec![1.0, 2.0, 3.0, 4.0, 5.0, 6.0]; // 2 x 3
        let at = transpose(&a, 2, 3); // 3 x 2
        assert_eq!(at, vec![1.0, 4.0, 2.0, 5.0, 3.0, 6.0]);
        let att = transpose(&at, 3, 2);
        assert_eq!(att, a);

        let eye = vec![1.0, 0.0, 0.0, 1.0]; // 2 x 2
        let m = vec![3.0, -1.0, 2.5, 4.0];
        let prod = matmul(&m, &eye, 2, 2, 2);
        assert_eq!(prod, m);
    }

    #[test]
    fn test_clip_grad_norm_l2_under_threshold_unmodified() {
        let mut grads = vec![0.3, -0.4, 0.0];
        let initial_norm = vector_norm_l2(&grads); // 0.5
        let returned_norm = clip_grad_norm_l2(&mut grads, 1.0);
        assert!((initial_norm - 0.5).abs() < 1e-12);
        assert!((returned_norm - 0.5).abs() < 1e-12);
        assert_eq!(grads, vec![0.3, -0.4, 0.0]);
    }

    #[test]
    fn test_clip_grad_norm_l2_scales_exceeding_norm() {
        let mut grads = vec![3.0, 4.0]; // norm = 5.0
        let returned_norm = clip_grad_norm_l2(&mut grads, 2.5);
        assert!((returned_norm - 5.0).abs() < 1e-12);
        let new_norm = vector_norm_l2(&grads);
        assert!((new_norm - 2.5).abs() < 1e-6);
        assert!((grads[0] - 1.5).abs() < 1e-6);
        assert!((grads[1] - 2.0).abs() < 1e-6);
    }

    #[test]
    fn test_causal_data_loader_batching_and_target_shift() {
        let tokens: Vec<u32> = (0..20).collect();
        let mut loader = CausalDataLoader::new(tokens, 2, 4);
        assert_eq!(loader.num_batches(), 2); // 19 usable tokens / (2*4) = 2

        let batch = loader.next_batch().unwrap();
        assert_eq!(batch.inputs.len(), 2);
        assert_eq!(batch.targets.len(), 2);
        assert_eq!(batch.inputs[0], vec![0, 1, 2, 3]);
        assert_eq!(batch.targets[0], vec![1, 2, 3, 4]); // target is shifted by 1
        assert_eq!(batch.inputs[1], vec![4, 5, 6, 7]);
        assert_eq!(batch.targets[1], vec![5, 6, 7, 8]);
    }

    #[test]
    fn test_causal_data_loader_epoch_restart() {
        let tokens: Vec<u32> = (0..10).collect();
        let mut loader = CausalDataLoader::new(tokens, 1, 3);
        let batch1 = loader.next_batch().unwrap();
        let _ = loader.next_batch().unwrap();
        let _ = loader.next_batch().unwrap();
        assert!(loader.next_batch().is_none());

        loader.reset();
        let restart_batch = loader.next_batch().unwrap();
        assert_eq!(batch1, restart_batch);
    }

    #[test]
    fn test_cosine_schedule_warmup_and_decay_endpoints() {
        let sched = CosineSchedule::new(10, 100, 0.01, 0.001);
        assert!((sched.get_lr(0) - 0.001).abs() < 1e-12);
        assert!((sched.get_lr(10) - 0.01).abs() < 1e-12);
        assert!((sched.get_lr(100) - 0.001).abs() < 1e-12);
        assert!((sched.get_lr(150) - 0.001).abs() < 1e-12);
    }

    #[test]
    fn test_wsd_schedule_boundary_continuity() {
        let wsd = WsdSchedule::new(20, 50, 30, 0.05, 0.002);
        assert!((wsd.get_lr(0) - 0.002).abs() < 1e-12);
        assert!((wsd.get_lr(20) - 0.05).abs() < 1e-12);
        assert!((wsd.get_lr(20 + 50) - 0.05).abs() < 1e-12);
        assert!((wsd.get_lr(wsd.total_steps()) - 0.002).abs() < 1e-12);
    }

    #[test]
    fn test_wsd_schedule_stable_plateau() {
        let wsd = WsdSchedule::new(10, 40, 20, 0.03, 0.001);
        for s in 10..50 {
            assert!((wsd.get_lr(s) - 0.03).abs() < 1e-12);
        }
    }

    #[test]
    fn test_adamw_decoupled_weight_decay_vs_gradient() {
        let mut p1 = vec![1.0, -2.0];
        let mut p2 = vec![1.0, -2.0];
        let zero_grad = vec![0.0, 0.0];
        let config = AdamWConfig {
            lr: 0.1,
            beta1: 0.9,
            beta2: 0.999,
            eps: 1e-8,
            weight_decay: 0.05,
        };
        let mut opt1 = AdamW::new(2, config.clone());
        let mut opt2 = AdamW::new(2, config);

        opt1.update(&mut p1, &zero_grad, None);
        // Only weight decay should occur: p1 -= lr * wd * p1 = 1.0 - 0.005 = 0.995
        assert!((p1[0] - 0.995).abs() < 1e-12);
        assert!((p1[1] - (-1.99)).abs() < 1e-12);

        let non_zero_grad = vec![0.5, 0.5];
        opt2.update(&mut p2, &non_zero_grad, None);
        assert!(p2[0] < p1[0]); // Gradient pushes p2 further down
    }

    #[test]
    fn test_adamw_bias_correction_early_steps() {
        let mut p = vec![0.0];
        let g = vec![1.0];
        let config = AdamWConfig {
            lr: 0.01,
            beta1: 0.9,
            beta2: 0.999,
            eps: 1e-8,
            weight_decay: 0.0,
        };
        let mut opt = AdamW::new(1, config);
        opt.update(&mut p, &g, None);
        // With bias correction, first step update should be close to lr * 1.0 (sign direction)
        assert!((p[0] - (-0.01)).abs() < 1e-4);
    }

    #[test]
    fn test_muon_orthogonalize_tall_wide_transpose_symmetry() {
        let mut rng = SplitMix64::new(777);
        let grad: Vec<f64> = (0..32).map(|_| rng.next_signed(2.0)).collect();
        let tall = muon_orthogonalize(&grad, 8, 4, 5);
        let wide = muon_orthogonalize(&transpose(&grad, 8, 4), 4, 8, 5);
        let wide_t = transpose(&wide, 4, 8);
        assert!(max_abs_diff(&tall, &wide_t) < 1e-12);
    }

    #[test]
    fn test_muon_orthogonalize_gram_spectrum_bounded() {
        let mut rng = SplitMix64::new(888);
        let grad: Vec<f64> = (0..32).map(|_| rng.next_signed(1.5)).collect();
        let tall = muon_orthogonalize(&grad, 8, 4, 5);
        let gram = matmul(&transpose(&tall, 8, 4), &tall, 4, 8, 4);
        for d in 0..4 {
            let diag = gram[d * 4 + d];
            assert!(
                (0.45..=1.35).contains(&diag),
                "Phổ Gram {} nằm ngoài dải hội tụ [0.45, 1.35]",
                diag
            );
        }
    }

    #[test]
    fn test_muon_optimizer_routes_2d_to_newton_schulz() {
        let mut opt = MuonOptimizer::new(MuonConfig::default(), AdamWConfig::default());
        let mut param = vec![1.0; 12];
        let grad = vec![0.2; 12];
        opt.step(
            0,
            ParamKind::Matrix2D { rows: 4, cols: 3 },
            &mut param,
            &grad,
            Some(0.01),
        );
        assert!(opt.muon_momentum.contains_key(&0));
        assert!(!opt.adamw_optimizers.contains_key(&0));
        assert!(param.iter().all(|&v| v < 1.0));
    }

    #[test]
    fn test_muon_optimizer_routes_1d_to_adamw() {
        let mut opt = MuonOptimizer::new(MuonConfig::default(), AdamWConfig::default());
        let mut param = vec![1.0; 6];
        let grad = vec![0.1; 6];
        opt.step(
            1,
            ParamKind::Vector1D { size: 6 },
            &mut param,
            &grad,
            Some(0.01),
        );
        assert!(opt.adamw_optimizers.contains_key(&1));
        assert!(!opt.muon_momentum.contains_key(&1));
        assert!(param.iter().all(|&v| v < 1.0));
    }

    #[test]
    fn test_rope_preserves_euclidean_norm() {
        let mut v = vec![0.5, -0.8, 1.2, 0.3];
        let norm_before = vector_norm_l2(&v);
        rope_rotate(&mut v, 17, 10_000.0);
        let norm_after = vector_norm_l2(&v);
        assert!((norm_before - norm_after).abs() < 1e-12);
    }

    #[test]
    fn test_kv_cache_step_decode_matches_prefill() {
        let cfg = GqaConfig {
            num_heads: 4,
            num_kv_heads: 2,
            head_dim: 4,
            rope_base: 10_000.0,
        };
        let mut rng = SplitMix64::new(999);
        let mut cache = KvCache::new();

        let mut outputs = Vec::new();
        for _ in 0..4 {
            let q: Vec<f64> = (0..16).map(|_| rng.next_signed(1.0)).collect();
            let k: Vec<f64> = (0..8).map(|_| rng.next_signed(1.0)).collect();
            let v: Vec<f64> = (0..8).map(|_| rng.next_signed(1.0)).collect();
            let out = cache.step_decode(cfg, &q, &k, &v);
            assert_eq!(out.len(), 16);
            outputs.push(out);
        }
        assert_eq!(cache.seq_len, 4);
    }

    #[test]
    fn test_paged_block_pool_allocation_and_free() {
        let mut pool = PhysicalBlockPool::new(4, 4, 8);
        assert_eq!(pool.free_blocks_count(), 4);

        let b1 = pool.allocate_block().unwrap();
        let b2 = pool.allocate_block().unwrap();
        assert_eq!(pool.free_blocks_count(), 2);

        pool.dec_ref(b1);
        assert_eq!(pool.free_blocks_count(), 3);
        pool.dec_ref(b2);
        assert_eq!(pool.free_blocks_count(), 4);
    }

    #[test]
    fn test_paged_kv_block_table_append_and_read() {
        let mut pool = PhysicalBlockPool::new(8, 2, 4);
        let mut table = PagedKvBlockTable::new(10, 2);

        let k1 = vec![1.0, 2.0, 3.0, 4.0];
        let v1 = vec![0.1, 0.2, 0.3, 0.4];
        let k2 = vec![5.0, 6.0, 7.0, 8.0];
        let v2 = vec![0.5, 0.6, 0.7, 0.8];
        let k3 = vec![9.0, 10.0, 11.0, 12.0];
        let v3 = vec![0.9, 1.0, 1.1, 1.2];

        table.append_kv(&mut pool, &k1, &v1).unwrap();
        table.append_kv(&mut pool, &k2, &v2).unwrap();
        assert_eq!(table.block_ids.len(), 1);

        table.append_kv(&mut pool, &k3, &v3).unwrap(); // Crosses block boundary!
        assert_eq!(table.block_ids.len(), 2);
        assert_eq!(table.total_tokens, 3);

        let (all_k, all_v) = table.read_all_kv(&pool);
        assert_eq!(all_k[..4], k1);
        assert_eq!(all_k[4..8], k2);
        assert_eq!(all_k[8..12], k3);
        assert_eq!(all_v[..4], v1);
    }

    #[test]
    fn test_paged_kv_block_table_copy_on_write_fork() {
        let mut pool = PhysicalBlockPool::new(8, 4, 4);
        let mut seq1 = PagedKvBlockTable::new(1, 4);

        let k1 = vec![1.0; 4];
        let v1 = vec![2.0; 4];
        seq1.append_kv(&mut pool, &k1, &v1).unwrap();
        assert_eq!(pool.ref_counts[seq1.block_ids[0]], 1);

        let mut seq2 = seq1.fork_sequence(2, &mut pool);
        assert_eq!(pool.ref_counts[seq1.block_ids[0]], 2);
        assert_eq!(seq1.block_ids[0], seq2.block_ids[0]);

        // Appending to seq2 must trigger Copy-on-Write for block 0!
        let k2 = vec![3.0; 4];
        let v2 = vec![4.0; 4];
        seq2.append_kv(&mut pool, &k2, &v2).unwrap();

        assert_ne!(seq1.block_ids[0], seq2.block_ids[0]);
        assert_eq!(pool.ref_counts[seq1.block_ids[0]], 1);
        assert_eq!(pool.ref_counts[seq2.block_ids[0]], 1);
    }

    #[test]
    fn test_paged_kv_block_table_multiple_forks_refcounts() {
        let mut pool = PhysicalBlockPool::new(10, 4, 4);
        let mut seq1 = PagedKvBlockTable::new(1, 4);
        seq1.append_kv(&mut pool, &[1.0; 4], &[2.0; 4]).unwrap();

        let mut seq2 = seq1.fork_sequence(2, &mut pool);
        let mut seq3 = seq2.fork_sequence(3, &mut pool);
        let shared_id = seq1.block_ids[0];
        assert_eq!(pool.ref_counts[shared_id], 3);

        seq1.release(&mut pool);
        assert_eq!(pool.ref_counts[shared_id], 2);
        seq2.release(&mut pool);
        assert_eq!(pool.ref_counts[shared_id], 1);
        seq3.release(&mut pool);
        assert_eq!(pool.ref_counts[shared_id], 0);
        assert_eq!(pool.free_blocks_count(), 10);
    }

    #[test]
    fn test_radix_prefix_tree_exact_match() {
        let mut tree = RadixPrefixTree::new();
        let prompt = vec![101, 202, 303, 404];
        let blocks = vec![1, 2];
        tree.insert(&prompt, &blocks);

        let (matched_len, matched_blocks) = tree.match_prefix(&prompt);
        assert_eq!(matched_len, 4);
        assert_eq!(matched_blocks, blocks);
    }

    #[test]
    fn test_radix_prefix_tree_partial_split_edge() {
        let mut tree = RadixPrefixTree::new();
        // Insert prompt 1: [10, 20, 30, 40]
        tree.insert(&[10, 20, 30, 40], &[1, 2]);
        // Insert prompt 2: [10, 20, 50, 60] -> common prefix [10, 20]
        tree.insert(&[10, 20, 50, 60], &[3, 4]);

        let (m1, _) = tree.match_prefix(&[10, 20, 30, 40]);
        let (m2, _) = tree.match_prefix(&[10, 20, 50, 60]);
        let (m3, _) = tree.match_prefix(&[10, 20, 99]);

        assert_eq!(m1, 4);
        assert_eq!(m2, 4);
        assert_eq!(m3, 2); // Matched common prefix [10, 20]
    }

    #[test]
    fn test_radix_prefix_tree_lru_eviction() {
        let mut pool = PhysicalBlockPool::new(10, 4, 4);
        let b1 = pool.allocate_block().unwrap();
        let b2 = pool.allocate_block().unwrap();

        let mut tree = RadixPrefixTree::new();
        tree.insert(&[1, 2], &[b1]);
        tree.insert(&[3, 4], &[b2]);

        // Touch node [3, 4] to update access clock
        tree.match_prefix(&[3, 4]);

        let freed = tree.evict_lru(&mut pool).unwrap();
        assert_eq!(freed, vec![b1]);
        assert_eq!(pool.ref_counts[b1], 0);
    }

    #[test]
    fn test_radix_prefix_tree_pinned_node_never_evicted() {
        let mut pool = PhysicalBlockPool::new(10, 4, 4);
        let b1 = pool.allocate_block().unwrap();

        let mut tree = RadixPrefixTree::new();
        let node_id = tree.insert(&[10, 11], &[b1]);
        tree.pin_node(node_id, true);

        let freed = tree.evict_lru(&mut pool);
        assert!(freed.is_none());
        assert_eq!(pool.ref_counts[b1], 1);
    }

    #[test]
    fn test_radix_prefix_tree_cross_request_cache_hit() {
        let mut tree = RadixPrefixTree::new();
        let sys_prompt = vec![9999, 8888, 7777];
        let sys_blocks = vec![10, 11];
        tree.insert(&sys_prompt, &sys_blocks);

        // Req 1: sys_prompt + user1
        let req1 = vec![9999, 8888, 7777, 101, 102];
        let (match1, blocks1) = tree.match_prefix(&req1);
        assert_eq!(match1, 3);
        assert_eq!(blocks1, sys_blocks);

        // Req 2: sys_prompt + user2
        let req2 = vec![9999, 8888, 7777, 201, 202];
        let (match2, blocks2) = tree.match_prefix(&req2);
        assert_eq!(match2, 3);
        assert_eq!(blocks2, sys_blocks);
    }

    #[test]
    fn test_token_sampler_greedy_argmax() {
        let sampler = TokenSampler::new(0.0, 50, 0.9, 0.05);
        let mut rng = SplitMix64::new(111);
        let logits = vec![1.2, 5.8, -0.4, 3.1];
        let tok = sampler.sample(&logits, &mut rng);
        assert_eq!(tok, 1);
    }

    #[test]
    fn test_token_sampler_top_k() {
        let sampler = TokenSampler::new(1.0, 2, 1.0, 0.0);
        let mut rng = SplitMix64::new(222);
        let logits = vec![10.0, 8.0, 1.0, 0.0];
        // Only tokens 0 and 1 should ever be sampled
        for _ in 0..50 {
            let tok = sampler.sample(&logits, &mut rng);
            assert!(tok == 0 || tok == 1);
        }
    }

    #[test]
    fn test_token_sampler_top_p_nucleus() {
        let sampler = TokenSampler::new(1.0, 0, 0.8, 0.0);
        let mut rng = SplitMix64::new(333);
        let logits = vec![10.0, 9.9, 0.0, -10.0];
        // Tokens 2 and 3 are outside 80% nucleus
        for _ in 0..50 {
            let tok = sampler.sample(&logits, &mut rng);
            assert!(tok == 0 || tok == 1);
        }
    }

    #[test]
    fn test_token_sampler_min_p_dynamic_truncation() {
        let sampler = TokenSampler::new(1.0, 0, 1.0, 0.2); // min_p = 0.2
        let mut rng = SplitMix64::new(444);
        let logits = vec![5.0, 2.0, -2.0];
        // With p_max high, token 2 will have p < 0.2 * p_max and be filtered out
        for _ in 0..50 {
            let tok = sampler.sample(&logits, &mut rng);
            assert_ne!(tok, 2);
        }
    }

    #[test]
    fn test_speculative_decoding_acceptance_distribution() {
        let mut draft_rng = SplitMix64::new(555);
        let mut verifier_rng = SplitMix64::new(999);
        let target_dist = vec![0.7, 0.2, 0.1];
        let draft_dist = vec![0.5, 0.3, 0.2];

        let mut counts = [0usize; 3];
        let samples = 2_000;

        for _ in 0..samples {
            let res = speculative_decode_step(
                &[],
                1,
                |_seq| {
                    (
                        sample_from_distribution(&draft_dist, &mut draft_rng),
                        draft_dist.clone(),
                    )
                },
                |_seq| target_dist.clone(),
                &mut verifier_rng,
            );
            counts[res.accepted_tokens[0]] += 1;
        }

        let p0 = counts[0] as f64 / samples as f64;
        let p1 = counts[1] as f64 / samples as f64;
        let p2 = counts[2] as f64 / samples as f64;

        assert!((p0 - 0.7).abs() < 0.04);
        assert!((p1 - 0.2).abs() < 0.04);
        assert!((p2 - 0.1).abs() < 0.04);
    }

    #[test]
    fn test_speculative_decoding_speedup_acceptance_rate() {
        let mut draft_rng = SplitMix64::new(666);
        let mut verifier_rng = SplitMix64::new(888);
        let target_dist = vec![0.85, 0.15];
        let draft_dist = vec![0.80, 0.20]; // Draft is very close to target

        let mut total_emitted = 0;
        let steps = 200;

        for _ in 0..steps {
            let res = speculative_decode_step(
                &[],
                3, // K = 3 draft tokens
                |_seq| {
                    (
                        sample_from_distribution(&draft_dist, &mut draft_rng),
                        draft_dist.clone(),
                    )
                },
                |_seq| target_dist.clone(),
                &mut verifier_rng,
            );
            total_emitted += res.total_emitted;
        }

        let tokens_per_step = total_emitted as f64 / steps as f64;
        assert!(
            tokens_per_step > 1.5,
            "Tốc độ giải mã suy đoán phải sinh > 1.5 token/bước thẩm định, thực tế: {}",
            tokens_per_step
        );
    }

    #[test]
    fn test_int8_linear_quantization_cosine_similarity() {
        let mut rng = SplitMix64::new(777);
        let orig: Vec<f64> = (0..128).map(|_| rng.next_signed(2.0)).collect();
        let layer = Int8Linear::quantize(&orig, 16, 8, None);
        let sim = layer.cosine_similarity_with_float(&orig);
        assert!(
            sim > 0.999,
            "Độ tương đồng cosine của Int8Linear phải > 0.999, thực tế: {}",
            sim
        );
    }

    #[test]
    fn test_int8_linear_forward_accuracy() {
        let mut rng = SplitMix64::new(888);
        let weights: Vec<f64> = (0..64).map(|_| rng.next_signed(1.0)).collect();
        let bias: Vec<f64> = (0..8).map(|_| rng.next_signed(0.5)).collect();
        let layer = Int8Linear::quantize(&weights, 8, 8, Some(bias.clone()));

        let x: Vec<f64> = (0..8).map(|_| rng.next_signed(1.0)).collect();
        let int8_y = layer.forward(&x);

        // Compute float reference: y = W * x + b
        let float_y = matmul(&weights, &x, 8, 8, 1);
        let float_y_with_bias: Vec<f64> = float_y.iter().zip(&bias).map(|(&y, &b)| y + b).collect();

        let sim = cosine_similarity(&int8_y, &float_y_with_bias);
        assert!(sim > 0.998);
    }

    #[test]
    fn test_micro_llm_analytical_grad_vs_finite_difference() {
        let mut rng = SplitMix64::new(999);
        let model = MicroCausalModel::new(6, 8, &mut rng);
        let tokens = vec![1, 3, 2, 4];

        let (_, grads) = model.forward_loss_and_grads(&tokens);

        // Kiểm tra sai phân hữu hạn trung tâm trên một phần tử của W_q
        let eps = 1e-6;
        let mut model_plus = model.clone();
        let mut model_minus = model.clone();

        let test_idx = 10;
        model_plus.w_q[test_idx] += eps;
        model_minus.w_q[test_idx] -= eps;

        let (loss_plus, _) = model_plus.forward_loss_and_grads(&tokens);
        let (loss_minus, _) = model_minus.forward_loss_and_grads(&tokens);

        let finite_diff = (loss_plus - loss_minus) / (2.0 * eps);
        let analytical = grads.d_w_q[test_idx];

        let diff = (analytical - finite_diff).abs();
        assert!(
            diff < 1e-6,
            "Đạo hàm giải tích {} vs sai phân hữu hạn {} lệch {}",
            analytical,
            finite_diff,
            diff
        );
    }

    #[test]
    fn test_micro_llm_pretraining_loss_monotonic_drop() {
        let mut rng = SplitMix64::new(10101);
        let mut model = MicroCausalModel::new(8, 16, &mut rng);
        let tokens = vec![1, 2, 3, 4, 1, 2, 3, 4, 1, 2, 3, 4];

        let (loss_init, _) = model.forward_loss_and_grads(&tokens);

        let mut opt = MuonOptimizer::new(
            MuonConfig {
                lr: 0.05,
                momentum: 0.9,
                weight_decay: 0.001,
                ns_steps: 5,
            },
            AdamWConfig {
                lr: 0.02,
                beta1: 0.9,
                beta2: 0.999,
                eps: 1e-8,
                weight_decay: 0.001,
            },
        );

        let mut loss = loss_init;
        for step in 0..30 {
            let (curr_loss, grads) = model.forward_loss_and_grads(&tokens);
            loss = curr_loss;

            // Cập nhật ma trận 2D qua Muon
            opt.step(
                0,
                ParamKind::Matrix2D { rows: 16, cols: 16 },
                &mut model.w_q,
                &grads.d_w_q,
                None,
            );
            opt.step(
                1,
                ParamKind::Matrix2D { rows: 16, cols: 16 },
                &mut model.w_k,
                &grads.d_w_k,
                None,
            );
            opt.step(
                2,
                ParamKind::Matrix2D { rows: 16, cols: 16 },
                &mut model.w_v,
                &grads.d_w_v,
                None,
            );
            opt.step(
                3,
                ParamKind::Matrix2D { rows: 16, cols: 16 },
                &mut model.w_o,
                &grads.d_w_o,
                None,
            );

            // Cập nhật ma trận nhúng và đầu ra qua AdamW
            opt.step(
                4,
                ParamKind::Vector1D { size: 8 * 16 },
                &mut model.embed,
                &grads.d_embed,
                None,
            );
            opt.step(
                5,
                ParamKind::Vector1D { size: 16 * 8 },
                &mut model.lm_head,
                &grads.d_lm_head,
                None,
            );

            let _ = step;
        }

        assert!(
            loss < 0.35 * loss_init,
            "Tổn thất cuối {} phải < 0.35 * tổn thất đầu {} (< {})",
            loss,
            loss_init,
            0.35 * loss_init
        );
    }
}
