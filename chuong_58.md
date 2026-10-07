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
2. **Nén tốt hơn**: dữ liệu cùng kiểu, cùng phân phối nằm cạnh nhau nén hiệu quả (ví dụ cột "region" chỉ có 3 giá trị lặp lại → mã hóa từ điển/run-length nén xuống còn rất ít byte).
3. **Vector hóa (SIMD)**: dữ liệu liền nhau cho phép CPU dùng lệnh SIMD xử lý 4–8 số `f64` mỗi lệnh — rất khó nếu dữ liệu rải rác giữa các trường khác. (Lưu ý: với số thực, trình biên dịch *không* tự vector hóa một phép cộng dồn tuần tự vì cộng số thực không kết hợp — đổi thứ tự cộng là đổi kết quả làm tròn; thư viện như Polars/Arrow tự viết vòng cộng theo nhiều làn song song để được SIMD.)

### 2. ETL — ba giai đoạn, và "làm sạch khi đọc"

**Extract** là nơi dữ liệu bẩn nhất. Dòng thiếu cột, giá trị không phải số, ô rỗng — tất cả phải xử lý ngay ở cổng vào. Mẫu Rust idiomatic là `filter_map` (Chương 16): thử phân tích từng dòng, **bỏ qua** cái hỏng thay vì để cả pipeline sập. Đây chính là "hàm toàn phần" ở Chương 13 áp dụng vào dữ liệu — mọi đầu vào đều có đường xử lý, kể cả đầu vào rác.

**Transform** gồm lọc, thêm cột dẫn xuất, và **xử lý giá trị thiếu** (`Value::Null`). Quyết định điền thiếu bằng gì (0, trung bình, giá trị trước đó) là một quyết định *nghiệp vụ*, không phải kỹ thuật — và nó ảnh hưởng lớn tới kết quả phân tích.

### 3. GROUP BY chính là Vị nhóm Tích (Chương 18)

Đây là mối liên hệ đẹp nhất chương. Khi bạn `GROUP BY region` rồi tính `(đếm, tổng, min, max)`, bạn đang:
- Gộp các giá trị của cùng một khóa bằng một **vị nhóm tích bốn thành phần** (Chương 18): `(Sum, Count, Min, Max)`.
- Mỗi thành phần là một vị nhóm kết hợp, nên phép gộp **song song hóa được** — chia dữ liệu ra nhiều nhân, gộp từng phần, rồi ghép lại (đúng như `rayon` làm với `par_iter`).

Nói cách khác: cả một động cơ GROUP BY của cơ sở dữ liệu phân tích thực chất là *một fold trên một vị nhóm*. Toán học ở Chương 18 không phải trang trí — nó là kiến trúc.

### 4. Window function và chuỗi thời gian

**Trung bình trượt** làm mượt nhiễu và làm lộ xu hướng. Nó dùng `slice::windows(w)` của thư viện chuẩn — trượt một cửa sổ độ rộng cố định qua dãy. Với dữ liệu lớn thật, phiên bản streaming chỉ giữ cửa sổ hiện tại trong RAM (O(w) bộ nhớ) thay vì cả dãy, và cập nhật một tổng chạy thay vì cộng lại cả cửa sổ.

**Phát hiện bất thường** dựa trên độ lệch chuẩn có một cạm bẫy thống kê quan trọng mà bài kiểm thử trong chương này phơi bày: *một điểm cực lạ tự làm phồng độ lệch chuẩn đến mức che chính nó*. Đây là lý do thống kê bền vững (robust statistics) dùng **trung vị và MAD** (median absolute deviation) thay cho trung bình và σ.

### 5. Join bằng chỉ mục băm

Ghép hai bảng theo khóa chung. Cách ngây thơ là vòng lặp lồng (mỗi hàng trái quét cả bảng phải — O(N×M)). Cách đúng: xây **chỉ mục băm** trên bảng phải (`HashMap<khóa, danh sách hàng>`), rồi mỗi hàng trái tra O(1) trung bình. Một chi tiết ngữ nghĩa dễ quên: theo SQL, khóa `NULL` **không bao giờ** khớp, kể cả với `NULL` khác. Đây chính là **hash join** — thuật toán join phổ biến nhất trong các cơ sở dữ liệu thật, và nó dùng đúng bảng băm ở Chương 30.

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
/// Dùng `slice::windows` của thư viện chuẩn: mỗi cửa sổ là một lát cắt MƯỢN, không sao
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
        assert_eq!(j.column_names.len(), 3); // id, name, score_right
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
| `` E0507: cannot move out of index of `Vec<Value>` `` | Lấy `Value` ra khỏi cột bằng phép gán | `.clone()`, hoặc thao tác qua tham chiếu `&self.columns[c][i]` |
| `E0502: cannot borrow as mutable` | Duyệt cột này để ghi vào cột khác của cùng `Table` | Tính ra `Vec` mới rồi mới gán vào bảng |
| `` E0277: the trait bound `f64: Ord` is not satisfied `` | `sort()` trên cột số thực | `sort_by(\|a, b\| a.total_cmp(b))` — `f64` chỉ có thứ tự bộ phận vì `NaN`; `total_cmp` cho thứ tự toàn phần, còn `partial_cmp(..).unwrap()` sẽ panic khi gặp NaN |
| `` E0599: no method named `iter` found for enum `Value` `` | Nhầm `Value` (một ô) với cột | Lấy cột qua `column_index` rồi mới `iter()` |
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
`group_and_aggregate` tính tổng/trung bình/min/max. Trung vị bền vững hơn với điểm lạ. Viết `median(values: &[f64]) -> Option<f64>` (trả `None` cho dãy rỗng) và một bài test.

