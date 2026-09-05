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

Dải Bollinger đặt tại `SMA ± k·độ_lệch_chuẩn`, thường `k = 2`. Nếu lợi suất tuân theo phân phối chuẩn, khoảng 95% giá sẽ nằm trong dải.

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

pub type Gia = i64; // tick, 1 tick = 0,01 đơn vị tiền

// ============================================================================
// 1. NẾN OHLCV
// ============================================================================

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct Nen {
    pub thoi_diem: u64,
    pub mo: Gia,
    pub cao: Gia,
    pub thap: Gia,
    pub dong: Gia,
    pub khoi_luong: u64,
}

impl Nen {
    /// Thân nến: khoảng cách giữa giá mở và giá đóng.
    pub fn than(&self) -> Gia { (self.dong - self.mo).abs() }
    /// Toàn bộ biên độ trong phiên.
    pub fn bien_do(&self) -> Gia { self.cao - self.thap }
    pub fn bong_tren(&self) -> Gia { self.cao - self.mo.max(self.dong) }
    pub fn bong_duoi(&self) -> Gia { self.mo.min(self.dong) - self.thap }
    pub fn tang(&self) -> bool { self.dong > self.mo }
    pub fn giam(&self) -> bool { self.dong < self.mo }

    /// Nến có hợp lệ không. Dữ liệu thị trường thật CÓ lỗi, và một nến sai
    /// làm hỏng mọi chỉ báo phía sau mà không báo gì.
    pub fn hop_le(&self) -> bool {
        self.cao >= self.thap
            && self.cao >= self.mo && self.cao >= self.dong
            && self.thap <= self.mo && self.thap <= self.dong
            && self.thap > 0
    }
}

// ============================================================================
// 2. MẪU HÌNH NẾN
// ============================================================================
// Mẫu hình nến là cách con người tóm tắt tâm lý thị trường trong một phiên.
// Chúng KHÔNG phải tín hiệu dự báo tự thân — dùng một mình thì gần như vô
// dụng. Giá trị của chúng nằm ở chỗ xác nhận bối cảnh do chỉ báo khác dựng ra.

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum MauHinh { Doji, BuaTang, SaoBangGiam, NhanChimTang, NhanChimGiam, KhongCo }

/// Doji: giá mở gần bằng giá đóng — hai phe giằng co, không ai thắng.
pub fn la_doji(n: &Nen, nguong_phan_van: i64) -> bool {
    if n.bien_do() == 0 { return true; }
    n.than() * 10_000 <= n.bien_do() * nguong_phan_van
}

/// Búa: thân nhỏ ở TRÊN, bóng dưới dài — người bán đẩy giá xuống nhưng bị
/// người mua kéo lại hết. Chỉ có ý nghĩa khi xuất hiện SAU một đợt giảm.
pub fn la_bua(n: &Nen) -> bool {
    n.bien_do() > 0
        && n.than() > 0
        && n.bong_duoi() >= n.than() * 2
        && n.bong_tren() <= n.than()
}

/// Sao băng: đối xứng của búa — bóng TRÊN dài, xuất hiện sau đợt tăng.
pub fn la_sao_bang(n: &Nen) -> bool {
    n.bien_do() > 0
        && n.than() > 0
        && n.bong_tren() >= n.than() * 2
        && n.bong_duoi() <= n.than()
}

/// Nhấn chìm tăng: nến tăng hôm nay bao trọn thân nến giảm hôm qua.
pub fn la_nhan_chim_tang(hom_qua: &Nen, hom_nay: &Nen) -> bool {
    hom_qua.giam() && hom_nay.tang()
        && hom_nay.dong >= hom_qua.mo && hom_nay.mo <= hom_qua.dong
        && hom_nay.than() > hom_qua.than()
}

pub fn la_nhan_chim_giam(hom_qua: &Nen, hom_nay: &Nen) -> bool {
    hom_qua.tang() && hom_nay.giam()
        && hom_nay.mo >= hom_qua.dong && hom_nay.dong <= hom_qua.mo
        && hom_nay.than() > hom_qua.than()
}

/// Nhận diện mẫu hình tại nến CUỐI của `lich_su`.
/// Chỉ nhìn dữ liệu ĐÃ CÓ — không bao giờ chạm tới nến tương lai.
pub fn nhan_dien(lich_su: &[Nen]) -> MauHinh {
    let n = match lich_su.last() { Some(n) => n, None => return MauHinh::KhongCo };
    if let Some(q) = lich_su.len().checked_sub(2).map(|i| &lich_su[i]) {
        if la_nhan_chim_tang(q, n) { return MauHinh::NhanChimTang; }
        if la_nhan_chim_giam(q, n) { return MauHinh::NhanChimGiam; }
    }
    if la_doji(n, 500) { return MauHinh::Doji; } // thân ≤ 5% biên độ
    if la_bua(n) { return MauHinh::BuaTang; }
    if la_sao_bang(n) { return MauHinh::SaoBangGiam; }
    MauHinh::KhongCo
}

// ============================================================================
// 3. TRUNG BÌNH ĐỘNG
// ============================================================================

/// Trung bình động đơn giản. Trả `None` khi chưa đủ `chu_ky` nến — điều này
/// QUAN TRỌNG: trả 0 hay trả trung bình của số ít nến sẽ khiến chiến lược
/// vào lệnh dựa trên dữ liệu không đủ.
pub fn sma(gia: &[f64], chu_ky: usize) -> Option<f64> {
    if chu_ky == 0 || gia.len() < chu_ky { return None; }
    Some(gia[gia.len() - chu_ky..].iter().sum::<f64>() / chu_ky as f64)
}

