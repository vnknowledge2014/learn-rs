#![allow(dead_code)]
//! Chương 84 — Giao dịch định lượng & Arbitrage thống kê bằng Rust: hồi quy
//! tuyến tính, tương quan, kiểm định đồng liên kết, lọc Kalman cho tỉ lệ phòng
//! hộ động, danh mục trung bình–phương sai, và các thước đo rủi ro đuôi.
//!
//! Chương cuối chuyển giáo trình *learn* của OpenAlgo sang Rust
//! (Quantitative Trading + Statistical Arbitrage + Risk Management).
//!
//! Thông điệp xuyên suốt: **thống kê trên dữ liệu tài chính rất dễ nói dối**.
//! Tương quan cao không có nghĩa quan hệ bền; kết quả đẹp trong mẫu không có
//! nghĩa chiến lược tốt. Mỗi công cụ ở đây đều đi kèm cách nó phản bội bạn.
//!
//! ⚠️ Tài liệu KỸ THUẬT, không phải lời khuyên đầu tư.

// ============================================================================
// 1. THỐNG KÊ NỀN
// ============================================================================

pub fn trung_binh(x: &[f64]) -> f64 {
    if x.is_empty() { return 0.0; }
    x.iter().sum::<f64>() / x.len() as f64
}

/// Phương sai MẪU (chia n−1). Dùng n−1 vì ta ước lượng trung bình từ chính
/// dữ liệu, nên mất một bậc tự do — chia n sẽ cho ước lượng thiên lệch thấp.
pub fn phuong_sai(x: &[f64]) -> f64 {
    if x.len() < 2 { return 0.0; }
    let tb = trung_binh(x);
    x.iter().map(|v| (v - tb).powi(2)).sum::<f64>() / (x.len() - 1) as f64
}

pub fn do_lech_chuan(x: &[f64]) -> f64 { phuong_sai(x).sqrt() }

pub fn hiep_phuong_sai(x: &[f64], y: &[f64]) -> f64 {
    let n = x.len().min(y.len());
    if n < 2 { return 0.0; }
    let (tx, ty) = (trung_binh(&x[..n]), trung_binh(&y[..n]));
    x[..n].iter().zip(y[..n].iter())
        .map(|(a, b)| (a - tx) * (b - ty)).sum::<f64>() / (n - 1) as f64
}

/// Hệ số tương quan Pearson, luôn nằm trong [−1, 1].
pub fn tuong_quan(x: &[f64], y: &[f64]) -> Option<f64> {
    let n = x.len().min(y.len());
    let (sx, sy) = (do_lech_chuan(&x[..n]), do_lech_chuan(&y[..n]));
    if sx < 1e-12 || sy < 1e-12 { return None; }
    Some((hiep_phuong_sai(x, y) / (sx * sy)).clamp(-1.0, 1.0))
}

// ============================================================================
// 2. HỒI QUY TUYẾN TÍNH
// ============================================================================

#[derive(Debug, Clone, Copy, PartialEq)]
pub struct KetQuaHoiQuy {
    /// Hệ số góc — trong tài chính gọi là beta, hay TỈ LỆ PHÒNG HỘ.
    pub beta: f64,
    /// Hệ số chặn — phần lợi suất không giải thích được bằng biến kia.
    pub alpha: f64,
    /// Tỉ lệ phương sai được giải thích, trong [0, 1].
    pub r_binh_phuong: f64,
    /// Độ lệch chuẩn của phần dư.
    pub sai_so_chuan: f64,
    pub so_quan_sat: usize,
}

/// Hồi quy bình phương tối thiểu: y = alpha + beta·x + nhiễu.
pub fn hoi_quy(x: &[f64], y: &[f64]) -> Option<KetQuaHoiQuy> {
    let n = x.len().min(y.len());
    if n < 3 { return None; }
    let vx = phuong_sai(&x[..n]);
    if vx < 1e-12 { return None; } // x không đổi thì không có hệ số góc

    let beta = hiep_phuong_sai(&x[..n], &y[..n]) / vx;
    let alpha = trung_binh(&y[..n]) - beta * trung_binh(&x[..n]);
    let du: Vec<f64> = (0..n).map(|i| y[i] - (alpha + beta * x[i])).collect();
    let vy = phuong_sai(&y[..n]);
    let r2 = if vy < 1e-12 { 0.0 } else { (1.0 - phuong_sai(&du) / vy).clamp(0.0, 1.0) };
    Some(KetQuaHoiQuy { beta, alpha, r_binh_phuong: r2,
                        sai_so_chuan: do_lech_chuan(&du), so_quan_sat: n })
}

/// Phần dư của hồi quy — chính là CHÊNH LỆCH mà arbitrage cặp giao dịch.
pub fn phan_du(x: &[f64], y: &[f64], kq: &KetQuaHoiQuy) -> Vec<f64> {
    let n = x.len().min(y.len());
    (0..n).map(|i| y[i] - (kq.alpha + kq.beta * x[i])).collect()
}

// ============================================================================
// 3. KIỂM ĐỊNH ĐỒNG LIÊN KẾT
// ============================================================================
// Hai chuỗi giá có thể tương quan cao mà KHÔNG đồng liên kết: chúng cùng đi
// lên nhưng chênh lệch giữa chúng ngày càng giãn. Giao dịch cặp trên quan hệ
// như vậy là thua chắc.
//
// Đồng liên kết nghĩa là chênh lệch QUAY VỀ trung bình. Ta kiểm bằng thống kê
// kiểu Dickey–Fuller: hồi quy Δe theo e; hệ số góc âm rõ rệt nghĩa là chênh
// lệch bị kéo về 0.

