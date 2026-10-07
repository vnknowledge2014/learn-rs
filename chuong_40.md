# Chương 40: Tự chế công cụ quét cổng mạng đa luồng siêu tốc (High-Speed Concurrent Network Port Scanner Tool)

## Giới thiệu & Mục tiêu học tập

Trong kho vũ khí của bất kỳ kỹ sư quản trị mạng hay chuyên gia bảo mật thâm nhập OSCP nào, công cụ đầu tiên luôn được rút ra khỏi bao chính là **Trình quét cổng mạng (Network Port Scanner)** — với đại diện lừng danh nhất thế giới là `Nmap`. Trước khi có thể bảo vệ hoặc kiểm thử một máy chủ, bạn bắt buộc phải biết máy chủ đó đang "mở những cánh cửa nào" ra thế giới bên ngoài.

Tuy nhiên, thay vì chỉ sử dụng các công cụ có sẵn một cách thụ động, việc tự tay lập trình một công cụ quét cổng mạng đa luồng (multi-threaded concurrent port scanner) từ con số không bằng Rust sẽ mang lại cho bạn những hiểu biết vô giá về:
- Cách thức hoạt động ở tầng giao vận (Transport Layer) của giao thức TCP và cơ chế bắt tay 3 bước (3-way handshake).
- Kỹ thuật lập trình ổ cắm mạng (Socket Programming) ở mức hệ thống với `std::net::TcpStream`.
- Mô hình điều phối đa luồng đồng thời bằng Rust: Phân chia công việc giữa các luồng (`std::thread`) và gom kết quả về luồng chính thông qua kênh truyền tin đa người gửi - một người nhận (`std::sync::mpsc`).
- Tối ưu hóa thời gian chờ (Timeout management) và kiểm soát tài nguyên hệ điều hành (File Descriptors).

---

## Hình tượng hóa đời sống (Intuitive Everyday Analogy)

Để hiểu rõ sự khác biệt giữa quét tuần tự đơn luồng và quét đồng thời đa luồng, hãy quan sát câu chuyện của người đưa thư:

```
┌──────────────────────────────────────────────────────────────────────────────────┐
│             HÌNH TƯỢNG HÓA: ĐỘI ĐƯA THƯ GÕ CỬA TÒA NHÀ 1000 PHÒNG                │
├──────────────────────────────────────────────────────────────────────────────────┤
│                                                                                  │
│ [CÁCH 1: ANH ĐƯA THƯ ĐƠN ĐỘC (QUÉT TUẦN TỰ ĐƠN LUỒNG - SYNCHRONOUS)]             │
│ - Anh đưa thư đi bộ đến Phòng 1 ──► Gõ cửa ──► Đứng đợi 3 giây (Timeout)         │
│ - Không ai mở cửa (Cổng đóng) ──► Bước sang Phòng 2 ──► Lại đợi 3 giây...        │
│   ===> Để gõ hết 1,000 phòng, anh ta mất: 1,000 x 3 = 3,000 giây (~50 PHÚT)!     │
│                                                                                  │
│ [CÁCH 2: BIỆT ĐỘI 50 NGƯỜI ĐƯA THƯ (QUÉT ĐA LUỒNG - MULTI-THREADED MPSC)]        │
│ ┌──────────────────────────────────────────────────────────────────────┐         │
│ │ Đội trưởng chia 1000 phòng cho 50 anh em (Mỗi người 20 phòng)        │         │
│ │ 50 người đồng loạt tỏa đi gõ cửa cùng một lúc!                       │         │
│ ├──────────────────────────────────────────────────────────────────────┤         │
│ │ Ai thấy phòng có người ra mở cửa (Cổng MỞ - TCP SYN-ACK):            │         │
│ │   Lập tức bấm bộ đàm thông báo về trung tâm (Kênh MPSC Channel)!     │         │
│ ├──────────────────────────────────────────────────────────────────────┤         │
│ │ Đội trưởng ngồi tại phòng bảo vệ chỉ việc ghi nhận danh sách phòng mở│         │
│ └──────────────────────────────────────────────────────────────────────┘         │
│   ===> Mỗi người chỉ 20 phòng x 3 giây = 60 giây: NHANH GẤP 50 LẦN!            │
└──────────────────────────────────────────────────────────────────────────────────┘
```

### 1. Cổng mạng (Port) giống như số phòng trong chung cư
- Địa chỉ IP (ví dụ `192.168.1.10`) giống như địa chỉ của một tòa nhà chung cư lớn.
- Số cổng mạng (từ `1` đến `65535`) giống như số phòng cụ thể bên trong tòa nhà đó:
  - Phòng `80` (HTTP): Quầy lễ tân công cộng mở cửa đón khách du lịch xem thông tin.
  - Phòng `443` (HTTPS): Quầy giao dịch tài chính có nhân viên bảo vệ kiểm tra căn cước (chứng chỉ SSL/TLS).
  - Phòng `22` (SSH): Phòng điều hành máy chủ bí mật ở tầng áp mái có khóa vân tay.
  - Các phòng còn lại: Cửa đóng then cài, không có người ở.

