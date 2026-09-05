#![allow(dead_code)]
//! Chương 73 — Nói chuyện với EVM bằng Rust: Keccak-256, mã hoá ABI, chữ ký hàm,
//! mã hoá RLP và cấu trúc giao dịch EIP-1559.
//!
//! Đây là lõi của hệ sinh thái [alloy-rs](https://github.com/alloy-rs) — bộ thư
//! viện Ethereum bằng Rust. Ta cài lại từ đầu để thấy macro `sol!` thật ra chỉ
//! sinh ra mã làm đúng những việc dưới đây.

// ============================================================================
// 1. KECCAK-256 — hàm băm của Ethereum
// ============================================================================
// Chú ý: Ethereum dùng Keccak-256 BẢN GỐC (đệ trình SHA-3), KHÔNG phải SHA3-256
// đã chuẩn hoá. Hai hàm chỉ khác nhau đúng MỘT byte đệm (0x01 so với 0x06),
// nhưng cho ra kết quả hoàn toàn khác. Nhầm lẫn này làm hỏng vô số thư viện.

const RC: [u64; 24] = [
    0x0000000000000001, 0x0000000000008082, 0x800000000000808a, 0x8000000080008000,
    0x000000000000808b, 0x0000000080000001, 0x8000000080008081, 0x8000000000008009,
    0x000000000000008a, 0x0000000000000088, 0x0000000080008009, 0x000000008000000a,
    0x000000008000808b, 0x800000000000008b, 0x8000000000008089, 0x8000000000008003,
    0x8000000000008002, 0x8000000000000080, 0x000000000000800a, 0x800000008000000a,
    0x8000000080008081, 0x8000000000008080, 0x0000000080000001, 0x8000000080008008,
];
const ROTC: [u32; 24] = [1, 3, 6, 10, 15, 21, 28, 36, 45, 55, 2, 14,
                         27, 41, 56, 8, 25, 43, 62, 18, 39, 61, 20, 44];
const PIL: [usize; 24] = [10, 7, 11, 17, 18, 3, 5, 16, 8, 21, 24, 4,
                          15, 23, 19, 13, 12, 2, 20, 14, 22, 9, 6, 1];

/// Hoán vị Keccak-f[1600] — 24 vòng trên trạng thái 5×5 lane 64-bit.
fn keccak_f(a: &mut [u64; 25]) {
    let mut bc = [0u64; 5];
    for round in 0..24 {
        // θ (theta): trộn mỗi cột với hai cột lân cận
        for x in 0..5 {
            bc[x] = a[x] ^ a[x + 5] ^ a[x + 10] ^ a[x + 15] ^ a[x + 20];
        }
        for x in 0..5 {
            let t = bc[(x + 4) % 5] ^ bc[(x + 1) % 5].rotate_left(1);
            for y in (0..25).step_by(5) { a[y + x] ^= t; }
        }
        // ρ (rho) + π (pi): xoay từng lane rồi hoán vị vị trí
        let mut t = a[1];
        for i in 0..24 {
            let j = PIL[i];
            let tmp = a[j];
            a[j] = t.rotate_left(ROTC[i]);
            t = tmp;
        }
        // χ (chi): phi tuyến — đây là bước DUY NHẤT không tuyến tính
        for y in (0..25).step_by(5) {
            for x in 0..5 { bc[x] = a[y + x]; }
            for x in 0..5 { a[y + x] ^= (!bc[(x + 1) % 5]) & bc[(x + 2) % 5]; }
        }
        // ι (iota): phá đối xứng bằng hằng số vòng
        a[0] ^= RC[round];
    }
}

/// Cấu trúc "bọt biển": hút dữ liệu vào theo từng khối `RATE` byte, rồi vắt ra.
pub fn keccak256(data: &[u8]) -> [u8; 32] {
    const RATE: usize = 136; // 1600 bit − 2×256 bit dung lượng = 1088 bit
    let mut a = [0u64; 25];
    let mut count = Vec::with_capacity(data.len() + RATE);
    count.extend_from_slice(data);
    // Đệm pad10*1 với byte miền 0x01 — CHỖ NÀY khác SHA3-256 (dùng 0x06)
    count.push(0x01);
    while count.len() % RATE != 0 { count.push(0x00); }
    let n = count.len();
    count[n - 1] |= 0x80;

    for khoi in count.chunks(RATE) {
        for (i, tu) in khoi.chunks(8).enumerate() {
            a[i] ^= u64::from_le_bytes(tu.try_into().unwrap());
        }
        keccak_f(&mut a);
    }
    let mut ra = [0u8; 32];
    for i in 0..4 { ra[i * 8..i * 8 + 8].copy_from_slice(&a[i].to_le_bytes()); }
    ra
}

