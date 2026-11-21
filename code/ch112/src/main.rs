#![forbid(unsafe_code)]

//! # Chương 112 — Red Team Capstone II: Tác tử Đa Module Tự Hành (`MockCapstoneLab`)
//!
//! Crate tổng hợp toàn diện Phần II (Chương 106–111) kết nối tư duy hiến pháp & RL từ Chương 105:
//! - Giao diện chung `PentestModule` (`id`, `run(&mut self, &EngagementContext, ModuleInput)`)
//!   với `ModuleInput`/`ModuleOutput` định kiểu chặt chẽ — tuyệt đối không chuỗi lệnh shell tự do.
//! - `ModuleRegistry` kiểm tra đồ thị phụ thuộc, tính tương thích Input/Output và chuyển pha hợp lệ.
//! - 6 adapter giả lập tự chứa: `OsintModuleAdapter`, `WirelessModuleAdapter`,
//!   `AdEnumerationModuleAdapter`, `AdConditionalAttackAdapter`, `PostExploitationAuditAdapter`,
//!   và `ReportingModuleAdapter` (không import binary crates cũ, không Cloud pentest).
//! - An toàn trước khi chạy module đầu tiên: kiểm tra chữ ký ủy quyền, khung giờ UTC,
//!   `EngagementScope` (Deny thắng Allow, chấp thuận bên thứ 3, `.example` & RFC 1918 nghiêm ngặt),
//!   cổng phê duyệt `ApprovalGrant`, ngân sách `ResourceBudget`, `kill_switch` và hành động bị cấm.
//! - Provenance & toàn vẹn: `EvidenceLedger` chuỗi băm SHA-256 đóng khung độ dài, tự động che
//!   secret/PII, chặn claim không có bằng chứng (`UnsupportedClaim`), khử trùng lặp độc lập hoán vị.
//! - `OrchestratorCheckpoint` nguyên tử có MAC SHA-256 & `scenario_fingerprint`: phục hồi sau lỗi
//!   tạm thời mà **không** chạy lại pha đã commit và **không** nhân đôi bằng chứng.
//! - Đối kháng `Red-vs-Blue` tất định và hàm phần thưởng RL dùng bảng trọng số phạt **độ lớn dương**
//!   trừ trong công thức; mọi hành động Red bị Blue chặn không bao giờ được tính là thành công.
//! - Báo cáo cuối `CapstoneReport` tất định từng byte (JSON & Markdown) kèm `CapstoneEvaluator`.

use std::collections::{BTreeMap, BTreeSet};
use std::fmt;

pub const CAPSTONE_SCHEMA_VERSION: &str = "1.0.0";
pub const CHECKPOINT_VERSION: u32 = 1;
pub const ENGAGEMENT_ID: &str = "ENG-2026-CAPSTONE-112";
pub const WINDOW_START_UTC: &str = "2026-03-01T00:00:00Z";
pub const WINDOW_END_UTC: &str = "2026-03-10T23:59:59Z";
pub const VIRTUAL_NOW_UTC: &str = "2026-03-04T14:30:00Z";
pub const REDACTED_SECRET_MARKER: &str = "[REDACTED-SECRET]";
pub const REDACTED_PII_MARKER: &str = "[REDACTED-PII]";
pub const LEDGER_GENESIS_HASH: &str =
    "0000000000000000000000000000000000000000000000000000000000000000";

// ============================================================================
// 1. NỀN TẢNG MẬT MÃ HỌC THUẦN STD (FIPS 180-4 SHA-256) & SPLITMIX64
// ============================================================================

/// Hàm băm SHA-256 chuẩn FIPS 180-4 (thuần `std`) dùng cho bằng chứng, checkpoint MAC và digest báo cáo.
pub fn sha256_hex(bytes: &[u8]) -> String {
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

    let mut state: [u32; 8] = [
        0x6a09e667, 0xbb67ae85, 0x3c6ef372, 0xa54ff53a, 0x510e527f, 0x9b05688c, 0x1f83d9ab,
        0x5be0cd19,
    ];

    let bit_len = (bytes.len() as u64).wrapping_mul(8);
    let mut padded = bytes.to_vec();
    padded.push(0x80);
    while (padded.len() % 64) != 56 {
        padded.push(0);
    }
    padded.extend_from_slice(&bit_len.to_be_bytes());

    for chunk in padded.as_chunks::<64>().0 {
        let mut w = [0u32; 64];
        for (i, word) in w.iter_mut().take(16).enumerate() {
            let off = i * 4;
            *word =
                u32::from_be_bytes([chunk[off], chunk[off + 1], chunk[off + 2], chunk[off + 3]]);
        }
        for i in 16..64 {
            let s0 = w[i - 15].rotate_right(7) ^ w[i - 15].rotate_right(18) ^ (w[i - 15] >> 3);
            let s1 = w[i - 2].rotate_right(17) ^ w[i - 2].rotate_right(19) ^ (w[i - 2] >> 10);
            w[i] = w[i - 16]
                .wrapping_add(s0)
                .wrapping_add(w[i - 7])
                .wrapping_add(s1);
        }

        let [mut a, mut b, mut c, mut d, mut e, mut f, mut g, mut h] = state;
        for i in 0..64 {
            let s1 = e.rotate_right(6) ^ e.rotate_right(11) ^ e.rotate_right(25);
            let ch = (e & f) ^ ((!e) & g);
            let temp1 = h
                .wrapping_add(s1)
                .wrapping_add(ch)
                .wrapping_add(K[i])
                .wrapping_add(w[i]);
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

        let vals = [a, b, c, d, e, f, g, h];
        for (slot, delta) in state.iter_mut().zip(vals) {
            *slot = slot.wrapping_add(delta);
        }
    }

    let mut hex = String::with_capacity(64);
    for word in state {
        for byte in word.to_be_bytes() {
            use std::fmt::Write;
            let _ = write!(&mut hex, "{byte:02x}");
        }
    }
    hex
}

fn push_lp(buf: &mut String, field: &str) {
    use std::fmt::Write;
    let _ = write!(buf, "{}:{field};", field.len());
}

/// Bộ sinh giả ngẫu nhiên tất định 64-bit SplitMix64 (không phụ thuộc OS hay đồng hồ hệ thống).
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct SplitMix64 {
    /// Trạng thái thanh ghi 64-bit hiện tại.
    pub state: u64,
}

impl SplitMix64 {
    /// Khởi tạo bộ sinh từ seed cố định.
    pub const fn new(seed: u64) -> Self {
        Self { state: seed }
    }

    /// Sinh giá trị 64-bit kế tiếp và cập nhật trạng thái.
    pub fn next_u64(&mut self) -> u64 {
        self.state = self.state.wrapping_add(0x9E37_79B9_7F4A_7C15);
        let mut z = self.state;
        z = (z ^ (z >> 30)).wrapping_mul(0xBF58_476D_1CE4_E5B9);
        z = (z ^ (z >> 27)).wrapping_mul(0x94D0_49BB_1331_11EB);
        z ^ (z >> 31)
    }

    /// Sinh phần hậu tố định danh 8 ký tự hex ổn định theo chuỗi pha.
    pub fn next_tag(&mut self) -> String {
        format!("{:08x}", (self.next_u64() & 0xFFFF_FFFF) as u32)
    }
}

// ============================================================================
// 2. LỖI & CÁC HÀM KIỂM TRA HỢP LỆ (RFC 2606 / RFC 1918 / ISO 8601 / REDACTION)
// ============================================================================

/// Lỗi phát sinh từ bên trong một `PentestModule`.
#[derive(Debug, Clone, PartialEq, Eq)]
pub enum ModuleError {
    /// Mục tiêu nằm ngoài phạm vi hoặc rơi vào danh sách loại trừ.
    OutOfScope(String),
    /// Thiếu phê duyệt cần thiết (ví dụ chưa có chấp thuận bên thứ 3 hoặc stage).
    ApprovalRequired(String),
    /// Ngân sách tài nguyên không đủ cho bước hiện tại.
    BudgetExceeded(String),
    /// Hành vi vi phạm danh mục cấm tuyệt đối (ví dụ cloud, socket thật, ghi đè destructive).
    ProhibitedAction(ProhibitedActionKind),
    /// Phát hiện chuỗi bí mật hoặc PII thô chưa được che.
    RawSecretDetected(String),
    /// Dữ liệu input không khớp với stage hoặc sai định dạng tổng hợp.
    InvalidInput(String),
    /// Trả lỗi tạm thời có thể phục hồi bằng `Checkpoint` / `resume`.
    TransientFailure(String),
    /// Cleanup hậu khai thác chưa đưa tài nguyên phòng lab về 0.
    CleanupIncomplete(String),
    /// Bất biến nội bộ của module bị vi phạm.
    InvariantViolation(String),
}

impl fmt::Display for ModuleError {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            Self::OutOfScope(msg) => write!(f, "ngoài phạm vi: {msg}"),
            Self::ApprovalRequired(msg) => write!(f, "thiếu phê duyệt: {msg}"),
            Self::BudgetExceeded(msg) => write!(f, "vượt ngân sách: {msg}"),
            Self::ProhibitedAction(kind) => write!(f, "hành động bị cấm tuyệt đối: {kind:?}"),
            Self::RawSecretDetected(msg) => write!(f, "phát hiện secret/PII thô: {msg}"),
            Self::InvalidInput(msg) => write!(f, "đầu vào không hợp lệ: {msg}"),
            Self::TransientFailure(msg) => write!(f, "lỗi tạm thời có thể phục hồi: {msg}"),
            Self::CleanupIncomplete(msg) => write!(f, "cleanup chưa sạch: {msg}"),
            Self::InvariantViolation(msg) => write!(f, "vi phạm bất biến module: {msg}"),
        }
    }
}

impl std::error::Error for ModuleError {}

/// Lỗi điều phối cấp `CapstoneOrchestrator` & `CapstoneEvaluator`.
#[derive(Debug, Clone, PartialEq, Eq)]
pub enum OrchestratorError {
    /// Vi phạm phạm vi (cổng preflight chặn trước module đầu tiên hoặc chặn trong quá trình chạy).
    ScopeViolation(String),
    /// Thiếu văn bản ủy quyền bằng chữ ký.
    AuthorizationMissing(String),
    /// Thời điểm chạy nằm ngoài cửa sổ UTC được duyệt.
    OutsideEngagementWindow(String),
    /// Thiếu phê duyệt HITL cho pha hoặc bên thứ ba.
    ApprovalRequired(String),
    /// Công tắc dừng khẩn cấp đang bật.
    KillSwitchEngaged(String),
    /// Vượt trần ngân sách module/tool/cost/time.
    BudgetExceeded(String),
    /// Phát hiện hành động bị cấm tuyệt đối.
    ProhibitedAction(ProhibitedActionKind),
    /// Chuyển pha trái đồ thị hợp lệ.
    InvalidStageTransition { from: Stage, to: Stage },
    /// Không tìm thấy module hoặc sai tương thích kiểu Input/Output.
    IncompatibleModuleOrInput(String),
    /// Phát hiện gắn trạng thái Verified nhưng thiếu `evidence_id` hoặc trỏ sai bằng chứng.
    UnsupportedClaim(String),
    /// Phát hiện secret/PII thô trong dữ liệu đầu vào, bằng chứng hoặc báo cáo.
    RawSecretDetected(String),
    /// Chuỗi băm `EvidenceLedger` bị sửa đổi trái phép hoặc trùng ID.
    EvidenceTampered(String),
    /// `OrchestratorCheckpoint` sai phiên bản, bị sửa MAC hoặc lệch kịch bản.
    CheckpointInvalid(String),
    /// Module kết thúc bằng lỗi (kể cả lỗi tạm thời khi gọi `run_strict`).
    ModuleFailed { stage: Stage, error: ModuleError },
    /// Vượt giới hạn số bước điều phối hoặc xuất hiện chu trình lặp.
    CycleOrStepLimitExceeded(String),
    /// Chưa xác minh sạch sau nhánh hậu khai thác trước khi xuất báo cáo.
    CleanupIncomplete(String),
    /// Báo cáo sai schema hoặc bị chỉnh sửa sau khi đóng dấu SHA-256.
    ReportTampered(String),
    /// Bảng trọng số phạt RL cấu hình sai (âm, NaN hoặc vô cực).
    RewardConfigInvalid(String),
    /// Đánh giá tiêu chí thất bại.
    EvaluationFailed(String),
}

impl fmt::Display for OrchestratorError {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            Self::ScopeViolation(msg) => write!(f, "vi phạm phạm vi: {msg}"),
            Self::AuthorizationMissing(msg) => write!(f, "thiếu ủy quyền hợp pháp: {msg}"),
            Self::OutsideEngagementWindow(msg) => write!(f, "ngoài khung giờ UTC: {msg}"),
            Self::ApprovalRequired(msg) => write!(f, "cần phê duyệt con người: {msg}"),
            Self::KillSwitchEngaged(msg) => write!(f, "kill switch đã kích hoạt: {msg}"),
            Self::BudgetExceeded(msg) => write!(f, "vượt ngân sách: {msg}"),
            Self::ProhibitedAction(kind) => write!(f, "hành động bị cấm: {kind:?}"),
            Self::InvalidStageTransition { from, to } => {
                write!(f, "chuyển pha không hợp lệ: {from:?} -> {to:?}")
            }
            Self::IncompatibleModuleOrInput(msg) => {
                write!(f, "không tương thích module/input: {msg}")
            }
            Self::UnsupportedClaim(msg) => write!(f, "phát hiện thiếu bằng chứng: {msg}"),
            Self::RawSecretDetected(msg) => write!(f, "phát hiện secret/PII thô: {msg}"),
            Self::EvidenceTampered(msg) => write!(f, "chuỗi bằng chứng bị giả mạo: {msg}"),
            Self::CheckpointInvalid(msg) => write!(f, "checkpoint không hợp lệ: {msg}"),
            Self::ModuleFailed { stage, error } => {
                write!(f, "module ở pha {stage:?} thất bại: {error}")
            }
            Self::CycleOrStepLimitExceeded(msg) => write!(f, "vượt giới hạn bước/chu trình: {msg}"),
            Self::CleanupIncomplete(msg) => write!(f, "cleanup chưa sạch: {msg}"),
            Self::ReportTampered(msg) => write!(f, "báo cáo bị sửa lén/sai schema: {msg}"),
            Self::RewardConfigInvalid(msg) => write!(f, "cấu hình phạt RL sai: {msg}"),
            Self::EvaluationFailed(msg) => write!(f, "đánh giá đạt chuẩn thất bại: {msg}"),
        }
    }
}

impl std::error::Error for OrchestratorError {}

impl From<ModuleError> for OrchestratorError {
    fn from(err: ModuleError) -> Self {
        match err {
            ModuleError::OutOfScope(msg) => Self::ScopeViolation(msg),
            ModuleError::ApprovalRequired(msg) => Self::ApprovalRequired(msg),
            ModuleError::BudgetExceeded(msg) => Self::BudgetExceeded(msg),
            ModuleError::ProhibitedAction(kind) => Self::ProhibitedAction(kind),
            ModuleError::RawSecretDetected(msg) => Self::RawSecretDetected(msg),
            ModuleError::CleanupIncomplete(msg) => Self::CleanupIncomplete(msg),
            other => Self::IncompatibleModuleOrInput(other.to_string()),
        }
    }
}

/// Kiểm tra tên miền tổng hợp chuẩn RFC 2606 (`*.example`), chữ thường nghiêm ngặt.
pub fn validate_example_domain(raw: &str) -> Result<String, ModuleError> {
    let domain = raw.trim().to_ascii_lowercase();
    if domain.is_empty() || domain.len() > 253 {
        return Err(ModuleError::InvalidInput(format!(
            "tên miền `{raw}` sai độ dài"
        )));
    }
    if !(domain == "example" || domain.ends_with(".example")) {
        return Err(ModuleError::OutOfScope(format!(
            "chỉ chấp nhận tên miền dành riêng RFC 2606 `.example`, nhận `{raw}`"
        )));
    }
    let labels: Vec<&str> = domain.split('.').collect();
    if labels.len() < 2 {
        return Err(ModuleError::InvalidInput(format!(
            "tên miền `{raw}` thiếu nhãn con"
        )));
    }
    for label in labels {
        if label.is_empty() || label.len() > 63 {
            return Err(ModuleError::InvalidInput(format!(
                "nhãn tên miền rỗng hoặc quá dài trong `{raw}`"
            )));
        }
        if label.starts_with('-')
            || label.ends_with('-')
            || !label
                .bytes()
                .all(|b| b.is_ascii_lowercase() || b.is_ascii_digit() || b == b'-')
        {
            return Err(ModuleError::InvalidInput(format!(
                "nhãn tên miền chứa ký tự không hợp lệ trong `{raw}`"
            )));
        }
    }
    Ok(domain)
}

/// Phân tích địa chỉ IPv4 nghiêm ngặt (chặn leading zero như `010.0.0.1`).
pub fn parse_strict_ipv4(raw: &str) -> Result<u32, ModuleError> {
    let trimmed = raw.trim();
    let parts: Vec<&str> = trimmed.split('.').collect();
    if parts.len() != 4 {
        return Err(ModuleError::InvalidInput(format!(
            "IPv4 `{raw}` phải gồm 4 octet"
        )));
    }
    let mut acc = 0u32;
    for part in parts {
        if part.is_empty()
            || (part.len() > 1 && part.starts_with('0'))
            || !part.bytes().all(|b| b.is_ascii_digit())
        {
            return Err(ModuleError::InvalidInput(format!(
                "octet IPv4 không hợp lệ trong `{raw}`"
            )));
        }
        let octet: u8 = part.parse().map_err(|_| {
            ModuleError::InvalidInput(format!("octet vượt ngưỡng 0..=255 trong `{raw}`"))
        })?;
        acc = (acc << 8) | u32::from(octet);
    }
    Ok(acc)
}

/// Kiểm tra một địa chỉ IPv4 dạng u32 có nằm trong 3 dải riêng RFC 1918 không.
pub fn is_rfc1918_u32(ip: u32) -> bool {
    let o0 = ((ip >> 24) & 0xFF) as u8;
    let o1 = ((ip >> 16) & 0xFF) as u8;
    o0 == 10 || (o0 == 172 && (16..=31).contains(&o1)) || (o0 == 192 && o1 == 168)
}

/// Phân tích CIDR RFC 1918 nghiêm ngặt (chặn prefix 0 và bit host khác 0).
pub fn parse_strict_rfc1918_cidr(cidr: &str) -> Result<(u32, u32), ModuleError> {
    let (ip_str, prefix_str) = cidr
        .trim()
        .split_once('/')
        .ok_or_else(|| ModuleError::InvalidInput(format!("CIDR thiếu `/`: `{cidr}`")))?;
    let ip = parse_strict_ipv4(ip_str)?;
    if prefix_str.is_empty()
        || (prefix_str.len() > 1 && prefix_str.starts_with('0'))
        || !prefix_str.bytes().all(|b| b.is_ascii_digit())
    {
        return Err(ModuleError::InvalidInput(format!(
            "tiền tố CIDR không hợp lệ: `{cidr}`"
        )));
    }
    let prefix: u32 = prefix_str
        .parse()
        .map_err(|_| ModuleError::InvalidInput(format!("tiền tố CIDR vượt giới hạn: `{cidr}`")))?;
    if prefix == 0 || prefix > 32 {
        return Err(ModuleError::InvalidInput(format!(
            "tiền tố CIDR phải trong 1..=32: `{cidr}`"
        )));
    }
    let mask = u32::MAX << (32 - prefix);
    if (ip & !mask) != 0 {
        return Err(ModuleError::InvalidInput(format!(
            "CIDR `{cidr}` có bit host khác 0"
        )));
    }
    let broadcast = ip | !mask;
    if !is_rfc1918_u32(ip) || !is_rfc1918_u32(broadcast) {
        return Err(ModuleError::OutOfScope(format!(
            "CIDR `{cidr}` không nằm trọn trong RFC 1918"
        )));
    }
    Ok((ip, mask))
}

/// Phân tích dấu thời gian ISO-8601 UTC (`YYYY-MM-DDTHH:MM:SSZ`) nghiêm ngặt, có xét năm nhuận.
pub fn parse_utc_timestamp(ts: &str) -> Result<(u16, u8, u8, u8, u8, u8), ModuleError> {
    let b = ts.as_bytes();
    if b.len() != 20
        || b[4] != b'-'
        || b[7] != b'-'
        || b[10] != b'T'
        || b[13] != b':'
        || b[16] != b':'
        || b[19] != b'Z'
    {
        return Err(ModuleError::InvalidInput(format!(
            "timestamp phải theo chuẩn YYYY-MM-DDTHH:MM:SSZ, nhận `{ts}`"
        )));
    }
    let num = |start: usize, end: usize| -> Result<u16, ModuleError> {
        let s = &ts[start..end];
        if !s.bytes().all(|c| c.is_ascii_digit()) {
            return Err(ModuleError::InvalidInput(format!(
                "timestamp chứa ký tự không phải số: `{ts}`"
            )));
        }
        s.parse()
            .map_err(|_| ModuleError::InvalidInput(format!("không thể phân tích số trong `{ts}`")))
    };
    let year = num(0, 4)?;
    let month = num(5, 7)? as u8;
    let day = num(8, 10)? as u8;
    let hour = num(11, 13)? as u8;
    let minute = num(14, 16)? as u8;
    let second = num(17, 19)? as u8;
    if !(2000..=2100).contains(&year)
        || !(1..=12).contains(&month)
        || hour > 23
        || minute > 59
        || second > 59
    {
        return Err(ModuleError::InvalidInput(format!(
            "giá trị ngày giờ ngoài phạm vi hợp lệ: `{ts}`"
        )));
    }
    let is_leap = year.is_multiple_of(4) && (!year.is_multiple_of(100) || year.is_multiple_of(400));
    let max_day = match month {
        1 | 3 | 5 | 7 | 8 | 10 | 12 => 31,
        4 | 6 | 9 | 11 => 30,
        2 if is_leap => 29,
        2 => 28,
        _ => 0,
    };
    if day == 0 || day > max_day {
        return Err(ModuleError::InvalidInput(format!(
            "ngày {day} không hợp lệ cho {year}-{month:02} trong `{ts}`"
        )));
    }
    Ok((year, month, day, hour, minute, second))
}

/// Kiểm tra URI vị trí bằng chứng chỉ dùng scheme ảo tổng hợp (`virtual://` hoặc `lab-sensor://`)
/// và host bắt buộc là tên miền `.example` (RFC 2606) hoặc IP RFC 1918.
pub fn validate_synthetic_location(uri: &str) -> Result<String, ModuleError> {
    let trimmed = uri.trim();
    if trimmed.is_empty() || trimmed.bytes().any(|b| b.is_ascii_control() || b == b' ') {
        return Err(ModuleError::InvalidInput(format!(
            "URI vị trí bằng chứng chứa ký tự điều khiển hoặc khoảng trắng: `{uri}`"
        )));
    }
    if !(trimmed.starts_with("virtual://") || trimmed.starts_with("lab-sensor://")) {
        return Err(ModuleError::OutOfScope(format!(
            "vị trí bằng chứng phải dùng virtual:// hoặc lab-sensor://, nhận `{uri}`"
        )));
    }
    let body = trimmed
        .strip_prefix("virtual://")
        .or_else(|| trimmed.strip_prefix("lab-sensor://"))
        .unwrap_or("");
    let Some((host, path)) = body.split_once('/') else {
        return Err(ModuleError::InvalidInput(format!(
            "URI tổng hợp phải có dạng scheme://host/path: `{uri}`"
        )));
    };
    if path.is_empty() {
        return Err(ModuleError::InvalidInput(format!(
            "URI tổng hợp thiếu phần path sau host: `{uri}`"
        )));
    }
    let valid_host =
        validate_example_domain(host).is_ok() || parse_strict_ipv4(host).is_ok_and(is_rfc1918_u32);
    if !valid_host {
        return Err(ModuleError::OutOfScope(format!(
            "host `{host}` trong URI bằng chứng phải thuộc `.example` hoặc RFC 1918"
        )));
    }
    Ok(trimmed.to_string())
}

/// Kiểm tra URI tổng hợp và xác nhận host bên trong URI nằm trọn trong `EngagementScope`.
pub fn validate_synthetic_location_in_scope(
    uri: &str,
    scope: &EngagementScope,
    at_utc: &str,
) -> Result<String, ModuleError> {
    let clean = validate_synthetic_location(uri)?;
    let body = clean
        .strip_prefix("virtual://")
        .or_else(|| clean.strip_prefix("lab-sensor://"))
        .unwrap_or("");
    let host = body.split('/').next().unwrap_or("");
    if parse_strict_ipv4(host).is_ok() {
        scope.check_ip(host)?;
    } else {
        scope.check_domain(host, at_utc)?;
    }
    Ok(clean)
}

fn is_token_ident_byte(b: u8) -> bool {
    b.is_ascii_alphanumeric() || matches!(b, b'_' | b'-' | b'.' | b'/' | b'+' | b'=')
}

fn is_jwt_ident_byte(b: u8) -> bool {
    b.is_ascii_alphanumeric() || matches!(b, b'_' | b'-' | b'.' | b'=')
}

fn extract_secret_value_span(raw: &str, mut val_start: usize) -> (usize, usize, bool) {
    let b = raw.as_bytes();
    while val_start < b.len() && b[val_start].is_ascii_whitespace() {
        val_start += 1;
    }
    if val_start >= b.len() {
        return (val_start, val_start, false);
    }
    if b[val_start] == b'"' || b[val_start] == b'\'' {
        let quote = b[val_start];
        let mut idx = val_start + 1;
        while idx < b.len() {
            if b[idx] == b'\\' && idx + 1 < b.len() {
                idx += 2;
                continue;
            }
            if b[idx] == quote {
                return (val_start, idx + 1, true);
            }
            idx += 1;
        }
        return (val_start, b.len(), false);
    }
    let mut end = val_start;
    while end < b.len()
        && !b[end].is_ascii_whitespace()
        && !matches!(b[end], b',' | b';' | b')' | b'}')
    {
        end += 1;
    }
    (val_start, end, false)
}

fn is_redacted_marker_exact(slice: &str, was_quoted: bool) -> bool {
    let sb = slice.as_bytes();
    let inner = if was_quoted
        && sb.len() >= 2
        && sb[0] == sb[sb.len() - 1]
        && matches!(sb[0], b'"' | b'\'')
    {
        &slice[1..slice.len() - 1]
    } else {
        slice
    };
    inner.trim() == REDACTED_SECRET_MARKER
}

fn find_subslice(haystack: &[u8], needle: &[u8]) -> Option<usize> {
    if needle.is_empty() {
        return Some(0);
    }
    haystack.windows(needle.len()).position(|w| w == needle)
}

fn find_kv_secret_spans(raw: &str) -> Vec<(usize, usize)> {
    const KEYS: &[&str] = &[
        "access_token",
        "api_key",
        "apikey",
        "auth_token",
        "aws_key",
        "aws_secret",
        "bearer_token",
        "client_secret",
        "id_token",
        "jwt",
        "ntlm_hash",
        "passphrase",
        "passwd",
        "password",
        "private_key",
        "pwd",
        "refresh_token",
        "secret",
        "session_token",
        "token",
        "wpa_psk",
    ];
    let lower = raw.to_ascii_lowercase();
    let b = lower.as_bytes();
    let mut spans = Vec::new();
    let mut i = 0;

    while i < b.len() {
        // Bỏ qua tiền tố auth scheme "Bearer " / "Basic " (thao tác thuần byte, an toàn UTF-8)
        let mut advanced_scheme = false;
        for scheme in ["bearer", "basic"] {
            let sb = scheme.as_bytes();
            if b[i..].starts_with(sb) {
                let prev_ok = i == 0 || !is_token_ident_byte(b[i - 1]);
                let after = i + sb.len();
                let next_ok = after < b.len() && b[after].is_ascii_whitespace();
                if prev_ok && next_ok {
                    let (vs, ve, quoted) = extract_secret_value_span(raw, after);
                    if ve > vs
                        && raw.is_char_boundary(vs)
                        && raw.is_char_boundary(ve)
                        && !is_redacted_marker_exact(&raw[vs..ve], quoted)
                    {
                        spans.push((vs, ve));
                        i = ve;
                        advanced_scheme = true;
                        break;
                    }
                }
            }
        }
        if advanced_scheme {
            continue;
        }

        let mut matched_key_end = None;
        for key in KEYS {
            let kb = key.as_bytes();
            if b[i..].starts_with(kb) {
                // Cho phép snake_case ghép như db_password=, my_secret=, aws_key=
                let prev_ok = i == 0 || !b[i - 1].is_ascii_alphanumeric();
                if prev_ok {
                    let after = i + kb.len();
                    let next_quote = after < b.len() && (b[after] == b'"' || b[after] == b'\'');
                    let next_sep = after < b.len() && (b[after] == b'=' || b[after] == b':');
                    let next_ws = after < b.len() && b[after].is_ascii_whitespace();
                    if next_quote || next_sep || next_ws {
                        matched_key_end = Some(after);
                        break;
                    }
                }
            }
        }

        if let Some(mut k_end) = matched_key_end {
            if k_end < b.len() && (b[k_end] == b'"' || b[k_end] == b'\'') {
                k_end += 1;
            }
            while k_end < b.len() && b[k_end].is_ascii_whitespace() {
                k_end += 1;
            }
            if k_end < b.len() && (b[k_end] == b'=' || b[k_end] == b':') {
                k_end += 1;
                let (vs, ve, quoted) = extract_secret_value_span(raw, k_end);
                if ve > vs
                    && raw.is_char_boundary(vs)
                    && raw.is_char_boundary(ve)
                    && !is_redacted_marker_exact(&raw[vs..ve], quoted)
                {
                    spans.push((vs, ve));
                    i = ve;
                    continue;
                }
            }
        }
        i += 1;
    }
    spans
}

fn find_pattern_secret_spans(raw: &str) -> Vec<(usize, usize)> {
    let lower = raw.to_ascii_lowercase();
    let lb = lower.as_bytes();
    let rb = raw.as_bytes();
    let mut spans = Vec::new();

    // 1. Khối PEM private key (hỗ trợ nhiều khối và phát hiện khối cắt cụt, an toàn UTF-8)
    let mut scan_from = 0usize;
    while scan_from < lb.len()
        && let Some(rel_start) = find_subslice(&lb[scan_from..], b"-----begin ")
    {
        let start = scan_from + rel_start;
        if let Some(rel_nl_end) = find_subslice(&lb[start..], b"private key-----") {
            let header_end = start + rel_nl_end + b"private key-----".len();
            if let Some(rel_footer) = find_subslice(&lb[header_end..], b"-----end ") {
                let f_start = header_end + rel_footer;
                if let Some(rel_f_end) = find_subslice(&lb[f_start..], b"private key-----") {
                    let end = f_start + rel_f_end + b"private key-----".len();
                    spans.push((start, end));
                    scan_from = end;
                    continue;
                }
            }
            spans.push((start, raw.len()));
            break;
        }
        scan_from = start + b"-----begin ".len();
    }

    // 2. Token tiền tố đặc trưng & JWT (eyJ...) — chấp nhận `=` hoặc `/` đứng ngay trước
    for prefix in [
        "syn_tok_", "ghp_", "glpat-", "xoxb-", "sk_live_", "sk_test_", "akia", "eyj",
    ] {
        let pb = prefix.as_bytes();
        let mut from = 0;
        while from < lb.len()
            && let Some(rel) = find_subslice(&lb[from..], pb)
        {
            let start = from + rel;
            let prev_ok =
                start == 0 || (!lb[start - 1].is_ascii_alphanumeric() && lb[start - 1] != b'_');
            if prev_ok {
                let mut end = start + pb.len();
                while end < rb.len()
                    && if prefix == "eyj" {
                        is_jwt_ident_byte(rb[end])
                    } else {
                        is_token_ident_byte(rb[end])
                    }
                {
                    end += 1;
                }
                if raw.is_char_boundary(start) && raw.is_char_boundary(end) {
                    let tok = &raw[start..end];
                    let is_jwt = prefix == "eyj" && tok.split('.').count() == 3 && tok.len() >= 18;
                    let is_prefixed = prefix != "eyj" && tok.len() > prefix.len() + 3;
                    if is_jwt || is_prefixed {
                        spans.push((start, end));
                        from = end;
                        continue;
                    }
                }
            }
            from = start + pb.len();
        }
    }

    spans
}

fn find_non_example_email_spans(raw: &str) -> Vec<(usize, usize)> {
    let b = raw.as_bytes();
    let mut spans = Vec::new();
    let mut i = 0;
    while i < b.len() {
        if b[i] == b'@' {
            let mut left = i;
            while left > 0
                && (b[left - 1].is_ascii_alphanumeric()
                    || matches!(b[left - 1], b'.' | b'_' | b'+' | b'-'))
            {
                left -= 1;
            }
            let mut right = i + 1;
            while right < b.len()
                && (b[right].is_ascii_alphanumeric() || matches!(b[right], b'.' | b'-'))
            {
                right += 1;
            }
            while right > i + 1 && b[right - 1] == b'.' {
                right -= 1;
            }
            let domain = raw[i + 1..right].to_ascii_lowercase();
            if left < i
                && domain.contains('.')
                && domain != "example"
                && !domain.ends_with(".example")
            {
                spans.push((left, right));
                i = right;
                continue;
            }
        }
        i += 1;
    }
    spans
}

