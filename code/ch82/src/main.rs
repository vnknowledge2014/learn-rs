#![allow(dead_code)]
//! Chương 82 — Phân tích kỹ thuật bằng Rust: nến OHLCV, mẫu hình nến, và bộ
//! chỉ báo đầy đủ (SMA, EMA, WMA, RSI, MACD, Bollinger, ATR) viết dưới dạng
//! HÀM THUẦN TÚY.
//!
//! Đây là chương đầu trong ba chương chuyển giáo trình *learn* của OpenAlgo
//! sang Rust. OpenAlgo dạy bằng Python; ta dạy cùng nội dung bằng Rust, với
//! hai khác biệt quan trọng:
//!
//! 1. **Tiền là số nguyên** (tick), không bao giờ là số thực — xem Chương 69.
//! 2. **Mỗi chỉ báo là một hàm thuần túy** trên lát cắt dữ liệu, nên không
//!    thể vô tình "nhìn trộm tương lai" — lỗi làm hỏng phần lớn bài kiểm định
//!    nghiệp dư.
//!
//! ⚠️ Tài liệu KỸ THUẬT, không phải lời khuyên đầu tư. Mọi số liệu là dữ liệu
//! giả lập tất định.

pub type Price = i64; // tick, 1 tick = 0,01 đơn vị tiền

// ============================================================================
// 1. NẾN OHLCV
// ============================================================================

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct Candle {
    pub timestamp: u64,
    pub open: Price,
    pub high: Price,
    pub low: Price,
    pub close: Price,
    pub volume: u64,
}

impl Candle {
    /// Thân nến: khoảng cách giữa giá mở và giá đóng.
    pub fn body(&self) -> Price {
        (self.close - self.open).abs()
    }
    /// Toàn bộ biên độ trong phiên.
    pub fn range(&self) -> Price {
        self.high - self.low
    }
    pub fn upper_wick(&self) -> Price {
        self.high - self.open.max(self.close)
    }
    pub fn lower_wick(&self) -> Price {
        self.open.min(self.close) - self.low
    }
    pub fn is_bullish(&self) -> bool {
        self.close > self.open
    }
    pub fn is_bearish(&self) -> bool {
        self.close < self.open
    }

    /// Nến có hợp lệ không. Dữ liệu thị trường thật CÓ lỗi, và một nến sai
    /// làm hỏng mọi chỉ báo phía sau mà không báo gì.
    pub fn is_valid(&self) -> bool {
        self.high >= self.low
            && self.high >= self.open
            && self.high >= self.close
            && self.low <= self.open
            && self.low <= self.close
            && self.low > 0
    }
}

// ============================================================================
// 2. MẪU HÌNH NẾN
// ============================================================================
// Mẫu hình nến là cách con người tóm tắt tâm lý thị trường trong một phiên.
// Chúng KHÔNG phải tín hiệu dự báo tự thân — dùng một mình thì gần như vô
// dụng. Giá trị của chúng nằm ở chỗ xác nhận bối cảnh do chỉ báo khác dựng ra.

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum Pattern {
    Doji,
    Hammer,
    ShootingStar,
    BullishEngulfing,
    BearishEngulfing,
    NoPattern,
}

/// Doji: giá mở gần bằng giá đóng — hai phe giằng co, không ai thắng.
pub fn is_doji(n: &Candle, threshold_bps: i64) -> bool {
    if n.range() == 0 {
        return true;
    }
    n.body() * 10_000 <= n.range() * threshold_bps
}

/// Búa: thân nhỏ ở TRÊN, bóng dưới dài — người bán đẩy giá xuống nhưng bị
/// người mua kéo lại hết. Chỉ có ý nghĩa khi xuất hiện SAU một đợt giảm.
pub fn is_hammer(n: &Candle) -> bool {
    n.range() > 0 && n.body() > 0 && n.lower_wick() >= n.body() * 2 && n.upper_wick() <= n.body()
}

/// Sao băng: đối xứng của búa — bóng TRÊN dài, xuất hiện sau đợt tăng.
pub fn is_shooting_star(n: &Candle) -> bool {
    n.range() > 0 && n.body() > 0 && n.upper_wick() >= n.body() * 2 && n.lower_wick() <= n.body()
}

/// Nhấn chìm tăng: nến tăng hôm nay bao trọn thân nến giảm hôm qua.
pub fn is_bullish_engulfing(prev: &Candle, curr: &Candle) -> bool {
    prev.is_bearish()
        && curr.is_bullish()
        && curr.close >= prev.open
        && curr.open <= prev.close
        && curr.body() > prev.body()
}

pub fn is_bearish_engulfing(prev: &Candle, curr: &Candle) -> bool {
    prev.is_bullish()
        && curr.is_bearish()
        && curr.open >= prev.close
        && curr.close <= prev.open
        && curr.body() > prev.body()
}