#[derive(Debug, Clone, Copy, PartialEq)]
pub struct KetQuaDongLienKet {
    /// Hệ số kéo về. Càng âm càng quay về trung bình nhanh.
    pub he_so_keo_ve: f64,
    /// Nửa chu kỳ: bao nhiêu bước để chênh lệch co lại một nửa.
    pub nua_chu_ky: f64,
    pub co_dong_lien_ket: bool,
}

pub fn kiem_dinh_dong_lien_ket(chenh_lech: &[f64], nguong: f64)
    -> Option<KetQuaDongLienKet>
{
    if chenh_lech.len() < 20 { return None; }
    let e: Vec<f64> = chenh_lech[..chenh_lech.len() - 1].to_vec();
    let de: Vec<f64> = chenh_lech.windows(2).map(|w| w[1] - w[0]).collect();
    let hq = hoi_quy(&e, &de)?;
    let lambda = hq.beta;
    // Chênh lệch co lại theo e^(λt); nửa chu kỳ là khi e^(λt) = 1/2
    let nua = if lambda < -1e-12 { (0.5f64).ln() / lambda } else { f64::INFINITY };
    Some(KetQuaDongLienKet {
        he_so_keo_ve: lambda,
        nua_chu_ky: nua,
        co_dong_lien_ket: lambda < nguong,
    })
}

// ============================================================================
// 4. LỌC KALMAN CHO TỈ LỆ PHÒNG HỘ ĐỘNG
// ============================================================================
// Hồi quy cho MỘT beta cố định cho cả giai đoạn. Nhưng quan hệ giữa hai mã
// trôi theo thời gian. Lọc Kalman cập nhật beta sau MỖI quan sát, cân bằng
// giữa "tin dữ liệu mới" và "tin ước lượng cũ".

#[derive(Debug, Clone, Copy, PartialEq)]
pub struct LocKalman {
    /// Ước lượng beta hiện tại.
    pub beta: f64,
    /// Độ bất định của ước lượng. Càng lớn càng sẵn sàng đổi ý.
    pub phuong_sai_uoc_luong: f64,
    /// Mức trôi của beta giữa hai bước (nhiễu quá trình).
    pub nhieu_qua_trinh: f64,
    /// Mức nhiễu của quan sát. Càng lớn càng ít tin dữ liệu mới.
    pub nhieu_quan_sat: f64,
    pub so_buoc: usize,
}

impl LocKalman {
    pub fn moi(beta_dau: f64, nhieu_qua_trinh: f64, nhieu_quan_sat: f64) -> Self {
        LocKalman { beta: beta_dau, phuong_sai_uoc_luong: 1.0,
                    nhieu_qua_trinh, nhieu_quan_sat, so_buoc: 0 }
    }

    /// Cập nhật với một cặp quan sát (x, y). Trả về sai số dự báo — chính là
    /// tín hiệu giao dịch: y lệch bao nhiêu so với mức beta·x dự đoán.
    pub fn cap_nhat(&mut self, x: f64, y: f64) -> f64 {
        // Dự đoán: beta không đổi, nhưng độ bất định lớn thêm
        let p_truoc = self.phuong_sai_uoc_luong + self.nhieu_qua_trinh;
        // Sai số dự báo
        let sai_so = y - self.beta * x;
        // Độ lợi Kalman: dữ liệu mới càng đáng tin thì càng gần 1
        let s = x * x * p_truoc + self.nhieu_quan_sat;
        let k = if s.abs() < 1e-12 { 0.0 } else { p_truoc * x / s };
        self.beta += k * sai_so;
        self.phuong_sai_uoc_luong = (1.0 - k * x) * p_truoc;
        self.so_buoc += 1;
        sai_so
    }
}

// ============================================================================
// 5. DANH MỤC TRUNG BÌNH – PHƯƠNG SAI
// ============================================================================

#[derive(Debug, Clone, Copy, PartialEq)]
pub struct ThongKeDanhMuc {
    pub loi_suat_ky_vong: f64,
    pub do_lech_chuan: f64,
    /// Lợi suất trên mỗi đơn vị rủi ro.
    pub ty_so_sharpe: f64,
}

/// Rủi ro của danh mục KHÔNG phải trung bình rủi ro các thành phần — nó phụ
/// thuộc tương quan. Đây là toàn bộ ý nghĩa của đa dạng hoá, và là "bữa trưa
/// miễn phí" duy nhất trong tài chính.
pub fn thong_ke_danh_muc(loi_suat: &[Vec<f64>], trong_so: &[f64], phi_rui_ro: f64)
    -> Option<ThongKeDanhMuc>
{
    let n = loi_suat.len();
    if n == 0 || trong_so.len() != n { return None; }
    let ls_ky_vong: f64 = (0..n).map(|i| trong_so[i] * trung_binh(&loi_suat[i])).sum();

    // Phương sai danh mục = Σᵢ Σⱼ wᵢ wⱼ Cov(i, j)
    let mut ps = 0.0;
    for i in 0..n {
        for j in 0..n {
            ps += trong_so[i] * trong_so[j] * hiep_phuong_sai(&loi_suat[i], &loi_suat[j]);
        }
    }
    let sd = ps.max(0.0).sqrt();
    Some(ThongKeDanhMuc {
        loi_suat_ky_vong: ls_ky_vong,
        do_lech_chuan: sd,
        ty_so_sharpe: if sd < 1e-12 { 0.0 } else { (ls_ky_vong - phi_rui_ro) / sd },
    })
}

// ============================================================================
// 6. RỦI RO ĐUÔI
// ============================================================================

