//! # Chương 96: Huấn Luyện Hậu Kỳ II — Học Tăng Cường Có Phần Thưởng Kiểm Chứng (RLVR)
//!
//! Crate `code/ch96`: Cài đặt toàn diện động cơ học tăng cường hậu kỳ cho mô hình ngôn ngữ
//! lớn (LLM Post-Training Reinforcement Learning) bằng 100% Rust 2024 thuần `std`,
//! không phụ thuộc thư viện ngoài (`[dependencies]` rỗng 100%).
//!
//! ## Các Trụ Cột Kỹ Thuật Cốt Lõi:
//! 1. **Mạng Actor-Critic Cổ Điển**:
//!    - Kiến trúc thân ẩn chia sẻ (`forward_hidden`) với đầu chính sách (`Actor`) và đầu giá trị (`Critic`).
//!    - Đầu Critic được khởi tạo chuẩn xác bằng `0.0` để giữ giá trị kỳ vọng ban đầu không thiên lệch.
//! 2. **Ước Lượng Lợi Thế Tổng Quát (GAE - Generalized Advantage Estimation)**:
//!    - Quét ngược thời gian tính sai số chênh lệch thời gian TD ($\delta_t$) và lợi thế tổng quát ($A_t$).
//!    - Chứng minh chặt chẽ tính bất biến: $\lambda = 0$ suy biến thành TD 1-bước, $\lambda = 1$ suy biến thành Monte-Carlo.
//! 3. **Tối Ưu Hóa Chính Sách Kẹp PPO (Proximal Policy Optimization)**:
//!    - Mục tiêu chính sách kẹp Surrogate $L^{\text{CLIP}}$, hàm tổn thất giá trị kẹp $L^{\text{VF}}$, và thưởng entropy $S[\pi]$.
//! 4. **Phạt Phân Kỳ KL Không Âm Mọi Nơi $k_3$ của John Schulman**:
//!    - Đảm bảo $D_{k3}(\pi, \pi_{\text{ref}}) = \exp(d) - d - 1 \ge 0$ với $d = \log \pi_{\text{ref}} - \log \pi$.
//!    - Đạo hàm giải tích xuôi/ngược khớp tuyệt đối sai phân hữu hạn trung tâm sai số $< 10^{-8}$.
//! 5. **Bộ Chấm Điểm Toán Học Tất Định (`MathAnswerVerifier`)**:
//!    - Bóc tách và thẩm định cú pháp thẻ `<think>...</think><answer>...</answer>`.
//!    - Bảng điểm 4 tầng minh bạch: `1.2` (đúng toán + chuẩn thẻ), `1.0` (đúng toán + thiếu thẻ),
//!      `0.2` (sai toán + chuẩn thẻ), `0.0` (sai toán + hỏng thẻ).
//! 6. **Bốn Biến Thể RLVR Hiện Đại (Frontier Reasoning Post-Training)**:
//!    - **`Grpo`**: Group Relative Policy Optimization (DeepSeekMath) — lợi thế $A_i = (R_i - \mu_R)/(\sigma_R + \epsilon)$,
//!      loss trung bình theo từng chuỗi (chia cho $|o_i|$).
//!    - **`DrGrpo`**: *GRPO Done Right* (Liu et al., 2025, arXiv 2503.20783) — bỏ chuẩn hóa $\sigma_R$ lẫn $1/|o_i|$:
//!      lợi thế chỉ là $R_i - \mu_R$, còn loss tổng token được chia cho một **hằng số** (ngân sách sinh `max_gen_len`).
//!    - **`Dapo`**: Decoupled Clip and Dynamic sAmpling Policy Optimization (Yu et al., 2025, arXiv 2503.14476) —
//!      Clip-Higher $[1 - \epsilon_{\text{low}}, 1 + \epsilon_{\text{high}}]$, lọc nhóm phương sai 0 (dynamic sampling),
//!      loss cấp token ($1/\sum_i |o_i|$), bỏ phạt KL ($\beta = 0$).
//!    - **`Gspo`**: Group Sequence Policy Optimization (Zheng et al., 2025, arXiv 2507.18071) — tỷ số quan trọng mức chuỗi
//!      $s_i = \exp\left(\frac{1}{|y_i|}\sum_t \Delta \log \pi_t\right)$ (chuẩn hóa độ dài), kẹp ở mức chuỗi.
//! 7. **Tín Dụng Hiệu Suất Gọt Tỉa Ngữ Cảnh Có Điều Kiện (Conditional Context Pruning Efficiency Credit)**:
//!    - Chỉ kích hoạt khi nhóm có tối thiểu $\ge 2$ quỹ đạo thành công (`wins.len() >= 2`).
//!    - Chặn đứng hành vi gian lận mô hình xóa sạch câu trả lời ngắn để trục lợi điểm hiệu suất khi giải sai.

use std::fmt;

// ============================================================================
// 1. CÔNG CỤ ĐẠI SỐ TUYẾN TÍNH & BỘ SINH SỐ NGẪU NHIÊN TẤT ĐỊNH SPLITMIX64
// ============================================================================

/// Bộ sinh số ngẫu nhiên giả tất định SplitMix64 64-bit hiệu năng cao.
///
/// Dùng để khởi tạo trọng số và lấy mẫu tất định trong môi trường kiểm thử CPU,
/// hoàn toàn không phụ thuộc thư viện ngoài như `rand`.
#[derive(Debug, Clone)]
pub struct SplitMix64 {
    state: u64,
}

impl SplitMix64 {
    /// Khởi tạo bộ sinh số ngẫu nhiên với hạt giống ban đầu.
    pub fn new(seed: u64) -> Self {
        Self { state: seed }
    }

    /// Trả về số nguyên 64-bit ngẫu nhiên tiếp theo.
    pub fn next_u64(&mut self) -> u64 {
        self.state = self.state.wrapping_add(0x9e3779b97f4a7c15);
        let mut z = self.state;
        z = (z ^ (z >> 30)).wrapping_mul(0xbf58476d1ce4e5b9);
        z = (z ^ (z >> 27)).wrapping_mul(0x94d049bb133111eb);
        z ^ (z >> 31)
    }

    /// Trả về số thực phân bố đều trong khoảng `[0.0, 1.0)`.
    pub fn next_f64(&mut self) -> f64 {
        (self.next_u64() >> 11) as f64 * (1.0 / (1u64 << 53) as f64)
    }

    /// Trả về số thực phân bố đều trong khoảng `[-scale, scale]`.
    pub fn next_signed(&mut self, scale: f64) -> f64 {
        (self.next_f64() * 2.0 - 1.0) * scale
    }
}

/// Tính tích vô hướng của hai vector cùng kích thước.
pub fn dot_product(a: &[f64], b: &[f64]) -> f64 {
    assert_eq!(a.len(), b.len(), "Độ dài hai vector phải bằng nhau");
    a.iter().zip(b.iter()).map(|(&x, &y)| x * y).sum()
}

/// Nhân ma trận `A` kích thước `(rows, cols)` với vector `x` kích thước `cols`,
/// sau đó cộng vector độ lệch `bias` kích thước `rows`.
pub fn linear_forward(a: &[f64], x: &[f64], bias: &[f64], rows: usize, cols: usize) -> Vec<f64> {
    assert_eq!(a.len(), rows * cols, "Kích thước ma trận A không khớp");
    assert_eq!(x.len(), cols, "Kích thước vector x không khớp");
    assert_eq!(bias.len(), rows, "Kích thước vector bias không khớp");

    let mut out = Vec::with_capacity(rows);
    for (r, &b) in bias.iter().enumerate().take(rows) {
        let row_start = r * cols;
        let row_slice = &a[row_start..row_start + cols];
        let val = dot_product(row_slice, x) + b;
        out.push(val);
    }
    out
}

/// Hàm kích hoạt Tanh mượt mà cho tầng ẩn.
pub fn tanh_activation(x: &[f64]) -> Vec<f64> {
    x.iter().map(|&v| v.tanh()).collect()
}

/// Hàm Softmax ổn định số học (trừ max trước khi lũy thừa) chuyển logits thành phân phối xác suất.
pub fn softmax(logits: &[f64]) -> Vec<f64> {
    if logits.is_empty() {
        return Vec::new();
    }
    let max_val = logits.iter().copied().fold(f64::NEG_INFINITY, f64::max);
    let exp_vals: Vec<f64> = logits.iter().map(|&v| (v - max_val).exp()).collect();
    let sum_exp: f64 = exp_vals.iter().sum();
    if sum_exp <= 0.0 || !sum_exp.is_finite() {
        // Trường hợp suy biến cực trị: phân bố đều
        let uniform = 1.0 / (logits.len() as f64);
        return vec![uniform; logits.len()];
    }
    exp_vals.iter().map(|&v| v / sum_exp).collect()
}

/// Hàm Log-Softmax ổn định số học: $\log \text{softmax}(x)_i = x_i - \max(x) - \log \sum \exp(x_j - \max(x))$.
pub fn log_softmax(logits: &[f64]) -> Vec<f64> {
    if logits.is_empty() {
        return Vec::new();
    }
    let max_val = logits.iter().copied().fold(f64::NEG_INFINITY, f64::max);
    let sum_exp: f64 = logits.iter().map(|&v| (v - max_val).exp()).sum();
    let log_sum_exp = sum_exp.ln();
    logits.iter().map(|&v| v - max_val - log_sum_exp).collect()
}

/// Tính xấp xỉ đạo hàm riêng bằng sai phân hữu hạn trung tâm (Central Finite Difference)
/// dùng để kiểm chứng toán học giải tích: $f'(x) \approx \frac{f(x + h) - f(x - h)}{2h}$.
pub fn central_difference<F>(x: &[f64], index: usize, eps: f64, mut func: F) -> f64
where
    F: FnMut(&[f64]) -> f64,
{
    let mut x_plus = x.to_vec();
    let mut x_minus = x.to_vec();
    x_plus[index] += eps;
    x_minus[index] -= eps;
    (func(&x_plus) - func(&x_minus)) / (2.0 * eps)
}

// ============================================================================
// 2. MẠNG ACTOR-CRITIC CHIA SẺ THÂN ẨN (ACTOR-CRITIC ARCHITECTURE)
// ============================================================================

