#![allow(dead_code)]
//! Chương 77 — Chiến lược & Quản trị rủi ro thời gian thực: cổng rủi ro trước
//! giao dịch, tín hiệu từ sổ lệnh, arbitrage thống kê theo cặp, định cỡ vị thế,
//! và các thước đo rủi ro.
//!
//! Nguyên tắc xuyên suốt: **cổng rủi ro là thứ DUY NHẤT không được phép có
//! ngoại lệ**. Chiến lược có thể sai; cổng rủi ro thì không.

use std::collections::VecDeque;

pub type Price = i64;      // tick
pub type Quantity = i64;

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum Side { Buy, Sell }

impl Side {
    pub fn first(self) -> i64 { match self { Side::Buy => 1, Side::Sell => -1 } }
}

// ============================================================================
// 1. CỔNG RỦI RO TRƯỚC GIAO DỊCH
// ============================================================================
// Mọi lệnh đều phải qua đây. Không có đường vòng, không có cờ "bỏ qua kiểm
// tra cho fast". Lịch sử ngành đầy những vụ sập vì ai đó mở một đường vòng.

#[derive(Debug, Clone, PartialEq)]
pub enum RejectReason {
    NonPositiveQuantity(Quantity),
    NonPositivePrice(Price),
    /// Ngón tay béo: giá lệch quá xa giá thị trường — gần như chắc chắn gõ nhầm.
    NgonTayBeo { price: Price, reference: Price, lech_percent: f64 },
    ExceedsOrderValue { value: i64, tran: i64 },
    ExceedsPosition { next_order: i64, tran: i64 },
    ExceedsDailyLoss { lo: i64, tran: i64 },
    ExceedsOrderRate { count: u32, tran: u32 },
    KillSwitchOn,
}

#[derive(Debug, Clone)]
pub struct LimitRisk {
    pub max_order_value: i64,
    pub max_position: i64,
    pub max_daily_loss: i64,
    pub so_lenh_moi_giay_toi_da: u32,
    /// Lệch quá tỉ lệ này so với giá tham chiếu thì coi là gõ nhầm.
    pub fat_finger_threshold: f64,
}

impl Default for LimitRisk {
    fn default() -> Self {
        LimitRisk {
            max_order_value: 100_000_000,
            max_position: 10_000,
            max_daily_loss: 5_000_000,
            so_lenh_moi_giay_toi_da: 100,
            fat_finger_threshold: 0.10, // 10%
        }
    }
}

#[derive(Debug, Clone)]
pub struct RiskGate {
    pub limit: LimitRisk,
    pub position: i64,
    pub realized_pnl: i64,
    /// Giá vốn bình quân của vị thế đang mở. KHÔNG có nó thì không tính được
    /// lãi/lỗ — chỉ biết dòng tiền, mà dòng tiền không phải lãi.
    pub cost_basis: f64,
    /// Dấu thời gian các lệnh gần đây, để đếm tần suất.
    window_order: VecDeque<u64>,
    /// Công tắc tắt: bật rồi thì KHÔNG tự tắt được. Chỉ người mới gỡ được.
    switch_all: bool,
    pub order_book_qua: u64,
    pub orders_blocked: u64,
}

impl RiskGate {
    pub fn new(limit: LimitRisk) -> Self {
        RiskGate { limit, position: 0, realized_pnl: 0, cost_basis: 0.0,
                    window_order: VecDeque::new(), switch_all: false,
                    order_book_qua: 0, orders_blocked: 0 }
    }

    pub fn da_tat(&self) -> bool { self.switch_all }
    /// Bật công tắc tắt. Một chiều — chỉ người vận hành mới gỡ được.
    pub fn enable_all_switches(&mut self) { self.switch_all = true; }
    pub fn operator_flips_switch(&mut self) { self.switch_all = false; }

    /// Kiểm tra một lệnh. `bay_gio_ns` dùng cho cửa sổ tần suất.
    pub fn check(&mut self, side: Side, price: Price, quantity: Quantity,
                    reference_price: Price, bay_gio_ns: u64) -> Result<(), RejectReason>
    {
        let ket_qua = self.check_join_unit(side, price, quantity, reference_price, bay_gio_ns);
        match &ket_qua {
            Ok(()) => {
                self.order_book_qua += 1;
                self.window_order.push_back(bay_gio_ns);
            }
            Err(_) => self.orders_blocked += 1,
        }
        ket_qua
    }

    fn check_join_unit(&mut self, side: Side, price: Price, quantity: Quantity,
                       reference_price: Price, bay_gio_ns: u64) -> Result<(), RejectReason>
    {
        // Công tắc tắt xét ĐẦU TIÊN. Đã tắt thì không gì lọt qua được.
        if self.switch_all { return Err(RejectReason::KillSwitchOn); }
        if quantity <= 0 { return Err(RejectReason::NonPositiveQuantity(quantity)); }
        if price <= 0 { return Err(RejectReason::NonPositivePrice(price)); }

        // Ngón tay béo: gõ 8400 thành 84000 là chuyện xảy ra hằng năm
        if reference_price > 0 {
            let lech = (price - reference_price).abs() as f64 / reference_price as f64;
            if lech > self.limit.fat_finger_threshold {
                return Err(RejectReason::NgonTayBeo { price, reference: reference_price,
                                                lech_percent: lech * 100.0 });
            }
        }

        let value = price * quantity;
        if value > self.limit.max_order_value {
            return Err(RejectReason::ExceedsOrderValue { value, tran: self.limit.max_order_value });
        }

        let next_order = self.position + side.first() * quantity;
        if next_order.abs() > self.limit.max_position {
            return Err(RejectReason::ExceedsPosition { next_order, tran: self.limit.max_position });
        }

        if self.realized_pnl < -self.limit.max_daily_loss {
            return Err(RejectReason::ExceedsDailyLoss { lo: -self.realized_pnl,
                                                 tran: self.limit.max_daily_loss });
        }

        // Cửa sổ trượt một giây
        while let Some(&t) = self.window_order.front() {
            if bay_gio_ns.saturating_sub(t) >= 1_000_000_000 { self.window_order.pop_front(); }
            else { break; }
        }
        let count = self.window_order.len() as u32;
        if count >= self.limit.so_lenh_moi_giay_toi_da {
            return Err(RejectReason::ExceedsOrderRate { count,
                                                   tran: self.limit.so_lenh_moi_giay_toi_da });
        }
        Ok(())
    }

