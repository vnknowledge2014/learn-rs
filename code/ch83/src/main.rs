#![allow(dead_code)]
//! Chương 83 — Quyền chọn & Phái sinh bằng Rust: công thức Black–Scholes,
//! các tham số nhạy (Greeks), ngang giá mua-bán, chiến lược quyền chọn, và
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
    if x < 0.0 { return 1.0 - norm_cdf(-x); }
    let k = 1.0 / (1.0 + P * x);
    let mat_do = (-0.5 * x * x).exp() / (2.0 * std::f64::consts::PI).sqrt();
    let da_thuc = k * (A1 + k * (A2 + k * (A3 + k * (A4 + k * A5))));
    1.0 - mat_do * da_thuc
}

/// Hàm mật độ xác suất chuẩn tắc — dùng cho gamma và vega.
pub fn mat_do_standard(x: f64) -> f64 {
    (-0.5 * x * x).exp() / (2.0 * std::f64::consts::PI).sqrt()
}

// ============================================================================
// 2. THAM SỐ & CÔNG THỨC BLACK–SCHOLES
// ============================================================================

#[derive(Debug, Clone, Copy, PartialEq)]
pub enum OptionKind { Buy, Sell }

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
    pub bien_dong: f64,
}

impl OptionParams {
    pub fn is_valid(&self) -> bool {
        self.spot > 0.0 && self.strike > 0.0
            && self.years >= 0.0 && self.bien_dong >= 0.0
    }

    /// d₁ và d₂ — hai đại lượng trung tâm của Black–Scholes.
    /// Trả `None` khi đã đáo hạn hoặc biến động bằng 0 (khi đó công thức
    /// suy biến và ta phải dùng giá trị nội tại).
    pub fn d1_d2(&self) -> Option<(f64, f64)> {
        if self.years <= 0.0 || self.bien_dong <= 0.0 { return None; }
        let sqrt_t = self.years.sqrt();
        let d1 = ((self.spot / self.strike).ln()
                  + (self.rate + 0.5 * self.bien_dong * self.bien_dong)
                    * self.years)
                 / (self.bien_dong * sqrt_t);
        Some((d1, d1 - self.bien_dong * sqrt_t))
    }

    /// Giá trị hiện tại của giá thực hiện.
    pub fn discounted_strike(&self) -> f64 {
        self.strike * (-self.rate * self.years).exp()
    }

    /// Giá trị NỘI TẠI: phần lãi nếu thực hiện ngay lập tức.
    pub fn intrinsic_value(&self, kind: OptionKind) -> f64 {
        match kind {
            OptionKind::Buy => (self.spot - self.strike).max(0.0),
            OptionKind::Sell => (self.strike - self.spot).max(0.0),
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
    /// quyền bán kiểu Mỹ luôn đắt hơn quyền bán châu Âu cùng tham số.
    pub fn european_lower_bound(&self, kind: OptionKind) -> f64 {
        let k_ck = self.discounted_strike();
        match kind {
            OptionKind::Buy => (self.spot - k_ck).max(0.0),
            OptionKind::Sell => (k_ck - self.spot).max(0.0),
        }
    }
}

/// Giá quyền chọn kiểu châu Âu theo Black–Scholes.
pub fn gia_black_scholes(t: &OptionParams, kind: OptionKind) -> f64 {
    match t.d1_d2() {
        // Đáo hạn hoặc không biến động → giá đúng bằng giá trị nội tại
        None => t.intrinsic_value(kind),
        Some((d1, d2)) => {
            let k_ck = t.discounted_strike();
            match kind {
                OptionKind::Buy => t.spot * norm_cdf(d1) - k_ck * norm_cdf(d2),
                OptionKind::Sell => k_ck * norm_cdf(-d2) - t.spot * norm_cdf(-d1),
            }
        }
    }
}

/// Giá trị THỜI GIAN = giá thị trường − giá trị nội tại. Nó luôn ≥ 0 và tan
/// dần về 0 khi tới ngày đáo hạn. Đây chính là thứ người bán quyền chọn ăn.
pub fn value_time_time(t: &OptionParams, kind: OptionKind) -> f64 {
    (gia_black_scholes(t, kind) - t.intrinsic_value(kind)).max(0.0)
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
            // Đã đáo hạn: delta là bậc thang 0/1, mọi thứ khác bằng 0
            let in_tien = match kind {
                OptionKind::Buy => t.spot > t.strike,
                OptionKind::Sell => t.spot < t.strike,
            };
            let d = if !in_tien { 0.0 }
                    else if kind == OptionKind::Buy { 1.0 } else { -1.0 };
            return Greeks { delta: d, gamma: 0.0, vega: 0.0, theta: 0.0, rho: 0.0 };
        }
    };
    let sqrt_t = t.years.sqrt();
    let md = mat_do_standard(d1);
    let k_ck = t.discounted_strike();

    // Gamma và vega GIỐNG HỆT NHAU cho quyền mua và quyền bán cùng tham số —
    // hệ quả trực tiếp của ngang giá mua-bán.
    let gamma = md / (t.spot * t.bien_dong * sqrt_t);
    let vega = t.spot * md * sqrt_t / 100.0; // trên 1 điểm phần trăm

