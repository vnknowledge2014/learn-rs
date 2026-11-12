//! # Chương 101: Tấn công Thông tin xác thực & Kerberos — Bẻ khoá Băm, Giao thức Kerberos, RC4-HMAC/AES & Đột biến Từ điển (Credential Attacks: Hash Cracking, Kerberos Protocol, RC4-HMAC/AES & Wordlist Mutation)
//!
//! Crate `ch101` nằm trong **Chủ đề 28 — An ninh mạng & Bảo mật ứng dụng** của giáo trình
//! `learn-rs`, đi tiếp nền tảng mã hoá học đã dựng ở Chương 70 (SHA-256 tự cài từ FIPS 180-4).
//!
//! Toàn bộ crate viết bằng **Rust 2024 thuần `std`**, `[dependencies]` rỗng tuyệt đối:
//! không crate băm, không crate mã hoá, không thư viện ASN.1 — tất cả tự cài từ RFC/FIPS công khai
//! (RFC 1321 MD5, RFC 1320 MD4, RFC 2202 HMAC-MD5, RFC 4231 HMAC-SHA-256, RFC 4758 RC4,
//! FIPS 197 AES, RFC 3962 CTS, RFC 4120 Kerberos 5, MS-NLMP NTLM).
//!
//! Nội dung chính:
//! - **MD5** (RFC 1321) và **MD4** (RFC 1320) — nền tảng của NT Hash & LM Hash.
//! - **HMAC-MD5**, **HMAC-SHA-256**, **PBKDF2** — các lớp băm xác thực.
//! - **SHA-256** (FIPS 180-4) — tái sử dụng ý tưởng từ Chương 70.
//! - **RC4** (RFC 4758) — mã hoá dòng cho Kerberos etype 23.
//! - **AES-256 + CBC/CTS** (FIPS-197, RFC 3962) — etype 18 của Kerberos.
//! - **ASN.1 DER rút gọn** — mã hoá/giải mã AS-REQ, AS-REP, TGS-REQ, TGS-REP.
//! - **Kerberoast & AS-REP Roasting** — bẻ khoá dịch vụ và bẻ khoá người dùng.
//! - **Bộ bẻ khoá từ điển** + **đột biến từ điển** — chiến lược tấn công mật khẩu.
//!
//! **Lưu ý đạo đức**: toàn bộ dữ liệu đầu vào là dữ liệu **tổng hợp, tất định**, do chính crate
//! này sinh ra; không có kết nối mạng thật, không tấn công hệ thống ngoài localhost.
#![allow(dead_code)]

use std::collections::HashSet;
use std::fmt;

// ============================================================================
// 1. TIỆN ÍCH HEX, BĂM NHẸ FNV-1a & CHUYỂN ĐỔI BYTE
// ============================================================================

/// Bảng chữ số hex viết thường, dùng để chuyển byte → chuỗi hex.
const HEX_DIGITS: [u8; 16] = *b"0123456789abcdef";

/// Chuyển một mảng byte thành chuỗi hex viết thường.
pub fn to_hex(bytes: &[u8]) -> String {
    let mut out = String::with_capacity(bytes.len() * 2);
    for &b in bytes {
        out.push(HEX_DIGITS[(b >> 4) as usize] as char);
        out.push(HEX_DIGITS[(b & 0x0f) as usize] as char);
    }
    out
}

/// Chuyển một chuỗi hex về mảng byte; trả về `None` nếu chuỗi không hợp lệ.
pub fn from_hex(text: &str) -> Option<Vec<u8>> {
    let cleaned: Vec<char> = text.chars().filter(|c| !c.is_whitespace()).collect();
    if !cleaned.len().is_multiple_of(2) {
        return None;
    }
    let mut out = Vec::with_capacity(cleaned.len() / 2);
    for pair in cleaned.chunks(2) {
        let hi = pair[0].to_digit(16)?;
        let lo = pair[1].to_digit(16)?;
        out.push(((hi << 4) | lo) as u8);
    }
    Some(out)
}

/// Hàm băm FNV-1a 32-bit — dùng để sinh nonce tất định cho thông điệp Kerberos.
pub fn fnv1a32(data: &[u8]) -> u32 {
    let mut hash: u32 = 0x811c_9dc5;
    for &b in data {
        hash ^= b as u32;
        hash = hash.wrapping_mul(0x0100_0193);
    }
    hash
}

/// Chuyển một chuỗi Rust sang mảng byte theo mã hoá UTF-16LE (định dạng NTLM).
pub fn to_utf16le(text: &str) -> Vec<u8> {
    let mut out = Vec::with_capacity(text.chars().count() * 2);
    for unit in text.encode_utf16() {
        out.extend_from_slice(&unit.to_le_bytes());
    }
    out
}

// ============================================================================
// 2. MD5 — TỰ CÀI TỪ ĐẦU (RFC 1321)
// ============================================================================

/// Hằng số lịch xoay bit của MD5 (4 vòng × 4 lần lặp).
const MD5_S: [u32; 64] = [
    7, 12, 17, 22, 7, 12, 17, 22, 7, 12, 17, 22, 7, 12, 17, 22, 5, 9, 14, 20, 5, 9, 14, 20, 5, 9,
    14, 20, 5, 9, 14, 20, 4, 11, 16, 23, 4, 11, 16, 23, 4, 11, 16, 23, 4, 11, 16, 23, 6, 10, 15,
    21, 6, 10, 15, 21, 6, 10, 15, 21, 6, 10, 15, 21,
];

/// Hằng số tròn $\lfloor 2^{32} \cdot |\sin(i+1)| \rfloor$ của MD5.
const MD5_K: [u32; 64] = [
    0xd76a_a478,
    0xe8c7_b756,
    0x2420_70db,
    0xc1bd_ceee,
    0xf57c_0faf,
    0x4787_c62a,
    0xa830_4613,
    0xfd46_9501,
    0x6980_98d8,
    0x8b44_f7af,
    0xffff_5bb1,
    0x895c_d7be,
    0x6b90_1122,
    0xfd98_7193,
    0xa679_438e,
    0x49b4_0821,
    0xf61e_2562,
    0xc040_b340,
    0x265e_5a51,
    0xe9b6_c7aa,
    0xd62f_105d,
    0x0244_1453,
    0xd8a1_e681,
    0xe7d3_fbc8,
    0x21e1_cde6,
    0xc337_07d6,
    0xf4d5_0d87,
    0x455a_14ed,
    0xa9e3_e905,
    0xfcef_a3f8,
    0x676f_02d9,
    0x8d2a_4c8a,
    0xfffa_3942,
    0x8771_f681,
    0x6d9d_6122,
    0xfde5_380c,
    0xa4be_ea44,
    0x4bde_cfa9,
    0xf6bb_4b60,
    0xbebf_bc70,
    0x289b_7ec6,
    0xeaa1_27fa,
    0xd4ef_3085,
    0x0488_1d05,
    0xd9d4_d039,
    0xe6db_99e5,
    0x1fa2_7cf8,
    0xc4ac_5665,
    0xf429_2244,
    0x432a_ff97,
    0xab94_23a7,
    0xfc93_a039,
    0x655b_59c3,
    0x8f0c_cc92,
    0xffef_f47d,
    0x8584_5dd1,
    0x6fa8_7e4f,
    0xfe2c_e6e0,
    0xa301_4314,
    0x4e08_11a1,
    0xf753_7e82,
    0xbd3a_f235,
    0x2ad7_d2bb,
    0xeb86_d391,
];

/// Nén một khối 64 byte vào trạng thái bốn từ 32-bit của MD5.
fn md5_compress(state: &mut [u32; 4], block: &[u8]) {
    let mut m = [0u32; 16];
    for (i, word) in m.iter_mut().enumerate() {
        *word = u32::from_le_bytes([
            block[i * 4],
            block[i * 4 + 1],
            block[i * 4 + 2],
            block[i * 4 + 3],
        ]);
    }
    let (mut a, mut b, mut c, mut d) = (state[0], state[1], state[2], state[3]);
    for i in 0..64 {
        let (f, g) = match i {
            0..=15 => ((b & c) | ((!b) & d), i),
            16..=31 => ((d & b) | ((!d) & c), (5 * i + 1) % 16),
            32..=47 => (b ^ c ^ d, (3 * i + 5) % 16),
            _ => (c ^ (b | (!d)), (7 * i) % 16),
        };
        let temp = d;
        d = c;
        c = b;
        let sum = a.wrapping_add(f).wrapping_add(MD5_K[i]).wrapping_add(m[g]);
        b = b.wrapping_add(sum.rotate_left(MD5_S[i]));
        a = temp;
    }
    state[0] = state[0].wrapping_add(a);
    state[1] = state[1].wrapping_add(b);
    state[2] = state[2].wrapping_add(c);
    state[3] = state[3].wrapping_add(d);
}

/// Bộ băm MD5 dòng — nhận dữ liệu theo từng khối rồi mới chốt kết quả.
///
/// Trạng thái gồm 4 từ 32-bit, bộ đệm 64 byte, độ dài bộ đệm và tổng số bit đã nạp.
#[derive(Debug, Clone)]
pub struct Md5Hasher {
    /// Bốn từ trạng thái (A, B, C, D).
    state: [u32; 4],
    /// Bộ đệm khối 64 byte.
    buffer: [u8; 64],
    /// Số byte đang nằm trong bộ đệm.
    buffer_len: usize,
    /// Tổng số bit dữ liệu đã nạp vào (theo đặc tả RFC 1321).
    length_bits: u64,
}

impl Default for Md5Hasher {
    fn default() -> Self {
        Self::new()
    }
}

impl Md5Hasher {
    /// Khởi tạo bộ băm MD5 với trạng thái chuẩn RFC 1321.
    pub fn new() -> Self {
        Self {
            state: [0x6745_2301, 0xefcd_ab89, 0x98ba_dcfe, 0x1032_5476],
            buffer: [0u8; 64],
            buffer_len: 0,
            length_bits: 0,
        }
    }

    /// Nạp thêm một mảng byte vào bộ băm.
    pub fn update(&mut self, data: &[u8]) {
        self.length_bits = self
            .length_bits
            .wrapping_add((data.len() as u64).wrapping_mul(8));
        let mut offset = 0usize;
        if self.buffer_len > 0 {
            let take = (64 - self.buffer_len).min(data.len());
            self.buffer[self.buffer_len..self.buffer_len + take].copy_from_slice(&data[..take]);
            self.buffer_len += take;
            offset = take;
            if self.buffer_len == 64 {
                md5_compress(&mut self.state, &self.buffer);
                self.buffer_len = 0;
            }
        }
        while offset + 64 <= data.len() {
            md5_compress(&mut self.state, &data[offset..offset + 64]);
            offset += 64;
        }
        if offset < data.len() {
            let rest = data.len() - offset;
            self.buffer[..rest].copy_from_slice(&data[offset..]);
            self.buffer_len = rest;
        }
    }

    /// Chốt bản băm 16 byte cuối cùng.
    pub fn finalize(&mut self) -> [u8; 16] {
        let bit_len = self.length_bits;
        self.buffer[self.buffer_len] = 0x80;
        self.buffer_len += 1;
        if self.buffer_len > 56 {
            self.buffer[self.buffer_len..].fill(0);
            md5_compress(&mut self.state, &self.buffer);
            self.buffer_len = 0;
        }
        self.buffer[self.buffer_len..56].fill(0);
        self.buffer[56..64].copy_from_slice(&bit_len.to_le_bytes());
        md5_compress(&mut self.state, &self.buffer);
        let mut out = [0u8; 16];
        for (i, word) in self.state.iter().enumerate() {
            out[i * 4..i * 4 + 4].copy_from_slice(&word.to_le_bytes());
        }
        out
    }
}

/// Tính MD5 của một mảng byte (hàm một lần).
pub fn md5(data: &[u8]) -> [u8; 16] {
    let mut hasher = Md5Hasher::new();
    hasher.update(data);
    hasher.finalize()
}

/// Tính MD5 rồi trả về chuỗi hex viết thường.
pub fn md5_hex(data: &[u8]) -> String {
    to_hex(&md5(data))
}

// ============================================================================
// 3. MD4 — TỰ CÀI TỪ ĐẦU (RFC 1320), NỀN TẢNG CỦA NT HASH
// ============================================================================

/// Nén một khối 64 byte của MD4.
fn md4_compress(state: &mut [u32; 4], block: &[u8]) {
    let mut x = [0u32; 16];
    for (i, word) in x.iter_mut().enumerate() {
        *word = u32::from_le_bytes([
            block[i * 4],
            block[i * 4 + 1],
            block[i * 4 + 2],
            block[i * 4 + 3],
        ]);
    }
    let (mut a, mut b, mut c, mut d) = (state[0], state[1], state[2], state[3]);

    // Vòng 1: hàm F(x,y,z) = (x & y) | (~x & z), thứ tự X[0..16], lịch xoay 3, 7, 11, 19.
    //
    // Bốn lệnh FF trong một nhóm dùng cùng một hàm F nhưng hoán đổi vị trí đối số theo chu kỳ
    // (b,c,d) -> (a,b,c) -> (d,a,b) -> (c,d,a).  Mẹo "xoay vai trò" bên dưới tự động giữ đúng
    // chu kỳ này: mỗi bước luôn dồn vào `a` rồi đảo vị trí (a,d,c,b).
    const R1_IDX: [usize; 16] = [0, 1, 2, 3, 4, 5, 6, 7, 8, 9, 10, 11, 12, 13, 14, 15];
    const R1_SHIFT: [u32; 16] = [3, 7, 11, 19, 3, 7, 11, 19, 3, 7, 11, 19, 3, 7, 11, 19];
    for step in 0..16 {
        let f = (b & c) | ((!b) & d);
        let sum = a
            .wrapping_add(f)
            .wrapping_add(x[R1_IDX[step]])
            .rotate_left(R1_SHIFT[step]);
        a = d;
        d = c;
        c = b;
        b = sum;
    }

    // Vòng 2: hàm G(x,y,z) = (x & y) | (x & z) | (y & z), lịch xoay 3, 5, 9, 13.
    //
    // Khác MD5, MD4 vòng 2 cộng thêm hằng số 0x5A827999 (≈ √2) vào MỖI phép toán.
    const MD4_K2: u32 = 0x5a82_7999;
    const R2_IDX: [usize; 16] = [0, 4, 8, 12, 1, 5, 9, 13, 2, 6, 10, 14, 3, 7, 11, 15];
    const R2_SHIFT: [u32; 16] = [3, 5, 9, 13, 3, 5, 9, 13, 3, 5, 9, 13, 3, 5, 9, 13];
    for step in 0..16 {
        let g = (b & c) | (b & d) | (c & d);
        let sum = a
            .wrapping_add(g)
            .wrapping_add(x[R2_IDX[step]])
            .wrapping_add(MD4_K2)
            .rotate_left(R2_SHIFT[step]);
        a = d;
        d = c;
        c = b;
        b = sum;
    }

    // Vòng 3: hàm H(x,y,z) = x ^ y ^ z, lịch xoay 3, 9, 11, 15.
    //
    // Vòng 3 cộng thêm hằng số 0x6ED9EBA1 (≈ √3) vào mỗi phép toán.
    const MD4_K3: u32 = 0x6ed9_eba1;
    const R3_IDX: [usize; 16] = [0, 8, 4, 12, 2, 10, 6, 14, 1, 9, 5, 13, 3, 11, 7, 15];
    const R3_SHIFT: [u32; 16] = [3, 9, 11, 15, 3, 9, 11, 15, 3, 9, 11, 15, 3, 9, 11, 15];
    for step in 0..16 {
        let h = b ^ c ^ d;
        let sum = a
            .wrapping_add(h)
            .wrapping_add(x[R3_IDX[step]])
            .wrapping_add(MD4_K3)
            .rotate_left(R3_SHIFT[step]);
        a = d;
        d = c;
        c = b;
        b = sum;
    }

    state[0] = state[0].wrapping_add(a);
    state[1] = state[1].wrapping_add(b);
    state[2] = state[2].wrapping_add(c);
    state[3] = state[3].wrapping_add(d);
}

/// Tính MD4 của một mảng byte — vector kiểm thử theo RFC 1320.
pub fn md4(data: &[u8]) -> [u8; 16] {
    let mut state: [u32; 4] = [0x6745_2301, 0xefcd_ab89, 0x98ba_dcfe, 0x1032_5476];
    let mut message = data.to_vec();
    let bit_len = (data.len() as u64).wrapping_mul(8);
    message.push(0x80);
    while message.len() % 64 != 56 {
        message.push(0);
    }
    message.extend_from_slice(&bit_len.to_le_bytes());
    for block in message.chunks(64) {
        md4_compress(&mut state, block);
    }
    let mut out = [0u8; 16];
    for (i, word) in state.iter().enumerate() {
        out[i * 4..i * 4 + 4].copy_from_slice(&word.to_le_bytes());
    }
    out
}

/// Tính MD4 rồi trả về chuỗi hex viết thường.
pub fn md4_hex(data: &[u8]) -> String {
    to_hex(&md4(data))
}

// ============================================================================
// 4. HMAC-MD5 & HMAC-SHA-256 (RFC 2202, RFC 4231)
// ============================================================================

/// Tính HMAC-MD5 theo RFC 2202.
pub fn hmac_md5(key: &[u8], msg: &[u8]) -> [u8; 16] {
    const BLOCK: usize = 64;
    let mut padded = [0u8; BLOCK];
    if key.len() > BLOCK {
        padded[..16].copy_from_slice(&md5(key));
    } else {
        padded[..key.len()].copy_from_slice(key);
    }
    let mut ipad = [0x36u8; BLOCK];
    let mut opad = [0x5cu8; BLOCK];
    for i in 0..BLOCK {
        ipad[i] ^= padded[i];
        opad[i] ^= padded[i];
    }
    let mut inner = Md5Hasher::new();
    inner.update(&ipad);
    inner.update(msg);
    let inner_hash = inner.finalize();
    let mut outer = Md5Hasher::new();
    outer.update(&opad);
    outer.update(&inner_hash);
    outer.finalize()
}