/// Giá trị chịu rủi ro theo phân vị lịch sử: mức lỗ mà `(1−p)` phần trăm số
/// phiên KHÔNG vượt qua. Trả về số DƯƠNG biểu thị mức lỗ.
///
/// Khuyết điểm chí mạng: nó nói "bạn sẽ không lỗ quá X trong 95% thời gian",
/// nhưng KHÔNG nói gì về 5% còn lại. Mà 5% đó mới là chỗ phá sản.
pub fn gia_tri_chiu_rui_ro(loi_suat: &[f64], muc_tin_cay: f64) -> Option<f64> {
    if loi_suat.is_empty() || !(0.0..1.0).contains(&muc_tin_cay) { return None; }
    let mut s = loi_suat.to_vec();
    s.sort_by(|a, b| a.partial_cmp(b).unwrap());
    let i = (((1.0 - muc_tin_cay) * s.len() as f64).floor() as usize).min(s.len() - 1);
    Some(-s[i])
}

/// Thiếu hụt kỳ vọng: lỗ TRUNG BÌNH trong những phiên tệ nhất.
/// Đây là câu trả lời cho câu hỏi mà VaR né tránh: "khi vượt ngưỡng thì tệ
/// tới mức nào?" Nó luôn ≥ VaR, và là thước đo mà quy định hiện đại dùng.
pub fn thieu_hut_ky_vong(loi_suat: &[f64], muc_tin_cay: f64) -> Option<f64> {
    if loi_suat.is_empty() || !(0.0..1.0).contains(&muc_tin_cay) { return None; }
    let mut s = loi_suat.to_vec();
    s.sort_by(|a, b| a.partial_cmp(b).unwrap());
    let k = (((1.0 - muc_tin_cay) * s.len() as f64).ceil() as usize).clamp(1, s.len());
    Some(-trung_binh(&s[..k]))
}

// ============================================================================
// 7. KIỂM ĐỊNH TIẾN — chống khớp quá mức
// ============================================================================
// Tối ưu tham số trên toàn bộ dữ liệu rồi khoe kết quả là tự lừa mình. Kiểm
// định tiến chia dữ liệu thành nhiều đoạn: chọn tham số trên đoạn TRONG MẪU,
// rồi chấm điểm trên đoạn NGOÀI MẪU ngay sau đó — mô phỏng đúng cách ta thật
// sự giao dịch: chỉ biết quá khứ.

#[derive(Debug, Clone, PartialEq)]
pub struct DoanKiemDinh {
    pub tham_so_chon: usize,
    pub diem_trong_mau: f64,
    pub diem_ngoai_mau: f64,
}

#[derive(Debug, PartialEq)]
pub struct KetQuaKiemDinhTien {
    pub cac_doan: Vec<DoanKiemDinh>,
    pub trung_binh_trong_mau: f64,
    pub trung_binh_ngoai_mau: f64,
    /// Mức tụt điểm khi ra ngoài mẫu. Tụt nhiều = đã khớp vào nhiễu.
    pub muc_sut_giam: f64,
}

/// `cham_diem(tham_so, tu, den)` chấm điểm một tham số trên đoạn `[tu, den)`.
pub fn kiem_dinh_tien<F>(
    tong_do_dai: usize, do_dai_trong_mau: usize, do_dai_ngoai_mau: usize,
    cac_tham_so: &[usize], mut cham_diem: F,
) -> KetQuaKiemDinhTien
where F: FnMut(usize, usize, usize) -> f64
{
    let mut cac_doan = Vec::new();
    if cac_tham_so.is_empty() || do_dai_ngoai_mau == 0 {
        return KetQuaKiemDinhTien { cac_doan, trung_binh_trong_mau: 0.0,
                                    trung_binh_ngoai_mau: 0.0, muc_sut_giam: 0.0 };
    }
    let mut dau = 0usize;
    while dau + do_dai_trong_mau + do_dai_ngoai_mau <= tong_do_dai {
        let het_trong = dau + do_dai_trong_mau;
        let het_ngoai = het_trong + do_dai_ngoai_mau;
        // Chọn tham số CHỈ dựa trên đoạn trong mẫu
        let (tot_nhat, diem_trong) = cac_tham_so.iter()
            .map(|&p| (p, cham_diem(p, dau, het_trong)))
            .fold((cac_tham_so[0], f64::MIN), |a, b| if b.1 > a.1 { b } else { a });
        // Rồi chấm nó trên đoạn ngoài mẫu ngay sau
        let diem_ngoai = cham_diem(tot_nhat, het_trong, het_ngoai);
        cac_doan.push(DoanKiemDinh { tham_so_chon: tot_nhat,
                                     diem_trong_mau: diem_trong,
                                     diem_ngoai_mau: diem_ngoai });
        dau += do_dai_ngoai_mau;
    }
    let tb_trong = trung_binh(&cac_doan.iter().map(|d| d.diem_trong_mau)
                                       .collect::<Vec<_>>());
    let tb_ngoai = trung_binh(&cac_doan.iter().map(|d| d.diem_ngoai_mau)
                                       .collect::<Vec<_>>());
    KetQuaKiemDinhTien {
        cac_doan, trung_binh_trong_mau: tb_trong, trung_binh_ngoai_mau: tb_ngoai,
        muc_sut_giam: tb_trong - tb_ngoai,
    }
}

/// Nhiễu tất định trải đều trong [−1, 1), băm từ (đoạn, tham số).
/// Dùng splitmix64 thay vì số học modulo thô: modulo thô làm các giá trị co
/// cụm, và khi đó "chọn tối đa" không còn thật sự khớp vào nhiễu nữa.
pub fn nhieu_tat_dinh(doan: usize, tham_so: usize) -> f64 {
    let mut z = (doan as u64)
        .wrapping_mul(0x9E37_79B9_7F4A_7C15)
        .wrapping_add((tham_so as u64).wrapping_mul(0xBF58_476D_1CE4_E5B9));
    z = (z ^ (z >> 30)).wrapping_mul(0xBF58_476D_1CE4_E5B9);
    z = (z ^ (z >> 27)).wrapping_mul(0x94D0_49BB_1331_11EB);
    z ^= z >> 31;
    ((z >> 40) as f64 / 8_388_608.0) - 1.0
}

