# Chương 83: Quyền chọn & Greeks bằng Rust — Black-Scholes, Biến động ngụ ý (OpenAlgo II)

## Giới thiệu & Mục tiêu học tập

Quyền chọn là công cụ tài chính đầu tiên có **công thức định giá đóng** được chấp nhận rộng rãi. Mô hình Black-Scholes (1973) đã thay đổi cả ngành, và Scholes cùng Merton nhận giải Nobel kinh tế năm 1997 cho nó.

Chương này cài lại toàn bộ từ đầu, kể cả hàm phân phối chuẩn tích luỹ — không dùng thư viện thống kê nào.

| Nội dung | Vì sao quan trọng |
|---|---|
| Phân phối chuẩn tích luỹ | Nền của mọi công thức; cài bằng xấp xỉ Abramowitz–Stegun |
| Black-Scholes | Giá lý thuyết của quyền chọn châu Âu |
| Greeks | Đo độ nhạy — công cụ quản trị rủi ro thật sự |
| Biến động ngụ ý | Đảo ngược công thức: từ giá suy ra biến động |
| Chiến lược | Payoff của spread, straddle, iron condor |

**Một điểm đính chính quan trọng.** Trong quá trình xây chương này, một bài kiểm thử đã khẳng định sai rằng giá quyền chọn luôn ≥ giá trị nội tại. Điều đó **không đúng với quyền BÁN châu Âu sâu trong tiền**: cận dưới đúng là `K·e^(−rT) − S`, thấp hơn giá trị nội tại `K − S`. Chênh lệch chính là **giá trị thực thi sớm** — thứ mà quyền chọn châu Âu không có, còn quyền chọn Mỹ thì có.

---

## Hình tượng hóa đời sống

```
┌──────────────────────────────────────────────────────────────────────────────┐
│  QUYỀN CHỌN = VÉ ĐẶT CHỖ CÓ THỂ KHÔNG DÙNG                                  │
│                                                                              │
│   Quyền MUA ở giá 100, phí 5:                                               │
│                                                                              │
│     lãi/lỗ                                                                   │
│        │              ╱                                                      │
│        │            ╱   ← lãi không giới hạn                                │
│      0 ├──────────┬───────                                                  │
│        │         ╱ 105 (hoà vốn)                                            │
│     −5 │────────╱     ← lỗ TỐI ĐA = phí đã trả                             │
│        └────────┴───────────► giá cổ phiếu                                  │
│                100                                                          │
│                                                                              │
│   Bất đối xứng: lỗ có trần, lãi không trần. Đó là lý do quyền chọn tồn tại. │
│                                                                              │
│  GREEKS = CÁC LOẠI ĐỘ NHẠY                                                  │
│                                                                              │
│   Delta  Δ  giá quyền đổi bao nhiêu khi cổ phiếu đổi 1?      (0 → 1)       │
│   Gamma  Γ  Delta đổi bao nhiêu khi cổ phiếu đổi 1?          (cong)        │
│   Vega   ν  giá quyền đổi bao nhiêu khi biến động đổi 1%?                   │
│   Theta  Θ  giá quyền mất bao nhiêu mỗi ngày trôi qua?       (thường âm)   │
│   Rho    ρ  giá quyền đổi bao nhiêu khi lãi suất đổi 1%?                    │
│                                                                              │
│   Delta ≈ 0,5 ở ngang tiền. Delta cũng XẤP XỈ xác suất kết thúc trong tiền. │
│                                                                              │
│  ĐỒNG HỒ CÁT THETA                                                          │
│                                                                              │
│   giá trị thời gian                                                          │
│      │████████                                                              │
│      │████████▓▓▓                                                           │
│      │████████▓▓▓░░░                                                        │
│      │████████▓▓▓░░░▁▁  ← 30 ngày cuối rơi NHANH NHẤT                      │
│      └──────────────────► ngày còn lại                                      │
│      90    60    30    0                                                    │
│                                                                              │
│   Giá trị thời gian giảm theo √T, nên tốc độ mất giá TĂNG khi gần đáo hạn.  │
│                                                                              │
│  ⚠ QUYỀN BÁN CHÂU ÂU SÂU TRONG TIỀN CÓ THỂ RẺ HƠN GIÁ TRỊ NỘI TẠI          │
│                                                                              │
│    S = 50, K = 100, r = 5%, T = 1 năm                                       │
│    Giá trị nội tại  = 100 − 50 = 50                                         │
│    Cận dưới châu Âu = 100·e^(−0,05) − 50 = 95,12 − 50 = 45,12               │
│                                                                              │
│    Vì sao? Bạn không thể thực thi sớm để lấy 100 ngay bây giờ. Phải chờ    │
│    một năm — nên 100 đó chỉ đáng 95,12 hôm nay.                            │
│    Chênh lệch 4,88 chính là GIÁ TRỊ THỰC THI SỚM của quyền chọn Mỹ.        │
└──────────────────────────────────────────────────────────────────────────────┘
```

---

## Khái niệm & Cơ chế kỹ thuật chuyên sâu

### 1. Công thức Black-Scholes và ý nghĩa từng phần

```
C = S·N(d₁) − K·e^(−rT)·N(d₂)
P = K·e^(−rT)·N(−d₂) − S·N(−d₁)

d₁ = [ln(S/K) + (r + σ²/2)·T] / (σ√T)
d₂ = d₁ − σ√T
```

Cách đọc trực giác: `N(d₂)` là xác suất (dưới độ đo trung hoà rủi ro) quyền chọn kết thúc trong tiền. `N(d₁)` là delta — số cổ phiếu cần nắm giữ để phòng vệ.

Nên `C = S·N(d₁) − K·e^(−rT)·N(d₂)` đọc là: "giá trị kỳ vọng của cổ phiếu bạn nhận, trừ giá trị hiện tại của tiền bạn phải trả, cả hai nhân với xác suất tương ứng".

Mô hình giả định: biến động không đổi, không có phí giao dịch, giao dịch liên tục, lợi suất phân phối log-chuẩn. **Không giả định nào đúng trong thực tế.** Nhưng mô hình vẫn hữu dụng vì nó cho một ngôn ngữ chung — và vì "nụ cười biến động" (xem dưới) chính là cách thị trường sửa lại các giả định sai đó.

### 2. Ngang giá quyền chọn mua–bán (put-call parity): quan hệ không thể sai

```
C − P = S − K·e^(−rT)
```

Đây **không phải mô hình**. Đây là quan hệ chênh lệch giá thuần tuý: nếu nó bị vi phạm, tồn tại lợi nhuận không rủi ro. Nó đúng bất kể mô hình định giá nào bạn dùng, bất kể giả định nào.

Trong thực hành, đây là bài kiểm tra tính đúng đắn hàng đầu: nếu cài đặt của bạn vi phạm ngang giá quyền chọn mua–bán, bạn có bug — không cần tranh luận thêm.

Từ đó suy ra cận dưới của quyền bán châu Âu: `P ≥ K·e^(−rT) − S`. Chú ý đây **thấp hơn** giá trị nội tại `K − S` khi `r > 0`, và đó chính là điểm đính chính ở đầu chương. Hệ quả cho mã: "giá trị thời gian" (`time_value` = giá − nội tại) của quyền bán châu Âu sâu trong tiền là số **âm**, nên hàm này không được chặn kết quả ở 0. Còn khi `σ = 0` mà vẫn còn thời gian, giá đáo hạn là tất định (`S·e^(rT)`), nên giá quyền chọn đúng bằng cận dưới này chứ không phải giá trị nội tại.

### 3. Gamma: Greek nguy hiểm nhất

Delta cho biết bạn cần phòng vệ bao nhiêu. Gamma cho biết **delta thay đổi nhanh thế nào** — tức là bạn phải điều chỉnh phòng vệ thường xuyên đến mức nào.

Gamma lớn nhất khi quyền chọn **ngang tiền và sắp đáo hạn**. Đó là lúc một biến động nhỏ của cổ phiếu làm delta nhảy từ 0,3 lên 0,7, và người bán quyền chọn phải mua bán liên tục để giữ trung tính.

Đây là nguồn của cái gọi là "cố định gamma" (gamma pinning) — hiện tượng giá cổ phiếu bị hút về mức giá thực hiện trong ngày đáo hạn, vì các nhà tạo lập phòng vệ tự động tạo ra áp lực mua khi giá giảm và áp lực bán khi giá tăng.

### 4. Biến động ngụ ý và giới hạn của nó

Black-Scholes có năm đầu vào: S, K, T, r, σ. Bốn cái đầu quan sát được. σ thì không — nên ta làm ngược lại: lấy giá thị trường, tìm σ khiến công thức khớp.

Không có nghiệm giải tích, nên dùng phương pháp số. Chương này dùng chia đôi vì nó **luôn hội tụ** (giá là hàm đơn điệu tăng theo σ), khác với Newton–Raphson nhanh hơn nhưng có thể phân kỳ.

