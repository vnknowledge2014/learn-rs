#![allow(dead_code, unused_variables)]
//! Chương 59 — Thiết kế hệ thống mở rộng: cân bằng tải, băm nhất quán,
//! giới hạn tần suất, back-pressure. Bổ sung cho Chương 48–54.

use std::collections::{BTreeMap, HashMap, VecDeque};

// ============================================================================
// 1. CÂN BẰNG TẢI (Load Balancing) — ba chiến lược
// ============================================================================

#[derive(Debug, Clone)]
pub struct MayChu {
    pub ten: String,
    pub ket_noi_hien_tai: u32,
    pub trong_so: u32, // máy mạnh hơn có trọng số cao hơn
}

pub trait ChienLuocCanBang {
    fn chon<'a>(&mut self, may_chu: &'a [MayChu]) -> Option<&'a MayChu>;
}

/// Xoay vòng (Round-Robin): lần lượt từng máy.
pub struct XoayVong { vi_tri: usize }
impl XoayVong { pub fn moi() -> Self { XoayVong { vi_tri: 0 } } }
impl ChienLuocCanBang for XoayVong {
    fn chon<'a>(&mut self, may_chu: &'a [MayChu]) -> Option<&'a MayChu> {
        if may_chu.is_empty() { return None; }
        let m = &may_chu[self.vi_tri % may_chu.len()];
        self.vi_tri += 1;
        Some(m)
    }
}

/// Ít kết nối nhất (Least-Connections): gửi tới máy đang rảnh nhất.
pub struct ItKetNoi;
impl ChienLuocCanBang for ItKetNoi {
    fn chon<'a>(&mut self, may_chu: &'a [MayChu]) -> Option<&'a MayChu> {
        may_chu.iter().min_by_key(|m| m.ket_noi_hien_tai)
    }
}

/// Xoay vòng có trọng số (Weighted): máy mạnh nhận nhiều hơn theo tỷ lệ trọng số.
pub struct XoayVongTrongSo { dem: u32 }
impl XoayVongTrongSo { pub fn moi() -> Self { XoayVongTrongSo { dem: 0 } } }
impl ChienLuocCanBang for XoayVongTrongSo {
    fn chon<'a>(&mut self, may_chu: &'a [MayChu]) -> Option<&'a MayChu> {
        if may_chu.is_empty() { return None; }
        let tong: u32 = may_chu.iter().map(|m| m.trong_so).sum();
        if tong == 0 { return may_chu.first(); }
        let muc = self.dem % tong;
        self.dem += 1;
        let mut cong_don = 0;
        for m in may_chu {
            cong_don += m.trong_so;
            if muc < cong_don { return Some(m); }
        }
        may_chu.last()
    }
}

// ============================================================================
// 2. BĂM NHẤT QUÁN (Consistent Hashing) — thêm/bớt máy chủ không xáo trộn toàn bộ
// ============================================================================

/// Băm đơn giản, tất định (FNV-1a) — đủ cho minh họa.
pub fn bam(khoa: &str) -> u64 {
    // FNV-1a để trộn từng byte...
    let mut h: u64 = 0xcbf29ce484222325;
    for b in khoa.bytes() {
        h ^= b as u64;
        h = h.wrapping_mul(0x100000001b3);
    }
    // ...rồi bộ trộn bit cuối (splitmix64 finalizer) để đạt "hiệu ứng tuyết lở":
    // đổi 1 bit đầu vào -> đổi ~1/2 số bit đầu ra. Thiếu bước này, các chuỗi
    // gần giống nhau ("A#0", "A#1") cho hash gần nhau -> vòng băm phân bố LỆCH.
    h ^= h >> 30;
    h = h.wrapping_mul(0xbf58476d1ce4e5b9);
    h ^= h >> 27;
    h = h.wrapping_mul(0x94d049bb133111eb);
    h ^= h >> 31;
    h
}

/// Vòng băm nhất quán. Mỗi máy chủ được đặt tại NHIỀU điểm ảo trên vòng,
/// để phân bố đều. Khóa đi theo chiều kim đồng hồ tới máy chủ gần nhất.
pub struct VongBamNhatQuan {
    vong: BTreeMap<u64, String>, // điểm trên vòng -> tên máy chủ
    so_diem_ao: u32,
}

