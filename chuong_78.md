# Chương 78: Thị trường blockchain — AMM, MEV & Chênh lệch giá CEX–DEX

## Giới thiệu & Mục tiêu học tập

Thị trường phi tập trung chơi theo luật khác hẳn thị trường truyền thống. Ở chương 74–77, lợi thế đến từ **tốc độ**. Ở đây, lợi thế đến từ **thứ tự** — và thứ tự thì mua được.

Ba khác biệt nền tảng:

| | Thị trường truyền thống | Thị trường blockchain |
|---|---|---|
| Cơ chế giá | Sổ lệnh giới hạn | Công thức toán (x·y = k) |
| Ưu tiên | Thời gian đến | Phí ưu tiên trả cho người xây khối |
| Ý định giao dịch | Riêng tư cho tới khi khớp | **Công khai** trong mempool |

Cái thứ ba là thứ thay đổi tất cả. Khi mọi người đều thấy ý định của bạn **trước khi** nó được thực thi, xuất hiện cả một ngành công nghiệp khai thác điều đó — gọi chung là **MEV** (Maximal Extractable Value).

---

## Hình tượng hóa đời sống

```
┌──────────────────────────────────────────────────────────────────────────────┐
│  AMM = MÁY BÁN HÀNG TỰ ĐỘNG THEO CÔNG THỨC                                  │
│                                                                              │
│   Bể: 1000 ETH × 2 000 000 USDC,  k = 2 000 000 000                         │
│                                                                              │
│   Mua 100 ETH:                                                              │
│     ETH còn lại 900  →  USDC cần có = k/900 = 2 222 222                     │
│     phải trả = 222 222 USDC  →  giá TB 2222/ETH                            │
│     giá niêm yết ban đầu chỉ 2000 → TRƯỢT GIÁ 11%!                         │
│                                                                              │
│   Đường cong x·y=k: mua càng nhiều, giá càng đắt — TỰ ĐỘNG.                 │
│   Không cần người tạo lập, không cần sổ lệnh. Chỉ cần công thức.            │
│                                                                              │
│  TỔN THẤT TẠM THỜI = CÁI GIÁ CỦA VIỆC LÀM MÁY BÁN HÀNG                      │
│                                                                              │
│   Bạn gửi 1 ETH + 2000 USDC. Giá ETH tăng gấp 4.                           │
│     Nếu chỉ GIỮ:        1 ETH (8000) + 2000 = 10 000                        │
│     Nếu để trong bể:    ~0,5 ETH + ~4000  =  8 000                          │
│     → mất 20%                                                               │
│                                                                              │
│   Vì sao? Bể tự động BÁN tài sản đang tăng giá cho người chênh lệch.        │
│   Bạn được nhận phí để bù — nhưng chỉ khi khối lượng đủ lớn.                │
│                                                                              │
│  KẸP LỆNH (sandwich) = KẺ TẤN CÔNG THẤY LỆNH BẠN TRƯỚC                     │
│                                                                              │
│     mempool công khai:  [ lệnh mua 10 ETH của bạn ]                         │
│                                    ↓ kẻ tấn công thấy                       │
│     khối được xây:  ┌────────────────────────────────────┐                 │
│                     │ 1. hắn MUA   (đẩy giá lên)         │                 │
│                     │ 2. bạn MUA   (giá đã xấu hơn)      │                 │
│                     │ 3. hắn BÁN   (thu lời)             │                 │
│                     └────────────────────────────────────┘                 │
│                                                                              │
│     Hắn trả phí ưu tiên cao hơn để được xếp trước. Hoàn toàn hợp lệ         │
│     theo luật giao thức. Phòng vệ DUY NHẤT: đặt SỐ NHẬN TỐI THIỂU chặt.     │
│                                                                              │
│  CHÊNH LỆCH CEX–DEX                                                         │
│     Binance: 2000    Uniswap: 2050                                          │
│     → mua trên Binance, bán trên Uniswap                                    │
│     NHƯNG: bán bao nhiêu thì tối ưu? Bán nhiều → tự đẩy giá DEX xuống.      │
│     → Bài toán tối ưu một biến, giải bằng tìm kiếm tam phân.               │
└──────────────────────────────────────────────────────────────────────────────┘
```

---

## Khái niệm & Cơ chế kỹ thuật chuyên sâu

### 1. Vì sao x·y = k mà không phải công thức khác?

Đường cong tích không đổi có ba tính chất khiến nó thắng:

- **Không bao giờ cạn.** Khi `x → 0` thì giá `→ ∞`. Không thể mua hết token cuối cùng ở giá hữu hạn. Bể luôn có thanh khoản, dù giá có thể rất xấu.
- **Không cần trạng thái.** Giá chỉ phụ thuộc vào hai số dư hiện tại. Không cần lưu sổ lệnh, không cần cập nhật gì — tiết kiệm gas, và gas là chi phí thật.
- **Tự cân bằng.** Chênh lệch giá tự động kéo giá bể về giá thị trường, không cần ai điều phối.

Nhược điểm cũng rõ: **kém hiệu quả vốn**. Phần lớn thanh khoản nằm ở những mức giá không bao giờ được chạm tới. Uniswap V3 sửa điều này bằng "thanh khoản tập trung" — cho phép nhà cung cấp chọn khoảng giá — nhưng đổi lại là phải quản lý vị thế chủ động.

### 2. Công thức tổn thất tạm thời

```
TTTT(r) = 2√r / (1 + r) − 1
```

với `r` là tỉ lệ thay đổi giá. Vài giá trị đáng nhớ:

| Giá đổi | Tổn thất |
|---|---|
| ×1,25 | −0,6% |
| ×1,5 | −2,0% |
| ×2 | −5,7% |
| ×4 | −20,0% |
| ×5 | −25,5% |

