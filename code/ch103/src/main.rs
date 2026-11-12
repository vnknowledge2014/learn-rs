//! # Chương 103: Xoay chiều Mạng & Khung Chỉ huy-Kiểm soát — Đường hầm SSH, Chuỗi SOCKS, C2 Mini & Kênh Mã hoá
//! # (Pivoting & C2: SSH Tunnels, SOCKS Relay, Mini C2 & Encrypted Channels)
//!
//! Crate `ch103` nằm trong **Chủ đề 28** (Mạng & Xâm nhập — Bảo mật Ứng dụng & Tấn công), và là
//! chương cầu nối giữa hai thế giới: **điều hướng lưu lượng** (pivoting) và **điều khiển từ xa**
//! (Command & Control). Toàn bộ crate viết bằng Rust 2024 thuần `std`, không một crate ngoài nào.
//!
//! Nội dung chính:
//! - `SshTunnelSimulator` + `ssh_command_line`: mô phỏng ba loại đường hầm SSH
//!   (`-L` local forward, `-R` remote forward, `-D` dynamic SOCKS) một cách tất định.
//! - `SocksRelayChain` + `simulate_chain`: chuỗi relay SOCKS nhiều bước và tổng độ trễ.
//! - `C2Server` / `C2Agent` / `MockShell`: khung chỉ huy-kiểm soát mini với đăng ký, beacon,
//!   hàng đợi nhiệm vụ và trả kết quả — chạy trên vỏ lệnh giả lập, không chạm hệ thống thật.
//! - `ChaCha20` (RFC 8439), `HChaCha20`, `XChaCha20Channel`: mã hoá kênh C2 tự cài từ đầu.
//! - `hkdf_sha256` (RFC 5869) dựa trên SHA-256 + HMAC-SHA256 tự cài: dẫn khóa phiên từ bí mật ECDH.
//! - `X25519` (RFC 7748): trao đổi khóa ECDH trên Montgomery ladder, kèm vector kiểm thử chuẩn.
//! - `TrafficObfuscator`: đóng gói lưu lượng C2 thành HTTP/DNS để né bộ dò quét heuristic.
//!
//! Mọi thao tác đều diễn ra trên dữ liệu tổng hợp tất định (bộ sinh `SplitMix64`, vỏ lệnh
//! `MockShell`), tuyệt đối không kết nối tới hệ thống thật ngoài loopback.
#![allow(dead_code)]

use std::collections::HashMap;
use std::fmt::Write as FmtWrite;
use std::net::SocketAddr;

// ============================================================================
// 1. BỘ SINH SỐ GIẢ NGẪU NHIÊN TẤT ĐỊNH SPLITMIX64
// ============================================================================

/// Bộ sinh số giả ngẫu nhiên 64-bit tất định `SplitMix64`.
///
/// Dùng chung cho toàn bộ crate: sinh khóa phiên, khóa X25519 và độ trễ beacon một cách
/// tái lập 100% trên mọi kiến trúc CPU.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct SplitMix64 {
    /// Trạng thái thanh ghi 64-bit nội bộ
    pub state: u64,
}

impl SplitMix64 {
    /// Khởi tạo bộ sinh số với hạt giống cố định
    pub const fn new(seed: u64) -> Self {
        Self { state: seed }
    }

    /// Sinh số nguyên không dấu 64-bit ngẫu nhiên tiếp theo
    pub fn next_u64(&mut self) -> u64 {
        self.state = self.state.wrapping_add(0x9E37_79B9_7F4A_7C15);
        let mut z = (self.state ^ (self.state >> 30)).wrapping_mul(0xBF58_476D_1CE4_E5B9);
        z = (z ^ (z >> 27)).wrapping_mul(0x94D0_49BB_1331_11EB);
        z ^ (z >> 31)
    }

    /// Sinh số thực dấu phẩy động trong nửa khoảng `[0.0, 1.0)`
    pub fn next_f64(&mut self) -> f64 {
        let u = (self.next_u64() >> 11) as f64;
        u * (1.0 / ((1u64 << 53) as f64))
    }

    /// Sinh số thực dấu phẩy động có dấu trong khoảng `[-scale, scale]`
    pub fn next_signed(&mut self, scale: f64) -> f64 {
        (self.next_f64() * 2.0 - 1.0) * scale
    }

    /// Sinh một mảng byte tất định có độ dài `len`
    pub fn fill_bytes(&mut self, len: usize) -> Vec<u8> {
        let mut out = Vec::with_capacity(len);
        while out.len() < len {
            let chunk = self.next_u64().to_le_bytes();
            let take = (len - out.len()).min(8);
            out.extend_from_slice(&chunk[..take]);
        }
        out
    }
}

// ============================================================================
// 2. HÀM BĂM SHA-256 & HMAC-SHA256 TỰ CÀI (FIPS 180-4 / RFC 2104)
// ============================================================================

/// Bảng hằng số tròn của SHA-256 (64 giá trị đầu tiên của phân rã nguyên phân của căn bậc ba)
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

/// Tính giá trị băm SHA-256 của một chuỗi byte.
///
/// Trả về đúng 32 byte theo thứ tự big-endian, tuân thủ FIPS 180-4.
pub fn sha256(data: &[u8]) -> [u8; 32] {
    let mut h: [u32; 8] = [
        0x6a09e667, 0xbb67ae85, 0x3c6ef372, 0xa54ff53a, 0x510e527f, 0x9b05688c, 0x1f83d9ab,
        0x5be0cd19,
    ];

    let bit_len = (data.len() as u64).wrapping_mul(8);
    let mut msg = data.to_vec();
    msg.push(0x80);
    while msg.len() % 64 != 56 {
        msg.push(0);
    }
    msg.extend_from_slice(&bit_len.to_be_bytes());

    for chunk in msg.chunks(64) {
        let mut w = [0u32; 64];
        for (i, word) in chunk.as_chunks::<4>().0.iter().enumerate() {
            w[i] = u32::from_be_bytes([word[0], word[1], word[2], word[3]]);
        }
        for i in 16..64 {
            let s0 = w[i - 15].rotate_right(7) ^ w[i - 15].rotate_right(18) ^ (w[i - 15] >> 3);
            let s1 = w[i - 2].rotate_right(17) ^ w[i - 2].rotate_right(19) ^ (w[i - 2] >> 10);
            w[i] = w[i - 16]
                .wrapping_add(s0)
                .wrapping_add(w[i - 7])
                .wrapping_add(s1);
        }

        let mut v = h;
        for i in 0..64 {
            let s1 = v[4].rotate_right(6) ^ v[4].rotate_right(11) ^ v[4].rotate_right(25);
            let ch = (v[4] & v[5]) ^ ((!v[4]) & v[6]);
            let t1 = v[7]
                .wrapping_add(s1)
                .wrapping_add(ch)
                .wrapping_add(SHA256_K[i])
                .wrapping_add(w[i]);
            let s0 = v[0].rotate_right(2) ^ v[0].rotate_right(13) ^ v[0].rotate_right(22);
            let maj = (v[0] & v[1]) ^ (v[0] & v[2]) ^ (v[1] & v[2]);
            let t2 = s0.wrapping_add(maj);

            v[7] = v[6];
            v[6] = v[5];
            v[5] = v[4];
            v[4] = v[3].wrapping_add(t1);
            v[3] = v[2];
            v[2] = v[1];
            v[1] = v[0];
            v[0] = t1.wrapping_add(t2);
        }

        for (dst, src) in h.iter_mut().zip(v.iter()) {
            *dst = dst.wrapping_add(*src);
        }
    }

    let mut out = [0u8; 32];
    for (i, word) in h.iter().enumerate() {
        out[4 * i..4 * i + 4].copy_from_slice(&word.to_be_bytes());
    }
    out
}

/// Tính mã xác thực HMAC-SHA256 theo RFC 2104.
///
/// `HMAC(K, m) = H((K' ^ opad) || H((K' ^ ipad) || m))` với khóa con `K'` được mở rộng
/// (hoặc băm lại nếu dài quá 64 byte) về đúng 64 byte.
pub fn hmac_sha256(key: &[u8], data: &[u8]) -> [u8; 32] {
    const BLOCK: usize = 64;

    let mut k = [0u8; BLOCK];
    if key.len() > BLOCK {
        k[..32].copy_from_slice(&sha256(key));
    } else {
        k[..key.len()].copy_from_slice(key);
    }

    let mut ipad = [0x36u8; BLOCK];
    let mut opad = [0x5cu8; BLOCK];
    for i in 0..BLOCK {
        ipad[i] ^= k[i];
        opad[i] ^= k[i];
    }

    let mut inner = Vec::with_capacity(BLOCK + data.len());
    inner.extend_from_slice(&ipad);
    inner.extend_from_slice(data);
    let inner_hash = sha256(&inner);

    let mut outer = Vec::with_capacity(BLOCK + 32);
    outer.extend_from_slice(&opad);
    outer.extend_from_slice(&inner_hash);
    sha256(&outer)
}

/// Mảng 32 byte zero dùng làm muối mặc định cho HKDF (salt rỗng).
const HKDF_ZERO_SALT: [u8; 32] = [0u8; 32];

/// Bước trích xuất PRK của HKDF (RFC 5869 §2.2): `PRK = HMAC-Hash(salt, IKM)`.
pub fn hkdf_extract(salt: &[u8], ikm: &[u8]) -> [u8; 32] {
    let effective_salt: &[u8] = if salt.is_empty() {
        &HKDF_ZERO_SALT
    } else {
        salt
    };
    hmac_sha256(effective_salt, ikm)
}

/// Bước mở rộng HKDF (RFC 5869 §2.3): sinh `len` byte khóa từ PRK và thông tin ngữ cảnh.
pub fn hkdf_expand(prk: &[u8; 32], info: &[u8], len: usize) -> Vec<u8> {
    let mut out: Vec<u8> = Vec::with_capacity(len);
    let mut t: Vec<u8> = Vec::new();
    let mut counter: u8 = 1;

    while out.len() < len {
        let mut input = Vec::with_capacity(t.len() + info.len() + 1);
        input.extend_from_slice(&t);
        input.extend_from_slice(info);
        input.push(counter);
        let block = hmac_sha256(prk, &input);
        t = block.to_vec();
        let take = (len - out.len()).min(32);
        out.extend_from_slice(&block[..take]);
        counter = counter.wrapping_add(1);
    }
    out
}

/// Dẫn khóa HKDF-SHA256 đầy đủ: `HKDF(ikm, salt, info, len)` theo RFC 5869.
pub fn hkdf_sha256(ikm: &[u8], salt: &[u8], info: &[u8], len: usize) -> Vec<u8> {
    let prk = hkdf_extract(salt, ikm);
    hkdf_expand(&prk, info, len)
}

// ============================================================================
// 3. MÃ HOÁ LUỒNG CHACHA20 (RFC 8439) & HCHACHA20
// ============================================================================

/// Giá trị khởi tạo của bộ đếm khối ChaCha20 (RFC 8439 §2.4.2 dùng "Initial Counter = 1").
pub const CHACHA_INITIAL_COUNTER: u32 = 1;

/// Bốn hằng số "expand 32-byte k" của ChaCha20 theo RFC 8439 §2.3.
const CHACHA_CONSTANTS: [u32; 4] = [0x6170_7865, 0x3320_646e, 0x7962_2d32, 0x6b20_6574];

/// Thực hiện một vòng phần tư (quarter round) của ChaCha20 trên trạng thái 16 từ 32-bit.
///
/// Biến thể `QR(a, b, c, d)` dùng các phép cộng/XOR/rotate đặc trưng của họ ARX.
pub fn quarter_round(state: &mut [u32; 16], a: usize, b: usize, c: usize, d: usize) {
    state[a] = state[a].wrapping_add(state[b]);
    state[d] = (state[d] ^ state[a]).rotate_left(16);
    state[c] = state[c].wrapping_add(state[d]);
    state[b] = (state[b] ^ state[c]).rotate_left(12);
    state[a] = state[a].wrapping_add(state[b]);
    state[d] = (state[d] ^ state[a]).rotate_left(8);
    state[c] = state[c].wrapping_add(state[d]);
    state[b] = (state[b] ^ state[c]).rotate_left(7);
}

/// Sinh một khối keystream 64 byte của ChaCha20 (RFC 8439 §2.3).
///
/// Trạng thái đầu vào gồm 4 hằng số, 8 từ khóa, 1 từ bộ đếm và 3 từ nonce; sau 20 vòng
/// (10 lần đôi vòng) cộng lại trạng thái ban đầu rồi xuất ra big-endian theo từng từ.
pub fn chacha20_block(key: &[u8; 32], counter: u32, nonce: &[u8; 12]) -> [u8; 64] {
    let mut state = [0u32; 16];
    state[0..4].copy_from_slice(&CHACHA_CONSTANTS);
    for (i, chunk) in key.as_chunks::<4>().0.iter().enumerate() {
        state[4 + i] = u32::from_le_bytes([chunk[0], chunk[1], chunk[2], chunk[3]]);
    }
    state[12] = counter;
    for (i, chunk) in nonce.as_chunks::<4>().0.iter().enumerate() {
        state[13 + i] = u32::from_le_bytes([chunk[0], chunk[1], chunk[2], chunk[3]]);
    }

    let mut working = state;
    for _ in 0..10 {
        quarter_round(&mut working, 0, 4, 8, 12);
        quarter_round(&mut working, 1, 5, 9, 13);
        quarter_round(&mut working, 2, 6, 10, 14);
        quarter_round(&mut working, 3, 7, 11, 15);
        quarter_round(&mut working, 0, 5, 10, 15);
        quarter_round(&mut working, 1, 6, 11, 12);
        quarter_round(&mut working, 2, 7, 8, 13);
        quarter_round(&mut working, 3, 4, 9, 14);
    }

    let mut out = [0u8; 64];
    for (i, word) in working.iter().enumerate() {
        let v = word.wrapping_add(state[i]);
        out[4 * i..4 * i + 4].copy_from_slice(&v.to_le_bytes());
    }
    out
}

/// Mã hoá/giải mã dữ liệu bằng ChaCha20 với nonce 12 byte (biến thể IETF của RFC 8439).
///
/// ChaCha20 là mã hoá dòng (stream cipher): `ciphertext = plaintext XOR keystream`, nên
/// phép giải mã giống hệt phép mã hoá.
pub fn chacha20(key: &[u8; 32], nonce: &[u8; 12], data: &[u8]) -> Vec<u8> {
    let mut out = Vec::with_capacity(data.len());
    for (block_index, chunk) in data.chunks(64).enumerate() {
        let keystream = chacha20_block(key, CHACHA_INITIAL_COUNTER + block_index as u32, nonce);
        for (i, &byte) in chunk.iter().enumerate() {
            out.push(byte ^ keystream[i]);
        }
    }
    out
}