/// Nhận diện mẫu hình tại nến CUỐI của `history`.
/// Chỉ nhìn dữ liệu ĐÃ CÓ — không bao giờ chạm tới nến tương lai.
pub fn detect_pattern(history: &[Candle]) -> Pattern {
    let n = match history.last() {
        Some(n) => n,
        None => return Pattern::NoPattern,
    };
    if let Some(q) = history.len().checked_sub(2).map(|i| &history[i]) {
        if is_bullish_engulfing(q, n) {
            return Pattern::BullishEngulfing;
        }
        if is_bearish_engulfing(q, n) {
            return Pattern::BearishEngulfing;
        }
    }
    if is_doji(n, 500) {
        return Pattern::Doji;
    } // thân ≤ 5% biên độ
    if is_hammer(n) {
        return Pattern::Hammer;
    }
    if is_shooting_star(n) {
        return Pattern::ShootingStar;
    }
    Pattern::NoPattern
}

// ============================================================================
// 3. TRUNG BÌNH ĐỘNG
// ============================================================================

/// Trung bình động đơn giản. Trả `None` khi chưa đủ `period` nến — điều này
/// QUAN TRỌNG: trả 0 hay trả trung bình của số ít nến sẽ khiến chiến lược
/// vào lệnh dựa trên dữ liệu không đủ.
pub fn sma(price: &[f64], period: usize) -> Option<f64> {
    if period == 0 || price.len() < period {
        return None;
    }
    Some(price[price.len() - period..].iter().sum::<f64>() / period as f64)
}

/// Toàn bộ chuỗi SMA. Phần tử `i` chỉ dùng dữ liệu tới `i` — không nhìn trước.
pub fn sma_series(price: &[f64], period: usize) -> Vec<Option<f64>> {
    (0..price.len())
        .map(|i| sma(&price[..=i], period))
        .collect()
}

/// Trung bình động luỹ thừa. Hệ số làm mượt α = 2/(n+1).
/// EMA phản ứng nhanh hơn SMA vì nó cho dữ liệu mới trọng số cao hơn — nhưng
/// cũng vì thế mà nhiễu hơn.
pub fn ema_series(price: &[f64], period: usize) -> Vec<Option<f64>> {
    let mut out = vec![None; price.len()];
    if period == 0 || price.len() < period {
        return out;
    }
    let alpha = 2.0 / (period as f64 + 1.0);
    // Mồi bằng SMA của `period` giá trị đầu — cách chuẩn của ngành
    let mut e = price[..period].iter().sum::<f64>() / period as f64;
    out[period - 1] = Some(e);
    for i in period..price.len() {
        e = price[i] * alpha + e * (1.0 - alpha);
        out[i] = Some(e);
    }
    out
}

/// Trung bình động có trọng số tuyến tính: giá mới nhất có trọng số n,
/// giá cũ nhất có trọng số 1.
pub fn wma(price: &[f64], period: usize) -> Option<f64> {
    if period == 0 || price.len() < period {
        return None;
    }
    let window = &price[price.len() - period..];
    let total_weight = (period * (period + 1) / 2) as f64;
    Some(
        window
            .iter()
            .enumerate()
            .map(|(i, &x)| x * (i + 1) as f64)
            .sum::<f64>()
            / total_weight,
    )
}

// ============================================================================
// 4. RSI — CHỈ SỐ SỨC MẠNH TƯƠNG ĐỐI
// ============================================================================
// RSI đo tương quan giữa mức tăng và mức giảm gần đây, quy về thang 0–100.
// Trên 70 thường gọi là "quá mua", dưới 30 là "quá bán" — nhưng trong xu
// hướng mạnh, RSI có thể nằm trên 70 hàng tuần liền. Đó là lý do dùng RSI
// một mình để đoán đảo chiều là cách mất tiền nhanh nhất.

pub fn rsi_series(price: &[f64], period: usize) -> Vec<Option<f64>> {
    let mut out = vec![None; price.len()];
    if period == 0 || price.len() <= period {
        return out;
    }

    let mut up_avg = 0.0;
    let mut down_avg = 0.0;
    for i in 1..=period {
        let d = price[i] - price[i - 1];
        if d > 0.0 {
            up_avg += d;
        } else {
            down_avg += -d;
        }
    }
    up_avg /= period as f64;
    down_avg /= period as f64;
    out[period] = Some(from_up_down(up_avg, down_avg));

    // Làm mượt kiểu Wilder: giống EMA với α = 1/n
    for i in (period + 1)..price.len() {
        let d = price[i] - price[i - 1];
        let (t, g) = if d > 0.0 { (d, 0.0) } else { (0.0, -d) };
        up_avg = (up_avg * (period - 1) as f64 + t) / period as f64;
        down_avg = (down_avg * (period - 1) as f64 + g) / period as f64;
        out[i] = Some(from_up_down(up_avg, down_avg));
    }
    out
}

fn from_up_down(up: f64, down: f64) -> f64 {
    // Không có phiên giảm nào → RSI = 100. Phải xử lý riêng để không chia cho 0.
    if down < 1e-12 {
        return if up < 1e-12 { 50.0 } else { 100.0 };
    }
    100.0 - 100.0 / (1.0 + up / down)
}