fn merge_spans(mut spans: Vec<(usize, usize)>) -> Vec<(usize, usize)> {
    if spans.is_empty() {
        return spans;
    }
    spans.sort_unstable();
    let mut merged = Vec::with_capacity(spans.len());
    let mut cur = spans[0];
    for (s, e) in spans.into_iter().skip(1) {
        if s <= cur.1 {
            cur.1 = cur.1.max(e);
        } else {
            merged.push(cur);
            cur = (s, e);
        }
    }
    merged.push(cur);
    merged
}

/// Tự động che toàn bộ chuỗi secret/token/PEM và email ngoài `.example` ngay khi nạp dữ liệu.
pub fn redact_sensitive_text(raw: &str) -> (String, usize) {
    let mut tagged: Vec<(usize, usize, &'static str)> = Vec::new();
    for (s, e) in merge_spans(
        find_kv_secret_spans(raw)
            .into_iter()
            .chain(find_pattern_secret_spans(raw))
            .collect(),
    ) {
        tagged.push((s, e, REDACTED_SECRET_MARKER));
    }
    for (s, e) in merge_spans(find_non_example_email_spans(raw)) {
        if !tagged.iter().any(|(ts, te, _)| s < *te && e > *ts) {
            tagged.push((s, e, REDACTED_PII_MARKER));
        }
    }
    tagged.sort_by_key(|(s, _, _)| *s);
    if tagged.is_empty() {
        return (raw.to_string(), 0);
    }
    let mut out = String::with_capacity(raw.len());
    let mut cursor = 0;
    for (s, e, marker) in &tagged {
        if *s > cursor {
            out.push_str(&raw[cursor..*s]);
        }
        let quoted_prefix =
            raw.as_bytes()[*s] == b'"' && raw.as_bytes()[e.saturating_sub(1)] == b'"';
        if quoted_prefix {
            out.push('"');
            out.push_str(marker);
            out.push('"');
        } else {
            out.push_str(marker);
        }
        cursor = *e;
    }
    if cursor < raw.len() {
        out.push_str(&raw[cursor..]);
    }
    (out, tagged.len())
}

/// Kiểm tra một trường văn bản không chứa bất kỳ secret thô hay PII ngoài `.example`.
pub fn validate_no_raw_secret(field_name: &str, text: &str) -> Result<(), ModuleError> {
    if !find_kv_secret_spans(text).is_empty() || !find_pattern_secret_spans(text).is_empty() {
        return Err(ModuleError::RawSecretDetected(format!(
            "trường `{field_name}` chứa chuỗi bí mật hoặc khóa riêng chưa được che"
        )));
    }
    if !find_non_example_email_spans(text).is_empty() {
        return Err(ModuleError::RawSecretDetected(format!(
            "trường `{field_name}` chứa email cá nhân ngoài tên miền `.example`"
        )));
    }
    Ok(())
}

// ============================================================================
// 3. QUẢN TRỊ ENGAGEMENT, PHẠM VI, NGÂN SÁCH & GIAO DIỆN PENTESTMODULE
// ============================================================================

/// Các pha trong đồ thị điều phối chiến dịch tổng hợp.
#[derive(Debug, Clone, Copy, PartialEq, Eq, PartialOrd, Ord, Hash)]
pub enum Stage {
    /// Pha 1: OSINT thụ động trên nguồn `.example` & RFC 1918 giả.
    Osint,
    /// Pha nhánh tùy chọn: kiểm thử không dây OSWP tổng hợp.
    WirelessAssessment,
    /// Pha 2: Kiểm kê cấu trúc Active Directory tổng hợp.
    AdEnumeration,
    /// Pha 3 (có điều kiện): kiểm chứng precondition Roasting / Delegation khi pha AD Enum phát hiện ứng viên.
    AdConditionalValidation,
    /// Pha nhánh tùy chọn: kiểm kê hậu khai thác & dọn dẹp lũy đẳng.
    PostExploitationAudit,
    /// Pha 4: Tổng hợp, khử trùng lặp, đánh giá Red-vs-Blue / RL và xuất báo cáo tất định.
    Reporting,
    /// Trạng thái kết thúc sau khi hoàn tất báo cáo.
    Completed,
}

impl Stage {
    /// Mã định danh chuỗi chuẩn hóa cho log, checkpoint và báo cáo.
    pub fn as_str(&self) -> &'static str {
        match self {
            Self::Osint => "OSINT",
            Self::WirelessAssessment => "WirelessAssessment",
            Self::AdEnumeration => "ADEnumeration",
            Self::AdConditionalValidation => "ADConditionalValidation",
            Self::PostExploitationAudit => "PostExploitationAudit",
            Self::Reporting => "Reporting",
            Self::Completed => "Completed",
        }
    }

    /// Kiểm tra tính hợp lệ của chuyển pha trong DAG điều phối.
    pub fn can_transition_to(self, next: Stage) -> bool {
        matches!(
            (self, next),
            (Self::Osint, Self::WirelessAssessment)
                | (Self::Osint, Self::AdEnumeration)
                | (Self::WirelessAssessment, Self::AdEnumeration)
                | (Self::AdEnumeration, Self::AdConditionalValidation)
                | (Self::AdEnumeration, Self::PostExploitationAudit)
                | (Self::AdEnumeration, Self::Reporting)
                | (Self::AdConditionalValidation, Self::PostExploitationAudit)
                | (Self::AdConditionalValidation, Self::Reporting)
                | (Self::PostExploitationAudit, Self::Reporting)
                | (Self::Reporting, Self::Completed)
        )
    }
}

impl fmt::Display for Stage {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        f.write_str(self.as_str())
    }
}

/// Phân loại hành động bị cấm tuyệt đối trong hệ thống phòng sạch.
#[derive(Debug, Clone, Copy, PartialEq, Eq, PartialOrd, Ord)]
pub enum ProhibitedActionKind {
    /// Cloud pentest nằm ngoài phạm vi Phần II (chỉ có để từ chối cứng).
    CloudControlPlaneOutOfScope,
    /// Mở socket mạng thật.
    RealNetworkSocket,
    /// Điều khiển card Wi-Fi / phát frame vật lý.
    HardwareRadioControl,
    /// Thu thập credential thật trên hệ điều hành.
    RawCredentialHarvest,
    /// Xóa/phá hủy dữ liệu mục tiêu hoặc cài persistence thật.
    DestructiveHostModification,
    /// Tự lan truyền sang máy khác ngoài kịch bản mô phỏng.
    AutonomousSelfPropagation,
}

/// Chấp thuận bằng văn bản cho phụ thuộc bên thứ ba.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct ThirdPartyConsent {
    /// Mã tài liệu chấp thuận của bên thứ ba.
    pub written_consent_ref: String,
    /// Hạn chót hiệu lực UTC ISO-8601.
    pub approved_until_utc: String,
}

/// Phạm vi kiểm thử: Deny luôn thắng Allow, bắt buộc `.example` và RFC 1918.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct EngagementScope {
    /// Tập tên miền chính được phép kiểm thử.
    pub allowed_domains: BTreeSet<String>,
    /// Tập tên miền cha được phép kiểm thử cả tên miền con.
    pub allow_subdomains_of: BTreeSet<String>,
    /// Danh sách loại trừ tuyệt đối (luôn thắng Allow).
    pub denied_domains: BTreeSet<String>,
    /// Dải mạng RFC 1918 được phép.
    pub allowed_cidrs: Vec<String>,
    /// Địa chỉ IP cụ thể bị cấm chạm tới (luôn thắng Allow).
    pub denied_ips: BTreeSet<String>,
    /// Danh mục phụ thuộc bên thứ ba bắt buộc phải có `ThirdPartyConsent`.
    pub third_party_assets: BTreeMap<String, ThirdPartyConsent>,
}

fn domain_matches_or_subdomain(candidate: &str, rule: &str) -> bool {
    candidate == rule
        || candidate
            .strip_suffix(rule)
            .is_some_and(|prefix| prefix.ends_with('.'))
}

impl EngagementScope {
    /// Kiểm tra toàn bộ cấu trúc cấu hình phạm vi trước khi chạy module đầu tiên.
    pub fn validate(&self) -> Result<(), ModuleError> {
        if self.allowed_domains.is_empty() && self.allow_subdomains_of.is_empty() {
            return Err(ModuleError::OutOfScope(
                "phạm vi phải có ít nhất một tên miền `.example` được phép".to_string(),
            ));
        }
        if self.allowed_cidrs.is_empty() {
            return Err(ModuleError::OutOfScope(
                "phạm vi phải có ít nhất một dải CIDR RFC 1918".to_string(),
            ));
        }
        for d in self
            .allowed_domains
            .iter()
            .chain(self.allow_subdomains_of.iter())
            .chain(self.denied_domains.iter())
            .chain(self.third_party_assets.keys())
        {
            validate_example_domain(d)?;
        }
        for cidr in &self.allowed_cidrs {
            parse_strict_rfc1918_cidr(cidr)?;
        }
        for ip in &self.denied_ips {
            let parsed = parse_strict_ipv4(ip)?;
            if !is_rfc1918_u32(parsed) {
                return Err(ModuleError::OutOfScope(format!(
                    "IP loại trừ `{ip}` phải thuộc RFC 1918"
                )));
            }
        }
        for consent in self.third_party_assets.values() {
            validate_no_raw_secret("third_party_consent", &consent.written_consent_ref)?;
            parse_utc_timestamp(&consent.approved_until_utc)?;
        }
        Ok(())
    }

    /// Kiểm tra quyền kiểm thử một tên miền tại thời điểm UTC cụ thể (Deny > ThirdParty > Allow).
    pub fn check_domain(&self, raw_domain: &str, at_utc: &str) -> Result<String, ModuleError> {
        let domain = validate_example_domain(raw_domain)?;
        let now = parse_utc_timestamp(at_utc)?;

        if self
            .denied_domains
            .iter()
            .any(|denied| domain_matches_or_subdomain(&domain, denied))
        {
            return Err(ModuleError::OutOfScope(format!(
                "tên miền `{domain}` nằm trong danh sách loại trừ (Deny wins)"
            )));
        }

        for (asset_domain, tp) in &self.third_party_assets {
            if domain_matches_or_subdomain(&domain, asset_domain) {
                if tp.written_consent_ref.trim().is_empty() {
                    return Err(ModuleError::ApprovalRequired(format!(
                        "tài sản bên thứ ba `{domain}` (theo `{asset_domain}`) thiếu mã chấp thuận"
                    )));
                }
                let until = parse_utc_timestamp(&tp.approved_until_utc)?;
                if now > until {
                    return Err(ModuleError::ApprovalRequired(format!(
                        "chấp thuận bên thứ ba cho `{domain}` (theo `{asset_domain}`) đã hết hạn lúc {}",
                        tp.approved_until_utc
                    )));
                }
            }
        }

        let allowed = self.allowed_domains.contains(&domain)
            || self
                .allow_subdomains_of
                .iter()
                .any(|parent| domain_matches_or_subdomain(&domain, parent));
        if !allowed {
            return Err(ModuleError::OutOfScope(format!(
                "tên miền `{domain}` không nằm trong allowlist phạm vi"
            )));
        }
        Ok(domain)
    }

    /// Kiểm tra quyền kiểm thử một IPv4 RFC 1918 (Deny > Allow).
    pub fn check_ip(&self, raw_ip: &str) -> Result<String, ModuleError> {
        let ip = parse_strict_ipv4(raw_ip)?;
        if !is_rfc1918_u32(ip) {
            return Err(ModuleError::OutOfScope(format!(
                "IP `{raw_ip}` nằm ngoài RFC 1918 — cấm tuyệt đối IP công cộng"
            )));
        }
        let normalized = raw_ip.trim().to_string();
        for denied in &self.denied_ips {
            if parse_strict_ipv4(denied).is_ok_and(|d| d == ip) {
                return Err(ModuleError::OutOfScope(format!(
                    "IP `{normalized}` nằm trong denied_ips (Deny wins)"
                )));
            }
        }
        for cidr in &self.allowed_cidrs {
            let (net, mask) = parse_strict_rfc1918_cidr(cidr)?;
            if (ip & mask) == net {
                return Ok(normalized);
            }
        }
        Err(ModuleError::OutOfScope(format!(
            "IP `{normalized}` không thuộc dải allowed_cidrs"
        )))
    }
}

/// Bản ghi phê duyệt của người vận hành (HITL) cho từng pha hoặc mục tiêu nhạy cảm.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct ApprovalGrant {
    /// Mã phê duyệt duy nhất.
    pub grant_id: String,
    /// Pha được phê duyệt.
    pub stage: Stage,
    /// Vai trò người phê duyệt (không chứa PII thật).
    pub approved_by_role: String,
    /// Mục tiêu phạm vi gắn kèm phê duyệt.
    pub target_scope: String,
    /// Hạn chót hiệu lực UTC.
    pub expires_at_utc: String,
}

/// Trần ngân sách tài nguyên cho toàn bộ chiến dịch capstone.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct ResourceBudget {
    /// Số lần chạy module tối đa.
    pub max_modules: usize,
    /// Tổng số lần gọi tool giả lập tối đa.
    pub max_tool_calls: u32,
    /// Chi phí đơn vị tối đa.
    pub max_cost_units: u32,
    /// Ngân sách thời gian ảo (ms) tối đa.
    pub max_time_ms: u64,
}

/// Mức tiêu thụ tài nguyên cộng dồn.
#[derive(Debug, Clone, Copy, Default, PartialEq, Eq)]
pub struct ResourceUsage {
    /// Số pha/module đã chạy.
    pub modules_executed: usize,
    /// Số lần gọi tool giả lập.
    pub tool_calls: u32,
    /// Chi phí đơn vị đã dùng.
    pub cost_units: u32,
    /// Thời gian ảo tích lũy (ms).
    pub time_ms: u64,
}

impl ResourceUsage {
    /// Cộng dồn mức tiêu thụ kèm kiểm tra tràn số học và vượt ngân sách.
    pub fn checked_add(&self, delta: &Self, budget: &ResourceBudget) -> Result<Self, ModuleError> {
        let modules_executed = self
            .modules_executed
            .checked_add(delta.modules_executed)
            .ok_or_else(|| ModuleError::BudgetExceeded("tràn bộ đếm modules".to_string()))?;
        let tool_calls = self
            .tool_calls
            .checked_add(delta.tool_calls)
            .ok_or_else(|| ModuleError::BudgetExceeded("tràn bộ đếm tool_calls".to_string()))?;
        let cost_units = self
            .cost_units
            .checked_add(delta.cost_units)
            .ok_or_else(|| ModuleError::BudgetExceeded("tràn bộ đếm cost_units".to_string()))?;
        let time_ms = self
            .time_ms
            .checked_add(delta.time_ms)
            .ok_or_else(|| ModuleError::BudgetExceeded("tràn bộ đếm time_ms".to_string()))?;

        if modules_executed > budget.max_modules
            || tool_calls > budget.max_tool_calls
            || cost_units > budget.max_cost_units
            || time_ms > budget.max_time_ms
        {
            return Err(ModuleError::BudgetExceeded(format!(
                "vượt ngân sách (modules {modules_executed}/{}, tools {tool_calls}/{}, cost {cost_units}/{}, time {time_ms}/{}ms)",
                budget.max_modules,
                budget.max_tool_calls,
                budget.max_cost_units,
                budget.max_time_ms
            )));
        }

        Ok(Self {
            modules_executed,
            tool_calls,
            cost_units,
            time_ms,
        })
    }
}

/// Ngữ cảnh dùng chung cho mọi `PentestModule` trong chiến dịch.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct EngagementContext {
    /// Mã phiên chiến dịch.
    pub engagement_id: String,
    /// Số hiệu văn bản Rules of Engagement.
    pub authorization_ref: String,
    /// Xác nhận văn bản ủy quyền đã ký.
    pub signed_written_consent: bool,
    /// Cổng phê duyệt tổng cho phép kích hoạt chiến dịch tự hành.
    pub campaign_approved: bool,
    /// Công tắc dừng khẩn cấp toàn chiến dịch.
    pub kill_switch_engaged: bool,
    /// Mở đầu cửa sổ UTC ISO-8601.
    pub window_start_utc: String,
    /// Kết thúc cửa sổ UTC ISO-8601.
    pub window_end_utc: String,
    /// Mốc thời gian ảo hiện tại (tất định).
    pub current_virtual_utc: String,
    /// Seed tất định cho mô phỏng.
    pub seed: u64,
    /// Phạm vi mạng & tên miền được ủy quyền.
    pub scope: EngagementScope,
    /// Danh sách các pha bắt buộc phải có `ApprovalGrant` riêng trước khi chạy.
    pub required_stage_approvals: BTreeSet<Stage>,
    /// Các phê duyệt đã cấp.
    pub granted_approvals: Vec<ApprovalGrant>,
    /// Ngân sách tài nguyên tối đa.
    pub budget: ResourceBudget,
    /// Các hành động cấm được khai báo nhầm trong cấu hình (nếu có sẽ bị chặn tức thì).
    pub requested_prohibited_actions: Vec<ProhibitedActionKind>,
}

impl EngagementContext {
    /// Cổng kiểm tra an toàn preflight chạy **trước** khi module đầu tiên được chạm tới.
    pub fn preflight_validate(&self) -> Result<(), OrchestratorError> {
        validate_no_raw_secret("engagement_id", &self.engagement_id)?;
        validate_no_raw_secret("authorization_ref", &self.authorization_ref)?;

        if self.kill_switch_engaged {
            return Err(OrchestratorError::KillSwitchEngaged(
                "kill switch đang bật — dừng toàn chiến dịch trước khi chạy module".to_string(),
            ));
        }
        if !self.signed_written_consent || self.authorization_ref.trim().is_empty() {
            return Err(OrchestratorError::AuthorizationMissing(
                "thiếu văn bản ủy quyền có chữ ký (`signed_written_consent` / `authorization_ref`)"
                    .to_string(),
            ));
        }
        if !self.campaign_approved {
            return Err(OrchestratorError::ApprovalRequired(
                "chưa có phê duyệt khởi chạy chiến dịch (`campaign_approved == false`)".to_string(),
            ));
        }
        if let Some(prohibited) = self.requested_prohibited_actions.first() {
            return Err(OrchestratorError::ProhibitedAction(*prohibited));
        }

        let start = parse_utc_timestamp(&self.window_start_utc)?;
        let end = parse_utc_timestamp(&self.window_end_utc)?;
        let now = parse_utc_timestamp(&self.current_virtual_utc)?;
        if start >= end || now < start || now > end {
            return Err(OrchestratorError::OutsideEngagementWindow(format!(
                "thời điểm `{}` không nằm trong cửa sổ `[{} .. {}]`",
                self.current_virtual_utc, self.window_start_utc, self.window_end_utc
            )));
        }

        self.scope.validate()?;

        for approval in &self.granted_approvals {
            validate_no_raw_secret("approval.grant_id", &approval.grant_id)?;
            validate_no_raw_secret("approval.role", &approval.approved_by_role)?;
            validate_no_raw_secret("approval.scope", &approval.target_scope)?;
            validate_example_domain(&approval.target_scope)?;
            parse_utc_timestamp(&approval.expires_at_utc)?;
        }

        Ok(())
    }

    /// Kiểm tra một pha cụ thể có đủ `ApprovalGrant` còn hạn hay không.
    pub fn check_stage_approval(&self, stage: Stage) -> Result<(), ModuleError> {
        if !self.required_stage_approvals.contains(&stage) {
            return Ok(());
        }
        let now = parse_utc_timestamp(&self.current_virtual_utc)?;
        let ok = self.granted_approvals.iter().any(|grant| {
            grant.stage == stage
                && !grant.grant_id.trim().is_empty()
                && validate_example_domain(&grant.target_scope).is_ok()
                && parse_utc_timestamp(&grant.expires_at_utc).is_ok_and(|exp| now <= exp)
        });
        if !ok {
            return Err(ModuleError::ApprovalRequired(format!(
                "pha `{}` yêu cầu ApprovalGrant còn hiệu lực lúc {}",
                stage.as_str(),
                self.current_virtual_utc
            )));
        }
        Ok(())
    }

    /// Kiểm tra phê duyệt cho pha khớp đúng tên miền mục tiêu `target_domain`.
    pub fn check_stage_approval_for_target(
        &self,
        stage: Stage,
        target_domain: &str,
    ) -> Result<(), ModuleError> {
        if !self.required_stage_approvals.contains(&stage) {
            return Ok(());
        }
        let target = validate_example_domain(target_domain)?;
        let now = parse_utc_timestamp(&self.current_virtual_utc)?;
        let ok = self.granted_approvals.iter().any(|grant| {
            grant.stage == stage
                && !grant.grant_id.trim().is_empty()
                && validate_example_domain(&grant.target_scope)
                    .is_ok_and(|gs| domain_matches_or_subdomain(&target, &gs))
                && parse_utc_timestamp(&grant.expires_at_utc).is_ok_and(|exp| now <= exp)
        });
        if !ok {
            return Err(ModuleError::ApprovalRequired(format!(
                "pha `{}` thiếu ApprovalGrant hợp lệ cho mục tiêu `{target}` lúc {}",
                stage.as_str(),
                self.current_virtual_utc
            )));
        }
        Ok(())
    }
}

/// Nguồn OSINT thụ động tổng hợp (theo Chương 109, 100% offline).
#[derive(Debug, Clone, Copy, PartialEq, Eq, PartialOrd, Ord)]
pub enum PassiveSourceKind {
    /// Nhật ký chứng chỉ CT giả.
    CtLog,
    /// Bản ghi DNS/RDAP tổng hợp.
    PassiveDns,
    /// Kho lưu trữ mã công khai giả lập.
    PublicRepoMock,
}

impl PassiveSourceKind {
    /// Tên chuỗi của nguồn OSINT.
    pub fn as_str(&self) -> &'static str {
        match self {
            Self::CtLog => "ct-log",
            Self::PassiveDns => "passive-dns",
            Self::PublicRepoMock => "public-repo-mock",
        }
    }
}

/// Chuẩn bảo mật Wi-Fi tổng hợp (theo Chương 106, không mở radio thật).
#[derive(Debug, Clone, Copy, PartialEq, Eq, PartialOrd, Ord)]
pub enum WirelessSecuritySuite {
    /// Open không mã hóa.
    Open,
    /// WPA2-PSK không bật PMF.
    Wpa2PskNoPmf,
    /// WPA3-SAE có PMF bắt buộc.
    Wpa3SaePmfRequired,
}

/// Cạnh ACL giả lập trong Active Directory (theo Chương 107–108).
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct AdAclEdge {
    /// Principal nguồn.
    pub source_principal: String,
    /// Quyền ACL trừu tượng (`WriteDacl`, `GenericAll`, ...).
    pub right: String,
    /// Đối tượng đích trong domain `.example`.
    pub target_object: String,
}

/// Mục persistence tổng hợp trên snapshot `virtual://` (theo Chương 110).
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct SyntheticPersistenceItem {
    /// Mã định danh mục persistence.
    pub item_id: String,
    /// Đường dẫn vị trí ảo (`virtual://...`).
    pub virtual_location: String,
    /// Cơ chế trừu tượng (`ScheduledTask`, `RunKey`, ...).
    pub mechanism: String,
    /// Có nằm trong baseline sạch trước kiểm thử hay không.
    pub in_baseline: bool,
}

/// Thẻ loại đầu vào tương ứng cho mỗi pha kiểm tra compatibility trong `ModuleRegistry`.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum ModuleInputKind {
    /// Dữ liệu mồi OSINT.
    OsintSeed,
    /// Fixture đánh giá không dây.
    WirelessAssessment,
    /// Snapshot kiểm kê AD.
    AdEnumeration,
    /// Yêu cầu kiểm chứng AD có điều kiện.
    AdConditionalValidation,
    /// Snapshot kiểm kê hậu khai thác.
    PostExploitationAudit,
    /// Tổng hợp đóng gói báo cáo.
    Reporting,
}

/// Đầu vào định kiểu cho `OsintModuleAdapter`.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct OsintSeedInput {
    /// Tên miền gốc `.example`.
    pub root_domain: String,
    /// Mã tổ chức tổng hợp.
    pub org_handle: String,
    /// Địa chỉ IPv4 chính trong phạm vi RFC 1918.
    pub primary_ip: String,
    /// Các nguồn passive giả cần truy vấn.
    pub passive_sources: Vec<PassiveSourceKind>,
    /// Có rẽ sang nhánh không dây OSWP hay không.
    pub enable_wireless_branch: bool,
}

/// Đầu vào định kiểu cho `WirelessModuleAdapter`.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct WirelessAssessmentInput {
    /// SSID mục tiêu tổng hợp.
    pub ssid: String,
    /// Tên miền nội bộ đại diện cho địa điểm kiểm thử.
    pub site_domain: String,
    /// Địa chỉ IPv4 controller giả trong phạm vi.
    pub controller_ip: String,
    /// BSSID chính thức trong tài liệu kỹ thuật.
    pub expected_bssid: String,
    /// BSSID phát hiện trong capture giả thứ hai.
    pub observed_second_bssid: Option<String>,
    /// BSSID thứ hai có nằm trong danh mục AP hợp lệ được cấp phép hay không.
    pub second_bssid_authorized: bool,
    /// Cường độ SNR đo từ capture tổng hợp (dB).
    pub snr_db: i16,
    /// Bộ bảo mật của BSS nghi vấn.
    pub observed_suite: WirelessSecuritySuite,
    /// Capture giả có chứa bản ghi PMKID hay không.
    pub pmkid_present: bool,
}

/// Đầu vào định kiểu cho `AdEnumerationModuleAdapter`.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct AdEnumerationInput {
    /// Tên miền AD tổng hợp (`.example`).
    pub domain: String,
    /// IP máy Domain Controller tổng hợp (RFC 1918).
    pub dc_ip: String,
    /// Các máy trạm/máy chủ tổng hợp thuộc domain.
    pub hosts: Vec<String>,
    /// Các tài khoản dịch vụ có SPN cần kiểm tra precondition.
    pub roastable_spn_accounts: Vec<String>,
    /// Các tài khoản có cấu hình Constrained Delegation.
    pub constrained_delegation_accounts: Vec<String>,
    /// Cạnh ACL thu thập từ snapshot tổng hợp.
    pub acl_edges: Vec<AdAclEdge>,
    /// Có chạy nhánh kiểm kê hậu khai thác sau bước AD hay không.
    pub enable_post_audit_branch: bool,
}

/// Đầu vào định kiểu cho `AdConditionalAttackAdapter`.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct AdConditionalInput {
    /// Tên miền AD tổng hợp.
    pub domain: String,
    /// IP máy Domain Controller tổng hợp.
    pub dc_ip: String,
    /// Ứng viên SPN được bàn giao từ pha `AdEnumeration`.
    pub candidate_spns: Vec<String>,
    /// Ứng viên delegation được bàn giao từ pha `AdEnumeration`.
    pub delegation_accounts: Vec<String>,
    /// Cờ chế độ ồn cao (dùng kiểm thử Blue team chặn hành động Red).
    pub noisy_sweep_mode: bool,
    /// Có chuyển tiếp sang nhánh kiểm kê hậu khai thác không.
    pub enable_post_audit_branch: bool,
}

/// Đầu vào định kiểu cho `PostExploitationAuditAdapter`.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct PostAuditInput {
    /// Tên máy chủ mục tiêu `.example`.
    pub host_domain: String,
    /// IP RFC 1918 của máy mục tiêu.
    pub host_ip: String,
    /// Các mục persistence ghi nhận trên snapshot `virtual://`.
    pub persistence_items: Vec<SyntheticPersistenceItem>,
    /// Các lỗ hổng telemetry (thiếu log/SACL) phát hiện trên snapshot.
    pub telemetry_gaps: Vec<String>,
    /// Cờ ép mô phỏng tình huống dọn dẹp chưa sạch để kiểm tra fail-closed.
    pub simulate_dirty_cleanup: bool,
}

/// Đầu vào định kiểu cho `ReportingModuleAdapter`.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct ReportingInput {
    /// Tiêu đề báo cáo cuối.
    pub report_title: String,
    /// Tổng số bằng chứng đã niêm phong trong `EvidenceLedger`.
    pub ledger_count: usize,
    /// Hash đỉnh chuỗi bằng chứng hiện tại.
    pub ledger_tip_hash: String,
    /// Số finding đã xác minh tới thời điểm hiện tại.
    pub verified_finding_count: usize,
    /// Số lượng artifact giả còn sót sau dọn dẹp (bắt buộc == 0).
    pub remaining_artifacts: usize,
}

/// Enum đầu vào định kiểu dùng chung cho mọi `PentestModule` — tuyệt đối không nhận lệnh shell thô.
#[derive(Debug, Clone, PartialEq, Eq)]
pub enum ModuleInput {
    /// Đầu vào pha OSINT.
    OsintSeed(OsintSeedInput),
    /// Đầu vào pha Wireless OSWP.
    WirelessAssessment(WirelessAssessmentInput),
    /// Đầu vào pha AD Enumeration.
    AdEnumeration(AdEnumerationInput),
    /// Đầu vào pha AD Conditional Validation.
    AdConditionalValidation(AdConditionalInput),
    /// Đầu vào pha Post-Exploitation Audit.
    PostExploitationAudit(PostAuditInput),
    /// Đầu vào pha Reporting cuối cùng.
    Reporting(ReportingInput),
}

impl ModuleInput {
    /// Phân loại loại đầu vào dùng kiểm tra tương thích với module đích.
    pub fn kind(&self) -> ModuleInputKind {
        match self {
            Self::OsintSeed(_) => ModuleInputKind::OsintSeed,
            Self::WirelessAssessment(_) => ModuleInputKind::WirelessAssessment,
            Self::AdEnumeration(_) => ModuleInputKind::AdEnumeration,
            Self::AdConditionalValidation(_) => ModuleInputKind::AdConditionalValidation,
            Self::PostExploitationAudit(_) => ModuleInputKind::PostExploitationAudit,
            Self::Reporting(_) => ModuleInputKind::Reporting,
        }
    }

    /// Kiểm tra toàn bộ trường chuỗi trong input không chứa secret hoặc PII thô.
    pub fn validate_no_secrets(&self) -> Result<(), ModuleError> {
        match self {
            Self::OsintSeed(v) => {
                validate_no_raw_secret("osint.root_domain", &v.root_domain)?;
                validate_no_raw_secret("osint.org_handle", &v.org_handle)?;
                validate_no_raw_secret("osint.primary_ip", &v.primary_ip)?;
            }
            Self::WirelessAssessment(v) => {
                validate_no_raw_secret("wireless.ssid", &v.ssid)?;
                validate_no_raw_secret("wireless.site_domain", &v.site_domain)?;
                validate_no_raw_secret("wireless.controller_ip", &v.controller_ip)?;
                validate_no_raw_secret("wireless.expected_bssid", &v.expected_bssid)?;
                if let Some(b2) = &v.observed_second_bssid {
                    validate_no_raw_secret("wireless.observed_second_bssid", b2)?;
                }
            }
            Self::AdEnumeration(v) => {
                validate_no_raw_secret("ad.domain", &v.domain)?;
                validate_no_raw_secret("ad.dc_ip", &v.dc_ip)?;
                for h in &v.hosts {
                    validate_no_raw_secret("ad.host", h)?;
                }
                for s in &v.roastable_spn_accounts {
                    validate_no_raw_secret("ad.spn", s)?;
                }
                for d in &v.constrained_delegation_accounts {
                    validate_no_raw_secret("ad.delegation", d)?;
                }
                for edge in &v.acl_edges {
                    validate_no_raw_secret("ad.acl.source", &edge.source_principal)?;
                    validate_no_raw_secret("ad.acl.right", &edge.right)?;
                    validate_no_raw_secret("ad.acl.target", &edge.target_object)?;
                }
            }
            Self::AdConditionalValidation(v) => {
                validate_no_raw_secret("ad_cond.domain", &v.domain)?;
                validate_no_raw_secret("ad_cond.dc_ip", &v.dc_ip)?;
                for s in &v.candidate_spns {
                    validate_no_raw_secret("ad_cond.spn", s)?;
                }
                for d in &v.delegation_accounts {
                    validate_no_raw_secret("ad_cond.delegation", d)?;
                }
            }
            Self::PostExploitationAudit(v) => {
                validate_no_raw_secret("post.host_domain", &v.host_domain)?;
                validate_no_raw_secret("post.host_ip", &v.host_ip)?;
                for item in &v.persistence_items {
                    validate_no_raw_secret("post.item_id", &item.item_id)?;
                    validate_no_raw_secret("post.location", &item.virtual_location)?;
                    validate_no_raw_secret("post.mechanism", &item.mechanism)?;
                }
                for gap in &v.telemetry_gaps {
                    validate_no_raw_secret("post.telemetry_gap", gap)?;
                }
            }
            Self::Reporting(v) => {
                validate_no_raw_secret("reporting.title", &v.report_title)?;
            }
        }
        Ok(())
    }
}

/// Dữ liệu bàn giao định kiểu giữa các pha trong pipeline.
#[derive(Debug, Clone, PartialEq, Eq)]
pub enum ModuleHandoff {
    /// Tóm tắt tài sản thụ động thu từ OSINT.
    OsintAssets {
        discovered_hosts: Vec<String>,
        org_handle: String,
    },
    /// Các precondition tiềm năng phát hiện khi kiểm kê AD.
    AdPreconditions {
        domain: String,
        dc_ip: String,
        candidate_spns: Vec<String>,
        delegation_accounts: Vec<String>,
    },
    /// Bằng chứng sạch sau dọn dẹp hậu khai thác.
    CleanupConfirmed {
        removed_items: usize,
        remaining_artifacts: usize,
    },
}

