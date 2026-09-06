//! # Chương 85: Hệ sinh thái HFT tích hợp — Nối mọi mảnh thành một hệ chạy được
//!
//! Chương 74–78 dựng từng mảnh rời: đo độ trễ, sổ lệnh, phát lại, cổng rủi ro, AMM.
//! Chương này **nối chúng lại** thành một hệ thống duy nhất chạy end-to-end:
//!
//! ```text
//!   nguồn phiên ──► bộ phát lại (đồng hồ ảo, đẩy tốc độ ×N)
//!                        │
//!            ┌───────────┴───────────┐
//!            ▼                       ▼
//!    sàn TRUYỀN THỐNG          sàn CHUỖI KHỐI
//!    (sổ lệnh giá–thời time)   (bể AMM x·y=k)
//!            └───────────┬───────────┘
//!                        ▼
//!              ảnh chụp thị trường hợp nhất
//!                        ▼
//!                 chiến lược (nhiều)
//!                        ▼
//!                   cổng rủi ro
//!                        ▼
//!         OMS: gửi lệnh CÓ ĐỘ TRỄ (hàng đợi theo thời điểm đến)
//!                        ▼
//!            sàn khớp ──► lãi lỗ, tồn store, đo lường
//! ```
//!
//! Ba tính chất bắt buộc, mỗi tính chất có bài kiểm thử riêng:
//! 1. **Tất định** — chạy hai lần cho kết quả trùng khớp từng bit.
//! 2. **Nhân quả** — chiến lược không bao giờ thấy dữ liệu tương lai; lệnh tới sàn
//!    sau một khoảng trễ, và khớp theo trạng thái sàn **tại thời điểm đến**.
//! 3. **Bất biến rủi ro** — không kịch bản nào vượt hạn mức, kể cả khi có lệnh treo.

use std::collections::{BTreeMap, VecDeque};

// ============================================================================
// 1. KIỂU NỀN
// ============================================================================

pub type Price = i64; // tick: 1 tick = 0,01 đơn vị tiền
pub type Quantity = i64;
pub type OrderId = u64;
pub type Nanos = u64;

#[derive(Debug, Clone, Copy, PartialEq, Eq, PartialOrd, Ord)]
pub enum Side {
    Buy,
    Sell,
}

impl Side {
    pub fn first(self) -> i64 {
        match self {
            Side::Buy => 1,
            Side::Sell => -1,
        }
    }
    pub fn inverse(self) -> Side {
        match self {
            Side::Buy => Side::Sell,
            Side::Sell => Side::Buy,
        }
    }
}

/// Định danh nơi giao dịch. Hệ sinh thái này chạy đồng thời hai loại sàn —
/// đó chính là "hai hướng" mà một hệ HFT hiện đại phải phủ.
#[derive(Debug, Clone, Copy, PartialEq, Eq, PartialOrd, Ord)]
pub enum Venue {
    /// Sàn truyền thống: sổ lệnh giới hạn, ưu tiên giá–thời gian.
    Lit,
    /// Sàn chuỗi khối: bể thanh khoản tự động, giá theo công thức.
    Chain,
}

// ============================================================================
// 2. ĐỒNG HỒ ẢO — NGUỒN THỜI GIAN DUY NHẤT
// ============================================================================

/// Mọi thành phần đọc thời gian **từ đây**, không bao giờ từ `Instant::now()`.
/// Một lời gọi đồng hồ thật lạc lõng là đủ phá cả tính tất định lẫn tính nhân quả.
#[derive(Debug, Clone, Copy, Default)]
pub struct VirtualClock {
    current: Nanos,
}

impl VirtualClock {
    pub fn new(start: Nanos) -> Self {
        VirtualClock { current: start }
    }
    pub fn now(&self) -> Nanos {
        self.current
    }
    /// Thời gian chỉ TIẾN. Lùi lại là dấu hiệu dữ liệu phiên bị xếp sai thứ tự.
    pub fn advance(&mut self, t: Nanos) -> bool {
        if t < self.current {
            return false;
        }
        self.current = t;
        true
    }
}

/// Hệ số nén thời gian tường. Không ảnh hưởng tới thời gian ẢO, nên kết quả
/// chiến lược **không đổi** dù chạy ở tốc độ nào — miễn là không ai đọc đồng hồ thật.
#[derive(Debug, Clone, Copy, PartialEq)]
pub enum ReplaySpeed {
    RealTime,
    Fast(u32),
    Unbounded,
}

impl ReplaySpeed {
    pub fn wall_delay(&self, khoang_ao_ns: Nanos) -> Nanos {
        match self {
            ReplaySpeed::RealTime => khoang_ao_ns,
            ReplaySpeed::Fast(n) => khoang_ao_ns / (*n).max(1) as u64,
            ReplaySpeed::Unbounded => 0,
        }
    }
}

// ============================================================================
// 3. MÔ HÌNH ĐỘ TRỄ
// ============================================================================

/// Ba khoảng trễ có thật, tách riêng vì chúng tối ưu được độc lập.
#[derive(Debug, Clone, Copy)]
pub struct LatencyModel {
    /// Sàn phát ─► ta nhận.
    pub inbound_ns: Nanos,
    /// Ta gửi ─► sàn nhận.
    pub outbound_ns: Nanos,
    /// Biên độ dao động; độ trễ thật có đuôi dài, không phải hằng số.
    pub jitter_ns: Nanos,
}

impl LatencyModel {
    pub fn typical() -> Self {
        LatencyModel { inbound_ns: 10_000, outbound_ns: 50_000, jitter_ns: 5_000 }
    }
    pub fn none() -> Self {
        LatencyModel { inbound_ns: 0, outbound_ns: 0, jitter_ns: 0 }
    }

    /// Dao động TẤT ĐỊNH theo hạt giống — cần nhiễu thật, nhưng phải tái lập được.
    pub fn order_latency(&self, hat: u64) -> Nanos {
        if self.jitter_ns == 0 {
            return self.outbound_ns;
        }
        self.outbound_ns + hash_in_range(hat, self.jitter_ns)
    }
}

/// splitmix64 — trộn đều thật sự. Phép chia dư đơn thuần làm giá trị co cụm
/// và khiến mọi phép đo dựa trên phân phối trở nên vô nghĩa.
pub fn hash64(mut x: u64) -> u64 {
    x = x.wrapping_add(0x9E3779B97F4A7C15);
    let mut z = x;
    z = (z ^ (z >> 30)).wrapping_mul(0xBF58476D1CE4E5B9);
    z = (z ^ (z >> 27)).wrapping_mul(0x94D049BB133111EB);
    z ^ (z >> 31)
}

pub fn hash_in_range(hat: u64, tran: u64) -> u64 {
    if tran == 0 { 0 } else { hash64(hat) % tran }
}

// ============================================================================
// 4. SỰ KIỆN PHIÊN
// ============================================================================

#[derive(Debug, Clone, PartialEq)]
pub enum EventKind {
    /// Sàn truyền thống: một lệnh giới hạn mới vào sổ.
    AddOrder { id: OrderId, side: Side, price: Price, quantity: Quantity },
    /// Sàn truyền thống: huỷ một lệnh đang treo.
    CancelOrder { id: OrderId },
    /// Sàn truyền thống: một giao dịch đã khớp (thông tin, không đổi sổ).
    Traded { price: Price, quantity: Quantity },
    /// Sàn chuỗi khối: ai đó hoán đổi trên bể, làm dự trữ đổi → giá đổi.
    PoolSwap { x_in: bool, quantity: u128 },
}

#[derive(Debug, Clone, PartialEq)]
pub struct SessionEvent {
    pub timestamp: Nanos,
    pub san: Venue,
    pub kind: EventKind,
}

/// Phiên đã ghi. Bất biến sống còn: `timestamp` không giảm.
#[derive(Debug, Clone, Default)]
pub struct RecordedSession {
    pub all_event: Vec<SessionEvent>,
}

impl RecordedSession {
    pub fn new() -> Self {
        RecordedSession::default()
    }

    /// Từ chối sự kiện lùi thời gian thay vì im lặng sắp xếp lại — dữ liệu
    /// xếp sai thứ tự là lỗi thu thập, và giấu nó đi thì phát lại sẽ nói dối.
    pub fn record(&mut self, sk: SessionEvent) -> bool {
        if let Some(last) = self.all_event.last() {
            if sk.timestamp < last.timestamp {
                return false;
            }
        }
        self.all_event.push(sk);
        true
    }

    pub fn event_count(&self) -> usize {
        self.all_event.len()
    }

    pub fn span_ns(&self) -> Nanos {
        match (self.all_event.first(), self.all_event.last()) {
            (Some(a), Some(b)) => b.timestamp - a.timestamp,
            _ => 0,
        }
    }

    pub fn is_ordered(&self) -> bool {
        self.all_event.windows(2).all(|w| w[0].timestamp <= w[1].timestamp)
    }
}

// ============================================================================
// 5. SÀN TRUYỀN THỐNG — SỔ LỆNH ƯU TIÊN GIÁ–THỜI GIAN
// ============================================================================

#[derive(Debug, Clone, Copy, PartialEq)]
pub struct PriceLevel {
    pub price: Price,
    pub quantity: Quantity,
}

#[derive(Debug, Clone, Default)]
pub struct LitVenue {
    /// `BTreeMap` chứ không `HashMap`: thứ tự duyệt phải tất định, nếu không
    /// phát lại sẽ không tái lập được và mọi phép gỡ lỗi đều vô nghĩa.
    buy: BTreeMap<Price, Quantity>,
    ban: BTreeMap<Price, Quantity>,
    /// Lệnh của THỊ TRƯỜNG (không phải của ta) để xử lý huỷ và khớp.
    market_orders: BTreeMap<OrderId, (Side, Price, Quantity)>,
    /// Hàng đợi FIFO tại mỗi (chiều, giá) — nền của ưu tiên thời gian.
    market_queues: BTreeMap<(Side, Price), VecDeque<OrderId>>,
    /// Lệnh của TA đang treo trên sàn.
    our_orders: BTreeMap<OrderId, OurOrder>,
    /// Khớp thụ động chờ bộ điều phối lấy ra.
    pending_passive_fills: Vec<Fill>,
}

#[derive(Debug, Clone, Copy, PartialEq)]
pub struct OurOrder {
    pub id: OrderId,
    pub side: Side,
    pub price: Price,
    pub remaining: Quantity,
    pub entered_at: Nanos,
    /// Khối lượng đứng trước tại thời điểm vào — nền của ước lượng khớp.
    pub prev_quantity: Quantity,
}

impl LitVenue {
    pub fn new() -> Self {
        LitVenue::default()
    }

    pub fn best_bid(&self) -> Option<PriceLevel> {
        self.buy.iter().next_back().map(|(&g, &k)| PriceLevel { price: g, quantity: k })
    }
    pub fn best_ask(&self) -> Option<PriceLevel> {
        self.ban.iter().next().map(|(&g, &k)| PriceLevel { price: g, quantity: k })
    }

    pub fn mid(&self) -> Option<f64> {
        match (self.best_bid(), self.best_ask()) {
            (Some(m), Some(b)) => Some((m.price + b.price) as f64 / 2.0),
            _ => None,
        }
    }

    /// Vi giá: trọng số NGƯỢC với khối lượng. Bên đông người kéo giá công bằng
    /// về phía bên mỏng, vì áp lực bên đó chưa được thoả mãn.
    pub fn micro_price(&self) -> Option<f64> {
        let (m, b) = (self.best_bid()?, self.best_ask()?);
        let tong = (m.quantity + b.quantity) as f64;
        if tong <= 0.0 {
            return None;
        }
        Some((m.price as f64 * b.quantity as f64 + b.price as f64 * m.quantity as f64) / tong)
    }

    pub fn imbalance(&self) -> Option<f64> {
        let (m, b) = (self.best_bid()?, self.best_ask()?);
        let tong = (m.quantity + b.quantity) as f64;
        if tong <= 0.0 {
            return None;
        }
        Some((m.quantity - b.quantity) as f64 / tong)
    }

    pub fn spread(&self) -> Option<Price> {
        Some(self.best_ask()?.price - self.best_bid()?.price)
    }

    /// Tổng khối lượng còn treo cả hai bên — thước đo độ "phình" của sổ.
    pub fn total_qty(&self) -> Quantity {
        self.buy.values().sum::<Quantity>() + self.ban.values().sum::<Quantity>()
    }