pub fn hex(b: &[u8]) -> String { b.iter().map(|x| format!("{:02x}", x)).collect() }

// ============================================================================
// 2. CHỮ KÝ HÀM — 4 byte quyết định EVM gọi hàm nào
// ============================================================================

/// `selector("transfer(address,uint256)")` = 0xa9059cbb — con số mà bất kỳ ai
/// từng đọc log Ethereum đều đã thấy hàng nghìn lần.
///
/// Chỉ 4 byte nghĩa là VA CHẠM CÓ THẬT: xác suất hai hàm khác nhau trùng
/// chữ ký chỉ khoảng 1/2³². Đã có người cố tình tìm hàm trùng để đánh lừa
/// giao diện ví — đó là lý do ví hiện đại hiển thị cả chữ ký đầy đủ.
pub fn selector(period: &str) -> [u8; 4] {
    let b = keccak256(period.as_bytes());
    [b[0], b[1], b[2], b[3]]
}

/// Chủ đề sự kiện (topic0) dùng cả 32 byte, nên an toàn hơn hẳn.
pub fn event_topic(period: &str) -> [u8; 32] { keccak256(period.as_bytes()) }

// ============================================================================
// 3. MÃ HOÁ ABI — quy tắc xếp tham số thành các ô 32 byte
// ============================================================================

pub type Address = [u8; 20];

#[derive(Debug, Clone, PartialEq)]
pub enum AbiValue {
    Uint(u128),
    Int(i128),
    Bool(bool),
    Address(Address),
    Bytes32([u8; 32]),
    // --- kiểu ĐỘNG: chỉ ghi con trỏ vào phần đầu, dữ liệu nằm ở đuôi ---
    Bytes(Vec<u8>),
    Chuoi(String),
    MangUint(Vec<u128>),
}

impl AbiValue {
    pub fn la_dong(&self) -> bool {
        matches!(self, AbiValue::Bytes(_) | AbiValue::Chuoi(_) | AbiValue::MangUint(_))
    }

    fn o_32(v: u128) -> [u8; 32] {
        let mut o = [0u8; 32];
        o[16..].copy_from_slice(&v.to_be_bytes()); // căn PHẢI
        o
    }

    /// Phần cố định: kiểu tĩnh ghi thẳng giá trị, kiểu động ghi con trỏ (điền sau).
    fn header(&self) -> [u8; 32] {
        match self {
            AbiValue::Uint(v) => Self::o_32(*v),
            AbiValue::Bool(b) => Self::o_32(*b as u128),
            AbiValue::Int(v) => {
                // Số âm dùng bù hai và MỞ RỘNG DẤU bằng 0xFF, không phải 0x00
                let mut o = if *v < 0 { [0xFFu8; 32] } else { [0u8; 32] };
                o[16..].copy_from_slice(&(*v as u128).to_be_bytes());
                o
            }
            AbiValue::Address(a) => {
                let mut o = [0u8; 32];
                o[12..].copy_from_slice(a); // 20 byte căn phải trong ô 32 byte
                o
            }
            AbiValue::Bytes32(b) => *b,
            _ => [0u8; 32], // kiểu động: chỗ này sẽ bị ghi đè bằng con trỏ
        }
    }

    /// Phần đuôi cho kiểu động: độ dài rồi tới dữ liệu, đệm cho tròn 32 byte.
    fn part_below(&self) -> Vec<u8> {
        match self {
            AbiValue::Bytes(b) => {
                let mut v = Self::o_32(b.len() as u128).to_vec();
                v.extend_from_slice(b);
                while v.len() % 32 != 0 { v.push(0); }
                v
            }
            AbiValue::Chuoi(s) => AbiValue::Bytes(s.as_bytes().to_vec()).part_below(),
            AbiValue::MangUint(m) => {
                let mut v = Self::o_32(m.len() as u128).to_vec();
                for x in m { v.extend_from_slice(&Self::o_32(*x)); }
                v
            }
            _ => Vec::new(),
        }
    }
}

/// Mã hoá danh sách tham số theo đúng đặc tả ABI của Solidity.
pub fn abi_encode(cac_gt: &[AbiValue]) -> Vec<u8> {
    let first_size = cac_gt.len() * 32;
    let mut first: Vec<u8> = Vec::with_capacity(first_size);
    let mut below: Vec<u8> = Vec::new();

    for gt in cac_gt {
        if gt.la_dong() {
            // Con trỏ tính từ ĐẦU vùng tham số, không phải từ đầu calldata.
            // Nhầm gốc toạ độ ở đây là lỗi ABI phổ biến nhất.
            let pointer = first_size + below.len();
            first.extend_from_slice(&AbiValue::o_32(pointer as u128));
            below.extend_from_slice(&gt.part_below());
        } else {
            first.extend_from_slice(&gt.header());
        }
    }
    first.extend_from_slice(&below);
    first
}

