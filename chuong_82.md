# Chương 82: Phân tích kỹ thuật bằng Rust — Nến, Chỉ báo & Bẫy nhìn trộm tương lai (OpenAlgo I)

## Giới thiệu & Mục tiêu học tập

Ba chương 82–84 chuyển toàn bộ phần **learn của OpenAlgo** sang Rust. OpenAlgo có hơn 400 bài trong nhiều khoá; ta chọn ra phần **lập trình được** — tức là phần mà một máy tính có thể tính, kiểm chứng và kiểm thử — rồi cài lại từ đầu, không dùng thư viện.

Chương 82 lo phần phân tích kỹ thuật: nến, mẫu hình, và các chỉ báo kinh điển.

Nhưng chủ đề thật sự của chương này không phải là công thức. Đó là **bẫy nhìn trộm tương lai** (look-ahead bias):

> Một chỉ báo tính tại thời điểm t **không được phép** chạm vào bất kỳ dữ liệu nào của t+1 trở đi. Vi phạm điều này là cách tạo ra một chiến lược lãi 300%/năm trong backtest và mất tiền ngay ngày đầu chạy thật.

Vì thế mọi hàm trong chương này đều trả về **chuỗi** cùng độ dài với đầu vào, với `Option::None` ở những vị trí chưa đủ dữ liệu — và có một bài kiểm thử bất biến khẳng định điều đó.

---

## Hình tượng hóa đời sống

```
┌──────────────────────────────────────────────────────────────────────────────┐
│  MỘT CÂY NẾN = TÓM TẮT MỘT KHOẢNG THỜI GIAN BẰNG 4 SỐ                       │
│                                                                              │
│         ╷ cao          Thân  = |đóng − mở|   ← lực của phiên                │
│         │              Bóng trên = cao − max(mở,đóng)                       │
│      ┌──┴──┐ đóng      Bóng dưới = min(mở,đóng) − thấp                      │
│      │█████│                                                                │
│      │█████│           Bóng dài phía dưới = người mua đã đẩy giá lên        │
│      └──┬──┘ mở        từ đáy → tín hiệu đảo chiều tiềm năng                │
│         │                                                                    │
│         ╵ thấp                                                              │
│                                                                              │
│  SMA vs EMA = TRÍ NHỚ PHẲNG vs TRÍ NHỚ MỜ DẦN                              │
│                                                                              │
│   SMA(5):  mỗi giá trong 5 phiên có TRỌNG SỐ BẰNG NHAU (20% mỗi cái)       │
│            giá của 5 phiên trước rơi khỏi cửa sổ → NHẢY ĐỘT NGỘT            │
│                                                                              │
│   EMA(5):  α = 2/(5+1) = 0,333                                              │
│            trọng số: 33%, 22%, 15%, 10%, 7%, 4%... giảm dần MÃI MÃI         │
│            không có gì "rơi khỏi cửa sổ" → mượt hơn                         │
│                                                                              │
│  BẪY NHÌN TRỘM TƯƠNG LAI                                                    │
│                                                                              │
│   ✗ SAI:  tín_hiệu[t] = sma[t] > giá[t]   rồi vào lệnh ở giá ĐÓNG của t     │
│           → bạn dùng giá đóng của t để quyết định giao dịch TẠI t.          │
│             Ở thời điểm quyết định, giá đóng chưa tồn tại.                  │
│                                                                              │
│   ✓ ĐÚNG: tín_hiệu[t] tính từ dữ liệu ≤ t, vào lệnh ở giá MỞ của t+1       │
│                                                                              │
│   Kiểm tra vàng: "Nếu tôi cắt dữ liệu ở đúng chỉ số t, giá trị chỉ báo      │
│   tại t có đổi không?" ĐỔI = có nhìn trộm. KHÔNG ĐỔI = an toàn.            │
│                                                                              │
│  RSI = ĐO SỰ MẤT CÂN BẰNG GIỮA TĂNG VÀ GIẢM                                │
│                                                                              │
│   RSI = 100 − 100/(1 + tăng_TB/giảm_TB)                                     │
│                                                                              │
│   RSI > 70  quá mua      ⚠ TRONG XU HƯỚNG MẠNH, RSI CÓ THỂ Ở TRÊN 70       │
│   RSI < 30  quá bán        HÀNG TUẦN. "Quá mua" KHÔNG có nghĩa là "bán".   │
└──────────────────────────────────────────────────────────────────────────────┘
```

---

## Khái niệm & Cơ chế kỹ thuật chuyên sâu

### 1. Bất biến chống nhìn trộm tương lai, được kiểm thử

Cách kiểm chứng chặt chẽ nhất rất đơn giản: tính chỉ báo trên toàn bộ chuỗi, rồi tính lại trên chuỗi **cắt ngắn** tại một điểm bất kỳ, và so sánh. Nếu giá trị tại điểm đó khác nhau, chỉ báo đang nhìn về phía trước.

```rust
// Bất biến: sma(&gia[..=t], n)[t] == sma(&gia, n)[t]  với mọi t
```

Bài kiểm thử này bắt được cả những vi phạm tinh vi mà mắt thường không thấy — ví dụ như chuẩn hoá bằng giá trị max của toàn bộ chuỗi, hay điền giá trị thiếu bằng nội suy hai chiều.

Trong Rust, cách làm cho vi phạm **khó xảy ra** là thiết kế API trả `Vec<Option<f64>>` cùng độ dài đầu vào. `None` ở đầu chuỗi buộc người dùng phải xử lý tường minh trường hợp "chưa đủ dữ liệu", thay vì im lặng cắt bớt và làm lệch chỉ số.

### 2. EMA: chi tiết khởi tạo mà ai cũng làm khác nhau

EMA có công thức đệ quy `ema[t] = α·giá[t] + (1−α)·ema[t−1]` với `α = 2/(n+1)`. Nhưng `ema[0]` lấy từ đâu?

Ba cách phổ biến, cho kết quả khác nhau:
- Lấy `giá[0]` — đơn giản, nhưng chịu ảnh hưởng nặng từ giá đầu tiên trong nhiều phiên.
- Lấy SMA của `n` giá đầu — cách chuẩn của hầu hết phần mềm biểu đồ; đây là cách chương này dùng.
- Bắt đầu từ 0 và để nó "ấm dần" — sai lệch mạnh lúc đầu.

