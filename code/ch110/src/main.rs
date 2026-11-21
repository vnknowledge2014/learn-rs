#![forbid(unsafe_code)]

//! # Chương 110 — Post-Exploitation & Persistence Audit (`MockPostExploit`)
//!
//! Crate minh họa 100% clean-room và thuần `std`:
//! - Kiểm kê trạng thái hậu khai thác trên `SystemSnapshot` chỉ đọc (read-only).
//! - So sánh baseline → current across services, scheduled tasks, startup entries, autoruns, accounts và security policy.
//! - Phân tích access inventory & lateral-movement graph (DFS phát hiện chu trình, phạm vi scope, segmentation, privilege, session expiry).
//! - Correlate findings với `TelemetryEvent`, phát hiện telemetry gaps do policy tamper (100% defensive engineering).
//! - Phân loại rủi ro rò rỉ dữ liệu qua `DataFlow` metadata thuần túy.
//! - Lập và thực thi `CleanupPlan` nguyên tử trên `MockCleanupState` cô lập: bảo vệ baseline, xử lý artifact đã mất và lũy đẳng tuyệt đối.

use std::collections::{BTreeMap, BTreeSet};
use std::fmt;

pub const ENGAGEMENT_ID: &str = "ENG-2026-POSTEX-01";
pub const REPORT_SCHEMA_VERSION: &str = "1.0.0";
pub const BASELINE_TIMESTAMP: &str = "2026-01-10T00:00:00Z";
pub const AUDIT_TIMESTAMP: &str = "2026-01-10T02:15:00Z";
pub const EXPIRY_PAST_TIMESTAMP: &str = "2026-01-10T01:30:00Z";
pub const EXPIRY_FUTURE_TIMESTAMP: &str = "2026-01-10T06:00:00Z";
pub const CLEANUP_TIMESTAMP: &str = "2026-01-10T03:00:00Z";
pub const REDACTED_CREDENTIAL_MARKER: &str = "SYNTHETIC-REDACTED-CREDENTIAL";

/// Hàm băm SHA-256 tự cài chuẩn FIPS 180-4 dùng để niêm phong digest artifact và `input_hash`.
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

// ============================================================================
// 1. LỖI, KIỂM CHỨNG ĐỊNH DANH, BOUNDARIES & CHỐNG THU THẬP CREDENTIAL
// ============================================================================

#[derive(Debug, Clone, PartialEq, Eq)]
pub enum PostExploitError {
    OutOfScope(String),
    InvalidDomain(String),
    InvalidRfc1918Ip(String),
    InvalidLocation(String),
    InvalidTimestamp(String),
    RawSecretProhibited(String),
    CredentialHarvestingProhibited(String),
    SnapshotMismatch(String),
    MissingEvidence(String),
    SeverityExceedsImpact {
        severity: Severity,
        impact: ImpactLevel,
    },
    BaselineProtected(String),
    ArtifactDigestMismatch {
        artifact_id: String,
        expected: String,
        actual: String,
    },
    DuplicateIdentifier(String),
    ReportSchemaInvalid(String),
    ValidationFailed(String),
}

impl fmt::Display for PostExploitError {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            Self::OutOfScope(msg) => write!(f, "OutOfScope({msg})"),
            Self::InvalidDomain(msg) => write!(f, "InvalidDomain({msg})"),
            Self::InvalidRfc1918Ip(msg) => write!(f, "InvalidRfc1918Ip({msg})"),
            Self::InvalidLocation(msg) => write!(f, "InvalidLocation({msg})"),
            Self::InvalidTimestamp(msg) => write!(f, "InvalidTimestamp({msg})"),
            Self::RawSecretProhibited(msg) => write!(f, "RawSecretProhibited({msg})"),
            Self::CredentialHarvestingProhibited(msg) => {
                write!(f, "CredentialHarvestingProhibited({msg})")
            }
            Self::SnapshotMismatch(msg) => write!(f, "SnapshotMismatch({msg})"),
            Self::MissingEvidence(msg) => write!(f, "MissingEvidence({msg})"),
            Self::SeverityExceedsImpact { severity, impact } => {
                write!(
                    f,
                    "SeverityExceedsImpact(severity={severity:?}, impact={impact:?})"
                )
            }
            Self::BaselineProtected(msg) => write!(f, "BaselineProtected({msg})"),
            Self::ArtifactDigestMismatch {
                artifact_id,
                expected,
                actual,
            } => write!(
                f,
                "ArtifactDigestMismatch({artifact_id}: expected {expected}, got {actual})"
            ),
            Self::DuplicateIdentifier(msg) => write!(f, "DuplicateIdentifier({msg})"),
            Self::ReportSchemaInvalid(msg) => write!(f, "ReportSchemaInvalid({msg})"),
            Self::ValidationFailed(msg) => write!(f, "ValidationFailed({msg})"),
        }
    }
}

impl std::error::Error for PostExploitError {}

/// Chuẩn hóa tên miền dành riêng `.example` (RFC 2606), không cho phép FQDN ngoài `.example`.
pub fn normalize_example_domain(raw: &str) -> Result<String, PostExploitError> {
    let trimmed = raw.trim();
    if trimmed.is_empty() || trimmed.len() > 253 {
        return Err(PostExploitError::InvalidDomain(format!(
            "tên miền '{raw}' rỗng hoặc quá dài"
        )));
    }
    let lowered = trimmed.to_ascii_lowercase();
    let without_dot = lowered.strip_suffix('.').unwrap_or(&lowered);
    if without_dot.is_empty() || without_dot.ends_with('.') || without_dot.starts_with('.') {
        return Err(PostExploitError::InvalidDomain(format!(
            "cú pháp dấu chấm không hợp lệ tại '{raw}'"
        )));
    }
    let labels: Vec<&str> = without_dot.split('.').collect();
    if labels.len() < 2 {
        return Err(PostExploitError::InvalidDomain(format!(
            "tên miền '{raw}' phải có ít nhất 2 nhãn"
        )));
    }
    for label in &labels {
        if label.is_empty()
            || label.len() > 63
            || label.starts_with('-')
            || label.ends_with('-')
            || !label
                .bytes()
                .all(|b| b.is_ascii_alphanumeric() || b == b'-')
        {
            return Err(PostExploitError::InvalidDomain(format!(
                "nhãn tên miền không hợp lệ trong '{raw}'"
            )));
        }
    }
    if labels.last() != Some(&"example") {
        return Err(PostExploitError::InvalidDomain(format!(
            "chỉ chấp nhận tên miền TLD .example (RFC 2606), nhận '{raw}'"
        )));
    }
    Ok(without_dot.to_string())
}

/// Kiểm tra địa chỉ IPv4 thuộc đúng dải riêng tư RFC 1918 (`10/8`, `172.16/12`, `192.168/16`).
pub fn is_rfc1918_ipv4(raw: &str) -> bool {
    let parts: Vec<&str> = raw.split('.').collect();
    if parts.len() != 4 {
        return false;
    }
    let mut octets = [0u8; 4];
    for (idx, part) in parts.iter().enumerate() {
        if part.is_empty()
            || (part.len() > 1 && part.starts_with('0'))
            || !part.bytes().all(|b| b.is_ascii_digit())
        {
            return false;
        }
        let Ok(val) = part.parse::<u8>() else {
            return false;
        };
        octets[idx] = val;
    }
    matches!(
        octets,
        [10, _, _, _] | [172, 16..=31, _, _] | [192, 168, _, _]
    )
}

/// Kiểm tra timestamp ISO 8601 UTC nghiêm ngặt dạng `YYYY-MM-DDTHH:MM:SSZ`.
pub fn is_valid_iso8601_utc(raw: &str) -> bool {
    let b = raw.as_bytes();
    if b.len() != 20 {
        return false;
    }
    if b[4] != b'-'
        || b[7] != b'-'
        || b[10] != b'T'
        || b[13] != b':'
        || b[16] != b':'
        || b[19] != b'Z'
    {
        return false;
    }
    for idx in [0, 1, 2, 3, 5, 6, 8, 9, 11, 12, 14, 15, 17, 18] {
        if !b[idx].is_ascii_digit() {
            return false;
        }
    }
    let year: u16 = match raw[0..4].parse() {
        Ok(y) => y,
        Err(_) => return false,
    };
    let month: u8 = match raw[5..7].parse() {
        Ok(m) => m,
        Err(_) => return false,
    };
    let day: u8 = match raw[8..10].parse() {
        Ok(d) => d,
        Err(_) => return false,
    };
    let hour: u8 = match raw[11..13].parse() {
        Ok(h) => h,
        Err(_) => return false,
    };
    let min: u8 = match raw[14..16].parse() {
        Ok(m) => m,
        Err(_) => return false,
    };
    let sec: u8 = match raw[17..19].parse() {
        Ok(s) => s,
        Err(_) => return false,
    };
    let is_leap = (year.is_multiple_of(4) && !year.is_multiple_of(100)) || year.is_multiple_of(400);
    let max_day = match month {
        1 | 3 | 5 | 7 | 8 | 10 | 12 => 31,
        4 | 6 | 9 | 11 => 30,
        2 => {
            if is_leap {
                29
            } else {
                28
            }
        }
        _ => return false,
    };
    (1..=max_day).contains(&day) && hour <= 23 && min <= 59 && sec <= 59
}

/// Chuẩn hóa URI vị trí mô phỏng `virtual://<host.example>/<subsystem>/<entry>`,
/// chặn tuyệt đối đường dẫn hệ điều hành thật (`C:\`, `/etc/`, `..`, shell metacharacters).
pub fn normalize_virtual_location(raw: &str) -> Result<String, PostExploitError> {
    if raw.contains('\\')
        || raw.contains("..")
        || raw.bytes().any(|b| b.is_ascii_control() || b == b' ')
    {
        return Err(PostExploitError::InvalidLocation(format!(
            "đường dẫn '{raw}' chứa ký tự hệ thống hoặc khoảng trắng bị cấm"
        )));
    }
    if raw
        .chars()
        .any(|c| matches!(c, ';' | '&' | '|' | '$' | '`' | '(' | ')' | '<' | '>'))
    {
        return Err(PostExploitError::InvalidLocation(format!(
            "đường dẫn '{raw}' chứa ký tự shell bị cấm"
        )));
    }
    let rest = raw.strip_prefix("virtual://").ok_or_else(|| {
        PostExploitError::InvalidLocation(format!(
            "vị trí phải dùng scheme virtual:// (không chạm OS thật), nhận '{raw}'"
        ))
    })?;
    let (raw_host, path) = rest.split_once('/').ok_or_else(|| {
        PostExploitError::InvalidLocation(format!("vị trí '{raw}' thiếu subsystem/entry sau host"))
    })?;
    let host = normalize_example_domain(raw_host)?;
    let segments: Vec<&str> = path.split('/').collect();
    if segments.len() < 2
        || segments.iter().any(|seg| {
            seg.is_empty()
                || *seg == "."
                || *seg == ".."
                || !seg
                    .bytes()
                    .all(|b| b.is_ascii_alphanumeric() || matches!(b, b'-' | b'_' | b'.'))
        })
    {
        return Err(PostExploitError::InvalidLocation(format!(
            "các phân đoạn trong '{raw}' không hợp lệ"
        )));
    }
    Ok(format!(
        "virtual://{}/{}",
        host,
        segments
            .iter()
            .map(|s| s.to_ascii_lowercase())
            .collect::<Vec<_>>()
            .join("/")
    ))
}

/// Chặn đưa secret thô hoặc hash thật vào snapshot, mô tả hay báo cáo.
pub fn validate_no_raw_secret(
    text: &str,
    field_name: Option<&str>,
) -> Result<(), PostExploitError> {
    let lower = text.to_ascii_lowercase();
    let blocked_markers = [
        "bearer ",
        "-----begin ",
        "ghp_",
        "github_pat_",
        "ak_live_",
        "sk-live-",
        "password=",
        "passwd=",
        "ntlm:",
    ];
    for marker in blocked_markers {
        if lower.contains(marker) {
            return Err(PostExploitError::RawSecretProhibited(format!(
                "phát hiện mẫu bí mật thô '{marker}'"
            )));
        }
    }
    if let Some(attr) = field_name {
        let attr_lower = attr.to_ascii_lowercase();
        let sensitive_keywords = [
            "secret",
            "password",
            "credential",
            "token",
            "private_key",
            "api_key",
            "nt_hash",
        ];
        if sensitive_keywords.iter().any(|k| attr_lower.contains(k))
            && text != REDACTED_CREDENTIAL_MARKER
            && !text.contains(REDACTED_CREDENTIAL_MARKER)
        {
            return Err(PostExploitError::RawSecretProhibited(format!(
                "trường nhạy cảm '{attr}' phải dùng marker '{REDACTED_CREDENTIAL_MARKER}'"
            )));
        }
    }
    Ok(())
}

/// Danh mục hành vi thu thập thông tin xác thực bị cấm hoàn toàn trong mô hình `ch110`.
#[derive(Debug, Clone, Copy, PartialEq, Eq, PartialOrd, Ord)]
pub enum HarvestingAction {
    LsassDump,
    SamDump,
    ShadowRead,
    TokenTheft,
    RegistrySecretExtract,
}

// ============================================================================
// 2. PHẠM VI, SNAPSHOT HỆ THỐNG, PERSISTENCE & ARTIFACTS
// ============================================================================

#[derive(Debug, Clone, Copy, PartialEq, Eq, PartialOrd, Ord)]
pub enum HostPlatform {
    Windows,
    Linux,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, PartialOrd, Ord)]
pub enum PrivilegeLevel {
    StandardUser,
    ServiceAccount,
    LocalAdmin,
    SystemOrRoot,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, PartialOrd, Ord)]
pub enum PersistenceMechanism {
    Service,
    ScheduledTask,
    StartupEntry,
    Autorun,
}

impl PersistenceMechanism {
    pub fn as_str(self) -> &'static str {
        match self {
            Self::Service => "Service",
            Self::ScheduledTask => "ScheduledTask",
            Self::StartupEntry => "StartupEntry",
            Self::Autorun => "Autorun",
        }
    }
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, PartialOrd, Ord)]
pub enum TelemetryChannel {
    ServiceControlManager,
    TaskScheduler,
    ProcessAudit,
    AuthAudit,
    FirewallFlow,
    SystemdJournal,
    CronAudit,
    PolicyChangeAudit,
}

impl TelemetryChannel {
    pub fn as_str(self) -> &'static str {
        match self {
            Self::ServiceControlManager => "ServiceControlManager",
            Self::TaskScheduler => "TaskScheduler",
            Self::ProcessAudit => "ProcessAudit",
            Self::AuthAudit => "AuthAudit",
            Self::FirewallFlow => "FirewallFlow",
            Self::SystemdJournal => "SystemdJournal",
            Self::CronAudit => "CronAudit",
            Self::PolicyChangeAudit => "PolicyChangeAudit",
        }
    }
}

#[derive(Debug, Clone, PartialEq, Eq, PartialOrd, Ord)]
pub struct EvidenceRecord {
    pub id: String,
    pub source: String,
    pub collected_at: String,
    pub summary: String,
    pub digest: String,
}

#[derive(Debug, Clone, PartialEq, Eq, PartialOrd, Ord)]
pub struct PersistenceRecord {
    pub id: String,
    pub host: String,
    pub mechanism: PersistenceMechanism,
    pub virtual_location: String,
    pub owner: String,
    pub privilege: PrivilegeLevel,
    pub target_descriptor: String,
    pub autostart: bool,
    pub signature_trusted: bool,
    pub artifact_id: Option<String>,
    pub evidence_id: String,
}

impl PersistenceRecord {
    #[allow(clippy::too_many_arguments)]
    pub fn new(
        id: &str,
        host: &str,
        mechanism: PersistenceMechanism,
        virtual_location: &str,
        owner: &str,
        privilege: PrivilegeLevel,
        target_descriptor: &str,
        autostart: bool,
        signature_trusted: bool,
        artifact_id: Option<&str>,
        evidence_id: &str,
    ) -> Self {
        Self {
            id: id.to_string(),
            host: host.to_string(),
            mechanism,
            virtual_location: virtual_location.to_string(),
            owner: owner.to_string(),
            privilege,
            target_descriptor: target_descriptor.to_string(),
            autostart,
            signature_trusted,
            artifact_id: artifact_id.map(str::to_string),
            evidence_id: evidence_id.to_string(),
        }
    }
}

#[derive(Debug, Clone, PartialEq, Eq, PartialOrd, Ord)]
pub struct AccountState {
    pub account_id: String,
    pub enabled: bool,
    pub privilege: PrivilegeLevel,
    pub groups: BTreeSet<String>,
    pub evidence_id: String,
}

#[derive(Debug, Clone, PartialEq, Eq, PartialOrd, Ord)]
pub struct SecurityPolicyRecord {
    pub policy_id: String,
    pub setting: String,
    pub secure_value: String,
    pub current_value: String,
    pub disables_channel: Option<TelemetryChannel>,
    pub evidence_id: String,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, PartialOrd, Ord)]
pub enum ArtifactKind {
    DropperBinary,
    ScriptPayload,
    StagingArchive,
    ModifiedConfig,
    TempMarker,
}

#[derive(Debug, Clone, PartialEq, Eq, PartialOrd, Ord)]
pub struct Artifact {
    pub id: String,
    pub host: String,
    pub virtual_location: String,
    pub kind: ArtifactKind,
    pub sha256_digest: String,
    pub size_bytes: u64,
    pub owner: String,
    pub created_in_lab: bool,
    pub evidence_id: String,
}

impl Artifact {
    #[allow(clippy::too_many_arguments)]
    pub fn new(
        id: &str,
        host: &str,
        virtual_location: &str,
        kind: ArtifactKind,
        synthetic_body: &str,
        owner: &str,
        created_in_lab: bool,
        evidence_id: &str,
    ) -> Self {
        Self {
            id: id.to_string(),
            host: host.to_string(),
            virtual_location: virtual_location.to_string(),
            kind,
            sha256_digest: sha256_hex(synthetic_body.as_bytes()),
            size_bytes: synthetic_body.len() as u64,
            owner: owner.to_string(),
            created_in_lab,
            evidence_id: evidence_id.to_string(),
        }
    }
}

#[derive(Debug, Clone, PartialEq, Eq, PartialOrd, Ord)]
pub struct TelemetryControl {
    pub channel: TelemetryChannel,
    pub enabled: bool,
    pub retention_days: u16,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct SystemSnapshot {
    pub snapshot_id: String,
    pub host: String,
    pub platform: HostPlatform,
    pub captured_at: String,
    pub persistence_records: BTreeMap<String, PersistenceRecord>,
    pub accounts: BTreeMap<String, AccountState>,
    pub security_policies: BTreeMap<String, SecurityPolicyRecord>,
    pub artifacts: BTreeMap<String, Artifact>,
    pub telemetry_controls: BTreeMap<TelemetryChannel, TelemetryControl>,
}

impl SystemSnapshot {
    pub fn new(
        snapshot_id: &str,
        host: &str,
        platform: HostPlatform,
        captured_at: &str,
    ) -> Result<Self, PostExploitError> {
        if snapshot_id.trim().is_empty() {
            return Err(PostExploitError::ValidationFailed(
                "snapshot_id không được rỗng".to_string(),
            ));
        }
        let canon_host = normalize_example_domain(host)?;
        if !is_valid_iso8601_utc(captured_at) {
            return Err(PostExploitError::InvalidTimestamp(captured_at.to_string()));
        }
        Ok(Self {
            snapshot_id: snapshot_id.to_string(),
            host: canon_host,
            platform,
            captured_at: captured_at.to_string(),
            persistence_records: BTreeMap::new(),
            accounts: BTreeMap::new(),
            security_policies: BTreeMap::new(),
            artifacts: BTreeMap::new(),
            telemetry_controls: BTreeMap::new(),
        })
    }

    /// Kiểm chứng mọi bản ghi trong snapshot dùng đường dẫn ảo hợp lệ, không secret và có evidence.
    pub fn validate(
        &self,
        evidence_store: &BTreeMap<String, EvidenceRecord>,
    ) -> Result<(), PostExploitError> {
        let host = normalize_example_domain(&self.host)?;
        if host != self.host || !is_valid_iso8601_utc(&self.captured_at) {
            return Err(PostExploitError::ValidationFailed(format!(
                "snapshot '{}' có host hoặc timestamp không chuẩn",
                self.snapshot_id
            )));
        }
        for (key, rec) in &self.persistence_records {
            if key.trim().is_empty() || key != &rec.id || rec.host != self.host {
                return Err(PostExploitError::SnapshotMismatch(format!(
                    "persistence '{key}' lệch khóa hoặc host trên '{}'",
                    self.host
                )));
            }
            let norm_loc = normalize_virtual_location(&rec.virtual_location)?;
            if norm_loc != rec.virtual_location {
                return Err(PostExploitError::InvalidLocation(format!(
                    "vị trí '{}' chưa chuẩn hóa thành '{norm_loc}'",
                    rec.virtual_location
                )));
            }
            if let Some(art_id) = &rec.artifact_id
                && (art_id.trim().is_empty() || !self.artifacts.contains_key(art_id))
            {
                return Err(PostExploitError::SnapshotMismatch(format!(
                    "persistence '{}' trỏ tới artifact không tồn tại '{art_id}'",
                    rec.id
                )));
            }
            if rec.evidence_id.trim().is_empty() || !evidence_store.contains_key(&rec.evidence_id) {
                return Err(PostExploitError::MissingEvidence(format!(
                    "persistence '{}' thiếu evidence '{}'",
                    rec.id, rec.evidence_id
                )));
            }
            validate_no_raw_secret(&rec.virtual_location, None)?;
            validate_no_raw_secret(&rec.target_descriptor, None)?;
            validate_no_raw_secret(&rec.owner, None)?;
        }
        for (key, acct) in &self.accounts {
            if key != &acct.account_id || key.trim().is_empty() {
                return Err(PostExploitError::SnapshotMismatch(format!(
                    "account key '{key}' không khớp account_id"
                )));
            }
            if acct.evidence_id.trim().is_empty() || !evidence_store.contains_key(&acct.evidence_id)
            {
                return Err(PostExploitError::MissingEvidence(format!(
                    "account '{}' thiếu evidence '{}'",
                    acct.account_id, acct.evidence_id
                )));
            }
            validate_no_raw_secret(&acct.account_id, None)?;
            for grp in &acct.groups {
                if grp.trim().is_empty() {
                    return Err(PostExploitError::ValidationFailed(format!(
                        "account '{}' chứa tên nhóm rỗng",
                        acct.account_id
                    )));
                }
                validate_no_raw_secret(grp, None)?;
            }
        }
        for (key, pol) in &self.security_policies {
            if key.trim().is_empty() || key != &pol.policy_id || pol.setting.trim().is_empty() {
                return Err(PostExploitError::SnapshotMismatch(format!(
                    "policy key '{key}' không khớp policy_id hoặc setting rỗng"
                )));
            }
            if pol.evidence_id.trim().is_empty() || !evidence_store.contains_key(&pol.evidence_id) {
                return Err(PostExploitError::MissingEvidence(format!(
                    "security policy '{}' thiếu evidence '{}'",
                    pol.policy_id, pol.evidence_id
                )));
            }
            validate_no_raw_secret(&pol.setting, Some(&pol.setting))?;
            validate_no_raw_secret(&pol.secure_value, Some(&pol.setting))?;
            validate_no_raw_secret(&pol.current_value, Some(&pol.setting))?;
        }
        for (key, art) in &self.artifacts {
            if key.trim().is_empty()
                || key != &art.id
                || art.host != self.host
                || art.owner.trim().is_empty()
            {
                return Err(PostExploitError::SnapshotMismatch(format!(
                    "artifact '{key}' lệch khóa, owner rỗng hoặc host trên '{}'",
                    self.host
                )));
            }
            let norm_loc = normalize_virtual_location(&art.virtual_location)?;
            if norm_loc != art.virtual_location {
                return Err(PostExploitError::InvalidLocation(format!(
                    "artifact location '{}' chưa chuẩn hóa thành '{norm_loc}'",
                    art.virtual_location
                )));
            }
            if art.sha256_digest.len() != 64
                || !art.sha256_digest.bytes().all(|b| b.is_ascii_hexdigit())
            {
                return Err(PostExploitError::ValidationFailed(format!(
                    "artifact '{}' có sha256_digest không hợp lệ",
                    art.id
                )));
            }
            if art.evidence_id.trim().is_empty() || !evidence_store.contains_key(&art.evidence_id) {
                return Err(PostExploitError::MissingEvidence(format!(
                    "artifact '{}' thiếu evidence '{}'",
                    art.id, art.evidence_id
                )));
            }
            validate_no_raw_secret(&art.virtual_location, None)?;
            validate_no_raw_secret(&art.owner, None)?;
        }
        Ok(())
    }
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct AuditScope {
    pub engagement_id: String,
    pub allowed_hosts: BTreeSet<String>,
    pub authorized_backup_destinations: BTreeSet<String>,
    pub allow_credential_harvesting: bool,
    pub allow_real_os_mutation: bool,
}

impl AuditScope {
    pub fn standard() -> Self {
        let mut allowed_hosts = BTreeSet::new();
        allowed_hosts.insert("win-srv01.corp.example".to_string());
        allowed_hosts.insert("linux-app01.corp.example".to_string());
        allowed_hosts.insert("files-int.corp.example".to_string());
        allowed_hosts.insert("db-tier0.corp.example".to_string());
        allowed_hosts.insert("audit-vault.corp.example".to_string());

        let mut authorized_backup_destinations = BTreeSet::new();
        authorized_backup_destinations.insert("backup01.corp.example".to_string());

        Self {
            engagement_id: ENGAGEMENT_ID.to_string(),
            allowed_hosts,
            authorized_backup_destinations,
            allow_credential_harvesting: false,
            allow_real_os_mutation: false,
        }
    }

    pub fn validate(&self) -> Result<(), PostExploitError> {
        if self.engagement_id.trim().is_empty() {
            return Err(PostExploitError::OutOfScope(
                "engagement_id không được rỗng".to_string(),
            ));
        }
        if self.allow_credential_harvesting {
            return Err(PostExploitError::CredentialHarvestingProhibited(
                "mô hình nghiêm cấm thu thập hoặc dump credential thật".to_string(),
            ));
        }
        if self.allow_real_os_mutation {
            return Err(PostExploitError::InvalidLocation(
                "mô hình nghiêm cấm thay đổi hệ điều hành thật".to_string(),
            ));
        }
        if self.allowed_hosts.is_empty() {
            return Err(PostExploitError::OutOfScope(
                "allowed_hosts không được rỗng".to_string(),
            ));
        }
        for host in &self.allowed_hosts {
            let norm = normalize_example_domain(host)?;
            if &norm != host {
                return Err(PostExploitError::InvalidDomain(format!(
                    "allowed_hosts chứa '{host}' chưa chuẩn hóa thành '{norm}'"
                )));
            }
        }
        for dest in &self.authorized_backup_destinations {
            let norm = normalize_example_domain(dest)?;
            if &norm != dest {
                return Err(PostExploitError::InvalidDomain(format!(
                    "authorized_backup_destinations chứa '{dest}' chưa chuẩn hóa thành '{norm}'"
                )));
            }
        }
        Ok(())
    }

    pub fn require_host(&self, host: &str) -> Result<String, PostExploitError> {
        let canon = normalize_example_domain(host)?;
        if !self.allowed_hosts.contains(&canon) {
            return Err(PostExploitError::OutOfScope(format!(
                "host '{canon}' nằm ngoài phạm vi kiểm toán"
            )));
        }
        Ok(canon)
    }