**Một giới hạn quan trọng đã được kiểm chứng trong chương này**: khi quyền chọn sâu trong tiền hoặc sâu ngoài tiền, vega gần bằng 0 — nghĩa là giá gần như không phụ thuộc σ. Khi đó việc khôi phục σ chính xác là **bất khả thi về mặt số học**, không phải do thuật toán kém. Vì thế các bài kiểm thử của chương chia làm hai loại:

- **Bất biến tái định giá** (luôn đúng): định giá lại bằng σ tìm được phải cho lại giá ban đầu.
- **Khôi phục chính xác** (chỉ gần giá thực hiện): σ tìm được khớp với σ gốc.

Sự phân tách này là bài học thật về kiểm thử số: đừng đòi hỏi một tính chất mà bài toán không có.

### 5. Nụ cười biến động: bằng chứng mô hình sai

Nếu Black-Scholes đúng, mọi quyền chọn cùng ngày đáo hạn phải có cùng biến động ngụ ý. Thực tế thì không — vẽ IV theo giá thực hiện sẽ ra hình cong, gọi là **nụ cười** hoặc **nghiêng** (skew).

Với chứng khoán, nghiêng thường đi xuống: quyền bán ngoài tiền đắt hơn mô hình. Lý do kinh tế rõ ràng: thị trường sụp nhanh hơn là tăng, và ai cũng muốn mua bảo hiểm cho kịch bản sụp.

Nói cách khác, nụ cười biến động là cách thị trường **sửa lại** giả định "lợi suất log-chuẩn" của mô hình mà vẫn giữ ngôn ngữ chung của nó.

---

## Mã nguồn minh họa thực chiến

Chạy bằng `cargo run -p ch83`, kiểm thử bằng `cargo test -p ch83`.

