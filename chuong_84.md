# Chương 84: Định lượng & Chênh lệch thống kê — Đồng liên kết, Kalman & Kiểm định tiến (OpenAlgo III)

## Giới thiệu & Mục tiêu học tập

Chương cuối của bộ ba OpenAlgo, và cũng là chương cuối của toàn bộ giáo trình. Nó nói về thứ khó nhất trong giao dịch định lượng: **phân biệt mẫu hình thật với mẫu hình do ngẫu nhiên tạo ra**.

Kết quả nổi bật của chương này là một thí nghiệm số cho thấy vì sao **tương quan cao không đủ để giao dịch cặp**:

```
cặp                       tương quan     hệ số kéo về     nửa chu kỳ
đồng liên kết thật            0,9893          −0,1907           3,6
chỉ tương quan cao            0,9772          −0,0198          35,0
```

Hai cặp có tương quan gần như bằng nhau (0,989 so với 0,977). Nhưng cặp thứ hai có nửa chu kỳ kéo về **gần gấp 10 lần** — nghĩa là chênh lệch của nó mất 35 phiên để về nửa đường, thay vì 3,6 phiên. Giao dịch nó sẽ giữ vị thế lâu gấp mười lần với cùng kỳ vọng lợi nhuận.

Đó là toàn bộ khác biệt giữa **tương quan** (hai chuỗi cùng đi lên xuống) và **đồng liên kết** (chênh lệch của chúng kéo về trung bình).

---

## Hình tượng hóa đời sống

```
┌──────────────────────────────────────────────────────────────────────────────┐
│  TƯƠNG QUAN vs ĐỒNG LIÊN KẾT                                                │
│                                                                              │
│   TƯƠNG QUAN = "hai người cùng đi lên đồi"                                  │
│     A: ↗ ↗ ↘ ↗ ↗        B: ↗ ↗ ↘ ↗ ↗                                       │
│     Cùng hướng mỗi bước. NHƯNG khoảng cách giữa họ có thể                   │
│     ngày càng xa mãi mãi.                                                   │
│                                                                              │
│   ĐỒNG LIÊN KẾT = "hai người bị buộc chung một sợi dây co giãn"            │
│     Đi đâu cũng được, nhưng khoảng cách LUÔN bị kéo về mức cũ.              │
│     ĐÂY mới là thứ giao dịch được.                                          │
│                                                                              │
│   ⚠ Hai chuỗi có thể tương quan 0,99 mà KHÔNG đồng liên kết.                │
│     Và có thể đồng liên kết mà tương quan thấp.                             │
│                                                                              │
│  NỬA CHU KỲ = "BAO LÂU THÌ CHÊNH LỆCH ĐI ĐƯỢC NỬA ĐƯỜNG VỀ?"               │
│                                                                              │
│    chênh lệch                                                                │
│       │╲                                                                     │
│       │ ╲___                nửa chu kỳ 3,6 phiên → giao dịch được           │
│       │     ╲______                                                          │
│    0  ├───────────────────                                                  │
│                                                                              │
│       │╲                                                                     │
│       │ ╲                   nửa chu kỳ 35 phiên → vốn bị kẹt quá lâu        │
│       │  ╲╲                                                                  │
│    0  ├────╲╲╲╲╲___________                                                 │
│                                                                              │
│  BỘ LỌC KALMAN = TIN TƯỞNG CÓ ĐIỀU KIỆN                                     │
│                                                                              │
│    Bạn đoán: 100 (khá chắc)     Đo được: 110 (nhiễu nhiều)                  │
│    → Kalman nghiêng về dự đoán:  102                                        │
│                                                                              │
│    Bạn đoán: 100 (không chắc)   Đo được: 110 (rất chính xác)                │
│    → Kalman nghiêng về phép đo:  109                                        │
│                                                                              │
│    Trọng số TỰ ĐIỀU CHỈNH theo độ tin cậy tương đối. Không tham số ma thuật.│
│                                                                              │
│  KIỂM ĐỊNH TIẾN = KHÔNG BAO GIỜ KIỂM TRÊN DỮ LIỆU ĐÃ TỐI ƯU                │
│                                                                              │
│    [══ luyện ══][ kiểm ]                                                    │
│           [══ luyện ══][ kiểm ]                                             │
│                  [══ luyện ══][ kiểm ]                                      │
│                                                                              │
│    Mỗi giai đoạn kiểm là dữ liệu HOÀN TOÀN MỚI với tham số vừa chọn.        │
│    Đây là cách duy nhất phát hiện quá khớp trước khi mất tiền thật.         │
└──────────────────────────────────────────────────────────────────────────────┘
```

---

## Khái niệm & Cơ chế kỹ thuật chuyên sâu

### 1. Vì sao đồng liên kết mới là điều kiện đúng

Giao dịch cặp đặt cược rằng chênh lệch giữa hai tài sản sẽ quay về trung bình. Nếu chênh lệch **không** kéo về, đó không phải giao dịch — đó là một vị thế mở vô thời hạn.

Tương quan đo **đồng chuyển động của lợi suất**: hai chuỗi có xu hướng cùng tăng cùng giảm. Nhưng hai chuỗi ngẫu nhiên đều có xu hướng đi lên vẫn cho tương quan cao mà chênh lệch của chúng đi lang thang không giới hạn.

Đồng liên kết (Cointegration) đo **tính dừng của phần dư**. Đó mới là điều kiện cần cho giao dịch cặp.

Kiểm định thực hiện bằng hai bước:
1. Hồi quy `y` theo `x` để tìm tỉ lệ phòng vệ.
2. Kiểm tra phần dư có dừng không, bằng cách hồi quy `Δphần_dư` theo `phần_dư` và xét hệ số. Hệ số âm rõ rệt nghĩa là có lực kéo về.

Từ hệ số đó suy ra **nửa chu kỳ**: `ln(2) / |λ|`. Đây là con số quan trọng nhất cho quyết định giao dịch — nó cho biết vốn của bạn sẽ bị kẹt bao lâu.

### 2. Kalman: tỉ lệ phòng vệ thay đổi theo thời gian

Hồi quy tuyến tính (Linear regression) cho một tỉ lệ phòng vệ **cố định**. Nhưng quan hệ giữa hai tài sản thay đổi theo thời gian — thay đổi cơ cấu ngành, thay đổi thanh khoản, thay đổi vốn hoá.

Cửa sổ trượt (Sliding window) là cách chữa thô: chọn cửa sổ ngắn thì nhiễu, chọn dài thì chậm phản ứng.

Kalman làm điều đó một cách có nguyên tắc. Nó duy trì cả **ước lượng** lẫn **độ bất định** của ước lượng, và cập nhật cả hai theo quy tắc Bayes. Khi phép đo mâu thuẫn với dự đoán, độ bất định tăng, và bộ lọc tự động tin phép đo hơn.

Hai tham số cần chọn: nhiễu quá trình `Q` (tham số thay đổi nhanh thế nào) và nhiễu đo `R` (phép đo nhiễu thế nào). Tỉ lệ `Q/R` quyết định bộ lọc thích ứng nhanh hay chậm — nó chính là "cửa sổ" nhưng ở dạng liên tục và có ý nghĩa xác suất.

