//! # Chương 89: Độ bền Phục hồi Sự cố — Cổng Mô hình, Nhật ký WAL, Cây Phiên, Outbox, Saga & Giao thức MCP + A2A + ANP
//!
//! Thư viện hoàn chỉnh 100% chuẩn Rust 2024 thuần `std` giải quyết các bài toán sống còn
//! trong kỹ nghệ xây dựng Agent tự hành chịu lỗi (Crash-Resilient Autonomous Agent):
//! 1. `LlmGateway`: Phân loại lỗi `Fatal4xx` vs `RateLimit429` vs `Transient5xx`,
//!    cầu dao `CircuitBreaker` không bao giờ mở oan vì lỗi 4xx client,
//!    thuật toán giật lùi phân tán `DecorrelatedJitterBackoff`, và FinOps `stable_fraction`.
//! 2. `TurnWal`: Nhật ký ghi trước cưỡng chế bất biến cốt lõi **"Model-visible means logged"**,
//!    khung nhị phân toàn vẹn SHA-256, tự gọt đuôi rách (torn-tail truncation) khi sập nguồn.
//! 3. `SessionTree`: Cây hội thoại rẽ nhánh `/tree` không bao giờ ghi đè lịch sử cha,
//!    hỗ trợ phân nhánh độc lập và phát lại tuyến tính từng byte.
//! 4. `IdempotencyStore`: Kho lũy đẳng bền vững hai pha `Pending { lease_until } -> Committed { output_hash }`.
//! 5. `TransactionalOutbox`: Bộ đệm xuất giao dịch bảo đảm tính chất "at-least-once" cho sự kiện liên hệ thống.
//! 6. `OrchestratedSaga`: Bộ điều phối bù trừ ngược từ đĩa, bảo đảm tiếp tục hoàn tác kể cả khi có bước đơn lẻ lỗi.
//! 7. `OptimisticFileVersion` & `FencingToken`: Kiểm soát tương tranh lạc quan CAS `(version, sha256)`
//!    và rào chắn đơn điệu ngăn chặn hoàn toàn tác tử cũ (zombie agent) ghi đè.
//! 8. Ngăn xếp 3 giao thức kết nối: `McpJsonRpcRouter` (Model Context Protocol) +
//!    `A2aAgentNode` (Agent-to-Agent với `AgentCard` & `A2aTaskState`) +
//!    `AnpDiscoveryRegistry` (Agent Network Protocol định tuyến cân bằng tải).

use std::collections::{BTreeMap, HashMap};
use std::fmt;

// ============================================================================
// 0. BẢNG MÃ BĂM MẬT MÃ VÀ PHÁT SINH SỐ NGẪU NHIÊN THUẦN STD (SHA-256 & SPLITMIX64)
// ============================================================================

/// Giá trị băm SHA-256 256-bit bất biến.
#[derive(Debug, Clone, Copy, PartialEq, Eq, PartialOrd, Ord, Hash)]
pub struct Sha256Digest {
    pub bytes: [u8; 32],
}

impl Sha256Digest {
    /// Biểu diễn băm dưới dạng chuỗi hexa 64 ký tự thường.
    pub fn to_hex_string(&self) -> String {
        let mut s = String::with_capacity(64);
        for b in &self.bytes {
            s.push_str(&format!("{b:02x}"));
        }
        s
    }
}

impl fmt::Display for Sha256Digest {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        write!(f, "{}", self.to_hex_string())
    }
}

/// Bộ tính toán SHA-256 thuần Rust `std` tuân chuẩn FIPS 180-4.
#[derive(Debug, Clone)]
pub struct PureSha256 {
    state: [u32; 8],
    buffer: Vec<u8>,
    total_len_bytes: u64,
}

impl Default for PureSha256 {
    fn default() -> Self {
        Self::new()
    }
}

impl PureSha256 {
    const K: [u32; 64] = [
        0x428a2f98, 0x71374491, 0xb5c0fbcf, 0xe9b5dba5, 0x3956c25b, 0x59f111f1, 0x923f82a4,
        0xab1c5ed5, 0xd807aa98, 0x12835b01, 0x243185be, 0x550c7dc3, 0x72be5d74, 0x80deb1fe,
        0x9bdc06a7, 0xc19bf174, 0xe49b69c1, 0xefbe4786, 0x0fc19dc6, 0x240ca1cc, 0x2de92c6f,
        0x4a7484aa, 0x5cb0a9dc, 0x76f988da, 0x983e5152, 0xa831c66d, 0xb00327c8, 0xbf597fc7,
        0xc6e00bf3, 0xd5a79147, 0x06ca6351, 0x14292967, 0x27b70a85, 0x2e1b2138, 0x4d2c6dfc,
        0x53380d13, 0x650a7354, 0x766a0abb, 0x81c2c92e, 0x92722c85, 0xa2bfe8a1, 0xa81a664b,
        0xc24b8b70, 0xc76c51a3, 0xd192e819, 0xd6990624, 0xf40e3585, 0x106aa070, 0x19a4c116,
        0x1e376c08, 0x2748774c, 0x34b0bcb5, 0x391c0cb3, 0x4ed8aa4a, 0x5b9cca4f, 0x682e6ff3,
        0x748f82ee, 0x78a5636f, 0x84c87814, 0x8cc70208, 0x90befffa, 0xa4506ceb, 0xbef9a3f7,
        0xc67178f2,
    ];

    pub fn new() -> Self {
        Self {
            state: [
                0x6a09e667, 0xbb67ae85, 0x3c6ef372, 0xa54ff53a, 0x510e527f, 0x9b05688c, 0x1f83d9ab,
                0x5be0cd19,
            ],
            buffer: Vec::with_capacity(64),
            total_len_bytes: 0,
        }
    }

    pub fn update(&mut self, data: &[u8]) {
        self.total_len_bytes += data.len() as u64;
        let mut idx = 0;
        while idx < data.len() {
            let space = 64 - self.buffer.len();
            let chunk_len = space.min(data.len() - idx);
            self.buffer.extend_from_slice(&data[idx..idx + chunk_len]);
            idx += chunk_len;
            if self.buffer.len() == 64 {
                let block: [u8; 64] = self.buffer.as_slice().try_into().expect("khối 64 byte");
                Self::process_block(&mut self.state, &block);
                self.buffer.clear();
            }
        }
    }

    pub fn finalize(mut self) -> Sha256Digest {
        let bit_len = self.total_len_bytes * 8;
        self.buffer.push(0x80);
        while (self.buffer.len() % 64) != 56 {
            self.buffer.push(0x00);
        }
        self.buffer.extend_from_slice(&bit_len.to_be_bytes());

        let (chunks, _) = self.buffer.as_chunks::<64>();
        for block in chunks {
            Self::process_block(&mut self.state, block);
        }

        let mut out = [0u8; 32];
        for (i, word) in self.state.iter().enumerate() {
            out[i * 4..(i + 1) * 4].copy_from_slice(&word.to_be_bytes());
        }
        Sha256Digest { bytes: out }
    }

    pub fn digest(data: &[u8]) -> Sha256Digest {
        let mut hasher = Self::new();
        hasher.update(data);
        hasher.finalize()
    }

    fn process_block(state: &mut [u32; 8], block: &[u8; 64]) {
        let mut w = [0u32; 64];
        for (i, w_item) in w.iter_mut().take(16).enumerate() {
            *w_item = u32::from_be_bytes([
                block[i * 4],
                block[i * 4 + 1],
                block[i * 4 + 2],
                block[i * 4 + 3],
            ]);
        }
        for i in 16..64 {
            let s0 = w[i - 15].rotate_right(7) ^ w[i - 15].rotate_right(18) ^ (w[i - 15] >> 3);
            let s1 = w[i - 2].rotate_right(17) ^ w[i - 2].rotate_right(19) ^ (w[i - 2] >> 10);
            w[i] = w[i - 16]
                .wrapping_add(s0)
                .wrapping_add(w[i - 7])
                .wrapping_add(s1);
        }

        let mut a = state[0];
        let mut b = state[1];
        let mut c = state[2];
        let mut d = state[3];
        let mut e = state[4];
        let mut f = state[5];
        let mut g = state[6];
        let mut h = state[7];

        for (i, &w_i) in w.iter().enumerate() {
            let s1 = e.rotate_right(6) ^ e.rotate_right(11) ^ e.rotate_right(25);
            let ch = (e & f) ^ ((!e) & g);
            let temp1 = h
                .wrapping_add(s1)
                .wrapping_add(ch)
                .wrapping_add(Self::K[i])
                .wrapping_add(w_i);
            let s0 = a.rotate_right(2) ^ a.rotate_right(13) ^ a.rotate_right(22);
            let maj = (a & b) ^ (a & c) ^ (b & c);
            let temp2 = s0.wrapping_add(maj);

            h = g;
            g = f;
            f = e;
            e = d.wrapping_add(temp1);
            d = c;
            c = b;
            b = a;
            a = temp1.wrapping_add(temp2);
        }

        state[0] = state[0].wrapping_add(a);
        state[1] = state[1].wrapping_add(b);
        state[2] = state[2].wrapping_add(c);
        state[3] = state[3].wrapping_add(d);
        state[4] = state[4].wrapping_add(e);
        state[5] = state[5].wrapping_add(f);
        state[6] = state[6].wrapping_add(g);
        state[7] = state[7].wrapping_add(h);
    }
}

/// Bộ phát số ngẫu nhiên giả tất định SplitMix64 64-bit.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct SplitMix64 {
    pub state: u64,
}

impl SplitMix64 {
    pub fn new(seed: u64) -> Self {
        Self { state: seed }
    }

    pub fn next_u64(&mut self) -> u64 {
        self.state = self.state.wrapping_add(0x9e3779b97f4a7c15);
        let mut z = self.state;
        z = (z ^ (z >> 30)).wrapping_mul(0xbf58476d1ce4e5b9);
        z = (z ^ (z >> 27)).wrapping_mul(0x94d049bb133111eb);
        z ^ (z >> 31)
    }

    /// Trả về số thực ngẫu nhiên trong khoảng `[0.0, 1.0)`.
    pub fn next_f64(&mut self) -> f64 {
        let v = self.next_u64() >> 11;
        (v as f64) / ((1u64 << 53) as f64)
    }

    /// Sinh số nguyên ngẫu nhiên đều trong đoạn `[min, max]`.
    pub fn range_u64(&mut self, min: u64, max: u64) -> u64 {
        if min >= max {
            return min;
        }
        let span = max - min + 1;
        min + (self.next_u64() % span)
    }
}

// ============================================================================
// 1. CỔNG MÔ HÌNH (LLM GATEWAY, ERROR CLASSIFICATION, CIRCUIT BREAKER & FINOPS)
// ============================================================================

/// Phân loại lỗi mạng và nhà cung cấp LLM.
#[derive(Debug, Clone, PartialEq, Eq)]
pub enum ErrorClass {
    /// Lỗi client chết người: 400 Bad Request, 401 Unauthorized, 403 Forbidden.
    /// Tuyệt đối KHÔNG thử lại và KHÔNG mở CircuitBreaker.
    Fatal4xx { status_code: u16, message: String },
    /// Vượt ngưỡng tần suất gọi 429 Too Many Requests. Cần thử lại theo backoff.
    RateLimit429 { retry_after_ms: u64 },
    /// Lỗi máy chủ tạm thời: 500 Internal Error, 502 Bad Gateway, 503 Service Unavailable.
    /// Thử lại có giới hạn và tích lũy mở CircuitBreaker.
    Transient5xx { status_code: u16, message: String },
    /// Tràn cửa sổ ngữ cảnh context window của mô hình (cần cắt tỉa hoặc fallback mô hình cửa sổ lớn).
    ContextOverflow {
        max_tokens: usize,
        requested_tokens: usize,
    },
}