    pub fn request_harvesting(&self, action: HarvestingAction) -> Result<(), PostExploitError> {
        Err(PostExploitError::CredentialHarvestingProhibited(format!(
            "từ chối tuyệt đối thao tác {action:?}: ngoài ranh giới audit hậu khai thác an toàn"
        )))
    }
}

// ============================================================================
// 3. SO SÁNH BASELINE (READ-ONLY DIFF), ACCESS, LATERAL GRAPH & TELEMETRY
// ============================================================================

pub type TelemetryControlDrift = (
    TelemetryChannel,
    Option<TelemetryControl>,
    Option<TelemetryControl>,
);

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct SnapshotDiff {
    pub host: String,
    pub platform: HostPlatform,
    pub added_persistence: Vec<PersistenceRecord>,
    pub modified_persistence: Vec<(PersistenceRecord, PersistenceRecord)>,
    pub removed_persistence: Vec<PersistenceRecord>,
    pub added_accounts: Vec<AccountState>,
    pub modified_accounts: Vec<(AccountState, AccountState)>,
    pub removed_accounts: Vec<AccountState>,
    pub added_policies: Vec<SecurityPolicyRecord>,
    pub policy_drifts: Vec<(SecurityPolicyRecord, SecurityPolicyRecord)>,
    pub removed_policies: Vec<SecurityPolicyRecord>,
    pub added_artifacts: Vec<Artifact>,
    pub modified_artifacts: Vec<(Artifact, Artifact)>,
    pub removed_artifacts: Vec<Artifact>,
    pub telemetry_drifts: Vec<TelemetryControlDrift>,
}

impl SnapshotDiff {
    pub fn is_clean(&self) -> bool {
        self.added_persistence.is_empty()
            && self.modified_persistence.is_empty()
            && self.removed_persistence.is_empty()
            && self.added_accounts.is_empty()
            && self.modified_accounts.is_empty()
            && self.removed_accounts.is_empty()
            && self.added_policies.is_empty()
            && self.policy_drifts.is_empty()
            && self.removed_policies.is_empty()
            && self.added_artifacts.is_empty()
            && self.modified_artifacts.is_empty()
            && self.removed_artifacts.is_empty()
            && self.telemetry_drifts.is_empty()
    }
}

fn diff_maps<T: Clone + PartialEq>(
    baseline: &BTreeMap<String, T>,
    current: &BTreeMap<String, T>,
) -> (Vec<T>, Vec<(T, T)>, Vec<T>) {
    let mut added = Vec::new();
    let mut modified = Vec::new();
    let mut removed = Vec::new();

    for (key, cur_val) in current {
        match baseline.get(key) {
            None => added.push(cur_val.clone()),
            Some(base_val) if base_val != cur_val => {
                modified.push((base_val.clone(), cur_val.clone()))
            }
            Some(_) => {}
        }
    }
    for (key, base_val) in baseline {
        if !current.contains_key(key) {
            removed.push(base_val.clone());
        }
    }
    (added, modified, removed)
}

/// So sánh hai snapshot bất biến (read-only); bất kỳ sai lệch không có bằng chứng đều báo lỗi.
pub fn diff_snapshots(
    baseline: &SystemSnapshot,
    current: &SystemSnapshot,
    evidence_store: &BTreeMap<String, EvidenceRecord>,
) -> Result<SnapshotDiff, PostExploitError> {
    if baseline.host != current.host || baseline.platform != current.platform {
        return Err(PostExploitError::SnapshotMismatch(format!(
            "không thể diff snapshot giữa '{}/{:?}' và '{}/{:?}'",
            baseline.host, baseline.platform, current.host, current.platform
        )));
    }
    if current.captured_at < baseline.captured_at {
        return Err(PostExploitError::InvalidTimestamp(format!(
            "current.captured_at '{}' sớm hơn baseline '{}'",
            current.captured_at, baseline.captured_at
        )));
    }
    baseline.validate(evidence_store)?;
    current.validate(evidence_store)?;

    for art in baseline.artifacts.values() {
        if art.created_in_lab {
            return Err(PostExploitError::BaselineProtected(format!(
                "artifact baseline '{}' trên '{}' không được gắn created_in_lab=true",
                art.id, baseline.host
            )));
        }
    }
    for (id, art) in &current.artifacts {
        if !baseline.artifacts.contains_key(id) && !art.created_in_lab {
            return Err(PostExploitError::BaselineProtected(format!(
                "artifact phát sinh sau baseline '{id}' trên '{}' phải mang created_in_lab=true",
                current.host
            )));
        }
    }

    let (added_persistence, modified_persistence, removed_persistence) =
        diff_maps(&baseline.persistence_records, &current.persistence_records);
    let (added_accounts, modified_accounts, removed_accounts) =
        diff_maps(&baseline.accounts, &current.accounts);
    let (added_policies, policy_drifts, removed_policies) =
        diff_maps(&baseline.security_policies, &current.security_policies);
    let (added_artifacts, modified_artifacts, removed_artifacts) =
        diff_maps(&baseline.artifacts, &current.artifacts);

    let channels: BTreeSet<TelemetryChannel> = baseline
        .telemetry_controls
        .keys()
        .chain(current.telemetry_controls.keys())
        .copied()
        .collect();
    let mut telemetry_drifts = Vec::new();
    for chan in channels {
        let base_ctrl = baseline.telemetry_controls.get(&chan).cloned();
        let cur_ctrl = current.telemetry_controls.get(&chan).cloned();
        if base_ctrl != cur_ctrl {
            telemetry_drifts.push((chan, base_ctrl, cur_ctrl));
        }
    }

    Ok(SnapshotDiff {
        host: current.host.clone(),
        platform: current.platform,
        added_persistence,
        modified_persistence,
        removed_persistence,
        added_accounts,
        modified_accounts,
        removed_accounts,
        added_policies,
        policy_drifts,
        removed_policies,
        added_artifacts,
        modified_artifacts,
        removed_artifacts,
        telemetry_drifts,
    })
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, PartialOrd, Ord)]
pub enum AccessKind {
    InteractiveSession,
    RemoteServiceAdmin,
    SshRoleKey,
    FileShareRead,
    WinRmManagement,
}

#[derive(Debug, Clone, PartialEq, Eq, PartialOrd, Ord)]
pub struct AccessGrant {
    pub grant_id: String,
    pub source_host: String,
    pub target_host: String,
    pub principal: String,
    pub access_kind: AccessKind,
    pub privilege: PrivilegeLevel,
    pub expires_at: String,
    pub created_in_lab: bool,
    pub evidence_id: String,
}

impl AccessGrant {
    #[allow(clippy::too_many_arguments)]
    pub fn new(
        grant_id: &str,
        source_host: &str,
        target_host: &str,
        principal: &str,
        access_kind: AccessKind,
        privilege: PrivilegeLevel,
        expires_at: &str,
        created_in_lab: bool,
        evidence_id: &str,
    ) -> Self {
        Self {
            grant_id: grant_id.to_string(),
            source_host: source_host.to_string(),
            target_host: target_host.to_string(),
            principal: principal.to_string(),
            access_kind,
            privilege,
            expires_at: expires_at.to_string(),
            created_in_lab,
            evidence_id: evidence_id.to_string(),
        }
    }

    pub fn is_active_at(&self, now_iso: &str) -> bool {
        self.expires_at.as_str() > now_iso
    }
}

#[derive(Debug, Clone, PartialEq, Eq, PartialOrd, Ord)]
pub struct LateralEdge {
    pub edge_id: String,
    pub from_host: String,
    pub to_host: String,
    pub required_access: AccessKind,
    pub min_privilege: PrivilegeLevel,
    pub network_reachable: bool,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, PartialOrd, Ord)]
pub enum LateralBlockReason {
    OutOfScope,
    NetworkSegmented,
    MissingAccessGrant,
    InsufficientPrivilege,
    SessionExpired,
    CycleDetected,
    MaxDepthReached,
}

#[derive(Debug, Clone, PartialEq, Eq, PartialOrd, Ord)]
pub struct LateralPath {
    pub hops: Vec<String>,
    pub edge_ids: Vec<String>,
    pub min_effective_privilege: PrivilegeLevel,
    pub evidence_ids: BTreeSet<String>,
}

#[derive(Debug, Clone, PartialEq, Eq, PartialOrd, Ord)]
pub struct BlockedTransition {
    pub edge_id: String,
    pub from_host: String,
    pub to_host: String,
    pub reason: LateralBlockReason,
    pub detail: String,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct LateralGraphReport {
    pub start_host: String,
    pub max_depth: usize,
    pub reachable_paths: Vec<LateralPath>,
    pub blocked_transitions: Vec<BlockedTransition>,
}

#[derive(Debug, Clone, PartialEq, Eq, PartialOrd, Ord)]
pub struct TelemetryEvent {
    pub event_id: String,
    pub timestamp: String,
    pub host: String,
    pub channel: TelemetryChannel,
    pub event_code: String,
    pub summary: String,
    pub subject_id: String,
    pub evidence_id: String,
}

// ============================================================================
// 4. PHÂN LOẠI EXFIL-RISK DATAFLOW (CHỈ METADATA)
// ============================================================================

#[derive(Debug, Clone, Copy, PartialEq, Eq, PartialOrd, Ord)]
pub enum DestinationCategory {
    InternalCollector,
    AuthorizedBackup,
    UnapprovedExternal,
    UnknownExternal,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, PartialOrd, Ord)]
pub enum FlowProtocol {
    HttpsMeta,
    DnsTxtMeta,
    SmbInternalMeta,
    SftpMeta,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, PartialOrd, Ord)]
pub enum DataSensitivity {
    Public,
    Internal,
    Confidential,
    RestrictedRegulated,
}

#[derive(Debug, Clone, PartialEq, Eq, PartialOrd, Ord)]
pub struct DataFlow {
    pub flow_id: String,
    pub source_host: String,
    pub destination: String,
    pub destination_category: DestinationCategory,
    pub protocol: FlowProtocol,
    pub bytes_sent: u64,
    pub duration_secs: u32,
    pub sensitivity: DataSensitivity,
    pub off_hours: bool,
    pub evidence_id: String,
}

impl DataFlow {
    #[allow(clippy::too_many_arguments)]
    pub fn new(
        flow_id: &str,
        source_host: &str,
        destination: &str,
        destination_category: DestinationCategory,
        protocol: FlowProtocol,
        bytes_sent: u64,
        duration_secs: u32,
        sensitivity: DataSensitivity,
        off_hours: bool,
        evidence_id: &str,
    ) -> Self {
        Self {
            flow_id: flow_id.to_string(),
            source_host: source_host.to_string(),
            destination: destination.to_string(),
            destination_category,
            protocol,
            bytes_sent,
            duration_secs,
            sensitivity,
            off_hours,
            evidence_id: evidence_id.to_string(),
        }
    }
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, PartialOrd, Ord)]
pub enum Severity {
    Low,
    Medium,
    High,
    Critical,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, PartialOrd, Ord)]
pub enum ImpactLevel {
    None,
    Low,
    Medium,
    High,
    Critical,
}

pub const fn max_severity_for_impact(impact: ImpactLevel) -> Severity {
    match impact {
        ImpactLevel::None | ImpactLevel::Low => Severity::Low,
        ImpactLevel::Medium => Severity::Medium,
        ImpactLevel::High => Severity::High,
        ImpactLevel::Critical => Severity::Critical,
    }
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, PartialOrd, Ord)]
pub enum ExfilRiskLevel {
    Benign,
    Low,
    Medium,
    High,
    Critical,
}

impl ExfilRiskLevel {
    pub fn as_severity(self) -> Option<Severity> {
        match self {
            Self::Benign => None,
            Self::Low => Some(Severity::Low),
            Self::Medium => Some(Severity::Medium),
            Self::High => Some(Severity::High),
            Self::Critical => Some(Severity::Critical),
        }
    }
}

#[derive(Debug, Clone, PartialEq, Eq, PartialOrd, Ord)]
pub struct ExfilAssessment {
    pub flow_id: String,
    pub source_host: String,
    pub destination: String,
    pub risk: ExfilRiskLevel,
    pub impact: ImpactLevel,
    pub reasons: Vec<String>,
    pub evidence_id: String,
}

/// Phân loại rủi ro rò rỉ dữ liệu dựa trên metadata lưu lượng (không chứa payload thật).
pub fn classify_data_flow(
    scope: &AuditScope,
    flow: &DataFlow,
    evidence_store: &BTreeMap<String, EvidenceRecord>,
) -> Result<ExfilAssessment, PostExploitError> {
    if flow.flow_id.trim().is_empty() {
        return Err(PostExploitError::ValidationFailed(
            "flow_id không được rỗng".to_string(),
        ));
    }
    validate_no_raw_secret(&flow.flow_id, None)?;
    let source_host = scope.require_host(&flow.source_host)?;
    let canon_dest = if is_rfc1918_ipv4(&flow.destination) {
        flow.destination.clone()
    } else {
        normalize_example_domain(&flow.destination)?
    };
    if flow.evidence_id.trim().is_empty() || !evidence_store.contains_key(&flow.evidence_id) {
        return Err(PostExploitError::MissingEvidence(format!(
            "data flow '{}' thiếu evidence '{}'",
            flow.flow_id, flow.evidence_id
        )));
    }
    if flow.duration_secs == 0 {
        return Err(PostExploitError::ValidationFailed(format!(
            "data flow '{}' có duration_secs == 0",
            flow.flow_id
        )));
    }

    let mut reasons = Vec::new();
    let classify_external = |reasons: &mut Vec<String>, cat_label: &str| {
        reasons.push(cat_label.to_string());
        if flow.protocol == FlowProtocol::DnsTxtMeta {
            reasons.push("giao thức DnsTxtMeta nghi tunneling".to_string());
        }
        if flow.off_hours {
            reasons.push("phát sinh ngoài giờ hành chính".to_string());
        }
        if flow.bytes_sent >= 1_000_000 {
            reasons.push(format!("khối lượng {} bytes vượt ngưỡng", flow.bytes_sent));
        }
        match flow.sensitivity {
            DataSensitivity::Public => {
                reasons.push("dữ liệu Public nên mức độ tác động giữ ở Low".to_string());
                (ExfilRiskLevel::Low, ImpactLevel::Low)
            }
            DataSensitivity::Internal => {
                if flow.protocol == FlowProtocol::DnsTxtMeta || flow.bytes_sent >= 1_000_000 {
                    (ExfilRiskLevel::High, ImpactLevel::High)
                } else {
                    (ExfilRiskLevel::Medium, ImpactLevel::Medium)
                }
            }
            DataSensitivity::Confidential => {
                if flow.bytes_sent >= 10_000_000 || (flow.off_hours && flow.bytes_sent >= 1_000_000)
                {
                    (ExfilRiskLevel::Critical, ImpactLevel::Critical)
                } else {
                    (ExfilRiskLevel::High, ImpactLevel::High)
                }
            }
            DataSensitivity::RestrictedRegulated => {
                (ExfilRiskLevel::Critical, ImpactLevel::Critical)
            }
        }
    };

    let (mut risk, mut impact) = match flow.destination_category {
        DestinationCategory::AuthorizedBackup => {
            if !scope.authorized_backup_destinations.contains(&canon_dest) {
                reasons.push("đích backup không nằm trong danh sách được ủy quyền".to_string());
                (ExfilRiskLevel::High, ImpactLevel::High)
            } else if flow.protocol == FlowProtocol::DnsTxtMeta {
                reasons.push("kênh DNS TXT bất thường trên đích backup".to_string());
                if flow.sensitivity >= DataSensitivity::Confidential {
                    reasons.push("mang siêu dữ liệu nhạy cảm".to_string());
                    (ExfilRiskLevel::High, ImpactLevel::High)
                } else {
                    (ExfilRiskLevel::Medium, ImpactLevel::Medium)
                }
            } else if flow.bytes_sent > 50_000_000 {
                reasons.push("dung lượng vượt ngưỡng baseline backup thông thường".to_string());
                (ExfilRiskLevel::Medium, ImpactLevel::Medium)
            } else {
                reasons.push("luồng backup nằm trong baseline được phép".to_string());
                (ExfilRiskLevel::Benign, ImpactLevel::None)
            }
        }
        DestinationCategory::InternalCollector => {
            let is_internal_dest =
                scope.allowed_hosts.contains(&canon_dest) || is_rfc1918_ipv4(&canon_dest);
            if !is_internal_dest {
                classify_external(
                    &mut reasons,
                    "đích gắn nhãn InternalCollector nhưng nằm ngoài hệ thống nội bộ",
                )
            } else if flow.protocol == FlowProtocol::DnsTxtMeta {
                reasons.push("kênh DNS TXT bất thường ngay cả trong nội bộ".to_string());
                if flow.sensitivity >= DataSensitivity::Confidential {
                    reasons.push("mang siêu dữ liệu nhạy cảm".to_string());
                    (ExfilRiskLevel::High, ImpactLevel::High)
                } else {
                    (ExfilRiskLevel::Medium, ImpactLevel::Medium)
                }
            } else if flow.bytes_sent > 50_000_000 {
                reasons.push("dung lượng vượt ngưỡng baseline nội bộ".to_string());
                (ExfilRiskLevel::Medium, ImpactLevel::Medium)
            } else {
                reasons.push("luồng nội bộ nằm trong baseline được phép".to_string());
                (ExfilRiskLevel::Benign, ImpactLevel::None)
            }
        }
        DestinationCategory::UnapprovedExternal | DestinationCategory::UnknownExternal => {
            classify_external(
                &mut reasons,
                &format!("đích {:?}", flow.destination_category),
            )
        }
    };

    if flow.sensitivity == DataSensitivity::Public && risk > ExfilRiskLevel::Low {
        reasons.push("dữ liệu Public khóa trần ở Low/Low".to_string());
        risk = ExfilRiskLevel::Low;
        impact = ImpactLevel::Low;
    }

    Ok(ExfilAssessment {
        flow_id: flow.flow_id.clone(),
        source_host,
        destination: canon_dest,
        risk,
        impact,
        reasons,
        evidence_id: flow.evidence_id.clone(),
    })
}

// ============================================================================
// 5. AUDIT FINDING, CLEANUP PLAN LŨY ĐẲNG & MOCK POST EXPLOIT ENGINE
// ============================================================================

#[derive(Debug, Clone, Copy, PartialEq, Eq, PartialOrd, Ord)]
pub enum FindingCategory {
    Persistence,
    AccountDrift,
    SecurityPolicyTamper,
    LateralExposure,
    TelemetryGap,
    ExfilRisk,
}

#[derive(Debug, Clone, PartialEq, Eq, PartialOrd, Ord)]
pub struct AuditFinding {
    pub finding_id: String,
    pub host: String,
    pub category: FindingCategory,
    pub severity: Severity,
    pub impact: ImpactLevel,
    pub title: String,
    pub preconditions: Vec<String>,
    pub evidence_ids: BTreeSet<String>,
    pub telemetry_ids: BTreeSet<String>,
    pub detection_gap: Option<String>,
    pub detection_rule: String,
    pub response_action: String,
    pub remediation: String,
    pub cleanup_refs: Vec<String>,
}

impl AuditFinding {
    pub fn validate_invariants(
        &self,
        evidence_store: &BTreeMap<String, EvidenceRecord>,
        telemetry_events: &BTreeMap<String, TelemetryEvent>,
    ) -> Result<(), PostExploitError> {
        if self.finding_id.trim().is_empty() || self.title.trim().is_empty() {
            return Err(PostExploitError::ValidationFailed(
                "finding_id hoặc title không được rỗng".to_string(),
            ));
        }
        if self.severity > max_severity_for_impact(self.impact) {
            return Err(PostExploitError::SeverityExceedsImpact {
                severity: self.severity,
                impact: self.impact,
            });
        }
        if self.evidence_ids.is_empty() {
            return Err(PostExploitError::MissingEvidence(format!(
                "finding '{}' không có bằng chứng nào",
                self.finding_id
            )));
        }
        for ev_id in &self.evidence_ids {
            if !evidence_store.contains_key(ev_id) {
                return Err(PostExploitError::MissingEvidence(format!(
                    "finding '{}' trỏ tới evidence giả '{ev_id}'",
                    self.finding_id
                )));
            }
        }
        for tel_id in &self.telemetry_ids {
            let ev = telemetry_events.get(tel_id).ok_or_else(|| {
                PostExploitError::ValidationFailed(format!(
                    "finding '{}' trỏ tới telemetry không tồn tại '{tel_id}'",
                    self.finding_id
                ))
            })?;
            if ev.host != self.host {
                return Err(PostExploitError::ValidationFailed(format!(
                    "telemetry '{tel_id}' trên '{}' không khớp host '{}' của finding '{}'",
                    ev.host, self.host, self.finding_id
                )));
            }
            let subject = ev.subject_id.trim();
            let relevant = !subject.is_empty()
                && (self.finding_id.ends_with(&format!(":{subject}"))
                    || (self.category == FindingCategory::LateralExposure
                        && self
                            .finding_id
                            .strip_prefix("find:lateral:")
                            .is_some_and(|rest| rest.split('+').any(|edge| edge == subject)))
                    || (self.category == FindingCategory::TelemetryGap
                        && self.cleanup_refs.iter().any(|r| {
                            r == &format!("policy:{subject}")
                                || r == &format!("telemetry:{subject}")
                        })));
            if !relevant {
                return Err(PostExploitError::ValidationFailed(format!(
                    "telemetry '{tel_id}' (subject '{}') không liên quan tới finding '{}'",
                    ev.subject_id, self.finding_id
                )));
            }
        }
        if self.telemetry_ids.is_empty() && self.detection_gap.is_none() {
            return Err(PostExploitError::ValidationFailed(format!(
                "finding '{}' thiếu cả telemetry_ids lẫn detection_gap",
                self.finding_id
            )));
        }
        if let Some(gap) = &self.detection_gap {
            if gap.trim().is_empty() {
                return Err(PostExploitError::ValidationFailed(format!(
                    "finding '{}' chứa detection_gap rỗng",
                    self.finding_id
                )));
            }
            validate_no_raw_secret(gap, None)?;
        }
        if self.preconditions.is_empty()
            || self.preconditions.iter().any(|p| p.trim().is_empty())
            || self.detection_rule.trim().is_empty()
            || self.response_action.trim().is_empty()
            || self.remediation.trim().is_empty()
        {
            return Err(PostExploitError::ValidationFailed(format!(
                "finding '{}' thiếu precondition, detection, response hoặc remediation",
                self.finding_id
            )));
        }
        validate_no_raw_secret(&self.finding_id, None)?;
        validate_no_raw_secret(&self.host, None)?;
        validate_no_raw_secret(&self.title, None)?;
        for pre in &self.preconditions {
            validate_no_raw_secret(pre, None)?;
        }
        for cref in &self.cleanup_refs {
            validate_no_raw_secret(cref, None)?;
        }
        validate_no_raw_secret(&self.detection_rule, None)?;
        validate_no_raw_secret(&self.response_action, None)?;
        validate_no_raw_secret(&self.remediation, None)?;
        Ok(())
    }
}

/// Xếp hạng các finding persistence theo `Severity` giảm dần, rồi `ImpactLevel` giảm dần, rồi `finding_id`.
pub fn rank_persistence_findings(findings: &[AuditFinding]) -> Vec<AuditFinding> {
    let mut ranked: Vec<AuditFinding> = findings
        .iter()
        .filter(|f| f.category == FindingCategory::Persistence)
        .cloned()
        .collect();
    ranked.sort_by(|a, b| {
        b.severity
            .cmp(&a.severity)
            .then_with(|| b.impact.cmp(&a.impact))
            .then_with(|| a.finding_id.cmp(&b.finding_id))
    });
    ranked
}

#[derive(Debug, Clone, PartialEq, Eq, PartialOrd, Ord)]
pub enum CleanupStep {
    RemovePersistence {
        host: String,
        record_id: String,
    },
    RestorePersistence {
        host: String,
        record_id: String,
    },
    RemoveArtifact {
        host: String,
        artifact_id: String,
        expected_sha256: Option<String>,
    },
    RestoreArtifact {
        host: String,
        artifact_id: String,
    },
    RestoreAccount {
        host: String,
        account_id: String,
    },
    RestorePolicy {
        host: String,
        policy_id: String,
    },
    RestoreTelemetryControl {
        host: String,
        channel: TelemetryChannel,
    },
    RevokeAccessGrant {
        grant_id: String,
    },
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct CleanupPlan {
    pub plan_id: String,
    pub engagement_id: String,
    pub steps: Vec<CleanupStep>,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, PartialOrd, Ord)]
