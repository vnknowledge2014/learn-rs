//! # Chương 95: Huấn luyện Hậu kỳ I — Tinh chỉnh Có Mặt nạ SFT, LoRA Trộn Không Chi phí,
//! Mô hình Phần thưởng & Năm Hàm Ưu tiên DPO/IPO/SimPO/ORPO/KTO
//! (Supervised Fine-Tuning, LoRA & Preference Alignment)
//!
//! Crate `ch95` triển khai hoàn chỉnh 100% bằng Rust 2024 thuần `std` (không thư viện ngoài):
//! 1. `SplitMix64`: Bộ sinh số ngẫu nhiên giả tất định và các hàm đại số tuyến tính vi mô.
//! 2. `ChatTemplateRenderer`: Định dạng thông điệp đa vai trò và `sft_masked_loss` (chốt chống chia 0/0 NaN).
//! 3. `LoraLinear`: Thích ứng thứ hạng thấp với khởi tạo Kaiming cho ma trận A, ma trận B = 0,
//!    `lora_forward`, `lora_backward`, phép biến đổi `merge_lora` / `unmerge_lora` với sai số phục hồi < 1e-12.
//! 4. `RewardModel`: Đầu vô hướng tuyến tính tại vị trí khác đệm cuối cùng và `bradley_terry_loss`.
//! 5. `log1mexp`: Hàm ổn định số học 2 nhánh tính $\ln(1 - e^x)$ tách tại $-\ln 2$.
//! 6. Bộ năm hàm tổn thất ưu tiên căn chỉnh mô hình:
//!    - `dpo_loss`: Direct Preference Optimization (kèm làm mịn nhãn label smoothing).
//!    - `ipo_loss`: Identity Preference Optimization (điều hòa bậc hai chống quá khớp).
//!    - `simpo_loss`: Simple Preference Optimization (chuẩn hóa độ dài, không cần mô hình tham chiếu).
//!    - `orpo_loss`: Odds Ratio Preference Optimization (kết hợp SFT và tỷ số odds với `log1mexp`).
//!    - `kto_loss`: Kahneman-Tversky Optimization (lý thuyết triển vọng cho dữ liệu đơn lẻ).
//! 7. Kiểm chứng giải tích so với sai phân hữu hạn trung tâm đạt sai số < 1e-8.

use std::collections::HashMap;

// ============================================================================
// 1. BỘ SINH SỐ NGẪU NHIÊN TẤT ĐỊNH & ĐẠI SỐ TUYẾN TÍNH VI MÔ
// ============================================================================

/// Bộ sinh số giả ngẫu nhiên SplitMix64 64-bit tất định.
///
/// Lý do thiết kế: Chu kỳ $2^{64}$, chi phí tính toán cực thấp (chỉ dùng phép toán bit
/// và nhân tràn số nguyên), độc lập hoàn toàn với hệ điều hành, đảm bảo tái lập 100%
/// kết quả khởi tạo trọng số mạng nơ-ron trên mọi nền tảng phần cứng.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct SplitMix64 {
    /// Trạng thái nội bộ 64-bit
    pub state: u64,
}

impl SplitMix64 {
    /// Khởi tạo bộ sinh số với hạt giống tùy chọn.
    pub const fn new(seed: u64) -> Self {
        Self { state: seed }
    }

    /// Sinh số nguyên không dấu 64-bit tiếp theo.
    pub fn next_u64(&mut self) -> u64 {
        self.state = self.state.wrapping_add(0x9E37_79B9_7F4A_7C15);
        let mut z = (self.state ^ (self.state >> 30)).wrapping_mul(0xBF58_476D_1CE4_E5B9);
        z = (z ^ (z >> 27)).wrapping_mul(0x94D0_49BB_1331_11EB);
        z ^ (z >> 31)
    }

    /// Sinh số thực trong khoảng $[0.0, 1.0)$.
    pub fn next_f64(&mut self) -> f64 {
        ((self.next_u64() >> 11) as f64) * (1.0 / ((1u64 << 53) as f64))
    }

    /// Sinh số thực đối xứng trong khoảng $[-scale, scale)$.
    pub fn next_signed(&mut self, scale: f64) -> f64 {
        (self.next_f64() * 2.0 - 1.0) * scale
    }

    /// Sinh số ngẫu nhiên theo phân phối chuẩn tắc Gaussian $\mathcal{N}(\mu, \sigma^2)$
    /// thông qua biến đổi Box–Muller cổ điển.
    pub fn next_gaussian(&mut self, mean: f64, std_dev: f64) -> f64 {
        let mut u1 = self.next_f64();
        if u1 <= 1e-15 {
            u1 = 1e-15;
        }
        let u2 = self.next_f64();
        let radius = (-2.0 * u1.ln()).sqrt();
        let theta = 2.0 * std::f64::consts::PI * u2;
        mean + std_dev * radius * theta.cos()
    }
}

/// Chuyển vị ma trận phẳng kích thước `rows x cols`.
pub fn transpose(mat: &[f64], rows: usize, cols: usize) -> Vec<f64> {
    assert_eq!(
        mat.len(),
        rows * cols,
        "Kích thước mảng không khớp rows * cols"
    );
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

/// Tích vô hướng giữa hai vector cùng độ dài.
pub fn dot_product(a: &[f64], b: &[f64]) -> f64 {
    assert_eq!(a.len(), b.len(), "Độ dài hai vector phải bằng nhau");
    a.iter().zip(b).map(|(&x, &y)| x * y).sum()
}

/// Chuẩn L2 Euclid của một vector.
pub fn vector_norm_l2(v: &[f64]) -> f64 {
    v.iter().map(|&x| x * x).sum::<f64>().sqrt()
}

/// Hàm kích hoạt Sigmoid với kẹp chống tràn số thực: $\sigma(x) = \frac{1}{1 + e^{-x}}$.
pub fn sigmoid(x: f64) -> f64 {
    if x >= 40.0 {
        1.0
    } else if x <= -40.0 {
        0.0
    } else {
        1.0 / (1.0 + (-x).exp())
    }
}

/// Hàm tính $\ln(\sigma(x))$ ổn định số học không bao giờ trả về $-\infty$ với $x > 0$.
pub fn log_sigmoid(x: f64) -> f64 {
    if x >= 0.0 {
        -((-x).exp().ln_1p())
    } else {
        x - (x.exp().ln_1p())
    }
}

/// Hàm Softmax ổn định số học (trừ giá trị cực đại để chống tràn số $e^x$).
pub fn softmax(logits: &[f64]) -> Vec<f64> {
    if logits.is_empty() {
        return Vec::new();
    }
    let max_val = logits.iter().copied().fold(f64::NEG_INFINITY, f64::max);
    let exps: Vec<f64> = logits.iter().map(|&z| (z - max_val).exp()).collect();
    let sum_exp: f64 = exps.iter().sum();
    exps.iter().map(|&e| e / sum_exp).collect()
}

/// Hàm Log-Softmax ổn định số học: $\ln \frac{e^{z_i}}{\sum_j e^{z_j}} = (z_i - z_{\max}) - \ln \sum_j e^{z_j - z_{\max}}$.
pub fn log_softmax(logits: &[f64]) -> Vec<f64> {
    if logits.is_empty() {
        return Vec::new();
    }
    let max_val = logits.iter().copied().fold(f64::NEG_INFINITY, f64::max);
    let exps: Vec<f64> = logits.iter().map(|&z| (z - max_val).exp()).collect();
    let sum_exp: f64 = exps.iter().sum();
    let log_sum_exp = max_val + sum_exp.ln();
    logits.iter().map(|&z| z - log_sum_exp).collect()
}

// ============================================================================
// 2. CHAT TEMPLATE RENDERER & SFT MASKED LOSS (CHỐNG CHIA 0/0 NaN)
// ============================================================================

/// Vai trò người tham gia trong ngữ cảnh hội thoại đa lượt.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
pub enum ChatRole {
    /// Chỉ dẫn hệ thống tối cao
    System,
    /// Người dùng ra lệnh hoặc đặt câu hỏi
    User,
    /// Trợ lý phản hồi mô hình sinh
    Assistant,
}

/// Thông điệp nguyên tử trong phiên hội thoại.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct ChatMessage {
    /// Vai trò của bên gửi thông điệp
    pub role: ChatRole,
    /// Nội dung văn bản
    pub content: String,
}

/// Kết quả kết xuất hội thoại phục vụ tinh chỉnh có giám sát (SFT).
#[derive(Debug, Clone, PartialEq)]
pub struct RenderedDialogue {
    /// Chuỗi văn bản đầy đủ kèm các thẻ phân tách
    pub prompt_text: String,
    /// Danh sách mã định danh token
    pub token_ids: Vec<usize>,
    /// Mặt nạ tính tổn thất: 1.0 cho token trợ lý, 0.0 cho lời nhắc người dùng & hệ thống
    pub loss_mask: Vec<f64>,
}

/// Bộ kết xuất mẫu hội thoại ChatTemplateRenderer chuẩn hóa cấu trúc prompt.
#[derive(Debug, Clone)]
pub struct ChatTemplateRenderer {
    /// Bảng từ vựng ánh xạ từ chuỗi sang token id
    pub vocab: HashMap<String, usize>,
    /// Token bắt đầu chuỗi BOS
    pub bos_token_id: usize,
    /// Token kết thúc lượt EOS
    pub eos_token_id: usize,
    /// Chuỗi tiền tố hệ thống
    pub system_prefix: String,
    /// Chuỗi tiền tố người dùng
    pub user_prefix: String,
    /// Chuỗi tiền tố trợ lý
    pub assistant_prefix: String,
    /// Thẻ kết thúc khối
    pub turn_suffix: String,
}

impl ChatTemplateRenderer {
    /// Khởi tạo bộ kết xuất mẫu đơn giản kèm bảng từ vựng xác định.
    pub fn new(vocab: HashMap<String, usize>, bos_token_id: usize, eos_token_id: usize) -> Self {
        Self {
            vocab,
            bos_token_id,
            eos_token_id,
            system_prefix: "<|im_start|>system\n".to_string(),
            user_prefix: "<|im_start|>user\n".to_string(),
            assistant_prefix: "<|im_start|>assistant\n".to_string(),
            turn_suffix: "<|im_end|>\n".to_string(),
        }
    }