// ============================================================================
// 5. MACD
// ============================================================================

#[derive(Debug, Clone, Copy, PartialEq)]
pub struct MacdValue {
    pub macd: f64,
    pub signal: f64,
    pub histogram: f64,
}

/// MACD = EMA nhanh − EMA chậm. Đường tín hiệu = EMA của chính MACD.
/// Biểu đồ = MACD − tín hiệu, đo đà tăng tốc.
pub fn macd_series(
    price: &[f64],
    fast: usize,
    slow: usize,
    signal: usize,
) -> Vec<Option<MacdValue>> {
    let mut out = vec![None; price.len()];
    if slow == 0 || price.len() < slow {
        return out;
    }
    let ema_fast = ema_series(price, fast);
    let ema_slow = ema_series(price, slow);

    // Chuỗi MACD chỉ có giá trị từ khi CẢ HAI đường EMA đã sẵn sàng
    let mut macd_line: Vec<f64> = Vec::new();
    let mut source_indices: Vec<usize> = Vec::new();
    for i in 0..price.len() {
        if let (Some(a), Some(b)) = (ema_fast[i], ema_slow[i]) {
            macd_line.push(a - b);
            source_indices.push(i);
        }
    }
    let signal_ema = ema_series(&macd_line, signal);
    for (k, &i) in source_indices.iter().enumerate() {
        if let Some(s) = signal_ema[k] {
            out[i] = Some(MacdValue {
                macd: macd_line[k],
                signal: s,
                histogram: macd_line[k] - s,
            });
        }
    }
    out
}

// ============================================================================
// 6. DẢI BOLLINGER
// ============================================================================

#[derive(Debug, Clone, Copy, PartialEq)]
pub struct BollingerBands {
    pub upper: f64,
    pub middle: f64,
    pub lower: f64,
}

impl BollingerBands {
    pub fn bandwidth(&self) -> f64 {
        if self.middle.abs() < 1e-12 {
            0.0
        } else {
            (self.upper - self.lower) / self.middle
        }
    }
    /// Vị trí của giá trong dải: 0 = chạm đáy, 1 = chạm đỉnh.
    pub fn percent_b(&self, price: f64) -> f64 {
        let d = self.upper - self.lower;
        if d.abs() < 1e-12 {
            0.5
        } else {
            (price - self.lower) / d
        }
    }
}

pub fn bollinger(price: &[f64], period: usize, num_std: f64) -> Option<BollingerBands> {
    let middle = sma(price, period)?;
    let window = &price[price.len() - period..];
    // Độ lệch chuẩn TỔNG THỂ (chia n) — quy ước chuẩn của dải Bollinger
    let var = window.iter().map(|x| (x - middle).powi(2)).sum::<f64>() / period as f64;
    let sd = var.max(0.0).sqrt();
    Some(BollingerBands {
        upper: middle + num_std * sd,
        middle,
        lower: middle - num_std * sd,
    })
}

// ============================================================================
// 7. ATR — BIÊN ĐỘ THẬT TRUNG BÌNH
// ============================================================================
// ATR đo mức dao động, KHÔNG đo hướng. Nó là công cụ định cỡ vị thế và đặt
// cắt lỗ tốt nhất: đặt cắt lỗ cách 2 ATR thì mức chấp nhận rủi ro tự động
// điều chỉnh theo trạng thái thị trường.

/// Biên độ thật: lớn nhất trong ba khoảng cách. Nó tính cả KHOẢNG NHẢY giữa
/// hai phiên — điều mà `high − low` bỏ sót hoàn toàn.
pub fn true_range(curr: &Candle, prev: Option<&Candle>) -> Price {
    match prev {
        None => curr.high - curr.low,
        Some(t) => (curr.high - curr.low)
            .max((curr.high - t.close).abs())
            .max((curr.low - t.close).abs()),
    }
}

pub fn atr_series(candle: &[Candle], period: usize) -> Vec<Option<f64>> {
    let mut out = vec![None; candle.len()];
    if period == 0 || candle.len() < period {
        return out;
    }
    let tr: Vec<f64> = candle
        .iter()
        .enumerate()
        .map(|(i, n)| true_range(n, i.checked_sub(1).map(|j| &candle[j])) as f64)
        .collect();
    let mut a = tr[..period].iter().sum::<f64>() / period as f64;
    out[period - 1] = Some(a);
    for i in period..candle.len() {
        a = (a * (period - 1) as f64 + tr[i]) / period as f64; // làm mượt Wilder
        out[i] = Some(a);
    }
    out
}

/// Định cỡ vị thế theo ATR: rủi ro mỗi lệnh cố định bằng tiền, nên mã dao
/// động mạnh thì mua ít. Đây là công thức nền của mọi hệ thống theo xu hướng.
pub fn atr_position_size(risk_capital: i64, atr: f64, stop_atr_multiple: f64) -> i64 {
    let risk_per_unit = atr * stop_atr_multiple;
    if risk_per_unit < 1e-9 {
        return 0;
    }
    (risk_capital as f64 / risk_per_unit) as i64
}