/// Bộ băm SHA-256 dòng — dùng cho HMAC-SHA-256 và PBKDF2.
#[derive(Debug, Clone)]
pub struct Sha256Hasher {
    /// Tám từ trạng thái (H0..H7).
    state: [u32; 8],
    /// Bộ đệm khối 64 byte.
    buffer: [u8; 64],
    /// Số byte đang nằm trong bộ đệm.
    buffer_len: usize,
    /// Tổng số bit dữ liệu đã nạp vào.
    length_bits: u64,
}

/// Hằng số tròn của SHA-256 (FIPS 180-4).
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

impl Default for Sha256Hasher {
    fn default() -> Self {
        Self::new()
    }
}

impl Sha256Hasher {
    /// Khởi tạo bộ băm SHA-256 với tám từ trạng thái chuẩn FIPS 180-4.
    pub fn new() -> Self {
        Self {
            state: [
                0x6a09_e667,
                0xbb67_ae85,
                0x3c6e_f372,
                0xa54f_f53a,
                0x510e_527f,
                0x9b05_688c,
                0x1f83_d9ab,
                0x5be0_cd19,
            ],
            buffer: [0u8; 64],
            buffer_len: 0,
            length_bits: 0,
        }
    }

    /// Nạp thêm một mảng byte vào bộ băm.
    pub fn update(&mut self, data: &[u8]) {
        self.length_bits = self
            .length_bits
            .wrapping_add((data.len() as u64).wrapping_mul(8));
        let mut offset = 0usize;
        if self.buffer_len > 0 {
            let take = (64 - self.buffer_len).min(data.len());
            self.buffer[self.buffer_len..self.buffer_len + take].copy_from_slice(&data[..take]);
            self.buffer_len += take;
            offset = take;
            if self.buffer_len == 64 {
                sha256_compress(&mut self.state, &self.buffer);
                self.buffer_len = 0;
            }
        }
        while offset + 64 <= data.len() {
            sha256_compress(&mut self.state, &data[offset..offset + 64]);
            offset += 64;
        }
        if offset < data.len() {
            let rest = data.len() - offset;
            self.buffer[..rest].copy_from_slice(&data[offset..]);
            self.buffer_len = rest;
        }
    }

    /// Chốt bản băm 32 byte cuối cùng.
    pub fn finalize(&mut self) -> [u8; 32] {
        let bit_len = self.length_bits;
        self.buffer[self.buffer_len] = 0x80;
        self.buffer_len += 1;
        if self.buffer_len > 56 {
            self.buffer[self.buffer_len..].fill(0);
            sha256_compress(&mut self.state, &self.buffer);
            self.buffer_len = 0;
        }
        self.buffer[self.buffer_len..56].fill(0);
        self.buffer[56..64].copy_from_slice(&bit_len.to_be_bytes());
        sha256_compress(&mut self.state, &self.buffer);
        let mut out = [0u8; 32];
        for (i, word) in self.state.iter().enumerate() {
            out[i * 4..i * 4 + 4].copy_from_slice(&word.to_be_bytes());
        }
        out
    }
}

