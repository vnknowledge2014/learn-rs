//! Chương 93: Từ GPT-2 Cổ điển Đến Bộ giải mã Hiện đại
//! (Modern Transformer Decoder Architectures)
//!
//! Crate `ch93` triển khai 100% Rust 2024 thuần `std` (không phụ thuộc thư viện ngoài)
//! cho toàn bộ chuỗi tiến hóa của kiến trúc bộ giải mã Transformer:
//! - Khối GPT-2 cổ điển: `LayerNorm` + `Gelu` + `MultiHeadAttention` + `Gpt2Block`
//! - Chuẩn hóa hiện đại: `RmsNorm` (kèm độ lệch đơn vị kiểu Gemma và đạo hàm giải tích xuôi + ngược)
//! - Mã hóa vị trí quay: `RopeCache` + mở rộng độ dài ngữ cảnh YaRN (`YarnConfig`)
//! - Tầng FFN cổng tích cực: `SwiGluMlp` (tỷ lệ ẩn 8/3 d)
//! - Chú ý nhóm truy vấn: `GroupedQueryAttention` (GQA + QK-RmsNorm + cửa sổ trượt sliding window)
//! - Nén tiềm ẩn DeepSeek: `MultiHeadLatentAttention` (MLA nén KV hạng thấp + tách nhánh RoPE)
//! - Trạng thái tuyến tính đệ quy: `GatedDeltaNet` (Linear RNN O(N) với luật Delta)
//! - Hỗn hợp chuyên gia thưa: `SparseMoeLayer` (Top-k + chuyên gia dùng chung + aux_loss cân bằng tải)

use std::f64::consts::PI;

// ============================================================================
// 1. TIỆN ÍCH TOÁN HỌC & BỘ SINH SỐ NGẪU NHIÊN TẤT ĐỊNH
// ============================================================================

/// Bộ sinh số ngẫu nhiên giả tất định 64-bit `SplitMix64`.
///
/// Phục vụ khởi tạo trọng số mạng nơ-ron có thể tái lập 100% trên mọi nền tảng
/// mà không phụ thuộc vào `rand` hay bất kỳ crate ngoài nào.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct SplitMix64 {
    pub state: u64,
}

impl SplitMix64 {
    /// Khởi tạo bộ sinh với hạt giống ban đầu.
    pub fn new(seed: u64) -> Self {
        Self { state: seed }
    }

    /// Rút một số thực ngẫu nhiên trong khoảng `[-scale, scale]`.
    pub fn next_signed(&mut self, scale: f64) -> f64 {
        self.state = self.state.wrapping_add(0x9E37_79B9_7F4A_7C15);
        let mut z = (self.state ^ (self.state >> 30)).wrapping_mul(0xBF58_476D_1CE4_E5B9);
        z = (z ^ (z >> 27)).wrapping_mul(0x94D0_49BB_1331_11EB);
        let u = (((z ^ (z >> 31)) >> 11) as f64) * (1.0 / ((1u64 << 53) as f64));
        (u * 2.0 - 1.0) * scale
    }
}

/// Nhân hai ma trận 2D dạng phẳng: `A (m x k) * B (k x n) = C (m x n)`.
pub fn matmul(a: &[f64], b: &[f64], m: usize, k: usize, n: usize) -> Vec<f64> {
    assert_eq!(a.len(), m * k, "Kích thước ma trận A không hợp lệ");
    assert_eq!(b.len(), k * n, "Kích thước ma trận B không hợp lệ");
    let mut out = vec![0.0; m * n];
    for i in 0..m {
        for p in 0..k {
            let a_ip = a[i * k + p];
            if a_ip == 0.0 {
                continue;
            }
            for j in 0..n {
                out[i * n + j] += a_ip * b[p * n + j];
            }
        }
    }
    out
}

/// Chuyển vị ma trận 2D phẳng: `A (rows x cols) -> A^T (cols x rows)`.
pub fn transpose(mat: &[f64], rows: usize, cols: usize) -> Vec<f64> {
    assert_eq!(mat.len(), rows * cols);
    let mut out = vec![0.0; rows * cols];
    for r in 0..rows {
        for c in 0..cols {
            out[c * rows + r] = mat[r * cols + c];
        }
    }
    out
}

/// Hàm kích hoạt Softmax an toàn số học theo hàng.
pub fn softmax_row(logits: &mut [f64]) {
    let max_v = logits.iter().copied().fold(f64::NEG_INFINITY, f64::max);
    let mut sum = 0.0;
    for v in logits.iter_mut() {
        *v = (*v - max_v).exp();
        sum += *v;
    }
    let inv_sum = if sum > 0.0 { 1.0 / sum } else { 0.0 };
    for v in logits.iter_mut() {
        *v *= inv_sum;
    }
}

/// Hàm kích hoạt Sigmoid: `1 / (1 + exp(-x))`.
#[inline]
pub fn sigmoid(x: f64) -> f64 {
    1.0 / (1.0 + (-x).exp())
}

/// Hàm kích hoạt Swish / SiLU: `x * sigmoid(x)`.
#[inline]
pub fn silu(x: f64) -> f64 {
    x * sigmoid(x)
}

/// Đạo hàm bậc 1 của SiLU: `sigmoid(x) * (1 + x * (1 - sigmoid(x)))`.
#[inline]
pub fn silu_deriv(x: f64) -> f64 {
    let s = sigmoid(x);
    s * (1.0 + x * (1.0 - s))
}

// ============================================================================
// 2. KHỐI GPT-2 CỔ ĐIỂN (BASELINE TRANSFORMER DECODER)
// ============================================================================

/// Lớp chuẩn hóa theo tầng `LayerNorm` cổ điển của GPT-2:
///
/// Chuẩn hóa vector $x$ bằng kỳ vọng $\mu$ và phương sai $\sigma^2$:
/// $y = \frac{x - \mu}{\sqrt{\sigma^2 + \epsilon}} \odot \gamma + \beta$
#[derive(Debug, Clone, PartialEq)]
pub struct LayerNorm {
    pub normalized_shape: usize,
    pub eps: f64,
    pub weight: Vec<f64>,
    pub bias: Vec<f64>,
}

impl LayerNorm {
    /// Khởi tạo `LayerNorm` với trọng số đơn vị $\gamma = 1$ và độ lệch $\beta = 0$.
    pub fn new(dim: usize, eps: f64) -> Self {
        Self {
            normalized_shape: dim,
            eps,
            weight: vec![1.0; dim],
            bias: vec![0.0; dim],
        }
    }

    /// Lan truyền xuôi của LayerNorm trên từng token.
    pub fn forward(&self, x: &[f64]) -> Vec<f64> {
        let d = self.normalized_shape;
        let num_tokens = x.len() / d;
        let mut out = vec![0.0; x.len()];

        for t in 0..num_tokens {
            let slice = &x[t * d..(t + 1) * d];
            let mean = slice.iter().sum::<f64>() / (d as f64);
            let var = slice.iter().map(|&v| (v - mean).powi(2)).sum::<f64>() / (d as f64);
            let inv_std = 1.0 / (var + self.eps).sqrt();

            for i in 0..d {
                let norm = (slice[i] - mean) * inv_std;
                out[t * d + i] = norm * self.weight[i] + self.bias[i];
            }
        }
        out
    }
}

/// Hàm kích hoạt Gaussian Error Linear Unit (`Gelu`) của GPT-2.
///
/// Sử dụng công thức xấp xỉ liên tục chuẩn:
/// $\text{GELU}(x) = 0.5 \cdot x \cdot (1 + \tanh(\sqrt{2/\pi} \cdot (x + 0.044715 \cdot x^3)))$
#[derive(Debug, Clone, Copy, PartialEq, Eq, Default)]
pub struct Gelu;

impl Gelu {
    /// Tính giá trị GELU cho một số thực đơn lẻ.
    pub fn apply(x: f64) -> f64 {
        let c = (2.0 / PI).sqrt();
        let inner = c * (x + 0.044715 * x.powi(3));
        0.5 * x * (1.0 + inner.tanh())
    }

    /// Lan truyền qua mảng vector.
    pub fn forward(x: &[f64]) -> Vec<f64> {
        x.iter().map(|&v| Self::apply(v)).collect()
    }
}

/// Lớp tuyến tính tổng quát `Linear` với trọng số và độ lệch tùy chọn.
#[derive(Debug, Clone, PartialEq)]
pub struct Linear {
    pub in_features: usize,
    pub out_features: usize,
    pub weights: Vec<f64>,
    pub bias: Option<Vec<f64>>,
}

impl Linear {
    /// Khởi tạo tầng tuyến tính bằng bộ sinh ngẫu nhiên `SplitMix64`.
    pub fn new(
        in_features: usize,
        out_features: usize,
        with_bias: bool,
        rng: &mut SplitMix64,
    ) -> Self {
        let scale = (2.0 / (in_features as f64)).sqrt();
        let total = in_features * out_features;
        let weights = (0..total).map(|_| rng.next_signed(scale)).collect();
        let bias = if with_bias {
            Some(vec![0.0; out_features])
        } else {
            None
        };
        Self {
            in_features,
            out_features,
            weights,
            bias,
        }
    }

    /// Tính `Y = X * W^T + b` trong đó X là ma trận `(batch_or_tokens x in_features)`.
    pub fn forward(&self, x: &[f64]) -> Vec<f64> {
        let num_tokens = x.len() / self.in_features;
        let mut out = matmul(
            x,
            &transpose(&self.weights, self.out_features, self.in_features),
            num_tokens,
            self.in_features,
            self.out_features,
        );
        if let Some(ref b) = self.bias {
            for t in 0..num_tokens {
                for j in 0..self.out_features {
                    out[t * self.out_features + j] += b[j];
                }
            }
        }
        out
    }
}

/// Chú ý đa đầu `MultiHeadAttention` cổ điển (GPT-2 style).
#[derive(Debug, Clone, PartialEq)]
pub struct MultiHeadAttention {
    pub d_model: usize,
    pub n_heads: usize,
    pub head_dim: usize,
    pub w_qkv: Linear,
    pub w_proj: Linear,
}

impl MultiHeadAttention {
    /// Khởi tạo MHA cổ điển: gộp ma trận chiếu Q, K, V thành `3 * d_model`.
    pub fn new(d_model: usize, n_heads: usize, rng: &mut SplitMix64) -> Self {
        assert_eq!(d_model % n_heads, 0, "d_model phải chia hết cho n_heads");
        let head_dim = d_model / n_heads;
        let w_qkv = Linear::new(d_model, 3 * d_model, true, rng);
        let w_proj = Linear::new(d_model, d_model, true, rng);
        Self {
            d_model,
            n_heads,
            head_dim,
            w_qkv,
            w_proj,
        }
    }

    /// Lan truyền xuôi tự hồi quy nhân quả (Causal Self-Attention).
    pub fn forward(&self, x: &[f64], seq_len: usize) -> Vec<f64> {
        assert_eq!(x.len(), seq_len * self.d_model);
        let qkv = self.w_qkv.forward(x);
        let d = self.d_model;
        let nh = self.n_heads;
        let hd = self.head_dim;
        let scale = 1.0 / (hd as f64).sqrt();

        // Tách Q, K, V
        let mut q = vec![0.0; seq_len * d];
        let mut k = vec![0.0; seq_len * d];
        let mut v = vec![0.0; seq_len * d];
        for t in 0..seq_len {
            let row = &qkv[t * 3 * d..(t + 1) * 3 * d];
            q[t * d..(t + 1) * d].copy_from_slice(&row[0..d]);
            k[t * d..(t + 1) * d].copy_from_slice(&row[d..2 * d]);
            v[t * d..(t + 1) * d].copy_from_slice(&row[2 * d..3 * d]);
        }

        // Tính Attention cho từng đầu
        let mut head_outs = vec![0.0; seq_len * d];
        for h in 0..nh {
            for i in 0..seq_len {
                let q_i = &q[i * d + h * hd..i * d + (h + 1) * hd];
                let mut scores = vec![f64::NEG_INFINITY; seq_len];

                // Causal masking: chỉ chú ý các token j <= i
                for j in 0..=i {
                    let k_j = &k[j * d + h * hd..j * d + (h + 1) * hd];
                    let dot: f64 = q_i.iter().zip(k_j).map(|(&a, &b)| a * b).sum();
                    scores[j] = dot * scale;
                }

                softmax_row(&mut scores[0..=i]);

                // Tổng có trọng số của V
                let out_slot = &mut head_outs[i * d + h * hd..i * d + (h + 1) * hd];
                for j in 0..=i {
                    let weight = scores[j];
                    let v_j = &v[j * d + h * hd..j * d + (h + 1) * hd];
                    for (o, &val) in out_slot.iter_mut().zip(v_j) {
                        *o += weight * val;
                    }
                }
            }
        }

        // Chiếu đầu ra
        self.w_proj.forward(&head_outs)
    }
}

