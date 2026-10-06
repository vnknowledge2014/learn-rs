//! Chương 70 — Blockchain từ đầu: SHA-256 tự cài, cây Merkle có bằng chứng,
//! chuỗi khối, bằng chứng công việc, mô hình UTXO, và tái tổ chức chuỗi.

use std::collections::{HashMap, HashSet};
use std::fmt;

// ============================================================================
// 1. SHA-256 — TỰ CÀI TỪ ĐẦU (FIPS 180-4)
// ============================================================================
// Cả blockchain đứng trên MỘT giả định: không ai tìm được hai đầu vào cho cùng
// một giá trị băm. Vì thế ta cài thật thay vì gọi thư viện — bạn cần thấy
// bên trong nó chỉ là phép xoay bit và cộng modulo 2^32.

const K: [u32; 64] = [
    0x428a2f98, 0x71374491, 0xb5c0fbcf, 0xe9b5dba5, 0x3956c25b, 0x59f111f1, 0x923f82a4, 0xab1c5ed5,
    0xd807aa98, 0x12835b01, 0x243185be, 0x550c7dc3, 0x72be5d74, 0x80deb1fe, 0x9bdc06a7, 0xc19bf174,
    0xe49b69c1, 0xefbe4786, 0x0fc19dc6, 0x240ca1cc, 0x2de92c6f, 0x4a7484aa, 0x5cb0a9dc, 0x76f988da,
    0x983e5152, 0xa831c66d, 0xb00327c8, 0xbf597fc7, 0xc6e00bf3, 0xd5a79147, 0x06ca6351, 0x14292967,
    0x27b70a85, 0x2e1b2138, 0x4d2c6dfc, 0x53380d13, 0x650a7354, 0x766a0abb, 0x81c2c92e, 0x92722c85,
    0xa2bfe8a1, 0xa81a664b, 0xc24b8b70, 0xc76c51a3, 0xd192e819, 0xd6990624, 0xf40e3585, 0x106aa070,
    0x19a4c116, 0x1e376c08, 0x2748774c, 0x34b0bcb5, 0x391c0cb3, 0x4ed8aa4a, 0x5b9cca4f, 0x682e6ff3,
    0x748f82ee, 0x78a5636f, 0x84c87814, 0x8cc70208, 0x90befffa, 0xa4506ceb, 0xbef9a3f7, 0xc67178f2,
];

/// Giá trị băm 256 bit. Dùng newtype (Chương 20) để không lẫn với mảng byte thường.
#[derive(Clone, Copy, PartialEq, Eq, Hash, PartialOrd, Ord)]
pub struct Hash256(pub [u8; 32]);

impl Hash256 {
    pub const ZERO: Hash256 = Hash256([0u8; 32]);
    pub fn hex(&self) -> String {
        self.0.iter().map(|b| format!("{:02x}", b)).collect()
    }
    pub fn short(&self) -> String {
        self.hex()[..12].to_string()
    }
    /// Đếm số bit 0 ở đầu — thước đo "độ khó" của bằng chứng công việc.
    pub fn leading_zero_bits(&self) -> u32 {
        let mut n = 0;
        for b in self.0 {
            if b == 0 {
                n += 8;
            } else {
                return n + b.leading_zeros();
            }
        }
        n
    }
}

impl fmt::Debug for Hash256 {
    fn fmt(&self, f: &mut fmt::Formatter) -> fmt::Result {
        write!(f, "Hash256({})", self.short())
    }
}

