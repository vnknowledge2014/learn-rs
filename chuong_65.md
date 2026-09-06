# Chương 65: Mạng máy tính & Giao thức — Từ Bit Trên Dây Tới HTTP (Computer Networking & Protocols)

## Giới thiệu & Mục tiêu học tập

Chương 40 dạy bạn *viết công cụ quét cổng*, Chương 41 dạy *phân tích gói tin*. Cả hai đều là **người dùng** của mạng. Chương này giải thích mạng **hoạt động thế nào** — thứ mà mọi lập trình viên backend đều cần khi hệ thống chạy chậm một cách khó hiểu.

Ba câu hỏi chương này trả lời dứt điểm:

- Vì sao gửi 5 byte dữ liệu lại tốn 77 byte trên dây? (Đóng gói theo tầng.)
- Vì sao kết nối vừa đóng mà cổng vẫn "bận" cả phút? (Trạng thái `TIME_WAIT`.)
- Vì sao tải một tệp lớn lúc đầu chậm rồi mới nhanh dần? (Khởi động chậm (Slow start) của TCP.)

Mục tiêu học tập:
- Hiểu **mô hình phân tầng** và sự **đóng gói** — vì sao mỗi tầng chỉ nói chuyện với tầng ngang hàng của nó.
- Cài **máy trạng thái TCP** theo RFC 793: bắt tay ba bước, đóng bốn bước, và `TIME_WAIT`.
- Hiểu **điều khiển tắc nghẽn** (AIMD): vì sao Internet không sụp đổ dù ai cũng gửi hết sức.
- Cài **tổng kiểm tra Internet** và tự tay tìm ra điểm yếu đã biết của nó.
- Tính toán **CIDR** và cài quy tắc định tuyến **khớp tiền tố dài nhất**.
- Cài **phân giải DNS** có chống vòng lặp CNAME, và giao thức cửa sổ trượt **Go-Back-N**.

---

## Hình tượng hóa đời sống (Intuitive Everyday Analogy)

```
┌───────────────────────────────────────────────────────────────────────────────┐
│      HÌNH TƯỢNG: GỬI MỘT LÁ THƯ QUA HỆ THỐNG BƯU CHÍNH QUỐC TẾ                │
├───────────────────────────────────────────────────────────────────────────────┤
│                                                                               │
│  L7 ỨNG DỤNG   │ Nội dung thư: "Mai họp lúc 9 giờ nhé"                        │
│                │ Chỉ người GỬI và người NHẬN hiểu ý nghĩa.                    │
│       ▼        │                                                              │
│  L4 GIAO VẬN   │ PHONG BÌ: ghi "gửi phòng 302, từ phòng 105"                  │
│                │ = SỐ CỔNG. Cùng một tòa nhà, nhiều phòng khác nhau.          │
│                │ Thư bảo đảm (TCP) có biên nhận; thư thường (UDP) thì không.  │
│       ▼        │                                                              │
│  L3 MẠNG       │ ĐỊA CHỈ TÒA NHÀ: "12 Nguyễn Huệ, TP.HCM"                     │
│                │ = ĐỊA CHỈ IP. Bưu điện dùng nó để định tuyến liên tỉnh.      │
│       ▼        │                                                              │
│  L2 LIÊN KẾT   │ TÚI BƯU CHÍNH của xe tải chặng này                          │
│                │ = ĐỊA CHỈ MAC. Chỉ có giá trị trong CHẶNG NÀY, sang chặng    │
│                │   sau là bóc túi cũ, đóng túi mới.                           │
│       ▼        │                                                              │
│  L1 VẬT LÝ     │ Bánh xe lăn trên đường / điện tử chạy trên dây đồng          │
│                                                                               │
│  ★ NGUYÊN TẮC VÀNG: mỗi tầng CHỈ nói chuyện với tầng CÙNG CẤP ở đầu kia.      │
│    Người viết thư không cần biết xe tải nào chở. Tài xế không đọc thư.        │
│                                                                               │
├───────────────────────────────────────────────────────────────────────────────┤
│      BẮT TAY BA BƯỚC = HAI NGƯỜI GỌI ĐIỆN QUA ĐƯỜNG DÂY XẤU                   │
│                                                                               │
│    An:   "Alô, nghe rõ không?"          → SYN                                 │
│    Bình: "Rõ! Còn tôi thì sao?"          → SYN + ACK                          │
│    An:   "Nghe rõ luôn."                 → ACK                                │
│                                                                               │
│    Sau ba câu này, CẢ HAI đều chắc chắn: mình nghe được, và đối phương        │
│    cũng nghe được. Hai câu là chưa đủ — đó là lý do phải ba, không phải hai.  │
│                                                                               │
├───────────────────────────────────────────────────────────────────────────────┤
│      ĐIỀU KHIỂN TẮC NGHẼN = LÁI XE TRÊN ĐƯỜNG LẠ                              │
│                                                                               │
│    Chưa biết đường: đi CHẬM rồi TĂNG TỐC GẤP ĐÔI mỗi lần thấy an toàn         │
│                     (KHỞI ĐỘNG CHẬM — thật ra tăng rất nhanh!)                │
│    Gần tới ngưỡng:  chỉ tăng thêm 1 chút mỗi lần (TRÁNH TẮC NGHẼN)           │
│    Suýt va chạm:    GIẢM NGAY MỘT NỬA                                         │
│    Đâm thật rồi:    DỪNG HẲN, bò lại từ đầu (HẾT GIỜ)                        │
│                                                                               │
│    Tăng thì CỘNG, giảm thì NHÂN → "AIMD". Chính quy tắc bất đối xứng này     │
│    khiến hàng tỉ máy tính chia băng thông công bằng mà không cần trọng tài.  │
└───────────────────────────────────────────────────────────────────────────────┘
```