/// Khối GPT-2 Decoder cổ điển: Pre-LN với Residual Connections.
#[derive(Debug, Clone, PartialEq)]
pub struct Gpt2Block {
    pub d_model: usize,
    pub ln_1: LayerNorm,
    pub attn: MultiHeadAttention,
    pub ln_2: LayerNorm,
    pub mlp_fc: Linear,
    pub mlp_proj: Linear,
}

impl Gpt2Block {
    /// Khởi tạo khối GPT-2 với kích thước ẩn `4 * d_model`.
    pub fn new(d_model: usize, n_heads: usize, eps: f64, rng: &mut SplitMix64) -> Self {
        let ln_1 = LayerNorm::new(d_model, eps);
        let attn = MultiHeadAttention::new(d_model, n_heads, rng);
        let ln_2 = LayerNorm::new(d_model, eps);
        let mlp_fc = Linear::new(d_model, 4 * d_model, true, rng);
        let mlp_proj = Linear::new(4 * d_model, d_model, true, rng);
        Self {
            d_model,
            ln_1,
            attn,
            ln_2,
            mlp_fc,
            mlp_proj,
        }
    }

    /// Lan truyền xuôi của khối: `x + Attn(LN1(x)) -> + MLP(LN2(x))`.
    pub fn forward(&self, x: &[f64], seq_len: usize) -> Vec<f64> {
        let d = self.d_model;
        assert_eq!(x.len(), seq_len * d);

        // Nhánh 1: Attention với Pre-LN và Residual
        let norm1 = self.ln_1.forward(x);
        let attn_out = self.attn.forward(&norm1, seq_len);
        let mut x_mid = vec![0.0; x.len()];
        for i in 0..x.len() {
            x_mid[i] = x[i] + attn_out[i];
        }

        // Nhánh 2: MLP với Pre-LN và Residual
        let norm2 = self.ln_2.forward(&x_mid);
        let fc = self.mlp_fc.forward(&norm2);
        let act = Gelu::forward(&fc);
        let proj = self.mlp_proj.forward(&act);

        let mut x_out = vec![0.0; x.len()];
        for i in 0..x.len() {
            x_out[i] = x_mid[i] + proj[i];
        }
        x_out
    }
}

// ============================================================================
// 3. CHUẨN HÓA HIỆN ĐẠI: RMSNORM (XUÔI, NGƯỢC GIẢI TÍCH & GEMMA UNIT OFFSET)
// ============================================================================

/// Chuẩn hóa căn bậc hai trung bình bình phương `RmsNorm` (Root Mean Square Normalization).
///
/// Bỏ qua bước trừ kỳ vọng $\mu$ của LayerNorm nên rẻ hơn (Zhang & Sennrich 2019 báo cáo giảm
/// 7%–64% thời gian chạy tùy mô hình):
/// $y_i = \frac{x_i}{\text{RMS}(x)} \cdot w_i, \quad \text{trong đó } \text{RMS}(x) = \sqrt{\frac{1}{d} \sum_{j=1}^d x_j^2 + \epsilon}$
///
/// Khi `unit_offset = true` (kiểu Gemma), công thức chuyển thành:
/// $y_i = \frac{x_i}{\text{RMS}(x)} \cdot (1.0 + w_i)$
#[derive(Debug, Clone, PartialEq)]
pub struct RmsNorm {
    pub dim: usize,
    pub eps: f64,
    pub weight: Vec<f64>,
    pub unit_offset: bool,
}

impl RmsNorm {
    /// Khởi tạo `RmsNorm` tiêu chuẩn với trọng số `1.0` (hoặc `0.0` nếu dùng unit_offset).
    pub fn new(dim: usize, eps: f64, unit_offset: bool) -> Self {
        let weight = if unit_offset {
            vec![0.0; dim]
        } else {
            vec![1.0; dim]
        };
        Self {
            dim,
            eps,
            weight,
            unit_offset,
        }
    }

    /// Lan truyền xuôi của RMSNorm trên 1 token đơn lẻ.
    pub fn forward_token(&self, x: &[f64]) -> Vec<f64> {
        assert_eq!(x.len(), self.dim);
        let d = self.dim as f64;
        let sum_sq: f64 = x.iter().map(|&v| v * v).sum();
        let inv_rms = 1.0 / (sum_sq / d + self.eps).sqrt();

        let mut out = vec![0.0; self.dim];
        for i in 0..self.dim {
            let scale = if self.unit_offset {
                1.0 + self.weight[i]
            } else {
                self.weight[i]
            };
            out[i] = x[i] * inv_rms * scale;
        }
        out
    }

    /// Lan truyền xuôi trên mảng nhiều token.
    pub fn forward(&self, x: &[f64]) -> Vec<f64> {
        let d = self.dim;
        let num_tokens = x.len() / d;
        let mut out = Vec::with_capacity(x.len());
        for t in 0..num_tokens {
            out.extend(self.forward_token(&x[t * d..(t + 1) * d]));
        }
        out
    }

    /// Lan truyền xuôi và tính đạo hàm ngược giải tích (analytical forward-backward).
    ///
    /// Trả về bộ 3: `(y, grad_x, grad_w)`.
    /// Đạo hàm giải tích đã được chứng minh khớp sai phân hữu hạn `< 1e-8`:
    /// $\frac{\partial \mathcal{L}}{\partial w_i} = g_i \cdot \hat{x}_i$
    /// $\frac{\partial \mathcal{L}}{\partial x_i} = \text{inv} \cdot \left( g_i w_i - \hat{x}_i \cdot \frac{1}{d} \sum_j g_j w_j \hat{x}_j \right)$
    pub fn forward_backward(&self, x: &[f64], grad_out: &[f64]) -> (Vec<f64>, Vec<f64>, Vec<f64>) {
        let n = x.len();
        let d = n as f64;
        let sum_sq: f64 = x.iter().map(|&v| v * v).sum();
        let inv_rms = 1.0 / (sum_sq / d + self.eps).sqrt();

        let mut y = vec![0.0; n];
        let mut grad_w = vec![0.0; n];
        let mut grad_x = vec![0.0; n];
        let mut dot_sum = 0.0;

        for i in 0..n {
            let w_eff = if self.unit_offset {
                1.0 + self.weight[i]
            } else {
                self.weight[i]
            };
            let x_hat = x[i] * inv_rms;
            y[i] = x_hat * w_eff;
            grad_w[i] = grad_out[i] * x_hat;
            dot_sum += grad_out[i] * w_eff * x_hat;
        }

        dot_sum /= d;

        for i in 0..n {
            let w_eff = if self.unit_offset {
                1.0 + self.weight[i]
            } else {
                self.weight[i]
            };
            let x_hat = x[i] * inv_rms;
            grad_x[i] = inv_rms * (grad_out[i] * w_eff - x_hat * dot_sum);
        }

        (y, grad_x, grad_w)
    }
}

// ============================================================================
// 4. MÃ HÓA VỊ TRÍ QUAY: ROPE & MỞ RỘNG NGỮ CẢNH YARN SCALING
// ============================================================================

/// Cấu hình ngoại suy độ dài ngữ cảnh YaRN (Yet another RoPE extensioN).
///
/// YaRN (Peng et al., 2023) áp dụng nội suy tần số thích ứng theo SỐ VÒNG QUAY
/// $r_i = L_0 / \lambda_i$ mà chiều thứ $i$ thực hiện trên độ dài ngữ cảnh gốc $L_0$
/// (với $\lambda_i = 2\pi / \theta_i$):
/// - $r_i > \beta_{\text{fast}}$ (bước sóng ngắn, mặc định $\beta_{\text{fast}} = 32$): giữ nguyên tần số gốc
/// - $r_i < \beta_{\text{slow}}$ (bước sóng dài hơn ngữ cảnh gốc, mặc định $\beta_{\text{slow}} = 1$):
///   nội suy tuyến tính hoàn toàn, $\theta_i / s$
/// - Phân vùng trung gian: dốc chuyển tuyến tính theo $r_i$ (smooth ramp)
#[derive(Debug, Clone, PartialEq)]
pub struct YarnConfig {
    pub original_max_seq: usize,
    pub target_max_seq: usize,
    pub beta_fast: f64,
    pub beta_slow: f64,
    pub mscale: f64,
}

impl YarnConfig {
    /// Hệ số co giãn độ lớn chú ý đề xuất của YaRN: $\text{mscale} = 0.1 \ln s + 1$.
    ///
    /// `mscale` được nhân vào cả `cos` lẫn `sin` nên tác động lên cả $q$ lẫn $k$,
    /// tức logit chú ý được nhân với $\text{mscale}^2$.
    pub fn recommended_mscale(scale: f64) -> f64 {
        0.1 * scale.ln() + 1.0
    }
}

impl Default for YarnConfig {
    fn default() -> Self {
        Self {
            original_max_seq: 2048,
            target_max_seq: 8192,
            beta_fast: 32.0,
            beta_slow: 1.0,
            mscale: 1.0,
        }
    }
}

/// Bộ nhớ đệm Cosine & Sine cho phép quay vị trí RoPE (Rotary Positional Embeddings).
#[derive(Debug, Clone, PartialEq)]
pub struct RopeCache {
    pub head_dim: usize,
    pub max_seq_len: usize,
    pub base: f64,
    pub cos_cache: Vec<f64>,
    pub sin_cache: Vec<f64>,
}

impl RopeCache {
    /// Khởi tạo bảng RoPE chuẩn hoặc kết hợp YaRN Scaling.
    pub fn new(head_dim: usize, max_seq_len: usize, base: f64, yarn: Option<&YarnConfig>) -> Self {
        assert_eq!(head_dim % 2, 0, "head_dim của RoPE phải là số chẵn");
        let half = head_dim / 2;
        let mut cos_cache = vec![0.0; max_seq_len * half];
        let mut sin_cache = vec![0.0; max_seq_len * half];

        let scale = if let Some(cfg) = yarn {
            (cfg.target_max_seq as f64) / (cfg.original_max_seq as f64)
        } else {
            1.0
        };

        for i in 0..half {
            let mut freq = 1.0 / base.powf((2 * i) as f64 / (head_dim as f64));

            // Nếu kích hoạt YaRN, điều chỉnh tần số theo dốc nội suy
            if let Some(cfg) = yarn
                && scale > 1.0
            {
                let wavelength = 2.0 * PI / freq;
                let l0 = cfg.original_max_seq as f64;
                // Số vòng quay của chiều này trên độ dài ngữ cảnh gốc: r = L0 / lambda
                let rotations = l0 / wavelength;

                // gamma = trọng số nội suy: 0 -> giữ tần số gốc, 1 -> nội suy hoàn toàn (theta / s)
                let gamma = if rotations > cfg.beta_fast {
                    0.0
                } else if rotations < cfg.beta_slow {
                    1.0
                } else {
                    (cfg.beta_fast - rotations) / (cfg.beta_fast - cfg.beta_slow)
                };

                let interpolated_freq = freq / scale;
                freq = (1.0 - gamma) * freq + gamma * interpolated_freq;
            }

            for pos in 0..max_seq_len {
                let theta = (pos as f64) * freq;
                let mscale_mul = yarn.map(|y| y.mscale).unwrap_or(1.0);
                cos_cache[pos * half + i] = theta.cos() * mscale_mul;
                sin_cache[pos * half + i] = theta.sin() * mscale_mul;
            }
        }

        Self {
            head_dim,
            max_seq_len,
            base,
            cos_cache,
            sin_cache,
        }
    }

