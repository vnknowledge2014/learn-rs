//! # Chương 99: Bộ Công cụ Thâm nhập Mạng — Proxy SOCKS5, Chuyển tiếp Cổng,
//! Nhận dạng Dịch vụ & Đầu độc ARP/DNS (Network Pentest Toolkit)
//!
//! Crate `ch99` là chương thứ hai của **Chủ đề 28** (Bộ công cụ thâm nhập mạng) trong
//! giáo trình `learn-rs`. Toàn bộ mã nguồn viết bằng Rust 2024 thuần `std`, không có
//! phụ thuộc bên ngoài nào (`[dependencies]` rỗng tuyệt đối).
//!
//! Crate hiện thực hoá một bộ công cụ "đỏ" (red-team) chạy **hoàn toàn trên dữ liệu tổng
//! hợp tất định và mạng loopback 127.0.0.1**, không đụng tới bất kỳ dịch vụ thật nào:
//!
//! - `Socks5Server` + `Socks5Client`: máy chủ & máy khách proxy SOCKS5 (RFC 1928) hỗ trợ
//!   phương thức "không xác thực" và "user/pass" (RFC 1929), chỉ chấp nhận lệnh CONNECT.
//! - `TcpPortForwarder` + `bidirectional_relay`: chuyển tiếp cổng TCP hai chiều có RAII.
//! - `ServiceFingerprinter`: nhận dạng dịch vụ từ banner
//!   (HTTP/SSH/SMTP/MySQL/FTP/Redis/SMB/LDAP/POP3/IMAP) kèm trích xuất phiên bản.
//! - `NetworkMapper` + `NetworkSimulator`: dựng bản đồ mạng từ mô phỏng 5 máy chủ.
//! - `ArpPacket` (RFC 826) & `DnsPacket` (RFC 1035): phân tích, dựng lại gói tin và cơ chế
//!   đầu độc bộ nhớ đệm ARP (ARP cache poisoning) & giả mạo phản hồi DNS.
//! - `IpHeader`: phân tích header IPv4 để suy luận hệ điều hành từ giá trị TTL.
//!
//! Toàn bộ socket mạng đều được bind trên `127.0.0.1` với cổng ngẫu nhiên (`port 0`),
//! nên chương có thể chạy song song an toàn trên cùng một máy mà không đụng cổng cố định.
#![allow(dead_code)]

use std::collections::HashMap;
use std::io::{Read, Write};
use std::net::{IpAddr, Ipv4Addr, Shutdown, SocketAddr, TcpListener, TcpStream};
use std::time::Duration;

// ============================================================================
// 1. BỘ SINH SỐ GIẢ NGẪU NHIÊN TẤT ĐỊNH SPLITMIX64
// ============================================================================

/// Bộ sinh số giả ngẫu nhiên 64-bit tất định `SplitMix64`.
///
/// Thiết kế dùng struct với trường có tên (named fields), cho phép tái lập 100%
/// mọi kết quả lấy mẫu trên mọi kiến trúc CPU mà không phụ thuộc thư viện ngoài.
/// Trong chương này, nó được dùng để tạo *thứ tự quét cổng* ngẫu nhiên hoá (port scan
/// order) và các khoảng trễ jitter giả lập.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct SplitMix64 {
    /// Trạng thái thanh ghi 64-bit nội bộ
    pub state: u64,
}

impl SplitMix64 {
    /// Khởi tạo bộ sinh số với hạt giống cố định
    pub const fn new(seed: u64) -> Self {
        Self { state: seed }
    }

    /// Sinh số nguyên không dấu 64-bit ngẫu nhiên tiếp theo
    pub fn next_u64(&mut self) -> u64 {
        self.state = self.state.wrapping_add(0x9E37_79B9_7F4A_7C15);
        let mut z = (self.state ^ (self.state >> 30)).wrapping_mul(0xBF58_476D_1CE4_E5B9);
        z = (z ^ (z >> 27)).wrapping_mul(0x94D0_49BB_1331_11EB);
        z ^ (z >> 31)
    }

    /// Sinh số thực dấu phẩy động trong nửa khoảng `[0.0, 1.0)`
    pub fn next_f64(&mut self) -> f64 {
        let u = (self.next_u64() >> 11) as f64;
        u * (1.0 / ((1u64 << 53) as f64))
    }

    /// Sinh số thực dấu phẩy động có dấu trong khoảng `[-scale, scale]`
    pub fn next_signed(&mut self, scale: f64) -> f64 {
        (self.next_f64() * 2.0 - 1.0) * scale
    }

    /// Hoán vị ngẫu nhiên danh sách cổng (thuật toán Fisher–Yates) để tránh quét theo thứ tự tuần tự
    ///
    /// Hàm là **tất định**: cùng hạt giống luôn cho cùng một thứ tự quét.
    pub fn shuffled_ports(&self, ports: &[u16]) -> Vec<u16> {
        let mut items = ports.to_vec();
        let mut rng = *self;
        if items.len() < 2 {
            return items;
        }
        for i in (1..items.len()).rev() {
            let j = (rng.next_u64() % (i as u64 + 1)) as usize;
            items.swap(i, j);
        }
        items
    }
}

// ============================================================================
// 2. MÁY CHỦ PROXY SOCKS5 (RFC 1928) & MÁY KHÁCH SOCKS5
// ============================================================================

/// Kết quả của một phiên proxy SOCKS5 đã bắt tay thành công
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct Socks5Session {
    /// Địa chỉ đích mà client yêu cầu kết nối tới
    pub target: SocketAddr,
    /// Địa chỉ cục bộ mà proxy dùng để mở kết nối ra (trường BND.ADDR của SOCKS5)
    pub bound: SocketAddr,
    /// Tổng số byte đã được chuyển tiếp qua cả hai chiều
    pub relayed_bytes: usize,
}

/// Máy chủ proxy SOCKS5 với bắt tay 3 bước theo RFC 1928 / RFC 1929
///
/// ```text
/// Client                          Server (Socks5Server)
///   │  VER=5 NM=1 METHODS=[0x00]  ──►  (greeting)
///   │  VER=5 METHOD=0x00          ◄──  (lựa chọn phương thức: không xác thực)
///   │  VER=5 CMD=1 ATYP=... PORT  ──►  (request CONNECT)
///   │  VER=5 REP=0 BND.ADDR/PORT ◄──  (phản hồi thành công)
///   │        ... dữ liệu relay hai chiều ...
/// ```
///
/// Với phương thức `0x02` (user/pass) sẽ có thêm bước phụ theo RFC 1929:
/// `VER=1 ULEN USER PLEN PASS` → `VER=1 STATUS`.
#[derive(Debug, Clone)]
pub struct Socks5Server {
    /// Địa chỉ mà máy chủ dự kiến bind
    pub bind_addr: SocketAddr,
    /// Cờ yêu cầu xác thực bằng user/pass
    pub require_auth: bool,
    /// Danh sách cặp `(tên đăng nhập, mật khẩu)` được chấp nhận
    pub credentials: Vec<(String, String)>,
}

/// Bộ phân giải tên miền mặc định: chỉ chấp nhận **địa chỉ IPv4 loopback** dạng văn bản.
///
/// Hàm này cố tình không gọi hệ thống phân giải tên (DNS) của máy để bảo đảm chương
/// không bao giờ chạm tới hệ thống thật.
pub fn default_resolver(host: &str, port: u16) -> Option<SocketAddr> {
    let ip: IpAddr = host.parse().ok()?;
    if ip.is_loopback() {
        Some(SocketAddr::new(ip, port))
    } else {
        None
    }
}

impl Socks5Server {
    /// Khởi tạo máy chủ SOCKS5 chưa yêu cầu xác thực
    pub fn new(bind_addr: SocketAddr) -> Self {
        Self {
            bind_addr,
            require_auth: false,
            credentials: Vec::new(),
        }
    }

    /// Bật chế độ xác thực user/pass với danh sách thông tin đăng nhập cho trước
    pub fn with_auth(mut self, credentials: Vec<(String, String)>) -> Self {
        self.require_auth = !credentials.is_empty();
        self.credentials = credentials;
        self
    }

    /// Bắt tay và phục vụ một client bằng bộ phân giải mặc định (chỉ loopback)
    pub fn handle_client(&self, stream: &mut TcpStream) -> Result<Socks5Session, String> {
        self.handle_client_with_target(stream, &default_resolver)
    }

    /// Chấp nhận một kết nối từ `listener` rồi phục vụ bằng bộ phân giải mặc định
    pub fn accept_and_handle(&self, listener: &TcpListener) -> Result<Socks5Session, String> {
        self.accept_and_handle_with(listener, &default_resolver)
    }

    /// Chấp nhận một kết nối từ `listener` rồi phục vụ bằng bộ phân giải tùy biến
    pub fn accept_and_handle_with(
        &self,
        listener: &TcpListener,
        resolver: &dyn Fn(&str, u16) -> Option<SocketAddr>,
    ) -> Result<Socks5Session, String> {
        let (mut stream, _peer) = listener
            .accept()
            .map_err(|e| format!("Lỗi chấp nhận kết nối SOCKS5: {e}"))?;
        self.handle_client_with_target(&mut stream, resolver)
    }

    /// Bắt tay SOCKS5 3 bước rồi relay dữ liệu hai chiều tới đích
    ///
    /// * Bước 1 — Greeting: client gửi `VER=5, NMETHODS, METHODS...`.
    /// * Bước 2 — Method selection: server trả `VER=5, METHOD` (`0x00` không xác thực,
    ///   `0x02` user/pass theo RFC 1929).
    /// * Bước 3 — Request: client gửi `VER=5, CMD, RSV, ATYP, ADDR, PORT`; server trả
    ///   `VER=5, REP=0, RSV, ATYP, BND.ADDR, BND.PORT`.
    pub fn handle_client_with_target(
        &self,
        stream: &mut TcpStream,
        resolver: &dyn Fn(&str, u16) -> Option<SocketAddr>,
    ) -> Result<Socks5Session, String> {
        // Timeout cho bước bắt tay để một client treo không chặn vô hạn server
        let _ = stream.set_read_timeout(Some(Duration::from_secs(5)));

        // ---------------- Bước 1: Greeting ----------------
        let mut head = [0u8; 2];
        read_exact(stream, &mut head)?;
        if head[0] != 0x05 {
            return Err(format!(
                "Phiên bản SOCKS không hợp lệ: 0x{:02X} (mong đợi 0x05)",
                head[0]
            ));
        }
        let nmethods = usize::from(head[1]);
        if nmethods == 0 {
            return Err("Bản tin chào SOCKS5 không chứa phương thức xác thực nào".to_string());
        }
        let mut methods = vec![0u8; nmethods];
        read_exact(stream, &mut methods)?;

        let offers_none = methods.contains(&0x00);
        let offers_userpass = methods.contains(&0x02);

        let selected = if self.require_auth {
            if !offers_userpass {
                let _ = stream.write_all(&[0x05, 0xFF]);
                return Err(
                    "Máy chủ yêu cầu xác thực user/pass nhưng client không hỗ trợ".to_string(),
                );
            }
            0x02u8
        } else if offers_none {
            0x00u8
        } else if offers_userpass {
            0x02u8
        } else {
            // Theo RFC 1928: METHOD = 0xFF nghĩa là "không có phương thức chấp nhận được"
            let _ = stream.write_all(&[0x05, 0xFF]);
            return Err("Không tìm thấy phương thức xác thực SOCKS5 được hỗ trợ".to_string());
        };

        stream
            .write_all(&[0x05, selected])
            .map_err(|e| format!("Lỗi gửi lựa chọn phương thức SOCKS5: {e}"))?;

        // ---------------- Bước 2 (tuỳ chọn): User/Password theo RFC 1929 ----------------
        if selected == 0x02 {
            let mut sub = [0u8; 2];
            read_exact(stream, &mut sub)?;
            if sub[0] != 0x01 {
                return Err(format!(
                    "Phiên bản sub-negotiation user/pass không hợp lệ: 0x{:02X}",
                    sub[0]
                ));
            }
            let mut user = vec![0u8; usize::from(sub[1])];
            read_exact(stream, &mut user)?;
            let mut plen = [0u8; 1];
            read_exact(stream, &mut plen)?;
            let mut pass = vec![0u8; usize::from(plen[0])];
            read_exact(stream, &mut pass)?;

            let user_str = String::from_utf8_lossy(&user).to_string();
            let pass_str = String::from_utf8_lossy(&pass).to_string();
            let accepted = self
                .credentials
                .iter()
                .any(|(u, p)| *u == user_str && *p == pass_str);

            if accepted {
                stream
                    .write_all(&[0x01, 0x00])
                    .map_err(|e| format!("Lỗi gửi xác nhận đăng nhập SOCKS5: {e}"))?;
            } else {
                let _ = stream.write_all(&[0x01, 0x01]);
                return Err("Thông tin đăng nhập SOCKS5 không hợp lệ".to_string());
            }
        }

        // ---------------- Bước 3: Request CONNECT ----------------
        let mut req = [0u8; 4];
        read_exact(stream, &mut req)?;
        if req[0] != 0x05 {
            return Err(format!(
                "Phiên bản SOCKS trong yêu cầu không hợp lệ: 0x{:02X}",
                req[0]
            ));
        }
        if req[1] != 0x01 {
            return Err(format!(
                "Chỉ hỗ trợ lệnh CONNECT (0x01), nhận: 0x{:02X}",
                req[1]
            ));
        }

        let target = match req[3] {
            0x01 => {
                let mut ip = [0u8; 4];
                read_exact(stream, &mut ip)?;
                let mut port = [0u8; 2];
                read_exact(stream, &mut port)?;
                SocketAddr::new(Ipv4Addr::from(ip).into(), u16::from_be_bytes(port))
            }
            0x03 => {
                let mut len = [0u8; 1];
                read_exact(stream, &mut len)?;
                let mut raw_name = vec![0u8; usize::from(len[0])];
                read_exact(stream, &mut raw_name)?;
                let mut port = [0u8; 2];
                read_exact(stream, &mut port)?;
                let host = String::from_utf8_lossy(&raw_name).to_string();
                let port = u16::from_be_bytes(port);
                match resolver(&host, port) {
                    Some(addr) => addr,
                    None => return Err(format!("Không phân giải được tên miền: {host}")),
                }
            }
            0x04 => {
                return Err("Chưa hỗ trợ địa chỉ IPv6 trong SOCKS5 (ATYP = 0x04)".to_string());
            }
            other => {
                return Err(format!(
                    "Mã kiểu địa chỉ SOCKS5 không hợp lệ: 0x{other:02X}"
                ));
            }
        };

        // Ràng buộc an toàn: chỉ kết nối tới loopback trong môi trường giáo trình
        if !target.ip().is_loopback() {
            return Err(format!(
                "Chỉ hỗ trợ kết nối tới loopback trong môi trường giáo trình, nhận: {target}"
            ));
        }

        let mut upstream = TcpStream::connect(target)
            .map_err(|e| format!("Không kết nối được tới đích {target}: {e}"))?;
        let bound = upstream.local_addr().unwrap_or(target);

        // Phản hồi thành công: VER=5, REP=0x00, RSV=0x00, ATYP=0x01, BND.ADDR, BND.PORT
        let mut reply = vec![0x05, 0x00, 0x00, 0x01];
        match bound.ip() {
            IpAddr::V4(v4) => reply.extend_from_slice(&v4.octets()),
            IpAddr::V6(_) => reply.extend_from_slice(&[0u8; 4]),
        }
        reply.extend_from_slice(&bound.port().to_be_bytes());
        stream
            .write_all(&reply)
            .map_err(|e| format!("Lỗi gửi phản hồi CONNECT SOCKS5: {e}"))?;

        // Bỏ timeout cho giai đoạn relay dài hạn
        let _ = stream.set_read_timeout(None);

        let relayed_bytes = bidirectional_relay(stream, &mut upstream).unwrap_or(0);

        Ok(Socks5Session {
            target,
            bound,
            relayed_bytes,
        })
    }
}

