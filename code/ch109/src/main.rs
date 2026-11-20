#![forbid(unsafe_code)]
#![allow(dead_code)]

//! Tác tử OSINT thụ động và đồ thị bằng chứng trên fixture hoàn toàn tổng hợp.
//! Không có kết nối mạng, DNS/WHOIS/API thật, thu thập dữ liệu cá nhân (PII),
//! khóa bí mật thật hay active probing. Mọi định danh chỉ dùng miền `.example`
//! (RFC 2606), IPv4 RFC1918 và role contact tổ chức tổng hợp.

use std::collections::{BTreeMap, BTreeSet, VecDeque};
use std::error::Error;
use std::fmt;

pub const ENGAGEMENT_ID: &str = "ENG-SYNTHETIC-109";
pub const FIXED_TIMESTAMP: &str = "2026-01-01T00:00:00Z";
pub const ARCHIVE_TIMESTAMP: &str = "2025-06-15T00:00:00Z";
pub const CHECKPOINT_VERSION: u32 = 1;
pub const REPORT_SCHEMA_VERSION: &str = "osint-report/v1";
pub const SYNTHETIC_REDACTED_MARKER: &str = "SYNTHETIC-REDACTED-NOT-A-SECRET";

const ALLOWED_ROLE_LOCAL_PARTS: &[&str] = &[
    "security",
    "noc",
    "hostmaster",
    "cert-admin",
    "ops",
    "abuse",
    "postmaster",
];

const PROHIBITED_PII_ATTRIBUTES: &[&str] = &[
    "person_name",
    "full_name",
    "phone",
    "ssn",
    "passport",
    "home_address",
    "personal_email",
    "employee_name",
    "employee_id",
];

// ============================================================================
// 1. HÀM BĂM SHA-256 THUẦN STD (FIPS 180-4) & LỖI TƯỜNG MINH
// ============================================================================

/// Cài đặt phòng sạch SHA-256 (FIPS 180-4) thuần `std` để tính `input_hash` và
/// `raw_digest` tất định trên mọi nền tảng.
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

#[derive(Debug, Clone, PartialEq, Eq)]
pub enum OsintError {
    OutOfScope(String),
    ActiveProbeProhibited,
    PersonalDataProhibited(String),
    RawSecretProhibited(String),
    InvalidDomain(String),
    InvalidRfc1918Ip(String),
    InvalidConfidence(String),
    ToolNotRegistered(SourceKind),
    ToolNotAllowed(SourceKind),
    SchemaMismatch {
        expected: ToolTargetKind,
        got: ToolTargetKind,
    },
    InvalidTarget(String),
    DuplicateSource(String),
    MalformedResult(String),
    SimulatedFaultTriggered {
        source: SourceKind,
        fault: SimulatedFault,
    },
    BudgetExhausted(StopReason),
    InvalidGraph(String),
    EntityNotFound(EntityId),
    AliasMismatch(String),
    InvalidCheckpoint(String),
    UnsupportedClaim(String),
    ConflictedFactClaim {
        entity_id: EntityId,
        attribute: String,
    },
    ReportSchemaInvalid(String),
}

impl fmt::Display for OsintError {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            Self::OutOfScope(msg) => write!(f, "ngoài phạm vi ủy quyền: {msg}"),
            Self::ActiveProbeProhibited => {
                write!(
                    f,
                    "bị chặn: không cho phép active probe trong OSINT thụ động"
                )
            }
            Self::PersonalDataProhibited(msg) => {
                write!(f, "vi phạm giảm thiểu riêng tư (PII bị chặn): {msg}")
            }
            Self::RawSecretProhibited(msg) => {
                write!(f, "chặn secret thô chưa gắn nhãn redact tổng hợp: {msg}")
            }
            Self::InvalidDomain(msg) => write!(f, "tên miền không hợp lệ: {msg}"),
            Self::InvalidRfc1918Ip(msg) => write!(f, "địa chỉ IP không thuộc RFC1918: {msg}"),
            Self::InvalidConfidence(msg) => write!(f, "giá trị confidence không hợp lệ: {msg}"),
            Self::ToolNotRegistered(src) => write!(f, "chưa đăng ký tool cho nguồn {src}"),
            Self::ToolNotAllowed(src) => write!(f, "nguồn {src} không nằm trong allowlist"),
            Self::SchemaMismatch { expected, got } => {
                write!(f, "sai kiểu target schema: cần {expected:?}, nhận {got:?}")
            }
            Self::InvalidTarget(msg) => write!(f, "target không hợp lệ: {msg}"),
            Self::DuplicateSource(msg) => write!(f, "xung đột call/source: {msg}"),
            Self::MalformedResult(msg) => write!(f, "dữ liệu kết quả tool sai định dạng: {msg}"),
            Self::SimulatedFaultTriggered { source, fault } => {
                write!(f, "lỗi mô phỏng tại nguồn {source}: {fault:?}")
            }
            Self::BudgetExhausted(reason) => write!(f, "dừng do giới hạn ngân sách: {reason:?}"),
            Self::InvalidGraph(msg) => write!(f, "bất biến đồ thị bị vi phạm: {msg}"),
            Self::EntityNotFound(id) => write!(f, "không tìm thấy thực thể {}", id.0),
            Self::AliasMismatch(msg) => write!(f, "không thể gộp thực thể khác loại/ID: {msg}"),
            Self::InvalidCheckpoint(msg) => write!(f, "checkpoint không hợp lệ: {msg}"),
            Self::UnsupportedClaim(msg) => write!(f, "phát biểu thiếu bằng chứng hợp lệ: {msg}"),
            Self::ConflictedFactClaim {
                entity_id,
                attribute,
            } => write!(
                f,
                "không được nâng thuộc tính đang mâu thuẫn thành Fact: {}.{}",
                entity_id.0, attribute
            ),
            Self::ReportSchemaInvalid(msg) => write!(f, "schema báo cáo không hợp lệ: {msg}"),
        }
    }
}

impl Error for OsintError {}

// ============================================================================
// 2. CHUẨN HÓA ĐỊNH DANH, SCOPE, PRIVACY & SECRET HYGIENE
// ============================================================================

/// Kiểm tra cú pháp tên miền cơ bản và chuẩn hóa về chữ thường, bỏ đúng một dấu `.` cuối.
fn parse_dns_name(raw: &str) -> Result<String, OsintError> {
    if raw.is_empty() || raw.chars().any(char::is_whitespace) {
        return Err(OsintError::InvalidDomain(
            "tên miền rỗng hoặc chứa khoảng trắng".to_string(),
        ));
    }
    let lowered = raw.to_ascii_lowercase();
    let stripped = lowered.strip_suffix('.').unwrap_or(&lowered);
    if stripped.is_empty() || stripped.ends_with('.') || stripped.len() > 253 {
        return Err(OsintError::InvalidDomain(raw.to_string()));
    }
    let labels: Vec<&str> = stripped.split('.').collect();
    if labels.len() < 2 {
        return Err(OsintError::InvalidDomain(raw.to_string()));
    }
    for label in labels {
        if label.is_empty() || label.len() > 63 {
            return Err(OsintError::InvalidDomain(raw.to_string()));
        }
        if label.starts_with('-') || label.ends_with('-') {
            return Err(OsintError::InvalidDomain(raw.to_string()));
        }
        if !label
            .bytes()
            .all(|b| b.is_ascii_lowercase() || b.is_ascii_digit() || b == b'-')
        {
            return Err(OsintError::InvalidDomain(raw.to_string()));
        }
    }
    Ok(stripped.to_string())
}

/// Chuẩn hóa tên miền thuộc TLD dành riêng cho tài liệu `.example` (RFC 2606).
pub fn normalize_example_domain(raw: &str) -> Result<String, OsintError> {
    let domain = parse_dns_name(raw)?;
    if !domain.ends_with(".example") || domain.len() <= ".example".len() {
        return Err(OsintError::OutOfScope(format!(
            "miền '{domain}' không nằm dưới hậu tố dành riêng .example"
        )));
    }
    Ok(domain)
}

/// Chuẩn hóa handle tổ chức (`corp-example-labs`).
pub fn normalize_org_handle(raw: &str) -> Result<String, OsintError> {
    if raw.is_empty() || raw.chars().any(char::is_whitespace) {
        return Err(OsintError::InvalidTarget(
            "org handle rỗng hoặc chứa khoảng trắng".to_string(),
        ));
    }
    let lowered = raw.to_ascii_lowercase();
    if lowered.len() > 64 || lowered.starts_with('-') || lowered.ends_with('-') {
        return Err(OsintError::InvalidTarget(raw.to_string()));
    }
    if !lowered
        .bytes()
        .all(|b| b.is_ascii_lowercase() || b.is_ascii_digit() || b == b'-')
    {
        return Err(OsintError::InvalidTarget(raw.to_string()));
    }
    Ok(lowered)
}

/// Kiểm tra địa chỉ IPv4 có nằm chặt trong ba dải RFC1918:
/// `10.0.0.0/8`, `172.16.0.0/12`, `192.168.0.0/16`.
pub fn is_rfc1918_ipv4(ip: &str) -> bool {
    let parts: Vec<&str> = ip.split('.').collect();
    if parts.len() != 4 {
        return false;
    }
    let mut octets = [0u8; 4];
    for (slot, part) in octets.iter_mut().zip(parts) {
        if part.is_empty() || part.len() > 3 || !part.bytes().all(|b| b.is_ascii_digit()) {
            return false;
        }
        if part.len() > 1 && part.starts_with('0') {
            return false;
        }
        let Ok(val) = part.parse::<u8>() else {
            return false;
        };
        *slot = val;
    }
    let [a, b, _, _] = octets;
    a == 10 || (a == 172 && (16..=31).contains(&b)) || (a == 192 && b == 168)
}

/// Kiểm tra timestamp chuẩn ISO 8601 UTC dạng cố định `YYYY-MM-DDTHH:MM:SSZ`.
pub fn is_valid_iso8601_utc(ts: &str) -> bool {
    let bytes = ts.as_bytes();
    if bytes.len() != 20 {
        return false;
    }
    if bytes[4] != b'-'
        || bytes[7] != b'-'
        || bytes[10] != b'T'
        || bytes[13] != b':'
        || bytes[16] != b':'
        || bytes[19] != b'Z'
    {
        return false;
    }
    let digits_ok = [0..4, 5..7, 8..10, 11..13, 14..16, 17..19]
        .into_iter()
        .all(|r| bytes[r].iter().all(u8::is_ascii_digit));
    if !digits_ok {
        return false;
    }
    let month: u8 = ts[5..7].parse().unwrap_or(0);
    let day: u8 = ts[8..10].parse().unwrap_or(0);
    let hour: u8 = ts[11..13].parse().unwrap_or(99);
    let minute: u8 = ts[14..16].parse().unwrap_or(99);
    let second: u8 = ts[17..19].parse().unwrap_or(99);
    (1..=12).contains(&month)
        && (1..=31).contains(&day)
        && hour <= 23
        && minute <= 59
        && second <= 59
}

fn validate_no_raw_secret(text: &str, attribute: Option<&str>) -> Result<(), OsintError> {
    if text == SYNTHETIC_REDACTED_MARKER {
        return Ok(());
    }
    if let Some(attr) = attribute {
        let lower_attr = attr.to_ascii_lowercase();
        let sensitive_attr = [
            "secret",
            "token",
            "api_key",
            "password",
            "private_key",
            "credential",
        ]
        .iter()
        .any(|k| lower_attr.contains(k));
        if sensitive_attr {
            return Err(OsintError::RawSecretProhibited(format!(
                "thuộc tính '{attr}' phải dùng nhãn {SYNTHETIC_REDACTED_MARKER}"
            )));
        }
    }
    let raw_markers = ["ghp_", "github_pat_", "sk-", "AKIA", "xoxb-"];
    let lower_text = text.to_ascii_lowercase();
    if raw_markers.iter().any(|prefix| text.contains(prefix))
        || lower_text.contains("bearer ")
        || lower_text.contains("-----begin ")
    {
        return Err(OsintError::RawSecretProhibited(
            "phát hiện mẫu secret/token thô trong nội dung".to_string(),
        ));
    }
    Ok(())
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct OsintScope {
    pub engagement_id: String,
    pub root_domains: BTreeSet<String>,
    pub allowed_org_handles: BTreeSet<String>,
    pub allowed_sources: BTreeSet<SourceKind>,
    pub allow_active_probing: bool,
}

impl OsintScope {
    pub fn standard() -> Self {
        let mut root_domains = BTreeSet::new();
        root_domains.insert("corp.example".to_string());

        let mut allowed_org_handles = BTreeSet::new();
        allowed_org_handles.insert("corp-example-labs".to_string());

        let mut allowed_sources = BTreeSet::new();
        for src in SourceKind::ALL {
            allowed_sources.insert(src);
        }

        Self {
            engagement_id: ENGAGEMENT_ID.to_string(),
            root_domains,
            allowed_org_handles,
            allowed_sources,
            allow_active_probing: false,
        }
    }

    pub fn validate(&self) -> Result<(), OsintError> {
        if self.engagement_id.trim().is_empty() {
            return Err(OsintError::OutOfScope(
                "engagement_id không được rỗng".to_string(),
            ));
        }
        if self.allow_active_probing {
            return Err(OsintError::ActiveProbeProhibited);
        }
        if self.root_domains.is_empty() {
            return Err(OsintError::OutOfScope(
                "root_domains trong scope không được rỗng".to_string(),
            ));
        }
        for root in &self.root_domains {
            let norm = normalize_example_domain(root)?;
            if &norm != root {
                return Err(OsintError::InvalidDomain(format!(
                    "root domain '{root}' phải ở dạng chuẩn '{norm}'"
                )));
            }
        }
        for handle in &self.allowed_org_handles {
            let norm = normalize_org_handle(handle)?;
            if &norm != handle {
                return Err(OsintError::InvalidTarget(format!(
                    "org handle '{handle}' phải ở dạng chuẩn '{norm}'"
                )));
            }
        }
        Ok(())
    }

    pub fn validate_domain(&self, raw: &str) -> Result<String, OsintError> {
        let domain = normalize_example_domain(raw)?;
        let in_scope = self
            .root_domains
            .iter()
            .any(|root| domain == *root || domain.ends_with(&format!(".{root}")));
        if !in_scope {
            return Err(OsintError::OutOfScope(format!(
                "miền '{domain}' không thuộc root_domains được phép"
            )));
        }
        Ok(domain)
    }

    pub fn validate_org_handle(&self, raw: &str) -> Result<String, OsintError> {
        let handle = normalize_org_handle(raw)?;
        if !self.allowed_org_handles.contains(&handle) {
            return Err(OsintError::OutOfScope(format!(
                "org handle '{handle}' không nằm trong scope"
            )));
        }
        Ok(handle)
    }

    pub fn validate_role_contact(&self, raw: &str) -> Result<String, OsintError> {
        if raw.chars().any(char::is_whitespace) {
            return Err(OsintError::PersonalDataProhibited(
                "địa chỉ liên hệ chứa khoảng trắng".to_string(),
            ));
        }
        let Some((local, domain_raw)) = raw.split_once('@') else {
            return Err(OsintError::PersonalDataProhibited(raw.to_string()));
        };
        if domain_raw.contains('@') {
            return Err(OsintError::PersonalDataProhibited(raw.to_string()));
        }
        let local_lower = local.to_ascii_lowercase();
        if !ALLOWED_ROLE_LOCAL_PARTS.contains(&local_lower.as_str()) {
            return Err(OsintError::PersonalDataProhibited(format!(
                "chỉ cho phép role mailbox tổ chức, từ chối '{raw}'"
            )));
        }
        let domain = self.validate_domain(domain_raw)?;
        Ok(format!("{local_lower}@{domain}"))
    }

    pub fn validate_request(&self, request: &ToolRequest) -> Result<String, OsintError> {
        self.validate()?;
        if request.engagement_id != self.engagement_id {
            return Err(OsintError::OutOfScope(format!(
                "sai engagement_id '{}'",
                request.engagement_id
            )));
        }
        if request.mode != CollectionMode::Passive {
            return Err(OsintError::ActiveProbeProhibited);
        }
        if !self.allowed_sources.contains(&request.source) {
            return Err(OsintError::ToolNotAllowed(request.source));
        }
        match request.target_kind {
            ToolTargetKind::Domain => self.validate_domain(&request.target),
            ToolTargetKind::OrgHandle => self.validate_org_handle(&request.target),
        }
    }
}

// ============================================================================
// 3. NGUỒN THU THẬP, REGISTRY ĐỊNH KIỂU & SÁU NGUỒN PASSIVE MOCK
// ============================================================================

#[derive(Debug, Clone, Copy, PartialEq, Eq, PartialOrd, Ord)]
pub enum SourceKind {
    Whois,
    Dns,
    CtLogs,
    GitHub,
    SocialMetadata,
    Wayback,
}

impl SourceKind {
    pub const ALL: [Self; 6] = [
        Self::Whois,
        Self::Dns,
        Self::CtLogs,
        Self::GitHub,
        Self::SocialMetadata,
        Self::Wayback,
    ];

    pub fn as_str(self) -> &'static str {
        match self {
            Self::Whois => "whois",
            Self::Dns => "dns",
            Self::CtLogs => "ct_logs",
            Self::GitHub => "github",
            Self::SocialMetadata => "social_metadata",
            Self::Wayback => "wayback",
        }
    }
}

impl fmt::Display for SourceKind {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        f.write_str(self.as_str())
    }
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, PartialOrd, Ord)]
pub enum CollectionMode {
    Passive,
    ActiveProbe,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, PartialOrd, Ord)]
pub enum ToolTargetKind {
    Domain,
    OrgHandle,
}

#[derive(Debug, Clone, PartialEq, Eq, PartialOrd, Ord)]
pub struct ToolRequest {
    pub engagement_id: String,
    pub source: SourceKind,
    pub target_kind: ToolTargetKind,
    pub target: String,
    pub mode: CollectionMode,
}

impl ToolRequest {
    pub fn passive(
        engagement_id: impl Into<String>,
        source: SourceKind,
        target_kind: ToolTargetKind,
        target: impl Into<String>,
    ) -> Self {
        Self {
            engagement_id: engagement_id.into(),
            source,
            target_kind,
            target: target.into(),
            mode: CollectionMode::Passive,
        }
    }

    pub fn with_mode(mut self, mode: CollectionMode) -> Self {
        self.mode = mode;
        self
    }
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, PartialOrd, Ord)]
pub enum EntityKind {
    Domain,
    IpAddress,
    Organization,
    Certificate,
    Repository,
    RoleContact,
}

#[derive(Debug, Clone, PartialEq, Eq, PartialOrd, Ord)]
pub struct EntityId(pub String);

impl EntityId {
    pub fn normalize(raw: &str) -> Result<Self, OsintError> {
        let trimmed = raw.trim().to_ascii_lowercase();
        if trimmed.is_empty() || trimmed.chars().any(char::is_whitespace) {
            return Err(OsintError::InvalidTarget(format!(
                "EntityId không hợp lệ: '{raw}'"
            )));
        }
        Ok(Self(trimmed))
    }
}

#[derive(Debug, Clone, PartialEq, Eq, PartialOrd, Ord)]
pub struct RawFindingRecord {
    pub entity_id: EntityId,
    pub entity_kind: EntityKind,
    pub canonical_name: String,
    pub attribute: String,
    pub value: String,
    pub observed_at: String,
    pub weight_milli: u16,
    pub alias_of: Option<EntityId>,
    pub discovered_targets: Vec<(ToolTargetKind, String)>,
}

impl RawFindingRecord {
    #[allow(clippy::too_many_arguments)]
    pub fn new(
        entity_id: &str,
        entity_kind: EntityKind,
        canonical_name: &str,
        attribute: &str,
        value: &str,
        observed_at: &str,
        weight_milli: u16,
    ) -> Self {
        Self {
            entity_id: EntityId(entity_id.to_string()),
            entity_kind,
            canonical_name: canonical_name.to_string(),
            attribute: attribute.to_string(),
            value: value.to_string(),
            observed_at: observed_at.to_string(),
            weight_milli,
            alias_of: None,
            discovered_targets: Vec::new(),
        }
    }

    pub fn with_alias_of(mut self, primary_id: &str) -> Self {
        self.alias_of = Some(EntityId(primary_id.to_string()));
        self
    }

