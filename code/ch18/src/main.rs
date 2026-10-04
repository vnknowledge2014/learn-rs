// Tệp: src/main.rs
// Chương trình thực chiến: Nửa nhóm, Vị nhóm và Kiểm thử theo tính chất

use std::cmp::Ordering;
use std::fmt::Debug;

// ============================================================================
// PHẦN 1: HAI TRAIT NỀN TẢNG
// ============================================================================

/// Nửa nhóm (Semigroup): có phép gộp hai thành một, tuân LUẬT KẾT HỢP.
pub trait Semigroup {
    fn compose(self, other: Self) -> Self;
}

/// Vị nhóm (Monoid): nửa nhóm có thêm PHẦN TỬ ĐƠN VỊ.
pub trait Monoid: Semigroup + Sized {
    fn empty() -> Self;
}

/// Hàm gộp vạn năng: dùng được cho MỌI vị nhóm.
/// Nó thay thế cho sum_all, concat_all, merge_lists, find_max... tất cả.
pub fn combine_all<M: Monoid>(list: impl IntoIterator<Item = M>) -> M {
    list.into_iter().fold(M::empty(), |acc, x| acc.compose(x))
}

// ============================================================================
// PHẦN 2: CÁC KIỂU CÓ SẴN CŨNG LÀ VỊ NHÓM
// ============================================================================

impl Semigroup for String {
    fn compose(self, other: Self) -> Self {
        self + &other // tái sử dụng bộ đệm của chuỗi thứ nhất
    }
}
impl Monoid for String {
    fn empty() -> Self {
        String::new()
    }
}

impl<T> Semigroup for Vec<T> {
    fn compose(mut self, mut other: Self) -> Self {
        self.append(&mut other);
        self
    }
}
impl<T> Monoid for Vec<T> {
    fn empty() -> Self {
        Vec::new()
    }
}