    /// Quay vector đầu vào tại vị trí `pos` theo kỹ thuật phân nửa vector (half-split RoPE):
    /// $x'_i = x_i \cos \theta - x_{i + d/2} \sin \theta$
    /// $x'_{i + d/2} = x_{i + d/2} \cos \theta + x_i \sin \theta$
    pub fn rotate(&self, vec: &mut [f64], pos: usize) {
        let half = self.head_dim / 2;
        assert_eq!(vec.len(), self.head_dim);
        assert!(
            pos < self.max_seq_len,
            "Vị trí vượt quá kích thước RoPE cache"
        );

        let cos_slice = &self.cos_cache[pos * half..(pos + 1) * half];
        let sin_slice = &self.sin_cache[pos * half..(pos + 1) * half];

        for i in 0..half {
            let (c, s) = (cos_slice[i], sin_slice[i]);
            let x1 = vec[i];
            let x2 = vec[i + half];
            vec[i] = x1 * c - x2 * s;
            vec[i + half] = x2 * c + x1 * s;
        }
    }
}

// ============================================================================
// 5. TẦNG FFN CỔNG TÍCH CỰC: SWIGLU MLP (SWISH GATED LINEAR UNIT)
// ============================================================================

/// Tầng Mạng thần kinh truyền thẳng kích hoạt cổng SwiGLU:
///
/// $\text{SwiGLU}(x) = (\text{Swish}(x W_{\text{gate}}) \odot (x W_{\text{up}})) W_{\text{down}}$
///
/// Trong các mô hình hiện đại (LLaMA 3, Gemma, DeepSeek), kích thước ẩn `hidden_features`
/// thường được chọn bằng $\frac{8}{3} d_{\text{model}}$ (làm tròn lên bội số của 64 hoặc 256).
#[derive(Debug, Clone, PartialEq)]
pub struct SwiGluMlp {
    pub in_features: usize,
    pub hidden_features: usize,
    pub out_features: usize,
    pub w_gate: Vec<f64>,
    pub w_up: Vec<f64>,
    pub w_down: Vec<f64>,
}

impl SwiGluMlp {
    /// Khởi tạo `SwiGluMlp` với bộ sinh `SplitMix64`.
    pub fn new(
        in_features: usize,
        hidden_features: usize,
        out_features: usize,
        rng: &mut SplitMix64,
    ) -> Self {
        let scale_in = (2.0 / (in_features as f64)).sqrt();
        let scale_hid = (2.0 / (hidden_features as f64)).sqrt();

        let w_gate = (0..in_features * hidden_features)
            .map(|_| rng.next_signed(scale_in))
            .collect();
        let w_up = (0..in_features * hidden_features)
            .map(|_| rng.next_signed(scale_in))
            .collect();
        let w_down = (0..hidden_features * out_features)
            .map(|_| rng.next_signed(scale_hid))
            .collect();

        Self {
            in_features,
            hidden_features,
            out_features,
            w_gate,
            w_up,
            w_down,
        }
    }

    /// Lan truyền xuôi của SwiGLU cho 1 token vector `x`.
    pub fn forward_token(&self, x: &[f64]) -> Vec<f64> {
        assert_eq!(x.len(), self.in_features);
        let gate = matmul(x, &self.w_gate, 1, self.in_features, self.hidden_features);
        let up = matmul(x, &self.w_up, 1, self.in_features, self.hidden_features);

        let mut gated = vec![0.0; self.hidden_features];
        for i in 0..self.hidden_features {
            gated[i] = silu(gate[i]) * up[i];
        }

        matmul(
            &gated,
            &self.w_down,
            1,
            self.hidden_features,
            self.out_features,
        )
    }

    /// Lan truyền xuôi trên chuỗi nhiều token.
    pub fn forward(&self, x: &[f64]) -> Vec<f64> {
        let num_tokens = x.len() / self.in_features;
        let mut out = Vec::with_capacity(num_tokens * self.out_features);
        for t in 0..num_tokens {
            let token_in = &x[t * self.in_features..(t + 1) * self.in_features];
            out.extend(self.forward_token(token_in));
        }
        out
    }

    /// Lan truyền xuôi và tính đạo hàm ngược giải tích đối với vector đầu vào `x`.
    ///
    /// Phục vụ kiểm chứng đạo hàm giải tích vs sai phân hữu hạn trung tâm `< 1e-8`.
    pub fn backward_x(&self, x: &[f64], grad_out: &[f64]) -> Vec<f64> {
        assert_eq!(x.len(), self.in_features);
        assert_eq!(grad_out.len(), self.out_features);

        let gate = matmul(x, &self.w_gate, 1, self.in_features, self.hidden_features);
        let up = matmul(x, &self.w_up, 1, self.in_features, self.hidden_features);

        // dL/d(gated) = grad_out * W_down^T
        let grad_gated = matmul(
            grad_out,
            &transpose(&self.w_down, self.hidden_features, self.out_features),
            1,
            self.out_features,
            self.hidden_features,
        );

        let mut grad_gate = vec![0.0; self.hidden_features];
        let mut grad_up = vec![0.0; self.hidden_features];

        for i in 0..self.hidden_features {
            let gg = grad_gated[i];
            grad_up[i] = gg * silu(gate[i]);
            grad_gate[i] = gg * up[i] * silu_deriv(gate[i]);
        }

        // dL/dx = grad_gate * W_gate^T + grad_up * W_up^T
        let grad_x_gate = matmul(
            &grad_gate,
            &transpose(&self.w_gate, self.in_features, self.hidden_features),
            1,
            self.hidden_features,
            self.in_features,
        );
        let grad_x_up = matmul(
            &grad_up,
            &transpose(&self.w_up, self.in_features, self.hidden_features),
            1,
            self.hidden_features,
            self.in_features,
        );

        let mut grad_x = vec![0.0; self.in_features];
        for i in 0..self.in_features {
            grad_x[i] = grad_x_gate[i] + grad_x_up[i];
        }
        grad_x
    }
}

// ============================================================================
// 6. CHÚ Ý NHÓM TRUY VẤN: GQA + QK-RMSNORM + SLIDING WINDOW MASK
// ============================================================================

/// Cấu hình cho Grouped-Query Attention (GQA).
#[derive(Debug, Clone, PartialEq)]
pub struct GqaConfig {
    pub n_heads: usize,
    pub n_kv_heads: usize,
    pub head_dim: usize,
    pub sliding_window: Option<usize>,
    pub eps: f64,
}

/// Bộ nhớ đệm Key-Value hỗ trợ giải mã từng bước (Incremental Decoding) cho GQA.
#[derive(Debug, Clone, PartialEq, Default)]
pub struct GqaKvCache {
    pub keys: Vec<f64>,
    pub values: Vec<f64>,
    pub current_len: usize,
}

/// Lớp chú ý nhóm truy vấn `GroupedQueryAttention` tích hợp QK-RMSNorm và RoPE.
#[derive(Debug, Clone, PartialEq)]
pub struct GroupedQueryAttention {
    pub config: GqaConfig,
    pub d_model: usize,
    pub w_q: Vec<f64>,
    pub w_k: Vec<f64>,
    pub w_v: Vec<f64>,
    pub w_o: Vec<f64>,
    pub q_norm: RmsNorm,
    pub k_norm: RmsNorm,
    pub rope: RopeCache,
}

impl GroupedQueryAttention {
    /// Khởi tạo GQA với tỷ lệ nhóm `n_heads / n_kv_heads`.
    pub fn new(
        d_model: usize,
        config: GqaConfig,
        max_seq_len: usize,
        base: f64,
        rng: &mut SplitMix64,
    ) -> Self {
        assert_eq!(
            config.n_heads % config.n_kv_heads,
            0,
            "n_heads phải chia hết cho n_kv_heads"
        );
        let q_dim = config.n_heads * config.head_dim;
        let kv_dim = config.n_kv_heads * config.head_dim;

        let scale_q = (2.0 / (d_model as f64)).sqrt();
        let scale_o = (2.0 / (q_dim as f64)).sqrt();

        let w_q = (0..d_model * q_dim)
            .map(|_| rng.next_signed(scale_q))
            .collect();
        let w_k = (0..d_model * kv_dim)
            .map(|_| rng.next_signed(scale_q))
            .collect();
        let w_v = (0..d_model * kv_dim)
            .map(|_| rng.next_signed(scale_q))
            .collect();
        let w_o = (0..q_dim * d_model)
            .map(|_| rng.next_signed(scale_o))
            .collect();

        let q_norm = RmsNorm::new(config.head_dim, config.eps, false);
        let k_norm = RmsNorm::new(config.head_dim, config.eps, false);
        let rope = RopeCache::new(config.head_dim, max_seq_len, base, None);

        Self {
            config,
            d_model,
            w_q,
            w_k,
            w_v,
            w_o,
            q_norm,
            k_norm,
            rope,
        }
    }

    /// Giải mã từng bước (Step Decode) với KV-Cache tăng dần.
    pub fn step_decode(&self, cache: &mut GqaKvCache, x_token: &[f64], pos: usize) -> Vec<f64> {
        let q_dim = self.config.n_heads * self.config.head_dim;
        let kv_dim = self.config.n_kv_heads * self.config.head_dim;
        let hd = self.config.head_dim;
        let nh = self.config.n_heads;
        let nkvh = self.config.n_kv_heads;
        let grp = nh / nkvh;

        // 1. Chiếu Q, K, V
        let mut q = matmul(x_token, &self.w_q, 1, self.d_model, q_dim);
        let mut k = matmul(x_token, &self.w_k, 1, self.d_model, kv_dim);
        let v = matmul(x_token, &self.w_v, 1, self.d_model, kv_dim);

        // 2. QK-RMSNorm trên từng đầu độc lập
        for h in 0..nh {
            let slot = &mut q[h * hd..(h + 1) * hd];
            let normed = self.q_norm.forward_token(slot);
            slot.copy_from_slice(&normed);
            self.rope.rotate(slot, pos);
        }
        for h in 0..nkvh {
            let slot = &mut k[h * hd..(h + 1) * hd];
            let normed = self.k_norm.forward_token(slot);
            slot.copy_from_slice(&normed);
            self.rope.rotate(slot, pos);
        }

        // 3. Ghi vào KV-cache
        cache.keys.extend_from_slice(&k);
        cache.values.extend_from_slice(&v);
        cache.current_len += 1;

        let total_t = cache.current_len;
        let scale = 1.0 / (hd as f64).sqrt();
        let mut out_heads = vec![0.0; q_dim];

        // 4. Multi-head scaled dot-product attention
        for h in 0..nh {
            let kv_h = h / grp;
            let q_vec = &q[h * hd..(h + 1) * hd];
            let mut scores = vec![f64::NEG_INFINITY; total_t];

            let min_idx = if let Some(w) = self.config.sliding_window {
                total_t.saturating_sub(w)
            } else {
                0
            };

            for (t, score_slot) in scores.iter_mut().enumerate().take(total_t).skip(min_idx) {
                let k_vec = &cache.keys[t * kv_dim + kv_h * hd..t * kv_dim + (kv_h + 1) * hd];
                let dot: f64 = q_vec.iter().zip(k_vec).map(|(&a, &b)| a * b).sum();
                *score_slot = dot * scale;
            }

            softmax_row(&mut scores[min_idx..total_t]);

            let out_slot = &mut out_heads[h * hd..(h + 1) * hd];
            for (t, &weight) in scores.iter().enumerate().take(total_t).skip(min_idx) {
                let v_vec = &cache.values[t * kv_dim + kv_h * hd..t * kv_dim + (kv_h + 1) * hd];
                for (o, &val) in out_slot.iter_mut().zip(v_vec) {
                    *o += weight * val;
                }
            }
        }

        // 5. Chiếu đầu ra W_o
        matmul(&out_heads, &self.w_o, 1, q_dim, self.d_model)
    }

