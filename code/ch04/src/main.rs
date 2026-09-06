#![allow(dead_code, unused_variables, unused_imports)]
// File: src/main.rs
// Chương trình mô phỏng Trạm Kiểm Soát Phóng Tên Lửa Vũ Trụ

fn main() {
    println!("============================================================");
    println!("       TRẠM ĐIỀU HÀNH VŨ TRỤ - CHƯƠNG TRÌNH PHÓNG TÊN LỬA    ");
    println!("============================================================");

    // 1. Sử dụng if như một biểu thức để xác định trạng thái thời tiết
    let toc_do_gio_kmh = 25;
    let troi_mua = false;

    // if/else trả về trực tiếp chuỗi trạng thái được gán vào biến
    let dieu_kien_thoi_tiet = if toc_do_gio_kmh < 40 && !troi_mua {
        "Hoàn hảo để phóng"
    } else if toc_do_gio_kmh < 60 {
        "Cần theo dõi thêm sức gió"
    } else {
        "Hủy lịch phóng vì thời tiết xấu"
    };
    println!("Tình trạng khí tượng hiện tại: {}", dieu_kien_thoi_tiet);

    // 2. Sử dụng vòng lặp 'loop' có 'break' mang giá trị về:
    // Kiểm tra áp suất nhiên liệu buồng đốt đến khi đạt chuẩn an toàn
    let mut current_pressure = 80;
    println!("\nBắt đầu kích áp buồng đốt nhiên liệu...");

    let ap_suat_chot = loop {
        current_pressure += 5;
        println!("- Áp suất đang tăng: {} PSI", current_pressure);

        if current_pressure >= 100 {
            // Khi áp suất đạt ngưỡng 100 PSI, thoát vòng lặp và mang giá trị về!
            break current_pressure;
        }
    };
    println!("==> Áp suất buồng đốt đã khóa an toàn tại mức: {} PSI", ap_suat_chot);

    // 3. Sử dụng vòng lặp 'while' để nạp năng lượng bình ắc-quy phụ
    let mut battery_capacity = 85;
    println!("\nĐang sạc bù hệ thống năng lượng dự phòng:");
    while battery_capacity < 100 {
        battery_capacity += 5;
        println!("  Đang sạc... mức pin hiện tại: {}%", battery_capacity);
    }
    println!("==> Hệ thống ắc-quy phụ đã đạt 100%!");

    // 4. Sử dụng vòng lặp lồng nhau với Nhãn (Loop Labels) để quét cảm biến
    println!("\nBắt đầu diễn tập kịch bản ngắt khẩn cấp trên 3 tầng tên lửa:");
    let mut has_emitted = false;

    'kiem_tra_tang_ten_lua: for tang in 1..=3 {
        println!("* Đang quét tầng tên lửa số {}", tang);
        for cam_bien in 1..=4 {
            if tang == 2 && cam_bien == 3 {
                has_emitted = true; // Kích hoạt sự cố mô phỏng!
                println!("  [!] Phát hiện sự cố tại tầng {}, cảm biến {}! Kích hoạt ngắt khẩn cấp!", 
                         tang, cam_bien);
                // Thoát thẳng ra ngoài cả hai vòng lặp nhờ nhãn:
                break 'kiem_tra_tang_ten_lua;
            }
            println!("  - Cảm biến {}.{} hoạt động bình thường", tang, cam_bien);
        }
    }

    if has_emitted {
        println!("==> Cơ chế ngắt khẩn cấp bằng nhãn đã dừng kiểm tra an toàn!");
        println!("==> Đội kỹ thuật đã khắc phục xong sự cố cảm biến 2.3.");
    }

    // 5. Vòng lặp 'for' an toàn đếm ngược thời gian phóng tên lửa
    // (1..=5).rev() tạo ra dãy số: 5, 4, 3, 2, 1
    println!("\nTẤT CẢ HỆ THỐNG SẴN SÀNG! ĐẾM NGƯỢC ĐỂ PHÓNG:");
    for giay in (1..=5).rev() {
        println!("T-minus {} giây...", giay);
    }

    println!("\n🚀 KHAI HỎA ĐỘNG CƠ CHÍNH! TÊN LỬA ĐÃ RỜI BỆ PHÓNG THÀNH CÔNG! 🚀");
}