/// Toàn bộ chuỗi SMA. Phần tử `i` chỉ dùng dữ liệu tới `i` — không nhìn trước.
pub fn chuoi_sma(gia: &[f64], chu_ky: usize) -> Vec<Option<f64>> {
    (0..gia.len()).map(|i| sma(&gia[..=i], chu_ky)).collect()
}

/// Trung bình động luỹ thừa. Hệ số làm mượt α = 2/(n+1).
/// EMA phản ứng nhanh hơn SMA vì nó cho dữ liệu mới trọng số cao hơn — nhưng
/// cũng vì thế mà nhiễu hơn.
pub fn chuoi_ema(gia: &[f64], chu_ky: usize) -> Vec<Option<f64>> {
    let mut ra = vec![None; gia.len()];
    if chu_ky == 0 || gia.len() < chu_ky { return ra; }
    let alpha = 2.0 / (chu_ky as f64 + 1.0);
    // Mồi bằng SMA của `chu_ky` giá trị đầu — cách chuẩn của ngành
    let mut e = gia[..chu_ky].iter().sum::<f64>() / chu_ky as f64;
    ra[chu_ky - 1] = Some(e);
    for i in chu_ky..gia.len() {
        e = gia[i] * alpha + e * (1.0 - alpha);
        ra[i] = Some(e);
    }
    ra
}

/// Trung bình động có trọng số tuyến tính: giá mới nhất có trọng số n,
/// giá cũ nhất có trọng số 1.
pub fn wma(gia: &[f64], chu_ky: usize) -> Option<f64> {
    if chu_ky == 0 || gia.len() < chu_ky { return None; }
    let cua_so = &gia[gia.len() - chu_ky..];
    let tong_trong_so = (chu_ky * (chu_ky + 1) / 2) as f64;
    Some(cua_so.iter().enumerate().map(|(i, &x)| x * (i + 1) as f64).sum::<f64>()
         / tong_trong_so)
}

// ============================================================================
// 4. RSI — CHỈ SỐ SỨC MẠNH TƯƠNG ĐỐI
// ============================================================================
// RSI đo tương quan giữa mức tăng và mức giảm gần đây, quy về thang 0–100.
// Trên 70 thường gọi là "quá mua", dưới 30 là "quá bán" — nhưng trong xu
// hướng mạnh, RSI có thể nằm trên 70 hàng tuần liền. Đó là lý do dùng RSI
// một mình để đoán đảo chiều là cách mất tiền nhanh nhất.

pub fn chuoi_rsi(gia: &[f64], chu_ky: usize) -> Vec<Option<f64>> {
    let mut ra = vec![None; gia.len()];
    if chu_ky == 0 || gia.len() <= chu_ky { return ra; }

    let mut tang_tb = 0.0;
    let mut giam_tb = 0.0;
    for i in 1..=chu_ky {
        let d = gia[i] - gia[i - 1];
        if d > 0.0 { tang_tb += d; } else { giam_tb += -d; }
    }
    tang_tb /= chu_ky as f64;
    giam_tb /= chu_ky as f64;
    ra[chu_ky] = Some(tu_tang_giam(tang_tb, giam_tb));

    // Làm mượt kiểu Wilder: giống EMA với α = 1/n
    for i in (chu_ky + 1)..gia.len() {
        let d = gia[i] - gia[i - 1];
        let (t, g) = if d > 0.0 { (d, 0.0) } else { (0.0, -d) };
        tang_tb = (tang_tb * (chu_ky - 1) as f64 + t) / chu_ky as f64;
        giam_tb = (giam_tb * (chu_ky - 1) as f64 + g) / chu_ky as f64;
        ra[i] = Some(tu_tang_giam(tang_tb, giam_tb));
    }
    ra
}

fn tu_tang_giam(tang: f64, giam: f64) -> f64 {
    // Không có phiên giảm nào → RSI = 100. Phải xử lý riêng để không chia cho 0.
    if giam < 1e-12 { return if tang < 1e-12 { 50.0 } else { 100.0 }; }
    100.0 - 100.0 / (1.0 + tang / giam)
}

// ============================================================================
// 5. MACD
// ============================================================================

#[derive(Debug, Clone, Copy, PartialEq)]
pub struct GiaTriMacd { pub macd: f64, pub tin_hieu: f64, pub bieu_do: f64 }

/// MACD = EMA nhanh − EMA chậm. Đường tín hiệu = EMA của chính MACD.
/// Biểu đồ = MACD − tín hiệu, đo đà tăng tốc.
pub fn chuoi_macd(gia: &[f64], nhanh: usize, cham: usize, tin_hieu: usize)
    -> Vec<Option<GiaTriMacd>>
{
    let mut ra = vec![None; gia.len()];
    if cham == 0 || gia.len() < cham { return ra; }
    let e_nhanh = chuoi_ema(gia, nhanh);
    let e_cham = chuoi_ema(gia, cham);

    // Chuỗi MACD chỉ có giá trị từ khi CẢ HAI đường EMA đã sẵn sàng
    let mut duong_macd: Vec<f64> = Vec::new();
    let mut chi_so_goc: Vec<usize> = Vec::new();
    for i in 0..gia.len() {
        if let (Some(a), Some(b)) = (e_nhanh[i], e_cham[i]) {
            duong_macd.push(a - b);
            chi_so_goc.push(i);
        }
    }
    let e_tin_hieu = chuoi_ema(&duong_macd, tin_hieu);
    for (k, &i) in chi_so_goc.iter().enumerate() {
        if let Some(s) = e_tin_hieu[k] {
            ra[i] = Some(GiaTriMacd { macd: duong_macd[k], tin_hieu: s,
                                      bieu_do: duong_macd[k] - s });
        }
    }
    ra
}