/// Máy khách SOCKS5 tối giản dùng cho kiểm thử và minh hoạ
#[derive(Debug, Clone, Copy)]
pub struct Socks5Client;

impl Socks5Client {
    /// Kết nối tới proxy và thực hiện trọn bắt tay SOCKS5, trả về socket đã sẵn sàng truyền dữ liệu
    pub fn connect(
        proxy: SocketAddr,
        target: SocketAddr,
        auth: Option<(String, String)>,
    ) -> Result<TcpStream, String> {
        let mut stream = TcpStream::connect(proxy)
            .map_err(|e| format!("Không kết nối được tới proxy {proxy}: {e}"))?;
        Self::handshake(&mut stream, target, auth)?;
        Ok(stream)
    }

    /// Thực hiện bắt tay SOCKS5 (greeting → method → request) trên một socket đã mở
    pub fn handshake(
        stream: &mut TcpStream,
        target: SocketAddr,
        auth: Option<(String, String)>,
    ) -> Result<(), String> {
        // Greeting
        let methods: Vec<u8> = if auth.is_some() {
            vec![0x00, 0x02]
        } else {
            vec![0x00]
        };
        let mut greeting = vec![0x05, methods.len() as u8];
        greeting.extend_from_slice(&methods);
        stream
            .write_all(&greeting)
            .map_err(|e| format!("Lỗi gửi greeting SOCKS5: {e}"))?;

        let mut resp = [0u8; 2];
        read_exact(stream, &mut resp)?;
        if resp[0] != 0x05 {
            return Err(format!(
                "Phản hồi greeting SOCKS5 không hợp lệ: 0x{:02X}",
                resp[0]
            ));
        }

        if resp[1] == 0x02 {
            let (user, pass) = auth.ok_or(
                "Máy chủ yêu cầu xác thực user/pass nhưng client không có thông tin đăng nhập",
            )?;
            let mut sub = vec![0x01, user.len() as u8];
            sub.extend_from_slice(user.as_bytes());
            sub.push(pass.len() as u8);
            sub.extend_from_slice(pass.as_bytes());
            stream
                .write_all(&sub)
                .map_err(|e| format!("Lỗi gửi thông tin đăng nhập SOCKS5: {e}"))?;

            let mut auth_resp = [0u8; 2];
            read_exact(stream, &mut auth_resp)?;
            if auth_resp[1] != 0x00 {
                return Err("Xác thực SOCKS5 thất bại: thông tin đăng nhập bị từ chối".to_string());
            }
        } else if resp[1] != 0x00 {
            return Err(format!(
                "Máy chủ chọn phương thức xác thực SOCKS5 không hỗ trợ: 0x{:02X}",
                resp[1]
            ));
        }

        // Request CONNECT
        let mut request = vec![0x05, 0x01, 0x00];
        match target.ip() {
            IpAddr::V4(v4) => {
                request.push(0x01);
                request.extend_from_slice(&v4.octets());
            }
            IpAddr::V6(v6) => {
                request.push(0x04);
                request.extend_from_slice(&v6.octets());
            }
        }
        request.extend_from_slice(&target.port().to_be_bytes());
        stream
            .write_all(&request)
            .map_err(|e| format!("Lỗi gửi yêu cầu CONNECT SOCKS5: {e}"))?;

        let mut reply = [0u8; 4];
        read_exact(stream, &mut reply)?;
        if reply[0] != 0x05 {
            return Err(format!(
                "Phản hồi CONNECT SOCKS5 không hợp lệ: 0x{:02X}",
                reply[0]
            ));
        }
        if reply[1] != 0x00 {
            return Err(format!(
                "Lệnh CONNECT SOCKS5 thất bại với mã lỗi 0x{:02X}",
                reply[1]
            ));
        }

        // Bỏ qua trường BND.ADDR/BND.PORT tuỳ theo kiểu địa chỉ
        match reply[3] {
            0x01 => {
                let mut rest = [0u8; 6];
                read_exact(stream, &mut rest)?;
            }
            0x03 => {
                let mut len = [0u8; 1];
                read_exact(stream, &mut len)?;
                let mut name = vec![0u8; usize::from(len[0])];
                read_exact(stream, &mut name)?;
                let mut port = [0u8; 2];
                read_exact(stream, &mut port)?;
            }
            0x04 => {
                let mut rest = [0u8; 18];
                read_exact(stream, &mut rest)?;
            }
            other => {
                return Err(format!(
                    "Mã kiểu địa chỉ SOCKS5 không hợp lệ: 0x{other:02X}"
                ));
            }
        }

        Ok(())
    }
}

/// Đọc chính xác `buf.len()` byte từ socket, trả về thông điệp lỗi tiếng Việt
fn read_exact(stream: &mut TcpStream, buf: &mut [u8]) -> Result<(), String> {
    stream
        .read_exact(buf)
        .map_err(|e| format!("Lỗi đọc dữ liệu từ socket: {e}"))
}

// ============================================================================
// 3. CHUYẾN TIẾP CỔNG TCP HAI CHIỀU (RAII)
// ============================================================================

/// Cấu hình chuyển tiếp cổng TCP: mọi kết nối tới `local_addr` được chuyển tiếp tới `remote_addr`
#[derive(Debug, Clone)]
pub struct TcpPortForwarder {
    /// Địa chỉ cục bộ lắng nghe
    pub local_addr: SocketAddr,
    /// Địa chỉ đích thật sự
    pub remote_addr: SocketAddr,
    /// Cờ trạng thái: `false` nghĩa là forwarder đã bị tắt
    pub active: bool,
}

impl TcpPortForwarder {
    /// Khởi tạo forwarder ở trạng thái hoạt động
    pub fn new(local_addr: SocketAddr, remote_addr: SocketAddr) -> Self {
        Self {
            local_addr,
            remote_addr,
            active: true,
        }
    }

    /// Chuyển tiếp dữ liệu hai chiều giữa `local` và `remote`, trả về tổng số byte đã relay
    ///
    /// Nếu forwarder đã bị tắt (`active == false`) hàm trả về lỗi `BrokenPipe`.
    pub fn forward_once(
        &self,
        local: &mut TcpStream,
        remote: &mut TcpStream,
    ) -> std::io::Result<usize> {
        if !self.active {
            return Err(std::io::Error::new(
                std::io::ErrorKind::BrokenPipe,
                "Forwarder đã bị tắt, không thể chuyển tiếp dữ liệu",
            ));
        }
        bidirectional_relay(local, remote)
    }

    /// Tắt forwarder (đóng cổng nghe theo quy ước RAII của crate)
    pub fn shutdown(&mut self) {
        self.active = false;
    }

    /// Chấp nhận một kết nối từ `listener`, mở kết nối tới đích rồi relay hai chiều
    pub fn accept_and_forward(&self, listener: &TcpListener) -> std::io::Result<usize> {
        if !self.active {
            return Err(std::io::Error::new(
                std::io::ErrorKind::BrokenPipe,
                "Forwarder đã bị tắt, không thể chấp nhận kết nối mới",
            ));
        }
        let (mut local, _peer) = listener.accept()?;
        let mut remote = TcpStream::connect(self.remote_addr)?;
        self.forward_once(&mut local, &mut remote)
    }
}

impl Drop for TcpPortForwarder {
    /// RAII: khi forwarder rơi khỏi phạm vi, cờ trạng thái được tự động tắt
    fn drop(&mut self) {
        self.active = false;
    }
}

/// Chuyển tiếp dữ liệu hai chiều giữa hai socket bằng 2 luồng, trả về tổng số byte đã relay
///
/// Mỗi luồng sao chép theo một chiều và thực hiện **half-close** (`shutdown(Write)`) khi
/// chiều đó kết thúc, nhờ đó một chiều EOF không làm giết chết chiều còn lại.
pub fn bidirectional_relay(a: &mut TcpStream, b: &mut TcpStream) -> std::io::Result<usize> {
    let a_reader = a.try_clone()?;
    let a_writer = a.try_clone()?;
    let b_reader = b.try_clone()?;
    let b_writer = b.try_clone()?;

    let forward = std::thread::spawn(move || relay_half(a_reader, b_writer));
    let backward = std::thread::spawn(move || relay_half(b_reader, a_writer));

    let fwd_bytes = forward.join().unwrap_or(0);
    let bwd_bytes = backward.join().unwrap_or(0);
    Ok(fwd_bytes + bwd_bytes)
}

/// Sao chép byte từ `src` sang `dst` cho tới khi EOF, trả về số byte đã sao chép
fn relay_half(mut src: TcpStream, mut dst: TcpStream) -> usize {
    let mut buf = [0u8; 8192];
    let mut total = 0usize;
    loop {
        match src.read(&mut buf) {
            Ok(0) => break,
            Ok(n) => {
                if dst.write_all(&buf[..n]).is_err() {
                    break;
                }
                total += n;
            }
            Err(_) => break,
        }
    }
    // Half-close: báo cho đầu bên kia biết chiều này đã kết thúc
    let _ = dst.shutdown(Shutdown::Write);
    total
}

// ============================================================================
// 4. NHẬN DẠNG DỊCH VỤ (SERVICE FINGERPRINTING) & SUY LUẬN HỆ ĐIỀU HÀNH
// ============================================================================

/// Hệ điều hành suy luận được từ dấu vân tay mạng
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
pub enum OperatingSystem {
    /// Hệ điều hành họ Linux/Unix (TTL 64)
    Linux,
    /// Hệ điều hành họ Windows (TTL 128)
    Windows,
    /// Thiết bị mạng (router/switch/firewall) — TTL 255
    NetworkDevice,
    /// Không xác định
    Unknown,
}

impl OperatingSystem {
    /// Nhãn hiển thị ngắn gọn
    pub fn label(&self) -> &'static str {
        match self {
            OperatingSystem::Linux => "Linux",
            OperatingSystem::Windows => "Windows",
            OperatingSystem::NetworkDevice => "NetworkDevice",
            OperatingSystem::Unknown => "Unknown",
        }
    }
}

impl std::fmt::Display for OperatingSystem {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        f.write_str(self.label())
    }
}

/// Một "probe" dùng để nhận dạng dịch vụ: gửi payload, rồi so khớp banner nhận về
#[derive(Debug, Clone)]
pub struct ServiceProbe {
    /// Tên dịch vụ (chuẩn hoá, chữ thường)
    pub name: &'static str,
    /// Cổng mặc định của dịch vụ
    pub port: u16,
    /// Payload gửi đi khi thăm dò (rỗng nghĩa là chỉ chờ banner chủ động)
    pub payload: &'static [u8],
    /// Danh sách cặp `(tên dịch vụ, mẫu cần xuất hiện)` để khớp banner
    pub matchers: &'static [(&'static str, &'static str)],
}

/// Kết quả nhận dạng dịch vụ
#[derive(Debug, Clone, PartialEq)]
pub struct ServiceMatch {
    /// Tên dịch vụ nhận dạng được
    pub service: String,
    /// Phiên bản trích được từ banner (có thể rỗng)
    pub version: String,
    /// Độ tin cậy trong khoảng `[0.0, 1.0]`
    pub confidence: f64,
}

/// Bộ nhận dạng dịch vụ dựa trên banner
#[derive(Debug, Clone)]
pub struct ServiceFingerprinter {
    /// Danh sách probe đang dùng
    pub probes: Vec<ServiceProbe>,
}

/// Danh mục probe mặc định của bộ nhận dạng dịch vụ
pub fn service_catalog() -> Vec<ServiceProbe> {
    vec![
        ServiceProbe {
            name: "http",
            port: 80,
            payload: b"GET / HTTP/1.0\r\nHost: probe.local\r\n\r\n",
            matchers: &[("http", "HTTP/1."), ("http", "Server:")],
        },
        ServiceProbe {
            name: "ssh",
            port: 22,
            payload: b"",
            matchers: &[("ssh", "SSH-")],
        },
        ServiceProbe {
            name: "smtp",
            port: 25,
            payload: b"EHLO probe.local\r\n",
            matchers: &[("smtp", "220 "), ("smtp", "250 ")],
        },
        ServiceProbe {
            name: "mysql",
            port: 3306,
            payload: b"",
            matchers: &[("mysql", "\n8.0"), ("mysql", "\n5.7"), ("mysql", "\n10.")],
        },
        ServiceProbe {
            name: "ftp",
            port: 21,
            payload: b"",
            matchers: &[("ftp", "220")],
        },
        ServiceProbe {
            name: "redis",
            port: 6379,
            payload: b"PING\r\n",
            matchers: &[("redis", "+PONG"), ("redis", "-NOAUTH")],
        },
        ServiceProbe {
            name: "smb",
            port: 445,
            payload: b"",
            matchers: &[("smb", "SMB")],
        },
        ServiceProbe {
            name: "ldap",
            port: 389,
            payload: b"",
            matchers: &[("ldap", "a\x7f")],
        },
        ServiceProbe {
            name: "pop3",
            port: 110,
            payload: b"",
            matchers: &[("pop3", "+OK")],
        },
        ServiceProbe {
            name: "imap",
            port: 143,
            payload: b"",
            matchers: &[("imap", "* OK")],
        },
    ]
}

impl ServiceFingerprinter {
    /// Khởi tạo bộ nhận dạng với danh mục probe mặc định
    pub fn new() -> Self {
        Self {
            probes: service_catalog(),
        }
    }

    /// Nhận dạng dịch vụ từ banner nhận về trên cổng `port`
    ///
    /// Chiến lược hai tầng: (1) ưu tiên probe có cổng khớp; (2) nếu không khớp, quét toàn bộ
    /// danh mục để bắt dịch vụ đang chạy trên cổng "lạ".
    pub fn fingerprint(&self, port: u16, response: &[u8]) -> Option<ServiceMatch> {
        if let Some(probe) = self.probes.iter().find(|p| p.port == port)
            && let Some(found) = match_probe(probe, response, true)
        {
            return Some(found);
        }

        for probe in &self.probes {
            if probe.port == port {
                continue;
            }
            if let Some(found) = match_probe(probe, response, false) {
                return Some(found);
            }
        }

        None
    }
}

impl Default for ServiceFingerprinter {
    fn default() -> Self {
        Self::new()
    }
}

/// Khớp một probe với banner, trả về kết quả nếu có mẫu khớp
fn match_probe(probe: &ServiceProbe, response: &[u8], port_matched: bool) -> Option<ServiceMatch> {
    for (index, (service, pattern)) in probe.matchers.iter().enumerate() {
        if find_bytes(response, pattern.as_bytes()).is_none() {
            continue;
        }
        let version = extract_version(service, response);
        let mut confidence: f64 = if port_matched { 0.60 } else { 0.40 };
        if !version.is_empty() {
            confidence += 0.25;
        }
        if index == 0 {
            confidence += 0.15;
        }
        return Some(ServiceMatch {
            service: (*service).to_string(),
            version,
            confidence: confidence.min(1.0),
        });
    }
    None
}