### 2. Quét cổng mạng (Port Scanning) giống như gõ cửa từng phòng
- Khi bạn muốn biết phòng nào đang hoạt động, bạn gõ nhẹ vào cửa phòng (`gửi gói tin TCP SYN`).
- Nếu phòng có người ra mở cửa và niềm nở chào bạn (`trả về TCP SYN-ACK`), bạn biết ngay phòng đó đang **MỞ (Open)**. Bạn lịch sự cảm ơn và rời đi.
- Nếu có người nói vọng ra *"Không có ai, đi đi!"* (`gói RST`), phòng đó **ĐÓNG (Closed)**. Nếu cửa im lìm, sau 200 mili-giây không ai trả lời (`Timeout`), bạn ghi nhận phòng đó là **BỊ LỌC (Filtered)** — có thể tường lửa đã nuốt mất tiếng gõ. Với TCP Connect Scan, cả hai trường hợp đều được tính là "không mở".
- Khi sử dụng Rust đa luồng kết hợp bộ đàm liên lạc (`mpsc`), công việc này diễn ra với tốc độ hàng ngàn phòng mỗi giây mà không bỏ sót bất kỳ dịch vụ nào!

---

## Khái niệm & Cơ chế kỹ thuật chuyên sâu (In-Depth Technical Mechanics)

### 1. Giao thức TCP và Cơ chế Bắt tay 3 bước (TCP 3-Way Handshake)

Giao thức TCP (Transmission Control Protocol) là giao thức truyền thông tin cậy hướng kết nối:

```
Máy quét (Scanner)                                  Máy chủ mục tiêu (Target)
      │                                                         │
      │  1. Gói tin SYN (Xin chào, tôi muốn kết nối)            │
      ├────────────────────────────────────────────────────────►│
      │                                                         │
      │  2. Gói tin SYN-ACK (Đồng ý, tôi mở cửa đón bạn!)       │ ◄── CỔNG MỞ (OPEN)
      │◄────────────────────────────────────────────────────────┤
      │                                                         │
      │  (HOẶC gói tin RST: Cổng này đóng, xin đừng làm phiền) │ ◄── CỔNG ĐÓNG (CLOSED)
      │◄────────────────────────────────────────────────────────┤
      │                                                         │
      │  3. Gói tin ACK (Xác nhận kết nối hoàn tất)             │
      ├────────────────────────────────────────────────────────►│
      │                                                         │
```

Kỹ thuật quét mà chúng ta triển khai mang tên **TCP Connect Scan**:
- Chương trình của chúng ta yêu cầu Hệ điều hành hoàn thành trọn vẹn quy trình bắt tay 3 bước thông qua lời gọi hàm `TcpStream::connect_timeout`.
- **Ưu điểm**: Hoạt động được trên mọi hệ điều hành (Linux, macOS, Windows) mà không đòi hỏi quyền hạn Quản trị viên tối cao (Root/Administrator), không cần cấu hình Raw Socket phức tạp.
- **Tính toán Timeout**: Nếu kết nối tới một cổng đóng bị lọc bởi tường lửa (Firewall), hệ điều hành có thể treo luồng từ vài chục giây tới ~2 phút (Linux mặc định thử lại SYN khoảng 127 giây) nếu không có cấu hình timeout. Bằng cách đặt thời gian chờ khoảng 200–500 ms (`Duration::from_millis(...)`), chúng ta có thể quét hàng ngàn cổng trong chớp mắt.

### 2. Kiến trúc Đa luồng và Kênh truyền tin (`std::sync::mpsc`)

Để đạt tốc độ tối đa mà không gây nghẽn (non-blocking), chúng ta áp dụng mô hình phân tán công việc:
1. **Chia việc (Work Division)**: Dải cổng cần quét (ví dụ từ cổng `1` đến `1024`) được phân bổ cho các luồng độc lập (`std::thread::spawn`).
2. **Kênh truyền tin (`mpsc: Multi-Producer, Single-Consumer`)**:
   - `Sender` (Người gửi): Được nhân bản (`tx.clone()`) và chuyển quyền sở hữu (ownership) vào từng luồng con.
   - Khi một luồng con phát hiện cổng mở, nó gửi số cổng `port: u16` qua kênh truyền tin.
   - `Receiver` (Người nhận): Nằm tại luồng chính, lắng nghe và thu thập các cổng mở được gửi về.
