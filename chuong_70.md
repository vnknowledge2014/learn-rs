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
│   HÌNH TƯỢNG: CUỐN SỔ CÁI CỦA CẢ LÀNG, AI CŨNG GIỮ MỘT BẢN                  │
├──────────────────────────────────────────────────────────────────────────────┤
│                                                                              │
│  HÀM BĂM = DẤU LĂN TAY CỦA MỘT TRANG GIẤY                                   │
│    Đổi một dấu phẩy trong trang → dấu lăn tay đổi hoàn toàn.                │
│    Không ai làm ngược được: từ dấu lăn tay không dựng lại được trang.        │
│                                                                              │
│  CHUỖI KHỐI = MỖI TRANG GHI DẤU LĂN TAY CỦA TRANG TRƯỚC                     │
│    ┌────────┐   ┌────────┐   ┌────────┐                                     │
│    │ Khối 1 │◄──│ Khối 2 │◄──│ Khối 3 │                                     │
│    └────────┘   └────────┘   └────────┘                                     │
│    Sửa khối 1 → dấu lăn tay khối 1 đổi → khối 2 trỏ sai → khối 3 sai...     │
│    Muốn sửa một trang, phải làm lại TOÀN BỘ các trang sau nó.               │
│                                                                              │
│  BẰNG CHỨNG CÔNG VIỆC = TÌM MỘT CON SỐ MAY MẮN                              │
│    "Hãy tìm số n sao cho băm(trang + n) bắt đầu bằng 20 số 0."              │
│    Tìm: phải thử hàng triệu lần. Kiểm tra: băm MỘT lần là xong.             │
│    Đó là toàn bộ cơ chế bảo vệ — bất đối xứng giữa làm và kiểm.             │
│                                                                              │
│  CÂY MERKLE = MỤC LỤC NHIỀU TẦNG                                            │
│         gốc              Muốn chứng minh giao dịch #3 có trong khối          │
│        /    \            chứa 1 triệu giao dịch? Không cần gửi cả           │
│      ab      cd          triệu — chỉ cần 20 giá trị băm dọc đường           │
│     /  \    /  \         từ lá lên gốc. 640 byte thay vì hàng trăm MB.      │
│    a    b  c    d                                                            │
│                                                                              │
│  TÁI TỔ CHỨC = HAI PHIÊN BẢN SỔ, LÀNG CHỌN BẢN "TỐN CÔNG NHẤT"             │
│    Giao dịch của bạn nằm ở nhánh thua → nó BIẾN MẤT như chưa từng có.       │
│    Đó là lý do người bán hàng chờ 6 xác nhận trước khi giao hàng.           │
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

Bitcoin dùng SHA-256 **hai lần**. Lý do lịch sử: cấu trúc Merkle–Damgård của SHA-256 có điểm yếu "mở rộng độ dài" — biết `băm(m)` và độ dài của `m` thì tính được `băm(m ‖ đệm ‖ x)` mà không cần biết `m`. Băm hai lần bịt lỗ hổng đó.

### 2. Bằng chứng gộp của cây Merkle

Đây là ý tưởng đẹp nhất trong cả chương. Muốn chứng minh lá thứ 500 nằm trong cây 1024 lá, bạn **không cần** gửi cả 1024 lá. Chỉ cần 10 giá trị băm — mỗi tầng một giá trị "anh em" — và người kiểm chứng tự tính ngược lên gốc.

Đây chính là cơ chế cho phép ví trên điện thoại xác minh giao dịch mà không tải cả blockchain hàng trăm gigabyte.

Một cạm bẫy có thật: khi số lá lẻ, ta nhân đôi nút cuối. Cách làm này từng gây ra **CVE-2012-2459** của Bitcoin — hai danh sách giao dịch khác nhau cho ra cùng một gốc Merkle. Bài học: mọi lựa chọn "cho tiện" trong cấu trúc dữ liệu mật mã đều cần được soi kỹ.

### 3. UTXO: tiền là những tờ giấy, không phải con số

Hầu hết mọi người hình dung tài khoản có một **số dư**. Bitcoin không làm vậy. Nó theo dõi những **đầu ra chưa tiêu** — giống như ví đựng các tờ tiền mệnh giá khác nhau.

Tiêu tiền nghĩa là: phá huỷ vài tờ cũ, tạo ra vài tờ mới. Muốn trả 30 mà chỉ có tờ 50 thì phải tạo hai tờ mới: 30 cho người nhận, 19 trả lại mình, và 1 làm phí cho thợ đào.

Ưu điểm của mô hình này: **kiểm tra tiêu hai lần trở nên cực đơn giản** — chỉ cần hỏi "tờ này còn trong tập chưa tiêu không?". Và các giao dịch không dùng chung tờ nào thì kiểm chứng song song được.

### 4. Chọn nhánh theo CÔNG VIỆC, không theo chiều dài

Sách phổ thông hay nói "chuỗi dài nhất thắng". Điều đó **không chính xác**. Quy tắc thật là **tổng công việc tích luỹ lớn nhất** thắng. Khi độ khó thay đổi giữa các khối, một chuỗi ngắn hơn nhưng gồm các khối khó hơn vẫn có thể thắng.

Chương này cũng cài một quy tắc quan trọng khác: **hoà thì giữ nguyên đỉnh cũ**. Nếu đổi đỉnh khi bằng điểm, mạng sẽ lật qua lật lại vô nghĩa mỗi lần có khối mới.

---

## Mã nguồn minh họa thực chiến

Chạy bằng `cargo run -p ch70`, kiểm thử bằng `cargo test -p ch70`.

