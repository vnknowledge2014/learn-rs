# Chương 58: Kỹ nghệ Dữ liệu & Phân tích bằng Rust — ETL, DataFrame dạng cột, Group-By, Window & Join (Data Engineering & Analytics)

## Giới thiệu & Mục tiêu học tập

Rust đang trở thành một ngôn ngữ hàng đầu cho **kỹ nghệ dữ liệu**. Thư viện `Polars` (viết bằng Rust) nhanh hơn `pandas` của Python nhiều lần và đang được cả cộng đồng Python dùng lại. `Apache Arrow` — chuẩn dữ liệu dạng cột của toàn ngành — có phần cài đặt Rust cực mạnh. Lý do: xử lý dữ liệu lớn cần đúng ba thứ Rust giỏi nhất — **tốc độ, an toàn bộ nhớ, và song song không sợ tranh chấp** (Chương 16, `rayon`).

Chương này xây một **mini-DataFrame dạng cột** từ đầu, để bạn hiểu *cơ chế bên dưới* Polars/Arrow, rồi mới dùng thư viện thật một cách sáng suốt. Toàn bộ đường ống **ETL** (Extract → Transform → Load/Analyze) được xây trên iterator (Chương 16) và closure (Chương 15) — kỹ nghệ dữ liệu về bản chất chính là **lập trình hàm trên dữ liệu quy mô lớn**.

Mục tiêu học tập:
- Hiểu **dữ liệu dạng cột (columnar)** và vì sao nó nhanh hơn dạng hàng cho phân tích.
- Xây đường ống **ETL**: Extract (phân tích CSV, bỏ dòng bẩn), Transform (lọc, điền thiếu), Analyze.
- Làm chủ **GROUP BY + Aggregate** — và nhận ra nó chính là *vị nhóm tích* ở Chương 18.
- Viết **window function** (trung bình trượt) và **phát hiện bất thường** cho chuỗi thời gian.
- Cài **inner join** hai bảng bằng chỉ mục băm (Chương 30).
- Biết hệ sinh thái dữ liệu Rust: Polars, Arrow, DataFusion — và khi nào dùng gì.

---

## Hình tượng hóa đời sống (Intuitive Everyday Analogy)

```
┌──────────────────────────────────────────────────────────────────────────────────┐
│         HÌNH TƯỢNG: TỦ HỒ SƠ THEO HÀNG vs THEO CỘT                                │
├────────────────────────────────────────┬─────────────────────────────────────────┤
│   LƯU THEO HÀNG (như một sổ ghi chép)  │   LƯU THEO CỘT (như bảng tính chuyên biệt)│
│                                        │                                         │
│   Hàng 1: [Tên: An | Tuổi: 30 | Lương] │   Cột Tên  : [An, Bình, Chi, Dũng, ...]  │
│   Hàng 2: [Tên: Bình| Tuổi: 25 | Lương] │   Cột Tuổi : [30, 25, 28, 41, ...]       │
│   Hàng 3: [Tên: Chi | Tuổi: 28 | Lương] │   Cột Lương: [15tr, 12tr, 18tr, ...]     │
│                                        │                                         │
│   "Tính TỔNG lương của 1 triệu người?" │   "Tính TỔNG lương của 1 triệu người?"   │
│   → phải nhảy qua từng hồ sơ, đọc cả   │   → quét MỘT vùng nhớ liền mạch (cột     │
│     tên và tuổi rồi mới tới lương.     │     Lương), CPU đọc cache cực nhanh,     │
│     Nhảy cóc trong RAM = trượt cache.  │     bỏ qua hoàn toàn Tên và Tuổi.        │
│                                        │                                         │
│   Tốt cho: "lấy TOÀN BỘ hồ sơ của An"  │   Tốt cho: "phân tích MỘT cột qua triệu │
│   (giao dịch — OLTP)                   │   hàng" (phân tích — OLAP)               │
└────────────────────────────────────────┴─────────────────────────────────────────┘
```

Cơ sở dữ liệu giao dịch (Chương 31–36) lưu theo hàng vì hay đọc/ghi trọn một bản ghi. Còn công cụ phân tích lưu theo cột vì hay quét một cột qua hàng triệu dòng. Đây cũng là lý do liên hệ trực tiếp tới **cache CPU** ở Chương 25: dữ liệu liền nhau = ít trượt cache = nhanh.

---

## Khái niệm & Cơ chế kỹ thuật chuyên sâu (In-Depth Technical Mechanics)

### 1. Vì sao dạng cột nhanh hơn cho phân tích

