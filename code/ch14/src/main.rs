// Tệp: src/main.rs
// Chương trình thực chiến: Ghép hàm, Curry hóa và Áp dụng từng phần trong Rust

use std::collections::HashMap;

// ============================================================================
// PHẦN 1: BỘ CÔNG CỤ GHÉP HÀM (COMPOSITION TOOLKIT)
// ============================================================================

/// Ghép 2 hàm: (A -> B) và (B -> C) thành (A -> C).
/// Đây chính là phép toán `g ∘ f` viết bằng cú pháp Rust.
pub fn compose<A, B, C>(f: impl Fn(A) -> B, g: impl Fn(B) -> C) -> impl Fn(A) -> C {
    move |x| g(f(x))
}

/// Ghép 3 hàm liên tiếp cho tiện dùng.
pub fn compose3<A, B, C, D>(
    f: impl Fn(A) -> B,
    g: impl Fn(B) -> C,
    h: impl Fn(C) -> D,
) -> impl Fn(A) -> D {
    move |x| h(g(f(x)))
}

/// Bộ kết hợp `identity`: phần tử đơn vị của phép ghép hàm.
pub fn identity<T>(x: T) -> T {
    x
}

/// Bộ kết hợp `const`: nuốt tham số, luôn trả về giá trị đã khóa sẵn.
pub fn constant<A: Clone, B>(value: A) -> impl Fn(B) -> A {
    move |_ignored| value.clone()
}

/// Bộ kết hợp `flip`: đảo thứ tự hai tham số của một hàm.
pub fn flip_args<A, B, C>(f: impl Fn(A, B) -> C) -> impl Fn(B, A) -> C {
    move |b, a| f(a, b)
}

// ============================================================================
// PHẦN 2: CÁC HÀM NHỎ THUẦN TÚY — TỪNG "ĐOẠN ỐNG" RIÊNG LẺ
// ============================================================================

/// Cắt bỏ khoảng trắng thừa ở hai đầu.
pub fn trim_whitespace(s: &str) -> String {
    s.trim().to_string()
}

/// Thu gọn nhiều khoảng trắng liên tiếp thành một khoảng trắng duy nhất.
pub fn collapse_whitespace(s: String) -> String {
    s.split_whitespace().collect::<Vec<&str>>().join(" ")
}

/// Viết hoa chữ cái đầu tiên của câu (an toàn với tiếng Việt có dấu).
pub fn capitalize_first(s: String) -> String {
    let mut chars = s.chars();
    match chars.next() {
        None => String::new(),
        Some(first) => first.to_uppercase().collect::<String>() + chars.as_str(),
    }
}

// ============================================================================
// PHẦN 3: CURRY HÓA & ÁP DỤNG TỪNG PHẦN — CÁC "NHÀ MÁY" SINH HÀM
// ============================================================================

/// Dạng thông thường: nhận đủ 2 tham số cùng lúc.
pub fn truncate(limit: usize, s: &str) -> String {
    if s.chars().count() <= limit {
        s.to_string()
    } else {
        let header: String = s.chars().take(limit).collect();
        format!("{}…", header)
    }
}

/// Dạng đã curry hóa: khóa trước `limit`, sinh ra một hàm chuyên dụng.
pub fn truncate_curried(limit: usize) -> impl Fn(&str) -> String {
    move |s: &str| truncate(limit, s)
}

/// Nhà máy sinh bộ lọc từ cấm: khóa sẵn danh sách từ, trả về một vị từ (predicate).
pub fn make_ban_filter(banned_words: Vec<String>) -> impl Fn(&str) -> bool {
    move |text: &str| {
        let lowercase = text.to_lowercase();
        !banned_words
            .iter()
            .any(|word| lowercase.contains(word.as_str()))
    }
}

/// Hạ một ký tự về chữ thường (với tiếng Việt, mỗi chữ hoa ứng đúng một chữ thường).
fn lower_char(c: char) -> char {
    c.to_lowercase().next().unwrap_or(c)
}