/// Tìm vị trí đầu tiên của `needle` trong `haystack`
pub fn find_bytes(haystack: &[u8], needle: &[u8]) -> Option<usize> {
    if needle.is_empty() || haystack.len() < needle.len() {
        return None;
    }
    haystack.windows(needle.len()).position(|w| w == needle)
}

/// Trích xuất phiên bản dịch vụ từ banner (regex-lite tự viết, không dùng crate ngoài)
pub fn extract_version(service: &str, response: &[u8]) -> String {
    match service {
        "http" => {
            // Ưu tiên giá trị của header "Server:" (ví dụ "nginx/1.18.0")
            if let Some(pos) = find_bytes(response, b"Server:") {
                let value = take_token(response, pos + 7);
                if !value.is_empty() {
                    return value;
                }
            }
            // Nếu không có header Server thì dùng chính phiên bản giao thức
            if let Some(pos) = find_bytes(response, b"HTTP/1.") {
                return take_token(response, pos);
            }
            String::new()
        }
        "ssh" => match find_bytes(response, b"SSH-") {
            Some(pos) => take_token(response, pos),
            None => String::new(),
        },
        "mysql" => {
            // Greeting MySQL: [độ dài 3 byte][sequence 1 byte][phiên bản giao thức 1 byte][chuỗi phiên bản\0]
            if response.len() > 5 && response[4] == 0x0A {
                let rest = &response[5..];
                let end = rest.iter().position(|&b| b == 0).unwrap_or(rest.len());
                return String::from_utf8_lossy(&rest[..end]).to_string();
            }
            String::new()
        }
        "smtp" | "ftp" | "pop3" | "imap" => {
            let line = take_line(response, 0);
            find_version_token(&line).unwrap_or(line)
        }
        "redis" => match find_bytes(response, b"redis_version:") {
            Some(pos) => take_token(response, pos + 13),
            None => String::new(),
        },
        "smb" => match response.first() {
            Some(0xFF) => "SMBv1".to_string(),
            Some(0xFE) => "SMBv2".to_string(),
            _ => String::new(),
        },
        _ => String::new(),
    }
}

/// Lấy một dòng văn bản (kết thúc bằng CR/LF) kể từ vị trí `start`
fn take_line(bytes: &[u8], start: usize) -> String {
    let slice = &bytes[start.min(bytes.len())..];
    let end = slice
        .iter()
        .position(|&b| b == b'\r' || b == b'\n')
        .unwrap_or(slice.len());
    String::from_utf8_lossy(&slice[..end]).trim().to_string()
}

/// Lấy một "token" (chuỗi không chứa khoảng trắng) kể từ vị trí `start`
fn take_token(bytes: &[u8], start: usize) -> String {
    let slice = &bytes[start.min(bytes.len())..];
    let begin = slice
        .iter()
        .position(|&b| !b.is_ascii_whitespace() && b != 0)
        .unwrap_or(slice.len());
    let rest = &slice[begin..];
    let end = rest
        .iter()
        .position(|&b| b.is_ascii_whitespace() || b == 0)
        .unwrap_or(rest.len());
    String::from_utf8_lossy(&rest[..end]).to_string()
}

/// Tìm token trông giống phiên bản (chứa cả chữ số và dấu chấm), ví dụ `1.3.5`, `8.9p1`
fn find_version_token(text: &str) -> Option<String> {
    for raw in text.split_whitespace() {
        let token: String = raw
            .chars()
            .filter(|c| c.is_ascii_alphanumeric() || *c == '.' || *c == '_' || *c == '-')
            .collect();
        if token.chars().any(|c| c.is_ascii_digit()) && token.contains('.') {
            return Some(token);
        }
    }
    None
}

/// Suy luận hệ điều hành từ giá trị TTL của gói IPv4 (64 = Linux, 128 = Windows, 255 = thiết bị mạng)
pub fn detect_os_from_ttl(ttl: u8) -> OperatingSystem {
    match ttl {
        64 => OperatingSystem::Linux,
        128 => OperatingSystem::Windows,
        255 => OperatingSystem::NetworkDevice,
        _ => OperatingSystem::Unknown,
    }
}

/// Suy luận hệ điều hành từ kích thước cửa sổ TCP (65535 = Windows XP, 8192/29200 = Linux)
pub fn detect_os_from_window(window: u16) -> OperatingSystem {
    match window {
        65535 => OperatingSystem::Windows,
        8192 | 29200 => OperatingSystem::Linux,
        _ => OperatingSystem::Unknown,
    }
}

// ============================================================================
// 5. BẢN ĐỒ MẠNG (NETWORK MAPPER)
// ============================================================================

/// Thông tin thu thập được về một máy chủ trong mạng
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct HostInfo {
    /// Địa chỉ IP của máy chủ
    pub ip: IpAddr,
    /// Hệ điều hành suy luận được
    pub os: OperatingSystem,
    /// Danh sách cổng đang mở
    pub open_ports: Vec<u16>,
    /// Ánh xạ cổng → tên dịch vụ
    pub services: HashMap<u16, String>,
    /// Tên máy chủ nếu biết
    pub hostname: Option<String>,
}

/// Bản đồ mạng: tập hợp thông tin của nhiều máy chủ
#[derive(Debug, Clone, Default)]
pub struct NetworkMapper {
    /// Ánh xạ địa chỉ IP → thông tin máy chủ
    pub hosts: HashMap<IpAddr, HostInfo>,
}

impl NetworkMapper {
    /// Khởi tạo bản đồ rỗng
    pub fn new() -> Self {
        Self {
            hosts: HashMap::new(),
        }
    }

    /// Thêm (hoặc cập nhật) một máy chủ vào bản đồ
    pub fn add_host(&mut self, ip: IpAddr, hostname: Option<String>) {
        let entry = self.hosts.entry(ip).or_insert_with(|| HostInfo {
            ip,
            os: OperatingSystem::Unknown,
            open_ports: Vec::new(),
            services: HashMap::new(),
            hostname: None,
        });
        if hostname.is_some() {
            entry.hostname = hostname;
        }
    }

    /// Ghi nhận một dịch vụ đang lắng nghe trên cổng `port` của máy `ip`
    pub fn record_service(&mut self, ip: IpAddr, port: u16, service: String) {
        self.add_host(ip, None);
        if let Some(entry) = self.hosts.get_mut(&ip) {
            entry.services.insert(port, service);
            if !entry.open_ports.contains(&port) {
                entry.open_ports.push(port);
            }
        }
    }

    /// Ghi nhận hệ điều hành của máy `ip`
    pub fn record_os(&mut self, ip: IpAddr, os: OperatingSystem) {
        self.add_host(ip, None);
        if let Some(entry) = self.hosts.get_mut(&ip) {
            entry.os = os;
        }
    }

    /// Sinh báo cáo bản đồ mạng dạng văn bản (thứ tự máy chủ được sắp xếp theo IP cho tính tất định)
    pub fn to_report(&self) -> String {
        let mut ordered: Vec<(&IpAddr, &HostInfo)> = self.hosts.iter().collect();
        ordered.sort_by_key(|(ip, _)| **ip);

        let mut out = String::new();
        out.push_str("================ BẢN ĐỒ MẠNG (NETWORK MAP) ================\n");
        out.push_str(&format!(
            "Tổng số host: {} | Tổng số cổng mở: {}\n",
            self.host_count(),
            self.total_open_ports()
        ));
        out.push_str("--------------------------------------------------------\n");

        for (ip, info) in ordered {
            let host_label = info.hostname.clone().unwrap_or_else(|| "-".to_string());
            out.push_str(&format!(
                "{:<16} {:<14} {}\n",
                ip.to_string(),
                format!("[{}]", info.os.label()),
                host_label
            ));

            let mut ports = info.open_ports.clone();
            ports.sort_unstable();
            for port in ports {
                let service = info
                    .services
                    .get(&port)
                    .cloned()
                    .unwrap_or_else(|| "unknown".to_string());
                out.push_str(&format!("    {port:>5}/tcp   {service}\n"));
            }
        }

        out
    }

    /// Số lượng máy chủ trong bản đồ
    pub fn host_count(&self) -> usize {
        self.hosts.len()
    }

    /// Tổng số cổng đang mở của toàn bộ máy chủ trong bản đồ
    pub fn total_open_ports(&self) -> usize {
        self.hosts.values().map(|h| h.open_ports.len()).sum()
    }
}

// ============================================================================
// 6. GÓI TIN ARP (RFC 826) & ĐẦU ĐỘC BỘ NHỚ ĐỆM ARP
// ============================================================================

/// Mã thao tác của gói ARP
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum ArpOp {
    /// Yêu cầu: "Ai có IP này? Hãy gửi địa chỉ MAC cho tôi"
    Request,
    /// Trả lời: "IP này nằm ở MAC này"
    Reply,
}

impl ArpOp {
    /// Mã số trên dây (1 = Request, 2 = Reply)
    pub fn code(&self) -> u16 {
        match self {
            ArpOp::Request => 1,
            ArpOp::Reply => 2,
        }
    }

    /// Chuyển mã số trên dây thành mã thao tác
    pub fn from_code(code: u16) -> Option<ArpOp> {
        match code {
            1 => Some(ArpOp::Request),
            2 => Some(ArpOp::Reply),
            _ => None,
        }
    }
}

/// Độ dài chuẩn của phần tải ARP (không tính phần header Ethernet 14 byte)
pub const ARP_PACKET_LEN: usize = 28;

/// Một gói ARP đầy đủ theo RFC 826
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct ArpPacket {
    /// Mã thao tác (Request/Reply)
    pub op: ArpOp,
    /// Địa chỉ MAC của máy gửi
    pub sender_mac: [u8; 6],
    /// Địa chỉ IP của máy gửi
    pub sender_ip: Ipv4Addr,
    /// Địa chỉ MAC của máy nhận (thường toàn 0 trong Request)
    pub target_mac: [u8; 6],
    /// Địa chỉ IP của máy nhận
    pub target_ip: Ipv4Addr,
}

impl ArpPacket {
    /// Phân tích gói ARP từ mảng byte
    ///
    /// Hàm chấp nhận cả hai dạng: **tải ARP thô** (28 byte) và **khung Ethernet đầy đủ**
    /// (bỏ qua 14 byte header Ethernet: 6 byte MAC đích + 6 byte MAC nguồn + 2 byte EtherType).
    pub fn parse(bytes: &[u8]) -> Result<ArpPacket, String> {
        if bytes.len() < ARP_PACKET_LEN {
            return Err(format!(
                "Gói ARP quá ngắn: {} byte (cần tối thiểu {ARP_PACKET_LEN})",
                bytes.len()
            ));
        }

        let offset = if looks_like_arp_payload(bytes) {
            0
        } else if bytes.len() >= 14 && looks_like_arp_payload(&bytes[14..]) {
            14
        } else if bytes.len() >= 12 && looks_like_arp_payload(&bytes[12..]) {
            12
        } else {
            return Err(
                "Gói ARP không hợp lệ: không tìm thấy trường htype=1/ptype=0x0800".to_string(),
            );
        };

        let payload = &bytes[offset..];
        let htype = u16::from_be_bytes([payload[0], payload[1]]);
        let ptype = u16::from_be_bytes([payload[2], payload[3]]);
        let hlen = payload[4];
        let plen = payload[5];

        if htype != 1 {
            return Err(format!(
                "Loại phần cứng ARP không hỗ trợ: {htype} (chỉ nhận 1)"
            ));
        }
        if ptype != 0x0800 {
            return Err(format!("Loại giao thức ARP không hỗ trợ: 0x{ptype:04X}"));
        }
        if hlen != 6 || plen != 4 {
            return Err(format!(
                "Độ dài địa chỉ ARP không hợp lệ: hlen={hlen}, plen={plen}"
            ));
        }

        let op_code = u16::from_be_bytes([payload[6], payload[7]]);
        let Some(op) = ArpOp::from_code(op_code) else {
            return Err(format!("Mã thao tác ARP không hợp lệ: {op_code}"));
        };

        let mut sender_mac = [0u8; 6];
        sender_mac.copy_from_slice(&payload[8..14]);
        let mut target_mac = [0u8; 6];
        target_mac.copy_from_slice(&payload[18..24]);

        let sender_ip = Ipv4Addr::new(payload[14], payload[15], payload[16], payload[17]);
        let target_ip = Ipv4Addr::new(payload[24], payload[25], payload[26], payload[27]);

        Ok(ArpPacket {
            op,
            sender_mac,
            sender_ip,
            target_mac,
            target_ip,
        })
    }

    /// Mã hoá gói ARP thành mảng byte (28 byte, không kèm header Ethernet)
    pub fn to_bytes(&self) -> Vec<u8> {
        let mut out = Vec::with_capacity(ARP_PACKET_LEN);
        out.extend_from_slice(&1u16.to_be_bytes()); // htype = Ethernet
        out.extend_from_slice(&0x0800u16.to_be_bytes()); // ptype = IPv4
        out.push(6); // hlen
        out.push(4); // plen
        out.extend_from_slice(&self.op.code().to_be_bytes());
        out.extend_from_slice(&self.sender_mac);
        out.extend_from_slice(&self.sender_ip.octets());
        out.extend_from_slice(&self.target_mac);
        out.extend_from_slice(&self.target_ip.octets());
        out
    }

    /// Dựng gói trả lời ARP độc hại nhằm đầu độc bộ nhớ đệm ARP (ARP cache poisoning)
    ///
    /// Gói trả lời này nói với nạn nhân rằng: *"IP của `attacker_ip` nằm ở MAC của kẻ tấn công"*.
    pub fn spoof_reply_for(
        request: &ArpPacket,
        attacker_mac: [u8; 6],
        attacker_ip: Ipv4Addr,
    ) -> ArpPacket {
        ArpPacket {
            op: ArpOp::Reply,
            sender_mac: attacker_mac,
            sender_ip: attacker_ip,
            target_mac: request.sender_mac,
            target_ip: request.sender_ip,
        }
    }
}

impl std::fmt::Display for ArpPacket {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        write!(
            f,
            "{} {} [{}] -> {} [{}]",
            match self.op {
                ArpOp::Request => "ARP-Request",
                ArpOp::Reply => "ARP-Reply",
            },
            mac_to_string(&self.sender_mac),
            self.sender_ip,
            mac_to_string(&self.target_mac),
            self.target_ip
        )
    }
}

/// Kiểm tra một lát byte có phải tải ARP hợp lệ không
fn looks_like_arp_payload(bytes: &[u8]) -> bool {
    bytes.len() >= 8
        && bytes[0] == 0x00
        && bytes[1] == 0x01
        && bytes[2] == 0x08
        && bytes[3] == 0x00
        && bytes[4] == 6
        && bytes[5] == 4
}

/// Tạo gói trả lời ARP độc hại nhằm đầu độc bộ nhớ đệm ARP (ARP cache poisoning)
///
/// Gói trả lời này nói với nạn nhân rằng: *"IP của `attacker_ip` nằm ở MAC của kẻ tấn công"*.
pub fn spoof_reply_for(
    request: &ArpPacket,
    attacker_mac: [u8; 6],
    attacker_ip: Ipv4Addr,
) -> ArpPacket {
    ArpPacket::spoof_reply_for(request, attacker_mac, attacker_ip)
}