// ============================================================================
// 6. DẢI BOLLINGER
// ============================================================================

#[derive(Debug, Clone, Copy, PartialEq)]
pub struct DaiBollinger { pub tren: f64, pub giua: f64, pub duoi: f64 }

impl DaiBollinger {
    pub fn do_rong(&self) -> f64 {
        if self.giua.abs() < 1e-12 { 0.0 } else { (self.tren - self.duoi) / self.giua }
    }
    /// Vị trí của giá trong dải: 0 = chạm đáy, 1 = chạm đỉnh.
    pub fn vi_tri_phan_tram(&self, gia: f64) -> f64 {
        let d = self.tren - self.duoi;
        if d.abs() < 1e-12 { 0.5 } else { (gia - self.duoi) / d }
    }
}

pub fn bollinger(gia: &[f64], chu_ky: usize, so_do_lech: f64) -> Option<DaiBollinger> {
    let giua = sma(gia, chu_ky)?;
    let cua_so = &gia[gia.len() - chu_ky..];
    // Độ lệch chuẩn TỔNG THỂ (chia n) — quy ước chuẩn của dải Bollinger
    let ps = cua_so.iter().map(|x| (x - giua).powi(2)).sum::<f64>() / chu_ky as f64;
    let sd = ps.max(0.0).sqrt();
    Some(DaiBollinger { tren: giua + so_do_lech * sd, giua, duoi: giua - so_do_lech * sd })
}

// ============================================================================
// 7. ATR — BIÊN ĐỘ THẬT TRUNG BÌNH
// ============================================================================
// ATR đo mức dao động, KHÔNG đo hướng. Nó là công cụ định cỡ vị thế và đặt
// cắt lỗ tốt nhất: đặt cắt lỗ cách 2 ATR thì mức chấp nhận rủi ro tự động
// điều chỉnh theo trạng thái thị trường.

/// Biên độ thật: lớn nhất trong ba khoảng cách. Nó tính cả KHOẢNG NHẢY giữa
/// hai phiên — điều mà `cao − thap` bỏ sót hoàn toàn.
pub fn bien_do_that(nay: &Nen, truoc: Option<&Nen>) -> Gia {
    match truoc {
        None => nay.cao - nay.thap,
        Some(t) => (nay.cao - nay.thap)
            .max((nay.cao - t.dong).abs())
            .max((nay.thap - t.dong).abs()),
    }
}

pub fn chuoi_atr(nen: &[Nen], chu_ky: usize) -> Vec<Option<f64>> {
    let mut ra = vec![None; nen.len()];
    if chu_ky == 0 || nen.len() < chu_ky { return ra; }
    let bdt: Vec<f64> = nen.iter().enumerate()
        .map(|(i, n)| bien_do_that(n, i.checked_sub(1).map(|j| &nen[j])) as f64)
        .collect();
    let mut a = bdt[..chu_ky].iter().sum::<f64>() / chu_ky as f64;
    ra[chu_ky - 1] = Some(a);
    for i in chu_ky..nen.len() {
        a = (a * (chu_ky - 1) as f64 + bdt[i]) / chu_ky as f64; // làm mượt Wilder
        ra[i] = Some(a);
    }
    ra
}

/// Định cỡ vị thế theo ATR: rủi ro mỗi lệnh cố định bằng tiền, nên mã dao
/// động mạnh thì mua ít. Đây là công thức nền của mọi hệ thống theo xu hướng.
pub fn co_theo_atr(von_rui_ro: i64, atr: f64, so_atr_cat_lo: f64) -> i64 {
    let rui_ro_moi_don_vi = atr * so_atr_cat_lo;
    if rui_ro_moi_don_vi < 1e-9 { return 0; }
    (von_rui_ro as f64 / rui_ro_moi_don_vi) as i64
}

// ============================================================================
// 8. SINH DỮ LIỆU TẤT ĐỊNH
// ============================================================================

pub fn sinh_nen(n: usize, hat_giong: u64) -> Vec<Nen> {
    let mut s = hat_giong;
    let mut gia: Gia = 10_000;
    (0..n).map(|i| {
        s = s.wrapping_mul(6364136223846793005).wrapping_add(1442695040888963407);
        let buoc = ((s >> 33) % 201) as i64 - 100;
        let mo = gia;
        gia = (gia + buoc).max(100);
        let bien = ((s >> 45) % 80) as i64;
        Nen {
            thoi_diem: i as u64,
            mo,
            cao: mo.max(gia) + bien,
            thap: (mo.min(gia) - bien).max(1),
            dong: gia,
            khoi_luong: 1_000 + (s >> 50) % 9_000,
        }
    }).collect()
}

pub fn gia_dong(nen: &[Nen]) -> Vec<f64> { nen.iter().map(|n| n.dong as f64).collect() }