    let (delta, theta, rho) = match kind {
        OptionKind::Buy => (
            norm_cdf(d1),
            (-t.spot * md * t.bien_dong / (2.0 * sqrt_t)
             - t.rate * k_ck * norm_cdf(d2)) / 365.0,
            k_ck * t.years * norm_cdf(d2) / 100.0,
        ),
        OptionKind::Sell => (
            norm_cdf(d1) - 1.0,
            (-t.spot * md * t.bien_dong / (2.0 * sqrt_t)
             + t.rate * k_ck * norm_cdf(-d2)) / 365.0,
            -k_ck * t.years * norm_cdf(-d2) / 100.0,
        ),
    };
    Greeks { delta, gamma, vega, theta, rho }
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
pub fn implied_volatility(t: &OptionParams, kind: OptionKind, gia_thi_truong: f64)
    -> Option<f64>
{
    // Dùng cận dưới CHÂU ÂU, không phải giá trị nội tại: quyền bán châu Âu
    // sâu trong tiền hợp lệ khi nằm DƯỚI nội tại. Nếu chặn theo nội tại,
    // ta sẽ từ chối oan những mức giá hoàn toàn bình thường.
    let lower_bound = t.european_lower_bound(kind);
    if gia_thi_truong < lower_bound - 1e-9 { return None; }
    if t.years <= 0.0 { return None; }

    let (mut lo, mut hi) = (1e-6f64, 5.0f64);
    let price_tai = |v: f64| {
        gia_black_scholes(&OptionParams { bien_dong: v, ..*t }, kind)
    };
    // Giá thị trường phải nằm trong khoảng dựng được
    if gia_thi_truong > price_tai(hi) { return None; }

    for _ in 0..200 {
        let mid = 0.5 * (lo + hi);
        if price_tai(mid) < gia_thi_truong { lo = mid; } else { hi = mid; }
        if hi - lo < 1e-10 { break; }
    }
    Some(0.5 * (lo + hi))
}

// ============================================================================
// 5. CHIẾN LƯỢC QUYỀN CHỌN
// ============================================================================
// Mỗi chiến lược chỉ là một tổ hợp các cấu phần. Điều quan trọng nhất không
// phải nhớ tên chiến lược, mà là đọc được ĐỒ THỊ LÃI/LỖ của nó tại đáo hạn.

#[derive(Debug, Clone, Copy, PartialEq)]
pub enum KindLeg { QuyenMua, QuyenBan, TaiSanCoSo }

#[derive(Debug, Clone, Copy, PartialEq)]
pub struct Leg {
    pub kind: KindLeg,
    /// Dương = mua (trường vị), âm = bán (đoản vị).
    pub quantity: f64,
    pub strike: f64,
    /// Số tiền đã trả (mua) hoặc nhận (bán) cho mỗi đơn vị.
    pub premium: f64,
}

impl Leg {
    /// Lãi/lỗ của riêng cấu phần này tại giá đáo hạn `s`.
    pub fn pnl(&self, s: f64) -> f64 {
        let value = match self.kind {
            KindLeg::QuyenMua => (s - self.strike).max(0.0),
            KindLeg::QuyenBan => (self.strike - s).max(0.0),
            KindLeg::TaiSanCoSo => s,
        };
        self.quantity * (value - self.premium)
    }
}

#[derive(Debug, Clone, PartialEq)]
pub struct OptionStrategy { pub name: String, pub leg: Vec<Leg> }

impl OptionStrategy {
    pub fn pnl(&self, s: f64) -> f64 {
        self.leg.iter().map(|c| c.pnl(s)).sum()
    }

    /// Chi phí ban đầu. Dương = phải trả tiền, âm = được nhận tiền.
    pub fn first_only_phi_sell(&self) -> f64 {
        self.leg.iter().map(|c| c.quantity * c.premium).sum()
    }

    /// Các điểm hoà vốn, tìm bằng cách quét dải giá và bắt chỗ đổi dấu.
    pub fn breakeven(&self, tu: f64, den: f64, step: f64) -> Vec<f64> {
        let mut ra = Vec::new();
        let mut s = tu;
        let mut prev = self.pnl(s);
        while s < den {
            s += step;
            let nay = self.pnl(s);
            if prev.signum() != nay.signum() && prev.abs() > 1e-9 {
                ra.push(s - step * 0.5);
            }
            prev = nay;
        }
        ra
    }

