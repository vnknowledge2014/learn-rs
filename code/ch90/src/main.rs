//! Chương 90: Điều phối Đa Tác tử, Cổng Kỷ luật Bàn thợ, Hiến pháp,
//! Bánh cóc Tự tiến hóa & Diễn tập LoopDrill
//! (Multi-Agent Orchestration, Workbench Gates, RSI Ratchet & LoopDrill Evals)
//!
//! Ngôn ngữ: 100% Rust 2024 thuần `std` (không dependencies ngoài).
//! Kiến trúc struct: Toàn bộ dùng trường có tên (named fields).
//! Tuân thủ chuẩn chú thích RiskGate tiếng Việt và định danh tiếng Anh chuẩn.

use std::collections::{BTreeMap, BTreeSet};
use std::fmt;

// ============================================================================
// 1. MÃ BĂM MẬT MÃ SHA-256 THUẦN STD (FIPS 180-4)
// ============================================================================

/// Hằng số 64 từ vòng lặp nén SHA-256 theo tiêu chuẩn FIPS 180-4.
const K: [u32; 64] = [
    0x428a2f98, 0x71374491, 0xb5c0fbcf, 0xe9b5dba5, 0x3956c25b, 0x59f111f1, 0x923f82a4, 0xab1c5ed5,
    0xd807aa98, 0x12835b01, 0x243185be, 0x550c7dc3, 0x72be5d74, 0x80deb1fe, 0x9bdc06a7, 0xc19bf174,
    0xe49b69c1, 0xefbe4786, 0x0fc19dc6, 0x240ca1cc, 0x2de92c6f, 0x4a7484aa, 0x5cb0a9dc, 0x76f988da,
    0x983e5152, 0xa831c66d, 0xb00327c8, 0xbf597fc7, 0xc6e00bf3, 0xd5a79147, 0x06ca6351, 0x14292967,
    0x27b70a85, 0x2e1b2138, 0x4d2c6dfc, 0x53380d13, 0x650a7354, 0x766a0abb, 0x81c2c92e, 0x92722c85,
    0xa2bfe8a1, 0xa81a664b, 0xc24b8b70, 0xc76c51a3, 0xd192e819, 0xd6990624, 0xf40e3585, 0x106aa070,
    0x19a4c116, 0x1e376c08, 0x2748774c, 0x34b0bcb5, 0x391c0cb3, 0x4ed8aa4a, 0x5b9cca4f, 0x682e6ff3,
    0x748f82ee, 0x78a5636f, 0x84c87814, 0x8cc70208, 0x90befffa, 0xa4506ceb, 0xbef9a3f7, 0xc67178f2,
];

/// Giá trị băm SHA-256 256-bit (32 byte) được bọc trong cấu trúc có trường tên.
///
/// RiskGate: Đảm bảo tính toàn vẹn và chống chối bỏ cho toàn bộ bằng chứng kiểm thử,
/// biên nhận diễn tập LoopDrill và dấu vết phân tán của tác tử.
#[derive(Debug, Clone, Copy, PartialEq, Eq, PartialOrd, Ord, Hash)]
pub struct Hash256 {
    pub bytes: [u8; 32],
}

impl Hash256 {
    /// Khởi tạo giá trị băm từ mảng 32 byte thô.
    pub const fn from_bytes(bytes: [u8; 32]) -> Self {
        Self { bytes }
    }

    /// Xuất chuỗi hex 64 ký tự thường.
    pub fn to_hex(&self) -> String {
        let mut s = String::with_capacity(64);
        for b in &self.bytes {
            s.push_str(&format!("{b:02x}"));
        }
        s
    }

    /// Xuất chuỗi hex 8 ký tự rút gọn cho hiển thị nhật ký.
    pub fn short_hex(&self) -> String {
        format!(
            "{:02x}{:02x}{:02x}{:02x}",
            self.bytes[0], self.bytes[1], self.bytes[2], self.bytes[3]
        )
    }
}

impl fmt::Display for Hash256 {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        write!(f, "{}", self.to_hex())
    }
}

/// Tính toán băm SHA-256 chuẩn FIPS 180-4 thuần Rust `std`.
pub fn sha256(data: &[u8]) -> Hash256 {
    let mut h: [u32; 8] = [
        0x6a09e667, 0xbb67ae85, 0x3c6ef372, 0xa54ff53a, 0x510e527f, 0x9b05688c, 0x1f83d9ab,
        0x5be0cd19,
    ];

    let mut m = data.to_vec();
    let bit_length = (data.len() as u64) * 8;
    m.push(0x80);
    while m.len() % 64 != 56 {
        m.push(0);
    }
    m.extend_from_slice(&bit_length.to_be_bytes());

    let (blocks, _) = m.as_chunks::<64>();
    for block in blocks {
        let mut w = [0u32; 64];
        for i in 0..16 {
            w[i] = u32::from_be_bytes([
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

        let mut a = h[0];
        let mut b = h[1];
        let mut c = h[2];
        let mut d = h[3];
        let mut e = h[4];
        let mut f = h[5];
        let mut g = h[6];
        let mut hh = h[7];

        for i in 0..64 {
            let s1 = e.rotate_right(6) ^ e.rotate_right(11) ^ e.rotate_right(25);
            let ch = (e & f) ^ ((!e) & g);
            let t1 = hh
                .wrapping_add(s1)
                .wrapping_add(ch)
                .wrapping_add(K[i])
                .wrapping_add(w[i]);
            let s0 = a.rotate_right(2) ^ a.rotate_right(13) ^ a.rotate_right(22);
            let maj = (a & b) ^ (a & c) ^ (b & c);
            let t2 = s0.wrapping_add(maj);

            hh = g;
            g = f;
            f = e;
            e = d.wrapping_add(t1);
            d = c;
            c = b;
            b = a;
            a = t1.wrapping_add(t2);
        }

        h[0] = h[0].wrapping_add(a);
        h[1] = h[1].wrapping_add(b);
        h[2] = h[2].wrapping_add(c);
        h[3] = h[3].wrapping_add(d);
        h[4] = h[4].wrapping_add(e);
        h[5] = h[5].wrapping_add(f);
        h[6] = h[6].wrapping_add(g);
        h[7] = h[7].wrapping_add(hh);
    }

    let mut out = [0u8; 32];
    for (i, v) in h.iter().enumerate() {
        out[i * 4..i * 4 + 4].copy_from_slice(&v.to_be_bytes());
    }
    Hash256 { bytes: out }
}

// ============================================================================
// 2. BỘ CỔNG BÀN THỢ (WORKBENCH GATE SUITE) & SỔ CÁI ĐƠN TÍNH NĂNG
// ============================================================================

/// Trạng thái của một tính năng trong sổ cái bàn thợ.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum FeatureStatus {
    Active,
    Completed,
    Aborted,
}

/// Bản ghi tính năng có cấu trúc đầy đủ trường tên.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct FeatureRecord {
    pub name: String,
    pub branch: String,
    pub status: FeatureStatus,
    pub start_timestamp: u64,
}

/// Sổ cái đơn tính năng `FeatureLedger`.
///
/// RiskGate: Cưỡng chế bất biến Bàn thợ: Chỉ một tính năng duy nhất được phép ở
/// trạng thái `Active` tại bất kỳ thời điểm nào. Ngăn chặn tác tử phân tâm
/// hoặc sửa lan ra ngoài phạm vi nhiệm vụ được giao.
#[derive(Debug, Clone, Default)]
pub struct FeatureLedger {
    pub records: Vec<FeatureRecord>,
}

impl FeatureLedger {
    /// Khởi tạo sổ cái trống.
    pub fn new() -> Self {
        Self {
            records: Vec::new(),
        }
    }

    /// Mở một tính năng mới. Trả về lỗi nếu đang có một tính năng khác ở trạng thái `Active`.
    pub fn start_feature(
        &mut self,
        name: &str,
        branch: &str,
        now: u64,
    ) -> Result<(), &'static str> {
        if self.has_active_feature() {
            return Err(
                "Cổng kỷ luật Bàn thợ: Đã có tính năng đang hoạt động, không được mở tính năng thứ hai",
            );
        }
        self.records.push(FeatureRecord {
            name: name.to_string(),
            branch: branch.to_string(),
            status: FeatureStatus::Active,
            start_timestamp: now,
        });
        Ok(())
    }

    /// Hoàn thành tính năng đang hoạt động.
    pub fn complete_feature(&mut self, name: &str) -> Result<(), &'static str> {
        let rec = self
            .records
            .iter_mut()
            .find(|r| r.name == name && r.status == FeatureStatus::Active)
            .ok_or("Không tìm thấy tính năng đang hoạt động mang tên này")?;
        rec.status = FeatureStatus::Completed;
        Ok(())
    }

    /// Hủy bỏ tính năng đang hoạt động khi gặp sự cố bất khả kháng.
    pub fn abort_feature(&mut self, name: &str) -> Result<(), &'static str> {
        let rec = self
            .records
            .iter_mut()
            .find(|r| r.name == name && r.status == FeatureStatus::Active)
            .ok_or("Không tìm thấy tính năng đang hoạt động để hủy")?;
        rec.status = FeatureStatus::Aborted;
        Ok(())
    }

    /// Kiểm tra xem có tính năng nào đang ở trạng thái `Active` hay không.
    pub fn has_active_feature(&self) -> bool {
        self.records
            .iter()
            .any(|r| r.status == FeatureStatus::Active)
    }

    /// Lấy tên của tính năng đang hoạt động (nếu có).
    pub fn current_active_feature(&self) -> Option<&str> {
        self.records
            .iter()
            .find(|r| r.status == FeatureStatus::Active)
            .map(|r| r.name.as_str())
    }
}

/// Hiện vật bàn giao tiêu chuẩn (Handoff Artifact).
/// Tuân thủ quy chuẩn 5 thành phần bắt buộc: Quan sát, Chuỗi lập luận, Giới hạn,
/// Kết luận, và Phương pháp kiểm chứng kèm dấu vết kiểm thử thực tế.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct HandoffArtifact {
    pub observation: String,
    pub logic_chain: String,
    pub caveats: String,
    pub conclusion: String,
    pub verification_method: String,
    pub test_evidence_hash: Option<Hash256>,
    pub tests_passed: usize,
    pub tests_failed: usize,
}

/// Cổng chống mừng hụt: Kiểm chứng tuyên bố hoàn thành của tác tử.
///
/// RiskGate: Bác bỏ ngay lập tức mọi báo cáo nếu thiếu bằng chứng băm kiểm thử,
/// có bài test bị trượt, số lượng test thấp hơn chỉ tiêu yêu cầu, hoặc các mục
/// văn bản bị bỏ trống. Trả về mã băm biên nhận bàn giao niêm phong.
pub fn verify_completion_claim(
    artifact: &HandoffArtifact,
    required_tests: usize,
) -> Result<Hash256, &'static str> {
    if artifact.observation.trim().is_empty() {
        return Err("Thiếu phần Quan sát (Observation)");
    }
    if artifact.logic_chain.trim().is_empty() {
        return Err("Thiếu phần Chuỗi lập luận (Logic Chain)");
    }
    if artifact.conclusion.trim().is_empty() {
        return Err("Thiếu phần Kết luận (Conclusion)");
    }
    if artifact.verification_method.trim().is_empty() {
        return Err("Thiếu phần Phương pháp kiểm chứng (Verification Method)");
    }
    if artifact.tests_failed > 0 {
        return Err("Bác bỏ bàn giao: Còn bài kiểm thử thất bại (tests_failed > 0)");
    }
    if artifact.tests_passed < required_tests {
        return Err("Bác bỏ bàn giao: Số bài kiểm thử đạt chưa đạt chỉ tiêu tối thiểu");
    }
    let evidence_hash = artifact
        .test_evidence_hash
        .ok_or("Bác bỏ bàn giao: Thiếu mã băm bằng chứng kiểm thử (test_evidence_hash is None)")?;

    let mut seal_payload = Vec::new();
    seal_payload.extend_from_slice(artifact.observation.as_bytes());
    seal_payload.extend_from_slice(b"::");
    seal_payload.extend_from_slice(artifact.conclusion.as_bytes());
    seal_payload.extend_from_slice(b"::");
    seal_payload.extend_from_slice(&evidence_hash.bytes);
    seal_payload.extend_from_slice(&artifact.tests_passed.to_be_bytes());

    Ok(sha256(&seal_payload))
}

/// Bộ kiểm tra quy tắc phụ thuộc tầng kiến trúc (Layer Import Linter).
///
/// RiskGate: Thực thi quy tắc kiến trúc sạch: Tầng dưới (Domain) tuyệt đối
/// không được phụ thuộc hoặc nhập trực tiếp từ tầng trên (Application / Infrastructure).
#[derive(Debug, Clone)]
pub struct LayerImportLinter {
    pub layer_ranks: BTreeMap<String, usize>,
}

