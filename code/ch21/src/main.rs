#![allow(dead_code, unused_variables, unused_imports)]
// Tệp: src/main.rs
// Chương trình thực chiến làm chủ macro_rules! và Bộ khớp cú pháp trong Rust

use std::collections::HashMap;
use std::time::Instant;

// ============================================================================
// 1. MACRO TẠO NHANH HASHMAP VỚI CÚ PHÁP TỪ ĐIỂN: tao_ban_do!
// ============================================================================

/// Macro nhận vào các cặp $khoa => $value cách nhau bởi dấu phẩy
/// Hỗ trợ dấu phẩy tùy chọn ở cuối cùng $(,)?
macro_rules! tao_ban_do {
    // Nhánh xử lý: $( $khoa:expr => $value:expr ),*
    ( $( $key:expr => $value:expr ),* $(,)? ) => {
        {
            let mut ban_do = HashMap::new();
            $(
                ban_do.insert($key, $value);
            )*
            ban_do
        }
    };
}

// ============================================================================
// 2. MACRO SOI SÁNG VÀ KIỂM TOÁN BIẾN: kiem_toan_bien!
// ============================================================================

/// Macro sử dụng $i:ident và $e:expr kết hợp với stringify!, file!, line!
/// Giúp lập trình viên gỡ lỗi với thông tin vị trí mã nguồn cực kỳ chi tiết
macro_rules! kiem_toan_bien {
    ( $ten_bien:ident ) => {
        println!(
            "[KIỂM TOÁN] Biến `{}` = {:?} (Tại tệp: {}, Dòng: {})",
            stringify!($ten_bien),
            $ten_bien,
            file!(),
            line!()
        );
    };
    ( $nhan_dan:expr, $bieu_thuc:expr ) => {
        println!(
            "[KIỂM TOÁN: {}] Biểu thức `{}` có giá trị = {:?} (Dòng: {})",
            $nhan_dan,
            stringify!($bieu_thuc),
            $bieu_thuc,
            line!()
        );
    };
}

// ============================================================================
// 3. MACRO ĐO THỜI GIAN KHỐI LỆNH: do_luong_thoi_gian!
// ============================================================================

/// Macro nhận một nhãn mô tả $ten:expr và một khối mã $khoi:block
/// Trả về trực tiếp kết quả của khối mã đó!
macro_rules! do_luong_thoi_gian {
    ( $name:expr, $khoi:block ) => {
        {
            println!(">>> [BẮT ĐẦU ĐO] {}", $name);
            let start = Instant::now();
            let ket_qua = $khoi; // Thực thi khối lệnh
            let time_time = start.elapsed();
            println!(">>> [KẾT THÚC] {} hoàn thành trong: {:?}", $name, time_time);
            ket_qua // Trả kết quả của khối lệnh về phía người gọi
        }
    };
}

// ============================================================================
// CHƯƠNG TRÌNH THỰC THI CHÍNH
// ============================================================================

fn main() {
    println!("============================================================");
    println!("     BỘ CÔNG CỤ SIÊU LẬP TRÌNH: DECLARATIVE MACRO RULES     ");
    println!("============================================================");

    // ------------------------------------------------------------------------
    // TÌNH HUỐNG 1: Sử dụng macro tao_ban_do! tạo cấu hình hệ thống
    // ------------------------------------------------------------------------
    println!("\n1. Khởi tạo Bản đồ thông số máy chủ bằng cú pháp trực quan:");
    let thong_num_server = tao_ban_do! {
        "cong_mang" => "8080",
        "ip_address" => "192.168.1.100",
        "moi_truong" => "SanXuat",
        "trang_thai" => "KichHoat", // Hỗ trợ dấu phẩy ở phần tử cuối cùng!
    };

    for (key, value) in &thong_num_server {
        println!("  - Tham số `{}`: {}", key, value);
    }

    // ------------------------------------------------------------------------
    // TÌNH HUỐNG 2: Sử dụng macro kiem_toan_bien! để soi dữ liệu
    // ------------------------------------------------------------------------
    println!("\n2. Soi sáng biến số và biểu thức bằng siêu lập trình:");
    let point_mean = 8.75;
    let danh_sach_lop = vec!["An", "Bình", "Cường"];

    // Gỡ lỗi biến đơn lẻ qua $ident
    kiem_toan_bien!(point_mean);
    kiem_toan_bien!(danh_sach_lop);

    // Gỡ lỗi biểu thức phức tạp qua $expr
    kiem_toan_bien!("Tính toán điểm cộng", point_mean + 1.25);

    // ------------------------------------------------------------------------
    // TÌNH HUỐNG 3: Đo lường khối lệnh tính toán qua do_luong_thoi_gian!
    // ------------------------------------------------------------------------
    println!("\n3. Đo lường hiệu năng của một khối thuật toán:");
    
    let total_tich_accum = do_luong_thoi_gian!("Tính tổng dãy 1 triệu số", {
        let mut tong: u64 = 0;
        for i in 1..=1_000_000 {
            tong += i;
        }
        tong // Giá trị trả về từ khối block
    });

    println!("-> Kết quả tính được từ khối mã: {}", total_tich_accum);

    println!("\n============================================================");
    println!("     XÁC THỰC CÁC MACRO KHAI BÁO HOÀN THÀNH AN TOÀN TUYỆT ĐỐI");
    println!("============================================================");
}
