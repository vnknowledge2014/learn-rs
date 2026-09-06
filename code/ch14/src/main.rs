#![allow(dead_code, unused_variables, unused_imports)]
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
pub fn ghep3<A, B, C, D>(
    f: impl Fn(A) -> B,
    g: impl Fn(B) -> C,
    h: impl Fn(C) -> D,
) -> impl Fn(A) -> D {
    move |x| h(g(f(x)))
}

/// Bộ kết hợp `identity`: phần tử đơn vị của phép ghép hàm.
pub fn closest<T>(x: T) -> T {
    x
}

/// Bộ kết hợp `const`: nuốt tham số, luôn trả về giá trị đã khóa sẵn.
pub fn queue_num<A: Clone, B>(value: A) -> impl Fn(B) -> A {
    move |_bo_qua| value.clone()
}

/// Bộ kết hợp `flip`: đảo thứ tự hai tham số của một hàm.
pub fn flip_args<A, B, C>(f: impl Fn(A, B) -> C) -> impl Fn(B, A) -> C {
    move |b, a| f(a, b)
}

// ============================================================================
// PHẦN 2: CÁC HÀM NHỎ THUẦN TÚY — TỪNG "ĐOẠN ỐNG" RIÊNG LẺ
// ============================================================================

/// Cắt bỏ khoảng trắng thừa ở hai đầu.
pub fn cut_range_state(s: &str) -> String {
    s.trim().to_string()
}

/// Thu gọn nhiều khoảng trắng liên tiếp thành một khoảng trắng duy nhất.
pub fn reduce_range(s: String) -> String {
    s.split_whitespace().collect::<Vec<&str>>().join(" ")
}

/// Viết hoa chữ cái đầu tiên của câu (an toàn với tiếng Việt có dấu).
pub fn capitalize_first(s: String) -> String {
    let mut all_ky_from = s.chars();
    match all_ky_from.next() {
        None => String::new(),
        Some(first) => first.to_uppercase().collect::<String>() + all_ky_from.as_str(),
    }
}

// ============================================================================
// PHẦN 3: CURRY HÓA & ÁP DỤNG TỪNG PHẦN — CÁC "NHÀ MÁY" SINH HÀM
// ============================================================================

/// Dạng thông thường: nhận đủ 2 tham số cùng lúc.
pub fn cat_bot(gioi_han: usize, s: &str) -> String {
    if s.chars().count() <= gioi_han {
        s.to_string()
    } else {
        let header: String = s.chars().take(gioi_han).collect();
        format!("{}…", header)
    }
}

/// Dạng đã curry hóa: khóa trước `gioi_han`, sinh ra một hàm chuyên dụng.
pub fn cat_bot_curry(gioi_han: usize) -> impl Fn(&str) -> String {
    move |s: &str| cat_bot(gioi_han, s)
}

/// Nhà máy sinh bộ lọc từ cấm: khóa sẵn danh sách từ, trả về một vị từ (predicate).
pub fn make_ban_filter(tu_cam: Vec<String>) -> impl Fn(&str) -> bool {
    move |van_ban: &str| {
        let lowercase = van_ban.to_lowercase();
        !tu_cam.iter().any(|tu| lowercase.contains(tu.as_str()))
    }
}

/// Nhà máy sinh bộ che từ cấm bằng dấu sao.
pub fn tao_bo_che_tu_cam(tu_cam: Vec<String>) -> impl Fn(String) -> String {
    move |van_ban: String| {
        tu_cam.iter().fold(van_ban, |ket_qua, tu| {
            let che = "*".repeat(tu.chars().count());
            ket_qua.replace(tu.as_str(), che.as_str())
        })
    }
}

// ============================================================================
// PHẦN 4: TIÊM PHỤ THUỘC BẰNG ÁP DỤNG TỪNG PHẦN
// ============================================================================

/// Bản ghi nhật ký kiểm duyệt (thay cho việc ghi ra tệp thật).
#[derive(Debug, Clone, PartialEq)]
pub struct SellRecordLog {
    pub ma_binh_luan: u32,
    pub ket_luan: String,
}

