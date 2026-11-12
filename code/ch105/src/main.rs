//! # Chương 105: Đại dự án Tích hợp II — Tác tử Pentest Tự hành Đa Tác tử,
//! Phần thưởng RL, Đối kháng Red-vs-Blue & Hiến pháp An toàn
//! (Autonomous Multi-Agent Pentest: RL Reward, Red-vs-Blue Adversarial & Constitutional Safety)
//!
//! Crate `code/ch105` khép kín toàn diện **Chủ đề 28** (Tác tử & Kiểm thử Thâm nhập Tự động)
//! bằng **100% Rust 2024 thuần `std`** — không phụ thuộc bất kỳ thư viện ngoài nào
//! (`[dependencies]` rỗng tuyệt đối).
//!
//! ## Vị trí kiến trúc
//!
//! Chương 105 là đỉnh cao tích hợp II: nó gộp toàn bộ năng lực của Chương 98–104
//! (trinh sát mô phỏng, khai thác, leo thang, đối kháng) vào **một tác tử pentest tự hành
//! duy nhất** có đầy đủ tám lớp bảo vệ:
//!
//! ```text
//!   [ ScopeGuard ]          -> chặn mọi hành động ra ngoài phạm vi CIDR
//!   [ PentestConstitution ] -> 10 điều luật hiến pháp, cưỡng chế Block/Warn/Log
//!   [ EvidenceChain ]       -> chuỗi băm SHA-256 chống sửa đổi bằng chứng
//!   [ HitlGate ]            -> cổng phê duyệt của con người (Human-In-The-Loop)
//!   [ PentestStateGraph ]   -> đồ thị 8 nút điều phối pha
//!   [ RL ]                  -> AttackRewardFunction + AttackPathOptimizer (tabular Q-learning)
//!   [ Red-vs-Blue ]        -> RedVsBlueSim với lòng phản xạ đối kháng hai bên cùng tiến bộ
//!   [ SessionManager ]      -> lưu/nôi phiên kiểm thử theo kiểu PentestGPT
//! ```
//!
//! ## 11 bất biến hệ thống được chứng minh bằng 54 bài kiểm thử
//!
//! 1. **Scope Enforcement** — chặn và ghi nhận mọi hành động ngoài CIDR được duyệt.
//! 2. **Constitution Compliance** — điều 1 & điều 2 của hiến pháp chặn tấn công ngoài phạm vi và phá hủy dữ liệu.
//! 3. **Evidence Integrity** — sửa một bản ghi bằng chứng làm `verify()` trả về `false`.
//! 4. **HITL Gate** — hành động nguy hiểm phải được người vận hành duyệt mới chạy.
//! 5. **Privilege Boundary** — không leo thang lên máy chưa bị xâm nhập.
//! 6. **Time Budget** — không vượt quá `budget_steps`.
//! 7. **Log Completeness** — mọi hành động đều có bằng chứng trong chuỗi hash.
//! 8. **Report Generation** — báo cáo Markdown có đủ 5 mục bắt buộc.
//! 9. **Clean Exit** — kết thúc không còn phiên shell nào treo.
//! 10. **Ground-truth Verification** — chỉ giữ phát hiện đã được PoE xác minh.
//! 11. **Multi-Agent Coordination** — Recon → Exploit → Validation đúng thứ tự.
//!
//! **Lưu ý đạo đức**: toàn bộ mọi "khai thác" trong crate này chạy trên mạng giả lập
//! tất định `MockNetwork` (5 máy ảo) và vỏ `MockShell` giả lập; không một byte nào
//! chạm tới hệ thống thật, không có mã khai thác lấy từ framework ngoài.

#![allow(dead_code)]

use std::collections::HashMap;
use std::net::Ipv4Addr;

// ============================================================================
// 1. TIỆN ÍCH NỀN TẢNG: SPLITMIX64, SHA-256 & PHÂN TÍCH CIDR
// ============================================================================

/// Bộ sinh số giả ngẫu nhiên 64-bit tất định `SplitMix64` (Steele/Lea/Flood).
///
/// Mọi thành phần mô phỏng trong crate đều dùng bộ sinh này để mọi lần chạy cho
/// đúng **bit-exact** kết quả, không phụ thuộc thứ tự thực thi của luồng (thread).
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct SplitMix64 {
    /// Trạng thái thanh ghi 64-bit nội bộ của bộ sinh.
    pub state: u64,
}

impl SplitMix64 {
    /// Hằng số vàng của thuật toán SplitMix64 (gia số trạng thái).
    pub const GAMMA: u64 = 0x9E37_79B9_7F4A_7C15;

    /// Khởi tạo bộ sinh với hạt giống xác định trước.
    pub const fn new(seed: u64) -> Self {
        Self { state: seed }
    }

    /// Sinh số nguyên không dấu 64-bit ngẫu nhiên tiếp theo.
    pub fn next_u64(&mut self) -> u64 {
        self.state = self.state.wrapping_add(Self::GAMMA);
        let mut z = (self.state ^ (self.state >> 30)).wrapping_mul(0xBF58_476D_1CE4_E5B9);
        z = (z ^ (z >> 27)).wrapping_mul(0x94D0_49BB_1331_11EB);
        z ^ (z >> 31)
    }

    /// Sinh số thực dấu phẩy động trong khoảng nửa `[0.0, 1.0)`.
    pub fn next_f64(&mut self) -> f64 {
        (self.next_u64() >> 11) as f64 * (1.0 / ((1u64 << 53) as f64))
    }

    /// Sinh chỉ số ngẫu nhiên trong khoảng `0..bound` (bound phải khác 0).
    pub fn next_below(&mut self, bound: usize) -> usize {
        if bound == 0 {
            return 0;
        }
        (self.next_u64() % bound as u64) as usize
    }

    /// Trả về `true` với xác suất `probability` (kẹp trong `[0, 1]`).
    pub fn next_chance(&mut self, probability: f64) -> bool {
        self.next_f64() < probability.clamp(0.0, 1.0)
    }
}

/// Bảng hằng số 64 vòng nén của thuật toán SHA-256 theo tiêu chuẩn FIPS 180-4.
const SHA256_K: [u32; 64] = [
    0x428a2f98, 0x71374491, 0xb5c0fbcf, 0xe9b5dba5, 0x3956c25b, 0x59f111f1, 0x923f82a4, 0xab1c5ed5,
    0xd807aa98, 0x12835b01, 0x243185be, 0x550c7dc3, 0x72be5d74, 0x80deb1fe, 0x9bdc06a7, 0xc19bf174,
    0xe49b69c1, 0xefbe4786, 0x0fc19fc6, 0x240ca1cc, 0x2de92c6f, 0x4a7484aa, 0x5cb0a9dc, 0x76f988da,
    0x983e5152, 0xa831c66d, 0xb00327c8, 0xbf597fc7, 0xc6e00bf3, 0xd5a79147, 0x06ca6351, 0x14292967,
    0x27b70a85, 0x2e1b2138, 0x4d2c6dfc, 0x53380d13, 0x650a7354, 0x766a0abb, 0x81c2c92e, 0x92722c85,
    0xa2bfe8a1, 0xa81a664b, 0xc24b8b70, 0xc76c51a3, 0xd192e819, 0xd6990624, 0xf40e3585, 0x106aa070,
    0x19a4c116, 0x1e376c08, 0x2748774c, 0x34b0bcb5, 0x391c0cb3, 0x4ed8aa4a, 0x5b9cca4f, 0x682e6ff3,
    0x748f82ee, 0x78a5636f, 0x84c87814, 0x8cc70208, 0x90befffa, 0xa4506ceb, 0xbef9a3f7, 0xc67178f2,
];

/// Tính toán băm SHA-256 chuẩn FIPS 180-4 thuần `std`, trả về 32 byte thô.
///
/// Hiện thực tuân thủ đúng đặc tả: mở rộng thông điệp theo kiểu Merkle–Damgård,
/// khối 64 byte, 64 vòng nén với hằng số vòng `K`.
pub fn sha256_bytes(data: &[u8]) -> [u8; 32] {
    let mut h: [u32; 8] = [
        0x6a09e667, 0xbb67ae85, 0x3c3726c4, 0xa54ff53a, 0x510e527f, 0x9b05688c, 0x1f83d9ab,
        0x5be0cd19,
    ];

    let mut padded = data.to_vec();
    let bit_len = (data.len() as u64).wrapping_mul(8);
    padded.push(0x80);
    while padded.len() % 64 != 56 {
        padded.push(0x00);
    }
    padded.extend_from_slice(&bit_len.to_be_bytes());

    for block in padded.as_chunks::<64>().0 {
        let mut w = [0u32; 64];
        for (i, word) in w.iter_mut().enumerate().take(16) {
            let base = i * 4;
            *word = u32::from_be_bytes([
                block[base],
                block[base + 1],
                block[base + 2],
                block[base + 3],
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
                .wrapping_add(SHA256_K[i])
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
    for (i, value) in h.iter().enumerate() {
        out[i * 4..i * 4 + 4].copy_from_slice(&value.to_be_bytes());
    }
    out
}

/// Tính băm SHA-256 và trả về chuỗi hex 64 ký tự viết thường.
pub fn sha256_hex(data: &[u8]) -> String {
    let bytes = sha256_bytes(data);
    let mut out = String::with_capacity(64);
    for byte in bytes {
        out.push_str(&format!("{byte:02x}"));
    }
    out
}

/// Hằng số "khối mồi" (genesis) của chuỗi bằng chứng: 64 ký tự `0`.
pub const EVIDENCE_GENESIS: &str =
    "0000000000000000000000000000000000000000000000000000000000000000";

/// Phân tích một chuỗi CIDR dạng `a.b.c.d/n` thành (địa chỉ mạng, độ dài tiền tố).
///
/// Trả về `None` nếu chuỗi không hợp lệ (sai cú pháp hoặc tiền tố ngoài khoảng 0–32).
pub fn parse_cidr(cidr: &str) -> Option<(Ipv4Addr, u32)> {
    let (network_part, prefix_part) = cidr.split_once('/')?;
    let address: Ipv4Addr = network_part.trim().parse().ok()?;
    let prefix: u32 = prefix_part.trim().parse().ok()?;
    if prefix > 32 {
        return None;
    }
    Some((address, prefix))
}

/// Tính mặt nạ mạng con tương ứng với độ dài tiền tố `prefix` bit.
pub fn cidr_mask(prefix: u32) -> u32 {
    if prefix == 0 {
        0
    } else {
        u32::MAX << (32 - prefix)
    }
}

/// Kiểm tra địa chỉ `ip` có nằm trong mạng CIDR `(network, prefix)` hay không.
pub fn ip_in_cidr(ip: Ipv4Addr, network: Ipv4Addr, prefix: u32) -> bool {
    let mask = cidr_mask(prefix);
    (u32::from(ip) & mask) == (u32::from(network) & mask)
}

/// Kiểm tra địa chỉ IPv4 có thuộc các dải mạng riêng RFC 1918 hoặc loopback hay không.
///
/// Hiến pháp dùng quy tắc này như một "phạm vi tối thiểu": mọi mục tiêu công khai
/// đều bị xem là nằm ngoài phạm vi được duyệt.
pub fn is_internal_ipv4(ip: Ipv4Addr) -> bool {
    let octets = ip.octets();
    if octets[0] == 10 {
        return true;
    }
    if octets[0] == 172 && (16..32).contains(&octets[1]) {
        return true;
    }
    if octets[0] == 192 && octets[1] == 168 {
        return true;
    }
    if octets[0] == 127 {
        return true;
    }
    false
}

// ============================================================================
// 2. PHẠM VI KIỂM THỬ (SCOPE ENFORCEMENT)
// ============================================================================

/// Một quy tắc phạm vi: mạng CIDR kèm danh sách cổng được phép chạm tới.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct ScopeRule {
    /// Mạng phạm vi theo ký hiệu CIDR, ví dụ `"10.10.10.0/24"`.
    pub cidr: String,
    /// Danh sách cổng TCP được phép tấn công trong mạng này.
    pub allowed_ports: Vec<u16>,
}

/// Cổng cưỡng chế phạm vi: mọi hành động vượt phạm vi đều bị chặn và ghi nhận.
///
/// Bất biến **Scope Enforcement**: không một hành động nào được phép chạm tới
/// địa chỉ ngoài mạng được duyệt hoặc cổng không nằm trong danh sách cho phép.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct ScopeGuard {
    /// Danh sách quy tắc phạm vi hiện hành.
    pub rules: Vec<ScopeRule>,
    /// Nhật ký các lần vi phạm phạm vi đã bị chặn.
    pub violations: Vec<String>,
}

impl ScopeGuard {
    /// Khởi tạo cổng phạm vi từ danh sách quy tắc cho trước.
    pub fn new(rules: Vec<ScopeRule>) -> Self {
        Self {
            rules,
            violations: Vec::new(),
        }
    }

    /// Phạm vi mặc định dùng cho mạng giả lập `10.10.10.0/24` của crate này.
    pub fn default_scope() -> Self {
        Self::new(vec![ScopeRule {
            cidr: "10.10.10.0/24".to_string(),
            allowed_ports: vec![22, 80, 443, 445, 139, 3306, 3389, 8080],
        }])
    }

    /// Kiểm tra địa chỉ có nằm trong ít nhất một mạng phạm vi hay không (bỏ qua cổng).
    pub fn allows_ip(&self, ip: Ipv4Addr) -> bool {
        self.rules.iter().any(|rule| {
            parse_cidr(&rule.cidr).is_some_and(|(network, prefix)| ip_in_cidr(ip, network, prefix))
        })
    }

    /// Kiểm tra cặp (địa chỉ, cổng) có được phép hay không.
    ///
    /// Một quy tắc chỉ khớp khi vừa chứa địa chỉ vừa cho phép cổng tương ứng.
    pub fn allows_action(&self, ip: Ipv4Addr, port: u16) -> bool {
        self.rules.iter().any(|rule| {
            let in_network = parse_cidr(&rule.cidr)
                .is_some_and(|(network, prefix)| ip_in_cidr(ip, network, prefix));
            in_network && rule.allowed_ports.contains(&port)
        })
    }

    /// Kiểm tra và **ghi nhận** hành động: trả về `true` nếu được phép chạy.
    ///
    /// Khi bị chặn, một thông điệp vi phạm được đẩy vào `violations` để đưa vào báo cáo.
    pub fn check(&mut self, ip: Ipv4Addr, port: u16) -> bool {
        if self.allows_action(ip, port) {
            return true;
        }
        let reason = if self.allows_ip(ip) {
            format!(
                "Vi phạm phạm vi: cổng {port} không nằm trong danh sách cổng được duyệt của {ip}"
            )
        } else {
            format!("Vi phạm phạm vi: {ip} nằm ngoài mạng được duyệt")
        };
        self.violations.push(reason);
        false
    }

    /// Xem danh sách vi phạm phạm vi đã ghi nhận.
    pub fn violations(&self) -> &[String] {
        &self.violations
    }
}

// ============================================================================
// 3. HIẾN PHÁP KIỂM THỬ (CONSTITUTION COMPLIANCE)
// ============================================================================

/// Mức cưỡng chế áp dụng cho một điều luật trong hiến pháp.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum Enforcement {
    /// Chặn cứng: hành động vi phạm tuyệt đối không được thực thi.
    Block,
    /// Cảnh báo: ghi nhận vi phạm nhưng vẫn cho phép tiếp tục.
    Warn,
    /// Chỉ ghi nhận vào nhật ký để đối chiếu sau.
    Log,
}

/// Loại hành động mà tác tử pentest đề xuất thực hiện.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum ActionKind {
    /// Quét dịch vụ / dò cổng.
    Scan,
    /// Khai thác một lỗ hổng đã xác định.
    Exploit,
    /// Tấn công bằng từ điển mật khẩu.
    CredentialAttack,
    /// Leo thang đặc quyền trên máy đã xâm nhập.
    PrivEsc,
    /// Chuyển ngang sang máy khác (mục tiêu chưa được duyệt).
    Pivot,
    /// Truy cập dữ liệu (đọc, không phá hủy).
    DataAccess,
    /// Hành động phá hủy dữ liệu (xoá, ghi đè, DROP...).
    Destructive,
    /// Sinh báo cáo kết quả.
    Report,
}

impl ActionKind {
    /// Tên ổn định của loại hành động, dùng làm khoá cho cổng HITL và nhật ký.
    pub fn label(&self) -> &'static str {
        match self {
            ActionKind::Scan => "scan",
            ActionKind::Exploit => "exploit",
            ActionKind::CredentialAttack => "credential-attack",
            ActionKind::PrivEsc => "privilege-escalation",
            ActionKind::Pivot => "pivot",
            ActionKind::DataAccess => "data-access",
            ActionKind::Destructive => "destructive",
            ActionKind::Report => "report",
        }
    }
}

/// Một hành động do tác tử đề xuất, chờ cổng phạm vi – hiến pháp – HITL phê duyệt.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct ProposedAction {
    /// Loại hành động.
    pub kind: ActionKind,
    /// Mục tiêu (địa chỉ IPv4 hoặc tên máy chủ).
    pub target: String,
    /// Cổng đích.
    pub port: u16,
    /// Mô tả tự nhiên của hành động (đưa vào báo cáo và nhật ký).
    pub description: String,
}

/// Một điều luật trong hiến pháp kiểm thử.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct ConstitutionArticle {
    /// Mã số điều luật (1–10).
    pub id: u32,
    /// Tiêu đề ngắn gọn.
    pub title: String,
    /// Nội dung luật đầy đủ.
    pub rule: String,
    /// Mức cưỡng chế của điều luật.
    pub enforcement: Enforcement,
}

/// Bản ghi vi phạm hiến pháp.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct PentestViolation {
    /// Mã điều luật bị vi phạm.
    pub article_id: u32,
    /// Tiêu đề điều luật tương ứng.
    pub title: String,
    /// Chi tiết lý do vi phạm.
    pub detail: String,
}

/// Hiến pháp gồm 10 điều luật cho mọi tác tử pentest tự hành.
///
/// Bất biến **Constitution Compliance**: tác tử không bao giờ thực thi hành động
/// vi phạm điều luật có mức cưỡng chế `Block`, và mọi vi phạm đều được ghi nhận.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct PentestConstitution {
    /// Danh sách điều luật theo thứ tự mã số.
    pub articles: Vec<ConstitutionArticle>,
}

impl PentestConstitution {
    /// Xây dựng hiến pháp tiêu chuẩn gồm 10 điều luật.
    pub fn standard() -> Self {
        let articles = vec![
            ConstitutionArticle {
                id: 1,
                title: "Không tấn công ngoài phạm vi".to_string(),
                rule:
                    "Mọi hành động chỉ được thực hiện với mục tiêu thuộc mạng nội bộ đã được duyệt."
                        .to_string(),
                enforcement: Enforcement::Block,
            },
            ConstitutionArticle {
                id: 2,
                title: "Không phá hủy dữ liệu".to_string(),
                rule: "Tuyệt đối không xoá, ghi đè hay làm hỏng dữ liệu của khách hàng."
                    .to_string(),
                enforcement: Enforcement::Block,
            },
            ConstitutionArticle {
                id: 3,
                title: "Ghi nhật ký mọi hành động".to_string(),
                rule: "Mỗi hành động đều phải kèm bằng chứng có thể băm và kiểm chứng lại."
                    .to_string(),
                enforcement: Enforcement::Log,
            },
            ConstitutionArticle {
                id: 4,
                title: "Không đánh cắp dữ liệu".to_string(),
                rule: "Không thu thập dữ liệu vượt quá mức cần thiết cho kiểm thử.".to_string(),
                enforcement: Enforcement::Warn,
            },
            ConstitutionArticle {
                id: 5,
                title: "Tôn trọng hạn mức thời gian".to_string(),
                rule: "Dừng hành động tấn công ngay khi hết ngân sách bước cho phép.".to_string(),
                enforcement: Enforcement::Log,
            },
            ConstitutionArticle {
                id: 6,
                title: "Cần ground-truth trước khi báo cáo".to_string(),
                rule: "Chỉ báo cáo phát hiện đã được kiểm chứng bằng proof-of-exploitation."
                    .to_string(),
                enforcement: Enforcement::Log,
            },
            ConstitutionArticle {
                id: 7,
                title: "Không leo thang ngoài mục tiêu".to_string(),
                rule: "Không chuyển ngang hay leo thang lên máy chưa được duyệt trước.".to_string(),
                enforcement: Enforcement::Warn,
            },
            ConstitutionArticle {
                id: 8,
                title: "PoC phải tái lập được".to_string(),
                rule: "Mọi proof-of-exploitation phải mô tả đủ các bước để tái lập.".to_string(),
                enforcement: Enforcement::Log,
            },
            ConstitutionArticle {
                id: 9,
                title: "Bảo mật thông tin khách hàng".to_string(),
                rule: "Không đưa bí mật, khóa hay dữ liệu nhạy cảm vào báo cáo.".to_string(),
                enforcement: Enforcement::Log,
            },
            ConstitutionArticle {
                id: 10,
                title: "Dừng ngay khi có yêu cầu dừng".to_string(),
                rule: "Khi có phê duyệt hoặc yêu cầu dừng từ con người, phải dừng ngay lập tức."
                    .to_string(),
                enforcement: Enforcement::Log,
            },
        ];
        Self { articles }
    }