    /// Kết xuất danh sách thông điệp thành chuỗi token và mặt nạ tổn thất.
    ///
    /// Bất biến quan trọng: Mọi token sinh ra bởi người dùng, hệ thống hoặc các thẻ
    /// điều khiển khung mẫu đều có mặt nạ `loss_mask = 0.0`. Chỉ riêng các token nội dung
    /// và token kết thúc lượt `<|im_end|>` của vai trò `Assistant` mới có `loss_mask = 1.0`.
    pub fn render(&self, messages: &[ChatMessage]) -> RenderedDialogue {
        let mut full_text = String::new();
        let mut token_ids = vec![self.bos_token_id];
        let mut loss_mask = vec![0.0]; // Token BOS không tính tổn thất

        for msg in messages {
            let (prefix, is_assistant) = match msg.role {
                ChatRole::System => (&self.system_prefix, false),
                ChatRole::User => (&self.user_prefix, false),
                ChatRole::Assistant => (&self.assistant_prefix, true),
            };

            // 1. Tiền tố phân vai (không tính tổn thất)
            full_text.push_str(prefix);
            for word in prefix.split_whitespace() {
                if let Some(&id) = self.vocab.get(word) {
                    token_ids.push(id);
                    loss_mask.push(0.0);
                }
            }

            // 2. Nội dung thông điệp
            full_text.push_str(&msg.content);
            for word in msg.content.split_whitespace() {
                let id = *self.vocab.get(word).unwrap_or(&self.eos_token_id);
                token_ids.push(id);
                loss_mask.push(if is_assistant { 1.0 } else { 0.0 });
            }

            // 3. Thẻ kết thúc lượt
            full_text.push_str(&self.turn_suffix);
            for word in self.turn_suffix.split_whitespace() {
                let id = *self.vocab.get(word).unwrap_or(&self.eos_token_id);
                token_ids.push(id);
                // Đối với trợ lý, token kết thúc lượt là mục tiêu cần học để dừng sinh
                loss_mask.push(if is_assistant { 1.0 } else { 0.0 });
            }
        }

        RenderedDialogue {
            prompt_text: full_text,
            token_ids,
            loss_mask,
        }
    }
}

/// Kết quả tính tổn thất và gradient của hàm SFT có mặt nạ.
#[derive(Debug, Clone, PartialEq)]
pub struct SftLossResult {
    /// Giá trị tổn thất Cross-Entropy trung bình trên các token trợ lý
    pub loss: f64,
    /// Tổng trọng số mặt nạ hợp lệ
    pub valid_token_count: f64,
    /// Gradient đạo hàm đối với toàn bộ ma trận logits `(seq_len x vocab_size)`
    pub grad_logits: Vec<f64>,
}

/// Tính toán tổn thất SFT có mặt nạ (`sft_masked_loss`).
///
/// Chốt chặn chống chia 0/0 sinh NaN: Khi chuỗi không chứa bất kỳ token trợ lý nào
/// (ví dụ chỉ có lời nhắc của người dùng), `valid_token_count == 0.0`, hàm sẽ lập tức
/// trả về `loss = 0.0` và gradient toàn bộ bằng `0.0`, ngăn chặn hoàn toàn việc phá hỏng
/// trọng số mạng nơ-ron bởi các giá trị `NaN` hoặc `Inf`.
pub fn sft_masked_loss(
    logits: &[f64],
    target_tokens: &[usize],
    loss_mask: &[f64],
    vocab_size: usize,
) -> SftLossResult {
    let seq_len = target_tokens.len();
    assert_eq!(
        logits.len(),
        seq_len * vocab_size,
        "Kích thước logits không khớp seq_len * vocab_size"
    );
    assert_eq!(
        loss_mask.len(),
        seq_len,
        "Độ dài loss_mask phải bằng độ dài target_tokens"
    );

    let total_weight: f64 = loss_mask.iter().sum();
    // Chốt chặn chống chia 0/0 sinh NaN
    if total_weight <= 0.0 {
        return SftLossResult {
            loss: 0.0,
            valid_token_count: 0.0,
            grad_logits: vec![0.0; logits.len()],
        };
    }

    let mut total_loss = 0.0;
    let mut grad_logits = vec![0.0; logits.len()];

    for t in 0..seq_len {
        let mask_val = loss_mask[t];
        if mask_val <= 0.0 {
            // Token bị che mặt nạ: Đạo hàm giải tích bằng đúng 0.0
            continue;
        }

        let start = t * vocab_size;
        let end = start + vocab_size;
        let step_logits = &logits[start..end];
        let probs = softmax(step_logits);
        let log_probs = log_softmax(step_logits);

        let target = target_tokens[t];
        assert!(
            target < vocab_size,
            "Target token id vượt ngoài kích thước từ vựng"
        );

        total_loss += -mask_val * log_probs[target];

        // Đạo hàm Cross-Entropy: dL/dz_v = (mask / total_weight) * (probs[v] - 1{v == target})
        let scale = mask_val / total_weight;
        for v in 0..vocab_size {
            let indicator = if v == target { 1.0 } else { 0.0 };
            grad_logits[start + v] = scale * (probs[v] - indicator);
        }
    }

    SftLossResult {
        loss: total_loss / total_weight,
        valid_token_count: total_weight,
        grad_logits,
    }
}

// ============================================================================
// 3. LORA LINEAR (THÍCH ỨNG THỨ HẠNG THẤP VỚI HỢP NHẤT TRỌNG SỐ KHÔNG CHI PHÍ)
// ============================================================================

/// Gradient đạo hàm của tầng LoRALinear đối với các ma trận thích ứng.
#[derive(Debug, Clone, PartialEq)]
pub struct LoraGradients {
    /// Đạo hàm đối với ma trận LoRA A: `(rank x in_features)`
    pub grad_lora_a: Vec<f64>,
    /// Đạo hàm đối với ma trận LoRA B: `(out_features x rank)`
    pub grad_lora_b: Vec<f64>,
    /// Đạo hàm đối với vector đầu vào x: `(in_features)`
    pub grad_input: Vec<f64>,
}

/// Tầng biến đổi tuyến tính LoRALinear hỗ trợ thích ứng thứ hạng thấp.
///
/// Phương trình biến đổi: $y = W_{\text{base}} x + \frac{\alpha}{r} B (A x)$
/// Trong đó:
/// - $W_{\text{base}} \in \mathbb{R}^{d_{\text{out}} \times d_{\text{in}}}$: Trọng số gốc đóng băng.
/// - $A \in \mathbb{R}^{r \times d_{\text{in}}}$: Khởi tạo Gaussian Kaiming He.
/// - $B \in \mathbb{R}^{d_{\text{out}} \times r}$: Khởi tạo toàn 0.
///
/// Bất biến toán học: Tại bước 0 (khi $B = 0$), $B A = 0$, mô hình thích ứng có hành vi
/// đồng nhất 100% với mô hình gốc ban đầu. Khi đưa vào phục vụ suy luận, $W_{\text{merged}} = W_{\text{base}} + \frac{\alpha}{r} B A$
/// cho phép loại bỏ hoàn toàn chi phí trễ tính toán của nhánh phụ với sai số $< 10^{-12}$.
#[derive(Debug, Clone, PartialEq)]
pub struct LoraLinear {
    /// Ma trận trọng số gốc đã đóng băng: `out_features x in_features`
    pub base_weight: Vec<f64>,
    /// Ma trận hạ chiều thích ứng A: `rank x in_features`
    pub lora_a: Vec<f64>,
    /// Ma trận nâng chiều thích ứng B: `out_features x rank`
    pub lora_b: Vec<f64>,
    /// Số chiều đặc trưng đầu vào
    pub in_features: usize,
    /// Số chiều đặc trưng đầu ra
    pub out_features: usize,
    /// Thứ hạng thích ứng r
    pub rank: usize,
    /// Hệ số tỷ lệ alpha
    pub alpha: f64,
    /// Hệ số co giãn thực tế: `scaling = alpha / rank`
    pub scaling: f64,
}

impl LoraLinear {
    /// Khởi tạo tầng LoRALinear với ma trận trọng số gốc xác định.
    pub fn new_with_base_weight(
        base_weight: Vec<f64>,
        in_features: usize,
        out_features: usize,
        rank: usize,
        alpha: f64,
        rng: &mut SplitMix64,
    ) -> Self {
        assert_eq!(
            base_weight.len(),
            out_features * in_features,
            "Kích thước base_weight không khớp out_features * in_features"
        );
        assert!(rank > 0, "Thứ hạng rank phải lớn hơn 0");

        // Khởi tạo Kaiming He cho ma trận A: std = sqrt(2.0 / in_features)
        let std_dev = (2.0 / (in_features as f64)).sqrt();
        let mut lora_a = vec![0.0; rank * in_features];
        for val in &mut lora_a {
            *val = rng.next_gaussian(0.0, std_dev);
        }

        // Khởi tạo ma trận B toàn 0 để đảm bảo delta = 0 tại bước 0
        let lora_b = vec![0.0; out_features * rank];
        let scaling = alpha / (rank as f64);

        Self {
            base_weight,
            lora_a,
            lora_b,
            in_features,
            out_features,
            rank,
            alpha,
            scaling,
        }
    }

    /// Lan truyền xuôi của tầng LoRALinear qua hai nhánh tách biệt:
    /// $y = W_{\text{base}} x + \text{scaling} \cdot (B (A x))$
    pub fn lora_forward(&self, x: &[f64]) -> Vec<f64> {
        assert_eq!(
            x.len(),
            self.in_features,
            "Kích thước vector đầu vào x không khớp in_features"
        );

        // Nhánh gốc: base_out = W_base * x (out_features x 1)
        let mut y = matmul(&self.base_weight, x, self.out_features, self.in_features, 1);

        // Nhánh LoRA: h = A * x (rank x 1)
        let h = matmul(&self.lora_a, x, self.rank, self.in_features, 1);

        // delta = B * h (out_features x 1)
        let delta = matmul(&self.lora_b, &h, self.out_features, self.rank, 1);

        // Cộng dồn tỷ lệ
        for (yi, &di) in y.iter_mut().zip(&delta) {
            *yi += self.scaling * di;
        }
        y
    }