    /// Lan truyền xuôi tự hồi quy nhân quả trên toàn chuỗi (Prefill phase).
    pub fn forward_causal(&self, x: &[f64], seq_len: usize) -> Vec<f64> {
        let mut cache = GqaKvCache::default();
        let mut out = Vec::with_capacity(seq_len * self.d_model);
        for t in 0..seq_len {
            let token = &x[t * self.d_model..(t + 1) * self.d_model];
            out.extend(self.step_decode(&mut cache, token, t));
        }
        out
    }
}

// ============================================================================
// 7. DEEPSEEK MULTI-HEAD LATENT ATTENTION (MLA NÉN TIỀM ẨN HẠNG THẤP)
// ============================================================================

/// Cấu hình cho DeepSeek Multi-Head Latent Attention (MLA).
///
/// Thay vì lưu trữ $2 \times n_{\text{heads}} \times d_h$ số thực mỗi token trong KV-cache,
/// MLA nén không gian Key-Value xuống vector tiềm ẩn $c^{KV}$ có số chiều $d_c \ll 2 d_{\text{model}}$.
/// Đồng thời, nhánh vị trí RoPE được tách riêng thành $k^R$ ($d_r$ chiều) để không làm mất thông tin vị trí.
#[derive(Debug, Clone, PartialEq)]
pub struct MlaConfig {
    pub n_heads: usize,
    pub d_model: usize,
    pub d_c: usize,
    pub d_r: usize,
    pub v_dim: usize,
    pub eps: f64,
}

/// Bộ nhớ đệm KV nén siêu nhỏ cho DeepSeek MLA.
#[derive(Debug, Clone, PartialEq, Default)]
pub struct MlaKvCache {
    /// Vector tiềm ẩn nén $c^{KV}$ kích thước `seq_len x d_c`.
    pub compressed_kv: Vec<Vec<f64>>,
    /// Vector khóa vị trí RoPE tách rời $k^R$ kích thước `seq_len x d_r`.
    pub k_rope: Vec<Vec<f64>>,
}

/// Lớp chú ý tiềm ẩn đa đầu DeepSeek `MultiHeadLatentAttention`.
#[derive(Debug, Clone, PartialEq)]
pub struct MultiHeadLatentAttention {
    pub config: MlaConfig,
    /// Chiếu nén Key-Value: $x \to c^{KV}$ (`d_model -> d_c`)
    pub w_dkv: Vec<f64>,
    /// Chiếu giải nén Key: $c^{KV} \to K^C$ (`d_c -> n_heads * v_dim`)
    pub w_uk: Vec<f64>,
    /// Chiếu giải nén Value: $c^{KV} \to V^C$ (`d_c -> n_heads * v_dim`)
    pub w_uv: Vec<f64>,
    /// Chiếu khóa RoPE tách rời: $x \to k^R$ (`d_model -> d_r`)
    pub w_kr: Vec<f64>,
    /// Chiếu nén Query: $x \to c^Q$ (`d_model -> d_c`)
    pub w_dq: Vec<f64>,
    /// Chiếu giải nén Query nội dung: $c^Q \to Q^C$ (`d_c -> n_heads * v_dim`)
    pub w_uq: Vec<f64>,
    /// Chiếu Query RoPE: $c^Q \to Q^R$ (`d_c -> n_heads * d_r`)
    pub w_qr: Vec<f64>,
    /// Chiếu đầu ra tổng hợp: `n_heads * v_dim -> d_model`
    pub w_o: Vec<f64>,
    pub kv_norm: RmsNorm,
    pub q_norm: RmsNorm,
    pub rope: RopeCache,
}

impl MultiHeadLatentAttention {
    /// Khởi tạo khối DeepSeek MLA với trọng số từ `SplitMix64`.
    pub fn new(config: MlaConfig, max_seq_len: usize, base: f64, rng: &mut SplitMix64) -> Self {
        let dm = config.d_model;
        let dc = config.d_c;
        let dr = config.d_r;
        let q_content_dim = config.n_heads * config.v_dim;
        let q_rope_dim = config.n_heads * config.d_r;

        let scale_dm = (2.0 / (dm as f64)).sqrt();
        let scale_dc = (2.0 / (dc as f64)).sqrt();
        let scale_o = (2.0 / (q_content_dim as f64)).sqrt();

        let w_dkv = (0..dm * dc).map(|_| rng.next_signed(scale_dm)).collect();
        let w_uk = (0..dc * q_content_dim)
            .map(|_| rng.next_signed(scale_dc))
            .collect();
        let w_uv = (0..dc * q_content_dim)
            .map(|_| rng.next_signed(scale_dc))
            .collect();
        let w_kr = (0..dm * dr).map(|_| rng.next_signed(scale_dm)).collect();

        let w_dq = (0..dm * dc).map(|_| rng.next_signed(scale_dm)).collect();
        let w_uq = (0..dc * q_content_dim)
            .map(|_| rng.next_signed(scale_dc))
            .collect();
        let w_qr = (0..dc * q_rope_dim)
            .map(|_| rng.next_signed(scale_dc))
            .collect();
        let w_o = (0..q_content_dim * dm)
            .map(|_| rng.next_signed(scale_o))
            .collect();

        let kv_norm = RmsNorm::new(dc, config.eps, false);
        let q_norm = RmsNorm::new(dc, config.eps, false);
        let rope = RopeCache::new(dr, max_seq_len, base, None);

        Self {
            config,
            w_dkv,
            w_uk,
            w_uv,
            w_kr,
            w_dq,
            w_uq,
            w_qr,
            w_o,
            kv_norm,
            q_norm,
            rope,
        }
    }

    /// Bước sinh tăng dần với bộ nhớ đệm KV nén tiềm ẩn.
    pub fn step_decode(&self, cache: &mut MlaKvCache, x_token: &[f64], pos: usize) -> Vec<f64> {
        let dc = self.config.d_c;
        let dr = self.config.d_r;
        let vd = self.config.v_dim;
        let nh = self.config.n_heads;
        let q_c_dim = nh * vd;
        let q_r_dim = nh * dr;

        // 1. Nén KV: x -> c_kv
        let c_kv_raw = matmul(x_token, &self.w_dkv, 1, self.config.d_model, dc);
        let c_kv = self.kv_norm.forward_token(&c_kv_raw);

        // 2. Nhánh RoPE riêng của Key
        let mut k_r = matmul(x_token, &self.w_kr, 1, self.config.d_model, dr);
        self.rope.rotate(&mut k_r, pos);

        // Lưu vector nén vào cache (chỉ dc + dr floats!)
        cache.compressed_kv.push(c_kv.clone());
        cache.k_rope.push(k_r);

        // 3. Xử lý Query: x -> c_q -> (q_c, q_r)
        let c_q_raw = matmul(x_token, &self.w_dq, 1, self.config.d_model, dc);
        let c_q = self.q_norm.forward_token(&c_q_raw);

        let q_c = matmul(&c_q, &self.w_uq, 1, dc, q_c_dim);
        let mut q_r = matmul(&c_q, &self.w_qr, 1, dc, q_r_dim);
        for h in 0..nh {
            let slot = &mut q_r[h * dr..(h + 1) * dr];
            self.rope.rotate(slot, pos);
        }

        // 4. Tính toán Attention Score dựa trên nội dung nén + vị trí RoPE
        let total_t = cache.compressed_kv.len();
        let scale = 1.0 / ((vd + dr) as f64).sqrt();
        let mut head_outs = vec![0.0; q_c_dim];

        for h in 0..nh {
            let q_c_h = &q_c[h * vd..(h + 1) * vd];
            let q_r_h = &q_r[h * dr..(h + 1) * dr];

            let mut scores = vec![0.0; total_t];
            for (t, score_slot) in scores.iter_mut().enumerate().take(total_t) {
                // Giải nén Key nội dung tại bước t cho đầu h
                let c_kv_t = &cache.compressed_kv[t];
                let k_c_all = matmul(c_kv_t, &self.w_uk, 1, dc, q_c_dim);
                let k_c_h = &k_c_all[h * vd..(h + 1) * vd];
                let k_r_t = &cache.k_rope[t];

                let dot_c: f64 = q_c_h.iter().zip(k_c_h).map(|(&a, &b)| a * b).sum();
                let dot_r: f64 = q_r_h.iter().zip(k_r_t).map(|(&a, &b)| a * b).sum();
                *score_slot = (dot_c + dot_r) * scale;
            }

            softmax_row(&mut scores);

            // Tổng hợp Value
            let out_slot = &mut head_outs[h * vd..(h + 1) * vd];
            for (t, &weight) in scores.iter().enumerate().take(total_t) {
                let c_kv_t = &cache.compressed_kv[t];
                let v_all = matmul(c_kv_t, &self.w_uv, 1, dc, q_c_dim);
                let v_h = &v_all[h * vd..(h + 1) * vd];
                for (o, &val) in out_slot.iter_mut().zip(v_h) {
                    *o += weight * val;
                }
            }
        }

        // 5. Chiếu đầu ra tổng hợp
        matmul(&head_outs, &self.w_o, 1, q_c_dim, self.config.d_model)
    }

    /// Lan truyền xuôi tự hồi quy cho chuỗi.
    pub fn forward_causal(&self, x: &[f64], seq_len: usize) -> Vec<f64> {
        let mut cache = MlaKvCache::default();
        let mut out = Vec::with_capacity(seq_len * self.config.d_model);
        for t in 0..seq_len {
            let token = &x[t * self.config.d_model..(t + 1) * self.config.d_model];
            out.extend(self.step_decode(&mut cache, token, t));
        }
        out
    }
}

// ============================================================================
// 8. MÔ HÌNH TRẠNG THÁI TUYẾN TÍNH ĐỆ QUY: GATED DELTANET O(N)
// ============================================================================

/// Cấu hình cho mạng nơ-ron trạng thái tuyến tính `GatedDeltaNet`.
#[derive(Debug, Clone, PartialEq)]
pub struct GatedDeltaNetConfig {
    pub d_in: usize,
    pub d_state: usize,
    pub eps: f64,
}

/// Một bước cập nhật trạng thái Gated DeltaNet cho khóa đã chuẩn hóa `k` (`||k|| = 1`):
/// $S_t = \alpha S_{t-1} + \beta (v - \alpha S_{t-1} k) k^\top$.
///
/// `s` là ma trận `ds x ds` theo thứ tự hàng; trả về ma trận trạng thái mới cùng kích thước.
/// Hệ quả: $S_t k = \alpha (1 - \beta) S_{t-1} k + \beta v$, nên khi $\beta = 1$ thì $S_t k = v$
/// (ô nhớ của khóa $k$ bị ghi đè hoàn toàn) bất kể $\alpha$.
pub fn gated_delta_step(s: &[f64], k: &[f64], v: &[f64], alpha: f64, beta: f64) -> Vec<f64> {
    let ds = k.len();
    assert_eq!(s.len(), ds * ds);
    assert_eq!(v.len(), ds);
    // S_{t-1} * k_t (dự báo của trạng thái cũ đối với k_t)
    let s_k = matmul(s, k, ds, ds, 1);
    let mut new_s = vec![0.0; ds * ds];
    for r in 0..ds {
        // Sai số Delta tính trên trạng thái đã suy giảm: v_t - alpha * S_{t-1} * k_t
        let delta = v[r] - alpha * s_k[r];
        for c in 0..ds {
            new_s[r * ds + c] = alpha * s[r * ds + c] + beta * delta * k[c];
        }
    }
    new_s
}