### 3. VaR và Thiếu hụt kỳ vọng: một cái thiếu, một cái đủ

**VaR 95%** trả lời: "5% ngày tệ nhất bắt đầu từ mức lỗ nào?"

Nhưng nó **không nói** 5% đó tệ đến đâu. Hai danh mục có cùng VaR có thể có mức lỗ thảm hoạ khác nhau hàng chục lần.

**Thiếu hụt kỳ vọng (Expected Shortfall)** (còn gọi là VaR có điều kiện) trả lời câu hỏi đúng: "trong 5% ngày tệ nhất, trung bình lỗ bao nhiêu?"

Có một lý do toán học sâu hơn để ưa ES: nó là **thước đo rủi ro nhất quán**, còn VaR thì không. Cụ thể, VaR vi phạm **tính dưới cộng tính** — VaR của danh mục gộp có thể lớn hơn tổng VaR của các phần. Điều đó vô lý về mặt kinh tế (đa dạng hoá không thể làm tăng rủi ro), và nó là lý do Basel III chuyển từ VaR sang ES.

### 4. Quá khớp: cái bẫy không thể tránh, chỉ có thể đo

Nếu bạn thử 1000 tổ hợp tham số trên cùng một tập dữ liệu, bạn **sẽ** tìm được tổ hợp trông tuyệt vời — kể cả khi dữ liệu hoàn toàn ngẫu nhiên. Đó là toán học, không phải xui xẻo.

Chương này chứng minh điều đó bằng thí nghiệm: sinh nhiễu tất định (không có tín hiệu nào cả), quét tham số, và cho thấy tham số "tốt nhất" trên dữ liệu luyện có hiệu suất **không tốt hơn ngẫu nhiên** trên dữ liệu kiểm.

*(Ghi chú kỹ thuật: bản đầu của thí nghiệm này dùng số học modulo để sinh nhiễu, và các giá trị bị co cụm khiến việc chọn max không thể hiện được hiện tượng quá khớp. Đã thay bằng hàm băm splitmix64 để phân phối đều thật sự.)*

Ba phòng vệ, theo thứ tự sức mạnh:
- **Kiểm định tiến (Walk-forward validation)**: kiểm trên dữ liệu chưa từng dùng để chọn tham số.
- **Ít tham số hơn**: mỗi tham số thêm vào là một bậc tự do để quá khớp.
- **Có giả thuyết kinh tế trước**: nếu bạn không giải thích được **vì sao** chiến lược nên hoạt động, có lẽ nó không hoạt động.

### 5. Danh mục hiệu quả và giới hạn của nó

Lý thuyết Markowitz cho danh mục có phương sai nhỏ nhất với một mức lợi suất cho trước. Về mặt toán học nó đẹp. Về mặt thực hành nó nổi tiếng là bất ổn: sai số nhỏ trong ước lượng lợi suất kỳ vọng gây thay đổi lớn trong trọng số tối ưu.

Các phương pháp thực tế đều là biến thể "làm cùn" mô hình: ràng buộc trọng số, co ma trận hiệp phương sai (Ledoit–Wolf), hoặc bỏ hẳn ước lượng lợi suất và chỉ dùng rủi ro (danh mục phương sai nhỏ nhất, ngang bằng rủi ro).

Đây là bài học chung của toàn chương, và cũng là kết luận thích hợp cho cả giáo trình: **một mô hình đúng với dữ liệu bạn có, chưa chắc đúng với dữ liệu bạn sẽ gặp.**

---

## Mã nguồn minh họa thực chiến

Chạy bằng `cargo run -p ch84`, kiểm thử bằng `cargo test -p ch84`.

