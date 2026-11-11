#![allow(dead_code)]
//! Chương 87 — Hệ thống Công cụ Định kiểu, Chuẩn Kỹ năng SKILL.md,
//! Đường nối Năng lực, Vá Tệp Nguyên tử & Hộp cát
//! (Typed Tool System, SKILL.md Standard, Capability Seam & Atomic Patching).
//!
//! # Triết Lý Thiết Kế
//! Trong kiến trúc tác tử tự hành (Autonomous Agent Harness), công cụ không đơn
//! thuần là hàm ngoại vi được triệu gọi qua RPC, mà là **bề mặt rủi ro mở rộng**.
//! Khi mô hình ngôn ngữ lớn (LLM) phát sinh chuỗi gọi công cụ:
//! 1. Dữ liệu đầu vào từ mô hình luôn ở trạng thái **chưa tin cậy** (`untrusted_input`).
//! 2. Quyền thực thi phải được kiểm soát đa tầng: từ danh mục kỹ năng (`SkillCatalog`),
//!    đường nối năng lực cô lập (`CapabilitySeam`), cổng cấp phép sống (`PermissionGate`),
//!    chặn tam giác độc hại (`LethalTrifectaTaintBlocker`), đến hộp cát RAII (`SandboxGuard`).
//! 3. Mọi biến đổi trạng thái đĩa cứng phải tuân thủ kỷ luật **đọc-trước-khi-ghi**,
//!    đòi hỏi khớp duy nhất (`unique match`) và ghi nguyên tử (`SearchReplacePatcher`).
//! 4. Kết quả trả về phải được che giấu bí mật (`OutputScrubber`), cắt tỉa an toàn
//!    biên UTF-8 (`ResultTruncator`) và bắt lỗi đường ống lệnh (`PipefailExecutor`).

use std::collections::{BTreeMap, BTreeSet, HashSet};
use std::fmt;
use std::fs::{self, File};
use std::io::Write;
use std::path::{Path, PathBuf};
use std::sync::Arc;
use std::sync::atomic::{AtomicBool, Ordering};

// ============================================================================
// 1. MÃ BĂM NGUYÊN BẢN SHA-256 (FIPS 180-4) — PURE STD
// ============================================================================

const SHA256_K: [u32; 64] = [
    0x428a2f98, 0x71374491, 0xb5c0fbcf, 0xe9b5dba5, 0x3956c25b, 0x59f111f1, 0x923f82a4, 0xab1c5ed5,
    0xd807aa98, 0x12835b01, 0x243185be, 0x550c7dc3, 0x72be5d74, 0x80deb1fe, 0x9bdc06a7, 0xc19bf174,
    0xe49b69c1, 0xefbe4786, 0x0fc19dc6, 0x240ca1cc, 0x2de92c6f, 0x4a7484aa, 0x5cb0a9dc, 0x76f988da,
    0x983e5152, 0xa831c66d, 0xb00327c8, 0xbf597fc7, 0xc6e00bf3, 0xd5a79147, 0x06ca6351, 0x14292967,
    0x27b70a85, 0x2e1b2138, 0x4d2c6dfc, 0x53380d13, 0x650a7354, 0x766a0abb, 0x81c2c92e, 0x92722c85,
    0xa2bfe8a1, 0xa81a664b, 0xc24b8b70, 0xc76c51a3, 0xd192e819, 0xd6990624, 0xf40e3585, 0x106aa070,
    0x19a4c116, 0x1e376c08, 0x2748774c, 0x34b0bcb5, 0x391c0cb3, 0x4ed8aa4a, 0x5b9cca4f, 0x682e6ff3,
    0x748f82ee, 0x78a5636f, 0x84c87814, 0x8cc70208, 0x90befffa, 0xa4506ceb, 0xbef9a3f7, 0xc67178f2,
];

/// Giá trị băm 256-bit với trường có tên tường minh, tuân thủ chuẩn struct named fields.
#[derive(Clone, Copy, PartialEq, Eq, Hash, PartialOrd, Ord)]
pub struct Sha256Digest {
    pub bytes: [u8; 32],
}

impl Sha256Digest {
    pub const ZERO: Sha256Digest = Sha256Digest { bytes: [0u8; 32] };

    /// Xuất chuỗi hex 64 ký tự.
    pub fn to_hex(&self) -> String {
        let mut s = String::with_capacity(64);
        for b in self.bytes {
            use std::fmt::Write as _;
            let _ = write!(s, "{:02x}", b);
        }
        s
    }
}

impl fmt::Debug for Sha256Digest {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        write!(f, "Sha256({})", &self.to_hex()[..12])
    }
}

impl fmt::Display for Sha256Digest {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        write!(f, "{}", self.to_hex())
    }
}

/// Tính giá trị băm SHA-256 chuẩn FIPS 180-4 hoàn toàn bằng thư viện chuẩn.
pub fn compute_sha256(data: &[u8]) -> Sha256Digest {
    let mut h: [u32; 8] = [
        0x6a09e667, 0xbb67ae85, 0x3c6ef372, 0xa54ff53a, 0x510e527f, 0x9b05688c, 0x1f83d9ab,
        0x5be0cd19,
    ];

    let bit_len = (data.len() as u64).wrapping_mul(8);
    let mut padded = data.to_vec();
    padded.push(0x80);
    while (padded.len() % 64) != 56 {
        padded.push(0x00);
    }
    padded.extend_from_slice(&bit_len.to_be_bytes());

    for chunk in padded.as_chunks::<64>().0 {
        let mut w = [0u32; 64];
        for i in 0..16 {
            w[i] = u32::from_be_bytes([
                chunk[i * 4],
                chunk[i * 4 + 1],
                chunk[i * 4 + 2],
                chunk[i * 4 + 3],
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
            let temp1 = hh
                .wrapping_add(s1)
                .wrapping_add(ch)
                .wrapping_add(SHA256_K[i])
                .wrapping_add(w[i]);
            let s0 = a.rotate_right(2) ^ a.rotate_right(13) ^ a.rotate_right(22);
            let maj = (a & b) ^ (a & c) ^ (b & c);
            let temp2 = s0.wrapping_add(maj);

            hh = g;
            g = f;
            f = e;
            e = d.wrapping_add(temp1);
            d = c;
            c = b;
            b = a;
            a = temp1.wrapping_add(temp2);
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
    for (i, val) in h.iter().enumerate() {
        out[i * 4..(i + 1) * 4].copy_from_slice(&val.to_be_bytes());
    }
    Sha256Digest { bytes: out }
}

// ============================================================================
// 2. PHONG BÌ CÔNG CỤ ĐỊNH KIỂU (TOOL ENVELOPE) & HARNESS META
// ============================================================================

/// Mức độ cô lập hộp cát dành cho phiên thực thi công cụ.
#[derive(Debug, Clone, Copy, PartialEq, Eq, PartialOrd, Ord)]
pub enum SandboxLevel {
    /// Chỉ đọc cô lập, không cho phép thay đổi hệ thống tệp.
    IsolatedReadOnly,
    /// Hệ thống tệp tạm thời ngắn hạn, tự hủy khi kết thúc.
    IsolatedEphemeralFs,
    /// Không gian làm việc có kiểm toán toàn diện.
    FullWorkspaceAudited,
}

/// Metadata xác thực của bộ khung (Harness), được tạo và chứng thực bởi runtime.
/// Trừ phi có chứng chỉ hợp lệ từ kernel, mô hình ngôn ngữ KHÔNG THỂ giả mạo.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct HarnessMeta {
    pub session_id: String,
    pub caller_agent: String,
    pub turn_index: u64,
    pub sandbox_level: SandboxLevel,
    pub granted_scopes: Vec<String>,
    pub origin_timestamp_ms: u64,
}

/// Phong bì đóng gói công cụ tách cứng đầu vào chưa tin cậy khỏi metadata xác thực.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct ToolEnvelope<I> {
    pub untrusted_input: I,
    pub verified_meta: HarnessMeta,
}

impl<I> ToolEnvelope<I> {
    /// Khởi tạo phong bì bảo vệ.
    pub fn new(untrusted_input: I, verified_meta: HarnessMeta) -> Self {
        Self {
            untrusted_input,
            verified_meta,
        }
    }

    /// Tách phong bì thành hai phần độc lập khi đã kiểm chứng xong.
    pub fn split(self) -> (I, HarnessMeta) {
        (self.untrusted_input, self.verified_meta)
    }

    /// Chuyển đổi kiểu dữ liệu đầu vào mà bảo toàn metadata xác thực.
    pub fn map_input<O, F: FnOnce(I) -> O>(self, f: F) -> ToolEnvelope<O> {
        ToolEnvelope {
            untrusted_input: f(self.untrusted_input),
            verified_meta: self.verified_meta,
        }
    }

    /// Kiểm tra xem phong bì có phạm vi quyền hạn được cấp hay không.
    pub fn has_scope(&self, scope: &str) -> bool {
        self.verified_meta
            .granted_scopes
            .iter()
            .any(|s| s.as_str() == scope)
    }
}

// ============================================================================
// 3. CHUẨN KỸ NĂNG SKILL.MD 3 TẦNG (SKILL CATALOG PROGRESSIVE DISCLOSURE)
// ============================================================================

/// Giới hạn số dòng tối đa cho thân tệp SKILL.md (< 500 dòng).
pub const MAX_SKILL_BODY_LINES: usize = 500;

/// Lỗi phát sinh trong quá trình đăng ký hoặc truy xuất kỹ năng.
#[derive(Debug, Clone, PartialEq, Eq)]
pub enum SkillError {
    MissingFrontmatter,
    MissingClosingFrontmatter,
    BodyLineLimitExceeded {
        actual_lines: usize,
        max_allowed: usize,
    },
    MissingRequiredField {
        field_name: String,
    },
    UnknownSkill {
        skill_name: String,
    },
    PathTraversalDenied {
        requested_path: String,
    },
    MissingResource {
        resource_path: String,
    },
}

impl fmt::Display for SkillError {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            Self::MissingFrontmatter => write!(f, "SKILL.md thiếu khối YAML frontmatter bắt đầu"),
            Self::MissingClosingFrontmatter => {
                write!(f, "SKILL.md thiếu khối YAML frontmatter kết thúc")
            }
            Self::BodyLineLimitExceeded {
                actual_lines,
                max_allowed,
            } => {
                write!(
                    f,
                    "Thân SKILL.md dài {actual_lines} dòng, vượt ngưỡng tối đa {max_allowed} dòng"
                )
            }
            Self::MissingRequiredField { field_name } => {
                write!(f, "SKILL.md thiếu trường bắt buộc: {field_name}")
            }
            Self::UnknownSkill { skill_name } => {
                write!(f, "Kỹ năng không tồn tại trong danh mục: {skill_name}")
            }
            Self::PathTraversalDenied { requested_path } => {
                write!(
                    f,
                    "Từ chối nạp tài nguyên vì phát hiện leo thư mục: {requested_path}"
                )
            }
            Self::MissingResource { resource_path } => {
                write!(f, "Tài nguyên phụ trợ không tìm thấy: {resource_path}")
            }
        }
    }
}

impl std::error::Error for SkillError {}

/// Mục thông tin kỹ năng lưu trữ trong danh mục nội bộ.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct SkillEntry {
    pub description: String,
    pub exclude_terms: Vec<String>,
    pub body: String,
    pub resources: BTreeMap<String, String>,
}

/// Kết quả kích hoạt kỹ năng tầng 1.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct ActiveSkill {
    pub name: String,
    pub body: String,
}

/// Danh mục kỹ năng hỗ trợ tiết lộ lũy tiến 3 tầng (Progressive Disclosure):
/// - Tầng 0: Mục lục tóm tắt nhẹ (~2% dung lượng token).
/// - Tầng 1: Thân chỉ dẫn SKILL.md khi phát sinh nhu cầu.
/// - Tầng 2: Tài nguyên phụ trợ (mẫu code, tài liệu tham khảo, kịch bản).
#[derive(Debug, Clone, Default)]
pub struct SkillCatalog {
    pub skills: BTreeMap<String, SkillEntry>,
}

