#![allow(dead_code)]
//! Chương 65 — Mạng máy tính & Giao thức: đóng gói theo tầng, máy trạng thái TCP,
//! điều khiển tắc nghẽn, tổng kiểm tra Internet, CIDR, và bản ghi DNS.

use std::collections::VecDeque;
use std::fmt;

// ============================================================================
// 1. MÔ HÌNH PHÂN TẦNG & SỰ ĐÓNG GÓI (encapsulation)
// ============================================================================

#[derive(Debug, Clone, Copy, PartialEq, Eq, PartialOrd, Ord)]
pub enum Tang {
    VatLy = 1,
    LienKet = 2,   // Ethernet — địa chỉ MAC, trong một mạng LAN
    Mang = 3,      // IP — địa chỉ IP, định tuyến giữa các mạng
    GiaoVan = 4,   // TCP/UDP — cổng, tin cậy
    UngDung = 7,   // HTTP/DNS — ý nghĩa dữ liệu
}

impl fmt::Display for Tang {
    fn fmt(&self, f: &mut fmt::Formatter) -> fmt::Result {
        let t = match self {
            Tang::VatLy => "Vật lý", Tang::LienKet => "Liên kết",
            Tang::Mang => "Mạng", Tang::GiaoVan => "Giao vận", Tang::UngDung => "Ứng dụng",
        };
        write!(f, "L{} {}", *self as u8, t)
    }
}

/// Mỗi tầng BỌC dữ liệu của tầng trên bằng phần đầu (header) của mình.
/// Giống gửi thư: thư → phong bì → túi bưu chính → xe tải.
#[derive(Debug, Clone, PartialEq)]
pub struct GoiTin {
    pub tang: Tang,
    pub phan_dau: Vec<u8>,
    pub tai: Vec<u8>, // payload — chính là gói của tầng trên đã tuần tự hóa
}

impl GoiTin {
    pub fn tuan_tu_hoa(&self) -> Vec<u8> {
        let mut v = self.phan_dau.clone();
        v.extend_from_slice(&self.tai);
        v
    }
    /// Bọc gói này vào một tầng thấp hơn.
    pub fn boc(self, tang_duoi: Tang, phan_dau: Vec<u8>) -> GoiTin {
        GoiTin { tang: tang_duoi, phan_dau, tai: self.tuan_tu_hoa() }
    }
    /// Tổng chi phí phần đầu khi biết kích thước từng header đã dùng.
    pub fn kich_thuoc(&self) -> usize { self.phan_dau.len() + self.tai.len() }
}

/// Dựng chồng giao thức: dữ liệu ứng dụng đi xuống, mỗi tầng thêm header.
pub fn dong_goi_xuong(du_lieu_ung_dung: &[u8]) -> GoiTin {
    let http = GoiTin { tang: Tang::UngDung, phan_dau: b"GET / HTTP/1.1\r\n\r\n".to_vec(),
                        tai: du_lieu_ung_dung.to_vec() };
    let tcp = http.boc(Tang::GiaoVan, vec![0u8; 20]);   // TCP header tối thiểu 20 byte
    let ip = tcp.boc(Tang::Mang, vec![0u8; 20]);        // IPv4 header tối thiểu 20 byte
    ip.boc(Tang::LienKet, vec![0u8; 14])                // Ethernet header 14 byte
}

