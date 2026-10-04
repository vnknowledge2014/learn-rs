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
    0x0000000000000001,
    0x0000000000008082,
    0x800000000000808a,
    0x8000000080008000,
    0x000000000000808b,
    0x0000000080000001,
    0x8000000080008081,
    0x8000000000008009,
    0x000000000000008a,
    0x0000000000000088,
    0x0000000080008009,
    0x000000008000000a,
    0x000000008000808b,
    0x800000000000008b,
    0x8000000000008089,
    0x8000000000008003,
    0x8000000000008002,
    0x8000000000000080,
    0x000000000000800a,
    0x800000008000000a,
    0x8000000080008081,
    0x8000000000008080,
    0x0000000080000001,
    0x8000000080008008,
];
const ROTC: [u32; 24] = [
    1, 3, 6, 10, 15, 21, 28, 36, 45, 55, 2, 14, 27, 41, 56, 8, 25, 43, 62, 18, 39, 61, 20, 44,
];
const PIL: [usize; 24] = [
    10, 7, 11, 17, 18, 3, 5, 16, 8, 21, 24, 4, 15, 23, 19, 13, 12, 2, 20, 14, 22, 9, 6, 1,
];

/// Hoán vị Keccak-f[1600] — 24 vòng trên trạng thái 5×5 lane 64-bit.
fn keccak_f(a: &mut [u64; 25]) {
    let mut bc = [0u64; 5];
    for &rc in RC.iter() {
        // θ (theta): trộn mỗi cột với hai cột lân cận
        for x in 0..5 {
            bc[x] = a[x] ^ a[x + 5] ^ a[x + 10] ^ a[x + 15] ^ a[x + 20];
        }
        for x in 0..5 {
            let t = bc[(x + 4) % 5] ^ bc[(x + 1) % 5].rotate_left(1);
            for y in (0..25).step_by(5) {
                a[y + x] ^= t;
            }
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
            bc.copy_from_slice(&a[y..y + 5]);
            for x in 0..5 {
                a[y + x] ^= (!bc[(x + 1) % 5]) & bc[(x + 2) % 5];
            }
        }
        // ι (iota): phá đối xứng bằng hằng số vòng
        a[0] ^= rc;
    }
}

/// Cấu trúc "bọt biển": hút dữ liệu vào theo từng khối `RATE` byte, rồi vắt ra.
pub fn keccak256(data: &[u8]) -> [u8; 32] {
    const RATE: usize = 136; // 1600 bit − 2×256 bit dung lượng = 1088 bit
    let mut a = [0u64; 25];
    let mut padded = Vec::with_capacity(data.len() + RATE);
    padded.extend_from_slice(data);
    // Đệm pad10*1 với byte miền 0x01 — CHỖ NÀY khác SHA3-256 (dùng 0x06)
    padded.push(0x01);
    while !padded.len().is_multiple_of(RATE) {
        padded.push(0x00);
    }
    let n = padded.len();
    padded[n - 1] |= 0x80;

    // Trạng thái là MẢNG PHẲNG [u64; 25]: lane (x, y) nằm ở chỉ số x + 5·y.
    for block in padded.chunks(RATE) {
        for (i, word) in block.chunks(8).enumerate() {
            a[i] ^= u64::from_le_bytes(word.try_into().unwrap());
        }
        keccak_f(&mut a);
    }
    let mut out = [0u8; 32];
    for (chunk, lane) in out.chunks_mut(8).zip(&a) {
        chunk.copy_from_slice(&lane.to_le_bytes());
    }
    out
}

pub fn hex(b: &[u8]) -> String {
    b.iter().map(|x| format!("{:02x}", x)).collect()
}

// ============================================================================
// 2. CHỮ KÝ HÀM — 4 byte quyết định EVM gọi hàm nào
// ============================================================================

/// `selector("transfer(address,uint256)")` = 0xa9059cbb — con số mà bất kỳ ai
/// từng đọc log Ethereum đều đã thấy hàng nghìn lần.
///
/// Chỉ 4 byte nghĩa là VA CHẠM CÓ THẬT: xác suất hai hàm khác nhau trùng
/// chữ ký chỉ khoảng 1/2³². Đã có người cố tình tìm hàm trùng để đánh lừa
/// giao diện ví — đó là lý do ví hiện đại hiển thị cả chữ ký đầy đủ.
pub fn selector(signature: &str) -> [u8; 4] {
    let b = keccak256(signature.as_bytes());
    [b[0], b[1], b[2], b[3]]
}

