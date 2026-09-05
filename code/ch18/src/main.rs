#![allow(dead_code, unused_variables, unused_imports)]
// Tệp: src/main.rs
// Chương trình thực chiến: Nửa nhóm, Vị nhóm và Kiểm thử theo tính chất

use std::cmp::Ordering;
use std::fmt::Debug;

// ============================================================================
// PHẦN 1: HAI TRAIT NỀN TẢNG
// ============================================================================

/// Nửa nhóm (Semigroup): có phép gộp hai thành một, tuân LUẬT KẾT HỢP.
pub trait NuaNhom {
    fn ghep(self, khac: Self) -> Self;
}

/// Vị nhóm (Monoid): nửa nhóm có thêm PHẦN TỬ ĐƠN VỊ.
pub trait ViNhom: NuaNhom + Sized {
    fn don_vi() -> Self;
}

/// Hàm gộp vạn năng: dùng được cho MỌI vị nhóm.
/// Nó thay thế cho tinh_tong, noi_chuoi, gop_mang, tim_max... tất cả.
pub fn gop_tat_ca<M: ViNhom>(danh_sach: impl IntoIterator<Item = M>) -> M {
    danh_sach
        .into_iter()
        .fold(M::don_vi(), |tich_luy, x| tich_luy.ghep(x))
}

// ============================================================================
// PHẦN 2: CÁC KIỂU CÓ SẴN CŨNG LÀ VỊ NHÓM
// ============================================================================

impl NuaNhom for String {
    fn ghep(self, khac: Self) -> Self {
        self + &khac // tái sử dụng bộ đệm của chuỗi thứ nhất
    }
}
impl ViNhom for String {
    fn don_vi() -> Self {
        String::new()
    }
}

impl<T> NuaNhom for Vec<T> {
    fn ghep(mut self, mut khac: Self) -> Self {
        self.append(&mut khac);
        self
    }
}
impl<T> ViNhom for Vec<T> {
    fn don_vi() -> Self {
        Vec::new()
    }
}