// ============================================================================
// 8. SINH DỮ LIỆU TẤT ĐỊNH
// ============================================================================

/// Hai chuỗi ĐỒNG LIÊN KẾT: cùng theo một nhân tố chung, chênh lệch quay về 0.
pub fn sinh_cap_dong_lien_ket(n: usize, hat_giong: u64, beta: f64)
    -> (Vec<f64>, Vec<f64>)
{
    let mut s = hat_giong;
    let mut chung = 100.0f64;
    let mut chenh = 0.0f64;
    let (mut a, mut b) = (Vec::with_capacity(n), Vec::with_capacity(n));
    for _ in 0..n {
        s = s.wrapping_mul(6364136223846793005).wrapping_add(1442695040888963407);
        let e1 = ((s >> 33) % 201) as f64 / 100.0 - 1.0;
        let e2 = ((s >> 45) % 201) as f64 / 100.0 - 1.0;
        chung += e1 * 0.5;
        // Chênh lệch quay về trung bình: kéo 20% về 0 mỗi bước
        chenh = chenh * 0.8 + e2 * 0.5;
        a.push(chung);
        b.push(beta * chung + chenh);
    }
    (a, b)
}

/// Hai chuỗi tương quan cao nhưng KHÔNG đồng liên kết: cả hai cùng đi lên,
/// nhưng chênh lệch tự nó cũng là bước ngẫu nhiên và giãn mãi.
pub fn sinh_cap_gia_dong_lien_ket(n: usize, hat_giong: u64) -> (Vec<f64>, Vec<f64>) {
    let mut s = hat_giong;
    let mut chung = 100.0f64;
    let mut troi = 0.0f64;
    let (mut a, mut b) = (Vec::with_capacity(n), Vec::with_capacity(n));
    for _ in 0..n {
        s = s.wrapping_mul(6364136223846793005).wrapping_add(1442695040888963407);
        let e1 = ((s >> 33) % 201) as f64 / 100.0 - 1.0;
        let e2 = ((s >> 45) % 201) as f64 / 100.0 - 1.0;
        chung += e1 * 0.5;
        troi += e2 * 0.3; // KHÔNG có lực kéo về — nó đi lang thang mãi
        a.push(chung);
        b.push(chung + troi);
    }
    (a, b)
}

pub fn sinh_loi_suat(n: usize, hat_giong: u64, do_lech: f64, ky_vong: f64) -> Vec<f64> {
    let mut s = hat_giong;
    (0..n).map(|_| {
        s = s.wrapping_mul(6364136223846793005).wrapping_add(1442695040888963407);
        // Tổng 3 biến đều → xấp xỉ phân phối chuẩn (định lý giới hạn trung tâm)
        let u: f64 = (0..3).map(|k| ((s >> (20 + k * 12)) % 1000) as f64 / 1000.0).sum();
        ky_vong + (u - 1.5) * do_lech
    }).collect()
}