```rust
#![allow(dead_code)]
//! Chương 83 — Quyền chọn & Phái sinh bằng Rust: công thức Black–Scholes,
//! các tham số nhạy (Greeks), ngang giá quyền chọn mua–bán, chiến lược quyền chọn, và
//! biến động ngụ ý.
//!
//! Chương thứ hai chuyển giáo trình *learn* của OpenAlgo sang Rust
//! (Options Basics + Options Strategies).
//!
//! Điểm khác biệt so với cách dạy thông thường: mọi công thức ở đây đều kèm
//! một BẤT BIẾN KIỂM CHỨNG ĐƯỢC. Ngang giá mua-bán, dấu của delta, tính đối
//! xứng của gamma — nếu cài sai, bài kiểm thử bắt được ngay.
//!
//! ⚠️ Tài liệu KỸ THUẬT, không phải lời khuyên đầu tư. Quyền chọn có thể mất
//! toàn bộ giá trị; bán quyền chọn trần trụi có rủi ro không giới hạn.

// ============================================================================
// 1. HÀM PHÂN PHỐI CHUẨN TÍCH LUỸ
// ============================================================================
// Black–Scholes cần N(x) — xác suất một biến chuẩn tắc nhỏ hơn x. Rust không
// có sẵn `erf` trong thư viện chuẩn, nên ta tự cài bằng xấp xỉ Abramowitz–
// Stegun 26.2.17, sai số tuyệt đối dưới 7,5·10⁻⁸.

pub fn norm_cdf(x: f64) -> f64 {
    const A1: f64 = 0.319381530;
    const A2: f64 = -0.356563782;
    const A3: f64 = 1.781477937;
    const A4: f64 = -1.821255978;
    const A5: f64 = 1.330274429;
    const P: f64 = 0.2316419;

    // Đối xứng: N(−x) = 1 − N(x). Xấp xỉ chỉ chính xác cho x ≥ 0.
    if x < 0.0 {
        return 1.0 - norm_cdf(-x);
    }
    let k = 1.0 / (1.0 + P * x);
    let density = (-0.5 * x * x).exp() / (2.0 * std::f64::consts::PI).sqrt();
    let poly = k * (A1 + k * (A2 + k * (A3 + k * (A4 + k * A5))));
    1.0 - density * poly
}

/// Hàm mật độ xác suất chuẩn tắc — dùng cho gamma và vega.
pub fn normal_pdf(x: f64) -> f64 {
    (-0.5 * x * x).exp() / (2.0 * std::f64::consts::PI).sqrt()
}

// ============================================================================
// 2. THAM SỐ & CÔNG THỨC BLACK–SCHOLES
// ============================================================================

#[derive(Debug, Clone, Copy, PartialEq)]
pub enum OptionKind {
    Call,
    Put,
}

#[derive(Debug, Clone, Copy, PartialEq)]
pub struct OptionParams {
    /// Giá tài sản cơ sở hiện tại.
    pub spot: f64,
    /// Giá thực hiện.
    pub strike: f64,
    /// Thời gian còn lại, tính bằng NĂM.
    pub years: f64,
    /// Lãi suất phi rủi ro, dạng thập phân (0.05 = 5%/năm).
    pub rate: f64,
    /// Biến động hằng năm, dạng thập phân (0.20 = 20%).
    pub vol: f64,
}

impl OptionParams {
    pub fn is_valid(&self) -> bool {
        self.spot > 0.0 && self.strike > 0.0 && self.years >= 0.0 && self.vol >= 0.0
    }

    /// d₁ và d₂ — hai đại lượng trung tâm của Black–Scholes.
    /// Trả `None` khi đã đáo hạn hoặc biến động bằng 0 (khi đó công thức
    /// suy biến và ta phải dùng giá trị nội tại).
    pub fn d1_d2(&self) -> Option<(f64, f64)> {
        if self.years <= 0.0 || self.vol <= 0.0 {
            return None;
        }
        let sqrt_t = self.years.sqrt();
        let d1 = ((self.spot / self.strike).ln()
            + (self.rate + 0.5 * self.vol * self.vol) * self.years)
            / (self.vol * sqrt_t);
        Some((d1, d1 - self.vol * sqrt_t))
    }

    /// Giá trị hiện tại của giá thực hiện.
    pub fn discounted_strike(&self) -> f64 {
        self.strike * (-self.rate * self.years).exp()
    }

    /// Giá trị NỘI TẠI: phần lãi nếu thực hiện ngay lập tức.
    pub fn intrinsic_value(&self, kind: OptionKind) -> f64 {
        match kind {
            OptionKind::Call => (self.spot - self.strike).max(0.0),
            OptionKind::Put => (self.strike - self.spot).max(0.0),
        }
    }

    /// CẬN DƯỚI của quyền chọn kiểu CHÂU ÂU — khác giá trị nội tại!
    ///
    /// Quyền châu Âu không được thực hiện sớm, nên thứ ta thực sự nắm giữ là
    /// quyền nhận `K` vào NGÀY ĐÁO HẠN, và giá trị hôm nay của nó chỉ là
    /// `K·e^(−rT)`. Hệ quả gây bất ngờ nhưng hoàn toàn đúng:
    ///
    /// **Quyền BÁN châu Âu sâu trong tiền có thể rẻ hơn giá trị nội tại.**
    ///
    /// Ví dụ: S = 50, K = 100, r = 5%, còn 2 năm. Nội tại là 50, nhưng cận
    /// dưới chỉ là 100·e^(−0,1) − 50 ≈ 40,5. Bạn không thể "mua rẻ rồi thực
    /// hiện ngay ăn chênh" vì không được phép thực hiện sớm.
    ///
    /// Chính khoảng chênh này là GIÁ TRỊ CỦA QUYỀN THỰC HIỆN SỚM, và là lý do
    /// quyền bán kiểu Mỹ luôn ĐẮT HƠN HOẶC BẰNG quyền bán châu Âu cùng tham số
    /// (bằng nhau khi r = 0: khi đó chẳng có lý do gì để thực hiện sớm).
    pub fn european_lower_bound(&self, kind: OptionKind) -> f64 {
        let disc_strike = self.discounted_strike();
        match kind {
            OptionKind::Call => (self.spot - disc_strike).max(0.0),
            OptionKind::Put => (disc_strike - self.spot).max(0.0),
        }
    }
}

/// Giá quyền chọn kiểu châu Âu theo Black–Scholes.
pub fn black_scholes_price(t: &OptionParams, kind: OptionKind) -> f64 {
    match t.d1_d2() {
        // Đã đáo hạn → giá đúng bằng giá trị nội tại
        None if t.years <= 0.0 => t.intrinsic_value(kind),
        // Còn thời gian nhưng biến động 0: giá đáo hạn là TẤT ĐỊNH, S_T = S·e^(rT).
        // Chiết khấu payoff về hôm nay cho đúng cận dưới châu Âu
        // max(S − K·e^(−rT), 0) — KHÔNG phải giá trị nội tại max(S − K, 0).
        None => t.european_lower_bound(kind),
        Some((d1, d2)) => {
            let disc_strike = t.discounted_strike();
            match kind {
                OptionKind::Call => t.spot * norm_cdf(d1) - disc_strike * norm_cdf(d2),
                OptionKind::Put => disc_strike * norm_cdf(-d2) - t.spot * norm_cdf(-d1),
            }
        }
    }
}

/// Giá trị THỜI GIAN = giá quyền chọn − giá trị nội tại. Nó tan dần về 0 khi
/// tới ngày đáo hạn — đây chính là thứ người bán quyền chọn ăn.
///
/// KHÔNG chặn ở 0: với quyền mua (không cổ tức) nó luôn ≥ 0, nhưng quyền BÁN
/// châu Âu sâu trong tiền có giá thấp hơn nội tại, nên giá trị thời gian ÂM
/// (xem `european_lower_bound`). Chặn về 0 sẽ che mất đúng hiện tượng đó.
pub fn time_value(t: &OptionParams, kind: OptionKind) -> f64 {
    black_scholes_price(t, kind) - t.intrinsic_value(kind)
}

// ============================================================================
// 3. CÁC THAM SỐ NHẠY (GREEKS)
// ============================================================================

#[derive(Debug, Clone, Copy, PartialEq)]
pub struct Greeks {
    /// Giá quyền đổi bao nhiêu khi cơ sở đổi 1 đơn vị.
    pub delta: f64,
    /// Delta đổi bao nhiêu khi cơ sở đổi 1 đơn vị — độ cong.
    pub gamma: f64,
    /// Giá quyền đổi bao nhiêu khi biến động tăng 1 điểm phần trăm.
    pub vega: f64,
    /// Giá quyền đổi bao nhiêu sau MỘT NGÀY trôi qua (thường âm).
    pub theta: f64,
    /// Giá quyền đổi bao nhiêu khi lãi suất tăng 1 điểm phần trăm.
    pub rho: f64,
}

pub fn greeks(t: &OptionParams, kind: OptionKind) -> Greeks {
    let (d1, d2) = match t.d1_d2() {
        Some(x) => x,
        None => {
            // Suy biến: giá là max(±(S − K·e^(−rT)), 0) — một hàm bậc thang
            // theo S. Đã đáo hạn thì e^(−rT) = 1 và mọi thứ ngoài delta bằng 0;
            // biến động 0 nhưng còn thời gian thì phần trong tiền vẫn nhạy với
            // lãi suất và thời gian qua K·e^(−rT).
            let disc_strike = t.discounted_strike();
            let itm = match kind {
                OptionKind::Call => t.spot > disc_strike,
                OptionKind::Put => t.spot < disc_strike,
            };
            let sign = if kind == OptionKind::Call { 1.0 } else { -1.0 };
            if !itm {
                return Greeks {
                    delta: 0.0,
                    gamma: 0.0,
                    vega: 0.0,
                    theta: 0.0,
                    rho: 0.0,
                };
            }
            return Greeks {
                delta: sign,
                gamma: 0.0,
                vega: 0.0,
                // giá = sign·(S − K·e^(−rT)); một ngày trôi qua làm T giảm 1/365.
                // Đã đáo hạn thì không còn thời gian để mất: theta = rho = 0.
                theta: if t.years > 0.0 {
                    -sign * t.rate * disc_strike / 365.0
                } else {
                    0.0
                },
                rho: sign * disc_strike * t.years / 100.0,
            };
        }
    };
    let sqrt_t = t.years.sqrt();
    let pdf = normal_pdf(d1);
    let disc_strike = t.discounted_strike();

    // Gamma và vega GIỐNG HỆT NHAU cho quyền mua và quyền bán cùng tham số —
    // hệ quả trực tiếp của ngang giá quyền chọn mua–bán.
    let gamma = pdf / (t.spot * t.vol * sqrt_t);
    let vega = t.spot * pdf * sqrt_t / 100.0; // trên 1 điểm phần trăm

    let (delta, theta, rho) = match kind {
        OptionKind::Call => (
            norm_cdf(d1),
            (-t.spot * pdf * t.vol / (2.0 * sqrt_t) - t.rate * disc_strike * norm_cdf(d2)) / 365.0,
            disc_strike * t.years * norm_cdf(d2) / 100.0,
        ),
        OptionKind::Put => (
            norm_cdf(d1) - 1.0,
            (-t.spot * pdf * t.vol / (2.0 * sqrt_t) + t.rate * disc_strike * norm_cdf(-d2)) / 365.0,
            -disc_strike * t.years * norm_cdf(-d2) / 100.0,
        ),
    };
    Greeks {
        delta,
        gamma,
        vega,
        theta,
        rho,
    }
}

// ============================================================================
// 4. BIẾN ĐỘNG NGỤ Ý
// ============================================================================
// Ta quan sát được GIÁ trên thị trường, nhưng không quan sát được biến động.
// Biến động ngụ ý là con số mà nếu đưa vào Black–Scholes sẽ cho ra đúng giá
// đang thấy. Không có công thức nghịch đảo, nên phải tìm bằng số.

/// Tìm biến động ngụ ý bằng chia đôi. Chọn chia đôi thay vì Newton–Raphson
/// vì nó LUÔN hội tụ khi hàm đơn điệu — mà giá quyền chọn thì đơn điệu tăng
/// theo biến động. Newton nhanh hơn nhưng có thể phân kỳ ở vùng biên.
pub fn implied_volatility(t: &OptionParams, kind: OptionKind, market_price: f64) -> Option<f64> {
    // Dùng cận dưới CHÂU ÂU, không phải giá trị nội tại: quyền bán châu Âu
    // sâu trong tiền hợp lệ khi nằm DƯỚI nội tại. Nếu chặn theo nội tại,
    // ta sẽ từ chối oan những mức giá hoàn toàn bình thường.
    let lower_bound = t.european_lower_bound(kind);
    if market_price < lower_bound - 1e-9 {
        return None;
    }
    if t.years <= 0.0 {
        return None;
    }

    let (mut lo, mut hi) = (1e-6f64, 5.0f64);
    let price_at = |v: f64| black_scholes_price(&OptionParams { vol: v, ..*t }, kind);
    // Giá thị trường phải nằm trong khoảng dựng được
    if market_price > price_at(hi) {
        return None;
    }

    for _ in 0..200 {
        let mid = 0.5 * (lo + hi);
        if price_at(mid) < market_price {
            lo = mid;
        } else {
            hi = mid;
        }
        if hi - lo < 1e-10 {
            break;
        }
    }
    Some(0.5 * (lo + hi))
}

// ============================================================================
// 5. CHIẾN LƯỢC QUYỀN CHỌN
// ============================================================================
// Mỗi chiến lược chỉ là một tổ hợp các cấu phần. Điều quan trọng nhất không
// phải nhớ tên chiến lược, mà là đọc được ĐỒ THỊ LÃI/LỖ của nó tại đáo hạn.

#[derive(Debug, Clone, Copy, PartialEq)]
pub enum LegKind {
    Call,
    Put,
    Underlying,
}

#[derive(Debug, Clone, Copy, PartialEq)]
pub struct Leg {
    pub kind: LegKind,
    /// Dương = mua (vị thế trường), âm = bán (vị thế đoản).
    pub quantity: f64,
    pub strike: f64,
    /// Số tiền đã trả (khi mua) hoặc nhận (khi bán) cho mỗi đơn vị.
    pub premium: f64,
}

impl Leg {
    /// Lãi/lỗ của riêng cấu phần này tại giá đáo hạn `s`.
    pub fn pnl(&self, s: f64) -> f64 {
        let value = match self.kind {
            LegKind::Call => (s - self.strike).max(0.0),
            LegKind::Put => (self.strike - s).max(0.0),
            LegKind::Underlying => s,
        };
        self.quantity * (value - self.premium)
    }
}

#[derive(Debug, Clone, PartialEq)]
pub struct OptionStrategy {
    pub name: String,
    pub legs: Vec<Leg>,
}

impl OptionStrategy {
    pub fn pnl(&self, s: f64) -> f64 {
        self.legs.iter().map(|c| c.pnl(s)).sum()
    }

    /// Chi phí ban đầu. Dương = phải trả tiền, âm = được nhận tiền.
    pub fn initial_cost(&self) -> f64 {
        self.legs.iter().map(|c| c.quantity * c.premium).sum()
    }

    /// Các điểm hoà vốn, tìm bằng cách quét dải giá và bắt chỗ đổi dấu.
    pub fn breakeven(&self, from: f64, to: f64, step: f64) -> Vec<f64> {
        let mut out = Vec::new();
        let mut s = from;
        let mut prev = self.pnl(s);
        while s < to {
            s += step;
            let curr = self.pnl(s);
            if prev.signum() != curr.signum() && prev.abs() > 1e-9 {
                out.push(s - step * 0.5);
            }
            prev = curr;
        }
        out
    }

    pub fn max_profit_in_range(&self, from: f64, to: f64, step: f64) -> f64 {
        let mut m = f64::MIN;
        let mut s = from;
        while s <= to {
            m = m.max(self.pnl(s));
            s += step;
        }
        m
    }
    pub fn max_loss_in_range(&self, from: f64, to: f64, step: f64) -> f64 {
        let mut m = f64::MAX;
        let mut s = from;
        while s <= to {
            m = m.min(self.pnl(s));
            s += step;
        }
        m
    }
}

// --- Các chiến lược dựng sẵn ---

/// Mua cả quyền mua lẫn quyền bán cùng giá thực hiện: cược GIÁ SẼ ĐỘNG MẠNH,
/// không quan tâm hướng nào. Lỗ tối đa = tổng phí, xảy ra khi giá đứng yên.
pub fn straddle(strike: f64, call_premium: f64, put_premium: f64) -> OptionStrategy {
    OptionStrategy {
        name: "Straddle (mua đôi cùng giá)".into(),
        legs: vec![
            Leg {
                kind: LegKind::Call,
                quantity: 1.0,
                strike,
                premium: call_premium,
            },
            Leg {
                kind: LegKind::Put,
                quantity: 1.0,
                strike,
                premium: put_premium,
            },
        ],
    }
}

/// Như straddle nhưng hai giá thực hiện cách xa nhau: rẻ hơn, nhưng cần giá
/// động mạnh hơn mới có lãi.
pub fn strangle(
    put_strike: f64,
    call_strike: f64,
    call_premium: f64,
    put_premium: f64,
) -> OptionStrategy {
    OptionStrategy {
        name: "Strangle (mua đôi khác giá)".into(),
        legs: vec![
            Leg {
                kind: LegKind::Call,
                quantity: 1.0,
                strike: call_strike,
                premium: call_premium,
            },
            Leg {
                kind: LegKind::Put,
                quantity: 1.0,
                strike: put_strike,
                premium: put_premium,
            },
        ],
    }
}

/// Mua quyền mua giá thấp, bán quyền mua giá cao: cược giá TĂNG VỪA PHẢI.
/// Cả lãi lẫn lỗ đều có trần — đây là điểm hấp dẫn của chênh lệch giá.
pub fn bull_call_spread(
    low_strike: f64,
    high_strike: f64,
    low_premium: f64,
    high_premium: f64,
) -> OptionStrategy {
    OptionStrategy {
        name: "Chênh lệch giá tăng".into(),
        legs: vec![
            Leg {
                kind: LegKind::Call,
                quantity: 1.0,
                strike: low_strike,
                premium: low_premium,
            },
            Leg {
                kind: LegKind::Call,
                quantity: -1.0,
                strike: high_strike,
                premium: high_premium,
            },
        ],
    }
}

/// Nắm giữ tài sản và bán quyền mua trên nó: thu thêm phí, đổi lại từ bỏ
/// phần tăng giá vượt quá giá thực hiện.
pub fn covered_call(cost_basis: f64, strike: f64, premium: f64) -> OptionStrategy {
    OptionStrategy {
        name: "Quyền mua có bảo đảm".into(),
        legs: vec![
            Leg {
                kind: LegKind::Underlying,
                quantity: 1.0,
                strike: 0.0,
                premium: cost_basis,
            },
            Leg {
                kind: LegKind::Call,
                quantity: -1.0,
                strike,
                premium,
            },
        ],
    }
}

/// Bốn chân: bán một strangle hẹp, mua một strangle rộng để chặn rủi ro.
/// Cược giá NẰM YÊN trong một khoảng. Lãi có trần, lỗ cũng có trần.
pub fn iron_condor(
    short_put_strike: f64,
    long_put_strike: f64,
    short_call_strike: f64,
    long_call_strike: f64,
    premiums: [f64; 4],
) -> OptionStrategy {
    OptionStrategy {
        name: "Iron condor".into(),
        legs: vec![
            Leg {
                kind: LegKind::Put,
                quantity: 1.0,
                strike: long_put_strike,
                premium: premiums[0],
            },
            Leg {
                kind: LegKind::Put,
                quantity: -1.0,
                strike: short_put_strike,
                premium: premiums[1],
            },
            Leg {
                kind: LegKind::Call,
                quantity: -1.0,
                strike: short_call_strike,
                premium: premiums[2],
            },
            Leg {
                kind: LegKind::Call,
                quantity: 1.0,
                strike: long_call_strike,
                premium: premiums[3],
            },
        ],
    }
}

fn main() {
    println!("═══════════════════════════════════════════════════════════");
    println!("   QUYỀN CHỌN & PHÁI SINH BẰNG RUST (giáo trình OpenAlgo)   ");
    println!("═══════════════════════════════════════════════════════════");

    let t = OptionParams {
        spot: 100.0,
        strike: 100.0,
        years: 0.25,
        rate: 0.05,
        vol: 0.20,
    };

    println!("\n1. HÀM PHÂN PHỐI CHUẨN — đối chiếu giá trị đã biết");
    for (x, expected) in [(0.0, 0.5000), (1.0, 0.8413), (1.96, 0.9750), (-1.0, 0.1587)] {
        println!(
            "   N({:>5.2}) = {:.4}   (kỳ vọng {:.4})",
            x,
            norm_cdf(x),
            expected
        );
    }

    println!("\n2. ĐỊNH GIÁ BLACK–SCHOLES");
    println!(
        "   Cơ sở {} · thực hiện {} · {} tháng · lãi suất {}% · biến động {}%",
        t.spot,
        t.strike,
        t.years * 12.0,
        t.rate * 100.0,
        t.vol * 100.0
    );
    let c = black_scholes_price(&t, OptionKind::Call);
    let p = black_scholes_price(&t, OptionKind::Put);
    println!(
        "   Quyền mua {:.4} (nội tại {:.2} + thời gian {:.4})",
        c,
        t.intrinsic_value(OptionKind::Call),
        time_value(&t, OptionKind::Call)
    );
    println!(
        "   Quyền bán {:.4} (nội tại {:.2} + thời gian {:.4})",
        p,
        t.intrinsic_value(OptionKind::Put),
        time_value(&t, OptionKind::Put)
    );

    println!("\n3. NGANG GIÁ MUA-BÁN — bất biến kiểm chứng được");
    let left = c - p;
    let right = t.spot - t.discounted_strike();
    println!("   C − P       = {:.10}", left);
    println!("   S − K·e^-rT = {:.10}", right);
    println!("   Sai lệch    = {:.2e}", (left - right).abs());
    println!("   → Nếu hệ thức này lệch trên thị trường thật thì có cơ hội arbitrage");
    println!("     KHÔNG RỦI RO. Vì thế nó gần như không bao giờ lệch.");

    println!("\n4. CÁC THAM SỐ NHẠY");
    let gm = greeks(&t, OptionKind::Call);
    let gb = greeks(&t, OptionKind::Put);
    println!("   {:<12} {:>14} {:>14}", "", "quyền mua", "quyền bán");
    println!("   {:<12} {:>14.4} {:>14.4}", "delta", gm.delta, gb.delta);
    println!("   {:<12} {:>14.4} {:>14.4}", "gamma", gm.gamma, gb.gamma);
    println!("   {:<12} {:>14.4} {:>14.4}", "vega", gm.vega, gb.vega);
    println!(
        "   {:<12} {:>14.4} {:>14.4}",
        "theta/ngày", gm.theta, gb.theta
    );
    println!("   {:<12} {:>14.4} {:>14.4}", "rho", gm.rho, gb.rho);
    println!(
        "   → gamma và vega GIỐNG HỆT nhau ở hai loại — hệ quả của ngang giá quyền chọn mua–bán."
    );
    println!(
        "   → delta quyền mua − delta quyền bán = {:.4} (luôn bằng 1).",
        gm.delta - gb.delta
    );

    println!("\n5. DELTA THEO GIÁ CƠ SỞ");
    println!(
        "   {:>10} {:>12} {:>12} {:>12}",
        "giá cơ sở", "delta mua", "gamma", "giá quyền"
    );
    for s in [70.0f64, 90.0, 100.0, 110.0, 130.0] {
        let x = OptionParams { spot: s, ..t };
        let g = greeks(&x, OptionKind::Call);
        println!(
            "   {:>10.0} {:>12.4} {:>12.4} {:>12.4}",
            s,
            g.delta,
            g.gamma,
            black_scholes_price(&x, OptionKind::Call)
        );
    }
    println!("   → Delta đi từ 0 tới 1. Gamma lớn nhất quanh giá thực hiện —");
    println!("     đó là chỗ delta thay đổi nhanh nhất, và cũng nguy hiểm nhất.");

    println!("\n6. THỜI GIAN TAN DẦN");
    println!(
        "   {:>14} {:>16} {:>18}",
        "còn lại", "giá quyền mua", "giá trị thời gian"
    );
    for days in [90.0f64, 60.0, 30.0, 7.0, 1.0, 0.0] {
        let x = OptionParams {
            years: days / 365.0,
            ..t
        };
        println!(
            "   {:>11.0} ngày {:>16.4} {:>18.4}",
            days,
            black_scholes_price(&x, OptionKind::Call),
            time_value(&x, OptionKind::Call)
        );
    }
    println!("   → Giá trị thời gian tan NHANH DẦN về cuối. Đó là lý do người bán");
    println!("     quyền chọn thích những tuần cuối, còn người mua thì sợ chúng.");

    println!("\n7. BIẾN ĐỘNG NGỤ Ý");
    for true_vol in [0.10f64, 0.20, 0.35, 0.60] {
        let x = OptionParams { vol: true_vol, ..t };
        let price = black_scholes_price(&x, OptionKind::Call);
        let found_vol = implied_volatility(&x, OptionKind::Call, price).unwrap();
        println!(
            "   biến động thật {:>5.1}% → giá {:>7.4} → tìm ngược ra {:>5.2}%",
            true_vol * 100.0,
            price,
            found_vol * 100.0
        );
    }

    println!("\n8. ĐỒ THỊ LÃI/LỖ CÁC CHIẾN LƯỢC TẠI ĐÁO HẠN");
    let strategies = vec![
        straddle(100.0, 4.0, 3.0),
        strangle(95.0, 105.0, 2.0, 1.5),
        bull_call_spread(95.0, 105.0, 7.0, 2.0),
        covered_call(100.0, 110.0, 3.0),
        iron_condor(95.0, 90.0, 105.0, 110.0, [1.0, 2.5, 2.5, 1.0]),
    ];
    print!("   {:<28}", "giá đáo hạn →");
    for s in [80.0f64, 90.0, 100.0, 110.0, 120.0] {
        print!("{:>9.0}", s);
    }
    println!();
    for c in &strategies {
        print!("   {:<28}", c.name);
        for s in [80.0f64, 90.0, 100.0, 110.0, 120.0] {
            print!("{:>9.1}", c.pnl(s));
        }
        println!();
    }
    println!(
        "\n   {:<28} {:>12} {:>12} {:>14}",
        "chiến lược", "chi phí đầu", "lãi tối đa", "lỗ tối đa"
    );
    for c in &strategies {
        println!(
            "   {:<28} {:>12.1} {:>12.1} {:>14.1}",
            c.name,
            c.initial_cost(),
            c.max_profit_in_range(0.0, 300.0, 0.5),
            c.max_loss_in_range(0.0, 300.0, 0.5)
        );
    }
    println!(
        "\n   Điểm hoà vốn của straddle: {:?}",
        strategies[0]
            .breakeven(50.0, 150.0, 0.1)
            .iter()
            .map(|x| (x * 10.0).round() / 10.0)
            .collect::<Vec<_>>()
    );

    println!("\n═══════════════════════════════════════════════════════════");
    println!("   MUA QUYỀN: LỖ CÓ TRẦN. BÁN QUYỀN TRẦN TRỤI: LỖ KHÔNG TRẦN");
    println!("═══════════════════════════════════════════════════════════");
}

#[cfg(test)]
mod tests {
    use super::*;

    fn ts() -> OptionParams {
        OptionParams {
            spot: 100.0,
            strike: 100.0,
            years: 0.25,
            rate: 0.05,
            vol: 0.20,
        }
    }

    // ---------- Phân phối chuẩn ----------
    #[test]
    fn norm_cdf_matches_known_values() {
        assert!((norm_cdf(0.0) - 0.5).abs() < 1e-7);
        assert!((norm_cdf(1.0) - 0.841_344_746).abs() < 1e-6);
        assert!((norm_cdf(-1.0) - 0.158_655_254).abs() < 1e-6);
        assert!((norm_cdf(1.96) - 0.975_002_105).abs() < 1e-6);
        assert!((norm_cdf(2.576) - 0.995_003_1).abs() < 1e-5);
    }

    #[test]
    fn norm_cdf_is_symmetric_and_monotonic() {
        let mut prev = 0.0;
        let mut x = -5.0;
        while x <= 5.0 {
            let v = norm_cdf(x);
            assert!((0.0..=1.0).contains(&v), "N({}) = {} ra ngoài [0,1]", x, v);
            assert!(v >= prev - 1e-12, "N phải tăng đơn điệu");
            assert!((v + norm_cdf(-x) - 1.0).abs() < 1e-7, "N(x) + N(−x) = 1");
            prev = v;
            x += 0.1;
        }
    }

    #[test]
    fn the_normal_pdf_peaks_at_zero() {
        let peak = normal_pdf(0.0);
        assert!((peak - 0.398_942_28).abs() < 1e-7);
        assert!(normal_pdf(1.0) < peak);
        assert!(
            (normal_pdf(1.5) - normal_pdf(-1.5)).abs() < 1e-12,
            "hàm chẵn"
        );
    }

    // ---------- Black–Scholes ----------
    #[test]
    fn put_call_parity_always_holds() {
        // BẤT BIẾN QUAN TRỌNG NHẤT của chương: C − P = S − K·e^(−rT).
        // Nếu nó lệch trên thị trường thật thì có arbitrage không rủi ro.
        for s in [50.0f64, 80.0, 100.0, 120.0, 200.0] {
            for k in [80.0f64, 100.0, 120.0] {
                for v in [0.1f64, 0.2, 0.5] {
                    for tg in [0.01f64, 0.25, 1.0, 2.0] {
                        let t = OptionParams {
                            spot: s,
                            strike: k,
                            years: tg,
                            rate: 0.05,
                            vol: v,
                        };
                        let c = black_scholes_price(&t, OptionKind::Call);
                        let p = black_scholes_price(&t, OptionKind::Put);
                        let gap = (c - p) - (s - t.discounted_strike());
                        assert!(
                            gap.abs() < 1e-4,
                            "ngang giá lệch {:.2e} tại S={} K={} v={} T={}",
                            gap,
                            s,
                            k,
                            v,
                            tg
                        );
                    }
                }
            }
        }
    }

    #[test]
    fn option_prices_are_never_negative() {
        for s in [1.0f64, 50.0, 100.0, 500.0] {
            for k in [50.0f64, 100.0, 200.0] {
                let t = OptionParams {
                    spot: s,
                    strike: k,
                    years: 0.5,
                    rate: 0.05,
                    vol: 0.3,
                };
                assert!(black_scholes_price(&t, OptionKind::Call) >= -1e-9);
                assert!(black_scholes_price(&t, OptionKind::Put) >= -1e-9);
            }
        }
    }

    #[test]
    fn prices_respect_the_european_lower_bound() {
        // Cận dưới ĐÚNG cho quyền châu Âu là max(0, S − K·e^(−rT)) và
        // max(0, K·e^(−rT) − S) — KHÔNG phải giá trị nội tại.
        for s in [20.0f64, 60.0, 100.0, 150.0, 300.0] {
            for tg in [0.1f64, 1.0, 5.0] {
                let t = OptionParams {
                    spot: s,
                    years: tg,
                    ..ts()
                };
                for kind in [OptionKind::Call, OptionKind::Put] {
                    assert!(
                        black_scholes_price(&t, kind) >= t.european_lower_bound(kind) - 1e-9,
                        "S={} T={} loại {:?}",
                        s,
                        tg,
                        kind
                    );
                }
            }
        }
    }

    #[test]
    fn european_calls_stay_above_intrinsic_value() {
        // Với quyền MUA thì cận dưới châu Âu còn CHẶT HƠN nội tại (vì
        // K·e^(−rT) < K), nên quyền mua không bao giờ rẻ hơn nội tại.
        for s in [60.0f64, 100.0, 200.0] {
            let t = OptionParams { spot: s, ..ts() };
            assert!(
                t.european_lower_bound(OptionKind::Call)
                    >= t.intrinsic_value(OptionKind::Call) - 1e-9
            );
            assert!(
                black_scholes_price(&t, OptionKind::Call)
                    >= t.intrinsic_value(OptionKind::Call) - 1e-9
            );
        }
    }

    #[test]
    fn deep_itm_european_put_can_trade_below_intrinsic() {
        // Kết quả gây bất ngờ nhưng hoàn toàn đúng — và là lý do quyền bán
        // kiểu Mỹ có giá ≥ quyền bán châu Âu cùng tham số.
        let t = OptionParams {
            spot: 50.0,
            strike: 100.0,
            years: 2.0,
            rate: 0.05,
            vol: 0.15,
        };
        let price = black_scholes_price(&t, OptionKind::Put);
        let intrinsic = t.intrinsic_value(OptionKind::Put);
        assert!(
            price < intrinsic,
            "quyền bán {:.3} phải rẻ hơn nội tại {:.3}",
            price,
            intrinsic
        );
        assert!(
            price >= t.european_lower_bound(OptionKind::Put) - 1e-9,
            "nhưng vẫn phải trên cận dưới châu Âu"
        );
        // Không có arbitrage: không được phép thực hiện sớm để ăn chênh lệch
        assert!(t.european_lower_bound(OptionKind::Put) < intrinsic);
    }

    #[test]
    fn at_expiry_price_equals_intrinsic_value() {
        for s in [80.0f64, 100.0, 120.0] {
            let t = OptionParams {
                spot: s,
                years: 0.0,
                ..ts()
            };
            assert_eq!(
                black_scholes_price(&t, OptionKind::Call),
                (s - 100.0f64).max(0.0)
            );
            assert_eq!(
                black_scholes_price(&t, OptionKind::Put),
                (100.0f64 - s).max(0.0)
            );
            assert_eq!(
                time_value(&t, OptionKind::Call),
                0.0,
                "đáo hạn thì giá trị thời gian bằng 0"
            );
        }
    }

    #[test]
    fn zero_volatility_prices_the_discounted_forward_payoff() {
        // σ = 0 nhưng còn thời gian: S_T = S·e^(rT) chắc chắn, nên giá quyền mua
        // là max(S − K·e^(−rT), 0) — KHÁC giá trị nội tại max(S − K, 0).
        // Trước đây hàm trả nội tại (= 0 ở ngang tiền), sai mất ~1,24.
        let t = OptionParams { vol: 0.0, ..ts() }; // S = K = 100, r = 5%, T = 0,25
        let call = black_scholes_price(&t, OptionKind::Call);
        let expected = 100.0 - 100.0 * (-0.05f64 * 0.25).exp();
        assert!((call - expected).abs() < 1e-12, "{call} so với {expected}");
        assert!(call > t.intrinsic_value(OptionKind::Call));
        assert_eq!(black_scholes_price(&t, OptionKind::Put), 0.0);
        // Ngang giá quyền chọn mua–bán vẫn phải đúng ở trường hợp suy biến
        let put = black_scholes_price(&t, OptionKind::Put);
        assert!(((call - put) - (t.spot - t.discounted_strike())).abs() < 1e-12);
        // Và giá σ → 0 phải liên tục với giá σ = 0
        let tiny = black_scholes_price(&OptionParams { vol: 1e-6, ..ts() }, OptionKind::Call);
        assert!((tiny - call).abs() < 1e-6);
    }

    #[test]
    fn zero_volatility_put_deep_itm_sits_below_intrinsic() {
        let t = OptionParams {
            spot: 50.0,
            vol: 0.0,
            years: 1.0,
            ..ts()
        };
        let put = black_scholes_price(&t, OptionKind::Put);
        assert!((put - (100.0 * (-0.05f64).exp() - 50.0)).abs() < 1e-12);
        assert!(time_value(&t, OptionKind::Put) < 0.0);
        let g = greeks(&t, OptionKind::Put);
        assert_eq!(g.delta, -1.0);
        assert!(
            g.theta > 0.0,
            "quyền bán sâu trong tiền ĐƯỢC LỢI khi thời gian trôi"
        );
    }

    #[test]
    fn deep_itm_european_put_has_negative_time_value() {
        // Trước đây `time_value` chặn ở 0 và giấu mất hiện tượng này.
        let t = OptionParams {
            spot: 50.0,
            strike: 100.0,
            years: 2.0,
            rate: 0.05,
            vol: 0.15,
        };
        assert!(time_value(&t, OptionKind::Put) < 0.0);
        assert!(time_value(&t, OptionKind::Call) >= 0.0);
    }

    #[test]
    fn call_price_rises_with_spot() {
        let mut prev = -1.0;
        for s in [50.0f64, 80.0, 100.0, 120.0, 200.0] {
            let g = black_scholes_price(&OptionParams { spot: s, ..ts() }, OptionKind::Call);
            assert!(g > prev, "quyền mua phải đắt dần theo giá cơ sở");
            prev = g;
        }
    }

    #[test]
    fn option_price_rises_with_volatility() {
        // Đây là lý do "bán biến động" là một chiến lược có thật: giá quyền
        // đơn điệu tăng theo biến động, nên bán khi biến động cao là bán đắt.
        for kind in [OptionKind::Call, OptionKind::Put] {
            let mut prev = -1.0;
            for v in [0.05f64, 0.1, 0.2, 0.4, 0.8] {
                let g = black_scholes_price(&OptionParams { vol: v, ..ts() }, kind);
                assert!(g > prev, "loại {:?} biến động {} phải đắt hơn", kind, v);
                prev = g;
            }
        }
    }

    #[test]
    fn option_price_rises_with_time_to_expiry() {
        let mut prev = -1.0;
        for tg in [0.01f64, 0.1, 0.25, 1.0, 2.0] {
            let g = black_scholes_price(&OptionParams { years: tg, ..ts() }, OptionKind::Call);
            assert!(g > prev, "còn nhiều thời gian thì quyền đắt hơn");
            prev = g;
        }
    }

    // ---------- Greeks ----------
    #[test]
    fn call_delta_in_unit_range_put_delta_negative() {
        for s in [20.0f64, 60.0, 100.0, 140.0, 300.0] {
            let t = OptionParams { spot: s, ..ts() };
            let dc = greeks(&t, OptionKind::Call).delta;
            let dp = greeks(&t, OptionKind::Put).delta;
            assert!(
                (0.0..=1.0).contains(&dc),
                "delta quyền mua {} tại S={}",
                dc,
                s
            );
            assert!(
                (-1.0..=0.0).contains(&dp),
                "delta quyền bán {} tại S={}",
                dp,
                s
            );
            assert!(
                (dc - dp - 1.0).abs() < 1e-9,
                "delta quyền mua − delta quyền bán phải luôn bằng 1"
            );
        }
    }

    #[test]
    fn call_delta_approaches_one_deep_in_the_money() {
        let deep = greeks(
            &OptionParams {
                spot: 500.0,
                ..ts()
            },
            OptionKind::Call,
        )
        .delta;
        assert!(
            deep > 0.99,
            "rất sâu trong tiền → delta ≈ 1, thực tế {:.4}",
            deep
        );
        let out = greeks(&OptionParams { spot: 10.0, ..ts() }, OptionKind::Call).delta;
        assert!(out < 0.01, "rất ngoài tiền → delta ≈ 0, thực tế {:.4}", out);
    }

    #[test]
    fn gamma_and_vega_are_identical_for_calls_and_puts() {
        // Hệ quả trực tiếp của ngang giá quyền chọn mua–bán: đạo hàm bậc hai theo giá và
        // đạo hàm theo biến động không phân biệt quyền mua hay quyền bán.
        for s in [70.0f64, 100.0, 130.0] {
            let t = OptionParams { spot: s, ..ts() };
            let a = greeks(&t, OptionKind::Call);
            let b = greeks(&t, OptionKind::Put);
            assert!((a.gamma - b.gamma).abs() < 1e-12);
            assert!((a.vega - b.vega).abs() < 1e-12);
        }
    }

    #[test]
    fn gamma_peaks_near_the_strike() {
        // Gamma là chỗ nguy hiểm nhất: quanh giá thực hiện, delta đổi nhanh
        // nhất, nên vị thế phòng hộ mất cân bằng nhanh nhất.
        let gamma_atm = greeks(&ts(), OptionKind::Call).gamma;
        for s in [60.0f64, 80.0, 130.0, 180.0] {
            let g = greeks(&OptionParams { spot: s, ..ts() }, OptionKind::Call).gamma;
            assert!(
                g < gamma_atm,
                "gamma tại S={} phải nhỏ hơn tại giá thực hiện",
                s
            );
        }
    }

    #[test]
    fn long_options_have_non_negative_gamma_and_vega() {
        for s in [50.0f64, 100.0, 200.0] {
            for v in [0.1f64, 0.3, 0.8] {
                let g = greeks(
                    &OptionParams {
                        spot: s,
                        vol: v,
                        ..ts()
                    },
                    OptionKind::Call,
                );
                assert!(g.gamma >= 0.0 && g.vega >= 0.0);
            }
        }
    }

    #[test]
    fn theta_is_negative_for_a_near_the_money_call() {
        // Thời gian là kẻ thù của người MUA quyền chọn.
        let th = greeks(&ts(), OptionKind::Call).theta;
        assert!(th < 0.0, "theta phải âm, thực tế {:.6}", th);
    }

    #[test]
    fn greeks_at_expiry_are_a_step_function() {
        let itm = greeks(
            &OptionParams {
                spot: 120.0,
                years: 0.0,
                ..ts()
            },
            OptionKind::Call,
        );
        assert_eq!(itm.delta, 1.0);
        assert_eq!(itm.gamma, 0.0);
        assert_eq!(itm.theta, 0.0);
        let out = greeks(
            &OptionParams {
                spot: 80.0,
                years: 0.0,
                ..ts()
            },
            OptionKind::Call,
        );
        assert_eq!(out.delta, 0.0);
    }

    #[test]
    fn delta_matches_the_numerical_derivative() {
        // Kiểm chứng chéo: delta phải bằng đạo hàm của giá theo giá cơ sở.
        let h = 0.001;
        for s in [80.0f64, 100.0, 120.0] {
            let t = OptionParams { spot: s, ..ts() };
            let up = black_scholes_price(
                &OptionParams {
                    spot: s + h,
                    ..ts()
                },
                OptionKind::Call,
            );
            let down = black_scholes_price(
                &OptionParams {
                    spot: s - h,
                    ..ts()
                },
                OptionKind::Call,
            );
            let numeric = (up - down) / (2.0 * h);
            let d = greeks(&t, OptionKind::Call).delta;
            assert!(
                (d - numeric).abs() < 1e-4,
                "delta {:.6} so với đạo hàm số {:.6} tại S={}",
                d,
                numeric,
                s
            );
        }
    }

    // ---------- Biến động ngụ ý ----------
    #[test]
    fn repricing_with_the_solved_vol_recovers_the_price() {
        // BẤT BIẾN ĐÚNG: đưa biến động tìm được vào lại Black–Scholes phải
        // ra đúng giá ban đầu. Đây mới là điều ta thật sự cần bảo đảm.
        for true_vol in [0.05f64, 0.1, 0.2, 0.35, 0.6, 1.0] {
            for s in [80.0f64, 100.0, 120.0] {
                let t = OptionParams {
                    spot: s,
                    vol: true_vol,
                    ..ts()
                };
                for kind in [OptionKind::Call, OptionKind::Put] {
                    let price = black_scholes_price(&t, kind);
                    let found = implied_volatility(&t, kind, price)
                        .unwrap_or_else(|| panic!("không tìm được IV tại S={} v={}", s, true_vol));
                    let repriced = black_scholes_price(&OptionParams { vol: found, ..t }, kind);
                    assert!(
                        (repriced - price).abs() < 1e-8,
                        "định giá lại ra {:.10} thay vì {:.10}",
                        repriced,
                        price
                    );
                }
            }
        }
    }

    #[test]
    fn vol_is_recovered_exactly_near_the_strike() {
        // Ở gần giá thực hiện, vega lớn nên giá rất nhạy với biến động và ta
        // khôi phục được con số chính xác.
        //
        // Ở rất sâu trong tiền hoặc rất ngoài tiền thì vega gần 0: giá gần
        // như không đổi dù biến động đổi nhiều, nên KHÔNG thể khôi phục chính
        // xác. Đây là hạn chế THẬT của biến động ngụ ý, không phải lỗi cài đặt.
        for true_vol in [0.05f64, 0.1, 0.2, 0.35, 0.6, 1.0] {
            let t = OptionParams {
                vol: true_vol,
                ..ts()
            }; // S = K = 100
            for kind in [OptionKind::Call, OptionKind::Put] {
                let price = black_scholes_price(&t, kind);
                let found = implied_volatility(&t, kind, price).unwrap();
                assert!(
                    (found - true_vol).abs() < 1e-5,
                    "tìm ra {:.6} thay vì {:.6}",
                    found,
                    true_vol
                );
            }
        }
    }

    #[test]
    fn near_zero_vega_makes_implied_vol_unreliable() {
        // Ghi lại giới hạn một cách tường minh: vega của quyền rất sâu trong
        // tiền gần bằng 0, nên biến động ngụ ý ở đó gần như vô nghĩa.
        let deep = OptionParams {
            spot: 500.0,
            ..ts()
        };
        let mid = ts();
        let vega_deep = greeks(&deep, OptionKind::Call).vega;
        let vega_atm = greeks(&mid, OptionKind::Call).vega;
        assert!(
            vega_deep < vega_atm / 100.0,
            "vega sâu trong tiền {:.8} phải nhỏ hơn hẳn ở giá thực hiện {:.8}",
            vega_deep,
            vega_atm
        );
    }

    #[test]
    fn a_price_below_intrinsic_is_rejected() {
        // Giá như vậy là bất khả — dữ liệu hỏng, hoặc có cơ hội arbitrage.
        let t = OptionParams {
            spot: 150.0,
            ..ts()
        };
        let lower_bound = t.european_lower_bound(OptionKind::Call);
        assert_eq!(
            implied_volatility(&t, OptionKind::Call, lower_bound - 1.0),
            None
        );
    }

    #[test]
    fn an_unreachable_price_returns_none() {
        let t = ts();
        assert_eq!(
            implied_volatility(&t, OptionKind::Call, 99.0),
            None,
            "không biến động nào cho ra giá đó"
        );
    }

    #[test]
    fn an_expired_option_has_no_implied_vol() {
        let t = OptionParams { years: 0.0, ..ts() };
        assert_eq!(implied_volatility(&t, OptionKind::Call, 5.0), None);
    }

    // ---------- Chiến lược ----------
    #[test]
    fn a_straddle_loses_most_when_price_is_flat() {
        let s = straddle(100.0, 4.0, 3.0);
        assert!(
            (s.pnl(100.0) + 7.0).abs() < 1e-9,
            "đúng giá thực hiện → mất cả 7"
        );
        assert!(s.pnl(80.0) > s.pnl(100.0), "giá động mạnh xuống → có lãi");
        assert!(s.pnl(120.0) > s.pnl(100.0), "giá động mạnh lên → có lãi");
        assert_eq!(s.initial_cost(), 7.0);
    }

    #[test]
    fn a_straddle_has_exactly_two_breakevens() {
        let s = straddle(100.0, 4.0, 3.0);
        let be = s.breakeven(50.0, 150.0, 0.01);
        assert_eq!(be.len(), 2, "straddle phải có đúng hai điểm hoà vốn");
        // Hoà vốn ở 100 ± 7
        assert!((be[0] - 93.0).abs() < 0.1, "điểm dưới {:.2}", be[0]);
        assert!((be[1] - 107.0).abs() < 0.1, "điểm trên {:.2}", be[1]);
    }

    #[test]
    fn a_strangle_is_cheaper_but_needs_a_bigger_move() {
        let st = straddle(100.0, 4.0, 3.0);
        let sg = strangle(95.0, 105.0, 2.0, 1.5);
        assert!(sg.initial_cost() < st.initial_cost(), "strangle rẻ hơn");
        // Ở ngay giá 100, strangle lỗ ít hơn (vì rẻ hơn)
        assert!(sg.pnl(100.0) > st.pnl(100.0));
        // Nhưng khi giá động vừa phải, straddle lãi hơn
        assert!(st.pnl(112.0) > sg.pnl(112.0));
    }

    #[test]
    fn bull_spread_caps_both_profit_and_loss() {
        let c = bull_call_spread(95.0, 105.0, 7.0, 2.0);
        let max_profit = c.max_profit_in_range(0.0, 500.0, 0.5);
        let max_loss = c.max_loss_in_range(0.0, 500.0, 0.5);
        // Lãi tối đa = (105−95) − (7−2) = 5 ; lỗ tối đa = phí ròng = 5
        assert!(
            (max_profit - 5.0).abs() < 0.1,
            "lãi tối đa {:.2}",
            max_profit
        );
        assert!((max_loss + 5.0).abs() < 0.1, "lỗ tối đa {:.2}", max_loss);
        // Giá tăng vô hạn cũng không lãi thêm — đó là ý nghĩa của "có trần"
        assert!((c.pnl(1_000.0) - c.pnl(200.0)).abs() < 1e-9);
    }

    #[test]
    fn a_covered_call_gives_up_the_upside() {
        let q = covered_call(100.0, 110.0, 3.0);
        // Giá đứng yên: lãi đúng bằng phí thu được
        assert!((q.pnl(100.0) - 3.0).abs() < 1e-9);
        // Giá vượt 110: lãi bị chặn ở 10 + 3 = 13
        assert!((q.pnl(150.0) - 13.0).abs() < 1e-9);
        assert!(
            (q.pnl(1_000.0) - 13.0).abs() < 1e-9,
            "dù giá lên tới đâu cũng chỉ lãi 13 — đó là cái giá của phí thu được"
        );
        // Giá sập: vẫn lỗ gần như toàn bộ
        assert!(q.pnl(50.0) < -45.0);
    }

    #[test]
    fn iron_condor_profits_when_flat_and_caps_losses() {
        let d = iron_condor(95.0, 90.0, 105.0, 110.0, [1.0, 2.5, 2.5, 1.0]);
        let mid = d.pnl(100.0);
        assert!(
            mid > 0.0,
            "giá nằm giữa hai chân bán → có lãi, thực tế {:.2}",
            mid
        );
        let lo = d.max_loss_in_range(0.0, 300.0, 0.5);
        assert!(lo > -10.0, "lỗ phải có trần, thực tế {:.2}", lo);
        assert!(
            (d.pnl(10.0) - d.pnl(50.0)).abs() < 1e-9,
            "quá xa về phía dưới thì lỗ không tăng thêm"
        );
        assert!(
            (d.pnl(200.0) - d.pnl(500.0)).abs() < 1e-9,
            "quá xa về phía trên cũng vậy"
        );
    }

    #[test]
    fn a_short_leg_mirrors_the_long_leg_pnl() {
        let buy = Leg {
            kind: LegKind::Call,
            quantity: 1.0,
            strike: 100.0,
            premium: 5.0,
        };
        let short = Leg {
            quantity: -1.0,
            ..buy
        };
        for s in [80.0f64, 100.0, 130.0] {
            assert!(
                (buy.pnl(s) + short.pnl(s)).abs() < 1e-12,
                "mua và bán cùng hợp đồng phải triệt tiêu nhau"
            );
        }
    }

    #[test]
    fn an_empty_strategy_has_no_pnl() {
        let c = OptionStrategy {
            name: "rỗng".into(),
            legs: vec![],
        };
        assert_eq!(c.pnl(100.0), 0.0);
        assert_eq!(c.initial_cost(), 0.0);
        assert!(c.breakeven(0.0, 200.0, 1.0).is_empty());
    }

    #[test]
    fn invalid_parameters_are_caught() {
        assert!(ts().is_valid());
        assert!(!OptionParams { spot: 0.0, ..ts() }.is_valid());
        assert!(
            !OptionParams {
                strike: -1.0,
                ..ts()
            }
            .is_valid()
        );
        assert!(
            !OptionParams {
                years: -0.1,
                ..ts()
            }
            .is_valid()
        );
        assert!(!OptionParams { vol: -0.2, ..ts() }.is_valid());
    }
}
```

