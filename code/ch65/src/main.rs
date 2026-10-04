//! Chương 65 — Mạng máy tính & Giao thức: đóng gói theo tầng, máy trạng thái TCP,
//! điều khiển tắc nghẽn, tổng kiểm tra Internet, CIDR, và bản ghi DNS.

use std::collections::VecDeque;
use std::fmt;

// ============================================================================
// 1. MÔ HÌNH PHÂN TẦNG & SỰ ĐÓNG GÓI (encapsulation)
// ============================================================================

#[derive(Debug, Clone, Copy, PartialEq, Eq, PartialOrd, Ord)]
pub enum Layer {
    Physical = 1,
    DataLink = 2,    // Ethernet — địa chỉ MAC, trong một mạng LAN
    Network = 3,     // IP — địa chỉ IP, định tuyến giữa các mạng
    Transport = 4,   // TCP/UDP — cổng, tin cậy
    Application = 7, // HTTP/DNS — ý nghĩa dữ liệu
}

impl fmt::Display for Layer {
    fn fmt(&self, f: &mut fmt::Formatter) -> fmt::Result {
        let t = match self {
            Layer::Physical => "Vật lý",
            Layer::DataLink => "Liên kết",
            Layer::Network => "Mạng",
            Layer::Transport => "Giao vận",
            Layer::Application => "Ứng dụng",
        };
        write!(f, "L{} {}", *self as u8, t)
    }
}

/// Mỗi tầng BỌC dữ liệu của tầng trên bằng phần đầu (header) của mình.
/// Giống gửi thư: thư → phong bì → túi bưu chính → xe tải.
#[derive(Debug, Clone, PartialEq)]
pub struct Packet {
    pub layer: Layer,
    pub header: Vec<u8>,
    pub payload: Vec<u8>, // payload — chính là gói của tầng trên đã tuần tự hóa
}

impl Packet {
    pub fn serialize(&self) -> Vec<u8> {
        let mut v = self.header.clone();
        v.extend_from_slice(&self.payload);
        v
    }
    /// Bọc gói này vào một tầng thấp hơn.
    pub fn wrap(self, lower: Layer, header: Vec<u8>) -> Packet {
        Packet {
            layer: lower,
            header,
            payload: self.serialize(),
        }
    }
    /// Tổng số byte trên dây: header của tầng này + mọi thứ nó bọc bên trong.
    pub fn size(&self) -> usize {
        self.header.len() + self.payload.len()
    }
}

/// Dựng chồng giao thức: dữ liệu ứng dụng đi xuống, mỗi tầng thêm header.
pub fn encapsulate(app_data: &[u8]) -> Packet {
    let http = Packet {
        layer: Layer::Application,
        header: b"GET / HTTP/1.1\r\n\r\n".to_vec(),
        payload: app_data.to_vec(),
    };
    let tcp = http.wrap(Layer::Transport, vec![0u8; 20]); // TCP header tối thiểu 20 byte
    let ip = tcp.wrap(Layer::Network, vec![0u8; 20]); // IPv4 header tối thiểu 20 byte
    ip.wrap(Layer::DataLink, vec![0u8; 14]) // Ethernet header 14 byte (chưa tính FCS 4 byte)
}