pub enum CleanupStepStatus {
    Removed,
    Restored,
    Revoked,
    AlreadyAbsent,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct BaselineVerificationReport {
    pub clean: bool,
    pub checked_hosts: Vec<String>,
    pub residual_drifts: Vec<String>,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct CleanupExecutionReport {
    pub plan_id: String,
    pub applied_mutations: usize,
    pub already_absent_steps: usize,
    pub step_outcomes: Vec<(CleanupStep, CleanupStepStatus)>,
    pub verification: BaselineVerificationReport,
}

#[derive(Debug, Clone, PartialEq, Eq)]
struct MockCleanupState {
    snapshots: BTreeMap<String, SystemSnapshot>,
    access_grants: BTreeMap<String, AccessGrant>,
}

impl MockCleanupState {
    fn from_current(
        snapshots: &BTreeMap<String, SystemSnapshot>,
        access_grants: &BTreeMap<String, AccessGrant>,
    ) -> Self {
        Self {
            snapshots: snapshots.clone(),
            access_grants: access_grants.clone(),
        }
    }
}

pub struct MockPostExploit {
    pub scope: AuditScope,
    pub audit_time: String,
    baseline_snapshots: BTreeMap<String, SystemSnapshot>,
    current_snapshots: BTreeMap<String, SystemSnapshot>,
    evidence_store: BTreeMap<String, EvidenceRecord>,
    access_grants: BTreeMap<String, AccessGrant>,
    lateral_edges: Vec<LateralEdge>,
    telemetry_events: BTreeMap<String, TelemetryEvent>,
    data_flows: Vec<DataFlow>,
    cleanup_state: MockCleanupState,
}

impl MockPostExploit {
    #[allow(clippy::too_many_arguments)]
    pub fn new(
        scope: AuditScope,
        audit_time: &str,
        baseline_snapshots: BTreeMap<String, SystemSnapshot>,
        current_snapshots: BTreeMap<String, SystemSnapshot>,
        evidence_store: BTreeMap<String, EvidenceRecord>,
        access_grants: BTreeMap<String, AccessGrant>,
        lateral_edges: Vec<LateralEdge>,
        telemetry_events: BTreeMap<String, TelemetryEvent>,
        data_flows: Vec<DataFlow>,
    ) -> Result<Self, PostExploitError> {
        scope.validate()?;
        if !is_valid_iso8601_utc(audit_time) {
            return Err(PostExploitError::InvalidTimestamp(audit_time.to_string()));
        }
        for (ev_id, ev) in &evidence_store {
            if &ev.id != ev_id
                || ev.summary.trim().is_empty()
                || !is_valid_iso8601_utc(&ev.collected_at)
            {
                return Err(PostExploitError::MissingEvidence(format!(
                    "bản ghi evidence '{ev_id}' không hợp lệ"
                )));
            }
            validate_no_raw_secret(&ev.summary, None)?;
        }
        for (host, snap) in &baseline_snapshots {
            scope.require_host(host)?;
            snap.validate(&evidence_store)?;
            if &snap.host != host {
                return Err(PostExploitError::SnapshotMismatch(format!(
                    "baseline key '{host}' khác snapshot.host '{}'",
                    snap.host
                )));
            }
            for art in snap.artifacts.values() {
                if art.created_in_lab {
                    return Err(PostExploitError::BaselineProtected(format!(
                        "artifact baseline '{}' trên '{host}' không được gắn created_in_lab=true",
                        art.id
                    )));
                }
            }
        }
        for (host, snap) in &current_snapshots {
            scope.require_host(host)?;
            snap.validate(&evidence_store)?;
            if &snap.host != host {
                return Err(PostExploitError::SnapshotMismatch(format!(
                    "current key '{host}' khác snapshot.host '{}'",
                    snap.host
                )));
            }
            let base = baseline_snapshots.get(host).ok_or_else(|| {
                PostExploitError::SnapshotMismatch(format!(
                    "current host '{host}' thiếu baseline đối chiếu"
                ))
            })?;
            for (art_id, art) in &snap.artifacts {
                if !base.artifacts.contains_key(art_id) && !art.created_in_lab {
                    return Err(PostExploitError::BaselineProtected(format!(
                        "artifact phát sinh sau baseline '{art_id}' trên '{host}' phải mang created_in_lab=true"
                    )));
                }
            }
        }
        for (gid, grant) in &access_grants {
            if gid != &grant.grant_id || grant.principal.trim().is_empty() {
                return Err(PostExploitError::DuplicateIdentifier(format!(
                    "grant key '{gid}' hoặc principal không hợp lệ"
                )));
            }
            if normalize_example_domain(&grant.source_host)? != grant.source_host {
                return Err(PostExploitError::InvalidDomain(format!(
                    "grant source_host '{}' chưa chuẩn hóa",
                    grant.source_host
                )));
            }
            if normalize_example_domain(&grant.target_host)? != grant.target_host {
                return Err(PostExploitError::InvalidDomain(format!(
                    "grant target_host '{}' chưa chuẩn hóa",
                    grant.target_host
                )));
            }
            validate_no_raw_secret(&grant.principal, None)?;
            if grant.evidence_id.trim().is_empty()
                || !evidence_store.contains_key(&grant.evidence_id)
            {
                return Err(PostExploitError::MissingEvidence(format!(
                    "access grant '{gid}' thiếu evidence '{}'",
                    grant.evidence_id
                )));
            }
            if !is_valid_iso8601_utc(&grant.expires_at) {
                return Err(PostExploitError::InvalidTimestamp(grant.expires_at.clone()));
            }
        }
        let mut seen_edges = BTreeSet::new();
        for edge in &lateral_edges {
            if edge.edge_id.trim().is_empty() || !seen_edges.insert(&edge.edge_id) {
                return Err(PostExploitError::DuplicateIdentifier(format!(
                    "lateral edge_id '{}' rỗng hoặc bị trùng",
                    edge.edge_id
                )));
            }
            if normalize_example_domain(&edge.from_host)? != edge.from_host {
                return Err(PostExploitError::InvalidDomain(format!(
                    "lateral from_host '{}' chưa chuẩn hóa",
                    edge.from_host
                )));
            }
            if normalize_example_domain(&edge.to_host)? != edge.to_host {
                return Err(PostExploitError::InvalidDomain(format!(
                    "lateral to_host '{}' chưa chuẩn hóa",
                    edge.to_host
                )));
            }
        }
        for (tid, tel) in &telemetry_events {
            if tid != &tel.event_id
                || !is_valid_iso8601_utc(&tel.timestamp)
                || tel.subject_id.trim().is_empty()
                || tel.event_code.trim().is_empty()
            {
                return Err(PostExploitError::ValidationFailed(format!(
                    "telemetry '{tid}' không hợp lệ"
                )));
            }
            if normalize_example_domain(&tel.host)? != tel.host {
                return Err(PostExploitError::InvalidDomain(format!(
                    "telemetry host '{}' chưa chuẩn hóa",
                    tel.host
                )));
            }
            if tel.evidence_id.trim().is_empty() || !evidence_store.contains_key(&tel.evidence_id) {
                return Err(PostExploitError::MissingEvidence(format!(
                    "telemetry '{tid}' thiếu evidence '{}'",
                    tel.evidence_id
                )));
            }
            validate_no_raw_secret(&tel.summary, None)?;
        }
        let mut seen_flows = BTreeSet::new();
        for flow in &data_flows {
            if flow.flow_id.trim().is_empty() || !seen_flows.insert(&flow.flow_id) {
                return Err(PostExploitError::DuplicateIdentifier(format!(
                    "data flow_id '{}' rỗng hoặc bị trùng",
                    flow.flow_id
                )));
            }
            let _ = classify_data_flow(&scope, flow, &evidence_store)?;
        }
        let cleanup_state = MockCleanupState::from_current(&current_snapshots, &access_grants);
        Ok(Self {
            scope,
            audit_time: audit_time.to_string(),
            baseline_snapshots,
            current_snapshots,
            evidence_store,
            access_grants,
            lateral_edges,
            telemetry_events,
            data_flows,
            cleanup_state,
        })
    }

    pub fn standard() -> Self {
        let scope = AuditScope::standard();
        let mut ev = BTreeMap::new();
        let mut add_ev = |id: &str, source: &str, summary: &str| {
            let digest = sha256_hex(format!("{id}|{source}|{summary}").as_bytes());
            ev.insert(
                id.to_string(),
                EvidenceRecord {
                    id: id.to_string(),
                    source: source.to_string(),
                    collected_at: AUDIT_TIMESTAMP.to_string(),
                    summary: summary.to_string(),
                    digest,
                },
            );
        };
        add_ev(
            "ev:base-win",
            "golden-image",
            "Baseline Windows host win-srv01",
        );
        add_ev(
            "ev:base-lin",
            "golden-image",
            "Baseline Linux host linux-app01",
        );
        add_ev(
            "ev:base-files",
            "golden-image",
            "Baseline internal file relay",
        );
        add_ev("ev:base-db", "golden-image", "Baseline tier0 db");
        add_ev("ev:base-vault", "golden-image", "Baseline audit vault");
        add_ev(
            "ev:win-task",
            "mock-collector",
            "Added scheduled task task:updater-sync",
        );
        add_ev(
            "ev:win-autorun",
            "mock-collector",
            "Added autorun run:agent-boot",
        );
        add_ev(
            "ev:win-pol",
            "mock-collector",
            "TaskScheduler audit policy changed to Disabled",
        );
        add_ev(
            "ev:win-acct",
            "mock-collector",
            "Account svc_maintenance enabled in RemoteManagementUsers",
        );
        add_ev(
            "ev:win-art1",
            "mock-collector",
            "Dropped helper artifact updater-helper.bin",
        );
        add_ev(
            "ev:win-art2",
            "mock-collector",
            "Dropped boot script boot-sync.ps1",
        );
        add_ev(
            "ev:lin-svc",
            "mock-collector",
            "Added systemd service svc:telemetry-forwarder",
        );
        add_ev(
            "ev:lin-cron",
            "mock-collector",
            "Added hourly cron entry startup:cron-profile",
        );
        add_ev(
            "ev:lin-art1",
            "mock-collector",
            "Dropped service script telemetry-fwd.sh",
        );
        add_ev(
            "ev:lin-art2",
            "mock-collector",
            "Dropped cron helper cron-sync.sh",
        );
        add_ev(
            "ev:lin-art3",
            "mock-collector",
            "Pre-existing temp marker artifact",
        );
        add_ev(
            "ev:grant-win-lin",
            "mock-inventory",
            "SshRoleKey grant win-srv01 -> linux-app01",
        );
        add_ev(
            "ev:grant-lin-files",
            "mock-inventory",
            "FileShareRead grant linux-app01 -> files-int",
        );
        add_ev(
            "ev:grant-win-db-exp",
            "mock-inventory",
            "Expired WinRmManagement grant win-srv01 -> db-tier0",
        );
        add_ev(
            "ev:grant-files-vault-low",
            "mock-inventory",
            "StandardUser FileShareRead grant files-int -> audit-vault",
        );
        add_ev(
            "ev:flow-backup",
            "firewall-meta",
            "Authorized SFTP backup flow",
        );
        add_ev(
            "ev:flow-dns",
            "firewall-meta",
            "Unapproved off-hours DnsTxtMeta flow",
        );
        add_ev(
            "ev:flow-exfil",
            "firewall-meta",
            "Critical RestrictedRegulated HTTPS metadata spike",
        );
        add_ev(
            "ev:flow-public",
            "firewall-meta",
            "Low public documentation fetch metadata",
        );
        add_ev(
            "ev:tel-link",
            "siem-meta",
            "Correlated SIEM telemetry record",
        );

        // --- Host 1: win-srv01.corp.example ---
        let mut base_win = SystemSnapshot::new(
            "snap-win-base",
            "win-srv01.corp.example",
            HostPlatform::Windows,
            BASELINE_TIMESTAMP,
        )
        .unwrap();
        base_win.persistence_records.insert(
            "svc:baseline-monitoring".to_string(),
            PersistenceRecord::new(
                "svc:baseline-monitoring",
                "win-srv01.corp.example",
                PersistenceMechanism::Service,
                "virtual://win-srv01.corp.example/services/baseline-monitoring",
                "nt-authority-system",
                PrivilegeLevel::SystemOrRoot,
                "synthetic://monitoring-agent --mode=passive",
                true,
                true,
                Some("art:win-baseline-agent"),
                "ev:base-win",
            ),
        );
        base_win.artifacts.insert(
            "art:win-baseline-agent".to_string(),
            Artifact::new(
                "art:win-baseline-agent",
                "win-srv01.corp.example",
                "virtual://win-srv01.corp.example/programdata/baseline-agent.bin",
                ArtifactKind::ModifiedConfig,
                "BASELINE-AGENT-CONFIG-V1",
                "system",
                false,
                "ev:base-win",
            ),
        );
        base_win.accounts.insert(
            "svc_maintenance".to_string(),
            AccountState {
                account_id: "svc_maintenance".to_string(),
                enabled: false,
                privilege: PrivilegeLevel::StandardUser,
                groups: BTreeSet::from(["DomainUsers".to_string()]),
                evidence_id: "ev:base-win".to_string(),
            },
        );
        base_win.security_policies.insert(
            "pol:task-audit".to_string(),
            SecurityPolicyRecord {
                policy_id: "pol:task-audit".to_string(),
                setting: "TaskSchedulerAuditing".to_string(),
                secure_value: "Enabled".to_string(),
                current_value: "Enabled".to_string(),
                disables_channel: Some(TelemetryChannel::TaskScheduler),
                evidence_id: "ev:base-win".to_string(),
            },
        );
        base_win.telemetry_controls.insert(
            TelemetryChannel::TaskScheduler,
            TelemetryControl {
                channel: TelemetryChannel::TaskScheduler,
                enabled: true,
                retention_days: 90,
            },
        );
        base_win.telemetry_controls.insert(
            TelemetryChannel::ServiceControlManager,
            TelemetryControl {
                channel: TelemetryChannel::ServiceControlManager,
                enabled: true,
                retention_days: 90,
            },
        );
        base_win.telemetry_controls.insert(
            TelemetryChannel::PolicyChangeAudit,
            TelemetryControl {
                channel: TelemetryChannel::PolicyChangeAudit,
                enabled: true,
                retention_days: 365,
            },
        );
        base_win.telemetry_controls.insert(
            TelemetryChannel::AuthAudit,
            TelemetryControl {
                channel: TelemetryChannel::AuthAudit,
                enabled: true,
                retention_days: 180,
            },
        );

        let mut cur_win = base_win.clone();
        cur_win.snapshot_id = "snap-win-cur".to_string();
        cur_win.captured_at = AUDIT_TIMESTAMP.to_string();
        cur_win.persistence_records.insert(
            "task:updater-sync".to_string(),
            PersistenceRecord::new(
                "task:updater-sync",
                "win-srv01.corp.example",
                PersistenceMechanism::ScheduledTask,
                "virtual://win-srv01.corp.example/tasks/updater-sync",
                "svc_deploy",
                PrivilegeLevel::LocalAdmin,
                "synthetic://updater-helper.bin --interval=60m",
                true,
                false,
                Some("art:win-helper-bin"),
                "ev:win-task",
            ),
        );
        cur_win.persistence_records.insert(
            "run:agent-boot".to_string(),
            PersistenceRecord::new(
                "run:agent-boot",
                "win-srv01.corp.example",
                PersistenceMechanism::Autorun,
                "virtual://win-srv01.corp.example/registry-mock/run/agent-boot",
                "corp-admin",
                PrivilegeLevel::LocalAdmin,
                "synthetic://boot-sync.ps1 --check",
                true,
                false,
                Some("art:win-boot-ps1"),
                "ev:win-autorun",
            ),
        );
        cur_win.artifacts.insert(
            "art:win-helper-bin".to_string(),
            Artifact::new(
                "art:win-helper-bin",
                "win-srv01.corp.example",
                "virtual://win-srv01.corp.example/staging/updater-helper.bin",
                ArtifactKind::DropperBinary,
                "SYNTHETIC-UPDATER-HELPER-V1",
                "svc_deploy",
                true,
                "ev:win-art1",
            ),
        );
        cur_win.artifacts.insert(
            "art:win-boot-ps1".to_string(),
            Artifact::new(
                "art:win-boot-ps1",
                "win-srv01.corp.example",
                "virtual://win-srv01.corp.example/staging/boot-sync.ps1",
                ArtifactKind::ScriptPayload,
                "SYNTHETIC-BOOT-SYNC-SCRIPT-V1",
                "corp-admin",
                true,
                "ev:win-art2",
            ),
        );
        if let Some(acct) = cur_win.accounts.get_mut("svc_maintenance") {
            acct.enabled = true;
            acct.privilege = PrivilegeLevel::LocalAdmin;
            acct.groups.insert("RemoteManagementUsers".to_string());
            acct.evidence_id = "ev:win-acct".to_string();
        }
        if let Some(pol) = cur_win.security_policies.get_mut("pol:task-audit") {
            pol.current_value = "Disabled".to_string();
            pol.evidence_id = "ev:win-pol".to_string();
        }
        if let Some(ctrl) = cur_win
            .telemetry_controls
            .get_mut(&TelemetryChannel::TaskScheduler)
        {
            ctrl.enabled = false;
        }

        // --- Host 2: linux-app01.corp.example ---
        let mut base_lin = SystemSnapshot::new(
            "snap-lin-base",
            "linux-app01.corp.example",
            HostPlatform::Linux,
            BASELINE_TIMESTAMP,
        )
        .unwrap();
        base_lin.telemetry_controls.insert(
            TelemetryChannel::SystemdJournal,
            TelemetryControl {
                channel: TelemetryChannel::SystemdJournal,
                enabled: true,
                retention_days: 90,
            },
        );
        base_lin.telemetry_controls.insert(
            TelemetryChannel::CronAudit,
            TelemetryControl {
                channel: TelemetryChannel::CronAudit,
                enabled: true,
                retention_days: 90,
            },
        );
        base_lin.telemetry_controls.insert(
            TelemetryChannel::FirewallFlow,
            TelemetryControl {
                channel: TelemetryChannel::FirewallFlow,
                enabled: true,
                retention_days: 90,
            },
        );

        let mut cur_lin = base_lin.clone();
        cur_lin.snapshot_id = "snap-lin-cur".to_string();
        cur_lin.captured_at = AUDIT_TIMESTAMP.to_string();
        cur_lin.persistence_records.insert(
            "svc:telemetry-forwarder".to_string(),
            PersistenceRecord::new(
                "svc:telemetry-forwarder",
                "linux-app01.corp.example",
                PersistenceMechanism::Service,
                "virtual://linux-app01.corp.example/systemd/telemetry-forwarder.service",
                "root",
                PrivilegeLevel::SystemOrRoot,
                "synthetic://telemetry-fwd.sh --daemon",
                true,
                false,
                Some("art:lin-svc-script"),
                "ev:lin-svc",
            ),
        );
        cur_lin.persistence_records.insert(
            "startup:cron-profile".to_string(),
            PersistenceRecord::new(
                "startup:cron-profile",
                "linux-app01.corp.example",
                PersistenceMechanism::StartupEntry,
                "virtual://linux-app01.corp.example/cron/hourly-sync",
                "appuser",
                PrivilegeLevel::StandardUser,
                "synthetic://cron-sync.sh",
                true,
                true,
                Some("art:lin-cron-sh"),
                "ev:lin-cron",
            ),
        );
        cur_lin.artifacts.insert(
            "art:lin-svc-script".to_string(),
            Artifact::new(
                "art:lin-svc-script",
                "linux-app01.corp.example",
                "virtual://linux-app01.corp.example/opt/lab/telemetry-fwd.sh",
                ArtifactKind::ScriptPayload,
                "SYNTHETIC-SYSTEMD-SVC-SCRIPT",
                "root",
                true,
                "ev:lin-art1",
            ),
        );
        cur_lin.artifacts.insert(
            "art:lin-cron-sh".to_string(),
            Artifact::new(
                "art:lin-cron-sh",
                "linux-app01.corp.example",
                "virtual://linux-app01.corp.example/opt/lab/cron-sync.sh",
                ArtifactKind::ScriptPayload,
                "SYNTHETIC-CRON-SCRIPT",
                "appuser",
                true,
                "ev:lin-art2",
            ),
        );

        // --- Host 3, 4, 5: clean baselines ---
        let base_files = SystemSnapshot::new(
            "snap-files-base",
            "files-int.corp.example",
            HostPlatform::Linux,
            BASELINE_TIMESTAMP,
        )
        .unwrap();
        let mut cur_files = base_files.clone();
        cur_files.snapshot_id = "snap-files-cur".to_string();
        cur_files.captured_at = AUDIT_TIMESTAMP.to_string();

        let base_db = SystemSnapshot::new(
            "snap-db-base",
            "db-tier0.corp.example",
            HostPlatform::Linux,
            BASELINE_TIMESTAMP,
        )
        .unwrap();
        let mut cur_db = base_db.clone();
        cur_db.snapshot_id = "snap-db-cur".to_string();
        cur_db.captured_at = AUDIT_TIMESTAMP.to_string();

        let base_vault = SystemSnapshot::new(
            "snap-vault-base",
            "audit-vault.corp.example",
            HostPlatform::Linux,
            BASELINE_TIMESTAMP,
        )
        .unwrap();
        let mut cur_vault = base_vault.clone();
        cur_vault.snapshot_id = "snap-vault-cur".to_string();
        cur_vault.captured_at = AUDIT_TIMESTAMP.to_string();

        let mut baseline_snapshots = BTreeMap::new();
        baseline_snapshots.insert(base_win.host.clone(), base_win);
        baseline_snapshots.insert(base_lin.host.clone(), base_lin);
        baseline_snapshots.insert(base_files.host.clone(), base_files);
        baseline_snapshots.insert(base_db.host.clone(), base_db);
        baseline_snapshots.insert(base_vault.host.clone(), base_vault);

        let mut current_snapshots = BTreeMap::new();
        current_snapshots.insert(cur_win.host.clone(), cur_win);
        current_snapshots.insert(cur_lin.host.clone(), cur_lin);
        current_snapshots.insert(cur_files.host.clone(), cur_files);
        current_snapshots.insert(cur_db.host.clone(), cur_db);
        current_snapshots.insert(cur_vault.host.clone(), cur_vault);

        let mut access_grants = BTreeMap::new();
        access_grants.insert(
            "grant:win-lin-ssh".to_string(),
            AccessGrant::new(
                "grant:win-lin-ssh",
                "win-srv01.corp.example",
                "linux-app01.corp.example",
                "svc_deploy",
                AccessKind::SshRoleKey,
                PrivilegeLevel::LocalAdmin,
                EXPIRY_FUTURE_TIMESTAMP,
                true,
                "ev:grant-win-lin",
            ),
        );
        access_grants.insert(
            "grant:lin-files-read".to_string(),
            AccessGrant::new(
                "grant:lin-files-read",
                "linux-app01.corp.example",
                "files-int.corp.example",
                "appuser",
                AccessKind::FileShareRead,
                PrivilegeLevel::StandardUser,
                EXPIRY_FUTURE_TIMESTAMP,
                true,
                "ev:grant-lin-files",
            ),
        );
        access_grants.insert(
            "grant:win-db-expired".to_string(),
            AccessGrant::new(
                "grant:win-db-expired",
                "win-srv01.corp.example",
                "db-tier0.corp.example",
                "svc_maintenance",
                AccessKind::WinRmManagement,
                PrivilegeLevel::LocalAdmin,
                EXPIRY_PAST_TIMESTAMP,
                true,
                "ev:grant-win-db-exp",
            ),
        );
        access_grants.insert(
            "grant:files-vault-low".to_string(),
            AccessGrant::new(
                "grant:files-vault-low",
                "files-int.corp.example",
                "audit-vault.corp.example",
                "appuser",
                AccessKind::RemoteServiceAdmin,
                PrivilegeLevel::StandardUser,
                EXPIRY_FUTURE_TIMESTAMP,
                false,
                "ev:grant-files-vault-low",
            ),
        );

        let lateral_edges = vec![
            LateralEdge {
                edge_id: "edge:win-to-lin".to_string(),
                from_host: "win-srv01.corp.example".to_string(),
                to_host: "linux-app01.corp.example".to_string(),
                required_access: AccessKind::SshRoleKey,
                min_privilege: PrivilegeLevel::ServiceAccount,
                network_reachable: true,
            },
            LateralEdge {
                edge_id: "edge:lin-to-files".to_string(),
                from_host: "linux-app01.corp.example".to_string(),
                to_host: "files-int.corp.example".to_string(),
                required_access: AccessKind::FileShareRead,
                min_privilege: PrivilegeLevel::StandardUser,
                network_reachable: true,
            },
            LateralEdge {
                edge_id: "edge:lin-to-win-cycle".to_string(),
                from_host: "linux-app01.corp.example".to_string(),
                to_host: "win-srv01.corp.example".to_string(),
                required_access: AccessKind::WinRmManagement,
                min_privilege: PrivilegeLevel::LocalAdmin,
                network_reachable: true,
            },
            LateralEdge {
                edge_id: "edge:files-to-db-segmented".to_string(),
                from_host: "files-int.corp.example".to_string(),
                to_host: "db-tier0.corp.example".to_string(),
                required_access: AccessKind::SshRoleKey,
                min_privilege: PrivilegeLevel::ServiceAccount,
                network_reachable: false,
            },
            LateralEdge {
                edge_id: "edge:win-to-db-expired".to_string(),
                from_host: "win-srv01.corp.example".to_string(),
                to_host: "db-tier0.corp.example".to_string(),
                required_access: AccessKind::WinRmManagement,
                min_privilege: PrivilegeLevel::LocalAdmin,
                network_reachable: true,
            },
            LateralEdge {
                edge_id: "edge:win-to-payroll-ooscope".to_string(),
                from_host: "win-srv01.corp.example".to_string(),
                to_host: "payroll-prod.corp.example".to_string(),
                required_access: AccessKind::RemoteServiceAdmin,
                min_privilege: PrivilegeLevel::LocalAdmin,
                network_reachable: true,
            },
            LateralEdge {
                edge_id: "edge:files-to-vault-lowpriv".to_string(),
                from_host: "files-int.corp.example".to_string(),
                to_host: "audit-vault.corp.example".to_string(),
                required_access: AccessKind::RemoteServiceAdmin,
                min_privilege: PrivilegeLevel::LocalAdmin,
                network_reachable: true,
            },
        ];

        let mut telemetry_events = BTreeMap::new();
        let mut add_tel = |id: &str,
                           host: &str,
                           channel: TelemetryChannel,
                           code: &str,
                           summary: &str,
                           subject: &str| {
            telemetry_events.insert(
                id.to_string(),
                TelemetryEvent {
                    event_id: id.to_string(),
                    timestamp: AUDIT_TIMESTAMP.to_string(),
                    host: host.to_string(),
                    channel,
                    event_code: code.to_string(),
                    summary: summary.to_string(),
                    subject_id: subject.to_string(),
                    evidence_id: "ev:tel-link".to_string(),
                },
            );
        };
        add_tel(
            "tel:win-autorun",
            "win-srv01.corp.example",
            TelemetryChannel::ProcessAudit,
            "MOCK-PROC-4688",
            "Autorun script entry registered at virtual run key",
            "run:agent-boot",
        );
        add_tel(
            "tel:win-policy",
            "win-srv01.corp.example",
            TelemetryChannel::PolicyChangeAudit,
            "MOCK-POL-4719",
            "TaskScheduler audit setting modified to Disabled",
            "pol:task-audit",
        );
        add_tel(
            "tel:win-acct",
            "win-srv01.corp.example",
            TelemetryChannel::AuthAudit,
            "MOCK-AUTH-4732",
            "svc_maintenance added to RemoteManagementUsers",
            "svc_maintenance",
        );
        add_tel(
            "tel:lin-svc",
            "linux-app01.corp.example",
            TelemetryChannel::SystemdJournal,
            "MOCK-SYSD-101",
            "New systemd unit telemetry-forwarder enabled",
            "svc:telemetry-forwarder",
        );
        add_tel(
            "tel:lin-cron",
            "linux-app01.corp.example",
            TelemetryChannel::CronAudit,
            "MOCK-CRON-202",
            "Hourly cron helper added for appuser",
            "startup:cron-profile",
        );
        add_tel(
            "tel:lin-dns-flow",
            "win-srv01.corp.example",
            TelemetryChannel::FirewallFlow,
            "MOCK-NET-305",
            "Off-hours DNS TXT metadata anomaly",
            "flow:dns-beacon",
        );
        add_tel(
            "tel:files-exfil-flow",
            "files-int.corp.example",
            TelemetryChannel::FirewallFlow,
            "MOCK-NET-901",
            "High volume external HTTPS transfer metadata on RestrictedRegulated store",
            "flow:exfil-staging",
        );
        add_tel(
            "tel:win-public-flow",
            "win-srv01.corp.example",
            TelemetryChannel::FirewallFlow,
            "MOCK-NET-102",
            "Small external documentation fetch metadata",
            "flow:public-check",
        );
        add_tel(
            "tel:lat-ssh",
            "win-srv01.corp.example",
            TelemetryChannel::AuthAudit,
            "MOCK-SSH-501",
            "SshRoleKey session grant observed",
            "edge:win-to-lin",
        );

        let data_flows = vec![
            DataFlow::new(
                "flow:backup-ok",
                "linux-app01.corp.example",
                "backup01.corp.example",
                DestinationCategory::AuthorizedBackup,
                FlowProtocol::SftpMeta,
                12_000_000,
                120,
                DataSensitivity::Internal,
                false,
                "ev:flow-backup",
            ),
            DataFlow::new(
                "flow:dns-beacon",
                "win-srv01.corp.example",
                "telemetry.drop.example",
                DestinationCategory::UnapprovedExternal,
                FlowProtocol::DnsTxtMeta,
                48_000,
                900,
                DataSensitivity::Confidential,
                true,
                "ev:flow-dns",
            ),
            DataFlow::new(
                "flow:exfil-staging",
                "files-int.corp.example",
                "archive.unapproved.example",
                DestinationCategory::UnapprovedExternal,
                FlowProtocol::HttpsMeta,
                25_000_000,
                180,
                DataSensitivity::RestrictedRegulated,
                true,
                "ev:flow-exfil",
            ),
            DataFlow::new(
                "flow:public-check",
                "win-srv01.corp.example",
                "docs.mirror.example",
                DestinationCategory::UnknownExternal,
                FlowProtocol::HttpsMeta,
                4_096,
                5,
                DataSensitivity::Public,
                false,
                "ev:flow-public",
            ),
        ];

        Self::new(
            scope,
            AUDIT_TIMESTAMP,
            baseline_snapshots,
            current_snapshots,
            ev,
            access_grants,
            lateral_edges,
            telemetry_events,
            data_flows,
        )
        .expect("standard fixture phải luôn hợp lệ")
    }

    pub fn baseline_snapshots(&self) -> &BTreeMap<String, SystemSnapshot> {
        &self.baseline_snapshots
    }

    pub fn current_snapshots(&self) -> &BTreeMap<String, SystemSnapshot> {
        &self.current_snapshots
    }

    pub fn evidence_store(&self) -> &BTreeMap<String, EvidenceRecord> {
        &self.evidence_store
    }

    pub fn telemetry_events(&self) -> &BTreeMap<String, TelemetryEvent> {
        &self.telemetry_events
    }

    pub fn access_grants(&self) -> &BTreeMap<String, AccessGrant> {
        &self.access_grants
    }

    pub fn data_flows(&self) -> &[DataFlow] {
        &self.data_flows
    }

    pub fn lateral_edges(&self) -> &[LateralEdge] {
        &self.lateral_edges
    }

    pub fn cleanup_snapshot(&self, host: &str) -> Option<&SystemSnapshot> {
        self.cleanup_state.snapshots.get(host)
    }

    pub fn cleanup_access_grants(&self) -> &BTreeMap<String, AccessGrant> {
        &self.cleanup_state.access_grants
    }

    /// Thực hiện diff toàn bộ host theo chế độ read-only trên `baseline` và `current`.
    pub fn diff_all_hosts(&self) -> Result<Vec<SnapshotDiff>, PostExploitError> {
        let mut diffs = Vec::with_capacity(self.current_snapshots.len());
        for (host, current) in &self.current_snapshots {
            let baseline = self.baseline_snapshots.get(host).ok_or_else(|| {
                PostExploitError::SnapshotMismatch(format!("thiếu baseline cho '{host}'"))
            })?;
            diffs.push(diff_snapshots(baseline, current, &self.evidence_store)?);
        }
        Ok(diffs)
    }

    fn evaluate_lateral_edge(&self, edge: &LateralEdge) -> Result<AccessGrant, BlockedTransition> {
        if !self.scope.allowed_hosts.contains(&edge.from_host)
            || !self.scope.allowed_hosts.contains(&edge.to_host)
        {
            let bad_host = if !self.scope.allowed_hosts.contains(&edge.from_host) {
                &edge.from_host
            } else {
                &edge.to_host
            };
            return Err(BlockedTransition {
                edge_id: edge.edge_id.clone(),
                from_host: edge.from_host.clone(),
                to_host: edge.to_host.clone(),
                reason: LateralBlockReason::OutOfScope,
                detail: format!("host '{bad_host}' nằm ngoài AuditScope"),
            });
        }
        if !edge.network_reachable {
            return Err(BlockedTransition {
                edge_id: edge.edge_id.clone(),
                from_host: edge.from_host.clone(),
                to_host: edge.to_host.clone(),
                reason: LateralBlockReason::NetworkSegmented,
                detail: "tường lửa phân vùng mạng chặn kết nối trực tiếp".to_string(),
            });
        }

        let matching: Vec<&AccessGrant> = self
            .access_grants
            .values()
            .filter(|g| {
                g.source_host == edge.from_host
                    && g.target_host == edge.to_host
                    && g.access_kind == edge.required_access
            })
            .collect();
        if matching.is_empty() {
            return Err(BlockedTransition {
                edge_id: edge.edge_id.clone(),
                from_host: edge.from_host.clone(),
                to_host: edge.to_host.clone(),
                reason: LateralBlockReason::MissingAccessGrant,
                detail: format!("không có AccessGrant loại {:?}", edge.required_access),
            });
        }

        let active: Vec<&&AccessGrant> = matching
            .iter()
            .filter(|g| g.is_active_at(&self.audit_time))
            .collect();
        if active.is_empty() {
            return Err(BlockedTransition {
                edge_id: edge.edge_id.clone(),
                from_host: edge.from_host.clone(),
                to_host: edge.to_host.clone(),
                reason: LateralBlockReason::SessionExpired,
                detail: format!("tất cả grant đã hết hạn trước {}", self.audit_time),
            });
        }

        let privileged: Vec<&&AccessGrant> = active
            .iter()
            .copied()
            .filter(|g| g.privilege >= edge.min_privilege)
            .collect();
        if let Some(best) = privileged.into_iter().max_by_key(|g| g.privilege) {
            Ok((*best).clone())
        } else {
            Err(BlockedTransition {
                edge_id: edge.edge_id.clone(),
                from_host: edge.from_host.clone(),
                to_host: edge.to_host.clone(),
                reason: LateralBlockReason::InsufficientPrivilege,
                detail: format!("grant hiện có không đạt tối thiểu {:?}", edge.min_privilege),
            })
        }
    }

    /// Phân tích đồ thị lateral movement bằng DFS dựa trên stack (`Vec::pop`), chặn chu trình và out-of-scope.
    pub fn analyze_lateral_paths(
        &self,
        start_host: &str,
        max_depth: usize,
    ) -> Result<LateralGraphReport, PostExploitError> {
        let start = self.scope.require_host(start_host)?;
        let mut reachable_paths = Vec::new();
        let mut blocked_set = BTreeSet::new();

        type StackFrame = (
            String,
            Vec<String>,
            Vec<String>,
            Option<PrivilegeLevel>,
            BTreeSet<String>,
        );

        let mut stack: Vec<StackFrame> = vec![(
            start.clone(),
            vec![start.clone()],
            Vec::new(),
            None,
            BTreeSet::new(),
        )];

        let mut sorted_edges = self.lateral_edges.clone();
        sorted_edges.sort();

        while let Some((node, hops, edge_ids, min_priv, ev_ids)) = stack.pop() {
            if !edge_ids.is_empty()
                && let Some(priv_level) = min_priv
            {
                reachable_paths.push(LateralPath {
                    hops: hops.clone(),
                    edge_ids: edge_ids.clone(),
                    min_effective_privilege: priv_level,
                    evidence_ids: ev_ids.clone(),
                });
            }

            let outgoing: Vec<&LateralEdge> = sorted_edges
                .iter()
                .filter(|e| e.from_host == node)
                .collect();
            for edge in outgoing.into_iter().rev() {
                if hops.contains(&edge.to_host) {
                    blocked_set.insert(BlockedTransition {
                        edge_id: edge.edge_id.clone(),
                        from_host: edge.from_host.clone(),
                        to_host: edge.to_host.clone(),
                        reason: LateralBlockReason::CycleDetected,
                        detail: format!("tránh chu trình quay lại '{}'", edge.to_host),
                    });
                    continue;
                }
                if edge_ids.len() >= max_depth {
                    blocked_set.insert(BlockedTransition {
                        edge_id: edge.edge_id.clone(),
                        from_host: edge.from_host.clone(),
                        to_host: edge.to_host.clone(),
                        reason: LateralBlockReason::MaxDepthReached,
                        detail: format!("vượt max_depth={max_depth}"),
                    });
                    continue;
                }

                match self.evaluate_lateral_edge(edge) {
                    Ok(grant) => {
                        let mut next_hops = hops.clone();
                        next_hops.push(edge.to_host.clone());
                        let mut next_edges = edge_ids.clone();
                        next_edges.push(edge.edge_id.clone());
                        let next_priv = Some(match min_priv {
                            Some(p) => p.min(grant.privilege),
                            None => grant.privilege,
                        });
                        let mut next_evs = ev_ids.clone();
                        next_evs.insert(grant.evidence_id.clone());
                        stack.push((
                            edge.to_host.clone(),
                            next_hops,
                            next_edges,
                            next_priv,
                            next_evs,
                        ));
                    }
                    Err(blocked) => {
                        blocked_set.insert(blocked);
                    }
                }
            }
        }

        reachable_paths.sort();
        Ok(LateralGraphReport {
            start_host: start,
            max_depth,
            reachable_paths,
            blocked_transitions: blocked_set.into_iter().collect(),
        })
    }

    fn expected_telemetry_channel(
        platform: HostPlatform,
        mech: PersistenceMechanism,
    ) -> TelemetryChannel {
        match (platform, mech) {
            (HostPlatform::Windows, PersistenceMechanism::Service) => {
                TelemetryChannel::ServiceControlManager
            }
            (HostPlatform::Windows, PersistenceMechanism::ScheduledTask) => {
                TelemetryChannel::TaskScheduler
            }
            (HostPlatform::Windows, _) => TelemetryChannel::ProcessAudit,
            (HostPlatform::Linux, PersistenceMechanism::Service) => {
                TelemetryChannel::SystemdJournal
            }
            (HostPlatform::Linux, PersistenceMechanism::StartupEntry) => {
                TelemetryChannel::CronAudit
            }
            (HostPlatform::Linux, _) => TelemetryChannel::ProcessAudit,
        }
    }

    fn telemetry_for_subject(&self, host: &str, subject_id: &str) -> BTreeSet<String> {
        self.telemetry_events
            .values()
            .filter(|ev| ev.host == host && ev.subject_id == subject_id)
            .map(|ev| ev.event_id.clone())
            .collect()
    }

    /// Tổng hợp audit read-only toàn diện: snapshot diff, persistence, policy, lateral, telemetry và data flows.
    pub fn run_audit(
        &self,
        start_host: &str,
        max_depth: usize,
    ) -> Result<AuditReport, PostExploitError> {
        let report = self.collect_audit_components(start_host, max_depth)?;
        report.validate(self)?;
        Ok(report)
    }

    #[allow(clippy::too_many_arguments)]
    fn push_persistence_finding(
        &self,
        findings: &mut Vec<AuditFinding>,
        host: &str,
        platform: HostPlatform,
        current_snap: &SystemSnapshot,
        rec: &PersistenceRecord,
        id_tag: Option<&str>,
        kind_label: &str,
    ) -> Result<(), PostExploitError> {
        let expected_chan = Self::expected_telemetry_channel(platform, rec.mechanism);
        let chan_enabled = current_snap
            .telemetry_controls
            .get(&expected_chan)
            .is_some_and(|c| c.enabled);
        let tel_ids = self.telemetry_for_subject(host, &rec.id);
        let gap = if !chan_enabled {
            Some(format!(
                "Kênh {} bị tắt trên {host} nên không thu được event cho {}",
                expected_chan.as_str(),
                rec.id
            ))
        } else if tel_ids.is_empty() {
            Some(format!(
                "Không ghi nhận telemetry khớp với {} trên kênh {}",
                rec.id,
                expected_chan.as_str()
            ))
        } else {
            None
        };

        let (impact, severity) = if rec.privilege == PrivilegeLevel::SystemOrRoot
            || (rec.privilege == PrivilegeLevel::LocalAdmin && !rec.signature_trusted)
        {
            (ImpactLevel::High, Severity::High)
        } else if rec.autostart {
            (ImpactLevel::Medium, Severity::Medium)
        } else {
            (ImpactLevel::Low, Severity::Low)
        };

        let mut evidence_ids = BTreeSet::from([rec.evidence_id.clone()]);
        let mut cleanup_refs = vec![format!("persistence:{}", rec.id)];
        if let Some(art_id) = &rec.artifact_id {
            if let Some(art) = current_snap.artifacts.get(art_id) {
                evidence_ids.insert(art.evidence_id.clone());
            }
            cleanup_refs.push(format!("artifact:{art_id}"));
        }

        let finding_id = match id_tag {
            Some(tag) => format!("find:{host}:{tag}:{}", rec.id),
            None => format!("find:{host}:{}", rec.id),
        };
        let f = AuditFinding {
            finding_id,
            host: host.to_string(),
            category: FindingCategory::Persistence,
            severity,
            impact,
            title: format!(
                "Persistence {kind_label} ({}) tại {}",
                rec.mechanism.as_str(),
                rec.virtual_location
            ),
            preconditions: vec![
                format!(
                    "Quyền tạo mục khởi động {:?} thuộc owner '{}'",
                    rec.privilege, rec.owner
                ),
                format!("Chữ ký tin cậy: {}", rec.signature_trusted),
            ],
            evidence_ids,
            telemetry_ids: tel_ids,
            detection_gap: gap,
            detection_rule: format!(
                "Bắt sự kiện thay đổi {} trên kênh {} và xác minh chữ ký nguồn gốc",
                rec.mechanism.as_str(),
                expected_chan.as_str()
            ),
            response_action: format!(
                "Cô lập host {host} ở chế độ giám sát và vô hiệu hóa/khôi phục mục {}",
                rec.id
            ),
            remediation: format!(
                "Xóa hoặc khôi phục {}, áp dụng least-privilege và chỉ cho phép binary/script đã ký",
                rec.id
            ),
            cleanup_refs,
        };
        f.validate_invariants(&self.evidence_store, &self.telemetry_events)?;
        findings.push(f);
        Ok(())
    }

    fn collect_audit_components(
        &self,
        start_host: &str,
        max_depth: usize,
    ) -> Result<AuditReport, PostExploitError> {
        self.scope.validate()?;
        let canon_start = self.scope.require_host(start_host)?;
        let diffs = self.diff_all_hosts()?;
        let mut findings = Vec::new();

        for diff in &diffs {
            let baseline_snap = self.baseline_snapshots.get(&diff.host).ok_or_else(|| {
                PostExploitError::SnapshotMismatch(format!("thiếu baseline cho '{}'", diff.host))
            })?;
            let current_snap = self.current_snapshots.get(&diff.host).ok_or_else(|| {
                PostExploitError::SnapshotMismatch(format!("thiếu snapshot cho '{}'", diff.host))
            })?;
            let mut linked_artifacts = BTreeSet::new();

            for rec in &diff.added_persistence {
                if let Some(art_id) = &rec.artifact_id {
                    linked_artifacts.insert(art_id.clone());
                }
                self.push_persistence_finding(
                    &mut findings,
                    &diff.host,
                    diff.platform,
                    current_snap,
                    rec,
                    None,
                    "mới",
                )?;
            }
            for (_base_rec, cur_rec) in &diff.modified_persistence {
                if let Some(art_id) = &cur_rec.artifact_id {
                    linked_artifacts.insert(art_id.clone());
                }
                self.push_persistence_finding(
                    &mut findings,
                    &diff.host,
                    diff.platform,
                    current_snap,
                    cur_rec,
                    Some("modified"),
                    "bị sửa",
                )?;
            }
            for rem_rec in &diff.removed_persistence {
                let expected_chan =
                    Self::expected_telemetry_channel(diff.platform, rem_rec.mechanism);
                let tel_ids = self.telemetry_for_subject(&diff.host, &rem_rec.id);
                let gap = if tel_ids.is_empty() {
                    Some(format!(
                        "Mục persistence baseline '{}' bị xóa không có telemetry khớp trên kênh {}",
                        rem_rec.id,
                        expected_chan.as_str()
                    ))
                } else {
                    None
                };
                let f = AuditFinding {
                    finding_id: format!("find:{}:removed:{}", diff.host, rem_rec.id),
                    host: diff.host.clone(),
                    category: FindingCategory::Persistence,
                    severity: Severity::Medium,
                    impact: ImpactLevel::Medium,
                    title: format!("Persistence baseline '{}' bị xóa bất thường", rem_rec.id),
                    preconditions: vec![format!("Mục '{}' từng tồn tại ở baseline", rem_rec.id)],
                    evidence_ids: BTreeSet::from([rem_rec.evidence_id.clone()]),
                    telemetry_ids: tel_ids,
                    detection_gap: gap,
                    detection_rule: format!(
                        "Cảnh báo xóa service/task chuẩn trên kênh {}",
                        expected_chan.as_str()
                    ),
                    response_action: format!(
                        "Kiểm tra tình trạng dịch vụ '{}' trên {}",
                        rem_rec.id, diff.host
                    ),
                    remediation: format!("Khôi phục cấu hình baseline cho '{}'", rem_rec.id),
                    cleanup_refs: vec![format!("persistence:{}", rem_rec.id)],
                };
                f.validate_invariants(&self.evidence_store, &self.telemetry_events)?;
                findings.push(f);
            }

            for art in &diff.added_artifacts {
                if linked_artifacts.contains(&art.id) {
                    continue;
                }
                let tel_ids = self.telemetry_for_subject(&diff.host, &art.id);
                let f = AuditFinding {
                    finding_id: format!("find:{}:art:{}", diff.host, art.id),
                    host: diff.host.clone(),
                    category: FindingCategory::Persistence,
                    severity: Severity::Medium,
                    impact: ImpactLevel::Medium,
                    title: format!(
                        "Artifact mới '{}' ({:?}) tại {}",
                        art.id, art.kind, art.virtual_location
                    ),
                    preconditions: vec![format!(
                        "Artifact do owner '{}' tạo ngoài baseline",
                        art.owner
                    )],
                    evidence_ids: BTreeSet::from([art.evidence_id.clone()]),
                    telemetry_ids: tel_ids.clone(),
                    detection_gap: if tel_ids.is_empty() {
                        Some("Chưa ghi nhận ProcessAudit cho artifact độc lập".to_string())
                    } else {
                        None
                    },
                    detection_rule: "Quét hash artifact mới xuất hiện trong thư mục staging/lab"
                        .to_string(),
                    response_action: format!(
                        "Cách ly và xóa artifact '{}' khỏi {}",
                        art.id, diff.host
                    ),
                    remediation: "Thu dọn toàn bộ artifact thử nghiệm và kiểm chứng hash baseline"
                        .to_string(),
                    cleanup_refs: vec![format!("artifact:{}", art.id)],
                };
                f.validate_invariants(&self.evidence_store, &self.telemetry_events)?;
                findings.push(f);
            }

            for (base_art, cur_art) in &diff.modified_artifacts {
                let tel_ids = self.telemetry_for_subject(&diff.host, &cur_art.id);
                let f = AuditFinding {
                    finding_id: format!("find:{}:art_modified:{}", diff.host, cur_art.id),
                    host: diff.host.clone(),
                    category: FindingCategory::Persistence,
                    severity: Severity::High,
                    impact: ImpactLevel::High,
                    title: format!(
                        "Artifact baseline '{}' bị thay đổi hash/vị trí tại {}",
                        cur_art.id, cur_art.virtual_location
                    ),
                    preconditions: vec![format!(
                        "SHA-256 lệch từ '{}' thành '{}'",
                        base_art.sha256_digest, cur_art.sha256_digest
                    )],
                    evidence_ids: BTreeSet::from([
                        base_art.evidence_id.clone(),
                        cur_art.evidence_id.clone(),
                    ]),
                    telemetry_ids: tel_ids.clone(),
                    detection_gap: if tel_ids.is_empty() {
                        Some("Chưa ghi nhận ProcessAudit cho sửa đổi artifact baseline".to_string())
                    } else {
                        None
                    },
                    detection_rule: "Giám sát FIM/SHA-256 mọi tệp nhị phân và cấu hình baseline"
                        .to_string(),
                    response_action: format!(
                        "Cách ly '{}' và khôi phục artifact '{}' từ golden image",
                        diff.host, cur_art.id
                    ),
                    remediation: format!(
                        "Khôi phục '{}' về digest chuẩn {}",
                        cur_art.id, base_art.sha256_digest
                    ),
                    cleanup_refs: vec![format!("artifact:{}", cur_art.id)],
                };
                f.validate_invariants(&self.evidence_store, &self.telemetry_events)?;
                findings.push(f);
            }

            for rem_art in &diff.removed_artifacts {
                let tel_ids = self.telemetry_for_subject(&diff.host, &rem_art.id);
                let f = AuditFinding {
                    finding_id: format!("find:{}:art_removed:{}", diff.host, rem_art.id),
                    host: diff.host.clone(),
                    category: FindingCategory::Persistence,
                    severity: Severity::Medium,
                    impact: ImpactLevel::Medium,
                    title: format!(
                        "Artifact baseline '{}' bị xóa khỏi {}",
                        rem_art.id, rem_art.virtual_location
                    ),
                    preconditions: vec![format!(
                        "Artifact '{}' tồn tại ở baseline nhưng vắng mặt ở hiện trạng",
                        rem_art.id
                    )],
                    evidence_ids: BTreeSet::from([rem_art.evidence_id.clone()]),
                    telemetry_ids: tel_ids.clone(),
                    detection_gap: if tel_ids.is_empty() {
                        Some("Thiếu log ProcessAudit khi xóa tệp baseline".to_string())
                    } else {
                        None
                    },
                    detection_rule: "Cảnh báo xóa tệp bảo mật hoặc tệp cấu hình baseline"
                        .to_string(),
                    response_action: format!(
                        "Kiểm tra nguyên nhân mất '{}' trên {}",
                        rem_art.id, diff.host
                    ),
                    remediation: format!("Khôi phục artifact '{}' từ baseline", rem_art.id),
                    cleanup_refs: vec![format!("artifact:{}", rem_art.id)],
                };
                f.validate_invariants(&self.evidence_store, &self.telemetry_events)?;
                findings.push(f);
            }

            for acct in &diff.added_accounts {
                let tel_ids = self.telemetry_for_subject(&diff.host, &acct.account_id);
                let (impact, severity) = if acct.privilege >= PrivilegeLevel::LocalAdmin {
                    (ImpactLevel::High, Severity::High)
                } else {
                    (ImpactLevel::Medium, Severity::Medium)
                };
                let f = AuditFinding {
                    finding_id: format!("find:{}:acct_added:{}", diff.host, acct.account_id),
                    host: diff.host.clone(),
                    category: FindingCategory::AccountDrift,
                    severity,
                    impact,
                    title: format!(
                        "Tài khoản mới '{}' được thêm ngoài baseline",
                        acct.account_id
                    ),
                    preconditions: vec![format!(
                        "Quyền {:?}, nhóm {:?}",
                        acct.privilege, acct.groups
                    )],
                    evidence_ids: BTreeSet::from([acct.evidence_id.clone()]),
                    telemetry_ids: tel_ids.clone(),
                    detection_gap: if tel_ids.is_empty() {
                        Some("Thiếu AuthAudit event cho tài khoản tạo mới".to_string())
                    } else {
                        None
                    },
                    detection_rule: "Cảnh báo tạo tài khoản cục bộ ngoài quy trình IAM".to_string(),
                    response_action: format!("Vô hiệu hóa/xóa tài khoản '{}'", acct.account_id),
                    remediation: "Xóa tài khoản phát sinh ngoài baseline và xoay vòng khóa dịch vụ"
                        .to_string(),
                    cleanup_refs: vec![format!("account:{}", acct.account_id)],
                };
                f.validate_invariants(&self.evidence_store, &self.telemetry_events)?;
                findings.push(f);
            }

            for (_base_acct, cur_acct) in &diff.modified_accounts {
                let tel_ids = self.telemetry_for_subject(&diff.host, &cur_acct.account_id);
                let (impact, severity) =
                    if cur_acct.enabled && cur_acct.privilege >= PrivilegeLevel::LocalAdmin {
                        (ImpactLevel::High, Severity::High)
                    } else {
                        (ImpactLevel::Medium, Severity::Medium)
                    };
                let f = AuditFinding {
                    finding_id: format!("find:{}:acct:{}", diff.host, cur_acct.account_id),
                    host: diff.host.clone(),
                    category: FindingCategory::AccountDrift,
                    severity,
                    impact,
                    title: format!(
                        "Tài khoản '{}' thay đổi trạng thái/nhóm quyền",
                        cur_acct.account_id
                    ),
                    preconditions: vec![format!(
                        "Tài khoản {} được kích hoạt với quyền {:?}",
                        cur_acct.account_id, cur_acct.privilege
                    )],
                    evidence_ids: BTreeSet::from([cur_acct.evidence_id.clone()]),
                    telemetry_ids: tel_ids.clone(),
                    detection_gap: if tel_ids.is_empty() {
                        Some("Thiếu AuthAudit event cho thay đổi nhóm tài khoản".to_string())
                    } else {
                        None
                    },
                    detection_rule:
                        "Cảnh báo sự kiện thêm thành viên nhóm quản trị / kích hoạt lại account dịch vụ"
                            .to_string(),
                    response_action: format!(
                        "Khóa lại '{}' và rà soát phiên truy cập liên quan",
                        cur_acct.account_id
                    ),
                    remediation: "Khôi phục account về trạng thái baseline bị vô hiệu hóa"
                        .to_string(),
                    cleanup_refs: vec![format!("account:{}", cur_acct.account_id)],
                };
                f.validate_invariants(&self.evidence_store, &self.telemetry_events)?;
                findings.push(f);
            }

            for rem_acct in &diff.removed_accounts {
                let tel_ids = self.telemetry_for_subject(&diff.host, &rem_acct.account_id);
                let f = AuditFinding {
                    finding_id: format!("find:{}:acct_removed:{}", diff.host, rem_acct.account_id),
                    host: diff.host.clone(),
                    category: FindingCategory::AccountDrift,
                    severity: Severity::Medium,
                    impact: ImpactLevel::Medium,
                    title: format!(
                        "Tài khoản baseline '{}' bị xóa khỏi {}",
                        rem_acct.account_id, diff.host
                    ),
                    preconditions: vec![format!(
                        "Account '{}' tồn tại ở baseline với quyền {:?}",
                        rem_acct.account_id, rem_acct.privilege
                    )],
                    evidence_ids: BTreeSet::from([rem_acct.evidence_id.clone()]),
                    telemetry_ids: tel_ids.clone(),
                    detection_gap: if tel_ids.is_empty() {
                        Some("Thiếu AuthAudit event cho thao tác xóa tài khoản".to_string())
                    } else {
                        None
                    },
                    detection_rule: "Cảnh báo xóa tài khoản dịch vụ/vận hành chuẩn".to_string(),
                    response_action: format!(
                        "Xác minh và khôi phục tài khoản '{}'",
                        rem_acct.account_id
                    ),
                    remediation: format!(
                        "Khôi phục cấu hình account '{}' theo baseline",
                        rem_acct.account_id
                    ),
                    cleanup_refs: vec![format!("account:{}", rem_acct.account_id)],
                };
                f.validate_invariants(&self.evidence_store, &self.telemetry_events)?;
                findings.push(f);
            }

            for cur_pol in &diff.added_policies {
                let tel_ids = self.telemetry_for_subject(&diff.host, &cur_pol.policy_id);
                let f = AuditFinding {
                    finding_id: format!("find:{}:pol_added:{}", diff.host, cur_pol.policy_id),
                    host: diff.host.clone(),
                    category: FindingCategory::SecurityPolicyTamper,
                    severity: Severity::High,
                    impact: ImpactLevel::High,
                    title: format!(
                        "Policy mới '{}' ({} = '{}') thêm ngoài baseline",
                        cur_pol.policy_id, cur_pol.setting, cur_pol.current_value
                    ),
                    preconditions: vec![format!(
                        "Chính sách {} không tồn tại trong ảnh baseline",
                        cur_pol.policy_id
                    )],
                    evidence_ids: BTreeSet::from([cur_pol.evidence_id.clone()]),
                    telemetry_ids: tel_ids.clone(),
                    detection_gap: if tel_ids.is_empty() {
                        Some("Thiếu PolicyChangeAudit event khi chèn chính sách mới".to_string())
                    } else {
                        None
                    },
                    detection_rule:
                        "Giám sát PolicyChangeAudit cho mọi rule cấu hình phát sinh mới".to_string(),
                    response_action: format!(
                        "Gỡ bỏ chính sách '{}' và đối soát cấu hình thực tế",
                        cur_pol.policy_id
                    ),
                    remediation: format!(
                        "Xóa '{}' để đưa chính sách về baseline",
                        cur_pol.policy_id
                    ),
                    cleanup_refs: vec![format!("policy:{}", cur_pol.policy_id)],
                };
                f.validate_invariants(&self.evidence_store, &self.telemetry_events)?;
                findings.push(f);
            }

            for (_base_pol, cur_pol) in &diff.policy_drifts {
                let tel_ids = self.telemetry_for_subject(&diff.host, &cur_pol.policy_id);
                let f = AuditFinding {
                    finding_id: format!("find:{}:pol:{}", diff.host, cur_pol.policy_id),
                    host: diff.host.clone(),
                    category: FindingCategory::SecurityPolicyTamper,
                    severity: Severity::High,
                    impact: ImpactLevel::High,
                    title: format!(
                        "Policy '{}' lệch từ '{}' thành '{}'",
                        cur_pol.setting, cur_pol.secure_value, cur_pol.current_value
                    ),
                    preconditions: vec![format!(
                        "Cấu hình {} bị sửa ngoài chuẩn baseline",
                        cur_pol.policy_id
                    )],
                    evidence_ids: BTreeSet::from([cur_pol.evidence_id.clone()]),
                    telemetry_ids: tel_ids.clone(),
                    detection_gap: if tel_ids.is_empty() {
                        Some("Không có PolicyChangeAudit event khi sửa chính sách".to_string())
                    } else {
                        None
                    },
                    detection_rule:
                        "Giám sát PolicyChangeAudit và so sánh integrity cấu hình định kỳ"
                            .to_string(),
                    response_action:
                        "Bật lại chính sách audit chuẩn và đối chiếu khoảng thời gian mù log"
                            .to_string(),
                    remediation: format!(
                        "Khôi phục {} về '{}'",
                        cur_pol.policy_id, cur_pol.secure_value
                    ),
                    cleanup_refs: vec![format!("policy:{}", cur_pol.policy_id)],
                };
                f.validate_invariants(&self.evidence_store, &self.telemetry_events)?;
                findings.push(f);
            }

            for rem_pol in &diff.removed_policies {
                let tel_ids = self.telemetry_for_subject(&diff.host, &rem_pol.policy_id);
                let f = AuditFinding {
                    finding_id: format!("find:{}:pol_removed:{}", diff.host, rem_pol.policy_id),
                    host: diff.host.clone(),
                    category: FindingCategory::SecurityPolicyTamper,
                    severity: Severity::High,
                    impact: ImpactLevel::High,
                    title: format!(
                        "Policy baseline '{}' ('{}') bị xóa khỏi {}",
                        rem_pol.policy_id, rem_pol.setting, diff.host
                    ),
                    preconditions: vec![format!(
                        "Policy {} tồn tại ở baseline nhưng bị gỡ ở hiện trạng",
                        rem_pol.policy_id
                    )],
                    evidence_ids: BTreeSet::from([rem_pol.evidence_id.clone()]),
                    telemetry_ids: tel_ids.clone(),
                    detection_gap: if tel_ids.is_empty() {
                        Some(
                            "Thiếu PolicyChangeAudit event khi xóa chính sách baseline".to_string(),
                        )
                    } else {
                        None
                    },
                    detection_rule: "Cảnh báo ngay khi chính sách bảo mật baseline bị gỡ bỏ"
                        .to_string(),
                    response_action: format!("Khôi phục lại chính sách '{}'", rem_pol.policy_id),
                    remediation: format!(
                        "Áp dụng lại policy '{}' theo baseline",
                        rem_pol.policy_id
                    ),
                    cleanup_refs: vec![format!("policy:{}", rem_pol.policy_id)],
                };
                f.validate_invariants(&self.evidence_store, &self.telemetry_events)?;
                findings.push(f);
            }

            for (chan, base_ctrl) in &baseline_snap.telemetry_controls {
                if !base_ctrl.enabled {
                    continue;
                }
                let degraded = match current_snap.telemetry_controls.get(chan) {
                    None => Some(format!(
                        "Mất hoàn toàn cấu hình kênh {} (bị xóa khỏi current snapshot)",
                        chan.as_str()
                    )),
                    Some(ctrl) if !ctrl.enabled => Some(format!(
                        "Mất hoàn toàn tầm nhìn kênh {} (đã bị tắt)",
                        chan.as_str()
                    )),
                    Some(ctrl) if ctrl.retention_days < base_ctrl.retention_days => Some(format!(
                        "Thời gian lưu trữ kênh {} bị hạ từ {} xuống {} ngày",
                        chan.as_str(),
                        base_ctrl.retention_days,
                        ctrl.retention_days
                    )),
                    Some(_) => None,
                };
                if let Some(gap_reason) = degraded {
                    let matching_policies: Vec<&SecurityPolicyRecord> = diff
                        .policy_drifts
                        .iter()
                        .map(|(_, p)| p)
                        .chain(diff.added_policies.iter())
                        .chain(diff.removed_policies.iter())
                        .filter(|p| p.disables_channel == Some(*chan))
                        .collect();
                    let ev_ids: BTreeSet<String> = matching_policies
                        .iter()
                        .map(|p| p.evidence_id.clone())
                        .collect();
                    let fallback_ev = ev_ids.into_iter().next().ok_or_else(|| {
                        PostExploitError::MissingEvidence(format!(
                            "kênh {} suy giảm/bị xóa trên '{}' nhưng thiếu policy evidence",
                            chan.as_str(),
                            diff.host
                        ))
                    })?;
                    let tel_ids: BTreeSet<String> = matching_policies
                        .iter()
                        .flat_map(|p| self.telemetry_for_subject(&diff.host, &p.policy_id))
                        .collect();
                    let f = AuditFinding {
                        finding_id: format!("find:{}:telgap:{}", diff.host, chan.as_str()),
                        host: diff.host.clone(),
                        category: FindingCategory::TelemetryGap,
                        severity: Severity::High,
                        impact: ImpactLevel::High,
                        title: format!(
                            "Telemetry kênh {} bị tắt hoặc suy giảm trên {}",
                            chan.as_str(),
                            diff.host
                        ),
                        preconditions: vec![
                            "Kênh log từng bật ở baseline nay bị tắt, xóa hoặc giảm retention"
                                .to_string(),
                        ],
                        evidence_ids: BTreeSet::from([fallback_ev]),
                        telemetry_ids: tel_ids,
                        detection_gap: Some(gap_reason),
                        detection_rule:
                            "Cảnh báo ngay khi log heartbeat hoặc retention kênh trọng yếu thay đổi/ngừng gửi về SIEM"
                                .to_string(),
                        response_action:
                            "Bật lại kênh log, khôi phục retention ngay lập tức và kiểm tra snapshot lịch sử"
                                .to_string(),
                        remediation:
                            "Khóa quyền sửa audit policy và áp dụng bảo vệ cấu hình tập trung"
                                .to_string(),
                        cleanup_refs: matching_policies
                            .iter()
                            .map(|p| format!("policy:{}", p.policy_id))
                            .chain(std::iter::once(format!("telemetry:{}", chan.as_str())))
                            .collect(),
                    };
                    f.validate_invariants(&self.evidence_store, &self.telemetry_events)?;
                    findings.push(f);
                }
            }
        }

        let lateral = self.analyze_lateral_paths(&canon_start, max_depth)?;
        for path in &lateral.reachable_paths {
            if let (Some(first_edge), Some(last_host)) = (path.edge_ids.first(), path.hops.last()) {
                let tel_ids = self.telemetry_for_subject(&canon_start, first_edge);
                let (impact, severity) = if path.hops.len() >= 3
                    || path.min_effective_privilege >= PrivilegeLevel::LocalAdmin
                {
                    (ImpactLevel::High, Severity::High)
                } else {
                    (ImpactLevel::Medium, Severity::Medium)
                };
                let mut path_cleanup_refs = BTreeSet::new();
                for edge_id in &path.edge_ids {
                    if let Some(edge) = self.lateral_edges.iter().find(|e| &e.edge_id == edge_id) {
                        for grant in self.access_grants.values() {
                            if grant.created_in_lab
                                && grant.source_host == edge.from_host
                                && grant.target_host == edge.to_host
                                && grant.access_kind == edge.required_access
                                && grant.is_active_at(&self.audit_time)
                            {
                                path_cleanup_refs.insert(format!("grant:{}", grant.grant_id));
                            }
                        }
                    }
                }
                let f = AuditFinding {
                    finding_id: format!("find:lateral:{}", path.edge_ids.join("+")),
                    host: canon_start.clone(),
                    category: FindingCategory::LateralExposure,
                    severity,
                    impact,
                    title: format!(
                        "Đường lateral khả dụng tới '{last_host}' qua {} hop",
                        path.edge_ids.len()
                    ),
                    preconditions: vec![format!(
                        "Các AccessGrant còn hạn với quyền tối thiểu {:?}",
                        path.min_effective_privilege
                    )],
                    evidence_ids: path.evidence_ids.clone(),
                    telemetry_ids: tel_ids.clone(),
                    detection_gap: if tel_ids.is_empty() {
                        Some(
                            "Đường di chuyển ngang chưa gắn đầy đủ log phiên AuthAudit".to_string(),
                        )
                    } else {
                        None
                    },
                    detection_rule:
                        "Theo dõi phiên quản trị liên máy chủ và rút ngắn TTL AccessGrant"
                            .to_string(),
                    response_action: format!(
                        "Thu hồi AccessGrant thử nghiệm cho chuỗi {}",
                        path.edge_ids.join(" -> ")
                    ),
                    remediation: "Áp dụng Just-in-Time access và phân vùng liên kết giữa các tier"
                        .to_string(),
                    cleanup_refs: path_cleanup_refs.into_iter().collect(),
                };
                f.validate_invariants(&self.evidence_store, &self.telemetry_events)?;
                findings.push(f);
            }
        }

        let mut exfil_assessments = Vec::with_capacity(self.data_flows.len());
        for flow in &self.data_flows {
            let assessment = classify_data_flow(&self.scope, flow, &self.evidence_store)?;
            if let Some(sev) = assessment.risk.as_severity() {
                let tel_ids = self.telemetry_for_subject(&assessment.source_host, &flow.flow_id);
                let f = AuditFinding {
                    finding_id: format!("find:exfil:{}", flow.flow_id),
                    host: assessment.source_host.clone(),
                    category: FindingCategory::ExfilRisk,
                    severity: sev,
                    impact: assessment.impact,
                    title: format!(
                        "Rủi ro truyền dữ liệu {:?} tới '{}' ({} bytes)",
                        assessment.risk, assessment.destination, flow.bytes_sent
                    ),
                    preconditions: assessment.reasons.clone(),
                    evidence_ids: BTreeSet::from([flow.evidence_id.clone()]),
                    telemetry_ids: tel_ids.clone(),
                    detection_gap: if tel_ids.is_empty() {
                        Some("Thiếu telemetry FirewallFlow khớp flow".to_string())
                    } else {
                        None
                    },
                    detection_rule: "Giới hạn egress qua proxy allowlist và phát hiện DNS TXT / đột biến ngoài giờ".to_string(),
                    response_action: format!("Chặn đích '{}' và đối chiếu nguồn gốc {}", assessment.destination, flow.flow_id),
                    remediation: "Áp dụng DLP metadata policy và chặn egress không được phê duyệt".to_string(),
                    cleanup_refs: Vec::new(),
                };
                f.validate_invariants(&self.evidence_store, &self.telemetry_events)?;
                findings.push(f);
            }
            exfil_assessments.push(assessment);
        }

        findings.sort();
        exfil_assessments.sort();

        let input_hash = self.compute_input_hash(&canon_start, max_depth);
        Ok(AuditReport {
            schema_version: REPORT_SCHEMA_VERSION.to_string(),
            engagement_id: self.scope.engagement_id.clone(),
            generated_at: self.audit_time.clone(),
            input_hash,
            diffs,
            findings,
            lateral,
            exfil_assessments,
        })
    }

    pub fn build_cleanup_plan(&self, plan_id: &str) -> Result<CleanupPlan, PostExploitError> {
        let mut steps = Vec::new();
        for (host, current) in &self.cleanup_state.snapshots {
            let baseline = self.baseline_snapshots.get(host).ok_or_else(|| {
                PostExploitError::SnapshotMismatch(format!("thiếu baseline cho '{host}'"))
            })?;
            for rec_id in current.persistence_records.keys() {
                if !baseline.persistence_records.contains_key(rec_id) {
                    steps.push(CleanupStep::RemovePersistence {
                        host: host.clone(),
                        record_id: rec_id.clone(),
                    });
                }
            }
            for (rec_id, base_rec) in &baseline.persistence_records {
                if current.persistence_records.get(rec_id) != Some(base_rec) {
                    steps.push(CleanupStep::RestorePersistence {
                        host: host.clone(),
                        record_id: rec_id.clone(),
                    });
                }
            }
            for (art_id, art) in &current.artifacts {
                if !baseline.artifacts.contains_key(art_id) {
                    steps.push(CleanupStep::RemoveArtifact {
                        host: host.clone(),
                        artifact_id: art_id.clone(),
                        expected_sha256: Some(art.sha256_digest.clone()),
                    });
                }
            }
            for (art_id, base_art) in &baseline.artifacts {
                if current.artifacts.get(art_id) != Some(base_art) {
                    steps.push(CleanupStep::RestoreArtifact {
                        host: host.clone(),
                        artifact_id: art_id.clone(),
                    });
                }
            }
            let all_accts: BTreeSet<&str> = baseline
                .accounts
                .keys()
                .chain(current.accounts.keys())
                .map(String::as_str)
                .collect();
            for acct_id in all_accts {
                if baseline.accounts.get(acct_id) != current.accounts.get(acct_id) {
                    steps.push(CleanupStep::RestoreAccount {
                        host: host.clone(),
                        account_id: acct_id.to_string(),
                    });
                }
            }
            let all_policies: BTreeSet<&str> = baseline
                .security_policies
                .keys()
                .chain(current.security_policies.keys())
                .map(String::as_str)
                .collect();
            let mut covered_channels = BTreeSet::new();
            for pol_id in all_policies {
                if baseline.security_policies.get(pol_id) != current.security_policies.get(pol_id) {
                    if let Some(base_pol) = baseline.security_policies.get(pol_id)
                        && let Some(chan) = base_pol.disables_channel
                    {
                        covered_channels.insert(chan);
                    }
                    steps.push(CleanupStep::RestorePolicy {
                        host: host.clone(),
                        policy_id: pol_id.to_string(),
                    });
                }
            }
            let all_chans: BTreeSet<TelemetryChannel> = baseline
                .telemetry_controls
                .keys()
                .chain(current.telemetry_controls.keys())
                .copied()
                .collect();
            for chan in all_chans {
                if baseline.telemetry_controls.get(&chan) != current.telemetry_controls.get(&chan)
                    && !covered_channels.contains(&chan)
                {
                    steps.push(CleanupStep::RestoreTelemetryControl {
                        host: host.clone(),
                        channel: chan,
                    });
                }
            }
        }
        for (gid, grant) in &self.cleanup_state.access_grants {
            if grant.created_in_lab {
                steps.push(CleanupStep::RevokeAccessGrant {
                    grant_id: gid.clone(),
                });
            }
        }
        steps.sort();
        Ok(CleanupPlan {
            plan_id: plan_id.to_string(),
            engagement_id: self.scope.engagement_id.clone(),
            steps,
        })
    }

    /// Thực thi `CleanupPlan` nguyên tử trên `cleanup_state` cô lập; an toàn khi artifact đã mất và lũy đẳng tuyệt đối.
    pub fn apply_cleanup(
        &mut self,
        plan: &CleanupPlan,
    ) -> Result<CleanupExecutionReport, PostExploitError> {
        if plan.plan_id.trim().is_empty() {
            return Err(PostExploitError::ValidationFailed(
                "plan_id không được rỗng".to_string(),
            ));
        }
        if plan.engagement_id != self.scope.engagement_id {
            return Err(PostExploitError::OutOfScope(format!(
                "plan engagement '{}' khác '{}'",
                plan.engagement_id, self.scope.engagement_id
            )));
        }
        let mut staged = self.cleanup_state.clone();
        let mut applied_mutations = 0usize;
        let mut already_absent_steps = 0usize;
        let mut step_outcomes = Vec::with_capacity(plan.steps.len());

        for step in &plan.steps {
            match step {
                CleanupStep::RemovePersistence { host, record_id } => {
                    let canon_host = self.scope.require_host(host)?;
                    let base = self.baseline_snapshots.get(&canon_host).ok_or_else(|| {
                        PostExploitError::SnapshotMismatch(format!("thiếu baseline '{canon_host}'"))
                    })?;
                    if base.persistence_records.contains_key(record_id) {
                        return Err(PostExploitError::BaselineProtected(format!(
                            "không thể xóa persistence baseline '{record_id}' trên '{canon_host}'"
                        )));
                    }
                    let cur = staged.snapshots.get_mut(&canon_host).ok_or_else(|| {
                        PostExploitError::SnapshotMismatch(format!(
                            "thiếu cleanup snapshot '{canon_host}'"
                        ))
                    })?;
                    if cur.persistence_records.remove(record_id).is_some() {
                        applied_mutations += 1;
                        step_outcomes.push((step.clone(), CleanupStepStatus::Removed));
                    } else {
                        already_absent_steps += 1;
                        step_outcomes.push((step.clone(), CleanupStepStatus::AlreadyAbsent));
                    }
                }
                CleanupStep::RestorePersistence { host, record_id } => {
                    let canon_host = self.scope.require_host(host)?;
                    let base = self.baseline_snapshots.get(&canon_host).ok_or_else(|| {
                        PostExploitError::SnapshotMismatch(format!("thiếu baseline '{canon_host}'"))
                    })?;
                    let base_rec = base.persistence_records.get(record_id).ok_or_else(|| {
                        PostExploitError::SnapshotMismatch(format!(
                            "baseline thiếu persistence '{record_id}' trên '{canon_host}'"
                        ))
                    })?;
                    let cur = staged.snapshots.get_mut(&canon_host).ok_or_else(|| {
                        PostExploitError::SnapshotMismatch(format!(
                            "thiếu cleanup snapshot '{canon_host}'"
                        ))
                    })?;
                    if cur.persistence_records.get(record_id) != Some(base_rec) {
                        cur.persistence_records
                            .insert(record_id.clone(), base_rec.clone());
                        applied_mutations += 1;
                        step_outcomes.push((step.clone(), CleanupStepStatus::Restored));
                    } else {
                        already_absent_steps += 1;
                        step_outcomes.push((step.clone(), CleanupStepStatus::AlreadyAbsent));
                    }
                }
                CleanupStep::RemoveArtifact {
                    host,
                    artifact_id,
                    expected_sha256,
                } => {
                    let canon_host = self.scope.require_host(host)?;
                    let base = self.baseline_snapshots.get(&canon_host).ok_or_else(|| {
                        PostExploitError::SnapshotMismatch(format!("thiếu baseline '{canon_host}'"))
                    })?;
                    if base.artifacts.contains_key(artifact_id) {
                        return Err(PostExploitError::BaselineProtected(format!(
                            "không thể xóa artifact baseline '{artifact_id}' trên '{canon_host}'"
                        )));
                    }
                    let cur = staged.snapshots.get_mut(&canon_host).ok_or_else(|| {
                        PostExploitError::SnapshotMismatch(format!(
                            "thiếu cleanup snapshot '{canon_host}'"
                        ))
                    })?;
                    if let Some(existing) = cur.artifacts.get(artifact_id) {
                        if !existing.created_in_lab {
                            return Err(PostExploitError::BaselineProtected(format!(
                                "artifact '{artifact_id}' không mang cờ created_in_lab"
                            )));
                        }
                        if let Some(expected) = expected_sha256
                            && &existing.sha256_digest != expected
                        {
                            return Err(PostExploitError::ArtifactDigestMismatch {
                                artifact_id: artifact_id.clone(),
                                expected: expected.clone(),
                                actual: existing.sha256_digest.clone(),
                            });
                        }
                        cur.artifacts.remove(artifact_id);
                        applied_mutations += 1;
                        step_outcomes.push((step.clone(), CleanupStepStatus::Removed));
                    } else {
                        already_absent_steps += 1;
                        step_outcomes.push((step.clone(), CleanupStepStatus::AlreadyAbsent));
                    }
                }
                CleanupStep::RestoreArtifact { host, artifact_id } => {
                    let canon_host = self.scope.require_host(host)?;
                    let base = self.baseline_snapshots.get(&canon_host).ok_or_else(|| {
                        PostExploitError::SnapshotMismatch(format!("thiếu baseline '{canon_host}'"))
                    })?;
                    let base_art = base.artifacts.get(artifact_id).ok_or_else(|| {
                        PostExploitError::SnapshotMismatch(format!(
                            "baseline thiếu artifact '{artifact_id}' trên '{canon_host}'"
                        ))
                    })?;
                    let cur = staged.snapshots.get_mut(&canon_host).ok_or_else(|| {
                        PostExploitError::SnapshotMismatch(format!(
                            "thiếu cleanup snapshot '{canon_host}'"
                        ))
                    })?;
                    if cur.artifacts.get(artifact_id) != Some(base_art) {
                        cur.artifacts.insert(artifact_id.clone(), base_art.clone());
                        applied_mutations += 1;
                        step_outcomes.push((step.clone(), CleanupStepStatus::Restored));
                    } else {
                        already_absent_steps += 1;
                        step_outcomes.push((step.clone(), CleanupStepStatus::AlreadyAbsent));
                    }
                }
                CleanupStep::RestoreAccount { host, account_id } => {
                    let canon_host = self.scope.require_host(host)?;
                    let base = self.baseline_snapshots.get(&canon_host).ok_or_else(|| {
                        PostExploitError::SnapshotMismatch(format!("thiếu baseline '{canon_host}'"))
                    })?;
                    let cur = staged.snapshots.get_mut(&canon_host).ok_or_else(|| {
                        PostExploitError::SnapshotMismatch(format!(
                            "thiếu cleanup snapshot '{canon_host}'"
                        ))
                    })?;
                    match base.accounts.get(account_id) {
                        Some(base_acct) => {
                            if cur.accounts.get(account_id) != Some(base_acct) {
                                cur.accounts.insert(account_id.clone(), base_acct.clone());
                                applied_mutations += 1;
                                step_outcomes.push((step.clone(), CleanupStepStatus::Restored));
                            } else {
                                already_absent_steps += 1;
                                step_outcomes
                                    .push((step.clone(), CleanupStepStatus::AlreadyAbsent));
                            }
                        }
                        None => {
                            if cur.accounts.remove(account_id).is_some() {
                                applied_mutations += 1;
                                step_outcomes.push((step.clone(), CleanupStepStatus::Removed));
                            } else {
                                already_absent_steps += 1;
                                step_outcomes
                                    .push((step.clone(), CleanupStepStatus::AlreadyAbsent));
                            }
                        }
                    }
                }
                CleanupStep::RestorePolicy { host, policy_id } => {
                    let canon_host = self.scope.require_host(host)?;
                    let base = self.baseline_snapshots.get(&canon_host).ok_or_else(|| {
                        PostExploitError::SnapshotMismatch(format!("thiếu baseline '{canon_host}'"))
                    })?;
                    let cur = staged.snapshots.get_mut(&canon_host).ok_or_else(|| {
                        PostExploitError::SnapshotMismatch(format!(
                            "thiếu cleanup snapshot '{canon_host}'"
                        ))
                    })?;
                    match base.security_policies.get(policy_id) {
                        Some(base_pol) => {
                            let pol_changed =
                                cur.security_policies.get(policy_id) != Some(base_pol);
                            let mut chan_changed = false;
                            if let Some(chan) = base_pol.disables_channel {
                                match base.telemetry_controls.get(&chan) {
                                    Some(base_ctrl) => {
                                        if cur.telemetry_controls.get(&chan) != Some(base_ctrl) {
                                            cur.telemetry_controls.insert(chan, base_ctrl.clone());
                                            chan_changed = true;
                                        }
                                    }
                                    None => {
                                        if cur.telemetry_controls.remove(&chan).is_some() {
                                            chan_changed = true;
                                        }
                                    }
                                }
                            }
                            if pol_changed || chan_changed {
                                cur.security_policies
                                    .insert(policy_id.clone(), base_pol.clone());
                                applied_mutations += 1;
                                step_outcomes.push((step.clone(), CleanupStepStatus::Restored));
                            } else {
                                already_absent_steps += 1;
                                step_outcomes
                                    .push((step.clone(), CleanupStepStatus::AlreadyAbsent));
                            }
                        }
                        None => {
                            if cur.security_policies.remove(policy_id).is_some() {
                                applied_mutations += 1;
                                step_outcomes.push((step.clone(), CleanupStepStatus::Removed));
                            } else {
                                already_absent_steps += 1;
                                step_outcomes
                                    .push((step.clone(), CleanupStepStatus::AlreadyAbsent));
                            }
                        }
                    }
                }
                CleanupStep::RestoreTelemetryControl { host, channel } => {
                    let canon_host = self.scope.require_host(host)?;
                    let base = self.baseline_snapshots.get(&canon_host).ok_or_else(|| {
                        PostExploitError::SnapshotMismatch(format!("thiếu baseline '{canon_host}'"))
                    })?;
                    let cur = staged.snapshots.get_mut(&canon_host).ok_or_else(|| {
                        PostExploitError::SnapshotMismatch(format!(
                            "thiếu cleanup snapshot '{canon_host}'"
                        ))
                    })?;
                    match base.telemetry_controls.get(channel) {
                        Some(base_ctrl) => {
                            if cur.telemetry_controls.get(channel) != Some(base_ctrl) {
                                cur.telemetry_controls.insert(*channel, base_ctrl.clone());
                                applied_mutations += 1;
                                step_outcomes.push((step.clone(), CleanupStepStatus::Restored));
                            } else {
                                already_absent_steps += 1;
                                step_outcomes
                                    .push((step.clone(), CleanupStepStatus::AlreadyAbsent));
                            }
                        }
                        None => {
                            if cur.telemetry_controls.remove(channel).is_some() {
                                applied_mutations += 1;
                                step_outcomes.push((step.clone(), CleanupStepStatus::Removed));
                            } else {
                                already_absent_steps += 1;
                                step_outcomes
                                    .push((step.clone(), CleanupStepStatus::AlreadyAbsent));
                            }
                        }
                    }
                }
                CleanupStep::RevokeAccessGrant { grant_id } => {
                    if let Some(existing) = staged.access_grants.get(grant_id) {
                        if !existing.created_in_lab {
                            return Err(PostExploitError::BaselineProtected(format!(
                                "access grant '{grant_id}' là grant baseline không phải grant lab"
                            )));
                        }
                        staged.access_grants.remove(grant_id);
                        applied_mutations += 1;
                        step_outcomes.push((step.clone(), CleanupStepStatus::Revoked));
                    } else {
                        already_absent_steps += 1;
                        step_outcomes.push((step.clone(), CleanupStepStatus::AlreadyAbsent));
                    }
                }
            }
        }

        self.cleanup_state = staged;
        Ok(CleanupExecutionReport {
            plan_id: plan.plan_id.clone(),
            applied_mutations,
            already_absent_steps,
            step_outcomes,
            verification: self.verify_baseline(),
        })
    }

    /// Kiểm chứng trạng thái sau cleanup đã quay sạch về baseline chưa; mọi tàn dư hay sai lệch đều được báo cáo rõ.
    pub fn verify_baseline(&self) -> BaselineVerificationReport {
        let mut residual_drifts = Vec::new();
        let mut checked_hosts = Vec::new();

        for (host, base) in &self.baseline_snapshots {
            checked_hosts.push(host.clone());
            let Some(cur) = self.cleanup_state.snapshots.get(host) else {
                residual_drifts.push(format!("thiếu snapshot sau cleanup cho '{host}'"));
                continue;
            };
            for id in cur.persistence_records.keys() {
                if !base.persistence_records.contains_key(id) {
                    residual_drifts.push(format!("{host}: còn sót persistence '{id}'"));
                }
            }
            for (id, base_rec) in &base.persistence_records {
                match cur.persistence_records.get(id) {
                    None => {
                        residual_drifts.push(format!("{host}: mất persistence baseline '{id}'"))
                    }
                    Some(cur_rec) if cur_rec != base_rec => {
                        residual_drifts.push(format!(
                            "{host}: persistence baseline '{id}' chưa khớp baseline"
                        ));
                    }
                    Some(_) => {}
                }
            }
            for id in cur.artifacts.keys() {
                if !base.artifacts.contains_key(id) {
                    residual_drifts.push(format!("{host}: còn sót artifact '{id}'"));
                }
            }
            for (id, base_art) in &base.artifacts {
                match cur.artifacts.get(id) {
                    None => residual_drifts.push(format!("{host}: mất artifact baseline '{id}'")),
                    Some(cur_art) if cur_art != base_art => {
                        residual_drifts.push(format!(
                            "{host}: artifact baseline '{id}' chưa khớp baseline"
                        ));
                    }
                    Some(_) => {}
                }
            }
            for (id, base_acct) in &base.accounts {
                if cur.accounts.get(id) != Some(base_acct) {
                    residual_drifts.push(format!("{host}: account '{id}' chưa khớp baseline"));
                }
            }
            for id in cur.accounts.keys() {
                if !base.accounts.contains_key(id) {
                    residual_drifts.push(format!("{host}: còn sót account lab '{id}'"));
                }
            }
            for (id, base_pol) in &base.security_policies {
                if cur.security_policies.get(id) != Some(base_pol) {
                    residual_drifts.push(format!("{host}: policy '{id}' chưa khớp baseline"));
                }
            }
            for id in cur.security_policies.keys() {
                if !base.security_policies.contains_key(id) {
                    residual_drifts.push(format!("{host}: còn sót policy ngoài baseline '{id}'"));
                }
            }
            for (chan, base_ctrl) in &base.telemetry_controls {
                if cur.telemetry_controls.get(chan) != Some(base_ctrl) {
                    residual_drifts.push(format!(
                        "{host}: telemetry '{}' chưa về baseline",
                        chan.as_str()
                    ));
                }
            }
            for chan in cur.telemetry_controls.keys() {
                if !base.telemetry_controls.contains_key(chan) {
                    residual_drifts.push(format!(
                        "{host}: còn sót telemetry control ngoài baseline '{}'",
                        chan.as_str()
                    ));
                }
            }
        }
        for (gid, grant) in &self.cleanup_state.access_grants {
            if grant.created_in_lab {
                residual_drifts.push(format!("access grant lab '{gid}' chưa được thu hồi"));
            }
        }
        residual_drifts.sort();
        BaselineVerificationReport {
            clean: residual_drifts.is_empty(),
            checked_hosts,
            residual_drifts,
        }
    }

    fn compute_input_hash(&self, start_host: &str, max_depth: usize) -> String {
        use std::fmt::Write;
        let mut buf = format!(
            "eng={}|time={}|start={}|depth={}|active_harvest={}|real_os={}\n",
            self.scope.engagement_id,
            self.audit_time,
            start_host,
            max_depth,
            self.scope.allow_credential_harvesting,
            self.scope.allow_real_os_mutation,
        );
        for host in &self.scope.allowed_hosts {
            let _ = writeln!(&mut buf, "allowed_host:{host}");
        }
        for dest in &self.scope.authorized_backup_destinations {
            let _ = writeln!(&mut buf, "backup_dest:{dest}");
        }
        for ev in self.evidence_store.values() {
            let _ = writeln!(&mut buf, "ev:{ev:?}");
        }
        for (host, snap) in &self.baseline_snapshots {
            let _ = writeln!(&mut buf, "base:{host}:{snap:?}");
        }
        for (host, snap) in &self.current_snapshots {
            let _ = writeln!(&mut buf, "cur:{host}:{snap:?}");
        }
        for grant in self.access_grants.values() {
            let _ = writeln!(&mut buf, "grant:{grant:?}");
        }
        let mut sorted_edges = self.lateral_edges.clone();
        sorted_edges.sort();
        for edge in &sorted_edges {
            let _ = writeln!(&mut buf, "edge:{edge:?}");
        }
        for tel in self.telemetry_events.values() {
            let _ = writeln!(&mut buf, "tel:{tel:?}");
        }
        let mut sorted_flows = self.data_flows.clone();
        sorted_flows.sort();
        for flow in &sorted_flows {
            let _ = writeln!(&mut buf, "flow:{flow:?}");
        }
        sha256_hex(buf.as_bytes())
    }
}

// ============================================================================
// 6. BÁO CÁO AUDIT TÁI LẬP & VALIDATION
// ============================================================================

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct AuditReport {
    pub schema_version: String,
    pub engagement_id: String,
    pub generated_at: String,
    pub input_hash: String,
    pub diffs: Vec<SnapshotDiff>,
    pub findings: Vec<AuditFinding>,
    pub lateral: LateralGraphReport,
    pub exfil_assessments: Vec<ExfilAssessment>,
}

fn json_escape(input: &str) -> String {
    let mut out = String::with_capacity(input.len());
    for ch in input.chars() {
        match ch {
            '"' => out.push_str("\\\""),
            '\\' => out.push_str("\\\\"),
            '\n' => out.push_str("\\n"),
            '\r' => out.push_str("\\r"),
            '\t' => out.push_str("\\t"),
            c if c.is_control() => {
                use std::fmt::Write;
                let _ = write!(&mut out, "\\u{:04x}", c as u32);
            }
            c => out.push(c),
        }
    }
    out
}

fn markdown_escape(input: &str) -> String {
    let mut out = String::with_capacity(input.len());
    for ch in input.chars() {
        match ch {
            '&' => out.push_str("&amp;"),
            '\\' => out.push_str("\\\\"),
            '|' => out.push_str("\\|"),
            '<' => out.push_str("&lt;"),
            '>' => out.push_str("&gt;"),
            '`' => out.push_str("\\`"),
            '\n' | '\r' | '\t' => out.push(' '),
            c => out.push(c),
        }
    }
    out
}

impl AuditReport {
    pub fn validate(&self, engine: &MockPostExploit) -> Result<(), PostExploitError> {
        if self.schema_version != REPORT_SCHEMA_VERSION {
            return Err(PostExploitError::ReportSchemaInvalid(format!(
                "schema_version '{}' khác '{}'",
                self.schema_version, REPORT_SCHEMA_VERSION
            )));
        }
        if self.engagement_id != engine.scope.engagement_id
            || !is_valid_iso8601_utc(&self.generated_at)
            || self.generated_at != engine.audit_time
            || self.input_hash.trim().is_empty()
        {
            return Err(PostExploitError::ReportSchemaInvalid(
                "engagement_id, generated_at hoặc input_hash không hợp lệ".to_string(),
            ));
        }

        let mut seen_ids = BTreeSet::new();
        for f in &self.findings {
            if !seen_ids.insert(&f.finding_id) {
                return Err(PostExploitError::DuplicateIdentifier(format!(
                    "trùng finding_id '{}'",
                    f.finding_id
                )));
            }
            f.validate_invariants(engine.evidence_store(), engine.telemetry_events())?;
        }

        for exfil in &self.exfil_assessments {
            if !engine.evidence_store().contains_key(&exfil.evidence_id) {
                return Err(PostExploitError::MissingEvidence(format!(
                    "exfil assessment '{}' thiếu evidence '{}'",
                    exfil.flow_id, exfil.evidence_id
                )));
            }
            for reason in &exfil.reasons {
                validate_no_raw_secret(reason, None)?;
            }
            if let Some(sev) = exfil.risk.as_severity()
                && sev > max_severity_for_impact(exfil.impact)
            {
                return Err(PostExploitError::SeverityExceedsImpact {
                    severity: sev,
                    impact: exfil.impact,
                });
            }
        }
        for blocked in &self.lateral.blocked_transitions {
            validate_no_raw_secret(&blocked.detail, None)?;
        }

        if normalize_example_domain(&self.lateral.start_host)? != self.lateral.start_host {
            return Err(PostExploitError::InvalidDomain(format!(
                "lateral.start_host '{}' chưa chuẩn hóa",
                self.lateral.start_host
            )));
        }

        let expected =
            engine.collect_audit_components(&self.lateral.start_host, self.lateral.max_depth)?;
        if self.input_hash != expected.input_hash {
            return Err(PostExploitError::ReportSchemaInvalid(
                "input_hash không khớp kết quả tính toán của engine".to_string(),
            ));
        }
        if self.diffs != expected.diffs {
            return Err(PostExploitError::ReportSchemaInvalid(
                "diffs trong báo cáo không khớp kết quả diff_all_hosts của engine".to_string(),
            ));
        }
        if self.lateral != expected.lateral {
            return Err(PostExploitError::ReportSchemaInvalid(
                "đồ thị lateral trong báo cáo không khớp kết quả phân tích của engine".to_string(),
            ));
        }
        if self.exfil_assessments != expected.exfil_assessments {
            return Err(PostExploitError::ReportSchemaInvalid(
                "exfil_assessments không khớp kết quả phân loại của engine".to_string(),
            ));
        }
        if self.findings != expected.findings {
            return Err(PostExploitError::ReportSchemaInvalid(
                "danh sách findings không khớp toàn vẹn với kết quả kiểm toán của engine"
                    .to_string(),
            ));
        }
        Ok(())
    }

    pub fn render_json(&self) -> String {
        use std::fmt::Write;
        let mut out = String::new();
        let _ = writeln!(out, "{{");
        let _ = writeln!(
            out,
            "  \"schema_version\": \"{}\",",
            json_escape(&self.schema_version)
        );
        let _ = writeln!(
            out,
            "  \"engagement_id\": \"{}\",",
            json_escape(&self.engagement_id)
        );
        let _ = writeln!(
            out,
            "  \"generated_at\": \"{}\",",
            json_escape(&self.generated_at)
        );
        let _ = writeln!(
            out,
            "  \"input_hash\": \"{}\",",
            json_escape(&self.input_hash)
        );
        let _ = writeln!(out, "  \"findings\": [");
        for (i, f) in self.findings.iter().enumerate() {
            let evs: Vec<String> = f
                .evidence_ids
                .iter()
                .map(|e| format!("\"{}\"", json_escape(e)))
                .collect();
            let comma = if i + 1 < self.findings.len() { "," } else { "" };
            let _ = writeln!(
                out,
                "    {{\"id\": \"{}\", \"host\": \"{}\", \"severity\": \"{:?}\", \"impact\": \"{:?}\", \"title\": \"{}\", \"evidence_ids\": [{}]}}{comma}",
                json_escape(&f.finding_id),
                json_escape(&f.host),
                f.severity,
                f.impact,
                json_escape(&f.title),
                evs.join(", ")
            );
        }
        let _ = writeln!(out, "  ],");
        let _ = writeln!(
            out,
            "  \"reachable_lateral_paths\": {},",
            self.lateral.reachable_paths.len()
        );
        let _ = writeln!(
            out,
            "  \"blocked_lateral_transitions\": {}",
            self.lateral.blocked_transitions.len()
        );
        let _ = writeln!(out, "}}");
        out
    }

    pub fn render_markdown(&self) -> String {
        use std::fmt::Write;
        let mut out = String::new();
        let _ = writeln!(
            out,
            "# Post-Exploitation & Persistence Audit ({})",
            markdown_escape(&self.engagement_id)
        );
        let _ = writeln!(out, "- Schema: {}", markdown_escape(&self.schema_version));
        let _ = writeln!(
            out,
            "- Generated At: {}",
            markdown_escape(&self.generated_at)
        );
        let _ = writeln!(out, "- Input Hash: {}", markdown_escape(&self.input_hash));

        let _ = writeln!(out, "\n## 1. Findings\n");
        let _ = writeln!(out, "| ID | Host | Category | Severity | Impact | Title |");
        let _ = writeln!(out, "|---|---|---|---|---|---|");
        for f in &self.findings {
            let _ = writeln!(
                out,
                "| {} | {} | {:?} | {:?} | {:?} | {} |",
                markdown_escape(&f.finding_id),
                markdown_escape(&f.host),
                f.category,
                f.severity,
                f.impact,
                markdown_escape(&f.title)
            );
        }
        let _ = writeln!(
            out,
            "\n## 2. Lateral Paths ({} reachable, {} blocked)\n",
            self.lateral.reachable_paths.len(),
            self.lateral.blocked_transitions.len()
        );
        for b in &self.lateral.blocked_transitions {
            let _ = writeln!(
                out,
                "- BLOCKED {} ({} -> {}): `{:?}` — {}",
                markdown_escape(&b.edge_id),
                markdown_escape(&b.from_host),
                markdown_escape(&b.to_host),
                b.reason,
                markdown_escape(&b.detail)
            );
        }
        out
    }
}

fn main() {
    let mut engine = MockPostExploit::standard();
    let harvest_guard = engine
        .scope
        .request_harvesting(HarvestingAction::LsassDump)
        .unwrap_err();
    let report = engine
        .run_audit("win-srv01.corp.example", 4)
        .expect("audit ch110 chuẩn phải thành công");

    let persistence_count = report
        .findings
        .iter()
        .filter(|f| f.category == FindingCategory::Persistence)
        .count();
    let policy_count = report
        .findings
        .iter()
        .filter(|f| f.category == FindingCategory::SecurityPolicyTamper)
        .count();
    let telgap_count = report
        .findings
        .iter()
        .filter(|f| f.category == FindingCategory::TelemetryGap)
        .count();
    let exfil_alert_count = report
        .findings
        .iter()
        .filter(|f| f.category == FindingCategory::ExfilRisk)
        .count();

    let plan = engine
        .build_cleanup_plan("plan-standard-01")
        .expect("build_cleanup_plan phải thành công");
    let first_cleanup = engine
        .apply_cleanup(&plan)
        .expect("apply_cleanup lần 1 phải thành công");
    let second_cleanup = engine
        .apply_cleanup(&plan)
        .expect("apply_cleanup lần 2 phải lũy đẳng");

    let engine2 = MockPostExploit::standard();
    let report2 = engine2
        .run_audit("win-srv01.corp.example", 4)
        .expect("audit lặp phải thành công");
    let deterministic = report.render_json() == report2.render_json()
        && report.render_markdown() == report2.render_markdown();

    println!(
        "MockPostExploit — 100% synthetic, read-only snapshot audit, std-only, không chạm host/OS"
    );
    println!("Credential guard: blocked({harvest_guard})");
    println!(
        "Hosts audited: {}; Findings: {} (Persistence={persistence_count}, PolicyTamper={policy_count}, TelemetryGap={telgap_count}, ExfilRisk={exfil_alert_count})",
        report.diffs.len(),
        report.findings.len()
    );
    println!(
        "Lateral paths: reachable={}; blocked={}",
        report.lateral.reachable_paths.len(),
        report.lateral.blocked_transitions.len()
    );
    println!(
        "Cleanup pass 1: mutations={}; pass 2: mutations={}, already_absent={}, verify_clean={}",
        first_cleanup.applied_mutations,
        second_cleanup.applied_mutations,
        second_cleanup.already_absent_steps,
        second_cleanup.verification.clean
    );
    println!("Deterministic replay byte-identical: {deterministic}");
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn sha256_matches_nist_vectors() {
        assert_eq!(
            sha256_hex(b""),
            "e3b0c44298fc1c149afbf4c8996fb92427ae41e4649b934ca495991b7852b855"
        );
        assert_eq!(
            sha256_hex(b"abc"),
            "ba7816bf8f01cfea414140de5dae2223b00361a396177a9cb410ff61f20015ad"
        );
    }

    #[test]
    fn domain_normalization_and_rfc1918_boundaries_are_enforced() {
        assert_eq!(
            normalize_example_domain("WIN-SRV01.CORP.EXAMPLE.").unwrap(),
            "win-srv01.corp.example"
        );
        assert!(matches!(
            normalize_example_domain("evil.com"),
            Err(PostExploitError::InvalidDomain(_))
        ));
        assert!(matches!(
            normalize_example_domain("bad..corp.example"),
            Err(PostExploitError::InvalidDomain(_))
        ));
        assert!(is_rfc1918_ipv4("10.0.0.1"));
        assert!(is_rfc1918_ipv4("172.16.0.1"));
        assert!(is_rfc1918_ipv4("172.31.255.254"));
        assert!(!is_rfc1918_ipv4("172.15.255.255"));
        assert!(!is_rfc1918_ipv4("172.32.0.1"));
        assert!(!is_rfc1918_ipv4("010.0.0.1"));
        assert!(!is_rfc1918_ipv4("8.8.8.8"));
    }

    #[test]
    fn virtual_location_rejects_os_paths_traversal_and_shell_chars() {
        assert_eq!(
            normalize_virtual_location("virtual://WIN-SRV01.CORP.EXAMPLE/Tasks/Updater").unwrap(),
            "virtual://win-srv01.corp.example/tasks/updater"
        );
        for bad in [
            "C:\\Windows\\System32\\cmd.exe",
            "/etc/cron.d/backdoor",
            "virtual://win-srv01.corp.example/tasks/../secret",
            "virtual://win-srv01.corp.example/tasks/bad;cmd",
            "virtual://win-srv01.corp.example/tasks/one space",
            "virtual://host.com/tasks/item",
        ] {
            assert!(
                normalize_virtual_location(bad).is_err(),
                "lẽ ra phải từ chối location '{bad}'"
            );
        }
    }

    #[test]
    fn iso8601_validator_handles_leap_years_and_bounds() {
        assert!(is_valid_iso8601_utc("2024-02-29T23:59:59Z"));
        assert!(!is_valid_iso8601_utc("2025-02-29T00:00:00Z"));
        assert!(!is_valid_iso8601_utc("2026-01-10T24:00:00Z"));
        assert!(!is_valid_iso8601_utc("2026-01-10 00:00:00Z"));
    }

    #[test]
    fn raw_secret_and_credential_harvesting_are_blocked() {
        let scope = AuditScope::standard();
        assert!(matches!(
            scope.request_harvesting(HarvestingAction::LsassDump),
            Err(PostExploitError::CredentialHarvestingProhibited(_))
        ));
        assert!(matches!(
            scope.request_harvesting(HarvestingAction::ShadowRead),
            Err(PostExploitError::CredentialHarvestingProhibited(_))
        ));
        assert!(matches!(
            validate_no_raw_secret("bearer abc-secret-token", None),
            Err(PostExploitError::RawSecretProhibited(_))
        ));
        assert!(matches!(
            validate_no_raw_secret("-----BEGIN OPENSSH PRIVATE KEY-----", None),
            Err(PostExploitError::RawSecretProhibited(_))
        ));
        assert!(validate_no_raw_secret(REDACTED_CREDENTIAL_MARKER, Some("api_key")).is_ok());
        assert!(matches!(
            validate_no_raw_secret("real-api-key-val", Some("api_key")),
            Err(PostExploitError::RawSecretProhibited(_))
        ));
    }

    #[test]
    fn identical_snapshots_produce_clean_diff_and_zero_findings() {
        let engine = MockPostExploit::standard();
        let base_win = engine
            .baseline_snapshots()
            .get("win-srv01.corp.example")
            .unwrap();
        let diff = diff_snapshots(base_win, base_win, engine.evidence_store()).unwrap();
        assert!(diff.is_clean());
    }

    #[test]
    fn diff_rejects_mismatched_host_platform_earlier_timestamp_and_missing_evidence() {
        let engine = MockPostExploit::standard();
        let base_win = engine
            .baseline_snapshots()
            .get("win-srv01.corp.example")
            .unwrap()
            .clone();
        let cur_lin = engine
            .current_snapshots()
            .get("linux-app01.corp.example")
            .unwrap()
            .clone();
        assert!(matches!(
            diff_snapshots(&base_win, &cur_lin, engine.evidence_store()),
            Err(PostExploitError::SnapshotMismatch(_))
        ));

        let mut cur_win = engine
            .current_snapshots()
            .get("win-srv01.corp.example")
            .unwrap()
            .clone();
        cur_win.captured_at = "2026-01-09T23:59:59Z".to_string();
        assert!(matches!(
            diff_snapshots(&base_win, &cur_win, engine.evidence_store()),
            Err(PostExploitError::InvalidTimestamp(_))
        ));

        let mut no_ev_cur = engine
            .current_snapshots()
            .get("win-srv01.corp.example")
            .unwrap()
            .clone();
        no_ev_cur
            .persistence_records
            .get_mut("task:updater-sync")
            .unwrap()
            .evidence_id = "ev:does-not-exist".to_string();
        assert!(matches!(
            diff_snapshots(&base_win, &no_ev_cur, engine.evidence_store()),
            Err(PostExploitError::MissingEvidence(_))
        ));
    }

    #[test]
    fn persistence_and_finding_enforce_severity_impact_cap() {
        let engine = MockPostExploit::standard();
        let mut report = engine.run_audit("win-srv01.corp.example", 4).unwrap();
        report.validate(&engine).unwrap();

        let mut escalated = report.clone();
        escalated.findings[0].impact = ImpactLevel::Low;
        escalated.findings[0].severity = Severity::High;
        assert!(matches!(
            escalated.validate(&engine),
            Err(PostExploitError::SeverityExceedsImpact { .. })
        ));

        report.findings[0].evidence_ids.clear();
        assert!(matches!(
            report.validate(&engine),
            Err(PostExploitError::MissingEvidence(_))
        ));
    }

    #[test]
    fn lateral_dfs_finds_reachable_paths_and_blocks_cycles_scope_segmented_expired_and_lowpriv() {
        let engine = MockPostExploit::standard();
        let lat = engine
            .analyze_lateral_paths("win-srv01.corp.example", 4)
            .unwrap();

        assert!(lat.reachable_paths.iter().any(|p| p.hops
            == [
                "win-srv01.corp.example",
                "linux-app01.corp.example",
                "files-int.corp.example"
            ]));

        let reasons: BTreeSet<LateralBlockReason> =
            lat.blocked_transitions.iter().map(|b| b.reason).collect();
        assert!(reasons.contains(&LateralBlockReason::CycleDetected));
        assert!(reasons.contains(&LateralBlockReason::OutOfScope));
        assert!(reasons.contains(&LateralBlockReason::NetworkSegmented));
        assert!(reasons.contains(&LateralBlockReason::SessionExpired));
        assert!(reasons.contains(&LateralBlockReason::InsufficientPrivilege));

        let shallow = engine
            .analyze_lateral_paths("win-srv01.corp.example", 1)
            .unwrap();
        assert!(
            shallow
                .blocked_transitions
                .iter()
                .any(|b| b.reason == LateralBlockReason::MaxDepthReached)
        );
    }

    #[test]
    fn policy_tamper_creates_telemetry_gap_and_attaches_gap_note_to_persistence() {
        let engine = MockPostExploit::standard();
        let report = engine.run_audit("win-srv01.corp.example", 4).unwrap();

        let task_finding = report
            .findings
            .iter()
            .find(|f| f.finding_id == "find:win-srv01.corp.example:task:updater-sync")
            .unwrap();
        assert!(task_finding.detection_gap.is_some());

        assert!(report.findings.iter().any(|f| {
            f.category == FindingCategory::TelemetryGap
                && f.host == "win-srv01.corp.example"
                && f.title.contains("TaskScheduler")
        }));
    }

    #[test]
    fn data_flow_classification_distinguishes_benign_dns_critical_and_public_cap() {
        let engine = MockPostExploit::standard();
        let flows: BTreeMap<&str, ExfilAssessment> = engine
            .run_audit("win-srv01.corp.example", 4)
            .unwrap()
            .exfil_assessments
            .into_iter()
            .map(|a| (Box::leak(a.flow_id.clone().into_boxed_str()) as &str, a))
            .collect();

        assert_eq!(flows["flow:backup-ok"].risk, ExfilRiskLevel::Benign);
        assert_eq!(flows["flow:dns-beacon"].risk, ExfilRiskLevel::High);
        assert_eq!(flows["flow:exfil-staging"].risk, ExfilRiskLevel::Critical);
        assert_eq!(flows["flow:public-check"].risk, ExfilRiskLevel::Low);
        assert_eq!(flows["flow:public-check"].impact, ImpactLevel::Low);

        let unauth_backup = DataFlow::new(
            "flow:bad-backup",
            "linux-app01.corp.example",
            "rogue-backup.corp.example",
            DestinationCategory::AuthorizedBackup,
            FlowProtocol::SftpMeta,
            100_000,
            10,
            DataSensitivity::Confidential,
            false,
            "ev:flow-backup",
        );
        let res =
            classify_data_flow(&engine.scope, &unauth_backup, engine.evidence_store()).unwrap();
        assert_eq!(res.risk, ExfilRiskLevel::High);
    }

    #[test]
    fn cleanup_is_idempotent_handles_missing_artifact_and_protects_baseline() {
        let mut engine = MockPostExploit::standard();
        let before_baseline = engine.baseline_snapshots().clone();
        let before_current = engine.current_snapshots().clone();

        let pre_verify = engine.verify_baseline();
        assert!(!pre_verify.clean);
        assert!(!pre_verify.residual_drifts.is_empty());

        let bad_plan = CleanupPlan {
            plan_id: "plan-bad-baseline".to_string(),
            engagement_id: ENGAGEMENT_ID.to_string(),
            steps: vec![
                CleanupStep::RemovePersistence {
                    host: "win-srv01.corp.example".to_string(),
                    record_id: "task:updater-sync".to_string(),
                },
                CleanupStep::RemoveArtifact {
                    host: "win-srv01.corp.example".to_string(),
                    artifact_id: "art:win-baseline-agent".to_string(),
                    expected_sha256: None,
                },
            ],
        };
        assert!(matches!(
            engine.apply_cleanup(&bad_plan),
            Err(PostExploitError::BaselineProtected(_))
        ));
        // Lỗi giữa chừng không được commit phần đầu của bad_plan.
        assert!(!engine.verify_baseline().clean);
        assert!(
            engine
                .cleanup_state
                .snapshots
                .get("win-srv01.corp.example")
                .unwrap()
                .persistence_records
                .contains_key("task:updater-sync")
        );

        let mismatch_plan = CleanupPlan {
            plan_id: "plan-digest-mismatch".to_string(),
            engagement_id: ENGAGEMENT_ID.to_string(),
            steps: vec![CleanupStep::RemoveArtifact {
                host: "win-srv01.corp.example".to_string(),
                artifact_id: "art:win-helper-bin".to_string(),
                expected_sha256: Some("0".repeat(64)),
            }],
        };
        assert!(matches!(
            engine.apply_cleanup(&mismatch_plan),
            Err(PostExploitError::ArtifactDigestMismatch { .. })
        ));

        let mut plan = engine.build_cleanup_plan("plan-full").unwrap();
        // Thêm bước trỏ tới artifact đã không tồn tại từ đầu để kiểm tra nhánh AlreadyAbsent.
        plan.steps.push(CleanupStep::RemoveArtifact {
            host: "linux-app01.corp.example".to_string(),
            artifact_id: "art:already-deleted-elsewhere".to_string(),
            expected_sha256: None,
        });

        let first = engine.apply_cleanup(&plan).unwrap();
        assert!(
            first.verification.clean,
            "còn drift: {:?}",
            first.verification.residual_drifts
        );
        assert_eq!(first.already_absent_steps, 1);

        let state_after_first = engine.cleanup_state.clone();
        let second = engine.apply_cleanup(&plan).unwrap();
        assert_eq!(second.applied_mutations, 0);
        assert_eq!(second.already_absent_steps, plan.steps.len());
        assert!(second.verification.clean);
        assert_eq!(engine.cleanup_state, state_after_first);

        // Read-only snapshots gốc hoàn toàn không đổi.
        assert_eq!(engine.baseline_snapshots(), &before_baseline);
        assert_eq!(engine.current_snapshots(), &before_current);
    }

    #[test]
    fn verify_baseline_catches_single_leftover_artifact() {
        let mut engine = MockPostExploit::standard();
        let mut partial_plan = engine.build_cleanup_plan("plan-partial").unwrap();
        partial_plan.steps.retain(|step| {
            !matches!(
                step,
                CleanupStep::RemoveArtifact {
                    artifact_id,
                    ..
                } if artifact_id == "art:lin-cron-sh"
            )
        });
        let exec = engine.apply_cleanup(&partial_plan).unwrap();
        assert!(!exec.verification.clean);
        assert!(
            exec.verification
                .residual_drifts
                .iter()
                .any(|d| d.contains("art:lin-cron-sh"))
        );
    }

    #[test]
    fn report_render_escapes_special_chars_and_replays_identically() {
        let e1 = MockPostExploit::standard();
        let e2 = MockPostExploit::standard();
        let r1 = e1.run_audit("win-srv01.corp.example", 4).unwrap();
        let r2 = e2.run_audit("win-srv01.corp.example", 4).unwrap();
        assert_eq!(r1.render_json(), r2.render_json());
        assert_eq!(r1.render_markdown(), r2.render_markdown());

        assert_eq!(markdown_escape(r"a\|b<script>"), r"a\\\|b&lt;script&gt;");
        assert_eq!(json_escape("a\\b\n\"c\""), "a\\\\b\\n\\\"c\\\"");
    }

    #[test]
    fn scope_validate_rejects_empty_engagement_and_disallowed_flags() {
        let mut s1 = AuditScope::standard();
        s1.engagement_id = "  ".to_string();
        assert!(matches!(
            s1.validate(),
            Err(PostExploitError::OutOfScope(_))
        ));

        let mut s2 = AuditScope::standard();
        s2.allow_credential_harvesting = true;
        assert!(matches!(
            s2.validate(),
            Err(PostExploitError::CredentialHarvestingProhibited(_))
        ));

        let mut s3 = AuditScope::standard();
        s3.allow_real_os_mutation = true;
        assert!(matches!(
            s3.validate(),
            Err(PostExploitError::InvalidLocation(_))
        ));
    }

    #[test]
    fn scope_validate_rejects_unnormalized_hosts_or_empty_allowlist() {
        let mut s1 = AuditScope::standard();
        s1.allowed_hosts.clear();
        assert!(matches!(
            s1.validate(),
            Err(PostExploitError::OutOfScope(_))
        ));

        let mut s2 = AuditScope::standard();
        s2.allowed_hosts.insert("BAD.CORP.EXAMPLE.".to_string());
        assert!(matches!(
            s2.validate(),
            Err(PostExploitError::InvalidDomain(_))
        ));

        let mut s3 = AuditScope::standard();
        s3.authorized_backup_destinations
            .insert("BACKUP.CORP.EXAMPLE".to_string());
        assert!(matches!(
            s3.validate(),
            Err(PostExploitError::InvalidDomain(_))
        ));
    }

    #[test]
    fn snapshot_validate_rejects_mismatched_keys_and_hosts() {
        let engine = MockPostExploit::standard();
        let mut bad_p = engine
            .current_snapshots()
            .get("win-srv01.corp.example")
            .unwrap()
            .clone();
        bad_p
            .persistence_records
            .get_mut("task:updater-sync")
            .unwrap()
            .host = "linux-app01.corp.example".to_string();
        assert!(matches!(
            bad_p.validate(engine.evidence_store()),
            Err(PostExploitError::SnapshotMismatch(_))
        ));

        let mut bad_art = engine
            .current_snapshots()
            .get("win-srv01.corp.example")
            .unwrap()
            .clone();
        let art = bad_art.artifacts.get("art:win-helper-bin").unwrap().clone();
        bad_art.artifacts.insert("art:wrong-key".to_string(), art);
        assert!(matches!(
            bad_art.validate(engine.evidence_store()),
            Err(PostExploitError::SnapshotMismatch(_))
        ));
    }

    #[test]
    fn snapshot_validate_rejects_unnormalized_location_and_bad_digest() {
        let engine = MockPostExploit::standard();
        let mut bad_loc = engine
            .current_snapshots()
            .get("win-srv01.corp.example")
            .unwrap()
            .clone();
        bad_loc
            .persistence_records
            .get_mut("task:updater-sync")
            .unwrap()
            .virtual_location = "virtual://WIN-SRV01.CORP.EXAMPLE/tasks/updater-sync".to_string();
        assert!(matches!(
            bad_loc.validate(engine.evidence_store()),
            Err(PostExploitError::InvalidLocation(_))
        ));

        let mut bad_digest = engine
            .current_snapshots()
            .get("win-srv01.corp.example")
            .unwrap()
            .clone();
        bad_digest
            .artifacts
            .get_mut("art:win-helper-bin")
            .unwrap()
            .sha256_digest = "not-64-hex".to_string();
        assert!(matches!(
            bad_digest.validate(engine.evidence_store()),
            Err(PostExploitError::ValidationFailed(_))
        ));
    }

    #[test]
    fn snapshot_validate_rejects_raw_secret_in_descriptor_or_policy() {
        let engine = MockPostExploit::standard();
        let mut bad_desc = engine
            .current_snapshots()
            .get("win-srv01.corp.example")
            .unwrap()
            .clone();
        bad_desc
            .persistence_records
            .get_mut("task:updater-sync")
            .unwrap()
            .target_descriptor = "cmd password=supersecret".to_string();
        assert!(matches!(
            bad_desc.validate(engine.evidence_store()),
            Err(PostExploitError::RawSecretProhibited(_))
        ));

        let mut bad_pol = engine
            .current_snapshots()
            .get("win-srv01.corp.example")
            .unwrap()
            .clone();
        bad_pol.security_policies.insert(
            "pol:secret".to_string(),
            SecurityPolicyRecord {
                policy_id: "pol:secret".to_string(),
                setting: "ServiceTokenPolicy".to_string(),
                secure_value: REDACTED_CREDENTIAL_MARKER.to_string(),
                current_value: "raw-unredacted-val".to_string(),
                disables_channel: None,
                evidence_id: "ev:win-pol".to_string(),
            },
        );
        assert!(matches!(
            bad_pol.validate(engine.evidence_store()),
            Err(PostExploitError::RawSecretProhibited(_))
        ));
    }

    #[test]
    fn snapshot_validate_rejects_missing_evidence_on_account_policy_artifact() {
        let engine = MockPostExploit::standard();
        let base = engine
            .current_snapshots()
            .get("win-srv01.corp.example")
            .unwrap()
            .clone();

        let mut bad_acct = base.clone();
        bad_acct
            .accounts
            .get_mut("svc_maintenance")
            .unwrap()
            .evidence_id = "ev:missing".to_string();
        assert!(matches!(
            bad_acct.validate(engine.evidence_store()),
            Err(PostExploitError::MissingEvidence(_))
        ));

        let mut bad_pol = base.clone();
        bad_pol
            .security_policies
            .get_mut("pol:task-audit")
            .unwrap()
            .evidence_id = "".to_string();
        assert!(matches!(
            bad_pol.validate(engine.evidence_store()),
            Err(PostExploitError::MissingEvidence(_))
        ));

        let mut bad_art = base;
        bad_art
            .artifacts
            .get_mut("art:win-helper-bin")
            .unwrap()
            .evidence_id = "ev:nope".to_string();
        assert!(matches!(
            bad_art.validate(engine.evidence_store()),
            Err(PostExploitError::MissingEvidence(_))
        ));
    }

    #[test]
    fn diff_all_hosts_captures_persistence_account_policy_and_artifacts() {
        let engine = MockPostExploit::standard();
        let diffs = engine.diff_all_hosts().unwrap();
        let win = diffs
            .iter()
            .find(|d| d.host == "win-srv01.corp.example")
            .unwrap();
        assert_eq!(win.added_persistence.len(), 2);
        assert_eq!(win.modified_accounts.len(), 1);
        assert_eq!(win.policy_drifts.len(), 1);
        assert_eq!(win.added_artifacts.len(), 2);

        let lin = diffs
            .iter()
            .find(|d| d.host == "linux-app01.corp.example")
            .unwrap();
        assert_eq!(lin.added_persistence.len(), 2);
        assert_eq!(lin.added_artifacts.len(), 2);
    }

    #[test]
    fn run_audit_detects_modified_persistence_removed_persistence_and_added_account() {
        let base_eng = MockPostExploit::standard();
        let mut cur_snaps = base_eng.current_snapshots().clone();
        let win = cur_snaps.get_mut("win-srv01.corp.example").unwrap();
        win.persistence_records.remove("svc:baseline-monitoring");
        win.accounts.insert(
            "backdoor_op".to_string(),
            AccountState {
                account_id: "backdoor_op".to_string(),
                enabled: true,
                privilege: PrivilegeLevel::LocalAdmin,
                groups: BTreeSet::from(["Administrators".to_string()]),
                evidence_id: "ev:win-acct".to_string(),
            },
        );
        let lin = cur_snaps.get_mut("linux-app01.corp.example").unwrap();
        lin.persistence_records.insert(
            "svc:telemetry-forwarder".to_string(),
            PersistenceRecord::new(
                "svc:telemetry-forwarder",
                "linux-app01.corp.example",
                PersistenceMechanism::Service,
                "virtual://linux-app01.corp.example/systemd/telemetry-forwarder.service",
                "root",
                PrivilegeLevel::SystemOrRoot,
                "synthetic://modified-daemon",
                true,
                false,
                Some("art:lin-svc-script"),
                "ev:lin-svc",
            ),
        );

        let mut mod_base = base_eng.baseline_snapshots().clone();
        mod_base
            .get_mut("linux-app01.corp.example")
            .unwrap()
            .persistence_records
            .insert(
                "svc:telemetry-forwarder".to_string(),
                PersistenceRecord::new(
                    "svc:telemetry-forwarder",
                    "linux-app01.corp.example",
                    PersistenceMechanism::Service,
                    "virtual://linux-app01.corp.example/systemd/telemetry-forwarder.service",
                    "root",
                    PrivilegeLevel::ServiceAccount,
                    "synthetic://orig-daemon",
                    false,
                    true,
                    None,
                    "ev:base-lin",
                ),
            );

        let custom = MockPostExploit::new(
            base_eng.scope.clone(),
            AUDIT_TIMESTAMP,
            mod_base,
            cur_snaps,
            base_eng.evidence_store().clone(),
            base_eng.access_grants().clone(),
            base_eng.lateral_edges().to_vec(),
            base_eng.telemetry_events().clone(),
            base_eng.data_flows().to_vec(),
        )
        .unwrap();
        let report = custom.run_audit("win-srv01.corp.example", 4).unwrap();
        assert!(
            report
                .findings
                .iter()
                .any(|f| f.finding_id
                    == "find:win-srv01.corp.example:removed:svc:baseline-monitoring")
        );
        assert!(
            report
                .findings
                .iter()
                .any(|f| f.finding_id == "find:win-srv01.corp.example:acct_added:backdoor_op")
        );
        assert!(
            report.findings.iter().any(|f| f.finding_id
                == "find:linux-app01.corp.example:modified:svc:telemetry-forwarder")
        );
    }

    #[test]
    fn run_audit_detects_standalone_orphan_artifact_without_persistence() {
        let base_eng = MockPostExploit::standard();
        let mut cur_snaps = base_eng.current_snapshots().clone();
        cur_snaps
            .get_mut("files-int.corp.example")
            .unwrap()
            .artifacts
            .insert(
                "art:orphan-archive".to_string(),
                Artifact::new(
                    "art:orphan-archive",
                    "files-int.corp.example",
                    "virtual://files-int.corp.example/tmp/staging.tar",
                    ArtifactKind::StagingArchive,
                    "SYNTHETIC-STAGING-BUNDLE",
                    "appuser",
                    true,
                    "ev:lin-art3",
                ),
            );
        let custom = MockPostExploit::new(
            base_eng.scope.clone(),
            AUDIT_TIMESTAMP,
            base_eng.baseline_snapshots().clone(),
            cur_snaps,
            base_eng.evidence_store().clone(),
            base_eng.access_grants().clone(),
            base_eng.lateral_edges().to_vec(),
            base_eng.telemetry_events().clone(),
            base_eng.data_flows().to_vec(),
        )
        .unwrap();
        let report = custom.run_audit("win-srv01.corp.example", 4).unwrap();
        assert!(
            report
                .findings
                .iter()
                .any(|f| f.finding_id == "find:files-int.corp.example:art:art:orphan-archive")
        );
    }

    #[test]
    fn run_audit_does_not_mutate_baseline_or_current_snapshots() {
        let engine = MockPostExploit::standard();
        let before_base = engine.baseline_snapshots().clone();
        let before_cur = engine.current_snapshots().clone();
        let _ = engine.run_audit("win-srv01.corp.example", 4).unwrap();
        assert_eq!(engine.baseline_snapshots(), &before_base);
        assert_eq!(engine.current_snapshots(), &before_cur);
    }

    #[test]
    fn rank_persistence_findings_orders_severity_impact_then_id() {
        let engine = MockPostExploit::standard();
        let report = engine.run_audit("win-srv01.corp.example", 4).unwrap();
        let ranked = rank_persistence_findings(&report.findings);
        assert_eq!(ranked.len(), 4);
        for pair in ranked.windows(2) {
            assert!(pair[0].severity >= pair[1].severity);
            if pair[0].severity == pair[1].severity {
                assert!(pair[0].impact >= pair[1].impact);
            }
        }
        assert_eq!(ranked.last().unwrap().severity, Severity::Medium);
    }

    #[test]
    fn max_severity_for_impact_enforces_none_to_critical_mapping() {
        assert_eq!(max_severity_for_impact(ImpactLevel::None), Severity::Low);
        assert_eq!(max_severity_for_impact(ImpactLevel::Low), Severity::Low);
        assert_eq!(
            max_severity_for_impact(ImpactLevel::Medium),
            Severity::Medium
        );
        assert_eq!(max_severity_for_impact(ImpactLevel::High), Severity::High);
        assert_eq!(
            max_severity_for_impact(ImpactLevel::Critical),
            Severity::Critical
        );
    }

    #[test]
    fn finding_validation_rejects_missing_preconditions_or_remediation() {
        let engine = MockPostExploit::standard();
        let report = engine.run_audit("win-srv01.corp.example", 4).unwrap();

        let mut no_pre = report.findings[0].clone();
        no_pre.preconditions.clear();
        assert!(matches!(
            no_pre.validate_invariants(engine.evidence_store(), engine.telemetry_events()),
            Err(PostExploitError::ValidationFailed(_))
        ));

        let mut no_rem = report.findings[0].clone();
        no_rem.remediation = " ".to_string();
        assert!(matches!(
            no_rem.validate_invariants(engine.evidence_store(), engine.telemetry_events()),
            Err(PostExploitError::ValidationFailed(_))
        ));
    }

    #[test]
    fn finding_validation_rejects_mismatched_or_fake_telemetry_id() {
        let engine = MockPostExploit::standard();
        let report = engine.run_audit("win-srv01.corp.example", 4).unwrap();

        let mut fake_tel = report.findings[0].clone();
        fake_tel
            .telemetry_ids
            .insert("tel:does-not-exist".to_string());
        assert!(matches!(
            fake_tel.validate_invariants(engine.evidence_store(), engine.telemetry_events()),
            Err(PostExploitError::ValidationFailed(_))
        ));

        let mut wrong_host_tel = report
            .findings
            .iter()
            .find(|f| f.host == "linux-app01.corp.example")
            .unwrap()
            .clone();
        wrong_host_tel
            .telemetry_ids
            .insert("tel:win-acct".to_string());
        assert!(matches!(
            wrong_host_tel.validate_invariants(engine.evidence_store(), engine.telemetry_events()),
            Err(PostExploitError::ValidationFailed(_))
        ));
    }

    #[test]
    fn finding_validation_rejects_missing_both_telemetry_and_detection_gap() {
        let engine = MockPostExploit::standard();
        let report = engine.run_audit("win-srv01.corp.example", 4).unwrap();
        let mut f = report.findings[0].clone();
        f.telemetry_ids.clear();
        f.detection_gap = None;
        assert!(matches!(
            f.validate_invariants(engine.evidence_store(), engine.telemetry_events()),
            Err(PostExploitError::ValidationFailed(_))
        ));
    }

    #[test]
    fn finding_validation_rejects_raw_secret_in_detection_or_remediation() {
        let engine = MockPostExploit::standard();
        let report = engine.run_audit("win-srv01.corp.example", 4).unwrap();
        let mut f = report.findings[0].clone();
        f.remediation = "rotate ghp_1234567890".to_string();
        assert!(matches!(
            f.validate_invariants(engine.evidence_store(), engine.telemetry_events()),
            Err(PostExploitError::RawSecretProhibited(_))
        ));
    }

    #[test]
    fn lateral_dfs_rejects_out_of_scope_start_host() {
        let engine = MockPostExploit::standard();
        assert!(matches!(
            engine.analyze_lateral_paths("rogue.external.example", 3),
            Err(PostExploitError::OutOfScope(_))
        ));
    }

    #[test]
    fn lateral_dfs_detects_missing_access_grant_distinctly() {
        let base_eng = MockPostExploit::standard();
        let mut edges = base_eng.lateral_edges().to_vec();
        edges.push(LateralEdge {
            edge_id: "edge:win-to-files-nogrant".to_string(),
            from_host: "win-srv01.corp.example".to_string(),
            to_host: "files-int.corp.example".to_string(),
            required_access: AccessKind::InteractiveSession,
            min_privilege: PrivilegeLevel::StandardUser,
            network_reachable: true,
        });
        let custom = MockPostExploit::new(
            base_eng.scope.clone(),
            AUDIT_TIMESTAMP,
            base_eng.baseline_snapshots().clone(),
            base_eng.current_snapshots().clone(),
            base_eng.evidence_store().clone(),
            base_eng.access_grants().clone(),
            edges,
            base_eng.telemetry_events().clone(),
            base_eng.data_flows().to_vec(),
        )
        .unwrap();
        let lat = custom
            .analyze_lateral_paths("win-srv01.corp.example", 4)
            .unwrap();
        assert!(lat.blocked_transitions.iter().any(|b| {
            b.edge_id == "edge:win-to-files-nogrant"
                && b.reason == LateralBlockReason::MissingAccessGrant
        }));
    }

    #[test]
    fn lateral_dfs_respects_expiry_boundary_equality() {
        let base_eng = MockPostExploit::standard();
        let mut grants = base_eng.access_grants().clone();
        grants.get_mut("grant:win-lin-ssh").unwrap().expires_at = AUDIT_TIMESTAMP.to_string();
        let custom = MockPostExploit::new(
            base_eng.scope.clone(),
            AUDIT_TIMESTAMP,
            base_eng.baseline_snapshots().clone(),
            base_eng.current_snapshots().clone(),
            base_eng.evidence_store().clone(),
            grants,
            base_eng.lateral_edges().to_vec(),
            base_eng.telemetry_events().clone(),
            base_eng.data_flows().to_vec(),
        )
        .unwrap();
        let lat = custom
            .analyze_lateral_paths("win-srv01.corp.example", 4)
            .unwrap();
        assert!(lat.blocked_transitions.iter().any(|b| {
            b.edge_id == "edge:win-to-lin" && b.reason == LateralBlockReason::SessionExpired
        }));
    }

    #[test]
    fn data_flow_rejects_missing_evidence_public_ip_and_zero_duration() {
        let engine = MockPostExploit::standard();

        let no_ev = DataFlow::new(
            "flow:noev",
            "win-srv01.corp.example",
            "10.20.40.50",
            DestinationCategory::InternalCollector,
            FlowProtocol::HttpsMeta,
            1000,
            10,
            DataSensitivity::Internal,
            false,
            "ev:missing",
        );
        assert!(matches!(
            classify_data_flow(&engine.scope, &no_ev, engine.evidence_store()),
            Err(PostExploitError::MissingEvidence(_))
        ));

        let public_ip = DataFlow::new(
            "flow:pubip",
            "win-srv01.corp.example",
            "8.8.8.8",
            DestinationCategory::UnknownExternal,
            FlowProtocol::HttpsMeta,
            1000,
            10,
            DataSensitivity::Internal,
            false,
            "ev:flow-public",
        );
        assert!(matches!(
            classify_data_flow(&engine.scope, &public_ip, engine.evidence_store()),
            Err(PostExploitError::InvalidDomain(_))
        ));

        let zero_dur = DataFlow::new(
            "flow:zerodur",
            "win-srv01.corp.example",
            "10.20.40.50",
            DestinationCategory::InternalCollector,
            FlowProtocol::HttpsMeta,
            1000,
            0,
            DataSensitivity::Internal,
            false,
            "ev:flow-public",
        );
        assert!(matches!(
            classify_data_flow(&engine.scope, &zero_dur, engine.evidence_store()),
            Err(PostExploitError::ValidationFailed(_))
        ));
    }

    #[test]
    fn data_flow_classifies_oversized_backup_and_internal_dns_txt() {
        let engine = MockPostExploit::standard();
        let big_backup = DataFlow::new(
            "flow:big-backup",
            "linux-app01.corp.example",
            "backup01.corp.example",
            DestinationCategory::AuthorizedBackup,
            FlowProtocol::SftpMeta,
            120_000_000,
            300,
            DataSensitivity::Internal,
            false,
            "ev:flow-backup",
        );
        let res1 = classify_data_flow(&engine.scope, &big_backup, engine.evidence_store()).unwrap();
        assert_eq!(res1.risk, ExfilRiskLevel::Medium);

        let internal_dns = DataFlow::new(
            "flow:int-dns",
            "win-srv01.corp.example",
            "10.20.40.99",
            DestinationCategory::InternalCollector,
            FlowProtocol::DnsTxtMeta,
            24_000,
            60,
            DataSensitivity::Confidential,
            false,
            "ev:flow-dns",
        );
        let res2 =
            classify_data_flow(&engine.scope, &internal_dns, engine.evidence_store()).unwrap();
        assert_eq!(res2.risk, ExfilRiskLevel::High);
    }

    #[test]
    fn engine_new_rejects_baseline_artifact_marked_created_in_lab() {
        let base_eng = MockPostExploit::standard();
        let mut bases = base_eng.baseline_snapshots().clone();
        bases
            .get_mut("win-srv01.corp.example")
            .unwrap()
            .artifacts
            .get_mut("art:win-baseline-agent")
            .unwrap()
            .created_in_lab = true;
        assert!(matches!(
            MockPostExploit::new(
                base_eng.scope.clone(),
                AUDIT_TIMESTAMP,
                bases,
                base_eng.current_snapshots().clone(),
                base_eng.evidence_store().clone(),
                base_eng.access_grants().clone(),
                base_eng.lateral_edges().to_vec(),
                base_eng.telemetry_events().clone(),
                base_eng.data_flows().to_vec(),
            ),
            Err(PostExploitError::BaselineProtected(_))
        ));
    }

    #[test]
    fn engine_new_rejects_duplicate_lateral_edges_and_duplicate_flows() {
        let base_eng = MockPostExploit::standard();
        let mut dup_edges = base_eng.lateral_edges().to_vec();
        dup_edges.push(dup_edges[0].clone());
        assert!(matches!(
            MockPostExploit::new(
                base_eng.scope.clone(),
                AUDIT_TIMESTAMP,
                base_eng.baseline_snapshots().clone(),
                base_eng.current_snapshots().clone(),
                base_eng.evidence_store().clone(),
                base_eng.access_grants().clone(),
                dup_edges,
                base_eng.telemetry_events().clone(),
                base_eng.data_flows().to_vec(),
            ),
            Err(PostExploitError::DuplicateIdentifier(_))
        ));

        let mut dup_flows = base_eng.data_flows().to_vec();
        dup_flows.push(dup_flows[0].clone());
        assert!(matches!(
            MockPostExploit::new(
                base_eng.scope.clone(),
                AUDIT_TIMESTAMP,
                base_eng.baseline_snapshots().clone(),
                base_eng.current_snapshots().clone(),
                base_eng.evidence_store().clone(),
                base_eng.access_grants().clone(),
                base_eng.lateral_edges().to_vec(),
                base_eng.telemetry_events().clone(),
                dup_flows,
            ),
            Err(PostExploitError::DuplicateIdentifier(_))
        ));
    }

    #[test]
    fn apply_cleanup_rejects_empty_plan_id_and_wrong_engagement() {
        let mut engine = MockPostExploit::standard();
        let empty_id = CleanupPlan {
            plan_id: "   ".to_string(),
            engagement_id: ENGAGEMENT_ID.to_string(),
            steps: Vec::new(),
        };
        assert!(matches!(
            engine.apply_cleanup(&empty_id),
            Err(PostExploitError::ValidationFailed(_))
        ));

        let wrong_eng = CleanupPlan {
            plan_id: "p1".to_string(),
            engagement_id: "OTHER-ENG".to_string(),
            steps: Vec::new(),
        };
        assert!(matches!(
            engine.apply_cleanup(&wrong_eng),
            Err(PostExploitError::OutOfScope(_))
        ));
    }

    #[test]
    fn apply_cleanup_rejects_out_of_scope_host_and_baseline_persistence() {
        let mut engine = MockPostExploit::standard();
        let oos_plan = CleanupPlan {
            plan_id: "p-oos".to_string(),
            engagement_id: ENGAGEMENT_ID.to_string(),
            steps: vec![CleanupStep::RemovePersistence {
                host: "rogue.corp.example".to_string(),
                record_id: "task:x".to_string(),
            }],
        };
        assert!(matches!(
            engine.apply_cleanup(&oos_plan),
            Err(PostExploitError::OutOfScope(_))
        ));

        let base_pers_plan = CleanupPlan {
            plan_id: "p-base-pers".to_string(),
            engagement_id: ENGAGEMENT_ID.to_string(),
            steps: vec![CleanupStep::RemovePersistence {
                host: "win-srv01.corp.example".to_string(),
                record_id: "svc:baseline-monitoring".to_string(),
            }],
        };
        assert!(matches!(
            engine.apply_cleanup(&base_pers_plan),
            Err(PostExploitError::BaselineProtected(_))
        ));
    }

    #[test]
    fn apply_cleanup_rejects_revoking_baseline_access_grant() {
        let mut engine = MockPostExploit::standard();
        let bad_grant_plan = CleanupPlan {
            plan_id: "p-base-grant".to_string(),
            engagement_id: ENGAGEMENT_ID.to_string(),
            steps: vec![CleanupStep::RevokeAccessGrant {
                grant_id: "grant:files-vault-low".to_string(),
            }],
        };
        assert!(matches!(
            engine.apply_cleanup(&bad_grant_plan),
            Err(PostExploitError::BaselineProtected(_))
        ));
    }

    #[test]
    fn apply_cleanup_removes_standalone_lab_account_without_baseline_entry() {
        let base_eng = MockPostExploit::standard();
        let mut cur_snaps = base_eng.current_snapshots().clone();
        cur_snaps
            .get_mut("files-int.corp.example")
            .unwrap()
            .accounts
            .insert(
                "temp_audit_acct".to_string(),
                AccountState {
                    account_id: "temp_audit_acct".to_string(),
                    enabled: true,
                    privilege: PrivilegeLevel::StandardUser,
                    groups: BTreeSet::from(["Users".to_string()]),
                    evidence_id: "ev:win-acct".to_string(),
                },
            );
        let mut engine = MockPostExploit::new(
            base_eng.scope.clone(),
            AUDIT_TIMESTAMP,
            base_eng.baseline_snapshots().clone(),
            cur_snaps,
            base_eng.evidence_store().clone(),
            base_eng.access_grants().clone(),
            base_eng.lateral_edges().to_vec(),
            base_eng.telemetry_events().clone(),
            base_eng.data_flows().to_vec(),
        )
        .unwrap();

        let plan = engine.build_cleanup_plan("plan-clean-acct").unwrap();
        let res = engine.apply_cleanup(&plan).unwrap();
        assert!(res.verification.clean);
        assert!(
            !engine
                .cleanup_snapshot("files-int.corp.example")
                .unwrap()
                .accounts
                .contains_key("temp_audit_acct")
        );
    }

    #[test]
    fn apply_cleanup_restores_disabled_telemetry_control_when_policy_restored() {
        let mut engine = MockPostExploit::standard();
        assert!(
            !engine
                .cleanup_snapshot("win-srv01.corp.example")
                .unwrap()
                .telemetry_controls
                .get(&TelemetryChannel::TaskScheduler)
                .unwrap()
                .enabled
        );
        let plan = engine.build_cleanup_plan("plan-restore-tel").unwrap();
        let res = engine.apply_cleanup(&plan).unwrap();
        assert!(res.verification.clean);
        assert!(
            engine
                .cleanup_snapshot("win-srv01.corp.example")
                .unwrap()
                .telemetry_controls
                .get(&TelemetryChannel::TaskScheduler)
                .unwrap()
                .enabled
        );
    }

    #[test]
    fn verify_baseline_detects_leftover_persistence_policy_account_or_grant() {
        let mut engine = MockPostExploit::standard();
        let mut no_grant_plan = engine.build_cleanup_plan("plan-no-grant").unwrap();
        no_grant_plan
            .steps
            .retain(|s| !matches!(s, CleanupStep::RevokeAccessGrant { .. }));
        let exec = engine.apply_cleanup(&no_grant_plan).unwrap();
        assert!(!exec.verification.clean);
        assert!(
            exec.verification
                .residual_drifts
                .iter()
                .any(|d| d.contains("access grant lab"))
        );
        assert!(
            engine
                .cleanup_access_grants()
                .contains_key("grant:win-lin-ssh")
        );
    }

    #[test]
    fn report_validate_rejects_bad_schema_engagement_timestamp_and_empty_hash() {
        let engine = MockPostExploit::standard();
        let valid = engine.run_audit("win-srv01.corp.example", 4).unwrap();

        let mut bad_ver = valid.clone();
        bad_ver.schema_version = "9.9.9".to_string();
        assert!(matches!(
            bad_ver.validate(&engine),
            Err(PostExploitError::ReportSchemaInvalid(_))
        ));

        let mut bad_eng = valid.clone();
        bad_eng.engagement_id = "WRONG".to_string();
        assert!(matches!(
            bad_eng.validate(&engine),
            Err(PostExploitError::ReportSchemaInvalid(_))
        ));

        let mut bad_ts = valid.clone();
        bad_ts.generated_at = "not-a-timestamp".to_string();
        assert!(matches!(
            bad_ts.validate(&engine),
            Err(PostExploitError::ReportSchemaInvalid(_))
        ));

        let mut empty_hash = valid;
        empty_hash.input_hash = "  ".to_string();
        assert!(matches!(
            empty_hash.validate(&engine),
            Err(PostExploitError::ReportSchemaInvalid(_))
        ));
    }

    #[test]
    fn report_validate_rejects_duplicate_findings_or_dropped_finding() {
        let engine = MockPostExploit::standard();
        let valid = engine.run_audit("win-srv01.corp.example", 4).unwrap();

        let mut dup = valid.clone();
        let first = dup.findings[0].clone();
        dup.findings.push(first);
        assert!(matches!(
            dup.validate(&engine),
            Err(PostExploitError::DuplicateIdentifier(_))
        ));

        let mut dropped = valid;
        dropped.findings.pop();
        assert!(matches!(
            dropped.validate(&engine),
            Err(PostExploitError::ReportSchemaInvalid(_))
        ));
    }

    #[test]
    fn report_validate_rejects_dropped_lateral_blocked_or_tampered_exfil() {
        let engine = MockPostExploit::standard();
        let valid = engine.run_audit("win-srv01.corp.example", 4).unwrap();

        let mut bad_lat = valid.clone();
        bad_lat.lateral.blocked_transitions.clear();
        assert!(matches!(
            bad_lat.validate(&engine),
            Err(PostExploitError::ReportSchemaInvalid(_))
        ));

        let mut bad_exfil = valid;
        bad_exfil.exfil_assessments[0].evidence_id = "ev:missing".to_string();
        assert!(matches!(
            bad_exfil.validate(&engine),
            Err(PostExploitError::MissingEvidence(_))
        ));
    }

    #[test]
    fn run_audit_detects_modified_removed_artifacts_accounts_and_added_removed_policies() {
        let base_eng = MockPostExploit::standard();
        let mut bases = base_eng.baseline_snapshots().clone();
        let mut curs = base_eng.current_snapshots().clone();

        // Thêm artifact baseline độc lập trên files-int để kiểm tra removed_artifacts & modified_artifacts.
        let base_files_art = Artifact::new(
            "art:files-base-cfg",
            "files-int.corp.example",
            "virtual://files-int.corp.example/etc-mock/audit.conf",
            ArtifactKind::ModifiedConfig,
            "ORIG-AUDIT-CONF",
            "root",
            false,
            "ev:base-files",
        );
        bases
            .get_mut("files-int.corp.example")
            .unwrap()
            .artifacts
            .insert("art:files-base-cfg".to_string(), base_files_art);
        bases
            .get_mut("files-int.corp.example")
            .unwrap()
            .security_policies
            .insert(
                "pol:files-base".to_string(),
                SecurityPolicyRecord {
                    policy_id: "pol:files-base".to_string(),
                    setting: "FirewallMode".to_string(),
                    secure_value: "Strict".to_string(),
                    current_value: "Strict".to_string(),
                    disables_channel: None,
                    evidence_id: "ev:base-files".to_string(),
                },
            );
        bases
            .get_mut("files-int.corp.example")
            .unwrap()
            .accounts
            .insert(
                "audit_reader".to_string(),
                AccountState {
                    account_id: "audit_reader".to_string(),
                    enabled: true,
                    privilege: PrivilegeLevel::StandardUser,
                    groups: BTreeSet::from(["Auditors".to_string()]),
                    evidence_id: "ev:base-files".to_string(),
                },
            );

        let cur_win = curs.get_mut("win-srv01.corp.example").unwrap();
        let mod_art = Artifact::new(
            "art:win-baseline-agent",
            "win-srv01.corp.example",
            "virtual://win-srv01.corp.example/programdata/baseline-agent.bin",
            ArtifactKind::ModifiedConfig,
            "TAMPERED-BASELINE-AGENT",
            "system",
            false,
            "ev:win-art1",
        );
        cur_win
            .artifacts
            .insert("art:win-baseline-agent".to_string(), mod_art);

        let cur_files = curs.get_mut("files-int.corp.example").unwrap();
        cur_files.security_policies.insert(
            "pol:files-added".to_string(),
            SecurityPolicyRecord {
                policy_id: "pol:files-added".to_string(),
                setting: "AllowWeakCiphers".to_string(),
                secure_value: "False".to_string(),
                current_value: "True".to_string(),
                disables_channel: None,
                evidence_id: "ev:win-pol".to_string(),
            },
        );

        let custom = MockPostExploit::new(
            base_eng.scope.clone(),
            AUDIT_TIMESTAMP,
            bases,
            curs,
            base_eng.evidence_store().clone(),
            base_eng.access_grants().clone(),
            base_eng.lateral_edges().to_vec(),
            base_eng.telemetry_events().clone(),
            base_eng.data_flows().to_vec(),
        )
        .unwrap();
        let report = custom.run_audit("win-srv01.corp.example", 4).unwrap();
        assert!(
            report.findings.iter().any(|f| f.finding_id
                == "find:win-srv01.corp.example:art_modified:art:win-baseline-agent")
        );
        assert!(
            report
                .findings
                .iter()
                .any(|f| f.finding_id
                    == "find:files-int.corp.example:art_removed:art:files-base-cfg")
        );
        assert!(
            report
                .findings
                .iter()
                .any(|f| f.finding_id == "find:files-int.corp.example:acct_removed:audit_reader")
        );
        assert!(
            report
                .findings
                .iter()
                .any(|f| f.finding_id == "find:files-int.corp.example:pol_removed:pol:files-base")
        );
        assert!(
            report
                .findings
                .iter()
                .any(|f| f.finding_id == "find:files-int.corp.example:pol_added:pol:files-added")
        );
    }

    #[test]
    fn telemetry_channel_deletion_or_retention_drop_requires_policy_evidence_and_emits_gap() {
        let base_eng = MockPostExploit::standard();

        // Xóa trực tiếp kênh ServiceControlManager khỏi current mà không có policy evidence -> MissingEvidence.
        let mut cur_removed = base_eng.current_snapshots().clone();
        cur_removed
            .get_mut("win-srv01.corp.example")
            .unwrap()
            .telemetry_controls
            .remove(&TelemetryChannel::ServiceControlManager);
        let unbacked_del = MockPostExploit::new(
            base_eng.scope.clone(),
            AUDIT_TIMESTAMP,
            base_eng.baseline_snapshots().clone(),
            cur_removed,
            base_eng.evidence_store().clone(),
            base_eng.access_grants().clone(),
            base_eng.lateral_edges().to_vec(),
            base_eng.telemetry_events().clone(),
            base_eng.data_flows().to_vec(),
        )
        .unwrap();
        assert!(matches!(
            unbacked_del.run_audit("win-srv01.corp.example", 4),
            Err(PostExploitError::MissingEvidence(_))
        ));

        // Giảm retention_days từ 90 xuống 0 không có policy evidence -> MissingEvidence.
        let mut cur_low_ret = base_eng.current_snapshots().clone();
        cur_low_ret
            .get_mut("win-srv01.corp.example")
            .unwrap()
            .telemetry_controls
            .get_mut(&TelemetryChannel::ServiceControlManager)
            .unwrap()
            .retention_days = 0;
        let unbacked_ret = MockPostExploit::new(
            base_eng.scope.clone(),
            AUDIT_TIMESTAMP,
            base_eng.baseline_snapshots().clone(),
            cur_low_ret.clone(),
            base_eng.evidence_store().clone(),
            base_eng.access_grants().clone(),
            base_eng.lateral_edges().to_vec(),
            base_eng.telemetry_events().clone(),
            base_eng.data_flows().to_vec(),
        )
        .unwrap();
        assert!(matches!(
            unbacked_ret.run_audit("win-srv01.corp.example", 4),
            Err(PostExploitError::MissingEvidence(_))
        ));

        // Khi có policy giải thích việc hạ retention -> phát sinh TelemetryGap hợp lệ.
        cur_low_ret
            .get_mut("win-srv01.corp.example")
            .unwrap()
            .security_policies
            .insert(
                "pol:scm-ret".to_string(),
                SecurityPolicyRecord {
                    policy_id: "pol:scm-ret".to_string(),
                    setting: "ScmRetentionDays".to_string(),
                    secure_value: "90".to_string(),
                    current_value: "0".to_string(),
                    disables_channel: Some(TelemetryChannel::ServiceControlManager),
                    evidence_id: "ev:win-pol".to_string(),
                },
            );
        let backed = MockPostExploit::new(
            base_eng.scope.clone(),
            AUDIT_TIMESTAMP,
            base_eng.baseline_snapshots().clone(),
            cur_low_ret,
            base_eng.evidence_store().clone(),
            base_eng.access_grants().clone(),
            base_eng.lateral_edges().to_vec(),
            base_eng.telemetry_events().clone(),
            base_eng.data_flows().to_vec(),
        )
        .unwrap();
        let report = backed.run_audit("win-srv01.corp.example", 4).unwrap();
        assert!(report
            .findings
            .iter()
            .any(|f| f.finding_id == "find:win-srv01.corp.example:telgap:ServiceControlManager"));
    }

    #[test]
    fn report_validate_rejects_mutated_finding_fields_order_hash_and_generated_at() {
        let engine = MockPostExploit::standard();
        let valid = engine.run_audit("win-srv01.corp.example", 4).unwrap();

        let mut bad_title = valid.clone();
        bad_title.findings[0].title = "Clean".to_string();
        assert!(matches!(
            bad_title.validate(&engine),
            Err(PostExploitError::ReportSchemaInvalid(_))
        ));

        let mut downgraded = valid.clone();
        downgraded.findings[0].impact = ImpactLevel::Low;
        downgraded.findings[0].severity = Severity::Low;
        assert!(matches!(
            downgraded.validate(&engine),
            Err(PostExploitError::ReportSchemaInvalid(_))
        ));

        let mut reordered = valid.clone();
        reordered.findings.reverse();
        assert!(matches!(
            reordered.validate(&engine),
            Err(PostExploitError::ReportSchemaInvalid(_))
        ));

        let mut bad_hash = valid.clone();
        bad_hash.input_hash = "tampered".to_string();
        assert!(matches!(
            bad_hash.validate(&engine),
            Err(PostExploitError::ReportSchemaInvalid(_))
        ));

        let mut bad_gen = valid;
        bad_gen.generated_at = "2099-01-01T00:00:00Z".to_string();
        assert!(matches!(
            bad_gen.validate(&engine),
            Err(PostExploitError::ReportSchemaInvalid(_))
        ));
    }

    #[test]
    fn cleanup_and_verify_baseline_cover_modified_persistence_artifacts_policies_and_direct_telemetry()
     {
        let base_eng = MockPostExploit::standard();
        let mut curs = base_eng.current_snapshots().clone();
        let win = curs.get_mut("win-srv01.corp.example").unwrap();
        win.persistence_records
            .get_mut("svc:baseline-monitoring")
            .unwrap()
            .target_descriptor = "synthetic://hijacked-target".to_string();
        win.artifacts.insert(
            "art:win-baseline-agent".to_string(),
            Artifact::new(
                "art:win-baseline-agent",
                "win-srv01.corp.example",
                "virtual://win-srv01.corp.example/programdata/baseline-agent.bin",
                ArtifactKind::ModifiedConfig,
                "TAMPERED-CONF",
                "system",
                false,
                "ev:win-art1",
            ),
        );
        win.telemetry_controls.remove(&TelemetryChannel::AuthAudit);
        win.security_policies.insert(
            "pol:lab-extra".to_string(),
            SecurityPolicyRecord {
                policy_id: "pol:lab-extra".to_string(),
                setting: "TempDebugFlag".to_string(),
                secure_value: "0".to_string(),
                current_value: "1".to_string(),
                disables_channel: None,
                evidence_id: "ev:win-pol".to_string(),
            },
        );
        // Thêm kênh CronAudit không thuộc baseline trên win-srv01 kèm policy baseline trỏ disables_channel=CronAudit.
        let mut bases = base_eng.baseline_snapshots().clone();
        bases
            .get_mut("win-srv01.corp.example")
            .unwrap()
            .security_policies
            .insert(
                "pol:cron-ref".to_string(),
                SecurityPolicyRecord {
                    policy_id: "pol:cron-ref".to_string(),
                    setting: "CronControlRef".to_string(),
                    secure_value: "None".to_string(),
                    current_value: "None".to_string(),
                    disables_channel: Some(TelemetryChannel::CronAudit),
                    evidence_id: "ev:base-win".to_string(),
                },
            );
        win.security_policies.insert(
            "pol:cron-ref".to_string(),
            SecurityPolicyRecord {
                policy_id: "pol:cron-ref".to_string(),
                setting: "CronControlRef".to_string(),
                secure_value: "None".to_string(),
                current_value: "Changed".to_string(),
                disables_channel: Some(TelemetryChannel::CronAudit),
                evidence_id: "ev:win-pol".to_string(),
            },
        );
        win.telemetry_controls.insert(
            TelemetryChannel::CronAudit,
            TelemetryControl {
                channel: TelemetryChannel::CronAudit,
                enabled: true,
                retention_days: 30,
            },
        );

        let mut custom = MockPostExploit::new(
            base_eng.scope.clone(),
            AUDIT_TIMESTAMP,
            bases,
            curs,
            base_eng.evidence_store().clone(),
            base_eng.access_grants().clone(),
            base_eng.lateral_edges().to_vec(),
            base_eng.telemetry_events().clone(),
            base_eng.data_flows().to_vec(),
        )
        .unwrap();

        let pre = custom.verify_baseline();
        assert!(!pre.clean);
        assert!(
            pre.residual_drifts
                .iter()
                .any(|d| d.contains("svc:baseline-monitoring"))
        );
        assert!(
            pre.residual_drifts
                .iter()
                .any(|d| d.contains("art:win-baseline-agent"))
        );
        assert!(pre.residual_drifts.iter().any(|d| d.contains("AuthAudit")));
        assert!(
            pre.residual_drifts
                .iter()
                .any(|d| d.contains("pol:lab-extra"))
        );

        let plan = custom.build_cleanup_plan("plan-full-restore").unwrap();
        let pass1 = custom.apply_cleanup(&plan).unwrap();
        assert!(
            pass1.verification.clean,
            "còn residual drifts: {:?}",
            pass1.verification.residual_drifts
        );
        let pass2 = custom.apply_cleanup(&plan).unwrap();
        assert_eq!(pass2.applied_mutations, 0);
        assert!(pass2.verification.clean);
    }

    #[test]
    fn iso8601_validator_rejects_plus_sign_prefixes() {
        assert!(!is_valid_iso8601_utc("+202-+1-+1T+0:+0:+0Z"));
        assert!(!is_valid_iso8601_utc("+000-+1-+1T+0:+0:+0Z"));
    }

    #[test]
    fn finding_validation_rejects_unrelated_telemetry_whitespace_and_secret_in_preconditions() {
        let engine = MockPostExploit::standard();
        let report = engine.run_audit("win-srv01.corp.example", 4).unwrap();

        let mut unrelated_tel = report
            .findings
            .iter()
            .find(|f| f.finding_id == "find:win-srv01.corp.example:task:updater-sync")
            .unwrap()
            .clone();
        unrelated_tel.telemetry_ids = BTreeSet::from(["tel:win-public-flow".to_string()]);
        unrelated_tel.detection_gap = None;
        assert!(matches!(
            unrelated_tel.validate_invariants(engine.evidence_store(), engine.telemetry_events()),
            Err(PostExploitError::ValidationFailed(_))
        ));

        let mut lat_wrong_host = report
            .findings
            .iter()
            .find(|f| f.category == FindingCategory::LateralExposure)
            .unwrap()
            .clone();
        lat_wrong_host
            .telemetry_ids
            .insert("tel:files-exfil-flow".to_string());
        assert!(matches!(
            lat_wrong_host.validate_invariants(engine.evidence_store(), engine.telemetry_events()),
            Err(PostExploitError::ValidationFailed(_))
        ));

        let mut custom_tels = engine.telemetry_events().clone();
        custom_tels.insert(
            "tel:prefix-collision".to_string(),
            TelemetryEvent {
                event_id: "tel:prefix-collision".to_string(),
                timestamp: AUDIT_TIMESTAMP.to_string(),
                host: "win-srv01.corp.example".to_string(),
                channel: TelemetryChannel::AuthAudit,
                event_code: "MOCK-SUB".to_string(),
                summary: "Substring prefix edge".to_string(),
                subject_id: "edge:win".to_string(),
                evidence_id: "ev:tel-link".to_string(),
            },
        );
        let mut lat_sub = report
            .findings
            .iter()
            .find(|f| f.finding_id == "find:lateral:edge:win-to-lin")
            .unwrap()
            .clone();
        lat_sub
            .telemetry_ids
            .insert("tel:prefix-collision".to_string());
        assert!(matches!(
            lat_sub.validate_invariants(engine.evidence_store(), &custom_tels),
            Err(PostExploitError::ValidationFailed(_))
        ));

        let mut ws_gap = report.findings[0].clone();
        ws_gap.detection_gap = Some("   ".to_string());
        assert!(matches!(
            ws_gap.validate_invariants(engine.evidence_store(), engine.telemetry_events()),
            Err(PostExploitError::ValidationFailed(_))
        ));

        let mut secret_pre = report.findings[0].clone();
        secret_pre
            .preconditions
            .push("token bearer secret-123".to_string());
        assert!(matches!(
            secret_pre.validate_invariants(engine.evidence_store(), engine.telemetry_events()),
            Err(PostExploitError::RawSecretProhibited(_))
        ));
    }

    #[test]
    fn classify_data_flow_enforces_backup_allowlist_internal_host_check_and_public_cap() {
        let engine = MockPostExploit::standard();

        let unauth_ip_backup = DataFlow::new(
            "flow:ip-backup",
            "linux-app01.corp.example",
            "10.99.99.99",
            DestinationCategory::AuthorizedBackup,
            FlowProtocol::SftpMeta,
            1_000_000,
            60,
            DataSensitivity::RestrictedRegulated,
            true,
            "ev:flow-backup",
        );
        let r1 =
            classify_data_flow(&engine.scope, &unauth_ip_backup, engine.evidence_store()).unwrap();
        assert_eq!(r1.risk, ExfilRiskLevel::High);

        let fake_internal = DataFlow::new(
            "flow:fake-internal",
            "linux-app01.corp.example",
            "rogue.external.example",
            DestinationCategory::InternalCollector,
            FlowProtocol::HttpsMeta,
            5_000_000,
            30,
            DataSensitivity::Confidential,
            true,
            "ev:flow-exfil",
        );
        let r2 =
            classify_data_flow(&engine.scope, &fake_internal, engine.evidence_store()).unwrap();
        assert_eq!(r2.risk, ExfilRiskLevel::Critical);

        let public_dns_internal = DataFlow::new(
            "flow:pub-dns",
            "win-srv01.corp.example",
            "10.20.40.99",
            DestinationCategory::InternalCollector,
            FlowProtocol::DnsTxtMeta,
            10_000,
            15,
            DataSensitivity::Public,
            false,
            "ev:flow-dns",
        );
        let r3 = classify_data_flow(&engine.scope, &public_dns_internal, engine.evidence_store())
            .unwrap();
        assert_eq!(r3.risk, ExfilRiskLevel::Low);
        assert_eq!(r3.impact, ImpactLevel::Low);
    }

    #[test]
    fn run_audit_normalizes_uppercase_start_host_and_new_rejects_unnormalized_nested_hosts() {
        let engine = MockPostExploit::standard();
        let lower_rep = engine.run_audit("win-srv01.corp.example", 4).unwrap();
        let upper_rep = engine.run_audit("WIN-SRV01.CORP.EXAMPLE.", 4).unwrap();
        assert_eq!(lower_rep.render_json(), upper_rep.render_json());

        let mut bad_grants = engine.access_grants().clone();
        bad_grants.get_mut("grant:win-lin-ssh").unwrap().source_host =
            "WIN-SRV01.CORP.EXAMPLE".to_string();
        assert!(matches!(
            MockPostExploit::new(
                engine.scope.clone(),
                AUDIT_TIMESTAMP,
                engine.baseline_snapshots().clone(),
                engine.current_snapshots().clone(),
                engine.evidence_store().clone(),
                bad_grants,
                engine.lateral_edges().to_vec(),
                engine.telemetry_events().clone(),
                engine.data_flows().to_vec(),
            ),
            Err(PostExploitError::InvalidDomain(_))
        ));

        let mut bad_edges = engine.lateral_edges().to_vec();
        bad_edges[0].to_host = "LINUX-APP01.CORP.EXAMPLE".to_string();
        assert!(matches!(
            MockPostExploit::new(
                engine.scope.clone(),
                AUDIT_TIMESTAMP,
                engine.baseline_snapshots().clone(),
                engine.current_snapshots().clone(),
                engine.evidence_store().clone(),
                engine.access_grants().clone(),
                bad_edges,
                engine.telemetry_events().clone(),
                engine.data_flows().to_vec(),
            ),
            Err(PostExploitError::InvalidDomain(_))
        ));
    }

    #[test]
    fn snapshot_validate_and_diff_reject_dangling_artifact_id_secret_group_and_nonlab_current_artifact()
     {
        let engine = MockPostExploit::standard();
        let mut base = engine
            .baseline_snapshots()
            .get("win-srv01.corp.example")
            .unwrap()
            .clone();
        let mut cur = engine
            .current_snapshots()
            .get("win-srv01.corp.example")
            .unwrap()
            .clone();

        // Dangling artifact_id
        let mut dangling = cur.clone();
        dangling
            .persistence_records
            .get_mut("task:updater-sync")
            .unwrap()
            .artifact_id = Some("art:nonexistent".to_string());
        assert!(matches!(
            dangling.validate(engine.evidence_store()),
            Err(PostExploitError::SnapshotMismatch(_))
        ));

        // Secret inside account group
        let mut secret_grp = cur.clone();
        secret_grp
            .accounts
            .get_mut("svc_maintenance")
            .unwrap()
            .groups
            .insert("ghp_123456789".to_string());
        assert!(matches!(
            secret_grp.validate(engine.evidence_store()),
            Err(PostExploitError::RawSecretProhibited(_))
        ));

        // Direct diff_snapshots rejects non-baseline artifact marked created_in_lab=false.
        cur.artifacts.insert(
            "art:stealth-false".to_string(),
            Artifact::new(
                "art:stealth-false",
                "win-srv01.corp.example",
                "virtual://win-srv01.corp.example/staging/stealth.bin",
                ArtifactKind::DropperBinary,
                "STEALTH",
                "svc_deploy",
                false,
                "ev:win-art1",
            ),
        );
        assert!(matches!(
            diff_snapshots(&base, &cur, engine.evidence_store()),
            Err(PostExploitError::BaselineProtected(_))
        ));

        base.artifacts
            .get_mut("art:win-baseline-agent")
            .unwrap()
            .created_in_lab = true;
        assert!(matches!(
            diff_snapshots(&base, &cur, engine.evidence_store()),
            Err(PostExploitError::BaselineProtected(_))
        ));
    }

    #[test]
    fn input_hash_tracks_content_markdown_escapes_ampersand_and_backticks_and_lateral_cleanup_refs_stay_scoped()
     {
        let e1 = MockPostExploit::standard();
        let mut flows2 = e1.data_flows().to_vec();
        flows2[0].bytes_sent += 1;
        let e2 = MockPostExploit::new(
            e1.scope.clone(),
            AUDIT_TIMESTAMP,
            e1.baseline_snapshots().clone(),
            e1.current_snapshots().clone(),
            e1.evidence_store().clone(),
            e1.access_grants().clone(),
            e1.lateral_edges().to_vec(),
            e1.telemetry_events().clone(),
            flows2,
        )
        .unwrap();
        let r1 = e1.run_audit("win-srv01.corp.example", 4).unwrap();
        let r2 = e2.run_audit("win-srv01.corp.example", 4).unwrap();
        assert_ne!(r1.input_hash, r2.input_hash);

        assert_eq!(
            markdown_escape("a&#60;`b`\\|c"),
            "a&amp;#60;\\`b\\`\\\\\\|c"
        );
        let md = r1.render_markdown();
        assert!(!md.contains("- Schema: `"));

        let lat_finding = r1
            .findings
            .iter()
            .find(|f| f.finding_id == "find:lateral:edge:win-to-lin")
            .unwrap();
        assert_eq!(
            lat_finding.cleanup_refs,
            vec!["grant:grant:win-lin-ssh".to_string()]
        );
    }
}