---

## Bảng tra cứu lỗi biên dịch thường gặp

| Lỗi | Nguyên nhân trong chương này | Cách sửa |
|---|---|---|
| `NaN` trong `d₁` | `T = 0` hoặc `σ = 0` → chia cho 0 | Xử lý riêng: đã đáo hạn thì giá = giá trị nội tại; `σ = 0` mà còn thời gian thì giá = `max(S − K·e^(−rT), 0)` (quyền bán: `max(K·e^(−rT) − S, 0)`) |
| Bài kiểm thử "giá ≥ nội tại" trượt | Quyền BÁN châu Âu sâu trong tiền **được phép** rẻ hơn | Cận đúng là `K·e^(−rT) − S` |
| Biến động ngụ ý không hội tụ | Vega ≈ 0 khi sâu trong/ngoài tiền | Chỉ đòi khôi phục chính xác gần giá thực hiện |
| Ngang giá quyền chọn mua–bán lệch | Quên chiết khấu `K` | `K·e^(−rT)`, không phải `K` |
| `E0308: mismatched types` (expected `f64`, found `i32`) | Truyền số ngày dạng nguyên: `years: days / 365` | Đổi sang năm: `days as f64 / 365.0` |

---

## Tóm tắt chương & Bài tập rèn luyện

### 5 điểm cốt lõi