    /// Ghi nhận một lần khớp — cập nhật vị thế, giá vốn và lãi/lỗ đã chốt.
    ///
    /// Điểm dễ sai nhất trong cả chương: lãi/lỗ KHÔNG phải dòng tiền của lệnh
    /// đóng. Bán 100 cổ giá 88,00 mang về tiền, nhưng nếu bid vào ở 90,00 thì
    /// đó là một khoản LỖ. Muốn biết lãi hay lỗ, bắt buộc phải nhớ GIÁ VỐN.
    pub fn record_recv_fill(&mut self, side: Side, price: Price, quantity: Quantity) {
        let prev = self.position;
        let d = side.first() * quantity;

        if prev == 0 || prev.signum() == d.signum() {
            // Mở mới hoặc mở thêm cùng chiều → bình quân lại giá vốn
            let tong = (prev.abs() + quantity) as f64;
            self.cost_basis = (self.cost_basis * prev.abs() as f64
                            + price as f64 * quantity as f64) / tong;
            self.position = prev + d;
        } else {
            // Đóng bớt hoặc đóng hết → hiện thực hoá lãi/lỗ phần đóng được
            let dong = quantity.min(prev.abs());
            self.realized_pnl +=
                ((price as f64 - self.cost_basis) * dong as f64 * prev.signum() as f64) as i64;
            self.position = prev + d;
            if self.position == 0 {
                self.cost_basis = 0.0;
            } else if self.position.signum() != prev.signum() {
                // Đảo chiều: phần dư là một vị thế MỚI, giá vốn là giá vừa khớp
                self.cost_basis = price as f64;
            }
        }

        // Tự bảo vệ: lỗ chạm trần thì tự bật công tắc tắt
        if self.realized_pnl < -self.limit.max_daily_loss {
            self.switch_all = true;
        }
    }
}

// ============================================================================
// 2. TÍN HIỆU TỪ SỔ LỆNH
// ============================================================================

/// Mất cân bằng khối lượng hai bên, chuẩn hoá về [-1, 1].
/// Dương = áp lực bid. Đây là tín hiệu đơn giản nhất mà vẫn có sức dự báo thật.
pub fn imbalance(qty_buy: u64, qty_sell: u64) -> f64 {
    let tong = qty_buy + qty_sell;
    if tong == 0 { return 0.0; }
    (qty_buy as f64 - qty_sell as f64) / tong as f64
}

/// Giá vi mô: giá giữa có gia quyền theo khối lượng ĐỐI ỨNG.
/// Nhiều người muốn bid → giá vi mô lệch về phía giá bán.
pub fn price_pos_open(price_buy: Price, qty_buy: u64, price_sell: Price, qty_sell: u64) -> Option<f64> {
    let tong = qty_buy + qty_sell;
    if tong == 0 { return None; }
    Some((price_buy as f64 * qty_sell as f64 + price_sell as f64 * qty_buy as f64) / tong as f64)
}

/// Cửa sổ trượt tính trung bình và độ lệch chuẩn — O(1) mỗi lần thêm.
#[derive(Debug, Clone)]
pub struct StatsWindow {
    o: VecDeque<f64>,
    capacity: usize,
    tong: f64,
    sum_of_squares: f64,
}

impl StatsWindow {
    pub fn new(capacity: usize) -> Self {
        StatsWindow { o: VecDeque::with_capacity(capacity), capacity,
                       tong: 0.0, sum_of_squares: 0.0 }
    }
    pub fn them(&mut self, x: f64) {
        if self.o.len() == self.capacity {
            if let Some(cu) = self.o.pop_front() {
                self.tong -= cu;
                self.sum_of_squares -= cu * cu;
            }
        }
        self.o.push_back(x);
        self.tong += x;
        self.sum_of_squares += x * x;
    }
    pub fn quantity(&self) -> usize { self.o.len() }
    pub fn day(&self) -> bool { self.o.len() == self.capacity }
    pub fn mean(&self) -> f64 {
        if self.o.is_empty() { 0.0 } else { self.tong / self.o.len() as f64 }
    }
    /// Phương sai mẫu (chia n−1). Trả 0 khi chưa đủ 2 điểm.
    pub fn variance(&self) -> f64 {
        let n = self.o.len() as f64;
        if n < 2.0 { return 0.0; }
        let ps = (self.sum_of_squares - self.tong * self.tong / n) / (n - 1.0);
        ps.max(0.0) // chặn sai số dấu phẩy động làm ra số âm
    }
    pub fn stddev(&self) -> f64 { self.variance().sqrt() }
    /// Điểm z: giá trị này lệch bao nhiêu độ lệch chuẩn so với trung bình.
    pub fn diem_z(&self, x: f64) -> Option<f64> {
        let s = self.stddev();
        if s < 1e-9 { None } else { Some((x - self.mean()) / s) }
    }
}

// ============================================================================
// 3. ARBITRAGE THỐNG KÊ THEO CẶP
// ============================================================================
// Ý tưởng: hai mã cùng ngành thường đi cùng nhau. Khi chênh lệch giãn bất
// thường, đặt cược nó sẽ co lại. Rủi ro lớn nhất KHÔNG phải chênh lệch không
// co, mà là quan hệ giữa hai mã ĐÃ GÃY HẲN mà ta không nhận ra.

#[derive(Debug, Clone, Copy, PartialEq)]
pub enum SignalCap { OpenLongA, MoDaiB, Dong, KhongLam }