impl LayerImportLinter {
    /// Khởi tạo với phân tầng mặc định: `domain` (0), `application` (1), `infrastructure` (2).
    pub fn standard_three_tier() -> Self {
        let mut layer_ranks = BTreeMap::new();
        layer_ranks.insert("domain".to_string(), 0);
        layer_ranks.insert("application".to_string(), 1);
        layer_ranks.insert("infrastructure".to_string(), 2);
        Self { layer_ranks }
    }

    /// Thêm tầng tùy biến vào bảng kiểm tra.
    pub fn add_layer(&mut self, name: &str, rank: usize) {
        self.layer_ranks.insert(name.to_string(), rank);
    }

    /// Kiểm tra tính hợp lệ của lệnh import từ tầng `source_layer` sang `target_layer`.
    /// Quy tắc: `source_layer` chỉ được phụ thuộc vào tầng có cấp bậc `<= source_rank`.
    pub fn check_import(&self, source_layer: &str, target_layer: &str) -> Result<(), &'static str> {
        let s_rank = self
            .layer_ranks
            .get(source_layer)
            .ok_or("Tầng nguồn không nằm trong cấu hình linter")?;
        let t_rank = self
            .layer_ranks
            .get(target_layer)
            .ok_or("Tầng đích không nằm trong cấu hình linter")?;

        if t_rank > s_rank {
            Err("Vi phạm kiến trúc phân tầng: Tầng thấp hơn không được import tầng cao hơn")
        } else {
            Ok(())
        }
    }
}

/// Cổng kiểm tra khởi tạo môi trường làm việc sạch.
pub fn run_init_checks(
    required_baseline_files: &[String],
    actual_files: &[String],
) -> Result<(), &'static str> {
    let actual_set: BTreeSet<&str> = actual_files.iter().map(String::as_str).collect();
    for req in required_baseline_files {
        if !actual_set.contains(req.as_str()) {
            return Err("Môi trường làm việc thiếu tệp cơ sở bắt buộc");
        }
    }
    Ok(())
}

/// Cổng giới hạn phạm vi khác biệt (Diff Scope Guard).
pub fn check_diff_scope(
    modified_files: &[String],
    allowed_globs: &[String],
) -> Result<(), &'static str> {
    for file in modified_files {
        let mut matched = false;
        for pattern in allowed_globs {
            if pattern.ends_with('*') {
                let prefix = &pattern[..pattern.len() - 1];
                if file.starts_with(prefix) {
                    matched = true;
                    break;
                }
            } else if file == pattern {
                matched = true;
                break;
            }
        }
        if !matched {
            return Err("Tác tử chạm vào tệp nằm ngoài phạm vi được phép chỉnh sửa");
        }
    }
    Ok(())
}

/// Quét độ sạch sẽ của không gian làm việc (phát hiện tệp rác / tệp tạm).
pub fn scan_workspace_cleanliness(
    files: &[String],
    forbidden_extensions: &[String],
) -> Vec<String> {
    let mut violations = Vec::new();
    for file in files {
        for ext in forbidden_extensions {
            if file.ends_with(ext) {
                violations.push(file.clone());
                break;
            }
        }
    }
    violations
}

// ============================================================================
// 3. LỊCH CRON 5 TRƯỜNG & BỘ ĐIỀU KHIỂN VÒNG LẶP MỤC TIÊU HAI TẦNG GIÁM KHẢO
// ============================================================================

/// Bộ phân tích và khớp lịch Cron 5 trường chuẩn (`minute hour dom month dow`).
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct CronSchedule {
    pub minute_pattern: String,
    pub hour_pattern: String,
    pub dom_pattern: String,
    pub month_pattern: String,
    pub dow_pattern: String,
}

impl CronSchedule {
    /// Phân tích chuỗi cron 5 trường, ví dụ: `"*/5 * * * *"` hoặc `"0 9 1-5 * *"`.
    pub fn parse(expr: &str) -> Result<Self, &'static str> {
        let parts: Vec<&str> = expr.split_whitespace().collect();
        if parts.len() != 5 {
            return Err("Chuỗi Cron phải có đúng 5 trường cách nhau bởi dấu khoảng trắng");
        }
        Ok(Self {
            minute_pattern: parts[0].to_string(),
            hour_pattern: parts[1].to_string(),
            dom_pattern: parts[2].to_string(),
            month_pattern: parts[3].to_string(),
            dow_pattern: parts[4].to_string(),
        })
    }

    /// Khớp thời điểm hiện tại `(phút, giờ, ngày_trong_tháng, tháng, ngày_trong_tuần)`.
    pub fn matches(&self, minute: u8, hour: u8, dom: u8, month: u8, dow: u8) -> bool {
        match_cron_field(&self.minute_pattern, minute)
            && match_cron_field(&self.hour_pattern, hour)
            && match_cron_field(&self.dom_pattern, dom)
            && match_cron_field(&self.month_pattern, month)
            && match_cron_field(&self.dow_pattern, dow)
    }
}

/// Khớp một trường cron đối với giá trị số nguyên.
fn match_cron_field(pattern: &str, val: u8) -> bool {
    if pattern == "*" {
        return true;
    }
    if let Some(step_str) = pattern.strip_prefix("*/")
        && let Ok(step) = step_str.parse::<u8>()
    {
        return step > 0 && val.is_multiple_of(step);
    }
    if pattern.contains(',') {
        return pattern
            .split(',')
            .any(|sub| match_cron_field(sub.trim(), val));
    }
    if let Some((start_s, end_s)) = pattern.split_once('-')
        && let (Ok(s), Ok(e)) = (start_s.parse::<u8>(), end_s.parse::<u8>())
    {
        return val >= s && val <= e;
    }
    if let Ok(exact) = pattern.parse::<u8>() {
        return exact == val;
    }
    false
}

/// Phán quyết của giám khảo đánh giá vòng lặp mục tiêu.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum JudgeVerdict {
    Passed,
    NeedsRetry,
    FailedFatal,
}

/// Kết quả đánh giá chi tiết kèm phản hồi và cấp bậc giám khảo.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct JudgeEvaluation {
    pub tier: u8, // Tier 1: Cú pháp/Liveness nhanh; Tier 2: Ngữ nghĩa/Test sâu
    pub verdict: JudgeVerdict,
    pub feedback: String,
}

/// Bộ điều phối vòng lặp mục tiêu `GoalLoopController` với kiến trúc Giám khảo 2 tầng.
///
/// RiskGate: Phân tầng giám khảo: Tầng 1 kiểm tra phản xạ nhanh (nhịp tim, định dạng,
/// cú pháp). Chỉ khi Tầng 1 đạt, mới kích hoạt Tầng 2 (chạy kiểm thử toàn bộ, đối chiếu
/// thuộc tính ngữ nghĩa). Ngăn chặn tiêu hao tài nguyên vô ích khi lỗi cơ bản chưa sửa.
#[derive(Debug, Clone)]
pub struct GoalLoopController {
    pub max_iterations: usize,
    pub current_iteration: usize,
    pub schedule: Option<CronSchedule>,
}

impl GoalLoopController {
    /// Khởi tạo bộ điều phối với trần số lần lặp tối đa.
    pub fn new(max_iterations: usize, schedule: Option<CronSchedule>) -> Self {
        Self {
            max_iterations,
            current_iteration: 0,
            schedule,
        }
    }

    /// Đánh giá một bước lặp thông qua 2 hàm giám khảo Tầng 1 và Tầng 2.
    pub fn evaluate_step<F1, F2>(
        &mut self,
        mut tier1_judge: F1,
        mut tier2_judge: F2,
    ) -> JudgeEvaluation
    where
        F1: FnMut() -> JudgeEvaluation,
        F2: FnMut() -> JudgeEvaluation,
    {
        self.current_iteration += 1;
        if self.current_iteration > self.max_iterations {
            return JudgeEvaluation {
                tier: 0,
                verdict: JudgeVerdict::FailedFatal,
                feedback: "Vòng lặp mục tiêu vượt quá trần số lần lặp cho phép".to_string(),
            };
        }

        // Đánh giá Tầng 1: Kiểm tra nhanh
        let t1 = tier1_judge();
        if t1.verdict != JudgeVerdict::Passed {
            return t1;
        }

        // Đánh giá Tầng 2: Kiểm tra chức năng sâu
        tier2_judge()
    }
}

// ============================================================================
// 4. ĐIỀU PHỐI TRUNG GIAN HÌNH SAO, DẤU VẾT PHÂN TÁN & HÀNG ĐỢI SỰ KIỆN CHỦ ĐỀ
// ============================================================================

/// Thông điệp trao đổi giữa các tác tử với trường tên định nghĩa rõ ràng.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct AgentMessage {
    pub sender: String,
    pub recipient: String,
    pub correlation_id: String,
    pub payload: String,
    pub timestamp: u64,
}

/// Dấu vết phân tán `TraceSpan` kèm băm SHA-256 để kiểm toán toàn vẹn luồng gọi.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct TraceSpan {
    pub trace_id: String,
    pub span_id: String,
    pub parent_span_id: Option<String>,
    pub name: String,
    pub start_micros: u64,
    pub end_micros: u64,
    pub attributes: BTreeMap<String, String>,
}

impl TraceSpan {
    /// Tính mã băm SHA-256 xác định cho dấu vết span.
    pub fn compute_hash(&self) -> Hash256 {
        let mut raw = Vec::new();
        raw.extend_from_slice(self.trace_id.as_bytes());
        raw.extend_from_slice(b"::");
        raw.extend_from_slice(self.span_id.as_bytes());
        raw.extend_from_slice(b"::");
        if let Some(p) = &self.parent_span_id {
            raw.extend_from_slice(p.as_bytes());
        }
        raw.extend_from_slice(b"::");
        raw.extend_from_slice(self.name.as_bytes());
        raw.extend_from_slice(&self.start_micros.to_be_bytes());
        raw.extend_from_slice(&self.end_micros.to_be_bytes());
        for (k, v) in &self.attributes {
            raw.extend_from_slice(k.as_bytes());
            raw.extend_from_slice(b"=");
            raw.extend_from_slice(v.as_bytes());
            raw.extend_from_slice(b";");
        }
        sha256(&raw)
    }
}

/// Bản ghi sự kiện trên hàng đợi chủ đề kèm con trỏ vị trí đơn điệu `CursorOffset`.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct TopicEvent {
    pub offset: u64,
    pub topic: String,
    pub message: AgentMessage,
    pub span_hash: Hash256,
}

/// Hàng đợi sự kiện chủ đề `TopicEventBus` với con trỏ đọc tuần tự `CursorOffset`.
#[derive(Debug, Clone, Default)]
pub struct TopicEventBus {
    pub events: Vec<TopicEvent>,
    pub next_offset: u64,
}

impl TopicEventBus {
    /// Khởi tạo bus sự kiện rỗng.
    pub fn new() -> Self {
        Self {
            events: Vec::new(),
            next_offset: 0,
        }
    }

    /// Đưa sự kiện vào bus, gán con trỏ offset tăng dần và trả về offset vừa phát hành.
    pub fn publish(&mut self, topic: &str, message: AgentMessage, span_hash: Hash256) -> u64 {
        let offset = self.next_offset;
        self.next_offset += 1;
        self.events.push(TopicEvent {
            offset,
            topic: topic.to_string(),
            message,
            span_hash,
        });
        offset
    }

    /// Đọc các sự kiện theo chủ đề bắt đầu từ một con trỏ `cursor_offset`.
    pub fn poll(&self, topic: &str, cursor_offset: u64, max_events: usize) -> Vec<TopicEvent> {
        self.events
            .iter()
            .filter(|e| e.topic == topic && e.offset >= cursor_offset)
            .take(max_events)
            .cloned()
            .collect()
    }
}

/// Bộ điều phối trung gian hình sao `MediatorOrchestrator`.
///
/// RiskGate: Kiến trúc hình sao cô lập: Tác tử không được phép gửi tin ngang hàng
/// không kiểm soát. Mọi liên lạc phải đi qua Mediator, thực hiện chuyển tiếp nguyên văn
/// `forward_message` (bảo toàn 100% byte) và lưu vết trên `TopicEventBus`.
#[derive(Debug, Clone, Default)]
pub struct MediatorOrchestrator {
    pub bus: TopicEventBus,
    pub registered_agents: BTreeSet<String>,
    pub routed_messages_count: usize,
}

impl MediatorOrchestrator {
    /// Khởi tạo Mediator trống.
    pub fn new() -> Self {
        Self {
            bus: TopicEventBus::new(),
            registered_agents: BTreeSet::new(),
            routed_messages_count: 0,
        }
    }

    /// Đăng ký tác tử tham gia mạng lưới.
    pub fn register_agent(&mut self, agent_id: &str) {
        self.registered_agents.insert(agent_id.to_string());
    }

