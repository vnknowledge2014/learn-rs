// File: src/main.rs
// Đây là chương trình Rust hoàn chỉnh đầu tiên của bạn!

fn main() {
    // println! là một công cụ (macro) in các dòng chữ ra màn hình terminal.
    // Dấu chấm than (!) biểu thị rằng đây là một Macro đặc biệt của Rust.
    println!("============================================================");
    println!("  CHƯƠNG TRÌNH KHÁM PHÁ BỘ NHỚ VẬT LÝ VÀ PHẦN CỨNG MÁY TÍNH  ");
    println!("============================================================");

    // 1. Khám phá kích thước của 1 Byte (gồm 8 bits công tắc)
    // std::mem::size_of::<T>() là hàm đo xem kiểu dữ liệu T chiếm bao nhiêu Byte trên RAM.
    let u8_size = std::mem::size_of::<u8>();
    println!(
        "- Kiểu u8 (số nguyên nhỏ 0..255) chiếm : {} byte ({} bits)",
        u8_size,
        u8_size * 8
    );

    // 2. Khám phá kiểu số nguyên tiêu chuẩn 32-bit (i32)
    let i32_size = std::mem::size_of::<i32>();
    println!(
        "- Kiểu i32 (số nguyên chuẩn) chiếm       : {} bytes ({} bits)",
        i32_size,
        i32_size * 8
    );

    // 3. Khám phá kiểu số nguyên cực lớn 64-bit (i64)
    let i64_size = std::mem::size_of::<i64>();
    println!(
        "- Kiểu i64 (số nguyên lớn) chiếm         : {} bytes ({} bits)",
        i64_size,
        i64_size * 8
    );

    // 4. Khám phá kiểu ký tự Unicode (char)
    // Trong Rust, một ký tự có thể là chữ cái tiếng Việt hoặc biểu tượng cảm xúc Emoji!
    let char_size = std::mem::size_of::<char>();
    println!(
        "- Kiểu char (ký tự Unicode/Emoji) chiếm  : {} bytes ({} bits)",
        char_size,
        char_size * 8
    );

    // 5. Khám phá kiểu logic Đúng/Sai (bool)
    let bool_size = std::mem::size_of::<bool>();
    println!(
        "- Kiểu bool (true/false) chiếm           : {} byte (dù chỉ cần 1 bit)",
        bool_size
    );

    println!("------------------------------------------------------------");

    // 6. Minh họa trực tiếp cách máy tính nhìn một con số dưới dạng công tắc bật/tắt (nhị phân)
    let favorite_number: u8 = 42;
    println!("Con số quen thuộc trong đời thực: {}", favorite_number);
    // Cú pháp {:08b} yêu cầu Rust in số này dưới dạng nhị phân 8 bit (0 và 1)
    println!(
        "Dãy 8 công tắc điện thực tế trong chip RAM: {:08b}",
        favorite_number
    );

    let mascot: char = '🦀'; // Cua Ferris - Linh vật chính thức của cộng đồng Rust
    println!("Linh vật đáng yêu của Rust: {}", mascot);
    println!(
        "Mã số đại diện trong bộ ký tự quốc tế: U+{:X}",
        mascot as u32
    );
}