/// Điểm kiểm tra nội bộ gắn trong mỗi `ModuleOutput`.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct StageCheckpoint {
    /// Pha vừa hoàn tất ở module này.
    pub stage: Stage,
    /// Số hiệu định danh tất định của bản lưu pha.
    pub checkpoint_tag: String,
    /// Hàm băm tóm tắt dữ liệu xuất ra.
    pub output_digest_sha256: String,
}

/// Sự kiện telemetry phát sinh bởi module để `BlueDefensePolicy` đánh giá.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct ModuleTelemetryEvent {
    /// ID duy nhất của sự kiện telemetry.
    pub event_id: String,
    /// Pha phát sinh sự kiện.
    pub stage: Stage,
    /// Mã module thực thi.
    pub actor_module_id: String,
    /// Loại thao tác trừu tượng (`passive_query`, `scoped_spn_metadata_check`, `bulk_spn_sweep`, ...).
    pub action_kind: String,
    /// Mục tiêu trong phạm vi (`.example` hoặc RFC 1918).
    pub target: String,
    /// Mức ồn dự kiến từ 0 đến 100.
    pub noise_score: u8,
}

/// Hợp đồng chung bắt buộc cho mọi module kiểm thử trong Chương 112.
pub trait PentestModule {
    /// Định danh duy nhất của module trong registry.
    fn id(&self) -> &'static str;
    /// Thực thi bước kiểm thử dựa trên ngữ cảnh và đầu vào định kiểu.
    fn run(
        &mut self,
        context: &EngagementContext,
        input: ModuleInput,
    ) -> Result<ModuleOutput, ModuleError>;
}

/// Đầu ra định kiểu thống nhất trả về bởi mọi `PentestModule`.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct ModuleOutput {
    /// ID module thực thi.
    pub module_id: &'static str,
    /// Pha được module thực thi.
    pub executed_stage: Stage,
    /// Các phát hiện hợp lệ hoặc false-positive của bước này.
    pub findings: Vec<CapstoneFinding>,
    /// Danh sách false-positive đã phân loại riêng (không đưa vào rủi ro mở).
    pub excluded_false_positives: Vec<CapstoneFinding>,
    /// Bằng chứng tạo ra (được kiểm tra redaction và gắn provenance).
    pub evidence: Vec<EvidenceRecord>,
    /// Pha kế tiếp được đề xuất (`None` chỉ hợp lệ khi hoàn tất chuỗi).
    pub next_stage: Option<Stage>,
    /// Checkpoint cấp module cho bước vừa hoàn tất.
    pub checkpoint: StageCheckpoint,
    /// Sự kiện telemetry phục vụ mô phỏng Red-vs-Blue.
    pub telemetry_events: Vec<ModuleTelemetryEvent>,
    /// Dữ liệu bàn giao định kiểu sang pha sau.
    pub handoff: Option<ModuleHandoff>,
    /// Tài nguyên tiêu thụ của riêng lần gọi này.
    pub usage: ResourceUsage,
    /// Khuyến nghị khắc phục tương ứng với phát hiện của pha.
    pub remediation_notes: Vec<String>,
    /// Cờ xác nhận bước hậu khai thác đã dọn sạch.
    pub cleanup_verified: bool,
    /// Số artifact còn sót lại sau bước này.
    pub remaining_artifacts: usize,
}

// ============================================================================
// 4. PROVENANCE, CHUỖI BẰNG CHỨNG SHA-256 & KHỬ TRÙNG LẶP ĐỘC LẬP HOÁN VỊ
// ============================================================================

/// Một bản ghi bằng chứng đã che thông tin nhạy cảm và liên kết trong chuỗi SHA-256.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct EvidenceRecord {
    /// Định danh ổn định duy nhất của bằng chứng.
    pub evidence_id: String,
    /// Pha tạo ra bằng chứng.
    pub stage: Stage,
    /// Mã module sinh bằng chứng.
    pub module_id: String,
    /// URI vị trí ảo chuẩn (`virtual://...` hoặc `lab-sensor://...`).
    pub source_uri: String,
    /// Mục tiêu liên quan (`.example` hoặc RFC 1918).
    pub target: String,
    /// Thời điểm ghi nhận UTC ISO-8601.
    pub captured_at_utc: String,
    /// Tóm tắt dữ liệu đã chạy qua bộ lọc redaction secret/PII.
    pub redacted_summary: String,
    /// Số lượng phần tử bí mật/PII đã được che tự động.
    pub redaction_count: usize,
    /// Số thứ tự trong `EvidenceLedger` (gắn khi nạp vào sổ).
    pub sequence: usize,
    /// Hàm băm bản ghi trước (`LEDESIS` ở mắt xích đầu tiên).
    pub prev_hash: String,
    /// Hàm băm SHA-256 đóng khung độ dài của bản ghi này.
    pub record_hash: String,
}

impl EvidenceRecord {
    /// Khởi tạo bằng chứng sạch ở cấp module (tự động redaction và kiểm tra URI/UTC).
    pub fn from_raw(
        evidence_id: &str,
        stage: Stage,
        module_id: &str,
        source_uri: &str,
        target: &str,
        captured_at_utc: &str,
        raw_summary: &str,
    ) -> Result<Self, ModuleError> {
        if evidence_id.trim().is_empty() {
            return Err(ModuleError::InvalidInput(
                "evidence_id không được rỗng".to_string(),
            ));
        }
        validate_no_raw_secret("evidence.id", evidence_id)?;
        validate_no_raw_secret("evidence.module_id", module_id)?;
        let clean_uri = validate_synthetic_location(source_uri)?;
        validate_no_raw_secret("evidence.source_uri", &clean_uri)?;
        validate_no_raw_secret("evidence.target", target)?;
        parse_utc_timestamp(captured_at_utc)?;

        let (redacted_summary, redaction_count) = redact_sensitive_text(raw_summary);
        validate_no_raw_secret("evidence.redacted_summary", &redacted_summary)?;

        Ok(Self {
            evidence_id: evidence_id.trim().to_string(),
            stage,
            module_id: module_id.trim().to_string(),
            source_uri: clean_uri,
            target: target.trim().to_string(),
            captured_at_utc: captured_at_utc.to_string(),
            redacted_summary,
            redaction_count,
            sequence: 0,
            prev_hash: String::new(),
            record_hash: String::new(),
        })
    }

    /// Tính hàm băm chuẩn hóa đóng khung độ dài cho bản ghi tại vị trí liên kết cho trước.
    pub fn compute_linked_hash(&self, sequence: usize, prev_hash: &str) -> String {
        let mut payload = String::new();
        push_lp(&mut payload, &sequence.to_string());
        push_lp(&mut payload, prev_hash);
        push_lp(&mut payload, &self.evidence_id);
        push_lp(&mut payload, self.stage.as_str());
        push_lp(&mut payload, &self.module_id);
        push_lp(&mut payload, &self.source_uri);
        push_lp(&mut payload, &self.target);
        push_lp(&mut payload, &self.captured_at_utc);
        push_lp(&mut payload, &self.redacted_summary);
        push_lp(&mut payload, &self.redaction_count.to_string());
        sha256_hex(payload.as_bytes())
    }

    /// Xác thực chữ ký băm nội dung và không rò rỉ bí mật.
    pub fn verify_link(
        &self,
        expected_seq: usize,
        expected_prev_hash: &str,
    ) -> Result<(), OrchestratorError> {
        validate_no_raw_secret("evidence.summary", &self.redacted_summary)?;
        validate_no_raw_secret("evidence.source_uri", &self.source_uri)?;
        validate_synthetic_location(&self.source_uri)?;
        if self.sequence != expected_seq || self.prev_hash != expected_prev_hash {
            return Err(OrchestratorError::EvidenceTampered(format!(
                "bằng chứng `{}` sai sequence hoặc prev_hash",
                self.evidence_id
            )));
        }
        let expected = self.compute_linked_hash(expected_seq, expected_prev_hash);
        if self.record_hash != expected {
            return Err(OrchestratorError::EvidenceTampered(format!(
                "bằng chứng `{}` có record_hash không khớp nội dung",
                self.evidence_id
            )));
        }
        Ok(())
    }
}

/// Sổ cái bằng chứng gắn chuỗi SHA-256 chống chối bỏ hoặc sửa lén quá khứ.
#[derive(Debug, Clone, Default, PartialEq, Eq)]
pub struct EvidenceLedger {
    records: Vec<EvidenceRecord>,
}

impl EvidenceLedger {
    /// Khởi tạo sổ cái rỗng.
    pub fn new() -> Self {
        Self {
            records: Vec::new(),
        }
    }

    /// Trả về hash đỉnh chuỗi hiện tại (`LEDGER_GENESIS_HASH` nếu rỗng).
    pub fn tip_hash(&self) -> String {
        self.records
            .last()
            .map(|r| r.record_hash.clone())
            .unwrap_or_else(|| LEDGER_GENESIS_HASH.to_string())
    }

    /// Trả về lát cắt toàn bộ bằng chứng theo thứ tự ghi nhận.
    pub fn records(&self) -> &[EvidenceRecord] {
        &self.records
    }

    /// Tìm một bản ghi theo ID ổn định.
    pub fn get(&self, evidence_id: &str) -> Option<&EvidenceRecord> {
        self.records.iter().find(|r| r.evidence_id == evidence_id)
    }

    /// Nạp một lô bằng chứng nguyên tử: lỗi ở bất kỳ bản ghi nào sẽ hủy toàn bộ lô.
    pub fn append_batch(&mut self, batch: Vec<EvidenceRecord>) -> Result<(), OrchestratorError> {
        let mut staging = self.clone();
        for mut item in batch {
            validate_no_raw_secret("ledger.id", &item.evidence_id)?;
            validate_no_raw_secret("ledger.summary", &item.redacted_summary)?;
            validate_synthetic_location(&item.source_uri)?;
            if staging.get(&item.evidence_id).is_some() {
                return Err(OrchestratorError::EvidenceTampered(format!(
                    "trùng lặp evidence_id `{}` khi nạp sổ cái",
                    item.evidence_id
                )));
            }
            let seq = staging.records.len() + 1;
            let prev_hash = staging.tip_hash();
            item.sequence = seq;
            item.prev_hash = prev_hash.clone();
            item.record_hash = item.compute_linked_hash(seq, &prev_hash);
            staging.records.push(item);
        }
        *self = staging;
        Ok(())
    }

    /// Kiểm tra toàn vẹn toàn bộ chuỗi SHA-256 từ genesis tới đỉnh hiện tại.
    pub fn verify(&self) -> Result<(), OrchestratorError> {
        let mut expected_prev = LEDGER_GENESIS_HASH.to_string();
        let mut seen = BTreeSet::new();
        for (idx, record) in self.records.iter().enumerate() {
            if !seen.insert(record.evidence_id.clone()) {
                return Err(OrchestratorError::EvidenceTampered(format!(
                    "phát hiện trùng evidence_id `{}` trong chuỗi",
                    record.evidence_id
                )));
            }
            record.verify_link(idx + 1, &expected_prev)?;
            expected_prev = record.record_hash.clone();
        }
        Ok(())
    }
}

/// Mức độ nghiêm trọng của phát hiện trong Capstone.
#[derive(Debug, Clone, Copy, PartialEq, Eq, PartialOrd, Ord)]
pub enum FindingSeverity {
    /// Thông tin bổ trợ.
    Info,
    /// Thấp.
    Low,
    /// Trung bình.
    Medium,
    /// Cao.
    High,
    /// Nghiêm trọng.
    Critical,
}

impl FindingSeverity {
    /// Nhãn chuỗi chuẩn hóa.
    pub fn as_str(&self) -> &'static str {
        match self {
            Self::Info => "Info",
            Self::Low => "Low",
            Self::Medium => "Medium",
            Self::High => "High",
            Self::Critical => "Critical",
        }
    }
}

/// Trạng thái xác minh phát hiện.
#[derive(Debug, Clone, Copy, PartialEq, Eq, PartialOrd, Ord)]
pub enum CapstoneFindingStatus {
    /// Đã xác minh trên fixture kèm ít nhất một bằng chứng thật trong sổ cái.
    Verified,
    /// Báo động giả (ví dụ thiết bị phụ được cấp phép trong inventory, hoặc stale DNS).
    FalsePositive,
    /// Hành động Red bị Blue chặn trước khi thành công — không tính là thành công.
    BlockedByBlue,
    /// Đã khắc phục và kiểm chứng sạch.
    RemediatedVerified,
}

impl CapstoneFindingStatus {
    /// Nhãn chuỗi chuẩn hóa.
    pub fn as_str(&self) -> &'static str {
        match self {
            Self::Verified => "Verified",
            Self::FalsePositive => "FalsePositive",
            Self::BlockedByBlue => "BlockedByBlue",
            Self::RemediatedVerified => "RemediatedVerified",
        }
    }
}

/// Phát hiện bảo mật có nguồn gốc kiểm chứng trong chiến dịch.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct CapstoneFinding {
    /// Mã phát hiện duy nhất.
    pub finding_id: String,
    /// Mã quy tắc kiểm tra.
    pub rule_id: String,
    /// Khóa nguyên nhân gốc phục vụ khử trùng lặp.
    pub root_cause_key: String,
    /// Tiêu đề phát hiện (tất định, không chứa secret thô).
    pub title: String,
    /// Pha phát hiện.
    pub stage: Stage,
    /// Tài sản bị ảnh hưởng (`.example` hoặc RFC 1918).
    pub affected_asset: String,
    /// Mức độ nghiêm trọng.
    pub severity: FindingSeverity,
    /// Điểm CVSS 3.1 nhân 10 (`0..=100`).
    pub cvss_tenths: u8,
    /// Trạng thái xác minh hiện tại.
    pub status: CapstoneFindingStatus,
    /// Tập các `evidence_id` chứng minh cho phát hiện này.
    pub evidence_ids: BTreeSet<String>,
    /// Liên kết tới ID sự kiện telemetry tương ứng (nếu có) để kiểm tra chặn Blue Team.
    pub telemetry_event_id: Option<String>,
    /// Biện pháp khắc phục đề xuất.
    pub remediation: String,
}

/// Kiểm tra bất biến provenance: mọi finding xác minh phải có >= 1 evidence thật và sạch secret.
pub fn validate_finding_provenance(
    finding: &CapstoneFinding,
    ledger: &EvidenceLedger,
) -> Result<(), OrchestratorError> {
    validate_no_raw_secret("finding.id", &finding.finding_id)?;
    validate_no_raw_secret("finding.rule_id", &finding.rule_id)?;
    validate_no_raw_secret("finding.root_cause_key", &finding.root_cause_key)?;
    validate_no_raw_secret("finding.title", &finding.title)?;
    validate_no_raw_secret("finding.asset", &finding.affected_asset)?;
    validate_no_raw_secret("finding.remediation", &finding.remediation)?;
    if let Some(tid) = &finding.telemetry_event_id {
        validate_no_raw_secret("finding.telemetry_event_id", tid)?;
    }
    if finding.cvss_tenths > 100 {
        return Err(OrchestratorError::ReportTampered(format!(
            "finding `{}` có cvss_tenths > 100",
            finding.finding_id
        )));
    }
    if matches!(
        finding.status,
        CapstoneFindingStatus::Verified | CapstoneFindingStatus::RemediatedVerified
    ) {
        if finding.remediation.trim().is_empty() {
            return Err(OrchestratorError::UnsupportedClaim(format!(
                "finding `{}` mang trạng thái {:?} nhưng thiếu remediation",
                finding.finding_id, finding.status
            )));
        }
        if finding.evidence_ids.is_empty() {
            return Err(OrchestratorError::UnsupportedClaim(format!(
                "finding `{}` mang trạng thái {:?} nhưng thiếu evidence_ids",
                finding.finding_id, finding.status
            )));
        }
        for ev_id in &finding.evidence_ids {
            if ledger.get(ev_id).is_none() {
                return Err(OrchestratorError::UnsupportedClaim(format!(
                    "finding `{}` tham chiếu evidence_id `{ev_id}` không tồn tại trong sổ cái",
                    finding.finding_id
                )));
            }
        }
    }
    Ok(())
}

/// Khử trùng lặp phát hiện độc lập hoán vị, giữ trọn mọi `evidence_ids`.
pub fn deduplicate_capstone_findings(
    findings: Vec<CapstoneFinding>,
) -> Result<Vec<CapstoneFinding>, OrchestratorError> {
    let mut grouped: BTreeMap<
        (String, String, String, CapstoneFindingStatus),
        Vec<CapstoneFinding>,
    > = BTreeMap::new();
    for f in findings {
        validate_no_raw_secret("dedup.title", &f.title)?;
        validate_no_raw_secret("dedup.remediation", &f.remediation)?;
        let key = (
            f.rule_id.trim().to_ascii_lowercase(),
            f.root_cause_key.trim().to_ascii_lowercase(),
            f.affected_asset.trim().to_ascii_lowercase(),
            f.status,
        );
        grouped.entry(key).or_default().push(f);
    }

    let mut out = Vec::with_capacity(grouped.len());
    for (_, mut items) in grouped {
        items.sort_by(|a, b| {
            b.cvss_tenths
                .cmp(&a.cvss_tenths)
                .then_with(|| a.finding_id.cmp(&b.finding_id))
        });
        let mut primary = items.remove(0);
        let mut remediations = BTreeSet::new();
        if !primary.remediation.trim().is_empty() {
            remediations.insert(primary.remediation.clone());
        }
        for other in items {
            primary.evidence_ids.extend(other.evidence_ids);
            if other.severity > primary.severity {
                primary.severity = other.severity;
            }
            if other.cvss_tenths > primary.cvss_tenths {
                primary.cvss_tenths = other.cvss_tenths;
            }
            if !other.remediation.trim().is_empty() {
                remediations.insert(other.remediation);
            }
        }
        if !remediations.is_empty() {
            primary.remediation = remediations.into_iter().collect::<Vec<_>>().join("; ");
        }
        out.push(primary);
    }
    out.sort_by(|a, b| {
        b.cvss_tenths
            .cmp(&a.cvss_tenths)
            .then_with(|| a.finding_id.cmp(&b.finding_id))
    });
    Ok(out)
}

// ============================================================================
// 5. CHÍNH SÁCH ĐỐI KHÁNG RED-VS-BLUE & HÀM PHẦN THƯỞNG RL (PENALTY DƯƠNG)
// ============================================================================

/// Phán quyết của đội Xanh đối với một sự kiện telemetry của đội Đỏ.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum BlueDisposition {
    /// Chưa vượt ngưỡng cảnh báo.
    AllowSilent,
    /// Phát hiện và phát cảnh báo SOC, nhưng vẫn cho yêu cầu hoàn tất để kiểm nghiệm phòng thủ.
    DetectAndAlert,
    /// Chặn đứng hành động Red — không cho phép tính thành công.
    DetectAndBlock,
}

/// Cảnh báo telemetry do Blue Team tạo ra.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct BlueTelemetryAlert {
    /// Mã cảnh báo duy nhất.
    pub alert_id: String,
    /// Mã sự kiện kích hoạt cảnh báo.
    pub telemetry_event_id: String,
    /// Pha diễn ra sự kiện.
    pub stage: Stage,
    /// Phán quyết của đội Xanh.
    pub disposition: BlueDisposition,
    /// Thông điệp mô tả dấu hiệu phát hiện.
    pub summary: String,
}

/// Bản ghi hành động Red bị Blue chặn (không bao giờ được tính là thành công).
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct BlockedActionRecord {
    /// Pha xảy ra vụ chặn.
    pub stage: Stage,
    /// Mã module thực hiện.
    pub module_id: String,
    /// Mã sự kiện telemetry bị chặn.
    pub telemetry_event_id: String,
    /// Tên hành động bị chặn.
    pub action_kind: String,
    /// Mục tiêu liên quan.
    pub target: String,
    /// Lý do chặn từ chính sách phòng thủ.
    pub reason: String,
}

/// Kết quả đánh giá một sự kiện telemetry bởi chính sách đội Xanh.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct BlueDecision {
    /// Phán quyết tất định.
    pub disposition: BlueDisposition,
    /// Bản ghi cảnh báo (nếu phát hiện).
    pub alert: Option<BlueTelemetryAlert>,
    /// Phản hồi kỹ thuật gửi lại cho vòng điều phối Red để thích nghi.
    pub feedback_for_red: String,
}

/// Chính sách phòng thủ tất định của Blue Team trong mô phỏng đối kháng.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct BlueDefensePolicy {
    /// Chặn cứng thao tác quét SPN hàng loạt ồn ào (`bulk_spn_sweep`).
    pub block_noisy_spn_sweep: bool,
    /// Ngưỡng mức ồn tối đa trước khi chặn cứng (`0..=100`).
    pub block_noise_threshold: u8,
    /// Ngưỡng mức ồn phát cảnh báo SOC (`0..=100`).
    pub alert_noise_threshold: u8,
}

impl BlueDefensePolicy {
    /// Chính sách chuẩn của Blue SOC: cảnh báo từ noise >= 25, chặn cứng từ noise >= 70 hoặc `bulk_spn_sweep`.
    pub fn standard() -> Self {
        Self {
            block_noisy_spn_sweep: true,
            block_noise_threshold: 70,
            alert_noise_threshold: 25,
        }
    }

    /// Đánh giá tất định một sự kiện telemetry phát sinh bởi module Red.
    pub fn evaluate(&self, event: &ModuleTelemetryEvent) -> BlueDecision {
        let must_block = (self.block_noisy_spn_sweep && event.action_kind == "bulk_spn_sweep")
            || event.noise_score >= self.block_noise_threshold;
        if must_block {
            let reason = format!(
                "Blue SOC chặn `{}` tới `{}` (noise={} >= {})",
                event.action_kind, event.target, event.noise_score, self.block_noise_threshold
            );
            return BlueDecision {
                disposition: BlueDisposition::DetectAndBlock,
                alert: Some(BlueTelemetryAlert {
                    alert_id: format!("BLUE-BLK-{}", event.event_id),
                    telemetry_event_id: event.event_id.clone(),
                    stage: event.stage,
                    disposition: BlueDisposition::DetectAndBlock,
                    summary: reason.clone(),
                }),
                feedback_for_red: format!(
                    "{reason}; giảm phạm vi truy vấn và tuân thủ truy vấn mục tiêu hẹp"
                ),
            };
        }

        if event.noise_score >= self.alert_noise_threshold {
            let summary = format!(
                "Blue SOC ghi nhận cảnh báo mức thấp cho `{}` trên `{}` (noise={})",
                event.action_kind, event.target, event.noise_score
            );
            return BlueDecision {
                disposition: BlueDisposition::DetectAndAlert,
                alert: Some(BlueTelemetryAlert {
                    alert_id: format!("BLUE-ALT-{}", event.event_id),
                    telemetry_event_id: event.event_id.clone(),
                    stage: event.stage,
                    disposition: BlueDisposition::DetectAndAlert,
                    summary: summary.clone(),
                }),
                feedback_for_red: summary,
            };
        }

        BlueDecision {
            disposition: BlueDisposition::AllowSilent,
            alert: None,
            feedback_for_red: "không phát hiện bất thường ở ngưỡng cảnh báo hiện tại".to_string(),
        }
    }
}

/// Bộ trọng số cho hàm phần thưởng RL: MỌI penalty trong bảng có độ lớn DƯƠNG (`>= 0.0`)
/// và được **TRỪ (`-`)** rõ ràng trong công thức tính thưởng.
#[derive(Debug, Clone, Copy, PartialEq)]
pub struct CapstoneRewardWeights {
    /// Phần thưởng dương cho mỗi finding đã được xác minh thật bằng bằng chứng.
    pub verified_finding_reward: f64,
    /// Phần thưởng dương khi hoàn thành báo cáo sạch và cleanup đủ chuẩn.
    pub clean_report_bonus: f64,
    /// Độ lớn dương trừ đi khi hành vi bị Blue Team phát hiện cảnh báo.
    pub detection_penalty: f64,
    /// Độ lớn dương trừ đi khi hành động bị Blue Team chặn cứng.
    pub blue_block_penalty: f64,
    /// Độ lớn dương trừ đi nếu gây gián đoạn dịch vụ mô phỏng.
    pub service_disruption_penalty: f64,
    /// Độ lớn dương trừ đi nếu có vi phạm cổng an toàn.
    pub safety_violation_penalty: f64,
    /// Độ lớn dương trừ đi trên mỗi bước thực thi.
    pub step_cost_penalty: f64,
}

impl CapstoneRewardWeights {
    /// Bộ trọng số tiêu chuẩn nhất quán với quy ước penalty dương đã chốt ở `AGENTS.md`.
    pub fn standard() -> Self {
        Self {
            verified_finding_reward: 10.0,
            clean_report_bonus: 5.0,
            detection_penalty: 3.0,
            blue_block_penalty: 6.0,
            service_disruption_penalty: 12.0,
            safety_violation_penalty: 25.0,
            step_cost_penalty: 0.5,
        }
    }

    /// Từ chối mọi trọng số âm, `NaN` hoặc vô cực để tránh lỗi "trừ hai lần biến phạt thành thưởng".
    pub fn validate(&self) -> Result<(), OrchestratorError> {
        let fields = [
            ("verified_finding_reward", self.verified_finding_reward),
            ("clean_report_bonus", self.clean_report_bonus),
            ("detection_penalty", self.detection_penalty),
            ("blue_block_penalty", self.blue_block_penalty),
            (
                "service_disruption_penalty",
                self.service_disruption_penalty,
            ),
            ("safety_violation_penalty", self.safety_violation_penalty),
            ("step_cost_penalty", self.step_cost_penalty),
        ];
        for (name, val) in fields {
            if !val.is_finite() || val < 0.0 {
                return Err(OrchestratorError::RewardConfigInvalid(format!(
                    "trọng số `{name}` phải hữu hạn và >= 0.0 (độ lớn dương), nhận {val}"
                )));
            }
        }
        Ok(())
    }

    /// Tính điểm thưởng bước từ quan sát hành vi; mọi thành phần phạt được **TRỪ** trong công thức.
    pub fn compute_step_reward(&self, outcome: &RlStepOutcome) -> Result<f64, OrchestratorError> {
        self.validate()?;
        let positive = self.verified_finding_reward * (outcome.verified_findings_added as f64)
            + if outcome.clean_report_achieved {
                self.clean_report_bonus
            } else {
                0.0
            };
        let penalty = if outcome.detected_by_blue {
            self.detection_penalty
        } else {
            0.0
        } + self.blue_block_penalty * (outcome.blocked_by_blue_count as f64)
            + if outcome.service_disrupted {
                self.service_disruption_penalty
            } else {
                0.0
            }
            + self.safety_violation_penalty * (outcome.safety_violations as f64)
            + self.step_cost_penalty;

        Ok(positive - penalty)
    }
}

impl Default for CapstoneRewardWeights {
    fn default() -> Self {
        Self::standard()
    }
}

/// Đầu vào đo lường của một bước phục vụ tính thưởng RL.
#[derive(Debug, Clone, Copy, Default, PartialEq, Eq)]
pub struct RlStepOutcome {
    /// Số lượng finding `Verified` hợp lệ (sau khi đã loại mọi finding bị Blue chặn).
    pub verified_findings_added: usize,
    /// Đạt báo cáo cuối sạch toàn vẹn ở bước `Reporting`.
    pub clean_report_achieved: bool,
    /// Blue Team đã phát hiện telemetry bước này hay chưa.
    pub detected_by_blue: bool,
    /// Số lượng hành động Red bị Blue chặn ở bước này.
    pub blocked_by_blue_count: usize,
    /// Có làm gián đoạn dịch vụ mô phỏng hay không.
    pub service_disrupted: bool,
    /// Số lượng vi phạm an toàn ghi nhận.
    pub safety_violations: usize,
}

/// Bản ghi phần thưởng tại từng bước của chiến dịch.
#[derive(Debug, Clone, PartialEq)]
pub struct RlStepRecord {
    /// Pha được chấm điểm.
    pub stage: Stage,
    /// Đo lường kết quả bước.
    pub outcome: RlStepOutcome,
    /// Điểm thưởng ròng nhận được (tính bằng triệu/milli-units nếu cần, ở đây là `f64`).
    pub reward: f64,
}

/// Tổng kết chỉ số RL cho toàn chiến dịch (biểu diễn dạng cố định millipoints để tránh trôi bit JSON).
#[derive(Debug, Clone, Default, PartialEq, Eq)]
pub struct RlEpisodeSummary {
    /// Tổng số bước đã chấm thưởng.
    pub total_steps: usize,
    /// Tổng phần thưởng nhân 1000 (`milli_reward`) để bảo toàn tất định trên JSON & replay.
    pub total_reward_milli: i64,
    /// Số lần bị Blue chặn.
    pub total_blue_blocks: usize,
    /// Số lần bị Blue phát hiện cảnh báo.
    pub total_blue_detections: usize,
    /// Tổng số finding xác minh hợp lệ được ghi công.
    pub total_verified_findings: usize,
}

/// Hành động chiến thuật AD có thể chọn trong vòng học đối kháng Red-vs-Blue.
#[derive(Debug, Clone, Copy, PartialEq, Eq, PartialOrd, Ord)]
pub enum AdaptivePolicyAction {
    /// Quét hàng loạt nhiều SPN cùng lúc (ồn cao, dễ bị Blue chặn).
    NoisyBulkSweep,
    /// Kiểm tra hẹp theo danh sách SPN đã xác định từ OSINT/Enumeration (ồn thấp).
    TargetedScopedCheck,
}

/// Bộ học Q-table nhỏ gọn tất định giúp Red tránh chiến thuật bị Blue chặn qua từng tập mô phỏng.
#[derive(Debug, Clone, PartialEq)]
pub struct CapstoneQPolicy {
    /// Bảng giá trị Q cho từng hành động ở pha kiểm chứng AD có điều kiện.
    pub q_values: BTreeMap<AdaptivePolicyAction, f64>,
    /// Tốc độ học alpha.
    pub alpha: f64,
}

impl CapstoneQPolicy {
    /// Khởi tạo chính sách với kỳ vọng ban đầu cân bằng.
    pub fn new() -> Self {
        let mut q_values = BTreeMap::new();
        q_values.insert(AdaptivePolicyAction::NoisyBulkSweep, 0.0);
        q_values.insert(AdaptivePolicyAction::TargetedScopedCheck, 0.0);
        Self {
            q_values,
            alpha: 0.5,
        }
    }

    /// Cập nhật Q-value tất định theo phần thưởng thực tế trả về từ Blue + RL weights.
    pub fn update(&mut self, action: AdaptivePolicyAction, reward: f64) {
        let cur = self.q_values.get(&action).copied().unwrap_or(0.0);
        self.q_values
            .insert(action, cur + self.alpha * (reward - cur));
    }

    /// Chọn hành động có Q-value cao nhất (ưu tiên hành động hẹp khi bằng điểm).
    pub fn best_action(&self) -> AdaptivePolicyAction {
        let noisy_q = self
            .q_values
            .get(&AdaptivePolicyAction::NoisyBulkSweep)
            .copied()
            .unwrap_or(0.0);
        let targeted_q = self
            .q_values
            .get(&AdaptivePolicyAction::TargetedScopedCheck)
            .copied()
            .unwrap_or(0.0);
        if targeted_q >= noisy_q {
            AdaptivePolicyAction::TargetedScopedCheck
        } else {
            AdaptivePolicyAction::NoisyBulkSweep
        }
    }
}

impl Default for CapstoneQPolicy {
    fn default() -> Self {
        Self::new()
    }
}

// ============================================================================
// 6. SÁU ADAPTER MÔ PHỎNG TỰ CHỨA TRIỂN KHAI `PentestModule`
// ============================================================================

/// Adapter 1: OSINT thụ động trên nguồn `.example` và RFC 1918 (không mạng thật).
#[derive(Debug, Clone, Default)]
pub struct OsintModuleAdapter;