Điểm quan trọng: công thức **đối xứng theo `r` và `1/r`` — giá tăng gấp đôi hay giảm một nửa đều mất 5,7%. Và tên gọi "tạm thời" gây hiểu nhầm: tổn thất chỉ tạm thời nếu giá **quay lại** mức cũ. Nếu bạn rút vốn khi giá đã đổi, tổn thất là **vĩnh viễn**.

### 3. Kẹp lệnh và phòng vệ thật sự

Kẹp lệnh không phải "hack". Nó tuân thủ mọi luật của giao thức — kẻ tấn công chỉ trả phí ưu tiên cao hơn để được xếp trước và sau bạn.

Phòng vệ **duy nhất có hiệu lực** là đặt số nhận tối thiểu (`amountOutMin`) chặt. Nếu bạn đặt dung sai trượt giá 0,5%, kẻ tấn công chỉ moi được tối đa 0,5%. Nếu bạn đặt 50% (hoặc, tệ hơn, đặt 0 — tức là "chấp nhận mọi giá"), bạn đang mời họ lấy sạch.

Các phòng vệ khác đều là giảm nhẹ, không phải chặn đứng:
- **Mempool riêng tư** (Flashbots Protect): giao dịch không hiện công khai, nhưng bạn phải tin nhà cung cấp.
- **Đấu giá theo lô** (CoW Swap): mọi lệnh trong một lô nhận cùng giá, nên không có gì để moi từ thứ tự.
- **Commit–reveal**: cam kết trước, tiết lộ sau — tốn hai giao dịch.

### 4. Chênh lệch CEX–DEX: bài toán tối ưu, không phải tín hiệu

Thấy Binance 2000 và Uniswap 2050 chưa phải là cơ hội. Câu hỏi thật là: **giao dịch bao nhiêu?**

Bán quá ít → bỏ lỡ lợi nhuận. Bán quá nhiều → tự đẩy giá DEX xuống dưới điểm hoà vốn. Lợi nhuận theo khối lượng là hàm **lõm có đúng một cực đại**, nên tìm kiếm tam phân hội tụ nhanh và chắc chắn.

Đừng quên trừ đủ chi phí: phí bể (thường 0,3%), phí gas (cố định, nên với lệnh nhỏ nó nuốt hết lợi nhuận), và phí của sàn tập trung. Rất nhiều "cơ hội" biến mất sau khi trừ gas.

---

## Mã nguồn minh họa thực chiến

Chạy bằng `cargo run -p ch78`, kiểm thử bằng `cargo test -p ch78`.

```rust
#![allow(dead_code)]
//! Chương 78 — Thị trường blockchain: bể thanh khoản tích không đổi, trượt giá,
//! tổn thất tạm thời, tấn công kẹp (sandwich), và arbitrage giữa sàn tập trung
//! với sàn phi tập trung.
//!
//! Khác biệt cốt lõi so với thị trường truyền thống (Chương 75–77): ở đây
//! **mọi giao dịch đều công khai TRƯỚC khi được thực thi**. Ai cũng đọc được
//! hàng chờ, và ai trả phí cao hơn thì được xếp trước. Đó là mảnh đất của MEV.
//!
//! ⚠️ Đây là tài liệu KỸ THUẬT nhằm giúp người đọc TỰ BẢO VỆ và hiểu rủi ro,
//! không phải hướng dẫn khai thác người dùng khác.

// ============================================================================
// 1. BỂ THANH KHOẢN TÍCH KHÔNG ĐỔI
// ============================================================================
// Toàn bộ Uniswap v2 gói gọn trong một bất biến: x · y = k.
// Không sổ lệnh, không người khớp lệnh, không ai phải chờ đối tác.
// Giá được suy ra từ tỉ lệ dự trữ, và tự động điều chỉnh sau mỗi giao dịch.

pub type Quantity = u128;

#[derive(Debug, Clone, Copy, PartialEq)]
pub struct Pool {
    pub reserve_x: Quantity,
    pub reserve_y: Quantity,
    /// Phí tính theo phần vạn: 30 = 0,30%
    pub fee_bps: u32,
}

#[derive(Debug, PartialEq)]
pub enum SwapError {
    ZeroInput,
    EmptyPool,
    ThanhKhoanKhongDu,
    /// Người dùng đặt sàn nhận tối thiểu, mà kết quả thấp hơn → huỷ giao dịch.
    TruotGiaVuotChoPhep { nhan_duoc: Quantity, min: Quantity },
}

impl Pool {
    pub fn new(x: Quantity, y: Quantity, fee_bps: u32) -> Self {
        Pool { reserve_x: x, reserve_y: y, fee_bps }
    }

    /// Hằng số bất biến. Nó chỉ được TĂNG (nhờ phí), không bao giờ giảm.
    pub fn k(&self) -> u128 { self.reserve_x * self.reserve_y }

    /// Giá hiện thời của X tính theo Y, dạng số thực (chỉ để hiển thị).
    pub fn price_x(&self) -> f64 {
        if self.reserve_x == 0 { return 0.0; }
        self.reserve_y as f64 / self.reserve_x as f64
    }

    /// Tính lượng Y nhận được khi đưa vào `x_in`, KHÔNG thay đổi bể.
    ///
    /// Công thức: dy = (y · dx') / (x + dx') với dx' = dx · (1 − phí).
    /// Toàn bộ tính bằng số nguyên — tiền không bao giờ dùng dấu phẩy động.
    pub fn try_swap_x_for_y(&self, x_in: Quantity) -> Result<Quantity, SwapError> {
        if x_in == 0 { return Err(SwapError::ZeroInput); }
        if self.reserve_x == 0 || self.reserve_y == 0 { return Err(SwapError::EmptyPool); }
        let next_phi = x_in * (10_000 - self.fee_bps as u128);
        let tu = self.reserve_y * next_phi;
        let mau = self.reserve_x * 10_000 + next_phi;
        let ra = tu / mau;
        if ra == 0 || ra >= self.reserve_y { return Err(SwapError::ThanhKhoanKhongDu); }
        Ok(ra)
    }

    /// Thực hiện hoán đổi, có kiểm tra sàn nhận tối thiểu.
    /// `min_y` chính là "bảo vệ trượt giá" mà ví hiển thị cho bạn.
    pub fn swap_x_for_y(&mut self, x_in: Quantity, min_y: Quantity)
        -> Result<Quantity, SwapError>
    {
        let ra = self.try_swap_x_for_y(x_in)?;
        if ra < min_y {
            return Err(SwapError::TruotGiaVuotChoPhep { nhan_duoc: ra, min: min_y });
        }
        self.reserve_x += x_in;
        self.reserve_y -= ra;
        Ok(ra)
    }

    pub fn try_swap_y_for_x(&self, vao_y: Quantity) -> Result<Quantity, SwapError> {
        let dao = Pool { reserve_x: self.reserve_y, reserve_y: self.reserve_x,
                                 fee_bps: self.fee_bps };
        dao.try_swap_x_for_y(vao_y)
    }

