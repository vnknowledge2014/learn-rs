#![allow(dead_code)]
//! Chương 67 — FPGA & Thiết kế phần cứng số bằng Rust: cổng logic, mạch tổ hợp,
//! mạch tuần tự có xung nhịp, đường ống, và vì sao phần cứng nhanh hơn phần mềm.
//!
//! Tinh thần lấy từ rust-hdl (nay đang được tác giả viết lại thành `rhdl`):
//! mô tả phần cứng bằng KIỂU của Rust, mô phỏng ngay trong `cargo test`,
//! rồi mới sinh Verilog. Sai thiết kế bị bắt lúc biên dịch, không phải sau
//! 40 phút tổng hợp mạch.

use std::collections::HashMap;

// ============================================================================
// 1. TÍN HIỆU & CỔNG LOGIC — vật liệu xây dựng duy nhất
// ============================================================================

/// Trong FPGA thật, tín hiệu còn có trạng thái 'X' (không xác định) và 'Z'
/// (trở kháng cao). Ta mô hình hóa cả 'X' vì nó là nguồn lỗi kinh điển:
/// quên khởi tạo thanh ghi → mạch chạy đúng trong mô phỏng, sai trên chip.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum TinHieu { Thap, Cao, KhongXacDinh }

impl TinHieu {
    pub fn tu_bool(b: bool) -> TinHieu { if b { TinHieu::Cao } else { TinHieu::Thap } }
    pub fn thanh_bool(self) -> Option<bool> {
        match self { TinHieu::Cao => Some(true), TinHieu::Thap => Some(false), _ => None }
    }
}

pub fn cong_khong(a: TinHieu) -> TinHieu {
    match a { TinHieu::Cao => TinHieu::Thap, TinHieu::Thap => TinHieu::Cao, x => x }
}
pub fn cong_va(a: TinHieu, b: TinHieu) -> TinHieu {
    // Lưu ý: 0 AND X = 0, KHÔNG phải X — vì kết quả đã xác định dù X là gì.
    // Đây gọi là "làm ngắn mạch giá trị điều khiển" và có thật trên silicon.
    match (a, b) {
        (TinHieu::Thap, _) | (_, TinHieu::Thap) => TinHieu::Thap,
        (TinHieu::Cao, TinHieu::Cao) => TinHieu::Cao,
        _ => TinHieu::KhongXacDinh,
    }
}
pub fn cong_hoac(a: TinHieu, b: TinHieu) -> TinHieu {
    match (a, b) {
        (TinHieu::Cao, _) | (_, TinHieu::Cao) => TinHieu::Cao,
        (TinHieu::Thap, TinHieu::Thap) => TinHieu::Thap,
        _ => TinHieu::KhongXacDinh,
    }
}
pub fn cong_xor(a: TinHieu, b: TinHieu) -> TinHieu {
    match (a.thanh_bool(), b.thanh_bool()) {
        (Some(x), Some(y)) => TinHieu::tu_bool(x ^ y),
        _ => TinHieu::KhongXacDinh, // XOR KHÔNG có giá trị điều khiển
    }
}
/// NAND là cổng "phổ dụng": mọi hàm logic đều dựng được chỉ từ NAND.
pub fn cong_nand(a: TinHieu, b: TinHieu) -> TinHieu { cong_khong(cong_va(a, b)) }

/// Bộ chọn kênh 2-1 — viên gạch của mọi thứ có chữ "if" trong phần cứng.
pub fn bo_chon(chon: TinHieu, khi_0: TinHieu, khi_1: TinHieu) -> TinHieu {
    cong_hoac(cong_va(cong_khong(chon), khi_0), cong_va(chon, khi_1))
}

// ============================================================================
// 2. MẠCH TỔ HỢP — đầu ra chỉ phụ thuộc đầu vào HIỆN TẠI
// ============================================================================

/// Bộ cộng bán phần: cộng 2 bit, cho tổng và nhớ.
pub fn cong_ban_phan(a: TinHieu, b: TinHieu) -> (TinHieu, TinHieu) {
    (cong_xor(a, b), cong_va(a, b))
}