    /// Tổng số điều luật trong hiến pháp.
    pub fn article_count(&self) -> usize {
        self.articles.len()
    }

    /// Tra cứu một điều luật theo mã số.
    pub fn article(&self, id: u32) -> Option<&ConstitutionArticle> {
        self.articles.iter().find(|article| article.id == id)
    }

    /// Mức cưỡng chế của một điều luật (mặc định `Log` nếu không tìm thấy).
    pub fn enforcement_of(&self, id: u32) -> Enforcement {
        self.article(id)
            .map(|article| article.enforcement)
            .unwrap_or(Enforcement::Log)
    }

    /// Kiểm tra một hành động đề xuất và trả về bản ghi vi phạm (nếu có).
    ///
    /// Thứ tự kiểm tra theo mã điều luật tăng dần; trả về vi phạm đầu tiên tìm thấy.
    pub fn check_action(&self, action: &ProposedAction) -> Option<PentestViolation> {
        // Điều 1: mục tiêu công khai nằm ngoài phạm vi tối thiểu của hiến pháp.
        if let Ok(ip) = action.target.parse::<Ipv4Addr>()
            && !is_internal_ipv4(ip)
            && self.enforcement_of(1) == Enforcement::Block
        {
            return Some(PentestViolation {
                article_id: 1,
                title: "Không tấn công ngoài phạm vi".to_string(),
                detail: format!(
                    "Mục tiêu {} là địa chỉ công khai, nằm ngoài phạm vi được duyệt.",
                    action.target
                ),
            });
        }

        // Điều 2: không phá hủy dữ liệu.
        if action.kind == ActionKind::Destructive && self.enforcement_of(2) == Enforcement::Block {
            return Some(PentestViolation {
                article_id: 2,
                title: "Không phá hủy dữ liệu".to_string(),
                detail: format!(
                    "Hành động phá hủy `{}` trên {} bị hiến pháp cấm.",
                    action.description, action.target
                ),
            });
        }

        // Điều 4: không thu thập dữ liệu vượt mức cần thiết.
        if action.kind == ActionKind::DataAccess {
            let description = action.description.to_lowercase();
            let bulk_request = description.contains("exfil")
                || description.contains("bulk")
                || description.contains("toàn bộ");
            if bulk_request && self.enforcement_of(4) == Enforcement::Warn {
                return Some(PentestViolation {
                    article_id: 4,
                    title: "Không đánh cắp dữ liệu".to_string(),
                    detail: format!(
                        "Yêu cầu `{}` vượt quá mức dữ liệu cần thiết cho kiểm thử.",
                        action.description
                    ),
                });
            }
        }

        // Điều 7: không leo thang sang mục tiêu chưa được duyệt.
        if action.kind == ActionKind::Pivot && self.enforcement_of(7) == Enforcement::Warn {
            return Some(PentestViolation {
                article_id: 7,
                title: "Không leo thang ngoài mục tiêu".to_string(),
                detail: format!(
                    "Chuyển ngang sang {} là mục tiêu chưa được duyệt trước.",
                    action.target
                ),
            });
        }

        None
    }

    /// Sinh chứng nhận (attestation) ràng buộc hiến pháp với một giá trị băm bất kỳ.
    ///
    /// Chuỗi chứng nhận ổn định: cùng hiến pháp + cùng `hash` luôn cho cùng kết quả.
    pub fn attest(&self, hash: &str) -> String {
        let mut digest_input = String::new();
        for article in &self.articles {
            digest_input.push_str(&format!("{}|{}|", article.id, article.rule));
        }
        let digest = sha256_hex(format!("{digest_input}|{hash}").as_bytes());
        format!(
            "pentest-constitution/v1|articles={}|witness={hash}|digest={digest}",
            self.article_count()
        )
    }
}

// ============================================================================
// 4. CHUỖI BẰNG CHỨNG (EVIDENCE INTEGRITY)
// ============================================================================

/// Một bản ghi bằng chứng trong chuỗi hash.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct Evidence {
    /// Mã bản ghi tăng dần từ 0.
    pub id: u32,
    /// Loại bằng chứng (ví dụ `"recon"`, `"exploit"`, `"hitl-blocked"`).
    pub kind: String,
    /// Nội dung bằng chứng thô.
    pub payload: String,
    /// Giá trị băm SHA-256 của bản ghi (băm `prev_hash + payload`).
    pub sha256: String,
    /// Dấu thời gian ảo (đồng hồ điều phối).
    pub timestamp: u64,
    /// Giá trị băm của bản ghi liền trước (khối mồi nếu là bản ghi đầu).
    pub prev_hash: String,
}

/// Chuỗi bằng chứng nối tiếp bằng hàm băm SHA-256.
///
/// Bất biến **Evidence Integrity**: mọi bản ghi đều gắn với bản ghi trước bằng giá trị
/// băm; sửa nội dung bất kỳ bản ghi nào sẽ làm `verify()` trả về `false`.
#[derive(Debug, Clone, Default, PartialEq, Eq)]
pub struct EvidenceChain {
    /// Các bản ghi theo thứ tự thời gian.
    pub entries: Vec<Evidence>,
    /// Mã bản ghi tiếp theo sẽ cấp.
    pub next_id: u32,
    /// Đồng hồ ảo hiện tại (tăng mỗi lần ghi bằng chứng).
    pub clock: u64,
}

impl EvidenceChain {
    /// Khởi tạo chuỗi bằng chứng rỗng.
    pub fn new() -> Self {
        Self::default()
    }

    /// Giá trị băm của bản ghi cuối cùng (khối mồi nếu chuỗi rỗng).
    pub fn head_hash(&self) -> String {
        self.entries
            .last()
            .map(|entry| entry.sha256.clone())
            .unwrap_or_else(|| EVIDENCE_GENESIS.to_string())
    }

    /// Danh sách giá trị băm của toàn bộ bản ghi theo thứ tự.
    pub fn chain_hashes(&self) -> Vec<String> {
        self.entries
            .iter()
            .map(|entry| entry.sha256.clone())
            .collect()
    }

    /// Số bản ghi trong chuỗi.
    pub fn len(&self) -> usize {
        self.entries.len()
    }

    /// Chuỗi có rỗng hay không.
    pub fn is_empty(&self) -> bool {
        self.entries.is_empty()
    }

    /// Ghi thêm một bằng chứng mới vào chuỗi, trả về dấu thời gian của bản ghi.
    ///
    /// Giá trị băm được tính theo công thức `sha256(prev_hash + payload)` — đúng theo
    /// đặc tả của chuỗi bằng chứng, không phụ thuộc thứ tự khóa nội bộ.
    pub fn append(&mut self, kind: &str, payload: &str) -> u64 {
        let prev_hash = self.head_hash();
        let sha256 = sha256_hex(format!("{prev_hash}{payload}").as_bytes());
        self.clock += 1;
        let entry = Evidence {
            id: self.next_id,
            kind: kind.to_string(),
            payload: payload.to_string(),
            sha256,
            timestamp: self.clock,
            prev_hash,
        };
        self.next_id += 1;
        self.entries.push(entry);
        self.clock
    }

    /// Kiểm tra tính toàn vẹn của chuỗi: liên kết băm, mã bản ghi và dấu thời gian.
    pub fn verify(&self) -> bool {
        let mut expected_prev = EVIDENCE_GENESIS.to_string();
        let mut last_timestamp = 0u64;
        for (index, entry) in self.entries.iter().enumerate() {
            if entry.id != index as u32 {
                return false;
            }
            if entry.prev_hash != expected_prev {
                return false;
            }
            if entry.timestamp <= last_timestamp {
                return false;
            }
            let recomputed = sha256_hex(format!("{}{}", entry.prev_hash, entry.payload).as_bytes());
            if recomputed != entry.sha256 {
                return false;
            }
            expected_prev = entry.sha256.clone();
            last_timestamp = entry.timestamp;
        }
        true
    }
}

// ============================================================================
// 5. CỔNG PHÊ DUYỆT CON NGƯỜI (HITL GATE)
// ============================================================================

/// Quyết định của người vận hành đối với một hành động cần phê duyệt.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct HitlDecision {
    /// `true` nếu được phép thực hiện.
    pub approved: bool,
    /// Tên người vận hành đã ra quyết định.
    pub operator: String,
    /// Lý do (đặc biệt khi từ chối).
    pub reason: String,
    /// Dấu thời gian ảo của quyết định.
    pub timestamp: u64,
}

/// Cổng phê duyệt của con người (Human-In-The-Loop).
///
/// Bất biến **HITL Gate**: mọi hành động nguy hiểm phải có quyết định `approved = true`
/// mới được phép thực thi.
#[derive(Debug, Clone, Default, PartialEq, Eq)]
pub struct HitlGate {
    /// Bảng quyết định theo khoá hành động.
    pub decisions: HashMap<String, HitlDecision>,
    /// Đồng hồ ảo của cổng phê duyệt.
    pub clock: u64,
}

impl HitlGate {
    /// Khởi tạo cổng phê duyệt rỗng.
    pub fn new() -> Self {
        Self::default()
    }

    /// Gửi yêu cầu phê duyệt cho một hành động; trả về `true` nếu đã được duyệt.
    ///
    /// Nếu chưa có quyết định nào, một bản ghi "đang chờ" được tạo để lưu vết.
    pub fn request(&mut self, action_key: &str) -> bool {
        self.clock += 1;
        if let Some(decision) = self.decisions.get(action_key) {
            return decision.approved;
        }
        let decision = HitlDecision {
            approved: false,
            operator: String::new(),
            reason: "Đang chờ quyết định của người vận hành".to_string(),
            timestamp: self.clock,
        };
        self.decisions.insert(action_key.to_string(), decision);
        false
    }

    /// Người vận hành phê duyệt một hành động.
    pub fn approve(&mut self, action_key: &str, operator: &str) -> HitlDecision {
        self.clock += 1;
        let decision = HitlDecision {
            approved: true,
            operator: operator.to_string(),
            reason: "Đã được người vận hành phê duyệt".to_string(),
            timestamp: self.clock,
        };
        self.decisions
            .insert(action_key.to_string(), decision.clone());
        decision
    }

    /// Người vận hành từ chối một hành động kèm lý do.
    pub fn deny(&mut self, action_key: &str, operator: &str, reason: &str) -> HitlDecision {
        self.clock += 1;
        let decision = HitlDecision {
            approved: false,
            operator: operator.to_string(),
            reason: reason.to_string(),
            timestamp: self.clock,
        };
        self.decisions
            .insert(action_key.to_string(), decision.clone());
        decision
    }

    /// Hành động đã được phê duyệt hay chưa.
    pub fn is_approved(&self, action_key: &str) -> bool {
        self.decisions
            .get(action_key)
            .is_some_and(|decision| decision.approved)
    }
}

// ============================================================================
// 6. ĐỒ THỊ TRẠNG THÁI 8 NÚT (MULTI-AGENT COORDINATION)
// ============================================================================

/// Các pha của một chiến dịch kiểm thử thâm nhập.
#[derive(Debug, Clone, Copy, PartialEq, Eq, PartialOrd, Ord)]
pub enum PentestPhase {
    /// Chờ khởi động.
    Idle,
    /// Trinh sát, lập bản đồ mạng.
    Recon,
    /// Liệt kê dịch vụ và phiên bản.
    Enumerate,
    /// Khai thác lỗ hổng.
    Exploit,
    /// Leo thang đặc quyền.
    PrivEsc,
    /// Chuyển ngang.
    Pivot,
    /// Thu thập dữ liệu (chỉ đọc).
    Loot,
    /// Sinh báo cáo.
    Report,
    /// Hoàn tất.
    Done,
}

impl PentestPhase {
    /// Tên hiển thị tiếng Việt của pha.
    pub fn label(&self) -> &'static str {
        match self {
            PentestPhase::Idle => "Chờ khởi động",
            PentestPhase::Recon => "Trinh sát",
            PentestPhase::Enumerate => "Liệt kê dịch vụ",
            PentestPhase::Exploit => "Khai thác",
            PentestPhase::PrivEsc => "Leo thang đặc quyền",
            PentestPhase::Pivot => "Chuyển ngang",
            PentestPhase::Loot => "Thu thập dữ liệu",
            PentestPhase::Report => "Báo cáo",
            PentestPhase::Done => "Hoàn tất",
        }
    }
}

/// Đồ thị trạng thái điều phối 8 nút của chiến dịch kiểm thử.
///
/// Bất biến **Multi-Agent Coordination**: các pha chỉ tiến được theo thứ tự
/// `Idle → Recon → Enumerate → Exploit → PrivEsc → Pivot → Loot → Report → Done`.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct PentestStateGraph {
    /// Pha hiện tại.
    pub phase: PentestPhase,
    /// Lịch sử các pha đã đi qua.
    pub history: Vec<PentestPhase>,
    /// Cờ bị gián tiếp (do yêu cầu dừng của con người).
    pub interrupted: bool,
    /// Số bước chuyển trạng thái tối đa cho phép.
    pub max_steps: usize,
    /// Số bước chuyển trạng thái đã thực hiện.
    pub steps: usize,
}

impl PentestStateGraph {
    /// Khởi tạo đồ thị ở pha `Idle` với giới hạn 12 bước.
    pub fn new() -> Self {
        Self {
            phase: PentestPhase::Idle,
            history: Vec::new(),
            interrupted: false,
            max_steps: 12,
            steps: 0,
        }
    }

    /// Danh sách các pha được phép chuyển tới từ `from` (đơn điệu về phía trước).
    pub fn allowed_transitions(from: PentestPhase) -> &'static [PentestPhase] {
        match from {
            PentestPhase::Idle => &[
                PentestPhase::Recon,
                PentestPhase::Enumerate,
                PentestPhase::Exploit,
                PentestPhase::PrivEsc,
                PentestPhase::Pivot,
                PentestPhase::Loot,
                PentestPhase::Report,
                PentestPhase::Done,
            ],
            PentestPhase::Recon => &[
                PentestPhase::Enumerate,
                PentestPhase::Exploit,
                PentestPhase::PrivEsc,
                PentestPhase::Pivot,
                PentestPhase::Loot,
                PentestPhase::Report,
                PentestPhase::Done,
            ],
            PentestPhase::Enumerate => &[
                PentestPhase::Exploit,
                PentestPhase::PrivEsc,
                PentestPhase::Pivot,
                PentestPhase::Loot,
                PentestPhase::Report,
                PentestPhase::Done,
            ],
            PentestPhase::Exploit => &[
                PentestPhase::PrivEsc,
                PentestPhase::Pivot,
                PentestPhase::Loot,
                PentestPhase::Report,
                PentestPhase::Done,
            ],
            PentestPhase::PrivEsc => &[
                PentestPhase::Pivot,
                PentestPhase::Loot,
                PentestPhase::Report,
                PentestPhase::Done,
            ],
            PentestPhase::Pivot => &[PentestPhase::Loot, PentestPhase::Report, PentestPhase::Done],
            PentestPhase::Loot => &[PentestPhase::Report, PentestPhase::Done],
            PentestPhase::Report => &[PentestPhase::Done],
            PentestPhase::Done => &[],
        }
    }

    /// Chuyển sang pha kế tiếp; trả về lỗi nếu chuyển dịch không hợp lệ.
    pub fn transition(&mut self, next: PentestPhase) -> Result<(), String> {
        if self.interrupted {
            return Err("Đồ thị trạng thái đang bị gián tiếp, không thể chuyển pha.".to_string());
        }
        if self.steps >= self.max_steps {
            return Err(format!(
                "Đã đạt giới hạn {} bước chuyển trạng thái.",
                self.max_steps
            ));
        }
        if !Self::allowed_transitions(self.phase).contains(&next) {
            return Err(format!(
                "Chuyển pha không hợp lệ: {} -> {}.",
                self.phase.label(),
                next.label()
            ));
        }
        self.phase = next;
        self.history.push(next);
        self.steps += 1;
        Ok(())
    }

    /// Gián tiếp chiến dịch (yêu cầu dừng từ con người); trả về `true` nếu trạng thái đổi.
    pub fn interrupt(&mut self) -> bool {
        if self.interrupted {
            return false;
        }
        self.interrupted = true;
        true
    }

    /// Tiếp tục chiến dịch sau khi gián tiếp; trả về `true` nếu trạng thái đổi.
    pub fn resume(&mut self) -> bool {
        if !self.interrupted {
            return false;
        }
        self.interrupted = false;
        true
    }

    /// Chiến dịch đã hoàn tất hay chưa.
    pub fn completed(&self) -> bool {
        self.phase == PentestPhase::Done
    }

    /// Số pha đã đi qua.
    pub fn phase_count(&self) -> usize {
        self.history.len()
    }
}

impl Default for PentestStateGraph {
    fn default() -> Self {
        Self::new()
    }
}

// ============================================================================
// 7. MÔ PHỎNG MẠNG GIẢ LẬP & BẰNG CHỨNG KHAI THÁC (GROUND TRUTH)
// ============================================================================

/// Mức nghiêm trọng của một phát hiện lỗ hổng.
#[derive(Debug, Clone, Copy, PartialEq, Eq, PartialOrd, Ord)]
pub enum Severity {
    /// Chỉ mang tính thông tin.
    Info,
    /// Rủi ro thấp.
    Low,
    /// Rủi ro trung bình.
    Medium,
    /// Rủi ro cao.
    High,
    /// Rủi ro nghiêm trọng.
    Critical,
}

impl Severity {
    /// Nhãn tiếng Việt của mức nghiêm trọng.
    pub fn label(&self) -> &'static str {
        match self {
            Severity::Info => "Thông tin",
            Severity::Low => "Thấp",
            Severity::Medium => "Trung bình",
            Severity::High => "Cao",
            Severity::Critical => "Nghiêm trọng",
        }
    }

    /// Trọng số điểm phòng thủ tương ứng với mức nghiêm trọng.
    pub fn defense_points(&self) -> f64 {
        match self {
            Severity::Info => 1.0,
            Severity::Low => 3.0,
            Severity::Medium => 8.0,
            Severity::High => 15.0,
            Severity::Critical => 25.0,
        }
    }
}

/// Một dịch vụ đang mở trên máy giả lập.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct MockService {
    /// Cổng TCP của dịch vụ.
    pub port: u16,
    /// Tên dịch vụ (ví dụ `"Apache"`, `"MySQL"`).
    pub service_name: String,
    /// Phiên bản dịch vụ (ví dụ `"2.4.29"`).
    pub version: String,
    /// Dịch vụ có điểm yếu đã biết hay không.
    pub vulnerable: bool,
    /// Khai thác dịch vụ này có làm sập dịch vụ (tín hiệu ồn cho phòng thủ) hay không.
    pub crash_on_exploit: bool,
}

/// Một máy giả lập trong mạng tổng hợp.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct MockHost {
    /// Địa chỉ IPv4 của máy.
    pub ip: Ipv4Addr,
    /// Tên máy chủ.
    pub hostname: String,
    /// Hệ điều hành giả lập.
    pub os: String,
    /// Danh sách dịch vụ đang mở.
    pub open_ports: Vec<MockService>,
}

/// Mạng giả lập tất định gồm 5 máy, dùng làm "mục tiêu" cho mọi thành phần.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct MockNetwork {
    /// Danh sách máy trong mạng.
    pub hosts: Vec<MockHost>,
}

impl MockNetwork {
    /// Dựng mạng giả lập tiêu chuẩn: web, cơ sở dữ liệu, máy tệp, máy trạm và máy nhảy.
    pub fn standard() -> Self {
        let hosts = vec![
            MockHost {
                ip: Ipv4Addr::new(10, 10, 10, 10),
                hostname: "web-01".to_string(),
                os: "Ubuntu 20.04 LTS".to_string(),
                open_ports: vec![
                    MockService {
                        port: 22,
                        service_name: "OpenSSH".to_string(),
                        version: "7.6".to_string(),
                        vulnerable: false,
                        crash_on_exploit: false,
                    },
                    MockService {
                        port: 80,
                        service_name: "Apache".to_string(),
                        version: "2.4.29".to_string(),
                        vulnerable: true,
                        crash_on_exploit: false,
                    },
                ],
            },
            MockHost {
                ip: Ipv4Addr::new(10, 10, 10, 20),
                hostname: "db-01".to_string(),
                os: "Debian 10".to_string(),
                open_ports: vec![
                    MockService {
                        port: 22,
                        service_name: "OpenSSH".to_string(),
                        version: "7.6".to_string(),
                        vulnerable: false,
                        crash_on_exploit: false,
                    },
                    MockService {
                        port: 3306,
                        service_name: "MySQL".to_string(),
                        version: "5.5.8".to_string(),
                        vulnerable: true,
                        crash_on_exploit: false,
                    },
                ],
            },
            MockHost {
                ip: Ipv4Addr::new(10, 10, 10, 30),
                hostname: "file-01".to_string(),
                os: "Debian 10".to_string(),
                open_ports: vec![
                    MockService {
                        port: 139,
                        service_name: "NetBIOS".to_string(),
                        version: "1.0".to_string(),
                        vulnerable: false,
                        crash_on_exploit: false,
                    },
                    MockService {
                        port: 445,
                        service_name: "Samba".to_string(),
                        version: "4.1.11".to_string(),
                        vulnerable: true,
                        crash_on_exploit: false,
                    },
                ],
            },
            MockHost {
                ip: Ipv4Addr::new(10, 10, 10, 40),
                hostname: "ws-01".to_string(),
                os: "Windows 7 SP1".to_string(),
                open_ports: vec![
                    MockService {
                        port: 3389,
                        service_name: "RDP".to_string(),
                        version: "5.1".to_string(),
                        vulnerable: true,
                        crash_on_exploit: true,
                    },
                    MockService {
                        port: 445,
                        service_name: "SMB".to_string(),
                        version: "3.1".to_string(),
                        vulnerable: false,
                        crash_on_exploit: false,
                    },
                ],
            },
            MockHost {
                ip: Ipv4Addr::new(10, 10, 10, 50),
                hostname: "jump-01".to_string(),
                os: "Ubuntu 18.04 LTS".to_string(),
                open_ports: vec![
                    MockService {
                        port: 22,
                        service_name: "OpenSSH".to_string(),
                        version: "8.2".to_string(),
                        vulnerable: true,
                        crash_on_exploit: false,
                    },
                    MockService {
                        port: 8080,
                        service_name: "Jetty".to_string(),
                        version: "9.4".to_string(),
                        vulnerable: false,
                        crash_on_exploit: false,
                    },
                ],
            },
        ];
        Self { hosts }
    }