```rust
#![allow(dead_code)]
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

const HANG_SO_K: [u32; 64] = [
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
pub struct Bam(pub [u8; 32]);

impl Bam {
    pub const KHONG: Bam = Bam([0u8; 32]);
    pub fn hex(&self) -> String {
        self.0.iter().map(|b| format!("{:02x}", b)).collect()
    }
    pub fn rut_gon(&self) -> String { self.hex()[..12].to_string() }
    /// Đếm số bit 0 ở đầu — thước đo "độ khó" của bằng chứng công việc.
    pub fn so_bit_khong_dau(&self) -> u32 {
        let mut n = 0;
        for b in self.0 {
            if b == 0 { n += 8; } else { return n + b.leading_zeros(); }
        }
        n
    }
}

impl fmt::Debug for Bam {
    fn fmt(&self, f: &mut fmt::Formatter) -> fmt::Result { write!(f, "Bam({})", self.rut_gon()) }
}

pub fn sha256(data: &[u8]) -> Bam {
    // Giá trị khởi tạo = 32 bit đầu phần thập phân của căn bậc hai 8 số nguyên tố đầu
    let mut h: [u32; 8] = [
        0x6a09e667, 0xbb67ae85, 0x3c6ef372, 0xa54ff53a,
        0x510e527f, 0x9b05688c, 0x1f83d9ab, 0x5be0cd19,
    ];

    // Đệm: thêm bit 1, rồi các bit 0, rồi độ dài 64-bit — sao cho chia hết 512 bit
    let mut m = data.to_vec();
    let do_dai_bit = (data.len() as u64) * 8;
    m.push(0x80);
    while m.len() % 64 != 56 { m.push(0); }
    m.extend_from_slice(&do_dai_bit.to_be_bytes());

    for khoi in m.chunks(64) {
        // Mở rộng 16 từ thành 64 từ
        let mut w = [0u32; 64];
        for i in 0..16 {
            w[i] = u32::from_be_bytes([khoi[i*4], khoi[i*4+1], khoi[i*4+2], khoi[i*4+3]]);
        }
        for i in 16..64 {
            let s0 = w[i-15].rotate_right(7) ^ w[i-15].rotate_right(18) ^ (w[i-15] >> 3);
            let s1 = w[i-2].rotate_right(17) ^ w[i-2].rotate_right(19) ^ (w[i-2] >> 10);
            w[i] = w[i-16].wrapping_add(s0).wrapping_add(w[i-7]).wrapping_add(s1);
        }

        // 64 vòng nén
        let (mut a, mut b, mut c, mut d) = (h[0], h[1], h[2], h[3]);
        let (mut e, mut f, mut g, mut hh) = (h[4], h[5], h[6], h[7]);
        for i in 0..64 {
            let s1 = e.rotate_right(6) ^ e.rotate_right(11) ^ e.rotate_right(25);
            let ch = (e & f) ^ ((!e) & g);
            let t1 = hh.wrapping_add(s1).wrapping_add(ch)
                       .wrapping_add(HANG_SO_K[i]).wrapping_add(w[i]);
            let s0 = a.rotate_right(2) ^ a.rotate_right(13) ^ a.rotate_right(22);
            let maj = (a & b) ^ (a & c) ^ (b & c);
            let t2 = s0.wrapping_add(maj);
            hh = g; g = f; f = e;
            e = d.wrapping_add(t1);
            d = c; c = b; b = a;
            a = t1.wrapping_add(t2);
        }
        for (i, v) in [a, b, c, d, e, f, g, hh].iter().enumerate() {
            h[i] = h[i].wrapping_add(*v);
        }
    }

    let mut ra = [0u8; 32];
    for (i, v) in h.iter().enumerate() { ra[i*4..i*4+4].copy_from_slice(&v.to_be_bytes()); }
    Bam(ra)
}

/// Bitcoin dùng SHA-256 HAI LẦN. Lý do lịch sử: phòng tấn công mở rộng độ dài
/// (length-extension) vốn có của cấu trúc Merkle–Damgård.
pub fn sha256d(data: &[u8]) -> Bam { sha256(&sha256(data).0) }

// ============================================================================
// 2. CÂY MERKLE — chứng minh "giao dịch này có trong khối" mà không cần tải khối
// ============================================================================

#[derive(Debug, Clone, PartialEq)]
pub struct ProofStep { pub bam_anh_em: Bam, pub ben_phai: bool }

pub struct MerkleTree { pub all_up: Vec<Vec<Bam>> }

impl MerkleTree {
    pub fn build(la: &[Bam]) -> MerkleTree {
        if la.is_empty() { return MerkleTree { all_up: vec![vec![Bam::KHONG]] }; }
        let mut all_up = vec![la.to_vec()];
        while all_up.last().unwrap().len() > 1 {
            let below = all_up.last().unwrap();
            let mut above = Vec::with_capacity(below.len().div_ceil(2));
            for cap in below.chunks(2) {
                // Số lẻ nút thì nhân đôi nút cuối. Chính chỗ này sinh ra lỗi
                // "CVE-2012-2459" của Bitcoin: hai cây khác nhau cho cùng gốc.
                let (t, p) = (cap[0], *cap.get(1).unwrap_or(&cap[0]));
                let mut v = Vec::with_capacity(64);
                v.extend_from_slice(&t.0); v.extend_from_slice(&p.0);
                above.push(sha256d(&v));
            }
            all_up.push(above);
        }
        MerkleTree { all_up }
    }

    pub fn goc(&self) -> Bam { *self.all_up.last().unwrap().first().unwrap() }

    /// Bằng chứng gộp: chỉ log₂(n) giá trị băm là đủ chứng minh một lá thuộc cây.
    /// 1 triệu giao dịch → chỉ 20 giá trị băm = 640 byte. Đây là nền của ví nhẹ (SPV).
    pub fn prove(&self, mut chi_so: usize) -> Option<Vec<ProofStep>> {
        if chi_so >= self.all_up[0].len() { return None; }
        let mut duong = Vec::new();
        for tang in &self.all_up[..self.all_up.len() - 1] {
            let chi_so_anh_em = if chi_so % 2 == 0 { chi_so + 1 } else { chi_so - 1 };
            let anh_em = *tang.get(chi_so_anh_em).unwrap_or(&tang[chi_so]);
            duong.push(ProofStep { bam_anh_em: anh_em, ben_phai: chi_so % 2 == 0 });
            chi_so /= 2;
        }
        Some(duong)
    }

    /// Kiểm chứng KHÔNG cần cây — chỉ cần lá, bằng chứng, và gốc.
    pub fn verify(la: Bam, duong: &[ProofStep], goc: Bam) -> bool {
        let mut current = la;
        for b in duong {
            let mut v = Vec::with_capacity(64);
            if b.ben_phai {
                v.extend_from_slice(&current.0); v.extend_from_slice(&b.bam_anh_em.0);
            } else {
                v.extend_from_slice(&b.bam_anh_em.0); v.extend_from_slice(&current.0);
            }
            current = sha256d(&v);
        }
        current == goc
    }
}

// ============================================================================
// 3. MÔ HÌNH UTXO — "tiền là những tờ chưa tiêu", không phải số dư
// ============================================================================

#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
pub struct OnlyDeriveOutput { pub id_trade: Bam, pub chi_so: u32 }

#[derive(Debug, Clone, PartialEq)]
pub struct Output { pub value: u64, pub owner: String }

#[derive(Debug, Clone, PartialEq)]
pub struct Trade {
    pub input: Vec<OnlyDeriveOutput>,
    pub output: Vec<Output>,
}

impl Trade {
    /// Giao dịch tạo tiền (coinbase): không có đầu vào, sinh tiền từ hư không.
    /// Đây là giao dịch DUY NHẤT được phép làm vậy, và chỉ một lần mỗi khối.
    pub fn tao_tien(nguoi_recv: &str, value: u64, height: u64) -> Trade {
        Trade {
            input: vec![],
            // Thêm chiều cao vào tên chủ sở hữu để hai coinbase khác khối có mã khác nhau
            output: vec![Output { value, owner: format!("{nguoi_recv}#{height}") }],
        }
    }
    pub fn la_tao_tien(&self) -> bool { self.input.is_empty() }

    pub fn ma(&self) -> Bam {
        let mut v = Vec::new();
        for d in &self.input {
            v.extend_from_slice(&d.id_trade.0);
            v.extend_from_slice(&d.chi_so.to_be_bytes());
        }
        for d in &self.output {
            v.extend_from_slice(&d.value.to_be_bytes());
            v.extend_from_slice(d.owner.as_bytes());
        }
        sha256d(&v)
    }
}

/// Tập các đầu ra chưa tiêu — TOÀN BỘ trạng thái của một blockchain kiểu Bitcoin.
#[derive(Debug, Clone, Default, PartialEq)]
pub struct TapUtxo { pub o: HashMap<OnlyDeriveOutput, Output> }

#[derive(Debug, PartialEq)]
pub enum ErrorTrade {
    DauVaoKhongTonTai(OnlyDeriveOutput),
    TieuHaiLan(OnlyDeriveOutput),
    ChiVuotThu { in_: u64, ra: u64 },
    KhongCoDauRa,
}

impl TapUtxo {
    pub fn balance(&self, chu: &str) -> u64 {
        self.o.values().filter(|d| d.owner.starts_with(chu)).map(|d| d.value).sum()
    }

    /// Kiểm tra một giao dịch mà KHÔNG thay đổi trạng thái. Trả về phí thợ đào.
    pub fn check(&self, gd: &Trade, da_tieu_trong_khoi: &HashSet<OnlyDeriveOutput>)
        -> Result<u64, ErrorTrade>
    {
        if gd.output.is_empty() { return Err(ErrorTrade::KhongCoDauRa); }
        if gd.la_tao_tien() { return Ok(0); }

        let mut total_in = 0u64;
        let mut thay_in_trade = HashSet::new();
        for cd in &gd.input {
            // Tiêu hai lần TRONG CÙNG một giao dịch hoặc cùng một khối
            if da_tieu_trong_khoi.contains(cd) || !thay_in_trade.insert(*cd) {
                return Err(ErrorTrade::TieuHaiLan(*cd));
            }
            match self.o.get(cd) {
                Some(d) => total_in += d.value,
                None => return Err(ErrorTrade::DauVaoKhongTonTai(*cd)),
            }
        }
        let tong_ra: u64 = gd.output.iter().map(|d| d.value).sum();
        if tong_ra > total_in {
            return Err(ErrorTrade::ChiVuotThu { in_: total_in, ra: tong_ra });
        }
        Ok(total_in - tong_ra) // phần chênh là PHÍ, thợ đào được lấy
    }

    pub fn apply(&mut self, gd: &Trade) {
        for cd in &gd.input { self.o.remove(cd); }
        let ma = gd.ma();
        for (i, d) in gd.output.iter().enumerate() {
            self.o.insert(OnlyDeriveOutput { id_trade: ma, chi_so: i as u32 }, d.clone());
        }
    }
}

// ============================================================================
// 4. KHỐI & BẰNG CHỨNG CÔNG VIỆC
// ============================================================================

#[derive(Debug, Clone, PartialEq)]
pub struct BlockHeader {
    pub prev_hash_block: Bam,
    pub merkle_root: Bam,
    pub timestamp: u64,
    pub difficulty: u32,   // số bit 0 đầu tối thiểu
    pub so_ngau_nhien: u64,
}

impl BlockHeader {
    pub fn ma(&self) -> Bam {
        let mut v = Vec::with_capacity(88);
        v.extend_from_slice(&self.prev_hash_block.0);
        v.extend_from_slice(&self.merkle_root.0);
        v.extend_from_slice(&self.timestamp.to_be_bytes());
        v.extend_from_slice(&self.difficulty.to_be_bytes());
        v.extend_from_slice(&self.so_ngau_nhien.to_be_bytes());
        sha256d(&v)
    }
    pub fn meets_difficulty(&self) -> bool { self.ma().so_bit_khong_dau() >= self.difficulty }
}

#[derive(Debug, Clone, PartialEq)]
pub struct Block {
    pub header: BlockHeader,
    pub trade: Vec<Trade>,
}

impl Block {
    pub fn ma(&self) -> Bam { self.header.ma() }

    pub fn merkle_root_tinh_lai(&self) -> Bam {
        let la: Vec<Bam> = self.trade.iter().map(|g| g.ma()).collect();
        MerkleTree::build(&la).goc()
    }

    /// Đào = thử từng số ngẫu nhiên tới khi giá trị băm đủ nhỏ.
    /// KHÔNG có cách nào nhanh hơn thử. Đó chính là "công việc" được chứng minh.
    pub fn dao(&mut self, so_lan_thu_toi_da: u64) -> Option<u64> {
        for n in 0..so_lan_thu_toi_da {
            self.header.so_ngau_nhien = n;
            if self.header.meets_difficulty() { return Some(n); }
        }
        None
    }
}

// ============================================================================
// 5. CHUỖI KHỐI & TÁI TỔ CHỨC
// ============================================================================

#[derive(Debug, PartialEq)]
pub enum ErrorBlock {
    KhoiTruocKhongTonTai(Bam),
    ChuaDatDoKho { set: u32, can: u32 },
    GocMerkleSai,
    ThoiDiemLui,
    NhieuHonMotTaoTien,
    TaoTienVuotThuong { lay: u64, can: u64 },
    ErrorTrade(ErrorTrade),
}

pub struct Chain {
    pub all_block: HashMap<Bam, Block>,
    pub height: HashMap<Bam, u64>,
    /// Tổng công việc tích luỹ — tiêu chí chọn nhánh THẬT, không phải chiều cao.
    pub cong_work: HashMap<Bam, u128>,
    pub peak: Bam,
    pub part_normal: u64,
}

impl Chain {
    pub fn new(difficulty: u32, part_normal: u64) -> Chain {
        let mut goc = Block {
            header: BlockHeader {
                prev_hash_block: Bam::KHONG, merkle_root: Bam::KHONG,
                timestamp: 0, difficulty, so_ngau_nhien: 0,
            },
            trade: vec![Trade::tao_tien("khoi-thuy", part_normal, 0)],
        };
        goc.header.merkle_root = goc.merkle_root_tinh_lai();
        goc.dao(1 << 22);
        let ma = goc.ma();
        let mut c = Chain {
            all_block: HashMap::new(), height: HashMap::new(),
            cong_work: HashMap::new(), peak: ma, part_normal,
        };
        c.height.insert(ma, 0);
        c.cong_work.insert(ma, 1u128 << goc.header.difficulty);
        c.all_block.insert(ma, goc);
        c
    }

    pub fn height_peak(&self) -> u64 { self.height[&self.peak] }

    /// Dựng tập UTXO bằng cách phát lại chuỗi từ khối thuỷ tới `tu_khoi`.
    /// Đây là lý do node "lưu trữ đầy đủ" phải giữ toàn bộ lịch sử.
    pub fn utxo_tai(&self, tu_khoi: Bam) -> TapUtxo {
        let mut duong = Vec::new();
        let mut current = tu_khoi;
        loop {
            let k = match self.all_block.get(&current) { Some(k) => k, None => break };
            duong.push(current);
            if k.header.prev_hash_block == Bam::KHONG { break; }
            current = k.header.prev_hash_block;
        }
        duong.reverse();
        let mut u = TapUtxo::default();
        for ma in duong {
            for gd in &self.all_block[&ma].trade { u.apply(gd); }
        }
        u
    }

    pub fn them(&mut self, khoi: Block) -> Result<bool, ErrorBlock> {
        let prev = khoi.header.prev_hash_block;
        let prev_block = self.all_block.get(&prev)
            .ok_or(ErrorBlock::KhoiTruocKhongTonTai(prev))?;

        // --- Kiểm tra phần đầu ---
        let set = khoi.ma().so_bit_khong_dau();
        if set < khoi.header.difficulty {
            return Err(ErrorBlock::ChuaDatDoKho { set, can: khoi.header.difficulty });
        }
        if khoi.merkle_root_tinh_lai() != khoi.header.merkle_root {
            return Err(ErrorBlock::GocMerkleSai);
        }
        if khoi.header.timestamp < prev_block.header.timestamp {
            return Err(ErrorBlock::ThoiDiemLui);
        }

        // --- Kiểm tra giao dịch trên UTXO của nhánh cha ---
        let so_tao_tien = khoi.trade.iter().filter(|g| g.la_tao_tien()).count();
        if so_tao_tien > 1 { return Err(ErrorBlock::NhieuHonMotTaoTien); }
        let u = self.utxo_tai(prev);
        let mut da_spend: HashSet<OnlyDeriveOutput> = HashSet::new();
        let mut tong_phi = 0u64;
        for gd in &khoi.trade {
            let phi = u.check(gd, &da_spend).map_err(ErrorBlock::ErrorTrade)?;
            tong_phi += phi;
            for cd in &gd.input { da_spend.insert(*cd); }
        }
        // Thợ đào chỉ được lấy phần thưởng + phí, không hơn một xu
        if let Some(tt) = khoi.trade.iter().find(|g| g.la_tao_tien()) {
            let lay: u64 = tt.output.iter().map(|d| d.value).sum();
            let can = self.part_normal + tong_phi;
            if lay > can { return Err(ErrorBlock::TaoTienVuotThuong { lay, can }); }
        }

        // --- Ghi nhận ---
        let ma = khoi.ma();
        let cc = self.height[&prev] + 1;
        let cv = self.cong_work[&prev] + (1u128 << khoi.header.difficulty);
        self.height.insert(ma, cc);
        self.cong_work.insert(ma, cv);
        self.all_block.insert(ma, khoi);

        // Chọn nhánh theo TỔNG CÔNG VIỆC, không phải chiều cao. Một chuỗi ngắn
        // nhưng khó hơn vẫn thắng — đó là quy tắc thật của Bitcoin.
        if cv > self.cong_work[&self.peak] {
            self.peak = ma;
            return Ok(true); // đã tái tổ chức / mở rộng đỉnh
        }
        Ok(false)
    }

    /// Tạo khối kế tiếp đã đào xong, gắn lên MỘT KHỐI CHA BẤT KỲ.
    ///
    /// Nhận `cha` tường minh chứ không mặc định lấy đỉnh — nếu không, muốn dựng
    /// một nhánh rẽ ta buộc phải gán tay `self.dinh`, và thế là phá vỡ bất biến
    /// "đỉnh luôn là khối nhiều công việc nhất". Chính bất biến đó là thứ hàm
    /// `them` dựa vào để quyết định có tái tổ chức hay không.
    pub fn mine_block_above(&self, cha: Bam, nguoi_dao: &str, trade: Vec<Trade>, timestamp: u64)
        -> Option<Block>
    {
        let peak = self.all_block.get(&cha)?;
        let cc = self.height.get(&cha)? + 1;
        let u = self.utxo_tai(cha);
        let mut da_spend = HashSet::new();
        let mut phi = 0u64;
        for gd in &trade {
            phi += u.check(gd, &da_spend).ok()?;
            for cd in &gd.input { da_spend.insert(*cd); }
        }
        let mut tat_ca = vec![Trade::tao_tien(nguoi_dao, self.part_normal + phi, cc)];
        tat_ca.extend(trade);
        let mut k = Block {
            header: BlockHeader {
                prev_hash_block: cha, merkle_root: Bam::KHONG,
                timestamp: timestamp.max(peak.header.timestamp),
                difficulty: peak.header.difficulty, so_ngau_nhien: 0,
            },
            trade: tat_ca,
        };
        k.header.merkle_root = k.merkle_root_tinh_lai();
        k.dao(1 << 24)?;
        Some(k)
    }

    /// Tiện lợi: đào tiếp lên đỉnh hiện tại — trường hợp thường gặp nhất.
    pub fn new_mine_block(&self, nguoi_dao: &str, trade: Vec<Trade>, timestamp: u64) -> Option<Block> {
        self.mine_block_above(self.peak, nguoi_dao, trade, timestamp)
    }

    /// Mã của khối thuỷ (chiều cao 0).
    pub fn genesis(&self) -> Bam {
        *self.height.iter().find(|(_, &v)| v == 0).map(|(m, _)| m).unwrap()
    }
}

fn main() {
    println!("═══════════════════════════════════════════════════════════");
    println!("   BLOCKCHAIN TỪ ĐẦU: SHA-256 · MERKLE · UTXO · ĐÀO · REORG ");
    println!("═══════════════════════════════════════════════════════════");

    println!("\n1. SHA-256 TỰ CÀI — đối chiếu vector chuẩn FIPS 180-4");
    println!("   sha256(\"\")    = {}", sha256(b"").hex());
    println!("   kỳ vọng        = e3b0c44298fc1c149afbf4c8996fb92427ae41e4649b934ca495991b7852b855");
    println!("   sha256(\"abc\") = {}", sha256(b"abc").hex());
    println!("   kỳ vọng        = ba7816bf8f01cfea414140de5dae2223b00361a396177a9cb410ff61f20015ad");

    println!("\n2. HIỆU ỨNG TUYẾT LỞ — đổi 1 bit, nửa số bit đầu ra đổi theo");
    let a = sha256(b"Chuoi khoi");
    let b = sha256(b"Chuoi khoj"); // đổi đúng một ký tự
    let other: u32 = a.0.iter().zip(b.0.iter()).map(|(x, y)| (x ^ y).count_ones()).sum();
    println!("   {} \n   {}", a.rut_gon(), b.rut_gon());
    println!("   Số bit khác nhau: {}/256 ({}%)", other, other * 100 / 256);

    println!("\n3. CÂY MERKLE — bằng chứng gộp");
    let la: Vec<Bam> = (0..8u32).map(|i| sha256(&i.to_be_bytes())).collect();
    let cay = MerkleTree::build(&la);
    println!("   8 lá → gốc {} ({} tầng)", cay.goc().rut_gon(), cay.all_up.len());
    let cm = cay.prove(3).unwrap();
    println!("   Bằng chứng cho lá #3: {} giá trị băm ({} byte) thay vì cả 8 lá",
             cm.len(), cm.len() * 32);
    println!("   Kiểm chứng đúng lá : {}", MerkleTree::verify(la[3], &cm, cay.goc()));
    println!("   Kiểm chứng lá giả  : {}", MerkleTree::verify(sha256(b"gia"), &cm, cay.goc()));

    println!("\n4. ĐÀO KHỐI — chi phí tăng theo cấp số nhân");
    for difficulty in [8u32, 12, 16] {
        let mut k = Block {
            header: BlockHeader { prev_hash_block: Bam::KHONG, merkle_root: Bam::KHONG,
                                    timestamp: 0, difficulty, so_ngau_nhien: 0 },
            trade: vec![Trade::tao_tien("tho-dao", 50, 0)],
        };
        k.header.merkle_root = k.merkle_root_tinh_lai();
        let n = k.dao(1 << 24).unwrap();
        println!("   {:>2} bit 0 đầu → {:>8} lần thử · băm {}", difficulty, n, k.ma().rut_gon());
    }
    println!("   → Mỗi bit độ khó tăng thêm, công sức NHÂN ĐÔI.");

    println!("\n5. CHUỖI, UTXO & TIÊU HAI LẦN");
    let mut c = Chain::new(10, 50);
    let k1 = c.new_mine_block("An", vec![], 1).unwrap();
    c.them(k1).unwrap();
    let k2 = c.new_mine_block("An", vec![], 2).unwrap();
    c.them(k2).unwrap();
    let u = c.utxo_tai(c.peak);
    println!("   Chiều cao {} · số dư An = {} · số UTXO = {}",
             c.height_peak(), u.balance("An"), u.o.len());

    let input = *u.o.keys().find(|k| u.o[k].owner.starts_with("An")).unwrap();
    let gd = Trade { input: vec![input],
                        output: vec![Output { value: 30, owner: "Binh".into() },
                                     Output { value: 15, owner: "An-thoi-lai".into() }] };
    println!("   An trả Bình 30 (phí {} cho thợ đào)",
             u.check(&gd, &HashSet::new()).unwrap());
    let gd2 = Trade { input: vec![input],
                         output: vec![Output { value: 45, owner: "Cuong".into() }] };
    let mut da = HashSet::new(); da.insert(input);
    println!("   Tiêu lại chính đồng đó → {:?}", u.check(&gd2, &da).unwrap_err());

    println!("\n6. TÁI TỔ CHỨC CHUỖI — nhánh nhiều CÔNG VIỆC hơn thắng");
    println!("   Trước : đỉnh {} cao {} · số dư An = {}",
             c.peak.rut_gon(), c.height_peak(), c.utxo_tai(c.peak).balance("An"));
    // Kẻ tấn công đào lại từ khối thuỷ, xây nhánh riêng cho tới khi vượt
    let mut cha = c.genesis();
    for t in 1..=3u64 {
        let k = c.mine_block_above(cha, "KeTanCong", vec![], t + 10).unwrap();
        cha = k.ma();
        let swap = c.them(k).unwrap();
        println!("   Nhánh tấn công cao {} → {}", t,
                 if swap { "ĐÃ CHIẾM ĐỈNH" } else { "chưa đủ công việc" });
    }
    println!("   Sau  : đỉnh {} cao {} · số dư An = {} (khối của An BỊ ĐẢO)",
             c.peak.rut_gon(), c.height_peak(), c.utxo_tai(c.peak).balance("An"));

    println!("\n═══════════════════════════════════════════════════════════");
    println!("   BLOCKCHAIN = CẤU TRÚC DỮ LIỆU + LUẬT + ĐỘNG CƠ KHUYẾN KHÍCH");
    println!("═══════════════════════════════════════════════════════════");
}

#[cfg(test)]
mod tests {
    use super::*;

    // ---------- SHA-256 đối chiếu vector chuẩn ----------
    #[test]
    fn sha256_khop_vector_chuan_fips() {
        // Đây là bài kiểm thử quan trọng nhất chương: nếu sai một bit,
        // toàn bộ chuỗi khối phía trên đều vô nghĩa.
        assert_eq!(sha256(b"").hex(),
            "e3b0c44298fc1c149afbf4c8996fb92427ae41e4649b934ca495991b7852b855");
        assert_eq!(sha256(b"abc").hex(),
            "ba7816bf8f01cfea414140de5dae2223b00361a396177a9cb410ff61f20015ad");
        assert_eq!(sha256(b"abcdbcdecdefdefgefghfghighijhijkijkljklmklmnlmnomnopnopq").hex(),
            "248d6a61d20638b8e5c026930c3e6039a33ce45964ff2167f6ecedd419db06c1");
    }

    #[test]
    fn sha256_dung_o_moi_bien_do_dai_khoi_dem() {
        // 55 byte = vừa đủ đệm trong 1 khối; 56 byte = phải sang khối thứ hai.
        // Đây là chỗ cài đặt SHA-256 hay sai nhất.
        assert_eq!(sha256(&[b'a'; 55]).hex(),
            "9f4390f8d30c2dd92ec9f095b65e2b9ae9b0a925a5258e241c9f1e910f734318");
        assert_eq!(sha256(&[b'a'; 56]).hex(),
            "b35439a4ac6f0948b6d6f9e3c6af0f5f590ce20f1bde7090ef7970686ec6738a");
        assert_eq!(sha256(&[b'a'; 64]).hex(),
            "ffe054fe7ae0cb6dc65c3af9b61d5209f439851db43d0ba5997337df154668eb");
    }

    #[test]
    fn sha256d_la_bam_hai_lan() {
        assert_eq!(sha256d(b"abc"), sha256(&sha256(b"abc").0));
    }

    #[test]
    fn hieu_ung_tuyet_lo_gan_mot_nua_so_bit() {
        // Tiêu chuẩn vàng của hàm băm mật mã: đổi 1 bit đầu vào phải làm
        // khoảng 50% bit đầu ra đổi theo, không thể đoán được bit nào.
        let mut tong = 0u32;
        let n = 64;
        for i in 0..n {
            let a = sha256(&[i as u8, 0]);
            let b = sha256(&[i as u8, 1]); // khác đúng 1 bit
            tong += a.0.iter().zip(b.0.iter()).map(|(x, y)| (x ^ y).count_ones()).sum::<u32>();
        }
        let tb = tong as f64 / n as f64;
        assert!((tb - 128.0).abs() < 15.0, "trung bình {} bit đổi, kỳ vọng ~128", tb);
    }

    #[test]
    fn dem_bit_khong_dau_dung() {
        assert_eq!(Bam::KHONG.so_bit_khong_dau(), 256);
        let mut b = [0u8; 32]; b[0] = 0xFF;
        assert_eq!(Bam(b).so_bit_khong_dau(), 0);
        let mut b = [0u8; 32]; b[1] = 0x01;
        assert_eq!(Bam(b).so_bit_khong_dau(), 15); // 8 bit của byte 0 + 7 bit của byte 1
    }

    // ---------- Cây Merkle ----------
    #[test]
    fn merkle_moi_la_deu_chung_minh_duoc() {
        for n in [1usize, 2, 3, 4, 5, 8, 9, 16, 17] {
            let la: Vec<Bam> = (0..n as u32).map(|i| sha256(&i.to_be_bytes())).collect();
            let cay = MerkleTree::build(&la);
            for i in 0..n {
                let cm = cay.prove(i).expect("phải có bằng chứng");
                assert!(MerkleTree::verify(la[i], &cm, cay.goc()),
                        "n={} lá #{} không kiểm chứng được", n, i);
            }
        }
    }

    #[test]
    fn merkle_reject_is_price() {
        let la: Vec<Bam> = (0..8u32).map(|i| sha256(&i.to_be_bytes())).collect();
        let cay = MerkleTree::build(&la);
        let cm = cay.prove(3).unwrap();
        assert!(!MerkleTree::verify(sha256(b"gia mao"), &cm, cay.goc()));
    }

    #[test]
    fn merkle_bang_chung_dai_log_chu_khong_tuyen_tinh() {
        let la: Vec<Bam> = (0..1024u32).map(|i| sha256(&i.to_be_bytes())).collect();
        let cay = MerkleTree::build(&la);
        let cm = cay.prove(500).unwrap();
        assert_eq!(cm.len(), 10, "1024 lá → log2(1024) = 10 bước, không phải 1024");
        assert!(MerkleTree::verify(la[500], &cm, cay.goc()));
    }

    #[test]
    fn merkle_doi_mot_la_lam_doi_goc() {
        let mut la: Vec<Bam> = (0..8u32).map(|i| sha256(&i.to_be_bytes())).collect();
        let old_goc = MerkleTree::build(&la).goc();
        la[5] = sha256(b"da bi sua");
        assert_ne!(MerkleTree::build(&la).goc(), old_goc, "sửa bất kỳ lá nào phải lộ ra ở gốc");
    }

    #[test]
    fn merkle_chi_so_ngoai_pham_vi_tra_none() {
        let la: Vec<Bam> = (0..4u32).map(|i| sha256(&i.to_be_bytes())).collect();
        assert!(MerkleTree::build(&la).prove(4).is_none());
    }

    // ---------- UTXO ----------
    fn series_has_tien(chu: &str) -> (Chain, OnlyDeriveOutput) {
        let mut c = Chain::new(8, 50);
        let k = c.new_mine_block(chu, vec![], 1).unwrap();
        c.them(k).unwrap();
        let u = c.utxo_tai(c.peak);
        let cd = *u.o.keys().find(|k| u.o[k].owner.starts_with(chu)).unwrap();
        (c, cd)
    }

    #[test]
    fn trade_hop_le_return_ve_phi() {
        let (c, cd) = series_has_tien("An");
        let u = c.utxo_tai(c.peak);
        let gd = Trade { input: vec![cd],
            output: vec![Output { value: 45, owner: "Binh".into() }] };
        assert_eq!(u.check(&gd, &HashSet::new()), Ok(5), "50 vào - 45 ra = 5 phí");
    }

    #[test]
    fn khong_the_chi_nhieu_hon_thu() {
        let (c, cd) = series_has_tien("An");
        let u = c.utxo_tai(c.peak);
        let gd = Trade { input: vec![cd],
            output: vec![Output { value: 999, owner: "An".into() }] };
        assert_eq!(u.check(&gd, &HashSet::new()),
                   Err(ErrorTrade::ChiVuotThu { in_: 50, ra: 999 }));
    }

    #[test]
    fn no_position_spend_two_lan_in_same_trade() {
        let (c, cd) = series_has_tien("An");
        let u = c.utxo_tai(c.peak);
        // Dùng CÙNG một đầu vào hai lần để "nhân đôi" tiền
        let gd = Trade { input: vec![cd, cd],
            output: vec![Output { value: 100, owner: "An".into() }] };
        assert_eq!(u.check(&gd, &HashSet::new()), Err(ErrorTrade::TieuHaiLan(cd)));
    }

    #[test]
    fn no_position_spend_input_no_ton_tai() {
        let (c, _) = series_has_tien("An");
        let u = c.utxo_tai(c.peak);
        let ma = OnlyDeriveOutput { id_trade: sha256(b"bia dat"), chi_so: 0 };
        let gd = Trade { input: vec![ma],
            output: vec![Output { value: 1, owner: "An".into() }] };
        assert_eq!(u.check(&gd, &HashSet::new()), Err(ErrorTrade::DauVaoKhongTonTai(ma)));
    }

    #[test]
    fn apply_trade_report_toan_total_value_tru_phi() {
        let (c, cd) = series_has_tien("An");
        let mut u = c.utxo_tai(c.peak);
        let prev: u64 = u.o.values().map(|d| d.value).sum();
        let gd = Trade { input: vec![cd],
            output: vec![Output { value: 30, owner: "Binh".into() },
                         Output { value: 18, owner: "An2".into() }] };
        let phi = u.check(&gd, &HashSet::new()).unwrap();
        u.apply(&gd);
        let next: u64 = u.o.values().map(|d| d.value).sum();
        assert_eq!(prev - next, phi, "chênh lệch đúng bằng phí, không mất mát ở đâu khác");
    }

    // ---------- Khối & đào ----------
    #[test]
    fn dao_tim_duoc_so_thoa_do_kho() {
        let mut k = Block {
            header: BlockHeader { prev_hash_block: Bam::KHONG, merkle_root: Bam::KHONG,
                                    timestamp: 0, difficulty: 12, so_ngau_nhien: 0 },
            trade: vec![Trade::tao_tien("x", 50, 0)],
        };
        k.header.merkle_root = k.merkle_root_tinh_lai();
        assert!(k.dao(1 << 22).is_some());
        assert!(k.header.meets_difficulty());
        assert!(k.ma().so_bit_khong_dau() >= 12);
    }

    #[test]
    fn kiem_chung_bang_chung_re_hon_tao_ra_no_rat_nhieu() {
        // Bất đối xứng này là toàn bộ ý nghĩa của "bằng chứng công việc":
        // tìm thì tốn hàng nghìn lần thử, kiểm tra chỉ tốn MỘT lần băm.
        let mut k = Block {
            header: BlockHeader { prev_hash_block: Bam::KHONG, merkle_root: Bam::KHONG,
                                    timestamp: 0, difficulty: 14, so_ngau_nhien: 0 },
            trade: vec![Trade::tao_tien("x", 50, 0)],
        };
        k.header.merkle_root = k.merkle_root_tinh_lai();
        let so_lan = k.dao(1 << 24).unwrap();
        assert!(so_lan > 100, "độ khó 14 bit phải tốn nhiều lần thử, thực tế {}", so_lan);
        assert!(k.header.meets_difficulty(), "nhưng kiểm tra chỉ cần 1 phép băm");
    }

    // ---------- Chuỗi ----------
    #[test]
    fn series_new_has_genesis_hop_le() {
        let c = Chain::new(8, 50);
        assert_eq!(c.height_peak(), 0);
        assert_eq!(c.utxo_tai(c.peak).balance("khoi-thuy"), 50);
    }

    #[test]
    fn them_khoi_lam_tang_chieu_cao_va_so_du() {
        let mut c = Chain::new(8, 50);
        for i in 1..=3u64 {
            let k = c.new_mine_block("An", vec![], i).unwrap();
            assert_eq!(c.them(k), Ok(true));
            assert_eq!(c.height_peak(), i);
        }
        assert_eq!(c.utxo_tai(c.peak).balance("An"), 150, "3 khối × 50");
    }

    #[test]
    fn reject_block_chua_meets_difficulty() {
        let mut c = Chain::new(12, 50);
        let mut k = c.new_mine_block("An", vec![], 1).unwrap();
        k.header.so_ngau_nhien = k.header.so_ngau_nhien.wrapping_add(1); // phá bằng chứng
        assert!(matches!(c.them(k), Err(ErrorBlock::ChuaDatDoKho { .. })));
    }

    #[test]
    fn reject_block_has_merkle_root_sai() {
        let mut c = Chain::new(8, 50);
        let mut k = c.new_mine_block("An", vec![], 1).unwrap();
        // Nhét thêm giao dịch mà không cập nhật gốc Merkle — đúng kiểu tấn công
        // "đổi nội dung nhưng giữ nguyên bằng chứng công việc"
        k.trade.push(Trade::tao_tien("KeGian", 1000, 99));
        assert!(matches!(c.them(k), Err(ErrorBlock::GocMerkleSai) | Err(ErrorBlock::ChuaDatDoKho{..})));
    }

    #[test]
    fn tu_choi_tho_dao_tu_thuong_qua_muc() {
        let mut c = Chain::new(8, 50);
        let mut k = c.new_mine_block("An", vec![], 1).unwrap();
        k.trade[0] = Trade::tao_tien("An", 1_000_000, 1); // tham lam
        k.header.merkle_root = k.merkle_root_tinh_lai();
        k.dao(1 << 20);
        assert!(matches!(c.them(k), Err(ErrorBlock::TaoTienVuotThuong { .. })));
    }

    #[test]
    fn tu_choi_khoi_khong_co_cha() {
        let mut c = Chain::new(8, 50);
        let id_price = sha256(b"khong ton tai");
        let mut k = Block {
            header: BlockHeader { prev_hash_block: id_price, merkle_root: Bam::KHONG,
                                    timestamp: 5, difficulty: 8, so_ngau_nhien: 0 },
            trade: vec![Trade::tao_tien("An", 50, 1)],
        };
        k.header.merkle_root = k.merkle_root_tinh_lai();
        k.dao(1 << 20);
        assert_eq!(c.them(k), Err(ErrorBlock::KhoiTruocKhongTonTai(id_price)));
    }

    #[test]
    fn tai_to_chuc_khi_nhanh_re_vuot_len() {
        let mut c = Chain::new(8, 50);
        let thuy = c.genesis();

        // Nhánh chính: An đào 2 khối
        for i in 1..=2u64 {
            let k = c.new_mine_block("An", vec![], i).unwrap();
            assert_eq!(c.them(k), Ok(true));
        }
        let peak_hidden = c.peak;
        assert_eq!(c.height_peak(), 2);
        assert_eq!(c.utxo_tai(c.peak).balance("An"), 100);

        // Nhánh rẽ dựng từ khối thuỷ — KHÔNG đụng tới self.dinh
        let d1 = c.mine_block_above(thuy, "Doi", vec![], 10).unwrap();
        let ma_d1 = d1.ma();
        assert_eq!(c.them(d1), Ok(false), "cao 1 < cao 2 → chưa chiếm được đỉnh");
        assert_eq!(c.peak, peak_hidden, "đỉnh vẫn phải là nhánh nhiều công việc hơn");

        let d2 = c.mine_block_above(ma_d1, "Doi", vec![], 11).unwrap();
        let ma_d2 = d2.ma();
        assert_eq!(c.them(d2), Ok(false), "hoà 2-2 thì người tới sau KHÔNG được lật");
        assert_eq!(c.peak, peak_hidden);

        // Khối thứ ba mới đủ vượt
        let d3 = c.mine_block_above(ma_d2, "Doi", vec![], 12).unwrap();
        assert_eq!(c.them(d3), Ok(true), "cao 3 > cao 2 → TÁI TỔ CHỨC");
        assert_eq!(c.height_peak(), 3);
        assert_eq!(c.utxo_tai(c.peak).balance("Doi"), 150);
        assert_eq!(c.utxo_tai(c.peak).balance("An"), 0,
                   "toàn bộ khối của An bị ĐẢO — đó chính là ý nghĩa của 'chờ đủ xác nhận'");

        // Nhánh cũ vẫn nằm trong kho, chỉ là không còn nằm trên đường tới đỉnh
        assert!(c.all_block.contains_key(&peak_hidden));
        assert_eq!(c.utxo_tai(peak_hidden).balance("An"), 100, "phát lại nhánh cũ vẫn ra kết quả cũ");
    }

    #[test]
    fn hoa_cong_viec_thi_giu_nguyen_dinh_dau_tien() {
        // Quy tắc chống dao động: chỉ đổi đỉnh khi THỰC SỰ nhiều việc hơn,
        // không đổi khi bằng. Nếu không, mạng sẽ lật qua lật lại vô nghĩa.
        let mut c = Chain::new(8, 50);
        let thuy = c.genesis();
        let a = c.new_mine_block("A", vec![], 1).unwrap();
        let ma_a = a.ma();
        c.them(a).unwrap();
        let b = c.mine_block_above(thuy, "B", vec![], 2).unwrap();
        assert_eq!(c.them(b), Ok(false));
        assert_eq!(c.peak, ma_a, "cùng công việc → giữ nguyên đỉnh cũ");
    }

    #[test]
    fn moi_khoi_deu_bam_ra_ma_khac_nhau() {
        let mut c = Chain::new(8, 50);
        let mut seen = HashSet::new();
        seen.insert(c.peak);
        for i in 1..=5u64 {
            let k = c.new_mine_block("An", vec![], i).unwrap();
            let ma = k.ma();
            assert!(seen.insert(ma), "trùng mã khối — không được xảy ra");
            c.them(k).unwrap();
        }
    }
}
```