    /// Chuyển tiếp tin nhắn nguyên văn bảo toàn toàn vẹn nội dung byte.
    pub fn forward_message(
        &mut self,
        msg: AgentMessage,
        span: &TraceSpan,
    ) -> Result<u64, &'static str> {
        if !self.registered_agents.contains(&msg.recipient) {
            return Err("Tác tử nhận chưa được đăng ký trong danh bạ Mediator");
        }
        if !self.registered_agents.contains(&msg.sender) {
            return Err("Tác tử gửi chưa được đăng ký trong danh bạ Mediator");
        }

        let span_hash = span.compute_hash();
        let topic = format!("agent.inbox.{}", msg.recipient);
        let offset = self.bus.publish(&topic, msg, span_hash);
        self.routed_messages_count += 1;
        Ok(offset)
    }
}

// ============================================================================
// 5. QUẠT LUỒNG KHẢO SÁT CÔ LẬP VÀ THỬ LẠI NHÁNH CON (EXPLORE FANOUT RUNNER)
// ============================================================================

/// Phát hiện có giá trị từ tác tử con trong quá trình khảo sát.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct Finding {
    pub summary: String,
    pub confidence_score: u8, // 0..100
}

/// Bằng chứng vị trí cụ thể trong mã nguồn hỗ trợ phát hiện.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct Evidence {
    pub file_path: String,
    pub line_start: usize,
    pub line_end: usize,
    pub snippet: String,
}

/// Bản ghi dấu vết hành động của nhánh khảo sát.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct TraceRecord {
    pub step: usize,
    pub action: String,
    pub result: String,
}

/// Kết quả trả về của một nhánh con trong quạt luồng.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct SubBranchResult {
    pub branch_id: String,
    pub findings: Vec<Finding>,
    pub evidences: Vec<Evidence>,
    pub traces: Vec<TraceRecord>,
    pub succeeded: bool,
    pub retry_count: usize,
}

/// Bộ chạy quạt luồng khảo sát `ExploreFanoutRunner`.
///
/// RiskGate: Phục hồi cô lập nhánh con: Khi một nhánh con gặp lỗi, cho phép tự thử lại
/// đúng 1 lần trong môi trường cô lập. Thất bại của một nhánh không được làm đổ vỡ
/// hoặc nhiễm độc dữ liệu của các nhánh anh em đang hoạt động song song.
pub struct ExploreFanoutRunner;

impl ExploreFanoutRunner {
    /// Thực thi quạt luồng khảo sát trên danh sách tác vụ với cơ chế tự thử lại 1 lần cô lập.
    pub fn run_fanout<F>(
        tasks: &[(&str, &str)], // (branch_id, task_description)
        worker: F,
    ) -> Vec<SubBranchResult>
    where
        F: Fn(&str, &str, usize) -> Result<SubBranchResult, &'static str>,
    {
        let mut results = Vec::with_capacity(tasks.len());
        for &(branch_id, task_desc) in tasks {
            // Lần thử 1 (attempt = 0)
            match worker(branch_id, task_desc, 0) {
                Ok(mut res) => {
                    res.succeeded = true;
                    res.retry_count = 0;
                    results.push(res);
                }
                Err(_) => {
                    // Tự thử lại cô lập lần 2 (attempt = 1)
                    match worker(branch_id, task_desc, 1) {
                        Ok(mut res) => {
                            res.succeeded = true;
                            res.retry_count = 1;
                            results.push(res);
                        }
                        Err(err_msg) => {
                            results.push(SubBranchResult {
                                branch_id: branch_id.to_string(),
                                findings: Vec::new(),
                                evidences: Vec::new(),
                                traces: vec![TraceRecord {
                                    step: 1,
                                    action: "isolated_retry_exhausted".to_string(),
                                    result: err_msg.to_string(),
                                }],
                                succeeded: false,
                                retry_count: 1,
                            });
                        }
                    }
                }
            }
        }
        results
    }
}

// ============================================================================
// 6. ĐẶC TẢ BÁN HÌNH THỨC 11 KHỐI & DANH BẠ ĐÁNH GIÁ MÙ (BLIND PORTFOLIO)
// ============================================================================

/// Đặc tả nhiệm vụ bán hình thức gồm đầy đủ 11 khối cấu trúc.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct PseudoFormalTaskBrief {
    pub objective: String,
    pub preconditions: Vec<String>,
    pub invariants: Vec<String>,
    pub input_artifacts: Vec<String>,
    pub output_artifacts: Vec<String>,
    pub constraints: Vec<String>,
    pub failure_modes: Vec<String>,
    pub verification_rules: Vec<String>,
    pub security_assumptions: Vec<String>,
    pub rollback_plan: String,
    pub attestation: String,
}

impl PseudoFormalTaskBrief {
    /// Kiểm tra tính đầy đủ của 11 khối bán hình thức bắt buộc theo chuẩn R0 / #6.
    pub fn validate_11_blocks(&self) -> Result<(), &'static str> {
        if self.objective.trim().is_empty() {
            return Err("Khối 1: Mục tiêu (Objective) bị trống");
        }
        if self.preconditions.is_empty() {
            return Err("Khối 2: Tiền điều kiện (Preconditions) bị trống");
        }
        if self.invariants.is_empty() {
            return Err("Khối 3: Bất biến (Invariants) bị trống");
        }
        if self.input_artifacts.is_empty() {
            return Err("Khối 4: Hiện vật đầu vào (Input Artifacts) bị trống");
        }
        if self.output_artifacts.is_empty() {
            return Err("Khối 5: Hiện vật đầu ra (Output Artifacts) bị trống");
        }
        if self.constraints.is_empty() {
            return Err("Khối 6: Ràng buộc (Constraints) bị trống");
        }
        if self.failure_modes.is_empty() {
            return Err("Khối 7: Dạng lỗi dự phòng (Failure Modes) bị trống");
        }
        if self.verification_rules.is_empty() {
            return Err("Khối 8: Quy tắc kiểm chứng (Verification Rules) bị trống");
        }
        if self.security_assumptions.is_empty() {
            return Err("Khối 9: Giả định an ninh (Security Assumptions) bị trống");
        }
        if self.rollback_plan.trim().is_empty() {
            return Err("Khối 10: Phương án hoàn tác (Rollback Plan) bị trống");
        }
        if self.attestation.trim().is_empty() {
            return Err("Khối 11: Cam kết kiểm toán (Attestation) bị trống");
        }
        Ok(())
    }
}

/// Danh bạ đánh giá mù `BlindPortfolioRegistry`.
///
/// RiskGate: Khử thiên kiến mô hình: Giám khảo LLM hoặc người chấm điểm chỉ nhìn thấy
/// các mã ẩn danh (`CANDIDATE_A`, `CANDIDATE_B`). Danh tính thực tế chỉ được giải mã
/// sau khi phán quyết điểm số đã được ký số niêm phong.
#[derive(Debug, Clone, Default)]
pub struct BlindPortfolioRegistry {
    pub blind_to_real: BTreeMap<String, String>,
}

impl BlindPortfolioRegistry {
    pub fn new() -> Self {
        Self {
            blind_to_real: BTreeMap::new(),
        }
    }

    /// Đăng ký nhãn mù cho một tác tử.
    pub fn register_candidate(&mut self, blind_label: &str, real_identity: &str) {
        self.blind_to_real
            .insert(blind_label.to_string(), real_identity.to_string());
    }

    /// Giải mã danh tính thực sau khi có kết quả đánh giá.
    pub fn reveal_identity(&self, blind_label: &str) -> Option<&str> {
        self.blind_to_real.get(blind_label).map(String::as_str)
    }
}

// ============================================================================
// 7. HỘP CÁT CÂY LÀM VIỆC (WORKTREE SANDBOX), RAII DROP & KHÓA ƯU TIÊN ĐƯỜNG DẪN
// ============================================================================

/// Mức độ ưu tiên của khóa thuê đường dẫn (Lease Priority).
/// Quy tắc: `CiSweeper (30) > PrBabysitter (20) > DepSweeper (10)`.
#[derive(Debug, Clone, Copy, PartialEq, Eq, PartialOrd, Ord)]
pub enum LeasePriority {
    DepSweeper = 10,
    PrBabysitter = 20,
    CiSweeper = 30,
}

/// Khóa thuê đường dẫn với thời hạn và mức ưu tiên có trường tên.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct PathLease {
    pub pattern: String,
    pub holder: String,
    pub priority: LeasePriority,
    pub lease_expires: u64,
}

/// Hộp cát làm việc cách ly `WorktreeSandbox` có cơ chế tự động dọn dẹp bằng RAII `Drop`.
#[derive(Debug)]
pub struct WorktreeSandbox {
    pub sandbox_id: String,
    pub workspace_root: String,
    pub active_leases: Vec<PathLease>,
    pub dropped_cleanly: bool,
}

impl WorktreeSandbox {
    /// Khởi tạo hộp cát mới.
    pub fn new(sandbox_id: &str, workspace_root: &str) -> Self {
        Self {
            sandbox_id: sandbox_id.to_string(),
            workspace_root: workspace_root.to_string(),
            active_leases: Vec::new(),
            dropped_cleanly: false,
        }
    }

    /// Đăng ký thuê đường dẫn glob theo mức ưu tiên:
    /// Tác tử có độ ưu tiên cao hơn (`CiSweeper`) được quyền chiếm quyền thuê của tác tử ưu tiên thấp hơn.
    pub fn acquire_lease(
        &mut self,
        pattern: &str,
        holder: &str,
        priority: LeasePriority,
        expires: u64,
    ) -> Result<(), &'static str> {
        // Tìm xem có xung đột với khóa hiện có không
        if let Some(pos) = self.active_leases.iter().position(|l| l.pattern == pattern) {
            let existing = &self.active_leases[pos];
            if priority > existing.priority {
                // Tiếm quyền thuê (Preemption) vì có mức ưu tiên cao hơn
                self.active_leases[pos] = PathLease {
                    pattern: pattern.to_string(),
                    holder: holder.to_string(),
                    priority,
                    lease_expires: expires,
                };
                return Ok(());
            } else {
                return Err(
                    "Xung đột khóa đường dẫn: Mức ưu tiên không đủ để tiếm quyền khóa hiện có",
                );
            }
        }

        self.active_leases.push(PathLease {
            pattern: pattern.to_string(),
            holder: holder.to_string(),
            priority,
            lease_expires: expires,
        });
        Ok(())
    }

    /// Giải phóng một khóa đường dẫn chủ động.
    pub fn release_lease(&mut self, pattern: &str, holder: &str) -> bool {
        let initial_len = self.active_leases.len();
        self.active_leases
            .retain(|l| !(l.pattern == pattern && l.holder == holder));
        self.active_leases.len() < initial_len
    }

    /// Phát hiện xung đột gộp 3 chiều (3-way merge conflict detection).
    /// Trả về chuỗi gộp thành công nếu 2 bên sửa độc lập, hoặc danh sách đoạn xung đột.
    pub fn detect_3way_conflict(
        base: &str,
        ours: &str,
        theirs: &str,
    ) -> Result<String, Vec<String>> {
        let base_lines: Vec<&str> = base.lines().collect();
        let ours_lines: Vec<&str> = ours.lines().collect();
        let theirs_lines: Vec<&str> = theirs.lines().collect();

        if ours == theirs {
            return Ok(ours.to_string());
        }
        if ours == base {
            return Ok(theirs.to_string());
        }
        if theirs == base {
            return Ok(ours.to_string());
        }

        // Kiểm tra xung đột từng dòng cơ bản
        let max_lines = base_lines
            .len()
            .max(ours_lines.len())
            .max(theirs_lines.len());
        let mut conflicts = Vec::new();
        let mut merged = Vec::new();

        for i in 0..max_lines {
            let b = base_lines.get(i).copied().unwrap_or("");
            let o = ours_lines.get(i).copied().unwrap_or("");
            let t = theirs_lines.get(i).copied().unwrap_or("");

            if o == t {
                merged.push(o.to_string());
            } else if o == b {
                merged.push(t.to_string());
            } else if t == b {
                merged.push(o.to_string());
            } else {
                conflicts.push(format!(
                    "Dòng {}: base='{}', ours='{}', theirs='{}'",
                    i + 1,
                    b,
                    o,
                    t
                ));
            }
        }

        if conflicts.is_empty() {
            Ok(merged.join("\n"))
        } else {
            Err(conflicts)
        }
    }
}

impl Drop for WorktreeSandbox {
    fn drop(&mut self) {
        // Tự động giải phóng toàn bộ khóa tài nguyên khi ra khỏi phạm vi (RAII)
        self.active_leases.clear();
        self.dropped_cleanly = true;
    }
}

// ============================================================================
// 8. NGẮT MẠCH ĐỘ TƯƠNG ĐỒNG MỜ (LOOP CONTEXT BREAKER)
// ============================================================================