/// Nén một khối 64 byte của SHA-256.
fn sha256_compress(state: &mut [u32; 8], block: &[u8]) {
    let mut w = [0u32; 64];
    for (i, word) in w.iter_mut().take(16).enumerate() {
        *word = u32::from_be_bytes([
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
    let (mut a, mut b, mut c, mut d) = (state[0], state[1], state[2], state[3]);
    let (mut e, mut f, mut g, mut h) = (state[4], state[5], state[6], state[7]);
    for i in 0..64 {
        let s1 = e.rotate_right(6) ^ e.rotate_right(11) ^ e.rotate_right(25);
        let ch = (e & f) ^ ((!e) & g);
        let t1 = h
            .wrapping_add(s1)
            .wrapping_add(ch)
            .wrapping_add(SHA256_K[i])
            .wrapping_add(w[i]);
        let s0 = a.rotate_right(2) ^ a.rotate_right(13) ^ a.rotate_right(22);
        let maj = (a & b) ^ (a & c) ^ (b & c);
        let t2 = s0.wrapping_add(maj);
        h = g;
        g = f;
        f = e;
        e = d.wrapping_add(t1);
        d = c;
        c = b;
        b = a;
        a = t1.wrapping_add(t2);
    }
    for (i, value) in [a, b, c, d, e, f, g, h].iter().enumerate() {
        state[i] = state[i].wrapping_add(*value);
    }
}

/// Tính SHA-256 của một mảng byte — tái sử dụng ý tưởng từ Chương 70.
pub fn sha256(data: &[u8]) -> [u8; 32] {
    let mut hasher = Sha256Hasher::new();
    hasher.update(data);
    hasher.finalize()
}

/// Tính SHA-256 rồi trả về chuỗi hex viết thường.
pub fn sha256_hex(data: &[u8]) -> String {
    to_hex(&sha256(data))
}

/// Tính HMAC-SHA-256 theo RFC 4231.
pub fn hmac_sha256(key: &[u8], msg: &[u8]) -> [u8; 32] {
    const BLOCK: usize = 64;
    let mut padded = [0u8; BLOCK];
    if key.len() > BLOCK {
        padded[..32].copy_from_slice(&sha256(key));
    } else {
        padded[..key.len()].copy_from_slice(key);
    }
    let mut ipad = [0x36u8; BLOCK];
    let mut opad = [0x5cu8; BLOCK];
    for i in 0..BLOCK {
        ipad[i] ^= padded[i];
        opad[i] ^= padded[i];
    }
    let mut inner = Sha256Hasher::new();
    inner.update(&ipad);
    inner.update(msg);
    let inner_hash = inner.finalize();
    let mut outer = Sha256Hasher::new();
    outer.update(&opad);
    outer.update(&inner_hash);
    outer.finalize()
}

/// Suy ra khóa bằng PBKDF2-HMAC-SHA-256 (RFC 8018).
///
/// Đây là biến thể *rút gọn* của PBKDF2 chuẩn Kerberos (vốn dùng HMAC-SHA-1 với 4096 vòng);
/// thuật toán học hoàn toàn giống, chỉ đổi hàm băm để tái sử dụng SHA-256 đã cài sẵn.
pub fn pbkdf2_hmac_sha256(password: &[u8], salt: &[u8], iterations: u32, dklen: usize) -> Vec<u8> {
    let mut out = Vec::with_capacity(dklen);
    let mut block_index: u32 = 1;
    while out.len() < dklen {
        let mut salted = Vec::with_capacity(salt.len() + 4);
        salted.extend_from_slice(salt);
        salted.extend_from_slice(&block_index.to_be_bytes());
        let mut u = hmac_sha256(password, &salted);
        let mut acc = u;
        for _ in 1..iterations {
            u = hmac_sha256(password, &u);
            for (i, byte) in u.iter().enumerate() {
                acc[i] ^= *byte;
            }
        }
        out.extend_from_slice(&acc);
        block_index += 1;
    }
    out.truncate(dklen);
    out
}

// ============================================================================
// 5. NTLM: NT HASH, NTLMv2 RESPONSE & LM HASH (MS-NLMP)
// ============================================================================

/// Bộ băm NT Hash (MD4 trên mật khẩu mã hoá UTF-16LE).
///
/// NT Hash là "bản băm" của mật khẩu Windows: `NT_Hash = MD4(UTF16LE(password))`.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct NtlmHasher;

impl Default for NtlmHasher {
    fn default() -> Self {
        Self::new()
    }
}

impl NtlmHasher {
    /// Khởi tạo bộ băm NT.
    pub fn new() -> Self {
        NtlmHasher
    }

    /// Tính NT Hash của một mật khẩu.
    pub fn nt_hash(password: &str) -> [u8; 16] {
        crate::nt_hash(password)
    }

    /// Tính NT Hash rồi trả về chuỗi hex viết thường.
    pub fn nt_hash_hex(password: &str) -> String {
        crate::nt_hash_hex(password)
    }
}

/// Tính NT Hash của một mật khẩu: `MD4(UTF16LE(password))`.
pub fn nt_hash(password: &str) -> [u8; 16] {
    md4(&to_utf16le(password))
}

/// Tính NT Hash rồi trả về chuỗi hex viết thường.
pub fn nt_hash_hex(password: &str) -> String {
    to_hex(&nt_hash(password))
}

/// Bộ bẻ khoá LM Hash — bản rút gọn minh hoạ cho mật khẩu kiểu cũ.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct LmHashCracker {
    /// Độ dài tối đa của mật khẩu LM (14 ký tự theo đặc tả Windows).
    pub max_length: usize,
}

impl Default for LmHashCracker {
    fn default() -> Self {
        Self::new()
    }
}

impl LmHashCracker {
    /// Khởi tạo bộ bẻ khoá với độ dài mật khẩu LM chuẩn (14 ký tự).
    pub fn new() -> Self {
        Self { max_length: 14 }
    }

    /// Tính LM Hash rút gọn của một mật khẩu.
    pub fn lm_hash(password: &str) -> [u8; 16] {
        crate::lm_hash(password)
    }

    /// Tìm mật khẩu trong từ điển khớp với giá trị LM Hash cho trước.
    pub fn crack(&self, target_hash: &str, wordlist: &[String]) -> Option<String> {
        let target = target_hash.to_ascii_lowercase();
        wordlist
            .iter()
            .find(|candidate| to_hex(&Self::lm_hash(candidate)).to_ascii_lowercase() == target)
            .cloned()
    }
}

/// Tính **LM Hash rút gọn** — KHÔNG phải LMHASH chuẩn của Windows.
///
/// LMHASH chuẩn dùng DES: `LMHASH = DES(pwd[0..8]) || DES(pwd[8..16])` với mật khẩu viết HOA,
/// cắt còn 14 ký tự rồi đệm NUL. Ở đây ta giữ nguyên *cấu trúc* chia hai nửa 8 byte nhưng
/// thay lõi DES bằng MD5 cắt 8 byte, nhằm tránh kéo thêm DES vào chương này.
/// Hàm này **chỉ dùng để minh hoạ luồng "bẻ khoá băm cũ"**, tuyệt đối không tương đương LMHASH thật.
pub fn lm_hash(password: &str) -> [u8; 16] {
    let upper: String = password.to_uppercase().chars().take(14).collect();
    let mut key = [0u8; 16];
    let bytes = upper.as_bytes();
    let take = bytes.len().min(16);
    key[..take].copy_from_slice(&bytes[..take]);
    let first = md5(&key[..8]);
    let second = md5(&key[8..]);
    let mut out = [0u8; 16];
    out[..8].copy_from_slice(&first[..8]);
    out[8..].copy_from_slice(&second[..8]);
    out
}

/// Bộ đáp ứng NTLMv2 — mô phỏng phía máy khách trong giao thức NTLM Challenge/Response.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct NetNtlmV2Responder {
    /// Tên đăng nhập của người dùng.
    pub username: String,
    /// Tên miền của người dùng.
    pub domain: String,
    /// Mật khẩu (chỉ dùng để suy ra NT Hash trong ví dụ tất định).
    pub password: String,
}

impl NetNtlmV2Responder {
    /// Khởi tạo bộ đáp ứng cho một bộ ba (người dùng, miền, mật khẩu).
    pub fn new(username: &str, domain: &str, password: &str) -> Self {
        Self {
            username: username.to_string(),
            domain: domain.to_string(),
            password: password.to_string(),
        }
    }

    /// NT Hash của mật khẩu đang cầm.
    pub fn nt_hash(&self) -> [u8; 16] {
        nt_hash(&self.password)
    }

    /// Tính NTLMv2 Hash: `HMAC-MD5(NT_Hash, UPPER(User) || UPPER(Domain))`.
    ///
    /// Hai tham số phải được truyền vào **đã viết hoa** đúng như đặc tả MS-NLMP yêu cầu.
    pub fn ntlmv2_hash(&self, user_upper: &str, domain_upper: &str) -> [u8; 16] {
        let mut buffer = Vec::with_capacity(user_upper.len() + domain_upper.len());
        buffer.extend_from_slice(user_upper.as_bytes());
        buffer.extend_from_slice(domain_upper.as_bytes());
        hmac_md5(&self.nt_hash(), &buffer)
    }

    /// Tính NTProofStr bằng biến thể *giữ nguyên* hoa/thường của tên và miền.
    ///
    /// `NTProofStr = HMAC-MD5(NTLMv2Hash, ServerChallenge || ClientChallenge || timestamp_LE || blob)`
    pub fn compute_response(
        &self,
        server_challenge: &[u8; 8],
        client_challenge: &[u8; 8],
        timestamp: i64,
    ) -> [u8; 16] {
        let ntv2 = self.ntlmv2_hash(&self.username, &self.domain);
        self.proof_string(&ntv2, server_challenge, client_challenge, timestamp)
    }

    /// Tính NTProofStr bằng biến thể *viết hoa toàn bộ* tên và miền.
    pub fn compute_response_upper(
        &self,
        server_challenge: &[u8; 8],
        client_challenge: &[u8; 8],
        timestamp: i64,
    ) -> [u8; 16] {
        let ntv2 = self.ntlmv2_hash(&self.username.to_uppercase(), &self.domain.to_uppercase());
        self.proof_string(&ntv2, server_challenge, client_challenge, timestamp)
    }

    /// Tính Session Key: `HMAC-MD5(NTLMv2Hash, NTProofStr)`.
    pub fn session_key(&self, proof_str: &[u8; 16]) -> [u8; 16] {
        let ntv2 = self.ntlmv2_hash(&self.username.to_uppercase(), &self.domain.to_uppercase());
        hmac_md5(&ntv2, proof_str)
    }

    /// Dựng blob NT v2 rồi băm thành NTProofStr.
    fn proof_string(
        &self,
        ntv2: &[u8; 16],
        server_challenge: &[u8; 8],
        client_challenge: &[u8; 8],
        timestamp: i64,
    ) -> [u8; 16] {
        let mut blob = Vec::with_capacity(28);
        blob.extend_from_slice(&[0x01, 0x01, 0x00, 0x00]);
        blob.extend_from_slice(&[0x00, 0x00, 0x00, 0x00]);
        blob.extend_from_slice(&timestamp.to_le_bytes());
        blob.extend_from_slice(client_challenge);
        blob.extend_from_slice(&[0x00, 0x00, 0x00, 0x00]);

        let mut data = Vec::with_capacity(8 + 8 + 8 + blob.len());
        data.extend_from_slice(server_challenge);
        data.extend_from_slice(client_challenge);
        data.extend_from_slice(&timestamp.to_le_bytes());
        data.extend_from_slice(&blob);
        hmac_md5(ntv2, &data)
    }
}

// ============================================================================
// 6. RC4 — MÃ HOÁ DÒNG (RFC 4758)
// ============================================================================

/// Bộ sinh dòng khoá RC4 (ARC4).
#[derive(Debug, Clone)]
pub struct Rc4 {
    /// Mảng hoán vị trạng thái S (256 byte).
    s: [u8; 256],
    /// Chỉ số i của PRGA.
    i: u8,
    /// Chỉ số j của PRGA.
    j: u8,
}

impl Rc4 {
    /// Khởi tạo RC4 từ khoá; chạy thuật toán hoán vị khoá (KSA).
    ///
    /// Khoá rỗng không gây panic: KSA được bỏ qua và dòng khoá sinh ra theo S-box khởi tạo.
    pub fn new(key: &[u8]) -> Self {
        let mut s = [0u8; 256];
        for (i, value) in s.iter_mut().enumerate() {
            *value = i as u8;
        }
        let mut j: u8 = 0;
        if !key.is_empty() {
            for i in 0..256 {
                j = j.wrapping_add(s[i]).wrapping_add(key[i % key.len()]);
                s.swap(i, j as usize);
            }
        }
        Self { s, i: 0, j: 0 }
    }

    /// Mã hoá (hoặc giải mã — RC4 là mã hoá dòng tự đồng bộ) một mảng byte.
    pub fn apply(&mut self, data: &[u8]) -> Vec<u8> {
        let mut out = Vec::with_capacity(data.len());
        for &byte in data {
            self.i = self.i.wrapping_add(1);
            self.j = self.j.wrapping_add(self.s[self.i as usize]);
            self.s.swap(self.i as usize, self.j as usize);
            let k =
                self.s[(self.s[self.i as usize].wrapping_add(self.s[self.j as usize])) as usize];
            out.push(byte ^ k);
        }
        out
    }
}

/// Mã hoá RC4 một lần: `RC4(key, data)`.
pub fn rc4(key: &[u8], data: &[u8]) -> Vec<u8> {
    Rc4::new(key).apply(data)
}

// ============================================================================
// 7. AES-256 + CBC/CTS — TỰ CÀI TỪ ĐẦU (FIPS-197, RFC 3962)
// ============================================================================

/// S-box chuẩn của AES theo FIPS-197 (256 byte).
const AES_SBOX: [u8; 256] = [
    0x63, 0x7c, 0x77, 0x7b, 0xf2, 0x6b, 0x6f, 0xc5, 0x30, 0x01, 0x67, 0x2b, 0xfe, 0xd7, 0xab, 0x76,
    0xca, 0x82, 0xc9, 0x7d, 0xfa, 0x59, 0x47, 0xf0, 0xad, 0xd4, 0xa2, 0xaf, 0x9c, 0xa4, 0x72, 0xc0,
    0xb7, 0xfd, 0x93, 0x26, 0x36, 0x3f, 0xf7, 0xcc, 0x34, 0xa5, 0xe5, 0xf1, 0x71, 0xd8, 0x31, 0x15,
    0x04, 0xc7, 0x23, 0xc3, 0x18, 0x96, 0x05, 0x9a, 0x07, 0x12, 0x80, 0xe2, 0xeb, 0x27, 0xb2, 0x75,
    0x09, 0x83, 0x2c, 0x1a, 0x1b, 0x6e, 0x5a, 0xa0, 0x52, 0x3b, 0xd6, 0xb3, 0x29, 0xe3, 0x2f, 0x84,
    0x53, 0xd1, 0x00, 0xed, 0x20, 0xfc, 0xb1, 0x5b, 0x6a, 0xcb, 0xbe, 0x39, 0x4a, 0x4c, 0x58, 0xcf,
    0xd0, 0xef, 0xaa, 0xfb, 0x43, 0x4d, 0x33, 0x85, 0x45, 0xf9, 0x02, 0x7f, 0x50, 0x3c, 0x9f, 0xa8,
    0x51, 0xa3, 0x40, 0x8f, 0x92, 0x9d, 0x38, 0xf5, 0xbc, 0xb6, 0xda, 0x21, 0x10, 0xff, 0xf3, 0xd2,
    0xcd, 0x0c, 0x13, 0xec, 0x5f, 0x97, 0x44, 0x17, 0xc4, 0xa7, 0x7e, 0x3d, 0x64, 0x5d, 0x19, 0x73,
    0x60, 0x81, 0x4f, 0xdc, 0x22, 0x2a, 0x90, 0x88, 0x46, 0xee, 0xb8, 0x14, 0xde, 0x5e, 0x0b, 0xdb,
    0xe0, 0x32, 0x3a, 0x0a, 0x49, 0x06, 0x24, 0x5c, 0xc2, 0xd3, 0xac, 0x62, 0x91, 0x95, 0xe4, 0x79,
    0xe7, 0xc8, 0x37, 0x6d, 0x8d, 0xd5, 0x4e, 0xa9, 0x6c, 0x56, 0xf4, 0xea, 0x65, 0x7a, 0xae, 0x08,
    0xba, 0x78, 0x25, 0x2e, 0x1c, 0xa6, 0xb4, 0xc6, 0xe8, 0xdd, 0x74, 0x1f, 0x4b, 0xbd, 0x8b, 0x8a,
    0x70, 0x3e, 0xb5, 0x66, 0x48, 0x03, 0xf6, 0x0e, 0x61, 0x35, 0x57, 0xb9, 0x86, 0xc1, 0x1d, 0x9e,
    0xe1, 0xf8, 0x98, 0x11, 0x69, 0xd9, 0x8e, 0x94, 0x9b, 0x1e, 0x87, 0xe9, 0xce, 0x55, 0x28, 0xdf,
    0x8c, 0xa1, 0x89, 0x0d, 0xbf, 0xe6, 0x42, 0x68, 0x41, 0x99, 0x2d, 0x0f, 0xb0, 0x54, 0xbb, 0x16,
];

/// Hằng số tròn (Rcon) của AES.
const AES_RCON: [u8; 11] = [
    0x00, 0x01, 0x02, 0x04, 0x08, 0x10, 0x20, 0x40, 0x80, 0x1b, 0x36,
];

/// Dựng S-box nghịch đảo tại thời điểm biên dịch từ S-box chuẩn.
const fn build_inverse_sbox() -> [u8; 256] {
    let mut inverse = [0u8; 256];
    let mut i = 0usize;
    while i < 256 {
        inverse[AES_SBOX[i] as usize] = i as u8;
        i += 1;
    }
    inverse
}

/// S-box nghịch đảo của AES.
const AES_INV_SBOX: [u8; 256] = build_inverse_sbox();

/// Nhân trong trường GF(2^8) với đa thức rút gọn x^8 + x^4 + x^3 + x + 1.
fn gf_mul(mut a: u8, mut b: u8) -> u8 {
    let mut result = 0u8;
    for _ in 0..8 {
        if b & 1 != 0 {
            result ^= a;
        }
        let carry = a & 0x80;
        a <<= 1;
        if carry != 0 {
            a ^= 0x1b;
        }
        b >>= 1;
    }
    result
}

/// Áp dụng S-box cho từng byte của một từ 32-bit.
fn sub_word(word: u32) -> u32 {
    let bytes = word.to_be_bytes();
    u32::from_be_bytes([
        AES_SBOX[bytes[0] as usize],
        AES_SBOX[bytes[1] as usize],
        AES_SBOX[bytes[2] as usize],
        AES_SBOX[bytes[3] as usize],
    ])
}

/// Mở rộng khoá AES thành các khoá vòng (round keys).
///
/// Hỗ trợ AES-128 (10 vòng), AES-192 (12 vòng) và AES-256 (14 vòng).
pub fn aes_key_expansion(key: &[u8]) -> Vec<[u8; 16]> {
    assert!(
        key.len() == 16 || key.len() == 24 || key.len() == 32,
        "khoá AES phải dài 16, 24 hoặc 32 byte, nhận: {}",
        key.len()
    );
    let nk = key.len() / 4;
    let rounds = nk + 6;
    let total_words = 4 * (rounds + 1);
    let mut w = vec![0u32; total_words];
    for (i, word) in w.iter_mut().take(nk).enumerate() {
        *word = u32::from_be_bytes([key[i * 4], key[i * 4 + 1], key[i * 4 + 2], key[i * 4 + 3]]);
    }
    for i in nk..total_words {
        let mut temp = w[i - 1];
        if i % nk == 0 {
            temp = sub_word(temp.rotate_left(8)) ^ ((AES_RCON[i / nk] as u32) << 24);
        } else if nk > 6 && i % nk == 4 {
            temp = sub_word(temp);
        }
        w[i] = w[i - nk] ^ temp;
    }
    let mut round_keys = Vec::with_capacity(rounds + 1);
    for round in 0..=rounds {
        let mut rk = [0u8; 16];
        for col in 0..4 {
            let word = w[round * 4 + col];
            rk[col * 4] = (word >> 24) as u8;
            rk[col * 4 + 1] = (word >> 16) as u8;
            rk[col * 4 + 2] = (word >> 8) as u8;
            rk[col * 4 + 3] = word as u8;
        }
        round_keys.push(rk);
    }
    round_keys
}

/// Cộng khoá vòng vào trạng thái (trạng thái lưu theo thứ tự cột).
fn add_round_key(state: &mut [u8; 16], round_key: &[u8; 16]) {
    for i in 0..16 {
        state[i] ^= round_key[i];
    }
}

/// Thay thế byte (SubBytes).
fn sub_bytes(state: &mut [u8; 16]) {
    for byte in state.iter_mut() {
        *byte = AES_SBOX[*byte as usize];
    }
}

/// Thay thế byte nghịch đảo (InvSubBytes).
fn inv_sub_bytes(state: &mut [u8; 16]) {
    for byte in state.iter_mut() {
        *byte = AES_INV_SBOX[*byte as usize];
    }
}

/// Dịch hàng (ShiftRows).
fn shift_rows(state: &mut [u8; 16]) {
    let original = *state;
    for row in 1..4 {
        for col in 0..4 {
            state[row + 4 * col] = original[row + 4 * ((col + row) % 4)];
        }
    }
}

/// Dịch hàng nghịch đảo (InvShiftRows).
fn inv_shift_rows(state: &mut [u8; 16]) {
    let original = *state;
    for row in 1..4 {
        for col in 0..4 {
            state[row + 4 * ((col + row) % 4)] = original[row + 4 * col];
        }
    }
}

/// Trộn cột (MixColumns).
fn mix_columns(state: &mut [u8; 16]) {
    for col in 0..4 {
        let base = col * 4;
        let a = [
            state[base],
            state[base + 1],
            state[base + 2],
            state[base + 3],
        ];
        state[base] = gf_mul(a[0], 2) ^ gf_mul(a[1], 3) ^ a[2] ^ a[3];
        state[base + 1] = a[0] ^ gf_mul(a[1], 2) ^ gf_mul(a[2], 3) ^ a[3];
        state[base + 2] = a[0] ^ a[1] ^ gf_mul(a[2], 2) ^ gf_mul(a[3], 3);
        state[base + 3] = gf_mul(a[0], 3) ^ a[1] ^ a[2] ^ gf_mul(a[3], 2);
    }
}

/// Trộn cột nghịch đảo (InvMixColumns).
fn inv_mix_columns(state: &mut [u8; 16]) {
    for col in 0..4 {
        let base = col * 4;
        let a = [
            state[base],
            state[base + 1],
            state[base + 2],
            state[base + 3],
        ];
        state[base] = gf_mul(a[0], 14) ^ gf_mul(a[1], 11) ^ gf_mul(a[2], 13) ^ gf_mul(a[3], 9);
        state[base + 1] = gf_mul(a[0], 9) ^ gf_mul(a[1], 14) ^ gf_mul(a[2], 11) ^ gf_mul(a[3], 13);
        state[base + 2] = gf_mul(a[0], 13) ^ gf_mul(a[1], 9) ^ gf_mul(a[2], 14) ^ gf_mul(a[3], 11);
        state[base + 3] = gf_mul(a[0], 11) ^ gf_mul(a[1], 13) ^ gf_mul(a[2], 9) ^ gf_mul(a[3], 14);
    }
}

/// Mã hoá một khối 16 byte bằng AES (vector FIPS-197 dùng cho kiểm thử).
pub fn aes_encrypt_block(key: &[u8], block: &[u8; 16]) -> [u8; 16] {
    let round_keys = aes_key_expansion(key);
    let rounds = round_keys.len() - 1;
    let mut state = *block;
    add_round_key(&mut state, &round_keys[0]);
    for round_key in round_keys.iter().take(rounds).skip(1) {
        sub_bytes(&mut state);
        shift_rows(&mut state);
        mix_columns(&mut state);
        add_round_key(&mut state, round_key);
    }
    sub_bytes(&mut state);
    shift_rows(&mut state);
    add_round_key(&mut state, &round_keys[rounds]);
    state
}

/// Giải mã một khối 16 byte bằng AES (Cipher bậc nghịch đảo của FIPS-197).
pub fn aes_decrypt_block(key: &[u8], block: &[u8; 16]) -> [u8; 16] {
    let round_keys = aes_key_expansion(key);
    let rounds = round_keys.len() - 1;
    let mut state = *block;
    add_round_key(&mut state, &round_keys[rounds]);
    for round in (1..rounds).rev() {
        inv_shift_rows(&mut state);
        inv_sub_bytes(&mut state);
        add_round_key(&mut state, &round_keys[round]);
        inv_mix_columns(&mut state);
    }
    inv_shift_rows(&mut state);
    inv_sub_bytes(&mut state);
    add_round_key(&mut state, &round_keys[0]);
    state
}

/// Kích thước khối AES.
const AES_BLOCK: usize = 16;

/// Mã hoá AES-CBC kèm "ciphertext stealing" (CTS) theo RFC 2040 §8 (có errata RFC 3962 Phụ lục A).
///
/// Thuật toán xử lý riêng hai khối cuối:
///
/// 1. `X_{n-1} = P_{n-1} XOR C_{n-2}` (dùng IV nếu không có `C_{n-2}`), `E_{n-1} = E(X_{n-1})`.
/// 2. `C_n` = `L_n` byte đầu của `E_{n-1}` (đây là phần "bị cướp").
/// 3. `D_n = E_{n-1} XOR P_n(đệm 0)`, `C_{n-1} = E(D_n)`.
/// 4. Bản mã = `C_1..C_{n-2} || C_{n-1} || C_n` — **giữ nguyên độ dài bản rõ**.
pub fn aes_cts_encrypt_with_iv(key: &[u8], iv: &[u8; 16], plaintext: &[u8]) -> Vec<u8> {
    if plaintext.len() <= AES_BLOCK {
        let mut block = [0u8; AES_BLOCK];
        block[..plaintext.len()].copy_from_slice(plaintext);
        for (index, byte) in block.iter_mut().enumerate() {
            *byte ^= iv[index];
        }
        return aes_encrypt_block(key, &block).to_vec();
    }
    let blocks = plaintext.len().div_ceil(AES_BLOCK);
    let last_len = plaintext.len() - (blocks - 1) * AES_BLOCK;
    let mut chain = *iv;
    let mut output = Vec::with_capacity(plaintext.len());
    for index in 0..(blocks - 2) {
        let start = index * AES_BLOCK;
        let mut block = [0u8; AES_BLOCK];
        block.copy_from_slice(&plaintext[start..start + AES_BLOCK]);
        for (position, byte) in block.iter_mut().enumerate() {
            *byte ^= chain[position];
        }
        let cipher = aes_encrypt_block(key, &block);
        output.extend_from_slice(&cipher);
        chain = cipher;
    }
    let start = (blocks - 2) * AES_BLOCK;
    let mut block = [0u8; AES_BLOCK];
    block.copy_from_slice(&plaintext[start..start + AES_BLOCK]);
    for (position, byte) in block.iter_mut().enumerate() {
        *byte ^= chain[position];
    }
    let e_last = aes_encrypt_block(key, &block);
    let mut tail = [0u8; AES_BLOCK];
    tail[..last_len].copy_from_slice(&plaintext[(blocks - 1) * AES_BLOCK..]);
    for (position, byte) in tail.iter_mut().enumerate() {
        *byte ^= e_last[position];
    }
    let c_previous = aes_encrypt_block(key, &tail);
    output.extend_from_slice(&c_previous);
    output.extend_from_slice(&e_last[..last_len]);
    output
}

/// Giải mã AES-CBC kèm "ciphertext stealing" theo RFC 2040 §8 (đảo ngược mã hoá).
///
/// Bản rõ = `P_1..P_{n-2} || P_{n-1} || P_n`, với `P_n` chỉ có `L_n` byte.
pub fn aes_cts_decrypt_with_iv(key: &[u8], iv: &[u8; 16], ciphertext: &[u8]) -> Vec<u8> {
    if ciphertext.len() <= AES_BLOCK {
        let mut block = [0u8; AES_BLOCK];
        block[..ciphertext.len()].copy_from_slice(ciphertext);
        let mut plain = aes_decrypt_block(key, &block);
        for (position, byte) in plain.iter_mut().enumerate() {
            *byte ^= iv[position];
        }
        return plain.to_vec();
    }
    let blocks = ciphertext.len().div_ceil(AES_BLOCK);
    let last_len = ciphertext.len() - (blocks - 1) * AES_BLOCK;
    let mut chain = *iv;
    let mut output = Vec::with_capacity(ciphertext.len());
    for index in 0..(blocks - 2) {
        let start = index * AES_BLOCK;
        let mut block = [0u8; AES_BLOCK];
        block.copy_from_slice(&ciphertext[start..start + AES_BLOCK]);
        let plain = aes_decrypt_block(key, &block);
        for (position, byte) in plain.iter().enumerate() {
            output.push(byte ^ chain[position]);
        }
        chain = block;
    }
    let mut c_previous = [0u8; AES_BLOCK];
    c_previous.copy_from_slice(&ciphertext[(blocks - 2) * AES_BLOCK..(blocks - 1) * AES_BLOCK]);
    let d_last = aes_decrypt_block(key, &c_previous);
    let mut c_last = [0u8; AES_BLOCK];
    c_last[..last_len].copy_from_slice(&ciphertext[(blocks - 1) * AES_BLOCK..]);
    let mut x_last = [0u8; AES_BLOCK];
    for (position, byte) in x_last.iter_mut().enumerate() {
        *byte = d_last[position] ^ c_last[position];
    }
    let mut e_last = c_last;
    e_last[last_len..].copy_from_slice(&x_last[last_len..]);
    let mut p_previous = aes_decrypt_block(key, &e_last);
    for (position, byte) in p_previous.iter_mut().enumerate() {
        *byte ^= chain[position];
    }
    output.extend_from_slice(&p_previous);
    output.extend_from_slice(&x_last[..last_len]);
    output
}

/// Mã hoá AES-CBC-CTS với IV toàn 0 (đúng như Kerberos dùng).
pub fn aes_cts_encrypt_raw(key: &[u8], plaintext: &[u8]) -> Vec<u8> {
    aes_cts_encrypt_with_iv(key, &[0u8; 16], plaintext)
}

/// Giải mã AES-CBC-CTS với IV toàn 0 (đúng như Kerberos dùng).
pub fn aes_cts_decrypt_raw(key: &[u8], ciphertext: &[u8]) -> Vec<u8> {
    aes_cts_decrypt_with_iv(key, &[0u8; 16], ciphertext)
}

/// Mã hoá kiểu Kerberos `aes256-cts-hmac-sha1-96`: confounder 16 byte + bản rõ.
pub fn aes_cts_encrypt_kerberos(key: &[u8], core: &[u8]) -> Vec<u8> {
    let mut plain = Vec::with_capacity(AES_BLOCK + core.len());
    plain.extend_from_slice(&md5(key));
    plain.extend_from_slice(core);
    aes_cts_encrypt_raw(key, &plain)
}

/// Giải mã kiểu Kerberos `aes256-cts-hmac-sha1-96`, tự bỏ confounder 16 byte.
pub fn aes_cts_decrypt(key: &[u8], ciphertext: &[u8]) -> Vec<u8> {
    let raw = aes_cts_decrypt_raw(key, ciphertext);
    if raw.len() > AES_BLOCK {
        raw[AES_BLOCK..].to_vec()
    } else {
        Vec::new()
    }
}

// ============================================================================
// 8. ASN.1 DER RÚT GỘN — MÃ HOÁ & GIẢI MÃ THÔNG ĐIỆP KERBEROS
// ============================================================================

/// Mã hoá một TLV (Tag-Length-Value) với độ dài dạng xác định.
pub fn encode_tlv(tag: u8, content: &[u8]) -> Vec<u8> {
    let mut out = Vec::with_capacity(content.len() + 4);
    out.push(tag);
    out.extend(encode_length(content.len()));
    out.extend(content);
    out
}

/// Mã hoá trường độ dài theo DER (dạng ngắn nếu < 128, ngược lại dạng dài).
fn encode_length(len: usize) -> Vec<u8> {
    if len < 0x80 {
        return vec![len as u8];
    }
    let mut bytes = Vec::new();
    let mut value = len;
    while value > 0 {
        bytes.push((value & 0xff) as u8);
        value >>= 8;
    }
    bytes.reverse();
    let mut out = vec![0x80 | bytes.len() as u8];
    out.extend(bytes);
    out
}

/// Mã hoá một số nguyên (INTEGER, thẻ 0x02) theo dạng hai's complement tối thiểu.
pub fn encode_integer(value: i64) -> Vec<u8> {
    let bytes = value.to_be_bytes();
    let mut content;
    if value < 0 {
        // Số âm: cắt các byte 0xFF thừa ở đầu, nhưng phải giữ byte đầu có bit cao = 1.
        let mut start = 0usize;
        while start < 7 && bytes[start] == 0xff && bytes[start + 1] & 0x80 != 0 {
            start += 1;
        }
        content = bytes[start..].to_vec();
        if content[0] & 0x80 == 0 {
            content.insert(0, 0xff);
        }
    } else {
        // Số không âm: cắt các byte 0x00 thừa ở đầu, nhưng byte đầu phải có bit cao = 0.
        let mut start = 0usize;
        while start < 7 && bytes[start] == 0 && bytes[start + 1] & 0x80 == 0 {
            start += 1;
        }
        content = bytes[start..].to_vec();
        if content[0] & 0x80 != 0 {
            content.insert(0, 0x00);
        }
    }
    encode_tlv(0x02, &content)
}

/// Mã hoá một chuỗi octet (OCTET STRING, thẻ 0x04).
pub fn encode_octet_string(value: &[u8]) -> Vec<u8> {
    encode_tlv(0x04, value)
}

/// Mã hoá một chuỗi ký tự tổng quát (GeneralString, thẻ 0x1B).
pub fn encode_general_string(value: &str) -> Vec<u8> {
    encode_tlv(0x1b, value.as_bytes())
}

/// Mã hoá một SEQUENCE (thẻ 0x30) chứa nhiều TLV con.
pub fn encode_sequence(items: &[Vec<u8>]) -> Vec<u8> {
    let mut content = Vec::new();
    for item in items {
        content.extend(item);
    }
    encode_tlv(0x30, &content)
}

/// Mã hoá một SEQUENCE chứa các phần tử không có thẻ riêng.
pub fn encode_sequence_of(items: &[Vec<u8>]) -> Vec<u8> {
    encode_sequence(items)
}

/// Mã hoá một thẻ ứng dụng (APPLICATION, thẻ 0x6n) dạng constructed.
pub fn encode_application(tag: u8, items: &[Vec<u8>]) -> Vec<u8> {
    let mut content = Vec::new();
    for item in items {
        content.extend(item);
    }
    encode_tlv(0x60 | (tag & 0x1f), &content)
}

/// Mã hoá một thẻ ngữ cảnh (CONTEXT-SPECIFIC, thẻ 0xAn) dạng constructed.
pub fn encode_context(tag: u8, items: &[Vec<u8>]) -> Vec<u8> {
    let mut content = Vec::new();
    for item in items {
        content.extend(item);
    }
    encode_tlv(0xa0 | (tag & 0x1f), &content)
}

/// Giải mã một TLV tại vị trí `offset`.
///
/// Trả về `(thẻ, nội dung, vị trí kế tiếp)`. Nội dung của thẻ constructed là chuỗi TLV con
/// thô, người gọi tự giải mã tiếp.
pub fn decode_tlv(bytes: &[u8], offset: usize) -> Result<(u8, Vec<u8>, usize), String> {
    if offset >= bytes.len() {
        return Err(format!("vượt quá cuối dữ liệu tại vị trí {offset}"));
    }
    let tag = bytes[offset];
    let mut pos = offset + 1;
    if pos >= bytes.len() {
        return Err("thiếu byte độ dài".to_string());
    }
    let first = bytes[pos];
    pos += 1;
    let length = if first < 0x80 {
        first as usize
    } else {
        let count = (first & 0x7f) as usize;
        if count == 0 {
            return Err("chưa hỗ trợ độ dài không xác định".to_string());
        }
        if count > 8 {
            return Err("độ dài quá lớn".to_string());
        }
        if pos + count > bytes.len() {
            return Err("thiếu byte độ dài".to_string());
        }
        let mut value = 0usize;
        for byte in &bytes[pos..pos + count] {
            value = (value << 8) | *byte as usize;
        }
        pos += count;
        value
    };
    if pos + length > bytes.len() {
        return Err("nội dung bị cắt cụt".to_string());
    }
    let content = bytes[pos..pos + length].to_vec();
    Ok((tag, content, pos + length))
}

/// Giải mã toàn bộ các TLV con của một nội dung constructed.
fn parse_children(content: &[u8]) -> Result<Vec<(u8, Vec<u8>)>, String> {
    let mut children = Vec::new();
    let mut pos = 0usize;
    while pos < content.len() {
        let (tag, child, next) = decode_tlv(content, pos)?;
        children.push((tag, child));
        pos = next;
    }
    Ok(children)
}

/// Đọc chuỗi đầu tiên nằm trong một SEQUENCE OF KerberosString.
fn first_string_child(content: &[u8]) -> Result<String, String> {
    let (tag, seq, _) = decode_tlv(content, 0)?;
    if tag != 0x30 {
        return Err(format!(
            "mong đợi SEQUENCE OF KerberosString, nhận 0x{tag:02x}"
        ));
    }
    let strings = parse_children(&seq)?;
    match strings.first() {
        Some((_, first)) => Ok(String::from_utf8_lossy(first).to_string()),
        None => Ok(String::new()),
    }
}

/// Tìm TLV con theo thẻ.
fn find_child(children: &[(u8, Vec<u8>)], tag: u8) -> Option<&Vec<u8>> {
    children
        .iter()
        .find(|(child_tag, _)| *child_tag == tag)
        .map(|(_, content)| content)
}

/// Đọc một số nguyên nằm trong nội dung của một TLV.
fn read_integer(content: &[u8]) -> Result<i64, String> {
    let (tag, value, _) = decode_tlv(content, 0)?;
    if tag != 0x02 {
        return Err(format!("mong đợi INTEGER (0x02), nhận 0x{tag:02x}"));
    }
    if value.is_empty() {
        return Ok(0);
    }
    let mut result: i64 = if value[0] & 0x80 != 0 { -1 } else { 0 };
    for byte in &value {
        result = (result << 8) | (*byte as i64);
    }
    Ok(result)
}

/// Đọc một chuỗi ký tự (GeneralString hoặc OCTET STRING).
fn read_string(content: &[u8]) -> Result<String, String> {
    let (tag, value, _) = decode_tlv(content, 0)?;
    match tag {
        0x1b | 0x04 | 0x0c | 0x13 => Ok(String::from_utf8_lossy(&value).to_string()),
        other => Err(format!("mong đợi chuỗi, nhận 0x{other:02x}")),
    }
}

/// Đọc một chuỗi octet.
fn read_octets(content: &[u8]) -> Result<Vec<u8>, String> {
    let (tag, value, _) = decode_tlv(content, 0)?;
    if tag != 0x04 {
        return Err(format!("mong đợi OCTET STRING (0x04), nhận 0x{tag:02x}"));
    }
    Ok(value)
}

// ============================================================================
// 9. GIAO THỨC KERBEROS 5 (RFC 4120) — CẤU TRÚC THÔNG ĐIỆP RÚT GỌN
// ============================================================================

/// Loại mã hoá (etype) của Kerberos.
///
/// Biểu diễn `#[repr(u16)]` để mã số etype (23, 18, 17) nằm thẳng trong kiểu;
/// biến thể `Unknown(u16)` đặt trước để không tranh chấp discriminant với các biến thể khác.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
#[repr(u16)]
pub enum Etype {
    /// etype chưa biết — mang theo mã số nhận được từ đường dây.
    Unknown(u16),
    /// RC4-HMAC — etype 23 (đã lỗi thời nhưng vẫn rất phổ biến).
    Rc4Hmac = 23,
    /// AES-256-CTS-HMAC-SHA1-96 — etype 18.
    Aes256Cts = 18,
    /// AES-128-CTS-HMAC-SHA1-96 — etype 17.
    Aes128Cts = 17,
}

impl Etype {
    /// Trả về mã số etype.
    pub fn id(&self) -> u16 {
        match self {
            Etype::Rc4Hmac => 23,
            Etype::Aes256Cts => 18,
            Etype::Aes128Cts => 17,
            Etype::Unknown(value) => *value,
        }
    }

    /// Dựng etype từ mã số.
    pub fn from_id(value: u16) -> Etype {
        match value {
            23 => Etype::Rc4Hmac,
            18 => Etype::Aes256Cts,
            17 => Etype::Aes128Cts,
            other => Etype::Unknown(other),
        }
    }

    /// Độ dài khoá (byte) của etype.
    pub fn key_length(&self) -> usize {
        match self {
            Etype::Rc4Hmac => 16,
            Etype::Aes256Cts => 32,
            Etype::Aes128Cts => 16,
            Etype::Unknown(_) => 16,
        }
    }
}

impl fmt::Display for Etype {
    fn fmt(&self, f: &mut fmt::Formatter) -> fmt::Result {
        let name = match self {
            Etype::Rc4Hmac => "rc4-hmac",
            Etype::Aes256Cts => "aes256-cts",
            Etype::Aes128Cts => "aes128-cts",
            Etype::Unknown(_) => "unknown",
        };
        write!(f, "{name} ({})", self.id())
    }
}

/// Loại thông điệp Kerberos.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum KerberosMessageType {
    /// AS-REQ — yêu cầu vé mật.
    AsReq,
    /// AS-REP — phản hồi vé mật.
    AsRep,
    /// TGS-REQ — yêu cầu vé dịch vụ.
    TgsReq,
    /// TGS-REP — phản hồi vé dịch vụ.
    TgsRep,
    /// KRB-ERROR — thông điệp lỗi.
    KrbError,
}