/// Bộ cộng toàn phần: cộng 2 bit CỘNG bit nhớ vào.
pub fn cong_toan_phan(a: TinHieu, b: TinHieu, nho_vao: TinHieu) -> (TinHieu, TinHieu) {
    let (t1, n1) = cong_ban_phan(a, b);
    let (tong, n2) = cong_ban_phan(t1, nho_vao);
    (tong, cong_hoac(n1, n2))
}

#[derive(Debug, PartialEq)]
pub struct KetQuaCong {
    pub tong: u16,
    pub tran: bool,
    /// Số tầng cổng mà tín hiệu phải đi qua — quyết định TẦN SỐ TỐI ĐA của mạch.
    pub do_sau_cong: usize,
}

/// Bộ cộng nhớ nối tiếp 8 bit — cách dựng đơn giản nhất, và CHẬM nhất.
/// Bit nhớ phải "chảy" tuần tự qua cả 8 tầng: độ trễ tỉ lệ THUẬN với số bit.
pub fn cong_noi_tiep_8bit(a: u8, b: u8) -> KetQuaCong {
    let mut nho = TinHieu::Thap;
    let mut tong = 0u16;
    for i in 0..8 {
        let bit_a = TinHieu::tu_bool((a >> i) & 1 == 1);
        let bit_b = TinHieu::tu_bool((b >> i) & 1 == 1);
        let (s, n) = cong_toan_phan(bit_a, bit_b, nho);
        if s == TinHieu::Cao { tong |= 1 << i; }
        nho = n;
    }
    KetQuaCong {
        tong,
        tran: nho == TinHieu::Cao,
        do_sau_cong: 8 * 3, // mỗi bộ cộng toàn phần ~3 tầng cổng, nối tiếp nhau
    }
}

/// Bộ cộng nhìn trước nhớ (carry-lookahead): tính TẤT CẢ bit nhớ SONG SONG
/// từ hai tín hiệu "sinh nhớ" (G = a·b) và "truyền nhớ" (P = a⊕b).
/// Cùng kết quả, nhưng độ sâu chỉ còn ~log(n) thay vì n. Đây là bài học
/// cốt lõi của phần cứng: ĐÁNH ĐỔI DIỆN TÍCH LẤY TỐC ĐỘ.
pub fn cong_nhin_truoc_8bit(a: u8, b: u8) -> KetQuaCong {
    let g = a & b;          // sinh nhớ
    let p = a ^ b;          // truyền nhớ
    let mut nho = [false; 9];
    for i in 0..8 {
        // c[i+1] = G[i] + P[i]·c[i] — trong phần cứng, khai triển hết thành
        // một biểu thức phẳng nên tính đồng thời chỉ trong vài tầng cổng.
        nho[i + 1] = ((g >> i) & 1 == 1) || (((p >> i) & 1 == 1) && nho[i]);
    }
    let mut tong = 0u16;
    for i in 0..8 {
        if ((p >> i) & 1 == 1) ^ nho[i] { tong |= 1 << i; }
    }
    KetQuaCong { tong, tran: nho[8], do_sau_cong: 5 } // ~log2(8) + vài tầng
}

// ============================================================================
// 3. MẠCH TUẦN TỰ — có xung nhịp và TRÍ NHỚ
// ============================================================================

/// Flip-flop D: viên gạch của mọi trí nhớ trong FPGA.
/// Ở MỖI sườn lên của xung nhịp, chốt lấy giá trị đầu vào; giữa hai sườn thì
/// giữ nguyên bất kể đầu vào đổi thế nào.
#[derive(Debug, Clone, Copy, PartialEq)]
pub struct FlipFlopD {
    q: TinHieu,
}

impl FlipFlopD {
    /// Chưa reset thì giá trị là KHÔNG XÁC ĐỊNH — đúng như silicon thật.
    pub fn moi() -> Self { FlipFlopD { q: TinHieu::KhongXacDinh } }
    pub fn q(&self) -> TinHieu { self.q }
    pub fn suon_len(&mut self, d: TinHieu) { self.q = d; }
    pub fn dat_lai(&mut self) { self.q = TinHieu::Thap; }
}