/// Máy mã hoá ChaCha20 có trạng thái (khóa, nonce, bộ đếm khối).
///
/// Dùng cho các bài kiểm thử cần kiểm soát bộ đếm khối một cách tường minh.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct ChaCha20 {
    /// Khóa 256-bit
    pub key: [u8; 32],
    /// Nonce 96-bit
    pub nonce: [u8; 12],
    /// Bộ đếm khối 32-bit hiện tại
    pub counter: u32,
}

impl ChaCha20 {
    /// Khởi tạo máy mã hoá với khóa và nonce cho trước.
    ///
    /// Bộ đếm khối khởi tạo bằng `CHACHA_INITIAL_COUNTER` (1) theo quy ước "Initial Counter = 1"
    /// của RFC 8439 §2.4.2.
    pub fn new(key: [u8; 32], nonce: [u8; 12]) -> Self {
        Self {
            key,
            nonce,
            counter: CHACHA_INITIAL_COUNTER,
        }
    }

    /// Mã hoá (hoặc giải mã) một mảng dữ liệu, tự tăng bộ đếm khối sau mỗi khối 64 byte.
    pub fn apply(&mut self, data: &[u8]) -> Vec<u8> {
        let mut out = Vec::with_capacity(data.len());
        for chunk in data.chunks(64) {
            let keystream = chacha20_block(&self.key, self.counter, &self.nonce);
            self.counter = self.counter.wrapping_add(1);
            for (i, &byte) in chunk.iter().enumerate() {
                out.push(byte ^ keystream[i]);
            }
        }
        out
    }
}

/// Hàm băm HChaCha20 — phần lõi của XChaCha20 (draft-irtf-cfrg-xchacha).
///
/// HChaCha20 chạy 20 vòng ChaCha20 với trạng thái gồm hằng số, khóa và 16 byte nonce
/// (không có bộ đếm), rồi trả về 16 từ đầu và 4 từ cuối làm khóa phụ.
pub fn hchacha20(key: &[u8; 32], nonce: &[u8]) -> [u8; 32] {
    assert_eq!(nonce.len(), 16, "HChaCha20 yêu cầu nonce đúng 16 byte");
    let mut state = [0u32; 16];
    state[0..4].copy_from_slice(&CHACHA_CONSTANTS);
    for (i, chunk) in key.as_chunks::<4>().0.iter().enumerate() {
        state[4 + i] = u32::from_le_bytes([chunk[0], chunk[1], chunk[2], chunk[3]]);
    }
    for (i, chunk) in nonce.as_chunks::<4>().0.iter().enumerate() {
        state[12 + i] = u32::from_le_bytes([chunk[0], chunk[1], chunk[2], chunk[3]]);
    }

    let mut working = state;
    for _ in 0..10 {
        quarter_round(&mut working, 0, 4, 8, 12);
        quarter_round(&mut working, 1, 5, 9, 13);
        quarter_round(&mut working, 2, 6, 10, 14);
        quarter_round(&mut working, 3, 7, 11, 15);
        quarter_round(&mut working, 0, 5, 10, 15);
        quarter_round(&mut working, 1, 6, 11, 12);
        quarter_round(&mut working, 2, 7, 8, 13);
        quarter_round(&mut working, 3, 4, 9, 14);
    }

    let mut out = [0u8; 32];
    for (i, word) in working[0..4].iter().enumerate() {
        out[4 * i..4 * i + 4].copy_from_slice(&word.to_le_bytes());
    }
    for (i, word) in working[12..16].iter().enumerate() {
        out[16 + 4 * i..16 + 4 * i + 4].copy_from_slice(&word.to_le_bytes());
    }
    out
}

/// Độ dài nonce (byte) dùng trên đường truyền của kênh XChaCha20.
pub const XCHACHA_NONCE_LEN: usize = 12;

/// Số byte nonce mở rộng (24 byte) dùng để dẫn khóa phụ HChaCha20.
pub const XCHACHA_EXTENDED_NONCE_LEN: usize = 24;

/// Trait mô tả một kênh mã hoá hai chiều dùng chung cho khung C2.
///
/// `encrypt` nhận `&self` nên lớp triển khai phải tự dẫn nonce theo cách không cần biến đổi
/// trạng thái (ví dụ dẫn nonce từ nội dung bản rõ).
pub trait ChannelCipher {
    /// Mã hoá bản rõ, trả về bản mã đã kèm nonce theo định dạng đường truyền.
    fn encrypt(&self, plaintext: &[u8]) -> Vec<u8>;
    /// Giải mã bản mã (kèm nonce) trả về bản rõ.
    fn decrypt(&self, ciphertext: &[u8]) -> Vec<u8>;
    /// Độ dài nonce (byte) nằm ở đầu mỗi bản mã trên đường truyền.
    fn nonce_len(&self) -> usize;
}

/// Kênh mã hoá XChaCha20 dùng cho lưu lượng C2.
///
/// Định dạng đường truyền: `nonce (12 byte) || ciphertext`. Khóa phụ cho mỗi tin nhắn được
/// dẫn bằng HChaCha20 từ nonce mở rộng 24 byte sinh ra từ khóa chính và bộ đếm.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct XChaCha20Channel {
    /// Khóa chính 256-bit của kênh
    pub key: [u8; 32],
    /// Bộ đếm sinh nonce, tăng mỗi lần phát bản tin
    pub nonce_counter: u64,
}

impl XChaCha20Channel {
    /// Khởi tạo kênh với khóa chính 256-bit cho trước.
    pub fn new(key: [u8; 32]) -> Self {
        Self {
            key,
            nonce_counter: 1,
        }
    }

    /// Sinh nonce 12 byte kế tiếp theo kiểu XChaCha20 (4 byte 0 + bộ đếm 8 byte LE).
    pub fn next_nonce(&mut self) -> [u8; 12] {
        let mut nonce = [0u8; 12];
        nonce[4..12].copy_from_slice(&self.nonce_counter.to_le_bytes());
        self.nonce_counter = self.nonce_counter.wrapping_add(1);
        nonce
    }

    /// Bọc bản rõ thành bản mã kèm nonce (thao tác `seal`).
    pub fn seal(&mut self, plaintext: &[u8]) -> Vec<u8> {
        let nonce = self.next_nonce();
        seal_with_nonce(&self.key, &nonce, plaintext)
    }

    /// Mở bản mã (thao tác `open`), tách nonce ở đầu rồi giải mã phần còn lại.
    pub fn open(&self, ciphertext: &[u8]) -> Result<Vec<u8>, String> {
        if ciphertext.len() < XCHACHA_NONCE_LEN {
            return Err(format!(
                "Bản mã quá ngắn: cần ít nhất {XCHACHA_NONCE_LEN} byte nonce, nhận {}",
                ciphertext.len()
            ));
        }
        let mut nonce = [0u8; 12];
        nonce.copy_from_slice(&ciphertext[..XCHACHA_NONCE_LEN]);
        Ok(open_with_nonce(
            &self.key,
            &nonce,
            &ciphertext[XCHACHA_NONCE_LEN..],
        ))
    }

    /// Tên gọi tương thích với trait `ChannelCipher` (bản `&mut self` có tăng bộ đếm).
    pub fn encrypt(&mut self, plaintext: &[u8]) -> Vec<u8> {
        self.seal(plaintext)
    }

    /// Tên gọi tương thích với trait `ChannelCipher` (bản trả về `Result`).
    pub fn decrypt(&self, ciphertext: &[u8]) -> Result<Vec<u8>, String> {
        self.open(ciphertext)
    }
}

/// Dẫn khóa phụ 256-bit từ khóa chính và nonce mở rộng 24 byte bằng HChaCha20.
///
/// Nonce mở rộng gồm 16 byte đầu băm từ (khóa, nonce) và 8 byte cuối là bộ đếm.
fn derive_subkey(key: &[u8; 32], nonce: &[u8; 12]) -> [u8; 32] {
    let mut extended = [0u8; XCHACHA_EXTENDED_NONCE_LEN];
    let mut material = Vec::with_capacity(32 + 12);
    material.extend_from_slice(key);
    material.extend_from_slice(nonce);
    let digest = sha256(&material);
    extended[..16].copy_from_slice(&digest[..16]);
    extended[16..24].copy_from_slice(&nonce[4..12]);
    hchacha20(key, &extended[..16])
}

/// Mã hoá bản rõ với một nonce cụ thể, trả về `nonce || ciphertext`.
fn seal_with_nonce(key: &[u8; 32], nonce: &[u8; 12], plaintext: &[u8]) -> Vec<u8> {
    let subkey = derive_subkey(key, nonce);
    let mut out = Vec::with_capacity(XCHACHA_NONCE_LEN + plaintext.len());
    out.extend_from_slice(nonce);
    out.extend_from_slice(&chacha20(&subkey, nonce, plaintext));
    out
}

/// Giải mã phần bản mã đã tách nonce.
fn open_with_nonce(key: &[u8; 32], nonce: &[u8; 12], ciphertext: &[u8]) -> Vec<u8> {
    let subkey = derive_subkey(key, nonce);
    chacha20(&subkey, nonce, ciphertext)
}

impl ChannelCipher for XChaCha20Channel {
    /// Mã hoá theo kiểu SIV: nonce được dẫn từ nội dung bản rõ nên không bao giờ tái sử dụng
    /// keystream cho hai bản rõ khác nhau, kể cả khi lời gọi chỉ có `&self`.
    fn encrypt(&self, plaintext: &[u8]) -> Vec<u8> {
        let mut material = Vec::with_capacity(32 + 8 + plaintext.len());
        material.extend_from_slice(&self.key);
        material.extend_from_slice(&self.nonce_counter.to_le_bytes());
        material.extend_from_slice(plaintext);
        let digest = sha256(&material);
        let mut nonce = [0u8; 12];
        nonce.copy_from_slice(&digest[..12]);
        seal_with_nonce(&self.key, &nonce, plaintext)
    }

    /// Giải mã bản mã theo định dạng `nonce || ciphertext`.
    fn decrypt(&self, ciphertext: &[u8]) -> Vec<u8> {
        if ciphertext.len() < XCHACHA_NONCE_LEN {
            return Vec::new();
        }
        let mut nonce = [0u8; 12];
        nonce.copy_from_slice(&ciphertext[..XCHACHA_NONCE_LEN]);
        open_with_nonce(&self.key, &nonce, &ciphertext[XCHACHA_NONCE_LEN..])
    }

    /// Độ dài nonce trên đường truyền.
    fn nonce_len(&self) -> usize {
        XCHACHA_NONCE_LEN
    }
}

// ============================================================================
// 4. TRAO ĐỔI KHÓA ECDH X25519 TRÊN MONTGOMERY LADDER (RFC 7748)
// ============================================================================

/// Phần tử của trường GF(2^255 - 19), biểu diễn bằng 5 limb 51-bit (radix 2^51).
type Fe = [u64; 5];

/// Mặt nạ giữ lại 51 bit thấp của một limb.
const FE_MASK: u64 = (1u64 << 51) - 1;

/// Biểu diễn của `2 * p` trong hệ limb 51-bit, dùng để tránh kẹp âm khi trừ.
const FE_TWO_P: [u64; 5] = [
    0xF_FFFF_FFFF_FFDA,
    0xF_FFFF_FFFF_FFFE,
    0xF_FFFF_FFFF_FFFE,
    0xF_FFFF_FFFF_FFFE,
    0xF_FFFF_FFFF_FFFE,
];

/// Số mũ đảo ngược trong trường: `p - 2 = 2^255 - 21` (little-endian).
const FE_P_MINUS_2: [u8; 32] = [
    0xEB, 0xFF, 0xFF, 0xFF, 0xFF, 0xFF, 0xFF, 0xFF, 0xFF, 0xFF, 0xFF, 0xFF, 0xFF, 0xFF, 0xFF, 0xFF,
    0xFF, 0xFF, 0xFF, 0xFF, 0xFF, 0xFF, 0xFF, 0xFF, 0xFF, 0xFF, 0xFF, 0xFF, 0xFF, 0xFF, 0xFF, 0x7F,
];

/// Hằng số trường `p = 2^255 - 19` ở dạng 32 byte little-endian.
const FE_PRIME_BYTES: [u8; 32] = [
    0xED, 0xFF, 0xFF, 0xFF, 0xFF, 0xFF, 0xFF, 0xFF, 0xFF, 0xFF, 0xFF, 0xFF, 0xFF, 0xFF, 0xFF, 0xFF,
    0xFF, 0xFF, 0xFF, 0xFF, 0xFF, 0xFF, 0xFF, 0xFF, 0xFF, 0xFF, 0xFF, 0xFF, 0xFF, 0xFF, 0xFF, 0x7F,
];

/// Phần tử 0 của trường.
fn fe_zero() -> Fe {
    [0; 5]
}

/// Phần tử 1 của trường.
fn fe_one() -> Fe {
    [1, 0, 0, 0, 0]
}

/// Chuẩn hoá limb: gán phần dư 2^51 và gấp phần dư vào limb 0 theo `2^255 = 19 (mod p)`.
fn fe_carry(r: &mut Fe) {
    let c0 = r[0] >> 51;
    r[0] &= FE_MASK;
    r[1] += c0;
    let c1 = r[1] >> 51;
    r[1] &= FE_MASK;
    r[2] += c1;
    let c2 = r[2] >> 51;
    r[2] &= FE_MASK;
    r[3] += c2;
    let c3 = r[3] >> 51;
    r[3] &= FE_MASK;
    r[4] += c3;
    let c4 = r[4] >> 51;
    r[4] &= FE_MASK;
    r[0] += c4 * 19;
    let c5 = r[0] >> 51;
    r[0] &= FE_MASK;
    r[1] += c5;
}

/// Cộng hai phần tử trường.
fn fe_add(a: &Fe, b: &Fe) -> Fe {
    let mut r = [0u64; 5];
    for (dst, (&x, &y)) in r.iter_mut().zip(a.iter().zip(b.iter())) {
        *dst = x + y;
    }
    fe_carry(&mut r);
    r
}

/// Trừ hai phần tử trường (cộng bù `2p` để không kẹp âm).
fn fe_sub(a: &Fe, b: &Fe) -> Fe {
    let mut r = [0u64; 5];
    for (dst, ((x, y), p2)) in r
        .iter_mut()
        .zip(a.iter().zip(b.iter()).zip(FE_TWO_P.iter()))
    {
        *dst = *x + *p2 - *y;
    }
    fe_carry(&mut r);
    r
}