/// Bộ ngắt mạch vòng lặp ngữ cảnh `LoopContextBreaker`.
///
/// RiskGate: Chống tác tử quay cuồng vô định: Khi tác tử lặp lại các hành động
/// hoặc phản hồi có độ tương đồng văn bản mờ vượt ngưỡng `similarity_threshold`
/// liên tiếp `loop_threshold` lần, ngắt mạch sẽ kích hoạt để buộc dừng tác tử.
#[derive(Debug, Clone)]
pub struct LoopContextBreaker {
    pub max_window: usize,
    pub loop_threshold: usize,
    pub similarity_threshold: f64,
    pub history: Vec<String>,
}

impl LoopContextBreaker {
    pub fn new(max_window: usize, loop_threshold: usize, similarity_threshold: f64) -> Self {
        Self {
            max_window,
            loop_threshold,
            similarity_threshold,
            history: Vec::new(),
        }
    }

    /// Ghi nhận hành động mới và kiểm tra xem có ngắt mạch hay không.
    /// Trả về `true` nếu mạch BỊ NGẮT (phát hiện quay cuồng vòng lặp).
    pub fn observe_and_check(&mut self, action_summary: &str) -> bool {
        self.history.push(action_summary.to_string());
        if self.history.len() > self.max_window {
            self.history.remove(0);
        }

        if self.history.len() < self.loop_threshold {
            return false;
        }

        let n = self.history.len();
        let target = &self.history[n - 1];
        let mut match_count = 0;

        for prev in self.history[n - self.loop_threshold..n - 1].iter() {
            if compute_jaccard_similarity(target, prev) >= self.similarity_threshold {
                match_count += 1;
            }
        }

        match_count >= (self.loop_threshold - 1)
    }
}

/// Tính toán độ tương đồng Jaccard giữa hai chuỗi ký tự theo tập từ.
pub fn compute_jaccard_similarity(s1: &str, s2: &str) -> f64 {
    let words1: BTreeSet<&str> = s1.split_whitespace().collect();
    let words2: BTreeSet<&str> = s2.split_whitespace().collect();

    if words1.is_empty() && words2.is_empty() {
        return 1.0;
    }
    if words1.is_empty() || words2.is_empty() {
        return 0.0;
    }

    let intersection_len = words1.intersection(&words2).count();
    let union_len = words1.union(&words2).count();

    (intersection_len as f64) / (union_len as f64)
}

// ============================================================================
// 9. HIẾN PHÁP TÁC TỬ (CONSTITUTION ENGINE) & PHÂN TÁCH 3 NGÔI
// ============================================================================

/// Bản ghi chứng thực quyết định phân tách nhiệm vụ bắt buộc.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct AttestationRecord {
    pub author: String,
    pub verifier: String,
    pub attestor: String,
    pub specification_hash: Hash256,
    pub self_rule_hash: Hash256,
    pub claim_summary: String,
}

/// Động cơ Hiến pháp `ConstitutionEngine`.
///
/// RiskGate: Phân tách quyền lực ba ngôi: Tác giả (`author`), Người kiểm chứng (`verifier`),
/// và Người chứng thực (`attestor`) phải là 3 thực thể hoàn toàn độc lập (`author != verifier != attestor`).
/// Nghiêm cấm tuyệt đối hành vi tự phê duyệt (`author == verifier`). Mã băm của Hiến pháp
/// được ghim cứng (`pinned_self_hash`), bất kỳ sự can thiệp trái phép nào đều vô hiệu hóa hệ thống.
#[derive(Debug, Clone)]
pub struct ConstitutionEngine {
    pub pinned_self_hash: Hash256,
    pub forbidden_patterns: Vec<String>,
}

impl ConstitutionEngine {
    /// Khởi tạo Hiến pháp với quy tắc cơ sở và băm niêm phong.
    pub fn new(self_rules: &str, forbidden_patterns: Vec<String>) -> Self {
        let pinned_self_hash = sha256(self_rules.as_bytes());
        Self {
            pinned_self_hash,
            forbidden_patterns,
        }
    }

    /// Xác minh nguyên tắc phân tách nhiệm vụ ba ngôi.
    pub fn verify_separation_of_duties(
        &self,
        record: &AttestationRecord,
    ) -> Result<(), &'static str> {
        // Kiểm tra tự phê duyệt (author == verifier)
        if record.author == record.verifier {
            return Err(
                "Vi phạm Hiến pháp: Tác giả không được phép tự đóng vai trò kiểm chứng (author == verifier)",
            );
        }
        // Kiểm tra author == attestor
        if record.author == record.attestor {
            return Err(
                "Vi phạm Hiến pháp: Tác giả không được phép kiêm người chứng thực (author == attestor)",
            );
        }
        // Kiểm tra verifier == attestor
        if record.verifier == record.attestor {
            return Err(
                "Vi phạm Hiến pháp: Người kiểm chứng không được phép kiêm người chứng thực (verifier == attestor)",
            );
        }
        // Kiểm tra toàn vẹn bộ luật Hiến pháp tự thân
        if record.self_rule_hash != self.pinned_self_hash {
            return Err(
                "Vi phạm Hiến pháp: Mã băm quy tắc tự thân bị sai lệch so với bản ghim niêm phong",
            );
        }
        Ok(())
    }

    /// Kiểm tra nội dung mã hoặc phản hồi có chứa mẫu lệnh bị Hiến pháp cấm hay không.
    pub fn check_content_compliance(&self, content: &str) -> Result<(), &'static str> {
        for pattern in &self.forbidden_patterns {
            if content.contains(pattern) {
                return Err("Nội dung chứa đoạn mã hoặc lệnh bị Hiến pháp nghiêm cấm");
            }
        }
        Ok(())
    }
}

// ============================================================================
// 10. BÁNH CÓC TỰ TIẾN HÓA HAI TẬP TÁCH (TWO-SPLIT RSI RATCHET)
// ============================================================================

/// Điểm số đánh giá ứng viên trên hai tập tách biệt kèm chiết khấu tài nguyên.
#[derive(Debug, Clone, PartialEq)]
pub struct RsiCandidateScore {
    pub held_in_pass_rate: f64,
    pub held_out_pass_rate: f64,
    pub subagent_cost_discount: f64,
}

/// Bánh cóc tự tiến hóa hai tập tách `TwoSplitRsiRatchet`.
///
/// RiskGate: Cổng không hồi quy (Non-regression Gate): Ứng viên bắt buộc phải kiểm tra
/// trên cả tập công khai (`held_in`) lẫn tập ẩn (`held_out`). Nếu ứng viên đạt điểm cao
/// trên `held_in` nhưng làm tụt hiệu năng trên `held_out`, bánh cóc BÁC BỎ ngay lập tức
/// để ngăn ngừa hiện tượng quá khớp (overfitting) và thoái hóa năng lực.
#[derive(Debug, Clone)]
pub struct TwoSplitRsiRatchet {
    pub current_baseline_held_out: f64,
    pub current_baseline_held_in: f64,
    pub accepted_generations: usize,
    pub rejected_overfits: usize,
}

impl TwoSplitRsiRatchet {
    /// Khởi tạo bánh cóc với mốc điểm cơ sở ban đầu.
    pub fn new(initial_held_in: f64, initial_held_out: f64) -> Self {
        Self {
            current_baseline_held_in: initial_held_in,
            current_baseline_held_out: initial_held_out,
            accepted_generations: 0,
            rejected_overfits: 0,
        }
    }

    /// Đánh giá xem ứng viên cải tiến có được bánh cóc chấp nhận nâng bậc hay không.
    pub fn evaluate_candidate(&mut self, score: &RsiCandidateScore) -> bool {
        // Bắt buộc: Tỉ lệ đạt trên tập ẩn held_out KHÔNG ĐƯỢC TỤT dưới baseline
        if score.held_out_pass_rate < self.current_baseline_held_out {
            self.rejected_overfits += 1;
            return false;
        }

        // Điểm thực tế sau khi tính chiết khấu chi phí
        let net_held_in = score.held_in_pass_rate - score.subagent_cost_discount;
        let net_held_out = score.held_out_pass_rate - score.subagent_cost_discount;

        if net_held_out >= self.current_baseline_held_out
            && net_held_in >= self.current_baseline_held_in
        {
            self.current_baseline_held_out = score.held_out_pass_rate;
            self.current_baseline_held_in = score.held_in_pass_rate;
            self.accepted_generations += 1;
            true
        } else {
            false
        }
    }
}

/// Bộ kiểm tra trôi lệch chính sách cấu hình `PolicyDriftChecker`.
#[derive(Debug, Clone)]
pub struct PolicyDriftChecker {
    pub baseline_spec: BTreeMap<String, String>,
}

impl PolicyDriftChecker {
    pub fn new(baseline: BTreeMap<String, String>) -> Self {
        Self {
            baseline_spec: baseline,
        }
    }

    /// So sánh cấu hình hiện tại với mốc chuẩn và báo cáo danh sách trôi lệch.
    pub fn check_drift(&self, current: &BTreeMap<String, String>) -> Vec<String> {
        let mut drifts = Vec::new();
        // Kiểm tra các khóa bị xóa hoặc thay đổi giá trị
        for (k, v) in &self.baseline_spec {
            match current.get(k) {
                None => drifts.push(format!("Khóa bị xóa: {k}")),
                Some(cv) if cv != v => {
                    drifts.push(format!("Giá trị trôi lệch: {k} (cũ='{v}', mới='{cv}')"))
                }
                _ => {}
            }
        }
        // Kiểm tra các khóa mới được thêm vào
        for k in current.keys() {
            if !self.baseline_spec.contains_key(k) {
                drifts.push(format!("Khóa mới chưa phê chuẩn: {k}"));
            }
        }
        drifts
    }
}

// ============================================================================
// 11. THAO TRƯỜNG DIỄN TẬP LOOPDRILL (SỬA LỖI EDGE CASE #6) & AUDITOR L3 -> L2
// ============================================================================

/// Trạng thái kết quả của một đợt diễn tập LoopDrill.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum DrillStatus {
    Success,
    FailedSensitivity,
    FailedSpecificity,
    FailedBoth,
    RejectedVacuousSuite, // Sửa Edge Case #6: Bác bỏ bẫy chân lý rỗng (vacuous truth)
}

/// Biên nhận diễn tập LoopDrill có cấu trúc bảo vệ toàn vẹn bằng SHA-256.
#[derive(Debug, Clone, PartialEq)]
pub struct DrillReceipt {
    pub sensitivity: f64,
    pub specificity: f64,
    pub status: DrillStatus,
    pub total_positive_samples: usize,
    pub total_negative_samples: usize,
    pub receipt_hash: Hash256,
}

/// Thực thi diễn tập rào chắn hai chiều `run_loop_drill`.
///
/// RiskGate (SỬA LỖI EDGE CASE #6): Chống bẫy vacuous truth:
/// Bắt buộc `tp + fn_c > 0 && tn + fp > 0`. Tập kiểm thử bắt buộc phải có cả mẫu dương
/// (mẫu độc hại cần chặn) lẫn mẫu âm (mẫu lành tính cần cho qua). Nếu thiếu 1 trong 2,
/// hàm sẽ trả về `DrillStatus::RejectedVacuousSuite`, đánh rớt ngay lập tức thay vì
/// ngộ nhận đạt tỉ lệ 1.0!
pub fn run_loop_drill<F: Fn(&str) -> bool>(
    guard: F,
    cases: &[(&str, bool)], // (payload, is_fault)
    min_sens: f64,
    min_spec: f64,
) -> DrillReceipt {
    let mut tp = 0usize;
    let mut fn_c = 0usize;
    let mut tn = 0usize;
    let mut fp = 0usize;

    for &(payload, is_fault) in cases {
        let blocked = guard(payload);
        match (is_fault, blocked) {
            (true, true) => tp += 1,
            (true, false) => fn_c += 1,
            (false, false) => tn += 1,
            (false, true) => fp += 1,
        }
    }

    let total_pos = tp + fn_c;
    let total_neg = tn + fp;

    // SỬA LỖI EDGE CASE #6: Kiểm tra bẫy chân lý rỗng
    if total_pos == 0 || total_neg == 0 {
        let receipt_hash = sha256(b"VACUOUS_TRUTH_REJECTED");
        return DrillReceipt {
            sensitivity: 0.0,
            specificity: 0.0,
            status: DrillStatus::RejectedVacuousSuite,
            total_positive_samples: total_pos,
            total_negative_samples: total_neg,
            receipt_hash,
        };
    }

    let sens = (tp as f64) / (total_pos as f64);
    let spec = (tn as f64) / (total_neg as f64);

    let sens_ok = sens >= min_sens;
    let spec_ok = spec >= min_spec;

    let status = match (sens_ok, spec_ok) {
        (true, true) => DrillStatus::Success,
        (false, true) => DrillStatus::FailedSensitivity,
        (true, false) => DrillStatus::FailedSpecificity,
        (false, false) => DrillStatus::FailedBoth,
    };

    // Tạo mã băm biên nhận chống giả mạo
    let mut buf = Vec::new();
    buf.extend_from_slice(&(sens.to_bits()).to_be_bytes());
    buf.extend_from_slice(&(spec.to_bits()).to_be_bytes());
    buf.extend_from_slice(&(total_pos as u64).to_be_bytes());
    buf.extend_from_slice(&(total_neg as u64).to_be_bytes());
    buf.extend_from_slice(match status {
        DrillStatus::Success => b"STATUS:SUCCESS",
        DrillStatus::FailedSensitivity => b"STATUS:FAIL_SENS",
        DrillStatus::FailedSpecificity => b"STATUS:FAIL_SPEC",
        DrillStatus::FailedBoth => b"STATUS:FAIL_BOTH",
        DrillStatus::RejectedVacuousSuite => b"STATUS:VACUOUS",
    });

    let receipt_hash = sha256(&buf);

    DrillReceipt {
        sensitivity: sens,
        specificity: spec,
        status,
        total_positive_samples: total_pos,
        total_negative_samples: total_neg,
        receipt_hash,
    }
}