/// Nhà máy sinh bộ che từ cấm bằng dấu sao.
/// Không phân biệt hoa/thường — khớp với `make_ban_filter`, để "SPAM" cũng bị che.
pub fn make_censor(banned_words: Vec<String>) -> impl Fn(String) -> String {
    // Chuẩn bị MỘT LẦN lúc tạo bộ che: mỗi từ cấm thành dãy ký tự chữ thường.
    let banned: Vec<Vec<char>> = banned_words
        .iter()
        .map(|word| word.chars().map(lower_char).collect())
        .filter(|word: &Vec<char>| !word.is_empty())
        .collect();
    move |text: String| {
        let original: Vec<char> = text.chars().collect();
        // So khớp trên bản chữ thường, nhưng che trên bản gốc (cùng chỉ số ký tự).
        let lower: Vec<char> = original.iter().copied().map(lower_char).collect();
        let mut masked = original;
        for word in &banned {
            for start in 0..lower.len() {
                if lower[start..].starts_with(word) {
                    masked[start..start + word.len()].fill('*');
                }
            }
        }
        masked.into_iter().collect()
    }
}

// ============================================================================
// PHẦN 4: TIÊM PHỤ THUỘC BẰNG ÁP DỤNG TỪNG PHẦN
// ============================================================================

/// Bản ghi nhật ký kiểm duyệt (thay cho việc ghi ra tệp thật).
#[derive(Debug, Clone, PartialEq)]
pub struct LogRecord {
    pub comment_id: u32,
    pub verdict: String,
}

/// "Phụ thuộc" ở đây là hàm ghi nhật ký. Ta KHÓA nó vào trong bộ kiểm duyệt
/// bằng áp dụng từng phần, thay vì để bộ kiểm duyệt tự đi tìm.
/// `log` phải là `FnMut` vì nó ghi thêm vào sổ sau mỗi lần gọi.
pub fn make_validator<L>(
    check_clean: impl Fn(&str) -> bool,
    sanitize: impl Fn(String) -> String,
    mut log: L,
) -> impl FnMut(u32, &str) -> String
where
    L: FnMut(LogRecord),
{
    move |id: u32, raw: &str| {
        let trimmed = trim_whitespace(raw);
        // Kiểm tra TRƯỚC khi che — nếu che trước thì từ cấm biến mất
        // và bộ kiểm tra sẽ luôn báo "hợp lệ". Thứ tự các bước rất quan trọng!
        let verdict = if check_clean(&trimmed) {
            "HỢP LỆ"
        } else {
            "CHỨA TỪ CẤM — ĐÃ CHE"
        };
        let cleaned = sanitize(trimmed);
        log(LogRecord {
            comment_id: id,
            verdict: verdict.to_string(),
        });
        cleaned
    }
}

// ============================================================================
// CHƯƠNG TRÌNH ĐIỀU HÀNH CHÍNH
// ============================================================================