    pub fn lai_max_in_long(&self, tu: f64, den: f64, step: f64) -> f64 {
        let mut m = f64::MIN;
        let mut s = tu;
        while s <= den { m = m.max(self.pnl(s)); s += step; }
        m
    }
    pub fn lo_max_in_long(&self, tu: f64, den: f64, step: f64) -> f64 {
        let mut m = f64::MAX;
        let mut s = tu;
        while s <= den { m = m.min(self.pnl(s)); s += step; }
        m
    }
}

// --- Các chiến lược dựng sẵn ---

/// Mua cả quyền mua lẫn quyền bán cùng giá thực hiện: cược GIÁ SẼ ĐỘNG MẠNH,
/// không quan tâm hướng nào. Lỗ tối đa = tổng phí, xảy ra khi giá đứng yên.
pub fn straddle(strike: f64, phi_mua: f64, phi_ban: f64) -> OptionStrategy {
    OptionStrategy {
        name: "Straddle (mua đôi cùng giá)".into(),
        leg: vec![
            Leg { kind: KindLeg::QuyenMua, quantity: 1.0, strike,
                      premium: phi_mua },
            Leg { kind: KindLeg::QuyenBan, quantity: 1.0, strike,
                      premium: phi_ban },
        ],
    }
}

/// Như straddle nhưng hai giá thực hiện cách xa nhau: rẻ hơn, nhưng cần giá
/// động mạnh hơn mới có lãi.
pub fn strangle(price_sell: f64, price_buy: f64, phi_mua: f64, phi_ban: f64)
    -> OptionStrategy
{
    OptionStrategy {
        name: "Strangle (mua đôi khác giá)".into(),
        leg: vec![
            Leg { kind: KindLeg::QuyenMua, quantity: 1.0,
                      strike: price_buy, premium: phi_mua },
            Leg { kind: KindLeg::QuyenBan, quantity: 1.0,
                      strike: price_sell, premium: phi_ban },
        ],
    }
}

/// Mua quyền mua giá thấp, bán quyền mua giá cao: cược giá TĂNG VỪA PHẢI.
/// Cả lãi lẫn lỗ đều có trần — đây là điểm hấp dẫn của chênh lệch giá.
pub fn spread_price_up(gia_thap: f64, gia_cao: f64, phi_thap: f64, phi_cao: f64)
    -> OptionStrategy
{
    OptionStrategy {
        name: "Chênh lệch giá tăng".into(),
        leg: vec![
            Leg { kind: KindLeg::QuyenMua, quantity: 1.0,
                      strike: gia_thap, premium: phi_thap },
            Leg { kind: KindLeg::QuyenMua, quantity: -1.0,
                      strike: gia_cao, premium: phi_cao },
        ],
    }
}

/// Nắm giữ tài sản và bán quyền mua trên nó: thu thêm phí, đổi lại từ bỏ
/// phần tăng giá vượt quá giá thực hiện.
pub fn covered_call(cost_basis: f64, strike: f64, phi: f64)
    -> OptionStrategy
{
    OptionStrategy {
        name: "Quyền mua có bảo đảm".into(),
        leg: vec![
            Leg { kind: KindLeg::TaiSanCoSo, quantity: 1.0,
                      strike: 0.0, premium: cost_basis },
            Leg { kind: KindLeg::QuyenMua, quantity: -1.0,
                      strike, premium: phi },
        ],
    }
}

/// Bốn chân: bán một strangle hẹp, mua một strangle rộng để chặn rủi ro.
/// Cược giá NẰM YÊN trong một khoảng. Lãi có trần, lỗ cũng có trần.
pub fn dieu_hau_sat(ban_thap: f64, mua_thap: f64, ban_cao: f64, mua_cao: f64,
                    phi: [f64; 4]) -> OptionStrategy
{
    OptionStrategy {
        name: "Điều hâu sắt".into(),
        leg: vec![
            Leg { kind: KindLeg::QuyenBan, quantity: 1.0,
                      strike: mua_thap, premium: phi[0] },
            Leg { kind: KindLeg::QuyenBan, quantity: -1.0,
                      strike: ban_thap, premium: phi[1] },
            Leg { kind: KindLeg::QuyenMua, quantity: -1.0,
                      strike: ban_cao, premium: phi[2] },
            Leg { kind: KindLeg::QuyenMua, quantity: 1.0,
                      strike: mua_cao, premium: phi[3] },
        ],
    }
}

fn main() {
    println!("═══════════════════════════════════════════════════════════");
    println!("   QUYỀN CHỌN & PHÁI SINH BẰNG RUST (giáo trình OpenAlgo)   ");
    println!("═══════════════════════════════════════════════════════════");

    let t = OptionParams { spot: 100.0, strike: 100.0,
                          years: 0.25, rate: 0.05, bien_dong: 0.20 };

    println!("\n1. HÀM PHÂN PHỐI CHUẨN — đối chiếu giá trị đã biết");
    for (x, mong) in [(0.0, 0.5000), (1.0, 0.8413), (1.96, 0.9750), (-1.0, 0.1587)] {
        println!("   N({:>5.2}) = {:.4}   (kỳ vọng {:.4})", x, norm_cdf(x), mong);
    }

    println!("\n2. ĐỊNH GIÁ BLACK–SCHOLES");
    println!("   Cơ sở {} · thực hiện {} · {} tháng · lãi suất {}% · biến động {}%",
             t.spot, t.strike, t.years * 12.0,
             t.rate * 100.0, t.bien_dong * 100.0);
    let c = gia_black_scholes(&t, OptionKind::Buy);
    let p = gia_black_scholes(&t, OptionKind::Sell);
    println!("   Quyền mua {:.4} (nội tại {:.2} + thời gian {:.4})",
             c, t.intrinsic_value(OptionKind::Buy), value_time_time(&t, OptionKind::Buy));
    println!("   Quyền bán {:.4} (nội tại {:.2} + thời gian {:.4})",
             p, t.intrinsic_value(OptionKind::Sell), value_time_time(&t, OptionKind::Sell));

    println!("\n3. NGANG GIÁ MUA-BÁN — bất biến kiểm chứng được");
    let left = c - p;
    let must = t.spot - t.discounted_strike();
    println!("   C − P       = {:.10}", left);
    println!("   S − K·e^-rT = {:.10}", must);
    println!("   Sai lệch    = {:.2e}", (left - must).abs());
    println!("   → Nếu hệ thức này lệch trên thị trường thật thì có cơ hội arbitrage");
    println!("     KHÔNG RỦI RO. Vì thế nó gần như không bao giờ lệch.");

    println!("\n4. CÁC THAM SỐ NHẠY");
    let gm = greeks(&t, OptionKind::Buy);
    let gb = greeks(&t, OptionKind::Sell);
    println!("   {:<12} {:>14} {:>14}", "", "quyền mua", "quyền bán");
    println!("   {:<12} {:>14.4} {:>14.4}", "delta", gm.delta, gb.delta);
    println!("   {:<12} {:>14.4} {:>14.4}", "gamma", gm.gamma, gb.gamma);
    println!("   {:<12} {:>14.4} {:>14.4}", "vega", gm.vega, gb.vega);
    println!("   {:<12} {:>14.4} {:>14.4}", "theta/ngày", gm.theta, gb.theta);
    println!("   {:<12} {:>14.4} {:>14.4}", "rho", gm.rho, gb.rho);
    println!("   → gamma và vega GIỐNG HỆT nhau ở hai loại — hệ quả của ngang giá.");
    println!("   → delta quyền mua − delta quyền bán = {:.4} (luôn bằng 1).",
             gm.delta - gb.delta);

    println!("\n5. DELTA THEO GIÁ CƠ SỞ");
    println!("   {:>10} {:>12} {:>12} {:>12}",
             "giá cơ sở", "delta mua", "gamma", "giá quyền");
    for s in [70.0f64, 90.0, 100.0, 110.0, 130.0] {
        let x = OptionParams { spot: s, ..t };
        let g = greeks(&x, OptionKind::Buy);
        println!("   {:>10.0} {:>12.4} {:>12.4} {:>12.4}",
                 s, g.delta, g.gamma, gia_black_scholes(&x, OptionKind::Buy));
    }
    println!("   → Delta đi từ 0 tới 1. Gamma lớn nhất quanh giá thực hiện —");
    println!("     đó là chỗ delta thay đổi nhanh nhất, và cũng nguy hiểm nhất.");

    println!("\n6. THỜI GIAN TAN DẦN");
    println!("   {:>14} {:>16} {:>18}", "còn lại", "giá quyền mua", "giá trị thời gian");
    for ngay in [90.0f64, 60.0, 30.0, 7.0, 1.0, 0.0] {
        let x = OptionParams { years: ngay / 365.0, ..t };
        println!("   {:>11.0} ngày {:>16.4} {:>18.4}",
                 ngay, gia_black_scholes(&x, OptionKind::Buy),
                 value_time_time(&x, OptionKind::Buy));
    }
    println!("   → Giá trị thời gian tan NHANH DẦN về cuối. Đó là lý do người bán");
    println!("     quyền chọn thích những tuần cuối, còn người mua thì sợ chúng.");

    println!("\n7. BIẾN ĐỘNG NGỤ Ý");
    for bd_that in [0.10f64, 0.20, 0.35, 0.60] {
        let x = OptionParams { bien_dong: bd_that, ..t };
        let price = gia_black_scholes(&x, OptionKind::Buy);
        let bd_tim = implied_volatility(&x, OptionKind::Buy, price).unwrap();
        println!("   biến động thật {:>5.1}% → giá {:>7.4} → tìm ngược ra {:>5.2}%",
                 bd_that * 100.0, price, bd_tim * 100.0);
    }

    println!("\n8. ĐỒ THỊ LÃI/LỖ CÁC CHIẾN LƯỢC TẠI ĐÁO HẠN");
    let cl = vec![
        straddle(100.0, 4.0, 3.0),
        strangle(95.0, 105.0, 2.0, 1.5),
        spread_price_up(95.0, 105.0, 7.0, 2.0),
        covered_call(100.0, 110.0, 3.0),
        dieu_hau_sat(95.0, 90.0, 105.0, 110.0, [1.0, 2.5, 2.5, 1.0]),
    ];
    print!("   {:<28}", "giá đáo hạn →");
    for s in [80.0f64, 90.0, 100.0, 110.0, 120.0] { print!("{:>9.0}", s); }
    println!();
    for c in &cl {
        print!("   {:<28}", c.name);
        for s in [80.0f64, 90.0, 100.0, 110.0, 120.0] { print!("{:>9.1}", c.pnl(s)); }
        println!();
    }
    println!("\n   {:<28} {:>12} {:>12} {:>14}",
             "chiến lược", "chi phí đầu", "lãi tối đa", "lỗ tối đa");
    for c in &cl {
        println!("   {:<28} {:>12.1} {:>12.1} {:>14.1}",
                 c.name, c.first_only_phi_sell(),
                 c.lai_max_in_long(0.0, 300.0, 0.5),
                 c.lo_max_in_long(0.0, 300.0, 0.5));
    }
    println!("\n   Điểm hoà vốn của straddle: {:?}",
             cl[0].breakeven(50.0, 150.0, 0.1).iter()
                  .map(|x| (x * 10.0).round() / 10.0).collect::<Vec<_>>());

    println!("\n═══════════════════════════════════════════════════════════");
    println!("   MUA QUYỀN: LỖ CÓ TRẦN. BÁN QUYỀN TRẦN TRỤI: LỖ KHÔNG TRẦN");
    println!("═══════════════════════════════════════════════════════════");
}

#[cfg(test)]
mod tests {
    use super::*;