Ba lý do kỹ thuật:
1. **Thân thiện cache** (Chương 25): quét một cột là quét một vùng nhớ liên tục; CPU nạp nguyên dòng cache và dùng hết.
2. **Nén tốt hơn**: dữ liệu cùng kiểu, cùng phân phối nằm cạnh nhau nén hiệu quả (ví dụ cột "khu_vuc" chỉ có 3 giá trị lặp lại → nén xuống vài byte).
3. **Vector hóa (SIMD)**: cộng một triệu số `f64` liền nhau cho phép CPU dùng lệnh SIMD xử lý 4–8 số mỗi nhịp — điều bất khả nếu dữ liệu rải rác giữa các trường khác.

### 2. ETL — ba giai đoạn, và "làm sạch khi đọc"

**Extract** là nơi dữ liệu bẩn nhất. Dòng thiếu cột, giá trị không phải số, ô rỗng — tất cả phải xử lý ngay ở cổng vào. Mẫu Rust idiomatic là `filter_map` (Chương 16): thử phân tích từng dòng, **bỏ qua** cái hỏng thay vì để cả pipeline sập. Đây chính là "hàm toàn phần" ở Chương 13 áp dụng vào dữ liệu — mọi đầu vào đều có đường xử lý, kể cả đầu vào rác.

**Transform** gồm lọc, thêm cột dẫn xuất, và **xử lý giá trị thiếu** (`Value::Rong`). Quyết định điền thiếu bằng gì (0, trung bình, giá trị trước đó) là một quyết định *nghiệp vụ*, không phải kỹ thuật — và nó ảnh hưởng lớn tới kết quả phân tích.

### 3. GROUP BY chính là Vị nhóm Tích (Chương 18)

Đây là mối liên hệ đẹp nhất chương. Khi bạn `GROUP BY khu_vuc` rồi tính `(đếm, tổng, min, max)`, bạn đang:
- Gộp các giá trị của cùng một khóa bằng một **vị nhóm tích bốn thành phần** (Chương 18): `(Tong, Dem, Min, Max)`.
- Mỗi thành phần là một vị nhóm kết hợp, nên phép gộp **song song hóa được** — chia dữ liệu ra nhiều nhân, gộp từng phần, rồi ghép lại (đúng như `rayon` làm với `par_iter`).

Nói cách khác: cả một động cơ GROUP BY của cơ sở dữ liệu phân tích thực chất là *một fold trên một vị nhóm*. Toán học ở Chương 18 không phải trang trí — nó là kiến trúc.

### 4. Window function và chuỗi thời gian

**Trung bình trượt** làm mượt nhiễu và làm lộ xu hướng. Nó dùng `slice::windows(w)` (Chương 16) — trượt một cửa sổ độ rộng cố định qua dãy. Với dữ liệu lớn thật, phiên bản streaming chỉ giữ cửa sổ hiện tại trong RAM (O(w) bộ nhớ) thay vì cả dãy.

**Phát hiện bất thường** dựa trên độ lệch chuẩn có một cạm bẫy thống kê quan trọng mà bài kiểm thử trong chương này phơi bày: *một điểm cực lạ tự làm phồng độ lệch chuẩn đến mức che chính nó*. Đây là lý do thống kê bền vững (robust statistics) dùng **trung vị và MAD** (median absolute deviation) thay cho trung bình và σ.

### 5. Join bằng chỉ mục băm

Ghép hai bảng theo khóa chung. Cách ngây thơ là vòng lặp lồng (mỗi hàng trái quét cả bảng phải — O(N×M)). Cách đúng: xây **chỉ mục băm** trên bảng phải (`HashMap<khóa, danh sách hàng>`), rồi mỗi hàng trái tra O(1). Đây chính là **hash join** — thuật toán join phổ biến nhất trong các cơ sở dữ liệu thật, và nó dùng đúng bảng băm ở Chương 30.

---

## Mã nguồn minh họa thực chiến (Idiomatic Runnable Rust Blueprint)

```bash
cd code
cargo run  -p ch58
cargo test -p ch58
```