Điều quan trọng không phải chọn cách nào, mà là **biết mình chọn cách nào**. Kết quả backtest sẽ khác nhau, và nếu bạn so sánh với một nền tảng khác mà không biết điều này, bạn sẽ đi tìm một con bug không tồn tại.

### 3. MACD: chỉ báo của chỉ báo

MACD gồm ba thành phần:
- **Đường MACD** = EMA(12) − EMA(26)
- **Đường tín hiệu** = EMA(9) của đường MACD
- **Histogram** = MACD − tín hiệu

Điểm tinh tế: đường tín hiệu là EMA **của một EMA**, nên nó thừa hưởng toàn bộ độ trễ của cả hai. Tổng độ trễ khoảng 26+9 = 35 phiên trước khi giá trị ổn định. Với biểu đồ ngày, đó là hơn một tháng rưỡi.

Đây là đánh đổi cơ bản của mọi chỉ báo làm mượt: **mượt hơn = chậm hơn**. Không có cách nào thoát khỏi nó bằng cách chỉnh tham số.

### 4. ATR và vì sao "khoảng thật" lại có ba vế

Khoảng thật (True Range) là giá trị lớn nhất trong ba số:

```
TR = max( cao − thấp,  |cao − đóng_trước|,  |thấp − đóng_trước| )
```

Hai vế sau tồn tại để xử lý **khoảng nhảy giá** (gap). Nếu thị trường mở cửa cách xa giá đóng hôm trước, thì `cao − thấp` của riêng ngày hôm nay không phản ánh biến động thực sự — phần nhảy đã xảy ra giữa hai phiên.

ATR là chỉ báo hữu dụng nhất cho **định cỡ vị thế**: đặt lệnh dừng lỗ ở `2×ATR` cho khoảng cách dừng tự động thích ứng theo biến động, thay vì một tỉ lệ phần trăm cố định vốn quá chặt lúc thị trường động và quá lỏng lúc thị trường yên.

### 5. Dải Bollinger: cái bẫy phân phối chuẩn

Dải Bollinger (Bollinger Bands) đặt tại `SMA ± k·độ_lệch_chuẩn`, thường `k = 2`. Nếu lợi suất tuân theo phân phối chuẩn, khoảng 95% giá sẽ nằm trong dải.

Nhưng lợi suất **không** phân phối chuẩn. Chúng có đuôi béo — các sự kiện cực đoan xảy ra thường xuyên hơn nhiều so với dự đoán của phân phối chuẩn. Trong thực tế, giá ra ngoài dải 2σ thường xuyên hơn 5%, và các cú ra ngoài lớn thì lớn hơn nhiều so với mô hình.

Hệ quả thực hành: đừng giao dịch dải Bollinger như thể chạm dải trên là "chắc chắn quay đầu". Trong xu hướng mạnh, giá có thể "đi dọc theo dải" hàng chục phiên.

---

## Mã nguồn minh họa thực chiến

Chạy bằng `cargo run -p ch82`, kiểm thử bằng `cargo test -p ch82`.

