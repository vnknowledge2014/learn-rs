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
    pub header: Vec<u8>,
    pub tai: Vec<u8>, // payload — chính là gói của tầng trên đã tuần tự hóa
}

impl GoiTin {
    pub fn serialize(&self) -> Vec<u8> {
        let mut v = self.header.clone();
        v.extend_from_slice(&self.tai);
        v
    }
    /// Bọc gói này vào một tầng thấp hơn.
    pub fn boc(self, tang_duoi: Tang, header: Vec<u8>) -> GoiTin {
        GoiTin { tang: tang_duoi, header, tai: self.serialize() }
    }
    /// Tổng chi phí phần đầu khi biết kích thước từng header đã dùng.
    pub fn size(&self) -> usize { self.header.len() + self.tai.len() }
}

/// Dựng chồng giao thức: dữ liệu ứng dụng đi xuống, mỗi tầng thêm header.
pub fn dong_goi_xuong(du_lieu_ung_dung: &[u8]) -> GoiTin {
    let http = GoiTin { tang: Tang::UngDung, header: b"GET / HTTP/1.1\r\n\r\n".to_vec(),
                        tai: du_lieu_ung_dung.to_vec() };
    let tcp = http.boc(Tang::GiaoVan, vec![0u8; 20]);   // TCP header tối thiểu 20 byte
    let ip = tcp.boc(Tang::Mang, vec![0u8; 20]);        // IPv4 header tối thiểu 20 byte
    ip.boc(Tang::LienKet, vec![0u8; 14])                // Ethernet header 14 byte
}