---

## Khái niệm & Cơ chế kỹ thuật chuyên sâu (In-Depth Technical Mechanics)

### 1. Chi phí đóng gói là có thật

Gửi 5 byte `"hello"` qua HTTP trên TCP/IP/Ethernet tốn **77 byte** trên dây: 18 byte header HTTP tối thiểu + 20 TCP + 20 IP + 14 Ethernet. Tức **93% là bao bì**.

Đây không phải chi tiết học thuật. Nó giải thích:
- Vì sao gộp nhiều thao tác nhỏ thành một yêu cầu lớn luôn nhanh hơn.
- Vì sao HTTP/2 phát minh ra nén header (HPACK).
- Vì sao game online dùng UDP với gói tin tự thiết kế thay vì HTTP.

### 2. `TIME_WAIT` — trạng thái bị hiểu lầm nhiều nhất

Sau khi đóng kết nối *chủ động*, TCP không về `CLOSED` ngay mà ngồi ở `TIME_WAIT` khoảng 2×MSL (thường 60 giây). Lập trình viên hay thấy nó như một phiền toái ("Address already in use"), nhưng nó tồn tại vì hai lý do nghiêm túc:

1. **Gói lạc đường phải chết hẳn.** Nếu mở ngay kết nối mới trên cùng cặp cổng, một gói tin cũ đi lạc đường về muộn có thể bị nhận nhầm là dữ liệu của kết nối mới. Hỏng dữ liệu, không có cách nào phát hiện.
2. **ACK cuối cùng có thể mất.** Nếu ACK cuối của ta mất, đối phương sẽ gửi lại FIN. Ta cần còn "sống" để trả lời, nếu không đối phương treo mãi.

Trong Rust, `SO_REUSEADDR` cho phép bind lại cổng đang ở `TIME_WAIT` — nhưng hãy hiểu bạn đang bỏ qua bảo vệ gì.

### 3. AIMD: vì sao Internet công bằng mà không cần trọng tài

Mỗi kết nối TCP tuân theo quy tắc:
- Thành công → cửa sổ **cộng** thêm 1 (mỗi RTT).
- Mất gói → cửa sổ **nhân** với 0.5.

Chứng minh trực giác cho tính công bằng: xét hai kết nối chia một đường truyền, vẽ trạng thái trên mặt phẳng `(cwnd₁, cwnd₂)`. Pha cộng đẩy điểm theo đường **chéo 45°** (song song đường công bằng). Pha nhân kéo điểm về **gốc tọa độ** theo đường thẳng qua gốc. Lặp hai bước này, điểm hội tụ về đường `cwnd₁ = cwnd₂` — tức chia đều.

Nếu đổi thành "tăng nhân, giảm nhân" (MIMD) thì tỉ lệ giữa hai kết nối không bao giờ đổi — kẻ chiếm nhiều mãi mãi chiếm nhiều. Sự **bất đối xứng** giữa cộng và nhân chính là nguồn gốc của công bằng.

### 4. Tổng kiểm tra Internet và giới hạn của nó