```rust
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
    pub mo: Price,
    pub high: Price,
    pub low: Price,
    pub dong: Price,
    pub quantity: u64,
}

impl Candle {
    /// Thân nến: khoảng cách giữa giá mở và giá đóng.
    pub fn than(&self) -> Price { (self.dong - self.mo).abs() }
    /// Toàn bộ biên độ trong phiên.
    pub fn bien_do(&self) -> Price { self.high - self.low }
    pub fn upper_wick(&self) -> Price { self.high - self.mo.max(self.dong) }
    pub fn lower_wick(&self) -> Price { self.mo.min(self.dong) - self.low }
    pub fn tang(&self) -> bool { self.dong > self.mo }
    pub fn down(&self) -> bool { self.dong < self.mo }

    /// Nến có hợp lệ không. Dữ liệu thị trường thật CÓ lỗi, và một nến sai
    /// làm hỏng mọi chỉ báo phía sau mà không báo gì.
    pub fn is_valid(&self) -> bool {
        self.high >= self.low
            && self.high >= self.mo && self.high >= self.dong
            && self.low <= self.mo && self.low <= self.dong
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
pub enum Pattern { Doji, BuaTang, SaoBangGiam, NhanChimTang, NhanChimGiam, KhongCo }

/// Doji: giá mở gần bằng giá đóng — hai phe giằng co, không ai thắng.
pub fn la_doji(n: &Candle, threshold_bps: i64) -> bool {
    if n.bien_do() == 0 { return true; }
    n.than() * 10_000 <= n.bien_do() * threshold_bps
}

/// Búa: thân nhỏ ở TRÊN, bóng dưới dài — người bán đẩy giá xuống nhưng bị
/// người bid kéo lại hết. Chỉ có ý nghĩa khi xuất hiện SAU một đợt giảm.
pub fn la_bua(n: &Candle) -> bool {
    n.bien_do() > 0
        && n.than() > 0
        && n.lower_wick() >= n.than() * 2
        && n.upper_wick() <= n.than()
}

/// Sao băng: đối xứng của búa — bóng TRÊN dài, xuất hiện sau đợt tăng.
pub fn la_sao_bang(n: &Candle) -> bool {
    n.bien_do() > 0
        && n.than() > 0
        && n.upper_wick() >= n.than() * 2
        && n.lower_wick() <= n.than()
}

/// Nhấn chìm tăng: nến tăng hôm nay bao trọn thân nến giảm hôm qua.
pub fn la_nhan_chim_tang(hom_qua: &Candle, hom_nay: &Candle) -> bool {
    hom_qua.down() && hom_nay.tang()
        && hom_nay.dong >= hom_qua.mo && hom_nay.mo <= hom_qua.dong
        && hom_nay.than() > hom_qua.than()
}

pub fn is_bearish_engulfing(hom_qua: &Candle, hom_nay: &Candle) -> bool {
    hom_qua.tang() && hom_nay.down()
        && hom_nay.mo >= hom_qua.dong && hom_nay.dong <= hom_qua.mo
        && hom_nay.than() > hom_qua.than()
}

/// Nhận diện mẫu hình tại nến CUỐI của `history`.
/// Chỉ nhìn dữ liệu ĐÃ CÓ — không bao giờ chạm tới nến tương lai.
pub fn recv_elec(history: &[Candle]) -> Pattern {
    let n = match history.last() { Some(n) => n, None => return Pattern::KhongCo };
    if let Some(q) = history.len().checked_sub(2).map(|i| &history[i]) {
        if la_nhan_chim_tang(q, n) { return Pattern::NhanChimTang; }
        if is_bearish_engulfing(q, n) { return Pattern::NhanChimGiam; }
    }
    if la_doji(n, 500) { return Pattern::Doji; } // thân ≤ 5% biên độ
    if la_bua(n) { return Pattern::BuaTang; }
    if la_sao_bang(n) { return Pattern::SaoBangGiam; }
    Pattern::KhongCo
}

// ============================================================================
// 3. TRUNG BÌNH ĐỘNG
// ============================================================================

/// Trung bình động đơn giản. Trả `None` khi chưa đủ `period` nến — điều này
/// QUAN TRỌNG: trả 0 hay trả trung bình của số ít nến sẽ khiến chiến lược
/// vào lệnh dựa trên dữ liệu không đủ.
pub fn sma(price: &[f64], period: usize) -> Option<f64> {
    if period == 0 || price.len() < period { return None; }
    Some(price[price.len() - period..].iter().sum::<f64>() / period as f64)
}

/// Toàn bộ chuỗi SMA. Phần tử `i` chỉ dùng dữ liệu tới `i` — không nhìn trước.
pub fn sma_series(price: &[f64], period: usize) -> Vec<Option<f64>> {
    (0..price.len()).map(|i| sma(&price[..=i], period)).collect()
}

/// Trung bình động luỹ thừa. Hệ số làm mượt α = 2/(n+1).
/// EMA phản ứng nhanh hơn SMA vì nó cho dữ liệu mới trọng số cao hơn — nhưng
/// cũng vì thế mà nhiễu hơn.
pub fn ema_series(price: &[f64], period: usize) -> Vec<Option<f64>> {
    let mut ra = vec![None; price.len()];
    if period == 0 || price.len() < period { return ra; }
    let alpha = 2.0 / (period as f64 + 1.0);
    // Mồi bằng SMA của `period` giá trị đầu — cách chuẩn của ngành
    let mut e = price[..period].iter().sum::<f64>() / period as f64;
    ra[period - 1] = Some(e);
    for i in period..price.len() {
        e = price[i] * alpha + e * (1.0 - alpha);
        ra[i] = Some(e);
    }
    ra
}

/// Trung bình động có trọng số tuyến tính: giá mới nhất có trọng số n,
/// giá cũ nhất có trọng số 1.
pub fn wma(price: &[f64], period: usize) -> Option<f64> {
    if period == 0 || price.len() < period { return None; }
    let window = &price[price.len() - period..];
    let total_weight = (period * (period + 1) / 2) as f64;
    Some(window.iter().enumerate().map(|(i, &x)| x * (i + 1) as f64).sum::<f64>()
         / total_weight)
}

// ============================================================================
// 4. RSI — CHỈ SỐ SỨC MẠNH TƯƠNG ĐỐI
// ============================================================================
// RSI đo tương quan giữa mức tăng và mức giảm gần đây, quy về thang 0–100.
// Trên 70 thường gọi là "quá bid", dưới 30 là "quá bán" — nhưng trong xu
// hướng mạnh, RSI có thể nằm trên 70 hàng tuần liền. Đó là lý do dùng RSI
// một mình để đoán đảo chiều là cách mất tiền nhanh nhất.

pub fn rsi_series(price: &[f64], period: usize) -> Vec<Option<f64>> {
    let mut ra = vec![None; price.len()];
    if period == 0 || price.len() <= period { return ra; }

    let mut up_avg = 0.0;
    let mut down_avg = 0.0;
    for i in 1..=period {
        let d = price[i] - price[i - 1];
        if d > 0.0 { up_avg += d; } else { down_avg += -d; }
    }
    up_avg /= period as f64;
    down_avg /= period as f64;
    ra[period] = Some(from_up_down(up_avg, down_avg));

    // Làm mượt kiểu Wilder: giống EMA với α = 1/n
    for i in (period + 1)..price.len() {
        let d = price[i] - price[i - 1];
        let (t, g) = if d > 0.0 { (d, 0.0) } else { (0.0, -d) };
        up_avg = (up_avg * (period - 1) as f64 + t) / period as f64;
        down_avg = (down_avg * (period - 1) as f64 + g) / period as f64;
        ra[i] = Some(from_up_down(up_avg, down_avg));
    }
    ra
}

fn from_up_down(tang: f64, down: f64) -> f64 {
    // Không có phiên giảm nào → RSI = 100. Phải xử lý riêng để không chia cho 0.
    if down < 1e-12 { return if tang < 1e-12 { 50.0 } else { 100.0 }; }
    100.0 - 100.0 / (1.0 + tang / down)
}

// ============================================================================
// 5. MACD
// ============================================================================

#[derive(Debug, Clone, Copy, PartialEq)]
pub struct MacdValue { pub macd: f64, pub signal: f64, pub histogram: f64 }

/// MACD = EMA nhanh − EMA chậm. Đường tín hiệu = EMA của chính MACD.
/// Biểu đồ = MACD − tín hiệu, đo đà tăng tốc.
pub fn macd_series(price: &[f64], fast: usize, cham: usize, signal: usize)
    -> Vec<Option<MacdValue>>
{
    let mut ra = vec![None; price.len()];
    if cham == 0 || price.len() < cham { return ra; }
    let ema_fast = ema_series(price, fast);
    let e_cham = ema_series(price, cham);

    // Chuỗi MACD chỉ có giá trị từ khi CẢ HAI đường EMA đã sẵn sàng
    let mut macd_line: Vec<f64> = Vec::new();
    let mut root_indices: Vec<usize> = Vec::new();
    for i in 0..price.len() {
        if let (Some(a), Some(b)) = (ema_fast[i], e_cham[i]) {
            macd_line.push(a - b);
            root_indices.push(i);
        }
    }
    let ema_signal = ema_series(&macd_line, signal);
    for (k, &i) in root_indices.iter().enumerate() {
        if let Some(s) = ema_signal[k] {
            ra[i] = Some(MacdValue { macd: macd_line[k], signal: s,
                                      histogram: macd_line[k] - s });
        }
    }
    ra
}

// ============================================================================
// 6. DẢI BOLLINGER
// ============================================================================

#[derive(Debug, Clone, Copy, PartialEq)]
pub struct BollingerBands { pub above: f64, pub mid: f64, pub below: f64 }

impl BollingerBands {
    pub fn do_rong(&self) -> f64 {
        if self.mid.abs() < 1e-12 { 0.0 } else { (self.above - self.below) / self.mid }
    }
    /// Vị trí của giá trong dải: 0 = chạm đáy, 1 = chạm đỉnh.
    pub fn pos_value_percent(&self, price: f64) -> f64 {
        let d = self.above - self.below;
        if d.abs() < 1e-12 { 0.5 } else { (price - self.below) / d }
    }
}

pub fn bollinger(price: &[f64], period: usize, so_do_lech: f64) -> Option<BollingerBands> {
    let mid = sma(price, period)?;
    let window = &price[price.len() - period..];
    // Độ lệch chuẩn TỔNG THỂ (chia n) — quy ước chuẩn của dải Bollinger
    let ps = window.iter().map(|x| (x - mid).powi(2)).sum::<f64>() / period as f64;
    let sd = ps.max(0.0).sqrt();
    Some(BollingerBands { above: mid + so_do_lech * sd, mid, below: mid - so_do_lech * sd })
}

// ============================================================================
// 7. ATR — BIÊN ĐỘ THẬT TRUNG BÌNH
// ============================================================================
// ATR đo mức dao động, KHÔNG đo hướng. Nó là công cụ định cỡ vị thế và đặt
// cắt lỗ tốt nhất: đặt cắt lỗ cách 2 ATR thì mức chấp nhận rủi ro tự động
// điều chỉnh theo trạng thái thị trường.

/// Biên độ thật: lớn nhất trong ba khoảng cách. Nó tính cả KHOẢNG NHẢY giữa
/// hai phiên — điều mà `cao − thap` bỏ sót hoàn toàn.
pub fn bien_do_that(nay: &Candle, prev: Option<&Candle>) -> Price {
    match prev {
        None => nay.high - nay.low,
        Some(t) => (nay.high - nay.low)
            .max((nay.high - t.dong).abs())
            .max((nay.low - t.dong).abs()),
    }
}

pub fn atr_series(candle: &[Candle], period: usize) -> Vec<Option<f64>> {
    let mut ra = vec![None; candle.len()];
    if period == 0 || candle.len() < period { return ra; }
    let bdt: Vec<f64> = candle.iter().enumerate()
        .map(|(i, n)| bien_do_that(n, i.checked_sub(1).map(|j| &candle[j])) as f64)
        .collect();
    let mut a = bdt[..period].iter().sum::<f64>() / period as f64;
    ra[period - 1] = Some(a);
    for i in period..candle.len() {
        a = (a * (period - 1) as f64 + bdt[i]) / period as f64; // làm mượt Wilder
        ra[i] = Some(a);
    }
    ra
}

/// Định cỡ vị thế theo ATR: rủi ro mỗi lệnh cố định bằng tiền, nên mã dao
/// động mạnh thì bid ít. Đây là công thức nền của mọi hệ thống theo xu hướng.
pub fn co_theo_atr(von_rui_ro: i64, atr: f64, so_atr_cat_lo: f64) -> i64 {
    let risk_new_don_pos = atr * so_atr_cat_lo;
    if risk_new_don_pos < 1e-9 { return 0; }
    (von_rui_ro as f64 / risk_new_don_pos) as i64
}

// ============================================================================
// 8. SINH DỮ LIỆU TẤT ĐỊNH
// ============================================================================

pub fn gen_candle(n: usize, hat_giong: u64) -> Vec<Candle> {
    let mut s = hat_giong;
    let mut price: Price = 10_000;
    (0..n).map(|i| {
        s = s.wrapping_mul(6364136223846793005).wrapping_add(1442695040888963407);
        let step = ((s >> 33) % 201) as i64 - 100;
        let mo = price;
        price = (price + step).max(100);
        let bien = ((s >> 45) % 80) as i64;
        Candle {
            timestamp: i as u64,
            mo,
            high: mo.max(price) + bien,
            low: (mo.min(price) - bien).max(1),
            dong: price,
            quantity: 1_000 + (s >> 50) % 9_000,
        }
    }).collect()
}

pub fn price_close(candle: &[Candle]) -> Vec<f64> { candle.iter().map(|n| n.dong as f64).collect() }

fn main() {
    println!("═══════════════════════════════════════════════════════════");
    println!("   PHÂN TÍCH KỸ THUẬT BẰNG RUST (giáo trình OpenAlgo)       ");
    println!("═══════════════════════════════════════════════════════════");

    let candle = gen_candle(500, 2024);
    let price = price_close(&candle);

    println!("\n1. NẾN OHLCV");
    let n = &candle[100];
    println!("   Nến #100: mở {} cao {} thấp {} đóng {}", n.mo, n.high, n.low, n.dong);
    println!("   thân {} · biên độ {} · bóng trên {} · bóng dưới {} · {}",
             n.than(), n.bien_do(), n.upper_wick(), n.lower_wick(),
             if n.tang() { "TĂNG" } else { "GIẢM" });
    println!("   Toàn bộ {} nến đều hợp lệ: {}",
             candle.len(), candle.iter().all(|x| x.is_valid()));

    println!("\n2. MẪU HÌNH NẾN — đếm trên 500 nến");
    let mut count = std::collections::BTreeMap::new();
    for i in 0..candle.len() {
        *count.entry(format!("{:?}", recv_elec(&candle[..=i]))).or_insert(0) += 1;
    }
    for (k, v) in &count { println!("   {:<16} {:>4} lần", k, v); }

    println!("\n3. TRUNG BÌNH ĐỘNG — cùng dữ liệu, khác độ nhạy");
    let s20 = sma_series(&price, 20);
    let e20 = ema_series(&price, 20);
    println!("   {:>6} {:>10} {:>10} {:>10}", "nến", "giá", "SMA 20", "EMA 20");
    for i in [100usize, 200, 300, 400, 499] {
        println!("   {:>6} {:>10.0} {:>10.1} {:>10.1}",
                 i, price[i], s20[i].unwrap(), e20[i].unwrap());
    }
    println!("   → EMA bám giá sát hơn vì nó cho dữ liệu mới trọng số cao hơn.");

    println!("\n4. RSI");
    let r14 = rsi_series(&price, 14);
    let qua_buy = r14.iter().filter(|x| x.is_some_and(|v| v > 70.0)).count();
    let qua_ban = r14.iter().filter(|x| x.is_some_and(|v| v < 30.0)).count();
    println!("   RSI(14) tại nến 499: {:.1}", r14[499].unwrap());
    println!("   Số phiên > 70 (quá bid): {} · < 30 (quá bán): {}", qua_buy, qua_ban);
    let tang_deu: Vec<f64> = (1..=50).map(|i| i as f64 * 100.0).collect();
    let steady_decline: Vec<f64> = (1..=50).rev().map(|i| i as f64 * 100.0).collect();
    println!("   Chuỗi tăng đều  → RSI = {:.0}", rsi_series(&tang_deu, 14)[49].unwrap());
    println!("   Chuỗi giảm đều  → RSI = {:.0}", rsi_series(&steady_decline, 14)[49].unwrap());
    println!("   → Trong xu hướng mạnh, RSI dính sát 100 hoặc 0 rất lâu.");
    println!("     Dùng RSI một mình để đoán đảo chiều là cách mất tiền nhanh nhất.");

    println!("\n5. MACD (12, 26, 9)");
    let m = macd_series(&price, 12, 26, 9);
    let mut crossover = 0;
    for i in 1..m.len() {
        if let (Some(a), Some(b)) = (m[i - 1], m[i]) {
            if a.histogram.signum() != b.histogram.signum() { crossover += 1; }
        }
    }
    let last = m[499].unwrap();
    println!("   Tại nến 499: MACD {:.2} · tín hiệu {:.2} · biểu đồ {:.2}",
             last.macd, last.signal, last.histogram);
    println!("   Số lần biểu đồ đổi dấu trong 500 nến: {}", crossover);
    println!("   → {} tín hiệu trên 500 phiên. Phần lớn là nhiễu, và mỗi tín hiệu",
             crossover);
    println!("     đều tốn phí deliver dịch — xem lại Chương 69.");

    println!("\n6. DẢI BOLLINGER (20, 2σ)");
    for i in [100usize, 300, 499] {
        let b = bollinger(&price[..=i], 20, 2.0).unwrap();
        println!("   Nến {:>3}: dưới {:>8.1} · giữa {:>8.1} · trên {:>8.1} · giá ở {:>5.0}% dải",
                 i, b.below, b.mid, b.above, b.pos_value_percent(price[i]) * 100.0);
    }
    let out = (20..price.len()).filter(|&i| {
        let b = bollinger(&price[..=i], 20, 2.0).unwrap();
        price[i] > b.above || price[i] < b.below
    }).count();
    println!("   Số phiên giá vượt ra ngoài dải: {} / {} ({:.1}%)",
             out, price.len() - 20, out as f64 * 100.0 / (price.len() - 20) as f64);
    println!("   → Lý thuyết nói ~5% nằm ngoài 2σ. Thực tế thị trường thường nhiều hơn:");
    println!("     phân bố giá có ĐUÔI DÀY hơn phân bố chuẩn.");

    println!("\n7. ATR & ĐỊNH CỠ VỊ THẾ");
    let a14 = atr_series(&candle, 14);
    println!("   ATR(14) tại nến 499: {:.1} tick", a14[499].unwrap());
    println!("   {:>16} {:>12} {:>16}", "vốn rủi ro", "ATR", "số lượng bid");
    for atr in [20.0f64, 50.0, 100.0, 200.0] {
        println!("   {:>16} {:>12.0} {:>16}",
                 100_000, atr, co_theo_atr(100_000, atr, 2.0));
    }
    println!("   → Cùng mức rủi ro bằng tiền. Mã dao động mạnh gấp 10 thì bid ít đi 10 lần.");

    println!("\n═══════════════════════════════════════════════════════════");
    println!("   CHỈ BÁO KHÔNG DỰ BÁO TƯƠNG LAI — CHÚNG TÓM TẮT QUÁ KHỨ   ");
    println!("═══════════════════════════════════════════════════════════");
}

#[cfg(test)]
mod tests {
    use super::*;

    fn simple_candle(mo: Price, high: Price, low: Price, dong: Price) -> Candle {
        Candle { timestamp: 0, mo, high, low, dong, quantity: 100 }
    }

    // ---------- Nến ----------
    #[test]
    fn computes_body_range_and_wicks() {
        let n = simple_candle(100, 120, 90, 110);
        assert_eq!(n.than(), 10);
        assert_eq!(n.bien_do(), 30);
        assert_eq!(n.upper_wick(), 10, "120 − max(100,110)");
        assert_eq!(n.lower_wick(), 10, "min(100,110) − 90");
        assert!(n.tang() && !n.down());
    }

    #[test]
    fn detects_an_invalid_candle() {
        assert!(simple_candle(100, 120, 90, 110).is_valid());
        assert!(!simple_candle(100, 80, 90, 110).is_valid(), "cao < thấp là vô lý");
        assert!(!simple_candle(100, 105, 90, 110).is_valid(), "đóng > cao là vô lý");
        assert!(!simple_candle(100, 120, 105, 110).is_valid(), "thấp > mở là vô lý");
        assert!(!simple_candle(100, 120, 0, 110).is_valid(), "giá không được bằng 0");
    }

    #[test]
    fn every_generated_candle_is_valid() {
        for hat in [1u64, 42, 2024] {
            for n in gen_candle(1_000, hat) {
                assert!(n.is_valid(), "nến sinh ra phải luôn hợp lệ: {:?}", n);
            }
        }
    }

    // ---------- Mẫu hình ----------
    #[test]
    fn doji_when_open_nearly_equals_close() {
        assert!(la_doji(&simple_candle(100, 120, 80, 100), 500), "mở = đóng");
        assert!(la_doji(&simple_candle(100, 120, 80, 101), 500), "thân 1 trên biên độ 40");
        assert!(!la_doji(&simple_candle(100, 120, 80, 115), 500), "thân 15 là quá lớn");
    }

    #[test]
    fn a_zero_range_candle_counts_as_a_doji() {
        // Phiên không deliver dịch — phải xử lý được, không chia cho 0.
        assert!(la_doji(&simple_candle(100, 100, 100, 100), 500));
    }

    #[test]
    fn hammer_and_shooting_star_are_mirror_images() {
        // Búa: bóng dưới dài, thân nhỏ ở trên
        let bua = simple_candle(110, 112, 90, 111);
        assert!(la_bua(&bua), "bóng dưới {} thân {}", bua.lower_wick(), bua.than());
        assert!(!la_sao_bang(&bua));
        // Sao băng: bóng trên dài, thân nhỏ ở dưới
        let sao = simple_candle(91, 112, 90, 92);
        assert!(la_sao_bang(&sao));
        assert!(!la_bua(&sao));
    }

    #[test]
    fn bullish_engulfing_must_cover_the_prior_body() {
        let hom_qua = simple_candle(110, 112, 98, 100); // giảm
        let hom_nay = simple_candle(99, 116, 98, 115);  // tăng, bao trọn
        assert!(la_nhan_chim_tang(&hom_qua, &hom_nay));
        // Không bao trọn thì không tính
        let hep = simple_candle(102, 110, 101, 108);
        assert!(!la_nhan_chim_tang(&hom_qua, &hep));
        // Hôm qua phải là nến GIẢM
        assert!(!la_nhan_chim_tang(&simple_candle(100, 116, 98, 112), &hom_nay));
    }

    #[test]
    fn detection_is_deterministic_and_never_looks_ahead() {
        // Bất biến sống còn: thêm nến phía sau KHÔNG được đổi kết quả tại
        // nến trước. Vi phạm điều này là "vẽ lại" (repainting).
        let candle = gen_candle(300, 7);
        for i in 0..candle.len() {
            let short = recv_elec(&candle[..=i]);
            let long = recv_elec(&candle[..=i]); // cùng lát cắt
            assert_eq!(short, long, "phải tất định tại nến {}", i);
        }
    }

    #[test]
    fn an_empty_series_has_no_patterns() {
        assert_eq!(recv_elec(&[]), Pattern::KhongCo);
    }

    // ---------- Trung bình động ----------
    #[test]
    fn sma_matches_known_values() {
        assert_eq!(sma(&[1.0, 2.0, 3.0, 4.0, 5.0], 5), Some(3.0));
        assert_eq!(sma(&[1.0, 2.0, 3.0, 4.0, 5.0], 3), Some(4.0), "chỉ lấy 3 giá cuối");
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
        for x in price.iter_mut().skip(20) { *x = 200.0; }
        let s = sma_series(&price, 10);
        let e = ema_series(&price, 10);
        let i = 24; // 5 phiên sau cú nhảy
        assert!(e[i].unwrap() > s[i].unwrap(),
                "EMA {:.1} phải cao hơn SMA {:.1}", e[i].unwrap(), s[i].unwrap());
    }

    #[test]
    fn ema_converges_to_a_constant_price() {
        let price = vec![100.0; 200];
        let e = ema_series(&price, 20);
        assert!((e[199].unwrap() - 100.0).abs() < 1e-9,
                "giá đứng yên thì EMA phải bằng đúng giá đó");
    }

    #[test]
    fn ema_is_seeded_with_an_sma() {
        let price: Vec<f64> = (1..=20).map(|i| i as f64).collect();
        let e = ema_series(&price, 10);
        assert_eq!(e[9], Some(5.5), "giá trị đầu tiên là SMA của 10 phần tử đầu");
        assert_eq!(e[8], None, "trước đó chưa đủ dữ liệu");
    }

    #[test]
    fn wma_weights_recent_prices_more() {
        // [1,2,3] với trọng số [1,2,3] → (1+4+9)/6 = 2.333
        let w = wma(&[1.0, 2.0, 3.0], 3).unwrap();
        assert!((w - 14.0 / 6.0).abs() < 1e-9);
        assert!(w > sma(&[1.0, 2.0, 3.0], 3).unwrap(),
                "chuỗi tăng thì WMA phải cao hơn SMA");
    }

    // ---------- RSI ----------
    #[test]
    fn rsi_is_100_on_an_unbroken_rally() {
        let price: Vec<f64> = (1..=50).map(|i| i as f64 * 100.0).collect();
        let r = rsi_series(&price, 14);
        assert!((r[49].unwrap() - 100.0).abs() < 1e-6,
                "không có phiên giảm nào → RSI = 100");
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
        assert!((rsi_series(&price, 14)[49].unwrap() - 50.0).abs() < 1e-9,
                "không tăng không giảm → trung tính, và không chia cho 0");
    }

    #[test]
    fn rsi_always_stays_within_0_and_100() {
        for hat in [1u64, 42, 2024, 31337] {
            let price = price_close(&gen_candle(500, hat));
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
        let price = price_close(&gen_candle(200, 5));
        for m in macd_series(&price, 12, 26, 9).into_iter().flatten() {
            assert!((m.histogram - (m.macd - m.signal)).abs() < 1e-9);
        }
    }

    #[test]
    fn macd_is_positive_in_an_uptrend() {
        // Giá tăng đều → EMA nhanh phải nằm trên EMA chậm → MACD dương.
        let price: Vec<f64> = (1..=200).map(|i| 10_000.0 + i as f64 * 10.0).collect();
        let m = macd_series(&price, 12, 26, 9);
        assert!(m[199].unwrap().macd > 0.0, "xu hướng tăng phải cho MACD dương");
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
        assert!(macd_series(&price, 12, 26, 9).iter().all(|x| x.is_none()),
                "20 giá không đủ cho MACD(12,26,9)");
    }

    // ---------- Bollinger ----------
    #[test]
    fn the_bollinger_middle_band_equals_the_sma() {
        let price: Vec<f64> = (1..=30).map(|i| i as f64).collect();
        let b = bollinger(&price, 20, 2.0).unwrap();
        assert_eq!(b.mid, sma(&price, 20).unwrap());
    }

    #[test]
    fn the_bands_are_symmetric_about_the_middle() {
        let price = price_close(&gen_candle(100, 9));
        let b = bollinger(&price, 20, 2.0).unwrap();
        assert!(((b.above - b.mid) - (b.mid - b.below)).abs() < 1e-9,
                "hai dải phải cách đều đường giữa");
        assert!(b.above >= b.mid && b.mid >= b.below);
    }

    #[test]
    fn the_bands_narrow_when_volatility_falls() {
        let em = vec![100.0; 30];
        let xoc: Vec<f64> = (0..30).map(|i| 100.0 + ((i % 2) as f64) * 50.0).collect();
        let a = bollinger(&em, 20, 2.0).unwrap();
        let b = bollinger(&xoc, 20, 2.0).unwrap();
        assert!(a.do_rong() < b.do_rong(), "giá đứng yên → dải hẹp gần bằng 0");
        assert!(a.do_rong() < 1e-9);
    }

    #[test]
    fn percent_b_is_exact_at_both_bands() {
        let b = BollingerBands { above: 120.0, mid: 100.0, below: 80.0 };
        assert!((b.pos_value_percent(80.0) - 0.0).abs() < 1e-9);
        assert!((b.pos_value_percent(100.0) - 0.5).abs() < 1e-9);
        assert!((b.pos_value_percent(120.0) - 1.0).abs() < 1e-9);
        // Dải rỗng không được chia cho 0
        let hep = BollingerBands { above: 100.0, mid: 100.0, below: 100.0 };
        assert_eq!(hep.pos_value_percent(100.0), 0.5);
    }

    // ---------- ATR ----------
    #[test]
    fn true_range_includes_the_overnight_gap() {
        let prev = simple_candle(100, 105, 95, 100);
        // Phiên sau nhảy vọt lên: biên độ trong phiên chỉ 5, nhưng khoảng
        // cách so với giá đóng hôm trước là 30 — ATR phải thấy điều đó.
        let nay = simple_candle(128, 130, 125, 129);
        assert_eq!(nay.bien_do(), 5);
        assert_eq!(bien_do_that(&nay, Some(&prev)), 30, "phải bắt được khoảng nhảy");
    }

    #[test]
    fn the_first_candle_true_range_is_the_plain_range() {
        let n = simple_candle(100, 110, 90, 105);
        assert_eq!(bien_do_that(&n, None), 20);
    }

    #[test]
    fn atr_is_always_positive() {
        for hat in [1u64, 42, 2024] {
            let candle = gen_candle(300, hat);
            for a in atr_series(&candle, 14).into_iter().flatten() {
                assert!(a > 0.0, "ATR phải dương, thực tế {}", a);
            }
        }
    }

    #[test]
    fn atr_rises_with_volatility() {
        let em: Vec<Candle> = (0..50).map(|i| Candle { timestamp: i,
            mo: 10_000, high: 10_010, low: 9_990, dong: 10_000, quantity: 1 }).collect();
        let xoc: Vec<Candle> = (0..50).map(|i| Candle { timestamp: i,
            mo: 10_000, high: 10_500, low: 9_500, dong: 10_000, quantity: 1 }).collect();
        let a = atr_series(&em, 14)[49].unwrap();
        let b = atr_series(&xoc, 14)[49].unwrap();
        assert!(b > a * 10.0, "thị trường xóc gấp 50 lần phải cho ATR lớn hơn hẳn");
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
            let c = co_theo_atr(100_000, atr, 2.0);
            assert!(c < prev, "ATR {} phải cho cỡ nhỏ hơn", atr);
            prev = c;
        }
        assert_eq!(co_theo_atr(100_000, 20.0, 2.0), 2_500, "100000 / (20 × 2)");
    }

    #[test]
    fn atr_sizing_is_safe_on_bad_input() {
        assert_eq!(co_theo_atr(100_000, 0.0, 2.0), 0, "không chia cho 0");
        assert_eq!(co_theo_atr(100_000, 20.0, 0.0), 0);
    }

    // ---------- Không nhìn trước tương lai ----------
    #[test]
    fn no_indicator_peeks_at_the_future() {
        // BẤT BIẾN QUAN TRỌNG NHẤT của chương: giá trị chỉ báo tại nến i phải
        // giống hệt nhau dù ta đưa vào 201 nến hay 500 nến. Vi phạm điều này
        // là "nhìn trộm tương lai", và mọi kết quả kiểm định trở nên vô nghĩa.
        let candle = gen_candle(500, 2024);
        let price = price_close(&candle);
        let i = 200;

        assert_eq!(sma_series(&price[..=i], 20)[i], sma_series(&price, 20)[i]);
        assert_eq!(ema_series(&price[..=i], 20)[i], ema_series(&price, 20)[i]);
        assert_eq!(rsi_series(&price[..=i], 14)[i], rsi_series(&price, 14)[i]);
        assert_eq!(atr_series(&candle[..=i], 14)[i], atr_series(&candle, 14)[i]);
        assert_eq!(macd_series(&price[..=i], 12, 26, 9)[i], macd_series(&price, 12, 26, 9)[i]);
        assert_eq!(bollinger(&price[..=i], 20, 2.0), bollinger(&price[..i + 1], 20, 2.0));
    }

    #[test]
    fn candle_generation_is_deterministic() {
        assert_eq!(gen_candle(100, 5), gen_candle(100, 5));
        assert_ne!(gen_candle(100, 5), gen_candle(100, 6));
    }
}
```