/// Dựng calldata hoàn chỉnh: 4 byte chữ ký hàm + tham số đã mã hoá.
pub fn dung_calldata(period: &str, cac_gt: &[AbiValue]) -> Vec<u8> {
    let mut v = selector(period).to_vec();
    v.extend_from_slice(&abi_encode(cac_gt));
    v
}

/// Giải mã ngược một tham số `uint256` ở vị trí `chi_so` (dùng để đọc kết quả).
pub fn doc_uint(data: &[u8], chi_so: usize) -> Option<u128> {
    let d = data.get(chi_so * 32..chi_so * 32 + 32)?;
    // 16 byte cao phải bằng 0, nếu không thì giá trị vượt u128
    if d[..16].iter().any(|&b| b != 0) { return None; }
    Some(u128::from_be_bytes(d[16..].try_into().ok()?))
}

pub fn read_address(data: &[u8], chi_so: usize) -> Option<Address> {
    let d = data.get(chi_so * 32..chi_so * 32 + 32)?;
    if d[..12].iter().any(|&b| b != 0) { return None; } // 12 byte đệm phải là 0
    d[12..].try_into().ok()
}

// ============================================================================
// 4. MÃ HOÁ RLP — định dạng tuần tự hoá của Ethereum
// ============================================================================

#[derive(Debug, Clone, PartialEq)]
pub enum Rlp { Chuoi(Vec<u8>), DanhSach(Vec<Rlp>) }

impl Rlp {
    pub fn encode(&self) -> Vec<u8> {
        match self {
            Rlp::Chuoi(b) => {
                if b.len() == 1 && b[0] < 0x80 {
                    b.clone() // byte đơn nhỏ tự mã hoá chính nó
                } else {
                    let mut v = Self::prefix(b.len(), 0x80);
                    v.extend_from_slice(b);
                    v
                }
            }
            Rlp::DanhSach(list) => {
                let mut than = Vec::new();
                for x in list { than.extend_from_slice(&x.encode()); }
                let mut v = Self::prefix(than.len(), 0xC0);
                v.extend_from_slice(&than);
                v
            }
        }
    }

    fn prefix(length: usize, root: u8) -> Vec<u8> {
        if length < 56 {
            vec![root + length as u8]
        } else {
            // Độ dài dài: ghi độ-dài-của-độ-dài rồi tới độ dài
            let b = length.to_be_bytes();
            let bo_qua = b.iter().position(|&x| x != 0).unwrap();
            let mut v = vec![root + 55 + (b.len() - bo_qua) as u8];
            v.extend_from_slice(&b[bo_qua..]);
            v
        }
    }

    /// Số nguyên trong RLP dùng big-endian KHÔNG có số 0 thừa ở đầu.
    /// Số 0 mã hoá thành chuỗi RỖNG, không phải byte 0x00 — điểm hay bị sai.
    pub fn numerator(v: u128) -> Rlp {
        if v == 0 { return Rlp::Chuoi(vec![]); }
        let b = v.to_be_bytes();
        let bo_qua = b.iter().position(|&x| x != 0).unwrap();
        Rlp::Chuoi(b[bo_qua..].to_vec())
    }
}

// ============================================================================
// 5. GIAO DỊCH EIP-1559
// ============================================================================

#[derive(Debug, Clone, PartialEq)]
pub struct Tx1559 {
    pub chain_id: u64,
    pub nonce: u64,           // nonce
    pub max_priority_fee: u128, // tiền "boa" cho người xây khối
    pub max_fee: u128,         // trần tổng phí mỗi đơn vị gas
    pub gas_limit: u64,
    pub den: Option<Address>,      // None = tạo hợp đồng mới
    pub value: u128,
    pub data: Vec<u8>,
}

impl Tx1559 {
    /// Tải trọng để ký: 0x02 || rlp([...]). Byte 0x02 là "loại giao dịch",
    /// thêm vào từ EIP-2718 để chuỗi phân biệt được các định dạng khác nhau.
    pub fn load_in_period(&self) -> Vec<u8> {
        let list = Rlp::DanhSach(vec![
            Rlp::numerator(self.chain_id as u128),
            Rlp::numerator(self.nonce as u128),
            Rlp::numerator(self.max_priority_fee),
            Rlp::numerator(self.max_fee),
            Rlp::numerator(self.gas_limit as u128),
            match self.den { Some(a) => Rlp::Chuoi(a.to_vec()), None => Rlp::Chuoi(vec![]) },
            Rlp::numerator(self.value),
            Rlp::Chuoi(self.data.clone()),
            Rlp::DanhSach(vec![]), // danh sách truy cập (EIP-2930), để trống
        ]);
        let mut v = vec![0x02];
        v.extend_from_slice(&list.encode());
        v
    }