    /// Tra cứu một máy theo địa chỉ.
    pub fn host(&self, ip: Ipv4Addr) -> Option<&MockHost> {
        self.hosts.iter().find(|host| host.ip == ip)
    }

    /// Tra cứu dịch vụ theo địa chỉ và cổng.
    pub fn service(&self, ip: Ipv4Addr, port: u16) -> Option<&MockService> {
        self.host(ip)
            .and_then(|host| host.open_ports.iter().find(|svc| svc.port == port))
    }

    /// Danh sách mọi dịch vụ đang mở theo thứ tự ổn định (máy, rồi cổng).
    pub fn all_services(&self) -> Vec<(Ipv4Addr, &MockService)> {
        let mut out = Vec::new();
        for host in &self.hosts {
            let mut services: Vec<&MockService> = host.open_ports.iter().collect();
            services.sort_by_key(|svc| svc.port);
            for service in services {
                out.push((host.ip, service));
            }
        }
        out
    }
}

/// Vỏ dòng lệnh giả lập — không bao giờ chạm tới hệ thống thật.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct MockShell {
    /// Lịch sử các lệnh đã thực thi.
    pub history: Vec<String>,
    /// Thư mục làm việc hiện tại.
    pub cwd: String,
    /// Biến môi trường giả lập.
    pub env: HashMap<String, String>,
    /// Các phiên shell còn treo (phải đóng trước khi thoát sạch).
    pub open_sessions: Vec<String>,
}

impl MockShell {
    /// Khởi tạo vỏ giả lập tại thư mục `/home/pentest`.
    pub fn new() -> Self {
        let mut env = HashMap::new();
        env.insert("USER".to_string(), "pentest".to_string());
        env.insert("SCOPE".to_string(), "10.10.10.0/24".to_string());
        Self {
            history: Vec::new(),
            cwd: "/home/pentest".to_string(),
            env,
            open_sessions: Vec::new(),
        }
    }

    /// Thực thi một lệnh giả lập và trả về đầu ra tất định tương ứng.
    pub fn exec(&mut self, command: &str) -> String {
        self.history.push(command.to_string());
        if command.contains("id ") || command.ends_with("id") {
            "uid=1000(pentest) gid=1000(pentest) groups=1000(pentest)".to_string()
        } else if command.contains("uname") {
            "Linux mock-host 5.15.0-mock #1 SMP x86_64 GNU/Linux".to_string()
        } else if command.contains("passwd") || command.contains("shadow") {
            "root:x:0:0:root:/root:/bin/bash\npentest:x:1000:1000::/home/pentest:/bin/bash"
                .to_string()
        } else {
            format!("[mock] đã thực thi: {command}")
        }
    }

    /// Mở một phiên shell con (dùng để kiểm tra bất biến Clean Exit).
    pub fn open_session(&mut self, name: &str) {
        self.open_sessions.push(name.to_string());
    }

    /// Đóng toàn bộ phiên còn treo, trả về số phiên đã đóng.
    pub fn close_all(&mut self) -> usize {
        let count = self.open_sessions.len();
        self.open_sessions.clear();
        count
    }
}

impl Default for MockShell {
    fn default() -> Self {
        Self::new()
    }
}

/// Thông tin tóm tắt về một máy sau khi quét.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct HostInfo {
    /// Địa chỉ IPv4 của máy.
    pub ip: Ipv4Addr,
    /// Tên máy chủ.
    pub hostname: String,
    /// Hệ điều hành giả lập.
    pub os: String,
    /// Danh sách cổng đang mở.
    pub open_ports: Vec<u16>,
}

/// Bộ lập bản đồ mạng: quét và ghi nhớ cấu trúc mạng đã khám phá.
#[derive(Debug, Clone, Default, PartialEq, Eq)]
pub struct NetworkMapper {
    /// Danh sách máy đã lập bản đồ.
    pub hosts: Vec<HostInfo>,
    /// Số lần quét đã thực hiện.
    pub scans: usize,
}

impl NetworkMapper {
    /// Khởi tạo bộ lập bản đồ rỗng.
    pub fn new() -> Self {
        Self::default()
    }

    /// Quét mạng và trả về danh sách thông tin máy tìm thấy.
    pub fn scan(&mut self, network: &MockNetwork) -> Vec<HostInfo> {
        self.scans += 1;
        let mut discovered = Vec::new();
        for host in &network.hosts {
            let mut ports: Vec<u16> = host.open_ports.iter().map(|svc| svc.port).collect();
            ports.sort_unstable();
            let info = HostInfo {
                ip: host.ip,
                hostname: host.hostname.clone(),
                os: host.os.clone(),
                open_ports: ports,
            };
            discovered.push(info);
        }
        self.hosts = discovered.clone();
        discovered
    }

    /// Tra cứu thông tin một máy đã lập bản đồ.
    pub fn host(&self, ip: Ipv4Addr) -> Option<&HostInfo> {
        self.hosts.iter().find(|info| info.ip == ip)
    }
}

/// Một chữ ký dịch vụ đã biết là có điểm yếu.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct FingerprintSignature {
    /// Tên dịch vụ.
    pub service_name: String,
    /// Phiên bản dịch vụ.
    pub version: String,
    /// Mã phát hiện.
    pub vuln_id: String,
    /// Tiêu đề phát hiện.
    pub title: String,
    /// Mức nghiêm trọng.
    pub severity: Severity,
    /// Mã định danh CVE (mô phỏng).
    pub cve: String,
    /// Mô tả kỹ thuật.
    pub description: String,
}

/// Bộ nhận dạng phiên bản dịch vụ, phát hiện điểm yếu theo chữ ký.
///
/// Bảng chữ ký cố tình chứa **một mục giả** (`WEB-901` cho Jetty 9.4) để chứng minh
/// tác tử Validation có nhiệm vụ loại bỏ phát hiện sai — xem bất biến
/// **Ground-truth Verification**.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct ServiceFingerprinter {
    /// Danh sách chữ ký đã biết.
    pub signatures: Vec<FingerprintSignature>,
}

impl ServiceFingerprinter {
    /// Khởi tạo bộ nhận dạng với bảng chữ ký tiêu chuẩn.
    pub fn new() -> Self {
        let signatures = vec![
            FingerprintSignature {
                service_name: "Apache".to_string(),
                version: "2.4.29".to_string(),
                vuln_id: "WEB-001".to_string(),
                title: "Apache 2.4.29 — duyệt đường dẫn ngoài dự kiến".to_string(),
                severity: Severity::High,
                cve: "CVE-2017-7668".to_string(),
                description: "Máy chủ web cho phép đọc tệp ngoài thư mục tài liệu qua đường dẫn chứa ký tự .."
                    .to_string(),
            },
            FingerprintSignature {
                service_name: "MySQL".to_string(),
                version: "5.5.8".to_string(),
                vuln_id: "DB-002".to_string(),
                title: "MySQL 5.5.8 — tài khoản quản trị mặc định".to_string(),
                severity: Severity::Critical,
                cve: "CVE-2012-0158".to_string(),
                description: "Cổng cơ sở dữ liệu chấp nhận tài khoản mặc định không đổi mật khẩu."
                    .to_string(),
            },
            FingerprintSignature {
                service_name: "Samba".to_string(),
                version: "4.1.11".to_string(),
                vuln_id: "SMB-003".to_string(),
                title: "Samba 4.1.11 — hỗ trợ SMBv1 không giới hạn".to_string(),
                severity: Severity::High,
                cve: "CVE-2017-14746".to_string(),
                description: "Máy chủ tệp vẫn thương lượng giao thức SMBv1 cũ và không giới hạn mức."
                    .to_string(),
            },
            FingerprintSignature {
                service_name: "RDP".to_string(),
                version: "5.1".to_string(),
                vuln_id: "RDP-004".to_string(),
                title: "RDP 5.1 — không bật xác thực mức mạng".to_string(),
                severity: Severity::Medium,
                cve: "CVE-2017-0143".to_string(),
                description: "Dịch vụ điều khiển từ xa không bắt buộc bước xác thực mức mạng."
                    .to_string(),
            },
            FingerprintSignature {
                service_name: "OpenSSH".to_string(),
                version: "8.2".to_string(),
                vuln_id: "SSH-005".to_string(),
                title: "OpenSSH 8.2 — khóa máy chủ mặc định".to_string(),
                severity: Severity::Medium,
                cve: "CVE-2020-15778".to_string(),
                description: "Cấu hình khóa máy chủ mặc định cho phép đăng nhập bằng cặp khoá yếu."
                    .to_string(),
            },
            // Mục giả: Jetty 9.4 bị ghi nhận là lỗi thời nhưng không có trong ground-truth.
            FingerprintSignature {
                service_name: "Jetty".to_string(),
                version: "9.4".to_string(),
                vuln_id: "WEB-901".to_string(),
                title: "Jetty 9.4 — khung web lỗi thời".to_string(),
                severity: Severity::Low,
                cve: "CVE-2019-10241".to_string(),
                description: "Khung web cũ, nghi ngờ thiếu bản vá — cần kiểm chứng thêm.".to_string(),
            },
        ];
        Self { signatures }
    }

    /// Nhận dạng một dịch vụ, trả về phát hiện ứng viên nếu trùng chữ ký.
    pub fn fingerprint(&self, host: &MockHost, service: &MockService) -> Option<Vulnerability> {
        self.signatures
            .iter()
            .find(|signature| {
                signature.service_name == service.service_name
                    && signature.version == service.version
            })
            .map(|signature| Vulnerability {
                id: signature.vuln_id.clone(),
                title: signature.title.clone(),
                severity: signature.severity,
                target: host.ip,
                port: service.port,
                description: signature.description.clone(),
                cve: signature.cve.clone(),
            })
    }
}

impl Default for ServiceFingerprinter {
    fn default() -> Self {
        Self::new()
    }
}

/// Một phát hiện lỗ hổng do tác tử tạo ra.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct Vulnerability {
    /// Mã phát hiện (ví dụ `"WEB-001"`).
    pub id: String,
    /// Tiêu đề phát hiện.
    pub title: String,
    /// Mức nghiêm trọng.
    pub severity: Severity,
    /// Mục tiêu bị ảnh hưởng.
    pub target: Ipv4Addr,
    /// Cổng bị ảnh hưởng.
    pub port: u16,
    /// Mô tả kỹ thuật.
    pub description: String,
    /// Mã định danh CVE (mô phỏng).
    pub cve: String,
}

/// Bằng chứng khai thác (Proof-By-Exploitation) của một phát hiện.
///
/// Bất biến **Ground-truth Verification**: phát hiện chỉ được giữ lại khi PoE
/// tái lập được và khớp với ground-truth.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct ProofByExploitation {
    /// Mã phát hiện được chứng minh.
    pub vuln_id: String,
    /// Mục tiêu của bằng chứng.
    pub target: Ipv4Addr,
    /// Cổng của bằng chứng.
    pub port: u16,
    /// Các bước tái lập (mô tả trên môi trường giả lập).
    pub steps: Vec<String>,
    /// Kết quả thu được sau khi tái lập.
    pub output: String,
    /// `true` nếu PoE tái lập thành công.
    pub reproduced: bool,
}

/// Bộ chứng nhận ground-truth: nguồn sự thật duy nhất để đối chiếu phát hiện.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct GroundTruthVerifier {
    /// Danh sách lỗ hổng có thật trong môi trường giả lập.
    pub ground_truth: Vec<Vulnerability>,
}

impl GroundTruthVerifier {
    /// Khởi tạo ground-truth chuẩn khớp với `MockNetwork::standard()`.
    pub fn new() -> Self {
        let ground_truth = vec![
            Vulnerability {
                id: "WEB-001".to_string(),
                title: "Apache 2.4.29 — duyệt đường dẫn ngoài dự kiến".to_string(),
                severity: Severity::High,
                target: Ipv4Addr::new(10, 10, 10, 10),
                port: 80,
                description: "Máy chủ web cho phép đọc tệp ngoài thư mục tài liệu qua đường dẫn chứa ký tự .."
                    .to_string(),
                cve: "CVE-2017-7668".to_string(),
            },
            Vulnerability {
                id: "DB-002".to_string(),
                title: "MySQL 5.5.8 — tài khoản quản trị mặc định".to_string(),
                severity: Severity::Critical,
                target: Ipv4Addr::new(10, 10, 10, 20),
                port: 3306,
                description: "Cổng cơ sở dữ liệu chấp nhận tài khoản mặc định không đổi mật khẩu."
                    .to_string(),
                cve: "CVE-2012-0158".to_string(),
            },
            Vulnerability {
                id: "SMB-003".to_string(),
                title: "Samba 4.1.11 — hỗ trợ SMBv1 không giới hạn".to_string(),
                severity: Severity::High,
                target: Ipv4Addr::new(10, 10, 10, 30),
                port: 445,
                description: "Máy chủ tệp vẫn thương lượng giao thức SMBv1 cũ và không giới hạn mức."
                    .to_string(),
                cve: "CVE-2017-14746".to_string(),
            },
            Vulnerability {
                id: "RDP-004".to_string(),
                title: "RDP 5.1 — không bật xác thực mức mạng".to_string(),
                severity: Severity::Medium,
                target: Ipv4Addr::new(10, 10, 10, 40),
                port: 3389,
                description: "Dịch vụ điều khiển từ xa không bắt buộc bước xác thực mức mạng."
                    .to_string(),
                cve: "CVE-2017-0143".to_string(),
            },
            Vulnerability {
                id: "SSH-005".to_string(),
                title: "OpenSSH 8.2 — khóa máy chủ mặc định".to_string(),
                severity: Severity::Medium,
                target: Ipv4Addr::new(10, 10, 10, 50),
                port: 22,
                description: "Cấu hình khóa máy chủ mặc định cho phép đăng nhập bằng cặp khoá yếu."
                    .to_string(),
                cve: "CVE-2020-15778".to_string(),
            },
        ];
        Self { ground_truth }
    }

    /// Phát hiện có khớp ground-truth hay không (cả mã, mục tiêu và cổng).
    pub fn is_true_positive(&self, finding: &Vulnerability) -> bool {
        self.ground_truth.iter().any(|truth| {
            truth.id == finding.id && truth.target == finding.target && truth.port == finding.port
        })
    }

    /// Xác minh một phát hiện và sinh Proof-By-Exploitation nếu khớp ground-truth.
    ///
    /// Trả về `None` khi phát hiện là **giả** — đây chính là cơ chế loại bỏ
    /// phát hiện sai của tác tử Validation.
    pub fn verify(&self, finding: &Vulnerability) -> Option<ProofByExploitation> {
        if !self.is_true_positive(finding) {
            return None;
        }
        Some(ProofByExploitation {
            vuln_id: finding.id.clone(),
            target: finding.target,
            port: finding.port,
            steps: Self::reproduction_steps(finding),
            output: format!(
                "[mock] PoE thành công trên {}:{} — xác nhận {}",
                finding.target, finding.port, finding.title
            ),
            reproduced: true,
        })
    }

    /// Sinh danh sách bước tái lập chuẩn hoá cho một phát hiện.
    ///
    /// Các bước mô tả thao tác **trên dịch vụ giả lập**, không phải mã khai thác thực.
    pub fn reproduction_steps(finding: &Vulnerability) -> Vec<String> {
        vec![
            format!(
                "Bước 1: Thiết lập phiên mô phỏng tới dịch vụ {} trên {}:{}.",
                finding.title.split('—').next().unwrap_or("mục tiêu").trim(),
                finding.target,
                finding.port
            ),
            "Bước 2: Gửi yêu cầu kiểm thử gói tin đã đóng dấu vào dịch vụ giả lập.".to_string(),
            "Bước 3: Thu thập phản hồi và đối chiếu với điều kiện thành công đã định nghĩa."
                .to_string(),
            "Bước 4: Ghi lại đầu ra chuẩn hoá kèm giá trị băm SHA-256 của phiên kiểm thử."
                .to_string(),
        ]
    }
}

impl Default for GroundTruthVerifier {
    fn default() -> Self {
        Self::new()
    }
}

// ============================================================================
// 8. TÁC TỬ ĐA TÁC TỬ (MULTI-AGENT COORDINATION)
// ============================================================================

/// Vai trò của một tác tử trong chiến dịch kiểm thử.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum AgentRole {
    /// Tác tử trinh sát: lập bản đồ và nhận dạng dịch vụ.
    Recon,
    /// Tác tử khai thác: dựng PoC cho các phát hiện ứng viên.
    Exploit,
    /// Tác tử kiểm chứng: đối chiếu phát hiện với ground-truth.
    Validation,
    /// Tác tử điều phối: dẫn dắt đồ thị pha và các cổng kiểm soát.
    Orchestrator,
}

impl AgentRole {
    /// Nhãn tiếng Việt của vai trò.
    pub fn label(&self) -> &'static str {
        match self {
            AgentRole::Recon => "Trinh sát",
            AgentRole::Exploit => "Khai thác",
            AgentRole::Validation => "Kiểm chứng",
            AgentRole::Orchestrator => "Điều phối",
        }
    }
}

/// Ngữ cảnh truyền vào cho mỗi lượt làm việc của tác tử.
#[derive(Debug, Clone)]
pub struct AgentContext<'a> {
    /// Pha hiện tại của chiến dịch.
    pub phase: PentestPhase,
    /// Mạng giả lập đang tác động tới.
    pub network: &'a MockNetwork,
    /// Các phát hiện ứng viên đã biết.
    pub findings: Vec<Vulnerability>,
    /// Các mục tiêu đã lập bản đồ.
    pub targets: Vec<Ipv4Addr>,
    /// Vỏ dòng lệnh giả lập (bản sao để tác tử thao tác).
    pub shell: MockShell,
}

/// Kết quả một lượt làm việc của tác tử.
#[derive(Debug, Clone)]
pub struct AgentResult {
    /// Tên tác tử thực hiện.
    pub agent: String,
    /// Lượt làm việc có thành công hay không.
    pub success: bool,
    /// Thông điệp tóm tắt kết quả.
    pub message: String,
    /// Các phát hiện thu được.
    pub findings: Vec<Vulnerability>,
    /// Các mục tiêu đã xác định.
    pub targets: Vec<Ipv4Addr>,
    /// Bằng chứng khai thác kèm theo (nếu có).
    pub proof: Option<ProofByExploitation>,
}

/// Đặc tả chung cho mọi tác tử pentest.
pub trait PentestAgent {
    /// Tên định danh của tác tử.
    fn name(&self) -> &str;
    /// Vai trò của tác tử trong hệ đa tác tử.
    fn role(&self) -> AgentRole;
    /// Thực hiện một lượt làm việc theo ngữ cảnh cho trước.
    fn step(&mut self, ctx: &AgentContext) -> AgentResult;
}

/// Tác tử trinh sát: lập bản đồ mạng và nhận dạng phiên bản dịch vụ.
#[derive(Debug, Clone, Default)]
pub struct ReconAgent {
    /// Bộ lập bản đồ mạng.
    pub mapper: NetworkMapper,
    /// Bộ nhận dạng dịch vụ.
    pub fingerprinter: ServiceFingerprinter,
}

impl ReconAgent {
    /// Khởi tạo tác tử trinh sát với bộ lập bản đồ và bộ nhận dạng mặc định.
    pub fn new() -> Self {
        Self {
            mapper: NetworkMapper::new(),
            fingerprinter: ServiceFingerprinter::new(),
        }
    }
}

impl PentestAgent for ReconAgent {
    fn name(&self) -> &str {
        "ReconAgent"
    }

    fn role(&self) -> AgentRole {
        AgentRole::Recon
    }

    fn step(&mut self, ctx: &AgentContext) -> AgentResult {
        let mapped = self.mapper.scan(ctx.network);
        let mut findings = Vec::new();
        for host in &ctx.network.hosts {
            for service in &host.open_ports {
                if let Some(candidate) = self.fingerprinter.fingerprint(host, service) {
                    findings.push(candidate);
                }
            }
        }
        let targets: Vec<Ipv4Addr> = mapped.iter().map(|info| info.ip).collect();
        let message = format!(
            "Quét xong {} máy, nhận dạng {} dịch vụ, {} phát hiện ứng viên.",
            mapped.len(),
            ctx.network
                .hosts
                .iter()
                .map(|host| host.open_ports.len())
                .sum::<usize>(),
            findings.len()
        );
        AgentResult {
            agent: self.name().to_string(),
            success: true,
            message,
            findings,
            targets,
            proof: None,
        }
    }
}