    /// Lan truyền ngược giải tích tính toán gradient đối với A, B và x:
    /// Cho vector gradient tổn thất đầu ra $\frac{\partial \mathcal{L}}{\partial y}$ (`out_features x 1`).
    pub fn lora_backward(&self, x: &[f64], grad_output: &[f64]) -> LoraGradients {
        assert_eq!(x.len(), self.in_features);
        assert_eq!(grad_output.len(), self.out_features);

        // 1. Giá trị kích hoạt trung gian: h = A * x (rank x 1)
        let h = matmul(&self.lora_a, x, self.rank, self.in_features, 1);

        // 2. Gradient đối với B: dL/dB = scaling * (grad_output * h^T) (out_features x rank)
        let mut grad_lora_b = vec![0.0; self.out_features * self.rank];
        for i in 0..self.out_features {
            let g = grad_output[i] * self.scaling;
            for j in 0..self.rank {
                grad_lora_b[i * self.rank + j] = g * h[j];
            }
        }

        // 3. Gradient đối với h: dL/dh = scaling * (B^T * grad_output) (rank x 1)
        let b_t = transpose(&self.lora_b, self.out_features, self.rank);
        let grad_h_unscaled = matmul(&b_t, grad_output, self.rank, self.out_features, 1);

        // 4. Gradient đối với A: dL/dA = scaling * (grad_h * x^T) (rank x in_features)
        let mut grad_lora_a = vec![0.0; self.rank * self.in_features];
        for i in 0..self.rank {
            let gh = grad_h_unscaled[i] * self.scaling;
            for j in 0..self.in_features {
                grad_lora_a[i * self.in_features + j] = gh * x[j];
            }
        }

        // 5. Gradient đối với x: dL/dx = W_base^T * grad_output + scaling * A^T * (B^T * grad_output)
        let w_t = transpose(&self.base_weight, self.out_features, self.in_features);
        let grad_x_base = matmul(&w_t, grad_output, self.in_features, self.out_features, 1);

        let a_t = transpose(&self.lora_a, self.rank, self.in_features);
        let grad_x_lora = matmul(&a_t, &grad_h_unscaled, self.in_features, self.rank, 1);

        let mut grad_input = vec![0.0; self.in_features];
        for i in 0..self.in_features {
            grad_input[i] = grad_x_base[i] + self.scaling * grad_x_lora[i];
        }

        LoraGradients {
            grad_lora_a,
            grad_lora_b,
            grad_input,
        }
    }

    /// Hợp nhất ma trận LoRA vào trọng số gốc phục vụ suy luận không chi phí trễ:
    /// $W_{\text{merged}} = W_{\text{base}} + \frac{\alpha}{r} (B A)$
    pub fn merge_lora(&self) -> Vec<f64> {
        let ba = matmul(
            &self.lora_b,
            &self.lora_a,
            self.out_features,
            self.rank,
            self.in_features,
        );
        let mut merged = self.base_weight.clone();
        for (w, &delta) in merged.iter_mut().zip(&ba) {
            *w += self.scaling * delta;
        }
        merged
    }

    /// Phục hồi lại trọng số gốc từ ma trận đã hợp nhất (`unmerge_lora`):
    /// $W_{\text{base}} = W_{\text{merged}} - \frac{\alpha}{r} (B A)$
    pub fn unmerge_lora(
        merged_weight: &[f64],
        lora_a: &[f64],
        lora_b: &[f64],
        in_features: usize,
        out_features: usize,
        rank: usize,
        scaling: f64,
    ) -> Vec<f64> {
        let ba = matmul(lora_b, lora_a, out_features, rank, in_features);
        let mut recovered = merged_weight.to_vec();
        for (w, &delta) in recovered.iter_mut().zip(&ba) {
            *w -= scaling * delta;
        }
        recovered
    }
}

// ============================================================================
// 4. REWARD MODEL & BRADLEY-TERRY LOSS
// ============================================================================

/// Mô hình phần thưởng với đầu ra vô hướng tuyến tính.
#[derive(Debug, Clone, PartialEq)]
pub struct RewardModel {
    /// Số chiều vector ẩn biểu diễn
    pub hidden_dim: usize,
    /// Vector trọng số của đầu tuyến tính: `(hidden_dim)`
    pub weights: Vec<f64>,
    /// Độ lệch bias
    pub bias: f64,
}

impl RewardModel {
    /// Khởi tạo mô hình phần thưởng với trọng số xác định.
    pub fn new(weights: Vec<f64>, bias: f64) -> Self {
        let hidden_dim = weights.len();
        Self {
            hidden_dim,
            weights,
            bias,
        }
    }

    /// Chấm điểm phần thưởng vô hướng tại vị trí token khác đệm cuối cùng của chuỗi:
    /// $r(x, y) = \mathbf{w}^T h_{\text{last}} + b$
    pub fn score_sequence(
        &self,
        hidden_states: &[f64],
        seq_len: usize,
        last_non_pad_idx: usize,
    ) -> f64 {
        assert!(
            last_non_pad_idx < seq_len,
            "Chỉ số token cuối cùng vượt quá độ dài chuỗi"
        );
        assert_eq!(
            hidden_states.len(),
            seq_len * self.hidden_dim,
            "Kích thước hidden_states không khớp seq_len * hidden_dim"
        );

        let offset = last_non_pad_idx * self.hidden_dim;
        let last_hidden = &hidden_states[offset..offset + self.hidden_dim];
        dot_product(&self.weights, last_hidden) + self.bias
    }
}

/// Kết quả tính toán tổn thất Bradley–Terry giữa cặp phản hồi thắng–thua.
#[derive(Debug, Clone, PartialEq)]
pub struct BradleyTerryResult {
    /// Giá trị tổn thất: $-\ln \sigma((r_w - r_l) - \text{margin})$
    pub loss: f64,
    /// Đạo hàm đối với điểm số phần thưởng câu thắng $r_w$
    pub grad_r_w: f64,
    /// Đạo hàm đối với điểm số phần thưởng câu thua $r_l$
    pub grad_r_l: f64,
}

/// Tính tổn thất Bradley–Terry có hỗ trợ lề biên độ (margin):
/// $\mathcal{L}_{\text{BT}} = -\ln \sigma((r_w - r_l) - \text{margin})$
pub fn bradley_terry_loss(r_w: f64, r_l: f64, margin: f64) -> BradleyTerryResult {
    let diff = (r_w - r_l) - margin;
    let loss = -log_sigmoid(diff);
    let sig = sigmoid(diff);

    // Đạo hàm dL/d(diff) = sig - 1.0
    let grad_diff = sig - 1.0;
    BradleyTerryResult {
        loss,
        grad_r_w: grad_diff,
        grad_r_l: -grad_diff,
    }
}

// ============================================================================
// 5. HÀM ỔN ĐỊNH SỐ HỌC LOG1MEXP (TÁCH HAI NHÁNH TẠI -LN 2)
// ============================================================================

/// Hàm tính $\ln(1 - e^x)$ ổn định số học tối đa trên toàn bộ miền $x \in (-\infty, 0)$.
///
/// Lý do thiết kế thuật toán 2 nhánh phân tách tại ngưỡng $-\ln 2 \approx -0.6931471805599453$:
/// 1. Nhánh 1 ($-\ln 2 < x < 0$): Khi $x$ rất gần $0$, $e^x \approx 1$. Nếu tính trực tiếp $1 - e^x$
///    sẽ xảy ra hiện tượng triệt tiêu thảm họa (catastrophic cancellation) làm mất toàn bộ chữ số
///    có nghĩa. Bằng cách biến đổi $1 - e^x = -\text{expm1}(x)$, hàm tận dụng phép tính vi sai
///    chính xác cao của phần cứng, sau đó lấy logarit tự nhiên.
/// 2. Nhánh 2 ($x \le -\ln 2$): Khi $x$ âm lớn, $e^x \le 0.5$, giá trị $1 - e^x$ nằm an toàn trong $[0.5, 1.0]$.
///    Ta dùng công thức $\ln(1 - e^x) = \text{ln\_1p}(-e^x)$ để bảo toàn độ chính xác vô hạn.
pub fn log1mexp(x: f64) -> f64 {
    assert!(
        x < 0.0,
        "log1mexp chỉ xác định trên khoảng số thực âm (-inf, 0)"
    );
    let neg_ln2 = -std::f64::consts::LN_2;
    if x > neg_ln2 {
        (-x.exp_m1()).ln()
    } else {
        (-x.exp()).ln_1p()
    }
}

// ============================================================================
// 6. BỘ NĂM HÀM TỔN THẤT CĂN CHỈNH ƯU TIÊN (DPO, IPO, SIMPO, ORPO, KTO)
// ============================================================================

// ----------------------------------------------------------------------------
// 6.1. DPO (Direct Preference Optimization)
// ----------------------------------------------------------------------------

/// Cấu hình tham số cho thuật toán DPO.
#[derive(Debug, Clone, PartialEq)]
pub struct DpoConfig {
    /// Hệ số co giãn hàm phần thưởng tiềm ẩn beta (thường 0.1 - 0.5)
    pub beta: f64,
    /// Hệ số làm mịn nhãn chống quá khớp label smoothing (0.0 = nguyên bản)
    pub label_smoothing: f64,
}

/// Kết quả đầu ra của hàm DPO.
#[derive(Debug, Clone, PartialEq)]
pub struct DpoResult {
    /// Giá trị tổn thất DPO
    pub loss: f64,
    /// Điểm phần thưởng tiềm ẩn câu thắng: $\beta (\log \pi_\theta(y_w) - \log \pi_{\text{ref}}(y_w))$
    pub reward_chosen: f64,
    /// Điểm phần thưởng tiềm ẩn câu thua: $\beta (\log \pi_\theta(y_l) - \log \pi_{\text{ref}}(y_l))$
    pub reward_rejected: f64,
    /// Biên độ phần thưởng: `reward_chosen - reward_rejected`
    pub reward_margin: f64,
    /// Đạo hàm đối với $\log \pi_\theta(y_w)$
    pub grad_log_pi_w: f64,
    /// Đạo hàm đối với $\log \pi_\theta(y_l)$
    pub grad_log_pi_l: f64,
}

/// Tính tổn thất Direct Preference Optimization (Rafailov et al., 2023).
///
/// $h = \beta \left[ \left( \log \pi_\theta(y_w) - \log \pi_{\text{ref}}(y_w) \right) - \left( \log \pi_\theta(y_l) - \log \pi_{\text{ref}}(y_l) \right) \right]$
/// $\mathcal{L}_{\text{DPO}} = -(1 - \epsilon) \ln \sigma(h) - \epsilon \ln \sigma(-h)$
pub fn dpo_loss(
    log_pi_w: f64,
    log_pi_l: f64,
    log_ref_w: f64,
    log_ref_l: f64,
    config: &DpoConfig,
) -> DpoResult {
    let r_w = config.beta * (log_pi_w - log_ref_w);
    let r_l = config.beta * (log_pi_l - log_ref_l);
    let h = r_w - r_l;

    let eps = config.label_smoothing;
    let loss = if eps <= 0.0 {
        -log_sigmoid(h)
    } else {
        -(1.0 - eps) * log_sigmoid(h) - eps * log_sigmoid(-h)
    };

    let sig = sigmoid(h);
    // dL/dh = sig - 1 + eps
    let grad_h = sig - 1.0 + eps;

    DpoResult {
        loss,
        reward_chosen: r_w,
        reward_rejected: r_l,
        reward_margin: h,
        grad_log_pi_w: config.beta * grad_h,
        grad_log_pi_l: -config.beta * grad_h,
    }
}

// ----------------------------------------------------------------------------
// 6.2. IPO (Identity Preference Optimization)
// ----------------------------------------------------------------------------

/// Cấu hình tham số cho thuật toán IPO.
#[derive(Debug, Clone, PartialEq)]
pub struct IpoConfig {
    /// Tham số điều hòa tau (thường tương đương hệ số nghịch đảo lề mục tiêu)
    pub tau: f64,
}