    pub fn swap_y_for_x(&mut self, vao_y: Quantity, toi_thieu_x: Quantity)
        -> Result<Quantity, SwapError>
    {
        let ra = self.try_swap_y_for_x(vao_y)?;
        if ra < toi_thieu_x {
            return Err(SwapError::TruotGiaVuotChoPhep { nhan_duoc: ra, min: toi_thieu_x });
        }
        self.reserve_y += vao_y;
        self.reserve_x -= ra;
        Ok(ra)
    }

    /// Trượt giá: chênh lệch giữa giá thực nhận và giá niêm yết trước giao dịch.
    /// Đây KHÔNG phải phí — nó là hệ quả toán học của đường cong x·y = k,
    /// và nó lớn dần theo quy mô giao dịch so với bể.
    pub fn slippage(&self, x_in: Quantity) -> Option<f64> {
        let ra = self.try_swap_x_for_y(x_in).ok()?;
        let gia_niem_yet = self.price_x();
        let exec_price = ra as f64 / x_in as f64;
        Some((gia_niem_yet - exec_price) / gia_niem_yet)
    }
}

// ============================================================================
// 2. TỔN THẤT TẠM THỜI — cái giá của việc làm nhà cung cấp thanh khoản
// ============================================================================

/// Khi giá đổi theo hệ số `r`, giá trị phần vốn góp so với việc CHỈ NẮM GIỮ là:
///
///     2·√r / (1 + r) − 1
///
/// Luôn ≤ 0, và bằng 0 chỉ khi r = 1 (giá không đổi). Nghĩa là: giá càng
/// biến động, người góp vốn càng thiệt so với người chỉ ngồi im — và phí thu
/// được phải bù nổi khoản đó thì góp vốn mới có lãi.
///
/// Chữ "tạm thời" gây hiểu lầm: nó chỉ tạm thời nếu giá QUAY VỀ mức cũ.
/// Không quay về thì nó vĩnh viễn.
pub fn impermanent_loss(ty_le_gia: f64) -> f64 {
    if ty_le_gia <= 0.0 { return 0.0; }
    2.0 * ty_le_gia.sqrt() / (1.0 + ty_le_gia) - 1.0
}

// ============================================================================
// 3. HÀNG CHỜ CÔNG KHAI & TẤN CÔNG KẸP
// ============================================================================
// Trên blockchain, giao dịch nằm trong hàng chờ CÔNG KHAI trước khi vào khối,
// và người xây khối sắp xếp theo phí ưu tiên. Ai trả cao hơn được xếp trước.
// Hệ quả: bất kỳ ai cũng thấy trước bạn định làm gì, và chen lên trước được.

#[derive(Debug, Clone, PartialEq)]
pub struct TradeWait {
    pub sender: String,
    pub x_in: Quantity,
    pub min_y: Quantity,
    /// Phí ưu tiên — con số quyết định thứ tự trong khối.
    pub priority_fee: u64,
}

/// Người xây khối sắp xếp theo phí ưu tiên GIẢM DẦN. Đây là toàn bộ cơ chế
/// khiến MEV tồn tại: thứ tự không theo thời gian tới, mà theo số tiền trả.
pub fn sort_arrange_block(mut cho: Vec<TradeWait>) -> Vec<TradeWait> {
    // `sort_by` của Rust là sắp xếp ỔN ĐỊNH → phí bằng nhau thì giữ nguyên
    // thứ tự, nên kết quả tất định và kiểm thử được.
    cho.sort_by(|a, b| b.priority_fee.cmp(&a.priority_fee));
    cho
}

#[derive(Debug, PartialEq)]
pub struct KetQuaKep {
    /// Nạn nhân nhận được bao nhiêu khi KHÔNG bị kẹp.
    pub receive_if_not_sandwiched: Quantity,
    /// Nạn nhân nhận được bao nhiêu khi BỊ kẹp.
    pub receive_when_sandwiched: Quantity,
    pub ke_attack_lai: i128,
    /// Giao dịch của nạn nhân có bị chặn nhờ sàn nhận tối thiểu không.
    pub blocked_by_guard: bool,
}

/// Mô phỏng một cú kẹp để thấy **vì sao phải đặt sàn nhận tối thiểu chặt**.
///
/// Kịch bản: kẻ tấn công thấy giao dịch của nạn nhân trong hàng chờ, trả phí
/// cao hơn để mua TRƯỚC (đẩy giá lên), để nạn nhân mua ở giá xấu, rồi bán
/// NGAY SAU đó ăn chênh lệch.
pub fn simulate_sandwich(be: &Pool, nan_nhan: &TradeWait, von_tan_cong: Quantity)
    -> KetQuaKep
{
    // (a) Nếu không ai chen ngang
    let clean = be.try_swap_x_for_y(nan_nhan.x_in).unwrap_or(0);

    // (b) Có kẻ chen ngang, mua trước để đẩy giá
    let mut b = *be;
    let prev_out = b.swap_x_for_y(von_tan_cong, 0).unwrap_or(0);

    let receive_when_sandwiched = b.try_swap_x_for_y(nan_nhan.x_in).unwrap_or(0);
    // ĐÂY là chỗ sàn nhận tối thiểu cứu nạn nhân: giao dịch bị huỷ, không mất vốn
    let is_block = receive_when_sandwiched < nan_nhan.min_y;
    if !is_block {
        let _ = b.swap_x_for_y(nan_nhan.x_in, nan_nhan.min_y);
    }

    // (c) Kẻ tấn công bán lại phần vừa mua
    let thu_ve = if is_block { 0 } else { b.try_swap_y_for_x(prev_out).unwrap_or(0) };
    let lai = if is_block { 0 } else { thu_ve as i128 - von_tan_cong as i128 };

    KetQuaKep {
        receive_if_not_sandwiched: clean,
        receive_when_sandwiched: if is_block { 0 } else { receive_when_sandwiched },
        ke_attack_lai: lai,
        blocked_by_guard: is_block,
    }
}

/// Tính sàn nhận tối thiểu từ mức trượt giá chấp nhận được (phần vạn).
/// Đặt 5% "cho chắc ăn" chính là mời kẻ tấn công lấy đúng 5% đó.
pub fn min_venue_recv(amount_in: Quantity, cho_phep_phan_van: u32) -> Quantity {
    amount_in * (10_000 - cho_phep_phan_van as u128) / 10_000
}

// ============================================================================
// 4. ARBITRAGE GIỮA SÀN TẬP TRUNG VÀ SÀN PHI TẬP TRUNG
// ============================================================================