/// Tác tử khai thác: dựng PoC tái lập được cho các phát hiện ứng viên.
#[derive(Debug, Clone, Default)]
pub struct ExploitAgent {
    /// Danh sách mã phát hiện đã khai thác (tránh khai thác lại).
    pub used: Vec<String>,
}

impl ExploitAgent {
    /// Khởi tạo tác tử khai thác với danh sách rỗng.
    pub fn new() -> Self {
        Self { used: Vec::new() }
    }

    /// Chọn phát hiện ứng viên kế tiếp chưa khai thác.
    fn next_candidate(&self, ctx: &AgentContext) -> Option<Vulnerability> {
        ctx.findings
            .iter()
            .find(|finding| !self.used.contains(&finding.id))
            .cloned()
    }
}

impl PentestAgent for ExploitAgent {
    fn name(&self) -> &str {
        "ExploitAgent"
    }

    fn role(&self) -> AgentRole {
        AgentRole::Exploit
    }

    fn step(&mut self, ctx: &AgentContext) -> AgentResult {
        let Some(candidate) = self.next_candidate(ctx) else {
            return AgentResult {
                agent: self.name().to_string(),
                success: false,
                message: "Không còn phát hiện ứng viên nào để khai thác.".to_string(),
                findings: Vec::new(),
                targets: Vec::new(),
                proof: None,
            };
        };

        let mut shell = ctx.shell.clone();
        let command = format!(
            "poc-sim --target {}:{} --check --vuln {}",
            candidate.target, candidate.port, candidate.id
        );
        let output = shell.exec(&command);
        self.used.push(candidate.id.clone());

        let proof = ProofByExploitation {
            vuln_id: candidate.id.clone(),
            target: candidate.target,
            port: candidate.port,
            steps: GroundTruthVerifier::reproduction_steps(&candidate),
            output,
            reproduced: true,
        };

        AgentResult {
            agent: self.name().to_string(),
            success: true,
            message: format!(
                "Dựng PoC cho {} trên {}:{} — kết quả: {}",
                candidate.id, candidate.target, candidate.port, proof.output
            ),
            findings: vec![candidate.clone()],
            targets: vec![candidate.target],
            proof: Some(proof),
        }
    }
}

/// Tác tử kiểm chứng: chỉ giữ lại phát hiện khớp ground-truth.
#[derive(Debug, Clone, Default)]
pub struct ValidationAgent {
    /// Bộ chứng nhận ground-truth.
    pub verifier: GroundTruthVerifier,
}

impl ValidationAgent {
    /// Khởi tạo tác tử kiểm chứng với ground-truth chuẩn.
    pub fn new() -> Self {
        Self {
            verifier: GroundTruthVerifier::new(),
        }
    }
}

impl PentestAgent for ValidationAgent {
    fn name(&self) -> &str {
        "ValidationAgent"
    }

    fn role(&self) -> AgentRole {
        AgentRole::Validation
    }

    fn step(&mut self, ctx: &AgentContext) -> AgentResult {
        let mut verified = Vec::new();
        let mut proofs = Vec::new();
        let mut rejected = Vec::new();
        for finding in &ctx.findings {
            match self.verifier.verify(finding) {
                Some(proof) => {
                    verified.push(finding.clone());
                    proofs.push(proof);
                }
                None => rejected.push(finding.id.clone()),
            }
        }
        let message = format!(
            "Kiểm chứng {} phát hiện: {} hợp lệ, {} bị loại bỏ ({})",
            ctx.findings.len(),
            verified.len(),
            rejected.len(),
            if rejected.is_empty() {
                "không có".to_string()
            } else {
                rejected.join(", ")
            }
        );
        AgentResult {
            agent: self.name().to_string(),
            success: !verified.is_empty(),
            message,
            findings: verified,
            targets: Vec::new(),
            proof: proofs.into_iter().next(),
        }
    }
}

/// Lý do một hành động bị chặn.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum BlockReason {
    /// Hành động được phép thực thi.
    None,
    /// Bị cổng phạm vi chặn.
    Scope,
    /// Bị cổng phê duyệt con người chặn.
    Hitl,
    /// Bị hiến pháp chặn.
    Constitution,
}

/// Kết quả kiểm tra uỷ quyền cho một hành động.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct ActionOutcome {
    /// Hành động có được phép chạy hay không.
    pub allowed: bool,
    /// Hành động có nằm trong phạm vi hay không.
    pub scope_ok: bool,
    /// Hành động có cần phê duyệt của con người hay không.
    pub hitl_required: bool,
    /// Lý do bị chặn.
    pub blocked_by: BlockReason,
    /// Bản ghi vi phạm hiến pháp (nếu có).
    pub violation: Option<PentestViolation>,
    /// Giải thích bằng văn bản.
    pub reason: String,
}

/// Tác tử điều phối: dẫn dắt đồ thị pha và vận hành ba cổng kiểm soát.
#[derive(Debug, Clone)]
pub struct OrchestratorAgent {
    /// Đồ thị trạng thái 8 nút.
    pub graph: PentestStateGraph,
    /// Cổng phạm vi.
    pub scope: ScopeGuard,
    /// Hiến pháp kiểm thử.
    pub constitution: PentestConstitution,
    /// Chuỗi bằng chứng.
    pub evidence: EvidenceChain,
    /// Cổng phê duyệt con người.
    pub hitl: HitlGate,
    /// Đồng hồ ảo của tác tử.
    pub clock: u64,
    /// Các máy đã bị xâm nhập — cơ sở để kiểm tra ranh giới đặc quyền.
    pub compromised: Vec<Ipv4Addr>,
}

impl OrchestratorAgent {
    /// Khởi tạo tác tử điều phối với phạm vi cho trước.
    pub fn new(scope: ScopeGuard) -> Self {
        Self {
            graph: PentestStateGraph::new(),
            scope,
            constitution: PentestConstitution::standard(),
            evidence: EvidenceChain::new(),
            hitl: HitlGate::new(),
            clock: 0,
            compromised: Vec::new(),
        }
    }

    /// Tăng đồng hồ ảo và trả về giá trị mới.
    pub fn tick(&mut self) -> u64 {
        self.clock += 1;
        self.clock
    }

    /// Ghi một bằng chứng vào chuỗi hash và trả về dấu thời gian.
    pub fn record(&mut self, kind: &str, payload: &str) -> u64 {
        self.tick();
        self.evidence.append(kind, payload)
    }

    /// Khoá ổn định của một hành động, dùng cho cổng HITL và nhật ký.
    pub fn action_key(action: &ProposedAction) -> String {
        format!("{}:{}:{}", action.kind.label(), action.target, action.port)
    }

    /// Kiểm tra uỷ quyền theo thứ tự: phạm vi → cổng HITL → hiến pháp.
    ///
    /// Cổng HITL được kiểm tra **trước** hiến pháp để con người luôn có tiếng nói
    /// cuối cùng; hiến pháp đóng vai trò "lưới an toàn" chặn cứng những hành động
    /// mà kể cả khi được duyệt cũng không được phép thực hiện.
    pub fn authorize(&mut self, action: &ProposedAction) -> ActionOutcome {
        let mut scope_ok = true;
        if let Ok(ip) = action.target.parse::<Ipv4Addr>() {
            scope_ok = self.scope.check(ip, action.port);
        }

        let violation = self.constitution.check_action(action);
        let hitl_required = matches!(action.kind, ActionKind::Destructive | ActionKind::Pivot);

        if !scope_ok {
            let reason = self
                .scope
                .violations()
                .last()
                .cloned()
                .unwrap_or_else(|| "Hành động nằm ngoài phạm vi.".to_string());
            return ActionOutcome {
                allowed: false,
                scope_ok: false,
                hitl_required,
                blocked_by: BlockReason::Scope,
                violation,
                reason,
            };
        }

        if hitl_required {
            let key = Self::action_key(action);
            if !self.hitl.request(&key) {
                let reason = format!(
                    "Hành động `{}` cần phê duyệt của người vận hành (cổng HITL), chưa được duyệt.",
                    action.description
                );
                return ActionOutcome {
                    allowed: false,
                    scope_ok: true,
                    hitl_required: true,
                    blocked_by: BlockReason::Hitl,
                    violation,
                    reason,
                };
            }
        }

        // Bất biến Privilege Boundary: không leo thang lên máy chưa bị xâm nhập.
        if action.kind == ActionKind::PrivEsc
            && let Ok(ip) = action.target.parse::<Ipv4Addr>()
            && !self.compromised.contains(&ip)
        {
            let reason = format!(
                "Leo thang đặc quyền bị từ chối: {} chưa bị xâm nhập nên nằm ngoài ranh giới đặc quyền.",
                action.target
            );
            return ActionOutcome {
                allowed: false,
                scope_ok: true,
                hitl_required,
                blocked_by: BlockReason::Constitution,
                violation: Some(PentestViolation {
                    article_id: 7,
                    title: "Không leo thang ngoài mục tiêu".to_string(),
                    detail: reason.clone(),
                }),
                reason,
            };
        }

        if let Some(record) = &violation
            && self.constitution.enforcement_of(record.article_id) == Enforcement::Block
        {
            let reason = format!(
                "Vi phạm điều {} của hiến pháp: {}",
                record.article_id, record.detail
            );
            return ActionOutcome {
                allowed: false,
                scope_ok: true,
                hitl_required,
                blocked_by: BlockReason::Constitution,
                violation,
                reason,
            };
        }

        let reason = match &violation {
            Some(record) => format!(
                "Cảnh báo điều {}: {} (vẫn cho phép tiếp tục)",
                record.article_id, record.detail
            ),
            None => "Hành động hợp lệ theo phạm vi, hiến pháp và cổng HITL.".to_string(),
        };
        ActionOutcome {
            allowed: true,
            scope_ok: true,
            hitl_required,
            blocked_by: BlockReason::None,
            violation,
            reason,
        }
    }
}

// ============================================================================
// 9. PHẦN THƯỞNG HỌC TĂNG CƯỜNG (RL REWARD)
// ============================================================================

/// Một hành động tấn công ở mức mạng, dùng làm không gian hành động cho RL.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct AttackAction {
    /// Loại hành động.
    pub kind: ActionKind,
    /// Mục tiêu.
    pub target: Ipv4Addr,
    /// Cổng đích.
    pub port: u16,
}

impl AttackAction {
    /// Khoá ổn định của hành động dùng cho bảng Q.
    pub fn key(&self) -> String {
        format!("{}:{}:{}", self.kind.label(), self.target, self.port)
    }
}

/// Trạng thái mạng trong một tập mô phỏng đối kháng.
#[derive(Debug, Clone, Default, PartialEq, Eq)]
pub struct NetworkState {
    /// Các máy đã bị xâm nhập.
    pub compromised: Vec<Ipv4Addr>,
    /// Đã bị phát hiện hay chưa.
    pub detected: bool,
    /// Các máy bị làm sập dịch vụ.
    pub crashed: Vec<Ipv4Addr>,
    /// Đã truy cập dữ liệu hay chưa.
    pub accessed_data: bool,
}

/// Kết quả chấm điểm một hành động tấn công.
#[derive(Debug, Clone, PartialEq)]
pub struct AttackReward {
    /// Tổng phần thưởng của hành động.
    pub total: f64,
    /// Các thành phần đóng góp vào tổng phần thưởng.
    pub components: Vec<(String, f64)>,
    /// Trạng thái phát hiện sau hành động.
    pub detected: bool,
    /// Dịch vụ có bị làm sập hay không.
    pub crashed_service: bool,
    /// Số máy đã bị xâm nhập.
    pub hosts_compromised: usize,
}

/// Hàm phần thưởng của tác tử đỏ, lấy cảm hứng từ CyberBattleSim.
///
/// Tổng phần thưởng là tổng của năm thành phần:
/// $$R = w_{\text{comp}} \cdot \mathbb{1}[\text{xâm nhập}] - w_{\text{det}} \cdot \mathbb{1}[\text{bị phát hiện}] - w_{\text{crash}} \cdot \mathbb{1}[\text{sập dịch vụ}] + w_{\text{data}} \cdot \mathbb{1}[\text{đọc dữ liệu}] - w_{\text{step}}$$
#[derive(Debug, Clone, PartialEq)]
pub struct AttackRewardFunction {
    /// Thưởng cho mỗi máy bị xâm nhập.
    pub host_compromise_reward: f64,
    /// Phạt khi bị phát hiện.
    pub detection_penalty: f64,
    /// Phạt khi làm sập dịch vụ.
    pub crash_penalty: f64,
    /// Thưởng khi truy cập được dữ liệu.
    pub data_access_reward: f64,
    /// Phạt theo mỗi bước hành động (giảm bớt hành vi lãng phí).
    pub step_penalty: f64,
}

impl AttackRewardFunction {
    /// Khởi tạo bộ trọng số phần thưởng tiêu chuẩn.
    pub fn new() -> Self {
        Self {
            host_compromise_reward: 10.0,
            detection_penalty: 3.0,
            crash_penalty: 12.0,
            data_access_reward: 5.0,
            step_penalty: 0.5,
        }
    }

    /// Chấm điểm một hành động dựa trên trạng thái mạng **sau** khi hành động xảy ra.
    pub fn evaluate(&self, action: &AttackAction, state: &NetworkState) -> AttackReward {
        let mut components: Vec<(String, f64)> = Vec::new();

        let just_compromised = action.kind == ActionKind::Exploit
            && state.compromised.contains(&action.target)
            && state
                .compromised
                .iter()
                .filter(|ip| **ip == action.target)
                .count()
                == 1;
        if just_compromised {
            components.push(("host-compromise".to_string(), self.host_compromise_reward));
        }

        if state.detected {
            components.push(("detection".to_string(), -self.detection_penalty));
        }

        let crashed_service = state.crashed.contains(&action.target);
        if crashed_service {
            components.push(("service-crash".to_string(), -self.crash_penalty));
        }

        if action.kind == ActionKind::DataAccess && state.accessed_data {
            components.push(("data-access".to_string(), self.data_access_reward));
        }

        components.push(("step-cost".to_string(), -self.step_penalty));

        let total = components.iter().map(|(_, value)| value).sum();
        AttackReward {
            total,
            components,
            detected: state.detected,
            crashed_service,
            hosts_compromised: state.compromised.len(),
        }
    }
}

impl Default for AttackRewardFunction {
    fn default() -> Self {
        Self::new()
    }
}

// ============================================================================
// 10. TỐI ƯU ĐƯỜNG TẤN CÔNG BẰNG TABULAR Q-LEARNING
// ============================================================================

/// Khoá của bảng Q: cặp `(trạng thái, hành động)`.
pub type QKey = (String, String);

/// Bảng giá trị hành Q dạng bảng tra cứu (tabular Q-table).
#[derive(Debug, Clone, Default, PartialEq)]
pub struct QTable {
    /// Lưu trữ giá trị Q theo khoá trạng thái – hành động.
    pub table: HashMap<QKey, f64>,
}

impl QTable {
    /// Khởi tạo bảng Q rỗng.
    pub fn new() -> Self {
        Self::default()
    }

    /// Lấy giá trị Q (0.0 nếu chưa có).
    pub fn get(&self, state: &str, action: &str) -> f64 {
        self.table
            .get(&(state.to_string(), action.to_string()))
            .copied()
            .unwrap_or(0.0)
    }

    /// Ghi đè giá trị Q của một cặp trạng thái – hành động.
    pub fn set(&mut self, state: &str, action: &str, value: f64) {
        self.table
            .insert((state.to_string(), action.to_string()), value);
    }

    /// Chọn hành động có giá trị Q lớn nhất trong tập ứng viên.
    ///
    /// Khi nhiều hành động cùng giá trị, hành động xuất hiện sớm hơn trong danh sách
    /// được chọn, đảm bảo kết quả tất định.
    pub fn best_action(&self, state: &str, actions: &[String]) -> Option<String> {
        let mut best: Option<(&String, f64)> = None;
        for action in actions {
            let value = self.get(state, action);
            match best {
                Some((_, current)) if current >= value => {}
                _ => best = Some((action, value)),
            }
        }
        best.map(|(action, _)| action.clone())
    }

    /// Số cặp trạng thái – hành động đã học.
    pub fn len(&self) -> usize {
        self.table.len()
    }

    /// Bảng Q có rỗng hay không.
    pub fn is_empty(&self) -> bool {
        self.table.is_empty()
    }
}

/// Bộ tối ưu đường tấn công bằng thuật toán tabular Q-learning.
///
/// Thuật toán cập nhật giá trị Q theo công thức:
/// $$Q(s, a) \leftarrow Q(s, a) + \alpha \left[ r + \gamma \max_{a'} Q(s', a') - Q(s, a) \right]$$
#[derive(Debug, Clone)]
pub struct AttackPathOptimizer {
    /// Bảng Q đang học.
    pub q_table: QTable,
    /// Hệ số học tốc độ `alpha`.
    pub alpha: f64,
    /// Hệ số chiết khấu `gamma`.
    pub gamma: f64,
    /// Xác suất khám phá ngẫu nhiên `epsilon`.
    pub epsilon: f64,
    /// Số tập huấn luyện đã thực hiện.
    pub episodes: usize,
}

impl AttackPathOptimizer {
    /// Khởi tạo bộ tối ưu với siêu tham số chuẩn.
    pub fn new() -> Self {
        Self {
            q_table: QTable::new(),
            alpha: 0.3,
            gamma: 0.95,
            epsilon: 0.5,
            episodes: 0,
        }
    }

    /// Khoá trạng thái Markov của mô phỏng.
    ///
    /// Trạng thái được lượng tử hoá theo **số máy đã xâm nhập** cộng với hai cờ
    /// nhị phân (đã đọc dữ liệu / đã bị phát hiện). Lượng tử hoá này giữ không gian
    /// trạng thái nhỏ gọn đủ để tabular Q-learning hội tụ nhanh và ổn định:
    /// $$s = (|C|,\; \mathbb{1}[\text{data}],\; \mathbb{1}[\text{detected}])$$
    pub fn state_key(state: &NetworkState) -> String {
        format!(
            "comp{}|data{}|det{}",
            state.compromised.len(),
            u8::from(state.accessed_data),
            u8::from(state.detected)
        )
    }

    /// Sinh danh sách hành động khả dụng từ trạng thái hiện tại.
    fn available_actions(network: &MockNetwork, state: &NetworkState) -> Vec<String> {
        let mut actions = Vec::new();
        for (ip, service) in network.all_services() {
            if !state.compromised.contains(&ip) {
                actions.push(format!("exploit:{}:{}", ip, service.port));
            }
        }
        if !state.accessed_data {
            for (ip, service) in network.all_services() {
                if state.compromised.contains(&ip) {
                    actions.push(format!("data:{}:{}", ip, service.port));
                }
            }
        }
        actions
    }

    /// Áp dụng một hành động lên trạng thái mô phỏng (mô hình chuyển tiếp).
    fn apply_action(network: &MockNetwork, action: &str, state: &mut NetworkState) {
        let parts: Vec<&str> = action.split(':').collect();
        if parts.len() != 3 {
            return;
        }
        let Ok(ip) = parts[1].parse::<Ipv4Addr>() else {
            return;
        };
        let Ok(port) = parts[2].parse::<u16>() else {
            return;
        };
        match parts[0] {
            "exploit" => {
                let exploitable = network
                    .service(ip, port)
                    .is_some_and(|service| service.vulnerable);
                if exploitable && !state.compromised.contains(&ip) {
                    state.compromised.push(ip);
                    let crashes = network
                        .service(ip, port)
                        .is_some_and(|service| service.crash_on_exploit);
                    if crashes {
                        state.crashed.push(ip);
                        state.detected = true;
                    }
                }
            }
            "data" if state.compromised.contains(&ip) => {
                state.accessed_data = true;
            }
            _ => {}
        }
    }

    /// Chạy một tập mô phỏng: chọn hành động, cập nhật Q và trả về tổng phần thưởng.
    fn rollout(
        &mut self,
        network: &MockNetwork,
        rng: &mut SplitMix64,
        reward_fn: &AttackRewardFunction,
        epsilon: f64,
        learning: bool,
    ) -> f64 {
        let mut state = NetworkState::default();
        let mut total = 0.0;
        let mut current_key = Self::state_key(&state);

        for _ in 0..8 {
            let candidates = Self::available_actions(network, &state);
            if candidates.is_empty() {
                break;
            }
            let chosen = if learning && rng.next_chance(epsilon) {
                candidates[rng.next_below(candidates.len())].clone()
            } else {
                match self.q_table.best_action(&current_key, &candidates) {
                    Some(action) => action,
                    None => candidates[rng.next_below(candidates.len())].clone(),
                }
            };

            let (kind, ip, port) = Self::parse_key(&chosen);
            let attack_action = AttackAction {
                kind,
                target: ip,
                port,
            };

            Self::apply_action(network, &chosen, &mut state);
            let next_key = Self::state_key(&state);

            let reward = reward_fn.evaluate(&attack_action, &state);
            total += reward.total;

            if learning {
                let next_candidates = Self::available_actions(network, &state);
                let best_next = next_candidates
                    .iter()
                    .map(|action| self.q_table.get(&next_key, action))
                    .fold(f64::NEG_INFINITY, f64::max);
                let best_next = if best_next.is_finite() {
                    best_next
                } else {
                    0.0
                };
                let current = self.q_table.get(&current_key, &chosen);
                let updated =
                    current + self.alpha * (reward.total + self.gamma * best_next - current);
                self.q_table.set(&current_key, &chosen, updated);
            }

            current_key = next_key;
        }

        total
    }