    pub fn is_crossed(&self) -> bool {
        match (self.best_bid(), self.best_ask()) {
            (Some(m), Some(b)) => m.price >= b.price,
            _ => false,
        }
    }

    fn ben(&mut self, c: Side) -> &mut BTreeMap<Price, Quantity> {
        match c {
            Side::Buy => &mut self.buy,
            Side::Sell => &mut self.ban,
        }
    }

    fn them(&mut self, c: Side, g: Price, k: Quantity) {
        if k <= 0 {
            return;
        }
        *self.ben(c).entry(g).or_insert(0) += k;
    }

    fn bot(&mut self, c: Side, g: Price, k: Quantity) {
        let ben = self.ben(c);
        if let Some(v) = ben.get_mut(&g) {
            *v -= k;
            if *v <= 0 {
                ben.remove(&g);
            }
        }
    }

    /// Khối lượng đứng trước một mức giá ở cùng chiều — vị trí xếp hàng.
    pub fn qty_at(&self, c: Side, g: Price) -> Quantity {
        match c {
            Side::Buy => self.buy.get(&g).copied().unwrap_or(0),
            Side::Sell => self.ban.get(&g).copied().unwrap_or(0),
        }
    }

    /// Tiêu thụ `can` đơn vị của lệnh THỊ TRƯỜNG tại (chiều, giá), theo FIFO.
    /// Trả về số thực sự tiêu được.
    fn consume_market(&mut self, c: Side, g: Price, mut can: Quantity) -> Quantity {
        let mut da = 0;
        let mut het = Vec::new();
        if let Some(q) = self.market_queues.get(&(c, g)) {
            for &m in q.iter() {
                if can <= 0 {
                    break;
                }
                let con = match self.market_orders.get(&m) {
                    Some(&(_, _, k)) => k,
                    None => continue,
                };
                let lay = can.min(con);
                can -= lay;
                da += lay;
                if let Some(v) = self.market_orders.get_mut(&m) {
                    v.2 -= lay;
                    if v.2 <= 0 {
                        het.push(m);
                    }
                }
            }
        }
        for m in &het {
            self.market_orders.remove(m);
        }
        if let Some(q) = self.market_queues.get_mut(&(c, g)) {
            q.retain(|m| !het.contains(m));
            if q.is_empty() {
                self.market_queues.remove(&(c, g));
            }
        }
        self.bot(c, g, da);
        da
    }

    /// Áp dụng một sự kiện thị trường. Bộ phát lại phải xử lý **mọi** loại —
    /// bỏ sót lệnh huỷ khiến sổ chỉ lớn lên rồi chéo vĩnh viễn.
    ///
    /// Lệnh mới cắt qua bên kia được KHỚP, không được chất lên sổ: một sàn thật
    /// không bao giờ để sổ chéo, và mô hình bỏ qua điều này sẽ cho chiến lược
    /// nhìn thấy những mức giá không tồn tại.
    pub fn apply(&mut self, sk: &EventKind) {
        match sk {
            EventKind::AddOrder { id, side, price, quantity } => {
                let mut con = *quantity;

                // Giai đoạn 1: khớp phần cắt qua với bên đối ứng.
                loop {
                    if con <= 0 {
                        break;
                    }
                    let swap = match side {
                        Side::Buy => self.ban.iter().next().map(|(&g, &k)| (g, k)),
                        Side::Sell => self.buy.iter().next_back().map(|(&g, &k)| (g, k)),
                    };
                    let (g, k) = match swap {
                        Some(x) => x,
                        None => break,
                    };
                    let cat = match side {
                        Side::Buy => *price >= g,
                        Side::Sell => *price <= g,
                    };
                    if !cat {
                        break;
                    }
                    // Lệnh của TA ở mức này cũng được khớp — đúng ưu tiên giá.
                    let ours: Vec<OrderId> = self
                        .our_orders
                        .values()
                        .filter(|l| l.side == side.inverse() && l.price == g)
                        .map(|l| l.id)
                        .collect();
                    let tt = self.consume_market(side.inverse(), g, con);
                    con -= tt;
                    if tt == 0 && !ours.is_empty() {
                        // Chỉ còn lệnh của ta ở mức này. Khớp ĐÚNG mức đó và
                        // ĐÚNG khối lượng còn lại — gọi hàm khớp toàn sổ ở đây
                        // sẽ ăn cả lệnh của ta ở những mức giá khác, và vị thế
                        // sẽ vọt qua hạn mức mà cổng rủi ro không hề biết.
                        let fill = self.fill_ours_at_level(side.inverse(), g, con);
                        let da: Quantity = fill.iter().map(|x| x.quantity).sum();
                        self.pending_passive_fills.extend(fill);
                        con -= da;
                        if da == 0 {
                            break;
                        }
                    } else if tt == 0 {
                        break;
                    }
                    let _ = k;
                }

                // Giai đoạn 2: phần còn lại nằm chờ trên sổ.
                if con > 0 {
                    self.them(*side, *price, con);
                    self.market_orders.insert(*id, (*side, *price, con));
                    self.market_queues.entry((*side, *price)).or_default().push_back(*id);
                }
            }
            EventKind::CancelOrder { id } => {
                if let Some((c, g, k)) = self.market_orders.remove(id) {
                    self.bot(c, g, k);
                    if let Some(q) = self.market_queues.get_mut(&(c, g)) {
                        q.retain(|x| x != id);
                        if q.is_empty() {
                            self.market_queues.remove(&(c, g));
                        }
                    }
                }
            }
            EventKind::Traded { .. } => {}
            EventKind::PoolSwap { .. } => {}
        }
    }

    /// Khớp lệnh của ta tại ĐÚNG một mức giá, không vượt quá `tran` đơn vị.
    /// Ưu tiên thời gian trong nội bộ mức.
    fn fill_ours_at_level(&mut self, side: Side, price: Price, tran: Quantity) -> Vec<Fill> {
        let mut ra = Vec::new();
        let mut con = tran;
        let mut candidates: Vec<OurOrder> = self
            .our_orders
            .values()
            .copied()
            .filter(|l| l.side == side && l.price == price)
            .collect();
        candidates.sort_by_key(|l| (l.entered_at, l.id));

        let mut done = Vec::new();
        for l in candidates {
            if con <= 0 {
                break;
            }
            let lay = con.min(l.remaining);
            if let Some(m) = self.our_orders.get_mut(&l.id) {
                m.remaining -= lay;
                if m.remaining <= 0 {
                    done.push(l.id);
                }
            }
            self.bot(side, price, lay);
            ra.push(Fill { id: l.id, side, price, quantity: lay, aggressive: false });
            con -= lay;
        }
        for m in done {
            self.our_orders.remove(&m);
        }
        ra
    }

    /// Lệnh treo của ta cũ hơn `tuoi_ns` — nhà tạo lập thật làm mới báo giá
    /// liên tục, và báo giá cũ là rủi ro chứ không phải cơ hội.
    pub fn our_orders_older_than(&self, now: Nanos, tuoi_ns: Nanos) -> Vec<OrderId> {
        self.our_orders
            .values()
            .filter(|l| now.saturating_sub(l.entered_at) > tuoi_ns)
            .map(|l| l.id)
            .collect()
    }

    /// Khớp thụ động phát sinh khi lệnh thị trường cắt qua lệnh treo của ta.
    /// Bộ điều phối lấy ra và ghi nhận vào vị thế.
    pub fn take_passive_fills(&mut self) -> Vec<Fill> {
        std::mem::take(&mut self.pending_passive_fills)
    }

    /// Đặt lệnh của ta. Nếu giá cắt qua bên kia thì khớp NGAY (lệnh chủ động).
    /// Ngược lại nó nằm chờ, và ta ghi lại khối lượng đứng trước.
    pub fn place_our_order(&mut self, l: OurOrder) -> Vec<Fill> {
        let mut fill = Vec::new();
        let mut con = l.remaining;

        // Lệnh chủ động: ăn qua các mức đối ứng theo thứ tự giá tốt nhất trước.
        loop {
            if con <= 0 {
                break;
            }
            let swap_resp = match l.side {
                Side::Buy => self.ban.iter().next().map(|(&g, &k)| (g, k)),
                Side::Sell => self.buy.iter().next_back().map(|(&g, &k)| (g, k)),
            };
            let (g, k) = match swap_resp {
                Some(x) => x,
                None => break,
            };
            let cat_qua = match l.side {
                Side::Buy => l.price >= g,
                Side::Sell => l.price <= g,
            };
            if !cat_qua {
                break;
            }
            let lay = con.min(k);
            self.bot(l.side.inverse(), g, lay);
            fill.push(Fill { id: l.id, side: l.side, price: g, quantity: lay, aggressive: true });
            con -= lay;
        }

        if con > 0 {
            let prev = self.qty_at(l.side, l.price);
            self.them(l.side, l.price, con);
            self.our_orders
                .insert(l.id, OurOrder { remaining: con, prev_quantity: prev, ..l });
        }
        fill
    }

    pub fn cancel_our_order(&mut self, id: OrderId) -> bool {
        match self.our_orders.remove(&id) {
            Some(l) => {
                self.bot(l.side, l.price, l.remaining);
                true
            }
            None => false,
        }
    }

    pub fn our_resting_orders(&self) -> Vec<OurOrder> {
        self.our_orders.values().copied().collect()
    }

    /// Khi thị trường khớp ở giá `g`, lệnh treo của ta ở giá tốt bằng hoặc hơn
    /// sẽ được khớp — nhưng chỉ sau khi hàng đứng trước đã tiêu hết.
    pub fn on_market_trade(&mut self, price: Price, mut quantity: Quantity) -> Vec<Fill> {
        let mut ra = Vec::new();
        let mut id_done = Vec::new();

        let mut candidates: Vec<OurOrder> = self
            .our_orders
            .values()
            .copied()
            .filter(|l| match l.side {
                Side::Buy => l.price >= price,
                Side::Sell => l.price <= price,
            })
            .collect();
        // Ưu tiên thời gian: ai vào trước được phục vụ trước.
        candidates.sort_by_key(|l| (l.entered_at, l.id));

        for l in candidates {
            if quantity <= 0 {
                break;
            }
            // Hàng đứng trước ăn phần của nó trước.
            let next_queue = quantity - l.prev_quantity;
            if next_queue <= 0 {
                if let Some(m) = self.our_orders.get_mut(&l.id) {
                    m.prev_quantity -= quantity;
                }
                break;
            }
            let lay = next_queue.min(l.remaining);
            if let Some(m) = self.our_orders.get_mut(&l.id) {
                m.prev_quantity = 0;
                m.remaining -= lay;
                if m.remaining <= 0 {
                    id_done.push(l.id);
                }
            }
            self.bot(l.side, l.price, lay);
            ra.push(Fill { id: l.id, side: l.side, price: l.price, quantity: lay, aggressive: false });
            quantity -= lay + l.prev_quantity;
        }
        for m in id_done {
            self.our_orders.remove(&m);
        }
        ra
    }
}

#[derive(Debug, Clone, Copy, PartialEq)]
pub struct Fill {
    pub id: OrderId,
    /// Chiều của LỆNH TA — mang theo, không suy ra từ giá. Đoán chiều từ giá
    /// là một lỗi thật đã gặp: đoán sai thì vị thế chạy ngược và mọi hạn mức
    /// rủi ro trở nên vô nghĩa.
    pub side: Side,
    pub price: Price,
    pub quantity: Quantity,
    /// `true` = ta chủ động ăn giá (trả phí taker), `false` = ta được khớp thụ động.
    pub aggressive: bool,
}

// ============================================================================
// 6. SÀN CHUỖI KHỐI — BỂ TÍCH KHÔNG ĐỔI
// ============================================================================

#[derive(Debug, Clone, Copy, PartialEq)]
pub struct ChainVenue {
    pub reserve_x: u128,
    pub reserve_y: u128,
    /// Phí theo phần vạn: 30 = 0,30%.
    pub fee_bps: u32,
}

#[derive(Debug, Clone, Copy, PartialEq)]
pub enum SwapError {
    ZeroInput,
    EmptyPool,
    BelowMinOut { nhan_duoc: u128, yeu_cau: u128 },
}

impl ChainVenue {
    pub fn new(x: u128, y: u128, fee_bps: u32) -> Self {
        ChainVenue { reserve_x: x, reserve_y: y, fee_bps }
    }