// ============================================================================
// PHẦN 3: KIỂU BỌC (NEWTYPE) — VÌ SỐ NGUYÊN CÓ NHIỀU VỊ NHÓM
// ============================================================================

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct Sum(pub i64);
impl Semigroup for Sum {
    fn compose(self, other: Self) -> Self {
        Sum(self.0 + other.0)
    }
}
impl Monoid for Sum {
    fn empty() -> Self {
        Sum(0)
    }
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct Product(pub i64);
impl Semigroup for Product {
    fn compose(self, other: Self) -> Self {
        Product(self.0.wrapping_mul(other.0))
    }
}
impl Monoid for Product {
    fn empty() -> Self {
        Product(1) // Chú ý: đơn vị của phép nhân là 1, KHÔNG phải 0!
    }
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct Max(pub i64);
impl Semigroup for Max {
    fn compose(self, other: Self) -> Self {
        Max(self.0.max(other.0))
    }
}
impl Monoid for Max {
    fn empty() -> Self {
        Max(i64::MIN) // "âm vô cực": gộp với gì cũng thua
    }
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct Min(pub i64);
impl Semigroup for Min {
    fn compose(self, other: Self) -> Self {
        Min(self.0.min(other.0))
    }
}
impl Monoid for Min {
    fn empty() -> Self {
        Min(i64::MAX)
    }
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct All(pub bool); // "tất cả đều đúng" — tương ứng .all()
impl Semigroup for All {
    fn compose(self, other: Self) -> Self {
        All(self.0 && other.0)
    }
}
impl Monoid for All {
    fn empty() -> Self {
        All(true)
    }
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct Any(pub bool); // "có ít nhất một cái đúng" — tương ứng .any()
impl Semigroup for Any {
    fn compose(self, other: Self) -> Self {
        Any(self.0 || other.0)
    }
}
impl Monoid for Any {
    fn empty() -> Self {
        Any(false)
    }
}

/// Vị nhóm "lấy cái đầu tiên có giá trị" — chính là ý tưởng của `Option::or`.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct First<T>(pub Option<T>);
impl<T> Semigroup for First<T> {
    fn compose(self, other: Self) -> Self {
        if self.0.is_some() { self } else { other }
    }
}
impl<T> Monoid for First<T> {
    fn empty() -> Self {
        First(None)
    }
}

// ============================================================================
// PHẦN 4: VỊ NHÓM TÍCH — GHÉP NHIỀU VỊ NHÓM THÀNH MỘT
// ============================================================================
// Mấu chốt: nếu A và B đều là vị nhóm thì cặp (A, B) cũng là vị nhóm.
// Nhờ vậy ta tính được NHIỀU chỉ số chỉ trong MỘT lượt duyệt dữ liệu.

impl<A: Semigroup, B: Semigroup> Semigroup for (A, B) {
    fn compose(self, other: Self) -> Self {
        (self.0.compose(other.0), self.1.compose(other.1))
    }
}
impl<A: Monoid, B: Monoid> Monoid for (A, B) {
    fn empty() -> Self {
        (A::empty(), B::empty())
    }
}

impl<A: Semigroup, B: Semigroup, C: Semigroup, D: Semigroup> Semigroup for (A, B, C, D) {
    fn compose(self, other: Self) -> Self {
        (
            self.0.compose(other.0),
            self.1.compose(other.1),
            self.2.compose(other.2),
            self.3.compose(other.3),
        )
    }
}
impl<A: Monoid, B: Monoid, C: Monoid, D: Monoid> Monoid for (A, B, C, D) {
    fn empty() -> Self {
        (A::empty(), B::empty(), C::empty(), D::empty())
    }
}

// ============================================================================
// PHẦN 5: ỨNG DỤNG THẬT — THỐNG KÊ NHẬT KÝ MÁY CHỦ
// ============================================================================

#[derive(Debug, Clone, PartialEq)]
pub struct AccessRecord {
    pub path: String,
    pub status_code: u16,
    pub time_ms: i64,
}

/// Bốn chỉ số cần tính, gói trong một vị nhóm tích 4 thành phần.
pub type Stats = (Sum, Max, Min, Any);

/// Biến một bản ghi thành "đóng góp" của nó vào thống kê tổng.
pub fn to_stats(record: &AccessRecord) -> Stats {
    (
        Sum(record.time_ms),
        Max(record.time_ms),
        Min(record.time_ms),
        Any(record.status_code >= 500),
    )
}

// ============================================================================
// PHẦN 6: BỘ SINH SỐ GIẢ NGẪU NHIÊN CHO KIỂM THỬ THEO TÍNH CHẤT
// ============================================================================

/// Bộ rng đồng dư tuyến tính (LCG) — tất định nên kiểm thử luôn lặp lại được.
pub struct Generator(u64);
impl Generator {
    pub fn new(seed: u64) -> Self {
        Generator(seed)
    }
    pub fn next_number(&mut self) -> i64 {
        // Hằng số nhân/cộng 64-bit của Knuth (MMIX)
        self.0 = self
            .0
            .wrapping_mul(6364136223846793005)
            .wrapping_add(1442695040888963407);
        ((self.0 >> 33) as i64) % 1000 - 500 // dải [-500, 499]
    }
}

/// Kiểm chứng LUẬT KẾT HỢP trên nhiều mẫu giả ngẫu nhiên.
pub fn verify_associativity<M, F>(name: &str, make: F, samples: usize) -> bool
where
    M: Semigroup + Clone + PartialEq + Debug,
    F: Fn(i64) -> M,
{
    let mut rng = Generator::new(2026);
    for _ in 0..samples {
        let a = make(rng.next_number());
        let b = make(rng.next_number());
        let c = make(rng.next_number());
        let left = a.clone().compose(b.clone()).compose(c.clone());
        let right = a.clone().compose(b.clone().compose(c.clone()));
        if left != right {
            println!(
                "  ✗ {} VI PHẠM luật kết hợp: {:?} vs {:?}",
                name, left, right
            );
            return false;
        }
    }
    println!("  ✓ {}: luật kết hợp đúng trên {} bộ mẫu", name, samples);
    true
}

/// Kiểm chứng LUẬT ĐƠN VỊ trên nhiều mẫu giả ngẫu nhiên.
pub fn verify_identity<M, F>(name: &str, make: F, samples: usize) -> bool
where
    M: Monoid + Clone + PartialEq + Debug,
    F: Fn(i64) -> M,
{
    let mut rng = Generator::new(777);
    for _ in 0..samples {
        let a = make(rng.next_number());
        if M::empty().compose(a.clone()) != a || a.clone().compose(M::empty()) != a {
            println!("  ✗ {} VI PHẠM luật đơn vị với {:?}", name, a);
            return false;
        }
    }
    println!("  ✓ {}: luật đơn vị đúng trên {} bộ mẫu", name, samples);
    true
}

// ============================================================================
// CHƯƠNG TRÌNH ĐIỀU HÀNH CHÍNH
// ============================================================================

fn main() {
    println!("============================================================");
    println!("    CẤU TRÚC ĐẠI SỐ: NỬA NHÓM, VỊ NHÓM VÀ LUẬT             ");
    println!("============================================================");

    // ------------------------------------------------------------------
    // 1. MỘT HÀM GỘP DUY NHẤT DÙNG CHO MỌI KIỂU
    // ------------------------------------------------------------------
    println!("\n1. HÀM `combine_all` VẠN NĂNG");
    let numbers = vec![Sum(3), Sum(8), Sum(-2), Sum(11)];
    println!("   Tổng các số       : {:?}", combine_all(numbers));

    let products = vec![Product(2), Product(3), Product(7)];
    println!("   Tích các số       : {:?}", combine_all(products));

    let strings = vec![
        String::from("Rust "),
        String::from("thật "),
        String::from("tuyệt!"),
    ];
    println!("   Nối chuỗi         : {:?}", combine_all(strings));

    let lists = vec![vec![1, 2], vec![3], vec![4, 5, 6]];
    println!("   Gộp danh sách     : {:?}", combine_all(lists));

    let set = vec![All(true), All(true), All(false)];
    println!("   Tất cả đều đạt?   : {:?}", combine_all(set));

    let config_sources: Vec<First<&str>> = vec![
        First(None),                // biến môi trường: không có
        First(Some("config.toml")), // tệp cấu hình: có!
        First(Some("default")),     // giá trị mặc định (không dùng tới)
    ];
    println!("   Nguồn cấu hình đầu: {:?}", combine_all(config_sources));

    // ------------------------------------------------------------------
    // 2. DANH SÁCH RỖNG — GIÁ TRỊ CỦA "HỘP RỖNG"
    // ------------------------------------------------------------------
    println!("\n2. VÌ SAO CẦN PHẦN TỬ ĐƠN VỊ?");
    let empty_sum: Vec<Sum> = Vec::new();
    let empty_product: Vec<Product> = Vec::new();
    println!(
        "   Tổng của danh sách RỖNG: {:?}  (đúng: 0)",
        combine_all(empty_sum)
    );
    println!(
        "   Tích của danh sách RỖNG: {:?}  (đúng: 1, KHÔNG phải 0!)",
        combine_all(empty_product)
    );

    // ------------------------------------------------------------------
    // 3. VỊ NHÓM TÍCH: 4 CHỈ SỐ TRONG 1 LƯỢT DUYỆT
    // ------------------------------------------------------------------
    println!("\n3. VỊ NHÓM TÍCH — 4 CHỈ SỐ, 1 LƯỢT DUYỆT");
    let log = [
        AccessRecord {
            path: "/api/don-hang".into(),
            status_code: 200,
            time_ms: 42,
        },
        AccessRecord {
            path: "/api/thanh-toan".into(),
            status_code: 500,
            time_ms: 1350,
        },
        AccessRecord {
            path: "/api/san-pham".into(),
            status_code: 200,
            time_ms: 17,
        },
        AccessRecord {
            path: "/api/kho".into(),
            status_code: 404,
            time_ms: 8,
        },
        AccessRecord {
            path: "/api/don-hang".into(),
            status_code: 200,
            time_ms: 63,
        },
    ];

    let (total, slowest, fastest, has_server_error): Stats = combine_all(log.iter().map(to_stats));

    println!("   Số bản ghi          : {}", log.len());
    println!("   Tổng thời gian      : {} ms", total.0);
    println!("   Trung bình          : {} ms", total.0 / log.len() as i64);
    println!("   Chậm nhất           : {} ms", slowest.0);
    println!("   Nhanh nhất          : {} ms", fastest.0);
    println!("   Có lỗi máy chủ 5xx? : {}", has_server_error.0);

    // ------------------------------------------------------------------
    // 4. LUẬT KẾT HỢP CHO PHÉP CHIA NHỎ & SONG SONG HÓA
    // ------------------------------------------------------------------
    println!("\n4. CHIA NHỎ RỒI GHÉP LẠI CHO CÙNG KẾT QUẢ");
    let all: Stats = combine_all(log.iter().map(to_stats));
    let (first_half, second_half) = log.split_at(2);
    let part_1: Stats = combine_all(first_half.iter().map(to_stats));
    let part_2: Stats = combine_all(second_half.iter().map(to_stats));
    let merged = part_1.compose(part_2);
    assert_eq!(all, merged);
    println!("   Gộp 1 lượt     : {:?}", all);
    println!("   Chia 2 rồi ghép: {:?}", merged);
    println!("   → GIỐNG NHAU ✓ Đây chính là cơ sở để chạy song song trên nhiều nhân CPU.");

    // ------------------------------------------------------------------
    // 5. KIỂM CHỨNG LUẬT BẰNG KIỂM THỬ THEO TÍNH CHẤT
    // ------------------------------------------------------------------
    println!("\n5. KIỂM THỬ THEO TÍNH CHẤT (1.000 bộ mẫu mỗi luật)");
    verify_associativity("Sum    ", Sum, 1000);
    verify_associativity("Product", Product, 1000);
    verify_associativity("Max    ", Max, 1000);
    verify_associativity("String ", |n: i64| n.to_string(), 1000);
    verify_identity("Sum    ", Sum, 1000);
    verify_identity("Product", Product, 1000);
    verify_identity("Max    ", Max, 1000);

    // ------------------------------------------------------------------
    // 6. PHẢN VÍ DỤ: PHÉP TRỪ KHÔNG PHẢI NỬA NHÓM
    // ------------------------------------------------------------------
    println!("\n6. PHẢN VÍ DỤ — PHÉP TRỪ VI PHẠM LUẬT KẾT HỢP");
    let (a, b, c) = (10i64, 3i64, 2i64);
    println!("   (10 - 3) - 2 = {}", (a - b) - c);
    println!("   10 - (3 - 2) = {}", a - (b - c));
    println!("   → KHÁC NHAU! Nên KHÔNG BAO GIỜ được chia nhỏ phép trừ ra nhiều luồng.");

    // ------------------------------------------------------------------
    // 7. VỊ NHÓM CÓ SẴN TRONG THƯ VIỆN CHUẨN: Ordering::then
    // ------------------------------------------------------------------
    println!("\n7. VỊ NHÓM `Ordering` — SẮP XẾP THEO NHIỀU TIÊU CHÍ");
    let mut employees = vec![
        ("Kỹ thuật", 3u32, "An"),
        ("Kinh doanh", 5, "Bình"),
        ("Kỹ thuật", 5, "Cường"),
        ("Kỹ thuật", 5, "Anh"),
    ];
    employees.sort_by(|x, y| {
        x.0.cmp(y.0) // 1. phòng ban tăng dần
            .then(y.1.cmp(&x.1)) // ⊕ 2. thâm niên giảm dần
            .then(x.2.cmp(y.2)) // ⊕ 3. họ tên tăng dần
    });
    for emp in &employees {
        println!("   {:<12} {} năm  {}", emp.0, emp.1, emp.2);
    }
    println!("   (Ordering::Equal chính là \"hộp rỗng\": bằng nhau thì xét tiêu chí sau)");

    // ------------------------------------------------------------------
    // 8. LUẬT PHẢN XẠ VÀ CÂU CHUYỆN f64 / NaN
    // ------------------------------------------------------------------
    println!("\n8. LUẬT CÓ THẬT: f64 KHÔNG CÓ TRAIT `Eq`");
    let nan = f64::NAN;
    // Cố ý so sánh một giá trị với chính nó để minh hoạ luật phản xạ bị phá vỡ;
    // clippy (eq_op) sẽ cảnh báo vì thông thường đây là lỗi gõ nhầm.
    #[allow(clippy::eq_op)]
    let nan_eq_nan = nan == nan;
    println!("   f64::NAN == f64::NAN  ->  {}", nan_eq_nan);
    println!("   → Luật phản xạ (a == a) bị phá vỡ, nên Rust TỪ CHỐI cài `Eq` cho f64.");
    println!("   → Hệ quả: không thể dùng f64 làm khóa HashMap / phần tử HashSet.");
    let ordering: Ordering = 3i64.cmp(&5i64);
    println!(
        "   (Còn i64 thì có đủ Eq + Ord: 3.cmp(&5) = {:?})",
        ordering
    );

    println!("\n============================================================");
    println!("   MỘT TRỪU TƯỢNG = MỘT CÁI TÊN + NHỮNG LUẬT LUÔN ĐÚNG      ");
    println!("============================================================");
}

// ============================================================================
// KIỂM THỬ TỰ ĐỘNG: LUẬT TRỞ THÀNH TEST CHẠY ĐƯỢC BẰNG `cargo test`
// ============================================================================

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn sum_is_associative() {
        assert!(verify_associativity("Sum", Sum, 500));
    }