Thuật toán: cộng mọi từ 16-bit theo kiểu bù-1 (phần nhớ gấp vòng lại), rồi lấy bù. Tính chất vàng: **checksum của dữ liệu đã kèm checksum luôn bằng 0**, nên máy nhận chỉ cần cộng tất cả và so với 0.

Nhưng phép cộng có tính **giao hoán**, nên hoán vị hai từ 16-bit cho ra *cùng* checksum. Chương này có một bài kiểm thử chứng minh điều đó bằng số cụ thể. Đây là lý do:
- Ethernet dùng CRC-32 (mạnh hơn nhiều) ở tầng liên kết.
- Ứng dụng quan trọng vẫn phải tự dùng hàm băm mật mã.

Tổng kiểm tra Internet được thiết kế cho **tốc độ**, không cho **độ tin cậy** — nó phải tính được bằng vài lệnh CPU trên phần cứng năm 1981.

### 5. Khớp tiền tố dài nhất

Bảng định tuyến có nhiều dòng cùng khớp một địa chỉ. Quy tắc: **tiền tố dài nhất thắng**, vì tiền tố dài nghĩa là *cụ thể hơn*.

```
  Đích 10.1.2.5 khớp với cả bốn dòng sau:
    0.0.0.0/0     → cổng mặc định   (khớp mọi thứ, cụ thể nhất… là không)
    10.0.0.0/8    → eth0
    10.1.0.0/16   → eth1
    10.1.2.0/24   → eth2   ← CHỌN CÁI NÀY (24 bit, dài nhất)
```

Quy tắc này cho phép xây bảng định tuyến **theo lớp**: một dòng tổng quát bắt hết, rồi các dòng cụ thể hơn ghi đè cho từng vùng. Bộ định tuyến xương sống Internet dùng cấu trúc dữ liệu chuyên biệt (cây Patricia / TCAM phần cứng) để làm việc này ở tốc độ hàng trăm triệu gói mỗi giây.

---

## Mã nguồn minh họa thực chiến (Idiomatic Runnable Rust Blueprint)

Chạy bằng `cargo run -p ch65`, kiểm thử bằng `cargo test -p ch65`.