fn main() {
    println!("═══════════════════════════════════════════════════════════");
    println!("   GIAO DỊCH ĐỊNH LƯỢNG & ARBITRAGE THỐNG KÊ (OpenAlgo)     ");
    println!("═══════════════════════════════════════════════════════════");

    println!("\n1. HỒI QUY & TỈ LỆ PHÒNG HỘ");
    let (a, b) = sinh_cap_dong_lien_ket(1_000, 2024, 1.5);
    let hq = hoi_quy(&a, &b).unwrap();
    println!("   beta {:.4} (đúng phải là 1.5) · alpha {:.4} · R² {:.4}",
             hq.beta, hq.alpha, hq.r_binh_phuong);
    println!("   Tương quan: {:.4}", tuong_quan(&a, &b).unwrap());
    println!("   → beta chính là số lượng mã B cần bán khi mua 1 mã A để trung hoà.");

    println!("\n2. TƯƠNG QUAN CAO KHÔNG BẰNG ĐỒNG LIÊN KẾT");
    let (c, d) = sinh_cap_gia_dong_lien_ket(1_000, 7);
    println!("   {:<24} {:>12} {:>16} {:>14}",
             "cặp", "tương quan", "hệ số kéo về", "nửa chu kỳ");
    for (ten, x, y) in [("đồng liên kết thật", &a, &b),
                        ("chỉ tương quan cao", &c, &d)] {
        let h = hoi_quy(x, y).unwrap();
        let e = phan_du(x, y, &h);
        let dlk = kiem_dinh_dong_lien_ket(&e, -0.05).unwrap();
        println!("   {:<24} {:>12.4} {:>16.4} {:>14.1}",
                 ten, tuong_quan(x, y).unwrap(), dlk.he_so_keo_ve, dlk.nua_chu_ky);
    }
    println!("   → CẢ HAI đều tương quan rất cao. Nhưng chỉ cặp đầu có chênh lệch");
    println!("     quay về trung bình. Giao dịch cặp thứ hai là thua chắc.");

    println!("\n3. LỌC KALMAN — beta trôi theo thời gian");
    let mut lk = LocKalman::moi(1.0, 1e-5, 1.0);
    println!("   {:>10} {:>16}", "bước", "beta ước lượng");
    for (i, (&x, &y)) in a.iter().zip(b.iter()).enumerate() {
        lk.cap_nhat(x, y);
        if [0usize, 10, 50, 200, 999].contains(&i) {
            println!("   {:>10} {:>16.4}", i, lk.beta);
        }
    }
    println!("   → Xuất phát từ 1.0 và tự tìm về {:.3} mà không cần biết trước.", lk.beta);

    println!("\n4. ĐA DẠNG HOÁ — bữa trưa miễn phí duy nhất");
    let ls_a = sinh_loi_suat(1_000, 1, 0.02, 0.0005);
    let ls_b = sinh_loi_suat(1_000, 999, 0.02, 0.0005);
    let mot_ma = thong_ke_danh_muc(&[ls_a.clone()], &[1.0], 0.0).unwrap();
    let hai_ma = thong_ke_danh_muc(&[ls_a.clone(), ls_b.clone()], &[0.5, 0.5], 0.0).unwrap();
    println!("   Chỉ mã A    : lợi suất {:.5} · rủi ro {:.5} · Sharpe {:.3}",
             mot_ma.loi_suat_ky_vong, mot_ma.do_lech_chuan, mot_ma.ty_so_sharpe);
    println!("   Nửa A nửa B : lợi suất {:.5} · rủi ro {:.5} · Sharpe {:.3}",
             hai_ma.loi_suat_ky_vong, hai_ma.do_lech_chuan, hai_ma.ty_so_sharpe);
    println!("   → Lợi suất kỳ vọng gần như không đổi, nhưng rủi ro giảm {:.0}%.",
             (1.0 - hai_ma.do_lech_chuan / mot_ma.do_lech_chuan) * 100.0);
    println!("     Đó là vì hai mã không tương quan hoàn toàn.");

    println!("\n5. RỦI RO ĐUÔI");
    let ls = sinh_loi_suat(5_000, 42, 0.02, 0.0003);
    println!("   {:>14} {:>14} {:>22}", "mức tin cậy", "VaR", "thiếu hụt kỳ vọng");
    for mtc in [0.90f64, 0.95, 0.99] {
        println!("   {:>13.0}% {:>14.5} {:>22.5}", mtc * 100.0,
                 gia_tri_chiu_rui_ro(&ls, mtc).unwrap(),
                 thieu_hut_ky_vong(&ls, mtc).unwrap());
    }
    println!("   → Thiếu hụt kỳ vọng LUÔN lớn hơn VaR. VaR nói \"95% thời gian bạn");
    println!("     không lỗ quá X\"; nó im lặng về 5% còn lại — mà đó mới là chỗ chết.");

    println!("\n6. KIỂM ĐỊNH TIẾN — phát hiện khớp quá mức");
    // Hàm chấm điểm giả: có một tham số "thật sự tốt" (20) cộng nhiễu phụ
    // thuộc đoạn dữ liệu. Tối ưu trên nhiễu chính là khớp quá mức.
    let cham = |p: usize, tu: usize, _den: usize| -> f64 {
        let nen = if p == 20 { 1.0 } else { 0.3 };
        nen + nhieu_tat_dinh(tu, p) * 0.8
    };
    let kq = kiem_dinh_tien(1_000, 200, 100, &[5, 10, 20, 50, 100], cham);
    println!("   {:>8} {:>16} {:>18} {:>18}",
             "đoạn", "tham số chọn", "điểm trong mẫu", "điểm ngoài mẫu");
    for (i, d) in kq.cac_doan.iter().enumerate() {
        println!("   {:>8} {:>16} {:>18.3} {:>18.3}",
                 i + 1, d.tham_so_chon, d.diem_trong_mau, d.diem_ngoai_mau);
    }
    println!("   Trung bình trong mẫu {:.3} · ngoài mẫu {:.3} · SỤT {:.3}",
             kq.trung_binh_trong_mau, kq.trung_binh_ngoai_mau, kq.muc_sut_giam);
    println!("   → Điểm trong mẫu luôn đẹp hơn, vì ta ĐÃ CHỌN tham số cho nó.");
    println!("     Chỉ điểm ngoài mẫu mới là con số đáng tin.");

    println!("\n═══════════════════════════════════════════════════════════");
    println!("   THỐNG KÊ TÀI CHÍNH DỄ NÓI DỐI. LUÔN HỎI: CÒN NGOÀI MẪU?  ");
    println!("═══════════════════════════════════════════════════════════");
}

#[cfg(test)]
mod kiem_thu {
    use super::*;

    // ---------- Thống kê nền ----------
    #[test]
    fn thong_ke_co_ban_dung() {
        let x = vec![2.0, 4.0, 4.0, 4.0, 5.0, 5.0, 7.0, 9.0];
        assert!((trung_binh(&x) - 5.0).abs() < 1e-12);
        // Phương sai MẪU (chia n−1) của dãy này là 32/7
        assert!((phuong_sai(&x) - 32.0 / 7.0).abs() < 1e-12);
        assert!((do_lech_chuan(&x) - (32.0f64 / 7.0).sqrt()).abs() < 1e-12);
    }

    #[test]
    fn thong_ke_du_lieu_qua_it_khong_panic() {
        assert_eq!(trung_binh(&[]), 0.0);
        assert_eq!(phuong_sai(&[]), 0.0);
        assert_eq!(phuong_sai(&[5.0]), 0.0, "một điểm thì không có phương sai mẫu");
        assert_eq!(hiep_phuong_sai(&[1.0], &[2.0]), 0.0);
    }

    #[test]
    fn tuong_quan_bang_1_khi_quan_he_tuyen_tinh_hoan_hao() {
        let x: Vec<f64> = (1..=100).map(|i| i as f64).collect();
        let tang: Vec<f64> = x.iter().map(|v| 3.0 * v + 7.0).collect();
        let giam: Vec<f64> = x.iter().map(|v| -2.0 * v + 5.0).collect();
        assert!((tuong_quan(&x, &tang).unwrap() - 1.0).abs() < 1e-9);
        assert!((tuong_quan(&x, &giam).unwrap() + 1.0).abs() < 1e-9);
    }