    pub fn k(&self) -> u128 {
        self.reserve_x * self.reserve_y
    }

    /// Giá niêm yết của X tính theo Y. Đây là giá **cận biên**, chỉ đúng cho
    /// khối lượng vô cùng nhỏ — mọi giao dịch thật đều tệ hơn con số này.
    pub fn price_x(&self) -> f64 {
        if self.reserve_x == 0 {
            return 0.0;
        }
        self.reserve_y as f64 / self.reserve_x as f64
    }

    pub fn try_swap(&self, x_in: bool, amount_in: u128) -> Result<u128, SwapError> {
        if amount_in == 0 {
            return Err(SwapError::ZeroInput);
        }
        if self.reserve_x == 0 || self.reserve_y == 0 {
            return Err(SwapError::EmptyPool);
        }
        let (dt_vao, dt_ra) =
            if x_in { (self.reserve_x, self.reserve_y) } else { (self.reserve_y, self.reserve_x) };
        let next_phi = amount_in * (10_000 - self.fee_bps as u128);
        // Làm tròn LUÔN có lợi cho bể — đó là chủ ý, không phải cẩu thả.
        Ok((next_phi * dt_ra) / (dt_vao * 10_000 + next_phi))
    }

    pub fn swap(&mut self, x_in: bool, amount_in: u128, toi_thieu_ra: u128) -> Result<u128, SwapError> {
        let ra = self.try_swap(x_in, amount_in)?;
        if ra < toi_thieu_ra {
            return Err(SwapError::BelowMinOut { nhan_duoc: ra, yeu_cau: toi_thieu_ra });
        }
        if x_in {
            self.reserve_x += amount_in;
            self.reserve_y -= ra;
        } else {
            self.reserve_y += amount_in;
            self.reserve_x -= ra;
        }
        Ok(ra)
    }

    /// Giá trung bình thực nhận — luôn tệ hơn `price_x()`. Đây mới là con số
    /// dùng để so với sàn truyền thống khi tìm chênh lệch.
    pub fn effective_price(&self, x_in: bool, amount_in: u128) -> Option<f64> {
        let ra = self.try_swap(x_in, amount_in).ok()?;
        if ra == 0 {
            return None;
        }
        Some(if x_in { ra as f64 / amount_in as f64 } else { amount_in as f64 / ra as f64 })
    }

    /// Nghịch đảo của `try_swap`: cần bỏ vào bao nhiêu để nhận ĐÚNG `ra`?
    /// Cần thiết cho phòng vệ chính xác — không có nó, chân phòng vệ lệch khối
    /// lượng và vị thế ròng không bao giờ về 0.
    pub fn input_for_output(&self, x_in: bool, ra_mong_muon: u128) -> Option<u128> {
        if ra_mong_muon == 0 {
            return None;
        }
        let (dt_vao, dt_ra) =
            if x_in { (self.reserve_x, self.reserve_y) } else { (self.reserve_y, self.reserve_x) };
        if ra_mong_muon >= dt_ra {
            return None; // không thể rút hết một phía
        }
        let tu = dt_vao * ra_mong_muon * 10_000;
        let mau = (dt_ra - ra_mong_muon) * (10_000 - self.fee_bps as u128);
        Some(tu / mau + 1) // +1: làm tròn LÊN, luôn có lợi cho bể
    }

    pub fn apply(&mut self, sk: &EventKind) {
        if let EventKind::PoolSwap { x_in, quantity } = sk {
            let _ = self.swap(*x_in, *quantity, 0);
        }
    }
}

// ============================================================================
// 7. ẢNH CHỤP THỊ TRƯỜNG HỢP NHẤT
// ============================================================================

/// Cái mà chiến lược được phép nhìn thấy — và **chỉ** cái này. Không có
/// tham chiếu tới phiên, không có chỉ số sự kiện, nên không thể nhìn trộm tương lai.
#[derive(Debug, Clone, Copy)]
pub struct MarketSnapshot {
    pub timestamp: Nanos,
    pub lit_buy: Option<PriceLevel>,
    pub lit_sell: Option<PriceLevel>,
    pub lit_micro_price: Option<f64>,
    pub lit_imbalance: Option<f64>,
    pub chain_price: f64,
    pub chain_reserve_x: u128,
    pub chain_reserve_y: u128,
}

impl MarketSnapshot {
    pub fn mid_price_traditional(&self) -> Option<f64> {
        match (self.lit_buy, self.lit_sell) {
            (Some(m), Some(b)) => Some((m.price + b.price) as f64 / 2.0),
            _ => None,
        }
    }

    /// Chênh lệch giá giữa hai sàn, tính bằng điểm cơ bản. Dương nghĩa là
    /// sàn chuỗi khối đang đắt hơn → mua truyền thống, bán chuỗi khối.
    pub fn cross_venue_bps(&self) -> Option<f64> {
        let tt = self.mid_price_traditional()?;
        if tt <= 0.0 {
            return None;
        }
        Some((self.chain_price - tt) / tt * 10_000.0)
    }
}

// ============================================================================
// 8. Ý ĐỊNH GIAO DỊCH & CỔNG RỦI RO
// ============================================================================

#[derive(Debug, Clone, Copy, PartialEq)]
pub enum Intent {
    Place { san: Venue, side: Side, price: Price, quantity: Quantity },
    CancelOrder { san: Venue, id: OrderId },
    /// Lệnh chính kèm **phòng vệ theo khối lượng đã khớp** trên sàn còn lại.
    ///
    /// Đặt cứng cả hai chân cùng lúc nghe có vẻ đúng nhưng vẫn hỏng: chân AMM
    /// luôn khớp đủ (công thức không bao giờ từ chối), còn chân sổ lệnh chỉ khớp
    /// một phần vì sổ đã đổi trong khoảng độ trễ. Chênh lệch đó đọng lại thành
    /// vị thế ròng — **bất đối xứng khớp**.
    ///
    /// Cách làm của ngành: thực thi chân KHÔNG CHẮC trước, rồi phòng vệ đúng
    /// bằng khối lượng thực sự khớp được.
    PlaceHedged { san: Venue, side: Side, price: Price, quantity: Quantity, hedge_on: Venue },
}

impl Intent {
    pub fn block_don(san: Venue, side: Side, price: Price, quantity: Quantity) -> Self {
        Intent::Place { san, side, price, quantity }
    }
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum RejectReason {
    KillSwitchOn,
    PriceOutOfBand,
    QuantityTooLarge,
    OrderValueTooLarge,
    PositionLimit,
    LossLimit,
    RateLimit,
}

/// Trạng thái vị thế theo giá vốn trung bình. Trường hợp **đảo chiều**
/// (vượt qua 0) phải xử lý riêng, nếu không giá vốn sai và mọi con số sau đó sai theo.
#[derive(Debug, Clone, Copy, Default)]
pub struct Position {
    pub quantity: Quantity,
    pub cost_basis: f64,
    pub realized_pnl: f64,
}

impl Position {
    pub fn record(&mut self, side: Side, price: Price, quantity: Quantity) {
        let prev = self.quantity;
        let d = side.first() * quantity;

        if prev == 0 || prev.signum() == d.signum() {
            // Mở rộng cùng chiều: cập nhật giá vốn trung bình có trọng số.
            let tong = (prev.abs() + quantity) as f64;
            if tong > 0.0 {
                self.cost_basis =
                    (self.cost_basis * prev.abs() as f64 + price as f64 * quantity as f64) / tong;
            }
            self.quantity = prev + d;
        } else {
            let dong = quantity.min(prev.abs());
            self.realized_pnl +=
                (price as f64 - self.cost_basis) * dong as f64 * prev.signum() as f64;
            self.quantity = prev + d;
            if self.quantity.signum() != prev.signum() && self.quantity != 0 {
                // Đảo chiều: phần dư mở vị thế mới ở đúng giá này.
                self.cost_basis = price as f64;
            } else if self.quantity == 0 {
                self.cost_basis = 0.0;
            }
        }
    }

    pub fn unrealized_pnl(&self, gia_hien_tai: f64) -> f64 {
        (gia_hien_tai - self.cost_basis) * self.quantity as f64
    }

    pub fn total_pnl(&self, gia_hien_tai: f64) -> f64 {
        self.realized_pnl + self.unrealized_pnl(gia_hien_tai)
    }
}

#[derive(Debug, Clone)]
pub struct RiskGate {
    pub min_price: Price,
    pub max_price: Price,
    pub max_quantity: Quantity,
    pub max_order_value: i64,
    pub max_position: Quantity,
    pub max_loss: f64,
    pub max_orders_per_sec: u32,
    pub kill_switch_on: bool,
    // trạng thái
    window_start: Nanos,
    window_count: u32,
    pub reject_counts: BTreeMap<u8, u32>,
}

impl RiskGate {
    pub fn typical() -> Self {
        RiskGate {
            min_price: 1,
            max_price: 10_000_000,
            max_quantity: 1_000,
            max_order_value: 100_000_000,
            max_position: 500,
            max_loss: 100_000.0,
            max_orders_per_sec: 10_000,
            kill_switch_on: false,
            window_start: 0,
            window_count: 0,
            reject_counts: BTreeMap::new(),
        }
    }

    fn record_reject(&mut self, t: RejectReason) -> RejectReason {
        *self.reject_counts.entry(t as u8).or_insert(0) += 1;
        t
    }

    /// Phơi nhiễm = vị thế đã khớp **cộng** khối lượng đang treo cùng chiều.
    /// Đếm thiếu phần treo là cách vị thế vượt hạn mức gấp ba lần mà không ai hay.
    pub fn check(
        &mut self,
        y: &Intent,
        position: Quantity,
        resting_bid: Quantity,
        resting_ask: Quantity,
        pnl: f64,
        now: Nanos,
    ) -> Result<(), RejectReason> {
        let (side, price, quantity) = match y {
            Intent::CancelOrder { .. } => return Ok(()), // huỷ luôn luôn được phép
            Intent::Place { side, price, quantity, .. } => (*side, *price, *quantity),
            // Bộ điều phối tách thành chân đơn trước khi tới đây, vì chỉ nó
            // mới biết đặt chỗ tích luỹ cho cả chân chính lẫn chân phòng vệ.
            Intent::PlaceHedged { .. } => return Ok(()),
        };

        if self.kill_switch_on {
            return Err(self.record_reject(RejectReason::KillSwitchOn));
        }
        if price < self.min_price || price > self.max_price {
            return Err(self.record_reject(RejectReason::PriceOutOfBand));
        }
        if quantity <= 0 || quantity > self.max_quantity {
            return Err(self.record_reject(RejectReason::QuantityTooLarge));
        }
        if price.saturating_mul(quantity) > self.max_order_value {
            return Err(self.record_reject(RejectReason::OrderValueTooLarge));
        }
        if pnl < -self.max_loss {
            return Err(self.record_reject(RejectReason::LossLimit));
        }

        // Kiểm CẢ HAI chiều phơi nhiễm, kể cả chiều mà lệnh này không chạm tới:
        // một lệnh mua vẫn phải bị chặn nếu chiều bán đã vượt hạn mức.
        let (sau_mua, sau_ban) = match side {
            Side::Buy => (position + resting_bid + quantity, position - resting_ask),
            Side::Sell => (position + resting_bid, position - resting_ask - quantity),
        };
        if sau_mua.abs() > self.max_position || sau_ban.abs() > self.max_position {
            return Err(self.record_reject(RejectReason::PositionLimit));
        }

        // Cửa sổ trượt một giây.
        if now.saturating_sub(self.window_start) >= 1_000_000_000 {
            self.window_start = now;
            self.window_count = 0;
        }
        if self.window_count >= self.max_orders_per_sec {
            return Err(self.record_reject(RejectReason::RateLimit));
        }
        self.window_count += 1;
        Ok(())
    }
}

// ============================================================================
// 9. CHIẾN LƯỢC
// ============================================================================

pub trait Strategy {
    fn name(&self) -> &str;
    /// Nhận ảnh chụp + vị thế hiện tại, trả về các ý định. Không có tham số nào
    /// cho phép nhìn về tương lai — đó là ràng buộc kiến trúc, không phải quy ước.
    fn evaluate(&mut self, snap: &MarketSnapshot, position: Quantity) -> Vec<Intent>;
}

/// Nhà tạo lập hai chiều có kiểm soát tồn kho: càng lệch vị thế thì càng
/// nghiêng báo giá về phía kéo vị thế về 0.
pub struct ManagedMaker {
    pub target_spread: Price,
    pub quantity: Quantity,
    pub inventory_limit: Quantity,
    pub skew_factor: f64,
    pub last_quote_at: Nanos,
    pub quote_interval_ns: Nanos,
}

impl ManagedMaker {
    pub fn new(inventory_limit: Quantity) -> Self {
        ManagedMaker {
            target_spread: 4,
            quantity: 20,
            inventory_limit,
            skew_factor: 0.5,
            last_quote_at: 0,
            quote_interval_ns: 1_000_000,
        }
    }
}

impl Strategy for ManagedMaker {
    fn name(&self) -> &str {
        "tao_lap_co_kiem_soat"
    }

