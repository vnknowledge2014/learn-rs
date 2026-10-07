//! Chương 69 — Hệ thống giao dịch thuật toán: sổ lệnh, động cơ khớp lệnh,
//! quản trị rủi ro bằng kiểu, và bộ kiểm định chiến lược trên dữ liệu quá khứ.
//!
//! Đây là phần LÕI của một nền tảng kiểu OpenAlgo — nhưng viết bằng Rust, nơi
//! không có bộ dọn rác chen ngang, nên độ trễ đuôi do chính mã của bạn quyết định
//! chứ không phải trung bình đẹp kèm những cú khựng bất chợt.
//!
//! ⚠️ Đây là tài liệu KỸ THUẬT, không phải lời khuyên đầu tư. Mọi số liệu đều
//! là dữ liệu giả lập tất định.

use std::collections::{BTreeMap, VecDeque};
use std::marker::PhantomData;

// ============================================================================
// 1. TIỀN LÀ SỐ NGUYÊN — sai lầm đắt giá nhất của người mới
// ============================================================================

/// KHÔNG BAO GIỜ dùng `f64` cho tiền. `0.1 + 0.2 != 0.3` trong nhị phân, và
/// sai số một xu nhân với triệu lệnh là một vụ kiện. Ngành tài chính dùng
/// SỐ NGUYÊN đơn vị nhỏ nhất — ở đây là "tick", 1 tick = 0,01 đơn vị tiền.
pub type Price = i64; // tính bằng tick
pub type Quantity = i64;
pub type OrderId = u64;

