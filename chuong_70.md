# Chương 70: Blockchain từ đầu — SHA-256, Cây Merkle, UTXO & Bằng chứng công việc (Building a Blockchain from Scratch)

## Giới thiệu & Mục tiêu học tập

Blockchain thường được kể như một điều huyền bí. Thực ra nó là **bốn ý tưởng cũ ghép lại**, và chúng ta sẽ dựng lại cả bốn từ số không — không thư viện, không phép màu:

| Thành phần | Ý tưởng cốt lõi | Có từ năm |
|---|---|---|
| Hàm băm mật mã | Đổi 1 bit đầu vào → đổi nửa số bit đầu ra | 1979 |
| Cây Merkle | Chứng minh "có trong tập" bằng log₂(n) giá trị băm | 1979 |
| Bằng chứng công việc | Tìm thì rất khó, kiểm tra thì rất dễ | 1993 |
| Chuỗi băm | Sửa quá khứ làm hỏng mọi thứ phía sau | 1991 |

Điều Bitcoin thêm vào không phải công nghệ mới, mà là **động cơ khuyến khích**: gắn tiền vào việc trung thực, khiến tấn công tốn kém hơn tuân thủ.

Mục tiêu học tập:
- Tự cài **SHA-256** từ đầu và đối chiếu với vector chuẩn FIPS 180-4.
- Dựng **cây Merkle** có bằng chứng gộp — nền của ví nhẹ.
- Hiểu **mô hình UTXO**: tiền là "những tờ chưa tiêu", không phải số dư.
- Cài **bằng chứng công việc** và tự thấy chi phí tăng theo cấp số nhân.
- Hiểu **tái tổ chức chuỗi** và vì sao phải "chờ đủ xác nhận".

---

## Hình tượng hóa đời sống

```
┌──────────────────────────────────────────────────────────────────────────────┐
│   HÌNH TƯỢNG: CUỐN SỔ CÁI CỦA CẢ LÀNG, AI CŨNG GIỮ MỘT BẢN                   │
├──────────────────────────────────────────────────────────────────────────────┤
│                                                                              │
│  HÀM BĂM = DẤU LĂN TAY CỦA MỘT TRANG GIẤY                                    │
│    Đổi một dấu phẩy trong trang → dấu lăn tay đổi hoàn toàn.                 │
│    Không ai làm ngược được: từ dấu lăn tay không dựng lại được trang.        │
│                                                                              │
│  CHUỖI KHỐI = MỖI TRANG GHI DẤU LĂN TAY CỦA TRANG TRƯỚC                      │
│    ┌────────┐   ┌────────┐   ┌────────┐                                      │
│    │ Khối 1 │◄──│ Khối 2 │◄──│ Khối 3 │                                      │
│    └────────┘   └────────┘   └────────┘                                      │
│    Sửa khối 1 → dấu lăn tay khối 1 đổi → khối 2 trỏ sai → khối 3 sai...      │
│    Muốn sửa một trang, phải làm lại TOÀN BỘ các trang sau nó.                │
│                                                                              │
│  BẰNG CHỨNG CÔNG VIỆC = TÌM MỘT CON SỐ MAY MẮN                               │
│    "Hãy tìm số n sao cho băm(trang + n) bắt đầu bằng 20 số 0."               │
│    Tìm: phải thử hàng triệu lần. Kiểm tra: băm MỘT lần là xong.              │
│    Đó là toàn bộ cơ chế bảo vệ — bất đối xứng giữa làm và kiểm.              │
│                                                                              │
│  CÂY MERKLE = MỤC LỤC NHIỀU TẦNG                                             │
│         gốc              Muốn chứng minh giao dịch #3 có trong khối          │
│        /    \            chứa 1 triệu giao dịch? Không cần gửi cả            │
│      ab      cd          triệu — chỉ cần 20 giá trị băm dọc đường            │
│     /  \    /  \         từ lá lên gốc. 640 byte thay vì hàng trăm MB.       │
│    a    b  c    d                                                            │
│                                                                              │
│  TÁI TỔ CHỨC = HAI PHIÊN BẢN SỔ, LÀNG CHỌN BẢN "TỐN CÔNG NHẤT"               │
│    Giao dịch của bạn nằm ở nhánh thua → nó BIẾN MẤT như chưa từng có.        │
│    Đó là lý do người bán hàng chờ 6 xác nhận trước khi giao hàng.            │
└──────────────────────────────────────────────────────────────────────────────┘
```