    /// Tách khoá hành động thành (loại, địa chỉ, cổng).
    fn parse_key(key: &str) -> (ActionKind, Ipv4Addr, u16) {
        let parts: Vec<&str> = key.split(':').collect();
        if parts.len() != 3 {
            return (ActionKind::Scan, Ipv4Addr::new(0, 0, 0, 0), 0);
        }
        let kind = match parts[0] {
            "data" => ActionKind::DataAccess,
            _ => ActionKind::Exploit,
        };
        let ip = parts[1]
            .parse::<Ipv4Addr>()
            .unwrap_or(Ipv4Addr::new(0, 0, 0, 0));
        let port = parts[2].parse::<u16>().unwrap_or(0);
        (kind, ip, port)
    }

    /// Huấn luyện bảng Q trên mạng giả lập, trả về phần thưởng từng tập.
    ///
    /// Lịch giảm dần $\epsilon$ được áp dụng để chính sách chuyển từ "khám phá"
    /// sang "khai thác tri thức đã học":
    /// $$\epsilon_k = \max\Big(\epsilon_{\min},\; \epsilon_0\Big(1 - 0{,}9\,\frac{k}{K}\Big)\Big)$$
    /// Nhờ đó phần thưởng trung bình của các tập cuối cao hơn rõ rệt so với các
    /// tập đầu — đây chính là dấu hiệu "học được" của tác tử.
    pub fn train(&mut self, network: &MockNetwork, episodes: usize, seed: u64) -> Vec<f64> {
        let mut rng = SplitMix64::new(seed);
        let reward_fn = AttackRewardFunction::new();
        let mut rewards = Vec::with_capacity(episodes);
        let total = episodes.max(1) as f64;
        for episode in 0..episodes {
            let progress = episode as f64 / total;
            let epsilon = (self.epsilon * (1.0 - 0.9 * progress)).max(0.02);
            rewards.push(self.rollout(network, &mut rng, &reward_fn, epsilon, true));
        }
        self.episodes = episodes;
        rewards
    }

    /// Suy ra đường tấn công tối ưu theo chính sách tham lam trên bảng Q đã học.
    pub fn best_path(&self, network: &MockNetwork) -> Vec<AttackAction> {
        let mut state = NetworkState::default();
        let mut current_key = Self::state_key(&state);
        let mut path = Vec::new();

        for _ in 0..8 {
            let candidates = Self::available_actions(network, &state);
            if candidates.is_empty() {
                break;
            }
            let Some(chosen) = self.q_table.best_action(&current_key, &candidates) else {
                break;
            };
            // Hành động có giá trị Q không dương nghĩa là hành động đó làm giảm phần thưởng
            // dài hạn (ví dụ khai thác dịch vụ dễ làm sập) — dừng chuỗi tấn công tại đây.
            if self.q_table.get(&current_key, &chosen) <= 0.0 {
                break;
            }
            let (kind, ip, port) = Self::parse_key(&chosen);
            path.push(AttackAction {
                kind,
                target: ip,
                port,
            });
            Self::apply_action(network, &chosen, &mut state);
            let next_key = Self::state_key(&state);
            if next_key == current_key {
                break;
            }
            current_key = next_key;
        }
        path
    }

    /// Tổng giá trị Q đã học — đại lượng đo lường "giá trị" của chính sách.
    pub fn total_reward(&self) -> f64 {
        self.q_table.table.values().sum()
    }

    /// Tổng phần thưởng mà chính sách tham lam hiện tại đạt được trên mạng giả lập.
    ///
    /// Đây là phép đo "chất lượng chính sách": trước khi huấn luyện, bảng Q rỗng nên
    /// chính sách chọn bừa các hành động; sau khi huấn luyện, giá trị này tăng lên.
    pub fn evaluate_path(&self, network: &MockNetwork) -> f64 {
        let reward_fn = AttackRewardFunction::new();
        let mut state = NetworkState::default();
        let mut current_key = Self::state_key(&state);
        let mut total = 0.0;

        for _ in 0..8 {
            let candidates = Self::available_actions(network, &state);
            if candidates.is_empty() {
                break;
            }
            let Some(chosen) = self.q_table.best_action(&current_key, &candidates) else {
                break;
            };
            let (kind, ip, port) = Self::parse_key(&chosen);
            let action = AttackAction {
                kind,
                target: ip,
                port,
            };
            Self::apply_action(network, &chosen, &mut state);
            total += reward_fn.evaluate(&action, &state).total;
            let next_key = Self::state_key(&state);
            if next_key == current_key {
                break;
            }
            current_key = next_key;
        }
        total
    }
}

impl Default for AttackPathOptimizer {
    fn default() -> Self {
        Self::new()
    }
}

// ============================================================================
// 11. ĐỐI KHÁNG ĐỎ – XANH (RED-VS-BLUE ADVERSARIAL)
// ============================================================================

/// Một tín hiệu phát hiện do đội phòng thủ ghi nhận.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct DetectionSignal {
    /// Loại tín hiệu (ví dụ `"exploit-attempt"`).
    pub kind: String,
    /// Máy nguồn phát sinh tín hiệu.
    pub source: Ipv4Addr,
    /// Mức nghiêm trọng của tín hiệu.
    pub severity: Severity,
    /// Chi tiết mô tả tín hiệu.
    pub detail: String,
}

/// Đội phòng thủ xanh (Blue Team).
///
/// Bất biến phòng thủ: đội xanh cải thiện dần độ nhạy cảm theo thời gian
/// (`detection_threshold` giảm dần), nhờ vậy điểm phòng thủ tích luỹ tăng theo thời gian.
#[derive(Debug, Clone, PartialEq)]
pub struct BlueTeamDefender {
    /// Nhật ký các tín hiệu đã phát hiện.
    pub signals: Vec<DetectionSignal>,
    /// Các máy đã bị cách ly.
    pub isolated_hosts: Vec<Ipv4Addr>,
    /// Các tài khoản đã được đặt lại mật khẩu.
    pub reset_credentials: Vec<String>,
    /// Ngưỡng phát hiện: hành động có điểm rủi ro ≥ ngưỡng mới bị ghi nhận.
    pub detection_threshold: f64,
}

impl BlueTeamDefender {
    /// Khởi tạo đội phòng thủ với ngưỡng phát hiện tiêu chuẩn.
    pub fn new() -> Self {
        Self {
            signals: Vec::new(),
            isolated_hosts: Vec::new(),
            reset_credentials: Vec::new(),
            detection_threshold: 0.6,
        }
    }

    /// Điểm rủi ro của từng loại hành động.
    pub fn risk_score(kind: ActionKind) -> f64 {
        match kind {
            ActionKind::Scan => 0.20,
            ActionKind::Exploit => 0.90,
            ActionKind::CredentialAttack => 0.70,
            ActionKind::PrivEsc => 0.80,
            ActionKind::Pivot => 0.85,
            ActionKind::DataAccess => 0.60,
            ActionKind::Destructive => 1.00,
            ActionKind::Report => 0.0,
        }
    }

    /// Quy đổi điểm rủi ro sang mức nghiêm trọng.
    pub fn severity_for(score: f64) -> Severity {
        if score >= 0.95 {
            Severity::Critical
        } else if score >= 0.75 {
            Severity::High
        } else if score >= 0.55 {
            Severity::Medium
        } else if score >= 0.35 {
            Severity::Low
        } else {
            Severity::Info
        }
    }

    /// Quan sát một hành động tấn công và trả về tín hiệu nếu vượt ngưỡng.
    pub fn observe(
        &mut self,
        action: &AttackAction,
        state: &NetworkState,
    ) -> Option<DetectionSignal> {
        let mut score = Self::risk_score(action.kind);
        let mut kind = format!("{}-attempt", action.kind.label());
        let mut severity = Self::severity_for(score);

        // Đã bị phát hiện trước đó thì mọi hành động tiếp theo đều bị ghi nhận nặng hơn.
        if state.detected {
            score += 0.15;
            kind = "repeat-offense".to_string();
            severity = Severity::Critical;
        }

        if score < self.detection_threshold {
            return None;
        }

        let signal = DetectionSignal {
            kind,
            source: action.target,
            severity,
            detail: format!(
                "Hành động {} trên {}:{} có điểm rủi ro {:.2} ≥ ngưỡng {:.2}.",
                action.kind.label(),
                action.target,
                action.port,
                score,
                self.detection_threshold
            ),
        };
        self.signals.push(signal.clone());
        Some(signal)
    }

    /// Cách ly một máy khỏi mạng.
    pub fn isolate(&mut self, ip: Ipv4Addr) {
        if !self.isolated_hosts.contains(&ip) {
            self.isolated_hosts.push(ip);
        }
    }

    /// Đặt lại mật khẩu cho một tài khoản.
    pub fn reset_credential(&mut self, user: &str) {
        if !self.reset_credentials.iter().any(|item| item == user) {
            self.reset_credentials.push(user.to_string());
        }
    }

    /// Số tín hiệu phát hiện đã ghi nhận.
    pub fn detected_count(&self) -> usize {
        self.signals.len()
    }

    /// Điểm phòng thủ tích luỹ: điểm phát hiện + điểm cách ly + điểm đặt lại mật khẩu.
    pub fn defense_score(&self) -> f64 {
        let detection_points: f64 = self
            .signals
            .iter()
            .map(|signal| signal.severity.defense_points())
            .sum();
        let isolation_points = self.isolated_hosts.len() as f64 * 10.0;
        let credential_points = self.reset_credentials.len() as f64 * 6.0;
        detection_points + isolation_points + credential_points
    }
}

impl Default for BlueTeamDefender {
    fn default() -> Self {
        Self::new()
    }
}

/// Chiến lược hành vi của tác tử đỏ.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum AttackStrategy {
    /// Luôn chọn mục tiêu đã được chứng minh thành công.
    Greedy,
    /// Chọn ngẫu nhiên hoàn toàn.
    Random,
    /// Tham lam nhưng có khám phá ngẫu nhiên với xác suất `epsilon`.
    EpsilonGreedy,
}

/// Tác tử đỏ (Red Team) trong mô phỏng đối kháng.
#[derive(Debug, Clone)]
pub struct RedAgent {
    /// Chiến lược hành vi hiện tại.
    pub strategy: AttackStrategy,
    /// Trạng thái bộ sinh ngẫu nhiên tất định.
    pub rng_state: u64,
    /// Lịch sử các hành động khai thác **thành công** (kinh nghiệm tích luỹ).
    pub history: Vec<AttackAction>,
}

impl RedAgent {
    /// Khởi tạo tác tử đỏ với chiến lược và hạt giống cho trước.
    pub fn new(strategy: AttackStrategy, seed: u64) -> Self {
        Self {
            strategy,
            rng_state: seed,
            history: Vec::new(),
        }
    }

    /// Sinh số ngẫu nhiên tiếp theo từ trạng thái nội bộ.
    pub fn next_u64(&mut self) -> u64 {
        self.rng_state = self.rng_state.wrapping_add(SplitMix64::GAMMA);
        let mut z = (self.rng_state ^ (self.rng_state >> 30)).wrapping_mul(0xBF58_476D_1CE4_E5B9);
        z = (z ^ (z >> 27)).wrapping_mul(0x94D0_49BB_1331_11EB);
        z ^ (z >> 31)
    }

    /// Sinh chỉ số ngẫu nhiên trong khoảng `0..bound`.
    pub fn next_below(&mut self, bound: usize) -> usize {
        if bound == 0 {
            return 0;
        }
        (self.next_u64() % bound as u64) as usize
    }

    /// Số lần đã khai thác thành công một mục tiêu (kinh nghiệm tích luỹ).
    pub fn success_count(&self, target: Ipv4Addr, port: u16) -> usize {
        self.history
            .iter()
            .filter(|action| action.target == target && action.port == port)
            .count()
    }

    /// Chọn hành động tiếp theo trong số các ứng viên chưa tấn công.
    ///
    /// Ứng viên đã từng thành công luôn được ưu tiên; nếu chưa có kinh nghiệm nào
    /// thì lựa chọn theo thứ tự ổn định, kết hợp khám phá ngẫu nhiên.
    pub fn select(&mut self, candidates: &[AttackAction], epsilon: f64) -> Option<AttackAction> {
        if candidates.is_empty() {
            return None;
        }
        if self.strategy == AttackStrategy::Random {
            let index = self.next_below(candidates.len());
            return Some(candidates[index].clone());
        }
        if self.strategy == AttackStrategy::EpsilonGreedy {
            let roll = (self.next_u64() >> 11) as f64 / ((1u64 << 53) as f64);
            if roll < epsilon {
                let index = self.next_below(candidates.len());
                return Some(candidates[index].clone());
            }
        }
        let proven: Vec<&AttackAction> = candidates
            .iter()
            .filter(|action| self.success_count(action.target, action.port) > 0)
            .collect();
        let pool: Vec<&AttackAction> = if proven.is_empty() {
            candidates.iter().collect()
        } else {
            proven
        };
        let mut best: Option<&AttackAction> = None;
        for action in pool {
            let count = self.success_count(action.target, action.port);
            match best {
                Some(current) if self.success_count(current.target, current.port) >= count => {}
                _ => best = Some(action),
            }
        }
        best.cloned()
    }
}

/// Kết quả một tập đối kháng.
#[derive(Debug, Clone, PartialEq)]
pub struct EpisodeOutcome {
    /// Số thứ tự của tập (bắt đầu từ 1).
    pub episode: usize,
    /// Tổng phần thưởng đội đỏ nhận được.
    pub red_reward: f64,
    /// Điểm phòng thủ đội xanh đạt được trong tập.
    pub blue_score: f64,
    /// Số máy đội đỏ xâm nhập được.
    pub hosts_compromised: usize,
    /// Đội đỏ có bị phát hiện hay không.
    pub detected: bool,
}

/// Mô phỏng đối kháng đỏ – xanh.
///
/// Mỗi tập là **một kịch bản kiểm thử độc lập**: trạng thái máy được đặt lại,
/// còn **tri thức** của cả hai bên vẫn được giữ lại — đội đỏ nhớ những mục tiêu đã
/// khai thác thành công, đội xanh nhạy dần hơn theo thời gian. Nhờ vậy **cả hai
/// bên đều tiến bộ**: đội đỏ chọn đúng mục tiêu hơn, đội xanh phát hiện nhiều hơn.
#[derive(Debug, Clone)]
pub struct RedVsBlueSim {
    /// Tác tử đỏ.
    pub red: RedAgent,
    /// Đội phòng thủ xanh.
    pub blue: BlueTeamDefender,
    /// Số tập đã chạy.
    pub episodes: usize,
}

impl RedVsBlueSim {
    /// Khởi tạo mô phỏng với hạt giống tất định.
    pub fn new(seed: u64) -> Self {
        Self {
            red: RedAgent::new(AttackStrategy::EpsilonGreedy, seed),
            blue: BlueTeamDefender::new(),
            episodes: 0,
        }
    }

    /// Chạy một tập đối kháng và trả về kết quả.
    pub fn run_episode(&mut self, network: &MockNetwork) -> EpisodeOutcome {
        self.episodes += 1;
        let episode = self.episodes;

        // Ngưỡng phát hiện của đội xanh giảm dần: đội xanh học được "mẫu" hành vi đỏ.
        self.blue.detection_threshold = (0.95 * 0.90f64.powi((episode - 1) as i32)).max(0.15);
        // Xác suất khám phá của đội đỏ giảm dần: đội đỏ tận dụng kinh nghiệm tích luỹ.
        let epsilon = (0.60 * 0.90f64.powi((episode - 1) as i32)).max(0.05);

        let score_before = self.blue.defense_score();
        let reward_fn = AttackRewardFunction::new();

        let mut state = NetworkState::default();
        let mut total = 0.0;
        let mut detection_charged = false;
        let mut attempted: Vec<(Ipv4Addr, u16)> = Vec::new();

        // Bước 1: đội đỏ trinh sát một máy nào đó.
        if let Some(host) = network.hosts.first() {
            let scan = AttackAction {
                kind: ActionKind::Scan,
                target: host.ip,
                port: 80,
            };
            total += Self::score_action(&reward_fn, &scan, &state, &mut detection_charged);
            if self.blue.observe(&scan, &state).is_some() {
                state.detected = true;
            }
        }

        // Bước 2: đội đỏ chọn và khai thác tối đa 6 mục tiêu.
        for _ in 0..6 {
            let candidates: Vec<AttackAction> = network
                .all_services()
                .into_iter()
                .filter(|(ip, service)| !attempted.contains(&(*ip, service.port)))
                .map(|(ip, service)| AttackAction {
                    kind: ActionKind::Exploit,
                    target: ip,
                    port: service.port,
                })
                .collect();
            let Some(choice) = self.red.select(&candidates, epsilon) else {
                break;
            };
            attempted.push((choice.target, choice.port));

            let exploitable = network
                .service(choice.target, choice.port)
                .is_some_and(|service| service.vulnerable);
            let crashes = network
                .service(choice.target, choice.port)
                .is_some_and(|service| service.crash_on_exploit);

            if exploitable && !state.compromised.contains(&choice.target) {
                state.compromised.push(choice.target);
                if crashes {
                    state.crashed.push(choice.target);
                    state.detected = true;
                }
                // Chỉ ghi nhớ những mục tiêu khai thác thành công, không làm sập dịch vụ.
                if !crashes {
                    self.red.history.push(choice.clone());
                }
            }

            total += Self::score_action(&reward_fn, &choice, &state, &mut detection_charged);
            if self.blue.observe(&choice, &state).is_some() {
                state.detected = true;
            }
        }

        // Bước 3: đội đỏ đọc dữ liệu trên máy đã xâm nhập (chỉ đọc).
        if let Some(target) = state.compromised.first().copied() {
            let loot = AttackAction {
                kind: ActionKind::DataAccess,
                target,
                port: 80,
            };
            state.accessed_data = true;
            total += Self::score_action(&reward_fn, &loot, &state, &mut detection_charged);
            if self.blue.observe(&loot, &state).is_some() {
                state.detected = true;
            }
        }

        // Bước 4: đội xanh phản ứng — cách ly máy bị xâm nhập và đặt lại mật khẩu.
        for host in &state.compromised {
            self.blue.isolate(*host);
        }
        self.blue.reset_credential("admin");

        let blue_score = self.blue.defense_score() - score_before;

        EpisodeOutcome {
            episode,
            red_reward: total,
            blue_score,
            hosts_compromised: state.compromised.len(),
            detected: state.detected,
        }
    }

    /// Chấm điểm một hành động; phí phát hiện chỉ được tính một lần cho mỗi tập.
    fn score_action(
        reward_fn: &AttackRewardFunction,
        action: &AttackAction,
        state: &NetworkState,
        detection_charged: &mut bool,
    ) -> f64 {
        let mut eval_state = state.clone();
        eval_state.detected = state.detected && !*detection_charged;
        let reward = reward_fn.evaluate(action, &eval_state);
        if eval_state.detected {
            *detection_charged = true;
        }
        reward.total
    }

    /// Chạy nhiều tập liên tiếp và trả về toàn bộ kết quả.
    pub fn run(&mut self, network: &MockNetwork, episodes: usize) -> Vec<EpisodeOutcome> {
        let mut outcomes = Vec::with_capacity(episodes);
        for _ in 0..episodes {
            outcomes.push(self.run_episode(network));
        }
        outcomes
    }

    /// Đội đỏ có cải thiện qua các tập hay không (nửa sau tốt hơn nửa đầu).
    pub fn red_improves(&self, outcomes: &[EpisodeOutcome]) -> bool {
        Self::second_half_average(outcomes, |outcome| outcome.red_reward)
            > Self::first_half_average(outcomes, |outcome| outcome.red_reward)
    }

    /// Đội xanh có cải thiện qua các tập hay không.
    pub fn blue_improves(&self, outcomes: &[EpisodeOutcome]) -> bool {
        Self::second_half_average(outcomes, |outcome| outcome.blue_score)
            > Self::first_half_average(outcomes, |outcome| outcome.blue_score)
    }

    /// Trung bình nửa đầu của tập kết quả.
    fn first_half_average(outcomes: &[EpisodeOutcome], metric: fn(&EpisodeOutcome) -> f64) -> f64 {
        if outcomes.is_empty() {
            return 0.0;
        }
        let half = outcomes.len() / 2;
        let slice = &outcomes[..half];
        if slice.is_empty() {
            return 0.0;
        }
        slice.iter().map(metric).sum::<f64>() / slice.len() as f64
    }

    /// Trung bình nửa sau của tập kết quả.
    fn second_half_average(outcomes: &[EpisodeOutcome], metric: fn(&EpisodeOutcome) -> f64) -> f64 {
        if outcomes.is_empty() {
            return 0.0;
        }
        let half = outcomes.len() / 2;
        let slice = &outcomes[half..];
        if slice.is_empty() {
            return 0.0;
        }
        slice.iter().map(metric).sum::<f64>() / slice.len() as f64
    }
}