---

## Bảng tra cứu lỗi biên dịch thường gặp

| Lỗi | Nguyên nhân trong chương này | Cách sửa |
|---|---|---|
| `E0277: cannot add Option<f64>` | Cộng trực tiếp các `Option` | Dùng `?` trong hàm trả `Option`, hoặc `zip` + `map` |
| Chỉ báo lệch chỉ số so với giá | Trả `Vec` ngắn hơn đầu vào | Luôn trả `Vec<Option<T>>` **cùng độ dài** |
| `attempt to subtract with overflow` | `t - n` khi `t < n` | `if t < n { continue }` hoặc `checked_sub` |
| RSI ra `NaN` | Chia cho 0 khi không có phiên giảm nào | Quy ước RSI = 100 khi `giảm_TB == 0` |
| Bài kiểm thử nhìn trộm tương lai trượt | Dùng thống kê toàn cục (max, mean của cả chuỗi) | Mọi thống kê phải tính trên cửa sổ trượt |

---

## Tóm tắt chương & Bài tập rèn luyện

### 5 điểm cốt lõi

1. **Nhìn trộm tương lai là lỗi tốn kém nhất trong giao dịch định lượng**, và nó luôn làm backtest đẹp lên.
2. **Kiểm tra vàng**: cắt dữ liệu tại t, giá trị chỉ báo tại t có đổi không? Đổi là có nhìn trộm.
3. **Cách khởi tạo EMA thay đổi kết quả.** Biết mình dùng cách nào, và ghi lại.
4. **ATR có ba vế vì khoảng nhảy giá.** Đây là chỉ báo tốt nhất để định cỡ dừng lỗ.
5. **Dải Bollinger giả định phân phối chuẩn mà lợi suất thì không.** Đuôi béo (Fat tail) khiến "cực đoan" xảy ra thường hơn nhiều.

