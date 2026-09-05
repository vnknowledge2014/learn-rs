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
| Chiến lược | Payoff của spread, straddle, condor |

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

### 2. Cân bằng quyền mua – quyền bán: quan hệ không thể sai

```
C − P = S − K·e^(−rT)
```

Đây **không phải mô hình**. Đây là quan hệ chênh lệch giá thuần tuý: nếu nó bị vi phạm, tồn tại lợi nhuận không rủi ro. Nó đúng bất kể mô hình định giá nào bạn dùng, bất kể giả định nào.

Trong thực hành, đây là bài kiểm tra tính đúng đắn hàng đầu: nếu cài đặt của bạn vi phạm cân bằng quyền mua – quyền bán, bạn có bug — không cần tranh luận thêm.

Từ đó suy ra cận dưới của quyền bán châu Âu: `P ≥ K·e^(−rT) − S`. Chú ý đây **thấp hơn** giá trị nội tại `K − S` khi `r > 0`, và đó chính là điểm đính chính ở đầu chương.

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

pub fn n_chuan(x: f64) -> f64 {
    const A1: f64 = 0.319381530;
    const A2: f64 = -0.356563782;
    const A3: f64 = 1.781477937;
    const A4: f64 = -1.821255978;
    const A5: f64 = 1.330274429;
    const P: f64 = 0.2316419;

    // Đối xứng: N(−x) = 1 − N(x). Xấp xỉ chỉ chính xác cho x ≥ 0.
    if x < 0.0 { return 1.0 - n_chuan(-x); }
    let k = 1.0 / (1.0 + P * x);
    let mat_do = (-0.5 * x * x).exp() / (2.0 * std::f64::consts::PI).sqrt();
    let da_thuc = k * (A1 + k * (A2 + k * (A3 + k * (A4 + k * A5))));
    1.0 - mat_do * da_thuc
}

/// Hàm mật độ xác suất chuẩn tắc — dùng cho gamma và vega.
pub fn mat_do_chuan(x: f64) -> f64 {
    (-0.5 * x * x).exp() / (2.0 * std::f64::consts::PI).sqrt()
}

// ============================================================================
// 2. THAM SỐ & CÔNG THỨC BLACK–SCHOLES
// ============================================================================

#[derive(Debug, Clone, Copy, PartialEq)]
pub enum LoaiQuyen { Mua, Ban }

#[derive(Debug, Clone, Copy, PartialEq)]
pub struct ThamSoQuyen {
    /// Giá tài sản cơ sở hiện tại.
    pub gia_co_so: f64,
    /// Giá thực hiện.
    pub gia_thuc_hien: f64,
    /// Thời gian còn lại, tính bằng NĂM.
    pub thoi_gian_nam: f64,
    /// Lãi suất phi rủi ro, dạng thập phân (0.05 = 5%/năm).
    pub lai_suat: f64,
    /// Biến động hằng năm, dạng thập phân (0.20 = 20%).
    pub bien_dong: f64,
}

impl ThamSoQuyen {
    pub fn hop_le(&self) -> bool {
        self.gia_co_so > 0.0 && self.gia_thuc_hien > 0.0
            && self.thoi_gian_nam >= 0.0 && self.bien_dong >= 0.0
    }

    /// d₁ và d₂ — hai đại lượng trung tâm của Black–Scholes.
    /// Trả `None` khi đã đáo hạn hoặc biến động bằng 0 (khi đó công thức
    /// suy biến và ta phải dùng giá trị nội tại).
    pub fn d1_d2(&self) -> Option<(f64, f64)> {
        if self.thoi_gian_nam <= 0.0 || self.bien_dong <= 0.0 { return None; }
        let sqrt_t = self.thoi_gian_nam.sqrt();
        let d1 = ((self.gia_co_so / self.gia_thuc_hien).ln()
                  + (self.lai_suat + 0.5 * self.bien_dong * self.bien_dong)
                    * self.thoi_gian_nam)
                 / (self.bien_dong * sqrt_t);
        Some((d1, d1 - self.bien_dong * sqrt_t))
    }

    /// Giá trị hiện tại của giá thực hiện.
    pub fn gia_thuc_hien_chiet_khau(&self) -> f64 {
        self.gia_thuc_hien * (-self.lai_suat * self.thoi_gian_nam).exp()
    }