1. **Black-Scholes cho ngôn ngữ chung**, dù mọi giả định của nó đều sai trong thực tế.
2. **Ngang giá quyền chọn mua–bán là quan hệ chênh lệch giá (arbitrage), không phải mô hình.** Vi phạm nó nghĩa là có bug.
3. **Quyền bán châu Âu sâu trong tiền có thể rẻ hơn giá trị nội tại** — chênh lệch chính là giá trị thực thi sớm.
4. **Vega ≈ 0 khi sâu trong/ngoài tiền**, nên biến động ngụ ý ở đó không khôi phục được — đó là giới hạn của bài toán, không phải của thuật toán.
5. **Nụ cười biến động là cách thị trường sửa mô hình** mà vẫn giữ ngôn ngữ của nó.

### Bài tập rèn luyện

**Bài 1.** Cài **cây nhị thức định giá quyền chọn Mỹ** và đo giá trị thực thi sớm.

<details>
<summary><b>Gợi ý</b></summary>

Black-Scholes chỉ định giá quyền chọn châu Âu. Với quyền chọn Mỹ, phải kiểm ở **mỗi nút** xem thực thi ngay có tốt hơn giữ tiếp không. Cây nhị thức (Binomial tree) Cox–Ross–Rubinstein làm được điều đó, và khi số bước tăng thì giá quyền chọn châu Âu hội tụ về Black-Scholes — một cách kiểm chứng chéo rất tốt.
</details>