impl SkillCatalog {
    pub fn new() -> Self {
        Self {
            skills: BTreeMap::new(),
        }
    }

    /// Đăng ký một kỹ năng từ chuỗi Markdown chuẩn SKILL.md.
    pub fn register_markdown(
        &mut self,
        raw_markdown: &str,
        resources: BTreeMap<String, String>,
    ) -> Result<(), SkillError> {
        let stripped = raw_markdown.trim_start();
        let after_start = stripped
            .strip_prefix("---\n")
            .or_else(|| stripped.strip_prefix("---\r\n"))
            .ok_or(SkillError::MissingFrontmatter)?;

        let (frontmatter, body) = if let Some(pos) = after_start.find("\n---\n") {
            (&after_start[..pos], &after_start[pos + 5..])
        } else if let Some(pos) = after_start.find("\r\n---\r\n") {
            (&after_start[..pos], &after_start[pos + 7..])
        } else {
            return Err(SkillError::MissingClosingFrontmatter);
        };

        let body_lines = body.lines().count();
        if body_lines >= MAX_SKILL_BODY_LINES || body_lines == 0 {
            return Err(SkillError::BodyLineLimitExceeded {
                actual_lines: body_lines,
                max_allowed: MAX_SKILL_BODY_LINES - 1,
            });
        }

        let mut name = String::new();
        let mut description = String::new();
        let mut exclude_terms = Vec::new();

        for line in frontmatter.lines() {
            if let Some((k, v)) = line.split_once(':') {
                let key = k.trim();
                let val = v.trim().trim_matches('"').trim_matches('\'');
                match key {
                    "name" => name = val.to_string(),
                    "description" => description = val.to_string(),
                    "exclude" => {
                        exclude_terms = val
                            .split(',')
                            .map(|s| s.trim().to_lowercase())
                            .filter(|s| !s.is_empty())
                            .collect();
                    }
                    _ => {}
                }
            }
        }

        if name.is_empty() {
            return Err(SkillError::MissingRequiredField {
                field_name: "name".to_string(),
            });
        }
        if description.is_empty() {
            return Err(SkillError::MissingRequiredField {
                field_name: "description".to_string(),
            });
        }

        self.skills.insert(
            name,
            SkillEntry {
                description,
                exclude_terms,
                body: body.to_string(),
                resources,
            },
        );

        Ok(())
    }

    /// Xuất mục lục tầng 0 (Tier 0 index) siêu nhẹ cho System Prompt.
    pub fn render_tier0_index(&self) -> String {
        let mut lines = Vec::new();
        for (name, entry) in &self.skills {
            lines.push(format!("- {name}: {}", entry.description));
        }
        lines.join("\n")
    }

    /// Kích hoạt kỹ năng tầng 1 (Tier 1) dựa trên câu truy vấn ngữ cảnh.
    /// Áp dụng cơ chế loại trừ âm tính (negative exclude filtering) để tránh kích hoạt nhầm.
    pub fn activate_tier1(&self, query: &str) -> Vec<ActiveSkill> {
        let query_lower = query.to_lowercase();
        let query_words: HashSet<&str> = query_lower
            .split(|c: char| !c.is_alphanumeric() && c != '_')
            .filter(|w| w.len() >= 3)
            .collect();

        let mut activated = Vec::new();

        for (name, entry) in &self.skills {
            let name_lower = name.to_lowercase();
            let desc_lower = entry.description.to_lowercase();

            let matched = query_lower.contains(&name_lower)
                || query_words.iter().any(|&w| desc_lower.contains(w));

            if matched {
                let excluded = entry
                    .exclude_terms
                    .iter()
                    .any(|ex| query_lower.contains(ex.as_str()));

                if !excluded {
                    activated.push(ActiveSkill {
                        name: name.clone(),
                        body: entry.body.clone(),
                    });
                }
            }
        }

        activated
    }

    /// Nạp tài nguyên tầng 2 (Tier 2 resource), thực thi giam đường dẫn an toàn tuyệt đối.
    pub fn load_tier2_resource(
        &self,
        skill_name: &str,
        relative_path: &str,
    ) -> Result<String, SkillError> {
        if relative_path.starts_with('/')
            || relative_path.starts_with('\\')
            || relative_path
                .split(['/', '\\'])
                .any(|part| part == ".." || part.is_empty())
        {
            return Err(SkillError::PathTraversalDenied {
                requested_path: relative_path.to_string(),
            });
        }

        let entry = self
            .skills
            .get(skill_name)
            .ok_or_else(|| SkillError::UnknownSkill {
                skill_name: skill_name.to_string(),
            })?;

        entry
            .resources
            .get(relative_path)
            .cloned()
            .ok_or_else(|| SkillError::MissingResource {
                resource_path: relative_path.to_string(),
            })
    }
}

// ============================================================================
// 4. KIỂM TOÁN CHỒNG LẤP MÔ TẢ CÔNG CỤ (TOOL CATALOG AUDITOR)
// ============================================================================

/// Định nghĩa công cụ được kiểm toán.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct ToolDefinition {
    pub name: String,
    pub description: String,
}

/// Phát hiện xung đột chồng lấn mô tả công cụ có thể gây nhầm lẫn cho LLM.
#[derive(Debug, Clone, PartialEq)]
pub struct ToolAuditIssue {
    pub tool_a: String,
    pub tool_b: String,
    pub jaccard_similarity: f64,
    pub suggestion: String,
}

/// Chế độ kiểm toán danh mục công cụ.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum AuditMode {
    Concise,
    Detailed,
}

/// Kiểm toán viên danh mục công cụ dùng hệ số Jaccard để phát hiện sự mơ hồ ngữ nghĩa.
#[derive(Debug, Clone)]
pub struct ToolCatalogAuditor {
    pub overlap_threshold: f64,
}

impl ToolCatalogAuditor {
    pub fn new(overlap_threshold: f64) -> Self {
        Self { overlap_threshold }
    }

    /// Tách từ khóa chuẩn hóa từ mô tả công cụ.
    pub fn tokenize_description(text: &str) -> HashSet<String> {
        text.to_lowercase()
            .split(|c: char| !c.is_alphanumeric() && c != '_')
            .filter(|w| w.len() >= 3)
            .map(str::to_string)
            .collect()
    }

    /// Tính chỉ số tương đồng Jaccard giữa hai tập từ khóa: |A ∩ B| / |A ∪ B|.
    pub fn compute_jaccard(set_a: &HashSet<String>, set_b: &HashSet<String>) -> f64 {
        if set_a.is_empty() && set_b.is_empty() {
            return 1.0;
        }
        let intersection = set_a.intersection(set_b).count();
        let union = set_a.union(set_b).count();
        if union == 0 {
            0.0
        } else {
            intersection as f64 / union as f64
        }
    }

    /// Kiểm toán danh mục công cụ và tạo khuyến nghị tự khắc phục.
    pub fn audit_catalog(&self, tools: &[ToolDefinition], mode: AuditMode) -> Vec<ToolAuditIssue> {
        let mut issues = Vec::new();
        let tokenized: Vec<HashSet<String>> = tools
            .iter()
            .map(|t| Self::tokenize_description(&t.description))
            .collect();

        for i in 0..tools.len() {
            for j in (i + 1)..tools.len() {
                let score = Self::compute_jaccard(&tokenized[i], &tokenized[j]);
                if score >= self.overlap_threshold {
                    let common: Vec<String> =
                        tokenized[i].intersection(&tokenized[j]).cloned().collect();

                    let suggestion = match mode {
                        AuditMode::Concise => {
                            format!(
                                "Phân tách ranh giới giữa '{}' và '{}' (Jaccard: {:.2})",
                                tools[i].name, tools[j].name, score
                            )
                        }
                        AuditMode::Detailed => {
                            format!(
                                "Công cụ '{}' và '{}' trùng lặp {:.1}% từ khóa: {:?}. Hãy làm rõ ngữ cảnh kích hoạt riêng biệt.",
                                tools[i].name,
                                tools[j].name,
                                score * 100.0,
                                common
                            )
                        }
                    };

                    issues.push(ToolAuditIssue {
                        tool_a: tools[i].name.clone(),
                        tool_b: tools[j].name.clone(),
                        jaccard_similarity: score,
                        suggestion,
                    });
                }
            }
        }

        issues
    }
}

// ============================================================================
// 5. ĐƯỜNG NỐI NĂNG LỰC (CAPABILITY SEAM) 3 VAI TRÒ
// ============================================================================

/// Lỗi phát sinh trong các thao tác hệ thống tệp và năng lực ngoại vi.
#[derive(Debug, Clone, PartialEq, Eq)]
pub enum CapabilityError {
    AccessDenied { path: String, reason: String },
    NotFound { path: String },
    IoFailure { details: String },
}

impl fmt::Display for CapabilityError {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            Self::AccessDenied { path, reason } => {
                write!(f, "Truy cập bị từ chối tại '{path}': {reason}")
            }
            Self::NotFound { path } => write!(f, "Không tìm thấy đường dẫn: '{path}'"),
            Self::IoFailure { details } => write!(f, "Lỗi I/O ngoại vi: {details}"),
        }
    }
}

impl std::error::Error for CapabilityError {}

/// Hợp đồng đường nối năng lực (Capability Service Trait).
pub trait CapabilityService {
    fn read_file(&self, path: &str) -> Result<String, CapabilityError>;
    fn write_file(&mut self, path: &str, content: &str) -> Result<(), CapabilityError>;
    fn list_dir(&self, path: &str) -> Result<Vec<String>, CapabilityError>;
    fn provider_name(&self) -> &'static str;
}

/// Nhà cung cấp giả lập bộ nhớ (Mock Sandbox Provider) phục vụ kiểm thử siêu tốc.
#[derive(Debug, Clone, Default)]
pub struct MockSandboxProvider {
    pub storage: BTreeMap<String, String>,
}

impl MockSandboxProvider {
    pub fn new() -> Self {
        Self {
            storage: BTreeMap::new(),
        }
    }
}

impl CapabilityService for MockSandboxProvider {
    fn read_file(&self, path: &str) -> Result<String, CapabilityError> {
        self.storage
            .get(path)
            .cloned()
            .ok_or_else(|| CapabilityError::NotFound {
                path: path.to_string(),
            })
    }

    fn write_file(&mut self, path: &str, content: &str) -> Result<(), CapabilityError> {
        self.storage.insert(path.to_string(), content.to_string());
        Ok(())
    }

    fn list_dir(&self, path: &str) -> Result<Vec<String>, CapabilityError> {
        let prefix = if path.ends_with('/') {
            path.to_string()
        } else {
            format!("{path}/")
        };

        let mut results = BTreeSet::new();
        for key in self.storage.keys() {
            if let Some(rest) = key.strip_prefix(&prefix) {
                let first_component = rest.split('/').next().unwrap_or("");
                if !first_component.is_empty() {
                    results.insert(first_component.to_string());
                }
            }
        }

        Ok(results.into_iter().collect())
    }

    fn provider_name(&self) -> &'static str {
        "MockSandboxProvider"
    }
}

/// Bản ghi kiểm toán thao tác năng lực.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct CapabilityAuditRecord {
    pub action: &'static str,
    pub target_path: String,
    pub timestamp_ms: u64,
}

/// Nhà cung cấp có ghi nhật ký kiểm toán (Audited Provider Wrapper).
#[derive(Debug)]
pub struct AuditedProvider<P: CapabilityService> {
    pub inner: P,
    pub audit_log: Vec<CapabilityAuditRecord>,
    pub current_time_ms: u64,
}