    /// Giá trị NỘI TẠI: phần lãi nếu thực hiện ngay lập tức.
    pub fn gia_tri_noi_tai(&self, loai: LoaiQuyen) -> f64 {
        match loai {
            LoaiQuyen::Mua => (self.gia_co_so - self.gia_thuc_hien).max(0.0),
            LoaiQuyen::Ban => (self.gia_thuc_hien - self.gia_co_so).max(0.0),
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
    pub fn can_duoi_chau_au(&self, loai: LoaiQuyen) -> f64 {
        let k_ck = self.gia_thuc_hien_chiet_khau();
        match loai {
            LoaiQuyen::Mua => (self.gia_co_so - k_ck).max(0.0),
            LoaiQuyen::Ban => (k_ck - self.gia_co_so).max(0.0),
        }
    }
}

/// Giá quyền chọn kiểu châu Âu theo Black–Scholes.
pub fn gia_black_scholes(t: &ThamSoQuyen, loai: LoaiQuyen) -> f64 {
    match t.d1_d2() {
        // Đáo hạn hoặc không biến động → giá đúng bằng giá trị nội tại
        None => t.gia_tri_noi_tai(loai),
        Some((d1, d2)) => {
            let k_ck = t.gia_thuc_hien_chiet_khau();
            match loai {
                LoaiQuyen::Mua => t.gia_co_so * n_chuan(d1) - k_ck * n_chuan(d2),
                LoaiQuyen::Ban => k_ck * n_chuan(-d2) - t.gia_co_so * n_chuan(-d1),
            }
        }
    }
}

/// Giá trị THỜI GIAN = giá thị trường − giá trị nội tại. Nó luôn ≥ 0 và tan
/// dần về 0 khi tới ngày đáo hạn. Đây chính là thứ người bán quyền chọn ăn.
pub fn gia_tri_thoi_gian(t: &ThamSoQuyen, loai: LoaiQuyen) -> f64 {
    (gia_black_scholes(t, loai) - t.gia_tri_noi_tai(loai)).max(0.0)
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

pub fn tinh_greeks(t: &ThamSoQuyen, loai: LoaiQuyen) -> Greeks {
    let (d1, d2) = match t.d1_d2() {
        Some(x) => x,
        None => {
            // Đã đáo hạn: delta là bậc thang 0/1, mọi thứ khác bằng 0
            let trong_tien = match loai {
                LoaiQuyen::Mua => t.gia_co_so > t.gia_thuc_hien,
                LoaiQuyen::Ban => t.gia_co_so < t.gia_thuc_hien,
            };
            let d = if !trong_tien { 0.0 }
                    else if loai == LoaiQuyen::Mua { 1.0 } else { -1.0 };
            return Greeks { delta: d, gamma: 0.0, vega: 0.0, theta: 0.0, rho: 0.0 };
        }
    };
    let sqrt_t = t.thoi_gian_nam.sqrt();
    let md = mat_do_chuan(d1);
    let k_ck = t.gia_thuc_hien_chiet_khau();

    // Gamma và vega GIỐNG HỆT NHAU cho quyền mua và quyền bán cùng tham số —
    // hệ quả trực tiếp của ngang giá mua-bán.
    let gamma = md / (t.gia_co_so * t.bien_dong * sqrt_t);
    let vega = t.gia_co_so * md * sqrt_t / 100.0; // trên 1 điểm phần trăm

    let (delta, theta, rho) = match loai {
        LoaiQuyen::Mua => (
            n_chuan(d1),
            (-t.gia_co_so * md * t.bien_dong / (2.0 * sqrt_t)
             - t.lai_suat * k_ck * n_chuan(d2)) / 365.0,
            k_ck * t.thoi_gian_nam * n_chuan(d2) / 100.0,
        ),
        LoaiQuyen::Ban => (
            n_chuan(d1) - 1.0,
            (-t.gia_co_so * md * t.bien_dong / (2.0 * sqrt_t)
             + t.lai_suat * k_ck * n_chuan(-d2)) / 365.0,
            -k_ck * t.thoi_gian_nam * n_chuan(-d2) / 100.0,
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
pub fn bien_dong_ngu_y(t: &ThamSoQuyen, loai: LoaiQuyen, gia_thi_truong: f64)
    -> Option<f64>
{
    // Dùng cận dưới CHÂU ÂU, không phải giá trị nội tại: quyền bán châu Âu
    // sâu trong tiền hợp lệ khi nằm DƯỚI nội tại. Nếu chặn theo nội tại,
    // ta sẽ từ chối oan những mức giá hoàn toàn bình thường.
    let can_duoi = t.can_duoi_chau_au(loai);
    if gia_thi_truong < can_duoi - 1e-9 { return None; }
    if t.thoi_gian_nam <= 0.0 { return None; }

    let (mut lo, mut hi) = (1e-6f64, 5.0f64);
    let gia_tai = |v: f64| {
        gia_black_scholes(&ThamSoQuyen { bien_dong: v, ..*t }, loai)
    };
    // Giá thị trường phải nằm trong khoảng dựng được
    if gia_thi_truong > gia_tai(hi) { return None; }

    for _ in 0..200 {
        let giua = 0.5 * (lo + hi);
        if gia_tai(giua) < gia_thi_truong { lo = giua; } else { hi = giua; }
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
pub enum LoaiCauPhan { QuyenMua, QuyenBan, TaiSanCoSo }

#[derive(Debug, Clone, Copy, PartialEq)]
pub struct CauPhan {
    pub loai: LoaiCauPhan,
    /// Dương = mua (trường vị), âm = bán (đoản vị).
    pub so_luong: f64,
    pub gia_thuc_hien: f64,
    /// Số tiền đã trả (mua) hoặc nhận (bán) cho mỗi đơn vị.
    pub phi_quyen: f64,
}

impl CauPhan {
    /// Lãi/lỗ của riêng cấu phần này tại giá đáo hạn `s`.
    pub fn lai_lo(&self, s: f64) -> f64 {
        let gia_tri = match self.loai {
            LoaiCauPhan::QuyenMua => (s - self.gia_thuc_hien).max(0.0),
            LoaiCauPhan::QuyenBan => (self.gia_thuc_hien - s).max(0.0),
            LoaiCauPhan::TaiSanCoSo => s,
        };
        self.so_luong * (gia_tri - self.phi_quyen)
    }
}

#[derive(Debug, Clone, PartialEq)]
pub struct ChienLuocQuyen { pub ten: String, pub cau_phan: Vec<CauPhan> }

impl ChienLuocQuyen {
    pub fn lai_lo(&self, s: f64) -> f64 {
        self.cau_phan.iter().map(|c| c.lai_lo(s)).sum()
    }

    /// Chi phí ban đầu. Dương = phải trả tiền, âm = được nhận tiền.
    pub fn chi_phi_ban_dau(&self) -> f64 {
        self.cau_phan.iter().map(|c| c.so_luong * c.phi_quyen).sum()
    }

    /// Các điểm hoà vốn, tìm bằng cách quét dải giá và bắt chỗ đổi dấu.
    pub fn diem_hoa_von(&self, tu: f64, den: f64, buoc: f64) -> Vec<f64> {
        let mut ra = Vec::new();
        let mut s = tu;
        let mut truoc = self.lai_lo(s);
        while s < den {
            s += buoc;
            let nay = self.lai_lo(s);
            if truoc.signum() != nay.signum() && truoc.abs() > 1e-9 {
                ra.push(s - buoc * 0.5);
            }
            truoc = nay;
        }
        ra
    }

    pub fn lai_toi_da_trong_dai(&self, tu: f64, den: f64, buoc: f64) -> f64 {
        let mut m = f64::MIN;
        let mut s = tu;
        while s <= den { m = m.max(self.lai_lo(s)); s += buoc; }
        m
    }
    pub fn lo_toi_da_trong_dai(&self, tu: f64, den: f64, buoc: f64) -> f64 {
        let mut m = f64::MAX;
        let mut s = tu;
        while s <= den { m = m.min(self.lai_lo(s)); s += buoc; }
        m
    }
}

// --- Các chiến lược dựng sẵn ---

/// Mua cả quyền mua lẫn quyền bán cùng giá thực hiện: cược GIÁ SẼ ĐỘNG MẠNH,
/// không quan tâm hướng nào. Lỗ tối đa = tổng phí, xảy ra khi giá đứng yên.
pub fn straddle(gia_thuc_hien: f64, phi_mua: f64, phi_ban: f64) -> ChienLuocQuyen {
    ChienLuocQuyen {
        ten: "Straddle (mua đôi cùng giá)".into(),
        cau_phan: vec![
            CauPhan { loai: LoaiCauPhan::QuyenMua, so_luong: 1.0, gia_thuc_hien,
                      phi_quyen: phi_mua },
            CauPhan { loai: LoaiCauPhan::QuyenBan, so_luong: 1.0, gia_thuc_hien,
                      phi_quyen: phi_ban },
        ],
    }
}

/// Như straddle nhưng hai giá thực hiện cách xa nhau: rẻ hơn, nhưng cần giá
/// động mạnh hơn mới có lãi.
pub fn strangle(gia_ban: f64, gia_mua: f64, phi_mua: f64, phi_ban: f64)
    -> ChienLuocQuyen
{
    ChienLuocQuyen {
        ten: "Strangle (mua đôi khác giá)".into(),
        cau_phan: vec![
            CauPhan { loai: LoaiCauPhan::QuyenMua, so_luong: 1.0,
                      gia_thuc_hien: gia_mua, phi_quyen: phi_mua },
            CauPhan { loai: LoaiCauPhan::QuyenBan, so_luong: 1.0,
                      gia_thuc_hien: gia_ban, phi_quyen: phi_ban },
        ],
    }
}

/// Mua quyền mua giá thấp, bán quyền mua giá cao: cược giá TĂNG VỪA PHẢI.
/// Cả lãi lẫn lỗ đều có trần — đây là điểm hấp dẫn của chênh lệch giá.
pub fn chenh_lech_gia_tang(gia_thap: f64, gia_cao: f64, phi_thap: f64, phi_cao: f64)
    -> ChienLuocQuyen
{
    ChienLuocQuyen {
        ten: "Chênh lệch giá tăng".into(),
        cau_phan: vec![
            CauPhan { loai: LoaiCauPhan::QuyenMua, so_luong: 1.0,
                      gia_thuc_hien: gia_thap, phi_quyen: phi_thap },
            CauPhan { loai: LoaiCauPhan::QuyenMua, so_luong: -1.0,
                      gia_thuc_hien: gia_cao, phi_quyen: phi_cao },
        ],
    }
}

/// Nắm giữ tài sản và bán quyền mua trên nó: thu thêm phí, đổi lại từ bỏ
/// phần tăng giá vượt quá giá thực hiện.
pub fn quyen_mua_co_bao_dam(gia_von: f64, gia_thuc_hien: f64, phi: f64)
    -> ChienLuocQuyen
{
    ChienLuocQuyen {
        ten: "Quyền mua có bảo đảm".into(),
        cau_phan: vec![
            CauPhan { loai: LoaiCauPhan::TaiSanCoSo, so_luong: 1.0,
                      gia_thuc_hien: 0.0, phi_quyen: gia_von },
            CauPhan { loai: LoaiCauPhan::QuyenMua, so_luong: -1.0,
                      gia_thuc_hien, phi_quyen: phi },
        ],
    }
}

/// Bốn chân: bán một strangle hẹp, mua một strangle rộng để chặn rủi ro.
/// Cược giá NẰM YÊN trong một khoảng. Lãi có trần, lỗ cũng có trần.
pub fn dieu_hau_sat(ban_thap: f64, mua_thap: f64, ban_cao: f64, mua_cao: f64,
                    phi: [f64; 4]) -> ChienLuocQuyen
{
    ChienLuocQuyen {
        ten: "Điều hâu sắt".into(),
        cau_phan: vec![
            CauPhan { loai: LoaiCauPhan::QuyenBan, so_luong: 1.0,
                      gia_thuc_hien: mua_thap, phi_quyen: phi[0] },
            CauPhan { loai: LoaiCauPhan::QuyenBan, so_luong: -1.0,
                      gia_thuc_hien: ban_thap, phi_quyen: phi[1] },
            CauPhan { loai: LoaiCauPhan::QuyenMua, so_luong: -1.0,
                      gia_thuc_hien: ban_cao, phi_quyen: phi[2] },
            CauPhan { loai: LoaiCauPhan::QuyenMua, so_luong: 1.0,
                      gia_thuc_hien: mua_cao, phi_quyen: phi[3] },
        ],
    }
}

fn main() {
    println!("═══════════════════════════════════════════════════════════");
    println!("   QUYỀN CHỌN & PHÁI SINH BẰNG RUST (giáo trình OpenAlgo)   ");
    println!("═══════════════════════════════════════════════════════════");

    let t = ThamSoQuyen { gia_co_so: 100.0, gia_thuc_hien: 100.0,
                          thoi_gian_nam: 0.25, lai_suat: 0.05, bien_dong: 0.20 };

    println!("\n1. HÀM PHÂN PHỐI CHUẨN — đối chiếu giá trị đã biết");
    for (x, mong) in [(0.0, 0.5000), (1.0, 0.8413), (1.96, 0.9750), (-1.0, 0.1587)] {
        println!("   N({:>5.2}) = {:.4}   (kỳ vọng {:.4})", x, n_chuan(x), mong);
    }

    println!("\n2. ĐỊNH GIÁ BLACK–SCHOLES");
    println!("   Cơ sở {} · thực hiện {} · {} tháng · lãi suất {}% · biến động {}%",
             t.gia_co_so, t.gia_thuc_hien, t.thoi_gian_nam * 12.0,
             t.lai_suat * 100.0, t.bien_dong * 100.0);
    let c = gia_black_scholes(&t, LoaiQuyen::Mua);
    let p = gia_black_scholes(&t, LoaiQuyen::Ban);
    println!("   Quyền mua {:.4} (nội tại {:.2} + thời gian {:.4})",
             c, t.gia_tri_noi_tai(LoaiQuyen::Mua), gia_tri_thoi_gian(&t, LoaiQuyen::Mua));
    println!("   Quyền bán {:.4} (nội tại {:.2} + thời gian {:.4})",
             p, t.gia_tri_noi_tai(LoaiQuyen::Ban), gia_tri_thoi_gian(&t, LoaiQuyen::Ban));

    println!("\n3. NGANG GIÁ MUA-BÁN — bất biến kiểm chứng được");
    let trai = c - p;
    let phai = t.gia_co_so - t.gia_thuc_hien_chiet_khau();
    println!("   C − P       = {:.10}", trai);
    println!("   S − K·e^-rT = {:.10}", phai);
    println!("   Sai lệch    = {:.2e}", (trai - phai).abs());
    println!("   → Nếu hệ thức này lệch trên thị trường thật thì có cơ hội arbitrage");
    println!("     KHÔNG RỦI RO. Vì thế nó gần như không bao giờ lệch.");

    println!("\n4. CÁC THAM SỐ NHẠY");
    let gm = tinh_greeks(&t, LoaiQuyen::Mua);
    let gb = tinh_greeks(&t, LoaiQuyen::Ban);
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
        let x = ThamSoQuyen { gia_co_so: s, ..t };
        let g = tinh_greeks(&x, LoaiQuyen::Mua);
        println!("   {:>10.0} {:>12.4} {:>12.4} {:>12.4}",
                 s, g.delta, g.gamma, gia_black_scholes(&x, LoaiQuyen::Mua));
    }
    println!("   → Delta đi từ 0 tới 1. Gamma lớn nhất quanh giá thực hiện —");
    println!("     đó là chỗ delta thay đổi nhanh nhất, và cũng nguy hiểm nhất.");

    println!("\n6. THỜI GIAN TAN DẦN");
    println!("   {:>14} {:>16} {:>18}", "còn lại", "giá quyền mua", "giá trị thời gian");
    for ngay in [90.0f64, 60.0, 30.0, 7.0, 1.0, 0.0] {
        let x = ThamSoQuyen { thoi_gian_nam: ngay / 365.0, ..t };
        println!("   {:>11.0} ngày {:>16.4} {:>18.4}",
                 ngay, gia_black_scholes(&x, LoaiQuyen::Mua),
                 gia_tri_thoi_gian(&x, LoaiQuyen::Mua));
    }
    println!("   → Giá trị thời gian tan NHANH DẦN về cuối. Đó là lý do người bán");
    println!("     quyền chọn thích những tuần cuối, còn người mua thì sợ chúng.");

    println!("\n7. BIẾN ĐỘNG NGỤ Ý");
    for bd_that in [0.10f64, 0.20, 0.35, 0.60] {
        let x = ThamSoQuyen { bien_dong: bd_that, ..t };
        let gia = gia_black_scholes(&x, LoaiQuyen::Mua);
        let bd_tim = bien_dong_ngu_y(&x, LoaiQuyen::Mua, gia).unwrap();
        println!("   biến động thật {:>5.1}% → giá {:>7.4} → tìm ngược ra {:>5.2}%",
                 bd_that * 100.0, gia, bd_tim * 100.0);
    }

    println!("\n8. ĐỒ THỊ LÃI/LỖ CÁC CHIẾN LƯỢC TẠI ĐÁO HẠN");
    let cl = vec![
        straddle(100.0, 4.0, 3.0),
        strangle(95.0, 105.0, 2.0, 1.5),
        chenh_lech_gia_tang(95.0, 105.0, 7.0, 2.0),
        quyen_mua_co_bao_dam(100.0, 110.0, 3.0),
        dieu_hau_sat(95.0, 90.0, 105.0, 110.0, [1.0, 2.5, 2.5, 1.0]),
    ];
    print!("   {:<28}", "giá đáo hạn →");
    for s in [80.0f64, 90.0, 100.0, 110.0, 120.0] { print!("{:>9.0}", s); }
    println!();
    for c in &cl {
        print!("   {:<28}", c.ten);
        for s in [80.0f64, 90.0, 100.0, 110.0, 120.0] { print!("{:>9.1}", c.lai_lo(s)); }
        println!();
    }
    println!("\n   {:<28} {:>12} {:>12} {:>14}",
             "chiến lược", "chi phí đầu", "lãi tối đa", "lỗ tối đa");
    for c in &cl {
        println!("   {:<28} {:>12.1} {:>12.1} {:>14.1}",
                 c.ten, c.chi_phi_ban_dau(),
                 c.lai_toi_da_trong_dai(0.0, 300.0, 0.5),
                 c.lo_toi_da_trong_dai(0.0, 300.0, 0.5));
    }
    println!("\n   Điểm hoà vốn của straddle: {:?}",
             cl[0].diem_hoa_von(50.0, 150.0, 0.1).iter()
                  .map(|x| (x * 10.0).round() / 10.0).collect::<Vec<_>>());

    println!("\n═══════════════════════════════════════════════════════════");
    println!("   MUA QUYỀN: LỖ CÓ TRẦN. BÁN QUYỀN TRẦN TRỤI: LỖ KHÔNG TRẦN");
    println!("═══════════════════════════════════════════════════════════");
}

#[cfg(test)]
mod kiem_thu {
    use super::*;

    fn ts() -> ThamSoQuyen {
        ThamSoQuyen { gia_co_so: 100.0, gia_thuc_hien: 100.0,
                      thoi_gian_nam: 0.25, lai_suat: 0.05, bien_dong: 0.20 }
    }

    // ---------- Phân phối chuẩn ----------
    #[test]
    fn n_chuan_khop_gia_tri_da_biet() {
        assert!((n_chuan(0.0) - 0.5).abs() < 1e-7);
        assert!((n_chuan(1.0) - 0.841_344_746).abs() < 1e-6);
        assert!((n_chuan(-1.0) - 0.158_655_254).abs() < 1e-6);
        assert!((n_chuan(1.96) - 0.975_002_105).abs() < 1e-6);
        assert!((n_chuan(2.576) - 0.995_003_1).abs() < 1e-5);
    }

    #[test]
    fn n_chuan_doi_xung_va_don_dieu() {
        let mut truoc = 0.0;
        let mut x = -5.0;
        while x <= 5.0 {
            let v = n_chuan(x);
            assert!((0.0..=1.0).contains(&v), "N({}) = {} ra ngoài [0,1]", x, v);
            assert!(v >= truoc - 1e-12, "N phải tăng đơn điệu");
            assert!((v + n_chuan(-x) - 1.0).abs() < 1e-7, "N(x) + N(−x) = 1");
            truoc = v;
            x += 0.1;
        }
    }

    #[test]
    fn mat_do_chuan_dat_dinh_tai_khong() {
        let dinh = mat_do_chuan(0.0);
        assert!((dinh - 0.398_942_28).abs() < 1e-7);
        assert!(mat_do_chuan(1.0) < dinh);
        assert!((mat_do_chuan(1.5) - mat_do_chuan(-1.5)).abs() < 1e-12, "hàm chẵn");
    }

    // ---------- Black–Scholes ----------
    #[test]
    fn ngang_gia_mua_ban_luon_dung() {
        // BẤT BIẾN QUAN TRỌNG NHẤT của chương: C − P = S − K·e^(−rT).
        // Nếu nó lệch trên thị trường thật thì có arbitrage không rủi ro.
        for s in [50.0f64, 80.0, 100.0, 120.0, 200.0] {
            for k in [80.0f64, 100.0, 120.0] {
                for v in [0.1f64, 0.2, 0.5] {
                    for tg in [0.01f64, 0.25, 1.0, 2.0] {
                        let t = ThamSoQuyen { gia_co_so: s, gia_thuc_hien: k,
                                              thoi_gian_nam: tg, lai_suat: 0.05,
                                              bien_dong: v };
                        let c = gia_black_scholes(&t, LoaiQuyen::Mua);
                        let p = gia_black_scholes(&t, LoaiQuyen::Ban);
                        let lech = (c - p) - (s - t.gia_thuc_hien_chiet_khau());
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
                let t = ThamSoQuyen { gia_co_so: s, gia_thuc_hien: k,
                                      thoi_gian_nam: 0.5, lai_suat: 0.05,
                                      bien_dong: 0.3 };
                assert!(gia_black_scholes(&t, LoaiQuyen::Mua) >= -1e-9);
                assert!(gia_black_scholes(&t, LoaiQuyen::Ban) >= -1e-9);
            }
        }
    }

    #[test]
    fn gia_quyen_luon_it_nhat_bang_can_duoi_chau_au() {
        // Cận dưới ĐÚNG cho quyền châu Âu là max(0, S − K·e^(−rT)) và
        // max(0, K·e^(−rT) − S) — KHÔNG phải giá trị nội tại.
        for s in [20.0f64, 60.0, 100.0, 150.0, 300.0] {
            for tg in [0.1f64, 1.0, 5.0] {
                let t = ThamSoQuyen { gia_co_so: s, thoi_gian_nam: tg, ..ts() };
                for loai in [LoaiQuyen::Mua, LoaiQuyen::Ban] {
                    assert!(gia_black_scholes(&t, loai) >= t.can_duoi_chau_au(loai) - 1e-9,
                            "S={} T={} loại {:?}", s, tg, loai);
                }
            }
        }
    }

    #[test]
    fn quyen_mua_chau_au_luon_tren_gia_tri_noi_tai() {
        // Với quyền MUA thì cận dưới châu Âu còn CHẶT HƠN nội tại (vì
        // K·e^(−rT) < K), nên quyền mua không bao giờ rẻ hơn nội tại.
        for s in [60.0f64, 100.0, 200.0] {
            let t = ThamSoQuyen { gia_co_so: s, ..ts() };
            assert!(t.can_duoi_chau_au(LoaiQuyen::Mua)
                    >= t.gia_tri_noi_tai(LoaiQuyen::Mua) - 1e-9);
            assert!(gia_black_scholes(&t, LoaiQuyen::Mua)
                    >= t.gia_tri_noi_tai(LoaiQuyen::Mua) - 1e-9);
        }
    }

    #[test]
    fn quyen_ban_chau_au_sau_trong_tien_CO_THE_re_hon_noi_tai() {
        // Kết quả gây bất ngờ nhưng hoàn toàn đúng — và là lý do quyền bán
        // kiểu Mỹ đắt hơn quyền bán châu Âu cùng tham số.
        let t = ThamSoQuyen { gia_co_so: 50.0, gia_thuc_hien: 100.0,
                              thoi_gian_nam: 2.0, lai_suat: 0.05, bien_dong: 0.15 };
        let gia = gia_black_scholes(&t, LoaiQuyen::Ban);
        let noi_tai = t.gia_tri_noi_tai(LoaiQuyen::Ban);
        assert!(gia < noi_tai,
                "quyền bán {:.3} phải rẻ hơn nội tại {:.3}", gia, noi_tai);
        assert!(gia >= t.can_duoi_chau_au(LoaiQuyen::Ban) - 1e-9,
                "nhưng vẫn phải trên cận dưới châu Âu");
        // Không có arbitrage: không được phép thực hiện sớm để ăn chênh lệch
        assert!(t.can_duoi_chau_au(LoaiQuyen::Ban) < noi_tai);
    }

    #[test]
    fn dao_han_thi_gia_bang_dung_gia_tri_noi_tai() {
        for s in [80.0f64, 100.0, 120.0] {
            let t = ThamSoQuyen { gia_co_so: s, thoi_gian_nam: 0.0, ..ts() };
            assert_eq!(gia_black_scholes(&t, LoaiQuyen::Mua), (s - 100.0f64).max(0.0));
            assert_eq!(gia_black_scholes(&t, LoaiQuyen::Ban), (100.0f64 - s).max(0.0));
            assert_eq!(gia_tri_thoi_gian(&t, LoaiQuyen::Mua), 0.0,
                       "đáo hạn thì giá trị thời gian bằng 0");
        }
    }

    #[test]
    fn khong_bien_dong_thi_khong_co_gia_tri_thoi_gian() {
        let t = ThamSoQuyen { bien_dong: 0.0, ..ts() };
        assert_eq!(gia_black_scholes(&t, LoaiQuyen::Mua),
                   t.gia_tri_noi_tai(LoaiQuyen::Mua));
    }

    #[test]
    fn gia_quyen_mua_tang_theo_gia_co_so() {
        let mut truoc = -1.0;
        for s in [50.0f64, 80.0, 100.0, 120.0, 200.0] {
            let g = gia_black_scholes(&ThamSoQuyen { gia_co_so: s, ..ts() },
                                      LoaiQuyen::Mua);
            assert!(g > truoc, "quyền mua phải đắt dần theo giá cơ sở");
            truoc = g;
        }
    }

    #[test]
    fn gia_quyen_tang_theo_bien_dong() {
        // Đây là lý do "bán biến động" là một chiến lược có thật: giá quyền
        // đơn điệu tăng theo biến động, nên bán khi biến động cao là bán đắt.
        for loai in [LoaiQuyen::Mua, LoaiQuyen::Ban] {
            let mut truoc = -1.0;
            for v in [0.05f64, 0.1, 0.2, 0.4, 0.8] {
                let g = gia_black_scholes(&ThamSoQuyen { bien_dong: v, ..ts() }, loai);
                assert!(g > truoc, "loại {:?} biến động {} phải đắt hơn", loai, v);
                truoc = g;
            }
        }
    }

    #[test]
    fn gia_quyen_tang_theo_thoi_gian_con_lai() {
        let mut truoc = -1.0;
        for tg in [0.01f64, 0.1, 0.25, 1.0, 2.0] {
            let g = gia_black_scholes(&ThamSoQuyen { thoi_gian_nam: tg, ..ts() },
                                      LoaiQuyen::Mua);
            assert!(g > truoc, "còn nhiều thời gian thì quyền đắt hơn");
            truoc = g;
        }
    }

    // ---------- Greeks ----------
    #[test]
    fn delta_mua_trong_0_1_delta_ban_trong_am_1_den_0() {
        for s in [20.0f64, 60.0, 100.0, 140.0, 300.0] {
            let t = ThamSoQuyen { gia_co_so: s, ..ts() };
            let dm = tinh_greeks(&t, LoaiQuyen::Mua).delta;
            let db = tinh_greeks(&t, LoaiQuyen::Ban).delta;
            assert!((0.0..=1.0).contains(&dm), "delta mua {} tại S={}", dm, s);
            assert!((-1.0..=0.0).contains(&db), "delta bán {} tại S={}", db, s);
            assert!((dm - db - 1.0).abs() < 1e-9,
                    "delta mua − delta bán phải luôn bằng 1");
        }
    }

    #[test]
    fn delta_tien_ve_1_khi_quyen_mua_rat_sau_trong_tien() {
        let sau = tinh_greeks(&ThamSoQuyen { gia_co_so: 500.0, ..ts() },
                              LoaiQuyen::Mua).delta;
        assert!(sau > 0.99, "rất sâu trong tiền → delta ≈ 1, thực tế {:.4}", sau);
        let ngoai = tinh_greeks(&ThamSoQuyen { gia_co_so: 10.0, ..ts() },
                                LoaiQuyen::Mua).delta;
        assert!(ngoai < 0.01, "rất ngoài tiền → delta ≈ 0, thực tế {:.4}", ngoai);
    }

    #[test]
    fn gamma_va_vega_giong_het_nhau_o_hai_loai_quyen() {
        // Hệ quả trực tiếp của ngang giá mua-bán: đạo hàm bậc hai theo giá và
        // đạo hàm theo biến động không phân biệt quyền mua hay quyền bán.
        for s in [70.0f64, 100.0, 130.0] {
            let t = ThamSoQuyen { gia_co_so: s, ..ts() };
            let a = tinh_greeks(&t, LoaiQuyen::Mua);
            let b = tinh_greeks(&t, LoaiQuyen::Ban);
            assert!((a.gamma - b.gamma).abs() < 1e-12);
            assert!((a.vega - b.vega).abs() < 1e-12);
        }
    }

    #[test]
    fn gamma_lon_nhat_quanh_gia_thuc_hien() {
        // Gamma là chỗ nguy hiểm nhất: quanh giá thực hiện, delta đổi nhanh
        // nhất, nên vị thế phòng hộ mất cân bằng nhanh nhất.
        let g_giua = tinh_greeks(&ts(), LoaiQuyen::Mua).gamma;
        for s in [60.0f64, 80.0, 130.0, 180.0] {
            let g = tinh_greeks(&ThamSoQuyen { gia_co_so: s, ..ts() },
                                LoaiQuyen::Mua).gamma;
            assert!(g < g_giua, "gamma tại S={} phải nhỏ hơn tại giá thực hiện", s);
        }
    }

    #[test]
    fn gamma_va_vega_luon_khong_am_khi_mua_quyen() {
        for s in [50.0f64, 100.0, 200.0] {
            for v in [0.1f64, 0.3, 0.8] {
                let g = tinh_greeks(&ThamSoQuyen { gia_co_so: s, bien_dong: v, ..ts() },
                                    LoaiQuyen::Mua);
                assert!(g.gamma >= 0.0 && g.vega >= 0.0);
            }
        }
    }

    #[test]
    fn theta_am_voi_quyen_mua_gan_gia_thuc_hien() {
        // Thời gian là kẻ thù của người MUA quyền chọn.
        let th = tinh_greeks(&ts(), LoaiQuyen::Mua).theta;
        assert!(th < 0.0, "theta phải âm, thực tế {:.6}", th);
    }

    #[test]
    fn greeks_tai_dao_han_la_bac_thang() {
        let trong = tinh_greeks(&ThamSoQuyen { gia_co_so: 120.0, thoi_gian_nam: 0.0,
                                               ..ts() }, LoaiQuyen::Mua);
        assert_eq!(trong.delta, 1.0);
        assert_eq!(trong.gamma, 0.0);
        assert_eq!(trong.theta, 0.0);
        let ngoai = tinh_greeks(&ThamSoQuyen { gia_co_so: 80.0, thoi_gian_nam: 0.0,
                                               ..ts() }, LoaiQuyen::Mua);
        assert_eq!(ngoai.delta, 0.0);
    }

    #[test]
    fn delta_khop_voi_dao_ham_so_cua_gia() {
        // Kiểm chứng chéo: delta phải bằng đạo hàm của giá theo giá cơ sở.
        let h = 0.001;
        for s in [80.0f64, 100.0, 120.0] {
            let t = ThamSoQuyen { gia_co_so: s, ..ts() };
            let len = gia_black_scholes(&ThamSoQuyen { gia_co_so: s + h, ..ts() },
                                        LoaiQuyen::Mua);
            let xuong = gia_black_scholes(&ThamSoQuyen { gia_co_so: s - h, ..ts() },
                                          LoaiQuyen::Mua);
            let dao_ham_so = (len - xuong) / (2.0 * h);
            let d = tinh_greeks(&t, LoaiQuyen::Mua).delta;
            assert!((d - dao_ham_so).abs() < 1e-4,
                    "delta {:.6} so với đạo hàm số {:.6} tại S={}", d, dao_ham_so, s);
        }
    }

    // ---------- Biến động ngụ ý ----------
    #[test]
    fn dinh_gia_lai_bang_bien_dong_tim_duoc_cho_ra_dung_gia_cu() {
        // BẤT BIẾN ĐÚNG: đưa biến động tìm được vào lại Black–Scholes phải
        // ra đúng giá ban đầu. Đây mới là điều ta thật sự cần bảo đảm.
        for v_that in [0.05f64, 0.1, 0.2, 0.35, 0.6, 1.0] {
            for s in [80.0f64, 100.0, 120.0] {
                let t = ThamSoQuyen { gia_co_so: s, bien_dong: v_that, ..ts() };
                for loai in [LoaiQuyen::Mua, LoaiQuyen::Ban] {
                    let gia = gia_black_scholes(&t, loai);
                    let tim = bien_dong_ngu_y(&t, loai, gia)
                        .unwrap_or_else(|| panic!("không tìm được IV tại S={} v={}", s, v_that));
                    let gia_lai = gia_black_scholes(
                        &ThamSoQuyen { bien_dong: tim, ..t }, loai);
                    assert!((gia_lai - gia).abs() < 1e-8,
                            "định giá lại ra {:.10} thay vì {:.10}", gia_lai, gia);
                }
            }
        }
    }

    #[test]
    fn tim_dung_bien_dong_khi_quyen_o_gan_gia_thuc_hien() {
        // Ở gần giá thực hiện, vega lớn nên giá rất nhạy với biến động và ta
        // khôi phục được con số chính xác.
        //
        // Ở rất sâu trong tiền hoặc rất ngoài tiền thì vega gần 0: giá gần
        // như không đổi dù biến động đổi nhiều, nên KHÔNG thể khôi phục chính
        // xác. Đây là hạn chế THẬT của biến động ngụ ý, không phải lỗi cài đặt.
        for v_that in [0.05f64, 0.1, 0.2, 0.35, 0.6, 1.0] {
            let t = ThamSoQuyen { bien_dong: v_that, ..ts() }; // S = K = 100
            for loai in [LoaiQuyen::Mua, LoaiQuyen::Ban] {
                let gia = gia_black_scholes(&t, loai);
                let tim = bien_dong_ngu_y(&t, loai, gia).unwrap();
                assert!((tim - v_that).abs() < 1e-5,
                        "tìm ra {:.6} thay vì {:.6}", tim, v_that);
            }
        }
    }

    #[test]
    fn vega_gan_khong_thi_bien_dong_ngu_y_kem_tin_cay() {
        // Ghi lại giới hạn một cách tường minh: vega của quyền rất sâu trong
        // tiền gần bằng 0, nên biến động ngụ ý ở đó gần như vô nghĩa.
        let sau = ThamSoQuyen { gia_co_so: 500.0, ..ts() };
        let giua = ts();
        let vega_sau = tinh_greeks(&sau, LoaiQuyen::Mua).vega;
        let vega_giua = tinh_greeks(&giua, LoaiQuyen::Mua).vega;
        assert!(vega_sau < vega_giua / 100.0,
                "vega sâu trong tiền {:.8} phải nhỏ hơn hẳn ở giá thực hiện {:.8}",
                vega_sau, vega_giua);
    }

    #[test]
    fn gia_duoi_gia_tri_noi_tai_bi_tu_choi() {
        // Giá như vậy là bất khả — dữ liệu hỏng, hoặc có cơ hội arbitrage.
        let t = ThamSoQuyen { gia_co_so: 150.0, ..ts() };
        let can_duoi = t.can_duoi_chau_au(LoaiQuyen::Mua);
        assert_eq!(bien_dong_ngu_y(&t, LoaiQuyen::Mua, can_duoi - 1.0), None);
    }

    #[test]
    fn gia_qua_cao_khong_dung_duoc_thi_tra_none() {
        let t = ts();
        assert_eq!(bien_dong_ngu_y(&t, LoaiQuyen::Mua, 99.0), None,
                   "không biến động nào cho ra giá đó");
    }

    #[test]
    fn da_dao_han_thi_khong_tinh_duoc_bien_dong_ngu_y() {
        let t = ThamSoQuyen { thoi_gian_nam: 0.0, ..ts() };
        assert_eq!(bien_dong_ngu_y(&t, LoaiQuyen::Mua, 5.0), None);
    }

    // ---------- Chiến lược ----------
    #[test]
    fn straddle_lo_nhieu_nhat_khi_gia_dung_yen() {
        let s = straddle(100.0, 4.0, 3.0);
        assert!((s.lai_lo(100.0) + 7.0).abs() < 1e-9, "đúng giá thực hiện → mất cả 7");
        assert!(s.lai_lo(80.0) > s.lai_lo(100.0), "giá động mạnh xuống → có lãi");
        assert!(s.lai_lo(120.0) > s.lai_lo(100.0), "giá động mạnh lên → có lãi");
        assert_eq!(s.chi_phi_ban_dau(), 7.0);
    }

    #[test]
    fn straddle_co_dung_hai_diem_hoa_von() {
        let s = straddle(100.0, 4.0, 3.0);
        let hv = s.diem_hoa_von(50.0, 150.0, 0.01);
        assert_eq!(hv.len(), 2, "straddle phải có đúng hai điểm hoà vốn");
        // Hoà vốn ở 100 ± 7
        assert!((hv[0] - 93.0).abs() < 0.1, "điểm dưới {:.2}", hv[0]);
        assert!((hv[1] - 107.0).abs() < 0.1, "điểm trên {:.2}", hv[1]);
    }

    #[test]
    fn strangle_re_hon_nhung_can_gia_dong_manh_hon() {
        let st = straddle(100.0, 4.0, 3.0);
        let sg = strangle(95.0, 105.0, 2.0, 1.5);
        assert!(sg.chi_phi_ban_dau() < st.chi_phi_ban_dau(), "strangle rẻ hơn");
        // Ở ngay giá 100, strangle lỗ ít hơn (vì rẻ hơn)
        assert!(sg.lai_lo(100.0) > st.lai_lo(100.0));
        // Nhưng khi giá động vừa phải, straddle lãi hơn
        assert!(st.lai_lo(112.0) > sg.lai_lo(112.0));
    }

    #[test]
    fn chenh_lech_gia_tang_co_tran_ca_lai_lan_lo() {
        let c = chenh_lech_gia_tang(95.0, 105.0, 7.0, 2.0);
        let lai_max = c.lai_toi_da_trong_dai(0.0, 500.0, 0.5);
        let lo_max = c.lo_toi_da_trong_dai(0.0, 500.0, 0.5);
        // Lãi tối đa = (105−95) − (7−2) = 5 ; lỗ tối đa = phí ròng = 5
        assert!((lai_max - 5.0).abs() < 0.1, "lãi tối đa {:.2}", lai_max);
        assert!((lo_max + 5.0).abs() < 0.1, "lỗ tối đa {:.2}", lo_max);
        // Giá tăng vô hạn cũng không lãi thêm — đó là ý nghĩa của "có trần"
        assert!((c.lai_lo(1_000.0) - c.lai_lo(200.0)).abs() < 1e-9);
    }

    #[test]
    fn quyen_mua_co_bao_dam_tu_bo_phan_tang_gia() {
        let q = quyen_mua_co_bao_dam(100.0, 110.0, 3.0);
        // Giá đứng yên: lãi đúng bằng phí thu được
        assert!((q.lai_lo(100.0) - 3.0).abs() < 1e-9);
        // Giá vượt 110: lãi bị chặn ở 10 + 3 = 13
        assert!((q.lai_lo(150.0) - 13.0).abs() < 1e-9);
        assert!((q.lai_lo(1_000.0) - 13.0).abs() < 1e-9,
                "dù giá lên tới đâu cũng chỉ lãi 13 — đó là cái giá của phí thu được");
        // Giá sập: vẫn lỗ gần như toàn bộ
        assert!(q.lai_lo(50.0) < -45.0);
    }

    #[test]
    fn dieu_hau_sat_lai_khi_gia_nam_yen_va_lo_co_tran() {
        let d = dieu_hau_sat(95.0, 90.0, 105.0, 110.0, [1.0, 2.5, 2.5, 1.0]);
        let giua = d.lai_lo(100.0);
        assert!(giua > 0.0, "giá nằm giữa hai chân bán → có lãi, thực tế {:.2}", giua);
        let lo = d.lo_toi_da_trong_dai(0.0, 300.0, 0.5);
        assert!(lo > -10.0, "lỗ phải có trần, thực tế {:.2}", lo);
        assert!((d.lai_lo(10.0) - d.lai_lo(50.0)).abs() < 1e-9,
                "quá xa về phía dưới thì lỗ không tăng thêm");
        assert!((d.lai_lo(200.0) - d.lai_lo(500.0)).abs() < 1e-9,
                "quá xa về phía trên cũng vậy");
    }

    #[test]
    fn cau_phan_ban_co_lai_lo_nguoc_dau_voi_mua() {
        let mua = CauPhan { loai: LoaiCauPhan::QuyenMua, so_luong: 1.0,
                            gia_thuc_hien: 100.0, phi_quyen: 5.0 };
        let ban = CauPhan { so_luong: -1.0, ..mua };
        for s in [80.0f64, 100.0, 130.0] {
            assert!((mua.lai_lo(s) + ban.lai_lo(s)).abs() < 1e-12,
                    "mua và bán cùng hợp đồng phải triệt tiêu nhau");
        }
    }

    #[test]
    fn chien_luoc_rong_thi_khong_lai_khong_lo() {
        let c = ChienLuocQuyen { ten: "rỗng".into(), cau_phan: vec![] };
        assert_eq!(c.lai_lo(100.0), 0.0);
        assert_eq!(c.chi_phi_ban_dau(), 0.0);
        assert!(c.diem_hoa_von(0.0, 200.0, 1.0).is_empty());
    }

    #[test]
    fn tham_so_khong_hop_le_bi_phat_hien() {
        assert!(ts().hop_le());
        assert!(!ThamSoQuyen { gia_co_so: 0.0, ..ts() }.hop_le());
        assert!(!ThamSoQuyen { gia_thuc_hien: -1.0, ..ts() }.hop_le());
        assert!(!ThamSoQuyen { thoi_gian_nam: -0.1, ..ts() }.hop_le());
        assert!(!ThamSoQuyen { bien_dong: -0.2, ..ts() }.hop_le());
    }
}
```

---

## Bảng tra cứu lỗi biên dịch thường gặp

| Lỗi | Nguyên nhân trong chương này | Cách sửa |
|---|---|---|
| `NaN` trong `d₁` | `T = 0` hoặc `σ = 0` → chia cho 0 | Xử lý riêng trường hợp đáo hạn: giá = giá trị nội tại |
| Bài kiểm thử "giá ≥ nội tại" trượt | Quyền BÁN châu Âu sâu trong tiền **được phép** rẻ hơn | Cận đúng là `K·e^(−rT) − S` |
| Biến động ngụ ý không hội tụ | Vega ≈ 0 khi sâu trong/ngoài tiền | Chỉ đòi khôi phục chính xác gần giá thực hiện |
| Cân bằng quyền mua–bán lệch | Quên chiết khấu `K` | `K·e^(−rT)`, không phải `K` |
| `E0308: expected f64, found i32` | Truyền số ngày dạng nguyên | Đổi sang năm: `ngay as f64 / 365.0` |

---

## Tóm tắt chương & Bài tập rèn luyện

### 5 điểm cốt lõi

1. **Black-Scholes cho ngôn ngữ chung**, dù mọi giả định của nó đều sai trong thực tế.
2. **Cân bằng quyền mua–quyền bán là chênh lệch giá, không phải mô hình.** Vi phạm nó nghĩa là có bug.
3. **Quyền bán châu Âu sâu trong tiền có thể rẻ hơn giá trị nội tại** — chênh lệch chính là giá trị thực thi sớm.
4. **Vega ≈ 0 khi sâu trong/ngoài tiền**, nên biến động ngụ ý ở đó không khôi phục được — đó là giới hạn của bài toán, không phải của thuật toán.
5. **Nụ cười biến động là cách thị trường sửa mô hình** mà vẫn giữ ngôn ngữ của nó.

### Bài tập rèn luyện

**Bài 1.** Cài **cây nhị thức định giá quyền chọn Mỹ** và đo giá trị thực thi sớm.

<details>
<summary><b>Gợi ý</b></summary>

Black-Scholes chỉ định giá quyền chọn châu Âu. Với quyền chọn Mỹ, phải kiểm ở **mỗi nút** xem thực thi ngay có tốt hơn giữ tiếp không. Cây nhị thức Cox–Ross–Rubinstein làm được điều đó, và khi số bước tăng thì giá quyền chọn châu Âu hội tụ về Black-Scholes — một cách kiểm chứng chéo rất tốt.
</details>

<details>
<summary><b>Lời giải</b></summary>

```rust
pub fn cay_nhi_thuc(ts: &ThamSoQuyen, loai: LoaiQuyen, so_buoc: usize, kieu_my: bool) -> f64 {
    let dt = ts.thoi_gian_nam / so_buoc as f64;
    let u: f64 = (ts.bien_dong * dt.sqrt()).exp();       // hệ số lên
    let d: f64 = 1.0 / u;                                 // hệ số xuống
    let p = ((ts.lai_suat * dt).exp() - d) / (u - d);// xác suất trung hoà rủi ro
    let chiet_khau = (-ts.lai_suat * dt).exp();

    // Giá trị tại đáo hạn
    let mut gt: Vec<f64> = (0..=so_buoc).map(|i| {
        let s = ts.gia_co_so * u.powi(i as i32) * d.powi((so_buoc - i) as i32);
        match loai {
            LoaiQuyen::Mua => (s - ts.gia_thuc_hien).max(0.0),
            LoaiQuyen::Ban => (ts.gia_thuc_hien - s).max(0.0),
        }
    }).collect();

    // Lùi dần về hiện tại
    for buoc in (0..so_buoc).rev() {
        for i in 0..=buoc {
            let giu = chiet_khau * (p * gt[i + 1] + (1.0 - p) * gt[i]);
            gt[i] = if kieu_my {
                let s = ts.gia_co_so * u.powi(i as i32) * d.powi((buoc - i) as i32);
                let thuc_thi_ngay = match loai {
                    LoaiQuyen::Mua => (s - ts.gia_thuc_hien).max(0.0),
                    LoaiQuyen::Ban => (ts.gia_thuc_hien - s).max(0.0),
                };
                giu.max(thuc_thi_ngay)      // ĐÂY là điểm khác biệt của kiểu Mỹ
            } else { giu };
        }
    }
    gt[0]
}

/// Phần giá trị chỉ quyền chọn Mỹ mới có.
pub fn gia_tri_thuc_thi_som(ts: &ThamSoQuyen, loai: LoaiQuyen, so_buoc: usize) -> f64 {
    cay_nhi_thuc(ts, loai, so_buoc, true) - cay_nhi_thuc(ts, loai, so_buoc, false)
}
```

Với quyền **mua** trên cổ phiếu không trả cổ tức, `gia_tri_thuc_thi_som` bằng 0 — thực thi sớm không bao giờ tối ưu. Với quyền **bán**, nó dương và tăng theo độ sâu trong tiền, đúng bằng khoản chênh lệch đã nói ở đầu chương.
</details>

**Bài 2.** Cài **danh mục quyền chọn trung tính delta** và mô phỏng chi phí phòng vệ lại.

<details>
<summary><b>Gợi ý</b></summary>

Trung tính delta nghĩa là danh mục không nhạy với biến động **nhỏ** của giá cổ phiếu. Nhưng gamma làm delta trôi, nên bạn phải phòng vệ lại liên tục — và mỗi lần phòng vệ lại đều tốn phí. Đây là cốt lõi của giao dịch biến động: bạn kiếm tiền từ chênh lệch giữa biến động ngụ ý (bán ra) và biến động thực (chi phí phòng vệ).
</details>

<details>
<summary><b>Lời giải</b></summary>

```rust
pub struct DanhMucPhongVe {
    pub so_quyen: f64,
    pub loai: LoaiQuyen,
    pub co_phieu_nam_giu: f64,
    pub tien_mat: f64,
    pub phi_moi_don_vi: f64,
    pub tong_phi: f64,
    pub so_lan_phong_ve: usize,
}

impl DanhMucPhongVe {
    /// Đưa danh mục về trung tính delta tại giá hiện tại.
    pub fn phong_ve_lai(&mut self, ts: &ThamSoQuyen) {
        let g = tinh_greeks(ts, self.loai);
        let can_nam_giu = -self.so_quyen * g.delta;   // bán quyền → mua cổ phiếu
        let phai_giao_dich = can_nam_giu - self.co_phieu_nam_giu;

        if phai_giao_dich.abs() < 1e-9 { return; }
        let phi = phai_giao_dich.abs() * self.phi_moi_don_vi;
        self.tien_mat -= phai_giao_dich * ts.gia_co_so + phi;
        self.co_phieu_nam_giu = can_nam_giu;
        self.tong_phi += phi;
        self.so_lan_phong_ve += 1;
    }

    /// Phòng vệ theo NGƯỠNG: chỉ giao dịch khi delta trôi quá xa.
    /// Đánh đổi: ngưỡng lớn → ít phí hơn nhưng rủi ro còn lại nhiều hơn.
    pub fn phong_ve_theo_nguong(&mut self, ts: &ThamSoQuyen, nguong: f64) {
        let g = tinh_greeks(ts, self.loai);
        let delta_dm = self.so_quyen * g.delta + self.co_phieu_nam_giu;
        if delta_dm.abs() > nguong { self.phong_ve_lai(ts); }
    }

    pub fn gia_tri(&self, ts: &ThamSoQuyen) -> f64 {
        self.tien_mat
            + self.co_phieu_nam_giu * ts.gia_co_so
            + self.so_quyen * gia_black_scholes(ts, self.loai)
    }
}
```

`phong_ve_theo_nguong` thể hiện đánh đổi trung tâm: phòng vệ liên tục thì không còn rủi ro delta nhưng phí ăn hết lợi nhuận; phòng vệ thưa thì rẻ hơn nhưng chịu rủi ro gamma. Ngưỡng tối ưu tỉ lệ với **căn bậc ba** của phí giao dịch — kết quả kinh điển của Whalley và Wilmott.
</details>