    #[test]
    fn sum_has_identity() {
        assert!(verify_identity("Sum", Sum, 500));
    }

    #[test]
    fn product_obeys_both_laws() {
        assert!(verify_associativity("Product", Product, 500));
        assert!(verify_identity("Product", Product, 500));
    }

    #[test]
    fn string_concat_is_associative() {
        assert!(verify_associativity("String", |n: i64| n.to_string(), 500));
    }

    #[test]
    fn empty_list_folds_to_identity() {
        let empty_sum: Vec<Sum> = Vec::new();
        let empty_product: Vec<Product> = Vec::new();
        let empty_max: Vec<Max> = Vec::new();
        assert_eq!(combine_all(empty_sum), Sum(0));
        assert_eq!(combine_all(empty_product), Product(1));
        assert_eq!(combine_all(empty_max), Max(i64::MIN));
    }

    #[test]
    fn product_monoid_aggregates_four_metrics() {
        let log = [
            AccessRecord {
                path: "/a".into(),
                status_code: 200,
                time_ms: 10,
            },
            AccessRecord {
                path: "/b".into(),
                status_code: 503,
                time_ms: 40,
            },
            AccessRecord {
                path: "/c".into(),
                status_code: 200,
                time_ms: 25,
            },
        ];
        let (total, max, min, error): Stats = combine_all(log.iter().map(to_stats));
        assert_eq!(total, Sum(75));
        assert_eq!(max, Max(40));
        assert_eq!(min, Min(10));
        assert_eq!(error, Any(true));
    }

