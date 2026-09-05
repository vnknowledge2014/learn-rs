#![allow(dead_code, unused_variables)]
//! Chương 58 — Kỹ nghệ Dữ liệu & Phân tích bằng Rust.
//! Một mini "DataFrame" dạng cột + đường ống ETL, xây trên iterator (Chương 16)
//! và closure (Chương 15). Chạy offline, không cần Polars/Arrow, nhưng cùng ý tưởng.

use std::collections::HashMap;

// ============================================================================
// 1. MÔ HÌNH DỮ LIỆU DẠNG CỘT (Columnar) — vì sao nhanh hơn dạng hàng
// ============================================================================

#[derive(Debug, Clone, PartialEq)]
pub enum GiaTri {
    So(f64),
    Chuoi(String),
    Rong, // giá trị thiếu (NULL/NaN)
}

impl GiaTri {
    pub fn so(&self) -> Option<f64> {
        match self { GiaTri::So(x) => Some(*x), _ => None }
    }
    pub fn chuoi(&self) -> Option<&str> {
        match self { GiaTri::Chuoi(s) => Some(s), _ => None }
    }
}

/// Bảng dữ liệu lưu theo CỘT: mỗi cột là một Vec cùng kiểu, nằm liền nhau
/// trên bộ nhớ. Đây là lý do phân tích cột (tính tổng doanh thu) cực nhanh —
/// CPU quét một vùng nhớ liên tục, thân thiện với cache (Chương 25).
#[derive(Debug, Clone)]
pub struct Bang {
    pub ten_cot: Vec<String>,
    pub cot: Vec<Vec<GiaTri>>,
}

impl Bang {
    pub fn moi(ten_cot: Vec<&str>) -> Self {
        Bang {
            ten_cot: ten_cot.iter().map(|s| s.to_string()).collect(),
            cot: vec![Vec::new(); ten_cot.len()],
        }
    }
    pub fn chi_so_cot(&self, ten: &str) -> Option<usize> {
        self.ten_cot.iter().position(|c| c == ten)
    }
    pub fn them_hang(&mut self, hang: Vec<GiaTri>) {
        for (i, gt) in hang.into_iter().enumerate() {
            self.cot[i].push(gt);
        }
    }
    pub fn so_hang(&self) -> usize {
        self.cot.first().map(|c| c.len()).unwrap_or(0)
    }
    pub fn lay(&self, hang: usize, ten_cot: &str) -> Option<&GiaTri> {
        self.chi_so_cot(ten_cot).and_then(|c| self.cot[c].get(hang))
    }
}

// ============================================================================
// 2. GIAI ĐOẠN E — EXTRACT: phân tích CSV thành bảng (chống dữ liệu bẩn)
// ============================================================================

/// Phân tích một dòng CSV đơn giản (không xử lý dấu ngoặc kép lồng nhau).
pub fn tach_dong_csv(dong: &str) -> Vec<String> {
    dong.split(',').map(|s| s.trim().to_string()).collect()
}

/// EXTRACT: đọc nhiều dòng thô -> Bang, dùng filter_map để BỎ QUA dòng hỏng
/// (số cột sai). Đây là mẫu "làm sạch khi đọc" ở Chương 16.
pub fn extract_csv(dong: &[&str]) -> Result<Bang, String> {
    let mut it = dong.iter();
    let tieu_de = it.next().ok_or("CSV rỗng")?;
    let ten_cot: Vec<&str> = tieu_de.split(',').map(|s| s.trim()).collect();
    let so_cot = ten_cot.len();
    let mut bang = Bang::moi(ten_cot);

    for &d in it {
        if d.trim().is_empty() { continue; }
        let o = tach_dong_csv(d);
        if o.len() != so_cot { continue; } // bỏ dòng lệch cột
        let hang: Vec<GiaTri> = o.into_iter().map(suy_kieu).collect();
        bang.them_hang(hang);
    }
    Ok(bang)
}

