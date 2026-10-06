// ============================================================================
// CHƯƠNG 47: ĐẠI DỰ ÁN CAPSTONE - CÔNG CỤ CLI LOGPULSE CHUẨN SẢN XUẤT
// Phương pháp: Vibe Coding (Kiến Trúc Sư Hệ Thống + Trợ Lý AI)
// ============================================================================

use std::collections::HashMap;
use std::fs::File;
use std::io::{self, BufRead, BufReader};
use std::process::ExitCode;

// ----------------------------------------------------------------------------
// 1. MÔ HÌNH DỮ LIỆU & ĐỊNH NGHĨA KIỂU NGHIỆP VỤ (DOMAIN MODELING)
// ----------------------------------------------------------------------------

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum HttpMethod {
    Get,
    Post,
    Put,
    Delete,
    Other,
}

impl HttpMethod {
    // So sánh không phân biệt hoa thường mà không cấp phát chuỗi mới
    pub fn from_str_slice(s: &str) -> Self {
        if s.eq_ignore_ascii_case("GET") {
            HttpMethod::Get
        } else if s.eq_ignore_ascii_case("POST") {
            HttpMethod::Post
        } else if s.eq_ignore_ascii_case("PUT") {
            HttpMethod::Put
        } else if s.eq_ignore_ascii_case("DELETE") {
            HttpMethod::Delete
        } else {
            HttpMethod::Other
        }
    }
}

// Một dòng nhật ký đã bóc tách. Các trường chuỗi là lát cắt `&'a str` MƯỢN thẳng
// từ dòng gốc (Zero-Copy Parsing): không cấp phát String nào khi phân tích.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct LogEntry<'a> {
    pub client_ip: &'a str,
    pub method: HttpMethod,
    pub path: &'a str,
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
// Chỉ giữ các bộ đếm tổng hợp, KHÔNG giữ lại từng dòng: bộ nhớ chỉ tăng theo số
// địa chỉ IP khác nhau, không theo kích thước tệp.
// ----------------------------------------------------------------------------

#[derive(Debug, Default)]
pub struct LogAnalyzer {
    error_only: bool,
    total_requests: usize,
    server_errors: usize,
    client_errors: usize,
    total_bytes: u64,
    skipped_lines: usize,
    ip_counts: HashMap<String, usize>,
}

impl LogAnalyzer {
    pub fn new(error_only: bool) -> Self {
        Self {
            error_only,
            ..Self::default()
        }
    }

    // Bóc tách một dòng văn bản thô theo định dạng chuẩn: "IP METHOD PATH STATUS BYTES"
    // Ví dụ: "192.168.1.1 GET /api/v1/users 200 1024"
    pub fn parse_line(line: &str) -> Option<LogEntry<'_>> {
        let mut parts = line.split_whitespace();
        let client_ip = parts.next()?;
        let method = HttpMethod::from_str_slice(parts.next()?);
        let path = parts.next()?;
        let status_code = parts.next()?.parse::<u16>().ok()?;
        let response_bytes = parts.next()?.parse::<u64>().ok()?;

