#![allow(dead_code, unused_variables, unused_imports)]
// Tệp: src/main.rs
// Chương trình thực chiến: Functor, Applicative, Monad và bản đồ sang Rust

// ============================================================================
// PHẦN 1: MÔ PHỎNG KIỂU BẬC CAO (HKT) THEO CÁCH CỦA fp-core.rs
// ============================================================================

/// `HKT<U>` trả lời câu hỏi: "cái hộp này đang chứa gì, và nếu đổi ruột
/// sang kiểu U thì nó trở thành kiểu gì?"
pub trait HKT<U> {
    type Current; // T trong Option<T>
    type DichDen; // Option<U>
}

impl<T, U> HKT<U> for Option<T> {
    type Current = T;
    type DichDen = Option<U>;
}
impl<T, U> HKT<U> for Vec<T> {
    type Current = T;
    type DichDen = Vec<U>;
}
impl<T, U, E> HKT<U> for Result<T, E> {
    type Current = T;
    type DichDen = Result<U, E>;
}

/// HÀM TỬ tổng quát: nhờ HKT, một trait duy nhất dùng chung cho Option, Result và Vec.
pub trait Functor<U>: HKT<U> {
    fn mapping<F>(self, f: F) -> Self::DichDen
    where
        F: FnMut(Self::Current) -> U;
}

impl<T, U> Functor<U> for Option<T> {
    fn mapping<F>(self, f: F) -> Option<U>
    where
        F: FnMut(T) -> U,
    {
        self.map(f)
    }
}
impl<T, U> Functor<U> for Vec<T> {
    fn mapping<F>(self, f: F) -> Vec<U>
    where
        F: FnMut(T) -> U,
    {
        self.into_iter().map(f).collect()
    }
}
impl<T, U, E> Functor<U> for Result<T, E> {
    fn mapping<F>(self, f: F) -> Result<U, E>
    where
        F: FnMut(T) -> U,
    {
        self.map(f)
    }
}

// ============================================================================
// PHẦN 2: KIỂU XÁC THỰC TÍCH LŨY LỖI (APPLICATIVE VALIDATION)
// ============================================================================

/// Khác `Result`: khi hỏng, `Auth` giữ lại TOÀN BỘ danh sách lỗi.
#[derive(Debug, Clone, PartialEq)]
pub enum Auth<T> {
    Set(T),
    Hong(Vec<String>),
}

impl<T> Auth<T> {
    /// FUNCTOR: sơn lại giá trị bên trong mà không đụng tới danh sách lỗi.
    pub fn mapping<U>(self, f: impl FnOnce(T) -> U) -> Auth<U> {
        match self {
            Auth::Set(x) => Auth::Set(f(x)),
            Auth::Hong(error) => Auth::Hong(error),
        }
    }

    /// Chuyển từ Result sang Auth để bắt đầu tích lũy lỗi.
    pub fn tu_ket_qua(kq: Result<T, String>) -> Self {
        match kq {
            Ok(x) => Auth::Set(x),
            Err(e) => Auth::Hong(vec![e]),
        }
    }

    pub fn is_set(&self) -> bool {
        matches!(self, Auth::Set(_))
    }
}

/// APPLICATIVE: gộp 2 kết quả ĐỘC LẬP. Nếu cả hai hỏng, giữ lại CẢ HAI lỗi.
pub fn ghep2<A, B>(a: Auth<A>, b: Auth<B>) -> Auth<(A, B)> {
    match (a, b) {
        (Auth::Set(x), Auth::Set(y)) => Auth::Set((x, y)),
        (Auth::Hong(mut e1), Auth::Hong(e2)) => {
            e1.extend(e2); // ← đây chính là chỗ LỖI ĐƯỢC TÍCH LŨY
            Auth::Hong(e1)
        }
        (Auth::Hong(e), _) => Auth::Hong(e),
        (_, Auth::Hong(e)) => Auth::Hong(e),
    }
}

/// Gộp 3 kết quả độc lập — xây trên `ghep2`, đúng tinh thần ghép hàm ở Chương 14.
pub fn ghep3<A, B, C>(a: Auth<A>, b: Auth<B>, c: Auth<C>) -> Auth<(A, B, C)> {
    ghep2(ghep2(a, b), c).mapping(|((x, y), z)| (x, y, z))
}