#[derive(Debug, PartialEq)]
pub struct ArbOpportunity {
    pub has_has_hoi: bool,
    pub quantity_toi_uu: Quantity,
    pub estimated_return: i128,
    pub dex_price_before: f64,
    pub dex_price_after: f64,
}

/// Tìm khối lượng hoán đổi tối ưu để kéo giá sàn phi tập trung về sát giá sàn
/// tập trung, và tính lãi ước tính sau phí.
///
/// Dùng tìm kiếm tam phân trên hàm lãi thay vì giải công thức đóng: đường cong
/// có phí và có làm tròn số nguyên, nên công thức đóng lệch với thực tế. Tìm
/// kiếm trên chính hàm sẽ thực thi thì luôn khớp với những gì xảy ra trên chuỗi.
pub fn find_arb(be: &Pool, gia_cex: f64, von_toi_da: Quantity) -> ArbOpportunity {
    let prev_price = be.price_x();
    let no_has = ArbOpportunity { has_has_hoi: false, quantity_toi_uu: 0, estimated_return: 0,
                              dex_price_before: prev_price, dex_price_after: prev_price };
    // Chỉ xét chiều: mua X trên DEX (đưa Y vào) khi X trên DEX RẺ hơn CEX
    if prev_price >= gia_cex || von_toi_da == 0 { return no_has; }

    let lai_when = |vao_y: Quantity| -> i128 {
        match be.try_swap_y_for_x(vao_y) {
            // Nhận `ra_x` đơn vị X, bán trên CEX được ra_x · gia_cex đơn vị Y
            Ok(ra_x) => (ra_x as f64 * gia_cex) as i128 - vao_y as i128,
            Err(_) => i128::MIN,
        }
    };

    // Hàm lãi lõm theo khối lượng → tìm kiếm tam phân
    let (mut lo, mut hi) = (1u128, von_toi_da);
    for _ in 0..200 {
        if hi <= lo + 2 { break; }
        let m1 = lo + (hi - lo) / 3;
        let m2 = hi - (hi - lo) / 3;
        if lai_when(m1) < lai_when(m2) { lo = m1 + 1; } else { hi = m2 - 1; }
    }
    let mut best = (lo, lai_when(lo));
    let mut v = lo;
    while v <= hi && v <= lo + 8 {
        let l = lai_when(v);
        if l > best.1 { best = (v, l); }
        v += 1;
    }

    if best.1 <= 0 { return no_has; }
    let mut next = *be;
    let _ = next.swap_y_for_x(best.0, 0);
    ArbOpportunity {
        has_has_hoi: true,
        quantity_toi_uu: best.0,
        estimated_return: best.1,
        dex_price_before: prev_price,
        dex_price_after: next.price_x(),
    }
}

fn main() {
    println!("═══════════════════════════════════════════════════════════");
    println!("   THỊ TRƯỜNG BLOCKCHAIN: AMM · TRƯỢT GIÁ · MEV            ");
    println!("═══════════════════════════════════════════════════════════");

    println!("\n1. BỂ THANH KHOẢN TÍCH KHÔNG ĐỔI");
    let be = Pool::new(1_000_000, 2_000_000_000, 30);
    println!("   Dự trữ: {} X · {} Y", be.reserve_x, be.reserve_y);
    println!("   Giá niêm yết: 1 X = {:.2} Y · k = {}", be.price_x(), be.k());

    println!("\n2. TRƯỢT GIÁ TĂNG THEO QUY MÔ GIAO DỊCH");
    println!("   {:>10} {:>16} {:>14} {:>10}", "đưa vào X", "nhận được Y", "giá thực", "trượt giá");
    for amount_in in [100u128, 1_000, 10_000, 100_000, 500_000] {
        let ra = be.try_swap_x_for_y(amount_in).unwrap();
        println!("   {:>10} {:>16} {:>14.2} {:>9.2}%",
                 amount_in, ra, ra as f64 / amount_in as f64, be.slippage(amount_in).unwrap() * 100.0);
    }
    println!("   → Giao dịch bằng 50% bể mất tới {:.0}% giá trị. Đây KHÔNG phải phí,",
             be.slippage(500_000).unwrap() * 100.0);
    println!("     mà là hình dạng của chính đường cong x·y = k.");

    println!("\n3. PHÍ LÀM HẰNG SỐ k LỚN DẦN — đó là lãi của người góp vốn");
    let mut b2 = be;
    let k_dau = b2.k();
    for _ in 0..10 { b2.swap_x_for_y(10_000, 0).unwrap(); }
    println!("   k trước: {} · sau 10 lần hoán đổi: {}", k_dau, b2.k());
    println!("   k tăng {:.4}% — phần đó thuộc về người góp vốn.",
             (b2.k() as f64 / k_dau as f64 - 1.0) * 100.0);

    println!("\n4. TỔN THẤT TẠM THỜI");
    println!("   {:>14} {:>18}", "giá đổi", "so với chỉ nắm giữ");
    for r in [0.25f64, 0.5, 0.8, 1.0, 1.25, 2.0, 4.0, 10.0] {
        println!("   {:>13.2}x {:>17.2}%", r, impermanent_loss(r) * 100.0);
    }
    println!("   → Luôn ≤ 0, chỉ bằng 0 khi giá không đổi. Phí thu được phải bù nổi");
    println!("     khoản này thì góp vốn mới thật sự có lãi.");

    println!("\n5. HÀNG CHỜ CÔNG KHAI — phí quyết định thứ tự, không phải thời gian tới");
    let cho = vec![
        TradeWait { sender: "nguoi-dung-thuong".into(), x_in: 50_000,
                      min_y: 0, priority_fee: 2 },
        TradeWait { sender: "bot-chen-truoc".into(), x_in: 30_000,
                      min_y: 0, priority_fee: 500 },
        TradeWait { sender: "nguoi-kien-nhan".into(), x_in: 1_000,
                      min_y: 0, priority_fee: 1 },
    ];
    for (i, g) in sort_arrange_block(cho).iter().enumerate() {
        println!("   #{} {:<20} phí ưu tiên {}", i + 1, g.sender, g.priority_fee);
    }

    println!("\n6. TẤN CÔNG KẸP — và cách sàn nhận tối thiểu cứu bạn");
    let amount_in = be.try_swap_x_for_y(50_000).unwrap();
    println!("   Nạn nhân định đổi 50 000 X, dự kiến nhận {} Y", amount_in);
    for wait_op in [5_000u32, 1_000, 100, 50] {
        let sn = min_venue_recv(amount_in, wait_op);
        let nn = TradeWait { sender: "nan-nhan".into(), x_in: 50_000,
                               min_y: sn, priority_fee: 1 };
        let kq = simulate_sandwich(&be, &nn, 200_000);
        if kq.blocked_by_guard {
            println!("   cho phép trượt {:>4.1}% → GIAO DỊCH BỊ HUỶ, nạn nhân không mất vốn",
                     wait_op as f64 / 100.0);
        } else {
            let mat = kq.receive_if_not_sandwiched - kq.receive_when_sandwiched;
            println!("   cho phép trượt {:>4.1}% → nạn nhân mất {:>8} Y · kẻ tấn công lãi {:>8}",
                     wait_op as f64 / 100.0, mat, kq.ke_attack_lai);
        }
    }
    println!("   → Đặt 5% \"cho chắc ăn\" chính là công khai mời người khác lấy 5% đó.");

    println!("\n7. ARBITRAGE CEX ↔ DEX");
    let lech = Pool::new(1_000_000, 1_900_000_000, 30); // DEX rẻ hơn
    let gia_cex = 2_000.0;
    println!("   Giá DEX {:.2} · giá CEX {:.2} → lệch {:.2}%",
             lech.price_x(), gia_cex, (gia_cex / lech.price_x() - 1.0) * 100.0);
    let ch = find_arb(&lech, gia_cex, 500_000_000);
    if ch.has_has_hoi {
        println!("   Khối lượng tối ưu: {} Y → lãi ước tính {} Y",
                 ch.quantity_toi_uu, ch.estimated_return);
        println!("   Giá DEX sau giao dịch: {:.2} (đã kéo về gần CEX)", ch.dex_price_after);
    }
    println!("   → Chính đội arbitrage giữ cho giá DEX bám sát thị trường.");
    println!("     Họ không làm từ thiện — họ được trả công bằng khoảng lệch đó.");

    println!("\n═══════════════════════════════════════════════════════════");
    println!("   HÀNG CHỜ CÔNG KHAI = MỌI Ý ĐỊNH ĐỀU BỊ ĐỌC TRƯỚC         ");
    println!("═══════════════════════════════════════════════════════════");
}