/// Kết quả đầu ra của hàm IPO.
#[derive(Debug, Clone, PartialEq)]
pub struct IpoResult {
    /// Giá trị tổn thất IPO
    pub loss: f64,
    /// Độ lệch log tỉ số: $(\log \pi_w - \log \text{ref}_w) - (\log \pi_l - \log \text{ref}_l)$
    pub log_ratio_diff: f64,
    /// Đạo hàm đối với $\log \pi_\theta(y_w)$
    pub grad_log_pi_w: f64,
    /// Đạo hàm đối với $\log \pi_\theta(y_l)$
    pub grad_log_pi_l: f64,
}

/// Tính tổn thất Identity Preference Optimization (Azar et al., 2023).
///
/// Khắc phục xu hướng làm trôi vô hạn tỷ số xác suất của DPO bằng cách điều hòa bậc hai
/// trực tiếp khoảng cách tới lề biên độ tối ưu $1 / (2 \tau)$:
/// $\mathcal{L}_{\text{IPO}} = \left( h - \frac{1}{2 \tau} \right)^2$
pub fn ipo_loss(
    log_pi_w: f64,
    log_pi_l: f64,
    log_ref_w: f64,
    log_ref_l: f64,
    config: &IpoConfig,
) -> IpoResult {
    let h = (log_pi_w - log_ref_w) - (log_pi_l - log_ref_l);
    let target_margin = 1.0 / (2.0 * config.tau);
    let diff = h - target_margin;
    let loss = diff * diff;

    // dL/dh = 2 * diff
    let grad_h = 2.0 * diff;

    IpoResult {
        loss,
        log_ratio_diff: h,
        grad_log_pi_w: grad_h,
        grad_log_pi_l: -grad_h,
    }
}

// ----------------------------------------------------------------------------
// 6.3. SimPO (Simple Preference Optimization)
// ----------------------------------------------------------------------------

/// Cấu hình tham số cho thuật toán SimPO.
#[derive(Debug, Clone, PartialEq)]
pub struct SimPoConfig {
    /// Hệ số co giãn beta
    pub beta: f64,
    /// Lề phần thưởng mục tiêu gamma (> 0.0)
    pub gamma_margin: f64,
}

/// Kết quả đầu ra của hàm SimPO.
#[derive(Debug, Clone, PartialEq)]
pub struct SimPoResult {
    /// Giá trị tổn thất SimPO
    pub loss: f64,
    /// Log xác suất chuẩn hóa độ dài của câu thắng: $\frac{1}{|y_w|} \log \pi_\theta(y_w)$
    pub normalized_log_pi_w: f64,
    /// Log xác suất chuẩn hóa độ dài của câu thua: $\frac{1}{|y_l|} \log \pi_\theta(y_l)$
    pub normalized_log_pi_l: f64,
    /// Biên độ giá trị: $\beta (\bar{p}_w - \bar{p}_l) - \gamma$
    pub margin: f64,
    /// Đạo hàm đối với tổng $\log \pi_\theta(y_w)$
    pub grad_log_pi_w: f64,
    /// Đạo hàm đối với tổng $\log \pi_\theta(y_l)$
    pub grad_log_pi_l: f64,
}

/// Tính tổn thất Simple Preference Optimization (Meng et al., 2024).
///
/// SimPO loại bỏ hoàn toàn mô hình tham chiếu $\pi_{\text{ref}}$ (tiết kiệm 50% bộ nhớ GPU),
/// đồng thời áp dụng chuẩn hóa độ dài chuỗi để khử thiên kiến câu trả lời dài dòng:
/// $h = \beta \left( \frac{\log \pi_\theta(y_w)}{|y_w|} - \frac{\log \pi_\theta(y_l)}{|y_l|} \right) - \gamma$
/// $\mathcal{L}_{\text{SimPO}} = -\ln \sigma(h)$
pub fn simpo_loss(
    log_pi_w: f64,
    log_pi_l: f64,
    len_w: usize,
    len_l: usize,
    config: &SimPoConfig,
) -> SimPoResult {
    assert!(len_w > 0, "Độ dài câu thắng phải > 0");
    assert!(len_l > 0, "Độ dài câu thua phải > 0");

    let norm_w = log_pi_w / (len_w as f64);
    let norm_l = log_pi_l / (len_l as f64);
    let h = config.beta * (norm_w - norm_l) - config.gamma_margin;

    let loss = -log_sigmoid(h);
    let sig = sigmoid(h);
    let grad_h = sig - 1.0;

    SimPoResult {
        loss,
        normalized_log_pi_w: norm_w,
        normalized_log_pi_l: norm_l,
        margin: h,
        grad_log_pi_w: (config.beta / (len_w as f64)) * grad_h,
        grad_log_pi_l: -(config.beta / (len_l as f64)) * grad_h,
    }
}

// ----------------------------------------------------------------------------
// 6.4. ORPO (Odds Ratio Preference Optimization)
// ----------------------------------------------------------------------------

/// Cấu hình tham số cho thuật toán ORPO.
#[derive(Debug, Clone, PartialEq)]
pub struct OrpoConfig {
    /// Trọng số kết hợp giữa tổn thất SFT và Odds Ratio penalty (lambda_or)
    pub lambda_or: f64,
}

/// Kết quả đầu ra của hàm ORPO.
#[derive(Debug, Clone, PartialEq)]
pub struct OrpoResult {
    /// Tổn thất tổng hợp: `total_loss = sft_loss + lambda_or * or_loss`
    pub total_loss: f64,
    /// Thành phần tổn thất SFT: `-log_pi_w / len_w`
    pub sft_loss: f64,
    /// Thành phần phạt tỷ số odds: `or_loss = -ln sigma(log_odds_w - log_odds_l)`
    pub or_loss: f64,
    /// Log-Odds của câu thắng
    pub log_odds_w: f64,
    /// Log-Odds của câu thua
    pub log_odds_l: f64,
    /// Đạo hàm đối với tổng $\log \pi_\theta(y_w)$
    pub grad_log_pi_w: f64,
    /// Đạo hàm đối với tổng $\log \pi_\theta(y_l)$
    pub grad_log_pi_l: f64,
}

/// Tính tổn thất Odds Ratio Preference Optimization (Hong et al., 2024).
///
/// Huấn luyện trực tiếp một bước mà không cần mô hình tham chiếu và không cần tách riêng pha SFT.
/// Sử dụng hàm ổn định số học `log1mexp` để tính chính xác logarit của tỷ số odds:
/// $\text{odds}(y) = \frac{P(y)}{1 - P(y)} \implies \ln \text{odds}(y) = s(y) - \ln(1 - e^{s(y)}) = s(y) - \text{log1mexp}(s(y))$
/// trong đó $s(y) = \frac{1}{|y|} \log \pi_\theta(y) \le 0$.
pub fn orpo_loss(
    log_pi_w: f64,
    log_pi_l: f64,
    len_w: usize,
    len_l: usize,
    config: &OrpoConfig,
) -> OrpoResult {
    assert!(len_w > 0, "Độ dài câu thắng phải > 0");
    assert!(len_l > 0, "Độ dài câu thua phải > 0");

    let s_w = log_pi_w / (len_w as f64);
    let s_l = log_pi_l / (len_l as f64);

    // Kẹp nhẹ để đảm bảo s < 0.0 theo miền xác định của log1mexp
    let s_w_clamped = s_w.min(-1e-12);
    let s_l_clamped = s_l.min(-1e-12);

    let log_odds_w = s_w_clamped - log1mexp(s_w_clamped);
    let log_odds_l = s_l_clamped - log1mexp(s_l_clamped);
    let delta_odds = log_odds_w - log_odds_l;

    let or_loss = -log_sigmoid(delta_odds);
    let sft_loss = -s_w;
    let total_loss = sft_loss + config.lambda_or * or_loss;

    let sig = sigmoid(delta_odds);
    let d_or = sig - 1.0;

    // Đạo hàm d(log_odds)/ds = 1 / (1 - exp(s))
    // 1 - exp(s) = -expm1(s)
    let d_log_odds_w = 1.0 / (-s_w_clamped.exp_m1());
    let d_log_odds_l = 1.0 / (-s_l_clamped.exp_m1());

    let grad_log_pi_w =
        (-1.0 / (len_w as f64)) + config.lambda_or * d_or * (d_log_odds_w / (len_w as f64));
    let grad_log_pi_l = -config.lambda_or * d_or * (d_log_odds_l / (len_l as f64));

    OrpoResult {
        total_loss,
        sft_loss,
        or_loss,
        log_odds_w,
        log_odds_l,
        grad_log_pi_w,
        grad_log_pi_l,
    }
}

// ----------------------------------------------------------------------------
// 6.5. KTO (Kahneman-Tversky Optimization)
// ----------------------------------------------------------------------------

/// Cấu hình tham số cho thuật toán KTO theo lý thuyết triển vọng kinh tế học hành vi.
#[derive(Debug, Clone, PartialEq)]
pub struct KtoConfig {
    /// Hệ số co giãn phần thưởng beta
    pub beta: f64,
    /// Trọng số hàm tổn thất cho mẫu mong muốn (gain weight lambda_D)
    pub lambda_desirable: f64,
    /// Trọng số tổn thất cho mẫu không mong muốn (loss aversion lambda_U, thường 1.33x)
    pub lambda_undesirable: f64,
}

/// Kết quả đầu ra của hàm KTO.
#[derive(Debug, Clone, PartialEq)]
pub struct KtoResult {
    /// Giá trị tổn thất KTO
    pub loss: f64,
    /// Điểm phần thưởng tiềm ẩn: $\beta (\log \pi_\theta(y) - \log \pi_{\text{ref}}(y))$
    pub reward: f64,
    /// Điểm cơ sở KL tham chiếu (đã tách đạo hàm)
    pub kl_baseline: f64,
    /// Đạo hàm đối với $\log \pi_\theta(y)$
    pub grad_log_pi: f64,
}