/// Suy kiểu: thử số trước, rỗng nếu là "" hoặc "NA", còn lại là chuỗi.
pub fn suy_kieu(o: String) -> GiaTri {
    let t = o.trim();
    if t.is_empty() || t.eq_ignore_ascii_case("NA") || t.eq_ignore_ascii_case("null") {
        GiaTri::Rong
    } else if let Ok(n) = t.parse::<f64>() {
        GiaTri::So(n)
    } else {
        GiaTri::Chuoi(t.to_string())
    }
}

// ============================================================================
// 3. GIAI ĐOẠN T — TRANSFORM: lọc, thêm cột dẫn xuất, xử lý giá trị thiếu
// ============================================================================

impl Bang {
    /// Lọc hàng theo một vị từ trên hàng (Chương 15: closure làm tham số).
    pub fn loc(&self, giu: impl Fn(&HashMap<&str, &GiaTri>) -> bool) -> Bang {
        let mut moi = Bang::moi(self.ten_cot.iter().map(|s| s.as_str()).collect());
        for h in 0..self.so_hang() {
            let hang: HashMap<&str, &GiaTri> = self.ten_cot.iter().enumerate()
                .map(|(i, ten)| (ten.as_str(), &self.cot[i][h])).collect();
            if giu(&hang) {
                moi.them_hang((0..self.ten_cot.len()).map(|i| self.cot[i][h].clone()).collect());
            }
        }
        moi
    }

    /// Điền giá trị thiếu trong một cột số bằng một hằng số.
    pub fn dien_thieu(&mut self, ten_cot: &str, gia_tri: f64) {
        if let Some(c) = self.chi_so_cot(ten_cot) {
            for gt in self.cot[c].iter_mut() {
                if *gt == GiaTri::Rong {
                    *gt = GiaTri::So(gia_tri);
                }
            }
        }
    }
}

// ============================================================================
// 4. GIAI ĐOẠN L / PHÂN TÍCH — GROUP BY + AGGREGATE (trái tim của DA)
// ============================================================================

#[derive(Debug, Clone, PartialEq)]
pub struct KetQuaNhom {
    pub khoa: String,
    pub dem: usize,
    pub tong: f64,
    pub trung_binh: f64,
    pub nho_nhat: f64,
    pub lon_nhat: f64,
}

impl Bang {
    /// GROUP BY cột `theo`, tính thống kê trên cột số `tren`.
    /// Đây chính là VỊ NHÓM TÍCH ở Chương 18: gộp (đếm, tổng, min, max) trong 1 lượt.
    pub fn nhom_va_tong_hop(&self, theo: &str, tren: &str) -> Vec<KetQuaNhom> {
        let c_theo = self.chi_so_cot(theo).expect("cột nhóm không tồn tại");
        let c_tren = self.chi_so_cot(tren).expect("cột tính không tồn tại");

        // (tổng, đếm, min, max) cho mỗi khóa
        let mut gom: HashMap<String, (f64, usize, f64, f64)> = HashMap::new();
        for h in 0..self.so_hang() {
            let khoa = match &self.cot[c_theo][h] {
                GiaTri::Chuoi(s) => s.clone(),
                GiaTri::So(n) => n.to_string(),
                GiaTri::Rong => "(thiếu)".to_string(),
            };
            if let GiaTri::So(v) = self.cot[c_tren][h] {
                let e = gom.entry(khoa).or_insert((0.0, 0, f64::INFINITY, f64::NEG_INFINITY));
                e.0 += v;
                e.1 += 1;
                e.2 = e.2.min(v);
                e.3 = e.3.max(v);
            }
        }
        let mut kq: Vec<KetQuaNhom> = gom.into_iter().map(|(k, (tong, dem, min, max))| {
            KetQuaNhom {
                khoa: k, dem, tong,
                trung_binh: tong / dem as f64,
                nho_nhat: min, lon_nhat: max,
            }
        }).collect();
        // sắp xếp tất định: theo tổng giảm dần, rồi theo khóa
        kq.sort_by(|a, b| b.tong.partial_cmp(&a.tong).unwrap()
            .then(a.khoa.cmp(&b.khoa)));
        kq
    }
}