// ============================================================================
// 8. SINH DỮ LIỆU TẤT ĐỊNH
// ============================================================================

pub fn gen_candle(n: usize, seed: u64) -> Vec<Candle> {
    let mut s = seed;
    let mut price: Price = 10_000;
    (0..n)
        .map(|i| {
            s = s
                .wrapping_mul(6364136223846793005)
                .wrapping_add(1442695040888963407);
            let step = ((s >> 33) % 201) as i64 - 100;
            let open = price;
            price = (price + step).max(100);
            let wick = ((s >> 45) % 80) as i64;
            Candle {
                timestamp: i as u64,
                open,
                high: open.max(price) + wick,
                low: (open.min(price) - wick).max(1),
                close: price,
                volume: 1_000 + (s >> 50) % 9_000,
            }
        })
        .collect()
}

pub fn close_price(candle: &[Candle]) -> Vec<f64> {
    candle.iter().map(|n| n.close as f64).collect()
}

fn main() {
    println!("═══════════════════════════════════════════════════════════");
    println!("   PHÂN TÍCH KỸ THUẬT BẰNG RUST (giáo trình OpenAlgo)       ");
    println!("═══════════════════════════════════════════════════════════");

    let candle = gen_candle(500, 2024);
    let price = close_price(&candle);

    println!("\n1. NẾN OHLCV");
    let n = &candle[100];
    println!(
        "   Nến #100: mở {} cao {} thấp {} đóng {}",
        n.open, n.high, n.low, n.close
    );
    println!(
        "   thân {} · biên độ {} · bóng trên {} · bóng dưới {} · {}",
        n.body(),
        n.range(),
        n.upper_wick(),
        n.lower_wick(),
        if n.is_bullish() { "TĂNG" } else { "GIẢM" }
    );
    println!(
        "   Toàn bộ {} nến đều hợp lệ: {}",
        candle.len(),
        candle.iter().all(|x| x.is_valid())
    );

    println!("\n2. MẪU HÌNH NẾN — đếm trên 500 nến");
    let mut count = std::collections::BTreeMap::new();
    for i in 0..candle.len() {
        *count
            .entry(format!("{:?}", detect_pattern(&candle[..=i])))
            .or_insert(0) += 1;
    }
    for (k, v) in &count {
        println!("   {:<16} {:>4} lần", k, v);
    }

    println!("\n3. TRUNG BÌNH ĐỘNG — cùng dữ liệu, khác độ nhạy");
    let s20 = sma_series(&price, 20);
    let e20 = ema_series(&price, 20);
    println!(
        "   {:>6} {:>10} {:>10} {:>10}",
        "nến", "giá", "SMA 20", "EMA 20"
    );
    for i in [100usize, 200, 300, 400, 499] {
        println!(
            "   {:>6} {:>10.0} {:>10.1} {:>10.1}",
            i,
            price[i],
            s20[i].unwrap(),
            e20[i].unwrap()
        );
    }
    println!("   → EMA bám giá sát hơn vì nó cho dữ liệu mới trọng số cao hơn.");

    println!("\n4. RSI");
    let r14 = rsi_series(&price, 14);
    let overbought = r14.iter().filter(|x| x.is_some_and(|v| v > 70.0)).count();
    let oversold = r14.iter().filter(|x| x.is_some_and(|v| v < 30.0)).count();
    println!("   RSI(14) tại nến 499: {:.1}", r14[499].unwrap());
    println!(
        "   Số phiên > 70 (quá mua): {} · < 30 (quá bán): {}",
        overbought, oversold
    );
    let steady_rise: Vec<f64> = (1..=50).map(|i| i as f64 * 100.0).collect();
    let steady_fall: Vec<f64> = (1..=50).rev().map(|i| i as f64 * 100.0).collect();
    println!(
        "   Chuỗi tăng đều  → RSI = {:.0}",
        rsi_series(&steady_rise, 14)[49].unwrap()
    );
    println!(
        "   Chuỗi giảm đều  → RSI = {:.0}",
        rsi_series(&steady_fall, 14)[49].unwrap()
    );
    println!("   → Trong xu hướng mạnh, RSI dính sát 100 hoặc 0 rất lâu.");
    println!("     Dùng RSI một mình để đoán đảo chiều là cách mất tiền nhanh nhất.");

    println!("\n5. MACD (12, 26, 9)");
    let m = macd_series(&price, 12, 26, 9);
    let mut crossover = 0;
    for i in 1..m.len() {
        if let (Some(a), Some(b)) = (m[i - 1], m[i])
            && a.histogram.signum() != b.histogram.signum()
        {
            crossover += 1;
        }
    }
    let last = m[499].unwrap();
    println!(
        "   Tại nến 499: MACD {:.2} · tín hiệu {:.2} · biểu đồ {:.2}",
        last.macd, last.signal, last.histogram
    );
    println!("   Số lần biểu đồ đổi dấu trong 500 nến: {}", crossover);
    println!(
        "   → {} tín hiệu trên 500 phiên. Phần lớn là nhiễu, và mỗi tín hiệu",
        crossover
    );
    println!("     đều tốn phí giao dịch — xem lại Chương 69.");

    println!("\n6. DẢI BOLLINGER (20, 2σ)");
    for i in [100usize, 300, 499] {
        let b = bollinger(&price[..=i], 20, 2.0).unwrap();
        println!(
            "   Nến {:>3}: dưới {:>8.1} · giữa {:>8.1} · trên {:>8.1} · giá ở {:>5.0}% dải",
            i,
            b.lower,
            b.middle,
            b.upper,
            b.percent_b(price[i]) * 100.0
        );
    }
    let outside = (20..price.len())
        .filter(|&i| {
            let b = bollinger(&price[..=i], 20, 2.0).unwrap();
            price[i] > b.upper || price[i] < b.lower
        })
        .count();
    println!(
        "   Số phiên giá vượt ra ngoài dải: {} / {} ({:.1}%)",
        outside,
        price.len() - 20,
        outside as f64 * 100.0 / (price.len() - 20) as f64
    );
    println!("   → Con số ~5% ngoài 2σ chỉ đúng khi các giá trị trong cửa sổ ĐỘC LẬP và");
    println!("     phân phối chuẩn. Giá là một bước ngẫu nhiên — giá mới nhất hay nằm ở");
    println!("     mép cửa sổ — nên tỉ lệ cao hơn hẳn, dù dữ liệu giả lập này KHÔNG có");
    println!("     đuôi béo. Thị trường thật còn cộng thêm đuôi béo.");

    println!("\n7. ATR & ĐỊNH CỠ VỊ THẾ");
    let a14 = atr_series(&candle, 14);
    println!("   ATR(14) tại nến 499: {:.1} tick", a14[499].unwrap());
    println!(
        "   {:>16} {:>12} {:>16}",
        "vốn rủi ro", "ATR", "số lượng mua"
    );
    for atr in [20.0f64, 50.0, 100.0, 200.0] {
        println!(
            "   {:>16} {:>12.0} {:>16}",
            100_000,
            atr,
            atr_position_size(100_000, atr, 2.0)
        );
    }
    println!("   → Cùng mức rủi ro bằng tiền. Mã dao động mạnh gấp 10 thì mua ít đi 10 lần.");

    println!("\n═══════════════════════════════════════════════════════════");
    println!("   CHỈ BÁO KHÔNG DỰ BÁO TƯƠNG LAI — CHÚNG TÓM TẮT QUÁ KHỨ   ");
    println!("═══════════════════════════════════════════════════════════");
}