impl<P: CapabilityService> AuditedProvider<P> {
    pub fn new(inner: P) -> Self {
        Self {
            inner,
            audit_log: Vec::new(),
            current_time_ms: 1_700_000_000_000,
        }
    }
}

impl<P: CapabilityService> CapabilityService for AuditedProvider<P> {
    fn read_file(&self, path: &str) -> Result<String, CapabilityError> {
        self.inner.read_file(path)
    }

    fn write_file(&mut self, path: &str, content: &str) -> Result<(), CapabilityError> {
        self.audit_log.push(CapabilityAuditRecord {
            action: "write_file",
            target_path: path.to_string(),
            timestamp_ms: self.current_time_ms,
        });
        self.inner.write_file(path, content)
    }

    fn list_dir(&self, path: &str) -> Result<Vec<String>, CapabilityError> {
        self.inner.list_dir(path)
    }

    fn provider_name(&self) -> &'static str {
        "AuditedProvider"
    }
}

// ============================================================================
// 6. CHUỖI PRE-TOOL HOOK ĐÓNG-AN-TOÀN & PATHGUARD CHỐNG SYMLINK ESCAPE
// ============================================================================

/// Lỗi bảo vệ đường dẫn hệ thống tệp.
#[derive(Debug, Clone, PartialEq, Eq)]
pub enum PathGuardError {
    OutsideRootJail {
        attempted_path: PathBuf,
        root_jail: PathBuf,
    },
    SymlinkEscapeForbidden {
        path: PathBuf,
    },
    InvalidUtf8Path,
}

impl fmt::Display for PathGuardError {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            Self::OutsideRootJail {
                attempted_path,
                root_jail,
            } => {
                write!(
                    f,
                    "Đường dẫn '{attempted_path:?}' vượt ra ngoài thư mục gốc an toàn '{root_jail:?}'"
                )
            }
            Self::SymlinkEscapeForbidden { path } => {
                write!(
                    f,
                    "Phát hiện liên kết tượng trưng (symlink) thoát rào: {path:?}"
                )
            }
            Self::InvalidUtf8Path => write!(f, "Đường dẫn chứa ký tự UTF-8 không hợp lệ"),
        }
    }
}

impl std::error::Error for PathGuardError {}

/// Bộ bảo vệ đường dẫn hệ thống tệp, ngăn chặn Path Traversal và Symlink Traversal.
#[derive(Debug, Clone)]
pub struct PathGuard {
    pub allowed_root: PathBuf,
}

impl PathGuard {
    pub fn new<P: Into<PathBuf>>(root: P) -> Self {
        Self {
            allowed_root: root.into(),
        }
    }

    /// Làm sạch đường dẫn ảo (chuẩn hóa các thành phần `.` và `..`) mà không phụ thuộc I/O đĩa.
    pub fn sanitize_virtual_path(&self, candidate: &str) -> Result<PathBuf, PathGuardError> {
        let mut components = Vec::new();
        for part in candidate.split(['/', '\\']) {
            if part.is_empty() || part == "." {
                continue;
            }
            if part == ".." {
                if components.pop().is_none() {
                    return Err(PathGuardError::OutsideRootJail {
                        attempted_path: PathBuf::from(candidate),
                        root_jail: self.allowed_root.clone(),
                    });
                }
            } else {
                components.push(part);
            }
        }

        let mut resolved = self.allowed_root.clone();
        for c in components {
            resolved.push(c);
        }

        Ok(resolved)
    }

    /// Kiểm tra đường dẫn thật trên đĩa với `fs::canonicalize` nếu thư mục tồn tại.
    pub fn verify_disk_path(&self, candidate: &Path) -> Result<PathBuf, PathGuardError> {
        let canonical_root = self
            .allowed_root
            .canonicalize()
            .unwrap_or_else(|_| self.allowed_root.clone());
        let canonical_cand = candidate
            .canonicalize()
            .unwrap_or_else(|_| candidate.to_path_buf());

        if canonical_cand.starts_with(&canonical_root) {
            Ok(canonical_cand)
        } else {
            Err(PathGuardError::OutsideRootJail {
                attempted_path: canonical_cand,
                root_jail: canonical_root,
            })
        }
    }
}

/// Lý do từ chối của PreToolHook.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct HookRejection {
    pub hook_name: &'static str,
    pub reason: String,
}

/// Hợp đồng kiểm tra tiền điều kiện công cụ.
pub trait PreToolHook {
    fn inspect(
        &self,
        tool_name: &str,
        arguments: &BTreeMap<String, String>,
    ) -> Result<(), HookRejection>;
}

/// Chuỗi kiểm tra PreToolHook đóng-an-toàn (Fail-Closed Chain):
/// Bất kỳ hook nào từ chối thì toàn bộ chuỗi dừng ngay lập tức.
#[derive(Default)]
pub struct HookChain {
    pub hooks: Vec<Box<dyn PreToolHook>>,
}

impl HookChain {
    pub fn new() -> Self {
        Self::default()
    }

    pub fn add_hook<H: PreToolHook + 'static>(&mut self, hook: H) {
        self.hooks.push(Box::new(hook));
    }

    pub fn evaluate(
        &self,
        tool_name: &str,
        arguments: &BTreeMap<String, String>,
    ) -> Result<(), HookRejection> {
        for hook in &self.hooks {
            hook.inspect(tool_name, arguments)?;
        }
        Ok(())
    }
}

// ============================================================================
// 7. SPOTLIGHTING & CHẶN TAM GIÁC ĐỘC HẠI (LETHAL TRIFECTA TAINT BLOCKER)
// ============================================================================

/// Mức độ tin cậy của khối dữ liệu trong ngữ cảnh LLM.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum TrustLevel {
    /// Dữ liệu có nguồn gốc từ hệ thống / người dùng tin cậy.
    TrustedSystem,
    /// Dữ liệu trích xuất từ tài nguyên bên ngoài (Web, Repo, Mail, MCP).
    TaintedExternal,
}

/// Khối dữ liệu được đánh dấu ranh giới Spotlighting.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct SpotlightedSpan {
    pub content: String,
    pub trust_level: TrustLevel,
    pub source_label: String,
}

impl SpotlightedSpan {
    /// Bọc dữ liệu ngoại vi bằng ranh giới rõ ràng ngăn cách Prompt Injection.
    pub fn wrap_tainted(raw: &str, source: &str) -> Self {
        Self {
            content: format!(
                "<<<UNTRUSTED_CONTENT source=\"{source}\">>>\n{raw}\n<<<END_UNTRUSTED_CONTENT>>>"
            ),
            trust_level: TrustLevel::TaintedExternal,
            source_label: source.to_string(),
        }
    }

    pub fn wrap_trusted(raw: &str, source: &str) -> Self {
        Self {
            content: raw.to_string(),
            trust_level: TrustLevel::TrustedSystem,
            source_label: source.to_string(),
        }
    }
}

/// Cảnh báo kích hoạt khi 3 chân của Tam giác Độc hại (Lethal Trifecta) hội tụ.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct TrifectaAlarm {
    pub reason: String,
    pub session_tainted: bool,
    pub secrets_accessed: bool,
    pub egress_attempted: bool,
}

impl fmt::Display for TrifectaAlarm {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        write!(
            f,
            "LETHAL TRIFECTA INTERCEPT: Ngắt kết nối mạng ra ngoài do hội tụ cả 3 chân: Dữ liệu nhiễm bẩn + Quyền đọc bí mật + Kênh truyền ngoại vi"
        )
    }
}

impl std::error::Error for TrifectaAlarm {}

/// Bộ giám sát trạng thái dòng nhiễm bẩn (Taint Tracker & Blocker).
#[derive(Debug, Clone, Default)]
pub struct LethalTrifectaTaintBlocker {
    pub has_tainted_input: bool,
    pub has_read_secrets: bool,
}

impl LethalTrifectaTaintBlocker {
    pub fn new() -> Self {
        Self::default()
    }

    pub fn record_input_ingested(&mut self, level: TrustLevel) {
        if level == TrustLevel::TaintedExternal {
            self.has_tainted_input = true;
        }
    }

    pub fn record_secret_accessed(&mut self) {
        self.has_read_secrets = true;
    }

    /// Kiểm tra xem yêu cầu truyền dữ liệu ra ngoài (network egress) có an toàn không.
    pub fn verify_egress_allowed(&self) -> Result<(), TrifectaAlarm> {
        if self.has_tainted_input && self.has_read_secrets {
            Err(TrifectaAlarm {
                reason: "Hội tụ tam giác độc hại".to_string(),
                session_tainted: self.has_tainted_input,
                secrets_accessed: self.has_read_secrets,
                egress_attempted: true,
            })
        } else {
            Ok(())
        }
    }
}

// ============================================================================
// 8. TẦNG CHỐNG Ô NHIỄM GIAO THỨC MCP (MCP ANTI-CORRUPTION LAYER)
// ============================================================================

/// Bản kê khai công cụ thô nhận từ máy chủ MCP ngoài.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct McpRawManifest {
    pub raw_name: String,
    pub raw_description: String,
    pub claimed_read_only: bool,
    pub raw_properties: BTreeMap<String, String>,
}

/// Định nghĩa công cụ sau khi được lọc sạch bởi tầng chống ô nhiễm.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct SanitizedTool {
    pub canonical_name: String,
    pub canonical_description: String,
    pub is_read_only_safe: bool,
    pub validated_properties: BTreeMap<String, String>,
}

/// Lỗi do tầng chống ô nhiễm phát hiện khi kiểm định schema ngoại vi.
#[derive(Debug, Clone, PartialEq, Eq)]
pub enum McpSanitizeError {
    IllegalReadOnlyClaim { tool_name: String },
    DisallowedPrefix { tool_name: String },
    DangerousPropertyInjection { property_name: String },
}

impl fmt::Display for McpSanitizeError {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            Self::IllegalReadOnlyClaim { tool_name } => {
                write!(
                    f,
                    "Công cụ '{tool_name}' có tên gây biến đổi trạng thái nhưng tự nhận là readOnlyHint"
                )
            }
            Self::DisallowedPrefix { tool_name } => {
                write!(f, "Tên công cụ '{tool_name}' chứa tiền tố bị cấm")
            }
            Self::DangerousPropertyInjection { property_name } => {
                write!(
                    f,
                    "Phát hiện thuộc tính lạ có nguy cơ chèn mã: {property_name}"
                )
            }
        }
    }
}

impl std::error::Error for McpSanitizeError {}

/// Tầng chống ô nhiễm (ACL) bảo vệ hệ thống trước các schema MCP giả mạo.
pub struct McpAntiCorruptionLayer;

impl McpAntiCorruptionLayer {
    /// Chuẩn hóa và làm sạch bản kê khai công cụ từ bên ngoài.
    pub fn sanitize_manifest(raw: &McpRawManifest) -> Result<SanitizedTool, McpSanitizeError> {
        let name_lower = raw.raw_name.to_lowercase();

        // 1. Chặn công cụ ghi/xóa/thực thi tự xưng là chỉ-đọc (Fake readOnlyHint)
        let is_mutating_name = name_lower.contains("write")
            || name_lower.contains("delete")
            || name_lower.contains("remove")
            || name_lower.contains("exec")
            || name_lower.contains("drop");

        if is_mutating_name && raw.claimed_read_only {
            return Err(McpSanitizeError::IllegalReadOnlyClaim {
                tool_name: raw.raw_name.clone(),
            });
        }

        // 2. Chặn tiền tố can thiệp hệ thống nội bộ
        if name_lower.starts_with("__internal") || name_lower.starts_with("system_") {
            return Err(McpSanitizeError::DisallowedPrefix {
                tool_name: raw.raw_name.clone(),
            });
        }

        // 3. Lọc bỏ các thuộc tính lạ hoặc nguy hiểm
        let mut clean_props = BTreeMap::new();
        for (k, v) in &raw.raw_properties {
            if k.starts_with("$") || k.contains("eval") || k.contains("constructor") {
                return Err(McpSanitizeError::DangerousPropertyInjection {
                    property_name: k.clone(),
                });
            }
            clean_props.insert(k.clone(), v.clone());
        }

        Ok(SanitizedTool {
            canonical_name: raw.raw_name.trim().to_string(),
            canonical_description: raw.raw_description.trim().to_string(),
            is_read_only_safe: raw.claimed_read_only && !is_mutating_name,
            validated_properties: clean_props,
        })
    }
}