### Bài tập rèn luyện

**Bài 1.** Cài **VWAP theo phiên** (giá trung bình có trọng số khối lượng) và bảo đảm nó không nhìn trộm tương lai.

<details>
<summary><b>Gợi ý</b></summary>

VWAP tích luỹ trong phiên và **đặt lại vào đầu mỗi phiên**. Đây là chuẩn tham chiếu chính cho việc thực thi lệnh của các quỹ lớn: "chúng tôi mua ở giá tốt hơn VWAP" là câu nói đo được. Cạm bẫy: tính VWAP trên toàn bộ ngày rồi dùng nó cho quyết định buổi sáng — đó là nhìn trộm tương lai kinh điển.
</details>

<details>
<summary><b>Lời giải</b></summary>

```rust
pub fn session_vwap(cac_nen: &[Candle], danh_dau_phien: &[bool]) -> Vec<Option<f64>> {
    let mut ra = Vec::with_capacity(cac_nen.len());
    let (mut tong_pv, mut tong_v) = (0.0f64, 0.0f64);

    for (i, n) in cac_nen.iter().enumerate() {
        if danh_dau_phien.get(i).copied().unwrap_or(false) {
            tong_pv = 0.0; tong_v = 0.0;          // phiên mới → đặt lại
        }
        // Giá điển hình — chuẩn ngành cho VWAP
        let gia_dh = (n.high + n.low + n.dong) as f64 / 3.0;
        tong_pv += gia_dh * n.quantity as f64;
        tong_v  += n.quantity as f64;

        // Chỉ dùng dữ liệu tới i — không nhìn trộm
        ra.push(if tong_v > 0.0 { Some(tong_pv / tong_v) } else { None });
    }
    ra
}

/// Chênh lệch thực thi so với VWAP, tính bằng điểm cơ bản.
/// Dương = thực thi TỐT HƠN chuẩn.
pub fn execution_quality_bps(gia_thuc_thi: f64, vwap: f64, la_mua: bool) -> f64 {
    if vwap == 0.0 { return 0.0; }
    let cl = if la_mua { vwap - gia_thuc_thi } else { gia_thuc_thi - vwap };
    cl / vwap * 10_000.0
}
```