// ============================================================================
// PHẦN 3: MIỀN NGHIỆP VỤ — ĐƠN ĐĂNG KÝ
// ============================================================================

#[derive(Debug, Clone, PartialEq)]
pub struct DonTho {
    pub name: String,
    pub email: String,
    pub age: String,
}

#[derive(Debug, Clone, PartialEq)]
pub struct User {
    pub name: String,
    pub email: String,
    pub age: u32,
}

pub fn check_name(tho: &str) -> Result<String, String> {
    let s = tho.trim();
    if s.chars().count() < 4 {
        Err(format!("Tên {:?} quá ngắn (cần ít nhất 4 ký tự)", s))
    } else if s.chars().count() > 30 {
        Err("Tên quá dài (tối đa 30 ký tự)".to_string())
    } else {
        Ok(s.to_string())
    }
}

pub fn validate_email(tho: &str) -> Result<String, String> {
    let s = tho.trim().to_lowercase();
    if !s.contains('@') {
        Err(format!("Email {:?} thiếu ký tự @", s))
    } else if !s.contains('.') {
        Err(format!("Email {:?} thiếu tên miền hợp lệ", s))
    } else {
        Ok(s)
    }
}

pub fn check_age(tho: &str) -> Result<u32, String> {
    let s = tho.trim();
    match s.parse::<u32>() {
        Err(_) => Err(format!("Tuổi {:?} không phải số nguyên", s)),
        Ok(n) if !(16..=100).contains(&n) => {
            Err(format!("Tuổi {} nằm ngoài khoảng cho phép 16-100", n))
        }
        Ok(n) => Ok(n),
    }
}

// ---------------------------------------------------------------------------
// CHIẾN LƯỢC A — MONAD: toán tử `?` dừng ngay ở lỗi ĐẦU TIÊN
// ---------------------------------------------------------------------------
pub fn short_circuit_register(don: &DonTho) -> Result<User, String> {
    let name = check_name(&don.name)?;
    let email = validate_email(&don.email)?;
    let age = check_age(&don.age)?;
    Ok(User { name, email, age })
}

// ---------------------------------------------------------------------------
// CHIẾN LƯỢC B — APPLICATIVE: chạy cả ba, gom TẤT CẢ lỗi
// ---------------------------------------------------------------------------
pub fn accumulator_register(don: &DonTho) -> Auth<User> {
    let name = Auth::tu_ket_qua(check_name(&don.name));
    let email = Auth::tu_ket_qua(validate_email(&don.email));
    let age = Auth::tu_ket_qua(check_age(&don.age));

    ghep3(name, email, age).mapping(|(name, email, age)| User { name, email, age })
}

// ============================================================================
// PHẦN 4: HÀM PHỤ TRỢ CHO PHẦN MONAD TUẦN TỰ
// ============================================================================

pub fn doc_ma_don(s: &str) -> Option<u32> {
    s.strip_prefix("ORD-")?.parse::<u32>().ok()
}

pub fn return_price(id: u32) -> Option<u64> {
    match id {
        8891 => Some(250_000),
        8892 => Some(1_200_000),
        _ => None,
    }
}

pub fn ap_thue(price: u64) -> Option<u64> {
    price.checked_mul(110)?.checked_div(100)
}

// ============================================================================
// CHƯƠNG TRÌNH ĐIỀU HÀNH CHÍNH
// ============================================================================