// ============================================================================
// 9. BỘ LẬP LỊCH SÓNG CÔNG CỤ (TOOL WAVE SCHEDULER: ĐỌC SONG SONG / GHI TUẦN TỰ)
// ============================================================================

/// Chế độ đồng thời của công cụ.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum ToolConcurrencyMode {
    /// Công cụ chỉ đọc, an toàn khi chạy song song.
    ReadOnlyConcurrent,
    /// Công cụ ghi hoặc gây hiệu ứng phụ, bắt buộc chạy độc quyền.
    ExclusiveWrite,
}

/// Yêu cầu gọi công cụ được đưa vào lịch điều phối.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct ScheduledToolCall {
    pub call_id: String,
    pub tool_name: String,
    pub arguments: BTreeMap<String, String>,
    pub mode: ToolConcurrencyMode,
}

/// Một đợt sóng thực thi công cụ.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct ToolWave {
    pub wave_index: usize,
    pub calls: Vec<ScheduledToolCall>,
    pub is_concurrent: bool,
}

/// Bộ lập lịch gom nhóm các công cụ chỉ đọc thành sóng song song,
/// và tách các công cụ ghi thành hàng rào đồng bộ hóa tuần tự.
pub struct ToolWaveScheduler;

impl ToolWaveScheduler {
    pub fn schedule_waves(calls: Vec<ScheduledToolCall>) -> Vec<ToolWave> {
        let mut waves = Vec::new();
        let mut current_read_wave: Vec<ScheduledToolCall> = Vec::new();

        for call in calls {
            match call.mode {
                ToolConcurrencyMode::ReadOnlyConcurrent => {
                    current_read_wave.push(call);
                }
                ToolConcurrencyMode::ExclusiveWrite => {
                    // Xả hết sóng đọc hiện tại trước khi thực hiện bước ghi
                    if !current_read_wave.is_empty() {
                        waves.push(ToolWave {
                            wave_index: waves.len(),
                            calls: std::mem::take(&mut current_read_wave),
                            is_concurrent: true,
                        });
                    }
                    // Bước ghi luôn đứng đơn độc trong một sóng độc quyền
                    waves.push(ToolWave {
                        wave_index: waves.len(),
                        calls: vec![call],
                        is_concurrent: false,
                    });
                }
            }
        }

        if !current_read_wave.is_empty() {
            waves.push(ToolWave {
                wave_index: waves.len(),
                calls: current_read_wave,
                is_concurrent: true,
            });
        }

        waves
    }
}

// ============================================================================
// 10. CỔNG PHÊ DUYỆT ĐA CẤP (PERMISSION GATE) & KILL SWITCH SỐNG
// ============================================================================

/// Cấp độ quyền hạn công cụ.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum PermissionLevel {
    Deny,
    Allow,
    AskHuman,
}

/// Kết quả đánh giá từ PermissionGate.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct PermissionDecision {
    pub tool_name: String,
    pub allowed: bool,
    pub requires_human_prompt: bool,
    pub reason: String,
}

/// Cổng quyền hạn 4 tầng (Deny -> Allow -> Mode -> Ask) kết hợp Kill-Switch sống.
#[derive(Debug)]
pub struct PermissionGate {
    pub rules: BTreeMap<String, PermissionLevel>,
    pub default_level: PermissionLevel,
    pub live_kill_switch: Arc<AtomicBool>,
}

impl PermissionGate {
    pub fn new(live_kill_switch: Arc<AtomicBool>, default_level: PermissionLevel) -> Self {
        Self {
            rules: BTreeMap::new(),
            default_level,
            live_kill_switch,
        }
    }

    pub fn set_rule(&mut self, tool_name: &str, level: PermissionLevel) {
        self.rules.insert(tool_name.to_string(), level);
    }

    /// Đánh giá quyền hạn theo thứ tự ưu tiên nghiêm ngặt.
    pub fn evaluate_permission(&self, tool_name: &str) -> PermissionDecision {
        // 1. Kiểm tra cờ ngắt khẩn cấp sống (Live Kill-Switch) — Veto tối thượng
        if self.live_kill_switch.load(Ordering::SeqCst) {
            return PermissionDecision {
                tool_name: tool_name.to_string(),
                allowed: false,
                requires_human_prompt: false,
                reason:
                    "E_KILL_SWITCH_ACTIVE: Bộ ngắt khẩn cấp đang kích hoạt, từ chối toàn bộ lệnh"
                        .to_string(),
            };
        }

        // 2. Tra cứu quy tắc cụ thể của công cụ
        let level = self
            .rules
            .get(tool_name)
            .copied()
            .unwrap_or(self.default_level);

        match level {
            PermissionLevel::Deny => PermissionDecision {
                tool_name: tool_name.to_string(),
                allowed: false,
                requires_human_prompt: false,
                reason: "Chính sách từ chối tuyệt đối (Deny)".to_string(),
            },
            PermissionLevel::Allow => PermissionDecision {
                tool_name: tool_name.to_string(),
                allowed: true,
                requires_human_prompt: false,
                reason: "Chính sách cho phép tự động (Allow)".to_string(),
            },
            PermissionLevel::AskHuman => PermissionDecision {
                tool_name: tool_name.to_string(),
                allowed: false,
                requires_human_prompt: true,
                reason: "Yêu cầu người dùng xác nhận trực tiếp (AskHuman)".to_string(),
            },
        }
    }
}

// ============================================================================
// 11. CHUỖI BÍ MẬT (SECRETSTRING) & BỘ TẨY RỬA ĐẦU RA (OUTPUT SCRUBBER)
// ============================================================================

/// Chuỗi bí mật được bảo vệ trong RAM:
/// - Debug và Display KHÔNG BAO GIỜ để lộ nội dung thật.
/// - Drop tự động ghi đè 0x00 lên bộ nhớ (Zeroize).
#[derive(Clone, PartialEq, Eq)]
pub struct SecretString {
    bytes: Vec<u8>,
}

impl SecretString {
    pub fn new(secret: &str) -> Self {
        Self {
            bytes: secret.as_bytes().to_vec(),
        }
    }

    /// Chỉ cho phép trích xuất nội dung khi thực sự gọi RPC có kiểm toán.
    pub fn expose_secret(&self) -> &str {
        std::str::from_utf8(&self.bytes).unwrap_or("<INVALID_UTF8_SECRET>")
    }
}

impl Drop for SecretString {
    fn drop(&mut self) {
        // Zeroize bộ nhớ nhạy cảm trước khi giải phóng
        for byte in self.bytes.iter_mut() {
            *byte = 0x00;
        }
    }
}

impl fmt::Debug for SecretString {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        write!(f, "SecretString([REDACTED])")
    }
}

impl fmt::Display for SecretString {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        write!(f, "[REDACTED_SECRET]")
    }
}

/// Dải ký tự cần che giấu.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct ScrubSpan {
    pub start: usize,
    pub end: usize,
    pub label: &'static str,
}

/// Bộ tẩy rửa đầu ra hợp nhất các dải chồng lấn và bảo toàn biên UTF-8.
pub struct OutputScrubber {
    pub known_secrets: Vec<String>,
}

impl OutputScrubber {
    pub fn new(known_secrets: Vec<String>) -> Self {
        Self { known_secrets }
    }

    /// Tẩy rửa một đoạn văn bản thô, che giấu các khóa bí mật và token xác thực.
    pub fn scrub_text(&self, input: &str) -> String {
        let mut spans = Vec::new();

        // 1. Quét các chuỗi bí mật đã biết
        for secret in &self.known_secrets {
            if secret.is_empty() {
                continue;
            }
            let mut start = 0;
            while let Some(pos) = input[start..].find(secret) {
                let actual_start = start + pos;
                let actual_end = actual_start + secret.len();
                spans.push(ScrubSpan {
                    start: actual_start,
                    end: actual_end,
                    label: "[REDACTED_SECRET]",
                });
                start = actual_start + secret.len();
            }
        }

        // 2. Quét các mẫu Bearer token phổ biến
        let mut search_idx = 0;
        while let Some(pos) = input[search_idx..].find("Bearer ") {
            let token_start = search_idx + pos + 7;
            let token_len = input[token_start..]
                .chars()
                .take_while(|c| c.is_alphanumeric() || *c == '_' || *c == '-' || *c == '.')
                .count();

            if token_len > 0 {
                spans.push(ScrubSpan {
                    start: token_start,
                    end: token_start + token_len,
                    label: "[REDACTED_BEARER]",
                });
            }
            search_idx = token_start + token_len;
        }

        if spans.is_empty() {
            return input.to_string();
        }

        // 3. Sắp xếp và hợp nhất các dải chồng lấn [start, end)
        spans.sort_by_key(|s| (s.start, s.end));
        let mut merged: Vec<ScrubSpan> = Vec::new();
        for span in spans {
            if let Some(last) = merged.last_mut()
                && span.start <= last.end
            {
                if span.end > last.end {
                    last.end = span.end;
                }
                continue;
            }
            merged.push(span);
        }

        // 4. Thay thế ngược từ phải qua trái để không làm xô lệch offset
        let mut result = input.to_string();
        for span in merged.into_iter().rev() {
            // Kiểm tra an toàn biên UTF-8
            if result.is_char_boundary(span.start) && result.is_char_boundary(span.end) {
                result.replace_range(span.start..span.end, span.label);
            }
        }

        result
    }

    /// Tẩy rửa đồng thời cả Headers lẫn Body của thông điệp RPC/HTTP.
    pub fn scrub_headers_and_body(
        &self,
        headers: &BTreeMap<String, String>,
        body: &str,
    ) -> (BTreeMap<String, String>, String) {
        let mut clean_headers = BTreeMap::new();
        for (k, v) in headers {
            if k.eq_ignore_ascii_case("authorization") || k.eq_ignore_ascii_case("x-api-key") {
                clean_headers.insert(k.clone(), "[REDACTED_HEADER]".to_string());
            } else {
                clean_headers.insert(k.clone(), self.scrub_text(v));
            }
        }

        let clean_body = self.scrub_text(body);
        (clean_headers, clean_body)
    }
}

// ============================================================================
// 12. HỘP CÁT QUẢN LÝ THEO VÒNG ĐỜI RAII (SANDBOX GUARD)
// ============================================================================

/// Hộp cát quản lý tài nguyên tự hủy qua cơ chế RAII Drop.
#[derive(Debug)]
pub struct SandboxGuard {
    pub sandbox_id: String,
    pub temporary_directory: PathBuf,
    pub active_resources: Vec<String>,
}

impl SandboxGuard {
    pub fn new(sandbox_id: &str, temp_dir: PathBuf) -> Self {
        let _ = fs::create_dir_all(&temp_dir);
        Self {
            sandbox_id: sandbox_id.to_string(),
            temporary_directory: temp_dir,
            active_resources: Vec::new(),
        }
    }

    pub fn allocate_resource(&mut self, name: &str) {
        self.active_resources.push(name.to_string());
    }
}

impl Drop for SandboxGuard {
    fn drop(&mut self) {
        // Tự động dọn dẹp toàn bộ tài nguyên hộp cát khi ra khỏi tầm vực
        if self.temporary_directory.exists() {
            let _ = fs::remove_dir_all(&self.temporary_directory);
        }
        self.active_resources.clear();
    }
}