/// Thanh ghi dịch — dùng cho SPI, UART, tính CRC, tạo số giả ngẫu nhiên.
pub struct ThanhGhiDich<const N: usize> {
    o: [FlipFlopD; N],
}

impl<const N: usize> ThanhGhiDich<N> {
    pub fn moi() -> Self { ThanhGhiDich { o: [FlipFlopD::moi(); N] } }
    pub fn dat_lai(&mut self) { for f in self.o.iter_mut() { f.dat_lai(); } }
    /// Đẩy 1 bit vào đầu, bit ở cuối rơi ra. Toàn bộ N flip-flop cập nhật
    /// ĐỒNG THỜI trong một chu kỳ — không có vòng lặp nào chạy trên chip.
    ///
    /// Chú ý vòng lặp chạy NGƯỢC (`(1..N).rev()`): phải chép từ cuối về đầu,
    /// nếu không giá trị mới của o[i-1] sẽ đè lên giá trị cũ mà o[i] cần đọc.
    /// Lỗi này khiến cả thanh ghi biến thành một flip-flop duy nhất.
    ///
    /// Đầu ra được lấy SAU sườn xung — đúng như Q của flip-flop cuối đổi
    /// giá trị ngay tại sườn. Đọc trước sườn sẽ trễ một chu kỳ; đây là lỗi
    /// lệch-một kinh điển khi viết mô phỏng HDL.
    pub fn suon_len(&mut self, vao: TinHieu) -> TinHieu {
        for i in (1..N).rev() {
            let truoc = self.o[i - 1].q();
            self.o[i].suon_len(truoc);
        }
        self.o[0].suon_len(vao);
        self.o[N - 1].q()
    }
    pub fn doc(&self) -> Vec<TinHieu> { self.o.iter().map(|f| f.q()).collect() }
}

/// Máy trạng thái hữu hạn có xung nhịp — đèn giao thông.
/// Đây là dạng mạch mà FPGA làm tốt nhất: điều khiển tất định, độ trễ đếm được.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum DenGiaoThong { Do, DoVang, Xanh, Vang }

pub struct BoDieuKhienDen {
    pub trang_thai: DenGiaoThong,
    pub bo_dem: u8,
    pub thoi_luong: [u8; 4],
}

impl BoDieuKhienDen {
    pub fn moi() -> Self {
        BoDieuKhienDen { trang_thai: DenGiaoThong::Do, bo_dem: 0, thoi_luong: [5, 1, 4, 2] }
    }
    fn chi_so(&self) -> usize {
        match self.trang_thai {
            DenGiaoThong::Do => 0, DenGiaoThong::DoVang => 1,
            DenGiaoThong::Xanh => 2, DenGiaoThong::Vang => 3,
        }
    }
    /// Một sườn xung nhịp. Toàn bộ logic là TỔ HỢP, chỉ `trang_thai` và
    /// `bo_dem` nằm trong flip-flop — đây là mẫu "logic tách khỏi thanh ghi".
    pub fn suon_len(&mut self) -> DenGiaoThong {
        self.bo_dem += 1;
        if self.bo_dem >= self.thoi_luong[self.chi_so()] {
            self.bo_dem = 0;
            self.trang_thai = match self.trang_thai {
                DenGiaoThong::Do => DenGiaoThong::DoVang,
                DenGiaoThong::DoVang => DenGiaoThong::Xanh,
                DenGiaoThong::Xanh => DenGiaoThong::Vang,
                DenGiaoThong::Vang => DenGiaoThong::Do,
            };
        }
        self.trang_thai
    }
    /// Ràng buộc AN TOÀN: không bao giờ được nhảy thẳng Xanh → Đỏ.
    pub fn chuyen_hop_le(tu: DenGiaoThong, den: DenGiaoThong) -> bool {
        use DenGiaoThong::*;
        matches!((tu, den), (Do, Do) | (Do, DoVang) | (DoVang, DoVang) | (DoVang, Xanh)
                          | (Xanh, Xanh) | (Xanh, Vang) | (Vang, Vang) | (Vang, Do))
    }
}

// ============================================================================
// 4. ĐƯỜNG ỐNG (pipeline) — bí quyết tăng thông lượng của mọi CPU/GPU
// ============================================================================