3. **Đóng kênh truyền an toàn (Graceful Channel Shutdown)**:
   - Trong Rust, kênh `mpsc` chỉ thực sự đóng lại khi **tất cả mọi bản sao của `Sender` đều bị tiêu hủy (`drop`)**.
   - Do đó, luồng chính sau khi nhân bản `tx` cho các luồng con bắt buộc phải gọi `drop(tx)` (tiêu hủy bản sao gốc của chính mình). Khi tất cả các luồng con hoàn thành và tự động `drop(tx_clone)`, bộ lặp `rx.iter()` trên luồng chính sẽ kết thúc vòng lặp một cách êm ái!

### 3. Tương thích Bộ nhớ và Quản lý Tài nguyên Hệ thống

- Mỗi tiến trình trên hệ điều hành đều có một giới hạn về số lượng kết nối mạng mở đồng thời (gọi là giới hạn **File Descriptors**). Giá trị mặc định thay đổi theo hệ điều hành: thường là 1024 trên Linux và chỉ 256 trên macOS. Bạn có thể xem bằng lệnh `ulimit -n`. Vì vậy đừng bao giờ sinh ra hàng nghìn luồng cùng mở socket một lúc — mã nguồn bên dưới chỉ tạo `thread_count` luồng, mỗi luồng quét lần lượt một lô cổng và chỉ mở **một** socket tại một thời điểm, nên số socket đồng thời không bao giờ vượt `thread_count`.
- `TcpStream` ngay sau khi kết nối thành công sẽ lập tức bị huỷ (ra khỏi phạm vi) — kết nối được đóng và mô tả tệp được trả lại cho hệ điều hành thông qua cơ chế RAII, nên trình quét không bao giờ giữ socket lâu hơn cần thiết.

---

## Mã nguồn minh họa thực chiến (Idiomatic Runnable Rust Blueprint)

Dưới đây là mã nguồn hoàn chỉnh của công cụ **Trình quét cổng mạng (Port Scanner)** đa luồng hiệu năng cao bằng Rust chuẩn mực, không cần thư viện ngoài, có khả năng quét dải cổng mạng song song với thời gian chờ thông minh:

```rust
use std::net::{IpAddr, SocketAddr, TcpStream};
use std::sync::mpsc::{Sender, channel};
use std::thread;
use std::time::Duration;

/// Cấu hình tham số quét mạng
#[derive(Debug, Clone)]
pub struct ScanConfig {
    pub target_ip: String,
    pub start_port: u16,
    pub end_port: u16,
    pub timeout_ms: u64,
    pub thread_count: usize,
}

/// Kết quả của một cổng được quét
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct PortResult {
    pub port: u16,
    pub is_open: bool,
    pub service_hint: &'static str,
}

/// Hàm phỏng đoán tên dịch vụ phổ biến dựa trên số hiệu cổng
fn guess_service_name(port: u16) -> &'static str {
    match port {
        21 => "FTP (File Transfer Protocol)",
        22 => "SSH (Secure Shell)",
        23 => "Telnet (Unencrypted Text)",
        25 => "SMTP (Simple Mail Transfer)",
        53 => "DNS (Domain Name System)",
        80 => "HTTP (Hypertext Transfer)",
        110 => "POP3 (Post Office Protocol)",
        143 => "IMAP (Internet Message Access)",
        443 => "HTTPS (HTTP Secure)",
        3306 => "MySQL Database Server",
        5432 => "PostgreSQL Database Server",
        6379 => "Redis In-Memory Key-Value Store",
        8080 => "HTTP Alternate / Web Proxy",
        _ => "Dịch vụ tùy chỉnh (Custom / Unknown)",
    }
}

/// Thực hiện kiểm tra trạng thái một cổng đơn lẻ với thời gian chờ xác định
pub fn check_single_port(ip: &str, port: u16, timeout: Duration) -> bool {
    // Ghép IpAddr + cổng bằng SocketAddr::new thay vì format!("{ip}:{port}"):
    // cách ghép chuỗi hỏng với IPv6 (phải viết [::1]:80).
    let Ok(ip_addr) = ip.parse::<IpAddr>() else {
        return false;
    };
    let socket_addr = SocketAddr::new(ip_addr, port);
    // Thực hiện bắt tay TCP Connect với thời gian chờ nghiêm ngặt.
    // Kết nối thành công thì TcpStream tạm thời bị drop ngay, socket tự đóng (RAII).
    TcpStream::connect_timeout(&socket_addr, timeout).is_ok()
}

/// Động cơ quét cổng mạng đa luồng tốc độ cao
pub fn execute_concurrent_scan(config: ScanConfig) -> Vec<PortResult> {
    let (tx, rx) = channel::<PortResult>();
    let mut thread_handles = Vec::new();
    let timeout = Duration::from_millis(config.timeout_ms);

    println!(
        "[*] Bắt đầu quét đa luồng mục tiêu {} (Dải cổng: {} -> {})...",
        config.target_ip, config.start_port, config.end_port
    );

    let ports: Vec<u16> = (config.start_port..=config.end_port).collect();
    if ports.is_empty() {
        return Vec::new(); // start_port > end_port: không có gì để quét
    }
    // Chia đều cho các luồng (làm tròn lên). max(1) tránh chia cho 0 khi thread_count = 0;
    // mỗi luồng chỉ mở MỘT socket tại một thời điểm -> tối đa thread_count socket cùng lúc.
    let chunk_size = ports.len().div_ceil(config.thread_count.max(1));

    for chunk in ports.chunks(chunk_size) {
        let chunk_vec = chunk.to_vec();
        let thread_tx: Sender<PortResult> = tx.clone();
        let target_ip_clone = config.target_ip.clone();

        let handle = thread::spawn(move || {
            for port in chunk_vec {
                if check_single_port(&target_ip_clone, port, timeout) {
                    let result = PortResult {
                        port,
                        is_open: true,
                        service_hint: guess_service_name(port),
                    };
                    let _ = thread_tx.send(result);
                }
            }
        });

        thread_handles.push(handle);
    }

    // Tiêu hủy bản sao Sender gốc để luồng nhận (Receiver) biết khi nào kết thúc
    drop(tx);

    // Chờ tất cả các luồng hoàn thành nhiệm vụ
    for handle in thread_handles {
        let _ = handle.join();
    }

    // Thu thập toàn bộ kết quả từ kênh truyền tin MPSC
    let mut open_ports: Vec<PortResult> = rx.into_iter().collect();

    // Sắp xếp lại danh sách cổng theo thứ tự tăng dần
    open_ports.sort_by_key(|res| res.port);
    open_ports
}

fn main() {
    println!("==================================================================");
    println!("   CÔNG CỤ QUÉT CỔNG MẠNG ĐA LUỒNG SIÊU TỐC (RUST PORT SCANNER)  ");
    println!("==================================================================");

    // Giả lập một dịch vụ đang chạy để kiểm tra tính chính xác của trình quét:
    // bind cổng 0 -> hệ điều hành tự chọn một cổng trống (không cần quyền root
    // như khi bind cổng 80).
    let mock_listener =
        std::net::TcpListener::bind("127.0.0.1:0").expect("không mở được cổng giả lập");
    let mock_port = mock_listener.local_addr().unwrap().port();

    // Thiết lập cấu hình kiểm thử quét trên máy cục bộ (Localhost 127.0.0.1)
    let config = ScanConfig {
        target_ip: "127.0.0.1".to_string(),
        start_port: mock_port.saturating_sub(5),
        end_port: mock_port.saturating_add(5),
        timeout_ms: 100, // 100ms timeout cực nhanh cho mạng nội bộ
        thread_count: 4, // 4 luồng quét song song
    };

    println!("    - Địa chỉ IP mục tiêu : {}", config.target_ip);
    println!(
        "    - Phạm vi cổng quét   : {} -> {}",
        config.start_port, config.end_port
    );
    println!("    - Số luồng chạy       : {}", config.thread_count);
    println!("    - Thời gian chờ tối đa: {} ms/cổng", config.timeout_ms);
    println!(
        "    [+] Đã kích hoạt cổng giả lập {} để kiểm thử.\n",
        mock_port
    );

    let results = execute_concurrent_scan(config);

    println!("\n==================================================================");
    println!("                  DANH SÁCH CỔNG ĐANG MỞ (OPEN PORTS)             ");
    println!("==================================================================");
    if results.is_empty() {
        println!("    [!] Không phát hiện thấy cổng nào mở trong phạm vi quét.");
    } else {
        for res in &results {
            println!(
                "    [+] Cổng {:5}/TCP : MỞ (Open) | Dịch vụ: {}",
                res.port, res.service_hint
            );
        }
    }
    assert!(results.iter().any(|r| r.port == mock_port));
    drop(mock_listener);

    println!("\n==================================================================");
    println!("   QUÉT CỔNG HOÀN TẤT AN TOÀN: KHÔNG DATA RACE, KHÔNG RÒ RỈ BỘ NHỚ! ");
    println!("==================================================================");
}
```

---

## Bảng tra cứu lỗi biên dịch & Cách khắc phục (Compiler Error Guide)

Dưới đây là các lỗi biên dịch thường gặp nhất khi xây dựng công cụ quét mạng đa luồng trong Rust:

