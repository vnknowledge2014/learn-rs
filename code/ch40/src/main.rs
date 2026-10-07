#![allow(dead_code, unused_variables, unused_imports)]
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
