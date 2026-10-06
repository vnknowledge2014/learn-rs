// Tệp: src/main.rs
// Chương trình thực chiến làm chủ Iterator: map, filter, fold, collect trong Rust

#[derive(Debug, Clone, PartialEq)]
pub struct SensorRecord {
    pub sensor_id: String,
    pub temp_c: f64,
    pub pressure_bar: f64,
    pub is_valid: bool,
}

#[derive(Debug, PartialEq)]
pub struct DangerAlert {
    pub position: usize,
    pub content: String,
    pub severity: String,
}

fn main() {
    println!("============================================================");
    println!("   HỆ THỐNG XỬ LÝ DÒNG DỮ LIỆU CẢM BIẾN NHÀ MÁY (IOT FP)   ");
    println!("============================================================");

    // 1. Khởi tạo danh sách dữ liệu cảm biến thô ban đầu
    let mut raw_data: Vec<SensorRecord> = vec![
        SensorRecord {
            sensor_id: String::from("CB-LO-01"),
            temp_c: 85.5,
            pressure_bar: 3.2,
            is_valid: true,
        },
        SensorRecord {
            sensor_id: String::from("CB-LO-02"),
            temp_c: -999.0, // Dữ liệu lỗi do đứt dây cáp
            pressure_bar: 0.0,
            is_valid: false,
        },
        SensorRecord {
            sensor_id: String::from("CB-LO-03"),
            temp_c: 125.0, // Nhiệt độ quá ngưỡng cảnh báo (> 100°C)
            pressure_bar: 4.8,
            is_valid: true,
        },
        SensorRecord {
            sensor_id: String::from("CB-LO-04"),
            temp_c: 72.0,
            pressure_bar: 2.9,
            is_valid: true,
        },
        SensorRecord {
            sensor_id: String::from("CB-LO-05"),
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
        .filter(|record| record.is_valid)
        .for_each(|record| {
            record.temp_c -= 0.5; // Trừ trực tiếp trên ô nhớ RAM
        });
    println!("-> Đã hiệu chỉnh sai số cho tất cả cảm biến hợp lệ thành công.");

    // ------------------------------------------------------------------------
    // KỸ THUẬT 2: Dùng .iter(), .filter(), .map() xây dựng đường ống lọc & trích xuất
    // Lấy danh sách nhiệt độ của các cảm biến an toàn (nhiệt độ <= 100°C)
    // ------------------------------------------------------------------------
    println!("\n2. Trích xuất danh sách nhiệt độ hoạt động an toàn (<= 100°C):");
    let safe_temps: Vec<f64> = raw_data
        .iter()
        .filter(|record| record.is_valid) // Lọc bỏ cảm biến hỏng
        .filter(|record| record.temp_c <= 100.0) // Lọc cảm biến trong ngưỡng an toàn
        .map(|record| record.temp_c) // Chỉ trích xuất lấy số đo nhiệt độ
        .collect(); // Gom tụ thành Vector mới

    println!("-> Các mức nhiệt độ an toàn: {:?}", safe_temps);

    // ------------------------------------------------------------------------
    // KỸ THUẬT 3: Dùng .fold() để tổng hợp thống kê phức tạp trong một lượt duyệt duy nhất
    // Tính tổng nhiệt độ và đếm số lượng cảm biến an toàn để tính trung bình
    // ------------------------------------------------------------------------
    println!("\n3. Tính nhiệt độ trung bình của phân xưởng qua .fold():");
    let (total_temp, valid_count) = raw_data
        .iter()
        .filter(|record| record.is_valid)
        .fold((0.0, 0usize), |(total, count), record| {
            (total + record.temp_c, count + 1)
        });

    if valid_count > 0 {
        let mean = total_temp / (valid_count as f64);
        println!(
            "-> Tổng nhiệt độ: {:.2}°C trên {} cảm biến.",
            total_temp, valid_count
        );
        println!("-> Nhiệt độ trung bình toàn xưởng: {:.2}°C", mean);
    }

    // ------------------------------------------------------------------------
    // KỸ THUẬT 4: Kết hợp .enumerate(), .filter(), và .collect()
    // Tạo danh sách cảnh báo khẩn cấp cho các cảm biến vượt ngưỡng (> 100°C)
    // ------------------------------------------------------------------------
    println!("\n4. Phát hiện nguy cơ và tổng hợp danh sách cảnh báo khẩn cấp:");
    let alerts: Vec<DangerAlert> = raw_data
        .iter()
        .enumerate() // Cung cấp chỉ số thứ tự (0, 1, 2...) đi kèm với phần tử
        .filter(|(_, record)| record.is_valid && record.temp_c > 100.0)
        .map(|(index, record)| DangerAlert {
            position: index + 1,
            content: format!(
                "Cảm biến [{}] vượt ngưỡng nhiệt độ: {:.2}°C",
                record.sensor_id, record.temp_c
            ),
            severity: String::from("KHẨN CẤP"),
        })
        .collect();

    for alert in &alerts {
        println!(
            "  [!] Vị trí #{}: {} (Mức độ: {})",
            alert.position, alert.content, alert.severity
        );
    }

    // ------------------------------------------------------------------------
    // KỸ THUẬT 5: Dùng .into_iter() để tiêu thụ toàn bộ dữ liệu và giải phóng bộ nhớ
    // ------------------------------------------------------------------------
    println!("\n5. Di chuyển quyền sở hữu toàn bộ qua .into_iter():");
    let all_sensor_ids: Vec<String> = raw_data
        .into_iter()
        .map(|record| record.sensor_id) // Đoạt quyền sở hữu trường String mà không cần clone!
        .collect();

    println!(
        "-> Danh sách mã thiết bị sau khi thu hồi: {:?}",
        all_sensor_ids
    );
    // raw_data đã bị tiêu thụ tại đây, giải phóng bộ nhớ sạch sẽ!

    println!("\n============================================================");
    println!("     XỬ LÝ TOÀN BỘ ĐƯỜNG ỐNG ITERATOR THÀNH CÔNG RỰC RỠ     ");
    println!("============================================================");
}