#[derive(Debug, PartialEq)]
pub struct KetQuaOng {
    pub dau_ra: Vec<u32>,
    pub so_chu_ky: usize,
    /// Độ trễ: bao nhiêu chu kỳ từ lúc nạp đến lúc có kết quả ĐẦU TIÊN.
    pub do_tre: usize,
}

/// Không đường ống: mỗi phần tử phải đi hết `so_tang` giai đoạn rồi mới
/// nạp phần tử kế. Thông lượng = 1 kết quả / `so_tang` chu kỳ.
pub fn xu_ly_khong_ong(dau_vao: &[u32], so_tang: usize, f: impl Fn(u32) -> u32) -> KetQuaOng {
    let dau_ra: Vec<u32> = dau_vao.iter().map(|&x| f(x)).collect();
    KetQuaOng { so_chu_ky: dau_vao.len() * so_tang, do_tre: so_tang, dau_ra }
}

/// Có đường ống: mỗi tầng có thanh ghi riêng, nên `so_tang` phần tử được xử lý
/// ĐỒNG THỜI ở các giai đoạn khác nhau. Sau khi ống đầy: 1 kết quả MỖI chu kỳ.
pub fn xu_ly_co_ong(dau_vao: &[u32], so_tang: usize, f: impl Fn(u32) -> u32) -> KetQuaOng {
    let mut tang: Vec<Option<u32>> = vec![None; so_tang];
    let mut dau_ra = Vec::new();
    let mut chi_so = 0;
    let mut chu_ky = 0;

    while dau_ra.len() < dau_vao.len() {
        // Dịch từ CUỐI về ĐẦU để không ghi đè dữ liệu chưa dùng —
        // giống hệt cách thanh ghi thật cập nhật đồng thời trên sườn xung.
        if let Some(v) = tang[so_tang - 1] { dau_ra.push(v); }
        for i in (1..so_tang).rev() { tang[i] = tang[i - 1]; }
        tang[0] = if chi_so < dau_vao.len() {
            let v = f(dau_vao[chi_so]); chi_so += 1; Some(v)
        } else { None };
        chu_ky += 1;
    }
    KetQuaOng { dau_ra, so_chu_ky: chu_ky, do_tre: so_tang }
}

// ============================================================================
// 5. NETLIST — mô tả mạch dưới dạng đồ thị, rồi mô phỏng
// ============================================================================

#[derive(Debug, Clone)]
pub enum Nut {
    DauVao(String),
    Khong(usize),
    Va(usize, usize),
    Hoac(usize, usize),
    Xor(usize, usize),
}

/// Danh sách nối (netlist) chính là thứ trình tổng hợp sinh ra từ HDL,
/// và cũng là thứ được nạp xuống FPGA.
pub struct MachDien {
    pub nut: Vec<Nut>,
}

impl MachDien {
    pub fn moi() -> Self { MachDien { nut: Vec::new() } }
    pub fn them(&mut self, n: Nut) -> usize { self.nut.push(n); self.nut.len() - 1 }

    /// Mô phỏng: vì netlist là đồ thị không chu trình, tính lần lượt theo
    /// thứ tự thêm vào là đủ — đó chính là "sắp xếp tô-pô" miễn phí.
    pub fn mo_phong(&self, dau_vao: &HashMap<String, TinHieu>) -> Vec<TinHieu> {
        let mut gt = vec![TinHieu::KhongXacDinh; self.nut.len()];
        for (i, n) in self.nut.iter().enumerate() {
            gt[i] = match n {
                Nut::DauVao(ten) => *dau_vao.get(ten).unwrap_or(&TinHieu::KhongXacDinh),
                Nut::Khong(a) => cong_khong(gt[*a]),
                Nut::Va(a, b) => cong_va(gt[*a], gt[*b]),
                Nut::Hoac(a, b) => cong_hoac(gt[*a], gt[*b]),
                Nut::Xor(a, b) => cong_xor(gt[*a], gt[*b]),
            };
        }
        gt
    }