impl fmt::Display for ErrorClass {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            Self::Fatal4xx {
                status_code,
                message,
            } => {
                write!(f, "Fatal4xx({status_code}): {message}")
            }
            Self::RateLimit429 { retry_after_ms } => {
                write!(f, "RateLimit429: thử lại sau {retry_after_ms}ms")
            }
            Self::Transient5xx {
                status_code,
                message,
            } => {
                write!(f, "Transient5xx({status_code}): {message}")
            }
            Self::ContextOverflow {
                max_tokens,
                requested_tokens,
            } => {
                write!(
                    f,
                    "ContextOverflow: yêu cầu {requested_tokens} vượt trần {max_tokens}"
                )
            }
        }
    }
}

/// Trạng thái cầu dao ngắt mạch (Circuit Breaker).
#[derive(Debug, Clone, PartialEq, Eq)]
pub enum CircuitBreakerState {
    /// Bình thường: mạch đóng, cho phép mọi yêu cầu.
    Closed { consecutive_failures: u32 },
    /// Mở ngắt mạch: từ chối ngay lập tức để bảo vệ hạ tầng.
    Open {
        opened_at_tick: u64,
        cooldown_ticks: u64,
    },
    /// Thử nghiệm: mở một phần, gửi một yêu cầu thăm dò (probe).
    HalfOpen { probe_active: bool },
}

/// Cầu dao ngắt mạch thông minh: không bao giờ mở mạch oan vì lỗi Fatal4xx.
#[derive(Debug, Clone)]
pub struct CircuitBreaker {
    pub failure_threshold: u32,
    pub cooldown_ticks: u64,
    pub state: CircuitBreakerState,
}

impl CircuitBreaker {
    pub fn new(failure_threshold: u32, cooldown_ticks: u64) -> Self {
        Self {
            failure_threshold,
            cooldown_ticks,
            state: CircuitBreakerState::Closed {
                consecutive_failures: 0,
            },
        }
    }

    /// Kiểm tra xem tại thời điểm `current_tick` có được phép phát yêu cầu không.
    pub fn can_execute(&mut self, current_tick: u64) -> bool {
        match &mut self.state {
            CircuitBreakerState::Closed { .. } => true,
            CircuitBreakerState::Open {
                opened_at_tick,
                cooldown_ticks,
            } => {
                if current_tick >= *opened_at_tick + *cooldown_ticks {
                    self.state = CircuitBreakerState::HalfOpen { probe_active: true };
                    true
                } else {
                    false
                }
            }
            CircuitBreakerState::HalfOpen { probe_active } => {
                if !*probe_active {
                    *probe_active = true;
                    true
                } else {
                    false
                }
            }
        }
    }

    /// Ghi nhận phản hồi thành công từ mô hình.
    pub fn record_success(&mut self) {
        match self.state {
            CircuitBreakerState::HalfOpen { .. } | CircuitBreakerState::Closed { .. } => {
                self.state = CircuitBreakerState::Closed {
                    consecutive_failures: 0,
                };
            }
            CircuitBreakerState::Open { .. } => {}
        }
    }

    /// Ghi nhận lỗi: chỉ lỗi `Transient5xx` mới làm tăng bộ đếm mở mạch!
    /// `Fatal4xx`, `RateLimit429`, và `ContextOverflow` KHÔNG BAO GIỜ mở mạch.
    pub fn record_failure(&mut self, error: &ErrorClass, current_tick: u64) -> bool {
        let is_server_crash = matches!(error, ErrorClass::Transient5xx { .. });
        if !is_server_crash {
            return false;
        }

        match &mut self.state {
            CircuitBreakerState::Closed {
                consecutive_failures,
            } => {
                *consecutive_failures += 1;
                if *consecutive_failures >= self.failure_threshold {
                    self.state = CircuitBreakerState::Open {
                        opened_at_tick: current_tick,
                        cooldown_ticks: self.cooldown_ticks,
                    };
                    return true;
                }
            }
            CircuitBreakerState::HalfOpen { .. } => {
                self.state = CircuitBreakerState::Open {
                    opened_at_tick: current_tick,
                    cooldown_ticks: self.cooldown_ticks,
                };
                return true;
            }
            CircuitBreakerState::Open { .. } => {}
        }
        false
    }
}

/// Thuật toán độ trễ giật lùi phi tương quan (Decorrelated Jitter Backoff).
/// Công thức: `sleep = min(max_delay, uniform(base_delay, prev_delay * 3))`
#[derive(Debug, Clone)]
pub struct DecorrelatedJitterBackoff {
    pub base_delay_ms: u64,
    pub max_delay_ms: u64,
    pub rng: SplitMix64,
}

impl DecorrelatedJitterBackoff {
    pub fn new(base_delay_ms: u64, max_delay_ms: u64, seed: u64) -> Self {
        Self {
            base_delay_ms,
            max_delay_ms,
            rng: SplitMix64::new(seed),
        }
    }

    pub fn next_delay(&mut self, prev_delay_ms: u64) -> u64 {
        let ceiling = prev_delay_ms.saturating_mul(3).max(self.base_delay_ms);
        let randomized = self.rng.range_u64(self.base_delay_ms, ceiling);
        randomized.min(self.max_delay_ms)
    }
}

/// Bậc mô hình ngôn ngữ trong cấu hình chuyển dự phòng (Fallback Hierarchy).
#[derive(Debug, Clone, Copy, PartialEq, Eq, PartialOrd, Ord)]
pub enum ModelTier {
    /// Mô hình chính: trí tuệ cao nhất, chi phí cao nhất (ví dụ: Opus / Pro).
    Primary,
    /// Mô hình phụ: tốc độ nhanh hơn, chi phí thấp hơn (ví dụ: Sonnet / Flash).
    Secondary,
    /// Mô hình cứu trợ cục bộ: luôn sẵn sàng kể cả khi mất Internet.
    LocalFallback,
}

/// Yêu cầu gửi tới Cổng mô hình LLM.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct LlmRequest {
    pub prompt: String,
    pub estimated_tokens: usize,
    pub idempotency_key: String,
    pub allow_fallback: bool,
}

/// Phản hồi nhận được từ Cổng mô hình LLM.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct LlmResponse {
    pub content: String,
    pub model_used: String,
    pub tokens_used: usize,
    pub cost_cents: u64,
}

/// Bộ mô phỏng nhà cung cấp LLM để kiểm thử tất định 100%.
pub trait LlmProviderMock {
    fn call(&self, model: &str, req: &LlmRequest) -> Result<LlmResponse, ErrorClass>;
}

/// Cổng mô hình `LlmGateway` tích hợp quản trị FinOps, Circuit Breaker và Fallback.
pub struct LlmGateway<P: LlmProviderMock> {
    pub provider: P,
    pub primary_model: String,
    pub secondary_model: String,
    pub local_model: String,
    pub circuit_breaker: CircuitBreaker,
    pub backoff: DecorrelatedJitterBackoff,
    pub total_budget_cents: u64,
    pub spent_cents: u64,
    /// Tỷ lệ phần trăm định tuyến vào mô hình ổn định (FinOps `stable_fraction`: 0..=100).
    pub stable_fraction: u32,
    pub rng: SplitMix64,
}

impl<P: LlmProviderMock> LlmGateway<P> {
    pub fn new(
        provider: P,
        primary_model: String,
        secondary_model: String,
        local_model: String,
        total_budget_cents: u64,
        stable_fraction: u32,
        seed: u64,
    ) -> Self {
        Self {
            provider,
            primary_model,
            secondary_model,
            local_model,
            circuit_breaker: CircuitBreaker::new(3, 100),
            backoff: DecorrelatedJitterBackoff::new(50, 2000, seed),
            total_budget_cents,
            spent_cents: 0,
            stable_fraction: stable_fraction.min(100),
            rng: SplitMix64::new(seed.wrapping_add(101)),
        }
    }

    /// Thực hiện gọi mô hình kèm chiến lược chuyển bậc dự phòng (fallback) tự động.
    pub fn execute(&mut self, req: &LlmRequest, current_tick: u64) -> Result<LlmResponse, String> {
        // Kiểm tra trần ngân sách FinOps
        if self.spent_cents >= self.total_budget_cents {
            return Err("Ngân sách FinOps đã cạn kiệt (Budget Exhausted)".to_string());
        }

        // Quyết định bậc mô hình ưu tiên dựa vào stable_fraction
        let dice = self.rng.range_u64(1, 100) as u32;
        let preferred_model = if dice <= self.stable_fraction {
            &self.primary_model
        } else {
            &self.secondary_model
        };

        // 1. Thử gọi mô hình ưu tiên nếu cầu dao chưa mở
        if self.circuit_breaker.can_execute(current_tick) {
            match self.provider.call(preferred_model, req) {
                Ok(resp) => {
                    self.circuit_breaker.record_success();
                    self.spent_cents = self.spent_cents.saturating_add(resp.cost_cents);
                    return Ok(resp);
                }
                Err(err) => {
                    // Fatal4xx: dừng ngay lập tức, không thử lại, không mở cầu dao!
                    if matches!(err, ErrorClass::Fatal4xx { .. }) {
                        return Err(format!("Lỗi client nghiêm trọng không thử lại: {err}"));
                    }
                    self.circuit_breaker.record_failure(&err, current_tick);
                    if !req.allow_fallback {
                        return Err(format!("Gọi mô hình thất bại và cấm fallback: {err}"));
                    }
                }
            }
        }

        // 2. Chuyển sang mô hình phụ (Secondary) nếu mô hình ưu tiên thất bại
        if preferred_model != &self.secondary_model {
            match self.provider.call(&self.secondary_model, req) {
                Ok(resp) => {
                    self.spent_cents = self.spent_cents.saturating_add(resp.cost_cents);
                    return Ok(resp);
                }
                Err(err) => {
                    if matches!(err, ErrorClass::Fatal4xx { .. }) {
                        return Err(format!("Lỗi client nghiêm trọng tại secondary: {err}"));
                    }
                }
            }
        }

        // 3. Chuyển sang mô hình cứu trợ cục bộ (Local Fallback)
        match self.provider.call(&self.local_model, req) {
            Ok(resp) => {
                self.spent_cents = self.spent_cents.saturating_add(resp.cost_cents);
                Ok(resp)
            }
            Err(err) => Err(format!("Tất cả các bậc mô hình đã kiệt quệ: {err}")),
        }
    }
}

// ============================================================================
// 2. NHẬT KÝ GHI TRƯỚC (TURN WAL: "MODEL-VISIBLE MEANS LOGGED")
// ============================================================================

/// Loại mục nhật ký trong một lượt hội thoại của Agent.
#[derive(Debug, Clone, PartialEq, Eq)]
pub enum WalPayloadKind {
    SystemPrompt,
    UserMessage,
    ModelInferenceOutput,
    ToolCallExecution,
    ToolResultReturned,
}