// ============================================================================
// 5. XỬ LÝ LUỒNG — WINDOW FUNCTION: trung bình trượt (streaming, O(1) bộ nhớ/cửa sổ)
// ============================================================================

/// Trung bình trượt cửa sổ `w` — mẫu cơ bản của phân tích chuỗi thời gian.
/// Dùng iterator `windows` (Chương 16), không nạp cả dãy vào RAM một lần.
pub fn trung_binh_truot(du_lieu: &[f64], w: usize) -> Vec<f64> {
    if w == 0 || du_lieu.len() < w {
        return Vec::new();
    }
    du_lieu.windows(w).map(|cua| cua.iter().sum::<f64>() / w as f64).collect()
}

/// Phát hiện điểm bất thường: lệch quá `nguong` lần độ lệch chuẩn khỏi trung bình.
pub fn phat_hien_bat_thuong(du_lieu: &[f64], nguong: f64) -> Vec<usize> {
    let n = du_lieu.len();
    if n == 0 { return Vec::new(); }
    let tb = du_lieu.iter().sum::<f64>() / n as f64;
    let phuong_sai = du_lieu.iter().map(|x| (x - tb).powi(2)).sum::<f64>() / n as f64;
    let do_lech = phuong_sai.sqrt();
    if do_lech == 0.0 { return Vec::new(); }
    du_lieu.iter().enumerate()
        .filter(|(_, &x)| (x - tb).abs() > nguong * do_lech)
        .map(|(i, _)| i)
        .collect()
}

// ============================================================================
// 6. JOIN — ghép hai bảng theo khóa chung
// ============================================================================

/// Inner join: chỉ giữ hàng có khóa khớp ở CẢ HAI bảng.
pub fn inner_join(trai: &Bang, phai: &Bang, khoa: &str) -> Bang {
    let ct = trai.chi_so_cot(khoa).expect("khóa không có ở bảng trái");
    let cp = phai.chi_so_cot(khoa).expect("khóa không có ở bảng phải");

    // Chỉ mục bảng phải theo khóa (băm) -> tra cứu O(1)
    let mut chi_muc: HashMap<String, Vec<usize>> = HashMap::new();
    for h in 0..phai.so_hang() {
        let k = format!("{:?}", phai.cot[cp][h]);
        chi_muc.entry(k).or_default().push(h);
    }

    // Cột kết quả: cột trái + cột phải (bỏ cột khóa trùng ở bảng phải)
    let mut ten: Vec<String> = trai.ten_cot.clone();
    for (i, t) in phai.ten_cot.iter().enumerate() {
        if i != cp { ten.push(format!("{}_phai", t)); }
    }
    let mut kq = Bang::moi(ten.iter().map(|s| s.as_str()).collect());

    for h in 0..trai.so_hang() {
        let k = format!("{:?}", trai.cot[ct][h]);
        if let Some(hang_phai) = chi_muc.get(&k) {
            for &hp in hang_phai {
                let mut hang: Vec<GiaTri> =
                    (0..trai.ten_cot.len()).map(|i| trai.cot[i][h].clone()).collect();
                for i in 0..phai.ten_cot.len() {
                    if i != cp { hang.push(phai.cot[i][hp].clone()); }
                }
                kq.them_hang(hang);
            }
        }
    }
    kq
}

