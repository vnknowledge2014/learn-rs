//! Chương 58 — Kỹ nghệ Dữ liệu & Phân tích bằng Rust.
//! Một mini "DataFrame" dạng cột + đường ống ETL, xây trên iterator (Chương 16)
//! và closure (Chương 15). Chạy offline, không cần Polars/Arrow, nhưng cùng ý tưởng.

use std::collections::HashMap;

// ============================================================================
// 1. MÔ HÌNH DỮ LIỆU DẠNG CỘT (Columnar) — vì sao nhanh hơn dạng hàng
// ============================================================================

#[derive(Debug, Clone, PartialEq)]
pub enum Value {
    Number(f64),
    Text(String),
    Null, // giá trị thiếu (NULL/NaN)
}

impl Value {
    pub fn as_number(&self) -> Option<f64> {
        match self {
            Value::Number(x) => Some(*x),
            _ => None,
        }
    }
    pub fn as_str(&self) -> Option<&str> {
        match self {
            Value::Text(s) => Some(s),
            _ => None,
        }
    }
}

/// Bảng dữ liệu lưu theo CỘT: mỗi cột là một Vec cùng kiểu, nằm liền nhau
/// trên bộ nhớ. Đây là lý do phân tích cột (tính tổng doanh thu) cực nhanh —
/// CPU quét một vùng nhớ liên tục, thân thiện với cache (Chương 25).
#[derive(Debug, Clone)]
pub struct Table {
    pub column_names: Vec<String>,
    pub columns: Vec<Vec<Value>>,
}

impl Table {
    pub fn new(column_names: Vec<&str>) -> Self {
        Table {
            column_names: column_names.iter().map(|s| s.to_string()).collect(),
            columns: vec![Vec::new(); column_names.len()],
        }
    }
    pub fn column_index(&self, name: &str) -> Option<usize> {
        self.column_names.iter().position(|c| c == name)
    }
    /// Thêm một hàng. Số ô phải đúng bằng số cột — nếu không, các cột sẽ lệch
    /// độ dài và mọi phép tính sau đó sai âm thầm, nên ta dừng ngay.
    pub fn add_row(&mut self, row: Vec<Value>) {
        assert_eq!(
            row.len(),
            self.columns.len(),
            "số ô trong hàng phải bằng số cột"
        );
        for (i, value) in row.into_iter().enumerate() {
            self.columns[i].push(value);
        }
    }
    pub fn num_rows(&self) -> usize {
        self.columns.first().map(|c| c.len()).unwrap_or(0)
    }
    pub fn get(&self, row: usize, column: &str) -> Option<&Value> {
        self.column_index(column)
            .and_then(|c| self.columns[c].get(row))
    }
}

// ============================================================================
// 2. GIAI ĐOẠN E — EXTRACT: phân tích CSV thành bảng (chống dữ liệu bẩn)
// ============================================================================

/// Phân tích một dòng CSV đơn giản (không xử lý dấu ngoặc kép lồng nhau).
pub fn split_csv_line(line: &str) -> Vec<String> {
    line.split(',').map(|s| s.trim().to_string()).collect()
}

/// EXTRACT: đọc nhiều dòng thô -> Table, dùng filter_map để BỎ QUA dòng hỏng
/// (số cột sai). Đây là mẫu "làm sạch khi đọc" ở Chương 16.
pub fn extract_csv(lines: &[&str]) -> Result<Table, String> {
    let mut it = lines.iter();
    let header = it.next().ok_or("CSV rỗng")?;
    let column_names: Vec<&str> = header.split(',').map(|s| s.trim()).collect();
    let column_count = column_names.len();
    let mut table = Table::new(column_names);

    let rows = it
        .filter(|line| !line.trim().is_empty())
        .filter_map(|line| {
            let cells = split_csv_line(line);
            // Dòng lệch cột -> None -> bị bỏ qua, pipeline không sập
            (cells.len() == column_count)
                .then(|| cells.into_iter().map(infer_type).collect::<Vec<Value>>())
        });
    for row in rows {
        table.add_row(row);
    }
    Ok(table)
}