    #[test]
    fn tuong_quan_luon_trong_khoang_am_mot_den_mot() {
        for hat in [1u64, 42, 2024] {
            let a = sinh_loi_suat(500, hat, 0.02, 0.0);
            let b = sinh_loi_suat(500, hat + 1000, 0.02, 0.0);
            let r = tuong_quan(&a, &b).unwrap();
            assert!((-1.0..=1.0).contains(&r), "tương quan {} ra ngoài khoảng", r);
        }
    }

    #[test]
    fn chuoi_khong_doi_thi_tuong_quan_khong_dinh_nghia_duoc() {
        let hang = vec![5.0; 100];
        let x: Vec<f64> = (1..=100).map(|i| i as f64).collect();
        assert_eq!(tuong_quan(&hang, &x), None, "không chia cho độ lệch bằng 0");
    }

    // ---------- Hồi quy ----------
    #[test]
    fn hoi_quy_tim_dung_he_so_khi_khong_co_nhieu() {
        let x: Vec<f64> = (1..=100).map(|i| i as f64).collect();
        let y: Vec<f64> = x.iter().map(|v| 2.5 * v + 10.0).collect();
        let h = hoi_quy(&x, &y).unwrap();
        assert!((h.beta - 2.5).abs() < 1e-9);
        assert!((h.alpha - 10.0).abs() < 1e-9);
        assert!((h.r_binh_phuong - 1.0).abs() < 1e-9, "khớp hoàn hảo → R² = 1");
        assert!(h.sai_so_chuan < 1e-9);
    }

    #[test]
    fn r_binh_phuong_luon_trong_khoang_0_1() {
        for hat in [1u64, 7, 42, 2024] {
            let (a, b) = sinh_cap_dong_lien_ket(500, hat, 1.5);
            let h = hoi_quy(&a, &b).unwrap();
            assert!((0.0..=1.0).contains(&h.r_binh_phuong));
        }
    }

    #[test]
    fn hoi_quy_tra_none_khi_khong_du_dieu_kien() {
        assert_eq!(hoi_quy(&[1.0, 2.0], &[1.0, 2.0]), None, "cần ít nhất 3 điểm");
        assert_eq!(hoi_quy(&[5.0; 10], &[1.0; 10]), None, "x không đổi thì vô nghĩa");
    }

    #[test]
    fn phan_du_co_trung_binh_bang_khong() {
        // Tính chất toán học của bình phương tối thiểu. Nếu không đúng thì
        // hồi quy đã cài sai.
        let (a, b) = sinh_cap_dong_lien_ket(500, 11, 1.5);
        let h = hoi_quy(&a, &b).unwrap();
        let e = phan_du(&a, &b, &h);
        assert!(trung_binh(&e).abs() < 1e-9,
                "trung bình phần dư {:.2e}", trung_binh(&e));
    }

    #[test]
    fn phan_du_khong_con_tuong_quan_voi_bien_giai_thich() {
        // Tính chất thứ hai: phần dư trực giao với biến giải thích. Nếu còn
        // tương quan thì vẫn còn thông tin chưa khai thác hết.
        let (a, b) = sinh_cap_dong_lien_ket(500, 13, 1.5);
        let h = hoi_quy(&a, &b).unwrap();
        let e = phan_du(&a, &b, &h);
        let r = tuong_quan(&a, &e).unwrap();
        assert!(r.abs() < 1e-9, "phần dư còn tương quan {:.2e} với x", r);
    }

    // ---------- Đồng liên kết ----------
    #[test]
    fn phat_hien_dung_cap_dong_lien_ket() {
        let (a, b) = sinh_cap_dong_lien_ket(1_000, 2024, 1.5);
        let h = hoi_quy(&a, &b).unwrap();
        let e = phan_du(&a, &b, &h);
        let k = kiem_dinh_dong_lien_ket(&e, -0.05).unwrap();
        assert!(k.co_dong_lien_ket, "hệ số kéo về {:.4} phải đủ âm", k.he_so_keo_ve);
        assert!(k.he_so_keo_ve < 0.0);
        assert!(k.nua_chu_ky.is_finite() && k.nua_chu_ky > 0.0,
                "nửa chu kỳ {:.2} phải hữu hạn và dương", k.nua_chu_ky);
    }

    #[test]
    fn tu_choi_cap_chi_tuong_quan_cao_ma_khong_dong_lien_ket() {
        // BÀI HỌC TRUNG TÂM: tương quan gần 1 nhưng chênh lệch giãn mãi.
        let (c, d) = sinh_cap_gia_dong_lien_ket(1_000, 7);
        let r = tuong_quan(&c, &d).unwrap();
        assert!(r > 0.8, "hai chuỗi này TƯƠNG QUAN rất cao: {:.3}", r);
        let h = hoi_quy(&c, &d).unwrap();
        let e = phan_du(&c, &d, &h);
        let k = kiem_dinh_dong_lien_ket(&e, -0.05).unwrap();
        assert!(!k.co_dong_lien_ket,
                "nhưng KHÔNG đồng liên kết: hệ số kéo về chỉ {:.4}", k.he_so_keo_ve);
    }

    #[test]
    fn keo_ve_cang_manh_thi_nua_chu_ky_cang_ngan() {
        let sinh_chenh = |he_so: f64| -> Vec<f64> {
            let mut s = 7u64;
            let mut e = 10.0f64;
            let mut v = Vec::new();
            for _ in 0..500 {
                s = s.wrapping_mul(6364136223846793005).wrapping_add(1);
                let n = ((s >> 33) % 101) as f64 / 100.0 - 0.5;
                e = e * he_so + n;
                v.push(e);
            }
            v
        };
        let a = kiem_dinh_dong_lien_ket(&sinh_chenh(0.5), -0.05).unwrap();
        let b = kiem_dinh_dong_lien_ket(&sinh_chenh(0.95), -0.05).unwrap();
        assert!(a.nua_chu_ky < b.nua_chu_ky,
                "kéo mạnh nửa chu kỳ {:.2} phải ngắn hơn kéo yếu {:.2}",
                a.nua_chu_ky, b.nua_chu_ky);
    }