/// Mạng Actor-Critic vi mô hỗ trợ tính toán xuôi và trích xuất đặc trưng trạng thái.
///
/// Bắt buộc dùng trường có tên (named fields), không dùng tuple struct vô danh.
#[derive(Debug, Clone, PartialEq)]
pub struct ActorCritic {
    /// Số chiều vector trạng thái đầu vào ($d_{\text{state}}$).
    pub input_dim: usize,
    /// Số chiều tầng biểu diễn ẩn chia sẻ ($d_{\text{hidden}}$).
    pub hidden_dim: usize,
    /// Số lượng hành động rời rạc khả dĩ ($d_{\text{action}}$).
    pub action_dim: usize,
    /// Ma trận trọng số tầng ẩn chia sẻ: kích thước `(hidden_dim, input_dim)`.
    pub w_hidden: Vec<f64>,
    /// Vector độ lệch tầng ẩn chia sẻ: kích thước `hidden_dim`.
    pub b_hidden: Vec<f64>,
    /// Ma trận trọng số đầu Actor (chính sách): kích thước `(action_dim, hidden_dim)`.
    pub w_actor: Vec<f64>,
    /// Vector độ lệch đầu Actor: kích thước `action_dim`.
    pub b_actor: Vec<f64>,
    /// Vector trọng số đầu Critic (giá trị trạng thái vô hướng): kích thước `hidden_dim`.
    pub w_critic: Vec<f64>,
    /// Độ lệch vô hướng đầu Critic: khởi tạo chuẩn xác `0.0`.
    pub b_critic: f64,
}

impl ActorCritic {
    /// Khởi tạo mạng ActorCritic với kích thước chỉ định và hạt giống ngẫu nhiên tất định.
    pub fn new(input_dim: usize, hidden_dim: usize, action_dim: usize, seed: u64) -> Self {
        assert!(input_dim > 0 && hidden_dim > 0 && action_dim > 0);
        let mut rng = SplitMix64::new(seed);

        let scale_hidden = (2.0 / (input_dim + hidden_dim) as f64).sqrt();
        let scale_actor = (2.0 / (hidden_dim + action_dim) as f64).sqrt();

        let w_hidden = (0..(hidden_dim * input_dim))
            .map(|_| rng.next_signed(scale_hidden))
            .collect();
        let b_hidden = vec![0.0; hidden_dim];

        let w_actor = (0..(action_dim * hidden_dim))
            .map(|_| rng.next_signed(scale_actor))
            .collect();
        let b_actor = vec![0.0; action_dim];

        // Đầu Critic được khởi tạo chuẩn xác bằng 0.0 để giá trị V(s) ban đầu không gây nhiễu
        let w_critic = vec![0.0; hidden_dim];
        let b_critic = 0.0;

        Self {
            input_dim,
            hidden_dim,
            action_dim,
            w_hidden,
            b_hidden,
            w_actor,
            b_actor,
            w_critic,
            b_critic,
        }
    }

    /// Tính toán biểu diễn ẩn kích hoạt sau hàm Tanh.
    pub fn forward_hidden(&self, state: &[f64]) -> Vec<f64> {
        let pre_act = linear_forward(
            &self.w_hidden,
            state,
            &self.b_hidden,
            self.hidden_dim,
            self.input_dim,
        );
        tanh_activation(&pre_act)
    }

    /// Lan truyền xuôi đầu Actor: trả về phân phối xác suất hành động qua hàm Softmax.
    pub fn forward_policy(&self, state: &[f64]) -> Vec<f64> {
        let hidden = self.forward_hidden(state);
        let logits = linear_forward(
            &self.w_actor,
            &hidden,
            &self.b_actor,
            self.action_dim,
            self.hidden_dim,
        );
        softmax(&logits)
    }

    /// Lan truyền xuôi đầu Actor: trả về log xác suất hành động qua hàm Log-Softmax.
    pub fn forward_log_policy(&self, state: &[f64]) -> Vec<f64> {
        let hidden = self.forward_hidden(state);
        let logits = linear_forward(
            &self.w_actor,
            &hidden,
            &self.b_actor,
            self.action_dim,
            self.hidden_dim,
        );
        log_softmax(&logits)
    }

    /// Lan truyền xuôi đầu Critic: trả về giá trị trạng thái vô hướng $V(s)$.
    pub fn forward_value(&self, state: &[f64]) -> f64 {
        let hidden = self.forward_hidden(state);
        dot_product(&self.w_critic, &hidden) + self.b_critic
    }

    /// Thực hiện cả hai đầu tính toán đồng thời: trả về `(xác suất chính sách, giá trị trạng thái)`.
    pub fn forward(&self, state: &[f64]) -> (Vec<f64>, f64) {
        let hidden = self.forward_hidden(state);
        let logits = linear_forward(
            &self.w_actor,
            &hidden,
            &self.b_actor,
            self.action_dim,
            self.hidden_dim,
        );
        let probs = softmax(&logits);
        let value = dot_product(&self.w_critic, &hidden) + self.b_critic;
        (probs, value)
    }
}

// ============================================================================
// 3. ƯỚC LƯỢNG LỢI THẾ TỔNG QUÁT (GAE - GENERALIZED ADVANTAGE ESTIMATION)
// ============================================================================

/// Kết quả tính toán lợi thế GAE và phần thưởng tích lũy mục tiêu.
#[derive(Debug, Clone, PartialEq)]
pub struct GaeOutput {
    /// Vector lợi thế $A_t^{\text{GAE}}$ tại từng bước thời gian $t \in [0, T-1]$.
    pub advantages: Vec<f64>,
    /// Vector phần thưởng tích lũy kỳ vọng $R_t = A_t + V(s_t)$.
    pub returns: Vec<f64>,
}

/// Tính toán ước lượng lợi thế tổng quát GAE ($\gamma, \lambda$) theo quy hoạch động ngược thời gian:
///
/// $$\delta_t = r_t + \gamma V(s_{t+1}) - V(s_t)$$
/// $$A_t = \delta_t + (\gamma \lambda) A_{t+1}$$
///
/// Bất biến toán học:
/// - Khi $\lambda = 0$: $A_t = \delta_t$ (ước lượng TD 1-bước thuần túy, phương sai thấp, độ chệch phụ thuộc Critic).
/// - Khi $\lambda = 1$: $A_t = \sum_{k=t}^{T-1} \gamma^{k-t} r_k - V(s_t)$ (ước lượng Monte-Carlo không chệch).
pub fn compute_gae(
    rewards: &[f64],
    values: &[f64],
    next_value: f64,
    gamma: f64,
    lam: f64,
) -> GaeOutput {
    assert_eq!(
        rewards.len(),
        values.len(),
        "Độ dài mảng rewards và values phải bằng nhau"
    );
    let t_steps = rewards.len();
    if t_steps == 0 {
        return GaeOutput {
            advantages: Vec::new(),
            returns: Vec::new(),
        };
    }

    let mut advantages = vec![0.0; t_steps];
    let mut gae = 0.0;

    // Duyệt ngược thời gian từ T-1 về 0
    for t in (0..t_steps).rev() {
        let v_next = if t + 1 < t_steps {
            values[t + 1]
        } else {
            next_value
        };
        let delta = rewards[t] + gamma * v_next - values[t];
        gae = delta + gamma * lam * gae;
        advantages[t] = gae;
    }

    let returns: Vec<f64> = advantages
        .iter()
        .zip(values.iter())
        .map(|(&adv, &val)| adv + val)
        .collect();

    GaeOutput {
        advantages,
        returns,
    }
}

// ============================================================================
// 4. BƯỚC CẬP NHẬT CHÍNH SÁCH KẸP PPO (PPO CLIPPED OBJECTIVE)
// ============================================================================

/// Kết quả phân tích tổn thất của một bước PPO Clipped.
#[derive(Debug, Clone, PartialEq)]
pub struct PpoStepResult {
    /// Giá trị tổn thất chính sách kẹp Surrogate (mang dấu âm khi tối thiểu hóa).
    pub policy_loss: f64,
    /// Giá trị tổn thất hàm giá trị bình phương sai số MSE.
    pub value_loss: f64,
    /// Giá trị entropy của chính sách $\sum -p \ln p$.
    pub entropy: f64,
    /// Tổng hàm mục tiêu tổn thất: `policy_loss + c1 * value_loss - c2 * entropy`.
    pub total_loss: f64,
    /// Xấp xỉ phân kỳ KL giữa chính sách mới và chính sách cũ: $(\log \pi_{\text{old}} - \log \pi)$.
    pub approx_kl: f64,
    /// Tỷ lệ các bước bị kẹp bởi biên $[1-\epsilon, 1+\epsilon]$.
    pub clip_fraction: f64,
}

/// Cấu hình tham số siêu phẳng cho bước cập nhật PPO.
#[derive(Debug, Clone, PartialEq)]
pub struct PpoConfig {
    /// Hệ số kẹp biên chính sách ($\epsilon$). Mặc định là 0.2.
    pub eps_clip: f64,
    /// Trọng số hàm tổn thất giá trị $c_1$ ($L^{\text{VF}}$). Mặc định là 0.5.
    pub vf_coef: f64,
    /// Trọng số thưởng entropy $c_2$ ($S[\pi]$). Mặc định là 0.01.
    pub entropy_coef: f64,
}

impl Default for PpoConfig {
    fn default() -> Self {
        Self {
            eps_clip: 0.2,
            vf_coef: 0.5,
            entropy_coef: 0.01,
        }
    }
}

/// Tính toán tổn thất PPO kẹp Surrogate cho một mẫu bước chuyển:
///
/// $$L^{\text{CLIP}}(\theta) = \hat{\mathbb{E}}_t \left[ \min\left( r_t(\theta)\hat{A}_t, \text{clip}(r_t(\theta), 1-\epsilon, 1+\epsilon)\hat{A}_t \right) \right]$$
///
/// Trong đó tỷ số quan trọng $r_t(\theta) = \frac{\pi_\theta(a_t|s_t)}{\pi_{\theta_{\text{old}}}(a_t|s_t)} = \exp(\log \pi_\theta - \log \pi_{\text{old}})$.
pub fn compute_ppo_clipped_loss(
    log_prob: f64,
    log_prob_old: f64,
    advantage: f64,
    predicted_value: f64,
    target_return: f64,
    policy_probs: &[f64],
    cfg: &PpoConfig,
) -> PpoStepResult {
    let ratio = (log_prob - log_prob_old).exp();
    let clipped_ratio = ratio.clamp(1.0 - cfg.eps_clip, 1.0 + cfg.eps_clip);

    let surr1 = ratio * advantage;
    let surr2 = clipped_ratio * advantage;
    let policy_surrogate = surr1.min(surr2);
    // Trong tối ưu hóa, ta muốn cực đại hóa surrogate -> tối thiểu hóa (-policy_surrogate)
    let policy_loss = -policy_surrogate;

    let value_diff = predicted_value - target_return;
    let value_loss = value_diff * value_diff;

    // Tính entropy phân phối: S = - sum(p * ln(p))
    let entropy: f64 = policy_probs
        .iter()
        .filter(|&&p| p > 1e-15)
        .map(|&p| -p * p.ln())
        .sum();

    let total_loss = policy_loss + cfg.vf_coef * value_loss - cfg.entropy_coef * entropy;
    let approx_kl = (log_prob_old - log_prob).max(0.0);
    let clip_fraction = if (ratio - clipped_ratio).abs() > 1e-12 {
        1.0
    } else {
        0.0
    };

    PpoStepResult {
        policy_loss,
        value_loss,
        entropy,
        total_loss,
        approx_kl,
        clip_fraction,
    }
}