```rust
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

pub fn mean(x: &[f64]) -> f64 {
    if x.is_empty() { return 0.0; }
    x.iter().sum::<f64>() / x.len() as f64
}

/// Phương sai MẪU (chia n−1). Dùng n−1 vì ta ước lượng trung bình từ chính
/// dữ liệu, nên mất một bậc tự do — chia n sẽ cho ước lượng thiên lệch thấp.
pub fn variance(x: &[f64]) -> f64 {
    if x.len() < 2 { return 0.0; }
    let tb = mean(x);
    x.iter().map(|v| (v - tb).powi(2)).sum::<f64>() / (x.len() - 1) as f64
}

pub fn stddev(x: &[f64]) -> f64 { variance(x).sqrt() }

pub fn covariance(x: &[f64], y: &[f64]) -> f64 {
    let n = x.len().min(y.len());
    if n < 2 { return 0.0; }
    let (tx, ty) = (mean(&x[..n]), mean(&y[..n]));
    x[..n].iter().zip(y[..n].iter())
        .map(|(a, b)| (a - tx) * (b - ty)).sum::<f64>() / (n - 1) as f64
}

/// Hệ số tương quan Pearson, luôn nằm trong [−1, 1].
pub fn correlation(x: &[f64], y: &[f64]) -> Option<f64> {
    let n = x.len().min(y.len());
    let (sx, sy) = (stddev(&x[..n]), stddev(&y[..n]));
    if sx < 1e-12 || sy < 1e-12 { return None; }
    Some((covariance(x, y) / (sx * sy)).clamp(-1.0, 1.0))
}

// ============================================================================
// 2. HỒI QUY TUYẾN TÍNH
// ============================================================================

#[derive(Debug, Clone, Copy, PartialEq)]
pub struct ResultRegression {
    /// Hệ số góc — trong tài chính gọi là beta, hay TỈ LỆ PHÒNG HỘ.
    pub beta: f64,
    /// Hệ số chặn — phần lợi suất không giải thích được bằng biến kia.
    pub alpha: f64,
    /// Tỉ lệ phương sai được giải thích, trong [0, 1].
    pub r_squared: f64,
    /// Độ lệch chuẩn của phần dư.
    pub sai_num_standard: f64,
    pub so_quan_sat: usize,
}

/// Hồi quy bình phương tối thiểu: y = alpha + beta·x + nhiễu.
pub fn regression(x: &[f64], y: &[f64]) -> Option<ResultRegression> {
    let n = x.len().min(y.len());
    if n < 3 { return None; }
    let vx = variance(&x[..n]);
    if vx < 1e-12 { return None; } // x không đổi thì không có hệ số góc

    let beta = covariance(&x[..n], &y[..n]) / vx;
    let alpha = mean(&y[..n]) - beta * mean(&x[..n]);
    let du: Vec<f64> = (0..n).map(|i| y[i] - (alpha + beta * x[i])).collect();
    let vy = variance(&y[..n]);
    let r2 = if vy < 1e-12 { 0.0 } else { (1.0 - variance(&du) / vy).clamp(0.0, 1.0) };
    Some(ResultRegression { beta, alpha, r_squared: r2,
                        sai_num_standard: stddev(&du), so_quan_sat: n })
}

/// Phần dư của hồi quy — chính là CHÊNH LỆCH mà arbitrage cặp giao dịch.
pub fn part_data(x: &[f64], y: &[f64], kq: &ResultRegression) -> Vec<f64> {
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
    pub reversion_coef: f64,
    /// Nửa chu kỳ: bao nhiêu bước để chênh lệch co lại một nửa.
    pub half_life: f64,
    pub has_cointegration: bool,
}

pub fn cointegration_test(spread: &[f64], threshold: f64)
    -> Option<KetQuaDongLienKet>
{
    if spread.len() < 20 { return None; }
    let e: Vec<f64> = spread[..spread.len() - 1].to_vec();
    let de: Vec<f64> = spread.windows(2).map(|w| w[1] - w[0]).collect();
    let hq = regression(&e, &de)?;
    let lambda = hq.beta;
    // Chênh lệch co lại theo e^(λt); nửa chu kỳ là khi e^(λt) = 1/2
    let half = if lambda < -1e-12 { (0.5f64).ln() / lambda } else { f64::INFINITY };
    Some(KetQuaDongLienKet {
        reversion_coef: lambda,
        half_life: half,
        has_cointegration: lambda < threshold,
    })
}

// ============================================================================
// 4. LỌC KALMAN CHO TỈ LỆ PHÒNG HỘ ĐỘNG
// ============================================================================
// Hồi quy cho MỘT beta cố định cho cả giai đoạn. Nhưng quan hệ giữa hai mã
// trôi theo thời gian. Lọc Kalman cập nhật beta sau MỖI quan sát, cân bằng
// giữa "tin dữ liệu mới" và "tin ước lượng cũ".

#[derive(Debug, Clone, Copy, PartialEq)]
pub struct KalmanFilter {
    /// Ước lượng beta hiện tại.
    pub beta: f64,
    /// Độ bất định của ước lượng. Càng lớn càng sẵn sàng đổi ý.
    pub estimated_variance: f64,
    /// Mức trôi của beta giữa hai bước (nhiễu quá trình).
    pub process_noise: f64,
    /// Mức nhiễu của quan sát. Càng lớn càng ít tin dữ liệu mới.
    pub observation_noise: f64,
    pub num_step: usize,
}

impl KalmanFilter {
    pub fn new(beta_dau: f64, process_noise: f64, observation_noise: f64) -> Self {
        KalmanFilter { beta: beta_dau, estimated_variance: 1.0,
                    process_noise, observation_noise, num_step: 0 }
    }

    /// Cập nhật với một cặp quan sát (x, y). Trả về sai số dự báo — chính là
    /// tín hiệu giao dịch: y lệch bao nhiêu so với mức beta·x dự đoán.
    pub fn update(&mut self, x: f64, y: f64) -> f64 {
        // Dự đoán: beta không đổi, nhưng độ bất định lớn thêm
        let p_truoc = self.estimated_variance + self.process_noise;
        // Sai số dự báo
        let sai_so = y - self.beta * x;
        // Độ lợi Kalman: dữ liệu mới càng đáng tin thì càng gần 1
        let s = x * x * p_truoc + self.observation_noise;
        let k = if s.abs() < 1e-12 { 0.0 } else { p_truoc * x / s };
        self.beta += k * sai_so;
        self.estimated_variance = (1.0 - k * x) * p_truoc;
        self.num_step += 1;
        sai_so
    }
}

// ============================================================================
// 5. DANH MỤC TRUNG BÌNH – PHƯƠNG SAI
// ============================================================================

#[derive(Debug, Clone, Copy, PartialEq)]
pub struct PortfolioStats {
    pub expected_return: f64,
    pub stddev: f64,
    /// Lợi suất trên mỗi đơn vị rủi ro.
    pub sharpe_ratio: f64,
}

/// Rủi ro của danh mục KHÔNG phải trung bình rủi ro các thành phần — nó phụ
/// thuộc tương quan. Đây là toàn bộ ý nghĩa của đa dạng hoá, và là "bữa trưa
/// miễn phí" duy nhất trong tài chính.
pub fn portfolio_stats(loi_suat: &[Vec<f64>], weight: &[f64], phi_rui_ro: f64)
    -> Option<PortfolioStats>
{
    let n = loi_suat.len();
    if n == 0 || weight.len() != n { return None; }
    let ls_ky_vong: f64 = (0..n).map(|i| weight[i] * mean(&loi_suat[i])).sum();

    // Phương sai danh mục = Σᵢ Σⱼ wᵢ wⱼ Cov(i, j)
    let mut ps = 0.0;
    for i in 0..n {
        for j in 0..n {
            ps += weight[i] * weight[j] * covariance(&loi_suat[i], &loi_suat[j]);
        }
    }
    let sd = ps.max(0.0).sqrt();
    Some(PortfolioStats {
        expected_return: ls_ky_vong,
        stddev: sd,
        sharpe_ratio: if sd < 1e-12 { 0.0 } else { (ls_ky_vong - phi_rui_ro) / sd },
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
pub fn value_at_risk(loi_suat: &[f64], muc_tin_cay: f64) -> Option<f64> {
    if loi_suat.is_empty() || !(0.0..1.0).contains(&muc_tin_cay) { return None; }
    let mut s = loi_suat.to_vec();
    s.sort_by(|a, b| a.partial_cmp(b).unwrap());
    let i = (((1.0 - muc_tin_cay) * s.len() as f64).floor() as usize).min(s.len() - 1);
    Some(-s[i])
}

/// Thiếu hụt kỳ vọng: lỗ TRUNG BÌNH trong những phiên tệ nhất.
/// Đây là câu trả lời cho câu hỏi mà VaR né tránh: "khi vượt ngưỡng thì tệ
/// tới mức nào?" Nó luôn ≥ VaR, và là thước đo mà quy định hiện đại dùng.
pub fn expected_shortfall(loi_suat: &[f64], muc_tin_cay: f64) -> Option<f64> {
    if loi_suat.is_empty() || !(0.0..1.0).contains(&muc_tin_cay) { return None; }
    let mut s = loi_suat.to_vec();
    s.sort_by(|a, b| a.partial_cmp(b).unwrap());
    let k = (((1.0 - muc_tin_cay) * s.len() as f64).ceil() as usize).clamp(1, s.len());
    Some(-mean(&s[..k]))
}

// ============================================================================
// 7. KIỂM ĐỊNH TIẾN — chống khớp quá mức
// ============================================================================
// Tối ưu tham số trên toàn bộ dữ liệu rồi khoe kết quả là tự lừa mình. Kiểm
// định tiến chia dữ liệu thành nhiều đoạn: chọn tham số trên đoạn TRONG MẪU,
// rồi chấm điểm trên đoạn NGOÀI MẪU ngay sau đó — mô phỏng đúng cách ta thật
// sự giao dịch: chỉ biết quá khứ.

#[derive(Debug, Clone, PartialEq)]
pub struct TestSegment {
    pub query_param: usize,
    pub point_in_mau: f64,
    pub point_out_mau: f64,
}

#[derive(Debug, PartialEq)]
pub struct ResultWalkForward {
    pub segments: Vec<TestSegment>,
    pub mean_in_mau: f64,
    pub mean_out_mau: f64,
    /// Mức tụt điểm khi ra ngoài mẫu. Tụt nhiều = đã khớp vào nhiễu.
    pub level_drawdown: f64,
}

/// `cham_diem(param, tu, den)` chấm điểm một tham số trên đoạn `[tu, den)`.
pub fn walk_forward<F>(
    total_do_long: usize, do_dai_trong_mau: usize, do_dai_ngoai_mau: usize,
    all_params: &[usize], mut cham_diem: F,
) -> ResultWalkForward
where F: FnMut(usize, usize, usize) -> f64
{
    let mut segments = Vec::new();
    if all_params.is_empty() || do_dai_ngoai_mau == 0 {
        return ResultWalkForward { segments, mean_in_mau: 0.0,
                                    mean_out_mau: 0.0, level_drawdown: 0.0 };
    }
    let mut first = 0usize;
    while first + do_dai_trong_mau + do_dai_ngoai_mau <= total_do_long {
        let done_in = first + do_dai_trong_mau;
        let done_out = done_in + do_dai_ngoai_mau;
        // Chọn tham số CHỈ dựa trên đoạn trong mẫu
        let (best, diem_trong) = all_params.iter()
            .map(|&p| (p, cham_diem(p, first, done_in)))
            .fold((all_params[0], f64::MIN), |a, b| if b.1 > a.1 { b } else { a });
        // Rồi chấm nó trên đoạn ngoài mẫu ngay sau
        let point_out = cham_diem(best, done_in, done_out);
        segments.push(TestSegment { query_param: best,
                                     point_in_mau: diem_trong,
                                     point_out_mau: point_out });
        first += do_dai_ngoai_mau;
    }
    let avg_in = mean(&segments.iter().map(|d| d.point_in_mau)
                                       .collect::<Vec<_>>());
    let avg_out = mean(&segments.iter().map(|d| d.point_out_mau)
                                       .collect::<Vec<_>>());
    ResultWalkForward {
        segments, mean_in_mau: avg_in, mean_out_mau: avg_out,
        level_drawdown: avg_in - avg_out,
    }
}

/// Nhiễu tất định trải đều trong [−1, 1), băm từ (đoạn, tham số).
/// Dùng splitmix64 thay vì số học modulo thô: modulo thô làm các giá trị co
/// cụm, và khi đó "chọn tối đa" không còn thật sự khớp vào nhiễu nữa.
pub fn deterministic_noise(doan: usize, param: usize) -> f64 {
    let mut z = (doan as u64)
        .wrapping_mul(0x9E37_79B9_7F4A_7C15)
        .wrapping_add((param as u64).wrapping_mul(0xBF58_476D_1CE4_E5B9));
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
pub fn gen_cap_price_cointegration(n: usize, hat_giong: u64) -> (Vec<f64>, Vec<f64>) {
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

pub fn gen_returns(n: usize, hat_giong: u64, do_lech: f64, expectation: f64) -> Vec<f64> {
    let mut s = hat_giong;
    (0..n).map(|_| {
        s = s.wrapping_mul(6364136223846793005).wrapping_add(1442695040888963407);
        // Tổng 3 biến đều → xấp xỉ phân phối chuẩn (định lý giới hạn trung tâm)
        let u: f64 = (0..3).map(|k| ((s >> (20 + k * 12)) % 1000) as f64 / 1000.0).sum();
        expectation + (u - 1.5) * do_lech
    }).collect()
}

fn main() {
    println!("═══════════════════════════════════════════════════════════");
    println!("   GIAO DỊCH ĐỊNH LƯỢNG & ARBITRAGE THỐNG KÊ (OpenAlgo)     ");
    println!("═══════════════════════════════════════════════════════════");

    println!("\n1. HỒI QUY & TỈ LỆ PHÒNG HỘ");
    let (a, b) = sinh_cap_dong_lien_ket(1_000, 2024, 1.5);
    let hq = regression(&a, &b).unwrap();
    println!("   beta {:.4} (đúng phải là 1.5) · alpha {:.4} · R² {:.4}",
             hq.beta, hq.alpha, hq.r_squared);
    println!("   Tương quan: {:.4}", correlation(&a, &b).unwrap());
    println!("   → beta chính là số lượng mã B cần bán khi mua 1 mã A để trung hoà.");

    println!("\n2. TƯƠNG QUAN CAO KHÔNG BẰNG ĐỒNG LIÊN KẾT");
    let (c, d) = gen_cap_price_cointegration(1_000, 7);
    println!("   {:<24} {:>12} {:>16} {:>14}",
             "cặp", "tương quan", "hệ số kéo về", "nửa chu kỳ");
    for (name, x, y) in [("đồng liên kết thật", &a, &b),
                        ("chỉ tương quan cao", &c, &d)] {
        let h = regression(x, y).unwrap();
        let e = part_data(x, y, &h);
        let dlk = cointegration_test(&e, -0.05).unwrap();
        println!("   {:<24} {:>12.4} {:>16.4} {:>14.1}",
                 name, correlation(x, y).unwrap(), dlk.reversion_coef, dlk.half_life);
    }
    println!("   → CẢ HAI đều tương quan rất cao. Nhưng chỉ cặp đầu có chênh lệch");
    println!("     quay về trung bình. Giao dịch cặp thứ hai là thua chắc.");

    println!("\n3. LỌC KALMAN — beta trôi theo thời gian");
    let mut lk = KalmanFilter::new(1.0, 1e-5, 1.0);
    println!("   {:>10} {:>16}", "bước", "beta ước lượng");
    for (i, (&x, &y)) in a.iter().zip(b.iter()).enumerate() {
        lk.update(x, y);
        if [0usize, 10, 50, 200, 999].contains(&i) {
            println!("   {:>10} {:>16.4}", i, lk.beta);
        }
    }
    println!("   → Xuất phát từ 1.0 và tự tìm về {:.3} mà không cần biết trước.", lk.beta);

    println!("\n4. ĐA DẠNG HOÁ — bữa trưa miễn phí duy nhất");
    let ls_a = gen_returns(1_000, 1, 0.02, 0.0005);
    let ls_b = gen_returns(1_000, 999, 0.02, 0.0005);
    let mot_ma = portfolio_stats(&[ls_a.clone()], &[1.0], 0.0).unwrap();
    let two_id = portfolio_stats(&[ls_a.clone(), ls_b.clone()], &[0.5, 0.5], 0.0).unwrap();
    println!("   Chỉ mã A    : lợi suất {:.5} · rủi ro {:.5} · Sharpe {:.3}",
             mot_ma.expected_return, mot_ma.stddev, mot_ma.sharpe_ratio);
    println!("   Nửa A nửa B : lợi suất {:.5} · rủi ro {:.5} · Sharpe {:.3}",
             two_id.expected_return, two_id.stddev, two_id.sharpe_ratio);
    println!("   → Lợi suất kỳ vọng gần như không đổi, nhưng rủi ro giảm {:.0}%.",
             (1.0 - two_id.stddev / mot_ma.stddev) * 100.0);
    println!("     Đó là vì hai mã không tương quan hoàn toàn.");

    println!("\n5. RỦI RO ĐUÔI");
    let ls = gen_returns(5_000, 42, 0.02, 0.0003);
    println!("   {:>14} {:>14} {:>22}", "mức tin cậy", "VaR", "thiếu hụt kỳ vọng");
    for mtc in [0.90f64, 0.95, 0.99] {
        println!("   {:>13.0}% {:>14.5} {:>22.5}", mtc * 100.0,
                 value_at_risk(&ls, mtc).unwrap(),
                 expected_shortfall(&ls, mtc).unwrap());
    }
    println!("   → Thiếu hụt kỳ vọng LUÔN lớn hơn VaR. VaR nói \"95% thời gian bạn");
    println!("     không lỗ quá X\"; nó im lặng về 5% còn lại — mà đó mới là chỗ chết.");

    println!("\n6. KIỂM ĐỊNH TIẾN — phát hiện khớp quá mức");
    // Hàm chấm điểm giả: có một tham số "thật sự tốt" (20) cộng nhiễu phụ
    // thuộc đoạn dữ liệu. Tối ưu trên nhiễu chính là khớp quá mức.
    let cham = |p: usize, tu: usize, _den: usize| -> f64 {
        let candle = if p == 20 { 1.0 } else { 0.3 };
        candle + deterministic_noise(tu, p) * 0.8
    };
    let kq = walk_forward(1_000, 200, 100, &[5, 10, 20, 50, 100], cham);
    println!("   {:>8} {:>16} {:>18} {:>18}",
             "đoạn", "tham số chọn", "điểm trong mẫu", "điểm ngoài mẫu");
    for (i, d) in kq.segments.iter().enumerate() {
        println!("   {:>8} {:>16} {:>18.3} {:>18.3}",
                 i + 1, d.query_param, d.point_in_mau, d.point_out_mau);
    }
    println!("   Trung bình trong mẫu {:.3} · ngoài mẫu {:.3} · SỤT {:.3}",
             kq.mean_in_mau, kq.mean_out_mau, kq.level_drawdown);
    println!("   → Điểm trong mẫu luôn đẹp hơn, vì ta ĐÃ CHỌN tham số cho nó.");
    println!("     Chỉ điểm ngoài mẫu mới là con số đáng tin.");

    println!("\n═══════════════════════════════════════════════════════════");
    println!("   THỐNG KÊ TÀI CHÍNH DỄ NÓI DỐI. LUÔN HỎI: CÒN NGOÀI MẪU?  ");
    println!("═══════════════════════════════════════════════════════════");
}

#[cfg(test)]
mod tests {
    use super::*;

    // ---------- Thống kê nền ----------
    #[test]
    fn thong_ke_has_sell_use() {
        let x = vec![2.0, 4.0, 4.0, 4.0, 5.0, 5.0, 7.0, 9.0];
        assert!((mean(&x) - 5.0).abs() < 1e-12);
        // Phương sai MẪU (chia n−1) của dãy này là 32/7
        assert!((variance(&x) - 32.0 / 7.0).abs() < 1e-12);
        assert!((stddev(&x) - (32.0f64 / 7.0).sqrt()).abs() < 1e-12);
    }

    #[test]
    fn stats_on_too_little_data_do_not_panic() {
        assert_eq!(mean(&[]), 0.0);
        assert_eq!(variance(&[]), 0.0);
        assert_eq!(variance(&[5.0]), 0.0, "một điểm thì không có phương sai mẫu");
        assert_eq!(covariance(&[1.0], &[2.0]), 0.0);
    }

    #[test]
    fn correlation_is_one_for_a_perfect_linear_relation() {
        let x: Vec<f64> = (1..=100).map(|i| i as f64).collect();
        let tang: Vec<f64> = x.iter().map(|v| 3.0 * v + 7.0).collect();
        let down: Vec<f64> = x.iter().map(|v| -2.0 * v + 5.0).collect();
        assert!((correlation(&x, &tang).unwrap() - 1.0).abs() < 1e-9);
        assert!((correlation(&x, &down).unwrap() + 1.0).abs() < 1e-9);
    }

    #[test]
    fn correlation_stays_within_minus_one_and_one() {
        for hat in [1u64, 42, 2024] {
            let a = gen_returns(500, hat, 0.02, 0.0);
            let b = gen_returns(500, hat + 1000, 0.02, 0.0);
            let r = correlation(&a, &b).unwrap();
            assert!((-1.0..=1.0).contains(&r), "tương quan {} ra ngoài khoảng", r);
        }
    }

    #[test]
    fn a_constant_series_has_undefined_correlation() {
        let queue = vec![5.0; 100];
        let x: Vec<f64> = (1..=100).map(|i| i as f64).collect();
        assert_eq!(correlation(&queue, &x), None, "không chia cho độ lệch bằng 0");
    }

    // ---------- Hồi quy ----------
    #[test]
    fn regression_recovers_the_coefficients_without_noise() {
        let x: Vec<f64> = (1..=100).map(|i| i as f64).collect();
        let y: Vec<f64> = x.iter().map(|v| 2.5 * v + 10.0).collect();
        let h = regression(&x, &y).unwrap();
        assert!((h.beta - 2.5).abs() < 1e-9);
        assert!((h.alpha - 10.0).abs() < 1e-9);
        assert!((h.r_squared - 1.0).abs() < 1e-9, "khớp hoàn hảo → R² = 1");
        assert!(h.sai_num_standard < 1e-9);
    }

    #[test]
    fn r_squared_stays_within_zero_and_one() {
        for hat in [1u64, 7, 42, 2024] {
            let (a, b) = sinh_cap_dong_lien_ket(500, hat, 1.5);
            let h = regression(&a, &b).unwrap();
            assert!((0.0..=1.0).contains(&h.r_squared));
        }
    }

    #[test]
    fn regression_returns_none_on_degenerate_input() {
        assert_eq!(regression(&[1.0, 2.0], &[1.0, 2.0]), None, "cần ít nhất 3 điểm");
        assert_eq!(regression(&[5.0; 10], &[1.0; 10]), None, "x không đổi thì vô nghĩa");
    }

    #[test]
    fn residuals_have_zero_mean() {
        // Tính chất toán học của bình phương tối thiểu. Nếu không đúng thì
        // hồi quy đã cài sai.
        let (a, b) = sinh_cap_dong_lien_ket(500, 11, 1.5);
        let h = regression(&a, &b).unwrap();
        let e = part_data(&a, &b, &h);
        assert!(mean(&e).abs() < 1e-9,
                "trung bình phần dư {:.2e}", mean(&e));
    }

    #[test]
    fn residuals_are_uncorrelated_with_the_regressor() {
        // Tính chất thứ hai: phần dư trực giao với biến giải thích. Nếu còn
        // tương quan thì vẫn còn thông tin chưa khai thác hết.
        let (a, b) = sinh_cap_dong_lien_ket(500, 13, 1.5);
        let h = regression(&a, &b).unwrap();
        let e = part_data(&a, &b, &h);
        let r = correlation(&a, &e).unwrap();
        assert!(r.abs() < 1e-9, "phần dư còn tương quan {:.2e} với x", r);
    }

    // ---------- Đồng liên kết ----------
    #[test]
    fn identifies_the_cointegrated_pair() {
        let (a, b) = sinh_cap_dong_lien_ket(1_000, 2024, 1.5);
        let h = regression(&a, &b).unwrap();
        let e = part_data(&a, &b, &h);
        let k = cointegration_test(&e, -0.05).unwrap();
        assert!(k.has_cointegration, "hệ số kéo về {:.4} phải đủ âm", k.reversion_coef);
        assert!(k.reversion_coef < 0.0);
        assert!(k.half_life.is_finite() && k.half_life > 0.0,
                "nửa chu kỳ {:.2} phải hữu hạn và dương", k.half_life);
    }

    #[test]
    fn rejects_a_merely_correlated_pair() {
        // BÀI HỌC TRUNG TÂM: tương quan gần 1 nhưng chênh lệch giãn mãi.
        let (c, d) = gen_cap_price_cointegration(1_000, 7);
        let r = correlation(&c, &d).unwrap();
        assert!(r > 0.8, "hai chuỗi này TƯƠNG QUAN rất cao: {:.3}", r);
        let h = regression(&c, &d).unwrap();
        let e = part_data(&c, &d, &h);
        let k = cointegration_test(&e, -0.05).unwrap();
        assert!(!k.has_cointegration,
                "nhưng KHÔNG đồng liên kết: hệ số kéo về chỉ {:.4}", k.reversion_coef);
    }

    #[test]
    fn stronger_reversion_means_a_shorter_half_life() {
        let gen_spread = |he_so: f64| -> Vec<f64> {
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
        let a = cointegration_test(&gen_spread(0.5), -0.05).unwrap();
        let b = cointegration_test(&gen_spread(0.95), -0.05).unwrap();
        assert!(a.half_life < b.half_life,
                "kéo mạnh nửa chu kỳ {:.2} phải ngắn hơn kéo yếu {:.2}",
                a.half_life, b.half_life);
    }

    #[test]
    fn too_short_a_series_cannot_be_tested() {
        assert_eq!(cointegration_test(&[1.0; 10], -0.05), None);
    }

    // ---------- Kalman ----------
    #[test]
    fn kalman_converges_to_the_true_beta() {
        let beta_that = 1.5;
        let (a, b) = sinh_cap_dong_lien_ket(2_000, 2024, beta_that);
        let mut lk = KalmanFilter::new(1.0, 1e-5, 1.0);
        for (&x, &y) in a.iter().zip(b.iter()) { lk.update(x, y); }
        assert!((lk.beta - beta_that).abs() < 0.15,
                "Kalman hội tụ về {:.4}, kỳ vọng {:.2}", lk.beta, beta_that);
    }

    #[test]
    fn kalman_uncertainty_falls_with_more_data() {
        let (a, b) = sinh_cap_dong_lien_ket(500, 5, 1.5);
        let mut lk = KalmanFilter::new(1.0, 1e-6, 1.0);
        let first = lk.estimated_variance;
        for (&x, &y) in a.iter().zip(b.iter()).take(200) { lk.update(x, y); }
        assert!(lk.estimated_variance < first,
                "càng nhiều dữ liệu thì càng tự tin: {:.2e} so với {:.2e}",
                lk.estimated_variance, first);
        assert_eq!(lk.num_step, 200);
    }

    #[test]
    fn kalman_does_not_panic_when_x_is_zero() {
        let mut lk = KalmanFilter::new(1.0, 1e-5, 0.0);
        let e = lk.update(0.0, 5.0);
        assert!(e.is_finite());
        assert!(lk.beta.is_finite(), "beta phải hữu hạn, không được thành NaN");
    }

    // ---------- Danh mục ----------
    #[test]
    fn diversification_cuts_risk_when_correlation_is_below_one() {
        // "Bữa trưa miễn phí" duy nhất trong tài chính.
        let a = gen_returns(1_000, 1, 0.02, 0.0005);
        let b = gen_returns(1_000, 999, 0.02, 0.0005);
        let mot = portfolio_stats(&[a.clone()], &[1.0], 0.0).unwrap();
        let two = portfolio_stats(&[a, b], &[0.5, 0.5], 0.0).unwrap();
        assert!(two.stddev < mot.stddev,
                "rủi ro danh mục {:.6} phải nhỏ hơn một mã {:.6}",
                two.stddev, mot.stddev);
        assert!(two.sharpe_ratio > mot.sharpe_ratio, "và Sharpe phải cao hơn");
    }

    #[test]
    fn identical_assets_give_no_diversification() {
        // Đa dạng hoá giả: mua hai mã y hệt nhau chẳng giảm rủi ro chút nào.
        let a = gen_returns(500, 1, 0.02, 0.0);
        let mot = portfolio_stats(&[a.clone()], &[1.0], 0.0).unwrap();
        let two = portfolio_stats(&[a.clone(), a], &[0.5, 0.5], 0.0).unwrap();
        assert!((two.stddev - mot.stddev).abs() < 1e-9,
                "cùng một mã thì rủi ro không đổi");
    }

    #[test]
    fn a_malformed_portfolio_returns_none() {
        let a = gen_returns(100, 1, 0.02, 0.0);
        assert_eq!(portfolio_stats(&[], &[], 0.0), None);
        assert_eq!(portfolio_stats(&[a], &[0.5, 0.5], 0.0), None,
                   "số trọng số phải khớp số tài sản");
    }

    #[test]
    fn portfolio_variance_is_never_negative() {
        for hat in [1u64, 42, 2024] {
            let a = gen_returns(300, hat, 0.02, 0.0);
            let b = gen_returns(300, hat + 7, 0.03, 0.0);
            let d = portfolio_stats(&[a, b], &[0.7, 0.3], 0.0).unwrap();
            assert!(d.stddev >= 0.0);
        }
    }

    // ---------- Rủi ro đuôi ----------
    #[test]
    fn expected_shortfall_is_never_below_var() {
        // Bất biến toán học: trung bình phần đuôi luôn tệ hơn ngưỡng đuôi.
        for hat in [1u64, 42, 2024] {
            let ls = gen_returns(2_000, hat, 0.02, 0.0);
            for mtc in [0.90f64, 0.95, 0.99] {
                let var = value_at_risk(&ls, mtc).unwrap();
                let es = expected_shortfall(&ls, mtc).unwrap();
                assert!(es >= var - 1e-9,
                        "thiếu hụt {:.6} phải ≥ VaR {:.6} tại {}", es, var, mtc);
            }
        }
    }

    #[test]
    fn var_grows_with_the_confidence_level() {
        let ls = gen_returns(2_000, 42, 0.02, 0.0);
        let mut prev = f64::MIN;
        for mtc in [0.80f64, 0.90, 0.95, 0.99] {
            let v = value_at_risk(&ls, mtc).unwrap();
            assert!(v >= prev, "mức tin cậy cao hơn phải cho VaR lớn hơn");
            prev = v;
        }
    }

    #[test]
    fn var_returns_none_on_bad_input() {
        assert_eq!(value_at_risk(&[], 0.95), None);
        assert_eq!(value_at_risk(&[1.0], 1.5), None);
        assert_eq!(expected_shortfall(&[], 0.95), None);
        assert_eq!(expected_shortfall(&[1.0], -0.1), None);
    }

    #[test]
    fn var_of_a_constant_series_is_that_constant() {
        let ls = vec![-0.01; 100];
        assert!((value_at_risk(&ls, 0.95).unwrap() - 0.01).abs() < 1e-12);
        assert!((expected_shortfall(&ls, 0.95).unwrap() - 0.01).abs() < 1e-12);
    }

    // ---------- Kiểm định tiến ----------
    #[test]
    fn walk_forward_splits_into_the_right_number_of_folds() {
        let cham = |_p: usize, _tu: usize, _den: usize| 1.0;
        let kq = walk_forward(1_000, 200, 100, &[5, 10, 20], cham);
        // Cửa sổ trượt 100 mỗi bước, cần 300 để đủ một đoạn → 8 đoạn
        assert_eq!(kq.segments.len(), 8);
    }

    #[test]
    fn out_of_sample_score_drops_when_overfitting() {
        // Chấm điểm có nhiễu phụ thuộc đoạn: chọn tham số theo nhiễu chính
        // là khớp quá mức, và điểm ngoài mẫu sẽ tụt.
        let cham = |p: usize, tu: usize, _den: usize| -> f64 {
            let candle = if p == 20 { 1.0 } else { 0.3 };
            candle + deterministic_noise(tu, p) * 0.8
        };
        let kq = walk_forward(2_000, 200, 100, &[5, 10, 20, 50, 100], cham);
        assert!(kq.level_drawdown > 0.0,
                "điểm phải TỤT khi ra ngoài mẫu: trong {:.3} ngoài {:.3}",
                kq.mean_in_mau, kq.mean_out_mau);
    }

    #[test]
    fn without_noise_there_is_no_degradation() {
        // Nếu tham số thật sự tốt (không phải khớp nhiễu), điểm ngoài mẫu
        // bằng điểm trong mẫu.
        let cham = |p: usize, _tu: usize, _den: usize| if p == 20 { 1.0 } else { 0.3 };
        let kq = walk_forward(1_000, 200, 100, &[5, 10, 20, 50], cham);
        assert!(kq.level_drawdown.abs() < 1e-9, "sụt {:.6}", kq.level_drawdown);
        assert!(kq.segments.iter().all(|d| d.query_param == 20),
                "phải luôn chọn đúng tham số tốt thật");
    }

    #[test]
    fn too_short_a_series_yields_no_folds() {
        let cham = |_p: usize, _tu: usize, _den: usize| 1.0;
        let kq = walk_forward(100, 200, 100, &[5], cham);
        assert!(kq.segments.is_empty());
        assert_eq!(kq.level_drawdown, 0.0);
    }

    #[test]
    fn an_empty_parameter_grid_does_not_panic() {
        let cham = |_p: usize, _tu: usize, _den: usize| 1.0;
        let kq = walk_forward(1_000, 200, 100, &[], cham);
        assert!(kq.segments.is_empty());
    }

    #[test]
    fn deterministic_noise_is_uniform_and_reproducible() {
        assert_eq!(deterministic_noise(100, 20), deterministic_noise(100, 20), "phải tất định");
        assert_ne!(deterministic_noise(100, 20), deterministic_noise(200, 20));
        assert_ne!(deterministic_noise(100, 20), deterministic_noise(100, 50));
        let mau: Vec<f64> = (0..2_000).map(|i| deterministic_noise(i, i * 7 % 13)).collect();
        for &x in &mau { assert!((-1.0..1.0).contains(&x), "giá trị {} ra ngoài khoảng", x); }
        let tb = mean(&mau);
        assert!(tb.abs() < 0.1, "trung bình {:.4} phải gần 0", tb);
        assert!(stddev(&mau) > 0.4, "phải trải đều, không co cụm");
    }

    // ---------- Sinh dữ liệu ----------
    #[test]
    fn data_generation_is_deterministic() {
        assert_eq!(sinh_cap_dong_lien_ket(100, 5, 1.5),
                   sinh_cap_dong_lien_ket(100, 5, 1.5));
        assert_ne!(sinh_cap_dong_lien_ket(100, 5, 1.5),
                   sinh_cap_dong_lien_ket(100, 6, 1.5));
        assert_eq!(gen_returns(100, 1, 0.02, 0.0), gen_returns(100, 1, 0.02, 0.0));
    }

    #[test]
    fn cap_dong_lien_ket_sinh_ra_dung_beta() {
        for beta in [1.0f64, 1.5, 2.5] {
            let (a, b) = sinh_cap_dong_lien_ket(2_000, 2024, beta);
            let h = regression(&a, &b).unwrap();
            assert!((h.beta - beta).abs() < 0.1,
                    "hồi quy ra {:.3}, kỳ vọng {:.2}", h.beta, beta);
        }
    }
}
```