---

## Bảng tra cứu lỗi biên dịch thường gặp

| Lỗi | Nguyên nhân trong chương này | Cách sửa |
|---|---|---|
| `attempt to add with overflow` | Dùng `+` thay vì `wrapping_add` trong SHA-256 | SHA-256 là số học modulo 2³² — bắt buộc `wrapping_*` |
| `E0369: binary operation == cannot be applied` | `derive(PartialEq)` trên kiểu ngoài mà kiểu trong không có | Thêm `PartialEq` cho mọi kiểu lồng bên trong |
| `E0502: cannot borrow as mutable` | `self.all_block.get()` rồi `self.height.insert()` | Lấy giá trị cần ra biến cục bộ trước khi mượn tiếp |
| Băm không khớp vector chuẩn | Nhầm little-endian, hoặc đệm sai | Kiểm thử ở mốc 55/56/64 byte để bắt lỗi đệm |
| `index out of bounds` khi dựng Merkle | Quên xử lý số lá lẻ | `cap.get(1).unwrap_or(&cap[0])` — nhân đôi nút cuối |

---

## Tóm tắt chương & Bài tập rèn luyện

### 5 điểm cốt lõi

1. **Blockchain không có công nghệ mới.** Nó ghép bốn ý tưởng có sẵn từ thập niên 1970–90; đóng góp thật nằm ở động cơ khuyến khích.
2. **Bằng chứng công việc dựa trên bất đối xứng**: tìm tốn hàng triệu lần thử, kiểm tra tốn một phép băm.
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
    /// Điều chỉnh mỗi `period` khối để bám sát `giay_moi_khoi_mong_muon`.
    pub fn do_kho_moi(&self, period: u64, giay_moi_khoi_mong_muon: u64) -> u32 {
        let cc = self.height_peak();
        let peak = &self.all_block[&self.peak];
        if cc == 0 || (cc + 1) % period != 0 { return peak.header.difficulty; }

        // Lần ngược `period` khối để lấy dấu thời gian đầu giai đoạn
        let mut ma = self.peak;
        for _ in 0..period {
            match self.all_block.get(&ma) {
                Some(k) if k.header.prev_hash_block != Bam::KHONG =>
                    ma = k.header.prev_hash_block,
                _ => return peak.header.difficulty,
            }
        }
        let start = self.all_block[&ma].header.timestamp;
        let thuc_te = peak.header.timestamp.saturating_sub(start).max(1);
        let mong_muon = period * giay_moi_khoi_mong_muon;

        // Chặn ở ×4 và ÷4 — nếu không, kẻ tấn công khai gian dấu thời gian
        // có thể kéo độ khó xuống đất chỉ trong một chu kỳ.
        let ratio = (thuc_te as f64 / mong_muon as f64).clamp(0.25, 4.0);
        // Nhanh gấp đôi → cần thêm 1 bit độ khó
        let dieu_chinh = -(ratio.log2()).round() as i64;
        (peak.header.difficulty as i64 + dieu_chinh).clamp(1, 240) as u32
    }
}
```

Chú ý phép `clamp(0.25, 4.0)`: không có nó, một thợ đào khai dấu thời gian gian dối có thể kéo độ khó xuống rất thấp chỉ trong một chu kỳ, rồi đào lại cả chuỗi với chi phí thấp.
</details>

**Bài 2.** Cài **ví nhẹ**: cho một gốc Merkle và một bằng chứng gộp, xác minh giao dịch mà **không** cần cả khối.

<details>
<summary><b>Gợi ý</b></summary>

Ví nhẹ chỉ tải **phần đầu khối** (80 byte mỗi khối) và kiểm hai điều: (a) phần đầu đạt độ khó, (b) bằng chứng Merkle dẫn từ giao dịch lên đúng gốc trong phần đầu đó.

Điểm yếu cần nêu rõ: ví nhẹ tin rằng chuỗi nhiều công việc nhất là chuỗi trung thực. Nó **không** tự kiểm chứng được luật giao dịch — ví dụ nó không biết một khối có tự thưởng quá mức hay không.
</details>

<details>
<summary><b>Lời giải</b></summary>

```rust
pub struct ViNhe {
    /// Chỉ lưu phần đầu khối, không lưu giao dịch. 80 byte mỗi khối.
    pub cac_phan_dau: Vec<BlockHeader>,
}