// ============================================================================
// 5. PHẠT PHÂN KỲ KL KHÔNG ÂM MỌI NƠI K3 CỦA JOHN SCHULMAN
// ============================================================================

/// Phạt phân kỳ KL xấp xỉ không âm mọi nơi $k_3$ theo công thức của John Schulman:
///
/// Cho $d = \log \pi_{\text{ref}}(y) - \log \pi_\theta(y)$, ta có:
///
/// $$D_{k3}(\pi_\theta, \pi_{\text{ref}}) = \exp(d) - d - 1$$
///
/// Chứng minh toán học tính không âm:
/// Đặt hàm số $g(d) = e^d - d - 1$.
/// Đạo hàm bậc nhất: $g'(d) = e^d - 1$.
/// Điểm dừng: $g'(d) = 0 \iff e^d = 1 \iff d = 0$.
/// Đạo hàm bậc hai: $g''(d) = e^d > 0 \; \forall d \in \mathbb{R}$.
/// Do đó, $g(d)$ đạt giá trị cực tiểu toàn cục duy nhất tại $d = 0$, với $g(0) = e^0 - 0 - 1 = 0$.
/// Vì vậy: $D_{k3}(\pi_\theta, \pi_{\text{ref}}) \ge 0 \; \forall \log \pi, \log \pi_{\text{ref}} \in \mathbb{R}$.
pub fn k3_kl(log_pi: f64, log_ref: f64) -> f64 {
    let d = log_ref - log_pi;
    d.exp() - d - 1.0
}

/// Đạo hàm giải tích của tổn thất $k_3$ đối với biến số $\log \pi$:
///
/// $$\frac{\partial}{\partial \log \pi} \left[ \beta \cdot (\exp(\log_{\text{ref}} - \log \pi) - (\log_{\text{ref}} - \log \pi) - 1) \right] = \beta \cdot \left( 1 - \exp(\log_{\text{ref}} - \log \pi) \right)$$
pub fn k3_kl_grad(log_pi: f64, log_ref: f64, beta: f64) -> f64 {
    let d = log_ref - log_pi;
    beta * (1.0 - d.exp())
}

/// Tính toán tổn thất cấp token của thuật toán GRPO kèm đạo hàm giải tích đối với $\log \pi$.
///
/// Trả về cặp `(tổn thất, đạo hàm giải tích)`:
/// - Khi unclipped: đạo hàm bao gồm gradient của surrogate và gradient phạt KL.
/// - Khi clipped: gradient của surrogate bằng 0.0, chỉ còn gradient phạt KL.
pub fn grpo_token_loss_and_grad(
    log_pi: f64,
    log_old: f64,
    log_ref: f64,
    advantage: f64,
    eps_clip: f64,
    beta: f64,
) -> (f64, f64) {
    let ratio = (log_pi - log_old).exp();
    let clipped_ratio = ratio.clamp(1.0 - eps_clip, 1.0 + eps_clip);

    let surr1 = ratio * advantage;
    let surr2 = clipped_ratio * advantage;

    // Xác định xem có nằm trong vùng unclipped hay không
    let (surr, ds) = if surr1 <= surr2 || (ratio > 1.0 - eps_clip && ratio < 1.0 + eps_clip) {
        (surr1.min(surr2), ratio * advantage)
    } else {
        (clipped_ratio * advantage, 0.0)
    };

    let loss = -surr + beta * k3_kl(log_pi, log_ref);
    let grad = -ds + k3_kl_grad(log_pi, log_ref, beta);
    (loss, grad)
}

// ============================================================================
// 6. BỘ CHẤM ĐIỂM TOÁN HỌC TẤT ĐỊNH (MATH ANSWER VERIFIER)
// ============================================================================

/// Kết quả bóc tách cú pháp và chấm điểm của bộ thẩm định toán học `MathAnswerVerifier`.
#[derive(Debug, Clone, PartialEq)]
pub struct VerificationResult {
    /// Cú pháp thẻ `<think>...</think><answer>...</answer>` có hoàn toàn hợp lệ không.
    pub has_valid_format: bool,
    /// Giá trị đáp án toán học trích xuất được có khớp với đáp án chân lý (Ground Truth) không.
    pub is_correct: bool,
    /// Điểm phần thưởng toán học (1.0 nếu đúng, 0.0 nếu sai).
    pub math_reward: f64,
    /// Điểm phần thưởng định dạng khuôn thẻ (0.2 nếu chuẩn, 0.0 nếu hỏng).
    pub format_reward: f64,
    /// Tổng điểm phần thưởng: `math_reward + format_reward` $\in \{1.2, 1.0, 0.2, 0.0\}$.
    pub total_reward: f64,
    /// Nội dung khối suy nghĩ trích xuất từ thẻ `<think>...</think>` (nếu có).
    pub extracted_think: Option<String>,
    /// Chuỗi đáp án trích xuất từ thẻ `<answer>...</answer>` (nếu có).
    pub extracted_answer: Option<String>,
}

impl fmt::Display for VerificationResult {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        write!(
            f,
            "Verification(score: {:.1}, math: {}, fmt: {})",
            self.total_reward, self.is_correct, self.has_valid_format
        )
    }
}

/// Bộ thẩm định toán học tất định kiểm tra định dạng thẻ XML và so khớp giá trị số học.
#[derive(Debug, Clone, Default)]
pub struct MathAnswerVerifier {
    /// Sai số cho phép khi so khớp số thực floating-point.
    pub tolerance: f64,
}

impl MathAnswerVerifier {
    /// Khởi tạo bộ thẩm định với sai số mặc định `1e-6`.
    pub fn new() -> Self {
        Self { tolerance: 1e-6 }
    }

    /// Khởi tạo với sai số chỉ định.
    pub fn with_tolerance(tolerance: f64) -> Self {
        Self { tolerance }
    }

    /// Trích xuất nội dung giữa hai thẻ mở và đóng. Kiểm tra tính duy nhất (chỉ xuất hiện đúng 1 lần).
    fn extract_tag_content(text: &str, open_tag: &str, close_tag: &str) -> Option<String> {
        let open_count = text.matches(open_tag).count();
        let close_count = text.matches(close_tag).count();
        if open_count != 1 || close_count != 1 {
            return None;
        }

        let start_pos = text.find(open_tag)? + open_tag.len();
        let end_pos = text.find(close_tag)?;
        if start_pos <= end_pos {
            Some(text[start_pos..end_pos].trim().to_string())
        } else {
            None
        }
    }

    /// Trích xuất số thực cuối cùng xuất hiện trong một chuỗi văn bản (dùng cho cơ chế dự phòng).
    fn extract_last_number(text: &str) -> Option<f64> {
        let mut words = text.split_whitespace().rev();
        for word in words.by_ref() {
            let cleaned: String = word
                .chars()
                .filter(|&c| c.is_ascii_digit() || c == '.' || c == '-')
                .collect();
            if let Ok(num) = cleaned.parse::<f64>() {
                return Some(num);
            }
        }
        None
    }

    /// So khớp hai số thực với sai số cho phép `tolerance`.
    pub fn is_numeric_match(&self, actual: f64, expected: f64) -> bool {
        (actual - expected).abs() <= self.tolerance
    }

    /// Thẩm định văn bản sinh ra bởi LLM đối chiếu với đáp án số học mong đợi.
    ///
    /// Bảng điểm trả về:
    /// - `1.2`: Đúng toán + Đúng chuẩn định dạng thẻ `<think>...</think><answer>...</answer>`.
    /// - `1.0`: Đúng toán nhưng thiếu thẻ hoặc định dạng thẻ bị sai/lộn xộn.
    /// - `0.2`: Đúng chuẩn định dạng thẻ nhưng tính toán sai đáp án số học.
    /// - `0.0`: Cả định dạng thẻ lẫn đáp án số học đều không chính xác.
    pub fn verify(&self, generated_text: &str, expected_answer: f64) -> VerificationResult {
        let think_open = "<think>";
        let think_close = "</think>";
        let answer_open = "<answer>";
        let answer_close = "</answer>";

        // Kiểm tra thứ tự xuất hiện các thẻ: <think> < </think> < <answer> < </answer>
        let valid_structure = match (
            generated_text.find(think_open),
            generated_text.find(think_close),
            generated_text.find(answer_open),
            generated_text.find(answer_close),
        ) {
            (Some(t_o), Some(t_c), Some(a_o), Some(a_c)) => t_o < t_c && t_c < a_o && a_o < a_c,
            _ => false,
        };

        let think_content = Self::extract_tag_content(generated_text, think_open, think_close);
        let answer_content = Self::extract_tag_content(generated_text, answer_open, answer_close);

        let has_valid_format =
            valid_structure && think_content.is_some() && answer_content.is_some();

        // Kiểm tra đáp án
        let mut is_correct = false;
        if let Some(ref ans_str) = answer_content {
            if let Ok(val) = ans_str.parse::<f64>() {
                is_correct = self.is_numeric_match(val, expected_answer);
            } else if let Some(val) = Self::extract_last_number(ans_str) {
                is_correct = self.is_numeric_match(val, expected_answer);
            }
        } else {
            // Dự phòng: nếu không có thẻ hoặc thẻ hỏng, thử tìm số cuối cùng trong toàn bộ văn bản
            if let Some(val) = Self::extract_last_number(generated_text) {
                is_correct = self.is_numeric_match(val, expected_answer);
            }
        }

        let math_reward = if is_correct { 1.0 } else { 0.0 };
        let format_reward = if has_valid_format { 0.2 } else { 0.0 };
        let total_reward = math_reward + format_reward;

        VerificationResult {
            has_valid_format,
            is_correct,
            math_reward,
            format_reward,
            total_reward,
            extracted_think: think_content,
            extracted_answer: answer_content,
        }
    }
}

// ============================================================================
// 7. BỐN BIẾN THỂ RLVR & LỢI THẾ NHÓM TƯƠNG ĐỐI
// ============================================================================

/// Liệt kê 4 biến thể thuật toán RLVR tiên tiến:
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum RlvrVariant {
    /// Group Relative Policy Optimization: lợi thế chuẩn hóa bằng $\mu_R$, $\sigma_R$; loss chia cho $|o_i|$.
    Grpo,
    /// GRPO Done Right (Dr. GRPO): lợi thế $R_i - \mu_R$ (không chia $\sigma_R$), loss tổng token chia cho
    /// hằng số `max_gen_len` thay vì độ dài thực $|o_i|$ (xem [`rlvr_loss_weights`]).
    DrGrpo,
    /// Decoupled Clip and Dynamic sAmpling Policy Optimization: Clip-Higher, lọc nhóm phương sai 0, loss cấp token,
    /// $\beta_{\text{KL}} = 0$.
    Dapo,
    /// Group Sequence Policy Optimization: tỷ số quan trọng mức chuỗi chuẩn hóa độ dài.
    Gspo,
}