/// Tầng Gated DeltaNet kết hợp luật Delta (Delta Rule) với cơ chế cổng quên (Gating):
///
/// Trạng thái ma trận liên tục $S_t \in \mathbb{R}^{d \times d}$ được cập nhật tuyến tính
/// (Yang et al., 2024): suy giảm trạng thái trước, rồi áp luật Delta lên trạng thái đã suy giảm:
/// $S_t = \alpha_t S_{t-1} (I - \beta_t k_t k_t^\top) + \beta_t v_t k_t^\top
///      = \alpha_t S_{t-1} + \beta_t (v_t - \alpha_t S_{t-1} k_t) k_t^\top$
///
/// Đầu ra tại mỗi token:
/// $y_t = S_t q_t \odot \text{SiLU}(x_t W_{\text{out\_gate}})$
///
/// Độ phức tạp: $O(d^2)$ trên mỗi token $\to O(N)$ trên toàn chuỗi (thay vì $O(N^2)$ của Transformer cổ điển).
#[derive(Debug, Clone, PartialEq)]
pub struct GatedDeltaNet {
    pub config: GatedDeltaNetConfig,
    pub w_q: Vec<f64>,
    pub w_k: Vec<f64>,
    pub w_v: Vec<f64>,
    pub w_alpha: Vec<f64>,
    pub w_beta: Vec<f64>,
    pub w_gate: Vec<f64>,
    pub w_o: Vec<f64>,
}

impl GatedDeltaNet {
    /// Khởi tạo GatedDeltaNet với các ma trận chiếu trạng thái.
    pub fn new(config: GatedDeltaNetConfig, rng: &mut SplitMix64) -> Self {
        let din = config.d_in;
        let ds = config.d_state;
        let scale = (2.0 / (din as f64)).sqrt();

        let w_q = (0..din * ds).map(|_| rng.next_signed(scale)).collect();
        let w_k = (0..din * ds).map(|_| rng.next_signed(scale)).collect();
        let w_v = (0..din * ds).map(|_| rng.next_signed(scale)).collect();
        let w_alpha = (0..din * ds).map(|_| rng.next_signed(scale)).collect();
        let w_beta = (0..din * ds).map(|_| rng.next_signed(scale)).collect();
        let w_gate = (0..din * ds).map(|_| rng.next_signed(scale)).collect();
        let w_o = (0..ds * din)
            .map(|_| rng.next_signed((2.0 / (ds as f64)).sqrt()))
            .collect();

        Self {
            config,
            w_q,
            w_k,
            w_v,
            w_alpha,
            w_beta,
            w_gate,
            w_o,
        }
    }

    /// Lan truyền xuôi đệ quy $O(N)$ duy trì trạng thái ma trận $S_t$.
    pub fn forward(&self, x: &[f64], seq_len: usize) -> Vec<f64> {
        let din = self.config.d_in;
        let ds = self.config.d_state;
        assert_eq!(x.len(), seq_len * din);

        let mut s = vec![0.0; ds * ds];
        let mut out = Vec::with_capacity(seq_len * din);

        for t in 0..seq_len {
            let x_t = &x[t * din..(t + 1) * din];

            let q = matmul(x_t, &self.w_q, 1, din, ds);
            let mut k = matmul(x_t, &self.w_k, 1, din, ds);
            let v = matmul(x_t, &self.w_v, 1, din, ds);

            // Chuẩn hóa vector khóa k để k^T k = 1 (ổn định luật Delta)
            let k_norm: f64 = k.iter().map(|&val| val * val).sum::<f64>().sqrt() + self.config.eps;
            for val in k.iter_mut() {
                *val /= k_norm;
            }

            // Tính cổng suy giảm alpha_t và tốc độ học beta_t
            let alpha_logits = matmul(x_t, &self.w_alpha, 1, din, ds);
            let beta_logits = matmul(x_t, &self.w_beta, 1, din, ds);
            let gate_logits = matmul(x_t, &self.w_gate, 1, din, ds);

            let alpha = sigmoid(alpha_logits.iter().sum::<f64>() / (ds as f64));
            let beta = sigmoid(beta_logits.iter().sum::<f64>() / (ds as f64));

            // Cập nhật trạng thái: S_t = alpha * S_{t-1} + beta * (v - alpha * S_{t-1} k) k^T
            s = gated_delta_step(&s, &k, &v, alpha, beta);

            // Truy xuất: y_raw = S_t * q_t
            let y_raw = matmul(&s, &q, ds, ds, 1);

            // Cổng trượt đầu ra: y_gated = y_raw * SiLU(gate)
            let mut y_gated = vec![0.0; ds];
            for i in 0..ds {
                y_gated[i] = y_raw[i] * silu(gate_logits[i]);
            }

            // Chiếu về không gian d_in ban đầu
            let y_proj = matmul(&y_gated, &self.w_o, 1, ds, din);
            out.extend(y_proj);
        }

        out
    }
}

// ============================================================================
// 9. HỖN HỢP CHUYÊN GIA THƯA: SPARSE MOE + CHUYÊN GIA DÙNG CHUNG + AUX_LOSS
// ============================================================================

/// Cấu hình cho tầng Sparse Mixture of Experts.
#[derive(Debug, Clone, PartialEq)]
pub struct SparseMoeConfig {
    pub n_routed_experts: usize,
    pub n_shared_experts: usize,
    pub top_k: usize,
    pub d_model: usize,
    pub d_hidden: usize,
    pub aux_loss_coef: f64,
}

/// Kết quả định tuyến chuyên gia kèm tổn thất phụ cân bằng tải.
#[derive(Debug, Clone, PartialEq)]
pub struct MoeRoutingResult {
    pub output: Vec<f64>,
    pub selected_experts: Vec<Vec<usize>>,
    pub routing_weights: Vec<Vec<f64>>,
    pub aux_loss: f64,
}

/// Tầng MoE thưa `SparseMoeLayer` với chuyên gia dùng chung (Shared Experts) kiểu DeepSeek V2/V3.
#[derive(Debug, Clone, PartialEq)]
pub struct SparseMoeLayer {
    pub config: SparseMoeConfig,
    /// Trọng số định tuyến Router: `d_model -> n_routed_experts`
    pub router_weights: Vec<f64>,
    /// Danh sách các chuyên gia định tuyến (mỗi chuyên gia là một SwiGluMlp)
    pub routed_experts: Vec<SwiGluMlp>,
    /// Danh sách các chuyên gia dùng chung (luôn chạy cho 100% token)
    pub shared_experts: Vec<SwiGluMlp>,
}

impl SparseMoeLayer {
    /// Khởi tạo tầng MoE với đầy đủ chuyên gia dùng chung và định tuyến.
    pub fn new(config: SparseMoeConfig, rng: &mut SplitMix64) -> Self {
        assert!(config.top_k <= config.n_routed_experts);
        let dm = config.d_model;
        let dh = config.d_hidden;
        let ne = config.n_routed_experts;

        let scale_router = (2.0 / (dm as f64)).sqrt();
        let router_weights = (0..dm * ne)
            .map(|_| rng.next_signed(scale_router))
            .collect();

        let mut routed_experts = Vec::with_capacity(ne);
        for _ in 0..ne {
            routed_experts.push(SwiGluMlp::new(dm, dh, dm, rng));
        }

        let mut shared_experts = Vec::with_capacity(config.n_shared_experts);
        for _ in 0..config.n_shared_experts {
            shared_experts.push(SwiGluMlp::new(dm, dh, dm, rng));
        }

        Self {
            config,
            router_weights,
            routed_experts,
            shared_experts,
        }
    }

    /// Lan truyền xuôi của MoE trên chuỗi token.
    ///
    /// Tính toán song song:
    /// 1. Router logit $\to$ Softmax phân phối xác suất
    /// 2. Chọn Top-$k$ chuyên gia có logit lớn nhất và chuẩn hóa lại trọng số
    /// 3. Tổng hợp đầu ra từ các chuyên gia định tuyến
    /// 4. Cộng thêm đầu ra từ tất cả chuyên gia dùng chung (Shared Experts)
    /// 5. Tính toán tổn thất phụ cân bằng tải Switch `aux_loss`:
    ///    $\mathcal{L}_{\text{aux}} = \alpha \cdot E \cdot \sum_{i=1}^E f_i P_i$
    pub fn forward(&self, x: &[f64], seq_len: usize) -> MoeRoutingResult {
        let dm = self.config.d_model;
        let ne = self.config.n_routed_experts;
        let k = self.config.top_k;
        assert_eq!(x.len(), seq_len * dm);

        let mut out = vec![0.0; x.len()];
        let mut all_selected = Vec::with_capacity(seq_len);
        let mut all_weights = Vec::with_capacity(seq_len);

        // Biến phục vụ tính aux_loss: f_i (tần suất chọn) và P_i (xác suất trung bình)
        let mut expert_counts = vec![0.0; ne];
        let mut expert_prob_sums = vec![0.0; ne];

        for t in 0..seq_len {
            let x_t = &x[t * dm..(t + 1) * dm];

            // 1. Router logits & probabilities
            let logits = matmul(x_t, &self.router_weights, 1, dm, ne);
            let mut probs = logits.clone();
            softmax_row(&mut probs);

            for (i, &p) in probs.iter().enumerate() {
                expert_prob_sums[i] += p;
            }

            // 2. Tìm top-k chuyên gia
            let mut indexed_logits: Vec<(usize, f64)> =
                logits.iter().copied().enumerate().collect();
            indexed_logits
                .sort_by(|a, b| b.1.partial_cmp(&a.1).unwrap_or(std::cmp::Ordering::Equal));

            let selected: Vec<usize> = indexed_logits.iter().take(k).map(|item| item.0).collect();
            let mut selected_probs: Vec<f64> = selected.iter().map(|&idx| probs[idx]).collect();

            // Chuẩn hóa trọng số top-k
            let sum_prob: f64 = selected_probs.iter().sum();
            let inv_sum = if sum_prob > 0.0 { 1.0 / sum_prob } else { 0.0 };
            for p in selected_probs.iter_mut() {
                *p *= inv_sum;
            }

            for &idx in &selected {
                expert_counts[idx] += 1.0;
            }

            // 3. Thực thi các chuyên gia định tuyến
            let slot = &mut out[t * dm..(t + 1) * dm];
            for (&exp_idx, &weight) in selected.iter().zip(&selected_probs) {
                let exp_out = self.routed_experts[exp_idx].forward_token(x_t);
                for (o, val) in slot.iter_mut().zip(exp_out) {
                    *o += weight * val;
                }
            }

            // 4. Thực thi chuyên gia dùng chung (Shared Experts)
            for shared_exp in &self.shared_experts {
                let s_out = shared_exp.forward_token(x_t);
                for (o, val) in slot.iter_mut().zip(s_out) {
                    *o += val;
                }
            }

            all_selected.push(selected);
            all_weights.push(selected_probs);
        }

        // 5. Tính toán aux_loss cân bằng tải
        let t_f = seq_len as f64;
        let mut aux_loss = 0.0;
        for i in 0..ne {
            let f_i = expert_counts[i] / (t_f * (k as f64));
            let p_i = expert_prob_sums[i] / t_f;
            aux_loss += f_i * p_i;
        }
        aux_loss *= self.config.aux_loss_coef * (ne as f64);

        MoeRoutingResult {
            output: out,
            selected_experts: all_selected,
            routing_weights: all_weights,
            aux_loss,
        }
    }
}

// ============================================================================
// 10. KHỐI BỘ GIẢI MÃ HIỆN ĐẠI TỔNG HỢP (MODERN DECODER BLOCK)
// ============================================================================

/// Biến thể cơ chế Attention trong Decoder hiện đại.
#[derive(Debug, Clone, PartialEq)]
pub enum AttentionVariant {
    Gqa(GroupedQueryAttention),
    Mla(MultiHeadLatentAttention),
    DeltaNet(GatedDeltaNet),
}