    pub fn with_discovered_target(mut self, kind: ToolTargetKind, target: &str) -> Self {
        self.discovered_targets.push((kind, target.to_string()));
        self
    }
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct ToolResult {
    pub call_id: String,
    pub source: SourceKind,
    pub canonical_target: String,
    pub captured_at: String,
    pub raw_digest: String,
    pub records: Vec<RawFindingRecord>,
}

impl ToolResult {
    pub fn new(
        call_id: impl Into<String>,
        source: SourceKind,
        canonical_target: impl Into<String>,
        captured_at: impl Into<String>,
        records: Vec<RawFindingRecord>,
    ) -> Self {
        let canonical_target = canonical_target.into();
        let captured_at = captured_at.into();
        let raw_digest = compute_result_digest(source, &canonical_target, &captured_at, &records);
        Self {
            call_id: call_id.into(),
            source,
            canonical_target,
            captured_at,
            raw_digest,
            records,
        }
    }
}

fn compute_result_digest(
    source: SourceKind,
    canonical_target: &str,
    captured_at: &str,
    records: &[RawFindingRecord],
) -> String {
    let mut buf = format!(
        "{}|{}|{}|{}\n",
        source.as_str(),
        canonical_target,
        captured_at,
        records.len()
    );
    for rec in records {
        buf.push_str(&format!(
            "{}|{:?}|{}|{}|{}|{}|{}|{:?}|{}\n",
            rec.entity_id.0,
            rec.entity_kind,
            rec.canonical_name,
            rec.attribute,
            rec.value,
            rec.observed_at,
            rec.weight_milli,
            rec.alias_of,
            rec.discovered_targets.len()
        ));
        for (tk, target) in &rec.discovered_targets {
            buf.push_str(&format!("  {tk:?}:{target}\n"));
        }
    }
    sha256_hex(buf.as_bytes())
}

fn validate_raw_record(scope: &OsintScope, rec: &RawFindingRecord) -> Result<(), OsintError> {
    let norm_id = EntityId::normalize(&rec.entity_id.0)?;
    if norm_id != rec.entity_id {
        return Err(OsintError::MalformedResult(format!(
            "EntityId '{}' chưa ở dạng chuẩn '{}'",
            rec.entity_id.0, norm_id.0
        )));
    }
    if rec.canonical_name.trim().is_empty()
        || rec.attribute.trim().is_empty()
        || rec.value.trim().is_empty()
    {
        return Err(OsintError::MalformedResult(
            "canonical_name, attribute hoặc value rỗng".to_string(),
        ));
    }
    if !is_valid_iso8601_utc(&rec.observed_at) {
        return Err(OsintError::MalformedResult(format!(
            "timestamp '{}' không đúng chuẩn YYYY-MM-DDTHH:MM:SSZ",
            rec.observed_at
        )));
    }
    if rec.weight_milli > 1000 {
        return Err(OsintError::InvalidConfidence(format!(
            "weight_milli {} vượt quá 1000",
            rec.weight_milli
        )));
    }
    let attr_lower = rec.attribute.to_ascii_lowercase();
    let pii_substrings = [
        "person",
        "first_name",
        "last_name",
        "full_name",
        "phone",
        "mobile",
        "ssn",
        "passport",
        "home_address",
        "street_address",
        "personal_email",
        "employee",
    ];
    if PROHIBITED_PII_ATTRIBUTES.contains(&attr_lower.as_str())
        || (attr_lower != "role_contact"
            && pii_substrings.iter().any(|pat| attr_lower.contains(pat)))
    {
        return Err(OsintError::PersonalDataProhibited(format!(
            "thuộc tính PII bị cấm: {}",
            rec.attribute
        )));
    }
    validate_no_raw_secret(&rec.entity_id.0, None)?;
    validate_no_raw_secret(&rec.attribute, None)?;
    validate_no_raw_secret(&rec.canonical_name, None)?;
    validate_no_raw_secret(&rec.value, Some(&rec.attribute))?;

    match rec.entity_kind {
        EntityKind::Domain => {
            let norm = scope.validate_domain(&rec.canonical_name)?;
            if norm != rec.canonical_name {
                return Err(OsintError::InvalidDomain(format!(
                    "canonical_name '{}' chưa ở dạng chuẩn '{}'",
                    rec.canonical_name, norm
                )));
            }
            if rec.entity_id.0 != format!("domain:{norm}") {
                return Err(OsintError::InvalidTarget(format!(
                    "EntityId '{}' không khớp thực thể Domain 'domain:{norm}'",
                    rec.entity_id.0
                )));
            }
        }
        EntityKind::IpAddress => {
            if !is_rfc1918_ipv4(&rec.canonical_name) {
                return Err(OsintError::InvalidRfc1918Ip(rec.canonical_name.clone()));
            }
            if rec.entity_id.0 != format!("ip:{}", rec.canonical_name) {
                return Err(OsintError::InvalidTarget(format!(
                    "EntityId '{}' không khớp thực thể IpAddress 'ip:{}'",
                    rec.entity_id.0, rec.canonical_name
                )));
            }
        }
        EntityKind::RoleContact => {
            let norm = scope.validate_role_contact(&rec.canonical_name)?;
            if norm != rec.canonical_name {
                return Err(OsintError::PersonalDataProhibited(format!(
                    "role contact '{}' chưa ở dạng chuẩn '{norm}'",
                    rec.canonical_name
                )));
            }
            if rec.entity_id.0 != format!("role:{norm}") {
                return Err(OsintError::InvalidTarget(format!(
                    "EntityId '{}' không khớp thực thể RoleContact 'role:{norm}'",
                    rec.entity_id.0
                )));
            }
        }
        EntityKind::Organization => {
            if !rec.entity_id.0.starts_with("org:") || rec.entity_id.0.len() <= 4 {
                return Err(OsintError::InvalidTarget(format!(
                    "EntityId '{}' phải bắt đầu bằng 'org:'",
                    rec.entity_id.0
                )));
            }
        }
        EntityKind::Certificate => {
            if !rec.entity_id.0.starts_with("cert:") || rec.entity_id.0.len() <= 5 {
                return Err(OsintError::InvalidTarget(format!(
                    "EntityId '{}' phải bắt đầu bằng 'cert:'",
                    rec.entity_id.0
                )));
            }
        }
        EntityKind::Repository => {
            if !rec.entity_id.0.starts_with("repo:") || rec.entity_id.0.len() <= 5 {
                return Err(OsintError::InvalidTarget(format!(
                    "EntityId '{}' phải bắt đầu bằng 'repo:'",
                    rec.entity_id.0
                )));
            }
        }
    }

    if (attr_lower == "a_record" || attr_lower == "ip_address") && !is_rfc1918_ipv4(&rec.value) {
        return Err(OsintError::InvalidRfc1918Ip(rec.value.clone()));
    }
    if [
        "san_domain",
        "cname_target",
        "mx_record",
        "hosted_domain",
        "referenced_domain",
        "official_domain",
        "domain",
    ]
    .contains(&attr_lower.as_str())
    {
        let norm = scope.validate_domain(&rec.value)?;
        if norm != rec.value {
            return Err(OsintError::InvalidDomain(format!(
                "giá trị miền '{}' chưa ở dạng chuẩn '{norm}'",
                rec.value
            )));
        }
    }

    if attr_lower == "role_contact" {
        let norm = scope.validate_role_contact(&rec.value)?;
        if norm != rec.value {
            return Err(OsintError::PersonalDataProhibited(format!(
                "role_contact '{}' chưa ở dạng chuẩn '{norm}'",
                rec.value
            )));
        }
    }

    if rec.entity_id.0.contains('@') {
        let email_part = rec
            .entity_id
            .0
            .split_once(':')
            .map_or(rec.entity_id.0.as_str(), |(_, rhs)| rhs);
        let norm = scope.validate_role_contact(email_part)?;
        if norm != email_part {
            return Err(OsintError::PersonalDataProhibited(email_part.to_string()));
        }
    }
    if rec.value.contains('@') {
        let norm = scope.validate_role_contact(&rec.value)?;
        if norm != rec.value {
            return Err(OsintError::PersonalDataProhibited(rec.value.clone()));
        }
    }
    if rec.canonical_name.contains('@') {
        let norm = scope.validate_role_contact(&rec.canonical_name)?;
        if norm != rec.canonical_name {
            return Err(OsintError::PersonalDataProhibited(
                rec.canonical_name.clone(),
            ));
        }
    }

    if let Some(alias) = &rec.alias_of {
        let norm_alias = EntityId::normalize(&alias.0)?;
        if norm_alias != *alias {
            return Err(OsintError::MalformedResult(format!(
                "alias_of '{}' chưa ở dạng chuẩn '{}'",
                alias.0, norm_alias.0
            )));
        }
    }
    Ok(())
}

pub trait OsintTool {
    fn source_kind(&self) -> SourceKind;
    fn expected_target_kind(&self) -> ToolTargetKind;
    fn query(&mut self, request: &ToolRequest) -> Result<Vec<RawFindingRecord>, OsintError>;
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, PartialOrd, Ord)]
pub enum SimulatedFault {
    TimeoutOnce,
    SourceUnavailable,
    MalformedPayload,
}

pub struct ToolRegistry {
    tools: BTreeMap<SourceKind, Box<dyn OsintTool>>,
    allowed_sources: BTreeSet<SourceKind>,
    invocations: BTreeMap<(SourceKind, String), usize>,
    faults: BTreeMap<(SourceKind, String), SimulatedFault>,
}

impl ToolRegistry {
    pub fn new(allowed_sources: BTreeSet<SourceKind>) -> Self {
        Self {
            tools: BTreeMap::new(),
            allowed_sources,
            invocations: BTreeMap::new(),
            faults: BTreeMap::new(),
        }
    }

    pub fn standard() -> Self {
        let mut allowed = BTreeSet::new();
        for src in SourceKind::ALL {
            allowed.insert(src);
        }
        let mut reg = Self::new(allowed);
        reg.register(MockWhoisTool);
        reg.register(MockDnsTool);
        reg.register(MockCtLogsTool);
        reg.register(MockGitHubTool);
        reg.register(MockSocialMetadataTool);
        reg.register(MockWaybackTool);
        reg
    }

    pub fn register<T: OsintTool + 'static>(&mut self, tool: T) {
        self.tools.insert(tool.source_kind(), Box::new(tool));
    }

    pub fn allow_source(&mut self, source: SourceKind) {
        self.allowed_sources.insert(source);
    }

    pub fn disallow_source(&mut self, source: SourceKind) {
        self.allowed_sources.remove(&source);
    }

    pub fn is_registered(&self, source: SourceKind) -> bool {
        self.tools.contains_key(&source)
    }

    pub fn expected_target_kind(&self, source: SourceKind) -> Option<ToolTargetKind> {
        self.tools.get(&source).map(|t| t.expected_target_kind())
    }

    pub fn inject_fault(&mut self, source: SourceKind, target: &str, fault: SimulatedFault) {
        let norm = target
            .trim()
            .to_ascii_lowercase()
            .trim_end_matches('.')
            .to_string();
        self.faults.insert((source, norm), fault);
    }

    pub fn clear_fault(&mut self, source: SourceKind, target: &str) {
        let norm = target
            .trim()
            .to_ascii_lowercase()
            .trim_end_matches('.')
            .to_string();
        self.faults.remove(&(source, norm));
    }

    pub fn invocation_count(&self, source: SourceKind, target: &str) -> usize {
        let norm = target
            .trim()
            .to_ascii_lowercase()
            .trim_end_matches('.')
            .to_string();
        self.invocations.get(&(source, norm)).copied().unwrap_or(0)
    }

    pub fn total_invocations(&self) -> usize {
        self.invocations.values().sum()
    }

    pub fn execute(
        &mut self,
        scope: &OsintScope,
        request: &ToolRequest,
        call_id: &str,
    ) -> Result<ToolResult, OsintError> {
        let canonical_target = scope.validate_request(request)?;
        if !self.allowed_sources.contains(&request.source) {
            return Err(OsintError::ToolNotAllowed(request.source));
        }
        let tool = self
            .tools
            .get_mut(&request.source)
            .ok_or(OsintError::ToolNotRegistered(request.source))?;

        let expected_kind = tool.expected_target_kind();
        if expected_kind != request.target_kind {
            return Err(OsintError::SchemaMismatch {
                expected: expected_kind,
                got: request.target_kind,
            });
        }

        let key = (request.source, canonical_target.clone());
        *self.invocations.entry(key.clone()).or_insert(0) += 1;

        if let Some(fault) = self.faults.get(&key).copied() {
            match fault {
                SimulatedFault::TimeoutOnce => {
                    self.faults.remove(&key);
                    return Err(OsintError::SimulatedFaultTriggered {
                        source: request.source,
                        fault,
                    });
                }
                SimulatedFault::SourceUnavailable => {
                    return Err(OsintError::SimulatedFaultTriggered {
                        source: request.source,
                        fault,
                    });
                }
                SimulatedFault::MalformedPayload => {
                    return Err(OsintError::MalformedResult(format!(
                        "payload giả bị hỏng từ nguồn {}",
                        request.source
                    )));
                }
            }
        }

        let normalized_req = ToolRequest {
            engagement_id: request.engagement_id.clone(),
            source: request.source,
            target_kind: request.target_kind,
            target: canonical_target.clone(),
            mode: request.mode,
        };
        let records = tool.query(&normalized_req)?;
        for rec in &records {
            validate_raw_record(scope, rec)?;
        }
        let raw_digest =
            compute_result_digest(request.source, &canonical_target, FIXED_TIMESTAMP, &records);

        Ok(ToolResult {
            call_id: call_id.to_string(),
            source: request.source,
            canonical_target,
            captured_at: FIXED_TIMESTAMP.to_string(),
            raw_digest,
            records,
        })
    }
}

struct MockWhoisTool;
impl OsintTool for MockWhoisTool {
    fn source_kind(&self) -> SourceKind {
        SourceKind::Whois
    }

    fn expected_target_kind(&self) -> ToolTargetKind {
        ToolTargetKind::Domain
    }

    fn query(&mut self, request: &ToolRequest) -> Result<Vec<RawFindingRecord>, OsintError> {
        if request.target != "corp.example" {
            return Ok(Vec::new());
        }
        Ok(vec![
            RawFindingRecord::new(
                "org:corp-example",
                EntityKind::Organization,
                "Corp Example",
                "registrar",
                "Synthetic-Registrar-Example",
                FIXED_TIMESTAMP,
                900,
            )
            .with_discovered_target(ToolTargetKind::OrgHandle, "corp-example-labs"),
            RawFindingRecord::new(
                "org:corp-example",
                EntityKind::Organization,
                "Corp Example",
                "official_domain",
                "corp.example",
                FIXED_TIMESTAMP,
                900,
            ),
            RawFindingRecord::new(
                "domain:corp.example",
                EntityKind::Domain,
                "corp.example",
                "registered_org",
                "Corp Example",
                FIXED_TIMESTAMP,
                900,
            ),
            RawFindingRecord::new(
                "domain:corp.example",
                EntityKind::Domain,
                "corp.example",
                "role_contact",
                "security@corp.example",
                FIXED_TIMESTAMP,
                880,
            ),
            RawFindingRecord::new(
                "role:security@corp.example",
                EntityKind::RoleContact,
                "security@corp.example",
                "mailbox_role",
                "security",
                FIXED_TIMESTAMP,
                900,
            ),
        ])
    }
}

struct MockDnsTool;
impl OsintTool for MockDnsTool {
    fn source_kind(&self) -> SourceKind {
        SourceKind::Dns
    }

    fn expected_target_kind(&self) -> ToolTargetKind {
        ToolTargetKind::Domain
    }

    fn query(&mut self, request: &ToolRequest) -> Result<Vec<RawFindingRecord>, OsintError> {
        match request.target.as_str() {
            "corp.example" => Ok(vec![
                RawFindingRecord::new(
                    "domain:corp.example",
                    EntityKind::Domain,
                    "corp.example",
                    "a_record",
                    "10.20.30.10",
                    FIXED_TIMESTAMP,
                    900,
                )
                .with_discovered_target(ToolTargetKind::Domain, "api.corp.example")
                .with_discovered_target(ToolTargetKind::Domain, "mail.corp.example"),
                RawFindingRecord::new(
                    "domain:corp.example",
                    EntityKind::Domain,
                    "corp.example",
                    "mx_record",
                    "mail.corp.example",
                    FIXED_TIMESTAMP,
                    880,
                ),
                RawFindingRecord::new(
                    "ip:10.20.30.10",
                    EntityKind::IpAddress,
                    "10.20.30.10",
                    "hosted_domain",
                    "corp.example",
                    FIXED_TIMESTAMP,
                    880,
                ),
            ]),
            "api.corp.example" => Ok(vec![
                RawFindingRecord::new(
                    "domain:api.corp.example",
                    EntityKind::Domain,
                    "api.corp.example",
                    "a_record",
                    "10.20.30.15",
                    FIXED_TIMESTAMP,
                    900,
                ),
                RawFindingRecord::new(
                    "domain:api.corp.example",
                    EntityKind::Domain,
                    "api.corp.example",
                    "deployment_state",
                    "active",
                    FIXED_TIMESTAMP,
                    900,
                ),
                RawFindingRecord::new(
                    "ip:10.20.30.15",
                    EntityKind::IpAddress,
                    "10.20.30.15",
                    "hosted_domain",
                    "api.corp.example",
                    FIXED_TIMESTAMP,
                    880,
                ),
            ]),
            "portal.corp.example" => Ok(vec![
                RawFindingRecord::new(
                    "domain:portal.corp.example",
                    EntityKind::Domain,
                    "portal.corp.example",
                    "cname_target",
                    "api.corp.example",
                    FIXED_TIMESTAMP,
                    880,
                ),
                RawFindingRecord::new(
                    "domain:portal.corp.example",
                    EntityKind::Domain,
                    "portal.corp.example",
                    "a_record",
                    "10.20.30.15",
                    FIXED_TIMESTAMP,
                    880,
                ),
            ]),
            "mail.corp.example" => Ok(vec![
                RawFindingRecord::new(
                    "domain:mail.corp.example",
                    EntityKind::Domain,
                    "mail.corp.example",
                    "a_record",
                    "10.20.30.11",
                    FIXED_TIMESTAMP,
                    880,
                ),
                RawFindingRecord::new(
                    "ip:10.20.30.11",
                    EntityKind::IpAddress,
                    "10.20.30.11",
                    "hosted_domain",
                    "mail.corp.example",
                    FIXED_TIMESTAMP,
                    850,
                ),
            ]),
            "staging-old.corp.example" => Ok(vec![
                RawFindingRecord::new(
                    "domain:staging-old.corp.example",
                    EntityKind::Domain,
                    "staging-old.corp.example",
                    "a_record",
                    "10.20.30.20",
                    FIXED_TIMESTAMP,
                    850,
                ),
                RawFindingRecord::new(
                    "domain:staging-old.corp.example",
                    EntityKind::Domain,
                    "staging-old.corp.example",
                    "deployment_state",
                    "decommissioned",
                    FIXED_TIMESTAMP,
                    850,
                ),
                RawFindingRecord::new(
                    "ip:10.20.30.20",
                    EntityKind::IpAddress,
                    "10.20.30.20",
                    "hosted_domain",
                    "staging-old.corp.example",
                    FIXED_TIMESTAMP,
                    800,
                ),
            ]),
            _ => Ok(Vec::new()),
        }
    }
}

struct MockCtLogsTool;
impl OsintTool for MockCtLogsTool {
    fn source_kind(&self) -> SourceKind {
        SourceKind::CtLogs
    }

    fn expected_target_kind(&self) -> ToolTargetKind {
        ToolTargetKind::Domain
    }

    fn query(&mut self, request: &ToolRequest) -> Result<Vec<RawFindingRecord>, OsintError> {
        match request.target.as_str() {
            "corp.example" => Ok(vec![
                RawFindingRecord::new(
                    "cert:corp.example-2026",
                    EntityKind::Certificate,
                    "CN=corp.example",
                    "san_domain",
                    "corp.example",
                    FIXED_TIMESTAMP,
                    920,
                )
                .with_discovered_target(ToolTargetKind::Domain, "api.corp.example")
                .with_discovered_target(ToolTargetKind::Domain, "portal.corp.example")
                .with_discovered_target(ToolTargetKind::Domain, "staging-old.corp.example"),
            ]),
            "api.corp.example" => Ok(vec![
                RawFindingRecord::new(
                    "cert:api.corp.example-2026",
                    EntityKind::Certificate,
                    "CN=api.corp.example",
                    "san_domain",
                    "api.corp.example",
                    FIXED_TIMESTAMP,
                    920,
                ),
                RawFindingRecord::new(
                    "cert:api.corp.example-2026",
                    EntityKind::Certificate,
                    "CN=api.corp.example",
                    "cert_status",
                    "valid",
                    FIXED_TIMESTAMP,
                    920,
                ),
            ]),
            "portal.corp.example" => Ok(vec![
                RawFindingRecord::new(
                    "cert:portal.corp.example-2026",
                    EntityKind::Certificate,
                    "CN=portal.corp.example",
                    "san_domain",
                    "portal.corp.example",
                    FIXED_TIMESTAMP,
                    920,
                ),
                RawFindingRecord::new(
                    "cert:portal.corp.example-2026",
                    EntityKind::Certificate,
                    "CN=portal.corp.example",
                    "cert_status",
                    "valid",
                    FIXED_TIMESTAMP,
                    920,
                ),
            ]),
            "staging-old.corp.example" => Ok(vec![
                RawFindingRecord::new(
                    "cert:staging-old.corp.example-2025",
                    EntityKind::Certificate,
                    "CN=staging-old.corp.example",
                    "san_domain",
                    "staging-old.corp.example",
                    FIXED_TIMESTAMP,
                    900,
                ),
                RawFindingRecord::new(
                    "cert:staging-old.corp.example-2025",
                    EntityKind::Certificate,
                    "CN=staging-old.corp.example",
                    "cert_status",
                    "expired-unrevoked",
                    FIXED_TIMESTAMP,
                    900,
                ),
            ]),
            _ => Ok(Vec::new()),
        }
    }
}

struct MockGitHubTool;
impl OsintTool for MockGitHubTool {
    fn source_kind(&self) -> SourceKind {
        SourceKind::GitHub
    }

    fn expected_target_kind(&self) -> ToolTargetKind {
        ToolTargetKind::OrgHandle
    }

    fn query(&mut self, request: &ToolRequest) -> Result<Vec<RawFindingRecord>, OsintError> {
        if request.target != "corp-example-labs" {
            return Ok(Vec::new());
        }
        Ok(vec![
            RawFindingRecord::new(
                "org:corp-example-labs",
                EntityKind::Organization,
                "corp-example-labs",
                "official_domain",
                "corp.example",
                FIXED_TIMESTAMP,
                860,
            )
            .with_alias_of("org:corp-example"),
            RawFindingRecord::new(
                "org:corp-example-labs",
                EntityKind::Organization,
                "corp-example-labs",
                "role_contact",
                "security@corp.example",
                FIXED_TIMESTAMP,
                860,
            ),
            RawFindingRecord::new(
                "repo:corp-example-labs/public-sdk",
                EntityKind::Repository,
                "corp-example-labs/public-sdk",
                "referenced_domain",
                "api.corp.example",
                FIXED_TIMESTAMP,
                870,
            )
            .with_discovered_target(ToolTargetKind::Domain, "api.corp.example"),
            RawFindingRecord::new(
                "repo:corp-example-labs/public-sdk",
                EntityKind::Repository,
                "corp-example-labs/public-sdk",
                "config_secret",
                SYNTHETIC_REDACTED_MARKER,
                FIXED_TIMESTAMP,
                900,
            ),
        ])
    }
}

struct MockSocialMetadataTool;
impl OsintTool for MockSocialMetadataTool {
    fn source_kind(&self) -> SourceKind {
        SourceKind::SocialMetadata
    }

    fn expected_target_kind(&self) -> ToolTargetKind {
        ToolTargetKind::OrgHandle
    }

    fn query(&mut self, request: &ToolRequest) -> Result<Vec<RawFindingRecord>, OsintError> {
        if request.target != "corp-example-labs" {
            return Ok(Vec::new());
        }
        Ok(vec![
            RawFindingRecord::new(
                "org:corp-example",
                EntityKind::Organization,
                "Corp Example",
                "official_domain",
                "corp.example",
                FIXED_TIMESTAMP,
                850,
            ),
            RawFindingRecord::new(
                "org:corp-example",
                EntityKind::Organization,
                "Corp Example",
                "verified_handle",
                "corp-example-labs",
                FIXED_TIMESTAMP,
                850,
            ),
        ])
    }
}

struct MockWaybackTool;
impl OsintTool for MockWaybackTool {
    fn source_kind(&self) -> SourceKind {
        SourceKind::Wayback
    }

    fn expected_target_kind(&self) -> ToolTargetKind {
        ToolTargetKind::Domain
    }

    fn query(&mut self, request: &ToolRequest) -> Result<Vec<RawFindingRecord>, OsintError> {
        match request.target.as_str() {
            "corp.example" => Ok(vec![RawFindingRecord::new(
                "domain:corp.example",
                EntityKind::Domain,
                "corp.example",
                "historical_snapshot",
                "2025-06-15",
                ARCHIVE_TIMESTAMP,
                800,
            )]),
            "staging-old.corp.example" => Ok(vec![
                // Mâu thuẫn có chủ đích với DNS hiện tại: IP cũ 10.20.30.99 và trạng thái active.
                RawFindingRecord::new(
                    "domain:staging-old.corp.example",
                    EntityKind::Domain,
                    "staging-old.corp.example",
                    "a_record",
                    "10.20.30.99",
                    ARCHIVE_TIMESTAMP,
                    650,
                ),
                RawFindingRecord::new(
                    "domain:staging-old.corp.example",
                    EntityKind::Domain,
                    "staging-old.corp.example",
                    "deployment_state",
                    "active",
                    ARCHIVE_TIMESTAMP,
                    650,
                ),
            ]),
            _ => Ok(Vec::new()),
        }
    }
}

// ============================================================================
// 4. ĐỒ THỊ BẰNG CHỨNG, CONFIDENCE & ENTITY RESOLUTION
// ============================================================================

#[derive(Debug, Clone, PartialEq, Eq, PartialOrd, Ord)]
pub struct Confidence {
    milli: u16,
    pub rationale: String,
}

impl Confidence {
    pub fn from_milli(milli: u16, rationale: impl Into<String>) -> Result<Self, OsintError> {
        if milli > 1000 {
            return Err(OsintError::InvalidConfidence(format!(
                "confidence milli {milli} vượt quá 1000"
            )));
        }
        let rationale = rationale.into();
        if rationale.trim().is_empty() {
            return Err(OsintError::InvalidConfidence(
                "rationale của confidence không được rỗng".to_string(),
            ));
        }
        Ok(Self { milli, rationale })
    }

    pub fn new(score: f64, rationale: impl Into<String>) -> Result<Self, OsintError> {
        if !score.is_finite() || !(0.0..=1.0).contains(&score) {
            return Err(OsintError::InvalidConfidence(format!(
                "confidence {score} nằm ngoài [0.0, 1.0]"
            )));
        }
        let milli = (score * 1000.0).round() as u16;
        Self::from_milli(milli, rationale)
    }

    pub fn milli(&self) -> u16 {
        self.milli
    }

    pub fn value(&self) -> f64 {
        f64::from(self.milli) / 1000.0
    }
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, PartialOrd, Ord)]
pub enum ResolutionStatus {
    Confirmed,
    Conflicted,
    Unknown,
}

#[derive(Debug, Clone, PartialEq, Eq, PartialOrd, Ord)]
pub struct AttributeObservation {
    pub value: String,
    pub observed_at: String,
    pub source_kind: SourceKind,
    pub weight_milli: u16,
    pub evidence_id: String,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct AttributeResolution {
    pub attribute: String,
    pub status: ResolutionStatus,
    pub resolved_value: Option<String>,
    pub candidates: Vec<AttributeObservation>,
    pub confidence: Confidence,
}

impl AttributeResolution {
    fn from_first(attribute: &str, obs: AttributeObservation) -> Result<Self, OsintError> {
        let mut res = Self {
            attribute: attribute.to_string(),
            status: ResolutionStatus::Confirmed,
            resolved_value: Some(obs.value.clone()),
            candidates: vec![obs],
            confidence: Confidence::from_milli(500, "khởi tạo")?,
        };
        res.recompute()?;
        Ok(res)
    }

    fn add_candidate(&mut self, obs: AttributeObservation) -> Result<(), OsintError> {
        if self
            .candidates
            .iter()
            .any(|existing| existing.evidence_id == obs.evidence_id)
        {
            return Ok(());
        }
        self.candidates.push(obs);
        self.candidates
            .sort_by(|a, b| a.evidence_id.cmp(&b.evidence_id));
        self.recompute()
    }