    fn evaluate(&mut self, snap: &MarketSnapshot, position: Quantity) -> Vec<Intent> {
        if snap.timestamp.saturating_sub(self.last_quote_at) < self.quote_interval_ns {
            return Vec::new();
        }
        let mid = match snap.lit_micro_price.or_else(|| snap.mid_price_traditional()) {
            Some(g) => g,
            None => return Vec::new(),
        };
        self.last_quote_at = snap.timestamp;

        // Nghiêng báo giá theo tồn kho: dài vị thế thì hạ cả hai giá để dễ bán hơn.
        let ratio = if self.inventory_limit > 0 {
            position as f64 / self.inventory_limit as f64
        } else {
            0.0
        };
        let skew = ratio * self.skew_factor * self.target_spread as f64;
        let half = self.target_spread as f64 / 2.0;

        let mut price_buy = (mid - half - skew).round() as Price;
        let mut price_sell = (mid + half - skew).round() as Price;

        // KHÔNG BAO GIỜ cắt qua sổ. Một nhà tạo lập cắt giá sẽ TRẢ chênh lệch
        // thay vì THU nó — nó trở thành người chủ động, và toàn bộ mô hình kinh
        // doanh sụp đổ. Đây là ràng buộc, không phải tối ưu hoá.
        if let Some(b) = snap.lit_sell {
            price_buy = price_buy.min(b.price - 1);
        }
        if let Some(m) = snap.lit_buy {
            price_sell = price_sell.max(m.price + 1);
        }
        if price_buy <= 0 || price_sell <= price_buy {
            return Vec::new();
        }

        let mut ra = Vec::new();
        // Chỉ báo giá bên nào chưa chạm hạn mức — hàng phòng vệ thứ nhất,
        // trước cả cổng rủi ro.
        if position < self.inventory_limit {
            ra.push(Intent::Place {
                san: Venue::Lit,
                side: Side::Buy,
                price: price_buy,
                quantity: self.quantity,
            });
        }
        if position > -self.inventory_limit {
            ra.push(Intent::Place {
                san: Venue::Lit,
                side: Side::Sell,
                price: price_sell,
                quantity: self.quantity,
            });
        }
        ra
    }
}

/// Chênh lệch giá giữa hai sàn — chiến lược **duy nhất** chạm cả hai loại thị
/// trường, và là lý do hệ sinh thái này phải hợp nhất chúng vào một ảnh chụp.
pub struct CrossVenueArb {
    pub threshold_bps: f64,
    pub quantity: Quantity,
    pub opportunities_seen: u64,
}

impl CrossVenueArb {
    pub fn new(threshold_bps: f64) -> Self {
        CrossVenueArb { threshold_bps, quantity: 10, opportunities_seen: 0 }
    }
}

impl Strategy for CrossVenueArb {
    fn name(&self) -> &str {
        "chenh_lech_hai_san"
    }

    fn evaluate(&mut self, snap: &MarketSnapshot, _vi_the: Quantity) -> Vec<Intent> {
        let cl = match snap.cross_venue_bps() {
            Some(x) => x,
            None => return Vec::new(),
        };
        if cl.abs() < self.threshold_bps {
            return Vec::new();
        }
        self.opportunities_seen += 1;

        // Chênh lệch giá là giao dịch HAI CHÂN. Chỉ đặt một chân thì đó không
        // phải chênh lệch giá — đó là cược một chiều đội lốt, và nó sẽ tích luỹ
        // vị thế cho tới khi chạm hạn mức rồi ngồi đó chịu lỗ.
        let (chieu_tt, level) = if cl > 0.0 {
            // Chuỗi khối đắt hơn → mua chân rẻ (truyền thống), bán chân đắt.
            (Side::Buy, snap.lit_sell)
        } else {
            (Side::Sell, snap.lit_buy)
        };
        let m = match level {
            Some(m) => m,
            None => return Vec::new(),
        };
        let kl = self.quantity.min(m.quantity);
        if kl <= 0 {
            return Vec::new();
        }
        // Chân truyền thống là chân KHÔNG CHẮC (phải xếp hàng, sổ có thể đã đổi).
        // Chân chuỗi khối là phòng vệ, chỉ chạy đúng bằng phần thực sự khớp.
        vec![Intent::PlaceHedged {
            san: Venue::Lit,
            side: chieu_tt,
            price: m.price,
            quantity: kl,
            hedge_on: Venue::Chain,
        }]
    }
}

// ============================================================================
// 10. ĐO LƯỜNG
// ============================================================================

/// Biểu đồ thùng logarit: giữ được dải từ 1 ns tới hàng phút với sai số tương
/// đối cố định, chỉ tốn vài trăm byte.
#[derive(Debug, Clone, Default)]
pub struct LatencyHistogram {
    thung: BTreeMap<u32, u64>,
    pub samples: u64,
    pub tong: u64,
    pub max: u64,
}

impl LatencyHistogram {
    pub fn new() -> Self {
        LatencyHistogram::default()
    }

    pub fn record(&mut self, ns: u64) {
        let k = if ns == 0 { 0 } else { 64 - ns.leading_zeros() };
        *self.thung.entry(k).or_insert(0) += 1;
        self.samples += 1;
        self.tong += ns;
        self.max = self.max.max(ns);
    }

    pub fn mean(&self) -> f64 {
        if self.samples == 0 {
            0.0
        } else {
            self.tong as f64 / self.samples as f64
        }
    }

    /// Phân vị là con số DUY NHẤT đáng nhìn trong HFT. Trung bình chỉ hữu ích
    /// để phát hiện là mình đã đo sai.
    pub fn percentile(&self, p: f64) -> u64 {
        if self.samples == 0 {
            return 0;
        }
        let level = (self.samples as f64 * p).ceil() as u64;
        let mut accum_ke = 0;
        for (&k, &c) in &self.thung {
            accum_ke += c;
            if accum_ke >= level {
                return if k == 0 { 0 } else { 1u64 << (k - 1) };
            }
        }
        self.max
    }
}

#[derive(Debug, Clone, Default)]
pub struct Metrics {
    pub signal_to_order: LatencyHistogram,
    pub intents: u64,
    pub orders_sent: u64,
    pub orders_blocked: u64,
    pub fill_count: u64,
    pub filled_qty: Quantity,
    pub aggressive_qty: Quantity,
    pub equity_curve: Vec<f64>,
}

impl Metrics {
    pub fn new() -> Self {
        Metrics::default()
    }

    pub fn block_ratio(&self) -> f64 {
        if self.intents == 0 {
            0.0
        } else {
            self.orders_blocked as f64 / self.intents as f64
        }
    }

    /// Tỉ lệ thụ động: phần khối lượng ta được khớp mà không phải ăn giá.
    /// Nhà tạo lập sống bằng con số này.
    pub fn passive_ratio(&self) -> f64 {
        if self.filled_qty == 0 {
            0.0
        } else {
            (self.filled_qty - self.aggressive_qty) as f64 / self.filled_qty as f64
        }
    }

    pub fn max_drawdown(&self) -> f64 {
        let mut peak = f64::NEG_INFINITY;
        let mut dd: f64 = 0.0;
        for &v in &self.equity_curve {
            peak = peak.max(v);
            if peak.is_finite() {
                dd = dd.max(peak - v);
            }
        }
        dd
    }
}

// ============================================================================
// 11. HỆ SINH THÁI — BỘ ĐIỀU PHỐI
// ============================================================================

/// Lệnh đang bay tới sàn. Nó **chưa tồn tại** với sàn cho tới `arrives_at`.
/// Bỏ qua khoảng này là dạng nhìn trộm tương lai tinh vi nhất trong HFT.
#[derive(Debug, Clone, Copy)]
struct InFlightOrder {
    arrives_at: Nanos,
    sent_at: Nanos,
    intent: Intent,
    id: OrderId,
}

pub struct Ecosystem {
    pub clock: VirtualClock,
    pub speed: ReplaySpeed,
    pub latency: LatencyModel,
    pub venue_lit: LitVenue,
    pub venue_chain: ChainVenue,
    pub gate: RiskGate,
    pub position: Position,
    pub metrics: Metrics,
    pub next_id: OrderId,
    in_flight: VecDeque<InFlightOrder>,
    resting_bid: Quantity,
    resting_ask: Quantity,
    /// Phơi nhiễm của lệnh ĐANG BAY — đã phát nhưng chưa tới sàn.
    /// Không đếm phần này là lỗ hổng kinh điển: nhiều lệnh phát trong cùng một
    /// nhịp đều thấy CÙNG một trạng thái vị thế, đều được cổng cho qua, rồi
    /// cùng khớp — và hạn mức bị vượt dù mọi phép kiểm đều "đã chạy".
    in_flight_bid: Quantity,
    in_flight_ask: Quantity,
    /// Mã lệnh cần phòng vệ, và phòng vệ trên sàn nào.
    pending_hedge: BTreeMap<OrderId, Venue>,
    /// Số lần phòng vệ đã chạy — chỉ số vận hành, không phải trang trí.
    pub hedge_count: u64,
    /// Nhật ký lệnh đã gửi — cơ sở của bài kiểm thử tính tất định.
    pub order_log: Vec<(Nanos, Venue, Side, Price, Quantity)>,
    /// Tuổi tối đa của một báo giá trước khi bị tự động rút. Không có chính sách
    /// này thì báo giá chất đống, phơi nhiễm treo tăng vô hạn và cổng rủi ro
    /// chặn gần như mọi lệnh mới — hệ thống tự bóp cổ mình.
    pub max_quote_age_ns: Nanos,
}

impl Ecosystem {
    pub fn new(venue_chain: ChainVenue, latency: LatencyModel, speed: ReplaySpeed) -> Self {
        Ecosystem {
            clock: VirtualClock::default(),
            speed,
            latency,
            venue_lit: LitVenue::new(),
            venue_chain,
            gate: RiskGate::typical(),
            position: Position::default(),
            metrics: Metrics::new(),
            next_id: 1_000_000,
            in_flight: VecDeque::new(),
            resting_bid: 0,
            resting_ask: 0,
            in_flight_bid: 0,
            in_flight_ask: 0,
            pending_hedge: BTreeMap::new(),
            hedge_count: 0,
            order_log: Vec::new(),
            max_quote_age_ns: 20_000_000, // 20 ms
        }
    }

    pub fn snapshot(&self) -> MarketSnapshot {
        MarketSnapshot {
            timestamp: self.clock.now(),
            lit_buy: self.venue_lit.best_bid(),
            lit_sell: self.venue_lit.best_ask(),
            lit_micro_price: self.venue_lit.micro_price(),
            lit_imbalance: self.venue_lit.imbalance(),
            chain_price: self.venue_chain.price_x(),
            chain_reserve_x: self.venue_chain.reserve_x,
            chain_reserve_y: self.venue_chain.reserve_y,
        }
    }

    fn reference_price(&self) -> f64 {
        self.venue_lit
            .mid()
            .or_else(|| self.venue_lit.best_bid().map(|m| m.price as f64))
            .unwrap_or(self.position.cost_basis)
    }