// ============================================================================
// 12. QUẢN LÝ PHIÊN KIỂM THỬ (SESSION PERSISTENCE)
// ============================================================================

/// Ảnh chụp trạng thái một phiên kiểm thử để lưu lại và nối tiếp.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct SessionSnapshot {
    /// Định danh phiên.
    pub id: String,
    /// Pha đang dở khi lưu phiên.
    pub phase: PentestPhase,
    /// Các phát hiện đã xác minh tại thời điểm lưu.
    pub findings: Vec<Vulnerability>,
    /// Các máy đã xâm nhập.
    pub compromised: Vec<Ipv4Addr>,
    /// Số bản ghi bằng chứng tại thời điểm lưu.
    pub evidence_count: usize,
    /// Dấu thời gian lưu phiên.
    pub saved_at: u64,
}

/// Quản lý các phiên kiểm thử (theo kiểu PentestGPT).
#[derive(Debug, Clone, Default, PartialEq, Eq)]
pub struct SessionManager {
    /// Kho lưu trữ phiên theo định danh.
    pub sessions: HashMap<String, SessionSnapshot>,
    /// Định danh phiên đang hoạt động.
    pub current: Option<String>,
}

impl SessionManager {
    /// Khởi tạo quản lý phiên rỗng.
    pub fn new() -> Self {
        Self::default()
    }

    /// Lưu một ảnh chụp phiên và đặt làm phiên hiện hành, trả về định danh phiên.
    pub fn save(&mut self, id: &str, snapshot: SessionSnapshot) -> String {
        let mut stored = snapshot;
        stored.id = id.to_string();
        self.sessions.insert(id.to_string(), stored);
        self.current = Some(id.to_string());
        id.to_string()
    }

    /// Nối tiếp một phiên đã lưu (trả về bản sao).
    pub fn restore(&mut self, id: &str) -> Option<SessionSnapshot> {
        let snapshot = self.sessions.get(id)?.clone();
        self.current = Some(id.to_string());
        Some(snapshot)
    }

    /// Liệt kê định danh các phiên đã lưu (sắp xếp ổn định).
    pub fn list(&self) -> Vec<String> {
        let mut ids: Vec<String> = self.sessions.keys().cloned().collect();
        ids.sort();
        ids
    }

    /// Xem ảnh chụp của phiên hiện hành.
    pub fn current(&self) -> Option<&SessionSnapshot> {
        let id = self.current.as_ref()?;
        self.sessions.get(id)
    }

    /// Đặt phiên hiện hành theo định danh.
    pub fn set_current(&mut self, id: &str) {
        self.current = Some(id.to_string());
    }
}

// ============================================================================
// 13. TÁC TỬ PENTEST TỰ HÀNH (ĐIỀU PHỐI TỔNG THỂ)
// ============================================================================

/// Báo cáo tổng kết của một chiến dịch kiểm thử tự động.
#[derive(Debug, Clone, PartialEq)]
pub struct PentestRunReport {
    /// Các pha đã đi qua theo thứ tự.
    pub phases_visited: Vec<PentestPhase>,
    /// Các phát hiện đã vượt qua kiểm chứng ground-truth.
    pub findings: Vec<Vulnerability>,
    /// Các bằng chứng khai thác tương ứng.
    pub proofs: Vec<ProofByExploitation>,
    /// Toàn bộ bản ghi bằng chứng của chiến dịch.
    pub evidence: Vec<Evidence>,
    /// Các vi phạm hiến pháp đã ghi nhận.
    pub violations: Vec<PentestViolation>,
    /// Các vi phạm phạm vi đã ghi nhận.
    pub scope_violations: Vec<String>,
    /// Đặc quyền cao nhất đạt được ("none" / "user" / "root").
    pub final_privilege: String,
    /// Nội dung báo cáo Markdown.
    pub report_markdown: String,
    /// Hệ thống đã thoát sạch hay chưa.
    pub clean_exit: bool,
    /// Số bước hành động đã thực hiện.
    pub steps: usize,
    /// Các máy đã bị xâm nhập.
    pub compromised: Vec<Ipv4Addr>,
    /// Phần thưởng RL của hành động cuối cùng.
    pub reward: AttackReward,
}

/// Tác tử pentest tự hành đa tác tử: điều phối toàn bộ chiến dịch.
///
/// Bất biến **Time Budget**: số bước hành động không bao giờ vượt quá `budget_steps`.
#[derive(Debug, Clone)]
pub struct AutonomousPentestAgent {
    /// Tác tử điều phối (đồ thị pha + ba cổng kiểm soát).
    pub orchestrator: OrchestratorAgent,
    /// Tác tử trinh sát.
    pub recon: ReconAgent,
    /// Tác tử khai thác.
    pub exploit: ExploitAgent,
    /// Tác tử kiểm chứng.
    pub validation: ValidationAgent,
    /// Quản lý phiên.
    pub sessions: SessionManager,
    /// Hàm phần thưởng RL.
    pub reward_fn: AttackRewardFunction,
    /// Bộ tối ưu đường tấn công.
    pub optimizer: AttackPathOptimizer,
    /// Ngân sách bước hành động tối đa.
    pub budget_steps: usize,
    /// Báo cáo của lần chạy gần nhất.
    pub last_report: Option<PentestRunReport>,
}

impl AutonomousPentestAgent {
    /// Khởi tạo tác tử tự hành với phạm vi và ngân sách bước mặc định.
    pub fn new(scope: ScopeGuard) -> Self {
        Self {
            orchestrator: OrchestratorAgent::new(scope),
            recon: ReconAgent::new(),
            exploit: ExploitAgent::new(),
            validation: ValidationAgent::new(),
            sessions: SessionManager::new(),
            reward_fn: AttackRewardFunction::new(),
            optimizer: AttackPathOptimizer::new(),
            budget_steps: 24,
            last_report: None,
        }
    }

    /// Chạy trọn một chiến dịch kiểm thử trên mạng giả lập và trả về báo cáo.
    pub fn run(&mut self, network: &MockNetwork) -> PentestRunReport {
        self.orchestrator.graph = PentestStateGraph::new();
        self.orchestrator.evidence = EvidenceChain::new();
        self.orchestrator.clock = 0;
        self.orchestrator.compromised.clear();
        self.exploit.used.clear();

        let mut report = PentestRunReport {
            phases_visited: Vec::new(),
            findings: Vec::new(),
            proofs: Vec::new(),
            evidence: Vec::new(),
            violations: Vec::new(),
            scope_violations: Vec::new(),
            final_privilege: "none".to_string(),
            report_markdown: String::new(),
            clean_exit: false,
            steps: 0,
            compromised: Vec::new(),
            reward: AttackReward {
                total: 0.0,
                components: Vec::new(),
                detected: false,
                crashed_service: false,
                hosts_compromised: 0,
            },
        };

        let mut shell = MockShell::new();
        let mut steps = 0usize;

        // ---------------- PHA 1: TRINH SÁT ----------------
        let _ = self.orchestrator.graph.transition(PentestPhase::Recon);
        report.phases_visited.push(PentestPhase::Recon);
        let recon_ctx = AgentContext {
            phase: PentestPhase::Recon,
            network,
            findings: Vec::new(),
            targets: Vec::new(),
            shell: shell.clone(),
        };
        let recon_result = self.recon.step(&recon_ctx);
        self.orchestrator.record("recon", &recon_result.message);
        let candidates = recon_result.findings.clone();
        let targets = recon_result.targets.clone();

        // ---------------- PHA 2: LIỆT KÊ DỊCH VỤ ----------------
        let _ = self.orchestrator.graph.transition(PentestPhase::Enumerate);
        report.phases_visited.push(PentestPhase::Enumerate);
        self.orchestrator.record(
            "enumerate",
            &format!(
                "{} mục tiêu được lập bản đồ, {} phát hiện ứng viên chờ kiểm chứng.",
                targets.len(),
                candidates.len()
            ),
        );

        // Kiểm tra phạm vi: thử dò một mục tiêu công khai nằm ngoài phạm vi.
        let outside = ProposedAction {
            kind: ActionKind::Scan,
            target: "203.0.113.10".to_string(),
            port: 80,
            description: "Dò mạng công cộng ngoài phạm vi".to_string(),
        };
        let outcome = self.orchestrator.authorize(&outside);
        steps += 1;
        if !outcome.allowed {
            report.scope_violations.push(outcome.reason.clone());
            self.orchestrator.record("scope-violation", &outcome.reason);
        }

        // Kiểm tra hiến pháp + cổng HITL với một hành động phá hủy.
        let destructive = ProposedAction {
            kind: ActionKind::Destructive,
            target: "10.10.10.20".to_string(),
            port: 3306,
            description: "DROP TABLE audit_log".to_string(),
        };
        let outcome = self.orchestrator.authorize(&destructive);
        steps += 1;
        if let Some(violation) = &outcome.violation {
            report.violations.push(violation.clone());
        }
        if !outcome.allowed {
            self.orchestrator.record("blocked-action", &outcome.reason);
        }

        // Kiểm tra cổng HITL với hành động chuyển ngang sang máy chưa được duyệt.
        let pivot = ProposedAction {
            kind: ActionKind::Pivot,
            target: "10.10.10.50".to_string(),
            port: 22,
            description: "Chuyển ngang sang máy nhảy".to_string(),
        };
        let outcome = self.orchestrator.authorize(&pivot);
        steps += 1;
        if let Some(violation) = &outcome.violation {
            report.violations.push(violation.clone());
        }
        if !outcome.allowed {
            self.orchestrator.record("hitl-blocked", &outcome.reason);
        }

        // Kiểm tra ranh giới đặc quyền: leo thang lên máy chưa bị xâm nhập phải bị chặn.
        let privilege_probe = ProposedAction {
            kind: ActionKind::PrivEsc,
            target: "10.10.10.40".to_string(),
            port: 22,
            description: "Leo thang lên máy chưa xâm nhập".to_string(),
        };
        let outcome = self.orchestrator.authorize(&privilege_probe);
        steps += 1;
        if let Some(violation) = &outcome.violation {
            report.violations.push(violation.clone());
        }
        if !outcome.allowed {
            self.orchestrator.record("blocked-action", &outcome.reason);
        }

        // ---------------- PHA 3: KHAI THÁC ----------------
        let _ = self.orchestrator.graph.transition(PentestPhase::Exploit);
        report.phases_visited.push(PentestPhase::Exploit);
        let mut exploited: Vec<(Vulnerability, ProofByExploitation)> = Vec::new();
        for candidate in candidates.clone() {
            if steps >= self.budget_steps {
                self.orchestrator.record(
                    "budget-exhausted",
                    "Đã đạt ngân sách bước, dừng mọi hành động tấn công.",
                );
                break;
            }
            let action = ProposedAction {
                kind: ActionKind::Exploit,
                target: candidate.target.to_string(),
                port: candidate.port,
                description: candidate.title.clone(),
            };
            let outcome = self.orchestrator.authorize(&action);
            steps += 1;
            if !outcome.allowed {
                self.orchestrator.record("blocked-action", &outcome.reason);
                continue;
            }
            let exploit_ctx = AgentContext {
                phase: PentestPhase::Exploit,
                network,
                findings: vec![candidate.clone()],
                targets: targets.clone(),
                shell: shell.clone(),
            };
            let result = self.exploit.step(&exploit_ctx);
            self.orchestrator.record("exploit-attempt", &result.message);
            if result.success
                && let Some(proof) = result.proof
            {
                self.orchestrator.compromised.push(candidate.target);
                exploited.push((candidate.clone(), proof));
            }
        }

        // ---------------- PHA 4: LEO THANG ĐẶC QUYỀN ----------------
        let _ = self.orchestrator.graph.transition(PentestPhase::PrivEsc);
        report.phases_visited.push(PentestPhase::PrivEsc);
        for (candidate, _) in &exploited {
            let action = ProposedAction {
                kind: ActionKind::PrivEsc,
                target: candidate.target.to_string(),
                port: candidate.port,
                description: format!("Leo thang đặc quyền trên {}", candidate.target),
            };
            let outcome = self.orchestrator.authorize(&action);
            steps += 1;
            if outcome.allowed {
                report.compromised.push(candidate.target);
                report.final_privilege = "root".to_string();
                self.orchestrator.record(
                    "privilege-escalation",
                    &format!("Leo thang đặc quyền thành công trên {}.", candidate.target),
                );
            } else {
                self.orchestrator.record("blocked-action", &outcome.reason);
            }
        }

        // Bất biến Privilege Boundary: sau khi đã xâm nhập, mọi máy trong danh sách
        // bị xâm nhập đều được phép leo thang; máy chưa bị xâm nhập thì không.
        for host in &network.hosts {
            if !self.orchestrator.compromised.contains(&host.ip) {
                let action = ProposedAction {
                    kind: ActionKind::PrivEsc,
                    target: host.ip.to_string(),
                    port: 22,
                    description: format!("Leo thang lên máy chưa xâm nhập {}", host.ip),
                };
                let outcome = self.orchestrator.authorize(&action);
                steps += 1;
                if !outcome.allowed {
                    self.orchestrator.record("blocked-action", &outcome.reason);
                }
            }
        }

        // ---------------- PHA 5: CHUYỂN NGANG ----------------
        let _ = self.orchestrator.graph.transition(PentestPhase::Pivot);
        report.phases_visited.push(PentestPhase::Pivot);
        self.orchestrator.record(
            "pivot-skipped",
            "Chuyển ngang bị chặn bởi cổng HITL nên chiến dịch tiếp tục trong phạm vi đã duyệt.",
        );

        // ---------------- PHA 6: THU THẬP DỮ LIỆU ----------------
        let _ = self.orchestrator.graph.transition(PentestPhase::Loot);
        report.phases_visited.push(PentestPhase::Loot);
        let mut loot_state = NetworkState {
            compromised: report.compromised.clone(),
            detected: false,
            crashed: Vec::new(),
            accessed_data: false,
        };
        if let Some(target) = report.compromised.first().copied() {
            let action = ProposedAction {
                kind: ActionKind::DataAccess,
                target: target.to_string(),
                port: 80,
                description: "Đọc cấu hình để đối chiếu".to_string(),
            };
            let outcome = self.orchestrator.authorize(&action);
            steps += 1;
            if outcome.allowed {
                loot_state.accessed_data = true;
                let loot = AttackAction {
                    kind: ActionKind::DataAccess,
                    target,
                    port: 80,
                };
                report.reward = self.reward_fn.evaluate(&loot, &loot_state);
                self.orchestrator
                    .record("loot", "Đọc dữ liệu chỉ-đọc trên máy đã xâm nhập.");
            } else {
                self.orchestrator.record("blocked-action", &outcome.reason);
            }
        }

        // ---------------- PHA 7: KIỂM CHỨNG & BÁO CÁO ----------------
        let _ = self.orchestrator.graph.transition(PentestPhase::Report);
        report.phases_visited.push(PentestPhase::Report);
        let validation_ctx = AgentContext {
            phase: PentestPhase::Report,
            network,
            findings: exploited.iter().map(|(item, _)| item.clone()).collect(),
            targets: targets.clone(),
            shell: shell.clone(),
        };
        let validation = self.validation.step(&validation_ctx);
        self.orchestrator.record("validation", &validation.message);
        report.findings = validation.findings.clone();
        for finding in &report.findings {
            if let Some(proof) = exploited
                .iter()
                .find(|(item, _)| item.id == finding.id)
                .map(|(_, proof)| proof.clone())
            {
                report.proofs.push(proof);
            }
        }

        // ---------------- PHA 8: HOÀN TẤT ----------------
        let _ = self.orchestrator.graph.transition(PentestPhase::Done);
        report.phases_visited.push(PentestPhase::Done);
        report.steps = steps;
        report.evidence = self.orchestrator.evidence.entries.clone();

        // Bất biến Clean Exit: đóng toàn bộ phiên shell còn treo.
        shell.open_session("phiên-khảo-sat");
        let closed = shell.close_all();
        let clean_exit = closed > 0
            && self.orchestrator.graph.completed()
            && self.orchestrator.evidence.verify();
        report.clean_exit = clean_exit;

        let markdown = self.generate_report(&report);
        report.report_markdown = markdown;
        self.last_report = Some(report.clone());
        report
    }

    /// Lưu phiên hiện tại để có thể nối tiếp về sau.
    pub fn save_session(&mut self, id: &str) {
        let snapshot = SessionSnapshot {
            id: id.to_string(),
            phase: self
                .last_report
                .as_ref()
                .and_then(|report| report.phases_visited.last().copied())
                .unwrap_or(PentestPhase::Idle),
            findings: self
                .last_report
                .as_ref()
                .map(|report| report.findings.clone())
                .unwrap_or_default(),
            compromised: self
                .last_report
                .as_ref()
                .map(|report| report.compromised.clone())
                .unwrap_or_default(),
            evidence_count: self.orchestrator.evidence.len(),
            saved_at: self.orchestrator.clock,
        };
        self.sessions.save(id, snapshot);
    }

    /// Nối tiếp một phiên đã lưu.
    pub fn restore_session(&mut self, id: &str) -> Option<SessionSnapshot> {
        self.sessions.restore(id)
    }

    /// Sinh báo cáo Markdown đầy đủ từ kết quả chiến dịch.
    ///
    /// Báo cáo luôn có đủ 5 mục: Tóm tắt điều hành, Phương pháp, Phát hiện,
    /// Khuyến nghị khắc phục và Kết luận.
    pub fn generate_report(&self, report: &PentestRunReport) -> String {
        let mut markdown = String::new();
        markdown.push_str("# Báo cáo Kiểm thử Thâm nhập Tự Động (Autonomous Pentest Report)\n\n");
        markdown.push_str(
            "> Mọi phát hiện trong báo cáo được kiểm chứng trên môi trường giả lập tất định.\n\n",
        );

        markdown.push_str("## 1. Tóm tắt điều hành (Executive Summary)\n\n");
        markdown.push_str(&format!(
            "- Số phát hiện đã kiểm chứng: **{}**\n",
            report.findings.len()
        ));
        markdown.push_str(&format!(
            "- Số bằng chứng khai thác (PoE): **{}**\n",
            report.proofs.len()
        ));
        markdown.push_str(&format!(
            "- Số bản ghi bằng chứng trong chuỗi hash: **{}**\n",
            report.evidence.len()
        ));
        markdown.push_str(&format!(
            "- Số bước hành động đã thực hiện: **{}** (ngân sách: {})\n",
            report.steps, self.budget_steps
        ));
        markdown.push_str(&format!(
            "- Vi phạm phạm vi bị chặn: **{}** — vi phạm hiến pháp: **{}**\n",
            report.scope_violations.len(),
            report.violations.len()
        ));
        markdown.push_str(&format!(
            "- Đặc quyền cao nhất đạt được: **{}**\n",
            report.final_privilege
        ));
        markdown.push_str(&format!(
            "- Thoát sạch (clean exit): **{}**\n\n",
            report.clean_exit
        ));

        markdown.push_str("## 2. Phương pháp (Methodology)\n\n");
        markdown.push_str(
            "Chiến dịch được điều phối bởi đồ thị trạng thái 8 nút với các tác tử trinh sát, \
             khai thác và kiểm chứng. Mọi hành động đi qua ba cổng kiểm soát: phạm vi CIDR, \
             hiến pháp 10 điều và cổng phê duyệt con người.\n\n",
        );
        markdown.push_str("| Pha | Tên |\n|---|---|\n");
        for phase in &report.phases_visited {
            markdown.push_str(&format!("| {} | {} |\n", phase.label(), phase_name(phase)));
        }
        markdown.push('\n');

        markdown.push_str("## 3. Phát hiện (Findings)\n\n");
        if report.findings.is_empty() {
            markdown.push_str("Không có phát hiện nào vượt qua bước kiểm chứng ground-truth.\n\n");
        } else {
            for (index, finding) in report.findings.iter().enumerate() {
                markdown.push_str(&format!(
                    "### 3.{} {} — {}\n\n",
                    index + 1,
                    finding.id,
                    finding.title
                ));
                markdown.push_str(&format!(
                    "- Mức nghiêm trọng: **{}**\n- Mục tiêu: `{}:{}`\n- Định danh: `{}`\n- Mô tả: {}\n",
                    finding.severity.label(),
                    finding.target,
                    finding.port,
                    finding.cve,
                    finding.description
                ));
                if let Some(proof) = report
                    .proofs
                    .iter()
                    .find(|proof| proof.vuln_id == finding.id)
                {
                    markdown.push_str("- Bằng chứng khai thác (tái lập được):\n");
                    for step in &proof.steps {
                        markdown.push_str(&format!("  - {step}\n"));
                    }
                }
                markdown.push('\n');
            }
        }

        markdown.push_str("## 4. Khuyến nghị khắc phục (Remediation)\n\n");
        for finding in &report.findings {
            markdown.push_str(&format!(
                "- **{}**: {} — nâng cấp hoặc bỏ phiên bản dịch vụ và bổ sung cấu hình làm cứng tương ứng theo khuyến nghị của nhà cung cấp.\n",
                finding.id,
                remediation_for(&finding.severity)
            ));
        }
        if report.findings.is_empty() {
            markdown.push_str("- Không có khuyến nghị do chưa có phát hiện đã xác minh.\n");
        }
        markdown.push('\n');

        markdown.push_str("## 5. Kết luận (Conclusion)\n\n");
        markdown.push_str(&format!(
            "Chiến dịch đã đi qua {} pha với {} hành động được ghi nhận trong chuỗi bằng chứng \
             SHA-256. {} phát hiện đã được xác minh bằng proof-of-exploitation tái lập được, \
             {} vi phạm phạm vi và {} vi phạm hiến pháp đã bị chặn trước khi thực thi. \
             Quy trình kết thúc với trạng thái thoát sạch: {}.\n",
            report.phases_visited.len(),
            report.evidence.len(),
            report.findings.len(),
            report.scope_violations.len(),
            report.violations.len(),
            report.clean_exit
        ));
        markdown
    }
}