| Mã lỗi | Thông báo mẫu từ trình biên dịch | Nguyên nhân cốt lõi | Cách khắc phục nhanh |
|---|---|---|---|
| **E0277** | `'Rc<i32>' cannot be sent between threads safely` | Bạn cố gắng truyền một con trỏ thông minh (smart pointer) đơn luồng (`Rc<T>`) qua ranh giới luồng trong `thread::spawn`. | Thay thế `Rc<T>` bằng con trỏ thông minh đa luồng an toàn: `Arc<T>` (Atomic Reference Counting). |
| **E0382** | `use of moved value: 'tx'` | Bạn truyền `tx` vào luồng thứ nhất khiến quyền sở hữu (ownership) bị di chuyển, sau đó lại cố gắng dùng lại `tx` ở luồng thứ hai. | Nhân bản `Sender` trước khi đưa vào luồng: `let tx_clone = tx.clone();`. |
| **E0373** | `closure may outlive the current function, but it borrows 'target_ip', which is owned by the current function` | Closure truyền cho `thread::spawn` phải là `'static` (luồng con có thể sống lâu hơn hàm cha), do đó nó không thể mượn biến cục bộ của hàm cha. (Nếu thứ bị mượn là một tham số `&str`, lỗi sẽ là **E0521** `borrowed data escapes outside of function`.) | Sử dụng từ khóa `move` và clone chuỗi thành kiểu có quyền sở hữu độc lập: `let ip = target_ip.clone();`. |
| **E0508** | `cannot move out of type '[String]', a non-copy slice` | Cố gắng lấy phần tử ra khỏi một lát cắt mượn `&[T]` (`let ip = ips[0];`) mà kiểu dữ liệu không triển khai trait `Copy`. | Sử dụng phương thức `.clone()` hoặc chuyển thành `Vec` riêng biệt. |

### Ví dụ phân tích lỗi `E0382` khi truyền Sender vào luồng con:

```rust
use std::sync::mpsc::channel;
use std::thread;

// Đoạn mã lỗi minh họa E0382:
fn e0382_broken() {
    let (tx, _rx) = channel::<u16>();

    // Luồng 1 lấy quyền sở hữu tx
    // thread::spawn(move || { let _ = tx.send(80); });

    // LỖI E0382: tx đã bị di chuyển vào luồng 1, không thể dùng ở luồng 2!
    // thread::spawn(move || { let _ = tx.send(443); });
}

// Cách sửa chữa đúng chuẩn: Nhân bản bản sao Sender cho mỗi luồng
fn e0382_correct() {
    let (tx, _rx) = channel::<u16>();

    let tx1 = tx.clone();
    thread::spawn(move || { let _ = tx1.send(80); });

    let tx2 = tx.clone();
    thread::spawn(move || { let _ = tx2.send(443); });
}
```

---

## Kiểm thử tự động (Automated Tests)

Kiểm thử một trình quét mạng nghe có vẻ khó vì cần một máy chủ thật — nhưng `TcpListener::bind("127.0.0.1:0")` cho ta một cổng đang lắng nghe trong chớp mắt, do hệ điều hành tự chọn, không cần quyền root. Các test dưới đây dùng đúng kỹ thuật đó, và khoá lại hai trường hợp biên từng làm phiên bản cũ panic: dải cổng rỗng và `thread_count = 0`.

```rust
#[cfg(test)]
mod tests {
    use super::*;
    use std::net::TcpListener;

    #[test]
    fn finds_a_listening_port_and_nothing_closed() {
        let listener = TcpListener::bind("127.0.0.1:0").unwrap();
        let port = listener.local_addr().unwrap().port();
        let results = execute_concurrent_scan(ScanConfig {
            target_ip: "127.0.0.1".into(),
            start_port: port,
            end_port: port,
            timeout_ms: 200,
            thread_count: 2,
        });
        assert_eq!(results.len(), 1);
        assert_eq!(results[0].port, port);
        assert!(results[0].is_open);
    }

    #[test]
    fn empty_range_and_zero_threads_do_not_panic() {
        // Lỗi cũ: thread_count = 0 -> chia cho 0; dải rỗng -> chunks(0) panic
        let empty = execute_concurrent_scan(ScanConfig {
            target_ip: "127.0.0.1".into(),
            start_port: 10,
            end_port: 5,
            timeout_ms: 50,
            thread_count: 4,
        });
        assert!(empty.is_empty());
        let listener = TcpListener::bind("127.0.0.1:0").unwrap();
        let port = listener.local_addr().unwrap().port();
        let zero_threads = execute_concurrent_scan(ScanConfig {
            target_ip: "127.0.0.1".into(),
            start_port: port,
            end_port: port,
            timeout_ms: 200,
            thread_count: 0,
        });
        assert_eq!(zero_threads.len(), 1);
    }

    #[test]
    fn invalid_ip_is_reported_closed() {
        assert!(!check_single_port(
            "không-phải-ip",
            80,
            Duration::from_millis(10)
        ));
        assert_eq!(guess_service_name(22), "SSH (Secure Shell)");
    }
}
```

## Tóm tắt chương & Bài tập rèn luyện (Summary & Exercises)