/// Dữ liệu quỹ đạo sinh ra của một ứng viên trong nhóm cuộn mẫu (Rollout).
#[derive(Debug, Clone, PartialEq)]
pub struct GrpoRollout {
    /// Điểm phần thưởng vô hướng của quỹ đạo.
    pub reward: f64,
    /// Quỹ đạo có đạt trạng thái thành công toán học hay không (`is_correct`).
    pub is_success: bool,
    /// Tổng số lượng token sinh ra trong quỹ đạo.
    pub token_len: usize,
    /// Mặt nạ các vị trí token được gọt tỉa / tối ưu hóa ngữ cảnh.
    pub context_pruned_mask: Vec<bool>,
}

/// Cấu hình tham số cho động cơ tính toán lợi thế nhóm RLVR.
#[derive(Debug, Clone, PartialEq)]
pub struct RlvrConfig {
    /// Trọng số thưởng hiệu suất ngữ cảnh $w_{\text{eff}}$.
    pub w_eff: f64,
    /// Hệ số làm mịn mẫu số tránh chia cho 0 ($\epsilon$).
    pub eps: f64,
    /// Ngân sách sinh tối đa (`max_gen_len`) dùng làm mẫu số **hằng số** để chuẩn hóa loss trong Dr. GRPO.
    pub max_gen_len: usize,
    /// Hệ số kẹp biên dưới của DAPO ($\epsilon_{\text{low}}$).
    pub eps_low: f64,
    /// Hệ số kẹp biên trên của DAPO ($\epsilon_{\text{high}}$).
    pub eps_high: f64,
    /// Ngưỡng phương sai phần thưởng tối thiểu để giữ lại nhóm trong DAPO.
    pub min_reward_variance: f64,
}

impl Default for RlvrConfig {
    fn default() -> Self {
        Self {
            w_eff: 0.4,
            eps: 1e-8,
            max_gen_len: 256,
            eps_low: 0.20,
            eps_high: 0.28,
            min_reward_variance: 1e-6,
        }
    }
}

/// Kết quả phân bổ lợi thế nhóm qua thuật toán RLVR.
#[derive(Debug, Clone, PartialEq)]
pub struct GroupAdvantageResult {
    /// Ma trận lợi thế từng token: `[số_rollout][số_token]`.
    pub token_advantages: Vec<Vec<f64>>,
    /// Lợi thế cấp chuỗi của từng rollout: `[số_rollout]`.
    pub sequence_advantages: Vec<f64>,
    /// Nhóm này có được kích hoạt tính gradient hay bị lọc bỏ (e.g. bởi DAPO).
    pub is_active_group: bool,
    /// Tín dụng hiệu suất ngữ cảnh có được kích hoạt (yêu cầu $\ge 2$ quỹ đạo thắng).
    pub efficiency_credit_activated: bool,
}

/// Tính toán lợi thế nhóm tương đối hỗ trợ cả 4 biến thể GRPO, Dr. GRPO, DAPO, GSPO
/// và cơ chế tín dụng hiệu suất gọt tỉa ngữ cảnh có điều kiện.
pub fn compute_rlvr_group_advantages(
    group: &[GrpoRollout],
    variant: RlvrVariant,
    config: &RlvrConfig,
) -> GroupAdvantageResult {
    let n = group.len();
    if n == 0 {
        return GroupAdvantageResult {
            token_advantages: Vec::new(),
            sequence_advantages: Vec::new(),
            is_active_group: false,
            efficiency_credit_activated: false,
        };
    }

    let n_f64 = n as f64;
    let mu_r = group.iter().map(|r| r.reward).sum::<f64>() / n_f64;
    let variance_r = group.iter().map(|r| (r.reward - mu_r).powi(2)).sum::<f64>() / n_f64;
    let sd_r = variance_r.sqrt();

    // 1. Kiểm tra điều kiện DAPO: Nếu phương sai phần thưởng xấp xỉ 0 (toàn thắng hoặc toàn thua),
    // DAPO loại bỏ nhóm này khỏi quá trình cập nhật gradient để tránh nhiễu và chia 0/0.
    if variant == RlvrVariant::Dapo && variance_r < config.min_reward_variance {
        let zero_token_advs: Vec<Vec<f64>> = group
            .iter()
            .map(|r| vec![0.0; r.context_pruned_mask.len()])
            .collect();
        return GroupAdvantageResult {
            token_advantages: zero_token_advs,
            sequence_advantages: vec![0.0; n],
            is_active_group: false,
            efficiency_credit_activated: false,
        };
    }

    // 2. Tính toán tín dụng hiệu suất ngữ cảnh có điều kiện (BẮT BUỘC >= 2 quỹ đạo thắng)
    let wins: Vec<f64> = group
        .iter()
        .filter(|r| r.is_success)
        .map(|r| r.token_len as f64)
        .collect();

    let efficiency_params = if wins.len() >= 2 {
        let nw = wins.len() as f64;
        let mu_len = wins.iter().sum::<f64>() / nw;
        let var_len = wins.iter().map(|&l| (l - mu_len).powi(2)).sum::<f64>() / nw;
        let sd_len = var_len.sqrt();
        Some((mu_len, sd_len))
    } else {
        None
    };

    let mut sequence_advantages = Vec::with_capacity(n);
    let mut token_advantages = Vec::with_capacity(n);

    for rollout in group {
        // Tính toán lợi thế kết quả theo từng biến thể
        let outcome_adv = match variant {
            RlvrVariant::Grpo | RlvrVariant::Gspo => (rollout.reward - mu_r) / (sd_r + config.eps),
            RlvrVariant::DrGrpo => {
                // Dr. GRPO: chỉ trừ đường cơ sở (trung bình nhóm), KHÔNG chia cho độ lệch chuẩn.
                // Phần chuẩn hóa độ dài bằng hằng số nằm ở phép tổng hợp loss (`rlvr_loss_weights`).
                rollout.reward - mu_r
            }
            RlvrVariant::Dapo => {
                // DAPO: lợi thế nhóm chuẩn hóa như GRPO; điểm khác biệt nằm ở kẹp Clip-Higher,
                // loss cấp token và lọc nhóm (xem `dapo_token_loss_and_grad`)
                (rollout.reward - mu_r) / (sd_r + config.eps)
            }
        };

        // Tính lợi thế hiệu suất (chỉ dành cho quỹ đạo thắng khi có >= 2 người thắng)
        let eff_adv = match (rollout.is_success, efficiency_params) {
            (true, Some((mu_len, sd_len))) => {
                // Quỹ đạo càng ngắn so với trung bình người thắng thì điểm thưởng càng dương
                (mu_len - (rollout.token_len as f64)) / (sd_len + config.eps)
            }
            _ => 0.0,
        };

        sequence_advantages.push(outcome_adv);

        // Phân bổ lợi thế đến từng vị trí token
        let token_advs: Vec<f64> = rollout
            .context_pruned_mask
            .iter()
            .map(|&is_pruned| {
                if is_pruned {
                    outcome_adv + config.w_eff * eff_adv
                } else {
                    outcome_adv
                }
            })
            .collect();
        token_advantages.push(token_advs);
    }

    GroupAdvantageResult {
        token_advantages,
        sequence_advantages,
        is_active_group: true,
        efficiency_credit_activated: efficiency_params.is_some(),
    }
}

/// Trọng số chuẩn hóa loss của từng quỹ đạo trong nhóm, tùy biến thể RLVR.
///
/// Trả về vector `w` (một phần tử cho mỗi rollout). Với GRPO / Dr. GRPO / DAPO, `w[i]` là hệ số nhân
/// lên **tổng** các loss token của quỹ đạo `i`; với GSPO, `w[i]` nhân lên loss mức chuỗi (mỗi chuỗi chỉ có
/// một số hạng, phần chuẩn hóa độ dài đã nằm trong tỷ số quan trọng $s_i$).
///
/// | Biến thể | Hệ số $w_i$ | Nguồn |
/// |---|---|---|
/// | `Grpo` | $\frac{1}{G \cdot \lvert o_i \rvert}$ | trung bình theo chuỗi rồi trung bình nhóm (thiên kiến độ dài) |
/// | `DrGrpo` | $\frac{1}{G \cdot L_{\max}}$ | hằng số = ngân sách sinh (Liu et al., 2025, Mục 3.2) |
/// | `Dapo` | $\frac{1}{\sum_j \lvert o_j \rvert}$ | loss cấp token trên toàn nhóm (Yu et al., 2025, Eq. 8) |
/// | `Gspo` | $\frac{1}{G}$ | loss mức chuỗi (Zheng et al., 2025, Eq. 5) |
///
/// Độ dài `token_len = 0` được coi là 1 để tránh chia cho 0; `max_gen_len = 0` cũng được coi là 1.
pub fn rlvr_loss_weights(
    group: &[GrpoRollout],
    variant: RlvrVariant,
    config: &RlvrConfig,
) -> Vec<f64> {
    let g = group.len() as f64;
    match variant {
        RlvrVariant::Grpo => group
            .iter()
            .map(|r| 1.0 / (g * r.token_len.max(1) as f64))
            .collect(),
        RlvrVariant::DrGrpo => {
            let constant = config.max_gen_len.max(1) as f64;
            vec![1.0 / (g * constant); group.len()]
        }
        RlvrVariant::Dapo => {
            let total_tokens: f64 = group.iter().map(|r| r.token_len.max(1) as f64).sum();
            vec![1.0 / total_tokens; group.len()]
        }
        RlvrVariant::Gspo => vec![1.0 / g; group.len()],
    }
}