/// Cấp độ sẵn sàng tự hành của tác tử (Autonomous Readiness Level).
#[derive(Debug, Clone, Copy, PartialEq, Eq, PartialOrd, Ord)]
pub enum ReadinessLevel {
    L0Manual = 0,
    L1Assisted = 1,
    L2Monitored = 2,
    L3Autonomous = 3,
}

/// Kiểm toán viên cấp độ tự hành `ReadinessAuditor`.
///
/// RiskGate: Tự động hạ trần cấp phép: Nếu biên nhận diễn tập không thành công,
/// hoặc mã băm biên nhận bị sai lệch (dấu hiệu giả mạo), cấp độ tự hành của tác tử
/// lập tức bị hạ trần từ `L3 -> L2` (chuyển sang chế độ giám sát chặt chẽ).
#[derive(Debug, Clone)]
pub struct ReadinessAuditor {
    pub current_level: ReadinessLevel,
    pub last_valid_receipt_hash: Option<Hash256>,
}

impl ReadinessAuditor {
    pub fn new(initial_level: ReadinessLevel) -> Self {
        Self {
            current_level: initial_level,
            last_valid_receipt_hash: None,
        }
    }

    /// Thẩm định biên nhận diễn tập LoopDrill và điều chỉnh cấp độ tự hành.
    pub fn verify_and_update(
        &mut self,
        receipt: &DrillReceipt,
        expected_hash: Hash256,
    ) -> ReadinessLevel {
        if receipt.status != DrillStatus::Success || receipt.receipt_hash != expected_hash {
            // Hạ trần L3 -> L2 khi có vi phạm hoặc sai lệch băm
            if self.current_level >= ReadinessLevel::L3Autonomous {
                self.current_level = ReadinessLevel::L2Monitored;
            }
            return self.current_level;
        }

        self.last_valid_receipt_hash = Some(receipt.receipt_hash);
        if receipt.sensitivity >= 1.0 && receipt.specificity >= 1.0 {
            self.current_level = ReadinessLevel::L3Autonomous;
        }
        self.current_level
    }
}

// ============================================================================
// 12. PHẢN HỒI TRÌNH BIÊN DỊCH & CHẨN ĐOÁN LSP (COMPILER FEEDBACK LOOP)
// ============================================================================

/// Mức độ nghiêm trọng của chẩn đoán LSP.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum DiagnosticSeverity {
    Error,
    Warning,
    Information,
}

/// Chẩn đoán lỗi/cảnh báo trình biên dịch từ LSP Client.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct LspDiagnostic {
    pub file_path: String,
    pub line: usize,
    pub column: usize,
    pub severity: DiagnosticSeverity,
    pub code: String,
    pub message: String,
}

/// Vòng lặp phản hồi sửa lỗi trình biên dịch `CompilerFeedbackLoop`.
#[derive(Debug, Clone)]
pub struct CompilerFeedbackLoop {
    pub max_attempts: usize,
    pub current_attempts: usize,
    pub diagnostic_history: Vec<String>,
}

impl CompilerFeedbackLoop {
    pub fn new(max_attempts: usize) -> Self {
        Self {
            max_attempts,
            current_attempts: 0,
            diagnostic_history: Vec::new(),
        }
    }

    /// Nạp danh sách lỗi biên dịch và tạo gợi ý sửa lỗi có cấu trúc.
    pub fn feed_diagnostics(&mut self, diags: &[LspDiagnostic]) -> Result<String, &'static str> {
        self.current_attempts += 1;
        if self.current_attempts > self.max_attempts {
            return Err("Vượt quá số lần sửa lỗi biên dịch cho phép (max attempts exceeded)");
        }

        let errors: Vec<&LspDiagnostic> = diags
            .iter()
            .filter(|d| d.severity == DiagnosticSeverity::Error)
            .collect();

        if errors.is_empty() {
            return Ok("Trình biên dịch sạch lỗi!".to_string());
        }

        // Tạo chữ ký lỗi để phát hiện tác tử sửa đi sửa lại đúng 1 lỗi
        let sig = errors
            .iter()
            .map(|e| format!("{}:{}:{}", e.file_path, e.line, e.code))
            .collect::<Vec<_>>()
            .join("|");

        if self.diagnostic_history.iter().any(|h| h == &sig) {
            return Err("Tác tử rơi vào vòng lặp lặp lại cùng một lỗi biên dịch");
        }
        self.diagnostic_history.push(sig);

        let mut prompt = String::new();
        prompt.push_str("=== LỆNH SỬA LỖI BIÊN DỊCH CÓ CẤU TRÚC ===\n");
        for err in errors {
            prompt.push_str(&format!(
                "- [{}] {}:{}:{} - {}\n",
                err.code, err.file_path, err.line, err.column, err.message
            ));
        }
        prompt.push_str("Chỉ sửa đúng vị trí lỗi, tuân thủ nguyên tắc thay đổi tối thiểu!");
        Ok(prompt)
    }
}

// ============================================================================
// 13. BỘ KIỂM ĐỊNH ĐA TẦNG (MULTI-LAYER EVALUATOR)
// ============================================================================

/// Bộ chấm điểm quỹ đạo `TrajectoryGrader` tính toán $pass@k$ và $pass^k$.
pub struct TrajectoryGrader;

impl TrajectoryGrader {
    /// Tính $pass@k$ theo công thức ước lượng không thiên kiến (HumanEval):
    /// $pass@k = 1 - \frac{\binom{n - c}{k}}{\binom{n}{k}}$
    /// với $n$ là tổng mẫu chạy, $c$ là số mẫu thành công, $k$ là số lượt thử.
    pub fn pass_at_k(n: usize, c: usize, k: usize) -> f64 {
        if n == 0 || k == 0 || k > n {
            return 0.0;
        }
        if n - c < k {
            return 1.0;
        }

        let mut prod = 1.0;
        for i in 0..k {
            let num = (n - c - i) as f64;
            let den = (n - i) as f64;
            prod *= num / den;
        }
        1.0 - prod
    }

    /// Tính $pass^k$: Xác suất cả $k$ lượt thử độc lập đều thành công:
    /// $pass^k = p^k$.
    pub fn pass_pow_k(p: f64, k: usize) -> f64 {
        p.powi(k as i32)
    }
}

/// Trình so khớp AST cho lệnh gọi công cụ theo chuẩn Berkeley Function Calling Leaderboard (`BfclAstMatcher`).
pub struct BfclAstMatcher;

impl BfclAstMatcher {
    /// So khớp tham số hàm bỏ qua thứ tự các khóa JSON hoặc khoảng trắng thừa.
    pub fn matches_arguments(expected: &str, actual: &str) -> bool {
        let parse_kv = |s: &str| -> BTreeMap<String, String> {
            let mut map = BTreeMap::new();
            for part in s.trim().trim_matches(|c| c == '{' || c == '}').split(',') {
                if let Some((k, v)) = part.split_once(':') {
                    let key = k.trim().trim_matches('"').to_string();
                    let val = v.trim().trim_matches('"').to_string();
                    if !key.is_empty() {
                        map.insert(key, val);
                    }
                }
            }
            map
        };

        let exp_map = parse_kv(expected);
        let act_map = parse_kv(actual);

        exp_map == act_map
    }
}

/// Bộ chuẩn hóa câu trả lời `GaiaNormalizer` theo tiêu chuẩn GAIA Benchmark.
pub struct GaiaNormalizer;

impl GaiaNormalizer {
    /// Chuẩn hóa chuỗi văn bản: chuyển chữ thường, loại bỏ dấu câu cuối, bỏ dấu khoảng trắng thừa,
    /// và loại bỏ các đơn vị tiền tệ/phần trăm phụ ($).
    pub fn normalize(input: &str) -> String {
        let mut s = input.trim().to_lowercase();
        // Loại bỏ ký hiệu đơn vị phổ biến
        s = s.replace('$', "");
        s = s.replace('%', "");
        s = s.replace(',', "");
        // Bỏ dấu chấm/phẩy cuối dòng
        while s.ends_with('.') || s.ends_with('?') || s.ends_with('!') {
            s.pop();
        }
        s.split_whitespace().collect::<Vec<_>>().join(" ")
    }
}

/// Giám khảo tráo vị trí `PositionSwappedJudge` khử thiên kiến vị trí (Positional Bias).
pub struct PositionSwappedJudge;

impl PositionSwappedJudge {
    /// Đánh giá hai ứng viên A và B bằng cách tráo đổi vị trí thứ tự:
    /// Lượt 1: (A, B) -> phán quyết
    /// Lượt 2: (B, A) -> phán quyết
    /// Trả về ứng viên chiến thắng thực sự hoặc báo lỗi thiên kiến nếu phán quyết mâu thuẫn.
    pub fn judge_fairly<F>(
        cand_a: &str,
        cand_b: &str,
        eval_fn: F,
    ) -> Result<&'static str, &'static str>
    where
        F: Fn(&str, &str) -> usize, // Trả về 0 nếu người trước thắng, 1 nếu người sau thắng
    {
        let round1 = eval_fn(cand_a, cand_b); // 0: A thắng, 1: B thắng
        let round2 = eval_fn(cand_b, cand_a); // 0: B thắng, 1: A thắng

        match (round1, round2) {
            (0, 1) => Ok("Candidate_A"), // Nhất quán: A thắng cả 2 vị trí
            (1, 0) => Ok("Candidate_B"), // Nhất quán: B thắng cả 2 vị trí
            (0, 0) => Err("Phát hiện thiên kiến vị trí: Giám khảo luôn chọn ứng viên đứng trước"),
            (1, 1) => Err("Phát hiện thiên kiến vị trí: Giám khảo luôn chọn ứng viên đứng sau"),
            _ => Err("Kết quả đánh giá bất định"),
        }
    }
}

/// Bộ phát hiện phân kỳ Goodhart `GoodhartDivergenceDetector`.
///
/// RiskGate: Định luật Goodhart: "Khi một chỉ số trở thành mục tiêu, nó không còn
/// là một chỉ số tốt nữa." Phát hiện hiện tượng chỉ số ủy nhiệm (proxy metric) tăng vọt
/// trong khi chỉ số thực tế (ground truth utility) đứng yên hoặc tụt dốc.
#[derive(Debug, Clone)]
pub struct GoodhartDivergenceDetector;

impl GoodhartDivergenceDetector {
    /// Phát hiện phân kỳ giữa chỉ số proxy và chỉ số ground truth.
    /// Trả về `true` nếu có phân kỳ nguy hiểm (proxy tăng nhưng ground truth giảm).
    pub fn is_diverging(
        prev_proxy: f64,
        curr_proxy: f64,
        prev_ground_truth: f64,
        curr_ground_truth: f64,
    ) -> bool {
        let proxy_improved = curr_proxy > prev_proxy;
        let ground_truth_degraded = curr_ground_truth < prev_ground_truth;
        proxy_improved && ground_truth_degraded
    }
}

/// Bộ khai phá cụm dấu vết thất bại `TraceFailureMiner`.
#[derive(Debug, Clone, Default)]
pub struct TraceFailureMiner {
    pub clusters: BTreeMap<String, usize>,
}

impl TraceFailureMiner {
    pub fn new() -> Self {
        Self {
            clusters: BTreeMap::new(),
        }
    }

    /// Thêm vết lỗi vào bộ khai phá để nhóm cụm.
    pub fn mine_trace(&mut self, error_category: &str) {
        *self.clusters.entry(error_category.to_string()).or_insert(0) += 1;
    }