/// Nhân hai phần tử trường bằng phép nhân học trên 5 limb 51-bit.
fn fe_mul(a: &Fe, b: &Fe) -> Fe {
    let a0 = a[0] as u128;
    let a1 = a[1] as u128;
    let a2 = a[2] as u128;
    let a3 = a[3] as u128;
    let a4 = a[4] as u128;
    let b0 = b[0] as u128;
    let b1 = b[1] as u128;
    let b2 = b[2] as u128;
    let b3 = b[3] as u128;
    let b4 = b[4] as u128;

    // Các hệ số 19 bù lại việc gấp limb thứ 5 (2^255 = 19 mod p).
    let b1_19 = 19 * b1;
    let b2_19 = 19 * b2;
    let b3_19 = 19 * b3;
    let b4_19 = 19 * b4;

    let r0 = a0 * b0 + a1 * b4_19 + a2 * b3_19 + a3 * b2_19 + a4 * b1_19;
    let r1 = a0 * b1 + a1 * b0 + a2 * b4_19 + a3 * b3_19 + a4 * b2_19;
    let r2 = a0 * b2 + a1 * b1 + a2 * b0 + a3 * b4_19 + a4 * b3_19;
    let r3 = a0 * b3 + a1 * b2 + a2 * b1 + a3 * b0 + a4 * b4_19;
    let r4 = a0 * b4 + a1 * b3 + a2 * b2 + a3 * b1 + a4 * b0;

    let mut out = [0u64; 5];
    let c = r0 >> 51;
    out[0] = (r0 as u64) & FE_MASK;
    let r1 = r1 + c;
    let c = r1 >> 51;
    out[1] = (r1 as u64) & FE_MASK;
    let r2 = r2 + c;
    let c = r2 >> 51;
    out[2] = (r2 as u64) & FE_MASK;
    let r3 = r3 + c;
    let c = r3 >> 51;
    out[3] = (r3 as u64) & FE_MASK;
    let r4 = r4 + c;
    let c = r4 >> 51;
    out[4] = (r4 as u64) & FE_MASK;
    out[0] += (c as u64) * 19;
    let c = out[0] >> 51;
    out[0] &= FE_MASK;
    out[1] += c;
    out
}

/// Bình phương một phần tử trường.
fn fe_sq(a: &Fe) -> Fe {
    fe_mul(a, a)
}

/// Nhân phần tử trường với một hằng số nhỏ (dùng cho hệ số `a24 = 121665`).
fn fe_mul_small(a: &Fe, n: u64) -> Fe {
    let mut out = [0u64; 5];
    let mut carry: u128 = 0;
    for (dst, &x) in out.iter_mut().zip(a.iter()) {
        let t = (x as u128) * (n as u128) + carry;
        *dst = (t as u64) & FE_MASK;
        carry = t >> 51;
    }
    out[0] += (carry as u64) * 19;
    let c = out[0] >> 51;
    out[0] &= FE_MASK;
    out[1] += c;
    out
}

/// Lũy thừa bằng phương pháp bình phương-nhân theo thứ tự bit từ cao xuống thấp.
///
/// Mảng `exp` được lưu theo thứ tự little-endian (byte 0 là byte ít trọng số nhất), vì vậy
/// phải duyệt từ byte cuối về byte đầu để thuật toán "bình phương-nhân" xử lý đúng thứ tự bit.
fn fe_pow(base: &Fe, exp: &[u8; 32]) -> Fe {
    let mut result = fe_one();
    for byte in exp.iter().rev() {
        for bit in (0..8).rev() {
            result = fe_sq(&result);
            if (byte >> bit) & 1 == 1 {
                result = fe_mul(&result, base);
            }
        }
    }
    result
}

/// Nghịch đảo phần tử trường: `z^(p-2)` theo định lý Fermat.
fn fe_invert(z: &Fe) -> Fe {
    fe_pow(z, &FE_P_MINUS_2)
}

/// Đổi 32 byte little-endian thành phần tử trường (bit cao nhất bị che theo RFC 7748).
fn fe_from_bytes(bytes: &[u8; 32]) -> Fe {
    let mut words = [0u64; 4];
    for (word, chunk) in words.iter_mut().zip(bytes.as_chunks::<8>().0.iter()) {
        *word = u64::from_le_bytes([
            chunk[0], chunk[1], chunk[2], chunk[3], chunk[4], chunk[5], chunk[6], chunk[7],
        ]);
    }
    let l0 = words[0];
    let l1 = words[1];
    let l2 = words[2];
    let l3 = words[3];
    [
        l0 & FE_MASK,
        ((l0 >> 51) | (l1 << 13)) & FE_MASK,
        ((l1 >> 38) | (l2 << 26)) & FE_MASK,
        ((l2 >> 25) | (l3 << 39)) & FE_MASK,
        (l3 >> 12) & FE_MASK,
    ]
}

/// Đổi phần tử trường thành 32 byte little-endian ở dạng chuẩn (0 <= u < p).
fn fe_to_bytes(a: &Fe) -> [u8; 32] {
    let mut out = [0u8; 32];
    let mut acc: u128 = 0;
    let mut bits: u32 = 0;
    let mut index = 0usize;
    for &limb in a.iter() {
        acc |= (limb as u128) << bits;
        bits += 51;
        while bits >= 8 && index < 32 {
            out[index] = (acc & 0xFF) as u8;
            acc >>= 8;
            bits -= 8;
            index += 1;
        }
    }
    if index < 32 {
        out[index] = (acc & 0xFF) as u8;
    }

    // Giảm về dạng chuẩn nếu giá trị >= p.
    if u256_cmp(&out, &FE_PRIME_BYTES) != std::cmp::Ordering::Less {
        u256_sub(&mut out, &FE_PRIME_BYTES);
    }
    out
}

/// So sánh hai số nguyên 256-bit little-endian.
fn u256_cmp(a: &[u8; 32], b: &[u8; 32]) -> std::cmp::Ordering {
    for (x, y) in a.iter().zip(b.iter()).rev() {
        match x.cmp(y) {
            std::cmp::Ordering::Equal => {}
            other => return other,
        }
    }
    std::cmp::Ordering::Equal
}

/// Trừ hai số nguyên 256-bit little-endian (giả định `a >= b`).
fn u256_sub(a: &mut [u8; 32], b: &[u8; 32]) {
    let mut borrow: i32 = 0;
    for (x, y) in a.iter_mut().zip(b.iter()) {
        let diff = *x as i32 - *y as i32 - borrow;
        if diff < 0 {
            *x = (diff + 256) as u8;
            borrow = 1;
        } else {
            *x = diff as u8;
            borrow = 0;
        }
    }
}

/// Hoán đổi hai phần tử trường theo kiểu hằng thời (constant-time) của RFC 7748 §5.
fn fe_cswap(swap: u64, a: &mut Fe, b: &mut Fe) {
    let mask = 0u64.wrapping_sub(swap);
    for (x, y) in a.iter_mut().zip(b.iter_mut()) {
        let dummy = mask & (*x ^ *y);
        *x ^= dummy;
        *y ^= dummy;
    }
}

/// Hằng số `a24 = (486662 - 2) / 4` của đường cong Montgomery curve25519.
const X25519_A24: u64 = 121665;

/// Hàm X25519 chuẩn của RFC 7748: nhân scalar với u-coord của điểm Montgomery.
///
/// Bản thân hàm đã chứa thao tác "clamp" scalar theo RFC 7748 §5 và che bit cao nhất
/// của u-coord đầu vào.
pub fn x25519(secret: &[u8; 32], peer_public: &[u8; 32]) -> [u8; 32] {
    let mut k = *secret;
    k[0] &= 248;
    k[31] &= 127;
    k[31] |= 64;

    let mut u_bytes = *peer_public;
    u_bytes[31] &= 127;
    let u = fe_from_bytes(&u_bytes);

    let x1 = u;
    let mut x2 = fe_one();
    let mut z2 = fe_zero();
    let mut x3 = u;
    let mut z3 = fe_one();
    let mut swap: u64 = 0;

    for t in (0..255).rev() {
        let kt = u64::from((k[t >> 3] >> (t & 7)) & 1);
        swap ^= kt;
        fe_cswap(swap, &mut x2, &mut x3);
        fe_cswap(swap, &mut z2, &mut z3);
        swap = kt;

        let a = fe_add(&x2, &z2);
        let aa = fe_sq(&a);
        let b = fe_sub(&x2, &z2);
        let bb = fe_sq(&b);
        let e = fe_sub(&aa, &bb);
        let c = fe_add(&x3, &z3);
        let d = fe_sub(&x3, &z3);
        let da = fe_mul(&d, &a);
        let cb = fe_mul(&c, &b);

        x3 = fe_sq(&fe_add(&da, &cb));
        z3 = fe_mul(&x1, &fe_sq(&fe_sub(&da, &cb)));
        x2 = fe_mul(&aa, &bb);
        z2 = fe_mul(&e, &fe_add(&aa, &fe_mul_small(&e, X25519_A24)));
    }

    fe_cswap(swap, &mut x2, &mut x3);
    fe_cswap(swap, &mut z2, &mut z3);

    fe_to_bytes(&fe_mul(&x2, &fe_invert(&z2)))
}

/// Khóa bí mật X25519 kèm các phép toán sinh khóa công khai và ECDH.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct X25519 {
    /// Khóa bí mật 32 byte đã được "clamp" theo RFC 7748.
    pub secret: [u8; 32],
}

impl X25519 {
    /// Sinh khóa bí mật ngẫu nhiên (tất định theo `SplitMix64`) và clamp theo RFC 7748.
    pub fn generate(rng: &mut SplitMix64) -> Self {
        let mut secret = [0u8; 32];
        for chunk in secret.chunks_mut(8) {
            chunk.copy_from_slice(&rng.next_u64().to_le_bytes());
        }
        secret[0] &= 248;
        secret[31] &= 127;
        secret[31] |= 64;
        Self { secret }
    }

    /// Tạo khóa bí mật từ một mảng byte cho trước (tự clamp theo RFC 7748).
    pub fn from_bytes(secret: &[u8; 32]) -> Self {
        let mut clamped = *secret;
        clamped[0] &= 248;
        clamped[31] &= 127;
        clamped[31] |= 64;
        Self { secret: clamped }
    }

    /// Khóa công khai tương ứng: `X25519(secret, 9)` với 9 là u-coord của điểm cơ sở.
    pub fn public_key(&self) -> [u8; 32] {
        let mut basepoint = [0u8; 32];
        basepoint[0] = 9;
        x25519(&self.secret, &basepoint)
    }

    /// Tính bí mật chung ECDH với khóa công khai của đối tác.
    pub fn diffie_hellman(&self, peer_public: &[u8; 32]) -> [u8; 32] {
        x25519(&self.secret, peer_public)
    }
}

// ============================================================================
// 5. MÃ HOÁ BASE64 (RFC 4648) - LỚP VỎ CHO LƯU LƯỢNG OBFUSCATE
// ============================================================================

/// Bảng ký tự chuẩn Base64 (RFC 4648 §4).
const BASE64_ALPHABET: &[u8; 64] =
    b"ABCDEFGHIJKLMNOPQRSTUVWXYZabcdefghijklmnopqrstuvwxyz0123456789+/";

/// Tra cứu giá trị 6-bit của một ký tự Base64.
fn base64_value(c: u8) -> Option<u8> {
    match c {
        b'A'..=b'Z' => Some(c - b'A'),
        b'a'..=b'z' => Some(c - b'a' + 26),
        b'0'..=b'9' => Some(c - b'0' + 52),
        b'+' => Some(62),
        b'/' => Some(63),
        _ => None,
    }
}

/// Mã hoá dữ liệu thành chuỗi Base64 có đệm `=`.
pub fn base64_encode(data: &[u8]) -> String {
    let mut out = String::with_capacity(data.len().div_ceil(3) * 4);
    for chunk in data.chunks(3) {
        let b0 = chunk[0] as u32;
        let b1 = *chunk.get(1).unwrap_or(&0) as u32;
        let b2 = *chunk.get(2).unwrap_or(&0) as u32;
        let n = (b0 << 16) | (b1 << 8) | b2;
        out.push(BASE64_ALPHABET[((n >> 18) & 63) as usize] as char);
        out.push(BASE64_ALPHABET[((n >> 12) & 63) as usize] as char);
        if chunk.len() > 1 {
            out.push(BASE64_ALPHABET[((n >> 6) & 63) as usize] as char);
        } else {
            out.push('=');
        }
        if chunk.len() > 2 {
            out.push(BASE64_ALPHABET[(n & 63) as usize] as char);
        } else {
            out.push('=');
        }
    }
    out
}

/// Giải mã chuỗi Base64, trả về lỗi nếu định dạng sai.
pub fn base64_decode(s: &str) -> Result<Vec<u8>, String> {
    let bytes = s.as_bytes();
    if !bytes.len().is_multiple_of(4) {
        return Err(format!(
            "Độ dài Base64 không hợp lệ: {} (phải chia hết cho 4)",
            bytes.len()
        ));
    }
    let mut out = Vec::with_capacity(bytes.len() / 4 * 3);
    let chunk_count = bytes.len() / 4;
    for (index, chunk) in bytes.chunks(4).enumerate() {
        let mut values = [0u32; 4];
        let mut pad = 0usize;
        for (i, &c) in chunk.iter().enumerate() {
            if c == b'=' {
                pad += 1;
                continue;
            }
            if pad > 0 {
                return Err("Ký tự đệm '=' xuất hiện giữa dữ liệu Base64".to_string());
            }
            values[i] = match base64_value(c) {
                Some(v) => u32::from(v),
                None => {
                    return Err(format!("Ký tự Base64 không hợp lệ: '{}'", char::from(c)));
                }
            };
        }
        if pad > 0 && index + 1 != chunk_count {
            return Err("Ký tự đệm '=' chỉ được phép ở khối cuối cùng".to_string());
        }
        let n = (values[0] << 18) | (values[1] << 12) | (values[2] << 6) | values[3];
        out.push(((n >> 16) & 0xFF) as u8);
        if pad < 2 {
            out.push(((n >> 8) & 0xFF) as u8);
        }
        if pad < 1 {
            out.push((n & 0xFF) as u8);
        }
    }
    Ok(out)
}

// ============================================================================
// 6. BỌC LƯU LƯỢNG GIẢ DẠNG HTTP/DNS (OBFUSCATION)
// ============================================================================

/// Kiểu lớp vỏ dùng để giấu lưu lượng C2 trong giao thức lành thường.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum ObfuscationMode {
    /// Gói tin nhắn thành một yêu cầu HTTP GET
    HttpRequest,
    /// Gói tin nhắn thành một phản hồi HTTP 200
    HttpResponse,
    /// Gói tin nhắn thành một truy vấn DNS (kỹ thuật DNS tunnelling)
    DnsQuery,
    /// Gói tin nhắn thành một phản hồi DNS
    DnsResponse,
    /// Không bọc, giữ nguyên dữ liệu thô
    Raw,
}