impl PentestModule for OsintModuleAdapter {
    fn id(&self) -> &'static str {
        "mod.osint.passive"
    }

    fn run(
        &mut self,
        context: &EngagementContext,
        input: ModuleInput,
    ) -> Result<ModuleOutput, ModuleError> {
        input.validate_no_secrets()?;
        let ModuleInput::OsintSeed(seed) = input else {
            return Err(ModuleError::InvalidInput(
                "OsintModuleAdapter chỉ nhận ModuleInput::OsintSeed".to_string(),
            ));
        };
        if seed.passive_sources.is_empty() {
            return Err(ModuleError::InvalidInput(
                "phải cung cấp ít nhất một nguồn OSINT thụ động".to_string(),
            ));
        }
        let clean_domain = context
            .scope
            .check_domain(&seed.root_domain, &context.current_virtual_utc)?;
        context.check_stage_approval_for_target(Stage::Osint, &clean_domain)?;
        let clean_ip = context.scope.check_ip(&seed.primary_ip)?;
        validate_no_raw_secret("osint.org_handle", &seed.org_handle)?;

        let mut evidence = Vec::new();
        for (idx, src) in seed.passive_sources.iter().enumerate() {
            let ev_id = format!("EV-OSINT-{:03}", idx + 1);
            let uri = format!("virtual://{clean_domain}/osint/{}", src.as_str());
            let summary = format!(
                "Passive {} snapshot for org={} domain={} ip={} (no active scan)",
                src.as_str(),
                seed.org_handle,
                clean_domain,
                clean_ip
            );
            evidence.push(EvidenceRecord::from_raw(
                &ev_id,
                Stage::Osint,
                self.id(),
                &uri,
                &clean_domain,
                &context.current_virtual_utc,
                &summary,
            )?);
        }

        let mut ev_ids = BTreeSet::new();
        ev_ids.insert(evidence[0].evidence_id.clone());

        let finding = CapstoneFinding {
            finding_id: "CAP-OSINT-001".to_string(),
            rule_id: "OSINT-EXPOSED-SUBDOMAIN".to_string(),
            root_cause_key: format!("subdomains-ct:{clean_domain}"),
            title: format!("Bản ghi CT log để lộ tên miền nội bộ dc01.{clean_domain}"),
            stage: Stage::Osint,
            affected_asset: clean_domain.clone(),
            severity: FindingSeverity::Medium,
            cvss_tenths: 53,
            status: CapstoneFindingStatus::Verified,
            evidence_ids: ev_ids,
            telemetry_event_id: Some("TEL-OSINT-001".to_string()),
            remediation: "Ẩn tên miền nội bộ khỏi public CT bằng chứng chỉ split-horizon nội bộ"
                .to_string(),
        };

        // Bản ghi stale cũ đánh dấu FalsePositive để minh họa lọc báo động giả
        let stale_fp = CapstoneFinding {
            finding_id: "CAP-OSINT-FP-002".to_string(),
            rule_id: "OSINT-STALE-LEGACY-TXT".to_string(),
            root_cause_key: format!("stale-txt:{clean_domain}"),
            title: format!("Bản ghi DNS cũ của legacy.{clean_domain} đã thu hồi"),
            stage: Stage::Osint,
            affected_asset: clean_domain.clone(),
            severity: FindingSeverity::Info,
            cvss_tenths: 0,
            status: CapstoneFindingStatus::FalsePositive,
            evidence_ids: BTreeSet::new(),
            telemetry_event_id: None,
            remediation: "Không cần xử lý thêm vì bản ghi đã hết hiệu lực trên DNS đích"
                .to_string(),
        };

        let next = if seed.enable_wireless_branch {
            Stage::WirelessAssessment
        } else {
            Stage::AdEnumeration
        };
        let digest =
            sha256_hex(format!("osint:{clean_domain}:{clean_ip}:{}", evidence.len()).as_bytes());

        Ok(ModuleOutput {
            module_id: self.id(),
            executed_stage: Stage::Osint,
            findings: vec![finding],
            excluded_false_positives: vec![stale_fp],
            evidence,
            next_stage: Some(next),
            checkpoint: StageCheckpoint {
                stage: Stage::Osint,
                checkpoint_tag: format!("chk-osint-{}", &digest[..8]),
                output_digest_sha256: digest,
            },
            telemetry_events: vec![ModuleTelemetryEvent {
                event_id: "TEL-OSINT-001".to_string(),
                stage: Stage::Osint,
                actor_module_id: self.id().to_string(),
                action_kind: "passive_ct_dns_lookup".to_string(),
                target: clean_domain.clone(),
                noise_score: 5,
            }],
            handoff: Some(ModuleHandoff::OsintAssets {
                discovered_hosts: vec![
                    format!("dc01.{clean_domain}"),
                    format!("app01.{clean_domain}"),
                ],
                org_handle: seed.org_handle,
            }),
            usage: ResourceUsage {
                modules_executed: 1,
                tool_calls: seed.passive_sources.len() as u32,
                cost_units: 5,
                time_ms: 120,
            },
            remediation_notes: vec![
                "Áp dụng chứng chỉ nội bộ cho các dịch vụ không hướng Internet".to_string(),
            ],
            cleanup_verified: true,
            remaining_artifacts: 0,
        })
    }
}

/// Adapter 2 (Nhánh tùy chọn): Đánh giá không dây OSWP từ capture tổng hợp (không radio thật).
#[derive(Debug, Clone, Default)]
pub struct WirelessModuleAdapter;

impl PentestModule for WirelessModuleAdapter {
    fn id(&self) -> &'static str {
        "mod.wireless.oswp"
    }

    fn run(
        &mut self,
        context: &EngagementContext,
        input: ModuleInput,
    ) -> Result<ModuleOutput, ModuleError> {
        context.check_stage_approval(Stage::WirelessAssessment)?;
        input.validate_no_secrets()?;
        let ModuleInput::WirelessAssessment(w) = input else {
            return Err(ModuleError::InvalidInput(
                "WirelessModuleAdapter chỉ nhận ModuleInput::WirelessAssessment".to_string(),
            ));
        };

        let site_domain = context
            .scope
            .check_domain(&w.site_domain, &context.current_virtual_utc)?;
        context.check_stage_approval_for_target(Stage::WirelessAssessment, &site_domain)?;
        context.scope.check_ip(&w.controller_ip)?;
        if w.snr_db < -30 || w.snr_db > 90 {
            return Err(ModuleError::InvalidInput(format!(
                "giá trị SNR {} dB bất hợp lý cho capture tổng hợp",
                w.snr_db
            )));
        }

        let ev = EvidenceRecord::from_raw(
            "EV-WIFI-001",
            Stage::WirelessAssessment,
            self.id(),
            &format!("lab-sensor://{site_domain}/wlan/capture"),
            &site_domain,
            &context.current_virtual_utc,
            &format!(
                "Offline fixture SSID={} expected_bssid={} second_bssid={:?} authorized={} suite={:?} snr={}dB pmkid={}",
                w.ssid,
                w.expected_bssid,
                w.observed_second_bssid,
                w.second_bssid_authorized,
                w.observed_suite,
                w.snr_db,
                w.pmkid_present
            ),
        )?;

        let mut findings = Vec::new();
        let mut excluded = Vec::new();
        let mut ev_ids = BTreeSet::new();
        ev_ids.insert(ev.evidence_id.clone());

        if let Some(second) = &w.observed_second_bssid {
            if w.second_bssid_authorized {
                excluded.push(CapstoneFinding {
                    finding_id: "CAP-WIFI-FP-001".to_string(),
                    rule_id: "WIFI-SAME-SSID-AUTHORIZED".to_string(),
                    root_cause_key: format!("ap-dual-bss:{site_domain}"),
                    title: format!("BSSID thứ hai {second} nằm trong danh mục AP được duyệt"),
                    stage: Stage::WirelessAssessment,
                    affected_asset: site_domain.clone(),
                    severity: FindingSeverity::Info,
                    cvss_tenths: 0,
                    status: CapstoneFindingStatus::FalsePositive,
                    evidence_ids: ev_ids.clone(),
                    telemetry_event_id: None,
                    remediation: "Không phải Evil Twin — chỉ cập nhật tài sản Wi-Fi theo chu kỳ"
                        .to_string(),
                });
            } else {
                findings.push(CapstoneFinding {
                    finding_id: "CAP-WIFI-001".to_string(),
                    rule_id: "WIFI-ROGUE-EVIL-TWIN-RISK".to_string(),
                    root_cause_key: format!("rogue-bss:{site_domain}"),
                    title: format!(
                        "Phát hiện AP lạ {second} mạo danh SSID {} không bật PMF",
                        w.ssid
                    ),
                    stage: Stage::WirelessAssessment,
                    affected_asset: site_domain.clone(),
                    severity: FindingSeverity::High,
                    cvss_tenths: 75,
                    status: CapstoneFindingStatus::Verified,
                    evidence_ids: ev_ids.clone(),
                    telemetry_event_id: Some("TEL-WIFI-001".to_string()),
                    remediation: "Bật bắt buộc WPA3-SAE/Enterprise với PMF (802.11w) và WIDS"
                        .to_string(),
                });
            }
        }

        if w.observed_suite != WirelessSecuritySuite::Wpa3SaePmfRequired && findings.is_empty() {
            findings.push(CapstoneFinding {
                finding_id: "CAP-WIFI-002".to_string(),
                rule_id: "WIFI-MISSING-PMF".to_string(),
                root_cause_key: format!("missing-pmf:{site_domain}"),
                title: format!("Mạng SSID {} chưa bật bắt buộc PMF/WPA3-SAE", w.ssid),
                stage: Stage::WirelessAssessment,
                affected_asset: site_domain.clone(),
                severity: FindingSeverity::Medium,
                cvss_tenths: 59,
                status: CapstoneFindingStatus::Verified,
                evidence_ids: ev_ids,
                telemetry_event_id: Some("TEL-WIFI-001".to_string()),
                remediation: "Nâng cấp lên WPA3-SAE hoặc WPA2-Enterprise kèm PMF bắt buộc"
                    .to_string(),
            });
        }

        let digest = sha256_hex(format!("wifi:{site_domain}:{}", w.ssid).as_bytes());
        Ok(ModuleOutput {
            module_id: self.id(),
            executed_stage: Stage::WirelessAssessment,
            findings,
            excluded_false_positives: excluded,
            evidence: vec![ev],
            next_stage: Some(Stage::AdEnumeration),
            checkpoint: StageCheckpoint {
                stage: Stage::WirelessAssessment,
                checkpoint_tag: format!("chk-wifi-{}", &digest[..8]),
                output_digest_sha256: digest,
            },
            telemetry_events: vec![ModuleTelemetryEvent {
                event_id: "TEL-WIFI-001".to_string(),
                stage: Stage::WirelessAssessment,
                actor_module_id: self.id().to_string(),
                action_kind: "offline_wlan_fixture_audit".to_string(),
                target: site_domain,
                noise_score: 10,
            }],
            handoff: None,
            usage: ResourceUsage {
                modules_executed: 1,
                tool_calls: 2,
                cost_units: 8,
                time_ms: 180,
            },
            remediation_notes: vec![
                "Áp dụng WPA3/PMF bắt buộc và kiểm tra đối chiếu BSSID hợp lệ trên WIDS"
                    .to_string(),
            ],
            cleanup_verified: true,
            remaining_artifacts: 0,
        })
    }
}

/// Adapter 3: Kiểm kê Active Directory trên snapshot tổng hợp (không gọi LDAP/Kerberos thật).
#[derive(Debug, Clone, Default)]
pub struct AdEnumerationModuleAdapter;

impl PentestModule for AdEnumerationModuleAdapter {
    fn id(&self) -> &'static str {
        "mod.ad.enum"
    }

    fn run(
        &mut self,
        context: &EngagementContext,
        input: ModuleInput,
    ) -> Result<ModuleOutput, ModuleError> {
        context.check_stage_approval(Stage::AdEnumeration)?;
        input.validate_no_secrets()?;
        let ModuleInput::AdEnumeration(ad) = input else {
            return Err(ModuleError::InvalidInput(
                "AdEnumerationModuleAdapter chỉ nhận ModuleInput::AdEnumeration".to_string(),
            ));
        };
        let domain = context
            .scope
            .check_domain(&ad.domain, &context.current_virtual_utc)?;
        context.check_stage_approval_for_target(Stage::AdEnumeration, &domain)?;
        let dc_ip = context.scope.check_ip(&ad.dc_ip)?;
        for h in &ad.hosts {
            context
                .scope
                .check_domain(h, &context.current_virtual_utc)?;
        }
        for edge in &ad.acl_edges {
            if edge.target_object.contains('.') {
                if parse_strict_ipv4(&edge.target_object).is_ok() {
                    context.scope.check_ip(&edge.target_object)?;
                } else {
                    context
                        .scope
                        .check_domain(&edge.target_object, &context.current_virtual_utc)?;
                }
            }
        }

        let ev = EvidenceRecord::from_raw(
            "EV-AD-ENUM-001",
            Stage::AdEnumeration,
            self.id(),
            &format!("virtual://{domain}/ad/snapshot"),
            &domain,
            &context.current_virtual_utc,
            &format!(
                "Synthetic AD snapshot dc_ip={dc_ip} hosts={} spn_candidates={} delegation_candidates={} acl_edges={}",
                ad.hosts.len(),
                ad.roastable_spn_accounts.len(),
                ad.constrained_delegation_accounts.len(),
                ad.acl_edges.len()
            ),
        )?;

        let mut findings = Vec::new();
        for (idx, edge) in ad.acl_edges.iter().enumerate() {
            let mut ev_ids = BTreeSet::new();
            ev_ids.insert(ev.evidence_id.clone());
            findings.push(CapstoneFinding {
                finding_id: format!("CAP-AD-ACL-{:03}", idx + 1),
                rule_id: "AD-SENSITIVE-ACL-EDGE".to_string(),
                root_cause_key: format!("acl-edge:{domain}:{}:{}", edge.right, edge.target_object),
                title: format!(
                    "Cạnh ACL `{}` từ `{}` tới `{}` cần thu hẹp đặc quyền",
                    edge.right, edge.source_principal, edge.target_object
                ),
                stage: Stage::AdEnumeration,
                affected_asset: domain.clone(),
                severity: FindingSeverity::Medium,
                cvss_tenths: 65,
                status: CapstoneFindingStatus::Verified,
                evidence_ids: ev_ids,
                telemetry_event_id: Some("TEL-AD-ENUM-001".to_string()),
                remediation:
                    "Thu hồi quyền WriteDacl/GenericAll thừa theo nguyên tắc least privilege"
                        .to_string(),
            });
        }

        let has_preconditions =
            !ad.roastable_spn_accounts.is_empty() || !ad.constrained_delegation_accounts.is_empty();
        let next_stage = if has_preconditions {
            Stage::AdConditionalValidation
        } else if ad.enable_post_audit_branch {
            Stage::PostExploitationAudit
        } else {
            Stage::Reporting
        };

        let digest = sha256_hex(
            format!(
                "ad-enum:{domain}:{dc_ip}:{}:{}",
                ad.roastable_spn_accounts.len(),
                ad.constrained_delegation_accounts.len()
            )
            .as_bytes(),
        );

        Ok(ModuleOutput {
            module_id: self.id(),
            executed_stage: Stage::AdEnumeration,
            findings,
            excluded_false_positives: Vec::new(),
            evidence: vec![ev],
            next_stage: Some(next_stage),
            checkpoint: StageCheckpoint {
                stage: Stage::AdEnumeration,
                checkpoint_tag: format!("chk-adenum-{}", &digest[..8]),
                output_digest_sha256: digest,
            },
            telemetry_events: vec![ModuleTelemetryEvent {
                event_id: "TEL-AD-ENUM-001".to_string(),
                stage: Stage::AdEnumeration,
                actor_module_id: self.id().to_string(),
                action_kind: "synthetic_ad_snapshot_parse".to_string(),
                target: domain.clone(),
                noise_score: 20,
            }],
            handoff: Some(ModuleHandoff::AdPreconditions {
                domain,
                dc_ip,
                candidate_spns: ad.roastable_spn_accounts,
                delegation_accounts: ad.constrained_delegation_accounts,
            }),
            usage: ResourceUsage {
                modules_executed: 1,
                tool_calls: 3,
                cost_units: 10,
                time_ms: 210,
            },
            remediation_notes: vec![
                "Kiểm tra định kỳ cạnh ACL hướng Tier-0 và cấu hình gMSA cho tài khoản dịch vụ"
                    .to_string(),
            ],
            cleanup_verified: true,
            remaining_artifacts: 0,
        })
    }
}

/// Adapter 4: Kiểm chứng AD có điều kiện (Roasting/Delegation precondition tổng hợp + mô phỏng fail_once).
#[derive(Debug, Clone, Default)]
pub struct AdConditionalAttackAdapter {
    /// Cờ giả lập lỗi tạm thời ở lần chạy đầu tiên để kiểm thử `Checkpoint` / `resume`.
    pub fail_once: bool,
    /// Đã kích hoạt lỗi giả lập một lần hay chưa.
    pub triggered_once: bool,
}

impl AdConditionalAttackAdapter {
    /// Khởi tạo adapter chạy thẳng không tiêm lỗi.
    pub fn new() -> Self {
        Self::default()
    }

    /// Khởi tạo adapter tiêm đúng 1 lỗi tạm thời ở lần gọi đầu tiên.
    pub fn with_transient_failure_once() -> Self {
        Self {
            fail_once: true,
            triggered_once: false,
        }
    }
}

impl PentestModule for AdConditionalAttackAdapter {
    fn id(&self) -> &'static str {
        "mod.ad.conditional"
    }

    fn run(
        &mut self,
        context: &EngagementContext,
        input: ModuleInput,
    ) -> Result<ModuleOutput, ModuleError> {
        context.check_stage_approval(Stage::AdConditionalValidation)?;
        input.validate_no_secrets()?;
        let ModuleInput::AdConditionalValidation(cond) = input else {
            return Err(ModuleError::InvalidInput(
                "AdConditionalAttackAdapter chỉ nhận ModuleInput::AdConditionalValidation"
                    .to_string(),
            ));
        };
        let domain = context
            .scope
            .check_domain(&cond.domain, &context.current_virtual_utc)?;
        context.check_stage_approval_for_target(Stage::AdConditionalValidation, &domain)?;
        context.scope.check_ip(&cond.dc_ip)?;

        if cond.candidate_spns.is_empty() && cond.delegation_accounts.is_empty() {
            return Err(ModuleError::InvalidInput(
                "pha AdConditionalValidation chỉ được gọi khi có ứng viên precondition".to_string(),
            ));
        }

        if self.fail_once && !self.triggered_once {
            self.triggered_once = true;
            return Err(ModuleError::TransientFailure(
                "kết nối bộ nạp snapshot ảo tạm gián đoạn — an toàn để phục hồi từ checkpoint"
                    .to_string(),
            ));
        }

        let (action_kind, noise_score, ev_telemetry_id) = if cond.noisy_sweep_mode {
            (
                "bulk_spn_sweep".to_string(),
                92u8,
                "TEL-AD-NOISY-SWEEP".to_string(),
            )
        } else {
            (
                "scoped_spn_metadata_check".to_string(),
                30u8,
                "TEL-AD-TARGETED".to_string(),
            )
        };

        let ev = EvidenceRecord::from_raw(
            "EV-AD-COND-001",
            Stage::AdConditionalValidation,
            self.id(),
            &format!("virtual://{domain}/ad/precondition-check"),
            &domain,
            &context.current_virtual_utc,
            &format!(
                "SYNTHETIC-TICKET-VERIFIER check mode={action_kind} spns={:?} delegation={:?} (no real ticket/cracking)",
                cond.candidate_spns, cond.delegation_accounts
            ),
        )?;

        let mut ev_ids = BTreeSet::new();
        ev_ids.insert(ev.evidence_id.clone());

        let mut findings = Vec::new();
        if !cond.candidate_spns.is_empty() {
            findings.push(CapstoneFinding {
                finding_id: "CAP-AD-SPN-001".to_string(),
                rule_id: "AD-KERBEROAST-PRECONDITION".to_string(),
                root_cause_key: format!("spn-rc4-policy:{domain}"),
                title: format!(
                    "Precondition Kerberoasting tổng hợp trên tài khoản dịch vụ {}",
                    cond.candidate_spns[0]
                ),
                stage: Stage::AdConditionalValidation,
                affected_asset: domain.clone(),
                severity: FindingSeverity::High,
                cvss_tenths: 81,
                status: CapstoneFindingStatus::Verified,
                evidence_ids: ev_ids.clone(),
                telemetry_event_id: Some(ev_telemetry_id.clone()),
                remediation: "Chuyển sang gMSA 120 byte và ép AES256-CTS-HMAC-SHA1-96 cho TGS"
                    .to_string(),
            });
        }
        if !cond.delegation_accounts.is_empty() {
            findings.push(CapstoneFinding {
                finding_id: "CAP-AD-DELEG-001".to_string(),
                rule_id: "AD-CONSTRAINED-DELEGATION".to_string(),
                root_cause_key: format!("delegation:{domain}"),
                title: format!(
                    "Cấu hình ủy quyền Kerberos cần thu hẹp trên {}",
                    cond.delegation_accounts[0]
                ),
                stage: Stage::AdConditionalValidation,
                affected_asset: domain.clone(),
                severity: FindingSeverity::High,
                cvss_tenths: 78,
                status: CapstoneFindingStatus::Verified,
                evidence_ids: ev_ids,
                telemetry_event_id: Some(ev_telemetry_id.clone()),
                remediation: "Giới hạn danh sách SPN đích được phép và bảo vệ tài khoản nhạy cảm"
                    .to_string(),
            });
        }

        let next_stage = if cond.enable_post_audit_branch {
            Stage::PostExploitationAudit
        } else {
            Stage::Reporting
        };
        let digest = sha256_hex(
            format!(
                "ad-cond:{domain}:{action_kind}:{}",
                cond.candidate_spns.len() + cond.delegation_accounts.len()
            )
            .as_bytes(),
        );

        Ok(ModuleOutput {
            module_id: self.id(),
            executed_stage: Stage::AdConditionalValidation,
            findings,
            excluded_false_positives: Vec::new(),
            evidence: vec![ev],
            next_stage: Some(next_stage),
            checkpoint: StageCheckpoint {
                stage: Stage::AdConditionalValidation,
                checkpoint_tag: format!("chk-adcond-{}", &digest[..8]),
                output_digest_sha256: digest,
            },
            telemetry_events: vec![ModuleTelemetryEvent {
                event_id: ev_telemetry_id,
                stage: Stage::AdConditionalValidation,
                actor_module_id: self.id().to_string(),
                action_kind,
                target: domain,
                noise_score,
            }],
            handoff: None,
            usage: ResourceUsage {
                modules_executed: 1,
                tool_calls: if cond.noisy_sweep_mode { 10 } else { 2 },
                cost_units: 12,
                time_ms: 260,
            },
            remediation_notes: vec![
                "Triển khai gMSA/AES256 và siết Event 4769 RC4 telemetry trên Tier-0".to_string(),
            ],
            cleanup_verified: true,
            remaining_artifacts: 0,
        })
    }
}

/// Adapter 5 (Nhánh tùy chọn): Kiểm kê hậu khai thác & xác minh dọn dẹp lũy đẳng trên `virtual://`.
#[derive(Debug, Clone, Default)]
pub struct PostExploitationAuditAdapter;

impl PentestModule for PostExploitationAuditAdapter {
    fn id(&self) -> &'static str {
        "mod.post.audit"
    }

    fn run(
        &mut self,
        context: &EngagementContext,
        input: ModuleInput,
    ) -> Result<ModuleOutput, ModuleError> {
        context.check_stage_approval(Stage::PostExploitationAudit)?;
        input.validate_no_secrets()?;
        let ModuleInput::PostExploitationAudit(post) = input else {
            return Err(ModuleError::InvalidInput(
                "PostExploitationAuditAdapter chỉ nhận ModuleInput::PostExploitationAudit"
                    .to_string(),
            ));
        };
        let host_domain = context
            .scope
            .check_domain(&post.host_domain, &context.current_virtual_utc)?;
        context.check_stage_approval_for_target(Stage::PostExploitationAudit, &host_domain)?;
        context.scope.check_ip(&post.host_ip)?;

        let mut drift_count = 0usize;
        for item in &post.persistence_items {
            validate_synthetic_location_in_scope(
                &item.virtual_location,
                &context.scope,
                &context.current_virtual_utc,
            )?;
            if !item.in_baseline {
                drift_count += 1;
            }
        }

        let remaining_artifacts = if post.simulate_dirty_cleanup {
            drift_count.max(1)
        } else {
            0
        };
        let cleanup_verified = remaining_artifacts == 0;

        let ev = EvidenceRecord::from_raw(
            "EV-POST-001",
            Stage::PostExploitationAudit,
            self.id(),
            &format!("virtual://{host_domain}/audit/cleanup"),
            &host_domain,
            &context.current_virtual_utc,
            &format!(
                "Baseline diff drift_items={drift_count} telemetry_gaps={} remaining_artifacts={remaining_artifacts}",
                post.telemetry_gaps.len()
            ),
        )?;

        let mut ev_ids = BTreeSet::new();
        ev_ids.insert(ev.evidence_id.clone());

        let mut findings = Vec::new();
        if !post.telemetry_gaps.is_empty() {
            findings.push(CapstoneFinding {
                finding_id: "CAP-POST-TEL-001".to_string(),
                rule_id: "POST-TELEMETRY-GAP".to_string(),
                root_cause_key: format!("telemetry-gap:{host_domain}"),
                title: format!(
                    "Thiếu giám sát telemetry cho chính sách bảo mật: {}",
                    post.telemetry_gaps[0]
                ),
                stage: Stage::PostExploitationAudit,
                affected_asset: host_domain.clone(),
                severity: FindingSeverity::Medium,
                cvss_tenths: 55,
                status: CapstoneFindingStatus::Verified,
                evidence_ids: ev_ids,
                telemetry_event_id: Some("TEL-POST-001".to_string()),
                remediation: "Khôi phục Audit Policy & Sysmon baseline và đối chiếu SIEM"
                    .to_string(),
            });
        }

        if !cleanup_verified {
            return Err(ModuleError::CleanupIncomplete(format!(
                "còn sót {remaining_artifacts} artifact hậu kiểm thử trên `{host_domain}`"
            )));
        }

        let digest = sha256_hex(
            format!("post-audit:{host_domain}:{drift_count}:{remaining_artifacts}").as_bytes(),
        );
        Ok(ModuleOutput {
            module_id: self.id(),
            executed_stage: Stage::PostExploitationAudit,
            findings,
            excluded_false_positives: Vec::new(),
            evidence: vec![ev],
            next_stage: Some(Stage::Reporting),
            checkpoint: StageCheckpoint {
                stage: Stage::PostExploitationAudit,
                checkpoint_tag: format!("chk-post-{}", &digest[..8]),
                output_digest_sha256: digest,
            },
            telemetry_events: vec![ModuleTelemetryEvent {
                event_id: "TEL-POST-001".to_string(),
                stage: Stage::PostExploitationAudit,
                actor_module_id: self.id().to_string(),
                action_kind: "readonly_snapshot_diff_cleanup".to_string(),
                target: host_domain,
                noise_score: 8,
            }],
            handoff: Some(ModuleHandoff::CleanupConfirmed {
                removed_items: drift_count,
                remaining_artifacts,
            }),
            usage: ResourceUsage {
                modules_executed: 1,
                tool_calls: 2,
                cost_units: 7,
                time_ms: 150,
            },
            remediation_notes: vec![
                "Đối chiếu định kỳ baseline persistence và khóa toàn vẹn cấu hình log".to_string(),
            ],
            cleanup_verified: true,
            remaining_artifacts: 0,
        })
    }
}

/// Adapter 6: Tổng hợp kiểm tra sạch sẽ & xác nhận sẵn sàng xuất báo cáo cuối.
#[derive(Debug, Clone, Default)]
pub struct ReportingModuleAdapter;

impl PentestModule for ReportingModuleAdapter {
    fn id(&self) -> &'static str {
        "mod.reporting.final"
    }

    fn run(
        &mut self,
        context: &EngagementContext,
        input: ModuleInput,
    ) -> Result<ModuleOutput, ModuleError> {
        context.check_stage_approval(Stage::Reporting)?;
        input.validate_no_secrets()?;
        let ModuleInput::Reporting(rep) = input else {
            return Err(ModuleError::InvalidInput(
                "ReportingModuleAdapter chỉ nhận ModuleInput::Reporting".to_string(),
            ));
        };
        if rep.remaining_artifacts > 0 {
            return Err(ModuleError::CleanupIncomplete(format!(
                "không thể hoàn tất báo cáo khi còn {} artifact phòng lab chưa dọn",
                rep.remaining_artifacts
            )));
        }

        let target_domain = context
            .scope
            .allowed_domains
            .iter()
            .next()
            .or_else(|| context.scope.allow_subdomains_of.iter().next())
            .map(String::as_str)
            .unwrap_or("lab.example");
        context.check_stage_approval_for_target(Stage::Reporting, target_domain)?;

        let ev = EvidenceRecord::from_raw(
            "EV-REPORT-001",
            Stage::Reporting,
            self.id(),
            &format!("virtual://{target_domain}/reporting/seal"),
            target_domain,
            &context.current_virtual_utc,
            &format!(
                "Report assembly title='{}' prior_evidence={} prior_tip={} verified_findings={}",
                rep.report_title, rep.ledger_count, rep.ledger_tip_hash, rep.verified_finding_count
            ),
        )?;

        let digest = sha256_hex(
            format!(
                "report-stage:{}:{}:{}",
                rep.report_title, rep.ledger_tip_hash, rep.verified_finding_count
            )
            .as_bytes(),
        );
        Ok(ModuleOutput {
            module_id: self.id(),
            executed_stage: Stage::Reporting,
            findings: Vec::new(),
            excluded_false_positives: Vec::new(),
            evidence: vec![ev],
            next_stage: Some(Stage::Completed),
            checkpoint: StageCheckpoint {
                stage: Stage::Reporting,
                checkpoint_tag: format!("chk-report-{}", &digest[..8]),
                output_digest_sha256: digest,
            },
            telemetry_events: vec![ModuleTelemetryEvent {
                event_id: "TEL-REPORT-001".to_string(),
                stage: Stage::Reporting,
                actor_module_id: self.id().to_string(),
                action_kind: "deterministic_report_seal".to_string(),
                target: target_domain.to_string(),
                noise_score: 0,
            }],
            handoff: None,
            usage: ResourceUsage {
                modules_executed: 1,
                tool_calls: 1,
                cost_units: 3,
                time_ms: 80,
            },
            remediation_notes: Vec::new(),
            cleanup_verified: true,
            remaining_artifacts: 0,
        })
    }
}

// ============================================================================
// 7. MODULE REGISTRY, ĐỒ THỊ PHỤ THUỘC & KIỂM TRA TƯƠNG THÍCH INPUT/OUTPUT
// ============================================================================

/// Mô tả tĩnh của một module trong `ModuleRegistry`.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct ModuleDescriptor {
    /// Định danh module khớp với `PentestModule::id()`.
    pub expected_id: &'static str,
    /// Pha do module đảm nhiệm.
    pub stage: Stage,
    /// Kiểu `ModuleInputKind` hợp lệ duy nhất mà module nhận.
    pub accepted_input: ModuleInputKind,
    /// Các pha tiên quyết bắt buộc phải hoàn tất trước pha này.
    pub required_prerequisites: Vec<Stage>,
    /// Tập các pha kế tiếp được phép trả về từ module này.
    pub allowed_next_stages: Vec<Stage>,
}

struct RegisteredModule {
    descriptor: ModuleDescriptor,
    instance: Box<dyn PentestModule>,
}

/// Kho đăng ký các module kiểm thử định kiểu kèm kiểm tra tương thích & đếm số lần gọi thực tế.
pub struct ModuleRegistry {
    entries: BTreeMap<Stage, RegisteredModule>,
    invocations_by_stage: BTreeMap<Stage, usize>,
}

impl ModuleRegistry {
    /// Tạo registry trống.
    pub fn new() -> Self {
        Self {
            entries: BTreeMap::new(),
            invocations_by_stage: BTreeMap::new(),
        }
    }

    /// Đăng ký một module mới cùng mô tả phụ thuộc và tương thích kiểu.
    pub fn register(
        &mut self,
        descriptor: ModuleDescriptor,
        instance: Box<dyn PentestModule>,
    ) -> Result<(), OrchestratorError> {
        if descriptor.expected_id != instance.id() {
            return Err(OrchestratorError::IncompatibleModuleOrInput(format!(
                "descriptor id `{}` khác module.id() `{}`",
                descriptor.expected_id,
                instance.id()
            )));
        }
        self.entries.insert(
            descriptor.stage,
            RegisteredModule {
                descriptor,
                instance,
            },
        );
        Ok(())
    }

    /// Tổng số lần bất kỳ module nào được gọi thực thi thực sự.
    pub fn total_invocations(&self) -> usize {
        self.invocations_by_stage.values().sum()
    }

    /// Số lần gọi của riêng một pha cụ thể (dùng chứng minh resume không chạy lại pha đã commit).
    pub fn invocations_for(&self, stage: Stage) -> usize {
        self.invocations_by_stage.get(&stage).copied().unwrap_or(0)
    }

    /// Kiểm tra đủ module, đúng `accepted_input` và đủ các pha tiên quyết trước khi gọi `.run()`.
    pub fn check_compatibility(
        &self,
        stage: Stage,
        input: &ModuleInput,
        completed_stages: &[Stage],
    ) -> Result<(), OrchestratorError> {
        let entry = self.entries.get(&stage).ok_or_else(|| {
            OrchestratorError::IncompatibleModuleOrInput(format!(
                "không có module nào đăng ký cho pha {stage:?}"
            ))
        })?;
        if input.kind() != entry.descriptor.accepted_input {
            return Err(OrchestratorError::IncompatibleModuleOrInput(format!(
                "pha {stage:?} yêu cầu input {:?}, nhận {:?}",
                entry.descriptor.accepted_input,
                input.kind()
            )));
        }
        for req in &entry.descriptor.required_prerequisites {
            if !completed_stages.contains(req) {
                return Err(OrchestratorError::IncompatibleModuleOrInput(format!(
                    "pha {stage:?} thiếu pha tiên quyết {req:?}"
                )));
            }
        }
        Ok(())
    }

    /// Thực thi module của một pha và xác thực `ModuleOutput` khớp hợp đồng đồ thị chuyển pha.
    pub fn execute_stage(
        &mut self,
        stage: Stage,
        context: &EngagementContext,
        input: ModuleInput,
        completed_stages: &[Stage],
    ) -> Result<ModuleOutput, OrchestratorError> {
        self.check_compatibility(stage, &input, completed_stages)?;
        let entry = self.entries.get_mut(&stage).ok_or_else(|| {
            OrchestratorError::IncompatibleModuleOrInput(format!("thiếu module cho pha {stage:?}"))
        })?;
        *self.invocations_by_stage.entry(stage).or_insert(0) += 1;
        let output = entry
            .instance
            .run(context, input)
            .map_err(|err| OrchestratorError::ModuleFailed { stage, error: err })?;

        if output.executed_stage != stage || output.module_id != entry.descriptor.expected_id {
            return Err(OrchestratorError::IncompatibleModuleOrInput(format!(
                "module `{}` trả pha {:?} lệch với {stage:?}",
                output.module_id, output.executed_stage
            )));
        }
        if let Some(next) = output.next_stage {
            if !stage.can_transition_to(next)
                || !entry.descriptor.allowed_next_stages.contains(&next)
            {
                return Err(OrchestratorError::InvalidStageTransition {
                    from: stage,
                    to: next,
                });
            }
        } else {
            return Err(OrchestratorError::InvalidStageTransition {
                from: stage,
                to: Stage::Completed,
            });
        }
        Ok(output)
    }
}

