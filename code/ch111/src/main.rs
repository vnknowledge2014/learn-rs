#![forbid(unsafe_code)]

//! # Chương 111 — Methodology, Reporting, CVSS 3.1 & Legal (`MockReportingLab`)
//!
//! Crate minh họa 100% clean-room và thuần `std`:
//! - Quản trị ủy quyền kiểm thử: `Scope`, `RulesOfEngagement`, chữ ký chấp thuận,
//!   chấp thuận bên thứ ba, điều kiện dừng khẩn cấp (`StopConditionTrigger`) và nhật ký kiểm toán băm chuỗi.
//! - Vòng đời engagement 8 pha (`Prepare -> ... -> Retest`) có cổng kiểm tra **trước khi chạy module**.
//! - Chuỗi bằng chứng toàn vẹn (`EvidenceRef`, `EvidenceChain`) tự động che secret/PII, gắn SHA-256
//!   và cấm tuyệt đối đưa credential/secret thô vào báo cáo.
//! - Vòng đời `Finding` & `FindingStatus`, cấm khẳng định đã khai thác khi chưa xác minh,
//!   khử trùng lặp độc lập hoán vị (giữ nguyên toàn bộ `EvidenceRef`) và quy trình `Retest`.
//! - Bộ tính CVSS v3.1 Base Score chuẩn FIRST.org (có Appendix A `Roundup`) tách biệt với `BusinessPriority`.
//! - `Report` và `ReportRenderer` tất định (Executive Markdown, Technical Markdown, Full Markdown, JSON)
//!   bắt buộc `schema_version`, trích dẫn chuẩn và escape an toàn.

use std::collections::{BTreeMap, BTreeSet};
use std::fmt;
use std::str::FromStr;

pub const ENGAGEMENT_ID: &str = "ENG-2026-METH-111";
pub const REPORT_SCHEMA_VERSION: &str = "1.0.0";
pub const WINDOW_START_UTC: &str = "2026-02-01T00:00:00Z";
pub const WINDOW_END_UTC: &str = "2026-02-14T23:59:59Z";
pub const RETENTION_DEADLINE_UTC: &str = "2026-05-15T23:59:59Z";
pub const REPORT_GENERATED_UTC: &str = "2026-02-12T16:30:00Z";
pub const RETEST_TIME_UTC: &str = "2026-02-13T10:00:00Z";
pub const REDACTED_SECRET_MARKER: &str = "[REDACTED-SECRET]";
pub const REDACTED_PII_MARKER: &str = "[REDACTED-PII]";
pub const AUDIT_GENESIS_HASH: &str =
    "0000000000000000000000000000000000000000000000000000000000000000";

/// Hàm băm SHA-256 chuẩn FIPS 180-4 (thuần `std`) dùng cho bằng chứng, audit log và digest báo cáo.
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

// ============================================================================
// 1. LỖI, VALIDATORS RFC 2606 / RFC 1918 / ISO 8601 & CHỐNG LỘ SECRET / PII
// ============================================================================

#[derive(Debug, Clone, PartialEq, Eq)]
pub enum ReportingError {
    InvalidDomain(String),
    InvalidIpOrCidr(String),
    InvalidTimestamp(String),
    InvalidLocation(String),
    OutOfScope(String),
    ThirdPartyConsentMissing(String),
    AuthorizationMissing(String),
    OutsideEngagementWindow(String),
    ProhibitedActionBlocked(ProhibitedAction),
    MethodNotAuthorized(TestingMethod),
    HumanApprovalRequired(TestingMethod),
    StopConditionTriggered(StopConditionTrigger),
    InvalidPhaseTransition {
        from: EngagementPhase,
        to: EngagementPhase,
    },
    CleanupNotVerified(String),
    RawSecretProhibited(String),
    EvidenceInvalid(String),
    DuplicateIdentifier(String),
    InvalidCvssVector(String),
    IllegalFindingTransition {
        from: FindingStatus,
        to: FindingStatus,
    },
    UnverifiedExploitationClaim(String),
    DeduplicationConflict(String),
    ReportSchemaInvalid(String),
    AuditLogTampered(String),
    RetentionViolation(String),
}

impl fmt::Display for ReportingError {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            Self::InvalidDomain(msg) => write!(f, "InvalidDomain({msg})"),
            Self::InvalidIpOrCidr(msg) => write!(f, "InvalidIpOrCidr({msg})"),
            Self::InvalidTimestamp(msg) => write!(f, "InvalidTimestamp({msg})"),
            Self::InvalidLocation(msg) => write!(f, "InvalidLocation({msg})"),
            Self::OutOfScope(msg) => write!(f, "OutOfScope({msg})"),
            Self::ThirdPartyConsentMissing(msg) => write!(f, "ThirdPartyConsentMissing({msg})"),
            Self::AuthorizationMissing(msg) => write!(f, "AuthorizationMissing({msg})"),
            Self::OutsideEngagementWindow(msg) => write!(f, "OutsideEngagementWindow({msg})"),
            Self::ProhibitedActionBlocked(act) => write!(f, "ProhibitedActionBlocked({act:?})"),
            Self::MethodNotAuthorized(m) => write!(f, "MethodNotAuthorized({m:?})"),
            Self::HumanApprovalRequired(m) => write!(f, "HumanApprovalRequired({m:?})"),
            Self::StopConditionTriggered(t) => write!(f, "StopConditionTriggered({t:?})"),
            Self::InvalidPhaseTransition { from, to } => {
                write!(f, "InvalidPhaseTransition({from:?} -> {to:?})")
            }
            Self::CleanupNotVerified(msg) => write!(f, "CleanupNotVerified({msg})"),
            Self::RawSecretProhibited(msg) => write!(f, "RawSecretProhibited({msg})"),
            Self::EvidenceInvalid(msg) => write!(f, "EvidenceInvalid({msg})"),
            Self::DuplicateIdentifier(msg) => write!(f, "DuplicateIdentifier({msg})"),
            Self::InvalidCvssVector(msg) => write!(f, "InvalidCvssVector({msg})"),
            Self::IllegalFindingTransition { from, to } => {
                write!(f, "IllegalFindingTransition({from:?} -> {to:?})")
            }
            Self::UnverifiedExploitationClaim(msg) => {
                write!(f, "UnverifiedExploitationClaim({msg})")
            }
            Self::DeduplicationConflict(msg) => write!(f, "DeduplicationConflict({msg})"),
            Self::ReportSchemaInvalid(msg) => write!(f, "ReportSchemaInvalid({msg})"),
            Self::AuditLogTampered(msg) => write!(f, "AuditLogTampered({msg})"),
            Self::RetentionViolation(msg) => write!(f, "RetentionViolation({msg})"),
        }
    }
}

impl std::error::Error for ReportingError {}