    /// Đường tới hạn: chuỗi cổng DÀI NHẤT từ đầu vào tới đầu ra.
    /// Tần số tối đa của mạch = 1 / (độ trễ đường tới hạn).
    pub fn duong_toi_han(&self) -> usize {
        let mut sau = vec![0usize; self.nut.len()];
        for (i, n) in self.nut.iter().enumerate() {
            sau[i] = match n {
                Nut::DauVao(_) => 0,
                Nut::Khong(a) => sau[*a] + 1,
                Nut::Va(a, b) | Nut::Hoac(a, b) | Nut::Xor(a, b) => sau[*a].max(sau[*b]) + 1,
            };
        }
        sau.into_iter().max().unwrap_or(0)
    }
}

fn main() {
    println!("═══════════════════════════════════════════════════════════");
    println!("   FPGA: CỔNG LOGIC · BỘ CỘNG · FLIP-FLOP · ĐƯỜNG ỐNG       ");
    println!("═══════════════════════════════════════════════════════════");

    println!("\n1. BẢNG CHÂN TRỊ CÓ TRẠNG THÁI 'X'");
    println!("   0 AND X = {:?}  ← đã xác định! (0 là giá trị điều khiển của AND)",
             cong_va(TinHieu::Thap, TinHieu::KhongXacDinh));
    println!("   1 AND X = {:?}", cong_va(TinHieu::Cao, TinHieu::KhongXacDinh));
    println!("   0 XOR X = {:?}  ← XOR không có giá trị điều khiển",
             cong_xor(TinHieu::Thap, TinHieu::KhongXacDinh));

    println!("\n2. HAI CÁCH DỰNG BỘ CỘNG 8 BIT — cùng kết quả, khác tốc độ");
    for (a, b) in [(200u8, 100u8), (255, 1), (37, 91)] {
        let nt = cong_noi_tiep_8bit(a, b);
        let lt = cong_nhin_truoc_8bit(a, b);
        println!("   {:>3} + {:>3} = {:>3} (tràn {}) | nối tiếp {} tầng · nhìn trước {} tầng",
                 a, b, nt.tong, nt.tran, nt.do_sau_cong, lt.do_sau_cong);
        assert_eq!(nt.tong, lt.tong);
    }
    println!("   → Cùng đáp số, nhưng mạch nhìn trước chạy nhanh hơn ~{}×",
             cong_noi_tiep_8bit(0,0).do_sau_cong / cong_nhin_truoc_8bit(0,0).do_sau_cong);

    println!("\n3. THANH GHI DỊCH 4 BIT");
    let mut tg: ThanhGhiDich<4> = ThanhGhiDich::moi();
    tg.dat_lai();
    print!("   Đẩy 1,0,1,1 → ra: ");
    for v in [true, false, true, true] {
        print!("{:?} ", tg.suon_len(TinHieu::tu_bool(v)));
    }
    println!("\n   Nội dung sau 4 chu kỳ: {:?}", tg.doc());

    println!("\n4. MÁY TRẠNG THÁI ĐÈN GIAO THÔNG (mỗi ký tự = 1 chu kỳ nhịp)");
    let mut den = BoDieuKhienDen::moi();
    let chuoi: String = (0..24).map(|_| match den.suon_len() {
        DenGiaoThong::Do => 'Đ', DenGiaoThong::DoVang => 'v',
        DenGiaoThong::Xanh => 'X', DenGiaoThong::Vang => 'V',
    }).collect();
    println!("   {}", chuoi);
    println!("   Không bao giờ có 'XĐ' (xanh nhảy thẳng sang đỏ): {}", !chuoi.contains("XĐ"));

    println!("\n5. ĐƯỜNG ỐNG — 100 phần tử qua mạch 5 tầng");
    let vao: Vec<u32> = (0..100).collect();
    let khong = xu_ly_khong_ong(&vao, 5, |x| x * x);
    let co = xu_ly_co_ong(&vao, 5, |x| x * x);
    println!("   Không ống: {} chu kỳ (độ trễ {})", khong.so_chu_ky, khong.do_tre);
    println!("   Có ống   : {} chu kỳ (độ trễ {}) → nhanh gấp {:.1}×",
             co.so_chu_ky, co.do_tre, khong.so_chu_ky as f64 / co.so_chu_ky as f64);
    println!("   → Độ trễ KHÔNG giảm; chỉ THÔNG LƯỢNG tăng. Hai đại lượng khác nhau.");

    println!("\n6. NETLIST & ĐƯỜNG TỚI HẠN");
    let mut m = MachDien::moi();
    let a = m.them(Nut::DauVao("a".into()));
    let b = m.them(Nut::DauVao("b".into()));
    let c = m.them(Nut::DauVao("c".into()));
    let x = m.them(Nut::Xor(a, b));
    let y = m.them(Nut::Xor(x, c));      // tổng của bộ cộng toàn phần
    let _ = y;
    let mut vao_map = HashMap::new();
    for (k, v) in [("a", true), ("b", true), ("c", false)] {
        vao_map.insert(k.to_string(), TinHieu::tu_bool(v));
    }
    println!("   1 XOR 1 XOR 0 = {:?}", m.mo_phong(&vao_map)[y]);
    println!("   Đường tới hạn = {} tầng cổng", m.duong_toi_han());

    println!("\n═══════════════════════════════════════════════════════════");
    println!("   PHẦN MỀM SONG SONG THEO THỜI GIAN — PHẦN CỨNG THEO KHÔNG GIAN");
    println!("═══════════════════════════════════════════════════════════");
}