fn main() {
    println!("============================================================");
    println!("     HÀM TỬ, HÀM TỬ ÁP DỤNG VÀ ĐƠN NGUYÊN TRONG RUST       ");
    println!("============================================================");

    // ------------------------------------------------------------------
    // 1. FUNCTOR: cùng một `map` cho ba chiếc hộp khác nhau
    // ------------------------------------------------------------------
    println!("\n1. HÀM TỬ (Functor) — MỘT `map`, BA CHIẾC HỘP");
    let hop_option: Option<i32> = Some(21);
    let hop_result: Result<i32, String> = Ok(21);
    let hop_vec: Vec<i32> = vec![1, 2, 3];

    println!("   Option : {:?} -> {:?}", hop_option, hop_option.map(|x| x * 2));
    println!("   Result : {:?} -> {:?}", hop_result.clone(), hop_result.map(|x| x * 2));
    println!(
        "   Vec    : {:?} -> {:?}",
        hop_vec.clone(),
        hop_vec.iter().map(|x| x * 2).collect::<Vec<_>>()
    );

    let hop_rong: Option<i32> = None;
    println!("   Hộp rỗng vẫn rỗng: {:?} -> {:?}", hop_rong, hop_rong.map(|x| x * 2));

    // Dùng trait Functor tổng quát tự viết (mô phỏng HKT)
    println!("\n   Qua trait `HamTu` tổng quát (mô phỏng HKT):");
    println!("   Option: {:?}", Some(5i32).mapping(|x| x + 1));
    println!("   Vec   : {:?}", vec![1i32, 2, 3].mapping(|x| x * 10));
    let r: Result<i32, String> = Ok(7);
    println!("   Result: {:?}", r.mapping(|x| x - 7));

    // ------------------------------------------------------------------
    // 2. HAI LUẬT FUNCTOR
    // ------------------------------------------------------------------
    println!("\n2. HAI LUẬT FUNCTOR");
    let x = Some(10i32);
    assert_eq!(x.map(|a| a), x);
    println!("   (F1) x.map(identity) == x  ✓");

    let f = |a: i32| a + 3;
    let g = |a: i32| a * 2;
    assert_eq!(x.map(f).map(g), x.map(|a| g(f(a))));
    println!("   (F2) x.map(f).map(g) == x.map(g∘f)  ✓");
    println!("        → Đây là lý do trình biên dịch gộp được 2 vòng map thành 1!");

    // ------------------------------------------------------------------
    // 3. BIFUNCTOR: Result có hai chân
    // ------------------------------------------------------------------
    println!("\n3. BIFUNCTOR — `Result` CÓ HAI CHÂN");
    let into_sum: Result<i32, String> = Ok(5);
    let that_bai: Result<i32, String> = Err("mất kết nối".into());
    println!("   map     (chân Ok) : {:?}", into_sum.map(|v| v * 100));
    println!(
        "   map_err (chân Err): {:?}",
        that_bai.map_err(|e| format!("[HỆ THỐNG] {}", e))
    );

    // ------------------------------------------------------------------
    // 4. MONAD: `and_then` chính là `bind`
    // ------------------------------------------------------------------
    println!("\n4. ĐƠN NGUYÊN — `and_then` CHÍNH LÀ `bind`");
    for id in ["ORD-8891", "ORD-9999", "SAI-DINH-DANG"] {
        let ket_qua = doc_ma_don(id).and_then(return_price).and_then(ap_thue);
        println!("   {:>14} -> {:?}", id, ket_qua);
    }

    println!("\n   Đẳng thức định nghĩa: bind(x,f) == x.map(f).flatten()");
    let x = Some(4i32);
    let f = |n: i32| if n > 0 { Some(n * 10) } else { None };
    assert_eq!(x.and_then(f), x.map(f).flatten());
    println!("   {:?} == {:?}  ✓", x.and_then(f), x.map(f).flatten());

    // ------------------------------------------------------------------
    // 5. BA LUẬT MONAD
    // ------------------------------------------------------------------
    println!("\n5. BA LUẬT MONAD");
    let a = 5i32;
    let m = Some(a);
    let f = |n: i32| Some(n + 1);
    let g = |n: i32| if n % 2 == 0 { Some(n / 2) } else { None };

    assert_eq!(Some(a).and_then(f), f(a));
    println!("   (M1) Đơn vị trái : Some(a).and_then(f) == f(a)  ✓");
    assert_eq!(m.and_then(Some), m);
    println!("   (M2) Đơn vị phải : m.and_then(Some) == m  ✓");
    assert_eq!(m.and_then(f).and_then(g), m.and_then(|x| f(x).and_then(g)));
    println!("   (M3) Kết hợp     : (m>>=f)>>=g == m>>=(x -> f(x)>>=g)  ✓");

    // ------------------------------------------------------------------
    // 6. TRAVERSABLE: đảo ngữ cảnh Vec<Result> -> Result<Vec>
    // ------------------------------------------------------------------
    println!("\n6. TRAVERSABLE — CÔNG CỤ BỊ BỎ QUÊN NHẤT CỦA RUST");
    let tot = vec!["10", "20", "30"];
    let hong = vec!["10", "hai mươi", "30"];

    let result_good: Result<Vec<i32>, _> = tot.iter().map(|s| s.parse::<i32>()).collect();
    let result_hong: Result<Vec<i32>, _> = hong.iter().map(|s| s.parse::<i32>()).collect();
    println!("   Vec<Result> -> Result<Vec> (tốt) : {:?}", result_good);
    println!("   Vec<Result> -> Result<Vec> (hỏng): có lỗi = {:?}", result_hong.is_err());

    let has_empty: Option<Vec<i32>> = vec![Some(1), None, Some(3)].into_iter().collect();
    let no_empty: Option<Vec<i32>> = vec![Some(1), Some(2)].into_iter().collect();
    println!("   Vec<Option> -> Option<Vec> (có None): {:?}", has_empty);
    println!("   Vec<Option> -> Option<Vec> (đủ)     : {:?}", no_empty);

    let lat: Option<Result<i32, String>> = Some(Ok(9));
    println!("   Option<Result> --transpose--> Result<Option>: {:?}", lat.transpose());

    // ------------------------------------------------------------------
    // 7. ALTERNATIVE: chuỗi phương án dự phòng
    // ------------------------------------------------------------------
    println!("\n7. ALTERNATIVE — CHUỖI PHƯƠNG ÁN DỰ PHÒNG");
    let missing_field: Option<&str> = None;
    let from_config_file: Option<&str> = Some("8080");
    let gate = missing_field.or(from_config_file).unwrap_or("3000");
    println!("   Cổng dùng: {} (biến môi trường -> tệp cấu hình -> mặc định)", gate);

    // ------------------------------------------------------------------
    // 8. SO SÁNH TRỰC DIỆN: MONAD NGẮN MẠCH vs APPLICATIVE TÍCH LŨY
    // ------------------------------------------------------------------
    println!("\n8. NGẮN MẠCH (Monad) vs TÍCH LŨY LỖI (Applicative)");
    let don_hong = DonTho {
        name: "An".into(),             // quá ngắn
        email: "an-tai-gmail".into(), // thiếu @
        age: "mười tám".into(),      // không phải số
    };

    println!("\n   [A] Dùng toán tử `?` (Monad — dừng ở lỗi đầu tiên):");
    match short_circuit_register(&don_hong) {
        Ok(nd) => println!("       Thành công: {:?}", nd),
        Err(e) => println!("       Báo về 1 lỗi duy nhất: {}", e),
    }

    println!("\n   [B] Dùng `XacThuc` (Applicative — gom hết lỗi):");
    match accumulator_register(&don_hong) {
        Auth::Set(nd) => println!("       Thành công: {:?}", nd),
        Auth::Hong(error) => {
            println!("       Báo về {} lỗi cùng lúc:", error.len());
            for (i, l) in error.iter().enumerate() {
                println!("         {}. {}", i + 1, l);
            }
        }
    }

    println!("\n   [C] Đơn hợp lệ đi qua cả hai chiến lược:");
    let don_tot = DonTho {
        name: "Nguyễn Văn An".into(),
        email: "  An.Nguyen@Example.COM ".into(),
        age: " 28 ".into(),
    };
    println!("       Ngắn mạch: {:?}", short_circuit_register(&don_tot));
    println!("       Tích lũy : hợp lệ = {}", accumulator_register(&don_tot).is_set());

    println!("\n============================================================");
    println!("  map = SƠN TRONG HỘP · zip = GỘP HỘP · and_then = MỞ HỘP   ");
    println!("============================================================");
}