/// Chuẩn hóa tên miền TLD `.example` (RFC 2606), từ chối mọi miền công khai ngoài lab.
pub fn normalize_example_domain(raw: &str) -> Result<String, ReportingError> {
    let trimmed = raw.trim();
    if trimmed.is_empty() || trimmed.len() > 253 {
        return Err(ReportingError::InvalidDomain(format!(
            "tên miền '{raw}' rỗng hoặc vượt 253 ký tự"
        )));
    }
    let lowered = trimmed.to_ascii_lowercase();
    let without_dot = lowered.strip_suffix('.').unwrap_or(&lowered);
    if without_dot.is_empty() || without_dot.starts_with('.') || without_dot.ends_with('.') {
        return Err(ReportingError::InvalidDomain(format!(
            "dấu chấm không hợp lệ ở '{raw}'"
        )));
    }
    let labels: Vec<&str> = without_dot.split('.').collect();
    if labels.len() < 2 {
        return Err(ReportingError::InvalidDomain(format!(
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
            return Err(ReportingError::InvalidDomain(format!(
                "nhãn không hợp lệ trong '{raw}'"
            )));
        }
    }
    if labels.last() != Some(&"example") {
        return Err(ReportingError::InvalidDomain(format!(
            "chỉ chấp nhận tên miền RFC 2606 .example, nhận '{raw}'"
        )));
    }
    Ok(without_dot.to_string())
}

fn domain_matches_zone(domain: &str, zone: &str) -> bool {
    domain == zone || domain.ends_with(&format!(".{zone}"))
}

/// Phân tích địa chỉ IPv4 dạng chuẩn (không khoảng trắng, không octet `010`).
pub fn parse_strict_ipv4(raw: &str) -> Result<u32, ReportingError> {
    let parts: Vec<&str> = raw.split('.').collect();
    if parts.len() != 4 {
        return Err(ReportingError::InvalidIpOrCidr(format!(
            "IPv4 không hợp lệ: '{raw}'"
        )));
    }
    let mut octets = [0u8; 4];
    for (idx, part) in parts.iter().enumerate() {
        if part.is_empty()
            || (part.len() > 1 && part.starts_with('0'))
            || !part.bytes().all(|b| b.is_ascii_digit())
        {
            return Err(ReportingError::InvalidIpOrCidr(format!(
                "octet IPv4 không hợp lệ trong '{raw}'"
            )));
        }
        let val: u8 = part.parse().map_err(|_| {
            ReportingError::InvalidIpOrCidr(format!("octet vượt quá 255 trong '{raw}'"))
        })?;
        octets[idx] = val;
    }
    Ok(u32::from_be_bytes(octets))
}

/// Kiểm tra địa chỉ IPv4 thuộc đúng dải riêng tư RFC 1918 (`10/8`, `172.16/12`, `192.168/16`).
pub fn is_rfc1918_ipv4(raw: &str) -> bool {
    let Ok(ip) = parse_strict_ipv4(raw) else {
        return false;
    };
    is_rfc1918_u32(ip)
}

fn is_rfc1918_u32(ip: u32) -> bool {
    let [a, b, _, _] = ip.to_be_bytes();
    a == 10 || (a == 172 && (16..=31).contains(&b)) || (a == 192 && b == 168)
}

/// Phân tích CIDR RFC 1918: yêu cầu toàn bộ không gian CIDR nằm gọn trong dải riêng tư
/// và không chứa bit host khác 0.
pub fn parse_rfc1918_cidr(raw: &str) -> Result<(u32, u8), ReportingError> {
    let (ip_str, prefix_str) = raw.split_once('/').ok_or_else(|| {
        ReportingError::InvalidIpOrCidr(format!("thiếu '/prefix' trong CIDR '{raw}'"))
    })?;
    if prefix_str.is_empty()
        || (prefix_str.len() > 1 && prefix_str.starts_with('0'))
        || !prefix_str.bytes().all(|b| b.is_ascii_digit())
    {
        return Err(ReportingError::InvalidIpOrCidr(format!(
            "prefix không hợp lệ trong CIDR '{raw}'"
        )));
    }
    let prefix: u8 = prefix_str
        .parse()
        .map_err(|_| ReportingError::InvalidIpOrCidr(format!("prefix tràn trong CIDR '{raw}'")))?;
    if !(8..=32).contains(&prefix) {
        return Err(ReportingError::InvalidIpOrCidr(format!(
            "prefix CIDR phải thuộc 8..=32 cho RFC 1918, nhận '{raw}'"
        )));
    }
    let net = parse_strict_ipv4(ip_str)?;
    let mask = if prefix == 0 {
        0
    } else {
        !0u32 << (32 - prefix)
    };
    if (net & !mask) != 0 {
        return Err(ReportingError::InvalidIpOrCidr(format!(
            "địa chỉ mạng '{raw}' chứa bit host khác 0"
        )));
    }
    let broadcast = net | !mask;
    if !is_rfc1918_u32(net) || !is_rfc1918_u32(broadcast) {
        return Err(ReportingError::InvalidIpOrCidr(format!(
            "CIDR '{raw}' vượt khỏi dải RFC 1918"
        )));
    }
    Ok((net, prefix))
}

fn ipv4_in_cidr(ip: u32, net: u32, prefix: u8) -> bool {
    let mask = if prefix == 0 {
        0
    } else {
        !0u32 << (32 - prefix)
    };
    (ip & mask) == net
}

/// Kiểm tra timestamp ISO 8601 UTC nghiêm ngặt dạng `YYYY-MM-DDTHH:MM:SSZ` (`year >= 1970`).
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
        Ok(y) if y >= 1970 => y,
        _ => return false,
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

/// Kiểm tra chuỗi hex SHA-256 gồm đúng 64 ký tự hex viết thường.
pub fn is_valid_sha256_hex(raw: &str) -> bool {
    raw.len() == 64
        && raw
            .bytes()
            .all(|b| b.is_ascii_digit() || (b'a'..=b'f').contains(&b))
}

/// Chuẩn hóa vị trí nguồn bằng chứng tổng hợp (`virtual://` hoặc `lab-sensor://`),
/// tuyệt đối không cho phép đường dẫn OS thật hay ký tự shell.
pub fn normalize_synthetic_source(raw: &str) -> Result<String, ReportingError> {
    if raw.contains('\\')
        || raw.contains("..")
        || raw.bytes().any(|b| b.is_ascii_control() || b == b' ')
    {
        return Err(ReportingError::InvalidLocation(format!(
            "nguồn bằng chứng '{raw}' chứa ký tự điều khiển, khoảng trắng hoặc '\\'/'..'"
        )));
    }
    if raw
        .chars()
        .any(|c| matches!(c, ';' | '&' | '|' | '$' | '`' | '(' | ')' | '<' | '>'))
    {
        return Err(ReportingError::InvalidLocation(format!(
            "nguồn bằng chứng '{raw}' chứa ký tự shell bị cấm"
        )));
    }
    let (scheme, rest) = if let Some(r) = raw.strip_prefix("virtual://") {
        ("virtual://", r)
    } else if let Some(r) = raw.strip_prefix("lab-sensor://") {
        ("lab-sensor://", r)
    } else {
        return Err(ReportingError::InvalidLocation(format!(
            "nguồn phải dùng virtual:// hoặc lab-sensor://, nhận '{raw}'"
        )));
    };
    let (raw_host, path) = rest.split_once('/').ok_or_else(|| {
        ReportingError::InvalidLocation(format!("nguồn '{raw}' thiếu subsystem/entry sau host"))
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
        return Err(ReportingError::InvalidLocation(format!(
            "đoạn path không hợp lệ trong '{raw}'"
        )));
    }
    Ok(format!("{scheme}{host}/{path}"))
}

fn is_role_contact_email(raw: &str) -> bool {
    let trimmed = raw.trim();
    let Some((local, domain)) = trimmed.split_once('@') else {
        return false;
    };
    if local.is_empty()
        || local.contains('@')
        || local.starts_with('.')
        || local.ends_with('.')
        || local.contains("..")
        || !local
            .bytes()
            .all(|b| b.is_ascii_alphanumeric() || matches!(b, b'.' | b'-' | b'_'))
    {
        return false;
    }
    normalize_example_domain(domain).is_ok()
}

fn is_redacted_marker_exact(val: &str) -> bool {
    val == REDACTED_SECRET_MARKER || val == "SYNTHETIC-REDACTED-CREDENTIAL"
}

fn extract_secret_value_span(bytes: &[u8], cursor: usize) -> (usize, usize, usize) {
    if cursor < bytes.len() && matches!(bytes[cursor], b'"' | b'\'') {
        let q = bytes[cursor];
        let inner_start = cursor + 1;
        let mut end = inner_start;
        let mut escaped = false;
        while end < bytes.len() {
            let b = bytes[end];
            if escaped {
                escaped = false;
            } else if b == b'\\' {
                escaped = true;
            } else if b == q || matches!(b, b'\n' | b'\r') {
                break;
            }
            end += 1;
        }
        let next_pos = if end < bytes.len() && bytes[end] == q {
            end + 1
        } else {
            end
        };
        (inner_start, end, next_pos)
    } else {
        let mut end = cursor;
        while end < bytes.len() && !bytes[end].is_ascii_whitespace() && bytes[end] != b')' {
            end += 1;
        }
        if end > cursor && matches!(bytes[end - 1], b',' | b';') {
            end -= 1;
        }
        (cursor, end, end)
    }
}

fn find_kv_secret_spans(text: &str) -> Vec<(usize, usize)> {
    const ASSIGN_KEYS: &[&str] = &[
        "password",
        "passwd",
        "pwd",
        "api_key",
        "apikey",
        "client_secret",
        "secret",
        "access_token",
        "refresh_token",
        "session_token",
        "ntlm_hash",
        "krb5tgs_hash",
    ];
    let lower = text.to_ascii_lowercase();
    let bytes = text.as_bytes();
    let mut spans = Vec::new();

    // 1. Key =/: Value (hỗ trợ cả YAML/JSON quoted keys `"password": "..."`, khoảng trắng quanh `=/ :`,
    //    xuống dòng/tab sau `=/:`, và giá trị có dấu cách trong ngoặc kép `password="my secret"`).
    for key in ASSIGN_KEYS {
        let mut search_from = 0usize;
        while let Some(rel) = lower[search_from..].find(key) {
            let start = search_from + rel;
            let open_quote = if start > 0 {
                let prev = bytes[start - 1];
                if prev.is_ascii_alphanumeric() {
                    search_from = start + 1;
                    continue;
                }
                if matches!(prev, b'"' | b'\'') {
                    Some(prev)
                } else {
                    None
                }
            } else {
                None
            };
            let mut cursor = start + key.len();
            if let Some(q) = open_quote
                && cursor < bytes.len()
                && bytes[cursor] == q
            {
                cursor += 1;
            }
            while cursor < bytes.len() && matches!(bytes[cursor], b' ' | b'\t') {
                cursor += 1;
            }
            if cursor >= bytes.len() || !matches!(bytes[cursor], b'=' | b':') {
                search_from = start + 1;
                continue;
            }
            cursor += 1;
            while cursor < bytes.len() && bytes[cursor].is_ascii_whitespace() {
                cursor += 1;
            }
            if cursor >= bytes.len() {
                search_from = cursor;
                continue;
            }
            let (val_start, val_end, after_val) = extract_secret_value_span(bytes, cursor);
            if val_end > val_start && !is_redacted_marker_exact(&text[val_start..val_end]) {
                spans.push((val_start, val_end));
            }
            search_from = after_val.max(start + 1);
        }
    }

    // 2. HTTP Bearer / Basic token (hỗ trợ khoảng trắng, tab, xuống dòng hoặc ngoặc kép sau Bearer / Basic).
    for kw in ["bearer", "basic"] {
        let mut search_from = 0usize;
        while let Some(rel) = lower[search_from..].find(kw) {
            let start = search_from + rel;
            if start > 0 {
                let prev = bytes[start - 1];
                if prev.is_ascii_alphanumeric() {
                    search_from = start + 1;
                    continue;
                }
            }
            if kw == "basic" {
                let prefix = lower[..start].trim_end();
                if !prefix.ends_with("authorization:") && !prefix.ends_with("authorization :") {
                    search_from = start + 1;
                    continue;
                }
            }
            let mut cursor = start + kw.len();
            if cursor >= bytes.len() || !bytes[cursor].is_ascii_whitespace() {
                search_from = start + 1;
                continue;
            }
            while cursor < bytes.len() && bytes[cursor].is_ascii_whitespace() {
                cursor += 1;
            }
            if cursor >= bytes.len() {
                search_from = cursor;
                continue;
            }
            let (val_start, val_end, after_val) = extract_secret_value_span(bytes, cursor);
            if val_end > val_start && !is_redacted_marker_exact(&text[val_start..val_end]) {
                spans.push((val_start, val_end));
            }
            search_from = after_val.max(start + 1);
        }
    }

    spans
}

fn find_pattern_secret_spans(text: &str) -> Vec<(usize, usize)> {
    let mut spans = find_kv_secret_spans(text);
    let bytes = text.as_bytes();
    let lower = text.to_ascii_lowercase();

    // Private key PEM headers (dùng while let để che MỌI khối liên tiếp trong cùng capture)
    for marker in [
        "-----begin rsa private key-----",
        "-----begin private key-----",
        "-----begin openssh private key-----",
        "-----begin ec private key-----",
        "-----begin dsa private key-----",
        "-----begin encrypted private key-----",
        "-----begin pgp private key block-----",
    ] {
        let mut from = 0usize;
        while let Some(rel) = lower[from..].find(marker) {
            let pos = from + rel;
            let end = lower[pos..]
                .find("-----end")
                .and_then(|end_rel| {
                    let after_end = pos + end_rel;
                    lower[after_end..]
                        .find("-----")
                        .and_then(|tail| {
                            let tail_start = after_end + tail;
                            lower[tail_start + 5..]
                                .find("-----")
                                .map(|fin| tail_start + 5 + fin + 5)
                        })
                        .or_else(|| lower[after_end..].find('\n').map(|e| after_end + e))
                })
                .unwrap_or(text.len());
            spans.push((pos, end));
            from = end.max(pos + marker.len());
        }
    }

    // Synthetic prefix tokens / Kerberos / NTLM hashes / JWT-like blobs
    for prefix in [
        "syn_tok_",
        "ghp_",
        "glpat-",
        "akia",
        "krb5tgs_hash:",
        "ntlm_hash:",
        "eyj",
    ] {
        let mut from = 0usize;
        while let Some(rel) = lower[from..].find(prefix) {
            let pos = from + rel;
            if pos > 0 && (bytes[pos - 1].is_ascii_alphanumeric() || bytes[pos - 1] == b'_') {
                from = pos + 1;
                continue;
            }
            let mut end = pos + prefix.len();
            while end < bytes.len()
                && !matches!(
                    bytes[end],
                    b' ' | b'\n' | b'\r' | b'\t' | b',' | b';' | b')' | b']' | b'"' | b'\''
                )
            {
                end += 1;
            }
            let token_len = end - pos;
            let suspicious_jwt = prefix == "eyj" && text[pos..end].matches('.').count() >= 2;
            let long_token = token_len >= prefix.len() + 6 && prefix != "eyj";
            if suspicious_jwt || long_token {
                spans.push((pos, end));
            }
            from = end.max(pos + 1);
        }
    }
    spans
}

fn find_non_example_email_spans(text: &str) -> Vec<(usize, usize)> {
    let bytes = text.as_bytes();
    let mut spans = Vec::new();
    for (idx, &b) in bytes.iter().enumerate() {
        if b != b'@' {
            continue;
        }
        let mut start = idx;
        while start > 0
            && (bytes[start - 1].is_ascii_alphanumeric()
                || matches!(bytes[start - 1], b'.' | b'-' | b'_' | b'+'))
        {
            start -= 1;
        }
        let mut end = idx + 1;
        while end < bytes.len()
            && (bytes[end].is_ascii_alphanumeric() || matches!(bytes[end], b'.' | b'-'))
        {
            end += 1;
        }
        while end > idx + 1 && bytes[end - 1] == b'.' {
            end -= 1;
        }
        if start < idx && end > idx + 1 {
            let domain = &text[idx + 1..end];
            if domain.contains('.') && normalize_example_domain(domain).is_err() {
                spans.push((start, end));
            }
        }
    }
    spans
}

fn merge_spans(mut spans: Vec<(usize, usize)>) -> Vec<(usize, usize)> {
    if spans.is_empty() {
        return spans;
    }
    spans.sort_unstable();
    let mut merged = vec![spans[0]];
    for (s, e) in spans.into_iter().skip(1) {
        let last = merged.last_mut().expect("non-empty");
        if s <= last.1 {
            last.1 = last.1.max(e);
        } else {
            merged.push((s, e));
        }
    }
    merged
}

/// Tự động che secret/credential tổng hợp và email cá nhân ngoài `.example`,
/// trả về văn bản đã làm sạch cùng số mục đã che.
pub fn redact_sensitive_text(raw: &str) -> (String, usize) {
    let mut tagged: Vec<(usize, usize, &'static str)> = Vec::new();
    for (s, e) in merge_spans(find_pattern_secret_spans(raw)) {
        tagged.push((s, e, REDACTED_SECRET_MARKER));
    }
    for (s, e) in merge_spans(find_non_example_email_spans(raw)) {
        let covered = tagged.iter().any(|&(ts, te, _)| !(e <= ts || s >= te));
        if !covered {
            tagged.push((s, e, REDACTED_PII_MARKER));
        }
    }
    tagged.sort_by_key(|item| item.0);
    if tagged.is_empty() {
        return (raw.to_string(), 0);
    }
    let mut out = String::with_capacity(raw.len());
    let mut cursor = 0usize;
    let mut count = 0usize;
    for (s, e, marker) in tagged {
        if s >= cursor {
            out.push_str(&raw[cursor..s]);
            out.push_str(marker);
            cursor = e;
            count += 1;
        }
    }
    out.push_str(&raw[cursor..]);
    (out, count)
}

/// Cấm tuyệt đối mọi secret/token/hash thô hoặc email cá nhân ngoài `.example` lọt vào dữ liệu/báo cáo.
pub fn validate_no_raw_secret(field_name: &str, raw: &str) -> Result<(), ReportingError> {
    if !find_pattern_secret_spans(raw).is_empty() {
        return Err(ReportingError::RawSecretProhibited(format!(
            "phát hiện chuỗi nhạy cảm chưa che trong '{field_name}'"
        )));
    }
    if !find_non_example_email_spans(raw).is_empty() {
        return Err(ReportingError::RawSecretProhibited(format!(
            "phát hiện địa chỉ email ngoài .example trong '{field_name}'"
        )));
    }
    Ok(())
}

// ============================================================================
// 2. PHẠM VI (SCOPE), QUY TẮC THAM GIA (ROE) & VÒNG ĐỜI ENGAGEMENT
// ============================================================================

#[derive(Debug, Clone, PartialEq, Eq, PartialOrd, Ord)]
pub enum TargetAsset {
    Domain(String),
    IpV4(String),
}

impl TargetAsset {
    pub fn canonical_label(&self) -> Result<String, ReportingError> {
        match self {
            Self::Domain(d) => normalize_example_domain(d),
            Self::IpV4(ip) => {
                let parsed = parse_strict_ipv4(ip)?;
                let [a, b, c, d] = parsed.to_be_bytes();
                Ok(format!("{a}.{b}.{c}.{d}"))
            }
        }
    }
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct ThirdPartyAuthorization {
    pub host: String,
    pub owner_org: String,
    pub written_consent_ref: String,
    pub approved_until_utc: String,
}

impl ThirdPartyAuthorization {
    pub fn validate(&self) -> Result<(), ReportingError> {
        let _ = normalize_example_domain(&self.host)?;
        if self.owner_org.trim().is_empty() {
            return Err(ReportingError::ThirdPartyConsentMissing(
                "owner_org của bên thứ ba rỗng".to_string(),
            ));
        }
        validate_no_raw_secret("third_party.owner_org", &self.owner_org)?;
        if self.written_consent_ref.trim().is_empty() {
            return Err(ReportingError::ThirdPartyConsentMissing(format!(
                "thiếu written_consent_ref cho bên thứ ba '{}'",
                self.host
            )));
        }
        validate_no_raw_secret("third_party.written_consent_ref", &self.written_consent_ref)?;
        if !is_valid_iso8601_utc(&self.approved_until_utc) {
            return Err(ReportingError::InvalidTimestamp(
                self.approved_until_utc.clone(),
            ));
        }
        Ok(())
    }
}

fn parse_scope_target_str(raw: &str) -> Result<TargetAsset, ReportingError> {
    let trimmed = raw.trim();
    if trimmed.contains(':') {
        return Err(ReportingError::InvalidIpOrCidr(format!(
            "target '{raw}' không được kèm port hay IPv6"
        )));
    }
    let parts: Vec<&str> = trimmed.split('.').collect();
    if parts.len() == 4
        && parts
            .iter()
            .all(|p| !p.is_empty() && p.bytes().all(|b| b.is_ascii_digit()))
    {
        let _ = parse_strict_ipv4(trimmed)?;
        return Ok(TargetAsset::IpV4(trimmed.to_string()));
    }
    Ok(TargetAsset::Domain(trimmed.to_string()))
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct Scope {
    pub allowed_domains: BTreeSet<String>,
    pub allow_subdomains_of: BTreeSet<String>,
    pub denied_domains: BTreeSet<String>,
    pub allowed_cidrs: BTreeSet<String>,
    pub denied_ips: BTreeSet<String>,
    pub third_party_assets: BTreeMap<String, ThirdPartyAuthorization>,
}

impl Scope {
    pub fn validate(&self) -> Result<(), ReportingError> {
        for d in self
            .allowed_domains
            .iter()
            .chain(self.allow_subdomains_of.iter())
            .chain(self.denied_domains.iter())
        {
            let _ = normalize_example_domain(d)?;
        }
        for cidr in &self.allowed_cidrs {
            let _ = parse_rfc1918_cidr(cidr)?;
        }
        for ip in &self.denied_ips {
            let _ = parse_strict_ipv4(ip)?;
        }
        for (key, auth) in &self.third_party_assets {
            let normalized = normalize_example_domain(key)?;
            let auth_host = normalize_example_domain(&auth.host)?;
            if normalized != auth_host {
                return Err(ReportingError::ThirdPartyConsentMissing(format!(
                    "khóa third_party '{key}' không khớp host '{}'",
                    auth.host
                )));
            }
            if auth.written_consent_ref.trim().is_empty() {
                return Err(ReportingError::ThirdPartyConsentMissing(format!(
                    "thiếu written_consent_ref cho '{normalized}'"
                )));
            }
            auth.validate()?;
        }
        if self.allowed_domains.is_empty()
            && self.allow_subdomains_of.is_empty()
            && self.allowed_cidrs.is_empty()
        {
            return Err(ReportingError::OutOfScope(
                "scope không có bất kỳ domain hay CIDR được phép nào".to_string(),
            ));
        }
        Ok(())
    }

    /// Kiểm tra một mục tiêu tại mốc `at_utc`:
    /// 1) Luật Deny luôn thắng Allow;
    /// 2) Tài sản phụ thuộc bên thứ ba bắt buộc có văn bản chấp thuận còn hạn;
    /// 3) Phải khớp AllowList cụ thể.
    pub fn check_target(&self, target: &TargetAsset, at_utc: &str) -> Result<(), ReportingError> {
        if !is_valid_iso8601_utc(at_utc) {
            return Err(ReportingError::InvalidTimestamp(at_utc.to_string()));
        }
        match target {
            TargetAsset::Domain(raw) => {
                let domain = normalize_example_domain(raw)?;
                for denied in &self.denied_domains {
                    let denied_norm = normalize_example_domain(denied)?;
                    if domain_matches_zone(&domain, &denied_norm) {
                        return Err(ReportingError::OutOfScope(format!(
                            "mục tiêu '{domain}' nằm trong vùng cấm '{denied_norm}'"
                        )));
                    }
                }
                for (tp_key, auth) in &self.third_party_assets {
                    let tp_norm = normalize_example_domain(tp_key)?;
                    if domain_matches_zone(&domain, &tp_norm) {
                        auth.validate()?;
                        let auth_host = normalize_example_domain(&auth.host)?;
                        if auth_host != tp_norm {
                            return Err(ReportingError::ThirdPartyConsentMissing(format!(
                                "khóa third_party '{tp_norm}' không khớp host '{auth_host}'"
                            )));
                        }
                        if at_utc > auth.approved_until_utc.as_str() {
                            return Err(ReportingError::ThirdPartyConsentMissing(format!(
                                "chấp thuận bên thứ ba cho '{domain}' đã hết hạn lúc {} (yêu cầu tại {at_utc})",
                                auth.approved_until_utc
                            )));
                        }
                    }
                }
                let exact_allowed = self
                    .allowed_domains
                    .iter()
                    .any(|d| normalize_example_domain(d).ok().as_deref() == Some(domain.as_str()));
                let zone_allowed = self.allow_subdomains_of.iter().any(|z| {
                    normalize_example_domain(z)
                        .is_ok_and(|zone| domain_matches_zone(&domain, &zone))
                });
                if !exact_allowed && !zone_allowed {
                    return Err(ReportingError::OutOfScope(format!(
                        "mục tiêu '{domain}' không nằm trong allowed_domains/allow_subdomains_of"
                    )));
                }
                Ok(())
            }
            TargetAsset::IpV4(raw) => {
                let ip = parse_strict_ipv4(raw)?;
                for denied in &self.denied_ips {
                    if parse_strict_ipv4(denied)? == ip {
                        return Err(ReportingError::OutOfScope(format!(
                            "IP '{raw}' nằm trong denied_ips"
                        )));
                    }
                }
                if !is_rfc1918_u32(ip) {
                    return Err(ReportingError::OutOfScope(format!(
                        "IP '{raw}' không thuộc dải RFC 1918 trong phạm vi lab"
                    )));
                }
                let allowed = self.allowed_cidrs.iter().any(|cidr| {
                    parse_rfc1918_cidr(cidr)
                        .is_ok_and(|(net, prefix)| ipv4_in_cidr(ip, net, prefix))
                });
                if !allowed {
                    return Err(ReportingError::OutOfScope(format!(
                        "IP '{raw}' nằm ngoài allowed_cidrs"
                    )));
                }
                Ok(())
            }
        }
    }
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, PartialOrd, Ord)]
pub enum TestingMethod {
    PassiveRecon,
    ConfigAudit,
    VulnerabilityValidation,
    ControlledLabExploit,
    PostExploitReadOnlyAudit,
    CleanupVerification,
    RetestVerification,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, PartialOrd, Ord)]
pub enum ProhibitedAction {
    DenialOfService,
    SocialEngineeringPhishing,
    CredentialDumping,
    DestructivePayload,
    UnapprovedThirdPartyPivot,
    ExternalExfiltration,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, PartialOrd, Ord)]
pub enum StopConditionTrigger {
    ProductionAvailabilityDegradation,
    UnintendedDataExposure,
    ActiveIncidentResponseDetected,
    ScopeBoundaryViolation,
    EmergencyClientAbort,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct DataHandlingPolicy {
    pub data_minimization_enforced: bool,
    pub redact_secrets_and_pii: bool,
    pub retention_days: u16,
    pub deletion_deadline_utc: String,
    pub storage_classification: String,
}

impl DataHandlingPolicy {
    pub fn validate(&self, window_end_utc: &str) -> Result<(), ReportingError> {
        if !self.data_minimization_enforced {
            return Err(ReportingError::RetentionViolation(
                "bắt buộc bật data_minimization_enforced".to_string(),
            ));
        }
        if !self.redact_secrets_and_pii {
            return Err(ReportingError::RetentionViolation(
                "bắt buộc bật redact_secrets_and_pii".to_string(),
            ));
        }
        if !(1..=365).contains(&self.retention_days) {
            return Err(ReportingError::RetentionViolation(format!(
                "retention_days={} nằm ngoài 1..=365",
                self.retention_days
            )));
        }
        if !is_valid_iso8601_utc(&self.deletion_deadline_utc) {
            return Err(ReportingError::InvalidTimestamp(
                self.deletion_deadline_utc.clone(),
            ));
        }
        if self.deletion_deadline_utc.as_str() < window_end_utc {
            return Err(ReportingError::RetentionViolation(
                "deletion_deadline_utc không được sớm hơn window_end_utc".to_string(),
            ));
        }
        if self.storage_classification.trim().is_empty() {
            return Err(ReportingError::RetentionViolation(
                "storage_classification rỗng".to_string(),
            ));
        }
        validate_no_raw_secret(
            "data_handling.storage_classification",
            &self.storage_classification,
        )?;
        Ok(())
    }
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct DisclosurePolicy {
    pub mode: String,
    pub coordination_window_days: u16,
    pub embargo_until_utc: String,
    pub remediation_contact: String,
}

impl DisclosurePolicy {
    pub fn validate(&self) -> Result<(), ReportingError> {
        if self.mode.trim().is_empty() || self.coordination_window_days == 0 {
            return Err(ReportingError::AuthorizationMissing(
                "disclosure policy thiếu mode hoặc coordination_window_days".to_string(),
            ));
        }
        if !is_valid_iso8601_utc(&self.embargo_until_utc) {
            return Err(ReportingError::InvalidTimestamp(
                self.embargo_until_utc.clone(),
            ));
        }
        if !is_role_contact_email(&self.remediation_contact) {
            return Err(ReportingError::RawSecretProhibited(
                "remediation_contact phải là role email thuộc .example".to_string(),
            ));
        }
        validate_no_raw_secret("disclosure.mode", &self.mode)?;
        Ok(())
    }
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct RulesOfEngagement {
    pub engagement_id: String,
    pub client_org: String,
    pub signed_written_consent: bool,
    pub authorization_ref: String,
    pub authorized_operator_contact: String,
    pub emergency_client_contact: String,
    pub window_start_utc: String,
    pub window_end_utc: String,
    pub allowed_methods: BTreeSet<TestingMethod>,
    pub prohibited_actions: BTreeSet<ProhibitedAction>,
    pub stop_conditions: BTreeSet<StopConditionTrigger>,
    pub agent_human_approval_required_for: BTreeSet<TestingMethod>,
    pub data_handling: DataHandlingPolicy,
    pub disclosure: DisclosurePolicy,
}

impl RulesOfEngagement {
    pub fn validate(&self) -> Result<(), ReportingError> {
        if self.engagement_id.trim().is_empty() || self.client_org.trim().is_empty() {
            return Err(ReportingError::AuthorizationMissing(
                "engagement_id hoặc client_org rỗng".to_string(),
            ));
        }
        validate_no_raw_secret("roe.engagement_id", &self.engagement_id)?;
        validate_no_raw_secret("roe.client_org", &self.client_org)?;
        if !self.signed_written_consent || self.authorization_ref.trim().is_empty() {
            return Err(ReportingError::AuthorizationMissing(
                "thiếu ủy quyền bằng văn bản đã ký (signed_written_consent / authorization_ref)"
                    .to_string(),
            ));
        }
        validate_no_raw_secret("roe.authorization_ref", &self.authorization_ref)?;
        if !is_role_contact_email(&self.authorized_operator_contact)
            || !is_role_contact_email(&self.emergency_client_contact)
        {
            return Err(ReportingError::RawSecretProhibited(
                "liên hệ RoE phải là địa chỉ vai trò tổ chức thuộc .example".to_string(),
            ));
        }
        if !is_valid_iso8601_utc(&self.window_start_utc)
            || !is_valid_iso8601_utc(&self.window_end_utc)
        {
            return Err(ReportingError::InvalidTimestamp(
                "window_start_utc hoặc window_end_utc không hợp lệ".to_string(),
            ));
        }
        if self.window_start_utc >= self.window_end_utc {
            return Err(ReportingError::OutsideEngagementWindow(
                "window_start_utc phải nhỏ hơn window_end_utc".to_string(),
            ));
        }
        if self.allowed_methods.is_empty() || self.stop_conditions.is_empty() {
            return Err(ReportingError::AuthorizationMissing(
                "allowed_methods và stop_conditions không được rỗng".to_string(),
            ));
        }
        self.data_handling.validate(&self.window_end_utc)?;
        self.disclosure.validate()?;
        Ok(())
    }
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, PartialOrd, Ord)]
pub enum EngagementPhase {
    Prepare,
    Recon,
    Validate,
    LabExploit,
    PostExploitationAudit,
    Cleanup,
    Report,
    Retest,
}

impl EngagementPhase {
    pub fn as_str(self) -> &'static str {
        match self {
            Self::Prepare => "Prepare",
            Self::Recon => "Recon",
            Self::Validate => "Validate",
            Self::LabExploit => "LabExploit",
            Self::PostExploitationAudit => "PostExploitationAudit",
            Self::Cleanup => "Cleanup",
            Self::Report => "Report",
            Self::Retest => "Retest",
        }
    }

    pub fn is_valid_transition(self, next: Self) -> bool {
        matches!(
            (self, next),
            (Self::Prepare, Self::Recon)
                | (Self::Recon, Self::Validate)
                | (Self::Validate, Self::LabExploit)
                | (Self::Validate, Self::PostExploitationAudit)
                | (Self::Validate, Self::Cleanup)
                | (Self::LabExploit, Self::PostExploitationAudit)
                | (Self::LabExploit, Self::Cleanup)
                | (Self::PostExploitationAudit, Self::Cleanup)
                | (Self::Cleanup, Self::Report)
                | (Self::Report, Self::Retest)
        )
    }

    pub fn allows_method(self, method: TestingMethod) -> bool {
        matches!(
            (self, method),
            (
                Self::Recon,
                TestingMethod::PassiveRecon | TestingMethod::ConfigAudit
            ) | (
                Self::Validate,
                TestingMethod::ConfigAudit | TestingMethod::VulnerabilityValidation
            ) | (Self::LabExploit, TestingMethod::ControlledLabExploit)
                | (
                    Self::PostExploitationAudit,
                    TestingMethod::PostExploitReadOnlyAudit
                )
                | (Self::Cleanup, TestingMethod::CleanupVerification)
                | (Self::Retest, TestingMethod::RetestVerification)
        )
    }
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct AgentApprovalRecord {
    pub approval_id: String,
    pub approver_role: String,
    pub method: TestingMethod,
    pub target: TargetAsset,
    pub approved_at_utc: String,
    pub expires_at_utc: String,
    pub justification: String,
}

impl AgentApprovalRecord {
    pub fn validate(&self) -> Result<(), ReportingError> {
        if self.approval_id.trim().is_empty()
            || self.approver_role.trim().is_empty()
            || self.justification.trim().is_empty()
        {
            return Err(ReportingError::AuthorizationMissing(
                "AgentApprovalRecord thiếu approval_id, approver_role hoặc justification"
                    .to_string(),
            ));
        }
        validate_no_raw_secret("approval.id", &self.approval_id)?;
        validate_no_raw_secret("approval.role", &self.approver_role)?;
        validate_no_raw_secret("approval.justification", &self.justification)?;
        let _ = self.target.canonical_label()?;
        if !is_valid_iso8601_utc(&self.approved_at_utc)
            || !is_valid_iso8601_utc(&self.expires_at_utc)
        {
            return Err(ReportingError::InvalidTimestamp(
                "timestamp trong AgentApprovalRecord không hợp lệ".to_string(),
            ));
        }
        if self.expires_at_utc < self.approved_at_utc {
            return Err(ReportingError::InvalidTimestamp(
                "expires_at_utc nhỏ hơn approved_at_utc".to_string(),
            ));
        }
        Ok(())
    }
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum AuditDecision {
    Allowed,
    Blocked,
    StopConditionHalted,
    PhaseTransition,
    FindingRetested,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct AuditLogEntry {
    pub sequence: u64,
    pub timestamp_utc: String,
    pub phase: EngagementPhase,
    pub actor: String,
    pub action: String,
    pub target_label: String,
    pub decision: AuditDecision,
    pub detail: String,
    pub prev_hash: String,
    pub entry_hash: String,
}

impl AuditLogEntry {
    fn compute_hash(&self) -> String {
        let decision_str = match self.decision {
            AuditDecision::Allowed => "Allowed",
            AuditDecision::Blocked => "Blocked",
            AuditDecision::StopConditionHalted => "StopConditionHalted",
            AuditDecision::PhaseTransition => "PhaseTransition",
            AuditDecision::FindingRetested => "FindingRetested",
        };
        let mut payload = String::new();
        push_lp(&mut payload, &self.sequence.to_string());
        push_lp(&mut payload, &self.timestamp_utc);
        push_lp(&mut payload, self.phase.as_str());
        push_lp(&mut payload, &self.actor);
        push_lp(&mut payload, &self.action);
        push_lp(&mut payload, &self.target_label);
        push_lp(&mut payload, decision_str);
        push_lp(&mut payload, &self.detail);
        push_lp(&mut payload, &self.prev_hash);
        sha256_hex(payload.as_bytes())
    }
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct ModuleStepRequest {
    pub step_id: String,
    pub actor: String,
    pub method: TestingMethod,
    pub target: TargetAsset,
    pub timestamp_utc: String,
    pub prohibited_action: Option<ProhibitedAction>,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct ModuleStepOutcome {
    pub step_id: String,
    pub target_label: String,
    pub summary: String,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct CleanupAttestation {
    pub plan_id: String,
    pub verified_clean: bool,
    pub remaining_artifacts: u32,
    pub attested_by_role: String,
    pub attested_at_utc: String,
}

impl CleanupAttestation {
    pub fn validate(&self) -> Result<(), ReportingError> {
        if self.plan_id.trim().is_empty() || self.attested_by_role.trim().is_empty() {
            return Err(ReportingError::CleanupNotVerified(
                "CleanupAttestation thiếu plan_id hoặc attested_by_role".to_string(),
            ));
        }
        validate_no_raw_secret("cleanup.plan_id", &self.plan_id)?;
        validate_no_raw_secret("cleanup.attested_by_role", &self.attested_by_role)?;
        if !is_valid_iso8601_utc(&self.attested_at_utc) {
            return Err(ReportingError::InvalidTimestamp(
                self.attested_at_utc.clone(),
            ));
        }
        Ok(())
    }
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum RetentionDisposition {
    ActiveWindow,
    WithinRetentionPeriod,
    DeletionRequiredNow,
    PurgedConfirmed,
}

// ============================================================================
// 3. CHUỖI BẰNG CHỨNG (`EvidenceRef`, `EvidenceChain`)
// ============================================================================

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct EvidenceRef {
    pub evidence_id: String,
    pub captured_at_utc: String,
    pub source_uri: String,
    pub artifact_sha256: String,
    pub redacted_excerpt: String,
    pub redaction_count: usize,
}

impl EvidenceRef {
    /// Tạo `EvidenceRef` từ capture thô: tự động che secret/PII trước khi băm và lưu excerpt.
    pub fn from_raw_capture(
        evidence_id: &str,
        captured_at_utc: &str,
        source_uri: &str,
        raw_payload: &str,
    ) -> Result<Self, ReportingError> {
        if evidence_id.trim().is_empty() {
            return Err(ReportingError::EvidenceInvalid(
                "evidence_id không được rỗng".to_string(),
            ));
        }
        validate_no_raw_secret("evidence_id", evidence_id)?;
        if !is_valid_iso8601_utc(captured_at_utc) {
            return Err(ReportingError::InvalidTimestamp(
                captured_at_utc.to_string(),
            ));
        }
        let normalized_uri = normalize_synthetic_source(source_uri)?;
        let (redacted_excerpt, redaction_count) = redact_sensitive_text(raw_payload);
        let artifact_sha256 = sha256_hex(redacted_excerpt.as_bytes());
        let item = Self {
            evidence_id: evidence_id.to_string(),
            captured_at_utc: captured_at_utc.to_string(),
            source_uri: normalized_uri,
            artifact_sha256,
            redacted_excerpt,
            redaction_count,
        };
        item.validate()?;
        Ok(item)
    }

    pub fn validate(&self) -> Result<(), ReportingError> {
        if self.evidence_id.trim().is_empty() {
            return Err(ReportingError::EvidenceInvalid(
                "evidence_id không được rỗng".to_string(),
            ));
        }
        validate_no_raw_secret("evidence.evidence_id", &self.evidence_id)?;
        if !is_valid_iso8601_utc(&self.captured_at_utc) {
            return Err(ReportingError::InvalidTimestamp(
                self.captured_at_utc.clone(),
            ));
        }
        let normalized_uri = normalize_synthetic_source(&self.source_uri)?;
        validate_no_raw_secret("evidence.source_uri", &normalized_uri)?;
        if !is_valid_sha256_hex(&self.artifact_sha256) {
            return Err(ReportingError::EvidenceInvalid(format!(
                "artifact_sha256 không hợp lệ cho '{}'",
                self.evidence_id
            )));
        }
        if self.redacted_excerpt.trim().is_empty() {
            return Err(ReportingError::EvidenceInvalid(format!(
                "redacted_excerpt rỗng cho '{}'",
                self.evidence_id
            )));
        }
        validate_no_raw_secret("evidence.redacted_excerpt", &self.redacted_excerpt)?;
        let expected_sha = sha256_hex(self.redacted_excerpt.as_bytes());
        if self.artifact_sha256 != expected_sha {
            return Err(ReportingError::EvidenceInvalid(format!(
                "artifact_sha256 của '{}' không khớp redacted_excerpt",
                self.evidence_id
            )));
        }
        Ok(())
    }
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct EvidenceLink {
    pub sequence: u64,
    pub evidence: EvidenceRef,
    pub prev_hash: String,
    pub chain_hash: String,
}

impl EvidenceLink {
    fn compute_hash(&self) -> String {
        let mut payload = String::new();
        push_lp(&mut payload, &self.sequence.to_string());
        push_lp(&mut payload, &self.evidence.evidence_id);
        push_lp(&mut payload, &self.evidence.captured_at_utc);
        push_lp(&mut payload, &self.evidence.source_uri);
        push_lp(&mut payload, &self.evidence.artifact_sha256);
        push_lp(&mut payload, &self.evidence.redacted_excerpt);
        push_lp(&mut payload, &self.evidence.redaction_count.to_string());
        push_lp(&mut payload, &self.prev_hash);
        sha256_hex(payload.as_bytes())
    }
}

#[derive(Debug, Clone, Default, PartialEq, Eq)]
pub struct EvidenceChain {
    links: Vec<EvidenceLink>,
    by_id: BTreeMap<String, usize>,
}

impl EvidenceChain {
    pub fn new() -> Self {
        Self::default()
    }

    pub fn append(&mut self, evidence: EvidenceRef) -> Result<(), ReportingError> {
        evidence.validate()?;
        if self.by_id.contains_key(&evidence.evidence_id) {
            return Err(ReportingError::DuplicateIdentifier(format!(
                "evidence_id '{}' đã tồn tại trong EvidenceChain",
                evidence.evidence_id
            )));
        }
        if let Some(prev) = self.links.last()
            && evidence.captured_at_utc < prev.evidence.captured_at_utc
        {
            return Err(ReportingError::InvalidTimestamp(format!(
                "timestamp bằng chứng '{}' ({}) sớm hơn mục trước ({})",
                evidence.evidence_id, evidence.captured_at_utc, prev.evidence.captured_at_utc
            )));
        }
        let prev_hash = self
            .links
            .last()
            .map_or_else(|| AUDIT_GENESIS_HASH.to_string(), |l| l.chain_hash.clone());
        let sequence = self.links.len() as u64;
        let mut link = EvidenceLink {
            sequence,
            evidence,
            prev_hash,
            chain_hash: String::new(),
        };
        link.chain_hash = link.compute_hash();
        self.by_id
            .insert(link.evidence.evidence_id.clone(), self.links.len());
        self.links.push(link);
        Ok(())
    }

    pub fn get(&self, evidence_id: &str) -> Option<&EvidenceRef> {
        self.by_id
            .get(evidence_id)
            .and_then(|&idx| self.links.get(idx))
            .map(|l| &l.evidence)
    }

    pub fn links(&self) -> &[EvidenceLink] {
        &self.links
    }

    pub fn links_mut_for_tamper_test(&mut self) -> &mut Vec<EvidenceLink> {
        &mut self.links
    }

    pub fn verify_integrity(&self) -> Result<(), ReportingError> {
        let mut expected_prev = AUDIT_GENESIS_HASH;
        let mut seen_ids = BTreeSet::new();
        for (idx, link) in self.links.iter().enumerate() {
            if link.sequence != idx as u64 {
                return Err(ReportingError::EvidenceInvalid(format!(
                    "thứ tự EvidenceChain sai tại vị trí {idx}"
                )));
            }
            link.evidence.validate()?;
            if !seen_ids.insert(link.evidence.evidence_id.as_str()) {
                return Err(ReportingError::DuplicateIdentifier(format!(
                    "trùng evidence_id '{}' trong chuỗi",
                    link.evidence.evidence_id
                )));
            }
            if idx > 0
                && link.evidence.captured_at_utc < self.links[idx - 1].evidence.captured_at_utc
            {
                return Err(ReportingError::EvidenceInvalid(
                    "timestamp trong EvidenceChain bị đảo ngược".to_string(),
                ));
            }
            if link.prev_hash != expected_prev {
                return Err(ReportingError::EvidenceInvalid(format!(
                    "prev_hash của '{}' bị gãy liên kết",
                    link.evidence.evidence_id
                )));
            }
            let recomputed = link.compute_hash();
            if recomputed != link.chain_hash {
                return Err(ReportingError::EvidenceInvalid(format!(
                    "chain_hash của '{}' không khớp nội dung",
                    link.evidence.evidence_id
                )));
            }
            expected_prev = link.chain_hash.as_str();
        }
        Ok(())
    }
}

// ============================================================================
// 4. CVSS v3.1 BASE SCORE & BUSINESS PRIORITY
// ============================================================================

#[derive(Debug, Clone, Copy, PartialEq, Eq, PartialOrd, Ord)]
pub enum AttackVector {
    Network,
    Adjacent,
    Local,
    Physical,
}

impl AttackVector {
    pub fn code(self) -> &'static str {
        match self {
            Self::Network => "N",
            Self::Adjacent => "A",
            Self::Local => "L",
            Self::Physical => "P",
        }
    }

    pub fn weight(self) -> f64 {
        match self {
            Self::Network => 0.85,
            Self::Adjacent => 0.62,
            Self::Local => 0.55,
            Self::Physical => 0.20,
        }
    }
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, PartialOrd, Ord)]
pub enum AttackComplexity {
    Low,
    High,
}

impl AttackComplexity {
    pub fn code(self) -> &'static str {
        match self {
            Self::Low => "L",
            Self::High => "H",
        }
    }

    pub fn weight(self) -> f64 {
        match self {
            Self::Low => 0.77,
            Self::High => 0.44,
        }
    }
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, PartialOrd, Ord)]
pub enum PrivilegesRequired {
    None,
    Low,
    High,
}

impl PrivilegesRequired {
    pub fn code(self) -> &'static str {
        match self {
            Self::None => "N",
            Self::Low => "L",
            Self::High => "H",
        }
    }

    pub fn weight(self, scope: CvssScope) -> f64 {
        match (self, scope) {
            (Self::None, _) => 0.85,
            (Self::Low, CvssScope::Unchanged) => 0.62,
            (Self::Low, CvssScope::Changed) => 0.68,
            (Self::High, CvssScope::Unchanged) => 0.27,
            (Self::High, CvssScope::Changed) => 0.50,
        }
    }
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, PartialOrd, Ord)]
pub enum UserInteraction {
    None,
    Required,
}

impl UserInteraction {
    pub fn code(self) -> &'static str {
        match self {
            Self::None => "N",
            Self::Required => "R",
        }
    }

    pub fn weight(self) -> f64 {
        match self {
            Self::None => 0.85,
            Self::Required => 0.62,
        }
    }
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, PartialOrd, Ord)]
pub enum CvssScope {
    Unchanged,
    Changed,
}

impl CvssScope {
    pub fn code(self) -> &'static str {
        match self {
            Self::Unchanged => "U",
            Self::Changed => "C",
        }
    }
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, PartialOrd, Ord)]
pub enum ImpactMetric {
    None,
    Low,
    High,
}

impl ImpactMetric {
    pub fn code(self) -> &'static str {
        match self {
            Self::None => "N",
            Self::Low => "L",
            Self::High => "H",
        }
    }

    pub fn weight(self) -> f64 {
        match self {
            Self::None => 0.0,
            Self::Low => 0.22,
            Self::High => 0.56,
        }
    }
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, PartialOrd, Ord)]
pub enum CvssSeverity {
    None,
    Low,
    Medium,
    High,
    Critical,
}

impl CvssSeverity {
    pub fn from_tenths(tenths: u8) -> Self {
        match tenths {
            0 => Self::None,
            1..=39 => Self::Low,
            40..=69 => Self::Medium,
            70..=89 => Self::High,
            _ => Self::Critical,
        }
    }

    pub fn as_str(self) -> &'static str {
        match self {
            Self::None => "None",
            Self::Low => "Low",
            Self::Medium => "Medium",
            Self::High => "High",
            Self::Critical => "Critical",
        }
    }
}

/// Hàm Roundup chuẩn FIRST.org CVSS v3.1 Appendix A (làm tròn theo số nguyên `* 100_000`
/// để tránh sai số dấu phẩy động kiểu `0.1 + 0.2 = 0.30000000000000004` làm nhảy bậc).
pub fn roundup_cvss31_tenths(input: f64) -> u8 {
    if input <= 0.0 {
        return 0;
    }
    let int_input = (input * 100_000.0).round() as u64;
    let tenths = if int_input.is_multiple_of(10_000) {
        int_input / 10_000
    } else {
        (int_input / 10_000) + 1
    };
    tenths.min(100) as u8
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, PartialOrd, Ord)]
pub struct Cvss31Vector {
    pub av: AttackVector,
    pub ac: AttackComplexity,
    pub pr: PrivilegesRequired,
    pub ui: UserInteraction,
    pub s: CvssScope,
    pub c: ImpactMetric,
    pub i: ImpactMetric,
    pub a: ImpactMetric,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, PartialOrd, Ord)]
pub struct Cvss31Score {
    pub vector: Cvss31Vector,
    /// Điểm Base nhân 10 (`98` tương ứng `9.8`) để so sánh chính xác, không lỗi float.
    pub score_tenths: u8,
    pub impact_tenths: u8,
    pub exploitability_tenths: u8,
    pub severity: CvssSeverity,
}

impl Cvss31Score {
    pub fn base_score(self) -> f64 {
        f64::from(self.score_tenths) / 10.0
    }

    pub fn score_string(self) -> String {
        format!("{}.{}", self.score_tenths / 10, self.score_tenths % 10)
    }
}

