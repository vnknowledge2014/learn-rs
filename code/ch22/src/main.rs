#![allow(dead_code, unused_variables, unused_imports)]
// Tệp: src/main.rs
// Chương trình thực chiến làm chủ Tính vệ sinh, Mẫu lặp lại & Đệ quy Macro trong Rust

// ============================================================================
// 1. MACRO CHỨNG MINH TÍNH VỆ SINH (MACRO HYGIENE)
// ============================================================================

macro_rules! phep_tinh_noi_bo {
    ( $input:expr ) => {
        {
            // Khai báo biến tạm mang tên 'value_temp' bên trong macro
            let value_temp = $input * 2;
            println!("  [Trong Macro] gia_tri_tam = {}", value_temp);
            value_temp + 5
        }
    };
}

// ============================================================================
// 2. MACRO MA TRẬN 2D VỚI CÚ PHÁP LẶP LỒNG NHAU: tao_ma_tran!
// ============================================================================

/// Macro tạo Vector lồng nhau (Ma trận 2 chiều) hỗ trợ dấu phẩy tùy chọn ở mọi cấp
macro_rules! tao_ma_tran {
    (
        $(
            [ $( $phan_tu:expr ),* $(,)? ]
        ),*
        $(,)?
    ) => {
        vec![
            $(
                vec![ $( $phan_tu ),* ],
            )*
        ]
    };
}

// ============================================================================
// 3. MACRO ĐỆ QUY TT MUNCHER: tinh_bieu_thuc_chuoi!
// ============================================================================

/// Macro đệ quy phân tích chuỗi phép toán từ trái sang phải
macro_rules! tinh_bieu_thuc_chuoi {
    // Nhánh dừng cơ sở: Chỉ còn lại duy nhất một giá trị
    ( $value:expr ) => {
        $value
    };

    // Nhánh đệ quy phép cộng: (x + y + rest...) -> tinh_bieu_thuc_chuoi!((x + y) + rest...)
    ( $x:expr, +, $y:expr $(, $below:tt )* ) => {
        tinh_bieu_thuc_chuoi!( ($x + $y) $(, $below )* )
    };

    // Nhánh đệ quy phép nhân: (x * y * rest...)
    ( $x:expr, *, $y:expr $(, $below:tt )* ) => {
        tinh_bieu_thuc_chuoi!( ($x * $y) $(, $below )* )
    };

    // Nhánh đệ quy phép trừ: (x - y - rest...)
    ( $x:expr, -, $y:expr $(, $below:tt )* ) => {
        tinh_bieu_thuc_chuoi!( ($x - $y) $(, $below )* )
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
    let value_temp = 7777; // Biến trùng tên ở phạm vi hàm main
    println!("Trước khi gọi macro: gia_tri_tam = {}", value_temp);

    let ket_qua_macro = phep_tinh_noi_bo!(10);
    println!("Kết quả trả về từ macro: {}", ket_qua_macro);

    // Xác nhận biến value_temp ngoài hàm main KHÔNG HỀ BỊ THAY ĐỔI!
    println!("Sau khi gọi macro: gia_tri_tam = {}", value_temp);
    assert_eq!(value_temp, 7777);
    println!("-> KẾT LUẬN: Biến trong macro được cách ly vô trùng tuyệt đối!");

    // ------------------------------------------------------------------------
    // TÌNH HUỐNG 2: Xây dựng Ma trận dữ liệu 2D với Mẫu lặp lồng nhau
    // ------------------------------------------------------------------------
    println!("\n2. Khởi tạo Bảng dữ liệu id trận 2D qua macro lồng nhau:");
    let ma_tran_diem = tao_ma_tran![
        [10, 20, 30,], // Dấu phẩy ở cuối hàng hợp lệ
        [40, 50, 60],
        [70, 80, 90],  // Dấu phẩy ở cuối khối id trận hợp lệ
    ];

    for (num_queue, queue) in ma_tran_diem.iter().enumerate() {
        println!("  Hàng #{}: {:?}", num_queue + 1, queue);
    }
    assert_eq!(ma_tran_diem[1][1], 50);

    // ------------------------------------------------------------------------
    // TÌNH HUỐNG 3: Vận hành TT Muncher phân tích chuỗi phép tính đệ quy
    // ------------------------------------------------------------------------
    println!("\n3. Vận hành Bộ nhai thẻ bài TT Muncher đệ quy:");
    // Tính toán: (((10 + 5) * 2) - 6) = 15 * 2 - 6 = 30 - 6 = 24
    let result_tinh = tinh_bieu_thuc_chuoi!(10, +, 5, *, 2, -, 6);
    println!("Kết quả phân tích đệ quy (10 + 5) * 2 - 6 = {}", result_tinh);
    assert_eq!(result_tinh, 24);

    println!("\n============================================================");
    println!("     XÁC THỰC CÁC MẪU MACRO NÂNG CAO HOÀN THÀNH THÀNH CÔNG  ");
    println!("============================================================");
}