/// Bộ bọc lưu lượng C2 thành HTTP/DNS để vượt qua bộ dò quét dựa trên heuristic.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct TrafficObfuscator {
    /// Chế độ bọc hiện tại
    pub mode: ObfuscationMode,
    /// Độ trễ jitter (ms) dùng để chống phân tích nhịp beacon
    pub jitter_ms: u64,
}

impl TrafficObfuscator {
    /// Khởi tạo bộ bọc theo chế độ cho trước.
    pub fn new(mode: ObfuscationMode) -> Self {
        Self { mode, jitter_ms: 0 }
    }

    /// Bổ sung tham số jitter theo mẫu builder.
    pub fn with_jitter(mut self, jitter_ms: u64) -> Self {
        self.jitter_ms = jitter_ms;
        self
    }

    /// Bọc bản rõ thành lưu lượng giả theo chế độ hiện tại.
    pub fn wrap(&self, payload: &[u8]) -> Vec<u8> {
        match self.mode {
            ObfuscationMode::HttpRequest => self.wrap_http_request(payload),
            ObfuscationMode::HttpResponse => self.wrap_http_response(payload),
            ObfuscationMode::DnsQuery => self.wrap_dns_query(payload),
            ObfuscationMode::DnsResponse => self.wrap_dns_response(payload),
            ObfuscationMode::Raw => payload.to_vec(),
        }
    }

    /// Tháo lớp vỏ để lấy lại bản rõ.
    pub fn unwrap(&self, data: &[u8]) -> Result<Vec<u8>, String> {
        match self.mode {
            ObfuscationMode::HttpRequest => unwrap_http_request(data),
            ObfuscationMode::HttpResponse => unwrap_http_response(data),
            ObfuscationMode::DnsQuery | ObfuscationMode::DnsResponse => {
                let rdata = dns_parse_rdata(data)?;
                base64_decode(&String::from_utf8_lossy(&rdata))
            }
            ObfuscationMode::Raw => Ok(data.to_vec()),
        }
    }

    /// Kiểm tra lưu lượng có giống HTTP hay không.
    pub fn looks_like_http(&self, data: &[u8]) -> bool {
        const METHODS: [&[u8]; 5] = [b"GET ", b"POST ", b"HEAD ", b"PUT ", b"DELETE "];
        if data.starts_with(b"HTTP/1.") {
            return true;
        }
        METHODS.iter().any(|m| data.starts_with(m))
    }

    /// Kiểm tra lưu lượng có giống DNS hay không.
    pub fn looks_like_dns(&self, data: &[u8]) -> bool {
        if data.len() < 12 {
            return false;
        }
        let flags = u16::from_be_bytes([data[2], data[3]]);
        let opcode = (flags >> 11) & 0x0F;
        let qdcount = u16::from_be_bytes([data[4], data[5]]);
        opcode == 0 && qdcount >= 1
    }

    /// Gói bản rõ thành yêu cầu HTTP GET có tham số `?d=<base64>`.
    fn wrap_http_request(&self, payload: &[u8]) -> Vec<u8> {
        let encoded = base64_encode(payload);
        let text = format!(
            "GET /static/img/px.gif?d={encoded} HTTP/1.1\r\n\
             Host: cdn.jsdelivr.net\r\n\
             User-Agent: Mozilla/5.0 (X11; Linux x86_64) AppleWebKit/537.36\r\n\
             Accept: */*\r\n\
             Accept-Encoding: gzip, deflate\r\n\
             Connection: keep-alive\r\n\r\n"
        );
        text.into_bytes()
    }

    /// Gói bản rõ thành phản hồi HTTP 200 với body Base64.
    fn wrap_http_response(&self, payload: &[u8]) -> Vec<u8> {
        let encoded = base64_encode(payload);
        let text = format!(
            "HTTP/1.1 200 OK\r\n\
             Server: nginx/1.22.1\r\n\
             Content-Type: application/octet-stream\r\n\
             Content-Length: {}\r\n\
             Cache-Control: no-store\r\n\
             X-Cache: HIT\r\n\r\n{encoded}",
            encoded.len()
        );
        text.into_bytes()
    }

    /// Gói bản rõ thành truy vấn DNS có bản ghi TXT bổ sung mang dữ liệu Base64.
    fn wrap_dns_query(&self, payload: &[u8]) -> Vec<u8> {
        let rdata = base64_encode(payload).into_bytes();
        let mut packet = dns_header(0x4D2C, 0x0100, 1, 0, 0, 1);
        packet.extend_from_slice(&dns_question("tunnel.example.com", 16));
        packet.extend_from_slice(&dns_record(&[0x00], 16, 0, &rdata));
        packet
    }

    /// Gói bản rõ thành phản hồi DNS có bản ghi trả lời TXT.
    fn wrap_dns_response(&self, payload: &[u8]) -> Vec<u8> {
        let rdata = base64_encode(payload).into_bytes();
        let mut packet = dns_header(0x4D2C, 0x8180, 1, 1, 0, 0);
        packet.extend_from_slice(&dns_question("tunnel.example.com", 16));
        packet.extend_from_slice(&dns_record(&[0xC0, 0x0C], 16, 60, &rdata));
        packet
    }
}

/// Dựng phần đầu gói DNS 12 byte.
fn dns_header(id: u16, flags: u16, qd: u16, an: u16, ns: u16, ar: u16) -> Vec<u8> {
    let mut out = Vec::with_capacity(12);
    out.extend_from_slice(&id.to_be_bytes());
    out.extend_from_slice(&flags.to_be_bytes());
    out.extend_from_slice(&qd.to_be_bytes());
    out.extend_from_slice(&an.to_be_bytes());
    out.extend_from_slice(&ns.to_be_bytes());
    out.extend_from_slice(&ar.to_be_bytes());
    out
}

/// Dựng phần câu hỏi DNS (QNAME + QTYPE + QCLASS).
fn dns_question(name: &str, qtype: u16) -> Vec<u8> {
    let mut out = Vec::new();
    for label in name.split('.') {
        out.push(label.len() as u8);
        out.extend_from_slice(label.as_bytes());
    }
    out.push(0);
    out.extend_from_slice(&qtype.to_be_bytes());
    out.extend_from_slice(&1u16.to_be_bytes());
    out
}

/// Dựng một bản ghi tài nguyên DNS (NAME + TYPE + CLASS + TTL + RDLENGTH + RDATA).
fn dns_record(name: &[u8], rtype: u16, ttl: u32, rdata: &[u8]) -> Vec<u8> {
    let mut out = Vec::with_capacity(name.len() + 10 + rdata.len());
    out.extend_from_slice(name);
    out.extend_from_slice(&rtype.to_be_bytes());
    out.extend_from_slice(&1u16.to_be_bytes());
    out.extend_from_slice(&ttl.to_be_bytes());
    out.extend_from_slice(&(rdata.len() as u16).to_be_bytes());
    out.extend_from_slice(rdata);
    out
}

/// Bỏ qua một tên miền trong gói DNS, trả về vị trí byte kế tiếp.
fn dns_skip_name(data: &[u8], mut pos: usize) -> Result<usize, String> {
    loop {
        if pos >= data.len() {
            return Err("Tên miền DNS bị cắt cụt".to_string());
        }
        let len = data[pos];
        if len == 0 {
            return Ok(pos + 1);
        }
        if len & 0xC0 == 0xC0 {
            return Ok(pos + 2);
        }
        pos += 1 + len as usize;
    }
}

/// Trích xuất RDATA của bản ghi đầu tiên (đáp án hoặc bản ghi bổ sung).
fn dns_parse_rdata(data: &[u8]) -> Result<Vec<u8>, String> {
    if data.len() < 12 {
        return Err("Gói DNS quá ngắn".to_string());
    }
    let qdcount = u16::from_be_bytes([data[4], data[5]]) as usize;
    let ancount = u16::from_be_bytes([data[6], data[7]]) as usize;
    let arcount = u16::from_be_bytes([data[10], data[11]]) as usize;
    if qdcount == 0 {
        return Err("Gói DNS không có câu hỏi nào".to_string());
    }

    let mut pos = 12usize;
    for _ in 0..qdcount {
        pos = dns_skip_name(data, pos)?;
        pos += 4;
        if pos > data.len() {
            return Err("Gói DNS bị cắt cụt ở phần câu hỏi".to_string());
        }
    }

    if ancount == 0 && arcount == 0 {
        return Err("Gói DNS không có bản ghi trả lời hay bổ sung".to_string());
    }

    pos = dns_skip_name(data, pos)?;
    if pos + 10 > data.len() {
        return Err("Gói DNS bị cắt cụt ở phần bản ghi".to_string());
    }
    let rdlength = u16::from_be_bytes([data[pos + 8], data[pos + 9]]) as usize;
    pos += 10;
    if pos + rdlength > data.len() {
        return Err("Gói DNS bị cắt cụt ở phần dữ liệu bản ghi".to_string());
    }
    Ok(data[pos..pos + rdlength].to_vec())
}

/// Tháo lớp vỏ khỏi một yêu cầu HTTP.
fn unwrap_http_request(data: &[u8]) -> Result<Vec<u8>, String> {
    let text = String::from_utf8_lossy(data);
    let start = text
        .find("?d=")
        .ok_or_else(|| "Không tìm thấy tham số '?d=' trong yêu cầu HTTP".to_string())?;
    let rest = &text[start + 3..];
    let end = rest.find(' ').unwrap_or(rest.len());
    base64_decode(rest[..end].trim())
}

/// Tháo lớp vỏ khỏi một phản hồi HTTP.
fn unwrap_http_response(data: &[u8]) -> Result<Vec<u8>, String> {
    let text = String::from_utf8_lossy(data);
    let sep = text
        .find("\r\n\r\n")
        .ok_or_else(|| "Không tìm thấy phân cách header/body trong phản hồi HTTP".to_string())?;
    base64_decode(text[sep + 4..].trim())
}

// ============================================================================
// 7. MÔ PHỎNG ĐƯỜNG HẦM SSH (LOCAL / REMOTE FORWARD & DYNAMIC SOCKS)
// ============================================================================

/// Tên người dùng mặc định trong lệnh SSH sinh ra.
pub const SSH_TUNNEL_USER: &str = "user";

/// Tên máy chủ "bậc" mặc định trong lệnh SSH sinh ra.
pub const SSH_TUNNEL_HOST: &str = "jump";

/// Ba loại đường hầm SSH được mô phỏng trong chương này.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum TunnelType {
    /// Chuyển tiếp cục bộ: `-L` (client lắng nghe, server kết nối tới đích)
    LocalForward,
    /// Chuyển tiếp từ xa: `-R` (server lắng nghe, client kết nối tới đích)
    RemoteForward,
    /// SOCKS động: `-D` (client làm proxy SOCKS cho toàn bộ đích đến)
    DynamicSocks,
}

/// Mô tả một phiên đường hầm SSH đang hoạt động.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct SshTunnel {
    /// Loại đường hầm
    pub tunnel_type: TunnelType,
    /// Địa chỉ cổng lắng nghe
    pub listen_addr: SocketAddr,
    /// Địa chỉ đích phía sau đường hầm
    pub target_addr: SocketAddr,
    /// Mã định danh phiên (session id)
    pub session_id: u32,
    /// Trạng thái hoạt động của đường hầm
    pub active: bool,
}

/// Bộ mô phỏng các phiên đường hầm SSH đang mở.
#[derive(Debug, Clone)]
pub struct SshTunnelSimulator {
    /// Danh sách các đường hầm đã mở
    pub tunnels: Vec<SshTunnel>,
    /// Mã phiên tiếp theo sẽ cấp phát
    pub next_session_id: u32,
}

impl Default for SshTunnelSimulator {
    fn default() -> Self {
        Self {
            tunnels: Vec::new(),
            next_session_id: 1,
        }
    }
}

impl SshTunnelSimulator {
    /// Khởi tạo bộ mô phỏng rỗng.
    pub fn new() -> Self {
        Self::default()
    }

    /// Mở đường hầm chuyển tiếp cục bộ (`ssh -L`).
    pub fn local_forward(&mut self, listen: SocketAddr, target: SocketAddr) -> u32 {
        self.open(TunnelType::LocalForward, listen, target)
    }

    /// Mở đường hầm chuyển tiếp từ xa (`ssh -R`).
    pub fn remote_forward(&mut self, listen: SocketAddr, target: SocketAddr) -> u32 {
        self.open(TunnelType::RemoteForward, listen, target)
    }

    /// Mở đường hầm SOCKS động (`ssh -D`).
    pub fn dynamic_socks(&mut self, listen: SocketAddr) -> u32 {
        let placeholder: SocketAddr = "0.0.0.0:0".parse().expect("Địa chỉ giữ chỗ hợp lệ");
        self.open(TunnelType::DynamicSocks, listen, placeholder)
    }

    /// Đóng một đường hầm theo mã phiên, trả về `true` nếu tìm thấy.
    pub fn close_tunnel(&mut self, session_id: u32) -> bool {
        if let Some(tunnel) = self.tunnels.iter_mut().find(|t| t.session_id == session_id) {
            tunnel.active = false;
            return true;
        }
        false
    }

    /// Đếm số đường hầm đang hoạt động.
    pub fn active_tunnels(&self) -> usize {
        self.tunnels.iter().filter(|t| t.active).count()
    }

    /// Sinh mô tả dạng văn bản của toàn bộ đường hầm.
    pub fn describe(&self) -> String {
        if self.tunnels.is_empty() {
            return "Chưa có đường hầm nào được mở".to_string();
        }
        let mut out = String::new();
        for tunnel in &self.tunnels {
            let kind = match tunnel.tunnel_type {
                TunnelType::LocalForward => "LOCAL",
                TunnelType::RemoteForward => "REMOTE",
                TunnelType::DynamicSocks => "SOCKS",
            };
            let state = if tunnel.active { "active" } else { "closed" };
            let _ = write!(
                out,
                "[#{kind} #{}] {} -> {} ({state}) | ",
                tunnel.session_id, tunnel.listen_addr, tunnel.target_addr
            );
        }
        out.trim_end().to_string()
    }

    /// Cấp phát phiên mới cho một đường hầm.
    fn open(&mut self, tunnel_type: TunnelType, listen: SocketAddr, target: SocketAddr) -> u32 {
        let session_id = self.next_session_id;
        self.next_session_id = self.next_session_id.wrapping_add(1);
        self.tunnels.push(SshTunnel {
            tunnel_type,
            listen_addr: listen,
            target_addr: target,
            session_id,
            active: true,
        });
        session_id
    }
}