pub fn tick_to_string(t: Price) -> String {
    // Tách dấu RIÊNG: với t = -5, `t / 100` = 0 nên sẽ mất dấu nếu in thẳng.
    let sign = if t < 0 { "-" } else { "" };
    let abs = t.unsigned_abs();
    format!("{sign}{}.{:02}", abs / 100, abs % 100)
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
pub enum Side {
    Buy,
    Sell,
}

impl Side {
    pub fn opposite(self) -> Side {
        match self {
            Side::Buy => Side::Sell,
            Side::Sell => Side::Buy,
        }
    }
    /// Dấu của vị thế: mua làm vị thế tăng, bán làm giảm.
    pub fn sign(self) -> i64 {
        match self {
            Side::Buy => 1,
            Side::Sell => -1,
        }
    }
}

// ============================================================================
// 2. VÒNG ĐỜI LỆNH BẰNG TYPESTATE — trạng thái nằm trong KIỂU
// ============================================================================
// Áp dụng Chương 20 vào nghiệp vụ thật: gửi hai lần cùng một lệnh, hoặc hủy
// một lệnh đã khớp hết, là những lỗi tốn tiền. Ở đây chúng KHÔNG BIÊN DỊCH ĐƯỢC.

// Ba nhãn trạng thái. Chúng là kiểu RỖNG — không chiếm một byte nào lúc chạy;
// toàn bộ tác dụng của chúng diễn ra trong trình biên dịch.
#[derive(Debug, Clone, Copy)]
pub struct Draft;
#[derive(Debug, Clone, Copy)]
pub struct RiskChecked;
#[derive(Debug, Clone, Copy)]
pub struct Sent;

#[derive(Debug, Clone)]
pub struct Order<State> {
    pub id: OrderId,
    pub symbol: String,
    pub side: Side,
    pub price: Price,
    pub quantity: Quantity,
    pub filled: Quantity,
    _state: PhantomData<State>,
}

impl Order<Draft> {
    pub fn new(id: OrderId, symbol: &str, side: Side, price: Price, quantity: Quantity) -> Self {
        Order {
            id,
            symbol: symbol.to_string(),
            side,
            price,
            quantity,
            filled: 0,
            _state: PhantomData,
        }
    }
}

impl<S> Order<S> {
    pub fn remaining(&self) -> Quantity {
        self.quantity - self.filled
    }
    /// Riêng tư: mã NGOÀI module chỉ đổi được trạng thái qua `Limit::check`
    /// và `send`. (Hàm `main` và module test nằm cùng module nên gọi được —
    /// chúng dùng nó để dựng nhanh lệnh mẫu.)
    fn transition<Next>(self) -> Order<Next> {
        Order {
            id: self.id,
            symbol: self.symbol,
            side: self.side,
            price: self.price,
            quantity: self.quantity,
            filled: self.filled,
            _state: PhantomData,
        }
    }
}

// Chỉ lệnh ĐÃ QUA kiểm tra rủi ro mới gửi được vào sổ lệnh.
impl Order<RiskChecked> {
    pub fn send(self) -> Order<Sent> {
        self.transition()
    }
}

// ============================================================================
// 3. KIỂM TRA RỦI RO — cổng bắt buộc trước khi lệnh ra thị trường
// ============================================================================

#[derive(Debug, Clone, PartialEq)]
pub enum RiskError {
    NonPositiveQuantity(Quantity),
    NonPositivePrice(Price),
    ExceedsMaxValue { value: i64, limit: i64 },
    ExceedsMaxPosition { position_after: i64, limit: i64 },
    UnknownSymbol(String),
}

pub struct Limit {
    pub max_order_value: i64,
    pub max_position: i64,
    pub allowed_symbols: Vec<String>,
}

impl Limit {
    /// Trả `Result` chứ không panic: từ chối lệnh là chuyện BÌNH THƯỜNG,
    /// không phải lỗi lập trình. Đây là ranh giới "parse, đừng validate".
    pub fn check(
        &self,
        order: Order<Draft>,
        current_position: i64,
    ) -> Result<Order<RiskChecked>, RiskError> {
        if order.quantity <= 0 {
            return Err(RiskError::NonPositiveQuantity(order.quantity));
        }
        if order.price <= 0 {
            return Err(RiskError::NonPositivePrice(order.price));
        }
        if !self.allowed_symbols.contains(&order.symbol) {
            return Err(RiskError::UnknownSymbol(order.symbol.clone()));
        }
        // `checked_mul`: giá × số lượng khổng lồ có thể tràn i64 — coi như vượt trần.
        let value = order.price.checked_mul(order.quantity).unwrap_or(i64::MAX);
        if value > self.max_order_value {
            return Err(RiskError::ExceedsMaxValue {
                value,
                limit: self.max_order_value,
            });
        }
        let position_after = current_position + order.side.sign() * order.quantity;
        if position_after.abs() > self.max_position {
            return Err(RiskError::ExceedsMaxPosition {
                position_after,
                limit: self.max_position,
            });
        }
        Ok(order.transition())
    }
}

// ============================================================================
// 4. SỔ LỆNH & ĐỘNG CƠ KHỚP LỆNH
// ============================================================================

#[derive(Debug, Clone, PartialEq)]
pub struct Fill {
    pub order_aggressive: OrderId,
    pub passive_order: OrderId,
    pub price: Price,
    pub quantity: Quantity,
}

/// Sổ lệnh giới hạn. `BTreeMap` cho phép lấy giá tốt nhất trong O(log n) và
/// duyệt các mức giá theo THỨ TỰ — đúng thứ động cơ khớp lệnh cần.
/// `VecDeque` ở mỗi mức giá giữ ưu tiên THỜI GIAN: ai đặt trước khớp trước.
#[derive(Default)]
pub struct OrderBook {
    /// Bên mua: khóa là giá ÂM để `BTreeMap` (vốn tăng dần) trả giá CAO nhất trước.
    buy_side: BTreeMap<Price, VecDeque<Order<Sent>>>,
    sell_side: BTreeMap<Price, VecDeque<Order<Sent>>>,
}

impl OrderBook {
    pub fn new() -> Self {
        Self::default()
    }

    /// Giá mua cao nhất — cái giá tốt nhất mà người bán có thể nhận ngay.
    pub fn best_bid(&self) -> Option<Price> {
        self.buy_side.keys().next().map(|k| -k)
    }
    /// Giá bán thấp nhất.
    pub fn best_ask(&self) -> Option<Price> {
        self.sell_side.keys().next().copied()
    }
    /// Chênh lệch mua-bán: chi phí ẩn của mọi giao dịch.
    pub fn spread(&self) -> Option<Price> {
        Some(self.best_ask()? - self.best_bid()?)
    }
    /// Giá giữa — ước lượng "giá trị thật" tốt hơn giá khớp gần nhất.
    pub fn mid(&self) -> Option<Price> {
        Some((self.best_ask()? + self.best_bid()?) / 2)
    }
    pub fn qty_at(&self, side: Side, price: Price) -> Quantity {
        let side_map = match side {
            Side::Buy => &self.buy_side,
            Side::Sell => &self.sell_side,
        };
        let key = match side {
            Side::Buy => -price,
            Side::Sell => price,
        };
        side_map
            .get(&key)
            .map_or(0, |q| q.iter().map(|order| order.remaining()).sum())
    }
    pub fn total_order_book(&self) -> usize {
        self.buy_side.values().map(|q| q.len()).sum::<usize>()
            + self.sell_side.values().map(|q| q.len()).sum::<usize>()
    }

    /// Nạp lệnh và khớp ngay phần khớp được; phần dư nằm lại sổ.
    /// Đây là trái tim của sàn: ƯU TIÊN GIÁ trước, rồi ƯU TIÊN THỜI GIAN.
    pub fn submit(&mut self, mut order: Order<Sent>) -> Vec<Fill> {
        let mut fills = Vec::new();
        let contra_is_sell = order.side == Side::Buy;

        loop {
            if order.remaining() == 0 {
                break;
            }
            // Mức giá đối ứng tốt nhất còn khớp được với giá giới hạn của ta?
            let best = {
                let contra_side = if contra_is_sell {
                    &self.sell_side
                } else {
                    &self.buy_side
                };
                match contra_side.keys().next().copied() {
                    Some(k) => {
                        let level_price = if contra_is_sell { k } else { -k };
                        let crosses = if contra_is_sell {
                            level_price <= order.price
                        } else {
                            level_price >= order.price
                        };
                        if crosses {
                            Some((k, level_price))
                        } else {
                            None
                        }
                    }
                    None => None,
                }
            };
            let (key, fill_price) = match best {
                Some(x) => x,
                None => break,
            };

            let contra_side = if contra_is_sell {
                &mut self.sell_side
            } else {
                &mut self.buy_side
            };
            let queue = contra_side.get_mut(&key).unwrap();
            while order.remaining() > 0 {
                let head = match queue.front_mut() {
                    Some(d) => d,
                    None => break,
                };
                let amount = order.remaining().min(head.remaining());
                order.filled += amount;
                head.filled += amount;
                fills.push(Fill {
                    order_aggressive: order.id,
                    passive_order: head.id,
                    // Giá khớp là giá của lệnh ĐÃ NẰM SẴN trong sổ — người
                    // đến sau được hưởng giá tốt hơn nếu có. Đây là quy tắc
                    // "cải thiện giá" của mọi sàn nghiêm túc.
                    price: fill_price,
                    quantity: amount,
                });
                if head.remaining() == 0 {
                    queue.pop_front();
                }
            }
            if queue.is_empty() {
                contra_side.remove(&key);
            }
        }

        if order.remaining() > 0 {
            let key = if order.side == Side::Buy {
                -order.price
            } else {
                order.price
            };
            let side_map = if order.side == Side::Buy {
                &mut self.buy_side
            } else {
                &mut self.sell_side
            };
            side_map.entry(key).or_default().push_back(order);
        }
        fills
    }

    pub fn cancel(&mut self, id: OrderId) -> bool {
        for side_map in [&mut self.buy_side, &mut self.sell_side] {
            let mut emptied = None;
            for (key, queue) in side_map.iter_mut() {
                if let Some(i) = queue.iter().position(|order| order.id == id) {
                    queue.remove(i);
                    if queue.is_empty() {
                        emptied = Some(*key);
                    }
                    if let Some(k) = emptied {
                        side_map.remove(&k);
                    }
                    return true;
                }
            }
        }
        false
    }
}

// ============================================================================
// 5. VỊ THẾ & LÃI/LỖ — một VỊ NHÓM (Chương 18) trá hình
// ============================================================================

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct Position {
    pub quantity: i64,
    /// Tiền mặt tính bằng tick. Mua làm tiền giảm, bán làm tiền tăng.
    pub cash: i64,
}

impl Position {
    pub const EMPTY: Position = Position {
        quantity: 0,
        cash: 0,
    };

    /// Phép `compose` này KẾT HỢP và có ĐƠN VỊ `EMPTY` → đúng định nghĩa vị nhóm.
    /// Nhờ vậy có thể gộp lãi/lỗ song song bằng `rayon` mà kết quả không đổi.
    pub fn compose(self, k: Position) -> Position {
        Position {
            quantity: self.quantity + k.quantity,
            cash: self.cash + k.cash,
        }
    }
    pub fn from_fill(side: Side, price: Price, quantity: Quantity) -> Position {
        Position {
            quantity: side.sign() * quantity,
            cash: -side.sign() * price * quantity,
        }
    }
    /// Giá trị ròng khi định giá lại theo giá thị trường hiện tại.
    pub fn net_value(&self, market_price: Price) -> i64 {
        self.cash + self.quantity * market_price
    }
}

// ============================================================================
// 6. BỘ KIỂM ĐỊNH CHIẾN LƯỢC (backtest) — hàm thuần túy trên lịch sử
// ============================================================================

#[derive(Debug, Clone, Copy, PartialEq)]
pub struct Candle {
    pub timestamp: u64,
    pub open: Price,
    pub high: Price,
    pub low: Price,
    pub close: Price,
}

#[derive(Debug, Clone, Copy, PartialEq)]
pub enum Signal {
    Buy(Quantity),
    Sell(Quantity),
    Hold,
}

/// Chiến lược là một HÀM THUẦN TÚY: cùng lịch sử → cùng tín hiệu, luôn luôn.
/// Nhờ tính chất này mà kết quả kiểm định tái lập được 100%.
pub trait Strategy {
    fn name(&self) -> &str;
    fn decide(&mut self, history: &[Candle], position: &Position) -> Signal;
}

/// Giao cắt trung bình động: kinh điển, dễ hiểu, và cố tình đơn giản.
pub struct MeanCross {
    pub fast: usize,
    pub slow: usize,
    pub lot_size: Quantity,
}

fn mean(candle: &[Candle], n: usize) -> Option<Price> {
    if candle.len() < n {
        return None;
    }
    Some(
        candle[candle.len() - n..]
            .iter()
            .map(|c| c.close)
            .sum::<Price>()
            / n as Price,
    )
}

impl Strategy for MeanCross {
    fn name(&self) -> &str {
        "Giao cắt trung bình động"
    }
    fn decide(&mut self, history: &[Candle], position: &Position) -> Signal {
        let (fast, slow) = match (mean(history, self.fast), mean(history, self.slow)) {
            (Some(a), Some(b)) => (a, b),
            _ => return Signal::Hold, // chưa đủ dữ liệu — KHÔNG đoán mò
        };
        if fast > slow && position.quantity <= 0 {
            Signal::Buy(self.lot_size)
        } else if fast < slow && position.quantity > 0 {
            Signal::Sell(position.quantity)
        } else {
            Signal::Hold
        }
    }
}

#[derive(Debug, PartialEq)]
pub struct BacktestResult {
    pub last_position: Position,
    pub last_value: i64,
    pub num_trades: usize,
    /// Mức sụt giảm sâu nhất từ đỉnh — con số quan trọng hơn cả lợi nhuận,
    /// vì nó quyết định bạn có chịu nổi để đi hết chiến lược hay không.
    pub max_drawdown: i64,
    pub equity_curve: Vec<i64>,
}

/// Chạy kiểm định. Có mô hình TRƯỢT GIÁ và PHÍ — bỏ hai thứ này là cách
/// nhanh nhất để tự lừa mình bằng một đường vốn đẹp nhưng không có thật.
pub fn run_backtest(
    data: &[Candle],
    strategy: &mut dyn Strategy,
    slippage_ticks: Price,
    fee_per_unit: i64,
) -> BacktestResult {
    let mut position = Position::EMPTY;
    let mut num_trades = 0;
    let mut equity_curve = Vec::with_capacity(data.len());
    let mut peak = i64::MIN;
    let mut max_dd = 0;

    // Lệnh quyết định ở nến i-1 chờ khớp ở GIÁ MỞ của nến i.
    let mut pending: Option<(Side, Quantity)> = None;
    for (i, candle) in data.iter().enumerate() {
        if let Some((side, amount)) = pending.take()
            && amount > 0
        {
            // Trượt giá: ta luôn mua đắt hơn và bán rẻ hơn giá lý thuyết.
            let price = candle.open + side.sign() * slippage_ticks;
            position = position.compose(Position::from_fill(side, price, amount));
            position.cash -= fee_per_unit * amount;
            num_trades += 1;
        }
        // Định giá lại và cập nhật sụt giảm ở MỌI nến — kể cả nến đứng ngoài,
        // vì lúc giữ vị thế chính là lúc sụt giảm xảy ra.
        let equity = position.net_value(candle.close);
        equity_curve.push(equity);
        peak = peak.max(equity);
        max_dd = max_dd.max(peak - equity);

        // Quyết định dựa trên các nến ĐÃ ĐÓNG (tới nến i), khớp ở nến KẾ TIẾP.
        // Khớp ở giá đóng của chính nến i = "nhìn trộm tương lai", lỗi kinh
        // điển khiến mọi chiến lược trông như in tiền. Tín hiệu ở nến cuối
        // không có nến sau để khớp nên tự rơi mất.
        pending = match strategy.decide(&data[..=i], &position) {
            Signal::Buy(q) => Some((Side::Buy, q)),
            Signal::Sell(q) => Some((Side::Sell, q)),
            Signal::Hold => None,
        };
    }

    let last_price = data.last().map_or(0, |n| n.close);
    BacktestResult {
        last_value: position.net_value(last_price),
        last_position: position,
        num_trades,
        max_drawdown: max_dd,
        equity_curve,
    }
}

/// Sinh dữ liệu giá tất định (bước ngẫu nhiên có hạt giống cố định).
/// Tất định là điều kiện BẮT BUỘC để kiểm thử hồi quy có ý nghĩa.
pub fn gen_data(num_candles: usize, start_price: Price, seed: u64) -> Vec<Candle> {
    let mut s = seed;
    let mut price = start_price;
    (0..num_candles)
        .map(|i| {
            s = s
                .wrapping_mul(6364136223846793005)
                .wrapping_add(1442695040888963407);
            let step = ((s >> 33) % 41) as i64 - 20; // -20..+20 tick
            let open = price;
            price = (price + step).max(1);
            Candle {
                timestamp: i as u64,
                open,
                high: open.max(price) + 5,
                low: (open.min(price) - 5).max(1),
                close: price,
            }
        })
        .collect()
}

fn main() {
    println!("═══════════════════════════════════════════════════════════");
    println!("   HỆ THỐNG GIAO DỊCH: SỔ LỆNH · KHỚP LỆNH · KIỂM ĐỊNH     ");
    println!("═══════════════════════════════════════════════════════════");

    println!("\n1. VÌ SAO TIỀN PHẢI LÀ SỐ NGUYÊN");
    let sum: f64 = (0..10).map(|_| 0.1f64).sum();
    println!("   Cộng 0.1 mười lần bằng f64 → {:.20}", sum);
    println!("   Bằng nhau với 1.0?          → {}", sum == 1.0);
    println!(
        "   Bằng số nguyên tick         → {} tick = {}",
        100,
        tick_to_string(100)
    );

    println!("\n2. CỔNG RỦI RO");
    let limits = Limit {
        max_order_value: 1_000_000,
        max_position: 500,
        allowed_symbols: vec!["VNM".into(), "FPT".into()],
    };
    for (description, order) in [
        (
            "hợp lệ         ",
            Order::new(1, "VNM", Side::Buy, 8_500, 100),
        ),
        (
            "mã lạ          ",
            Order::new(2, "XYZ", Side::Buy, 8_500, 100),
        ),
        (
            "quá to         ",
            Order::new(3, "VNM", Side::Buy, 8_500, 1_000),
        ),
        (
            "số lượng âm    ",
            Order::new(4, "VNM", Side::Buy, 8_500, -5),
        ),
    ] {
        match limits.check(order, 0) {
            Ok(_) => println!("   {} → CHO QUA", description),
            Err(e) => println!("   {} → CHẶN: {:?}", description, e),
        }
    }

    println!("\n3. SỔ LỆNH & ƯU TIÊN GIÁ–THỜI GIAN");
    let mut book = OrderBook::new();
    let send = |id, side, price, qty| {
        Order::<Draft>::new(id, "VNM", side, price, qty)
            .transition::<RiskChecked>()
            .send()
    };
    for (id, price, qty) in [
        (10u64, 8_400i64, 100i64),
        (11, 8_400, 200),
        (12, 8_390, 500),
    ] {
        book.submit(send(id, Side::Buy, price, qty));
    }
    for (id, price, qty) in [(20u64, 8_420i64, 150i64), (21, 8_430, 300)] {
        book.submit(send(id, Side::Sell, price, qty));
    }
    println!(
        "   Mua tốt nhất {} · Bán tốt nhất {} · Chênh lệch {} tick",
        tick_to_string(book.best_bid().unwrap()),
        tick_to_string(book.best_ask().unwrap()),
        book.spread().unwrap()
    );
    println!(
        "   Khối lượng chờ mua ở {}: {}",
        tick_to_string(8_400),
        book.qty_at(Side::Buy, 8_400)
    );

    println!("\n4. KHỚP LỆNH — lệnh bán 250 quét qua bên mua");
    let fill = book.submit(send(30, Side::Sell, 8_390, 250));
    for k in &fill {
        println!(
            "   {} đơn vị @ {} (đối tác lệnh #{})",
            k.quantity,
            tick_to_string(k.price),
            k.passive_order
        );
    }
    println!("   → Lệnh #10 (đặt trước) khớp hết TRƯỚC lệnh #11, dù cùng giá.");
    println!(
        "   → Khớp ở giá {} chứ không phải {} — người đến sau được cải thiện giá.",
        tick_to_string(8_400),
        tick_to_string(8_390)
    );

    println!("\n5. VỊ THẾ LÀ MỘT VỊ NHÓM");
    let a = Position::from_fill(Side::Buy, 8_400, 100);
    let b = Position::from_fill(Side::Sell, 8_500, 60);
    println!("   Mua 100@84.00 rồi bán 60@85.00 → {:?}", a.compose(b));
    println!(
        "   Kết hợp: (a·b)·c == a·(b·c) → {}",
        a.compose(b).compose(Position::EMPTY) == a.compose(b.compose(Position::EMPTY))
    );

    println!("\n6. KIỂM ĐỊNH CHIẾN LƯỢC — 500 nến, có phí và trượt giá");
    let data = gen_data(500, 8_000, 42);
    for (slippage, fee) in [(0i64, 0i64), (2, 3)] {
        let mut strategy = MeanCross {
            fast: 5,
            slow: 20,
            lot_size: 100,
        };
        let result = run_backtest(&data, &mut strategy, slippage, fee);
        println!(
            "   trượt {} tick, phí {}/đv → lãi {:>8} tick · {} lệnh · sụt sâu nhất {} tick",
            slippage, fee, result.last_value, result.num_trades, result.max_drawdown
        );
    }
    println!("   → Cùng một chiến lược: bỏ qua phí và trượt giá là tự lừa mình.");

    println!("\n═══════════════════════════════════════════════════════════");
    println!("   KHÔNG CÓ GC CHEN NGANG — LÝ DO NGÀNH NÀY CHỌN RUST       ");
    println!("═══════════════════════════════════════════════════════════");
}

#[cfg(test)]
mod tests {
    use super::*;

    fn sent_order(id: OrderId, side: Side, price: Price, qty: Quantity) -> Order<Sent> {
        Order::<Draft>::new(id, "VNM", side, price, qty)
            .transition::<RiskChecked>()
            .send()
    }

    // ---------- Tiền & kiểu ----------
    #[test]
    fn integer_money_has_no_drift() {
        let f64_sum: f64 = (0..1000).map(|_| 0.01f64).sum();
        assert_ne!(
            f64_sum, 10.0,
            "f64 KHÔNG cộng đúng — đây là lý do không dùng nó cho tiền"
        );
        let tick_sum: i64 = (0..1000).map(|_| 1i64).sum();
        assert_eq!(tick_sum, 1000, "số nguyên thì chính xác tuyệt đối");
    }

    #[test]
    fn tick_display_handles_negatives() {
        assert_eq!(tick_to_string(8_450), "84.50");
        assert_eq!(tick_to_string(5), "0.05");
        assert_eq!(tick_to_string(-8_450), "-84.50");
        assert_eq!(
            tick_to_string(-5),
            "-0.05",
            "dấu âm không được mất khi |t| < 100"
        );
    }

    // ---------- Rủi ro ----------
    #[test]
    fn risk_gate_blocks_each_violation_kind() {
        let limits = Limit {
            max_order_value: 1_000_000,
            max_position: 500,
            allowed_symbols: vec!["VNM".into()],
        };
        // Dùng `unwrap_err()` chứ không `assert_eq!` cả `Result`: `Order` không
        // cài `PartialEq` (so sánh hai lệnh theo giá trị là vô nghĩa — mỗi lệnh
        // có danh tính riêng qua `id`).
        assert!(
            limits
                .check(Order::new(1, "VNM", Side::Buy, 8_500, 100), 0)
                .is_ok()
        );
        assert_eq!(
            limits
                .check(Order::new(2, "VNM", Side::Buy, 8_500, 0), 0)
                .unwrap_err(),
            RiskError::NonPositiveQuantity(0)
        );
        assert_eq!(
            limits
                .check(Order::new(3, "VNM", Side::Buy, 0, 10), 0)
                .unwrap_err(),
            RiskError::NonPositivePrice(0)
        );
        assert_eq!(
            limits
                .check(Order::new(4, "XYZ", Side::Buy, 100, 10), 0)
                .unwrap_err(),
            RiskError::UnknownSymbol("XYZ".into())
        );
        assert!(matches!(
            limits
                .check(Order::new(5, "VNM", Side::Buy, 8_500, 1_000), 0)
                .unwrap_err(),
            RiskError::ExceedsMaxValue { .. }
        ));
    }

    #[test]
    fn position_limit_covers_the_short_side_too() {
        let limits = Limit {
            max_order_value: i64::MAX,
            max_position: 100,
            allowed_symbols: vec!["VNM".into()],
        };
        // bán khống 150 khi đang giữ 0 → vị thế -150, vượt trần 100
        assert_eq!(
            limits
                .check(Order::new(1, "VNM", Side::Sell, 100, 150), 0)
                .unwrap_err(),
            RiskError::ExceedsMaxPosition {
                position_after: -150,
                limit: 100
            }
        );
        // nhưng bán 150 khi đang giữ 100 → còn -50, hợp lệ
        assert!(
            limits
                .check(Order::new(2, "VNM", Side::Sell, 100, 150), 100)
                .is_ok()
        );
    }

    // ---------- Sổ lệnh ----------
    #[test]
    fn book_reports_best_on_both_sides() {
        let mut s = OrderBook::new();
        s.submit(sent_order(1, Side::Buy, 100, 10));
        s.submit(sent_order(2, Side::Buy, 105, 10)); // giá cao hơn = tốt hơn cho bên mua
        s.submit(sent_order(3, Side::Sell, 120, 10));
        s.submit(sent_order(4, Side::Sell, 110, 10)); // giá thấp hơn = tốt hơn cho bên bán
        assert_eq!(s.best_bid(), Some(105));
        assert_eq!(s.best_ask(), Some(110));
        assert_eq!(s.spread(), Some(5));
        assert_eq!(s.mid(), Some(107));
    }

    #[test]
    fn non_crossing_order_rests_on_book() {
        let mut s = OrderBook::new();
        assert!(s.submit(sent_order(1, Side::Buy, 100, 10)).is_empty());
        assert!(s.submit(sent_order(2, Side::Sell, 110, 10)).is_empty());
        assert_eq!(s.total_order_book(), 2);
    }

    #[test]
    fn time_priority_within_a_price_level() {
        let mut s = OrderBook::new();
        s.submit(sent_order(1, Side::Buy, 100, 50)); // đến TRƯỚC
        s.submit(sent_order(2, Side::Buy, 100, 50)); // đến SAU
        let fill = s.submit(sent_order(3, Side::Sell, 100, 60));
        assert_eq!(fill.len(), 2);
        assert_eq!(fill[0].passive_order, 1, "lệnh đến trước phải khớp trước");
        assert_eq!(fill[0].quantity, 50);
        assert_eq!(fill[1].passive_order, 2);
        assert_eq!(fill[1].quantity, 10);
    }

    #[test]
    fn price_priority_beats_time_priority() {
        let mut s = OrderBook::new();
        s.submit(sent_order(1, Side::Buy, 100, 50)); // đến trước, giá THẤP hơn
        s.submit(sent_order(2, Side::Buy, 105, 50)); // đến sau, giá CAO hơn
        let fill = s.submit(sent_order(3, Side::Sell, 100, 10));
        assert_eq!(fill[0].passive_order, 2, "giá tốt hơn thắng, dù đến sau");
        assert_eq!(fill[0].price, 105);
    }

    #[test]
    fn later_arrival_gets_price_improvement() {
        let mut s = OrderBook::new();
        s.submit(sent_order(1, Side::Sell, 100, 10)); // ai đó chào bán rẻ
        // ta sẵn sàng mua tới 120, nhưng chỉ phải trả 100
        let fill = s.submit(sent_order(2, Side::Buy, 120, 10));
        assert_eq!(fill[0].price, 100, "khớp ở giá của lệnh nằm sẵn trong sổ");
    }

    #[test]
    fn large_order_sweeps_multiple_levels() {
        let mut s = OrderBook::new();
        s.submit(sent_order(1, Side::Sell, 100, 10));
        s.submit(sent_order(2, Side::Sell, 101, 10));
        s.submit(sent_order(3, Side::Sell, 102, 10));
        let fill = s.submit(sent_order(4, Side::Buy, 102, 25));
        assert_eq!(fill.len(), 3);
        assert_eq!(
            fill.iter().map(|k| k.price).collect::<Vec<_>>(),
            vec![100, 101, 102],
            "phải ăn từ giá tốt nhất trở đi"
        );
        assert_eq!(fill.iter().map(|k| k.quantity).sum::<i64>(), 25);
        assert_eq!(s.total_order_book(), 1, "mức 102 còn dư 5 đơn vị");
    }

    #[test]
    fn aggressive_remainder_rests_on_book() {
        let mut s = OrderBook::new();
        s.submit(sent_order(1, Side::Sell, 100, 10));
        let fill = s.submit(sent_order(2, Side::Buy, 100, 30));
        assert_eq!(fill.iter().map(|k| k.quantity).sum::<i64>(), 10);
        assert_eq!(
            s.best_bid(),
            Some(100),
            "20 đơn vị còn lại thành lệnh chờ mua"
        );
        assert_eq!(s.qty_at(Side::Buy, 100), 20);
    }

    #[test]
    fn quantity_is_conserved_across_fills() {
        // BẤT BIẾN SỐNG CÒN của mọi sàn: không đơn vị nào được sinh ra
        // hay biến mất trong quá trình khớp.
        let mut s = OrderBook::new();
        let mut submitted = 0i64;
        let mut filled = 0i64;
        for i in 0..60u64 {
            let side = if i % 2 == 0 { Side::Buy } else { Side::Sell };
            let price = 100 + ((i * 7) % 11) as i64 - 5;
            let qty = 10 + (i % 13) as i64;
            submitted += qty;
            filled += s
                .submit(sent_order(i, side, price, qty))
                .iter()
                .map(|k| k.quantity)
                .sum::<i64>();
        }
        let resting: i64 = [Side::Buy, Side::Sell]
            .iter()
            .flat_map(|&c| (80..=120).map(move |g| (c, g)))
            .map(|(c, g)| s.qty_at(c, g))
            .sum();
        // Mỗi lần khớp tiêu thụ khối lượng từ CẢ HAI phía
        assert_eq!(
            submitted - 2 * filled,
            resting,
            "khối lượng phải cân bằng tuyệt đối"
        );
    }

    #[test]
    fn cancel_removes_order_and_prunes_empty_level() {
        let mut s = OrderBook::new();
        s.submit(sent_order(1, Side::Buy, 100, 10));
        s.submit(sent_order(2, Side::Buy, 100, 20));
        assert!(s.cancel(1));
        assert_eq!(s.qty_at(Side::Buy, 100), 20);
        assert!(s.cancel(2));
        assert_eq!(s.best_bid(), None, "mức giá rỗng phải bị xóa khỏi sổ");
        assert!(!s.cancel(999), "hủy lệnh không tồn tại phải trả false");
    }

    #[test]
    fn empty_book_has_no_price_and_no_panic() {
        let s = OrderBook::new();
        assert_eq!(s.best_bid(), None);
        assert_eq!(s.spread(), None);
        assert_eq!(s.mid(), None);
        assert_eq!(s.total_order_book(), 0);
    }

    // ---------- Vị thế ----------
    #[test]
    fn position_obeys_monoid_laws() {
        let a = Position::from_fill(Side::Buy, 100, 10);
        let b = Position::from_fill(Side::Sell, 110, 5);
        let c = Position::from_fill(Side::Buy, 90, 3);
        assert_eq!(
            a.compose(b).compose(c),
            a.compose(b.compose(c)),
            "luật kết hợp"
        );
        assert_eq!(a.compose(Position::EMPTY), a, "luật đơn vị phải");
        assert_eq!(Position::EMPTY.compose(a), a, "luật đơn vị trái");
    }

    #[test]
    fn chunked_position_merge_agrees() {
        // Vì là vị nhóm, chia nhỏ rồi gộp lại (như khi dùng rayon) cho kết quả
        // Y HỆT tính tuần tự. Đây là bảo chứng toán học, không phải may mắn.
        let fill: Vec<Position> = (0..100)
            .map(|i| {
                let side = if i % 3 == 0 { Side::Sell } else { Side::Buy };
                Position::from_fill(side, 100 + i % 7, 1 + i % 5)
            })
            .collect();
        let sequential = fill.iter().fold(Position::EMPTY, |a, &b| a.compose(b));
        let chunked = fill
            .chunks(7)
            .map(|k| k.iter().fold(Position::EMPTY, |a, &b| a.compose(b)))
            .fold(Position::EMPTY, |a, b| a.compose(b));
        assert_eq!(sequential, chunked);
    }

    #[test]
    fn buy_then_sell_higher_is_profitable() {
        let v = Position::from_fill(Side::Buy, 8_000, 100).compose(Position::from_fill(
            Side::Sell,
            8_500,
            100,
        ));
        assert_eq!(v.quantity, 0, "đã đóng hết vị thế");
        assert_eq!(v.net_value(0), 50_000, "(8500-8000) × 100 tick");
    }

    #[test]
    fn open_position_is_marked_to_market() {
        let v = Position::from_fill(Side::Buy, 8_000, 100);
        assert_eq!(v.net_value(8_000), 0, "vừa mua xong thì hòa vốn");
        assert_eq!(v.net_value(8_100), 10_000, "giá lên 100 tick → lãi 10 000");
        assert_eq!(v.net_value(7_900), -10_000, "giá xuống thì lỗ đối xứng");
    }

    // ---------- Kiểm định ----------
    #[test]
    fn data_generation_is_seed_deterministic() {
        assert_eq!(gen_data(50, 8_000, 7), gen_data(50, 8_000, 7));
        assert_ne!(gen_data(50, 8_000, 7), gen_data(50, 8_000, 8));
    }

    #[test]
    fn generated_candles_are_well_formed() {
        for candle in gen_data(500, 8_000, 99) {
            assert!(
                candle.high >= candle.open && candle.high >= candle.close,
                "đỉnh phải cao nhất"
            );
            assert!(
                candle.low <= candle.open && candle.low <= candle.close,
                "đáy phải thấp nhất"
            );
            assert!(candle.low > 0, "giá không bao giờ âm");
        }
    }

    #[test]
    fn strategy_stays_silent_until_warm() {
        let mut strategy = MeanCross {
            fast: 5,
            slow: 20,
            lot_size: 100,
        };
        let few_candles = gen_data(10, 8_000, 1);
        assert_eq!(
            strategy.decide(&few_candles, &Position::EMPTY),
            Signal::Hold,
            "chưa đủ 20 nến thì KHÔNG được đoán mò"
        );
    }

    #[test]
    fn backtest_is_fully_reproducible() {
        let data = gen_data(300, 8_000, 42);
        let run = || {
            let mut strategy = MeanCross {
                fast: 5,
                slow: 20,
                lot_size: 100,
            };
            run_backtest(&data, &mut strategy, 2, 3)
        };
        assert_eq!(
            run(),
            run(),
            "cùng dữ liệu + cùng chiến lược = cùng kết quả, luôn luôn"
        );
    }

    #[test]
    fn fees_and_slippage_always_hurt() {
        let data = gen_data(400, 8_000, 2024);
        let mut strategy1 = MeanCross {
            fast: 5,
            slow: 20,
            lot_size: 100,
        };
        let ideal = run_backtest(&data, &mut strategy1, 0, 0);
        let mut strategy2 = MeanCross {
            fast: 5,
            slow: 20,
            lot_size: 100,
        };
        let actual = run_backtest(&data, &mut strategy2, 2, 3);
        assert_eq!(ideal.num_trades, actual.num_trades, "cùng số lệnh");
        assert!(
            actual.last_value < ideal.last_value,
            "chi phí giao dịch luôn ăn vào lợi nhuận: {} so với {}",
            actual.last_value,
            ideal.last_value
        );
    }

    #[test]
    fn max_drawdown_is_never_negative() {
        for seed in [1u64, 7, 42, 2024, 31337] {
            let data = gen_data(200, 8_000, seed);
            let mut strategy = MeanCross {
                fast: 3,
                slow: 10,
                lot_size: 50,
            };
            let result = run_backtest(&data, &mut strategy, 1, 1);
            assert!(
                result.max_drawdown >= 0,
                "sụt giảm là khoảng cách, không thể âm"
            );
            assert_eq!(result.equity_curve.len(), data.len());
        }
    }

    #[test]
    fn drawdown_is_tracked_while_holding() {
        // Mua một lần rồi GIỮ trong khi giá rơi: sụt giảm phải được ghi nhận
        // ở các nến giữ vị thế (trước đây vòng lặp bỏ qua chúng).
        struct BuyOnce(bool);
        impl Strategy for BuyOnce {
            fn name(&self) -> &str {
                "mua một lần"
            }
            fn decide(&mut self, _: &[Candle], _: &Position) -> Signal {
                if self.0 {
                    Signal::Hold
                } else {
                    self.0 = true;
                    Signal::Buy(10)
                }
            }
        }
        let falling: Vec<Candle> = (0..10)
            .map(|i| {
                let p = 1_000 - 10 * i as Price;
                Candle {
                    timestamp: i,
                    open: p,
                    high: p,
                    low: p,
                    close: p,
                }
            })
            .collect();
        let result = run_backtest(&falling, &mut BuyOnce(false), 0, 0);
        // mua 10 ở giá mở nến 1 (990), giá đóng nến cuối 910 → lỗ 800, sụt đúng 800
        assert_eq!(result.last_value, -800);
        assert_eq!(result.max_drawdown, 800);
    }

    #[test]
    fn no_trades_means_no_pnl() {
        struct NoOp;
        impl Strategy for NoOp {
            fn name(&self) -> &str {
                "đứng ngoài"
            }
            fn decide(&mut self, _: &[Candle], _: &Position) -> Signal {
                Signal::Hold
            }
        }
        let data = gen_data(200, 8_000, 5);
        let result = run_backtest(&data, &mut NoOp, 5, 10);
        assert_eq!(result.num_trades, 0);
        assert_eq!(
            result.last_value, 0,
            "không vào lệnh thì không thể mất tiền"
        );
        assert_eq!(result.max_drawdown, 0);
    }

    #[test]
    fn strategy_cannot_peek_at_the_future() {
        // Nếu bộ kiểm định khớp ở giá ĐÓNG của chính cây nến ra tín hiệu,
        // ta đã dùng thông tin chưa tồn tại. Ở đây khớp ở giá MỞ của nến kế
        // tiếp, nên nến CUỐI CÙNG không thể sinh giao dịch nào.
        let data = gen_data(30, 8_000, 3);
        struct AlwaysBuy;
        impl Strategy for AlwaysBuy {
            fn name(&self) -> &str {
                "luôn mua"
            }
            fn decide(&mut self, _: &[Candle], _: &Position) -> Signal {
                Signal::Buy(1)
            }
        }
        let result = run_backtest(&data, &mut AlwaysBuy, 0, 0);
        assert_eq!(
            result.num_trades,
            data.len() - 1,
            "nến cuối không có nến kế tiếp để khớp — không được bịa ra giao dịch"
        );
    }
}