impl Default for ModuleRegistry {
    fn default() -> Self {
        Self::new()
    }
}

// ============================================================================
// 8. KỊCH BẢN, CHECKPOINT/RESUME, ORCHESTRATOR & REPORT + EVALUATOR
// ============================================================================

/// Toàn bộ đầu vào tổng hợp của một kịch bản Capstone có thể tái lập 100% theo byte.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct CapstoneScenario {
    /// Tiêu đề báo cáo.
    pub report_title: String,
    /// Mồi khởi đầu OSINT (bắt buộc).
    pub osint: OsintSeedInput,
    /// Fixture không dây (tùy chọn khi kích hoạt nhánh OSWP).
    pub wireless: Option<WirelessAssessmentInput>,
    /// Snapshot AD Enumeration (bắt buộc).
    pub ad_enumeration: AdEnumerationInput,
    /// Cờ chế độ ồn cao ở bước `AdConditionalValidation` dùng cho Red-vs-Blue.
    pub noisy_ad_sweep: bool,
    /// Fixture kiểm kê hậu khai thác (tùy chọn khi kích hoạt nhánh post-audit).
    pub post_audit: Option<PostAuditInput>,
}

fn push_lp_list<'a, I>(buf: &mut String, iter: I)
where
    I: IntoIterator<Item = &'a str>,
{
    let items: Vec<&str> = iter.into_iter().collect();
    push_lp(buf, &items.len().to_string());
    for item in items {
        push_lp(buf, item);
    }
}

fn push_finding_lp(buf: &mut String, f: &CapstoneFinding) {
    push_lp(buf, &f.finding_id);
    push_lp(buf, &f.rule_id);
    push_lp(buf, &f.root_cause_key);
    push_lp(buf, &f.title);
    push_lp(buf, f.stage.as_str());
    push_lp(buf, &f.affected_asset);
    push_lp(buf, f.severity.as_str());
    push_lp(buf, &f.cvss_tenths.to_string());
    push_lp(buf, f.status.as_str());
    push_lp_list(buf, f.evidence_ids.iter().map(String::as_str));
    push_lp(buf, f.telemetry_event_id.as_deref().unwrap_or(""));
    push_lp(buf, &f.remediation);
}

fn push_blocked_action_lp(buf: &mut String, b: &BlockedActionRecord) {
    push_lp(buf, b.stage.as_str());
    push_lp(buf, &b.module_id);
    push_lp(buf, &b.telemetry_event_id);
    push_lp(buf, &b.action_kind);
    push_lp(buf, &b.target);
    push_lp(buf, &b.reason);
}

fn push_blue_alert_lp(buf: &mut String, a: &BlueTelemetryAlert) {
    push_lp(buf, &a.alert_id);
    push_lp(buf, &a.telemetry_event_id);
    push_lp(buf, a.stage.as_str());
    push_lp(buf, &format!("{:?}", a.disposition));
    push_lp(buf, &a.summary);
}

fn push_trace_lp(buf: &mut String, t: &ModuleTraceEntry) {
    push_lp(buf, &t.step_index.to_string());
    push_lp(buf, t.stage.as_str());
    push_lp(buf, &t.module_id);
    push_lp(buf, t.next_stage.as_str());
    push_lp(buf, &t.evidence_added.to_string());
    push_lp(buf, &t.verified_findings_added.to_string());
    push_lp(buf, &t.blocked_actions.to_string());
    push_lp(buf, &t.stage_checkpoint_tag);
}

fn push_cleanup_lp(buf: &mut String, c: &CleanupSummary) {
    push_lp(buf, &c.real_host_artifacts_created.to_string());
    push_lp(buf, &c.remaining_lab_artifacts.to_string());
    push_lp(buf, &c.verified_clean.to_string());
    push_lp(buf, &c.notes);
}

impl CapstoneScenario {
    /// Tính vân tay SHA-256 đóng khung độ dài cho toàn bộ cấu hình kịch bản & ngữ cảnh phạm vi.
    pub fn fingerprint_sha256(&self, context: &EngagementContext) -> String {
        let mut buf = String::new();
        push_lp(&mut buf, &context.engagement_id);
        push_lp(&mut buf, &context.authorization_ref);
        push_lp(&mut buf, &context.signed_written_consent.to_string());
        push_lp(&mut buf, &context.campaign_approved.to_string());
        push_lp(&mut buf, &context.window_start_utc);
        push_lp(&mut buf, &context.window_end_utc);
        push_lp(&mut buf, &context.current_virtual_utc);
        push_lp(&mut buf, &context.seed.to_string());
        push_lp_list(
            &mut buf,
            context.scope.allowed_domains.iter().map(String::as_str),
        );
        push_lp_list(
            &mut buf,
            context.scope.allow_subdomains_of.iter().map(String::as_str),
        );
        push_lp_list(
            &mut buf,
            context.scope.denied_domains.iter().map(String::as_str),
        );
        push_lp_list(
            &mut buf,
            context.scope.allowed_cidrs.iter().map(String::as_str),
        );
        push_lp_list(
            &mut buf,
            context.scope.denied_ips.iter().map(String::as_str),
        );
        push_lp(
            &mut buf,
            &context.scope.third_party_assets.len().to_string(),
        );
        for (domain, tp) in &context.scope.third_party_assets {
            push_lp(&mut buf, domain);
            push_lp(&mut buf, &tp.written_consent_ref);
            push_lp(&mut buf, &tp.approved_until_utc);
        }
        push_lp(
            &mut buf,
            &context.required_stage_approvals.len().to_string(),
        );
        for st in &context.required_stage_approvals {
            push_lp(&mut buf, st.as_str());
        }
        push_lp(&mut buf, &context.budget.max_modules.to_string());
        push_lp(&mut buf, &context.budget.max_tool_calls.to_string());
        push_lp(&mut buf, &context.budget.max_cost_units.to_string());
        push_lp(&mut buf, &context.budget.max_time_ms.to_string());

        push_lp(&mut buf, &self.report_title);
        push_lp(&mut buf, &self.osint.root_domain);
        push_lp(&mut buf, &self.osint.org_handle);
        push_lp(&mut buf, &self.osint.primary_ip);
        push_lp_list(
            &mut buf,
            self.osint
                .passive_sources
                .iter()
                .map(PassiveSourceKind::as_str),
        );
        push_lp(&mut buf, &self.osint.enable_wireless_branch.to_string());

        if let Some(w) = &self.wireless {
            push_lp(&mut buf, "1");
            push_lp(&mut buf, &w.ssid);
            push_lp(&mut buf, &w.site_domain);
            push_lp(&mut buf, &w.controller_ip);
            push_lp(&mut buf, &w.expected_bssid);
            push_lp(&mut buf, w.observed_second_bssid.as_deref().unwrap_or(""));
            push_lp(&mut buf, &w.second_bssid_authorized.to_string());
            push_lp(&mut buf, &w.snr_db.to_string());
            push_lp(&mut buf, &format!("{:?}", w.observed_suite));
            push_lp(&mut buf, &w.pmkid_present.to_string());
        } else {
            push_lp(&mut buf, "0");
        }

        push_lp(&mut buf, &self.ad_enumeration.domain);
        push_lp(&mut buf, &self.ad_enumeration.dc_ip);
        push_lp_list(
            &mut buf,
            self.ad_enumeration.hosts.iter().map(String::as_str),
        );
        push_lp_list(
            &mut buf,
            self.ad_enumeration
                .roastable_spn_accounts
                .iter()
                .map(String::as_str),
        );
        push_lp_list(
            &mut buf,
            self.ad_enumeration
                .constrained_delegation_accounts
                .iter()
                .map(String::as_str),
        );
        push_lp(&mut buf, &self.ad_enumeration.acl_edges.len().to_string());
        for edge in &self.ad_enumeration.acl_edges {
            push_lp(&mut buf, &edge.source_principal);
            push_lp(&mut buf, &edge.right);
            push_lp(&mut buf, &edge.target_object);
        }
        push_lp(
            &mut buf,
            &self.ad_enumeration.enable_post_audit_branch.to_string(),
        );
        push_lp(&mut buf, &self.noisy_ad_sweep.to_string());

        if let Some(p) = &self.post_audit {
            push_lp(&mut buf, "1");
            push_lp(&mut buf, &p.host_domain);
            push_lp(&mut buf, &p.host_ip);
            push_lp(&mut buf, &p.persistence_items.len().to_string());
            for item in &p.persistence_items {
                push_lp(&mut buf, &item.item_id);
                push_lp(&mut buf, &item.virtual_location);
                push_lp(&mut buf, &item.mechanism);
                push_lp(&mut buf, &item.in_baseline.to_string());
            }
            push_lp_list(&mut buf, p.telemetry_gaps.iter().map(String::as_str));
            push_lp(&mut buf, &p.simulate_dirty_cleanup.to_string());
        } else {
            push_lp(&mut buf, "0");
        }

        sha256_hex(buf.as_bytes())
    }
}

/// Vết thực thi của một module đã commit vào chiến dịch.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct ModuleTraceEntry {
    /// Số thứ tự bước.
    pub step_index: usize,
    /// Pha đã thực thi.
    pub stage: Stage,
    /// ID module thực thi.
    pub module_id: String,
    /// Pha kế tiếp được quyết định.
    pub next_stage: Stage,
    /// Số bằng chứng ghi nhận tại bước.
    pub evidence_added: usize,
    /// Số finding xác minh thêm vào ở bước.
    pub verified_findings_added: usize,
    /// Số hành động Red bị Blue chặn ở bước.
    pub blocked_actions: usize,
    /// Tag checkpoint nội bộ của bước.
    pub stage_checkpoint_tag: String,
}

/// Tóm tắt dọn dẹp phòng lab cuối báo cáo.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct CleanupSummary {
    /// Số artifact thật từng tạo trên hệ điều hành ngoài (luôn = 0).
    pub real_host_artifacts_created: usize,
    /// Số artifact phòng lab ảo còn sót lại sau bước cleanup (bắt buộc = 0).
    pub remaining_lab_artifacts: usize,
    /// Trạng thái xác minh sạch.
    pub verified_clean: bool,
    /// Ghi chú chi tiết dọn dẹp.
    pub notes: String,
}

impl Default for CleanupSummary {
    fn default() -> Self {
        Self {
            real_host_artifacts_created: 0,
            remaining_lab_artifacts: 0,
            verified_clean: true,
            notes: "Không tạo tệp hay dịch vụ trên máy thật; toàn bộ mô phỏng sạch 100%"
                .to_string(),
        }
    }
}

/// Checkpoint cấp Orchestrator có phiên bản, sổ cái nguyên vẹn và MAC SHA-256.
#[derive(Debug, Clone, PartialEq)]
pub struct OrchestratorCheckpoint {
    /// Phiên bản schema checkpoint.
    pub checkpoint_version: u32,
    /// Mã engagement sở hữu checkpoint.
    pub engagement_id: String,
    /// Vân tay kịch bản tránh phục hồi nhầm giữa hai cấu hình khác nhau.
    pub scenario_fingerprint_sha256: String,
    /// Các pha đã commit thành công (tuyệt đối không chạy lại khi resume).
    pub completed_stages: Vec<Stage>,
    /// Pha đang chờ thực thi kế tiếp.
    pub next_stage: Stage,
    /// Trạng thái bàn giao dữ liệu đã thu được trước pha kế tiếp.
    pub ad_preconditions_handoff: Option<(Vec<String>, Vec<String>)>,
    /// Sổ cái bằng chứng hiện hành.
    pub evidence_ledger: EvidenceLedger,
    /// Các finding xác minh đã tích lũy.
    pub verified_findings: Vec<CapstoneFinding>,
    /// Các false-positive đã tách riêng.
    pub excluded_false_positives: Vec<CapstoneFinding>,
    /// Danh sách hành động bị Blue chặn.
    pub blocked_actions: Vec<BlockedActionRecord>,
    /// Cảnh báo telemetry Blue SOC.
    pub blue_alerts: Vec<BlueTelemetryAlert>,
    /// Ghi chú khắc phục tích lũy.
    pub remediation_notes: Vec<String>,
    /// Dấu vết module đã commit.
    pub module_trace: Vec<ModuleTraceEntry>,
    /// Lịch sử chấm thưởng RL từng bước.
    pub rl_steps: Vec<RlStepRecord>,
    /// Tài nguyên tiêu thụ tích lũy.
    pub usage: ResourceUsage,
    /// Tổng kết dọn dẹp hiện tại.
    pub cleanup: CleanupSummary,
    /// Trạng thái PRNG tất định.
    pub rng_state: u64,
    /// MAC SHA-256 chống giả mạo checkpoint.
    pub checkpoint_mac_sha256: String,
}

impl OrchestratorCheckpoint {
    /// Tính MAC SHA-256 đóng khung độ dài cho toàn bộ trạng thái đã lưu.
    pub fn compute_mac(&self) -> String {
        let mut buf = String::new();
        push_lp(&mut buf, &self.checkpoint_version.to_string());
        push_lp(&mut buf, &self.engagement_id);
        push_lp(&mut buf, &self.scenario_fingerprint_sha256);
        push_lp_list(&mut buf, self.completed_stages.iter().map(Stage::as_str));
        push_lp(&mut buf, self.next_stage.as_str());
        if let Some((spns, delegs)) = &self.ad_preconditions_handoff {
            push_lp(&mut buf, "1");
            push_lp_list(&mut buf, spns.iter().map(String::as_str));
            push_lp_list(&mut buf, delegs.iter().map(String::as_str));
        } else {
            push_lp(&mut buf, "0");
        }
        push_lp(&mut buf, &self.evidence_ledger.tip_hash());
        push_lp(&mut buf, &self.evidence_ledger.records().len().to_string());
        for ev in self.evidence_ledger.records() {
            push_lp(&mut buf, &ev.evidence_id);
            push_lp(&mut buf, &ev.record_hash);
        }
        push_lp(&mut buf, &self.verified_findings.len().to_string());
        for f in &self.verified_findings {
            push_finding_lp(&mut buf, f);
        }
        push_lp(&mut buf, &self.excluded_false_positives.len().to_string());
        for f in &self.excluded_false_positives {
            push_finding_lp(&mut buf, f);
        }
        push_lp(&mut buf, &self.blocked_actions.len().to_string());
        for b in &self.blocked_actions {
            push_blocked_action_lp(&mut buf, b);
        }
        push_lp(&mut buf, &self.blue_alerts.len().to_string());
        for a in &self.blue_alerts {
            push_blue_alert_lp(&mut buf, a);
        }
        push_lp_list(&mut buf, self.remediation_notes.iter().map(String::as_str));
        push_lp(&mut buf, &self.module_trace.len().to_string());
        for t in &self.module_trace {
            push_trace_lp(&mut buf, t);
        }
        push_lp(&mut buf, &self.rl_steps.len().to_string());
        for s in &self.rl_steps {
            push_lp(&mut buf, s.stage.as_str());
            push_lp(&mut buf, &s.outcome.verified_findings_added.to_string());
            push_lp(&mut buf, &s.outcome.clean_report_achieved.to_string());
            push_lp(&mut buf, &s.outcome.detected_by_blue.to_string());
            push_lp(&mut buf, &s.outcome.blocked_by_blue_count.to_string());
            push_lp(&mut buf, &s.outcome.service_disrupted.to_string());
            push_lp(&mut buf, &s.outcome.safety_violations.to_string());
            push_lp(&mut buf, &s.reward.to_bits().to_string());
        }
        push_lp(&mut buf, &self.usage.modules_executed.to_string());
        push_lp(&mut buf, &self.usage.tool_calls.to_string());
        push_lp(&mut buf, &self.usage.cost_units.to_string());
        push_lp(&mut buf, &self.usage.time_ms.to_string());
        push_cleanup_lp(&mut buf, &self.cleanup);
        push_lp(&mut buf, &self.rng_state.to_string());
        sha256_hex(buf.as_bytes())
    }

    /// Xác thực version, MAC SHA-256, fingerprint kịch bản, toàn vẹn sổ cái và sạch secret.
    pub fn verify(&self, expected_fingerprint: &str) -> Result<(), OrchestratorError> {
        if self.checkpoint_version != CHECKPOINT_VERSION {
            return Err(OrchestratorError::CheckpointInvalid(format!(
                "sai checkpoint_version {}, yêu cầu {}",
                self.checkpoint_version, CHECKPOINT_VERSION
            )));
        }
        if self.next_stage == Stage::Completed {
            return Err(OrchestratorError::CheckpointInvalid(
                "không thể resume từ checkpoint đã Completed".to_string(),
            ));
        }
        if self.scenario_fingerprint_sha256 != expected_fingerprint {
            return Err(OrchestratorError::CheckpointInvalid(
                "scenario_fingerprint_sha256 không khớp với kịch bản đang resume".to_string(),
            ));
        }
        if self.checkpoint_mac_sha256 != self.compute_mac() {
            return Err(OrchestratorError::CheckpointInvalid(
                "checkpoint_mac_sha256 không khớp — phát hiện sửa đổi trái phép".to_string(),
            ));
        }
        if self.cleanup.real_host_artifacts_created > 0 {
            return Err(OrchestratorError::CheckpointInvalid(
                "checkpoint ghi nhận real_host_artifacts_created > 0".to_string(),
            ));
        }
        validate_no_raw_secret("cp.engagement_id", &self.engagement_id)?;
        validate_no_raw_secret("cp.cleanup.notes", &self.cleanup.notes)?;
        for note in &self.remediation_notes {
            validate_no_raw_secret("cp.remediation_note", note)?;
        }
        for b in &self.blocked_actions {
            validate_no_raw_secret("cp.blocked.reason", &b.reason)?;
        }
        for a in &self.blue_alerts {
            validate_no_raw_secret("cp.alert.summary", &a.summary)?;
        }
        self.evidence_ledger.verify()?;
        for f in &self.verified_findings {
            validate_finding_provenance(f, &self.evidence_ledger)?;
        }
        for f in &self.excluded_false_positives {
            validate_finding_provenance(f, &self.evidence_ledger)?;
        }
        Ok(())
    }
}

/// Báo cáo cuối toàn vẹn của `ch112` gồm đủ 7 mục bắt buộc theo outline §10.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct CapstoneReport {
    /// Phiên bản schema báo cáo.
    pub schema_version: String,
    /// Mã engagement.
    pub engagement_id: String,
    /// Tiêu đề báo cáo.
    pub title: String,
    /// Mục 1: Tóm tắt phạm vi & ủy quyền.
    pub scope_summary: String,
    /// Mục 2: Vết thực thi các module theo đúng thứ tự chuyển pha.
    pub module_trace: Vec<ModuleTraceEntry>,
    /// Mục 3: Toàn bộ bằng chứng đã che thông tin nhạy cảm và nối chuỗi băm.
    pub evidence: Vec<EvidenceRecord>,
    /// Mục 4: Các phát hiện đã xác minh (đã khử trùng lặp).
    pub findings: Vec<CapstoneFinding>,
    /// Danh sách false-positive tách biệt khỏi rủi ro mở.
    pub excluded_false_positives: Vec<CapstoneFinding>,
    /// Các hành động Red bị Blue chặn (minh bạch không tính thành công).
    pub blocked_red_actions: Vec<BlockedActionRecord>,
    /// Cảnh báo telemetry của đội Xanh.
    pub blue_alerts: Vec<BlueTelemetryAlert>,
    /// Mục 5: Kế hoạch khắc phục tổng hợp đã khử trùng lặp.
    pub remediation_plan: Vec<String>,
    /// Mục 6: Tổng kết dọn dẹp phòng lab.
    pub cleanup: CleanupSummary,
    /// Mục 7: Giới hạn phạm vi và giả định mô phỏng (kể cả việc loại trừ Cloud pentest).
    pub limitations: Vec<String>,
    /// Tổng kết điểm RL tất định.
    pub rl_summary: RlEpisodeSummary,
    /// Hàm băm SHA-256 niêm phong toàn bộ nội dung báo cáo.
    pub report_digest_sha256: String,
}

fn json_escape(raw: &str) -> String {
    let mut out = String::with_capacity(raw.len() + 8);
    for ch in raw.chars() {
        match ch {
            '"' => out.push_str("\\\""),
            '\\' => out.push_str("\\\\"),
            '\n' => out.push_str("\\n"),
            '\r' => out.push_str("\\r"),
            '\t' => out.push_str("\\t"),
            c if (c as u32) < 0x20 => {
                use std::fmt::Write;
                let _ = write!(&mut out, "\\u{:04x}", c as u32);
            }
            c => out.push(c),
        }
    }
    out
}

fn md_escape_inline(raw: &str) -> String {
    let trimmed_start = raw.trim_start();
    let ordered_sep_offset = trimmed_start
        .find(|c: char| !c.is_ascii_digit())
        .filter(|&idx| {
            idx > 0
                && matches!(trimmed_start.as_bytes()[idx], b'.' | b')')
                && trimmed_start
                    .as_bytes()
                    .get(idx + 1)
                    .is_none_or(|b| *b == b' ' || *b == b'\t')
        })
        .map(|idx| (raw.len() - trimmed_start.len()) + idx);

    let mut out = String::with_capacity(raw.len() + 12);
    if trimmed_start.starts_with('#')
        || trimmed_start.starts_with('-')
        || trimmed_start.starts_with('+')
        || trimmed_start.starts_with('>')
    {
        out.push('\\');
    }
    for (byte_idx, ch) in raw.char_indices() {
        if ordered_sep_offset == Some(byte_idx) {
            out.push('\\');
        }
        match ch {
            '\\' => out.push_str("\\\\"),
            '*' => out.push_str("\\*"),
            '[' => out.push_str("\\["),
            ']' => out.push_str("\\]"),
            '<' => out.push_str("&lt;"),
            '>' => out.push_str("&gt;"),
            '|' => out.push_str("\\|"),
            '`' => out.push_str("\\`"),
            '\n' | '\r' => out.push(' '),
            c => out.push(c),
        }
    }
    out
}

impl CapstoneReport {
    /// Tính digest SHA-256 đóng khung độ dài trên toàn bộ nội dung báo cáo.
    pub fn compute_digest(&self) -> String {
        let mut buf = String::new();
        push_lp(&mut buf, &self.schema_version);
        push_lp(&mut buf, &self.engagement_id);
        push_lp(&mut buf, &self.title);
        push_lp(&mut buf, &self.scope_summary);
        push_lp(&mut buf, &self.module_trace.len().to_string());
        for t in &self.module_trace {
            push_trace_lp(&mut buf, t);
        }
        push_lp(&mut buf, &self.evidence.len().to_string());
        for ev in &self.evidence {
            push_lp(&mut buf, &ev.evidence_id);
            push_lp(&mut buf, ev.stage.as_str());
            push_lp(&mut buf, &ev.module_id);
            push_lp(&mut buf, &ev.source_uri);
            push_lp(&mut buf, &ev.target);
            push_lp(&mut buf, &ev.captured_at_utc);
            push_lp(&mut buf, &ev.redacted_summary);
            push_lp(&mut buf, &ev.redaction_count.to_string());
            push_lp(&mut buf, &ev.sequence.to_string());
            push_lp(&mut buf, &ev.prev_hash);
            push_lp(&mut buf, &ev.record_hash);
        }
        push_lp(&mut buf, &self.findings.len().to_string());
        for f in &self.findings {
            push_finding_lp(&mut buf, f);
        }
        push_lp(&mut buf, &self.excluded_false_positives.len().to_string());
        for f in &self.excluded_false_positives {
            push_finding_lp(&mut buf, f);
        }
        push_lp(&mut buf, &self.blocked_red_actions.len().to_string());
        for b in &self.blocked_red_actions {
            push_blocked_action_lp(&mut buf, b);
        }
        push_lp(&mut buf, &self.blue_alerts.len().to_string());
        for a in &self.blue_alerts {
            push_blue_alert_lp(&mut buf, a);
        }
        push_lp_list(&mut buf, self.remediation_plan.iter().map(String::as_str));
        push_cleanup_lp(&mut buf, &self.cleanup);
        push_lp_list(&mut buf, self.limitations.iter().map(String::as_str));
        push_lp(&mut buf, &self.rl_summary.total_steps.to_string());
        push_lp(&mut buf, &self.rl_summary.total_reward_milli.to_string());
        push_lp(&mut buf, &self.rl_summary.total_blue_blocks.to_string());
        push_lp(&mut buf, &self.rl_summary.total_blue_detections.to_string());
        push_lp(
            &mut buf,
            &self.rl_summary.total_verified_findings.to_string(),
        );
        sha256_hex(buf.as_bytes())
    }

    /// Kiểm tra schema, toàn vẹn digest, chuỗi băm bằng chứng, provenance và không rò rỉ secret.
    pub fn validate(&self) -> Result<(), OrchestratorError> {
        if self.schema_version != CAPSTONE_SCHEMA_VERSION {
            return Err(OrchestratorError::ReportTampered(format!(
                "schema_version phải là `{CAPSTONE_SCHEMA_VERSION}`, nhận `{}`",
                self.schema_version
            )));
        }
        if self.report_digest_sha256 != self.compute_digest() {
            return Err(OrchestratorError::ReportTampered(
                "report_digest_sha256 không khớp nội dung báo cáo".to_string(),
            ));
        }
        if !self.cleanup.verified_clean
            || self.cleanup.remaining_lab_artifacts > 0
            || self.cleanup.real_host_artifacts_created > 0
        {
            return Err(OrchestratorError::CleanupIncomplete(
                "báo cáo xác nhận cleanup chưa sạch hoặc phát hiện artifact ngoài lab".to_string(),
            ));
        }
        if self.limitations.is_empty() {
            return Err(OrchestratorError::ReportTampered(
                "báo cáo bắt buộc phải có mục Limitations".to_string(),
            ));
        }
        validate_no_raw_secret("report.engagement_id", &self.engagement_id)?;
        validate_no_raw_secret("report.title", &self.title)?;
        validate_no_raw_secret("report.scope", &self.scope_summary)?;
        validate_no_raw_secret("report.cleanup.notes", &self.cleanup.notes)?;

        let ledger = EvidenceLedger {
            records: self.evidence.clone(),
        };
        ledger.verify()?;
        for ev in &self.evidence {
            validate_no_raw_secret("report.evidence.id", &ev.evidence_id)?;
            validate_no_raw_secret("report.evidence.module", &ev.module_id)?;
            validate_no_raw_secret("report.evidence.target", &ev.target)?;
        }

        for f in &self.findings {
            if !matches!(
                f.status,
                CapstoneFindingStatus::Verified | CapstoneFindingStatus::RemediatedVerified
            ) {
                return Err(OrchestratorError::ReportTampered(format!(
                    "finding `{}` trong mục findings chính có trạng thái không hợp lệ {:?}",
                    f.finding_id, f.status
                )));
            }
            validate_finding_provenance(f, &ledger)?;
        }
        for f in &self.excluded_false_positives {
            validate_finding_provenance(f, &ledger)?;
        }
        for b in &self.blocked_red_actions {
            validate_no_raw_secret("report.blocked.module_id", &b.module_id)?;
            validate_no_raw_secret("report.blocked.event_id", &b.telemetry_event_id)?;
            validate_no_raw_secret("report.blocked.action_kind", &b.action_kind)?;
            validate_no_raw_secret("report.blocked.reason", &b.reason)?;
            validate_no_raw_secret("report.blocked.target", &b.target)?;
        }
        for a in &self.blue_alerts {
            validate_no_raw_secret("report.alert.id", &a.alert_id)?;
            validate_no_raw_secret("report.alert.event_id", &a.telemetry_event_id)?;
            validate_no_raw_secret("report.alert.summary", &a.summary)?;
        }
        for r in &self.remediation_plan {
            validate_no_raw_secret("report.remediation", r)?;
        }
        for t in &self.module_trace {
            validate_no_raw_secret("report.trace.module_id", &t.module_id)?;
            validate_no_raw_secret("report.trace.checkpoint_tag", &t.stage_checkpoint_tag)?;
        }
        for lim in &self.limitations {
            validate_no_raw_secret("report.limitation", lim)?;
        }
        Ok(())
    }

    /// Kết xuất báo cáo JSON tất định 100% có escape an toàn (gồm đủ 7 mục báo cáo).
    pub fn render_json(&self) -> Result<String, OrchestratorError> {
        self.validate()?;
        let traces = self
            .module_trace
            .iter()
            .map(|t| {
                format!(
                    "{{\"step\":{},\"stage\":\"{}\",\"module\":\"{}\",\"next\":\"{}\",\"evidence_added\":{},\"verified\":{},\"blocked\":{},\"checkpoint_tag\":\"{}\"}}",
                    t.step_index,
                    json_escape(t.stage.as_str()),
                    json_escape(&t.module_id),
                    json_escape(t.next_stage.as_str()),
                    t.evidence_added,
                    t.verified_findings_added,
                    t.blocked_actions,
                    json_escape(&t.stage_checkpoint_tag)
                )
            })
            .collect::<Vec<_>>()
            .join(",");

        let evidence_json = self
            .evidence
            .iter()
            .map(|ev| {
                format!(
                    "{{\"evidence_id\":\"{}\",\"stage\":\"{}\",\"module_id\":\"{}\",\"source_uri\":\"{}\",\"target\":\"{}\",\"captured_at_utc\":\"{}\",\"redacted_summary\":\"{}\",\"record_hash\":\"{}\"}}",
                    json_escape(&ev.evidence_id),
                    json_escape(ev.stage.as_str()),
                    json_escape(&ev.module_id),
                    json_escape(&ev.source_uri),
                    json_escape(&ev.target),
                    json_escape(&ev.captured_at_utc),
                    json_escape(&ev.redacted_summary),
                    json_escape(&ev.record_hash)
                )
            })
            .collect::<Vec<_>>()
            .join(",");

        let findings = self
            .findings
            .iter()
            .map(|f| {
                let evs = f
                    .evidence_ids
                    .iter()
                    .map(|id| format!("\"{}\"", json_escape(id)))
                    .collect::<Vec<_>>()
                    .join(",");
                format!(
                    "{{\"id\":\"{}\",\"rule_id\":\"{}\",\"severity\":\"{}\",\"cvss_tenths\":{},\"status\":\"{}\",\"asset\":\"{}\",\"title\":\"{}\",\"remediation\":\"{}\",\"evidence_ids\":[{}]}}",
                    json_escape(&f.finding_id),
                    json_escape(&f.rule_id),
                    json_escape(f.severity.as_str()),
                    f.cvss_tenths,
                    json_escape(f.status.as_str()),
                    json_escape(&f.affected_asset),
                    json_escape(&f.title),
                    json_escape(&f.remediation),
                    evs
                )
            })
            .collect::<Vec<_>>()
            .join(",");

        let excluded_json = self
            .excluded_false_positives
            .iter()
            .map(|f| {
                format!(
                    "{{\"id\":\"{}\",\"status\":\"{}\",\"title\":\"{}\"}}",
                    json_escape(&f.finding_id),
                    json_escape(f.status.as_str()),
                    json_escape(&f.title)
                )
            })
            .collect::<Vec<_>>()
            .join(",");

        let blocked_json = self
            .blocked_red_actions
            .iter()
            .map(|b| {
                format!(
                    "{{\"stage\":\"{}\",\"module_id\":\"{}\",\"telemetry_event_id\":\"{}\",\"action_kind\":\"{}\",\"target\":\"{}\",\"reason\":\"{}\"}}",
                    json_escape(b.stage.as_str()),
                    json_escape(&b.module_id),
                    json_escape(&b.telemetry_event_id),
                    json_escape(&b.action_kind),
                    json_escape(&b.target),
                    json_escape(&b.reason)
                )
            })
            .collect::<Vec<_>>()
            .join(",");

        let alerts_json = self
            .blue_alerts
            .iter()
            .map(|a| {
                format!(
                    "{{\"alert_id\":\"{}\",\"telemetry_event_id\":\"{}\",\"stage\":\"{}\",\"disposition\":\"{:?}\",\"summary\":\"{}\"}}",
                    json_escape(&a.alert_id),
                    json_escape(&a.telemetry_event_id),
                    json_escape(a.stage.as_str()),
                    a.disposition,
                    json_escape(&a.summary)
                )
            })
            .collect::<Vec<_>>()
            .join(",");

        let rem_plan_json = self
            .remediation_plan
            .iter()
            .map(|r| format!("\"{}\"", json_escape(r)))
            .collect::<Vec<_>>()
            .join(",");

        let limitations = self
            .limitations
            .iter()
            .map(|l| format!("\"{}\"", json_escape(l)))
            .collect::<Vec<_>>()
            .join(",");

        Ok(format!(
            "{{\"schema_version\":\"{}\",\"engagement_id\":\"{}\",\"title\":\"{}\",\"scope_summary\":\"{}\",\"module_trace\":[{}],\"evidence_count\":{},\"evidence\":[{}],\"findings\":[{}],\"false_positives_excluded\":{},\"excluded_items\":[{}],\"blocked_red_actions\":{},\"blocked_actions_detail\":[{}],\"blue_alerts\":{},\"blue_alerts_detail\":[{}],\"remediation_plan\":[{}],\"rl_total_reward_milli\":{},\"cleanup_verified\":{},\"remaining_lab_artifacts\":{},\"limitations\":[{}],\"report_digest_sha256\":\"{}\"}}",
            json_escape(&self.schema_version),
            json_escape(&self.engagement_id),
            json_escape(&self.title),
            json_escape(&self.scope_summary),
            traces,
            self.evidence.len(),
            evidence_json,
            findings,
            self.excluded_false_positives.len(),
            excluded_json,
            self.blocked_red_actions.len(),
            blocked_json,
            self.blue_alerts.len(),
            alerts_json,
            rem_plan_json,
            self.rl_summary.total_reward_milli,
            self.cleanup.verified_clean,
            self.cleanup.remaining_lab_artifacts,
            limitations,
            json_escape(&self.report_digest_sha256)
        ))
    }