---

## Bảng tra cứu lỗi biên dịch thường gặp

| Lỗi | Nguyên nhân trong chương này | Cách sửa |
|---|---|---|
| Hồi quy trả `NaN` | Phương sai của `x` bằng 0 | Trả `Option::None` khi `var_x < ε` |
| Thí nghiệm quá khớp không cho thấy gì | Bộ sinh nhiễu co cụm giá trị (modulo) | Dùng băm splitmix64 để phân phối đều thật |
| `E0277: f64 does not implement Ord` | `sort()` trên `Vec<f64>` | `sort_by(\|a, b\| a.partial_cmp(b).unwrap())` |
| VaR ra dấu ngược | Nhầm quy ước lỗ dương hay lỗ âm | Chọn một quy ước và ghi rõ trong tài liệu hàm |
| Kalman phân kỳ | `Q` hoặc `R` bằng 0 | Cả hai phải dương ngặt |

---

## Tóm tắt chương & Bài tập rèn luyện

### 5 điểm cốt lõi

1. **Tương quan không đủ; đồng liên kết mới là điều kiện đúng** cho giao dịch cặp — và thí nghiệm trong chương chứng minh khác biệt gấp 10 lần về nửa chu kỳ.
2. **Nửa chu kỳ (Half-life) là con số quan trọng nhất**: nó cho biết vốn bị kẹt bao lâu.
3. **Kalman thay tham số cố định bằng tham số thích ứng có nguyên tắc** — trọng số tự điều chỉnh theo độ tin cậy tương đối.
4. **Thiếu hụt kỳ vọng trả lời câu hỏi đúng; VaR thì không** — và VaR còn vi phạm tính dưới cộng tính.
5. **Quá khớp (Overfitting) là toán học, không phải xui xẻo.** Kiểm định tiến là phòng vệ mạnh nhất, và giả thuyết kinh tế trước là phòng vệ tốt thứ hai.

