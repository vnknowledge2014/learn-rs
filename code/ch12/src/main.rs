#![allow(dead_code, unused_variables, unused_imports)]
// File: src/main.rs
// Chương trình thực chiến làm chủ Generics, Traits & Tổ chức Mô-đun trong Rust

// ============================================================================
// MÔ-ĐUN 1: CÁC GIAO ƯỚC VÀ THIẾT BỊ PHẦN CỨNG
// ============================================================================
mod thiet_bi_thong_minh {
    use std::fmt::Display;

    // 1. Định nghĩa Trait giao ước cho mọi cảm biến trong tòa nhà
    pub trait Sensor: Display {
        // Phương thức bắt buộc mọi cảm biến phải tự hiện thực
        fn read_value(&self) -> f64;
        fn don_pos_do(&self) -> &str;

        // Phương thức mặc định (Default implementation): Dùng chung cho tất cả cảm biến
        fn check_computed_state(&self) {
            println!("-> Cảm biến [{}] đang hoạt động bình thường.", self);
        }
    }

    // 2. Struct Cảm biến Nhiệt độ phòng
    pub struct TempSensor {
        pub pos_value: String,
        pub do_c: f64,
    }

    // Cài đặt Display cho TempSensor (thỏa mãn điều kiện Sensor: Display)
    impl Display for TempSensor {
        fn fmt(&self, f: &mut std::fmt::Formatter) -> std::fmt::Result {
            write!(f, "Cảm biến Nhiệt độ tại {}", self.pos_value)
        }
    }

    // Triển khai Trait Sensor cho TempSensor
    impl Sensor for TempSensor {
        fn read_value(&self) -> f64 { self.do_c }
        fn don_pos_do(&self) -> &str { "°C" }
    }

    // 3. Struct Cảm biến Khói báo cháy
    pub struct SmokeSensor {
        pub khu_vuc: String,
        pub mat_do_khoi_ppm: f64,
    }

    impl Display for SmokeSensor {
        fn fmt(&self, f: &mut std::fmt::Formatter) -> std::fmt::Result {
            write!(f, "Cảm biến Khói tại {}", self.khu_vuc)
        }
    }

    impl Sensor for SmokeSensor {
        fn read_value(&self) -> f64 { self.mat_do_khoi_ppm }
        fn don_pos_do(&self) -> &str { "PPM" }
    }
}

// ============================================================================
// MÔ-ĐUN 2: TRUNG TÂM GIÁM SÁT TỔNG HỢP VÀ HÀM GENERICS
// ============================================================================
mod trung_tam_dieu_khien {
    use super::thiet_bi_thong_minh::Sensor;

    // Hàm Generics nhận bất kỳ cảm biến nào tuân thủ Trait Sensor
    // Sử dụng mệnh đề 'where' để cấu trúc mã sạch đẹp và chuyên nghiệp
    pub fn monitor_metrics<T>(cam_bien: &T, nguong_canh_bao: f64)
    where
        T: Sensor,
    {
        println!("------------------------------------------------------------");
        // Gọi phương thức mặc định của Trait
        cam_bien.check_computed_state();

        let value = cam_bien.read_value();
        let don_pos = cam_bien.don_pos_do();

        println!("Chỉ số đo được : {:.2} {}", value, don_pos);

        if value >= nguong_canh_bao {
            println!("[CẢNH BÁO NGUY HIỂM] Chỉ số vượt ngưỡng an toàn ({:.2} {})!", 
                     nguong_canh_bao, don_pos);
        } else {
            println!("[AN TOÀN] Chỉ số nằm trong giới hạn cho phép.");
        }
    }
}

// Sử dụng lệnh 'use' để đưa các thành phần cần thiết vào phạm vi làm việc
use thiet_bi_thong_minh::{TempSensor, SmokeSensor};
use trung_tam_dieu_khien::monitor_metrics;

fn main() {
    println!("============================================================");
    println!("     HỆ THỐNG ĐIỀU HÀNH TỰ ĐỘNG HÓA TÒA NHÀ THÔNG MINH      ");
    println!("============================================================");

    // Khởi tạo cảm biến nhiệt độ phòng máy chủ
    let cb_nhiet = TempSensor {
        pos_value: String::from("Phòng Máy Chủ Tầng 5"),
        do_c: 28.5,
    };

    // Khởi tạo cảm biến khói khu nhà bếp
    let cb_khoi = SmokeSensor {
        khu_vuc: String::from("Khu Bếp Nhà Hàng Tầng 1"),
        mat_do_khoi_ppm: 65.0,
    };

    // Cùng một hàm monitor_metrics nhưng nhận hai kiểu dữ liệu khác nhau!
    // Trình biên dịch Rust áp dụng Monomorphization tối ưu hóa mã máy hoàn hảo:
    println!("\n1. Giám sát hệ thống cảm biến nhiệt độ:");
    monitor_metrics(&cb_nhiet, 35.0); // Ngưỡng cảnh báo nhiệt độ là 35°C

    println!("\n2. Giám sát hệ thống cảm biến khói báo cháy:");
    monitor_metrics(&cb_khoi, 50.0);  // Ngưỡng cảnh báo mật độ khói là 50 PPM

    println!("\n============================================================");
    println!("   CHÚC MỪNG BẠN ĐÃ HOÀN THÀNH TOÀN BỘ 12 CHƯƠNG NỀN TẢNG!  ");
    println!("============================================================");
}