```rust
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
    Transport = 4,   // TCP/UDP — cổng, tin cậy
    Application = 7,   // HTTP/DNS — ý nghĩa dữ liệu
}

impl fmt::Display for Tang {
    fn fmt(&self, f: &mut fmt::Formatter) -> fmt::Result {
        let t = match self {
            Tang::VatLy => "Vật lý", Tang::LienKet => "Liên kết",
            Tang::Mang => "Mạng", Tang::Transport => "Giao vận", Tang::Application => "Ứng dụng",
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

/// Dựng chồng deliver thức: dữ liệu ứng dụng đi xuống, mỗi tầng thêm header.
pub fn dong_goi_xuong(du_lieu_ung_dung: &[u8]) -> GoiTin {
    let http = GoiTin { tang: Tang::Application, header: b"GET / HTTP/1.1\r\n\r\n".to_vec(),
                        tai: du_lieu_ung_dung.to_vec() };
    let tcp = http.boc(Tang::Transport, vec![0u8; 20]);   // TCP header tối thiểu 20 byte
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
    SynSent,      // SYN_SENT
    DaNhanSyn,     // SYN_RECEIVED
    DaThietLap,    // ESTABLISHED
    ChoDong1,      // FIN_WAIT_1
    ChoDong2,      // FIN_WAIT_2
    LastAck,       // TIME_WAIT — chờ 2×MSL để gói lạc đường chết hẳn
    CloseWait,// CLOSE_WAIT
    TimeWait,// LAST_ACK
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum TcpEvent {
    ActiveOpen,   // ứng dụng gọi connect()
    PassiveOpen,   // ứng dụng gọi listen()
    NhanSyn,
    NhanSynAck,
    NhanAck,
    NhanFin,
    AppClose, // ứng dụng gọi close()
    HetGio,      // hết 2×MSL
}

/// Chuyển trạng thái TCP — bảng này lấy thẳng từ RFC 793.
/// Trả về `None` nghĩa là sự kiện không hợp lệ ở trạng thái đó (gói bị bỏ).
pub fn transfer_state(tt: TcpState, sk: TcpEvent) -> Option<TcpState> {
    use TcpEvent::*;
    use TcpState::*;
    Some(match (tt, sk) {
        // --- Mở kết nối: bắt tay ba bước ---
        (Dong, ActiveOpen)      => SynSent,      // gửi SYN
        (Dong, PassiveOpen)      => Nghe,
        (Nghe, NhanSyn)        => DaNhanSyn,     // gửi SYN+ACK
        (SynSent, NhanSynAck) => DaThietLap,    // gửi ACK  ← bước 3
        (SynSent, NhanSyn)    => DaNhanSyn,     // mở đồng thời (hiếm)
        (DaNhanSyn, NhanAck)   => DaThietLap,

        // --- Đóng chủ động: bắt tay bốn bước ---
        (DaThietLap, AppClose) => ChoDong1,   // gửi FIN
        (ChoDong1, NhanAck)       => ChoDong2,
        (ChoDong2, NhanFin)       => LastAck,    // gửi ACK
        (ChoDong1, NhanFin)       => LastAck,    // đóng đồng thời
        (LastAck, HetGio)         => Dong,       // sau 2×MSL

        // --- Đóng thụ động ---
        (DaThietLap, NhanFin)        => CloseWait, // gửi ACK
        (CloseWait, AppClose)=> TimeWait, // gửi FIN
        (TimeWait, NhanAck)    => Dong,
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

    println!("\n1. ĐÓNG GÓI THEO TẦNG — 5 byte dữ liệu đi hết chồng deliver thức");
    let goi = dong_goi_xuong(b"hello");
    println!("   Gói cuối ở {} — tổng {} byte", goi.tang, goi.size());
    println!("   Chi phí phần đầu = {} byte cho 5 byte dữ liệu ({}% là bao bì)",
             goi.size() - 5, (goi.size() - 5) * 100 / goi.size());

    println!("\n2. BẮT TAY BA BƯỚC");
    use TcpEvent::*;
    let kq = run_session(TcpState::Dong, &[ActiveOpen, NhanSynAck]);
    println!("   Máy khách: Dong -SYN-> SynSent -SYN/ACK-> {:?}", kq.unwrap());
    let kq = run_session(TcpState::Dong, &[PassiveOpen, NhanSyn, NhanAck]);
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
        assert_eq!(run_session(Dong, &[ActiveOpen, NhanSynAck]), Ok(DaThietLap));
    }

    #[test]
    fn three_way_handshake_server_side() {
        assert_eq!(run_session(Dong, &[PassiveOpen, NhanSyn, NhanAck]), Ok(DaThietLap));
    }

    #[test]
    fn active_close_passes_through_time_wait() {
        let kq = run_session(Dong, &[ActiveOpen, NhanSynAck, AppClose, NhanAck, NhanFin]);
        assert_eq!(kq, Ok(LastAck), "phải dừng ở TIME_WAIT chứ không đóng ngay");
        assert_eq!(transfer_state(LastAck, HetGio), Some(Dong));
    }

    #[test]
    fn passive_close_passes_through_close_wait() {
        let kq = run_session(Dong, &[PassiveOpen, NhanSyn, NhanAck, NhanFin, AppClose, NhanAck]);
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
        // Điểm YẾU đã biết: phép cộng có tính deliver hoán nên đảo chỗ hai từ 16-bit
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
        assert_eq!(kq.da_nhan, (0..10).collect::<Vec<u32>>(), "phải deliver đủ và đúng thứ tự");
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
```

---

## Bảng tra cứu lỗi biên dịch thường gặp

| Lỗi | Nguyên nhân trong chương này | Cách sửa |
|---|---|---|
| `attempt to shift left with overflow` | `!0u32 << 32` khi tiền tố là `/0` | Xử lý riêng: `if self.prefix == 0 { 0 } else { !0u32 << (32 - t) }` |
| `attempt to subtract with overflow` | `(1u64 << (32 - t)) - 2` khi `t = 32` | Tách trường hợp biên `/31` và `/32` theo RFC 3021 |
| `E0277: the trait bound ... Option<Vec<u8>>` | `.map(\|x\| x.parse().ok()).collect::<Option<_>>()` cần chú thích kiểu | Ghi rõ `let o: Vec<u8> = ...` |
| `E0507: cannot move out of borrowed content` | `self.canh.get(&d)` rồi lặp và gọi `self.dfs` đệ quy | `.clone()` danh sách kề trước khi lặp — cách đơn giản nhất để cắt mượn |
| Kết quả checksum sai lệch trên máy khác | Dùng `from_le_bytes` thay vì `from_be_bytes` | Giao thức mạng **luôn** dùng thứ tự byte lớn trước (big-endian) |

---

## Tóm tắt chương & Bài tập rèn luyện (Summary & Exercises)

### 5 điểm cốt lõi cần ghi nhớ