fn main() {
    println!("============================================================");
    println!("   GHÉP HÀM, CURRY HÓA & ÁP DỤNG TỪNG PHẦN TRONG RUST      ");
    println!("============================================================");

    // ------------------------------------------------------------------
    // 1. LẮP REN ỐNG NƯỚC: ghép 3 hàm nhỏ thành 1 đường ống chuẩn hóa
    // ------------------------------------------------------------------
    let normalize = compose3(trim_whitespace, collapse_whitespace, capitalize_first);

    let raw = "   xin    chào     các bạn  ";
    println!("\n1. GHÉP HÀM (Composition)");
    println!("   Đầu vào thô  : {:?}", raw);
    println!("   Sau đường ống: {:?}", normalize(raw));

    // ------------------------------------------------------------------
    // 2. KIỂM CHỨNG LUẬT KẾT HỢP: h ∘ (g ∘ f) == (h ∘ g) ∘ f
    // ------------------------------------------------------------------
    let way_a = compose(
        compose(trim_whitespace, collapse_whitespace),
        capitalize_first,
    );
    let way_b = compose(
        trim_whitespace,
        compose(collapse_whitespace, capitalize_first),
    );
    assert_eq!(way_a(raw), way_b(raw));
    println!("\n2. LUẬT KẾT HỢP");
    println!("   h∘(g∘f) và (h∘g)∘f cho cùng kết quả: {:?} ✓", way_a(raw));

    // ------------------------------------------------------------------
    // 3. LUẬT ĐƠN VỊ: ghép với `identity` không làm thay đổi gì
    // ------------------------------------------------------------------
    let with_identity = compose(identity::<&str>, &normalize);
    assert_eq!(with_identity(raw), normalize(raw));
    println!("\n3. LUẬT ĐƠN VỊ");
    println!("   identity ∘ f == f  ✓ (kết quả không đổi)");

    // ------------------------------------------------------------------
    // 4. CURRY HÓA: một hàm gốc sinh ra nhiều hàm chuyên dụng
    // ------------------------------------------------------------------
    println!("\n4. CURRY HÓA & ÁP DỤNG TỪNG PHẦN");
    let truncate_10 = truncate_curried(10); // Máy đã khóa núm "10 ký tự"
    let truncate_25 = truncate_curried(25); // Máy đã khóa núm "25 ký tự"

    let sentence = "Rust là ngôn ngữ lập trình hệ thống hiện đại";
    println!("   Bản gốc   : {}", sentence);
    println!("   Cắt còn 10: {}", truncate_10(sentence));
    println!("   Cắt còn 25: {}", truncate_25(sentence));

    // ------------------------------------------------------------------
    // 5. NHÀ MÁY SINH HÀM: cùng một danh sách từ cấm, hai công cụ khác nhau
    // ------------------------------------------------------------------
    let banned_words: Vec<String> = vec!["lừa đảo".to_string(), "spam".to_string()];
    let is_clean = make_ban_filter(banned_words.clone());
    let censor = make_censor(banned_words.clone());

    println!("\n5. NHÀ MÁY SINH HÀM (Closure Factory)");
    let dirty_comment = "Đây là tin spam lừa đảo";
    println!(
        "   {:?} có sạch không? {}",
        dirty_comment,
        is_clean(dirty_comment)
    );
    println!("   Sau khi che: {}", censor(dirty_comment.to_string()));

    // ------------------------------------------------------------------
    // 6. TIÊM PHỤ THUỘC: khóa "bộ ghi nhật ký" vào bộ kiểm duyệt
    // ------------------------------------------------------------------
    println!("\n6. TIÊM PHỤ THUỘC BẰNG ÁP DỤNG TỪNG PHẦN");
    let mut log_book: Vec<LogRecord> = Vec::new();

    {
        // Phụ thuộc thật: ghi vào sổ nhật ký trong bộ nhớ.
        let write_to_log = |record: LogRecord| log_book.push(record);
        let mut validator = make_validator(&is_clean, &censor, write_to_log);

        println!("   #101 -> {}", validator(101, "  Bài viết rất hay!  "));
        println!(
            "   #102 -> {}",
            validator(102, "  Cẩn thận kẻo bị lừa đảo  ")
        );
    }

    println!("   Nhật ký thu được ({} dòng):", log_book.len());
    for record in &log_book {
        println!(
            "     - Bình luận #{}: {}",
            record.comment_id, record.verdict
        );
    }

    // ------------------------------------------------------------------
    // 7. BỘ KẾT HỢP `flip` VÀ `const`
    // ------------------------------------------------------------------
    println!("\n7. BỘ KẾT HỢP flip & const");
    let divide = |a: f64, b: f64| a / b;
    let divide_flipped = flip_args(divide);
    println!("   divide(10, 2)       = {}", divide(10.0, 2.0));
    println!("   flip(divide)(10, 2) = {}", divide_flipped(10.0, 2.0)); // = divide(2, 10)

    let always_zero = constant::<i32, &str>(0);
    println!("   const(0)(\"bất kỳ\") = {}", always_zero("bất kỳ"));

    // ------------------------------------------------------------------
    // 8. `identity` GIÚP LỌC BỎ None — ỨNG DỤNG THỰC TẾ
    // ------------------------------------------------------------------
    let raw_data: Vec<Option<i32>> = vec![Some(1), None, Some(3), None, Some(5)];
    let clean: Vec<i32> = raw_data.into_iter().flat_map(identity).collect();
    println!("\n8. identity LỌC BỎ None: {:?}", clean);
    assert_eq!(clean, vec![1, 3, 5]);

    // ------------------------------------------------------------------
    // 9. GHÉP HÀM QUY MÔ LỚN: xử lý cả một danh sách bình luận
    // ------------------------------------------------------------------
    println!("\n9. ÁP DỤNG ĐƯỜNG ỐNG LÊN TOÀN BỘ DỮ LIỆU");
    let raw_comments = [
        "   rust rất   thú vị  ",
        " cẩn thận trò spam này ",
        "   giáo trình  hay quá   ",
    ];

    let stats: HashMap<bool, usize> =
        raw_comments
            .iter()
            .map(|c| normalize(c))
            .fold(HashMap::new(), |mut table, sentence| {
                *table.entry(is_clean(&sentence)).or_insert(0) += 1;
                table
            });

    for c in raw_comments.iter() {
        println!("   {:?} -> {:?}", c, normalize(c));
    }
    println!("   Thống kê [sạch = true/false]: {:?}", stats);

    println!("\n============================================================");
    println!("      HOÀN TẤT: TỪ HÀM NHỎ LẮP THÀNH HỆ THỐNG LỚN          ");
    println!("============================================================");
}