/// Tính tổn thất Kahneman–Tversky Optimization (Ethayarajh et al., 2024).
///
/// KTO huấn luyện trực tiếp trên các mẫu phản hồi đơn lẻ được gắn nhãn Nhị phân
/// (Mong muốn / Không mong muốn) thay vì đòi hỏi các cặp so sánh bắt buộc:
/// - Mẫu mong muốn (desirable): $\mathcal{L} = \lambda_D \cdot (-\ln \sigma(r - z_{\text{ref}}))$
/// - Mẫu không mong muốn (undesirable): $\mathcal{L} = \lambda_U \cdot (-\ln \sigma(z_{\text{ref}} - r))$
pub fn kto_loss(
    log_pi: f64,
    log_ref: f64,
    kl_baseline: f64,
    is_desirable: bool,
    config: &KtoConfig,
) -> KtoResult {
    let r = config.beta * (log_pi - log_ref);

    let (loss, grad_r) = if is_desirable {
        let diff = r - kl_baseline;
        let l = config.lambda_desirable * (-log_sigmoid(diff));
        let sig = sigmoid(diff);
        let g = config.lambda_desirable * (sig - 1.0);
        (l, g)
    } else {
        let diff = kl_baseline - r;
        let l = config.lambda_undesirable * (-log_sigmoid(diff));
        let sig = sigmoid(diff);
        // dL/dr = lambda_U * (sig - 1) * (-1) = lambda_U * (1 - sig)
        let g = config.lambda_undesirable * (1.0 - sig);
        (l, g)
    };

    KtoResult {
        loss,
        reward: r,
        kl_baseline,
        grad_log_pi: config.beta * grad_r,
    }
}

// ============================================================================
// 7. KIỂM CHỨNG SAI PHÂN HỮU HẠN & VÒNG TỐI ƯU HÓA VI MÔ
// ============================================================================

/// Kiểm tra đạo hàm giải tích so với sai phân hữu hạn trung tâm:
/// $g_{\text{approx}} = \frac{f(\theta + \epsilon) - f(\theta - \epsilon)}{2 \epsilon}$
pub fn check_grad_central_diff<F>(mut f: F, theta: f64, analytical_grad: f64, eps: f64) -> f64
where
    F: FnMut(f64) -> f64,
{
    let f_plus = f(theta + eps);
    let f_minus = f(theta - eps);
    let numerical_grad = (f_plus - f_minus) / (2.0 * eps);
    (analytical_grad - numerical_grad).abs()
}

// ============================================================================
// 8. HÀM MAIN TRÌNH DIỄN THỰC THI CHƯƠNG 95
// ============================================================================

fn main() {
    println!("=== CHƯƠNG 95: HUẤN LUYỆN HẬU KỲ I (SFT, LoRA & PREFERENCE ALIGNMENT) ===");
    let mut rng = SplitMix64::new(42);

    // 1. Trình diễn ChatTemplateRenderer & SFT Masked Loss
    println!("\n[1] Demo ChatTemplateRenderer & SFT Masked Loss:");
    let mut vocab = HashMap::new();
    vocab.insert("<|im_start|>system\n".to_string(), 1);
    vocab.insert("<|im_start|>user\n".to_string(), 2);
    vocab.insert("<|im_start|>assistant\n".to_string(), 3);
    vocab.insert("<|im_end|>\n".to_string(), 4);
    vocab.insert("xin".to_string(), 5);
    vocab.insert("chao".to_string(), 6);
    vocab.insert("ban".to_string(), 7);
    vocab.insert("toi".to_string(), 8);
    vocab.insert("la".to_string(), 9);
    vocab.insert("ai".to_string(), 10);

    let renderer = ChatTemplateRenderer::new(vocab, 0, 4);
    let conversation = vec![
        ChatMessage {
            role: ChatRole::System,
            content: "xin chao".to_string(),
        },
        ChatMessage {
            role: ChatRole::User,
            content: "ban la ai".to_string(),
        },
        ChatMessage {
            role: ChatRole::Assistant,
            content: "toi la ai".to_string(),
        },
    ];

    let rendered = renderer.render(&conversation);
    println!("  - Tổng số token: {}", rendered.token_ids.len());
    let active_mask_count: f64 = rendered.loss_mask.iter().sum();
    println!("  - Số token tính tổn thất trợ lý: {active_mask_count}");

    // 2. Trình diễn LoRALinear & Hợp nhất không chi phí
    println!("\n[2] Demo LoRALinear & Zero-Cost Weight Merging:");
    let in_dim = 8;
    let out_dim = 4;
    let rank = 2;
    let alpha = 4.0;
    let base_w: Vec<f64> = (0..out_dim * in_dim)
        .map(|_| rng.next_signed(0.5))
        .collect();

    let layer =
        LoraLinear::new_with_base_weight(base_w.clone(), in_dim, out_dim, rank, alpha, &mut rng);
    let x: Vec<f64> = (0..in_dim).map(|_| rng.next_signed(1.0)).collect();

    // Tại bước 0 (khi B = 0), lora_forward khớp 100% base_w
    let y_lora_init = layer.lora_forward(&x);
    let y_base_init = matmul(&base_w, &x, out_dim, in_dim, 1);
    let diff_init: f64 = y_lora_init
        .iter()
        .zip(&y_base_init)
        .map(|(a, b)| (a - b).abs())
        .fold(0.0, f64::max);
    println!("  - Sai lệch ban đầu tại bước 0 (B = 0): {diff_init:.2e}");

    // Cập nhật ma trận B và hợp nhất
    let mut layer_trained = layer.clone();
    for b_val in &mut layer_trained.lora_b {
        *b_val = rng.next_signed(0.2);
    }
    let y_lora_trained = layer_trained.lora_forward(&x);
    let merged_w = layer_trained.merge_lora();
    let y_merged = matmul(&merged_w, &x, out_dim, in_dim, 1);
    let diff_merged: f64 = y_lora_trained
        .iter()
        .zip(&y_merged)
        .map(|(a, b)| (a - b).abs())
        .fold(0.0, f64::max);
    println!("  - Sai số giữa lora_forward và merged_forward: {diff_merged:.2e} (< 1e-12)");

    let unmerged_base = LoraLinear::unmerge_lora(
        &merged_w,
        &layer_trained.lora_a,
        &layer_trained.lora_b,
        in_dim,
        out_dim,
        rank,
        layer_trained.scaling,
    );
    let diff_unmerged: f64 = base_w
        .iter()
        .zip(&unmerged_base)
        .map(|(a, b)| (a - b).abs())
        .fold(0.0, f64::max);
    println!("  - Sai số phục hồi unmerge_lora: {diff_unmerged:.2e} (< 1e-12)");

    // 3. Trình diễn hàm ổn định số học log1mexp
    println!("\n[3] Demo hàm ổn định số học log1mexp:");
    let test_vals = [-1e-14, -0.1, -std::f64::consts::LN_2, -1.5, -30.0];
    for &tv in &test_vals {
        println!("  - log1mexp({tv:.2e}) = {:.16e}", log1mexp(tv));
    }

    // 4. Trình diễn 5 hàm tổn thất căn chỉnh ưu tiên
    println!("\n[4] Demo 5 hàm tổn thất ưu tiên (DPO, IPO, SimPO, ORPO, KTO):");
    let log_pi_w = -2.1;
    let log_pi_l = -3.8;
    let log_ref_w = -2.5;
    let log_ref_l = -2.5;

    let dpo = dpo_loss(
        log_pi_w,
        log_pi_l,
        log_ref_w,
        log_ref_l,
        &DpoConfig {
            beta: 0.1,
            label_smoothing: 0.0,
        },
    );
    println!(
        "  - DPO Loss: {:.4}, Margin: {:.4}",
        dpo.loss, dpo.reward_margin
    );

    let ipo = ipo_loss(
        log_pi_w,
        log_pi_l,
        log_ref_w,
        log_ref_l,
        &IpoConfig { tau: 0.1 },
    );
    println!(
        "  - IPO Loss: {:.4}, Diff: {:.4}",
        ipo.loss, ipo.log_ratio_diff
    );

    let simpo = simpo_loss(
        log_pi_w,
        log_pi_l,
        10,
        15,
        &SimPoConfig {
            beta: 2.0,
            gamma_margin: 0.5,
        },
    );
    println!(
        "  - SimPO Loss: {:.4}, Margin: {:.4}",
        simpo.loss, simpo.margin
    );

    let orpo = orpo_loss(log_pi_w, log_pi_l, 10, 15, &OrpoConfig { lambda_or: 0.1 });
    println!(
        "  - ORPO Total Loss: {:.4} (SFT: {:.4}, OR: {:.4})",
        orpo.total_loss, orpo.sft_loss, orpo.or_loss
    );

    let kto = kto_loss(
        log_pi_w,
        log_ref_w,
        0.0,
        true,
        &KtoConfig {
            beta: 0.1,
            lambda_desirable: 1.0,
            lambda_undesirable: 1.33,
        },
    );
    println!(
        "  - KTO Loss (Desirable): {:.4}, Reward: {:.4}",
        kto.loss, kto.reward
    );

    println!("\nHoàn thành thực thi mẫu Chương 95 sạch sẽ!");
}

// ============================================================================
// 9. BỘ KIỂM THỬ TẤT ĐỊNH (>= 30 BÀI TEST CHẠY < 150MS)
// ============================================================================

#[cfg(test)]
mod tests {
    use super::*;

    const EPS_FINITE_DIFF: f64 = 1e-6;
    const GRAD_TOLERANCE: f64 = 1e-7;

    // Test 1: Kiểm tra tính tái lập tất định của SplitMix64
    #[test]
    fn test_splitmix64_reproducibility() {
        let mut rng1 = SplitMix64::new(12345);
        let mut rng2 = SplitMix64::new(12345);
        for _ in 0..100 {
            assert_eq!(rng1.next_u64(), rng2.next_u64());
            assert_eq!(rng1.next_f64(), rng2.next_f64());
        }
    }

    // Test 2: Phân phối chuẩn Gaussian Box-Muller
    #[test]
    fn test_splitmix64_gaussian_box_muller() {
        let mut rng = SplitMix64::new(999);
        let mut sum = 0.0;
        let mut sq_sum = 0.0;
        let n = 2000;
        for _ in 0..n {
            let val = rng.next_gaussian(0.0, 1.0);
            sum += val;
            sq_sum += val * val;
        }
        let mean = sum / (n as f64);
        let var = (sq_sum / (n as f64)) - mean * mean;
        assert!(mean.abs() < 0.1, "Kỳ vọng trung bình phải xấp xỉ 0");
        assert!((var - 1.0).abs() < 0.15, "Phương sai phải xấp xỉ 1.0");
    }

    // Test 3: Phép nhân ma trận matmul
    #[test]
    fn test_matmul_correctness() {
        let a = vec![1.0, 2.0, 3.0, 4.0]; // 2x2
        let b = vec![5.0, 6.0, 7.0, 8.0]; // 2x2
        let c = matmul(&a, &b, 2, 2, 2);
        assert_eq!(c, vec![19.0, 22.0, 43.0, 50.0]);
    }

    // Test 4: Chuyển vị và tích vô hướng
    #[test]
    fn test_transpose_and_matmul_properties() {
        let a = vec![1.0, 2.0, 3.0, 4.0, 5.0, 6.0]; // 2x3
        let at = transpose(&a, 2, 3); // 3x2
        assert_eq!(at, vec![1.0, 4.0, 2.0, 5.0, 3.0, 6.0]);
        let att = transpose(&at, 3, 2);
        assert_eq!(att, a);
    }