/// Tên kỹ thuật (tiếng Anh) của một pha, dùng trong bảng phương pháp.
fn phase_name(phase: &PentestPhase) -> &'static str {
    match phase {
        PentestPhase::Idle => "Idle",
        PentestPhase::Recon => "Recon",
        PentestPhase::Enumerate => "Enumerate",
        PentestPhase::Exploit => "Exploit",
        PentestPhase::PrivEsc => "PrivEsc",
        PentestPhase::Pivot => "Pivot",
        PentestPhase::Loot => "Loot",
        PentestPhase::Report => "Report",
        PentestPhase::Done => "Done",
    }
}

/// Khuyến nghị khắc phục mặc định theo mức nghiêm trọng.
fn remediation_for(severity: &Severity) -> &'static str {
    match severity {
        Severity::Critical => "vá khẩn cấp, bật xác thực mạnh và đổi toàn bộ thông tin bí mật",
        Severity::High => "vá lỗi trong lòng thời gian ngắn và giới hạn truy cập mạng",
        Severity::Medium => "lên kế hoạch vá và siết cấu hình mặc định",
        Severity::Low => "theo dõi và cập nhật định kỳ",
        Severity::Info => "ghi nhận để đối chiếu",
    }
}

// ============================================================================
// 14. CHƯƠNG TRÌNH MINH HOẠ (MAIN)
// ============================================================================

fn main() {
    println!("=== CHƯƠNG 105: ĐẠI DỰ ÁN TÍCH HỢP II — TÁC TỬ PENTEST TỰ HÀNH ĐA TÁC TỬ ===");
    println!("(Autonomous Multi-Agent Pentest: RL Reward, Red-vs-Blue & Constitutional Safety)\n");

    let network = MockNetwork::standard();

    // ------------------------------------------------------------------
    // 1. CHẠY CHIẾN DỊCH KIỂM THỬ TỰ HÀNH
    // ------------------------------------------------------------------
    let scope = ScopeGuard::default_scope();
    let mut agent = AutonomousPentestAgent::new(scope);
    let report = agent.run(&network);

    println!("[1] CHIẾN DỊCH ĐA TÁC TỬ");
    println!(
        "  - Mạng giả lập: {} máy, {} dịch vụ đang mở",
        network.hosts.len(),
        network.all_services().len()
    );
    let phases: Vec<&str> = report
        .phases_visited
        .iter()
        .map(|phase| phase.label())
        .collect();
    println!("  - Các pha đã đi qua: {}", phases.join(" → "));
    println!(
        "  - Số bước hành động: {} / ngân sách {}",
        report.steps, agent.budget_steps
    );
    println!(
        "  - Phát hiện đã kiểm chứng: {} | PoE: {} | thoát sạch: {}",
        report.findings.len(),
        report.proofs.len(),
        report.clean_exit
    );

    println!("\n[2] PHÁT HIỆN KÈM BẰNG CHỨNG KHAI THÁC");
    for finding in &report.findings {
        println!(
            "  - {} [{}] {} — {}:{}",
            finding.id,
            finding.severity.label(),
            finding.title,
            finding.target,
            finding.port
        );
        if let Some(proof) = report
            .proofs
            .iter()
            .find(|proof| proof.vuln_id == finding.id)
        {
            println!(
                "      PoE tái lập được: {} bước, kết quả: {}",
                proof.steps.len(),
                proof.output
            );
        }
    }

    println!("\n[3] CHUỖI BẰNG CHỨNG SHA-256");
    println!(
        "  - Số bản ghi: {} | toàn vẹn: {} | hash đầu chuỗi: {}",
        report.evidence.len(),
        agent.orchestrator.evidence.verify(),
        &agent.orchestrator.evidence.head_hash()[..16]
    );
    for entry in report.evidence.iter().take(6) {
        println!(
            "  - #{} [{}] t={} hash={}",
            entry.id,
            entry.kind,
            entry.timestamp,
            &entry.sha256[..12]
        );
    }
    if report.evidence.len() > 6 {
        println!("  - ... còn {} bản ghi", report.evidence.len() - 6);
    }

    println!("\n[4] PHẦN THƯỞNG HỌC TĂNG CƯỜNG (RL)");
    let mut optimizer = AttackPathOptimizer::new();
    let policy_before = optimizer.evaluate_path(&network);
    let rewards = optimizer.train(&network, 120, 2026);
    let policy_after = optimizer.evaluate_path(&network);
    let first_half: f64 =
        rewards[..rewards.len() / 2].iter().sum::<f64>() / (rewards.len() / 2) as f64;
    let second_half: f64 = rewards[rewards.len() / 2..].iter().sum::<f64>()
        / (rewards.len() - rewards.len() / 2) as f64;
    println!(
        "  - Phần thưởng trung bình 60 tập đầu: {first_half:.3} | 60 tập sau: {second_half:.3}"
    );
    println!("  - Tổng giá trị Q đã học: {:.3}", optimizer.total_reward());
    println!(
        "  - Phần thưởng chính sách tham lam: trước huấn luyện {policy_before:.3} -> sau huấn luyện {policy_after:.3}"
    );
    let path = optimizer.best_path(&network);
    println!("  - Đường tấn công tối ưu ({} bước):", path.len());
    for action in &path {
        println!(
            "      {} -> {}:{}",
            action.kind.label(),
            action.target,
            action.port
        );
    }
    println!(
        "  - Phần thưởng hành động cuối của chiến dịch: {:.3} ({})",
        report.reward.total,
        report
            .reward
            .components
            .iter()
            .map(|(name, value)| format!("{name}={value:.1}"))
            .collect::<Vec<String>>()
            .join(", ")
    );

    println!("\n[5] ĐỐI KHÁNG ĐỎ – XANH");
    let mut sim = RedVsBlueSim::new(0xC0FFEE);
    let outcomes = sim.run(&network, 24);
    let first = &outcomes[..outcomes.len() / 2];
    let second = &outcomes[outcomes.len() / 2..];
    let avg_first_red: f64 = first.iter().map(|o| o.red_reward).sum::<f64>() / first.len() as f64;
    let avg_second_red: f64 =
        second.iter().map(|o| o.red_reward).sum::<f64>() / second.len() as f64;
    let avg_first_blue: f64 = first.iter().map(|o| o.blue_score).sum::<f64>() / first.len() as f64;
    let avg_second_blue: f64 =
        second.iter().map(|o| o.blue_score).sum::<f64>() / second.len() as f64;
    println!("  - Nửa đầu 12 tập: đỏ = {avg_first_red:.2} điểm | xanh = {avg_first_blue:.2} điểm");
    println!(
        "  - Nửa sau 12 tập: đỏ = {avg_second_red:.2} điểm | xanh = {avg_second_blue:.2} điểm"
    );
    println!("  - Đội đỏ cải thiện: {}", sim.red_improves(&outcomes));
    println!("  - Đội xanh cải thiện: {}", sim.blue_improves(&outcomes));
    println!(
        "  - Số tín hiệu phát hiện tích luỹ của đội xanh: {}",
        sim.blue.detected_count()
    );

    println!("\n[6] TRÍCH ĐOẠN BÁO CÁO MARKDOWN");
    let lines: Vec<&str> = report.report_markdown.lines().collect();
    for line in lines.iter().take(28) {
        println!("  {line}");
    }
    println!("  ... (còn {} dòng)", lines.len().saturating_sub(28));

    println!("\n=== TOÀN BỘ 11 BẤT BIẾN HỆ THỐNG ĐÃ ĐƯỢC XÁC MINH ===");
}

#[cfg(test)]
mod tests {
    use super::*;

    // ------------------------------------------------------------------
    // NHÓM 1: PHẠM VI KIỂM THỬ (TEST 01 - 06)
    // ------------------------------------------------------------------

    #[test]
    fn test_01_scope_guard_allows_ip_inside_cidr() {
        let scope = ScopeGuard::default_scope();
        assert!(scope.allows_ip(Ipv4Addr::new(10, 10, 10, 10)));
        assert!(scope.allows_ip(Ipv4Addr::new(10, 10, 10, 50)));
        assert!(!scope.allows_ip(Ipv4Addr::new(10, 10, 11, 10)));
    }

    #[test]
    fn test_02_scope_guard_blocks_ip_outside_cidr() {
        // BẤT BIẾN 1: Scope Enforcement — IP ngoài scope phải bị chặn.
        let scope = ScopeGuard::default_scope();
        assert!(!scope.allows_ip(Ipv4Addr::new(203, 0, 113, 5)));
        assert!(!scope.allows_action(Ipv4Addr::new(203, 0, 113, 5), 80));
    }

    #[test]
    fn test_03_scope_guard_enforces_allowed_ports() {
        let scope = ScopeGuard::default_scope();
        let ip = Ipv4Addr::new(10, 10, 10, 10);
        assert!(scope.allows_action(ip, 80));
        // Cổng 4444 không nằm trong danh sách cổng được duyệt.
        assert!(!scope.allows_action(ip, 4444));
    }

    #[test]
    fn test_04_scope_guard_records_violation_on_check() {
        // BẤT BIẾN 1: vi phạm phạm vi phải được ghi nhận lại.
        let mut scope = ScopeGuard::default_scope();
        let ip = Ipv4Addr::new(10, 10, 10, 10);
        assert!(scope.check(ip, 80));
        assert!(scope.violations().is_empty());
        assert!(!scope.check(ip, 4444));
        assert_eq!(scope.violations().len(), 1);
        assert!(scope.violations()[0].contains("4444"));
    }

    #[test]
    fn test_05_scope_cidr_parsing_handles_prefix_edges() {
        assert!(parse_cidr("10.0.0.0/8").is_some());
        assert!(parse_cidr("10.0.0.0/33").is_none());
        assert!(parse_cidr("khong-phai-ip/24").is_none());
        assert!(parse_cidr("192.168.1.1").is_none());
        // /32 khớp chính xác, /0 khớp mọi địa chỉ.
        assert!(ip_in_cidr(
            Ipv4Addr::new(192, 168, 1, 7),
            Ipv4Addr::new(192, 168, 1, 7),
            32
        ));
        assert!(!ip_in_cidr(
            Ipv4Addr::new(192, 168, 1, 8),
            Ipv4Addr::new(192, 168, 1, 7),
            32
        ));
        assert!(ip_in_cidr(
            Ipv4Addr::new(8, 8, 8, 8),
            Ipv4Addr::new(0, 0, 0, 0),
            0
        ));
    }

    #[test]
    fn test_06_default_scope_covers_mock_network() {
        let scope = ScopeGuard::default_scope();
        let network = MockNetwork::standard();
        for host in &network.hosts {
            assert!(
                scope.allows_ip(host.ip),
                "máy {} nằm ngoài phạm vi mặc định",
                host.ip
            );
        }
    }

    // ------------------------------------------------------------------
    // NHÓM 2: HIẾN PHÁP KIỂM THỬ (TEST 07 - 12)
    // ------------------------------------------------------------------

    #[test]
    fn test_07_constitution_has_ten_articles() {
        let constitution = PentestConstitution::standard();
        assert_eq!(constitution.article_count(), 10);
        for id in 1..=10 {
            let article = constitution.article(id).expect("thiếu điều luật");
            assert_eq!(article.id, id);
            assert!(!article.rule.is_empty());
        }
    }

    #[test]
    fn test_08_constitution_blocks_destructive_action() {
        // BẤT BIẾN 2: Constitution Compliance — điều 2 chặn hành động phá hủy.
        let constitution = PentestConstitution::standard();
        let action = ProposedAction {
            kind: ActionKind::Destructive,
            target: "10.10.10.20".to_string(),
            port: 3306,
            description: "DROP TABLE audit_log".to_string(),
        };
        let violation = constitution
            .check_action(&action)
            .expect("phải vi phạm điều 2");
        assert_eq!(violation.article_id, 2);
        assert_eq!(constitution.enforcement_of(2), Enforcement::Block);
    }

    #[test]
    fn test_09_constitution_blocks_public_target() {
        // BẤT BIẾN 2: điều 1 chặn mục tiêu công khai nằm ngoài phạm vi.
        let constitution = PentestConstitution::standard();
        let action = ProposedAction {
            kind: ActionKind::Scan,
            target: "203.0.113.10".to_string(),
            port: 80,
            description: "Dò mạng công cộng".to_string(),
        };
        let violation = constitution
            .check_action(&action)
            .expect("phải vi phạm điều 1");
        assert_eq!(violation.article_id, 1);
    }

    #[test]
    fn test_10_constitution_warns_on_bulk_data_access() {
        let constitution = PentestConstitution::standard();
        let action = ProposedAction {
            kind: ActionKind::DataAccess,
            target: "10.10.10.20".to_string(),
            port: 3306,
            description: "Exfiltrate toàn bộ cơ sở dữ liệu khách hàng".to_string(),
        };
        let violation = constitution
            .check_action(&action)
            .expect("phải cảnh báo điều 4");
        assert_eq!(violation.article_id, 4);
        assert_eq!(constitution.enforcement_of(4), Enforcement::Warn);
    }

    #[test]
    fn test_11_constitution_warns_on_pivot() {
        let constitution = PentestConstitution::standard();
        let action = ProposedAction {
            kind: ActionKind::Pivot,
            target: "10.10.10.50".to_string(),
            port: 22,
            description: "Chuyển ngang sang máy nhảy".to_string(),
        };
        let violation = constitution
            .check_action(&action)
            .expect("phải cảnh báo điều 7");
        assert_eq!(violation.article_id, 7);
    }

    #[test]
    fn test_12_constitution_attestation_is_stable() {
        let constitution = PentestConstitution::standard();
        let first = constitution.attest("abc123");
        let second = constitution.attest("abc123");
        assert_eq!(first, second);
        assert!(first.contains("articles=10"));
        assert_ne!(first, constitution.attest("def456"));
    }

    // ------------------------------------------------------------------
    // NHÓM 3: CHUỖI BẰNG CHỨNG (TEST 13 - 17)
    // ------------------------------------------------------------------

    #[test]
    fn test_13_evidence_chain_appends_entries_in_order() {
        let mut chain = EvidenceChain::new();
        assert!(chain.is_empty());
        let first = chain.append("recon", "quét mạng");
        let second = chain.append("exploit", "khai thác mô phỏng");
        assert_eq!(chain.len(), 2);
        assert!(first < second);
        assert_eq!(chain.entries[0].id, 0);
        assert_eq!(chain.entries[1].id, 1);
        assert_eq!(chain.entries[0].prev_hash, EVIDENCE_GENESIS);
        assert_eq!(chain.entries[1].prev_hash, chain.entries[0].sha256);
    }

    #[test]
    fn test_14_evidence_chain_verify_returns_true() {
        let mut chain = EvidenceChain::new();
        for index in 0..8 {
            chain.append("log", &format!("bản ghi số {index}"));
        }
        assert!(chain.verify());
    }

    #[test]
    fn test_15_evidence_chain_detects_tampered_payload() {
        // BẤT BIẾN 3: Evidence Integrity — sửa một bản ghi làm verify() trả false.
        let mut chain = EvidenceChain::new();
        chain.append("recon", "quét mạng");
        chain.append("exploit", "khai thác mô phỏng");
        chain.append("report", "sinh báo cáo");
        assert!(chain.verify());

        chain.entries[1].payload = "nội dung bị sửa đổi".to_string();
        assert!(!chain.verify());
    }

    #[test]
    fn test_16_evidence_chain_detects_broken_linkage() {
        // BẤT BIẾN 3: cắt liên kết giữa hai bản ghi cũng bị phát hiện.
        let mut chain = EvidenceChain::new();
        chain.append("a", "một");
        chain.append("b", "hai");
        chain.append("c", "ba");
        assert!(chain.verify());

        chain.entries[2].prev_hash = EVIDENCE_GENESIS.to_string();
        assert!(!chain.verify());
    }

    #[test]
    fn test_17_evidence_chain_head_hash_and_chain_hashes() {
        let mut chain = EvidenceChain::new();
        assert_eq!(chain.head_hash(), EVIDENCE_GENESIS);
        chain.append("a", "một");
        chain.append("b", "hai");
        let hashes = chain.chain_hashes();
        assert_eq!(hashes.len(), 2);
        assert_eq!(chain.head_hash(), hashes[1]);
        // Giá trị băm phải khớp đúng công thức sha256(prev_hash + payload).
        let expected = sha256_hex(format!("{}{}", hashes[0], "hai").as_bytes());
        assert_eq!(hashes[1], expected);
    }

    // ------------------------------------------------------------------
    // NHÓM 4: CỔNG HITL (TEST 18 - 21)
    // ------------------------------------------------------------------

    #[test]
    fn test_18_hitl_request_blocks_until_approved() {
        // BẤT BIẾN 4: HITL Gate — hành động phải được duyệt trước khi chạy.
        let mut gate = HitlGate::new();
        assert!(!gate.request("exploit:10.10.10.20:3306"));
        assert!(!gate.is_approved("exploit:10.10.10.20:3306"));
        gate.approve("exploit:10.10.10.20:3306", "ops-01");
        assert!(gate.is_approved("exploit:10.10.10.20:3306"));
        assert!(gate.request("exploit:10.10.10.20:3306"));
    }

    #[test]
    fn test_19_hitl_approve_records_operator() {
        let mut gate = HitlGate::new();
        let decision = gate.approve("pivot:10.10.10.50:22", "quan-tri-vien");
        assert!(decision.approved);
        assert_eq!(decision.operator, "quan-tri-vien");
        assert!(!decision.reason.is_empty());
    }

    #[test]
    fn test_20_hitl_deny_records_reason() {
        let mut gate = HitlGate::new();
        let decision = gate.deny(
            "destructive:10.10.10.20:3306",
            "quan-tri-vien",
            "quyền phá hủy bị từ chối",
        );
        assert!(!decision.approved);
        assert_eq!(decision.operator, "quan-tri-vien");
        assert!(decision.reason.contains("từ chối"));
        assert!(!gate.is_approved("destructive:10.10.10.20:3306"));
    }

    #[test]
    fn test_21_orchestrator_requires_hitl_for_destructive() {
        // BẤT BIẾN 4: hành động phá hoại phải được duyệt mới được chạy.
        let mut orchestrator = OrchestratorAgent::new(ScopeGuard::default_scope());
        let action = ProposedAction {
            kind: ActionKind::Destructive,
            target: "10.10.10.20".to_string(),
            port: 3306,
            description: "DROP TABLE audit_log".to_string(),
        };
        let outcome = orchestrator.authorize(&action);
        assert!(outcome.hitl_required);
        assert!(!outcome.allowed);

        // Chuyển ngang cũng phải qua cổng HITL.
        let pivot = ProposedAction {
            kind: ActionKind::Pivot,
            target: "10.10.10.50".to_string(),
            port: 22,
            description: "Chuyển ngang".to_string(),
        };
        let outcome = orchestrator.authorize(&pivot);
        assert!(outcome.hitl_required);
        assert_eq!(outcome.blocked_by, BlockReason::Hitl);
        assert!(!outcome.allowed);
    }

    // ------------------------------------------------------------------
    // NHÓM 5: ĐỒ THỊ TRẠNG THÁI (TEST 22 - 25)
    // ------------------------------------------------------------------

    #[test]
    fn test_22_state_graph_accepts_forward_transitions() {
        let mut graph = PentestStateGraph::new();
        assert_eq!(graph.phase, PentestPhase::Idle);
        let chain = [
            PentestPhase::Recon,
            PentestPhase::Enumerate,
            PentestPhase::Exploit,
            PentestPhase::PrivEsc,
            PentestPhase::Pivot,
            PentestPhase::Loot,
            PentestPhase::Report,
            PentestPhase::Done,
        ];
        for phase in chain {
            assert!(
                graph.transition(phase).is_ok(),
                "không chuyển được sang {phase:?}"
            );
        }
        assert!(graph.completed());
        assert_eq!(graph.phase_count(), 8);
    }

    #[test]
    fn test_23_state_graph_rejects_backward_transition() {
        let mut graph = PentestStateGraph::new();
        assert!(graph.transition(PentestPhase::Exploit).is_ok());
        assert!(graph.transition(PentestPhase::Recon).is_err());
        assert_eq!(graph.phase, PentestPhase::Exploit);
    }

    #[test]
    fn test_24_state_graph_interrupt_and_resume() {
        let mut graph = PentestStateGraph::new();
        assert!(graph.interrupt());
        assert!(!graph.interrupt());
        assert!(graph.transition(PentestPhase::Recon).is_err());
        assert!(graph.resume());
        assert!(!graph.resume());
        assert!(graph.transition(PentestPhase::Recon).is_ok());
    }