/// Suy kiểu: thử số trước, rỗng nếu là "", "NA", "null" hoặc "NaN", còn lại là chuỗi.
pub fn infer_type(cell: String) -> Value {
    let t = cell.trim();
    if t.is_empty() || t.eq_ignore_ascii_case("NA") || t.eq_ignore_ascii_case("null") {
        Value::Null
    } else if let Ok(n) = t.parse::<f64>() {
        // "NaN".parse::<f64>() THÀNH CÔNG — nhưng NaN làm hỏng mọi tổng/so sánh
        // phía sau, nên coi nó là giá trị thiếu.
        if n.is_nan() {
            Value::Null
        } else {
            Value::Number(n)
        }
    } else {
        Value::Text(t.to_string())
    }
}

// ============================================================================
// 3. GIAI ĐOẠN T — TRANSFORM: lọc, thêm cột dẫn xuất, xử lý giá trị thiếu
// ============================================================================

impl Table {
    /// Lọc hàng theo một vị từ trên hàng (Chương 15: closure làm tham số).
    pub fn filter(&self, keep: impl Fn(&HashMap<&str, &Value>) -> bool) -> Table {
        let mut new = Table::new(self.column_names.iter().map(|s| s.as_str()).collect());
        for h in 0..self.num_rows() {
            let row: HashMap<&str, &Value> = self
                .column_names
                .iter()
                .enumerate()
                .map(|(i, name)| (name.as_str(), &self.columns[i][h]))
                .collect();
            if keep(&row) {
                new.add_row(
                    (0..self.column_names.len())
                        .map(|i| self.columns[i][h].clone())
                        .collect(),
                );
            }
        }
        new
    }

    /// Điền giá trị thiếu trong một cột số bằng một hằng số.
    pub fn fill_missing(&mut self, column: &str, value: f64) {
        if let Some(c) = self.column_index(column) {
            for cell in self.columns[c].iter_mut() {
                if *cell == Value::Null {
                    *cell = Value::Number(value);
                }
            }
        }
    }
}

// ============================================================================
// 4. GIAI ĐOẠN L / PHÂN TÍCH — GROUP BY + AGGREGATE (trái tim của DA)
// ============================================================================

#[derive(Debug, Clone, PartialEq)]
pub struct GroupResult {
    pub key: String,
    pub count: usize,
    pub sum: f64,
    pub mean: f64,
    pub min: f64,
    pub max: f64,
}

impl Table {
    /// GROUP BY cột `by`, tính thống kê trên cột số `on`.
    /// Đây chính là VỊ NHÓM TÍCH ở Chương 18: gộp (đếm, tổng, min, max) trong 1 lượt.
    pub fn group_and_aggregate(&self, by: &str, on: &str) -> Vec<GroupResult> {
        let by_col = self.column_index(by).expect("cột nhóm không tồn tại");
        let on_col = self.column_index(on).expect("cột tính không tồn tại");

        // (tổng, đếm, min, max) cho mỗi khóa
        let mut acc: HashMap<String, (f64, usize, f64, f64)> = HashMap::new();
        for h in 0..self.num_rows() {
            let key = match &self.columns[by_col][h] {
                Value::Text(s) => s.clone(),
                Value::Number(n) => n.to_string(),
                Value::Null => "(thiếu)".to_string(),
            };
            if let Value::Number(v) = self.columns[on_col][h] {
                let e = acc
                    .entry(key)
                    .or_insert((0.0, 0, f64::INFINITY, f64::NEG_INFINITY));
                e.0 += v;
                e.1 += 1;
                e.2 = e.2.min(v);
                e.3 = e.3.max(v);
            }
        }
        let mut result: Vec<GroupResult> = acc
            .into_iter()
            .map(|(k, (sum, count, min, max))| GroupResult {
                key: k,
                count,
                sum,
                mean: sum / count as f64,
                min,
                max,
            })
            .collect();
        // sắp xếp tất định: theo tổng giảm dần, rồi theo khóa
        result.sort_by(|a, b| b.sum.total_cmp(&a.sum).then(a.key.cmp(&b.key)));
        result
    }
}

// ============================================================================
// 5. CHUỖI THỜI GIAN — WINDOW FUNCTION: trung bình trượt
// ============================================================================