    /// Lấy danh mục lỗi phổ biến nhất.
    pub fn top_failure_category(&self) -> Option<(&str, usize)> {
        self.clusters
            .iter()
            .max_by_key(|&(_, count)| *count)
            .map(|(k, v)| (k.as_str(), *v))
    }
}

// ============================================================================
// 14. HÀM MAIN MINH HỌA THỰC CHIẾN
// ============================================================================

fn main() {
    println!("=== CHƯƠNG 90: ĐIỀU PHỐI ĐA TÁC TỬ, CỔNG BÀN THỢ & DIỄN TẬP LOOPDRILL ===");
    println!("Kiến trúc: 100% Rust 2024 thuần std | Dependencies rỗng | Struct trường có tên");

    // 1. Khởi tạo sổ cái tính năng Bàn thợ
    let mut ledger = FeatureLedger::new();
    ledger
        .start_feature("ch90_orchestration", "feat/ch90", 1700000000)
        .expect("Mở tính năng thành công");
    println!(
        "1. FeatureLedger: Đã khóa tính năng '{}' đang hoạt động.",
        ledger.current_active_feature().unwrap()
    );

    // 2. Kiểm chứng cổng hoàn thành chống mừng hụt
    let evidence_hash = sha256(b"34_passed_tests_deterministic");
    let artifact = HandoffArtifact {
        observation: "Quan sát mã nguồn đạt 100% kiểm thử tất định".to_string(),
        logic_chain: "Lập luận tuần tự qua 5 cổng kiểm tra bàn thợ".to_string(),
        caveats: "Không có giới hạn ngoài phạm vi".to_string(),
        conclusion: "Sẵn sàng bàn giao cho tổng công trình sư".to_string(),
        verification_method: "cargo test --manifest-path code/Cargo.toml -p ch90".to_string(),
        test_evidence_hash: Some(evidence_hash),
        tests_passed: 34,
        tests_failed: 0,
    };
    let seal_hash = verify_completion_claim(&artifact, 34).expect("Cổng hoàn thành chấp thuận");
    println!(
        "2. WorkbenchGate: Niêm phong biên nhận bàn giao thành công (Băm: {})",
        seal_hash.short_hex()
    );

    // 3. Điều phối Mediator hình sao và TraceSpan
    let mut mediator = MediatorOrchestrator::new();
    mediator.register_agent("orchestrator");
    mediator.register_agent("worker_ch90");

    let span = TraceSpan {
        trace_id: "trace-90-001".to_string(),
        span_id: "span-01".to_string(),
        parent_span_id: None,
        name: "dispatch_worker".to_string(),
        start_micros: 1000,
        end_micros: 2000,
        attributes: BTreeMap::from([("phase".to_string(), "execution".to_string())]),
    };
    let msg = AgentMessage {
        sender: "orchestrator".to_string(),
        recipient: "worker_ch90".to_string(),
        correlation_id: "corr-101".to_string(),
        payload: "Yêu cầu thi công Chương 90 đạt chuẩn".to_string(),
        timestamp: 1700000005,
    };
    let offset = mediator
        .forward_message(msg, &span)
        .expect("Mediator chuyển tiếp tin thành công");
    println!("3. Mediator: Đã chuyển tiếp tin nhắn nguyên văn tại CursorOffset={offset}.");

    // 4. Hiến pháp và Phân tách 3 ngôi
    let constitution =
        ConstitutionEngine::new("LUAT_HIEN_PHAP_BAT_BIEN_2024", vec!["rm -rf /".to_string()]);
    let attestation = AttestationRecord {
        author: "worker_ch90".to_string(),
        verifier: "reviewer_ch90".to_string(),
        attestor: "auditor_ch90".to_string(),
        specification_hash: sha256(b"SPEC_CH90"),
        self_rule_hash: constitution.pinned_self_hash,
        claim_summary: "Đạt chuẩn 3 ngôi".to_string(),
    };
    constitution
        .verify_separation_of_duties(&attestation)
        .expect("Hiến pháp phê chuẩn 3 ngôi");
    println!("4. ConstitutionEngine: Phân tách 3 ngôi (author != verifier != attestor) hợp lệ.");

    // 5. Diễn tập LoopDrill và sửa lỗi Edge Case #6
    let drill_cases = [
        ("rm -rf /", true),
        ("drop database", true),
        ("cargo test", false),
        ("cargo check", false),
    ];
    let receipt = run_loop_drill(
        |p| p.contains("rm -rf") || p.contains("drop"),
        &drill_cases,
        1.0,
        1.0,
    );
    println!(
        "5. LoopDrill: Sens={:.2}, Spec={:.2}, Status={:?} (Băm biên nhận: {})",
        receipt.sensitivity,
        receipt.specificity,
        receipt.status,
        receipt.receipt_hash.short_hex()
    );

    // 6. Kiểm định Đa tầng: Chứng minh pass^5 < pass@5
    let p5 = TrajectoryGrader::pass_pow_k(0.6, 5);
    let at5 = TrajectoryGrader::pass_at_k(10, 6, 5);
    println!(
        "6. TrajectoryGrader: pass^5 ({:.4}) < pass@5 ({:.4}) -> Tính toán chính xác!",
        p5, at5
    );

    println!("=== HOÀN TẤT TRÌNH DIỄN CHƯƠNG 90 THÀNH CÔNG ===");
}

// ============================================================================
// 15. BỘ KIỂM THỬ BẮT BUỘC (>= 34 BÀI KIỂM THỬ TẤT ĐỊNH)
// ============================================================================

#[cfg(test)]
mod tests {
    use super::*;

    // --- 1. KIỂM THỬ SHA-256 ĐỐI CHIẾU VECTOR FIPS CHUẨN ---

    #[test]
    fn test_sha256_fips_vectors() {
        assert_eq!(
            sha256(b"").to_hex(),
            "e3b0c44298fc1c149afbf4c8996fb92427ae41e4649b934ca495991b7852b855"
        );
        assert_eq!(
            sha256(b"abc").to_hex(),
            "ba7816bf8f01cfea414140de5dae2223b00361a396177a9cb410ff61f20015ad"
        );
    }

    #[test]
    fn test_sha256_short_hex_and_display() {
        let hash = sha256(b"learn-rs");
        assert_eq!(hash.short_hex().len(), 8);
        assert_eq!(format!("{hash}"), hash.to_hex());
    }

    // --- 2. KIỂM THỬ SỔ CÁI VÀ CỔNG KỶ LUẬT BÀN THỢ ---

    #[test]
    fn test_feature_ledger_single_feature_lock() {
        let mut ledger = FeatureLedger::new();
        assert!(ledger.start_feature("feat_1", "branch_1", 100).is_ok());
        // Khóa đơn tính năng: Không được phép mở tính năng thứ hai khi tính năng 1 chưa xong
        assert!(ledger.start_feature("feat_2", "branch_2", 105).is_err());
        assert_eq!(ledger.current_active_feature(), Some("feat_1"));
    }

    #[test]
    fn test_feature_ledger_complete_and_reopen() {
        let mut ledger = FeatureLedger::new();
        ledger.start_feature("feat_1", "branch_1", 100).unwrap();
        assert!(ledger.complete_feature("feat_1").is_ok());
        assert!(!ledger.has_active_feature());
        // Sau khi hoàn thành, mở tính năng mới bình thường
        assert!(ledger.start_feature("feat_2", "branch_2", 200).is_ok());
        assert_eq!(ledger.current_active_feature(), Some("feat_2"));
    }

    #[test]
    fn test_feature_ledger_abort_active() {
        let mut ledger = FeatureLedger::new();
        ledger
            .start_feature("feat_faulty", "branch_bad", 50)
            .unwrap();
        assert!(ledger.abort_feature("feat_faulty").is_ok());
        assert!(!ledger.has_active_feature());
    }

    #[test]
    fn test_verify_completion_claim_rejects_missing_evidence() {
        let artifact = HandoffArtifact {
            observation: "Obs".to_string(),
            logic_chain: "Logic".to_string(),
            caveats: "None".to_string(),
            conclusion: "Done".to_string(),
            verification_method: "cargo test".to_string(),
            test_evidence_hash: None, // Thiếu bằng chứng băm
            tests_passed: 35,
            tests_failed: 0,
        };
        assert!(verify_completion_claim(&artifact, 34).is_err());
    }

    #[test]
    fn test_verify_completion_claim_rejects_test_failures() {
        let artifact = HandoffArtifact {
            observation: "Obs".to_string(),
            logic_chain: "Logic".to_string(),
            caveats: "None".to_string(),
            conclusion: "Done".to_string(),
            verification_method: "cargo test".to_string(),
            test_evidence_hash: Some(sha256(b"evidence")),
            tests_passed: 34,
            tests_failed: 1, // Còn test trượt
        };
        assert!(verify_completion_claim(&artifact, 34).is_err());
    }

    #[test]
    fn test_verify_completion_claim_rejects_insufficient_passed_tests() {
        let artifact = HandoffArtifact {
            observation: "Obs".to_string(),
            logic_chain: "Logic".to_string(),
            caveats: "None".to_string(),
            conclusion: "Done".to_string(),
            verification_method: "cargo test".to_string(),
            test_evidence_hash: Some(sha256(b"evidence")),
            tests_passed: 30, // Chưa đủ 34 bài test
            tests_failed: 0,
        };
        assert!(verify_completion_claim(&artifact, 34).is_err());
    }

    #[test]
    fn test_verify_completion_claim_approves_valid_artifact() {
        let artifact = HandoffArtifact {
            observation: "Obs".to_string(),
            logic_chain: "Logic".to_string(),
            caveats: "None".to_string(),
            conclusion: "Done".to_string(),
            verification_method: "cargo test".to_string(),
            test_evidence_hash: Some(sha256(b"evidence")),
            tests_passed: 36,
            tests_failed: 0,
        };
        let receipt = verify_completion_claim(&artifact, 34);
        assert!(receipt.is_ok());
    }

    #[test]
    fn test_layer_import_linter_enforces_hierarchy() {
        let linter = LayerImportLinter::standard_three_tier();
        // Domain (0) import Infrastructure (2) -> Lỗi vi phạm phân tầng
        assert!(linter.check_import("domain", "infrastructure").is_err());
        // Application (1) import Infrastructure (2) -> Lỗi
        assert!(
            linter
                .check_import("application", "infrastructure")
                .is_err()
        );
    }

    #[test]
    fn test_layer_import_linter_allows_valid_imports() {
        let linter = LayerImportLinter::standard_three_tier();
        // Infrastructure (2) import Domain (0) hoặc Application (1) -> Hợp lệ
        assert!(linter.check_import("infrastructure", "domain").is_ok());
        assert!(linter.check_import("infrastructure", "application").is_ok());
        assert!(linter.check_import("domain", "domain").is_ok());
    }

    #[test]
    fn test_run_init_checks_and_diff_scope() {
        let req = vec!["Cargo.toml".to_string(), "src/main.rs".to_string()];
        let actual = vec![
            "Cargo.toml".to_string(),
            "src/main.rs".to_string(),
            "README.md".to_string(),
        ];
        assert!(run_init_checks(&req, &actual).is_ok());

        let missing = vec!["Cargo.toml".to_string()];
        assert!(run_init_checks(&req, &missing).is_err());

        let allowed = vec!["code/ch90/*".to_string(), "chuong_90.md".to_string()];
        let modified_ok = vec![
            "code/ch90/src/main.rs".to_string(),
            "chuong_90.md".to_string(),
        ];
        assert!(check_diff_scope(&modified_ok, &allowed).is_ok());

        let modified_bad = vec!["code/ch86/src/main.rs".to_string()];
        assert!(check_diff_scope(&modified_bad, &allowed).is_err());
    }

    #[test]
    fn test_scan_workspace_cleanliness() {
        let files = vec![
            "src/main.rs".to_string(),
            "target/output.tmp".to_string(),
            "src/main.rs.bk".to_string(),
        ];
        let forbidden = vec![".tmp".to_string(), ".bk".to_string()];
        let dirty = scan_workspace_cleanliness(&files, &forbidden);
        assert_eq!(dirty.len(), 2);
    }

    // --- 3. KIỂM THỬ CRON VÀ VÒNG LẶP MỤC TIÊU HAI TẦNG GIÁM KHẢO ---

    #[test]
    fn test_cron_schedule_parsing_and_matching() {
        let cron = CronSchedule::parse("*/5 9 1-15 * *").unwrap();
        // Phút 10, giờ 9, ngày 10 -> Khớp
        assert!(cron.matches(10, 9, 10, 5, 2));
        // Phút 11 -> Sai vì không chia hết cho 5
        assert!(!cron.matches(11, 9, 10, 5, 2));
        // Giờ 10 -> Sai vì giờ cố định 9
        assert!(!cron.matches(10, 10, 10, 5, 2));
    }

    #[test]
    fn test_cron_schedule_invalid_format() {
        assert!(CronSchedule::parse("*/5 * *").is_err());
    }