/// Chuyển địa chỉ MAC 6 byte thành chuỗi `aa:bb:cc:dd:ee:ff`
pub fn mac_to_string(mac: &[u8; 6]) -> String {
    mac.iter()
        .map(|b| format!("{b:02x}"))
        .collect::<Vec<String>>()
        .join(":")
}

// ============================================================================
// 7. GÓI TIN DNS (RFC 1035) & GIẢ MẠO PHẢN HỒI DNS
// ============================================================================

/// Kiểu bản ghi DNS
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum DnsRecordType {
    /// Bản ghi địa chỉ IPv4 (mã 1)
    A,
    /// Bản ghi địa chỉ IPv6 (mã 28)
    Aaaa,
    /// Bản ghi bí danh (mã 5)
    Cname,
    /// Bản ghi máy chủ tên miền (mã 2)
    Ns,
    /// Kiểu không biết, giữ nguyên mã số
    Unknown(u16),
}

impl DnsRecordType {
    /// Mã số của kiểu bản ghi trên dây
    pub fn to_code(self) -> u16 {
        match self {
            DnsRecordType::A => 1,
            DnsRecordType::Ns => 2,
            DnsRecordType::Cname => 5,
            DnsRecordType::Aaaa => 28,
            DnsRecordType::Unknown(code) => code,
        }
    }

    /// Chuyển mã số trên dây thành kiểu bản ghi
    pub fn from_code(code: u16) -> Self {
        match code {
            1 => DnsRecordType::A,
            2 => DnsRecordType::Ns,
            5 => DnsRecordType::Cname,
            28 => DnsRecordType::Aaaa,
            other => DnsRecordType::Unknown(other),
        }
    }
}

/// Một câu hỏi trong gói DNS
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct DnsQuestion {
    /// Tên miền được hỏi
    pub name: String,
    /// Kiểu bản ghi mong muốn
    pub qtype: DnsRecordType,
    /// Lớp của câu hỏi (1 = IN, Internet)
    pub qclass: u16,
}

/// Dữ liệu của một bản ghi DNS
#[derive(Debug, Clone, PartialEq, Eq)]
pub enum DnsRecordData {
    /// Địa chỉ IPv4
    IPv4(Ipv4Addr),
    /// Địa chỉ IPv6 (16 byte)
    IPv6([u8; 16]),
    /// Tên miền (CNAME/NS)
    Cname(String),
    /// Dữ liệu thô không phân loại được
    Raw(Vec<u8>),
}

/// Một bản ghi trả lời trong gói DNS
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct DnsRecord {
    /// Tên miền sở hữu bản ghi
    pub name: String,
    /// Kiểu bản ghi
    pub rtype: DnsRecordType,
    /// Thời gian sống của bản ghi (giây)
    pub ttl: u32,
    /// Nội dung bản ghi
    pub data: DnsRecordData,
}

/// Một gói tin DNS đầy đủ
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct DnsPacket {
    /// Mã định danh giao dịch (16 bit)
    pub id: u16,
    /// `true` nếu đây là gói phản hồi
    pub is_response: bool,
    /// Danh sách câu hỏi
    pub questions: Vec<DnsQuestion>,
    /// Danh sách bản ghi trả lời
    pub answers: Vec<DnsRecord>,
}

impl DnsPacket {
    /// Dựng một câu hỏi truy vấn với mã định danh tất định (FNV-1a trên tên miền)
    pub fn build_query(name: &str, qtype: DnsRecordType) -> DnsPacket {
        DnsPacket {
            id: query_id_for(name, qtype),
            is_response: false,
            questions: vec![DnsQuestion {
                name: name.to_string(),
                qtype,
                qclass: 1,
            }],
            answers: Vec::new(),
        }
    }

    /// Dựng một gói phản hồi từ câu hỏi gốc
    pub fn build_response(query: &DnsPacket, answers: Vec<DnsRecord>) -> DnsPacket {
        DnsPacket {
            id: query.id,
            is_response: true,
            questions: query.questions.clone(),
            answers,
        }
    }

    /// Dựng phản hồi DNS giả mạo (DNS spoofing) trả về địa chỉ `answer_ip` cho câu hỏi đầu tiên
    pub fn spoof_response(query: &DnsPacket, answer_ip: Ipv4Addr, ttl: u32) -> DnsPacket {
        let name = query
            .questions
            .first()
            .map(|q| q.name.clone())
            .unwrap_or_default();
        let answer = DnsRecord {
            name: name.clone(),
            rtype: DnsRecordType::A,
            ttl,
            data: DnsRecordData::IPv4(answer_ip),
        };
        DnsPacket::build_response(query, vec![answer])
    }

    /// Mã hoá gói DNS thành mảng byte
    pub fn to_bytes(&self) -> Vec<u8> {
        let mut out = Vec::new();
        out.extend_from_slice(&self.id.to_be_bytes());

        // Flags: QR (phản hồi) + RD (yêu cầu đệ quy) + RA (trả lời đệ quy)
        let flags: u16 = if self.is_response { 0x8180 } else { 0x0100 };
        out.extend_from_slice(&flags.to_be_bytes());

        out.extend_from_slice(&(self.questions.len() as u16).to_be_bytes());
        out.extend_from_slice(&(self.answers.len() as u16).to_be_bytes());
        out.extend_from_slice(&0u16.to_be_bytes()); // NSCOUNT
        out.extend_from_slice(&0u16.to_be_bytes()); // ARCOUNT

        for question in &self.questions {
            out.extend_from_slice(&encode_name(&question.name));
            out.extend_from_slice(&question.qtype.to_code().to_be_bytes());
            out.extend_from_slice(&question.qclass.to_be_bytes());
        }

        for answer in &self.answers {
            out.extend_from_slice(&encode_name(&answer.name));
            out.extend_from_slice(&answer.rtype.to_code().to_be_bytes());
            out.extend_from_slice(&1u16.to_be_bytes()); // CLASS = IN
            out.extend_from_slice(&answer.ttl.to_be_bytes());
            let rdata = encode_rdata(answer);
            out.extend_from_slice(&(rdata.len() as u16).to_be_bytes());
            out.extend_from_slice(&rdata);
        }

        out
    }

    /// Phân tích gói DNS từ mảng byte
    pub fn parse(bytes: &[u8]) -> Result<DnsPacket, String> {
        if bytes.len() < 12 {
            return Err(format!(
                "Gói DNS quá ngắn: {} byte (cần tối thiểu 12 byte cho phần header)",
                bytes.len()
            ));
        }

        let id = u16::from_be_bytes([bytes[0], bytes[1]]);
        let flags = u16::from_be_bytes([bytes[2], bytes[3]]);
        let is_response = flags & 0x8000 != 0;
        let qdcount = usize::from(u16::from_be_bytes([bytes[4], bytes[5]]));
        let ancount = usize::from(u16::from_be_bytes([bytes[6], bytes[7]]));

        let mut offset = 12usize;
        let mut questions = Vec::with_capacity(qdcount);
        for _ in 0..qdcount {
            let (name, next) = decode_name(bytes, offset)?;
            if next + 4 > bytes.len() {
                return Err("Phần câu hỏi DNS bị cắt ngắn".to_string());
            }
            let qtype =
                DnsRecordType::from_code(u16::from_be_bytes([bytes[next], bytes[next + 1]]));
            let qclass = u16::from_be_bytes([bytes[next + 2], bytes[next + 3]]);
            offset = next + 4;
            questions.push(DnsQuestion {
                name,
                qtype,
                qclass,
            });
        }

        let mut answers = Vec::with_capacity(ancount);
        for _ in 0..ancount {
            let (name, next) = decode_name(bytes, offset)?;
            if next + 10 > bytes.len() {
                return Err("Phần trả lời DNS bị cắt ngắn".to_string());
            }
            let rtype =
                DnsRecordType::from_code(u16::from_be_bytes([bytes[next], bytes[next + 1]]));
            let ttl = u32::from_be_bytes([
                bytes[next + 4],
                bytes[next + 5],
                bytes[next + 6],
                bytes[next + 7],
            ]);
            let rdlength = usize::from(u16::from_be_bytes([bytes[next + 8], bytes[next + 9]]));
            let data_start = next + 10;
            if data_start + rdlength > bytes.len() {
                return Err("Dữ liệu bản ghi DNS bị cắt ngắn".to_string());
            }
            let data = decode_rdata(rtype, &bytes[data_start..data_start + rdlength])?;
            offset = data_start + rdlength;
            answers.push(DnsRecord {
                name,
                rtype,
                ttl,
                data,
            });
        }

        Ok(DnsPacket {
            id,
            is_response,
            questions,
            answers,
        })
    }
}

/// Mã định danh truy vấn tất định (FNV-1a 32 bit, lấy 16 bit thấp)
pub fn query_id_for(name: &str, qtype: DnsRecordType) -> u16 {
    let mut hash: u32 = 0x811C_9DC5;
    for byte in name.as_bytes() {
        hash ^= u32::from(*byte);
        hash = hash.wrapping_mul(0x0100_0193);
    }
    hash ^= u32::from(qtype.to_code());
    hash = hash.wrapping_mul(0x0100_0193);
    (hash & 0xFFFF) as u16
}

/// Mã hoá tên miền theo định dạng nhãn (label) của DNS
pub fn encode_name(name: &str) -> Vec<u8> {
    let mut out = Vec::new();
    let trimmed = name.trim_end_matches('.');
    if trimmed.is_empty() {
        out.push(0);
        return out;
    }
    for label in trimmed.split('.') {
        let bytes = label.as_bytes();
        let len = bytes.len().min(63);
        out.push(len as u8);
        out.extend_from_slice(&bytes[..len]);
    }
    out.push(0);
    out
}

/// Giải mã tên miền từ byte, hỗ trợ con trỏ nén (RFC 1035 §4.1.4)
///
/// Trả về `(tên miền, offset tiếp theo)` — offset tiếp theo là vị trí ngay sau
/// con trỏ nén (hoặc sau byte kết thúc nếu không có nén).
pub fn decode_name(bytes: &[u8], offset: usize) -> Result<(String, usize), String> {
    let mut labels: Vec<String> = Vec::new();
    let mut pos = offset;
    let mut jumps = 0usize;
    let mut next_offset = offset;

    loop {
        if pos >= bytes.len() {
            return Err("Tên miền DNS bị cắt ngắn".to_string());
        }
        let len = bytes[pos];

        if len == 0 {
            pos += 1;
            if jumps == 0 {
                next_offset = pos;
            }
            break;
        }

        if len & 0xC0 == 0xC0 {
            if pos + 1 >= bytes.len() {
                return Err("Con trỏ nén tên miền DNS bị cắt ngắn".to_string());
            }
            let pointer = ((usize::from(len & 0x3F)) << 8) | usize::from(bytes[pos + 1]);
            if jumps == 0 {
                next_offset = pos + 2;
            }
            jumps += 1;
            if jumps > 16 {
                return Err("Phát hiện vòng lặp con trỏ nén tên miền DNS".to_string());
            }
            pos = pointer;
            continue;
        }

        if len > 63 {
            return Err(format!("Độ dài nhãn tên miền DNS không hợp lệ: {len}"));
        }
        let start = pos + 1;
        let end = start + usize::from(len);
        if end > bytes.len() {
            return Err("Nhãn tên miền DNS bị cắt ngắn".to_string());
        }
        labels.push(String::from_utf8_lossy(&bytes[start..end]).to_string());
        pos = end;
    }

    let name = if labels.is_empty() {
        ".".to_string()
    } else {
        labels.join(".")
    };
    Ok((name, next_offset))
}

/// Mã hoá phần dữ liệu (RDATA) của một bản ghi
fn encode_rdata(record: &DnsRecord) -> Vec<u8> {
    match &record.data {
        DnsRecordData::IPv4(ip) => ip.octets().to_vec(),
        DnsRecordData::IPv6(bytes) => bytes.to_vec(),
        DnsRecordData::Cname(name) => encode_name(name),
        DnsRecordData::Raw(bytes) => bytes.clone(),
    }
}

/// Giải mã phần dữ liệu (RDATA) của một bản ghi
fn decode_rdata(rtype: DnsRecordType, data: &[u8]) -> Result<DnsRecordData, String> {
    match rtype {
        DnsRecordType::A => {
            if data.len() != 4 {
                return Err(format!(
                    "Dữ liệu bản ghi A phải dài đúng 4 byte, nhận {}",
                    data.len()
                ));
            }
            Ok(DnsRecordData::IPv4(Ipv4Addr::new(
                data[0], data[1], data[2], data[3],
            )))
        }
        DnsRecordType::Aaaa => {
            if data.len() != 16 {
                return Err(format!(
                    "Dữ liệu bản ghi AAAA phải dài đúng 16 byte, nhận {}",
                    data.len()
                ));
            }
            let mut bytes = [0u8; 16];
            bytes.copy_from_slice(data);
            Ok(DnsRecordData::IPv6(bytes))
        }
        DnsRecordType::Cname | DnsRecordType::Ns => {
            let (name, _) = decode_name(data, 0)?;
            Ok(DnsRecordData::Cname(name))
        }
        DnsRecordType::Unknown(_) => Ok(DnsRecordData::Raw(data.to_vec())),
    }
}

// ============================================================================
// 8. PHÂN TÍCH HEADER IPv4 (HỖ TRỢ SUY LUẬN HỆ ĐIỀU HÀNH QUA TTL)
// ============================================================================

/// Header IPv4 tối giản (20 byte) dùng để suy luận dấu vân tay hệ điều hành
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct IpHeader {
    /// Phiên bản IP (luôn là 4)
    pub version: u8,
    /// Giá trị Time To Live — dấu vân tay đặc trưng của hệ điều hành
    pub ttl: u8,
    /// Mã giao thức tầng trên (6 = TCP, 17 = UDP)
    pub protocol: u8,
    /// Địa chỉ nguồn
    pub src: Ipv4Addr,
    /// Địa chỉ đích
    pub dst: Ipv4Addr,
    /// Tổng độ dài gói (header + payload)
    pub total_len: u16,
}

impl IpHeader {
    /// Phân tích header IPv4 từ mảng byte
    pub fn parse(bytes: &[u8]) -> Result<IpHeader, String> {
        if bytes.len() < 20 {
            return Err(format!(
                "Header IPv4 quá ngắn: {} byte (cần tối thiểu 20 byte)",
                bytes.len()
            ));
        }
        let version = bytes[0] >> 4;
        let ihl = bytes[0] & 0x0F;
        if version != 4 {
            return Err(format!("Phiên bản IP không phải 4: {version}"));
        }
        if ihl < 5 {
            return Err(format!(
                "Độ dài header IP không hợp lệ: {ihl} từ (cần >= 5)"
            ));
        }
        let total_len = u16::from_be_bytes([bytes[2], bytes[3]]);
        let ttl = bytes[8];
        let protocol = bytes[9];
        let src = Ipv4Addr::new(bytes[12], bytes[13], bytes[14], bytes[15]);
        let dst = Ipv4Addr::new(bytes[16], bytes[17], bytes[18], bytes[19]);
        Ok(IpHeader {
            version,
            ttl,
            protocol,
            src,
            dst,
            total_len,
        })
    }