/// Trung bình trượt cửa sổ `w` — mẫu cơ bản của phân tích chuỗi thời gian.
/// Dùng `slice::windows` (Chương 16): mỗi cửa sổ là một lát cắt MƯỢN, không sao
/// chép. Bản này cần cả dãy trong RAM và cộng lại `w` số mỗi bước (O(n·w)); bản
/// streaming thật giữ một tổng chạy (cộng phần tử vào, trừ phần tử ra) — O(1)/bước.
pub fn moving_average(data: &[f64], w: usize) -> Vec<f64> {
    if w == 0 || data.len() < w {
        return Vec::new();
    }
    data.windows(w)
        .map(|window| window.iter().sum::<f64>() / w as f64)
        .collect()
}

/// Phát hiện điểm bất thường: lệch quá `threshold` lần độ lệch chuẩn khỏi trung bình.
pub fn detect_anomalies(data: &[f64], threshold: f64) -> Vec<usize> {
    let n = data.len();
    if n == 0 {
        return Vec::new();
    }
    let mean = data.iter().sum::<f64>() / n as f64;
    let variance = data.iter().map(|x| (x - mean).powi(2)).sum::<f64>() / n as f64;
    let std_dev = variance.sqrt();
    if std_dev == 0.0 {
        return Vec::new();
    }
    data.iter()
        .enumerate()
        .filter(|&(_, &x)| (x - mean).abs() > threshold * std_dev)
        .map(|(i, _)| i)
        .collect()
}

// ============================================================================
// 6. JOIN — ghép hai bảng theo khóa chung
// ============================================================================

/// Inner join: chỉ giữ hàng có khóa khớp ở CẢ HAI bảng.
/// Như SQL: khóa NULL không khớp với gì cả (kể cả NULL).
pub fn inner_join(left: &Table, right: &Table, key: &str) -> Table {
    let left_key = left.column_index(key).expect("khóa không có ở bảng trái");
    let right_key = right.column_index(key).expect("khóa không có ở bảng phải");

    // Chỉ mục bảng phải theo khóa (băm) -> tra cứu O(1)
    let mut index: HashMap<String, Vec<usize>> = HashMap::new();
    for h in 0..right.num_rows() {
        let cell = &right.columns[right_key][h];
        if *cell != Value::Null {
            index.entry(format!("{:?}", cell)).or_default().push(h);
        }
    }

    // Cột kết quả: cột trái + cột phải (bỏ cột khóa trùng ở bảng phải)
    let mut names: Vec<String> = left.column_names.clone();
    for (i, t) in right.column_names.iter().enumerate() {
        if i != right_key {
            names.push(format!("{}_right", t));
        }
    }
    let mut result = Table::new(names.iter().map(|s| s.as_str()).collect());

    for h in 0..left.num_rows() {
        let cell = &left.columns[left_key][h];
        if *cell == Value::Null {
            continue;
        }
        if let Some(right_rows) = index.get(&format!("{:?}", cell)) {
            for &rr in right_rows {
                let mut row: Vec<Value> = (0..left.column_names.len())
                    .map(|i| left.columns[i][h].clone())
                    .collect();
                for i in 0..right.column_names.len() {
                    if i != right_key {
                        row.push(right.columns[i][rr].clone());
                    }
                }
                result.add_row(row);
            }
        }
    }
    result
}