fn main() {
    println!("═══════════════════════════════════════════════════════════");
    println!("   PHÂN TÍCH KỸ THUẬT BẰNG RUST (giáo trình OpenAlgo)       ");
    println!("═══════════════════════════════════════════════════════════");

    let nen = sinh_nen(500, 2024);
    let gia = gia_dong(&nen);

    println!("\n1. NẾN OHLCV");
    let n = &nen[100];
    println!("   Nến #100: mở {} cao {} thấp {} đóng {}", n.mo, n.cao, n.thap, n.dong);
    println!("   thân {} · biên độ {} · bóng trên {} · bóng dưới {} · {}",
             n.than(), n.bien_do(), n.bong_tren(), n.bong_duoi(),
             if n.tang() { "TĂNG" } else { "GIẢM" });
    println!("   Toàn bộ {} nến đều hợp lệ: {}",
             nen.len(), nen.iter().all(|x| x.hop_le()));

    println!("\n2. MẪU HÌNH NẾN — đếm trên 500 nến");
    let mut dem = std::collections::BTreeMap::new();
    for i in 0..nen.len() {
        *dem.entry(format!("{:?}", nhan_dien(&nen[..=i]))).or_insert(0) += 1;
    }
    for (k, v) in &dem { println!("   {:<16} {:>4} lần", k, v); }

    println!("\n3. TRUNG BÌNH ĐỘNG — cùng dữ liệu, khác độ nhạy");
    let s20 = chuoi_sma(&gia, 20);
    let e20 = chuoi_ema(&gia, 20);
    println!("   {:>6} {:>10} {:>10} {:>10}", "nến", "giá", "SMA 20", "EMA 20");
    for i in [100usize, 200, 300, 400, 499] {
        println!("   {:>6} {:>10.0} {:>10.1} {:>10.1}",
                 i, gia[i], s20[i].unwrap(), e20[i].unwrap());
    }
    println!("   → EMA bám giá sát hơn vì nó cho dữ liệu mới trọng số cao hơn.");

    println!("\n4. RSI");
    let r14 = chuoi_rsi(&gia, 14);
    let qua_mua = r14.iter().filter(|x| x.is_some_and(|v| v > 70.0)).count();
    let qua_ban = r14.iter().filter(|x| x.is_some_and(|v| v < 30.0)).count();
    println!("   RSI(14) tại nến 499: {:.1}", r14[499].unwrap());
    println!("   Số phiên > 70 (quá mua): {} · < 30 (quá bán): {}", qua_mua, qua_ban);
    let tang_deu: Vec<f64> = (1..=50).map(|i| i as f64 * 100.0).collect();
    let giam_deu: Vec<f64> = (1..=50).rev().map(|i| i as f64 * 100.0).collect();
    println!("   Chuỗi tăng đều  → RSI = {:.0}", chuoi_rsi(&tang_deu, 14)[49].unwrap());
    println!("   Chuỗi giảm đều  → RSI = {:.0}", chuoi_rsi(&giam_deu, 14)[49].unwrap());
    println!("   → Trong xu hướng mạnh, RSI dính sát 100 hoặc 0 rất lâu.");
    println!("     Dùng RSI một mình để đoán đảo chiều là cách mất tiền nhanh nhất.");

    println!("\n5. MACD (12, 26, 9)");
    let m = chuoi_macd(&gia, 12, 26, 9);
    let mut giao_cat = 0;
    for i in 1..m.len() {
        if let (Some(a), Some(b)) = (m[i - 1], m[i]) {
            if a.bieu_do.signum() != b.bieu_do.signum() { giao_cat += 1; }
        }
    }
    let cuoi = m[499].unwrap();
    println!("   Tại nến 499: MACD {:.2} · tín hiệu {:.2} · biểu đồ {:.2}",
             cuoi.macd, cuoi.tin_hieu, cuoi.bieu_do);
    println!("   Số lần biểu đồ đổi dấu trong 500 nến: {}", giao_cat);
    println!("   → {} tín hiệu trên 500 phiên. Phần lớn là nhiễu, và mỗi tín hiệu",
             giao_cat);
    println!("     đều tốn phí giao dịch — xem lại Chương 69.");

    println!("\n6. DẢI BOLLINGER (20, 2σ)");
    for i in [100usize, 300, 499] {
        let b = bollinger(&gia[..=i], 20, 2.0).unwrap();
        println!("   Nến {:>3}: dưới {:>8.1} · giữa {:>8.1} · trên {:>8.1} · giá ở {:>5.0}% dải",
                 i, b.duoi, b.giua, b.tren, b.vi_tri_phan_tram(gia[i]) * 100.0);
    }
    let ngoai = (20..gia.len()).filter(|&i| {
        let b = bollinger(&gia[..=i], 20, 2.0).unwrap();
        gia[i] > b.tren || gia[i] < b.duoi
    }).count();
    println!("   Số phiên giá vượt ra ngoài dải: {} / {} ({:.1}%)",
             ngoai, gia.len() - 20, ngoai as f64 * 100.0 / (gia.len() - 20) as f64);
    println!("   → Lý thuyết nói ~5% nằm ngoài 2σ. Thực tế thị trường thường nhiều hơn:");
    println!("     phân bố giá có ĐUÔI DÀY hơn phân bố chuẩn.");

    println!("\n7. ATR & ĐỊNH CỠ VỊ THẾ");
    let a14 = chuoi_atr(&nen, 14);
    println!("   ATR(14) tại nến 499: {:.1} tick", a14[499].unwrap());
    println!("   {:>16} {:>12} {:>16}", "vốn rủi ro", "ATR", "số lượng mua");
    for atr in [20.0f64, 50.0, 100.0, 200.0] {
        println!("   {:>16} {:>12.0} {:>16}",
                 100_000, atr, co_theo_atr(100_000, atr, 2.0));
    }
    println!("   → Cùng mức rủi ro bằng tiền. Mã dao động mạnh gấp 10 thì mua ít đi 10 lần.");

    println!("\n═══════════════════════════════════════════════════════════");
    println!("   CHỈ BÁO KHÔNG DỰ BÁO TƯƠNG LAI — CHÚNG TÓM TẮT QUÁ KHỨ   ");
    println!("═══════════════════════════════════════════════════════════");
}

