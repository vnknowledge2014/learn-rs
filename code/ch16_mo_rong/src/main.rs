#![allow(dead_code, unused_variables, unused_imports)]
// Tệp: src/main.rs
// Chương trình thực chiến làm chủ Iterator: map, filter, fold, collect trong Rust

#[derive(Debug, Clone, PartialEq)]
pub struct SensorRecord {
    pub ma_cam_bien: String,
    pub temp_c: f64,
    pub pressure_bar: f64,
    pub is_valid: bool,
}

#[derive(Debug, PartialEq)]
pub struct ThongReportUnsafe {
    pub fold_records: usize,
    pub content: String,
    pub level_do: String,
}

fn main() {
    println!("============================================================");
    println!("   HỆ THỐNG XỬ LÝ DÒNG DỮ LIỆU CẢM BIẾN NHÀ MÁY (IOT FP)   ");
    println!("============================================================");

    // 1. Khởi tạo danh sách dữ liệu cảm biến thô ban đầu
    let mut raw_data: Vec<SensorRecord> = vec![
        SensorRecord {
            ma_cam_bien: String::from("CB-LO-01"),
            temp_c: 85.5,
            pressure_bar: 3.2,
            is_valid: true,
        },
        SensorRecord {
            ma_cam_bien: String::from("CB-LO-02"),
            temp_c: -999.0, // Dữ liệu lỗi do đứt dây cáp
            pressure_bar: 0.0,
            is_valid: false,
        },
        SensorRecord {
            ma_cam_bien: String::from("CB-LO-03"),
            temp_c: 125.0, // Nhiệt độ quá ngưỡng cảnh báo (> 100°C)
            pressure_bar: 4.8,
            is_valid: true,
        },
        SensorRecord {
            ma_cam_bien: String::from("CB-LO-04"),
            temp_c: 72.0,
            pressure_bar: 2.9,
            is_valid: true,
        },
        SensorRecord {
            ma_cam_bien: String::from("CB-LO-05"),
            temp_c: 110.5, // Nhiệt độ quá ngưỡng cảnh báo (> 100°C)
            pressure_bar: 5.1,
            is_valid: true,
        },
    ];

    println!("Số lượng bản ghi thu thập được: {}", raw_data.len());

    // ------------------------------------------------------------------------
    // KỸ THUẬT 1: Dùng .iter_mut() để hiệu chỉnh dữ liệu trực tiếp tại chỗ
    // Giả sử cảm biến có sai số cố định +0.5°C cần được bù trừ
    // ------------------------------------------------------------------------
    println!("\n1. Tiến hành bù trừ sai số thiết bị qua .iter_mut():");
    raw_data
        .iter_mut()
        .filter(|sell_record| sell_record.is_valid)
        .for_each(|sell_record| {
            sell_record.temp_c -= 0.5; // Trừ trực tiếp trên ô nhớ RAM
        });
    println!("-> Đã hiệu chỉnh sai số cho tất cả cảm biến hợp lệ thành công.");

    // ------------------------------------------------------------------------
    // KỸ THUẬT 2: Dùng .iter(), .filter(), .map() xây dựng đường ống lọc & trích xuất
    // Lấy danh sách nhiệt độ của các cảm biến an toàn (nhiệt độ <= 100°C)
    // ------------------------------------------------------------------------
    println!("\n2. Trích xuất danh sách nhiệt độ hoạt động an toàn (<= 100°C):");
    let nhiet_do_an_toan: Vec<f64> = raw_data
        .iter()
        .filter(|bg| bg.is_valid)                  // Lọc bỏ cảm biến hỏng
        .filter(|bg| bg.temp_c <= 100.0)     // Lọc cảm biến trong ngưỡng an toàn
        .map(|bg| bg.temp_c)                 // Chỉ trích xuất lấy số đo nhiệt độ
        .collect();                              // Gom tụ thành Vector mới

    println!("-> Các mức nhiệt độ an toàn: {:?}", nhiet_do_an_toan);

    // ------------------------------------------------------------------------
    // KỸ THUẬT 3: Dùng .fold() để tổng hợp thống kê phức tạp trong một lượt duyệt duy nhất
    // Tính tổng nhiệt độ và đếm số lượng cảm biến an toàn để tính trung bình
    // ------------------------------------------------------------------------
    println!("\n3. Tính nhiệt độ trung bình của phân xưởng qua .fold():");
    let (tong_nhiet, quantity) = raw_data
        .iter()
        .filter(|bg| bg.is_valid)
        .fold((0.0, 0usize), |(tong, count), bg| {
            (tong + bg.temp_c, count + 1)
        });

    if quantity > 0 {
        let mean = tong_nhiet / (quantity as f64);
        println!("-> Tổng nhiệt độ: {:.2}°C trên {} cảm biến.", tong_nhiet, quantity);
        println!("-> Nhiệt độ trung bình toàn xưởng: {:.2}°C", mean);
    }

    // ------------------------------------------------------------------------
    // KỸ THUẬT 4: Kết hợp .enumerate(), .filter(), và .collect()
    // Tạo danh sách cảnh báo khẩn cấp cho các cảm biến vượt ngưỡng (> 100°C)
    // ------------------------------------------------------------------------
    println!("\n4. Phát hiện nguy cơ và tổng hợp danh sách cảnh báo khẩn cấp:");
    let list_edge_report: Vec<ThongReportUnsafe> = raw_data
        .iter()
        .enumerate() // Cung cấp chỉ số thứ tự (0, 1, 2...) đi kèm với phần tử
        .filter(|(_, bg)| bg.is_valid && bg.temp_c > 100.0)
        .map(|(chi_so, bg)| ThongReportUnsafe {
            fold_records: chi_so + 1,
            content: format!("Cảm biến [{}] vượt ngưỡng nhiệt độ: {:.2}°C", bg.ma_cam_bien, bg.temp_c),
            level_do: String::from("KHẨN CẤP"),
        })
        .collect();

    for cb in &list_edge_report {
        println!("  [!] Vị trí #{}: {} (Mức độ: {})", 
                 cb.fold_records, cb.content, cb.level_do);
    }

    // ------------------------------------------------------------------------
    // KỸ THUẬT 5: Dùng .into_iter() để tiêu thụ toàn bộ dữ liệu và giải phóng bộ nhớ
    // ------------------------------------------------------------------------
    println!("\n5. Di chuyển quyền sở hữu toàn bộ qua .into_iter():");
    let ma_tat_ca_cam_bien: Vec<String> = raw_data
        .into_iter()
        .map(|bg| bg.ma_cam_bien) // Đoạt quyền sở hữu trường String mà không cần clone!
        .collect();

    println!("-> Danh sách mã thiết bị sau khi thu hồi: {:?}", ma_tat_ca_cam_bien);
    // raw_data đã bị tiêu thụ tại đây, giải phóng bộ nhớ sạch sẽ!

    println!("\n============================================================");
    println!("     XỬ LÝ TOÀN BỘ ĐƯỜNG ỐNG ITERATOR THÀNH CÔNG RỰC RỠ     ");
    println!("============================================================");
}
