#![allow(dead_code, unused_variables, unused_imports)]
// File: src/main.rs
// Chương trình thực hành chuyên sâu về Vay mượn (Borrowing) và Tham chiếu (References)

// 1. Hàm mượn chỉ đọc (&String): Nhận dữ liệu để tính toán nhưng KHÔNG cướp quyền sở hữu
fn series_length(series: &String) -> usize {
    // chuoi là một tham chiếu chỉ đọc, ta chỉ có thể xem nội dung qua .len()
    series.len()
}

// 2. Hàm mượn sửa đổi (&mut String): Cho phép thay đổi trực tiếp nội dung biến gốc
fn add_greeting(chuoi_goc: &mut String) {
    // Phương thức .push_str() ghi thêm ký tự vào bãi đỗ Heap của biến gốc
    chuoi_goc.push_str(" - Chúc bạn một ngày tràn đầy năng lượng!");
}

// 3. Hàm minh họa toán tử giải tham chiếu (Dereferencing '*') với số nguyên
fn double(so: &mut i32) {
    // Dấu * dùng để đi theo địa chỉ con trỏ và can thiệp thẳng vào giá trị thực bên trong ô nhớ
    *so = *so * 2;
}

fn main() {
    println!("============================================================");
    println!("     CHƯƠNG TRÌNH LÀM CHỦ VAY MƯỢN & THAM CHIẾU TRONG RUST  ");
    println!("============================================================");

    // --- PHẦN 1: THAM CHIẾU BẤT BIẾN (&T - MƯỢN ĐỂ ĐỌC) ---
    println!("\n1. Minh họa mượn dữ liệu chỉ để đọc:");
    let thong_tin_xe = String::from("Xe máy Honda SH 150i");

    // Truyền &thong_tin_xe: Ta chỉ đưa "tấm ảnh chụp" địa chỉ ô nhớ cho hàm mượn
    let length = series_length(&thong_tin_xe);
    
    // Biến thong_tin_xe vẫn còn nguyên quyền sở hữu thuộc về hàm main!
    println!("- Xe máy: '{}'", thong_tin_xe);
    println!("- Số lượng ký tự trong chuỗi thông tin: {}", length);

    // Nhiều người có thể cùng mượn đọc đồng thời một lúc:
    let reader_1 = &thong_tin_xe;
    let reader_2 = &thong_tin_xe;
    println!("- Độc giả 1 đọc: {}", reader_1);
    println!("- Độc giả 2 đọc: {}", reader_2);

    // --- PHẦN 2: THAM CHIẾU KHẢ BIẾN (&mut T - MƯỢN ĐỂ SỬA) ---
    println!("\n2. Minh họa mượn dữ liệu để sửa đổi trực tiếp:");
    let mut buc_thu = String::from("Xin chào bạn thân mến");
    println!("- Bức thư ban đầu: '{}'", buc_thu);

    // Mượn để chỉnh sửa nội dung thông qua &mut
    add_greeting(&mut buc_thu);
    println!("- Bức thư sau khi sửa: '{}'", buc_thu);

    // --- PHẦN 3: GIẢI THAM CHIẾU VỚI TOÁN TỬ '*' TRÊN SỐ NGUYÊN ---
    println!("\n3. Thao tác ô nhớ số nguyên với toán tử giải tham chiếu (*):");
    let mut account_xu = 500;
    println!("- Số xu trước khi nhân đôi: {}", account_xu);

    double(&mut account_xu);
    println!("- Số xu sau khi nhân đôi  : {}", account_xu);

    // --- PHẦN 4: LÁT CẮT CHUỖI (STRING SLICES - &str) ---
    println!("\n4. Trích xuất văn bản bằng Lát cắt chuỗi (String Slices):");
    let cau_noi = String::from("Rust an toàn tuyệt đối");

    // Lát cắt trỏ vào một phần ô nhớ của chuỗi mà không tạo dữ liệu mới:
    let first_from: &str = &cau_noi[0..4];    // Cắt từ chỉ số byte 0 đến trước 4 ("Rust")
    let from_two: &str = &cau_noi[5..13];   // Cắt từ chỉ số byte 5 đến trước 13 ("an toàn")

    println!("- Câu nói gốc: '{}'", cau_noi);
    println!("- Từ thứ nhất : '{}' (chiếm {} bytes trên Stack)", first_from, std::mem::size_of_val(&first_from));
    println!("- Từ thứ hai  : '{}'", from_two);

    // --- PHẦN 5: CHỨNG MINH TÍNH LINH HOẠT CỦA NLL (NON-LEXICAL LIFETIMES) ---
    println!("\n5. Kiểm tra cơ chế Vòng đời không từ vựng (NLL):");
    let mut order_log = String::from("Nhật ký ngày 01");

    let read_log = &order_log; // Bắt đầu mượn đọc
    println!("- Đọc nhật ký: {}", read_log);
    // Sau dòng print trên, read_log không còn được dùng nữa -> Hết hiệu lực mượn!

    let fix_log = &mut order_log; // Được phép mượn sửa ngay lập tức mà không xung đột!
    fix_log.push_str(" - Đã ghi thêm sự kiện mới");
    println!("- Nội dung sau cập nhật: {}", fix_log);
}