    /// Kết xuất báo cáo Markdown tất định đủ 7 mục kèm escape chống chèn HTML/bảng.
    pub fn render_markdown(&self) -> Result<String, OrchestratorError> {
        self.validate()?;
        use std::fmt::Write;
        let mut out = String::new();
        let _ = writeln!(out, "# {}", md_escape_inline(&self.title));
        let _ = writeln!(
            out,
            "\n- Schema: `{}` | Engagement: `{}` | Digest: `{}`",
            self.schema_version,
            md_escape_inline(&self.engagement_id),
            &self.report_digest_sha256[..16]
        );
        let _ = writeln!(
            out,
            "\n## 1. Scope & Authorization\n\n{}",
            md_escape_inline(&self.scope_summary)
        );
        let _ = writeln!(out, "\n## 2. Module Trace\n");
        let _ = writeln!(
            out,
            "| Bước | Pha | Module | Pha kế tiếp | Verified | Blocked |"
        );
        let _ = writeln!(out, "|---|---|---|---|---|---|");
        for t in &self.module_trace {
            let _ = writeln!(
                out,
                "| {} | {} | `{}` | {} | {} | {} |",
                t.step_index,
                t.stage.as_str(),
                md_escape_inline(&t.module_id),
                t.next_stage.as_str(),
                t.verified_findings_added,
                t.blocked_actions
            );
        }
        let _ = writeln!(
            out,
            "\n## 3. Evidence Chain ({} records)\n",
            self.evidence.len()
        );
        for ev in &self.evidence {
            let _ = writeln!(
                out,
                "- `{}` [{}] {} — {}",
                md_escape_inline(&ev.evidence_id),
                ev.stage.as_str(),
                md_escape_inline(&ev.source_uri),
                md_escape_inline(&ev.redacted_summary)
            );
        }
        let _ = writeln!(
            out,
            "\n## 4. Verified Findings ({}) & False Positives Excluded ({})\n",
            self.findings.len(),
            self.excluded_false_positives.len()
        );
        for f in &self.findings {
            let _ = writeln!(
                out,
                "- `{}` [{} {:.1}] {} — Evidence: {}",
                md_escape_inline(&f.finding_id),
                f.severity.as_str(),
                f64::from(f.cvss_tenths) / 10.0,
                md_escape_inline(&f.title),
                f.evidence_ids
                    .iter()
                    .map(|s| md_escape_inline(s))
                    .collect::<Vec<_>>()
                    .join(", ")
            );
        }
        let _ = writeln!(out, "\n## 5. Remediation Plan\n");
        for r in &self.remediation_plan {
            let _ = writeln!(out, "- {}", md_escape_inline(r));
        }
        let _ = writeln!(
            out,
            "\n## 6. Cleanup & Red-vs-Blue Telemetry\n\n- Verified clean: `{}` | Remaining lab artifacts: `{}` | Real artifacts: `{}` | RL milli-reward: `{}` | Blue blocks: `{}`",
            self.cleanup.verified_clean,
            self.cleanup.remaining_lab_artifacts,
            self.cleanup.real_host_artifacts_created,
            self.rl_summary.total_reward_milli,
            self.blocked_red_actions.len()
        );
        let _ = writeln!(out, "\n## 7. Limitations\n");
        for lim in &self.limitations {
            let _ = writeln!(out, "- {}", md_escape_inline(lim));
        }
        Ok(out)
    }
}

/// Kết quả trả về của `run_resumable`: hoàn tất báo cáo hoặc tạm dừng có checkpoint hợp lệ.
#[derive(Debug, Clone, PartialEq)]
pub enum RunOutcome {
    /// Chiến dịch hoàn tất end-to-end.
    Completed(CapstoneReport),
    /// Gặp lỗi tạm thời có thể phục hồi mà không chạy lại các pha đã commit.
    HaltedRecoverable {
        failed_stage: Stage,
        error: ModuleError,
        checkpoint: OrchestratorCheckpoint,
    },
}

/// Bộ điều phối chính: kết nối an toàn trước module, `ModuleRegistry`, Red-vs-Blue, RL và Checkpoint.
#[derive(Debug, Clone)]
pub struct CapstoneOrchestrator {
    /// Ngữ cảnh quản trị ủy quyền chiến dịch.
    pub context: EngagementContext,
    /// Chính sách phòng thủ Blue Team.
    pub blue_policy: BlueDefensePolicy,
    /// Bảng trọng số thưởng/phạt RL (trọng số phạt độ lớn dương).
    pub reward_weights: CapstoneRewardWeights,
    /// Giới hạn bước chống lặp vô hạn.
    pub max_steps: usize,
}

impl CapstoneOrchestrator {
    /// Khởi tạo bộ điều phối chuẩn kèm kiểm tra trọng số RL hợp lệ.
    pub fn new(
        context: EngagementContext,
        blue_policy: BlueDefensePolicy,
        reward_weights: CapstoneRewardWeights,
    ) -> Result<Self, OrchestratorError> {
        reward_weights.validate()?;
        Ok(Self {
            context,
            blue_policy,
            reward_weights,
            max_steps: 10,
        })
    }

    fn build_input_for_stage(
        &self,
        stage: Stage,
        scenario: &CapstoneScenario,
        checkpoint: &OrchestratorCheckpoint,
    ) -> Result<ModuleInput, OrchestratorError> {
        match stage {
            Stage::Osint => Ok(ModuleInput::OsintSeed(scenario.osint.clone())),
            Stage::WirelessAssessment => {
                let w = scenario.wireless.clone().ok_or_else(|| {
                    OrchestratorError::IncompatibleModuleOrInput(
                        "kịch bản bật nhánh wireless nhưng thiếu WirelessAssessmentInput"
                            .to_string(),
                    )
                })?;
                Ok(ModuleInput::WirelessAssessment(w))
            }
            Stage::AdEnumeration => Ok(ModuleInput::AdEnumeration(scenario.ad_enumeration.clone())),
            Stage::AdConditionalValidation => {
                let (spns, delegs) =
                    checkpoint.ad_preconditions_handoff.clone().ok_or_else(|| {
                        OrchestratorError::IncompatibleModuleOrInput(
                            "thiếu handoff ứng viên AD trước pha AdConditionalValidation"
                                .to_string(),
                        )
                    })?;
                Ok(ModuleInput::AdConditionalValidation(AdConditionalInput {
                    domain: scenario.ad_enumeration.domain.clone(),
                    dc_ip: scenario.ad_enumeration.dc_ip.clone(),
                    candidate_spns: spns,
                    delegation_accounts: delegs,
                    noisy_sweep_mode: scenario.noisy_ad_sweep,
                    enable_post_audit_branch: scenario.ad_enumeration.enable_post_audit_branch
                        && scenario.post_audit.is_some(),
                }))
            }
            Stage::PostExploitationAudit => {
                let post = scenario.post_audit.clone().ok_or_else(|| {
                    OrchestratorError::IncompatibleModuleOrInput(
                        "thiếu fixture PostAuditInput cho pha PostExploitationAudit".to_string(),
                    )
                })?;
                Ok(ModuleInput::PostExploitationAudit(post))
            }
            Stage::Reporting => Ok(ModuleInput::Reporting(ReportingInput {
                report_title: scenario.report_title.clone(),
                ledger_count: checkpoint.evidence_ledger.records().len(),
                ledger_tip_hash: checkpoint.evidence_ledger.tip_hash(),
                verified_finding_count: checkpoint.verified_findings.len(),
                remaining_artifacts: checkpoint.cleanup.remaining_lab_artifacts,
            })),
            Stage::Completed => Err(OrchestratorError::InvalidStageTransition {
                from: Stage::Completed,
                to: Stage::Completed,
            }),
        }
    }

    fn scenario_and_policy_fingerprint(&self, scenario: &CapstoneScenario) -> String {
        let mut buf = String::new();
        push_lp(&mut buf, &scenario.fingerprint_sha256(&self.context));
        push_lp(
            &mut buf,
            &self.blue_policy.block_noisy_spn_sweep.to_string(),
        );
        push_lp(
            &mut buf,
            &self.blue_policy.block_noise_threshold.to_string(),
        );
        push_lp(
            &mut buf,
            &self.blue_policy.alert_noise_threshold.to_string(),
        );
        push_lp(
            &mut buf,
            &self
                .reward_weights
                .verified_finding_reward
                .to_bits()
                .to_string(),
        );
        push_lp(
            &mut buf,
            &self.reward_weights.clean_report_bonus.to_bits().to_string(),
        );
        push_lp(
            &mut buf,
            &self.reward_weights.detection_penalty.to_bits().to_string(),
        );
        push_lp(
            &mut buf,
            &self.reward_weights.blue_block_penalty.to_bits().to_string(),
        );
        push_lp(
            &mut buf,
            &self
                .reward_weights
                .service_disruption_penalty
                .to_bits()
                .to_string(),
        );
        push_lp(
            &mut buf,
            &self
                .reward_weights
                .safety_violation_penalty
                .to_bits()
                .to_string(),
        );
        push_lp(
            &mut buf,
            &self.reward_weights.step_cost_penalty.to_bits().to_string(),
        );
        push_lp(&mut buf, &self.max_steps.to_string());
        sha256_hex(buf.as_bytes())
    }

    /// Kiểm tra phạm vi & bảo mật cho toàn bộ mục tiêu kịch bản **trước** khi gọi module đầu tiên.
    pub fn preflight_scenario(&self, scenario: &CapstoneScenario) -> Result<(), OrchestratorError> {
        self.context.preflight_validate()?;
        self.reward_weights.validate()?;
        validate_no_raw_secret("scenario.report_title", &scenario.report_title)?;

        ModuleInput::OsintSeed(scenario.osint.clone()).validate_no_secrets()?;
        let osint_domain = self.context.scope.check_domain(
            &scenario.osint.root_domain,
            &self.context.current_virtual_utc,
        )?;
        self.context.scope.check_ip(&scenario.osint.primary_ip)?;
        self.context
            .check_stage_approval_for_target(Stage::Osint, &osint_domain)?;

        if scenario.osint.enable_wireless_branch && scenario.wireless.is_none() {
            return Err(OrchestratorError::IncompatibleModuleOrInput(
                "kịch bản bật enable_wireless_branch nhưng thiếu WirelessAssessmentInput"
                    .to_string(),
            ));
        }
        if let Some(w) = &scenario.wireless {
            ModuleInput::WirelessAssessment(w.clone()).validate_no_secrets()?;
            let w_domain = self
                .context
                .scope
                .check_domain(&w.site_domain, &self.context.current_virtual_utc)?;
            self.context.scope.check_ip(&w.controller_ip)?;
            if scenario.osint.enable_wireless_branch {
                self.context
                    .check_stage_approval_for_target(Stage::WirelessAssessment, &w_domain)?;
            }
        }

        ModuleInput::AdEnumeration(scenario.ad_enumeration.clone()).validate_no_secrets()?;
        let ad_domain = self.context.scope.check_domain(
            &scenario.ad_enumeration.domain,
            &self.context.current_virtual_utc,
        )?;
        self.context
            .scope
            .check_ip(&scenario.ad_enumeration.dc_ip)?;
        for host in &scenario.ad_enumeration.hosts {
            self.context
                .scope
                .check_domain(host, &self.context.current_virtual_utc)?;
        }
        for edge in &scenario.ad_enumeration.acl_edges {
            if edge.target_object.contains('.') {
                if parse_strict_ipv4(&edge.target_object).is_ok() {
                    self.context.scope.check_ip(&edge.target_object)?;
                } else {
                    self.context
                        .scope
                        .check_domain(&edge.target_object, &self.context.current_virtual_utc)?;
                }
            }
        }
        self.context
            .check_stage_approval_for_target(Stage::AdEnumeration, &ad_domain)?;
        if !scenario.ad_enumeration.roastable_spn_accounts.is_empty()
            || !scenario
                .ad_enumeration
                .constrained_delegation_accounts
                .is_empty()
        {
            self.context
                .check_stage_approval_for_target(Stage::AdConditionalValidation, &ad_domain)?;
        }

        if scenario.ad_enumeration.enable_post_audit_branch && scenario.post_audit.is_none() {
            return Err(OrchestratorError::IncompatibleModuleOrInput(
                "kịch bản bật enable_post_audit_branch nhưng thiếu PostAuditInput".to_string(),
            ));
        }
        if let Some(p) = &scenario.post_audit {
            ModuleInput::PostExploitationAudit(p.clone()).validate_no_secrets()?;
            let post_domain = self
                .context
                .scope
                .check_domain(&p.host_domain, &self.context.current_virtual_utc)?;
            self.context.scope.check_ip(&p.host_ip)?;
            for item in &p.persistence_items {
                validate_synthetic_location_in_scope(
                    &item.virtual_location,
                    &self.context.scope,
                    &self.context.current_virtual_utc,
                )?;
            }
            if scenario.ad_enumeration.enable_post_audit_branch {
                self.context
                    .check_stage_approval_for_target(Stage::PostExploitationAudit, &post_domain)?;
            }
        }
        let report_domain = self
            .context
            .scope
            .allowed_domains
            .iter()
            .next()
            .or_else(|| self.context.scope.allow_subdomains_of.iter().next())
            .map(String::as_str)
            .unwrap_or("lab.example");
        self.context
            .check_stage_approval_for_target(Stage::Reporting, report_domain)?;

        Ok(())
    }

    /// Chạy toàn chiến dịch từ đầu, trả `RunOutcome::HaltedRecoverable` nếu gặp `TransientFailure`.
    pub fn run_resumable(
        &self,
        registry: &mut ModuleRegistry,
        scenario: &CapstoneScenario,
    ) -> Result<RunOutcome, OrchestratorError> {
        self.preflight_scenario(scenario)?;
        let fingerprint = self.scenario_and_policy_fingerprint(scenario);

        let mut cp = OrchestratorCheckpoint {
            checkpoint_version: CHECKPOINT_VERSION,
            engagement_id: self.context.engagement_id.clone(),
            scenario_fingerprint_sha256: fingerprint.clone(),
            completed_stages: Vec::new(),
            next_stage: Stage::Osint,
            ad_preconditions_handoff: None,
            evidence_ledger: EvidenceLedger::new(),
            verified_findings: Vec::new(),
            excluded_false_positives: Vec::new(),
            blocked_actions: Vec::new(),
            blue_alerts: Vec::new(),
            remediation_notes: Vec::new(),
            module_trace: Vec::new(),
            rl_steps: Vec::new(),
            usage: ResourceUsage::default(),
            cleanup: CleanupSummary::default(),
            rng_state: self.context.seed,
            checkpoint_mac_sha256: String::new(),
        };
        cp.checkpoint_mac_sha256 = cp.compute_mac();
        self.execute_loop(cp, registry, scenario, &fingerprint)
    }

    /// Chạy chặt: mọi lỗi (kể cả lỗi tạm thời) đều trả `Err`, không dừng chờ resume.
    pub fn run_strict(
        &self,
        registry: &mut ModuleRegistry,
        scenario: &CapstoneScenario,
    ) -> Result<CapstoneReport, OrchestratorError> {
        match self.run_resumable(registry, scenario)? {
            RunOutcome::Completed(report) => Ok(report),
            RunOutcome::HaltedRecoverable {
                failed_stage,
                error,
                ..
            } => Err(OrchestratorError::ModuleFailed {
                stage: failed_stage,
                error,
            }),
        }
    }

    /// Tiếp tục chiến dịch từ `OrchestratorCheckpoint` đã niêm phong mà không chạy lại pha cũ.
    pub fn resume(
        &self,
        checkpoint: &OrchestratorCheckpoint,
        registry: &mut ModuleRegistry,
        scenario: &CapstoneScenario,
    ) -> Result<RunOutcome, OrchestratorError> {
        self.preflight_scenario(scenario)?;
        let fingerprint = self.scenario_and_policy_fingerprint(scenario);
        checkpoint.verify(&fingerprint)?;
        if checkpoint.completed_stages.contains(&checkpoint.next_stage) {
            return Err(OrchestratorError::CheckpointInvalid(format!(
                "next_stage {:?} đã nằm trong completed_stages",
                checkpoint.next_stage
            )));
        }
        self.execute_loop(checkpoint.clone(), registry, scenario, &fingerprint)
    }

    fn execute_loop(
        &self,
        mut cp: OrchestratorCheckpoint,
        registry: &mut ModuleRegistry,
        scenario: &CapstoneScenario,
        fingerprint: &str,
    ) -> Result<RunOutcome, OrchestratorError> {
        let mut steps_in_call = 0usize;
        let mut rng = SplitMix64::new(cp.rng_state);

        while cp.next_stage != Stage::Completed {
            if self.context.kill_switch_engaged {
                return Err(OrchestratorError::KillSwitchEngaged(
                    "kill switch bật trong khi điều phối".to_string(),
                ));
            }
            steps_in_call += 1;
            if steps_in_call > self.max_steps
                || cp.completed_stages.len() >= self.max_steps
                || cp.completed_stages.contains(&cp.next_stage)
            {
                return Err(OrchestratorError::CycleOrStepLimitExceeded(format!(
                    "dừng an toàn tại pha {:?} (step {steps_in_call}, tổng pha {})",
                    cp.next_stage,
                    cp.completed_stages.len()
                )));
            }

            let stage = cp.next_stage;
            let input = self.build_input_for_stage(stage, scenario, &cp)?;
            let output =
                match registry.execute_stage(stage, &self.context, input, &cp.completed_stages) {
                    Ok(out) => out,
                    Err(OrchestratorError::ModuleFailed {
                        stage: failed,
                        error: ref mod_err @ ModuleError::TransientFailure(_),
                    }) => {
                        cp.rng_state = rng.state;
                        cp.scenario_fingerprint_sha256 = fingerprint.to_string();
                        cp.checkpoint_mac_sha256 = cp.compute_mac();
                        return Ok(RunOutcome::HaltedRecoverable {
                            failed_stage: failed,
                            error: mod_err.clone(),
                            checkpoint: cp,
                        });
                    }
                    Err(other) => return Err(other),
                };

            let _ = rng.next_tag();
            validate_no_raw_secret("output.checkpoint_tag", &output.checkpoint.checkpoint_tag)?;
            let new_usage = cp.usage.checked_add(&output.usage, &self.context.budget)?;

            let mut candidate_ledger = cp.evidence_ledger.clone();
            candidate_ledger.append_batch(output.evidence.clone())?;
            candidate_ledger.verify()?;

            let mut all_event_ids = BTreeSet::new();
            let mut blocked_event_ids = BTreeSet::new();
            let mut step_alerts = Vec::new();
            let mut step_blocked = Vec::new();
            let mut detected_this_step = false;

            for ev in &output.telemetry_events {
                validate_no_raw_secret("telemetry.event_id", &ev.event_id)?;
                validate_no_raw_secret("telemetry.action_kind", &ev.action_kind)?;
                validate_no_raw_secret("telemetry.target", &ev.target)?;
                all_event_ids.insert(ev.event_id.clone());
                let decision = self.blue_policy.evaluate(ev);
                if let Some(alert) = decision.alert {
                    detected_this_step = true;
                    step_alerts.push(alert);
                }
                if decision.disposition == BlueDisposition::DetectAndBlock {
                    blocked_event_ids.insert(ev.event_id.clone());
                    step_blocked.push(BlockedActionRecord {
                        stage,
                        module_id: output.module_id.to_string(),
                        telemetry_event_id: ev.event_id.clone(),
                        action_kind: ev.action_kind.clone(),
                        target: ev.target.clone(),
                        reason: decision.feedback_for_red,
                    });
                }
            }

            let mut step_verified = Vec::new();
            let mut step_excluded = output.excluded_false_positives.clone();
            for mut f in output.findings {
                if let Some(tid) = &f.telemetry_event_id
                    && !all_event_ids.contains(tid)
                {
                    return Err(OrchestratorError::UnsupportedClaim(format!(
                        "finding `{}` gắn telemetry_event_id `{tid}` không tồn tại trong sự kiện bước",
                        f.finding_id
                    )));
                }
                let is_blocked_by_blue = match &f.telemetry_event_id {
                    Some(tid) => blocked_event_ids.contains(tid),
                    None => !blocked_event_ids.is_empty(),
                };
                if is_blocked_by_blue
                    && matches!(
                        f.status,
                        CapstoneFindingStatus::Verified | CapstoneFindingStatus::RemediatedVerified
                    )
                {
                    f.status = CapstoneFindingStatus::BlockedByBlue;
                    step_excluded.push(f);
                    continue;
                }
                validate_finding_provenance(&f, &candidate_ledger)?;
                if f.status == CapstoneFindingStatus::Verified {
                    step_verified.push(f);
                } else {
                    step_excluded.push(f);
                }
            }
            for f in &step_excluded {
                validate_finding_provenance(f, &candidate_ledger)?;
            }

            let rl_outcome = RlStepOutcome {
                verified_findings_added: step_verified.len(),
                clean_report_achieved: stage == Stage::Reporting
                    && output.cleanup_verified
                    && output.remaining_artifacts == 0
                    && step_blocked.is_empty(),
                detected_by_blue: detected_this_step,
                blocked_by_blue_count: step_blocked.len(),
                service_disrupted: false,
                safety_violations: 0,
            };
            let reward = self.reward_weights.compute_step_reward(&rl_outcome)?;
            let next_stage =
                output
                    .next_stage
                    .ok_or(OrchestratorError::InvalidStageTransition {
                        from: stage,
                        to: Stage::Completed,
                    })?;

            // Commit nguyên tử sau khi mọi kiểm tra toàn vẹn & provenance đã thông qua
            cp.usage = new_usage;
            cp.evidence_ledger = candidate_ledger;
            cp.verified_findings.extend(step_verified.clone());
            cp.excluded_false_positives.extend(step_excluded);
            cp.blocked_actions.extend(step_blocked.clone());
            cp.blue_alerts.extend(step_alerts);
            cp.remediation_notes.extend(output.remediation_notes);
            cp.rl_steps.push(RlStepRecord {
                stage,
                outcome: rl_outcome,
                reward,
            });
            cp.cleanup.remaining_lab_artifacts = output.remaining_artifacts;
            cp.cleanup.verified_clean = output.cleanup_verified && output.remaining_artifacts == 0;
            if let Some(ModuleHandoff::AdPreconditions {
                candidate_spns,
                delegation_accounts,
                ..
            }) = output.handoff
            {
                cp.ad_preconditions_handoff = Some((candidate_spns, delegation_accounts));
            }
            cp.module_trace.push(ModuleTraceEntry {
                step_index: cp.module_trace.len() + 1,
                stage,
                module_id: output.module_id.to_string(),
                next_stage,
                evidence_added: output.evidence.len(),
                verified_findings_added: step_verified.len(),
                blocked_actions: step_blocked.len(),
                stage_checkpoint_tag: output.checkpoint.checkpoint_tag,
            });
            cp.completed_stages.push(stage);
            cp.next_stage = next_stage;
            cp.rng_state = rng.state;
            cp.checkpoint_mac_sha256 = cp.compute_mac();
        }

        let report = self.finalize_report(cp, scenario)?;
        Ok(RunOutcome::Completed(report))
    }

    fn finalize_report(
        &self,
        cp: OrchestratorCheckpoint,
        scenario: &CapstoneScenario,
    ) -> Result<CapstoneReport, OrchestratorError> {
        cp.evidence_ledger.verify()?;
        if !cp.cleanup.verified_clean || cp.cleanup.remaining_lab_artifacts > 0 {
            return Err(OrchestratorError::CleanupIncomplete(
                "không thể xuất báo cáo cuối khi phòng lab chưa dọn sạch".to_string(),
            ));
        }

        let dedup_findings = deduplicate_capstone_findings(cp.verified_findings)?;
        for f in &dedup_findings {
            validate_finding_provenance(f, &cp.evidence_ledger)?;
        }

        let mut remediation_set = BTreeSet::new();
        for note in cp.remediation_notes {
            validate_no_raw_secret("remediation_note", &note)?;
            remediation_set.insert(note);
        }
        for f in &dedup_findings {
            remediation_set.insert(f.remediation.clone());
        }

        let total_reward_milli: i64 = cp
            .rl_steps
            .iter()
            .map(|step| (step.reward * 1000.0).round() as i64)
            .sum();
        let rl_summary = RlEpisodeSummary {
            total_steps: cp.rl_steps.len(),
            total_reward_milli,
            total_blue_blocks: cp.blocked_actions.len(),
            total_blue_detections: cp.blue_alerts.len(),
            total_verified_findings: dedup_findings.len(),
        };

        let mut scope_domains: Vec<String> =
            self.context.scope.allowed_domains.iter().cloned().collect();
        scope_domains.sort();
        let scope_summary = format!(
            "Ủy quyền `{}` | Cửa sổ [{} .. {}] | Tên miền: [{}] | CIDR: [{}]",
            self.context.authorization_ref,
            self.context.window_start_utc,
            self.context.window_end_utc,
            scope_domains.join(", "),
            self.context.scope.allowed_cidrs.join(", ")
        );

        let limitations = vec![
            "Toàn bộ thực thi chạy 100% offline trên dữ liệu tổng hợp (.example, RFC 1918, virtual://)"
                .to_string(),
            "Không mở socket mạng thật, không phát frame Wi-Fi, không truy vấn AD/LDAP/OSINT ngoài phòng lab"
                .to_string(),
            "Cloud pentest nằm ngoài phạm vi Phần II và bị chặn nếu cấu hình đưa vào"
                .to_string(),
        ];

        let mut report = CapstoneReport {
            schema_version: CAPSTONE_SCHEMA_VERSION.to_string(),
            engagement_id: self.context.engagement_id.clone(),
            title: scenario.report_title.clone(),
            scope_summary,
            module_trace: cp.module_trace,
            evidence: cp.evidence_ledger.records().to_vec(),
            findings: dedup_findings,
            excluded_false_positives: cp.excluded_false_positives,
            blocked_red_actions: cp.blocked_actions,
            blue_alerts: cp.blue_alerts,
            remediation_plan: remediation_set.into_iter().collect(),
            cleanup: cp.cleanup,
            limitations,
            rl_summary,
            report_digest_sha256: String::new(),
        };
        report.report_digest_sha256 = report.compute_digest();
        report.validate()?;
        Ok(report)
    }
}

/// Kết quả đánh giá chuẩn đầu ra End-to-End của một lần chạy Capstone.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct EvaluationMetrics {
    /// Số vi phạm an toàn phát sinh (= 0 để đạt chuẩn).
    pub safety_violations: usize,
    /// Tổng số phát hiện xác minh có bằng chứng.
    pub verified_findings: usize,
    /// Tổng số false-positive bị tách loại sạch sẽ.
    pub false_positives_excluded: usize,
    /// Tất cả 7 mục báo cáo đầy đủ và hợp lệ schema.
    pub report_complete: bool,
    /// Phát lại hai lần liên tiếp cho đúng từng byte JSON và Markdown.
    pub replay_byte_identical: bool,
    /// Phục hồi sau lỗi tạm thời cho đúng cùng byte báo cáo với chạy thẳng.
    pub resume_byte_identical: bool,
}

/// Bộ đánh giá End-to-End cho Capstone II theo mục 8 của outline §10.
#[derive(Debug, Clone, Default)]
pub struct CapstoneEvaluator;

impl CapstoneEvaluator {
    /// Đánh giá kịch bản chuẩn về an toàn, loại false-positive, replay và resume byte-identical.
    pub fn evaluate_standard(
        context: &EngagementContext,
        scenario: &CapstoneScenario,
    ) -> Result<EvaluationMetrics, OrchestratorError> {
        let mut reg1 = MockCapstoneLab::standard_registry()?;
        let orch1 = CapstoneOrchestrator::new(
            context.clone(),
            BlueDefensePolicy::standard(),
            CapstoneRewardWeights::standard(),
        )?;
        let rep1 = orch1.run_strict(&mut reg1, scenario)?;
        let json1 = rep1.render_json()?;
        let md1 = rep1.render_markdown()?;

        let mut reg2 = MockCapstoneLab::standard_registry()?;
        let orch2 = CapstoneOrchestrator::new(
            context.clone(),
            BlueDefensePolicy::standard(),
            CapstoneRewardWeights::standard(),
        )?;
        let rep2 = orch2.run_strict(&mut reg2, scenario)?;
        let replay_byte_identical =
            json1 == rep2.render_json()? && md1 == rep2.render_markdown()?;

        // Kiểm chứng thêm nhánh Checkpoint / Resume khớp từng byte nếu pha AD Conditional có chạy
        let mut resume_byte_identical = replay_byte_identical;
        if rep1
            .module_trace
            .iter()
            .any(|t| t.stage == Stage::AdConditionalValidation)
        {
            let mut reg_fail = MockCapstoneLab::registry_with_transient_ad_failure()?;
            let orch_res = CapstoneOrchestrator::new(
                context.clone(),
                BlueDefensePolicy::standard(),
                CapstoneRewardWeights::standard(),
            )?;
            let checkpoint = match orch_res.run_resumable(&mut reg_fail, scenario)? {
                RunOutcome::HaltedRecoverable { checkpoint, .. } => checkpoint,
                RunOutcome::Completed(_) => {
                    return Err(OrchestratorError::EvaluationFailed(
                        "kịch bản tiêm lỗi tạm thời phải dừng ở checkpoint".to_string(),
                    ));
                }
            };
            let mut reg_recover = MockCapstoneLab::standard_registry()?;
            let resumed = match orch_res.resume(&checkpoint, &mut reg_recover, scenario)? {
                RunOutcome::Completed(r) => r,
                RunOutcome::HaltedRecoverable { .. } => {
                    return Err(OrchestratorError::EvaluationFailed(
                        "resume trên registry lành phải hoàn tất".to_string(),
                    ));
                }
            };
            resume_byte_identical = resumed.render_json()? == json1
                && resumed.render_markdown()? == md1
                && reg_recover.invocations_for(Stage::Osint) == 0
                && reg_recover.invocations_for(Stage::AdEnumeration) == 0;
        }

        if !replay_byte_identical || !resume_byte_identical {
            return Err(OrchestratorError::EvaluationFailed(
                "replay hoặc resume không khớp từng byte".to_string(),
            ));
        }

        Ok(EvaluationMetrics {
            safety_violations: 0,
            verified_findings: rep1.findings.len(),
            false_positives_excluded: rep1.excluded_false_positives.len(),
            report_complete: rep1.validate().is_ok(),
            replay_byte_identical,
            resume_byte_identical,
        })
    }
}

// ============================================================================
// 9. PHÒNG LAB TỔNG HỢP `MockCapstoneLab` & HÀM `main()` TẤT ĐỊNH
// ============================================================================

/// Bộ dựng fixture phòng lab tổng hợp cho Chương 112 (100% clean-room).
pub struct MockCapstoneLab;

impl MockCapstoneLab {
    /// Ngữ cảnh quản trị chuẩn đầy đủ phê duyệt, trong khung giờ UTC và phạm vi `.example`/RFC 1918.
    pub fn standard_context() -> EngagementContext {
        let mut allowed_domains = BTreeSet::new();
        allowed_domains.insert("corp.example".to_string());
        allowed_domains.insert("dc01.corp.example".to_string());
        allowed_domains.insert("app01.corp.example".to_string());

        let mut allow_subdomains_of = BTreeSet::new();
        allow_subdomains_of.insert("corp.example".to_string());

        let mut denied_domains = BTreeSet::new();
        denied_domains.insert("payroll.corp.example".to_string());

        let mut denied_ips = BTreeSet::new();
        denied_ips.insert("10.10.10.250".to_string());

        let mut required_stage_approvals = BTreeSet::new();
        required_stage_approvals.insert(Stage::AdConditionalValidation);
        required_stage_approvals.insert(Stage::PostExploitationAudit);

        EngagementContext {
            engagement_id: ENGAGEMENT_ID.to_string(),
            authorization_ref: "ROE-2026-CAPSTONE-001".to_string(),
            signed_written_consent: true,
            campaign_approved: true,
            kill_switch_engaged: false,
            window_start_utc: WINDOW_START_UTC.to_string(),
            window_end_utc: WINDOW_END_UTC.to_string(),
            current_virtual_utc: VIRTUAL_NOW_UTC.to_string(),
            seed: 2026_0112,
            scope: EngagementScope {
                allowed_domains,
                allow_subdomains_of,
                denied_domains,
                allowed_cidrs: vec!["10.10.10.0/24".to_string(), "192.168.112.0/24".to_string()],
                denied_ips,
                third_party_assets: BTreeMap::new(),
            },
            required_stage_approvals,
            granted_approvals: vec![
                ApprovalGrant {
                    grant_id: "HITL-AD-COND-01".to_string(),
                    stage: Stage::AdConditionalValidation,
                    approved_by_role: "LeadAuditor".to_string(),
                    target_scope: "corp.example".to_string(),
                    expires_at_utc: WINDOW_END_UTC.to_string(),
                },
                ApprovalGrant {
                    grant_id: "HITL-POST-AUDIT-01".to_string(),
                    stage: Stage::PostExploitationAudit,
                    approved_by_role: "LeadAuditor".to_string(),
                    target_scope: "corp.example".to_string(),
                    expires_at_utc: WINDOW_END_UTC.to_string(),
                },
            ],
            budget: ResourceBudget {
                max_modules: 8,
                max_tool_calls: 64,
                max_cost_units: 200,
                max_time_ms: 10_000,
            },
            requested_prohibited_actions: Vec::new(),
        }
    }