        Some(LogEntry {
            client_ip,
            method,
            path,
            status_code,
            response_bytes,
        })
    }

    // Ghi nhận một dòng: bỏ qua dòng trống/chú thích, đếm dòng dị dạng
    pub fn ingest_line(&mut self, line: &str) {
        let trimmed = line.trim();
        if trimmed.is_empty() || trimmed.starts_with('#') {
            return;
        }
        let Some(entry) = Self::parse_line(trimmed) else {
            self.skipped_lines += 1;
            return;
        };
        if self.error_only && entry.status_code < 400 {
            return;
        }

        self.total_requests += 1;
        self.total_bytes += entry.response_bytes;
        match entry.status_code {
            400..=499 => self.client_errors += 1,
            500..=599 => self.server_errors += 1,
            _ => {}
        }
        // Chỉ cấp phát String khi gặp một IP MỚI
        if let Some(count) = self.ip_counts.get_mut(entry.client_ip) {
            *count += 1;
        } else {
            self.ip_counts.insert(entry.client_ip.to_string(), 1);
        }
    }

    // Đọc luồng theo từng dòng qua bộ đệm (BufRead): tái sử dụng MỘT bộ đệm `line`,
    // nên tệp nhiều Gigabyte vẫn chỉ tốn bộ nhớ cỡ một dòng.
    pub fn load_from_reader<R: BufRead>(&mut self, mut reader: R) -> io::Result<()> {
        let mut line = String::new();
        loop {
            line.clear();
            if reader.read_line(&mut line)? == 0 {
                return Ok(()); // hết dữ liệu (EOF)
            }
            self.ingest_line(&line);
        }
    }

    pub fn total_requests(&self) -> usize {
        self.total_requests
    }

    // Số lượng lỗi máy chủ (Mã 5xx)
    pub fn count_server_errors(&self) -> usize {
        self.server_errors
    }

    // Số lượng lỗi phía khách hàng (Mã 4xx)
    pub fn count_client_errors(&self) -> usize {
        self.client_errors
    }

    // Tổng số byte dữ liệu máy chủ đã truyền tải
    pub fn total_data_transferred_bytes(&self) -> u64 {
        self.total_bytes
    }

    // Số dòng dị dạng đã bỏ qua
    pub fn skipped_lines(&self) -> usize {
        self.skipped_lines
    }

    // Địa chỉ IP gửi nhiều yêu cầu nhất. Hòa số lượng thì chọn IP nhỏ hơn theo thứ
    // tự từ điển, để kết quả không phụ thuộc thứ tự duyệt ngẫu nhiên của HashMap.
    pub fn find_top_client_ip(&self) -> Option<(&str, usize)> {
        self.ip_counts
            .iter()
            .max_by(|a, b| a.1.cmp(b.1).then_with(|| b.0.cmp(a.0)))
            .map(|(ip, &count)| (ip.as_str(), count))
    }
}

// ----------------------------------------------------------------------------
// 3. TẦNG ĐỊNH DẠNG BẢNG BÁO CÁO (PRESENTATION LAYER)
// ----------------------------------------------------------------------------

pub struct ReportPrinter;

impl ReportPrinter {
    // In báo cáo định dạng bảng ASCII
    pub fn print_summary(analyzer: &LogAnalyzer, config: &CliConfig) {
        println!("+-------------------------------------------------------------+");
        println!("|            LOGPULSE - BÁO CÁO PHÂN TÍCH NHẬT KÝ MÁY CHỦ    |");
        println!("+-------------------------------------------------------------+");
        println!("| Tệp tin mục tiêu       : {:<34} |", config.target_file);
        println!(
            "| Tổng số lượt yêu cầu   : {:<34} |",
            analyzer.total_requests()
        );
        println!(
            "| Lỗi máy chủ (5xx)      : {:<34} |",
            analyzer.count_server_errors()
        );
        println!(
            "| Lỗi người dùng (4xx)   : {:<34} |",
            analyzer.count_client_errors()
        );

        let total_kb = analyzer.total_data_transferred_bytes() as f64 / 1024.0;
        println!("| Tổng dung lượng truyền : {:<31.2} KB |", total_kb);

        if let Some((top_ip, count)) = analyzer.find_top_client_ip() {
            let ip_summary = format!("{} ({} lần)", top_ip, count);
            println!("| Địa chỉ IP truy cập top: {:<34} |", ip_summary);
        }
        if config.verbose {
            println!(
                "| Dòng dị dạng bỏ qua    : {:<34} |",
                analyzer.skipped_lines()
            );
        }
        println!("+-------------------------------------------------------------+");
    }
}

// ----------------------------------------------------------------------------
// 4. HÀM MAIN: KỊCH BẢN THỰC THI TOÀN DIỆN
// Mã thoát: 0 = thành công, 1 = sai tham số, 2 = lỗi đọc tệp.
// ----------------------------------------------------------------------------

// Dữ liệu mẫu dùng khi chạy không kèm đối số (chế độ demo)
const SAMPLE_ACCESS_LOG: &str = r#"
    192.168.1.100 GET /index.html 200 4096
    192.168.1.101 POST /api/v1/auth/login 200 1024
    192.168.1.102 GET /secret/admin 403 512
    192.168.1.100 GET /images/logo.png 200 12048
    10.0.0.50 POST /api/v1/payment/checkout 500 256
    192.168.1.100 POST /api/v1/comments 201 1024
    10.0.0.51 GET /non-existent-page 404 128
    10.0.0.50 POST /api/v1/payment/checkout 503 256
    dòng-rác-không-đúng-định-dạng