#[cfg(test)]
mod tests {
    use super::*;

    fn sample_pool() -> Pool { Pool::new(1_000_000, 2_000_000_000, 30) }

    // ---------- Bể thanh khoản ----------
    #[test]
    fn price_derives_from_the_reserve_ratio() {
        let b = sample_pool();
        assert!((b.price_x() - 2_000.0).abs() < 1e-9);
        assert_eq!(Pool::new(0, 100, 30).price_x(), 0.0, "bể rỗng không chia cho 0");
    }

    #[test]
    fn swaps_grow_k_never_shrink_it() {
        // Bất biến sống còn của AMM: phí làm k lớn dần, và đó chính là
        // phần lãi thuộc về người góp vốn.
        let mut b = sample_pool();
        let mut k = b.k();
        for _ in 0..50 {
            b.swap_x_for_y(10_000, 0).unwrap();
            let k_moi = b.k();
            assert!(k_moi >= k, "k giảm từ {} xuống {} — bể bị rút ruột", k, k_moi);
            k = k_moi;
        }
        assert!(b.k() > sample_pool().k(), "sau 50 lần hoán đổi k phải lớn hơn hẳn");
    }

    #[test]
    fn with_zero_fee_k_is_nearly_constant() {
        let mut b = Pool::new(1_000_000, 2_000_000_000, 0);
        let k_dau = b.k();
        b.swap_x_for_y(10_000, 0).unwrap();
        // Chỉ lệch do làm tròn số nguyên, không phải do phí
        let lech = (b.k() as f64 / k_dau as f64 - 1.0).abs();
        assert!(lech < 1e-4, "không phí thì k gần như đứng yên, lệch {:.6}", lech);
    }

    #[test]
    fn more_input_returns_more_but_less_efficiently() {
        let b = sample_pool();
        let mut prev_effective_price = f64::MAX;
        let mut prev_out = 0u128;
        for amount_in in [100u128, 1_000, 10_000, 100_000] {
            let ra = b.try_swap_x_for_y(amount_in).unwrap();
            assert!(ra > prev_out, "đưa vào nhiều hơn phải nhận nhiều hơn");
            let exec_price = ra as f64 / amount_in as f64;
            assert!(exec_price < prev_effective_price, "nhưng giá mỗi đơn vị phải TỆ dần");
            prev_out = ra;
            prev_effective_price = exec_price;
        }
    }

    #[test]
    fn slippage_always_duong_and_up_theo_quy_open() {
        let b = sample_pool();
        let mut prev = 0.0;
        for amount_in in [100u128, 1_000, 10_000, 100_000, 500_000] {
            let t = b.slippage(amount_in).unwrap();
            assert!(t > 0.0, "trượt giá luôn dương — bạn luôn nhận ít hơn giá niêm yết");
            assert!(t > prev, "và tăng dần theo quy mô");
            prev = t;
        }
        assert!(b.slippage(500_000).unwrap() > 0.3,
                "giao dịch bằng nửa bể phải mất hơn 30%");
    }

    #[test]
    fn the_pool_can_never_be_drained() {
        // Bất biến toán học của x·y = k: không lượng đầu vào hữu hạn nào lấy
        // hết được phía bên kia. Đây là điều khiến AMM không thể bị "vét sạch".
        let b = sample_pool();
        for amount_in in [1_000_000u128, 10_000_000, 1_000_000_000] {
            match b.try_swap_x_for_y(amount_in) {
                Ok(ra) => assert!(ra < b.reserve_y, "nhận {} mà bể chỉ có {}", ra, b.reserve_y),
                Err(e) => assert_eq!(e, SwapError::ThanhKhoanKhongDu),
            }
        }
    }

    #[test]
    fn swap_no_hop_le_is_reject() {
        let b = sample_pool();
        assert_eq!(b.try_swap_x_for_y(0), Err(SwapError::ZeroInput));
        assert_eq!(Pool::new(0, 0, 30).try_swap_x_for_y(100),
                   Err(SwapError::EmptyPool));
    }