    pub fn id_hash_ky(&self) -> [u8; 32] { keccak256(&self.load_in_period()) }

    /// Chi phí TỐI ĐA có thể bị trừ khỏi ví. Ví phải kiểm tra con số này
    /// chứ không phải phí thực tế — vì phí thực tế chỉ biết sau khi khai thác.
    pub fn chi_phi_toi_da(&self) -> u128 {
        self.value + self.max_fee * self.gas_limit as u128
    }

    /// Phí thực trả theo EIP-1559: phần đốt (base fee) + tiền boa, nhưng
    /// không bao giờ vượt trần người dùng đặt.
    pub fn effective_fee(&self, phi_co_ban: u128) -> u128 {
        let boa = self.max_priority_fee.min(self.max_fee.saturating_sub(phi_co_ban));
        phi_co_ban + boa
    }
}

// ============================================================================
// 6. RÀNG BUỘC KIỂU — "macro sol!" thu nhỏ
// ============================================================================
// alloy sinh ra kiểu Rust từ ABI để bạn không tự tay ghép byte. Đây là bản
// làm tay của cùng ý tưởng: mỗi hàm hợp đồng là một phương thức có kiểu rõ ràng.

pub struct Erc20 { pub address: Address }

impl Erc20 {
    pub const CK_CHUYEN: &'static str = "transfer(address,uint256)";
    pub const CK_SO_DU: &'static str = "balanceOf(address)";
    pub const CK_CHO_PHEP: &'static str = "approve(address,uint256)";
    pub const SK_CHUYEN: &'static str = "Transfer(address,address,uint256)";

    pub fn transfer(&self, den: Address, quantity: u128) -> Vec<u8> {
        dung_calldata(Self::CK_CHUYEN, &[AbiValue::Address(den), AbiValue::Uint(quantity)])
    }
    pub fn balance_of(&self, ai: Address) -> Vec<u8> {
        dung_calldata(Self::CK_SO_DU, &[AbiValue::Address(ai)])
    }
    pub fn wait_op(&self, ai: Address, quantity: u128) -> Vec<u8> {
        dung_calldata(Self::CK_CHO_PHEP, &[AbiValue::Address(ai), AbiValue::Uint(quantity)])
    }
    /// Giải mã giá trị `uint256` trả về từ `eth_call`.
    pub fn read_balance(ket_qua: &[u8]) -> Option<u128> { doc_uint(ket_qua, 0) }
}

pub fn address_from_hex(s: &str) -> Address {
    let s = s.trim_start_matches("0x");
    let mut a = [0u8; 20];
    for i in 0..20 {
        a[i] = u8::from_str_radix(&s[i * 2..i * 2 + 2], 16).unwrap_or(0);
    }
    a
}