    /// Mã hoá header IPv4 thành mảng byte 20 byte
    pub fn to_bytes(&self) -> Vec<u8> {
        let mut out = vec![0u8; 20];
        out[0] = (self.version << 4) | 0x05; // version 4, IHL = 5 từ
        out[2] = (self.total_len >> 8) as u8;
        out[3] = (self.total_len & 0xFF) as u8;
        out[8] = self.ttl;
        out[9] = self.protocol;
        out[12..16].copy_from_slice(&self.src.octets());
        out[16..20].copy_from_slice(&self.dst.octets());
        out
    }

    /// Tên giao thức tầng trên
    pub fn protocol_name(&self) -> &'static str {
        match self.protocol {
            1 => "ICMP",
            6 => "TCP",
            17 => "UDP",
            47 => "GRE",
            50 => "ESP",
            _ => "Unknown",
        }
    }
}

// ============================================================================
// 9. MÔ PHỎNG MẠNG TẤT ĐỊNH (NETWORK SIMULATOR)
// ============================================================================

/// Mô phỏng một máy chủ trong mạng nội bộ
#[derive(Debug, Clone)]
pub struct HostSim {
    /// Địa chỉ IP của máy
    pub ip: IpAddr,
    /// Tên máy chủ
    pub hostname: String,
    /// Danh sách cổng đang mở
    pub open_ports: Vec<u16>,
    /// Banner trả về cho từng cổng
    pub banners: HashMap<u16, Vec<u8>>,
    /// Hệ điều hành của máy
    pub os: OperatingSystem,
}

/// Mô phỏng một mạng nội bộ đầy đủ với 5 máy chủ mẫu
#[derive(Debug, Clone)]
pub struct NetworkSimulator {
    /// Các máy chủ trong mạng mô phỏng
    pub hosts: HashMap<IpAddr, HostSim>,
    /// Bộ đếm địa chỉ kế tiếp sẽ quét (dùng để sinh thứ tự quét)
    pub next_ip: u32,
}

/// Sinh địa chỉ MAC giả lập tất định từ địa chỉ IPv4
pub fn simulated_mac(ip: Ipv4Addr) -> [u8; 6] {
    let octets = ip.octets();
    [0x02, 0x00, 0x00, 0x00, 0x00, octets[3]]
}

/// Dựng banner chào của dịch vụ MySQL (gói greeting theo giao thức MySQL 4.1+)
fn mysql_greeting(version: &str) -> Vec<u8> {
    let mut body: Vec<u8> = Vec::new();
    body.push(0x0A); // phiên bản giao thức 10
    body.extend_from_slice(version.as_bytes());
    body.push(0x00); // kết thúc chuỗi phiên bản
    body.extend_from_slice(&[0x2A, 0x00, 0x00, 0x00]); // thread id
    body.extend_from_slice(b"scrambl"); // dữ liệu ngẫu nhiên phần 1
    body.push(0x00);

    let mut packet: Vec<u8> = Vec::new();
    let len_bytes = (body.len() as u32).to_le_bytes();
    packet.extend_from_slice(&len_bytes[..3]);
    packet.push(0x00); // số thứ tự gói
    packet.extend_from_slice(&body);
    packet
}

/// Dựng header SMB1 tối giản (bắt đầu bằng chữ ký 0xFF 'S' 'M' 'B')
fn smb_banner() -> Vec<u8> {
    let mut banner: Vec<u8> = vec![0xFF, b'S', b'M', b'B'];
    banner.extend_from_slice(&[0x72, 0x00, 0x00, 0x00]); // command/response + status
    banner.extend_from_slice(&[0x00; 24]);
    banner
}

/// Dựng phản hồi BindResponse LDAP (mã 0x61 0x7F = LDAPMessage BindResponse)
fn ldap_banner() -> Vec<u8> {
    vec![0x61, 0x7F, 0x01, 0x01, 0x00, 0x04, 0x00, 0x04, 0x00]
}

impl NetworkSimulator {
    /// Khởi tạo mô phỏng với 5 máy chủ mẫu
    ///
    /// | IP | Hostname | HĐH | Cổng mở |
    /// |---|---|---|---|
    /// | 10.10.10.5 | web01.lab | Linux | 80, 443, 8080 |
    /// | 10.10.10.10 | db01.lab | Linux | 22, 3306 |
    /// | 10.10.10.20 | files01.lab | Linux | 21, 139, 445 |
    /// | 10.10.10.30 | mail01.lab | Windows | 25, 110, 143, 389, 587 |
    /// | 10.10.10.254 | router.lab | NetworkDevice | 22, 23, 161 |
    pub fn new() -> Self {
        let mut hosts: HashMap<IpAddr, HostSim> = HashMap::new();

        let http_banner =
            b"HTTP/1.1 200 OK\r\nServer: nginx/1.18.0 (Ubuntu)\r\nContent-Length: 2\r\n\r\nOK"
                .to_vec();

        let insert = |hosts: &mut HashMap<IpAddr, HostSim>, host: HostSim| {
            hosts.insert(host.ip, host);
        };

        insert(
            &mut hosts,
            HostSim {
                ip: Ipv4Addr::new(10, 10, 10, 5).into(),
                hostname: "web01.lab".to_string(),
                open_ports: vec![80, 443, 8080],
                banners: HashMap::from([
                    (80, http_banner.clone()),
                    (443, http_banner.clone()),
                    (
                        8080,
                        b"HTTP/1.0 403 Forbidden\r\nServer: nginx/1.18.0\r\n\r\n".to_vec(),
                    ),
                ]),
                os: OperatingSystem::Linux,
            },
        );

        insert(
            &mut hosts,
            HostSim {
                ip: Ipv4Addr::new(10, 10, 10, 10).into(),
                hostname: "db01.lab".to_string(),
                open_ports: vec![22, 3306],
                banners: HashMap::from([
                    (22, b"SSH-2.0-OpenSSH_8.9p1 Ubuntu-3ubuntu0.4\r\n".to_vec()),
                    (3306, mysql_greeting("8.0.32-0ubuntu0.2")),
                ]),
                os: OperatingSystem::Linux,
            },
        );

        insert(
            &mut hosts,
            HostSim {
                ip: Ipv4Addr::new(10, 10, 10, 20).into(),
                hostname: "files01.lab".to_string(),
                open_ports: vec![21, 139, 445],
                banners: HashMap::from([
                    (21, b"220-ProFTPD 1.3.5 Server (Debian) [::1]\r\n".to_vec()),
                    (445, smb_banner()),
                    (139, b"\xff\x53\x4d\x42\x72\x00".to_vec()),
                ]),
                os: OperatingSystem::Linux,
            },
        );

        insert(
            &mut hosts,
            HostSim {
                ip: Ipv4Addr::new(10, 10, 10, 30).into(),
                hostname: "mail01.lab".to_string(),
                open_ports: vec![25, 110, 143, 389, 587],
                banners: HashMap::from([
                    (25, b"220 mail.lab ESMTP Postfix\r\n".to_vec()),
                    (110, b"+OK POP3 mail.lab server ready\r\n".to_vec()),
                    (143, b"* OK IMAP4rev1 Service Ready\r\n".to_vec()),
                    (389, ldap_banner()),
                    (587, b"220 mail.lab ESMTP Submission\r\n".to_vec()),
                ]),
                os: OperatingSystem::Windows,
            },
        );

        insert(
            &mut hosts,
            HostSim {
                ip: Ipv4Addr::new(10, 10, 10, 254).into(),
                hostname: "router.lab".to_string(),
                open_ports: vec![22, 23, 161],
                banners: HashMap::from([(22, b"SSH-2.0-OpenSSH_7.4p1 Cisco\r\n".to_vec())]),
                os: OperatingSystem::NetworkDevice,
            },
        );

        Self { hosts, next_ip: 1 }
    }

    /// Quét một máy chủ trong mô phỏng và dựng thông tin `HostInfo`
    pub fn scan(&self, ip: IpAddr) -> HostInfo {
        let fingerprinter = ServiceFingerprinter::new();
        let Some(host) = self.hosts.get(&ip) else {
            return HostInfo {
                ip,
                os: OperatingSystem::Unknown,
                open_ports: Vec::new(),
                services: HashMap::new(),
                hostname: None,
            };
        };

        let mut services = HashMap::new();
        for port in &host.open_ports {
            let service = host
                .banners
                .get(port)
                .and_then(|banner| fingerprinter.fingerprint(*port, banner))
                .map(|m| m.service)
                .unwrap_or_else(|| "unknown".to_string());
            services.insert(*port, service);
        }

        HostInfo {
            ip: host.ip,
            os: host.os,
            open_ports: host.open_ports.clone(),
            services,
            hostname: Some(host.hostname.clone()),
        }
    }

    /// Dựng bản đồ mạng đầy đủ từ toàn bộ máy chủ trong mô phỏng
    pub fn map_all(&self) -> NetworkMapper {
        let mut mapper = NetworkMapper::new();
        let mut ips: Vec<IpAddr> = self.hosts.keys().copied().collect();
        ips.sort();

        for ip in ips {
            let info = self.scan(ip);
            mapper.add_host(info.ip, info.hostname.clone());
            mapper.record_os(info.ip, info.os);
            let mut entries: Vec<(u16, String)> = info
                .services
                .iter()
                .map(|(port, service)| (*port, service.clone()))
                .collect();
            entries.sort_by_key(|(port, _)| *port);
            for (port, service) in entries {
                mapper.record_service(info.ip, port, service);
            }
        }

        mapper
    }

    /// Sinh gói ARP đầu độc nhắm vào `victim`, khai báo `gateway` nằm ở MAC của kẻ tấn công
    pub fn arp_poison(&self, victim: IpAddr, gateway: IpAddr) -> ArpPacket {
        let attacker_mac = [0x02, 0x00, 0x00, 0x00, 0x00, 0xFF];
        let target_mac = match self.hosts.get(&victim) {
            Some(host) => match host.ip {
                IpAddr::V4(v4) => simulated_mac(v4),
                IpAddr::V6(_) => [0u8; 6],
            },
            None => [0u8; 6],
        };

        ArpPacket {
            op: ArpOp::Reply,
            sender_mac: attacker_mac,
            sender_ip: match gateway {
                IpAddr::V4(v4) => v4,
                IpAddr::V6(_) => Ipv4Addr::UNSPECIFIED,
            },
            target_mac,
            target_ip: match victim {
                IpAddr::V4(v4) => v4,
                IpAddr::V6(_) => Ipv4Addr::UNSPECIFIED,
            },
        }
    }

    /// Sinh địa chỉ IP kế tiếp để quét (dùng cho thứ tự quét ngẫu nhiên hoá)
    pub fn next_scan_ip(&mut self) -> Option<IpAddr> {
        if self.next_ip > 254 {
            return None;
        }
        let ip = Ipv4Addr::new(10, 10, 10, self.next_ip as u8);
        self.next_ip += 1;
        Some(ip.into())
    }
}

impl Default for NetworkSimulator {
    fn default() -> Self {
        Self::new()
    }
}

// ============================================================================
// 10. TIỆN ÍCH HIỂN THỊ: HEX DUMP & MÁY CHỦ ECHO CỤC BỘ
// ============================================================================

/// Tạo bản hex dump (dạng `hexdump -C`) cho một mảng byte
pub fn hex_dump(data: &[u8]) -> String {
    let mut out = String::new();
    for (index, chunk) in data.chunks(16).enumerate() {
        let hex: Vec<String> = chunk.iter().map(|b| format!("{b:02x}")).collect();
        let ascii: String = chunk
            .iter()
            .map(|&b| {
                if (32..127).contains(&b) {
                    b as char
                } else {
                    '.'
                }
            })
            .collect();
        out.push_str(&format!(
            "{:04x}  {:<47}  |{}|\n",
            index * 16,
            hex.join(" "),
            ascii
        ));
    }
    out
}

/// Khởi động một máy chủ echo cục bộ trên `127.0.0.1` với cổng ngẫu nhiên
///
/// Máy chủ này chỉ phục vụ **một** kết nối và dùng để kiểm thử proxy/chuyển tiếp cổng.
pub fn spawn_echo_server() -> SocketAddr {
    let listener =
        TcpListener::bind("127.0.0.1:0").expect("Không bind được cổng loopback ngẫu nhiên");
    let addr = listener
        .local_addr()
        .expect("Không lấy được địa chỉ của cổng loopback");
    std::thread::spawn(move || {
        if let Ok((mut stream, _peer)) = listener.accept() {
            let _ = stream.set_read_timeout(Some(Duration::from_secs(5)));
            let mut buf = [0u8; 4096];
            while let Ok(n) = stream.read(&mut buf) {
                if n == 0 {
                    break;
                }
                if stream.write_all(&buf[..n]).is_err() {
                    break;
                }
            }
        }
    });
    addr
}

// ============================================================================
// 11. HÀM MAIN MINH HỌA
// ============================================================================