/// Sinh lệnh `ssh` tương ứng với một đường hầm.
///
/// • `-L` — chuyển tiếp cục bộ: `ssh -L <cổng>:<đích>:<cổng đích> user@jump`
/// • `-R` — chuyển tiếp từ xa: `ssh -R <cổng>:<đích>:<cổng đích> user@jump`
/// • `-D` — SOCKS động: `ssh -D <cổng> user@jump`
pub fn ssh_command_line(tunnel: &SshTunnel) -> String {
    match tunnel.tunnel_type {
        TunnelType::LocalForward | TunnelType::RemoteForward => {
            let flag = if tunnel.tunnel_type == TunnelType::LocalForward {
                "-L"
            } else {
                "-R"
            };
            format!(
                "ssh {flag} {}:{}:{} {SSH_TUNNEL_USER}@{SSH_TUNNEL_HOST}",
                tunnel.listen_addr.port(),
                tunnel.target_addr.ip(),
                tunnel.target_addr.port()
            )
        }
        TunnelType::DynamicSocks => format!(
            "ssh -D {} {SSH_TUNNEL_USER}@{SSH_TUNNEL_HOST}",
            tunnel.listen_addr.port()
        ),
    }
}

// ============================================================================
// 8. CHUỖI RELAY SOCKS NHIỀU BƯỚC (PIVOTING)
// ============================================================================

/// Chuỗi các trạm trung gian SOCKS mà lưu lượng phải đi qua.
#[derive(Debug, Clone, Default)]
pub struct SocksRelayChain {
    /// Danh sách các trạm trung gian theo thứ tự đi qua
    pub hops: Vec<SocketAddr>,
}

impl SocksRelayChain {
    /// Khởi tạo chuỗi rỗng.
    pub fn new() -> Self {
        Self::default()
    }

    /// Thêm một trạm trung gian vào cuối chuỗi.
    pub fn add_hop(&mut self, addr: SocketAddr) -> &mut Self {
        self.hops.push(addr);
        self
    }

    /// Số trạm trung gian trong chuỗi.
    pub fn hop_count(&self) -> usize {
        self.hops.len()
    }

    /// Dựng đường đi đầy đủ từ client qua các trạm trung gian tới đích.
    pub fn resolve_path(&self, target: &str, port: u16) -> Vec<String> {
        let mut path = vec!["client".to_string()];
        for hop in &self.hops {
            path.push(hop.to_string());
        }
        path.push(format!("{target}:{port}"));
        path
    }

    /// Tổng độ trễ của chuỗi (mỗi trạm một giá trị, trạm thiếu được tính là 0).
    pub fn total_latency_ms(&self, hop_latencies: &[f64]) -> f64 {
        self.hops
            .iter()
            .enumerate()
            .map(|(i, _)| hop_latencies.get(i).copied().unwrap_or(0.0))
            .sum()
    }
}

/// Mô phỏng đường đi của một chuỗi SOCKS: `client → hopA → hopB → target`.
pub fn simulate_chain(hops: &[SocketAddr], target: &str) -> Vec<String> {
    let mut path = vec!["client".to_string()];
    for hop in hops {
        path.push(hop.to_string());
    }
    path.push(target.to_string());
    path
}

// ============================================================================
// 9. KHUNG CHỈ HUY-KIỂM SOÁT (C2) MINI
// ============================================================================

/// Loại lệnh trao đổi giữa máy chủ C2 và tác tử.
#[derive(Debug, Clone, PartialEq, Eq)]
pub enum C2Command {
    /// Tác tử đăng ký với máy chủ
    Register,
    /// Tác tử gửi tín hiệu còn sống (beacon)
    Beacon,
    /// Máy chủ yêu cầu tác tử thực thi một lệnh
    TaskRequest,
    /// Tác tử báo kết quả thực thi
    TaskResult,
    /// Tác tử gửi kết quả thô của vỏ lệnh
    ShellOutput,
    /// Yêu cầu tự hủy
    Kill,
}

impl C2Command {
    /// Mã byte định danh lệnh trên đường truyền.
    fn tag(&self) -> u8 {
        match self {
            C2Command::Register => 0x01,
            C2Command::Beacon => 0x02,
            C2Command::TaskRequest => 0x03,
            C2Command::TaskResult => 0x04,
            C2Command::ShellOutput => 0x05,
            C2Command::Kill => 0x06,
        }
    }

    /// Khôi phục lệnh từ mã byte trên đường truyền.
    fn from_tag(tag: u8) -> Option<C2Command> {
        match tag {
            0x01 => Some(C2Command::Register),
            0x02 => Some(C2Command::Beacon),
            0x03 => Some(C2Command::TaskRequest),
            0x04 => Some(C2Command::TaskResult),
            0x05 => Some(C2Command::ShellOutput),
            0x06 => Some(C2Command::Kill),
            _ => None,
        }
    }
}

/// Phiên đăng ký của một tác tử trên máy chủ C2.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct AgentSession {
    /// Định danh tác tử
    pub id: String,
    /// Mốc thời gian beacon gần nhất
    pub last_seen: u64,
    /// Tên máy chủ của tác tử
    pub hostname: String,
    /// Hệ điều hành của tác tử
    pub os: String,
    /// Quyền hạn của tác tử (ví dụ `user`, `root`)
    pub privileges: String,
    /// Tác tử còn sống hay đã "chết"
    pub alive: bool,
}

/// Một nhiệm vụ (task) do máy chủ giao cho tác tử.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct Task {
    /// Mã định danh nhiệm vụ
    pub id: u32,
    /// Tác tử được giao nhiệm vụ
    pub agent_id: String,
    /// Nội dung lệnh cần thực thi
    pub command: String,
    /// Mốc thời gian giao nhiệm vụ
    pub issued_at: u64,
    /// Nhiệm vụ đã hoàn tất hay chưa
    pub completed: bool,
    /// Kết quả thực thi (nếu có)
    pub result: Option<String>,
}

/// Máy chủ chỉ huy-kiểm soát (Command & Control) mini.
///
/// Chứa danh sách phiên tác tử, hàng đợi nhiệm vụ và tuỳ chọn một kênh mã hoá.
pub struct C2Server {
    /// Các phiên tác tử đã đăng ký
    pub sessions: HashMap<String, AgentSession>,
    /// Hàng đợi nhiệm vụ
    pub tasks: Vec<Task>,
    /// Mã nhiệm vụ tiếp theo
    pub next_task_id: u32,
    /// Đồng hồ logic của máy chủ (tính bằng tick)
    pub clock: u64,
    /// Kênh mã hoá tùy chọn cho lưu lượng
    pub encryption: Option<Box<dyn ChannelCipher>>,
}

impl Default for C2Server {
    fn default() -> Self {
        Self {
            sessions: HashMap::new(),
            tasks: Vec::new(),
            next_task_id: 1,
            clock: 0,
            encryption: None,
        }
    }
}

/// Độ dài phần đầu thông điệp C2: 1 byte mã lệnh + 8 byte đồng hồ.
pub const C2_HEADER_LEN: usize = 9;

/// In thông tin tóm tắt của máy chủ (không in nội dung kênh mã hoá).
impl std::fmt::Debug for C2Server {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        f.debug_struct("C2Server")
            .field("sessions", &self.sessions.len())
            .field("tasks", &self.tasks.len())
            .field("next_task_id", &self.next_task_id)
            .field("clock", &self.clock)
            .field("encrypted", &self.encryption.is_some())
            .finish()
    }
}

impl C2Server {
    /// Khởi tạo máy chủ trống, không mã hoá.
    pub fn new() -> Self {
        Self::default()
    }

    /// Khởi tạo máy chủ có kèm kênh mã hoá.
    pub fn with_cipher(cipher: Box<dyn ChannelCipher>) -> Self {
        Self {
            encryption: Some(cipher),
            ..Self::default()
        }
    }

    /// Đăng ký một tác tử mới và trả về phiên tương ứng.
    pub fn register(
        &mut self,
        id: &str,
        hostname: &str,
        os: &str,
        privileges: &str,
    ) -> AgentSession {
        let session = AgentSession {
            id: id.to_string(),
            last_seen: self.clock,
            hostname: hostname.to_string(),
            os: os.to_string(),
            privileges: privileges.to_string(),
            alive: true,
        };
        self.sessions.insert(id.to_string(), session.clone());
        session
    }

    /// Ghi nhận một lần beacon của tác tử, trả về mốc thời gian hiện tại.
    pub fn beacon(&mut self, id: &str) -> Option<u64> {
        let now = self.clock;
        match self.sessions.get_mut(id) {
            Some(session) => {
                session.last_seen = now;
                session.alive = true;
                Some(now)
            }
            None => None,
        }
    }

    /// Giao một nhiệm vụ mới cho tác tử và trả về mã nhiệm vụ.
    pub fn enqueue_task(&mut self, agent_id: &str, command: &str) -> u32 {
        let id = self.next_task_id;
        self.next_task_id = self.next_task_id.wrapping_add(1);
        self.tasks.push(Task {
            id,
            agent_id: agent_id.to_string(),
            command: command.to_string(),
            issued_at: self.clock,
            completed: false,
            result: None,
        });
        id
    }

    /// Lấy nhiệm vụ chưa hoàn tất đầu tiên của tác tử.
    pub fn next_task(&mut self, agent_id: &str) -> Option<&mut Task> {
        self.tasks
            .iter_mut()
            .find(|t| t.agent_id == agent_id && !t.completed)
    }

    /// Đánh dấu một nhiệm vụ đã hoàn tất kèm kết quả.
    pub fn complete_task(&mut self, task_id: u32, result: &str) -> bool {
        match self.tasks.iter_mut().find(|t| t.id == task_id) {
            Some(task) => {
                task.completed = true;
                task.result = Some(result.to_string());
                true
            }
            None => false,
        }
    }

    /// Danh sách phiên tác tử, sắp xếp theo định danh để thứ tự tất định.
    pub fn sessions(&self) -> Vec<&AgentSession> {
        let mut list: Vec<&AgentSession> = self.sessions.values().collect();
        list.sort_by(|a, b| a.id.cmp(&b.id));
        list
    }

    /// Số phiên tác tử còn sống.
    pub fn active_sessions(&self) -> usize {
        self.sessions.values().filter(|s| s.alive).count()
    }

    /// Đóng gói một lệnh + payload thành thông điệp byte (có mã hoá nếu được cấu hình).
    pub fn encode_message(&self, command: &C2Command, payload: &str) -> Vec<u8> {
        let mut raw = Vec::with_capacity(C2_HEADER_LEN + payload.len());
        raw.push(command.tag());
        raw.extend_from_slice(&self.clock.to_le_bytes());
        raw.extend_from_slice(payload.as_bytes());
        match &self.encryption {
            Some(cipher) => cipher.encrypt(&raw),
            None => raw,
        }
    }

    /// Giải mã một thông điệp byte thành cặp (lệnh, payload).
    pub fn decode_message(&self, bytes: &[u8]) -> Result<(C2Command, String), String> {
        let raw = match &self.encryption {
            Some(cipher) => cipher.decrypt(bytes),
            None => bytes.to_vec(),
        };
        if raw.len() < C2_HEADER_LEN {
            return Err(format!(
                "Thông điệp C2 quá ngắn: cần ít nhất {C2_HEADER_LEN} byte, nhận {}",
                raw.len()
            ));
        }
        let command = C2Command::from_tag(raw[0])
            .ok_or_else(|| format!("Mã lệnh C2 không hợp lệ: 0x{:02X}", raw[0]))?;
        let payload = String::from_utf8(raw[C2_HEADER_LEN..].to_vec())
            .map_err(|e| format!("Payload C2 không phải UTF-8 hợp lệ: {e}"))?;
        Ok((command, payload))
    }

    /// Tăng đồng hồ logic của máy chủ và trả về giá trị mới.
    pub fn tick(&mut self) -> u64 {
        self.clock = self.clock.wrapping_add(1);
        self.clock
    }
}

/// Vỏ lệnh giả lập (mock shell) — không chạm vào hệ thống thật.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct MockShell {
    /// Tài khoản đang đăng nhập
    pub current_user: String,
    /// Tài khoản có đặc quyền root hay không
    pub is_root: bool,
    /// Lịch sử các lệnh đã thực thi dạng (lệnh, kết quả)
    pub history: Vec<(String, String)>,
}

impl Default for MockShell {
    fn default() -> Self {
        Self {
            current_user: "agent".to_string(),
            is_root: false,
            history: Vec::new(),
        }
    }
}

impl MockShell {
    /// Khởi tạo vỏ lệnh giả lập với tài khoản không đặc quyền.
    pub fn new() -> Self {
        Self::default()
    }

    /// Nâng vỏ lệnh lên tài khoản root (mẫu builder).
    pub fn as_root(mut self) -> Self {
        self.current_user = "root".to_string();
        self.is_root = true;
        self
    }

    /// Thực thi một lệnh giả lập và ghi vào lịch sử.
    pub fn exec(&mut self, command: &str) -> String {
        let cmd = command.trim();
        let output = self.dispatch(cmd);
        self.history.push((cmd.to_string(), output.clone()));
        output
    }

    /// Bảng tra cứu lệnh giả lập.
    fn dispatch(&self, cmd: &str) -> String {
        match cmd {
            "whoami" => self.current_user.clone(),
            "id" => format!(
                "uid=1000({user}) gid=1000({user}) groups=1000({user})",
                user = self.current_user
            ),
            "hostname" => "mock-agent-host".to_string(),
            "pwd" => {
                if self.is_root {
                    "/root".to_string()
                } else {
                    "/home/agent".to_string()
                }
            }
            "uname -a" => "Linux mock-agent-host 5.15.0-mock #1 SMP x86_64 GNU/Linux".to_string(),
            "ls" => "agent.log  notes.txt  payloads".to_string(),
            "kill" => kill_signal(),
            "reboot" => {
                if self.is_root {
                    "OK".to_string()
                } else {
                    "Permission denied".to_string()
                }
            }
            _ => {
                if let Some(rest) = cmd.strip_prefix("echo ") {
                    rest.to_string()
                } else if let Some(rest) = cmd.strip_prefix("cat ") {
                    format!("{rest}: dữ liệu mô phỏng tất định")
                } else {
                    format!("{cmd}: command not found")
                }
            }
        }
    }
}

/// Tín hiệu huỷ diệt tiêu chuẩn dùng trong khung C2.
pub fn kill_signal() -> String {
    "KILL".to_string()
}

