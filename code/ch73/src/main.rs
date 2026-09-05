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
    for vong in 0..24 {
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
        a[0] ^= RC[vong];
    }
}

/// Cấu trúc "bọt biển": hút dữ liệu vào theo từng khối `RATE` byte, rồi vắt ra.
pub fn keccak256(du_lieu: &[u8]) -> [u8; 32] {
    const RATE: usize = 136; // 1600 bit − 2×256 bit dung lượng = 1088 bit
    let mut a = [0u64; 25];
    let mut dem = Vec::with_capacity(du_lieu.len() + RATE);
    dem.extend_from_slice(du_lieu);
    // Đệm pad10*1 với byte miền 0x01 — CHỖ NÀY khác SHA3-256 (dùng 0x06)
    dem.push(0x01);
    while dem.len() % RATE != 0 { dem.push(0x00); }
    let n = dem.len();
    dem[n - 1] |= 0x80;

    for khoi in dem.chunks(RATE) {
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

/// `chu_ky_ham("transfer(address,uint256)")` = 0xa9059cbb — con số mà bất kỳ ai
/// từng đọc log Ethereum đều đã thấy hàng nghìn lần.
///
/// Chỉ 4 byte nghĩa là VA CHẠM CÓ THẬT: xác suất hai hàm khác nhau trùng
/// chữ ký chỉ khoảng 1/2³². Đã có người cố tình tìm hàm trùng để đánh lừa
/// giao diện ví — đó là lý do ví hiện đại hiển thị cả chữ ký đầy đủ.
pub fn chu_ky_ham(chu_ky: &str) -> [u8; 4] {
    let b = keccak256(chu_ky.as_bytes());
    [b[0], b[1], b[2], b[3]]
}

/// Chủ đề sự kiện (topic0) dùng cả 32 byte, nên an toàn hơn hẳn.
pub fn chu_de_su_kien(chu_ky: &str) -> [u8; 32] { keccak256(chu_ky.as_bytes()) }

// ============================================================================
// 3. MÃ HOÁ ABI — quy tắc xếp tham số thành các ô 32 byte
// ============================================================================

pub type DiaChi = [u8; 20];

#[derive(Debug, Clone, PartialEq)]
pub enum GiaTriAbi {
    Uint(u128),
    Int(i128),
    Bool(bool),
    Address(DiaChi),
    Bytes32([u8; 32]),
    // --- kiểu ĐỘNG: chỉ ghi con trỏ vào phần đầu, dữ liệu nằm ở đuôi ---
    Bytes(Vec<u8>),
    Chuoi(String),
    MangUint(Vec<u128>),
}

impl GiaTriAbi {
    pub fn la_dong(&self) -> bool {
        matches!(self, GiaTriAbi::Bytes(_) | GiaTriAbi::Chuoi(_) | GiaTriAbi::MangUint(_))
    }

    fn o_32(v: u128) -> [u8; 32] {
        let mut o = [0u8; 32];
        o[16..].copy_from_slice(&v.to_be_bytes()); // căn PHẢI
        o
    }

    /// Phần cố định: kiểu tĩnh ghi thẳng giá trị, kiểu động ghi con trỏ (điền sau).
    fn phan_dau(&self) -> [u8; 32] {
        match self {
            GiaTriAbi::Uint(v) => Self::o_32(*v),
            GiaTriAbi::Bool(b) => Self::o_32(*b as u128),
            GiaTriAbi::Int(v) => {
                // Số âm dùng bù hai và MỞ RỘNG DẤU bằng 0xFF, không phải 0x00
                let mut o = if *v < 0 { [0xFFu8; 32] } else { [0u8; 32] };
                o[16..].copy_from_slice(&(*v as u128).to_be_bytes());
                o
            }
            GiaTriAbi::Address(a) => {
                let mut o = [0u8; 32];
                o[12..].copy_from_slice(a); // 20 byte căn phải trong ô 32 byte
                o
            }
            GiaTriAbi::Bytes32(b) => *b,
            _ => [0u8; 32], // kiểu động: chỗ này sẽ bị ghi đè bằng con trỏ
        }
    }

    /// Phần đuôi cho kiểu động: độ dài rồi tới dữ liệu, đệm cho tròn 32 byte.
    fn phan_duoi(&self) -> Vec<u8> {
        match self {
            GiaTriAbi::Bytes(b) => {
                let mut v = Self::o_32(b.len() as u128).to_vec();
                v.extend_from_slice(b);
                while v.len() % 32 != 0 { v.push(0); }
                v
            }
            GiaTriAbi::Chuoi(s) => GiaTriAbi::Bytes(s.as_bytes().to_vec()).phan_duoi(),
            GiaTriAbi::MangUint(m) => {
                let mut v = Self::o_32(m.len() as u128).to_vec();
                for x in m { v.extend_from_slice(&Self::o_32(*x)); }
                v
            }
            _ => Vec::new(),
        }
    }
}

/// Mã hoá danh sách tham số theo đúng đặc tả ABI của Solidity.
pub fn ma_hoa_abi(cac_gt: &[GiaTriAbi]) -> Vec<u8> {
    let kich_thuoc_dau = cac_gt.len() * 32;
    let mut dau: Vec<u8> = Vec::with_capacity(kich_thuoc_dau);
    let mut duoi: Vec<u8> = Vec::new();

    for gt in cac_gt {
        if gt.la_dong() {
            // Con trỏ tính từ ĐẦU vùng tham số, không phải từ đầu calldata.
            // Nhầm gốc toạ độ ở đây là lỗi ABI phổ biến nhất.
            let con_tro = kich_thuoc_dau + duoi.len();
            dau.extend_from_slice(&GiaTriAbi::o_32(con_tro as u128));
            duoi.extend_from_slice(&gt.phan_duoi());
        } else {
            dau.extend_from_slice(&gt.phan_dau());
        }
    }
    dau.extend_from_slice(&duoi);
    dau
}

/// Dựng calldata hoàn chỉnh: 4 byte chữ ký hàm + tham số đã mã hoá.
pub fn dung_calldata(chu_ky: &str, cac_gt: &[GiaTriAbi]) -> Vec<u8> {
    let mut v = chu_ky_ham(chu_ky).to_vec();
    v.extend_from_slice(&ma_hoa_abi(cac_gt));
    v
}

/// Giải mã ngược một tham số `uint256` ở vị trí `chi_so` (dùng để đọc kết quả).
pub fn doc_uint(du_lieu: &[u8], chi_so: usize) -> Option<u128> {
    let d = du_lieu.get(chi_so * 32..chi_so * 32 + 32)?;
    // 16 byte cao phải bằng 0, nếu không thì giá trị vượt u128
    if d[..16].iter().any(|&b| b != 0) { return None; }
    Some(u128::from_be_bytes(d[16..].try_into().ok()?))
}

pub fn doc_dia_chi(du_lieu: &[u8], chi_so: usize) -> Option<DiaChi> {
    let d = du_lieu.get(chi_so * 32..chi_so * 32 + 32)?;
    if d[..12].iter().any(|&b| b != 0) { return None; } // 12 byte đệm phải là 0
    d[12..].try_into().ok()
}

// ============================================================================
// 4. MÃ HOÁ RLP — định dạng tuần tự hoá của Ethereum
// ============================================================================

#[derive(Debug, Clone, PartialEq)]
pub enum Rlp { Chuoi(Vec<u8>), DanhSach(Vec<Rlp>) }

impl Rlp {
    pub fn ma_hoa(&self) -> Vec<u8> {
        match self {
            Rlp::Chuoi(b) => {
                if b.len() == 1 && b[0] < 0x80 {
                    b.clone() // byte đơn nhỏ tự mã hoá chính nó
                } else {
                    let mut v = Self::tien_to(b.len(), 0x80);
                    v.extend_from_slice(b);
                    v
                }
            }
            Rlp::DanhSach(ds) => {
                let mut than = Vec::new();
                for x in ds { than.extend_from_slice(&x.ma_hoa()); }
                let mut v = Self::tien_to(than.len(), 0xC0);
                v.extend_from_slice(&than);
                v
            }
        }
    }

    fn tien_to(do_dai: usize, goc: u8) -> Vec<u8> {
        if do_dai < 56 {
            vec![goc + do_dai as u8]
        } else {
            // Độ dài dài: ghi độ-dài-của-độ-dài rồi tới độ dài
            let b = do_dai.to_be_bytes();
            let bo_qua = b.iter().position(|&x| x != 0).unwrap();
            let mut v = vec![goc + 55 + (b.len() - bo_qua) as u8];
            v.extend_from_slice(&b[bo_qua..]);
            v
        }
    }

    /// Số nguyên trong RLP dùng big-endian KHÔNG có số 0 thừa ở đầu.
    /// Số 0 mã hoá thành chuỗi RỖNG, không phải byte 0x00 — điểm hay bị sai.
    pub fn tu_so(v: u128) -> Rlp {
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
pub struct GiaoDich1559 {
    pub ma_chuoi: u64,
    pub so_thu_tu: u64,           // nonce
    pub phi_uu_tien_toi_da: u128, // tiền "boa" cho người xây khối
    pub phi_toi_da: u128,         // trần tổng phí mỗi đơn vị gas
    pub gioi_han_gas: u64,
    pub den: Option<DiaChi>,      // None = tạo hợp đồng mới
    pub gia_tri: u128,
    pub du_lieu: Vec<u8>,
}

impl GiaoDich1559 {
    /// Tải trọng để ký: 0x02 || rlp([...]). Byte 0x02 là "loại giao dịch",
    /// thêm vào từ EIP-2718 để chuỗi phân biệt được các định dạng khác nhau.
    pub fn tai_trong_ky(&self) -> Vec<u8> {
        let ds = Rlp::DanhSach(vec![
            Rlp::tu_so(self.ma_chuoi as u128),
            Rlp::tu_so(self.so_thu_tu as u128),
            Rlp::tu_so(self.phi_uu_tien_toi_da),
            Rlp::tu_so(self.phi_toi_da),
            Rlp::tu_so(self.gioi_han_gas as u128),
            match self.den { Some(a) => Rlp::Chuoi(a.to_vec()), None => Rlp::Chuoi(vec![]) },
            Rlp::tu_so(self.gia_tri),
            Rlp::Chuoi(self.du_lieu.clone()),
            Rlp::DanhSach(vec![]), // danh sách truy cập (EIP-2930), để trống
        ]);
        let mut v = vec![0x02];
        v.extend_from_slice(&ds.ma_hoa());
        v
    }

    pub fn ma_bam_ky(&self) -> [u8; 32] { keccak256(&self.tai_trong_ky()) }

    /// Chi phí TỐI ĐA có thể bị trừ khỏi ví. Ví phải kiểm tra con số này
    /// chứ không phải phí thực tế — vì phí thực tế chỉ biết sau khi khai thác.
    pub fn chi_phi_toi_da(&self) -> u128 {
        self.gia_tri + self.phi_toi_da * self.gioi_han_gas as u128
    }

    /// Phí thực trả theo EIP-1559: phần đốt (base fee) + tiền boa, nhưng
    /// không bao giờ vượt trần người dùng đặt.
    pub fn phi_thuc_te(&self, phi_co_ban: u128) -> u128 {
        let boa = self.phi_uu_tien_toi_da.min(self.phi_toi_da.saturating_sub(phi_co_ban));
        phi_co_ban + boa
    }
}

// ============================================================================
// 6. RÀNG BUỘC KIỂU — "macro sol!" thu nhỏ
// ============================================================================
// alloy sinh ra kiểu Rust từ ABI để bạn không tự tay ghép byte. Đây là bản
// làm tay của cùng ý tưởng: mỗi hàm hợp đồng là một phương thức có kiểu rõ ràng.

pub struct Erc20 { pub dia_chi: DiaChi }

impl Erc20 {
    pub const CK_CHUYEN: &'static str = "transfer(address,uint256)";
    pub const CK_SO_DU: &'static str = "balanceOf(address)";
    pub const CK_CHO_PHEP: &'static str = "approve(address,uint256)";
    pub const SK_CHUYEN: &'static str = "Transfer(address,address,uint256)";

    pub fn chuyen(&self, den: DiaChi, so_luong: u128) -> Vec<u8> {
        dung_calldata(Self::CK_CHUYEN, &[GiaTriAbi::Address(den), GiaTriAbi::Uint(so_luong)])
    }
    pub fn so_du_cua(&self, ai: DiaChi) -> Vec<u8> {
        dung_calldata(Self::CK_SO_DU, &[GiaTriAbi::Address(ai)])
    }
    pub fn cho_phep(&self, ai: DiaChi, so_luong: u128) -> Vec<u8> {
        dung_calldata(Self::CK_CHO_PHEP, &[GiaTriAbi::Address(ai), GiaTriAbi::Uint(so_luong)])
    }
    /// Giải mã giá trị `uint256` trả về từ `eth_call`.
    pub fn doc_so_du(ket_qua: &[u8]) -> Option<u128> { doc_uint(ket_qua, 0) }
}

pub fn dia_chi_tu_hex(s: &str) -> DiaChi {
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
        println!("   0x{} ← {}", hex(&chu_ky_ham(ck)), ck);
    }
    println!("   topic0 sự kiện Transfer = 0x{}", hex(&chu_de_su_kien(Erc20::SK_CHUYEN)));

    println!("\n3. MÃ HOÁ ABI");
    let token = Erc20 { dia_chi: dia_chi_tu_hex("0xA0b86991c6218b36c1d19D4a2e9Eb0cE3606eB48") };
    let den = dia_chi_tu_hex("0x742d35Cc6634C0532925a3b844Bc454e4438f44e");
    let cd = token.chuyen(den, 1_000_000);
    println!("   transfer(0x742d…f44e, 1000000) → {} byte", cd.len());
    println!("   chữ ký hàm: 0x{}", hex(&cd[..4]));
    println!("   tham số 1 : {}", hex(&cd[4..36]));
    println!("   tham số 2 : {}", hex(&cd[36..68]));

    println!("\n4. KIỂU ĐỘNG — con trỏ ở đầu, dữ liệu ở đuôi");
    let ma = ma_hoa_abi(&[
        GiaTriAbi::Uint(42),
        GiaTriAbi::Chuoi("xin chao".into()),
        GiaTriAbi::Bool(true),
    ]);
    println!("   (uint 42, string \"xin chao\", bool true) → {} byte", ma.len());
    println!("   ô 0 (uint)     : {}", hex(&ma[0..32]));
    println!("   ô 1 (con trỏ)  : {} ← trỏ tới byte {}", hex(&ma[32..64]), doc_uint(&ma, 1).unwrap());
    println!("   ô 2 (bool)     : {}", hex(&ma[64..96]));
    println!("   ô 3 (độ dài)   : {}", hex(&ma[96..128]));
    println!("   ô 4 (dữ liệu)  : {}", hex(&ma[128..160]));

    println!("\n5. RLP");
    println!("   RLP(\"dog\")         = {}", hex(&Rlp::Chuoi(b"dog".to_vec()).ma_hoa()));
    println!("   RLP(0)              = {} (chuỗi RỖNG, không phải 0x00)", hex(&Rlp::tu_so(0).ma_hoa()));
    println!("   RLP(15)             = {}", hex(&Rlp::tu_so(15).ma_hoa()));
    println!("   RLP(1024)           = {}", hex(&Rlp::tu_so(1024).ma_hoa()));
    println!("   RLP([\"cat\",\"dog\"]) = {}",
             hex(&Rlp::DanhSach(vec![Rlp::Chuoi(b"cat".to_vec()),
                                     Rlp::Chuoi(b"dog".to_vec())]).ma_hoa()));

    println!("\n6. GIAO DỊCH EIP-1559");
    let gd = GiaoDich1559 {
        ma_chuoi: 1, so_thu_tu: 42,
        phi_uu_tien_toi_da: 2_000_000_000,     // 2 gwei tiền boa
        phi_toi_da: 100_000_000_000,           // trần 100 gwei
        gioi_han_gas: 65_000,
        den: Some(token.dia_chi), gia_tri: 0, du_lieu: cd.clone(),
    };
    println!("   Tải trọng ký: {} byte, bắt đầu bằng 0x{:02x} (loại giao dịch)",
             gd.tai_trong_ky().len(), gd.tai_trong_ky()[0]);
    println!("   Băm để ký   : 0x{}", hex(&gd.ma_bam_ky()));
    println!("   Chi phí tối đa bị khoá: {} wei", gd.chi_phi_toi_da());
    for phi_co_ban in [10_000_000_000u128, 50_000_000_000, 99_000_000_000] {
        println!("   base fee {:>3} gwei → thực trả {:>3} gwei/gas",
                 phi_co_ban / 1_000_000_000, gd.phi_thuc_te(phi_co_ban) / 1_000_000_000);
    }

    println!("\n═══════════════════════════════════════════════════════════");
    println!("   MỌI GIAO DỊCH ETHEREUM CHỈ LÀ BYTE ĐƯỢC XẾP ĐÚNG CHỖ     ");
    println!("═══════════════════════════════════════════════════════════");
}

#[cfg(test)]
mod kiem_thu {
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
        let mut da_thay = std::collections::HashSet::new();
        for n in [0usize, 1, 135, 136, 137, 271, 272, 273, 500] {
            let b = keccak256(&vec![b'a'; n]);
            assert_eq!(b.len(), 32);
            assert!(da_thay.insert(b), "độ dài {} cho ra băm trùng với độ dài khác", n);
            // tất định
            assert_eq!(keccak256(&vec![b'a'; n]), b);
        }
    }

    #[test]
    fn keccak_nhay_voi_moi_byte_trong_input_nhieu_khoi() {
        // Với input 300 byte (3 khối), lật BẤT KỲ byte nào cũng phải đổi băm.
        // Nếu vòng lặp bọt biển bỏ sót một khối, bài này sẽ bắt được.
        let goc = vec![7u8; 300];
        let bam_goc = keccak256(&goc);
        for vi_tri in [0usize, 135, 136, 200, 271, 272, 299] {
            let mut sua = goc.clone();
            sua[vi_tri] ^= 1;
            assert_ne!(keccak256(&sua), bam_goc, "lật byte {} mà băm không đổi", vi_tri);
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
        assert_eq!(hex(&chu_ky_ham("transfer(address,uint256)")), "a9059cbb");
        assert_eq!(hex(&chu_ky_ham("balanceOf(address)")), "70a08231");
        assert_eq!(hex(&chu_ky_ham("approve(address,uint256)")), "095ea7b3");
        assert_eq!(hex(&chu_ky_ham("transferFrom(address,address,uint256)")), "23b872dd");
        assert_eq!(hex(&chu_ky_ham("totalSupply()")), "18160ddd");
    }

    #[test]
    fn topic0_su_kien_transfer_dung() {
        assert_eq!(hex(&chu_de_su_kien("Transfer(address,address,uint256)")),
            "ddf252ad1be2c89b69c2b068fc378daa952ba7f163c4a11628f55a4df523b3ef");
    }

    #[test]
    fn khoang_trang_trong_chu_ky_lam_doi_ket_qua() {
        // Chữ ký phải viết SÁT, không dấu cách. Sai chỗ này là gọi nhầm hàm.
        assert_ne!(chu_ky_ham("transfer(address,uint256)"),
                   chu_ky_ham("transfer(address, uint256)"));
    }

    // ---------- ABI ----------
    #[test]
    fn kieu_tinh_can_phai_trong_o_32_byte() {
        let m = ma_hoa_abi(&[GiaTriAbi::Uint(1)]);
        assert_eq!(m.len(), 32);
        assert_eq!(m[31], 1, "giá trị nằm ở byte CUỐI, 31 byte đầu là đệm 0");
        assert!(m[..31].iter().all(|&b| b == 0));
    }

    #[test]
    fn dia_chi_duoc_dem_12_byte_o_dau() {
        let a = dia_chi_tu_hex("0x742d35Cc6634C0532925a3b844Bc454e4438f44e");
        let m = ma_hoa_abi(&[GiaTriAbi::Address(a)]);
        assert!(m[..12].iter().all(|&b| b == 0), "12 byte đầu phải là đệm");
        assert_eq!(&m[12..32], &a);
        assert_eq!(doc_dia_chi(&m, 0), Some(a), "đọc ngược phải ra đúng địa chỉ");
    }

    #[test]
    fn so_am_duoc_mo_rong_dau_bang_ff() {
        let m = ma_hoa_abi(&[GiaTriAbi::Int(-1)]);
        assert!(m.iter().all(|&b| b == 0xFF), "-1 trong bù hai là toàn bit 1");
        let m2 = ma_hoa_abi(&[GiaTriAbi::Int(1)]);
        assert!(m2[..31].iter().all(|&b| b == 0), "số dương thì đệm 0");
    }

    #[test]
    fn bool_ma_hoa_thanh_0_hoac_1() {
        assert_eq!(ma_hoa_abi(&[GiaTriAbi::Bool(true)])[31], 1);
        assert_eq!(ma_hoa_abi(&[GiaTriAbi::Bool(false)])[31], 0);
    }

    #[test]
    fn kieu_dong_ghi_con_tro_dung_vi_tri() {
        let m = ma_hoa_abi(&[
            GiaTriAbi::Uint(42),
            GiaTriAbi::Chuoi("xin chao".into()),
            GiaTriAbi::Bool(true),
        ]);
        assert_eq!(doc_uint(&m, 0), Some(42));
        assert_eq!(doc_uint(&m, 1), Some(96), "con trỏ trỏ ngay sau phần đầu (3 ô × 32)");
        assert_eq!(doc_uint(&m, 2), Some(1), "bool nằm ở ô 2, không bị đẩy đi đâu");
        assert_eq!(doc_uint(&m, 3), Some(8), "ô đầu phần đuôi là độ dài chuỗi");
        assert_eq!(&m[128..136], b"xin chao");
    }

    #[test]
    fn kieu_dong_duoc_dem_cho_tron_32_byte() {
        let m = ma_hoa_abi(&[GiaTriAbi::Chuoi("a".into())]);
        assert_eq!(m.len() % 32, 0, "toàn bộ mã hoá ABI luôn là bội của 32");
        assert_eq!(m.len(), 32 + 32 + 32, "con trỏ + độ dài + 1 ô dữ liệu đã đệm");
    }

    #[test]
    fn nhieu_kieu_dong_khong_de_len_nhau() {
        let m = ma_hoa_abi(&[
            GiaTriAbi::Chuoi("mot".into()),
            GiaTriAbi::Chuoi("hai ba bon nam sau bay".into()),
        ]);
        let p1 = doc_uint(&m, 0).unwrap() as usize;
        let p2 = doc_uint(&m, 1).unwrap() as usize;
        assert!(p2 > p1, "con trỏ thứ hai phải nằm SAU dữ liệu thứ nhất");
        assert_eq!(&m[p1 + 32..p1 + 35], b"mot");
        assert_eq!(&m[p2 + 32..p2 + 54], b"hai ba bon nam sau bay");
    }

    #[test]
    fn mang_uint_ma_hoa_do_dai_roi_toi_phan_tu() {
        let m = ma_hoa_abi(&[GiaTriAbi::MangUint(vec![10, 20, 30])]);
        assert_eq!(doc_uint(&m, 0), Some(32), "con trỏ");
        assert_eq!(doc_uint(&m, 1), Some(3), "độ dài mảng");
        assert_eq!(doc_uint(&m, 2), Some(10));
        assert_eq!(doc_uint(&m, 3), Some(20));
        assert_eq!(doc_uint(&m, 4), Some(30));
    }

    #[test]
    fn calldata_transfer_khop_dinh_dang_that() {
        let t = Erc20 { dia_chi: [0u8; 20] };
        let den = dia_chi_tu_hex("0x742d35Cc6634C0532925a3b844Bc454e4438f44e");
        let cd = t.chuyen(den, 1_000_000);
        assert_eq!(cd.len(), 4 + 32 + 32, "4 byte chữ ký hàm + 2 ô tham số");
        assert_eq!(hex(&cd[..4]), "a9059cbb");
        assert_eq!(doc_dia_chi(&cd[4..], 0), Some(den));
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
        assert_eq!(doc_dia_chi(&d, 0), None);
    }

    // ---------- RLP ----------
    #[test]
    fn rlp_khop_vi_du_trong_sach_vang() {
        // Các ví dụ này lấy thẳng từ Ethereum Yellow Paper.
        assert_eq!(hex(&Rlp::Chuoi(b"dog".to_vec()).ma_hoa()), "83646f67");
        assert_eq!(hex(&Rlp::Chuoi(vec![]).ma_hoa()), "80");
        assert_eq!(hex(&Rlp::DanhSach(vec![]).ma_hoa()), "c0");
        assert_eq!(hex(&Rlp::Chuoi(vec![0x0f]).ma_hoa()), "0f", "byte nhỏ tự mã hoá");
        assert_eq!(hex(&Rlp::Chuoi(vec![0x04, 0x00]).ma_hoa()), "820400");
        assert_eq!(hex(&Rlp::DanhSach(vec![
            Rlp::Chuoi(b"cat".to_vec()), Rlp::Chuoi(b"dog".to_vec())]).ma_hoa()),
            "c88363617483646f67");
    }

    #[test]
    fn rlp_so_khong_la_chuoi_rong() {
        // Bẫy kinh điển: RLP(0) KHÔNG phải 0x00 mà là 0x80 (chuỗi rỗng).
        assert_eq!(hex(&Rlp::tu_so(0).ma_hoa()), "80");
        assert_ne!(Rlp::tu_so(0), Rlp::Chuoi(vec![0]));
    }

    #[test]
    fn rlp_so_khong_co_so_khong_thua_o_dau() {
        assert_eq!(Rlp::tu_so(1024), Rlp::Chuoi(vec![0x04, 0x00]));
        assert_eq!(Rlp::tu_so(255), Rlp::Chuoi(vec![0xff]));
        assert_eq!(Rlp::tu_so(256), Rlp::Chuoi(vec![0x01, 0x00]));
    }

    #[test]
    fn rlp_chuoi_dai_dung_dinh_dang_do_dai_dai() {
        let dai = vec![b'a'; 100];
        let m = Rlp::Chuoi(dai).ma_hoa();
        assert_eq!(m[0], 0xB7 + 1, "0xB7 + số byte cần để ghi độ dài");
        assert_eq!(m[1], 100);
        assert_eq!(m.len(), 2 + 100);
    }

    #[test]
    fn rlp_bien_55_va_56_byte() {
        // 55 byte dùng định dạng ngắn, 56 byte chuyển sang định dạng dài
        assert_eq!(Rlp::Chuoi(vec![b'a'; 55]).ma_hoa()[0], 0x80 + 55);
        assert_eq!(Rlp::Chuoi(vec![b'a'; 56]).ma_hoa()[0], 0xB7 + 1);
    }

    // ---------- Giao dịch ----------
    fn gd_mau() -> GiaoDich1559 {
        GiaoDich1559 {
            ma_chuoi: 1, so_thu_tu: 42,
            phi_uu_tien_toi_da: 2_000_000_000,
            phi_toi_da: 100_000_000_000,
            gioi_han_gas: 21_000,
            den: Some(dia_chi_tu_hex("0x742d35Cc6634C0532925a3b844Bc454e4438f44e")),
            gia_tri: 1_000_000_000_000_000_000, // 1 ETH
            du_lieu: vec![],
        }
    }

    #[test]
    fn tai_trong_ky_bat_dau_bang_loai_giao_dich() {
        assert_eq!(gd_mau().tai_trong_ky()[0], 0x02, "EIP-1559 là loại 0x02");
    }

    #[test]
    fn doi_bat_ky_truong_nao_cung_doi_ma_bam() {
        // Bất biến sống còn: chữ ký phải phủ TOÀN BỘ nội dung giao dịch.
        // Nếu một trường lọt ra ngoài, kẻ tấn công sửa được nó mà chữ ký vẫn hợp lệ.
        let goc = gd_mau();
        let b0 = goc.ma_bam_ky();
        let bien_the: Vec<GiaoDich1559> = vec![
            GiaoDich1559 { ma_chuoi: 5, ..goc.clone() },
            GiaoDich1559 { so_thu_tu: 43, ..goc.clone() },
            GiaoDich1559 { phi_uu_tien_toi_da: 3_000_000_000, ..goc.clone() },
            GiaoDich1559 { phi_toi_da: 90_000_000_000, ..goc.clone() },
            GiaoDich1559 { gioi_han_gas: 30_000, ..goc.clone() },
            GiaoDich1559 { den: None, ..goc.clone() },
            GiaoDich1559 { gia_tri: 2, ..goc.clone() },
            GiaoDich1559 { du_lieu: vec![1], ..goc.clone() },
        ];
        for (i, v) in bien_the.iter().enumerate() {
            assert_ne!(v.ma_bam_ky(), b0, "biến thể {} phải cho mã băm khác", i);
        }
    }

    #[test]
    fn tao_hop_dong_ma_hoa_dich_thanh_chuoi_rong() {
        let tao = GiaoDich1559 { den: None, ..gd_mau() };
        let gui = gd_mau();
        assert_ne!(tao.tai_trong_ky(), gui.tai_trong_ky());
        // `den: None` phải thành 0x80 (chuỗi rỗng), không phải 20 byte 0
        assert!(tao.tai_trong_ky().len() < gui.tai_trong_ky().len());
    }

    #[test]
    fn chi_phi_toi_da_dung_cong_thuc_vi_phai_khoa() {
        let gd = gd_mau();
        assert_eq!(gd.chi_phi_toi_da(),
                   1_000_000_000_000_000_000 + 100_000_000_000 * 21_000);
    }

    #[test]
    fn phi_thuc_te_khong_bao_gio_vuot_tran_nguoi_dung_dat() {
        let gd = gd_mau();
        for base in [1u128, 50_000_000_000, 99_000_000_000, 100_000_000_000] {
            assert!(gd.phi_thuc_te(base) <= gd.phi_toi_da,
                    "base {} → thực trả {} vượt trần {}",
                    base, gd.phi_thuc_te(base), gd.phi_toi_da);
        }
    }

    #[test]
    fn base_fee_thap_thi_tra_tron_ven_tien_boa() {
        let gd = gd_mau();
        let base = 10_000_000_000u128;
        assert_eq!(gd.phi_thuc_te(base), base + gd.phi_uu_tien_toi_da);
    }

    #[test]
    fn base_fee_gan_tran_thi_tien_boa_bi_bop_lai() {
        let gd = gd_mau();
        let base = 99_000_000_000u128; // trần 100 gwei, chỉ còn 1 gwei cho boa
        assert_eq!(gd.phi_thuc_te(base), 100_000_000_000,
                   "tiền boa bị cắt xuống 1 gwei chứ không phải 2");
    }

    #[test]
    fn base_fee_vuot_tran_thi_phep_tinh_khong_tran_so() {
        let gd = gd_mau();
        assert_eq!(gd.phi_thuc_te(200_000_000_000), 200_000_000_000,
                   "giao dịch này sẽ không được chọn vào khối, nhưng không được panic");
    }
}