    #[test]
    fn reserve_update_use_next_swap() {
        let mut b = sample_pool();
        let ra = b.swap_x_for_y(10_000, 0).unwrap();
        assert_eq!(b.reserve_x, 1_010_000, "X vào bể");
        assert_eq!(b.reserve_y, 2_000_000_000 - ra, "Y ra khỏi bể");
    }

    #[test]
    fn a_round_trip_loses_to_double_fees() {
        let mut b = sample_pool();
        let y = b.swap_x_for_y(10_000, 0).unwrap();
        assert!(y > 0);
        let x = b.swap_y_for_x(y, 0).unwrap();
        assert!(x > 0);
        assert!(x < 10_000, "đổi đi rồi đổi lại phải LỖ, nhận về {} thay vì 10 000", x);
    }

    #[test]
    fn min_out_blocks_a_bad_trade() {
        let mut b = sample_pool();
        let amount_in = b.try_swap_x_for_y(10_000).unwrap();
        // Đòi nhiều hơn mức có thể → phải bị chặn, và bể KHÔNG được đổi
        let prev = b;
        let e = b.swap_x_for_y(10_000, amount_in + 1).unwrap_err();
        assert!(matches!(e, SwapError::TruotGiaVuotChoPhep { .. }));
        assert_eq!(b, prev, "giao dịch hỏng phải KHÔNG để lại thay đổi nào");
    }

    #[test]
    fn tinh_venue_recv_min_use() {
        assert_eq!(min_venue_recv(1_000_000, 50), 995_000, "0,5%");
        assert_eq!(min_venue_recv(1_000_000, 100), 990_000, "1%");
        assert_eq!(min_venue_recv(1_000_000, 5_000), 500_000, "50% là quá lỏng");
        assert_eq!(min_venue_recv(1_000_000, 0), 1_000_000);
    }

    // ---------- Tổn thất tạm thời ----------
    #[test]
    fn ton_true_table_no_when_price_no_swap() {
        assert!(impermanent_loss(1.0).abs() < 1e-12);
    }

    #[test]
    fn ton_true_always_no_duong() {
        for r in [0.01f64, 0.1, 0.5, 0.9, 1.0, 1.1, 2.0, 5.0, 100.0] {
            assert!(impermanent_loss(r) <= 1e-12,
                    "r={} cho {} — không bao giờ được dương", r, impermanent_loss(r));
        }
    }

    #[test]
    fn impermanent_loss_is_symmetric_under_inversion() {
        // Giá tăng gấp đôi hay giảm một nửa đều thiệt như nhau.
        for r in [2.0f64, 4.0, 10.0] {
            let a = impermanent_loss(r);
            let b = impermanent_loss(1.0 / r);
            assert!((a - b).abs() < 1e-12, "r={}: {} so với {}", r, a, b);
        }
    }

    #[test]
    fn impermanent_loss_grows_with_price_divergence() {
        let mut prev = 0.0;
        for r in [1.1f64, 1.5, 2.0, 4.0, 10.0] {
            let t = impermanent_loss(r);
            assert!(t < prev, "biến động mạnh hơn phải thiệt hơn");
            prev = t;
        }
        // Con số hay được trích dẫn: giá gấp đôi → thiệt khoảng 5,7%
        assert!((impermanent_loss(2.0) + 0.0572).abs() < 0.001);
        assert!((impermanent_loss(4.0) + 0.20).abs() < 0.001, "gấp 4 → thiệt 20%");
    }

    #[test]
    fn impermanent_loss_handles_bad_input() {
        assert_eq!(impermanent_loss(0.0), 0.0);
        assert_eq!(impermanent_loss(-1.0), 0.0);
    }

    // ---------- Hàng chờ & MEV ----------
    #[test]
    fn block_sort_arrange_theo_phi_uu_tien_down_derive() {
        let cho = vec![
            TradeWait { sender: "a".into(), x_in: 1, min_y: 0, priority_fee: 2 },
            TradeWait { sender: "b".into(), x_in: 1, min_y: 0, priority_fee: 500 },
            TradeWait { sender: "c".into(), x_in: 1, min_y: 0, priority_fee: 1 },
        ];
        let sap = sort_arrange_block(cho);
        assert_eq!(sap.iter().map(|g| g.sender.as_str()).collect::<Vec<_>>(),
                   vec!["b", "a", "c"], "trả nhiều nhất được xếp đầu");
        for w in sap.windows(2) {
            assert!(w[0].priority_fee >= w[1].priority_fee);
        }
    }

    #[test]
    fn ordering_is_stable_on_equal_fees() {
        let cho: Vec<TradeWait> = (0..5).map(|i| TradeWait {
            sender: format!("n{}", i), x_in: 1, min_y: 0, priority_fee: 10,
        }).collect();
        let sap = sort_arrange_block(cho);
        assert_eq!(sap.iter().map(|g| g.sender.clone()).collect::<Vec<_>>(),
                   vec!["n0", "n1", "n2", "n3", "n4"], "phí bằng nhau thì giữ nguyên thứ tự");
    }

    #[test]
    fn no_min_out_means_the_sandwich_takes_your_money() {
        // `min_y = 0` nghĩa là "nhận bao nhiêu cũng được" — lời mời công khai.
        let b = sample_pool();
        let nn = TradeWait { sender: "nan-nhan".into(), x_in: 50_000,
                               min_y: 0, priority_fee: 1 };
        let kq = simulate_sandwich(&b, &nn, 200_000);
        assert!(!kq.blocked_by_guard, "không có bảo vệ thì không gì chặn được");
        assert!(kq.receive_when_sandwiched < kq.receive_if_not_sandwiched,
                "bị kẹp thì nhận ít hơn: {} so với {}",
                kq.receive_when_sandwiched, kq.receive_if_not_sandwiched);
        assert!(kq.ke_attack_lai > 0, "và kẻ tấn công có lãi");
    }

    #[test]
    fn a_tight_min_out_reverts_instead_of_being_exploited() {
        // Bị huỷ giao dịch là KẾT QUẢ TỐT: bạn chỉ mất phí gas, không mất vốn.
        let b = sample_pool();
        let amount_in = b.try_swap_x_for_y(50_000).unwrap();
        let nn = TradeWait { sender: "can-than".into(), x_in: 50_000,
                               min_y: min_venue_recv(amount_in, 50), // 0,5%
                               priority_fee: 1 };
        let kq = simulate_sandwich(&b, &nn, 200_000);
        assert!(kq.blocked_by_guard, "sàn chặt phải chặn được cú kẹp");
        assert_eq!(kq.ke_attack_lai, 0, "kẻ tấn công không ăn được gì");
    }