impl KerberosMessageType {
    /// Trả về mã số msg-type theo RFC 4120.
    pub fn id(&self) -> u32 {
        match self {
            KerberosMessageType::AsReq => 10,
            KerberosMessageType::AsRep => 11,
            KerberosMessageType::TgsReq => 12,
            KerberosMessageType::TgsRep => 13,
            KerberosMessageType::KrbError => 30,
        }
    }

    /// Dựng loại thông điệp từ mã số.
    pub fn from_id(value: u32) -> KerberosMessageType {
        match value {
            10 => KerberosMessageType::AsReq,
            11 => KerberosMessageType::AsRep,
            12 => KerberosMessageType::TgsReq,
            13 => KerberosMessageType::TgsRep,
            _ => KerberosMessageType::KrbError,
        }
    }
}

/// Một phần tử PA-DATA (pre-authentication data) trong thông điệp Kerberos.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct PaData {
    /// Mã loại PA-DATA (ví dụ 2 = PA-ENC-TIMESTAMP, 3 = PA-ENC-TICKET-PART).
    pub padata_type: u16,
    /// etype gợi ý cho phần tử này (nếu có).
    pub etype: Option<Etype>,
    /// Giá trị thô của phần tử.
    pub value: Vec<u8>,
}

impl PaData {
    /// Mã hoá PA-DATA thành TLV ASN.1.
    pub fn to_asn1(&self) -> Vec<u8> {
        let mut items = vec![
            encode_context(1, &[encode_integer(self.padata_type as i64)]),
            encode_context(2, &[encode_octet_string(&self.value)]),
        ];
        if let Some(etype) = self.etype {
            items.push(encode_context(3, &[encode_integer(etype.id() as i64)]));
        }
        encode_sequence(&items)
    }

    /// Giải mã PA-DATA từ nội dung TLV.
    pub fn parse(content: &[u8]) -> Result<PaData, String> {
        let (tag, body, _) = decode_tlv(content, 0)?;
        if tag != 0x30 {
            return Err(format!("mong đợi SEQUENCE cho PA-DATA, nhận 0x{tag:02x}"));
        }
        let children = parse_children(&body)?;
        let padata_type = match find_child(&children, 0xa1) {
            Some(inner) => read_integer(inner)? as u16,
            None => 0,
        };
        let value = match find_child(&children, 0xa2) {
            Some(inner) => read_octets(inner)?,
            None => Vec::new(),
        };
        let etype = match find_child(&children, 0xa3) {
            Some(inner) => Some(Etype::from_id(read_integer(inner)? as u16)),
            None => None,
        };
        Ok(PaData {
            padata_type,
            etype,
            value,
        })
    }
}

/// Tên chính của một principal Kerberos.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct KrbPrincipal {
    /// Kiểu tên (1 = NT-PRINCIPAL, 2 = NT-SRV-INST).
    pub name_type: u32,
    /// Tên thành phần, ví dụ `alice` hoặc `HTTP/web01`.
    pub name: String,
    /// Vùng (realm), ví dụ `LAB.LOCAL`.
    pub realm: String,
}

impl KrbPrincipal {
    /// Khởi tạo một principal kiểu người dùng.
    pub fn user(name: &str, realm: &str) -> Self {
        Self {
            name_type: 1,
            name: name.to_string(),
            realm: realm.to_string(),
        }
    }

    /// Khởi tạo một principal kiểu dịch vụ.
    pub fn service(name: &str, realm: &str) -> Self {
        Self {
            name_type: 2,
            name: name.to_string(),
            realm: realm.to_string(),
        }
    }

    /// Mã hoá principal thành TLV ASN.1.
    ///
    /// `name-string[1]` là một SEQUENCE OF KerberosString theo RFC 4120.
    pub fn to_asn1(&self) -> Vec<u8> {
        encode_sequence(&[
            encode_context(0, &[encode_integer(self.name_type as i64)]),
            encode_context(
                1,
                &[encode_sequence_of(&[encode_general_string(&self.name)])],
            ),
        ])
    }

    /// Giải mã principal từ nội dung TLV.
    pub fn parse(content: &[u8]) -> Result<KrbPrincipal, String> {
        let (tag, body, _) = decode_tlv(content, 0)?;
        if tag != 0x30 {
            return Err(format!(
                "mong đợi SEQUENCE cho PrincipalName, nhận 0x{tag:02x}"
            ));
        }
        let children = parse_children(&body)?;
        let name_type = match find_child(&children, 0xa0) {
            Some(inner) => read_integer(inner)? as u32,
            None => 0,
        };
        let name = match find_child(&children, 0xa1) {
            Some(inner) => {
                let (tag, _, _) = decode_tlv(inner, 0)?;
                match tag {
                    0x30 => first_string_child(inner)?,
                    0x1b | 0x0c | 0x13 => read_string(inner)?,
                    other => {
                        return Err(format!(
                            "mong đợi SEQUENCE OF KerberosString, nhận 0x{other:02x}"
                        ));
                    }
                }
            }
            None => String::new(),
        };
        Ok(KrbPrincipal {
            name_type,
            name,
            realm: String::new(),
        })
    }
}

/// Thông điệp AS-REQ — yêu cầu vé mật (Authentication Service Request).
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct AsReq {
    /// Loại thông điệp.
    pub msg_type: KerberosMessageType,
    /// Principal của người dùng (cname).
    pub principal: KrbPrincipal,
    /// Vùng của người dùng.
    pub realm: String,
    /// Các phần tử pre-authentication.
    pub padatas: Vec<PaData>,
    /// Nonce ngẫu nhiên chống chơi lại.
    pub nonce: u32,
}

impl AsReq {
    /// Dựng một AS-REQ hoàn chỉnh cho principal và etype cho trước.
    pub fn build(principal: KrbPrincipal, realm: &str, etype: Etype) -> Self {
        let mut principal = principal;
        principal.realm = realm.to_string();
        let nonce = fnv1a32(format!("{realm}/{}", principal.name).as_bytes());
        let padatas = vec![PaData {
            padata_type: 19,
            etype: Some(etype),
            value: etype.id().to_be_bytes().to_vec(),
        }];
        Self {
            msg_type: KerberosMessageType::AsReq,
            principal,
            realm: realm.to_string(),
            padatas,
            nonce,
        }
    }

    /// Mã hoá AS-REQ thành byte ASN.1 DER.
    pub fn to_asn1(&self) -> Vec<u8> {
        let body = encode_sequence(&[
            encode_context(0, &[encode_integer(0)]),
            encode_context(1, &[self.principal.to_asn1()]),
            encode_context(2, &[encode_general_string(&self.realm)]),
            encode_context(7, &[encode_integer(self.nonce as i64)]),
        ]);
        let padata = encode_sequence_of(
            &self
                .padatas
                .iter()
                .map(PaData::to_asn1)
                .collect::<Vec<Vec<u8>>>(),
        );
        let inner = encode_sequence(&[
            encode_context(1, &[encode_integer(5)]),
            encode_context(2, &[encode_integer(self.msg_type.id() as i64)]),
            encode_context(3, &[padata]),
            encode_context(4, &[body]),
        ]);
        encode_application(10, &[inner])
    }

    /// Giải mã AS-REQ từ byte ASN.1 DER.
    pub fn parse(bytes: &[u8]) -> Result<AsReq, String> {
        let (tag, content, _) = decode_tlv(bytes, 0)?;
        if tag != 0x6a {
            return Err(format!("mong đợi APPLICATION 10, nhận 0x{tag:02x}"));
        }
        let (tag, inner, _) = decode_tlv(&content, 0)?;
        if tag != 0x30 {
            return Err(format!("mong đợi SEQUENCE KDC-REQ, nhận 0x{tag:02x}"));
        }
        let children = parse_children(&inner)?;
        let msg_type = match find_child(&children, 0xa2) {
            Some(inner) => KerberosMessageType::from_id(read_integer(inner)? as u32),
            None => KerberosMessageType::AsReq,
        };
        let padatas = match find_child(&children, 0xa3) {
            Some(inner) => {
                let (tag, seq, _) = decode_tlv(inner, 0)?;
                if tag != 0x30 {
                    return Err(format!("mong đợi SEQUENCE OF PA-DATA, nhận 0x{tag:02x}"));
                }
                let mut list = Vec::new();
                for item in parse_children(&seq)? {
                    list.push(PaData::parse(&encode_tlv(item.0, &item.1))?);
                }
                list
            }
            None => Vec::new(),
        };
        let body_content =
            find_child(&children, 0xa4).ok_or_else(|| "thiếu req-body".to_string())?;
        let (tag, body, _) = decode_tlv(body_content, 0)?;
        if tag != 0x30 {
            return Err(format!("mong đợi SEQUENCE req-body, nhận 0x{tag:02x}"));
        }
        let body_children = parse_children(&body)?;
        let principal = match find_child(&body_children, 0xa1) {
            Some(inner) => KrbPrincipal::parse(inner)?,
            None => KrbPrincipal {
                name_type: 0,
                name: String::new(),
                realm: String::new(),
            },
        };
        let realm = match find_child(&body_children, 0xa2) {
            Some(inner) => read_string(inner)?,
            None => String::new(),
        };
        let nonce = match find_child(&body_children, 0xa7) {
            Some(inner) => read_integer(inner)? as u32,
            None => 0,
        };
        let mut principal = principal;
        principal.realm = realm.clone();
        Ok(AsReq {
            msg_type,
            principal,
            realm,
            padatas,
            nonce,
        })
    }
}

