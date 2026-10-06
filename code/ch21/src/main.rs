// Tệp: src/main.rs
// Chương trình thực chiến làm chủ macro_rules! và Bộ khớp cú pháp trong Rust

use std::collections::HashMap;
use std::time::Instant;

// ============================================================================
// 1. MACRO TẠO NHANH HASHMAP VỚI CÚ PHÁP TỪ ĐIỂN: hash_map!
// ============================================================================

/// Macro nhận vào các cặp $key => $value cách nhau bởi dấu phẩy
/// Hỗ trợ dấu phẩy tùy chọn ở cuối cùng $(,)?
macro_rules! hash_map {
    // Nhánh xử lý: $( $key:expr => $value:expr ),*
    ( $( $key:expr => $value:expr ),* $(,)? ) => {
        {
            let mut map = HashMap::new();
            $(
                map.insert($key, $value);
            )*
            map
        }
    };
}

// ============================================================================
// 2. MACRO SOI SÁNG VÀ KIỂM TOÁN BIẾN: inspect_var!
// ============================================================================

/// Macro sử dụng $i:ident và $e:expr kết hợp với stringify!, file!, line!
/// Giúp lập trình viên gỡ lỗi với thông tin vị trí mã nguồn cực kỳ chi tiết
macro_rules! inspect_var {
    ( $var:ident ) => {
        println!(
            "[KIỂM TOÁN] Biến `{}` = {:?} (Tại tệp: {}, Dòng: {})",
            stringify!($var),
            $var,
            file!(),
            line!()
        );
    };
    ( $label:expr, $expression:expr ) => {
        println!(
            "[KIỂM TOÁN: {}] Biểu thức `{}` có giá trị = {:?} (Dòng: {})",
            $label,
            stringify!($expression),
            $expression,
            line!()
        );
    };
}

// ============================================================================
// 3. MACRO ĐO THỜI GIAN KHỐI LỆNH: measure_time!
// ============================================================================

/// Macro nhận một nhãn mô tả $name:expr và một khối mã $body:block
/// Trả về trực tiếp kết quả của khối mã đó!
macro_rules! measure_time {
    ( $name:expr, $body:block ) => {{
        println!(">>> [BẮT ĐẦU ĐO] {}", $name);
        let start = Instant::now();
        let result = $body; // Thực thi khối lệnh
        let elapsed = start.elapsed();
        println!(">>> [KẾT THÚC] {} hoàn thành trong: {:?}", $name, elapsed);
        result // Trả kết quả của khối lệnh về phía người gọi
    }};
}

// ============================================================================
// CHƯƠNG TRÌNH THỰC THI CHÍNH
// ============================================================================

fn main() {
    println!("============================================================");
    println!("     BỘ CÔNG CỤ SIÊU LẬP TRÌNH: DECLARATIVE MACRO RULES     ");
    println!("============================================================");

    // ------------------------------------------------------------------------
    // TÌNH HUỐNG 1: Sử dụng macro hash_map! tạo cấu hình hệ thống
    // ------------------------------------------------------------------------
    println!("\n1. Khởi tạo Bản đồ thông số máy chủ bằng cú pháp trực quan:");
    let server_config = hash_map! {
        "port" => "8080",
        "ip_address" => "192.168.1.100",
        "environment" => "production",
        "status" => "active", // Hỗ trợ dấu phẩy ở phần tử cuối cùng!
    };

    for (key, value) in &server_config {
        println!("  - Tham số `{}`: {}", key, value);
    }

    // ------------------------------------------------------------------------
    // TÌNH HUỐNG 2: Sử dụng macro inspect_var! để soi dữ liệu
    // ------------------------------------------------------------------------
    println!("\n2. Soi sáng biến số và biểu thức bằng siêu lập trình:");
    let average_score = 8.75;
    let class_list = vec!["An", "Bình", "Cường"];

    // Gỡ lỗi biến đơn lẻ qua $ident
    inspect_var!(average_score);
    inspect_var!(class_list);

    // Gỡ lỗi biểu thức phức tạp qua $expr
    inspect_var!("Tính toán điểm cộng", average_score + 1.25);

    // ------------------------------------------------------------------------
    // TÌNH HUỐNG 3: Đo lường khối lệnh tính toán qua measure_time!
    // ------------------------------------------------------------------------
    println!("\n3. Đo lường hiệu năng của một khối thuật toán:");

    let total_accumulated = measure_time!("Tính tổng dãy 1 triệu số", {
        let mut total: u64 = 0;
        for i in 1..=1_000_000 {
            total += i;
        }
        total // Giá trị trả về từ khối block
    });

    println!("-> Kết quả tính được từ khối mã: {}", total_accumulated);

    println!("\n============================================================");
    println!("     XÁC THỰC CÁC MACRO KHAI BÁO HOÀN THÀNH AN TOÀN TUYỆT ĐỐI");
    println!("============================================================");
}

// ============================================================================
// KIỂM THỬ
// ============================================================================

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn hash_map_accepts_trailing_comma() {
        let m = hash_map! { "a" => 1, "b" => 2, };
        assert_eq!(m.len(), 2);
        assert_eq!(m.get("b"), Some(&2));
    }

    #[test]
    fn measure_time_returns_block_value() {
        let v = measure_time!("cộng", { 40 + 2 });
        assert_eq!(v, 42);
    }
}