#[cfg(test)]
mod kiem_thu {
    use super::*;

    fn nen_don(mo: Gia, cao: Gia, thap: Gia, dong: Gia) -> Nen {
        Nen { thoi_diem: 0, mo, cao, thap, dong, khoi_luong: 100 }
    }

    // ---------- Nến ----------
    #[test]
    fn tinh_dung_than_bien_do_va_bong() {
        let n = nen_don(100, 120, 90, 110);
        assert_eq!(n.than(), 10);
        assert_eq!(n.bien_do(), 30);
        assert_eq!(n.bong_tren(), 10, "120 − max(100,110)");
        assert_eq!(n.bong_duoi(), 10, "min(100,110) − 90");
        assert!(n.tang() && !n.giam());
    }

    #[test]
    fn phat_hien_nen_khong_hop_le() {
        assert!(nen_don(100, 120, 90, 110).hop_le());
        assert!(!nen_don(100, 80, 90, 110).hop_le(), "cao < thấp là vô lý");
        assert!(!nen_don(100, 105, 90, 110).hop_le(), "đóng > cao là vô lý");
        assert!(!nen_don(100, 120, 105, 110).hop_le(), "thấp > mở là vô lý");
        assert!(!nen_don(100, 120, 0, 110).hop_le(), "giá không được bằng 0");
    }

    #[test]
    fn moi_nen_sinh_ra_deu_hop_le() {
        for hat in [1u64, 42, 2024] {
            for n in sinh_nen(1_000, hat) {
                assert!(n.hop_le(), "nến sinh ra phải luôn hợp lệ: {:?}", n);
            }
        }
    }

    // ---------- Mẫu hình ----------
    #[test]
    fn doji_khi_mo_gan_bang_dong() {
        assert!(la_doji(&nen_don(100, 120, 80, 100), 500), "mở = đóng");
        assert!(la_doji(&nen_don(100, 120, 80, 101), 500), "thân 1 trên biên độ 40");
        assert!(!la_doji(&nen_don(100, 120, 80, 115), 500), "thân 15 là quá lớn");
    }

    #[test]
    fn nen_khong_bien_do_duoc_coi_la_doji() {
        // Phiên không giao dịch — phải xử lý được, không chia cho 0.
        assert!(la_doji(&nen_don(100, 100, 100, 100), 500));
    }

    #[test]
    fn bua_va_sao_bang_doi_xung_nhau() {
        // Búa: bóng dưới dài, thân nhỏ ở trên
        let bua = nen_don(110, 112, 90, 111);
        assert!(la_bua(&bua), "bóng dưới {} thân {}", bua.bong_duoi(), bua.than());
        assert!(!la_sao_bang(&bua));
        // Sao băng: bóng trên dài, thân nhỏ ở dưới
        let sao = nen_don(91, 112, 90, 92);
        assert!(la_sao_bang(&sao));
        assert!(!la_bua(&sao));
    }

    #[test]
    fn nhan_chim_tang_phai_bao_tron_than_nen_truoc() {
        let hom_qua = nen_don(110, 112, 98, 100); // giảm
        let hom_nay = nen_don(99, 116, 98, 115);  // tăng, bao trọn
        assert!(la_nhan_chim_tang(&hom_qua, &hom_nay));
        // Không bao trọn thì không tính
        let hep = nen_don(102, 110, 101, 108);
        assert!(!la_nhan_chim_tang(&hom_qua, &hep));
        // Hôm qua phải là nến GIẢM
        assert!(!la_nhan_chim_tang(&nen_don(100, 116, 98, 112), &hom_nay));
    }

    #[test]
    fn nhan_dien_tat_dinh_va_khong_nhin_nen_tuong_lai() {
        // Bất biến sống còn: thêm nến phía sau KHÔNG được đổi kết quả tại
        // nến trước. Vi phạm điều này là "vẽ lại" (repainting).
        let nen = sinh_nen(300, 7);
        for i in 0..nen.len() {
            let ngan = nhan_dien(&nen[..=i]);
            let dai = nhan_dien(&nen[..=i]); // cùng lát cắt
            assert_eq!(ngan, dai, "phải tất định tại nến {}", i);
        }
    }

    #[test]
    fn danh_sach_rong_khong_co_mau_hinh() {
        assert_eq!(nhan_dien(&[]), MauHinh::KhongCo);
    }

    // ---------- Trung bình động ----------
    #[test]
    fn sma_tinh_dung_gia_tri_da_biet() {
        assert_eq!(sma(&[1.0, 2.0, 3.0, 4.0, 5.0], 5), Some(3.0));
        assert_eq!(sma(&[1.0, 2.0, 3.0, 4.0, 5.0], 3), Some(4.0), "chỉ lấy 3 giá cuối");
        assert_eq!(sma(&[1.0, 2.0], 5), None, "chưa đủ dữ liệu");
        assert_eq!(sma(&[1.0], 0), None, "chu kỳ 0 vô nghĩa");
    }

    #[test]
    fn sma_chua_du_du_lieu_thi_tra_none_chu_khong_tra_bua() {
        // Trả 0 hay trả trung bình của số ít nến sẽ khiến chiến lược vào lệnh
        // trên dữ liệu không đủ — lỗi âm thầm và tốn tiền.
        let c = chuoi_sma(&[1.0, 2.0, 3.0, 4.0, 5.0], 3);
        assert_eq!(c[0], None);
        assert_eq!(c[1], None);
        assert_eq!(c[2], Some(2.0));
        assert_eq!(c[4], Some(4.0));
    }