### Bài tập rèn luyện

**Bài 1.** Cài **kiểm định Johansen đơn giản hoá** cho ba tài sản trở lên.

<details>
<summary><b>Gợi ý</b></summary>

Engle–Granger chỉ xử lý được hai chuỗi và kết quả phụ thuộc vào chuỗi nào bạn chọn làm biến phụ thuộc. Johansen xử lý nhiều chuỗi cùng lúc và tìm **mọi** quan hệ đồng liên kết. Bản đầy đủ cần phân rã giá trị riêng; bản đơn giản dưới đây quét tổ hợp trọng số và tìm tổ hợp có tính dừng mạnh nhất.
</details>

<details>
<summary><b>Lời giải</b></summary>

```rust
#[derive(Debug)]
pub struct KetQuaDaBien {
    pub weight: Vec<f64>,
    pub reversion_coef: f64,
    pub half_life: f64,
}

/// Quét thô trên đơn hình trọng số — không thanh lịch bằng Johansen,
/// nhưng dễ hiểu và đủ dùng cho 3 tài sản.
pub fn tim_to_hop_dung(cac_chuoi: &[Vec<f64>], step: usize) -> Option<KetQuaDaBien> {
    let k = cac_chuoi.len();
    if k < 2 || step == 0 { return None; }
    let n = cac_chuoi.iter().map(|c| c.len()).min()?;
    if n < 30 { return None; }

    let mut best: Option<KetQuaDaBien> = None;
    // Chuẩn hoá: trọng số đầu tiên luôn = 1, các trọng số sau quét trong [−2, 2]
    let mut ts = vec![1.0; k];
    let tong_to_hop = step.pow((k - 1) as u32);

    for id in 0..tong_to_hop {
        let mut m = id;
        for j in 1..k {
            let b = m % step;
            m /= step;
            ts[j] = -2.0 + 4.0 * b as f64 / (step - 1).max(1) as f64;
        }
        let chenh: Vec<f64> = (0..n)
            .map(|i| (0..k).map(|j| ts[j] * cac_chuoi[j][i]).sum())
            .collect();

        if let Some(kq) = cointegration_test(&chenh, -0.01) {
            let tot_hon = best.as_ref()
                .map_or(true, |t| kq.reversion_coef < t.reversion_coef);
            if tot_hon && kq.reversion_coef < 0.0 {
                best = Some(KetQuaDaBien {
                    weight: ts.clone(),
                    reversion_coef: kq.reversion_coef,
                    half_life: kq.half_life,
                });
            }
        }
    }
    best
}
```