#[cfg(test)]
mod kiem_thu {
    use super::*;
    use TinHieu::{Cao, KhongXacDinh, Thap};

    // ---------- Cổng logic ----------
    #[test]
    fn gia_tri_dieu_khien_lam_tan_bien_trang_thai_x() {
        // Bài học phần cứng thật: 0·X = 0 và 1+X = 1, dù X là gì đi nữa.
        assert_eq!(cong_va(Thap, KhongXacDinh), Thap);
        assert_eq!(cong_va(KhongXacDinh, Thap), Thap);
        assert_eq!(cong_hoac(Cao, KhongXacDinh), Cao);
        // nhưng khi không có giá trị điều khiển thì X lan ra
        assert_eq!(cong_va(Cao, KhongXacDinh), KhongXacDinh);
        assert_eq!(cong_xor(Thap, KhongXacDinh), KhongXacDinh);
    }

    #[test]
    fn nand_la_cong_pho_dung() {
        // Dựng NOT, AND, OR chỉ từ NAND — nền tảng của mọi thư viện cổng.
        let khong = |a| cong_nand(a, a);
        let va = |a, b| khong(cong_nand(a, b));
        let hoac = |a, b| cong_nand(khong(a), khong(b));
        for a in [Thap, Cao] {
            assert_eq!(khong(a), cong_khong(a));
            for b in [Thap, Cao] {
                assert_eq!(va(a, b), cong_va(a, b));
                assert_eq!(hoac(a, b), cong_hoac(a, b));
            }
        }
    }

    #[test]
    fn bo_chon_hoat_dong_nhu_lenh_if() {
        assert_eq!(bo_chon(Thap, Cao, Thap), Cao, "chọn=0 → lấy nhánh 0");
        assert_eq!(bo_chon(Cao, Cao, Thap), Thap, "chọn=1 → lấy nhánh 1");
    }

    #[test]
    fn luat_de_morgan_dung_tren_mach() {
        for a in [Thap, Cao] {
            for b in [Thap, Cao] {
                assert_eq!(cong_khong(cong_va(a, b)),
                           cong_hoac(cong_khong(a), cong_khong(b)));
                assert_eq!(cong_khong(cong_hoac(a, b)),
                           cong_va(cong_khong(a), cong_khong(b)));
            }
        }
    }

    // ---------- Bộ cộng ----------
    #[test]
    fn cong_toan_phan_dung_ca_8_to_hop() {
        for a in [false, true] { for b in [false, true] { for c in [false, true] {
            let (t, n) = cong_toan_phan(TinHieu::tu_bool(a), TinHieu::tu_bool(b), TinHieu::tu_bool(c));
            let tong = a as u8 + b as u8 + c as u8;
            assert_eq!(t.thanh_bool(), Some(tong & 1 == 1));
            assert_eq!(n.thanh_bool(), Some(tong >= 2));
        }}}
    }