/// Tác tử phía máy chủ bị điều khiển (agent).
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct C2Agent {
    /// Định danh tác tử
    pub id: String,
    /// Tên máy chủ
    pub hostname: String,
    /// Hệ điều hành
    pub os: String,
    /// Quyền hạn hiện tại
    pub privileges: String,
    /// Chu kỳ beacon (ms)
    pub beacon_interval_ms: u64,
    /// Độ lệch ngẫu nhiên của beacon (ms)
    pub jitter_ms: u64,
    /// Tác tử đã tự hủy hay chưa
    pub destroyed: bool,
    /// Nhật ký thao tác của tác tử
    pub trace: Vec<String>,
}

impl C2Agent {
    /// Khởi tạo tác tử mới với các giá trị mặc định.
    pub fn new(id: &str) -> Self {
        Self {
            id: id.to_string(),
            hostname: format!("host-{id}"),
            os: "Linux 5.15 x86_64".to_string(),
            privileges: "user".to_string(),
            beacon_interval_ms: 5_000,
            jitter_ms: 0,
            destroyed: false,
            trace: Vec::new(),
        }
    }

    /// Thiết lập độ lệch beacon (mẫu builder).
    pub fn with_jitter(mut self, jitter_ms: u64) -> Self {
        self.jitter_ms = jitter_ms;
        self
    }

    /// Tính độ trễ tới lượt beacon kế tiếp (chu kỳ + jitter ngẫu nhiên).
    pub fn next_beacon_delay(&self, rng: &mut SplitMix64) -> u64 {
        if self.jitter_ms == 0 {
            return self.beacon_interval_ms;
        }
        let jitter = rng.next_u64() % (self.jitter_ms + 1);
        self.beacon_interval_ms.saturating_add(jitter)
    }

    /// Thực thi một nhiệm vụ trên vỏ lệnh giả lập và trả về kết quả.
    pub fn handle_task(&mut self, task: &Task, shell: &mut MockShell) -> String {
        if self.destroyed {
            let signal = kill_signal();
            self.trace
                .push(format!("task#{} bị từ chối do đã tự hủy", task.id));
            return signal;
        }
        let output = shell.exec(&task.command);
        self.trace.push(format!(
            "task#{} '{}' -> '{}'",
            task.id, task.command, output
        ));
        output
    }

    /// Tự hủy tác tử: đánh dấu `destroyed` và ghi nhật ký.
    pub fn self_destruct(&mut self) {
        self.destroyed = true;
        self.trace
            .push("self-destruct: đã gọi kill_signal()".to_string());
    }

    /// Nhật ký thao tác của tác tử.
    pub fn trace(&self) -> &[String] {
        &self.trace
    }
}

/// Phiên làm việc phía tác tử của kênh chỉ huy-kiểm soát.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct C2Session {
    /// Định danh tác tử sở hữu phiên
    pub agent_id: String,
    /// Khoá phiên dẫn xuất từ bí mật ECDH
    pub session_key: [u8; 32],
    /// Số lần beacon đã gửi
    pub beacon_count: u64,
    /// Mốc thời gian thiết lập phiên
    pub established_at: u64,
}

impl C2Session {
    /// Khởi tạo phiên tác tử với khoá phiên cho trước.
    pub fn new(agent_id: &str, session_key: [u8; 32], established_at: u64) -> Self {
        Self {
            agent_id: agent_id.to_string(),
            session_key,
            beacon_count: 0,
            established_at,
        }
    }

    /// Ghi nhận một lần beacon và tăng bộ đếm.
    pub fn record_beacon(&mut self) -> u64 {
        self.beacon_count = self.beacon_count.wrapping_add(1);
        self.beacon_count
    }
}

// ============================================================================
// 10. DEMO TỔNG HỢP: PIVOT + C2 + OBFUSCATION
// ============================================================================

/// Báo cáo tổng kết của một lần chạy demo C2.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct C2DemoReport {
    /// Số tác tử đã đăng ký
    pub agents_registered: usize,
    /// Số nhiệm vụ đã hoàn tất
    pub tasks_completed: usize,
    /// Tổng số byte đã mã hoá trên kênh
    pub bytes_encrypted: usize,
    /// Số lưu lượng đã obfuscate và vẫn giống HTTP/DNS
    pub detection_evasions: usize,
}

/// Chạy trọn vẹn một vòng demo C2: đăng ký tác tử, beacon, giao nhiệm vụ,
/// thực thi trên vỏ lệnh giả lập, trả kết quả và obfuscate lưu lượng.
pub fn run_c2_demo(seed: u64) -> C2DemoReport {
    let mut rng = SplitMix64::new(seed);
    let material = rng.fill_bytes(32);
    let mut key = [0u8; 32];
    key.copy_from_slice(&material);

    let mut server = C2Server::with_cipher(Box::new(XChaCha20Channel::new(key)));
    let mut agent = C2Agent::new("agent-alpha").with_jitter(750);
    let mut shell = MockShell::new();

    server.register(&agent.id, &agent.hostname, &agent.os, &agent.privileges);

    let mut tasks_completed = 0usize;
    let mut bytes_encrypted = 0usize;
    let mut detection_evasions = 0usize;

    let plan: [(ObfuscationMode, &str); 3] = [
        (ObfuscationMode::HttpRequest, "whoami"),
        (ObfuscationMode::DnsQuery, "uname -a"),
        (ObfuscationMode::HttpResponse, "ls"),
    ];

    for (mode, default_command) in plan {
        server.tick();
        server.beacon(&agent.id);
        let _delay = agent.next_beacon_delay(&mut rng);

        // Lệnh thực thi được chọn từ menu bằng bộ sinh tất định theo hạt giống đầu vào.
        let menu = ["whoami", "uname -a", "ls", "id"];
        let command = if default_command == "whoami" {
            menu[(rng.next_u64() % menu.len() as u64) as usize]
        } else {
            default_command
        };

        let task_id = server.enqueue_task(&agent.id, command);
        let request = server.encode_message(&C2Command::TaskRequest, command);
        bytes_encrypted += request.len();

        let obfuscator = TrafficObfuscator::new(mode);
        let wrapped = obfuscator.wrap(&request);
        if obfuscator.looks_like_http(&wrapped) || obfuscator.looks_like_dns(&wrapped) {
            detection_evasions += 1;
        }
        let restored = obfuscator
            .unwrap(&wrapped)
            .expect("tháo lớp vỏ obfuscate phải thành công");

        let (decoded_command, decoded_payload) = server
            .decode_message(&restored)
            .expect("giải mã thông điệp C2 phải thành công");
        assert_eq!(decoded_command, C2Command::TaskRequest);
        assert_eq!(decoded_payload, command);

        let output = {
            let task = server
                .next_task(&agent.id)
                .expect("phải có nhiệm vụ chờ tác tử");
            agent.handle_task(task, &mut shell)
        };
        server.complete_task(task_id, &output);
        tasks_completed += 1;

        let result = server.encode_message(&C2Command::TaskResult, &output);
        bytes_encrypted += result.len();
    }

    C2DemoReport {
        agents_registered: server.sessions().len(),
        tasks_completed,
        bytes_encrypted,
        detection_evasions,
    }
}

// ============================================================================
// 11. CHẠY THỬ MINH HOẠ (MAIN)
// ============================================================================

/// Chuyển một chuỗi địa chỉ thành `SocketAddr` (tiện ích cho phần trình diễn).
fn addr(text: &str) -> SocketAddr {
    text.parse().expect("địa chỉ socket phải hợp lệ")
}

/// Chuyển `count` byte đầu tiên thành chuỗi hex dạng "aa bb cc".
fn hex_prefix(data: &[u8], count: usize) -> String {
    let mut out = String::new();
    for byte in data.iter().take(count) {
        let _ = write!(out, "{byte:02x}");
    }
    out
}

fn main() {
    println!("=== CHƯƠNG 103: XOAY CHIỀU MẠNG & KHUNG CHỈ HUY-KIỂM SOÁT ===");
    println!("    (Pivoting & C2: SSH Tunnels, SOCKS Relay, Mini C2 & Encrypted Channels)");
    println!();

    // ---- 1. Đường hầm SSH ----
    println!("[1] MÔ PHỎNG ĐƯỜNG HẦM SSH (PIVOTING)");
    let mut sim = SshTunnelSimulator::new();
    let local_id = sim.local_forward(addr("127.0.0.1:8080"), addr("10.0.0.5:80"));
    let remote_id = sim.remote_forward(addr("10.0.0.9:3306"), addr("127.0.0.1:3306"));
    let socks_id = sim.dynamic_socks(addr("127.0.0.1:1080"));
    for tunnel in &sim.tunnels {
        println!(
            "    phiên #{:<3} {}",
            tunnel.session_id,
            ssh_command_line(tunnel)
        );
    }
    println!(
        "    -> {} đường hầm hoạt động (phiên local=#{local_id}, remote=#{remote_id}, socks=#{socks_id})",
        sim.active_tunnels()
    );
    sim.close_tunnel(local_id);
    println!(
        "    -> sau khi đóng phiên #{local_id}: còn {} đường hầm hoạt động",
        sim.active_tunnels()
    );
    println!();

    // ---- 2. Chuỗi relay SOCKS ----
    println!("[2] CHUỖI RELAY SOCKS NHIỀU BƯỚC");
    let mut chain = SocksRelayChain::new();
    chain
        .add_hop(addr("10.0.0.1:1080"))
        .add_hop(addr("10.0.0.2:1080"))
        .add_hop(addr("10.0.0.3:1080"));
    let path = chain.resolve_path("victim.internal", 445);
    println!("    đường đi: {}", path.join(" -> "));
    println!(
        "    tổng độ trễ: {:.2} ms qua {} trạm trung gian",
        chain.total_latency_ms(&[12.5, 8.0, 21.25]),
        chain.hop_count()
    );
    println!();

    // ---- 3. Vòng lặp C2 ----
    println!("[3] VÒNG LẶP C2: ĐĂNG KÝ -> BEACON -> NHIỆM VỤ -> KẾT QUẢ");
    let mut server = C2Server::with_cipher(Box::new(XChaCha20Channel::new([7u8; 32])));
    let mut agent = C2Agent::new("agent-alpha").with_jitter(500);
    let mut shell = MockShell::new();
    let session = server.register(&agent.id, &agent.hostname, &agent.os, &agent.privileges);
    println!(
        "    phiên đăng ký: {} / {} / {}",
        session.id, session.hostname, session.os
    );
    server.tick();
    let beacon_at = server.beacon(&agent.id);
    println!(
        "    beacon tại mốc {:?}, số phiên còn sống: {}",
        beacon_at,
        server.active_sessions()
    );

    let task_id = server.enqueue_task(&agent.id, "uname -a");
    let request = server.encode_message(&C2Command::TaskRequest, "uname -a");
    println!(
        "    thông điệp mã hoá ({} byte), 24 byte đầu: {}",
        request.len(),
        hex_prefix(&request, 24)
    );
    let (decoded_command, decoded_payload) = server
        .decode_message(&request)
        .expect("giải mã thông điệp C2 phải thành công");
    println!("    máy chủ giải mã được: {decoded_command:?} / {decoded_payload}");

    let output = {
        let task = server
            .next_task(&agent.id)
            .expect("phải có nhiệm vụ đang chờ tác tử");
        agent.handle_task(task, &mut shell)
    };
    server.complete_task(task_id, &output);
    println!("    kết quả tác tử: {output}");
    println!();

    // ---- 4. Obfuscate lưu lượng ----
    println!("[4] OBFUSCATE LƯU LƯỢNG & NHẬN DIỆN HÌNH DẠNG");
    let obfuscator = TrafficObfuscator::new(ObfuscationMode::HttpRequest);
    let wrapped = obfuscator.wrap(&request);
    let first_line = String::from_utf8_lossy(&wrapped)
        .lines()
        .next()
        .unwrap_or("")
        .to_string();
    println!("    lưu lượng sau obfuscate ({} byte):", wrapped.len());
    println!("      {first_line}");
    println!(
        "    trông giống HTTP: {} | trông giống DNS: {}",
        obfuscator.looks_like_http(&wrapped),
        obfuscator.looks_like_dns(&wrapped)
    );
    let restored = obfuscator
        .unwrap(&wrapped)
        .expect("tháo lớp vỏ obfuscate phải thành công");
    println!("    khôi phục khớp bản gốc: {}", restored == request);
    println!();

    // ---- 5. Demo tổng hợp ----
    println!("[5] DEMO TỔNG HỢP PIVOT + C2 + OBFUSCATION");
    let report = run_c2_demo(2026);
    println!("    tác tử đăng ký       : {}", report.agents_registered);
    println!("    nhiệm vụ hoàn tất    : {}", report.tasks_completed);
    println!("    byte đã mã hoá       : {}", report.bytes_encrypted);
    println!("    lưu lượng né dò quét : {}", report.detection_evasions);
    println!();

    println!("Toàn bộ mô phỏng chạy trên dữ liệu tổng hợp tất định, không chạm hệ thống thật.");
}

// ============================================================================
// 12. BỘ KIỂM THỬ TẤT ĐỊNH (44 BÀI TEST CHỨNG MINH CÁC BẤT BIẾN)
// ============================================================================

#[cfg(test)]
mod tests {
    use super::*;

    /// Chuyển mảng byte thành chuỗi hex liền (tiện ích cho các bài test).
    fn to_hex(data: &[u8]) -> String {
        data.iter().map(|b| format!("{b:02x}")).collect()
    }

    /// Khoá 32 byte dùng chung cho các bài test mã hoá.
    fn test_key() -> [u8; 32] {
        let mut key = [0u8; 32];
        for (i, chunk) in key.chunks_mut(8).enumerate() {
            chunk.copy_from_slice(&(i as u64 * 7 + 11).to_le_bytes());
        }
        key
    }

    // ------------------------------------------------------------------
    // NHÓM 1: TIỆN ÍCH, HÀM BĂM & DERIVATION (TEST 01 - 11)
    // ------------------------------------------------------------------

    #[test]
    fn test_01_splitmix64_deterministic_reproducibility() {
        let mut rng1 = SplitMix64::new(4242);
        let mut rng2 = SplitMix64::new(4242);
        for _ in 0..64 {
            assert_eq!(rng1.next_u64(), rng2.next_u64());
        }
        assert_eq!(
            SplitMix64::new(7).fill_bytes(24),
            SplitMix64::new(7).fill_bytes(24)
        );
    }