Cảnh báo quan trọng: quét thô trên nhiều tham số **chính là** công thức gây quá khớp mà mục 4 vừa cảnh báo. Với `k` tài sản và `b` bước, bạn thử `b^(k−1)` tổ hợp — và một trong số đó sẽ trông tốt kể cả trên dữ liệu ngẫu nhiên. Kết quả từ hàm này **bắt buộc** phải qua kiểm định tiến trước khi được tin.
</details>

**Bài 2.** Cài **định cỡ vị thế theo ngang bằng rủi ro** — mỗi tài sản đóng góp rủi ro bằng nhau.

<details>
<summary><b>Gợi ý</b></summary>

Danh mục 60/40 truyền thống trông cân bằng theo **vốn**, nhưng cổ phiếu biến động gấp khoảng ba lần trái phiếu, nên thực chất hơn 90% **rủi ro** đến từ cổ phiếu. Ngang bằng rủi ro (Risk parity) cân bằng theo đóng góp rủi ro thay vì theo vốn — và đó là cơ sở của các quỹ như Bridgewater All Weather.
</details>

<details>
<summary><b>Lời giải</b></summary>

```rust
/// Xấp xỉ nghịch đảo biến động — đúng chính xác khi các tài sản không tương quan.
pub fn trong_so_nghich_dao_bien_dong(bien_dong: &[f64]) -> Vec<f64> {
    let nghich: Vec<f64> = bien_dong.iter()
        .map(|v| if *v > 1e-12 { 1.0 / v } else { 0.0 }).collect();
    let tong: f64 = nghich.iter().sum();
    if tong <= 0.0 { return vec![0.0; bien_dong.len()]; }
    nghich.iter().map(|x| x / tong).collect()
}

/// Đóng góp rủi ro của từng tài sản. Ngang bằng rủi ro ⇔ mọi giá trị bằng nhau.
pub fn dong_gop_rui_ro(weight: &[f64], covariance: &[Vec<f64>]) -> Vec<f64> {
    let n = weight.len();
    // σ_p = √(wᵀ Σ w)
    let mut ps = 0.0;
    for i in 0..n { for j in 0..n {
        ps += weight[i] * weight[j] * covariance[i][j];
    }}
    let sigma = ps.sqrt();
    if sigma <= 0.0 { return vec![0.0; n]; }

    (0..n).map(|i| {
        let bien: f64 = (0..n).map(|j| covariance[i][j] * weight[j]).sum();
        weight[i] * bien / sigma        // đóng góp cận biên × trọng số
    }).collect()
}

/// Lặp cho tới khi mọi đóng góp rủi ro bằng nhau (có tương quan).
pub fn ngang_bang_rui_ro(covariance: &[Vec<f64>], num_round: usize) -> Vec<f64> {
    let n = covariance.len();
    let mut w = vec![1.0 / n as f64; n];
    let level_spend = 1.0 / n as f64;

    for _ in 0..num_round {
        let dg = dong_gop_rui_ro(&w, covariance);
        let tong: f64 = dg.iter().sum();
        if tong <= 0.0 { break; }
        for i in 0..n {
            let ratio = dg[i] / tong;
            // Đóng góp quá lớn → giảm trọng số, và ngược lại
            w[i] *= (level_spend / ratio.max(1e-12)).powf(0.5);   // giảm chấn 0,5
        }
        let s: f64 = w.iter().sum();
        for x in w.iter_mut() { *x /= s; }
    }
    w
}
```