    #[test]
    fn bo_cong_8bit_khop_voi_so_hoc_may_tinh() {
        // Kiểm thử vét cạn TOÀN BỘ 65 536 tổ hợp — điều bất khả với mạch lớn,
        // nhưng với 8 bit thì đây là chứng minh tuyệt đối.
        for a in 0u16..256 {
            for b in 0u16..256 {
                let kq = cong_noi_tiep_8bit(a as u8, b as u8);
                let that = a + b;
                assert_eq!(kq.tong, that & 0xFF, "{a}+{b}");
                assert_eq!(kq.tran, that > 255, "{a}+{b} phải báo tràn");
            }
        }
    }

    #[test]
    fn hai_kien_truc_cong_cho_ket_qua_y_het_nhau() {
        for a in 0u16..256 {
            for b in 0u16..256 {
                let nt = cong_noi_tiep_8bit(a as u8, b as u8);
                let lt = cong_nhin_truoc_8bit(a as u8, b as u8);
                assert_eq!((nt.tong, nt.tran), (lt.tong, lt.tran),
                           "hai kiến trúc phải tương đương về CHỨC NĂNG: {a}+{b}");
            }
        }
    }

    #[test]
    fn nhin_truoc_nong_hon_noi_tiep() {
        // Đây là toàn bộ lý do người ta chịu tốn thêm cổng cho carry-lookahead.
        assert!(cong_nhin_truoc_8bit(0, 0).do_sau_cong < cong_noi_tiep_8bit(0, 0).do_sau_cong);
    }

    // ---------- Mạch tuần tự ----------
    #[test]
    fn flip_flop_chua_reset_la_khong_xac_dinh() {
        let f = FlipFlopD::moi();
        assert_eq!(f.q(), KhongXacDinh, "silicon thật cũng vậy — phải reset trước khi dùng");
    }

    #[test]
    fn flip_flop_chot_gia_tri_tai_suon_len() {
        let mut f = FlipFlopD::moi();
        f.dat_lai();
        assert_eq!(f.q(), Thap);
        f.suon_len(Cao);
        assert_eq!(f.q(), Cao);
    }

    #[test]
    fn thanh_ghi_dich_tra_bit_sau_dung_n_chu_ky() {
        let mut tg: ThanhGhiDich<4> = ThanhGhiDich::moi();
        tg.dat_lai();
        // Bit đầu tiên phải mất ĐÚNG N = 4 chu kỳ mới ra tới đầu kia.
        // Đây chính là độ trễ của thanh ghi dịch — nền của SPI và UART.
        assert_eq!(tg.suon_len(Cao), Thap);
        assert_eq!(tg.suon_len(Thap), Thap);
        assert_eq!(tg.suon_len(Thap), Thap);
        assert_eq!(tg.suon_len(Thap), Cao, "bit '1' xuất hiện đúng ở chu kỳ thứ 4");
        assert_eq!(tg.suon_len(Thap), Thap, "sau đó ống rỗng trở lại");
    }

    #[test]
    fn den_giao_thong_khong_bao_gio_nhay_xanh_sang_do() {
        let mut d = BoDieuKhienDen::moi();
        let mut truoc = d.trang_thai;
        for _ in 0..200 {
            let nay = d.suon_len();
            assert!(BoDieuKhienDen::chuyen_hop_le(truoc, nay),
                    "chuyển trái phép {:?} → {:?}", truoc, nay);
            truoc = nay;
        }
    }

    #[test]
    fn den_giao_thong_di_het_chu_trinh_va_lap_lai() {
        let mut d = BoDieuKhienDen::moi();
        let tong: u32 = d.thoi_luong.iter().map(|&x| x as u32).sum();
        let mot_vong: Vec<DenGiaoThong> = (0..tong).map(|_| d.suon_len()).collect();
        let vong_hai: Vec<DenGiaoThong> = (0..tong).map(|_| d.suon_len()).collect();
        assert_eq!(mot_vong, vong_hai, "máy trạng thái phải tuần hoàn đúng chu kỳ");
        // và ghé qua đủ cả 4 trạng thái
        for tt in [DenGiaoThong::Do, DenGiaoThong::DoVang, DenGiaoThong::Xanh, DenGiaoThong::Vang] {
            assert!(mot_vong.contains(&tt), "thiếu trạng thái {:?}", tt);
        }
    }