    #[test]
    fn du_lieu_qua_ngan_thi_khong_kiem_dinh_duoc() {
        assert_eq!(kiem_dinh_dong_lien_ket(&[1.0; 10], -0.05), None);
    }

    // ---------- Kalman ----------
    #[test]
    fn kalman_hoi_tu_ve_beta_that() {
        let beta_that = 1.5;
        let (a, b) = sinh_cap_dong_lien_ket(2_000, 2024, beta_that);
        let mut lk = LocKalman::moi(1.0, 1e-5, 1.0);
        for (&x, &y) in a.iter().zip(b.iter()) { lk.cap_nhat(x, y); }
        assert!((lk.beta - beta_that).abs() < 0.15,
                "Kalman hội tụ về {:.4}, kỳ vọng {:.2}", lk.beta, beta_that);
    }

    #[test]
    fn kalman_bot_bat_dinh_khi_co_them_du_lieu() {
        let (a, b) = sinh_cap_dong_lien_ket(500, 5, 1.5);
        let mut lk = LocKalman::moi(1.0, 1e-6, 1.0);
        let dau = lk.phuong_sai_uoc_luong;
        for (&x, &y) in a.iter().zip(b.iter()).take(200) { lk.cap_nhat(x, y); }
        assert!(lk.phuong_sai_uoc_luong < dau,
                "càng nhiều dữ liệu thì càng tự tin: {:.2e} so với {:.2e}",
                lk.phuong_sai_uoc_luong, dau);
        assert_eq!(lk.so_buoc, 200);
    }

    #[test]
    fn kalman_khong_panic_voi_x_bang_khong() {
        let mut lk = LocKalman::moi(1.0, 1e-5, 0.0);
        let e = lk.cap_nhat(0.0, 5.0);
        assert!(e.is_finite());
        assert!(lk.beta.is_finite(), "beta phải hữu hạn, không được thành NaN");
    }

    // ---------- Danh mục ----------
    #[test]
    fn da_dang_hoa_giam_rui_ro_khi_hai_ma_khong_tuong_quan_hoan_toan() {
        // "Bữa trưa miễn phí" duy nhất trong tài chính.
        let a = sinh_loi_suat(1_000, 1, 0.02, 0.0005);
        let b = sinh_loi_suat(1_000, 999, 0.02, 0.0005);
        let mot = thong_ke_danh_muc(&[a.clone()], &[1.0], 0.0).unwrap();
        let hai = thong_ke_danh_muc(&[a, b], &[0.5, 0.5], 0.0).unwrap();
        assert!(hai.do_lech_chuan < mot.do_lech_chuan,
                "rủi ro danh mục {:.6} phải nhỏ hơn một mã {:.6}",
                hai.do_lech_chuan, mot.do_lech_chuan);
        assert!(hai.ty_so_sharpe > mot.ty_so_sharpe, "và Sharpe phải cao hơn");
    }

    #[test]
    fn hai_ma_giong_het_nhau_thi_khong_da_dang_hoa_duoc() {
        // Đa dạng hoá giả: mua hai mã y hệt nhau chẳng giảm rủi ro chút nào.
        let a = sinh_loi_suat(500, 1, 0.02, 0.0);
        let mot = thong_ke_danh_muc(&[a.clone()], &[1.0], 0.0).unwrap();
        let hai = thong_ke_danh_muc(&[a.clone(), a], &[0.5, 0.5], 0.0).unwrap();
        assert!((hai.do_lech_chuan - mot.do_lech_chuan).abs() < 1e-9,
                "cùng một mã thì rủi ro không đổi");
    }

    #[test]
    fn danh_muc_tham_so_sai_tra_none() {
        let a = sinh_loi_suat(100, 1, 0.02, 0.0);
        assert_eq!(thong_ke_danh_muc(&[], &[], 0.0), None);
        assert_eq!(thong_ke_danh_muc(&[a], &[0.5, 0.5], 0.0), None,
                   "số trọng số phải khớp số tài sản");
    }

    #[test]
    fn phuong_sai_danh_muc_khong_bao_gio_am() {
        for hat in [1u64, 42, 2024] {
            let a = sinh_loi_suat(300, hat, 0.02, 0.0);
            let b = sinh_loi_suat(300, hat + 7, 0.03, 0.0);
            let d = thong_ke_danh_muc(&[a, b], &[0.7, 0.3], 0.0).unwrap();
            assert!(d.do_lech_chuan >= 0.0);
        }
    }

    // ---------- Rủi ro đuôi ----------
    #[test]
    fn thieu_hut_ky_vong_luon_lon_hon_hoac_bang_var() {
        // Bất biến toán học: trung bình phần đuôi luôn tệ hơn ngưỡng đuôi.
        for hat in [1u64, 42, 2024] {
            let ls = sinh_loi_suat(2_000, hat, 0.02, 0.0);
            for mtc in [0.90f64, 0.95, 0.99] {
                let var = gia_tri_chiu_rui_ro(&ls, mtc).unwrap();
                let es = thieu_hut_ky_vong(&ls, mtc).unwrap();
                assert!(es >= var - 1e-9,
                        "thiếu hụt {:.6} phải ≥ VaR {:.6} tại {}", es, var, mtc);
            }
        }
    }

    #[test]
    fn var_tang_theo_muc_tin_cay() {
        let ls = sinh_loi_suat(2_000, 42, 0.02, 0.0);
        let mut truoc = f64::MIN;
        for mtc in [0.80f64, 0.90, 0.95, 0.99] {
            let v = gia_tri_chiu_rui_ro(&ls, mtc).unwrap();
            assert!(v >= truoc, "mức tin cậy cao hơn phải cho VaR lớn hơn");
            truoc = v;
        }
    }