    /// Giao các lệnh đã tới hạn. Chúng được khớp theo trạng thái sàn
    /// **tại thời điểm đến**, không phải lúc phát — đó là toàn bộ ý nghĩa của độ trễ.
    fn deliver_due(&mut self) {
        let now = self.clock.now();
        while self.in_flight.front().map_or(false, |l| l.arrives_at <= now) {
            let l = self.in_flight.pop_front().unwrap();
            match l.intent {
                Intent::CancelOrder { san: Venue::Lit, id } => {
                    if let Some(t) = self.venue_lit.our_resting_orders().iter().find(|x| x.id == id) {
                        match t.side {
                            Side::Buy => self.resting_bid -= t.remaining,
                            Side::Sell => self.resting_ask -= t.remaining,
                        }
                    }
                    self.venue_lit.cancel_our_order(id);
                }
                Intent::CancelOrder { .. } => {}
                Intent::PlaceHedged { .. } => {}
                Intent::Place { san: Venue::Lit, side, price, quantity } => {
                    match side {
                        Side::Buy => {
                            self.in_flight_bid = (self.in_flight_bid - quantity).max(0);
                            self.resting_bid += quantity;
                        }
                        Side::Sell => {
                            self.in_flight_ask = (self.in_flight_ask - quantity).max(0);
                            self.resting_ask += quantity;
                        }
                    }
                    let fill = self.venue_lit.place_our_order(OurOrder {
                        id: l.id,
                        side,
                        price,
                        remaining: quantity,
                        entered_at: l.arrives_at,
                        prev_quantity: 0,
                    });
                    self.metrics.signal_to_order.record(l.arrives_at - l.sent_at);
                    self.order_log.push((l.arrives_at, Venue::Lit, side, price, quantity));
                    for k in fill {
                        self.apply_fill(k);
                    }
                }
                Intent::Place { san: Venue::Chain, side, price, quantity } => {
                    // Sàn AMM khớp tức thì theo công thức — không xếp hàng, nhưng
                    // vẫn phải chịu độ trễ tới lượt được đưa vào khối.
                    match side {
                        Side::Buy => self.in_flight_bid = (self.in_flight_bid - quantity).max(0),
                        Side::Sell => self.in_flight_ask = (self.in_flight_ask - quantity).max(0),
                    }
                    let x_in = side == Side::Sell;
                    if self.venue_chain.swap(x_in, quantity as u128, 0).is_ok() {
                        self.metrics.signal_to_order.record(l.arrives_at - l.sent_at);
                        self.order_log.push((l.arrives_at, Venue::Chain, side, price, quantity));
                        self.apply_fill(Fill {
                            id: l.id,
                            side,
                            price,
                            quantity,
                            aggressive: true,
                        });
                    }
                }
            }
        }
    }

    fn apply_fill(&mut self, k: Fill) {
        self.position.record(k.side, k.price, k.quantity);

        // Phòng vệ NGAY, đúng bằng khối lượng vừa khớp. Đây là chỗ bất đối xứng
        // khớp được triệt tiêu: chân chắc chắn chỉ chạy sau khi chân không chắc
        // đã cho biết nó khớp được bao nhiêu.
        if let Some(&san_pv) = self.pending_hedge.get(&k.id) {
            if san_pv == Venue::Chain && k.quantity > 0 {
                let inverse = k.side.inverse();
                let kl = k.quantity as u128;
                // Giá phải là giá THỰC NHẬN trên bể, không phải giá của sàn kia.
                // Ghi sổ chân phòng vệ ở giá sàn truyền thống khiến chênh lệch
                // thu được luôn bằng 0 — chiến lược "phi rủi ro" chỉ còn chi phí.
                let exec_price = match inverse {
                    // Bán X trên bể: bỏ vào kl X, nhận `ra` Y → giá = ra/kl.
                    Side::Sell => self
                        .venue_chain
                        .swap(true, kl, 0)
                        .ok()
                        .map(|ra| ra as f64 / kl as f64),
                    // Mua X trên bể: cần bỏ vào bao nhiêu Y để nhận đúng kl X?
                    Side::Buy => self.venue_chain.input_for_output(false, kl).and_then(|vao_y| {
                        self.venue_chain
                            .swap(false, vao_y, 0)
                            .ok()
                            .map(|_| vao_y as f64 / kl as f64)
                    }),
                };
                if let Some(g) = exec_price {
                    let gt = g.round().max(1.0) as Price;
                    self.position.record(inverse, gt, k.quantity);
                    self.hedge_count += 1;
                    self.order_log.push((
                        self.clock.now(),
                        Venue::Chain,
                        inverse,
                        gt,
                        k.quantity,
                    ));
                }
            }
        }

        self.metrics.fill_count += 1;
        self.metrics.filled_qty += k.quantity;
        if k.aggressive {
            self.metrics.aggressive_qty += k.quantity;
        }
        match k.side {
            Side::Buy => self.resting_bid = (self.resting_bid - k.quantity).max(0),
            Side::Sell => self.resting_ask = (self.resting_ask - k.quantity).max(0),
        }
    }

    /// Đưa các ý định qua cổng rủi ro rồi xếp vào hàng đợi bay.
    pub fn publish(&mut self, cac_y: Vec<Intent>) {
        let now = self.clock.now();
        let pnl = self.position.total_pnl(self.reference_price());

        for y in cac_y {
            self.metrics.intents += 1;

            // --- lệnh chính + phòng vệ theo khối lượng đã khớp ---
            if let Intent::PlaceHedged { san, side, price, quantity, hedge_on } = y {
                let don = Intent::block_don(san, side, price, quantity);
                if self
                    .gate
                    .check(
                        &don,
                        self.position.quantity,
                        self.resting_bid + self.in_flight_bid,
                        self.resting_ask + self.in_flight_ask,
                        pnl,
                        now,
                    )
                    .is_err()
                {
                    self.metrics.orders_blocked += 1;
                    continue;
                }
                let id = self.next_id;
                self.next_id += 1;
                match side {
                    Side::Buy => self.in_flight_bid += quantity,
                    Side::Sell => self.in_flight_ask += quantity,
                }
                // Ghi nhớ: mọi phần khớp của mã này phải được phòng vệ ngay.
                self.pending_hedge.insert(id, hedge_on);
                let lat = self.latency.order_latency(id ^ now);
                self.in_flight.push_back(InFlightOrder {
                    arrives_at: now + lat,
                    sent_at: now,
                    intent: don,
                    id,
                });
                self.metrics.orders_sent += 1;
                continue;
            }

            // Cộng cả phơi nhiễm đang bay: đây là điểm khác biệt giữa một cổng
            // rủi ro đúng và một cổng chỉ trông có vẻ đúng.
            let ok = self.gate.check(
                &y,
                self.position.quantity,
                self.resting_bid + self.in_flight_bid,
                self.resting_ask + self.in_flight_ask,
                pnl,
                now,
            );
            if ok.is_err() {
                self.metrics.orders_blocked += 1;
                continue;
            }
            // ĐẶT CHỖ ngay lập tức, trước khi xét ý định tiếp theo trong cùng nhịp.
            #[allow(clippy::single_match)]
            if let Intent::Place { side, quantity, .. } = y {
                match side {
                    Side::Buy => self.in_flight_bid += quantity,
                    Side::Sell => self.in_flight_ask += quantity,
                }
            }
            let id = self.next_id;
            self.next_id += 1;
            // Hạt giống dẫn xuất từ (mã lệnh, thời điểm) → dao động tất định.
            let lat = self.latency.order_latency(id ^ now);
            self.in_flight.push_back(InFlightOrder {
                arrives_at: now + lat,
                sent_at: now,
                intent: y,
                id,
            });
            self.metrics.orders_sent += 1;
        }
        // Hàng đợi phải theo thứ tự thời gian đến; dao động có thể đảo thứ tự phát.
        let mut v: Vec<InFlightOrder> = self.in_flight.drain(..).collect();
        v.sort_by_key(|l| (l.arrives_at, l.id));
        self.in_flight = v.into();
    }