fn main() {
    println!("=== CHƯƠNG 99: BỘ CÔNG CỤ THÂM NHẬP MẠNG (NETWORK PENTEST TOOLKIT) ===");
    println!("    Proxy SOCKS5 | Chuyển tiếp cổng | Nhận dạng dịch vụ | Đầu độc ARP/DNS");
    println!("    (toàn bộ chạy trên mạng loopback 127.0.0.1 với dữ liệu tổng hợp tất định)");

    // ---------------------------------------------------------------- 1. Bản đồ mạng
    println!("\n--- 1. BẢN ĐỒ MẠNG (NetworkMapper) ---");
    let simulator = NetworkSimulator::new();
    let mapper = simulator.map_all();
    print!("{}", mapper.to_report());
    println!(
        "    => {} host, {} cổng mở, tổng {} dịch vụ đã nhận dạng",
        mapper.host_count(),
        mapper.total_open_ports(),
        mapper.total_open_ports()
    );

    // ---------------------------------------------------------------- 2. Fingerprint
    println!("\n--- 2. NHẬN DẠNG DỊCH VỤ (ServiceFingerprinter) ---");
    let fingerprinter = ServiceFingerprinter::new();
    let mut ips: Vec<IpAddr> = simulator.hosts.keys().copied().collect();
    ips.sort();
    for ip in ips {
        let host = &simulator.hosts[&ip];
        let mut ports: Vec<u16> = host.banners.keys().copied().collect();
        ports.sort_unstable();
        for port in ports {
            let banner = &host.banners[&port];
            if let Some(m) = fingerprinter.fingerprint(port, banner) {
                let version = if m.version.is_empty() {
                    "-".to_string()
                } else {
                    m.version.clone()
                };
                println!(
                    "    {ip}:{port:<5} -> {:<6} {:<26} confidence={:.2}",
                    m.service, version, m.confidence
                );
            }
        }
    }
    println!(
        "    TTL 64 -> {}, TTL 128 -> {}, TTL 255 -> {}",
        detect_os_from_ttl(64),
        detect_os_from_ttl(128),
        detect_os_from_ttl(255)
    );

    // ---------------------------------------------------------------- 3. Header IPv4
    println!("\n--- 3. HEADER IPv4 & SUY LUẬN HỆ ĐIỀU HÀNH ---");
    let ip_header = IpHeader {
        version: 4,
        ttl: 64,
        protocol: 6,
        src: Ipv4Addr::new(10, 10, 10, 5),
        dst: Ipv4Addr::new(10, 10, 10, 254),
        total_len: 60,
    };
    print!("{}", hex_dump(&ip_header.to_bytes()));
    println!(
        "    version={} ttl={} protocol={} ({}) src={} dst={} total_len={}",
        ip_header.version,
        ip_header.ttl,
        ip_header.protocol,
        ip_header.protocol_name(),
        ip_header.src,
        ip_header.dst,
        ip_header.total_len
    );
    println!(
        "    Suy luận hệ điều hành từ TTL: {}",
        detect_os_from_ttl(ip_header.ttl)
    );

    // ---------------------------------------------------------------- 4. Gói ARP
    println!("\n--- 4. GÓI ARP & ĐẦU ĐỘC BỘ NHỚ ĐỆM ARP (RFC 826) ---");
    let victim_ip = IpAddr::V4(Ipv4Addr::new(10, 10, 10, 5));
    let gateway_ip = IpAddr::V4(Ipv4Addr::new(10, 10, 10, 254));
    let request = ArpPacket {
        op: ArpOp::Request,
        sender_mac: simulated_mac(Ipv4Addr::new(10, 10, 10, 5)),
        sender_ip: Ipv4Addr::new(10, 10, 10, 5),
        target_mac: [0u8; 6],
        target_ip: Ipv4Addr::new(10, 10, 10, 254),
    };
    println!("    Yêu cầu ARP: {request}");
    print!("{}", hex_dump(&request.to_bytes()));

    let poison = simulator.arp_poison(victim_ip, gateway_ip);
    println!("\n    Gói đầu độc (ARP cache poisoning): {poison}");
    print!("{}", hex_dump(&poison.to_bytes()));
    let parsed_poison =
        ArpPacket::parse(&poison.to_bytes()).expect("Phải phân tích lại được gói ARP");
    println!(
        "    Kiểm chứng round-trip: {} byte, sender_ip={} (giả mạo cổng gateway)",
        ARP_PACKET_LEN, parsed_poison.sender_ip
    );

    // ---------------------------------------------------------------- 5. Gói DNS
    println!("\n--- 5. GÓI DNS & GIẢ MẠO PHẢN HỒI (RFC 1035) ---");
    let query = DnsPacket::build_query("mail.lab", DnsRecordType::A);
    println!(
        "    Truy vấn: id=0x{:04X} type={:?} qdcount={}",
        query.id,
        query.questions[0].qtype,
        query.questions.len()
    );
    print!("{}", hex_dump(&query.to_bytes()));

    let spoofed = DnsPacket::spoof_response(&query, Ipv4Addr::new(6, 6, 6, 6), 60);
    println!(
        "\n    Phản hồi giả mạo: is_response={} answers={} ttl={}",
        spoofed.is_response,
        spoofed.answers.len(),
        spoofed.answers[0].ttl
    );
    print!("{}", hex_dump(&spoofed.to_bytes()));
    let reparsed = DnsPacket::parse(&spoofed.to_bytes()).expect("Phải phân tích lại được gói DNS");
    println!(
        "    Kiểm chứng round-trip: {} câu hỏi, {} bản ghi, địa chỉ={:?}",
        reparsed.questions.len(),
        reparsed.answers.len(),
        reparsed.answers[0].data
    );

    // ---------------------------------------------------------------- 6. Proxy SOCKS5
    println!("\n--- 6. PROXY SOCKS5 TRÊN LOOPBACK (RFC 1928/1929) ---");
    let backend = spawn_echo_server();
    let proxy = Socks5Server::new("127.0.0.1:0".parse().expect("Địa chỉ loopback hợp lệ"));
    let listener = TcpListener::bind(proxy.bind_addr).expect("Không bind được proxy SOCKS5");
    let proxy_addr = listener
        .local_addr()
        .expect("Không lấy được địa chỉ của proxy SOCKS5");
    let proxy_thread = std::thread::spawn(move || {
        let mut stream = listener.accept().expect("Chờ kết nối SOCKS5").0;
        proxy.handle_client(&mut stream)
    });

    let mut socks_client =
        Socks5Client::connect(proxy_addr, backend, None).expect("Bắt tay SOCKS5 phải thành công");
    socks_client
        .write_all(b"xin-chao-SOCKS5")
        .expect("Ghi dữ liệu qua proxy");
    let mut echoed = [0u8; 64];
    let n = socks_client
        .read(&mut echoed)
        .expect("Đọc dữ liệu phản hồi qua proxy");
    let _ = socks_client.shutdown(Shutdown::Write);
    let session = proxy_thread
        .join()
        .expect("Luồng proxy không được panic")
        .expect("Phiên SOCKS5 phải thành công");
    println!(
        "    Client -> proxy {proxy_addr} -> đích {}",
        session.target
    );
    println!(
        "    Phản hồi echo: {:?}",
        String::from_utf8_lossy(&echoed[..n])
    );
    println!(
        "    Tổng byte relay hai chiều: {} byte (bound={})",
        session.relayed_bytes, session.bound
    );

    // ---------------------------------------------------------------- 7. Chuyển tiếp cổng
    println!("\n--- 7. CHUYỂN TIẾP CỔNG TCP HAI CHIỀU (RAII) ---");
    let backend2 = spawn_echo_server();
    let local_listener =
        TcpListener::bind("127.0.0.1:0").expect("Không bind được cổng chuyển tiếp");
    let local_addr = local_listener
        .local_addr()
        .expect("Không lấy được địa chỉ cổng chuyển tiếp");
    let forwarder = TcpPortForwarder::new(local_addr, backend2);
    let forward_thread = std::thread::spawn(move || forwarder.accept_and_forward(&local_listener));

    let mut forwarded = TcpStream::connect(local_addr).expect("Kết nối tới cổng chuyển tiếp");
    forwarded
        .write_all(b"chuyen-tiep-port")
        .expect("Ghi qua cổng chuyển tiếp");
    let mut fwd_buf = [0u8; 64];
    let fwd_n = forwarded
        .read(&mut fwd_buf)
        .expect("Đọc phản hồi qua chuyển tiếp");
    let _ = forwarded.shutdown(Shutdown::Write);
    let relayed = forward_thread
        .join()
        .expect("Luồng chuyển tiếp không được panic")
        .expect("Chuyển tiếp cổng phải thành công");
    println!("    Cổng cục bộ {local_addr} -> đích {backend2}",);
    println!(
        "    Phản hồi: {:?} | tổng byte relay: {relayed} (đọc được {fwd_n} byte)",
        String::from_utf8_lossy(&fwd_buf[..fwd_n])
    );

    // ---------------------------------------------------------------- 8. Thứ tự quét
    println!("\n--- 8. THỨ TỰ QUÉT CỔNG NGẪU NHIÊN HOÁ (SplitMix64) ---");
    let mut sim = NetworkSimulator::new();
    let mut scan_order = Vec::new();
    while let Some(ip) = sim.next_scan_ip() {
        scan_order.push(ip.to_string());
        if scan_order.len() >= 8 {
            break;
        }
    }
    let rng = SplitMix64::new(0xC0FFEE);
    let ports: Vec<u16> = vec![21, 22, 25, 80, 110, 143, 389, 445, 3306, 6379];
    println!("    Thứ tự IP quét: {}", scan_order.join(" -> "));
    println!(
        "    Thứ tự cổng quét (Fisher-Yates tất định): {:?}",
        rng.shuffled_ports(&ports)
    );

    println!(
        "\n=== HOÀN TẤT CHƯƠNG 99: toàn bộ thành phần đã được minh hoạ trên dữ liệu tổng hợp ==="
    );
}

// ============================================================================
// 12. BỘ KIỂM THỬ TẤT ĐỊNH (42 BÀI TEST CHỨNG MINH CÁC BẤT BIẾN)
// ============================================================================

#[cfg(test)]
mod tests {
    use super::*;

    /// Bộ phân giải tên miền dùng cho kiểm thử: ánh xạ "lab.internal" về loopback
    fn test_resolver(host: &str, port: u16) -> Option<SocketAddr> {
        if host == "lab.internal" {
            Some(SocketAddr::new(IpAddr::V4(Ipv4Addr::LOCALHOST), port))
        } else {
            None
        }
    }

    /// Gửi greeting SOCKS5 thô và đọc lại 2 byte phản hồi
    fn raw_greet(stream: &mut TcpStream, methods: &[u8]) -> std::io::Result<[u8; 2]> {
        let mut greeting = vec![0x05, methods.len() as u8];
        greeting.extend_from_slice(methods);
        stream.write_all(&greeting)?;
        let mut resp = [0u8; 2];
        stream.read_exact(&mut resp)?;
        Ok(resp)
    }

    /// Tạo cặp socket đã kết nối trực tiếp với nhau trên loopback
    fn connected_pair() -> (TcpStream, TcpStream) {
        let listener = TcpListener::bind("127.0.0.1:0").expect("Không bind được cổng loopback");
        let addr = listener
            .local_addr()
            .expect("Không lấy được địa chỉ cổng loopback");
        let client = TcpStream::connect(addr).expect("Không kết nối được tới cổng loopback");
        let (peer, _) = listener
            .accept()
            .expect("Không chấp nhận được kết nối loopback");
        (client, peer)
    }

    // ------------------------------------------------------------------
    // NHÓM 1: BỘ SINH SỒ NGẪU NHIÊN TẤT ĐỊNH (TEST 1 - 2)
    // ------------------------------------------------------------------

    #[test]
    fn test_01_splitmix64_deterministic_reproducibility() {
        // BẤT BIẾN: Cùng hạt giống ⇒ chuỗi số giống hệt nhau trên mọi lần chạy
        let mut a = SplitMix64::new(0xDEAD_BEEF);
        let mut b = SplitMix64::new(0xDEAD_BEEF);
        for _ in 0..64 {
            assert_eq!(a.next_u64(), b.next_u64());
        }
        let mut c = SplitMix64::new(1);
        let mut d = SplitMix64::new(2);
        assert_ne!(c.next_u64(), d.next_u64());
    }

    #[test]
    fn test_02_splitmix64_f64_range_and_port_shuffle_permutation() {
        // BẤT BIẾN: f64 luôn nằm trong [0,1) và hoán vị cổng vẫn là một hoán vị
        let mut rng = SplitMix64::new(99);
        for _ in 0..1000 {
            let value = rng.next_f64();
            assert!(
                (0.0..1.0).contains(&value),
                "f64 ngoài khoảng [0,1): {value}"
            );
        }
        let ports: Vec<u16> = (1..=20).collect();
        let shuffled = rng.shuffled_ports(&ports);
        assert_eq!(shuffled.len(), ports.len());
        let mut sorted = shuffled.clone();
        sorted.sort_unstable();
        assert_eq!(sorted, ports, "Hoán vị cổng phải giữ nguyên tập cổng");
    }

    // ------------------------------------------------------------------
    // NHÓM 2: PROXY SOCKS5 (TEST 3 - 11)
    // ------------------------------------------------------------------

    #[test]
    fn test_03_socks5_no_auth_handshake_relays_payload() {
        // BẤT BIẾN: Bắt tay SOCKS5 không xác thực thành công và relay hai chiều trọn vẹn
        let backend = spawn_echo_server();
        let proxy = Socks5Server::new("127.0.0.1:0".parse().unwrap());
        let listener = TcpListener::bind(proxy.bind_addr).unwrap();
        let proxy_addr = listener.local_addr().unwrap();

        let handle = std::thread::spawn(move || {
            let mut stream = listener.accept().unwrap().0;
            proxy.handle_client(&mut stream)
        });

        let mut client =
            Socks5Client::connect(proxy_addr, backend, None).expect("Bắt tay thất bại");
        client.write_all(b"hello-socks5").unwrap();
        let mut buf = [0u8; 32];
        client.read_exact(&mut buf[..12]).unwrap();
        assert_eq!(&buf[..12], b"hello-socks5");
        let _ = client.shutdown(Shutdown::Write);

        let session = handle
            .join()
            .unwrap()
            .expect("Phiên SOCKS5 phải thành công");
        assert_eq!(session.target, backend);
        assert_eq!(session.relayed_bytes, 24, "12 byte mỗi chiều");
    }

    #[test]
    fn test_04_socks5_auth_accepts_valid_credentials() {
        // BẤT BIẾN: Thông tin đăng nhập đúng được chấp nhận (RFC 1929)
        let backend = spawn_echo_server();
        let proxy = Socks5Server::new("127.0.0.1:0".parse().unwrap())
            .with_auth(vec![("scanner".to_string(), "s3cret".to_string())]);
        let listener = TcpListener::bind(proxy.bind_addr).unwrap();
        let proxy_addr = listener.local_addr().unwrap();

        let handle = std::thread::spawn(move || {
            let mut stream = listener.accept().unwrap().0;
            proxy.handle_client(&mut stream)
        });

        let mut client = Socks5Client::connect(
            proxy_addr,
            backend,
            Some(("scanner".to_string(), "s3cret".to_string())),
        )
        .expect("Xác thực đúng phải thành công");
        client.write_all(b"auth-ok").unwrap();
        let mut buf = [0u8; 16];
        client.read_exact(&mut buf[..7]).unwrap();
        assert_eq!(&buf[..7], b"auth-ok");
        let _ = client.shutdown(Shutdown::Write);

        let session = handle
            .join()
            .unwrap()
            .expect("Phiên SOCKS5 phải thành công");
        assert_eq!(session.target, backend);
        assert!(session.relayed_bytes > 0);
    }

    #[test]
    fn test_05_socks5_auth_rejects_invalid_password() {
        // BẤT BIẾN: Mật khẩu sai ⇒ máy chủ từ chối và báo lỗi tiếng Việt
        spawn_echo_server();
        let proxy = Socks5Server::new("127.0.0.1:0".parse().unwrap())
            .with_auth(vec![("scanner".to_string(), "s3cret".to_string())]);
        let listener = TcpListener::bind(proxy.bind_addr).unwrap();
        let proxy_addr = listener.local_addr().unwrap();

        let handle = std::thread::spawn(move || {
            let mut stream = listener.accept().unwrap().0;
            proxy.handle_client(&mut stream)
        });

        let mut client = TcpStream::connect(proxy_addr).unwrap();
        let resp = raw_greet(&mut client, &[0x02]).unwrap();
        assert_eq!(resp[1], 0x02, "Máy chủ phải chọn phương thức user/pass");

        let user = b"scanner";
        let pass = b"wrong";
        let mut sub = vec![0x01, user.len() as u8];
        sub.extend_from_slice(user);
        sub.push(pass.len() as u8);
        sub.extend_from_slice(pass);
        client.write_all(&sub).unwrap();

        let mut auth_resp = [0u8; 2];
        client.read_exact(&mut auth_resp).unwrap();
        assert_eq!(auth_resp[1], 0x01, "Trạng thái xác thực phải là thất bại");

        let error = handle
            .join()
            .unwrap()
            .expect_err("Phải từ chối đăng nhập sai");
        assert!(error.contains("không hợp lệ"), "Thông điệp lỗi: {error}");
    }

    #[test]
    fn test_06_socks5_rejects_invalid_protocol_version() {
        // BẤT BIẾN: Phiên bản SOCKS khác 0x05 bị từ chối ngay ở bước greeting
        let proxy = Socks5Server::new("127.0.0.1:0".parse().unwrap());
        let listener = TcpListener::bind(proxy.bind_addr).unwrap();
        let proxy_addr = listener.local_addr().unwrap();

        let handle = std::thread::spawn(move || {
            let mut stream = listener.accept().unwrap().0;
            proxy.handle_client(&mut stream)
        });

        let mut client = TcpStream::connect(proxy_addr).unwrap();
        client.write_all(&[0x04, 0x01, 0x00]).unwrap();
        let error = handle
            .join()
            .unwrap()
            .expect_err("Phải từ chối phiên bản sai");
        assert!(
            error.contains("Phiên bản SOCKS không hợp lệ"),
            "Lỗi: {error}"
        );
    }

