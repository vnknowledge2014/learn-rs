// File: src/main.rs
// Chương trình thực hành chuyên sâu về Vay mượn (Borrowing) và Tham chiếu (References)

// 1. Hàm mượn chỉ đọc (&String): Nhận dữ liệu để tính toán nhưng KHÔNG cướp quyền sở hữu
// Clippy sẽ gợi ý dùng `&str` thay cho `&String` (lint ptr_arg) — đó mới là cách viết
// thành ngữ (xem mục Lát cắt chuỗi). Ở đây cố ý giữ `&String` để minh hoạ "mượn đúng kiểu gốc".
#[allow(clippy::ptr_arg)]
fn string_length(text: &String) -> usize {
    // text là một tham chiếu chỉ đọc, ta chỉ có thể xem nội dung, ví dụ qua .len()
    text.len()
}

// 2. Hàm mượn sửa đổi (&mut String): Cho phép thay đổi trực tiếp nội dung biến gốc
fn add_greeting(text: &mut String) {
    // Phương thức .push_str() ghi thêm ký tự vào bãi đỗ Heap của biến gốc
    text.push_str(" - Chúc bạn một ngày tràn đầy năng lượng!");
}

// 3. Hàm minh họa toán tử giải tham chiếu (Dereferencing '*') với số nguyên
fn double(n: &mut i32) {
    // Dấu * dùng để đi theo địa chỉ con trỏ và can thiệp thẳng vào giá trị thực bên trong ô nhớ
    // (viết gọn của `*n = *n * 2;`)
    *n *= 2;
}

fn main() {
    println!("============================================================");
    println!("     CHƯƠNG TRÌNH LÀM CHỦ VAY MƯỢN & THAM CHIẾU TRONG RUST  ");
    println!("============================================================");

    // --- PHẦN 1: THAM CHIẾU BẤT BIẾN (&T - MƯỢN ĐỂ ĐỌC) ---
    println!("\n1. Minh họa mượn dữ liệu chỉ để đọc:");
    let bike_info = String::from("Xe máy Honda SH 150i");

    // Truyền &bike_info: Ta chỉ đưa "tấm ảnh chụp" địa chỉ ô nhớ cho hàm mượn
    let length = string_length(&bike_info);

    // Biến bike_info vẫn còn nguyên quyền sở hữu thuộc về hàm main!
    println!("- Xe máy: '{}'", bike_info);
    println!("- Độ dài chuỗi thông tin (tính bằng byte): {}", length);

    // Nhiều người có thể cùng mượn đọc đồng thời một lúc:
    let reader_1 = &bike_info;
    let reader_2 = &bike_info;
    println!("- Độc giả 1 đọc: {}", reader_1);
    println!("- Độc giả 2 đọc: {}", reader_2);

    // --- PHẦN 2: THAM CHIẾU KHẢ BIẾN (&mut T - MƯỢN ĐỂ SỬA) ---
    println!("\n2. Minh họa mượn dữ liệu để sửa đổi trực tiếp:");
    let mut letter = String::from("Xin chào bạn thân mến");
    println!("- Bức thư ban đầu: '{}'", letter);

    // Mượn để chỉnh sửa nội dung thông qua &mut
    add_greeting(&mut letter);
    println!("- Bức thư sau khi sửa: '{}'", letter);

    // --- PHẦN 3: GIẢI THAM CHIẾU VỚI TOÁN TỬ '*' TRÊN SỐ NGUYÊN ---
    println!("\n3. Thao tác ô nhớ số nguyên với toán tử giải tham chiếu (*):");
    let mut coins = 500;
    println!("- Số xu trước khi nhân đôi: {}", coins);

    double(&mut coins);
    println!("- Số xu sau khi nhân đôi  : {}", coins);

    // --- PHẦN 4: LÁT CẮT CHUỖI (STRING SLICES - &str) ---
    println!("\n4. Trích xuất văn bản bằng Lát cắt chuỗi (String Slices):");
    let sentence = String::from("Rust an toàn tuyệt đối");

    // Lát cắt trỏ vào một phần ô nhớ của chuỗi mà không tạo dữ liệu mới:
    let first_word: &str = &sentence[0..4]; // Cắt từ chỉ số byte 0 đến trước 4 ("Rust")
    let second_word: &str = &sentence[5..13]; // Cắt từ chỉ số byte 5 đến trước 13 ("an toàn")

    println!("- Câu nói gốc: '{}'", sentence);
    println!(
        "- Từ thứ nhất : '{}' (bản thân lát cắt &str chiếm {} bytes trên Stack)",
        first_word,
        std::mem::size_of::<&str>()
    );
    println!("- Từ thứ hai  : '{}'", second_word);

    // --- PHẦN 5: CHỨNG MINH TÍNH LINH HOẠT CỦA NLL (NON-LEXICAL LIFETIMES) ---
    println!("\n5. Kiểm tra cơ chế Vòng đời không từ vựng (NLL):");
    let mut log = String::from("Nhật ký ngày 01");

    let read_log = &log; // Bắt đầu mượn đọc
    println!("- Đọc nhật ký: {}", read_log);
    // Sau dòng print trên, read_log không còn được dùng nữa -> Hết hiệu lực mượn!

    let fix_log = &mut log; // Được phép mượn sửa ngay lập tức mà không xung đột!
    fix_log.push_str(" - Đã ghi thêm sự kiện mới");
    println!("- Nội dung sau cập nhật: {}", fix_log);
}
