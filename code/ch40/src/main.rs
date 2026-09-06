#![allow(dead_code, unused_variables, unused_imports)]
use std::net::{SocketAddr, TcpStream};
use std::sync::mpsc::{channel, Sender};
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
    let address_str = format!("{}:{}", ip, port);
    if let Ok(socket_addr) = address_str.parse::<SocketAddr>() {
        // Thực hiện bắt tay TCP Connect với thời gian chờ nghiêm ngặt
        if let Ok(_stream) = TcpStream::connect_timeout(&socket_addr, timeout) {
            // Kết nối thành công! _stream sẽ tự động đóng kết nối khi ra khỏi phạm vi
            return true;
        }
    }
    false
}

/// Động cơ quét cổng mạng đa luồng tốc độ high
pub fn execute_concurrent_scan(config: ScanConfig) -> Vec<PortResult> {
    let (tx, rx) = channel::<PortResult>();
    let mut thread_handles = Vec::new();
    let timeout = Duration::from_millis(config.timeout_ms);

    println!(
        "[*] Bắt đầu quét đa luồng mục tiêu {} (Dải cổng: {} -> {})...",
        config.target_ip, config.start_port, config.end_port
    );

    let ports: Vec<u16> = (config.start_port..=config.end_port).collect();
    let chunk_size = (ports.len() + config.thread_count - 1) / config.thread_count;

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
    println!("   CONG CU QUET CONG MANG DA LUONG SIEU TOC (RUST PORT SCANNER)  ");
    println!("==================================================================");

    // Thiết lập cấu hình kiểm thử quét trên máy cục bộ (Localhost 127.0.0.1)
    let config = ScanConfig {
        target_ip: "127.0.0.1".to_string(),
        start_port: 75,
        end_port: 85,
        timeout_ms: 100, // 100ms timeout cực fast cho mạng nội bộ
        thread_count: 4,  // 4 luồng quét song song
    };

    println!("    - Dia chi IP muc tieu : {}", config.target_ip);
    println!("    - Pham vi cong quet   : {} -> {}", config.start_port, config.end_port);
    println!("    - So luong luong chay : {}", config.thread_count);
    println!("    - Thoi gian cho toi da: {} ms/port\n", config.timeout_ms);

    // Giả lập mở một cổng cục bộ để kiểm tra tính chính xác của trình quét
    let mock_listener = std::net::TcpListener::bind("127.0.0.1:80").ok();
    if mock_listener.is_some() {
        println!("    [+] Da kich hoat cong gia lap 80 (HTTP) de kiem thu.");
    }

    let results = execute_concurrent_scan(config);

    println!("\n==================================================================");
    println!("                  DANH SACH CONG DANG MO (OPEN PORTS)             ");
    println!("==================================================================");
    if results.is_empty() {
        println!("    [!] Low phat hien thay cong nao mo trong pham vi quet.");
    } else {
        for res in &results {
            println!(
                "    [+] Cong {:5}/TCP : MO (Open) | Dich vu: {}",
                res.port, res.service_hint
            );
        }
    }

    println!("\n==================================================================");
    println!("   QUET CONG HOAN TAT AN TOAN: ZERO DATA RACE & ZERO MEMORY LEAK! ");
    println!("==================================================================");
}