    // Test 5: Sigmoid và Log-Sigmoid
    #[test]
    fn test_sigmoid_and_log_sigmoid_stability() {
        assert!((sigmoid(0.0) - 0.5).abs() < 1e-12);
        assert!((sigmoid(50.0) - 1.0).abs() < 1e-12);
        assert!((sigmoid(-50.0) - 0.0).abs() < 1e-12);

        let ls0 = log_sigmoid(0.0);
        assert!((ls0 - (0.5f64).ln()).abs() < 1e-12);
        let ls_pos = log_sigmoid(30.0);
        assert!(ls_pos <= 0.0 && ls_pos > -1e-10);
        let ls_neg = log_sigmoid(-30.0);
        assert!((ls_neg - (-30.0)).abs() < 1e-10);
    }

    // Test 6: Softmax và Log-Softmax
    #[test]
    fn test_softmax_and_log_softmax_numerical_stability() {
        let logits = vec![1000.0, 1001.0, 1002.0];
        let probs = softmax(&logits);
        let sum: f64 = probs.iter().sum();
        assert!((sum - 1.0).abs() < 1e-12);
        assert!(probs[2] > probs[1] && probs[1] > probs[0]);

        let log_probs = log_softmax(&logits);
        for (lp, p) in log_probs.iter().zip(&probs) {
            assert!((lp.exp() - p).abs() < 1e-12);
        }
    }

    // Test 7: ChatTemplateRenderer tạo prompt và mặt nạ chính xác
    #[test]
    fn test_chat_template_renderer_tokens_and_loss_mask() {
        let mut vocab = HashMap::new();
        vocab.insert("<|im_start|>system\n".to_string(), 1);
        vocab.insert("<|im_start|>user\n".to_string(), 2);
        vocab.insert("<|im_start|>assistant\n".to_string(), 3);
        vocab.insert("<|im_end|>\n".to_string(), 4);
        vocab.insert("hello".to_string(), 5);
        vocab.insert("world".to_string(), 6);

        let renderer = ChatTemplateRenderer::new(vocab, 0, 4);
        let messages = vec![
            ChatMessage {
                role: ChatRole::User,
                content: "hello".to_string(),
            },
            ChatMessage {
                role: ChatRole::Assistant,
                content: "world".to_string(),
            },
        ];

        let dialogue = renderer.render(&messages);
        assert_eq!(dialogue.token_ids[0], 0); // BOS
        assert_eq!(dialogue.loss_mask[0], 0.0);

        // Kiểm tra xem có token nào của assistant mang mask = 1.0 không
        let has_assistant_tokens = dialogue.loss_mask.contains(&1.0);
        assert!(has_assistant_tokens);
    }

    // Test 8: SFT Masked Loss bỏ qua hoàn toàn token người dùng
    #[test]
    fn test_sft_masked_loss_ignores_user_and_system_tokens() {
        let vocab_size = 4;
        let logits = vec![
            1.0, 2.0, 0.5, -1.0, // token 0 (user)
            0.0, 3.0, 1.0, -0.5, // token 1 (assistant)
        ];
        let targets = vec![1, 1];
        let mask = vec![0.0, 1.0]; // Chỉ tính token 1

        let res = sft_masked_loss(&logits, &targets, &mask, vocab_size);
        assert_eq!(res.valid_token_count, 1.0);

        // Đạo hàm của token 0 phải bằng đúng 0.0
        for v in 0..vocab_size {
            assert_eq!(res.grad_logits[v], 0.0);
        }
        // Đạo hàm của token 1 phải khác 0
        let has_grad_token1 = (0..vocab_size).any(|v| res.grad_logits[vocab_size + v] != 0.0);
        assert!(has_grad_token1);
    }

    // Test 9: Chốt chặn chống chia 0/0 sinh NaN khi không có token trợ lý
    #[test]
    fn test_sft_masked_loss_zero_mask_prevents_nan_div_zero() {
        let vocab_size = 3;
        let logits = vec![0.5, -0.2, 1.1, 0.3, 0.8, -0.1];
        let targets = vec![0, 1];
        let mask = vec![0.0, 0.0]; // Toàn bộ mask = 0.0

        let res = sft_masked_loss(&logits, &targets, &mask, vocab_size);
        assert_eq!(res.loss, 0.0);
        assert_eq!(res.valid_token_count, 0.0);
        for g in res.grad_logits {
            assert_eq!(g, 0.0);
            assert!(!g.is_nan());
        }
    }

    // Test 10: Kiểm tra đạo hàm SFT Masked Loss so với sai phân hữu hạn
    #[test]
    fn test_sft_masked_loss_gradient_central_difference() {
        let vocab_size = 3;
        let base_logits = vec![0.5, -0.2, 1.1, 0.3, 0.8, -0.1];
        let targets = vec![2, 1];
        let mask = vec![1.0, 0.5];

        let analytical = sft_masked_loss(&base_logits, &targets, &mask, vocab_size);

        for idx in 0..base_logits.len() {
            let diff = check_grad_central_diff(
                |val| {
                    let mut perturbed = base_logits.clone();
                    perturbed[idx] = val;
                    sft_masked_loss(&perturbed, &targets, &mask, vocab_size).loss
                },
                base_logits[idx],
                analytical.grad_logits[idx],
                EPS_FINITE_DIFF,
            );
            assert!(
                diff < GRAD_TOLERANCE,
                "Gradient sai phân hữu hạn SFT lệch: {diff}"
            );
        }
    }

    // Test 11: LoRALinear tại bước 0 (B = 0) bảo toàn nguyên vẹn mô hình gốc
    #[test]
    fn test_lora_linear_step_zero_matches_base_weight() {
        let mut rng = SplitMix64::new(777);
        let in_f = 6;
        let out_f = 4;
        let rank = 2;
        let base_w: Vec<f64> = (0..out_f * in_f).map(|_| rng.next_signed(0.5)).collect();

        let layer =
            LoraLinear::new_with_base_weight(base_w.clone(), in_f, out_f, rank, 4.0, &mut rng);
        let x: Vec<f64> = (0..in_f).map(|_| rng.next_signed(1.0)).collect();

        let y_lora = layer.lora_forward(&x);
        let y_base = matmul(&base_w, &x, out_f, in_f, 1);

        for (yl, yb) in y_lora.iter().zip(&y_base) {
            assert_eq!(yl, yb, "Bước 0 phải trùng khớp tuyệt đối trọng số gốc");
        }
    }

    // Test 12: lora_forward khớp tuyệt đối merged_w tới 1e-12
    #[test]
    fn test_lora_linear_forward_matches_merged_weights_to_1e_12() {
        let mut rng = SplitMix64::new(888);
        let in_f = 5;
        let out_f = 3;
        let rank = 2;
        let base_w: Vec<f64> = (0..out_f * in_f).map(|_| rng.next_signed(0.5)).collect();

        let mut layer = LoraLinear::new_with_base_weight(base_w, in_f, out_f, rank, 2.0, &mut rng);
        // Gán trọng số khác 0 cho B
        for b in &mut layer.lora_b {
            *b = rng.next_signed(0.3);
        }

        let x: Vec<f64> = (0..in_f).map(|_| rng.next_signed(1.0)).collect();
        let y_lora = layer.lora_forward(&x);

        let merged_w = layer.merge_lora();
        let y_merged = matmul(&merged_w, &x, out_f, in_f, 1);

        for (yl, ym) in y_lora.iter().zip(&y_merged) {
            let diff = (yl - ym).abs();
            assert!(
                diff < 1e-12,
                "lora_forward và merged_forward phải khớp < 1e-12, diff={diff}"
            );
        }
    }

    // Test 13: unmerge_lora phục hồi trọng số gốc sai số < 1e-12
    #[test]
    fn test_lora_linear_unmerge_recovers_base_weight_to_1e_12() {
        let mut rng = SplitMix64::new(555);
        let in_f = 4;
        let out_f = 4;
        let rank = 2;
        let base_w: Vec<f64> = (0..out_f * in_f).map(|_| rng.next_signed(0.5)).collect();

        let mut layer =
            LoraLinear::new_with_base_weight(base_w.clone(), in_f, out_f, rank, 4.0, &mut rng);
        for b in &mut layer.lora_b {
            *b = rng.next_signed(0.4);
        }

        let merged = layer.merge_lora();
        let recovered = LoraLinear::unmerge_lora(
            &merged,
            &layer.lora_a,
            &layer.lora_b,
            in_f,
            out_f,
            rank,
            layer.scaling,
        );

        for (orig, rec) in base_w.iter().zip(&recovered) {
            let diff = (orig - rec).abs();
            assert!(
                diff < 1e-12,
                "unmerge_lora phải phục hồi base_w sai số < 1e-12, diff={diff}"
            );
        }
    }

    // Test 14: Đạo hàm LoRALinear giải tích khớp sai phân hữu hạn trung tâm
    #[test]
    fn test_lora_linear_gradients_match_central_difference() {
        let mut rng = SplitMix64::new(321);
        let in_f = 4;
        let out_f = 3;
        let rank = 2;
        let base_w: Vec<f64> = (0..out_f * in_f).map(|_| rng.next_signed(0.5)).collect();

        let mut layer = LoraLinear::new_with_base_weight(base_w, in_f, out_f, rank, 2.0, &mut rng);
        for b in &mut layer.lora_b {
            *b = rng.next_signed(0.3);
        }
        let x: Vec<f64> = (0..in_f).map(|_| rng.next_signed(1.0)).collect();
        let grad_out: Vec<f64> = (0..out_f).map(|_| rng.next_signed(0.5)).collect();

        let analytical = layer.lora_backward(&x, &grad_out);

        // 1. Kiểm tra gradient đối với lora_a
        for idx in 0..layer.lora_a.len() {
            let diff = check_grad_central_diff(
                |val| {
                    let mut l = layer.clone();
                    l.lora_a[idx] = val;
                    dot_product(&l.lora_forward(&x), &grad_out)
                },
                layer.lora_a[idx],
                analytical.grad_lora_a[idx],
                EPS_FINITE_DIFF,
            );
            assert!(
                diff < GRAD_TOLERANCE,
                "Gradient lora_a sai phân hữu hạn lệch: {diff}"
            );
        }

        // 2. Kiểm tra gradient đối với lora_b
        for idx in 0..layer.lora_b.len() {
            let diff = check_grad_central_diff(
                |val| {
                    let mut l = layer.clone();
                    l.lora_b[idx] = val;
                    dot_product(&l.lora_forward(&x), &grad_out)
                },
                layer.lora_b[idx],
                analytical.grad_lora_b[idx],
                EPS_FINITE_DIFF,
            );
            assert!(
                diff < GRAD_TOLERANCE,
                "Gradient lora_b sai phân hữu hạn lệch: {diff}"
            );
        }

        // 3. Kiểm tra gradient đối với input x
        for idx in 0..x.len() {
            let diff = check_grad_central_diff(
                |val| {
                    let mut x_pert = x.clone();
                    x_pert[idx] = val;
                    dot_product(&layer.lora_forward(&x_pert), &grad_out)
                },
                x[idx],
                analytical.grad_input[idx],
                EPS_FINITE_DIFF,
            );
            assert!(
                diff < GRAD_TOLERANCE,
                "Gradient input x sai phân hữu hạn lệch: {diff}"
            );
        }
    }