    fn ts() -> OptionParams {
        OptionParams { spot: 100.0, strike: 100.0,
                      years: 0.25, rate: 0.05, bien_dong: 0.20 }
    }

    // ---------- Phân phối chuẩn ----------
    #[test]
    fn n_chuan_khop_gia_tri_da_biet() {
        assert!((norm_cdf(0.0) - 0.5).abs() < 1e-7);
        assert!((norm_cdf(1.0) - 0.841_344_746).abs() < 1e-6);
        assert!((norm_cdf(-1.0) - 0.158_655_254).abs() < 1e-6);
        assert!((norm_cdf(1.96) - 0.975_002_105).abs() < 1e-6);
        assert!((norm_cdf(2.576) - 0.995_003_1).abs() < 1e-5);
    }

    #[test]
    fn n_chuan_doi_xung_va_don_dieu() {
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
    fn mat_do_standard_set_peak_tai_no() {
        let peak = mat_do_standard(0.0);
        assert!((peak - 0.398_942_28).abs() < 1e-7);
        assert!(mat_do_standard(1.0) < peak);
        assert!((mat_do_standard(1.5) - mat_do_standard(-1.5)).abs() < 1e-12, "hàm chẵn");
    }

    // ---------- Black–Scholes ----------
    #[test]
    fn flat_price_buy_sell_always_use() {
        // BẤT BIẾN QUAN TRỌNG NHẤT của chương: C − P = S − K·e^(−rT).
        // Nếu nó lệch trên thị trường thật thì có arbitrage không rủi ro.
        for s in [50.0f64, 80.0, 100.0, 120.0, 200.0] {
            for k in [80.0f64, 100.0, 120.0] {
                for v in [0.1f64, 0.2, 0.5] {
                    for tg in [0.01f64, 0.25, 1.0, 2.0] {
                        let t = OptionParams { spot: s, strike: k,
                                              years: tg, rate: 0.05,
                                              bien_dong: v };
                        let c = gia_black_scholes(&t, OptionKind::Buy);
                        let p = gia_black_scholes(&t, OptionKind::Sell);
                        let lech = (c - p) - (s - t.discounted_strike());
                        assert!(lech.abs() < 1e-4,
                                "ngang giá lệch {:.2e} tại S={} K={} v={} T={}",
                                lech, s, k, v, tg);
                    }
                }
            }
        }
    }

    #[test]
    fn gia_quyen_khong_bao_gio_am() {
        for s in [1.0f64, 50.0, 100.0, 500.0] {
            for k in [50.0f64, 100.0, 200.0] {
                let t = OptionParams { spot: s, strike: k,
                                      years: 0.5, rate: 0.05,
                                      bien_dong: 0.3 };
                assert!(gia_black_scholes(&t, OptionKind::Buy) >= -1e-9);
                assert!(gia_black_scholes(&t, OptionKind::Sell) >= -1e-9);
            }
        }
    }

    #[test]
    fn price_quyen_always_few_nhat_table_european_lower_bound() {
        // Cận dưới ĐÚNG cho quyền châu Âu là max(0, S − K·e^(−rT)) và
        // max(0, K·e^(−rT) − S) — KHÔNG phải giá trị nội tại.
        for s in [20.0f64, 60.0, 100.0, 150.0, 300.0] {
            for tg in [0.1f64, 1.0, 5.0] {
                let t = OptionParams { spot: s, years: tg, ..ts() };
                for kind in [OptionKind::Buy, OptionKind::Sell] {
                    assert!(gia_black_scholes(&t, kind) >= t.european_lower_bound(kind) - 1e-9,
                            "S={} T={} loại {:?}", s, tg, kind);
                }
            }
        }
    }

    #[test]
    fn quyen_buy_european_always_above_intrinsic_value() {
        // Với quyền MUA thì cận dưới châu Âu còn CHẶT HƠN nội tại (vì
        // K·e^(−rT) < K), nên quyền mua không bao giờ rẻ hơn nội tại.
        for s in [60.0f64, 100.0, 200.0] {
            let t = OptionParams { spot: s, ..ts() };
            assert!(t.european_lower_bound(OptionKind::Buy)
                    >= t.intrinsic_value(OptionKind::Buy) - 1e-9);
            assert!(gia_black_scholes(&t, OptionKind::Buy)
                    >= t.intrinsic_value(OptionKind::Buy) - 1e-9);
        }
    }

    #[test]
    fn quyen_ban_chau_au_sau_trong_tien_CO_THE_re_hon_noi_tai() {
        // Kết quả gây bất ngờ nhưng hoàn toàn đúng — và là lý do quyền bán
        // kiểu Mỹ đắt hơn quyền bán châu Âu cùng tham số.
        let t = OptionParams { spot: 50.0, strike: 100.0,
                              years: 2.0, rate: 0.05, bien_dong: 0.15 };
        let price = gia_black_scholes(&t, OptionKind::Sell);
        let noi_tai = t.intrinsic_value(OptionKind::Sell);
        assert!(price < noi_tai,
                "quyền bán {:.3} phải rẻ hơn nội tại {:.3}", price, noi_tai);
        assert!(price >= t.european_lower_bound(OptionKind::Sell) - 1e-9,
                "nhưng vẫn phải trên cận dưới châu Âu");
        // Không có arbitrage: không được phép thực hiện sớm để ăn chênh lệch
        assert!(t.european_lower_bound(OptionKind::Sell) < noi_tai);
    }

    #[test]
    fn reverse_han_thi_price_table_use_intrinsic_value() {
        for s in [80.0f64, 100.0, 120.0] {
            let t = OptionParams { spot: s, years: 0.0, ..ts() };
            assert_eq!(gia_black_scholes(&t, OptionKind::Buy), (s - 100.0f64).max(0.0));
            assert_eq!(gia_black_scholes(&t, OptionKind::Sell), (100.0f64 - s).max(0.0));
            assert_eq!(value_time_time(&t, OptionKind::Buy), 0.0,
                       "đáo hạn thì giá trị thời gian bằng 0");
        }
    }

    #[test]
    fn no_volatility_thi_no_has_value_time_time() {
        let t = OptionParams { bien_dong: 0.0, ..ts() };
        assert_eq!(gia_black_scholes(&t, OptionKind::Buy),
                   t.intrinsic_value(OptionKind::Buy));
    }

    #[test]
    fn price_quyen_buy_up_theo_spot() {
        let mut prev = -1.0;
        for s in [50.0f64, 80.0, 100.0, 120.0, 200.0] {
            let g = gia_black_scholes(&OptionParams { spot: s, ..ts() },
                                      OptionKind::Buy);
            assert!(g > prev, "quyền mua phải đắt dần theo giá cơ sở");
            prev = g;
        }
    }

    #[test]
    fn price_quyen_up_theo_volatility() {
        // Đây là lý do "bán biến động" là một chiến lược có thật: giá quyền
        // đơn điệu tăng theo biến động, nên bán khi biến động cao là bán đắt.
        for kind in [OptionKind::Buy, OptionKind::Sell] {
            let mut prev = -1.0;
            for v in [0.05f64, 0.1, 0.2, 0.4, 0.8] {
                let g = gia_black_scholes(&OptionParams { bien_dong: v, ..ts() }, kind);
                assert!(g > prev, "loại {:?} biến động {} phải đắt hơn", kind, v);
                prev = g;
            }
        }
    }

    #[test]
    fn price_quyen_up_theo_time_time_remaining() {
        let mut prev = -1.0;
        for tg in [0.01f64, 0.1, 0.25, 1.0, 2.0] {
            let g = gia_black_scholes(&OptionParams { years: tg, ..ts() },
                                      OptionKind::Buy);
            assert!(g > prev, "còn nhiều thời gian thì quyền đắt hơn");
            prev = g;
        }
    }

    // ---------- Greeks ----------
    #[test]
    fn delta_mua_trong_0_1_delta_ban_trong_am_1_den_0() {
        for s in [20.0f64, 60.0, 100.0, 140.0, 300.0] {
            let t = OptionParams { spot: s, ..ts() };
            let dm = greeks(&t, OptionKind::Buy).delta;
            let db = greeks(&t, OptionKind::Sell).delta;
            assert!((0.0..=1.0).contains(&dm), "delta mua {} tại S={}", dm, s);
            assert!((-1.0..=0.0).contains(&db), "delta bán {} tại S={}", db, s);
            assert!((dm - db - 1.0).abs() < 1e-9,
                    "delta mua − delta bán phải luôn bằng 1");
        }
    }

    #[test]
    fn delta_tien_ve_1_khi_quyen_mua_rat_sau_trong_tien() {
        let next = greeks(&OptionParams { spot: 500.0, ..ts() },
                              OptionKind::Buy).delta;
        assert!(next > 0.99, "rất sâu trong tiền → delta ≈ 1, thực tế {:.4}", next);
        let out = greeks(&OptionParams { spot: 10.0, ..ts() },
                                OptionKind::Buy).delta;
        assert!(out < 0.01, "rất ngoài tiền → delta ≈ 0, thực tế {:.4}", out);
    }

    #[test]
    fn gamma_va_vega_giong_het_nhau_o_hai_loai_quyen() {
        // Hệ quả trực tiếp của ngang giá mua-bán: đạo hàm bậc hai theo giá và
        // đạo hàm theo biến động không phân biệt quyền mua hay quyền bán.
        for s in [70.0f64, 100.0, 130.0] {
            let t = OptionParams { spot: s, ..ts() };
            let a = greeks(&t, OptionKind::Buy);
            let b = greeks(&t, OptionKind::Sell);
            assert!((a.gamma - b.gamma).abs() < 1e-12);
            assert!((a.vega - b.vega).abs() < 1e-12);
        }
    }

    #[test]
    fn gamma_lon_nhat_quanh_gia_thuc_hien() {
        // Gamma là chỗ nguy hiểm nhất: quanh giá thực hiện, delta đổi nhanh
        // nhất, nên vị thế phòng hộ mất cân bằng nhanh nhất.
        let g_giua = greeks(&ts(), OptionKind::Buy).gamma;
        for s in [60.0f64, 80.0, 130.0, 180.0] {
            let g = greeks(&OptionParams { spot: s, ..ts() },
                                OptionKind::Buy).gamma;
            assert!(g < g_giua, "gamma tại S={} phải nhỏ hơn tại giá thực hiện", s);
        }
    }

    #[test]
    fn gamma_va_vega_luon_khong_am_khi_mua_quyen() {
        for s in [50.0f64, 100.0, 200.0] {
            for v in [0.1f64, 0.3, 0.8] {
                let g = greeks(&OptionParams { spot: s, bien_dong: v, ..ts() },
                                    OptionKind::Buy);
                assert!(g.gamma >= 0.0 && g.vega >= 0.0);
            }
        }
    }

    #[test]
    fn theta_am_voi_quyen_mua_gan_gia_thuc_hien() {
        // Thời gian là kẻ thù của người MUA quyền chọn.
        let th = greeks(&ts(), OptionKind::Buy).theta;
        assert!(th < 0.0, "theta phải âm, thực tế {:.6}", th);
    }

    #[test]
    fn greeks_tai_dao_han_la_bac_thang() {
        let in_ = greeks(&OptionParams { spot: 120.0, years: 0.0,
                                               ..ts() }, OptionKind::Buy);
        assert_eq!(in_.delta, 1.0);
        assert_eq!(in_.gamma, 0.0);
        assert_eq!(in_.theta, 0.0);
        let out = greeks(&OptionParams { spot: 80.0, years: 0.0,
                                               ..ts() }, OptionKind::Buy);
        assert_eq!(out.delta, 0.0);
    }

    #[test]
    fn delta_khop_voi_dao_ham_so_cua_gia() {
        // Kiểm chứng chéo: delta phải bằng đạo hàm của giá theo giá cơ sở.
        let h = 0.001;
        for s in [80.0f64, 100.0, 120.0] {
            let t = OptionParams { spot: s, ..ts() };
            let len = gia_black_scholes(&OptionParams { spot: s + h, ..ts() },
                                        OptionKind::Buy);
            let xuong = gia_black_scholes(&OptionParams { spot: s - h, ..ts() },
                                          OptionKind::Buy);
            let dao_ham_so = (len - xuong) / (2.0 * h);
            let d = greeks(&t, OptionKind::Buy).delta;
            assert!((d - dao_ham_so).abs() < 1e-4,
                    "delta {:.6} so với đạo hàm số {:.6} tại S={}", d, dao_ham_so, s);
        }
    }

    // ---------- Biến động ngụ ý ----------
    #[test]
    fn old_peak_price_lai_table_volatility_find_can_wait_out_use_price() {
        // BẤT BIẾN ĐÚNG: đưa biến động tìm được vào lại Black–Scholes phải
        // ra đúng giá ban đầu. Đây mới là điều ta thật sự cần bảo đảm.
        for v_that in [0.05f64, 0.1, 0.2, 0.35, 0.6, 1.0] {
            for s in [80.0f64, 100.0, 120.0] {
                let t = OptionParams { spot: s, bien_dong: v_that, ..ts() };
                for kind in [OptionKind::Buy, OptionKind::Sell] {
                    let price = gia_black_scholes(&t, kind);
                    let tim = implied_volatility(&t, kind, price)
                        .unwrap_or_else(|| panic!("không tìm được IV tại S={} v={}", s, v_that));
                    let price_lai = gia_black_scholes(
                        &OptionParams { bien_dong: tim, ..t }, kind);
                    assert!((price_lai - price).abs() < 1e-8,
                            "định giá lại ra {:.10} thay vì {:.10}", price_lai, price);
                }
            }
        }
    }

    #[test]
    fn find_use_volatility_when_quyen_cell_near_strike() {
        // Ở gần giá thực hiện, vega lớn nên giá rất nhạy với biến động và ta
        // khôi phục được con số chính xác.
        //
        // Ở rất sâu trong tiền hoặc rất ngoài tiền thì vega gần 0: giá gần
        // như không đổi dù biến động đổi nhiều, nên KHÔNG thể khôi phục chính
        // xác. Đây là hạn chế THẬT của biến động ngụ ý, không phải lỗi cài đặt.
        for v_that in [0.05f64, 0.1, 0.2, 0.35, 0.6, 1.0] {
            let t = OptionParams { bien_dong: v_that, ..ts() }; // S = K = 100
            for kind in [OptionKind::Buy, OptionKind::Sell] {
                let price = gia_black_scholes(&t, kind);
                let tim = implied_volatility(&t, kind, price).unwrap();
                assert!((tim - v_that).abs() < 1e-5,
                        "tìm ra {:.6} thay vì {:.6}", tim, v_that);
            }
        }
    }

    #[test]
    fn vega_gan_khong_thi_bien_dong_ngu_y_kem_tin_cay() {
        // Ghi lại giới hạn một cách tường minh: vega của quyền rất sâu trong
        // tiền gần bằng 0, nên biến động ngụ ý ở đó gần như vô nghĩa.
        let next = OptionParams { spot: 500.0, ..ts() };
        let mid = ts();
        let vega_sau = greeks(&next, OptionKind::Buy).vega;
        let vega_giua = greeks(&mid, OptionKind::Buy).vega;
        assert!(vega_sau < vega_giua / 100.0,
                "vega sâu trong tiền {:.8} phải nhỏ hơn hẳn ở giá thực hiện {:.8}",
                vega_sau, vega_giua);
    }

    #[test]
    fn price_below_intrinsic_value_is_reject() {
        // Giá như vậy là bất khả — dữ liệu hỏng, hoặc có cơ hội arbitrage.
        let t = OptionParams { spot: 150.0, ..ts() };
        let lower_bound = t.european_lower_bound(OptionKind::Buy);
        assert_eq!(implied_volatility(&t, OptionKind::Buy, lower_bound - 1.0), None);
    }

    #[test]
    fn gia_qua_cao_khong_dung_duoc_thi_tra_none() {
        let t = ts();
        assert_eq!(implied_volatility(&t, OptionKind::Buy, 99.0), None,
                   "không biến động nào cho ra giá đó");
    }

    #[test]
    fn da_reverse_han_thi_no_tinh_can_implied_volatility() {
        let t = OptionParams { years: 0.0, ..ts() };
        assert_eq!(implied_volatility(&t, OptionKind::Buy, 5.0), None);
    }

    // ---------- Chiến lược ----------
    #[test]
    fn straddle_lo_nhieu_nhat_khi_gia_dung_yen() {
        let s = straddle(100.0, 4.0, 3.0);
        assert!((s.pnl(100.0) + 7.0).abs() < 1e-9, "đúng giá thực hiện → mất cả 7");
        assert!(s.pnl(80.0) > s.pnl(100.0), "giá động mạnh xuống → có lãi");
        assert!(s.pnl(120.0) > s.pnl(100.0), "giá động mạnh lên → có lãi");
        assert_eq!(s.first_only_phi_sell(), 7.0);
    }

    #[test]
    fn straddle_co_dung_hai_diem_hoa_von() {
        let s = straddle(100.0, 4.0, 3.0);
        let hv = s.breakeven(50.0, 150.0, 0.01);
        assert_eq!(hv.len(), 2, "straddle phải có đúng hai điểm hoà vốn");
        // Hoà vốn ở 100 ± 7
        assert!((hv[0] - 93.0).abs() < 0.1, "điểm dưới {:.2}", hv[0]);
        assert!((hv[1] - 107.0).abs() < 0.1, "điểm trên {:.2}", hv[1]);
    }

    #[test]
    fn strangle_re_hon_nhung_can_gia_dong_manh_hon() {
        let st = straddle(100.0, 4.0, 3.0);
        let sg = strangle(95.0, 105.0, 2.0, 1.5);
        assert!(sg.first_only_phi_sell() < st.first_only_phi_sell(), "strangle rẻ hơn");
        // Ở ngay giá 100, strangle lỗ ít hơn (vì rẻ hơn)
        assert!(sg.pnl(100.0) > st.pnl(100.0));
        // Nhưng khi giá động vừa phải, straddle lãi hơn
        assert!(st.pnl(112.0) > sg.pnl(112.0));
    }

    #[test]
    fn spread_price_up_has_cap_all_lai_lan_lo() {
        let c = spread_price_up(95.0, 105.0, 7.0, 2.0);
        let lai_max = c.lai_max_in_long(0.0, 500.0, 0.5);
        let lo_max = c.lo_max_in_long(0.0, 500.0, 0.5);
        // Lãi tối đa = (105−95) − (7−2) = 5 ; lỗ tối đa = phí ròng = 5
        assert!((lai_max - 5.0).abs() < 0.1, "lãi tối đa {:.2}", lai_max);
        assert!((lo_max + 5.0).abs() < 0.1, "lỗ tối đa {:.2}", lo_max);
        // Giá tăng vô hạn cũng không lãi thêm — đó là ý nghĩa của "có trần"
        assert!((c.pnl(1_000.0) - c.pnl(200.0)).abs() < 1e-9);
    }

    #[test]
    fn quyen_mua_co_bao_dam_tu_bo_phan_tang_gia() {
        let q = covered_call(100.0, 110.0, 3.0);
        // Giá đứng yên: lãi đúng bằng phí thu được
        assert!((q.pnl(100.0) - 3.0).abs() < 1e-9);
        // Giá vượt 110: lãi bị chặn ở 10 + 3 = 13
        assert!((q.pnl(150.0) - 13.0).abs() < 1e-9);
        assert!((q.pnl(1_000.0) - 13.0).abs() < 1e-9,
                "dù giá lên tới đâu cũng chỉ lãi 13 — đó là cái giá của phí thu được");
        // Giá sập: vẫn lỗ gần như toàn bộ
        assert!(q.pnl(50.0) < -45.0);
    }

    #[test]
    fn dieu_hau_sat_lai_khi_gia_nam_yen_va_lo_co_tran() {
        let d = dieu_hau_sat(95.0, 90.0, 105.0, 110.0, [1.0, 2.5, 2.5, 1.0]);
        let mid = d.pnl(100.0);
        assert!(mid > 0.0, "giá nằm giữa hai chân bán → có lãi, thực tế {:.2}", mid);
        let lo = d.lo_max_in_long(0.0, 300.0, 0.5);
        assert!(lo > -10.0, "lỗ phải có trần, thực tế {:.2}", lo);
        assert!((d.pnl(10.0) - d.pnl(50.0)).abs() < 1e-9,
                "quá xa về phía dưới thì lỗ không tăng thêm");
        assert!((d.pnl(200.0) - d.pnl(500.0)).abs() < 1e-9,
                "quá xa về phía trên cũng vậy");
    }

    #[test]
    fn leg_sell_has_pnl_inverse_first_with_buy() {
        let buy = Leg { kind: KindLeg::QuyenMua, quantity: 1.0,
                            strike: 100.0, premium: 5.0 };
        let ban = Leg { quantity: -1.0, ..buy };
        for s in [80.0f64, 100.0, 130.0] {
            assert!((buy.pnl(s) + ban.pnl(s)).abs() < 1e-12,
                    "mua và bán cùng hợp đồng phải triệt tiêu nhau");
        }
    }

    #[test]
    fn strategy_empty_thi_no_lai_no_lo() {
        let c = OptionStrategy { name: "rỗng".into(), leg: vec![] };
        assert_eq!(c.pnl(100.0), 0.0);
        assert_eq!(c.first_only_phi_sell(), 0.0);
        assert!(c.breakeven(0.0, 200.0, 1.0).is_empty());
    }

    #[test]
    fn tham_num_no_hop_le_is_phat_show() {
        assert!(ts().is_valid());
        assert!(!OptionParams { spot: 0.0, ..ts() }.is_valid());
        assert!(!OptionParams { strike: -1.0, ..ts() }.is_valid());
        assert!(!OptionParams { years: -0.1, ..ts() }.is_valid());
        assert!(!OptionParams { bien_dong: -0.2, ..ts() }.is_valid());
    }
}