impl VongBamNhatQuan {
    pub fn moi(so_diem_ao: u32) -> Self {
        VongBamNhatQuan { vong: BTreeMap::new(), so_diem_ao }
    }
    pub fn them_may_chu(&mut self, ten: &str) {
        for i in 0..self.so_diem_ao {
            self.vong.insert(bam(&format!("{}#{}", ten, i)), ten.to_string());
        }
    }
    pub fn bo_may_chu(&mut self, ten: &str) {
        self.vong.retain(|_, v| v != ten);
    }
    /// Tìm máy chủ chịu trách nhiệm cho một khóa: điểm đầu tiên >= hash(khóa),
    /// hoặc quay vòng về đầu (vòng tròn).
    pub fn tim_may_chu(&self, khoa: &str) -> Option<&str> {
        if self.vong.is_empty() { return None; }
        let h = bam(khoa);
        self.vong.range(h..).next()
            .or_else(|| self.vong.iter().next()) // quay vòng
            .map(|(_, v)| v.as_str())
    }
}

// ============================================================================
// 3. GIỚI HẠN TẦN SUẤT (Rate Limiting) — thuật toán Token Bucket
// ============================================================================

/// Xô token: mỗi yêu cầu tốn 1 token; token được đổ lại theo thời gian.
/// Cho phép "bùng nổ" ngắn (dùng token tích lũy) nhưng giới hạn tốc độ trung bình.
pub struct XoToken {
    dung_luong: f64,
    token: f64,
    toc_do_do: f64, // token/giây
}

impl XoToken {
    pub fn moi(dung_luong: f64, toc_do_do: f64) -> Self {
        XoToken { dung_luong, token: dung_luong, toc_do_do }
    }
    /// Nạp token theo thời gian trôi qua (giây), rồi thử tiêu 1 token.
    pub fn cho_phep(&mut self, thoi_gian_troi: f64) -> bool {
        self.token = (self.token + thoi_gian_troi * self.toc_do_do).min(self.dung_luong);
        if self.token >= 1.0 {
            self.token -= 1.0;
            true
        } else {
            false
        }
    }
    pub fn token_con(&self) -> f64 { self.token }
}

// ============================================================================
// 4. BACK-PRESSURE — hàng đợi có giới hạn, từ chối khi đầy
// ============================================================================

#[derive(Debug, PartialEq)]
pub enum KetQuaNhan {
    DaNhan,
    TuChoi, // hàng đầy — báo ngược lên nguồn để nó chậm lại (back-pressure)
}

/// Hàng đợi có giới hạn: khi đầy, TỪ CHỐI thay vì phình vô hạn.
/// Đây là cốt lõi của back-pressure: hệ thống chậm phải BÁO cho hệ thống nhanh
/// biết mà giảm tốc, thay vì âm thầm chất đống đến khi hết RAM.
pub struct HangDoiGioiHan<T> {
    hang: VecDeque<T>,
    suc_chua: usize,
    da_tu_choi: u64,
}

impl<T> HangDoiGioiHan<T> {
    pub fn moi(suc_chua: usize) -> Self {
        HangDoiGioiHan { hang: VecDeque::new(), suc_chua, da_tu_choi: 0 }
    }
    pub fn gui(&mut self, viec: T) -> KetQuaNhan {
        if self.hang.len() >= self.suc_chua {
            self.da_tu_choi += 1;
            KetQuaNhan::TuChoi
        } else {
            self.hang.push_back(viec);
            KetQuaNhan::DaNhan
        }
    }
    pub fn nhan(&mut self) -> Option<T> { self.hang.pop_front() }
    pub fn so_cho(&self) -> usize { self.hang.len() }
    pub fn so_da_tu_choi(&self) -> u64 { self.da_tu_choi }
}