    /// Chạy trọn một phiên. Đây là điểm mà mọi mảnh của chương 74–78 gặp nhau.
    pub fn run(&mut self, session: &RecordedSession, cac_chien_luoc: &mut [Box<dyn Strategy>]) {
        for sk in &session.all_event {
            // 1. Thời gian tiến tới thời điểm sự kiện.
            if !self.clock.advance(sk.timestamp) {
                continue;
            }
            // 2. Giao mọi lệnh đã tới nơi TRƯỚC sự kiện này.
            self.deliver_due();

            // 2b. Rút báo giá đã quá cũ. Huỷ đi thẳng, không qua độ trễ gửi:
            // sàn thật xử lý huỷ trên đường ưu tiên, và quan trọng hơn — nếu
            // huỷ cũng phải xếp hàng thì rủi ro tồn kho không bao giờ giảm được.
            let cu = self
                .venue_lit
                .our_orders_older_than(self.clock.now(), self.max_quote_age_ns);
            for id in cu {
                if let Some(t) = self.venue_lit.our_resting_orders().iter().find(|x| x.id == id) {
                    match t.side {
                        Side::Buy => self.resting_bid = (self.resting_bid - t.remaining).max(0),
                        Side::Sell => self.resting_ask = (self.resting_ask - t.remaining).max(0),
                    }
                }
                self.venue_lit.cancel_our_order(id);
            }

            // 3. Áp dụng sự kiện lên đúng sàn của nó.
            match sk.san {
                Venue::Lit => {
                    self.venue_lit.apply(&sk.kind);
                    // Lệnh thị trường cắt qua lệnh treo của ta → khớp thụ động.
                    for k in self.venue_lit.take_passive_fills() {
                        self.apply_fill(k);
                    }
                    if let EventKind::Traded { price, quantity } = sk.kind {
                        for k in self.venue_lit.on_market_trade(price, quantity) {
                            self.apply_fill(k);
                        }
                    }
                }
                Venue::Chain => self.venue_chain.apply(&sk.kind),
            }

            // 4. Chiến lược nhìn ảnh chụp SAU sự kiện — và chỉ ảnh chụp.
            let snap = self.snapshot();
            let mut intent = Vec::new();
            for cl in cac_chien_luoc.iter_mut() {
                intent.extend(cl.evaluate(&snap, self.position.quantity));
            }
            self.publish(intent);

            self.metrics.equity_curve.push(self.position.total_pnl(self.reference_price()));
        }
        // Xả nốt các lệnh còn đang bay.
        self.clock.advance(self.clock.now() + 1_000_000_000);
        self.deliver_due();
    }
}

// ============================================================================
// 12. BỘ SINH PHIÊN TỔNG HỢP
// ============================================================================

/// Sinh một phiên hai sàn tất định. Hai chi tiết quan trọng, cả hai đều là
/// bài học rút ra từ lỗi thật: huỷ **lệnh sống cũ nhất** (không phải mã ngẫu
/// nhiên, vì phần lớn mã ngẫu nhiên đã chết), và **giới hạn số lệnh sống**
/// để sổ không phình ra rồi chéo vĩnh viễn.
pub const BE_KHOI_DAU: (u128, u128, u32) = (2_000_000, 20_000_000_000, 30);

pub fn generate_session(event_count: usize, hat_giong: u64, gia_neo: Price) -> RecordedSession {
    let mut p = RecordedSession::new();
    let mut t: Nanos = 1_000_000_000;
    let mut id: OrderId = 1;
    let mut song: VecDeque<OrderId> = VecDeque::new();
    let mut price_show = gia_neo;
    // Bản sao bể để chọn CHIỀU hoán đổi. Nó đại diện cho **phần còn lại của
    // thị trường**: những nhà chênh lệch khác liên tục kéo giá bể về sát sàn
    // truyền thống. Không có lực này, bể trôi tự do và mọi chiến lược chênh
    // lệch trong mô hình sẽ in ra tiền — một kết quả hoàn toàn giả.
    let mut pool = ChainVenue::new(BE_KHOI_DAU.0, BE_KHOI_DAU.1, BE_KHOI_DAU.2);

    for i in 0..event_count {
        let r = hash64(hat_giong ^ (i as u64).wrapping_mul(0x1000193));
        t += 1_000 + (r % 200_000);

        // Bước ngẫu nhiên có neo: kéo giá về `gia_neo` để chuỗi không trôi mất.
        let step = (hash64(r) % 5) as Price - 2;
        price_show = (price_show + step).max(gia_neo - 40).min(gia_neo + 40);

        let nhanh = r % 100;
        if nhanh < 8 {
            // Hoán đổi trên bể chuỗi khối. Chiều được chọn để KÉO giá bể về
            // phía giá sàn truyền thống, cộng thêm một phần nhiễu từ người
            // giao dịch thường.
            let lech = pool.price_x() - price_show as f64;
            let many = hash64(r ^ 0x5A5A) % 5 == 0; // 20% là nhiễu thuần
            let x_in = if many { (r >> 8) % 2 == 0 } else { lech > 0.0 };
            let kl = 1 + (hash64(r ^ 0xABC) % 500) as u128;
            let _ = pool.swap(x_in, kl, 0);
            p.record(SessionEvent {
                timestamp: t,
                san: Venue::Chain,
                kind: EventKind::PoolSwap { x_in, quantity: kl },
            });
        } else if nhanh < 20 && !song.is_empty() {
            // Giao dịch đã khớp trên sàn truyền thống.
            let kl = 1 + (hash64(r ^ 0xDEF) % 40) as Quantity;
            p.record(SessionEvent {
                timestamp: t,
                san: Venue::Lit,
                kind: EventKind::Traded { price: price_show, quantity: kl },
            });
        } else if song.len() >= 120 || (nhanh < 55 && song.len() > 20) {
            // Huỷ lệnh SỐNG CŨ NHẤT — mô phỏng đúng hành vi nhà tạo lập thật.
            if let Some(m) = song.pop_front() {
                p.record(SessionEvent {
                    timestamp: t,
                    san: Venue::Lit,
                    kind: EventKind::CancelOrder { id: m },
                });
            }
        } else {
            let side_buy = (r >> 16) % 2 == 0;
            let lech = 1 + (hash64(r ^ 0x777) % 6) as Price;
            let (side, price) = if side_buy {
                (Side::Buy, price_show - lech)
            } else {
                (Side::Sell, price_show + lech)
            };
            let kl = 10 + (hash64(r ^ 0x999) % 90) as Quantity;
            p.record(SessionEvent {
                timestamp: t,
                san: Venue::Lit,
                kind: EventKind::AddOrder { id, side, price, quantity: kl },
            });
            song.push_back(id);
            id += 1;
        }
    }
    p
}

// ============================================================================
// 13. TRÌNH DIỄN
// ============================================================================

fn main() {
    println!("=== CHƯƠNG 85: HỆ SINH THÁI HFT TÍCH HỢP ===\n");

    let session = generate_session(20_000, 0xC0FFEE, 10_000);
    println!("1. PHIÊN ĐÃ GHI");
    println!("   sự kiện        : {}", session.event_count());
    println!("   khoảng thời gian: {:.3} giây", session.span_ns() as f64 / 1e9);
    println!("   đúng thứ tự    : {}", session.is_ordered());

    println!("\n2. PHÁT LẠI Ở NHIỀU TỐC ĐỘ — kết quả PHẢI trùng nhau");
    println!("   {:<16} {:>10} {:>10} {:>12}", "tốc độ", "lệnh gửi", "khớp", "lãi/lỗ");
    let mut first_tien = None;
    for toc in [ReplaySpeed::Unbounded, ReplaySpeed::Fast(1_000), ReplaySpeed::RealTime] {
        let mut eco = Ecosystem::new(
            ChainVenue::new(2_000_000, 20_000_000_000, 30),
            LatencyModel::typical(),
            toc,
        );
        let mut cls: Vec<Box<dyn Strategy>> = vec![
            Box::new(ManagedMaker::new(200)),
            Box::new(CrossVenueArb::new(150.0)),
        ];
        eco.run(&session, &mut cls);
        let ll = eco.position.total_pnl(eco.reference_price());
        let name = match toc {
            ReplaySpeed::Unbounded => "vô hạn",
            ReplaySpeed::Fast(n) => {
                let _ = n;
                "×1000"
            }
            ReplaySpeed::RealTime => "thời gian thực",
        };
        println!(
            "   {:<16} {:>10} {:>10} {:>12.1}",
            name, eco.metrics.orders_sent, eco.metrics.fill_count, ll
        );
        let first_van = (eco.metrics.orders_sent, eco.metrics.fill_count, eco.order_log.len());
        match first_tien {
            None => first_tien = Some(first_van),
            Some(d) => assert_eq!(d, first_van, "phát lại KHÔNG tất định giữa các tốc độ"),
        }
    }

    println!("\n3. HỆ SINH THÁI ĐẦY ĐỦ — hai sàn, hai chiến lược");
    let mut eco = Ecosystem::new(
        ChainVenue::new(2_000_000, 20_000_000_000, 30),
        LatencyModel::typical(),
        ReplaySpeed::Unbounded,
    );
    let mut cls: Vec<Box<dyn Strategy>> = vec![
        Box::new(ManagedMaker::new(200)),
        Box::new(CrossVenueArb::new(150.0)),
    ];
    eco.run(&session, &mut cls);

    let m = &eco.metrics;
    println!("   ý định sinh ra     : {}", m.intents);
    println!("   lệnh gửi đi        : {}", m.orders_sent);
    println!("   bị cổng rủi ro chặn: {} ({:.1}%)", m.orders_blocked, m.block_ratio() * 100.0);
    println!("   số lần khớp        : {}", m.fill_count);
    println!("   khối lượng khớp    : {}", m.filled_qty);
    println!("   tỉ lệ thụ động     : {:.1}%", m.passive_ratio() * 100.0);
    println!("   lần phòng vệ chạy  : {}", eco.hedge_count);
    println!("   vị thế cuối        : {}", eco.position.quantity);
    println!("   lãi/lỗ đã chốt     : {:.1}", eco.position.realized_pnl);
    println!("   sụt giảm tối đa    : {:.1}", m.max_drawdown());

    println!("\n4. ĐỘ TRỄ TÍN HIỆU → LỆNH TỚI SÀN (nanosecond)");
    let h = &m.signal_to_order;
    println!("   mẫu   : {}", h.samples);
    println!("   trung bình: {:.0}", h.mean());
    println!("   p50   : {}", h.percentile(0.50));
    println!("   p99   : {}", h.percentile(0.99));
    println!("   lớn nhất: {}", h.max);

    println!("\n5. CỔNG RỦI RO ĐÃ CHẶN GÌ");
    let name = |k: u8| match k {
        0 => "đã ngắt khẩn cấp",
        1 => "giá ngoài biên",
        2 => "khối lượng quá lớn",
        3 => "giá trị lệnh quá lớn",
        4 => "vượt hạn mức vị thế",
        5 => "vượt hạn mức lỗ",
        _ => "vượt tần suất",
    };
    if eco.gate.reject_counts.is_empty() {
        println!("   (không có lệnh nào bị chặn)");
    }
    for (k, v) in &eco.gate.reject_counts {
        println!("   {:<24} {}", name(*k), v);
    }

    println!("\n6. VÌ SAO KHÔNG ĐƯỢC TIN CON SỐ LÃI/LỖ Ở TRÊN");
    println!("   Phiên này là TỔNG HỢP, và mối liên kết giữa hai sàn chỉ được mô");
    println!("   phỏng một phần: bể chuỗi khối được kéo về giá sàn truyền thống,");
    println!("   nhưng không hoàn hảo. Khe hở còn lại là quà tặng cho chiến lược");
    println!("   chênh lệch — thứ không tồn tại trên thị trường thật, nơi hàng trăm");
    println!("   hãng cùng săn đúng khe hở đó trong vài trăm nanosecond.");
    println!("   Thứ ĐÁNG tin ở chương này là các BẤT BIẾN bên dưới, không phải lãi/lỗ.");

    println!("\n7. BẤT BIẾN RỦI RO");
    println!(
        "   |vị thế| ≤ hạn mức {} : {}",
        eco.gate.max_position,
        eco.position.quantity.abs() <= eco.gate.max_position
    );
    println!("   sổ lệnh không chéo   : {}", !eco.venue_lit.is_crossed());
}

// ============================================================================
// KIỂM THỬ
// ============================================================================

#[cfg(test)]
mod tests {
    use super::*;

    fn new_ecosystem() -> Ecosystem {
        Ecosystem::new(
            ChainVenue::new(2_000_000, 20_000_000_000, 30),
            LatencyModel::typical(),
            ReplaySpeed::Unbounded,
        )
    }

    // ---- đồng hồ ảo ----

    #[test]
    fn clock_only_moves_forward() {
        let mut d = VirtualClock::new(100);
        assert!(d.advance(200));
        assert_eq!(d.now(), 200);
        assert!(!d.advance(150), "phải từ chối lùi thời gian");
        assert_eq!(d.now(), 200);
    }

    #[test]
    fn replay_speed_leaves_virtual_time_intact() {
        assert_eq!(ReplaySpeed::RealTime.wall_delay(1_000_000), 1_000_000);
        assert_eq!(ReplaySpeed::Fast(1000).wall_delay(1_000_000), 1_000);
        assert_eq!(ReplaySpeed::Unbounded.wall_delay(1_000_000), 0);
    }

    // ---- phiên ----

    #[test]
    fn session_rejects_backwards_events() {
        let mut p = RecordedSession::new();
        assert!(p.record(SessionEvent {
            timestamp: 100,
            san: Venue::Lit,
            kind: EventKind::Traded { price: 10, quantity: 1 },
        }));
        assert!(!p.record(SessionEvent {
            timestamp: 50,
            san: Venue::Lit,
            kind: EventKind::Traded { price: 10, quantity: 1 },
        }));
        assert_eq!(p.event_count(), 1);
    }

    #[test]
    fn generated_session_is_ordered() {
        let p = generate_session(5_000, 1, 10_000);
        assert!(p.is_ordered());
        assert_eq!(p.event_count(), 5_000);
    }

    #[test]
    fn session_covers_both_venues() {
        let p = generate_session(5_000, 7, 10_000);
        let tt = p.all_event.iter().filter(|s| s.san == Venue::Lit).count();
        let ck = p.all_event.iter().filter(|s| s.san == Venue::Chain).count();
        assert!(tt > 0 && ck > 0, "phiên phải phủ cả hai loại thị trường");
    }

    // ---- sổ lệnh truyền thống ----

    #[test]
    fn book_tracks_best_prices() {
        let mut s = LitVenue::new();
        for (id, c, g, k) in [
            (1, Side::Buy, 99, 10),
            (2, Side::Buy, 100, 20),
            (3, Side::Sell, 102, 15),
            (4, Side::Sell, 101, 5),
        ] {
            s.apply(&EventKind::AddOrder { id, side: c, price: g, quantity: k });
        }
        assert_eq!(s.best_bid().unwrap().price, 100);
        assert_eq!(s.best_ask().unwrap().price, 101);
        assert_eq!(s.spread(), Some(1));
        assert!(!s.is_crossed());
    }

    #[test]
    fn cancel_shrinks_book() {
        let mut s = LitVenue::new();
        s.apply(&EventKind::AddOrder { id: 1, side: Side::Buy, price: 100, quantity: 50 });
        assert_eq!(s.qty_at(Side::Buy, 100), 50);
        s.apply(&EventKind::CancelOrder { id: 1 });
        assert_eq!(s.qty_at(Side::Buy, 100), 0);
        assert!(s.best_bid().is_none());
    }

    #[test]
    fn skipping_cancels_inflates_book() {
        // LỖI THẬT đã gặp: bộ phát lại chỉ xử lý "thêm". Với động cơ khớp đúng,
        // hậu quả không phải là sổ chéo (lệnh cắt qua bị khớp mất) mà là
        // BÁO GIÁ CŨ KHÔNG BAO GIỜ BIẾN MẤT: sổ phình ra và chênh lệch hẹp giả tạo.
        // Chiến lược khi đó thấy thanh khoản không tồn tại.
        let mut has_cancel = LitVenue::new();
        let mut no_cancel = LitVenue::new();
        let p = generate_session(3_000, 42, 10_000);
        for sk in &p.all_event {
            if sk.san != Venue::Lit {
                continue;
            }
            has_cancel.apply(&sk.kind);
            if !matches!(sk.kind, EventKind::CancelOrder { .. }) {
                no_cancel.apply(&sk.kind);
            }
        }
        assert!(
            no_cancel.total_qty() > has_cancel.total_qty() * 2,
            "bỏ lệnh huỷ thì sổ phình lên: {} so với {}",
            no_cancel.total_qty(),
            has_cancel.total_qty()
        );
    }