/// Tổn thất token của DAPO với **Clip-Higher** (không có phạt KL, $\beta = 0$) kèm đạo hàm theo $\log \pi$.
///
/// $$L = -\min\left(r A,\ \text{clip}(r,\ 1 - \epsilon_{\text{low}},\ 1 + \epsilon_{\text{high}})\, A\right),
/// \qquad r = \exp(\log \pi - \log \pi_{\text{old}})$$
///
/// Khác GRPO ở chỗ hai biên kẹp được tách rời (mặc định $\epsilon_{\text{low}} = 0.20$, $\epsilon_{\text{high}} = 0.28$):
/// biên trên rộng hơn để token xác suất thấp ("khám phá") còn chỗ tăng, chống sụp entropy.
///
/// Trả về `(tổn thất, đạo hàm)`. Khi bị kẹp, gradient của surrogate bằng 0.
pub fn dapo_token_loss_and_grad(
    log_pi: f64,
    log_old: f64,
    advantage: f64,
    eps_low: f64,
    eps_high: f64,
) -> (f64, f64) {
    let ratio = (log_pi - log_old).exp();
    let clipped_ratio = ratio.clamp(1.0 - eps_low, 1.0 + eps_high);
    let surr_unclipped = ratio * advantage;
    let surr_clipped = clipped_ratio * advantage;
    if surr_unclipped <= surr_clipped {
        // Nhánh min là số hạng chưa kẹp: d(r A)/d(log pi) = r A
        (-surr_unclipped, -surr_unclipped)
    } else {
        (-surr_clipped, 0.0)
    }
}

/// Tính tỷ số quan trọng mức chuỗi (Sequence-level Importance Ratio) cho GSPO:
///
/// $$r_{\text{seq}} = \exp\left( \frac{1}{T} \sum_{t=1}^T (\log \pi(y_t) - \log \pi_{\text{old}}(y_t)) \right)$$
///
/// Bằng cách lấy trung bình hình học trên toàn bộ chuỗi token thay vì nhân lũy tích,
/// GSPO ngăn chặn hiện tượng bùng nổ phương sai của tích tỷ số trên chuỗi suy luận dài hàng nghìn token.
pub fn compute_gspo_sequence_ratio(log_probs: &[f64], old_log_probs: &[f64]) -> f64 {
    assert_eq!(
        log_probs.len(),
        old_log_probs.len(),
        "Độ dài log_probs và old_log_probs phải khớp nhau"
    );
    if log_probs.is_empty() {
        return 1.0;
    }
    let t = log_probs.len() as f64;
    let sum_diff: f64 = log_probs
        .iter()
        .zip(old_log_probs.iter())
        .map(|(&lp, &lo)| lp - lo)
        .sum();
    (sum_diff / t).exp()
}

/// Tính tổn thất GSPO mức chuỗi có kẹp Surrogate và phạt phân kỳ KL $k_3$.
pub fn gspo_sequence_loss_and_grad(
    log_probs: &[f64],
    old_log_probs: &[f64],
    ref_log_probs: &[f64],
    advantage: f64,
    eps_clip: f64,
    beta: f64,
) -> (f64, Vec<f64>) {
    let t = log_probs.len();
    if t == 0 {
        return (0.0, Vec::new());
    }
    let seq_ratio = compute_gspo_sequence_ratio(log_probs, old_log_probs);
    let clipped_seq_ratio = seq_ratio.clamp(1.0 - eps_clip, 1.0 + eps_clip);

    let surr1 = seq_ratio * advantage;
    let surr2 = clipped_seq_ratio * advantage;

    let is_unclipped = surr1 <= surr2 || (seq_ratio > 1.0 - eps_clip && seq_ratio < 1.0 + eps_clip);
    let surr = if is_unclipped {
        surr1.min(surr2)
    } else {
        surr2
    };

    // Phạt KL tính trung bình trên các token
    let avg_kl: f64 = log_probs
        .iter()
        .zip(ref_log_probs.iter())
        .map(|(&lp, &lr)| k3_kl(lp, lr))
        .sum::<f64>()
        / (t as f64);

    let loss = -surr + beta * avg_kl;

    // Đạo hàm theo từng token log_pi[i]
    let t_f64 = t as f64;
    let mut grads = Vec::with_capacity(t);
    for i in 0..t {
        let d_surr = if is_unclipped {
            // d/d(log_pi_i) [seq_ratio * adv] = (seq_ratio / T) * adv
            (seq_ratio / t_f64) * advantage
        } else {
            0.0
        };
        let d_kl = k3_kl_grad(log_probs[i], ref_log_probs[i], beta) / t_f64;
        grads.push(-d_surr + d_kl);
    }

    (loss, grads)
}

// ============================================================================
// 8. ĐỘNG CƠ MÔ PHỎNG HUẤN LUYỆN RLVR ĐẦU-CUỐI
// ============================================================================

/// Một bài toán kiểm thử số học sinh tự động.
#[derive(Debug, Clone, PartialEq)]
pub struct MathProblem {
    /// Câu hỏi biểu diễn dưới dạng chuỗi (ví dụ: "7 * 8 + 4 = ?").
    pub question: String,
    /// Đáp án số học chính xác.
    pub answer: f64,
}

/// Bộ sinh các bài toán số học vi mô dùng cho kiểm thử huấn luyện RLVR.
pub fn generate_micro_math_dataset() -> Vec<MathProblem> {
    vec![
        MathProblem {
            question: "3 * 4 + 5 = ?".into(),
            answer: 17.0,
        },
        MathProblem {
            question: "10 - 2 * 3 = ?".into(),
            answer: 4.0,
        },
        MathProblem {
            question: "12 / 4 + 7 = ?".into(),
            answer: 10.0,
        },
        MathProblem {
            question: "15 - 9 + 8 = ?".into(),
            answer: 14.0,
        },
        MathProblem {
            question: "6 * 6 - 16 = ?".into(),
            answer: 20.0,
        },
    ]
}

/// Mô phỏng câu trả lời sinh bởi mô hình theo mức năng lực (độ chính xác và tuân thủ thẻ).
pub fn format_model_response(
    think_content: &str,
    answer_val: f64,
    use_correct_tags: bool,
) -> String {
    if use_correct_tags {
        format!(
            "<think>\n{}\n</think>\n<answer>\n{}\n</answer>",
            think_content, answer_val
        )
    } else {
        format!(
            "Suy nghĩ: {}\nĐáp án cuối cùng là: {}",
            think_content, answer_val
        )
    }
}

// ============================================================================
// 9. HÀM MAIN MINH HỌA THỰC THI (CLI DEMONSTRATION)
// ============================================================================

/// Định dạng một lát cắt số thực thành chuỗi `[a, b, c]` với số chữ số thập phân cố định.
fn format_vec(values: &[f64], decimals: usize) -> String {
    let items: Vec<String> = values.iter().map(|v| format!("{v:.decimals$}")).collect();
    format!("[{}]", items.join(", "))
}

fn main() {
    println!("=== CHƯƠNG 96: HUẤN LUYỆN HẬU KỲ II (RLVR & PPO/GRPO/DAPO/GSPO) ===");

    // 1. Khởi tạo mạng Actor-Critic
    let ac = ActorCritic::new(8, 16, 4, 42);
    let dummy_state = vec![0.1, -0.2, 0.5, 0.8, -0.4, 0.3, 0.9, -0.1];
    let (probs, val) = ac.forward(&dummy_state);
    println!("\n[1] Mạng Actor-Critic vi mô (std 100%):");
    println!("  - Xác suất hành động Softmax: {}", format_vec(&probs, 4));
    println!("  - Giá trị Critic ban đầu V(s): {:.4}", val);

    // 2. Tính toán GAE
    let rewards = vec![1.0, 0.0, 2.0, 1.0];
    let values = vec![0.5, 0.8, 1.2, 0.9];
    let gae_out = compute_gae(&rewards, &values, 0.0, 0.99, 0.95);
    println!("\n[2] Ước lượng lợi thế tổng quát GAE:");
    println!(
        "  - Lợi thế Advantages: {}",
        format_vec(&gae_out.advantages, 4)
    );
    println!(
        "  - Mục tiêu Returns:    {}",
        format_vec(&gae_out.returns, 4)
    );

    // 3. Kiểm chứng phạt phân kỳ KL k3 của Schulman
    let lp = -1.1;
    let lr = -1.3;
    let k3_val = k3_kl(lp, lr);
    println!("\n[3] Phạt phân kỳ KL k3 của John Schulman:");
    println!("  - log_pi = {}, log_ref = {}", lp, lr);
    println!(
        "  - k3_kl: {:.6} (Bất biến không âm: {})",
        k3_val,
        k3_val >= 0.0
    );

    // 4. Thẩm định bài toán toán học với MathAnswerVerifier
    let verifier = MathAnswerVerifier::new();
    let text_perfect =
        "<think>\nTa có 3 * 4 = 12, sau đó 12 + 5 = 17.\n</think>\n<answer>\n17\n</answer>";
    let res_perfect = verifier.verify(text_perfect, 17.0);
    println!("\n[4] Bộ chấm điểm toán học tất định MathAnswerVerifier:");
    println!("  - Phản hồi chuẩn thẻ & đúng số: {}", res_perfect);
    println!(
        "    -> Điểm tổng: {:.2} (Toán: {:.1}, Khuôn: {:.1})",
        res_perfect.total_reward, res_perfect.math_reward, res_perfect.format_reward
    );

    let text_wrong_calc = "<think>\nTính nhẩm sai 3 * 4 = 15.\n</think>\n<answer>\n20\n</answer>";
    let res_wrong_calc = verifier.verify(text_wrong_calc, 17.0);
    println!("  - Phản hồi chuẩn thẻ nhưng sai số: {}", res_wrong_calc);

    // 5. Tính toán lợi thế nhóm qua 4 biến thể RLVR
    let rollouts = vec![
        GrpoRollout {
            reward: 1.2,
            is_success: true,
            token_len: 45,
            context_pruned_mask: vec![true; 5],
        },
        GrpoRollout {
            reward: 1.2,
            is_success: true,
            token_len: 85,
            context_pruned_mask: vec![true; 5],
        },
        GrpoRollout {
            reward: 0.2,
            is_success: false,
            token_len: 60,
            context_pruned_mask: vec![true; 5],
        },
        GrpoRollout {
            reward: 0.0,
            is_success: false,
            token_len: 20,
            context_pruned_mask: vec![true; 5],
        },
    ];

    let cfg = RlvrConfig::default();
    println!("\n[5] So sánh 4 biến thể RLVR trên cùng một nhóm Rollout:");
    for &variant in &[
        RlvrVariant::Grpo,
        RlvrVariant::DrGrpo,
        RlvrVariant::Dapo,
        RlvrVariant::Gspo,
    ] {
        let adv_res = compute_rlvr_group_advantages(&rollouts, variant, &cfg);
        println!(
            "  - Biến thể {:?}: Lợi thế chuỗi = {}, Hiệu suất bật = {}",
            variant,
            format_vec(&adv_res.sequence_advantages, 4),
            adv_res.efficiency_credit_activated
        );
    }

    // 6. Trọng số chuẩn hóa loss: nơi Dr. GRPO khác GRPO về độ dài
    println!("\n[6] Trọng số chuẩn hóa loss theo từng token (độ dài rollout = 45, 85, 60, 20):");
    for &variant in &[
        RlvrVariant::Grpo,
        RlvrVariant::DrGrpo,
        RlvrVariant::Dapo,
        RlvrVariant::Gspo,
    ] {
        let weights = rlvr_loss_weights(&rollouts, variant, &cfg);
        println!(
            "  - Biến thể {:?}: w = {}",
            variant,
            format_vec(&weights, 6)
        );
    }

    // 7. Clip-Higher của DAPO: ratio = 1.25 nằm giữa biên 1.20 và 1.28
    let log_old = 0.0;
    let log_pi = 1.25f64.ln();
    let (_, grad_sym) = dapo_token_loss_and_grad(log_pi, log_old, 1.0, 0.20, 0.20);
    let (_, grad_high) = dapo_token_loss_and_grad(log_pi, log_old, 1.0, 0.20, 0.28);
    println!("\n[7] Clip-Higher của DAPO tại tỷ số r = 1.25, lợi thế A = +1:");
    println!("  - Kẹp đối xứng [0.80, 1.20]: đạo hàm = {grad_sym:.4}");
    println!("  - Kẹp Clip-Higher [0.80, 1.28]: đạo hàm = {grad_high:.4}");

    println!("\nToàn bộ 4 cổng kiểm tra toán học và kỹ nghệ của Chương 96 đã sẵn sàng.");
}