    #[test]
    fn a_looser_min_out_means_bigger_losses() {
        let b = sample_pool();
        let amount_in = b.try_swap_x_for_y(50_000).unwrap();
        let mut thiet_hai_truoc = 0u128;
        // Đi từ chặt tới lỏng
        for wait_op in [50u32, 100, 500, 1_000, 5_000] {
            let nn = TradeWait { sender: "n".into(), x_in: 50_000,
                                   min_y: min_venue_recv(amount_in, wait_op),
                                   priority_fee: 1 };
            let kq = simulate_sandwich(&b, &nn, 200_000);
            if !kq.blocked_by_guard {
                let thiet = kq.receive_if_not_sandwiched - kq.receive_when_sandwiched;
                assert!(thiet >= thiet_hai_truoc,
                        "nới sàn nhận thì thiệt hại không được giảm");
                thiet_hai_truoc = thiet;
            }
        }
        assert!(thiet_hai_truoc > 0, "phải có ít nhất một mức bị bóc lột");
    }

    #[test]
    fn deeper_pools_are_harder_to_sandwich() {
        // Thanh khoản dày là biện pháp phòng vệ tự nhiên: cùng một cú tấn công
        // đẩy giá được ít hơn hẳn.
        let nong = Pool::new(100_000, 200_000_000, 30);
        let next = Pool::new(10_000_000, 20_000_000_000, 30);
        let thiet = |b: &Pool| {
            let nn = TradeWait { sender: "n".into(), x_in: 10_000,
                                   min_y: 0, priority_fee: 1 };
            let kq = simulate_sandwich(b, &nn, 50_000);
            (kq.receive_if_not_sandwiched - kq.receive_when_sandwiched) as f64
                / kq.receive_if_not_sandwiched as f64
        };
        assert!(thiet(&next) < thiet(&nong),
                "bể sâu thiệt {:.4} phải nhỏ hơn bể nông {:.4}", thiet(&next), thiet(&nong));
    }

    // ---------- Arbitrage CEX-DEX ----------
    #[test]
    fn no_report_has_hoi_when_price_da_table_each() {
        let b = sample_pool(); // giá 2000
        let ch = find_arb(&b, 2_000.0, 1_000_000_000);
        assert!(!ch.has_has_hoi, "giá bằng nhau thì không có gì để ăn");
        assert_eq!(ch.quantity_toi_uu, 0);
    }

    #[test]
    fn no_opportunity_when_the_dex_is_dearer() {
        let b = sample_pool(); // DEX 2000
        let ch = find_arb(&b, 1_900.0, 1_000_000_000);
        assert!(!ch.has_has_hoi, "chiều này không có lãi");
    }

    #[test]
    fn finds_a_profitable_opportunity_when_the_dex_is_cheaper() {
        let b = Pool::new(1_000_000, 1_900_000_000, 30); // DEX = 1900
        let ch = find_arb(&b, 2_000.0, 500_000_000);
        assert!(ch.has_has_hoi);
        assert!(ch.estimated_return > 0, "lãi phải dương thì mới gọi là cơ hội");
        assert!(ch.quantity_toi_uu > 0);
    }

    #[test]
    fn arbitrage_pulls_the_dex_toward_the_cex() {
        // Đây là lý do arbitrage tồn tại và có ích: nó khiến giá hội tụ.
        let b = Pool::new(1_000_000, 1_900_000_000, 30);
        let gia_cex = 2_000.0;
        let ch = find_arb(&b, gia_cex, 500_000_000);
        assert!(ch.has_has_hoi);
        let prev_lech = (gia_cex - ch.dex_price_before).abs();
        let next_lech = (gia_cex - ch.dex_price_after).abs();
        assert!(next_lech < prev_lech,
                "sau arbitrage giá phải gần nhau hơn: {:.2} so với {:.2}", next_lech, prev_lech);
    }

    #[test]
    fn quantity_toi_uu_true_su_toi_uu() {
        // So với các khối lượng lân cận, khối lượng tìm được phải cho lãi cao nhất.
        let b = Pool::new(1_000_000, 1_900_000_000, 30);
        let gia_cex = 2_000.0;
        let ch = find_arb(&b, gia_cex, 500_000_000);
        let lai = |v: u128| -> i128 {
            match b.try_swap_y_for_x(v) {
                Ok(x) => (x as f64 * gia_cex) as i128 - v as i128,
                Err(_) => i128::MIN,
            }
        };
        let v = ch.quantity_toi_uu;
        for other in [v / 4, v / 2, v * 2, v * 4] {
            if other > 0 && other < 500_000_000 {
                assert!(lai(v) >= lai(other),
                        "khối lượng {} cho lãi {} > {} tại {}", other, lai(other), lai(v), v);
            }
        }
    }

    #[test]
    fn von_table_no_thi_no_has_has_hoi() {
        let b = Pool::new(1_000_000, 1_900_000_000, 30);
        assert!(!find_arb(&b, 2_000.0, 0).has_has_hoi);
    }