```rust
//! Chương 58 — Kỹ nghệ Dữ liệu & Phân tích bằng Rust.
//! Một mini "DataFrame" dạng cột + đường ống ETL, xây trên iterator (Chương 16)
//! và closure (Chương 15). Chạy offline, không cần Polars/Arrow, nhưng cùng ý tưởng.

use std::collections::HashMap;

// ============================================================================
// 1. MÔ HÌNH DỮ LIỆU DẠNG CỘT (Columnar) — vì sao nhanh hơn dạng hàng
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
/// trên bộ nhớ. Đây là lý do phân tích cột (tính tổng doanh attempt) cực nhanh —
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
        let c_above = self.chi_so_cot(above).expect("cột tính không tồn tại");

        // (tổng, đếm, min, max) cho mỗi khóa
        let mut gom: HashMap<String, (f64, usize, f64, f64)> = HashMap::new();
        for h in 0..self.num_queue() {
            let key = match &self.cot[c_theo][h] {
                Value::Text(s) => s.clone(),
                Value::So(n) => n.to_string(),
                Value::Rong => "(thiếu)".to_string(),
            };
            if let Value::So(v) = self.cot[c_above][h] {
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
// 6. JOIN — ghép hai bảng theo khóa shared
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

    println!("\n4. LỌC: chỉ giữ doanh attempt > 1000");
    let large = bang.filter(|h| h["doanh_thu"].so().map(|x| x > 1000.0).unwrap_or(false));
    println!("   Còn {} hàng", large.num_queue());

    println!("\n5. XỬ LÝ LUỒNG: trung bình trượt & phát hiện bất thường");
    let series = [10.0, 11.0, 9.0, 10.0, 50.0, 11.0, 10.0]; // 50 là điểm lạ
    println!("   Trung bình trượt (w=3): {:?}",
             moving_average(&series, 3).iter().map(|x| (x * 10.0).round() / 10.0).collect::<Vec<_>>());
    println!("   Vị trí bất thường (>2σ): {:?}", emit_normal(&series, 2.0));

    println!("\n6. JOIN: ghép doanh attempt với dân số khu vực");
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
            "khu,attempt",
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
        b.missing_signal("attempt", 0.0);
        // Sau khi điền, cột 'attempt' không còn Rong nào
        let c = b.chi_so_cot("attempt").unwrap();
        assert!(!b.cot[c].contains(&Value::Rong));
    }

    #[test]
    fn group_by_tong_hop_dung() {
        let b = bang_mau();
        let r = b.group_and_total_hop("khu", "attempt");
        // Sắp theo tổng giảm dần: A(180) > B(280)? -> B=280, A=180. Kiểm cụ thể.
        let a = r.iter().find(|x| x.key == "A").unwrap();
        assert_eq!(a.count, 3);
        assert_eq!(a.tong, 180.0);
        assert_eq!(a.mean, 60.0);
        assert_eq!(a.min, 30.0);
        assert_eq!(a.max, 100.0);
        let b_group = r.iter().find(|x| x.key == "B").unwrap();
        assert_eq!(b_group.tong, 280.0);
        // C chỉ có NA nên không xuất hiện (không có giá trị số nào)
        assert!(r.iter().find(|x| x.key == "C").is_none());
    }

    #[test]
    fn filters_by_predicate() {
        let b = bang_mau();
        let l = b.filter(|h| h["attempt"].so().map(|x| x >= 100.0).unwrap_or(false));
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
```

---

## Hệ sinh thái Dữ liệu Rust