#[cfg(test)]
mod tests {
    use super::*;

    fn simple_candle(open: Price, high: Price, low: Price, close: Price) -> Candle {
        Candle {
            timestamp: 0,
            open,
            high,
            low,
            close,
            volume: 100,
        }
    }

    // ---------- Nến ----------
    #[test]
    fn computes_body_range_and_wicks() {
        let n = simple_candle(100, 120, 90, 110);
        assert_eq!(n.body(), 10);
        assert_eq!(n.range(), 30);
        assert_eq!(n.upper_wick(), 10, "120 − max(100,110)");
        assert_eq!(n.lower_wick(), 10, "min(100,110) − 90");
        assert!(n.is_bullish() && !n.is_bearish());
    }

    #[test]
    fn detects_an_invalid_candle() {
        assert!(simple_candle(100, 120, 90, 110).is_valid());
        assert!(
            !simple_candle(100, 80, 90, 110).is_valid(),
            "cao < thấp là vô lý"
        );
        assert!(
            !simple_candle(100, 105, 90, 110).is_valid(),
            "đóng > cao là vô lý"
        );
        assert!(
            !simple_candle(100, 120, 105, 110).is_valid(),
            "thấp > mở là vô lý"
        );
        assert!(
            !simple_candle(100, 120, 0, 110).is_valid(),
            "giá không được bằng 0"
        );
    }

    #[test]
    fn every_generated_candle_is_valid() {
        for seed in [1u64, 42, 2024] {
            for n in gen_candle(1_000, seed) {
                assert!(n.is_valid(), "nến sinh ra phải luôn hợp lệ: {:?}", n);
            }
        }
    }

    // ---------- Mẫu hình ----------
    #[test]
    fn doji_when_open_nearly_equals_close() {
        assert!(is_doji(&simple_candle(100, 120, 80, 100), 500), "mở = đóng");
        assert!(
            is_doji(&simple_candle(100, 120, 80, 101), 500),
            "thân 1 trên biên độ 40"
        );
        assert!(
            !is_doji(&simple_candle(100, 120, 80, 115), 500),
            "thân 15 là quá lớn"
        );
    }

    #[test]
    fn a_zero_range_candle_counts_as_a_doji() {
        // Phiên không giao dịch — phải xử lý được, không chia cho 0.
        assert!(is_doji(&simple_candle(100, 100, 100, 100), 500));
    }