### 4 Điểm cốt lõi cần ghi nhớ:
1. **Nguyên lý TCP Connect Scan**: Sử dụng cơ chế bắt tay 3 bước chuẩn mực của hệ điều hành để xác định cổng mở mà không cần quyền hạn quản trị viên đặc biệt.
2. **Sức mạnh Concurrency của Rust**: Phân chia khối lượng công việc cho các luồng độc lập, truyền dữ liệu qua kênh `mpsc` mà không lo ngại tranh chấp dữ liệu (Data Race).
3. **Quản lý Vòng đời Kênh MPSC**: Luồng chính phải giải phóng bản sao `tx` gốc để vòng lặp đọc `rx` biết thời điểm kết thúc khi tất cả các luồng con hoàn tất.
4. **An toàn Tài nguyên**: Khái niệm quyền sở hữu (ownership), mượn (borrow), thời gian sống (lifetime), con trỏ thông minh (smart pointer) và bộ nhớ đệm (buffer) kết hợp để đảm bảo các kết nối mạng `TcpStream` được dọn dẹp tức thì, không làm cạn kiệt tài nguyên hệ thống.

### Bài tập rèn luyện tự giải:
1. **Bài tập 1 (Bổ sung tính năng Banner Grabbing)**:  
   Khi phát hiện một cổng mở (ví dụ cổng `80` hoặc cổng `21`), hãy cho chương trình gửi một chuỗi ngắn `b"HEAD / HTTP/1.0\r\n\r\n"` và đọc tối đa 128 bytes phản hồi đầu tiên từ máy chủ. In chuỗi thông tin này ra màn hình để biết chính xác phiên bản phần mềm máy chủ đang chạy.
2. **Bài tập 2 (Tối ưu hóa số luồng động Worker Pool)**:  
   Thay vì tạo số luồng bằng với số cổng (có thể gây quá tải CPU nếu quét 10,000 cổng), hãy thiết lập một hàng đợi công việc cố định gồm đúng 20 luồng công nhân (Worker Threads), liên tục rút việc từ một kênh chung cho đến khi hết cổng cần quét.
3. **Bài tập 3 (Suy ngẫm OSCP: Sự khác biệt giữa SYN Stealth Scan và Connect Scan)**:  
   Tại sao trong các bài thi kiểm thử thâm nhập OSCP thực tế, các chuyên gia lại thích sử dụng kiểu quét `SYN Stealth Scan` (chỉ gửi SYN, nhận SYN-ACK rồi gửi RST hủy ngay thay vì gửi ACK hoàn tất)? Ưu điểm về mặt tàng hình (evasion) của kỹ thuật này đối với các hệ thống ghi nhật ký (Firewall / IDS Log) là gì?

---

### Gợi ý & Lời giải

<details>
<summary><b>Bài tập 1 — Gợi ý</b></summary>

Sau khi `TcpStream::connect` thành công, bạn có một luồng hai chiều: `write_all` gửi yêu cầu, rồi `read` đọc phản hồi. Đặt thời gian chờ đọc để một cổng mở nhưng câm lặng không treo mãi.
</details>

<details>
<summary><b>Bài tập 1 — Lời giải</b></summary>

```rust
use std::io::{Read, Write};
use std::net::{IpAddr, SocketAddr, TcpStream};
use std::time::Duration;

/// Kết nối tới `ip:port`, gửi một yêu cầu HEAD và đọc tối đa 128 byte đầu.
/// Trả về `None` nếu không kết nối được hoặc máy chủ không nói gì.
pub fn grab_banner(ip: &str, port: u16, timeout: Duration) -> Option<String> {
    let addr = SocketAddr::new(ip.parse::<IpAddr>().ok()?, port);
    // Kết nối cũng phải có thời gian chờ: cổng bị lọc có thể treo hàng chục giây.
    let mut stream = TcpStream::connect_timeout(&addr, timeout).ok()?;

    // BẮT BUỘC đặt thời gian chờ ĐỌC. Một cổng có thể mở nhưng dịch vụ
    // im lặng chờ ta nói trước; không có timeout thì `read` treo vô hạn.
    stream.set_read_timeout(Some(timeout)).ok()?;

    // Gửi yêu cầu tối thiểu để khều máy chủ trả lời.
    stream.write_all(b"HEAD / HTTP/1.0\r\n\r\n").ok()?;

    // Đọc tối đa 128 byte đầu — đủ để lộ dòng "Server:" mà không đọc cả trang.
    let mut buf = [0u8; 128];
    let n = stream.read(&mut buf).ok()?;
    if n == 0 {
        return None; // Kết nối được nhưng máy chủ đóng ngay, không có banner.
    }

    // Phản hồi có thể chứa byte không phải UTF-8 -> dùng bản mất mát cho an toàn.
    Some(String::from_utf8_lossy(&buf[..n]).into_owned())
}

#[test]
fn banner_is_none_when_connection_fails() {
    // Cổng 1 trên địa chỉ loopback gần như chắc chắn đóng -> None, không treo.
    let r = grab_banner("127.0.0.1", 1, Duration::from_millis(200));
    assert!(r.is_none());
}
```