fn main() {
    println!("═══════════════════════════════════════════════════════════════");
    println!("   THIẾT KẾ HỆ THỐNG MỞ RỘNG: CÂN BẰNG TẢI · BĂM NHẤT QUÁN     ");
    println!("═══════════════════════════════════════════════════════════════");

    let may = vec![
        MayChu { ten: "web-1".into(), ket_noi_hien_tai: 5, trong_so: 1 },
        MayChu { ten: "web-2".into(), ket_noi_hien_tai: 2, trong_so: 3 },
        MayChu { ten: "web-3".into(), ket_noi_hien_tai: 8, trong_so: 1 },
    ];

    println!("\n1. CÂN BẰNG TẢI");
    let mut xv = XoayVong::moi();
    let chuoi: Vec<&str> = (0..5).filter_map(|_| xv.chon(&may).map(|m| m.ten.as_str())).collect();
    println!("   Xoay vòng     : {:?}", chuoi);
    println!("   Ít kết nối    : {:?}", ItKetNoi.chon(&may).map(|m| &m.ten)); // web-2 (2 kết nối)
    let mut wt = XoayVongTrongSo::moi();
    let ws: Vec<&str> = (0..5).filter_map(|_| wt.chon(&may).map(|m| m.ten.as_str())).collect();
    println!("   Trọng số      : {:?} (web-2 xuất hiện nhiều nhất)", ws);

    println!("\n2. BĂM NHẤT QUÁN — thêm/bớt máy chủ ít xáo trộn");
    let mut vong = VongBamNhatQuan::moi(100);
    for m in ["cache-A", "cache-B", "cache-C"] { vong.them_may_chu(m); }
    let khoa = ["user:1", "user:2", "user:3", "user:4", "user:5"];
    let truoc: HashMap<&str, String> = khoa.iter()
        .map(|k| (*k, vong.tim_may_chu(k).unwrap().to_string())).collect();
    println!("   Trước khi bỏ cache-B: {:?}", truoc);
    vong.bo_may_chu("cache-B");
    let mut giu_nguyen = 0;
    for k in &khoa {
        let sau = vong.tim_may_chu(k).unwrap();
        if sau == truoc[k] { giu_nguyen += 1; }
    }
    println!("   Sau khi bỏ cache-B: {}/{} khóa GIỮ NGUYÊN máy chủ", giu_nguyen, khoa.len());
    println!("   → Băm thường (hash % N) sẽ xáo trộn GẦN NHƯ TẤT CẢ khóa!");

    println!("\n3. GIỚI HẠN TẦN SUẤT (Token Bucket: 3 token, đổ 1/giây)");
    let mut xo = XoToken::moi(3.0, 1.0);
    for i in 1..=5 {
        print!("   Yêu cầu {} (tức thì): {} | ", i, if xo.cho_phep(0.0) { "CHO" } else { "CHẶN" });
    }
    println!();
    println!("   Chờ 2 giây rồi thử lại: {}", if xo.cho_phep(2.0) { "CHO" } else { "CHẶN" });

    println!("\n4. BACK-PRESSURE (hàng đợi sức chứa 3)");
    let mut hq: HangDoiGioiHan<u32> = HangDoiGioiHan::moi(3);
    for i in 1..=5 {
        println!("   Gửi việc {}: {:?}", i, hq.gui(i));
    }
    println!("   → 2 việc bị TỪ CHỐI. Nguồn gửi phải chậm lại, không được ép thêm.");

    println!("\n═══════════════════════════════════════════════════════════════");
    println!("   MỞ RỘNG NGANG = PHÂN TÁN THÔNG MINH + BIẾT NÓI \"KHÔNG\"        ");
    println!("═══════════════════════════════════════════════════════════════");
}

#[cfg(test)]
mod kiem_thu {
    use super::*;

    fn may3() -> Vec<MayChu> {
        vec![
            MayChu { ten: "a".into(), ket_noi_hien_tai: 5, trong_so: 1 },
            MayChu { ten: "b".into(), ket_noi_hien_tai: 2, trong_so: 3 },
            MayChu { ten: "c".into(), ket_noi_hien_tai: 8, trong_so: 1 },
        ]
    }