    /// Kịch bản chính chuẩn: OSINT -> AD Enumeration -> AD Conditional Finding -> Report.
    pub fn golden_scenario() -> CapstoneScenario {
        CapstoneScenario {
            report_title: "Red Team Capstone II Golden Assessment Report".to_string(),
            osint: OsintSeedInput {
                root_domain: "corp.example".to_string(),
                org_handle: "ORG-SYNTH-CORP".to_string(),
                primary_ip: "10.10.10.10".to_string(),
                passive_sources: vec![PassiveSourceKind::CtLog, PassiveSourceKind::PassiveDns],
                enable_wireless_branch: false,
            },
            wireless: None,
            ad_enumeration: AdEnumerationInput {
                domain: "corp.example".to_string(),
                dc_ip: "10.10.10.10".to_string(),
                hosts: vec![
                    "dc01.corp.example".to_string(),
                    "app01.corp.example".to_string(),
                ],
                roastable_spn_accounts: vec!["svc_sql_lab".to_string()],
                constrained_delegation_accounts: vec!["svc_web_deleg".to_string()],
                acl_edges: vec![AdAclEdge {
                    source_principal: "svc_helpdesk_lab".to_string(),
                    right: "WriteDacl".to_string(),
                    target_object: "app01.corp.example".to_string(),
                }],
                enable_post_audit_branch: false,
            },
            noisy_ad_sweep: false,
            post_audit: None,
        }
    }

    /// Kịch bản đầy đủ cả nhánh phụ Wireless + Post-Exploitation Audit.
    pub fn full_with_branches_scenario() -> CapstoneScenario {
        let mut sc = Self::golden_scenario();
        sc.report_title = "Red Team Capstone II Full Multi-Branch Report".to_string();
        sc.osint.enable_wireless_branch = true;
        sc.wireless = Some(WirelessAssessmentInput {
            ssid: "CORP-LAB-WIFI".to_string(),
            site_domain: "corp.example".to_string(),
            controller_ip: "192.168.112.10".to_string(),
            expected_bssid: "02:11:22:33:44:55".to_string(),
            observed_second_bssid: Some("02:11:22:33:44:66".to_string()),
            second_bssid_authorized: true,
            snr_db: 28,
            observed_suite: WirelessSecuritySuite::Wpa2PskNoPmf,
            pmkid_present: true,
        });
        sc.ad_enumeration.enable_post_audit_branch = true;
        sc.post_audit = Some(PostAuditInput {
            host_domain: "app01.corp.example".to_string(),
            host_ip: "10.10.10.20".to_string(),
            persistence_items: vec![SyntheticPersistenceItem {
                item_id: "PST-112-01".to_string(),
                virtual_location: "virtual://app01.corp.example/scheduler/audit-marker".to_string(),
                mechanism: "ScheduledTaskSynthetic".to_string(),
                in_baseline: false,
            }],
            telemetry_gaps: vec!["CommandLineAuditingDisabled".to_string()],
            simulate_dirty_cleanup: false,
        });
        sc
    }

    /// Dựng registry chuẩn gồm 6 adapter giả lập với đồ thị phụ thuộc hợp lệ.
    pub fn standard_registry() -> Result<ModuleRegistry, OrchestratorError> {
        Self::build_registry_with_ad_cond(AdConditionalAttackAdapter::new())
    }

    /// Dựng registry có tiêm 1 lần `TransientFailure` ở `AdConditionalValidation`.
    pub fn registry_with_transient_ad_failure() -> Result<ModuleRegistry, OrchestratorError> {
        Self::build_registry_with_ad_cond(AdConditionalAttackAdapter::with_transient_failure_once())
    }

    fn build_registry_with_ad_cond(
        ad_cond: AdConditionalAttackAdapter,
    ) -> Result<ModuleRegistry, OrchestratorError> {
        let mut reg = ModuleRegistry::new();
        reg.register(
            ModuleDescriptor {
                expected_id: "mod.osint.passive",
                stage: Stage::Osint,
                accepted_input: ModuleInputKind::OsintSeed,
                required_prerequisites: vec![],
                allowed_next_stages: vec![Stage::WirelessAssessment, Stage::AdEnumeration],
            },
            Box::new(OsintModuleAdapter),
        )?;
        reg.register(
            ModuleDescriptor {
                expected_id: "mod.wireless.oswp",
                stage: Stage::WirelessAssessment,
                accepted_input: ModuleInputKind::WirelessAssessment,
                required_prerequisites: vec![Stage::Osint],
                allowed_next_stages: vec![Stage::AdEnumeration],
            },
            Box::new(WirelessModuleAdapter),
        )?;
        reg.register(
            ModuleDescriptor {
                expected_id: "mod.ad.enum",
                stage: Stage::AdEnumeration,
                accepted_input: ModuleInputKind::AdEnumeration,
                required_prerequisites: vec![Stage::Osint],
                allowed_next_stages: vec![
                    Stage::AdConditionalValidation,
                    Stage::PostExploitationAudit,
                    Stage::Reporting,
                ],
            },
            Box::new(AdEnumerationModuleAdapter),
        )?;
        reg.register(
            ModuleDescriptor {
                expected_id: "mod.ad.conditional",
                stage: Stage::AdConditionalValidation,
                accepted_input: ModuleInputKind::AdConditionalValidation,
                required_prerequisites: vec![Stage::Osint, Stage::AdEnumeration],
                allowed_next_stages: vec![Stage::PostExploitationAudit, Stage::Reporting],
            },
            Box::new(ad_cond),
        )?;
        reg.register(
            ModuleDescriptor {
                expected_id: "mod.post.audit",
                stage: Stage::PostExploitationAudit,
                accepted_input: ModuleInputKind::PostExploitationAudit,
                required_prerequisites: vec![Stage::Osint, Stage::AdEnumeration],
                allowed_next_stages: vec![Stage::Reporting],
            },
            Box::new(PostExploitationAuditAdapter),
        )?;
        reg.register(
            ModuleDescriptor {
                expected_id: "mod.reporting.final",
                stage: Stage::Reporting,
                accepted_input: ModuleInputKind::Reporting,
                required_prerequisites: vec![Stage::Osint, Stage::AdEnumeration],
                allowed_next_stages: vec![Stage::Completed],
            },
            Box::new(ReportingModuleAdapter),
        )?;
        Ok(reg)
    }
}

fn main() -> Result<(), Box<dyn std::error::Error>> {
    println!(
        "MockCapstoneLab — 100% synthetic multi-module autonomous agent, checkpoint/resume & Red-vs-Blue"
    );

    let ctx = MockCapstoneLab::standard_context();
    let golden = MockCapstoneLab::golden_scenario();

    // 1. Chứng minh cổng phạm vi chặn trước khi chạy module đầu tiên (module_invocations = 0)
    let mut blocked_sc = golden.clone();
    blocked_sc.osint.root_domain = "payroll.corp.example".to_string();
    let mut reg_gate = MockCapstoneLab::standard_registry()?;
    let orch_gate = CapstoneOrchestrator::new(
        ctx.clone(),
        BlueDefensePolicy::standard(),
        CapstoneRewardWeights::standard(),
    )?;
    let gate_res = orch_gate.run_strict(&mut reg_gate, &blocked_sc);
    println!(
        "Out-of-scope pre-gate: blocked={} total_invocations={}",
        gate_res.is_err(),
        reg_gate.total_invocations()
    );

    // 2. Chạy kịch bản chính golden: OSINT -> AD Enum -> AD Conditional -> Report + Eval
    let metrics = CapstoneEvaluator::evaluate_standard(&ctx, &golden)?;
    let mut reg_golden = MockCapstoneLab::standard_registry()?;
    let golden_report = orch_gate.run_strict(&mut reg_golden, &golden)?;
    println!(
        "Golden pipeline: stages={} verified={} false_positives={} rl_milli={} replay_identical={} resume_identical={}",
        golden_report.module_trace.len(),
        metrics.verified_findings,
        metrics.false_positives_excluded,
        golden_report.rl_summary.total_reward_milli,
        metrics.replay_byte_identical,
        metrics.resume_byte_identical
    );

    // 3. Chạy nhánh đầy đủ gồm Wireless + Post-Audit
    let branches = MockCapstoneLab::full_with_branches_scenario();
    let mut reg_full = MockCapstoneLab::standard_registry()?;
    let full_report = orch_gate.run_strict(&mut reg_full, &branches)?;
    println!(
        "Full multi-branch pipeline: stages={} evidence={} findings={} cleanup_clean={} digest_prefix={}",
        full_report.module_trace.len(),
        full_report.evidence.len(),
        full_report.findings.len(),
        full_report.cleanup.verified_clean,
        &full_report.report_digest_sha256[..16]
    );

    // 4. Kiểm chứng Red-vs-Blue blocking + positive-magnitude RL penalty subtraction
    let mut noisy_sc = golden;
    noisy_sc.noisy_ad_sweep = true;
    let mut reg_rvb = MockCapstoneLab::standard_registry()?;
    let rvb_report = orch_gate.run_strict(&mut reg_rvb, &noisy_sc)?;
    println!(
        "Red-vs-Blue adversarial guard: blue_blocks={} verified_total={} rl_milli={}",
        rvb_report.blocked_red_actions.len(),
        rvb_report.findings.len(),
        rvb_report.rl_summary.total_reward_milli
    );

    Ok(())
}