**Vì sao phải `set_read_timeout` chứ không chỉ `connect_timeout`:** hai thời gian chờ này canh hai giai đoạn khác nhau. `connect_timeout` giới hạn *bắt tay TCP* — "cổng này có mở không". `set_read_timeout` giới hạn *chờ dữ liệu* — "máy chủ có nói gì không". Một cổng mở của dịch vụ chờ-khách-nói-trước (nhiều giao thức nhị phân là vậy) sẽ qua được `connect` rồi treo mãi ở `read`. Bỏ sót timeout thứ hai là lý do phổ biến khiến trình quét tự viết bị đơ.

**Đây chính là bước biến "cổng mở" thành thông tin tình báo.** Biết cổng 22 mở chỉ cho biết *có* SSH; đọc banner `SSH-2.0-OpenSSH_7.4` cho biết *phiên bản nào* — và phiên bản là thứ tra thẳng ra danh sách lỗ hổng đã biết (CVE). Đó là toàn bộ giá trị của giai đoạn banner grabbing trong trinh sát.
</details>

<details>
<summary><b>Bài tập 2 — Gợi ý</b></summary>

Điểm mấu chốt: 20 luồng chia nhau **một** hàng đợi, không phải một luồng mỗi cổng. Bọc phía nhận của kênh trong `Arc<Mutex<...>>` để mọi luồng cùng rút việc từ đó cho tới khi cạn.
</details>

<details>
<summary><b>Bài tập 2 — Lời giải</b></summary>

```rust
// Đặt trong một module riêng của cùng crate; dùng lại `check_single_port`
// của chương trình chính (`use super::check_single_port;`).
use std::sync::mpsc::{channel, Receiver};
use std::sync::{Arc, Mutex};
use std::thread;
use std::time::Duration;

/// Quét dải cổng bằng ĐÚNG 20 luồng công nhân dùng chung một hàng đợi việc,
/// bất kể phải quét 100 hay 10.000 cổng. Trả về các cổng mở, đã sắp xếp.
pub fn scan_with_worker_pool(ip: &str, ports: Vec<u16>, timeout: Duration) -> Vec<u16> {
    const WORKERS: usize = 20;

    // Hàng đợi việc: nạp mọi cổng vào kênh rồi ĐÓNG phía gửi.
    // Khi kênh cạn và đã đóng, `recv()` trả Err -> tín hiệu cho công nhân dừng.
    let (job_tx, job_rx) = channel::<u16>();
    for p in ports {
        job_tx.send(p).unwrap();
    }
    drop(job_tx); // Đóng: không còn việc mới. Thiếu dòng này -> công nhân chờ mãi.

    // Nhiều luồng cùng rút từ MỘT Receiver -> phải bọc trong Arc<Mutex<...>>.
    let job_rx = Arc::new(Mutex::new(job_rx));
    let (result_tx, result_rx) = channel::<u16>();
    let ip = ip.to_string();

    let mut handles = Vec::new();
    for _ in 0..WORKERS {
        let job_rx: Arc<Mutex<Receiver<u16>>> = Arc::clone(&job_rx);
        let result_tx = result_tx.clone();
        let ip = ip.clone();
        handles.push(thread::spawn(move || loop {
            // Giữ khoá CHỈ đủ lâu để lấy một việc, rồi thả ngay
            // để công nhân khác rút việc song song. Đây là mấu chốt hiệu năng.
            let port = {
                let guard = job_rx.lock().unwrap();
                guard.recv()
            };
            match port {
                Ok(p) => {
                    if check_single_port(&ip, p, timeout) {
                        result_tx.send(p).unwrap();
                    }
                }
                Err(_) => break, // Hàng đợi cạn và đã đóng -> xong việc.
            }
        }));
    }
    drop(result_tx); // Thả bản sao của luồng chính, nếu không `result_rx` treo.

    for h in handles {
        h.join().unwrap();
    }
    let mut open: Vec<u16> = result_rx.iter().collect();
    open.sort_unstable();
    open
}
```

**Vì sao 20 luồng cố định thắng "một luồng mỗi cổng":** mỗi luồng do `std::thread::spawn` tạo ra mặc định giữ 2 MiB ngăn xếp (bộ nhớ ảo) và tốn công cho hệ điều hành lập lịch. Quét 10.000 cổng theo kiểu một-luồng-một-cổng đòi khoảng 20 GB bộ nhớ ảo chỉ riêng cho ngăn xếp, mở 10.000 socket cùng lúc (vượt xa `ulimit -n`) và làm bộ lập lịch nghẹt thở. Việc quét lại **bị chặn bởi I/O** (chờ mạng), không phải bởi CPU — nên 20 luồng, mỗi luồng lần lượt xử lý nhiều cổng, đã đủ giữ đường truyền luôn bận mà chi phí không đổi dù dải cổng lớn đến đâu.

