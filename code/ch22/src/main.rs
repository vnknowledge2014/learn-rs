// Tệp: src/main.rs
// Chương trình thực chiến làm chủ Tính vệ sinh, Mẫu lặp lại & Đệ quy Macro trong Rust

// ============================================================================
// 1. MACRO CHỨNG MINH TÍNH VỆ SINH (MACRO HYGIENE)
// ============================================================================

macro_rules! internal_calc {
    ( $input:expr ) => {{
        // Khai báo biến tạm mang tên 'temp_value' bên trong macro
        let temp_value = $input * 2;
        println!("  [Trong Macro] temp_value = {}", temp_value);
        temp_value + 5
    }};
}

// ============================================================================
// 2. MACRO MA TRẬN 2D VỚI CÚ PHÁP LẶP LỒNG NHAU: matrix!
// ============================================================================

/// Macro tạo Vector lồng nhau (Ma trận 2 chiều) hỗ trợ dấu phẩy tùy chọn ở mọi cấp
macro_rules! matrix {
    (
        $(
            [ $( $item:expr ),* $(,)? ]
        ),*
        $(,)?
    ) => {
        vec![
            $(
                vec![ $( $item ),* ],
            )*
        ]
    };
}

// ============================================================================
// 3. MACRO ĐỆ QUY TT MUNCHER: eval_chain!
// ============================================================================

/// Macro đệ quy phân tích chuỗi phép toán từ trái sang phải
macro_rules! eval_chain {
    // Nhánh dừng cơ sở: Chỉ còn lại duy nhất một giá trị
    ( $value:expr ) => {
        $value
    };

    // Nhánh đệ quy phép cộng: (x, +, y, rest...) -> eval_chain!((x + y), rest...)
    ( $x:expr, +, $y:expr $(, $rest:tt )* ) => {
        eval_chain!( ($x + $y) $(, $rest )* )
    };

    // Nhánh đệ quy phép nhân: (x * y * rest...)
    ( $x:expr, *, $y:expr $(, $rest:tt )* ) => {
        eval_chain!( ($x * $y) $(, $rest )* )
    };

    // Nhánh đệ quy phép trừ: (x - y - rest...)
    ( $x:expr, -, $y:expr $(, $rest:tt )* ) => {
        eval_chain!( ($x - $y) $(, $rest )* )
    };
}

// ============================================================================
// CHƯƠNG TRÌNH THỰC THI CHÍNH
// ============================================================================

fn main() {
    println!("============================================================");
    println!("     NÂNG CAO METAPROGRAMMING: HYGIENE, REPETITIONS & TT    ");
    println!("============================================================");

    // ------------------------------------------------------------------------
    // TÌNH HUỐNG 1: Kiểm chứng Tính vệ sinh không làm ô nhiễm biến ngoài
    // ------------------------------------------------------------------------
    println!("\n1. Kiểm chứng Tính vệ sinh của Macro (Macro Hygiene):");
    let temp_value = 7777; // Biến trùng tên ở phạm vi hàm main
    println!("Trước khi gọi macro: temp_value = {}", temp_value);

    let macro_result = internal_calc!(10);
    println!("Kết quả trả về từ macro: {}", macro_result);

    // Xác nhận biến temp_value ngoài hàm main KHÔNG HỀ BỊ THAY ĐỔI!
    println!("Sau khi gọi macro: temp_value = {}", temp_value);
    assert_eq!(temp_value, 7777);
    println!("-> KẾT LUẬN: Biến trong macro được cách ly vô trùng tuyệt đối!");

    // ------------------------------------------------------------------------
    // TÌNH HUỐNG 2: Xây dựng Ma trận dữ liệu 2D với Mẫu lặp lồng nhau
    // ------------------------------------------------------------------------
    println!("\n2. Khởi tạo Bảng dữ liệu ma trận 2D qua macro lồng nhau:");
    let score_matrix = matrix![
        [10, 20, 30,], // Dấu phẩy ở cuối hàng hợp lệ
        [40, 50, 60],
        [70, 80, 90], // Dấu phẩy ở cuối khối ma trận hợp lệ
    ];

    for (row_index, row) in score_matrix.iter().enumerate() {
        println!("  Hàng #{}: {:?}", row_index + 1, row);
    }
    assert_eq!(score_matrix[1][1], 50);

    // ------------------------------------------------------------------------
    // TÌNH HUỐNG 3: Vận hành TT Muncher phân tích chuỗi phép tính đệ quy
    // ------------------------------------------------------------------------
    println!("\n3. Vận hành Bộ nhai thẻ bài TT Muncher đệ quy:");
    // Tính toán: (((10 + 5) * 2) - 6) = 15 * 2 - 6 = 30 - 6 = 24
    let computed_result = eval_chain!(10, +, 5, *, 2, -, 6);
    println!(
        "Kết quả phân tích đệ quy (10 + 5) * 2 - 6 = {}",
        computed_result
    );
    assert_eq!(computed_result, 24);

    println!("\n============================================================");
    println!("     XÁC THỰC CÁC MẪU MACRO NÂNG CAO HOÀN THÀNH THÀNH CÔNG  ");
    println!("============================================================");
}

// ============================================================================
// KIỂM THỬ
// ============================================================================

#[cfg(test)]
mod tests {
    #[test]
    fn hygiene_keeps_outer_variable() {
        let temp_value = 1;
        let r = internal_calc!(10); // trong macro: temp_value = 20
        assert_eq!(r, 25);
        assert_eq!(temp_value, 1);
    }

    #[test]
    fn matrix_accepts_trailing_commas() {
        let m = matrix![[1, 2,], [3, 4],];
        assert_eq!(m, vec![vec![1, 2], vec![3, 4]]);
    }

    #[test]
    fn eval_chain_is_left_to_right() {
        // Trái sang phải, KHÔNG theo thứ tự ưu tiên: (2 + 3) * 4 = 20, không phải 14
        assert_eq!(eval_chain!(2, +, 3, *, 4), 20);
        assert_eq!(eval_chain!(7), 7);
    }
}