    #[test]
    fn a_wider_dislocation_yields_more_profit() {
        let mut prev = 0i128;
        for gia_y in [1_950_000_000u128, 1_900_000_000, 1_800_000_000, 1_600_000_000] {
            let b = Pool::new(1_000_000, gia_y, 30);
            let ch = find_arb(&b, 2_000.0, 2_000_000_000);
            assert!(ch.has_has_hoi);
            assert!(ch.estimated_return > prev,
                    "lệch giá lớn hơn phải cho lãi lớn hơn: {} so với {}",
                    ch.estimated_return, prev);
            prev = ch.estimated_return;
        }
    }
}
```

---

## Bảng tra cứu lỗi biên dịch thường gặp

| Lỗi | Nguyên nhân trong chương này | Cách sửa |
|---|---|---|
| `attempt to multiply with overflow` | `x * y` với `u128` trên số dư lớn | Dùng `U256` trong sản xuất; ở đây `u128` đủ nếu kiểm biên |
| Kết quả hoán đổi lệch 1 đơn vị | Làm tròn số nguyên | AMM thật **luôn** làm tròn có lợi cho bể — đó là chủ ý |
| `E0308: expected u128, found f64` | Trộn giá thực với số dư nguyên | Giữ số dư ở dạng nguyên, chỉ ép sang `f64` ở lớp hiển thị |
| Tìm kiếm tam phân không hội tụ | Hàm lợi nhuận không lõm (phí bậc thang) | Kiểm tính lõm, hoặc quét thô rồi tinh chỉnh cục bộ |
| Chia cho 0 khi tính tổn thất | `r = 0` hoặc bể rỗng | Bảo vệ biên trước khi tính |

---

## Tóm tắt chương & Bài tập rèn luyện

### 5 điểm cốt lõi

1. **x·y = k thắng nhờ đơn giản**: không bao giờ cạn thanh khoản, không cần trạng thái, tự cân bằng.
2. **Tổn thất tạm thời là cái giá của việc làm máy bán hàng tự động** — và nó chỉ "tạm thời" nếu giá quay lại.
3. **Mempool công khai nghĩa là ý định của bạn là thông tin công khai.** MEV là hệ quả tất yếu, không phải lỗi.
4. **Phòng vệ kẹp lệnh duy nhất có hiệu lực là số nhận tối thiểu chặt.** Dung sai trượt giá chính là mức lỗ tối đa bạn cho phép.
5. **Chênh lệch giá là bài toán tối ưu.** Câu hỏi không phải "có cơ hội không" mà là "bao nhiêu thì tối ưu sau phí".

### Bài tập rèn luyện

**Bài 1.** Cài **định tuyến qua nhiều bể**: chia lệnh lớn qua vài bể để giảm tổng trượt giá.

<details>
<summary><b>Gợi ý</b></summary>

Vì trượt giá **tăng nhanh hơn tuyến tính** theo khối lượng, chia lệnh qua nhiều bể luôn cho kết quả tốt hơn dồn vào một bể. Cách chia tối ưu là cân bằng **giá cận biên** giữa các bể — giống hệt nguyên lý phân bổ tài nguyên trong kinh tế học.
</details>

<details>
<summary><b>Lời giải</b></summary>

```rust
pub struct DinhTuyen { pub cac_be: Vec<Pool> }

impl DinhTuyen {
    /// Chia tham lam theo từng lát: mỗi lát đi vào bể đang cho giá tốt nhất.
    /// Vì hàm "nhận được theo khối lượng" là lõm, tham lam theo lát hội tụ về
    /// nghiệm tối ưu khi lát đủ nhỏ.
    pub fn chia_lenh(&self, tong: Quantity, so_lat: usize) -> Vec<Quantity> {
        let mut part = vec![0 as Quantity; self.cac_be.len()];
        if so_lat == 0 || self.cac_be.is_empty() { return part; }
        let mut tam = self.cac_be.clone();
        let lat = tong / so_lat as Quantity;
        if lat == 0 { return part; }

        for _ in 0..so_lat {
            let mut best = None;
            let mut nhieu_nhat: Quantity = 0;
            for (i, be) in tam.iter().enumerate() {
                if let Ok(ra) = be.try_swap_x_for_y(lat) {
                    if best.is_none() || ra > nhieu_nhat { nhieu_nhat = ra; best = Some(i); }
                }
            }
            let i = match best { Some(i) => i, None => break };
            // Cập nhật trạng thái bể: lát sau sẽ thấy giá đã xấu đi
            let _ = tam[i].swap_x_for_y(lat, 0);
            part[i] += lat;
        }
        part
    }

    pub fn tong_nhan_duoc(&self, part: &[Quantity]) -> Quantity {
        let mut tam = self.cac_be.clone();
        part.iter().enumerate()
            .filter(|(_, &v)| v > 0)
            .filter_map(|(i, &v)| tam[i].swap_x_for_y(v, 0).ok())
            .sum()
    }
}
```

Với hai bể cùng kích thước và một lệnh lớn, chia 50/50 có thể cải thiện kết quả vài phần trăm — đủ để biến một cơ hội không lãi thành có lãi. Đây chính là công việc chính của các bộ tổng hợp như 1inch.
</details>

**Bài 2.** Cài **bộ mô phỏng đấu giá theo lô** kiểu CoW Swap: mọi lệnh trong lô nhận cùng một giá thanh toán.

<details>
<summary><b>Gợi ý</b></summary>

Ý tưởng chính: nếu ai cũng nhận **cùng một giá**, thì thứ tự trong lô không còn giá trị — và MEV theo thứ tự biến mất. Thêm nữa, các lệnh ngược chiều có thể **khớp trực tiếp với nhau** (trùng hợp mong muốn, coincidence of wants), bỏ qua bể hoàn toàn và tiết kiệm cả phí bể lẫn trượt giá.
</details>

<details>
<summary><b>Lời giải</b></summary>

```rust
#[derive(Debug, Clone)]
pub struct LenhLo { pub mua_token_x: bool, pub quantity: Quantity, pub gia_gioi_han: f64 }

#[derive(Debug)]
pub struct KetQuaLo {
    pub gia_thanh_toan: f64,
    pub khop_truc_tiep: Quantity,
    pub du_qua_be: Quantity,
    pub so_lenh_duoc_khop: usize,
}

pub fn giai_lo(cac_lenh: &[LenhLo], be: &Pool) -> KetQuaLo {
    let gia_be = be.price_x();

    // Lệnh nào chấp nhận được giá thanh toán chung thì tham gia lô.
    let buy: Vec<&LenhLo> = cac_lenh.iter()
        .filter(|l| l.mua_token_x && l.gia_gioi_han >= gia_be).collect();
    let ban: Vec<&LenhLo> = cac_lenh.iter()
        .filter(|l| !l.mua_token_x && l.gia_gioi_han <= gia_be).collect();

    let tong_mua: Quantity = buy.iter().map(|l| l.quantity).sum();
    let tong_ban: Quantity = ban.iter().map(|l| l.quantity).sum();

    // Phần trùng khớp trực tiếp — KHÔNG chạm bể: không phí, không trượt giá.
    let khop_truc_tiep = tong_mua.min(tong_ban);
    let du = tong_mua.max(tong_ban) - khop_truc_tiep;

    KetQuaLo {
        gia_thanh_toan: gia_be,
        khop_truc_tiep,
        du_qua_be: du,
        so_lenh_duoc_khop: buy.len() + ban.len(),
    }
}
```

Khi `khop_truc_tiep` lớn, người dùng tiết kiệm được toàn bộ phí bể và trượt giá cho phần đó. Đó là lý do đấu giá theo lô không chỉ chống MEV mà còn cho giá **tốt hơn** trong nhiều trường hợp — hai lợi ích từ cùng một thay đổi cơ chế.
</details>
