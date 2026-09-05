#![allow(dead_code, unused_variables, unused_imports)]
// ============================================================================
// CHƯƠNG 42: MINH HỌA TRÌNH BIÊN DỊCH LÀ TRỌNG TÀI TỐI CAO & TÁI CẤU TRÚC MÃ
// Tác giả: Kỹ Sư Hệ Thống Rust
// ============================================================================

// ----------------------------------------------------------------------------
// PHẦN 1: MÔ HÌNH DỮ LIỆU ĐO KIỂM HIỆU NĂNG GIAO DỊCH
// ----------------------------------------------------------------------------
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct MetricRecord {
    pub service_name: String,
    pub response_time_ms: u32,
    pub is_success: bool,
}

impl MetricRecord {
    pub fn new(service: &str, time_ms: u32, success: bool) -> Self {
        Self {
            service_name: service.to_string(),
            response_time_ms: time_ms,
            is_success: success,
        }
    }
}

// ----------------------------------------------------------------------------
// PHẦN 2: PHONG CÁCH CŨ (TRƯỚC KHI TÁI CẤU TRÚC)
// Vấn đề: Cấp phát bộ nhớ thừa thãi qua `.clone()`, dùng chỉ số mảng dễ lỗi
// ----------------------------------------------------------------------------
pub fn filter_slow_services_old(records: &Vec<MetricRecord>, threshold_ms: u32) -> Vec<String> {
    let mut slow_services: Vec<String> = Vec::new();

    // Dùng vòng lặp chỉ số và sao chép toàn bộ chuỗi String không cần thiết
    for i in 0..records.len() {
        if records[i].is_success && records[i].response_time_ms > threshold_ms {
            // Lạm dụng .clone() gây lãng phí bộ nhớ Heap
            let name_copy = records[i].service_name.clone();
            if !slow_services.contains(&name_copy) {
                slow_services.push(name_copy);
            }
        }
    }

    slow_services
}

// ----------------------------------------------------------------------------
// PHẦN 3: PHONG CÁCH CHUẨN RUST HIỆN ĐẠI (SAU KHI AI ĐƯỢC HƯỚNG DẪN TÁI CẤU TRÚC)
// Ưu điểm:
// 1. Nhận lát cắt `&[MetricRecord]` thay vì tham chiếu cụ thể `&Vec<MetricRecord>`
// 2. Tận dụng đường ống Iterator: filter, map
// 3. Mượn tham chiếu chuỗi `&str` thay vì nhân bản vô tội vạ, tiết kiệm 100% chi phí cấp phát
// ----------------------------------------------------------------------------
pub fn filter_slow_services_idiomatic<'a>(
    records: &'a [MetricRecord],
    threshold_ms: u32,
) -> Vec<&'a str> {
    // Thu thập danh sách các lát cắt chuỗi không sao chép (Zero-Copy)
    let mut results: Vec<&'a str> = records
        .iter()
        .filter(|r| r.is_success && r.response_time_ms > threshold_ms)
        .map(|r| r.service_name.as_str())
        .collect();

    // Loại bỏ các phần tử trùng lặp một cách thanh lịch
    results.sort_unstable();
    results.dedup();
    results
}

// ----------------------------------------------------------------------------
// PHẦN 4: BỘ PHÂN TÍCH VÀ BÁO CÁO THỐNG KÊ (METRICS SUMMARY ENGINE)
// Minh họa sự an toàn tuyệt đối khi quản lý quyền sở hữu (ownership)
// ----------------------------------------------------------------------------
pub struct MetricsAnalyzer<'a> {
    pub records: &'a [MetricRecord],
}

impl<'a> MetricsAnalyzer<'a> {
    pub fn new(records: &'a [MetricRecord]) -> Self {
        Self { records }
    }

    // Tính toán thời gian phản hồi trung bình của các yêu cầu thành công
    pub fn calculate_average_success_time(&self) -> Option<u32> {
        let (total_time, count) = self
            .records
            .iter()
            .filter(|r| r.is_success)
            .fold((0u64, 0u64), |(acc_time, acc_count), r| {
                (acc_time + r.response_time_ms as u64, acc_count + 1)
            });

        if count == 0 {
            None
        } else {
            Some((total_time / count) as u32)
        }
    }
}

// ----------------------------------------------------------------------------
// PHẦN 5: HÀM MAIN KIỂM CHỨNG KẾT QUẢ ĐỐI CHIẾU
// ----------------------------------------------------------------------------
fn main() {
    println!("=== CHƯƠNG 42: KIỂM CHỨNG TÁI CẤU TRÚC MÃ & TRỌNG TÀI BIÊN DỊCH RUST ===");

    // Tạo tập dữ liệu đo kiểm giả lập
    let metrics = vec![
        MetricRecord::new("AuthService", 120, true),
        MetricRecord::new("PaymentGateway", 450, true), // Chậm (> 300ms)
        MetricRecord::new("EmailNotifier", 80, true),
        MetricRecord::new("OrderProcessor", 620, true), // Chậm (> 300ms)
        MetricRecord::new("PaymentGateway", 510, true), // Chậm trùng lặp (> 300ms)
        MetricRecord::new("AnalyticsService", 990, false), // Chậm nhưng thất bại -> bỏ qua
    ];

    println!("Tập dữ liệu đầu vào gồm {} bản ghi đo lường.", metrics.len());

    // 1. Chạy phương pháp cũ
    let slow_old = filter_slow_services_old(&metrics, 300);
    println!("\n[Cách viết cũ] Danh sách dịch vụ chậm: {:?}", slow_old);

    // 2. Chạy phương pháp mới sau tái cấu trúc (Zero-copy)
    let slow_idiomatic = filter_slow_services_idiomatic(&metrics, 300);
    println!("[Sau tái cấu trúc] Danh sách dịch vụ chậm (Zero-Copy): {:?}", slow_idiomatic);

    // Xác nhận hai phương pháp cho cùng kết quả nghiệp vụ chính xác
    assert_eq!(slow_old.len(), slow_idiomatic.len());
    for name in &slow_idiomatic {
        assert!(slow_old.contains(&name.to_string()));
    }

    // 3. Phân tích thống kê với MetricsAnalyzer
    let analyzer = MetricsAnalyzer::new(&metrics);
    if let Some(avg) = analyzer.calculate_average_success_time() {
        println!("\n[Thống kê] Thời gian phản hồi trung bình của các dịch vụ thành công: {} ms", avg);
    }

    println!("\n[Tổng kết] Mã nguồn sau khi tái cấu trúc hoàn toàn sạch sẽ, không tốn tài nguyên cấp phát dư thừa!");
}