/// Thông điệp AS-REP — phản hồi vé mật.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct AsRep {
    /// Loại thông điệp.
    pub msg_type: KerberosMessageType,
    /// Principal của người dùng.
    pub principal: KrbPrincipal,
    /// Các phần tử pre-authentication.
    pub padatas: Vec<PaData>,
    /// Phần EncryptedData đã mã hoá.
    pub enc_part: Vec<u8>,
    /// etype dùng để mã hoá `enc_part`.
    pub etype: Option<Etype>,
}

impl AsRep {
    /// Mã hoá AS-REP thành byte ASN.1 DER.
    pub fn to_asn1(&self) -> Vec<u8> {
        let mut enc_items = Vec::new();
        if let Some(etype) = self.etype {
            enc_items.push(encode_context(0, &[encode_integer(etype.id() as i64)]));
        }
        enc_items.push(encode_context(1, &[encode_octet_string(&self.enc_part)]));
        let padata = encode_sequence_of(
            &self
                .padatas
                .iter()
                .map(PaData::to_asn1)
                .collect::<Vec<Vec<u8>>>(),
        );
        let inner = encode_sequence(&[
            encode_context(1, &[encode_integer(5)]),
            encode_context(2, &[encode_integer(self.msg_type.id() as i64)]),
            encode_context(3, &[padata]),
            encode_context(4, &[encode_sequence(&enc_items)]),
        ]);
        encode_application(11, &[inner])
    }

    /// Giải mã AS-REP từ byte ASN.1 DER.
    pub fn parse(bytes: &[u8]) -> Result<AsRep, String> {
        let (tag, content, _) = decode_tlv(bytes, 0)?;
        if tag != 0x6b {
            return Err(format!("mong đợi APPLICATION 11, nhận 0x{tag:02x}"));
        }
        let (tag, inner, _) = decode_tlv(&content, 0)?;
        if tag != 0x30 {
            return Err(format!("mong đợi SEQUENCE KDC-REP, nhận 0x{tag:02x}"));
        }
        let children = parse_children(&inner)?;
        let msg_type = match find_child(&children, 0xa2) {
            Some(inner) => KerberosMessageType::from_id(read_integer(inner)? as u32),
            None => KerberosMessageType::AsRep,
        };
        let padatas = match find_child(&children, 0xa3) {
            Some(inner) => {
                let (tag, seq, _) = decode_tlv(inner, 0)?;
                if tag != 0x30 {
                    return Err(format!("mong đợi SEQUENCE OF PA-DATA, nhận 0x{tag:02x}"));
                }
                let mut list = Vec::new();
                for item in parse_children(&seq)? {
                    list.push(PaData::parse(&encode_tlv(item.0, &item.1))?);
                }
                list
            }
            None => Vec::new(),
        };
        let mut etype = None;
        let mut enc_part = Vec::new();
        if let Some(enc_content) = find_child(&children, 0xa4) {
            let (tag, body, _) = decode_tlv(enc_content, 0)?;
            if tag != 0x30 {
                return Err(format!("mong đợi SEQUENCE EncryptedData, nhận 0x{tag:02x}"));
            }
            let enc_children = parse_children(&body)?;
            if let Some(inner) = find_child(&enc_children, 0xa0) {
                etype = Some(Etype::from_id(read_integer(inner)? as u16));
            }
            if let Some(inner) = find_child(&enc_children, 0xa1) {
                enc_part = read_octets(inner)?;
            }
        }
        let principal = match find_child(&children, 0xa5) {
            Some(inner) => KrbPrincipal::parse(inner)?,
            None => KrbPrincipal {
                name_type: 0,
                name: String::new(),
                realm: String::new(),
            },
        };
        Ok(AsRep {
            msg_type,
            principal,
            padatas,
            enc_part,
            etype,
        })
    }

    /// Lấy phần tử PA-ENC-TIMESTAMP (padata-type 2).
    pub fn extract_pa_enc_timestamp(&self) -> Option<&PaData> {
        self.padatas.iter().find(|item| item.padata_type == 2)
    }

    /// Lấy phần Enc-Ticket-Part đã mã hoá (padata-type 3) — nguyên liệu để "roast".
    pub fn extract_enc_ticket_part(&self) -> Option<&PaData> {
        self.padatas.iter().find(|item| item.padata_type == 3)
    }
}

/// Thông điệp TGS-REQ — yêu cầu vé dịch vụ.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct TgsReq {
    /// Loại thông điệp.
    pub msg_type: KerberosMessageType,
    /// Principal của người dùng.
    pub principal: KrbPrincipal,
    /// Principal của dịch vụ được xin vé.
    pub service_principal: KrbPrincipal,
    /// Vé mật đã nhận (để chuyển tiếp).
    pub ticket: Vec<u8>,
    /// Các phần tử pre-authentication.
    pub padatas: Vec<PaData>,
}

impl TgsReq {
    /// Dựng một TGS-REQ cho principal và dịch vụ.
    pub fn build(principal: KrbPrincipal, service: KrbPrincipal, ticket: Vec<u8>) -> Self {
        Self {
            msg_type: KerberosMessageType::TgsReq,
            principal,
            service_principal: service,
            ticket,
            padatas: Vec::new(),
        }
    }

    /// Mã hoá TGS-REQ thành byte ASN.1 DER.
    pub fn to_asn1(&self) -> Vec<u8> {
        let body = encode_sequence(&[
            encode_context(0, &[encode_integer(0)]),
            encode_context(1, &[self.principal.to_asn1()]),
            encode_context(2, &[encode_general_string(&self.principal.realm)]),
            encode_context(3, &[self.service_principal.to_asn1()]),
            encode_context(11, &[encode_octet_string(&self.ticket)]),
        ]);
        let padata = encode_sequence_of(
            &self
                .padatas
                .iter()
                .map(PaData::to_asn1)
                .collect::<Vec<Vec<u8>>>(),
        );
        let inner = encode_sequence(&[
            encode_context(1, &[encode_integer(5)]),
            encode_context(2, &[encode_integer(self.msg_type.id() as i64)]),
            encode_context(3, &[padata]),
            encode_context(4, &[body]),
        ]);
        encode_application(12, &[inner])
    }

    /// Giải mã TGS-REQ từ byte ASN.1 DER.
    pub fn parse(bytes: &[u8]) -> Result<TgsReq, String> {
        let (tag, content, _) = decode_tlv(bytes, 0)?;
        if tag != 0x6c {
            return Err(format!("mong đợi APPLICATION 12, nhận 0x{tag:02x}"));
        }
        let (tag, inner, _) = decode_tlv(&content, 0)?;
        if tag != 0x30 {
            return Err(format!("mong đợi SEQUENCE KDC-REQ, nhận 0x{tag:02x}"));
        }
        let children = parse_children(&inner)?;
        let msg_type = match find_child(&children, 0xa2) {
            Some(inner) => KerberosMessageType::from_id(read_integer(inner)? as u32),
            None => KerberosMessageType::TgsReq,
        };
        let padatas = match find_child(&children, 0xa3) {
            Some(inner) => {
                let (tag, seq, _) = decode_tlv(inner, 0)?;
                if tag != 0x30 {
                    return Err(format!("mong đợi SEQUENCE OF PA-DATA, nhận 0x{tag:02x}"));
                }
                let mut list = Vec::new();
                for item in parse_children(&seq)? {
                    list.push(PaData::parse(&encode_tlv(item.0, &item.1))?);
                }
                list
            }
            None => Vec::new(),
        };
        let body_content =
            find_child(&children, 0xa4).ok_or_else(|| "thiếu req-body".to_string())?;
        let (tag, body, _) = decode_tlv(body_content, 0)?;
        if tag != 0x30 {
            return Err(format!("mong đợi SEQUENCE req-body, nhận 0x{tag:02x}"));
        }
        let body_children = parse_children(&body)?;
        let mut principal = match find_child(&body_children, 0xa1) {
            Some(inner) => KrbPrincipal::parse(inner)?,
            None => KrbPrincipal {
                name_type: 0,
                name: String::new(),
                realm: String::new(),
            },
        };
        let realm = match find_child(&body_children, 0xa2) {
            Some(inner) => read_string(inner)?,
            None => String::new(),
        };
        principal.realm = realm;
        let service_principal = match find_child(&body_children, 0xa3) {
            Some(inner) => KrbPrincipal::parse(inner)?,
            None => KrbPrincipal {
                name_type: 0,
                name: String::new(),
                realm: String::new(),
            },
        };
        let ticket = match find_child(&body_children, 0xab) {
            Some(inner) => read_octets(inner)?,
            None => Vec::new(),
        };
        Ok(TgsReq {
            msg_type,
            principal,
            service_principal,
            ticket,
            padatas,
        })
    }
}

/// Thông điệp TGS-REP — phản hồi vé dịch vụ.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct TgsRep {
    /// Loại thông điệp.
    pub msg_type: KerberosMessageType,
    /// Principal của người dùng.
    pub principal: KrbPrincipal,
    /// Phần EncryptedData đã mã hoá.
    pub enc_part: Vec<u8>,
    /// etype dùng để mã hoá `enc_part`.
    pub etype: Option<Etype>,
}

impl TgsRep {
    /// Mã hoá TGS-REP thành byte ASN.1 DER.
    pub fn to_asn1(&self) -> Vec<u8> {
        let mut enc_items = Vec::new();
        if let Some(etype) = self.etype {
            enc_items.push(encode_context(0, &[encode_integer(etype.id() as i64)]));
        }
        enc_items.push(encode_context(1, &[encode_octet_string(&self.enc_part)]));
        let inner = encode_sequence(&[
            encode_context(1, &[encode_integer(5)]),
            encode_context(2, &[encode_integer(self.msg_type.id() as i64)]),
            encode_context(4, &[encode_sequence(&enc_items)]),
            encode_context(5, &[self.principal.to_asn1()]),
        ]);
        encode_application(13, &[inner])
    }

    /// Giải mã TGS-REP từ byte ASN.1 DER.
    pub fn parse(bytes: &[u8]) -> Result<TgsRep, String> {
        let (tag, content, _) = decode_tlv(bytes, 0)?;
        if tag != 0x6d {
            return Err(format!("mong đợi APPLICATION 13, nhận 0x{tag:02x}"));
        }
        let (tag, inner, _) = decode_tlv(&content, 0)?;
        if tag != 0x30 {
            return Err(format!("mong đợi SEQUENCE KDC-REP, nhận 0x{tag:02x}"));
        }
        let children = parse_children(&inner)?;
        let msg_type = match find_child(&children, 0xa2) {
            Some(inner) => KerberosMessageType::from_id(read_integer(inner)? as u32),
            None => KerberosMessageType::TgsRep,
        };
        let mut etype = None;
        let mut enc_part = Vec::new();
        if let Some(enc_content) = find_child(&children, 0xa4) {
            let (tag, body, _) = decode_tlv(enc_content, 0)?;
            if tag != 0x30 {
                return Err(format!("mong đợi SEQUENCE EncryptedData, nhận 0x{tag:02x}"));
            }
            let enc_children = parse_children(&body)?;
            if let Some(inner) = find_child(&enc_children, 0xa0) {
                etype = Some(Etype::from_id(read_integer(inner)? as u16));
            }
            if let Some(inner) = find_child(&enc_children, 0xa1) {
                enc_part = read_octets(inner)?;
            }
        }
        let principal = match find_child(&children, 0xa5) {
            Some(inner) => KrbPrincipal::parse(inner)?,
            None => KrbPrincipal {
                name_type: 0,
                name: String::new(),
                realm: String::new(),
            },
        };
        Ok(TgsRep {
            msg_type,
            principal,
            enc_part,
            etype,
        })
    }

    /// Lấy phần Enc-Ticket-Part đã mã hoá từ EncryptedData.
    ///
    /// Trong mô hình rút gọn của chương này, TGS-REP không mang danh sách PA-DATA riêng:
    /// khối đã mã hoá nằm trong `enc_part`, còn hàm này trả về một *mô tả* của ô
    /// enc-ticket-part (padata-type 3) kèm etype của thông điệp, hoặc `None` nếu
    /// thông điệp không mang nội dung nào.
    pub fn extract_enc_ticket_part(&self) -> Option<&PaData> {
        if self.enc_part.is_empty() {
            return None;
        }
        Some(&TGS_ENC_TICKET_PART_SLOT)
    }
}

/// Mô tả tĩnh của ô enc-ticket-part (padata-type 3) trong TGS-REP.
static TGS_ENC_TICKET_PART_SLOT: PaData = PaData {
    padata_type: 3,
    etype: None,
    value: Vec::new(),
};

// ============================================================================
// 10. RC4-HMAC, KERBEROAST & AS-REP ROASTING
// ============================================================================

/// Độ dài confounder (byte) của lược đồ RC4-HMAC rút gọn.
pub const RC4_HMAC_CONFOUNDER_LEN: usize = 16;

/// Độ dài checksum (byte) của lược đồ RC4-HMAC rút gọn.
pub const RC4_HMAC_CHECKSUM_LEN: usize = 5;

/// Chữ ký đặc trưng ở đầu bản rõ của vé dịch vụ — dùng làm "oracle" khi bẻ khoá.
pub const TICKET_PLAINTEXT_MAGIC: &[u8; 8] = b"KRB5TICK";

/// Số vòng lặp PBKDF2 dùng trong chương này (Kerberos thật dùng 4096 vòng HMAC-SHA-1).
pub const PBKDF2_ITERATIONS: u32 = 1000;

/// Tên người dùng mặc định dùng cho biến thể NTLMv2 của `hash_for`.
pub const DEMO_NTLMV2_USER: &str = "ADMIN";

/// Miền mặc định dùng cho biến thể NTLMv2 của `hash_for`.
pub const DEMO_NTLMV2_DOMAIN: &str = "LAB.LOCAL";

/// Tính *salt* của Kerberos từ tên principal.
///
/// Salt chuẩn là phần realm viết hoa nối với từng thành phần tên viết hoa.
/// Ví dụ `HTTP/web01.lab.local@LAB.LOCAL` → `LAB.LOCALHTTPWEB01.LAB.LOCAL`.
pub fn kerberos_salt(principal: &str) -> String {
    let (name_part, realm) = match principal.split_once('@') {
        Some((name, realm)) => (name, realm),
        None => (principal, ""),
    };
    let mut salt = realm.to_uppercase();
    for component in name_part.split('/') {
        salt.push_str(&component.to_uppercase());
    }
    salt
}

/// Suy ra khoá RC4-HMAC (etype 23) từ mật khẩu và salt.
///
/// Lược đồ rút gọn: `key = MD5(MD5(password) || salt)` — hai lớp MD5 làm khoá "chậm" hơn
/// một lần băm trực tiếp, đồng thời phụ thuộc salt nên cùng một mật khẩu ở các realm khác
/// nhau sẽ sinh ra khoá khác nhau.
pub fn rc4_hmac_key(password: &str, salt: &str) -> [u8; 16] {
    let base = md5(password.as_bytes());
    let mut buffer = Vec::with_capacity(16 + salt.len());
    buffer.extend_from_slice(&base);
    buffer.extend_from_slice(salt.as_bytes());
    md5(&buffer)
}

/// Suy ra khoá AES-256-CTS (etype 18) từ mật khẩu và salt bằng PBKDF2-HMAC-SHA-256.
pub fn aes256_cts_key(password: &str, salt: &str, iterations: u32) -> [u8; 32] {
    let derived = pbkdf2_hmac_sha256(password.as_bytes(), salt.as_bytes(), iterations, 32);
    let mut key = [0u8; 32];
    key.copy_from_slice(&derived);
    key
}

/// Suy ra khoá AES-128-CTS (etype 17) từ mật khẩu và salt bằng PBKDF2-HMAC-SHA-256.
pub fn aes128_cts_key(password: &str, salt: &str, iterations: u32) -> [u8; 16] {
    let derived = pbkdf2_hmac_sha256(password.as_bytes(), salt.as_bytes(), iterations, 16);
    let mut key = [0u8; 16];
    key.copy_from_slice(&derived);
    key
}

/// Mã hoá RC4-HMAC theo lược đồ rút gọn của chương này.
///
/// Bố cục bản rõ: `confounder(16) || core || checksum(5)`, toàn bộ được XOR với dòng khoá RC4.
/// Checksum là 5 byte đầu của `HMAC-MD5(key, core)`.
///
/// Confounder ở đây là `MD5(key || core)` thay cho số ngẫu nhiên, nhằm giữ kết quả
/// **tất định và tái lập được** cho các bài kiểm thử; trong Kerberos thật confounder phải
/// là dữ liệu ngẫu nhiên an toàn.
pub fn rc4_hmac_encrypt(key: &[u8], core: &[u8]) -> Vec<u8> {
    let mut seed = Vec::with_capacity(key.len() + core.len());
    seed.extend_from_slice(key);
    seed.extend_from_slice(core);
    let confounder = md5(&seed);
    let checksum = hmac_md5(key, core);
    let mut plain =
        Vec::with_capacity(RC4_HMAC_CONFOUNDER_LEN + core.len() + RC4_HMAC_CHECKSUM_LEN);
    plain.extend_from_slice(&confounder);
    plain.extend_from_slice(core);
    plain.extend_from_slice(&checksum[..RC4_HMAC_CHECKSUM_LEN]);
    rc4(key, &plain)
}

/// Giải mã RC4-HMAC (etype 23) theo lược đồ rút gọn của chương này.
///
/// Bản rõ = `RC4(key) XOR ciphertext`; 5 byte cuối là checksum = 5 byte đầu của
/// `HMAC-MD5(key, core)`. Hàm trả về **phần core** (đã bỏ confounder và checksum),
/// hoặc vector rỗng nếu checksum sai — đây chính là "oracle" để nhận biết mật khẩu đúng.
pub fn rc4_hmac_decrypt(key: &[u8], ciphertext: &[u8], etype: Etype) -> Vec<u8> {
    if etype != Etype::Rc4Hmac {
        return Vec::new();
    }
    if ciphertext.len() < RC4_HMAC_CONFOUNDER_LEN + RC4_HMAC_CHECKSUM_LEN {
        return Vec::new();
    }
    let plain = rc4(key, ciphertext);
    let core_end = plain.len() - RC4_HMAC_CHECKSUM_LEN;
    if core_end <= RC4_HMAC_CONFOUNDER_LEN {
        return Vec::new();
    }
    let core = &plain[RC4_HMAC_CONFOUNDER_LEN..core_end];
    let checksum = &plain[core_end..];
    let expected = hmac_md5(key, core);
    if checksum != &expected[..RC4_HMAC_CHECKSUM_LEN] {
        return Vec::new();
    }
    core.to_vec()
}

