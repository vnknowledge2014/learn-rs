#![allow(dead_code, unused_variables, unused_imports)]
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
pub trait PosGroup: Semigroup + Sized {
    fn don_pos() -> Self;
}

/// Hàm gộp vạn năng: dùng được cho MỌI vị nhóm.
/// Nó thay thế cho tinh_tong, noi_chuoi, gop_mang, tim_max... tất cả.
pub fn coalesce_all_all<M: PosGroup>(list: impl IntoIterator<Item = M>) -> M {
    list
        .into_iter()
        .fold(M::don_pos(), |accumulate, x| accumulate.compose(x))
}

// ============================================================================
// PHẦN 2: CÁC KIỂU CÓ SẴN CŨNG LÀ VỊ NHÓM
// ============================================================================

impl Semigroup for String {
    fn compose(self, other: Self) -> Self {
        self + &other // tái sử dụng bộ đệm của chuỗi thứ nhất
    }
}
impl PosGroup for String {
    fn don_pos() -> Self {
        String::new()
    }
}

impl<T> Semigroup for Vec<T> {
    fn compose(mut self, mut other: Self) -> Self {
        self.append(&mut other);
        self
    }
}
impl<T> PosGroup for Vec<T> {
    fn don_pos() -> Self {
        Vec::new()
    }
}