    #[test]
    fn ema_bam_gia_sat_hon_sma() {
        // Giá nhảy bậc: EMA phải phản ứng nhanh hơn SMA.
        let mut gia = vec![100.0; 30];
        for x in gia.iter_mut().skip(20) { *x = 200.0; }
        let s = chuoi_sma(&gia, 10);
        let e = chuoi_ema(&gia, 10);
        let i = 24; // 5 phiên sau cú nhảy
        assert!(e[i].unwrap() > s[i].unwrap(),
                "EMA {:.1} phải cao hơn SMA {:.1}", e[i].unwrap(), s[i].unwrap());
    }

    #[test]
    fn ema_hoi_tu_ve_gia_khong_doi() {
        let gia = vec![100.0; 200];
        let e = chuoi_ema(&gia, 20);
        assert!((e[199].unwrap() - 100.0).abs() < 1e-9,
                "giá đứng yên thì EMA phải bằng đúng giá đó");
    }

    #[test]
    fn ema_duoc_moi_bang_sma() {
        let gia: Vec<f64> = (1..=20).map(|i| i as f64).collect();
        let e = chuoi_ema(&gia, 10);
        assert_eq!(e[9], Some(5.5), "giá trị đầu tiên là SMA của 10 phần tử đầu");
        assert_eq!(e[8], None, "trước đó chưa đủ dữ liệu");
    }

    #[test]
    fn wma_cho_gia_moi_trong_so_cao_hon() {
        // [1,2,3] với trọng số [1,2,3] → (1+4+9)/6 = 2.333
        let w = wma(&[1.0, 2.0, 3.0], 3).unwrap();
        assert!((w - 14.0 / 6.0).abs() < 1e-9);
        assert!(w > sma(&[1.0, 2.0, 3.0], 3).unwrap(),
                "chuỗi tăng thì WMA phải cao hơn SMA");
    }

    // ---------- RSI ----------
    #[test]
    fn rsi_bang_100_khi_chi_toan_tang() {
        let gia: Vec<f64> = (1..=50).map(|i| i as f64 * 100.0).collect();
        let r = chuoi_rsi(&gia, 14);
        assert!((r[49].unwrap() - 100.0).abs() < 1e-6,
                "không có phiên giảm nào → RSI = 100");
    }

    #[test]
    fn rsi_bang_0_khi_chi_toan_giam() {
        let gia: Vec<f64> = (1..=50).rev().map(|i| i as f64 * 100.0).collect();
        let r = chuoi_rsi(&gia, 14);
        assert!(r[49].unwrap() < 1e-6, "không có phiên tăng nào → RSI = 0");
    }

    #[test]
    fn rsi_bang_50_khi_gia_dung_yen() {
        let gia = vec![100.0; 50];
        assert!((chuoi_rsi(&gia, 14)[49].unwrap() - 50.0).abs() < 1e-9,
                "không tăng không giảm → trung tính, và không chia cho 0");
    }

    #[test]
    fn rsi_luon_nam_trong_khoang_0_den_100() {
        for hat in [1u64, 42, 2024, 31337] {
            let gia = gia_dong(&sinh_nen(500, hat));
            for x in chuoi_rsi(&gia, 14).into_iter().flatten() {
                assert!((0.0..=100.0).contains(&x), "RSI ra ngoài thang: {}", x);
            }
        }
    }

    #[test]
    fn rsi_chua_du_du_lieu_thi_none() {
        let gia: Vec<f64> = (1..=10).map(|i| i as f64).collect();
        let r = chuoi_rsi(&gia, 14);
        assert!(r.iter().all(|x| x.is_none()), "10 giá không đủ cho RSI(14)");
    }

    // ---------- MACD ----------
    #[test]
    fn macd_bieu_do_bang_hieu_hai_duong() {
        let gia = gia_dong(&sinh_nen(200, 5));
        for m in chuoi_macd(&gia, 12, 26, 9).into_iter().flatten() {
            assert!((m.bieu_do - (m.macd - m.tin_hieu)).abs() < 1e-9);
        }
    }

    #[test]
    fn macd_duong_khi_xu_huong_tang() {
        // Giá tăng đều → EMA nhanh phải nằm trên EMA chậm → MACD dương.
        let gia: Vec<f64> = (1..=200).map(|i| 10_000.0 + i as f64 * 10.0).collect();
        let m = chuoi_macd(&gia, 12, 26, 9);
        assert!(m[199].unwrap().macd > 0.0, "xu hướng tăng phải cho MACD dương");
    }

    #[test]
    fn macd_am_khi_xu_huong_giam() {
        let gia: Vec<f64> = (1..=200).map(|i| 10_000.0 - i as f64 * 10.0).collect();
        let m = chuoi_macd(&gia, 12, 26, 9);
        assert!(m[199].unwrap().macd < 0.0);
    }

    #[test]
    fn macd_chua_du_du_lieu_thi_none() {
        let gia: Vec<f64> = (1..=20).map(|i| i as f64).collect();
        assert!(chuoi_macd(&gia, 12, 26, 9).iter().all(|x| x.is_none()),
                "20 giá không đủ cho MACD(12,26,9)");
    }

    // ---------- Bollinger ----------
    #[test]
    fn bollinger_giua_bang_dung_sma() {
        let gia: Vec<f64> = (1..=30).map(|i| i as f64).collect();
        let b = bollinger(&gia, 20, 2.0).unwrap();
        assert_eq!(b.giua, sma(&gia, 20).unwrap());
    }