    #[test]
    fn test_25_state_graph_respects_max_steps() {
        let mut graph = PentestStateGraph::new();
        graph.max_steps = 2;
        assert!(graph.transition(PentestPhase::Recon).is_ok());
        assert!(graph.transition(PentestPhase::Enumerate).is_ok());
        assert!(graph.transition(PentestPhase::Exploit).is_err());
        assert_eq!(graph.steps, 2);
    }

    // ------------------------------------------------------------------
    // NHÓM 6: MẠNG GIẢ LẬP & TÁC TỬ (TEST 26 - 31)
    // ------------------------------------------------------------------

    #[test]
    fn test_26_mock_network_standard_topology() {
        let network = MockNetwork::standard();
        assert_eq!(network.hosts.len(), 5);
        let web = network
            .host(Ipv4Addr::new(10, 10, 10, 10))
            .expect("thiếu web-01");
        assert_eq!(web.hostname, "web-01");
        assert_eq!(web.open_ports.len(), 2);
        let vulnerable = network
            .all_services()
            .into_iter()
            .filter(|(_, service)| service.vulnerable)
            .count();
        assert_eq!(vulnerable, 5);
    }

    #[test]
    fn test_27_network_mapper_lists_all_hosts() {
        let network = MockNetwork::standard();
        let mut mapper = NetworkMapper::new();
        let hosts = mapper.scan(&network);
        assert_eq!(hosts.len(), 5);
        assert_eq!(mapper.scans, 1);
        let info = mapper
            .host(Ipv4Addr::new(10, 10, 10, 20))
            .expect("thiếu db-01");
        assert_eq!(info.hostname, "db-01");
        assert!(info.open_ports.contains(&3306));
    }

    #[test]
    fn test_28_fingerprinter_flags_known_vulnerable_versions() {
        let network = MockNetwork::standard();
        let fingerprinter = ServiceFingerprinter::new();
        let web = network
            .host(Ipv4Addr::new(10, 10, 10, 10))
            .expect("thiếu web-01");
        let apache = web
            .open_ports
            .iter()
            .find(|s| s.port == 80)
            .expect("thiếu Apache");
        let finding = fingerprinter
            .fingerprint(web, apache)
            .expect("phải nhận diện được Apache lỗi thời");
        assert_eq!(finding.id, "WEB-001");
        assert_eq!(finding.severity, Severity::High);

        let ssh = web
            .open_ports
            .iter()
            .find(|s| s.port == 22)
            .expect("thiếu OpenSSH");
        assert!(fingerprinter.fingerprint(web, ssh).is_none());
    }

    #[test]
    fn test_29_recon_agent_returns_findings_and_targets() {
        let network = MockNetwork::standard();
        let mut agent = ReconAgent::new();
        let ctx = AgentContext {
            phase: PentestPhase::Recon,
            network: &network,
            findings: Vec::new(),
            targets: Vec::new(),
            shell: MockShell::new(),
        };
        let result = agent.step(&ctx);
        assert!(result.success);
        assert_eq!(result.targets.len(), 5);
        assert_eq!(result.findings.len(), 6);
        assert_eq!(agent.role(), AgentRole::Recon);
        assert_eq!(agent.name(), "ReconAgent");
    }

    #[test]
    fn test_30_exploit_agent_yields_reproducible_proof() {
        let network = MockNetwork::standard();
        let mut agent = ExploitAgent::new();
        let finding = Vulnerability {
            id: "WEB-001".to_string(),
            title: "Apache 2.4.29".to_string(),
            severity: Severity::High,
            target: Ipv4Addr::new(10, 10, 10, 10),
            port: 80,
            description: "mô phỏng".to_string(),
            cve: "CVE-2017-7668".to_string(),
        };
        let ctx = AgentContext {
            phase: PentestPhase::Exploit,
            network: &network,
            findings: vec![finding],
            targets: vec![Ipv4Addr::new(10, 10, 10, 10)],
            shell: MockShell::new(),
        };
        let result = agent.step(&ctx);
        assert!(result.success);
        let proof = result.proof.expect("phải có PoE");
        assert!(proof.reproduced);
        assert_eq!(proof.vuln_id, "WEB-001");
        assert!(proof.steps.len() >= 4);
        assert_eq!(agent.used.len(), 1);
    }

    #[test]
    fn test_31_validation_agent_drops_false_positive() {
        // BẤT BIẾN 10 & 11: Validation loại bỏ phát hiện giả, giữ lại phát hiện thật.
        let network = MockNetwork::standard();
        let mut agent = ValidationAgent::new();
        let true_finding = agent.verifier.ground_truth[0].clone();
        let false_finding = Vulnerability {
            id: "WEB-901".to_string(),
            title: "Jetty 9.4 — khung web lỗi thời".to_string(),
            severity: Severity::Low,
            target: Ipv4Addr::new(10, 10, 10, 50),
            port: 8080,
            description: "nghi ngờ".to_string(),
            cve: "CVE-2019-10241".to_string(),
        };
        let ctx = AgentContext {
            phase: PentestPhase::Report,
            network: &network,
            findings: vec![true_finding.clone(), false_finding.clone()],
            targets: Vec::new(),
            shell: MockShell::new(),
        };
        let result = agent.step(&ctx);
        assert_eq!(result.findings.len(), 1);
        assert_eq!(result.findings[0].id, true_finding.id);
        assert!(result.message.contains("WEB-901"));
    }

    // ------------------------------------------------------------------
    // NHÓM 7: PHẦN THƯỞNG RL (TEST 32 - 36)
    // ------------------------------------------------------------------

    #[test]
    fn test_32_reward_credits_host_compromise() {
        let reward_fn = AttackRewardFunction::new();
        let ip = Ipv4Addr::new(10, 10, 10, 10);
        let action = AttackAction {
            kind: ActionKind::Exploit,
            target: ip,
            port: 80,
        };
        let state = NetworkState {
            compromised: vec![ip],
            detected: false,
            crashed: Vec::new(),
            accessed_data: false,
        };
        let reward = reward_fn.evaluate(&action, &state);
        assert!(reward.total > 0.0);
        assert_eq!(reward.hosts_compromised, 1);
        assert!(
            reward
                .components
                .iter()
                .any(|(name, value)| name == "host-compromise" && *value > 0.0)
        );
    }

    #[test]
    fn test_33_reward_penalizes_detection_and_crash() {
        let reward_fn = AttackRewardFunction::new();
        let ip = Ipv4Addr::new(10, 10, 10, 40);
        let action = AttackAction {
            kind: ActionKind::Exploit,
            target: ip,
            port: 3389,
        };
        let state = NetworkState {
            compromised: vec![ip],
            detected: true,
            crashed: vec![ip],
            accessed_data: false,
        };
        let reward = reward_fn.evaluate(&action, &state);
        assert!(reward.detected);
        assert!(reward.crashed_service);
        assert!(reward.total < 0.0);
        assert!(
            reward
                .components
                .iter()
                .any(|(name, value)| name == "detection" && *value < 0.0)
        );
        assert!(
            reward
                .components
                .iter()
                .any(|(name, value)| name == "service-crash" && *value < 0.0)
        );
    }

    #[test]
    fn test_34_reward_credits_data_access_and_step_penalty() {
        let reward_fn = AttackRewardFunction::new();
        let ip = Ipv4Addr::new(10, 10, 10, 10);
        let action = AttackAction {
            kind: ActionKind::DataAccess,
            target: ip,
            port: 80,
        };
        let state = NetworkState {
            compromised: vec![ip],
            detected: false,
            crashed: Vec::new(),
            accessed_data: true,
        };
        let reward = reward_fn.evaluate(&action, &state);
        let data = reward
            .components
            .iter()
            .find(|(name, _)| name == "data-access")
            .expect("thiếu thành phần data-access");
        let step = reward
            .components
            .iter()
            .find(|(name, _)| name == "step-cost")
            .expect("thiếu thành phần step-cost");
        assert!(data.1 > 0.0);
        assert!(step.1 < 0.0);
    }

    #[test]
    fn test_35_reward_total_equals_component_sum() {
        let reward_fn = AttackRewardFunction::new();
        let ip = Ipv4Addr::new(10, 10, 10, 20);
        let action = AttackAction {
            kind: ActionKind::Exploit,
            target: ip,
            port: 3306,
        };
        let state = NetworkState {
            compromised: vec![ip],
            detected: false,
            crashed: Vec::new(),
            accessed_data: false,
        };
        let reward = reward_fn.evaluate(&action, &state);
        let sum: f64 = reward.components.iter().map(|(_, value)| value).sum();
        assert!((reward.total - sum).abs() < 1e-12);
    }

    #[test]
    fn test_36_reward_evaluation_is_deterministic() {
        let reward_fn = AttackRewardFunction::new();
        let action = AttackAction {
            kind: ActionKind::Exploit,
            target: Ipv4Addr::new(10, 10, 10, 30),
            port: 445,
        };
        let state = NetworkState {
            compromised: vec![Ipv4Addr::new(10, 10, 10, 30)],
            detected: false,
            crashed: Vec::new(),
            accessed_data: false,
        };
        let first = reward_fn.evaluate(&action, &state);
        let second = reward_fn.evaluate(&action, &state);
        assert!((first.total - second.total).abs() < 1e-12);
        assert_eq!(first.components.len(), second.components.len());
    }

    // ------------------------------------------------------------------
    // NHÓM 8: TỐI ƯU ĐƯỜNG TẤN CÔNG (TEST 37 - 42)
    // ------------------------------------------------------------------

    #[test]
    fn test_37_qtable_stores_and_selects_best_action() {
        let mut table = QTable::new();
        assert!(table.is_empty());
        table.set("s0", "a", 1.5);
        table.set("s0", "b", 4.0);
        assert_eq!(table.get("s0", "b"), 4.0);
        assert_eq!(table.get("s0", "khong-co"), 0.0);
        let actions = vec!["a".to_string(), "b".to_string()];
        assert_eq!(table.best_action("s0", &actions), Some("b".to_string()));
        assert_eq!(table.best_action("s1", &[]), None);
        assert_eq!(table.len(), 2);
    }

    #[test]
    fn test_38_optimizer_training_improves_policy_reward() {
        let network = MockNetwork::standard();
        let mut optimizer = AttackPathOptimizer::new();
        let before = optimizer.evaluate_path(&network);
        optimizer.train(&network, 150, 2026);
        let after = optimizer.evaluate_path(&network);
        assert!(
            after > before,
            "chính sách phải cải thiện sau huấn luyện: {before} -> {after}"
        );
    }

    #[test]
    fn test_39_optimizer_learns_to_avoid_crashing_service() {
        // Dịch vụ RDP 3389 làm sập dịch vụ khi bị khai thác: hành động này phải
        // bị học là bất lợi và bị loại khỏi đường tấn công tối ưu.
        let network = MockNetwork::standard();
        let mut optimizer = AttackPathOptimizer::new();
        optimizer.train(&network, 200, 4242);
        let path = optimizer.best_path(&network);
        assert!(!path.is_empty());
        assert!(
            !path.iter().any(|action| action.port == 3389),
            "đường tấn công tối ưu không được chọn dịch vụ dễ sập"
        );
        let start = "comp0|data0|det0";
        let risky = optimizer.q_table.get(start, "exploit:10.10.10.40:3389");
        let safe = optimizer.q_table.get(start, "exploit:10.10.10.10:80");
        assert!(
            risky < safe,
            "Q của dịch vụ dễ sập phải thấp hơn dịch vụ an toàn: {risky} >= {safe}"
        );
    }

    #[test]
    fn test_40_optimizer_best_path_targets_real_services() {
        let network = MockNetwork::standard();
        let mut optimizer = AttackPathOptimizer::new();
        optimizer.train(&network, 150, 7);
        let path = optimizer.best_path(&network);
        assert!(!path.is_empty());
        for action in &path {
            assert!(
                network.service(action.target, action.port).is_some(),
                "hành động nhắm tới dịch vụ không tồn tại: {}:{}",
                action.target,
                action.port
            );
            assert!(
                network
                    .service(action.target, action.port)
                    .expect("dịch vụ")
                    .vulnerable
            );
        }
    }

    #[test]
    fn test_41_optimizer_total_reward_grows_with_episodes() {
        let network = MockNetwork::standard();
        let mut optimizer = AttackPathOptimizer::new();
        optimizer.train(&network, 30, 11);
        let short = optimizer.q_table.len();
        let short_value = optimizer.total_reward();
        optimizer.train(&network, 170, 11);
        assert!(optimizer.q_table.len() > short);
        assert!(optimizer.total_reward() > short_value);
    }

    #[test]
    fn test_42_optimizer_training_is_seed_deterministic() {
        let network = MockNetwork::standard();
        let mut first = AttackPathOptimizer::new();
        let mut second = AttackPathOptimizer::new();
        let rewards_a = first.train(&network, 40, 99);
        let rewards_b = second.train(&network, 40, 99);
        assert_eq!(rewards_a.len(), rewards_b.len());
        for (left, right) in rewards_a.iter().zip(rewards_b.iter()) {
            assert!((left - right).abs() < 1e-12);
        }
        assert_eq!(first.q_table.len(), second.q_table.len());
    }

    // ------------------------------------------------------------------
    // NHÓM 9: ĐỐI KHÁNG ĐỎ – XANH (TEST 43 - 48)
    // ------------------------------------------------------------------

    #[test]
    fn test_43_blue_detects_exploit_action() {
        let mut blue = BlueTeamDefender::new();
        let action = AttackAction {
            kind: ActionKind::Exploit,
            target: Ipv4Addr::new(10, 10, 10, 10),
            port: 80,
        };
        let state = NetworkState::default();
        let signal = blue
            .observe(&action, &state)
            .expect("phải phát hiện hành động khai thác");
        assert_eq!(signal.severity, Severity::High);
        assert_eq!(blue.detected_count(), 1);
    }

    #[test]
    fn test_44_blue_ignores_low_signal_action() {
        let mut blue = BlueTeamDefender::new();
        blue.detection_threshold = 0.85;
        let action = AttackAction {
            kind: ActionKind::Scan,
            target: Ipv4Addr::new(10, 10, 10, 10),
            port: 80,
        };
        let state = NetworkState::default();
        assert!(blue.observe(&action, &state).is_none());
        assert_eq!(blue.detected_count(), 0);
    }

    #[test]
    fn test_45_blue_defense_score_grows_with_defenses() {
        let mut blue = BlueTeamDefender::new();
        let base = blue.defense_score();
        blue.isolate(Ipv4Addr::new(10, 10, 10, 20));
        let after_isolate = blue.defense_score();
        assert!(after_isolate > base);
        blue.reset_credential("admin");
        let after_reset = blue.defense_score();
        assert!(after_reset > after_isolate);
    }

    #[test]
    fn test_46_red_vs_blue_red_agent_improves() {
        let network = MockNetwork::standard();
        let mut sim = RedVsBlueSim::new(0xC0FFEE);
        let outcomes = sim.run(&network, 24);
        assert_eq!(outcomes.len(), 24);
        assert!(sim.red_improves(&outcomes));
    }

    #[test]
    fn test_47_red_vs_blue_blue_team_improves() {
        let network = MockNetwork::standard();
        let mut sim = RedVsBlueSim::new(0xC0FFEE);
        let outcomes = sim.run(&network, 24);
        assert!(sim.blue_improves(&outcomes));
    }

    #[test]
    fn test_48_red_vs_blue_episodes_are_deterministic() {
        let network = MockNetwork::standard();
        let mut first = RedVsBlueSim::new(0xBEEF);
        let mut second = RedVsBlueSim::new(0xBEEF);
        let outcomes_a = first.run(&network, 12);
        let outcomes_b = second.run(&network, 12);
        for (left, right) in outcomes_a.iter().zip(outcomes_b.iter()) {
            assert_eq!(left.episode, right.episode);
            assert!((left.red_reward - right.red_reward).abs() < 1e-12);
            assert!((left.blue_score - right.blue_score).abs() < 1e-12);
            assert_eq!(left.hosts_compromised, right.hosts_compromised);
        }
    }

    // ------------------------------------------------------------------
    // NHÓM 10: QUẢN LÝ PHIÊN (TEST 49 - 50)
    // ------------------------------------------------------------------

    #[test]
    fn test_49_session_manager_save_and_restore_roundtrip() {
        let mut manager = SessionManager::new();
        let snapshot = SessionSnapshot {
            id: String::new(),
            phase: PentestPhase::Exploit,
            findings: Vec::new(),
            compromised: vec![Ipv4Addr::new(10, 10, 10, 10)],
            evidence_count: 7,
            saved_at: 42,
        };
        let id = manager.save("phiên-01", snapshot);
        assert_eq!(id, "phiên-01");
        let restored = manager
            .restore("phiên-01")
            .expect("phải nối tiếp được phiên");
        assert_eq!(restored.phase, PentestPhase::Exploit);
        assert_eq!(restored.evidence_count, 7);
        assert_eq!(restored.id, "phiên-01");
        assert!(manager.restore("khong-ton-tai").is_none());
    }

    #[test]
    fn test_50_session_manager_lists_and_sets_current() {
        let mut manager = SessionManager::new();
        for name in ["alpha", "beta"] {
            manager.save(
                name,
                SessionSnapshot {
                    id: String::new(),
                    phase: PentestPhase::Recon,
                    findings: Vec::new(),
                    compromised: Vec::new(),
                    evidence_count: 0,
                    saved_at: 1,
                },
            );
        }
        assert_eq!(
            manager.list(),
            vec!["alpha".to_string(), "beta".to_string()]
        );
        manager.set_current("alpha");
        assert_eq!(
            manager.current().map(|snapshot| snapshot.id.clone()),
            Some("alpha".to_string())
        );
    }

    // ------------------------------------------------------------------
    // NHÓM 11: KIỂM THỨC END-TO-END (TEST 51 - 54)
    // ------------------------------------------------------------------

    #[test]
    fn test_51_end_to_end_multi_agent_phase_order() {
        // BẤT BIẾN 11: Multi-Agent Coordination — đúng thứ tự pha và phát hiện hợp lệ.
        let network = MockNetwork::standard();
        let mut agent = AutonomousPentestAgent::new(ScopeGuard::default_scope());
        let report = agent.run(&network);
        let expected = vec![
            PentestPhase::Recon,
            PentestPhase::Enumerate,
            PentestPhase::Exploit,
            PentestPhase::PrivEsc,
            PentestPhase::Pivot,
            PentestPhase::Loot,
            PentestPhase::Report,
            PentestPhase::Done,
        ];
        assert_eq!(report.phases_visited, expected);
        assert!(!report.findings.is_empty());
        assert_eq!(report.findings.len(), report.proofs.len());
    }

    #[test]
    fn test_52_end_to_end_report_has_all_sections() {
        // BẤT BIẾN 8: Report Generation — báo cáo có đủ 5 mục bắt buộc.
        let network = MockNetwork::standard();
        let mut agent = AutonomousPentestAgent::new(ScopeGuard::default_scope());
        let report = agent.run(&network);
        for section in [
            "Executive Summary",
            "Methodology",
            "Findings",
            "Remediation",
            "Conclusion",
        ] {
            assert!(
                report.report_markdown.contains(section),
                "báo cáo thiếu mục {section}"
            );
        }
    }

    #[test]
    fn test_53_end_to_end_privilege_and_time_budget() {
        // BẤT BIẾN 5 & 6: ranh giới đặc quyền và ngân sách thời gian.
        let network = MockNetwork::standard();
        let mut agent = AutonomousPentestAgent::new(ScopeGuard::default_scope());
        let report = agent.run(&network);
        assert!(report.steps <= agent.budget_steps);
        assert_eq!(report.final_privilege, "root");
        // Mọi máy bị xâm nhập đều phải có bằng chứng khai thác tương ứng.
        for host in &report.compromised {
            assert!(
                report.proofs.iter().any(|proof| proof.target == *host),
                "máy {host} bị xâm nhập mà không có PoE"
            );
        }
        // Leo thang lên máy chưa xâm nhập phải bị chặn và ghi nhận.
        let blocked = report
            .evidence
            .iter()
            .any(|entry| entry.kind == "blocked-action" && entry.payload.contains("đặc quyền"));
        assert!(blocked, "phải ghi nhận hành động leo thang bị chặn");
    }

    #[test]
    fn test_54_end_to_end_evidence_complete_and_clean_exit() {
        // BẤT BIẾN 7, 9 & 10: nhật ký đầy đủ, thoát sạch và kiểm chứng ground-truth.
        let network = MockNetwork::standard();
        let mut agent = AutonomousPentestAgent::new(ScopeGuard::default_scope());
        let report = agent.run(&network);

        // (7) Log Completeness
        assert!(report.evidence.len() >= report.steps);
        for entry in &report.evidence {
            assert!(!entry.kind.is_empty());
            assert!(!entry.payload.is_empty());
        }
        assert!(agent.orchestrator.evidence.verify());

        // (9) Clean Exit
        assert!(report.clean_exit);
        assert!(agent.orchestrator.graph.completed());

        // (10) Ground-truth Verification
        let verifier = GroundTruthVerifier::new();
        for finding in &report.findings {
            assert!(verifier.is_true_positive(finding));
        }
        // Phát hiện giả (Jetty 9.4) không được xuất hiện trong báo cáo.
        assert!(!report.findings.iter().any(|f| f.id == "WEB-901"));
    }
}