    #[test]
    fn hammer_and_shooting_star_are_mirror_images() {
        // Búa: bóng dưới dài, thân nhỏ ở trên
        let hammer = simple_candle(110, 112, 90, 111);
        assert!(
            is_hammer(&hammer),
            "bóng dưới {} thân {}",
            hammer.lower_wick(),
            hammer.body()
        );
        assert!(!is_shooting_star(&hammer));
        // Sao băng: bóng trên dài, thân nhỏ ở dưới
        let star = simple_candle(91, 112, 90, 92);
        assert!(is_shooting_star(&star));
        assert!(!is_hammer(&star));
    }

    #[test]
    fn bullish_engulfing_must_cover_the_prior_body() {
        let prev = simple_candle(110, 112, 98, 100); // giảm
        let curr = simple_candle(99, 116, 98, 115); // tăng, bao trọn
        assert!(is_bullish_engulfing(&prev, &curr));
        // Không bao trọn thì không tính
        let narrow = simple_candle(102, 110, 101, 108);
        assert!(!is_bullish_engulfing(&prev, &narrow));
        // Hôm qua phải là nến GIẢM
        assert!(!is_bullish_engulfing(
            &simple_candle(100, 116, 98, 112),
            &curr
        ));
    }

    #[test]
    fn detection_is_deterministic_and_never_looks_ahead() {
        // Bất biến sống còn: thay đổi các nến SAU nến i không được đổi kết quả
        // tại nến i. Vi phạm điều này là "vẽ lại" (repainting).
        let candles = gen_candle(300, 7);
        let mut other_future = candles.clone();
        for c in other_future.iter_mut().skip(150) {
            *c = Candle {
                open: c.close,
                close: c.open,
                ..*c
            }; // đảo hướng mọi nến từ 150 trở đi
        }
        for i in 0..150 {
            assert_eq!(
                detect_pattern(&candles[..=i]),
                detect_pattern(&other_future[..=i]),
                "tương lai khác nhau không được đổi kết quả tại nến {}",
                i
            );
        }
    }

    #[test]
    fn an_empty_series_has_no_patterns() {
        assert_eq!(detect_pattern(&[]), Pattern::NoPattern);
    }

    // ---------- Trung bình động ----------
    #[test]
    fn sma_matches_known_values() {
        assert_eq!(sma(&[1.0, 2.0, 3.0, 4.0, 5.0], 5), Some(3.0));
        assert_eq!(
            sma(&[1.0, 2.0, 3.0, 4.0, 5.0], 3),
            Some(4.0),
            "chỉ lấy 3 giá cuối"
        );
        assert_eq!(sma(&[1.0, 2.0], 5), None, "chưa đủ dữ liệu");
        assert_eq!(sma(&[1.0], 0), None, "chu kỳ 0 vô nghĩa");
    }

    #[test]
    fn sma_returns_none_rather_than_garbage_when_cold() {
        // Trả 0 hay trả trung bình của số ít nến sẽ khiến chiến lược vào lệnh
        // trên dữ liệu không đủ — lỗi âm thầm và tốn tiền.
        let c = sma_series(&[1.0, 2.0, 3.0, 4.0, 5.0], 3);
        assert_eq!(c[0], None);
        assert_eq!(c[1], None);
        assert_eq!(c[2], Some(2.0));
        assert_eq!(c[4], Some(4.0));
    }

    #[test]
    fn ema_tracks_price_more_closely_than_sma() {
        // Giá nhảy bậc: EMA phải phản ứng nhanh hơn SMA.
        let mut price = vec![100.0; 30];
        for x in price.iter_mut().skip(20) {
            *x = 200.0;
        }
        let s = sma_series(&price, 10);
        let e = ema_series(&price, 10);
        let i = 24; // 5 phiên sau cú nhảy
        assert!(
            e[i].unwrap() > s[i].unwrap(),
            "EMA {:.1} phải cao hơn SMA {:.1}",
            e[i].unwrap(),
            s[i].unwrap()
        );
    }

    #[test]
    fn ema_converges_to_a_constant_price() {
        let price = vec![100.0; 200];
        let e = ema_series(&price, 20);
        assert!(
            (e[199].unwrap() - 100.0).abs() < 1e-9,
            "giá đứng yên thì EMA phải bằng đúng giá đó"
        );
    }

    #[test]
    fn ema_is_seeded_with_an_sma() {
        let price: Vec<f64> = (1..=20).map(|i| i as f64).collect();
        let e = ema_series(&price, 10);
        assert_eq!(
            e[9],
            Some(5.5),
            "giá trị đầu tiên là SMA của 10 phần tử đầu"
        );
        assert_eq!(e[8], None, "trước đó chưa đủ dữ liệu");
    }