impl Cvss31Vector {
    /// Phân tích chuỗi vector CVSS v3.1 (chấp nhận cả `CVSS:3.1/...` lẫn chuỗi viết gọn `AV:N/...`),
    /// từ chối phiên bản khác (như `CVSS:3.0/...`), sai thứ tự hoặc thiếu/trùng metric.
    pub fn parse(raw: &str) -> Result<Self, ReportingError> {
        let trimmed = raw.trim();
        let body = if let Some(after) = trimmed.strip_prefix("CVSS:") {
            after.strip_prefix("3.1/").ok_or_else(|| {
                ReportingError::InvalidCvssVector(format!("yêu cầu CVSS:3.1, nhận '{raw}'"))
            })?
        } else {
            trimmed
        };
        let parts: Vec<&str> = body.split('/').collect();
        if parts.len() != 8 {
            return Err(ReportingError::InvalidCvssVector(format!(
                "vector CVSS 3.1 phải có đúng 8 metric cơ sở, nhận '{}'",
                parts.len()
            )));
        }
        let expected_prefixes = ["AV:", "AC:", "PR:", "UI:", "S:", "C:", "I:", "A:"];
        for (idx, part) in parts.iter().enumerate() {
            if !part.starts_with(expected_prefixes[idx]) {
                return Err(ReportingError::InvalidCvssVector(format!(
                    "metric ở vị trí {idx} phải bắt đầu bằng '{}', nhận '{part}'",
                    expected_prefixes[idx]
                )));
            }
        }
        let av = match &parts[0][3..] {
            "N" => AttackVector::Network,
            "A" => AttackVector::Adjacent,
            "L" => AttackVector::Local,
            "P" => AttackVector::Physical,
            other => {
                return Err(ReportingError::InvalidCvssVector(format!(
                    "AV không hợp lệ: '{other}'"
                )));
            }
        };
        let ac = match &parts[1][3..] {
            "L" => AttackComplexity::Low,
            "H" => AttackComplexity::High,
            other => {
                return Err(ReportingError::InvalidCvssVector(format!(
                    "AC không hợp lệ: '{other}'"
                )));
            }
        };
        let pr = match &parts[2][3..] {
            "N" => PrivilegesRequired::None,
            "L" => PrivilegesRequired::Low,
            "H" => PrivilegesRequired::High,
            other => {
                return Err(ReportingError::InvalidCvssVector(format!(
                    "PR không hợp lệ: '{other}'"
                )));
            }
        };
        let ui = match &parts[3][3..] {
            "N" => UserInteraction::None,
            "R" => UserInteraction::Required,
            other => {
                return Err(ReportingError::InvalidCvssVector(format!(
                    "UI không hợp lệ: '{other}'"
                )));
            }
        };
        let s = match &parts[4][2..] {
            "U" => CvssScope::Unchanged,
            "C" => CvssScope::Changed,
            other => {
                return Err(ReportingError::InvalidCvssVector(format!(
                    "S không hợp lệ: '{other}'"
                )));
            }
        };
        let parse_impact = |name: &str, token: &str| -> Result<ImpactMetric, ReportingError> {
            match token {
                "N" => Ok(ImpactMetric::None),
                "L" => Ok(ImpactMetric::Low),
                "H" => Ok(ImpactMetric::High),
                other => Err(ReportingError::InvalidCvssVector(format!(
                    "{name} không hợp lệ: '{other}'"
                ))),
            }
        };
        let c = parse_impact("C", &parts[5][2..])?;
        let i = parse_impact("I", &parts[6][2..])?;
        let a = parse_impact("A", &parts[7][2..])?;
        Ok(Self {
            av,
            ac,
            pr,
            ui,
            s,
            c,
            i,
            a,
        })
    }

    pub fn canonical_string(self) -> String {
        format!(
            "CVSS:3.1/AV:{}/AC:{}/PR:{}/UI:{}/S:{}/C:{}/I:{}/A:{}",
            self.av.code(),
            self.ac.code(),
            self.pr.code(),
            self.ui.code(),
            self.s.code(),
            self.c.code(),
            self.i.code(),
            self.a.code()
        )
    }

    /// Tính CVSS v3.1 Base Score theo đúng phương trình mục 7.1 của FIRST.org.
    pub fn score(self) -> Cvss31Score {
        let iss =
            1.0 - ((1.0 - self.c.weight()) * (1.0 - self.i.weight()) * (1.0 - self.a.weight()));
        let impact = match self.s {
            CvssScope::Unchanged => 6.42 * iss,
            CvssScope::Changed => 7.52 * (iss - 0.029) - 3.25 * (iss - 0.02_f64).powi(15),
        };
        let exploitability =
            8.22 * self.av.weight() * self.ac.weight() * self.pr.weight(self.s) * self.ui.weight();

        let impact_tenths = ((impact.max(0.0) * 10.0).round() as u64).min(100) as u8;
        let exploitability_tenths = ((exploitability * 10.0).round() as u64).min(100) as u8;

        let score_tenths = if impact <= 0.0 {
            0
        } else {
            let raw = match self.s {
                CvssScope::Unchanged => (impact + exploitability).min(10.0),
                CvssScope::Changed => (1.08 * (impact + exploitability)).min(10.0),
            };
            roundup_cvss31_tenths(raw)
        };
        let severity = CvssSeverity::from_tenths(score_tenths);
        Cvss31Score {
            vector: self,
            score_tenths,
            impact_tenths,
            exploitability_tenths,
            severity,
        }
    }
}

impl FromStr for Cvss31Vector {
    type Err = ReportingError;

    fn from_str(s: &str) -> Result<Self, Self::Err> {
        Self::parse(s)
    }
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, PartialOrd, Ord)]
pub enum AssetCriticality {
    DevSandbox,
    InternalTool,
    CoreIdentityOrRevenue,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, PartialOrd, Ord)]
pub enum ExposureContext {
    IsolatedLabOnly,
    AuthenticatedInternal,
    InternetFacing,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, PartialOrd, Ord)]
pub enum BusinessPriority {
    P4Backlog,
    P3Scheduled,
    P2Accelerated,
    P1Immediate,
}

impl BusinessPriority {
    pub fn as_str(self) -> &'static str {
        match self {
            Self::P4Backlog => "P4-Backlog",
            Self::P3Scheduled => "P3-Scheduled",
            Self::P2Accelerated => "P2-Accelerated",
            Self::P1Immediate => "P1-Immediate",
        }
    }
}

/// Tách `CvssSeverity` kỹ thuật khỏi `BusinessPriority` kinh doanh:
/// ví dụ lỗ hổng `Critical` nhưng chỉ ở `DevSandbox + IsolatedLabOnly` sẽ thành `P3Scheduled`,
/// trong khi `Medium` ở `CoreIdentityOrRevenue + InternetFacing` nâng lên `P1Immediate`.
pub fn compute_business_priority(
    severity: CvssSeverity,
    criticality: AssetCriticality,
    exposure: ExposureContext,
) -> BusinessPriority {
    if severity == CvssSeverity::None {
        return BusinessPriority::P4Backlog;
    }
    let sev_pts: i32 = match severity {
        CvssSeverity::None => 0,
        CvssSeverity::Low => 1,
        CvssSeverity::Medium => 2,
        CvssSeverity::High => 3,
        CvssSeverity::Critical => 4,
    };
    let crit_adj: i32 = match criticality {
        AssetCriticality::DevSandbox => -2,
        AssetCriticality::InternalTool => 0,
        AssetCriticality::CoreIdentityOrRevenue => 1,
    };
    let exp_adj: i32 = match exposure {
        ExposureContext::IsolatedLabOnly => -1,
        ExposureContext::AuthenticatedInternal => 0,
        ExposureContext::InternetFacing => 1,
    };
    let rank = sev_pts + crit_adj + exp_adj;
    if rank >= 4 {
        BusinessPriority::P1Immediate
    } else if rank >= 3 {
        BusinessPriority::P2Accelerated
    } else if rank >= 1 {
        BusinessPriority::P3Scheduled
    } else {
        BusinessPriority::P4Backlog
    }
}

// ============================================================================
// 5. VÒNG ĐỜI FINDING, KHỬ TRÙNG LẶP & RETEST
// ============================================================================

#[derive(Debug, Clone, Copy, PartialEq, Eq, PartialOrd, Ord)]
pub enum FindingStatus {
    Draft,
    Verified,
    Duplicate,
    FalsePositive,
    AcceptedRisk,
    Remediated,
    Retested,
}