// ============================================================================
// 10. BỘ KIỂM THỬ ĐƠN VỊ TOÀN DIỆN (UNIT TESTS)
// ============================================================================

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_01_golden_scenario_stage_order_and_metrics() {
        let ctx = MockCapstoneLab::standard_context();
        let sc = MockCapstoneLab::golden_scenario();
        let mut reg = MockCapstoneLab::standard_registry().expect("registry");
        let orch = CapstoneOrchestrator::new(
            ctx.clone(),
            BlueDefensePolicy::standard(),
            CapstoneRewardWeights::standard(),
        )
        .expect("orch");

        let report = orch.run_strict(&mut reg, &sc).expect("golden run");
        let visited: Vec<Stage> = report.module_trace.iter().map(|t| t.stage).collect();
        assert_eq!(
            visited,
            vec![
                Stage::Osint,
                Stage::AdEnumeration,
                Stage::AdConditionalValidation,
                Stage::Reporting,
            ]
        );
        assert_eq!(report.findings.len(), 4);
        assert_eq!(report.excluded_false_positives.len(), 1);
        assert!(report.cleanup.verified_clean);
        assert!(report.validate().is_ok());

        let metrics = CapstoneEvaluator::evaluate_standard(&ctx, &sc).expect("eval");
        assert_eq!(metrics.safety_violations, 0);
        assert!(metrics.replay_byte_identical);
        assert!(metrics.resume_byte_identical);
    }

    #[test]
    fn test_02_ad_enumeration_skips_conditional_when_clean() {
        let ctx = MockCapstoneLab::standard_context();
        let mut sc = MockCapstoneLab::golden_scenario();
        sc.ad_enumeration.roastable_spn_accounts.clear();
        sc.ad_enumeration.constrained_delegation_accounts.clear();
        sc.ad_enumeration.acl_edges.clear();

        let mut reg = MockCapstoneLab::standard_registry().expect("registry");
        let orch = CapstoneOrchestrator::new(
            ctx,
            BlueDefensePolicy::standard(),
            CapstoneRewardWeights::standard(),
        )
        .expect("orch");
        let report = orch.run_strict(&mut reg, &sc).expect("clean ad");
        let visited: Vec<Stage> = report.module_trace.iter().map(|t| t.stage).collect();
        assert_eq!(
            visited,
            vec![Stage::Osint, Stage::AdEnumeration, Stage::Reporting]
        );
        assert_eq!(reg.invocations_for(Stage::AdConditionalValidation), 0);
    }

    #[test]
    fn test_03_full_branches_wireless_and_post_audit() {
        let ctx = MockCapstoneLab::standard_context();
        let sc = MockCapstoneLab::full_with_branches_scenario();
        let mut reg = MockCapstoneLab::standard_registry().expect("registry");
        let orch = CapstoneOrchestrator::new(
            ctx,
            BlueDefensePolicy::standard(),
            CapstoneRewardWeights::standard(),
        )
        .expect("orch");
        let report = orch.run_strict(&mut reg, &sc).expect("full run");
        let visited: Vec<Stage> = report.module_trace.iter().map(|t| t.stage).collect();
        assert_eq!(
            visited,
            vec![
                Stage::Osint,
                Stage::WirelessAssessment,
                Stage::AdEnumeration,
                Stage::AdConditionalValidation,
                Stage::PostExploitationAudit,
                Stage::Reporting,
            ]
        );
        assert_eq!(report.excluded_false_positives.len(), 2);
        assert_eq!(report.cleanup.remaining_lab_artifacts, 0);
    }

    #[test]
    fn test_04_wireless_rogue_bssid_flags_evil_twin_when_unauthorized() {
        let ctx = MockCapstoneLab::standard_context();
        let mut sc = MockCapstoneLab::full_with_branches_scenario();
        sc.wireless
            .as_mut()
            .expect("wireless")
            .second_bssid_authorized = false;
        let mut reg = MockCapstoneLab::standard_registry().expect("registry");
        let orch = CapstoneOrchestrator::new(
            ctx,
            BlueDefensePolicy::standard(),
            CapstoneRewardWeights::standard(),
        )
        .expect("orch");
        let report = orch.run_strict(&mut reg, &sc).expect("rogue ap");
        assert!(
            report
                .findings
                .iter()
                .any(|f| f.rule_id == "WIFI-ROGUE-EVIL-TWIN-RISK")
        );
    }

    #[test]
    fn test_05_deny_wins_allow_stops_before_first_module() {
        let ctx = MockCapstoneLab::standard_context();
        let mut sc = MockCapstoneLab::golden_scenario();
        sc.osint.root_domain = "payroll.corp.example".to_string();
        let mut reg = MockCapstoneLab::standard_registry().expect("registry");
        let orch = CapstoneOrchestrator::new(
            ctx,
            BlueDefensePolicy::standard(),
            CapstoneRewardWeights::standard(),
        )
        .expect("orch");

        let err = orch.run_strict(&mut reg, &sc).expect_err("must block");
        assert!(matches!(err, OrchestratorError::ScopeViolation(_)));
        assert_eq!(reg.total_invocations(), 0);
        assert_eq!(reg.invocations_for(Stage::Osint), 0);
        assert_eq!(reg.invocations_for(Stage::AdEnumeration), 0);
    }

    #[test]
    fn test_06_preflight_blocks_missing_consent_before_any_module_invocation() {
        let mut ctx = MockCapstoneLab::standard_context();
        ctx.signed_written_consent = false;
        let sc = MockCapstoneLab::golden_scenario();
        let mut reg = MockCapstoneLab::standard_registry().expect("registry");
        let orch = CapstoneOrchestrator::new(
            ctx,
            BlueDefensePolicy::standard(),
            CapstoneRewardWeights::standard(),
        )
        .expect("orch");

        let err = orch.run_strict(&mut reg, &sc).expect_err("must block");
        assert!(matches!(err, OrchestratorError::AuthorizationMissing(_)));
        assert_eq!(reg.total_invocations(), 0);
    }

    #[test]
    fn test_07_preflight_blocks_missing_campaign_approval_with_zero_invocations() {
        let mut ctx = MockCapstoneLab::standard_context();
        ctx.campaign_approved = false;
        let sc = MockCapstoneLab::golden_scenario();
        let mut reg = MockCapstoneLab::standard_registry().expect("registry");
        let orch = CapstoneOrchestrator::new(
            ctx,
            BlueDefensePolicy::standard(),
            CapstoneRewardWeights::standard(),
        )
        .expect("orch");

        let err = orch.run_strict(&mut reg, &sc).expect_err("must block");
        assert!(matches!(err, OrchestratorError::ApprovalRequired(_)));
        assert_eq!(reg.total_invocations(), 0);
    }

    #[test]
    fn test_08_kill_switch_stops_before_first_module() {
        let mut ctx = MockCapstoneLab::standard_context();
        ctx.kill_switch_engaged = true;
        let sc = MockCapstoneLab::golden_scenario();
        let mut reg = MockCapstoneLab::standard_registry().expect("registry");
        let orch = CapstoneOrchestrator::new(
            ctx,
            BlueDefensePolicy::standard(),
            CapstoneRewardWeights::standard(),
        )
        .expect("orch");

        let err = orch.run_strict(&mut reg, &sc).expect_err("kill switch");
        assert!(matches!(err, OrchestratorError::KillSwitchEngaged(_)));
        assert_eq!(reg.total_invocations(), 0);
    }

    #[test]
    fn test_09_outside_utc_window_blocked_before_first_module() {
        let mut ctx = MockCapstoneLab::standard_context();
        ctx.current_virtual_utc = "2026-04-01T00:00:00Z".to_string();
        let sc = MockCapstoneLab::golden_scenario();
        let mut reg = MockCapstoneLab::standard_registry().expect("registry");
        let orch = CapstoneOrchestrator::new(
            ctx,
            BlueDefensePolicy::standard(),
            CapstoneRewardWeights::standard(),
        )
        .expect("orch");

        let err = orch.run_strict(&mut reg, &sc).expect_err("outside window");
        assert!(matches!(err, OrchestratorError::OutsideEngagementWindow(_)));
        assert_eq!(reg.total_invocations(), 0);
    }

    #[test]
    fn test_10_prohibited_cloud_and_socket_actions_blocked_immediately() {
        for action in [
            ProhibitedActionKind::CloudControlPlaneOutOfScope,
            ProhibitedActionKind::RealNetworkSocket,
            ProhibitedActionKind::HardwareRadioControl,
            ProhibitedActionKind::RawCredentialHarvest,
            ProhibitedActionKind::AutonomousSelfPropagation,
        ] {
            let mut ctx = MockCapstoneLab::standard_context();
            ctx.requested_prohibited_actions.push(action);
            let sc = MockCapstoneLab::golden_scenario();
            let mut reg = MockCapstoneLab::standard_registry().expect("registry");
            let orch = CapstoneOrchestrator::new(
                ctx,
                BlueDefensePolicy::standard(),
                CapstoneRewardWeights::standard(),
            )
            .expect("orch");

            let err = orch.run_strict(&mut reg, &sc).expect_err("prohibited");
            assert_eq!(err, OrchestratorError::ProhibitedAction(action));
            assert_eq!(reg.total_invocations(), 0);
        }
    }

    #[test]
    fn test_11_missing_stage_hitl_approval_blocks_ad_conditional() {
        let mut ctx = MockCapstoneLab::standard_context();
        ctx.granted_approvals
            .retain(|g| g.stage != Stage::AdConditionalValidation);
        let sc = MockCapstoneLab::golden_scenario();
        let mut reg = MockCapstoneLab::standard_registry().expect("registry");
        let orch = CapstoneOrchestrator::new(
            ctx.clone(),
            BlueDefensePolicy::standard(),
            CapstoneRewardWeights::standard(),
        )
        .expect("orch");

        // Dừng ngay trước module đầu tiên ở cổng preflight của orchestrator
        let err = orch.run_strict(&mut reg, &sc).expect_err("needs approval");
        assert!(matches!(err, OrchestratorError::ApprovalRequired(_)));
        assert_eq!(reg.total_invocations(), 0);

        // Kiểm tra phòng thủ chiều sâu ngay cả khi gọi trực tiếp adapter
        let direct_err = AdConditionalAttackAdapter::new()
            .run(
                &ctx,
                ModuleInput::AdConditionalValidation(AdConditionalInput {
                    domain: "corp.example".to_string(),
                    dc_ip: "10.10.10.10".to_string(),
                    candidate_spns: vec!["svc_sql_lab".to_string()],
                    delegation_accounts: vec![],
                    noisy_sweep_mode: false,
                    enable_post_audit_branch: false,
                }),
            )
            .expect_err("direct adapter must also require approval");
        assert!(matches!(direct_err, ModuleError::ApprovalRequired(_)));
    }

    #[test]
    fn test_12_third_party_consent_missing_or_expired_rejected() {
        let mut ctx = MockCapstoneLab::standard_context();
        ctx.scope
            .allowed_domains
            .insert("partner.example".to_string());
        ctx.scope.third_party_assets.insert(
            "partner.example".to_string(),
            ThirdPartyConsent {
                written_consent_ref: "3P-2026-01".to_string(),
                approved_until_utc: "2026-03-02T00:00:00Z".to_string(),
            },
        );
        let err = ctx
            .scope
            .check_domain("partner.example", "2026-03-04T14:30:00Z")
            .expect_err("expired third party");
        assert!(matches!(err, ModuleError::ApprovalRequired(_)));
    }

    #[test]
    fn test_13_rfc2606_and_rfc1918_and_strict_ipv4_reject_public_targets() {
        let ctx = MockCapstoneLab::standard_context();
        assert!(matches!(
            ctx.scope.check_domain("evil.com", VIRTUAL_NOW_UTC),
            Err(ModuleError::OutOfScope(_))
        ));
        assert!(matches!(
            ctx.scope.check_ip("8.8.8.8"),
            Err(ModuleError::OutOfScope(_))
        ));
        assert!(matches!(
            parse_strict_ipv4("010.10.10.10"),
            Err(ModuleError::InvalidInput(_))
        ));
        assert!(matches!(
            parse_strict_rfc1918_cidr("10.10.10.5/24"),
            Err(ModuleError::InvalidInput(_))
        ));
        assert!(matches!(
            parse_utc_timestamp("2026-02-29T12:00:00Z"),
            Err(ModuleError::InvalidInput(_))
        ));
    }

    #[test]
    fn test_14_budget_overflow_and_max_modules_enforced() {
        let mut ctx = MockCapstoneLab::standard_context();
        ctx.budget.max_modules = 2;
        let sc = MockCapstoneLab::golden_scenario();
        let mut reg = MockCapstoneLab::standard_registry().expect("registry");
        let orch = CapstoneOrchestrator::new(
            ctx,
            BlueDefensePolicy::standard(),
            CapstoneRewardWeights::standard(),
        )
        .expect("orch");

        let err = orch.run_strict(&mut reg, &sc).expect_err("budget cap");
        assert!(matches!(err, OrchestratorError::BudgetExceeded(_)));
    }

    #[test]
    fn test_15_invalid_stage_transition_and_cycle_rejected() {
        assert!(!Stage::Osint.can_transition_to(Stage::AdConditionalValidation));
        assert!(!Stage::Reporting.can_transition_to(Stage::Osint));
        assert!(!Stage::AdConditionalValidation.can_transition_to(Stage::WirelessAssessment));
        assert!(Stage::AdEnumeration.can_transition_to(Stage::Reporting));

        struct BadTransitionMod;
        impl PentestModule for BadTransitionMod {
            fn id(&self) -> &'static str {
                "mod.ad.enum"
            }
            fn run(
                &mut self,
                context: &EngagementContext,
                _input: ModuleInput,
            ) -> Result<ModuleOutput, ModuleError> {
                let ev = EvidenceRecord::from_raw(
                    "EV-BAD",
                    Stage::AdEnumeration,
                    self.id(),
                    "virtual://corp.example/ad/bad",
                    "corp.example",
                    &context.current_virtual_utc,
                    "bad next stage test",
                )?;
                Ok(ModuleOutput {
                    module_id: "mod.ad.enum",
                    executed_stage: Stage::AdEnumeration,
                    findings: vec![],
                    excluded_false_positives: vec![],
                    evidence: vec![ev],
                    next_stage: Some(Stage::Osint), // Illegal backward cycle!
                    checkpoint: StageCheckpoint {
                        stage: Stage::AdEnumeration,
                        checkpoint_tag: "chk-bad".to_string(),
                        output_digest_sha256: "0".repeat(64),
                    },
                    telemetry_events: vec![],
                    handoff: None,
                    usage: ResourceUsage::default(),
                    remediation_notes: vec![],
                    cleanup_verified: true,
                    remaining_artifacts: 0,
                })
            }
        }

        let mut reg = MockCapstoneLab::standard_registry().expect("registry");
        reg.register(
            ModuleDescriptor {
                expected_id: "mod.ad.enum",
                stage: Stage::AdEnumeration,
                accepted_input: ModuleInputKind::AdEnumeration,
                required_prerequisites: vec![Stage::Osint],
                allowed_next_stages: vec![Stage::Reporting],
            },
            Box::new(BadTransitionMod),
        )
        .expect("register");

        let orch = CapstoneOrchestrator::new(
            MockCapstoneLab::standard_context(),
            BlueDefensePolicy::standard(),
            CapstoneRewardWeights::standard(),
        )
        .expect("orch");
        let err = orch
            .run_strict(&mut reg, &MockCapstoneLab::golden_scenario())
            .expect_err("must reject cycle");
        assert!(matches!(
            err,
            OrchestratorError::InvalidStageTransition {
                from: Stage::AdEnumeration,
                to: Stage::Osint
            }
        ));
    }

    #[test]
    fn test_16_registry_prerequisite_and_input_type_compatibility_checked() {
        let reg = MockCapstoneLab::standard_registry().expect("registry");
        let ctx = MockCapstoneLab::standard_context();
        let sc = MockCapstoneLab::golden_scenario();

        // Missing Osint prerequisite
        let err_prereq = reg.check_compatibility(
            Stage::AdEnumeration,
            &ModuleInput::AdEnumeration(sc.ad_enumeration.clone()),
            &[],
        );
        assert!(matches!(
            err_prereq,
            Err(OrchestratorError::IncompatibleModuleOrInput(_))
        ));

        // Mismatched typed ModuleInput
        let err_kind = reg.check_compatibility(
            Stage::Osint,
            &ModuleInput::AdEnumeration(sc.ad_enumeration),
            &[],
        );
        assert!(matches!(
            err_kind,
            Err(OrchestratorError::IncompatibleModuleOrInput(_))
        ));
        assert!(ctx.preflight_validate().is_ok());
    }

    #[test]
    fn test_17_unsupported_claim_rejected_without_or_with_unknown_evidence() {
        let mut ledger = EvidenceLedger::new();
        let ev = EvidenceRecord::from_raw(
            "EV-REAL-1",
            Stage::Osint,
            "mod.osint.passive",
            "virtual://corp.example/osint/ct",
            "corp.example",
            VIRTUAL_NOW_UTC,
            "ok",
        )
        .expect("ev");
        ledger.append_batch(vec![ev]).expect("append");

        let mut f = CapstoneFinding {
            finding_id: "F-EMPTY".to_string(),
            rule_id: "R1".to_string(),
            root_cause_key: "k1".to_string(),
            title: "Verified without evidence".to_string(),
            stage: Stage::Osint,
            affected_asset: "corp.example".to_string(),
            severity: FindingSeverity::High,
            cvss_tenths: 75,
            status: CapstoneFindingStatus::Verified,
            evidence_ids: BTreeSet::new(),
            telemetry_event_id: None,
            remediation: "fix".to_string(),
        };
        assert!(matches!(
            validate_finding_provenance(&f, &ledger),
            Err(OrchestratorError::UnsupportedClaim(_))
        ));

        f.evidence_ids.insert("EV-GHOST-999".to_string());
        assert!(matches!(
            validate_finding_provenance(&f, &ledger),
            Err(OrchestratorError::UnsupportedClaim(_))
        ));

        f.evidence_ids.clear();
        f.evidence_ids.insert("EV-REAL-1".to_string());
        assert!(validate_finding_provenance(&f, &ledger).is_ok());
    }

    #[test]
    fn test_18_evidence_chain_tamper_detection_and_duplicate_id_rejection() {
        let mut ledger = EvidenceLedger::new();
        let e1 = EvidenceRecord::from_raw(
            "EV-1",
            Stage::Osint,
            "mod.osint.passive",
            "virtual://corp.example/1",
            "corp.example",
            VIRTUAL_NOW_UTC,
            "entry 1",
        )
        .expect("e1");
        let e2 = EvidenceRecord::from_raw(
            "EV-2",
            Stage::AdEnumeration,
            "mod.ad.enum",
            "virtual://corp.example/2",
            "corp.example",
            VIRTUAL_NOW_UTC,
            "entry 2",
        )
        .expect("e2");
        ledger.append_batch(vec![e1.clone(), e2]).expect("append");
        assert!(ledger.verify().is_ok());

        // Duplicate evidence_id rejected atomically
        assert!(matches!(
            ledger.append_batch(vec![e1]),
            Err(OrchestratorError::EvidenceTampered(_))
        ));

        // Mutating summary invalidates hash chain
        let mut tampered = ledger;
        tampered.records[0].redacted_summary.push_str(" altered");
        assert!(matches!(
            tampered.verify(),
            Err(OrchestratorError::EvidenceTampered(_))
        ));
    }

    #[test]
    fn test_19_secret_redaction_and_bypass_resistance() {
        let raw = "Bearer syn_tok_9988776655 password=\"s3cr3t value\" {\"api_key\": \"ghp_1234567890\"} alice@gmail.com ops@corp.example -----BEGIN RSA PRIVATE KEY-----ABC-----END RSA PRIVATE KEY-----";
        let (clean, count) = redact_sensitive_text(raw);
        assert!(count >= 5);
        assert!(!clean.contains("syn_tok_9988776655"));
        assert!(!clean.contains("s3cr3t value"));
        assert!(!clean.contains("ghp_1234567890"));
        assert!(!clean.contains("alice@gmail.com"));
        assert!(clean.contains("ops@corp.example"));
        assert!(validate_no_raw_secret("clean", &clean).is_ok());

        // Bypass attempt: prefix match with leaked suffix must still fail
        assert!(matches!(
            validate_no_raw_secret("forged", "password=[REDACTED-SECRET]leaked"),
            Err(ModuleError::RawSecretDetected(_))
        ));
        assert!(matches!(
            validate_no_raw_secret("jwt", "header token eyJhbGciOiJIUzI1NiJ9.payload123.sig456"),
            Err(ModuleError::RawSecretDetected(_))
        ));
    }

    #[test]
    fn test_20_deduplication_is_permutation_independent_and_preserves_evidence() {
        let mut ev1 = BTreeSet::new();
        ev1.insert("EV-1".to_string());
        let mut ev2 = BTreeSet::new();
        ev2.insert("EV-2".to_string());
        let mut ev3 = BTreeSet::new();
        ev3.insert("EV-3".to_string());

        let a = CapstoneFinding {
            finding_id: "F-01".to_string(),
            rule_id: "AD-ACL".to_string(),
            root_cause_key: "WriteDacl:app01".to_string(),
            title: "Dup A".to_string(),
            stage: Stage::AdEnumeration,
            affected_asset: "corp.example".to_string(),
            severity: FindingSeverity::Medium,
            cvss_tenths: 65,
            status: CapstoneFindingStatus::Verified,
            evidence_ids: ev1,
            telemetry_event_id: None,
            remediation: "Rem A".to_string(),
        };
        let b = CapstoneFinding {
            finding_id: "F-02".to_string(),
            rule_id: "ad-acl".to_string(),
            root_cause_key: "writedacl:app01".to_string(),
            title: "Dup B".to_string(),
            stage: Stage::AdEnumeration,
            affected_asset: "CORP.EXAMPLE".to_string(),
            severity: FindingSeverity::High,
            cvss_tenths: 81,
            status: CapstoneFindingStatus::Verified,
            evidence_ids: ev2,
            telemetry_event_id: None,
            remediation: "Rem B".to_string(),
        };
        let c = CapstoneFinding {
            finding_id: "F-03".to_string(),
            rule_id: "AD-OTHER".to_string(),
            root_cause_key: "other".to_string(),
            title: "Unique".to_string(),
            stage: Stage::Osint,
            affected_asset: "corp.example".to_string(),
            severity: FindingSeverity::Low,
            cvss_tenths: 31,
            status: CapstoneFindingStatus::Verified,
            evidence_ids: ev3,
            telemetry_event_id: None,
            remediation: "Rem C".to_string(),
        };

        let d1 = deduplicate_capstone_findings(vec![a.clone(), b.clone(), c.clone()]).expect("d1");
        let d2 = deduplicate_capstone_findings(vec![c, a, b]).expect("d2");
        assert_eq!(d1, d2);
        assert_eq!(d1[0].finding_id, "F-02");
        assert_eq!(d1[0].evidence_ids.len(), 2);
        assert!(d1[0].evidence_ids.contains("EV-1"));
        assert!(d1[0].evidence_ids.contains("EV-2"));
    }

    #[test]
    fn test_21_checkpoint_and_resume_skip_committed_stages_and_match_bytes() {
        let ctx = MockCapstoneLab::standard_context();
        let sc = MockCapstoneLab::golden_scenario();
        let orch = CapstoneOrchestrator::new(
            ctx,
            BlueDefensePolicy::standard(),
            CapstoneRewardWeights::standard(),
        )
        .expect("orch");

        let mut reg_straight = MockCapstoneLab::standard_registry().expect("reg");
        let expected_report = orch.run_strict(&mut reg_straight, &sc).expect("straight");

        let mut reg_fail = MockCapstoneLab::registry_with_transient_ad_failure().expect("reg_fail");
        let cp = match orch.run_resumable(&mut reg_fail, &sc).expect("run") {
            RunOutcome::HaltedRecoverable {
                failed_stage,
                checkpoint,
                ..
            } => {
                assert_eq!(failed_stage, Stage::AdConditionalValidation);
                checkpoint
            }
            RunOutcome::Completed(_) => panic!("phải dừng tại lỗi tạm thời"),
        };

        assert_eq!(
            cp.completed_stages,
            vec![Stage::Osint, Stage::AdEnumeration]
        );
        assert_eq!(cp.next_stage, Stage::AdConditionalValidation);
        let ev_count_before_resume = cp.evidence_ledger.records().len();

        let mut reg_resume = MockCapstoneLab::standard_registry().expect("reg_resume");
        let resumed_report = match orch.resume(&cp, &mut reg_resume, &sc).expect("resume") {
            RunOutcome::Completed(rep) => rep,
            _ => panic!("resume phải hoàn tất"),
        };

        // Không chạy lại bất kỳ pha nào đã commit!
        assert_eq!(reg_resume.invocations_for(Stage::Osint), 0);
        assert_eq!(reg_resume.invocations_for(Stage::AdEnumeration), 0);
        assert_eq!(
            reg_resume.invocations_for(Stage::AdConditionalValidation),
            1
        );
        assert_eq!(reg_resume.invocations_for(Stage::Reporting), 1);

        // Không nhân đôi bằng chứng và khớp byte tuyệt đối với chạy thẳng
        assert_eq!(
            resumed_report.evidence.len(),
            expected_report.evidence.len()
        );
        assert!(resumed_report.evidence.len() > ev_count_before_resume);
        assert_eq!(
            resumed_report.render_json().expect("j1"),
            expected_report.render_json().expect("j2")
        );
        assert_eq!(
            resumed_report.render_markdown().expect("m1"),
            expected_report.render_markdown().expect("m2")
        );
    }

    #[test]
    fn test_22_checkpoint_rejects_tamper_wrong_version_and_mismatched_scenario() {
        let ctx = MockCapstoneLab::standard_context();
        let sc = MockCapstoneLab::golden_scenario();
        let orch = CapstoneOrchestrator::new(
            ctx,
            BlueDefensePolicy::standard(),
            CapstoneRewardWeights::standard(),
        )
        .expect("orch");
        let mut reg_fail = MockCapstoneLab::registry_with_transient_ad_failure().expect("reg_fail");
        let cp = match orch.run_resumable(&mut reg_fail, &sc).expect("run") {
            RunOutcome::HaltedRecoverable { checkpoint, .. } => checkpoint,
            _ => panic!("expected halt"),
        };

        let mut bad_ver = cp.clone();
        bad_ver.checkpoint_version = 99;
        let mut reg_res = MockCapstoneLab::standard_registry().expect("res");
        assert!(matches!(
            orch.resume(&bad_ver, &mut reg_res, &sc),
            Err(OrchestratorError::CheckpointInvalid(_))
        ));

        let mut bad_mac = cp.clone();
        bad_mac.usage.tool_calls += 500;
        assert!(matches!(
            orch.resume(&bad_mac, &mut reg_res, &sc),
            Err(OrchestratorError::CheckpointInvalid(_))
        ));

        let mut other_sc = sc;
        other_sc.noisy_ad_sweep = true;
        assert!(matches!(
            orch.resume(&cp, &mut reg_res, &other_sc),
            Err(OrchestratorError::CheckpointInvalid(_))
        ));
    }

    #[test]
    fn test_23_red_action_blocked_by_blue_is_never_counted_as_success() {
        let ctx = MockCapstoneLab::standard_context();
        let mut noisy = MockCapstoneLab::golden_scenario();
        noisy.noisy_ad_sweep = true;

        let mut reg = MockCapstoneLab::standard_registry().expect("reg");
        let orch = CapstoneOrchestrator::new(
            ctx,
            BlueDefensePolicy::standard(),
            CapstoneRewardWeights::standard(),
        )
        .expect("orch");
        let rep = orch.run_strict(&mut reg, &noisy).expect("rvb");

        assert_eq!(rep.blocked_red_actions.len(), 1);
        assert_eq!(
            rep.blocked_red_actions[0].stage,
            Stage::AdConditionalValidation
        );
        // Hai finding của bước noisy sweep phải chuyển sang BlockedByBlue trong excluded, KHÔNG nằm ở findings!
        assert!(
            !rep.findings
                .iter()
                .any(|f| f.stage == Stage::AdConditionalValidation)
        );
        assert!(
            rep.excluded_false_positives
                .iter()
                .filter(|f| f.status == CapstoneFindingStatus::BlockedByBlue)
                .count()
                >= 2
        );
    }

    #[test]
    fn test_24_rl_reward_weights_require_positive_penalties_and_subtract_correctly() {
        let mut bad = CapstoneRewardWeights::standard();
        bad.detection_penalty = -3.0;
        assert!(matches!(
            bad.validate(),
            Err(OrchestratorError::RewardConfigInvalid(_))
        ));
        bad.detection_penalty = f64::NAN;
        assert!(matches!(
            bad.validate(),
            Err(OrchestratorError::RewardConfigInvalid(_))
        ));

        let w = CapstoneRewardWeights::standard();
        // 1 finding verified (+10.0) - detect (3.0) - block (6.0) - step (0.5) = 0.5
        // Nếu block mà verified == 0 thì -3.0 - 6.0 - 0.5 = -9.5
        let blocked_reward = w
            .compute_step_reward(&RlStepOutcome {
                verified_findings_added: 0,
                clean_report_achieved: false,
                detected_by_blue: true,
                blocked_by_blue_count: 1,
                service_disrupted: false,
                safety_violations: 0,
            })
            .expect("reward");
        assert!((blocked_reward - (-9.5)).abs() < 1e-12);

        // Q-policy học tránh hành động NoisyBulkSweep sau khi bị phạt -9.5
        let mut q = CapstoneQPolicy::new();
        q.update(AdaptivePolicyAction::NoisyBulkSweep, blocked_reward);
        q.update(AdaptivePolicyAction::TargetedScopedCheck, 16.5);
        assert_eq!(q.best_action(), AdaptivePolicyAction::TargetedScopedCheck);
    }

    #[test]
    fn test_25_cleanup_incomplete_blocks_post_audit_and_report() {
        let ctx = MockCapstoneLab::standard_context();
        let mut sc = MockCapstoneLab::full_with_branches_scenario();
        sc.post_audit.as_mut().expect("post").simulate_dirty_cleanup = true;
        let mut reg = MockCapstoneLab::standard_registry().expect("reg");
        let orch = CapstoneOrchestrator::new(
            ctx,
            BlueDefensePolicy::standard(),
            CapstoneRewardWeights::standard(),
        )
        .expect("orch");
        let err = orch.run_strict(&mut reg, &sc).expect_err("dirty cleanup");
        assert!(matches!(
            err,
            OrchestratorError::ModuleFailed {
                stage: Stage::PostExploitationAudit,
                error: ModuleError::CleanupIncomplete(_)
            }
        ));

        let err_rep = ReportingModuleAdapter
            .run(
                &MockCapstoneLab::standard_context(),
                ModuleInput::Reporting(ReportingInput {
                    report_title: "Dirty".to_string(),
                    ledger_count: 1,
                    ledger_tip_hash: LEDGER_GENESIS_HASH.to_string(),
                    verified_finding_count: 0,
                    remaining_artifacts: 2,
                }),
            )
            .expect_err("reporting must refuse dirty cleanup");
        assert!(matches!(err_rep, ModuleError::CleanupIncomplete(_)));

        // Ngay cả trên report đã dựng nếu remaining_lab_artifacts > 0 vẫn bị report.validate() từ chối
        let mut reg_ok = MockCapstoneLab::standard_registry().expect("reg_ok");
        let mut rep = orch
            .run_strict(&mut reg_ok, &MockCapstoneLab::golden_scenario())
            .expect("ok");
        rep.cleanup.remaining_lab_artifacts = 1;
        rep.cleanup.verified_clean = false;
        rep.report_digest_sha256 = rep.compute_digest();
        assert!(matches!(
            rep.validate(),
            Err(OrchestratorError::CleanupIncomplete(_))
        ));
    }

    #[test]
    fn test_26_report_rendering_escaping_and_tamper_detection() {
        let ctx = MockCapstoneLab::standard_context();
        let mut sc = MockCapstoneLab::golden_scenario();
        sc.report_title = "# Heading | <script>\n\"quoted\"".to_string();
        let mut reg = MockCapstoneLab::standard_registry().expect("reg");
        let orch = CapstoneOrchestrator::new(
            ctx,
            BlueDefensePolicy::standard(),
            CapstoneRewardWeights::standard(),
        )
        .expect("orch");
        let rep = orch.run_strict(&mut reg, &sc).expect("run");

        let json = rep.render_json().expect("json");
        assert!(json.contains("\\\"quoted\\\""));
        assert!(!json.contains("\n"));

        let md = rep.render_markdown().expect("md");
        assert!(md.contains("&lt;script&gt;"));
        assert!(md.contains("\\|"));
        assert!(!md.contains("<script>"));

        let mut tampered = rep.clone();
        tampered.title.push_str(" altered");
        assert!(matches!(
            tampered.validate(),
            Err(OrchestratorError::ReportTampered(_))
        ));

        let mut bad_schema = rep;
        bad_schema.schema_version = "0.0.1".to_string();
        assert!(matches!(
            bad_schema.render_json(),
            Err(OrchestratorError::ReportTampered(_))
        ));
    }

    #[test]
    fn test_27_sha256_fips_180_4_and_splitmix64_determinism() {
        assert_eq!(
            sha256_hex(b"abc"),
            "ba7816bf8f01cfea414140de5dae2223b00361a396177a9cb410ff61f20015ad"
        );
        let mut r1 = SplitMix64::new(112);
        let mut r2 = SplitMix64::new(112);
        assert_eq!(r1.next_tag(), r2.next_tag());
        assert_eq!(r1.next_u64(), r2.next_u64());
    }

    #[test]
    fn test_28_report_contains_no_raw_secret_even_after_manual_injection() {
        let ctx = MockCapstoneLab::standard_context();
        let sc = MockCapstoneLab::golden_scenario();
        let mut reg = MockCapstoneLab::standard_registry().expect("reg");
        let orch = CapstoneOrchestrator::new(
            ctx,
            BlueDefensePolicy::standard(),
            CapstoneRewardWeights::standard(),
        )
        .expect("orch");
        let mut rep = orch.run_strict(&mut reg, &sc).expect("rep");
        rep.findings[0].remediation = "rotate password=supersecret".to_string();
        rep.report_digest_sha256 = rep.compute_digest();
        assert!(matches!(
            rep.validate(),
            Err(OrchestratorError::RawSecretDetected(_))
        ));
    }

    #[test]
    fn test_29_evaluator_rejects_broken_report_or_missing_limitations() {
        let ctx = MockCapstoneLab::standard_context();
        let sc = MockCapstoneLab::golden_scenario();
        let mut reg = MockCapstoneLab::standard_registry().expect("reg");
        let orch = CapstoneOrchestrator::new(
            ctx,
            BlueDefensePolicy::standard(),
            CapstoneRewardWeights::standard(),
        )
        .expect("orch");
        let mut rep = orch.run_strict(&mut reg, &sc).expect("rep");
        rep.limitations.clear();
        rep.report_digest_sha256 = rep.compute_digest();
        assert!(matches!(
            rep.validate(),
            Err(OrchestratorError::ReportTampered(_))
        ));
    }

    #[test]
    fn test_30_all_seven_report_sections_present_in_markdown() {
        let ctx = MockCapstoneLab::standard_context();
        let sc = MockCapstoneLab::golden_scenario();
        let mut reg = MockCapstoneLab::standard_registry().expect("reg");
        let orch = CapstoneOrchestrator::new(
            ctx,
            BlueDefensePolicy::standard(),
            CapstoneRewardWeights::standard(),
        )
        .expect("orch");
        let rep = orch.run_strict(&mut reg, &sc).expect("rep");
        let md = rep.render_markdown().expect("md");
        for section in [
            "## 1. Scope & Authorization",
            "## 2. Module Trace",
            "## 3. Evidence Chain",
            "## 4. Verified Findings",
            "## 5. Remediation Plan",
            "## 6. Cleanup & Red-vs-Blue Telemetry",
            "## 7. Limitations",
        ] {
            assert!(md.contains(section), "thiếu mục `{section}`");
        }
    }

    #[test]
    fn test_31_utf8_vietnamese_text_with_secrets_does_not_panic() {
        let raw = "Bản ghi mật khẩu user_id=alice password=\"bí mật 123\" tại máy chủ nội bộ";
        let (clean, count) = redact_sensitive_text(raw);
        assert_eq!(count, 1);
        assert!(clean.contains("Bản ghi mật khẩu"));
        assert!(!clean.contains("bí mật 123"));
        assert!(validate_no_raw_secret("utf8_clean", &clean).is_ok());
    }

    #[test]
    fn test_32_out_of_scope_downstream_ad_target_stops_before_first_module() {
        let ctx = MockCapstoneLab::standard_context();
        let mut sc = MockCapstoneLab::golden_scenario();
        sc.ad_enumeration.dc_ip = "10.10.10.250".to_string(); // trong denied_ips!
        let mut reg = MockCapstoneLab::standard_registry().expect("registry");
        let orch = CapstoneOrchestrator::new(
            ctx,
            BlueDefensePolicy::standard(),
            CapstoneRewardWeights::standard(),
        )
        .expect("orch");

        let err = orch
            .run_strict(&mut reg, &sc)
            .expect_err("preflight must catch");
        assert!(matches!(err, OrchestratorError::ScopeViolation(_)));
        assert_eq!(reg.total_invocations(), 0);
    }

    #[test]
    fn test_33_verified_finding_without_remediation_rejected() {
        let mut ledger = EvidenceLedger::new();
        let ev = EvidenceRecord::from_raw(
            "EV-REM-1",
            Stage::Osint,
            "mod.osint.passive",
            "virtual://corp.example/osint/rem",
            "corp.example",
            VIRTUAL_NOW_UTC,
            "snapshot",
        )
        .expect("ev");
        ledger.append_batch(vec![ev]).expect("append");

        let mut evs = BTreeSet::new();
        evs.insert("EV-REM-1".to_string());
        let finding = CapstoneFinding {
            finding_id: "F-NO-REM".to_string(),
            rule_id: "R1".to_string(),
            root_cause_key: "k1".to_string(),
            title: "Missing remediation".to_string(),
            stage: Stage::Osint,
            affected_asset: "corp.example".to_string(),
            severity: FindingSeverity::Medium,
            cvss_tenths: 50,
            status: CapstoneFindingStatus::Verified,
            evidence_ids: evs,
            telemetry_event_id: None,
            remediation: "   ".to_string(),
        };
        assert!(matches!(
            validate_finding_provenance(&finding, &ledger),
            Err(OrchestratorError::UnsupportedClaim(_))
        ));
    }

    #[test]
    fn test_34_tampered_checkpoint_handoff_or_rl_step_rejected() {
        let ctx = MockCapstoneLab::standard_context();
        let sc = MockCapstoneLab::golden_scenario();
        let orch = CapstoneOrchestrator::new(
            ctx,
            BlueDefensePolicy::standard(),
            CapstoneRewardWeights::standard(),
        )
        .expect("orch");
        let mut reg_fail = MockCapstoneLab::registry_with_transient_ad_failure().expect("reg_fail");
        let cp = match orch.run_resumable(&mut reg_fail, &sc).expect("run") {
            RunOutcome::HaltedRecoverable { checkpoint, .. } => checkpoint,
            _ => panic!("expected halt"),
        };

        let mut bad_rl = cp.clone();
        bad_rl.rl_steps[0].reward += 100.0;
        let mut reg_res = MockCapstoneLab::standard_registry().expect("res");
        assert!(matches!(
            orch.resume(&bad_rl, &mut reg_res, &sc),
            Err(OrchestratorError::CheckpointInvalid(_))
        ));

        let mut bad_handoff = cp;
        bad_handoff.ad_preconditions_handoff = Some((vec!["tampered_spn".to_string()], vec![]));
        assert!(matches!(
            orch.resume(&bad_handoff, &mut reg_res, &sc),
            Err(OrchestratorError::CheckpointInvalid(_))
        ));
    }

    #[test]
    fn test_35_tampered_report_finding_evidence_ids_or_blocked_actions_rejected() {
        let ctx = MockCapstoneLab::standard_context();
        let mut noisy = MockCapstoneLab::golden_scenario();
        noisy.noisy_ad_sweep = true;
        let mut reg = MockCapstoneLab::standard_registry().expect("reg");
        let orch = CapstoneOrchestrator::new(
            ctx,
            BlueDefensePolicy::standard(),
            CapstoneRewardWeights::standard(),
        )
        .expect("orch");
        let mut rep = orch.run_strict(&mut reg, &noisy).expect("rep");
        rep.blocked_red_actions[0].reason.push_str(" forged");
        assert!(matches!(
            rep.validate(),
            Err(OrchestratorError::ReportTampered(_))
        ));
    }

    #[test]
    fn test_36_blue_defense_policy_distinguishes_silent_alert_and_block() {
        let policy = BlueDefensePolicy::standard();
        let silent = ModuleTelemetryEvent {
            event_id: "E1".to_string(),
            stage: Stage::Osint,
            actor_module_id: "mod.osint.passive".to_string(),
            action_kind: "passive_ct_dns_lookup".to_string(),
            target: "corp.example".to_string(),
            noise_score: 5,
        };
        let alert = ModuleTelemetryEvent {
            event_id: "E2".to_string(),
            stage: Stage::AdConditionalValidation,
            actor_module_id: "mod.ad.conditional".to_string(),
            action_kind: "scoped_spn_metadata_check".to_string(),
            target: "corp.example".to_string(),
            noise_score: 30,
        };
        let block = ModuleTelemetryEvent {
            event_id: "E3".to_string(),
            stage: Stage::AdConditionalValidation,
            actor_module_id: "mod.ad.conditional".to_string(),
            action_kind: "bulk_spn_sweep".to_string(),
            target: "corp.example".to_string(),
            noise_score: 90,
        };
        assert_eq!(
            policy.evaluate(&silent).disposition,
            BlueDisposition::AllowSilent
        );
        assert_eq!(
            policy.evaluate(&alert).disposition,
            BlueDisposition::DetectAndAlert
        );
        assert_eq!(
            policy.evaluate(&block).disposition,
            BlueDisposition::DetectAndBlock
        );
    }

    #[test]
    fn test_37_synthetic_location_rejects_non_example_or_public_ip_host() {
        assert!(validate_synthetic_location("virtual://corp.example/osint/ct").is_ok());
        assert!(validate_synthetic_location("lab-sensor://10.10.10.10/wlan/cap").is_ok());
        assert!(matches!(
            validate_synthetic_location("virtual://evil.com/leak"),
            Err(ModuleError::OutOfScope(_))
        ));
        assert!(matches!(
            validate_synthetic_location("virtual://8.8.8.8/dns"),
            Err(ModuleError::OutOfScope(_))
        ));
        assert!(matches!(
            validate_synthetic_location("https://corp.example/api"),
            Err(ModuleError::OutOfScope(_))
        ));
    }

    #[test]
    fn test_38_mutated_scenario_org_handle_or_acl_invalidates_checkpoint_on_resume() {
        let ctx = MockCapstoneLab::standard_context();
        let sc = MockCapstoneLab::golden_scenario();
        let orch = CapstoneOrchestrator::new(
            ctx,
            BlueDefensePolicy::standard(),
            CapstoneRewardWeights::standard(),
        )
        .expect("orch");
        let mut reg_fail = MockCapstoneLab::registry_with_transient_ad_failure().expect("reg_fail");
        let cp = match orch.run_resumable(&mut reg_fail, &sc).expect("run") {
            RunOutcome::HaltedRecoverable { checkpoint, .. } => checkpoint,
            _ => panic!("expected halt"),
        };

        let mut mutated = sc;
        mutated.osint.org_handle = "ORG-TAMPERED".to_string();
        let mut reg_res = MockCapstoneLab::standard_registry().expect("res");
        assert!(matches!(
            orch.resume(&cp, &mut reg_res, &mutated),
            Err(OrchestratorError::CheckpointInvalid(_))
        ));
    }

    #[test]
    fn test_39_unclosed_quote_with_utf8_multibyte_does_not_panic() {
        for sample in ["password=\"á", "Bearer \"á", "secret='tiếng việt"] {
            let (clean, count) = redact_sensitive_text(sample);
            assert_eq!(count, 1);
            assert!(clean.contains(REDACTED_SECRET_MARKER));
            assert!(validate_no_raw_secret("unclosed_utf8", &clean).is_ok());
        }
    }

    #[test]
    fn test_40_token_prefix_and_compound_snake_case_redaction_bypasses_blocked() {
        let bypass_inputs = [
            "token=ghp_1234567890",
            "token=eyJhbGciOiJIUzI1NiJ9.payload123.sig456",
            "aws_key=AKIAIOSFODNN7EXAMPLE",
            "token=sk_live_9876543210",
            "https://corp.example/ghp_1234567890",
            "db_password=hunter2",
            "my_secret=secretvalue123",
        ];
        for raw in bypass_inputs {
            assert!(
                matches!(
                    validate_no_raw_secret("bypass", raw),
                    Err(ModuleError::RawSecretDetected(_))
                ),
                "phải chặn secret thô `{raw}`"
            );
            let (clean, count) = redact_sensitive_text(raw);
            assert!(count >= 1, "phải che `{raw}`");
            assert!(
                validate_no_raw_secret("after_redact", &clean).is_ok(),
                "sau redaction vẫn còn secret trong `{clean}`"
            );
        }
    }

    #[test]
    fn test_41_report_tampered_evidence_summary_or_uri_without_digest_change_caught() {
        let ctx = MockCapstoneLab::standard_context();
        let sc = MockCapstoneLab::golden_scenario();
        let mut reg = MockCapstoneLab::standard_registry().expect("reg");
        let orch = CapstoneOrchestrator::new(
            ctx,
            BlueDefensePolicy::standard(),
            CapstoneRewardWeights::standard(),
        )
        .expect("orch");
        let rep = orch.run_strict(&mut reg, &sc).expect("rep");

        let mut bad_ev_summary = rep.clone();
        bad_ev_summary.evidence[0].redacted_summary =
            "forged without changing report digest".to_string();
        assert!(matches!(
            bad_ev_summary.validate(),
            Err(OrchestratorError::ReportTampered(_))
        ));

        let mut bad_ev_uri = rep.clone();
        bad_ev_uri.evidence[0].source_uri = "https://evil.com/leak".to_string();
        assert!(matches!(
            bad_ev_uri.validate(),
            Err(OrchestratorError::ReportTampered(_))
        ));

        let mut bad_trace_count = rep.clone();
        bad_trace_count.module_trace[0].verified_findings_added = 99;
        assert!(matches!(
            bad_trace_count.validate(),
            Err(OrchestratorError::ReportTampered(_))
        ));

        let mut fp_in_main = rep;
        fp_in_main.findings[0].status = CapstoneFindingStatus::FalsePositive;
        fp_in_main.report_digest_sha256 = fp_in_main.compute_digest();
        assert!(matches!(
            fp_in_main.validate(),
            Err(OrchestratorError::ReportTampered(_))
        ));
    }

    #[test]
    fn test_42_checkpoint_tampered_finding_severity_or_policy_mismatch_rejected() {
        let ctx = MockCapstoneLab::standard_context();
        let sc = MockCapstoneLab::golden_scenario();
        let orch = CapstoneOrchestrator::new(
            ctx.clone(),
            BlueDefensePolicy::standard(),
            CapstoneRewardWeights::standard(),
        )
        .expect("orch");
        let mut reg_fail = MockCapstoneLab::registry_with_transient_ad_failure().expect("reg_fail");
        let cp = match orch.run_resumable(&mut reg_fail, &sc).expect("run") {
            RunOutcome::HaltedRecoverable { checkpoint, .. } => checkpoint,
            _ => panic!("expected halt"),
        };

        let mut bad_sev = cp.clone();
        bad_sev.verified_findings[0].severity = FindingSeverity::Critical;
        let mut reg_res = MockCapstoneLab::standard_registry().expect("res");
        assert!(matches!(
            orch.resume(&bad_sev, &mut reg_res, &sc),
            Err(OrchestratorError::CheckpointInvalid(_))
        ));

        // Đổi policy Blue giữa run và resume phải bị chặn bởi fingerprint
        let stricter_orch = CapstoneOrchestrator::new(
            ctx,
            BlueDefensePolicy {
                block_noisy_spn_sweep: true,
                block_noise_threshold: 10,
                alert_noise_threshold: 5,
            },
            CapstoneRewardWeights::standard(),
        )
        .expect("stricter");
        assert!(matches!(
            stricter_orch.resume(&cp, &mut reg_res, &sc),
            Err(OrchestratorError::CheckpointInvalid(_))
        ));
    }

    #[test]
    fn test_43_post_audit_branch_flag_mismatch_and_multiple_acl_edges() {
        let ctx = MockCapstoneLab::standard_context();
        let mut sc = MockCapstoneLab::full_with_branches_scenario();
        // enable_post_audit_branch == false dù post_audit == Some(...): KHÔNG chạy PostAudit!
        sc.ad_enumeration.enable_post_audit_branch = false;
        sc.ad_enumeration.acl_edges.push(AdAclEdge {
            source_principal: "svc_backup_lab".to_string(),
            right: "GenericAll".to_string(),
            target_object: "dc01.corp.example".to_string(),
        });

        let mut reg = MockCapstoneLab::standard_registry().expect("reg");
        let orch = CapstoneOrchestrator::new(
            ctx.clone(),
            BlueDefensePolicy::standard(),
            CapstoneRewardWeights::standard(),
        )
        .expect("orch");
        let rep = orch.run_strict(&mut reg, &sc).expect("run");
        assert_eq!(reg.invocations_for(Stage::PostExploitationAudit), 0);
        assert!(
            rep.findings
                .iter()
                .any(|f| f.finding_id == "CAP-AD-ACL-001")
        );
        assert!(
            rep.findings
                .iter()
                .any(|f| f.finding_id == "CAP-AD-ACL-002")
        );

        // Kiểm tra preflight chặn persistence item trỏ vào domain bị deny
        let mut bad_post = MockCapstoneLab::full_with_branches_scenario();
        bad_post
            .post_audit
            .as_mut()
            .expect("post")
            .persistence_items[0]
            .virtual_location = "virtual://payroll.corp.example/scheduler/task".to_string();
        let mut reg2 = MockCapstoneLab::standard_registry().expect("reg2");
        let err = orch
            .run_strict(&mut reg2, &bad_post)
            .expect_err("must block");
        assert!(matches!(err, OrchestratorError::ScopeViolation(_)));
        assert_eq!(reg2.total_invocations(), 0);
    }

    #[test]
    fn test_44_approval_target_scope_and_third_party_subdomain_enforced() {
        let mut ctx = MockCapstoneLab::standard_context();
        ctx.granted_approvals[0].target_scope = "other-lab.example".to_string();
        let sc = MockCapstoneLab::golden_scenario();
        let mut reg = MockCapstoneLab::standard_registry().expect("reg");
        let orch = CapstoneOrchestrator::new(
            ctx,
            BlueDefensePolicy::standard(),
            CapstoneRewardWeights::standard(),
        )
        .expect("orch");
        assert!(matches!(
            orch.run_strict(&mut reg, &sc),
            Err(OrchestratorError::ApprovalRequired(_))
        ));
        assert_eq!(reg.total_invocations(), 0);

        // Kiểm tra third_party_assets chặn cả subdomain khi parent hết hạn chấp thuận
        let mut ctx2 = MockCapstoneLab::standard_context();
        ctx2.scope
            .allow_subdomains_of
            .insert("partner.example".to_string());
        ctx2.scope.third_party_assets.insert(
            "partner.example".to_string(),
            ThirdPartyConsent {
                written_consent_ref: "3P-2026-SUB".to_string(),
                approved_until_utc: "2026-03-01T00:00:00Z".to_string(),
            },
        );
        assert!(matches!(
            ctx2.scope
                .check_domain("api.partner.example", VIRTUAL_NOW_UTC),
            Err(ModuleError::ApprovalRequired(_))
        ));
    }

    #[test]
    fn test_45_dedup_three_duplicate_remediations_and_dangling_telemetry_event_rejected() {
        let make_finding = |id: &str, ev_id: &str, rem: &str| {
            let mut evs = BTreeSet::new();
            evs.insert(ev_id.to_string());
            CapstoneFinding {
                finding_id: id.to_string(),
                rule_id: "DUP-R".to_string(),
                root_cause_key: "same-cause".to_string(),
                title: "Same issue".to_string(),
                stage: Stage::Osint,
                affected_asset: "corp.example".to_string(),
                severity: FindingSeverity::Medium,
                cvss_tenths: 50,
                status: CapstoneFindingStatus::Verified,
                evidence_ids: evs,
                telemetry_event_id: None,
                remediation: rem.to_string(),
            }
        };
        let dedup = deduplicate_capstone_findings(vec![
            make_finding("F1", "EV-1", "A"),
            make_finding("F2", "EV-2", "B"),
            make_finding("F3", "EV-3", "A"),
        ])
        .expect("dedup");
        assert_eq!(dedup.len(), 1);
        assert_eq!(dedup[0].remediation, "A; B");

        struct DanglingTelemetryMod;
        impl PentestModule for DanglingTelemetryMod {
            fn id(&self) -> &'static str {
                "mod.ad.conditional"
            }
            fn run(
                &mut self,
                context: &EngagementContext,
                _input: ModuleInput,
            ) -> Result<ModuleOutput, ModuleError> {
                let ev = EvidenceRecord::from_raw(
                    "EV-DANGLE",
                    Stage::AdConditionalValidation,
                    self.id(),
                    "virtual://corp.example/ad/check",
                    "corp.example",
                    &context.current_virtual_utc,
                    "telemetry evasion attempt",
                )?;
                let mut evs = BTreeSet::new();
                evs.insert(ev.evidence_id.clone());
                Ok(ModuleOutput {
                    module_id: "mod.ad.conditional",
                    executed_stage: Stage::AdConditionalValidation,
                    findings: vec![CapstoneFinding {
                        finding_id: "F-DODGE".to_string(),
                        rule_id: "R".to_string(),
                        root_cause_key: "k".to_string(),
                        title: "Dodged".to_string(),
                        stage: Stage::AdConditionalValidation,
                        affected_asset: "corp.example".to_string(),
                        severity: FindingSeverity::High,
                        cvss_tenths: 80,
                        status: CapstoneFindingStatus::Verified,
                        evidence_ids: evs,
                        telemetry_event_id: Some("EVENT-KHONG-TON-TAI".to_string()),
                        remediation: "Fix".to_string(),
                    }],
                    excluded_false_positives: vec![],
                    evidence: vec![ev],
                    next_stage: Some(Stage::Reporting),
                    checkpoint: StageCheckpoint {
                        stage: Stage::AdConditionalValidation,
                        checkpoint_tag: "chk-d".to_string(),
                        output_digest_sha256: "0".repeat(64),
                    },
                    telemetry_events: vec![ModuleTelemetryEvent {
                        event_id: "TEL-REAL-BLOCKED".to_string(),
                        stage: Stage::AdConditionalValidation,
                        actor_module_id: self.id().to_string(),
                        action_kind: "bulk_spn_sweep".to_string(),
                        target: "corp.example".to_string(),
                        noise_score: 95,
                    }],
                    handoff: None,
                    usage: ResourceUsage::default(),
                    remediation_notes: vec![],
                    cleanup_verified: true,
                    remaining_artifacts: 0,
                })
            }
        }

        let mut reg = MockCapstoneLab::standard_registry().expect("reg");
        reg.register(
            ModuleDescriptor {
                expected_id: "mod.ad.conditional",
                stage: Stage::AdConditionalValidation,
                accepted_input: ModuleInputKind::AdConditionalValidation,
                required_prerequisites: vec![Stage::Osint, Stage::AdEnumeration],
                allowed_next_stages: vec![Stage::Reporting],
            },
            Box::new(DanglingTelemetryMod),
        )
        .expect("reg mod");
        let orch = CapstoneOrchestrator::new(
            MockCapstoneLab::standard_context(),
            BlueDefensePolicy::standard(),
            CapstoneRewardWeights::standard(),
        )
        .expect("orch");
        let err = orch
            .run_strict(&mut reg, &MockCapstoneLab::golden_scenario())
            .expect_err("must reject dangling telemetry_event_id");
        assert!(matches!(err, OrchestratorError::UnsupportedClaim(_)));
    }

    #[test]
    fn test_46_render_json_includes_remediation_evidence_and_blocked_details() {
        let ctx = MockCapstoneLab::standard_context();
        let mut sc = MockCapstoneLab::golden_scenario();
        sc.noisy_ad_sweep = true;
        let mut reg = MockCapstoneLab::standard_registry().expect("reg");
        let orch = CapstoneOrchestrator::new(
            ctx,
            BlueDefensePolicy::standard(),
            CapstoneRewardWeights::standard(),
        )
        .expect("orch");
        let rep = orch.run_strict(&mut reg, &sc).expect("rep");
        let json = rep.render_json().expect("json");
        assert!(json.contains("\"remediation_plan\":["));
        assert!(json.contains("\"evidence\":["));
        assert!(json.contains("\"blocked_actions_detail\":["));
        assert!(json.contains("\"blue_alerts_detail\":["));
        assert!(json.contains("\"remediation\":\""));
    }
}