**Hai chi tiết quyết định đúng/sai:**
1. **`drop(job_tx)` trước khi công nhân chạy.** `recv()` chỉ trả `Err` khi kênh vừa cạn *vừa* đã đóng mọi phía gửi. Quên `drop` thì công nhân cuối rút hết việc rồi vẫn ngồi chờ việc không bao giờ tới — treo vĩnh viễn.
2. **Thả khoá ngay sau `recv()`.** Nếu giữ khoá suốt cả lần `check_single_port` (kéo dài bằng cả timeout), thì tại mỗi thời điểm chỉ một công nhân làm việc — 20 luồng hoá ra chạy tuần tự. Phải rút việc dưới khoá, nhưng làm việc *ngoài* khoá.
</details>

<details>
<summary><b>Bài tập 3 — Gợi ý</b></summary>

Câu hỏi không đòi code — nó đòi bạn hiểu điều gì để lại dấu vết trong nhật ký. Hãy so sánh: một lần bắt tay TCP hoàn tất khác gì với một lần bắt tay bị bỏ dở giữa chừng?
</details>

<details>
<summary><b>Bài tập 3 — Lời giải</b></summary>

**Bắt tay TCP đầy đủ (ba bước):**

```text
Máy quét  --SYN-->      Mục tiêu
Máy quét  <--SYN/ACK--  Mục tiêu
Máy quét  --ACK-->      Mục tiêu     [kết nối HOÀN TẤT]
```

Đây là điều `TcpStream::connect` (và `check_single_port` trong chương này) làm — một kết nối trọn vẹn. Vấn đề: **một kết nối hoàn tất là một sự kiện mà ứng dụng nhìn thấy được.** Máy chủ web `accept()` được kết nối, ghi một dòng "đã nhận kết nối từ IP X" vào nhật ký, rồi mới thấy ta ngắt ngay mà chẳng gửi yêu cầu gì. Quét 1.000 cổng kiểu này để lại 1.000 dòng nhật ký đáng ngờ.

**Quét SYN Stealth (bắt tay nửa vời):**

```text
Máy quét  --SYN-->      Mục tiêu
Máy quét  <--SYN/ACK--  Mục tiêu     [cổng MỞ — đã biết đủ]
Máy quét  --RST-->      Mục tiêu     [huỷ bỏ, KHÔNG hoàn tất]
```

Máy quét cố tình không gửi `ACK` cuối. Nhận được `SYN/ACK` là đã đủ trả lời câu hỏi "cổng có mở không"; gửi `RST` để xé bỏ kết nối dở dang.

**Ưu điểm tàng hình, và vì sao nó hiệu quả:**

| | Connect Scan | SYN Stealth |
|---|---|---|
| Bắt tay | hoàn tất cả ba bước | dừng ở bước hai |
| Tầng OS thấy kết nối? | có | có |
| **Tầng ứng dụng thấy?** | **có — ghi nhật ký** | **không — chưa từng `accept`** |
| Chi phí mỗi cổng | dựng rồi phá cả socket | nhẹ hơn, không có socket đầy đủ |

Mấu chốt nằm ở **ranh giới giữa nhân hệ điều hành và ứng dụng**. Một kết nối chỉ "hiện ra" cho ứng dụng (máy chủ web, SSH…) sau khi bắt tay xong *và* nhân trao nó qua `accept()`. Dừng ở bước hai nghĩa là kết nối chưa bao giờ hoàn tất, nên `accept()` không bao giờ trả về nó, nên **phần mềm ghi nhật ký của ứng dụng không có gì để ghi**. Dấu vết chỉ còn ở tầng gói tin — nơi cần công cụ chuyên dụng (IDS) mới thấy, chứ không nằm trong nhật ký ứng dụng thường ngày.

**Hai điều cần nói thẳng cho đúng thực tế:**
- SYN scan **không vô hình**. Một IDS như Snort/Suricata theo dõi ở tầng gói tin vẫn phát hiện được cơn mưa SYN-rồi-RST — đó là dấu hiệu kinh điển của quét cổng. "Stealth" ở đây nghĩa là *né được nhật ký tầng ứng dụng*, không phải né được mọi con mắt.
- Gửi gói SYN thô đòi tạo gói tin thủ công, nên cần **quyền root** (raw socket). Đây là lý do `nmap -sS` phải chạy với `sudo`, còn quét Connect thường (`-sT`) thì không. Bản thân `TcpStream` của Rust không làm SYN scan được — nó luôn hoàn tất bắt tay; muốn stealth phải xuống tầng tạo gói thô bằng crate như `pnet`.
</details>