/// Khung nhị phân của một mục nhật ký WAL kèm mã kiểm tra SHA-256.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct WalFrame {
    pub frame_seq: u64,
    pub session_id: String,
    pub kind: WalPayloadKind,
    pub payload_bytes: Vec<u8>,
    pub checksum: Sha256Digest,
}

/// Bất biến cốt lõi: `"Model-visible means logged"`.
/// Mọi token/chuỗi được đưa vào cửa sổ ngữ cảnh mà mô hình nhìn thấy BẮT BUỘC
/// phải được ghi nhận vào WAL trước khi gửi gói tin qua socket.
pub struct TurnWal {
    pub frames: Vec<WalFrame>,
    pub next_seq: u64,
}

impl Default for TurnWal {
    fn default() -> Self {
        Self::new()
    }
}

impl TurnWal {
    pub fn new() -> Self {
        Self {
            frames: Vec::new(),
            next_seq: 1,
        }
    }

    /// Ghi thêm một khung mới vào WAL. Tính toán mã băm SHA-256 bảo vệ toàn vẹn.
    pub fn append(&mut self, session_id: &str, kind: WalPayloadKind, payload: &[u8]) -> &WalFrame {
        let seq = self.next_seq;
        self.next_seq += 1;

        let mut hash_input = Vec::new();
        hash_input.extend_from_slice(&seq.to_be_bytes());
        hash_input.extend_from_slice(session_id.as_bytes());
        hash_input.extend_from_slice(payload);
        let checksum = PureSha256::digest(&hash_input);

        let frame = WalFrame {
            frame_seq: seq,
            session_id: session_id.to_string(),
            kind,
            payload_bytes: payload.to_vec(),
            checksum,
        };
        self.frames.push(frame);
        self.frames.last().expect("vừa push")
    }

    /// Kiểm chứng bất biến `"Model-visible means logged"`:
    /// Chuỗi văn bản dự kiến đưa vào model có khớp hoàn toàn với một khung đã được log chưa?
    pub fn verify_model_visible_logged(&self, session_id: &str, expected_content: &[u8]) -> bool {
        self.frames.iter().any(|f| {
            f.session_id == session_id
                && (f.payload_bytes == expected_content
                    || f.payload_bytes
                        .windows(expected_content.len())
                        .any(|window| window == expected_content))
        })
    }

    /// Xuất chuỗi nhị phân mô phỏng lưu trữ trên đĩa.
    pub fn serialize_to_bytes(&self) -> Vec<u8> {
        let mut buf = Vec::new();
        for f in &self.frames {
            // Định dạng khung: [SEQ: 8b][KIND: 1b][SESS_LEN: 2b][SESS][PAYLOAD_LEN: 4b][PAYLOAD][CHECKSUM: 32b]
            buf.extend_from_slice(&f.frame_seq.to_be_bytes());
            let kind_byte = match f.kind {
                WalPayloadKind::SystemPrompt => 0u8,
                WalPayloadKind::UserMessage => 1u8,
                WalPayloadKind::ModelInferenceOutput => 2u8,
                WalPayloadKind::ToolCallExecution => 3u8,
                WalPayloadKind::ToolResultReturned => 4u8,
            };
            buf.push(kind_byte);
            let sess_bytes = f.session_id.as_bytes();
            buf.extend_from_slice(&(sess_bytes.len() as u16).to_be_bytes());
            buf.extend_from_slice(sess_bytes);
            buf.extend_from_slice(&(f.payload_bytes.len() as u32).to_be_bytes());
            buf.extend_from_slice(&f.payload_bytes);
            buf.extend_from_slice(&f.checksum.bytes);
        }
        buf
    }

    /// Tái tạo trạng thái WAL từ dòng byte đĩa. Tự động cắt bỏ phần đuôi rách (torn tail)
    /// nếu sập nguồn đột ngột giữa chừng khi đang ghi một khung dở dang.
    pub fn recover_from_bytes(bytes: &[u8]) -> (Self, usize) {
        let mut wal = Self::new();
        let mut offset = 0;
        let mut recovered_count = 0;

        while offset < bytes.len() {
            // Cần tối thiểu 8 + 1 + 2 + 4 + 32 = 47 byte cho phần tiêu đề và checksum tối thiểu
            if offset + 47 > bytes.len() {
                // Khung rách phần đầu: dừng và gọt đuôi rách
                break;
            }

            let seq =
                u64::from_be_bytes(bytes[offset..offset + 8].try_into().expect("đọc u64 seq"));
            let kind = match bytes[offset + 8] {
                0 => WalPayloadKind::SystemPrompt,
                1 => WalPayloadKind::UserMessage,
                2 => WalPayloadKind::ModelInferenceOutput,
                3 => WalPayloadKind::ToolCallExecution,
                4 => WalPayloadKind::ToolResultReturned,
                _ => break, // Byte loại không hợp lệ -> dừng
            };
            let sess_len = u16::from_be_bytes(
                bytes[offset + 9..offset + 11]
                    .try_into()
                    .expect("đọc u16 sess_len"),
            ) as usize;

            let after_sess = offset + 11 + sess_len;
            if after_sess + 4 > bytes.len() {
                break;
            }

            let sess_id = match String::from_utf8(bytes[offset + 11..after_sess].to_vec()) {
                Ok(s) => s,
                Err(_) => break,
            };

            let payload_len = u32::from_be_bytes(
                bytes[after_sess..after_sess + 4]
                    .try_into()
                    .expect("đọc u32 payload_len"),
            ) as usize;
            let after_payload = after_sess + 4 + payload_len;
            if after_payload + 32 > bytes.len() {
                // Khung rách phần thân payload hoặc checksum: gọt bỏ
                break;
            }

            let payload = &bytes[after_sess + 4..after_payload];
            let expected_checksum: [u8; 32] = bytes[after_payload..after_payload + 32]
                .try_into()
                .expect("đọc 32 byte sha");

            // Kiểm tra toàn vẹn băm
            let mut hash_input = Vec::new();
            hash_input.extend_from_slice(&seq.to_be_bytes());
            hash_input.extend_from_slice(sess_id.as_bytes());
            hash_input.extend_from_slice(payload);
            let computed_checksum = PureSha256::digest(&hash_input);

            if computed_checksum.bytes != expected_checksum {
                // Checksum không khớp (dữ liệu biến chất do sập đĩa): gọt đuôi rách ngay lập tức
                break;
            }

            wal.frames.push(WalFrame {
                frame_seq: seq,
                session_id: sess_id,
                kind,
                payload_bytes: payload.to_vec(),
                checksum: Sha256Digest {
                    bytes: expected_checksum,
                },
            });
            wal.next_seq = seq + 1;
            recovered_count += 1;
            offset = after_payload + 32;
        }

        (wal, recovered_count)
    }
}

// ============================================================================
// 3. CÂY PHIÊN RẼ NHÁNH (SESSION TREE & TREE-REPLAY)
// ============================================================================

/// Định danh nút trong Cây phiên.
pub type SessionNodeId = u64;

/// Vai trò của thực thể phát ngôn trong phiên.
#[derive(Debug, Clone, PartialEq, Eq)]
pub enum SessionRole {
    System,
    User,
    Assistant,
    Tool,
}

/// Một nút dữ liệu trong cây phân nhánh `SessionTree`.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct SessionNode {
    pub node_id: SessionNodeId,
    pub parent_id: Option<SessionNodeId>,
    pub role: SessionRole,
    pub content: String,
    pub metadata: BTreeMap<String, String>,
}

/// Cây phiên rẽ nhánh `SessionTree`: cho phép người dùng thử các hướng rẽ nhánh `/tree`
/// mà không bao giờ làm mất dữ liệu của các nhánh cũ.
#[derive(Debug, Clone)]
pub struct SessionTree {
    pub nodes: HashMap<SessionNodeId, SessionNode>,
    pub children: HashMap<SessionNodeId, Vec<SessionNodeId>>,
    pub roots: Vec<SessionNodeId>,
    pub next_node_id: SessionNodeId,
}

impl Default for SessionTree {
    fn default() -> Self {
        Self::new()
    }
}

impl SessionTree {
    pub fn new() -> Self {
        Self {
            nodes: HashMap::new(),
            children: HashMap::new(),
            roots: Vec::new(),
            next_node_id: 1,
        }
    }

    /// Tạo một phiên mới với nút gốc (thường là System Prompt hoặc User Greeting).
    pub fn create_root(&mut self, role: SessionRole, content: &str) -> SessionNodeId {
        let id = self.next_node_id;
        self.next_node_id += 1;

        let node = SessionNode {
            node_id: id,
            parent_id: None,
            role,
            content: content.to_string(),
            metadata: BTreeMap::new(),
        };
        self.nodes.insert(id, node);
        self.roots.push(id);
        id
    }

    /// Rẽ nhánh hoặc nối tiếp từ một nút cha bất kỳ.
    pub fn fork_branch(
        &mut self,
        parent_id: SessionNodeId,
        role: SessionRole,
        content: &str,
    ) -> Result<SessionNodeId, String> {
        if !self.nodes.contains_key(&parent_id) {
            return Err(format!("Nút cha {parent_id} không tồn tại"));
        }

        let id = self.next_node_id;
        self.next_node_id += 1;

        let node = SessionNode {
            node_id: id,
            parent_id: Some(parent_id),
            role,
            content: content.to_string(),
            metadata: BTreeMap::new(),
        };
        self.nodes.insert(id, node);
        self.children.entry(parent_id).or_default().push(id);
        Ok(id)
    }

    /// Tái tạo tuyến tính toàn bộ lịch sử từ nút gốc tới nút lá chỉ định (linear replay path).
    pub fn replay_path(&self, leaf_id: SessionNodeId) -> Result<Vec<SessionNode>, String> {
        let mut path = Vec::new();
        let mut current_id = Some(leaf_id);

        while let Some(id) = current_id {
            let node = self
                .nodes
                .get(&id)
                .ok_or_else(|| format!("Đứt gãy con trỏ tổ tiên tại nút {id}"))?;
            path.push(node.clone());
            current_id = node.parent_id;
        }

        path.reverse();
        Ok(path)
    }

    /// Liệt kê tất cả các nút con trực tiếp của một nút.
    pub fn list_children(&self, parent_id: SessionNodeId) -> Vec<SessionNodeId> {
        self.children.get(&parent_id).cloned().unwrap_or_default()
    }
}

// ============================================================================
// 4. KHO LŨY ĐẲNG HAI PHA (TWO-PHASE IDEMPOTENCY STORE)
// ============================================================================

/// Trạng thái của một khóa lũy đẳng.
#[derive(Debug, Clone, PartialEq, Eq)]
pub enum IdempotencyPhase {
    /// Đang xử lý: được giữ bởi một lease có hạn định `lease_until_tick`.
    Pending {
        lease_until_tick: u64,
        request_hash: Sha256Digest,
    },
    /// Đã xử lý xong: lưu kết quả đầu ra đã cam kết và hash phản hồi.
    Committed {
        output_hash: Sha256Digest,
        response_payload: Vec<u8>,
    },
}