"#;

fn main() -> ExitCode {
    println!("=== KHỞI ĐỘNG DỰ ÁN CAPSTONE: CÔNG CỤ LOGPULSE CLI (VIBE CODING) ===\n");

    // Đối số thật từ terminal; nếu không có, chạy chế độ demo với dữ liệu mẫu
    let real_args: Vec<String> = std::env::args().collect();
    let demo_mode = real_args.len() < 2;
    let args = if demo_mode {
        vec![
            "logpulse".to_string(),
            "<dữ liệu mẫu>".to_string(),
            "--verbose".to_string(),
        ]
    } else {
        real_args
    };

    // 1. Phân tích cờ dòng lệnh
    let config = match CliConfig::parse_from_args(&args) {
        Ok(cfg) => cfg,
        Err(err) => {
            eprintln!("[Lỗi tham số] {}", err);
            return ExitCode::from(1);
        }
    };

    println!(
        "[Khởi tạo] Đang phân tích: {} (Verbose: {})",
        config.target_file, config.verbose
    );

    // 2. Nạp dữ liệu theo luồng vào Động cơ phân tích
    let mut analyzer = LogAnalyzer::new(config.error_only);
    let load_result = if demo_mode {
        // &[u8] cũng cài BufRead, nên dùng chung đường đọc với tệp thật
        analyzer.load_from_reader(SAMPLE_ACCESS_LOG.as_bytes())
    } else {
        File::open(&config.target_file)
            .and_then(|file| analyzer.load_from_reader(BufReader::new(file)))
    };
    if let Err(err) = load_result {
        eprintln!("[Lỗi I/O] Không đọc được '{}': {}", config.target_file, err);
        return ExitCode::from(2);
    }

    // 3. In bảng báo cáo tổng kết ra màn hình
    ReportPrinter::print_summary(&analyzer, &config);
    ExitCode::SUCCESS
}

#[cfg(test)]
mod tests {
    use super::*;

    fn analyze(text: &str, error_only: bool) -> LogAnalyzer {
        let mut analyzer = LogAnalyzer::new(error_only);
        analyzer.load_from_reader(text.as_bytes()).unwrap();
        analyzer
    }

    #[test]
    fn sample_log_statistics() {
        let a = analyze(SAMPLE_ACCESS_LOG, false);
        assert_eq!(a.total_requests(), 8);
        assert_eq!(a.count_server_errors(), 2); // Mã 500 và 503
        assert_eq!(a.count_client_errors(), 2); // Mã 403 và 404
        assert_eq!(a.total_data_transferred_bytes(), 19_344);
        assert_eq!(a.skipped_lines(), 1);
        assert_eq!(a.find_top_client_ip(), Some(("192.168.1.100", 3)));
    }

    #[test]
    fn error_only_flag_filters_successes() {
        let a = analyze(SAMPLE_ACCESS_LOG, true);
        assert_eq!(a.total_requests(), 4);
        assert_eq!(a.find_top_client_ip(), Some(("10.0.0.50", 2)));
    }

    #[test]
    fn parse_line_is_zero_copy_and_rejects_garbage() {
        let line = "1.2.3.4 get /a 200 10";
        let entry = LogAnalyzer::parse_line(line).unwrap();
        assert_eq!(entry.method, HttpMethod::Get);
        // client_ip trỏ thẳng vào bộ nhớ của `line`, không phải bản sao
        assert_eq!(entry.client_ip.as_ptr(), line.as_ptr());
        assert_eq!(LogAnalyzer::parse_line("1.2.3.4 GET /a abc 10"), None);
        assert_eq!(LogAnalyzer::parse_line("1.2.3.4 GET"), None);
    }

    #[test]
    fn top_ip_tie_is_deterministic() {
        let a = analyze("b 1 / 200 1\na 1 / 200 1\n", false);
        assert_eq!(a.find_top_client_ip(), Some(("a", 1)));
    }

    #[test]
    fn cli_rejects_unknown_flag() {
        let args: Vec<String> = ["logpulse", "f.log", "--bogus"]
            .iter()
            .map(|s| s.to_string())
            .collect();
        assert!(CliConfig::parse_from_args(&args).is_err());
        assert!(CliConfig::parse_from_args(&args[..1]).is_err());
    }
}