/// "Phụ thuộc" ở đây là hàm ghi nhật ký. Ta KHÓA nó vào trong bộ kiểm duyệt
/// bằng áp dụng từng phần, thay vì để bộ kiểm duyệt tự đi tìm.
/// `ghi_nhat_ky` phải là `FnMut` vì nó ghi thêm vào sổ sau mỗi lần gọi.
pub fn make_validator<L>(
    check_clean: impl Fn(&str) -> bool,
    sanitize: impl Fn(String) -> String,
    mut ghi_nhat_ky: L,
) -> impl FnMut(u32, &str) -> String
where
    L: FnMut(SellRecordLog),
{
    move |id: u32, tho: &str| {
        let standard = cut_range_state(tho);
        // Kiểm tra TRƯỚC khi che — nếu che trước thì từ cấm biến mất
        // và bộ kiểm tra sẽ luôn báo "hợp lệ". Thứ tự các bước rất quan trọng!
        let ket_luan = if check_clean(&standard) {
            "HỢP LỆ"
        } else {
            "CHỨA TỪ CẤM — ĐÃ CHE"
        };
        let da_lam_sach = sanitize(standard);
        ghi_nhat_ky(SellRecordLog {
            ma_binh_luan: id,
            ket_luan: ket_luan.to_string(),
        });
        da_lam_sach
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
    let normalize = ghep3(cut_range_state, reduce_range, capitalize_first);

    let tho = "   xin    chào     các bạn  ";
    println!("\n1. GHÉP HÀM (Composition)");
    println!("   Đầu vào thô  : {:?}", tho);
    println!("   Sau đường ống: {:?}", normalize(tho));

    // ------------------------------------------------------------------
    // 2. KIỂM CHỨNG LUẬT KẾT HỢP: h ∘ (g ∘ f) == (h ∘ g) ∘ f
    // ------------------------------------------------------------------
    let cach_a = compose(compose(cut_range_state, reduce_range), capitalize_first);
    let cach_b = compose(cut_range_state, compose(reduce_range, capitalize_first));
    assert_eq!(cach_a(tho), cach_b(tho));
    println!("\n2. LUẬT KẾT HỢP");
    println!("   h∘(g∘f) và (h∘g)∘f cho cùng kết quả: {:?} ✓", cach_a(tho));

    // ------------------------------------------------------------------
    // 3. LUẬT ĐƠN VỊ: ghép với `identity` không làm thay đổi gì
    // ------------------------------------------------------------------
    let with_don_pos = compose(closest::<&str>, &normalize);
    assert_eq!(with_don_pos(tho), normalize(tho));
    println!("\n3. LUẬT ĐƠN VỊ");
    println!("   identity ∘ f == f  ✓ (kết quả không đổi)");

    // ------------------------------------------------------------------
    // 4. CURRY HÓA: một hàm gốc sinh ra nhiều hàm chuyên dụng
    // ------------------------------------------------------------------
    println!("\n4. CURRY HÓA & ÁP DỤNG TỪNG PHẦN");
    let truncate = cat_bot_curry(10); // Máy đã khóa núm "10 ký tự"
    let cut_long = cat_bot_curry(25);  // Máy đã khóa núm "25 ký tự"

    let cau = "Rust là ngôn ngữ lập trình hệ thống hiện đại";
    println!("   Bản gốc   : {}", cau);
    println!("   Cắt còn 10: {}", truncate(cau));
    println!("   Cắt còn 25: {}", cut_long(cau));

    // ------------------------------------------------------------------
    // 5. NHÀ MÁY SINH HÀM: cùng một danh sách từ cấm, hai công cụ khác nhau
    // ------------------------------------------------------------------
    let tu_cam: Vec<String> = vec!["lừa đảo".to_string(), "spam".to_string()];
    let is_clean = make_ban_filter(tu_cam.clone());
    let che_di = tao_bo_che_tu_cam(tu_cam.clone());

    println!("\n5. NHÀ MÁY SINH HÀM (Closure Factory)");
    let binh_luan_ban = "Đây là tin spam lừa đảo";
    println!("   {:?} có sạch không? {}", binh_luan_ban, is_clean(binh_luan_ban));
    println!("   Sau khi che: {}", che_di(binh_luan_ban.to_string()));

    // ------------------------------------------------------------------
    // 6. TIÊM PHỤ THUỘC: khóa "bộ ghi nhật ký" vào bộ kiểm duyệt
    // ------------------------------------------------------------------
    println!("\n6. TIÊM PHỤ THUỘC BẰNG ÁP DỤNG TỪNG PHẦN");
    let mut num_log: Vec<SellRecordLog> = Vec::new();

    {
        // Phụ thuộc thật: ghi vào sổ nhật ký trong bộ nhớ.
        let record_in_num = |sell_record: SellRecordLog| num_log.push(sell_record);
        let mut validator = make_validator(&is_clean, &che_di, record_in_num);

        println!("   #101 -> {}", validator(101, "  Bài viết rất hay!  "));
        println!("   #102 -> {}", validator(102, "  Cẩn thận kẻo bị lừa đảo  "));
    }

    println!("   Nhật ký thu được ({} dòng):", num_log.len());
    for sell_record in &num_log {
        println!("     - Bình luận #{}: {}", sell_record.ma_binh_luan, sell_record.ket_luan);
    }

    // ------------------------------------------------------------------
    // 7. BỘ KẾT HỢP `flip` VÀ `const`
    // ------------------------------------------------------------------
    println!("\n7. BỘ KẾT HỢP flip & const");
    let chia = |a: f64, b: f64| a / b;
    let chia_nguoc = flip_args(chia);
    println!("   chia(10, 2)       = {}", chia(10.0, 2.0));
    println!("   flip(chia)(10, 2) = {}", chia_nguoc(10.0, 2.0)); // = chia(2, 10)

    let always_return_ve_0 = queue_num::<i32, &str>(0);
    println!("   const(0)(\"bất kỳ\") = {}", always_return_ve_0("bất kỳ"));

    // ------------------------------------------------------------------
    // 8. `identity` GIÚP LỌC BỎ None — ỨNG DỤNG THỰC TẾ
    // ------------------------------------------------------------------
    let raw_data: Vec<Option<i32>> = vec![Some(1), None, Some(3), None, Some(5)];
    let clean: Vec<i32> = raw_data.into_iter().flat_map(closest).collect();
    println!("\n8. identity LỌC BỎ None: {:?}", clean);
    assert_eq!(clean, vec![1, 3, 5]);

    // ------------------------------------------------------------------
    // 9. GHÉP HÀM QUY MÔ LỚN: xử lý cả một danh sách bình luận
    // ------------------------------------------------------------------
    println!("\n9. ÁP DỤNG ĐƯỜNG ỐNG LÊN TOÀN BỘ DỮ LIỆU");
    let binh_luan_tho = vec![
        "   rust rất   thú vị  ",
        " cẩn thận trò spam này ",
        "   giáo trình  hay quá   ",
    ];

    let thong_ke: HashMap<bool, usize> = binh_luan_tho
        .iter()
        .map(|b| normalize(b))
        .fold(HashMap::new(), |mut bang, cau| {
            *bang.entry(is_clean(&cau)).or_insert(0) += 1;
            bang
        });

    for b in binh_luan_tho.iter() {
        println!("   {:?} -> {:?}", b, normalize(b));
    }
    println!("   Thống kê [sạch = true/false]: {:?}", thong_ke);

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
        let mau = ["  a   b ", "Xin   chào", "   rust  "];
        for s in mau {
            let a = compose(compose(cut_range_state, reduce_range), capitalize_first);
            let b = compose(cut_range_state, compose(reduce_range, capitalize_first));
            assert_eq!(a(s), b(s), "Luật kết hợp bị vi phạm với đầu vào {:?}", s);
        }
    }

    #[test]
    fn composition_has_identity() {
        let f = compose(cut_range_state, capitalize_first);
        let left = compose(closest::<&str>, &f);
        for s in ["  xin chào ", "rust"] {
            assert_eq!(left(s), f(s));
        }
    }

    #[test]
    fn curried_matches_original() {
        let cat_15 = cat_bot_curry(15);
        let cau = "Rust là ngôn ngữ tuyệt vời";
        assert_eq!(cat_15(cau), cat_bot(15, cau));
    }

    #[test]
    fn flip_swaps_argument_order() {
        let subtract = |a: i32, b: i32| a - b;
        let flipped_subtract = flip_args(subtract);
        assert_eq!(subtract(10, 3), 7);
        assert_eq!(flipped_subtract(10, 3), -7); // = tru(3, 10)
    }

    #[test]
    fn generated_closures_are_independent() {
        let filter = make_ban_filter(vec!["spam".to_string()]);
        assert!(filter("bài viết hay"));
        assert!(!filter("đây là SPAM"));
    }
}