// ============================================================================
// 2. MÁY TRẠNG THÁI TCP — trái tim của độ tin cậy
// ============================================================================

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum TrangThaiTcp {
    Dong,          // CLOSED
    Nghe,          // LISTEN
    DaGuiSyn,      // SYN_SENT
    DaNhanSyn,     // SYN_RECEIVED
    DaThietLap,    // ESTABLISHED
    ChoDong1,      // FIN_WAIT_1
    ChoDong2,      // FIN_WAIT_2
    ChoCuoi,       // TIME_WAIT — chờ 2×MSL để gói lạc đường chết hẳn
    ChoDongThuDong,// CLOSE_WAIT
    ChoXacNhanCuoi,// LAST_ACK
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum SuKienTcp {
    MoChuDong,   // ứng dụng gọi connect()
    MoThuDong,   // ứng dụng gọi listen()
    NhanSyn,
    NhanSynAck,
    NhanAck,
    NhanFin,
    UngDungDong, // ứng dụng gọi close()
    HetGio,      // hết 2×MSL
}

/// Chuyển trạng thái TCP — bảng này lấy thẳng từ RFC 793.
/// Trả về `None` nghĩa là sự kiện không hợp lệ ở trạng thái đó (gói bị bỏ).
pub fn chuyen_trang_thai(tt: TrangThaiTcp, sk: SuKienTcp) -> Option<TrangThaiTcp> {
    use SuKienTcp::*;
    use TrangThaiTcp::*;
    Some(match (tt, sk) {
        // --- Mở kết nối: bắt tay ba bước ---
        (Dong, MoChuDong)      => DaGuiSyn,      // gửi SYN
        (Dong, MoThuDong)      => Nghe,
        (Nghe, NhanSyn)        => DaNhanSyn,     // gửi SYN+ACK
        (DaGuiSyn, NhanSynAck) => DaThietLap,    // gửi ACK  ← bước 3
        (DaGuiSyn, NhanSyn)    => DaNhanSyn,     // mở đồng thời (hiếm)
        (DaNhanSyn, NhanAck)   => DaThietLap,

        // --- Đóng chủ động: bắt tay bốn bước ---
        (DaThietLap, UngDungDong) => ChoDong1,   // gửi FIN
        (ChoDong1, NhanAck)       => ChoDong2,
        (ChoDong2, NhanFin)       => ChoCuoi,    // gửi ACK
        (ChoDong1, NhanFin)       => ChoCuoi,    // đóng đồng thời
        (ChoCuoi, HetGio)         => Dong,       // sau 2×MSL

        // --- Đóng thụ động ---
        (DaThietLap, NhanFin)        => ChoDongThuDong, // gửi ACK
        (ChoDongThuDong, UngDungDong)=> ChoXacNhanCuoi, // gửi FIN
        (ChoXacNhanCuoi, NhanAck)    => Dong,
        _ => return None,
    })
}

/// Chạy một chuỗi sự kiện; trả về trạng thái cuối hoặc lỗi tại bước nào.
pub fn chay_phien(mut tt: TrangThaiTcp, cac_sk: &[SuKienTcp]) -> Result<TrangThaiTcp, (usize, TrangThaiTcp, SuKienTcp)> {
    for (i, &sk) in cac_sk.iter().enumerate() {
        tt = chuyen_trang_thai(tt, sk).ok_or((i, tt, sk))?;
    }
    Ok(tt)
}

// ============================================================================
// 3. ĐIỀU KHIỂN TẮC NGHẼN — vì sao Internet không sập
// ============================================================================

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum PhaTacNghen { KhoiDongCham, TranhTacNghen }

/// Mô phỏng TCP Reno: cửa sổ tắc nghẽn `cwnd` tính bằng số MSS.
#[derive(Debug, Clone)]
pub struct BoTacNghen {
    pub cwnd: f64,
    pub nguong: f64, // ssthresh
    pub pha: PhaTacNghen,
    pub lich_su: Vec<f64>,
}

impl BoTacNghen {
    pub fn moi(nguong_ban_dau: f64) -> Self {
        BoTacNghen { cwnd: 1.0, nguong: nguong_ban_dau,
                     pha: PhaTacNghen::KhoiDongCham, lich_su: vec![1.0] }
    }

    /// Nhận ACK: khởi động chậm nhân đôi mỗi RTT; tránh tắc nghẽn cộng 1 mỗi RTT.
    pub fn nhan_ack(&mut self) {
        match self.pha {
            PhaTacNghen::KhoiDongCham => {
                self.cwnd *= 2.0;               // TĂNG THEO CẤP SỐ NHÂN
                if self.cwnd >= self.nguong {
                    self.cwnd = self.nguong;
                    self.pha = PhaTacNghen::TranhTacNghen;
                }
            }
            PhaTacNghen::TranhTacNghen => self.cwnd += 1.0, // TĂNG TUYẾN TÍNH
        }
        self.lich_su.push(self.cwnd);
    }

    /// Mất gói phát hiện qua 3 ACK trùng: giảm một nửa (Fast Recovery).
    pub fn mat_goi_nhe(&mut self) {
        self.nguong = (self.cwnd / 2.0).max(2.0);
        self.cwnd = self.nguong;
        self.pha = PhaTacNghen::TranhTacNghen;
        self.lich_su.push(self.cwnd);
    }

    /// Hết giờ (timeout): mạng có thể đã sập — về vạch xuất phát.
    pub fn het_gio(&mut self) {
        self.nguong = (self.cwnd / 2.0).max(2.0);
        self.cwnd = 1.0;
        self.pha = PhaTacNghen::KhoiDongCham;
        self.lich_su.push(self.cwnd);
    }
}

// ============================================================================
// 4. TỔNG KIỂM TRA INTERNET (RFC 1071) — dùng trong IP, TCP, UDP, ICMP
// ============================================================================

/// Cộng bù-1 16-bit rồi lấy bù. Tính chất vàng: checksum của dữ liệu ĐÃ kèm
/// checksum luôn bằng 0 — máy nhận chỉ cần cộng hết và so với 0.
pub fn tong_kiem_tra(du_lieu: &[u8]) -> u16 {
    let mut tong: u32 = 0;
    let mut i = 0;
    while i + 1 < du_lieu.len() {
        tong += u16::from_be_bytes([du_lieu[i], du_lieu[i + 1]]) as u32;
        i += 2;
    }
    if i < du_lieu.len() {
        tong += (du_lieu[i] as u32) << 8; // byte lẻ được đệm 0 bên phải
    }
    while tong >> 16 != 0 {
        tong = (tong & 0xFFFF) + (tong >> 16); // gấp phần nhớ vòng lại
    }
    !(tong as u16)
}

pub fn kiem_tra_hop_le(du_lieu_kem_checksum: &[u8]) -> bool {
    tong_kiem_tra(du_lieu_kem_checksum) == 0
}

// ============================================================================
// 5. ĐỊA CHỈ IP & CIDR — chia mạng con
// ============================================================================

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct MangCon {
    pub dia_chi: u32,
    pub tien_to: u8, // /24, /16 ...
}

impl MangCon {
    pub fn phan_tich(s: &str) -> Option<MangCon> {
        let (ip, tt) = s.split_once('/')?;
        let o: Vec<u8> = ip.split('.').map(|x| x.parse().ok()).collect::<Option<_>>()?;
        if o.len() != 4 { return None; }
        let tien_to: u8 = tt.parse().ok()?;
        if tien_to > 32 { return None; }
        Some(MangCon {
            dia_chi: u32::from_be_bytes([o[0], o[1], o[2], o[3]]),
            tien_to,
        })
    }
    pub fn mat_na(&self) -> u32 {
        if self.tien_to == 0 { 0 } else { !0u32 << (32 - self.tien_to) }
    }
    pub fn dia_chi_mang(&self) -> u32 { self.dia_chi & self.mat_na() }
    pub fn quang_ba(&self) -> u32 { self.dia_chi_mang() | !self.mat_na() }
    /// Số máy chủ gán được = tổng địa chỉ - 2 (địa chỉ mạng + quảng bá).
    pub fn so_may_chu(&self) -> u64 {
        match self.tien_to {
            32 => 1, 31 => 2, // RFC 3021: liên kết điểm-điểm
            t => (1u64 << (32 - t)) - 2,
        }
    }
    pub fn chua(&self, ip: u32) -> bool { ip & self.mat_na() == self.dia_chi_mang() }
    pub fn hien_thi(ip: u32) -> String {
        let b = ip.to_be_bytes();
        format!("{}.{}.{}.{}", b[0], b[1], b[2], b[3])
    }
}

/// Định tuyến "khớp tiền tố dài nhất" — quy tắc CỐT LÕI của mọi bộ định tuyến.
pub fn chon_tuyen<'a>(bang: &'a [(MangCon, &'a str)], ip: u32) -> Option<&'a str> {
    bang.iter()
        .filter(|(m, _)| m.chua(ip))
        .max_by_key(|(m, _)| m.tien_to)   // tiền tố DÀI NHẤT thắng
        .map(|(_, cong)| *cong)
}

// ============================================================================
// 6. DNS — phân giải tên bằng đệ quy có bộ nhớ đệm
// ============================================================================

#[derive(Debug, Clone, PartialEq)]
pub enum BanGhi { A(String), CNAME(String), NS(String) }

pub struct MayChuDns {
    pub ban_ghi: Vec<(String, BanGhi)>,
}

impl MayChuDns {
    /// Phân giải, đi theo CNAME. Giới hạn số bước để chặn vòng lặp CNAME.
    pub fn phan_giai(&self, ten: &str) -> Result<String, String> {
        let mut hien_tai = ten.to_string();
        for _ in 0..8 {
            match self.ban_ghi.iter().find(|(n, _)| *n == hien_tai) {
                Some((_, BanGhi::A(ip))) => return Ok(ip.clone()),
                Some((_, BanGhi::CNAME(dich))) => hien_tai = dich.clone(),
                Some((_, BanGhi::NS(_))) => return Err(format!("cần hỏi máy chủ khác cho {hien_tai}")),
                None => return Err(format!("NXDOMAIN: không có bản ghi cho {hien_tai}")),
            }
        }
        Err("vượt quá 8 bước CNAME — nghi ngờ vòng lặp".into())
    }
}

// ============================================================================
// 7. CỬA SỔ TRƯỢT — truyền tin cậy trên kênh không tin cậy
// ============================================================================

#[derive(Debug, PartialEq)]
pub struct KetQuaTruyen {
    pub da_nhan: Vec<u32>,
    pub so_lan_gui: usize,
}

/// Go-Back-N: gửi tối đa `cua_so` gói chưa được xác nhận. Gói nào mất thì
/// gửi lại TỪ ĐÓ TRỞ ĐI — đơn giản nhưng lãng phí băng thông.
pub fn go_back_n(tong_goi: u32, cua_so: u32, mat_tai: &[u32]) -> KetQuaTruyen {
    let mut da_nhan = Vec::new();
    let mut so_lan_gui = 0;
    let mut co_so = 0u32;      // gói đầu tiên chưa được ACK
    let mut da_mat: VecDeque<u32> = mat_tai.iter().copied().collect();

    while co_so < tong_goi {
        let mut mat_trong_lot = None;
        for stt in co_so..(co_so + cua_so).min(tong_goi) {
            so_lan_gui += 1;
            if da_mat.front() == Some(&stt) {
                da_mat.pop_front();          // gói này mất, chỉ mất MỘT LẦN
                mat_trong_lot = Some(stt);
                break;                        // các gói sau sẽ bị bỏ (ngoài thứ tự)
            }
            da_nhan.push(stt);
        }
        co_so = match mat_trong_lot {
            Some(stt) => stt,                 // quay lại N — gửi lại từ gói mất
            None => (co_so + cua_so).min(tong_goi),
        };
    }
    KetQuaTruyen { da_nhan, so_lan_gui }
}

fn main() {
    println!("═══════════════════════════════════════════════════════════");
    println!("   MẠNG MÁY TÍNH: PHÂN TẦNG · TCP · TẮC NGHẼN · CIDR · DNS  ");
    println!("═══════════════════════════════════════════════════════════");

    println!("\n1. ĐÓNG GÓI THEO TẦNG — 5 byte dữ liệu đi hết chồng giao thức");
    let goi = dong_goi_xuong(b"hello");
    println!("   Gói cuối ở {} — tổng {} byte", goi.tang, goi.kich_thuoc());
    println!("   Chi phí phần đầu = {} byte cho 5 byte dữ liệu ({}% là bao bì)",
             goi.kich_thuoc() - 5, (goi.kich_thuoc() - 5) * 100 / goi.kich_thuoc());

    println!("\n2. BẮT TAY BA BƯỚC");
    use SuKienTcp::*;
    let kq = chay_phien(TrangThaiTcp::Dong, &[MoChuDong, NhanSynAck]);
    println!("   Máy khách: Dong -SYN-> DaGuiSyn -SYN/ACK-> {:?}", kq.unwrap());
    let kq = chay_phien(TrangThaiTcp::Dong, &[MoThuDong, NhanSyn, NhanAck]);
    println!("   Máy chủ  : Dong -listen-> Nghe -SYN-> DaNhanSyn -ACK-> {:?}", kq.unwrap());
    println!("   Sự kiện sai: {:?}", chay_phien(TrangThaiTcp::Dong, &[NhanAck]).unwrap_err());

    println!("\n3. ĐIỀU KHIỂN TẮC NGHẼN (TCP Reno)");
    let mut bt = BoTacNghen::moi(16.0);
    for _ in 0..5 { bt.nhan_ack(); }
    println!("   Khởi động chậm : {:?}", &bt.lich_su);
    bt.mat_goi_nhe();
    for _ in 0..3 { bt.nhan_ack(); }
    println!("   Sau mất gói nhẹ: {:?}", &bt.lich_su[5..]);
    bt.het_gio();
    println!("   Sau hết giờ    : cwnd = {} (về 1, quay lại khởi động chậm)", bt.cwnd);

    println!("\n4. TỔNG KIỂM TRA INTERNET");
    let than = [0x45u8, 0x00, 0x00, 0x3c, 0x1c, 0x46, 0x40, 0x00];
    let cs = tong_kiem_tra(&than);
    let mut kem = than.to_vec();
    kem.extend_from_slice(&cs.to_be_bytes());
    println!("   checksum = 0x{:04X} | gói kèm checksum hợp lệ: {}", cs, kiem_tra_hop_le(&kem));
    kem[0] ^= 0x01; // làm hỏng 1 bit
    println!("   sau khi lật 1 bit                         : {}", kiem_tra_hop_le(&kem));

    println!("\n5. CIDR & ĐỊNH TUYẾN KHỚP TIỀN TỐ DÀI NHẤT");
    let m = MangCon::phan_tich("192.168.10.130/26").unwrap();
    println!("   192.168.10.130/26 → mạng {} · quảng bá {} · {} máy chủ",
             MangCon::hien_thi(m.dia_chi_mang()), MangCon::hien_thi(m.quang_ba()), m.so_may_chu());
    let bang = [
        (MangCon::phan_tich("0.0.0.0/0").unwrap(), "cong-mac-dinh"),
        (MangCon::phan_tich("10.0.0.0/8").unwrap(), "eth0"),
        (MangCon::phan_tich("10.1.0.0/16").unwrap(), "eth1"),
        (MangCon::phan_tich("10.1.2.0/24").unwrap(), "eth2"),
    ];
    for ip in ["10.1.2.5", "10.1.9.9", "10.5.0.1", "8.8.8.8"] {
        let n = MangCon::phan_tich(&format!("{ip}/32")).unwrap().dia_chi;
        println!("   {:<12} → {}", ip, chon_tuyen(&bang, n).unwrap());
    }

    println!("\n6. DNS");
    let dns = MayChuDns { ban_ghi: vec![
        ("www.vidu.vn".into(), BanGhi::CNAME("may-chu.vidu.vn".into())),
        ("may-chu.vidu.vn".into(), BanGhi::A("203.0.113.7".into())),
    ]};
    println!("   www.vidu.vn  → {:?}", dns.phan_giai("www.vidu.vn"));
    println!("   khong-co.vn  → {:?}", dns.phan_giai("khong-co.vn"));

    println!("\n7. CỬA SỔ TRƯỢT GO-BACK-N (10 gói, cửa sổ 4, mất gói #2 và #6)");
    let kq = go_back_n(10, 4, &[2, 6]);
    println!("   Đã gửi {} lần cho 10 gói → hiệu suất {}%",
             kq.so_lan_gui, 10 * 100 / kq.so_lan_gui);

    println!("\n═══════════════════════════════════════════════════════════");
    println!("   GIAO THỨC = HỢP ĐỒNG GIỮA HAI MÁY KHÔNG TIN NHAU          ");
    println!("═══════════════════════════════════════════════════════════");
}

#[cfg(test)]
mod kiem_thu {
    use super::*;
    use SuKienTcp::*;
    use TrangThaiTcp::*;

    #[test]
    fn dong_goi_them_dung_54_byte_phan_dau() {
        let g = dong_goi_xuong(b"hello");
        // HTTP 18 + TCP 20 + IP 20 + Ethernet 14 = 72 byte header cho 5 byte dữ liệu
        assert_eq!(g.tang, Tang::LienKet);
        assert_eq!(g.kich_thuoc(), 18 + 20 + 20 + 14 + 5);
    }

    #[test]
    fn du_lieu_goc_van_nam_o_cuoi_sau_khi_boc() {
        let g = dong_goi_xuong(b"hello");
        let byte = g.tuan_tu_hoa();
        assert!(byte.ends_with(b"hello"), "tải trọng phải nguyên vẹn dưới đáy các header");
    }

    #[test]
    fn bat_tay_ba_buoc_phia_khach() {
        assert_eq!(chay_phien(Dong, &[MoChuDong, NhanSynAck]), Ok(DaThietLap));
    }

    #[test]
    fn bat_tay_ba_buoc_phia_chu() {
        assert_eq!(chay_phien(Dong, &[MoThuDong, NhanSyn, NhanAck]), Ok(DaThietLap));
    }

    #[test]
    fn dong_chu_dong_di_qua_time_wait() {
        let kq = chay_phien(Dong, &[MoChuDong, NhanSynAck, UngDungDong, NhanAck, NhanFin]);
        assert_eq!(kq, Ok(ChoCuoi), "phải dừng ở TIME_WAIT chứ không đóng ngay");
        assert_eq!(chuyen_trang_thai(ChoCuoi, HetGio), Some(Dong));
    }

    #[test]
    fn dong_thu_dong_di_qua_close_wait() {
        let kq = chay_phien(Dong, &[MoThuDong, NhanSyn, NhanAck, NhanFin, UngDungDong, NhanAck]);
        assert_eq!(kq, Ok(Dong));
    }

    #[test]
    fn goi_khong_hop_le_bi_tu_choi_dung_vi_tri() {
        // ACK tới khi chưa có kết nối nào -> sai ngay từ sự kiện thứ 0
        let e = chay_phien(Dong, &[NhanAck]).unwrap_err();
        assert_eq!(e, (0, Dong, NhanAck));
    }

    #[test]
    fn khoi_dong_cham_nhan_doi_roi_chuyen_pha_o_nguong() {
        let mut b = BoTacNghen::moi(16.0);
        for _ in 0..4 { b.nhan_ack(); }
        // 1 -> 2 -> 4 -> 8 -> 16: nhân đôi mỗi RTT, dừng nhân đúng tại ngưỡng
        assert_eq!(&b.lich_su[..], &[1.0, 2.0, 4.0, 8.0, 16.0]);
        assert_eq!(b.cwnd, 16.0);
        assert_eq!(b.pha, PhaTacNghen::TranhTacNghen, "chạm ngưỡng thì đổi pha");

        // Ngưỡng KHÔNG phải trần cứng: qua ngưỡng, cửa sổ vẫn lớn dần — nhưng
        // theo cấp số CỘNG. Đây chính là chữ "AI" trong AIMD.
        for _ in 0..6 { b.nhan_ack(); }
        assert_eq!(b.cwnd, 22.0, "16 + 6 lần cộng 1");
    }

    #[test]
    fn tranh_tac_nghen_tang_tuyen_tinh() {
        let mut b = BoTacNghen::moi(4.0);
        for _ in 0..2 { b.nhan_ack(); }        // 1 -> 2 -> 4 (chạm ngưỡng)
        let truoc = b.cwnd;
        b.nhan_ack();
        assert_eq!(b.cwnd, truoc + 1.0, "pha tránh tắc nghẽn cộng 1, không nhân 2");
    }

    #[test]
    fn het_gio_ve_mot_con_mat_goi_nhe_chi_giam_nua() {
        let mut a = BoTacNghen::moi(64.0);
        for _ in 0..5 { a.nhan_ack(); }        // cwnd = 32
        let mut b = a.clone();
        a.mat_goi_nhe();
        b.het_gio();
        assert_eq!(a.cwnd, 16.0, "3 ACK trùng: giảm một nửa");
        assert_eq!(b.cwnd, 1.0, "hết giờ: về vạch xuất phát");
        assert_eq!(b.pha, PhaTacNghen::KhoiDongCham);
    }

    #[test]
    fn checksum_cua_du_lieu_kem_checksum_bang_khong() {
        let than = [0x45u8, 0x00, 0x00, 0x3c, 0x1c, 0x46, 0x40, 0x00, 0x40, 0x06];
        let cs = tong_kiem_tra(&than);
        let mut kem = than.to_vec();
        kem.extend_from_slice(&cs.to_be_bytes());
        assert!(kiem_tra_hop_le(&kem), "tính chất vàng của tổng bù-1");
    }

    #[test]
    fn checksum_bat_duoc_loi_lat_mot_bit() {
        let than = [0x45u8, 0x00, 0x00, 0x3c, 0x1c, 0x46, 0x40, 0x00];
        let cs = tong_kiem_tra(&than);
        let mut hong = than.to_vec();
        hong.extend_from_slice(&cs.to_be_bytes());
        hong[3] ^= 0x08;
        assert!(!kiem_tra_hop_le(&hong));
    }

    #[test]
    fn checksum_khong_bat_duoc_loi_hoan_vi_hai_tu() {
        // Điểm YẾU đã biết: phép cộng có tính giao hoán nên đảo chỗ hai từ 16-bit
        // cho ra cùng checksum. Đây là lý do tầng ứng dụng vẫn cần CRC/hash mạnh.
        let a = [0x11u8, 0x22, 0x33, 0x44];
        let b = [0x33u8, 0x44, 0x11, 0x22];
        assert_eq!(tong_kiem_tra(&a), tong_kiem_tra(&b));
    }

    #[test]
    fn cidr_tinh_dung_mang_va_quang_ba() {
        let m = MangCon::phan_tich("192.168.10.130/26").unwrap();
        assert_eq!(MangCon::hien_thi(m.dia_chi_mang()), "192.168.10.128");
        assert_eq!(MangCon::hien_thi(m.quang_ba()), "192.168.10.191");
        assert_eq!(m.so_may_chu(), 62); // 2^6 - 2
    }

    #[test]
    fn cidr_truong_hop_bien() {
        assert_eq!(MangCon::phan_tich("10.0.0.1/32").unwrap().so_may_chu(), 1);
        assert_eq!(MangCon::phan_tich("10.0.0.0/31").unwrap().so_may_chu(), 2);
        assert_eq!(MangCon::phan_tich("10.0.0.0/24").unwrap().so_may_chu(), 254);
        assert_eq!(MangCon::phan_tich("0.0.0.0/0").unwrap().mat_na(), 0);
        assert!(MangCon::phan_tich("10.0.0.0/33").is_none());
    }

    #[test]
    fn dinh_tuyen_chon_tien_to_dai_nhat() {
        let bang = [
            (MangCon::phan_tich("0.0.0.0/0").unwrap(), "mac-dinh"),
            (MangCon::phan_tich("10.0.0.0/8").unwrap(), "eth0"),
            (MangCon::phan_tich("10.1.0.0/16").unwrap(), "eth1"),
            (MangCon::phan_tich("10.1.2.0/24").unwrap(), "eth2"),
        ];
        let ip = |s: &str| MangCon::phan_tich(&format!("{s}/32")).unwrap().dia_chi;
        assert_eq!(chon_tuyen(&bang, ip("10.1.2.5")), Some("eth2")); // /24 thắng /16 và /8
        assert_eq!(chon_tuyen(&bang, ip("10.1.9.9")), Some("eth1"));
        assert_eq!(chon_tuyen(&bang, ip("10.5.0.1")), Some("eth0"));
        assert_eq!(chon_tuyen(&bang, ip("8.8.8.8")), Some("mac-dinh"));
    }

    #[test]
    fn dns_di_theo_chuoi_cname() {
        let d = MayChuDns { ban_ghi: vec![
            ("a.vn".into(), BanGhi::CNAME("b.vn".into())),
            ("b.vn".into(), BanGhi::CNAME("c.vn".into())),
            ("c.vn".into(), BanGhi::A("1.2.3.4".into())),
        ]};
        assert_eq!(d.phan_giai("a.vn"), Ok("1.2.3.4".into()));
    }

    #[test]
    fn dns_chan_vong_lap_cname() {
        let d = MayChuDns { ban_ghi: vec![
            ("x.vn".into(), BanGhi::CNAME("y.vn".into())),
            ("y.vn".into(), BanGhi::CNAME("x.vn".into())),
        ]};
        assert!(d.phan_giai("x.vn").unwrap_err().contains("vòng lặp"));
    }

    #[test]
    fn dns_bao_nxdomain() {
        let d = MayChuDns { ban_ghi: vec![] };
        assert!(d.phan_giai("khong-ton-tai.vn").unwrap_err().contains("NXDOMAIN"));
    }

    #[test]
    fn go_back_n_giao_du_moi_goi_theo_thu_tu() {
        let kq = go_back_n(10, 4, &[2, 6]);
        assert_eq!(kq.da_nhan, (0..10).collect::<Vec<u32>>(), "phải giao đủ và đúng thứ tự");
    }

    #[test]
    fn go_back_n_ton_bang_thong_khi_mat_goi() {
        let sach = go_back_n(10, 4, &[]);
        let mat = go_back_n(10, 4, &[2, 6]);
        assert_eq!(sach.so_lan_gui, 10, "kênh sạch: mỗi gói gửi đúng 1 lần");
        assert!(mat.so_lan_gui > sach.so_lan_gui,
                "Go-Back-N gửi lại cả các gói KHÔNG mất — đó là cái giá của sự đơn giản");
    }
}