/// Vé dịch vụ Kerberos đã bị mã hoá — nguyên liệu cho Kerberoast.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct ServiceTicket {
    /// Tên principal của dịch vụ, ví dụ `HTTP/web01.lab.local@LAB.LOCAL`.
    pub service_principal: String,
    /// Loại mã hoá dùng cho vé.
    pub etype: Etype,
    /// Khối đã mã hoá (ứng viên để bẻ khoá).
    pub cipher: Vec<u8>,
    /// Vé Kerberos gốc (dữ liệu minh hoạ, không mã hoá).
    pub krb5_ticket: Vec<u8>,
}

/// Bộ bẻ khoá vé dịch vụ — mô phỏng kỹ thuật Kerberoast.
#[derive(Debug, Clone)]
pub struct KerberoastCracker {
    /// Từ điển ứng viên.
    pub wordlist: Vec<String>,
}

impl KerberoastCracker {
    /// Khởi tạo bộ bẻ khoá với từ điển cho trước.
    pub fn new(wordlist: Vec<String>) -> Self {
        Self { wordlist }
    }

    /// Bẻ khoá một vé dịch vụ, tự chọn thuật toán theo etype của vé.
    pub fn crack(&self, ticket: &ServiceTicket) -> Option<String> {
        match ticket.etype {
            Etype::Rc4Hmac => self.crack_etype23(ticket),
            Etype::Aes256Cts | Etype::Aes128Cts => self.crack_aes(ticket),
            Etype::Unknown(_) => None,
        }
    }

    /// Bẻ khoá vé dịch vụ dùng RC4-HMAC (etype 23).
    pub fn crack_etype23(&self, ticket: &ServiceTicket) -> Option<String> {
        if ticket.etype != Etype::Rc4Hmac {
            return None;
        }
        let salt = kerberos_salt(&ticket.service_principal);
        for candidate in &self.wordlist {
            let key = rc4_hmac_key(candidate, &salt);
            let plain = rc4_hmac_decrypt(&key, &ticket.cipher, Etype::Rc4Hmac);
            if plain.starts_with(TICKET_PLAINTEXT_MAGIC) {
                return Some(candidate.clone());
            }
        }
        None
    }

    /// Bẻ khoá vé dịch vụ dùng AES-CTS (etype 17/18).
    pub fn crack_aes(&self, ticket: &ServiceTicket) -> Option<String> {
        let salt = kerberos_salt(&ticket.service_principal);
        for candidate in &self.wordlist {
            let plain = match ticket.etype {
                Etype::Aes256Cts => {
                    let key = aes256_cts_key(candidate, &salt, PBKDF2_ITERATIONS);
                    aes_cts_decrypt(&key, &ticket.cipher)
                }
                Etype::Aes128Cts => {
                    let key = aes128_cts_key(candidate, &salt, PBKDF2_ITERATIONS);
                    aes_cts_decrypt(&key, &ticket.cipher)
                }
                _ => return None,
            };
            if plain.starts_with(TICKET_PLAINTEXT_MAGIC) {
                return Some(candidate.clone());
            }
        }
        None
    }

    /// Số phép băm sẽ thực hiện nếu quét hết từ điển.
    pub fn attempts(&self) -> usize {
        self.wordlist.len()
    }
}

/// Bộ bẻ khoá AS-REP — mô phỏng kỹ thuật AS-REP Roasting.
#[derive(Debug, Clone)]
pub struct AsRepRoaster {
    /// Từ điển ứng viên.
    pub wordlist: Vec<String>,
}

impl AsRepRoaster {
    /// Khởi tạo bộ bẻ khoá với từ điển cho trước.
    pub fn new(wordlist: Vec<String>) -> Self {
        Self { wordlist }
    }

    /// Bẻ khoá một AS-REP: thử từng mật khẩu, giải mã phần enc-ticket-part và kiểm tra chữ ký.
    pub fn crack(&self, asrep: &AsRep) -> Option<String> {
        let etype = asrep.etype?;
        let blob = match asrep.extract_enc_ticket_part() {
            Some(item) if !item.value.is_empty() => item.value.clone(),
            _ => asrep.enc_part.clone(),
        };
        if blob.is_empty() {
            return None;
        }
        let salt = kerberos_salt(&format!(
            "{}@{}",
            asrep.principal.name, asrep.principal.realm
        ));
        for candidate in &self.wordlist {
            let plain = match etype {
                Etype::Rc4Hmac => {
                    let key = rc4_hmac_key(candidate, &salt);
                    rc4_hmac_decrypt(&key, &blob, Etype::Rc4Hmac)
                }
                Etype::Aes256Cts => {
                    let key = aes256_cts_key(candidate, &salt, PBKDF2_ITERATIONS);
                    aes_cts_decrypt(&key, &blob)
                }
                Etype::Aes128Cts => {
                    let key = aes128_cts_key(candidate, &salt, PBKDF2_ITERATIONS);
                    aes_cts_decrypt(&key, &blob)
                }
                Etype::Unknown(_) => return None,
            };
            if plain.starts_with(TICKET_PLAINTEXT_MAGIC) {
                return Some(candidate.clone());
            }
        }
        None
    }
}

/// Dựng một vé dịch vụ đã mã hoá (dữ liệu tổng hợp, tất định) để minh hoạ Kerberoast.
pub fn build_service_ticket(principal: &str, password: &str, etype: Etype) -> ServiceTicket {
    let salt = kerberos_salt(principal);
    let mut core = TICKET_PLAINTEXT_MAGIC.to_vec();
    core.extend_from_slice(b"|enc-part-ticket|");
    core.extend_from_slice(principal.as_bytes());
    let cipher = match etype {
        Etype::Rc4Hmac => {
            let key = rc4_hmac_key(password, &salt);
            rc4_hmac_encrypt(&key, &core)
        }
        Etype::Aes256Cts => {
            let key = aes256_cts_key(password, &salt, PBKDF2_ITERATIONS);
            aes_cts_encrypt_kerberos(&key, &core)
        }
        Etype::Aes128Cts => {
            let key = aes128_cts_key(password, &salt, PBKDF2_ITERATIONS);
            aes_cts_encrypt_kerberos(&key, &core)
        }
        Etype::Unknown(_) => Vec::new(),
    };
    let krb5_ticket = {
        let mut raw = vec![0x00, 0x00, 0x00, 0x00];
        raw.extend_from_slice(&(core.len() as u16).to_be_bytes());
        raw.extend_from_slice(&core);
        raw
    };
    ServiceTicket {
        service_principal: principal.to_string(),
        etype,
        cipher,
        krb5_ticket,
    }
}

/// Dựng một AS-REP tổng hợp (dữ liệu tất định) để minh hoạ AS-REP Roasting.
pub fn build_as_rep(principal: KrbPrincipal, password: &str, etype: Etype) -> AsRep {
    let salt = kerberos_salt(&format!("{}@{}", principal.name, principal.realm));
    let mut core = TICKET_PLAINTEXT_MAGIC.to_vec();
    core.extend_from_slice(b"|enc-kdc-rep-part|");
    core.extend_from_slice(principal.name.as_bytes());
    let blob = match etype {
        Etype::Rc4Hmac => {
            let key = rc4_hmac_key(password, &salt);
            rc4_hmac_encrypt(&key, &core)
        }
        Etype::Aes256Cts => {
            let key = aes256_cts_key(password, &salt, PBKDF2_ITERATIONS);
            aes_cts_encrypt_kerberos(&key, &core)
        }
        Etype::Aes128Cts => {
            let key = aes128_cts_key(password, &salt, PBKDF2_ITERATIONS);
            aes_cts_encrypt_kerberos(&key, &core)
        }
        Etype::Unknown(_) => Vec::new(),
    };
    let padatas = vec![
        PaData {
            padata_type: 2,
            etype: None,
            value: b"2026-01-01T00:00:00Z".to_vec(),
        },
        PaData {
            padata_type: 3,
            etype: Some(etype),
            value: blob.clone(),
        },
    ];
    AsRep {
        msg_type: KerberosMessageType::AsRep,
        principal,
        padatas,
        enc_part: blob,
        etype: Some(etype),
    }
}

// ============================================================================
// 11. BỘ BẺ KHOÁ TỪ ĐIỂN & ĐỘT BIẾN TỪ ĐIỂN
// ============================================================================

/// Thuật toán băm được hỗ trợ bởi bộ bẻ khoá.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum HashAlgorithm {
    /// MD5 (RFC 1321).
    Md5,
    /// NT Hash = MD4(UTF-16LE(password)).
    NtHash,
    /// SHA-256 (FIPS 180-4).
    Sha256,
    /// NTLMv2 Hash = HMAC-MD5(NT_Hash, UPPER(user) || UPPER(domain)).
    Ntlmv2,
    /// LM Hash rút gọn (xấp xỉ minh hoạ).
    Lm,
}

impl fmt::Display for HashAlgorithm {
    fn fmt(&self, f: &mut fmt::Formatter) -> fmt::Result {
        let name = match self {
            HashAlgorithm::Md5 => "md5",
            HashAlgorithm::NtHash => "nt-hash",
            HashAlgorithm::Sha256 => "sha256",
            HashAlgorithm::Ntlmv2 => "ntlmv2",
            HashAlgorithm::Lm => "lm",
        };
        f.write_str(name)
    }
}

/// Tính giá trị băm (dạng hex) của một ứng viên theo thuật toán đã chọn.
///
/// Với `HashAlgorithm::Ntlmv2`, hàm dùng danh tính minh hoạ cố định
/// (`DEMO_NTLMV2_USER` / `DEMO_NTLMV2_DOMAIN`) để bảo đảm tính tất định và tái lập được.
pub fn hash_for(algorithm: HashAlgorithm, candidate: &str) -> String {
    match algorithm {
        HashAlgorithm::Md5 => md5_hex(candidate.as_bytes()),
        HashAlgorithm::NtHash => nt_hash_hex(candidate),
        HashAlgorithm::Sha256 => sha256_hex(candidate.as_bytes()),
        HashAlgorithm::Ntlmv2 => {
            let nt = nt_hash(candidate);
            let identity = format!("{DEMO_NTLMV2_USER}{DEMO_NTLMV2_DOMAIN}").to_uppercase();
            to_hex(&hmac_md5(&nt, identity.as_bytes()))
        }
        HashAlgorithm::Lm => to_hex(&lm_hash(candidate)),
    }
}

/// Một phiên bẻ khoá: thuật toán + từ điển + giá trị băm cần tìm.
#[derive(Debug, Clone)]
pub struct CrackEngine {
    /// Thuật toán băm đang dùng.
    pub algorithm: HashAlgorithm,
    /// Từ điển ứng viên.
    pub wordlist: Vec<String>,
    /// Giá trị băm cần tìm (dạng hex).
    pub target_hash: String,
}

impl CrackEngine {
    /// Khởi tạo một phiên bẻ khoá.
    pub fn new(algorithm: HashAlgorithm, wordlist: Vec<String>, target_hash: String) -> Self {
        Self {
            algorithm,
            wordlist,
            target_hash,
        }
    }

    /// Quét từ điển và trả về mật khẩu tìm được (nếu có).
    pub fn crack(&self) -> Option<String> {
        let target = self.target_hash.trim().to_ascii_lowercase();
        self.wordlist
            .iter()
            .find(|candidate| hash_for(self.algorithm, candidate).to_ascii_lowercase() == target)
            .cloned()
    }

    /// Số phép băm sẽ thực hiện nếu quét hết từ điển.
    pub fn attempts(&self) -> usize {
        self.wordlist.len()
    }
}

/// Bộ gom nhiều phiên bẻ khoá lại với nhau.
#[derive(Debug, Clone)]
pub struct HashCracker {
    /// Danh sách các phiên bẻ khoá.
    pub engines: Vec<CrackEngine>,
}

impl Default for HashCracker {
    fn default() -> Self {
        Self::new()
    }
}

impl HashCracker {
    /// Khởi tạo bộ bẻ khoá rỗng.
    pub fn new() -> Self {
        Self {
            engines: Vec::new(),
        }
    }

    /// Thêm một phiên bẻ khoá vào bộ.
    pub fn add(&mut self, engine: CrackEngine) {
        self.engines.push(engine);
    }

    /// Chạy toàn bộ phiên và trả về kết quả theo thứ tự đã thêm.
    pub fn crack_all(&self) -> Vec<(HashAlgorithm, Option<String>)> {
        self.engines
            .iter()
            .map(|engine| (engine.algorithm, engine.crack()))
            .collect()
    }
}

/// Quy tắc đột biến từ điển.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum MutationRule {
    /// Viết hoa chữ cái đầu.
    UppercaseFirst,
    /// Viết hoa chữ cái đầu, còn lại viết thường.
    Capitalize,
    /// Thay ký tự bằng ký tự giống hình: a→@, e→3, i→1, o→0, s→$.
    Leet,
    /// Thêm hậu tố số.
    AppendDigits,
    /// Thêm hậu tố ký tự đặc biệt.
    AppendSpecial,
    /// Đảo ngược chuỗi.
    Reverse,
    /// Lặp lại chuỗi hai lần.
    Duplicate,
}

/// Bộ sinh biến thể từ một từ điển.
#[derive(Debug, Clone)]
pub struct WordlistMutator {
    /// Danh sách quy tắc áp dụng theo thứ tự.
    pub rules: Vec<MutationRule>,
    /// Số lớn nhất khi thêm hậu tố số.
    pub max_digits: u32,
    /// Các ký tự đặc biệt dùng cho hậu tố.
    pub specials: Vec<char>,
}

impl Default for WordlistMutator {
    fn default() -> Self {
        Self::new()
    }
}

impl WordlistMutator {
    /// Khởi tạo bộ đột biến với quy tắc mặc định.
    pub fn new() -> Self {
        Self {
            rules: vec![
                MutationRule::UppercaseFirst,
                MutationRule::Leet,
                MutationRule::AppendDigits,
                MutationRule::AppendSpecial,
            ],
            max_digits: 4,
            specials: vec!['!', '@', '#'],
        }
    }

    /// Sinh toàn bộ biến thể của một từ (gồm chính từ đó, luôn ở vị trí đầu).
    pub fn generate(&self, word: &str) -> Vec<String> {
        let mut output = vec![word.to_string()];
        for rule in &self.rules {
            let variants = match rule {
                MutationRule::UppercaseFirst => vec![uppercase_first(word)],
                MutationRule::Capitalize => vec![capitalize(word)],
                MutationRule::Leet => vec![WordlistMutator::leet(word)],
                MutationRule::AppendDigits => WordlistMutator::append_digits(word, self.max_digits),
                MutationRule::AppendSpecial => {
                    WordlistMutator::append_specials(word, &self.specials)
                }
                MutationRule::Reverse => vec![word.chars().rev().collect()],
                MutationRule::Duplicate => vec![format!("{word}{word}")],
            };
            for variant in variants {
                if !output.contains(&variant) {
                    output.push(variant);
                }
            }
        }
        output
    }

    /// Sinh biến thể cho cả từ điển, loại bỏ trùng lặp, giữ thứ tự xuất hiện.
    pub fn generate_all(&self, words: &[String]) -> Vec<String> {
        let mut output: Vec<String> = Vec::new();
        for word in words {
            for variant in self.generate(word) {
                if !output.contains(&variant) {
                    output.push(variant);
                }
            }
        }
        output
    }

    /// Tạo iterator **lười biến** sinh biến thể — không dựng sẵn toàn bộ danh sách.
    pub fn iter_mutations<'a>(&'a self, words: &'a [String]) -> MutationIter<'a> {
        MutationIter {
            mutator: self,
            words: words.iter(),
            pending: Vec::new().into_iter(),
            seen: HashSet::new(),
        }
    }

    /// Biến ký tự theo phong cách "leet".
    pub fn leet(word: &str) -> String {
        let mut output = String::with_capacity(word.len());
        for ch in word.chars() {
            let replaced = match ch {
                'a' => '@',
                'e' => '3',
                'i' => '1',
                'o' => '0',
                's' => '$',
                other => other,
            };
            output.push(replaced);
        }
        output
    }

    /// Sinh các biến thể thêm hậu tố số: `word1`, `word2`, …, `word{max}`.
    pub fn append_digits(word: &str, max: u32) -> Vec<String> {
        (1..=max).map(|n| format!("{word}{n}")).collect()
    }

    /// Sinh các biến thể thêm hậu tố ký tự đặc biệt.
    pub fn append_specials(word: &str, specials: &[char]) -> Vec<String> {
        specials.iter().map(|ch| format!("{word}{ch}")).collect()
    }

    /// Thêm hậu tố `123` — biến thể phổ biến nhất trong các cuộc tấn công mật khẩu.
    pub fn with_digits(word: &str) -> String {
        format!("{word}123")
    }
}

/// Iterator lười biến sinh biến thể từ điển.
#[derive(Debug)]
pub struct MutationIter<'a> {
    /// Bộ đột biến sở hữu iterator.
    pub mutator: &'a WordlistMutator,
    /// Iterator trên các từ gốc.
    pub words: std::slice::Iter<'a, String>,
    /// Các biến thể đang chờ phát ra của từ hiện tại.
    pub pending: std::vec::IntoIter<String>,
    /// Tập các biến thể đã phát ra để loại trùng.
    pub seen: HashSet<String>,
}

impl Iterator for MutationIter<'_> {
    type Item = String;

    fn next(&mut self) -> Option<String> {
        loop {
            if let Some(variant) = self.pending.next() {
                if self.seen.insert(variant.clone()) {
                    return Some(variant);
                }
                continue;
            }
            let word = self.words.next()?;
            self.pending = self.mutator.generate(word).into_iter();
        }
    }
}