<details>
<summary><b>Lời giải</b></summary>

```rust
pub fn binomial_tree(
    params: &OptionParams,
    kind: OptionKind,
    num_steps: usize,
    american: bool,
) -> f64 {
    let dt = params.years / num_steps as f64;
    let u: f64 = (params.vol * dt.sqrt()).exp(); // hệ số lên
    let d: f64 = 1.0 / u; // hệ số xuống
    let p = ((params.rate * dt).exp() - d) / (u - d); // xác suất trung hoà rủi ro
    let discount = (-params.rate * dt).exp();
    let payoff = |s: f64| match kind {
        OptionKind::Call => (s - params.strike).max(0.0),
        OptionKind::Put => (params.strike - s).max(0.0),
    };
    let spot_at = |step: usize, i: usize| {
        params.spot * u.powi(i as i32) * d.powi((step - i) as i32)
    };

    // Giá trị tại đáo hạn
    let mut values: Vec<f64> = (0..=num_steps).map(|i| payoff(spot_at(num_steps, i))).collect();

    // Lùi dần về hiện tại
    for step in (0..num_steps).rev() {
        for i in 0..=step {
            let hold = discount * (p * values[i + 1] + (1.0 - p) * values[i]);
            values[i] = if american {
                hold.max(payoff(spot_at(step, i))) // ĐÂY là điểm khác biệt của kiểu Mỹ
            } else {
                hold
            };
        }
    }
    values[0]
}

/// Phần giá trị chỉ quyền chọn Mỹ mới có.
pub fn early_exercise_value(params: &OptionParams, kind: OptionKind, num_steps: usize) -> f64 {
    binomial_tree(params, kind, num_steps, true) - binomial_tree(params, kind, num_steps, false)
}
```