// ============================================================================
// 10. BỘ KIỂM THỬ TẤT ĐỊNH TOÀN DIỆN (>= 32 BÀI TEST CHẠY NHANH < 150MS)
// ============================================================================

#[cfg(test)]
mod tests {
    use super::*;

    // --- NHÓM 1: CÔNG CỤ ĐẠI SỐ TUYẾN TÍNH & BỘ SINH SỐ NGẪU NHIÊN ---

    #[test]
    fn test_splitmix64_determinism() {
        let mut rng1 = SplitMix64::new(12345);
        let mut rng2 = SplitMix64::new(12345);
        for _ in 0..20 {
            assert_eq!(rng1.next_u64(), rng2.next_u64());
            assert_eq!(rng1.next_f64(), rng2.next_f64());
        }
    }

    #[test]
    fn test_dot_product_correctness() {
        let a = [1.0, 2.0, 3.0];
        let b = [4.0, -1.0, 2.0];
        // 1*4 + 2*(-1) + 3*2 = 4 - 2 + 6 = 8.0
        assert!((dot_product(&a, &b) - 8.0).abs() < 1e-12);
    }

    #[test]
    fn test_softmax_properties() {
        let logits = [2.0, 1.0, 0.1];
        let probs = softmax(&logits);
        let sum: f64 = probs.iter().sum();
        assert!((sum - 1.0).abs() < 1e-12);
        assert!(probs[0] > probs[1] && probs[1] > probs[2]);
        for &p in &probs {
            assert!(p > 0.0 && p < 1.0);
        }
    }

    #[test]
    fn test_softmax_numerical_stability_with_large_values() {
        let logits = [1000.0, 1001.0, 1002.0];
        let probs = softmax(&logits);
        let sum: f64 = probs.iter().sum();
        assert!((sum - 1.0).abs() < 1e-12);
        assert!(!probs[0].is_nan());
        assert!(!probs[2].is_nan());
    }

    #[test]
    fn test_log_softmax_matches_softmax_ln() {
        let logits = [1.5, -0.5, 0.8];
        let lsm = log_softmax(&logits);
        let sm = softmax(&logits);
        for (&l, &s) in lsm.iter().zip(sm.iter()) {
            assert!((l - s.ln()).abs() < 1e-12);
        }
    }

    // --- NHÓM 2: MẠNG ACTOR-CRITIC ---

    #[test]
    fn test_actor_critic_forward_dimensions() {
        let ac = ActorCritic::new(6, 12, 3, 999);
        let state = vec![0.5; 6];
        let (probs, val) = ac.forward(&state);
        assert_eq!(probs.len(), 3);
        assert!((probs.iter().sum::<f64>() - 1.0).abs() < 1e-12);
        // Critic khởi tạo 0.0 nên giá trị ban đầu phải đúng bằng 0.0
        assert_eq!(val, 0.0);
    }

    #[test]
    fn test_actor_critic_hidden_activation_range() {
        let ac = ActorCritic::new(4, 8, 2, 77);
        let state = vec![10.0, -10.0, 5.0, -5.0];
        let hidden = ac.forward_hidden(&state);
        assert_eq!(hidden.len(), 8);
        for &h in &hidden {
            assert!((-1.0..=1.0).contains(&h)); // Tanh bound
        }
    }

    #[test]
    fn test_actor_critic_log_policy_consistency() {
        let ac = ActorCritic::new(4, 8, 3, 88);
        let state = vec![0.1, -0.2, 0.3, -0.4];
        let log_p = ac.forward_log_policy(&state);
        let p = ac.forward_policy(&state);
        for (&lp, &prob) in log_p.iter().zip(p.iter()) {
            assert!((lp - prob.ln()).abs() < 1e-12);
        }
    }

    // --- NHÓM 3: GAE & BẤT BIẾN TOÁN HỌC ---

    #[test]
    fn test_gae_lam_0_degenerates_to_one_step_td() {
        let rewards = [1.0, 2.0, 1.5];
        let values = [0.8, 1.1, 1.0];
        let next_val = 0.5;
        let gamma = 0.9;
        let lam = 0.0;

        let gae = compute_gae(&rewards, &values, next_val, gamma, lam);
        // Với lam = 0: A_t = delta_t = r_t + gamma * V_{t+1} - V_t
        let delta_0 = rewards[0] + gamma * values[1] - values[0];
        let delta_1 = rewards[1] + gamma * values[2] - values[1];
        let delta_2 = rewards[2] + gamma * next_val - values[2];

        assert!((gae.advantages[0] - delta_0).abs() < 1e-12);
        assert!((gae.advantages[1] - delta_1).abs() < 1e-12);
        assert!((gae.advantages[2] - delta_2).abs() < 1e-12);
    }

    #[test]
    fn test_gae_lam_1_degenerates_to_discounted_monte_carlo() {
        let rewards = [1.0, 2.0, 3.0];
        let values = [0.5, 0.6, 0.7];
        let next_val = 0.0;
        let gamma = 0.95;
        let lam = 1.0;

        let gae = compute_gae(&rewards, &values, next_val, gamma, lam);
        // Với lam = 1: A_t = Returns_MC - V_t
        let g2 = rewards[2];
        let g1 = rewards[1] + gamma * g2;
        let g0 = rewards[0] + gamma * g1;

        assert!((gae.advantages[0] - (g0 - values[0])).abs() < 1e-12);
        assert!((gae.advantages[1] - (g1 - values[1])).abs() < 1e-12);
        assert!((gae.advantages[2] - (g2 - values[2])).abs() < 1e-12);
        assert!((gae.returns[0] - g0).abs() < 1e-12);
    }

    #[test]
    fn test_gae_empty_input_handled_safely() {
        let gae = compute_gae(&[], &[], 0.0, 0.99, 0.95);
        assert!(gae.advantages.is_empty());
        assert!(gae.returns.is_empty());
    }

    #[test]
    fn test_gae_gamma_zero_gives_immediate_reward_minus_value() {
        let rewards = [2.5, -1.0];
        let values = [1.0, 0.5];
        let gae = compute_gae(&rewards, &values, 0.0, 0.0, 0.9);
        assert!((gae.advantages[0] - (rewards[0] - values[0])).abs() < 1e-12);
        assert!((gae.advantages[1] - (rewards[1] - values[1])).abs() < 1e-12);
    }

    // --- NHÓM 4: PPO CLIPPED OBJECTIVE ---

    #[test]
    fn test_ppo_clipped_surrogate_loss_activation() {
        let adv_pos = 1.5;
        let cfg = PpoConfig {
            eps_clip: 0.2,
            vf_coef: 0.5,
            entropy_coef: 0.01,
        };
        // Tỷ số ratio = exp(0.5) ~ 1.648 > 1 + eps = 1.2
        let res_clipped = compute_ppo_clipped_loss(0.5, 0.0, adv_pos, 1.0, 1.0, &[0.5, 0.5], &cfg);
        assert_eq!(res_clipped.clip_fraction, 1.0);
        // Khi advantage dương và ratio vượt trần, surrogate = (1 + eps) * adv
        assert!((res_clipped.policy_loss - (-(1.0 + cfg.eps_clip) * adv_pos)).abs() < 1e-12);
    }

    #[test]
    fn test_ppo_clipped_negative_advantage_lower_bound() {
        let adv_neg = -2.0;
        let cfg = PpoConfig {
            eps_clip: 0.2,
            vf_coef: 0.5,
            entropy_coef: 0.0,
        };
        // Tỷ số ratio = exp(-0.5) ~ 0.606 < 1 - eps = 0.8
        let res = compute_ppo_clipped_loss(-0.5, 0.0, adv_neg, 0.0, 0.0, &[0.5, 0.5], &cfg);
        assert_eq!(res.clip_fraction, 1.0);
        // Khi advantage âm và ratio dưới sàn, surrogate = (1 - eps) * adv_neg
        assert!((res.policy_loss - (-(1.0 - cfg.eps_clip) * adv_neg)).abs() < 1e-12);
    }

    #[test]
    fn test_ppo_entropy_bonus_maximal_at_uniform_distribution() {
        let p_uniform = [0.25, 0.25, 0.25, 0.25];
        let p_skewed = [0.85, 0.05, 0.05, 0.05];
        let cfg = PpoConfig {
            eps_clip: 0.2,
            vf_coef: 0.5,
            entropy_coef: 0.1,
        };
        let res_u = compute_ppo_clipped_loss(0.0, 0.0, 1.0, 0.0, 0.0, &p_uniform, &cfg);
        let res_s = compute_ppo_clipped_loss(0.0, 0.0, 1.0, 0.0, 0.0, &p_skewed, &cfg);
        assert!(res_u.entropy > res_s.entropy);
        assert!((res_u.entropy - 4.0f64.ln()).abs() < 1e-12);
    }

    // --- NHÓM 5: PHẠT PHÂN KỲ KL K3 CỦA SCHULMAN ---

    #[test]
    fn test_k3_kl_is_non_negative_everywhere() {
        let test_cases = [
            (-1.0, -1.0), // d = 0 -> k3 = 0.0
            (-0.5, -1.0), // log_pi > log_ref -> d < 0
            (-1.5, -1.0), // log_pi < log_ref -> d > 0
            (0.0, 0.0),
            (-5.0, 0.0),
            (0.0, -5.0),
        ];

        for &(lp, lr) in &test_cases {
            let val = k3_kl(lp, lr);
            assert!(val >= -1e-15, "k3_kl({}) must be >= 0", val);
        }
        assert_eq!(k3_kl(-1.2, -1.2), 0.0);
    }