1. **Phân tầng đổi hiệu suất lấy khả năng thay thế.** 93% bao bì là cái giá để bạn đổi Wi-Fi sang cáp quang mà không phải sửa một dòng mã ứng dụng nào.
2. **Bắt tay ba bước (Three-way handshake) không thừa một bước nào.** Ba lượt là số ít nhất để *cả hai* bên cùng chắc chắn kênh hai chiều thông suốt.
3. **`TIME_WAIT` là tính năng, không phải lỗi.** Nó bảo vệ bạn khỏi gói tin lạc đường của kết nối cũ.
4. **AIMD tạo ra công bằng từ sự bất đối xứng.** Tăng thì cộng, giảm thì nhân — không có quy tắc nào đơn giản hơn mà vẫn hội tụ.
5. **Tổng kiểm tra Internet là bộ lọc nhanh, không phải bảo chứng.** Nó bắt lỗi ngẫu nhiên, không bắt lỗi cố ý và không bắt hoán vị.

### Bài tập rèn luyện tự giải

**Bài 1.** Mở rộng máy trạng thái TCP để hỗ trợ **`RST`** (đặt lại kết nối): từ *bất kỳ* trạng thái nào, nhận `RST` đều đưa về `Dong` ngay lập tức, không qua `TIME_WAIT`.

<details>
<summary><b>Gợi ý</b></summary>

Thêm biến thể `NhanRst` vào `TcpEvent`, rồi đặt một nhánh `(_, NhanRst) => Dong` ở **đầu** khối `match` — trước mọi nhánh khác. Trong Rust, `match` xét các nhánh theo thứ tự viết, nên nhánh bao trùm đặt trước sẽ thắng.

Ý nghĩa thực tế: `RST` là cách nói "quên kết nối này đi ngay". Đó là điều xảy ra khi bạn kết nối tới một cổng không có ai lắng nghe — nhận `RST` chứ không phải chờ hết giờ.
</details>

<details>
<summary><b>Lời giải</b></summary>

```rust
// BƯỚC 1: thêm biến thể mới vào enum sự kiện.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum TcpEventV2 {
    ActiveOpen, PassiveOpen, NhanSyn, NhanSynAck, NhanAck, NhanFin,
    AppClose, HetGio,
    NhanRst,                     // ← mới
}

impl TcpEventV2 {
    /// Ánh xạ về sự kiện gốc; `None` với `NhanRst` vì nó không có tương ứng.
    fn to_base_event(self) -> Option<TcpEvent> {
        Some(match self {
            TcpEventV2::ActiveOpen => TcpEvent::ActiveOpen,
            TcpEventV2::PassiveOpen => TcpEvent::PassiveOpen,
            TcpEventV2::NhanSyn => TcpEvent::NhanSyn,
            TcpEventV2::NhanSynAck => TcpEvent::NhanSynAck,
            TcpEventV2::NhanAck => TcpEvent::NhanAck,
            TcpEventV2::NhanFin => TcpEvent::NhanFin,
            TcpEventV2::AppClose => TcpEvent::AppClose,
            TcpEventV2::HetGio => TcpEvent::HetGio,
            TcpEventV2::NhanRst => return None,
        })
    }
}

// BƯỚC 2: xét RST TRƯỚC mọi luật khác — nó phá kết nối từ bất kỳ trạng thái nào.
pub fn transition_v2(tt: TcpState, sk: TcpEventV2) -> Option<TcpState> {
    match sk.to_base_event() {
        None => Some(TcpState::Dong),          // NhanRst: về CLOSED ngay
        Some(root) => transfer_state(tt, root),
    }
}

// Kiểm chứng: RST hạ kết nối từ MỌI trạng thái, kể cả trạng thái mà
// không sự kiện nào khác làm được điều đó.
//   for tt in [TcpState::DaThietLap, TcpState::Nghe, TcpState::LastAck] {
//       assert_eq!(transition_v2(tt, TcpEventV2::NhanRst), Some(TcpState::Dong));
//   }
```

Lưu ý sự khác biệt quan trọng: đóng bằng `FIN` là **đóng lịch sự** — dữ liệu đang trên đường vẫn được giao xong. Đóng bằng `RST` là **cắt phăng** — dữ liệu trong bộ đệm bị vứt bỏ. Đó là lý do không nên dùng `RST` để đóng kết nối bình thường.
</details>

**Bài 2.** Cài giao thức **Lặp lại chọn lọc** (Selective Repeat) và so sánh với Go-Back-N: chỉ gửi lại *đúng* gói bị mất thay vì gửi lại từ gói đó trở đi.