    #[test]
    fn test_02_chacha20_rfc8439_block_vector() {
        // RFC 8439 §2.3.2: key = 00..1f, nonce = 00:00:00:09:00:00:00:4a:00:00:00:00, counter = 1
        let key: [u8; 32] = core::array::from_fn(|i| i as u8);
        let nonce: [u8; 12] = [
            0x00, 0x00, 0x00, 0x09, 0x00, 0x00, 0x00, 0x4a, 0x00, 0x00, 0x00, 0x00,
        ];
        let block = chacha20_block(&key, 1, &nonce);
        assert_eq!(
            to_hex(&block),
            "10f1e7e4d13b5915500fdd1fa32071c4c7d1f4c733c068030422aa9ac3d46c4e\
             d2826446079faa0914c2d705d98b02a2b5129cd1de164eb9cbd083e8a2503c4e"
        );
    }

    #[test]
    fn test_03_chacha20_rfc8439_cipher_vector() {
        // RFC 8439 §2.4.2: nonce = 00:00:00:00:00:00:00:4a:00:00:00:00, counter = 1
        let key: [u8; 32] = core::array::from_fn(|i| i as u8);
        let nonce: [u8; 12] = [
            0x00, 0x00, 0x00, 0x00, 0x00, 0x00, 0x00, 0x4a, 0x00, 0x00, 0x00, 0x00,
        ];
        let plaintext = b"Ladies and Gentlemen of the class of '99: If I could offer you only one tip for the future, sunscreen would be it.";
        let ciphertext = chacha20(&key, &nonce, plaintext);
        assert_eq!(ciphertext.len(), plaintext.len());
        assert_eq!(
            to_hex(&ciphertext),
            "6e2e359a2568f98041ba0728dd0d6981e97e7aec1d4360c20a27afccfd9fae0b\
             f91b65c5524733ab8f593dabcd62b3571639d624e65152ab8f530c359f0861d8\
             07ca0dbf500d6a6156a38e088a22b65e52bc514d16ccf806818ce91ab77937365\
             af90bbf74a35be6b40b8eedf2785e42874d"
        );
        // Bản rõ khớp với tiền tố do SPEC_103 ghi (nonce 00:00:00:09:... sinh khối §2.3.2).
        assert!(to_hex(&ciphertext).starts_with("6e2e359a2568f98041ba0728dd0d6981"));
    }

    #[test]
    fn test_04_chacha20_quarter_round_reference_values() {
        // Bộ số kiểm thử kinh điển của RFC 8439 §2.1.1
        let mut state: [u32; 16] = [0; 16];
        state[0] = 0x1111_1111;
        state[1] = 0x0102_0304;
        state[2] = 0x9b8d_6f43;
        state[3] = 0x0123_4567;
        quarter_round(&mut state, 0, 1, 2, 3);
        assert_eq!(state[0], 0xea2a_92f4);
        assert_eq!(state[1], 0xcb1c_f8ce);
        assert_eq!(state[2], 0x4581_472e);
        assert_eq!(state[3], 0x5881_c4bb);
    }

    #[test]
    fn test_05_chacha20_apply_counter_advances() {
        let key = test_key();
        let nonce = [9u8; 12];
        let mut cipher = ChaCha20::new(key, nonce);
        assert_eq!(cipher.counter, CHACHA_INITIAL_COUNTER);
        let first = cipher.apply(&[0u8; 64]);
        assert_eq!(cipher.counter, CHACHA_INITIAL_COUNTER + 1);
        let second = cipher.apply(&[0u8; 64]);
        assert_eq!(cipher.counter, CHACHA_INITIAL_COUNTER + 2);
        assert_ne!(
            first, second,
            "hai khối liên tiếp phải dùng keystream khác nhau"
        );
    }

    #[test]
    fn test_06_chacha20_stream_cipher_symmetry() {
        let key = test_key();
        let nonce = [3u8; 12];
        let data = b"khanh C2 - chuoi ma hoa ChaCha20".to_vec();
        let encrypted = chacha20(&key, &nonce, &data);
        assert_ne!(encrypted, data);
        let decrypted = chacha20(&key, &nonce, &encrypted);
        assert_eq!(decrypted, data);
    }

    #[test]
    fn test_07_sha256_known_answer_vectors() {
        assert_eq!(
            to_hex(&sha256(b"")),
            "e3b0c44298fc1c149afbf4c8996fb92427ae41e4649b934ca495991b7852b855"
        );
        assert_eq!(
            to_hex(&sha256(b"abc")),
            "ba7816bf8f01cfea414140de5dae2223b00361a396177a9cb410ff61f20015ad"
        );
        assert_eq!(
            to_hex(&sha256(
                b"abcdbcdecdefdefgefghfghighijhijkijkljklmklmnlmnomnopnopq"
            )),
            "248d6a61d20638b8e5c026930c3e6039a33ce45964ff2167f6ecedd419db06c1"
        );
    }

    #[test]
    fn test_08_hmac_sha256_rfc4231_vector() {
        // RFC 4231 test case 1
        let key = [0x0bu8; 20];
        assert_eq!(
            to_hex(&hmac_sha256(&key, b"Hi There")),
            "b0344c61d8db38535ca8afceaf0bf12b881dc200c9833da726e9376c2e32cff7"
        );
        // RFC 4231 test case 2
        assert_eq!(
            to_hex(&hmac_sha256(b"Jefe", b"what do ya want for nothing?")),
            "5bdcc146bf60754e6a042426089575c75a003f089d2739839dec58b964ec3843"
        );
    }

    #[test]
    fn test_09_hkdf_sha256_rfc5869_vector() {
        // RFC 5869 Appendix A.1
        let ikm = [0x0bu8; 22];
        let salt: Vec<u8> = (0u8..13).collect();
        let info: Vec<u8> = (0xf0u8..0xfa).collect();
        let okm = hkdf_sha256(&ikm, &salt, &info, 42);
        assert_eq!(okm.len(), 42);
        assert_eq!(
            to_hex(&okm),
            "3cb25f25faacd57a90434f64d0362f2a2d2d0a90cf1a5a4c5db02d56ecc4c5bf\
             34007208d5b887185865"
        );
        // HKDF phải tương đương với trình tự extract -> expand
        let prk = hkdf_extract(&salt, &ikm);
        assert_eq!(okm, hkdf_expand(&prk, &info, 42));
    }

    #[test]
    fn test_10_hkdf_output_length_and_determinism() {
        let ikm = b"khoa-bi-mat-tho".to_vec();
        let salt = b"muoi".to_vec();
        let info = b"chuc-nang-huong-dan".to_vec();
        for len in [1usize, 32, 64, 100] {
            let okm = hkdf_sha256(&ikm, &salt, &info, len);
            assert_eq!(okm.len(), len);
            assert_eq!(okm, hkdf_sha256(&ikm, &salt, &info, len));
        }
        // Hai thông tin ngữ cảnh khác nhau phải cho khoá khác nhau
        let a = hkdf_sha256(&ikm, &salt, &info, 32);
        let b = hkdf_sha256(&ikm, &salt, b"khac", 32);
        assert_ne!(a, b);
    }

    #[test]
    fn test_11_hchacha20_deterministic_subkey() {
        let key = test_key();
        let nonce: [u8; 16] = core::array::from_fn(|i| i as u8);
        let subkey_a = hchacha20(&key, &nonce);
        let subkey_b = hchacha20(&key, &nonce);
        assert_eq!(subkey_a, subkey_b);
        assert_ne!(subkey_a, key, "khóa phụ phải khác khóa chính");
        let other: [u8; 16] = core::array::from_fn(|i| (i as u8).wrapping_add(1));
        assert_ne!(subkey_a, hchacha20(&key, &other));
    }

    // ------------------------------------------------------------------
    // NHÓM 2: KÊNH MÃ HOÁ XCHACHA20 (TEST 12 - 16)
    // ------------------------------------------------------------------

    #[test]
    fn test_12_xchacha20_seal_open_roundtrip() {
        let mut channel = XChaCha20Channel::new(test_key());
        let plaintext = b"Agent beacon: host-agent-alpha up".to_vec();
        let sealed = channel.seal(&plaintext);
        assert_eq!(sealed.len(), plaintext.len() + XCHACHA_NONCE_LEN);
        let opened = channel.open(&sealed).expect("mo bam ma phai thanh cong");
        assert_eq!(opened, plaintext);
    }

    #[test]
    fn test_13_xchacha20_nonce_counter_increments() {
        let mut channel = XChaCha20Channel::new(test_key());
        let before = channel.nonce_counter;
        let n1 = channel.next_nonce();
        let n2 = channel.next_nonce();
        assert_ne!(n1, n2, "hai nonce liên tiếp phai khac nhau");
        assert_eq!(channel.nonce_counter, before + 2);
        assert_eq!(&n1[4..12], &before.to_le_bytes());
    }

    #[test]
    fn test_14_xchacha20_channel_cipher_trait_roundtrip() {
        let cipher: Box<dyn ChannelCipher> = Box::new(XChaCha20Channel::new(test_key()));
        let plaintext = b"TaskResult: uname -a output".to_vec();
        let ciphertext = cipher.encrypt(&plaintext);
        assert_eq!(cipher.nonce_len(), XCHACHA_NONCE_LEN);
        assert_eq!(cipher.decrypt(&ciphertext), plaintext);
        // Hai bản rõ khác nhau không được dùng lại cùng một keystream
        let other = cipher.encrypt(b"khac mot ban ro khac");
        assert_ne!(ciphertext, other);
    }

    #[test]
    fn test_15_xchacha20_rejects_short_ciphertext() {
        let channel = XChaCha20Channel::new(test_key());
        assert!(channel.open(&[0u8; 4]).is_err());
        let cipher: Box<dyn ChannelCipher> = Box::new(XChaCha20Channel::new(test_key()));
        assert!(cipher.decrypt(&[0u8; 2]).is_empty());
    }

    // ------------------------------------------------------------------
    // NHÓM 3: ECDH X25519 & DẪN KHOÁ PHIÊN (TEST 16 - 21)
    // ------------------------------------------------------------------

    #[test]
    fn test_16_x25519_rfc7748_test_vector() {
        // RFC 7748 §5.2: scalar a546..., u = e6db... -> c3da...
        let scalar: [u8; 32] =
            hex32("a546e36bf0527c9d3b16154b82465edd62144c0ac1fc5a18506a2244ba449ac4");
        let u: [u8; 32] = hex32("e6db6867583030db3594c1a424b15f7c726624ec26b3353b10a903a6d0ab1c4c");
        let expected: [u8; 32] =
            hex32("c3da55379de9c6908e94ea4df28d084f32eccf03491c71f754b4075577a28552");
        assert_eq!(x25519(&scalar, &u), expected);
    }

    #[test]
    fn test_17_x25519_rfc7748_diffie_hellman_vector() {
        // RFC 7748 §6.1: vector trao đổi khoá Diffie-Hellman chuẩn
        let alice_sk: [u8; 32] =
            hex32("77076d0a7318a57d3c16c17251b26645df4c2f87ebc0992ab177fba51db92c2a");
        let bob_sk: [u8; 32] =
            hex32("5dab087e624a8a4b79e17f8b83800ee66f3bb1292618b6fd1c2f8b27ff88e0eb");
        let alice_pk = X25519::from_bytes(&alice_sk).public_key();
        let bob_pk = X25519::from_bytes(&bob_sk).public_key();
        assert_eq!(
            to_hex(&alice_pk),
            "8520f0098930a754748b7ddcb43ef75a0dbf3a0d26381af4eba4a98eaa9b4e6a"
        );
        assert_eq!(
            to_hex(&bob_pk),
            "de9edb7d7b7dc1b4d35b61c2ece435373f8343c85b78674dadfc7e146f882b4f"
        );
        let shared_a = X25519::from_bytes(&alice_sk).diffie_hellman(&bob_pk);
        let shared_b = X25519::from_bytes(&bob_sk).diffie_hellman(&alice_pk);
        assert_eq!(shared_a, shared_b);
        assert_eq!(
            to_hex(&shared_a),
            "4a5d9d5ba4ce2de1728e3bf480350f25e07e21c947d19e3376f09b3c1e161742"
        );
    }

    #[test]
    fn test_18_x25519_secret_clamping() {
        let mut raw = [0xFFu8; 32];
        let key = X25519::from_bytes(&raw);
        assert_eq!(key.secret[0] & 0x07, 0, "3 bit thap nhat phai bang 0");
        assert_eq!(key.secret[31] & 0x80, 0, "bit cao nhat phai bang 0");
        assert_eq!(
            key.secret[31] & 0x40,
            0x40,
            "bit thu hai cao nhat phai bang 1"
        );
        raw[0] = 0x09;
        raw[31] = 0x09;
        let key2 = X25519::from_bytes(&raw);
        assert_eq!(key2.secret[0], 0x08);
        assert_eq!(key2.secret[31], 0x49);
    }

    #[test]
    fn test_19_x25519_public_key_determinism() {
        let mut rng1 = SplitMix64::new(99);
        let mut rng2 = SplitMix64::new(99);
        let mut rng3 = SplitMix64::new(100);
        let a = X25519::generate(&mut rng1).public_key();
        let b = X25519::generate(&mut rng2).public_key();
        let c = X25519::generate(&mut rng3).public_key();
        assert_eq!(a, b);
        assert_ne!(a, c);
    }

    #[test]
    fn test_20_x25519_ecdh_shared_secret_agreement() {
        let mut rng_a = SplitMix64::new(2024);
        let mut rng_b = SplitMix64::new(2025);
        let alice = X25519::generate(&mut rng_a);
        let bob = X25519::generate(&mut rng_b);
        let shared_a = alice.diffie_hellman(&bob.public_key());
        let shared_b = bob.diffie_hellman(&alice.public_key());
        assert_eq!(
            shared_a, shared_b,
            "hai ben phai suy ra cung mot bi mat chung"
        );
        assert!(
            shared_a.iter().any(|&b| b != 0),
            "bi mat chung khong duoc bang toan 0"
        );
    }

    #[test]
    fn test_21_ecdh_hkdf_session_key_agreement() {
        let mut rng_a = SplitMix64::new(555);
        let mut rng_b = SplitMix64::new(777);
        let alice = X25519::generate(&mut rng_a);
        let bob = X25519::generate(&mut rng_b);
        let shared_a = alice.diffie_hellman(&bob.public_key());
        let shared_b = bob.diffie_hellman(&alice.public_key());
        assert_eq!(shared_a, shared_b);

        let salt = b"learn-rs-ch103";
        let info = b"c2-session-key";
        let key_a = hkdf_sha256(&shared_a, salt, info, 32);
        let key_b = hkdf_sha256(&shared_b, salt, info, 32);
        assert_eq!(key_a, key_b, "khoa phien phai giong nhau o hai ben");

        let mut session_key_a = [0u8; 32];
        let mut session_key_b = [0u8; 32];
        session_key_a.copy_from_slice(&key_a);
        session_key_b.copy_from_slice(&key_b);
        let mut session_a = XChaCha20Channel::new(session_key_a);
        let session_b = XChaCha20Channel::new(session_key_b);
        let secret = b"khoa kien truc tiep qua kenh C2";
        let sealed = session_a.seal(secret);
        assert_eq!(session_b.open(&sealed).unwrap(), secret);
    }