fn main() {
    println!("═══════════════════════════════════════════════════════════");
    println!("   EVM & ALLOY-RS: KECCAK · ABI · RLP · EIP-1559           ");
    println!("═══════════════════════════════════════════════════════════");

    println!("\n1. KECCAK-256 — đối chiếu vector chuẩn");
    println!("   keccak256(\"\")    = {}", hex(&keccak256(b"")));
    println!("   kỳ vọng           = c5d2460186f7233c927e7db2dcc703c0e500b653ca82273b7bfad8045d85a470");
    println!("   keccak256(\"abc\") = {}", hex(&keccak256(b"abc")));

    println!("\n2. CHỮ KÝ HÀM — con số bạn thấy trong mọi log Ethereum");
    for ck in [Erc20::CK_CHUYEN, Erc20::CK_SO_DU, Erc20::CK_CHO_PHEP,
               "transferFrom(address,address,uint256)", "totalSupply()"] {
        println!("   0x{} ← {}", hex(&selector(ck)), ck);
    }
    println!("   topic0 sự kiện Transfer = 0x{}", hex(&event_topic(Erc20::SK_CHUYEN)));

    println!("\n3. MÃ HOÁ ABI");
    let token = Erc20 { address: address_from_hex("0xA0b86991c6218b36c1d19D4a2e9Eb0cE3606eB48") };
    let den = address_from_hex("0x742d35Cc6634C0532925a3b844Bc454e4438f44e");
    let cd = token.transfer(den, 1_000_000);
    println!("   transfer(0x742d…f44e, 1000000) → {} byte", cd.len());
    println!("   chữ ký hàm: 0x{}", hex(&cd[..4]));
    println!("   tham số 1 : {}", hex(&cd[4..36]));
    println!("   tham số 2 : {}", hex(&cd[36..68]));

    println!("\n4. KIỂU ĐỘNG — con trỏ ở đầu, dữ liệu ở đuôi");
    let id = abi_encode(&[
        AbiValue::Uint(42),
        AbiValue::Chuoi("xin chao".into()),
        AbiValue::Bool(true),
    ]);
    println!("   (uint 42, string \"xin chao\", bool true) → {} byte", id.len());
    println!("   ô 0 (uint)     : {}", hex(&id[0..32]));
    println!("   ô 1 (con trỏ)  : {} ← trỏ tới byte {}", hex(&id[32..64]), doc_uint(&id, 1).unwrap());
    println!("   ô 2 (bool)     : {}", hex(&id[64..96]));
    println!("   ô 3 (độ dài)   : {}", hex(&id[96..128]));
    println!("   ô 4 (dữ liệu)  : {}", hex(&id[128..160]));

    println!("\n5. RLP");
    println!("   RLP(\"dog\")         = {}", hex(&Rlp::Chuoi(b"dog".to_vec()).encode()));
    println!("   RLP(0)              = {} (chuỗi RỖNG, không phải 0x00)", hex(&Rlp::numerator(0).encode()));
    println!("   RLP(15)             = {}", hex(&Rlp::numerator(15).encode()));
    println!("   RLP(1024)           = {}", hex(&Rlp::numerator(1024).encode()));
    println!("   RLP([\"cat\",\"dog\"]) = {}",
             hex(&Rlp::DanhSach(vec![Rlp::Chuoi(b"cat".to_vec()),
                                     Rlp::Chuoi(b"dog".to_vec())]).encode()));

    println!("\n6. GIAO DỊCH EIP-1559");
    let gd = Tx1559 {
        chain_id: 1, nonce: 42,
        max_priority_fee: 2_000_000_000,     // 2 gwei tiền boa
        max_fee: 100_000_000_000,           // trần 100 gwei
        gas_limit: 65_000,
        den: Some(token.address), value: 0, data: cd.clone(),
    };
    println!("   Tải trọng ký: {} byte, bắt đầu bằng 0x{:02x} (loại giao dịch)",
             gd.load_in_period().len(), gd.load_in_period()[0]);
    println!("   Băm để ký   : 0x{}", hex(&gd.id_hash_ky()));
    println!("   Chi phí tối đa bị khoá: {} wei", gd.chi_phi_toi_da());
    for phi_co_ban in [10_000_000_000u128, 50_000_000_000, 99_000_000_000] {
        println!("   base fee {:>3} gwei → thực trả {:>3} gwei/gas",
                 phi_co_ban / 1_000_000_000, gd.effective_fee(phi_co_ban) / 1_000_000_000);
    }

    println!("\n═══════════════════════════════════════════════════════════");
    println!("   MỌI GIAO DỊCH ETHEREUM CHỈ LÀ BYTE ĐƯỢC XẾP ĐÚNG CHỖ     ");
    println!("═══════════════════════════════════════════════════════════");
}

#[cfg(test)]
mod tests {
    use super::*;

    // ---------- Keccak-256 ----------
    #[test]
    fn keccak_khop_vector_chuan() {
        // Nếu bài này hỏng thì mọi thứ phía sau đều vô nghĩa.
        assert_eq!(hex(&keccak256(b"")),
            "c5d2460186f7233c927e7db2dcc703c0e500b653ca82273b7bfad8045d85a470");
        assert_eq!(hex(&keccak256(b"abc")),
            "4e03657aea45a94fc7d47ba826c8d667c0d1e6e33a64a036ec44f58fa12d6c45");
        assert_eq!(hex(&keccak256(b"testing")),
            "5f16f4c7f149ac4f9510d9cf8cf384038ad348b3bcdc01915f95de12df9d1b02");
    }

    #[test]
    fn keccak_hoat_dong_qua_bien_khoi_136_byte() {
        // RATE = 136 byte. Các mốc 135/136/137 là chỗ cài đặt hay sai nhất:
        // sai đệm ở đây thì input ngắn vẫn đúng mà input dài thì hỏng.
        let mut seen = std::collections::HashSet::new();
        for n in [0usize, 1, 135, 136, 137, 271, 272, 273, 500] {
            let b = keccak256(&vec![b'a'; n]);
            assert_eq!(b.len(), 32);
            assert!(seen.insert(b), "độ dài {} cho ra băm trùng với độ dài khác", n);
            // tất định
            assert_eq!(keccak256(&vec![b'a'; n]), b);
        }
    }