/// Viết hoa chữ cái đầu tiên của chuỗi.
fn uppercase_first(word: &str) -> String {
    let mut chars = word.chars();
    match chars.next() {
        Some(first) => first.to_uppercase().collect::<String>() + chars.as_str(),
        None => String::new(),
    }
}

/// Viết hoa chữ cái đầu, các chữ còn lại viết thường.
fn capitalize(word: &str) -> String {
    let mut chars = word.chars();
    match chars.next() {
        Some(first) => first.to_uppercase().collect::<String>() + &chars.as_str().to_lowercase(),
        None => String::new(),
    }
}

// ============================================================================
// 12. KHO THÔNG TIN TÍN DỤNG (CREDENTIAL DB)
// ============================================================================

/// Một khoá Kerberos của tài khoản.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct KerberosKey {
    /// Tên principal.
    pub principal: String,
    /// Vùng.
    pub realm: String,
    /// Số phiên bản khoá (kvno).
    pub kvno: u32,
    /// Loại mã hoá của khoá.
    pub enctype: Etype,
    /// Giá trị khoá thô.
    pub key: Vec<u8>,
}

/// Một tài khoản trong miền.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct Account {
    /// Tên đăng nhập.
    pub username: String,
    /// NT Hash của mật khẩu.
    pub nt_hash: [u8; 16],
    /// Mật khẩu (chỉ tồn tại trong dữ liệu tổng hợp của chương này).
    pub password: String,
    /// Các Service Principal Name gắn với tài khoản.
    pub spns: Vec<String>,
    /// Các khoá Kerberos của tài khoản.
    pub kerberos_keys: Vec<KerberosKey>,
    /// Tài khoản có phải tài khoản dịch vụ hay không.
    pub is_service_account: bool,
}

/// Kho thông tin tín dụng của một miền — dữ liệu **tổng hợp, tất định**.
#[derive(Debug, Clone)]
pub struct CredentialDb {
    /// Danh sách tài khoản.
    pub accounts: Vec<Account>,
    /// Tên miền.
    pub domain: String,
}

impl CredentialDb {
    /// Khởi tạo kho tín dụng cho một miền.
    pub fn new(domain: &str) -> Self {
        Self {
            accounts: Vec::new(),
            domain: domain.to_string(),
        }
    }

    /// Dựng kho tín dụng minh hoạ với vài tài khoản tổng hợp.
    pub fn demo() -> Self {
        let domain = "LAB.LOCAL".to_string();
        let mut db = CredentialDb::new(&domain);
        let service_principal = format!("HTTP/web01.lab.local@{domain}");
        let sql_principal = format!("MSSQLSvc/db01.lab.local@{domain}");
        db.add_account(Account {
            username: "alice".to_string(),
            nt_hash: nt_hash("P@ssw0rd!"),
            password: "P@ssw0rd!".to_string(),
            spns: vec![],
            kerberos_keys: vec![],
            is_service_account: false,
        });
        db.add_account(Account {
            username: "svc_sql".to_string(),
            nt_hash: nt_hash("SqlSvc2026!"),
            password: "SqlSvc2026!".to_string(),
            spns: vec![sql_principal.clone()],
            kerberos_keys: vec![KerberosKey {
                principal: sql_principal,
                realm: domain.clone(),
                kvno: 3,
                enctype: Etype::Aes256Cts,
                key: aes256_cts_key("SqlSvc2026!", "LAB.LOCALSVC_SQL", 64).to_vec(),
            }],
            is_service_account: true,
        });
        db.add_account(Account {
            username: "svc_backup".to_string(),
            nt_hash: nt_hash("Backup#2026"),
            password: "Backup#2026".to_string(),
            spns: vec![service_principal.clone()],
            kerberos_keys: vec![KerberosKey {
                principal: service_principal,
                realm: domain.clone(),
                kvno: 5,
                enctype: Etype::Rc4Hmac,
                key: rc4_hmac_key("Backup#2026", "LAB.LOCALSVC_BACKUP").to_vec(),
            }],
            is_service_account: true,
        });
        db
    }

    /// Thêm một tài khoản vào kho.
    pub fn add_account(&mut self, account: Account) {
        self.accounts.push(account);
    }

    /// Tìm tài khoản theo tên đăng nhập.
    pub fn find_by_username(&self, username: &str) -> Option<&Account> {
        self.accounts
            .iter()
            .find(|account| account.username.eq_ignore_ascii_case(username))
    }

    /// Tìm tài khoản theo Service Principal Name.
    pub fn find_by_spn(&self, spn: &str) -> Option<&Account> {
        self.accounts
            .iter()
            .find(|account| account.spns.iter().any(|candidate| candidate == spn))
    }

    /// Tổng số tài khoản trong kho.
    pub fn total_accounts(&self) -> usize {
        self.accounts.len()
    }
}

// ============================================================================
// 13. CHẠY MINH HOẠ
// ============================================================================

/// In một tiêu đề phân đoạn trong báo cáo.
fn section(title: &str) {
    println!();
    println!("--- {title} ---");
}

fn main() {
    println!("================================================================");
    println!("  CHƯƠNG 101: TẤN CÔNG THÔNG TIN XÁC THỰC & KERBEROS");
    println!("  Bẻ khoá Băm · Giao thức Kerberos · RC4-HMAC/AES · Đột biến Từ điển");
    println!("================================================================");

    section("1. KIỂM CHỨNG VECTOR CHUẨN");
    println!(
        "MD5(\"\")                                  = {}",
        md5_hex(b"")
    );
    println!(
        "MD5(\"abc\")                                = {}",
        md5_hex(b"abc")
    );
    println!(
        "MD5(\"The quick brown fox ...\")            = {}",
        md5_hex(b"The quick brown fox jumps over the lazy dog")
    );
    println!(
        "MD4(\"\")                                  = {}",
        md4_hex(b"")
    );
    println!(
        "MD4(\"abc\")                                = {}",
        md4_hex(b"abc")
    );
    println!(
        "SHA-256(\"abc\")                            = {}",
        sha256_hex(b"abc")
    );
    println!(
        "RC4(khoá=\"Key\", rõ=\"Plaintext\")           = {}",
        to_hex(&rc4(b"Key", b"Plaintext"))
    );
    let aes_key: Vec<u8> = (0u8..32).collect();
    let aes_plain = [
        0x00u8, 0x11, 0x22, 0x33, 0x44, 0x55, 0x66, 0x77, 0x88, 0x99, 0xaa, 0xbb, 0xcc, 0xdd, 0xee,
        0xff,
    ];
    println!(
        "AES-256(FIPS-197)                          = {}",
        to_hex(&aes_encrypt_block(&aes_key, &aes_plain))
    );

    section("2. BẺ KHOÁ BĂM TỪ TỪ ĐIỂN + ĐỘT BIẾN");
    let wordlist = vec![
        "admin".to_string(),
        "password".to_string(),
        "letmein".to_string(),
        "welcome".to_string(),
        "changeme".to_string(),
        "alice".to_string(),
        "P@ssw0rd".to_string(),
        "Summer2026".to_string(),
        "P@ssw0rd!".to_string(),
        "Backup#2026".to_string(),
        "SqlSvc2026!".to_string(),
    ];
    let target = md5_hex(b"Summer2026");
    let mutator = WordlistMutator::new();
    let mutated = mutator.generate_all(&wordlist);
    println!("Từ điển gốc            : {} mật khẩu", wordlist.len());
    println!("Sau đột biến           : {} mật khẩu", mutated.len());
    println!("Mục tiêu MD5           : {target}");
    let engine = CrackEngine::new(HashAlgorithm::Md5, mutated.clone(), target.clone());
    println!("Số phép băm dự kiến    : {}", engine.attempts());
    match engine.crack() {
        Some(found) => println!("=> ĐÃ BẺ KHOÁ: \"{found}\""),
        None => println!("=> Không tìm thấy mật khẩu"),
    }

    section("3. NT HASH & NTLMv2 RESPONSE");
    let responder = NetNtlmV2Responder::new("alice", "LAB.LOCAL", "P@ssw0rd!");
    let server_challenge = [0x11u8, 0x22, 0x33, 0x44, 0x55, 0x66, 0x77, 0x88];
    let client_challenge = [0x99u8, 0xaa, 0xbb, 0xcc, 0xdd, 0xee, 0xff, 0x00];
    let timestamp = 1_767_225_600i64;
    println!("NT Hash của \"P@ssw0rd!\"   = {}", nt_hash_hex("P@ssw0rd!"));
    println!(
        "NTLMv2 Hash             = {}",
        to_hex(&responder.ntlmv2_hash("ALICE", "LAB.LOCAL"))
    );
    println!(
        "NTProofStr (viết hoa)   = {}",
        to_hex(&responder.compute_response_upper(&server_challenge, &client_challenge, timestamp))
    );
    println!(
        "NTProofStr (giữ hoa/thường) = {}",
        to_hex(&responder.compute_response(&server_challenge, &client_challenge, timestamp))
    );

    section("4. GIAO THỨC KERBEROS — MÃ HOÁ/ GIẢI MÃ ASN.1");
    let principal = KrbPrincipal::user("alice", "LAB.LOCAL");
    let as_req = AsReq::build(principal.clone(), "LAB.LOCAL", Etype::Aes256Cts);
    let as_req_bytes = as_req.to_asn1();
    println!("AS-REQ  : {} byte", as_req_bytes.len());
    println!("  hex   : {}", to_hex(&as_req_bytes));
    match AsReq::parse(&as_req_bytes) {
        Ok(parsed) => println!(
            "  giải mã: msg-type={:?} cname={} realm={} nonce=0x{:08x} padata[0]={:?}",
            parsed.msg_type,
            parsed.principal.name,
            parsed.realm,
            parsed.nonce,
            parsed.padatas.first().map(|item| item.etype)
        ),
        Err(error) => println!("  lỗi giải mã: {error}"),
    }

    section("5. PHÂN TÍCH AS-REP VÀ AS-REP ROASTING");
    let as_rep = build_as_rep(principal.clone(), "P@ssw0rd!", Etype::Rc4Hmac);
    println!(
        "AS-REP  : {} byte, etype = {}",
        as_rep.enc_part.len(),
        as_rep.etype.map(|e| e.to_string()).unwrap_or_default()
    );
    if let Some(timestamp) = as_rep.extract_pa_enc_timestamp() {
        println!(
            "  PA-ENC-TIMESTAMP : {}",
            String::from_utf8_lossy(&timestamp.value)
        );
    }
    if let Some(ticket_part) = as_rep.extract_enc_ticket_part() {
        println!(
            "  PA-ENC-TICKET-PART: {} byte đã mã hoá, etype = {:?}",
            ticket_part.value.len(),
            ticket_part.etype
        );
    }
    let roaster = AsRepRoaster::new(wordlist.clone());
    match roaster.crack(&as_rep) {
        Some(found) => println!("=> ĐÃ BẺ KHOÁ AS-REP: \"{found}\""),
        None => println!("=> Không tìm thấy mật khẩu trong AS-REP"),
    }

    section("6. KERBEROAST — BẺ KHOÁ DỊCH VỤ");
    let rc4_ticket = build_service_ticket(
        "HTTP/web01.lab.local@LAB.LOCAL",
        "Backup#2026",
        Etype::Rc4Hmac,
    );
    let aes_ticket = build_service_ticket(
        "MSSQLSvc/db01.lab.local@LAB.LOCAL",
        "SqlSvc2026!",
        Etype::Aes256Cts,
    );
    let cracker = KerberoastCracker::new(wordlist.clone());
    println!(
        "Vé etype 23 : salt = {}",
        kerberos_salt(&rc4_ticket.service_principal)
    );
    match cracker.crack(&rc4_ticket) {
        Some(found) => println!("=> ĐÃ BẺ KHOÁ DỊCH VỤ (RC4-HMAC): \"{found}\""),
        None => println!("=> Không tìm thấy khoá dịch vụ"),
    }
    println!(
        "Vé etype 18 : salt = {}",
        kerberos_salt(&aes_ticket.service_principal)
    );
    match cracker.crack(&aes_ticket) {
        Some(found) => println!("=> ĐÃ BẺ KHOÁ DỊCH VỤ (AES-256-CTS): \"{found}\""),
        None => println!("=> Không tìm thấy khoá dịch vụ"),
    }

    section("7. ĐỘT BIẾN TỪ ĐIỂN");
    println!("Quy tắc mặc định: {:?}", mutator.rules);
    let sample = mutator.generate("password");
    println!("Biến thể của \"password\" ({} mục):", sample.len());
    println!("  {}", sample.join(", "));
    println!(
        "Leet(\"secret\") = {} | with_digits(\"admin\") = {}",
        WordlistMutator::leet("secret"),
        WordlistMutator::with_digits("admin")
    );

    section("8. KHO THÔNG TIN TÍN DỤNG");
    let db = CredentialDb::demo();
    println!(
        "Miền: {} — tổng số tài khoản: {}",
        db.domain,
        db.total_accounts()
    );
    if let Some(account) = db.find_by_username("svc_sql") {
        println!(
            "  svc_sql là tài khoản dịch vụ: {}",
            account.is_service_account
        );
        println!("  SPN: {}", account.spns.join(", "));
        if let Some(key) = account.kerberos_keys.first() {
            println!(
                "  Khoá Kerberos: kvno={} enctype={} salt={}",
                key.kvno,
                key.enctype,
                kerberos_salt(&key.principal)
            );
        }
    }
    if let Some(account) = db.find_by_spn("HTTP/web01.lab.local@LAB.LOCAL") {
        println!(
            "  SPN HTTP/web01.lab.local@LAB.LOCAL -> tài khoản {}",
            account.username
        );
    }

    println!();
    println!("================================================================");
    println!("  KẾT THÚC — mọi dữ liệu đều là dữ liệu TỔNG HỢP, TẤT ĐỊNH.");
    println!("================================================================");
}

// ============================================================================
// 14. KIỂM THỬ TỰ ĐỘNG — 50 BÀI
// ============================================================================

#[cfg(test)]
mod tests {
    use super::*;

    /// Từ điển dùng chung cho các bài kiểm thử bẻ khoá.
    fn demo_wordlist() -> Vec<String> {
        vec![
            "admin".to_string(),
            "password".to_string(),
            "letmein".to_string(),
            "welcome".to_string(),
            "changeme".to_string(),
            "alice".to_string(),
            "P@ssw0rd".to_string(),
            "Summer2026".to_string(),
        ]
    }

    #[test]
    fn test_01_md5_empty_string_vector() {
        assert_eq!(md5_hex(b""), "d41d8cd98f00b204e9800998ecf8427e");
    }

    #[test]
    fn test_02_md5_abc_vector() {
        assert_eq!(md5_hex(b"abc"), "900150983cd24fb0d6963f7d28e17f72");
    }

    #[test]
    fn test_03_md5_quick_brown_fox_vector() {
        assert_eq!(
            md5_hex(b"The quick brown fox jumps over the lazy dog"),
            "9e107d9d372bb6826bd81d3542a419d6"
        );
    }

    #[test]
    fn test_04_md5_streaming_equals_one_shot() {
        let data: Vec<u8> = (0u8..200).collect();
        let mut hasher = Md5Hasher::new();
        for chunk in data.chunks(7) {
            hasher.update(chunk);
        }
        assert_eq!(hasher.finalize(), md5(&data));
    }

    #[test]
    fn test_05_md4_empty_string_vector() {
        assert_eq!(md4_hex(b""), "31d6cfe0d16ae931b73c59d7e0c089c0");
    }

    #[test]
    fn test_06_md4_abc_vector() {
        assert_eq!(md4_hex(b"abc"), "a448017aaf21d8525fc10ae87aa6729d");
    }

    #[test]
    fn test_07_md4_rfc1320_test_suite() {
        assert_eq!(md4_hex(b"a"), "bde52cb31de33e46245e05fbdbd6fb24");
        assert_eq!(
            md4_hex(b"message digest"),
            "d9130a8164549fe818874806e1c7014b"
        );
        assert_eq!(
            md4_hex(b"abcdefghijklmnopqrstuvwxyz"),
            "d79e1c308aa5bbcdeea8ed63df412da9"
        );
        assert_eq!(
            md4_hex(b"ABCDEFGHIJKLMNOPQRSTUVWXYZabcdefghijklmnopqrstuvwxyz0123456789"),
            "043f8582f241db351ce627e153e7f0e4"
        );
    }

    #[test]
    fn test_08_hmac_md5_rfc2202_case_1() {
        let key = [0x0bu8; 16];
        assert_eq!(
            to_hex(&hmac_md5(&key, b"Hi There")),
            "9294727a3638bb1c13f48ef8158bfc9d"
        );
    }

    #[test]
    fn test_09_hmac_md5_rfc2202_case_2() {
        assert_eq!(
            to_hex(&hmac_md5(b"Jefe", b"what do ya want for nothing?")),
            "750c783e6ab0b503eaa86e310a5db738"
        );
    }

    #[test]
    fn test_10_nt_hash_of_password_vector() {
        assert_eq!(nt_hash_hex("password"), "8846f7eaee8fb117ad06bdd830b7586c");
    }

    #[test]
    fn test_11_ntlmv2_hash_matches_specification() {
        let responder = NetNtlmV2Responder::new("alice", "LAB.LOCAL", "P@ssw0rd!");
        let expected = hmac_md5(&responder.nt_hash(), b"ALICELAB.LOCAL");
        assert_eq!(responder.ntlmv2_hash("ALICE", "LAB.LOCAL"), expected);
    }

    #[test]
    fn test_12_ntlmv2_response_upper_differs_from_mixed_case() {
        let responder = NetNtlmV2Responder::new("alice", "LAB.LOCAL", "P@ssw0rd!");
        let server = [0x11u8, 0x22, 0x33, 0x44, 0x55, 0x66, 0x77, 0x88];
        let client = [0x99u8, 0xaa, 0xbb, 0xcc, 0xdd, 0xee, 0xff, 0x00];
        let upper = responder.compute_response_upper(&server, &client, 1_767_225_600);
        let mixed = responder.compute_response(&server, &client, 1_767_225_600);
        assert_ne!(upper, mixed);
        assert_eq!(upper.len(), 16);
    }