fn main() {
    println!("═══════════════════════════════════════════════════════════════");
    println!("   KỸ NGHỆ DỮ LIỆU: EXTRACT → TRANSFORM → PHÂN TÍCH → JOIN     ");
    println!("═══════════════════════════════════════════════════════════════");

    let csv = vec![
        "ngay,khu_vuc,doanh_thu",
        "2026-01,Hà Nội,1500",
        "2026-01,TP.HCM,2200",
        "dòng hỏng thiếu cột",
        "2026-01,Hà Nội,800",
        "2026-02,TP.HCM,NA",     // giá trị thiếu
        "2026-02,Hà Nội,1200",
        "2026-02,Đà Nẵng,600",
    ];
    let mut bang = extract_csv(&csv).unwrap();
    println!("\n1. EXTRACT: {} dòng hợp lệ (đã bỏ dòng lỗi + tiêu đề)", bang.so_hang());

    println!("\n2. TRANSFORM: điền giá trị thiếu bằng 0");
    bang.dien_thieu("doanh_thu", 0.0);

    println!("\n3. PHÂN TÍCH: GROUP BY khu_vuc, tổng hợp doanh_thu");
    for r in bang.nhom_va_tong_hop("khu_vuc", "doanh_thu") {
        println!("   {:<10} | {} bản ghi | tổng {:>6.0} | TB {:>6.1} | [{:.0}–{:.0}]",
                 r.khoa, r.dem, r.tong, r.trung_binh, r.nho_nhat, r.lon_nhat);
    }

    println!("\n4. LỌC: chỉ giữ doanh thu > 1000");
    let lon = bang.loc(|h| h["doanh_thu"].so().map(|x| x > 1000.0).unwrap_or(false));
    println!("   Còn {} hàng", lon.so_hang());

    println!("\n5. XỬ LÝ LUỒNG: trung bình trượt & phát hiện bất thường");
    let chuoi = [10.0, 11.0, 9.0, 10.0, 50.0, 11.0, 10.0]; // 50 là điểm lạ
    println!("   Trung bình trượt (w=3): {:?}",
             trung_binh_truot(&chuoi, 3).iter().map(|x| (x * 10.0).round() / 10.0).collect::<Vec<_>>());
    println!("   Vị trí bất thường (>2σ): {:?}", phat_hien_bat_thuong(&chuoi, 2.0));

    println!("\n6. JOIN: ghép doanh thu với dân số khu vực");
    let mut ds = Bang::moi(vec!["khu_vuc", "dan_so_trieu"]);
    ds.them_hang(vec![GiaTri::Chuoi("Hà Nội".into()), GiaTri::So(8.4)]);
    ds.them_hang(vec![GiaTri::Chuoi("TP.HCM".into()), GiaTri::So(9.3)]);
    let ghep = inner_join(&bang, &ds, "khu_vuc");
    println!("   Kết quả join có {} hàng, {} cột (Đà Nẵng bị loại vì không có dân số)",
             ghep.so_hang(), ghep.ten_cot.len());

    println!("\n═══════════════════════════════════════════════════════════════");
    println!("   DỮ LIỆU DẠNG CỘT + ĐƯỜNG ỐNG HÀM = PHÂN TÍCH NHANH & AN TOÀN ");
    println!("═══════════════════════════════════════════════════════════════");
}

#[cfg(test)]
mod kiem_thu {
    use super::*;

    fn bang_mau() -> Bang {
        let csv = vec![
            "khu,thu",
            "A,100", "B,200", "A,50", "C,NA", "A,30", "B,80",
        ];
        extract_csv(&csv).unwrap()
    }

    #[test]
    fn extract_bo_qua_dong_hong() {
        let csv = vec!["a,b", "1,2", "thieu_cot", "3,4"];
        let b = extract_csv(&csv).unwrap();
        assert_eq!(b.so_hang(), 2); // "thieu_cot" bị loại
    }

    #[test]
    fn suy_kieu_dung() {
        assert_eq!(suy_kieu("42".into()), GiaTri::So(42.0));
        assert_eq!(suy_kieu("3.14".into()), GiaTri::So(3.14));
        assert_eq!(suy_kieu("Hà Nội".into()), GiaTri::Chuoi("Hà Nội".into()));
        assert_eq!(suy_kieu("".into()), GiaTri::Rong);
        assert_eq!(suy_kieu("NA".into()), GiaTri::Rong);
    }