// ============================================================================
// KIỂM THỬ: BIẾN "LUẬT" THÀNH BÀI TEST CHẠY ĐƯỢC
// ============================================================================

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn composition_is_associative() {
        let samples = ["  a   b ", "Xin   chào", "   rust  "];
        for s in samples {
            let a = compose(
                compose(trim_whitespace, collapse_whitespace),
                capitalize_first,
            );
            let b = compose(
                trim_whitespace,
                compose(collapse_whitespace, capitalize_first),
            );
            assert_eq!(a(s), b(s), "Luật kết hợp bị vi phạm với đầu vào {:?}", s);
        }
    }

    #[test]
    fn composition_has_identity() {
        let f = compose(trim_whitespace, capitalize_first);
        let left = compose(identity::<&str>, &f);
        for s in ["  xin chào ", "rust"] {
            assert_eq!(left(s), f(s));
        }
    }

    #[test]
    fn curried_matches_original() {
        let truncate_15 = truncate_curried(15);
        let sentence = "Rust là ngôn ngữ tuyệt vời";
        assert_eq!(truncate_15(sentence), truncate(15, sentence));
    }

    #[test]
    fn flip_swaps_argument_order() {
        let subtract = |a: i32, b: i32| a - b;
        let flipped_subtract = flip_args(subtract);
        assert_eq!(subtract(10, 3), 7);
        assert_eq!(flipped_subtract(10, 3), -7); // = subtract(3, 10)
    }

    #[test]
    fn generated_closures_are_independent() {
        let filter = make_ban_filter(vec!["spam".to_string()]);
        assert!(filter("bài viết hay"));
        assert!(!filter("đây là SPAM"));
    }

    #[test]
    fn censor_is_case_insensitive_like_the_filter() {
        // Trước đây bộ lọc không phân biệt hoa/thường nhưng bộ che thì có,
        // nên "SPAM" bị báo là từ cấm mà vẫn không bị che.
        let censor = make_censor(vec!["spam".to_string(), "lừa đảo".to_string()]);
        assert_eq!(censor("đây là SPAM".to_string()), "đây là ****");
        assert_eq!(censor("Lừa Đảo nè".to_string()), "******* nè");
        assert_eq!(censor("bài viết hay".to_string()), "bài viết hay");
    }
}
