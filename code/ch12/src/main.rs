// File: src/main.rs
// Chương trình thực chiến làm chủ Generics, Traits & Tổ chức Mô-đun trong Rust

// ============================================================================
// MÔ-ĐUN 1: CÁC GIAO ƯỚC VÀ THIẾT BỊ PHẦN CỨNG
// ============================================================================
mod smart_devices {
    use std::fmt::Display;

    // 1. Định nghĩa Trait giao ước cho mọi cảm biến trong tòa nhà
    pub trait Sensor: Display {
        // Phương thức bắt buộc mọi cảm biến phải tự hiện thực
        fn read_value(&self) -> f64;
        fn unit(&self) -> &str;

        // Phương thức mặc định (Default implementation): Dùng chung cho tất cả cảm biến
        fn report_status(&self) {
            println!("-> Cảm biến [{}] đang hoạt động bình thường.", self);
        }
    }

    // 2. Struct Cảm biến Nhiệt độ phòng
    pub struct TempSensor {
        pub location: String,
        pub celsius: f64,
    }

    // Cài đặt Display cho TempSensor (thỏa mãn điều kiện Sensor: Display)
    impl Display for TempSensor {
        fn fmt(&self, f: &mut std::fmt::Formatter) -> std::fmt::Result {
            write!(f, "Cảm biến Nhiệt độ tại {}", self.location)
        }
    }

    // Triển khai Trait Sensor cho TempSensor
    impl Sensor for TempSensor {
        fn read_value(&self) -> f64 {
            self.celsius
        }
        fn unit(&self) -> &str {
            "°C"
        }
    }

    // 3. Struct Cảm biến Khói báo cháy
    pub struct SmokeSensor {
        pub area: String,
        pub smoke_ppm: f64,
    }

    impl Display for SmokeSensor {
        fn fmt(&self, f: &mut std::fmt::Formatter) -> std::fmt::Result {
            write!(f, "Cảm biến Khói tại {}", self.area)
        }
    }

    impl Sensor for SmokeSensor {
        fn read_value(&self) -> f64 {
            self.smoke_ppm
        }
        fn unit(&self) -> &str {
            "PPM"
        }
    }
}

// ============================================================================
// MÔ-ĐUN 2: TRUNG TÂM GIÁM SÁT TỔNG HỢP VÀ HÀM GENERICS
// ============================================================================
mod control_center {
    use super::smart_devices::Sensor;

    // Hàm Generics nhận bất kỳ cảm biến nào tuân thủ Trait Sensor
    // Sử dụng mệnh đề 'where' để cấu trúc mã sạch đẹp và chuyên nghiệp
    pub fn monitor_metrics<T>(sensor: &T, alert_threshold: f64)
    where
        T: Sensor,
    {
        println!("------------------------------------------------------------");
        // Gọi phương thức mặc định của Trait
        sensor.report_status();

        let value = sensor.read_value();
        let unit = sensor.unit();

        println!("Chỉ số đo được : {:.2} {}", value, unit);

        if value >= alert_threshold {
            println!(
                "[CẢNH BÁO NGUY HIỂM] Chỉ số vượt ngưỡng an toàn ({:.2} {})!",
                alert_threshold, unit
            );
        } else {
            println!("[AN TOÀN] Chỉ số nằm trong giới hạn cho phép.");
        }
    }
}

// Sử dụng lệnh 'use' để đưa các thành phần cần thiết vào phạm vi làm việc
use control_center::monitor_metrics;
use smart_devices::{SmokeSensor, TempSensor};

fn main() {
    println!("============================================================");
    println!("     HỆ THỐNG ĐIỀU HÀNH TỰ ĐỘNG HÓA TÒA NHÀ THÔNG MINH      ");
    println!("============================================================");

    // Khởi tạo cảm biến nhiệt độ phòng máy chủ
    let temp_sensor = TempSensor {
        location: String::from("Phòng Máy Chủ Tầng 5"),
        celsius: 28.5,
    };

    // Khởi tạo cảm biến khói khu nhà bếp
    let smoke_sensor = SmokeSensor {
        area: String::from("Khu Bếp Nhà Hàng Tầng 1"),
        smoke_ppm: 65.0,
    };

    // Cùng một hàm monitor_metrics nhưng nhận hai kiểu dữ liệu khác nhau!
    // Trình biên dịch Rust áp dụng Monomorphization tối ưu hóa mã máy hoàn hảo:
    println!("\n1. Giám sát hệ thống cảm biến nhiệt độ:");
    monitor_metrics(&temp_sensor, 35.0); // Ngưỡng cảnh báo nhiệt độ là 35°C

    println!("\n2. Giám sát hệ thống cảm biến khói báo cháy:");
    monitor_metrics(&smoke_sensor, 50.0); // Ngưỡng cảnh báo mật độ khói là 50 PPM

    println!("\n============================================================");
    println!("   CHÚC MỪNG BẠN ĐÃ HOÀN THÀNH TOÀN BỘ 12 CHƯƠNG NỀN TẢNG!  ");
    println!("============================================================");
}