pub struct ArbCap {
    pub proxy_ratio: f64, // beta: 1 đơn vị A ứng với bao nhiêu đơn vị B
    pub window: StatsWindow,
    pub threshold_in: f64,
    pub threshold_out: f64,
    /// Chênh lệch giãn quá mức này thì coi như quan hệ đã gãy — CẮT LỖ.
    pub threshold_use: f64,
    pub is_open: Option<SignalCap>,
}

impl ArbCap {
    pub fn new(proxy_ratio: f64, window: usize,
               threshold_in: f64, threshold_out: f64, threshold_use: f64) -> Self {
        ArbCap { proxy_ratio, window: StatsWindow::new(window),
                 threshold_in, threshold_out, threshold_use, is_open: None }
    }

    pub fn spread(&self, gia_a: Price, gia_b: Price) -> f64 {
        gia_a as f64 - self.proxy_ratio * gia_b as f64
    }

    pub fn update(&mut self, gia_a: Price, gia_b: Price) -> SignalCap {
        let cl = self.spread(gia_a, gia_b);
        // Tính điểm z TRƯỚC khi thêm điểm mới — nếu không, chính điểm dị
        // biệt ta muốn phát hiện lại kéo trung bình về phía nó và tự che mình.
        let previous_series = self.window.day();
        let z = self.window.diem_z(cl);
        self.window.them(cl);

        let z = match z {
            Some(z) if previous_series => z,
            _ => return SignalCap::KhongLam,
        };

        match self.is_open {
            None => {
                if z > self.threshold_in {
                    // A đắt bất thường so với B → bán A, bid B
                    self.is_open = Some(SignalCap::MoDaiB);
                    SignalCap::MoDaiB
                } else if z < -self.threshold_in {
                    self.is_open = Some(SignalCap::OpenLongA);
                    SignalCap::OpenLongA
                } else { SignalCap::KhongLam }
            }
            Some(_) => {
                // Cắt lỗ đứng TRƯỚC chốt lời: quan hệ gãy thì phải thoát ngay
                if z.abs() > self.threshold_use || z.abs() < self.threshold_out {
                    self.is_open = None;
                    SignalCap::Dong
                } else { SignalCap::KhongLam }
            }
        }
    }
}

// ============================================================================
// 4. ĐỊNH CỠ VỊ THẾ
// ============================================================================

/// Tỉ lệ Kelly: f* = (p·b − q) / b, với p = xác suất thắng, b = tỉ lệ thắng/thua.
///
/// Kelly toàn phần tối ưu về tốc độ tăng trưởng dài hạn, nhưng dao động khủng
/// khiếp và cực nhạy với sai số ước lượng `p`. Thực tế người ta dùng một PHẦN
/// của Kelly (thường 1/4 tới 1/2) — đánh đổi chút tăng trưởng lấy nhiều bình yên.
pub fn kelly_fraction(xac_suat_thang: f64, ty_le_thang_thua: f64) -> f64 {
    if ty_le_thang_thua <= 0.0 { return 0.0; }
    let q = 1.0 - xac_suat_thang;
    ((xac_suat_thang * ty_le_thang_thua - q) / ty_le_thang_thua).max(0.0)
}

pub fn fractional_kelly(xac_suat_thang: f64, ty_le_thang_thua: f64, part: f64) -> f64 {
    (kelly_fraction(xac_suat_thang, ty_le_thang_thua) * part).clamp(0.0, 1.0)
}

/// Định cỡ theo mục tiêu biến động: mã càng dao động mạnh thì bid càng ít,
/// sao cho rủi ro tính bằng tiền là như nhau ở mọi mã.
pub fn has_theo_volatility(von: i64, bien_dong_muc_tieu: f64,
                         volatility_default_peak: f64, price: Price) -> Quantity {
    if volatility_default_peak <= 0.0 || price <= 0 { return 0; }
    let ty_in = (bien_dong_muc_tieu / volatility_default_peak).min(1.0);
    ((von as f64 * ty_in) / price as f64) as Quantity
}

// ============================================================================
// 5. THƯỚC ĐO RỦI RO
// ============================================================================

#[derive(Debug, PartialEq)]
pub struct RiskOwned {
    pub total_pnl: i64,
    pub max_drawdown: i64,
    pub ratio_drawdown: f64,
    pub num_session_lai: usize,
    pub num_session_lo: usize,
    /// Tỉ số lợi nhuận trên độ dao động — càng high càng "êm".
    pub sharpe_ratio: f64,
}

pub fn risk_level(equity_curve: &[i64]) -> RiskOwned {
    if equity_curve.len() < 2 {
        return RiskOwned { total_pnl: 0, max_drawdown: 0, ratio_drawdown: 0.0,
                              num_session_lai: 0, num_session_lo: 0, sharpe_ratio: 0.0 };
    }
    let mut peak = equity_curve[0];
    let mut dd = 0i64;
    for &v in equity_curve {
        peak = peak.max(v);
        dd = dd.max(peak - v);
    }
    let deltas: Vec<f64> = equity_curve.windows(2).map(|w| (w[1] - w[0]) as f64).collect();
    let n = deltas.len() as f64;
    let tb = deltas.iter().sum::<f64>() / n;
    let ps = deltas.iter().map(|x| (x - tb).powi(2)).sum::<f64>() / (n - 1.0).max(1.0);
    let sd = ps.sqrt();
    RiskOwned {
        total_pnl: equity_curve[equity_curve.len() - 1] - equity_curve[0],
        max_drawdown: dd,
        ratio_drawdown: if peak.abs() > 0 { dd as f64 / peak.abs() as f64 } else { 0.0 },
        num_session_lai: deltas.iter().filter(|&&x| x > 0.0).count(),
        num_session_lo: deltas.iter().filter(|&&x| x < 0.0).count(),
        sharpe_ratio: if sd < 1e-12 { 0.0 } else { tb / sd },
    }
}

// ============================================================================
// 6. SINH DỮ LIỆU TẤT ĐỊNH
// ============================================================================