    #[test]
    fn test_k3_kl_gradient_matches_central_finite_difference() {
        let test_points = [(-1.1, -1.3), (-0.8, -1.5), (-1.4, -0.9), (-2.0, -2.0)];
        let beta = 0.05;

        for &(lp, lr) in &test_points {
            let analytic_grad = k3_kl_grad(lp, lr, beta);
            let num_grad = central_difference(&[lp], 0, 1e-6, |x| beta * k3_kl(x[0], lr));
            assert!(
                (analytic_grad - num_grad).abs() < 1e-8,
                "Lệch đạo hàm k3_kl: analytic={}, numeric={}",
                analytic_grad,
                num_grad
            );
        }
    }

    #[test]
    fn test_grpo_token_loss_and_grad_analytic_vs_finite_diff() {
        let lp = -1.1;
        let lo = -1.15;
        let lr = -1.3;
        let adv = 0.75;
        let eps = 0.2;
        let beta = 0.04;

        let (_, ag) = grpo_token_loss_and_grad(lp, lo, lr, adv, eps, beta);
        let ng = central_difference(&[lp], 0, 1e-6, |p| {
            grpo_token_loss_and_grad(p[0], lo, lr, adv, eps, beta).0
        });

        assert!(
            (ag - ng).abs() < 1e-8,
            "Lệch đạo hàm GRPO token loss: ag={}, ng={}",
            ag,
            ng
        );
    }

    // --- NHÓM 6: BỘ CHẤM ĐIỂM TOÁN HỌC MATH ANSWER VERIFIER ---

    #[test]
    fn test_math_verifier_tier1_correct_and_valid_format() {
        let verifier = MathAnswerVerifier::new();
        let prompt_out = "<think>\n2 + 3 = 5\n</think>\n<answer>\n5\n</answer>";
        let res = verifier.verify(prompt_out, 5.0);
        assert!(res.has_valid_format);
        assert!(res.is_correct);
        assert_eq!(res.math_reward, 1.0);
        assert_eq!(res.format_reward, 0.2);
        assert!((res.total_reward - 1.2).abs() < 1e-12);
        assert_eq!(res.extracted_answer.as_deref(), Some("5"));
    }

    #[test]
    fn test_math_verifier_tier2_correct_math_missing_tags() {
        let verifier = MathAnswerVerifier::new();
        let text = "Sau một hồi tính toán ta được đáp án là 42.";
        let res = verifier.verify(text, 42.0);
        assert!(!res.has_valid_format);
        assert!(res.is_correct);
        assert_eq!(res.math_reward, 1.0);
        assert_eq!(res.format_reward, 0.0);
        assert!((res.total_reward - 1.0).abs() < 1e-12);
    }

    #[test]
    fn test_math_verifier_tier3_wrong_math_valid_format() {
        let verifier = MathAnswerVerifier::new();
        let text = "<think>\nTính nhẩm 7 * 8 = 54 (sai)\n</think>\n<answer>\n54\n</answer>";
        let res = verifier.verify(text, 56.0);
        assert!(res.has_valid_format);
        assert!(!res.is_correct);
        assert_eq!(res.math_reward, 0.0);
        assert_eq!(res.format_reward, 0.2);
        assert!((res.total_reward - 0.2).abs() < 1e-12);
    }

    #[test]
    fn test_math_verifier_tier4_wrong_math_invalid_format() {
        let verifier = MathAnswerVerifier::new();
        let text = "Tôi không biết giải bài này.";
        let res = verifier.verify(text, 10.0);
        assert!(!res.has_valid_format);
        assert!(!res.is_correct);
        assert_eq!(res.total_reward, 0.0);
    }

    #[test]
    fn test_math_verifier_rejects_reversed_tags() {
        let verifier = MathAnswerVerifier::new();
        // Thẻ <answer> đứng trước <think> -> hỏng format
        let text = "<answer>\n10\n</answer>\n<think>\n10\n</think>";
        let res = verifier.verify(text, 10.0);
        assert!(!res.has_valid_format);
        assert!(res.is_correct);
        assert!((res.total_reward - 1.0).abs() < 1e-12);
    }

    #[test]
    fn test_math_verifier_rejects_duplicate_tags() {
        let verifier = MathAnswerVerifier::new();
        let text = "<think>a</think><think>b</think><answer>10</answer>";
        let res = verifier.verify(text, 10.0);
        assert!(!res.has_valid_format);
    }

    // --- NHÓM 7: BỐN BIẾN THỂ RLVR & TÍN DỤNG HIỆU SUẤT CÓ ĐIỀU KIỆN ---

    #[test]
    fn test_grpo_advantages_sum_to_approximately_zero() {
        let group = [
            GrpoRollout {
                reward: 1.2,
                is_success: true,
                token_len: 30,
                context_pruned_mask: vec![false; 3],
            },
            GrpoRollout {
                reward: 1.0,
                is_success: true,
                token_len: 40,
                context_pruned_mask: vec![false; 3],
            },
            GrpoRollout {
                reward: 0.2,
                is_success: false,
                token_len: 50,
                context_pruned_mask: vec![false; 3],
            },
            GrpoRollout {
                reward: 0.0,
                is_success: false,
                token_len: 20,
                context_pruned_mask: vec![false; 3],
            },
        ];
        let cfg = RlvrConfig::default();
        let res = compute_rlvr_group_advantages(&group, RlvrVariant::Grpo, &cfg);
        let sum_adv: f64 = res.sequence_advantages.iter().sum();
        assert!(sum_adv.abs() < 1e-8, "Tổng lợi thế GRPO phải xấp xỉ 0");
    }

    /// Tạo rollout thử nghiệm với phần thưởng, độ dài và cờ thắng cho trước.
    fn make_rollout(reward: f64, is_success: bool, token_len: usize) -> GrpoRollout {
        GrpoRollout {
            reward,
            is_success,
            token_len,
            context_pruned_mask: vec![false],
        }
    }

    #[test]
    fn test_dr_grpo_advantage_is_centered_reward_without_std_or_length() {
        // Dr. GRPO (Liu et al., 2025): A_i = R_i - mean(R), không chia std, không chia độ dài
        let group = [
            make_rollout(1.2, true, 50),
            make_rollout(1.2, true, 200), // cố tình kéo dài câu
            make_rollout(0.0, false, 20),
        ];
        // Thay đổi max_gen_len không được làm đổi lợi thế
        let cfg_small = RlvrConfig {
            max_gen_len: 64,
            ..Default::default()
        };
        let cfg_large = RlvrConfig {
            max_gen_len: 4096,
            ..Default::default()
        };
        let res_small = compute_rlvr_group_advantages(&group, RlvrVariant::DrGrpo, &cfg_small);
        let res_large = compute_rlvr_group_advantages(&group, RlvrVariant::DrGrpo, &cfg_large);
        assert_eq!(res_small.sequence_advantages, res_large.sequence_advantages);

        let mu = (1.2 + 1.2 + 0.0) / 3.0;
        for (adv, rollout) in res_small.sequence_advantages.iter().zip(group.iter()) {
            assert!((adv - (rollout.reward - mu)).abs() < 1e-12);
        }
        // Hai chuỗi cùng phần thưởng, độ dài khác nhau -> cùng lợi thế
        assert_eq!(
            res_small.sequence_advantages[0],
            res_small.sequence_advantages[1]
        );
        // Tổng lợi thế trong nhóm bằng 0 (đường cơ sở trung bình nhóm)
        let sum_adv: f64 = res_small.sequence_advantages.iter().sum();
        assert!(sum_adv.abs() < 1e-12);
    }

    #[test]
    fn test_dr_grpo_scales_with_reward_while_grpo_is_scale_invariant() {
        let base = [
            make_rollout(1.0, true, 30),
            make_rollout(0.0, false, 30),
            make_rollout(0.0, false, 30),
            make_rollout(0.0, false, 30),
        ];
        let scaled: Vec<GrpoRollout> = base
            .iter()
            .map(|r| make_rollout(r.reward * 3.0, r.is_success, r.token_len))
            .collect();
        let cfg = RlvrConfig::default();

        let dr_base = compute_rlvr_group_advantages(&base, RlvrVariant::DrGrpo, &cfg);
        let dr_scaled = compute_rlvr_group_advantages(&scaled, RlvrVariant::DrGrpo, &cfg);
        // Dr. GRPO không chuẩn hóa std nên lợi thế tăng đúng 3 lần
        for (a, b) in dr_base
            .sequence_advantages
            .iter()
            .zip(dr_scaled.sequence_advantages.iter())
        {
            assert!((3.0 * a - b).abs() < 1e-12);
        }

        // GRPO chuẩn hóa std nên (gần như) bất biến với thang phần thưởng
        let g_base = compute_rlvr_group_advantages(&base, RlvrVariant::Grpo, &cfg);
        let g_scaled = compute_rlvr_group_advantages(&scaled, RlvrVariant::Grpo, &cfg);
        for (a, b) in g_base
            .sequence_advantages
            .iter()
            .zip(g_scaled.sequence_advantages.iter())
        {
            assert!((a - b).abs() < 1e-6);
        }
    }

    #[test]
    fn test_rlvr_loss_weights_grpo_has_length_bias_dr_grpo_does_not() {
        let group = [make_rollout(1.0, true, 25), make_rollout(1.0, true, 500)];
        let cfg = RlvrConfig {
            max_gen_len: 1000,
            ..Default::default()
        };
        // GRPO: trọng số mỗi token tỷ lệ nghịch với độ dài -> chuỗi ngắn nặng gấp 20 lần
        let w_grpo = rlvr_loss_weights(&group, RlvrVariant::Grpo, &cfg);
        assert!((w_grpo[0] / w_grpo[1] - 20.0).abs() < 1e-9);
        // Dr. GRPO: mẫu số là hằng số G * max_gen_len, không phụ thuộc độ dài thực
        let w_dr = rlvr_loss_weights(&group, RlvrVariant::DrGrpo, &cfg);
        assert_eq!(w_dr[0], w_dr[1]);
        assert!((w_dr[0] - 1.0 / (2.0 * 1000.0)).abs() < 1e-15);
    }

    #[test]
    fn test_rlvr_loss_weights_dapo_token_level_and_gspo_sequence_level() {
        let group = [
            make_rollout(1.0, true, 10),
            make_rollout(0.0, false, 30),
            make_rollout(0.0, false, 60),
        ];
        let cfg = RlvrConfig::default();
        // DAPO: 1 / tổng số token toàn nhóm => tổng w_i * |o_i| = 1 (mỗi token đóng góp ngang nhau)
        let w_dapo = rlvr_loss_weights(&group, RlvrVariant::Dapo, &cfg);
        let total: f64 = group
            .iter()
            .zip(w_dapo.iter())
            .map(|(r, w)| w * r.token_len as f64)
            .sum();
        assert!((total - 1.0).abs() < 1e-12);
        // GSPO: mỗi chuỗi một số hạng, trọng số 1/G
        let w_gspo = rlvr_loss_weights(&group, RlvrVariant::Gspo, &cfg);
        for w in &w_gspo {
            assert!((w - 1.0 / 3.0).abs() < 1e-15);
        }
    }