<details>
<summary><b>Lời giải</b></summary>

```rust
pub fn median(values: &[f64]) -> Option<f64> {
    if values.is_empty() { return None; }   // dãy rỗng không có trung vị
    let mut v = values.to_vec();
    v.sort_by(|a, b| a.total_cmp(b));       // không panic kể cả khi có NaN
    let n = v.len();
    Some(if n % 2 == 1 { v[n / 2] } else { (v[n / 2 - 1] + v[n / 2]) / 2.0 })
}

#[cfg(test)]
mod exercise_1 {
    use super::*;
    #[test]
    fn median_resists_outliers() {
        // Trung bình bị điểm 1000 kéo lên; trung vị thì không.
        let data = [1.0, 2.0, 3.0, 4.0, 1000.0];
        assert_eq!(median(&data), Some(3.0));   // ổn định
        let mean = data.iter().sum::<f64>() / 5.0;
        assert_eq!(mean, 202.0);                // bị bóp méo
        assert_eq!(median(&[]), None);
    }
}
```
</details>

**Bài tập 2 (Left join)**
`inner_join` chỉ giữ hàng khớp cả hai bên. Viết `left_join` giữ **mọi** hàng bảng trái; hàng không khớp thì các cột phải điền `Value::Null`. Test với "Đà Nẵng" không có dân số.

<details>
<summary><b>Gợi ý</b></summary>

Giống `inner_join` nhưng khi không tìm thấy khóa ở chỉ mục phải, vẫn thêm hàng trái và đệm `Value::Null` cho đủ số cột phải. Đây là join hay dùng nhất khi làm giàu (enrich) dữ liệu — giữ nguyên bảng chính, gắn thêm thông tin nếu có.
</details>

<details>
<summary><b>Lời giải</b></summary>

```rust
/// Left join: giữ MỌI hàng bảng trái; hàng không khớp thì các cột phải là `Null`.
/// Như SQL: khóa `NULL` bên trái không khớp gì, nhưng hàng đó VẪN được giữ.
pub fn left_join(left: &Table, right: &Table, key: &str) -> Table {
    let left_key = left.column_index(key).expect("khóa không có ở bảng trái");
    let right_key = right.column_index(key).expect("khóa không có ở bảng phải");

    // Chỉ mục băm trên bảng phải — y hệt `inner_join`
    let mut index: HashMap<String, Vec<usize>> = HashMap::new();
    for h in 0..right.num_rows() {
        let cell = &right.columns[right_key][h];
        if *cell != Value::Null {
            index.entry(format!("{:?}", cell)).or_default().push(h);
        }
    }

    let mut names: Vec<String> = left.column_names.clone();
    for (i, t) in right.column_names.iter().enumerate() {
        if i != right_key {
            names.push(format!("{}_right", t));
        }
    }
    let mut result = Table::new(names.iter().map(|s| s.as_str()).collect());
    let right_width = right.column_names.len() - 1; // bỏ cột khóa trùng

    for h in 0..left.num_rows() {
        let left_row: Vec<Value> = (0..left.column_names.len())
            .map(|i| left.columns[i][h].clone())
            .collect();
        let cell = &left.columns[left_key][h];
        let matches = if *cell == Value::Null {
            None // NULL không khớp gì
        } else {
            index.get(&format!("{:?}", cell))
        };
        match matches {
            Some(right_rows) => {
                for &rr in right_rows {
                    let mut row = left_row.clone();
                    for i in 0..right.column_names.len() {
                        if i != right_key {
                            row.push(right.columns[i][rr].clone());
                        }
                    }
                    result.add_row(row);
                }
            }
            // KHÁC inner_join ở đúng chỗ này: không khớp vẫn giữ, đệm Null cho đủ cột
            None => {
                let mut row = left_row;
                row.extend(std::iter::repeat_n(Value::Null, right_width));
                result.add_row(row);
            }
        }
    }
    result
}

#[cfg(test)]
mod exercise_2 {
    use super::*;
    #[test]
    fn left_join_keeps_unmatched_rows_with_null() {
        let mut cities = Table::new(vec!["city", "region"]);
        cities.add_row(vec![
            Value::Text("Hà Nội".into()),
            Value::Text("Bắc".into()),
        ]);
        cities.add_row(vec![
            Value::Text("Đà Nẵng".into()),
            Value::Text("Trung".into()),
        ]);
        let mut population = Table::new(vec!["city", "population"]);
        population.add_row(vec![Value::Text("Hà Nội".into()), Value::Number(8.4)]);

        let j = left_join(&cities, &population, "city");
        assert_eq!(j.num_rows(), 2); // inner_join chỉ cho 1 hàng
        assert_eq!(j.column_names, ["city", "region", "population_right"]);
        assert_eq!(j.get(0, "population_right"), Some(&Value::Number(8.4)));
        // Đà Nẵng không có dân số -> vẫn còn, cột phải là Null
        assert_eq!(j.get(1, "city"), Some(&Value::Text("Đà Nẵng".into())));
        assert_eq!(j.get(1, "population_right"), Some(&Value::Null));
        assert_eq!(inner_join(&cities, &population, "city").num_rows(), 1);
    }
}
```

Khác biệt duy nhất so với `inner_join` nằm ở nhánh `None`: hàng trái không tìm thấy đối tác vẫn được ghi ra, các cột phải đệm `Value::Null`. Vì thế số hàng của left join **luôn ≥** số hàng bảng trái (bằng nhau khi mỗi khóa khớp tối đa một hàng phải) — một bất biến đáng viết thành test khi làm giàu dữ liệu, vì nếu khóa phải bị trùng thì bảng kết quả sẽ "phình" âm thầm.
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