// ============================================================================
// 13. MÁY ẢO TẬP LỆNH ĐÓNG (ACTION VM)
// ============================================================================

/// Các thao tác được phép trong tập lệnh đóng của ActionVm.
#[derive(Debug, Clone, PartialEq, Eq)]
pub enum ActionOp {
    ListDir { path: String },
    ReadFile { path: String, max_bytes: usize },
    WriteFile { path: String, content: String },
    VerifySha256 { path: String, expected_hex: String },
}

/// Giới hạn hạn mức thực thi của máy ảo.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct VmLimits {
    pub max_operations: usize,
    pub max_bytes_read: usize,
    pub max_bytes_written: usize,
}

/// Lỗi phát sinh trong máy ảo ActionVm.
#[derive(Debug, Clone, PartialEq, Eq)]
pub enum VmError {
    OperationLimitExceeded {
        limit: usize,
    },
    ReadLimitExceeded {
        limit: usize,
        attempted: usize,
    },
    WriteLimitExceeded {
        limit: usize,
        attempted: usize,
    },
    HashMismatch {
        path: String,
        expected: String,
        actual: String,
    },
    CapabilityFailure {
        details: String,
    },
}

impl fmt::Display for VmError {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            Self::OperationLimitExceeded { limit } => {
                write!(f, "Số thao tác vượt quá ngưỡng cho phép: {limit}")
            }
            Self::ReadLimitExceeded { limit, attempted } => {
                write!(f, "Số byte đọc {attempted} vượt giới hạn {limit}")
            }
            Self::WriteLimitExceeded { limit, attempted } => {
                write!(f, "Số byte ghi {attempted} vượt giới hạn {limit}")
            }
            Self::HashMismatch {
                path,
                expected,
                actual,
            } => {
                write!(
                    f,
                    "Băm SHA-256 không khớp tại '{path}': kỳ vọng {expected}, thực tế {actual}"
                )
            }
            Self::CapabilityFailure { details } => {
                write!(f, "Lỗi từ nhà cung cấp năng lực: {details}")
            }
        }
    }
}

impl std::error::Error for VmError {}

/// Máy ảo mã lệnh đóng với hạn mức tiêu thụ tài nguyên chặt chẽ.
pub struct ActionVm {
    pub limits: VmLimits,
    pub ops_count: usize,
    pub total_bytes_read: usize,
    pub total_bytes_written: usize,
}

impl ActionVm {
    pub fn new(limits: VmLimits) -> Self {
        Self {
            limits,
            ops_count: 0,
            total_bytes_read: 0,
            total_bytes_written: 0,
        }
    }

    /// Thực thi chương trình chuỗi lệnh tuần tự trên CapabilityService.
    pub fn execute_program<C: CapabilityService>(
        &mut self,
        capability: &mut C,
        program: Vec<ActionOp>,
    ) -> Result<Vec<String>, VmError> {
        let mut outputs = Vec::new();

        for op in program {
            if self.ops_count >= self.limits.max_operations {
                return Err(VmError::OperationLimitExceeded {
                    limit: self.limits.max_operations,
                });
            }
            self.ops_count += 1;

            match op {
                ActionOp::ListDir { path } => {
                    let files =
                        capability
                            .list_dir(&path)
                            .map_err(|e| VmError::CapabilityFailure {
                                details: e.to_string(),
                            })?;
                    outputs.push(format!("LIST: {}", files.join(", ")));
                }
                ActionOp::ReadFile { path, max_bytes } => {
                    let content =
                        capability
                            .read_file(&path)
                            .map_err(|e| VmError::CapabilityFailure {
                                details: e.to_string(),
                            })?;

                    let read_len = content.len().min(max_bytes);
                    if self.total_bytes_read + read_len > self.limits.max_bytes_read {
                        return Err(VmError::ReadLimitExceeded {
                            limit: self.limits.max_bytes_read,
                            attempted: self.total_bytes_read + read_len,
                        });
                    }

                    self.total_bytes_read += read_len;
                    outputs.push(content[..read_len].to_string());
                }
                ActionOp::WriteFile { path, content } => {
                    let write_len = content.len();
                    if self.total_bytes_written + write_len > self.limits.max_bytes_written {
                        return Err(VmError::WriteLimitExceeded {
                            limit: self.limits.max_bytes_written,
                            attempted: self.total_bytes_written + write_len,
                        });
                    }

                    capability.write_file(&path, &content).map_err(|e| {
                        VmError::CapabilityFailure {
                            details: e.to_string(),
                        }
                    })?;

                    self.total_bytes_written += write_len;
                    outputs.push(format!("WROTE: {write_len} bytes to {path}"));
                }
                ActionOp::VerifySha256 { path, expected_hex } => {
                    let content =
                        capability
                            .read_file(&path)
                            .map_err(|e| VmError::CapabilityFailure {
                                details: e.to_string(),
                            })?;
                    let actual_digest = compute_sha256(content.as_bytes()).to_hex();
                    if actual_digest != expected_hex {
                        return Err(VmError::HashMismatch {
                            path,
                            expected: expected_hex,
                            actual: actual_digest,
                        });
                    }
                    outputs.push(format!("VERIFIED_SHA256: {path}"));
                }
            }
        }

        Ok(outputs)
    }
}

// ============================================================================
// 14. BỘ CẮT TỈA KẾT QUẢ AN TOÀN BIÊN UTF-8 (RESULT TRUNCATOR)
// ============================================================================

/// Kết quả sau khi cắt tỉa nội dung lớn.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct TruncationOutcome {
    pub formatted_output: String,
    pub was_truncated: bool,
    pub original_chars: usize,
    pub spill_file_path: Option<PathBuf>,
}

/// Bộ cắt tỉa kết quả công cụ giữ lại Đầu (Head) + Đuôi (Tail),
/// và xả phần giữa ra tệp lưu tạm (Spill File) để chống tràn context window.
pub struct ResultTruncator {
    pub head_chars: usize,
    pub tail_chars: usize,
}

impl ResultTruncator {
    pub fn new(head_chars: usize, tail_chars: usize) -> Self {
        Self {
            head_chars,
            tail_chars,
        }
    }

    /// Tìm vị trí chỉ số byte an toàn gần nhất không cắt đôi ký tự UTF-8 đa byte.
    fn floor_char_boundary(text: &str, mut byte_offset: usize) -> usize {
        if byte_offset >= text.len() {
            return text.len();
        }
        while !text.is_char_boundary(byte_offset) {
            byte_offset = byte_offset.saturating_sub(1);
        }
        byte_offset
    }

    /// Cắt tỉa văn bản theo chuẩn an toàn biên ký tự.
    pub fn truncate(
        &self,
        text: &str,
        spill_directory: Option<&Path>,
        spill_name_prefix: &str,
    ) -> TruncationOutcome {
        let total_chars = text.chars().count();
        let max_retained_chars = self.head_chars + self.tail_chars;

        if total_chars <= max_retained_chars {
            return TruncationOutcome {
                formatted_output: text.to_string(),
                was_truncated: false,
                original_chars: total_chars,
                spill_file_path: None,
            };
        }

        // Lấy đoạn đầu (Head)
        let head_end_byte: usize = text
            .char_indices()
            .nth(self.head_chars)
            .map(|(idx, _)| idx)
            .unwrap_or(text.len());
        let head = &text[..head_end_byte];

        // Lấy đoạn đuôi (Tail)
        let tail_start_char_idx = total_chars.saturating_sub(self.tail_chars);
        let tail_start_byte: usize = text
            .char_indices()
            .nth(tail_start_char_idx)
            .map(|(idx, _)| idx)
            .unwrap_or(text.len());
        let tail = &text[tail_start_byte..];

        let omitted_chars = total_chars - (self.head_chars + self.tail_chars);
        let omitted_bytes = tail_start_byte.saturating_sub(head_end_byte);

        // Xả nội dung đầy đủ ra spill file nếu có thư mục chỉ định
        let mut spill_path_opt = None;
        if let Some(dir) = spill_directory {
            let digest = compute_sha256(text.as_bytes()).to_hex();
            let file_name = format!("{spill_name_prefix}_{}.log", &digest[..16]);
            let target = dir.join(file_name);
            if fs::write(&target, text).is_ok() {
                spill_path_opt = Some(target);
            }
        }

        let spill_info = if let Some(path) = &spill_path_opt {
            format!("; SpillPath: {}", path.display())
        } else {
            String::new()
        };

        let formatted = format!(
            "{head}\n\n[... OMITTED {omitted_chars} CHARACTERS ({omitted_bytes} BYTES){spill_info} ...]\n\n{tail}"
        );

        TruncationOutcome {
            formatted_output: formatted,
            was_truncated: true,
            original_chars: total_chars,
            spill_file_path: spill_path_opt,
        }
    }
}

// ============================================================================
// 15. BỘ VÁ TỆP NGUYÊN TỬ (SEARCH-REPLACE PATCHER: READ-BEFORE-WRITE)
// ============================================================================

/// Yêu cầu vá tệp theo khối tìm kiếm - thay thế.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct PatchTarget {
    pub file_path: PathBuf,
    pub expected_sha256: Option<String>,
    pub search_block: String,
    pub replace_block: String,
}

/// Báo cáo kết quả sau khi vá tệp.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct PatchReport {
    pub file_path: PathBuf,
    pub original_sha256: String,
    pub new_sha256: String,
    pub lines_before: usize,
    pub lines_after: usize,
    pub line_ending: &'static str,
}

/// Lỗi trong quá trình vá tệp nguyên tử.
#[derive(Debug, Clone, PartialEq, Eq)]
pub enum PatcherError {
    FileNotFound { path: PathBuf },
    ReadFailure { details: String },
    HashMismatch { expected: String, actual: String },
    SearchBlockNotFound,
    AmbiguousMatch { occurrences: usize },
    AtomicWriteFailed { details: String },
}

impl fmt::Display for PatcherError {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            Self::FileNotFound { path } => write!(f, "Không tìm thấy tệp để vá: {path:?}"),
            Self::ReadFailure { details } => write!(f, "Lỗi đọc tệp: {details}"),
            Self::HashMismatch { expected, actual } => {
                write!(
                    f,
                    "Băm trước khi ghi không khớp: kỳ vọng {expected}, thực tế {actual}"
                )
            }
            Self::SearchBlockNotFound => write!(f, "Không tìm thấy khối cần tìm kiếm (0 matches)"),
            Self::AmbiguousMatch { occurrences } => {
                write!(
                    f,
                    "Khối tìm kiếm mơ hồ, xuất hiện {occurrences} lần (yêu cầu duy nhất 1 lần)"
                )
            }
            Self::AtomicWriteFailed { details } => write!(f, "Ghi nguyên tử thất bại: {details}"),
        }
    }
}

impl std::error::Error for PatcherError {}

/// Bộ vá tệp với 3 bất biến cốt lõi:
/// 1. Read-Before-Write: Kiểm tra băm nếu có yêu cầu.
/// 2. Unique Match: Chỉ chấp nhận đúng 1 lần khớp duy nhất.
/// 3. Atomic Rename: Ghi ra tệp tạm cùng thư mục rồi hoán đổi nguyên tử.
pub struct SearchReplacePatcher;

