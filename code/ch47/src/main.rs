#![allow(dead_code, unused_variables, unused_imports)]
// ============================================================================
// CHƯƠNG 43: ĐẠI DỰ ÁN CAPSTONE - CÔNG CỤ CLI LOGPULSE CHUẨN SẢN XUẤT
// Phương pháp: Vibe Coding (Kiến Trúc Sư Hệ Thống + Trợ Lý AI)
// ============================================================================

use std::collections::HashMap;

// ----------------------------------------------------------------------------
// 1. MÔ HÌNH DỮ LIỆU & ĐỊNH NGHĨA KIỂU NGHIỆP VỤ (DOMAIN MODELING)
// ----------------------------------------------------------------------------

#[derive(Debug, Clone, PartialEq, Eq)]
pub enum HttpMethod {
    Get,
    Post,
    Put,
    Delete,
    Other(String),
}

impl HttpMethod {
    pub fn from_str_slice(s: &str) -> Self {
        match s.to_ascii_uppercase().as_str() {
            "GET" => HttpMethod::Get,
            "POST" => HttpMethod::Post,
            "PUT" => HttpMethod::Put,
            "DELETE" => HttpMethod::Delete,
            other => HttpMethod::Other(other.to_string()),
        }
    }
}

// Cấu trúc một dòng nhật ký máy chủ web đã được bóc tách
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct LogEntry {
    pub client_ip: String,
    pub method: HttpMethod,
    pub path: String,
    pub status_code: u16,
    pub response_bytes: u64,
}

// Cấu hình tham số dòng lệnh (CLI Options)
#[derive(Debug, Clone)]
pub struct CliConfig {
    pub target_file: String,
    pub verbose: bool,
    pub error_only: bool,
}

impl CliConfig {
    // Phân tích danh sách đối số dòng lệnh an toàn, không làm văng panic
    // Nhận tham chiếu mượn (borrow) lát cắt &[String]
    pub fn parse_from_args(args: &[String]) -> Result<Self, String> {
        if args.len() < 2 {
            return Err("Sử dụng: logpulse <file_path> [--verbose] [--error-only]".to_string());
        }

        let target_file = args[1].clone();
        let mut verbose = false;
        let mut error_only = false;

        for arg in &args[2..] {
            match arg.as_str() {
                "--verbose" | "-v" => verbose = true,
                "--error-only" | "-e" => error_only = true,
                unknown => return Err(format!("Cờ dòng lệnh không xác định: {}", unknown)),
            }
        }

        Ok(CliConfig {
            target_file,
            verbose,
            error_only,
        })
    }
}

// ----------------------------------------------------------------------------
// 2. ĐỘNG CƠ PHÂN TÍCH NHẬT KÝ (LOG ANALYZER ENGINE)
// Áp dụng quyền sở hữu (ownership) và mượn tham chiếu an toàn tuyệt đối
// ----------------------------------------------------------------------------

pub struct LogAnalyzer {
    entries: Vec<LogEntry>,
}

impl LogAnalyzer {
    pub fn new() -> Self {
        Self {
            entries: Vec::new(),
        }
    }

    // Bóc tách một dòng văn bản thô theo định dạng chuẩn: "IP METHOD PATH STATUS BYTES"
    // Ví dụ: "192.168.1.1 GET /api/v1/users 200 1024"
    pub fn parse_line(line: &str) -> Option<LogEntry> {
        let parts: Vec<&str> = line.split_whitespace().collect();
        if parts.len() < 5 {
            return None; // Dòng không hợp lệ hoặc bị lỗi định dạng
        }

        let client_ip = parts[0].to_string();
        let method = HttpMethod::from_str_slice(parts[1]);
        let path = parts[2].to_string();
        let status_code = parts[3].parse::<u16>().ok()?;
        let response_bytes = parts[4].parse::<u64>().ok()?;

        Some(LogEntry {
            client_ip,
            method,
            path,
            status_code,
            response_bytes,
        })
    }

    // Nạp toàn bộ dữ liệu mẫu (hoặc nội dung đọc từ buffer) vào bộ nhớ
    pub fn load_from_raw_text(&mut self, text: &str) {
        for line in text.lines() {
            let trimmed = line.trim();
            if !trimmed.is_empty() && !trimmed.starts_with('#') {
                if let Some(entry) = Self::parse_line(trimmed) {
                    self.entries.push(entry);
                }
            }
        }
    }

    // Đếm tổng số lượng yêu cầu
    pub fn total_requests(&self) -> usize {
        self.entries.len()
    }

    // Đếm số lượng lỗi máy chủ (Mã 5xx)
    pub fn count_server_errors(&self) -> usize {
        self.entries
            .iter()
            .filter(|e| e.status_code >= 500 && e.status_code < 600)
            .count()
    }

    // Đếm số lượng lỗi phía khách hàng (Mã 4xx)
    pub fn count_client_errors(&self) -> usize {
        self.entries
            .iter()
            .filter(|e| e.status_code >= 400 && e.status_code < 500)
            .count()
    }

