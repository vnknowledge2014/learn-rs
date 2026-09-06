#![allow(dead_code, unused_variables)]
//! Chương 58 — Kỹ nghệ Dữ liệu & Phân tích bằng Rust.
//! Một mini "DataFrame" dạng cột + đường ống ETL, xây trên iterator (Chương 16)
//! và closure (Chương 15). Chạy offline, không cần Polars/Arrow, nhưng cùng ý tưởng.

use std::collections::HashMap;

// ============================================================================
// 1. MÔ HÌNH DỮ LIỆU DẠNG CỘT (Columnar) — vì sao fast hơn dạng hàng
// ============================================================================

#[derive(Debug, Clone, PartialEq)]
pub enum Value {
    So(f64),
    Text(String),
    Rong, // giá trị thiếu (NULL/NaN)
}

impl Value {
    pub fn so(&self) -> Option<f64> {
        match self { Value::So(x) => Some(*x), _ => None }
    }
    pub fn series(&self) -> Option<&str> {
        match self { Value::Text(s) => Some(s), _ => None }
    }
}

/// Bảng dữ liệu lưu theo CỘT: mỗi cột là một Vec cùng kiểu, nằm liền nhau
/// trên bộ nhớ. Đây là lý do phân tích cột (tính tổng doanh thu) cực fast —
/// CPU quét một vùng nhớ liên tục, thân thiện với cache (Chương 25).
#[derive(Debug, Clone)]
pub struct Bang {
    pub ten_cot: Vec<String>,
    pub cot: Vec<Vec<Value>>,
}

impl Bang {
    pub fn new(ten_cot: Vec<&str>) -> Self {
        Bang {
            ten_cot: ten_cot.iter().map(|s| s.to_string()).collect(),
            cot: vec![Vec::new(); ten_cot.len()],
        }
    }
    pub fn chi_so_cot(&self, name: &str) -> Option<usize> {
        self.ten_cot.iter().position(|c| c == name)
    }
    pub fn add_queue(&mut self, queue: Vec<Value>) {
        for (i, gt) in queue.into_iter().enumerate() {
            self.cot[i].push(gt);
        }
    }
    pub fn num_queue(&self) -> usize {
        self.cot.first().map(|c| c.len()).unwrap_or(0)
    }
    pub fn lay(&self, queue: usize, ten_cot: &str) -> Option<&Value> {
        self.chi_so_cot(ten_cot).and_then(|c| self.cot[c].get(queue))
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
    let title = it.next().ok_or("CSV rỗng")?;
    let ten_cot: Vec<&str> = title.split(',').map(|s| s.trim()).collect();
    let so_cot = ten_cot.len();
    let mut bang = Bang::new(ten_cot);

    for &d in it {
        if d.trim().is_empty() { continue; }
        let o = tach_dong_csv(d);
        if o.len() != so_cot { continue; } // bỏ dòng lệch cột
        let queue: Vec<Value> = o.into_iter().map(infer_type).collect();
        bang.add_queue(queue);
    }
    Ok(bang)
}

/// Suy kiểu: thử số trước, rỗng nếu là "" hoặc "NA", còn lại là chuỗi.
pub fn infer_type(o: String) -> Value {
    let t = o.trim();
    if t.is_empty() || t.eq_ignore_ascii_case("NA") || t.eq_ignore_ascii_case("null") {
        Value::Rong
    } else if let Ok(n) = t.parse::<f64>() {
        Value::So(n)
    } else {
        Value::Text(t.to_string())
    }
}

// ============================================================================
// 3. GIAI ĐOẠN T — TRANSFORM: lọc, thêm cột dẫn xuất, xử lý giá trị thiếu
// ============================================================================

impl Bang {
    /// Lọc hàng theo một vị từ trên hàng (Chương 15: closure làm tham số).
    pub fn filter(&self, giu: impl Fn(&HashMap<&str, &Value>) -> bool) -> Bang {
        let mut new = Bang::new(self.ten_cot.iter().map(|s| s.as_str()).collect());
        for h in 0..self.num_queue() {
            let queue: HashMap<&str, &Value> = self.ten_cot.iter().enumerate()
                .map(|(i, name)| (name.as_str(), &self.cot[i][h])).collect();
            if giu(&queue) {
                new.add_queue((0..self.ten_cot.len()).map(|i| self.cot[i][h].clone()).collect());
            }
        }
        new
    }