// ============================================================================
// 2. MÁY TRẠNG THÁI TCP — trái tim của độ tin cậy
// ============================================================================

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum TcpState {
    Closed,      // CLOSED
    Listen,      // LISTEN
    SynSent,     // SYN_SENT
    SynReceived, // SYN_RECEIVED
    Established, // ESTABLISHED
    FinWait1,    // FIN_WAIT_1
    FinWait2,    // FIN_WAIT_2
    Closing,     // CLOSING — hai bên cùng gửi FIN một lúc
    TimeWait,    // TIME_WAIT — chờ 2×MSL để gói lạc đường chết hẳn
    CloseWait,   // CLOSE_WAIT
    LastAck,     // LAST_ACK
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum TcpEvent {
    ActiveOpen,  // ứng dụng gọi connect()
    PassiveOpen, // ứng dụng gọi listen()
    RecvSyn,
    RecvSynAck,
    RecvAck,
    RecvFin,
    AppClose, // ứng dụng gọi close()
    Timeout,  // hết 2×MSL
}

/// Chuyển trạng thái TCP — theo sơ đồ trạng thái của RFC 793 (nay là RFC 9293).
/// Trả về `None` nghĩa là sự kiện không hợp lệ ở trạng thái đó (gói bị bỏ).
pub fn transition(state: TcpState, event: TcpEvent) -> Option<TcpState> {
    use TcpEvent::*;
    use TcpState::*;
    Some(match (state, event) {
        // --- Mở kết nối: bắt tay ba bước ---
        (Closed, ActiveOpen) => SynSent, // gửi SYN
        (Closed, PassiveOpen) => Listen,
        (Listen, RecvSyn) => SynReceived,     // gửi SYN+ACK
        (SynSent, RecvSynAck) => Established, // gửi ACK  ← bước 3
        (SynSent, RecvSyn) => SynReceived,    // mở đồng thời (hiếm)
        (SynReceived, RecvAck) => Established,

        // --- Đóng chủ động: bắt tay bốn bước ---
        (Established, AppClose) => FinWait1, // gửi FIN
        (FinWait1, RecvAck) => FinWait2,
        (FinWait2, RecvFin) => TimeWait, // gửi ACK
        (FinWait1, RecvFin) => Closing,  // đóng đồng thời: gửi ACK, chờ ACK cho FIN của ta
        (Closing, RecvAck) => TimeWait,
        (TimeWait, Timeout) => Closed, // sau 2×MSL

        // --- Đóng thụ động ---
        (Established, RecvFin) => CloseWait, // gửi ACK
        (CloseWait, AppClose) => LastAck,    // gửi FIN
        (LastAck, RecvAck) => Closed,
        _ => return None,
    })
}

/// Chạy một chuỗi sự kiện; trả về trạng thái cuối hoặc lỗi tại bước nào.
pub fn run_session(
    mut state: TcpState,
    events: &[TcpEvent],
) -> Result<TcpState, (usize, TcpState, TcpEvent)> {
    for (i, &event) in events.iter().enumerate() {
        state = transition(state, event).ok_or((i, state, event))?;
    }
    Ok(state)
}

// ============================================================================
// 3. ĐIỀU KHIỂN TẮC NGHẼN — vì sao Internet không sập
// ============================================================================

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum CongestionPhase {
    SlowStart,
    CongestionAvoidance,
}

/// Mô phỏng TCP Reno: cửa sổ tắc nghẽn `cwnd` tính bằng số MSS.
#[derive(Debug, Clone)]
pub struct CongestionControl {
    pub cwnd: f64,
    pub ssthresh: f64, // ngưỡng khởi động chậm (slow start threshold)
    pub phase: CongestionPhase,
    pub history: Vec<f64>,
}

impl CongestionControl {
    pub fn new(initial_ssthresh: f64) -> Self {
        CongestionControl {
            cwnd: 1.0,
            ssthresh: initial_ssthresh,
            phase: CongestionPhase::SlowStart,
            history: vec![1.0],
        }
    }

    /// Nhận ACK: khởi động chậm nhân đôi mỗi RTT; tránh tắc nghẽn cộng 1 mỗi RTT.
    pub fn on_ack(&mut self) {
        match self.phase {
            CongestionPhase::SlowStart => {
                self.cwnd *= 2.0; // TĂNG THEO CẤP SỐ NHÂN
                if self.cwnd >= self.ssthresh {
                    self.cwnd = self.ssthresh;
                    self.phase = CongestionPhase::CongestionAvoidance;
                }
            }
            CongestionPhase::CongestionAvoidance => self.cwnd += 1.0, // TĂNG TUYẾN TÍNH
        }
        self.history.push(self.cwnd);
    }

    /// Mất gói phát hiện qua 3 ACK trùng: giảm một nửa (Fast Recovery).
    pub fn on_triple_dup_ack(&mut self) {
        self.ssthresh = (self.cwnd / 2.0).max(2.0);
        self.cwnd = self.ssthresh;
        self.phase = CongestionPhase::CongestionAvoidance;
        self.history.push(self.cwnd);
    }

    /// Hết giờ (timeout): mạng có thể đã sập — về vạch xuất phát.
    pub fn on_timeout(&mut self) {
        self.ssthresh = (self.cwnd / 2.0).max(2.0);
        self.cwnd = 1.0;
        self.phase = CongestionPhase::SlowStart;
        self.history.push(self.cwnd);
    }
}

// ============================================================================
// 4. TỔNG KIỂM TRA INTERNET (RFC 1071) — dùng trong IP, TCP, UDP, ICMP
// ============================================================================

/// Cộng bù-1 16-bit rồi lấy bù. Tính chất vàng: checksum của dữ liệu ĐÃ kèm
/// checksum luôn bằng 0 — máy nhận chỉ cần cộng hết và so với 0.
pub fn checksum(data: &[u8]) -> u16 {
    let mut sum: u32 = 0;
    let mut i = 0;
    while i + 1 < data.len() {
        sum += u16::from_be_bytes([data[i], data[i + 1]]) as u32;
        i += 2;
    }
    if i < data.len() {
        sum += (data[i] as u32) << 8; // byte lẻ được đệm 0 bên phải
    }
    while sum >> 16 != 0 {
        sum = (sum & 0xFFFF) + (sum >> 16); // gấp phần nhớ vòng lại
    }
    !(sum as u16)
}

pub fn verify_checksum(data_with_checksum: &[u8]) -> bool {
    checksum(data_with_checksum) == 0
}

// ============================================================================
// 5. ĐỊA CHỈ IP & CIDR — chia mạng con
// ============================================================================

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct Subnet {
    pub address: u32,
    pub prefix: u8, // /24, /16 ...
}

impl Subnet {
    pub fn parse(s: &str) -> Option<Subnet> {
        let (ip, len) = s.split_once('/')?;
        let octets: Vec<u8> = ip
            .split('.')
            .map(|x| x.parse().ok())
            .collect::<Option<_>>()?;
        if octets.len() != 4 {
            return None;
        }
        let prefix: u8 = len.parse().ok()?;
        if prefix > 32 {
            return None;
        }
        Some(Subnet {
            address: u32::from_be_bytes([octets[0], octets[1], octets[2], octets[3]]),
            prefix,
        })
    }
    pub fn mask(&self) -> u32 {
        if self.prefix == 0 {
            0
        } else {
            !0u32 << (32 - self.prefix)
        }
    }
    pub fn network_address(&self) -> u32 {
        self.address & self.mask()
    }
    pub fn broadcast(&self) -> u32 {
        self.network_address() | !self.mask()
    }
    /// Số địa chỉ máy gán được = tổng địa chỉ - 2 (địa chỉ mạng + quảng bá).
    pub fn usable_hosts(&self) -> u64 {
        match self.prefix {
            32 => 1,
            31 => 2, // RFC 3021: liên kết điểm-điểm
            t => (1u64 << (32 - t)) - 2,
        }
    }
    pub fn contains(&self, ip: u32) -> bool {
        ip & self.mask() == self.network_address()
    }
    pub fn display(ip: u32) -> String {
        let b = ip.to_be_bytes();
        format!("{}.{}.{}.{}", b[0], b[1], b[2], b[3])
    }
}

/// Định tuyến "khớp tiền tố dài nhất" — quy tắc CỐT LÕI của mọi bộ định tuyến.
pub fn match_route<'a>(table: &'a [(Subnet, &'a str)], ip: u32) -> Option<&'a str> {
    table
        .iter()
        .filter(|(m, _)| m.contains(ip))
        .max_by_key(|(m, _)| m.prefix) // tiền tố DÀI NHẤT thắng
        .map(|(_, iface)| *iface)
}

// ============================================================================
// 6. DNS — phân giải tên, đi theo chuỗi CNAME
// ============================================================================

#[derive(Debug, Clone, PartialEq)]
pub enum DnsRecord {
    A(String),
    CNAME(String),
    NS(String),
}

pub struct DnsServer {
    pub records: Vec<(String, DnsRecord)>,
}

impl DnsServer {
    /// Phân giải, đi theo CNAME. Giới hạn số bước để chặn vòng lặp CNAME.
    pub fn resolve(&self, name: &str) -> Result<String, String> {
        let mut current = name.to_string();
        for _ in 0..8 {
            match self.records.iter().find(|(n, _)| *n == current) {
                Some((_, DnsRecord::A(ip))) => return Ok(ip.clone()),
                Some((_, DnsRecord::CNAME(target))) => current = target.clone(),
                Some((_, DnsRecord::NS(_))) => {
                    return Err(format!("cần hỏi máy chủ khác cho {current}"));
                }
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
    pub delivered: Vec<u32>, // các gói máy nhận chấp nhận, theo thứ tự
    pub send_count: usize,
}

/// Go-Back-N (mô hình theo lượt): mỗi lượt gửi CẢ cửa sổ `window` gói.
/// Máy nhận chỉ nhận gói ĐÚNG THỨ TỰ: gặp một gói mất thì bỏ mọi gói sau nó
/// trong lượt đó, và máy gửi phải gửi lại TỪ gói mất TRỞ ĐI — đơn giản nhưng
/// lãng phí băng thông. `lost` liệt kê các gói bị mất ở lần gửi ĐẦU TIÊN.
pub fn go_back_n(total: u32, window: u32, lost: &[u32]) -> TransferResult {
    let mut delivered = Vec::new();
    let mut send_count = 0;
    let mut base = 0u32; // gói đầu tiên chưa được ACK
    let mut pending_losses: VecDeque<u32> = lost.iter().copied().collect();

    while base < total {
        let mut first_lost = None;
        for seq in base..(base + window).min(total) {
            send_count += 1; // gói nào trong cửa sổ cũng được phát lên dây
            if first_lost.is_some() {
                continue; // tới nơi nhưng NGOÀI THỨ TỰ -> máy nhận vứt bỏ
            }
            if pending_losses.front() == Some(&seq) {
                pending_losses.pop_front(); // gói này mất, chỉ mất MỘT LẦN
                first_lost = Some(seq);
                continue;
            }
            delivered.push(seq);
        }
        base = match first_lost {
            Some(seq) => seq, // quay lại N — gửi lại từ gói mất
            None => (base + window).min(total),
        };
    }
    TransferResult {
        delivered,
        send_count,
    }
}

fn main() {
    println!("═══════════════════════════════════════════════════════════");
    println!("   MẠNG MÁY TÍNH: PHÂN TẦNG · TCP · TẮC NGHẼN · CIDR · DNS  ");
    println!("═══════════════════════════════════════════════════════════");

    println!("\n1. ĐÓNG GÓI THEO TẦNG — 5 byte dữ liệu đi hết chồng giao thức");
    let frame = encapsulate(b"hello");
    println!("   Gói cuối ở {} — tổng {} byte", frame.layer, frame.size());
    println!(
        "   Chi phí phần đầu = {} byte cho 5 byte dữ liệu ({}% là bao bì)",
        frame.size() - 5,
        (frame.size() - 5) * 100 / frame.size()
    );

    println!("\n2. BẮT TAY BA BƯỚC");
    use TcpEvent::*;
    let result = run_session(TcpState::Closed, &[ActiveOpen, RecvSynAck]);
    println!(
        "   Máy khách: Closed -SYN-> SynSent -SYN/ACK-> {:?}",
        result.unwrap()
    );
    let result = run_session(TcpState::Closed, &[PassiveOpen, RecvSyn, RecvAck]);
    println!(
        "   Máy chủ  : Closed -listen-> Listen -SYN-> SynReceived -ACK-> {:?}",
        result.unwrap()
    );
    println!(
        "   Sự kiện sai: {:?}",
        run_session(TcpState::Closed, &[RecvAck]).unwrap_err()
    );

    println!("\n3. ĐIỀU KHIỂN TẮC NGHẼN (TCP Reno)");
    let mut cc = CongestionControl::new(16.0);
    for _ in 0..5 {
        cc.on_ack();
    }
    println!("   Khởi động chậm : {:?}", cc.history);
    cc.on_triple_dup_ack();
    for _ in 0..3 {
        cc.on_ack();
    }
    println!("   Sau mất gói nhẹ: {:?}", &cc.history[5..]);
    cc.on_timeout();
    println!(
        "   Sau hết giờ    : cwnd = {} (về 1, quay lại khởi động chậm)",
        cc.cwnd
    );

    println!("\n4. TỔNG KIỂM TRA INTERNET");
    let data = [0x45u8, 0x00, 0x00, 0x3c, 0x1c, 0x46, 0x40, 0x00];
    let cs = checksum(&data);
    let mut with_cs = data.to_vec();
    with_cs.extend_from_slice(&cs.to_be_bytes());
    println!(
        "   checksum = 0x{:04X} | gói kèm checksum hợp lệ: {}",
        cs,
        verify_checksum(&with_cs)
    );
    with_cs[0] ^= 0x01; // làm hỏng 1 bit
    println!(
        "   sau khi lật 1 bit                         : {}",
        verify_checksum(&with_cs)
    );

    println!("\n5. CIDR & ĐỊNH TUYẾN KHỚP TIỀN TỐ DÀI NHẤT");
    let m = Subnet::parse("192.168.10.130/26").unwrap();
    println!(
        "   192.168.10.130/26 → mạng {} · quảng bá {} · {} địa chỉ máy dùng được",
        Subnet::display(m.network_address()),
        Subnet::display(m.broadcast()),
        m.usable_hosts()
    );
    let table = [
        (Subnet::parse("0.0.0.0/0").unwrap(), "default-gw"),
        (Subnet::parse("10.0.0.0/8").unwrap(), "eth0"),
        (Subnet::parse("10.1.0.0/16").unwrap(), "eth1"),
        (Subnet::parse("10.1.2.0/24").unwrap(), "eth2"),
    ];
    for ip in ["10.1.2.5", "10.1.9.9", "10.5.0.1", "8.8.8.8"] {
        let n = Subnet::parse(&format!("{ip}/32")).unwrap().address;
        println!("   {:<12} → {}", ip, match_route(&table, n).unwrap());
    }

    println!("\n6. DNS");
    let dns = DnsServer {
        records: vec![
            (
                "www.example.vn".into(),
                DnsRecord::CNAME("server.example.vn".into()),
            ),
            (
                "server.example.vn".into(),
                DnsRecord::A("203.0.113.7".into()),
            ),
        ],
    };
    println!("   www.example.vn → {:?}", dns.resolve("www.example.vn"));
    println!("   missing.vn     → {:?}", dns.resolve("missing.vn"));

    println!("\n7. CỬA SỔ TRƯỢT GO-BACK-N (10 gói, cửa sổ 4, mất gói #2 và #6)");
    let result = go_back_n(10, 4, &[2, 6]);
    println!(
        "   Đã gửi {} lần cho 10 gói → hiệu suất {}%",
        result.send_count,
        10 * 100 / result.send_count
    );

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
    fn encapsulation_adds_72_header_bytes() {
        let g = encapsulate(b"hello");
        // HTTP 18 + TCP 20 + IP 20 + Ethernet 14 = 72 byte header cho 5 byte dữ liệu
        assert_eq!(g.layer, Layer::DataLink);
        assert_eq!(g.size(), 18 + 20 + 20 + 14 + 5);
    }

    #[test]
    fn payload_survives_decapsulation() {
        let g = encapsulate(b"hello");
        let bytes = g.serialize();
        assert!(
            bytes.ends_with(b"hello"),
            "tải trọng phải nguyên vẹn dưới đáy các header"
        );
    }

    #[test]
    fn three_way_handshake_client_side() {
        assert_eq!(
            run_session(Closed, &[ActiveOpen, RecvSynAck]),
            Ok(Established)
        );
    }

    #[test]
    fn three_way_handshake_server_side() {
        assert_eq!(
            run_session(Closed, &[PassiveOpen, RecvSyn, RecvAck]),
            Ok(Established)
        );
    }

    #[test]
    fn active_close_passes_through_time_wait() {
        let result = run_session(
            Closed,
            &[ActiveOpen, RecvSynAck, AppClose, RecvAck, RecvFin],
        );
        assert_eq!(
            result,
            Ok(TimeWait),
            "phải dừng ở TIME_WAIT chứ không đóng ngay"
        );
        assert_eq!(transition(TimeWait, Timeout), Some(Closed));
    }

    #[test]
    fn passive_close_passes_through_close_wait() {
        let result = run_session(Closed, &[PassiveOpen, RecvSyn, RecvAck, RecvFin, AppClose]);
        assert_eq!(result, Ok(LastAck), "bên đóng thụ động KHÔNG qua TIME_WAIT");
        assert_eq!(transition(LastAck, RecvAck), Some(Closed));
    }

    #[test]
    fn simultaneous_close_goes_through_closing() {
        let result = run_session(Established, &[AppClose, RecvFin, RecvAck]);
        assert_eq!(result, Ok(TimeWait));
        assert_eq!(transition(FinWait1, RecvFin), Some(Closing));
    }

    #[test]
    fn invalid_event_reports_its_position() {
        // ACK tới khi chưa có kết nối nào -> sai ngay từ sự kiện thứ 0
        let e = run_session(Closed, &[RecvAck]).unwrap_err();
        assert_eq!(e, (0, Closed, RecvAck));
    }

    #[test]
    fn slow_start_doubles_then_switches_at_threshold() {
        let mut b = CongestionControl::new(16.0);
        for _ in 0..4 {
            b.on_ack();
        }
        // 1 -> 2 -> 4 -> 8 -> 16: nhân đôi mỗi RTT, dừng nhân đúng tại ngưỡng
        assert_eq!(&b.history[..], &[1.0, 2.0, 4.0, 8.0, 16.0]);
        assert_eq!(b.cwnd, 16.0);
        assert_eq!(
            b.phase,
            CongestionPhase::CongestionAvoidance,
            "chạm ngưỡng thì đổi pha"
        );

        // Ngưỡng KHÔNG phải trần cứng: qua ngưỡng, cửa sổ vẫn lớn dần — nhưng
        // theo cấp số CỘNG. Đây chính là chữ "AI" trong AIMD.
        for _ in 0..6 {
            b.on_ack();
        }
        assert_eq!(b.cwnd, 22.0, "16 + 6 lần cộng 1");
    }

    #[test]
    fn congestion_avoidance_grows_linearly() {
        let mut b = CongestionControl::new(4.0);
        for _ in 0..2 {
            b.on_ack();
        } // 1 -> 2 -> 4 (chạm ngưỡng)
        let prev = b.cwnd;
        b.on_ack();
        assert_eq!(
            b.cwnd,
            prev + 1.0,
            "pha tránh tắc nghẽn cộng 1, không nhân 2"
        );
    }

    #[test]
    fn timeout_resets_to_one_while_light_loss_halves() {
        let mut a = CongestionControl::new(64.0);
        for _ in 0..5 {
            a.on_ack();
        } // cwnd = 32
        let mut b = a.clone();
        a.on_triple_dup_ack();
        b.on_timeout();
        assert_eq!(a.cwnd, 16.0, "3 ACK trùng: giảm một nửa");
        assert_eq!(b.cwnd, 1.0, "hết giờ: về vạch xuất phát");
        assert_eq!(b.phase, CongestionPhase::SlowStart);
    }

    #[test]
    fn checksum_over_data_plus_checksum_is_zero() {
        let data = [0x45u8, 0x00, 0x00, 0x3c, 0x1c, 0x46, 0x40, 0x00, 0x40, 0x06];
        let cs = checksum(&data);
        let mut with_cs = data.to_vec();
        with_cs.extend_from_slice(&cs.to_be_bytes());
        assert!(verify_checksum(&with_cs), "tính chất vàng của tổng bù-1");
    }

    #[test]
    fn checksum_catches_single_bit_flip() {
        let data = [0x45u8, 0x00, 0x00, 0x3c, 0x1c, 0x46, 0x40, 0x00];
        let cs = checksum(&data);
        let mut corrupted = data.to_vec();
        corrupted.extend_from_slice(&cs.to_be_bytes());
        corrupted[3] ^= 0x08;
        assert!(!verify_checksum(&corrupted));
    }

    #[test]
    fn checksum_misses_word_transposition() {
        // Điểm YẾU đã biết: phép cộng có tính giao hoán nên đảo chỗ hai từ 16-bit
        // cho ra cùng checksum. Đây là lý do tầng ứng dụng vẫn cần CRC/hash mạnh.
        let a = [0x11u8, 0x22, 0x33, 0x44];
        let b = [0x33u8, 0x44, 0x11, 0x22];
        assert_eq!(checksum(&a), checksum(&b));
    }

    #[test]
    fn cidr_computes_network_and_broadcast() {
        let m = Subnet::parse("192.168.10.130/26").unwrap();
        assert_eq!(Subnet::display(m.network_address()), "192.168.10.128");
        assert_eq!(Subnet::display(m.broadcast()), "192.168.10.191");
        assert_eq!(m.usable_hosts(), 62); // 2^6 - 2
    }

    #[test]
    fn cidr_edge_cases() {
        assert_eq!(Subnet::parse("10.0.0.1/32").unwrap().usable_hosts(), 1);
        assert_eq!(Subnet::parse("10.0.0.0/31").unwrap().usable_hosts(), 2);
        assert_eq!(Subnet::parse("10.0.0.0/24").unwrap().usable_hosts(), 254);
        assert_eq!(Subnet::parse("0.0.0.0/0").unwrap().mask(), 0);
        assert!(Subnet::parse("10.0.0.0/33").is_none());
    }

    #[test]
    fn routing_picks_longest_prefix() {
        let table = [
            (Subnet::parse("0.0.0.0/0").unwrap(), "default"),
            (Subnet::parse("10.0.0.0/8").unwrap(), "eth0"),
            (Subnet::parse("10.1.0.0/16").unwrap(), "eth1"),
            (Subnet::parse("10.1.2.0/24").unwrap(), "eth2"),
        ];
        let ip = |s: &str| Subnet::parse(&format!("{s}/32")).unwrap().address;
        assert_eq!(match_route(&table, ip("10.1.2.5")), Some("eth2")); // /24 thắng /16 và /8
        assert_eq!(match_route(&table, ip("10.1.9.9")), Some("eth1"));
        assert_eq!(match_route(&table, ip("10.5.0.1")), Some("eth0"));
        assert_eq!(match_route(&table, ip("8.8.8.8")), Some("default"));
    }

    #[test]
    fn dns_follows_cname_chain() {
        let d = DnsServer {
            records: vec![
                ("a.vn".into(), DnsRecord::CNAME("b.vn".into())),
                ("b.vn".into(), DnsRecord::CNAME("c.vn".into())),
                ("c.vn".into(), DnsRecord::A("1.2.3.4".into())),
            ],
        };
        assert_eq!(d.resolve("a.vn"), Ok("1.2.3.4".into()));
    }

    #[test]
    fn dns_breaks_cname_loops() {
        let d = DnsServer {
            records: vec![
                ("x.vn".into(), DnsRecord::CNAME("y.vn".into())),
                ("y.vn".into(), DnsRecord::CNAME("x.vn".into())),
            ],
        };
        assert!(d.resolve("x.vn").unwrap_err().contains("vòng lặp"));
    }

    #[test]
    fn dns_reports_nxdomain() {
        let d = DnsServer { records: vec![] };
        assert!(
            d.resolve("does-not-exist.vn")
                .unwrap_err()
                .contains("NXDOMAIN")
        );
    }

    #[test]
    fn go_back_n_delivers_every_packet_in_order() {
        let result = go_back_n(10, 4, &[2, 6]);
        assert_eq!(
            result.delivered,
            (0..10).collect::<Vec<u32>>(),
            "phải giao đủ và đúng thứ tự"
        );
    }

    #[test]
    fn go_back_n_wastes_bandwidth_on_loss() {
        let clean = go_back_n(10, 4, &[]);
        let lossy = go_back_n(10, 4, &[2, 6]);
        assert_eq!(clean.send_count, 10, "kênh sạch: mỗi gói gửi đúng 1 lần");
        assert!(
            lossy.send_count > clean.send_count,
            "Go-Back-N gửi lại cả các gói KHÔNG mất — đó là cái giá của sự đơn giản"
        );
        // Lượt 1: 0..4 (mất 2, vứt 3) · lượt 2: 2..6 · lượt 3: 6..10 (mất 6) · lượt 4: 6..10
        assert_eq!(lossy.send_count, 16);
    }
}