---

## Khái niệm & Cơ chế kỹ thuật chuyên sâu

### 1. SHA-256 chỉ là xoay bit và cộng

Nhìn kỹ thì SHA-256 không có gì huyền bí: 64 vòng, mỗi vòng chỉ gồm phép xoay bit, XOR, AND, và cộng modulo 2³². Sức mạnh của nó đến từ **số lượng vòng** và cách chúng trộn dữ liệu, chứ không phải từ một phép toán bí ẩn nào.

Ba chi tiết dễ sai khi tự cài:
- **Đệm** phải theo đúng quy tắc: thêm bit 1, rồi các bit 0, rồi 8 byte độ dài. Sai ở đây thì input ngắn vẫn đúng mà input dài thì hỏng — nên chương này có bài kiểm thử ở đúng các mốc 55/56/64 byte.
- **Big-endian** ở mọi chỗ. Nhầm sang little-endian cho ra kết quả hoàn toàn khác.
- **`wrapping_add`** chứ không phải `+`. SHA-256 dùng số học modulo 2³², và trong Rust bản debug thì `+` sẽ panic khi tràn.

Bitcoin dùng SHA-256 **hai lần**. Satoshi chưa từng giải thích chính thức; lý do thường được nêu: cấu trúc Merkle–Damgård của SHA-256 có điểm yếu "mở rộng độ dài" — biết `băm(m)` và độ dài của `m` thì tính được `băm(m ‖ đệm ‖ x)` mà không cần biết `m`. Băm hai lần bịt lỗ hổng đó.

### 2. Bằng chứng gộp của cây Merkle

Đây là ý tưởng đẹp nhất trong cả chương. Muốn chứng minh lá thứ 500 nằm trong cây 1024 lá, bạn **không cần** gửi cả 1024 lá. Chỉ cần 10 giá trị băm — mỗi tầng một giá trị "anh em" — và người kiểm chứng tự tính ngược lên gốc.

Đây chính là cơ chế cho phép ví trên điện thoại xác minh giao dịch mà không tải cả blockchain hàng trăm gigabyte.

Một cạm bẫy có thật: khi số lá lẻ, ta nhân đôi nút cuối. Cách làm này từng gây ra **CVE-2012-2459** của Bitcoin — hai danh sách giao dịch khác nhau cho ra cùng một gốc Merkle. Bài học: mọi lựa chọn "cho tiện" trong cấu trúc dữ liệu mật mã đều cần được soi kỹ.

### 3. UTXO: tiền là những tờ giấy, không phải con số

Hầu hết mọi người hình dung tài khoản có một **số dư**. Bitcoin không làm vậy. Nó theo dõi những **đầu ra chưa tiêu** — giống như ví đựng các tờ tiền mệnh giá khác nhau.

Tiêu tiền nghĩa là: phá huỷ vài tờ cũ, tạo ra vài tờ mới. Muốn trả 30 mà chỉ có tờ 50 thì phải tạo hai tờ mới: 30 cho người nhận, 19 trả lại mình, và 1 làm phí cho thợ đào.

Trong mã của chương, `UtxoSet::balance` cộng các tờ có chủ **đúng bằng** tên cần tra (so khớp tuyệt đối — so tiền tố thì "An" sẽ nuốt luôn tiền của "Anh"), còn giao dịch tạo tiền mang chiều cao khối (`coinbase_height`, như BIP34 của Bitcoin) để hai coinbase giống hệt nhau ở hai khối khác nhau vẫn có mã khác nhau.

Ưu điểm của mô hình này: **kiểm tra tiêu hai lần trở nên cực đơn giản** — chỉ cần hỏi "tờ này còn trong tập chưa tiêu không?". Và các giao dịch không dùng chung tờ nào thì kiểm chứng song song được.

### 4. Chọn nhánh theo CÔNG VIỆC, không theo chiều dài

Sách phổ thông hay nói "chuỗi dài nhất thắng". Điều đó **không chính xác**. Quy tắc thật là **tổng công việc tích luỹ lớn nhất** thắng. Khi độ khó thay đổi giữa các khối, một chuỗi ngắn hơn nhưng gồm các khối khó hơn vẫn có thể thắng.

Chương này cũng cài một quy tắc quan trọng khác: **hoà thì giữ nguyên đỉnh cũ**. Nếu đổi đỉnh khi bằng điểm, mạng sẽ lật qua lật lại vô nghĩa mỗi lần có khối mới.