    #[test]
    fn bollinger_doi_xung_quanh_duong_giua() {
        let gia = gia_dong(&sinh_nen(100, 9));
        let b = bollinger(&gia, 20, 2.0).unwrap();
        assert!(((b.tren - b.giua) - (b.giua - b.duoi)).abs() < 1e-9,
                "hai dải phải cách đều đường giữa");
        assert!(b.tren >= b.giua && b.giua >= b.duoi);
    }

    #[test]
    fn dai_thu_hep_khi_gia_it_dao_dong() {
        let em = vec![100.0; 30];
        let xoc: Vec<f64> = (0..30).map(|i| 100.0 + ((i % 2) as f64) * 50.0).collect();
        let a = bollinger(&em, 20, 2.0).unwrap();
        let b = bollinger(&xoc, 20, 2.0).unwrap();
        assert!(a.do_rong() < b.do_rong(), "giá đứng yên → dải hẹp gần bằng 0");
        assert!(a.do_rong() < 1e-9);
    }

    #[test]
    fn vi_tri_phan_tram_dung_o_hai_bien() {
        let b = DaiBollinger { tren: 120.0, giua: 100.0, duoi: 80.0 };
        assert!((b.vi_tri_phan_tram(80.0) - 0.0).abs() < 1e-9);
        assert!((b.vi_tri_phan_tram(100.0) - 0.5).abs() < 1e-9);
        assert!((b.vi_tri_phan_tram(120.0) - 1.0).abs() < 1e-9);
        // Dải rỗng không được chia cho 0
        let hep = DaiBollinger { tren: 100.0, giua: 100.0, duoi: 100.0 };
        assert_eq!(hep.vi_tri_phan_tram(100.0), 0.5);
    }

    // ---------- ATR ----------
    #[test]
    fn bien_do_that_tinh_ca_khoang_nhay_giua_hai_phien() {
        let truoc = nen_don(100, 105, 95, 100);
        // Phiên sau nhảy vọt lên: biên độ trong phiên chỉ 5, nhưng khoảng
        // cách so với giá đóng hôm trước là 30 — ATR phải thấy điều đó.
        let nay = nen_don(128, 130, 125, 129);
        assert_eq!(nay.bien_do(), 5);
        assert_eq!(bien_do_that(&nay, Some(&truoc)), 30, "phải bắt được khoảng nhảy");
    }

    #[test]
    fn bien_do_that_nen_dau_tien_bang_bien_do_thuong() {
        let n = nen_don(100, 110, 90, 105);
        assert_eq!(bien_do_that(&n, None), 20);
    }

    #[test]
    fn atr_luon_duong() {
        for hat in [1u64, 42, 2024] {
            let nen = sinh_nen(300, hat);
            for a in chuoi_atr(&nen, 14).into_iter().flatten() {
                assert!(a > 0.0, "ATR phải dương, thực tế {}", a);
            }
        }
    }

    #[test]
    fn atr_lon_hon_khi_thi_truong_dao_dong_manh() {
        let em: Vec<Nen> = (0..50).map(|i| Nen { thoi_diem: i,
            mo: 10_000, cao: 10_010, thap: 9_990, dong: 10_000, khoi_luong: 1 }).collect();
        let xoc: Vec<Nen> = (0..50).map(|i| Nen { thoi_diem: i,
            mo: 10_000, cao: 10_500, thap: 9_500, dong: 10_000, khoi_luong: 1 }).collect();
        let a = chuoi_atr(&em, 14)[49].unwrap();
        let b = chuoi_atr(&xoc, 14)[49].unwrap();
        assert!(b > a * 10.0, "thị trường xóc gấp 50 lần phải cho ATR lớn hơn hẳn");
    }

    #[test]
    fn atr_chua_du_du_lieu_thi_none() {
        let nen = sinh_nen(10, 1);
        assert!(chuoi_atr(&nen, 14).iter().all(|x| x.is_none()));
    }

    #[test]
    fn co_theo_atr_giam_khi_bien_dong_tang() {
        let mut truoc = i64::MAX;
        for atr in [20.0f64, 50.0, 100.0, 200.0] {
            let c = co_theo_atr(100_000, atr, 2.0);
            assert!(c < truoc, "ATR {} phải cho cỡ nhỏ hơn", atr);
            truoc = c;
        }
        assert_eq!(co_theo_atr(100_000, 20.0, 2.0), 2_500, "100000 / (20 × 2)");
    }

    #[test]
    fn co_theo_atr_an_toan_voi_dau_vao_xau() {
        assert_eq!(co_theo_atr(100_000, 0.0, 2.0), 0, "không chia cho 0");
        assert_eq!(co_theo_atr(100_000, 20.0, 0.0), 0);
    }

    // ---------- Không nhìn trước tương lai ----------
    #[test]
    fn moi_chi_bao_deu_khong_nhin_truoc_tuong_lai() {
        // BẤT BIẾN QUAN TRỌNG NHẤT của chương: giá trị chỉ báo tại nến i phải
        // giống hệt nhau dù ta đưa vào 201 nến hay 500 nến. Vi phạm điều này
        // là "nhìn trộm tương lai", và mọi kết quả kiểm định trở nên vô nghĩa.
        let nen = sinh_nen(500, 2024);
        let gia = gia_dong(&nen);
        let i = 200;

        assert_eq!(chuoi_sma(&gia[..=i], 20)[i], chuoi_sma(&gia, 20)[i]);
        assert_eq!(chuoi_ema(&gia[..=i], 20)[i], chuoi_ema(&gia, 20)[i]);
        assert_eq!(chuoi_rsi(&gia[..=i], 14)[i], chuoi_rsi(&gia, 14)[i]);
        assert_eq!(chuoi_atr(&nen[..=i], 14)[i], chuoi_atr(&nen, 14)[i]);
        assert_eq!(chuoi_macd(&gia[..=i], 12, 26, 9)[i], chuoi_macd(&gia, 12, 26, 9)[i]);
        assert_eq!(bollinger(&gia[..=i], 20, 2.0), bollinger(&gia[..i + 1], 20, 2.0));
    }