// ============================================================================
// 2. MÁY TRẠNG THÁI TCP — trái tim của độ tin cậy
// ============================================================================

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum TcpState {
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
pub enum TcpEvent {
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
pub fn transfer_state(tt: TcpState, sk: TcpEvent) -> Option<TcpState> {
    use TcpEvent::*;
    use TcpState::*;
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
pub fn run_session(mut tt: TcpState, cac_sk: &[TcpEvent]) -> Result<TcpState, (usize, TcpState, TcpEvent)> {
    for (i, &sk) in cac_sk.iter().enumerate() {
        tt = transfer_state(tt, sk).ok_or((i, tt, sk))?;
    }
    Ok(tt)
}

// ============================================================================
// 3. ĐIỀU KHIỂN TẮC NGHẼN — vì sao Internet không sập
// ============================================================================

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum CongestionPhase { KhoiDongCham, TranhTacNghen }

/// Mô phỏng TCP Reno: cửa sổ tắc nghẽn `cwnd` tính bằng số MSS.
#[derive(Debug, Clone)]
pub struct CongestionControl {
    pub cwnd: f64,
    pub threshold: f64, // ssthresh
    pub pha: CongestionPhase,
    pub history: Vec<f64>,
}

impl CongestionControl {
    pub fn new(nguong_ban_dau: f64) -> Self {
        CongestionControl { cwnd: 1.0, threshold: nguong_ban_dau,
                     pha: CongestionPhase::KhoiDongCham, history: vec![1.0] }
    }

    /// Nhận ACK: khởi động chậm nhân đôi mỗi RTT; tránh tắc nghẽn cộng 1 mỗi RTT.
    pub fn nhan_ack(&mut self) {
        match self.pha {
            CongestionPhase::KhoiDongCham => {
                self.cwnd *= 2.0;               // TĂNG THEO CẤP SỐ NHÂN
                if self.cwnd >= self.threshold {
                    self.cwnd = self.threshold;
                    self.pha = CongestionPhase::TranhTacNghen;
                }
            }
            CongestionPhase::TranhTacNghen => self.cwnd += 1.0, // TĂNG TUYẾN TÍNH
        }
        self.history.push(self.cwnd);
    }

    /// Mất gói phát hiện qua 3 ACK trùng: giảm một nửa (Fast Recovery).
    pub fn mat_call_light(&mut self) {
        self.threshold = (self.cwnd / 2.0).max(2.0);
        self.cwnd = self.threshold;
        self.pha = CongestionPhase::TranhTacNghen;
        self.history.push(self.cwnd);
    }

    /// Hết giờ (timeout): mạng có thể đã sập — về vạch xuất phát.
    pub fn het_gio(&mut self) {
        self.threshold = (self.cwnd / 2.0).max(2.0);
        self.cwnd = 1.0;
        self.pha = CongestionPhase::KhoiDongCham;
        self.history.push(self.cwnd);
    }
}

// ============================================================================
// 4. TỔNG KIỂM TRA INTERNET (RFC 1071) — dùng trong IP, TCP, UDP, ICMP
// ============================================================================

/// Cộng bù-1 16-bit rồi lấy bù. Tính chất vàng: checksum của dữ liệu ĐÃ kèm
/// checksum luôn bằng 0 — máy nhận chỉ cần cộng hết và so với 0.
pub fn total_check(data: &[u8]) -> u16 {
    let mut tong: u32 = 0;
    let mut i = 0;
    while i + 1 < data.len() {
        tong += u16::from_be_bytes([data[i], data[i + 1]]) as u32;
        i += 2;
    }
    if i < data.len() {
        tong += (data[i] as u32) << 8; // byte lẻ được đệm 0 bên phải
    }
    while tong >> 16 != 0 {
        tong = (tong & 0xFFFF) + (tong >> 16); // gấp phần nhớ vòng lại
    }
    !(tong as u16)
}

pub fn check_hop_le(du_lieu_kem_checksum: &[u8]) -> bool {
    total_check(du_lieu_kem_checksum) == 0
}

// ============================================================================
// 5. ĐỊA CHỈ IP & CIDR — chia mạng con
// ============================================================================

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct MangCon {
    pub address: u32,
    pub prefix: u8, // /24, /16 ...
}

impl MangCon {
    pub fn analyze(s: &str) -> Option<MangCon> {
        let (ip, tt) = s.split_once('/')?;
        let o: Vec<u8> = ip.split('.').map(|x| x.parse().ok()).collect::<Option<_>>()?;
        if o.len() != 4 { return None; }
        let prefix: u8 = tt.parse().ok()?;
        if prefix > 32 { return None; }
        Some(MangCon {
            address: u32::from_be_bytes([o[0], o[1], o[2], o[3]]),
            prefix,
        })
    }
    pub fn mat_na(&self) -> u32 {
        if self.prefix == 0 { 0 } else { !0u32 << (32 - self.prefix) }
    }
    pub fn address_array(&self) -> u32 { self.address & self.mat_na() }
    pub fn quang_ba(&self) -> u32 { self.address_array() | !self.mat_na() }
    /// Số máy chủ gán được = tổng địa chỉ - 2 (địa chỉ mạng + quảng bá).
    pub fn num_server(&self) -> u64 {
        match self.prefix {
            32 => 1, 31 => 2, // RFC 3021: liên kết điểm-điểm
            t => (1u64 << (32 - t)) - 2,
        }
    }
    pub fn contains(&self, ip: u32) -> bool { ip & self.mat_na() == self.address_array() }
    pub fn display(ip: u32) -> String {
        let b = ip.to_be_bytes();
        format!("{}.{}.{}.{}", b[0], b[1], b[2], b[3])
    }
}

/// Định tuyến "khớp tiền tố dài nhất" — quy tắc CỐT LÕI của mọi bộ định tuyến.
pub fn match_route<'a>(bang: &'a [(MangCon, &'a str)], ip: u32) -> Option<&'a str> {
    bang.iter()
        .filter(|(m, _)| m.contains(ip))
        .max_by_key(|(m, _)| m.prefix)   // tiền tố DÀI NHẤT thắng
        .map(|(_, gate)| *gate)
}

// ============================================================================
// 6. DNS — phân giải tên bằng đệ quy có bộ nhớ đệm
// ============================================================================

#[derive(Debug, Clone, PartialEq)]
pub enum SellRecord { A(String), CNAME(String), NS(String) }

pub struct DnsServer {
    pub sell_record: Vec<(String, SellRecord)>,
}

impl DnsServer {
    /// Phân giải, đi theo CNAME. Giới hạn số bước để chặn vòng lặp CNAME.
    pub fn part_solve(&self, name: &str) -> Result<String, String> {
        let mut current = name.to_string();
        for _ in 0..8 {
            match self.sell_record.iter().find(|(n, _)| *n == current) {
                Some((_, SellRecord::A(ip))) => return Ok(ip.clone()),
                Some((_, SellRecord::CNAME(dich))) => current = dich.clone(),
                Some((_, SellRecord::NS(_))) => return Err(format!("cần hỏi máy chủ khác cho {current}")),
                None => return Err(format!("NXDOMAIN: không có bản ghi cho {current}")),
            }
        }
        Err("vượt quá 8 bước CNAME — nghi ngờ vòng lặp".into())
    }
}

// ============================================================================
// 7. CỬA SỔ TRƯỢT — truyền tin cậy trên kênh không tin cậy
// ============================================================================

#[derive(Debug, PartialEq)]
pub struct TransferResult {
    pub da_nhan: Vec<u32>,
    pub count_send: usize,
}

/// Go-Back-N: gửi tối đa `window` gói chưa được xác nhận. Gói nào mất thì
/// gửi lại TỪ ĐÓ TRỞ ĐI — đơn giản nhưng lãng phí băng thông.
pub fn go_back_n(tong_goi: u32, window: u32, mat_tai: &[u32]) -> TransferResult {
    let mut da_nhan = Vec::new();
    let mut count_send = 0;
    let mut has_num = 0u32;      // gói đầu tiên chưa được ACK
    let mut da_mat: VecDeque<u32> = mat_tai.iter().copied().collect();

    while has_num < tong_goi {
        let mut lost_in_window = None;
        for stt in has_num..(has_num + window).min(tong_goi) {
            count_send += 1;
            if da_mat.front() == Some(&stt) {
                da_mat.pop_front();          // gói này mất, chỉ mất MỘT LẦN
                lost_in_window = Some(stt);
                break;                        // các gói sau sẽ bị bỏ (ngoài thứ tự)
            }
            da_nhan.push(stt);
        }
        has_num = match lost_in_window {
            Some(stt) => stt,                 // quay lại N — gửi lại từ gói mất
            None => (has_num + window).min(tong_goi),
        };
    }
    TransferResult { da_nhan, count_send }
}

fn main() {
    println!("═══════════════════════════════════════════════════════════");
    println!("   MẠNG MÁY TÍNH: PHÂN TẦNG · TCP · TẮC NGHẼN · CIDR · DNS  ");
    println!("═══════════════════════════════════════════════════════════");

    println!("\n1. ĐÓNG GÓI THEO TẦNG — 5 byte dữ liệu đi hết chồng giao thức");
    let goi = dong_goi_xuong(b"hello");
    println!("   Gói cuối ở {} — tổng {} byte", goi.tang, goi.size());
    println!("   Chi phí phần đầu = {} byte cho 5 byte dữ liệu ({}% là bao bì)",
             goi.size() - 5, (goi.size() - 5) * 100 / goi.size());

    println!("\n2. BẮT TAY BA BƯỚC");
    use TcpEvent::*;
    let kq = run_session(TcpState::Dong, &[MoChuDong, NhanSynAck]);
    println!("   Máy khách: Dong -SYN-> DaGuiSyn -SYN/ACK-> {:?}", kq.unwrap());
    let kq = run_session(TcpState::Dong, &[MoThuDong, NhanSyn, NhanAck]);
    println!("   Máy chủ  : Dong -listen-> Nghe -SYN-> DaNhanSyn -ACK-> {:?}", kq.unwrap());
    println!("   Sự kiện sai: {:?}", run_session(TcpState::Dong, &[NhanAck]).unwrap_err());

    println!("\n3. ĐIỀU KHIỂN TẮC NGHẼN (TCP Reno)");
    let mut bt = CongestionControl::new(16.0);
    for _ in 0..5 { bt.nhan_ack(); }
    println!("   Khởi động chậm : {:?}", &bt.history);
    bt.mat_call_light();
    for _ in 0..3 { bt.nhan_ack(); }
    println!("   Sau mất gói nhẹ: {:?}", &bt.history[5..]);
    bt.het_gio();
    println!("   Sau hết giờ    : cwnd = {} (về 1, quay lại khởi động chậm)", bt.cwnd);

    println!("\n4. TỔNG KIỂM TRA INTERNET");
    let than = [0x45u8, 0x00, 0x00, 0x3c, 0x1c, 0x46, 0x40, 0x00];
    let cs = total_check(&than);
    let mut kem = than.to_vec();
    kem.extend_from_slice(&cs.to_be_bytes());
    println!("   checksum = 0x{:04X} | gói kèm checksum hợp lệ: {}", cs, check_hop_le(&kem));
    kem[0] ^= 0x01; // làm hỏng 1 bit
    println!("   sau khi lật 1 bit                         : {}", check_hop_le(&kem));

    println!("\n5. CIDR & ĐỊNH TUYẾN KHỚP TIỀN TỐ DÀI NHẤT");
    let m = MangCon::analyze("192.168.10.130/26").unwrap();
    println!("   192.168.10.130/26 → mạng {} · quảng bá {} · {} máy chủ",
             MangCon::display(m.address_array()), MangCon::display(m.quang_ba()), m.num_server());
    let bang = [
        (MangCon::analyze("0.0.0.0/0").unwrap(), "cong-mac-dinh"),
        (MangCon::analyze("10.0.0.0/8").unwrap(), "eth0"),
        (MangCon::analyze("10.1.0.0/16").unwrap(), "eth1"),
        (MangCon::analyze("10.1.2.0/24").unwrap(), "eth2"),
    ];
    for ip in ["10.1.2.5", "10.1.9.9", "10.5.0.1", "8.8.8.8"] {
        let n = MangCon::analyze(&format!("{ip}/32")).unwrap().address;
        println!("   {:<12} → {}", ip, match_route(&bang, n).unwrap());
    }

    println!("\n6. DNS");
    let dns = DnsServer { sell_record: vec![
        ("www.vidu.vn".into(), SellRecord::CNAME("may-owner.vidu.vn".into())),
        ("may-owner.vidu.vn".into(), SellRecord::A("203.0.113.7".into())),
    ]};
    println!("   www.vidu.vn  → {:?}", dns.part_solve("www.vidu.vn"));
    println!("   khong-co.vn  → {:?}", dns.part_solve("khong-co.vn"));

    println!("\n7. CỬA SỔ TRƯỢT GO-BACK-N (10 gói, cửa sổ 4, mất gói #2 và #6)");
    let kq = go_back_n(10, 4, &[2, 6]);
    println!("   Đã gửi {} lần cho 10 gói → hiệu suất {}%",
             kq.count_send, 10 * 100 / kq.count_send);

    println!("\n═══════════════════════════════════════════════════════════");
    println!("   GIAO THỨC = HỢP ĐỒNG GIỮA HAI MÁY KHÔNG TIN NHAU          ");
    println!("═══════════════════════════════════════════════════════════");
}

#[cfg(test)]
mod tests {
    use super::*;
    use TcpEvent::*;
    use TcpState::*;

    #[test]
    fn encapsulation_adds_exactly_54_header_bytes() {
        let g = dong_goi_xuong(b"hello");
        // HTTP 18 + TCP 20 + IP 20 + Ethernet 14 = 72 byte header cho 5 byte dữ liệu
        assert_eq!(g.tang, Tang::LienKet);
        assert_eq!(g.size(), 18 + 20 + 20 + 14 + 5);
    }

    #[test]
    fn payload_survives_decapsulation() {
        let g = dong_goi_xuong(b"hello");
        let byte = g.serialize();
        assert!(byte.ends_with(b"hello"), "tải trọng phải nguyên vẹn dưới đáy các header");
    }

    #[test]
    fn three_way_handshake_client_side() {
        assert_eq!(run_session(Dong, &[MoChuDong, NhanSynAck]), Ok(DaThietLap));
    }

    #[test]
    fn three_way_handshake_server_side() {
        assert_eq!(run_session(Dong, &[MoThuDong, NhanSyn, NhanAck]), Ok(DaThietLap));
    }

    #[test]
    fn active_close_passes_through_time_wait() {
        let kq = run_session(Dong, &[MoChuDong, NhanSynAck, UngDungDong, NhanAck, NhanFin]);
        assert_eq!(kq, Ok(ChoCuoi), "phải dừng ở TIME_WAIT chứ không đóng ngay");
        assert_eq!(transfer_state(ChoCuoi, HetGio), Some(Dong));
    }

    #[test]
    fn passive_close_passes_through_close_wait() {
        let kq = run_session(Dong, &[MoThuDong, NhanSyn, NhanAck, NhanFin, UngDungDong, NhanAck]);
        assert_eq!(kq, Ok(Dong));
    }

    #[test]
    fn call_no_hop_le_is_reject_use_pos_value() {
        // ACK tới khi chưa có kết nối nào -> sai ngay từ sự kiện thứ 0
        let e = run_session(Dong, &[NhanAck]).unwrap_err();
        assert_eq!(e, (0, Dong, NhanAck));
    }

    #[test]
    fn slow_start_doubles_then_switches_at_threshold() {
        let mut b = CongestionControl::new(16.0);
        for _ in 0..4 { b.nhan_ack(); }
        // 1 -> 2 -> 4 -> 8 -> 16: nhân đôi mỗi RTT, dừng nhân đúng tại ngưỡng
        assert_eq!(&b.history[..], &[1.0, 2.0, 4.0, 8.0, 16.0]);
        assert_eq!(b.cwnd, 16.0);
        assert_eq!(b.pha, CongestionPhase::TranhTacNghen, "chạm ngưỡng thì đổi pha");

        // Ngưỡng KHÔNG phải trần cứng: qua ngưỡng, cửa sổ vẫn lớn dần — nhưng
        // theo cấp số CỘNG. Đây chính là chữ "AI" trong AIMD.
        for _ in 0..6 { b.nhan_ack(); }
        assert_eq!(b.cwnd, 22.0, "16 + 6 lần cộng 1");
    }

    #[test]
    fn congestion_avoidance_grows_linearly() {
        let mut b = CongestionControl::new(4.0);
        for _ in 0..2 { b.nhan_ack(); }        // 1 -> 2 -> 4 (chạm ngưỡng)
        let prev = b.cwnd;
        b.nhan_ack();
        assert_eq!(b.cwnd, prev + 1.0, "pha tránh tắc nghẽn cộng 1, không nhân 2");
    }

    #[test]
    fn timeout_resets_to_one_while_light_loss_halves() {
        let mut a = CongestionControl::new(64.0);
        for _ in 0..5 { a.nhan_ack(); }        // cwnd = 32
        let mut b = a.clone();
        a.mat_call_light();
        b.het_gio();
        assert_eq!(a.cwnd, 16.0, "3 ACK trùng: giảm một nửa");
        assert_eq!(b.cwnd, 1.0, "hết giờ: về vạch xuất phát");
        assert_eq!(b.pha, CongestionPhase::KhoiDongCham);
    }

    #[test]
    fn checksum_over_data_plus_checksum_is_zero() {
        let than = [0x45u8, 0x00, 0x00, 0x3c, 0x1c, 0x46, 0x40, 0x00, 0x40, 0x06];
        let cs = total_check(&than);
        let mut kem = than.to_vec();
        kem.extend_from_slice(&cs.to_be_bytes());
        assert!(check_hop_le(&kem), "tính chất vàng của tổng bù-1");
    }

    #[test]
    fn checksum_catches_single_bit_flip() {
        let than = [0x45u8, 0x00, 0x00, 0x3c, 0x1c, 0x46, 0x40, 0x00];
        let cs = total_check(&than);
        let mut hong = than.to_vec();
        hong.extend_from_slice(&cs.to_be_bytes());
        hong[3] ^= 0x08;
        assert!(!check_hop_le(&hong));
    }

    #[test]
    fn checksum_misses_word_transposition() {
        // Điểm YẾU đã biết: phép cộng có tính giao hoán nên đảo chỗ hai từ 16-bit
        // cho ra cùng checksum. Đây là lý do tầng ứng dụng vẫn cần CRC/hash mạnh.
        let a = [0x11u8, 0x22, 0x33, 0x44];
        let b = [0x33u8, 0x44, 0x11, 0x22];
        assert_eq!(total_check(&a), total_check(&b));
    }

    #[test]
    fn cidr_computes_network_and_broadcast() {
        let m = MangCon::analyze("192.168.10.130/26").unwrap();
        assert_eq!(MangCon::display(m.address_array()), "192.168.10.128");
        assert_eq!(MangCon::display(m.quang_ba()), "192.168.10.191");
        assert_eq!(m.num_server(), 62); // 2^6 - 2
    }

    #[test]
    fn cidr_edge_cases() {
        assert_eq!(MangCon::analyze("10.0.0.1/32").unwrap().num_server(), 1);
        assert_eq!(MangCon::analyze("10.0.0.0/31").unwrap().num_server(), 2);
        assert_eq!(MangCon::analyze("10.0.0.0/24").unwrap().num_server(), 254);
        assert_eq!(MangCon::analyze("0.0.0.0/0").unwrap().mat_na(), 0);
        assert!(MangCon::analyze("10.0.0.0/33").is_none());
    }

    #[test]
    fn routing_picks_longest_prefix() {
        let bang = [
            (MangCon::analyze("0.0.0.0/0").unwrap(), "mac-dinh"),
            (MangCon::analyze("10.0.0.0/8").unwrap(), "eth0"),
            (MangCon::analyze("10.1.0.0/16").unwrap(), "eth1"),
            (MangCon::analyze("10.1.2.0/24").unwrap(), "eth2"),
        ];
        let ip = |s: &str| MangCon::analyze(&format!("{s}/32")).unwrap().address;
        assert_eq!(match_route(&bang, ip("10.1.2.5")), Some("eth2")); // /24 thắng /16 và /8
        assert_eq!(match_route(&bang, ip("10.1.9.9")), Some("eth1"));
        assert_eq!(match_route(&bang, ip("10.5.0.1")), Some("eth0"));
        assert_eq!(match_route(&bang, ip("8.8.8.8")), Some("mac-dinh"));
    }

    #[test]
    fn dns_follows_cname_chain() {
        let d = DnsServer { sell_record: vec![
            ("a.vn".into(), SellRecord::CNAME("b.vn".into())),
            ("b.vn".into(), SellRecord::CNAME("c.vn".into())),
            ("c.vn".into(), SellRecord::A("1.2.3.4".into())),
        ]};
        assert_eq!(d.part_solve("a.vn"), Ok("1.2.3.4".into()));
    }

    #[test]
    fn dns_breaks_cname_loops() {
        let d = DnsServer { sell_record: vec![
            ("x.vn".into(), SellRecord::CNAME("y.vn".into())),
            ("y.vn".into(), SellRecord::CNAME("x.vn".into())),
        ]};
        assert!(d.part_solve("x.vn").unwrap_err().contains("vòng lặp"));
    }

    #[test]
    fn dns_reports_nxdomain() {
        let d = DnsServer { sell_record: vec![] };
        assert!(d.part_solve("khong-ton-tai.vn").unwrap_err().contains("NXDOMAIN"));
    }

    #[test]
    fn go_back_n_delivers_every_packet_in_order() {
        let kq = go_back_n(10, 4, &[2, 6]);
        assert_eq!(kq.da_nhan, (0..10).collect::<Vec<u32>>(), "phải giao đủ và đúng thứ tự");
    }

    #[test]
    fn go_back_n_wastes_bandwidth_on_loss() {
        let clean = go_back_n(10, 4, &[]);
        let mat = go_back_n(10, 4, &[2, 6]);
        assert_eq!(clean.count_send, 10, "kênh sạch: mỗi gói gửi đúng 1 lần");
        assert!(mat.count_send > clean.count_send,
                "Go-Back-N gửi lại cả các gói KHÔNG mất — đó là cái giá của sự đơn giản");
    }
}