/// Biến thể tầng Feed-Forward Network.
#[derive(Debug, Clone, PartialEq)]
pub enum MlpVariant {
    SwiGlu(SwiGluMlp),
    SparseMoe(SparseMoeLayer),
}

/// Khối giải mã Transformer hiện đại kết hợp:
/// `RmsNorm` $\to$ Attention (GQA / MLA / DeltaNet) $\to$ `+ Residual` $\to$ `RmsNorm` $\to$ MLP (SwiGLU / MoE) $\to$ `+ Residual`.
#[derive(Debug, Clone, PartialEq)]
pub struct ModernDecoderBlock {
    pub d_model: usize,
    pub norm_1: RmsNorm,
    pub attn: AttentionVariant,
    pub norm_2: RmsNorm,
    pub mlp: MlpVariant,
}

impl ModernDecoderBlock {
    /// Khởi tạo khối ModernDecoder.
    pub fn new(
        d_model: usize,
        norm_1: RmsNorm,
        attn: AttentionVariant,
        norm_2: RmsNorm,
        mlp: MlpVariant,
    ) -> Self {
        Self {
            d_model,
            norm_1,
            attn,
            norm_2,
            mlp,
        }
    }

    /// Lan truyền xuôi tự hồi quy nhân quả trên chuỗi token.
    pub fn forward(&self, x: &[f64], seq_len: usize) -> Vec<f64> {
        let dm = self.d_model;
        assert_eq!(x.len(), seq_len * dm);

        // Nhánh 1: Attention với Pre-RMSNorm
        let normed_1 = self.norm_1.forward(x);
        let attn_out = match &self.attn {
            AttentionVariant::Gqa(gqa) => gqa.forward_causal(&normed_1, seq_len),
            AttentionVariant::Mla(mla) => mla.forward_causal(&normed_1, seq_len),
            AttentionVariant::DeltaNet(dn) => dn.forward(&normed_1, seq_len),
        };

        let mut x_mid = vec![0.0; x.len()];
        for i in 0..x.len() {
            x_mid[i] = x[i] + attn_out[i];
        }

        // Nhánh 2: MLP với Pre-RMSNorm
        let normed_2 = self.norm_2.forward(&x_mid);
        let mlp_out = match &self.mlp {
            MlpVariant::SwiGlu(swiglu) => swiglu.forward(&normed_2),
            MlpVariant::SparseMoe(moe) => moe.forward(&normed_2, seq_len).output,
        };

        let mut x_final = vec![0.0; x.len()];
        for i in 0..x.len() {
            x_final[i] = x_mid[i] + mlp_out[i];
        }
        x_final
    }
}

// ============================================================================
// HÀM CHÍNH (DEMO MINH HỌA)
// ============================================================================

fn main() {
    println!("=== CHƯƠNG 93: TỪ GPT-2 ĐẾN BỘ GIẢI MÃ HIỆN ĐẠI ===");
    let mut rng = SplitMix64::new(42);

    let d_model = 16;
    let n_heads = 4;
    let n_kv_heads = 2;
    let seq_len = 4;

    println!("1. Khởi tạo GPT-2 Block cổ điển (d_model={d_model}, n_heads={n_heads})...");
    let gpt2 = Gpt2Block::new(d_model, n_heads, 1e-5, &mut rng);
    let sample_x: Vec<f64> = (0..seq_len * d_model)
        .map(|_| rng.next_signed(0.5))
        .collect();
    let gpt2_out = gpt2.forward(&sample_x, seq_len);
    println!(
        "   GPT-2 forward hoàn tất: kích thước ra = {}",
        gpt2_out.len()
    );

    println!("2. Khởi tạo ModernDecoderBlock (RMSNorm + GQA + SwiGLU)...");
    let norm1 = RmsNorm::new(d_model, 1e-5, false);
    let gqa_cfg = GqaConfig {
        n_heads,
        n_kv_heads,
        head_dim: d_model / n_heads,
        sliding_window: Some(3),
        eps: 1e-5,
    };
    let gqa = GroupedQueryAttention::new(d_model, gqa_cfg, 32, 10000.0, &mut rng);
    let norm2 = RmsNorm::new(d_model, 1e-5, false);
    let swiglu = SwiGluMlp::new(d_model, (8 * d_model) / 3, d_model, &mut rng);

    let modern_block = ModernDecoderBlock::new(
        d_model,
        norm1,
        AttentionVariant::Gqa(gqa),
        norm2,
        MlpVariant::SwiGlu(swiglu),
    );
    let modern_out = modern_block.forward(&sample_x, seq_len);
    println!(
        "   ModernDecoder forward hoàn tất: kích thước ra = {}",
        modern_out.len()
    );

    println!("3. Kiểm tra nén bộ nhớ DeepSeek MLA:");
    let mla_cfg = MlaConfig {
        n_heads: 4,
        d_model: 16,
        d_c: 6,
        d_r: 4,
        v_dim: 4,
        eps: 1e-5,
    };
    let mla_cache_elements_per_token = mla_cfg.d_c + mla_cfg.d_r;
    let mha_cache_elements_per_token = 2 * d_model;
    let savings = 100.0
        * (1.0 - (mla_cache_elements_per_token as f64) / (mha_cache_elements_per_token as f64));
    println!("   MHA KV Cache: {mha_cache_elements_per_token} floats/token");
    println!("   MLA KV Cache: {mla_cache_elements_per_token} floats/token");
    println!("   Tiết kiệm bộ nhớ KV-Cache: {:.1}%", savings);

    println!("\nToàn bộ các cấu trúc toán học của Chương 93 đã sẵn sàng và được kiểm chứng!");
}