    #[test]
    fn keccak_nhay_voi_moi_byte_trong_input_nhieu_khoi() {
        // Với input 300 byte (3 khối), lật BẤT KỲ byte nào cũng phải đổi băm.
        // Nếu vòng lặp bọt biển bỏ sót một khối, bài này sẽ bắt được.
        let root = vec![7u8; 300];
        let hash_goc = keccak256(&root);
        for pos_value in [0usize, 135, 136, 200, 271, 272, 299] {
            let mut fix = root.clone();
            fix[pos_value] ^= 1;
            assert_ne!(keccak256(&fix), hash_goc, "lật byte {} mà băm không đổi", pos_value);
        }
    }

    #[test]
    fn keccak_hieu_ung_tuyet_lo() {
        let mut tong = 0u32;
        for i in 0..64u8 {
            let a = keccak256(&[i, 0]);
            let b = keccak256(&[i, 1]);
            tong += a.iter().zip(b.iter()).map(|(x, y)| (x ^ y).count_ones()).sum::<u32>();
        }
        let tb = tong as f64 / 64.0;
        assert!((tb - 128.0).abs() < 15.0, "trung bình {} bit đổi, kỳ vọng ~128", tb);
    }

    // ---------- Chữ ký hàm ----------
    #[test]
    fn chu_ky_ham_khop_gia_tri_ai_cung_biet() {
        // Đây là những chữ ký có thật, tra được trên Etherscan.
        // Chúng đồng thời là bằng chứng độc lập rằng Keccak-256 ở trên đúng.
        assert_eq!(hex(&selector("transfer(address,uint256)")), "a9059cbb");
        assert_eq!(hex(&selector("balanceOf(address)")), "70a08231");
        assert_eq!(hex(&selector("approve(address,uint256)")), "095ea7b3");
        assert_eq!(hex(&selector("transferFrom(address,address,uint256)")), "23b872dd");
        assert_eq!(hex(&selector("totalSupply()")), "18160ddd");
    }

    #[test]
    fn topic0_su_kien_transfer_dung() {
        assert_eq!(hex(&event_topic("Transfer(address,address,uint256)")),
            "ddf252ad1be2c89b69c2b068fc378daa952ba7f163c4a11628f55a4df523b3ef");
    }

    #[test]
    fn khoang_trang_trong_chu_ky_lam_doi_ket_qua() {
        // Chữ ký phải viết SÁT, không dấu cách. Sai chỗ này là gọi nhầm hàm.
        assert_ne!(selector("transfer(address,uint256)"),
                   selector("transfer(address, uint256)"));
    }

    // ---------- ABI ----------
    #[test]
    fn kieu_tinh_can_phai_trong_o_32_byte() {
        let m = abi_encode(&[AbiValue::Uint(1)]);
        assert_eq!(m.len(), 32);
        assert_eq!(m[31], 1, "giá trị nằm ở byte CUỐI, 31 byte đầu là đệm 0");
        assert!(m[..31].iter().all(|&b| b == 0));
    }

    #[test]
    fn dia_chi_duoc_dem_12_byte_o_dau() {
        let a = address_from_hex("0x742d35Cc6634C0532925a3b844Bc454e4438f44e");
        let m = abi_encode(&[AbiValue::Address(a)]);
        assert!(m[..12].iter().all(|&b| b == 0), "12 byte đầu phải là đệm");
        assert_eq!(&m[12..32], &a);
        assert_eq!(read_address(&m, 0), Some(a), "đọc ngược phải ra đúng địa chỉ");
    }

    #[test]
    fn so_am_duoc_mo_rong_dau_bang_ff() {
        let m = abi_encode(&[AbiValue::Int(-1)]);
        assert!(m.iter().all(|&b| b == 0xFF), "-1 trong bù hai là toàn bit 1");
        let m2 = abi_encode(&[AbiValue::Int(1)]);
        assert!(m2[..31].iter().all(|&b| b == 0), "số dương thì đệm 0");
    }

    #[test]
    fn bool_ma_hoa_thanh_0_hoac_1() {
        assert_eq!(abi_encode(&[AbiValue::Bool(true)])[31], 1);
        assert_eq!(abi_encode(&[AbiValue::Bool(false)])[31], 0);
    }

    #[test]
    fn kind_close_record_pointer_use_pos_value() {
        let m = abi_encode(&[
            AbiValue::Uint(42),
            AbiValue::Chuoi("xin chao".into()),
            AbiValue::Bool(true),
        ]);
        assert_eq!(doc_uint(&m, 0), Some(42));
        assert_eq!(doc_uint(&m, 1), Some(96), "con trỏ trỏ ngay sau phần đầu (3 ô × 32)");
        assert_eq!(doc_uint(&m, 2), Some(1), "bool nằm ở ô 2, không bị đẩy đi đâu");
        assert_eq!(doc_uint(&m, 3), Some(8), "ô đầu phần đuôi là độ dài chuỗi");
        assert_eq!(&m[128..136], b"xin chao");
    }