impl FindingStatus {
    pub fn as_str(self) -> &'static str {
        match self {
            Self::Draft => "Draft",
            Self::Verified => "Verified",
            Self::Duplicate => "Duplicate",
            Self::FalsePositive => "FalsePositive",
            Self::AcceptedRisk => "AcceptedRisk",
            Self::Remediated => "Remediated",
            Self::Retested => "Retested",
        }
    }

    pub fn can_transition_to(self, next: Self) -> bool {
        if self == next {
            return true;
        }
        matches!(
            (self, next),
            (Self::Draft, Self::Verified)
                | (Self::Draft, Self::Duplicate)
                | (Self::Draft, Self::FalsePositive)
                | (Self::Verified, Self::Duplicate)
                | (Self::Verified, Self::AcceptedRisk)
                | (Self::Verified, Self::Remediated)
                | (Self::Verified, Self::Retested)
                | (Self::Remediated, Self::Retested)
                | (Self::AcceptedRisk, Self::Remediated)
        )
    }

    pub fn is_verified_family(self) -> bool {
        matches!(
            self,
            Self::Verified | Self::AcceptedRisk | Self::Remediated | Self::Retested
        )
    }

    pub fn is_open_risk(self, retest: Option<&RetestRecord>) -> bool {
        match self {
            Self::Verified => true,
            Self::Retested => {
                !retest.is_some_and(|r| matches!(r.outcome, RetestOutcome::FixedVerified))
            }
            _ => false,
        }
    }

    fn verification_rank(self) -> u8 {
        match self {
            Self::Retested => 6,
            Self::Remediated => 5,
            Self::AcceptedRisk => 4,
            Self::Verified => 3,
            Self::Duplicate => 2,
            Self::Draft => 1,
            Self::FalsePositive => 0,
        }
    }
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub enum ExploitationClaim {
    NotExploited,
    ValidatedPreconditionOnly,
    ControlledLabExploited { lab_proof_evidence_id: String },
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub enum RetestOutcome {
    FixedVerified,
    PartiallyMitigated { residual_vector: Cvss31Vector },
    StillVulnerable,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct RetestRecord {
    pub retested_at_utc: String,
    pub retest_evidence_id: String,
    pub outcome: RetestOutcome,
    pub verifier_note: String,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct StandardCitation {
    pub citation_id: String,
    pub title: String,
    pub reference: String,
}

impl StandardCitation {
    pub fn validate(&self) -> Result<(), ReportingError> {
        if self.citation_id.trim().is_empty()
            || self.title.trim().is_empty()
            || self.reference.trim().is_empty()
        {
            return Err(ReportingError::ReportSchemaInvalid(
                "StandardCitation có trường rỗng".to_string(),
            ));
        }
        validate_no_raw_secret("citation.id", &self.citation_id)?;
        validate_no_raw_secret("citation.title", &self.title)?;
        validate_no_raw_secret("citation.reference", &self.reference)?;
        Ok(())
    }
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct Finding {
    pub finding_id: String,
    pub rule_id: String,
    pub root_cause_key: String,
    pub primary_component: String,
    pub title: String,
    pub affected_assets: BTreeSet<String>,
    pub status: FindingStatus,
    pub duplicate_of: Option<String>,
    pub merged_duplicate_ids: BTreeSet<String>,
    pub exploitation_claim: ExploitationClaim,
    pub cvss_vector: Cvss31Vector,
    pub asset_criticality: AssetCriticality,
    pub exposure_context: ExposureContext,
    pub business_priority: BusinessPriority,
    pub evidence_refs: Vec<EvidenceRef>,
    pub lab_reproduction: String,
    pub impact_analysis: String,
    pub remediation: String,
    pub cleanup_steps: String,
    pub detection_control: String,
    pub retest: Option<RetestRecord>,
    pub citation_ids: BTreeSet<String>,
}

impl Finding {
    pub fn cvss_score(&self) -> Cvss31Score {
        self.cvss_vector.score()
    }

    pub fn fingerprint(&self) -> (String, String, String) {
        (
            self.rule_id.trim().to_ascii_lowercase(),
            self.root_cause_key.trim().to_ascii_lowercase(),
            self.primary_component.trim().to_ascii_lowercase(),
        )
    }

    pub fn transition_status(&mut self, next: FindingStatus) -> Result<(), ReportingError> {
        if !self.status.can_transition_to(next) {
            return Err(ReportingError::IllegalFindingTransition {
                from: self.status,
                to: next,
            });
        }
        if !next.is_verified_family()
            && matches!(
                self.exploitation_claim,
                ExploitationClaim::ControlledLabExploited { .. }
            )
        {
            return Err(ReportingError::UnverifiedExploitationClaim(format!(
                "không thể chuyển '{}' sang {:?} khi vẫn giữ claim ControlledLabExploited",
                self.finding_id, next
            )));
        }
        self.status = next;
        Ok(())
    }

    pub fn set_exploitation_claim(
        &mut self,
        claim: ExploitationClaim,
    ) -> Result<(), ReportingError> {
        if let ExploitationClaim::ControlledLabExploited {
            lab_proof_evidence_id,
        } = &claim
        {
            if !self.status.is_verified_family() {
                return Err(ReportingError::UnverifiedExploitationClaim(format!(
                    "finding '{}' đang ở trạng thái {:?}, cấm gắn ControlledLabExploited",
                    self.finding_id, self.status
                )));
            }
            if !self
                .evidence_refs
                .iter()
                .any(|ev| ev.evidence_id == *lab_proof_evidence_id)
            {
                return Err(ReportingError::UnverifiedExploitationClaim(format!(
                    "lab_proof_evidence_id '{lab_proof_evidence_id}' không nằm trong evidence_refs của '{}'",
                    self.finding_id
                )));
            }
        }
        self.exploitation_claim = claim;
        Ok(())
    }

    fn validate_shape(&self) -> Result<BTreeSet<&str>, ReportingError> {
        if self.finding_id.trim().is_empty()
            || self.rule_id.trim().is_empty()
            || self.root_cause_key.trim().is_empty()
            || self.primary_component.trim().is_empty()
            || self.title.trim().is_empty()
        {
            return Err(ReportingError::EvidenceInvalid(format!(
                "finding '{}' thiếu metadata định danh",
                self.finding_id
            )));
        }
        if self.affected_assets.is_empty()
            || self.lab_reproduction.trim().is_empty()
            || self.impact_analysis.trim().is_empty()
            || self.remediation.trim().is_empty()
            || self.cleanup_steps.trim().is_empty()
            || self.detection_control.trim().is_empty()
        {
            return Err(ReportingError::EvidenceInvalid(format!(
                "finding '{}' thiếu affected_assets, reproduction, impact, remediation, cleanup hoặc detection",
                self.finding_id
            )));
        }
        for (label, text) in [
            ("finding.id", self.finding_id.as_str()),
            ("finding.rule_id", self.rule_id.as_str()),
            ("finding.root_cause_key", self.root_cause_key.as_str()),
            ("finding.primary_component", self.primary_component.as_str()),
            ("finding.title", self.title.as_str()),
            ("finding.reproduction", self.lab_reproduction.as_str()),
            ("finding.impact", self.impact_analysis.as_str()),
            ("finding.remediation", self.remediation.as_str()),
            ("finding.cleanup", self.cleanup_steps.as_str()),
            ("finding.detection", self.detection_control.as_str()),
        ] {
            validate_no_raw_secret(label, text)?;
        }
        if let Some(dup_target) = &self.duplicate_of {
            validate_no_raw_secret("finding.duplicate_of", dup_target)?;
        }
        for dup_id in &self.merged_duplicate_ids {
            validate_no_raw_secret("finding.merged_duplicate_id", dup_id)?;
        }
        for cite in &self.citation_ids {
            validate_no_raw_secret("finding.citation_id", cite)?;
        }
        for asset in &self.affected_assets {
            validate_no_raw_secret("finding.asset", asset)?;
        }
        if self.evidence_refs.is_empty() {
            return Err(ReportingError::EvidenceInvalid(format!(
                "finding '{}' phải có ít nhất một EvidenceRef",
                self.finding_id
            )));
        }
        let mut seen_ev = BTreeSet::new();
        for ev in &self.evidence_refs {
            ev.validate()?;
            if !seen_ev.insert(ev.evidence_id.as_str()) {
                return Err(ReportingError::DuplicateIdentifier(format!(
                    "evidence_id '{}' bị lặp trong finding '{}'",
                    ev.evidence_id, self.finding_id
                )));
            }
        }
        if let ExploitationClaim::ControlledLabExploited {
            lab_proof_evidence_id,
        } = &self.exploitation_claim
        {
            validate_no_raw_secret("finding.lab_proof_evidence_id", lab_proof_evidence_id)?;
            if !self.status.is_verified_family() {
                return Err(ReportingError::UnverifiedExploitationClaim(format!(
                    "finding '{}' có status {:?} không được phép ghi nhận ControlledLabExploited",
                    self.finding_id, self.status
                )));
            }
            if !seen_ev.contains(lab_proof_evidence_id.as_str()) {
                return Err(ReportingError::UnverifiedExploitationClaim(format!(
                    "finding '{}' thiếu bằng chứng chứng minh '{lab_proof_evidence_id}'",
                    self.finding_id
                )));
            }
        }
        if self.status == FindingStatus::Duplicate && self.duplicate_of.is_none() {
            return Err(ReportingError::DeduplicationConflict(format!(
                "finding '{}' mang status Duplicate nhưng thiếu duplicate_of",
                self.finding_id
            )));
        }
        if self.status != FindingStatus::Duplicate && self.duplicate_of.is_some() {
            return Err(ReportingError::DeduplicationConflict(format!(
                "finding '{}' mang status {:?} nhưng lại gán duplicate_of",
                self.finding_id, self.status
            )));
        }
        match (&self.status, &self.retest) {
            (FindingStatus::Retested, None) => {
                return Err(ReportingError::EvidenceInvalid(format!(
                    "finding '{}' mang status Retested nhưng thiếu RetestRecord",
                    self.finding_id
                )));
            }
            (other, Some(_)) if *other != FindingStatus::Retested => {
                return Err(ReportingError::IllegalFindingTransition {
                    from: *other,
                    to: FindingStatus::Retested,
                });
            }
            (_, Some(retest)) => {
                if !is_valid_iso8601_utc(&retest.retested_at_utc) {
                    return Err(ReportingError::InvalidTimestamp(
                        retest.retested_at_utc.clone(),
                    ));
                }
                validate_no_raw_secret("retest.evidence_id", &retest.retest_evidence_id)?;
                validate_no_raw_secret("retest.note", &retest.verifier_note)?;
                if retest.verifier_note.trim().is_empty() {
                    return Err(ReportingError::EvidenceInvalid(format!(
                        "retest verifier_note của '{}' rỗng",
                        self.finding_id
                    )));
                }
                if !seen_ev.contains(retest.retest_evidence_id.as_str()) {
                    return Err(ReportingError::EvidenceInvalid(format!(
                        "retest_evidence_id '{}' chưa gắn vào finding '{}'",
                        retest.retest_evidence_id, self.finding_id
                    )));
                }
            }
            (_, None) => {}
        }
        let expected_priority = compute_business_priority(
            self.cvss_score().severity,
            self.asset_criticality,
            self.exposure_context,
        );
        if self.business_priority != expected_priority {
            return Err(ReportingError::ReportSchemaInvalid(format!(
                "business_priority của '{}' không khớp (kỳ vọng {:?}, nhận {:?})",
                self.finding_id, expected_priority, self.business_priority
            )));
        }
        Ok(seen_ev)
    }

    pub fn validate(&self, chain: &EvidenceChain) -> Result<(), ReportingError> {
        let _ = self.validate_shape()?;
        for ev in &self.evidence_refs {
            let stored = chain.get(&ev.evidence_id).ok_or_else(|| {
                ReportingError::EvidenceInvalid(format!(
                    "evidence_id '{}' của finding '{}' không có trong EvidenceChain",
                    ev.evidence_id, self.finding_id
                ))
            })?;
            if stored != ev {
                return Err(ReportingError::EvidenceInvalid(format!(
                    "evidence_id '{}' trong finding '{}' lệch với bản trong EvidenceChain",
                    ev.evidence_id, self.finding_id
                )));
            }
        }
        Ok(())
    }
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct DeduplicationResult {
    pub canonical_findings: Vec<Finding>,
    pub excluded_non_actionable: Vec<Finding>,
}

fn merge_evidence_vecs(
    target: &mut Vec<EvidenceRef>,
    incoming: &[EvidenceRef],
) -> Result<(), ReportingError> {
    let mut by_id: BTreeMap<String, EvidenceRef> = BTreeMap::new();
    for ev in target.drain(..) {
        by_id.insert(ev.evidence_id.clone(), ev);
    }
    for ev in incoming {
        if let Some(existing) = by_id.get(&ev.evidence_id) {
            if existing != ev {
                return Err(ReportingError::DeduplicationConflict(format!(
                    "xung đột digest/nội dung cùng evidence_id '{}'",
                    ev.evidence_id
                )));
            }
        } else {
            by_id.insert(ev.evidence_id.clone(), ev.clone());
        }
    }
    *target = by_id.into_values().collect();
    Ok(())
}

fn merge_duplicate_into(canonical: &mut Finding, other: Finding) -> Result<(), ReportingError> {
    // Nếu bản chính đã trải qua Retest (đặc biệt là PartiallyMitigated với residual_vector),
    // không cho bản trùng chưa retest ghi đè ngược điểm CVSS cũ lên bản chính.
    let preserve_retested_score =
        canonical.status == FindingStatus::Retested && other.retest.is_none();
    if !preserve_retested_score
        && other.cvss_score().score_tenths > canonical.cvss_score().score_tenths
    {
        canonical.cvss_vector = other.cvss_vector;
    }
    canonical.asset_criticality = canonical.asset_criticality.max(other.asset_criticality);
    canonical.exposure_context = canonical.exposure_context.max(other.exposure_context);
    canonical.business_priority = compute_business_priority(
        canonical.cvss_score().severity,
        canonical.asset_criticality,
        canonical.exposure_context,
    );
    merge_evidence_vecs(&mut canonical.evidence_refs, &other.evidence_refs)?;
    if canonical.retest.is_none()
        && canonical.status == FindingStatus::Retested
        && other.retest.is_some()
    {
        canonical.retest = other.retest;
    }
    if !matches!(
        canonical.exploitation_claim,
        ExploitationClaim::ControlledLabExploited { .. }
    ) && matches!(
        other.exploitation_claim,
        ExploitationClaim::ControlledLabExploited { .. }
    ) {
        canonical.exploitation_claim = other.exploitation_claim;
    }
    canonical.merged_duplicate_ids.insert(other.finding_id);
    canonical
        .merged_duplicate_ids
        .extend(other.merged_duplicate_ids);
    canonical.affected_assets.extend(other.affected_assets);
    canonical.citation_ids.extend(other.citation_ids);
    Ok(())
}

/// Khử trùng lặp hoàn toàn độc lập hoán vị đầu vào:
/// - Chọn bản ghi đại diện theo `(verification_rank_desc, cvss_desc, finding_id_asc)`;
/// - Tách riêng bucket theo trạng thái còn rủi ro mở (`is_open_risk`) để không bao giờ
///   đóng nhầm một phát hiện `Verified` đang mở dưới một bản `Retested(FixedVerified)` đã vá;
/// - Gom toàn bộ `evidence_refs`, `affected_assets`, `citation_ids` và `merged_duplicate_ids`;
/// - Chặn `Duplicate` trỏ thiếu, tự trỏ chính mình hoặc trỏ vào `FalsePositive`/`Draft` chưa xác minh;
/// - Tách riêng `Draft` và `FalsePositive` vào `excluded_non_actionable`.
pub fn deduplicate_findings(
    raw_findings: &[Finding],
    chain: &EvidenceChain,
) -> Result<DeduplicationResult, ReportingError> {
    let mut by_id: BTreeMap<String, Finding> = BTreeMap::new();
    for f in raw_findings {
        f.validate(chain)?;
        if by_id.contains_key(&f.finding_id) {
            return Err(ReportingError::DuplicateIdentifier(format!(
                "finding_id '{}' bị trùng khi deduplicate",
                f.finding_id
            )));
        }
        by_id.insert(f.finding_id.clone(), f.clone());
    }

    // Kiểm tra liên kết duplicate_of tường minh trước khi gộp nhóm.
    let mut explicit_links: BTreeMap<String, String> = BTreeMap::new();
    for (id, f) in &by_id {
        if let Some(target) = &f.duplicate_of {
            if target == id {
                return Err(ReportingError::DeduplicationConflict(format!(
                    "finding '{id}' tự trỏ duplicate_of vào chính nó"
                )));
            }
            let target_f = by_id.get(target).ok_or_else(|| {
                ReportingError::DeduplicationConflict(format!(
                    "finding '{id}' trỏ duplicate_of tới '{target}' không tồn tại"
                ))
            })?;
            if target_f.duplicate_of.is_some() || target_f.status == FindingStatus::Duplicate {
                return Err(ReportingError::DeduplicationConflict(format!(
                    "finding '{id}' trỏ duplicate_of dây chuyền tới '{target}'"
                )));
            }
            if !target_f.status.is_verified_family() {
                return Err(ReportingError::DeduplicationConflict(format!(
                    "finding '{id}' trỏ duplicate_of tới '{target}' chưa xác minh ({:?})",
                    target_f.status
                )));
            }
            explicit_links.insert(id.clone(), target.clone());
        }
    }

    let mut excluded_non_actionable = Vec::new();
    let mut active_verified: Vec<Finding> = Vec::new();
    for (_, f) in by_id {
        if matches!(
            f.status,
            FindingStatus::Draft | FindingStatus::FalsePositive
        ) {
            excluded_non_actionable.push(f);
        } else {
            active_verified.push(f);
        }
    }

    // Gán nhóm bằng chữ ký kỹ thuật + trạng thái mở/đóng cho nhóm đã xác minh + gắn Duplicate tường minh.
    let mut explicit_by_target: BTreeMap<String, Vec<Finding>> = BTreeMap::new();
    let mut non_duplicate_verified: BTreeMap<String, Finding> = BTreeMap::new();
    for f in active_verified {
        if let Some(target) = explicit_links.get(&f.finding_id) {
            explicit_by_target
                .entry(target.clone())
                .or_default()
                .push(f);
        } else {
            non_duplicate_verified.insert(f.finding_id.clone(), f);
        }
    }

    type DedupGroupKey = ((String, String, String), bool);
    let mut fingerprint_groups: BTreeMap<DedupGroupKey, Vec<Finding>> = BTreeMap::new();
    for mut f in non_duplicate_verified.into_values() {
        if let Some(mut extras) = explicit_by_target.remove(&f.finding_id) {
            extras.sort_by(|a, b| a.finding_id.cmp(&b.finding_id));
            for extra in extras {
                merge_duplicate_into(&mut f, extra)?;
            }
        }
        let open_bucket = f.status.is_open_risk(f.retest.as_ref());
        fingerprint_groups
            .entry((f.fingerprint(), open_bucket))
            .or_default()
            .push(f);
    }

    let mut canonical_findings = Vec::with_capacity(fingerprint_groups.len());
    for (_, mut members) in fingerprint_groups {
        members.sort_by(|a, b| {
            b.status
                .verification_rank()
                .cmp(&a.status.verification_rank())
                .then_with(|| {
                    b.cvss_score()
                        .score_tenths
                        .cmp(&a.cvss_score().score_tenths)
                })
                .then_with(|| a.finding_id.cmp(&b.finding_id))
        });
        let mut canonical = members.remove(0);
        for other in members {
            merge_duplicate_into(&mut canonical, other)?;
        }
        canonical_findings.push(canonical);
    }

    sort_canonical_findings(&mut canonical_findings);
    excluded_non_actionable.sort_by(|a, b| a.finding_id.cmp(&b.finding_id));
    Ok(DeduplicationResult {
        canonical_findings,
        excluded_non_actionable,
    })
}

fn sort_canonical_findings(findings: &mut [Finding]) {
    findings.sort_by(|a, b| {
        let a_open = a.status.is_open_risk(a.retest.as_ref());
        let b_open = b.status.is_open_risk(b.retest.as_ref());
        b_open
            .cmp(&a_open)
            .then_with(|| b.business_priority.cmp(&a.business_priority))
            .then_with(|| {
                b.cvss_score()
                    .score_tenths
                    .cmp(&a.cvss_score().score_tenths)
            })
            .then_with(|| a.finding_id.cmp(&b.finding_id))
    });
}

// ============================================================================
// 6. BÁO CÁO & TRÌNH KẾT XUẤT TẤT ĐỊNH (`Report`, `ReportRenderer`)
// ============================================================================

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct ExecutiveSummary {
    pub total_verified_findings: usize,
    pub excluded_draft_or_false_positive: usize,
    pub critical_count: usize,
    pub high_count: usize,
    pub medium_count: usize,
    pub low_count: usize,
    pub none_count: usize,
    pub retested_fixed_count: usize,
    pub highest_cvss_tenths: u8,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct Report {
    pub schema_version: String,
    pub engagement_id: String,
    pub title: String,
    pub client_org: String,
    pub authorization_ref: String,
    pub generated_at_utc: String,
    pub final_phase: EngagementPhase,
    pub halted_by_stop_condition: Option<StopConditionTrigger>,
    pub cleanup_attestation: Option<CleanupAttestation>,
    pub retention_days: u16,
    pub deletion_deadline_utc: String,
    pub executive_summary: ExecutiveSummary,
    pub executive_narrative: String,
    pub methodology_overview: String,
    pub scope_summary: Vec<String>,
    pub findings: Vec<Finding>,
    pub excluded_non_actionable_ids: Vec<(String, FindingStatus)>,
    pub evidence_manifest: Vec<EvidenceRef>,
    pub citations: BTreeMap<String, StandardCitation>,
    pub input_digest_sha256: String,
}

impl Report {
    fn compute_digest(&self) -> String {
        let mut buf = String::new();
        push_lp(&mut buf, &self.schema_version);
        push_lp(&mut buf, &self.engagement_id);
        push_lp(&mut buf, &self.title);
        push_lp(&mut buf, &self.client_org);
        push_lp(&mut buf, &self.authorization_ref);
        push_lp(&mut buf, &self.generated_at_utc);
        push_lp(&mut buf, self.final_phase.as_str());
        let halt_str = self
            .halted_by_stop_condition
            .map_or_else(|| "None".to_string(), |s| format!("{s:?}"));
        push_lp(&mut buf, &halt_str);
        push_lp(&mut buf, &self.retention_days.to_string());
        push_lp(&mut buf, &self.deletion_deadline_utc);
        push_lp(&mut buf, &self.executive_narrative);
        push_lp(&mut buf, &self.methodology_overview);

        match &self.cleanup_attestation {
            None => push_lp(&mut buf, "none"),
            Some(c) => {
                push_lp(&mut buf, "some");
                push_lp(&mut buf, &c.plan_id);
                push_lp(&mut buf, &c.verified_clean.to_string());
                push_lp(&mut buf, &c.remaining_artifacts.to_string());
                push_lp(&mut buf, &c.attested_by_role);
                push_lp(&mut buf, &c.attested_at_utc);
            }
        }

        push_lp(&mut buf, &self.scope_summary.len().to_string());
        for line in &self.scope_summary {
            push_lp(&mut buf, line);
        }

        push_lp(&mut buf, &self.findings.len().to_string());
        for f in &self.findings {
            push_lp(&mut buf, &f.finding_id);
            push_lp(&mut buf, &f.rule_id);
            push_lp(&mut buf, &f.root_cause_key);
            push_lp(&mut buf, &f.primary_component);
            push_lp(&mut buf, &f.title);
            push_lp(&mut buf, f.status.as_str());
            push_lp(&mut buf, f.duplicate_of.as_deref().unwrap_or(""));
            push_lp(&mut buf, &f.cvss_vector.canonical_string());
            push_lp(&mut buf, &format!("{:?}", f.asset_criticality));
            push_lp(&mut buf, &format!("{:?}", f.exposure_context));
            push_lp(&mut buf, f.business_priority.as_str());
            push_lp(&mut buf, &format!("{:?}", f.exploitation_claim));
            push_lp(&mut buf, &f.affected_assets.len().to_string());
            for a in &f.affected_assets {
                push_lp(&mut buf, a);
            }
            push_lp(&mut buf, &f.merged_duplicate_ids.len().to_string());
            for m in &f.merged_duplicate_ids {
                push_lp(&mut buf, m);
            }
            push_lp(&mut buf, &f.citation_ids.len().to_string());
            for c in &f.citation_ids {
                push_lp(&mut buf, c);
            }
            push_lp(&mut buf, &f.evidence_refs.len().to_string());
            for ev in &f.evidence_refs {
                push_lp(&mut buf, &ev.evidence_id);
                push_lp(&mut buf, &ev.captured_at_utc);
                push_lp(&mut buf, &ev.source_uri);
                push_lp(&mut buf, &ev.artifact_sha256);
                push_lp(&mut buf, &ev.redacted_excerpt);
                push_lp(&mut buf, &ev.redaction_count.to_string());
            }
            push_lp(&mut buf, &f.lab_reproduction);
            push_lp(&mut buf, &f.impact_analysis);
            push_lp(&mut buf, &f.remediation);
            push_lp(&mut buf, &f.cleanup_steps);
            push_lp(&mut buf, &f.detection_control);
            match &f.retest {
                None => push_lp(&mut buf, "none"),
                Some(rt) => {
                    push_lp(&mut buf, "some");
                    push_lp(&mut buf, &rt.retested_at_utc);
                    push_lp(&mut buf, &rt.retest_evidence_id);
                    push_lp(&mut buf, &format!("{:?}", rt.outcome));
                    push_lp(&mut buf, &rt.verifier_note);
                }
            }
        }

        push_lp(
            &mut buf,
            &self.excluded_non_actionable_ids.len().to_string(),
        );
        for (ex_id, ex_status) in &self.excluded_non_actionable_ids {
            push_lp(&mut buf, ex_id);
            push_lp(&mut buf, ex_status.as_str());
        }

        push_lp(&mut buf, &self.evidence_manifest.len().to_string());
        for ev in &self.evidence_manifest {
            push_lp(&mut buf, &ev.evidence_id);
            push_lp(&mut buf, &ev.captured_at_utc);
            push_lp(&mut buf, &ev.source_uri);
            push_lp(&mut buf, &ev.artifact_sha256);
            push_lp(&mut buf, &ev.redacted_excerpt);
            push_lp(&mut buf, &ev.redaction_count.to_string());
        }

        push_lp(&mut buf, &self.citations.len().to_string());
        for (cid, c) in &self.citations {
            push_lp(&mut buf, cid);
            push_lp(&mut buf, &c.title);
            push_lp(&mut buf, &c.reference);
        }
        sha256_hex(buf.as_bytes())
    }

    pub fn validate(&self) -> Result<(), ReportingError> {
        if self.schema_version != REPORT_SCHEMA_VERSION {
            return Err(ReportingError::ReportSchemaInvalid(format!(
                "schema_version phải là '{REPORT_SCHEMA_VERSION}', nhận '{}'",
                self.schema_version
            )));
        }
        if self.engagement_id.trim().is_empty()
            || self.title.trim().is_empty()
            || self.client_org.trim().is_empty()
            || self.authorization_ref.trim().is_empty()
            || self.executive_narrative.trim().is_empty()
            || self.methodology_overview.trim().is_empty()
            || self.scope_summary.is_empty()
        {
            return Err(ReportingError::ReportSchemaInvalid(
                "báo cáo thiếu metadata bắt buộc".to_string(),
            ));
        }
        if !(1..=365).contains(&self.retention_days) {
            return Err(ReportingError::RetentionViolation(format!(
                "retention_days={} nằm ngoài 1..=365",
                self.retention_days
            )));
        }
        if !is_valid_iso8601_utc(&self.generated_at_utc)
            || !is_valid_iso8601_utc(&self.deletion_deadline_utc)
        {
            return Err(ReportingError::InvalidTimestamp(
                "generated_at_utc hoặc deletion_deadline_utc không hợp lệ".to_string(),
            ));
        }
        for (label, text) in [
            ("report.engagement_id", self.engagement_id.as_str()),
            ("report.title", self.title.as_str()),
            ("report.client_org", self.client_org.as_str()),
            ("report.authorization_ref", self.authorization_ref.as_str()),
            (
                "report.executive_narrative",
                self.executive_narrative.as_str(),
            ),
            (
                "report.methodology_overview",
                self.methodology_overview.as_str(),
            ),
        ] {
            validate_no_raw_secret(label, text)?;
        }
        for line in &self.scope_summary {
            validate_no_raw_secret("report.scope_summary", line)?;
        }
        if self.halted_by_stop_condition.is_none() {
            let cleanup = self.cleanup_attestation.as_ref().ok_or_else(|| {
                ReportingError::CleanupNotVerified(
                    "báo cáo thiếu CleanupAttestation sạch".to_string(),
                )
            })?;
            cleanup.validate()?;
            if !cleanup.verified_clean || cleanup.remaining_artifacts > 0 {
                return Err(ReportingError::CleanupNotVerified(
                    "CleanupAttestation trong Report chưa sạch".to_string(),
                ));
            }
        } else if let Some(cleanup) = &self.cleanup_attestation {
            cleanup.validate()?;
        }

        let mut manifest_by_id = BTreeMap::new();
        for ev in &self.evidence_manifest {
            ev.validate()?;
            if manifest_by_id
                .insert(ev.evidence_id.clone(), ev.clone())
                .is_some()
            {
                return Err(ReportingError::DuplicateIdentifier(format!(
                    "trùng evidence_id '{}' trong evidence_manifest",
                    ev.evidence_id
                )));
            }
        }
        for (cid, citation) in &self.citations {
            if cid != &citation.citation_id {
                return Err(ReportingError::ReportSchemaInvalid(
                    "citation_id không khớp khóa BTreeMap".to_string(),
                ));
            }
            citation.validate()?;
        }

        let mut expected_sorted = self.findings.clone();
        sort_canonical_findings(&mut expected_sorted);
        if expected_sorted != self.findings {
            return Err(ReportingError::ReportSchemaInvalid(
                "thứ tự findings trong Report không tuân thủ chuẩn tất định".to_string(),
            ));
        }

        let mut critical_count = 0usize;
        let mut high_count = 0usize;
        let mut medium_count = 0usize;
        let mut low_count = 0usize;
        let mut none_count = 0usize;
        let mut retested_fixed_count = 0usize;
        let mut highest_cvss_tenths = 0u8;
        let mut seen_finding_ids = BTreeSet::new();

        for f in &self.findings {
            if !seen_finding_ids.insert(f.finding_id.as_str()) {
                return Err(ReportingError::DuplicateIdentifier(format!(
                    "finding_id '{}' bị lặp trong Report",
                    f.finding_id
                )));
            }
            if matches!(
                f.status,
                FindingStatus::Draft | FindingStatus::FalsePositive | FindingStatus::Duplicate
            ) {
                return Err(ReportingError::UnverifiedExploitationClaim(format!(
                    "finding '{}' mang trạng thái {:?} không được đưa vào danh sách verified findings",
                    f.finding_id, f.status
                )));
            }
            let seen_ev = f.validate_shape()?;
            for ev in &f.evidence_refs {
                let man_ev = manifest_by_id.get(&ev.evidence_id).ok_or_else(|| {
                    ReportingError::EvidenceInvalid(format!(
                        "evidence '{}' của '{}' thiếu trong manifest",
                        ev.evidence_id, f.finding_id
                    ))
                })?;
                if man_ev != ev {
                    return Err(ReportingError::EvidenceInvalid(format!(
                        "evidence '{}' lệch với manifest",
                        ev.evidence_id
                    )));
                }
            }
            if let Some(rt) = &f.retest
                && (!seen_ev.contains(rt.retest_evidence_id.as_str())
                    || !manifest_by_id.contains_key(&rt.retest_evidence_id))
            {
                return Err(ReportingError::EvidenceInvalid(format!(
                    "retest_evidence_id '{}' thiếu trong finding hoặc manifest",
                    rt.retest_evidence_id
                )));
            }
            for cid in &f.citation_ids {
                if !self.citations.contains_key(cid) {
                    return Err(ReportingError::ReportSchemaInvalid(format!(
                        "citation '{cid}' không có trong report.citations"
                    )));
                }
            }
            let sc = f.cvss_score();
            highest_cvss_tenths = highest_cvss_tenths.max(sc.score_tenths);
            match sc.severity {
                CvssSeverity::Critical => critical_count += 1,
                CvssSeverity::High => high_count += 1,
                CvssSeverity::Medium => medium_count += 1,
                CvssSeverity::Low => low_count += 1,
                CvssSeverity::None => none_count += 1,
            }
            if f.status == FindingStatus::Retested
                && f.retest
                    .as_ref()
                    .is_some_and(|r| matches!(r.outcome, RetestOutcome::FixedVerified))
            {
                retested_fixed_count += 1;
            }
        }

        let mut seen_excluded_ids = BTreeSet::new();
        for (ex_id, ex_status) in &self.excluded_non_actionable_ids {
            validate_no_raw_secret("report.excluded_id", ex_id)?;
            if !seen_excluded_ids.insert(ex_id.as_str()) {
                return Err(ReportingError::DuplicateIdentifier(format!(
                    "finding_id '{ex_id}' bị lặp trong excluded_non_actionable_ids"
                )));
            }
            if seen_finding_ids.contains(ex_id.as_str()) {
                return Err(ReportingError::DuplicateIdentifier(format!(
                    "finding_id '{ex_id}' vừa nằm trong verified vừa nằm trong excluded"
                )));
            }
            if !matches!(
                ex_status,
                FindingStatus::Draft | FindingStatus::FalsePositive
            ) {
                return Err(ReportingError::ReportSchemaInvalid(format!(
                    "mục excluded '{ex_id}' phải là Draft hoặc FalsePositive"
                )));
            }
        }

        if self.executive_summary.total_verified_findings != self.findings.len()
            || self.executive_summary.excluded_draft_or_false_positive
                != self.excluded_non_actionable_ids.len()
            || self.executive_summary.critical_count != critical_count
            || self.executive_summary.high_count != high_count
            || self.executive_summary.medium_count != medium_count
            || self.executive_summary.low_count != low_count
            || self.executive_summary.none_count != none_count
            || self.executive_summary.retested_fixed_count != retested_fixed_count
            || self.executive_summary.highest_cvss_tenths != highest_cvss_tenths
        {
            return Err(ReportingError::ReportSchemaInvalid(
                "ExecutiveSummary không khớp tập findings".to_string(),
            ));
        }

        let recomputed_digest = self.compute_digest();
        if recomputed_digest != self.input_digest_sha256 {
            return Err(ReportingError::ReportSchemaInvalid(
                "input_digest_sha256 không khớp nội dung Report".to_string(),
            ));
        }
        Ok(())
    }
}

#[derive(Debug, Clone, Default)]
pub struct ReportRenderer;

fn escape_json_string(raw: &str) -> String {
    let mut out = String::with_capacity(raw.len() + 4);
    out.push('"');
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
    out.push('"');
    out
}

fn escape_markdown_inline(raw: &str) -> String {
    let mut out = String::with_capacity(raw.len());
    for ch in raw.chars() {
        match ch {
            '\\' | '`' | '*' | '_' | '[' | ']' | '<' | '>' | '|' | '&' | '#' | '~' | '$' => {
                out.push('\\');
                out.push(ch);
            }
            '\n' | '\r' => out.push(' '),
            c => out.push(c),
        }
    }
    out
}

fn escape_markdown_block(raw: &str) -> String {
    let inline = escape_markdown_inline(raw);
    let trimmed = inline.trim_start();
    let indent_len = inline.len() - trimmed.len();
    let indent = &inline[..indent_len];
    if trimmed.starts_with('-') || trimmed.starts_with('+') {
        return format!("{indent}\\{trimmed}");
    }
    if let Some(sep_pos) = trimmed.find(['.', ')'])
        && sep_pos > 0
        && trimmed[..sep_pos].bytes().all(|b| b.is_ascii_digit())
    {
        let sep = trimmed.as_bytes()[sep_pos] as char;
        return format!(
            "{indent}{}\\{sep}{}",
            &trimmed[..sep_pos],
            &trimmed[sep_pos + 1..]
        );
    }
    inline
}

fn escape_markdown_code(raw: &str) -> String {
    let mut out = String::with_capacity(raw.len());
    for ch in raw.chars() {
        match ch {
            '`' => out.push('\''),
            '\n' | '\r' => out.push(' '),
            c => out.push(c),
        }
    }
    out
}

impl ReportRenderer {
    pub fn render_json(&self, report: &Report) -> Result<String, ReportingError> {
        report.validate()?;
        let mut out = String::new();
        out.push_str("{\n");
        out.push_str(&format!(
            "  \"schema_version\": {},\n",
            escape_json_string(&report.schema_version)
        ));
        out.push_str(&format!(
            "  \"engagement_id\": {},\n",
            escape_json_string(&report.engagement_id)
        ));
        out.push_str(&format!(
            "  \"title\": {},\n",
            escape_json_string(&report.title)
        ));
        out.push_str(&format!(
            "  \"client_org\": {},\n",
            escape_json_string(&report.client_org)
        ));
        out.push_str(&format!(
            "  \"authorization_ref\": {},\n",
            escape_json_string(&report.authorization_ref)
        ));
        out.push_str(&format!(
            "  \"generated_at_utc\": {},\n",
            escape_json_string(&report.generated_at_utc)
        ));
        out.push_str(&format!(
            "  \"final_phase\": {},\n",
            escape_json_string(report.final_phase.as_str())
        ));
        match report.halted_by_stop_condition {
            None => out.push_str("  \"halted_by_stop_condition\": null,\n"),
            Some(trigger) => out.push_str(&format!(
                "  \"halted_by_stop_condition\": {},\n",
                escape_json_string(&format!("{trigger:?}"))
            )),
        }
        match &report.cleanup_attestation {
            None => out.push_str("  \"cleanup_attestation\": null,\n"),
            Some(c) => out.push_str(&format!(
                "  \"cleanup_attestation\": {{\"plan_id\": {}, \"verified_clean\": {}, \"remaining_artifacts\": {}, \"attested_by_role\": {}, \"attested_at_utc\": {}}},\n",
                escape_json_string(&c.plan_id),
                c.verified_clean,
                c.remaining_artifacts,
                escape_json_string(&c.attested_by_role),
                escape_json_string(&c.attested_at_utc),
            )),
        }
        out.push_str(&format!(
            "  \"retention_days\": {},\n",
            report.retention_days
        ));
        out.push_str(&format!(
            "  \"deletion_deadline_utc\": {},\n",
            escape_json_string(&report.deletion_deadline_utc)
        ));
        out.push_str(&format!(
            "  \"input_digest_sha256\": {},\n",
            escape_json_string(&report.input_digest_sha256)
        ));
        out.push_str("  \"executive_summary\": {\n");
        out.push_str(&format!(
            "    \"total_verified_findings\": {},\n",
            report.executive_summary.total_verified_findings
        ));
        out.push_str(&format!(
            "    \"excluded_draft_or_false_positive\": {},\n",
            report.executive_summary.excluded_draft_or_false_positive
        ));
        out.push_str(&format!(
            "    \"critical\": {},\n    \"high\": {},\n    \"medium\": {},\n    \"low\": {},\n    \"none\": {},\n",
            report.executive_summary.critical_count,
            report.executive_summary.high_count,
            report.executive_summary.medium_count,
            report.executive_summary.low_count,
            report.executive_summary.none_count
        ));
        out.push_str(&format!(
            "    \"retested_fixed\": {},\n",
            report.executive_summary.retested_fixed_count
        ));
        out.push_str(&format!(
            "    \"highest_cvss\": {}.{}\n",
            report.executive_summary.highest_cvss_tenths / 10,
            report.executive_summary.highest_cvss_tenths % 10
        ));
        out.push_str("  },\n");
        out.push_str(&format!(
            "  \"executive_narrative\": {},\n",
            escape_json_string(&report.executive_narrative)
        ));
        out.push_str(&format!(
            "  \"methodology_overview\": {},\n",
            escape_json_string(&report.methodology_overview)
        ));
        out.push_str("  \"scope_summary\": [\n");
        for (idx, line) in report.scope_summary.iter().enumerate() {
            let comma = if idx + 1 == report.scope_summary.len() {
                ""
            } else {
                ","
            };
            out.push_str(&format!("    {}{comma}\n", escape_json_string(line)));
        }
        out.push_str("  ],\n");
        out.push_str("  \"findings\": [\n");
        for (idx, f) in report.findings.iter().enumerate() {
            let score = f.cvss_score();
            out.push_str("    {\n");
            out.push_str(&format!(
                "      \"finding_id\": {},\n",
                escape_json_string(&f.finding_id)
            ));
            out.push_str(&format!(
                "      \"rule_id\": {},\n",
                escape_json_string(&f.rule_id)
            ));
            out.push_str(&format!(
                "      \"root_cause_key\": {},\n",
                escape_json_string(&f.root_cause_key)
            ));
            out.push_str(&format!(
                "      \"primary_component\": {},\n",
                escape_json_string(&f.primary_component)
            ));
            out.push_str(&format!(
                "      \"title\": {},\n",
                escape_json_string(&f.title)
            ));
            out.push_str(&format!(
                "      \"status\": {},\n",
                escape_json_string(f.status.as_str())
            ));
            let claim_str = match &f.exploitation_claim {
                ExploitationClaim::NotExploited => "NotExploited".to_string(),
                ExploitationClaim::ValidatedPreconditionOnly => {
                    "ValidatedPreconditionOnly".to_string()
                }
                ExploitationClaim::ControlledLabExploited {
                    lab_proof_evidence_id,
                } => format!("ControlledLabExploited({lab_proof_evidence_id})"),
            };
            out.push_str(&format!(
                "      \"exploitation_claim\": {},\n",
                escape_json_string(&claim_str)
            ));
            out.push_str(&format!(
                "      \"cvss_vector\": {},\n",
                escape_json_string(&f.cvss_vector.canonical_string())
            ));
            out.push_str(&format!(
                "      \"cvss_score\": {},\n",
                score.score_string()
            ));
            out.push_str(&format!(
                "      \"cvss_severity\": {},\n",
                escape_json_string(score.severity.as_str())
            ));
            out.push_str(&format!(
                "      \"business_priority\": {},\n",
                escape_json_string(f.business_priority.as_str())
            ));
            let assets: Vec<String> = f
                .affected_assets
                .iter()
                .map(|a| escape_json_string(a))
                .collect();
            out.push_str(&format!(
                "      \"affected_assets\": [{}],\n",
                assets.join(", ")
            ));
            let dups: Vec<String> = f
                .merged_duplicate_ids
                .iter()
                .map(|d| escape_json_string(d))
                .collect();
            out.push_str(&format!(
                "      \"merged_duplicate_ids\": [{}],\n",
                dups.join(", ")
            ));
            let ev_ids: Vec<String> = f
                .evidence_refs
                .iter()
                .map(|e| escape_json_string(&e.evidence_id))
                .collect();
            out.push_str(&format!(
                "      \"evidence_ids\": [{}],\n",
                ev_ids.join(", ")
            ));
            let cites: Vec<String> = f
                .citation_ids
                .iter()
                .map(|c| escape_json_string(c))
                .collect();
            out.push_str(&format!(
                "      \"citation_ids\": [{}],\n",
                cites.join(", ")
            ));
            out.push_str(&format!(
                "      \"lab_reproduction\": {},\n",
                escape_json_string(&f.lab_reproduction)
            ));
            out.push_str(&format!(
                "      \"impact_analysis\": {},\n",
                escape_json_string(&f.impact_analysis)
            ));
            out.push_str(&format!(
                "      \"remediation\": {},\n",
                escape_json_string(&f.remediation)
            ));
            out.push_str(&format!(
                "      \"cleanup_steps\": {},\n",
                escape_json_string(&f.cleanup_steps)
            ));
            out.push_str(&format!(
                "      \"detection_control\": {},\n",
                escape_json_string(&f.detection_control)
            ));
            match &f.retest {
                None => out.push_str("      \"retest\": null\n"),
                Some(rt) => {
                    let outcome_str = match &rt.outcome {
                        RetestOutcome::FixedVerified => "FixedVerified".to_string(),
                        RetestOutcome::PartiallyMitigated { residual_vector } => {
                            format!("PartiallyMitigated({})", residual_vector.canonical_string())
                        }
                        RetestOutcome::StillVulnerable => "StillVulnerable".to_string(),
                    };
                    out.push_str(&format!(
                        "      \"retest\": {{\"retested_at_utc\": {}, \"retest_evidence_id\": {}, \"outcome\": {}, \"verifier_note\": {}}}\n",
                        escape_json_string(&rt.retested_at_utc),
                        escape_json_string(&rt.retest_evidence_id),
                        escape_json_string(&outcome_str),
                        escape_json_string(&rt.verifier_note)
                    ));
                }
            }
            let comma = if idx + 1 == report.findings.len() {
                ""
            } else {
                ","
            };
            out.push_str(&format!("    }}{comma}\n"));
        }
        out.push_str("  ],\n");
        out.push_str("  \"excluded_non_actionable\": [\n");
        for (idx, (ex_id, ex_status)) in report.excluded_non_actionable_ids.iter().enumerate() {
            let comma = if idx + 1 == report.excluded_non_actionable_ids.len() {
                ""
            } else {
                ","
            };
            out.push_str(&format!(
                "    {{\"finding_id\": {}, \"status\": {}}}{comma}\n",
                escape_json_string(ex_id),
                escape_json_string(ex_status.as_str())
            ));
        }
        out.push_str("  ],\n");
        out.push_str("  \"citations\": [\n");
        let cite_items: Vec<&StandardCitation> = report.citations.values().collect();
        for (idx, c) in cite_items.iter().enumerate() {
            let comma = if idx + 1 == cite_items.len() { "" } else { "," };
            out.push_str(&format!(
                "    {{\"id\": {}, \"title\": {}, \"reference\": {}}}{comma}\n",
                escape_json_string(&c.citation_id),
                escape_json_string(&c.title),
                escape_json_string(&c.reference)
            ));
        }
        out.push_str("  ]\n");
        out.push('}');
        Ok(out)
    }

    pub fn render_executive_markdown(&self, report: &Report) -> Result<String, ReportingError> {
        report.validate()?;
        let mut out = String::new();
        out.push_str(&format!(
            "# Executive Security Report — {}\n\n",
            escape_markdown_inline(&report.title)
        ));
        let halted_label = report
            .halted_by_stop_condition
            .map_or_else(|| "None".to_string(), |s| format!("{s:?}"));
        out.push_str(&format!(
            "- Schema Version: `{}`\n- Engagement ID: `{}`\n- Client: {}\n- Authorization: {}\n- Generated UTC: `{}`\n- Final Phase: `{}` | Halted By Stop Condition: `{}`\n- Retention / Deletion Deadline: {} ngày (tối đa `{}`)\n\n",
            escape_markdown_code(&report.schema_version),
            escape_markdown_code(&report.engagement_id),
            escape_markdown_inline(&report.client_org),
            escape_markdown_inline(&report.authorization_ref),
            escape_markdown_code(&report.generated_at_utc),
            escape_markdown_code(report.final_phase.as_str()),
            escape_markdown_code(&halted_label),
            report.retention_days,
            escape_markdown_code(&report.deletion_deadline_utc),
        ));
        out.push_str("## Executive Summary\n\n");
        out.push_str(&format!(
            "{}\n\n",
            escape_markdown_block(&report.executive_narrative)
        ));
        out.push_str("| Verified | Critical | High | Medium | Low | Retested Fixed | Excluded |\n|---:|---:|---:|---:|---:|---:|---:|\n");
        out.push_str(&format!(
            "| {} | {} | {} | {} | {} | {} | {} |\n\n",
            report.executive_summary.total_verified_findings,
            report.executive_summary.critical_count,
            report.executive_summary.high_count,
            report.executive_summary.medium_count,
            report.executive_summary.low_count,
            report.executive_summary.retested_fixed_count,
            report.executive_summary.excluded_draft_or_false_positive
        ));
        out.push_str("| ID | Priority | CVSS 3.1 | Status | Title |\n|---|---|---|---|---|\n");
        for f in &report.findings {
            let sc = f.cvss_score();
            out.push_str(&format!(
                "| {} | {} | {} ({}) | {} | {} |\n",
                escape_markdown_inline(&f.finding_id),
                escape_markdown_inline(f.business_priority.as_str()),
                score_display(sc),
                escape_markdown_inline(sc.severity.as_str()),
                escape_markdown_inline(f.status.as_str()),
                escape_markdown_inline(&f.title)
            ));
        }
        Ok(out)
    }

    pub fn render_technical_markdown(&self, report: &Report) -> Result<String, ReportingError> {
        report.validate()?;
        let mut out = String::new();
        out.push_str(&format!(
            "# Technical Assessment Report — {}\n\n",
            escape_markdown_inline(&report.title)
        ));
        let halted_label = report
            .halted_by_stop_condition
            .map_or_else(|| "None".to_string(), |s| format!("{s:?}"));
        out.push_str(&format!(
            "- Schema Version: `{}`\n- Engagement ID: `{}`\n- Halted By Stop Condition: `{}`\n- Input Digest: `{}`\n\n",
            escape_markdown_code(&report.schema_version),
            escape_markdown_code(&report.engagement_id),
            escape_markdown_code(&halted_label),
            escape_markdown_code(&report.input_digest_sha256)
        ));
        out.push_str("## Scope & Methodology\n\n");
        out.push_str(&format!(
            "{}\n\n",
            escape_markdown_block(&report.methodology_overview)
        ));
        for item in &report.scope_summary {
            out.push_str(&format!("- {}\n", escape_markdown_inline(item)));
        }
        if let Some(cleanup) = &report.cleanup_attestation {
            out.push_str(&format!(
                "\n- Cleanup Attestation `{}`: verified_clean={}, remaining_artifacts={}, attested_at=`{}`\n",
                escape_markdown_code(&cleanup.plan_id),
                cleanup.verified_clean,
                cleanup.remaining_artifacts,
                escape_markdown_code(&cleanup.attested_at_utc)
            ));
        }
        out.push_str("\n## Verified Findings\n\n");
        for f in &report.findings {
            let sc = f.cvss_score();
            out.push_str(&format!(
                "### {} — {}\n\n",
                escape_markdown_inline(&f.finding_id),
                escape_markdown_inline(&f.title)
            ));
            out.push_str(&format!(
                "- Status: `{}` | CVSS: `{}` ({} — `{}`) | Priority: `{}`\n",
                escape_markdown_code(f.status.as_str()),
                score_display(sc),
                escape_markdown_inline(sc.severity.as_str()),
                escape_markdown_code(&f.cvss_vector.canonical_string()),
                escape_markdown_code(f.business_priority.as_str())
            ));
            let assets: Vec<String> = f
                .affected_assets
                .iter()
                .map(|a| escape_markdown_inline(a))
                .collect();
            out.push_str(&format!("- Affected Assets: {}\n", assets.join(", ")));
            if !f.merged_duplicate_ids.is_empty() {
                let merged: Vec<String> = f
                    .merged_duplicate_ids
                    .iter()
                    .map(|m| escape_markdown_inline(m))
                    .collect();
                out.push_str(&format!("- Merged Duplicates: {}\n", merged.join(", ")));
            }
            out.push_str(&format!(
                "- Reproduction (Lab): {}\n",
                escape_markdown_inline(&f.lab_reproduction)
            ));
            out.push_str(&format!(
                "- Impact: {}\n",
                escape_markdown_inline(&f.impact_analysis)
            ));
            out.push_str(&format!(
                "- Remediation: {}\n",
                escape_markdown_inline(&f.remediation)
            ));
            out.push_str(&format!(
                "- Cleanup: {}\n",
                escape_markdown_inline(&f.cleanup_steps)
            ));
            out.push_str(&format!(
                "- Detection: {}\n",
                escape_markdown_inline(&f.detection_control)
            ));
            out.push_str("- Evidence Chain:\n");
            for ev in &f.evidence_refs {
                let sha_short = ev.artifact_sha256.get(..16).unwrap_or(&ev.artifact_sha256);
                out.push_str(&format!(
                    "  - `{}` @ `{}` (`{}` | sha256=`{}` | redacted={}): {}\n",
                    escape_markdown_code(&ev.evidence_id),
                    escape_markdown_code(&ev.captured_at_utc),
                    escape_markdown_code(&ev.source_uri),
                    escape_markdown_code(sha_short),
                    ev.redaction_count,
                    escape_markdown_inline(&ev.redacted_excerpt)
                ));
            }
            if let Some(rt) = &f.retest {
                let outcome_label = match &rt.outcome {
                    RetestOutcome::FixedVerified => "FixedVerified".to_string(),
                    RetestOutcome::PartiallyMitigated { residual_vector } => {
                        format!("PartiallyMitigated({})", residual_vector.canonical_string())
                    }
                    RetestOutcome::StillVulnerable => "StillVulnerable".to_string(),
                };
                out.push_str(&format!(
                    "- Retest @ `{}` (`{}` — `{}`): {}\n",
                    escape_markdown_code(&rt.retested_at_utc),
                    escape_markdown_code(&rt.retest_evidence_id),
                    escape_markdown_code(&outcome_label),
                    escape_markdown_inline(&rt.verifier_note)
                ));
            }
            let cites: Vec<String> = f
                .citation_ids
                .iter()
                .map(|c| escape_markdown_inline(c))
                .collect();
            out.push_str(&format!("- Citations: {}\n\n", cites.join(", ")));
        }
        out.push_str("## Citations\n\n");
        for c in report.citations.values() {
            out.push_str(&format!(
                "- `{}`: {} ({})\n",
                escape_markdown_code(&c.citation_id),
                escape_markdown_inline(&c.title),
                escape_markdown_inline(&c.reference)
            ));
        }
        Ok(out)
    }

    pub fn render_markdown(&self, report: &Report) -> Result<String, ReportingError> {
        let exec = self.render_executive_markdown(report)?;
        let tech = self.render_technical_markdown(report)?;
        Ok(format!("{exec}\n---\n\n{tech}"))
    }
}

fn score_display(score: Cvss31Score) -> String {
    score.score_string()
}

// ============================================================================
// 7. ENGAGEMENT ORCHESTRATOR & FIXTURE TỔNG HỢP (`Engagement`, `MockReportingLab`)
// ============================================================================

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct Engagement {
    pub scope: Scope,
    pub roe: RulesOfEngagement,
    pub phase: EngagementPhase,
    pub halted_by: Option<StopConditionTrigger>,
    pub approvals: BTreeMap<String, AgentApprovalRecord>,
    pub evidence_chain: EvidenceChain,
    pub findings: Vec<Finding>,
    pub cleanup_attestation: Option<CleanupAttestation>,
    pub audit_log: Vec<AuditLogEntry>,
    pub citations: BTreeMap<String, StandardCitation>,
    pub data_purged: bool,
}

impl Engagement {
    pub fn new(scope: Scope, roe: RulesOfEngagement) -> Result<Self, ReportingError> {
        scope.validate()?;
        roe.validate()?;
        Ok(Self {
            scope,
            roe,
            phase: EngagementPhase::Prepare,
            halted_by: None,
            approvals: BTreeMap::new(),
            evidence_chain: EvidenceChain::new(),
            findings: Vec::new(),
            cleanup_attestation: None,
            audit_log: Vec::new(),
            citations: standard_citations(),
            data_purged: false,
        })
    }

    fn append_audit_log(
        &mut self,
        timestamp_utc: &str,
        actor: &str,
        action: &str,
        target_label: &str,
        decision: AuditDecision,
        detail: &str,
    ) -> Result<(), ReportingError> {
        if !is_valid_iso8601_utc(timestamp_utc) {
            return Err(ReportingError::InvalidTimestamp(timestamp_utc.to_string()));
        }
        validate_no_raw_secret("audit.actor", actor)?;
        validate_no_raw_secret("audit.action", action)?;
        validate_no_raw_secret("audit.target_label", target_label)?;
        validate_no_raw_secret("audit.detail", detail)?;
        let prev_hash = self
            .audit_log
            .last()
            .map_or_else(|| AUDIT_GENESIS_HASH.to_string(), |e| e.entry_hash.clone());
        let sequence = self.audit_log.len() as u64;
        let mut entry = AuditLogEntry {
            sequence,
            timestamp_utc: timestamp_utc.to_string(),
            phase: self.phase,
            actor: actor.to_string(),
            action: action.to_string(),
            target_label: target_label.to_string(),
            decision,
            detail: detail.to_string(),
            prev_hash,
            entry_hash: String::new(),
        };
        entry.entry_hash = entry.compute_hash();
        self.audit_log.push(entry);
        Ok(())
    }

    pub fn verify_audit_log(&self) -> Result<(), ReportingError> {
        let mut expected_prev = AUDIT_GENESIS_HASH;
        for (idx, entry) in self.audit_log.iter().enumerate() {
            if entry.sequence != idx as u64 {
                return Err(ReportingError::AuditLogTampered(format!(
                    "sequence lệch tại {idx}"
                )));
            }
            if !is_valid_iso8601_utc(&entry.timestamp_utc) {
                return Err(ReportingError::AuditLogTampered(format!(
                    "timestamp không hợp lệ tại {idx}"
                )));
            }
            validate_no_raw_secret("audit.actor", &entry.actor)?;
            validate_no_raw_secret("audit.action", &entry.action)?;
            validate_no_raw_secret("audit.target_label", &entry.target_label)?;
            validate_no_raw_secret("audit.detail", &entry.detail)?;
            if entry.prev_hash != expected_prev {
                return Err(ReportingError::AuditLogTampered(format!(
                    "prev_hash đứt gãy tại sequence {idx}"
                )));
            }
            let recomputed = entry.compute_hash();
            if recomputed != entry.entry_hash {
                return Err(ReportingError::AuditLogTampered(format!(
                    "entry_hash không khớp tại sequence {idx}"
                )));
            }
            expected_prev = entry.entry_hash.as_str();
        }
        Ok(())
    }

    pub fn register_approval(&mut self, record: AgentApprovalRecord) -> Result<(), ReportingError> {
        record.validate()?;
        if self.approvals.contains_key(&record.approval_id) {
            return Err(ReportingError::DuplicateIdentifier(format!(
                "approval_id '{}' đã tồn tại",
                record.approval_id
            )));
        }
        self.approvals.insert(record.approval_id.clone(), record);
        Ok(())
    }

    pub fn advance_phase(
        &mut self,
        next: EngagementPhase,
        at_utc: &str,
        actor: &str,
    ) -> Result<(), ReportingError> {
        if let Some(trigger) = self.halted_by {
            return Err(ReportingError::StopConditionTriggered(trigger));
        }
        if !self.phase.is_valid_transition(next) {
            return Err(ReportingError::InvalidPhaseTransition {
                from: self.phase,
                to: next,
            });
        }
        if next == EngagementPhase::Report {
            let attestation = self.cleanup_attestation.as_ref().ok_or_else(|| {
                ReportingError::CleanupNotVerified(
                    "cần CleanupAttestation trước khi lập Report".to_string(),
                )
            })?;
            attestation.validate()?;
            if !attestation.verified_clean || attestation.remaining_artifacts > 0 {
                return Err(ReportingError::CleanupNotVerified(
                    "cleanup chưa sạch (verified_clean=false hoặc còn tàn dư)".to_string(),
                ));
            }
        }
        if !is_valid_iso8601_utc(at_utc) {
            return Err(ReportingError::InvalidTimestamp(at_utc.to_string()));
        }
        validate_no_raw_secret("audit.actor", actor)?;
        let from = self.phase;
        let transition_detail = format!("{from:?} -> {next:?}");
        self.phase = next;
        self.append_audit_log(
            at_utc,
            actor,
            "advance_phase",
            "engagement",
            AuditDecision::PhaseTransition,
            &transition_detail,
        )?;
        Ok(())
    }

    pub fn record_cleanup_attestation(
        &mut self,
        attestation: CleanupAttestation,
    ) -> Result<(), ReportingError> {
        attestation.validate()?;
        self.cleanup_attestation = Some(attestation);
        Ok(())
    }

    pub fn trigger_stop_condition(
        &mut self,
        trigger: StopConditionTrigger,
        timestamp_utc: &str,
        actor: &str,
        note: &str,
    ) -> Result<(), ReportingError> {
        if !self.roe.stop_conditions.contains(&trigger) {
            return Err(ReportingError::AuthorizationMissing(format!(
                "stop condition {trigger:?} chưa đăng ký trong RoE"
            )));
        }
        self.append_audit_log(
            timestamp_utc,
            actor,
            "stop_condition",
            "engagement",
            AuditDecision::StopConditionHalted,
            note,
        )?;
        self.halted_by = Some(trigger);
        Ok(())
    }

    /// Thực thi một bước mô phỏng có kiểm tra đầy đủ cổng RoE/Scope/Window/Approval **trước khi** gọi `module_fn`.
    /// Nếu bị chặn ở bất kỳ điều kiện nào, `module_fn` không bao giờ được gọi.
    pub fn execute_module_step<F>(
        &mut self,
        req: &ModuleStepRequest,
        module_fn: F,
    ) -> Result<ModuleStepOutcome, ReportingError>
    where
        F: FnOnce(&ModuleStepRequest) -> Result<String, ReportingError>,
    {
        if !is_valid_iso8601_utc(&req.timestamp_utc) {
            return Err(ReportingError::InvalidTimestamp(req.timestamp_utc.clone()));
        }
        validate_no_raw_secret("step.actor", &req.actor)?;
        validate_no_raw_secret("step.step_id", &req.step_id)?;
        let target_label = req.target.canonical_label()?;
        if let Some(trigger) = self.halted_by {
            self.append_audit_log(
                &req.timestamp_utc,
                &req.actor,
                &req.step_id,
                &target_label,
                AuditDecision::Blocked,
                "blocked: engagement already halted by stop condition",
            )?;
            return Err(ReportingError::StopConditionTriggered(trigger));
        }
        if !self.roe.signed_written_consent {
            self.append_audit_log(
                &req.timestamp_utc,
                &req.actor,
                &req.step_id,
                &target_label,
                AuditDecision::Blocked,
                "blocked: missing signed_written_consent",
            )?;
            return Err(ReportingError::AuthorizationMissing(
                "chưa có ủy quyền bằng văn bản".to_string(),
            ));
        }
        self.roe.validate()?;
        self.scope.validate()?;
        if req.timestamp_utc < self.roe.window_start_utc
            || req.timestamp_utc > self.roe.window_end_utc
        {
            self.append_audit_log(
                &req.timestamp_utc,
                &req.actor,
                &req.step_id,
                &target_label,
                AuditDecision::Blocked,
                "blocked: outside engagement time window",
            )?;
            return Err(ReportingError::OutsideEngagementWindow(format!(
                "mốc {} nằm ngoài cửa sổ [{}..{}]",
                req.timestamp_utc, self.roe.window_start_utc, self.roe.window_end_utc
            )));
        }
        if let Some(prohibited) = req.prohibited_action {
            self.append_audit_log(
                &req.timestamp_utc,
                &req.actor,
                &req.step_id,
                &target_label,
                AuditDecision::Blocked,
                &format!("blocked prohibited action {prohibited:?}"),
            )?;
            return Err(ReportingError::ProhibitedActionBlocked(prohibited));
        }
        if let Err(err) = self.scope.check_target(&req.target, &req.timestamp_utc) {
            self.append_audit_log(
                &req.timestamp_utc,
                &req.actor,
                &req.step_id,
                &target_label,
                AuditDecision::Blocked,
                &format!("blocked scope check: {err}"),
            )?;
            return Err(err);
        }
        if !self.roe.allowed_methods.contains(&req.method) {
            self.append_audit_log(
                &req.timestamp_utc,
                &req.actor,
                &req.step_id,
                &target_label,
                AuditDecision::Blocked,
                &format!("blocked unauthorized method {:?}", req.method),
            )?;
            return Err(ReportingError::MethodNotAuthorized(req.method));
        }
        if !self.phase.allows_method(req.method) {
            self.append_audit_log(
                &req.timestamp_utc,
                &req.actor,
                &req.step_id,
                &target_label,
                AuditDecision::Blocked,
                &format!("blocked method {:?} in phase {:?}", req.method, self.phase),
            )?;
            return Err(ReportingError::MethodNotAuthorized(req.method));
        }
        if self
            .roe
            .agent_human_approval_required_for
            .contains(&req.method)
        {
            let has_valid_approval = self.approvals.values().any(|ap| {
                ap.method == req.method
                    && ap.target.canonical_label().ok().as_deref() == Some(target_label.as_str())
                    && req.timestamp_utc >= ap.approved_at_utc
                    && req.timestamp_utc <= ap.expires_at_utc
            });
            if !has_valid_approval {
                self.append_audit_log(
                    &req.timestamp_utc,
                    &req.actor,
                    &req.step_id,
                    &target_label,
                    AuditDecision::Blocked,
                    &format!("blocked missing human approval for {:?}", req.method),
                )?;
                return Err(ReportingError::HumanApprovalRequired(req.method));
            }
        }

        let summary = module_fn(req)?;
        validate_no_raw_secret("module_step.summary", &summary)?;
        self.append_audit_log(
            &req.timestamp_utc,
            &req.actor,
            &req.step_id,
            &target_label,
            AuditDecision::Allowed,
            &summary,
        )?;
        Ok(ModuleStepOutcome {
            step_id: req.step_id.clone(),
            target_label,
            summary,
        })
    }

    pub fn add_evidence(&mut self, ev: EvidenceRef) -> Result<(), ReportingError> {
        self.evidence_chain.append(ev)
    }

    fn validate_finding_scope_and_window(&self, finding: &Finding) -> Result<(), ReportingError> {
        let latest_ts = finding
            .evidence_refs
            .iter()
            .map(|e| e.captured_at_utc.as_str())
            .max()
            .unwrap_or(self.roe.window_start_utc.as_str());
        if finding.status != FindingStatus::Retested {
            for ev in &finding.evidence_refs {
                if ev.captured_at_utc < self.roe.window_start_utc
                    || ev.captured_at_utc > self.roe.window_end_utc
                {
                    return Err(ReportingError::OutsideEngagementWindow(format!(
                        "bằng chứng '{}' ({}) nằm ngoài cửa sổ kiểm thử",
                        ev.evidence_id, ev.captured_at_utc
                    )));
                }
            }
        }
        for asset in &finding.affected_assets {
            let target = parse_scope_target_str(asset)?;
            self.scope.check_target(&target, latest_ts)?;
        }
        let component_host = finding
            .primary_component
            .split('/')
            .next()
            .unwrap_or("")
            .trim();
        let comp_target = parse_scope_target_str(component_host)?;
        self.scope.check_target(&comp_target, latest_ts)?;
        Ok(())
    }

    pub fn add_finding(&mut self, finding: Finding) -> Result<(), ReportingError> {
        finding.validate(&self.evidence_chain)?;
        self.validate_finding_scope_and_window(&finding)?;
        if self
            .findings
            .iter()
            .any(|existing| existing.finding_id == finding.finding_id)
        {
            return Err(ReportingError::DuplicateIdentifier(format!(
                "finding_id '{}' đã tồn tại",
                finding.finding_id
            )));
        }
        for cid in &finding.citation_ids {
            if !self.citations.contains_key(cid) {
                return Err(ReportingError::ReportSchemaInvalid(format!(
                    "citation '{cid}' chưa đăng ký trong engagement.citations"
                )));
            }
        }
        self.findings.push(finding);
        Ok(())
    }

    pub fn apply_retest(
        &mut self,
        finding_id: &str,
        retest_evidence: EvidenceRef,
        outcome: RetestOutcome,
        verifier_note: &str,
        actor: &str,
    ) -> Result<(), ReportingError> {
        if let Some(trigger) = self.halted_by {
            return Err(ReportingError::StopConditionTriggered(trigger));
        }
        let retest_ts = retest_evidence.captured_at_utc.clone();
        let retest_ev_id = retest_evidence.evidence_id.clone();
        if self.phase != EngagementPhase::Retest {
            return Err(ReportingError::InvalidPhaseTransition {
                from: self.phase,
                to: EngagementPhase::Retest,
            });
        }
        validate_no_raw_secret("retest.actor", actor)?;
        validate_no_raw_secret("retest.verifier_note", verifier_note)?;
        if actor.trim().is_empty() || verifier_note.trim().is_empty() {
            return Err(ReportingError::EvidenceInvalid(
                "actor hoặc verifier_note của retest không được rỗng".to_string(),
            ));
        }

        let idx = self
            .findings
            .iter()
            .position(|f| f.finding_id == finding_id)
            .ok_or_else(|| {
                ReportingError::EvidenceInvalid(format!("không tìm thấy finding '{finding_id}'"))
            })?;
        let current_status = self.findings[idx].status;
        if !current_status.can_transition_to(FindingStatus::Retested) {
            return Err(ReportingError::IllegalFindingTransition {
                from: current_status,
                to: FindingStatus::Retested,
            });
        }
        if let Some(latest_prior) = self.findings[idx]
            .evidence_refs
            .iter()
            .map(|e| e.captured_at_utc.as_str())
            .max()
            && retest_ts.as_str() < latest_prior
        {
            return Err(ReportingError::InvalidTimestamp(format!(
                "retest timestamp '{retest_ts}' sớm hơn bằng chứng trước '{latest_prior}'"
            )));
        }

        self.evidence_chain.append(retest_evidence.clone())?;
        let f = &mut self.findings[idx];
        f.status = FindingStatus::Retested;
        f.evidence_refs.push(retest_evidence);
        if let RetestOutcome::PartiallyMitigated { residual_vector } = outcome {
            f.cvss_vector = residual_vector;
            f.business_priority = compute_business_priority(
                residual_vector.score().severity,
                f.asset_criticality,
                f.exposure_context,
            );
        }
        f.retest = Some(RetestRecord {
            retested_at_utc: retest_ts.clone(),
            retest_evidence_id: retest_ev_id.clone(),
            outcome,
            verifier_note: verifier_note.to_string(),
        });

        self.append_audit_log(
            &retest_ts,
            actor,
            "apply_retest",
            finding_id,
            AuditDecision::FindingRetested,
            &format!("retested with evidence {retest_ev_id}"),
        )?;
        Ok(())
    }

    pub fn evaluate_retention(
        &self,
        check_at_utc: &str,
    ) -> Result<RetentionDisposition, ReportingError> {
        if !is_valid_iso8601_utc(check_at_utc) {
            return Err(ReportingError::InvalidTimestamp(check_at_utc.to_string()));
        }
        if self.data_purged {
            return Ok(RetentionDisposition::PurgedConfirmed);
        }
        if check_at_utc <= self.roe.window_end_utc.as_str() {
            Ok(RetentionDisposition::ActiveWindow)
        } else if check_at_utc <= self.roe.data_handling.deletion_deadline_utc.as_str() {
            Ok(RetentionDisposition::WithinRetentionPeriod)
        } else {
            Ok(RetentionDisposition::DeletionRequiredNow)
        }
    }

    pub fn build_report(
        &self,
        generated_at_utc: &str,
        title: &str,
    ) -> Result<Report, ReportingError> {
        if !is_valid_iso8601_utc(generated_at_utc) {
            return Err(ReportingError::InvalidTimestamp(
                generated_at_utc.to_string(),
            ));
        }
        self.scope.validate()?;
        self.roe.validate()?;
        self.evidence_chain.verify_integrity()?;
        self.verify_audit_log()?;
        if !matches!(
            self.phase,
            EngagementPhase::Report | EngagementPhase::Retest
        ) {
            return Err(ReportingError::CleanupNotVerified(format!(
                "không thể build report khi đang ở pha {:?}",
                self.phase
            )));
        }
        if self.halted_by.is_none() {
            let attestation = self.cleanup_attestation.as_ref().ok_or_else(|| {
                ReportingError::CleanupNotVerified(
                    "cần CleanupAttestation trước khi lập Report".to_string(),
                )
            })?;
            attestation.validate()?;
            if !attestation.verified_clean || attestation.remaining_artifacts > 0 {
                return Err(ReportingError::CleanupNotVerified(
                    "cleanup chưa sạch (verified_clean=false hoặc còn tàn dư)".to_string(),
                ));
            }
        } else if let Some(att) = &self.cleanup_attestation {
            att.validate()?;
        }
        for finding in &self.findings {
            self.validate_finding_scope_and_window(finding)?;
        }

        let dedup = deduplicate_findings(&self.findings, &self.evidence_chain)?;
        let mut critical_count = 0usize;
        let mut high_count = 0usize;
        let mut medium_count = 0usize;
        let mut low_count = 0usize;
        let mut none_count = 0usize;
        let mut retested_fixed_count = 0usize;
        let mut highest_cvss_tenths = 0u8;

        for f in &dedup.canonical_findings {
            let sc = f.cvss_score();
            let expected_priority =
                compute_business_priority(sc.severity, f.asset_criticality, f.exposure_context);
            if f.business_priority != expected_priority {
                return Err(ReportingError::ReportSchemaInvalid(format!(
                    "business_priority của '{}' không khớp (kỳ vọng {:?}, nhận {:?})",
                    f.finding_id, expected_priority, f.business_priority
                )));
            }
            highest_cvss_tenths = highest_cvss_tenths.max(sc.score_tenths);
            match sc.severity {
                CvssSeverity::Critical => critical_count += 1,
                CvssSeverity::High => high_count += 1,
                CvssSeverity::Medium => medium_count += 1,
                CvssSeverity::Low => low_count += 1,
                CvssSeverity::None => none_count += 1,
            }
            if f.status == FindingStatus::Retested
                && f.retest
                    .as_ref()
                    .is_some_and(|r| matches!(r.outcome, RetestOutcome::FixedVerified))
            {
                retested_fixed_count += 1;
            }
        }

        let excluded_ids: Vec<(String, FindingStatus)> = dedup
            .excluded_non_actionable
            .iter()
            .map(|f| (f.finding_id.clone(), f.status))
            .collect();

        let evidence_manifest: Vec<EvidenceRef> = self
            .evidence_chain
            .links()
            .iter()
            .map(|l| l.evidence.clone())
            .collect();

        let mut scope_summary: Vec<String> = self
            .scope
            .allowed_domains
            .iter()
            .map(|d| format!("Allowed domain: {d}"))
            .collect();
        for z in &self.scope.allow_subdomains_of {
            scope_summary.push(format!("Allowed zone: *.{z}"));
        }
        for c in &self.scope.allowed_cidrs {
            scope_summary.push(format!("Allowed CIDR (RFC 1918): {c}"));
        }
        for d in &self.scope.denied_domains {
            scope_summary.push(format!("Denied zone: {d}"));
        }

        let mut report = Report {
            schema_version: REPORT_SCHEMA_VERSION.to_string(),
            engagement_id: self.roe.engagement_id.clone(),
            title: title.to_string(),
            client_org: self.roe.client_org.clone(),
            authorization_ref: self.roe.authorization_ref.clone(),
            generated_at_utc: generated_at_utc.to_string(),
            final_phase: self.phase,
            halted_by_stop_condition: self.halted_by,
            cleanup_attestation: self.cleanup_attestation.clone(),
            retention_days: self.roe.data_handling.retention_days,
            deletion_deadline_utc: self.roe.data_handling.deletion_deadline_utc.clone(),
            executive_summary: ExecutiveSummary {
                total_verified_findings: dedup.canonical_findings.len(),
                excluded_draft_or_false_positive: excluded_ids.len(),
                critical_count,
                high_count,
                medium_count,
                low_count,
                none_count,
                retested_fixed_count,
                highest_cvss_tenths,
            },
            executive_narrative: format!(
                "Đánh giá có ủy quyền {} xác minh {} phát hiện (loại {} mục Draft/FalsePositive trước khi xuất báo cáo). Mọi dấu vết lab đã được kiểm tra cleanup sạch trước khi phát hành.",
                self.roe.engagement_id,
                dedup.canonical_findings.len(),
                excluded_ids.len()
            ),
            methodology_overview:
                "Tuân thủ vòng đời 8 pha (Prepare -> Recon -> Validate -> LabExploit -> PostExploitationAudit -> Cleanup -> Report -> Retest), chuỗi bằng chứng SHA-256 tự che secret/PII, CVSS v3.1 chuẩn FIRST.org và tách bạch BusinessPriority."
                    .to_string(),
            scope_summary,
            findings: dedup.canonical_findings,
            excluded_non_actionable_ids: excluded_ids,
            evidence_manifest,
            citations: self.citations.clone(),
            input_digest_sha256: String::new(),
        };
        report.input_digest_sha256 = report.compute_digest();
        report.validate()?;
        Ok(report)
    }
}

pub fn standard_citations() -> BTreeMap<String, StandardCitation> {
    let items = [
        StandardCitation {
            citation_id: "FIRST-CVSS-3.1".to_string(),
            title: "Common Vulnerability Scoring System v3.1 Specification".to_string(),
            reference: "FIRST.org CVSS v3.1 Section 7 & Appendix A".to_string(),
        },
        StandardCitation {
            citation_id: "NIST-800-115".to_string(),
            title: "Technical Guide to Information Security Testing and Assessment".to_string(),
            reference: "NIST SP 800-115".to_string(),
        },
        StandardCitation {
            citation_id: "ISO-29147".to_string(),
            title: "Information technology — Security techniques — Vulnerability disclosure"
                .to_string(),
            reference: "ISO/IEC 29147".to_string(),
        },
        StandardCitation {
            citation_id: "OWASP-WSTG-ATHZ".to_string(),
            title: "OWASP Web Security Testing Guide — Authorization Testing".to_string(),
            reference: "OWASP WSTG v4.2".to_string(),
        },
    ];
    items
        .into_iter()
        .map(|c| (c.citation_id.clone(), c))
        .collect()
}

pub struct MockReportingLab;

impl MockReportingLab {
    /// Tạo sẵn `Engagement` tổng hợp chuẩn qua đủ các pha tới `Report`,
    /// sẵn sàng minh họa khử trùng lặp, redaction, CVSS 3.1, render và retest.
    pub fn standard_engagement() -> Result<Engagement, ReportingError> {
        let mut allowed_domains = BTreeSet::new();
        allowed_domains.insert("portal.corp.example".to_string());
        allowed_domains.insert("cdn-auth.vendor.example".to_string());

        let mut allow_subdomains_of = BTreeSet::new();
        allow_subdomains_of.insert("corp.example".to_string());

        let mut denied_domains = BTreeSet::new();
        denied_domains.insert("payroll.corp.example".to_string());
        denied_domains.insert("prod-billing.corp.example".to_string());

        let mut allowed_cidrs = BTreeSet::new();
        allowed_cidrs.insert("10.42.0.0/24".to_string());

        let mut denied_ips = BTreeSet::new();
        denied_ips.insert("10.42.0.254".to_string());

        let mut third_party_assets = BTreeMap::new();
        third_party_assets.insert(
            "cdn-auth.vendor.example".to_string(),
            ThirdPartyAuthorization {
                host: "cdn-auth.vendor.example".to_string(),
                owner_org: "Vendor Identity Lab".to_string(),
                written_consent_ref: "3P-CONSENT-2026-09".to_string(),
                approved_until_utc: "2026-02-20T00:00:00Z".to_string(),
            },
        );
        third_party_assets.insert(
            "legacy-sso.vendor.example".to_string(),
            ThirdPartyAuthorization {
                host: "legacy-sso.vendor.example".to_string(),
                owner_org: "Vendor SSO Legacy".to_string(),
                written_consent_ref: "3P-CONSENT-EXPIRED-01".to_string(),
                approved_until_utc: "2026-01-15T00:00:00Z".to_string(),
            },
        );

        let scope = Scope {
            allowed_domains,
            allow_subdomains_of,
            denied_domains,
            allowed_cidrs,
            denied_ips,
            third_party_assets,
        };

        let roe = RulesOfEngagement {
            engagement_id: ENGAGEMENT_ID.to_string(),
            client_org: "ACME Synthetic Lab Holdings".to_string(),
            signed_written_consent: true,
            authorization_ref: "ROE-2026-111-SIGNED".to_string(),
            authorized_operator_contact: "redteam-lead@lab.example".to_string(),
            emergency_client_contact: "soc-oncall@corp.example".to_string(),
            window_start_utc: WINDOW_START_UTC.to_string(),
            window_end_utc: WINDOW_END_UTC.to_string(),
            allowed_methods: BTreeSet::from([
                TestingMethod::PassiveRecon,
                TestingMethod::ConfigAudit,
                TestingMethod::VulnerabilityValidation,
                TestingMethod::ControlledLabExploit,
                TestingMethod::PostExploitReadOnlyAudit,
                TestingMethod::CleanupVerification,
                TestingMethod::RetestVerification,
            ]),
            prohibited_actions: BTreeSet::from([
                ProhibitedAction::DenialOfService,
                ProhibitedAction::SocialEngineeringPhishing,
                ProhibitedAction::CredentialDumping,
                ProhibitedAction::DestructivePayload,
                ProhibitedAction::UnapprovedThirdPartyPivot,
                ProhibitedAction::ExternalExfiltration,
            ]),
            stop_conditions: BTreeSet::from([
                StopConditionTrigger::ProductionAvailabilityDegradation,
                StopConditionTrigger::UnintendedDataExposure,
                StopConditionTrigger::ActiveIncidentResponseDetected,
                StopConditionTrigger::ScopeBoundaryViolation,
                StopConditionTrigger::EmergencyClientAbort,
            ]),
            agent_human_approval_required_for: BTreeSet::from([
                TestingMethod::ControlledLabExploit,
            ]),
            data_handling: DataHandlingPolicy {
                data_minimization_enforced: true,
                redact_secrets_and_pii: true,
                retention_days: 90,
                deletion_deadline_utc: RETENTION_DEADLINE_UTC.to_string(),
                storage_classification: "Confidential-Lab-Encrypted".to_string(),
            },
            disclosure: DisclosurePolicy {
                mode: "Coordinated Private Client Disclosure".to_string(),
                coordination_window_days: 45,
                embargo_until_utc: "2026-04-01T00:00:00Z".to_string(),
                remediation_contact: "appsec-triage@corp.example".to_string(),
            },
        };

        let mut eng = Engagement::new(scope, roe)?;
        eng.register_approval(AgentApprovalRecord {
            approval_id: "APR-001".to_string(),
            approver_role: "Lead Security Assessor".to_string(),
            method: TestingMethod::ControlledLabExploit,
            target: TargetAsset::Domain("api.corp.example".to_string()),
            approved_at_utc: "2026-02-05T08:00:00Z".to_string(),
            expires_at_utc: "2026-02-06T20:00:00Z".to_string(),
            justification: "Xác nhận PoC an toàn trên lab api.corp.example".to_string(),
        })?;

        eng.advance_phase(EngagementPhase::Recon, "2026-02-02T09:00:00Z", "operator")?;
        eng.execute_module_step(
            &ModuleStepRequest {
                step_id: "step-recon-01".to_string(),
                actor: "recon-agent".to_string(),
                method: TestingMethod::PassiveRecon,
                target: TargetAsset::Domain("portal.corp.example".to_string()),
                timestamp_utc: "2026-02-02T09:05:00Z".to_string(),
                prohibited_action: None,
            },
            |_| Ok("Passive DNS & header inventory on portal.corp.example".to_string()),
        )?;

        eng.advance_phase(
            EngagementPhase::Validate,
            "2026-02-03T10:00:00Z",
            "operator",
        )?;
        eng.execute_module_step(
            &ModuleStepRequest {
                step_id: "step-val-01".to_string(),
                actor: "validator-agent".to_string(),
                method: TestingMethod::VulnerabilityValidation,
                target: TargetAsset::IpV4("10.42.0.15".to_string()),
                timestamp_utc: "2026-02-03T10:15:00Z".to_string(),
                prohibited_action: None,
            },
            |_| Ok("Validated misconfigured role binding on 10.42.0.15".to_string()),
        )?;

        eng.advance_phase(
            EngagementPhase::LabExploit,
            "2026-02-05T09:00:00Z",
            "operator",
        )?;
        eng.execute_module_step(
            &ModuleStepRequest {
                step_id: "step-exp-01".to_string(),
                actor: "exploit-agent".to_string(),
                method: TestingMethod::ControlledLabExploit,
                target: TargetAsset::Domain("api.corp.example".to_string()),
                timestamp_utc: "2026-02-05T09:10:00Z".to_string(),
                prohibited_action: None,
            },
            |_| Ok("Controlled read-only PoC proved missing authZ on api.corp.example".to_string()),
        )?;

        let ev1 = EvidenceRef::from_raw_capture(
            "EV-001",
            "2026-02-05T09:11:00Z",
            "virtual://api.corp.example/http/authz-trace",
            "GET /admin/export returned HTTP 200; Authorization: Bearer syn_tok_demo_alpha987654; operator=alice.private@personal.org",
        )?;
        let ev2 = EvidenceRef::from_raw_capture(
            "EV-002",
            "2026-02-05T09:12:00Z",
            "lab-sensor://api.corp.example/replay/dup-trace",
            "Replay from second scanner confirmed HTTP 200 on /admin/export without admin role",
        )?;
        let ev3 = EvidenceRef::from_raw_capture(
            "EV-003",
            "2026-02-06T11:00:00Z",
            "virtual://sandbox.corp.example/jenkins/script-console",
            "Simulated unauthenticated admin endpoint in isolated lab dev sandbox",
        )?;
        let ev4 = EvidenceRef::from_raw_capture(
            "EV-004",
            "2026-02-06T14:00:00Z",
            "virtual://portal.corp.example/headers/csp",
            "Missing strict Content-Security-Policy header on login page",
        )?;
        let ev5 = EvidenceRef::from_raw_capture(
            "EV-005",
            "2026-02-06T15:00:00Z",
            "virtual://portal.corp.example/scanner/banner-guess",
            "Draft heuristic noticed old library string without reproduction",
        )?;

        for ev in [&ev1, &ev2, &ev3, &ev4, &ev5] {
            eng.add_evidence(ev.clone())?;
        }

        // F-101: Critical RCE/AuthZ bypass (9.8) trên lõi định danh/API lộ ra ngoài -> P1Immediate
        eng.add_finding(Finding {
            finding_id: "F-101".to_string(),
            rule_id: "AUTHZ-ADMIN-BYPASS".to_string(),
            root_cause_key: "missing-rbac-middleware".to_string(),
            primary_component: "api.corp.example/admin".to_string(),
            title: "Thiếu kiểm tra RBAC trên endpoint quản trị công khai".to_string(),
            affected_assets: BTreeSet::from(["api.corp.example".to_string()]),
            status: FindingStatus::Verified,
            duplicate_of: None,
            merged_duplicate_ids: BTreeSet::new(),
            exploitation_claim: ExploitationClaim::ControlledLabExploited {
                lab_proof_evidence_id: "EV-001".to_string(),
            },
            cvss_vector: Cvss31Vector::parse("AV:N/AC:L/PR:N/UI:N/S:U/C:H/I:H/A:H")?,
            asset_criticality: AssetCriticality::CoreIdentityOrRevenue,
            exposure_context: ExposureContext::InternetFacing,
            business_priority: compute_business_priority(
                CvssSeverity::Critical,
                AssetCriticality::CoreIdentityOrRevenue,
                ExposureContext::InternetFacing,
            ),
            evidence_refs: vec![ev1],
            lab_reproduction:
                "Gửi yêu cầu HTTP giả lập thiếu vai trò admin tới /admin/export và nhận 200 OK."
                    .to_string(),
            impact_analysis: "Cho phép đọc/ghi cấu hình và dữ liệu nhạy cảm trên api.corp.example."
                .to_string(),
            remediation:
                "Bắt buộc middleware kiểm tra vai trò Fail-Closed và kiểm thử phân quyền tích hợp."
                    .to_string(),
            cleanup_steps: "Thu hồi token phiên lab và kiểm tra log audit trước khi đóng."
                .to_string(),
            detection_control: "Cảnh báo mọi truy cập /admin/* không kèm admin claim hợp lệ."
                .to_string(),
            retest: None,
            citation_ids: BTreeSet::from([
                "FIRST-CVSS-3.1".to_string(),
                "OWASP-WSTG-ATHZ".to_string(),
            ]),
        })?;

        // F-102: Trùng lặp với F-101 qua Duplicate tường minh -> phải gộp EV-002 vào F-101
        eng.add_finding(Finding {
            finding_id: "F-102".to_string(),
            rule_id: "AUTHZ-ADMIN-BYPASS".to_string(),
            root_cause_key: "missing-rbac-middleware".to_string(),
            primary_component: "api.corp.example/admin".to_string(),
            title: "Scanner 2 phát hiện endpoint /admin/export không xác thực vai trò".to_string(),
            affected_assets: BTreeSet::from(["10.42.0.15".to_string()]),
            status: FindingStatus::Duplicate,
            duplicate_of: Some("F-101".to_string()),
            merged_duplicate_ids: BTreeSet::new(),
            exploitation_claim: ExploitationClaim::ValidatedPreconditionOnly,
            cvss_vector: Cvss31Vector::parse("AV:N/AC:L/PR:N/UI:N/S:U/C:H/I:H/A:H")?,
            asset_criticality: AssetCriticality::CoreIdentityOrRevenue,
            exposure_context: ExposureContext::InternetFacing,
            business_priority: compute_business_priority(
                CvssSeverity::Critical,
                AssetCriticality::CoreIdentityOrRevenue,
                ExposureContext::InternetFacing,
            ),
            evidence_refs: vec![ev2],
            lab_reproduction: "Replay từ sensor xác nhận cùng lỗ hổng /admin/export.".to_string(),
            impact_analysis: "Cùng nguyên nhân gốc missing-rbac-middleware với F-101.".to_string(),
            remediation: "Áp dụng bản vá RBAC của F-101.".to_string(),
            cleanup_steps: "Không sinh artifact mới.".to_string(),
            detection_control: "Gộp quy tắc cảnh báo với F-101.".to_string(),
            retest: None,
            citation_ids: BTreeSet::from(["OWASP-WSTG-ATHZ".to_string()]),
        })?;

        // F-103: CVSS 9.8 Critical nhưng nằm trong DevSandbox + IsolatedLabOnly -> P3Scheduled!
        eng.add_finding(Finding {
            finding_id: "F-103".to_string(),
            rule_id: "DEV-SANDBOX-CONSOLE".to_string(),
            root_cause_key: "unauthenticated-dev-console".to_string(),
            primary_component: "sandbox.corp.example/console".to_string(),
            title: "Bảng điều khiển lab không khóa xác thực trong sandbox cô lập".to_string(),
            affected_assets: BTreeSet::from(["sandbox.corp.example".to_string()]),
            status: FindingStatus::Verified,
            duplicate_of: None,
            merged_duplicate_ids: BTreeSet::new(),
            exploitation_claim: ExploitationClaim::ValidatedPreconditionOnly,
            cvss_vector: Cvss31Vector::parse("AV:N/AC:L/PR:N/UI:N/S:U/C:H/I:H/A:H")?,
            asset_criticality: AssetCriticality::DevSandbox,
            exposure_context: ExposureContext::IsolatedLabOnly,
            business_priority: compute_business_priority(
                CvssSeverity::Critical,
                AssetCriticality::DevSandbox,
                ExposureContext::IsolatedLabOnly,
            ),
            evidence_refs: vec![ev3],
            lab_reproduction: "Kiểm tra trang trạng thái console giả trong VLAN sandbox cô lập."
                .to_string(),
            impact_analysis:
                "Ảnh hưởng cục bộ tới container sandbox thử nghiệm, không nối dữ liệu thật."
                    .to_string(),
            remediation: "Bật SSO hoặc hạn chế IP chỉ cho quản trị viên lab.".to_string(),
            cleanup_steps: "Đặt lại trạng thái phiên kiểm tra read-only.".to_string(),
            detection_control: "Ghi log truy cập sandbox và chặn egress.".to_string(),
            retest: None,
            citation_ids: BTreeSet::from([
                "FIRST-CVSS-3.1".to_string(),
                "NIST-800-115".to_string(),
            ]),
        })?;

        // F-104: Medium 6.1 ở tài sản Core + InternetFacing -> được nâng thành P1Immediate
        eng.add_finding(Finding {
            finding_id: "F-104".to_string(),
            rule_id: "WEB-MISSING-CSP".to_string(),
            root_cause_key: "missing-csp-header".to_string(),
            primary_component: "portal.corp.example/login".to_string(),
            title: "Thiếu chính sách Content-Security-Policy chặt trên cổng đăng nhập".to_string(),
            affected_assets: BTreeSet::from(["portal.corp.example".to_string()]),
            status: FindingStatus::Verified,
            duplicate_of: None,
            merged_duplicate_ids: BTreeSet::new(),
            exploitation_claim: ExploitationClaim::ValidatedPreconditionOnly,
            cvss_vector: Cvss31Vector::parse("CVSS:3.1/AV:N/AC:L/PR:N/UI:R/S:C/C:L/I:L/A:N")?,
            asset_criticality: AssetCriticality::CoreIdentityOrRevenue,
            exposure_context: ExposureContext::InternetFacing,
            business_priority: compute_business_priority(
                CvssSeverity::Medium,
                AssetCriticality::CoreIdentityOrRevenue,
                ExposureContext::InternetFacing,
            ),
            evidence_refs: vec![ev4],
            lab_reproduction: "Kiểm tra response header giả từ portal.corp.example thiếu CSP."
                .to_string(),
            impact_analysis:
                "Tăng bề mặt rủi ro XSS nếu xuất hiện điểm chèn DOM trên trang đăng nhập."
                    .to_string(),
            remediation: "Bổ sung header CSP với default-src 'self' và nonce ngẫu nhiên."
                .to_string(),
            cleanup_steps: "Không tạo thay đổi trạng thái.".to_string(),
            detection_control: "Theo dõi báo cáo CSP violation endpoint.".to_string(),
            retest: None,
            citation_ids: BTreeSet::from(["ISO-29147".to_string()]),
        })?;

        // F-105: Draft chưa xác minh -> bị loại khỏi danh sách Verified của báo cáo
        eng.add_finding(Finding {
            finding_id: "F-105".to_string(),
            rule_id: "BANNER-HEURISTIC".to_string(),
            root_cause_key: "unverified-banner".to_string(),
            primary_component: "portal.corp.example/static".to_string(),
            title: "Nghi vấn phiên bản thư viện từ banner (chưa xác minh)".to_string(),
            affected_assets: BTreeSet::from(["portal.corp.example".to_string()]),
            status: FindingStatus::Draft,
            duplicate_of: None,
            merged_duplicate_ids: BTreeSet::new(),
            exploitation_claim: ExploitationClaim::NotExploited,
            cvss_vector: Cvss31Vector::parse("CVSS:3.1/AV:N/AC:H/PR:N/UI:N/S:U/C:L/I:N/A:N")?,
            asset_criticality: AssetCriticality::InternalTool,
            exposure_context: ExposureContext::AuthenticatedInternal,
            business_priority: compute_business_priority(
                CvssSeverity::Low,
                AssetCriticality::InternalTool,
                ExposureContext::AuthenticatedInternal,
            ),
            evidence_refs: vec![ev5],
            lab_reproduction: "Mới ghi nhận chuỗi version banner; chưa tái hiện hành vi lỗi."
                .to_string(),
            impact_analysis: "Chưa xác nhận ảnh hưởng thực tế.".to_string(),
            remediation: "Xác minh phiên bản thực tế qua quản trị cấu hình.".to_string(),
            cleanup_steps: "Không có thay đổi.".to_string(),
            detection_control: "Kiểm kê SBOM định kỳ.".to_string(),
            retest: None,
            citation_ids: BTreeSet::from(["NIST-800-115".to_string()]),
        })?;

        eng.advance_phase(EngagementPhase::Cleanup, "2026-02-11T10:00:00Z", "operator")?;
        eng.execute_module_step(
            &ModuleStepRequest {
                step_id: "step-clean-01".to_string(),
                actor: "cleanup-verifier".to_string(),
                method: TestingMethod::CleanupVerification,
                target: TargetAsset::Domain("api.corp.example".to_string()),
                timestamp_utc: "2026-02-11T10:05:00Z".to_string(),
                prohibited_action: None,
            },
            |_| {
                Ok(
                    "Verified baseline clean on api.corp.example and portal.corp.example"
                        .to_string(),
                )
            },
        )?;
        eng.record_cleanup_attestation(CleanupAttestation {
            plan_id: "CLEANUP-111-FINAL".to_string(),
            verified_clean: true,
            remaining_artifacts: 0,
            attested_by_role: "Post-Audit Verifier".to_string(),
            attested_at_utc: "2026-02-11T10:10:00Z".to_string(),
        })?;
        eng.advance_phase(EngagementPhase::Report, REPORT_GENERATED_UTC, "operator")?;
        Ok(eng)
    }
}

fn main() -> Result<(), Box<dyn std::error::Error>> {
    println!(
        "MockReportingLab — 100% synthetic governance, evidence chain, CVSS 3.1 & deterministic reporting"
    );

    let mut eng = MockReportingLab::standard_engagement()?;
    let mut blocked_invocations = 0usize;
    let out_of_scope = eng.execute_module_step(
        &ModuleStepRequest {
            step_id: "step-oos".to_string(),
            actor: "recon-agent".to_string(),
            method: TestingMethod::PassiveRecon,
            target: TargetAsset::Domain("payroll.corp.example".to_string()),
            timestamp_utc: "2026-02-12T16:31:00Z".to_string(),
            prohibited_action: None,
        },
        |_| {
            blocked_invocations += 1;
            Ok("never executed".to_string())
        },
    );
    println!(
        "Out-of-scope gate: blocked={}; module_invocations={}",
        out_of_scope.is_err(),
        blocked_invocations
    );

    let cvss_std = Cvss31Vector::parse("AV:N/AC:L/PR:N/UI:N/S:U/C:H/I:H/A:H")?.score();
    let cvss_changed = Cvss31Vector::parse("CVSS:3.1/AV:N/AC:L/PR:H/UI:N/S:C/C:H/I:H/A:H")?.score();
    println!(
        "CVSS 3.1 check: std={} ({}) | scope_changed_high_pr={} ({})",
        cvss_std.score_string(),
        cvss_std.severity.as_str(),
        cvss_changed.score_string(),
        cvss_changed.severity.as_str()
    );

    let report = eng.build_report(
        REPORT_GENERATED_UTC,
        "Báo cáo kiểm thử phòng thí nghiệm ACME",
    )?;
    let renderer = ReportRenderer;
    let json_a = renderer.render_json(&report)?;
    let json_b = renderer.render_json(&report)?;
    let md_exec = renderer.render_executive_markdown(&report)?;
    let f101 = report
        .findings
        .iter()
        .find(|f| f.finding_id == "F-101")
        .expect("F-101");

    println!(
        "Report verified={} excluded={} f101_evidence={} f101_priority={} f103_priority={} replay_identical={}",
        report.executive_summary.total_verified_findings,
        report.executive_summary.excluded_draft_or_false_positive,
        f101.evidence_refs.len(),
        f101.business_priority.as_str(),
        report
            .findings
            .iter()
            .find(|f| f.finding_id == "F-103")
            .expect("F-103")
            .business_priority
            .as_str(),
        json_a == json_b && !md_exec.is_empty()
    );

    eng.advance_phase(EngagementPhase::Retest, "2026-02-13T09:50:00Z", "operator")?;
    let rt_ev = EvidenceRef::from_raw_capture(
        "EV-RETEST-101",
        RETEST_TIME_UTC,
        "virtual://api.corp.example/http/retest-101",
        "Retest confirmed HTTP 403 Forbidden without admin claim",
    )?;
    eng.apply_retest(
        "F-101",
        rt_ev,
        RetestOutcome::FixedVerified,
        "Đã xác minh middleware Fail-Closed chặn truy cập trái phép",
        "verifier",
    )?;
    let retest_report = eng.build_report("2026-02-13T10:30:00Z", "Báo cáo sau tái kiểm thử")?;
    let digest_short = retest_report
        .input_digest_sha256
        .get(..16)
        .unwrap_or(&retest_report.input_digest_sha256);
    println!(
        "Retest verified fixed={} digest_prefix={}",
        retest_report.executive_summary.retested_fixed_count, digest_short
    );
    Ok(())
}

// ============================================================================
// 8. UNIT TESTS (TOÀN DIỆN & TẤT ĐỊNH)
// ============================================================================

#[cfg(test)]
mod tests {
    use super::*;

    fn mk_evidence(id: &str, ts: &str, suffix: &str) -> EvidenceRef {
        EvidenceRef::from_raw_capture(
            id,
            ts,
            &format!("virtual://api.corp.example/tests/{suffix}"),
            &format!("synthetic test evidence {id} ({suffix})"),
        )
        .expect("valid evidence")
    }

    fn mk_verified_finding(id: &str, ev: EvidenceRef) -> Finding {
        Finding {
            finding_id: id.to_string(),
            rule_id: format!("RULE-{id}"),
            root_cause_key: format!("cause-{id}"),
            primary_component: format!("api.corp.example/{id}"),
            title: format!("Verified synthetic issue {id}"),
            affected_assets: BTreeSet::from(["api.corp.example".to_string()]),
            status: FindingStatus::Verified,
            duplicate_of: None,
            merged_duplicate_ids: BTreeSet::new(),
            exploitation_claim: ExploitationClaim::ValidatedPreconditionOnly,
            cvss_vector: Cvss31Vector::parse("AV:N/AC:L/PR:N/UI:N/S:U/C:H/I:H/A:H").unwrap(),
            asset_criticality: AssetCriticality::CoreIdentityOrRevenue,
            exposure_context: ExposureContext::InternetFacing,
            business_priority: BusinessPriority::P1Immediate,
            evidence_refs: vec![ev],
            lab_reproduction: "Lab HTTP trace".to_string(),
            impact_analysis: "Lab impact".to_string(),
            remediation: "Apply guard".to_string(),
            cleanup_steps: "Reset mock".to_string(),
            detection_control: "Log alert".to_string(),
            retest: None,
            citation_ids: BTreeSet::from(["FIRST-CVSS-3.1".to_string()]),
        }
    }

    #[test]
    fn test_sha256_nist_vectors() {
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
    fn test_domain_ipv4_cidr_and_iso_validators() {
        assert_eq!(
            normalize_example_domain(" API.Corp.Example. ").unwrap(),
            "api.corp.example"
        );
        assert!(normalize_example_domain("corp.com").is_err());
        assert!(normalize_example_domain("example").is_err());
        assert!(normalize_example_domain("-bad.corp.example").is_err());

        assert!(is_rfc1918_ipv4("10.0.0.1"));
        assert!(is_rfc1918_ipv4("172.16.5.9"));
        assert!(is_rfc1918_ipv4("172.31.255.254"));
        assert!(is_rfc1918_ipv4("192.168.1.44"));
        assert!(!is_rfc1918_ipv4("010.0.0.1"));
        assert!(!is_rfc1918_ipv4("172.15.0.1"));
        assert!(!is_rfc1918_ipv4("172.32.0.1"));
        assert!(!is_rfc1918_ipv4("8.8.8.8"));

        assert_eq!(
            parse_rfc1918_cidr("10.42.0.0/24").unwrap(),
            (0x0a2a_0000, 24)
        );
        assert!(parse_rfc1918_cidr("10.42.0.1/24").is_err());
        assert!(parse_rfc1918_cidr("192.0.2.0/24").is_err());
        assert!(parse_rfc1918_cidr("172.16.0.0/11").is_err());

        assert!(is_valid_iso8601_utc("2024-02-29T23:59:59Z"));
        assert!(!is_valid_iso8601_utc("2026-02-29T12:00:00Z"));
        assert!(!is_valid_iso8601_utc("1969-12-31T23:59:59Z"));
        assert!(!is_valid_iso8601_utc("2026-02-01T00:00:00+07:00"));
    }

    #[test]
    fn test_synthetic_source_isolation() {
        assert_eq!(
            normalize_synthetic_source("virtual://API.corp.example/http/req1").unwrap(),
            "virtual://api.corp.example/http/req1"
        );
        assert!(normalize_synthetic_source("/etc/passwd").is_err());
        assert!(normalize_synthetic_source(r"C:\Windows\System32\cmd.exe").is_err());
        assert!(normalize_synthetic_source("virtual://corp.example/../secret").is_err());
        assert!(normalize_synthetic_source("virtual://corp.example/a/b;rm").is_err());
        assert!(normalize_synthetic_source("virtual://corp.com/a/b").is_err());
    }

    #[test]
    fn test_cvss31_official_vectors_and_appendix_a_roundup() {
        let cases = [
            (
                "AV:N/AC:L/PR:N/UI:N/S:U/C:H/I:H/A:H",
                98u8,
                CvssSeverity::Critical,
            ),
            (
                "CVSS:3.1/AV:N/AC:L/PR:N/UI:N/S:C/C:H/I:H/A:H",
                100,
                CvssSeverity::Critical,
            ),
            (
                "CVSS:3.1/AV:N/AC:L/PR:N/UI:R/S:C/C:L/I:L/A:N",
                61,
                CvssSeverity::Medium,
            ),
            (
                "CVSS:3.1/AV:L/AC:L/PR:N/UI:R/S:U/C:H/I:H/A:H",
                78,
                CvssSeverity::High,
            ),
            (
                "CVSS:3.1/AV:L/AC:H/PR:L/UI:N/S:U/C:H/I:H/A:H",
                70,
                CvssSeverity::High,
            ),
            (
                "CVSS:3.1/AV:N/AC:L/PR:L/UI:N/S:C/C:H/I:H/A:H",
                99,
                CvssSeverity::Critical,
            ),
            (
                "CVSS:3.1/AV:N/AC:L/PR:H/UI:N/S:C/C:H/I:H/A:H",
                91,
                CvssSeverity::Critical,
            ),
            (
                "CVSS:3.1/AV:N/AC:L/PR:H/UI:N/S:U/C:H/I:H/A:H",
                72,
                CvssSeverity::High,
            ),
            (
                "CVSS:3.1/AV:N/AC:L/PR:N/UI:N/S:U/C:N/I:N/A:N",
                0,
                CvssSeverity::None,
            ),
            (
                "CVSS:3.1/AV:P/AC:H/PR:H/UI:R/S:U/C:L/I:N/A:N",
                16,
                CvssSeverity::Low,
            ),
            (
                "CVSS:3.1/AV:A/AC:L/PR:N/UI:N/S:U/C:H/I:N/A:N",
                65,
                CvssSeverity::Medium,
            ),
            (
                "CVSS:3.1/AV:L/AC:L/PR:L/UI:N/S:C/C:H/I:H/A:H",
                88,
                CvssSeverity::High,
            ),
        ];
        for (vec_str, expected_tenths, expected_sev) in cases {
            let parsed = Cvss31Vector::parse(vec_str).expect(vec_str);
            let sc = parsed.score();
            assert_eq!(
                sc.score_tenths, expected_tenths,
                "mismatch for vector {vec_str}"
            );
            assert_eq!(sc.severity, expected_sev, "severity mismatch for {vec_str}");
        }

        // Appendix A floating-point stability checks
        assert_eq!(roundup_cvss31_tenths(0.1 + 0.2), 3);
        assert_eq!(roundup_cvss31_tenths(4.00), 40);
        assert_eq!(roundup_cvss31_tenths(4.02), 41);
        assert_eq!(roundup_cvss31_tenths(0.0), 0);
        assert_eq!(roundup_cvss31_tenths(-1.0), 0);
        assert_eq!(roundup_cvss31_tenths(10.8), 100);
    }

    #[test]
    fn test_cvss31_parser_rejects_malformed_and_wrong_version() {
        assert!(matches!(
            Cvss31Vector::parse("CVSS:3.0/AV:N/AC:L/PR:N/UI:N/S:U/C:H/I:H/A:H"),
            Err(ReportingError::InvalidCvssVector(_))
        ));
        assert!(matches!(
            Cvss31Vector::parse("AC:L/AV:N/PR:N/UI:N/S:U/C:H/I:H/A:H"),
            Err(ReportingError::InvalidCvssVector(_))
        ));
        assert!(matches!(
            Cvss31Vector::parse("CVSS:3.1/AV:N/AC:L/PR:N/UI:N/S:U/C:H/I:H"),
            Err(ReportingError::InvalidCvssVector(_))
        ));
        assert!(matches!(
            Cvss31Vector::parse("CVSS:3.1/AV:X/AC:L/PR:N/UI:N/S:U/C:H/I:H/A:H"),
            Err(ReportingError::InvalidCvssVector(_))
        ));
    }

    #[test]
    fn test_severity_vs_business_priority_separation() {
        assert_eq!(
            compute_business_priority(
                CvssSeverity::Critical,
                AssetCriticality::DevSandbox,
                ExposureContext::IsolatedLabOnly
            ),
            BusinessPriority::P3Scheduled
        );
        assert_eq!(
            compute_business_priority(
                CvssSeverity::Medium,
                AssetCriticality::CoreIdentityOrRevenue,
                ExposureContext::InternetFacing
            ),
            BusinessPriority::P1Immediate
        );
        assert_eq!(
            compute_business_priority(
                CvssSeverity::None,
                AssetCriticality::CoreIdentityOrRevenue,
                ExposureContext::InternetFacing
            ),
            BusinessPriority::P4Backlog
        );
    }

    #[test]
    fn test_scope_allow_deny_and_third_party_consent() {
        let eng = MockReportingLab::standard_engagement().unwrap();
        assert!(
            eng.scope
                .check_target(
                    &TargetAsset::Domain("auth.corp.example".to_string()),
                    "2026-02-05T10:00:00Z"
                )
                .is_ok()
        );
        // Deny overrides wildcard allow_subdomains_of("corp.example")
        assert!(matches!(
            eng.scope.check_target(
                &TargetAsset::Domain("payroll.corp.example".to_string()),
                "2026-02-05T10:00:00Z"
            ),
            Err(ReportingError::OutOfScope(_))
        ));
        assert!(matches!(
            eng.scope.check_target(
                &TargetAsset::Domain("sub.prod-billing.corp.example".to_string()),
                "2026-02-05T10:00:00Z"
            ),
            Err(ReportingError::OutOfScope(_))
        ));
        // Third-party valid consent vs expired consent vs missing consent
        assert!(
            eng.scope
                .check_target(
                    &TargetAsset::Domain("cdn-auth.vendor.example".to_string()),
                    "2026-02-10T00:00:00Z"
                )
                .is_ok()
        );
        assert!(matches!(
            eng.scope.check_target(
                &TargetAsset::Domain("cdn-auth.vendor.example".to_string()),
                "2026-02-21T00:00:00Z"
            ),
            Err(ReportingError::ThirdPartyConsentMissing(_))
        ));
        assert!(matches!(
            eng.scope.check_target(
                &TargetAsset::Domain("legacy-sso.vendor.example".to_string()),
                "2026-02-05T10:00:00Z"
            ),
            Err(ReportingError::ThirdPartyConsentMissing(_))
        ));
        // IPv4 scope checks
        assert!(
            eng.scope
                .check_target(
                    &TargetAsset::IpV4("10.42.0.50".to_string()),
                    "2026-02-05T10:00:00Z"
                )
                .is_ok()
        );
        assert!(matches!(
            eng.scope.check_target(
                &TargetAsset::IpV4("10.42.0.254".to_string()),
                "2026-02-05T10:00:00Z"
            ),
            Err(ReportingError::OutOfScope(_))
        ));
        assert!(matches!(
            eng.scope.check_target(
                &TargetAsset::IpV4("203.0.113.9".to_string()),
                "2026-02-05T10:00:00Z"
            ),
            Err(ReportingError::OutOfScope(_))
        ));
    }

    #[test]
    fn test_pre_execution_gates_never_invoke_module_on_rejection() {
        let base = MockReportingLab::standard_engagement().unwrap();
        let mut eng_oos = base.clone();
        eng_oos.phase = EngagementPhase::Recon;
        let mut calls = 0usize;
        let err = eng_oos.execute_module_step(
            &ModuleStepRequest {
                step_id: "oos".to_string(),
                actor: "agent".to_string(),
                method: TestingMethod::PassiveRecon,
                target: TargetAsset::Domain("evil.external.example".to_string()),
                timestamp_utc: "2026-02-05T12:00:00Z".to_string(),
                prohibited_action: None,
            },
            |_| {
                calls += 1;
                Ok("must not run".to_string())
            },
        );
        assert!(matches!(err, Err(ReportingError::OutOfScope(_))));
        assert_eq!(calls, 0);

        // Missing human approval blocks ControlledLabExploit
        let mut eng_no_apr = base.clone();
        eng_no_apr.phase = EngagementPhase::LabExploit;
        let err_apr = eng_no_apr.execute_module_step(
            &ModuleStepRequest {
                step_id: "no-apr".to_string(),
                actor: "agent".to_string(),
                method: TestingMethod::ControlledLabExploit,
                target: TargetAsset::Domain("portal.corp.example".to_string()),
                timestamp_utc: "2026-02-05T12:00:00Z".to_string(),
                prohibited_action: None,
            },
            |_| {
                calls += 1;
                Ok("must not run".to_string())
            },
        );
        assert!(matches!(
            err_apr,
            Err(ReportingError::HumanApprovalRequired(_))
        ));
        assert_eq!(calls, 0);

        // Outside engagement window
        let mut eng_win = base.clone();
        eng_win.phase = EngagementPhase::Recon;
        let err_win = eng_win.execute_module_step(
            &ModuleStepRequest {
                step_id: "late".to_string(),
                actor: "agent".to_string(),
                method: TestingMethod::PassiveRecon,
                target: TargetAsset::Domain("portal.corp.example".to_string()),
                timestamp_utc: "2026-02-28T00:00:00Z".to_string(),
                prohibited_action: None,
            },
            |_| {
                calls += 1;
                Ok("must not run".to_string())
            },
        );
        assert!(matches!(
            err_win,
            Err(ReportingError::OutsideEngagementWindow(_))
        ));
        assert_eq!(calls, 0);

        // Prohibited action blocked
        let mut eng_proh = base.clone();
        eng_proh.phase = EngagementPhase::Recon;
        let err_proh = eng_proh.execute_module_step(
            &ModuleStepRequest {
                step_id: "dos".to_string(),
                actor: "agent".to_string(),
                method: TestingMethod::PassiveRecon,
                target: TargetAsset::Domain("portal.corp.example".to_string()),
                timestamp_utc: "2026-02-05T12:00:00Z".to_string(),
                prohibited_action: Some(ProhibitedAction::DenialOfService),
            },
            |_| {
                calls += 1;
                Ok("must not run".to_string())
            },
        );
        assert!(matches!(
            err_proh,
            Err(ReportingError::ProhibitedActionBlocked(
                ProhibitedAction::DenialOfService
            ))
        ));
        assert_eq!(calls, 0);
    }

    #[test]
    fn test_stop_condition_and_audit_log_tamper_detection() {
        let mut eng = MockReportingLab::standard_engagement().unwrap();
        eng.trigger_stop_condition(
            StopConditionTrigger::EmergencyClientAbort,
            "2026-02-12T17:00:00Z",
            "client-ciso",
            "Khách hàng yêu cầu dừng khẩn cấp",
        )
        .unwrap();
        assert!(matches!(
            eng.advance_phase(EngagementPhase::Retest, "2026-02-12T17:01:00Z", "operator"),
            Err(ReportingError::StopConditionTriggered(
                StopConditionTrigger::EmergencyClientAbort
            ))
        ));
        eng.verify_audit_log().unwrap();

        eng.audit_log[1].detail = "tampered log text".to_string();
        assert!(matches!(
            eng.verify_audit_log(),
            Err(ReportingError::AuditLogTampered(_))
        ));
    }

    #[test]
    fn test_cleanup_gate_blocks_report_until_verified_clean() {
        let mut eng = MockReportingLab::standard_engagement().unwrap();
        eng.phase = EngagementPhase::Cleanup;
        eng.cleanup_attestation = None;
        assert!(matches!(
            eng.advance_phase(EngagementPhase::Report, "2026-02-12T00:00:00Z", "operator"),
            Err(ReportingError::CleanupNotVerified(_))
        ));

        eng.record_cleanup_attestation(CleanupAttestation {
            plan_id: "C1".to_string(),
            verified_clean: true,
            remaining_artifacts: 1,
            attested_by_role: "Verifier".to_string(),
            attested_at_utc: "2026-02-11T10:10:00Z".to_string(),
        })
        .unwrap();
        assert!(matches!(
            eng.advance_phase(EngagementPhase::Report, "2026-02-12T00:00:00Z", "operator"),
            Err(ReportingError::CleanupNotVerified(_))
        ));
    }

    #[test]
    fn test_evidence_redaction_and_raw_secret_rejection() {
        let ev = EvidenceRef::from_raw_capture(
            "EV-REDACT",
            "2026-02-05T09:00:00Z",
            "virtual://api.corp.example/auth/trace",
            "Authorization: Bearer syn_tok_super_secret_123 password=LabPassword123 user=john.doe@external-personal.org contact=sec@corp.example",
        )
        .unwrap();
        assert_eq!(ev.redaction_count, 3);
        assert!(!ev.redacted_excerpt.contains("syn_tok_super_secret_123"));
        assert!(!ev.redacted_excerpt.contains("LabPassword123"));
        assert!(!ev.redacted_excerpt.contains("external-personal.org"));
        assert!(ev.redacted_excerpt.contains("sec@corp.example"));
        assert!(ev.redacted_excerpt.contains(REDACTED_SECRET_MARKER));
        assert!(ev.redacted_excerpt.contains(REDACTED_PII_MARKER));

        assert!(matches!(
            validate_no_raw_secret("test", "api_key=syn_tok_leaked_key_999"),
            Err(ReportingError::RawSecretProhibited(_))
        ));
        assert!(matches!(
            validate_no_raw_secret("test", "-----BEGIN PRIVATE KEY-----"),
            Err(ReportingError::RawSecretProhibited(_))
        ));
    }

    #[test]
    fn test_evidence_chain_integrity_and_tamper_rejection() {
        let mut chain = EvidenceChain::new();
        chain
            .append(mk_evidence("EV-1", "2026-02-05T08:00:00Z", "one"))
            .unwrap();
        chain
            .append(mk_evidence("EV-2", "2026-02-05T09:00:00Z", "two"))
            .unwrap();
        chain.verify_integrity().unwrap();

        // Reject duplicate id & monotonic clock violation
        assert!(matches!(
            chain.append(mk_evidence("EV-2", "2026-02-05T09:30:00Z", "dup")),
            Err(ReportingError::DuplicateIdentifier(_))
        ));
        assert!(matches!(
            chain.append(mk_evidence("EV-3", "2026-02-05T07:00:00Z", "past")),
            Err(ReportingError::InvalidTimestamp(_))
        ));

        chain.links_mut_for_tamper_test()[0]
            .evidence
            .redacted_excerpt = "tampered".to_string();
        chain.links_mut_for_tamper_test()[0]
            .evidence
            .artifact_sha256 = sha256_hex(b"tampered");
        assert!(matches!(
            chain.verify_integrity(),
            Err(ReportingError::EvidenceInvalid(_))
        ));
    }

    #[test]
    fn test_unverified_finding_cannot_claim_exploited() {
        let ev = mk_evidence("EV-1", "2026-02-05T08:00:00Z", "one");
        let mut chain = EvidenceChain::new();
        chain.append(ev.clone()).unwrap();

        let mut f = mk_verified_finding("F-DRAFT", ev);
        f.status = FindingStatus::Draft;
        assert!(matches!(
            f.set_exploitation_claim(ExploitationClaim::ControlledLabExploited {
                lab_proof_evidence_id: "EV-1".to_string()
            }),
            Err(ReportingError::UnverifiedExploitationClaim(_))
        ));
        assert!(matches!(
            f.transition_status(FindingStatus::Remediated),
            Err(ReportingError::IllegalFindingTransition { .. })
        ));

        let mut f_fp = f.clone();
        f_fp.status = FindingStatus::FalsePositive;
        f_fp.exploitation_claim = ExploitationClaim::ControlledLabExploited {
            lab_proof_evidence_id: "EV-1".to_string(),
        };
        assert!(matches!(
            f_fp.validate(&chain),
            Err(ReportingError::UnverifiedExploitationClaim(_))
        ));
    }

    #[test]
    fn test_deduplication_is_permutation_invariant_and_preserves_all_evidence() {
        let ev1 = mk_evidence("EV-A", "2026-02-05T08:00:00Z", "a");
        let ev2 = mk_evidence("EV-B", "2026-02-05T08:01:00Z", "b");
        let ev3 = mk_evidence("EV-C", "2026-02-05T08:02:00Z", "c");
        let mut chain = EvidenceChain::new();
        for ev in [&ev1, &ev2, &ev3] {
            chain.append(ev.clone()).unwrap();
        }

        let f1 = mk_verified_finding("F-01", ev1);
        let mut f2 = mk_verified_finding("F-02", ev2);
        f2.rule_id = f1.rule_id.clone();
        f2.root_cause_key = f1.root_cause_key.clone();
        f2.primary_component = f1.primary_component.clone();
        f2.affected_assets = BTreeSet::from(["10.42.0.15".to_string()]);
        f2.status = FindingStatus::Verified;

        let mut f3 = mk_verified_finding("F-03", ev3);
        f3.rule_id = f1.rule_id.clone();
        f3.root_cause_key = f1.root_cause_key.clone();
        f3.primary_component = f1.primary_component.clone();
        f3.status = FindingStatus::Duplicate;
        f3.duplicate_of = Some("F-01".to_string());

        let perms = [
            vec![f1.clone(), f2.clone(), f3.clone()],
            vec![f3.clone(), f2.clone(), f1.clone()],
            vec![f2.clone(), f3.clone(), f1.clone()],
            vec![f3.clone(), f1.clone(), f2.clone()],
        ];
        let baseline = deduplicate_findings(&perms[0], &chain).unwrap();
        assert_eq!(baseline.canonical_findings.len(), 1);
        assert_eq!(baseline.canonical_findings[0].finding_id, "F-01");
        assert_eq!(baseline.canonical_findings[0].evidence_refs.len(), 3);
        assert_eq!(
            baseline.canonical_findings[0].merged_duplicate_ids,
            BTreeSet::from(["F-02".to_string(), "F-03".to_string()])
        );
        for p in &perms[1..] {
            let res = deduplicate_findings(p, &chain).unwrap();
            assert_eq!(res, baseline);
        }
    }

    #[test]
    fn test_deduplication_rejects_conflicting_evidence_digest_and_cycles() {
        let ev1 = mk_evidence("EV-A", "2026-02-05T08:00:00Z", "a");
        let mut chain = EvidenceChain::new();
        chain.append(ev1.clone()).unwrap();
        let mut f_self = mk_verified_finding("F-SELF", ev1);
        f_self.status = FindingStatus::Duplicate;
        f_self.duplicate_of = Some("F-SELF".to_string());
        assert!(matches!(
            deduplicate_findings(&[f_self], &chain),
            Err(ReportingError::DeduplicationConflict(_))
        ));
    }

    #[test]
    fn test_retest_lifecycle_and_retention_evaluation() {
        let mut eng = MockReportingLab::standard_engagement().unwrap();
        assert_eq!(
            eng.evaluate_retention("2026-02-10T00:00:00Z").unwrap(),
            RetentionDisposition::ActiveWindow
        );
        assert_eq!(
            eng.evaluate_retention("2026-03-01T00:00:00Z").unwrap(),
            RetentionDisposition::WithinRetentionPeriod
        );
        assert_eq!(
            eng.evaluate_retention("2026-06-01T00:00:00Z").unwrap(),
            RetentionDisposition::DeletionRequiredNow
        );

        eng.advance_phase(EngagementPhase::Retest, "2026-02-13T09:00:00Z", "operator")
            .unwrap();
        let too_early = mk_evidence("EV-EARLY", "2026-02-01T00:00:00Z", "early");
        assert!(matches!(
            eng.apply_retest(
                "F-101",
                too_early,
                RetestOutcome::FixedVerified,
                "backdated",
                "verifier"
            ),
            Err(ReportingError::InvalidTimestamp(_))
        ));

        let rt_partial = mk_evidence("EV-RT-104", "2026-02-13T10:15:00Z", "partial");
        let residual = Cvss31Vector::parse("CVSS:3.1/AV:N/AC:H/PR:L/UI:R/S:U/C:L/I:N/A:N").unwrap();
        eng.apply_retest(
            "F-104",
            rt_partial,
            RetestOutcome::PartiallyMitigated {
                residual_vector: residual,
            },
            "CSP deployed in report-only mode; residual XSS scope mitigated",
            "verifier",
        )
        .unwrap();
        let f104 = eng
            .findings
            .iter()
            .find(|f| f.finding_id == "F-104")
            .unwrap();
        assert_eq!(f104.status, FindingStatus::Retested);
        assert!(f104.status.is_open_risk(f104.retest.as_ref()));
    }

    #[test]
    fn test_report_determinism_schema_version_and_escaping() {
        let eng = MockReportingLab::standard_engagement().unwrap();
        let report = eng
            .build_report(REPORT_GENERATED_UTC, "Báo cáo chuẩn | <lab> & `test`")
            .unwrap();
        let renderer = ReportRenderer;

        let j1 = renderer.render_json(&report).unwrap();
        let j2 = renderer.render_json(&report).unwrap();
        let m1 = renderer.render_markdown(&report).unwrap();
        let m2 = renderer.render_markdown(&report).unwrap();
        assert_eq!(j1, j2);
        assert_eq!(m1, m2);
        assert!(j1.contains("\"schema_version\": \"1.0.0\""));
        assert!(m1.contains(r"\| \<lab\> \& \`test\`"));

        // Tamper schema_version -> rejected before render
        let mut bad_schema = report.clone();
        bad_schema.schema_version = String::new();
        assert!(matches!(
            renderer.render_json(&bad_schema),
            Err(ReportingError::ReportSchemaInvalid(_))
        ));

        // Inject raw secret into narrative -> rejected before render
        let mut secret_leak = report;
        secret_leak
            .executive_narrative
            .push_str(" Bearer syn_tok_leaked_abc12345");
        assert!(matches!(
            renderer.render_markdown(&secret_leak),
            Err(ReportingError::RawSecretProhibited(_))
        ));
    }

    #[test]
    fn test_roe_rejects_missing_written_consent_or_personal_email() {
        let mut eng = MockReportingLab::standard_engagement().unwrap();
        eng.roe.signed_written_consent = false;
        assert!(matches!(
            eng.roe.validate(),
            Err(ReportingError::AuthorizationMissing(_))
        ));
        eng.roe.signed_written_consent = true;
        eng.roe.authorized_operator_contact = "alice@gmail.com".to_string();
        assert!(matches!(
            eng.roe.validate(),
            Err(ReportingError::RawSecretProhibited(_))
        ));
    }

    #[test]
    fn test_roe_rejects_bad_window_empty_methods_and_retention_bounds() {
        let mut eng = MockReportingLab::standard_engagement().unwrap();
        eng.roe.window_end_utc = eng.roe.window_start_utc.clone();
        assert!(matches!(
            eng.roe.validate(),
            Err(ReportingError::OutsideEngagementWindow(_))
        ));

        let mut eng2 = MockReportingLab::standard_engagement().unwrap();
        eng2.roe.allowed_methods.clear();
        assert!(matches!(
            eng2.roe.validate(),
            Err(ReportingError::AuthorizationMissing(_))
        ));

        let mut eng3 = MockReportingLab::standard_engagement().unwrap();
        eng3.roe.data_handling.retention_days = 0;
        assert!(matches!(
            eng3.roe.validate(),
            Err(ReportingError::RetentionViolation(_))
        ));

        let mut eng4 = MockReportingLab::standard_engagement().unwrap();
        eng4.roe.data_handling.retention_days = 366;
        assert!(matches!(
            eng4.roe.validate(),
            Err(ReportingError::RetentionViolation(_))
        ));

        let mut eng5 = MockReportingLab::standard_engagement().unwrap();
        eng5.roe.data_handling.data_minimization_enforced = false;
        assert!(matches!(
            eng5.roe.validate(),
            Err(ReportingError::RetentionViolation(_))
        ));
    }

    #[test]
    fn test_scope_validation_rejects_empty_allowlist_and_mismatched_third_party() {
        let mut eng = MockReportingLab::standard_engagement().unwrap();
        eng.scope.allowed_domains.clear();
        eng.scope.allow_subdomains_of.clear();
        eng.scope.allowed_cidrs.clear();
        assert!(matches!(
            eng.scope.validate(),
            Err(ReportingError::OutOfScope(_))
        ));

        let mut eng2 = MockReportingLab::standard_engagement().unwrap();
        eng2.scope.third_party_assets.insert(
            "cdn-auth.vendor.example".to_string(),
            ThirdPartyAuthorization {
                host: "other.vendor.example".to_string(),
                owner_org: "Vendor".to_string(),
                written_consent_ref: "REF-1".to_string(),
                approved_until_utc: "2026-02-20T00:00:00Z".to_string(),
            },
        );
        assert!(matches!(
            eng2.scope.validate(),
            Err(ReportingError::ThirdPartyConsentMissing(_))
        ));

        let mut eng3 = MockReportingLab::standard_engagement().unwrap();
        eng3.scope.third_party_assets.insert(
            "cdn-auth.vendor.example".to_string(),
            ThirdPartyAuthorization {
                host: "cdn-auth.vendor.example".to_string(),
                owner_org: "Vendor".to_string(),
                written_consent_ref: "   ".to_string(),
                approved_until_utc: "2026-02-20T00:00:00Z".to_string(),
            },
        );
        assert!(matches!(
            eng3.scope.validate(),
            Err(ReportingError::ThirdPartyConsentMissing(_))
        ));
    }

    #[test]
    fn test_phase_transitions_and_method_gating() {
        let eng = MockReportingLab::standard_engagement().unwrap();
        assert!(EngagementPhase::Prepare.is_valid_transition(EngagementPhase::Recon));
        assert!(!EngagementPhase::Prepare.is_valid_transition(EngagementPhase::Report));
        assert!(!EngagementPhase::Report.is_valid_transition(EngagementPhase::LabExploit));

        let mut wrong_phase = eng;
        wrong_phase.phase = EngagementPhase::Recon;
        let mut calls = 0usize;
        let err = wrong_phase.execute_module_step(
            &ModuleStepRequest {
                step_id: "wrong-phase".to_string(),
                actor: "agent".to_string(),
                method: TestingMethod::ControlledLabExploit,
                target: TargetAsset::Domain("api.corp.example".to_string()),
                timestamp_utc: "2026-02-05T10:00:00Z".to_string(),
                prohibited_action: None,
            },
            |_| {
                calls += 1;
                Ok("no".to_string())
            },
        );
        assert!(matches!(err, Err(ReportingError::MethodNotAuthorized(_))));
        assert_eq!(calls, 0);
    }

    #[test]
    fn test_expired_human_approval_blocks_controlled_exploit() {
        let mut eng = MockReportingLab::standard_engagement().unwrap();
        eng.phase = EngagementPhase::LabExploit;
        let mut calls = 0usize;
        let err = eng.execute_module_step(
            &ModuleStepRequest {
                step_id: "expired-apr".to_string(),
                actor: "exploit-agent".to_string(),
                method: TestingMethod::ControlledLabExploit,
                target: TargetAsset::Domain("api.corp.example".to_string()),
                timestamp_utc: "2026-02-08T10:00:00Z".to_string(),
                prohibited_action: None,
            },
            |_| {
                calls += 1;
                Ok("no".to_string())
            },
        );
        assert!(matches!(err, Err(ReportingError::HumanApprovalRequired(_))));
        assert_eq!(calls, 0);
    }

    #[test]
    fn test_redact_sensitive_text_quoted_pem_jwt_and_prefixes() {
        let (clean_quoted, cnt1) = redact_sensitive_text(
            "config password=\"MySecretPass99\" and api_key='ghp_AbCdEf123456'",
        );
        assert_eq!(cnt1, 2);
        assert!(!clean_quoted.contains("MySecretPass99"));
        assert!(!clean_quoted.contains("ghp_AbCdEf123456"));
        validate_no_raw_secret("clean_quoted", &clean_quoted).unwrap();

        let (clean_pem, cnt2) = redact_sensitive_text(
            "key leak: -----BEGIN RSA PRIVATE KEY-----\nSYNTHETIC_PEM_BODY\n-----END RSA PRIVATE KEY-----",
        );
        assert_eq!(cnt2, 1);
        validate_no_raw_secret("clean_pem", &clean_pem).unwrap();

        let (clean_jwt, cnt3) =
            redact_sensitive_text("cookie=eyJhbGciOiJIUzI1NiJ9.eyJzdWIiOiIxMjMifQ.syntheticSigXYZ");
        assert_eq!(cnt3, 1);
        validate_no_raw_secret("clean_jwt", &clean_jwt).unwrap();
    }

    #[test]
    fn test_finding_status_transition_matrix() {
        let ev = mk_evidence("EV-M1", "2026-02-05T08:00:00Z", "m1");
        let mut chain = EvidenceChain::new();
        chain.append(ev.clone()).unwrap();

        let mut f = mk_verified_finding("F-MAT", ev);
        f.transition_status(FindingStatus::AcceptedRisk).unwrap();
        f.transition_status(FindingStatus::Remediated).unwrap();
        f.transition_status(FindingStatus::Retested).unwrap();
        assert!(matches!(
            f.transition_status(FindingStatus::Draft),
            Err(ReportingError::IllegalFindingTransition { .. })
        ));

        let mut fp =
            mk_verified_finding("F-FP", mk_evidence("EV-M2", "2026-02-05T08:05:00Z", "m2"));
        fp.status = FindingStatus::Draft;
        fp.transition_status(FindingStatus::FalsePositive).unwrap();
        assert!(matches!(
            fp.transition_status(FindingStatus::Verified),
            Err(ReportingError::IllegalFindingTransition { .. })
        ));
    }

    #[test]
    fn test_deduplication_rejects_chains_draft_targets_and_evidence_conflicts() {
        let ev1 = mk_evidence("EV-D1", "2026-02-05T08:00:00Z", "d1");
        let ev2 = mk_evidence("EV-D2", "2026-02-05T08:01:00Z", "d2");
        let mut chain = EvidenceChain::new();
        chain.append(ev1.clone()).unwrap();
        chain.append(ev2.clone()).unwrap();

        let f1 = mk_verified_finding("F-1", ev1.clone());
        let mut f2 = mk_verified_finding("F-2", ev2.clone());
        f2.status = FindingStatus::Duplicate;
        f2.duplicate_of = Some("F-1".to_string());
        let mut f3 = mk_verified_finding("F-3", ev2.clone());
        f3.status = FindingStatus::Duplicate;
        f3.duplicate_of = Some("F-2".to_string());
        assert!(matches!(
            deduplicate_findings(&[f1.clone(), f2, f3], &chain),
            Err(ReportingError::DeduplicationConflict(_))
        ));

        let mut draft_target = f1.clone();
        draft_target.status = FindingStatus::Draft;
        let mut dup_of_draft = mk_verified_finding("F-DUP-DRAFT", ev2.clone());
        dup_of_draft.status = FindingStatus::Duplicate;
        dup_of_draft.duplicate_of = Some("F-1".to_string());
        assert!(matches!(
            deduplicate_findings(&[draft_target, dup_of_draft], &chain),
            Err(ReportingError::DeduplicationConflict(_))
        ));

        let mut f_dup = mk_verified_finding("F-4", ev1);
        f_dup.rule_id = "DIFFERENT".to_string();
        assert!(deduplicate_findings(&[f1.clone(), f_dup], &chain).is_ok());
        let mut bad_dup = f1;
        bad_dup.finding_id = "F-1".to_string();
        assert!(matches!(
            deduplicate_findings(&[bad_dup.clone(), bad_dup], &chain),
            Err(ReportingError::DuplicateIdentifier(_))
        ));
    }

    #[test]
    fn test_add_finding_rejects_out_of_scope_asset_and_unknown_citation() {
        let mut eng = MockReportingLab::standard_engagement().unwrap();
        let ev = mk_evidence("EV-EXTRA", "2026-02-06T16:00:00Z", "extra");
        eng.add_evidence(ev.clone()).unwrap();

        let mut f_oos = mk_verified_finding("F-OOS", ev.clone());
        f_oos.affected_assets = BTreeSet::from(["payroll.corp.example".to_string()]);
        assert!(matches!(
            eng.add_finding(f_oos),
            Err(ReportingError::OutOfScope(_))
        ));

        let mut f_bad_cite = mk_verified_finding("F-CITE", ev);
        f_bad_cite.citation_ids = BTreeSet::from(["UNKNOWN-STD".to_string()]);
        assert!(matches!(
            eng.add_finding(f_bad_cite),
            Err(ReportingError::ReportSchemaInvalid(_))
        ));
    }

    #[test]
    fn test_retest_still_vulnerable_and_partial_priority_update() {
        let mut eng = MockReportingLab::standard_engagement().unwrap();
        eng.advance_phase(EngagementPhase::Retest, "2026-02-13T09:00:00Z", "operator")
            .unwrap();
        let rt_still = mk_evidence("EV-RT-STILL", "2026-02-13T10:20:00Z", "still");
        eng.apply_retest(
            "F-101",
            rt_still,
            RetestOutcome::StillVulnerable,
            "Endpoint vẫn trả 200 OK khi không có admin claim",
            "verifier",
        )
        .unwrap();
        let f101 = eng
            .findings
            .iter()
            .find(|f| f.finding_id == "F-101")
            .unwrap();
        assert!(f101.status.is_open_risk(f101.retest.as_ref()));

        let rt_partial = mk_evidence("EV-RT-104B", "2026-02-13T10:30:00Z", "part2");
        let low_vec = Cvss31Vector::parse("CVSS:3.1/AV:L/AC:H/PR:H/UI:R/S:U/C:L/I:N/A:N").unwrap();
        eng.apply_retest(
            "F-104",
            rt_partial,
            RetestOutcome::PartiallyMitigated {
                residual_vector: low_vec,
            },
            "Đã giảm mức phơi bày chỉ còn trên kịch bản cục bộ",
            "verifier",
        )
        .unwrap();
        let f104 = eng
            .findings
            .iter()
            .find(|f| f.finding_id == "F-104")
            .unwrap();
        assert_eq!(f104.business_priority, BusinessPriority::P2Accelerated);
    }

    #[test]
    fn test_report_validation_catches_tampering_and_renders_separate_views() {
        let eng = MockReportingLab::standard_engagement().unwrap();
        let report = eng
            .build_report(REPORT_GENERATED_UTC, "Báo cáo chuẩn kiểm thử")
            .unwrap();
        let renderer = ReportRenderer;

        let exec = renderer.render_executive_markdown(&report).unwrap();
        let tech = renderer.render_technical_markdown(&report).unwrap();
        let json = renderer.render_json(&report).unwrap();
        assert!(exec.contains("# Executive Security Report"));
        assert!(!exec.contains("## Verified Findings"));
        assert!(tech.contains("## Verified Findings"));
        assert!(tech.contains("Merged Duplicates: F-102"));
        assert!(json.contains("\"excluded_non_actionable\""));
        assert!(json.contains("\"detection_control\""));

        let mut tampered_rem = report.clone();
        tampered_rem.findings[0].remediation.push_str(" (modified)");
        assert!(matches!(
            tampered_rem.validate(),
            Err(ReportingError::ReportSchemaInvalid(_))
        ));

        let mut tampered_summary = report.clone();
        tampered_summary.executive_summary.critical_count = 99;
        assert!(matches!(
            tampered_summary.validate(),
            Err(ReportingError::ReportSchemaInvalid(_))
        ));

        let mut overlap = report;
        overlap
            .excluded_non_actionable_ids
            .push(("F-101".to_string(), FindingStatus::Draft));
        assert!(matches!(
            overlap.validate(),
            Err(ReportingError::DuplicateIdentifier(_))
        ));
    }

    #[test]
    fn test_cvss31_from_str_and_scope_pr_weight_difference() {
        let v_u: Cvss31Vector = "AV:N/AC:L/PR:H/UI:N/S:U/C:H/I:H/A:H".parse().unwrap();
        let v_c: Cvss31Vector = "AV:N/AC:L/PR:H/UI:N/S:C/C:H/I:H/A:H".parse().unwrap();
        assert_eq!(v_u.score().score_tenths, 72);
        assert_eq!(v_c.score().score_tenths, 91);
        assert_eq!(
            v_c.canonical_string(),
            "CVSS:3.1/AV:N/AC:L/PR:H/UI:N/S:C/C:H/I:H/A:H"
        );
    }

    #[test]
    fn test_merge_evidence_vecs_rejects_conflicting_digest() {
        let ev1 = mk_evidence("EV-X", "2026-02-05T08:00:00Z", "x1");
        let ev1_diff = mk_evidence("EV-X", "2026-02-05T08:00:00Z", "x2");
        let mut target = vec![ev1];
        assert!(matches!(
            merge_evidence_vecs(&mut target, &[ev1_diff]),
            Err(ReportingError::DeduplicationConflict(_))
        ));
    }

    #[test]
    fn test_dedup_preserves_exploited_claim_and_criticality_order_independently() {
        let ev1 = mk_evidence("EV-P1", "2026-02-05T08:00:00Z", "p1");
        let ev2 = mk_evidence("EV-P2", "2026-02-05T08:01:00Z", "p2");
        let mut chain = EvidenceChain::new();
        chain.append(ev1.clone()).unwrap();
        chain.append(ev2.clone()).unwrap();

        let mut f_low = mk_verified_finding("F-A", ev1);
        f_low.cvss_vector = Cvss31Vector::parse("AV:N/AC:L/PR:N/UI:R/S:C/C:L/I:L/A:N").unwrap();
        f_low.asset_criticality = AssetCriticality::DevSandbox;
        f_low.exposure_context = ExposureContext::IsolatedLabOnly;
        f_low.business_priority = compute_business_priority(
            f_low.cvss_score().severity,
            f_low.asset_criticality,
            f_low.exposure_context,
        );

        let mut f_exp = mk_verified_finding("F-B", ev2);
        f_exp.rule_id = f_low.rule_id.clone();
        f_exp.root_cause_key = f_low.root_cause_key.clone();
        f_exp.primary_component = f_low.primary_component.clone();
        f_exp
            .set_exploitation_claim(ExploitationClaim::ControlledLabExploited {
                lab_proof_evidence_id: "EV-P2".to_string(),
            })
            .unwrap();

        let r1 = deduplicate_findings(&[f_low.clone(), f_exp.clone()], &chain).unwrap();
        let r2 = deduplicate_findings(&[f_exp, f_low], &chain).unwrap();
        assert_eq!(r1, r2);
        assert!(matches!(
            r1.canonical_findings[0].exploitation_claim,
            ExploitationClaim::ControlledLabExploited { ref lab_proof_evidence_id }
                if lab_proof_evidence_id == "EV-P2"
        ));
        assert_eq!(
            r1.canonical_findings[0].business_priority,
            BusinessPriority::P1Immediate
        );
    }

    #[test]
    fn test_json_and_markdown_code_escaping_safety() {
        let escaped = escape_json_string("line1\nline2\t\"quoted\"\\\x07");
        assert_eq!(escaped, "\"line1\\nline2\\t\\\"quoted\\\"\\\\\\u0007\"");
        assert_eq!(escape_markdown_code("EV`001\nid"), "EV'001 id");
        assert_eq!(escape_markdown_inline("a|b*c`d"), r"a\|b\*c\`d");
    }

    #[test]
    fn test_data_handling_and_disclosure_policy_boundaries() {
        let mut eng = MockReportingLab::standard_engagement().unwrap();
        eng.roe.data_handling.deletion_deadline_utc = "2026-02-01T00:00:00Z".to_string();
        assert!(matches!(
            eng.roe.validate(),
            Err(ReportingError::RetentionViolation(_))
        ));

        let mut eng2 = MockReportingLab::standard_engagement().unwrap();
        eng2.roe.disclosure.remediation_contact = "security@public-vendor.org".to_string();
        assert!(matches!(
            eng2.roe.validate(),
            Err(ReportingError::RawSecretProhibited(_))
        ));
    }

    #[test]
    fn test_data_purged_returns_purged_confirmed_even_past_deadline() {
        let mut eng = MockReportingLab::standard_engagement().unwrap();
        eng.data_purged = true;
        assert_eq!(
            eng.evaluate_retention("2028-01-01T00:00:00Z").unwrap(),
            RetentionDisposition::PurgedConfirmed
        );
    }

    #[test]
    fn test_build_report_rejected_before_report_phase() {
        let mut eng = MockReportingLab::standard_engagement().unwrap();
        eng.phase = EngagementPhase::LabExploit;
        assert!(matches!(
            eng.build_report(REPORT_GENERATED_UTC, "Too early"),
            Err(ReportingError::CleanupNotVerified(_))
        ));
    }

    #[test]
    fn test_apply_retest_rejected_before_retest_phase_and_on_empty_note() {
        let mut eng = MockReportingLab::standard_engagement().unwrap();
        let ev = mk_evidence("EV-RT-EARLY", RETEST_TIME_UTC, "early-phase");
        assert!(matches!(
            eng.apply_retest("F-101", ev.clone(), RetestOutcome::FixedVerified, "ok", "v"),
            Err(ReportingError::InvalidPhaseTransition { .. })
        ));
        eng.advance_phase(EngagementPhase::Retest, "2026-02-13T09:00:00Z", "op")
            .unwrap();
        assert!(matches!(
            eng.apply_retest("F-101", ev, RetestOutcome::FixedVerified, "   ", "v"),
            Err(ReportingError::EvidenceInvalid(_))
        ));
    }

    #[test]
    fn test_finding_validate_catches_priority_mismatch_and_duplicate_evidence() {
        let ev = mk_evidence("EV-VAL1", "2026-02-05T08:00:00Z", "val1");
        let mut chain = EvidenceChain::new();
        chain.append(ev.clone()).unwrap();

        let mut f_bad_prio = mk_verified_finding("F-PRIO", ev.clone());
        f_bad_prio.business_priority = BusinessPriority::P4Backlog;
        assert!(matches!(
            f_bad_prio.validate(&chain),
            Err(ReportingError::ReportSchemaInvalid(_))
        ));

        let mut f_dup_ev = mk_verified_finding("F-DUPEV", ev.clone());
        f_dup_ev.evidence_refs.push(ev);
        assert!(matches!(
            f_dup_ev.validate(&chain),
            Err(ReportingError::DuplicateIdentifier(_))
        ));
    }

    #[test]
    fn test_report_validate_rejects_bad_retention_and_empty_scope() {
        let eng = MockReportingLab::standard_engagement().unwrap();
        let report = eng.build_report(REPORT_GENERATED_UTC, "Valid").unwrap();

        let mut r_ret = report.clone();
        r_ret.retention_days = 0;
        assert!(matches!(
            r_ret.validate(),
            Err(ReportingError::RetentionViolation(_))
        ));

        let mut r_scope = report;
        r_scope.scope_summary.clear();
        assert!(matches!(
            r_scope.validate(),
            Err(ReportingError::ReportSchemaInvalid(_))
        ));
    }

    #[test]
    fn test_audit_log_rejects_sequence_and_timestamp_tampering() {
        let mut eng = MockReportingLab::standard_engagement().unwrap();
        eng.audit_log[0].timestamp_utc = "not-a-utc".to_string();
        assert!(matches!(
            eng.verify_audit_log(),
            Err(ReportingError::AuditLogTampered(_))
        ));
    }

    #[test]
    fn test_role_contact_and_cidr_prefix_bounds() {
        assert!(is_role_contact_email("soc-lead@corp.example"));
        assert!(!is_role_contact_email("..@corp.example"));
        assert!(!is_role_contact_email(".lead@corp.example"));
        assert!(!is_role_contact_email("lead.@corp.example"));
        assert!(!is_role_contact_email("no-at-sign"));
        assert!(!is_role_contact_email("user@public.org"));
        assert!(parse_rfc1918_cidr("192.168.0.0/16").is_ok());
        assert!(parse_rfc1918_cidr("10.0.0.0/7").is_err());
        assert!(parse_rfc1918_cidr("10.0.0.0/33").is_err());
    }

    #[test]
    fn test_h01_redaction_and_secret_validator_close_bypasses() {
        for raw in [
            "Authorization: Bearer\nsyn_tok_secret123",
            "Bearer\tMySecretToken99",
            "password=\tRealSecret99",
            "api_key:\nRealKey99",
            "password = RealSecret99",
            "{\"password\": \"RealSecret99\"}",
            "password=\"my secret pass with spaces\"",
        ] {
            assert!(
                validate_no_raw_secret("bypass", raw).is_err(),
                "expected secret rejection for: {raw}"
            );
            let (clean, cnt) = redact_sensitive_text(raw);
            assert!(cnt >= 1, "expected redaction for: {raw}");
            assert!(!clean.contains("RealSecret99"));
            assert!(!clean.contains("RealKey99"));
            assert!(!clean.contains("MySecretToken99"));
            assert!(!clean.contains("with spaces"));
            validate_no_raw_secret("cleaned", &clean).unwrap();
        }

        // Reject forged adjacent secret suffixes after fake [REDACTED-SECRET] marker
        for forged in [
            "password=[REDACTED-SECRET]\"real_secret123",
            "password=[REDACTED-SECRET];real_secret123",
        ] {
            assert!(
                matches!(
                    validate_no_raw_secret("forged", forged),
                    Err(ReportingError::RawSecretProhibited(_))
                ),
                "expected forged suffix rejection: {forged}"
            );
        }

        // Redact multiple consecutive PEM blocks & DSA/ENCRYPTED headers
        let multi_pem = "-----BEGIN RSA PRIVATE KEY-----\nKEY1\n-----END RSA PRIVATE KEY-----\n\
                         -----BEGIN DSA PRIVATE KEY-----\nKEY2\n-----END DSA PRIVATE KEY-----";
        let (clean_pem, pem_cnt) = redact_sensitive_text(multi_pem);
        assert_eq!(pem_cnt, 2);
        assert!(!clean_pem.contains("KEY1"));
        assert!(!clean_pem.contains("KEY2"));
        validate_no_raw_secret("multi_pem", &clean_pem).unwrap();
    }

    #[test]
    fn test_h02_h03_all_fields_protected_and_excerpt_tampering_caught() {
        let mut eng = MockReportingLab::standard_engagement().unwrap();
        let mut ev_bad_uri = mk_evidence("EV-URI", "2026-02-06T16:00:00Z", "uri");
        ev_bad_uri.source_uri = "virtual://api.corp.example/leaks/ghp_AbCdEf123456".to_string();
        assert!(matches!(
            ev_bad_uri.validate(),
            Err(ReportingError::RawSecretProhibited(_))
        ));

        let mut bad_step = ModuleStepRequest {
            step_id: "Authorization: Bearer syn_tok_leaked_step".to_string(),
            actor: "agent".to_string(),
            method: TestingMethod::PassiveRecon,
            target: TargetAsset::Domain("api.corp.example".to_string()),
            timestamp_utc: "2026-02-02T09:10:00Z".to_string(),
            prohibited_action: None,
        };
        assert!(matches!(
            eng.execute_module_step(&bad_step, |_| Ok("ok".to_string())),
            Err(ReportingError::RawSecretProhibited(_))
        ));
        bad_step.step_id = "ok-step".to_string();
        bad_step.actor = "password=ActorPass99".to_string();
        assert!(matches!(
            eng.execute_module_step(&bad_step, |_| Ok("ok".to_string())),
            Err(ReportingError::RawSecretProhibited(_))
        ));

        let mut report = eng.build_report(REPORT_GENERATED_UTC, "Check").unwrap();
        let mut tam_excerpt = report.clone();
        tam_excerpt.evidence_manifest[0].redacted_excerpt = "forged excerpt".to_string();
        assert!(matches!(
            tam_excerpt.validate(),
            Err(ReportingError::EvidenceInvalid(_))
        ));

        let mut tam_count = report.clone();
        tam_count.evidence_manifest[0].redaction_count = 999;
        tam_count.findings[0].evidence_refs[0].redaction_count = 999;
        assert!(matches!(
            tam_count.validate(),
            Err(ReportingError::ReportSchemaInvalid(_))
        ));

        report.findings[0].rule_id = "RULE_password=Leak99".to_string();
        assert!(matches!(
            report.validate(),
            Err(ReportingError::RawSecretProhibited(_))
        ));
    }

    #[test]
    fn test_h04_retest_partial_preserves_residual_score_and_open_not_closed_by_dup() {
        let mut eng = MockReportingLab::standard_engagement().unwrap();
        eng.advance_phase(EngagementPhase::Retest, "2026-02-13T09:00:00Z", "operator")
            .unwrap();
        let ev_rt = mk_evidence("EV-RT-PART-101", "2026-02-13T10:00:00Z", "part-101");
        let low_vec = Cvss31Vector::parse("CVSS:3.1/AV:L/AC:H/PR:H/UI:R/S:U/C:L/I:N/A:N").unwrap();
        let expected_tenths = low_vec.score().score_tenths;
        eng.apply_retest(
            "F-101",
            ev_rt,
            RetestOutcome::PartiallyMitigated {
                residual_vector: low_vec,
            },
            "Residual vector reduced after fail-closed guard",
            "verifier",
        )
        .unwrap();
        let report = eng
            .build_report("2026-02-13T10:30:00Z", "Retest Report")
            .unwrap();
        let f101 = report
            .findings
            .iter()
            .find(|f| f.finding_id == "F-101")
            .unwrap();
        assert_eq!(f101.cvss_score().score_tenths, expected_tenths);
        assert!(f101.merged_duplicate_ids.contains("F-102"));

        // Same fingerprint: closed FixedVerified must NOT swallow open Verified
        let ev_closed = mk_evidence("EV-CL", "2026-02-05T08:00:00Z", "cl");
        let ev_rt_cl = mk_evidence("EV-RT-CL", "2026-02-05T09:00:00Z", "rt-cl");
        let ev_open = mk_evidence("EV-OP", "2026-02-05T08:10:00Z", "op");
        let mut chain = EvidenceChain::new();
        chain.append(ev_closed.clone()).unwrap();
        chain.append(ev_open.clone()).unwrap();
        chain.append(ev_rt_cl.clone()).unwrap();

        let mut closed_f = mk_verified_finding("F-CL", ev_closed);
        closed_f.rule_id = "SHARED-RULE".to_string();
        closed_f.root_cause_key = "shared-cause".to_string();
        closed_f.primary_component = "api.corp.example/v1".to_string();
        closed_f.status = FindingStatus::Retested;
        closed_f.evidence_refs.push(ev_rt_cl);
        closed_f.retest = Some(RetestRecord {
            retested_at_utc: "2026-02-05T09:00:00Z".to_string(),
            retest_evidence_id: "EV-RT-CL".to_string(),
            outcome: RetestOutcome::FixedVerified,
            verifier_note: "Fixed on host 1".to_string(),
        });

        let mut open_f = mk_verified_finding("F-OP", ev_open);
        open_f.rule_id = "SHARED-RULE".to_string();
        open_f.root_cause_key = "shared-cause".to_string();
        open_f.primary_component = "api.corp.example/v1".to_string();
        open_f.affected_assets = BTreeSet::from(["portal.corp.example".to_string()]);

        let d1 = deduplicate_findings(&[closed_f.clone(), open_f.clone()], &chain).unwrap();
        let d2 = deduplicate_findings(&[open_f, closed_f], &chain).unwrap();
        assert_eq!(d1, d2);
        assert_eq!(d1.canonical_findings.len(), 2);
        assert!(d1.canonical_findings[0].status.is_open_risk(None));
    }

    #[test]
    fn test_m01_m02_m03_m06_l03_l04_governance_and_framing() {
        let mut eng = MockReportingLab::standard_engagement().unwrap();
        eng.record_cleanup_attestation(CleanupAttestation {
            plan_id: "DIRTY".to_string(),
            verified_clean: false,
            remaining_artifacts: 2,
            attested_by_role: "Verifier".to_string(),
            attested_at_utc: "2026-02-11T10:10:00Z".to_string(),
        })
        .unwrap();
        assert!(matches!(
            eng.build_report(REPORT_GENERATED_UTC, "Should Fail"),
            Err(ReportingError::CleanupNotVerified(_))
        ));

        let mut eng_halted = MockReportingLab::standard_engagement().unwrap();
        eng_halted
            .advance_phase(EngagementPhase::Retest, "2026-02-13T09:00:00Z", "op")
            .unwrap();
        eng_halted
            .trigger_stop_condition(
                StopConditionTrigger::UnintendedDataExposure,
                "2026-02-13T09:05:00Z",
                "soc",
                "Emergency stop",
            )
            .unwrap();
        let rt_ev = mk_evidence("EV-RT-HALT", RETEST_TIME_UTC, "halt");
        assert!(matches!(
            eng_halted.apply_retest("F-101", rt_ev, RetestOutcome::FixedVerified, "ok", "v"),
            Err(ReportingError::StopConditionTriggered(
                StopConditionTrigger::UnintendedDataExposure
            ))
        ));

        // Length-prefixed audit log hash distinguishes "a|b"+"c" vs "a"+"b|c"
        let e1 = AuditLogEntry {
            sequence: 0,
            timestamp_utc: WINDOW_START_UTC.to_string(),
            phase: EngagementPhase::Recon,
            actor: "a|b".to_string(),
            action: "c".to_string(),
            target_label: "t".to_string(),
            decision: AuditDecision::Allowed,
            detail: "d".to_string(),
            prev_hash: AUDIT_GENESIS_HASH.to_string(),
            entry_hash: String::new(),
        };
        let e2 = AuditLogEntry {
            actor: "a".to_string(),
            action: "b|c".to_string(),
            ..e1.clone()
        };
        assert_ne!(e1.compute_hash(), e2.compute_hash());

        // FindingStatus::Retested with None retest must fail validation and count as open risk
        assert!(FindingStatus::Retested.is_open_risk(None));
        let ev_bad = mk_evidence("EV-NORT", "2026-02-05T08:00:00Z", "nort");
        let mut f_nort = mk_verified_finding("F-NORT", ev_bad);
        f_nort.status = FindingStatus::Retested;
        assert!(matches!(
            f_nort.validate(&EvidenceChain::new()),
            Err(ReportingError::EvidenceInvalid(_))
        ));

        // Malformed IPv4 returns InvalidIpOrCidr; trailing ellipsis on .example email stays unredacted
        let mut eng_ip = MockReportingLab::standard_engagement().unwrap();
        let ev_ip = mk_evidence("EV-IPERR", "2026-02-06T16:00:00Z", "iperr");
        eng_ip.add_evidence(ev_ip.clone()).unwrap();
        let mut f_ip = mk_verified_finding("F-IP", ev_ip);
        f_ip.affected_assets = BTreeSet::from(["010.0.0.1".to_string()]);
        assert!(matches!(
            eng_ip.add_finding(f_ip),
            Err(ReportingError::InvalidIpOrCidr(_))
        ));

        let (ex_clean, cnt_ex) =
            redact_sensitive_text("contact alice@corp.example... or bob@gmail.com.");
        assert_eq!(cnt_ex, 1);
        assert!(ex_clean.contains("alice@corp.example..."));
        assert!(ex_clean.ends_with("[REDACTED-PII]."));

        // Mismatched third_party_assets entry caught directly by Scope::check_target
        eng_ip.scope.third_party_assets.insert(
            "cdn-auth.vendor.example".to_string(),
            ThirdPartyAuthorization {
                host: "tampered.vendor.example".to_string(),
                owner_org: "Vendor".to_string(),
                written_consent_ref: "REF".to_string(),
                approved_until_utc: "2026-02-20T00:00:00Z".to_string(),
            },
        );
        assert!(matches!(
            eng_ip.scope.check_target(
                &TargetAsset::Domain("cdn-auth.vendor.example".to_string()),
                "2026-02-05T10:00:00Z"
            ),
            Err(ReportingError::ThirdPartyConsentMissing(_))
        ));
    }

    #[test]
    fn test_m04_m05_markdown_block_escaping_json_types_and_outcomes() {
        let mut eng = MockReportingLab::standard_engagement().unwrap();
        eng.advance_phase(EngagementPhase::Retest, "2026-02-13T09:00:00Z", "op")
            .unwrap();
        let rt_ev = mk_evidence("EV-RT-MD", RETEST_TIME_UTC, "rtmd");
        eng.apply_retest(
            "F-101",
            rt_ev,
            RetestOutcome::FixedVerified,
            "Verified clean",
            "verifier",
        )
        .unwrap();
        let mut report = eng
            .build_report("2026-02-13T10:30:00Z", "Title | Test")
            .unwrap();
        report.executive_narrative = "# Injected Heading\n- bullet".to_string();
        report.input_digest_sha256 = report.compute_digest();

        let renderer = ReportRenderer;
        let md = renderer.render_markdown(&report).unwrap();
        assert!(md.contains("\\# Injected Heading - bullet"));
        assert!(md.contains("Halted By Stop Condition: `None`"));
        assert!(md.contains("`FixedVerified`"));

        let json = renderer.render_json(&report).unwrap();
        assert!(json.contains("\"highest_cvss\": 9.8"));
        assert!(json.contains("\"halted_by_stop_condition\": null"));
        assert!(json.contains("\"cleanup_attestation\": {"));
        assert!(json.contains("\"exploitation_claim\":"));
    }

    #[test]
    fn test_advance_phase_and_stop_condition_atomicity_plus_hardening() {
        let mut eng = MockReportingLab::standard_engagement().unwrap();
        let before_phase = eng.phase;
        let before_log_len = eng.audit_log.len();

        // Invalid timestamp in advance_phase must leave phase & audit_log unchanged
        assert!(matches!(
            eng.advance_phase(EngagementPhase::Retest, "bad-timestamp", "operator"),
            Err(ReportingError::InvalidTimestamp(_))
        ));
        assert_eq!(eng.phase, before_phase);
        assert_eq!(eng.audit_log.len(), before_log_len);

        // Raw secret in actor must fail atomically without transitioning phase
        assert!(matches!(
            eng.advance_phase(
                EngagementPhase::Retest,
                "2026-02-13T09:00:00Z",
                "password=LeakActor99"
            ),
            Err(ReportingError::RawSecretProhibited(_))
        ));
        assert_eq!(eng.phase, before_phase);
        assert_eq!(eng.audit_log.len(), before_log_len);

        // Failed trigger_stop_condition leaves halted_by unchanged
        assert!(matches!(
            eng.trigger_stop_condition(
                StopConditionTrigger::EmergencyClientAbort,
                "bad-ts",
                "soc",
                "abort"
            ),
            Err(ReportingError::InvalidTimestamp(_))
        ));
        assert!(eng.halted_by.is_none());

        // Quoted Bearer token with spaces is redacted completely
        let (clean_bearer, cnt) =
            redact_sensitive_text("Authorization: Bearer \"syn tok spaced secret 99\"");
        assert_eq!(cnt, 1);
        assert!(!clean_bearer.contains("spaced secret"));
        validate_no_raw_secret("clean_bearer", &clean_bearer).unwrap();

        // Numbered `1) ` list marker escaped at block start
        assert_eq!(
            escape_markdown_block("1) injected item"),
            "1\\) injected item"
        );
        assert_eq!(
            escape_markdown_block("1. injected item"),
            "1\\. injected item"
        );
    }
}