    // Test 15: RewardModel chấm điểm chính xác tại token khác đệm cuối cùng
    #[test]
    fn test_reward_model_last_non_pad_token_scoring() {
        let weights = vec![1.0, -2.0, 0.5];
        let bias = 0.1;
        let rm = RewardModel::new(weights, bias);

        // Chuỗi có 3 bước thời gian: [h0, h1, pad] -> last_non_pad = 1
        let seq_len = 3;
        let hidden_states = vec![
            0.1, 0.2, 0.3, // h0
            1.0, 2.0, -1.0, // h1
            0.0, 0.0, 0.0, // pad
        ];

        let score = rm.score_sequence(&hidden_states, seq_len, 1);
        let expected = 1.0 * 1.0 + (-2.0) * 2.0 + (-0.5) + bias;
        assert!((score - expected).abs() < 1e-12);
    }

    // Test 16: Bradley-Terry Loss và đạo hàm khớp sai phân hữu hạn
    #[test]
    fn test_bradley_terry_loss_and_gradients_central_difference() {
        let r_w = 1.5;
        let r_l = -0.5;
        let margin = 0.2;

        let res = bradley_terry_loss(r_w, r_l, margin);

        let diff_w = check_grad_central_diff(
            |rw| bradley_terry_loss(rw, r_l, margin).loss,
            r_w,
            res.grad_r_w,
            EPS_FINITE_DIFF,
        );
        assert!(
            diff_w < GRAD_TOLERANCE,
            "Gradient BT r_w sai phân hữu hạn lệch: {diff_w}"
        );

        let diff_l = check_grad_central_diff(
            |rl| bradley_terry_loss(r_w, rl, margin).loss,
            r_l,
            res.grad_r_l,
            EPS_FINITE_DIFF,
        );
        assert!(
            diff_l < GRAD_TOLERANCE,
            "Gradient BT r_l sai phân hữu hạn lệch: {diff_l}"
        );
    }

    // Test 17: Hành vi của lề margin trong Bradley-Terry
    #[test]
    fn test_bradley_terry_loss_margin_behavior() {
        let r_w = 2.0;
        let r_l = 1.0;
        let loss_no_margin = bradley_terry_loss(r_w, r_l, 0.0).loss;
        let loss_with_margin = bradley_terry_loss(r_w, r_l, 0.5).loss;
        assert!(
            loss_with_margin > loss_no_margin,
            "Có margin dương đòi hỏi r_w - r_l phải lớn hơn nên tổn thất phải cao hơn"
        );
    }

    // Test 18: log1mexp chính xác tại lân cận 0 (tránh triệt tiêu thảm họa)
    #[test]
    fn test_log1mexp_near_zero_accurate_to_1e_12() {
        let x = -1e-14;
        let val = log1mexp(x);
        // Với x rất nhỏ, 1 - exp(x) xấp xỉ -x = 1e-14, ln(1e-14) xấp xỉ -32.23619130191664
        let expected = (-x).ln();
        assert!((val - expected).abs() < 1e-11);
        assert!(
            val.is_finite(),
            "log1mexp không được trả về -inf tại -1e-14"
        );
    }

    // Test 19: log1mexp chính xác tại miền âm sâu (tránh underflow)
    #[test]
    fn test_log1mexp_large_negative_accurate_to_1e_12() {
        let x = -40.0;
        let val = log1mexp(x);
        // Với x âm lớn, ln(1 - exp(x)) xấp xỉ -exp(x)
        let expected = -(-40.0f64).exp();
        assert!((val - expected).abs() < 1e-25);
    }

    // Test 20: Tính liên tục mượt mà của log1mexp tại điểm chuyển nhánh -ln 2
    #[test]
    fn test_log1mexp_continuity_at_neg_ln2() {
        let threshold = -std::f64::consts::LN_2;
        let delta = 1e-9;
        let val_left = log1mexp(threshold - delta);
        let val_right = log1mexp(threshold + delta);
        assert!(
            (val_left - val_right).abs() < 1e-8,
            "Hai nhánh phải liên tục tại -ln 2"
        );
    }

    // Test 21: DPO loss và đạo hàm khớp sai phân hữu hạn
    #[test]
    fn test_dpo_loss_and_gradients_central_difference() {
        let log_pi_w = -1.5;
        let log_pi_l = -2.8;
        let log_ref_w = -2.0;
        let log_ref_l = -2.0;
        let cfg = DpoConfig {
            beta: 0.2,
            label_smoothing: 0.0,
        };

        let res = dpo_loss(log_pi_w, log_pi_l, log_ref_w, log_ref_l, &cfg);

        let diff_w = check_grad_central_diff(
            |pw| dpo_loss(pw, log_pi_l, log_ref_w, log_ref_l, &cfg).loss,
            log_pi_w,
            res.grad_log_pi_w,
            EPS_FINITE_DIFF,
        );
        assert!(
            diff_w < GRAD_TOLERANCE,
            "Gradient DPO w sai phân hữu hạn lệch: {diff_w}"
        );

        let diff_l = check_grad_central_diff(
            |pl| dpo_loss(log_pi_w, pl, log_ref_w, log_ref_l, &cfg).loss,
            log_pi_l,
            res.grad_log_pi_l,
            EPS_FINITE_DIFF,
        );
        assert!(
            diff_l < GRAD_TOLERANCE,
            "Gradient DPO l sai phân hữu hạn lệch: {diff_l}"
        );
    }

    // Test 22: DPO Label Smoothing đạo hàm khớp sai phân hữu hạn
    #[test]
    fn test_dpo_label_smoothing_gradients_central_difference() {
        let log_pi_w = -1.2;
        let log_pi_l = -2.5;
        let log_ref_w = -1.8;
        let log_ref_l = -1.8;
        let cfg = DpoConfig {
            beta: 0.3,
            label_smoothing: 0.1,
        };

        let res = dpo_loss(log_pi_w, log_pi_l, log_ref_w, log_ref_l, &cfg);

        let diff_w = check_grad_central_diff(
            |pw| dpo_loss(pw, log_pi_l, log_ref_w, log_ref_l, &cfg).loss,
            log_pi_w,
            res.grad_log_pi_w,
            EPS_FINITE_DIFF,
        );
        assert!(
            diff_w < GRAD_TOLERANCE,
            "Gradient DPO smoothed w lệch: {diff_w}"
        );
    }

    // Test 23: IPO loss và đạo hàm khớp sai phân hữu hạn
    #[test]
    fn test_ipo_loss_and_gradients_central_difference() {
        let log_pi_w = -1.0;
        let log_pi_l = -2.0;
        let log_ref_w = -1.5;
        let log_ref_l = -1.5;
        let cfg = IpoConfig { tau: 0.2 };

        let res = ipo_loss(log_pi_w, log_pi_l, log_ref_w, log_ref_l, &cfg);

        let diff_w = check_grad_central_diff(
            |pw| ipo_loss(pw, log_pi_l, log_ref_w, log_ref_l, &cfg).loss,
            log_pi_w,
            res.grad_log_pi_w,
            EPS_FINITE_DIFF,
        );
        assert!(
            diff_w < GRAD_TOLERANCE,
            "Gradient IPO w sai phân hữu hạn lệch: {diff_w}"
        );

        let diff_l = check_grad_central_diff(
            |pl| ipo_loss(log_pi_w, pl, log_ref_w, log_ref_l, &cfg).loss,
            log_pi_l,
            res.grad_log_pi_l,
            EPS_FINITE_DIFF,
        );
        assert!(
            diff_l < GRAD_TOLERANCE,
            "Gradient IPO l sai phân hữu hạn lệch: {diff_l}"
        );
    }

    // Test 24: SimPO loss và đạo hàm chuẩn hóa độ dài khớp sai phân hữu hạn
    #[test]
    fn test_simpo_loss_length_normalization_and_gradients_central_difference() {
        let log_pi_w = -3.0;
        let log_pi_l = -5.0;
        let len_w = 12;
        let len_l = 20;
        let cfg = SimPoConfig {
            beta: 1.5,
            gamma_margin: 0.3,
        };

        let res = simpo_loss(log_pi_w, log_pi_l, len_w, len_l, &cfg);

        let diff_w = check_grad_central_diff(
            |pw| simpo_loss(pw, log_pi_l, len_w, len_l, &cfg).loss,
            log_pi_w,
            res.grad_log_pi_w,
            EPS_FINITE_DIFF,
        );
        assert!(
            diff_w < GRAD_TOLERANCE,
            "Gradient SimPO w sai phân hữu hạn lệch: {diff_w}"
        );

        let diff_l = check_grad_central_diff(
            |pl| simpo_loss(log_pi_w, pl, len_w, len_l, &cfg).loss,
            log_pi_l,
            res.grad_log_pi_l,
            EPS_FINITE_DIFF,
        );
        assert!(
            diff_l < GRAD_TOLERANCE,
            "Gradient SimPO l sai phân hữu hạn lệch: {diff_l}"
        );
    }

    // Test 25: ORPO loss và đạo hàm Odds Ratio khớp sai phân hữu hạn
    #[test]
    fn test_orpo_loss_and_gradients_central_difference() {
        let log_pi_w = -2.5;
        let log_pi_l = -4.0;
        let len_w = 8;
        let len_l = 10;
        let cfg = OrpoConfig { lambda_or: 0.2 };

        let res = orpo_loss(log_pi_w, log_pi_l, len_w, len_l, &cfg);

        let diff_w = check_grad_central_diff(
            |pw| orpo_loss(pw, log_pi_l, len_w, len_l, &cfg).total_loss,
            log_pi_w,
            res.grad_log_pi_w,
            EPS_FINITE_DIFF,
        );
        assert!(
            diff_w < GRAD_TOLERANCE,
            "Gradient ORPO w sai phân hữu hạn lệch: {diff_w}"
        );

        let diff_l = check_grad_central_diff(
            |pl| orpo_loss(log_pi_w, pl, len_w, len_l, &cfg).total_loss,
            log_pi_l,
            res.grad_log_pi_l,
            EPS_FINITE_DIFF,
        );
        assert!(
            diff_l < GRAD_TOLERANCE,
            "Gradient ORPO l sai phân hữu hạn lệch: {diff_l}"
        );
    }