    #[test]
    fn kieu_dong_duoc_dem_cho_tron_32_byte() {
        let m = abi_encode(&[AbiValue::Chuoi("a".into())]);
        assert_eq!(m.len() % 32, 0, "toàn bộ mã hoá ABI luôn là bội của 32");
        assert_eq!(m.len(), 32 + 32 + 32, "con trỏ + độ dài + 1 ô dữ liệu đã đệm");
    }

    #[test]
    fn nhieu_kieu_dong_khong_de_len_nhau() {
        let m = abi_encode(&[
            AbiValue::Chuoi("mot".into()),
            AbiValue::Chuoi("hai ba bon nam sau bay".into()),
        ]);
        let p1 = doc_uint(&m, 0).unwrap() as usize;
        let p2 = doc_uint(&m, 1).unwrap() as usize;
        assert!(p2 > p1, "con trỏ thứ hai phải nằm SAU dữ liệu thứ nhất");
        assert_eq!(&m[p1 + 32..p1 + 35], b"mot");
        assert_eq!(&m[p2 + 32..p2 + 54], b"hai ba bon nam sau bay");
    }

    #[test]
    fn mang_uint_ma_hoa_do_dai_roi_toi_phan_tu() {
        let m = abi_encode(&[AbiValue::MangUint(vec![10, 20, 30])]);
        assert_eq!(doc_uint(&m, 0), Some(32), "con trỏ");
        assert_eq!(doc_uint(&m, 1), Some(3), "độ dài mảng");
        assert_eq!(doc_uint(&m, 2), Some(10));
        assert_eq!(doc_uint(&m, 3), Some(20));
        assert_eq!(doc_uint(&m, 4), Some(30));
    }

    #[test]
    fn calldata_transfer_khop_dinh_dang_that() {
        let t = Erc20 { address: [0u8; 20] };
        let den = address_from_hex("0x742d35Cc6634C0532925a3b844Bc454e4438f44e");
        let cd = t.transfer(den, 1_000_000);
        assert_eq!(cd.len(), 4 + 32 + 32, "4 byte chữ ký hàm + 2 ô tham số");
        assert_eq!(hex(&cd[..4]), "a9059cbb");
        assert_eq!(read_address(&cd[4..], 0), Some(den));
        assert_eq!(doc_uint(&cd[4..], 1), Some(1_000_000));
    }

    #[test]
    fn doc_uint_tu_choi_gia_tri_vuot_u128() {
        let mut d = [0u8; 32];
        d[0] = 1; // bit cao của uint256, vượt xa u128
        assert_eq!(doc_uint(&d, 0), None, "phải báo lỗi chứ không cắt cụt âm thầm");
    }

    #[test]
    fn doc_dia_chi_tu_choi_o_co_rac_o_phan_dem() {
        let mut d = [0u8; 32];
        d[0] = 0xAA; // rác trong 12 byte đệm — dấu hiệu dữ liệu hỏng
        assert_eq!(read_address(&d, 0), None);
    }

    // ---------- RLP ----------
    #[test]
    fn rlp_khop_vi_du_trong_sach_vang() {
        // Các ví dụ này lấy thẳng từ Ethereum Yellow Paper.
        assert_eq!(hex(&Rlp::Chuoi(b"dog".to_vec()).encode()), "83646f67");
        assert_eq!(hex(&Rlp::Chuoi(vec![]).encode()), "80");
        assert_eq!(hex(&Rlp::DanhSach(vec![]).encode()), "c0");
        assert_eq!(hex(&Rlp::Chuoi(vec![0x0f]).encode()), "0f", "byte nhỏ tự mã hoá");
        assert_eq!(hex(&Rlp::Chuoi(vec![0x04, 0x00]).encode()), "820400");
        assert_eq!(hex(&Rlp::DanhSach(vec![
            Rlp::Chuoi(b"cat".to_vec()), Rlp::Chuoi(b"dog".to_vec())]).encode()),
            "c88363617483646f67");
    }

    #[test]
    fn rlp_so_khong_la_chuoi_rong() {
        // Bẫy kinh điển: RLP(0) KHÔNG phải 0x00 mà là 0x80 (chuỗi rỗng).
        assert_eq!(hex(&Rlp::numerator(0).encode()), "80");
        assert_ne!(Rlp::numerator(0), Rlp::Chuoi(vec![0]));
    }

    #[test]
    fn rlp_so_khong_co_so_khong_thua_o_dau() {
        assert_eq!(Rlp::numerator(1024), Rlp::Chuoi(vec![0x04, 0x00]));
        assert_eq!(Rlp::numerator(255), Rlp::Chuoi(vec![0xff]));
        assert_eq!(Rlp::numerator(256), Rlp::Chuoi(vec![0x01, 0x00]));
    }