    #[test]
    fn xoay_vong_deu_va_quay_lai() {
        let m = may3();
        let mut xv = XoayVong::moi();
        let ten: Vec<&str> = (0..6).map(|_| xv.chon(&m).unwrap().ten.as_str()).collect();
        assert_eq!(ten, vec!["a", "b", "c", "a", "b", "c"]);
    }

    #[test]
    fn it_ket_noi_chon_may_ranh_nhat() {
        assert_eq!(ItKetNoi.chon(&may3()).unwrap().ten, "b"); // b có 2 kết nối
    }

    #[test]
    fn trong_so_phan_bo_dung_ty_le() {
        let m = may3(); // trọng số a=1, b=3, c=1 -> tổng 5
        let mut wt = XoayVongTrongSo::moi();
        let mut dem: HashMap<String, u32> = HashMap::new();
        for _ in 0..5 { *dem.entry(wt.chon(&m).unwrap().ten.clone()).or_insert(0) += 1; }
        assert_eq!(dem["b"], 3); // b nhận 3/5
        assert_eq!(dem["a"], 1);
        assert_eq!(dem["c"], 1);
    }

    #[test]
    fn bam_nhat_quan_it_xao_tron_khi_bo_may() {
        let mut vong = VongBamNhatQuan::moi(150);
        for m in ["A", "B", "C", "D"] { vong.them_may_chu(m); }
        let khoa: Vec<String> = (0..1000).map(|i| format!("k{}", i)).collect();
        let truoc: HashMap<&String, String> =
            khoa.iter().map(|k| (k, vong.tim_may_chu(k).unwrap().to_string())).collect();

        vong.bo_may_chu("B"); // bỏ 1 trong 4 máy

        let giu = khoa.iter().filter(|k| vong.tim_may_chu(k).unwrap() == truoc[*k]).count();
        // Lý thuyết: chỉ ~1/4 khóa (thuộc B) phải di chuyển. Giữ nguyên phải > 60%.
        assert!(giu as f64 / 1000.0 > 0.6, "chỉ giữ {} khóa — xáo trộn quá nhiều", giu);
    }

    #[test]
    fn bam_nhat_quan_khoa_on_dinh() {
        let mut vong = VongBamNhatQuan::moi(50);
        vong.them_may_chu("X");
        vong.them_may_chu("Y");
        // Cùng một khóa luôn cho cùng một máy chủ
        let a = vong.tim_may_chu("user:42").unwrap().to_string();
        let b = vong.tim_may_chu("user:42").unwrap().to_string();
        assert_eq!(a, b);
    }

    #[test]
    fn token_bucket_gioi_han_va_hoi_phuc() {
        let mut xo = XoToken::moi(3.0, 1.0);
        // 3 token đầu -> cho; token thứ 4 tức thì -> chặn
        assert!(xo.cho_phep(0.0));
        assert!(xo.cho_phep(0.0));
        assert!(xo.cho_phep(0.0));
        assert!(!xo.cho_phep(0.0));
        // Chờ 1 giây -> đổ lại 1 token -> cho đúng 1 lần
        assert!(xo.cho_phep(1.0));
        assert!(!xo.cho_phep(0.0));
    }

    #[test]
    fn token_bucket_khong_vuot_dung_luong() {
        let mut xo = XoToken::moi(2.0, 100.0);
        // chờ rất lâu nhưng token bị GHIM ở dung lượng, không tràn
        xo.cho_phep(1000.0);
        assert!(xo.token_con() <= 2.0);
    }

    #[test]
    fn back_pressure_tu_choi_khi_day() {
        let mut hq: HangDoiGioiHan<u32> = HangDoiGioiHan::moi(2);
        assert_eq!(hq.gui(1), KetQuaNhan::DaNhan);
        assert_eq!(hq.gui(2), KetQuaNhan::DaNhan);
        assert_eq!(hq.gui(3), KetQuaNhan::TuChoi); // đầy!
        assert_eq!(hq.so_da_tu_choi(), 1);
        // Lấy ra 1 -> có chỗ -> nhận lại được
        assert_eq!(hq.nhan(), Some(1));
        assert_eq!(hq.gui(3), KetQuaNhan::DaNhan);
    }
}