    #[test]
    fn micro_price_leans_to_thin_side() {
        let mut s = LitVenue::new();
        s.apply(&EventKind::AddOrder { id: 1, side: Side::Buy, price: 100, quantity: 900 });
        s.apply(&EventKind::AddOrder { id: 2, side: Side::Sell, price: 102, quantity: 100 });
        let mid = s.mid().unwrap();
        let vi = s.micro_price().unwrap();
        assert!(vi > mid, "bên mua đông → vi giá phải cao hơn giá giữa");
        assert!(vi < 102.0);
    }

    #[test]
    fn imbalance_has_correct_sign() {
        let mut s = LitVenue::new();
        s.apply(&EventKind::AddOrder { id: 1, side: Side::Buy, price: 100, quantity: 900 });
        s.apply(&EventKind::AddOrder { id: 2, side: Side::Sell, price: 102, quantity: 100 });
        assert!((s.imbalance().unwrap() - 0.8).abs() < 1e-9);
    }

    #[test]
    fn aggressive_order_fills_immediately() {
        let mut s = LitVenue::new();
        s.apply(&EventKind::AddOrder { id: 1, side: Side::Sell, price: 100, quantity: 30 });
        s.apply(&EventKind::AddOrder { id: 2, side: Side::Sell, price: 101, quantity: 30 });
        let fill = s.place_our_order(OurOrder {
            id: 9,
            side: Side::Buy,
            price: 101,
            remaining: 50,
            entered_at: 0,
            prev_quantity: 0,
        });
        assert_eq!(fill.len(), 2);
        assert_eq!(fill[0].price, 100, "phải ăn giá tốt nhất trước");
        assert_eq!(fill.iter().map(|k| k.quantity).sum::<Quantity>(), 50);
        assert!(fill.iter().all(|k| k.aggressive));
    }

    #[test]
    fn passive_order_waits_in_queue() {
        let mut s = LitVenue::new();
        s.apply(&EventKind::AddOrder { id: 1, side: Side::Buy, price: 100, quantity: 200 });
        let fill = s.place_our_order(OurOrder {
            id: 9,
            side: Side::Buy,
            price: 100,
            remaining: 50,
            entered_at: 0,
            prev_quantity: 0,
        });
        assert!(fill.is_empty(), "không cắt qua thì không khớp ngay");
        let resting = s.our_resting_orders();
        assert_eq!(resting.len(), 1);
        assert_eq!(resting[0].prev_quantity, 200, "phải ghi nhận hàng đứng trước");
    }

    #[test]
    fn queue_ahead_is_served_first() {
        let mut s = LitVenue::new();
        s.apply(&EventKind::AddOrder { id: 1, side: Side::Buy, price: 100, quantity: 100 });
        s.place_our_order(OurOrder {
            id: 9,
            side: Side::Buy,
            price: 100,
            remaining: 50,
            entered_at: 1,
            prev_quantity: 0,
        });
        // Thị trường khớp 60: 100 đơn vị đứng trước chưa tiêu hết → ta không được gì.
        let k = s.on_market_trade(100, 60);
        assert!(k.is_empty(), "hàng đứng trước phải tiêu hết trước khi tới lượt ta");
        // Khớp thêm 120: vượt qua 40 còn lại của hàng → ta được khớp phần dư.
        let k2 = s.on_market_trade(100, 120);
        assert!(!k2.is_empty());
        assert!(k2.iter().all(|x| !x.aggressive));
    }

    // ---- sàn chuỗi khối ----

    #[test]
    fn swap_never_decreases_k() {
        let mut b = ChainVenue::new(1_000_000, 1_000_000, 30);
        let k0 = b.k();
        b.swap(true, 10_000, 0).unwrap();
        assert!(b.k() >= k0, "phí làm tích TĂNG, không bao giờ giảm");
    }

    #[test]
    fn larger_size_gets_worse_price() {
        let b = ChainVenue::new(1_000_000, 1_000_000, 30);
        let small = b.effective_price(true, 1_000).unwrap();
        let large = b.effective_price(true, 100_000).unwrap();
        assert!(large < small, "khối lượng lớn nhận được ít hơn trên mỗi đơn vị");
    }

    #[test]
    fn pool_never_runs_dry() {
        let mut b = ChainVenue::new(1_000, 1_000, 30);
        for _ in 0..50 {
            let _ = b.swap(true, 10_000, 0);
        }
        assert!(b.reserve_y > 0, "x·y=k khiến bể không thể bị hút cạn");
    }

    #[test]
    fn min_out_blocks_bad_price() {
        let mut b = ChainVenue::new(1_000_000, 1_000_000, 30);
        let amount_in = b.try_swap(true, 10_000).unwrap();
        let r = b.swap(true, 10_000, amount_in + 1);
        assert!(matches!(r, Err(SwapError::BelowMinOut { .. })));
        assert_eq!(b.reserve_x, 1_000_000, "giao dịch bị chặn thì bể không đổi");
    }

    // ---- vị thế & lãi lỗ ----

    #[test]
    fn average_cost_basis_is_correct() {
        let mut v = Position::default();
        v.record(Side::Buy, 100, 10);
        v.record(Side::Buy, 110, 10);
        assert_eq!(v.quantity, 20);
        assert!((v.cost_basis - 105.0).abs() < 1e-9);
    }

    #[test]
    fn partial_close_realizes_correct_pnl() {
        let mut v = Position::default();
        v.record(Side::Buy, 100, 10);
        v.record(Side::Sell, 120, 4);
        assert_eq!(v.quantity, 6);
        assert!((v.realized_pnl - 80.0).abs() < 1e-9, "(120−100)×4 = 80");
    }

    #[test]
    fn reversal_resets_cost_basis() {
        let mut v = Position::default();
        v.record(Side::Buy, 100, 10);
        v.record(Side::Sell, 120, 15); // đóng 10, mở bán 5
        assert_eq!(v.quantity, -5);
        assert!((v.realized_pnl - 200.0).abs() < 1e-9);
        assert!((v.cost_basis - 120.0).abs() < 1e-9, "phần dư mở ở giá giao dịch");
    }

    #[test]
    fn full_close_zeroes_cost_basis() {
        let mut v = Position::default();
        v.record(Side::Buy, 100, 10);
        v.record(Side::Sell, 105, 10);
        assert_eq!(v.quantity, 0);
        assert_eq!(v.cost_basis, 0.0);
        assert!((v.realized_pnl - 50.0).abs() < 1e-9);
    }

    // ---- cổng rủi ro ----

    #[test]
    fn gate_blocks_out_of_band_price() {
        let mut c = RiskGate::typical();
        let y = Intent::Place {
            san: Venue::Lit,
            side: Side::Buy,
            price: 0,
            quantity: 10,
        };
        assert_eq!(c.check(&y, 0, 0, 0, 0.0, 0), Err(RejectReason::PriceOutOfBand));
    }

    #[test]
    fn gate_counts_resting_orders() {
        let mut c = RiskGate::typical();
        c.max_position = 100;
        let y = Intent::Place {
            san: Venue::Lit,
            side: Side::Buy,
            price: 100,
            quantity: 50,
        };
        // Vị thế 0 nhưng đã treo mua 60 → thêm 50 nữa là vượt 100.
        assert_eq!(c.check(&y, 0, 60, 0, 0.0, 0), Err(RejectReason::PositionLimit));
        // Không có lệnh treo thì cùng lệnh đó qua được.
        assert!(c.check(&y, 0, 0, 0, 0.0, 0).is_ok());
    }

    #[test]
    fn gate_blocks_on_loss_limit() {
        let mut c = RiskGate::typical();
        c.max_loss = 1_000.0;
        let y = Intent::Place {
            san: Venue::Lit,
            side: Side::Buy,
            price: 100,
            quantity: 10,
        };
        assert_eq!(c.check(&y, 0, 0, 0, -1_500.0, 0), Err(RejectReason::LossLimit));
    }

    #[test]
    fn kill_switch_blocks_new_orders() {
        let mut c = RiskGate::typical();
        c.kill_switch_on = true;
        let y = Intent::Place {
            san: Venue::Lit,
            side: Side::Buy,
            price: 100,
            quantity: 1,
        };
        assert_eq!(c.check(&y, 0, 0, 0, 0.0, 0), Err(RejectReason::KillSwitchOn));
    }

    #[test]
    fn cancels_always_allowed() {
        let mut c = RiskGate::typical();
        c.kill_switch_on = true;
        // Ngắt khẩn cấp phải cho HUỶ qua — nếu không, bạn không rút được chân ra.
        let y = Intent::CancelOrder { san: Venue::Lit, id: 1 };
        assert!(c.check(&y, 0, 0, 0, 0.0, 0).is_ok());
    }

    #[test]
    fn gate_rate_limits_on_sliding_window() {
        let mut c = RiskGate::typical();
        c.max_orders_per_sec = 3;
        let y = Intent::Place {
            san: Venue::Lit,
            side: Side::Buy,
            price: 100,
            quantity: 1,
        };
        for _ in 0..3 {
            assert!(c.check(&y, 0, 0, 0, 0.0, 0).is_ok());
        }
        assert_eq!(c.check(&y, 0, 0, 0, 0.0, 0), Err(RejectReason::RateLimit));
        // Sang giây mới thì cửa sổ mở lại.
        assert!(c.check(&y, 0, 0, 0, 0.0, 1_000_000_000).is_ok());
    }

    // ---- độ trễ ----

    #[test]
    fn latency_jitters_but_is_deterministic() {
        let m = LatencyModel::typical();
        let a: Vec<u64> = (0..100).map(|i| m.order_latency(i)).collect();
        let b: Vec<u64> = (0..100).map(|i| m.order_latency(i)).collect();
        assert_eq!(a, b, "cùng hạt giống phải cho cùng độ trễ");
        assert!(a.iter().any(|&x| x != a[0]), "phải có dao động thật, không phải hằng số");
        assert!(a.iter().all(|&x| x >= m.outbound_ns));
    }

    #[test]
    fn hash_is_uniformly_distributed() {
        // Số học chia dư đơn thuần làm giá trị co cụm và phá mọi phép đo phân phối.
        let mut thung = [0u32; 8];
        for i in 0..8_000u64 {
            thung[(hash64(i) % 8) as usize] += 1;
        }
        assert!(thung.iter().all(|&c| c > 800 && c < 1_200), "phân bố phải đều: {:?}", thung);
    }

    // ---- biểu đồ ----

    #[test]
    fn percentiles_catch_tail_that_mean_hides() {
        let mut h = LatencyHistogram::new();
        for i in 0..10_000 {
            // 99,9% nhanh, 0,1% chậm 50 µs — đúng hình dạng độ trễ thật.
            h.record(if i % 1000 == 0 { 50_000 } else { 300 });
        }
        assert!(h.percentile(0.50) <= 512);
        assert!(h.percentile(0.99) <= 512, "p99 vẫn nhanh — cái đuôi bị giấu");
        assert_eq!(h.max, 50_000);
        assert!(h.max as f64 > h.mean() * 100.0, "max lớn hơn trung bình >100×");
    }

    // ---- chiến lược ----

    #[test]
    fn maker_skews_quotes_by_inventory() {
        let snap = MarketSnapshot {
            timestamp: 10_000_000,
            lit_buy: Some(PriceLevel { price: 100, quantity: 50 }),
            lit_sell: Some(PriceLevel { price: 104, quantity: 50 }),
            lit_micro_price: Some(102.0),
            lit_imbalance: Some(0.0),
            chain_price: 102.0,
            chain_reserve_x: 1,
            chain_reserve_y: 102,
        };
        let lay = |position| {
            let mut m = ManagedMaker::new(100);
            let y = m.evaluate(&snap, position);
            y.iter()
                .filter_map(|x| match x {
                    Intent::Place { side: Side::Buy, price, .. } => Some(*price),
                    _ => None,
                })
                .next()
        };
        let duplicate_loop = lay(0).unwrap();
        let long = lay(80).unwrap();
        assert!(long < duplicate_loop, "dài vị thế → hạ giá mua để bớt mua thêm");
    }

    #[test]
    fn maker_never_crosses_book() {
        // Sổ hẹp hơn chênh lệch mục tiêu của nhà tạo lập — nếu không có ràng buộc,
        // báo giá sẽ cắt qua và biến nhà tạo lập thành người chủ động.
        let snap = MarketSnapshot {
            timestamp: 10_000_000,
            lit_buy: Some(PriceLevel { price: 101, quantity: 50 }),
            lit_sell: Some(PriceLevel { price: 102, quantity: 50 }),
            lit_micro_price: Some(101.5),
            lit_imbalance: Some(0.0),
            chain_price: 101.5,
            chain_reserve_x: 1,
            chain_reserve_y: 101,
        };
        let mut m = ManagedMaker::new(100);
        for y in m.evaluate(&snap, 0) {
            if let Intent::Place { side, price, .. } = y {
                match side {
                    Side::Buy => assert!(price < 102, "giá mua {} cắt qua giá bán tốt nhất", price),
                    Side::Sell => assert!(price > 101, "giá bán {} cắt qua giá mua tốt nhất", price),
                }
            }
        }
    }