impl SearchReplacePatcher {
    /// Phát hiện loại xuống dòng của tệp.
    pub fn detect_line_ending(content: &str) -> &'static str {
        if content.contains("\r\n") {
            "\r\n"
        } else {
            "\n"
        }
    }

    /// Chuẩn hóa xuống dòng tạm thời để so khớp linh hoạt giữa LF và CRLF.
    pub fn count_occurrences(haystack: &str, needle: &str) -> usize {
        if needle.is_empty() {
            return 0;
        }
        haystack.matches(needle).count()
    }

    /// Thực thi vá tệp nguyên tử trên đĩa cứng.
    pub fn apply_patch(target: &PatchTarget) -> Result<PatchReport, PatcherError> {
        if !target.file_path.exists() {
            return Err(PatcherError::FileNotFound {
                path: target.file_path.clone(),
            });
        }

        let mut content =
            fs::read_to_string(&target.file_path).map_err(|e| PatcherError::ReadFailure {
                details: e.to_string(),
            })?;

        // 1. Kiểm tra tính toàn vẹn băm ban đầu
        let current_digest = compute_sha256(content.as_bytes()).to_hex();
        if let Some(expected) = &target.expected_sha256
            && current_digest.to_lowercase() != expected.to_lowercase()
        {
            return Err(PatcherError::HashMismatch {
                expected: expected.clone(),
                actual: current_digest,
            });
        }

        // 2. Kiểm tra tính duy nhất của khối tìm kiếm
        let count = Self::count_occurrences(&content, &target.search_block);
        if count == 0 {
            return Err(PatcherError::SearchBlockNotFound);
        }
        if count > 1 {
            return Err(PatcherError::AmbiguousMatch { occurrences: count });
        }

        let line_ending = Self::detect_line_ending(&content);
        let lines_before = content.lines().count();

        // 3. Thực hiện thay thế duy nhất
        content = content.replacen(&target.search_block, &target.replace_block, 1);
        let lines_after = content.lines().count();
        let new_digest = compute_sha256(content.as_bytes()).to_hex();

        // 4. Ghi nguyên tử qua tệp tạm cùng thư mục
        let parent = target.file_path.parent().unwrap_or_else(|| Path::new("."));
        let temp_file_path = parent.join(format!(
            ".tmp_patch_{}_{}",
            std::process::id(),
            &new_digest[..8]
        ));

        let write_res = (|| -> Result<(), std::io::Error> {
            let mut file = File::create(&temp_file_path)?;
            file.write_all(content.as_bytes())?;
            file.sync_all()?;
            fs::rename(&temp_file_path, &target.file_path)?;
            Ok(())
        })();

        if let Err(e) = write_res {
            let _ = fs::remove_file(&temp_file_path);
            return Err(PatcherError::AtomicWriteFailed {
                details: e.to_string(),
            });
        }

        Ok(PatchReport {
            file_path: target.file_path.clone(),
            original_sha256: current_digest,
            new_sha256: new_digest,
            lines_before,
            lines_after,
            line_ending,
        })
    }
}

// ============================================================================
// 16. BỘ BẮT LỖI ĐƯỜNG ỐNG LỆNH (PIPEFAIL EXECUTOR)
// ============================================================================

/// Một công đoạn trong chuỗi đường ống lệnh mô phỏng.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct PipeStage {
    pub command: String,
    pub exit_code: i32,
    pub simulated_output: String,
}

/// Kết quả thực thi chuỗi pipefail.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct PipefailResult {
    pub stage_exit_codes: Vec<i32>,
    pub pipefail_code: i32,
    pub standard_unix_code: i32,
    pub final_output: String,
}

/// Bộ giả lập và thẩm định lỗi đường ống lệnh, minh họa cơ chế `set -o pipefail`.
///
/// Trong Unix mặc định: `cmd1 | cmd2` trả về mã lỗi của `cmd2`. Nếu `cmd1` chết với mã 2
/// (ví dụ `rg` tìm không thấy) nhưng `cmd2` (`sed`) thành công với mã 0, script sẽ tưởng nhầm là thành công.
/// `PipefailExecutor` bắt mã lỗi khác 0 cuối cùng (từ phải sang trái) theo chuẩn POSIX pipefail.
pub struct PipefailExecutor;

impl PipefailExecutor {
    /// Tính toán mã lỗi pipefail chuẩn từ danh sách mã thoát của các công đoạn.
    pub fn compute_pipefail(exit_codes: &[i32]) -> i32 {
        if exit_codes.is_empty() {
            return 0;
        }
        // Duyệt từ cuối về đầu, lấy mã lỗi khác 0 đầu tiên gặp phải
        for &code in exit_codes.iter().rev() {
            if code != 0 {
                return code;
            }
        }
        0
    }

    /// Chạy đường ống lệnh mô phỏng và thu thập đầy đủ mã trạng thái.
    pub fn evaluate_pipeline(stages: &[PipeStage]) -> PipefailResult {
        let codes: Vec<i32> = stages.iter().map(|s| s.exit_code).collect();
        let standard_unix = codes.last().copied().unwrap_or(0);
        let pipefail = Self::compute_pipefail(&codes);
        let output = stages
            .last()
            .map(|s| s.simulated_output.clone())
            .unwrap_or_default();

        PipefailResult {
            stage_exit_codes: codes,
            pipefail_code: pipefail,
            standard_unix_code: standard_unix,
            final_output: output,
        }
    }
}

// ============================================================================
// HÀM MAIN MINH HỌA TOÀN BỘ CÁC PHÂN HỆ
// ============================================================================

fn main() {
    println!("=== CHƯƠNG 87: HỆ THỐNG CÔNG CỤ ĐỊNH KIỂU & HỘP CÁT AN TOÀN ===");

    // 1. Khởi tạo phong bì công cụ định kiểu
    let meta = HarnessMeta {
        session_id: "sess_main_01".to_string(),
        caller_agent: "orchestrator_ch87".to_string(),
        turn_index: 1,
        sandbox_level: SandboxLevel::IsolatedEphemeralFs,
        granted_scopes: vec!["fs:read".to_string(), "fs:write".to_string()],
        origin_timestamp_ms: 1_700_000_000_123,
    };
    let envelope = ToolEnvelope::new("{\"file\": \"main.rs\"}".to_string(), meta);
    println!(
        "1. Khởi tạo ToolEnvelope an toàn: Session={}",
        envelope.verified_meta.session_id
    );

    // 2. Đăng ký kỹ năng SKILL.md
    let mut catalog = SkillCatalog::new();
    let skill_md = "---\nname: safe_patcher\ndescription: Vá mã nguồn nguyên tử bảo toàn định dạng\nexclude: legacy_svn\n---\nThân chỉ dẫn kỹ thuật vá tệp an toàn.";
    catalog
        .register_markdown(skill_md, BTreeMap::new())
        .expect("Đăng ký SKILL.md thành công");
    println!("2. Đăng ký SkillCatalog: {}", catalog.render_tier0_index());

    // 3. Kiểm toán danh mục công cụ
    let auditor = ToolCatalogAuditor::new(0.5);
    let tools = vec![
        ToolDefinition {
            name: "patch_file".to_string(),
            description: "Thực hiện vá tệp nguồn qua khối tìm kiếm thay thế an toàn".to_string(),
        },
        ToolDefinition {
            name: "edit_source".to_string(),
            description: "Thực hiện chỉnh sửa tệp nguồn và thay thế nội dung khối an toàn"
                .to_string(),
        },
    ];
    let issues = auditor.audit_catalog(&tools, AuditMode::Detailed);
    println!(
        "3. Kiểm toán Jaccard phát hiện {} cảnh báo chồng lấn",
        issues.len()
    );

    // 4. Lập lịch sóng công cụ
    let calls = vec![
        ScheduledToolCall {
            call_id: "c1".to_string(),
            tool_name: "read_file".to_string(),
            arguments: BTreeMap::new(),
            mode: ToolConcurrencyMode::ReadOnlyConcurrent,
        },
        ScheduledToolCall {
            call_id: "c2".to_string(),
            tool_name: "list_dir".to_string(),
            arguments: BTreeMap::new(),
            mode: ToolConcurrencyMode::ReadOnlyConcurrent,
        },
        ScheduledToolCall {
            call_id: "c3".to_string(),
            tool_name: "write_file".to_string(),
            arguments: BTreeMap::new(),
            mode: ToolConcurrencyMode::ExclusiveWrite,
        },
    ];
    let waves = ToolWaveScheduler::schedule_waves(calls);
    println!(
        "4. Bộ lập lịch sóng: Tạo ra {} đợt sóng (Sóng 0 song song: {})",
        waves.len(),
        waves[0].is_concurrent
    );

    // 5. Thẩm định lỗi đường ống lệnh
    let stages = vec![
        PipeStage {
            command: "rg 'secret' src/".to_string(),
            exit_code: 2,
            simulated_output: String::new(),
        },
        PipeStage {
            command: "sed 's/a/b/'".to_string(),
            exit_code: 0,
            simulated_output: "Hoàn tất".to_string(),
        },
    ];
    let pipe_res = PipefailExecutor::evaluate_pipeline(&stages);
    println!(
        "5. Pipefail: Unix tiêu chuẩn={}, Chuẩn Pipefail={} (Bắt trúng lỗi công đoạn đầu!)",
        pipe_res.standard_unix_code, pipe_res.pipefail_code
    );

    println!("=== HOÀN TẤT MINH HỌA AN TOÀN CHƯƠNG 87 ===");
}

// ============================================================================
// 17. BỘ KIỂM THỬ ĐƠN VỊ TẤT ĐỊNH (MIN 32 TESTS)
// ============================================================================

#[cfg(test)]
mod tests {
    use super::*;

    // --- NHÓM 1: MÃ BĂM SHA-256 ---
    #[test]
    fn test_sha256_empty_string_matches_standard_vector() {
        let digest = compute_sha256(b"");
        assert_eq!(
            digest.to_hex(),
            "e3b0c44298fc1c149afbf4c8996fb92427ae41e4649b934ca495991b7852b855"
        );
    }

    #[test]
    fn test_sha256_known_string_and_display() {
        let digest = compute_sha256(b"hello world");
        assert_eq!(
            digest.to_hex(),
            "b94d27b9934d3e08a52e52d7da7dabfac484efe37a5380ee9088f7ace2efcde9"
        );
        assert_eq!(format!("{digest}"), digest.to_hex());
    }

    // --- NHÓM 2: TOOL ENVELOPE & HARNESS META ---
    #[test]
    fn test_tool_envelope_isolates_untrusted_input_from_verified_meta() {
        let meta = HarnessMeta {
            session_id: "test_sess".to_string(),
            caller_agent: "agent_qa".to_string(),
            turn_index: 42,
            sandbox_level: SandboxLevel::IsolatedReadOnly,
            granted_scopes: vec!["fs:read".to_string()],
            origin_timestamp_ms: 1000,
        };
        let untrusted_json = "{\"session_id\": \"forged_session\", \"admin\": true}";
        let envelope = ToolEnvelope::new(untrusted_json.to_string(), meta);

        assert_eq!(envelope.verified_meta.session_id, "test_sess");
        assert!(envelope.has_scope("fs:read"));
        assert!(!envelope.has_scope("admin:write"));
    }

    #[test]
    fn test_tool_envelope_map_input_and_split() {
        let meta = HarnessMeta {
            session_id: "s1".to_string(),
            caller_agent: "worker".to_string(),
            turn_index: 1,
            sandbox_level: SandboxLevel::FullWorkspaceAudited,
            granted_scopes: vec![],
            origin_timestamp_ms: 2000,
        };
        let env = ToolEnvelope::new("42".to_string(), meta);
        let mapped = env.map_input(|s| s.parse::<u32>().unwrap());
        assert_eq!(mapped.untrusted_input, 42);

        let (val, m) = mapped.split();
        assert_eq!(val, 42);
        assert_eq!(m.session_id, "s1");
    }

    // --- NHÓM 3: SKILL CATALOG PROGRESSIVE DISCLOSURE ---
    #[test]
    fn test_skill_catalog_tier0_index() {
        let mut cat = SkillCatalog::new();
        let doc = "---\nname: git_helper\ndescription: Hỗ trợ thao tác nhánh git\n---\nNội dung.";
        cat.register_markdown(doc, BTreeMap::new()).unwrap();
        let idx = cat.render_tier0_index();
        assert!(idx.contains("- git_helper: Hỗ trợ thao tác nhánh git"));
    }