/// Hai chuỗi giá đồng liên kết: chúng cùng đi theo một nhân tố chung, cộng
/// thêm nhiễu riêng. Đây đúng là tình huống mà arbitrage cặp khai thác.
pub fn gen_cap_price(n: usize, hat_giong: u64, beta: f64) -> (Vec<Price>, Vec<Price>) {
    let mut s = hat_giong;
    let mut recv_to_chung = 10_000.0f64;
    let (mut a, mut b) = (Vec::with_capacity(n), Vec::with_capacity(n));
    for _ in 0..n {
        s = s.wrapping_mul(6364136223846793005).wrapping_add(1442695040888963407);
        let e1 = ((s >> 33) % 201) as f64 - 100.0;
        let e2 = ((s >> 45) % 61) as f64 - 30.0;
        let e3 = ((s >> 20) % 61) as f64 - 30.0;
        recv_to_chung += e1 * 0.1;
        a.push((recv_to_chung + e2) as Price);
        b.push(((recv_to_chung + e3) / beta) as Price);
    }
    (a, b)
}

fn main() {
    println!("═══════════════════════════════════════════════════════════");
    println!("   CHIẾN LƯỢC & QUẢN TRỊ RỦI RO THỜI GIAN THỰC             ");
    println!("═══════════════════════════════════════════════════════════");

    println!("\n1. CỔNG RỦI RO — mọi lệnh đều phải qua đây");
    let mut gate = RiskGate::new(LimitRisk {
        max_order_value: 10_000_000, max_position: 500,
        max_daily_loss: 100_000, so_lenh_moi_giay_toi_da: 5,
        fat_finger_threshold: 0.10,
    });
    let tc = 8_400;
    for (description, side, price, sl) in [
        ("hợp lệ            ", Side::Buy, 8_400i64, 100i64),
        ("ngón tay béo x10  ", Side::Buy, 84_000, 100),
        ("giá trị quá lớn   ", Side::Buy, 8_400, 100_000),
        ("số lượng âm       ", Side::Buy, 8_400, -5),
        ("vượt trần vị thế  ", Side::Buy, 8_400, 600),
    ] {
        match gate.check(side, price, sl, tc, 1_000_000_000) {
            Ok(()) => println!("   {} → CHO QUA", description),
            Err(e) => println!("   {} → CHẶN: {:?}", description, e),
        }
    }

    println!("\n2. GIỚI HẠN TẦN SUẤT — chống vòng lặp lỗi bắn lệnh liên tục");
    let mut c2 = RiskGate::new(LimitRisk { so_lenh_moi_giay_toi_da: 5, ..Default::default() });
    let mut qua = 0;
    for i in 0..10u64 {
        if c2.check(Side::Buy, 8_400, 1, tc, 1_000_000_000 + i * 1_000_000).is_ok() {
            qua += 1;
        }
    }
    println!("   Bắn 10 lệnh trong 10 ms → chỉ {} lệnh lọt qua (trần 5/giây)", qua);

    println!("\n3. CÔNG TẮC TẮT TỰ ĐỘNG KHI LỖ CHẠM TRẦN");
    let mut c3 = RiskGate::new(LimitRisk { max_daily_loss: 10_000,
                                              ..Default::default() });
    c3.record_recv_fill(Side::Buy, 9_000, 100);
    c3.record_recv_fill(Side::Sell, 8_800, 100); // lỗ 20 000
    println!("   Sau khi lỗ {} → công tắc tắt: {}", -c3.realized_pnl, c3.da_tat());
    println!("   Lệnh tiếp theo → {:?}",
             c3.check(Side::Buy, 8_400, 1, tc, 2_000_000_000).unwrap_err());
    c3.operator_flips_switch();
    println!("   Người vận hành gỡ công tắc → giao dịch lại được: {}",
             c3.check(Side::Buy, 8_400, 1, tc, 3_000_000_000).is_ok());

    println!("\n4. TÍN HIỆU TỪ SỔ LỆNH");
    for (m, b) in [(1000u64, 1000u64), (9000, 1000), (1000, 9000)] {
        println!("   bid {:>4} / bán {:>4} → mất cân bằng {:>6.2} · giá vi mô {:>8.2}",
                 m, b, imbalance(m, b), price_pos_open(8_400, m, 8_410, b).unwrap());
    }
    println!("   → Nhiều người chờ bid thì giá vi mô lệch LÊN phía giá bán.");

    println!("\n5. ARBITRAGE CẶP");
    let (ga, gb) = gen_cap_price(3_000, 2024, 1.5);
    let mut arb = ArbCap::new(1.5, 100, 2.0, 0.5, 4.0);
    let (mut entries, mut ra) = (0, 0);
    for i in 0..ga.len() {
        match arb.update(ga[i], gb[i]) {
            SignalCap::OpenLongA | SignalCap::MoDaiB => entries += 1,
            SignalCap::Dong => ra += 1,
            SignalCap::KhongLam => {}
        }
    }
    println!("   {} điểm dữ liệu → vào lệnh {} lần · thoát {} lần", ga.len(), entries, ra);
    println!("   → Ngưỡng dừng 4σ tồn tại vì chênh lệch giãn mãi nghĩa là quan hệ ĐÃ GÃY,");
    println!("     không phải 'cơ hội càng ngon hơn'.");

    println!("\n6. ĐỊNH CỠ VỊ THẾ");
    println!("   {:<28} {:>8} {:>10}", "kịch bản", "Kelly", "1/4 Kelly");
    for (description, p, b) in [
        ("55% thắng, ăn 1 thua 1  ", 0.55, 1.0),
        ("60% thắng, ăn 1 thua 1  ", 0.60, 1.0),
        ("40% thắng, ăn 2 thua 1  ", 0.40, 2.0),
        ("45% thắng, ăn 1 thua 1  ", 0.45, 1.0),
    ] {
        println!("   {} {:>7.1}% {:>9.1}%", description,
                 kelly_fraction(p, b) * 100.0, fractional_kelly(p, b, 0.25) * 100.0);
    }
    println!("   → Lợi thế âm thì Kelly = 0: công thức tự bảo bạn ĐỪNG đánh.");

    println!("\n7. THƯỚC ĐO RỦI RO — hai đường vốn cùng đích, khác hẳn nhau");
    // "Êm" KHÔNG có nghĩa là đường thẳng tuyệt đối — đường thẳng thì độ lệch
    // chuẩn bằng 0 và Sharpe không định nghĩa được. Êm nghĩa là dao động nhỏ.
    let em: Vec<i64> = (0..100).map(|i| 100_000 + i * 500 + (i % 5) * 40).collect();
    let mut xoc: Vec<i64> = Vec::new();
    let mut v = 100_000i64;
    for i in 0..100 { v += if i % 3 == 0 { -8_000 } else { 5_750 }; xoc.push(v); }
    for (name, d) in [("êm ", &em), ("xóc", &xoc)] {
        let r = risk_level(d);
        println!("   {} → lãi {:>6} · sụt sâu nhất {:>6} · Sharpe {:>5.2} · thắng {}/{}",
                 name, r.total_pnl, r.max_drawdown, r.sharpe_ratio,
                 r.num_session_lai, r.num_session_lai + r.num_session_lo);
    }
    println!("   → Đường xóc lãi NHIỀU HƠN, nhưng Sharpe thấp hơn ~35 lần và có");
    println!("     những cú sụt 8.000 giữa đường. Phần lớn người sẽ bỏ cuộc trước khi");
    println!("     nó kịp về đích — lợi nhuận trên giấy không phải lợi nhuận thu được.");

    println!("\n═══════════════════════════════════════════════════════════");
    println!("   CHIẾN LƯỢC ĐƯỢC PHÉP SAI. CỔNG RỦI RO THÌ KHÔNG.         ");
    println!("═══════════════════════════════════════════════════════════");
}