    /// Đây là bài test QUAN TRỌNG NHẤT chương: nó chứng minh rằng
    /// chia nhỏ dữ liệu rồi ghép lại luôn cho cùng kết quả —
    /// tức là thuật toán này SONG SONG HÓA ĐƯỢC một cách an toàn.
    #[test]
    fn split_then_merge_gives_same_result() {
        let mut rng = Generator::new(12345);
        let data: Vec<Sum> = (0..100).map(|_| Sum(rng.next_number())).collect();

        let one_pass = combine_all(data.clone());
        for cut in [0usize, 1, 37, 50, 99, 100] {
            let (left, right) = data.split_at(cut);
            let compose = combine_all(left.to_vec()).compose(combine_all(right.to_vec()));
            assert_eq!(one_pass, compose, "Sai khi cắt tại vị trí {}", cut);
        }
    }

    #[test]
    fn subtraction_is_not_a_semigroup() {
        // Phản ví dụ: chứng minh phép trừ VI PHẠM luật kết hợp.
        assert_ne!((10i64 - 3) - 2, 10i64 - (3 - 2));
    }

    #[test]
    // Cố ý so sánh giá trị với chính nó (eq_op) — đó chính là luật phản xạ cần kiểm.
    #[allow(clippy::eq_op)]
    fn nan_breaks_reflexivity() {
        let nan = f64::NAN;
        assert!(!(nan == nan), "NaN phải KHÔNG bằng chính nó theo IEEE 754");
        // Còn số nguyên thì luôn thỏa luật phản xạ:
        for i in -5i64..5 {
            assert!(i == i);
        }
    }
}