/// Chủ đề sự kiện (topic0) dùng cả 32 byte, nên an toàn hơn hẳn.
pub fn event_topic(signature: &str) -> [u8; 32] {
    keccak256(signature.as_bytes())
}

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
    Text(String),
    UintArray(Vec<u128>),
}

impl AbiValue {
    pub fn is_dynamic(&self) -> bool {
        matches!(
            self,
            AbiValue::Bytes(_) | AbiValue::Text(_) | AbiValue::UintArray(_)
        )
    }

    fn word(v: u128) -> [u8; 32] {
        let mut w = [0u8; 32];
        w[16..].copy_from_slice(&v.to_be_bytes()); // căn PHẢI
        w
    }

    /// Phần cố định: kiểu tĩnh ghi thẳng giá trị, kiểu động ghi con trỏ (điền sau).
    fn head_word(&self) -> [u8; 32] {
        match self {
            AbiValue::Uint(v) => Self::word(*v),
            AbiValue::Bool(b) => Self::word(*b as u128),
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
    fn tail(&self) -> Vec<u8> {
        match self {
            AbiValue::Bytes(b) => {
                let mut v = Self::word(b.len() as u128).to_vec();
                v.extend_from_slice(b);
                while !v.len().is_multiple_of(32) {
                    v.push(0);
                }
                v
            }
            AbiValue::Text(s) => AbiValue::Bytes(s.as_bytes().to_vec()).tail(),
            AbiValue::UintArray(m) => {
                let mut v = Self::word(m.len() as u128).to_vec();
                for x in m {
                    v.extend_from_slice(&Self::word(*x));
                }
                v
            }
            _ => Vec::new(),
        }
    }
}

/// Mã hoá danh sách tham số theo đúng đặc tả ABI của Solidity.
pub fn abi_encode(values: &[AbiValue]) -> Vec<u8> {
    let head_size = values.len() * 32;
    let mut head: Vec<u8> = Vec::with_capacity(head_size);
    let mut tail: Vec<u8> = Vec::new();

    for value in values {
        if value.is_dynamic() {
            // Con trỏ tính từ ĐẦU vùng tham số, không phải từ đầu calldata.
            // Nhầm gốc toạ độ ở đây là lỗi ABI phổ biến nhất.
            let offset = head_size + tail.len();
            head.extend_from_slice(&AbiValue::word(offset as u128));
            tail.extend_from_slice(&value.tail());
        } else {
            head.extend_from_slice(&value.head_word());
        }
    }
    head.extend_from_slice(&tail);
    head
}

/// Dựng calldata hoàn chỉnh: 4 byte chữ ký hàm + tham số đã mã hoá.
pub fn build_calldata(signature: &str, values: &[AbiValue]) -> Vec<u8> {
    let mut v = selector(signature).to_vec();
    v.extend_from_slice(&abi_encode(values));
    v
}

/// Giải mã ngược một tham số `uint256` ở ô thứ `index` (dùng để đọc kết quả).
pub fn read_uint(data: &[u8], index: usize) -> Option<u128> {
    let d = data.get(index * 32..index * 32 + 32)?;
    // 16 byte cao phải bằng 0, nếu không thì giá trị vượt u128
    if d[..16].iter().any(|&b| b != 0) {
        return None;
    }
    Some(u128::from_be_bytes(d[16..].try_into().ok()?))
}

pub fn read_address(data: &[u8], index: usize) -> Option<Address> {
    let d = data.get(index * 32..index * 32 + 32)?;
    if d[..12].iter().any(|&b| b != 0) {
        return None;
    } // 12 byte đệm phải là 0
    d[12..].try_into().ok()
}

// ============================================================================
// 4. MÃ HOÁ RLP — định dạng tuần tự hoá của Ethereum
// ============================================================================

#[derive(Debug, Clone, PartialEq)]
pub enum Rlp {
    Bytes(Vec<u8>),
    List(Vec<Rlp>),
}

impl Rlp {
    pub fn encode(&self) -> Vec<u8> {
        match self {
            Rlp::Bytes(b) => {
                if b.len() == 1 && b[0] < 0x80 {
                    b.clone() // byte đơn nhỏ tự mã hoá chính nó
                } else {
                    let mut v = Self::prefix(b.len(), 0x80);
                    v.extend_from_slice(b);
                    v
                }
            }
            Rlp::List(list) => {
                let mut payload = Vec::new();
                for x in list {
                    payload.extend_from_slice(&x.encode());
                }
                let mut v = Self::prefix(payload.len(), 0xC0);
                v.extend_from_slice(&payload);
                v
            }
        }
    }

    fn prefix(length: usize, base: u8) -> Vec<u8> {
        if length < 56 {
            vec![base + length as u8]
        } else {
            // Độ dài dài: ghi độ-dài-của-độ-dài rồi tới độ dài
            let b = length.to_be_bytes();
            let skip = b.iter().position(|&x| x != 0).unwrap();
            let mut v = vec![base + 55 + (b.len() - skip) as u8];
            v.extend_from_slice(&b[skip..]);
            v
        }
    }

    /// Số nguyên trong RLP dùng big-endian KHÔNG có số 0 thừa ở đầu.
    /// Số 0 mã hoá thành chuỗi RỖNG, không phải byte 0x00 — điểm hay bị sai.
    pub fn uint(v: u128) -> Rlp {
        if v == 0 {
            return Rlp::Bytes(vec![]);
        }
        let b = v.to_be_bytes();
        let skip = b.iter().position(|&x| x != 0).unwrap();
        Rlp::Bytes(b[skip..].to_vec())
    }
}

// ============================================================================
// 5. GIAO DỊCH EIP-1559
// ============================================================================

#[derive(Debug, Clone, PartialEq)]
pub struct Tx1559 {
    pub chain_id: u64,
    pub nonce: u64,             // nonce
    pub max_priority_fee: u128, // tiền "boa" cho người xây khối
    pub max_fee: u128,          // trần tổng phí mỗi đơn vị gas
    pub gas_limit: u64,
    pub to: Option<Address>, // None = tạo hợp đồng mới
    pub value: u128,
    pub data: Vec<u8>,
}

impl Tx1559 {
    /// Tải trọng để ký: 0x02 || rlp([...]). Byte 0x02 là "loại giao dịch",
    /// thêm vào từ EIP-2718 để chuỗi phân biệt được các định dạng khác nhau.
    pub fn signing_payload(&self) -> Vec<u8> {
        let list = Rlp::List(vec![
            Rlp::uint(self.chain_id as u128),
            Rlp::uint(self.nonce as u128),
            Rlp::uint(self.max_priority_fee),
            Rlp::uint(self.max_fee),
            Rlp::uint(self.gas_limit as u128),
            match self.to {
                Some(a) => Rlp::Bytes(a.to_vec()),
                None => Rlp::Bytes(vec![]),
            },
            Rlp::uint(self.value),
            Rlp::Bytes(self.data.clone()),
            Rlp::List(vec![]), // danh sách truy cập (EIP-2930), để trống
        ]);
        let mut v = vec![0x02];
        v.extend_from_slice(&list.encode());
        v
    }

    pub fn signing_hash(&self) -> [u8; 32] {
        keccak256(&self.signing_payload())
    }

    /// Chi phí TỐI ĐA có thể bị trừ khỏi ví. Ví phải kiểm tra con số này
    /// chứ không phải phí thực tế — vì phí thực tế chỉ biết sau khi khai thác.
    pub fn max_cost(&self) -> u128 {
        self.value + self.max_fee * self.gas_limit as u128
    }

    /// Giá gas thực trả theo EIP-1559: phần đốt (base fee) + tiền boa, nhưng
    /// không bao giờ vượt trần `max_fee` người dùng đặt.
    ///
    /// Trả `None` khi `base_fee > max_fee`: giao dịch như vậy KHÔNG hợp lệ
    /// trong khối này (không được đưa vào khối) — chứ không phải "trả base fee".
    pub fn effective_gas_price(&self, base_fee: u128) -> Option<u128> {
        let headroom = self.max_fee.checked_sub(base_fee)?;
        let tip = self.max_priority_fee.min(headroom);
        Some(base_fee + tip)
    }
}

// ============================================================================
// 6. RÀNG BUỘC KIỂU — "macro sol!" thu nhỏ
// ============================================================================
// alloy sinh ra kiểu Rust từ ABI để bạn không tự tay ghép byte. Đây là bản
// làm tay của cùng ý tưởng: mỗi hàm hợp đồng là một phương thức có kiểu rõ ràng.

pub struct Erc20 {
    pub address: Address,
}

impl Erc20 {
    pub const TRANSFER_SIG: &'static str = "transfer(address,uint256)";
    pub const BALANCE_OF_SIG: &'static str = "balanceOf(address)";
    pub const APPROVE_SIG: &'static str = "approve(address,uint256)";
    pub const TRANSFER_EVENT: &'static str = "Transfer(address,address,uint256)";

    pub fn transfer(&self, to: Address, quantity: u128) -> Vec<u8> {
        build_calldata(
            Self::TRANSFER_SIG,
            &[AbiValue::Address(to), AbiValue::Uint(quantity)],
        )
    }
    pub fn balance_of(&self, owner: Address) -> Vec<u8> {
        build_calldata(Self::BALANCE_OF_SIG, &[AbiValue::Address(owner)])
    }
    pub fn approve(&self, spender: Address, quantity: u128) -> Vec<u8> {
        build_calldata(
            Self::APPROVE_SIG,
            &[AbiValue::Address(spender), AbiValue::Uint(quantity)],
        )
    }
    /// Giải mã giá trị `uint256` trả về từ `eth_call`.
    pub fn read_balance(result: &[u8]) -> Option<u128> {
        read_uint(result, 0)
    }
}

pub fn address_from_hex(s: &str) -> Address {
    let s = s.trim_start_matches("0x");
    let mut a = [0u8; 20];
    for (i, byte) in a.iter_mut().enumerate() {
        *byte = u8::from_str_radix(&s[i * 2..i * 2 + 2], 16).unwrap_or(0);
    }
    a
}

fn main() {
    println!("═══════════════════════════════════════════════════════════");
    println!("   EVM & ALLOY-RS: KECCAK · ABI · RLP · EIP-1559           ");
    println!("═══════════════════════════════════════════════════════════");

    println!("\n1. KECCAK-256 — đối chiếu vector chuẩn");
    println!("   keccak256(\"\")    = {}", hex(&keccak256(b"")));
    println!(
        "   kỳ vọng           = c5d2460186f7233c927e7db2dcc703c0e500b653ca82273b7bfad8045d85a470"
    );
    println!("   keccak256(\"abc\") = {}", hex(&keccak256(b"abc")));

    println!("\n2. CHỮ KÝ HÀM — con số bạn thấy trong mọi log Ethereum");
    for sig in [
        Erc20::TRANSFER_SIG,
        Erc20::BALANCE_OF_SIG,
        Erc20::APPROVE_SIG,
        "transferFrom(address,address,uint256)",
        "totalSupply()",
    ] {
        println!("   0x{} ← {}", hex(&selector(sig)), sig);
    }
    println!(
        "   topic0 sự kiện Transfer = 0x{}",
        hex(&event_topic(Erc20::TRANSFER_EVENT))
    );

    println!("\n3. MÃ HOÁ ABI");
    let token = Erc20 {
        address: address_from_hex("0xA0b86991c6218b36c1d19D4a2e9Eb0cE3606eB48"),
    };
    let to = address_from_hex("0x742d35Cc6634C0532925a3b844Bc454e4438f44e");
    let calldata = token.transfer(to, 1_000_000);
    println!(
        "   transfer(0x742d…f44e, 1000000) → {} byte",
        calldata.len()
    );
    println!("   chữ ký hàm: 0x{}", hex(&calldata[..4]));
    println!("   tham số 1 : {}", hex(&calldata[4..36]));
    println!("   tham số 2 : {}", hex(&calldata[36..68]));

    println!("\n4. KIỂU ĐỘNG — con trỏ ở đầu, dữ liệu ở đuôi");
    let enc = abi_encode(&[
        AbiValue::Uint(42),
        AbiValue::Text("xin chào".into()),
        AbiValue::Bool(true),
    ]);
    println!(
        "   (uint 42, string \"xin chào\", bool true) → {} byte",
        enc.len()
    );
    println!("   ô 0 (uint)     : {}", hex(&enc[0..32]));
    println!(
        "   ô 1 (con trỏ)  : {} ← trỏ tới byte {}",
        hex(&enc[32..64]),
        read_uint(&enc, 1).unwrap()
    );
    println!("   ô 2 (bool)     : {}", hex(&enc[64..96]));
    println!("   ô 3 (độ dài)   : {}", hex(&enc[96..128]));
    println!("   ô 4 (dữ liệu)  : {}", hex(&enc[128..160]));

    println!("\n5. RLP");
    println!(
        "   RLP(\"dog\")          = {}",
        hex(&Rlp::Bytes(b"dog".to_vec()).encode())
    );
    println!(
        "   RLP(0)              = {} (chuỗi RỖNG, không phải 0x00)",
        hex(&Rlp::uint(0).encode())
    );
    println!("   RLP(15)             = {}", hex(&Rlp::uint(15).encode()));
    println!(
        "   RLP(1024)           = {}",
        hex(&Rlp::uint(1024).encode())
    );
    println!(
        "   RLP([\"cat\",\"dog\"])  = {}",
        hex(&Rlp::List(vec![
            Rlp::Bytes(b"cat".to_vec()),
            Rlp::Bytes(b"dog".to_vec())
        ])
        .encode())
    );

    println!("\n6. GIAO DỊCH EIP-1559");
    let tx = Tx1559 {
        chain_id: 1,
        nonce: 42,
        max_priority_fee: 2_000_000_000, // 2 gwei tiền boa
        max_fee: 100_000_000_000,        // trần 100 gwei
        gas_limit: 65_000,
        to: Some(token.address),
        value: 0,
        data: calldata.clone(),
    };
    println!(
        "   Tải trọng ký: {} byte, bắt đầu bằng 0x{:02x} (loại giao dịch)",
        tx.signing_payload().len(),
        tx.signing_payload()[0]
    );
    println!("   Băm để ký   : 0x{}", hex(&tx.signing_hash()));
    println!("   Chi phí tối đa bị khoá: {} wei", tx.max_cost());
    for base_fee in [
        10_000_000_000u128,
        50_000_000_000,
        99_000_000_000,
        120_000_000_000,
    ] {
        match tx.effective_gas_price(base_fee) {
            Some(p) => println!(
                "   base fee {:>3} gwei → thực trả {:>3} gwei/gas",
                base_fee / 1_000_000_000,
                p / 1_000_000_000
            ),
            None => println!(
                "   base fee {:>3} gwei → vượt trần max_fee: KHÔNG được đưa vào khối",
                base_fee / 1_000_000_000
            ),
        }
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
    fn keccak_matches_reference_vectors() {
        // Nếu bài này hỏng thì mọi thứ phía sau đều vô nghĩa.
        assert_eq!(
            hex(&keccak256(b"")),
            "c5d2460186f7233c927e7db2dcc703c0e500b653ca82273b7bfad8045d85a470"
        );
        assert_eq!(
            hex(&keccak256(b"abc")),
            "4e03657aea45a94fc7d47ba826c8d667c0d1e6e33a64a036ec44f58fa12d6c45"
        );
        assert_eq!(
            hex(&keccak256(b"testing")),
            "5f16f4c7f149ac4f9510d9cf8cf384038ad348b3bcdc01915f95de12df9d1b02"
        );
    }

    #[test]
    fn keccak_spans_the_136_byte_block_boundary() {
        // RATE = 136 byte. Các mốc 135/136/137 là chỗ cài đặt hay sai nhất:
        // sai đệm ở đây thì input ngắn vẫn đúng mà input dài thì hỏng.
        let mut seen = std::collections::HashSet::new();
        for n in [0usize, 1, 135, 136, 137, 271, 272, 273, 500] {
            let b = keccak256(&vec![b'a'; n]);
            assert_eq!(b.len(), 32);
            assert!(
                seen.insert(b),
                "độ dài {} cho ra băm trùng với độ dài khác",
                n
            );
            // tất định
            assert_eq!(keccak256(&vec![b'a'; n]), b);
        }
    }

    #[test]
    fn keccak_is_sensitive_to_every_byte() {
        // Với input 300 byte (3 khối), lật BẤT KỲ byte nào cũng phải đổi băm.
        // Nếu vòng lặp bọt biển bỏ sót một khối, bài này sẽ bắt được.
        let base = vec![7u8; 300];
        let base_hash = keccak256(&base);
        for index in [0usize, 135, 136, 200, 271, 272, 299] {
            let mut flipped = base.clone();
            flipped[index] ^= 1;
            assert_ne!(
                keccak256(&flipped),
                base_hash,
                "lật byte {} mà băm không đổi",
                index
            );
        }
    }

    #[test]
    fn keccak_avalanche() {
        let mut total = 0u32;
        for i in 0..64u8 {
            let a = keccak256(&[i, 0]);
            let b = keccak256(&[i, 1]);
            total += a
                .iter()
                .zip(b.iter())
                .map(|(x, y)| (x ^ y).count_ones())
                .sum::<u32>();
        }
        let avg = total as f64 / 64.0;
        assert!(
            (avg - 128.0).abs() < 15.0,
            "trung bình {} bit đổi, kỳ vọng ~128",
            avg
        );
    }

    // ---------- Chữ ký hàm ----------
    #[test]
    fn selectors_match_well_known_values() {
        // Đây là những chữ ký có thật, tra được trên Etherscan.
        // Chúng đồng thời là bằng chứng độc lập rằng Keccak-256 ở trên đúng.
        assert_eq!(hex(&selector("transfer(address,uint256)")), "a9059cbb");
        assert_eq!(hex(&selector("balanceOf(address)")), "70a08231");
        assert_eq!(hex(&selector("approve(address,uint256)")), "095ea7b3");
        assert_eq!(
            hex(&selector("transferFrom(address,address,uint256)")),
            "23b872dd"
        );
        assert_eq!(hex(&selector("totalSupply()")), "18160ddd");
    }

    #[test]
    fn transfer_event_topic0_is_correct() {
        assert_eq!(
            hex(&event_topic("Transfer(address,address,uint256)")),
            "ddf252ad1be2c89b69c2b068fc378daa952ba7f163c4a11628f55a4df523b3ef"
        );
    }

    #[test]
    fn whitespace_in_signature_changes_the_selector() {
        // Chữ ký phải viết SÁT, không dấu cách. Sai chỗ này là gọi nhầm hàm.
        assert_ne!(
            selector("transfer(address,uint256)"),
            selector("transfer(address, uint256)")
        );
    }

    // ---------- ABI ----------
    #[test]
    fn static_types_right_align_in_a_32_byte_word() {
        let enc = abi_encode(&[AbiValue::Uint(1)]);
        assert_eq!(enc.len(), 32);
        assert_eq!(enc[31], 1, "giá trị nằm ở byte CUỐI, 31 byte đầu là đệm 0");
        assert!(enc[..31].iter().all(|&b| b == 0));
    }

    #[test]
    fn address_is_left_padded_with_12_bytes() {
        let a = address_from_hex("0x742d35Cc6634C0532925a3b844Bc454e4438f44e");
        let enc = abi_encode(&[AbiValue::Address(a)]);
        assert!(enc[..12].iter().all(|&b| b == 0), "12 byte đầu phải là đệm");
        assert_eq!(&enc[12..32], &a);
        assert_eq!(
            read_address(&enc, 0),
            Some(a),
            "đọc ngược phải ra đúng địa chỉ"
        );
    }

    #[test]
    fn negative_ints_are_sign_extended_with_ff() {
        let enc = abi_encode(&[AbiValue::Int(-1)]);
        assert!(
            enc.iter().all(|&b| b == 0xFF),
            "-1 trong bù hai là toàn bit 1"
        );
        let enc2 = abi_encode(&[AbiValue::Int(1)]);
        assert!(enc2[..31].iter().all(|&b| b == 0), "số dương thì đệm 0");
    }

    #[test]
    fn bool_encodes_to_zero_or_one() {
        assert_eq!(abi_encode(&[AbiValue::Bool(true)])[31], 1);
        assert_eq!(abi_encode(&[AbiValue::Bool(false)])[31], 0);
    }

    #[test]
    fn dynamic_types_write_correct_offsets() {
        let enc = abi_encode(&[
            AbiValue::Uint(42),
            AbiValue::Text("xin chào".into()),
            AbiValue::Bool(true),
        ]);
        assert_eq!(read_uint(&enc, 0), Some(42));
        assert_eq!(
            read_uint(&enc, 1),
            Some(96),
            "con trỏ trỏ ngay sau phần đầu (3 ô × 32)"
        );
        assert_eq!(
            read_uint(&enc, 2),
            Some(1),
            "bool nằm ở ô 2, không bị đẩy đi đâu"
        );
        assert_eq!(
            read_uint(&enc, 3),
            Some(9),
            "ô đầu phần đuôi là độ dài chuỗi TÍNH BẰNG BYTE (UTF-8), không phải ký tự"
        );
        assert_eq!(&enc[128..137], "xin chào".as_bytes()); // "à" chiếm 2 byte UTF-8
    }

    #[test]
    fn dynamic_data_is_padded_to_32_bytes() {
        let enc = abi_encode(&[AbiValue::Text("a".into())]);
        assert_eq!(enc.len() % 32, 0, "toàn bộ mã hoá ABI luôn là bội của 32");
        assert_eq!(
            enc.len(),
            32 + 32 + 32,
            "con trỏ + độ dài + 1 ô dữ liệu đã đệm"
        );
    }

    #[test]
    fn multiple_dynamic_types_do_not_overlap() {
        let enc = abi_encode(&[
            AbiValue::Text("one".into()),
            AbiValue::Text("the second one, longer".into()),
        ]);
        let p1 = read_uint(&enc, 0).unwrap() as usize;
        let p2 = read_uint(&enc, 1).unwrap() as usize;
        assert!(p2 > p1, "con trỏ thứ hai phải nằm SAU dữ liệu thứ nhất");
        assert_eq!(&enc[p1 + 32..p1 + 35], b"one");
        assert_eq!(&enc[p2 + 32..p2 + 54], b"the second one, longer");
    }

    #[test]
    fn uint_array_encodes_length_then_elements() {
        let enc = abi_encode(&[AbiValue::UintArray(vec![10, 20, 30])]);
        assert_eq!(read_uint(&enc, 0), Some(32), "con trỏ");
        assert_eq!(read_uint(&enc, 1), Some(3), "độ dài mảng");
        assert_eq!(read_uint(&enc, 2), Some(10));
        assert_eq!(read_uint(&enc, 3), Some(20));
        assert_eq!(read_uint(&enc, 4), Some(30));
    }

    #[test]
    fn transfer_calldata_matches_the_real_format() {
        let t = Erc20 { address: [0u8; 20] };
        let to = address_from_hex("0x742d35Cc6634C0532925a3b844Bc454e4438f44e");
        let calldata = t.transfer(to, 1_000_000);
        assert_eq!(
            calldata.len(),
            4 + 32 + 32,
            "4 byte chữ ký hàm + 2 ô tham số"
        );
        assert_eq!(hex(&calldata[..4]), "a9059cbb");
        assert_eq!(read_address(&calldata[4..], 0), Some(to));
        assert_eq!(read_uint(&calldata[4..], 1), Some(1_000_000));
    }

    #[test]
    fn decoding_rejects_uint_beyond_u128() {
        let mut d = [0u8; 32];
        d[0] = 1; // bit cao của uint256, vượt xa u128
        assert_eq!(
            read_uint(&d, 0),
            None,
            "phải báo lỗi chứ không cắt cụt âm thầm"
        );
    }

    #[test]
    fn address_decode_rejects_dirty_padding() {
        let mut d = [0u8; 32];
        d[0] = 0xAA; // rác trong 12 byte đệm — dấu hiệu dữ liệu hỏng
        assert_eq!(read_address(&d, 0), None);
    }

    // ---------- RLP ----------
    #[test]
    fn rlp_matches_yellow_paper_examples() {
        // Các ví dụ này lấy thẳng từ Ethereum Yellow Paper.
        assert_eq!(hex(&Rlp::Bytes(b"dog".to_vec()).encode()), "83646f67");
        assert_eq!(hex(&Rlp::Bytes(vec![]).encode()), "80");
        assert_eq!(hex(&Rlp::List(vec![]).encode()), "c0");
        assert_eq!(
            hex(&Rlp::Bytes(vec![0x0f]).encode()),
            "0f",
            "byte nhỏ tự mã hoá"
        );
        assert_eq!(hex(&Rlp::Bytes(vec![0x04, 0x00]).encode()), "820400");
        assert_eq!(
            hex(&Rlp::List(vec![
                Rlp::Bytes(b"cat".to_vec()),
                Rlp::Bytes(b"dog".to_vec())
            ])
            .encode()),
            "c88363617483646f67"
        );
    }

    #[test]
    fn rlp_encodes_zero_as_empty_string() {
        // Bẫy kinh điển: RLP(0) KHÔNG phải 0x00 mà là 0x80 (chuỗi rỗng).
        assert_eq!(hex(&Rlp::uint(0).encode()), "80");
        assert_ne!(Rlp::uint(0), Rlp::Bytes(vec![0]));
    }

    #[test]
    fn rlp_integers_have_no_leading_zeros() {
        assert_eq!(Rlp::uint(1024), Rlp::Bytes(vec![0x04, 0x00]));
        assert_eq!(Rlp::uint(255), Rlp::Bytes(vec![0xff]));
        assert_eq!(Rlp::uint(256), Rlp::Bytes(vec![0x01, 0x00]));
    }

    #[test]
    fn long_strings_use_the_long_length_form() {
        let long = vec![b'a'; 100];
        let enc = Rlp::Bytes(long).encode();
        assert_eq!(enc[0], 0xB7 + 1, "0xB7 + số byte cần để ghi độ dài");
        assert_eq!(enc[1], 100);
        assert_eq!(enc.len(), 2 + 100);
    }

    #[test]
    fn rlp_boundary_at_55_and_56_bytes() {
        // 55 byte dùng định dạng ngắn, 56 byte chuyển sang định dạng dài
        assert_eq!(Rlp::Bytes(vec![b'a'; 55]).encode()[0], 0x80 + 55);
        assert_eq!(Rlp::Bytes(vec![b'a'; 56]).encode()[0], 0xB7 + 1);
    }

    // ---------- Giao dịch ----------
    fn sample_tx() -> Tx1559 {
        Tx1559 {
            chain_id: 1,
            nonce: 42,
            max_priority_fee: 2_000_000_000,
            max_fee: 100_000_000_000,
            gas_limit: 21_000,
            to: Some(address_from_hex(
                "0x742d35Cc6634C0532925a3b844Bc454e4438f44e",
            )),
            value: 1_000_000_000_000_000_000, // 1 ETH
            data: vec![],
        }
    }

    #[test]
    fn signing_payload_starts_with_the_tx_type() {
        assert_eq!(
            sample_tx().signing_payload()[0],
            0x02,
            "EIP-1559 là loại 0x02"
        );
    }

    #[test]
    fn changing_any_field_changes_the_hash() {
        // Bất biến sống còn: chữ ký phải phủ TOÀN BỘ nội dung giao dịch.
        // Nếu một trường lọt ra ngoài, kẻ tấn công sửa được nó mà chữ ký vẫn hợp lệ.
        let base = sample_tx();
        let h0 = base.signing_hash();
        let variants: Vec<Tx1559> = vec![
            Tx1559 {
                chain_id: 5,
                ..base.clone()
            },
            Tx1559 {
                nonce: 43,
                ..base.clone()
            },
            Tx1559 {
                max_priority_fee: 3_000_000_000,
                ..base.clone()
            },
            Tx1559 {
                max_fee: 90_000_000_000,
                ..base.clone()
            },
            Tx1559 {
                gas_limit: 30_000,
                ..base.clone()
            },
            Tx1559 {
                to: None,
                ..base.clone()
            },
            Tx1559 {
                value: 2,
                ..base.clone()
            },
            Tx1559 {
                data: vec![1],
                ..base.clone()
            },
        ];
        for (i, v) in variants.iter().enumerate() {
            assert_ne!(v.signing_hash(), h0, "biến thể {} phải cho mã băm khác", i);
        }
    }

    #[test]
    fn contract_creation_encodes_empty_destination() {
        let create = Tx1559 {
            to: None,
            ..sample_tx()
        };
        let call = sample_tx();
        assert_ne!(create.signing_payload(), call.signing_payload());
        // `to: None` phải thành 0x80 (chuỗi rỗng), không phải 20 byte 0
        assert!(create.signing_payload().len() < call.signing_payload().len());
    }

    #[test]
    fn max_cost_matches_the_locking_formula() {
        let tx = sample_tx();
        assert_eq!(
            tx.max_cost(),
            1_000_000_000_000_000_000 + 100_000_000_000 * 21_000
        );
    }

    #[test]
    fn effective_price_never_exceeds_the_user_cap() {
        let tx = sample_tx();
        for base in [1u128, 50_000_000_000, 99_000_000_000, 100_000_000_000] {
            let price = tx.effective_gas_price(base).expect("base ≤ max_fee");
            assert!(
                price <= tx.max_fee,
                "base {} → thực trả {} vượt trần {}",
                base,
                price,
                tx.max_fee
            );
        }
    }

    #[test]
    fn low_base_fee_pays_the_full_tip() {
        let tx = sample_tx();
        let base = 10_000_000_000u128;
        assert_eq!(
            tx.effective_gas_price(base),
            Some(base + tx.max_priority_fee)
        );
    }

    #[test]
    fn base_fee_near_cap_squeezes_the_tip() {
        let tx = sample_tx();
        let base = 99_000_000_000u128; // trần 100 gwei, chỉ còn 1 gwei cho boa
        assert_eq!(
            tx.effective_gas_price(base),
            Some(100_000_000_000),
            "tiền boa bị cắt xuống 1 gwei chứ không phải 2"
        );
    }

    #[test]
    fn base_fee_above_cap_makes_tx_invalid() {
        // Trước đây hàm trả về chính base fee (200 gwei) — VƯỢT trần 100 gwei
        // người dùng đặt, trái với lời hứa "không bao giờ vượt trần". Giao dịch
        // như vậy phải bị coi là không hợp lệ trong khối, không phải bị tính giá.
        let tx = sample_tx();
        assert_eq!(tx.effective_gas_price(200_000_000_000), None);
        assert_eq!(tx.effective_gas_price(100_000_000_001), None);
        assert!(
            tx.effective_gas_price(100_000_000_000).is_some(),
            "đúng bằng trần thì vẫn hợp lệ"
        );
    }
}