    #[test]
    fn wma_weights_recent_prices_more() {
        // [1,2,3] với trọng số [1,2,3] → (1+4+9)/6 = 2.333
        let w = wma(&[1.0, 2.0, 3.0], 3).unwrap();
        assert!((w - 14.0 / 6.0).abs() < 1e-9);
        assert!(
            w > sma(&[1.0, 2.0, 3.0], 3).unwrap(),
            "chuỗi tăng thì WMA phải cao hơn SMA"
        );
    }

    // ---------- RSI ----------
    #[test]
    fn rsi_is_100_on_an_unbroken_rally() {
        let price: Vec<f64> = (1..=50).map(|i| i as f64 * 100.0).collect();
        let r = rsi_series(&price, 14);
        assert!(
            (r[49].unwrap() - 100.0).abs() < 1e-6,
            "không có phiên giảm nào → RSI = 100"
        );
    }

    #[test]
    fn rsi_is_0_on_an_unbroken_selloff() {
        let price: Vec<f64> = (1..=50).rev().map(|i| i as f64 * 100.0).collect();
        let r = rsi_series(&price, 14);
        assert!(r[49].unwrap() < 1e-6, "không có phiên tăng nào → RSI = 0");
    }

    #[test]
    fn rsi_is_50_when_price_is_flat() {
        let price = vec![100.0; 50];
        assert!(
            (rsi_series(&price, 14)[49].unwrap() - 50.0).abs() < 1e-9,
            "không tăng không giảm → trung tính, và không chia cho 0"
        );
    }

    #[test]
    fn rsi_always_stays_within_0_and_100() {
        for seed in [1u64, 42, 2024, 31337] {
            let price = close_price(&gen_candle(500, seed));
            for x in rsi_series(&price, 14).into_iter().flatten() {
                assert!((0.0..=100.0).contains(&x), "RSI ra ngoài thang: {}", x);
            }
        }
    }

    #[test]
    fn rsi_is_none_until_warm() {
        let price: Vec<f64> = (1..=10).map(|i| i as f64).collect();
        let r = rsi_series(&price, 14);
        assert!(r.iter().all(|x| x.is_none()), "10 giá không đủ cho RSI(14)");
    }

    // ---------- MACD ----------
    #[test]
    fn the_macd_histogram_is_the_difference_of_the_two_lines() {
        let price = close_price(&gen_candle(200, 5));
        for m in macd_series(&price, 12, 26, 9).into_iter().flatten() {
            assert!((m.histogram - (m.macd - m.signal)).abs() < 1e-9);
        }
    }

    #[test]
    fn macd_is_positive_in_an_uptrend() {
        // Giá tăng đều → EMA nhanh phải nằm trên EMA chậm → MACD dương.
        let price: Vec<f64> = (1..=200).map(|i| 10_000.0 + i as f64 * 10.0).collect();
        let m = macd_series(&price, 12, 26, 9);
        assert!(
            m[199].unwrap().macd > 0.0,
            "xu hướng tăng phải cho MACD dương"
        );
    }

    #[test]
    fn macd_is_negative_in_a_downtrend() {
        let price: Vec<f64> = (1..=200).map(|i| 10_000.0 - i as f64 * 10.0).collect();
        let m = macd_series(&price, 12, 26, 9);
        assert!(m[199].unwrap().macd < 0.0);
    }

    #[test]
    fn macd_is_none_until_warm() {
        let price: Vec<f64> = (1..=20).map(|i| i as f64).collect();
        assert!(
            macd_series(&price, 12, 26, 9).iter().all(|x| x.is_none()),
            "20 giá không đủ cho MACD(12,26,9)"
        );
    }

    // ---------- Bollinger ----------
    #[test]
    fn the_bollinger_middle_band_equals_the_sma() {
        let price: Vec<f64> = (1..=30).map(|i| i as f64).collect();
        let b = bollinger(&price, 20, 2.0).unwrap();
        assert_eq!(b.middle, sma(&price, 20).unwrap());
    }

    #[test]
    fn the_bands_are_symmetric_about_the_middle() {
        let price = close_price(&gen_candle(100, 9));
        let b = bollinger(&price, 20, 2.0).unwrap();
        assert!(
            ((b.upper - b.middle) - (b.middle - b.lower)).abs() < 1e-9,
            "hai dải phải cách đều đường giữa"
        );
        assert!(b.upper >= b.middle && b.middle >= b.lower);
    }

    #[test]
    fn the_bands_narrow_when_volatility_falls() {
        let calm = vec![100.0; 30];
        let choppy: Vec<f64> = (0..30).map(|i| 100.0 + ((i % 2) as f64) * 50.0).collect();
        let a = bollinger(&calm, 20, 2.0).unwrap();
        let b = bollinger(&choppy, 20, 2.0).unwrap();
        assert!(
            a.bandwidth() < b.bandwidth(),
            "giá đứng yên → dải hẹp gần bằng 0"
        );
        assert!(a.bandwidth() < 1e-9);
    }