Hệ số giảm chấn `0.5` trong số mũ giữ cho vòng lặp ổn định — không có nó, các trọng số dao động và không hội tụ. Đây là mẫu chung của mọi thuật toán điểm bất động: bước nhỏ hơn thì chậm hơn nhưng chắc chắn hơn, và trong tài chính thì "chắc chắn" gần như luôn là lựa chọn đúng.

</details>

---

## Lời kết cho toàn bộ giáo trình

Chương 84 khép lại một hành trình bắt đầu từ `println!` ở chương 1 và kết thúc ở đây — chênh lệch thống kê, cây Merkle, mạch FPGA, nhân GPU.

Nhưng thứ đọng lại không phải là danh sách chủ đề. Đó là một tập thói quen mà mọi chương đều lặp lại:

- **Chạy thì mới biết.** Mỗi khối mã trong sách này đã được biên dịch và kiểm thử. Nhiều kết luận ban đầu của chính tác giả đã bị chính máy tính bác bỏ — quyền bán châu Âu, tích phân Euler, phân vị đuôi, `HashMap` phá tính tất định.
- **Đối chiếu với sự thật bên ngoài.** SHA-256 khớp vector FIPS. Keccak khớp chữ ký ERC-20 công khai. Tự kiểm tra bằng chính mình thì không chứng minh được gì.
- **Kiểu dữ liệu là nơi mã hoá luật.** `Deps` với `DepsMut`, typestate, newtype cho tiền tệ. Thứ gì trình biên dịch bắt được thì con người không cần nhớ.
- **Biết giới hạn của thứ mình xây.** Vega bằng 0 thì không khôi phục được biến động. Quá khớp là toán học. Một mô hình đúng hôm nay chưa chắc đúng ngày mai.

Chúc bạn viết được thứ chạy đúng — và biết vì sao nó chạy đúng.