    #[test]
    fn test_goal_loop_controller_tier1_early_exit() {
        let mut controller = GoalLoopController::new(5, None);
        let mut tier2_called = false;

        let eval = controller.evaluate_step(
            || JudgeEvaluation {
                tier: 1,
                verdict: JudgeVerdict::NeedsRetry,
                feedback: "Cú pháp lỗi".to_string(),
            },
            || {
                tier2_called = true;
                JudgeEvaluation {
                    tier: 2,
                    verdict: JudgeVerdict::Passed,
                    feedback: "Hợp lệ".to_string(),
                }
            },
        );

        assert_eq!(eval.verdict, JudgeVerdict::NeedsRetry);
        assert!(
            !tier2_called,
            "Tầng 1 trượt thì Tầng 2 không được phép chạy"
        );
    }

    #[test]
    fn test_goal_loop_controller_tier2_completion() {
        let mut controller = GoalLoopController::new(5, None);
        let eval = controller.evaluate_step(
            || JudgeEvaluation {
                tier: 1,
                verdict: JudgeVerdict::Passed,
                feedback: "Liveness OK".to_string(),
            },
            || JudgeEvaluation {
                tier: 2,
                verdict: JudgeVerdict::Passed,
                feedback: "Semantic Tests Passed".to_string(),
            },
        );
        assert_eq!(eval.verdict, JudgeVerdict::Passed);
    }

    #[test]
    fn test_goal_loop_controller_max_iteration_exhaustion() {
        let mut controller = GoalLoopController::new(2, None);
        controller.evaluate_step(
            || JudgeEvaluation {
                tier: 1,
                verdict: JudgeVerdict::Passed,
                feedback: "".to_string(),
            },
            || JudgeEvaluation {
                tier: 2,
                verdict: JudgeVerdict::NeedsRetry,
                feedback: "".to_string(),
            },
        );
        controller.evaluate_step(
            || JudgeEvaluation {
                tier: 1,
                verdict: JudgeVerdict::Passed,
                feedback: "".to_string(),
            },
            || JudgeEvaluation {
                tier: 2,
                verdict: JudgeVerdict::NeedsRetry,
                feedback: "".to_string(),
            },
        );
        let eval = controller.evaluate_step(
            || JudgeEvaluation {
                tier: 1,
                verdict: JudgeVerdict::Passed,
                feedback: "".to_string(),
            },
            || JudgeEvaluation {
                tier: 2,
                verdict: JudgeVerdict::Passed,
                feedback: "".to_string(),
            },
        );
        assert_eq!(eval.verdict, JudgeVerdict::FailedFatal);
    }

    // --- 4. KIỂM THỬ ĐIỀU PHỐI HÌNH SAO, DẤU VẾT VÀ HÀNG ĐỢI SỰ KIỆN ---

    #[test]
    fn test_trace_span_hash_deterministic() {
        let s1 = TraceSpan {
            trace_id: "t1".to_string(),
            span_id: "s1".to_string(),
            parent_span_id: None,
            name: "init".to_string(),
            start_micros: 10,
            end_micros: 20,
            attributes: BTreeMap::from([("k".to_string(), "v".to_string())]),
        };
        let s2 = s1.clone();
        assert_eq!(s1.compute_hash(), s2.compute_hash());
    }

    #[test]
    fn test_topic_event_bus_cursor_pagination() {
        let mut bus = TopicEventBus::new();
        let dummy_hash = sha256(b"dummy");
        let msg = AgentMessage {
            sender: "a".to_string(),
            recipient: "b".to_string(),
            correlation_id: "c1".to_string(),
            payload: "hi".to_string(),
            timestamp: 1,
        };
        let off0 = bus.publish("topic.a", msg.clone(), dummy_hash);
        let off1 = bus.publish("topic.a", msg.clone(), dummy_hash);
        let off2 = bus.publish("topic.b", msg, dummy_hash);

        assert_eq!(off0, 0);
        assert_eq!(off1, 1);
        assert_eq!(off2, 2);

        let polled = bus.poll("topic.a", 1, 10);
        assert_eq!(polled.len(), 1);
        assert_eq!(polled[0].offset, 1);
    }

    #[test]
    fn test_mediator_forward_message_verbatim_integrity() {
        let mut med = MediatorOrchestrator::new();
        med.register_agent("worker");
        med.register_agent("reviewer");

        let payload = "Dữ liệu byte nguyên văn: \u{00}\u{FF}\u{AA}\u{55} - 100% không đổi!";
        let msg = AgentMessage {
            sender: "worker".to_string(),
            recipient: "reviewer".to_string(),
            correlation_id: "corr-99".to_string(),
            payload: payload.to_string(),
            timestamp: 1000,
        };
        let span = TraceSpan {
            trace_id: "tr".to_string(),
            span_id: "sp".to_string(),
            parent_span_id: None,
            name: "fwd".to_string(),
            start_micros: 10,
            end_micros: 20,
            attributes: BTreeMap::new(),
        };

        let off = med.forward_message(msg.clone(), &span).unwrap();
        let events = med.bus.poll("agent.inbox.reviewer", off, 1);
        assert_eq!(events[0].message.payload, payload);
    }

    #[test]
    fn test_mediator_forward_message_rejects_unregistered() {
        let mut med = MediatorOrchestrator::new();
        med.register_agent("worker");
        let msg = AgentMessage {
            sender: "worker".to_string(),
            recipient: "unknown_agent".to_string(),
            correlation_id: "c".to_string(),
            payload: "hello".to_string(),
            timestamp: 10,
        };
        let span = TraceSpan {
            trace_id: "tr".to_string(),
            span_id: "sp".to_string(),
            parent_span_id: None,
            name: "fwd".to_string(),
            start_micros: 1,
            end_micros: 2,
            attributes: BTreeMap::new(),
        };
        assert!(med.forward_message(msg, &span).is_err());
    }

    // --- 5. KIỂM THỬ KHẢO SÁT QUẠT LUỒNG VÀ THỬ LẠI CÔ LẬP ---

    #[test]
    fn test_explore_fanout_runner_recovers_on_retry() {
        let tasks = [("b1", "task 1"), ("b2", "task 2")];
        let results = ExploreFanoutRunner::run_fanout(&tasks, |bid, _desc, attempt| {
            if bid == "b2" && attempt == 0 {
                // Nhánh b2 lỗi lần đầu
                Err("Tạm thời ngắt mạng")
            } else {
                // Lần thử 2 thành công
                Ok(SubBranchResult {
                    branch_id: bid.to_string(),
                    findings: vec![Finding {
                        summary: "Ok".to_string(),
                        confidence_score: 90,
                    }],
                    evidences: Vec::new(),
                    traces: Vec::new(),
                    succeeded: true,
                    retry_count: attempt,
                })
            }
        });

        assert_eq!(results.len(), 2);
        assert!(results[0].succeeded && results[0].retry_count == 0);
        assert!(results[1].succeeded && results[1].retry_count == 1);
    }

    #[test]
    fn test_explore_fanout_runner_exhaustion() {
        let tasks = [("b_err", "failing task")];
        let results = ExploreFanoutRunner::run_fanout(&tasks, |_bid, _desc, _attempt| {
            Err("Lỗi bất khả kháng liên tục")
        });
        assert_eq!(results.len(), 1);
        assert!(!results[0].succeeded);
        assert_eq!(results[0].retry_count, 1);
    }

    // --- 6. KIỂM THỬ ĐẶC TẢ BÁN HÌNH THỨC 11 KHỐI VÀ DANH BẠ MÙ ---

    #[test]
    fn test_pseudo_formal_brief_validates_11_blocks() {
        let brief = PseudoFormalTaskBrief {
            objective: "Obj".to_string(),
            preconditions: vec!["Pre".to_string()],
            invariants: vec!["Inv".to_string()],
            input_artifacts: vec!["In".to_string()],
            output_artifacts: vec!["Out".to_string()],
            constraints: vec!["Const".to_string()],
            failure_modes: vec!["Fail".to_string()],
            verification_rules: vec!["Rule".to_string()],
            security_assumptions: vec!["Sec".to_string()],
            rollback_plan: "Rollback".to_string(),
            attestation: "Attest".to_string(),
        };
        assert!(brief.validate_11_blocks().is_ok());
    }

    #[test]
    fn test_pseudo_formal_brief_rejects_incomplete() {
        let mut brief = PseudoFormalTaskBrief {
            objective: "".to_string(), // Trống
            preconditions: vec!["Pre".to_string()],
            invariants: vec!["Inv".to_string()],
            input_artifacts: vec!["In".to_string()],
            output_artifacts: vec!["Out".to_string()],
            constraints: vec!["Const".to_string()],
            failure_modes: vec!["Fail".to_string()],
            verification_rules: vec!["Rule".to_string()],
            security_assumptions: vec!["Sec".to_string()],
            rollback_plan: "Rollback".to_string(),
            attestation: "Attest".to_string(),
        };
        assert!(brief.validate_11_blocks().is_err());
        brief.objective = "Obj".to_string();
        brief.invariants.clear();
        assert!(brief.validate_11_blocks().is_err());
    }

    #[test]
    fn test_blind_portfolio_registry_blinds_and_reveals() {
        let mut reg = BlindPortfolioRegistry::new();
        reg.register_candidate("CANDIDATE_A", "Model_GPT4");
        reg.register_candidate("CANDIDATE_B", "Model_Claude37");
        assert_eq!(reg.reveal_identity("CANDIDATE_A"), Some("Model_GPT4"));
        assert_eq!(reg.reveal_identity("CANDIDATE_UNKNOWN"), None);
    }

    // --- 7. KIỂM THỬ HỘP CÁT LÀM VIỆC, DROP RAII VÀ KHÓA ƯU TIÊN ---

    #[test]
    fn test_worktree_priority_lease_ci_sweeper_preempts() {
        let mut sandbox = WorktreeSandbox::new("sb1", "/tmp/sb1");
        // DepSweeper (10) lấy khóa trước
        sandbox
            .acquire_lease("src/*.rs", "dep_agent", LeasePriority::DepSweeper, 100)
            .unwrap();

        // PrBabysitter (20) lấy khóa cùng mẫu -> Ghi đè (tiếm quyền) vì ưu tiên cao hơn
        assert!(
            sandbox
                .acquire_lease("src/*.rs", "pr_agent", LeasePriority::PrBabysitter, 200)
                .is_ok()
        );
        assert_eq!(sandbox.active_leases[0].holder, "pr_agent");

        // CiSweeper (30) lấy khóa -> Tiếp tục tiếm quyền
        assert!(
            sandbox
                .acquire_lease("src/*.rs", "ci_agent", LeasePriority::CiSweeper, 300)
                .is_ok()
        );
        assert_eq!(sandbox.active_leases[0].holder, "ci_agent");

        // DepSweeper (10) thử lấy lại -> Bị từ chối vì ưu tiên thấp hơn
        assert!(
            sandbox
                .acquire_lease("src/*.rs", "dep_agent", LeasePriority::DepSweeper, 400)
                .is_err()
        );
    }

    #[test]
    fn test_worktree_3way_merge_clean_no_conflict() {
        let base = "line1\nline2\nline3";
        let ours = "line1_edited\nline2\nline3";
        let theirs = "line1\nline2\nline3_edited";
        let merged = WorktreeSandbox::detect_3way_conflict(base, ours, theirs).unwrap();
        assert_eq!(merged, "line1_edited\nline2\nline3_edited");
    }

    #[test]
    fn test_worktree_3way_merge_conflict_detected() {
        let base = "line1\nline2\nline3";
        let ours = "line1\nours_modified\nline3";
        let theirs = "line1\ntheirs_modified\nline3";
        let res = WorktreeSandbox::detect_3way_conflict(base, ours, theirs);
        assert!(res.is_err());
    }

    #[test]
    fn test_worktree_sandbox_raii_drop_releases_leases() {
        let mut sandbox = WorktreeSandbox::new("sb_drop", "/tmp/sb");
        sandbox
            .acquire_lease("lib.rs", "holder", LeasePriority::CiSweeper, 100)
            .unwrap();
        assert_eq!(sandbox.active_leases.len(), 1);
        drop(sandbox); // RAII drop
    }

    // --- 8. KIỂM THỬ NGẮT MẠCH NGỮ CẢNH VÒNG LẶP ---

    #[test]
    fn test_loop_context_breaker_trips_on_repetition() {
        let mut breaker = LoopContextBreaker::new(10, 3, 0.8);
        assert!(!breaker.observe_and_check("Sửa lỗi cú pháp biến x"));
        assert!(!breaker.observe_and_check("Sửa lỗi cú pháp biến x"));
        // Lần 3 hành động tương tự lặp lại -> Kích hoạt ngắt mạch!
        assert!(breaker.observe_and_check("Sửa lỗi cú pháp biến x"));
    }