    #[test]
    fn rlp_chuoi_dai_dung_dinh_dang_do_dai_dai() {
        let long = vec![b'a'; 100];
        let m = Rlp::Chuoi(long).encode();
        assert_eq!(m[0], 0xB7 + 1, "0xB7 + số byte cần để ghi độ dài");
        assert_eq!(m[1], 100);
        assert_eq!(m.len(), 2 + 100);
    }

    #[test]
    fn rlp_bien_55_va_56_byte() {
        // 55 byte dùng định dạng ngắn, 56 byte chuyển sang định dạng dài
        assert_eq!(Rlp::Chuoi(vec![b'a'; 55]).encode()[0], 0x80 + 55);
        assert_eq!(Rlp::Chuoi(vec![b'a'; 56]).encode()[0], 0xB7 + 1);
    }

    // ---------- Giao dịch ----------
    fn trade_mau() -> Tx1559 {
        Tx1559 {
            chain_id: 1, nonce: 42,
            max_priority_fee: 2_000_000_000,
            max_fee: 100_000_000_000,
            gas_limit: 21_000,
            den: Some(address_from_hex("0x742d35Cc6634C0532925a3b844Bc454e4438f44e")),
            value: 1_000_000_000_000_000_000, // 1 ETH
            data: vec![],
        }
    }

    #[test]
    fn tai_in_ky_start_table_kind_trade() {
        assert_eq!(trade_mau().load_in_period()[0], 0x02, "EIP-1559 là loại 0x02");
    }

    #[test]
    fn swap_enable_ky_truong_which_same_swap_id_hash() {
        // Bất biến sống còn: chữ ký phải phủ TOÀN BỘ nội dung giao dịch.
        // Nếu một trường lọt ra ngoài, kẻ tấn công sửa được nó mà chữ ký vẫn hợp lệ.
        let root = trade_mau();
        let b0 = root.id_hash_ky();
        let bien_the: Vec<Tx1559> = vec![
            Tx1559 { chain_id: 5, ..root.clone() },
            Tx1559 { nonce: 43, ..root.clone() },
            Tx1559 { max_priority_fee: 3_000_000_000, ..root.clone() },
            Tx1559 { max_fee: 90_000_000_000, ..root.clone() },
            Tx1559 { gas_limit: 30_000, ..root.clone() },
            Tx1559 { den: None, ..root.clone() },
            Tx1559 { value: 2, ..root.clone() },
            Tx1559 { data: vec![1], ..root.clone() },
        ];
        for (i, v) in bien_the.iter().enumerate() {
            assert_ne!(v.id_hash_ky(), b0, "biến thể {} phải cho mã băm khác", i);
        }
    }

    #[test]
    fn make_contract_encode_dich_into_series_empty() {
        let tao = Tx1559 { den: None, ..trade_mau() };
        let send = trade_mau();
        assert_ne!(tao.load_in_period(), send.load_in_period());
        // `den: None` phải thành 0x80 (chuỗi rỗng), không phải 20 byte 0
        assert!(tao.load_in_period().len() < send.load_in_period().len());
    }

    #[test]
    fn only_max_fee_use_cong_thuc_pos_must_key() {
        let gd = trade_mau();
        assert_eq!(gd.chi_phi_toi_da(),
                   1_000_000_000_000_000_000 + 100_000_000_000 * 21_000);
    }

    #[test]
    fn phi_thuc_te_khong_bao_gio_vuot_tran_nguoi_dung_dat() {
        let gd = trade_mau();
        for base in [1u128, 50_000_000_000, 99_000_000_000, 100_000_000_000] {
            assert!(gd.effective_fee(base) <= gd.max_fee,
                    "base {} → thực trả {} vượt trần {}",
                    base, gd.effective_fee(base), gd.max_fee);
        }
    }

    #[test]
    fn base_fee_thap_thi_tra_tron_ven_tien_boa() {
        let gd = trade_mau();
        let base = 10_000_000_000u128;
        assert_eq!(gd.effective_fee(base), base + gd.max_priority_fee);
    }

    #[test]
    fn base_fee_gan_tran_thi_tien_boa_bi_bop_lai() {
        let gd = trade_mau();
        let base = 99_000_000_000u128; // trần 100 gwei, chỉ còn 1 gwei cho boa
        assert_eq!(gd.effective_fee(base), 100_000_000_000,
                   "tiền boa bị cắt xuống 1 gwei chứ không phải 2");
    }

    #[test]
    fn base_fee_vuot_tran_thi_phep_tinh_khong_tran_so() {
        let gd = trade_mau();
        assert_eq!(gd.effective_fee(200_000_000_000), 200_000_000_000,
                   "giao dịch này sẽ không được chọn vào khối, nhưng không được panic");
    }
}