    #[test]
    fn dien_gia_tri_thieu() {
        let mut b = bang_mau();
        b.dien_thieu("thu", 0.0);
        // Sau khi điền, cột 'thu' không còn Rong nào
        let c = b.chi_so_cot("thu").unwrap();
        assert!(!b.cot[c].contains(&GiaTri::Rong));
    }

    #[test]
    fn group_by_tong_hop_dung() {
        let b = bang_mau();
        let r = b.nhom_va_tong_hop("khu", "thu");
        // Sắp theo tổng giảm dần: A(180) > B(280)? -> B=280, A=180. Kiểm cụ thể.
        let a = r.iter().find(|x| x.khoa == "A").unwrap();
        assert_eq!(a.dem, 3);
        assert_eq!(a.tong, 180.0);
        assert_eq!(a.trung_binh, 60.0);
        assert_eq!(a.nho_nhat, 30.0);
        assert_eq!(a.lon_nhat, 100.0);
        let b_nhom = r.iter().find(|x| x.khoa == "B").unwrap();
        assert_eq!(b_nhom.tong, 280.0);
        // C chỉ có NA nên không xuất hiện (không có giá trị số nào)
        assert!(r.iter().find(|x| x.khoa == "C").is_none());
    }

    #[test]
    fn loc_theo_vi_tu() {
        let b = bang_mau();
        let l = b.loc(|h| h["thu"].so().map(|x| x >= 100.0).unwrap_or(false));
        assert_eq!(l.so_hang(), 2); // A=100, B=200
    }

    #[test]
    fn trung_binh_truot_dung() {
        assert_eq!(trung_binh_truot(&[1.0, 2.0, 3.0, 4.0], 2), vec![1.5, 2.5, 3.5]);
        assert_eq!(trung_binh_truot(&[1.0], 3), Vec::<f64>::new()); // ngắn hơn cửa sổ
        assert_eq!(trung_binh_truot(&[1.0, 2.0], 0), Vec::<f64>::new());
    }

    #[test]
    fn phat_hien_diem_bat_thuong() {
        // Ở ngưỡng 1.5σ, điểm 100 bị phát hiện.
        let bt = phat_hien_bat_thuong(&[10.0, 10.0, 10.0, 100.0, 10.0], 1.5);
        assert_eq!(bt, vec![3]);
        // BÀI HỌC THỐNG KÊ: cùng dữ liệu nhưng ở ngưỡng 2σ thì KHÔNG phát hiện,
        // vì một điểm cực lạ tự làm PHỒNG độ lệch chuẩn đến mức che chính nó.
        // Đây là lý do thống kê bền vững dùng trung vị + MAD thay cho TB + σ.
        assert!(phat_hien_bat_thuong(&[10.0, 10.0, 10.0, 100.0, 10.0], 2.0).is_empty());
        // Dãy phẳng không có bất thường.
        assert!(phat_hien_bat_thuong(&[5.0, 5.0, 5.0], 2.0).is_empty());
    }

    #[test]
    fn inner_join_chi_giu_khoa_khop() {
        let mut t = Bang::moi(vec!["id", "ten"]);
        t.them_hang(vec![GiaTri::So(1.0), GiaTri::Chuoi("An".into())]);
        t.them_hang(vec![GiaTri::So(2.0), GiaTri::Chuoi("Bình".into())]);
        t.them_hang(vec![GiaTri::So(3.0), GiaTri::Chuoi("Chi".into())]);
        let mut p = Bang::moi(vec!["id", "diem"]);
        p.them_hang(vec![GiaTri::So(1.0), GiaTri::So(9.0)]);
        p.them_hang(vec![GiaTri::So(2.0), GiaTri::So(8.0)]);
        // id=3 không có bên phải -> bị loại
        let j = inner_join(&t, &p, "id");
        assert_eq!(j.so_hang(), 2);
        assert_eq!(j.ten_cot.len(), 3); // id, ten, diem_phai
    }
}