#[cfg(test)]
mod tests {
    use super::*;

    fn sample_gate() -> RiskGate {
        RiskGate::new(LimitRisk {
            max_order_value: 10_000_000, max_position: 500,
            max_daily_loss: 100_000, so_lenh_moi_giay_toi_da: 5,
            fat_finger_threshold: 0.10,
        })
    }

    // ---------- Cổng rủi ro ----------
    #[test]
    fn a_valid_order_passes() {
        let mut c = sample_gate();
        assert_eq!(c.check(Side::Buy, 8_400, 100, 8_400, 1_000_000_000), Ok(()));
        assert_eq!(c.order_book_qua, 1);
        assert_eq!(c.orders_blocked, 0);
    }

    #[test]
    fn blocks_fat_finger_prices() {
        // Gõ 8400 thành 84000 — lỗi có thật, xảy ra hằng năm ở mọi thị trường.
        let mut c = sample_gate();
        let e = c.check(Side::Buy, 84_000, 1, 8_400, 1_000_000_000).unwrap_err();
        assert!(matches!(e, RejectReason::NgonTayBeo { .. }));
        // Lệch nhỏ trong ngưỡng thì vẫn cho qua
        assert!(c.check(Side::Buy, 8_800, 1, 8_400, 1_000_000_000).is_ok());
    }

    #[test]
    fn no_reference_price_skips_the_fat_finger_check() {
        // Mã mới niêm yết chưa có giá tham chiếu — không được chặn oan.
        let mut c = sample_gate();
        assert!(c.check(Side::Buy, 9_000, 1, 0, 1_000_000_000).is_ok());
    }

    #[test]
    fn blocks_invalid_size_and_price() {
        let mut c = sample_gate();
        assert_eq!(c.check(Side::Buy, 8_400, 0, 8_400, 1).unwrap_err(),
                   RejectReason::NonPositiveQuantity(0));
        assert_eq!(c.check(Side::Buy, 8_400, -5, 8_400, 1).unwrap_err(),
                   RejectReason::NonPositiveQuantity(-5));
        assert_eq!(c.check(Side::Buy, 0, 10, 0, 1).unwrap_err(),
                   RejectReason::NonPositivePrice(0));
    }

    #[test]
    fn blocks_oversized_notional() {
        let mut c = sample_gate();
        assert!(matches!(c.check(Side::Buy, 8_400, 100_000, 8_400, 1).unwrap_err(),
                         RejectReason::ExceedsOrderValue { .. }));
    }

    #[test]
    fn blocks_position_breach_on_both_sides() {
        let mut c = sample_gate();
        assert!(matches!(c.check(Side::Buy, 8_400, 501, 8_400, 1).unwrap_err(),
                         RejectReason::ExceedsPosition { next_order: 501, tran: 500 }));
        assert!(matches!(c.check(Side::Sell, 8_400, 501, 8_400, 1).unwrap_err(),
                         RejectReason::ExceedsPosition { next_order: -501, tran: 500 }),
                "bán khống cũng phải bị chặn, không chỉ bid");
    }

    #[test]
    fn current_position_counts_toward_the_limit() {
        let mut c = sample_gate();
        c.record_recv_fill(Side::Buy, 8_400, 400);
        assert!(c.check(Side::Buy, 8_400, 100, 8_400, 1).is_ok(), "400+100 = 500, vừa trần");
        assert!(c.check(Side::Buy, 8_400, 101, 8_400, 1).is_err(), "400+101 vượt trần");
        assert!(c.check(Side::Sell, 8_400, 400, 8_400, 1).is_ok(), "bán thì giảm vị thế");
    }

    #[test]
    fn rate_limit_blocks_at_the_right_count() {
        let mut c = sample_gate(); // trần 5 lệnh/giây
        let mut qua = 0;
        for i in 0..20u64 {
            if c.check(Side::Buy, 8_400, 1, 8_400, 1_000_000_000 + i * 1_000_000).is_ok() {
                qua += 1;
            }
        }
        assert_eq!(qua, 5, "đúng 5 lệnh lọt qua trong một giây");
    }

    #[test]
    fn the_rate_window_slides_with_time() {
        let mut c = sample_gate();
        for i in 0..5u64 {
            assert!(c.check(Side::Buy, 8_400, 1, 8_400, 1_000_000_000 + i).is_ok());
        }
        assert!(c.check(Side::Buy, 8_400, 1, 8_400, 1_000_000_100).is_err(), "đã đủ 5");
        // Sang giây sau thì cửa sổ trượt qua, lại cho phép
        assert!(c.check(Side::Buy, 8_400, 1, 8_400, 2_500_000_000).is_ok());
    }