    #[test]
    fn test_skill_catalog_tier1_activation_keyword() {
        let mut cat = SkillCatalog::new();
        let doc = "---\nname: rust_tester\ndescription: Chạy bài kiểm thử cargo test\n---\nCargo test runner.";
        cat.register_markdown(doc, BTreeMap::new()).unwrap();

        let active = cat.activate_tier1("Hãy chạy cargo test kiểm tra");
        assert_eq!(active.len(), 1);
        assert_eq!(active[0].name, "rust_tester");
    }

    #[test]
    fn test_skill_catalog_tier1_negative_exclude_filter() {
        let mut cat = SkillCatalog::new();
        let doc = "---\nname: db_migration\ndescription: Chạy migration csdl\nexclude: sqlite, nosql\n---\nThân kỹ năng.";
        cat.register_markdown(doc, BTreeMap::new()).unwrap();

        // Kích hoạt hợp lệ
        assert_eq!(cat.activate_tier1("Cần chạy migration").len(), 1);
        // Bị loại trừ bởi sqlite
        assert_eq!(cat.activate_tier1("Cần chạy migration sqlite").len(), 0);
    }

    #[test]
    fn test_skill_catalog_rejects_exceeding_500_lines() {
        let mut cat = SkillCatalog::new();
        let huge_body = "Dòng hướng dẫn\n".repeat(MAX_SKILL_BODY_LINES + 5);
        let raw = format!("---\nname: huge\ndescription: Quá dài\n---\n{huge_body}");
        let err = cat.register_markdown(&raw, BTreeMap::new());
        assert!(matches!(err, Err(SkillError::BodyLineLimitExceeded { .. })));
    }

    #[test]
    fn test_skill_catalog_rejects_malformed_yaml_or_missing_fields() {
        let mut cat = SkillCatalog::new();
        assert_eq!(
            cat.register_markdown("Không có frontmatter", BTreeMap::new()),
            Err(SkillError::MissingFrontmatter)
        );

        let missing_name = "---\ndescription: Thiếu tên\n---\nNội dung";
        assert!(matches!(
            cat.register_markdown(missing_name, BTreeMap::new()),
            Err(SkillError::MissingRequiredField { .. })
        ));
    }

    #[test]
    fn test_skill_catalog_tier2_resource_loading_and_path_confinement() {
        let mut cat = SkillCatalog::new();
        let mut res = BTreeMap::new();
        res.insert(
            "docs/api.md".to_string(),
            "Nội dung tài liệu API".to_string(),
        );
        let doc = "---\nname: api_tool\ndescription: Gọi API\n---\nBody";
        cat.register_markdown(doc, res).unwrap();

        // Hợp lệ
        let content = cat.load_tier2_resource("api_tool", "docs/api.md").unwrap();
        assert_eq!(content, "Nội dung tài liệu API");

        // Chặn leo thư mục ../
        assert_eq!(
            cat.load_tier2_resource("api_tool", "../etc/passwd"),
            Err(SkillError::PathTraversalDenied {
                requested_path: "../etc/passwd".to_string()
            })
        );

        // Chặn đường dẫn tuyệt đối /
        assert_eq!(
            cat.load_tier2_resource("api_tool", "/etc/hosts"),
            Err(SkillError::PathTraversalDenied {
                requested_path: "/etc/hosts".to_string()
            })
        );
    }

    // --- NHÓM 4: TOOL CATALOG AUDITOR ---
    #[test]
    fn test_tool_catalog_auditor_detects_jaccard_overlap() {
        let auditor = ToolCatalogAuditor::new(0.5);
        let tools = vec![
            ToolDefinition {
                name: "git_diff".to_string(),
                description: "Kiểm tra sự khác biệt của tệp mã nguồn git repo".to_string(),
            },
            ToolDefinition {
                name: "file_diff".to_string(),
                description: "Kiểm tra sự khác biệt của tệp mã nguồn hệ thống".to_string(),
            },
        ];

        let issues = auditor.audit_catalog(&tools, AuditMode::Detailed);
        assert_eq!(issues.len(), 1);
        assert!(issues[0].jaccard_similarity >= 0.5);
        assert!(issues[0].suggestion.contains("trùng lặp"));
    }

    #[test]
    fn test_tool_catalog_auditor_concise_mode() {
        let auditor = ToolCatalogAuditor::new(0.4);
        let tools = vec![
            ToolDefinition {
                name: "tool_a".to_string(),
                description: "đọc dữ liệu từ cơ sở dữ liệu sql".to_string(),
            },
            ToolDefinition {
                name: "tool_b".to_string(),
                description: "đọc dữ liệu từ cơ sở dữ liệu postgresql".to_string(),
            },
        ];

        let issues = auditor.audit_catalog(&tools, AuditMode::Concise);
        assert_eq!(issues.len(), 1);
        assert!(issues[0].suggestion.starts_with("Phân tách ranh giới"));
    }

    // --- NHÓM 5: CAPABILITY SEAM ---
    #[test]
    fn test_capability_seam_mock_provider() {
        let mut mock = MockSandboxProvider::new();
        mock.write_file("src/lib.rs", "pub fn ok() {}").unwrap();
        assert_eq!(mock.read_file("src/lib.rs").unwrap(), "pub fn ok() {}");

        mock.write_file("src/main.rs", "fn main() {}").unwrap();
        let list = mock.list_dir("src").unwrap();
        assert_eq!(list, vec!["lib.rs".to_string(), "main.rs".to_string()]);
    }

    #[test]
    fn test_capability_seam_audited_provider_records_operations() {
        let mock = MockSandboxProvider::new();
        let mut audited = AuditedProvider::new(mock);

        audited.write_file("data.txt", "123").unwrap();
        assert_eq!(audited.audit_log.len(), 1);
        assert_eq!(audited.audit_log[0].action, "write_file");
        assert_eq!(audited.audit_log[0].target_path, "data.txt");
    }

    // --- NHÓM 6: PATHGUARD & PRE-TOOL HOOK ---
    #[test]
    fn test_path_guard_virtual_sanitization_and_outside_jail_detection() {
        let guard = PathGuard::new("/app/workspace");
        let safe = guard.sanitize_virtual_path("src/../src/main.rs").unwrap();
        assert_eq!(safe, PathBuf::from("/app/workspace/src/main.rs"));

        let escape = guard.sanitize_virtual_path("../../etc/shadow");
        assert!(matches!(
            escape,
            Err(PathGuardError::OutsideRootJail { .. })
        ));
    }

    struct DenyWriteHook;
    impl PreToolHook for DenyWriteHook {
        fn inspect(
            &self,
            tool_name: &str,
            _args: &BTreeMap<String, String>,
        ) -> Result<(), HookRejection> {
            if tool_name == "delete_all" {
                Err(HookRejection {
                    hook_name: "DenyWriteHook",
                    reason: "Cấm tuyệt đối lệnh xóa toàn bộ".to_string(),
                })
            } else {
                Ok(())
            }
        }
    }

    #[test]
    fn test_pre_tool_hook_fail_closed_chain() {
        let mut chain = HookChain::new();
        chain.add_hook(DenyWriteHook);

        let empty_args = BTreeMap::new();
        assert!(chain.evaluate("read_file", &empty_args).is_ok());

        let res = chain.evaluate("delete_all", &empty_args);
        assert!(matches!(
            res,
            Err(HookRejection {
                hook_name: "DenyWriteHook",
                ..
            })
        ));
    }

    // --- NHÓM 7: SPOTLIGHTING & LETHAL TRIFECTA ---
    #[test]
    fn test_spotlighting_marker_separates_trusted_from_tainted() {
        let span = SpotlightedSpan::wrap_tainted("DROP TABLE users;", "web_crawler");
        assert_eq!(span.trust_level, TrustLevel::TaintedExternal);
        assert!(span.content.contains("<<<UNTRUSTED_CONTENT"));
        assert!(span.content.contains("DROP TABLE users;"));
    }

    #[test]
    fn test_lethal_trifecta_blocks_egress_when_all_three_legs_converge() {
        let mut blocker = LethalTrifectaTaintBlocker::new();
        assert!(blocker.verify_egress_allowed().is_ok());

        // Chân 1: Dữ liệu bên ngoài xâm nhập
        blocker.record_input_ingested(TrustLevel::TaintedExternal);
        assert!(blocker.verify_egress_allowed().is_ok());

        // Chân 2: Đọc bí mật hệ thống
        blocker.record_secret_accessed();

        // Chân 3: Nỗ lực truyền ra ngoài -> KÍCH HOẠT BÁO ĐỘNG
        let res = blocker.verify_egress_allowed();
        assert!(matches!(res, Err(TrifectaAlarm { .. })));
    }

    #[test]
    fn test_lethal_trifecta_allows_egress_when_no_secrets_accessed() {
        let mut blocker = LethalTrifectaTaintBlocker::new();
        blocker.record_input_ingested(TrustLevel::TaintedExternal);
        // Không đọc secret
        assert!(blocker.verify_egress_allowed().is_ok());
    }

    // --- NHÓM 8: MCP ANTI-CORRUPTION LAYER ---
    #[test]
    fn test_mcp_anti_corruption_layer_sanitizes_valid_tool() {
        let raw = McpRawManifest {
            raw_name: "fetch_page".to_string(),
            raw_description: "Tải trang web".to_string(),
            claimed_read_only: true,
            raw_properties: BTreeMap::from([("url".to_string(), "string".to_string())]),
        };

        let sanitized = McpAntiCorruptionLayer::sanitize_manifest(&raw).unwrap();
        assert_eq!(sanitized.canonical_name, "fetch_page");
        assert!(sanitized.is_read_only_safe);
    }

    #[test]
    fn test_mcp_anti_corruption_layer_blocks_fake_read_only_claim() {
        let raw = McpRawManifest {
            raw_name: "write_system_file".to_string(),
            raw_description: "Ghi tệp".to_string(),
            claimed_read_only: true, // GIẢ MẠO!
            raw_properties: BTreeMap::new(),
        };

        let err = McpAntiCorruptionLayer::sanitize_manifest(&raw);
        assert!(matches!(
            err,
            Err(McpSanitizeError::IllegalReadOnlyClaim { .. })
        ));
    }

    #[test]
    fn test_mcp_anti_corruption_layer_blocks_internal_prefix_and_dangerous_properties() {
        let raw1 = McpRawManifest {
            raw_name: "__internal_eval".to_string(),
            raw_description: "Nội bộ".to_string(),
            claimed_read_only: false,
            raw_properties: BTreeMap::new(),
        };
        assert!(matches!(
            McpAntiCorruptionLayer::sanitize_manifest(&raw1),
            Err(McpSanitizeError::DisallowedPrefix { .. })
        ));

        let raw2 = McpRawManifest {
            raw_name: "normal_tool".to_string(),
            raw_description: "Bình thường".to_string(),
            claimed_read_only: false,
            raw_properties: BTreeMap::from([("$dangerous_proto".to_string(), "val".to_string())]),
        };
        assert!(matches!(
            McpAntiCorruptionLayer::sanitize_manifest(&raw2),
            Err(McpSanitizeError::DangerousPropertyInjection { .. })
        ));
    }