fn main() {
    println!("═══════════════════════════════════════════════════════════════");
    println!("   KỸ NGHỆ DỮ LIỆU: EXTRACT → TRANSFORM → PHÂN TÍCH → JOIN     ");
    println!("═══════════════════════════════════════════════════════════════");

    let csv = vec![
        "date,region,revenue",
        "2026-01,Hà Nội,1500",
        "2026-01,TP.HCM,2200",
        "dòng hỏng thiếu cột",
        "2026-01,Hà Nội,800",
        "2026-02,TP.HCM,NA", // giá trị thiếu
        "2026-02,Hà Nội,1200",
        "2026-02,Đà Nẵng,600",
    ];
    let mut table = extract_csv(&csv).unwrap();
    println!(
        "\n1. EXTRACT: {} dòng hợp lệ (đã bỏ dòng lỗi + tiêu đề)",
        table.num_rows()
    );

    println!("\n2. TRANSFORM: điền giá trị thiếu bằng 0");
    table.fill_missing("revenue", 0.0);

    println!("\n3. PHÂN TÍCH: GROUP BY region, tổng hợp revenue");
    for r in table.group_and_aggregate("region", "revenue") {
        println!(
            "   {:<10} | {} bản ghi | tổng {:>6.0} | TB {:>6.1} | [{:.0}–{:.0}]",
            r.key, r.count, r.sum, r.mean, r.min, r.max
        );
    }

    println!("\n4. LỌC: chỉ giữ doanh thu > 1000");
    let big = table.filter(|h| {
        h["revenue"]
            .as_number()
            .map(|x| x > 1000.0)
            .unwrap_or(false)
    });
    println!("   Còn {} hàng", big.num_rows());

    println!("\n5. XỬ LÝ LUỒNG: trung bình trượt & phát hiện bất thường");
    let series = [10.0, 11.0, 9.0, 10.0, 50.0, 11.0, 10.0]; // 50 là điểm lạ
    println!(
        "   Trung bình trượt (w=3): {:?}",
        moving_average(&series, 3)
            .iter()
            .map(|x| (x * 10.0).round() / 10.0)
            .collect::<Vec<_>>()
    );
    println!(
        "   Vị trí bất thường (>2σ): {:?}",
        detect_anomalies(&series, 2.0)
    );

    println!("\n6. JOIN: ghép doanh thu với dân số khu vực");
    let mut population = Table::new(vec!["region", "population_millions"]);
    population.add_row(vec![Value::Text("Hà Nội".into()), Value::Number(8.4)]);
    population.add_row(vec![Value::Text("TP.HCM".into()), Value::Number(9.3)]);
    let joined = inner_join(&table, &population, "region");
    println!(
        "   Kết quả join có {} hàng, {} cột (Đà Nẵng bị loại vì không có dân số)",
        joined.num_rows(),
        joined.column_names.len()
    );

    println!("\n═══════════════════════════════════════════════════════════════");
    println!("   DỮ LIỆU DẠNG CỘT + ĐƯỜNG ỐNG HÀM = PHÂN TÍCH NHANH & AN TOÀN ");
    println!("═══════════════════════════════════════════════════════════════");
}

#[cfg(test)]
mod tests {
    use super::*;

    fn sample_table() -> Table {
        let csv = vec![
            "region,revenue",
            "A,100",
            "B,200",
            "A,50",
            "C,NA",
            "A,30",
            "B,80",
        ];
        extract_csv(&csv).unwrap()
    }

    #[test]
    fn extract_skips_bad_lines() {
        let csv = vec!["a,b", "1,2", "missing_column", "3,4"];
        let b = extract_csv(&csv).unwrap();
        assert_eq!(b.num_rows(), 2); // "missing_column" bị loại
    }

    #[test]
    fn type_inference_is_correct() {
        assert_eq!(infer_type("42".into()), Value::Number(42.0));
        assert_eq!(infer_type("2.5".into()), Value::Number(2.5));
        assert_eq!(infer_type("NaN".into()), Value::Null);
        assert_eq!(infer_type("Hà Nội".into()), Value::Text("Hà Nội".into()));
        assert_eq!(infer_type("".into()), Value::Null);
        assert_eq!(infer_type("NA".into()), Value::Null);
    }

    #[test]
    fn fills_missing_values() {
        let mut b = sample_table();
        b.fill_missing("revenue", 0.0);
        // Sau khi điền, cột 'revenue' không còn Null nào
        let c = b.column_index("revenue").unwrap();
        assert!(!b.columns[c].contains(&Value::Null));
    }

    #[test]
    fn group_by_aggregates_correctly() {
        let b = sample_table();
        let r = b.group_and_aggregate("region", "revenue");
        let a = r.iter().find(|x| x.key == "A").unwrap();
        assert_eq!(a.count, 3);
        assert_eq!(a.sum, 180.0);
        assert_eq!(a.mean, 60.0);
        assert_eq!(a.min, 30.0);
        assert_eq!(a.max, 100.0);
        let b_group = r.iter().find(|x| x.key == "B").unwrap();
        assert_eq!(b_group.sum, 280.0);
        // C chỉ có NA nên không xuất hiện (không có giá trị số nào)
        assert!(r.iter().find(|x| x.key == "C").is_none());
    }