    #[test]
    fn the_kill_switch_blocks_everything_and_never_self_clears() {
        let mut c = sample_gate();
        c.enable_all_switches();
        // Kể cả lệnh hoàn toàn hợp lệ cũng không lọt
        assert_eq!(c.check(Side::Buy, 8_400, 1, 8_400, 1).unwrap_err(),
                   RejectReason::KillSwitchOn);
        assert!(c.da_tat(), "công tắc KHÔNG được tự tắt sau khi chặn");
        c.operator_flips_switch();
        assert!(c.check(Side::Buy, 8_400, 1, 8_400, 1).is_ok());
    }

    #[test]
    fn hitting_the_loss_cap_trips_the_kill_switch() {
        let mut c = RiskGate::new(LimitRisk { max_daily_loss: 10_000,
                                                 ..Default::default() });
        assert!(!c.da_tat());
        c.record_recv_fill(Side::Buy, 9_000, 100);
        c.record_recv_fill(Side::Sell, 8_800, 100); // lỗ 20 000 > trần 10 000
        assert_eq!(c.realized_pnl, -20_000);
        assert!(c.da_tat(), "vượt trần lỗ phải tự dừng, không chờ người can thiệp");
    }

    #[test]
    fn a_profitable_close_does_not_trip_the_switch() {
        let mut c = RiskGate::new(LimitRisk { max_daily_loss: 10_000,
                                                 ..Default::default() });
        c.record_recv_fill(Side::Buy, 8_000, 100);
        c.record_recv_fill(Side::Sell, 8_500, 100);
        assert_eq!(c.realized_pnl, 50_000, "bid 80.00 bán 85.00 → lãi");
        assert!(!c.da_tat());
        assert_eq!(c.position, 0);
    }

    #[test]
    fn cost_basis_averages_when_adding() {
        let mut c = sample_gate();
        c.record_recv_fill(Side::Buy, 8_000, 100);
        c.record_recv_fill(Side::Buy, 9_000, 100);
        assert!((c.cost_basis - 8_500.0).abs() < 1e-9, "bình quân 8000 và 9000 = 8500");
        c.record_recv_fill(Side::Sell, 8_500, 200);
        assert_eq!(c.realized_pnl, 0, "bán đúng giá vốn thì hoà vốn");
        assert_eq!(c.position, 0);
        assert_eq!(c.cost_basis, 0.0, "đóng hết thì giá vốn phải về 0");
    }

    #[test]
    fn reversing_resets_the_cost_basis() {
        let mut c = sample_gate();
        c.record_recv_fill(Side::Buy, 8_000, 100);
        // Bán 300: đóng 100 (lãi) rồi mở mới 200 ở chiều bán
        c.record_recv_fill(Side::Sell, 8_500, 300);
        assert_eq!(c.position, -200);
        assert_eq!(c.realized_pnl, 50_000, "chỉ phần ĐÓNG mới tính lãi");
        assert!((c.cost_basis - 8_500.0).abs() < 1e-9, "phần dư là vị thế mới ở giá 8500");
    }

    #[test]
    fn short_then_cheaper_buyback_is_profitable() {
        let mut c = sample_gate();
        c.record_recv_fill(Side::Sell, 9_000, 100);
        assert_eq!(c.position, -100);
        c.record_recv_fill(Side::Buy, 8_500, 100);
        assert_eq!(c.realized_pnl, 50_000, "bán khống 90.00 bid lại 85.00 → lãi");
    }

    #[test]
    fn adding_in_the_same_direction_realizes_nothing() {
        let mut c = sample_gate();
        c.record_recv_fill(Side::Buy, 8_000, 100);
        c.record_recv_fill(Side::Buy, 9_000, 100);
        assert_eq!(c.position, 200);
        assert_eq!(c.realized_pnl, 0, "chưa đóng gì thì chưa chốt lãi/lỗ");
    }

    #[test]
    fn counts_passed_and_blocked_orders_correctly() {
        let mut c = sample_gate();
        c.check(Side::Buy, 8_400, 100, 8_400, 1).ok();
        c.check(Side::Buy, 84_000, 100, 8_400, 1).ok();
        c.check(Side::Buy, 8_400, -1, 8_400, 1).ok();
        assert_eq!(c.order_book_qua, 1);
        assert_eq!(c.orders_blocked, 2);
    }

    // ---------- Tín hiệu ----------
    #[test]
    fn imbalance_stays_within_minus_one_and_one() {
        assert_eq!(imbalance(0, 0), 0.0, "sổ rỗng thì trung tính, không chia cho 0");
        assert_eq!(imbalance(100, 100), 0.0);
        assert_eq!(imbalance(100, 0), 1.0);
        assert_eq!(imbalance(0, 100), -1.0);
        for (m, b) in [(1u64, 999u64), (500, 500), (999, 1), (7, 13)] {
            let x = imbalance(m, b);
            assert!((-1.0..=1.0).contains(&x));
        }
    }

    #[test]
    fn micro_price_leans_toward_the_thin_side() {
        // Nhiều người chờ MUA → áp lực đẩy giá lên → giá vi mô gần giá BÁN.
        let many_buy = price_pos_open(8_400, 9_000, 8_410, 1_000).unwrap();
        let many_sell = price_pos_open(8_400, 1_000, 8_410, 9_000).unwrap();
        let can_bang = price_pos_open(8_400, 1_000, 8_410, 1_000).unwrap();
        assert!(many_buy > can_bang, "áp lực bid đẩy giá vi mô lên");
        assert!(many_sell < can_bang, "áp lực bán kéo xuống");
        assert!((can_bang - 8_405.0).abs() < 1e-9, "cân bằng thì đúng giá giữa");
        assert!(many_buy > 8_400.0 && many_buy < 8_410.0, "luôn nằm trong chênh lệch");
    }