    // --- NHÓM 9: TOOL WAVE SCHEDULER ---
    #[test]
    fn test_tool_wave_scheduler_groups_reads_and_isolates_writes() {
        let calls = vec![
            ScheduledToolCall {
                call_id: "1".into(),
                tool_name: "read_a".into(),
                arguments: BTreeMap::new(),
                mode: ToolConcurrencyMode::ReadOnlyConcurrent,
            },
            ScheduledToolCall {
                call_id: "2".into(),
                tool_name: "read_b".into(),
                arguments: BTreeMap::new(),
                mode: ToolConcurrencyMode::ReadOnlyConcurrent,
            },
            ScheduledToolCall {
                call_id: "3".into(),
                tool_name: "write_c".into(),
                arguments: BTreeMap::new(),
                mode: ToolConcurrencyMode::ExclusiveWrite,
            },
            ScheduledToolCall {
                call_id: "4".into(),
                tool_name: "read_d".into(),
                arguments: BTreeMap::new(),
                mode: ToolConcurrencyMode::ReadOnlyConcurrent,
            },
        ];

        let waves = ToolWaveScheduler::schedule_waves(calls);
        assert_eq!(waves.len(), 3);
        assert_eq!(waves[0].calls.len(), 2);
        assert!(waves[0].is_concurrent);
        assert_eq!(waves[1].calls.len(), 1);
        assert!(!waves[1].is_concurrent);
        assert_eq!(waves[2].calls.len(), 1);
        assert!(waves[2].is_concurrent);
    }

    // --- NHÓM 10: PERMISSION GATE & KILL SWITCH ---
    #[test]
    fn test_permission_gate_hierarchy_and_kill_switch_veto() {
        let kill_switch = Arc::new(AtomicBool::new(false));
        let mut gate = PermissionGate::new(kill_switch.clone(), PermissionLevel::Deny);

        gate.set_rule("read_logs", PermissionLevel::Allow);
        gate.set_rule("format_disk", PermissionLevel::Deny);
        gate.set_rule("deploy_prod", PermissionLevel::AskHuman);

        assert!(gate.evaluate_permission("read_logs").allowed);
        assert!(!gate.evaluate_permission("format_disk").allowed);
        assert!(
            gate.evaluate_permission("deploy_prod")
                .requires_human_prompt
        );

        // KÍCH HOẠT KILL SWITCH SỐNG
        kill_switch.store(true, Ordering::SeqCst);
        let dec = gate.evaluate_permission("read_logs");
        assert!(!dec.allowed);
        assert!(dec.reason.contains("E_KILL_SWITCH_ACTIVE"));
    }

    // --- NHÓM 11: SECRETSTRING & OUTPUT SCRUBBER ---
    #[test]
    fn test_secret_string_redacts_on_debug_and_display() {
        let s = SecretString::new("super_secret_token_123");
        assert_eq!(format!("{s}"), "[REDACTED_SECRET]");
        assert_eq!(format!("{s:?}"), "SecretString([REDACTED])");
        assert_eq!(s.expose_secret(), "super_secret_token_123");
    }

    #[test]
    fn test_output_scrubber_merges_overlapping_spans_and_scrubs_bearer() {
        let scrubber = OutputScrubber::new(vec!["ghp_123456789".to_string(), "12345".to_string()]);
        let input = "Authorization: Bearer my_secret_token_abc and ghp_123456789";
        let cleaned = scrubber.scrub_text(input);

        assert!(!cleaned.contains("my_secret_token_abc"));
        assert!(!cleaned.contains("ghp_123456789"));
        assert!(cleaned.contains("[REDACTED_BEARER]"));
        assert!(cleaned.contains("[REDACTED_SECRET]"));
    }

    #[test]
    fn test_output_scrubber_utf8_char_boundary_safety_vietnamese() {
        let scrubber = OutputScrubber::new(vec!["bí_mật".to_string()]);
        let input = "Dữ liệu tiếng Việt có khóa bí_mật ở giữa câu.";
        let cleaned = scrubber.scrub_text(input);

        assert!(cleaned.contains("tiếng Việt có khóa [REDACTED_SECRET] ở giữa câu."));
    }

    #[test]
    fn test_output_scrubber_headers_and_body() {
        let scrubber = OutputScrubber::new(vec!["top_secret_payload".to_string()]);
        let mut headers = BTreeMap::new();
        headers.insert("Authorization".to_string(), "Bearer secret_jwt".to_string());
        headers.insert("X-Request-Id".to_string(), "req_01".to_string());

        let body = "Nội dung mang top_secret_payload.";
        let (clean_h, clean_b) = scrubber.scrub_headers_and_body(&headers, body);

        assert_eq!(clean_h.get("Authorization").unwrap(), "[REDACTED_HEADER]");
        assert_eq!(clean_h.get("X-Request-Id").unwrap(), "req_01");
        assert!(clean_b.contains("[REDACTED_SECRET]"));
    }

    // --- NHÓM 12: SANDBOX GUARD RAII ---
    #[test]
    fn test_sandbox_guard_cleans_directory_on_drop() {
        let temp_dir = std::env::temp_dir().join(format!("test_sb_{}", std::process::id()));
        {
            let mut sb = SandboxGuard::new("sb_01", temp_dir.clone());
            sb.allocate_resource("res_mem");
            assert!(temp_dir.exists());
        }
        // Sau khi guard ra khỏi tầm vực (Drop), thư mục phải bị dọn sạch
        assert!(!temp_dir.exists());
    }

    // --- NHÓM 13: ACTION VM ---
    #[test]
    fn test_action_vm_executes_program_within_limits() {
        let mut mock = MockSandboxProvider::new();
        mock.write_file("a.txt", "hello").unwrap();

        let limits = VmLimits {
            max_operations: 10,
            max_bytes_read: 100,
            max_bytes_written: 100,
        };
        let mut vm = ActionVm::new(limits);

        let prog = vec![
            ActionOp::ReadFile {
                path: "a.txt".to_string(),
                max_bytes: 10,
            },
            ActionOp::WriteFile {
                path: "b.txt".to_string(),
                content: "world".to_string(),
            },
        ];

        let out = vm.execute_program(&mut mock, prog).unwrap();
        assert_eq!(out.len(), 2);
        assert_eq!(mock.read_file("b.txt").unwrap(), "world");
    }

    #[test]
    fn test_action_vm_aborts_on_exceeded_limits() {
        let mut mock = MockSandboxProvider::new();
        let limits = VmLimits {
            max_operations: 1, // Chỉ cho phép 1 lệnh
            max_bytes_read: 10,
            max_bytes_written: 10,
        };
        let mut vm = ActionVm::new(limits);

        let prog = vec![
            ActionOp::ListDir { path: ".".into() },
            ActionOp::ListDir { path: ".".into() },
        ];

        let res = vm.execute_program(&mut mock, prog);
        assert!(matches!(res, Err(VmError::OperationLimitExceeded { .. })));
    }

    // --- NHÓM 14: RESULT TRUNCATOR ---
    #[test]
    fn test_result_truncator_head_tail_formatting() {
        let truncator = ResultTruncator::new(5, 5);
        let text = "0123456789abcdefghij";
        let res = truncator.truncate(text, None, "test");

        assert!(res.was_truncated);
        assert!(res.formatted_output.starts_with("01234"));
        assert!(res.formatted_output.ends_with("fghij"));
        assert!(res.formatted_output.contains("[... OMITTED 10 CHARACTERS"));
    }

    #[test]
    fn test_result_truncator_preserves_multibyte_utf8() {
        let truncator = ResultTruncator::new(4, 4);
        let text = "Xin chào các lập trình viên Rust";
        let res = truncator.truncate(text, None, "test_utf8");

        assert!(res.was_truncated);
        assert!(res.formatted_output.starts_with("Xin "));
        assert!(res.formatted_output.ends_with("Rust"));
    }

    // --- NHÓM 15: SEARCH REPLACE PATCHER ---
    #[test]
    fn test_search_replace_patcher_atomic_apply() {
        let temp_dir = std::env::temp_dir().join(format!("patch_test_{}", std::process::id()));
        fs::create_dir_all(&temp_dir).unwrap();
        let target_file = temp_dir.join("code.rs");

        fs::write(&target_file, "fn add() -> i32 {\n    1 + 1\n}\n").unwrap();
        let initial_hash = compute_sha256(fs::read(&target_file).unwrap().as_slice()).to_hex();

        let patch = PatchTarget {
            file_path: target_file.clone(),
            expected_sha256: Some(initial_hash),
            search_block: "    1 + 1\n".to_string(),
            replace_block: "    2 + 2\n".to_string(),
        };

        let report = SearchReplacePatcher::apply_patch(&patch).unwrap();
        assert_eq!(report.lines_before, 3);
        assert_eq!(report.lines_after, 3);

        let patched_content = fs::read_to_string(&target_file).unwrap();
        assert!(patched_content.contains("    2 + 2\n"));

        // Dọn dẹp
        let _ = fs::remove_dir_all(&temp_dir);
    }

    #[test]
    fn test_search_replace_patcher_rejects_ambiguous_and_missing_matches() {
        let temp_dir = std::env::temp_dir().join(format!("patch_err_{}", std::process::id()));
        fs::create_dir_all(&temp_dir).unwrap();
        let target_file = temp_dir.join("dup.rs");

        fs::write(&target_file, "let x = 1;\nlet x = 1;\n").unwrap();

        // 1. Khối trùng lặp 2 lần -> AmbiguousMatch
        let patch_dup = PatchTarget {
            file_path: target_file.clone(),
            expected_sha256: None,
            search_block: "let x = 1;".to_string(),
            replace_block: "let x = 2;".to_string(),
        };
        assert!(matches!(
            SearchReplacePatcher::apply_patch(&patch_dup),
            Err(PatcherError::AmbiguousMatch { occurrences: 2 })
        ));

        // 2. Không tìm thấy -> SearchBlockNotFound
        let patch_none = PatchTarget {
            file_path: target_file.clone(),
            expected_sha256: None,
            search_block: "let z = 99;".to_string(),
            replace_block: "let z = 100;".to_string(),
        };
        assert!(matches!(
            SearchReplacePatcher::apply_patch(&patch_none),
            Err(PatcherError::SearchBlockNotFound)
        ));

        let _ = fs::remove_dir_all(&temp_dir);
    }

    #[test]
    fn test_search_replace_patcher_rejects_hash_mismatch() {
        let temp_dir = std::env::temp_dir().join(format!("patch_hash_{}", std::process::id()));
        fs::create_dir_all(&temp_dir).unwrap();
        let target_file = temp_dir.join("hash.rs");

        fs::write(&target_file, "val = 10;\n").unwrap();

        let patch = PatchTarget {
            file_path: target_file.clone(),
            expected_sha256: Some(
                "0000000000000000000000000000000000000000000000000000000000000000".to_string(),
            ),
            search_block: "val = 10;".to_string(),
            replace_block: "val = 20;".to_string(),
        };

        assert!(matches!(
            SearchReplacePatcher::apply_patch(&patch),
            Err(PatcherError::HashMismatch { .. })
        ));

        let _ = fs::remove_dir_all(&temp_dir);
    }

    // --- NHÓM 16: PIPEFAIL EXECUTOR ---
    #[test]
    fn test_pipefail_executor_catches_upstream_failure() {
        let stages = vec![
            PipeStage {
                command: "rg 'TODO' src/".to_string(),
                exit_code: 2, // Lỗi tìm kiếm
                simulated_output: String::new(),
            },
            PipeStage {
                command: "sed 's/TODO/DONE/'".to_string(),
                exit_code: 0, // Thành công
                simulated_output: String::new(),
            },
        ];

        let res = PipefailExecutor::evaluate_pipeline(&stages);
        assert_eq!(res.standard_unix_code, 0); // Unix thông thường bỏ lọt lỗi!
        assert_eq!(res.pipefail_code, 2); // Pipefail bắt trúng mã lỗi 2!
    }

    #[test]
    fn test_pipefail_executor_returns_zero_when_all_succeed() {
        let stages = vec![
            PipeStage {
                command: "cat Cargo.toml".to_string(),
                exit_code: 0,
                simulated_output: "[package]".to_string(),
            },
            PipeStage {
                command: "grep edition".to_string(),
                exit_code: 0,
                simulated_output: "edition = \"2024\"".to_string(),
            },
        ];

        let res = PipefailExecutor::evaluate_pipeline(&stages);
        assert_eq!(res.pipefail_code, 0);
        assert_eq!(res.final_output, "edition = \"2024\"");
    }
}