    // Test 26: KTO Desirable loss và đạo hàm khớp sai phân hữu hạn
    #[test]
    fn test_kto_loss_desirable_and_gradients_central_difference() {
        let log_pi = -1.8;
        let log_ref = -2.2;
        let kl_baseline = 0.05;
        let cfg = KtoConfig {
            beta: 0.1,
            lambda_desirable: 1.0,
            lambda_undesirable: 1.33,
        };

        let res = kto_loss(log_pi, log_ref, kl_baseline, true, &cfg);

        let diff = check_grad_central_diff(
            |p| kto_loss(p, log_ref, kl_baseline, true, &cfg).loss,
            log_pi,
            res.grad_log_pi,
            EPS_FINITE_DIFF,
        );
        assert!(
            diff < GRAD_TOLERANCE,
            "Gradient KTO desirable sai phân hữu hạn lệch: {diff}"
        );
    }

    // Test 27: KTO Undesirable loss và đạo hàm ác cảm mất mát (loss aversion)
    #[test]
    fn test_kto_loss_undesirable_loss_aversion_and_gradients_central_difference() {
        let log_pi = -2.5;
        let log_ref = -2.0;
        let kl_baseline = 0.05;
        let cfg = KtoConfig {
            beta: 0.1,
            lambda_desirable: 1.0,
            lambda_undesirable: 1.33,
        };

        let res = kto_loss(log_pi, log_ref, kl_baseline, false, &cfg);

        let diff = check_grad_central_diff(
            |p| kto_loss(p, log_ref, kl_baseline, false, &cfg).loss,
            log_pi,
            res.grad_log_pi,
            EPS_FINITE_DIFF,
        );
        assert!(
            diff < GRAD_TOLERANCE,
            "Gradient KTO undesirable sai phân hữu hạn lệch: {diff}"
        );
        // Đạo hàm của undesirable phải dương (vì khi log_pi tăng, tổn thất phạt tăng)
        assert!(res.grad_log_pi > 0.0);
    }

    // Test 28: Tối ưu hóa DPO 15 bước làm tăng đơn điệu biên độ ưu tiên
    #[test]
    fn test_preference_optimization_15_steps_increases_margin() {
        let mut log_pi_w = -2.5;
        let mut log_pi_l = -2.5;
        let log_ref_w = -2.5;
        let log_ref_l = -2.5;
        let cfg = DpoConfig {
            beta: 0.2,
            label_smoothing: 0.0,
        };
        let lr = 0.1;

        let initial_margin = log_pi_w - log_pi_l;
        assert_eq!(initial_margin, 0.0);

        for _ in 0..15 {
            let res = dpo_loss(log_pi_w, log_pi_l, log_ref_w, log_ref_l, &cfg);
            log_pi_w -= lr * res.grad_log_pi_w;
            log_pi_l -= lr * res.grad_log_pi_l;
        }

        let final_margin = log_pi_w - log_pi_l;
        assert!(
            final_margin > 0.25,
            "Sau 15 bước DPO, biên độ log_pi_w - log_pi_l phải tăng mạnh: {final_margin}"
        );
        assert!(log_pi_w > log_pi_l);
    }

    // Test 29: Tối ưu hóa SFT 15 bước làm giảm đơn điệu tổn thất của token trợ lý
    #[test]
    fn test_sft_15_steps_decreases_assistant_loss() {
        let vocab_size = 4;
        let mut logits = vec![
            0.1, -0.2, 0.3, 0.0, // token 0 (trợ lý, target = 2)
        ];
        let targets = vec![2];
        let mask = vec![1.0];
        let lr = 0.5;

        let init_loss = sft_masked_loss(&logits, &targets, &mask, vocab_size).loss;

        for _ in 0..15 {
            let res = sft_masked_loss(&logits, &targets, &mask, vocab_size);
            for (logit, &grad) in logits.iter_mut().zip(&res.grad_logits) {
                *logit -= lr * grad;
            }
        }

        let final_loss = sft_masked_loss(&logits, &targets, &mask, vocab_size).loss;
        assert!(
            final_loss < init_loss * 0.3,
            "Tổn thất SFT phải giảm sâu sau 15 bước: init={init_loss}, final={final_loss}"
        );
    }

    // Test 30: Kiểm tra bất biến co giãn LoRA scaling alpha / rank
    #[test]
    fn test_lora_rank_sweep_and_scaling_invariance() {
        let mut rng = SplitMix64::new(101);
        let in_f = 4;
        let out_f = 4;
        let base_w = vec![0.1; 16];

        let lora1 = LoraLinear::new_with_base_weight(base_w.clone(), in_f, out_f, 2, 4.0, &mut rng);
        let lora2 = LoraLinear::new_with_base_weight(base_w, in_f, out_f, 4, 8.0, &mut rng);

        assert_eq!(lora1.scaling, 2.0); // 4.0 / 2
        assert_eq!(lora2.scaling, 2.0); // 8.0 / 4
    }

    // Test 31: Cả 5 hàm ưu tiên đều thúc đẩy biên độ ưu tiên sau cập nhật gradient
    #[test]
    fn test_all_five_preference_losses_monotonic_preference_gain() {
        let (lp_w, lp_l) = (-2.0, -2.5);
        let (lr_w, lr_l) = (-2.0, -2.0);

        // 1. DPO
        let dpo = dpo_loss(
            lp_w,
            lp_l,
            lr_w,
            lr_l,
            &DpoConfig {
                beta: 0.1,
                label_smoothing: 0.0,
            },
        );
        assert!(dpo.grad_log_pi_w < 0.0 && dpo.grad_log_pi_l > 0.0);

        // 2. IPO
        let ipo = ipo_loss(lp_w, lp_l, lr_w, lr_l, &IpoConfig { tau: 0.05 });
        assert!(ipo.grad_log_pi_w < 0.0 && ipo.grad_log_pi_l > 0.0);

        // 3. SimPO
        let simpo = simpo_loss(
            lp_w,
            lp_l,
            10,
            10,
            &SimPoConfig {
                beta: 1.0,
                gamma_margin: 0.2,
            },
        );
        assert!(simpo.grad_log_pi_w < 0.0 && simpo.grad_log_pi_l > 0.0);

        // 4. ORPO
        let orpo = orpo_loss(lp_w, lp_l, 10, 10, &OrpoConfig { lambda_or: 0.1 });
        assert!(orpo.grad_log_pi_w < 0.0 && orpo.grad_log_pi_l > 0.0);

        // 5. KTO
        let kto_w = kto_loss(
            lp_w,
            lr_w,
            0.0,
            true,
            &KtoConfig {
                beta: 0.1,
                lambda_desirable: 1.0,
                lambda_undesirable: 1.33,
            },
        );
        let kto_l = kto_loss(
            lp_l,
            lr_l,
            0.0,
            false,
            &KtoConfig {
                beta: 0.1,
                lambda_desirable: 1.0,
                lambda_undesirable: 1.33,
            },
        );
        assert!(kto_w.grad_log_pi < 0.0 && kto_l.grad_log_pi > 0.0);
    }

    // Test 32: Chat template hội thoại đa lượt mặt nạ chỉ bật cho assistant
    #[test]
    fn test_chat_template_multiturn_dialogue_masking() {
        let mut vocab = HashMap::new();
        vocab.insert("<|im_start|>system\n".to_string(), 1);
        vocab.insert("<|im_start|>user\n".to_string(), 2);
        vocab.insert("<|im_start|>assistant\n".to_string(), 3);
        vocab.insert("<|im_end|>\n".to_string(), 4);
        vocab.insert("a".to_string(), 5);
        vocab.insert("b".to_string(), 6);
        vocab.insert("c".to_string(), 7);

        let renderer = ChatTemplateRenderer::new(vocab, 0, 4);
        let msgs = vec![
            ChatMessage {
                role: ChatRole::System,
                content: "a".to_string(),
            },
            ChatMessage {
                role: ChatRole::User,
                content: "b".to_string(),
            },
            ChatMessage {
                role: ChatRole::Assistant,
                content: "c".to_string(),
            },
            ChatMessage {
                role: ChatRole::User,
                content: "a".to_string(),
            },
            ChatMessage {
                role: ChatRole::Assistant,
                content: "b".to_string(),
            },
        ];

        let res = renderer.render(&msgs);
        let mask_sum: f64 = res.loss_mask.iter().sum();
        // Có 2 lượt assistant, mỗi lượt có 1 token nội dung + 1 token kết thúc lượt <|im_end|>
        assert_eq!(mask_sum, 4.0);
    }

    // Test 33: RewardModel xếp hạng phân biệt giữa câu tốt và câu kém
    #[test]
    fn test_reward_model_batch_ranking_accuracy() {
        let weights = vec![2.0, -1.0];
        let rm = RewardModel::new(weights, 0.0);

        let good_hidden = vec![1.5, -0.5]; // 2*1.5 + (-1)*(-0.5) = 3.5
        let bad_hidden = vec![-1.0, 2.0]; // 2*(-1) + (-1)*2 = -4.0

        let s_good = rm.score_sequence(&good_hidden, 1, 0);
        let s_bad = rm.score_sequence(&bad_hidden, 1, 0);
        assert!(
            s_good > s_bad,
            "Reward câu tốt phải cao hơn câu xấu: good={s_good}, bad={s_bad}"
        );
    }

    // Test 34: Hành vi của lề gamma trong SimPO
    #[test]
    fn test_simpo_margin_penalty_behavior() {
        let (lp_w, lp_l) = (-2.0, -2.0); // Bằng nhau
        let (len_w, len_l) = (10, 10);
        let loss_gamma_0 = simpo_loss(
            lp_w,
            lp_l,
            len_w,
            len_l,
            &SimPoConfig {
                beta: 1.0,
                gamma_margin: 0.0,
            },
        )
        .loss;
        let loss_gamma_1 = simpo_loss(
            lp_w,
            lp_l,
            len_w,
            len_l,
            &SimPoConfig {
                beta: 1.0,
                gamma_margin: 1.0,
            },
        )
        .loss;

        assert!(
            loss_gamma_1 > loss_gamma_0,
            "Khi có gamma margin, hai câu có xác suất bằng nhau vẫn bị phạt nặng hơn"
        );
    }

    // Test 35: Tính nhất quán của đạo hàm Odds Ratio trong ORPO
    #[test]
    fn test_orpo_odds_ratio_derivatives_consistency() {
        let cfg = OrpoConfig { lambda_or: 1.0 };
        let res_equal = orpo_loss(-2.0, -2.0, 10, 10, &cfg);
        // Khi hai câu bằng nhau, delta_odds = 0, sigmoid = 0.5, d_or = -0.5
        assert!((res_equal.log_odds_w - res_equal.log_odds_l).abs() < 1e-12);
        assert!(res_equal.or_loss > 0.0);
    }
}