| Thư viện | Vai trò | Khi nào dùng |
|---|---|---|
| **[Polars](https://pola.rs/)** | DataFrame tốc độ cao, API giống pandas | Phân tích dữ liệu vừa và lớn trên một máy |
| **[Apache Arrow](https://arrow.apache.org/)** | Chuẩn bộ nhớ dạng cột của toàn ngành | Trao đổi dữ liệu zero-copy giữa các hệ thống |
| **[DataFusion](https://datafusion.apache.org/)** | Engine truy vấn SQL trên Arrow | Chạy SQL trên tệp Parquet/CSV rất lớn |
| **`csv` + `serde`** | Đọc/ghi CSV có kiểu | ETL cơ bản, an toàn kiểu |
| **`rayon`** | Song song hóa dữ liệu | Tăng tốc group-by, map trên nhiều nhân |

> **Vì sao học tự xây trước khi dùng Polars?** Vì khi Polars chạy chậm hay cho kết quả lạ, bạn cần hiểu *nó đang làm gì bên dưới* — dạng cột, hash join, lazy evaluation. Kiến thức trong chương này chính là bản đồ để đọc và gỡ lỗi công cụ thật.

---

## Bảng tra cứu lỗi biên dịch thường gặp

| Lỗi | Nguyên nhân trong chương này | Cách sửa |
|---|---|---|
| `E0507: cannot move out of index` | Lấy `Value` ra khỏi cột bằng phép gán | `.clone()`, hoặc thao tác qua tham chiếu `&self.o[i]` |
| `E0502: cannot borrow as mutable` | Duyệt cột này để ghi vào cột khác của cùng `Bang` | Tính ra `Vec` mới rồi mới gán vào bảng |
| `E0277: f64 does not implement Ord` | `sort()` trên cột số thực | `sort_by(\|a, b\| a.partial_cmp(b).unwrap())` — `f64` chỉ có thứ tự bộ phận vì `NaN` |
| `E0599: no method named iter found for Value` | Nhầm `Value` (một ô) với cột | Lấy cột qua `chi_so_cot` rồi mới `iter()` |
| Gộp nhóm ra kết quả khác nhau mỗi lần chạy | Duyệt `HashMap` khi gom nhóm | `BTreeMap` cho thứ tự tất định — điều kiện để so sánh kết quả |

---

## Tóm tắt chương & Bài tập rèn luyện (Summary & Exercises)

### 4 Điểm cốt lõi cần ghi nhớ:
1. **Dạng cột nhanh hơn cho phân tích** nhờ thân thiện cache, nén tốt và vector hóa SIMD — đối lập với dạng hàng của cơ sở dữ liệu giao dịch.
2. **ETL là lập trình hàm trên dữ liệu**: `filter_map` để làm sạch khi đọc, `filter`/`map` để biến đổi, `fold` để tổng hợp.
3. **GROUP BY = fold trên một vị nhóm tích** (Chương 18) — nên nó song song hóa được. Toán học là kiến trúc, không phải trang trí.
4. **Hash join** dùng bảng băm (Chương 30) để đạt O(N+M) thay vì O(N×M). Đây là cách cơ sở dữ liệu thật ghép bảng.

### Bài tập rèn luyện tự giải:

**Bài tập 1 (Thêm hàm tổng hợp trung vị)**
`group_and_total_hop` tính tổng/trung bình/min/max. Trung vị bền vững hơn với điểm lạ. Viết `trung_vi(so: &[f64]) -> f64` và một bài test.

<details>
<summary><b>Lời giải</b></summary>

```rust
pub fn median(so: &[f64]) -> f64 {
    if so.is_empty() { return 0.0; }
    let mut v = so.to_vec();
    v.sort_by(|a, b| a.partial_cmp(b).unwrap());
    let n = v.len();
    if n % 2 == 1 { v[n / 2] } else { (v[n / 2 - 1] + v[n / 2]) / 2.0 }
}

#[cfg(test)]
mod bt1 {
    use super::*;
    #[test]
    fn median_resists_outliers() {
        // Trung bình bị điểm 1000 kéo lên; trung vị thì không.
        let data = [1.0, 2.0, 3.0, 4.0, 1000.0];
        assert_eq!(median(&data), 3.0);       // ổn định
        let tb = data.iter().sum::<f64>() / 5.0;
        assert_eq!(tb, 202.0);                     // bị bóp méo
    }
}
```
</details>

**Bài tập 2 (Left join)**
`inner_join` chỉ giữ hàng khớp cả hai bên. Viết `left_join` giữ **mọi** hàng bảng trái; hàng không khớp thì các cột phải điền `Value::Rong`. Test với "Đà Nẵng" không có dân số.

<details>
<summary><b>Gợi ý</b></summary>

Giống `inner_join` nhưng khi không tìm thấy khóa ở chỉ mục phải, vẫn thêm hàng trái và đệm `Value::Rong` cho đủ số cột phải. Đây là join hay dùng nhất khi làm giàu (enrich) dữ liệu — giữ nguyên bảng chính, gắn thêm thông tin nếu có.
</details>

**Bài tập 3 (Tư duy: dạng hàng hay dạng cột?)**
Với mỗi hệ thống, chọn cách lưu và giải thích:
1. Ứng dụng ngân hàng: xem/sửa số dư một tài khoản.
2. Bảng điều khiển phân tích: doanh thu trung bình theo tháng qua 5 năm.
3. Mạng xã hội: tải toàn bộ hồ sơ một người dùng.
4. Hệ thống gợi ý: tính điểm tương đồng trên một cột đặc trưng qua hàng triệu người dùng.

<details>
<summary><b>Lời giải tham khảo</b></summary>

1. **Dạng hàng** (OLTP). Luôn đọc/ghi trọn một bản ghi tài khoản.
2. **Dạng cột** (OLAP). Chỉ quét cột doanh thu qua nhiều dòng.
3. **Dạng hàng**. Lấy tất cả trường của một thực thể.
4. **Dạng cột**. Quét một cột đặc trưng qua hàng triệu hàng — chính là thế mạnh SIMD của dạng cột.

Quy tắc: **giao dịch → hàng; phân tích → cột.** Nhiều hệ thống lớn dùng CẢ HAI (kiến trúc HTAP): cơ sở dữ liệu hàng cho giao dịch, đồng bộ sang kho cột cho phân tích.
</details>