---

## Mã nguồn minh họa thực chiến

Chạy bằng `cargo run -p ch70`, kiểm thử bằng `cargo test -p ch70`.

```rust
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
```

---

## Bảng tra cứu lỗi biên dịch thường gặp

| Lỗi | Nguyên nhân trong chương này | Cách sửa |
|---|---|---|
| Panic lúc chạy `attempt to add with overflow` (bản debug) | Dùng `+` thay vì `wrapping_add` trong SHA-256 | SHA-256 là số học modulo 2³² — bắt buộc `wrapping_*` |
| `E0369`: binary operation `==` cannot be applied to type `&TransactionError` | `derive(PartialEq)` trên `BlockError` mà kiểu lồng bên trong (`TransactionError`) không có | Thêm `PartialEq` cho mọi kiểu lồng bên trong |
| `E0502`: cannot borrow `self.blocks` as mutable because it is also borrowed as immutable | Giữ `prev_block = self.blocks.get(..)` rồi `self.blocks.insert(..)` trong khi vẫn còn dùng `prev_block` | Dùng xong `prev_block` (hoặc chép giá trị cần ra biến cục bộ) trước khi chèn; mượn `self.blocks` rồi ghi `self.height` thì KHÔNG lỗi — hai trường khác nhau |
| Băm không khớp vector chuẩn | Nhầm little-endian, hoặc đệm sai | Kiểm thử ở mốc 55/56/64 byte để bắt lỗi đệm |
| Panic lúc chạy `index out of bounds` khi dựng Merkle | Quên xử lý số lá lẻ (`pair[1]` với cặp chỉ có một phần tử) | `pair.get(1).unwrap_or(&pair[0])` — nhân đôi nút cuối |

---

## Tóm tắt chương & Bài tập rèn luyện

### 5 điểm cốt lõi

1. **Blockchain không có công nghệ mới.** Nó ghép bốn ý tưởng có sẵn từ thập niên 1970–90; đóng góp thật nằm ở động cơ khuyến khích.
2. **Bằng chứng công việc (Proof of Work) dựa trên bất đối xứng**: tìm tốn hàng triệu lần thử, kiểm tra tốn một phép băm.
3. **Bằng chứng Merkle chỉ cần log₂(n) giá trị băm.** Đó là lý do ví nhẹ tồn tại được.
4. **UTXO làm việc kiểm tra tiêu hai lần trở nên tầm thường** — chỉ cần tra một tập hợp.
5. **Chọn nhánh theo tổng công việc, không theo chiều dài.** Và giao dịch trên nhánh thua sẽ biến mất — đó là lý do phải chờ xác nhận.

### Bài tập rèn luyện

**Bài 1.** Cài **điều chỉnh độ khó**: sau mỗi N khối, tăng hoặc giảm độ khó để thời gian trung bình mỗi khối bám sát một mục tiêu.

<details>
<summary><b>Gợi ý</b></summary>

So thời gian thực tế đào N khối gần nhất với thời gian mục tiêu. Nhanh quá thì tăng độ khó, chậm quá thì giảm. Bitcoin giới hạn mỗi lần điều chỉnh trong khoảng ×4 và ÷4 để chống thao túng dấu thời gian.
</details>

<details>
<summary><b>Lời giải</b></summary>

```rust
impl Chain {
    /// Điều chỉnh mỗi `period` khối để bám sát `target_secs_per_block`.
    pub fn next_difficulty(&self, period: u64, target_secs_per_block: u64) -> u32 {
        let h = self.tip_height();
        let tip = &self.blocks[&self.tip];
        if h == 0 || (h + 1) % period != 0 {
            return tip.header.difficulty;
        }

        // Cửa sổ gồm `period` khối = `period - 1` khoảng thời gian.
        // Lần ngược `period - 1` bước để tới khối ĐẦU cửa sổ.
        let mut id = self.tip;
        for _ in 0..period - 1 {
            match self.blocks.get(&id) {
                Some(b) if b.header.prev_block_hash != Hash256::ZERO => {
                    id = b.header.prev_block_hash
                }
                _ => return tip.header.difficulty,
            }
        }
        let start = self.blocks[&id].header.timestamp;
        let actual = tip.header.timestamp.saturating_sub(start).max(1);
        let expected = (period - 1) * target_secs_per_block;

        // Chặn ở ×4 và ÷4 — nếu không, kẻ tấn công khai gian dấu thời gian
        // có thể kéo độ khó xuống đất chỉ trong một chu kỳ.
        let ratio = (actual as f64 / expected as f64).clamp(0.25, 4.0);
        // Nhanh gấp đôi → cần thêm 1 bit độ khó
        let adjustment = -(ratio.log2()).round() as i64;
        (tip.header.difficulty as i64 + adjustment).clamp(1, 240) as u32
    }
}

#[test]
fn fast_blocks_raise_difficulty() {
    let mut c = Chain::new(8, 50);
    // Khối thuỷ (t=0) + 3 khối cách nhau 1 giây; mục tiêu 4 giây/khối
    // → 3 khoảng mất 3 giây thay vì 12: nhanh gấp 4 → +2 bit
    for t in 1..=3u64 {
        let b = c.mine_new_block("An", vec![], t).unwrap();
        c.add_block(b).unwrap();
    }
    assert_eq!(c.next_difficulty(4, 4), 10);
    assert_eq!(c.next_difficulty(5, 4), 8, "chưa tới mốc điều chỉnh thì giữ nguyên");
}
```