<details>
<summary><b>Gợi ý</b></summary>

Máy nhận cần một **bộ đệm sắp xếp lại**: nó chấp nhận gói ngoài thứ tự và giữ lại, chờ lỗ hổng được lấp. Máy gửi cần theo dõi ACK riêng cho *từng* gói thay vì một mốc `has_num` duy nhất.

Đây chính là cách TCP hiện đại hoạt động, thông qua tùy chọn **SACK** (Selective Acknowledgment).
</details>

<details>
<summary><b>Lời giải</b></summary>

```rust
pub fn selective_repeat(tong_goi: u32, window: u32, mat_tai: &[u32]) -> TransferResult {
    let mut da_nhan_co: Vec<bool> = vec![false; tong_goi as usize];
    let mut count_send = 0;
    let mut con_mat: std::collections::VecDeque<u32> = mat_tai.iter().copied().collect();
    let mut has_num = 0u32;

    while has_num < tong_goi {
        for stt in has_num..(has_num + window).min(tong_goi) {
            if da_nhan_co[stt as usize] { continue; } // đã nhận rồi, không gửi lại
            count_send += 1;
            if con_mat.front() == Some(&stt) {
                con_mat.pop_front();
                continue; // CHỈ gói này mất; các gói sau vẫn tới nơi bình thường
            }
            da_nhan_co[stt as usize] = true;
        }
        // cửa sổ chỉ trượt qua phần đầu đã liên tục
        while has_num < tong_goi && da_nhan_co[has_num as usize] { has_num += 1; }
    }
    TransferResult {
        da_nhan: (0..tong_goi).collect(),
        count_send,
    }
}
```

Với 10 gói, cửa sổ 4, mất gói #2 và #6: Go-Back-N tốn **12** lượt gửi, Lặp lại chọn lọc chỉ tốn **12** lượt trong trường hợp này nhưng khoảng cách nới rộng nhanh khi cửa sổ lớn — với cửa sổ 100 và mất 1 gói, Go-Back-N gửi lại tới 100 gói còn Lặp lại chọn lọc chỉ gửi lại 1.

Cái giá: máy nhận phải có bộ đệm và logic phức tạp hơn hẳn. Đây là đánh đổi bộ nhớ ↔ băng thông kinh điển.
</details>

**Bài 3.** Cài bộ **chia mạng con**: cho một khối CIDR và số mạng con cần chia, trả về danh sách các khối con.

<details>
<summary><b>Gợi ý</b></summary>

Muốn chia thành `k` mạng con, cần mượn `ceil(log2(k))` bit từ phần máy chủ. Tiền tố mới = tiền tố cũ + số bit mượn. Mỗi mạng con cách nhau đúng `2^(32 - tiền_tố_mới)` địa chỉ.

Nhớ kiểm tra: tiền tố mới không được vượt quá 32.
</details>

<details>
<summary><b>Lời giải</b></summary>

```rust
pub fn chia_mang_con(root: MangCon, so_mang: u32) -> Option<Vec<MangCon>> {
    if so_mang == 0 { return None; }
    // Số bit cần mượn = trần của log2(so_mang).
    // Ví dụ: 3 mạng con vẫn phải mượn 2 bit (2 bit cho được 4 khối, dùng 3).
    let wanted_bit = if so_mang <= 1 { 0 } else { (so_mang - 1).ilog2() + 1 };
    let tien_to_moi = root.prefix as u32 + wanted_bit;
    if tien_to_moi > 32 { return None; } // không đủ địa chỉ để chia

    let step = if tien_to_moi == 32 { 1u32 } else { 1u32 << (32 - tien_to_moi) };
    let first = root.address_array();
    Some((0..so_mang)
        .map(|i| MangCon { address: first + i * step, prefix: tien_to_moi as u8 })
        .collect())
}
```

Ví dụ: chia `192.168.1.0/24` thành 4 mạng con → mượn 2 bit → bốn khối `/26`:
`192.168.1.0/26`, `192.168.1.64/26`, `192.168.1.128/26`, `192.168.1.192/26`.

Mỗi khối có 62 máy chủ dùng được (64 địa chỉ trừ địa chỉ mạng và địa chỉ quảng bá). Chú ý ta **mất** 8 địa chỉ so với `/24` nguyên khối (254 → 4×62 = 248) — đó là chi phí cố hữu của việc chia mạng con.
</details>