/// Hành động mà bên gọi cần thực hiện khi yêu cầu khóa lũy đẳng.
#[derive(Debug, Clone, PartialEq, Eq)]
pub enum IdempotencyAction {
    /// Được phép thực thi tác vụ mới.
    ExecuteNew,
    /// Đã hoàn thành trước đó: trả về ngay kết quả lưu trong cache, không gọi lại tool.
    ReturnCached { response_payload: Vec<u8> },
    /// Đang có một tiến trình khác xử lý (chưa hết hạn lease). Phải đợi hoặc từ chối.
    InFlightConflict,
    /// Thu hồi thành công lease đã hết hạn từ một tiến trình bị sập trước đó.
    ReclaimExpiredLease,
}

/// Kho kiểm soát tính lũy đẳng bền vững `IdempotencyStore`.
#[derive(Debug, Clone)]
pub struct IdempotencyStore {
    pub records: HashMap<String, IdempotencyPhase>,
}

impl Default for IdempotencyStore {
    fn default() -> Self {
        Self::new()
    }
}

impl IdempotencyStore {
    pub fn new() -> Self {
        Self {
            records: HashMap::new(),
        }
    }

    /// Bắt đầu một giao dịch có tính lũy đẳng.
    pub fn begin_execution(
        &mut self,
        key: &str,
        request_hash: Sha256Digest,
        current_tick: u64,
        lease_duration: u64,
    ) -> IdempotencyAction {
        if let Some(phase) = self.records.get_mut(key) {
            match phase {
                IdempotencyPhase::Committed {
                    response_payload, ..
                } => IdempotencyAction::ReturnCached {
                    response_payload: response_payload.clone(),
                },
                IdempotencyPhase::Pending {
                    lease_until_tick,
                    request_hash: old_hash,
                } => {
                    if current_tick < *lease_until_tick {
                        IdempotencyAction::InFlightConflict
                    } else {
                        // Lease đã quá hạn (tiến trình trước bị crash): cho phép thu hồi
                        *lease_until_tick = current_tick + lease_duration;
                        *old_hash = request_hash;
                        IdempotencyAction::ReclaimExpiredLease
                    }
                }
            }
        } else {
            self.records.insert(
                key.to_string(),
                IdempotencyPhase::Pending {
                    lease_until_tick: current_tick + lease_duration,
                    request_hash,
                },
            );
            IdempotencyAction::ExecuteNew
        }
    }

    /// Cam kết kết quả của giao dịch lũy đẳng.
    pub fn commit_execution(&mut self, key: &str, response_payload: &[u8]) -> Result<(), String> {
        let output_hash = PureSha256::digest(response_payload);
        if let Some(record) = self.records.get_mut(key) {
            *record = IdempotencyPhase::Committed {
                output_hash,
                response_payload: response_payload.to_vec(),
            };
            Ok(())
        } else {
            Err(format!("Khóa {key} không tồn tại để commit"))
        }
    }
}

// ============================================================================
// 5. HỘP THƯ RA GIAO DỊCH (TRANSACTIONAL OUTBOX)
// ============================================================================

/// Trạng thái phát tán của sự kiện trong Outbox.
#[derive(Debug, Clone, PartialEq, Eq)]
pub enum OutboxStatus {
    Staged,
    Dispatched { dispatched_at_tick: u64 },
}

/// Bản ghi sự kiện cần phát tán trong `TransactionalOutbox`.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct OutboxEvent {
    pub event_id: u64,
    pub topic: String,
    pub payload: Vec<u8>,
    pub status: OutboxStatus,
    pub created_at_tick: u64,
}

/// Hộp thư ra giao dịch bảo đảm tính tin cậy tuyệt đối:
/// Dữ liệu nội bộ và sự kiện bắn ra bên ngoài luôn được ghi nhận trong cùng 1 khối nguyên tử.
#[derive(Debug, Clone)]
pub struct TransactionalOutbox {
    pub events: Vec<OutboxEvent>,
    pub next_event_id: u64,
}

impl Default for TransactionalOutbox {
    fn default() -> Self {
        Self::new()
    }
}

impl TransactionalOutbox {
    pub fn new() -> Self {
        Self {
            events: Vec::new(),
            next_event_id: 1,
        }
    }

    /// Xếp một sự kiện vào Outbox ở trạng thái `Staged`.
    pub fn stage_event(&mut self, topic: &str, payload: &[u8], current_tick: u64) -> u64 {
        let id = self.next_event_id;
        self.next_event_id += 1;
        self.events.push(OutboxEvent {
            event_id: id,
            topic: topic.to_string(),
            payload: payload.to_vec(),
            status: OutboxStatus::Staged,
            created_at_tick: current_tick,
        });
        id
    }

    /// Lấy danh sách các sự kiện đang chờ bắn ra hệ thống bên ngoài.
    pub fn sweep_pending(&self, limit: usize) -> Vec<OutboxEvent> {
        self.events
            .iter()
            .filter(|e| matches!(e.status, OutboxStatus::Staged))
            .take(limit)
            .cloned()
            .collect()
    }

    /// Đánh dấu sự kiện đã được gửi đi thành công tại nhịp `current_tick`.
    pub fn mark_dispatched(&mut self, event_id: u64, current_tick: u64) -> bool {
        if let Some(ev) = self.events.iter_mut().find(|e| e.event_id == event_id) {
            ev.status = OutboxStatus::Dispatched {
                dispatched_at_tick: current_tick,
            };
            true
        } else {
            false
        }
    }
}

// ============================================================================
// 6. ĐIỀU PHỐI SAGA BÙ TRỪ TỰ ĐỘNG (ORCHESTRATED SAGA WITH DISK LOG)
// ============================================================================

/// Trạng thái của từng bước trong Saga.
#[derive(Debug, Clone, PartialEq, Eq)]
pub enum SagaStepStatus {
    Pending,
    ExecutedSuccessfully,
    CompensatedSuccessfully,
    CompensationFailed { error: String },
}

/// Một bước tác vụ có định nghĩa hành động tiến (forward) và hành động bù trừ hoàn tác (compensate).
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct SagaStepDef {
    pub step_id: u32,
    pub name: String,
    pub forward_action: String,
    pub compensation_action: String,
}

/// Nhật ký ghi trạng thái thực thi Saga.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct SagaLogEntry {
    pub step_id: u32,
    pub status: SagaStepStatus,
}

/// Bộ điều phối Saga: khi một bước giữa chừng thất bại, nó duyệt ngược `step_k -> ... -> step_1`
/// để hoàn tác. Bất biến quan trọng: nếu 1 bước bù trừ lỗi, hệ thống KHÔNG ĐƯỢC dừng
/// mà phải ghi nhận lỗi và tiếp tục bù trừ các bước còn lại!
pub struct OrchestratedSaga {
    pub steps: Vec<SagaStepDef>,
    pub execution_log: Vec<SagaLogEntry>,
}

impl OrchestratedSaga {
    pub fn new(steps: Vec<SagaStepDef>) -> Self {
        Self {
            steps,
            execution_log: Vec::new(),
        }
    }

    /// Chạy toàn bộ luồng Saga.
    /// `runner` là hàm mô phỏng nhận vào tên action và trả về Ok hoặc Err.
    pub fn run<F>(&mut self, mut runner: F) -> Result<(), Vec<String>>
    where
        F: FnMut(&str, bool) -> Result<(), String>,
    {
        let mut executed_indices = Vec::new();

        // 1. Chạy xuôi các bước
        for (idx, step) in self.steps.iter().enumerate() {
            match runner(&step.forward_action, false) {
                Ok(()) => {
                    self.execution_log.push(SagaLogEntry {
                        step_id: step.step_id,
                        status: SagaStepStatus::ExecutedSuccessfully,
                    });
                    executed_indices.push(idx);
                }
                Err(forward_err) => {
                    // Bước này gãy -> kích hoạt hoàn tác bù trừ ngược!
                    let mut comp_errors = Vec::new();
                    comp_errors.push(format!(
                        "Bước {} ({}) thất bại: {}",
                        step.step_id, step.name, forward_err
                    ));

                    // Duyệt ngược từ bước vừa chạy thành công gần nhất về bước 0
                    for &rev_idx in executed_indices.iter().rev() {
                        let comp_step = &self.steps[rev_idx];
                        match runner(&comp_step.compensation_action, true) {
                            Ok(()) => {
                                self.execution_log.push(SagaLogEntry {
                                    step_id: comp_step.step_id,
                                    status: SagaStepStatus::CompensatedSuccessfully,
                                });
                            }
                            Err(comp_err) => {
                                // Ghi nhận lỗi nhưng TIẾP TỤC các bước sau!
                                self.execution_log.push(SagaLogEntry {
                                    step_id: comp_step.step_id,
                                    status: SagaStepStatus::CompensationFailed {
                                        error: comp_err.clone(),
                                    },
                                });
                                comp_errors.push(format!(
                                    "Hoàn tác bước {} ({}) gặp lỗi: {}",
                                    comp_step.step_id, comp_step.name, comp_err
                                ));
                            }
                        }
                    }

                    return Err(comp_errors);
                }
            }
        }

        Ok(())
    }
}

// ============================================================================
// 7. KHÓA LẠC QUAN & RÀO CHẮN ĐƠN ĐIỆU (OPTIMISTIC FILE VERSION & FENCING TOKEN)
// ============================================================================

/// Quản lý phiên bản tệp lạc quan CAS bằng cặp `(version: u64, sha256: [u8; 32])`.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct OptimisticFileVersion {
    pub file_path: String,
    pub version: u64,
    pub content_hash: Sha256Digest,
    pub content_bytes: Vec<u8>,
}

impl OptimisticFileVersion {
    pub fn new(file_path: &str, initial_content: &[u8]) -> Self {
        let content_hash = PureSha256::digest(initial_content);
        Self {
            file_path: file_path.to_string(),
            version: 1,
            content_hash,
            content_bytes: initial_content.to_vec(),
        }
    }

    /// Thao tác Compare-And-Swap (CAS): chỉ cho phép ghi đè nếu phiên bản và mã băm
    /// mà bên yêu cầu nắm giữ trùng khớp 100% với trạng thái trên đĩa.
    pub fn compare_and_swap(
        &mut self,
        expected_version: u64,
        expected_hash: Sha256Digest,
        new_content: &[u8],
    ) -> Result<u64, String> {
        if self.version != expected_version {
            return Err(format!(
                "Xung đột phiên bản CAS: kỳ vọng phiên bản {expected_version}, thực tế trên đĩa là {}",
                self.version
            ));
        }
        if self.content_hash != expected_hash {
            return Err(format!(
                "Xung đột nội dung CAS: hash kỳ vọng {} khác hash hiện tại {}",
                expected_hash.to_hex_string(),
                self.content_hash.to_hex_string()
            ));
        }

        let new_hash = PureSha256::digest(new_content);
        self.version += 1;
        self.content_hash = new_hash;
        self.content_bytes = new_content.to_vec();
        Ok(self.version)
    }
}

/// Thẻ rào chắn đơn điệu (Fencing Token) ngăn chặn tác tử zombie hết hạn ghi đè tài nguyên.
#[derive(Debug, Clone, Copy, PartialEq, Eq, PartialOrd, Ord)]
pub struct FencingToken {
    pub epoch: u64,
}

/// Bộ kiểm tra rào chắn đơn điệu.
#[derive(Debug, Clone)]
pub struct FencingController {
    pub current_epoch: u64,
}

impl Default for FencingController {
    fn default() -> Self {
        Self::new()
    }
}

impl FencingController {
    pub fn new() -> Self {
        Self { current_epoch: 0 }
    }