pub fn sha256(data: &[u8]) -> Hash256 {
    // Giá trị khởi tạo = 32 bit đầu phần thập phân của căn bậc hai 8 số nguyên tố đầu
    let mut h: [u32; 8] = [
        0x6a09e667, 0xbb67ae85, 0x3c6ef372, 0xa54ff53a, 0x510e527f, 0x9b05688c, 0x1f83d9ab,
        0x5be0cd19,
    ];

    // Đệm: thêm bit 1, rồi các bit 0, rồi độ dài 64-bit — sao cho chia hết 512 bit
    let mut m = data.to_vec();
    let bit_length = (data.len() as u64) * 8;
    m.push(0x80);
    while m.len() % 64 != 56 {
        m.push(0);
    }
    m.extend_from_slice(&bit_length.to_be_bytes());

    for block in m.chunks(64) {
        // Mở rộng 16 từ thành 64 từ
        let mut w = [0u32; 64];
        for i in 0..16 {
            w[i] = u32::from_be_bytes([
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

        // 64 vòng nén
        let (mut a, mut b, mut c, mut d) = (h[0], h[1], h[2], h[3]);
        let (mut e, mut f, mut g, mut hh) = (h[4], h[5], h[6], h[7]);
        for i in 0..64 {
            let s1 = e.rotate_right(6) ^ e.rotate_right(11) ^ e.rotate_right(25);
            let ch = (e & f) ^ ((!e) & g);
            let t1 = hh
                .wrapping_add(s1)
                .wrapping_add(ch)
                .wrapping_add(K[i])
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
        for (i, v) in [a, b, c, d, e, f, g, hh].iter().enumerate() {
            h[i] = h[i].wrapping_add(*v);
        }
    }

    let mut out = [0u8; 32];
    for (i, v) in h.iter().enumerate() {
        out[i * 4..i * 4 + 4].copy_from_slice(&v.to_be_bytes());
    }
    Hash256(out)
}

/// Bitcoin dùng SHA-256 HAI LẦN. Lý do lịch sử: phòng tấn công mở rộng độ dài
/// (length-extension) vốn có của cấu trúc Merkle–Damgård.
pub fn sha256d(data: &[u8]) -> Hash256 {
    sha256(&sha256(data).0)
}

// ============================================================================
// 2. CÂY MERKLE — chứng minh "giao dịch này có trong khối" mà không cần tải khối
// ============================================================================

#[derive(Debug, Clone, PartialEq)]
pub struct ProofStep {
    pub sibling_hash: Hash256,
    pub is_right: bool,
}

pub struct MerkleTree {
    pub layers: Vec<Vec<Hash256>>,
}

impl MerkleTree {
    pub fn build(leaves: &[Hash256]) -> MerkleTree {
        if leaves.is_empty() {
            return MerkleTree {
                layers: vec![vec![Hash256::ZERO]],
            };
        }
        let mut layers = vec![leaves.to_vec()];
        while layers.last().unwrap().len() > 1 {
            let below = layers.last().unwrap();
            let mut above = Vec::with_capacity(below.len().div_ceil(2));
            for pair in below.chunks(2) {
                // Số lẻ nút thì nhân đôi nút cuối. Chính chỗ này sinh ra lỗi
                // "CVE-2012-2459" của Bitcoin: hai cây khác nhau cho cùng gốc.
                let (t, p) = (pair[0], *pair.get(1).unwrap_or(&pair[0]));
                let mut v = Vec::with_capacity(64);
                v.extend_from_slice(&t.0);
                v.extend_from_slice(&p.0);
                above.push(sha256d(&v));
            }
            layers.push(above);
        }
        MerkleTree { layers }
    }

    pub fn root(&self) -> Hash256 {
        *self.layers.last().unwrap().first().unwrap()
    }

    /// Bằng chứng gộp: chỉ log₂(n) giá trị băm là đủ chứng minh một lá thuộc cây.
    /// 1 triệu giao dịch → chỉ 20 giá trị băm = 640 byte. Đây là nền của ví nhẹ (SPV).
    pub fn prove(&self, mut index: usize) -> Option<Vec<ProofStep>> {
        if index >= self.layers[0].len() {
            return None;
        }
        let mut proof = Vec::new();
        for layer in &self.layers[..self.layers.len() - 1] {
            let sibling_index = if index.is_multiple_of(2) {
                index + 1
            } else {
                index - 1
            };
            let sibling = *layer.get(sibling_index).unwrap_or(&layer[index]);
            proof.push(ProofStep {
                sibling_hash: sibling,
                is_right: index.is_multiple_of(2),
            });
            index /= 2;
        }
        Some(proof)
    }

    /// Kiểm chứng KHÔNG cần cây — chỉ cần lá, bằng chứng, và gốc.
    pub fn verify(leaves: Hash256, proof: &[ProofStep], root: Hash256) -> bool {
        let mut current = leaves;
        for b in proof {
            let mut v = Vec::with_capacity(64);
            if b.is_right {
                v.extend_from_slice(&current.0);
                v.extend_from_slice(&b.sibling_hash.0);
            } else {
                v.extend_from_slice(&b.sibling_hash.0);
                v.extend_from_slice(&current.0);
            }
            current = sha256d(&v);
        }
        current == root
    }
}

// ============================================================================
// 3. MÔ HÌNH UTXO — "tiền là những tờ chưa tiêu", không phải số dư
// ============================================================================

#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
pub struct OutPoint {
    pub transaction_id: Hash256,
    pub index: u32,
}

#[derive(Debug, Clone, PartialEq)]
pub struct Output {
    pub value: u64,
    pub owner: String,
}

#[derive(Debug, Clone, PartialEq)]
pub struct Transaction {
    pub input: Vec<OutPoint>,
    pub output: Vec<Output>,
    /// Chỉ giao dịch tạo tiền mới có: chiều cao khối chứa nó. Nhờ vậy hai
    /// coinbase trả cùng người cùng số tiền ở hai khối khác nhau vẫn có mã
    /// khác nhau (Bitcoin làm điều này qua BIP34).
    pub coinbase_height: Option<u64>,
}

impl Transaction {
    /// Giao dịch tạo tiền (coinbase): không có đầu vào, sinh tiền từ hư không.
    /// Đây là giao dịch DUY NHẤT được phép làm vậy, và chỉ một lần mỗi khối.
    pub fn coinbase(recipient: &str, value: u64, height: u64) -> Transaction {
        Transaction {
            input: vec![],
            output: vec![Output {
                value,
                owner: recipient.to_string(),
            }],
            coinbase_height: Some(height),
        }
    }
    /// Giao dịch thường: tiêu `input`, tạo `output`.
    pub fn spend(input: Vec<OutPoint>, output: Vec<Output>) -> Transaction {
        Transaction {
            input,
            output,
            coinbase_height: None,
        }
    }
    pub fn is_coinbase(&self) -> bool {
        self.input.is_empty()
    }

    pub fn id(&self) -> Hash256 {
        let mut v = Vec::new();
        if let Some(h) = self.coinbase_height {
            v.extend_from_slice(&h.to_be_bytes());
        }
        for d in &self.input {
            v.extend_from_slice(&d.transaction_id.0);
            v.extend_from_slice(&d.index.to_be_bytes());
        }
        for d in &self.output {
            v.extend_from_slice(&d.value.to_be_bytes());
            // Ghi độ dài trước chuỗi: không có nó, ("ab","c") và ("a","bc") băm như nhau.
            v.extend_from_slice(&(d.owner.len() as u32).to_be_bytes());
            v.extend_from_slice(d.owner.as_bytes());
        }
        sha256d(&v)
    }
}

/// Tập các đầu ra chưa tiêu — TOÀN BỘ trạng thái của một blockchain kiểu Bitcoin.
#[derive(Debug, Clone, Default, PartialEq)]
pub struct UtxoSet {
    pub outputs: HashMap<OutPoint, Output>,
}

#[derive(Debug, PartialEq)]
pub enum TransactionError {
    UnknownInput(OutPoint),
    DoubleSpend(OutPoint),
    SpendsMoreThanReceives { total_in: u64, out: u64 },
    NoOutputs,
}

impl UtxoSet {
    pub fn balance(&self, owner: &str) -> u64 {
        self.outputs
            .values()
            .filter(|d| d.owner == owner)
            .map(|d| d.value)
            .sum()
    }

    /// Kiểm tra một giao dịch mà KHÔNG thay đổi trạng thái. Trả về phí thợ đào.
    pub fn check(
        &self,
        tx: &Transaction,
        spent_in_block: &HashSet<OutPoint>,
    ) -> Result<u64, TransactionError> {
        if tx.output.is_empty() {
            return Err(TransactionError::NoOutputs);
        }
        if tx.is_coinbase() {
            return Ok(0);
        }

        let mut total_in = 0u64;
        let mut seen_in_tx = HashSet::new();
        for input_ref in &tx.input {
            // Tiêu hai lần TRONG CÙNG một giao dịch hoặc cùng một khối
            if spent_in_block.contains(input_ref) || !seen_in_tx.insert(*input_ref) {
                return Err(TransactionError::DoubleSpend(*input_ref));
            }
            match self.outputs.get(input_ref) {
                Some(d) => total_in += d.value,
                None => return Err(TransactionError::UnknownInput(*input_ref)),
            }
        }
        let total_out: u64 = tx.output.iter().map(|d| d.value).sum();
        if total_out > total_in {
            return Err(TransactionError::SpendsMoreThanReceives {
                total_in,
                out: total_out,
            });
        }
        Ok(total_in - total_out) // phần chênh là PHÍ, thợ đào được lấy
    }

    pub fn apply(&mut self, tx: &Transaction) {
        for input_ref in &tx.input {
            self.outputs.remove(input_ref);
        }
        let id = tx.id();
        for (i, d) in tx.output.iter().enumerate() {
            self.outputs.insert(
                OutPoint {
                    transaction_id: id,
                    index: i as u32,
                },
                d.clone(),
            );
        }
    }
}

// ============================================================================
// 4. KHỐI & BẰNG CHỨNG CÔNG VIỆC
// ============================================================================

#[derive(Debug, Clone, PartialEq)]
pub struct BlockHeader {
    pub prev_block_hash: Hash256,
    pub merkle_root: Hash256,
    pub timestamp: u64,
    pub difficulty: u32, // số bit 0 đầu tối thiểu
    pub nonce: u64,
}

impl BlockHeader {
    pub fn id(&self) -> Hash256 {
        let mut v = Vec::with_capacity(88);
        v.extend_from_slice(&self.prev_block_hash.0);
        v.extend_from_slice(&self.merkle_root.0);
        v.extend_from_slice(&self.timestamp.to_be_bytes());
        v.extend_from_slice(&self.difficulty.to_be_bytes());
        v.extend_from_slice(&self.nonce.to_be_bytes());
        sha256d(&v)
    }
    pub fn meets_difficulty(&self) -> bool {
        self.id().leading_zero_bits() >= self.difficulty
    }
}

#[derive(Debug, Clone, PartialEq)]
pub struct Block {
    pub header: BlockHeader,
    pub transactions: Vec<Transaction>,
}

impl Block {
    pub fn id(&self) -> Hash256 {
        self.header.id()
    }

    pub fn recompute_merkle_root(&self) -> Hash256 {
        let leaves: Vec<Hash256> = self.transactions.iter().map(|g| g.id()).collect();
        MerkleTree::build(&leaves).root()
    }

    /// Đào = thử từng số ngẫu nhiên tới khi giá trị băm đủ nhỏ.
    /// KHÔNG có cách nào nhanh hơn thử. Đó chính là "công việc" được chứng minh.
    pub fn mine(&mut self, max_tries: u64) -> Option<u64> {
        for n in 0..max_tries {
            self.header.nonce = n;
            if self.header.meets_difficulty() {
                return Some(n);
            }
        }
        None
    }
}

// ============================================================================
// 5. CHUỖI KHỐI & TÁI TỔ CHỨC
// ============================================================================

#[derive(Debug, PartialEq)]
pub enum BlockError {
    UnknownParent(Hash256),
    BelowDifficulty { got: u32, need: u32 },
    WrongMerkleRoot,
    TimestampWentBackwards,
    MoreThanOneCoinbase,
    CoinbaseOverpays { claimed: u64, allowed: u64 },
    TransactionError(TransactionError),
}

pub struct Chain {
    pub blocks: HashMap<Hash256, Block>,
    pub height: HashMap<Hash256, u64>,
    /// Tổng công việc tích luỹ — tiêu chí chọn nhánh THẬT, không phải chiều cao.
    pub work: HashMap<Hash256, u128>,
    pub tip: Hash256,
    pub block_reward: u64,
}

impl Chain {
    pub fn new(difficulty: u32, block_reward: u64) -> Chain {
        let mut root = Block {
            header: BlockHeader {
                prev_block_hash: Hash256::ZERO,
                merkle_root: Hash256::ZERO,
                timestamp: 0,
                difficulty,
                nonce: 0,
            },
            transactions: vec![Transaction::coinbase("genesis", block_reward, 0)],
        };
        root.header.merkle_root = root.recompute_merkle_root();
        root.mine(1 << 22);
        let id = root.id();
        let mut c = Chain {
            blocks: HashMap::new(),
            height: HashMap::new(),
            work: HashMap::new(),
            tip: id,
            block_reward,
        };
        c.height.insert(id, 0);
        c.work.insert(id, 1u128 << root.header.difficulty);
        c.blocks.insert(id, root);
        c
    }

    pub fn tip_height(&self) -> u64 {
        self.height[&self.tip]
    }

    /// Dựng tập UTXO bằng cách phát lại chuỗi từ khối thuỷ tới `tip`.
    /// Đây là lý do node "lưu trữ đầy đủ" phải giữ toàn bộ lịch sử.
    pub fn utxo_at(&self, tip: Hash256) -> UtxoSet {
        // Lần ngược từ `tip` về khối thuỷ, rồi phát lại theo chiều xuôi.
        let mut path = Vec::new();
        let mut current = tip;
        while let Some(blk) = self.blocks.get(&current) {
            path.push(current);
            if blk.header.prev_block_hash == Hash256::ZERO {
                break;
            }
            current = blk.header.prev_block_hash;
        }
        path.reverse();
        let mut utxo = UtxoSet::default();
        for id in path {
            for tx in &self.blocks[&id].transactions {
                utxo.apply(tx);
            }
        }
        utxo
    }

    pub fn add_block(&mut self, block: Block) -> Result<bool, BlockError> {
        let prev = block.header.prev_block_hash;
        let prev_block = self
            .blocks
            .get(&prev)
            .ok_or(BlockError::UnknownParent(prev))?;

        // --- Kiểm tra phần đầu ---
        let got = block.id().leading_zero_bits();
        if got < block.header.difficulty {
            return Err(BlockError::BelowDifficulty {
                got,
                need: block.header.difficulty,
            });
        }
        if block.recompute_merkle_root() != block.header.merkle_root {
            return Err(BlockError::WrongMerkleRoot);
        }
        if block.header.timestamp < prev_block.header.timestamp {
            return Err(BlockError::TimestampWentBackwards);
        }

        // --- Kiểm tra giao dịch trên UTXO của nhánh cha ---
        let coinbase_count = block
            .transactions
            .iter()
            .filter(|g| g.is_coinbase())
            .count();
        if coinbase_count > 1 {
            return Err(BlockError::MoreThanOneCoinbase);
        }
        let utxo = self.utxo_at(prev);
        let mut spent: HashSet<OutPoint> = HashSet::new();
        let mut total_fees = 0u64;
        for tx in &block.transactions {
            let fee = utxo
                .check(tx, &spent)
                .map_err(BlockError::TransactionError)?;
            total_fees += fee;
            for input_ref in &tx.input {
                spent.insert(*input_ref);
            }
        }
        // Thợ đào chỉ được lấy phần thưởng + phí, không hơn một xu
        if let Some(cb) = block.transactions.iter().find(|g| g.is_coinbase()) {
            let claimed: u64 = cb.output.iter().map(|d| d.value).sum();
            let allowed = self.block_reward + total_fees;
            if claimed > allowed {
                return Err(BlockError::CoinbaseOverpays { claimed, allowed });
            }
        }

        // --- Ghi nhận ---
        let id = block.id();
        let h = self.height[&prev] + 1;
        let w = self.work[&prev] + (1u128 << block.header.difficulty);
        self.height.insert(id, h);
        self.work.insert(id, w);
        self.blocks.insert(id, block);

        // Chọn nhánh theo TỔNG CÔNG VIỆC, không phải chiều cao. Một chuỗi ngắn
        // nhưng khó hơn vẫn thắng — đó là quy tắc thật của Bitcoin.
        if w > self.work[&self.tip] {
            self.tip = id;
            return Ok(true); // đã tái tổ chức / mở rộng đỉnh
        }
        Ok(false)
    }

    /// Tạo khối kế tiếp đã đào xong, gắn lên MỘT KHỐI CHA BẤT KỲ.
    ///
    /// Nhận `parent` tường minh chứ không mặc định lấy đỉnh — nếu không, muốn dựng
    /// một nhánh rẽ ta buộc phải gán tay `self.tip`, và thế là phá vỡ bất biến
    /// "đỉnh luôn là khối nhiều công việc nhất". Chính bất biến đó là thứ hàm
    /// `add_block` dựa vào để quyết định có tái tổ chức hay không.
    pub fn mine_on(
        &self,
        parent: Hash256,
        miner: &str,
        transactions: Vec<Transaction>,
        timestamp: u64,
    ) -> Option<Block> {
        let parent_block = self.blocks.get(&parent)?;
        let h = self.height.get(&parent)? + 1;
        let utxo = self.utxo_at(parent);
        let mut spent = HashSet::new();
        let mut fee = 0u64;
        for tx in &transactions {
            fee += utxo.check(tx, &spent).ok()?;
            for input_ref in &tx.input {
                spent.insert(*input_ref);
            }
        }
        let mut all = vec![Transaction::coinbase(miner, self.block_reward + fee, h)];
        all.extend(transactions);
        let mut blk = Block {
            header: BlockHeader {
                prev_block_hash: parent,
                merkle_root: Hash256::ZERO,
                timestamp: timestamp.max(parent_block.header.timestamp),
                difficulty: parent_block.header.difficulty,
                nonce: 0,
            },
            transactions: all,
        };
        blk.header.merkle_root = blk.recompute_merkle_root();
        blk.mine(1 << 24)?;
        Some(blk)
    }

    /// Tiện lợi: đào tiếp lên đỉnh hiện tại — trường hợp thường gặp nhất.
    pub fn mine_new_block(
        &self,
        miner: &str,
        transactions: Vec<Transaction>,
        timestamp: u64,
    ) -> Option<Block> {
        self.mine_on(self.tip, miner, transactions, timestamp)
    }

    /// Mã của khối thuỷ (chiều cao 0).
    pub fn genesis(&self) -> Hash256 {
        *self
            .height
            .iter()
            .find(|&(_, &v)| v == 0)
            .map(|(m, _)| m)
            .unwrap()
    }
}

fn main() {
    println!("═══════════════════════════════════════════════════════════");
    println!("   BLOCKCHAIN TỪ ĐẦU: SHA-256 · MERKLE · UTXO · ĐÀO · REORG ");
    println!("═══════════════════════════════════════════════════════════");

    println!("\n1. SHA-256 TỰ CÀI — đối chiếu vector chuẩn FIPS 180-4");
    println!("   sha256(\"\")    = {}", sha256(b"").hex());
    println!(
        "   kỳ vọng        = e3b0c44298fc1c149afbf4c8996fb92427ae41e4649b934ca495991b7852b855"
    );
    println!("   sha256(\"abc\") = {}", sha256(b"abc").hex());
    println!(
        "   kỳ vọng        = ba7816bf8f01cfea414140de5dae2223b00361a396177a9cb410ff61f20015ad"
    );

    println!("\n2. HIỆU ỨNG TUYẾT LỞ — đổi 1 bit, nửa số bit đầu ra đổi theo");
    let a = sha256(b"block");
    let b = sha256(b"blocj"); // đổi đúng một ký tự (i → j)
    let diff_bits: u32 =
        a.0.iter()
            .zip(b.0.iter())
            .map(|(x, y)| (x ^ y).count_ones())
            .sum();
    println!("   {} \n   {}", a.short(), b.short());
    println!(
        "   Số bit khác nhau: {}/256 ({}%)",
        diff_bits,
        diff_bits * 100 / 256
    );

    println!("\n3. CÂY MERKLE — bằng chứng gộp");
    let leaves: Vec<Hash256> = (0..8u32).map(|i| sha256(&i.to_be_bytes())).collect();
    let tree = MerkleTree::build(&leaves);
    println!(
        "   8 lá → gốc {} ({} tầng)",
        tree.root().short(),
        tree.layers.len()
    );
    let proof = tree.prove(3).unwrap();
    println!(
        "   Bằng chứng cho lá #3: {} giá trị băm ({} byte) thay vì cả 8 lá",
        proof.len(),
        proof.len() * 32
    );
    println!(
        "   Kiểm chứng đúng lá : {}",
        MerkleTree::verify(leaves[3], &proof, tree.root())
    );
    println!(
        "   Kiểm chứng lá giả  : {}",
        MerkleTree::verify(sha256(b"forged"), &proof, tree.root())
    );

    println!("\n4. ĐÀO KHỐI — chi phí tăng theo cấp số nhân");
    for difficulty in [8u32, 12, 16] {
        let mut blk = Block {
            header: BlockHeader {
                prev_block_hash: Hash256::ZERO,
                merkle_root: Hash256::ZERO,
                timestamp: 0,
                difficulty,
                nonce: 0,
            },
            transactions: vec![Transaction::coinbase("miner", 50, 0)],
        };
        blk.header.merkle_root = blk.recompute_merkle_root();
        let n = blk.mine(1 << 24).unwrap();
        println!(
            "   {:>2} bit 0 đầu → {:>8} lần thử (kỳ vọng ≈ {:>6}) · băm {}",
            difficulty,
            n,
            1u64 << difficulty,
            blk.id().short()
        );
    }
    println!(
        "   → Mỗi bit độ khó tăng thêm, công sức KỲ VỌNG nhân đôi (từng lần đào thì hên xui)."
    );

    println!("\n5. CHUỖI, UTXO & TIÊU HAI LẦN");
    let mut c = Chain::new(10, 50);
    let k1 = c.mine_new_block("An", vec![], 1).unwrap();
    c.add_block(k1).unwrap();
    let k2 = c.mine_new_block("An", vec![], 2).unwrap();
    c.add_block(k2).unwrap();
    let utxo = c.utxo_at(c.tip);
    println!(
        "   Chiều cao {} · số dư An = {} · số UTXO = {}",
        c.tip_height(),
        utxo.balance("An"),
        utxo.outputs.len()
    );

    let input = *utxo
        .outputs
        .keys()
        .find(|blk| utxo.outputs[blk].owner == "An")
        .unwrap();
    let tx = Transaction {
        input: vec![input],
        output: vec![
            Output {
                value: 30,
                owner: "Binh".into(),
            },
            Output {
                value: 15,
                owner: "An".into(),
            },
        ],
        coinbase_height: None,
    };
    println!(
        "   An trả Bình 30 (phí {} cho thợ đào)",
        utxo.check(&tx, &HashSet::new()).unwrap()
    );
    let tx2 = Transaction {
        input: vec![input],
        output: vec![Output {
            value: 45,
            owner: "Cuong".into(),
        }],
        coinbase_height: None,
    };
    let mut spent = HashSet::new();
    spent.insert(input);
    println!(
        "   Tiêu lại chính đồng đó → {:?}",
        utxo.check(&tx2, &spent).unwrap_err()
    );

    println!("\n6. TÁI TỔ CHỨC CHUỖI — nhánh nhiều CÔNG VIỆC hơn thắng");
    println!(
        "   Trước : đỉnh {} cao {} · số dư An = {}",
        c.tip.short(),
        c.tip_height(),
        c.utxo_at(c.tip).balance("An")
    );
    // Kẻ tấn công đào lại từ khối thuỷ, xây nhánh riêng cho tới khi vượt
    let mut parent = c.genesis();
    for t in 1..=3u64 {
        let blk = c.mine_on(parent, "Attacker", vec![], t + 10).unwrap();
        parent = blk.id();
        let took_tip = c.add_block(blk).unwrap();
        println!(
            "   Nhánh tấn công cao {} → {}",
            t,
            if took_tip {
                "ĐÃ CHIẾM ĐỈNH"
            } else {
                "chưa đủ công việc"
            }
        );
    }
    println!(
        "   Sau  : đỉnh {} cao {} · số dư An = {} (khối của An BỊ ĐẢO)",
        c.tip.short(),
        c.tip_height(),
        c.utxo_at(c.tip).balance("An")
    );

    println!("\n═══════════════════════════════════════════════════════════");
    println!("   BLOCKCHAIN = CẤU TRÚC DỮ LIỆU + LUẬT + ĐỘNG CƠ KHUYẾN KHÍCH");
    println!("═══════════════════════════════════════════════════════════");
}

#[cfg(test)]
mod tests {
    use super::*;

    // ---------- SHA-256 đối chiếu vector chuẩn ----------
    #[test]
    fn sha256_matches_fips_vectors() {
        // Đây là bài kiểm thử quan trọng nhất chương: nếu sai một bit,
        // toàn bộ chuỗi khối phía trên đều vô nghĩa.
        assert_eq!(
            sha256(b"").hex(),
            "e3b0c44298fc1c149afbf4c8996fb92427ae41e4649b934ca495991b7852b855"
        );
        assert_eq!(
            sha256(b"abc").hex(),
            "ba7816bf8f01cfea414140de5dae2223b00361a396177a9cb410ff61f20015ad"
        );
        assert_eq!(
            sha256(b"abcdbcdecdefdefgefghfghighijhijkijkljklmklmnlmnomnopnopq").hex(),
            "248d6a61d20638b8e5c026930c3e6039a33ce45964ff2167f6ecedd419db06c1"
        );
    }

    #[test]
    fn sha256_correct_at_every_padding_boundary() {
        // 55 byte = vừa đủ đệm trong 1 khối; 56 byte = phải sang khối thứ hai.
        // Đây là chỗ cài đặt SHA-256 hay sai nhất.
        assert_eq!(
            sha256(&[b'a'; 55]).hex(),
            "9f4390f8d30c2dd92ec9f095b65e2b9ae9b0a925a5258e241c9f1e910f734318"
        );
        assert_eq!(
            sha256(&[b'a'; 56]).hex(),
            "b35439a4ac6f0948b6d6f9e3c6af0f5f590ce20f1bde7090ef7970686ec6738a"
        );
        assert_eq!(
            sha256(&[b'a'; 64]).hex(),
            "ffe054fe7ae0cb6dc65c3af9b61d5209f439851db43d0ba5997337df154668eb"
        );
    }

    #[test]
    fn sha256d_is_double_hash() {
        assert_eq!(sha256d(b"abc"), sha256(&sha256(b"abc").0));
    }

    #[test]
    fn avalanche_flips_about_half_the_bits() {
        // Tiêu chuẩn vàng của hàm băm mật mã: đổi 1 bit đầu vào phải làm
        // khoảng 50% bit đầu ra đổi theo, không thể đoán được bit nào.
        let mut total = 0u32;
        let n = 64;
        for i in 0..n {
            let a = sha256(&[i as u8, 0]);
            let b = sha256(&[i as u8, 1]); // khác đúng 1 bit
            total +=
                a.0.iter()
                    .zip(b.0.iter())
                    .map(|(x, y)| (x ^ y).count_ones())
                    .sum::<u32>();
        }
        let avg = total as f64 / n as f64;
        assert!(
            (avg - 128.0).abs() < 15.0,
            "trung bình {} bit đổi, kỳ vọng ~128",
            avg
        );
    }

    #[test]
    fn leading_zero_bits_is_correct() {
        assert_eq!(Hash256::ZERO.leading_zero_bits(), 256);
        let mut b = [0u8; 32];
        b[0] = 0xFF;
        assert_eq!(Hash256(b).leading_zero_bits(), 0);
        let mut b = [0u8; 32];
        b[1] = 0x01;
        assert_eq!(Hash256(b).leading_zero_bits(), 15); // 8 bit của byte 0 + 7 bit của byte 1
    }

    // ---------- Cây Merkle ----------
    #[test]
    fn every_leaf_has_a_valid_proof() {
        for n in [1usize, 2, 3, 4, 5, 8, 9, 16, 17] {
            let leaves: Vec<Hash256> = (0..n as u32).map(|i| sha256(&i.to_be_bytes())).collect();
            let tree = MerkleTree::build(&leaves);
            for (i, &leaf) in leaves.iter().enumerate() {
                let proof = tree.prove(i).expect("phải có bằng chứng");
                assert!(
                    MerkleTree::verify(leaf, &proof, tree.root()),
                    "n={} lá #{} không kiểm chứng được",
                    n,
                    i
                );
            }
        }
    }

    #[test]
    fn merkle_rejects_forged_leaf() {
        let leaves: Vec<Hash256> = (0..8u32).map(|i| sha256(&i.to_be_bytes())).collect();
        let tree = MerkleTree::build(&leaves);
        let proof = tree.prove(3).unwrap();
        assert!(!MerkleTree::verify(
            sha256(b"forged leaf"),
            &proof,
            tree.root()
        ));
    }

    #[test]
    fn proof_length_is_logarithmic() {
        let leaves: Vec<Hash256> = (0..1024u32).map(|i| sha256(&i.to_be_bytes())).collect();
        let tree = MerkleTree::build(&leaves);
        let proof = tree.prove(500).unwrap();
        assert_eq!(
            proof.len(),
            10,
            "1024 lá → log2(1024) = 10 bước, không phải 1024"
        );
        assert!(MerkleTree::verify(leaves[500], &proof, tree.root()));
    }

    #[test]
    fn changing_one_leaf_changes_the_root() {
        let mut leaves: Vec<Hash256> = (0..8u32).map(|i| sha256(&i.to_be_bytes())).collect();
        let old_root = MerkleTree::build(&leaves).root();
        leaves[5] = sha256(b"tampered");
        assert_ne!(
            MerkleTree::build(&leaves).root(),
            old_root,
            "sửa bất kỳ lá nào phải lộ ra ở gốc"
        );
    }

    #[test]
    fn out_of_range_index_returns_none() {
        let leaves: Vec<Hash256> = (0..4u32).map(|i| sha256(&i.to_be_bytes())).collect();
        assert!(MerkleTree::build(&leaves).prove(4).is_none());
    }

    // ---------- UTXO ----------
    fn chain_with_funds(owner: &str) -> (Chain, OutPoint) {
        let mut c = Chain::new(8, 50);
        let blk = c.mine_new_block(owner, vec![], 1).unwrap();
        c.add_block(blk).unwrap();
        let utxo = c.utxo_at(c.tip);
        let input_ref = *utxo
            .outputs
            .keys()
            .find(|blk| utxo.outputs[blk].owner == owner)
            .unwrap();
        (c, input_ref)
    }

    #[test]
    fn valid_tx_returns_fee() {
        let (c, input_ref) = chain_with_funds("An");
        let utxo = c.utxo_at(c.tip);
        let tx = Transaction {
            input: vec![input_ref],
            output: vec![Output {
                value: 45,
                owner: "Binh".into(),
            }],
            coinbase_height: None,
        };
        assert_eq!(
            utxo.check(&tx, &HashSet::new()),
            Ok(5),
            "50 vào - 45 ra = 5 phí"
        );
    }

    #[test]
    fn cannot_spend_more_than_received() {
        let (c, input_ref) = chain_with_funds("An");
        let utxo = c.utxo_at(c.tip);
        let tx = Transaction {
            input: vec![input_ref],
            output: vec![Output {
                value: 999,
                owner: "An".into(),
            }],
            coinbase_height: None,
        };
        assert_eq!(
            utxo.check(&tx, &HashSet::new()),
            Err(TransactionError::SpendsMoreThanReceives {
                total_in: 50,
                out: 999
            })
        );
    }

    #[test]
    fn no_double_spend_within_one_tx() {
        let (c, input_ref) = chain_with_funds("An");
        let utxo = c.utxo_at(c.tip);
        // Dùng CÙNG một đầu vào hai lần để "nhân đôi" tiền
        let tx = Transaction {
            input: vec![input_ref, input_ref],
            output: vec![Output {
                value: 100,
                owner: "An".into(),
            }],
            coinbase_height: None,
        };
        assert_eq!(
            utxo.check(&tx, &HashSet::new()),
            Err(TransactionError::DoubleSpend(input_ref))
        );
    }

    #[test]
    fn cannot_spend_nonexistent_input() {
        let (c, _) = chain_with_funds("An");
        let utxo = c.utxo_at(c.tip);
        let id = OutPoint {
            transaction_id: sha256(b"made up"),
            index: 0,
        };
        let tx = Transaction {
            input: vec![id],
            output: vec![Output {
                value: 1,
                owner: "An".into(),
            }],
            coinbase_height: None,
        };
        assert_eq!(
            utxo.check(&tx, &HashSet::new()),
            Err(TransactionError::UnknownInput(id))
        );
    }

    #[test]
    fn balance_matches_owner_exactly() {
        // Trước đây số dư dùng `starts_with`, nên "An" nuốt luôn tiền của "Anh".
        let mut utxo = UtxoSet::default();
        utxo.apply(&Transaction::coinbase("An", 50, 1));
        utxo.apply(&Transaction::coinbase("Anh", 70, 2));
        assert_eq!(utxo.balance("An"), 50);
        assert_eq!(utxo.balance("Anh"), 70);
    }

    #[test]
    fn coinbases_at_different_heights_have_different_ids() {
        assert_ne!(
            Transaction::coinbase("An", 50, 1).id(),
            Transaction::coinbase("An", 50, 2).id()
        );
    }

    #[test]
    fn applying_tx_conserves_value_minus_fee() {
        let (c, input_ref) = chain_with_funds("An");
        let mut utxo = c.utxo_at(c.tip);
        let prev: u64 = utxo.outputs.values().map(|d| d.value).sum();
        let tx = Transaction {
            input: vec![input_ref],
            output: vec![
                Output {
                    value: 30,
                    owner: "Binh".into(),
                },
                Output {
                    value: 18,
                    owner: "An".into(),
                },
            ],
            coinbase_height: None,
        };
        let fee = utxo.check(&tx, &HashSet::new()).unwrap();
        utxo.apply(&tx);
        let next: u64 = utxo.outputs.values().map(|d| d.value).sum();
        assert_eq!(
            prev - next,
            fee,
            "chênh lệch đúng bằng phí, không mất mát ở đâu khác"
        );
    }

    // ---------- Khối & đào ----------
    #[test]
    fn mining_finds_a_nonce_meeting_difficulty() {
        let mut blk = Block {
            header: BlockHeader {
                prev_block_hash: Hash256::ZERO,
                merkle_root: Hash256::ZERO,
                timestamp: 0,
                difficulty: 12,
                nonce: 0,
            },
            transactions: vec![Transaction::coinbase("x", 50, 0)],
        };
        blk.header.merkle_root = blk.recompute_merkle_root();
        assert!(blk.mine(1 << 22).is_some());
        assert!(blk.header.meets_difficulty());
        assert!(blk.id().leading_zero_bits() >= 12);
    }

    #[test]
    fn verifying_is_far_cheaper_than_mining() {
        // Bất đối xứng này là toàn bộ ý nghĩa của "bằng chứng công việc":
        // tìm thì tốn hàng nghìn lần thử, kiểm tra chỉ tốn MỘT lần băm.
        let mut blk = Block {
            header: BlockHeader {
                prev_block_hash: Hash256::ZERO,
                merkle_root: Hash256::ZERO,
                timestamp: 0,
                difficulty: 14,
                nonce: 0,
            },
            transactions: vec![Transaction::coinbase("x", 50, 0)],
        };
        blk.header.merkle_root = blk.recompute_merkle_root();
        let tries = blk.mine(1 << 24).unwrap();
        assert!(
            tries > 100,
            "độ khó 14 bit phải tốn nhiều lần thử, thực tế {}",
            tries
        );
        assert!(
            blk.header.meets_difficulty(),
            "nhưng kiểm tra chỉ cần 1 phép băm"
        );
    }

    // ---------- Chuỗi ----------
    #[test]
    fn new_chain_has_valid_genesis() {
        let c = Chain::new(8, 50);
        assert_eq!(c.tip_height(), 0);
        assert_eq!(c.utxo_at(c.tip).balance("genesis"), 50);
    }

    #[test]
    fn adding_block_raises_height_and_balances() {
        let mut c = Chain::new(8, 50);
        for i in 1..=3u64 {
            let blk = c.mine_new_block("An", vec![], i).unwrap();
            assert_eq!(c.add_block(blk), Ok(true));
            assert_eq!(c.tip_height(), i);
        }
        assert_eq!(c.utxo_at(c.tip).balance("An"), 150, "3 khối × 50");
    }

    #[test]
    fn rejects_block_below_difficulty() {
        let mut c = Chain::new(12, 50);
        let mut blk = c.mine_new_block("An", vec![], 1).unwrap();
        blk.header.nonce = blk.header.nonce.wrapping_add(1); // phá bằng chứng
        assert!(matches!(
            c.add_block(blk),
            Err(BlockError::BelowDifficulty { .. })
        ));
    }

    #[test]
    fn rejects_block_with_wrong_merkle_root() {
        let mut c = Chain::new(8, 50);
        let mut blk = c.mine_new_block("An", vec![], 1).unwrap();
        // Nhét thêm giao dịch mà không cập nhật gốc Merkle — đúng kiểu tấn công
        // "đổi nội dung nhưng giữ nguyên bằng chứng công việc"
        blk.transactions
            .push(Transaction::coinbase("Cheater", 1000, 99));
        assert!(matches!(
            c.add_block(blk),
            Err(BlockError::WrongMerkleRoot) | Err(BlockError::BelowDifficulty { .. })
        ));
    }

    #[test]
    fn rejects_miner_overpaying_itself() {
        let mut c = Chain::new(8, 50);
        let mut blk = c.mine_new_block("An", vec![], 1).unwrap();
        blk.transactions[0] = Transaction::coinbase("An", 1_000_000, 1); // tham lam
        blk.header.merkle_root = blk.recompute_merkle_root();
        blk.mine(1 << 20);
        assert!(matches!(
            c.add_block(blk),
            Err(BlockError::CoinbaseOverpays { .. })
        ));
    }

    #[test]
    fn rejects_orphan_block() {
        let mut c = Chain::new(8, 50);
        let missing_parent = sha256(b"no such block");
        let mut blk = Block {
            header: BlockHeader {
                prev_block_hash: missing_parent,
                merkle_root: Hash256::ZERO,
                timestamp: 5,
                difficulty: 8,
                nonce: 0,
            },
            transactions: vec![Transaction::coinbase("An", 50, 1)],
        };
        blk.header.merkle_root = blk.recompute_merkle_root();
        blk.mine(1 << 20);
        assert_eq!(
            c.add_block(blk),
            Err(BlockError::UnknownParent(missing_parent))
        );
    }

    #[test]
    fn reorg_when_side_branch_outweighs() {
        let mut c = Chain::new(8, 50);
        let genesis_id = c.genesis();

        // Nhánh chính: An đào 2 khối
        for i in 1..=2u64 {
            let blk = c.mine_new_block("An", vec![], i).unwrap();
            assert_eq!(c.add_block(blk), Ok(true));
        }
        let an_tip = c.tip;
        assert_eq!(c.tip_height(), 2);
        assert_eq!(c.utxo_at(c.tip).balance("An"), 100);

        // Nhánh rẽ dựng từ khối thuỷ — KHÔNG đụng tới self.tip
        let d1 = c.mine_on(genesis_id, "Rival", vec![], 10).unwrap();
        let d1_id = d1.id();
        assert_eq!(
            c.add_block(d1),
            Ok(false),
            "cao 1 < cao 2 → chưa chiếm được đỉnh"
        );
        assert_eq!(c.tip, an_tip, "đỉnh vẫn phải là nhánh nhiều công việc hơn");

        let d2 = c.mine_on(d1_id, "Rival", vec![], 11).unwrap();
        let d2_id = d2.id();
        assert_eq!(
            c.add_block(d2),
            Ok(false),
            "hoà 2-2 thì người tới sau KHÔNG được lật"
        );
        assert_eq!(c.tip, an_tip);

        // Khối thứ ba mới đủ vượt
        let d3 = c.mine_on(d2_id, "Rival", vec![], 12).unwrap();
        assert_eq!(c.add_block(d3), Ok(true), "cao 3 > cao 2 → TÁI TỔ CHỨC");
        assert_eq!(c.tip_height(), 3);
        assert_eq!(c.utxo_at(c.tip).balance("Rival"), 150);
        assert_eq!(
            c.utxo_at(c.tip).balance("An"),
            0,
            "toàn bộ khối của An bị ĐẢO — đó chính là ý nghĩa của 'chờ đủ xác nhận'"
        );

        // Nhánh cũ vẫn nằm trong kho, chỉ là không còn nằm trên đường tới đỉnh
        assert!(c.blocks.contains_key(&an_tip));
        assert_eq!(
            c.utxo_at(an_tip).balance("An"),
            100,
            "phát lại nhánh cũ vẫn ra kết quả cũ"
        );
    }

    #[test]
    fn tie_on_work_keeps_first_tip() {
        // Quy tắc chống dao động: chỉ đổi đỉnh khi THỰC SỰ nhiều việc hơn,
        // không đổi khi bằng. Nếu không, mạng sẽ lật qua lật lại vô nghĩa.
        let mut c = Chain::new(8, 50);
        let genesis_id = c.genesis();
        let a = c.mine_new_block("A", vec![], 1).unwrap();
        let a_id = a.id();
        c.add_block(a).unwrap();
        let b = c.mine_on(genesis_id, "B", vec![], 2).unwrap();
        assert_eq!(c.add_block(b), Ok(false));
        assert_eq!(c.tip, a_id, "cùng công việc → giữ nguyên đỉnh cũ");
    }

    #[test]
    fn every_block_hashes_uniquely() {
        let mut c = Chain::new(8, 50);
        let mut seen = HashSet::new();
        seen.insert(c.tip);
        for i in 1..=5u64 {
            let blk = c.mine_new_block("An", vec![], i).unwrap();
            let id = blk.id();
            assert!(seen.insert(id), "trùng mã khối — không được xảy ra");
            c.add_block(blk).unwrap();
        }
    }
}