// ============================================================================
// KIỂM THỬ: BIẾN LUẬT FUNCTOR VÀ MONAD THÀNH TEST CHẠY ĐƯỢC
// ============================================================================

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn functor_identity_law() {
        for x in [Some(1i32), Some(-7), None] {
            assert_eq!(x.map(|a| a), x);
        }
    }

    #[test]
    fn functor_composition_law() {
        let f = |a: i32| a + 3;
        let g = |a: i32| a * 2;
        for x in [Some(0i32), Some(10), Some(-4), None] {
            assert_eq!(x.map(f).map(g), x.map(|a| g(f(a))));
        }
    }

    #[test]
    fn monad_left_and_right_identity() {
        let f = |n: i32| if n > 0 { Some(n * 2) } else { None };
        for a in [-3i32, 0, 5, 100] {
            assert_eq!(Some(a).and_then(f), f(a)); // M1
        }
        for m in [Some(1i32), None] {
            assert_eq!(m.and_then(Some), m); // M2
        }
    }

    #[test]
    fn monad_associativity() {
        let f = |n: i32| if n >= 0 { Some(n + 1) } else { None };
        let g = |n: i32| if n % 2 == 0 { Some(n / 2) } else { None };
        for m in [Some(-5i32), Some(0), Some(3), Some(8), None] {
            assert_eq!(
                m.and_then(f).and_then(g),
                m.and_then(|x| f(x).and_then(g)) // M3
            );
        }
    }

    #[test]
    fn bind_bang_map_roi_flatten() {
        let f = |n: i32| if n > 0 { Some(n * 10) } else { None };
        for x in [Some(4i32), Some(-1), None] {
            assert_eq!(x.and_then(f), x.map(f).flatten());
        }
    }

    #[test]
    fn traversable_swaps_contexts() {
        let tot: Result<Vec<i32>, _> = ["1", "2", "3"].iter().map(|s| s.parse::<i32>()).collect();
        assert_eq!(tot, Ok(vec![1, 2, 3]));

        let hong: Result<Vec<i32>, _> = ["1", "x", "3"].iter().map(|s| s.parse::<i32>()).collect();
        assert!(hong.is_err());

        let co_none: Option<Vec<i32>> = vec![Some(1), None].into_iter().collect();
        assert_eq!(co_none, None);

        let lat: Option<Result<i32, String>> = Some(Ok(9));
        assert_eq!(lat.transpose(), Ok(Some(9)));
    }

    #[test]
    fn applicative_collects_all_three_errors() {
        let don = DonTho {
            name: "An".into(),
            email: "khong-co-a-cong".into(),
            age: "abc".into(),
        };
        match accumulator_register(&don) {
            Auth::Hong(error) => {
                assert_eq!(error.len(), 3, "Phải gom đủ 3 lỗi, nhận được {:?}", error)
            }
            Auth::Set(_) => panic!("Đơn hỏng mà lại được chấp nhận!"),
        }
    }

    #[test]
    fn monad_reports_only_first_error() {
        let don = DonTho {
            name: "An".into(),
            email: "khong-co-a-cong".into(),
            age: "abc".into(),
        };
        // Toán tử `?` dừng ngay ở lỗi đầu tiên: chỉ nhận được 1 thông báo.
        let error = short_circuit_register(&don).unwrap_err();
        assert!(error.contains("quá ngắn"), "Phải là lỗi ĐẦU TIÊN, nhận: {}", error);
    }

    #[test]
    fn valid_order_passes_both_strategies() {
        let don = DonTho {
            name: "Nguyễn Văn An".into(),
            email: " An.Nguyen@Example.COM ".into(),
            age: " 28 ".into(),
        };
        let expected = User {
            name: "Nguyễn Văn An".to_string(),
            email: "an.nguyen@example.com".to_string(),
            age: 28,
        };
        assert_eq!(short_circuit_register(&don), Ok(expected.clone()));
        assert_eq!(accumulator_register(&don), Auth::Set(expected));
    }

    #[test]
    fn the_generic_functor_works_for_three_types() {
        assert_eq!(Some(5i32).mapping(|x| x + 1), Some(6));
        assert_eq!(vec![1i32, 2, 3].mapping(|x| x * 10), vec![10, 20, 30]);
        let r: Result<i32, String> = Ok(7);
        assert_eq!(r.mapping(|x| x - 7), Ok(0));
    }
}