Với quyền **mua** trên cổ phiếu không trả cổ tức, `early_exercise_value` bằng 0 — thực thi sớm không bao giờ tối ưu. Với quyền **bán** và `r > 0`, nó dương và tăng theo độ sâu trong tiền — đó là khoản chênh lệch đã nói ở đầu chương. Khi `r = 0` thì nó cũng bằng 0: quyền bán Mỹ có giá **lớn hơn hoặc bằng** quyền bán châu Âu, không phải lúc nào cũng lớn hơn hẳn.
</details>

**Bài 2.** Cài **danh mục quyền chọn trung tính delta** và mô phỏng chi phí phòng vệ lại.

<details>
<summary><b>Gợi ý</b></summary>

Trung tính delta (Delta-neutral) nghĩa là danh mục không nhạy với biến động **nhỏ** của giá cổ phiếu. Nhưng gamma làm delta trôi, nên bạn phải phòng vệ lại liên tục — và mỗi lần phòng vệ lại đều tốn phí. Đây là cốt lõi của giao dịch biến động: bạn kiếm tiền từ chênh lệch giữa biến động ngụ ý (bán ra) và biến động thực (chi phí phòng vệ).
</details>

<details>
<summary><b>Lời giải</b></summary>

```rust
pub struct HedgedBook {
    /// Âm = đã BÁN quyền chọn.
    pub option_count: f64,
    pub kind: OptionKind,
    pub shares: f64,
    pub cash: f64,
    pub fee_per_share: f64,
    pub total_fees: f64,
    pub hedge_count: usize,
}

impl HedgedBook {
    /// Đưa danh mục về trung tính delta tại giá hiện tại.
    pub fn rehedge(&mut self, params: &OptionParams) {
        let g = greeks(params, self.kind);
        let target = -self.option_count * g.delta; // bán quyền mua → mua cổ phiếu
        let to_trade = target - self.shares;

        if to_trade.abs() < 1e-9 {
            return;
        }
        let fee = to_trade.abs() * self.fee_per_share;
        self.cash -= to_trade * params.spot + fee;
        self.shares = target;
        self.total_fees += fee;
        self.hedge_count += 1;
    }

    /// Phòng vệ theo NGƯỠNG: chỉ giao dịch khi delta trôi quá xa.
    /// Đánh đổi: ngưỡng lớn → ít phí hơn nhưng rủi ro còn lại nhiều hơn.
    pub fn rehedge_on_threshold(&mut self, params: &OptionParams, threshold: f64) {
        let g = greeks(params, self.kind);
        let book_delta = self.option_count * g.delta + self.shares;
        if book_delta.abs() > threshold {
            self.rehedge(params);
        }
    }

    pub fn value(&self, params: &OptionParams) -> f64 {
        self.cash
            + self.shares * params.spot
            + self.option_count * black_scholes_price(params, self.kind)
    }
}
```

`rehedge_on_threshold` thể hiện đánh đổi trung tâm: phòng vệ liên tục thì không còn rủi ro delta nhưng phí ăn hết lợi nhuận; phòng vệ thưa thì rẻ hơn nhưng chịu rủi ro gamma. Ngưỡng tối ưu tỉ lệ với **căn bậc ba** của phí giao dịch — kết quả kinh điển của Whalley và Wilmott.
</details>