    #[test]
    fn gia_vi_mo_so_rong_tra_none() {
        assert_eq!(price_pos_open(8_400, 0, 8_410, 0), None);
    }

    // ---------- Cửa sổ thống kê ----------
    #[test]
    fn window_computes_mean_and_stddev_correctly() {
        let mut c = StatsWindow::new(5);
        for x in [2.0, 4.0, 4.0, 4.0, 5.0] { c.them(x); }
        assert!((c.mean() - 3.8).abs() < 1e-9);
        // phương sai mẫu của [2,4,4,4,5] = 1.2
        assert!((c.variance() - 1.2).abs() < 1e-9);
        assert!(c.day());
    }

    #[test]
    fn the_sliding_window_drops_old_values() {
        let mut c = StatsWindow::new(3);
        for x in [1.0, 2.0, 3.0, 4.0, 5.0] { c.them(x); }
        assert_eq!(c.quantity(), 3);
        assert!((c.mean() - 4.0).abs() < 1e-9, "chỉ còn [3,4,5]");
    }

    #[test]
    fn variance_is_never_negative_despite_float_error() {
        let mut c = StatsWindow::new(50);
        for _ in 0..50 { c.them(1_000_000.0); } // toàn giá trị giống hệt, cỡ lớn
        assert!(c.variance() >= 0.0, "phải chặn sai số làm ra số âm");
        assert!(c.variance() < 1e-3, "dữ liệu không đổi thì phương sai ~0");
        assert_eq!(c.diem_z(1_000_000.0), None, "độ lệch ~0 thì điểm z vô nghĩa");
    }

    #[test]
    fn fewer_than_two_points_gives_zero_variance() {
        let mut c = StatsWindow::new(10);
        assert_eq!(c.variance(), 0.0);
        c.them(5.0);
        assert_eq!(c.variance(), 0.0, "một điểm thì không có phương sai mẫu");
    }

    #[test]
    fn diem_z_do_dung_do_lech() {
        let mut c = StatsWindow::new(100);
        for i in 0..100 { c.them((i % 10) as f64); }
        let z = c.diem_z(c.mean()).unwrap();
        assert!(z.abs() < 1e-9, "đúng giá trị trung bình thì z = 0");
        let z2 = c.diem_z(c.mean() + 2.0 * c.stddev()).unwrap();
        assert!((z2 - 2.0).abs() < 1e-9);
    }

    // ---------- Arbitrage cặp ----------
    #[test]
    fn arb_stays_silent_until_warm() {
        let mut a = ArbCap::new(1.5, 100, 2.0, 0.5, 4.0);
        let (ga, gb) = gen_cap_price(50, 1, 1.5);
        for i in 0..50 {
            assert_eq!(a.update(ga[i], gb[i]), SignalCap::KhongLam,
                       "cửa sổ chưa đầy thì tuyệt đối không được vào lệnh");
        }
    }

    #[test]
    fn arb_enters_on_an_abnormal_spread() {
        let mut a = ArbCap::new(1.0, 20, 2.0, 0.5, 10.0);
        // 20 điểm ổn định quanh 0 (có dao động nhỏ để độ lệch chuẩn khác 0)
        for i in 0..20 { a.update(10_000 + (i % 3), 10_000); }
        // rồi một cú giãn mạnh
        let th = a.update(10_100, 10_000);
        assert_eq!(th, SignalCap::MoDaiB, "A đắt bất thường → bán A bid B");
        assert_eq!(a.is_open, Some(SignalCap::MoDaiB));
    }

    #[test]
    fn arb_never_opens_two_positions_at_once() {
        let mut a = ArbCap::new(1.0, 20, 2.0, 0.5, 100.0);
        for i in 0..20 { a.update(10_000 + (i % 3), 10_000); }
        assert_ne!(a.update(10_100, 10_000), SignalCap::KhongLam);
        for _ in 0..5 {
            let t = a.update(10_120, 10_000);
            assert!(matches!(t, SignalCap::KhongLam | SignalCap::Dong),
                    "đang có vị thế thì không được mở thêm");
        }
    }

    #[test]
    fn arb_stops_out_beyond_the_threshold() {
        // Bài học sống còn: chênh lệch giãn mãi nghĩa là quan hệ ĐÃ GÃY,
        // không phải "cơ hội càng tốt hơn". Phải thoát.
        let mut a = ArbCap::new(1.0, 20, 2.0, 0.5, 3.0);
        for i in 0..20 { a.update(10_000 + (i % 3), 10_000); }
        a.update(10_050, 10_000); // vào lệnh
        assert!(a.is_open.is_some());
        let t = a.update(10_500, 10_000); // giãn cực mạnh
        assert_eq!(t, SignalCap::Dong, "vượt ngưỡng dừng phải CẮT LỖ");
        assert_eq!(a.is_open, None);
    }

    #[test]
    fn the_spread_uses_the_correct_hedge_ratio() {
        let a = ArbCap::new(1.5, 10, 2.0, 0.5, 4.0);
        assert!((a.spread(15_000, 10_000) - 0.0).abs() < 1e-9);
        assert!((a.spread(15_150, 10_000) - 150.0).abs() < 1e-9);
    }

    // ---------- Định cỡ ----------
    #[test]
    fn kelly_is_zero_without_an_edge() {
        assert_eq!(kelly_fraction(0.5, 1.0), 0.0, "tung đồng xu công bằng → đừng đánh");
        assert_eq!(kelly_fraction(0.4, 1.0), 0.0, "lợi thế âm → tuyệt đối đừng đánh");
        assert_eq!(kelly_fraction(0.3, 0.5), 0.0);
    }

    #[test]
    fn kelly_grows_with_the_edge() {
        let mut prev = 0.0;
        for p in [0.55, 0.60, 0.65, 0.70, 0.80] {
            let f = kelly_fraction(p, 1.0);
            assert!(f > prev, "lợi thế lớn hơn phải cho cỡ lớn hơn");
            assert!(f <= 1.0);
            prev = f;
        }
    }