    /// Điền giá trị thiếu trong một cột số bằng một hằng số.
    pub fn missing_signal(&mut self, ten_cot: &str, value: f64) {
        if let Some(c) = self.chi_so_cot(ten_cot) {
            for gt in self.cot[c].iter_mut() {
                if *gt == Value::Rong {
                    *gt = Value::So(value);
                }
            }
        }
    }
}

// ============================================================================
// 4. GIAI ĐOẠN L / PHÂN TÍCH — GROUP BY + AGGREGATE (trái tim của DA)
// ============================================================================

#[derive(Debug, Clone, PartialEq)]
pub struct ResultGroup {
    pub key: String,
    pub count: usize,
    pub tong: f64,
    pub mean: f64,
    pub min: f64,
    pub max: f64,
}

impl Bang {
    /// GROUP BY cột `theo`, tính thống kê trên cột số `tren`.
    /// Đây chính là VỊ NHÓM TÍCH ở Chương 18: gộp (đếm, tổng, min, max) trong 1 lượt.
    pub fn group_and_total_hop(&self, theo: &str, above: &str) -> Vec<ResultGroup> {
        let c_theo = self.chi_so_cot(theo).expect("cột nhóm không tồn tại");
        let c_tren = self.chi_so_cot(above).expect("cột tính không tồn tại");

        // (tổng, đếm, min, max) cho mỗi khóa
        let mut gom: HashMap<String, (f64, usize, f64, f64)> = HashMap::new();
        for h in 0..self.num_queue() {
            let key = match &self.cot[c_theo][h] {
                Value::Text(s) => s.clone(),
                Value::So(n) => n.to_string(),
                Value::Rong => "(thiếu)".to_string(),
            };
            if let Value::So(v) = self.cot[c_tren][h] {
                let e = gom.entry(key).or_insert((0.0, 0, f64::INFINITY, f64::NEG_INFINITY));
                e.0 += v;
                e.1 += 1;
                e.2 = e.2.min(v);
                e.3 = e.3.max(v);
            }
        }
        let mut kq: Vec<ResultGroup> = gom.into_iter().map(|(k, (tong, count, min, max))| {
            ResultGroup {
                key: k, count, tong,
                mean: tong / count as f64,
                min: min, max: max,
            }
        }).collect();
        // sắp xếp tất định: theo tổng giảm dần, rồi theo khóa
        kq.sort_by(|a, b| b.tong.partial_cmp(&a.tong).unwrap()
            .then(a.key.cmp(&b.key)));
        kq
    }
}

// ============================================================================
// 5. XỬ LÝ LUỒNG — WINDOW FUNCTION: trung bình trượt (streaming, O(1) bộ nhớ/cửa sổ)
// ============================================================================

/// Trung bình trượt cửa sổ `w` — mẫu cơ bản của phân tích chuỗi thời gian.
/// Dùng iterator `windows` (Chương 16), không nạp cả dãy vào RAM một lần.
pub fn moving_average(data: &[f64], w: usize) -> Vec<f64> {
    if w == 0 || data.len() < w {
        return Vec::new();
    }
    data.windows(w).map(|cua| cua.iter().sum::<f64>() / w as f64).collect()
}

/// Phát hiện điểm bất thường: lệch quá `nguong` lần độ lệch chuẩn khỏi trung bình.
pub fn emit_normal(data: &[f64], threshold: f64) -> Vec<usize> {
    let n = data.len();
    if n == 0 { return Vec::new(); }
    let tb = data.iter().sum::<f64>() / n as f64;
    let variance = data.iter().map(|x| (x - tb).powi(2)).sum::<f64>() / n as f64;
    let do_lech = variance.sqrt();
    if do_lech == 0.0 { return Vec::new(); }
    data.iter().enumerate()
        .filter(|(_, &x)| (x - tb).abs() > threshold * do_lech)
        .map(|(i, _)| i)
        .collect()
}

// ============================================================================
// 6. JOIN — ghép hai bảng theo khóa chung
// ============================================================================

/// Inner join: chỉ giữ hàng có khóa khớp ở CẢ HAI bảng.
pub fn inner_join(left: &Bang, right: &Bang, key: &str) -> Bang {
    let ct = left.chi_so_cot(key).expect("khóa không có ở bảng trái");
    let cp = right.chi_so_cot(key).expect("khóa không có ở bảng phải");

    // Chỉ mục bảng phải theo khóa (băm) -> tra cứu O(1)
    let mut only_level: HashMap<String, Vec<usize>> = HashMap::new();
    for h in 0..right.num_queue() {
        let k = format!("{:?}", right.cot[cp][h]);
        only_level.entry(k).or_default().push(h);
    }

    // Cột kết quả: cột trái + cột phải (bỏ cột khóa trùng ở bảng phải)
    let mut name: Vec<String> = left.ten_cot.clone();
    for (i, t) in right.ten_cot.iter().enumerate() {
        if i != cp { name.push(format!("{}_phai", t)); }
    }
    let mut kq = Bang::new(name.iter().map(|s| s.as_str()).collect());

    for h in 0..left.num_queue() {
        let k = format!("{:?}", left.cot[ct][h]);
        if let Some(hang_phai) = only_level.get(&k) {
            for &hp in hang_phai {
                let mut queue: Vec<Value> =
                    (0..left.ten_cot.len()).map(|i| left.cot[i][h].clone()).collect();
                for i in 0..right.ten_cot.len() {
                    if i != cp { queue.push(right.cot[i][hp].clone()); }
                }
                kq.add_queue(queue);
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
    println!("\n1. EXTRACT: {} dòng hợp lệ (đã bỏ dòng lỗi + tiêu đề)", bang.num_queue());

    println!("\n2. TRANSFORM: điền giá trị thiếu bằng 0");
    bang.missing_signal("doanh_thu", 0.0);

    println!("\n3. PHÂN TÍCH: GROUP BY khu_vuc, tổng hợp doanh_thu");
    for r in bang.group_and_total_hop("khu_vuc", "doanh_thu") {
        println!("   {:<10} | {} bản ghi | tổng {:>6.0} | TB {:>6.1} | [{:.0}–{:.0}]",
                 r.key, r.count, r.tong, r.mean, r.min, r.max);
    }

    println!("\n4. LỌC: chỉ giữ doanh thu > 1000");
    let large = bang.filter(|h| h["doanh_thu"].so().map(|x| x > 1000.0).unwrap_or(false));
    println!("   Còn {} hàng", large.num_queue());

    println!("\n5. XỬ LÝ LUỒNG: trung bình trượt & phát hiện bất thường");
    let series = [10.0, 11.0, 9.0, 10.0, 50.0, 11.0, 10.0]; // 50 là điểm lạ
    println!("   Trung bình trượt (w=3): {:?}",
             moving_average(&series, 3).iter().map(|x| (x * 10.0).round() / 10.0).collect::<Vec<_>>());
    println!("   Vị trí bất thường (>2σ): {:?}", emit_normal(&series, 2.0));

    println!("\n6. JOIN: ghép doanh thu với dân số khu vực");
    let mut list = Bang::new(vec!["khu_vuc", "dan_so_trieu"]);
    list.add_queue(vec![Value::Text("Hà Nội".into()), Value::So(8.4)]);
    list.add_queue(vec![Value::Text("TP.HCM".into()), Value::So(9.3)]);
    let compose = inner_join(&bang, &list, "khu_vuc");
    println!("   Kết quả join có {} hàng, {} cột (Đà Nẵng bị loại vì không có dân số)",
             compose.num_queue(), compose.ten_cot.len());

    println!("\n═══════════════════════════════════════════════════════════════");
    println!("   DỮ LIỆU DẠNG CỘT + ĐƯỜNG ỐNG HÀM = PHÂN TÍCH NHANH & AN TOÀN ");
    println!("═══════════════════════════════════════════════════════════════");
}

#[cfg(test)]
mod tests {
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
        assert_eq!(b.num_queue(), 2); // "thieu_cot" bị loại
    }

    #[test]
    fn type_inference_is_correct() {
        assert_eq!(infer_type("42".into()), Value::So(42.0));
        assert_eq!(infer_type("3.14".into()), Value::So(3.14));
        assert_eq!(infer_type("Hà Nội".into()), Value::Text("Hà Nội".into()));
        assert_eq!(infer_type("".into()), Value::Rong);
        assert_eq!(infer_type("NA".into()), Value::Rong);
    }

    #[test]
    fn fills_missing_values() {
        let mut b = bang_mau();
        b.missing_signal("thu", 0.0);
        // Sau khi điền, cột 'thu' không còn Rong nào
        let c = b.chi_so_cot("thu").unwrap();
        assert!(!b.cot[c].contains(&Value::Rong));
    }

    #[test]
    fn group_by_tong_hop_dung() {
        let b = bang_mau();
        let r = b.group_and_total_hop("khu", "thu");
        // Sắp theo tổng giảm dần: A(180) > B(280)? -> B=280, A=180. Kiểm cụ thể.
        let a = r.iter().find(|x| x.key == "A").unwrap();
        assert_eq!(a.count, 3);
        assert_eq!(a.tong, 180.0);
        assert_eq!(a.mean, 60.0);
        assert_eq!(a.min, 30.0);
        assert_eq!(a.max, 100.0);
        let b_nhom = r.iter().find(|x| x.key == "B").unwrap();
        assert_eq!(b_nhom.tong, 280.0);
        // C chỉ có NA nên không xuất hiện (không có giá trị số nào)
        assert!(r.iter().find(|x| x.key == "C").is_none());
    }

    #[test]
    fn filters_by_predicate() {
        let b = bang_mau();
        let l = b.filter(|h| h["thu"].so().map(|x| x >= 100.0).unwrap_or(false));
        assert_eq!(l.num_queue(), 2); // A=100, B=200
    }

    #[test]
    fn moving_average_is_correct() {
        assert_eq!(moving_average(&[1.0, 2.0, 3.0, 4.0], 2), vec![1.5, 2.5, 3.5]);
        assert_eq!(moving_average(&[1.0], 3), Vec::<f64>::new()); // ngắn hơn cửa sổ
        assert_eq!(moving_average(&[1.0, 2.0], 0), Vec::<f64>::new());
    }

    #[test]
    fn detects_outliers() {
        // Ở ngưỡng 1.5σ, điểm 100 bị phát hiện.
        let bt = emit_normal(&[10.0, 10.0, 10.0, 100.0, 10.0], 1.5);
        assert_eq!(bt, vec![3]);
        // BÀI HỌC THỐNG KÊ: cùng dữ liệu nhưng ở ngưỡng 2σ thì KHÔNG phát hiện,
        // vì một điểm cực lạ tự làm PHỒNG độ lệch chuẩn đến mức che chính nó.
        // Đây là lý do thống kê bền vững dùng trung vị + MAD thay cho TB + σ.
        assert!(emit_normal(&[10.0, 10.0, 10.0, 100.0, 10.0], 2.0).is_empty());
        // Dãy phẳng không có bất thường.
        assert!(emit_normal(&[5.0, 5.0, 5.0], 2.0).is_empty());
    }

    #[test]
    fn inner_join_keeps_only_matching_keys() {
        let mut t = Bang::new(vec!["id", "ten"]);
        t.add_queue(vec![Value::So(1.0), Value::Text("An".into())]);
        t.add_queue(vec![Value::So(2.0), Value::Text("Bình".into())]);
        t.add_queue(vec![Value::So(3.0), Value::Text("Chi".into())]);
        let mut p = Bang::new(vec!["id", "diem"]);
        p.add_queue(vec![Value::So(1.0), Value::So(9.0)]);
        p.add_queue(vec![Value::So(2.0), Value::So(8.0)]);
        // id=3 không có bên phải -> bị loại
        let j = inner_join(&t, &p, "id");
        assert_eq!(j.num_queue(), 2);
        assert_eq!(j.ten_cot.len(), 3); // id, ten, diem_phai
    }
}