    #[test]
    fn filters_by_predicate() {
        let b = sample_table();
        let l = b.filter(|h| {
            h["revenue"]
                .as_number()
                .map(|x| x >= 100.0)
                .unwrap_or(false)
        });
        assert_eq!(l.num_rows(), 2); // A=100, B=200
    }

    #[test]
    fn moving_average_is_correct() {
        assert_eq!(
            moving_average(&[1.0, 2.0, 3.0, 4.0], 2),
            vec![1.5, 2.5, 3.5]
        );
        assert_eq!(moving_average(&[1.0], 3), Vec::<f64>::new()); // ngắn hơn cửa sổ
        assert_eq!(moving_average(&[1.0, 2.0], 0), Vec::<f64>::new());
    }

    #[test]
    fn detects_outliers() {
        // Ở ngưỡng 1.5σ, điểm 100 bị phát hiện.
        let anomalies = detect_anomalies(&[10.0, 10.0, 10.0, 100.0, 10.0], 1.5);
        assert_eq!(anomalies, vec![3]);
        // BÀI HỌC THỐNG KÊ: cùng dữ liệu nhưng ở ngưỡng 2σ thì KHÔNG phát hiện,
        // vì một điểm cực lạ tự làm PHỒNG độ lệch chuẩn đến mức che chính nó.
        // Đây là lý do thống kê bền vững dùng trung vị + MAD thay cho TB + σ.
        assert!(detect_anomalies(&[10.0, 10.0, 10.0, 100.0, 10.0], 2.0).is_empty());
        // Dãy phẳng không có bất thường.
        assert!(detect_anomalies(&[5.0, 5.0, 5.0], 2.0).is_empty());
    }

    #[test]
    fn inner_join_keeps_only_matching_keys() {
        let mut t = Table::new(vec!["id", "name"]);
        t.add_row(vec![Value::Number(1.0), Value::Text("An".into())]);
        t.add_row(vec![Value::Number(2.0), Value::Text("Bình".into())]);
        t.add_row(vec![Value::Number(3.0), Value::Text("Chi".into())]);
        let mut p = Table::new(vec!["id", "score"]);
        p.add_row(vec![Value::Number(1.0), Value::Number(9.0)]);
        p.add_row(vec![Value::Number(2.0), Value::Number(8.0)]);
        // id=3 không có bên phải -> bị loại
        let j = inner_join(&t, &p, "id");
        assert_eq!(j.num_rows(), 2);
        assert_eq!(j.column_names.len(), 3); // id, ten, score_right
    }

    #[test]
    fn join_never_matches_null_keys() {
        let mut left = Table::new(vec!["id", "name"]);
        left.add_row(vec![Value::Null, Value::Text("Không mã".into())]);
        left.add_row(vec![Value::Number(1.0), Value::Text("An".into())]);
        let mut right = Table::new(vec!["id", "score"]);
        right.add_row(vec![Value::Null, Value::Number(5.0)]);
        right.add_row(vec![Value::Number(1.0), Value::Number(9.0)]);
        // Bản cũ so khóa bằng chuỗi Debug nên "Null" khớp "Null" -> 2 hàng
        assert_eq!(inner_join(&left, &right, "id").num_rows(), 1);
    }

    #[test]
    fn group_by_survives_nan_input() {
        // Bản cũ: "NaN" thành số NaN, tổng nhóm NaN, rồi sort_by(...unwrap()) panic
        let csv = vec!["region,revenue", "A,1", "A,NaN", "B,2"];
        let groups = extract_csv(&csv)
            .unwrap()
            .group_and_aggregate("region", "revenue");
        assert_eq!(groups.len(), 2);
        assert!(groups.iter().all(|g| !g.sum.is_nan()));
    }

    #[test]
    #[should_panic(expected = "số ô trong hàng phải bằng số cột")]
    fn add_row_rejects_wrong_width() {
        let mut t = Table::new(vec!["a", "b"]);
        t.add_row(vec![Value::Null]);
    }
}