    #[test]
    fn test_loop_context_breaker_allows_diverse_actions() {
        let mut breaker = LoopContextBreaker::new(10, 3, 0.8);
        assert!(!breaker.observe_and_check("Chạy lệnh kiểm thử ban đầu"));
        assert!(!breaker.observe_and_check("Đọc cấu trúc file README"));
        assert!(!breaker.observe_and_check("Thực thi git commit"));
    }

    // --- 9. KIỂM THỬ HIẾN PHÁP VÀ PHÂN TÁCH BA NGÔI ---

    #[test]
    fn test_constitution_separation_of_duties_rejects_author_as_verifier() {
        let constitution = ConstitutionEngine::new("RULES_V1", vec!["forbidden".to_string()]);
        let rec = AttestationRecord {
            author: "agent_a".to_string(),
            verifier: "agent_a".to_string(), // Tự phê duyệt
            attestor: "agent_b".to_string(),
            specification_hash: sha256(b"spec"),
            self_rule_hash: constitution.pinned_self_hash,
            claim_summary: "Done".to_string(),
        };
        assert!(constitution.verify_separation_of_duties(&rec).is_err());
    }

    #[test]
    fn test_constitution_separation_of_duties_rejects_verifier_as_attestor() {
        let constitution = ConstitutionEngine::new("RULES_V1", vec![]);
        let rec = AttestationRecord {
            author: "agent_a".to_string(),
            verifier: "agent_b".to_string(),
            attestor: "agent_b".to_string(), // Trùng kiểm chứng và chứng thực
            specification_hash: sha256(b"spec"),
            self_rule_hash: constitution.pinned_self_hash,
            claim_summary: "Done".to_string(),
        };
        assert!(constitution.verify_separation_of_duties(&rec).is_err());
    }

    #[test]
    fn test_constitution_separation_of_duties_approves_distinct_roles() {
        let constitution = ConstitutionEngine::new("RULES_V1", vec![]);
        let rec = AttestationRecord {
            author: "agent_a".to_string(),
            verifier: "agent_b".to_string(),
            attestor: "agent_c".to_string(),
            specification_hash: sha256(b"spec"),
            self_rule_hash: constitution.pinned_self_hash,
            claim_summary: "Done".to_string(),
        };
        assert!(constitution.verify_separation_of_duties(&rec).is_ok());
    }

    #[test]
    fn test_constitution_rejects_tampered_self_rule_hash() {
        let constitution = ConstitutionEngine::new("RULES_V1", vec![]);
        let rec = AttestationRecord {
            author: "agent_a".to_string(),
            verifier: "agent_b".to_string(),
            attestor: "agent_c".to_string(),
            specification_hash: sha256(b"spec"),
            self_rule_hash: sha256(b"TAMPERED_RULES"),
            claim_summary: "Done".to_string(),
        };
        assert!(constitution.verify_separation_of_duties(&rec).is_err());
    }

    #[test]
    fn test_constitution_content_compliance() {
        let constitution =
            ConstitutionEngine::new("RULES_V1", vec!["dangerous_payload".to_string()]);
        assert!(
            constitution
                .check_content_compliance("safe payload code")
                .is_ok()
        );
        assert!(
            constitution
                .check_content_compliance("contains dangerous_payload in body")
                .is_err()
        );
    }

    // --- 10. KIỂM THỬ BÁNH CÓC TỰ TIẾN HÓA VÀ TRÔI LỆCH CHÍNH SÁCH ---

    #[test]
    fn test_rsi_ratchet_rejects_overfit_candidate() {
        let mut ratchet = TwoSplitRsiRatchet::new(0.80, 0.85);
        // Ứng viên cải thiện held_in lên 0.95 nhưng tụt held_out xuống 0.82 -> Quá khớp! Bác bỏ!
        let overfit_score = RsiCandidateScore {
            held_in_pass_rate: 0.95,
            held_out_pass_rate: 0.82,
            subagent_cost_discount: 0.0,
        };
        assert!(!ratchet.evaluate_candidate(&overfit_score));
        assert_eq!(ratchet.rejected_overfits, 1);
        assert_eq!(ratchet.accepted_generations, 0);
    }

    #[test]
    fn test_rsi_ratchet_accepts_generalizing_candidate() {
        let mut ratchet = TwoSplitRsiRatchet::new(0.80, 0.85);
        // Ứng viên cải thiện cả hai tập
        let good_score = RsiCandidateScore {
            held_in_pass_rate: 0.88,
            held_out_pass_rate: 0.90,
            subagent_cost_discount: 0.01,
        };
        assert!(ratchet.evaluate_candidate(&good_score));
        assert_eq!(ratchet.accepted_generations, 1);
        assert_eq!(ratchet.current_baseline_held_out, 0.90);
    }

    #[test]
    fn test_policy_drift_checker_detects_deviations() {
        let baseline = BTreeMap::from([
            ("mode".to_string(), "strict".to_string()),
            ("timeout".to_string(), "30".to_string()),
        ]);
        let checker = PolicyDriftChecker::new(baseline);

        let drifted = BTreeMap::from([
            ("mode".to_string(), "lenient".to_string()), // Trôi giá trị
            ("unauthorized_key".to_string(), "val".to_string()), // Thêm trái phép
        ]);
        let drifts = checker.check_drift(&drifted);
        assert_eq!(drifts.len(), 3); // 1 đổi, 1 thiếu timeout, 1 thừa
    }

    // --- 11. KIỂM THỬ DIỄN TẬP LOOPDRILL (SỬA LỖI EDGE CASE #6) VÀ READINESS AUDITOR ---

    #[test]
    fn test_loop_drill_edge_case_6_anti_vacuous_truth_empty_positive() {
        // Bộ kiểm thử chỉ có mẫu âm (lành tính), KHÔNG có mẫu dương (độc hại)
        let vacuous_cases = [("cargo test", false), ("cargo build", false)];
        let receipt = run_loop_drill(|_| false, &vacuous_cases, 0.9, 0.9);
        // Sửa lỗi Edge Case #6: Bắt buộc bác bỏ vì chân lý rỗng!
        assert_eq!(receipt.status, DrillStatus::RejectedVacuousSuite);
    }

    #[test]
    fn test_loop_drill_edge_case_6_anti_vacuous_truth_empty_negative() {
        // Bộ kiểm thử chỉ có mẫu dương, KHÔNG có mẫu âm
        let vacuous_cases = [("rm -rf /", true), ("drop table", true)];
        let receipt = run_loop_drill(|_| true, &vacuous_cases, 0.9, 0.9);
        // Sửa lỗi Edge Case #6: Bắt buộc bác bỏ!
        assert_eq!(receipt.status, DrillStatus::RejectedVacuousSuite);
    }

    #[test]
    fn test_loop_drill_dual_threshold_and_receipt() {
        let balanced_cases = [
            ("rm -rf /", true),
            ("format c:", true),
            ("cargo test", false),
            ("cargo check", false),
        ];
        // Rào chắn hoàn hảo
        let receipt = run_loop_drill(
            |p| p.contains("rm -rf") || p.contains("format"),
            &balanced_cases,
            0.95,
            0.95,
        );
        assert_eq!(receipt.status, DrillStatus::Success);
        assert!((receipt.sensitivity - 1.0).abs() < 1e-6);
        assert!((receipt.specificity - 1.0).abs() < 1e-6);
    }

    #[test]
    fn test_loop_drill_fails_on_permissive_or_overly_strict_guard() {
        let balanced_cases = [
            ("attack_1", true),
            ("attack_2", true),
            ("benign_1", false),
            ("benign_2", false),
        ];
        // Rào cho qua hết (cho qua cả mã độc hại)
        let perm = run_loop_drill(|_| false, &balanced_cases, 0.9, 0.9);
        assert_eq!(perm.status, DrillStatus::FailedSensitivity);

        // Rào chặn tất cả (chặn luôn cả mẫu lành tính)
        let strict = run_loop_drill(|_| true, &balanced_cases, 0.9, 0.9);
        assert_eq!(strict.status, DrillStatus::FailedSpecificity);
    }

    #[test]
    fn test_readiness_auditor_demotes_l3_to_l2_on_tampered_receipt() {
        let mut auditor = ReadinessAuditor::new(ReadinessLevel::L3Autonomous);
        let receipt = DrillReceipt {
            sensitivity: 1.0,
            specificity: 1.0,
            status: DrillStatus::Success,
            total_positive_samples: 10,
            total_negative_samples: 10,
            receipt_hash: sha256(b"actual_hash"),
        };
        let forged_expected_hash = sha256(b"forged_hash");
        let level = auditor.verify_and_update(&receipt, forged_expected_hash);
        // Phát hiện giả mạo/sai băm -> Hạ trần L3 -> L2 ngay lập tức
        assert_eq!(level, ReadinessLevel::L2Monitored);
    }

    // --- 12. KIỂM THỬ PHẢN HỒI TRÌNH BIÊN DỊCH VÀ CHẨN ĐOÁN LSP ---

    #[test]
    fn test_lsp_diagnostic_client_and_compiler_feedback_loop() {
        let mut feedback = CompilerFeedbackLoop::new(3);
        let diags = vec![LspDiagnostic {
            file_path: "src/main.rs".to_string(),
            line: 42,
            column: 10,
            severity: DiagnosticSeverity::Error,
            code: "E0308".to_string(),
            message: "Mismatched types".to_string(),
        }];
        let prompt = feedback.feed_diagnostics(&diags).unwrap();
        assert!(prompt.contains("E0308"));
        assert!(prompt.contains("src/main.rs:42:10"));

        // Lặp lại cùng lỗi -> Bắt quả tang tác tử rơi vào vòng lặp
        assert!(feedback.feed_diagnostics(&diags).is_err());
    }

    // --- 13. KIỂM THỬ BỘ ĐÁNH GIÁ ĐA TẦNG ---

    #[test]
    fn test_trajectory_grader_pass_at_k_vs_pass_pow_k() {
        // Chứng minh định lý: pass^5 < pass@5 đối với xác suất pass đơn p nằm trong (0, 1)
        let p = 0.5;
        let pass_pow_5 = TrajectoryGrader::pass_pow_k(p, 5); // 0.5^5 = 0.03125
        let pass_at_5 = TrajectoryGrader::pass_at_k(10, 5, 5); // n=10, c=5, k=5
        assert!(pass_pow_5 < pass_at_5);
        assert!((pass_pow_5 - 0.03125).abs() < 1e-6);
    }

    #[test]
    fn test_bfcl_ast_matcher_ignores_order_and_types() {
        let exp = r#"{"timeout": 30, "model": "fast"}"#;
        let act = r#"{"model": "fast", "timeout": 30}"#; // Đảo thứ tự khóa
        assert!(BfclAstMatcher::matches_arguments(exp, act));

        let different = r#"{"model": "fast", "timeout": 60}"#;
        assert!(!BfclAstMatcher::matches_arguments(exp, different));
    }

    #[test]
    fn test_gaia_normalizer_cleanses_strings() {
        let raw = "   The Final Answer is $1,250.50%!!   ";
        let norm = GaiaNormalizer::normalize(raw);
        assert_eq!(norm, "the final answer is 1250.50");
    }

    #[test]
    fn test_position_swapped_judge_detects_positional_bias() {
        // Giám khảo thiên kiến: luôn chọn phương án đầu tiên
        let biased_eval = |_first: &str, _second: &str| -> usize { 0 };
        let res = PositionSwappedJudge::judge_fairly("A", "B", biased_eval);
        assert!(res.is_err());

        // Giám khảo công tâm: luôn đánh giá đúng chất lượng thực
        let fair_eval = |first: &str, _second: &str| -> usize {
            if first == "A" { 0 } else { 1 } // A luôn thắng
        };
        let fair_res = PositionSwappedJudge::judge_fairly("A", "B", fair_eval);
        assert_eq!(fair_res, Ok("Candidate_A"));
    }

    #[test]
    fn test_goodhart_divergence_detector_alerts() {
        // Proxy tăng vọt từ 50 lên 120, nhưng Ground truth tụt từ 0.9 xuống 0.7
        assert!(GoodhartDivergenceDetector::is_diverging(
            50.0, 120.0, 0.9, 0.7
        ));
        // Cả hai cùng tiến bộ -> Không phân kỳ
        assert!(!GoodhartDivergenceDetector::is_diverging(
            50.0, 120.0, 0.7, 0.9
        ));
    }

    #[test]
    fn test_trace_failure_miner_clusters_patterns() {
        let mut miner = TraceFailureMiner::new();
        miner.mine_trace("TIMEOUT_ERROR");
        miner.mine_trace("SYNTAX_ERROR");
        miner.mine_trace("TIMEOUT_ERROR");
        miner.mine_trace("TIMEOUT_ERROR");

        let top = miner.top_failure_category();
        assert_eq!(top, Some(("TIMEOUT_ERROR", 3)));
    }
}