    #[test]
    fn test_13_lm_hash_is_sixteen_bytes_and_deterministic() {
        let first = lm_hash("P@ssw0rd");
        let second = lm_hash("P@ssw0rd");
        assert_eq!(first, second);
        assert_eq!(first.len(), 16);
        assert_ne!(first, lm_hash("KhacMatKhau"));
    }

    #[test]
    fn test_14_sha256_abc_vector() {
        assert_eq!(
            sha256_hex(b"abc"),
            "ba7816bf8f01cfea414140de5dae2223b00361a396177a9cb410ff61f20015ad"
        );
    }

    #[test]
    fn test_15_sha256_empty_string_vector() {
        assert_eq!(
            sha256_hex(b""),
            "e3b0c44298fc1c149afbf4c8996fb92427ae41e4649b934ca495991b7852b855"
        );
    }

    #[test]
    fn test_16_sha256_long_message_vector() {
        assert_eq!(
            sha256_hex(b"abcdbcdecdefdefgefghfghighijhijkijkljklmklmnlmnomnopnopq"),
            "248d6a61d20638b8e5c026930c3e6039a33ce45964ff2167f6ecedd419db06c1"
        );
    }

    #[test]
    fn test_17_hmac_sha256_rfc4231_case_1() {
        let key = [0x0bu8; 20];
        assert_eq!(
            to_hex(&hmac_sha256(&key, b"Hi There")),
            "b0344c61d8db38535ca8afceaf0bf12b881dc200c9833da726e9376c2e32cff7"
        );
    }

    #[test]
    fn test_18_pbkdf2_hmac_sha256_vector() {
        assert_eq!(
            to_hex(&pbkdf2_hmac_sha256(b"password", b"salt", 1, 32)),
            "120fb6cffcf8b32c43e7225256c4f837a86548c92ccc35480805987cb70be17b"
        );
        assert_eq!(
            to_hex(&pbkdf2_hmac_sha256(b"password", b"salt", 2, 32)),
            "ae4d0c95af6b46d32d0adff928f06dd02a303f8ef3c251dfd6e2d85a95474c43"
        );
    }

    #[test]
    fn test_19_rc4_key_plaintext_vector() {
        assert_eq!(to_hex(&rc4(b"Key", b"Plaintext")), "bbf316e8d940af0ad3");
    }

    #[test]
    fn test_20_rc4_wiki_pedia_vector() {
        assert_eq!(to_hex(&rc4(b"Wiki", b"pedia")), "1021bf0420");
    }

    #[test]
    fn test_21_rc4_encrypt_decrypt_is_symmetric() {
        let mut stream = Rc4::new(b"khoa-bi-mat");
        let cipher = stream.apply(b"thong-diep-kiem-thu");
        let mut inverse = Rc4::new(b"khoa-bi-mat");
        assert_eq!(inverse.apply(&cipher), b"thong-diep-kiem-thu".to_vec());
    }

    #[test]
    fn test_22_crack_engine_finds_md5_password() {
        let wordlist = demo_wordlist();
        let target = md5_hex(b"Summer2026");
        let engine = CrackEngine::new(HashAlgorithm::Md5, wordlist, target);
        assert_eq!(engine.crack(), Some("Summer2026".to_string()));
        assert_eq!(engine.attempts(), 8);
    }

    #[test]
    fn test_23_crack_engine_returns_none_when_password_absent() {
        let wordlist = demo_wordlist();
        let target = md5_hex(b"khong-ton-tai-trong-tu-dien");
        let engine = CrackEngine::new(HashAlgorithm::Md5, wordlist, target);
        assert_eq!(engine.crack(), None);
    }

    #[test]
    fn test_24_hash_for_produces_expected_digest_lengths() {
        assert_eq!(hash_for(HashAlgorithm::Md5, "x").len(), 32);
        assert_eq!(hash_for(HashAlgorithm::NtHash, "x").len(), 32);
        assert_eq!(hash_for(HashAlgorithm::Sha256, "x").len(), 64);
        assert_eq!(hash_for(HashAlgorithm::Ntlmv2, "x").len(), 32);
        assert_eq!(hash_for(HashAlgorithm::Lm, "x").len(), 32);
    }

    #[test]
    fn test_25_hash_cracker_runs_every_engine() {
        let mut cracker = HashCracker::new();
        cracker.add(CrackEngine::new(
            HashAlgorithm::Md5,
            demo_wordlist(),
            md5_hex(b"alice"),
        ));
        cracker.add(CrackEngine::new(
            HashAlgorithm::Sha256,
            demo_wordlist(),
            sha256_hex(b"changeme"),
        ));
        let results = cracker.crack_all();
        assert_eq!(results.len(), 2);
        assert_eq!(results[0].0, HashAlgorithm::Md5);
        assert_eq!(results[0].1, Some("alice".to_string()));
        assert_eq!(results[1].1, Some("changeme".to_string()));
    }

    #[test]
    fn test_26_wordlist_mutator_default_rules() {
        let mutator = WordlistMutator::new();
        assert_eq!(
            mutator.rules,
            vec![
                MutationRule::UppercaseFirst,
                MutationRule::Leet,
                MutationRule::AppendDigits,
                MutationRule::AppendSpecial,
            ]
        );
        assert_eq!(mutator.max_digits, 4);
        assert_eq!(mutator.specials.len(), 3);
    }

    #[test]
    fn test_27_wordlist_mutator_leet_mapping() {
        assert_eq!(WordlistMutator::leet("password"), "p@$$w0rd");
        assert_eq!(WordlistMutator::leet("abc"), "@bc");
    }

    #[test]
    fn test_28_wordlist_mutator_append_digits_count() {
        let variants = WordlistMutator::append_digits("admin", 3);
        assert_eq!(variants.len(), 3);
        assert_eq!(variants, vec!["admin1", "admin2", "admin3"]);
    }

    #[test]
    fn test_29_wordlist_mutator_append_specials() {
        let variants = WordlistMutator::append_specials("admin", &['!', '@']);
        assert_eq!(variants, vec!["admin!", "admin@"]);
    }

    #[test]
    fn test_30_wordlist_mutator_with_digits_appends_123() {
        assert_eq!(WordlistMutator::with_digits("admin"), "admin123");
    }

    #[test]
    fn test_31_wordlist_mutator_generate_all_removes_duplicates() {
        let mutator = WordlistMutator::new();
        let words = vec!["admin".to_string(), "admin".to_string()];
        let all = mutator.generate_all(&words);
        let mut unique = all.clone();
        unique.sort();
        unique.dedup();
        assert_eq!(all.len(), unique.len());
        assert_eq!(all[0], "admin");
    }

    #[test]
    fn test_32_mutation_iterator_is_lazy_and_deduplicated() {
        let mutator = WordlistMutator::new();
        let words = demo_wordlist();
        let eager = mutator.generate_all(&words);
        let lazy: Vec<String> = mutator.iter_mutations(&words).collect();
        assert_eq!(eager, lazy);
    }

    #[test]
    fn test_33_asn1_integer_roundtrip() {
        for value in [0i64, 1, 127, 128, -1, -128, 65_535, 1_234_567] {
            let encoded = encode_integer(value);
            let (tag, _content, next) = decode_tlv(&encoded, 0).expect("giải mã INTEGER");
            assert_eq!(tag, 0x02);
            assert_eq!(next, encoded.len());
            assert_eq!(read_integer(&encoded).expect("đọc INTEGER"), value);
        }
    }

    #[test]
    fn test_34_asn1_string_encoders_roundtrip() {
        let octets = encode_octet_string(&[0x01, 0x02, 0xff]);
        let (tag, _content, _) = decode_tlv(&octets, 0).expect("giải mã OCTET STRING");
        assert_eq!(tag, 0x04);
        assert_eq!(
            read_octets(&octets).expect("đọc OCTET STRING"),
            vec![1, 2, 255]
        );

        let text = encode_general_string("LAB.LOCAL");
        let (tag, _content, _) = decode_tlv(&text, 0).expect("giải mã GeneralString");
        assert_eq!(tag, 0x1b);
        assert_eq!(read_string(&text).expect("đọc GeneralString"), "LAB.LOCAL");
    }

    #[test]
    fn test_35_asn1_sequence_and_context_roundtrip() {
        let sequence = encode_sequence(&[
            encode_integer(5),
            encode_general_string("abc"),
            encode_octet_string(b"xy"),
        ]);
        let (tag, content, next) = decode_tlv(&sequence, 0).expect("giải mã SEQUENCE");
        assert_eq!(tag, 0x30);
        assert_eq!(next, sequence.len());
        let children = parse_children(&content).expect("đọc phần tử con");
        assert_eq!(children.len(), 3);

        let context = encode_context(7, &[encode_integer(42)]);
        let (tag, content, _) = decode_tlv(&context, 0).expect("giải mã CONTEXT");
        assert_eq!(tag, 0xa7);
        assert_eq!(read_integer(&content).expect("đọc số trong CONTEXT"), 42);
    }

    #[test]
    fn test_36_asn1_decoder_rejects_truncated_input() {
        let encoded = encode_integer(1234);
        assert!(decode_tlv(&encoded[..encoded.len() - 1], 0).is_err());
        assert!(decode_tlv(&[], 0).is_err());
        assert!(decode_tlv(&encoded, encoded.len()).is_err());
    }

    #[test]
    fn test_37_as_req_asn1_roundtrip() {
        let request = AsReq::build(
            KrbPrincipal::user("alice", "LAB.LOCAL"),
            "LAB.LOCAL",
            Etype::Aes256Cts,
        );
        let bytes = request.to_asn1();
        let parsed = AsReq::parse(&bytes).expect("giải mã AS-REQ");
        assert_eq!(parsed.msg_type, KerberosMessageType::AsReq);
        assert_eq!(parsed.principal, request.principal);
        assert_eq!(parsed.realm, request.realm);
        assert_eq!(parsed.nonce, request.nonce);
        assert_eq!(parsed.padatas, request.padatas);
    }

    #[test]
    fn test_38_as_rep_asn1_roundtrip() {
        let reply = build_as_rep(
            KrbPrincipal::user("alice", "LAB.LOCAL"),
            "P@ssw0rd!",
            Etype::Rc4Hmac,
        );
        let bytes = reply.to_asn1();
        let parsed = AsRep::parse(&bytes).expect("giải mã AS-REP");
        assert_eq!(parsed.msg_type, KerberosMessageType::AsRep);
        assert_eq!(parsed.enc_part, reply.enc_part);
        assert_eq!(parsed.etype, reply.etype);
        assert_eq!(parsed.padatas.len(), reply.padatas.len());
    }

    #[test]
    fn test_39_tgs_req_asn1_roundtrip() {
        let request = TgsReq::build(
            KrbPrincipal::user("alice", "LAB.LOCAL"),
            KrbPrincipal::service("HTTP/web01.lab.local", "LAB.LOCAL"),
            vec![0xde, 0xad, 0xbe, 0xef],
        );
        let bytes = request.to_asn1();
        let parsed = TgsReq::parse(&bytes).expect("giải mã TGS-REQ");
        assert_eq!(parsed.msg_type, KerberosMessageType::TgsReq);
        assert_eq!(parsed.principal.name, "alice");
        assert_eq!(parsed.principal.realm, "LAB.LOCAL");
        assert_eq!(parsed.service_principal.name, "HTTP/web01.lab.local");
        assert_eq!(parsed.ticket, vec![0xde, 0xad, 0xbe, 0xef]);
    }

    #[test]
    fn test_40_tgs_rep_asn1_roundtrip() {
        let reply = TgsRep {
            msg_type: KerberosMessageType::TgsRep,
            principal: KrbPrincipal::user("alice", "LAB.LOCAL"),
            enc_part: vec![0x01, 0x02, 0x03, 0x04],
            etype: Some(Etype::Aes256Cts),
        };
        let bytes = reply.to_asn1();
        let parsed = TgsRep::parse(&bytes).expect("giải mã TGS-REP");
        assert_eq!(parsed.msg_type, KerberosMessageType::TgsRep);
        assert_eq!(parsed.enc_part, reply.enc_part);
        assert_eq!(parsed.etype, reply.etype);
        assert_eq!(parsed.principal.name, "alice");
        assert!(parsed.extract_enc_ticket_part().is_some());
    }

    #[test]
    fn test_41_as_rep_extracts_pa_data_elements() {
        let reply = build_as_rep(
            KrbPrincipal::user("alice", "LAB.LOCAL"),
            "P@ssw0rd!",
            Etype::Rc4Hmac,
        );
        let timestamp = reply
            .extract_pa_enc_timestamp()
            .expect("có PA-ENC-TIMESTAMP");
        assert_eq!(timestamp.padata_type, 2);
        let ticket_part = reply
            .extract_enc_ticket_part()
            .expect("có PA-ENC-TICKET-PART");
        assert_eq!(ticket_part.padata_type, 3);
        assert!(!ticket_part.value.is_empty());
    }

    #[test]
    fn test_42_aes256_fips197_block_vector() {
        let key: Vec<u8> = (0u8..32).collect();
        let plain = [
            0x00u8, 0x11, 0x22, 0x33, 0x44, 0x55, 0x66, 0x77, 0x88, 0x99, 0xaa, 0xbb, 0xcc, 0xdd,
            0xee, 0xff,
        ];
        let cipher = aes_encrypt_block(&key, &plain);
        assert_eq!(to_hex(&cipher), "8ea2b7ca516745bfeafc49904b496089");
    }

    #[test]
    fn test_43_aes256_block_decryption_is_inverse() {
        let key: Vec<u8> = (0u8..32).collect();
        let plain = [
            0x00u8, 0x11, 0x22, 0x33, 0x44, 0x55, 0x66, 0x77, 0x88, 0x99, 0xaa, 0xbb, 0xcc, 0xdd,
            0xee, 0xff,
        ];
        let cipher = aes_encrypt_block(&key, &plain);
        assert_eq!(aes_decrypt_block(&key, &cipher), plain);
    }

    #[test]
    fn test_44_aes_cts_roundtrip_on_full_blocks() {
        let key: Vec<u8> = (0u8..32).collect();
        let plain: Vec<u8> = (0u8..64).collect();
        let cipher = aes_cts_encrypt_raw(&key, &plain);
        assert_eq!(cipher.len(), plain.len());
        assert_eq!(aes_cts_decrypt_raw(&key, &cipher), plain);
    }

    #[test]
    fn test_45_aes_cts_roundtrip_on_partial_last_block() {
        let key: Vec<u8> = (0u8..32).collect();
        let plain: Vec<u8> = (0u8..43).collect();
        let cipher = aes_cts_encrypt_raw(&key, &plain);
        assert_eq!(cipher.len(), plain.len());
        assert_eq!(aes_cts_decrypt_raw(&key, &cipher), plain);

        // Vector chính thức của RFC 3962 (Phụ lục B): khoá "chicken teriyaki", IV toàn 0.
        let rfc_key = b"chicken teriyaki";
        let rfc_plain = b"I would like the ";
        let rfc_cipher = aes_cts_encrypt_raw(rfc_key, rfc_plain);
        assert_eq!(to_hex(&rfc_cipher), "c6353568f2bf8cb4d8a580362da7ff7f97");
        assert_eq!(
            aes_cts_decrypt_raw(rfc_key, &rfc_cipher),
            rfc_plain.to_vec()
        );
    }

    #[test]
    fn test_46_rc4_hmac_encrypt_decrypt_roundtrip() {
        let key = rc4_hmac_key("Backup#2026", "LAB.LOCALSVC_BACKUP");
        let core = b"noi-dung-can-bao-ve";
        let cipher = rc4_hmac_encrypt(&key, core);
        let plain = rc4_hmac_decrypt(&key, &cipher, Etype::Rc4Hmac);
        assert_eq!(plain, core.to_vec());
        let wrong = rc4_hmac_decrypt(&key, &cipher, Etype::Aes256Cts);
        assert!(wrong.is_empty());
    }

    #[test]
    fn test_47_kerberoast_cracks_etype23_service_ticket() {
        let ticket = build_service_ticket(
            "HTTP/web01.lab.local@LAB.LOCAL",
            "Backup#2026",
            Etype::Rc4Hmac,
        );
        let mut wordlist = demo_wordlist();
        wordlist.push("Backup#2026".to_string());
        let cracker = KerberoastCracker::new(wordlist.clone());
        assert_eq!(cracker.attempts(), wordlist.len());
        assert_eq!(cracker.crack(&ticket), Some("Backup#2026".to_string()));
        assert_eq!(
            cracker.crack_etype23(&ticket),
            Some("Backup#2026".to_string())
        );
    }

    #[test]
    fn test_48_kerberoast_cracks_aes256_service_ticket() {
        let ticket = build_service_ticket(
            "MSSQLSvc/db01.lab.local@LAB.LOCAL",
            "SqlSvc2026!",
            Etype::Aes256Cts,
        );
        let mut wordlist = demo_wordlist();
        wordlist.push("SqlSvc2026!".to_string());
        let cracker = KerberoastCracker::new(wordlist);
        assert_eq!(cracker.crack(&ticket), Some("SqlSvc2026!".to_string()));
    }

    #[test]
    fn test_49_as_rep_roaster_cracks_user_password() {
        let reply = build_as_rep(
            KrbPrincipal::user("alice", "LAB.LOCAL"),
            "Summer2026",
            Etype::Rc4Hmac,
        );
        let roaster = AsRepRoaster::new(demo_wordlist());
        assert_eq!(roaster.crack(&reply), Some("Summer2026".to_string()));
    }

    #[test]
    fn test_50_credential_db_lookup_by_username_and_spn() {
        let db = CredentialDb::demo();
        assert_eq!(db.domain, "LAB.LOCAL");
        assert_eq!(db.total_accounts(), 3);
        assert!(db.find_by_username("alice").is_some());
        assert!(db.find_by_username("khong-ton-tai").is_none());
        let account = db
            .find_by_spn("HTTP/web01.lab.local@LAB.LOCAL")
            .expect("tìm thấy theo SPN");
        assert_eq!(account.username, "svc_backup");
        assert!(account.is_service_account);
        assert_eq!(account.kerberos_keys.len(), 1);
        assert_eq!(account.kerberos_keys[0].enctype, Etype::Rc4Hmac);
    }
}