    #[test]
    fn kelly_matches_the_textbook_value() {
        // 60% thắng, ăn 1 thua 1 → Kelly = 2p − 1 = 0.20
        assert!((kelly_fraction(0.60, 1.0) - 0.20).abs() < 1e-9);
        // 40% thắng, ăn 2 thua 1 → (0.4·2 − 0.6)/2 = 0.10
        assert!((kelly_fraction(0.40, 2.0) - 0.10).abs() < 1e-9);
    }

    #[test]
    fn fractional_kelly_is_always_below_full_kelly() {
        for p in [0.55, 0.60, 0.75] {
            let toan = kelly_fraction(p, 1.0);
            let part = fractional_kelly(p, 1.0, 0.25);
            assert!(part < toan);
            assert!((part - toan * 0.25).abs() < 1e-9);
        }
    }

    #[test]
    fn kelly_never_divides_by_zero() {
        assert_eq!(kelly_fraction(0.9, 0.0), 0.0);
        assert_eq!(kelly_fraction(0.9, -1.0), 0.0);
    }

    #[test]
    fn has_theo_volatility_down_when_volatility_up() {
        let von = 1_000_000i64;
        let a = has_theo_volatility(von, 0.10, 0.10, 100);
        let b = has_theo_volatility(von, 0.10, 0.40, 100);
        assert!(b < a, "mã dao động mạnh gấp 4 thì bid ít hơn hẳn");
        assert_eq!(a, 10_000, "biến động khớp mục tiêu → dùng toàn bộ vốn");
        assert_eq!(b, 2_500, "gấp 4 lần biến động → 1/4 tỉ trọng");
    }

    #[test]
    fn vol_sizing_never_levers_beyond_capital() {
        // Mã êm hơn mục tiêu KHÔNG được dẫn tới bid vượt vốn.
        let c = has_theo_volatility(1_000_000, 0.40, 0.05, 100);
        assert_eq!(c, 10_000, "tỉ trọng bị chặn ở 1.0, không dùng đòn bẩy ngầm");
    }

    #[test]
    fn vol_sizing_is_safe_on_bad_input() {
        assert_eq!(has_theo_volatility(1_000_000, 0.1, 0.0, 100), 0);
        assert_eq!(has_theo_volatility(1_000_000, 0.1, 0.1, 0), 0);
        assert_eq!(has_theo_volatility(1_000_000, 0.1, -0.5, 100), 0);
    }

    // ---------- Thước đo rủi ro ----------
    #[test]
    fn a_monotonic_equity_curve_has_no_drawdown() {
        let d: Vec<i64> = (0..50).map(|i| 100_000 + i * 100).collect();
        let r = risk_level(&d);
        assert_eq!(r.max_drawdown, 0);
        assert_eq!(r.num_session_lo, 0);
        assert_eq!(r.total_pnl, 4_900);
    }

    #[test]
    fn drawdown_measures_distance_from_the_peak() {
        let d = vec![100, 150, 120, 80, 130];
        let r = risk_level(&d);
        assert_eq!(r.max_drawdown, 70, "từ đỉnh 150 xuống đáy 80");
    }

    #[test]
    fn drawdown_is_never_negative() {
        for hat in [1u64, 7, 42] {
            let mut s = hat;
            let d: Vec<i64> = (0..200).map(|_| {
                s = s.wrapping_mul(6364136223846793005).wrapping_add(1);
                ((s >> 40) % 200_000) as i64
            }).collect();
            assert!(risk_level(&d).max_drawdown >= 0);
        }
    }

    #[test]
    fn a_smooth_curve_has_a_higher_sharpe() {
        // Cùng đích đến, nhưng đường êm mới là đường người ta đi hết được.
        // Đường "êm" vẫn phải có dao động nhỏ: đường thẳng tuyệt đối cho độ
        // lệch chuẩn 0, và khi đó Sharpe không định nghĩa được (ta trả 0).
        let em: Vec<i64> = (0..100).map(|i| 100_000 + i * 500 + (i % 5) * 40).collect();
        let mut xoc = Vec::new();
        let mut v = 100_000i64;
        for i in 0..100 { v += if i % 3 == 0 { -8_000 } else { 5_750 }; xoc.push(v); }
        let (a, b) = (risk_level(&em), risk_level(&xoc));
        assert!(a.sharpe_ratio > b.sharpe_ratio,
                "êm {:.2} phải high hơn xóc {:.2}", a.sharpe_ratio, b.sharpe_ratio);
        assert!(b.max_drawdown > a.max_drawdown);
    }

    #[test]
    fn a_very_short_curve_does_not_panic() {
        assert_eq!(risk_level(&[]).total_pnl, 0);
        assert_eq!(risk_level(&[100]).max_drawdown, 0);
        assert_eq!(risk_level(&[100, 100]).sharpe_ratio, 0.0, "không dao động → Sharpe 0");
    }

    // ---------- Sinh dữ liệu ----------
    #[test]
    fn pair_generation_is_deterministic() {
        assert_eq!(gen_cap_price(100, 5, 1.5), gen_cap_price(100, 5, 1.5));
        assert_ne!(gen_cap_price(100, 5, 1.5), gen_cap_price(100, 6, 1.5));
    }

    #[test]
    fn the_two_series_really_do_move_together() {
        // Nếu chúng không đồng biến thì cả chương arbitrage cặp là vô nghĩa.
        let (a, b) = gen_cap_price(2_000, 2024, 1.5);
        let n = a.len() as f64;
        let (ta, tb) = (a.iter().sum::<i64>() as f64 / n, b.iter().sum::<i64>() as f64 / n);
        let mut tu = 0.0;
        let (mut sa, mut sb) = (0.0, 0.0);
        for i in 0..a.len() {
            let (da, db) = (a[i] as f64 - ta, b[i] as f64 - tb);
            tu += da * db; sa += da * da; sb += db * db;
        }
        let correlation = tu / (sa.sqrt() * sb.sqrt());
        assert!(correlation > 0.8, "tương quan {:.3} phải high", correlation);
    }
}