    // ------------------------------------------------------------------
    // NHÓM 4: BASE64 (TEST 22 - 24)
    // ------------------------------------------------------------------

    #[test]
    fn test_22_base64_encode_known_vectors() {
        assert_eq!(base64_encode(b""), "");
        assert_eq!(base64_encode(b"f"), "Zg==");
        assert_eq!(base64_encode(b"fo"), "Zm8=");
        assert_eq!(base64_encode(b"foo"), "Zm9v");
        assert_eq!(base64_encode(b"foob"), "Zm9vYg==");
        assert_eq!(base64_encode(b"fooba"), "Zm9vYmE=");
        assert_eq!(base64_encode(b"foobar"), "Zm9vYmFy");
        assert_eq!(base64_encode(b"Man"), "TWFu");
    }

    #[test]
    fn test_23_base64_decode_roundtrip() {
        let mut rng = SplitMix64::new(31337);
        for len in [0usize, 1, 2, 3, 5, 16, 31, 64, 255] {
            let data = rng.fill_bytes(len);
            let encoded = base64_encode(&data);
            assert_eq!(encoded.len() % 4, 0);
            let decoded = base64_decode(&encoded).expect("giai ma Base64 phai thanh cong");
            assert_eq!(decoded, data);
        }
    }

    #[test]
    fn test_24_base64_decode_rejects_invalid_input() {
        assert!(base64_decode("A").is_err(), "do dai khong chia het cho 4");
        assert!(base64_decode("A@==").is_err(), "ky tu khong hop le");
        assert!(base64_decode("AB=C").is_err(), "ky tu dem o giua du lieu");
    }

    // ------------------------------------------------------------------
    // NHÓM 5: ĐƯỜNG HẦM SSH & CHUỖI SOCKS (TEST 25 - 31)
    // ------------------------------------------------------------------

    #[test]
    fn test_25_ssh_local_forward_command_line() {
        let mut sim = SshTunnelSimulator::new();
        sim.local_forward(addr("127.0.0.1:8080"), addr("10.0.0.5:80"));
        let command = ssh_command_line(&sim.tunnels[0]);
        assert_eq!(command, "ssh -L 8080:10.0.0.5:80 user@jump");
        assert!(command.contains("-L "));
        assert!(!command.contains("-R "));
    }

    #[test]
    fn test_26_ssh_remote_forward_command_line() {
        let mut sim = SshTunnelSimulator::new();
        sim.remote_forward(addr("10.0.0.9:3306"), addr("127.0.0.1:3306"));
        let command = ssh_command_line(&sim.tunnels[0]);
        assert_eq!(command, "ssh -R 3306:127.0.0.1:3306 user@jump");
        assert!(command.contains("-R "));
    }

    #[test]
    fn test_27_ssh_dynamic_socks_command_line() {
        let mut sim = SshTunnelSimulator::new();
        sim.dynamic_socks(addr("127.0.0.1:1080"));
        let command = ssh_command_line(&sim.tunnels[0]);
        assert_eq!(command, "ssh -D 1080 user@jump");
        assert_eq!(sim.tunnels[0].tunnel_type, TunnelType::DynamicSocks);
    }

    #[test]
    fn test_28_ssh_tunnel_simulator_open_close_lifecycle() {
        let mut sim = SshTunnelSimulator::new();
        let a = sim.local_forward(addr("127.0.0.1:8080"), addr("10.0.0.5:80"));
        let b = sim.remote_forward(addr("10.0.0.9:3306"), addr("127.0.0.1:3306"));
        let c = sim.dynamic_socks(addr("127.0.0.1:1080"));
        assert_eq!((a, b, c), (1, 2, 3));
        assert_eq!(sim.active_tunnels(), 3);
        assert!(sim.close_tunnel(b));
        assert_eq!(sim.active_tunnels(), 2);
        assert!(
            !sim.close_tunnel(9999),
            "dong phiend khong ton tai phai tra false"
        );
        assert!(sim.describe().contains("REMOTE"));
    }

    #[test]
    fn test_29_socks_chain_resolve_path() {
        let mut chain = SocksRelayChain::new();
        chain
            .add_hop(addr("10.0.0.1:1080"))
            .add_hop(addr("10.0.0.2:1080"));
        assert_eq!(chain.hop_count(), 2);
        let path = chain.resolve_path("victim.internal", 445);
        assert_eq!(
            path,
            vec![
                "client".to_string(),
                "10.0.0.1:1080".to_string(),
                "10.0.0.2:1080".to_string(),
                "victim.internal:445".to_string(),
            ]
        );
    }

    #[test]
    fn test_30_socks_chain_total_latency() {
        let mut chain = SocksRelayChain::new();
        chain
            .add_hop(addr("10.0.0.1:1080"))
            .add_hop(addr("10.0.0.2:1080"))
            .add_hop(addr("10.0.0.3:1080"));
        let total = chain.total_latency_ms(&[12.5, 8.0, 21.25]);
        assert!((total - 41.75).abs() < 1e-9);
        // Trạm không có phép đo được tính bằng 0
        let partial = chain.total_latency_ms(&[5.0]);
        assert!((partial - 5.0).abs() < 1e-9);
    }

    #[test]
    fn test_31_simulate_chain_matches_resolve_path() {
        let hops = vec![addr("10.0.0.1:1080"), addr("10.0.0.2:1080")];
        let chain = simulate_chain(&hops, "victim.internal:445");
        assert_eq!(chain.len(), 4);
        assert_eq!(chain[0], "client");
        assert_eq!(chain[3], "victim.internal:445");

        let mut relay = SocksRelayChain::new();
        for hop in &hops {
            relay.add_hop(*hop);
        }
        assert_eq!(relay.resolve_path("victim.internal", 445), chain);
    }

    // ------------------------------------------------------------------
    // NHÓM 6: OBFUSCATION LƯU LƯỢNG (TEST 32 - 37)
    // ------------------------------------------------------------------

    #[test]
    fn test_32_obfuscator_http_request_roundtrip_and_fingerprint() {
        let obfuscator = TrafficObfuscator::new(ObfuscationMode::HttpRequest);
        let payload = b"\x00\x01\x02binary c2 payload";
        let wrapped = obfuscator.wrap(payload);
        let text = String::from_utf8_lossy(&wrapped);
        assert!(text.starts_with("GET "));
        assert!(text.contains("HTTP/1.1\r\n"));
        assert!(obfuscator.looks_like_http(&wrapped));
        assert!(!obfuscator.looks_like_dns(&wrapped));
        assert_eq!(obfuscator.unwrap(&wrapped).unwrap(), payload);
    }

    #[test]
    fn test_33_obfuscator_http_response_roundtrip() {
        let obfuscator = TrafficObfuscator::new(ObfuscationMode::HttpResponse);
        let payload = b"task-result-payload";
        let wrapped = obfuscator.wrap(payload);
        let text = String::from_utf8_lossy(&wrapped);
        assert!(text.starts_with("HTTP/1.1 200 OK\r\n"));
        assert!(obfuscator.looks_like_http(&wrapped));
        assert_eq!(obfuscator.unwrap(&wrapped).unwrap(), payload);
    }

    #[test]
    fn test_34_obfuscator_dns_query_roundtrip_and_fingerprint() {
        let obfuscator = TrafficObfuscator::new(ObfuscationMode::DnsQuery);
        let payload = b"dns-tunnel-c2-beacon";
        let wrapped = obfuscator.wrap(payload);
        assert!(obfuscator.looks_like_dns(&wrapped));
        assert!(!obfuscator.looks_like_http(&wrapped));
        assert_eq!(obfuscator.unwrap(&wrapped).unwrap(), payload);
    }

    #[test]
    fn test_35_obfuscator_dns_response_roundtrip() {
        let obfuscator = TrafficObfuscator::new(ObfuscationMode::DnsResponse);
        let payload = b"dns-tunnel-task-result";
        let wrapped = obfuscator.wrap(payload);
        assert!(obfuscator.looks_like_dns(&wrapped));
        assert_eq!(obfuscator.unwrap(&wrapped).unwrap(), payload);
    }

    #[test]
    fn test_36_obfuscator_raw_mode_passthrough() {
        let obfuscator = TrafficObfuscator::new(ObfuscationMode::Raw);
        let payload = b"raw-bytes-untouched";
        let wrapped = obfuscator.wrap(payload);
        assert_eq!(wrapped, payload);
        assert_eq!(obfuscator.unwrap(&wrapped).unwrap(), payload);
    }

    #[test]
    fn test_37_obfuscator_rejects_mismatched_mode() {
        let http = TrafficObfuscator::new(ObfuscationMode::HttpRequest);
        let dns = TrafficObfuscator::new(ObfuscationMode::DnsQuery);
        let wrapped = http.wrap(b"khong phai DNS");
        assert!(
            dns.unwrap(&wrapped).is_err(),
            "DNS mode phai tu choi goi HTTP"
        );
        let raw = TrafficObfuscator::new(ObfuscationMode::Raw);
        assert_eq!(raw.unwrap(&wrapped).unwrap(), wrapped);
        assert!(http.unwrap(b"GET /x HTTP/1.1\r\n\r\n").is_err());
    }

    // ------------------------------------------------------------------
    // NHÓM 7: KHUNG C2 (TEST 38 - 44)
    // ------------------------------------------------------------------

    #[test]
    fn test_38_c2_register_and_session_listing() {
        let mut server = C2Server::new();
        let session = server.register("agent-b", "host-b", "Linux", "root");
        server.register("agent-a", "host-a", "Linux", "user");
        assert_eq!(session.id, "agent-b");
        assert!(session.alive);
        assert_eq!(server.sessions().len(), 2);
        assert_eq!(
            server.sessions()[0].id,
            "agent-a",
            "danh sach phai sap xep dinh danh"
        );
        assert_eq!(server.active_sessions(), 2);
    }

    #[test]
    fn test_39_c2_beacon_updates_last_seen() {
        let mut server = C2Server::new();
        server.register("agent-a", "host-a", "Linux", "user");
        server.tick();
        server.tick();
        let at = server
            .beacon("agent-a")
            .expect("beacon cua tac tu da dang ky");
        assert_eq!(at, 2);
        assert_eq!(server.sessions()[0].last_seen, 2);
        assert!(server.beacon("agent-khong-ton-tai").is_none());
    }

    #[test]
    fn test_40_c2_task_queue_roundtrip() {
        let mut server = C2Server::new();
        server.register("agent-a", "host-a", "Linux", "user");
        let task_id = server.enqueue_task("agent-a", "whoami");
        assert_eq!(task_id, 1);
        {
            let task = server.next_task("agent-a").expect("phai co nhieu vu");
            assert_eq!(task.command, "whoami");
            assert!(!task.completed);
            assert_eq!(task.result, None);
        }
        assert!(server.complete_task(task_id, "agent"));
        assert!(server.next_task("agent-a").is_none());
        assert!(!server.complete_task(999, "x"));
        let task = &server.tasks[0];
        assert!(task.completed);
        assert_eq!(task.result.as_deref(), Some("agent"));
    }

    #[test]
    fn test_41_c2_message_encode_decode_plain() {
        let server = C2Server::new();
        let message = server.encode_message(&C2Command::TaskRequest, "uname -a");
        let (command, payload) = server.decode_message(&message).expect("giai ma phai ok");
        assert_eq!(command, C2Command::TaskRequest);
        assert_eq!(payload, "uname -a");
        assert!(server.decode_message(&[]).is_err());
        assert!(
            server
                .decode_message(&[0xFF, 0, 0, 0, 0, 0, 0, 0, 0])
                .is_err()
        );
    }

    #[test]
    fn test_42_c2_message_encode_decode_with_cipher() {
        let server = C2Server::with_cipher(Box::new(XChaCha20Channel::new(test_key())));
        let message = server.encode_message(&C2Command::ShellOutput, "Linux mock-agent-host");
        assert!(message.len() > XCHACHA_NONCE_LEN);
        let (command, payload) = server.decode_message(&message).expect("giai ma phai ok");
        assert_eq!(command, C2Command::ShellOutput);
        assert_eq!(payload, "Linux mock-agent-host");
        // Bản rõ đã mã hoá không được chứa chuỗi rõ của payload
        assert!(!message.windows(6).any(|w| w == b"Linux "));
    }

    #[test]
    fn test_43_c2_agent_task_execution_and_self_destruct() {
        let mut agent = C2Agent::new("agent-alpha").with_jitter(500);
        let mut shell = MockShell::new();
        let task = Task {
            id: 1,
            agent_id: agent.id.clone(),
            command: "whoami".to_string(),
            issued_at: 0,
            completed: false,
            result: None,
        };
        let output = agent.handle_task(&task, &mut shell);
        assert_eq!(output, "agent");
        assert_eq!(shell.history.len(), 1);
        assert_eq!(shell.history[0].0, "whoami");

        let delay = agent.next_beacon_delay(&mut SplitMix64::new(9));
        assert!(delay >= agent.beacon_interval_ms);
        assert!(delay <= agent.beacon_interval_ms + agent.jitter_ms);

        agent.self_destruct();
        assert!(agent.destroyed);
        let refused = agent.handle_task(&task, &mut shell);
        assert_eq!(refused, kill_signal());
        assert_eq!(refused, "KILL");
        assert_eq!(agent.trace().len(), 3);
    }

    #[test]
    fn test_44_c2_demo_report_and_determinism() {
        let report = run_c2_demo(2026);
        assert_eq!(report.agents_registered, 1);
        assert_eq!(report.tasks_completed, 3);
        assert!(report.bytes_encrypted > 0);
        assert_eq!(report.detection_evasions, 3);
        // Cùng hạt giống phải cho cùng kết quả
        assert_eq!(run_c2_demo(2026), report);
        assert_ne!(run_c2_demo(2027), report);
    }

    /// Đổi chuỗi hex 64 ký tự thành mảng 32 byte.
    fn hex32(text: &str) -> [u8; 32] {
        let mut out = [0u8; 32];
        let bytes = text.as_bytes();
        for (i, chunk) in bytes.as_chunks::<2>().0.iter().enumerate() {
            let hi = (chunk[0] as char).to_digit(16).expect("hex hop le") as u8;
            let lo = (chunk[1] as char).to_digit(16).expect("hex hop le") as u8;
            out[i] = (hi << 4) | lo;
        }
        out
    }
}