Chú ý phép `clamp(0.25, 4.0)`: không có nó, một thợ đào khai dấu thời gian gian dối có thể kéo độ khó xuống rất thấp chỉ trong một chu kỳ, rồi đào lại cả chuỗi với chi phí thấp.
</details>

**Bài 2.** Cài **ví nhẹ**: cho một gốc Merkle và một bằng chứng gộp, xác minh giao dịch mà **không** cần cả khối.

<details>
<summary><b>Gợi ý</b></summary>

Ví nhẹ (Light client) chỉ tải **phần đầu khối** (80 byte mỗi khối) và kiểm hai điều: (a) phần đầu đạt độ khó, (b) bằng chứng Merkle dẫn từ giao dịch lên đúng gốc trong phần đầu đó.

Điểm yếu cần nêu rõ: ví nhẹ tin rằng chuỗi nhiều công việc nhất là chuỗi trung thực. Nó **không** tự kiểm chứng được luật giao dịch — ví dụ nó không biết một khối có tự thưởng quá mức hay không.
</details>

<details>
<summary><b>Lời giải</b></summary>

```rust
pub struct LightClient {
    /// Chỉ lưu phần đầu khối, không lưu giao dịch (Bitcoin: 80 byte mỗi khối).
    pub headers: Vec<BlockHeader>,
}

#[derive(Debug, PartialEq)]
pub enum VerifyOutcome {
    Confirmed { depth: usize },
    BlockNotFound,
    BadProof,
    HeaderBelowDifficulty,
}

impl LightClient {
    pub fn verify(&self, tx_id: Hash256, proof: &[ProofStep], block_id: Hash256) -> VerifyOutcome {
        let pos = match self.headers.iter().position(|h| h.id() == block_id) {
            Some(i) => i,
            None => return VerifyOutcome::BlockNotFound,
        };
        let header = &self.headers[pos];
        if !header.meets_difficulty() {
            return VerifyOutcome::HeaderBelowDifficulty;
        }
        if !MerkleTree::verify(tx_id, proof, header.merkle_root) {
            return VerifyOutcome::BadProof;
        }
        VerifyOutcome::Confirmed { depth: self.headers.len() - pos }
    }
}

#[test]
fn light_client_confirms_tx_without_block_body() {
    let mut c = Chain::new(8, 50);
    let b = c.mine_new_block("An", vec![], 1).unwrap();
    let (block_id, tx_id) = (b.id(), b.transactions[0].id());
    let leaves: Vec<Hash256> = b.transactions.iter().map(|t| t.id()).collect();
    let proof = MerkleTree::build(&leaves).prove(0).unwrap();
    let headers = vec![b.header.clone()];
    c.add_block(b).unwrap();
    let client = LightClient { headers };
    assert_eq!(client.verify(tx_id, &proof, block_id), VerifyOutcome::Confirmed { depth: 1 });
    assert_eq!(client.verify(sha256(b"x"), &proof, block_id), VerifyOutcome::BadProof);
}
```

Một khối chứa một triệu giao dịch nặng khoảng 250 MB. Ví nhẹ tải 80 byte phần đầu cộng 20 giá trị băm bằng chứng — tổng **720 byte**. Đó là tỉ lệ 350 000 lần, và là lý do bạn dùng được ví tiền mã hoá trên điện thoại.
</details>