#[derive(Debug, PartialEq)]
pub enum KetQuaXacMinh {
    DaXacNhan { do_sau: usize },
    KhongTimThayKhoi,
    BangChungSai,
    PhanDauKhongDatDoKho,
}

impl ViNhe {
    pub fn xac_minh(&self, id_trade: Bam, bang_chung: &[ProofStep],
                    ma_khoi: Bam) -> KetQuaXacMinh
    {
        let pos_value = match self.cac_phan_dau.iter().position(|h| h.ma() == ma_khoi) {
            Some(i) => i,
            None => return KetQuaXacMinh::KhongTimThayKhoi,
        };
        let pd = &self.cac_phan_dau[pos_value];
        if !pd.meets_difficulty() { return KetQuaXacMinh::PhanDauKhongDatDoKho; }
        if !MerkleTree::verify(id_trade, bang_chung, pd.merkle_root) {
            return KetQuaXacMinh::BangChungSai;
        }
        KetQuaXacMinh::DaXacNhan { do_sau: self.cac_phan_dau.len() - pos_value }
    }
}
```

Một khối chứa một triệu giao dịch nặng khoảng 250 MB. Ví nhẹ tải 80 byte phần đầu cộng 20 giá trị băm bằng chứng — tổng **720 byte**. Đó là tỉ lệ 350 000 lần, và là lý do bạn dùng được ví tiền mã hoá trên điện thoại.
</details>