    /// Cấp phát một thẻ rào chắn mới có epoch tăng đơn điệu nghiêm ngặt.
    pub fn issue_next_token(&mut self) -> FencingToken {
        self.current_epoch += 1;
        FencingToken {
            epoch: self.current_epoch,
        }
    }

    /// Xác minh tính hợp lệ của token gửi kèm yêu cầu.
    /// Nếu token có epoch nhỏ hơn epoch hiện tại của tài nguyên -> Bác bỏ ngay lập tức!
    pub fn validate_and_fence(&self, token: &FencingToken) -> Result<(), String> {
        if token.epoch < self.current_epoch {
            Err(format!(
                "Zombie Agent bị chặn: Thẻ epoch {} nhỏ hơn epoch hiện tại {}",
                token.epoch, self.current_epoch
            ))
        } else {
            Ok(())
        }
    }
}

// ============================================================================
// 8. NGĂN XẾP 3 GIAO THỨC TÁC TỬ (MCP + A2A + ANP)
// ============================================================================

// --- Giao thức 1: MCP (Model Context Protocol qua JSON-RPC 2.0 Router) ---

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct McpRequest {
    pub jsonrpc: String,
    pub id: u64,
    pub method: String,
    pub params: BTreeMap<String, String>,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct McpResponse {
    pub jsonrpc: String,
    pub id: u64,
    pub result: Option<String>,
    pub error: Option<String>,
}

pub struct McpJsonRpcRouter {
    pub registered_tools: HashMap<String, String>, // tool_name -> description
}

impl Default for McpJsonRpcRouter {
    fn default() -> Self {
        Self::new()
    }
}

impl McpJsonRpcRouter {
    pub fn new() -> Self {
        Self {
            registered_tools: HashMap::new(),
        }
    }

    pub fn register_tool(&mut self, name: &str, description: &str) {
        self.registered_tools
            .insert(name.to_string(), description.to_string());
    }

    pub fn handle_request(&self, req: &McpRequest) -> McpResponse {
        if req.jsonrpc != "2.0" {
            return McpResponse {
                jsonrpc: "2.0".to_string(),
                id: req.id,
                result: None,
                error: Some("Phiên bản giao thức không hỗ trợ (chỉ chấp nhận '2.0')".to_string()),
            };
        }

        match req.method.as_str() {
            "tools/list" => {
                let mut tools_summary = Vec::new();
                for (name, desc) in &self.registered_tools {
                    tools_summary.push(format!("{name}: {desc}"));
                }
                tools_summary.sort();
                McpResponse {
                    jsonrpc: "2.0".to_string(),
                    id: req.id,
                    result: Some(tools_summary.join("; ")),
                    error: None,
                }
            }
            "tools/call" => {
                let tool_name = req.params.get("name").map(|s| s.as_str()).unwrap_or("");
                if let Some(desc) = self.registered_tools.get(tool_name) {
                    let arg = req.params.get("arg").map(|s| s.as_str()).unwrap_or("");
                    McpResponse {
                        jsonrpc: "2.0".to_string(),
                        id: req.id,
                        result: Some(format!(
                            "Thực thi thành công {tool_name} [{desc}] với arg={arg}"
                        )),
                        error: None,
                    }
                } else {
                    McpResponse {
                        jsonrpc: "2.0".to_string(),
                        id: req.id,
                        result: None,
                        error: Some(format!(
                            "Công cụ '{tool_name}' không tồn tại trong danh mục MCP"
                        )),
                    }
                }
            }
            "ping" => McpResponse {
                jsonrpc: "2.0".to_string(),
                id: req.id,
                result: Some("pong".to_string()),
                error: None,
            },
            other => McpResponse {
                jsonrpc: "2.0".to_string(),
                id: req.id,
                result: None,
                error: Some(format!("Phương thức không được hỗ trợ: '{other}'")),
            },
        }
    }
}

// --- Giao thức 2: A2A (Agent-to-Agent Communication) ---

/// Thẻ năng lực của tác tử trong mạng lưới A2A (AgentCard).
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct AgentCard {
    pub agent_id: String,
    pub name: String,
    pub capabilities: Vec<String>,
    pub cost_tier: u8,
}

/// Máy trạng thái của một tác vụ A2A.
#[derive(Debug, Clone, PartialEq, Eq)]
pub enum A2aTaskState {
    Submitted { prompt: String },
    Working { worker_agent_id: String },
    Completed { result: String },
    Failed { reason: String },
}

/// Nút tác tử tham gia truyền thông Agent-to-Agent.
#[derive(Debug, Clone)]
pub struct A2aAgentNode {
    pub card: AgentCard,
    pub tasks: HashMap<u64, A2aTaskState>,
    pub next_task_id: u64,
}

impl A2aAgentNode {
    pub fn new(card: AgentCard) -> Self {
        Self {
            card,
            tasks: HashMap::new(),
            next_task_id: 1,
        }
    }

    /// Nhận ủy quyền một tác vụ từ Agent khác.
    pub fn submit_task(&mut self, prompt: &str) -> u64 {
        let task_id = self.next_task_id;
        self.next_task_id += 1;
        self.tasks.insert(
            task_id,
            A2aTaskState::Submitted {
                prompt: prompt.to_string(),
            },
        );
        task_id
    }

    /// Bắt đầu xử lý tác vụ.
    pub fn start_working(&mut self, task_id: u64) -> Result<(), String> {
        if let Some(task) = self.tasks.get_mut(&task_id) {
            match task {
                A2aTaskState::Submitted { .. } => {
                    *task = A2aTaskState::Working {
                        worker_agent_id: self.card.agent_id.clone(),
                    };
                    Ok(())
                }
                _ => Err("Chỉ có thể chuyển sang Working từ trạng thái Submitted".to_string()),
            }
        } else {
            Err(format!("Không tìm thấy tác vụ {task_id}"))
        }
    }

    /// Hoàn thành tác vụ và trả kết quả.
    pub fn complete_task(&mut self, task_id: u64, result: &str) -> Result<(), String> {
        if let Some(task) = self.tasks.get_mut(&task_id) {
            match task {
                A2aTaskState::Working { .. } => {
                    *task = A2aTaskState::Completed {
                        result: result.to_string(),
                    };
                    Ok(())
                }
                _ => Err("Chỉ có thể Completed khi đang Working".to_string()),
            }
        } else {
            Err(format!("Không tìm thấy tác vụ {task_id}"))
        }
    }
}

// --- Giao thức 3: ANP (Agent Network Protocol Discovery & Least-Loaded Routing) ---

/// Mục đăng ký khám phá mạng lưới tác tử ANP.
#[derive(Debug, Clone)]
pub struct AnpAgentRegistration {
    pub card: AgentCard,
    pub in_flight_load: usize,
    pub is_healthy: bool,
}

/// Sổ đăng bạ khám phá mạng tác tử ANP (Agent Network Protocol Discovery Registry).
#[derive(Debug, Clone)]
pub struct AnpDiscoveryRegistry {
    pub agents: HashMap<String, AnpAgentRegistration>,
}

impl Default for AnpDiscoveryRegistry {
    fn default() -> Self {
        Self::new()
    }
}

impl AnpDiscoveryRegistry {
    pub fn new() -> Self {
        Self {
            agents: HashMap::new(),
        }
    }

    /// Đăng ký hoặc cập nhật một tác tử vào sổ ANP.
    pub fn register_agent(&mut self, card: AgentCard) {
        let agent_id = card.agent_id.clone();
        self.agents.insert(
            agent_id,
            AnpAgentRegistration {
                card,
                in_flight_load: 0,
                is_healthy: true,
            },
        );
    }

    /// Cập nhật tải thực tế của tác tử.
    pub fn update_load(&mut self, agent_id: &str, in_flight: usize) {
        if let Some(reg) = self.agents.get_mut(agent_id) {
            reg.in_flight_load = in_flight;
        }
    }

    /// Định tuyến tới tác tử có năng lực phù hợp và đang có tải nhỏ nhất (Least-Loaded Routing).
    pub fn route_least_loaded(&self, required_capability: &str) -> Option<String> {
        self.agents
            .values()
            .filter(|reg| {
                reg.is_healthy
                    && reg
                        .card
                        .capabilities
                        .iter()
                        .any(|c| c == required_capability)
            })
            .min_by_key(|reg| (reg.in_flight_load, reg.card.cost_tier))
            .map(|reg| reg.card.agent_id.clone())
    }
}

// ============================================================================
// HÀM MAIN MINH HỌA END-TO-END HỆ SINH THÁI ĐỘ BỀN CHƯƠNG 89
// ============================================================================

fn main() {
    println!("=== CHƯƠNG 89: ĐỘ BỀN PHỤC HỒI SỰ CỐ & GIAO THỨC TÁC TỬ (MCP + A2A + ANP) ===");

    // 1. Minh họa TurnWal: Model-visible means logged
    let mut wal = TurnWal::new();
    let prompt = b"Hay phan tich do ben cua he thong Agent";
    wal.append("sess-001", WalPayloadKind::UserMessage, prompt);
    let is_logged = wal.verify_model_visible_logged("sess-001", prompt);
    println!(
        "1. Bất biến 'Model-visible means logged': {}",
        if is_logged {
            "ĐẠT (PASSED)"
        } else {
            "VI PHẠM"
        }
    );

    // 2. Minh họa SessionTree: rẽ nhánh an toàn
    let mut tree = SessionTree::new();
    let root = tree.create_root(SessionRole::System, "Ban la chuyen gia Rust");
    let branch_a = tree
        .fork_branch(root, SessionRole::User, "Viet code WAL")
        .unwrap();
    let branch_b = tree
        .fork_branch(root, SessionRole::User, "Viet code Saga")
        .unwrap();
    println!(
        "2. SessionTree: Đã rẽ nhánh độc lập A (ID={branch_a}) và B (ID={branch_b}) từ gốc (ID={root})"
    );

    // 3. Minh họa FencingToken: ngăn chặn zombie agent
    let mut fencing = FencingController::new();
    let token_1 = fencing.issue_next_token();
    let token_2 = fencing.issue_next_token();
    let zombie_check = fencing.validate_and_fence(&token_1);
    println!(
        "3. FencingToken: Tiến trình cũ với epoch {} ghi đè -> {}",
        token_1.epoch,
        zombie_check.is_err()
    );
    let valid_check = fencing.validate_and_fence(&token_2);
    println!(
        "   Tiến trình mới với epoch {} ghi đè -> {}",
        token_2.epoch,
        valid_check.is_ok()
    );

    // 4. Minh họa ANP Discovery
    let mut registry = AnpDiscoveryRegistry::new();
    registry.register_agent(AgentCard {
        agent_id: "agent-code-refactor".to_string(),
        name: "Chuyên gia Refactor".to_string(),
        capabilities: vec!["refactor".to_string(), "lint".to_string()],
        cost_tier: 1,
    });
    registry.register_agent(AgentCard {
        agent_id: "agent-code-architect".to_string(),
        name: "Kiến trúc sư Trưởng".to_string(),
        capabilities: vec!["refactor".to_string(), "design".to_string()],
        cost_tier: 3,
    });
    let best = registry.route_least_loaded("refactor").unwrap();
    println!("4. ANP Routing: Tác tử chi phí tối ưu nhất cho 'refactor' là: {best}");
    println!("=== HOÀN TẤT MINH HỌA HỆ THỐNG ĐỘ BỀN ===");
}

// ============================================================================
// BỘ KIỂM THỬ ĐƠN VỊ TOÀN DIỆN (>= 30 BÀI TEST CHỨNG MINH CÁC BẤT BIẾN CỐT LÕI)
// ============================================================================

#[cfg(test)]
mod tests {
    use super::*;

    // --- Mock Provider phục vụ kiểm thử LLM Gateway ---
    struct MockProvider {
        force_error: Option<ErrorClass>,
        success_content: String,
        cost_cents: u64,
    }

    impl LlmProviderMock for MockProvider {
        fn call(&self, model: &str, req: &LlmRequest) -> Result<LlmResponse, ErrorClass> {
            if let Some(err) = &self.force_error {
                Err(err.clone())
            } else {
                Ok(LlmResponse {
                    content: format!("{}: {}", model, self.success_content),
                    model_used: model.to_string(),
                    tokens_used: req.estimated_tokens,
                    cost_cents: self.cost_cents,
                })
            }
        }
    }

    // Test 1: Băm SHA-256 thuần std tính đúng hash rỗng chuẩn NIST
    #[test]
    fn test_sha256_empty_string_nist() {
        let digest = PureSha256::digest(b"");
        assert_eq!(
            digest.to_hex_string(),
            "e3b0c44298fc1c149afbf4c8996fb92427ae41e4649b934ca495991b7852b855"
        );
    }

    // Test 2: Băm SHA-256 chuỗi ký tự chuẩn NIST
    #[test]
    fn test_sha256_quick_brown_fox_nist() {
        let digest = PureSha256::digest(b"The quick brown fox jumps over the lazy dog");
        assert_eq!(
            digest.to_hex_string(),
            "d7a8fbb307d7809469ca9abcb0082e4f8d5651e46d3cdb762d02d0bf37c9e592"
        );
    }

    // Test 3: SplitMix64 phát sinh số tất định có tính lặp lại
    #[test]
    fn test_splitmix64_determinism() {
        let mut rng1 = SplitMix64::new(42);
        let mut rng2 = SplitMix64::new(42);
        for _ in 0..100 {
            assert_eq!(rng1.next_u64(), rng2.next_u64());
        }
    }

    // Test 4: DecorrelatedJitterBackoff không vượt quá trần max_delay_ms
    #[test]
    fn test_decorrelated_jitter_within_bounds() {
        let mut backoff = DecorrelatedJitterBackoff::new(50, 1000, 12345);
        let mut prev = 50;
        for _ in 0..50 {
            let next = backoff.next_delay(prev);
            assert!(next >= 50);
            assert!(next <= 1000);
            prev = next;
        }
    }

    // Test 5: Lỗi Fatal4xx dừng ngay lập tức và KHÔNG thử lại
    #[test]
    fn test_gateway_fatal_4xx_no_retry_no_circuit_breaker() {
        let provider = MockProvider {
            force_error: Some(ErrorClass::Fatal4xx {
                status_code: 400,
                message: "Bad Request: Malformed JSON".to_string(),
            }),
            success_content: "OK".to_string(),
            cost_cents: 10,
        };
        let mut gateway = LlmGateway::new(
            provider,
            "primary".to_string(),
            "secondary".to_string(),
            "local".to_string(),
            1000,
            100,
            42,
        );
        let req = LlmRequest {
            prompt: "Test".to_string(),
            estimated_tokens: 10,
            idempotency_key: "k1".to_string(),
            allow_fallback: true,
        };
        let res = gateway.execute(&req, 1);
        assert!(res.is_err());
        assert!(res.unwrap_err().contains("Lỗi client nghiêm trọng"));
        // Cầu dao vẫn phải ở trạng thái Closed, failure_count = 0
        match gateway.circuit_breaker.state {
            CircuitBreakerState::Closed {
                consecutive_failures,
            } => {
                assert_eq!(consecutive_failures, 0);
            }
            _ => panic!("Cầu dao không được phép mở khi gặp 4xx"),
        }
    }

    // Test 6: Lỗi Transient5xx mở CircuitBreaker sau khi vượt ngưỡng failure_threshold
    #[test]
    fn test_gateway_circuit_breaker_opens_on_consecutive_5xx() {
        let mut cb = CircuitBreaker::new(3, 50);
        let err503 = ErrorClass::Transient5xx {
            status_code: 503,
            message: "Service Unavailable".to_string(),
        };

        assert!(cb.can_execute(10));
        cb.record_failure(&err503, 10);
        cb.record_failure(&err503, 11);
        assert!(cb.can_execute(12));
        let opened = cb.record_failure(&err503, 12);
        assert!(opened);

        // Mạch đã mở: can_execute phải trả về false
        assert!(!cb.can_execute(20));
        // Sau thời gian cooldown (12 + 50 = 62): nhịp 65 chuyển sang HalfOpen
        assert!(cb.can_execute(65));
    }

    // Test 7: FinOps budget cạn kiệt chặn đứng các yêu cầu tiếp theo
    #[test]
    fn test_gateway_finops_budget_exhaustion() {
        let provider = MockProvider {
            force_error: None,
            success_content: "Trang thai tot".to_string(),
            cost_cents: 60,
        };
        let mut gateway = LlmGateway::new(
            provider,
            "p".to_string(),
            "s".to_string(),
            "l".to_string(),
            100, // Ngân sách chỉ 100 cents
            100,
            99,
        );
        let req = LlmRequest {
            prompt: "Lan 1".to_string(),
            estimated_tokens: 10,
            idempotency_key: "k1".to_string(),
            allow_fallback: false,
        };
        let r1 = gateway.execute(&req, 1);
        assert!(r1.is_ok());
        assert_eq!(gateway.spent_cents, 60);

        let req2 = LlmRequest {
            prompt: "Lan 2".to_string(),
            estimated_tokens: 10,
            idempotency_key: "k2".to_string(),
            allow_fallback: false,
        };
        let r2 = gateway.execute(&req2, 2);
        assert!(r2.is_ok());
        assert_eq!(gateway.spent_cents, 120);

        // Lần 3: ngân sách đã vượt 100 -> chặn đứng
        let r3 = gateway.execute(&req2, 3);
        assert!(r3.is_err());
        assert!(r3.unwrap_err().contains("Ngân sách FinOps"));
    }

    // Test 8: Bất biến "Model-visible means logged" bắt quả tang dữ liệu chèn lén
    #[test]
    fn test_wal_model_visible_means_logged_invariant() {
        let mut wal = TurnWal::new();
        let prompt_logged = b"Xin chao, toi la nguoi dung da xac thuc";
        wal.append("s1", WalPayloadKind::UserMessage, prompt_logged);

        assert!(wal.verify_model_visible_logged("s1", prompt_logged));

        // Chuỗi tiêm lén chưa từng được log vào WAL
        let rogue_prompt = b"DROP TABLE users; -- Lenh chen len khong ghi nhat ky";
        assert!(!wal.verify_model_visible_logged("s1", rogue_prompt));
    }

    // Test 9: Tuần tự hóa và giải tuần tự WAL toàn vẹn
    #[test]
    fn test_wal_serialization_roundtrip() {
        let mut wal = TurnWal::new();
        wal.append("sess_a", WalPayloadKind::SystemPrompt, b"System Init");
        wal.append("sess_a", WalPayloadKind::UserMessage, b"Cau hoi 1");

        let bytes = wal.serialize_to_bytes();
        let (recovered, count) = TurnWal::recover_from_bytes(&bytes);

        assert_eq!(count, 2);
        assert_eq!(recovered.frames.len(), 2);
        assert_eq!(recovered.frames[0].payload_bytes, b"System Init");
        assert_eq!(recovered.frames[1].payload_bytes, b"Cau hoi 1");
    }

    // Test 10: Phục hồi WAL tự cắt đuôi rách (torn-tail truncation) khi sập nguồn
    #[test]
    fn test_wal_torn_tail_truncation_on_crash() {
        let mut wal = TurnWal::new();
        wal.append("sess_b", WalPayloadKind::SystemPrompt, b"Khung hoan hao 1");
        wal.append("sess_b", WalPayloadKind::UserMessage, b"Khung hoan hao 2");

        let mut bytes = wal.serialize_to_bytes();
        // Giả lập sập nguồn: ghi dở dang thêm 15 byte rác vào đuôi tệp
        bytes.extend_from_slice(&[0x00, 0x01, 0x02, 0x03, 0x04, 0x05, 0x06, 0x07]);

        let (recovered, count) = TurnWal::recover_from_bytes(&bytes);
        assert_eq!(count, 2);
        assert_eq!(recovered.frames.len(), 2);
        assert_eq!(recovered.frames[1].payload_bytes, b"Khung hoan hao 2");
    }

    // Test 11: Phục hồi WAL bác bỏ khung bị biến chất dữ liệu (checksum mismatch)
    #[test]
    fn test_wal_corrupted_checksum_truncation() {
        let mut wal = TurnWal::new();
        wal.append("s", WalPayloadKind::SystemPrompt, b"Khung 1");
        wal.append("s", WalPayloadKind::UserMessage, b"Khung 2 bi sua trom");

        let mut bytes = wal.serialize_to_bytes();
        // Làm biến chất 1 byte bên trong khung 2
        let len = bytes.len();
        bytes[len - 40] ^= 0xff;

        let (recovered, count) = TurnWal::recover_from_bytes(&bytes);
        assert_eq!(count, 1);
        assert_eq!(recovered.frames.len(), 1);
        assert_eq!(recovered.frames[0].payload_bytes, b"Khung 1");
    }

    // Test 12: SessionTree tạo gốc và rẽ 2 nhánh độc lập không ghi đè dữ liệu cha
    #[test]
    fn test_session_tree_forking_independence() {
        let mut tree = SessionTree::new();
        let root = tree.create_root(SessionRole::System, "He thong khoi tao");

        let n1_a = tree
            .fork_branch(root, SessionRole::User, "Hoi Huong A")
            .unwrap();
        let n2_a = tree
            .fork_branch(n1_a, SessionRole::Assistant, "Tra loi Huong A")
            .unwrap();

        let n1_b = tree
            .fork_branch(root, SessionRole::User, "Hoi Huong B")
            .unwrap();
        let n2_b = tree
            .fork_branch(n1_b, SessionRole::Assistant, "Tra loi Huong B")
            .unwrap();

        let children_root = tree.list_children(root);
        assert_eq!(children_root.len(), 2);
        assert!(children_root.contains(&n1_a));
        assert!(children_root.contains(&n1_b));

        // Tuyến replay của nhánh A
        let path_a = tree.replay_path(n2_a).unwrap();
        assert_eq!(path_a.len(), 3);
        assert_eq!(path_a[0].content, "He thong khoi tao");
        assert_eq!(path_a[1].content, "Hoi Huong A");
        assert_eq!(path_a[2].content, "Tra loi Huong A");

        // Tuyến replay của nhánh B
        let path_b = tree.replay_path(n2_b).unwrap();
        assert_eq!(path_b.len(), 3);
        assert_eq!(path_b[0].content, "He thong khoi tao");
        assert_eq!(path_b[1].content, "Hoi Huong B");
        assert_eq!(path_b[2].content, "Tra loi Huong B");
    }

    // Test 13: SessionTree báo lỗi khi rẽ nhánh từ nút cha không tồn tại
    #[test]
    fn test_session_tree_invalid_parent() {
        let mut tree = SessionTree::new();
        let err = tree.fork_branch(9999, SessionRole::User, "Nhanh ma");
        assert!(err.is_err());
    }

    // Test 14: IdempotencyStore thực thi tác vụ mới ở pha Pending
    #[test]
    fn test_idempotency_store_new_execution() {
        let mut store = IdempotencyStore::new();
        let req_hash = PureSha256::digest(b"action_create_order");

        let act = store.begin_execution("order-101", req_hash, 10, 50);
        assert_eq!(act, IdempotencyAction::ExecuteNew);
    }

    // Test 15: IdempotencyStore chặn xung đột khi cùng một yêu cầu đang xử lý trong hạn lease
    #[test]
    fn test_idempotency_store_inflight_conflict() {
        let mut store = IdempotencyStore::new();
        let req_hash = PureSha256::digest(b"action_create_order");

        store.begin_execution("order-102", req_hash, 10, 50);
        let act2 = store.begin_execution("order-102", req_hash, 20, 50);
        assert_eq!(act2, IdempotencyAction::InFlightConflict);
    }

    // Test 16: IdempotencyStore cho phép thu hồi lease khi tiến trình trước bị sập quá hạn
    #[test]
    fn test_idempotency_store_reclaim_expired_lease() {
        let mut store = IdempotencyStore::new();
        let req_hash = PureSha256::digest(b"action_create_order");

        store.begin_execution("order-103", req_hash, 10, 50); // Hết hạn tại nhịp 60
        let act = store.begin_execution("order-103", req_hash, 70, 50); // Gọi tại nhịp 70
        assert_eq!(act, IdempotencyAction::ReclaimExpiredLease);
    }

    // Test 17: IdempotencyStore trả về kết quả trong cache khi đã Committed
    #[test]
    fn test_idempotency_store_cached_result_after_commit() {
        let mut store = IdempotencyStore::new();
        let req_hash = PureSha256::digest(b"charge_credit_card");

        store.begin_execution("tx-999", req_hash, 10, 50);
        store
            .commit_execution("tx-999", b"{\"status\": \"PAID\"}")
            .unwrap();

        let act = store.begin_execution("tx-999", req_hash, 20, 50);
        match act {
            IdempotencyAction::ReturnCached { response_payload } => {
                assert_eq!(response_payload, b"{\"status\": \"PAID\"}");
            }
            _ => panic!("Phải trả về ReturnCached"),
        }
    }

    // Test 18: TransactionalOutbox đưa sự kiện vào Staged và sweep đúng số lượng
    #[test]
    fn test_transactional_outbox_staging_and_sweep() {
        let mut outbox = TransactionalOutbox::new();
        outbox.stage_event("orders", b"order_created_1", 100);
        outbox.stage_event("orders", b"order_created_2", 101);

        let pending = outbox.sweep_pending(10);
        assert_eq!(pending.len(), 2);
        assert_eq!(pending[0].payload, b"order_created_1");
        assert_eq!(pending[1].payload, b"order_created_2");
    }

    // Test 19: TransactionalOutbox đánh dấu Dispatched thành công
    #[test]
    fn test_transactional_outbox_mark_dispatched() {
        let mut outbox = TransactionalOutbox::new();
        let id = outbox.stage_event("notifications", b"email_sent", 10);
        assert!(outbox.mark_dispatched(id, 15));

        let pending = outbox.sweep_pending(10);
        assert_eq!(pending.len(), 0);
    }

    // Test 20: Saga chạy xuôi thành công toàn bộ các bước
    #[test]
    fn test_orchestrated_saga_full_success() {
        let steps = vec![
            SagaStepDef {
                step_id: 1,
                name: "Đặt chỗ hàng tồn kho".to_string(),
                forward_action: "reserve_inventory".to_string(),
                compensation_action: "release_inventory".to_string(),
            },
            SagaStepDef {
                step_id: 2,
                name: "Trừ tiền ví".to_string(),
                forward_action: "debit_wallet".to_string(),
                compensation_action: "refund_wallet".to_string(),
            },
        ];
        let mut saga = OrchestratedSaga::new(steps);
        let mut trace = Vec::new();
        let res = saga.run(|action, is_comp| {
            trace.push((action.to_string(), is_comp));
            Ok(())
        });
        assert!(res.is_ok());
        assert_eq!(trace.len(), 2);
        assert_eq!(trace[0], ("reserve_inventory".to_string(), false));
        assert_eq!(trace[1], ("debit_wallet".to_string(), false));
    }

    // Test 21: Saga hoàn tác ngược đúng thứ tự 3 -> 2 -> 1 khi bước 4 gãy
    #[test]
    fn test_orchestrated_saga_backward_compensation_order() {
        let steps = vec![
            SagaStepDef {
                step_id: 1,
                name: "B1".to_string(),
                forward_action: "fwd_1".to_string(),
                compensation_action: "comp_1".to_string(),
            },
            SagaStepDef {
                step_id: 2,
                name: "B2".to_string(),
                forward_action: "fwd_2".to_string(),
                compensation_action: "comp_2".to_string(),
            },
            SagaStepDef {
                step_id: 3,
                name: "B3".to_string(),
                forward_action: "fwd_3".to_string(),
                compensation_action: "comp_3".to_string(),
            },
            SagaStepDef {
                step_id: 4,
                name: "B4".to_string(),
                forward_action: "fwd_4".to_string(),
                compensation_action: "comp_4".to_string(),
            },
        ];
        let mut saga = OrchestratedSaga::new(steps);
        let mut trace = Vec::new();

        let res = saga.run(|action, _is_comp| {
            trace.push(action.to_string());
            if action == "fwd_4" {
                Err("Hết hạn ngạch tài nguyên tại bước 4".to_string())
            } else {
                Ok(())
            }
        });

        assert!(res.is_err());
        // Thứ tự gọi phải là: fwd_1 -> fwd_2 -> fwd_3 -> fwd_4 (lỗi) -> comp_3 -> comp_2 -> comp_1
        assert_eq!(
            trace,
            vec![
                "fwd_1", "fwd_2", "fwd_3", "fwd_4", "comp_3", "comp_2", "comp_1"
            ]
        );
    }

    // Test 22: Saga tiếp tục bù trừ kể cả khi một bước hoàn tác đơn lẻ gặp lỗi
    #[test]
    fn test_orchestrated_saga_continues_compensation_on_error() {
        let steps = vec![
            SagaStepDef {
                step_id: 1,
                name: "B1".to_string(),
                forward_action: "f1".to_string(),
                compensation_action: "c1".to_string(),
            },
            SagaStepDef {
                step_id: 2,
                name: "B2".to_string(),
                forward_action: "f2".to_string(),
                compensation_action: "c2".to_string(),
            },
            SagaStepDef {
                step_id: 3,
                name: "B3".to_string(),
                forward_action: "f3".to_string(),
                compensation_action: "c3".to_string(),
            },
        ];
        let mut saga = OrchestratedSaga::new(steps);
        let mut trace = Vec::new();

        let res = saga.run(|action, _is_comp| {
            trace.push(action.to_string());
            if action == "f3" {
                Err("Lỗi f3".to_string())
            } else if action == "c2" {
                Err("Lỗi bù trừ c2".to_string())
            } else {
                Ok(())
            }
        });

        assert!(res.is_err());
        // c2 bị lỗi nhưng c1 VẪN PHẢI ĐƯỢC GỌI
        assert!(trace.contains(&"c2".to_string()));
        assert!(trace.contains(&"c1".to_string()));
        assert_eq!(trace.last().unwrap(), "c1");
    }

    // Test 23: OptimisticFileVersion CAS thành công khi khớp version và hash
    #[test]
    fn test_optimistic_file_cas_success() {
        let initial = b"fn main() {}";
        let mut file = OptimisticFileVersion::new("src/main.rs", initial);
        let initial_hash = file.content_hash;

        let new_content = b"fn main() { println!(\"Hello\"); }";
        let res = file.compare_and_swap(1, initial_hash, new_content);
        assert!(res.is_ok());
        assert_eq!(file.version, 2);
        assert_eq!(file.content_bytes, new_content);
    }

    // Test 24: OptimisticFileVersion CAS từ chối khi lệch version hoặc nội dung đã bị sửa
    #[test]
    fn test_optimistic_file_cas_conflict() {
        let initial = b"let x = 1;";
        let mut file = OptimisticFileVersion::new("code.rs", initial);
        let initial_hash = file.content_hash;

        // Thử CAS với version sai (2 thay vì 1)
        let res_ver = file.compare_and_swap(2, initial_hash, b"let x = 2;");
        assert!(res_ver.is_err());

        // Thử CAS với hash sai
        let dummy_hash = PureSha256::digest(b"different");
        let res_hash = file.compare_and_swap(1, dummy_hash, b"let x = 2;");
        assert!(res_hash.is_err());
    }

    // Test 25: FencingToken chặn đứng hoàn toàn Zombie Agent có epoch cũ
    #[test]
    fn test_fencing_token_rejects_zombie_agent() {
        let mut controller = FencingController::new();
        let token_1 = controller.issue_next_token();
        let token_2 = controller.issue_next_token();
        let token_3 = controller.issue_next_token();

        assert_eq!(token_1.epoch, 1);
        assert_eq!(token_2.epoch, 2);
        assert_eq!(token_3.epoch, 3);

        // controller đang ở epoch 3: token 1 và token 2 là zombie
        assert!(controller.validate_and_fence(&token_1).is_err());
        assert!(controller.validate_and_fence(&token_2).is_err());
        assert!(controller.validate_and_fence(&token_3).is_ok());
    }

    // Test 26: McpJsonRpcRouter xử lý method "ping" và "tools/list"
    #[test]
    fn test_mcp_router_ping_and_list() {
        let mut router = McpJsonRpcRouter::new();
        router.register_tool("bash", "Chay lenh Linux");
        router.register_tool("grep", "Tim kiem chuoi");

        let ping_req = McpRequest {
            jsonrpc: "2.0".to_string(),
            id: 1,
            method: "ping".to_string(),
            params: BTreeMap::new(),
        };
        let ping_resp = router.handle_request(&ping_req);
        assert_eq!(ping_resp.result.unwrap(), "pong");

        let list_req = McpRequest {
            jsonrpc: "2.0".to_string(),
            id: 2,
            method: "tools/list".to_string(),
            params: BTreeMap::new(),
        };
        let list_resp = router.handle_request(&list_req);
        let list_res = list_resp.result.unwrap();
        assert!(list_res.contains("bash: Chay lenh Linux"));
        assert!(list_res.contains("grep: Tim kiem chuoi"));
    }

    // Test 27: McpJsonRpcRouter thực thi tool hợp lệ và từ chối tool vô danh
    #[test]
    fn test_mcp_router_call_tool() {
        let mut router = McpJsonRpcRouter::new();
        router.register_tool("calculator", "Tinh toan bieu thuc");

        let mut params = BTreeMap::new();
        params.insert("name".to_string(), "calculator".to_string());
        params.insert("arg".to_string(), "1 + 1".to_string());

        let call_req = McpRequest {
            jsonrpc: "2.0".to_string(),
            id: 3,
            method: "tools/call".to_string(),
            params,
        };
        let resp = router.handle_request(&call_req);
        assert!(resp.error.is_none());
        assert!(
            resp.result
                .unwrap()
                .contains("Thực thi thành công calculator")
        );

        // Gọi tool không tồn tại
        let mut invalid_params = BTreeMap::new();
        invalid_params.insert("name".to_string(), "unknown_tool".to_string());
        let inv_req = McpRequest {
            jsonrpc: "2.0".to_string(),
            id: 4,
            method: "tools/call".to_string(),
            params: invalid_params,
        };
        let inv_resp = router.handle_request(&inv_req);
        assert!(inv_resp.result.is_none());
        assert!(inv_resp.error.unwrap().contains("không tồn tại"));
    }

    // Test 28: A2aAgentNode chuyển đổi trạng thái vòng đời tác vụ hợp lệ
    #[test]
    fn test_a2a_agent_node_lifecycle() {
        let card = AgentCard {
            agent_id: "agent-qa-1".to_string(),
            name: "QA Inspector".to_string(),
            capabilities: vec!["audit".to_string()],
            cost_tier: 1,
        };
        let mut node = A2aAgentNode::new(card);
        let task_id = node.submit_task("Kiem thu tinh toan ven");

        assert!(node.start_working(task_id).is_ok());
        assert!(node.complete_task(task_id, "Ket qua: 100% Passed").is_ok());

        let final_task = node.tasks.get(&task_id).unwrap();
        match final_task {
            A2aTaskState::Completed { result } => {
                assert_eq!(result, "Ket qua: 100% Passed");
            }
            _ => panic!("Tác vụ phải ở trạng thái Completed"),
        }
    }

    // Test 29: AnpDiscoveryRegistry định tuyến tới tác tử tải nhỏ nhất và chi phí thấp nhất
    #[test]
    fn test_anp_discovery_least_loaded_routing() {
        let mut reg = AnpDiscoveryRegistry::new();
        reg.register_agent(AgentCard {
            agent_id: "agent-a".to_string(),
            name: "Worker A".to_string(),
            capabilities: vec!["code-gen".to_string()],
            cost_tier: 2,
        });
        reg.register_agent(AgentCard {
            agent_id: "agent-b".to_string(),
            name: "Worker B".to_string(),
            capabilities: vec!["code-gen".to_string()],
            cost_tier: 1,
        });

        // Cả 2 đều tải 0 -> ưu tiên agent-b vì cost_tier 1 < 2
        let best1 = reg.route_least_loaded("code-gen").unwrap();
        assert_eq!(best1, "agent-b");

        // Tăng tải agent-b lên 5 -> phải định tuyến sang agent-a (tải 0)
        reg.update_load("agent-b", 5);
        let best2 = reg.route_least_loaded("code-gen").unwrap();
        assert_eq!(best2, "agent-a");
    }

    // Test 30: Kiểm thử khép kín toàn diện MCP + A2A + ANP (End-to-End Protocol Handshake)
    #[test]
    fn test_end_to_end_mcp_a2a_anp_handshake() {
        // 1. ANP Discovery
        let mut registry = AnpDiscoveryRegistry::new();
        let card = AgentCard {
            agent_id: "agent-math-solver".to_string(),
            name: "Math Agent".to_string(),
            capabilities: vec!["math".to_string()],
            cost_tier: 1,
        };
        registry.register_agent(card.clone());
        let routed_id = registry.route_least_loaded("math").expect("tìm thấy agent");
        assert_eq!(routed_id, "agent-math-solver");

        // 2. A2A Task Submission
        let mut node = A2aAgentNode::new(card);
        let task_id = node.submit_task("Tinh bieu thuc 12 * 12");
        node.start_working(task_id).unwrap();

        // 3. MCP Tool Call bên trong Agent
        let mut mcp = McpJsonRpcRouter::new();
        mcp.register_tool("multiply", "Nhan hai so");
        let mut params = BTreeMap::new();
        params.insert("name".to_string(), "multiply".to_string());
        params.insert("arg".to_string(), "12, 12".to_string());
        let mcp_res = mcp.handle_request(&McpRequest {
            jsonrpc: "2.0".to_string(),
            id: 100,
            method: "tools/call".to_string(),
            params,
        });
        assert!(mcp_res.error.is_none());

        // 4. A2A Complete
        node.complete_task(task_id, "144").unwrap();
        match node.tasks.get(&task_id).unwrap() {
            A2aTaskState::Completed { result } => assert_eq!(result, "144"),
            _ => panic!("Lỗi trạng thái A2A"),
        }
    }

    // Test 31: Kiểm tra ErrorClass Display format không rỗng
    #[test]
    fn test_error_class_display_format() {
        let e1 = ErrorClass::Fatal4xx {
            status_code: 404,
            message: "Not Found".to_string(),
        };
        let e2 = ErrorClass::RateLimit429 {
            retry_after_ms: 500,
        };
        let e3 = ErrorClass::Transient5xx {
            status_code: 502,
            message: "Bad Gateway".to_string(),
        };
        let e4 = ErrorClass::ContextOverflow {
            max_tokens: 4096,
            requested_tokens: 8192,
        };
        assert!(e1.to_string().contains("404"));
        assert!(e2.to_string().contains("500ms"));
        assert!(e3.to_string().contains("502"));
        assert!(e4.to_string().contains("8192"));
    }

    // Test 32: CircuitBreaker không chuyển sang Open khi nhận lỗi RateLimit429
    #[test]
    fn test_circuit_breaker_ignores_rate_limit() {
        let mut cb = CircuitBreaker::new(2, 100);
        let err429 = ErrorClass::RateLimit429 {
            retry_after_ms: 1000,
        };
        let changed = cb.record_failure(&err429, 10);
        assert!(!changed);
        assert!(cb.can_execute(11));
    }

    // Test 33: Bài tập 1 - CheckpointedWal phục hồi nhanh qua checkpoint
    #[derive(Debug, Clone, PartialEq, Eq)]
    pub struct WalCheckpoint {
        pub last_seq: u64,
        pub session_states: HashMap<String, Vec<u8>>,
    }

    pub struct CheckpointedWal {
        pub wal: TurnWal,
        pub latest_checkpoint: Option<WalCheckpoint>,
        pub checkpoint_interval: u64,
    }

    impl CheckpointedWal {
        pub fn new(interval: u64) -> Self {
            Self {
                wal: TurnWal::new(),
                latest_checkpoint: None,
                checkpoint_interval: interval,
            }
        }

        pub fn append_and_maybe_checkpoint(
            &mut self,
            session_id: &str,
            kind: WalPayloadKind,
            payload: &[u8],
        ) {
            self.wal.append(session_id, kind, payload);
            let current_seq = self.wal.next_seq - 1;

            if current_seq.is_multiple_of(self.checkpoint_interval) {
                let mut states = HashMap::new();
                states.insert(session_id.to_string(), payload.to_vec());
                self.latest_checkpoint = Some(WalCheckpoint {
                    last_seq: current_seq,
                    session_states: states,
                });
            }
        }

        pub fn fast_recovery_start_seq(&self) -> u64 {
            self.latest_checkpoint
                .as_ref()
                .map(|cp| cp.last_seq + 1)
                .unwrap_or(1)
        }
    }

    #[test]
    fn test_checkpointed_wal_fast_recovery() {
        let mut cwal = CheckpointedWal::new(5);
        for i in 1..=12 {
            cwal.append_and_maybe_checkpoint(
                "s1",
                WalPayloadKind::UserMessage,
                format!("Msg {i}").as_bytes(),
            );
        }
        assert_eq!(cwal.fast_recovery_start_seq(), 11);
    }

    // Test 34: Bài tập 2 - SlidingWindowCircuitBreaker tính tỷ lệ lỗi cửa sổ trượt
    use std::collections::VecDeque;

    #[derive(Debug, Clone)]
    pub struct SlidingWindowCircuitBreaker {
        pub window_size: usize,
        pub failure_rate_threshold: f64,
        pub samples: VecDeque<bool>,
        pub is_open: bool,
    }

    impl SlidingWindowCircuitBreaker {
        pub fn new(window_size: usize, threshold: f64) -> Self {
            Self {
                window_size,
                failure_rate_threshold: threshold,
                samples: VecDeque::with_capacity(window_size),
                is_open: false,
            }
        }

        pub fn record_result(&mut self, is_success: bool) {
            if self.samples.len() == self.window_size {
                self.samples.pop_front();
            }
            self.samples.push_back(is_success);

            if self.samples.len() >= self.window_size / 2 {
                let failures = self.samples.iter().filter(|&&ok| !ok).count();
                let rate = (failures as f64) / (self.samples.len() as f64);
                self.is_open = rate >= self.failure_rate_threshold;
            }
        }

        pub fn can_execute(&self) -> bool {
            !self.is_open
        }
    }

    #[test]
    fn test_sliding_window_circuit_breaker() {
        let mut sw = SlidingWindowCircuitBreaker::new(10, 0.4);
        for _ in 0..6 {
            sw.record_result(true);
        }
        assert!(sw.can_execute());

        for _ in 0..4 {
            sw.record_result(false);
        }
        assert!(!sw.can_execute());
    }

    // Test 35: Bài tập 3 - Hàng đợi ưu tiên chống đói tài nguyên
    use std::cmp::Ordering;
    use std::collections::BinaryHeap;

    #[derive(Debug, Clone, PartialEq, Eq)]
    pub struct PrioritizedTask {
        pub task_id: u64,
        pub base_priority: u32,
        pub submitted_tick: u64,
    }

    impl PrioritizedTask {
        pub fn effective_priority(&self, current_tick: u64) -> u64 {
            let age = current_tick.saturating_sub(self.submitted_tick);
            let bonus = age / 10;
            (self.base_priority as u64) + bonus
        }
    }

    struct HeapItem {
        task: PrioritizedTask,
        current_tick: u64,
    }

    impl PartialEq for HeapItem {
        fn eq(&self, other: &Self) -> bool {
            self.task.effective_priority(self.current_tick)
                == other.task.effective_priority(other.current_tick)
        }
    }
    impl Eq for HeapItem {}

    impl Ord for HeapItem {
        fn cmp(&self, other: &Self) -> Ordering {
            self.task
                .effective_priority(self.current_tick)
                .cmp(&other.task.effective_priority(other.current_tick))
        }
    }

    impl PartialOrd for HeapItem {
        fn partial_cmp(&self, other: &Self) -> Option<Ordering> {
            Some(self.cmp(other))
        }
    }

    #[test]
    fn test_aging_priority_queue_anti_starvation() {
        let t_normal = PrioritizedTask {
            task_id: 1,
            base_priority: 5,
            submitted_tick: 0,
        };
        let t_vip = PrioritizedTask {
            task_id: 2,
            base_priority: 10,
            submitted_tick: 60,
        };

        assert_eq!(t_normal.effective_priority(0), 5);
        assert_eq!(t_vip.effective_priority(70), 11);
        assert_eq!(t_normal.effective_priority(70), 12);
        assert!(t_normal.effective_priority(70) > t_vip.effective_priority(70));

        let mut heap = BinaryHeap::new();
        heap.push(HeapItem {
            task: t_normal,
            current_tick: 70,
        });
        heap.push(HeapItem {
            task: t_vip,
            current_tick: 70,
        });
        let top = heap.pop().unwrap();
        assert_eq!(top.task.task_id, 1);
    }
}