    #[test]
    fn maker_fills_mostly_passively() {
        // Hệ quả đo được của ràng buộc trên: phần lớn khối lượng phải đến từ
        // khớp THỤ ĐỘNG. Nhà tạo lập chủ yếu chủ động là nhà tạo lập đang lỗ.
        let p = generate_session(10_000, 0x4242, 10_000);
        let mut h = new_ecosystem();
        let mut cls: Vec<Box<dyn Strategy>> = vec![Box::new(ManagedMaker::new(200))];
        h.run(&p, &mut cls);
        assert!(h.metrics.filled_qty > 0);
        assert!(
            h.metrics.passive_ratio() > 0.8,
            "tỉ lệ thụ động chỉ {:.1}% — nhà tạo lập đang cắt qua sổ",
            h.metrics.passive_ratio() * 100.0
        );
    }

    #[test]
    fn maker_stops_quoting_at_limit() {
        let snap = MarketSnapshot {
            timestamp: 10_000_000,
            lit_buy: Some(PriceLevel { price: 100, quantity: 50 }),
            lit_sell: Some(PriceLevel { price: 104, quantity: 50 }),
            lit_micro_price: Some(102.0),
            lit_imbalance: Some(0.0),
            chain_price: 102.0,
            chain_reserve_x: 1,
            chain_reserve_y: 102,
        };
        let mut m = ManagedMaker::new(100);
        let y = m.evaluate(&snap, 100);
        assert!(
            y.iter().all(|x| !matches!(x, Intent::Place { side: Side::Buy, .. })),
            "chạm hạn mức dài thì không báo giá mua nữa"
        );
    }

    #[test]
    fn arb_fires_only_above_threshold() {
        let mut snap = MarketSnapshot {
            timestamp: 1,
            lit_buy: Some(PriceLevel { price: 10_000, quantity: 100 }),
            lit_sell: Some(PriceLevel { price: 10_002, quantity: 100 }),
            lit_micro_price: Some(10_001.0),
            lit_imbalance: Some(0.0),
            chain_price: 10_001.0,
            chain_reserve_x: 1,
            chain_reserve_y: 10_001,
        };
        let mut c = CrossVenueArb::new(50.0);
        assert!(c.evaluate(&snap, 0).is_empty(), "hai sàn ngang giá → không giao dịch");

        snap.chain_price = 10_001.0 * 1.02; // lệch 200 bp
        let y = c.evaluate(&snap, 0);
        assert_eq!(y.len(), 1);
        match y[0] {
            Intent::PlaceHedged { san, side, hedge_on, .. } => {
                // Chân KHÔNG CHẮC (sổ lệnh) chạy trước; chân chắc chắn (AMM)
                // chỉ phòng vệ đúng phần thực sự khớp.
                assert_eq!(san, Venue::Lit);
                assert_eq!(side, Side::Buy, "chuỗi khối đắt hơn → mua chân truyền thống");
                assert_eq!(hedge_on, Venue::Chain);
            }
            _ => panic!("chênh lệch giá phải là lệnh có phòng vệ, không phải lệnh trần"),
        }
    }

    #[test]
    fn hedged_arb_stays_flat() {
        // Hệ quả kiểm chứng được của việc đặt đủ hai chân: chiến lược chênh lệch
        // giá KHÔNG tích luỹ vị thế ròng, khác hẳn bản chỉ đặt một chân.
        let p = generate_session(8_000, 0x7777, 10_000);
        let mut h = Ecosystem::new(
            ChainVenue::new(2_000_000, 20_000_000_000, 30),
            LatencyModel::typical(),
            ReplaySpeed::Unbounded,
        );
        let mut cls: Vec<Box<dyn Strategy>> = vec![Box::new(CrossVenueArb::new(20.0))];
        h.run(&p, &mut cls);
        assert!(h.metrics.fill_count > 0, "phải có giao dịch xảy ra");
        assert!(h.hedge_count > 0, "phải có phòng vệ chạy trên sàn còn lại");
        assert_eq!(
            h.position.quantity, 0,
            "phòng vệ theo khối lượng đã khớp phải triệt tiêu vị thế ròng hoàn toàn"
        );
    }

    // ---- hệ sinh thái end-to-end ----

    #[test]
    fn ecosystem_runs_full_session() {
        let p = generate_session(8_000, 0xABC, 10_000);
        let mut h = new_ecosystem();
        let mut cls: Vec<Box<dyn Strategy>> = vec![Box::new(ManagedMaker::new(200))];
        h.run(&p, &mut cls);
        assert!(h.metrics.intents > 0, "chiến lược phải sinh ra ý định");
        assert!(h.metrics.orders_sent > 0, "phải có lệnh ra khỏi cổng rủi ro");
        assert!(h.metrics.fill_count > 0, "phải có lệnh được khớp");
    }

    #[test]
    fn replay_is_deterministic_across_runs() {
        let p = generate_session(8_000, 0xBEEF, 10_000);
        let run = || {
            let mut h = new_ecosystem();
            let mut cls: Vec<Box<dyn Strategy>> = vec![
                Box::new(ManagedMaker::new(200)),
                Box::new(CrossVenueArb::new(50.0)),
            ];
            h.run(&p, &mut cls);
            (h.order_log.clone(), h.position.quantity, h.position.realized_pnl.to_bits())
        };
        assert_eq!(run(), run(), "hai lần chạy phải trùng khớp từng bit");
    }

    #[test]
    fn replay_speed_does_not_change_results() {
        let p = generate_session(6_000, 0xF00D, 10_000);
        let run = |toc| {
            let mut h = Ecosystem::new(
                ChainVenue::new(2_000_000, 20_000_000_000, 30),
                LatencyModel::typical(),
                toc,
            );
            let mut cls: Vec<Box<dyn Strategy>> = vec![Box::new(ManagedMaker::new(200))];
            h.run(&p, &mut cls);
            h.order_log.clone()
        };
        // Đẩy tốc độ chỉ nén thời gian TƯỜNG. Thời gian ẢO không đổi, nên
        // kết quả chiến lược phải y hệt — miễn là không ai đọc đồng hồ thật.
        assert_eq!(run(ReplaySpeed::Unbounded), run(ReplaySpeed::Fast(1_000)));
        assert_eq!(run(ReplaySpeed::Unbounded), run(ReplaySpeed::RealTime));
    }

    #[test]
    fn position_limit_is_never_breached() {
        for hat in [1u64, 99, 12345, 0xDEAD] {
            let p = generate_session(8_000, hat, 10_000);
            let mut h = new_ecosystem();
            h.gate.max_position = 150;
            let mut cls: Vec<Box<dyn Strategy>> = vec![Box::new(ManagedMaker::new(120))];
            h.run(&p, &mut cls);
            assert!(
                h.position.quantity.abs() <= h.gate.max_position,
                "hạt {}: vị thế {} vượt hạn mức {}",
                hat,
                h.position.quantity,
                h.gate.max_position
            );
        }
    }

    #[test]
    fn latency_delays_order_arrival() {
        let p = generate_session(4_000, 5, 10_000);
        let run = |lat| {
            let mut h = Ecosystem::new(
                ChainVenue::new(2_000_000, 20_000_000_000, 30),
                lat,
                ReplaySpeed::Unbounded,
            );
            let mut cls: Vec<Box<dyn Strategy>> = vec![Box::new(ManagedMaker::new(200))];
            h.run(&p, &mut cls);
            h.order_log.first().map(|x| x.0).unwrap_or(0)
        };
        let no = run(LatencyModel::none());
        let co = run(LatencyModel::typical());
        assert!(co > no, "có độ trễ thì lệnh đầu tiên tới sàn muộn hơn");
    }

    #[test]
    fn ignoring_latency_changes_everything() {
        // Bỏ qua độ trễ là dạng nhìn trộm tương lai tinh vi nhất: không ai gọi
        // tên nó như vậy, nhưng nó cho chiến lược khớp ở giá đã không còn tồn tại.
        let p = generate_session(6_000, 0x1234, 10_000);
        let run = |lat| {
            let mut h = Ecosystem::new(
                ChainVenue::new(2_000_000, 20_000_000_000, 30),
                lat,
                ReplaySpeed::Unbounded,
            );
            let mut cls: Vec<Box<dyn Strategy>> = vec![
                Box::new(ManagedMaker::new(200)),
                Box::new(CrossVenueArb::new(50.0)),
            ];
            h.run(&p, &mut cls);
            (h.order_log.clone(), h.metrics.filled_qty)
        };
        assert_ne!(
            run(LatencyModel::none()),
            run(LatencyModel::typical()),
            "backtest bỏ qua độ trễ cho dòng lệnh KHÁC HẲN — đó chính là vấn đề"
        );
    }

    #[test]
    fn kill_switch_halts_new_orders() {
        let p = generate_session(4_000, 77, 10_000);
        let mut h = new_ecosystem();
        h.gate.kill_switch_on = true;
        let mut cls: Vec<Box<dyn Strategy>> = vec![Box::new(ManagedMaker::new(200))];
        h.run(&p, &mut cls);
        assert_eq!(h.metrics.orders_sent, 0);
        assert!(h.metrics.orders_blocked > 0);
        assert_eq!(h.position.quantity, 0);
    }

    #[test]
    fn both_venues_get_updated() {
        let p = generate_session(6_000, 0x5EED, 10_000);
        let mut h = new_ecosystem();
        let x0 = h.venue_chain.reserve_x;
        let mut cls: Vec<Box<dyn Strategy>> = vec![Box::new(ManagedMaker::new(200))];
        h.run(&p, &mut cls);
        assert_ne!(h.venue_chain.reserve_x, x0, "sự kiện chuỗi khối phải làm bể đổi");
        assert!(h.venue_lit.mid().is_some(), "sổ truyền thống phải có hai chiều");
    }

    #[test]
    fn log_records_only_gated_orders() {
        let p = generate_session(5_000, 0x99, 10_000);
        let mut h = new_ecosystem();
        let mut cls: Vec<Box<dyn Strategy>> = vec![Box::new(ManagedMaker::new(200))];
        h.run(&p, &mut cls);
        assert_eq!(h.order_log.len() as u64, h.metrics.orders_sent);
        assert_eq!(h.metrics.intents, h.metrics.orders_sent + h.metrics.orders_blocked);
    }

    #[test]
    fn log_timestamps_never_decrease() {
        let p = generate_session(6_000, 0x2468, 10_000);
        let mut h = new_ecosystem();
        let mut cls: Vec<Box<dyn Strategy>> = vec![Box::new(ManagedMaker::new(200))];
        h.run(&p, &mut cls);
        assert!(
            h.order_log.windows(2).all(|w| w[0].0 <= w[1].0),
            "lệnh phải tới sàn theo đúng thứ tự thời gian, dù dao động đảo thứ tự phát"
        );
    }

    #[test]
    fn max_drawdown_is_non_negative() {
        let p = generate_session(5_000, 0x1111, 10_000);
        let mut h = new_ecosystem();
        let mut cls: Vec<Box<dyn Strategy>> = vec![Box::new(ManagedMaker::new(200))];
        h.run(&p, &mut cls);
        assert!(h.metrics.max_drawdown() >= 0.0);
    }

    #[test]
    fn two_strategies_emit_more_intents() {
        let p = generate_session(6_000, 0x3333, 10_000);
        let count = |n: usize| {
            let mut h = new_ecosystem();
            let mut cls: Vec<Box<dyn Strategy>> = if n == 1 {
                vec![Box::new(ManagedMaker::new(200))]
            } else {
                vec![
                    Box::new(ManagedMaker::new(200)),
                    Box::new(CrossVenueArb::new(1.0)),
                ]
            };
            h.run(&p, &mut cls);
            h.metrics.intents
        };
        assert!(count(2) > count(1), "thêm chiến lược thì phải có thêm ý định");
    }
}