// ============================================================================
// PHẦN 3: KIỂU BỌC (NEWTYPE) — VÌ SỐ NGUYÊN CÓ NHIỀU VỊ NHÓM
// ============================================================================

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct Tong(pub i64);
impl NuaNhom for Tong {
    fn ghep(self, k: Self) -> Self {
        Tong(self.0 + k.0)
    }
}
impl ViNhom for Tong {
    fn don_vi() -> Self {
        Tong(0)
    }
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct Tich(pub i64);
impl NuaNhom for Tich {
    fn ghep(self, k: Self) -> Self {
        Tich(self.0.wrapping_mul(k.0))
    }
}
impl ViNhom for Tich {
    fn don_vi() -> Self {
        Tich(1) // Chú ý: đơn vị của phép nhân là 1, KHÔNG phải 0!
    }
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct LonNhat(pub i64);
impl NuaNhom for LonNhat {
    fn ghep(self, k: Self) -> Self {
        LonNhat(self.0.max(k.0))
    }
}
impl ViNhom for LonNhat {
    fn don_vi() -> Self {
        LonNhat(i64::MIN) // "âm vô cực": gộp với gì cũng thua
    }
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct NhoNhat(pub i64);
impl NuaNhom for NhoNhat {
    fn ghep(self, k: Self) -> Self {
        NhoNhat(self.0.min(k.0))
    }
}
impl ViNhom for NhoNhat {
    fn don_vi() -> Self {
        NhoNhat(i64::MAX)
    }
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct MoiDeu(pub bool); // "tất cả đều đúng" — tương ứng .all()
impl NuaNhom for MoiDeu {
    fn ghep(self, k: Self) -> Self {
        MoiDeu(self.0 && k.0)
    }
}
impl ViNhom for MoiDeu {
    fn don_vi() -> Self {
        MoiDeu(true)
    }
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct CoIt(pub bool); // "có ít nhất một cái đúng" — tương ứng .any()
impl NuaNhom for CoIt {
    fn ghep(self, k: Self) -> Self {
        CoIt(self.0 || k.0)
    }
}
impl ViNhom for CoIt {
    fn don_vi() -> Self {
        CoIt(false)
    }
}

/// Vị nhóm "lấy cái đầu tiên có giá trị" — chính là ý tưởng của `Option::or`.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct DauTien<T>(pub Option<T>);
impl<T> NuaNhom for DauTien<T> {
    fn ghep(self, k: Self) -> Self {
        if self.0.is_some() {
            self
        } else {
            k
        }
    }
}
impl<T> ViNhom for DauTien<T> {
    fn don_vi() -> Self {
        DauTien(None)
    }
}

// ============================================================================
// PHẦN 4: VỊ NHÓM TÍCH — GHÉP NHIỀU VỊ NHÓM THÀNH MỘT
// ============================================================================
// Mấu chốt: nếu A và B đều là vị nhóm thì cặp (A, B) cũng là vị nhóm.
// Nhờ vậy ta tính được NHIỀU chỉ số chỉ trong MỘT lượt duyệt dữ liệu.

impl<A: NuaNhom, B: NuaNhom> NuaNhom for (A, B) {
    fn ghep(self, k: Self) -> Self {
        (self.0.ghep(k.0), self.1.ghep(k.1))
    }
}
impl<A: ViNhom, B: ViNhom> ViNhom for (A, B) {
    fn don_vi() -> Self {
        (A::don_vi(), B::don_vi())
    }
}

impl<A: NuaNhom, B: NuaNhom, C: NuaNhom, D: NuaNhom> NuaNhom for (A, B, C, D) {
    fn ghep(self, k: Self) -> Self {
        (
            self.0.ghep(k.0),
            self.1.ghep(k.1),
            self.2.ghep(k.2),
            self.3.ghep(k.3),
        )
    }
}
impl<A: ViNhom, B: ViNhom, C: ViNhom, D: ViNhom> ViNhom for (A, B, C, D) {
    fn don_vi() -> Self {
        (A::don_vi(), B::don_vi(), C::don_vi(), D::don_vi())
    }
}

// ============================================================================
// PHẦN 5: ỨNG DỤNG THẬT — THỐNG KÊ NHẬT KÝ MÁY CHỦ
// ============================================================================

#[derive(Debug, Clone, PartialEq)]
pub struct BanGhiTruyCap {
    pub duong_dan: String,
    pub ma_trang_thai: u16,
    pub thoi_gian_ms: i64,
}

/// Bốn chỉ số cần tính, gói trong một vị nhóm tích 4 thành phần.
pub type ThongKe = (Tong, LonNhat, NhoNhat, CoIt);

/// Biến một bản ghi thành "đóng góp" của nó vào thống kê tổng.
pub fn thanh_thong_ke(bg: &BanGhiTruyCap) -> ThongKe {
    (
        Tong(bg.thoi_gian_ms),
        LonNhat(bg.thoi_gian_ms),
        NhoNhat(bg.thoi_gian_ms),
        CoIt(bg.ma_trang_thai >= 500),
    )
}

// ============================================================================
// PHẦN 6: BỘ SINH SỐ GIẢ NGẪU NHIÊN CHO KIỂM THỬ THEO TÍNH CHẤT
// ============================================================================

/// Bộ sinh đồng dư tuyến tính (LCG) — tất định nên kiểm thử luôn lặp lại được.
pub struct BoSinh(u64);
impl BoSinh {
    pub fn moi(hat_giong: u64) -> Self {
        BoSinh(hat_giong)
    }
    pub fn so_tiep(&mut self) -> i64 {
        // Hằng số của cuốn Numerical Recipes
        self.0 = self
            .0
            .wrapping_mul(6364136223846793005)
            .wrapping_add(1442695040888963407);
        ((self.0 >> 33) as i64) % 1000 - 500 // dải [-500, 499]
    }
}

/// Kiểm chứng LUẬT KẾT HỢP trên nhiều mẫu giả ngẫu nhiên.
pub fn kiem_chung_ket_hop<M, F>(ten: &str, tao: F, so_mau: usize) -> bool
where
    M: NuaNhom + Clone + PartialEq + Debug,
    F: Fn(i64) -> M,
{
    let mut sinh = BoSinh::moi(2026);
    for _ in 0..so_mau {
        let a = tao(sinh.so_tiep());
        let b = tao(sinh.so_tiep());
        let c = tao(sinh.so_tiep());
        let trai = a.clone().ghep(b.clone()).ghep(c.clone());
        let phai = a.clone().ghep(b.clone().ghep(c.clone()));
        if trai != phai {
            println!("  ✗ {} VI PHẠM luật kết hợp: {:?} vs {:?}", ten, trai, phai);
            return false;
        }
    }
    println!("  ✓ {}: luật kết hợp đúng trên {} bộ mẫu", ten, so_mau);
    true
}

/// Kiểm chứng LUẬT ĐƠN VỊ trên nhiều mẫu giả ngẫu nhiên.
pub fn kiem_chung_don_vi<M, F>(ten: &str, tao: F, so_mau: usize) -> bool
where
    M: ViNhom + Clone + PartialEq + Debug,
    F: Fn(i64) -> M,
{
    let mut sinh = BoSinh::moi(777);
    for _ in 0..so_mau {
        let a = tao(sinh.so_tiep());
        if M::don_vi().ghep(a.clone()) != a || a.clone().ghep(M::don_vi()) != a {
            println!("  ✗ {} VI PHẠM luật đơn vị với {:?}", ten, a);
            return false;
        }
    }
    println!("  ✓ {}: luật đơn vị đúng trên {} bộ mẫu", ten, so_mau);
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
    println!("   Tổng các số       : {:?}", gop_tat_ca(so));

    let tich = vec![Tich(2), Tich(3), Tich(7)];
    println!("   Tích các số       : {:?}", gop_tat_ca(tich));

    let chuoi = vec![
        String::from("Rust "),
        String::from("thật "),
        String::from("tuyệt!"),
    ];
    println!("   Nối chuỗi         : {:?}", gop_tat_ca(chuoi));

    let mang = vec![vec![1, 2], vec![3], vec![4, 5, 6]];
    println!("   Gộp danh sách     : {:?}", gop_tat_ca(mang));

    let dat = vec![MoiDeu(true), MoiDeu(true), MoiDeu(false)];
    println!("   Tất cả đều đạt?   : {:?}", gop_tat_ca(dat));

    let cau_hinh: Vec<DauTien<&str>> = vec![
        DauTien(None),                // biến môi trường: không có
        DauTien(Some("config.toml")), // tệp cấu hình: có!
        DauTien(Some("mac_dinh")),    // giá trị mặc định (không dùng tới)
    ];
    println!("   Nguồn cấu hình đầu: {:?}", gop_tat_ca(cau_hinh));

    // ------------------------------------------------------------------
    // 2. DANH SÁCH RỖNG — GIÁ TRỊ CỦA "HỘP RỖNG"
    // ------------------------------------------------------------------
    println!("\n2. VÌ SAO CẦN PHẦN TỬ ĐƠN VỊ?");
    let rong_cong: Vec<Tong> = Vec::new();
    let rong_nhan: Vec<Tich> = Vec::new();
    println!("   Tổng của danh sách RỖNG: {:?}  (đúng: 0)", gop_tat_ca(rong_cong));
    println!(
        "   Tích của danh sách RỖNG: {:?}  (đúng: 1, KHÔNG phải 0!)",
        gop_tat_ca(rong_nhan)
    );

    // ------------------------------------------------------------------
    // 3. VỊ NHÓM TÍCH: 4 CHỈ SỐ TRONG 1 LƯỢT DUYỆT
    // ------------------------------------------------------------------
    println!("\n3. VỊ NHÓM TÍCH — 4 CHỈ SỐ, 1 LƯỢT DUYỆT");
    let nhat_ky = vec![
        BanGhiTruyCap { duong_dan: "/api/don-hang".into(), ma_trang_thai: 200, thoi_gian_ms: 42 },
        BanGhiTruyCap { duong_dan: "/api/thanh-toan".into(), ma_trang_thai: 500, thoi_gian_ms: 1350 },
        BanGhiTruyCap { duong_dan: "/api/san-pham".into(), ma_trang_thai: 200, thoi_gian_ms: 17 },
        BanGhiTruyCap { duong_dan: "/api/kho".into(), ma_trang_thai: 404, thoi_gian_ms: 8 },
        BanGhiTruyCap { duong_dan: "/api/don-hang".into(), ma_trang_thai: 200, thoi_gian_ms: 63 },
    ];

    let (tong, cham_nhat, nhanh_nhat, co_loi_may_chu): ThongKe =
        gop_tat_ca(nhat_ky.iter().map(thanh_thong_ke));

    println!("   Số bản ghi          : {}", nhat_ky.len());
    println!("   Tổng thời gian      : {} ms", tong.0);
    println!("   Trung bình          : {} ms", tong.0 / nhat_ky.len() as i64);
    println!("   Chậm nhất           : {} ms", cham_nhat.0);
    println!("   Nhanh nhất          : {} ms", nhanh_nhat.0);
    println!("   Có lỗi máy chủ 5xx? : {}", co_loi_may_chu.0);

    // ------------------------------------------------------------------
    // 4. LUẬT KẾT HỢP CHO PHÉP CHIA NHỎ & SONG SONG HÓA
    // ------------------------------------------------------------------
    println!("\n4. CHIA NHỎ RỒI GHÉP LẠI CHO CÙNG KẾT QUẢ");
    let tat_ca: ThongKe = gop_tat_ca(nhat_ky.iter().map(thanh_thong_ke));
    let (nua_dau, nua_sau) = nhat_ky.split_at(2);
    let phan_1: ThongKe = gop_tat_ca(nua_dau.iter().map(thanh_thong_ke));
    let phan_2: ThongKe = gop_tat_ca(nua_sau.iter().map(thanh_thong_ke));
    let ghep_lai = phan_1.ghep(phan_2);
    assert_eq!(tat_ca, ghep_lai);
    println!("   Gộp 1 lượt     : {:?}", tat_ca);
    println!("   Chia 2 rồi ghép: {:?}", ghep_lai);
    println!("   → GIỐNG NHAU ✓ Đây chính là cơ sở để chạy song song trên nhiều nhân CPU.");

    // ------------------------------------------------------------------
    // 5. KIỂM CHỨNG LUẬT BẰNG KIỂM THỬ THEO TÍNH CHẤT
    // ------------------------------------------------------------------
    println!("\n5. KIỂM THỬ THEO TÍNH CHẤT (1.000 bộ mẫu mỗi luật)");
    kiem_chung_ket_hop("Tong   ", Tong, 1000);
    kiem_chung_ket_hop("Tich   ", Tich, 1000);
    kiem_chung_ket_hop("LonNhat", LonNhat, 1000);
    kiem_chung_ket_hop("String ", |n: i64| n.to_string(), 1000);
    kiem_chung_don_vi("Tong   ", Tong, 1000);
    kiem_chung_don_vi("Tich   ", Tich, 1000);
    kiem_chung_don_vi("LonNhat", LonNhat, 1000);

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
mod kiem_thu {
    use super::*;

    #[test]
    fn tong_tuan_thu_luat_ket_hop() {
        assert!(kiem_chung_ket_hop("Tong", Tong, 500));
    }

    #[test]
    fn tong_tuan_thu_luat_don_vi() {
        assert!(kiem_chung_don_vi("Tong", Tong, 500));
    }

    #[test]
    fn tich_tuan_thu_ca_hai_luat() {
        assert!(kiem_chung_ket_hop("Tich", Tich, 500));
        assert!(kiem_chung_don_vi("Tich", Tich, 500));
    }

    #[test]
    fn chuoi_tuan_thu_luat_ket_hop() {
        assert!(kiem_chung_ket_hop("String", |n: i64| n.to_string(), 500));
    }

    #[test]
    fn danh_sach_rong_tra_ve_phan_tu_don_vi() {
        let rong_cong: Vec<Tong> = Vec::new();
        let rong_nhan: Vec<Tich> = Vec::new();
        let rong_max: Vec<LonNhat> = Vec::new();
        assert_eq!(gop_tat_ca(rong_cong), Tong(0));
        assert_eq!(gop_tat_ca(rong_nhan), Tich(1));
        assert_eq!(gop_tat_ca(rong_max), LonNhat(i64::MIN));
    }

    #[test]
    fn vi_nhom_tich_gop_dung_bon_chi_so() {
        let nhat_ky = vec![
            BanGhiTruyCap { duong_dan: "/a".into(), ma_trang_thai: 200, thoi_gian_ms: 10 },
            BanGhiTruyCap { duong_dan: "/b".into(), ma_trang_thai: 503, thoi_gian_ms: 40 },
            BanGhiTruyCap { duong_dan: "/c".into(), ma_trang_thai: 200, thoi_gian_ms: 25 },
        ];
        let (tong, max, min, loi): ThongKe = gop_tat_ca(nhat_ky.iter().map(thanh_thong_ke));
        assert_eq!(tong, Tong(75));
        assert_eq!(max, LonNhat(40));
        assert_eq!(min, NhoNhat(10));
        assert_eq!(loi, CoIt(true));
    }

    /// Đây là bài test QUAN TRỌNG NHẤT chương: nó chứng minh rằng
    /// chia nhỏ dữ liệu rồi ghép lại luôn cho cùng kết quả —
    /// tức là thuật toán này SONG SONG HÓA ĐƯỢC một cách an toàn.
    #[test]
    fn chia_nho_roi_ghep_lai_cho_cung_ket_qua() {
        let mut sinh = BoSinh::moi(12345);
        let du_lieu: Vec<Tong> = (0..100).map(|_| Tong(sinh.so_tiep())).collect();

        let mot_luot = gop_tat_ca(du_lieu.clone());
        for diem_cat in [0usize, 1, 37, 50, 99, 100] {
            let (trai, phai) = du_lieu.split_at(diem_cat);
            let ghep = gop_tat_ca(trai.to_vec()).ghep(gop_tat_ca(phai.to_vec()));
            assert_eq!(mot_luot, ghep, "Sai khi cắt tại vị trí {}", diem_cat);
        }
    }

    #[test]
    fn phep_tru_khong_phai_nua_nhom() {
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