    #[test]
    fn test_07_socks5_rejects_non_connect_command() {
        // BẤT BIẾN: Chỉ lệnh CONNECT (0x01) được hỗ trợ
        let proxy = Socks5Server::new("127.0.0.1:0".parse().unwrap());
        let listener = TcpListener::bind(proxy.bind_addr).unwrap();
        let proxy_addr = listener.local_addr().unwrap();

        let handle = std::thread::spawn(move || {
            let mut stream = listener.accept().unwrap().0;
            proxy.handle_client(&mut stream)
        });

        let mut client = TcpStream::connect(proxy_addr).unwrap();
        raw_greet(&mut client, &[0x00]).unwrap();
        // BIND (0x02) tới 127.0.0.1:80
        client
            .write_all(&[0x05, 0x02, 0x00, 0x01, 127, 0, 0, 1, 0, 80])
            .unwrap();
        let error = handle
            .join()
            .unwrap()
            .expect_err("Phải từ chối lệnh khác CONNECT");
        assert!(error.contains("CONNECT"), "Lỗi: {error}");
    }

    #[test]
    fn test_08_socks5_rejects_ipv6_address_type() {
        // BẤT BIẾN: ATYP = 0x04 (IPv6) chưa được hỗ trợ và phải báo lỗi rõ ràng
        let proxy = Socks5Server::new("127.0.0.1:0".parse().unwrap());
        let listener = TcpListener::bind(proxy.bind_addr).unwrap();
        let proxy_addr = listener.local_addr().unwrap();

        let handle = std::thread::spawn(move || {
            let mut stream = listener.accept().unwrap().0;
            proxy.handle_client(&mut stream)
        });

        let mut client = TcpStream::connect(proxy_addr).unwrap();
        raw_greet(&mut client, &[0x00]).unwrap();
        let mut request = vec![0x05, 0x01, 0x00, 0x04];
        request.extend_from_slice(&[0u8; 16]);
        request.extend_from_slice(&80u16.to_be_bytes());
        client.write_all(&request).unwrap();

        let error = handle.join().unwrap().expect_err("Phải từ chối IPv6");
        assert!(error.contains("IPv6"), "Lỗi: {error}");
    }

    #[test]
    fn test_09_socks5_resolves_domain_name_via_resolver() {
        // BẤT BIẾN: ATYP = 0x03 (tên miền) được phân giải qua resolver tùy biến
        let backend = spawn_echo_server();
        let proxy = Socks5Server::new("127.0.0.1:0".parse().unwrap());
        let listener = TcpListener::bind(proxy.bind_addr).unwrap();
        let proxy_addr = listener.local_addr().unwrap();

        let handle = std::thread::spawn(move || {
            let mut stream = listener.accept().unwrap().0;
            proxy.handle_client_with_target(&mut stream, &test_resolver)
        });

        let mut client = TcpStream::connect(proxy_addr).unwrap();
        raw_greet(&mut client, &[0x00]).unwrap();
        let mut request = vec![0x05, 0x01, 0x00, 0x03];
        request.push("lab.internal".len() as u8);
        request.extend_from_slice(b"lab.internal");
        request.extend_from_slice(&backend.port().to_be_bytes());
        client.write_all(&request).unwrap();

        let mut reply = [0u8; 10];
        client.read_exact(&mut reply).unwrap();
        assert_eq!(reply[1], 0x00, "Phản hồi CONNECT phải là thành công");

        client.write_all(b"domain").unwrap();
        let mut buf = [0u8; 16];
        client.read_exact(&mut buf[..6]).unwrap();
        assert_eq!(&buf[..6], b"domain");
        let _ = client.shutdown(Shutdown::Write);

        let session = handle
            .join()
            .unwrap()
            .expect("Phiên SOCKS5 phải thành công");
        assert_eq!(session.target, backend);
        assert_eq!(session.relayed_bytes, 12);
    }

    #[test]
    fn test_10_socks5_rejects_unknown_auth_method() {
        // BẤT BIẾN: Client không đưa ra phương thức xác thực nào được hỗ trợ ⇒ bị từ chối
        let proxy = Socks5Server::new("127.0.0.1:0".parse().unwrap());
        let listener = TcpListener::bind(proxy.bind_addr).unwrap();
        let proxy_addr = listener.local_addr().unwrap();

        let handle = std::thread::spawn(move || {
            let mut stream = listener.accept().unwrap().0;
            proxy.handle_client(&mut stream)
        });

        let mut client = TcpStream::connect(proxy_addr).unwrap();
        raw_greet(&mut client, &[0x80]).unwrap();
        let error = handle
            .join()
            .unwrap()
            .expect_err("Phải từ chối phương thức lạ");
        assert!(
            error.contains("phương thức xác thực") || error.contains("xác thực"),
            "Lỗi: {error}"
        );
    }

    #[test]
    fn test_11_socks5_session_reports_target_and_bound_addresses() {
        // BẤT BIẾN: Session trả về đúng địa chỉ đích và địa chỉ cục bộ đã bind
        let backend = spawn_echo_server();
        let proxy = Socks5Server::new("127.0.0.1:0".parse().unwrap());
        let listener = TcpListener::bind(proxy.bind_addr).unwrap();
        let proxy_addr = listener.local_addr().unwrap();

        let handle = std::thread::spawn(move || {
            let mut stream = listener.accept().unwrap().0;
            proxy.handle_client(&mut stream)
        });

        let client = Socks5Client::connect(proxy_addr, backend, None).unwrap();
        let _ = client.shutdown(Shutdown::Write);
        let session = handle
            .join()
            .unwrap()
            .expect("Phiên SOCKS5 phải thành công");

        assert_eq!(session.target, backend);
        assert!(
            session.bound.ip().is_loopback(),
            "BND.ADDR phải là loopback"
        );
        assert_ne!(
            session.bound.port(),
            0,
            "BND.PORT phải là cổng cục bộ đã bind"
        );
        assert_eq!(session.target.port(), backend.port());
    }

    // ------------------------------------------------------------------
    // NHÓM 3: CHUYỂN TIẾP CỔNG TCP (TEST 12 - 15)
    // ------------------------------------------------------------------

    #[test]
    fn test_12_port_forwarder_relays_bidirectional_traffic() {
        // BẤT BIẾN: Dữ liệu đi qua forwarder tới đích và quay lại nguyên vẹn
        let backend = spawn_echo_server();
        let listener = TcpListener::bind("127.0.0.1:0").unwrap();
        let local_addr = listener.local_addr().unwrap();
        let forwarder = TcpPortForwarder::new(local_addr, backend);

        let handle = std::thread::spawn(move || forwarder.accept_and_forward(&listener));

        let mut client = TcpStream::connect(local_addr).unwrap();
        client.write_all(b"forward-12345").unwrap();
        let mut buf = [0u8; 32];
        client.read_exact(&mut buf[..13]).unwrap();
        assert_eq!(&buf[..13], b"forward-12345");
        let _ = client.shutdown(Shutdown::Write);

        let total = handle.join().unwrap().expect("Chuyển tiếp phải thành công");
        assert_eq!(total, 26, "13 byte mỗi chiều");
    }

    #[test]
    fn test_13_port_forwarder_shutdown_clears_active_flag() {
        // BẤT BIẾN: Sau khi shutdown, cờ active = false và forwarder không còn chuyển tiếp
        let backend = spawn_echo_server();
        let listener = TcpListener::bind("127.0.0.1:0").unwrap();
        let local_addr = listener.local_addr().unwrap();
        let mut forwarder = TcpPortForwarder::new(local_addr, backend);
        assert!(forwarder.active);

        forwarder.shutdown();
        assert!(!forwarder.active, "shutdown() phải tắt cờ active");

        let error = forwarder
            .accept_and_forward(&listener)
            .expect_err("Forwarder đã tắt không được phục vụ");
        assert_eq!(error.kind(), std::io::ErrorKind::BrokenPipe);
    }

    #[test]
    fn test_14_port_forwarder_drop_is_raii_safe() {
        // BẤT BIẾN RAII: Thả forwarder không gây panic và forwarder mới hoạt động độc lập
        let backend = spawn_echo_server();
        let listener = TcpListener::bind("127.0.0.1:0").unwrap();
        let local_addr = listener.local_addr().unwrap();

        let forwarder = TcpPortForwarder::new(local_addr, backend);
        assert!(forwarder.active);
        drop(forwarder); // RAII: Drop chạy, không panic

        let fresh = TcpPortForwarder::new(local_addr, backend);
        assert!(
            fresh.active,
            "Forwarder mới phải hoạt động độc lập với forwarder đã thả"
        );
    }

    #[test]
    fn test_15_bidirectional_relay_counts_total_bytes() {
        // BẤT BIẾN: Tổng số byte relay = tổng byte của cả hai chiều
        let backend = spawn_echo_server();
        let (relay_side, app_side) = connected_pair();
        let mut relay_side = relay_side;
        let mut app_side = app_side;
        let mut to_backend = TcpStream::connect(backend).expect("Không kết nối được tới backend");

        // Relay chạy trong luồng riêng với hai socket riêng của nó
        let relay_thread =
            std::thread::spawn(move || bidirectional_relay(&mut relay_side, &mut to_backend));

        let payload = b"0123456789";
        app_side.write_all(payload).unwrap();
        let mut buf = [0u8; 16];
        app_side.read_exact(&mut buf[..10]).unwrap();
        assert_eq!(&buf[..10], payload);

        // Đóng chiều ghi để relay nhận EOF và kết thúc cả hai chiều
        let _ = app_side.shutdown(Shutdown::Write);
        let relayed = relay_thread.join().unwrap().unwrap();
        assert_eq!(
            relayed,
            payload.len() * 2,
            "Mỗi chiều phải truyền đủ 10 byte"
        );
    }

    // ------------------------------------------------------------------
    // NHÓM 4: NHẬN DẠNG DỊCH VỤ (TEST 16 - 23)
    // ------------------------------------------------------------------

    #[test]
    fn test_16_fingerprint_detects_http_server_banner() {
        // BẤT BIẾN: Banner HTTP được nhận dạng đúng dịch vụ + phiên bản
        let fp = ServiceFingerprinter::new();
        let banner = b"HTTP/1.1 200 OK\r\nServer: nginx/1.18.0 (Ubuntu)\r\n\r\n";
        let found = fp
            .fingerprint(80, banner)
            .expect("Phải nhận dạng được HTTP");
        assert_eq!(found.service, "http");
        assert_eq!(found.version, "nginx/1.18.0");
        assert!(
            found.confidence > 0.9,
            "Độ tin cậy thấp: {}",
            found.confidence
        );
    }

    #[test]
    fn test_17_fingerprint_detects_ssh_banner_with_version() {
        // BẤT BIẾN: Banner SSH-2.0-OpenSSH được nhận dạng kèm phiên bản đầy đủ
        let fp = ServiceFingerprinter::new();
        let banner = b"SSH-2.0-OpenSSH_8.9p1 Ubuntu-3ubuntu0.4\r\n";
        let found = fp.fingerprint(22, banner).expect("Phải nhận dạng được SSH");
        assert_eq!(found.service, "ssh");
        assert_eq!(found.version, "SSH-2.0-OpenSSH_8.9p1");
        assert!(found.confidence >= 0.9);
    }

    #[test]
    fn test_18_fingerprint_detects_mysql_greeting_version() {
        // BẤT BIẾN: Greeting MySQL được nhận dạng kèm chuỗi phiên bản 8.0.32
        let fp = ServiceFingerprinter::new();
        let banner = mysql_greeting("8.0.32-0ubuntu0.2");
        let found = fp
            .fingerprint(3306, &banner)
            .expect("Phải nhận dạng được MySQL");
        assert_eq!(found.service, "mysql");
        assert_eq!(found.version, "8.0.32-0ubuntu0.2");
    }

    #[test]
    fn test_19_fingerprint_detects_smb_header() {
        // BẤT BIẾN: Header SMB (0xFF 'S' 'M' 'B') được nhận dạng là SMB phiên bản 1
        let fp = ServiceFingerprinter::new();
        let banner = smb_banner();
        let found = fp
            .fingerprint(445, &banner)
            .expect("Phải nhận dạng được SMB");
        assert_eq!(found.service, "smb");
        assert_eq!(found.version, "SMBv1");
    }

    #[test]
    fn test_20_fingerprint_detects_redis_and_rejects_unknown() {
        // BẤT BIẾN: "+PONG" ⇒ redis; dữ liệu rác ⇒ không nhận dạng được gì
        let fp = ServiceFingerprinter::new();
        let found = fp
            .fingerprint(6379, b"+PONG\r\n")
            .expect("Phải nhận dạng được Redis");
        assert_eq!(found.service, "redis");
        assert!(found.confidence > 0.5);

        assert!(fp.fingerprint(9999, b"\x01\x02\x03\x04\x05").is_none());
    }

    #[test]
    fn test_21_service_catalog_covers_expected_ports() {
        // BẤT BIẾN: Danh mục probe chứa đủ 10 dịch vụ và các cổng chuẩn tương ứng
        let catalog = service_catalog();
        assert_eq!(catalog.len(), 10);
        let ports: Vec<u16> = catalog.iter().map(|p| p.port).collect();
        for expected in [21u16, 22, 25, 80, 110, 143, 389, 445, 3306, 6379] {
            assert!(ports.contains(&expected), "Thiếu probe cho cổng {expected}");
        }
        let fp = ServiceFingerprinter::new();
        assert_eq!(fp.probes.len(), catalog.len());
    }

    #[test]
    fn test_22_detect_os_from_ttl_signature() {
        // BẤT BIẾN: TTL chuẩn ánh xạ đúng sang hệ điều hành
        assert_eq!(detect_os_from_ttl(64), OperatingSystem::Linux);
        assert_eq!(detect_os_from_ttl(128), OperatingSystem::Windows);
        assert_eq!(detect_os_from_ttl(255), OperatingSystem::NetworkDevice);
        assert_eq!(detect_os_from_ttl(32), OperatingSystem::Unknown);
        assert_eq!(detect_os_from_ttl(0), OperatingSystem::Unknown);
    }

    #[test]
    fn test_23_detect_os_from_tcp_window_signature() {
        // BẤT BIẾN: Kích thước cửa sổ TCP chuẩn ánh xạ đúng sang hệ điều hành
        assert_eq!(detect_os_from_window(65535), OperatingSystem::Windows);
        assert_eq!(detect_os_from_window(8192), OperatingSystem::Linux);
        assert_eq!(detect_os_from_window(29200), OperatingSystem::Linux);
        assert_eq!(detect_os_from_window(16384), OperatingSystem::Unknown);
    }

    // ------------------------------------------------------------------
    // NHÓM 5: BẢN ĐỒ MẠNG (TEST 24 - 27)
    // ------------------------------------------------------------------

    #[test]
    fn test_24_network_mapper_add_host_counts() {
        // BẤT BIẾN: Mỗi IP chỉ được đếm một lần trong bản đồ
        let mut mapper = NetworkMapper::new();
        let ip = IpAddr::V4(Ipv4Addr::new(10, 0, 0, 1));
        mapper.add_host(ip, Some("alpha.lab".to_string()));
        mapper.add_host(ip, None);
        assert_eq!(mapper.host_count(), 1);
        assert_eq!(
            mapper.hosts[&ip].hostname.as_deref(),
            Some("alpha.lab"),
            "Hostname đã biết không được ghi đè bằng None"
        );
    }