// ============================================================================
// PHẦN 3: KIỂU BỌC (NEWTYPE) — VÌ SỐ NGUYÊN CÓ NHIỀU VỊ NHÓM
// ============================================================================

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct Tong(pub i64);
impl Semigroup for Tong {
    fn compose(self, k: Self) -> Self {
        Tong(self.0 + k.0)
    }
}
impl PosGroup for Tong {
    fn don_pos() -> Self {
        Tong(0)
    }
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct Product(pub i64);
impl Semigroup for Product {
    fn compose(self, k: Self) -> Self {
        Product(self.0.wrapping_mul(k.0))
    }
}
impl PosGroup for Product {
    fn don_pos() -> Self {
        Product(1) // Chú ý: đơn vị của phép nhân là 1, KHÔNG phải 0!
    }
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct Max(pub i64);
impl Semigroup for Max {
    fn compose(self, k: Self) -> Self {
        Max(self.0.max(k.0))
    }
}
impl PosGroup for Max {
    fn don_pos() -> Self {
        Max(i64::MIN) // "âm vô cực": gộp với gì cũng thua
    }
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct Min(pub i64);
impl Semigroup for Min {
    fn compose(self, k: Self) -> Self {
        Min(self.0.min(k.0))
    }
}
impl PosGroup for Min {
    fn don_pos() -> Self {
        Min(i64::MAX)
    }
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct MoiDeu(pub bool); // "tất cả đều đúng" — tương ứng .all()
impl Semigroup for MoiDeu {
    fn compose(self, k: Self) -> Self {
        MoiDeu(self.0 && k.0)
    }
}
impl PosGroup for MoiDeu {
    fn don_pos() -> Self {
        MoiDeu(true)
    }
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct HasFew(pub bool); // "có ít nhất một cái đúng" — tương ứng .any()
impl Semigroup for HasFew {
    fn compose(self, k: Self) -> Self {
        HasFew(self.0 || k.0)
    }
}
impl PosGroup for HasFew {
    fn don_pos() -> Self {
        HasFew(false)
    }
}

/// Vị nhóm "lấy cái đầu tiên có giá trị" — chính là ý tưởng của `Option::or`.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct FirstTien<T>(pub Option<T>);
impl<T> Semigroup for FirstTien<T> {
    fn compose(self, k: Self) -> Self {
        if self.0.is_some() {
            self
        } else {
            k
        }
    }
}
impl<T> PosGroup for FirstTien<T> {
    fn don_pos() -> Self {
        FirstTien(None)
    }
}

// ============================================================================
// PHẦN 4: VỊ NHÓM TÍCH — GHÉP NHIỀU VỊ NHÓM THÀNH MỘT
// ============================================================================
// Mấu chốt: nếu A và B đều là vị nhóm thì cặp (A, B) cũng là vị nhóm.
// Nhờ vậy ta tính được NHIỀU chỉ số chỉ trong MỘT lượt duyệt dữ liệu.

impl<A: Semigroup, B: Semigroup> Semigroup for (A, B) {
    fn compose(self, k: Self) -> Self {
        (self.0.compose(k.0), self.1.compose(k.1))
    }
}
impl<A: PosGroup, B: PosGroup> PosGroup for (A, B) {
    fn don_pos() -> Self {
        (A::don_pos(), B::don_pos())
    }
}

impl<A: Semigroup, B: Semigroup, C: Semigroup, D: Semigroup> Semigroup for (A, B, C, D) {
    fn compose(self, k: Self) -> Self {
        (
            self.0.compose(k.0),
            self.1.compose(k.1),
            self.2.compose(k.2),
            self.3.compose(k.3),
        )
    }
}
impl<A: PosGroup, B: PosGroup, C: PosGroup, D: PosGroup> PosGroup for (A, B, C, D) {
    fn don_pos() -> Self {
        (A::don_pos(), B::don_pos(), C::don_pos(), D::don_pos())
    }
}

// ============================================================================
// PHẦN 5: ỨNG DỤNG THẬT — THỐNG KÊ NHẬT KÝ MÁY CHỦ
// ============================================================================

#[derive(Debug, Clone, PartialEq)]
pub struct SellRecordAccessCap {
    pub path: String,
    pub id_state: u16,
    pub time_ms: i64,
}

/// Bốn chỉ số cần tính, gói trong một vị nhóm tích 4 thành phần.
pub type ThongKe = (Tong, Max, Min, HasFew);

/// Biến một bản ghi thành "đóng góp" của nó vào thống kê tổng.
pub fn into_thong_ke(bg: &SellRecordAccessCap) -> ThongKe {
    (
        Tong(bg.time_ms),
        Max(bg.time_ms),
        Min(bg.time_ms),
        HasFew(bg.id_state >= 500),
    )
}

// ============================================================================
// PHẦN 6: BỘ SINH SỐ GIẢ NGẪU NHIÊN CHO KIỂM THỬ THEO TÍNH CHẤT
// ============================================================================

/// Bộ sinh đồng dư tuyến tính (LCG) — tất định nên kiểm thử luôn lặp lại được.
pub struct Generator(u64);
impl Generator {
    pub fn new(hat_giong: u64) -> Self {
        Generator(hat_giong)
    }
    pub fn num_cont(&mut self) -> i64 {
        // Hằng số của cuốn Numerical Recipes
        self.0 = self
            .0
            .wrapping_mul(6364136223846793005)
            .wrapping_add(1442695040888963407);
        ((self.0 >> 33) as i64) % 1000 - 500 // dải [-500, 499]
    }
}

/// Kiểm chứng LUẬT KẾT HỢP trên nhiều mẫu giả ngẫu nhiên.
pub fn verify_link_hop<M, F>(name: &str, tao: F, samples: usize) -> bool
where
    M: Semigroup + Clone + PartialEq + Debug,
    F: Fn(i64) -> M,
{
    let mut sinh = Generator::new(2026);
    for _ in 0..samples {
        let a = tao(sinh.num_cont());
        let b = tao(sinh.num_cont());
        let c = tao(sinh.num_cont());
        let left = a.clone().compose(b.clone()).compose(c.clone());
        let must = a.clone().compose(b.clone().compose(c.clone()));
        if left != must {
            println!("  ✗ {} VI PHẠM luật kết hợp: {:?} vs {:?}", name, left, must);
            return false;
        }
    }
    println!("  ✓ {}: luật kết hợp đúng trên {} bộ mẫu", name, samples);
    true
}

/// Kiểm chứng LUẬT ĐƠN VỊ trên nhiều mẫu giả ngẫu nhiên.
pub fn verify_don_pos<M, F>(name: &str, tao: F, samples: usize) -> bool
where
    M: PosGroup + Clone + PartialEq + Debug,
    F: Fn(i64) -> M,
{
    let mut sinh = Generator::new(777);
    for _ in 0..samples {
        let a = tao(sinh.num_cont());
        if M::don_pos().compose(a.clone()) != a || a.clone().compose(M::don_pos()) != a {
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
    println!("\n1. HÀM `gop_tat_ca` VẠN NĂNG");
    let so = vec![Tong(3), Tong(8), Tong(-2), Tong(11)];
    println!("   Tổng các số       : {:?}", coalesce_all_all(so));

    let tich = vec![Product(2), Product(3), Product(7)];
    println!("   Tích các số       : {:?}", coalesce_all_all(tich));

    let series = vec![
        String::from("Rust "),
        String::from("thật "),
        String::from("tuyệt!"),
    ];
    println!("   Nối chuỗi         : {:?}", coalesce_all_all(series));

    let mang = vec![vec![1, 2], vec![3], vec![4, 5, 6]];
    println!("   Gộp danh sách     : {:?}", coalesce_all_all(mang));

    let set = vec![MoiDeu(true), MoiDeu(true), MoiDeu(false)];
    println!("   Tất cả đều đạt?   : {:?}", coalesce_all_all(set));

    let cau_hinh: Vec<FirstTien<&str>> = vec![
        FirstTien(None),                // biến môi trường: không có
        FirstTien(Some("config.toml")), // tệp cấu hình: có!
        FirstTien(Some("mac_dinh")),    // giá trị mặc định (không dùng tới)
    ];
    println!("   Nguồn cấu hình đầu: {:?}", coalesce_all_all(cau_hinh));

    // ------------------------------------------------------------------
    // 2. DANH SÁCH RỖNG — GIÁ TRỊ CỦA "HỘP RỖNG"
    // ------------------------------------------------------------------
    println!("\n2. VÌ SAO CẦN PHẦN TỬ ĐƠN VỊ?");
    let empty_cong: Vec<Tong> = Vec::new();
    let rong_nhan: Vec<Product> = Vec::new();
    println!("   Tổng của danh sách RỖNG: {:?}  (đúng: 0)", coalesce_all_all(empty_cong));
    println!(
        "   Tích của danh sách RỖNG: {:?}  (đúng: 1, KHÔNG phải 0!)",
        coalesce_all_all(rong_nhan)
    );

    // ------------------------------------------------------------------
    // 3. VỊ NHÓM TÍCH: 4 CHỈ SỐ TRONG 1 LƯỢT DUYỆT
    // ------------------------------------------------------------------
    println!("\n3. VỊ NHÓM TÍCH — 4 CHỈ SỐ, 1 LƯỢT DUYỆT");
    let order_log = vec![
        SellRecordAccessCap { path: "/api/don-hang".into(), id_state: 200, time_ms: 42 },
        SellRecordAccessCap { path: "/api/thanh-toan".into(), id_state: 500, time_ms: 1350 },
        SellRecordAccessCap { path: "/api/san-pham".into(), id_state: 200, time_ms: 17 },
        SellRecordAccessCap { path: "/api/kho".into(), id_state: 404, time_ms: 8 },
        SellRecordAccessCap { path: "/api/don-hang".into(), id_state: 200, time_ms: 63 },
    ];

    let (tong, cham_nhat, nhanh_nhat, co_loi_may_chu): ThongKe =
        coalesce_all_all(order_log.iter().map(into_thong_ke));

    println!("   Số bản ghi          : {}", order_log.len());
    println!("   Tổng thời gian      : {} ms", tong.0);
    println!("   Trung bình          : {} ms", tong.0 / order_log.len() as i64);
    println!("   Chậm nhất           : {} ms", cham_nhat.0);
    println!("   Nhanh nhất          : {} ms", nhanh_nhat.0);
    println!("   Có lỗi máy chủ 5xx? : {}", co_loi_may_chu.0);

    // ------------------------------------------------------------------
    // 4. LUẬT KẾT HỢP CHO PHÉP CHIA NHỎ & SONG SONG HÓA
    // ------------------------------------------------------------------
    println!("\n4. CHIA NHỎ RỒI GHÉP LẠI CHO CÙNG KẾT QUẢ");
    let all: ThongKe = coalesce_all_all(order_log.iter().map(into_thong_ke));
    let (nua_dau, nua_sau) = order_log.split_at(2);
    let part_1: ThongKe = coalesce_all_all(nua_dau.iter().map(into_thong_ke));
    let part_2: ThongKe = coalesce_all_all(nua_sau.iter().map(into_thong_ke));
    let compose_lai = part_1.compose(part_2);
    assert_eq!(all, compose_lai);
    println!("   Gộp 1 lượt     : {:?}", all);
    println!("   Chia 2 rồi ghép: {:?}", compose_lai);
    println!("   → GIỐNG NHAU ✓ Đây chính là cơ sở để chạy song song trên nhiều nhân CPU.");

    // ------------------------------------------------------------------
    // 5. KIỂM CHỨNG LUẬT BẰNG KIỂM THỬ THEO TÍNH CHẤT
    // ------------------------------------------------------------------
    println!("\n5. KIỂM THỬ THEO TÍNH CHẤT (1.000 bộ mẫu mỗi luật)");
    verify_link_hop("Tong   ", Tong, 1000);
    verify_link_hop("Product   ", Product, 1000);
    verify_link_hop("LonNhat", Max, 1000);
    verify_link_hop("String ", |n: i64| n.to_string(), 1000);
    verify_don_pos("Tong   ", Tong, 1000);
    verify_don_pos("Product   ", Product, 1000);
    verify_don_pos("LonNhat", Max, 1000);

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
    let mut nhan_vien = vec![
        ("Kỹ thuật", 3u32, "An"),
        ("Kinh doanh", 5, "Bình"),
        ("Kỹ thuật", 5, "Cường"),
        ("Kỹ thuật", 5, "Anh"),
    ];
    nhan_vien.sort_by(|x, y| {
        x.0.cmp(y.0) // 1. phòng ban tăng dần
            .then(y.1.cmp(&x.1)) // ⊕ 2. thâm niên giảm dần
            .then(x.2.cmp(y.2)) // ⊕ 3. họ tên tăng dần
    });
    for nv in &nhan_vien {
        println!("   {:<12} {} năm  {}", nv.0, nv.1, nv.2);
    }
    println!("   (Ordering::Equal chính là \"hộp rỗng\": bằng nhau thì xét tiêu chí sau)");

    // ------------------------------------------------------------------
    // 8. LUẬT PHẢN XẠ VÀ CÂU CHUYỆN f64 / NaN
    // ------------------------------------------------------------------
    println!("\n8. LUẬT CÓ THẬT: f64 KHÔNG CÓ TRAIT `Eq`");
    let nan = f64::NAN;
    println!("   f64::NAN == f64::NAN  ->  {}", nan == nan);
    println!("   → Luật phản xạ (a == a) bị phá vỡ, nên Rust TỪ CHỐI cài `Eq` cho f64.");
    println!("   → Hệ quả: không thể dùng f64 làm khóa HashMap / phần tử HashSet.");
    let so_sanh: Ordering = 3i64.cmp(&5i64);
    println!("   (Còn i64 thì có đủ Eq + Ord: 3.cmp(&5) = {:?})", so_sanh);

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
    fn tong_tuan_thu_luat_ket_hop() {
        assert!(verify_link_hop("Tong", Tong, 500));
    }

    #[test]
    fn tong_tuan_thu_luat_don_vi() {
        assert!(verify_don_pos("Tong", Tong, 500));
    }

    #[test]
    fn tich_tuan_thu_ca_hai_luat() {
        assert!(verify_link_hop("Product", Product, 500));
        assert!(verify_don_pos("Product", Product, 500));
    }

    #[test]
    fn chuoi_tuan_thu_luat_ket_hop() {
        assert!(verify_link_hop("String", |n: i64| n.to_string(), 500));
    }

    #[test]
    fn list_empty_return_ve_part_from_don_pos() {
        let empty_cong: Vec<Tong> = Vec::new();
        let rong_nhan: Vec<Product> = Vec::new();
        let rong_max: Vec<Max> = Vec::new();
        assert_eq!(coalesce_all_all(empty_cong), Tong(0));
        assert_eq!(coalesce_all_all(rong_nhan), Product(1));
        assert_eq!(coalesce_all_all(rong_max), Max(i64::MIN));
    }

    #[test]
    fn vi_nhom_tich_gop_dung_bon_chi_so() {
        let order_log = vec![
            SellRecordAccessCap { path: "/a".into(), id_state: 200, time_ms: 10 },
            SellRecordAccessCap { path: "/b".into(), id_state: 503, time_ms: 40 },
            SellRecordAccessCap { path: "/c".into(), id_state: 200, time_ms: 25 },
        ];
        let (tong, max, min, error): ThongKe = coalesce_all_all(order_log.iter().map(into_thong_ke));
        assert_eq!(tong, Tong(75));
        assert_eq!(max, Max(40));
        assert_eq!(min, Min(10));
        assert_eq!(error, HasFew(true));
    }

    /// Đây là bài test QUAN TRỌNG NHẤT chương: nó chứng minh rằng
    /// chia nhỏ dữ liệu rồi ghép lại luôn cho cùng kết quả —
    /// tức là thuật toán này SONG SONG HÓA ĐƯỢC một cách an toàn.
    #[test]
    fn chia_nho_roi_ghep_lai_cho_cung_ket_qua() {
        let mut sinh = Generator::new(12345);
        let data: Vec<Tong> = (0..100).map(|_| Tong(sinh.num_cont())).collect();

        let mot_luot = coalesce_all_all(data.clone());
        for diem_cat in [0usize, 1, 37, 50, 99, 100] {
            let (left, must) = data.split_at(diem_cat);
            let compose = coalesce_all_all(left.to_vec()).compose(coalesce_all_all(must.to_vec()));
            assert_eq!(mot_luot, compose, "Sai khi cắt tại vị trí {}", diem_cat);
        }
    }

    #[test]
    fn op_tru_no_must_nua_group() {
        // Phản ví dụ: chứng minh phép trừ VI PHẠM luật kết hợp.
        assert_ne!((10i64 - 3) - 2, 10i64 - (3 - 2));
    }

    #[test]
    fn nan_pha_vo_luat_phan_xa() {
        let nan = f64::NAN;
        assert!(!(nan == nan), "NaN phải KHÔNG bằng chính nó theo IEEE 754");
        // Còn số nguyên thì luôn thỏa luật phản xạ:
        for i in -5i64..5 {
            assert!(i == i);
        }
    }
}