    #[test]
    fn test_rlvr_loss_weights_handles_zero_lengths_and_empty_group() {
        let cfg = RlvrConfig {
            max_gen_len: 0,
            ..Default::default()
        };
        let group = [make_rollout(1.0, true, 0), make_rollout(0.0, false, 0)];
        for variant in [
            RlvrVariant::Grpo,
            RlvrVariant::DrGrpo,
            RlvrVariant::Dapo,
            RlvrVariant::Gspo,
        ] {
            let w = rlvr_loss_weights(&group, variant, &cfg);
            assert!(w.iter().all(|x| x.is_finite() && *x > 0.0));
        }
        assert!(rlvr_loss_weights(&[], RlvrVariant::DrGrpo, &cfg).is_empty());
    }

    #[test]
    fn test_dapo_filters_zero_variance_groups_cleanly() {
        // Toàn bộ các mẫu đều được điểm 1.2 (phương sai = 0)
        let group_all_win = [
            GrpoRollout {
                reward: 1.2,
                is_success: true,
                token_len: 30,
                context_pruned_mask: vec![true; 2],
            },
            GrpoRollout {
                reward: 1.2,
                is_success: true,
                token_len: 40,
                context_pruned_mask: vec![true; 2],
            },
        ];

        let cfg = RlvrConfig::default();
        let res = compute_rlvr_group_advantages(&group_all_win, RlvrVariant::Dapo, &cfg);
        // DAPO phải đánh dấu nhóm không hoạt động để bỏ qua gradient update
        assert!(!res.is_active_group);
        for row in res.token_advantages {
            for adv in row {
                assert_eq!(adv, 0.0);
            }
        }
    }

    #[test]
    fn test_dapo_active_when_variance_exists() {
        let group_mixed = [
            GrpoRollout {
                reward: 1.2,
                is_success: true,
                token_len: 30,
                context_pruned_mask: vec![true; 2],
            },
            GrpoRollout {
                reward: 0.0,
                is_success: false,
                token_len: 40,
                context_pruned_mask: vec![true; 2],
            },
        ];
        let cfg = RlvrConfig::default();
        let res = compute_rlvr_group_advantages(&group_mixed, RlvrVariant::Dapo, &cfg);
        assert!(res.is_active_group);
        assert!(res.sequence_advantages[0] > 0.0);
        assert!(res.sequence_advantages[1] < 0.0);
    }

    #[test]
    fn test_dapo_clip_higher_keeps_gradient_where_symmetric_clip_cuts_it() {
        // r = 1.25 nằm giữa biên trên đối xứng (1.20) và Clip-Higher (1.28), A > 0
        let log_pi = 1.25f64.ln();
        let (_, grad_symmetric) = dapo_token_loss_and_grad(log_pi, 0.0, 1.0, 0.20, 0.20);
        assert_eq!(grad_symmetric, 0.0);
        let (loss, grad_high) = dapo_token_loss_and_grad(log_pi, 0.0, 1.0, 0.20, 0.28);
        assert!((grad_high - (-1.25)).abs() < 1e-12);
        assert!((loss - (-1.25)).abs() < 1e-12);
    }

    #[test]
    fn test_dapo_token_loss_clips_on_both_sides() {
        // Biên trên: r = 1.5 > 1.28, A > 0 -> loss = -(1.28) * A, gradient 0
        let (loss_hi, grad_hi) = dapo_token_loss_and_grad(1.5f64.ln(), 0.0, 2.0, 0.20, 0.28);
        assert!((loss_hi - (-1.28 * 2.0)).abs() < 1e-12);
        assert_eq!(grad_hi, 0.0);
        // Biên dưới: r = 0.5 < 0.8, A < 0 -> loss = -(0.8) * A, gradient 0
        let (loss_lo, grad_lo) = dapo_token_loss_and_grad(0.5f64.ln(), 0.0, -1.0, 0.20, 0.28);
        assert!((loss_lo - 0.8).abs() < 1e-12);
        assert_eq!(grad_lo, 0.0);
        // A > 0 nhưng r thấp hơn biên dưới: min chọn số hạng chưa kẹp -> gradient còn nguyên
        let (_, grad_keep) = dapo_token_loss_and_grad(0.5f64.ln(), 0.0, 1.0, 0.20, 0.28);
        assert!((grad_keep - (-0.5)).abs() < 1e-12);
    }

    #[test]
    fn test_dapo_token_loss_gradient_matches_finite_difference() {
        let lo = -1.2;
        let adv = 0.9;
        let (_, ag) = dapo_token_loss_and_grad(-1.1, lo, adv, 0.20, 0.28);
        let ng = central_difference(&[-1.1], 0, 1e-6, |p| {
            dapo_token_loss_and_grad(p[0], lo, adv, 0.20, 0.28).0
        });
        assert!((ag - ng).abs() < 1e-8, "ag={ag}, ng={ng}");
    }

    #[test]
    fn test_gspo_sequence_ratio_geometric_mean() {
        let log_p = [-0.5, -0.8, -0.2];
        let log_old = [-0.6, -0.7, -0.3];
        let ratio = compute_gspo_sequence_ratio(&log_p, &log_old);
        // diffs = 0.1, -0.1, 0.1 -> sum = 0.1 -> avg = 0.1 / 3
        let expected = (0.1 / 3.0f64).exp();
        assert!((ratio - expected).abs() < 1e-12);
    }

    #[test]
    fn test_gspo_sequence_loss_and_grad_analytic_vs_finite_diff() {
        let lp = [-1.0, -1.2];
        let lo = [-1.05, -1.15];
        let lr = [-1.2, -1.3];
        let adv = 0.8;
        let eps = 0.2;
        let beta = 0.03;

        let (_, grads) = gspo_sequence_loss_and_grad(&lp, &lo, &lr, adv, eps, beta);

        for (i, &grad_val) in grads.iter().enumerate().take(lp.len()) {
            let num_g = central_difference(&lp, i, 1e-6, |p| {
                gspo_sequence_loss_and_grad(p, &lo, &lr, adv, eps, beta).0
            });
            assert!(
                (grad_val - num_g).abs() < 1e-8,
                "GSPO grad lệch tại token {}: ag={}, ng={}",
                i,
                grad_val,
                num_g
            );
        }
    }

    // --- NHÓM 8: TÍN DỤNG HIỆU SUẤT NGỮ CẢNH CÓ ĐIỀU KIỆN ---

    #[test]
    fn test_conditional_efficiency_strictly_zero_when_less_than_two_wins() {
        // Chỉ có 1 quỹ đạo thắng -> KHÔNG kích hoạt thưởng hiệu suất
        let g1 = [
            GrpoRollout {
                reward: 1.0,
                is_success: true,
                token_len: 40,
                context_pruned_mask: vec![false, true],
            },
            GrpoRollout {
                reward: 0.0,
                is_success: false,
                token_len: 5, // Cực ngắn nhưng giải sai -> cấm nhận thưởng ngắn!
                context_pruned_mask: vec![true, true],
            },
        ];

        let cfg = RlvrConfig::default();
        let res = compute_rlvr_group_advantages(&g1, RlvrVariant::Grpo, &cfg);
        assert!(!res.efficiency_credit_activated);
        // Cả vị trí false lẫn true đều nhận cùng outcome advantage (không bị cộng thêm hiệu suất)
        assert_eq!(res.token_advantages[0][0], res.token_advantages[0][1]);
        assert_eq!(res.token_advantages[1][0], res.token_advantages[1][1]);
    }

    #[test]
    fn test_conditional_efficiency_rewards_shorter_winner_when_two_or_more_wins() {
        let g2 = [
            GrpoRollout {
                reward: 1.0,
                is_success: true,
                token_len: 50, // Thắng ngắn gọn -> nhận thưởng
                context_pruned_mask: vec![false, true],
            },
            GrpoRollout {
                reward: 1.0,
                is_success: true,
                token_len: 150, // Thắng dài dòng -> bị phạt hiệu suất
                context_pruned_mask: vec![false, true],
            },
            GrpoRollout {
                reward: 0.0,
                is_success: false,
                token_len: 10, // Thua -> không nhận thưởng dù rất ngắn
                context_pruned_mask: vec![true, true],
            },
        ];

        let cfg = RlvrConfig {
            w_eff: 0.5,
            ..Default::default()
        };
        let res = compute_rlvr_group_advantages(&g2, RlvrVariant::Grpo, &cfg);
        assert!(res.efficiency_credit_activated);

        // Winner ngắn có advantage ở vị trí pruned (index 1) LỚN HƠN vị trí gốc (index 0)
        assert!(res.token_advantages[0][1] > res.token_advantages[0][0]);
        // Winner dài có advantage ở vị trí pruned (index 1) NHỎ HƠN vị trí gốc (index 0)
        assert!(res.token_advantages[1][1] < res.token_advantages[1][0]);
        // Kẻ thua cuộc (index 2) không nhận thêm hiệu suất
        assert_eq!(res.token_advantages[2][0], res.token_advantages[2][1]);
    }

    #[test]
    fn test_end_to_end_micro_math_training_step() {
        // Mô phỏng 1 bước tối ưu hóa chính sách với dữ liệu toán học
        let ac = ActorCritic::new(4, 8, 2, 42);
        let math_data = generate_micro_math_dataset();
        assert_eq!(math_data.len(), 5);

        let initial_probs = ac.forward_policy(&[0.5, 0.5, 0.5, 0.5]);
        assert_eq!(initial_probs.len(), 2);
    }

    #[test]
    fn test_math_verifier_float_tolerance() {
        let verifier = MathAnswerVerifier::with_tolerance(1e-3);
        let text = "<think>sqrt(2) approx 1.414</think><answer>1.4142</answer>";
        let res = verifier.verify(text, 1.4140);
        assert!(res.is_correct);
        assert!(res.has_valid_format);
        assert!((res.total_reward - 1.2).abs() < 1e-12);
    }

    #[test]
    fn test_format_model_response_tag_structure() {
        let valid_str = format_model_response("buoc 1: 2+2=4", 4.0, true);
        assert!(valid_str.contains("<think>") && valid_str.contains("</think>"));
        assert!(valid_str.contains("<answer>") && valid_str.contains("</answer>"));

        let invalid_str = format_model_response("buoc 1: 2+2=4", 4.0, false);
        assert!(!invalid_str.contains("<think>"));
        assert!(!invalid_str.contains("<answer>"));
    }
}