    #[test]
    fn test_25_network_mapper_record_service_tracks_open_ports() {
        // BẤT BIẾN: Mỗi cổng chỉ xuất hiện đúng một lần trong danh sách cổng mở
        let mut mapper = NetworkMapper::new();
        let ip = IpAddr::V4(Ipv4Addr::new(10, 0, 0, 2));
        mapper.record_service(ip, 80, "http".to_string());
        mapper.record_service(ip, 80, "http".to_string());
        mapper.record_service(ip, 22, "ssh".to_string());
        assert_eq!(mapper.hosts[&ip].open_ports.len(), 2);
        assert_eq!(mapper.hosts[&ip].services.get(&80).unwrap(), "http");
        assert_eq!(mapper.hosts[&ip].services.get(&22).unwrap(), "ssh");
    }

    #[test]
    fn test_26_network_mapper_report_lists_services() {
        // BẤT BIẾN: Báo cáo chứa địa chỉ, hostname và tên dịch vụ của từng máy
        let mut mapper = NetworkMapper::new();
        let ip = IpAddr::V4(Ipv4Addr::new(10, 0, 0, 3));
        mapper.add_host(ip, Some("gamma.lab".to_string()));
        mapper.record_os(ip, OperatingSystem::Linux);
        mapper.record_service(ip, 443, "https".to_string());

        let report = mapper.to_report();
        assert!(report.contains("10.0.0.3"), "Báo cáo thiếu IP: {report}");
        assert!(
            report.contains("gamma.lab"),
            "Báo cáo thiếu hostname: {report}"
        );
        assert!(report.contains("https"), "Báo cáo thiếu dịch vụ: {report}");
        assert!(
            report.contains("Linux"),
            "Báo cáo thiếu hệ điều hành: {report}"
        );
    }

    #[test]
    fn test_27_network_mapper_total_open_ports_accumulates() {
        // BẤT BIẾN: Tổng cổng mở = tổng số cổng của mọi máy trong bản đồ
        let mut mapper = NetworkMapper::new();
        let a = IpAddr::V4(Ipv4Addr::new(10, 0, 0, 4));
        let b = IpAddr::V4(Ipv4Addr::new(10, 0, 0, 5));
        mapper.record_service(a, 80, "http".to_string());
        mapper.record_service(a, 22, "ssh".to_string());
        mapper.record_service(b, 445, "smb".to_string());
        assert_eq!(mapper.host_count(), 2);
        assert_eq!(mapper.total_open_ports(), 3);
    }

    // ------------------------------------------------------------------
    // NHÓM 6: GÓI ARP & ĐẦU ĐỘC (TEST 28 - 32)
    // ------------------------------------------------------------------

    #[test]
    fn test_28_arp_packet_roundtrip_bytes() {
        // BẤT BIẾN: Mã hoá rồi phân tích lại cho đúng gói ARP ban đầu
        let packet = ArpPacket {
            op: ArpOp::Request,
            sender_mac: [0x00, 0x11, 0x22, 0x33, 0x44, 0x55],
            sender_ip: Ipv4Addr::new(192, 168, 1, 10),
            target_mac: [0u8; 6],
            target_ip: Ipv4Addr::new(192, 168, 1, 1),
        };
        let bytes = packet.to_bytes();
        assert_eq!(bytes.len(), ARP_PACKET_LEN);
        let parsed = ArpPacket::parse(&bytes).expect("Phải phân tích lại được gói ARP");
        assert_eq!(parsed, packet);
    }

    #[test]
    fn test_29_arp_parse_skips_ethernet_header() {
        // BẤT BIẾN: Khung Ethernet đầy đủ (14 byte header) được bỏ qua đúng
        let packet = ArpPacket {
            op: ArpOp::Reply,
            sender_mac: [0xAA, 0xBB, 0xCC, 0xDD, 0xEE, 0xFF],
            sender_ip: Ipv4Addr::new(10, 0, 0, 1),
            target_mac: [0x11, 0x22, 0x33, 0x44, 0x55, 0x66],
            target_ip: Ipv4Addr::new(10, 0, 0, 2),
        };
        let mut frame = vec![0xFF; 6]; // MAC đích
        frame.extend_from_slice(&[0x00; 6]); // MAC nguồn
        frame.extend_from_slice(&0x0806u16.to_be_bytes()); // EtherType ARP
        frame.extend_from_slice(&packet.to_bytes());

        let parsed = ArpPacket::parse(&frame).expect("Phải bỏ qua header Ethernet");
        assert_eq!(parsed, packet);
    }

    #[test]
    fn test_30_arp_parse_rejects_invalid_opcode() {
        // BẤT BIẾN: Mã thao tác ARP không hợp lệ bị từ chối bằng thông điệp tiếng Việt
        let mut bytes = ArpPacket {
            op: ArpOp::Request,
            sender_mac: [0u8; 6],
            sender_ip: Ipv4Addr::UNSPECIFIED,
            target_mac: [0u8; 6],
            target_ip: Ipv4Addr::UNSPECIFIED,
        }
        .to_bytes();
        bytes[6] = 0x00;
        bytes[7] = 0x09;
        let error = ArpPacket::parse(&bytes).expect_err("Phải từ chối opcode sai");
        assert!(
            error.contains("Mã thao tác ARP không hợp lệ"),
            "Lỗi: {error}"
        );
    }

    #[test]
    fn test_31_arp_spoof_reply_claims_attacker_address() {
        // BẤT BIẾN: Gói đầu độc khai báo IP của kẻ tấn công và nhắm vào nạn nhân
        let request = ArpPacket {
            op: ArpOp::Request,
            sender_mac: [0x00, 0x0C, 0x29, 0x11, 0x22, 0x33],
            sender_ip: Ipv4Addr::new(10, 10, 10, 5),
            target_mac: [0u8; 6],
            target_ip: Ipv4Addr::new(10, 10, 10, 254),
        };
        let attacker_mac = [0xDE, 0xAD, 0xBE, 0xEF, 0x00, 0x01];
        let attacker_ip = Ipv4Addr::new(10, 10, 10, 254);

        let poison = ArpPacket::spoof_reply_for(&request, attacker_mac, attacker_ip);
        assert_eq!(poison.op, ArpOp::Reply);
        assert_eq!(poison.sender_mac, attacker_mac);
        assert_eq!(poison.sender_ip, attacker_ip);
        assert_eq!(poison.target_mac, request.sender_mac);
        assert_eq!(poison.target_ip, request.sender_ip);
    }

    #[test]
    fn test_32_mac_to_string_canonical_format() {
        // BẤT BIẾN: Định dạng MAC luôn là 6 cặp hex phân tách bằng dấu hai chấm
        let mac = [0x00u8, 0x1A, 0x2B, 0x3C, 0x4D, 0x5E];
        assert_eq!(mac_to_string(&mac), "00:1a:2b:3c:4d:5e");
        assert_eq!(mac_to_string(&[0u8; 6]), "00:00:00:00:00:00");
        assert_eq!(mac_to_string(&mac).split(':').count(), 6);
    }

    // ------------------------------------------------------------------
    // NHÓM 7: GÓI DNS & GIẢ MẠO (TEST 33 - 40)
    // ------------------------------------------------------------------

    #[test]
    fn test_33_dns_build_query_structure() {
        // BẤT BIẾN: Truy vấn DNS có đúng 1 câu hỏi, là_response = false và mã định danh tất định
        let query = DnsPacket::build_query("mail.lab", DnsRecordType::A);
        assert!(!query.is_response);
        assert_eq!(query.questions.len(), 1);
        assert_eq!(query.questions[0].name, "mail.lab");
        assert_eq!(query.questions[0].qtype, DnsRecordType::A);
        assert_eq!(query.questions[0].qclass, 1);
        assert!(query.answers.is_empty());
        assert_eq!(query.id, query_id_for("mail.lab", DnsRecordType::A));
    }

    #[test]
    fn test_34_dns_query_bytes_roundtrip() {
        // BẤT BIẾN: Mã hoá rồi phân tích lại gói truy vấn cho đúng nội dung ban đầu
        let query = DnsPacket::build_query("www.example.com", DnsRecordType::A);
        let bytes = query.to_bytes();
        let parsed = DnsPacket::parse(&bytes).expect("Phải phân tích lại được gói DNS");
        assert_eq!(parsed.id, query.id);
        assert!(!parsed.is_response);
        assert_eq!(parsed.questions.len(), 1);
        assert_eq!(parsed.questions[0].name, "www.example.com");
        assert_eq!(parsed.questions[0].qtype, DnsRecordType::A);
    }

    #[test]
    fn test_35_dns_encode_decode_name_labels() {
        // BẤT BIẾN: Tên miền nhiều nhãn được mã hoá/giải mã không mất thông tin
        let encoded = encode_name("api.v2.example.com");
        assert_eq!(encoded[0], 3, "Nhãn đầu phải dài 3 byte");
        assert_eq!(*encoded.last().unwrap(), 0, "Phải có byte kết thúc");
        let (name, next) = decode_name(&encoded, 0).expect("Giải mã tên miền phải thành công");
        assert_eq!(name, "api.v2.example.com");
        assert_eq!(next, encoded.len());

        let (root, _) = decode_name(&encode_name("."), 0).unwrap();
        assert_eq!(root, ".");
    }

    #[test]
    fn test_36_dns_decode_name_follows_compression_pointer() {
        // BẤT BIẾN: Con trỏ nén (RFC 1035) được giải mã đúng và offset trả về là vị trí sau con trỏ
        let mut bytes: Vec<u8> = Vec::new();
        bytes.extend_from_slice(&encode_name("www.example.com"));
        bytes.extend_from_slice(&[0xC0, 0x00]); // con trỏ trỏ về offset 0
        let (name, next) = decode_name(&bytes, bytes.len() - 2).expect("Phải theo con trỏ nén");
        assert_eq!(name, "www.example.com");
        assert_eq!(next, bytes.len());
    }

    #[test]
    fn test_37_dns_build_response_preserves_query_id() {
        // BẤT BIẾN: Phản hồi giữ nguyên mã định danh và câu hỏi của truy vấn gốc
        let query = DnsPacket::build_query("shop.lab", DnsRecordType::A);
        let answer = DnsRecord {
            name: "shop.lab".to_string(),
            rtype: DnsRecordType::A,
            ttl: 300,
            data: DnsRecordData::IPv4(Ipv4Addr::new(10, 10, 10, 9)),
        };
        let response = DnsPacket::build_response(&query, vec![answer.clone()]);
        assert!(response.is_response);
        assert_eq!(response.id, query.id);
        assert_eq!(response.questions, query.questions);
        assert_eq!(response.answers, vec![answer]);
    }

    #[test]
    fn test_38_dns_spoof_response_injects_a_record() {
        // BẤT BIẾN: Phản hồi giả mạo chèn đúng một bản ghi A với địa chỉ & TTL yêu cầu
        let query = DnsPacket::build_query("bank.lab", DnsRecordType::A);
        let spoofed = DnsPacket::spoof_response(&query, Ipv4Addr::new(6, 6, 6, 6), 42);
        assert!(spoofed.is_response);
        assert_eq!(spoofed.id, query.id);
        assert_eq!(spoofed.answers.len(), 1);
        assert_eq!(spoofed.answers[0].name, "bank.lab");
        assert_eq!(spoofed.answers[0].rtype, DnsRecordType::A);
        assert_eq!(spoofed.answers[0].ttl, 42);
        assert_eq!(
            spoofed.answers[0].data,
            DnsRecordData::IPv4(Ipv4Addr::new(6, 6, 6, 6))
        );
    }

    #[test]
    fn test_39_dns_response_bytes_roundtrip() {
        // BẤT BIẾN: Gói phản hồi có bản ghi A mã hoá rồi giải mã cho đúng dữ liệu
        let query = DnsPacket::build_query("target.lab", DnsRecordType::A);
        let spoofed = DnsPacket::spoof_response(&query, Ipv4Addr::new(192, 168, 0, 77), 300);
        let parsed =
            DnsPacket::parse(&spoofed.to_bytes()).expect("Phải phân tích lại được gói DNS");
        assert!(parsed.is_response);
        assert_eq!(parsed.answers.len(), 1);
        assert_eq!(parsed.answers[0].ttl, 300);
        assert_eq!(
            parsed.answers[0].data,
            DnsRecordData::IPv4(Ipv4Addr::new(192, 168, 0, 77))
        );
    }

    #[test]
    fn test_40_dns_parse_rejects_truncated_buffer() {
        // BẤT BIẾN: Gói DNS ngắn hơn 12 byte bị từ chối
        let error = DnsPacket::parse(&[0u8; 5]).expect_err("Phải từ chối gói quá ngắn");
        assert!(error.contains("quá ngắn"), "Lỗi: {error}");
        assert!(DnsPacket::parse(&[]).is_err());
    }

    // ------------------------------------------------------------------
    // NHÓM 8: HEADER IPv4 & MÔ PHỎNG MẠNG (TEST 41 - 42)
    // ------------------------------------------------------------------

    #[test]
    fn test_41_ip_header_roundtrip_and_os_guess() {
        // BẤT BIẾN: Header IPv4 mã hoá rồi giải mã cho đúng, và TTL suy luận đúng HĐH
        let header = IpHeader {
            version: 4,
            ttl: 64,
            protocol: 6,
            src: Ipv4Addr::new(10, 10, 10, 5),
            dst: Ipv4Addr::new(10, 10, 10, 254),
            total_len: 60,
        };
        let bytes = header.to_bytes();
        assert_eq!(bytes.len(), 20);
        let parsed = IpHeader::parse(&bytes).expect("Phải phân tích lại được header IPv4");
        assert_eq!(parsed, header);
        assert_eq!(detect_os_from_ttl(parsed.ttl), OperatingSystem::Linux);
        assert_eq!(parsed.protocol_name(), "TCP");
        assert!(IpHeader::parse(&[0u8; 10]).is_err());
    }

    #[test]
    fn test_42_network_simulator_map_all_and_arp_poison() {
        // BẤT BIẾN: Mô phỏng dựng đúng 5 máy chủ, dịch vụ được nhận dạng và gói đầu độc hợp lệ
        let simulator = NetworkSimulator::new();
        assert_eq!(simulator.hosts.len(), 5);

        let mapper = simulator.map_all();
        assert_eq!(mapper.host_count(), 5);
        assert!(mapper.total_open_ports() >= 12);

        let web = simulator.scan(IpAddr::V4(Ipv4Addr::new(10, 10, 10, 5)));
        assert_eq!(web.services.get(&80).map(String::as_str), Some("http"));
        assert_eq!(web.os, OperatingSystem::Linux);

        let poison = simulator.arp_poison(
            IpAddr::V4(Ipv4Addr::new(10, 10, 10, 5)),
            IpAddr::V4(Ipv4Addr::new(10, 10, 10, 254)),
        );
        assert_eq!(poison.op, ArpOp::Reply);
        assert_eq!(poison.sender_ip, Ipv4Addr::new(10, 10, 10, 254));
        assert_eq!(poison.target_ip, Ipv4Addr::new(10, 10, 10, 5));
        let bytes = poison.to_bytes();
        assert_eq!(bytes.len(), ARP_PACKET_LEN);
        assert_eq!(ArpPacket::parse(&bytes).unwrap(), poison);
    }
}