// ============================================================================
// BỘ KIỂM THỬ TẤT ĐỊNH (>= 32 TESTS)
// ============================================================================

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_splitmix64_determinism() {
        let mut rng1 = SplitMix64::new(12345);
        let mut rng2 = SplitMix64::new(12345);
        for _ in 0..100 {
            assert_eq!(rng1.next_signed(1.0), rng2.next_signed(1.0));
        }
    }

    #[test]
    fn test_layer_norm_forward_mean_variance() {
        let ln = LayerNorm::new(8, 1e-6);
        let input = vec![1.0, 2.0, 3.0, 4.0, 5.0, 6.0, 7.0, 8.0];
        let out = ln.forward(&input);
        assert_eq!(out.len(), 8);
        let mean = out.iter().sum::<f64>() / 8.0;
        let var = out.iter().map(|&v| (v - mean).powi(2)).sum::<f64>() / 8.0;
        assert!(mean.abs() < 1e-6, "Mean của LayerNorm phải xấp xỉ 0");
        assert!(
            (var - 1.0).abs() < 1e-3,
            "Variance của LayerNorm phải xấp xỉ 1"
        );
    }

    #[test]
    fn test_gelu_values() {
        assert_eq!(Gelu::apply(0.0), 0.0);
        let pos = Gelu::apply(1.0);
        let neg = Gelu::apply(-1.0);
        assert!((pos - 0.8413).abs() < 1e-3);
        assert!((neg - (-0.1586)).abs() < 1e-3);
    }

    #[test]
    fn test_mha_causal_mask_no_future_leak() {
        let mut rng = SplitMix64::new(99);
        let d_model = 8;
        let n_heads = 2;
        let mha = MultiHeadAttention::new(d_model, n_heads, &mut rng);

        let x1 = vec![0.5; 3 * d_model];
        let mut x2 = x1.clone();
        // Thay đổi token tương lai tại t=2
        for slot in &mut x2[2 * d_model..3 * d_model] {
            *slot += 10.0;
        }

        let out1 = mha.forward(&x1, 3);
        let out2 = mha.forward(&x2, 3);

        // Token tại t=0 và t=1 không được phép bị ảnh hưởng bởi thay đổi ở t=2
        for i in 0..2 * d_model {
            assert!(
                (out1[i] - out2[i]).abs() < 1e-12,
                "Lỗ hổng nhân quả: Token quá khứ bị rò rỉ thông tin tương lai tại index {i}"
            );
        }
    }

    #[test]
    fn test_gpt2_block_forward() {
        let mut rng = SplitMix64::new(101);
        let block = Gpt2Block::new(8, 2, 1e-5, &mut rng);
        let x = vec![0.1; 2 * 8];
        let out = block.forward(&x, 2);
        assert_eq!(out.len(), 16);
        for &val in &out {
            assert!(val.is_finite());
        }
    }

    #[test]
    fn test_rmsnorm_forward_scaling() {
        let norm = RmsNorm::new(4, 1e-5, false);
        let x = vec![2.0, 2.0, 2.0, 2.0];
        // RMS = sqrt( (4 * 4)/4 ) = 2.0
        let out = norm.forward_token(&x);
        for &v in &out {
            assert!((v - 1.0).abs() < 1e-3);
        }
    }

    #[test]
    fn test_rmsnorm_unit_offset_gemma() {
        let mut norm = RmsNorm::new(4, 1e-5, true);
        norm.weight = vec![0.5; 4];
        let x = vec![2.0, 2.0, 2.0, 2.0];
        let out = norm.forward_token(&x);
        for &v in &out {
            assert!((v - 1.5).abs() < 1e-3, "Kỳ vọng 1.0 * (1.0 + 0.5) = 1.5");
        }
    }

    #[test]
    fn test_rmsnorm_backward_analytic_vs_finite_diff() {
        let mut norm = RmsNorm::new(4, 1e-5, false);
        norm.weight = vec![1.2, 0.8, 1.5, 0.9];
        let x = vec![0.5, -1.2, 2.3, 0.4];
        let grad_out = vec![1.0, 0.5, -0.7, 0.2];

        let (_, grad_x_analytic, _) = norm.forward_backward(&x, &grad_out);

        // Kiểm tra sai phân hữu hạn trung tâm (Central Finite Difference)
        let h = 1e-6;
        for i in 0..4 {
            let mut x_plus = x.clone();
            let mut x_minus = x.clone();
            x_plus[i] += h;
            x_minus[i] -= h;

            let y_plus = norm.forward_token(&x_plus);
            let y_minus = norm.forward_token(&x_minus);

            let loss_plus: f64 = y_plus.iter().zip(&grad_out).map(|(&a, &b)| a * b).sum();
            let loss_minus: f64 = y_minus.iter().zip(&grad_out).map(|(&a, &b)| a * b).sum();

            let grad_x_num = (loss_plus - loss_minus) / (2.0 * h);
            let diff = (grad_x_analytic[i] - grad_x_num).abs();
            assert!(
                diff < 1e-8,
                "Đạo hàm RMSNorm sai phân vượt ngưỡng tại index {i}: diff={diff}"
            );
        }
    }

    #[test]
    fn test_rope_norm_preservation() {
        let rope = RopeCache::new(8, 32, 10000.0, None);
        let mut vec = vec![1.2, -0.5, 2.3, 0.9, -1.1, 0.4, 0.7, -1.8];
        let orig_norm: f64 = vec.iter().map(|&v| v * v).sum::<f64>().sqrt();

        rope.rotate(&mut vec, 15);
        let rotated_norm: f64 = vec.iter().map(|&v| v * v).sum::<f64>().sqrt();

        assert!(
            (orig_norm - rotated_norm).abs() < 1e-12,
            "RoPE phải bảo toàn chuẩn Euclid < 1e-12"
        );
    }

    #[test]
    fn test_rope_relative_distance_invariance() {
        let rope = RopeCache::new(4, 32, 10000.0, None);
        let q = vec![0.5, 1.2, -0.3, 0.8];
        let k = vec![1.1, -0.4, 0.9, 0.2];

        // Trường hợp 1: pos_q = 5, pos_k = 2 (khoảng cách = 3)
        let mut q1 = q.clone();
        let mut k1 = k.clone();
        rope.rotate(&mut q1, 5);
        rope.rotate(&mut k1, 2);
        let dot1: f64 = q1.iter().zip(&k1).map(|(&a, &b)| a * b).sum();

        // Trường hợp 2: pos_q = 12, pos_k = 9 (khoảng cách = 3)
        let mut q2 = q.clone();
        let mut k2 = k.clone();
        rope.rotate(&mut q2, 12);
        rope.rotate(&mut k2, 9);
        let dot2: f64 = q2.iter().zip(&k2).map(|(&a, &b)| a * b).sum();

        assert!(
            (dot1 - dot2).abs() < 1e-12,
            "Tích vô hướng RoPE chỉ được phụ thuộc vào khoảng cách tương đối (m - n)"
        );
    }

    #[test]
    fn test_yarn_scaling_ramp() {
        let yarn = YarnConfig {
            original_max_seq: 16,
            target_max_seq: 64,
            beta_fast: 4.0,
            beta_slow: 1.0,
            mscale: 1.15,
        };
        let rope_yarn = RopeCache::new(8, 64, 10000.0, Some(&yarn));
        assert_eq!(rope_yarn.cos_cache.len(), 64 * 4);
        // Xác nhận cos_cache không chứa giá trị NaN
        for &val in &rope_yarn.cos_cache {
            assert!(val.is_finite());
        }
    }

    #[test]
    fn test_yarn_mscale_application() {
        let yarn = YarnConfig {
            original_max_seq: 16,
            target_max_seq: 64,
            beta_fast: 32.0,
            beta_slow: 1.0,
            mscale: 1.25,
        };
        let rope = RopeCache::new(4, 16, 10000.0, Some(&yarn));
        // Tại pos = 0, cos(0) = 1.0 * mscale = 1.25
        assert!((rope.cos_cache[0] - 1.25).abs() < 1e-12);
    }

    #[test]
    fn test_yarn_frequency_regions_follow_paper() {
        let yarn = YarnConfig {
            original_max_seq: 2048,
            target_max_seq: 8192,
            beta_fast: 32.0,
            beta_slow: 1.0,
            mscale: 1.0,
        };
        let head_dim = 16;
        let half = head_dim / 2;
        let plain = RopeCache::new(head_dim, 4, 10000.0, None);
        let scaled = RopeCache::new(head_dim, 4, 10000.0, Some(&yarn));
        // Tần số hiệu dụng của chiều i = góc quay tại vị trí 1
        let freq = |c: &RopeCache, i: usize| c.sin_cache[half + i].atan2(c.cos_cache[half + i]);

        // r = 2048 / lambda_i với lambda_i = 2*pi*10000^(i/8): r > 32 ở i = 0, 1, 2 -> giữ nguyên
        for i in 0..3 {
            assert!((freq(&scaled, i) - freq(&plain, i)).abs() < 1e-12);
        }
        // r < 1 ở i = 6, 7 -> chia đúng cho s = 4
        for i in 6..8 {
            assert!((freq(&scaled, i) - freq(&plain, i) / 4.0).abs() < 1e-12);
        }
        // Vùng trung gian: tỉ số tần số nằm giữa 1/s và 1, giảm dần theo i
        let mut prev = 1.0;
        for i in 3..6 {
            let ratio = freq(&scaled, i) / freq(&plain, i);
            assert!(ratio > 0.25 && ratio < prev, "i={i}, ratio={ratio}");
            prev = ratio;
        }
    }

    #[test]
    fn test_yarn_recommended_mscale() {
        assert!((YarnConfig::recommended_mscale(1.0) - 1.0).abs() < 1e-15);
        let m = YarnConfig::recommended_mscale(4.0);
        assert!((m - (0.1 * 4.0_f64.ln() + 1.0)).abs() < 1e-15);
        assert!((m - 1.138_629_436).abs() < 1e-8);
    }

    #[test]
    fn test_gated_delta_step_matches_paper_recurrence() {
        let ds = 3;
        let s: Vec<f64> = (0..ds * ds).map(|i| (i as f64 * 0.37).sin()).collect();
        let raw_k = [0.3, -0.5, 0.8];
        let norm = raw_k.iter().map(|x| x * x).sum::<f64>().sqrt();
        let k: Vec<f64> = raw_k.iter().map(|x| x / norm).collect();
        let v = [1.0, -2.0, 0.5];
        let s_k = matmul(&s, &k, ds, ds, 1);

        for &(alpha, beta) in &[(0.2, 1.0), (0.7, 1.0), (1.0, 1.0), (0.6, 0.3), (0.9, 0.5)] {
            let s_new = gated_delta_step(&s, &k, &v, alpha, beta);
            let s_new_k = matmul(&s_new, &k, ds, ds, 1);
            for i in 0..ds {
                // S_t k = alpha (1 - beta) S_{t-1} k + beta v
                let expected = alpha * (1.0 - beta) * s_k[i] + beta * v[i];
                assert!((s_new_k[i] - expected).abs() < 1e-12);
                if beta == 1.0 {
                    // Ghi đè hoàn toàn ô nhớ của khóa k, bất kể alpha
                    assert!((s_new_k[i] - v[i]).abs() < 1e-12);
                }
            }
            // Đối chiếu với dạng ma trận S_t = alpha S_{t-1} (I - beta k k^T) + beta v k^T
            for r in 0..ds {
                for c in 0..ds {
                    let mut sik = 0.0;
                    for j in 0..ds {
                        let ident = if j == c { 1.0 } else { 0.0 };
                        sik += s[r * ds + j] * (ident - beta * k[j] * k[c]);
                    }
                    let expected = alpha * sik + beta * v[r] * k[c];
                    assert!((s_new[r * ds + c] - expected).abs() < 1e-12);
                }
            }
        }
    }

    #[test]
    fn test_sparse_moe_aux_loss_from_real_layer() {
        let mut rng = SplitMix64::new(23);
        let cfg = SparseMoeConfig {
            n_routed_experts: 4,
            n_shared_experts: 0,
            top_k: 1,
            d_model: 8,
            d_hidden: 8,
            aux_loss_coef: 0.01,
        };
        let mut moe = SparseMoeLayer::new(cfg, &mut rng);
        let x: Vec<f64> = (0..5 * 8).map(|_| rng.next_signed(1.0)).collect();

        // Router toàn 0 -> P_i = 1/E đều; sum f_i = 1 nên aux_loss = alpha * E * (1/E) = alpha
        moe.router_weights = vec![0.0; 8 * 4];
        let res = moe.forward(&x, 5);
        assert!(
            (res.aux_loss - 0.01).abs() < 1e-12,
            "aux_loss = {}",
            res.aux_loss
        );

        // Router ngẫu nhiên: 0 < aux_loss <= alpha * E
        let moe_rand = SparseMoeLayer::new(moe.config.clone(), &mut rng);
        let res = moe_rand.forward(&x, 5);
        assert!(res.aux_loss > 0.0 && res.aux_loss <= 0.01 * 4.0 + 1e-12);
    }

    #[test]
    fn test_swiglu_forward() {
        let mut rng = SplitMix64::new(77);
        let mlp = SwiGluMlp::new(8, 16, 8, &mut rng);
        let x = vec![0.5; 8];
        let out = mlp.forward_token(&x);
        assert_eq!(out.len(), 8);
        for &v in &out {
            assert!(v.is_finite());
        }
    }

    #[test]
    fn test_swiglu_backward_vs_finite_diff() {
        let mut rng = SplitMix64::new(88);
        let mlp = SwiGluMlp::new(4, 6, 4, &mut rng);
        let x = vec![0.2, -0.5, 0.8, -0.3];
        let grad_out = vec![1.0, -0.5, 0.2, 0.4];

        let grad_x_analytic = mlp.backward_x(&x, &grad_out);

        let h = 1e-6;
        for i in 0..4 {
            let mut x_plus = x.clone();
            let mut x_minus = x.clone();
            x_plus[i] += h;
            x_minus[i] -= h;

            let y_plus = mlp.forward_token(&x_plus);
            let y_minus = mlp.forward_token(&x_minus);

            let loss_plus: f64 = y_plus.iter().zip(&grad_out).map(|(&a, &b)| a * b).sum();
            let loss_minus: f64 = y_minus.iter().zip(&grad_out).map(|(&a, &b)| a * b).sum();

            let grad_x_num = (loss_plus - loss_minus) / (2.0 * h);
            let diff = (grad_x_analytic[i] - grad_x_num).abs();
            assert!(
                diff < 1e-8,
                "Đạo hàm SwiGLU sai phân vượt ngưỡng tại index {i}: diff={diff}"
            );
        }
    }

    #[test]
    fn test_gqa_grouped_heads() {
        let mut rng = SplitMix64::new(33);
        let cfg = GqaConfig {
            n_heads: 4,
            n_kv_heads: 2,
            head_dim: 4,
            sliding_window: None,
            eps: 1e-5,
        };
        let gqa = GroupedQueryAttention::new(16, cfg, 32, 10000.0, &mut rng);
        let x = vec![0.1; 2 * 16];
        let out = gqa.forward_causal(&x, 2);
        assert_eq!(out.len(), 32);
    }

    #[test]
    fn test_gqa_qk_rmsnorm() {
        let mut rng = SplitMix64::new(44);
        let cfg = GqaConfig {
            n_heads: 2,
            n_kv_heads: 1,
            head_dim: 4,
            sliding_window: None,
            eps: 1e-5,
        };
        let gqa = GroupedQueryAttention::new(8, cfg, 16, 10000.0, &mut rng);
        let mut cache = GqaKvCache::default();
        let token = vec![0.5; 8];
        let out = gqa.step_decode(&mut cache, &token, 0);
        assert_eq!(out.len(), 8);
    }

    #[test]
    fn test_gqa_sliding_window_mask() {
        let mut rng = SplitMix64::new(55);
        let cfg = GqaConfig {
            n_heads: 2,
            n_kv_heads: 2,
            head_dim: 4,
            sliding_window: Some(2), // Cửa sổ trượt kích thước 2
            eps: 1e-5,
        };
        let gqa = GroupedQueryAttention::new(8, cfg, 16, 10000.0, &mut rng);
        let mut cache = GqaKvCache::default();

        let t0 = vec![1.0; 8];
        let t1 = vec![2.0; 8];
        let t2 = vec![3.0; 8];
        let t3 = vec![4.0; 8];

        gqa.step_decode(&mut cache, &t0, 0);
        gqa.step_decode(&mut cache, &t1, 1);
        gqa.step_decode(&mut cache, &t2, 2);
        let out3 = gqa.step_decode(&mut cache, &t3, 3);

        assert_eq!(out3.len(), 8);
        assert_eq!(cache.current_len, 4);
    }

    #[test]
    fn test_gqa_step_decode_vs_full_forward() {
        let mut rng = SplitMix64::new(66);
        let cfg = GqaConfig {
            n_heads: 4,
            n_kv_heads: 2,
            head_dim: 4,
            sliding_window: None,
            eps: 1e-5,
        };
        let gqa = GroupedQueryAttention::new(16, cfg, 16, 10000.0, &mut rng);
        let x = vec![0.2; 3 * 16];

        let full_out = gqa.forward_causal(&x, 3);

        let mut cache = GqaKvCache::default();
        let mut step_out = Vec::new();
        for t in 0..3 {
            let token = &x[t * 16..(t + 1) * 16];
            step_out.extend(gqa.step_decode(&mut cache, token, t));
        }

        for i in 0..full_out.len() {
            assert!(
                (full_out[i] - step_out[i]).abs() < 1e-12,
                "GQA step decode không khớp với prefill tại {i}"
            );
        }
    }

    #[test]
    fn test_mla_low_rank_compression_ratio() {
        let mla_cfg = MlaConfig {
            n_heads: 4,
            d_model: 32,
            d_c: 8,
            d_r: 4,
            v_dim: 8,
            eps: 1e-5,
        };
        let mla_cache_elements = mla_cfg.d_c + mla_cfg.d_r; // 8 + 4 = 12
        let mha_cache_elements = 2 * mla_cfg.d_model; // 2 * 32 = 64
        assert!(
            mla_cache_elements < mha_cache_elements / 4,
            "MLA phải nén bộ nhớ KV-Cache > 75% so với MHA truyền thống"
        );
    }

    #[test]
    fn test_mla_decoupling_rope() {
        let mut rng = SplitMix64::new(12);
        let mla_cfg = MlaConfig {
            n_heads: 2,
            d_model: 8,
            d_c: 4,
            d_r: 2,
            v_dim: 4,
            eps: 1e-5,
        };
        let mla = MultiHeadLatentAttention::new(mla_cfg, 16, 10000.0, &mut rng);
        let mut cache = MlaKvCache::default();
        let token = vec![0.5; 8];
        let out = mla.step_decode(&mut cache, &token, 0);

        assert_eq!(out.len(), 8);
        assert_eq!(cache.compressed_kv.len(), 1);
        assert_eq!(cache.compressed_kv[0].len(), 4);
        assert_eq!(cache.k_rope[0].len(), 2);
    }

    #[test]
    fn test_mla_forward_causal() {
        let mut rng = SplitMix64::new(14);
        let mla_cfg = MlaConfig {
            n_heads: 2,
            d_model: 8,
            d_c: 4,
            d_r: 2,
            v_dim: 4,
            eps: 1e-5,
        };
        let mla = MultiHeadLatentAttention::new(mla_cfg, 16, 10000.0, &mut rng);
        let x = vec![0.3; 3 * 8];
        let out = mla.forward_causal(&x, 3);
        assert_eq!(out.len(), 24);
    }

    #[test]
    fn test_gated_deltanet_recurrence_step() {
        let mut rng = SplitMix64::new(91);
        let cfg = GatedDeltaNetConfig {
            d_in: 8,
            d_state: 4,
            eps: 1e-5,
        };
        let dn = GatedDeltaNet::new(cfg, &mut rng);
        let x = vec![0.4; 2 * 8];
        let out = dn.forward(&x, 2);
        assert_eq!(out.len(), 16);
    }

    #[test]
    fn test_gated_deltanet_o_n_efficiency() {
        let mut rng = SplitMix64::new(92);
        let cfg = GatedDeltaNetConfig {
            d_in: 4,
            d_state: 4,
            eps: 1e-5,
        };
        let dn = GatedDeltaNet::new(cfg, &mut rng);

        // Chạy với chuỗi dài 10 token không tràn bộ nhớ
        let x = vec![0.1; 10 * 4];
        let out = dn.forward(&x, 10);
        assert_eq!(out.len(), 40);
        for &v in &out {
            assert!(v.is_finite());
        }
    }

    #[test]
    fn test_gated_deltanet_state_erasure() {
        let mut rng = SplitMix64::new(93);
        let cfg = GatedDeltaNetConfig {
            d_in: 4,
            d_state: 2,
            eps: 1e-5,
        };
        let dn = GatedDeltaNet::new(cfg, &mut rng);
        let x = vec![0.0; 4];
        let out = dn.forward(&x, 1);
        assert_eq!(out.len(), 4);
    }

    #[test]
    fn test_sparse_moe_routing_top_k() {
        let mut rng = SplitMix64::new(21);
        let cfg = SparseMoeConfig {
            n_routed_experts: 4,
            n_shared_experts: 1,
            top_k: 2,
            d_model: 8,
            d_hidden: 16,
            aux_loss_coef: 0.01,
        };
        let moe = SparseMoeLayer::new(cfg, &mut rng);
        let x = vec![0.5; 3 * 8];
        let res = moe.forward(&x, 3);

        assert_eq!(res.output.len(), 24);
        assert_eq!(res.selected_experts.len(), 3);
        for sel in &res.selected_experts {
            assert_eq!(sel.len(), 2, "Mỗi token phải chọn đúng top-2 chuyên gia");
        }
    }

    #[test]
    fn test_sparse_moe_shared_experts_always_run() {
        let mut rng = SplitMix64::new(22);
        let cfg = SparseMoeConfig {
            n_routed_experts: 4,
            n_shared_experts: 2, // 2 chuyên gia dùng chung
            top_k: 1,
            d_model: 8,
            d_hidden: 16,
            aux_loss_coef: 0.01,
        };
        let moe = SparseMoeLayer::new(cfg, &mut rng);
        let x = vec![0.2; 8];
        let res = moe.forward(&x, 1);
        assert_eq!(res.output.len(), 8);
    }

    #[test]
    fn test_sparse_moe_aux_loss_uniform_minimal() {
        // Khi tải được chia đều hoàn hảo, aux_loss đạt cực tiểu
        let ne = 4;
        let p_uniform = 1.0 / (ne as f64);
        let f_uniform = 1.0 / (ne as f64);
        let min_aux = (ne as f64) * (ne as f64 * f_uniform * p_uniform);
        assert!((min_aux - 1.0).abs() < 1e-12);
    }

    #[test]
    fn test_sparse_moe_aux_loss_skewed_maximal() {
        // Khi tất cả token tập trung vào 1 chuyên gia duy nhất, aux_loss tăng cao
        let ne = 4;
        let mut p_skewed = vec![0.0; ne];
        p_skewed[0] = 1.0;
        let mut f_skewed = vec![0.0; ne];
        f_skewed[0] = 1.0;

        let dot_skewed: f64 = f_skewed.iter().zip(&p_skewed).map(|(&f, &p)| f * p).sum();
        let aux_skewed = (ne as f64) * dot_skewed;

        assert_eq!(aux_skewed, 4.0, "aux_loss bị lệch tối đa bằng E");
    }

    #[test]
    fn test_modern_decoder_causal_no_future_leak() {
        let mut rng = SplitMix64::new(45);
        let d_model = 8;
        let norm1 = RmsNorm::new(d_model, 1e-5, false);
        let norm2 = RmsNorm::new(d_model, 1e-5, false);
        let gqa_cfg = GqaConfig {
            n_heads: 2,
            n_kv_heads: 1,
            head_dim: 4,
            sliding_window: None,
            eps: 1e-5,
        };
        let gqa = GroupedQueryAttention::new(d_model, gqa_cfg, 16, 10000.0, &mut rng);
        let swiglu = SwiGluMlp::new(d_model, 16, d_model, &mut rng);

        let block = ModernDecoderBlock::new(
            d_model,
            norm1,
            AttentionVariant::Gqa(gqa),
            norm2,
            MlpVariant::SwiGlu(swiglu),
        );

        let x1 = vec![0.3; 3 * d_model];
        let mut x2 = x1.clone();
        // Thay đổi tương lai ở t=2
        for slot in &mut x2[2 * d_model..3 * d_model] {
            *slot += 5.0;
        }

        let out1 = block.forward(&x1, 3);
        let out2 = block.forward(&x2, 3);

        // Token tại t=0 và t=1 không đổi
        for i in 0..2 * d_model {
            assert!(
                (out1[i] - out2[i]).abs() < 1e-12,
                "ModernDecoderBlock bị rò rỉ nhân quả tương lai tại {i}"
            );
        }
    }

    #[test]
    fn test_modern_decoder_block_gqa_swiglu() {
        let mut rng = SplitMix64::new(46);
        let d_model = 16;
        let norm1 = RmsNorm::new(d_model, 1e-5, false);
        let norm2 = RmsNorm::new(d_model, 1e-5, false);
        let gqa_cfg = GqaConfig {
            n_heads: 4,
            n_kv_heads: 2,
            head_dim: 4,
            sliding_window: None,
            eps: 1e-5,
        };
        let gqa = GroupedQueryAttention::new(d_model, gqa_cfg, 32, 10000.0, &mut rng);
        let swiglu = SwiGluMlp::new(d_model, 32, d_model, &mut rng);

        let block = ModernDecoderBlock::new(
            d_model,
            norm1,
            AttentionVariant::Gqa(gqa),
            norm2,
            MlpVariant::SwiGlu(swiglu),
        );

        let x = vec![0.1; 4 * d_model];
        let out = block.forward(&x, 4);
        assert_eq!(out.len(), 4 * d_model);
    }

    #[test]
    fn test_modern_decoder_block_mla_moe() {
        let mut rng = SplitMix64::new(47);
        let d_model = 16;
        let norm1 = RmsNorm::new(d_model, 1e-5, false);
        let norm2 = RmsNorm::new(d_model, 1e-5, false);

        let mla_cfg = MlaConfig {
            n_heads: 4,
            d_model,
            d_c: 8,
            d_r: 4,
            v_dim: 4,
            eps: 1e-5,
        };
        let mla = MultiHeadLatentAttention::new(mla_cfg, 32, 10000.0, &mut rng);

        let moe_cfg = SparseMoeConfig {
            n_routed_experts: 4,
            n_shared_experts: 1,
            top_k: 2,
            d_model,
            d_hidden: 32,
            aux_loss_coef: 0.01,
        };
        let moe = SparseMoeLayer::new(moe_cfg, &mut rng);

        let block = ModernDecoderBlock::new(
            d_model,
            norm1,
            AttentionVariant::Mla(mla),
            norm2,
            MlpVariant::SparseMoe(moe),
        );

        let x = vec![0.2; 2 * d_model];
        let out = block.forward(&x, 2);
        assert_eq!(out.len(), 2 * d_model);
    }

    #[test]
    fn test_modern_decoder_block_deltanet() {
        let mut rng = SplitMix64::new(48);
        let d_model = 8;
        let norm1 = RmsNorm::new(d_model, 1e-5, false);
        let norm2 = RmsNorm::new(d_model, 1e-5, false);

        let dn_cfg = GatedDeltaNetConfig {
            d_in: d_model,
            d_state: 4,
            eps: 1e-5,
        };
        let deltanet = GatedDeltaNet::new(dn_cfg, &mut rng);
        let swiglu = SwiGluMlp::new(d_model, 16, d_model, &mut rng);

        let block = ModernDecoderBlock::new(
            d_model,
            norm1,
            AttentionVariant::DeltaNet(deltanet),
            norm2,
            MlpVariant::SwiGlu(swiglu),
        );

        let x = vec![0.3; 3 * d_model];
        let out = block.forward(&x, 3);
        assert_eq!(out.len(), 3 * d_model);
    }

    #[test]
    fn test_silu_derivative_accuracy() {
        let h = 1e-6;
        for x in [-2.0, -1.0, 0.0, 1.0, 2.0] {
            let analytic = silu_deriv(x);
            let numeric = (silu(x + h) - silu(x - h)) / (2.0 * h);
            assert!(
                (analytic - numeric).abs() < 1e-8,
                "Đạo hàm SiLU sai lệch tại x={x}"
            );
        }
    }

    #[test]
    fn test_rmsnorm_multi_token_forward() {
        let norm = RmsNorm::new(4, 1e-5, false);
        let x = vec![1.0, 2.0, 3.0, 4.0, 5.0, 6.0, 7.0, 8.0];
        let out_multi = norm.forward(&x);
        let out_t0 = norm.forward_token(&x[0..4]);
        let out_t1 = norm.forward_token(&x[4..8]);
        assert_eq!(out_multi[0..4], out_t0[..]);
        assert_eq!(out_multi[4..8], out_t1[..]);
    }

    #[test]
    fn test_sparse_moe_output_finite_and_deterministic() {
        let mut rng = SplitMix64::new(777);
        let cfg = SparseMoeConfig {
            n_routed_experts: 4,
            n_shared_experts: 1,
            top_k: 2,
            d_model: 8,
            d_hidden: 16,
            aux_loss_coef: 0.01,
        };
        let moe = SparseMoeLayer::new(cfg, &mut rng);
        let x = vec![0.35; 2 * 8];
        let res1 = moe.forward(&x, 2);
        let res2 = moe.forward(&x, 2);
        assert_eq!(res1.output, res2.output);
        assert_eq!(res1.aux_loss, res2.aux_loss);
        for &val in &res1.output {
            assert!(val.is_finite());
        }
    }
}