    // ---------- Đường ống ----------
    #[test]
    fn duong_ong_cho_cung_ket_qua_nhung_nhanh_hon_nhieu() {
        let vao: Vec<u32> = (1..=50).collect();
        let khong = xu_ly_khong_ong(&vao, 5, |x| x * 3);
        let co = xu_ly_co_ong(&vao, 5, |x| x * 3);
        assert_eq!(khong.dau_ra, co.dau_ra, "đường ống không được đổi KẾT QUẢ");
        assert!(co.so_chu_ky < khong.so_chu_ky);
    }

    #[test]
    fn duong_ong_dat_thong_luong_mot_ket_qua_moi_chu_ky() {
        let vao: Vec<u32> = (0..100).collect();
        let co = xu_ly_co_ong(&vao, 5, |x| x + 1);
        // 100 phần tử + 5 chu kỳ đổ đầy ống ≈ 105, chứ không phải 500
        assert!(co.so_chu_ky <= vao.len() + 5,
                "sau khi đầy ống phải ra 1 kết quả/chu kỳ, thực tế {} chu kỳ", co.so_chu_ky);
    }

    #[test]
    fn duong_ong_khong_lam_giam_do_tre() {
        let vao: Vec<u32> = (0..20).collect();
        let khong = xu_ly_khong_ong(&vao, 4, |x| x);
        let co = xu_ly_co_ong(&vao, 4, |x| x);
        assert_eq!(co.do_tre, khong.do_tre,
                   "đường ống tăng THÔNG LƯỢNG, không giảm ĐỘ TRỄ — đừng nhầm hai thứ");
    }

    // ---------- Netlist ----------
    #[test]
    fn mo_phong_netlist_khop_voi_ham_truc_tiep() {
        let mut m = MachDien::moi();
        let a = m.them(Nut::DauVao("a".into()));
        let b = m.them(Nut::DauVao("b".into()));
        let c = m.them(Nut::DauVao("c".into()));
        let x = m.them(Nut::Xor(a, b));
        let y = m.them(Nut::Xor(x, c));
        for va in [false, true] { for vb in [false, true] { for vc in [false, true] {
            let mut vao = HashMap::new();
            vao.insert("a".to_string(), TinHieu::tu_bool(va));
            vao.insert("b".to_string(), TinHieu::tu_bool(vb));
            vao.insert("c".to_string(), TinHieu::tu_bool(vc));
            let (tong_that, _) = cong_toan_phan(TinHieu::tu_bool(va), TinHieu::tu_bool(vb), TinHieu::tu_bool(vc));
            assert_eq!(m.mo_phong(&vao)[y], tong_that);
        }}}
    }

    #[test]
    fn duong_toi_han_dem_dung_so_tang_sau_nhat() {
        let mut m = MachDien::moi();
        let a = m.them(Nut::DauVao("a".into()));
        let b = m.them(Nut::DauVao("b".into()));
        let x = m.them(Nut::Va(a, b));         // sâu 1
        let y = m.them(Nut::Khong(x));         // sâu 2
        let _z = m.them(Nut::Hoac(y, a));      // sâu 3 (nhánh a sâu 0, lấy max)
        assert_eq!(m.duong_toi_han(), 3);
    }

    #[test]
    fn dau_vao_thieu_lan_truyen_thanh_x() {
        let mut m = MachDien::moi();
        let a = m.them(Nut::DauVao("a".into()));
        let b = m.them(Nut::DauVao("b_quen_noi".into()));
        let x = m.them(Nut::Xor(a, b));
        let mut vao = HashMap::new();
        vao.insert("a".to_string(), Cao);
        assert_eq!(m.mo_phong(&vao)[x], KhongXacDinh,
                   "quên nối một dây → X lan tới đầu ra, đúng như mô phỏng thật");
    }
}