    #[test]
    fn percent_b_is_exact_at_both_bands() {
        let b = BollingerBands {
            upper: 120.0,
            middle: 100.0,
            lower: 80.0,
        };
        assert!((b.percent_b(80.0) - 0.0).abs() < 1e-9);
        assert!((b.percent_b(100.0) - 0.5).abs() < 1e-9);
        assert!((b.percent_b(120.0) - 1.0).abs() < 1e-9);
        // Dải rỗng không được chia cho 0
        let narrow = BollingerBands {
            upper: 100.0,
            middle: 100.0,
            lower: 100.0,
        };
        assert_eq!(narrow.percent_b(100.0), 0.5);
    }

    // ---------- ATR ----------
    #[test]
    fn true_range_includes_the_overnight_gap() {
        let prev = simple_candle(100, 105, 95, 100);
        // Phiên sau nhảy vọt lên: biên độ trong phiên chỉ 5, nhưng khoảng
        // cách so với giá đóng hôm trước là 30 — ATR phải thấy điều đó.
        let curr = simple_candle(128, 130, 125, 129);
        assert_eq!(curr.range(), 5);
        assert_eq!(
            true_range(&curr, Some(&prev)),
            30,
            "phải bắt được khoảng nhảy"
        );
    }

    #[test]
    fn the_first_candle_true_range_is_the_plain_range() {
        let n = simple_candle(100, 110, 90, 105);
        assert_eq!(true_range(&n, None), 20);
    }

    #[test]
    fn atr_is_always_positive() {
        for seed in [1u64, 42, 2024] {
            let candle = gen_candle(300, seed);
            for a in atr_series(&candle, 14).into_iter().flatten() {
                assert!(a > 0.0, "ATR phải dương, thực tế {}", a);
            }
        }
    }

    #[test]
    fn atr_rises_with_volatility() {
        let calm: Vec<Candle> = (0..50)
            .map(|i| Candle {
                timestamp: i,
                open: 10_000,
                high: 10_010,
                low: 9_990,
                close: 10_000,
                volume: 1,
            })
            .collect();
        let choppy: Vec<Candle> = (0..50)
            .map(|i| Candle {
                timestamp: i,
                open: 10_000,
                high: 10_500,
                low: 9_500,
                close: 10_000,
                volume: 1,
            })
            .collect();
        let a = atr_series(&calm, 14)[49].unwrap();
        let b = atr_series(&choppy, 14)[49].unwrap();
        assert!(
            b > a * 10.0,
            "thị trường xóc gấp 50 lần phải cho ATR lớn hơn hẳn"
        );
    }

    #[test]
    fn atr_is_none_until_warm() {
        let candle = gen_candle(10, 1);
        assert!(atr_series(&candle, 14).iter().all(|x| x.is_none()));
    }

    #[test]
    fn atr_sizing_shrinks_as_volatility_rises() {
        let mut prev = i64::MAX;
        for atr in [20.0f64, 50.0, 100.0, 200.0] {
            let c = atr_position_size(100_000, atr, 2.0);
            assert!(c < prev, "ATR {} phải cho cỡ nhỏ hơn", atr);
            prev = c;
        }
        assert_eq!(
            atr_position_size(100_000, 20.0, 2.0),
            2_500,
            "100000 / (20 × 2)"
        );
    }

    #[test]
    fn atr_sizing_is_safe_on_bad_input() {
        assert_eq!(atr_position_size(100_000, 0.0, 2.0), 0, "không chia cho 0");
        assert_eq!(atr_position_size(100_000, 20.0, 0.0), 0);
    }

    // ---------- Không nhìn trước tương lai ----------
    #[test]
    fn no_indicator_peeks_at_the_future() {
        // BẤT BIẾN QUAN TRỌNG NHẤT của chương: giá trị chỉ báo tại nến i phải
        // giống hệt nhau dù ta đưa vào 201 nến hay 500 nến. Vi phạm điều này
        // là "nhìn trộm tương lai", và mọi kết quả kiểm định trở nên vô nghĩa.
        let candle = gen_candle(500, 2024);
        let price = close_price(&candle);
        let i = 200;

        assert_eq!(sma_series(&price[..=i], 20)[i], sma_series(&price, 20)[i]);
        assert_eq!(ema_series(&price[..=i], 20)[i], ema_series(&price, 20)[i]);
        assert_eq!(rsi_series(&price[..=i], 14)[i], rsi_series(&price, 14)[i]);
        assert_eq!(atr_series(&candle[..=i], 14)[i], atr_series(&candle, 14)[i]);
        assert_eq!(
            macd_series(&price[..=i], 12, 26, 9)[i],
            macd_series(&price, 12, 26, 9)[i]
        );
        // Bollinger chỉ được dùng đúng 20 giá cuối tính tới i
        assert_eq!(
            bollinger(&price[..=i], 20, 2.0),
            bollinger(&price[i - 19..=i], 20, 2.0)
        );
    }

    #[test]
    fn candle_generation_is_deterministic() {
        assert_eq!(gen_candle(100, 5), gen_candle(100, 5));
        assert_ne!(gen_candle(100, 5), gen_candle(100, 6));
    }
}