    #[test]
    fn var_dau_vao_xau_tra_none() {
        assert_eq!(gia_tri_chiu_rui_ro(&[], 0.95), None);
        assert_eq!(gia_tri_chiu_rui_ro(&[1.0], 1.5), None);
        assert_eq!(thieu_hut_ky_vong(&[], 0.95), None);
        assert_eq!(thieu_hut_ky_vong(&[1.0], -0.1), None);
    }

    #[test]
    fn var_cua_chuoi_khong_doi_bang_chinh_gia_tri_do() {
        let ls = vec![-0.01; 100];
        assert!((gia_tri_chiu_rui_ro(&ls, 0.95).unwrap() - 0.01).abs() < 1e-12);
        assert!((thieu_hut_ky_vong(&ls, 0.95).unwrap() - 0.01).abs() < 1e-12);
    }

    // ---------- Kiểm định tiến ----------
    #[test]
    fn kiem_dinh_tien_chia_dung_so_doan() {
        let cham = |_p: usize, _tu: usize, _den: usize| 1.0;
        let kq = kiem_dinh_tien(1_000, 200, 100, &[5, 10, 20], cham);
        // Cửa sổ trượt 100 mỗi bước, cần 300 để đủ một đoạn → 8 đoạn
        assert_eq!(kq.cac_doan.len(), 8);
    }

    #[test]
    fn diem_ngoai_mau_thap_hon_trong_mau_khi_co_khop_qua_muc() {
        // Chấm điểm có nhiễu phụ thuộc đoạn: chọn tham số theo nhiễu chính
        // là khớp quá mức, và điểm ngoài mẫu sẽ tụt.
        let cham = |p: usize, tu: usize, _den: usize| -> f64 {
            let nen = if p == 20 { 1.0 } else { 0.3 };
            nen + nhieu_tat_dinh(tu, p) * 0.8
        };
        let kq = kiem_dinh_tien(2_000, 200, 100, &[5, 10, 20, 50, 100], cham);
        assert!(kq.muc_sut_giam > 0.0,
                "điểm phải TỤT khi ra ngoài mẫu: trong {:.3} ngoài {:.3}",
                kq.trung_binh_trong_mau, kq.trung_binh_ngoai_mau);
    }

    #[test]
    fn khong_co_nhieu_thi_khong_sut_giam() {
        // Nếu tham số thật sự tốt (không phải khớp nhiễu), điểm ngoài mẫu
        // bằng điểm trong mẫu.
        let cham = |p: usize, _tu: usize, _den: usize| if p == 20 { 1.0 } else { 0.3 };
        let kq = kiem_dinh_tien(1_000, 200, 100, &[5, 10, 20, 50], cham);
        assert!(kq.muc_sut_giam.abs() < 1e-9, "sụt {:.6}", kq.muc_sut_giam);
        assert!(kq.cac_doan.iter().all(|d| d.tham_so_chon == 20),
                "phải luôn chọn đúng tham số tốt thật");
    }

    #[test]
    fn du_lieu_qua_ngan_thi_khong_co_doan_nao() {
        let cham = |_p: usize, _tu: usize, _den: usize| 1.0;
        let kq = kiem_dinh_tien(100, 200, 100, &[5], cham);
        assert!(kq.cac_doan.is_empty());
        assert_eq!(kq.muc_sut_giam, 0.0);
    }

    #[test]
    fn khong_co_tham_so_nao_thi_khong_panic() {
        let cham = |_p: usize, _tu: usize, _den: usize| 1.0;
        let kq = kiem_dinh_tien(1_000, 200, 100, &[], cham);
        assert!(kq.cac_doan.is_empty());
    }

    #[test]
    fn nhieu_tat_dinh_trai_deu_va_lap_lai_duoc() {
        assert_eq!(nhieu_tat_dinh(100, 20), nhieu_tat_dinh(100, 20), "phải tất định");
        assert_ne!(nhieu_tat_dinh(100, 20), nhieu_tat_dinh(200, 20));
        assert_ne!(nhieu_tat_dinh(100, 20), nhieu_tat_dinh(100, 50));
        let mau: Vec<f64> = (0..2_000).map(|i| nhieu_tat_dinh(i, i * 7 % 13)).collect();
        for &x in &mau { assert!((-1.0..1.0).contains(&x), "giá trị {} ra ngoài khoảng", x); }
        let tb = trung_binh(&mau);
        assert!(tb.abs() < 0.1, "trung bình {:.4} phải gần 0", tb);
        assert!(do_lech_chuan(&mau) > 0.4, "phải trải đều, không co cụm");
    }

    // ---------- Sinh dữ liệu ----------
    #[test]
    fn sinh_du_lieu_tat_dinh() {
        assert_eq!(sinh_cap_dong_lien_ket(100, 5, 1.5),
                   sinh_cap_dong_lien_ket(100, 5, 1.5));
        assert_ne!(sinh_cap_dong_lien_ket(100, 5, 1.5),
                   sinh_cap_dong_lien_ket(100, 6, 1.5));
        assert_eq!(sinh_loi_suat(100, 1, 0.02, 0.0), sinh_loi_suat(100, 1, 0.02, 0.0));
    }

    #[test]
    fn cap_dong_lien_ket_sinh_ra_dung_beta() {
        for beta in [1.0f64, 1.5, 2.5] {
            let (a, b) = sinh_cap_dong_lien_ket(2_000, 2024, beta);
            let h = hoi_quy(&a, &b).unwrap();
            assert!((h.beta - beta).abs() < 0.1,
                    "hồi quy ra {:.3}, kỳ vọng {:.2}", h.beta, beta);
        }
    }
}