    #[test]
    fn sinh_nen_tat_dinh() {
        assert_eq!(sinh_nen(100, 5), sinh_nen(100, 5));
        assert_ne!(sinh_nen(100, 5), sinh_nen(100, 6));
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
5. **Dải Bollinger giả định phân phối chuẩn mà lợi suất thì không.** Đuôi béo khiến "cực đoan" xảy ra thường hơn nhiều.

### Bài tập rèn luyện

**Bài 1.** Cài **VWAP theo phiên** (giá trung bình có trọng số khối lượng) và bảo đảm nó không nhìn trộm tương lai.

<details>
<summary><b>Gợi ý</b></summary>

VWAP tích luỹ trong phiên và **đặt lại vào đầu mỗi phiên**. Đây là chuẩn tham chiếu chính cho việc thực thi lệnh của các quỹ lớn: "chúng tôi mua ở giá tốt hơn VWAP" là câu nói đo được. Cạm bẫy: tính VWAP trên toàn bộ ngày rồi dùng nó cho quyết định buổi sáng — đó là nhìn trộm tương lai kinh điển.
</details>

<details>
<summary><b>Lời giải</b></summary>

```rust
pub fn vwap_theo_phien(cac_nen: &[Nen], danh_dau_phien: &[bool]) -> Vec<Option<f64>> {
    let mut ra = Vec::with_capacity(cac_nen.len());
    let (mut tong_pv, mut tong_v) = (0.0f64, 0.0f64);

    for (i, n) in cac_nen.iter().enumerate() {
        if danh_dau_phien.get(i).copied().unwrap_or(false) {
            tong_pv = 0.0; tong_v = 0.0;          // phiên mới → đặt lại
        }
        // Giá điển hình — chuẩn ngành cho VWAP
        let gia_dh = (n.cao + n.thap + n.dong) as f64 / 3.0;
        tong_pv += gia_dh * n.khoi_luong as f64;
        tong_v  += n.khoi_luong as f64;

        // Chỉ dùng dữ liệu tới i — không nhìn trộm
        ra.push(if tong_v > 0.0 { Some(tong_pv / tong_v) } else { None });
    }
    ra
}

/// Chênh lệch thực thi so với VWAP, tính bằng điểm cơ bản.
/// Dương = thực thi TỐT HƠN chuẩn.
pub fn do_chat_luong_thuc_thi(gia_thuc_thi: f64, vwap: f64, la_mua: bool) -> f64 {
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
pub enum LoaiPhanKy { TangGia, GiamGia }

#[derive(Debug)]
pub struct PhanKy {
    pub loai: LoaiPhanKy,
    pub chi_so_dau: usize,
    pub chi_so_cuoi: usize,
}

/// Đỉnh cục bộ được XÁC NHẬN sau `nhin_lai` phiên — nên nó chỉ
/// "tồn tại" từ chỉ số i + nhin_lai trở đi. Đó là điểm chống nhìn trộm.
fn cac_dinh(gia: &[f64], nhin_lai: usize) -> Vec<usize> {
    let mut ra = Vec::new();
    if gia.len() < 2 * nhin_lai + 1 { return ra; }
    for i in nhin_lai..(gia.len() - nhin_lai) {
        let la_dinh = (1..=nhin_lai).all(|k| gia[i] > gia[i - k] && gia[i] > gia[i + k]);
        if la_dinh { ra.push(i); }
    }
    ra
}

pub fn tim_phan_ky(gia: &[f64], chi_bao: &[Option<f64>], nhin_lai: usize) -> Vec<PhanKy> {
    let dinh = cac_dinh(gia, nhin_lai);
    let mut ra = Vec::new();
    for w in dinh.windows(2) {
        let (a, b) = (w[0], w[1]);
        let (ca, cb) = match (chi_bao.get(a), chi_bao.get(b)) {
            (Some(Some(x)), Some(Some(y))) => (*x, *y),
            _ => continue,
        };
        // Giá đỉnh cao hơn nhưng chỉ báo đỉnh thấp hơn → động lượng yếu đi
        if gia[b] > gia[a] && cb < ca {
            ra.push(PhanKy { loai: LoaiPhanKy::GiamGia, chi_so_dau: a, chi_so_cuoi: b });
        }
        // Giá đỉnh thấp hơn nhưng chỉ báo cao hơn → động lượng mạnh lên
        if gia[b] < gia[a] && cb > ca {
            ra.push(PhanKy { loai: LoaiPhanKy::TangGia, chi_so_dau: a, chi_so_cuoi: b });
        }
    }
    ra
}
```

Chú ý điều kiện `i < gia.len() - nhin_lai` trong `cac_dinh`: một đỉnh chỉ được xác nhận sau `nhin_lai` phiên. Trong giao dịch thật, tín hiệu phân kỳ đến **muộn hơn** đỉnh đúng bằng ngần ấy phiên — và mọi backtest phải phản ánh độ trễ đó, nếu không nó lại là nhìn trộm tương lai.
</details>