    // Tính tổng số byte dữ liệu máy chủ đã truyền tải
    pub fn total_data_transferred_bytes(&self) -> u64 {
        self.entries.iter().map(|e| e.response_bytes).sum()
    }

    // Tìm địa chỉ IP gửi nhiều yêu cầu nhất thông qua bảng băm HashMap
    pub fn find_top_client_ip(&self) -> Option<(String, usize)> {
        let mut frequency_map: HashMap<&str, usize> = HashMap::new();

        for entry in &self.entries {
            *frequency_map.entry(entry.client_ip.as_str()).or_insert(0) += 1;
        }

        frequency_map
            .into_iter()
            .max_by_key(|&(_, count)| count)
            .map(|(ip, count)| (ip.to_string(), count))
    }
}

// ----------------------------------------------------------------------------
// 3. TẦNG ĐỊNH DẠNG BẢNG BÁO CÁO (PRESENTATION LAYER)
// ----------------------------------------------------------------------------

pub struct ReportPrinter;

impl ReportPrinter {
    // In báo cáo định dạng bảng ASCII sắc nét, chuyên nghiệp
    pub fn print_summary(analyzer: &LogAnalyzer, config: &CliConfig) {
        println!("+-------------------------------------------------------------+");
        println!("|            LOGPULSE - BÁO CÁO PHÂN TÍCH NHẬT KÝ MÁY CHỦ    |");
        println!("+-------------------------------------------------------------+");
        println!("| Tệp tin mục tiêu       : {:<34} |", config.target_file);
        println!("| Tổng số lượt yêu cầu   : {:<34} |", analyzer.total_requests());
        println!("| Lỗi máy chủ (5xx)      : {:<34} |", analyzer.count_server_errors());
        println!("| Lỗi người dùng (4xx)   : {:<34} |", analyzer.count_client_errors());

        let total_kb = analyzer.total_data_transferred_bytes() as f64 / 1024.0;
        println!("| Tổng dung lượng truyền : {:<31.2} KB |", total_kb);

        if let Some((top_ip, count)) = analyzer.find_top_client_ip() {
            let ip_summary = format!("{} ({} lần)", top_ip, count);
            println!("| Địa chỉ IP truy cập top: {:<34} |", ip_summary);
        }
        println!("+-------------------------------------------------------------+");
    }
}

// ----------------------------------------------------------------------------
// 4. HÀM MAIN: KỊCH BẢN THỰC THI TOÀN DIỆN
// ----------------------------------------------------------------------------

fn main() {
    println!("=== KHỞI ĐỘNG DỰ ÁN CAPSTONE: CÔNG CỤ LOGPULSE CLI (VIBE CODING) ===\n");

    // Giả lập đối số dòng lệnh mà người dùng nhập vào terminal
    let simulated_cli_args = vec![
        "logpulse".to_string(),
        "/var/log/nginx/access.log".to_string(),
        "--verbose".to_string(),
    ];

    // 1. Phân tích cờ dòng lệnh
    let config = match CliConfig::parse_from_args(&simulated_cli_args) {
        Ok(cfg) => cfg,
        Err(err) => {
            eprintln!("[Lỗi tham số] {}", err);
            std::process::exit(1);
        }
    };

    println!("[Khởi tạo] Đang phân tích tệp: {} (Verbose: {})", config.target_file, config.verbose);

    // 2. Dữ liệu nhật ký mẫu mô phỏng dữ liệu đọc từ bộ nhớ đệm (buffer)
    let sample_access_log = r#"
        192.168.1.100 GET /index.html 200 4096
        192.168.1.101 POST /api/v1/auth/login 200 1024
        192.168.1.102 GET /secret/admin 403 512
        192.168.1.100 GET /images/logo.png 200 12048
        10.0.0.50 POST /api/v1/payment/checkout 500 256
        192.168.1.100 POST /api/v1/comments 201 1024
        10.0.0.51 GET /non-existent-page 404 128
        10.0.0.50 POST /api/v1/payment/checkout 503 256
    "#;

    // 3. Nạp dữ liệu vào Động cơ phân tích
    let mut analyzer = LogAnalyzer::new();
    analyzer.load_from_raw_text(sample_access_log);

    // 4. In bảng báo cáo tổng kết ra màn hình
    ReportPrinter::print_summary(&analyzer, &config);

    // 5. Kiểm chứng tính toàn vẹn của kết quả phân tích
    assert_eq!(analyzer.total_requests(), 8);
    assert_eq!(analyzer.count_server_errors(), 2); // Mã 500 và 503
    assert_eq!(analyzer.count_client_errors(), 2); // Mã 403 và 404
    assert_eq!(analyzer.find_top_client_ip(), Some(("192.168.1.100".to_string(), 3)));

    println!("\n[Thành công] Công cụ CLI đã thực thi hoàn hảo, kiểm tra Assertions vượt qua 100%!");
}