    fn recompute(&mut self) -> Result<(), OsintError> {
        if self.candidates.is_empty() {
            self.status = ResolutionStatus::Unknown;
            self.resolved_value = None;
            self.confidence = Confidence::from_milli(0, "chưa có quan sát")?;
            return Ok(());
        }

        let distinct_values: BTreeSet<&str> =
            self.candidates.iter().map(|c| c.value.as_str()).collect();
        let distinct_sources: BTreeSet<SourceKind> =
            self.candidates.iter().map(|c| c.source_kind).collect();

        if distinct_values.len() == 1 {
            let value = (*distinct_values.iter().next().expect("non-empty")).to_string();
            let max_weight = self
                .candidates
                .iter()
                .map(|c| c.weight_milli)
                .max()
                .unwrap_or(500);
            let milli = if distinct_sources.len() >= 2 {
                max_weight.saturating_add(80).min(980)
            } else {
                max_weight.min(900)
            };
            let rationale = format!(
                "{} nguồn ({}) đồng thuận giá trị '{}'",
                distinct_sources.len(),
                distinct_sources
                    .iter()
                    .map(|s| s.as_str())
                    .collect::<Vec<_>>()
                    .join(", "),
                value
            );
            self.status = ResolutionStatus::Confirmed;
            self.resolved_value = Some(value);
            self.confidence = Confidence::from_milli(milli, rationale)?;
        } else {
            let vals: Vec<&str> = distinct_values.iter().copied().collect();
            let sources: Vec<&str> = distinct_sources.iter().map(|s| s.as_str()).collect();
            let milli = (1000u16 / vals.len() as u16).saturating_sub(150).min(450);
            let rationale = format!(
                "mâu thuẫn giữa {} giá trị [{}] từ các nguồn [{}]; giữ nguyên mọi ứng viên",
                vals.len(),
                vals.join(" vs "),
                sources.join(", ")
            );
            self.status = ResolutionStatus::Conflicted;
            self.resolved_value = None;
            self.confidence = Confidence::from_milli(milli, rationale)?;
        }
        Ok(())
    }
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct SourceRecord {
    pub id: String,
    pub kind: SourceKind,
    pub label: String,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct Observation {
    pub id: String,
    pub source_id: String,
    pub source_kind: SourceKind,
    pub entity_id: EntityId,
    pub attribute: String,
    pub value: String,
    pub observed_at: String,
    pub weight_milli: u16,
    pub alias_of: Option<EntityId>,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct Evidence {
    pub id: String,
    pub observation_id: String,
    pub source_id: String,
    pub source_kind: SourceKind,
    pub summary: String,
    pub digest: String,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct Entity {
    pub id: EntityId,
    pub kind: EntityKind,
    pub canonical_name: String,
    pub aliases: BTreeSet<EntityId>,
    pub attributes: BTreeMap<String, AttributeResolution>,
    pub evidence_ids: BTreeSet<String>,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, PartialOrd, Ord)]
pub enum EdgeKind {
    Supports,
    Contradicts,
    SameAs,
}

#[derive(Debug, Clone, PartialEq, Eq, PartialOrd, Ord)]
pub struct GraphEdge {
    pub kind: EdgeKind,
    pub from: EntityId,
    pub to: EntityId,
    pub attribute: String,
    pub evidence_ids: BTreeSet<String>,
    pub note: String,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct EvidenceGraph {
    sources: BTreeMap<String, SourceRecord>,
    observations: BTreeMap<String, Observation>,
    evidence: BTreeMap<String, Evidence>,
    entities: BTreeMap<EntityId, Entity>,
    edges: Vec<GraphEdge>,
    ingested_calls: BTreeMap<String, String>,
    merged_aliases: BTreeMap<EntityId, EntityId>,
    pending_aliases: BTreeMap<(EntityId, EntityId), (EntityKind, BTreeSet<String>)>,
    duplicate_ingest_skips: usize,
}

impl Default for EvidenceGraph {
    fn default() -> Self {
        Self::new()
    }
}

impl EvidenceGraph {
    pub fn new() -> Self {
        Self {
            sources: BTreeMap::new(),
            observations: BTreeMap::new(),
            evidence: BTreeMap::new(),
            entities: BTreeMap::new(),
            edges: Vec::new(),
            ingested_calls: BTreeMap::new(),
            merged_aliases: BTreeMap::new(),
            pending_aliases: BTreeMap::new(),
            duplicate_ingest_skips: 0,
        }
    }

    pub fn sources(&self) -> &BTreeMap<String, SourceRecord> {
        &self.sources
    }

    pub fn observations(&self) -> &BTreeMap<String, Observation> {
        &self.observations
    }

    pub fn evidence(&self) -> &BTreeMap<String, Evidence> {
        &self.evidence
    }

    pub fn entities(&self) -> &BTreeMap<EntityId, Entity> {
        &self.entities
    }

    pub fn edges(&self) -> &[GraphEdge] {
        &self.edges
    }

    pub fn merged_aliases(&self) -> &BTreeMap<EntityId, EntityId> {
        &self.merged_aliases
    }

    pub fn duplicate_ingest_skips(&self) -> usize {
        self.duplicate_ingest_skips
    }

    pub fn resolve_entity_id(&self, id: &EntityId) -> EntityId {
        let mut current = id.clone();
        let mut hops = 0usize;
        while let Some(next) = self.merged_aliases.get(&current) {
            if next == &current || hops > self.merged_aliases.len() {
                break;
            }
            current = next.clone();
            hops += 1;
        }
        current
    }

    fn push_edge_unique(&mut self, edge: GraphEdge) {
        if let Some(existing) = self.edges.iter_mut().find(|e| {
            e.kind == edge.kind
                && e.from == edge.from
                && e.to == edge.to
                && e.attribute == edge.attribute
        }) {
            for ev in edge.evidence_ids {
                existing.evidence_ids.insert(ev);
            }
        } else {
            self.edges.push(edge);
            self.edges.sort();
        }
    }

    /// Nạp `ToolResult` vào đồ thị theo cơ chế nguyên tử và lũy đẳng: cùng `call_id` + `raw_digest`
    /// không bao giờ tạo bản sao thực thể, quan sát, bằng chứng hay cạnh; nếu có lỗi giữa chừng,
    /// trạng thái đồ thị giữ nguyên như trước khi gọi.
    pub fn ingest_result(
        &mut self,
        scope: &OsintScope,
        result: &ToolResult,
    ) -> Result<(), OsintError> {
        let computed = compute_result_digest(
            result.source,
            &result.canonical_target,
            &result.captured_at,
            &result.records,
        );
        if computed != result.raw_digest {
            return Err(OsintError::MalformedResult(format!(
                "raw_digest không khớp cho call_id '{}'",
                result.call_id
            )));
        }
        if let Some(existing_digest) = self.ingested_calls.get(&result.call_id) {
            if existing_digest == &result.raw_digest {
                self.duplicate_ingest_skips += 1;
                return Ok(());
            }
            return Err(OsintError::DuplicateSource(format!(
                "call_id '{}' trùng nhưng khác raw_digest",
                result.call_id
            )));
        }

        for rec in &result.records {
            validate_raw_record(scope, rec)?;
        }

        let mut staged = self.clone();
        let source_id = format!("src:{}", result.source.as_str());
        staged
            .sources
            .entry(source_id.clone())
            .or_insert_with(|| SourceRecord {
                id: source_id.clone(),
                kind: result.source,
                label: format!("synthetic-mock-{}", result.source.as_str()),
            });

        let register_alias = |graph: &mut EvidenceGraph,
                              from_raw: &EntityId,
                              from_kind: EntityKind,
                              primary_raw: &EntityId,
                              ev_id: String|
         -> Result<(), OsintError> {
            let resolved_primary = graph.resolve_entity_id(primary_raw);
            if let Some(primary) = graph.entities.get(&resolved_primary) {
                if primary.kind != from_kind {
                    return Err(OsintError::AliasMismatch(format!(
                        "{} ({:?}) không khớp kiểu với {} ({:?})",
                        from_raw.0, from_kind, resolved_primary.0, primary.kind
                    )));
                }
                let mut evs = BTreeSet::new();
                evs.insert(ev_id);
                graph.push_edge_unique(GraphEdge {
                    kind: EdgeKind::SameAs,
                    from: from_raw.clone(),
                    to: primary_raw.clone(),
                    attribute: "identity_alias".to_string(),
                    evidence_ids: evs,
                    note: format!("{} là bí danh của {}", from_raw.0, primary_raw.0),
                });
            } else {
                let entry = graph
                    .pending_aliases
                    .entry((from_raw.clone(), resolved_primary))
                    .or_insert_with(|| (from_kind, BTreeSet::new()));
                if entry.0 != from_kind {
                    return Err(OsintError::AliasMismatch(format!(
                        "mâu thuẫn kiểu alias cho {}",
                        from_raw.0
                    )));
                }
                entry.1.insert(ev_id);
            }
            Ok(())
        };

        for (idx, rec) in result.records.iter().enumerate() {
            let target_eid = staged.resolve_entity_id(&rec.entity_id);
            let duplicate_ev_id = staged
                .observations
                .values()
                .find(|existing| {
                    existing.source_id == source_id
                        && staged.resolve_entity_id(&existing.entity_id) == target_eid
                        && existing.attribute == rec.attribute
                        && existing.value == rec.value
                        && existing.observed_at == rec.observed_at
                })
                .and_then(|existing| {
                    staged
                        .evidence
                        .values()
                        .find(|ev| ev.observation_id == existing.id)
                        .map(|ev| ev.id.clone())
                });
            if let Some(existing_ev_id) = duplicate_ev_id {
                if let Some(primary_id) = &rec.alias_of {
                    register_alias(
                        &mut staged,
                        &rec.entity_id,
                        rec.entity_kind,
                        primary_id,
                        existing_ev_id,
                    )?;
                }
                continue;
            }
            let obs_id = format!("obs:{}:{}", result.call_id, idx);
            let ev_id = format!("ev:{}:{}", result.call_id, idx);

            let obs = Observation {
                id: obs_id.clone(),
                source_id: source_id.clone(),
                source_kind: result.source,
                entity_id: rec.entity_id.clone(),
                attribute: rec.attribute.clone(),
                value: rec.value.clone(),
                observed_at: rec.observed_at.clone(),
                weight_milli: rec.weight_milli,
                alias_of: rec.alias_of.clone(),
            };
            let ev = Evidence {
                id: ev_id.clone(),
                observation_id: obs_id.clone(),
                source_id: source_id.clone(),
                source_kind: result.source,
                summary: format!(
                    "{}: {}.{} = {} @ {}",
                    result.source.as_str(),
                    rec.entity_id.0,
                    rec.attribute,
                    rec.value,
                    rec.observed_at
                ),
                digest: result.raw_digest.clone(),
            };

            staged.observations.insert(obs_id, obs);
            staged.evidence.insert(ev_id.clone(), ev);

            let attr_obs = AttributeObservation {
                value: rec.value.clone(),
                observed_at: rec.observed_at.clone(),
                source_kind: result.source,
                weight_milli: rec.weight_milli,
                evidence_id: ev_id.clone(),
            };

            let became_conflicted: Option<(Vec<String>, BTreeSet<String>)> =
                if let Some(entity) = staged.entities.get_mut(&target_eid) {
                    if entity.kind != rec.entity_kind {
                        return Err(OsintError::AliasMismatch(format!(
                            "thực thể '{}' đang là {:?}, nhận {:?}",
                            target_eid.0, entity.kind, rec.entity_kind
                        )));
                    }
                    entity.evidence_ids.insert(ev_id.clone());
                    if let Some(res) = entity.attributes.get_mut(&rec.attribute) {
                        res.add_candidate(attr_obs)?;
                        if res.status == ResolutionStatus::Conflicted {
                            let vals: Vec<String> = res
                                .candidates
                                .iter()
                                .map(|c| c.value.clone())
                                .collect::<BTreeSet<_>>()
                                .into_iter()
                                .collect();
                            let evs: BTreeSet<String> = res
                                .candidates
                                .iter()
                                .map(|c| c.evidence_id.clone())
                                .collect();
                            Some((vals, evs))
                        } else {
                            None
                        }
                    } else {
                        entity.attributes.insert(
                            rec.attribute.clone(),
                            AttributeResolution::from_first(&rec.attribute, attr_obs)?,
                        );
                        None
                    }
                } else {
                    let mut attributes = BTreeMap::new();
                    attributes.insert(
                        rec.attribute.clone(),
                        AttributeResolution::from_first(&rec.attribute, attr_obs)?,
                    );
                    let mut evidence_ids = BTreeSet::new();
                    evidence_ids.insert(ev_id.clone());
                    staged.entities.insert(
                        target_eid.clone(),
                        Entity {
                            id: target_eid.clone(),
                            kind: rec.entity_kind,
                            canonical_name: rec.canonical_name.clone(),
                            aliases: BTreeSet::new(),
                            attributes,
                            evidence_ids,
                        },
                    );
                    None
                };

            let ready_pending: Vec<(EntityId, EntityId)> = staged
                .pending_aliases
                .keys()
                .filter(|(_, to)| staged.resolve_entity_id(to) == target_eid)
                .cloned()
                .collect();
            for key in ready_pending {
                if let Some((from_kind, evs)) = staged.pending_aliases.remove(&key) {
                    if from_kind != rec.entity_kind {
                        return Err(OsintError::AliasMismatch(format!(
                            "{} ({:?}) không khớp kiểu với {} ({:?})",
                            key.0.0, from_kind, target_eid.0, rec.entity_kind
                        )));
                    }
                    staged.push_edge_unique(GraphEdge {
                        kind: EdgeKind::SameAs,
                        from: key.0.clone(),
                        to: key.1.clone(),
                        attribute: "identity_alias".to_string(),
                        evidence_ids: evs,
                        note: format!("{} là bí danh của {}", key.0.0, key.1.0),
                    });
                }
            }

            if let Some((vals, evs)) = became_conflicted {
                staged.push_edge_unique(GraphEdge {
                    kind: EdgeKind::Contradicts,
                    from: target_eid.clone(),
                    to: target_eid.clone(),
                    attribute: rec.attribute.clone(),
                    evidence_ids: evs,
                    note: format!(
                        "mâu thuẫn thuộc tính '{}' trên {}: {}",
                        rec.attribute,
                        target_eid.0,
                        vals.join(" vs ")
                    ),
                });
            }

            if let Some(primary_id) = &rec.alias_of {
                register_alias(
                    &mut staged,
                    &rec.entity_id,
                    rec.entity_kind,
                    primary_id,
                    ev_id,
                )?;
            }
        }

        staged
            .ingested_calls
            .insert(result.call_id.clone(), result.raw_digest.clone());
        *self = staged;
        Ok(())
    }

    /// Gộp các thực thể được nối bởi cạnh `SameAs`, giữ nguyên bằng chứng không nhân đôi,
    /// khử trùng quan sát cùng nguồn trên hai bí danh và bảo toàn mâu thuẫn thuộc tính.
    pub fn merge_same_as_entities(&mut self) -> Result<usize, OsintError> {
        let same_as_pairs: Vec<(EntityId, EntityId)> = self
            .edges
            .iter()
            .filter(|e| e.kind == EdgeKind::SameAs)
            .map(|e| (e.from.clone(), e.to.clone()))
            .collect();

        let mut merged_count = 0usize;
        for (raw_alias, raw_primary) in same_as_pairs {
            let alias_id = self.resolve_entity_id(&raw_alias);
            let primary_id = self.resolve_entity_id(&raw_primary);
            if alias_id == primary_id {
                continue;
            }
            let Some(alias_entity) = self.entities.get(&alias_id).cloned() else {
                continue;
            };
            let Some(primary_entity) = self.entities.get(&primary_id).cloned() else {
                return Err(OsintError::EntityNotFound(primary_id));
            };
            if alias_entity.kind != primary_entity.kind {
                return Err(OsintError::AliasMismatch(format!(
                    "không thể gộp {:?} '{}' vào {:?} '{}'",
                    alias_entity.kind, alias_id.0, primary_entity.kind, primary_id.0
                )));
            }

            let removed = self
                .entities
                .remove(&alias_id)
                .ok_or_else(|| OsintError::EntityNotFound(alias_id.clone()))?;
            self.merged_aliases
                .insert(alias_id.clone(), primary_id.clone());

            let mut new_conflicts = Vec::new();
            {
                let target = self
                    .entities
                    .get_mut(&primary_id)
                    .ok_or_else(|| OsintError::EntityNotFound(primary_id.clone()))?;
                target.aliases.insert(alias_id.clone());
                for sub_alias in removed.aliases {
                    target.aliases.insert(sub_alias);
                }
                for ev in removed.evidence_ids {
                    target.evidence_ids.insert(ev);
                }
                for (attr_name, alias_res) in removed.attributes {
                    if let Some(existing) = target.attributes.get_mut(&attr_name) {
                        for cand in alias_res.candidates {
                            existing.add_candidate(cand)?;
                        }
                        if existing.status == ResolutionStatus::Conflicted {
                            let vals: Vec<String> = existing
                                .candidates
                                .iter()
                                .map(|c| c.value.clone())
                                .collect::<BTreeSet<_>>()
                                .into_iter()
                                .collect();
                            let evs: BTreeSet<String> = existing
                                .candidates
                                .iter()
                                .map(|c| c.evidence_id.clone())
                                .collect();
                            new_conflicts.push((attr_name, vals, evs));
                        }
                    } else {
                        target.attributes.insert(attr_name, alias_res);
                    }
                }
            }

            type ObsDedupKey = (String, String, String, String);
            let mut canonical_by_key: BTreeMap<ObsDedupKey, String> = BTreeMap::new();
            let mut obs_for_primary: Vec<(bool, String, ObsDedupKey)> = Vec::new();
            for (obs_id, obs) in &self.observations {
                if self.resolve_entity_id(&obs.entity_id) == primary_id {
                    let key = (
                        obs.source_id.clone(),
                        obs.attribute.clone(),
                        obs.value.clone(),
                        obs.observed_at.clone(),
                    );
                    let is_primary_native = obs.entity_id == primary_id;
                    obs_for_primary.push((!is_primary_native, obs_id.clone(), key));
                }
            }
            obs_for_primary.sort();

            let mut ev_remap: BTreeMap<String, String> = BTreeMap::new();
            let mut dup_obs_ids: Vec<String> = Vec::new();
            for (_, obs_id, key) in obs_for_primary {
                let Some(ev_id) = self
                    .evidence
                    .values()
                    .find(|ev| ev.observation_id == obs_id)
                    .map(|ev| ev.id.clone())
                else {
                    continue;
                };
                if let Some(canon_ev) = canonical_by_key.get(&key) {
                    ev_remap.insert(ev_id, canon_ev.clone());
                    dup_obs_ids.push(obs_id);
                } else {
                    canonical_by_key.insert(key, ev_id);
                }
            }

            if !ev_remap.is_empty() {
                if let Some(target) = self.entities.get_mut(&primary_id) {
                    for (dup_ev, canon_ev) in &ev_remap {
                        if target.evidence_ids.remove(dup_ev) {
                            target.evidence_ids.insert(canon_ev.clone());
                        }
                    }
                    for res in target.attributes.values_mut() {
                        let mut seen_evs = BTreeSet::new();
                        let mut deduped = Vec::new();
                        for mut cand in std::mem::take(&mut res.candidates) {
                            if let Some(canon_ev) = ev_remap.get(&cand.evidence_id) {
                                cand.evidence_id = canon_ev.clone();
                            }
                            if seen_evs.insert(cand.evidence_id.clone()) {
                                deduped.push(cand);
                            }
                        }
                        deduped.sort_by(|a, b| a.evidence_id.cmp(&b.evidence_id));
                        res.candidates = deduped;
                        res.recompute()?;
                    }
                }
                for edge in &mut self.edges {
                    let mut updated = BTreeSet::new();
                    for ev in &edge.evidence_ids {
                        updated.insert(ev_remap.get(ev).cloned().unwrap_or_else(|| ev.clone()));
                    }
                    edge.evidence_ids = updated;
                }
                for (_, evs) in self.pending_aliases.values_mut() {
                    let mut updated = BTreeSet::new();
                    for ev in evs.iter() {
                        updated.insert(ev_remap.get(ev).cloned().unwrap_or_else(|| ev.clone()));
                    }
                    *evs = updated;
                }
                for dup_ev in ev_remap.keys() {
                    self.evidence.remove(dup_ev);
                }
                for dup_obs in dup_obs_ids {
                    self.observations.remove(&dup_obs);
                }
            }

            for (attr_name, vals, evs) in new_conflicts {
                let remapped_evs: BTreeSet<String> = evs
                    .into_iter()
                    .map(|e| ev_remap.get(&e).cloned().unwrap_or(e))
                    .collect();
                self.push_edge_unique(GraphEdge {
                    kind: EdgeKind::Contradicts,
                    from: primary_id.clone(),
                    to: primary_id.clone(),
                    attribute: attr_name.clone(),
                    evidence_ids: remapped_evs,
                    note: format!(
                        "mâu thuẫn sau hợp nhất '{}' trên {}: {}",
                        attr_name,
                        primary_id.0,
                        vals.join(" vs ")
                    ),
                });
            }
            merged_count += 1;
        }
        self.build_support_edges();
        Ok(merged_count)
    }

    fn build_support_edges(&mut self) {
        let mut additions = Vec::new();
        for entity in self.entities.values() {
            for (attr, res) in &entity.attributes {
                for cand in &res.candidates {
                    let target_opt = match attr.as_str() {
                        "official_domain" | "san_domain" | "hosted_domain" | "cname_target"
                        | "mx_record" | "referenced_domain" => {
                            Some(EntityId(format!("domain:{}", cand.value)))
                        }
                        "a_record" => Some(EntityId(format!("ip:{}", cand.value))),
                        "role_contact" => Some(EntityId(format!("role:{}", cand.value))),
                        _ => None,
                    };
                    if let Some(raw_to) = target_opt {
                        let to = self.resolve_entity_id(&raw_to);
                        if self.entities.contains_key(&to) && to != entity.id {
                            let mut evs = BTreeSet::new();
                            evs.insert(cand.evidence_id.clone());
                            additions.push(GraphEdge {
                                kind: EdgeKind::Supports,
                                from: entity.id.clone(),
                                to,
                                attribute: attr.clone(),
                                evidence_ids: evs,
                                note: format!("{} -> {} ({})", entity.id.0, cand.value, attr),
                            });
                        }
                    }
                }
            }
        }
        for edge in additions {
            self.push_edge_unique(edge);
        }
    }

    /// Kiểm chứng toàn vẹn khóa ngoại, confidence bounds, provenance và mâu thuẫn trên đồ thị.
    pub fn validate(&self) -> Result<(), OsintError> {
        let mut seen_obs_keys = BTreeSet::new();
        for (ev_id, ev) in &self.evidence {
            if &ev.id != ev_id || ev.summary.trim().is_empty() || ev.digest.trim().is_empty() {
                return Err(OsintError::InvalidGraph(format!(
                    "bằng chứng '{ev_id}' thiếu trường bắt buộc"
                )));
            }
            let obs = self.observations.get(&ev.observation_id).ok_or_else(|| {
                OsintError::InvalidGraph(format!(
                    "bằng chứng '{ev_id}' trỏ tới observation '{}' không tồn tại",
                    ev.observation_id
                ))
            })?;
            let src = self.sources.get(&ev.source_id).ok_or_else(|| {
                OsintError::InvalidGraph(format!(
                    "bằng chứng '{ev_id}' trỏ tới source '{}' không tồn tại",
                    ev.source_id
                ))
            })?;
            if src.kind != ev.source_kind || obs.source_id != ev.source_id {
                return Err(OsintError::InvalidGraph(format!(
                    "không khớp source giữa evidence '{ev_id}' và observation '{}'",
                    obs.id
                )));
            }
        }

        for (obs_id, obs) in &self.observations {
            if &obs.id != obs_id {
                return Err(OsintError::InvalidGraph(format!(
                    "observation key '{obs_id}' không khớp id"
                )));
            }
            if !self.sources.contains_key(&obs.source_id) {
                return Err(OsintError::InvalidGraph(format!(
                    "observation '{obs_id}' thiếu source '{}'",
                    obs.source_id
                )));
            }
            let resolved_eid = self.resolve_entity_id(&obs.entity_id);
            let entity = self.entities.get(&resolved_eid).ok_or_else(|| {
                OsintError::InvalidGraph(format!(
                    "observation '{obs_id}' trỏ tới thực thể '{}' không tồn tại",
                    resolved_eid.0
                ))
            })?;
            let has_matching_ev = entity.evidence_ids.iter().any(|eid| {
                self.evidence
                    .get(eid)
                    .is_some_and(|ev| ev.observation_id == *obs_id)
            });
            if !has_matching_ev {
                return Err(OsintError::InvalidGraph(format!(
                    "thực thể '{}' thiếu evidence cho observation '{obs_id}'",
                    resolved_eid.0
                )));
            }
            let obs_key = (
                obs.source_id.clone(),
                resolved_eid.0,
                obs.attribute.clone(),
                obs.value.clone(),
                obs.observed_at.clone(),
            );
            if !seen_obs_keys.insert(obs_key) {
                return Err(OsintError::InvalidGraph(format!(
                    "phát hiện quan sát trùng lặp tại observation '{obs_id}'"
                )));
            }
            let attr_lower = obs.attribute.to_ascii_lowercase();
            if PROHIBITED_PII_ATTRIBUTES.contains(&attr_lower.as_str()) {
                return Err(OsintError::PersonalDataProhibited(obs.attribute.clone()));
            }
            validate_no_raw_secret(&obs.value, Some(&obs.attribute))?;
        }

        for (eid, entity) in &self.entities {
            if &entity.id != eid {
                return Err(OsintError::InvalidGraph(format!(
                    "entity key '{}' không khớp id '{}'",
                    eid.0, entity.id.0
                )));
            }
            if entity.evidence_ids.is_empty() {
                return Err(OsintError::InvalidGraph(format!(
                    "thực thể '{}' không có bằng chứng nào",
                    eid.0
                )));
            }
            for ev_id in &entity.evidence_ids {
                if !self.evidence.contains_key(ev_id) {
                    return Err(OsintError::InvalidGraph(format!(
                        "thực thể '{}' tham chiếu evidence không tồn tại '{ev_id}'",
                        eid.0
                    )));
                }
            }
            for (attr_name, res) in &entity.attributes {
                if res.confidence.milli() > 1000 {
                    return Err(OsintError::InvalidConfidence(format!(
                        "confidence trên {}.{} vượt 1000",
                        eid.0, attr_name
                    )));
                }
                let distinct_vals: BTreeSet<&str> =
                    res.candidates.iter().map(|c| c.value.as_str()).collect();
                if distinct_vals.len() > 1 && res.status != ResolutionStatus::Conflicted {
                    return Err(OsintError::InvalidGraph(format!(
                        "thuộc tính đa giá trị {}.{} phải ở trạng thái Conflicted",
                        eid.0, attr_name
                    )));
                }
                if res.status == ResolutionStatus::Conflicted {
                    let has_contradict_edge = self.edges.iter().any(|e| {
                        e.kind == EdgeKind::Contradicts
                            && self.resolve_entity_id(&e.from) == *eid
                            && e.attribute == *attr_name
                    });
                    if !has_contradict_edge {
                        return Err(OsintError::InvalidGraph(format!(
                            "thuộc tính Conflicted {}.{} thiếu cạnh Contradicts",
                            eid.0, attr_name
                        )));
                    }
                }
                for cand in &res.candidates {
                    if !self.evidence.contains_key(&cand.evidence_id) {
                        return Err(OsintError::InvalidGraph(format!(
                            "candidate thuộc tính {}.{} thiếu evidence '{}'",
                            eid.0, attr_name, cand.evidence_id
                        )));
                    }
                }
            }
        }

        for edge in &self.edges {
            if edge.evidence_ids.is_empty() {
                return Err(OsintError::InvalidGraph(format!(
                    "cạnh {:?} ({} -> {}) không có evidence_ids",
                    edge.kind, edge.from.0, edge.to.0
                )));
            }
            for ev_id in &edge.evidence_ids {
                if !self.evidence.contains_key(ev_id) {
                    return Err(OsintError::InvalidGraph(format!(
                        "cạnh {:?} tham chiếu evidence giả '{ev_id}'",
                        edge.kind
                    )));
                }
            }
            let from_resolved = self.resolve_entity_id(&edge.from);
            let to_resolved = self.resolve_entity_id(&edge.to);
            if !self.entities.contains_key(&from_resolved)
                || !self.entities.contains_key(&to_resolved)
            {
                return Err(OsintError::InvalidGraph(format!(
                    "cạnh {:?} trỏ tới thực thể không tồn tại ({} -> {})",
                    edge.kind, edge.from.0, edge.to.0
                )));
            }
        }

        for ((from, _to), (_, ev_ids)) in &self.pending_aliases {
            let from_resolved = self.resolve_entity_id(from);
            if !self.entities.contains_key(&from_resolved) || ev_ids.is_empty() {
                return Err(OsintError::InvalidGraph(format!(
                    "pending alias '{}' không hợp lệ",
                    from.0
                )));
            }
            for ev_id in ev_ids {
                if !self.evidence.contains_key(ev_id) {
                    return Err(OsintError::InvalidGraph(format!(
                        "pending alias '{}' thiếu evidence '{ev_id}'",
                        from.0
                    )));
                }
            }
        }

        Ok(())
    }
}

// ============================================================================
// 5. STAGE, REACT STATE, CHECKPOINT & BÁO CÁO TÁI LẬP
// ============================================================================

#[derive(Debug, Clone, Copy, PartialEq, Eq, PartialOrd, Ord)]
pub enum Stage {
    ScopeValidation,
    SeedCollection,
    PassiveExpansion,
    EntityResolution,
    GraphVerification,
    ReportGeneration,
}

impl Stage {
    pub const ALL: [Self; 6] = [
        Self::ScopeValidation,
        Self::SeedCollection,
        Self::PassiveExpansion,
        Self::EntityResolution,
        Self::GraphVerification,
        Self::ReportGeneration,
    ];
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, PartialOrd, Ord)]
pub enum StageStatus {
    Pending,
    InProgress,
    Completed,
    Halted,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, PartialOrd, Ord)]
pub enum StopReason {
    Completed,
    BudgetExhausted,
    CyclePrevented,
    ScopeBlocked,
    SourceFailure,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub enum AgentAction {
    ValidateScope,
    CallTool(ToolRequest),
    SkipVisited { source: SourceKind, target: String },
    ResolveEntities,
    VerifyGraph,
    BuildReport,
    Halt(StopReason),
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct AgentState {
    pub current_stage: Stage,
    pub steps_used: usize,
    pub tool_calls_used: usize,
    pub seeds_remaining: usize,
    pub pending_targets: VecDeque<(ToolTargetKind, String)>,
    pub visited_calls: BTreeSet<(SourceKind, String)>,
    pub action_trace: Vec<AgentAction>,
    pub stop_reason: Option<StopReason>,
}

impl AgentState {
    fn new() -> Self {
        Self {
            current_stage: Stage::ScopeValidation,
            steps_used: 0,
            tool_calls_used: 0,
            seeds_remaining: 0,
            pending_targets: VecDeque::new(),
            visited_calls: BTreeSet::new(),
            action_trace: Vec::new(),
            stop_reason: None,
        }
    }
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct Checkpoint {
    pub version: u32,
    pub engagement_id: String,
    pub input_hash: String,
    pub stage_statuses: BTreeMap<Stage, StageStatus>,
    pub state: AgentState,
    pub completed_calls: BTreeMap<String, ToolResult>,
    pub graph: EvidenceGraph,
}

#[derive(Debug, Clone, PartialEq, Eq, PartialOrd, Ord)]
pub struct ReportClaim {
    pub claim_id: String,
    pub entity_id: EntityId,
    pub attribute: String,
    pub claimed_value: String,
    pub statement: String,
    pub confidence: Confidence,
    pub evidence_ids: BTreeSet<String>,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct ConflictCandidate {
    pub value: String,
    pub observed_at: String,
    pub source_kind: SourceKind,
    pub evidence_id: String,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct ConflictRecord {
    pub entity_id: EntityId,
    pub attribute: String,
    pub candidates: Vec<ConflictCandidate>,
    pub confidence: Confidence,
    pub evidence_ids: BTreeSet<String>,
    pub explanation: String,
}

#[derive(Debug, Clone, PartialEq, Eq, PartialOrd, Ord)]
pub struct UnknownGap {
    pub entity_id: Option<EntityId>,
    pub topic: String,
    pub reason: String,
}

#[derive(Debug, Clone, PartialEq, Eq, PartialOrd, Ord)]
pub struct DefensiveRemediation {
    pub target: String,
    pub control: String,
    pub action: String,
    pub evidence_ids: BTreeSet<String>,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct OsintReport {
    pub schema_version: String,
    pub engagement_id: String,
    pub generated_at: String,
    pub input_hash: String,
    pub stop_reason: StopReason,
    pub facts: Vec<ReportClaim>,
    pub inferences: Vec<ReportClaim>,
    pub conflicts: Vec<ConflictRecord>,
    pub unknowns: Vec<UnknownGap>,
    pub remediations: Vec<DefensiveRemediation>,
}

fn json_escape(s: &str) -> String {
    let mut out = String::with_capacity(s.len());
    for ch in s.chars() {
        match ch {
            '"' => out.push_str("\\\""),
            '\\' => out.push_str("\\\\"),
            '\n' => out.push_str("\\n"),
            '\r' => out.push_str("\\r"),
            '\t' => out.push_str("\\t"),
            '\u{08}' => out.push_str("\\b"),
            '\u{0c}' => out.push_str("\\f"),
            c if (c as u32) < 0x20 => {
                use std::fmt::Write;
                let _ = write!(&mut out, "\\u{:04x}", c as u32);
            }
            c => out.push(c),
        }
    }
    out
}

fn markdown_escape(s: &str) -> String {
    let mut out = String::with_capacity(s.len());
    for ch in s.chars() {
        match ch {
            '\\' => out.push_str("\\\\"),
            '|' => out.push_str("\\|"),
            '<' => out.push_str("&lt;"),
            '>' => out.push_str("&gt;"),
            '`' => out.push_str("\\`"),
            '\n' | '\r' => out.push(' '),
            c => out.push(c),
        }
    }
    out
}

impl OsintReport {
    /// Kiểm tra toàn bộ bất biến trích dẫn, chống nâng khống mâu thuẫn và kiểm soát rò rỉ secret.
    pub fn validate(&self, graph: &EvidenceGraph) -> Result<(), OsintError> {
        if self.schema_version != REPORT_SCHEMA_VERSION {
            return Err(OsintError::ReportSchemaInvalid(format!(
                "schema_version '{}' khác '{}'",
                self.schema_version, REPORT_SCHEMA_VERSION
            )));
        }
        if !is_valid_iso8601_utc(&self.generated_at) || self.input_hash.trim().is_empty() {
            return Err(OsintError::ReportSchemaInvalid(
                "generated_at hoặc input_hash không hợp lệ".to_string(),
            ));
        }

        let mut seen_claim_ids = BTreeSet::new();
        for fact in &self.facts {
            if fact.claim_id.trim().is_empty() || fact.statement.trim().is_empty() {
                return Err(OsintError::ReportSchemaInvalid(
                    "fact claim_id hoặc statement không được rỗng".to_string(),
                ));
            }
            if !seen_claim_ids.insert(&fact.claim_id) {
                return Err(OsintError::ReportSchemaInvalid(format!(
                    "trùng claim_id '{}'",
                    fact.claim_id
                )));
            }
            validate_no_raw_secret(&fact.statement, None)?;
            validate_no_raw_secret(&fact.claimed_value, Some(&fact.attribute))?;
            if fact.evidence_ids.is_empty() {
                return Err(OsintError::UnsupportedClaim(format!(
                    "fact '{}' thiếu evidence_ids",
                    fact.claim_id
                )));
            }
            let resolved_eid = graph.resolve_entity_id(&fact.entity_id);
            let entity = graph.entities().get(&resolved_eid).ok_or_else(|| {
                OsintError::UnsupportedClaim(format!(
                    "fact '{}' trỏ tới thực thể không tồn tại '{}'",
                    fact.claim_id, resolved_eid.0
                ))
            })?;
            let attr_res = entity.attributes.get(&fact.attribute).ok_or_else(|| {
                OsintError::UnsupportedClaim(format!(
                    "fact '{}' tham chiếu thuộc tính không tồn tại '{}.{}'",
                    fact.claim_id, resolved_eid.0, fact.attribute
                ))
            })?;
            match attr_res.status {
                ResolutionStatus::Conflicted => {
                    return Err(OsintError::ConflictedFactClaim {
                        entity_id: resolved_eid,
                        attribute: fact.attribute.clone(),
                    });
                }
                ResolutionStatus::Unknown => {
                    return Err(OsintError::UnsupportedClaim(format!(
                        "thuộc tính '{}.{}' đang ở trạng thái Unknown",
                        resolved_eid.0, fact.attribute
                    )));
                }
                ResolutionStatus::Confirmed => {
                    if attr_res.resolved_value.as_deref() != Some(fact.claimed_value.as_str()) {
                        return Err(OsintError::UnsupportedClaim(format!(
                            "giá trị fact '{}' không khớp giá trị xác nhận trên đồ thị",
                            fact.claim_id
                        )));
                    }
                }
            }
            let attr_evs: BTreeSet<&str> = attr_res
                .candidates
                .iter()
                .map(|c| c.evidence_id.as_str())
                .collect();
            for ev_id in &fact.evidence_ids {
                if !graph.evidence().contains_key(ev_id) || !attr_evs.contains(ev_id.as_str()) {
                    return Err(OsintError::UnsupportedClaim(format!(
                        "fact '{}' dùng evidence '{ev_id}' không hỗ trợ trực tiếp cho thuộc tính '{}.{}'",
                        fact.claim_id, resolved_eid.0, fact.attribute
                    )));
                }
            }
        }

        for inf in &self.inferences {
            if inf.claim_id.trim().is_empty() || inf.statement.trim().is_empty() {
                return Err(OsintError::ReportSchemaInvalid(
                    "inference claim_id hoặc statement không được rỗng".to_string(),
                ));
            }
            if !seen_claim_ids.insert(&inf.claim_id) {
                return Err(OsintError::ReportSchemaInvalid(format!(
                    "trùng claim_id '{}'",
                    inf.claim_id
                )));
            }
            validate_no_raw_secret(&inf.statement, None)?;
            validate_no_raw_secret(&inf.claimed_value, Some(&inf.attribute))?;
            let inf_attr_lower = inf.attribute.to_ascii_lowercase();
            if (inf_attr_lower == "a_record" || inf_attr_lower == "ip_address")
                && !is_rfc1918_ipv4(&inf.claimed_value)
            {
                return Err(OsintError::InvalidRfc1918Ip(inf.claimed_value.clone()));
            }
            if [
                "san_domain",
                "cname_target",
                "mx_record",
                "hosted_domain",
                "referenced_domain",
                "official_domain",
                "domain",
            ]
            .contains(&inf_attr_lower.as_str())
            {
                normalize_example_domain(&inf.claimed_value)?;
            }
            if inf.evidence_ids.len() < 2 {
                return Err(OsintError::UnsupportedClaim(format!(
                    "inference '{}' phải có tối thiểu 2 bằng chứng độc lập",
                    inf.claim_id
                )));
            }
            let resolved_eid = graph.resolve_entity_id(&inf.entity_id);
            let entity = graph.entities().get(&resolved_eid).ok_or_else(|| {
                OsintError::UnsupportedClaim(format!(
                    "inference '{}' trỏ tới thực thể không tồn tại '{}'",
                    inf.claim_id, resolved_eid.0
                ))
            })?;
            let attr_res = entity.attributes.get(&inf.attribute).ok_or_else(|| {
                OsintError::UnsupportedClaim(format!(
                    "inference '{}' tham chiếu thuộc tính không tồn tại '{}.{}'",
                    inf.claim_id, resolved_eid.0, inf.attribute
                ))
            })?;
            match attr_res.status {
                ResolutionStatus::Conflicted => {
                    return Err(OsintError::ConflictedFactClaim {
                        entity_id: resolved_eid,
                        attribute: inf.attribute.clone(),
                    });
                }
                ResolutionStatus::Unknown => {
                    return Err(OsintError::UnsupportedClaim(format!(
                        "inference '{}' tham chiếu thuộc tính Unknown '{}.{}'",
                        inf.claim_id, resolved_eid.0, inf.attribute
                    )));
                }
                ResolutionStatus::Confirmed => {
                    if attr_res.resolved_value.as_deref() != Some(inf.claimed_value.as_str()) {
                        return Err(OsintError::UnsupportedClaim(format!(
                            "giá trị inference '{}' không khớp giá trị trên đồ thị",
                            inf.claim_id
                        )));
                    }
                }
            }

            let mut connected_entities = BTreeSet::new();
            connected_entities.insert(resolved_eid.clone());
            for edge in graph.edges() {
                let from_r = graph.resolve_entity_id(&edge.from);
                let to_r = graph.resolve_entity_id(&edge.to);
                if from_r == resolved_eid {
                    connected_entities.insert(to_r);
                } else if to_r == resolved_eid {
                    connected_entities.insert(from_r);
                }
            }

            let mut has_direct_attr_ev = false;
            let direct_evs: BTreeSet<&str> = attr_res
                .candidates
                .iter()
                .map(|c| c.evidence_id.as_str())
                .collect();

            for ev_id in &inf.evidence_ids {
                let ev = graph.evidence().get(ev_id).ok_or_else(|| {
                    OsintError::UnsupportedClaim(format!(
                        "inference '{}' dùng evidence không tồn tại '{ev_id}'",
                        inf.claim_id
                    ))
                })?;
                let obs = graph
                    .observations()
                    .get(&ev.observation_id)
                    .ok_or_else(|| {
                        OsintError::UnsupportedClaim(format!(
                            "inference '{}' dùng evidence '{ev_id}' thiếu observation",
                            inf.claim_id
                        ))
                    })?;
                let obs_eid = graph.resolve_entity_id(&obs.entity_id);
                if !connected_entities.contains(&obs_eid) {
                    return Err(OsintError::UnsupportedClaim(format!(
                        "inference '{}' dùng evidence '{ev_id}' của thực thể không liên quan '{}'",
                        inf.claim_id, obs_eid.0
                    )));
                }
                if let Some(obs_entity) = graph.entities().get(&obs_eid)
                    && let Some(obs_attr) = obs_entity.attributes.get(&obs.attribute)
                    && obs_attr.status == ResolutionStatus::Conflicted
                {
                    return Err(OsintError::ConflictedFactClaim {
                        entity_id: obs_eid,
                        attribute: obs.attribute.clone(),
                    });
                }
                if direct_evs.contains(ev_id.as_str()) {
                    has_direct_attr_ev = true;
                }
            }
            if !has_direct_attr_ev {
                return Err(OsintError::UnsupportedClaim(format!(
                    "inference '{}' thiếu bằng chứng gắn trực tiếp với '{}.{}'",
                    inf.claim_id, resolved_eid.0, inf.attribute
                )));
            }
        }

        let mut seen_conflicts = BTreeSet::new();
        for conflict in &self.conflicts {
            validate_no_raw_secret(&conflict.explanation, None)?;
            validate_no_raw_secret(&conflict.attribute, None)?;
            if conflict.evidence_ids.len() < 2 || conflict.candidates.len() < 2 {
                return Err(OsintError::UnsupportedClaim(format!(
                    "conflict '{}.{}' phải có tối thiểu 2 ứng viên và 2 bằng chứng",
                    conflict.entity_id.0, conflict.attribute
                )));
            }
            let resolved_eid = graph.resolve_entity_id(&conflict.entity_id);
            if !seen_conflicts.insert((resolved_eid.clone(), conflict.attribute.clone())) {
                return Err(OsintError::ReportSchemaInvalid(format!(
                    "trùng bản ghi mâu thuẫn '{}.{}'",
                    resolved_eid.0, conflict.attribute
                )));
            }
            let entity = graph.entities().get(&resolved_eid).ok_or_else(|| {
                OsintError::UnsupportedClaim(format!(
                    "conflict trỏ tới thực thể không tồn tại '{}'",
                    resolved_eid.0
                ))
            })?;
            let attr = entity.attributes.get(&conflict.attribute).ok_or_else(|| {
                OsintError::UnsupportedClaim(format!(
                    "conflict thiếu thuộc tính '{}.{}'",
                    resolved_eid.0, conflict.attribute
                ))
            })?;
            if attr.status != ResolutionStatus::Conflicted {
                return Err(OsintError::UnsupportedClaim(format!(
                    "conflict '{}.{}' không ở trạng thái Conflicted trên đồ thị",
                    resolved_eid.0, conflict.attribute
                )));
            }
            let attr_evs: BTreeSet<&str> = attr
                .candidates
                .iter()
                .map(|c| c.evidence_id.as_str())
                .collect();
            for ev_id in &conflict.evidence_ids {
                if !graph.evidence().contains_key(ev_id) || !attr_evs.contains(ev_id.as_str()) {
                    return Err(OsintError::UnsupportedClaim(format!(
                        "conflict dùng evidence '{ev_id}' không thuộc thuộc tính mâu thuẫn"
                    )));
                }
            }
            let reported_evs: BTreeSet<&str> =
                conflict.evidence_ids.iter().map(String::as_str).collect();
            if reported_evs != attr_evs {
                return Err(OsintError::UnsupportedClaim(format!(
                    "conflict '{}.{}' không bao phủ đủ bằng chứng mâu thuẫn trên đồ thị",
                    resolved_eid.0, conflict.attribute
                )));
            }
            if conflict.candidates.len() != attr.candidates.len() {
                return Err(OsintError::UnsupportedClaim(format!(
                    "conflict '{}.{}' bị thiếu hoặc thừa ứng viên so với đồ thị",
                    resolved_eid.0, conflict.attribute
                )));
            }
            let mut seen_cand_evs = BTreeSet::new();
            for cand in &conflict.candidates {
                validate_no_raw_secret(&cand.value, Some(&conflict.attribute))?;
                if !seen_cand_evs.insert(cand.evidence_id.as_str()) {
                    return Err(OsintError::UnsupportedClaim(format!(
                        "ứng viên conflict '{}.{}' bị lặp evidence_id '{}'",
                        resolved_eid.0, conflict.attribute, cand.evidence_id
                    )));
                }
                let matches_graph = attr.candidates.iter().any(|g| {
                    g.value == cand.value
                        && g.observed_at == cand.observed_at
                        && g.source_kind == cand.source_kind
                        && g.evidence_id == cand.evidence_id
                });
                if !matches_graph {
                    return Err(OsintError::UnsupportedClaim(format!(
                        "ứng viên conflict '{}.{}' không khớp dữ liệu đồ thị",
                        resolved_eid.0, conflict.attribute
                    )));
                }
            }
            if seen_cand_evs != attr_evs {
                return Err(OsintError::UnsupportedClaim(format!(
                    "ứng viên conflict '{}.{}' không bao phủ đủ các bằng chứng mâu thuẫn",
                    resolved_eid.0, conflict.attribute
                )));
            }
        }

        // Không được xóa lặng lẽ bất kỳ thuộc tính Conflicted nào khỏi báo cáo.
        for (eid, entity) in graph.entities() {
            for (attr_name, res) in &entity.attributes {
                if res.status == ResolutionStatus::Conflicted
                    && !seen_conflicts.contains(&(eid.clone(), attr_name.clone()))
                {
                    return Err(OsintError::UnsupportedClaim(format!(
                        "báo cáo đã bỏ sót mâu thuẫn '{}.{}' trên đồ thị",
                        eid.0, attr_name
                    )));
                }
            }
        }

        for gap in &self.unknowns {
            validate_no_raw_secret(&gap.topic, None)?;
            validate_no_raw_secret(&gap.reason, None)?;
        }
        for rem in &self.remediations {
            validate_no_raw_secret(&rem.target, None)?;
            validate_no_raw_secret(&rem.control, None)?;
            validate_no_raw_secret(&rem.action, None)?;
            if rem.evidence_ids.is_empty() {
                return Err(OsintError::UnsupportedClaim(format!(
                    "remediation '{}' thiếu evidence_ids",
                    rem.target
                )));
            }
            for ev_id in &rem.evidence_ids {
                if !graph.evidence().contains_key(ev_id) {
                    return Err(OsintError::UnsupportedClaim(format!(
                        "remediation '{}' tham chiếu evidence không tồn tại '{ev_id}'",
                        rem.target
                    )));
                }
            }
        }

        Ok(())
    }

    pub fn render_markdown(&self) -> String {
        use std::fmt::Write;
        let mut out = String::new();
        let _ = writeln!(
            out,
            "# OSINT Passive Report ({})",
            markdown_escape(&self.engagement_id)
        );
        let _ = writeln!(out, "- Schema: `{}`", markdown_escape(&self.schema_version));
        let _ = writeln!(
            out,
            "- Generated At: `{}`",
            markdown_escape(&self.generated_at)
        );
        let _ = writeln!(out, "- Input Hash: `{}`", markdown_escape(&self.input_hash));
        let _ = writeln!(out, "- Stop Reason: `{:?}`", self.stop_reason);

        let _ = writeln!(out, "\n## 1. Facts (Confirmed)\n");
        let _ = writeln!(
            out,
            "| Entity | Attribute | Value | Confidence | Evidence |"
        );
        let _ = writeln!(out, "|---|---|---|---|---|");
        for f in &self.facts {
            let evs: Vec<String> = f.evidence_ids.iter().map(|s| markdown_escape(s)).collect();
            let _ = writeln!(
                out,
                "| {} | {} | {} | {:.3} | {} |",
                markdown_escape(&f.entity_id.0),
                markdown_escape(&f.attribute),
                markdown_escape(&f.claimed_value),
                f.confidence.value(),
                evs.join(", ")
            );
        }

        let _ = writeln!(out, "\n## 2. Inferences (Corroborated)\n");
        for inf in &self.inferences {
            let evs: Vec<String> = inf
                .evidence_ids
                .iter()
                .map(|s| markdown_escape(s))
                .collect();
            let _ = writeln!(
                out,
                "- **{}** ({:.3}): {} [Evidence: {}]",
                markdown_escape(&inf.entity_id.0),
                inf.confidence.value(),
                markdown_escape(&inf.statement),
                evs.join(", ")
            );
        }

        let _ = writeln!(out, "\n## 3. Conflicts (Preserved)\n");
        for c in &self.conflicts {
            let evs: Vec<String> = c.evidence_ids.iter().map(|s| markdown_escape(s)).collect();
            let _ = writeln!(
                out,
                "- **{}.{}** ({:.3}): {} [Evidence: {}]",
                markdown_escape(&c.entity_id.0),
                markdown_escape(&c.attribute),
                c.confidence.value(),
                markdown_escape(&c.explanation),
                evs.join(", ")
            );
        }

        let _ = writeln!(out, "\n## 4. Unknowns & Scope Boundaries\n");
        for u in &self.unknowns {
            let target = u
                .entity_id
                .as_ref()
                .map_or_else(|| "engagement".to_string(), |e| markdown_escape(&e.0));
            let _ = writeln!(
                out,
                "- **{}** — {}: {}",
                target,
                markdown_escape(&u.topic),
                markdown_escape(&u.reason)
            );
        }

        let _ = writeln!(out, "\n## 5. Defensive Exposure Remediation\n");
        for r in &self.remediations {
            let evs: Vec<String> = r.evidence_ids.iter().map(|s| markdown_escape(s)).collect();
            let _ = writeln!(
                out,
                "- **{}** [{}]: {} [Evidence: {}]",
                markdown_escape(&r.target),
                markdown_escape(&r.control),
                markdown_escape(&r.action),
                evs.join(", ")
            );
        }
        out
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
        let _ = writeln!(out, "  \"stop_reason\": \"{:?}\",", self.stop_reason);

        let write_claims = |out: &mut String, key: &str, claims: &[ReportClaim]| {
            let _ = writeln!(out, "  \"{key}\": [");
            for (i, c) in claims.iter().enumerate() {
                let evs: Vec<String> = c
                    .evidence_ids
                    .iter()
                    .map(|e| format!("\"{}\"", json_escape(e)))
                    .collect();
                let comma = if i + 1 < claims.len() { "," } else { "" };
                let _ = writeln!(
                    out,
                    "    {{\"claim_id\": \"{}\", \"entity_id\": \"{}\", \"attribute\": \"{}\", \"claimed_value\": \"{}\", \"statement\": \"{}\", \"confidence_milli\": {}, \"evidence_ids\": [{}]}}{}",
                    json_escape(&c.claim_id),
                    json_escape(&c.entity_id.0),
                    json_escape(&c.attribute),
                    json_escape(&c.claimed_value),
                    json_escape(&c.statement),
                    c.confidence.milli(),
                    evs.join(", "),
                    comma
                );
            }
            let _ = writeln!(out, "  ],");
        };

        write_claims(&mut out, "facts", &self.facts);
        write_claims(&mut out, "inferences", &self.inferences);

        let _ = writeln!(out, "  \"conflicts\": [");
        for (i, c) in self.conflicts.iter().enumerate() {
            let evs: Vec<String> = c
                .evidence_ids
                .iter()
                .map(|e| format!("\"{}\"", json_escape(e)))
                .collect();
            let cands: Vec<String> = c
                .candidates
                .iter()
                .map(|cand| {
                    format!(
                        "{{\"value\": \"{}\", \"observed_at\": \"{}\", \"source\": \"{}\", \"evidence_id\": \"{}\"}}",
                        json_escape(&cand.value),
                        json_escape(&cand.observed_at),
                        cand.source_kind.as_str(),
                        json_escape(&cand.evidence_id)
                    )
                })
                .collect();
            let comma = if i + 1 < self.conflicts.len() {
                ","
            } else {
                ""
            };
            let _ = writeln!(
                out,
                "    {{\"entity_id\": \"{}\", \"attribute\": \"{}\", \"confidence_milli\": {}, \"explanation\": \"{}\", \"candidates\": [{}], \"evidence_ids\": [{}]}}{}",
                json_escape(&c.entity_id.0),
                json_escape(&c.attribute),
                c.confidence.milli(),
                json_escape(&c.explanation),
                cands.join(", "),
                evs.join(", "),
                comma
            );
        }
        let _ = writeln!(out, "  ],");

        let _ = writeln!(out, "  \"unknowns\": [");
        for (i, u) in self.unknowns.iter().enumerate() {
            let eid_json = u.entity_id.as_ref().map_or_else(
                || "null".to_string(),
                |e| format!("\"{}\"", json_escape(&e.0)),
            );
            let comma = if i + 1 < self.unknowns.len() { "," } else { "" };
            let _ = writeln!(
                out,
                "    {{\"entity_id\": {}, \"topic\": \"{}\", \"reason\": \"{}\"}}{}",
                eid_json,
                json_escape(&u.topic),
                json_escape(&u.reason),
                comma
            );
        }
        let _ = writeln!(out, "  ],");

        let _ = writeln!(out, "  \"remediations\": [");
        for (i, r) in self.remediations.iter().enumerate() {
            let evs: Vec<String> = r
                .evidence_ids
                .iter()
                .map(|e| format!("\"{}\"", json_escape(e)))
                .collect();
            let comma = if i + 1 < self.remediations.len() {
                ","
            } else {
                ""
            };
            let _ = writeln!(
                out,
                "    {{\"target\": \"{}\", \"control\": \"{}\", \"action\": \"{}\", \"evidence_ids\": [{}]}}{}",
                json_escape(&r.target),
                json_escape(&r.control),
                json_escape(&r.action),
                evs.join(", "),
                comma
            );
        }
        let _ = writeln!(out, "  ]");
        let _ = writeln!(out, "}}");
        out
    }
}

// ============================================================================
// 6. MOCK OSINT AGENT (REACT HARNESS, CHECKPOINT/RESUME & EVALUATION)
// ============================================================================

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct EvaluationMetrics {
    pub sources_covered: usize,
    pub entity_count: usize,
    pub edge_count: usize,
    pub fact_count: usize,
    pub inference_count: usize,
    pub conflict_count: usize,
    pub unknown_count: usize,
    pub unsupported_claim_count: usize,
    pub duplicate_entity_or_evidence_count: usize,
    pub completeness_milli: u16,
    pub budget_exhausted: bool,
}

impl EvaluationMetrics {
    pub fn completeness(&self) -> f64 {
        f64::from(self.completeness_milli) / 1000.0
    }
}

pub struct MockOSINTAgent {
    pub scope: OsintScope,
    pub registry: ToolRegistry,
    pub seed_targets: Vec<(ToolTargetKind, String)>,
    pub max_steps: usize,
    pub max_tool_calls: usize,
    state: AgentState,
    stage_statuses: BTreeMap<Stage, StageStatus>,
    completed_calls: BTreeMap<String, ToolResult>,
    graph: EvidenceGraph,
    last_checkpoint: Option<Checkpoint>,
}

impl MockOSINTAgent {
    pub fn new(scope: OsintScope, registry: ToolRegistry) -> Self {
        let mut stage_statuses = BTreeMap::new();
        for stage in Stage::ALL {
            stage_statuses.insert(stage, StageStatus::Pending);
        }
        Self {
            scope,
            registry,
            seed_targets: vec![(ToolTargetKind::Domain, "corp.example".to_string())],
            max_steps: 64,
            max_tool_calls: 32,
            state: AgentState::new(),
            stage_statuses,
            completed_calls: BTreeMap::new(),
            graph: EvidenceGraph::new(),
            last_checkpoint: None,
        }
    }

    pub fn standard() -> Self {
        Self::new(OsintScope::standard(), ToolRegistry::standard())
    }

    pub fn with_seed_targets(mut self, seeds: Vec<(ToolTargetKind, String)>) -> Self {
        self.seed_targets = seeds;
        self
    }

    pub fn with_budget(mut self, max_steps: usize, max_tool_calls: usize) -> Self {
        self.max_steps = max_steps;
        self.max_tool_calls = max_tool_calls;
        self
    }

    pub fn state(&self) -> &AgentState {
        &self.state
    }

    pub fn graph(&self) -> &EvidenceGraph {
        &self.graph
    }

    pub fn last_checkpoint(&self) -> Option<&Checkpoint> {
        self.last_checkpoint.as_ref()
    }

    pub fn stage_statuses(&self) -> &BTreeMap<Stage, StageStatus> {
        &self.stage_statuses
    }

    pub fn input_hash(&self) -> String {
        let mut buf = format!(
            "eng={}|steps={}|calls={}|active={}\n",
            self.scope.engagement_id,
            self.max_steps,
            self.max_tool_calls,
            self.scope.allow_active_probing
        );
        for d in &self.scope.root_domains {
            buf.push_str(&format!("root:{d}\n"));
        }
        for h in &self.scope.allowed_org_handles {
            buf.push_str(&format!("handle:{h}\n"));
        }
        for s in &self.scope.allowed_sources {
            buf.push_str(&format!("src:{}\n", s.as_str()));
        }
        for (tk, target) in &self.seed_targets {
            buf.push_str(&format!("seed:{tk:?}:{target}\n"));
        }
        sha256_hex(buf.as_bytes())
    }

    fn snapshot_checkpoint(&self, input_hash: &str) -> Checkpoint {
        Checkpoint {
            version: CHECKPOINT_VERSION,
            engagement_id: self.scope.engagement_id.clone(),
            input_hash: input_hash.to_string(),
            stage_statuses: self.stage_statuses.clone(),
            state: self.state.clone(),
            completed_calls: self.completed_calls.clone(),
            graph: self.graph.clone(),
        }
    }

    fn planned_sources_for(&self, kind: ToolTargetKind) -> Vec<SourceKind> {
        SourceKind::ALL
            .into_iter()
            .filter(|src| {
                self.scope.allowed_sources.contains(src)
                    && self.registry.is_registered(*src)
                    && self.registry.expected_target_kind(*src) == Some(kind)
            })
            .collect()
    }

    fn canonicalize_target(&self, kind: ToolTargetKind, raw: &str) -> Result<String, OsintError> {
        match kind {
            ToolTargetKind::Domain => self.scope.validate_domain(raw),
            ToolTargetKind::OrgHandle => self.scope.validate_org_handle(raw),
        }
    }

    fn enqueue_discovered(&mut self, targets: Vec<(ToolTargetKind, String)>) {
        let mut sorted = targets;
        sorted.sort();
        sorted.dedup();
        for (tk, raw) in sorted {
            let Ok(canon) = self.canonicalize_target(tk, &raw) else {
                continue;
            };
            let item = (tk, canon);
            let already_pending = self.state.pending_targets.contains(&item);
            let sources = self.planned_sources_for(tk);
            let all_visited = !sources.is_empty()
                && sources
                    .iter()
                    .all(|src| self.state.visited_calls.contains(&(*src, item.1.clone())));
            if !already_pending && !all_visited {
                self.state.pending_targets.push_back(item);
            }
        }
    }

    /// Thực thi vòng lặp ReAct từ đầu. Trả `Err` nếu scope sai hoặc nguồn gặp lỗi mô phỏng
    /// (khi đó `last_checkpoint()` giữ nguyên trạng thái sạch ngay trước bước lỗi).
    pub fn run(&mut self) -> Result<OsintReport, OsintError> {
        self.state = AgentState::new();
        self.stage_statuses.clear();
        for stage in Stage::ALL {
            self.stage_statuses.insert(stage, StageStatus::Pending);
        }
        self.completed_calls.clear();
        self.graph = EvidenceGraph::new();
        self.last_checkpoint = None;
        self.drive_pipeline()
    }

    /// Khôi phục từ `Checkpoint` đã xác thực và chạy tiếp các bước chưa commit.
    pub fn resume(&mut self, checkpoint: &Checkpoint) -> Result<OsintReport, OsintError> {
        self.scope.validate()?;
        if checkpoint.version != CHECKPOINT_VERSION {
            return Err(OsintError::InvalidCheckpoint(format!(
                "version {} không khớp {}",
                checkpoint.version, CHECKPOINT_VERSION
            )));
        }
        if checkpoint.engagement_id != self.scope.engagement_id {
            return Err(OsintError::InvalidCheckpoint(format!(
                "engagement_id '{}' không khớp '{}'",
                checkpoint.engagement_id, self.scope.engagement_id
            )));
        }
        let expected_hash = self.input_hash();
        if checkpoint.input_hash != expected_hash {
            return Err(OsintError::InvalidCheckpoint(
                "input_hash không khớp cấu hình agent hiện tại".to_string(),
            ));
        }
        if checkpoint.state.tool_calls_used < checkpoint.completed_calls.len()
            || checkpoint.state.steps_used < checkpoint.completed_calls.len()
        {
            return Err(OsintError::InvalidCheckpoint(
                "bộ đếm bước/tool trong checkpoint nhỏ hơn số cuộc gọi đã commit".to_string(),
            ));
        }
        for (call_id, result) in &checkpoint.completed_calls {
            if &result.call_id != call_id {
                return Err(OsintError::InvalidCheckpoint(format!(
                    "call_id '{call_id}' không khớp kết quả"
                )));
            }
            let expected_digest = compute_result_digest(
                result.source,
                &result.canonical_target,
                &result.captured_at,
                &result.records,
            );
            if expected_digest != result.raw_digest {
                return Err(OsintError::InvalidCheckpoint(format!(
                    "raw_digest trong checkpoint bị sai lệch tại '{call_id}'"
                )));
            }
            if !self.scope.allowed_sources.contains(&result.source) {
                return Err(OsintError::ToolNotAllowed(result.source));
            }
            let expected_kind = self
                .registry
                .expected_target_kind(result.source)
                .ok_or(OsintError::ToolNotRegistered(result.source))?;
            let canon = self.canonicalize_target(expected_kind, &result.canonical_target)?;
            if canon != result.canonical_target
                || call_id != &format!("{}:{}", result.source.as_str(), result.canonical_target)
            {
                return Err(OsintError::InvalidCheckpoint(format!(
                    "call_id hoặc canonical_target không hợp lệ tại '{call_id}'"
                )));
            }
            for rec in &result.records {
                validate_raw_record(&self.scope, rec)?;
            }
        }
        for (tk, target) in &checkpoint.state.pending_targets {
            let canon = self.canonicalize_target(*tk, target)?;
            if &canon != target {
                return Err(OsintError::InvalidCheckpoint(format!(
                    "pending target '{target}' trong checkpoint chưa chuẩn hóa thành '{canon}'"
                )));
            }
        }
        if checkpoint.graph.ingested_calls.len() != checkpoint.completed_calls.len() {
            return Err(OsintError::InvalidCheckpoint(
                "số lượng completed_calls không khớp graph.ingested_calls trong checkpoint"
                    .to_string(),
            ));
        }
        for (cid, res) in &checkpoint.completed_calls {
            if checkpoint.graph.ingested_calls.get(cid) != Some(&res.raw_digest) {
                return Err(OsintError::InvalidCheckpoint(format!(
                    "completed_calls '{cid}' lệch với graph.ingested_calls"
                )));
            }
        }
        if checkpoint.stage_statuses.get(&Stage::PassiveExpansion) == Some(&StageStatus::Completed)
            && checkpoint.state.stop_reason != Some(StopReason::BudgetExhausted)
            && !checkpoint.state.pending_targets.is_empty()
        {
            return Err(OsintError::InvalidCheckpoint(
                "PassiveExpansion đã Completed nhưng pending_targets vẫn còn mục".to_string(),
            ));
        }
        if (checkpoint.stage_statuses.get(&Stage::SeedCollection) == Some(&StageStatus::Completed)
            || checkpoint.stage_statuses.get(&Stage::PassiveExpansion)
                == Some(&StageStatus::Completed))
            && checkpoint.state.stop_reason != Some(StopReason::BudgetExhausted)
            && checkpoint.completed_calls.is_empty()
            && self
                .seed_targets
                .iter()
                .any(|(tk, _)| !self.planned_sources_for(*tk).is_empty())
        {
            return Err(OsintError::InvalidCheckpoint(
                "giai đoạn thu thập đã Completed nhưng không có cuộc gọi nào được ghi nhận"
                    .to_string(),
            ));
        }
        if checkpoint.stage_statuses.get(&Stage::ScopeValidation) == Some(&StageStatus::Completed)
            && checkpoint.state.stop_reason != Some(StopReason::BudgetExhausted)
        {
            for (tk, raw_seed) in &self.seed_targets {
                let canon = self.canonicalize_target(*tk, raw_seed)?;
                let in_pending = checkpoint
                    .state
                    .pending_targets
                    .contains(&(*tk, canon.clone()));
                let any_visited = self.planned_sources_for(*tk).into_iter().any(|src| {
                    checkpoint
                        .state
                        .visited_calls
                        .contains(&(src, canon.clone()))
                });
                if !in_pending && !any_visited && !self.planned_sources_for(*tk).is_empty() {
                    return Err(OsintError::InvalidCheckpoint(format!(
                        "seed '{canon}' bị thiếu khỏi cả pending_targets lẫn visited_calls"
                    )));
                }
            }
        }
        checkpoint.graph.validate()?;

        self.state = checkpoint.state.clone();
        self.state.stop_reason = None;
        self.stage_statuses = checkpoint.stage_statuses.clone();
        for status in self.stage_statuses.values_mut() {
            if *status == StageStatus::Halted {
                *status = StageStatus::InProgress;
            }
        }
        self.completed_calls = checkpoint.completed_calls.clone();
        self.graph = checkpoint.graph.clone();
        self.last_checkpoint = Some(checkpoint.clone());
        self.drive_pipeline()
    }

    fn drive_pipeline(&mut self) -> Result<OsintReport, OsintError> {
        let input_hash = self.input_hash();

        // Stage 1: ScopeValidation
        if self.stage_statuses.get(&Stage::ScopeValidation) != Some(&StageStatus::Completed) {
            self.state.current_stage = Stage::ScopeValidation;
            self.stage_statuses
                .insert(Stage::ScopeValidation, StageStatus::InProgress);
            self.scope.validate()?;

            let mut normalized_seeds = Vec::with_capacity(self.seed_targets.len());
            for (tk, raw) in &self.seed_targets {
                let canon = self.canonicalize_target(*tk, raw)?;
                normalized_seeds.push((*tk, canon));
            }
            self.state.pending_targets.clear();
            for item in normalized_seeds {
                if !self.state.pending_targets.contains(&item) {
                    self.state.pending_targets.push_back(item);
                }
            }
            self.state.seeds_remaining = self.state.pending_targets.len();
            self.state.action_trace.push(AgentAction::ValidateScope);
            self.stage_statuses
                .insert(Stage::ScopeValidation, StageStatus::Completed);
            self.last_checkpoint = Some(self.snapshot_checkpoint(&input_hash));
        }

        // Stage 2 & 3: SeedCollection + PassiveExpansion
        if self.stage_statuses.get(&Stage::SeedCollection) != Some(&StageStatus::Completed)
            || self.stage_statuses.get(&Stage::PassiveExpansion) != Some(&StageStatus::Completed)
        {
            if self.state.seeds_remaining > 0
                && self.stage_statuses.get(&Stage::SeedCollection) != Some(&StageStatus::Completed)
            {
                self.state.current_stage = Stage::SeedCollection;
                self.stage_statuses
                    .insert(Stage::SeedCollection, StageStatus::InProgress);
            } else {
                self.state.current_stage = Stage::PassiveExpansion;
                self.stage_statuses
                    .insert(Stage::PassiveExpansion, StageStatus::InProgress);
            }

            while let Some((tk, target)) = self.state.pending_targets.front().cloned() {
                if self.state.seeds_remaining == 0
                    && self.stage_statuses.get(&Stage::SeedCollection)
                        != Some(&StageStatus::Completed)
                {
                    self.stage_statuses
                        .insert(Stage::SeedCollection, StageStatus::Completed);
                    self.state.current_stage = Stage::PassiveExpansion;
                    self.stage_statuses
                        .insert(Stage::PassiveExpansion, StageStatus::InProgress);
                    self.last_checkpoint = Some(self.snapshot_checkpoint(&input_hash));
                }

                let sources = self.planned_sources_for(tk);
                let mut halted_for_budget = false;

                for source in sources {
                    let call_id = format!("{}:{}", source.as_str(), target);
                    if self.completed_calls.contains_key(&call_id) {
                        continue;
                    }
                    if self.state.steps_used >= self.max_steps
                        || self.state.tool_calls_used >= self.max_tool_calls
                    {
                        self.state.stop_reason = Some(StopReason::BudgetExhausted);
                        self.state
                            .action_trace
                            .push(AgentAction::Halt(StopReason::BudgetExhausted));
                        self.stage_statuses
                            .insert(self.state.current_stage, StageStatus::Halted);
                        self.last_checkpoint = Some(self.snapshot_checkpoint(&input_hash));
                        halted_for_budget = true;
                        break;
                    }
                    if self.state.visited_calls.contains(&(source, target.clone())) {
                        self.state.steps_used += 1;
                        self.state.action_trace.push(AgentAction::SkipVisited {
                            source,
                            target: target.clone(),
                        });
                        continue;
                    }

                    // Lưu checkpoint sạch ngay trước khi thực thi tool để nếu lỗi,
                    // resume() khôi phục chính xác trạng thái trước bước bị lỗi.
                    let pre_call_cp = self.snapshot_checkpoint(&input_hash);
                    let req =
                        ToolRequest::passive(&self.scope.engagement_id, source, tk, target.clone());
                    match self.registry.execute(&self.scope, &req, &call_id) {
                        Ok(result) => {
                            self.state.steps_used += 1;
                            self.state.tool_calls_used += 1;
                            self.state.visited_calls.insert((source, target.clone()));
                            self.state.action_trace.push(AgentAction::CallTool(req));

                            let mut discovered = Vec::new();
                            for rec in &result.records {
                                discovered.extend(rec.discovered_targets.clone());
                            }

                            self.graph.ingest_result(&self.scope, &result)?;
                            self.completed_calls.insert(call_id, result);
                            self.enqueue_discovered(discovered);
                            self.last_checkpoint = Some(self.snapshot_checkpoint(&input_hash));
                        }
                        Err(err) => {
                            self.last_checkpoint = Some(pre_call_cp);
                            self.state.stop_reason = Some(StopReason::SourceFailure);
                            self.stage_statuses
                                .insert(self.state.current_stage, StageStatus::Halted);
                            return Err(err);
                        }
                    }
                }

                if halted_for_budget {
                    break;
                }

                self.state.pending_targets.pop_front();
                if self.state.seeds_remaining > 0 {
                    self.state.seeds_remaining -= 1;
                }
                if self.state.seeds_remaining == 0
                    && self.stage_statuses.get(&Stage::SeedCollection)
                        != Some(&StageStatus::Completed)
                {
                    self.stage_statuses
                        .insert(Stage::SeedCollection, StageStatus::Completed);
                    self.state.current_stage = Stage::PassiveExpansion;
                    self.stage_statuses
                        .insert(Stage::PassiveExpansion, StageStatus::InProgress);
                }
                self.last_checkpoint = Some(self.snapshot_checkpoint(&input_hash));
            }

            if self.state.stop_reason.is_none() {
                self.stage_statuses
                    .insert(Stage::SeedCollection, StageStatus::Completed);
                self.stage_statuses
                    .insert(Stage::PassiveExpansion, StageStatus::Completed);
            }
        }

        // Stage 4: EntityResolution
        if self.stage_statuses.get(&Stage::EntityResolution) != Some(&StageStatus::Completed) {
            self.state.current_stage = Stage::EntityResolution;
            self.stage_statuses
                .insert(Stage::EntityResolution, StageStatus::InProgress);
            self.graph.merge_same_as_entities()?;
            self.state.action_trace.push(AgentAction::ResolveEntities);
            self.stage_statuses
                .insert(Stage::EntityResolution, StageStatus::Completed);
        }

        // Stage 5: GraphVerification
        if self.stage_statuses.get(&Stage::GraphVerification) != Some(&StageStatus::Completed) {
            self.state.current_stage = Stage::GraphVerification;
            self.stage_statuses
                .insert(Stage::GraphVerification, StageStatus::InProgress);
            self.graph.validate()?;
            self.state.action_trace.push(AgentAction::VerifyGraph);
            self.stage_statuses
                .insert(Stage::GraphVerification, StageStatus::Completed);
        }

        // Stage 6: ReportGeneration
        self.state.current_stage = Stage::ReportGeneration;
        self.stage_statuses
            .insert(Stage::ReportGeneration, StageStatus::InProgress);
        let stop_reason = self.state.stop_reason.unwrap_or(StopReason::Completed);
        self.state.stop_reason = Some(stop_reason);
        self.state.action_trace.push(AgentAction::BuildReport);
        let report = self.build_report(&input_hash, stop_reason)?;
        report.validate(&self.graph)?;
        self.stage_statuses
            .insert(Stage::ReportGeneration, StageStatus::Completed);
        self.last_checkpoint = Some(self.snapshot_checkpoint(&input_hash));
        Ok(report)
    }

    fn build_report(
        &self,
        input_hash: &str,
        stop_reason: StopReason,
    ) -> Result<OsintReport, OsintError> {
        let mut facts = Vec::new();
        let mut conflicts = Vec::new();
        let mut unknowns = Vec::new();
        let mut remediations = Vec::new();

        for (eid, entity) in self.graph.entities() {
            for (attr, res) in &entity.attributes {
                match res.status {
                    ResolutionStatus::Confirmed => {
                        if let Some(val) = &res.resolved_value {
                            let ev_ids: BTreeSet<String> = res
                                .candidates
                                .iter()
                                .map(|c| c.evidence_id.clone())
                                .collect();
                            facts.push(ReportClaim {
                                claim_id: format!("fact:{}:{}", eid.0, attr),
                                entity_id: eid.clone(),
                                attribute: attr.clone(),
                                claimed_value: val.clone(),
                                statement: format!("{}.{} = {}", eid.0, attr, val),
                                confidence: res.confidence.clone(),
                                evidence_ids: ev_ids,
                            });
                        }
                    }
                    ResolutionStatus::Conflicted => {
                        let ev_ids: BTreeSet<String> = res
                            .candidates
                            .iter()
                            .map(|c| c.evidence_id.clone())
                            .collect();
                        let candidates: Vec<ConflictCandidate> = res
                            .candidates
                            .iter()
                            .map(|c| ConflictCandidate {
                                value: c.value.clone(),
                                observed_at: c.observed_at.clone(),
                                source_kind: c.source_kind,
                                evidence_id: c.evidence_id.clone(),
                            })
                            .collect();
                        conflicts.push(ConflictRecord {
                            entity_id: eid.clone(),
                            attribute: attr.clone(),
                            candidates,
                            confidence: res.confidence.clone(),
                            evidence_ids: ev_ids.clone(),
                            explanation: res.confidence.rationale.clone(),
                        });
                        unknowns.push(UnknownGap {
                            entity_id: Some(eid.clone()),
                            topic: format!("conflict:{attr}"),
                            reason: format!(
                                "thuộc tính '{}' có dữ liệu trái chiều giữa DNS và Wayback; cần xác minh ngoài mô hình bằng thay đổi cấu hình được ủy quyền",
                                attr
                            ),
                        });
                    }
                    ResolutionStatus::Unknown => {
                        unknowns.push(UnknownGap {
                            entity_id: Some(eid.clone()),
                            topic: format!("missing:{attr}"),
                            reason: "không đủ dữ liệu thụ động".to_string(),
                        });
                    }
                }
            }
        }

        // Suy luận tổng hợp chỉ được tạo khi có >= 2 bằng chứng thực tế trong đồ thị.
        let mut inferences = Vec::new();
        if let Some(org) = self
            .graph
            .entities()
            .get(&EntityId("org:corp-example".to_string()))
            && org.evidence_ids.len() >= 2
            && !org.aliases.is_empty()
            && let Some(domain_res) = org.attributes.get("official_domain")
            && domain_res.status == ResolutionStatus::Confirmed
            && domain_res.resolved_value.as_deref() == Some("corp.example")
        {
            let evs: BTreeSet<String> = domain_res
                .candidates
                .iter()
                .map(|c| c.evidence_id.clone())
                .collect();
            if evs.len() >= 2 {
                inferences.push(ReportClaim {
                    claim_id: "inf:org-domain-ownership".to_string(),
                    entity_id: org.id.clone(),
                    attribute: "official_domain".to_string(),
                    claimed_value: "corp.example".to_string(),
                    statement: "WHOIS, GitHub (hợp nhất SameAs) và SocialMetadata cùng liên kết tổ chức corp-example với corp.example".to_string(),
                    confidence: Confidence::from_milli(910, "liên kết đa nguồn sau hợp nhất SameAs")?,
                    evidence_ids: evs,
                });
            }
        }

        if let (Some(portal), Some(api)) = (
            self.graph
                .entities()
                .get(&EntityId("domain:portal.corp.example".to_string())),
            self.graph
                .entities()
                .get(&EntityId("domain:api.corp.example".to_string())),
        ) && let Some(cname_res) = portal.attributes.get("cname_target")
            && cname_res.status == ResolutionStatus::Confirmed
            && cname_res.resolved_value.as_deref() == Some("api.corp.example")
        {
            let mut evs = BTreeSet::new();
            for res in portal
                .attributes
                .values()
                .filter(|r| r.status == ResolutionStatus::Confirmed)
            {
                for cand in &res.candidates {
                    evs.insert(cand.evidence_id.clone());
                }
            }
            for res in api
                .attributes
                .values()
                .filter(|r| r.status == ResolutionStatus::Confirmed)
            {
                for cand in &res.candidates {
                    evs.insert(cand.evidence_id.clone());
                }
            }
            if evs.len() >= 2 {
                inferences.push(ReportClaim {
                    claim_id: "inf:portal-api-shared-backend".to_string(),
                    entity_id: portal.id.clone(),
                    attribute: "cname_target".to_string(),
                    claimed_value: "api.corp.example".to_string(),
                    statement: "portal.corp.example trỏ CNAME tới api.corp.example và cùng dùng IP RFC1918 10.20.30.15".to_string(),
                    confidence: Confidence::from_milli(890, "DNS CNAME/A và CT logs cùng xác nhận")?,
                    evidence_ids: evs,
                });
            }
        }

        if let Some(mail) = self
            .graph
            .entities()
            .get(&EntityId("domain:mail.corp.example".to_string()))
        {
            let has_mail_cert = self.graph.entities().values().any(|e| {
                e.kind == EntityKind::Certificate
                    && e.attributes
                        .get("san_domain")
                        .and_then(|r| r.resolved_value.as_deref())
                        == Some("mail.corp.example")
            });
            if !has_mail_cert {
                unknowns.push(UnknownGap {
                    entity_id: Some(mail.id.clone()),
                    topic: "missing_ct_certificate".to_string(),
                    reason:
                        "chưa ghi nhận chứng chỉ TLS cho mail.corp.example trong CT logs tổng hợp"
                            .to_string(),
                });
            }
        }

        if let Some(repo) = self
            .graph
            .entities()
            .get(&EntityId("repo:corp-example-labs/public-sdk".to_string()))
            && let Some(sec_res) = repo.attributes.get("config_secret")
        {
            let evs: BTreeSet<String> = sec_res
                .candidates
                .iter()
                .map(|c| c.evidence_id.clone())
                .collect();
            unknowns.push(UnknownGap {
                entity_id: Some(repo.id.clone()),
                topic: "redacted_secret_rotation_state".to_string(),
                reason: "mẫu cấu hình công khai đã bị gắn nhãn redact tổng hợp; nguồn thụ động không biết bí mật nội bộ đã xoay vòng hay chưa".to_string(),
            });
            remediations.push(DefensiveRemediation {
                target: repo.id.0.clone(),
                control: "Secret Hygiene & CI Pre-commit Scanning".to_string(),
                action: "rà soát lịch sử repo nội bộ, thu hồi/xoay vòng giá trị thật nếu từng lộ và bật secret scanning".to_string(),
                evidence_ids: evs,
            });
        }

        if let Some(cert) = self
            .graph
            .entities()
            .get(&EntityId("cert:staging-old.corp.example-2025".to_string()))
            && let Some(status_res) = cert.attributes.get("cert_status")
            && status_res.resolved_value.as_deref() == Some("expired-unrevoked")
        {
            let evs: BTreeSet<String> = status_res
                .candidates
                .iter()
                .map(|c| c.evidence_id.clone())
                .collect();
            remediations.push(DefensiveRemediation {
                target: "staging-old.corp.example".to_string(),
                control: "Certificate & Stale DNS Lifecycle".to_string(),
                action: "xác minh trạng thái gỡ bỏ dịch vụ, thu hồi/dọn chứng chỉ hết hạn và xóa bản ghi DNS cũ nếu không còn sử dụng".to_string(),
                evidence_ids: evs,
            });
        }

        for (from, to) in self.graph.pending_aliases.keys() {
            unknowns.push(UnknownGap {
                entity_id: Some(from.clone()),
                topic: "unresolved_alias_target".to_string(),
                reason: format!(
                    "bí danh '{}' tham chiếu thực thể '{}' chưa thu thập được trước khi dừng",
                    from.0, to.0
                ),
            });
        }

        if stop_reason == StopReason::BudgetExhausted && !self.state.pending_targets.is_empty() {
            unknowns.push(UnknownGap {
                entity_id: None,
                topic: "incomplete_collection_budget".to_string(),
                reason: format!(
                    "dừng sớm do cạn ngân sách; còn {} target trong hàng đợi chưa thu thập hết",
                    self.state.pending_targets.len()
                ),
            });
        }

        facts.sort();
        inferences.sort();
        unknowns.sort();
        remediations.sort();

        Ok(OsintReport {
            schema_version: REPORT_SCHEMA_VERSION.to_string(),
            engagement_id: self.scope.engagement_id.clone(),
            generated_at: FIXED_TIMESTAMP.to_string(),
            input_hash: input_hash.to_string(),
            stop_reason,
            facts,
            inferences,
            conflicts,
            unknowns,
            remediations,
        })
    }

    pub fn evaluate(&self, report: &OsintReport) -> Result<EvaluationMetrics, OsintError> {
        report.validate(&self.graph)?;

        let covered_sources: BTreeSet<SourceKind> =
            self.graph.sources().values().map(|s| s.kind).collect();
        let sources_covered = covered_sources.len();

        let expected_golden_entities = [
            "org:corp-example",
            "domain:corp.example",
            "domain:api.corp.example",
            "domain:portal.corp.example",
            "domain:staging-old.corp.example",
            "repo:corp-example-labs/public-sdk",
        ];
        let golden_hits = expected_golden_entities
            .iter()
            .filter(|id| {
                self.graph
                    .entities()
                    .contains_key(&EntityId((*id).to_string()))
            })
            .count();

        let has_conflict = !report.conflicts.is_empty();
        let total_checks = SourceKind::ALL.len() + expected_golden_entities.len() + 1;
        let passed_checks = sources_covered + golden_hits + usize::from(has_conflict);
        let completeness_milli = ((passed_checks * 1000) / total_checks) as u16;

        Ok(EvaluationMetrics {
            sources_covered,
            entity_count: self.graph.entities().len(),
            edge_count: self.graph.edges().len(),
            fact_count: report.facts.len(),
            inference_count: report.inferences.len(),
            conflict_count: report.conflicts.len(),
            unknown_count: report.unknowns.len(),
            unsupported_claim_count: 0,
            duplicate_entity_or_evidence_count: self.graph.duplicate_ingest_skips(),
            completeness_milli,
            budget_exhausted: report.stop_reason == StopReason::BudgetExhausted,
        })
    }
}

fn main() -> Result<(), Box<dyn Error>> {
    let mut agent = MockOSINTAgent::standard();
    let report = agent.run()?;
    let metrics = agent.evaluate(&report)?;

    // Kiểm chứng active probe bị chặn trước khi tạo tool result.
    let active_req = ToolRequest::passive(
        ENGAGEMENT_ID,
        SourceKind::Dns,
        ToolTargetKind::Domain,
        "corp.example",
    )
    .with_mode(CollectionMode::ActiveProbe);
    let active_status = match agent
        .registry
        .execute(&agent.scope, &active_req, "demo-active-probe")
    {
        Err(OsintError::ActiveProbeProhibited) => "blocked(ActiveProbeProhibited)",
        other => return Err(format!("kỳ vọng chặn active probe nhưng nhận {other:?}").into()),
    };

    // Kiểm chứng resume sau TimeoutOnce cho output JSON + Markdown giống hệt từng byte.
    let mut fault_agent = MockOSINTAgent::standard();
    fault_agent.registry.inject_fault(
        SourceKind::Dns,
        "api.corp.example",
        SimulatedFault::TimeoutOnce,
    );
    let _ = fault_agent
        .run()
        .expect_err("lần chạy có TimeoutOnce phải dừng và lưu checkpoint");
    let cp = fault_agent
        .last_checkpoint()
        .expect("checkpoint phải tồn tại")
        .clone();
    let mut recovered = MockOSINTAgent::standard();
    let resumed_report = recovered.resume(&cp)?;
    let byte_identical = report.render_json() == resumed_report.render_json()
        && report.render_markdown() == resumed_report.render_markdown();

    // Kiểm chứng budget exhaustion dừng an toàn thay vì lặp vô hạn.
    let mut bounded = MockOSINTAgent::standard().with_budget(4, 4);
    let bounded_report = bounded.run()?;

    println!("MockOSINTAgent — 100% synthetic, passive-only, std-only, không network/API/PII");
    println!("Active probe guard: {active_status}");
    println!(
        "Sources covered: {}/6; entities: {}; edges: {}",
        metrics.sources_covered, metrics.entity_count, metrics.edge_count
    );
    println!(
        "Merged alias (SameAs): org:corp-example-labs -> org:corp-example (aliases={})",
        agent
            .graph()
            .entities()
            .get(&EntityId("org:corp-example".to_string()))
            .map_or(0, |e| e.aliases.len())
    );
    println!(
        "Conflicts preserved: {}; facts: {}; inferences: {}; unknowns: {}",
        report.conflicts.len(),
        report.facts.len(),
        report.inferences.len(),
        report.unknowns.len()
    );
    println!(
        "Checkpoint/resume byte-identical: {byte_identical}; duplicate_count={}",
        metrics.duplicate_entity_or_evidence_count
    );
    println!("Budget guard: stop_reason={:?}", bounded_report.stop_reason);
    println!(
        "Evaluation: completeness={:.3}; unsupported_claims={}; duplicates={}",
        metrics.completeness(),
        metrics.unsupported_claim_count,
        metrics.duplicate_entity_or_evidence_count
    );
    Ok(())
}

// ============================================================================
// 7. BỘ KIỂM THỬ ĐƠN VỊ TẤT ĐỊNH
// ============================================================================

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn sha256_matches_nist_abc_vector() {
        assert_eq!(
            sha256_hex(b"abc"),
            "ba7816bf8f01cfea414140de5dae2223b00361a396177a9cb410ff61f20015ad"
        );
    }

    #[test]
    fn domain_normalization_handles_fqdn_and_rejects_bad_or_non_example_inputs() {
        assert_eq!(
            normalize_example_domain("API.CORP.EXAMPLE.").unwrap(),
            "api.corp.example"
        );
        assert!(matches!(
            normalize_example_domain("corp.com"),
            Err(OsintError::OutOfScope(_))
        ));
        assert!(matches!(
            normalize_example_domain("example"),
            Err(OsintError::InvalidDomain(_))
        ));
        for bad in [
            "",
            "bad domain.example",
            "corp..example",
            "-bad.example",
            "bad-.example",
            "corp.example..",
        ] {
            assert!(
                matches!(
                    normalize_example_domain(bad),
                    Err(OsintError::InvalidDomain(_))
                ),
                "expected InvalidDomain for '{bad}'"
            );
        }
    }

    #[test]
    fn rfc1918_ipv4_boundaries_are_enforced() {
        for ok in ["10.0.0.1", "172.16.0.1", "172.31.255.254", "192.168.1.10"] {
            assert!(is_rfc1918_ipv4(ok), "expected RFC1918 valid for {ok}");
        }
        for bad in [
            "172.15.255.255",
            "172.32.0.1",
            "127.0.0.1",
            "8.8.8.8",
            "0.0.0.0",
            "010.0.0.1",
            "192.168.1",
            "10.0.0.256",
            "10.0.0.-1",
        ] {
            assert!(!is_rfc1918_ipv4(bad), "expected RFC1918 reject for {bad}");
        }
    }

    #[test]
    fn scope_blocks_out_of_scope_domain_wrong_engagement_and_active_probe() {
        let scope = OsintScope::standard();
        let mut reg = ToolRegistry::standard();

        let outside = ToolRequest::passive(
            ENGAGEMENT_ID,
            SourceKind::Dns,
            ToolTargetKind::Domain,
            "other.example",
        );
        assert!(matches!(
            reg.execute(&scope, &outside, "c-outside"),
            Err(OsintError::OutOfScope(_))
        ));

        let wrong_eng = ToolRequest::passive(
            "ENG-WRONG",
            SourceKind::Dns,
            ToolTargetKind::Domain,
            "corp.example",
        );
        assert!(matches!(
            reg.execute(&scope, &wrong_eng, "c-eng"),
            Err(OsintError::OutOfScope(_))
        ));

        let active = ToolRequest::passive(
            ENGAGEMENT_ID,
            SourceKind::Dns,
            ToolTargetKind::Domain,
            "corp.example",
        )
        .with_mode(CollectionMode::ActiveProbe);
        assert_eq!(
            reg.execute(&scope, &active, "c-active"),
            Err(OsintError::ActiveProbeProhibited)
        );
    }

    #[test]
    fn registry_rejects_disallowed_source_unregistered_tool_and_schema_mismatch() {
        let scope = OsintScope::standard();
        let mut reg = ToolRegistry::standard();
        reg.disallow_source(SourceKind::Wayback);

        let disallowed = ToolRequest::passive(
            ENGAGEMENT_ID,
            SourceKind::Wayback,
            ToolTargetKind::Domain,
            "corp.example",
        );
        assert_eq!(
            reg.execute(&scope, &disallowed, "c-disallow"),
            Err(OsintError::ToolNotAllowed(SourceKind::Wayback))
        );

        let mismatch = ToolRequest::passive(
            ENGAGEMENT_ID,
            SourceKind::GitHub,
            ToolTargetKind::Domain,
            "corp.example",
        );
        assert_eq!(
            reg.execute(&scope, &mismatch, "c-mismatch"),
            Err(OsintError::SchemaMismatch {
                expected: ToolTargetKind::OrgHandle,
                got: ToolTargetKind::Domain,
            })
        );

        let empty_reg = ToolRegistry::new(scope.allowed_sources.clone());
        let mut empty_reg = empty_reg;
        let unreg = ToolRequest::passive(
            ENGAGEMENT_ID,
            SourceKind::Dns,
            ToolTargetKind::Domain,
            "corp.example",
        );
        assert_eq!(
            empty_reg.execute(&scope, &unreg, "c-unreg"),
            Err(OsintError::ToolNotRegistered(SourceKind::Dns))
        );
    }

    #[test]
    fn privacy_filter_allows_role_mailbox_and_rejects_personal_pii() {
        let scope = OsintScope::standard();
        assert_eq!(
            scope
                .validate_role_contact("SECURITY@CORP.EXAMPLE.")
                .unwrap(),
            "security@corp.example"
        );
        assert!(matches!(
            scope.validate_role_contact("alice.nguyen@corp.example"),
            Err(OsintError::PersonalDataProhibited(_))
        ));
        assert!(matches!(
            scope.validate_role_contact("security@other.example"),
            Err(OsintError::OutOfScope(_))
        ));

        struct LeakyPiiTool;
        impl OsintTool for LeakyPiiTool {
            fn source_kind(&self) -> SourceKind {
                SourceKind::SocialMetadata
            }
            fn expected_target_kind(&self) -> ToolTargetKind {
                ToolTargetKind::OrgHandle
            }
            fn query(&mut self, _req: &ToolRequest) -> Result<Vec<RawFindingRecord>, OsintError> {
                Ok(vec![RawFindingRecord::new(
                    "org:corp-example",
                    EntityKind::Organization,
                    "Corp Example",
                    "personal_email",
                    "alice.nguyen@corp.example",
                    FIXED_TIMESTAMP,
                    500,
                )])
            }
        }

        let mut reg = ToolRegistry::standard();
        reg.register(LeakyPiiTool);
        let req = ToolRequest::passive(
            ENGAGEMENT_ID,
            SourceKind::SocialMetadata,
            ToolTargetKind::OrgHandle,
            "corp-example-labs",
        );
        assert!(matches!(
            reg.execute(&scope, &req, "c-pii"),
            Err(OsintError::PersonalDataProhibited(_))
        ));
    }

    #[test]
    fn raw_secret_filter_rejects_unredacted_tokens() {
        let scope = OsintScope::standard();
        struct LeakySecretTool;
        impl OsintTool for LeakySecretTool {
            fn source_kind(&self) -> SourceKind {
                SourceKind::GitHub
            }
            fn expected_target_kind(&self) -> ToolTargetKind {
                ToolTargetKind::OrgHandle
            }
            fn query(&mut self, _req: &ToolRequest) -> Result<Vec<RawFindingRecord>, OsintError> {
                Ok(vec![RawFindingRecord::new(
                    "repo:corp-example-labs/public-sdk",
                    EntityKind::Repository,
                    "corp-example-labs/public-sdk",
                    "config_secret",
                    "ghp_1234567890synthetic",
                    FIXED_TIMESTAMP,
                    500,
                )])
            }
        }
        let mut reg = ToolRegistry::standard();
        reg.register(LeakySecretTool);
        let req = ToolRequest::passive(
            ENGAGEMENT_ID,
            SourceKind::GitHub,
            ToolTargetKind::OrgHandle,
            "corp-example-labs",
        );
        assert!(matches!(
            reg.execute(&scope, &req, "c-secret"),
            Err(OsintError::RawSecretProhibited(_))
        ));
    }

    #[test]
    fn confidence_enforces_unit_interval_and_non_empty_rationale() {
        let c0 = Confidence::new(0.0, "không có bằng chứng").unwrap();
        let c1 = Confidence::new(1.0, "xác nhận đầy đủ").unwrap();
        assert_eq!(c0.milli(), 0);
        assert_eq!(c1.milli(), 1000);
        assert!(matches!(
            Confidence::new(-0.01, "âm"),
            Err(OsintError::InvalidConfidence(_))
        ));
        assert!(matches!(
            Confidence::new(1.001, "quá 1"),
            Err(OsintError::InvalidConfidence(_))
        ));
        assert!(matches!(
            Confidence::new(f64::NAN, "nan"),
            Err(OsintError::InvalidConfidence(_))
        ));
        assert!(matches!(
            Confidence::from_milli(1001, "vượt 1000"),
            Err(OsintError::InvalidConfidence(_))
        ));
        assert!(matches!(
            Confidence::from_milli(500, "   "),
            Err(OsintError::InvalidConfidence(_))
        ));
    }

    #[test]
    fn custom_tool_rejects_public_ip_and_bad_timestamp_inside_records() {
        let scope = OsintScope::standard();
        struct BadIpTool;
        impl OsintTool for BadIpTool {
            fn source_kind(&self) -> SourceKind {
                SourceKind::Dns
            }
            fn expected_target_kind(&self) -> ToolTargetKind {
                ToolTargetKind::Domain
            }
            fn query(&mut self, req: &ToolRequest) -> Result<Vec<RawFindingRecord>, OsintError> {
                if req.target != "corp.example" {
                    return Ok(Vec::new());
                }
                Ok(vec![RawFindingRecord::new(
                    "domain:corp.example",
                    EntityKind::Domain,
                    "corp.example",
                    "a_record",
                    "8.8.8.8",
                    FIXED_TIMESTAMP,
                    700,
                )])
            }
        }
        let mut reg = ToolRegistry::standard();
        reg.register(BadIpTool);
        let req = ToolRequest::passive(
            ENGAGEMENT_ID,
            SourceKind::Dns,
            ToolTargetKind::Domain,
            "corp.example",
        );
        assert!(matches!(
            reg.execute(&scope, &req, "c-bad-ip"),
            Err(OsintError::InvalidRfc1918Ip(_))
        ));

        struct BadTsTool;
        impl OsintTool for BadTsTool {
            fn source_kind(&self) -> SourceKind {
                SourceKind::Dns
            }
            fn expected_target_kind(&self) -> ToolTargetKind {
                ToolTargetKind::Domain
            }
            fn query(&mut self, req: &ToolRequest) -> Result<Vec<RawFindingRecord>, OsintError> {
                if req.target != "corp.example" {
                    return Ok(Vec::new());
                }
                Ok(vec![RawFindingRecord::new(
                    "domain:corp.example",
                    EntityKind::Domain,
                    "corp.example",
                    "a_record",
                    "10.0.0.1",
                    "not-a-timestamp",
                    700,
                )])
            }
        }
        reg.register(BadTsTool);
        assert!(matches!(
            reg.execute(&scope, &req, "c-bad-ts"),
            Err(OsintError::MalformedResult(_))
        ));
    }

    #[test]
    fn all_six_mock_sources_run_and_golden_metrics_pass() {
        let mut agent = MockOSINTAgent::standard();
        let report = agent.run().unwrap();
        let metrics = agent.evaluate(&report).unwrap();

        assert_eq!(metrics.sources_covered, 6);
        assert_eq!(metrics.unsupported_claim_count, 0);
        assert_eq!(metrics.duplicate_entity_or_evidence_count, 0);
        assert_eq!(metrics.completeness_milli, 1000);
        assert!(!metrics.budget_exhausted);
        assert_eq!(report.stop_reason, StopReason::Completed);
        assert!(!report.facts.is_empty());
        assert!(!report.inferences.is_empty());
        assert!(!report.conflicts.is_empty());
        assert!(!report.unknowns.is_empty());
        assert!(!report.remediations.is_empty());
    }

    #[test]
    fn same_as_merge_deduplicates_evidence_and_is_idempotent() {
        let scope = OsintScope::standard();
        let mut reg = ToolRegistry::standard();
        let mut graph = EvidenceGraph::new();

        let whois = reg
            .execute(
                &scope,
                &ToolRequest::passive(
                    ENGAGEMENT_ID,
                    SourceKind::Whois,
                    ToolTargetKind::Domain,
                    "corp.example",
                ),
                "whois:corp.example",
            )
            .unwrap();
        let github = reg
            .execute(
                &scope,
                &ToolRequest::passive(
                    ENGAGEMENT_ID,
                    SourceKind::GitHub,
                    ToolTargetKind::OrgHandle,
                    "corp-example-labs",
                ),
                "github:corp-example-labs",
            )
            .unwrap();

        graph.ingest_result(&scope, &whois).unwrap();
        graph.ingest_result(&scope, &github).unwrap();
        // Nạp trùng kết quả GitHub: không được tạo bản sao.
        graph.ingest_result(&scope, &github).unwrap();
        assert_eq!(graph.duplicate_ingest_skips(), 1);

        let merged = graph.merge_same_as_entities().unwrap();
        assert_eq!(merged, 1);
        assert_eq!(graph.merge_same_as_entities().unwrap(), 0);
        graph.validate().unwrap();

        let org = graph
            .entities()
            .get(&EntityId("org:corp-example".to_string()))
            .expect("merged org exists");
        assert!(
            org.aliases
                .contains(&EntityId("org:corp-example-labs".to_string()))
        );
        assert!(
            !graph
                .entities()
                .contains_key(&EntityId("org:corp-example-labs".to_string()))
        );
        assert!(org.attributes.contains_key("role_contact"));
        assert!(graph.edges().iter().any(|e| e.kind == EdgeKind::SameAs));
    }

    #[test]
    fn same_as_merge_rejects_mismatched_entity_kinds() {
        let scope = OsintScope::standard();
        let mut graph = EvidenceGraph::new();

        let rec1 = RawFindingRecord::new(
            "domain:api.corp.example",
            EntityKind::Domain,
            "api.corp.example",
            "a_record",
            "10.20.30.15",
            FIXED_TIMESTAMP,
            800,
        );
        let d1 = compute_result_digest(
            SourceKind::Dns,
            "api.corp.example",
            FIXED_TIMESTAMP,
            std::slice::from_ref(&rec1),
        );
        graph
            .ingest_result(
                &scope,
                &ToolResult {
                    call_id: "c1".to_string(),
                    source: SourceKind::Dns,
                    canonical_target: "api.corp.example".to_string(),
                    captured_at: FIXED_TIMESTAMP.to_string(),
                    raw_digest: d1,
                    records: vec![rec1],
                },
            )
            .unwrap();

        let rec2 = RawFindingRecord::new(
            "org:corp-example",
            EntityKind::Organization,
            "Corp Example",
            "official_domain",
            "corp.example",
            FIXED_TIMESTAMP,
            800,
        )
        .with_alias_of("domain:api.corp.example");
        let d2 = compute_result_digest(
            SourceKind::Whois,
            "corp.example",
            FIXED_TIMESTAMP,
            std::slice::from_ref(&rec2),
        );
        let err = graph.ingest_result(
            &scope,
            &ToolResult {
                call_id: "c2".to_string(),
                source: SourceKind::Whois,
                canonical_target: "corp.example".to_string(),
                captured_at: FIXED_TIMESTAMP.to_string(),
                raw_digest: d2,
                records: vec![rec2],
            },
        );
        assert!(matches!(err, Err(OsintError::AliasMismatch(_))));
    }

    #[test]
    fn conflicting_dns_and_wayback_observations_preserve_both_values_and_contradicts_edge() {
        let mut agent = MockOSINTAgent::standard();
        let report = agent.run().unwrap();
        let staging = agent
            .graph()
            .entities()
            .get(&EntityId("domain:staging-old.corp.example".to_string()))
            .expect("staging-old exists");

        let a_res = staging.attributes.get("a_record").unwrap();
        assert_eq!(a_res.status, ResolutionStatus::Conflicted);
        assert_eq!(a_res.resolved_value, None);
        assert_eq!(a_res.candidates.len(), 2);
        let values: BTreeSet<&str> = a_res.candidates.iter().map(|c| c.value.as_str()).collect();
        assert!(values.contains("10.20.30.20"));
        assert!(values.contains("10.20.30.99"));
        assert!(a_res.confidence.milli() <= 450);

        assert!(agent.graph().edges().iter().any(|e| {
            e.kind == EdgeKind::Contradicts
                && e.from == EntityId("domain:staging-old.corp.example".to_string())
                && e.attribute == "a_record"
        }));
        assert!(report.conflicts.iter().any(|c| {
            c.entity_id == EntityId("domain:staging-old.corp.example".to_string())
                && c.attribute == "a_record"
        }));
        assert!(!report.facts.iter().any(|f| {
            f.entity_id == EntityId("domain:staging-old.corp.example".to_string())
                && (f.attribute == "a_record" || f.attribute == "deployment_state")
        }));
    }

    #[test]
    fn cycle_detection_normalizes_case_and_trailing_dot() {
        let mut agent = MockOSINTAgent::standard().with_seed_targets(vec![
            (ToolTargetKind::Domain, "api.corp.example".to_string()),
            (ToolTargetKind::Domain, "API.CORP.EXAMPLE.".to_string()),
        ]);
        let _report = agent.run().unwrap();
        assert_eq!(
            agent
                .registry
                .invocation_count(SourceKind::Dns, "api.corp.example"),
            1
        );
    }

    #[test]
    fn budget_exhaustion_stops_cleanly_on_steps_and_tool_calls() {
        let mut agent_calls = MockOSINTAgent::standard().with_budget(100, 3);
        let rep_calls = agent_calls.run().unwrap();
        assert_eq!(rep_calls.stop_reason, StopReason::BudgetExhausted);
        assert_eq!(agent_calls.state().tool_calls_used, 3);
        assert_eq!(
            agent_calls.state().stop_reason,
            Some(StopReason::BudgetExhausted)
        );

        let mut agent_steps = MockOSINTAgent::standard().with_budget(2, 100);
        let rep_steps = agent_steps.run().unwrap();
        assert_eq!(rep_steps.stop_reason, StopReason::BudgetExhausted);
        assert_eq!(agent_steps.state().steps_used, 2);

        let mut zero_budget = MockOSINTAgent::standard().with_budget(0, 0);
        let rep_zero = zero_budget.run().unwrap();
        assert_eq!(rep_zero.stop_reason, StopReason::BudgetExhausted);
        assert_eq!(zero_budget.state().tool_calls_used, 0);
    }

    #[test]
    fn checkpoint_and_resume_skip_committed_calls_and_match_straight_run_bytes() {
        let mut baseline = MockOSINTAgent::standard();
        let baseline_report = baseline.run().unwrap();

        let mut faulty = MockOSINTAgent::standard();
        faulty.registry.inject_fault(
            SourceKind::Dns,
            "api.corp.example",
            SimulatedFault::TimeoutOnce,
        );
        let err = faulty.run().unwrap_err();
        assert_eq!(
            err,
            OsintError::SimulatedFaultTriggered {
                source: SourceKind::Dns,
                fault: SimulatedFault::TimeoutOnce,
            }
        );
        let cp = faulty.last_checkpoint().expect("checkpoint saved").clone();
        assert!(cp.completed_calls.contains_key("whois:corp.example"));
        assert!(!cp.completed_calls.contains_key("dns:api.corp.example"));

        let whois_before = faulty
            .registry
            .invocation_count(SourceKind::Whois, "corp.example");
        assert_eq!(whois_before, 1);

        let resumed_report = faulty.resume(&cp).unwrap();
        assert_eq!(
            faulty
                .registry
                .invocation_count(SourceKind::Whois, "corp.example"),
            1,
            "committed tool must not be re-invoked on resume"
        );
        assert_eq!(
            faulty
                .registry
                .invocation_count(SourceKind::Dns, "api.corp.example"),
            2,
            "timed-out tool should run once more on resume"
        );
        assert_eq!(baseline_report.render_json(), resumed_report.render_json());
        assert_eq!(
            baseline_report.render_markdown(),
            resumed_report.render_markdown()
        );
        assert_eq!(faulty.graph().duplicate_ingest_skips(), 0);
    }

    #[test]
    fn resume_into_fresh_agent_never_calls_committed_tools() {
        let mut baseline = MockOSINTAgent::standard();
        let baseline_report = baseline.run().unwrap();

        let mut first = MockOSINTAgent::standard();
        first.registry.inject_fault(
            SourceKind::Wayback,
            "staging-old.corp.example",
            SimulatedFault::TimeoutOnce,
        );
        assert!(first.run().is_err());
        let cp = first.last_checkpoint().unwrap().clone();

        let mut second = MockOSINTAgent::standard();
        let resumed = second.resume(&cp).unwrap();
        assert_eq!(
            second
                .registry
                .invocation_count(SourceKind::Whois, "corp.example"),
            0
        );
        assert_eq!(
            second
                .registry
                .invocation_count(SourceKind::Wayback, "staging-old.corp.example"),
            1
        );
        assert_eq!(baseline_report.render_json(), resumed.render_json());
        assert_eq!(baseline_report.render_markdown(), resumed.render_markdown());
    }

    #[test]
    fn resume_rejects_tampered_version_hash_or_graph() {
        let mut agent = MockOSINTAgent::standard();
        agent.registry.inject_fault(
            SourceKind::Dns,
            "api.corp.example",
            SimulatedFault::TimeoutOnce,
        );
        assert!(agent.run().is_err());
        let valid_cp = agent.last_checkpoint().unwrap().clone();

        let mut bad_ver = valid_cp.clone();
        bad_ver.version = 999;
        assert!(matches!(
            agent.resume(&bad_ver),
            Err(OsintError::InvalidCheckpoint(_))
        ));

        let mut bad_hash = valid_cp.clone();
        bad_hash.input_hash = "0000".to_string();
        assert!(matches!(
            agent.resume(&bad_hash),
            Err(OsintError::InvalidCheckpoint(_))
        ));

        let mut bad_result = valid_cp.clone();
        if let Some(first_res) = bad_result.completed_calls.values_mut().next() {
            first_res.canonical_target = "tampered.example".to_string();
        }
        assert!(matches!(
            agent.resume(&bad_result),
            Err(OsintError::InvalidCheckpoint(_))
        ));
    }

    #[test]
    fn report_validation_blocks_unsupported_claims_and_conflicted_facts() {
        let mut agent = MockOSINTAgent::standard();
        let report = agent.run().unwrap();

        let mut empty_ev = report.clone();
        empty_ev.facts[0].evidence_ids.clear();
        assert!(matches!(
            empty_ev.validate(agent.graph()),
            Err(OsintError::UnsupportedClaim(_))
        ));

        let mut forged_ev = report.clone();
        forged_ev.facts[0]
            .evidence_ids
            .insert("ev:forged-999".to_string());
        assert!(matches!(
            forged_ev.validate(agent.graph()),
            Err(OsintError::UnsupportedClaim(_))
        ));

        let mut promoted_conflict = report.clone();
        promoted_conflict.facts.push(ReportClaim {
            claim_id: "fact:forged-conflict".to_string(),
            entity_id: EntityId("domain:staging-old.corp.example".to_string()),
            attribute: "a_record".to_string(),
            claimed_value: "10.20.30.20".to_string(),
            statement: "giả định chọn một phía mâu thuẫn".to_string(),
            confidence: Confidence::from_milli(900, "tự gắn").unwrap(),
            evidence_ids: BTreeSet::from(["ev:dns:staging-old.corp.example:0".to_string()]),
        });
        assert!(matches!(
            promoted_conflict.validate(agent.graph()),
            Err(OsintError::ConflictedFactClaim { .. })
        ));

        let mut single_ev_inf = report.clone();
        if let Some(first_inf) = single_ev_inf.inferences.first_mut() {
            let one = first_inf.evidence_ids.iter().next().unwrap().clone();
            first_inf.evidence_ids = BTreeSet::from([one]);
        }
        assert!(matches!(
            single_ev_inf.validate(agent.graph()),
            Err(OsintError::UnsupportedClaim(_))
        ));
    }

    #[test]
    fn render_json_and_markdown_escape_special_characters_safely() {
        let report = OsintReport {
            schema_version: REPORT_SCHEMA_VERSION.to_string(),
            engagement_id: "ENG-\"PIPE\"|<script>\n1".to_string(),
            generated_at: FIXED_TIMESTAMP.to_string(),
            input_hash: "abc".to_string(),
            stop_reason: StopReason::Completed,
            facts: vec![ReportClaim {
                claim_id: "c1".to_string(),
                entity_id: EntityId("domain:corp.example".to_string()),
                attribute: "note".to_string(),
                claimed_value: "a|b\t\"quoted\"\\`x`<img/>".to_string(),
                statement: "line1\nline2".to_string(),
                confidence: Confidence::from_milli(800, "ok").unwrap(),
                evidence_ids: BTreeSet::from(["ev:1".to_string()]),
            }],
            inferences: Vec::new(),
            conflicts: Vec::new(),
            unknowns: Vec::new(),
            remediations: Vec::new(),
        };

        let json = report.render_json();
        assert!(json.contains("\\\"PIPE\\\""));
        assert!(json.contains("\\n1"));
        assert!(json.contains("\\t\\\"quoted\\\"\\\\"));

        let md = report.render_markdown();
        assert!(!md.contains("<script>"));
        assert!(md.contains("&lt;script&gt;"));
        assert!(md.contains("a\\|b"));
        assert!(md.contains("\\`x\\`"));
    }

    #[test]
    fn replay_on_independent_agents_is_byte_identical() {
        let mut a1 = MockOSINTAgent::standard();
        let mut a2 = MockOSINTAgent::standard();
        let r1 = a1.run().unwrap();
        let r2 = a2.run().unwrap();
        assert_eq!(r1, r2);
        assert_eq!(r1.render_json(), r2.render_json());
        assert_eq!(r1.render_markdown(), r2.render_markdown());
        assert_eq!(a1.state().action_trace, a2.state().action_trace);
    }

    #[test]
    fn graph_validation_detects_missing_evidence_or_forged_edge() {
        let scope = OsintScope::standard();
        let mut reg = ToolRegistry::standard();
        let res = reg
            .execute(
                &scope,
                &ToolRequest::passive(
                    ENGAGEMENT_ID,
                    SourceKind::Dns,
                    ToolTargetKind::Domain,
                    "corp.example",
                ),
                "dns:corp.example",
            )
            .unwrap();
        let mut graph = EvidenceGraph::new();
        graph.ingest_result(&scope, &res).unwrap();
        graph.validate().unwrap();

        let mut bad_edge_graph = graph.clone();
        bad_edge_graph.edges.push(GraphEdge {
            kind: EdgeKind::Supports,
            from: EntityId("domain:corp.example".to_string()),
            to: EntityId("ip:10.20.30.10".to_string()),
            attribute: "a_record".to_string(),
            evidence_ids: BTreeSet::new(),
            note: "missing ev".to_string(),
        });
        assert!(matches!(
            bad_edge_graph.validate(),
            Err(OsintError::InvalidGraph(_))
        ));

        let mut missing_obs_graph = graph;
        missing_obs_graph.observations.clear();
        assert!(matches!(
            missing_obs_graph.validate(),
            Err(OsintError::InvalidGraph(_))
        ));
    }

    #[test]
    fn ingest_rejects_digest_tampering_and_conflicting_call_id() {
        let scope = OsintScope::standard();
        let mut reg = ToolRegistry::standard();
        let res = reg
            .execute(
                &scope,
                &ToolRequest::passive(
                    ENGAGEMENT_ID,
                    SourceKind::Whois,
                    ToolTargetKind::Domain,
                    "corp.example",
                ),
                "whois:corp.example",
            )
            .unwrap();
        let mut graph = EvidenceGraph::new();
        graph.ingest_result(&scope, &res).unwrap();

        let mut tampered = res.clone();
        tampered.raw_digest = "deadbeef".to_string();
        assert!(matches!(
            graph.ingest_result(&scope, &tampered),
            Err(OsintError::MalformedResult(_))
        ));

        let different_res = reg
            .execute(
                &scope,
                &ToolRequest::passive(
                    ENGAGEMENT_ID,
                    SourceKind::Dns,
                    ToolTargetKind::Domain,
                    "corp.example",
                ),
                "whois:corp.example",
            )
            .unwrap();
        assert!(matches!(
            graph.ingest_result(&scope, &different_res),
            Err(OsintError::DuplicateSource(_))
        ));
    }

    #[test]
    fn sha256_matches_empty_input_vector() {
        assert_eq!(
            sha256_hex(b""),
            "e3b0c44298fc1c149afbf4c8996fb92427ae41e4649b934ca495991b7852b855"
        );
    }

    #[test]
    fn org_handle_normalization_enforces_charset_and_hyphens() {
        assert_eq!(
            normalize_org_handle("Corp-Example-Labs").unwrap(),
            "corp-example-labs"
        );
        for bad in ["", "-corp", "corp-", "corp labs", "corp.labs", "corp_labs"] {
            assert!(
                matches!(normalize_org_handle(bad), Err(OsintError::InvalidTarget(_))),
                "expected InvalidTarget for '{bad}'"
            );
        }
    }

    #[test]
    fn iso8601_utc_validator_enforces_strict_format() {
        assert!(is_valid_iso8601_utc(FIXED_TIMESTAMP));
        assert!(is_valid_iso8601_utc(ARCHIVE_TIMESTAMP));
        for bad in [
            "",
            "2026-01-01",
            "2026-13-01T00:00:00Z",
            "2026-01-32T00:00:00Z",
            "2026-01-01T24:00:00Z",
            "2026-01-01T00:00:00+07:00",
        ] {
            assert!(!is_valid_iso8601_utc(bad), "expected invalid ts '{bad}'");
        }
    }

    #[test]
    fn scope_validate_rejects_empty_engagement_and_active_flag() {
        let mut empty_eng = OsintScope::standard();
        empty_eng.engagement_id = "   ".to_string();
        assert!(matches!(
            empty_eng.validate(),
            Err(OsintError::OutOfScope(_))
        ));

        let mut active_scope = OsintScope::standard();
        active_scope.allow_active_probing = true;
        assert_eq!(
            active_scope.validate(),
            Err(OsintError::ActiveProbeProhibited)
        );
    }

    #[test]
    fn scope_blocks_out_of_scope_org_handle() {
        let scope = OsintScope::standard();
        let mut reg = ToolRegistry::standard();
        let req = ToolRequest::passive(
            ENGAGEMENT_ID,
            SourceKind::GitHub,
            ToolTargetKind::OrgHandle,
            "unknown-org",
        );
        assert!(matches!(
            reg.execute(&scope, &req, "c-handle"),
            Err(OsintError::OutOfScope(_))
        ));
    }

    #[test]
    fn seed_validation_rejects_out_of_scope_seed_before_tool_calls() {
        let mut agent = MockOSINTAgent::standard()
            .with_seed_targets(vec![(ToolTargetKind::Domain, "attacker.com".to_string())]);
        let err = agent.run().unwrap_err();
        assert!(matches!(err, OsintError::OutOfScope(_)));
        assert_eq!(agent.registry.total_invocations(), 0);
    }

    #[test]
    fn raw_secret_filter_allows_synthetic_redacted_marker() {
        assert!(validate_no_raw_secret(SYNTHETIC_REDACTED_MARKER, Some("config_secret")).is_ok());
    }

    #[test]
    fn raw_secret_filter_rejects_sensitive_attribute_without_marker() {
        assert!(matches!(
            validate_no_raw_secret("some-plaintext-value", Some("api_key")),
            Err(OsintError::RawSecretProhibited(_))
        ));
        assert!(matches!(
            validate_no_raw_secret("Bearer eyJhbGciOiJIUzI1NiJ9", None),
            Err(OsintError::RawSecretProhibited(_))
        ));
    }

    #[test]
    fn multi_source_corroboration_raises_confidence_within_unit_bound() {
        let mut agent = MockOSINTAgent::standard();
        let _report = agent.run().unwrap();
        let org = agent
            .graph()
            .entities()
            .get(&EntityId("org:corp-example".to_string()))
            .unwrap();
        let official = org.attributes.get("official_domain").unwrap();
        assert_eq!(official.status, ResolutionStatus::Confirmed);
        assert!(official.candidates.len() >= 2);
        assert!(official.confidence.milli() > 900);
        assert!(official.confidence.milli() <= 980);
    }

    #[test]
    fn same_as_merge_preserves_conflicts_across_merged_aliases() {
        let scope = OsintScope::standard();
        let mut graph = EvidenceGraph::new();

        let rec1 = RawFindingRecord::new(
            "org:corp-example",
            EntityKind::Organization,
            "Corp Example",
            "hq_city",
            "Hanoi",
            FIXED_TIMESTAMP,
            800,
        );
        let res1 = ToolResult::new(
            "c1",
            SourceKind::Whois,
            "corp.example",
            FIXED_TIMESTAMP,
            vec![rec1],
        );
        graph.ingest_result(&scope, &res1).unwrap();

        let rec2 = RawFindingRecord::new(
            "org:corp-example-labs",
            EntityKind::Organization,
            "corp-example-labs",
            "hq_city",
            "Da Nang",
            ARCHIVE_TIMESTAMP,
            700,
        )
        .with_alias_of("org:corp-example");
        let res2 = ToolResult::new(
            "c2",
            SourceKind::GitHub,
            "corp-example-labs",
            FIXED_TIMESTAMP,
            vec![rec2],
        );
        graph.ingest_result(&scope, &res2).unwrap();

        assert_eq!(graph.merge_same_as_entities().unwrap(), 1);
        graph.validate().unwrap();
        let org = graph
            .entities()
            .get(&EntityId("org:corp-example".to_string()))
            .unwrap();
        let city = org.attributes.get("hq_city").unwrap();
        assert_eq!(city.status, ResolutionStatus::Conflicted);
        assert_eq!(city.candidates.len(), 2);
        assert!(graph.edges().iter().any(|e| {
            e.kind == EdgeKind::Contradicts
                && e.from == EntityId("org:corp-example".to_string())
                && e.attribute == "hq_city"
        }));
    }

    #[test]
    fn discovered_targets_outside_scope_are_not_scheduled() {
        struct ExternalDiscoveryTool;
        impl OsintTool for ExternalDiscoveryTool {
            fn source_kind(&self) -> SourceKind {
                SourceKind::Whois
            }
            fn expected_target_kind(&self) -> ToolTargetKind {
                ToolTargetKind::Domain
            }
            fn query(&mut self, _req: &ToolRequest) -> Result<Vec<RawFindingRecord>, OsintError> {
                Ok(vec![
                    RawFindingRecord::new(
                        "domain:corp.example",
                        EntityKind::Domain,
                        "corp.example",
                        "registered_org",
                        "Corp Example",
                        FIXED_TIMESTAMP,
                        850,
                    )
                    .with_discovered_target(ToolTargetKind::Domain, "evil-outside.com")
                    .with_discovered_target(ToolTargetKind::Domain, "partner.example"),
                ])
            }
        }

        let mut agent = MockOSINTAgent::standard();
        agent.registry.register(ExternalDiscoveryTool);
        let report = agent.run().unwrap();
        assert_eq!(report.stop_reason, StopReason::Completed);
        assert_eq!(
            agent
                .registry
                .invocation_count(SourceKind::Dns, "evil-outside.com"),
            0
        );
        assert_eq!(
            agent
                .registry
                .invocation_count(SourceKind::Dns, "partner.example"),
            0
        );
    }

    #[test]
    fn cycle_detection_records_skip_visited_when_queued_twice() {
        let mut agent = MockOSINTAgent::standard();
        agent
            .state
            .pending_targets
            .push_back((ToolTargetKind::Domain, "corp.example".to_string()));
        agent
            .state
            .visited_calls
            .insert((SourceKind::Dns, "corp.example".to_string()));
        agent
            .stage_statuses
            .insert(Stage::ScopeValidation, StageStatus::Completed);
        agent.drive_pipeline().unwrap();
        assert!(agent.state().action_trace.iter().any(|act| matches!(
            act,
            AgentAction::SkipVisited {
                source: SourceKind::Dns,
                target,
            } if target == "corp.example"
        )));

        // Kiểm chứng enqueue_discovered bỏ qua target đã thăm toàn bộ.
        let mut reg_agent = MockOSINTAgent::standard();
        reg_agent.run().unwrap();
        let before_invocations = reg_agent
            .registry
            .invocation_count(SourceKind::Dns, "api.corp.example");
        reg_agent.enqueue_discovered(vec![
            (ToolTargetKind::Domain, "API.CORP.EXAMPLE.".to_string()),
            (ToolTargetKind::Domain, "api.corp.example".to_string()),
        ]);
        assert!(reg_agent.state().pending_targets.is_empty());
        assert_eq!(before_invocations, 1);
    }

    #[test]
    fn fault_source_unavailable_persists_until_cleared() {
        let mut agent = MockOSINTAgent::standard();
        agent.registry.inject_fault(
            SourceKind::CtLogs,
            "corp.example",
            SimulatedFault::SourceUnavailable,
        );
        assert_eq!(
            agent.run().unwrap_err(),
            OsintError::SimulatedFaultTriggered {
                source: SourceKind::CtLogs,
                fault: SimulatedFault::SourceUnavailable,
            }
        );
        let cp = agent.last_checkpoint().unwrap().clone();
        assert_eq!(
            agent.resume(&cp).unwrap_err(),
            OsintError::SimulatedFaultTriggered {
                source: SourceKind::CtLogs,
                fault: SimulatedFault::SourceUnavailable,
            }
        );
        agent
            .registry
            .clear_fault(SourceKind::CtLogs, "corp.example");
        let ok_report = agent.resume(&cp).unwrap();
        assert_eq!(ok_report.stop_reason, StopReason::Completed);
    }

    #[test]
    fn fault_malformed_payload_returns_malformed_result() {
        let scope = OsintScope::standard();
        let mut reg = ToolRegistry::standard();
        reg.inject_fault(
            SourceKind::Dns,
            "corp.example",
            SimulatedFault::MalformedPayload,
        );
        let req = ToolRequest::passive(
            ENGAGEMENT_ID,
            SourceKind::Dns,
            ToolTargetKind::Domain,
            "corp.example",
        );
        assert!(matches!(
            reg.execute(&scope, &req, "c-malformed"),
            Err(OsintError::MalformedResult(_))
        ));
    }

    #[test]
    fn report_validation_rejects_cross_attribute_evidence_id() {
        let mut agent = MockOSINTAgent::standard();
        let mut report = agent.run().unwrap();
        let other_ev = report
            .facts
            .last()
            .unwrap()
            .evidence_ids
            .iter()
            .next()
            .unwrap()
            .clone();
        report.facts[0].evidence_ids = BTreeSet::from([other_ev]);
        assert!(matches!(
            report.validate(agent.graph()),
            Err(OsintError::UnsupportedClaim(_))
        ));
    }

    #[test]
    fn report_validation_rejects_missing_or_fake_remediation_evidence() {
        let mut agent = MockOSINTAgent::standard();
        let mut report = agent.run().unwrap();
        report.remediations[0].evidence_ids.clear();
        assert!(matches!(
            report.validate(agent.graph()),
            Err(OsintError::UnsupportedClaim(_))
        ));

        let mut report2 = agent.run().unwrap();
        report2.remediations[0]
            .evidence_ids
            .insert("ev:nonexistent".to_string());
        assert!(matches!(
            report2.validate(agent.graph()),
            Err(OsintError::UnsupportedClaim(_))
        ));
    }

    #[test]
    fn report_validation_rejects_duplicate_fact_claim_ids() {
        let mut agent = MockOSINTAgent::standard();
        let mut report = agent.run().unwrap();
        let dup = report.facts[0].clone();
        report.facts.push(dup);
        assert!(matches!(
            report.validate(agent.graph()),
            Err(OsintError::ReportSchemaInvalid(_))
        ));
    }

    #[test]
    fn duplicate_observation_still_preserves_same_as_alias_edge() {
        let scope = OsintScope::standard();
        let mut graph = EvidenceGraph::new();

        let rec1 = RawFindingRecord::new(
            "org:corp-example",
            EntityKind::Organization,
            "Corp Example",
            "official_domain",
            "corp.example",
            FIXED_TIMESTAMP,
            850,
        );
        let res1 = ToolResult::new(
            "c-primary",
            SourceKind::Whois,
            "corp.example",
            FIXED_TIMESTAMP,
            vec![rec1],
        );
        graph.ingest_result(&scope, &res1).unwrap();

        let rec2 = RawFindingRecord::new(
            "org:corp-example-labs",
            EntityKind::Organization,
            "corp-example-labs",
            "official_domain",
            "corp.example",
            FIXED_TIMESTAMP,
            850,
        );
        let res2 = ToolResult::new(
            "c-alias-1",
            SourceKind::GitHub,
            "corp-example-labs",
            FIXED_TIMESTAMP,
            vec![rec2],
        );
        graph.ingest_result(&scope, &res2).unwrap();

        // Một lần gọi khác từ cùng nguồn phát hiện thêm alias_of trên cùng quan sát đã có.
        let rec3 = RawFindingRecord::new(
            "org:corp-example-labs",
            EntityKind::Organization,
            "corp-example-labs",
            "official_domain",
            "corp.example",
            FIXED_TIMESTAMP,
            850,
        )
        .with_alias_of("org:corp-example");
        let res3 = ToolResult::new(
            "c-alias-2",
            SourceKind::GitHub,
            "corp-example-labs",
            FIXED_TIMESTAMP,
            vec![rec3],
        );
        graph.ingest_result(&scope, &res3).unwrap();

        assert!(graph.edges().iter().any(|e| {
            e.kind == EdgeKind::SameAs
                && e.from == EntityId("org:corp-example-labs".to_string())
                && e.to == EntityId("org:corp-example".to_string())
        }));
        assert_eq!(graph.merge_same_as_entities().unwrap(), 1);
        graph.validate().unwrap();
    }

    #[test]
    fn checkpoint_resume_rejects_out_of_scope_pending_target_and_call_graph_mismatch() {
        let mut agent = MockOSINTAgent::standard();
        agent.registry.inject_fault(
            SourceKind::Dns,
            "api.corp.example",
            SimulatedFault::TimeoutOnce,
        );
        assert!(agent.run().is_err());
        let valid_cp = agent.last_checkpoint().unwrap().clone();

        let mut bad_pending = valid_cp.clone();
        bad_pending
            .state
            .pending_targets
            .push_back((ToolTargetKind::Domain, "attacker.com".to_string()));
        assert!(matches!(
            agent.resume(&bad_pending),
            Err(OsintError::OutOfScope(_))
        ));

        let mut mismatched_calls = valid_cp.clone();
        mismatched_calls.completed_calls.clear();
        assert!(matches!(
            agent.resume(&mismatched_calls),
            Err(OsintError::InvalidCheckpoint(_))
        ));

        let mut bad_stage_state = valid_cp;
        bad_stage_state
            .stage_statuses
            .insert(Stage::PassiveExpansion, StageStatus::Completed);
        bad_stage_state.state.stop_reason = Some(StopReason::Completed);
        assert!(matches!(
            agent.resume(&bad_stage_state),
            Err(OsintError::InvalidCheckpoint(_))
        ));
    }

    #[test]
    fn report_validation_rejects_conflicted_observation_in_inference_and_forged_values() {
        let scope = OsintScope::standard();
        let mut graph = EvidenceGraph::new();

        let rec1 = RawFindingRecord::new(
            "domain:portal.corp.example",
            EntityKind::Domain,
            "portal.corp.example",
            "official_domain",
            "corp.example",
            FIXED_TIMESTAMP,
            850,
        );
        graph
            .ingest_result(
                &scope,
                &ToolResult::new(
                    "c-1",
                    SourceKind::Whois,
                    "portal.corp.example",
                    FIXED_TIMESTAMP,
                    vec![rec1],
                ),
            )
            .unwrap();

        let rec2 = RawFindingRecord::new(
            "domain:portal.corp.example",
            EntityKind::Domain,
            "portal.corp.example",
            "official_domain",
            "corp.example",
            FIXED_TIMESTAMP,
            840,
        );
        graph
            .ingest_result(
                &scope,
                &ToolResult::new(
                    "c-2",
                    SourceKind::CtLogs,
                    "portal.corp.example",
                    FIXED_TIMESTAMP,
                    vec![rec2],
                ),
            )
            .unwrap();

        let rec3 = RawFindingRecord::new(
            "domain:portal.corp.example",
            EntityKind::Domain,
            "portal.corp.example",
            "a_record",
            "10.20.30.11",
            FIXED_TIMESTAMP,
            800,
        );
        graph
            .ingest_result(
                &scope,
                &ToolResult::new(
                    "c-3",
                    SourceKind::Dns,
                    "portal.corp.example",
                    FIXED_TIMESTAMP,
                    vec![rec3],
                ),
            )
            .unwrap();

        let rec4 = RawFindingRecord::new(
            "domain:portal.corp.example",
            EntityKind::Domain,
            "portal.corp.example",
            "a_record",
            "10.20.30.22",
            ARCHIVE_TIMESTAMP,
            700,
        );
        graph
            .ingest_result(
                &scope,
                &ToolResult::new(
                    "c-4",
                    SourceKind::Wayback,
                    "portal.corp.example",
                    ARCHIVE_TIMESTAMP,
                    vec![rec4],
                ),
            )
            .unwrap();

        let rec5 = RawFindingRecord::new(
            "domain:isolated.corp.example",
            EntityKind::Domain,
            "isolated.corp.example",
            "a_record",
            "10.20.30.55",
            FIXED_TIMESTAMP,
            820,
        );
        graph
            .ingest_result(
                &scope,
                &ToolResult::new(
                    "c-5",
                    SourceKind::Dns,
                    "isolated.corp.example",
                    FIXED_TIMESTAMP,
                    vec![rec5],
                ),
            )
            .unwrap();

        let a_res = graph
            .entities()
            .get(&EntityId("domain:portal.corp.example".to_string()))
            .unwrap()
            .attributes
            .get("a_record")
            .unwrap();
        let base_report = OsintReport {
            schema_version: REPORT_SCHEMA_VERSION.to_string(),
            engagement_id: ENGAGEMENT_ID.to_string(),
            generated_at: FIXED_TIMESTAMP.to_string(),
            input_hash: "abc".to_string(),
            stop_reason: StopReason::Completed,
            facts: Vec::new(),
            inferences: vec![ReportClaim {
                claim_id: "inf:portal".to_string(),
                entity_id: EntityId("domain:portal.corp.example".to_string()),
                attribute: "official_domain".to_string(),
                claimed_value: "corp.example".to_string(),
                statement: "corroborated domain link".to_string(),
                confidence: Confidence::from_milli(860, "2 nguồn").unwrap(),
                evidence_ids: BTreeSet::from(["ev:c-1:0".to_string(), "ev:c-2:0".to_string()]),
            }],
            conflicts: vec![ConflictRecord {
                entity_id: EntityId("domain:portal.corp.example".to_string()),
                attribute: "a_record".to_string(),
                candidates: a_res
                    .candidates
                    .iter()
                    .map(|c| ConflictCandidate {
                        value: c.value.clone(),
                        observed_at: c.observed_at.clone(),
                        source_kind: c.source_kind,
                        evidence_id: c.evidence_id.clone(),
                    })
                    .collect(),
                confidence: a_res.confidence.clone(),
                evidence_ids: BTreeSet::from(["ev:c-3:0".to_string(), "ev:c-4:0".to_string()]),
                explanation: a_res.confidence.rationale.clone(),
            }],
            unknowns: Vec::new(),
            remediations: Vec::new(),
        };
        base_report.validate(&graph).unwrap();

        let mut backed_by_conflict = base_report.clone();
        backed_by_conflict.inferences[0]
            .evidence_ids
            .insert("ev:c-3:0".to_string());
        assert!(matches!(
            backed_by_conflict.validate(&graph),
            Err(OsintError::ConflictedFactClaim { .. })
        ));

        let mut unrelated_ev = base_report.clone();
        unrelated_ev.inferences[0]
            .evidence_ids
            .insert("ev:c-5:0".to_string());
        assert!(matches!(
            unrelated_ev.validate(&graph),
            Err(OsintError::UnsupportedClaim(_))
        ));

        let mut wrong_value = base_report.clone();
        wrong_value.inferences[0].claimed_value = "other.corp.example".to_string();
        assert!(matches!(
            wrong_value.validate(&graph),
            Err(OsintError::UnsupportedClaim(_))
        ));

        let mut public_ip_inf = base_report;
        public_ip_inf.inferences[0].attribute = "a_record".to_string();
        public_ip_inf.inferences[0].claimed_value = "8.8.8.8".to_string();
        assert!(matches!(
            public_ip_inf.validate(&graph),
            Err(OsintError::InvalidRfc1918Ip(_))
        ));
    }

    #[test]
    fn merge_same_as_deduplicates_same_source_observation_across_merged_aliases() {
        let scope = OsintScope::standard();
        let mut graph = EvidenceGraph::new();

        let rec1 = RawFindingRecord::new(
            "org:corp-example",
            EntityKind::Organization,
            "Corp Example",
            "official_domain",
            "corp.example",
            FIXED_TIMESTAMP,
            850,
        );
        graph
            .ingest_result(
                &scope,
                &ToolResult::new(
                    "c-primary",
                    SourceKind::Whois,
                    "corp.example",
                    FIXED_TIMESTAMP,
                    vec![rec1],
                ),
            )
            .unwrap();

        let rec2 = RawFindingRecord::new(
            "org:corp-example-labs",
            EntityKind::Organization,
            "corp-example-labs",
            "official_domain",
            "corp.example",
            FIXED_TIMESTAMP,
            850,
        )
        .with_alias_of("org:corp-example");
        graph
            .ingest_result(
                &scope,
                &ToolResult::new(
                    "c-alias",
                    SourceKind::Whois,
                    "corp-example-labs",
                    FIXED_TIMESTAMP,
                    vec![rec2],
                ),
            )
            .unwrap();

        assert_eq!(graph.merge_same_as_entities().unwrap(), 1);
        graph.validate().unwrap();
        assert_eq!(graph.entities().len(), 1);
        assert_eq!(graph.observations().len(), 1);
        assert_eq!(graph.evidence().len(), 1);
        assert!(graph.edges().iter().any(|e| {
            e.kind == EdgeKind::SameAs
                && e.evidence_ids
                    .iter()
                    .all(|ev| graph.evidence().contains_key(ev))
        }));
    }

    #[test]
    fn unresolved_alias_target_is_reported_as_unknown_gap_and_promotes_when_primary_arrives() {
        let mut agent = MockOSINTAgent::standard()
            .with_seed_targets(vec![
                (ToolTargetKind::OrgHandle, "corp-example-labs".to_string()),
                (ToolTargetKind::Domain, "corp.example".to_string()),
            ])
            .with_budget(1, 1);
        let report = agent.run().unwrap();
        assert_eq!(report.stop_reason, StopReason::BudgetExhausted);
        assert!(
            report
                .unknowns
                .iter()
                .any(|u| u.topic == "unresolved_alias_target")
        );
        report.validate(agent.graph()).unwrap();

        let scope = OsintScope::standard();
        let mut graph = EvidenceGraph::new();
        let alias_first = RawFindingRecord::new(
            "org:corp-example-labs",
            EntityKind::Organization,
            "corp-example-labs",
            "official_domain",
            "corp.example",
            FIXED_TIMESTAMP,
            820,
        )
        .with_alias_of("org:corp-example");
        graph
            .ingest_result(
                &scope,
                &ToolResult::new(
                    "c-first",
                    SourceKind::GitHub,
                    "corp-example-labs",
                    FIXED_TIMESTAMP,
                    vec![alias_first],
                ),
            )
            .unwrap();
        assert!(!graph.edges().iter().any(|e| e.kind == EdgeKind::SameAs));
        graph.validate().unwrap();

        let primary_later = RawFindingRecord::new(
            "org:corp-example",
            EntityKind::Organization,
            "Corp Example",
            "sector",
            "security-training",
            FIXED_TIMESTAMP,
            850,
        );
        graph
            .ingest_result(
                &scope,
                &ToolResult::new(
                    "c-second",
                    SourceKind::Whois,
                    "corp.example",
                    FIXED_TIMESTAMP,
                    vec![primary_later],
                ),
            )
            .unwrap();
        assert!(graph.edges().iter().any(|e| {
            e.kind == EdgeKind::SameAs
                && e.from == EntityId("org:corp-example-labs".to_string())
                && e.to == EntityId("org:corp-example".to_string())
        }));
        assert_eq!(graph.merge_same_as_entities().unwrap(), 1);
        graph.validate().unwrap();
    }

    #[test]
    fn report_validation_rejects_dropped_tampered_or_duplicate_conflicts() {
        let mut agent = MockOSINTAgent::standard();
        let report = agent.run().unwrap();
        assert!(!report.conflicts.is_empty());

        let mut cleared = report.clone();
        cleared.conflicts.clear();
        assert!(matches!(
            cleared.validate(agent.graph()),
            Err(OsintError::UnsupportedClaim(_))
        ));

        let mut dropped_cand = report.clone();
        dropped_cand.conflicts[0].candidates.pop();
        assert!(matches!(
            dropped_cand.validate(agent.graph()),
            Err(OsintError::UnsupportedClaim(_))
        ));

        let mut tampered_cand = report.clone();
        tampered_cand.conflicts[0].candidates[0].value = "tampered.example".to_string();
        assert!(matches!(
            tampered_cand.validate(agent.graph()),
            Err(OsintError::UnsupportedClaim(_))
        ));

        let mut dup_cand = report.clone();
        dup_cand.conflicts[0].candidates[1] = dup_cand.conflicts[0].candidates[0].clone();
        assert!(matches!(
            dup_cand.validate(agent.graph()),
            Err(OsintError::UnsupportedClaim(_))
        ));

        let mut dup_conflict = report;
        let first = dup_conflict.conflicts[0].clone();
        dup_conflict.conflicts.push(first);
        assert!(matches!(
            dup_conflict.validate(agent.graph()),
            Err(OsintError::ReportSchemaInvalid(_))
        ));
    }

    #[test]
    fn validate_raw_record_and_secret_filter_reject_unnormalized_mismatched_and_pii_inputs() {
        let scope = OsintScope::standard();

        let unnorm_domain = RawFindingRecord::new(
            "domain:api.corp.example",
            EntityKind::Domain,
            "API.CORP.EXAMPLE.",
            "a_record",
            "10.20.30.15",
            FIXED_TIMESTAMP,
            800,
        );
        assert!(matches!(
            validate_raw_record(&scope, &unnorm_domain),
            Err(OsintError::InvalidDomain(_))
        ));

        let mismatch_ip = RawFindingRecord::new(
            "ip:10.0.0.2",
            EntityKind::IpAddress,
            "10.0.0.1",
            "ptr_domain",
            "corp.example",
            FIXED_TIMESTAMP,
            800,
        );
        assert!(matches!(
            validate_raw_record(&scope, &mismatch_ip),
            Err(OsintError::InvalidTarget(_))
        ));

        for pii_attr in ["phone_number", "first_name"] {
            let rec = RawFindingRecord::new(
                "org:corp-example",
                EntityKind::Organization,
                "Corp Example",
                pii_attr,
                "synthetic-val",
                FIXED_TIMESTAMP,
                700,
            );
            assert!(matches!(
                validate_raw_record(&scope, &rec),
                Err(OsintError::PersonalDataProhibited(_))
            ));
        }

        let bad_role_attr = RawFindingRecord::new(
            "org:corp-example",
            EntityKind::Organization,
            "Corp Example",
            "role_contact",
            "Alice Nguyen",
            FIXED_TIMESTAMP,
            700,
        );
        assert!(matches!(
            validate_raw_record(&scope, &bad_role_attr),
            Err(OsintError::PersonalDataProhibited(_))
        ));

        assert!(matches!(
            validate_no_raw_secret("bearer tok-123456", None),
            Err(OsintError::RawSecretProhibited(_))
        ));
        assert!(matches!(
            validate_no_raw_secret("-----begin private key-----", None),
            Err(OsintError::RawSecretProhibited(_))
        ));
    }

    #[test]
    fn checkpoint_resume_rejects_inconsistent_counters_completed_stages_missing_seed_and_disallowed_source()
     {
        let mut agent = MockOSINTAgent::standard();
        agent.registry.inject_fault(
            SourceKind::Dns,
            "api.corp.example",
            SimulatedFault::TimeoutOnce,
        );
        assert!(agent.run().is_err());
        let valid_cp = agent.last_checkpoint().unwrap().clone();

        let mut zero_counter = valid_cp.clone();
        zero_counter.state.tool_calls_used = 0;
        assert!(matches!(
            agent.resume(&zero_counter),
            Err(OsintError::InvalidCheckpoint(_))
        ));

        let mut empty_completed_stage = valid_cp.clone();
        empty_completed_stage.completed_calls.clear();
        empty_completed_stage.graph = EvidenceGraph::new();
        empty_completed_stage
            .stage_statuses
            .insert(Stage::SeedCollection, StageStatus::Completed);
        assert!(matches!(
            agent.resume(&empty_completed_stage),
            Err(OsintError::InvalidCheckpoint(_))
        ));

        let mut missing_seed = valid_cp.clone();
        missing_seed
            .state
            .pending_targets
            .retain(|(tk, t)| !(*tk == ToolTargetKind::Domain && t == "corp.example"));
        missing_seed
            .state
            .visited_calls
            .retain(|(_, t)| t != "corp.example");
        assert!(matches!(
            agent.resume(&missing_seed),
            Err(OsintError::InvalidCheckpoint(_))
        ));

        let mut restricted = MockOSINTAgent::standard();
        restricted.scope.allowed_sources.remove(&SourceKind::Whois);
        assert!(matches!(
            restricted.resume(&valid_cp),
            Err(OsintError::InvalidCheckpoint(_))
        ));
        let mut disallowed_cp = valid_cp;
        disallowed_cp.input_hash = restricted.input_hash();
        assert_eq!(
            restricted.resume(&disallowed_cp).unwrap_err(),
            OsintError::ToolNotAllowed(SourceKind::Whois)
        );
    }

    #[test]
    fn zero_step_budget_halts_even_when_target_already_visited() {
        let mut agent = MockOSINTAgent::standard().with_budget(0, 32);
        agent
            .state
            .visited_calls
            .insert((SourceKind::Whois, "corp.example".to_string()));
        let report = agent.run().unwrap();
        assert_eq!(report.stop_reason, StopReason::BudgetExhausted);
        assert_eq!(agent.state().steps_used, 0);
    }

    #[test]
    fn ingest_result_is_atomic_on_mid_batch_error() {
        let scope = OsintScope::standard();
        let mut graph = EvidenceGraph::new();
        let rec0 = RawFindingRecord::new(
            "org:corp-example",
            EntityKind::Organization,
            "Corp Example",
            "official_domain",
            "corp.example",
            FIXED_TIMESTAMP,
            850,
        );
        graph
            .ingest_result(
                &scope,
                &ToolResult::new(
                    "c-seed",
                    SourceKind::Whois,
                    "corp.example",
                    FIXED_TIMESTAMP,
                    vec![rec0],
                ),
            )
            .unwrap();
        let before = graph.clone();

        let bad_batch = ToolResult::new(
            "c-bad-batch",
            SourceKind::Dns,
            "sub.corp.example",
            FIXED_TIMESTAMP,
            vec![
                RawFindingRecord::new(
                    "domain:sub.corp.example",
                    EntityKind::Domain,
                    "sub.corp.example",
                    "a_record",
                    "10.20.30.40",
                    FIXED_TIMESTAMP,
                    800,
                ),
                RawFindingRecord::new(
                    "org:corp-example",
                    EntityKind::Domain,
                    "corp.example",
                    "a_record",
                    "10.20.30.41",
                    FIXED_TIMESTAMP,
                    800,
                ),
            ],
        );
        assert!(graph.ingest_result(&scope, &bad_batch).is_err());
        assert_eq!(graph, before);
    }

    #[test]
    fn markdown_escape_escapes_backslash_before_pipe() {
        assert_eq!(markdown_escape(r"a\|b"), r"a\\\|b");
    }
}