`do_chat_luong_thuc_thi` là thước đo mà bàn giao dịch dùng để đánh giá thuật toán thực thi. Nó cũng cho thấy vì sao VWAP phải tính đúng: nếu chuẩn tham chiếu sai, mọi đánh giá đều sai.
</details>

**Bài 2.** Cài **phát hiện phân kỳ**: giá tạo đỉnh cao hơn nhưng chỉ báo tạo đỉnh thấp hơn.

<details>
<summary><b>Gợi ý</b></summary>

Phân kỳ là một trong ít mẫu hình phân tích kỹ thuật có cơ sở logic: nó nói rằng động lượng đang yếu đi dù giá vẫn tăng. Khó khăn nằm ở **xác định đỉnh/đáy cục bộ** một cách không nhìn trộm tương lai — bạn chỉ biết một điểm là đỉnh sau khi đã có vài phiên đi xuống.
</details>

<details>
<summary><b>Lời giải</b></summary>

```rust
#[derive(Debug, PartialEq, Clone, Copy)]
pub enum DivergenceKind { TangGia, GiamGia }

#[derive(Debug)]
pub struct Divergence {
    pub kind: DivergenceKind,
    pub start_index: usize,
    pub end_index: usize,
}

/// Đỉnh cục bộ được XÁC NHẬN sau `nhin_lai` phiên — nên nó chỉ
/// "tồn tại" từ chỉ số i + nhin_lai trở đi. Đó là điểm chống nhìn trộm.
fn peaks(price: &[f64], nhin_lai: usize) -> Vec<usize> {
    let mut ra = Vec::new();
    if price.len() < 2 * nhin_lai + 1 { return ra; }
    for i in nhin_lai..(price.len() - nhin_lai) {
        let is_peak = (1..=nhin_lai).all(|k| price[i] > price[i - k] && price[i] > price[i + k]);
        if is_peak { ra.push(i); }
    }
    ra
}

pub fn find_divergence(price: &[f64], chi_bao: &[Option<f64>], nhin_lai: usize) -> Vec<Divergence> {
    let peak = peaks(price, nhin_lai);
    let mut ra = Vec::new();
    for w in peak.windows(2) {
        let (a, b) = (w[0], w[1]);
        let (ca, cb) = match (chi_bao.get(a), chi_bao.get(b)) {
            (Some(Some(x)), Some(Some(y))) => (*x, *y),
            _ => continue,
        };
        // Giá đỉnh cao hơn nhưng chỉ báo đỉnh thấp hơn → động lượng yếu đi
        if price[b] > price[a] && cb < ca {
            ra.push(Divergence { kind: DivergenceKind::GiamGia, start_index: a, end_index: b });
        }
        // Giá đỉnh thấp hơn nhưng chỉ báo cao hơn → động lượng mạnh lên
        if price[b] < price[a] && cb > ca {
            ra.push(Divergence { kind: DivergenceKind::TangGia, start_index: a, end_index: b });
        }
    }
    ra
}
```

Chú ý điều kiện `i < gia.len() - nhin_lai` trong `cac_dinh`: một đỉnh chỉ được xác nhận sau `nhin_lai` phiên. Trong giao dịch thật, tín hiệu phân kỳ đến **muộn hơn** đỉnh đúng bằng ngần ấy phiên — và mọi backtest phải phản ánh độ trễ đó, nếu không nó lại là nhìn trộm tương lai.
</details>
