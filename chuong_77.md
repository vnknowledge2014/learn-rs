# Chương 77: Chiến lược & Quản trị rủi ro — Cổng rủi ro, Tín hiệu & Định cỡ vị thế (Strategy & Risk Management)

## Giới thiệu & Mục tiêu học tập

Một chiến lược lãi mà không có kiểm soát rủi ro thì chỉ là **quả bom hẹn giờ**. Lịch sử ngành có sẵn ví dụ: Knight Capital mất 440 triệu đô trong 45 phút năm 2012, vì một đoạn mã cũ được bật nhầm và **không có gì chặn nó lại**.

Chương này dựng ba lớp mà mọi bàn giao dịch chuyên nghiệp đều có:

| Lớp | Nhiệm vụ | Đặc điểm |
|---|---|---|
| Cổng rủi ro trước lệnh | Chặn lệnh xấu trước khi ra khỏi máy | Phải chạy trên **mọi** lệnh, không ngoại lệ |
| Tín hiệu | Biến sổ lệnh thành dự báo | Đơn giản và giải thích được, không phải hộp đen |
| Định cỡ vị thế | Quyết định đặt bao nhiêu | Sai chỗ này thì tín hiệu tốt vẫn phá sản |

Nguyên tắc xuyên suốt: **cổng rủi ro nằm trên đường nóng và không được phép bỏ qua**. Nó phải nhanh (chỉ vài phép so sánh số nguyên — cỡ vài chục nanosecond) để không ai có động cơ tắt nó đi.

---

## Hình tượng hóa đời sống

```
┌──────────────────────────────────────────────────────────────────────────────┐
│  CỔNG RỦI RO = TRẠM KIỂM SOÁT KHÔNG CÓ ĐƯỜNG VÒNG                           │
│                                                                              │
│    chiến lược ──► ┌──────────────────────────┐ ──► sàn                      │
│                   │  1. Công tắc ngắt bật?   │                              │
│                   │  2. Giá hợp lệ?          │                              │
│                   │  3. Khối lượng ≤ hạn?    │                              │
│                   │  4. Giá trị lệnh ≤ hạn?  │                              │
│                   │  5. Vị thế sau lệnh ≤ ?  │                              │
│                   │  6. Lỗ trong ngày ≤ ?    │                              │
│                   │  7. Tốc độ gửi ≤ ?       │                              │
│                   └──────────────────────────┘                              │
│                                                                              │
│    KHÔNG CÓ cờ "bỏ qua kiểm tra". Không có "chế độ khẩn cấp".               │
│    Knight Capital 2012: 440 triệu đô trong 45 phút vì thiếu đúng cái này.   │
│                                                                              │
│  MẤT CÂN BẰNG SỔ LỆNH = ĐẾM NGƯỜI XẾP HÀNG HAI BÊN                         │
│                                                                              │
│     mua 900  ████████████████████                                           │
│     bán 100  ██                                                             │
│     mất cân bằng = (900−100)/(900+100) = +0,8  → áp lực MUA mạnh            │
│                                                                              │
│  VI GIÁ (micro-price) = GIÁ GIỮA CÓ TRỌNG SỐ NGƯỢC                          │
│                                                                              │
│     mua 100.50 × 900   |   bán 100.52 × 100                                 │
│     giá giữa thường = 100.51   (ngây thơ)                                   │
│     vi giá = (100.50×100 + 100.52×900)/1000 = 100.518                       │
│              └──trọng số NGƯỢC: bên NHIỀU khối lượng KÉO GIÁ về phía kia   │
│                                                                              │
│     Trực giác: bên mua đông nghĩa là áp lực mua chưa được thoả mãn.         │
│     Giá "công bằng" nghiêng về phía bên bán mỏng.                           │
│                                                                              │
│  CÔNG THỨC KELLY = CƯỢC BAO NHIÊU THÌ TỐI ƯU?                              │
│                                                                              │
│     f* = (p·b − q) / b     p=xác suất thắng, b=tỉ lệ thắng/thua             │
│                                                                              │
│     Kelly toàn phần tối đa hoá tăng trưởng dài hạn NHƯNG dao động khủng     │
│     khiếp: sụt 50% là chuyện thường. Thực tế dùng ¼ đến ½ Kelly.            │
│     Lý do: bạn KHÔNG biết p chính xác. Ước lượng p cao hơn thật một chút   │
│     là đủ để Kelly toàn phần cược gấp đôi mức đúng → tăng trưởng về 0.     │
└──────────────────────────────────────────────────────────────────────────────┘
```

---

## Khái niệm & Cơ chế kỹ thuật chuyên sâu

### 1. Lãi lỗ trung bình giá vốn — chỗ mọi người làm sai

Đây là lỗi thật đã xảy ra khi xây chương này: tính lãi lỗ đã chốt bằng cách **cộng dòng tiền của giao dịch đóng**, thay vì tính chênh lệch so với giá vốn.

Công thức đúng cần theo dõi `cost_basis` (giá vốn trung bình) và xử lý ba trường hợp:

- **Mở rộng vị thế** (cùng chiều): cập nhật giá vốn trung bình có trọng số.
- **Đóng bớt** (ngược chiều, chưa vượt): chốt lãi lỗ = `(giá − giá_vốn) × khối_lượng × dấu_vị_thế`.
- **Đảo chiều** (ngược chiều, vượt qua 0): chốt toàn bộ phần cũ, rồi giá vốn mới là giá của phần dư.

Trường hợp đảo chiều là chỗ hay sai nhất. Nếu xử lý nó như "đóng bớt" thông thường, giá vốn sẽ sai và mọi con số sau đó đều sai theo.

### 2. Thứ tự các phép kiểm tra là một quyết định thiết kế

Trong khi xây chương này, một lỗi tinh vi xuất hiện: kiểm tra **giá trị lệnh** đứng trước kiểm tra **vị thế**, nên nhiều bài kiểm thử không bao giờ chạm tới nhánh vị thế — chúng bị chặn sớm hơn.

Bài học rộng hơn: khi cổng có nhiều luật, **luật nào chặn trước sẽ che khuất luật sau**. Điều đó ảnh hưởng tới cả kiểm thử lẫn chẩn đoán sản xuất — thông báo lỗi bạn nhận được không nhất thiết là vấn đề nghiêm trọng nhất.

Thứ tự hợp lý là: rẻ trước, đắt sau; và trong nhóm cùng chi phí thì nghiêm trọng trước.

### 3. Vi giá: vì sao trọng số lại ngược

Vi giá là

```
vi_giá = (giá_mua × KL_bán + giá_bán × KL_mua) / (KL_mua + KL_bán)
```

Chú ý: khối lượng **bán** nhân với giá **mua**. Trọng số ngược, và đó là chủ ý.

Trực giác: nếu bên mua có 900 đơn vị và bên bán chỉ 100, thì có rất nhiều người muốn mua chưa được thoả mãn. Áp lực đó sẽ đẩy giá lên, nên giá "công bằng" phải gần phía bán hơn. Trọng số ngược tạo ra đúng hiệu ứng đó.

Về mặt thực nghiệm, vi giá là **dự báo tốt hơn** giá giữa cho giá giữa ở thời điểm tiếp theo. Đó là một trong những tín hiệu đơn giản nhất mà thực sự hoạt động.

### 4. Kelly và vì sao không ai dùng Kelly toàn phần

Công thức Kelly (Kelly criterion) `f* = (p·b − q)/b` tối đa hoá tốc độ tăng trưởng logarit dài hạn. Về mặt toán học nó tối ưu. Về mặt thực hành nó nguy hiểm, vì hai lý do:

- **Dao động khủng khiếp.** Với Kelly toàn phần, sụt giảm 50% từ đỉnh là chuyện bình thường, không phải bất thường.
- **Bạn không biết `p`.** Nếu ước lượng xác suất thắng cao hơn thật chỉ vài phần trăm, Kelly toàn phần trở thành cược vượt mức. Cược gấp đôi Kelly thật thì tăng trưởng dài hạn về **0**; vượt quá mức đó thì tăng trưởng **âm** — dù mỗi cược đều có kỳ vọng dương. Ví dụ: thắng thật 55% (ăn 1 thua 1) thì Kelly là 10%; ước lượng nhầm thành 60% là đã cược 20% — đúng ngưỡng tăng trưởng bằng 0.

Điểm thứ hai là điểm quan trọng: sai lầm theo hướng cược quá nhiều bị phạt **nặng hơn nhiều** so với cược quá ít. Vì thế thực tế dùng ¼ đến ½ Kelly, chấp nhận tăng trưởng chậm hơn để đổi lấy khả năng sống sót.

### 5. Sụt giảm quan trọng hơn lợi nhuận

Một chiến lược lãi 20%/năm với sụt giảm tối đa 5% thì đầu tư được. Cùng chiến lược đó với sụt giảm 40% thì không — không phải vì toán học, mà vì **không ai chịu được**: nhà đầu tư rút vốn, ban lãnh đạo cắt hạn mức, và người vận hành mất niềm tin đúng lúc đáy.

Tỉ số Sharpe (Sharpe ratio) đo lợi nhuận trên đơn vị biến động (bản trong chương tính theo từng phiên, không trừ lãi suất phi rủi ro và không quy ra năm). Nhưng nó phạt biến động **tăng** giống hệt biến động **giảm** — điều mà không nhà đầu tư nào đồng ý. Đó là lý do phải nhìn cả sụt giảm tối đa, và vì sao chương này tính cả hai.

Một lưu ý kỹ thuật nhỏ nhưng thú vị: một đường vốn **tăng tuyệt đối đều đặn** có độ lệch chuẩn bằng 0, nên Sharpe bằng 0 (hoặc vô định). Đó là lý do bài kiểm thử "chiến lược mượt" trong chương này phải thêm nhiễu nhỏ — đường vốn hoàn hảo không tồn tại, và công thức giả định điều đó.

---

## Mã nguồn minh họa thực chiến

Chạy bằng `cargo run -p ch77`, kiểm thử bằng `cargo test -p ch77`.

```rust
#![allow(dead_code)]
//! Chương 77 — Chiến lược & Quản trị rủi ro thời gian thực: cổng rủi ro trước
//! giao dịch, tín hiệu từ sổ lệnh, arbitrage thống kê theo cặp, định cỡ vị thế,
//! và các thước đo rủi ro.
//!
//! Nguyên tắc xuyên suốt: **cổng rủi ro là thứ DUY NHẤT không được phép có
//! ngoại lệ**. Chiến lược có thể sai; cổng rủi ro thì không.

use std::collections::VecDeque;

pub type Price = i64; // tick
pub type Quantity = i64;

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum Side {
    Buy,
    Sell,
}

impl Side {
    pub fn sign(self) -> i64 {
        match self {
            Side::Buy => 1,
            Side::Sell => -1,
        }
    }
}

// ============================================================================
// 1. CỔNG RỦI RO TRƯỚC GIAO DỊCH
// ============================================================================
// Mọi lệnh đều phải qua đây. Không có đường vòng, không có cờ "bỏ qua kiểm
// tra cho nhanh". Lịch sử ngành đầy những vụ sập vì ai đó mở một đường vòng.

#[derive(Debug, Clone, PartialEq)]
pub enum RejectReason {
    NonPositiveQuantity(Quantity),
    NonPositivePrice(Price),
    /// Ngón tay béo: giá lệch quá xa giá thị trường — gần như chắc chắn gõ nhầm.
    FatFinger {
        price: Price,
        reference: Price,
        deviation_pct: f64,
    },
    ExceedsOrderValue {
        value: i64,
        limit: i64,
    },
    ExceedsPosition {
        position_after: i64,
        limit: i64,
    },
    ExceedsDailyLoss {
        loss: i64,
        limit: i64,
    },
    ExceedsOrderRate {
        count: u32,
        limit: u32,
    },
    KillSwitchOn,
}

#[derive(Debug, Clone)]
pub struct RiskLimits {
    pub max_order_value: i64,
    pub max_position: i64,
    pub max_daily_loss: i64,
    pub max_orders_per_second: u32,
    /// Lệch quá tỉ lệ này so với giá tham chiếu thì coi là gõ nhầm.
    pub fat_finger_threshold: f64,
}

impl Default for RiskLimits {
    fn default() -> Self {
        RiskLimits {
            max_order_value: 100_000_000,
            max_position: 10_000,
            max_daily_loss: 5_000_000,
            max_orders_per_second: 100,
            fat_finger_threshold: 0.10, // 10%
        }
    }
}

#[derive(Debug, Clone)]
pub struct RiskGate {
    pub limit: RiskLimits,
    pub position: i64,
    pub realized_pnl: i64,
    /// Giá vốn bình quân của vị thế đang mở. KHÔNG có nó thì không tính được
    /// lãi/lỗ — chỉ biết dòng tiền, mà dòng tiền không phải lãi.
    pub cost_basis: f64,
    /// Dấu thời gian các lệnh gần đây, để đếm tần suất.
    order_times: VecDeque<u64>,
    /// Công tắc tắt: bật rồi thì KHÔNG tự tắt được. Chỉ người mới gỡ được.
    kill_switch: bool,
    pub orders_passed: u64,
    pub orders_blocked: u64,
}

impl RiskGate {
    pub fn new(limit: RiskLimits) -> Self {
        RiskGate {
            limit,
            position: 0,
            realized_pnl: 0,
            cost_basis: 0.0,
            order_times: VecDeque::new(),
            kill_switch: false,
            orders_passed: 0,
            orders_blocked: 0,
        }
    }

    pub fn is_killed(&self) -> bool {
        self.kill_switch
    }
    /// Bật công tắc tắt. Một chiều — chỉ người vận hành mới gỡ được.
    pub fn trip_kill_switch(&mut self) {
        self.kill_switch = true;
    }
    pub fn operator_reset(&mut self) {
        self.kill_switch = false;
    }

    /// Kiểm tra một lệnh. `now_ns` dùng cho cửa sổ tần suất.
    pub fn check(
        &mut self,
        side: Side,
        price: Price,
        quantity: Quantity,
        reference_price: Price,
        now_ns: u64,
    ) -> Result<(), RejectReason> {
        let result = self.check_inner(side, price, quantity, reference_price, now_ns);
        match &result {
            Ok(()) => {
                self.orders_passed += 1;
                self.order_times.push_back(now_ns);
            }
            Err(_) => self.orders_blocked += 1,
        }
        result
    }

    fn check_inner(
        &mut self,
        side: Side,
        price: Price,
        quantity: Quantity,
        reference_price: Price,
        now_ns: u64,
    ) -> Result<(), RejectReason> {
        // Công tắc tắt xét ĐẦU TIÊN. Đã tắt thì không gì lọt qua được.
        if self.kill_switch {
            return Err(RejectReason::KillSwitchOn);
        }
        if quantity <= 0 {
            return Err(RejectReason::NonPositiveQuantity(quantity));
        }
        if price <= 0 {
            return Err(RejectReason::NonPositivePrice(price));
        }

        // Ngón tay béo: gõ 8400 thành 84000 là chuyện xảy ra hằng năm
        if reference_price > 0 {
            let deviation = (price - reference_price).abs() as f64 / reference_price as f64;
            if deviation > self.limit.fat_finger_threshold {
                return Err(RejectReason::FatFinger {
                    price,
                    reference: reference_price,
                    deviation_pct: deviation * 100.0,
                });
            }
        }

        let value = price * quantity;
        if value > self.limit.max_order_value {
            return Err(RejectReason::ExceedsOrderValue {
                value,
                limit: self.limit.max_order_value,
            });
        }

        let position_after = self.position + side.sign() * quantity;
        if position_after.abs() > self.limit.max_position {
            return Err(RejectReason::ExceedsPosition {
                position_after,
                limit: self.limit.max_position,
            });
        }

        if self.realized_pnl < -self.limit.max_daily_loss {
            return Err(RejectReason::ExceedsDailyLoss {
                loss: -self.realized_pnl,
                limit: self.limit.max_daily_loss,
            });
        }

        // Cửa sổ trượt một giây
        while let Some(&t) = self.order_times.front() {
            if now_ns.saturating_sub(t) >= 1_000_000_000 {
                self.order_times.pop_front();
            } else {
                break;
            }
        }
        let count = self.order_times.len() as u32;
        if count >= self.limit.max_orders_per_second {
            return Err(RejectReason::ExceedsOrderRate {
                count,
                limit: self.limit.max_orders_per_second,
            });
        }
        Ok(())
    }

    /// Ghi nhận một lần khớp — cập nhật vị thế, giá vốn và lãi/lỗ đã chốt.
    ///
    /// Điểm dễ sai nhất trong cả chương: lãi/lỗ KHÔNG phải dòng tiền của lệnh
    /// đóng. Bán 100 cổ giá 88,00 mang về tiền, nhưng nếu mua vào ở 90,00 thì
    /// đó là một khoản LỖ. Muốn biết lãi hay lỗ, bắt buộc phải nhớ GIÁ VỐN.
    pub fn record_fill(&mut self, side: Side, price: Price, quantity: Quantity) {
        let prev = self.position;
        let d = side.sign() * quantity;

        if prev == 0 || prev.signum() == d.signum() {
            // Mở mới hoặc mở thêm cùng chiều → bình quân lại giá vốn
            let total = (prev.abs() + quantity) as f64;
            self.cost_basis =
                (self.cost_basis * prev.abs() as f64 + price as f64 * quantity as f64) / total;
            self.position = prev + d;
        } else {
            // Đóng bớt hoặc đóng hết → hiện thực hoá lãi/lỗ phần đóng được
            let closed = quantity.min(prev.abs());
            self.realized_pnl +=
                ((price as f64 - self.cost_basis) * closed as f64 * prev.signum() as f64) as i64;
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
            self.kill_switch = true;
        }
    }
}

// ============================================================================
// 2. TÍN HIỆU TỪ SỔ LỆNH
// ============================================================================

/// Mất cân bằng khối lượng hai bên, chuẩn hoá về [-1, 1].
/// Dương = áp lực mua. Đây là tín hiệu đơn giản nhất mà vẫn có sức dự báo thật.
pub fn imbalance(bid_qty: u64, ask_qty: u64) -> f64 {
    let total = bid_qty + ask_qty;
    if total == 0 {
        return 0.0;
    }
    (bid_qty as f64 - ask_qty as f64) / total as f64
}

/// Giá vi mô: giá giữa có gia quyền theo khối lượng ĐỐI ỨNG.
/// Nhiều người muốn mua → giá vi mô lệch về phía giá bán.
pub fn micro_price(bid_price: Price, bid_qty: u64, ask_price: Price, ask_qty: u64) -> Option<f64> {
    let total = bid_qty + ask_qty;
    if total == 0 {
        return None;
    }
    Some((bid_price as f64 * ask_qty as f64 + ask_price as f64 * bid_qty as f64) / total as f64)
}

/// Cửa sổ trượt tính trung bình và độ lệch chuẩn — O(1) mỗi lần thêm.
#[derive(Debug, Clone)]
pub struct StatsWindow {
    values: VecDeque<f64>,
    capacity: usize,
    total: f64,
    sum_of_squares: f64,
}

impl StatsWindow {
    pub fn new(capacity: usize) -> Self {
        StatsWindow {
            values: VecDeque::with_capacity(capacity),
            capacity,
            total: 0.0,
            sum_of_squares: 0.0,
        }
    }
    pub fn push(&mut self, x: f64) {
        if self.values.len() == self.capacity
            && let Some(oldest) = self.values.pop_front()
        {
            self.total -= oldest;
            self.sum_of_squares -= oldest * oldest;
        }
        self.values.push_back(x);
        self.total += x;
        self.sum_of_squares += x * x;
    }
    pub fn len(&self) -> usize {
        self.values.len()
    }
    pub fn is_empty(&self) -> bool {
        self.values.is_empty()
    }
    pub fn is_full(&self) -> bool {
        self.values.len() == self.capacity
    }
    pub fn mean(&self) -> f64 {
        if self.values.is_empty() {
            0.0
        } else {
            self.total / self.values.len() as f64
        }
    }
    /// Phương sai mẫu (chia n−1). Trả 0 khi chưa đủ 2 điểm.
    pub fn variance(&self) -> f64 {
        let n = self.values.len() as f64;
        if n < 2.0 {
            return 0.0;
        }
        let var = (self.sum_of_squares - self.total * self.total / n) / (n - 1.0);
        var.max(0.0) // chặn sai số dấu phẩy động làm ra số âm
    }
    pub fn stddev(&self) -> f64 {
        self.variance().sqrt()
    }
    /// Điểm z: giá trị này lệch bao nhiêu độ lệch chuẩn so với trung bình.
    pub fn z_score(&self, x: f64) -> Option<f64> {
        let s = self.stddev();
        if s < 1e-9 {
            None
        } else {
            Some((x - self.mean()) / s)
        }
    }
}

// ============================================================================
// 3. ARBITRAGE THỐNG KÊ THEO CẶP
// ============================================================================
// Ý tưởng: hai mã cùng ngành thường đi cùng nhau. Khi chênh lệch giãn bất
// thường, đặt cược nó sẽ co lại. Rủi ro lớn nhất KHÔNG phải chênh lệch không
// co, mà là quan hệ giữa hai mã ĐÃ GÃY HẲN mà ta không nhận ra.

#[derive(Debug, Clone, Copy, PartialEq)]
pub enum PairSignal {
    OpenLongA,
    OpenLongB,
    Close,
    Hold,
}

pub struct PairArb {
    pub hedge_ratio: f64, // beta: 1 đơn vị A ứng với bao nhiêu đơn vị B
    pub window: StatsWindow,
    pub entry_threshold: f64,
    pub exit_threshold: f64,
    /// Chênh lệch giãn quá mức này thì coi như quan hệ đã gãy — CẮT LỖ.
    pub stop_threshold: f64,
    pub open_signal: Option<PairSignal>,
}

impl PairArb {
    pub fn new(
        hedge_ratio: f64,
        window: usize,
        entry_threshold: f64,
        exit_threshold: f64,
        stop_threshold: f64,
    ) -> Self {
        PairArb {
            hedge_ratio,
            window: StatsWindow::new(window),
            entry_threshold,
            exit_threshold,
            stop_threshold,
            open_signal: None,
        }
    }

    pub fn spread(&self, price_a: Price, price_b: Price) -> f64 {
        price_a as f64 - self.hedge_ratio * price_b as f64
    }

    pub fn update(&mut self, price_a: Price, price_b: Price) -> PairSignal {
        let spread = self.spread(price_a, price_b);
        // Tính điểm z TRƯỚC khi thêm điểm mới — nếu không, chính điểm dị
        // biệt ta muốn phát hiện lại kéo trung bình về phía nó và tự che mình.
        let window_was_full = self.window.is_full();
        let z = self.window.z_score(spread);
        self.window.push(spread);

        let z = match z {
            Some(z) if window_was_full => z,
            _ => return PairSignal::Hold,
        };

        match self.open_signal {
            None => {
                if z > self.entry_threshold {
                    // A đắt bất thường so với B → bán A, mua B
                    self.open_signal = Some(PairSignal::OpenLongB);
                    PairSignal::OpenLongB
                } else if z < -self.entry_threshold {
                    self.open_signal = Some(PairSignal::OpenLongA);
                    PairSignal::OpenLongA
                } else {
                    PairSignal::Hold
                }
            }
            Some(_) => {
                // Cắt lỗ đứng TRƯỚC chốt lời: quan hệ gãy thì phải thoát ngay
                if z.abs() > self.stop_threshold || z.abs() < self.exit_threshold {
                    self.open_signal = None;
                    PairSignal::Close
                } else {
                    PairSignal::Hold
                }
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
pub fn kelly_fraction(win_prob: f64, win_loss_ratio: f64) -> f64 {
    if win_loss_ratio <= 0.0 {
        return 0.0;
    }
    let q = 1.0 - win_prob;
    ((win_prob * win_loss_ratio - q) / win_loss_ratio).max(0.0)
}

pub fn fractional_kelly(win_prob: f64, win_loss_ratio: f64, part: f64) -> f64 {
    (kelly_fraction(win_prob, win_loss_ratio) * part).clamp(0.0, 1.0)
}

/// Định cỡ theo mục tiêu biến động: mã càng dao động mạnh thì mua càng ít,
/// sao cho rủi ro tính bằng tiền là như nhau ở mọi mã.
pub fn size_by_volatility(capital: i64, target_vol: f64, asset_vol: f64, price: Price) -> Quantity {
    if asset_vol <= 0.0 || price <= 0 {
        return 0;
    }
    let weight = (target_vol / asset_vol).min(1.0);
    ((capital as f64 * weight) / price as f64) as Quantity
}

// ============================================================================
// 5. THƯỚC ĐO RỦI RO
// ============================================================================

#[derive(Debug, PartialEq)]
pub struct RiskMetrics {
    pub total_pnl: i64,
    pub max_drawdown: i64,
    /// Sụt giảm lớn nhất tính theo TỈ LỆ so với đỉnh ngay trước nó.
    pub ratio_drawdown: f64,
    pub winning_sessions: usize,
    pub losing_sessions: usize,
    /// Tỉ số lợi nhuận trên độ dao động — càng cao càng "êm". Ở đây tính
    /// theo từng phiên, không trừ lãi suất phi rủi ro và không quy ra năm.
    pub sharpe_ratio: f64,
}

pub fn measure_risk(equity_curve: &[i64]) -> RiskMetrics {
    if equity_curve.len() < 2 {
        return RiskMetrics {
            total_pnl: 0,
            max_drawdown: 0,
            ratio_drawdown: 0.0,
            winning_sessions: 0,
            losing_sessions: 0,
            sharpe_ratio: 0.0,
        };
    }
    let mut peak = equity_curve[0];
    let mut dd = 0i64;
    let mut ratio_dd = 0.0f64;
    for &v in equity_curve {
        peak = peak.max(v);
        dd = dd.max(peak - v);
        // Tỉ lệ phải chia cho đỉnh NGAY TRƯỚC cú sụt, không phải đỉnh cao
        // nhất cả chuỗi — nếu không, cú sụt sớm bị đánh giá nhẹ đi.
        if peak > 0 {
            ratio_dd = ratio_dd.max((peak - v) as f64 / peak as f64);
        }
    }
    let deltas: Vec<f64> = equity_curve
        .windows(2)
        .map(|w| (w[1] - w[0]) as f64)
        .collect();
    let n = deltas.len() as f64;
    let mean = deltas.iter().sum::<f64>() / n;
    let var = deltas.iter().map(|x| (x - mean).powi(2)).sum::<f64>() / (n - 1.0).max(1.0);
    let sd = var.sqrt();
    RiskMetrics {
        total_pnl: equity_curve[equity_curve.len() - 1] - equity_curve[0],
        max_drawdown: dd,
        ratio_drawdown: ratio_dd,
        winning_sessions: deltas.iter().filter(|&&x| x > 0.0).count(),
        losing_sessions: deltas.iter().filter(|&&x| x < 0.0).count(),
        sharpe_ratio: if sd < 1e-12 { 0.0 } else { mean / sd },
    }
}

// ============================================================================
// 6. SINH DỮ LIỆU TẤT ĐỊNH
// ============================================================================

/// Hai chuỗi giá đồng liên kết: chúng cùng đi theo một nhân tố chung, cộng
/// thêm nhiễu riêng. Đây đúng là tình huống mà arbitrage cặp khai thác.
pub fn gen_price_pair(n: usize, seed: u64, beta: f64) -> (Vec<Price>, Vec<Price>) {
    let mut s = seed;
    let mut common_factor = 10_000.0f64;
    let (mut a, mut b) = (Vec::with_capacity(n), Vec::with_capacity(n));
    for _ in 0..n {
        s = s
            .wrapping_mul(6364136223846793005)
            .wrapping_add(1442695040888963407);
        let e1 = ((s >> 33) % 201) as f64 - 100.0;
        let e2 = ((s >> 45) % 61) as f64 - 30.0;
        let e3 = ((s >> 20) % 61) as f64 - 30.0;
        common_factor += e1 * 0.1;
        a.push((common_factor + e2) as Price);
        b.push(((common_factor + e3) / beta) as Price);
    }
    (a, b)
}

fn main() {
    println!("═══════════════════════════════════════════════════════════");
    println!("   CHIẾN LƯỢC & QUẢN TRỊ RỦI RO THỜI GIAN THỰC             ");
    println!("═══════════════════════════════════════════════════════════");

    println!("\n1. CỔNG RỦI RO — mọi lệnh đều phải qua đây");
    let mut gate = RiskGate::new(RiskLimits {
        max_order_value: 10_000_000,
        max_position: 500,
        max_daily_loss: 100_000,
        max_orders_per_second: 5,
        fat_finger_threshold: 0.10,
    });
    let reference = 8_400;
    for (description, side, price, qty) in [
        ("hợp lệ            ", Side::Buy, 8_400i64, 100i64),
        ("ngón tay béo x10  ", Side::Buy, 84_000, 100),
        ("giá trị quá lớn   ", Side::Buy, 8_400, 100_000),
        ("số lượng âm       ", Side::Buy, 8_400, -5),
        ("vượt trần vị thế  ", Side::Buy, 8_400, 600),
    ] {
        match gate.check(side, price, qty, reference, 1_000_000_000) {
            Ok(()) => println!("   {} → CHO QUA", description),
            Err(e) => println!("   {} → CHẶN: {:?}", description, e),
        }
    }

    println!("\n2. GIỚI HẠN TẦN SUẤT — chống vòng lặp lỗi bắn lệnh liên tục");
    let mut rate_gate = RiskGate::new(RiskLimits {
        max_orders_per_second: 5,
        ..Default::default()
    });
    let mut passed = 0;
    for i in 0..10u64 {
        if rate_gate
            .check(
                Side::Buy,
                8_400,
                1,
                reference,
                1_000_000_000 + i * 1_000_000,
            )
            .is_ok()
        {
            passed += 1;
        }
    }
    println!(
        "   Bắn 10 lệnh trong 10 ms → chỉ {} lệnh lọt qua (trần 5/giây)",
        passed
    );

    println!("\n3. CÔNG TẮC TẮT TỰ ĐỘNG KHI LỖ CHẠM TRẦN");
    let mut loss_gate = RiskGate::new(RiskLimits {
        max_daily_loss: 10_000,
        ..Default::default()
    });
    loss_gate.record_fill(Side::Buy, 9_000, 100);
    loss_gate.record_fill(Side::Sell, 8_800, 100); // lỗ 20 000
    println!(
        "   Sau khi lỗ {} → công tắc tắt: {}",
        -loss_gate.realized_pnl,
        loss_gate.is_killed()
    );
    println!(
        "   Lệnh tiếp theo → {:?}",
        loss_gate
            .check(Side::Buy, 8_400, 1, reference, 2_000_000_000)
            .unwrap_err()
    );
    loss_gate.operator_reset();
    println!(
        "   Người vận hành gỡ công tắc → lệnh kế tiếp: {:?}",
        loss_gate.check(Side::Buy, 8_400, 1, reference, 3_000_000_000)
    );
    println!("   → Gỡ công tắc chưa đủ: lỗ trong ngày vẫn vượt trần nên lệnh VẪN bị chặn.");

    println!("\n4. TÍN HIỆU TỪ SỔ LỆNH");
    for (m, b) in [(1000u64, 1000u64), (9000, 1000), (1000, 9000)] {
        println!(
            "   mua {:>4} / bán {:>4} → mất cân bằng {:>6.2} · giá vi mô {:>8.2}",
            m,
            b,
            imbalance(m, b),
            micro_price(8_400, m, 8_410, b).unwrap()
        );
    }
    println!("   → Nhiều người chờ mua thì giá vi mô lệch LÊN phía giá bán.");

    println!("\n5. ARBITRAGE CẶP");
    let (ga, gb) = gen_price_pair(3_000, 2024, 1.5);
    let mut arb = PairArb::new(1.5, 100, 2.0, 0.5, 4.0);
    let (mut entries, mut exits) = (0, 0);
    for i in 0..ga.len() {
        match arb.update(ga[i], gb[i]) {
            PairSignal::OpenLongA | PairSignal::OpenLongB => entries += 1,
            PairSignal::Close => exits += 1,
            PairSignal::Hold => {}
        }
    }
    println!(
        "   {} điểm dữ liệu → vào lệnh {} lần · thoát {} lần",
        ga.len(),
        entries,
        exits
    );
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
        println!(
            "   {} {:>7.1}% {:>9.1}%",
            description,
            kelly_fraction(p, b) * 100.0,
            fractional_kelly(p, b, 0.25) * 100.0
        );
    }
    println!("   → Lợi thế âm thì Kelly = 0: công thức tự bảo bạn ĐỪNG đánh.");

    println!("\n7. THƯỚC ĐO RỦI RO — hai đường vốn cùng đích, khác hẳn nhau");
    // "Êm" KHÔNG có nghĩa là đường thẳng tuyệt đối — đường thẳng thì độ lệch
    // chuẩn bằng 0 và Sharpe không định nghĩa được. Êm nghĩa là dao động nhỏ.
    let smooth: Vec<i64> = (0..100).map(|i| 100_000 + i * 500 + (i % 5) * 40).collect();
    let mut choppy: Vec<i64> = Vec::new();
    let mut v = 100_000i64;
    for i in 0..100 {
        v += if i % 3 == 0 { -8_000 } else { 5_750 };
        choppy.push(v);
    }
    for (name, d) in [("êm ", &smooth), ("xóc", &choppy)] {
        let r = measure_risk(d);
        println!(
            "   {} → lãi {:>6} · sụt sâu nhất {:>6} · Sharpe {:>5.2} · thắng {}/{}",
            name,
            r.total_pnl,
            r.max_drawdown,
            r.sharpe_ratio,
            r.winning_sessions,
            r.winning_sessions + r.losing_sessions
        );
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
        RiskGate::new(RiskLimits {
            max_order_value: 10_000_000,
            max_position: 500,
            max_daily_loss: 100_000,
            max_orders_per_second: 5,
            fat_finger_threshold: 0.10,
        })
    }

    // ---------- Cổng rủi ro ----------
    #[test]
    fn a_valid_order_passes() {
        let mut c = sample_gate();
        assert_eq!(c.check(Side::Buy, 8_400, 100, 8_400, 1_000_000_000), Ok(()));
        assert_eq!(c.orders_passed, 1);
        assert_eq!(c.orders_blocked, 0);
    }

    #[test]
    fn blocks_fat_finger_prices() {
        // Gõ 8400 thành 84000 — lỗi có thật, xảy ra hằng năm ở mọi thị trường.
        let mut c = sample_gate();
        let e = c
            .check(Side::Buy, 84_000, 1, 8_400, 1_000_000_000)
            .unwrap_err();
        assert!(matches!(e, RejectReason::FatFinger { .. }));
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
        assert_eq!(
            c.check(Side::Buy, 8_400, 0, 8_400, 1).unwrap_err(),
            RejectReason::NonPositiveQuantity(0)
        );
        assert_eq!(
            c.check(Side::Buy, 8_400, -5, 8_400, 1).unwrap_err(),
            RejectReason::NonPositiveQuantity(-5)
        );
        assert_eq!(
            c.check(Side::Buy, 0, 10, 0, 1).unwrap_err(),
            RejectReason::NonPositivePrice(0)
        );
    }

    #[test]
    fn blocks_oversized_notional() {
        let mut c = sample_gate();
        assert!(matches!(
            c.check(Side::Buy, 8_400, 100_000, 8_400, 1).unwrap_err(),
            RejectReason::ExceedsOrderValue { .. }
        ));
    }

    #[test]
    fn blocks_position_breach_on_both_sides() {
        let mut c = sample_gate();
        assert!(matches!(
            c.check(Side::Buy, 8_400, 501, 8_400, 1).unwrap_err(),
            RejectReason::ExceedsPosition {
                position_after: 501,
                limit: 500
            }
        ));
        assert!(
            matches!(
                c.check(Side::Sell, 8_400, 501, 8_400, 1).unwrap_err(),
                RejectReason::ExceedsPosition {
                    position_after: -501,
                    limit: 500
                }
            ),
            "bán khống cũng phải bị chặn, không chỉ mua"
        );
    }

    #[test]
    fn current_position_counts_toward_the_limit() {
        let mut c = sample_gate();
        c.record_fill(Side::Buy, 8_400, 400);
        assert!(
            c.check(Side::Buy, 8_400, 100, 8_400, 1).is_ok(),
            "400+100 = 500, vừa trần"
        );
        assert!(
            c.check(Side::Buy, 8_400, 101, 8_400, 1).is_err(),
            "400+101 vượt trần"
        );
        assert!(
            c.check(Side::Sell, 8_400, 400, 8_400, 1).is_ok(),
            "bán thì giảm vị thế"
        );
    }

    #[test]
    fn rate_limit_blocks_at_the_right_count() {
        let mut c = sample_gate(); // trần 5 lệnh/giây
        let mut passed = 0;
        for i in 0..20u64 {
            if c.check(Side::Buy, 8_400, 1, 8_400, 1_000_000_000 + i * 1_000_000)
                .is_ok()
            {
                passed += 1;
            }
        }
        assert_eq!(passed, 5, "đúng 5 lệnh lọt qua trong một giây");
    }

    #[test]
    fn the_rate_window_slides_with_time() {
        let mut c = sample_gate();
        for i in 0..5u64 {
            assert!(
                c.check(Side::Buy, 8_400, 1, 8_400, 1_000_000_000 + i)
                    .is_ok()
            );
        }
        assert!(
            c.check(Side::Buy, 8_400, 1, 8_400, 1_000_000_100).is_err(),
            "đã đủ 5"
        );
        // Sang giây sau thì cửa sổ trượt qua, lại cho phép
        assert!(c.check(Side::Buy, 8_400, 1, 8_400, 2_500_000_000).is_ok());
    }

    #[test]
    fn the_kill_switch_blocks_everything_and_never_self_clears() {
        let mut c = sample_gate();
        c.trip_kill_switch();
        // Kể cả lệnh hoàn toàn hợp lệ cũng không lọt
        assert_eq!(
            c.check(Side::Buy, 8_400, 1, 8_400, 1).unwrap_err(),
            RejectReason::KillSwitchOn
        );
        assert!(c.is_killed(), "công tắc KHÔNG được tự tắt sau khi chặn");
        c.operator_reset();
        assert!(c.check(Side::Buy, 8_400, 1, 8_400, 1).is_ok());
    }

    #[test]
    fn hitting_the_loss_cap_trips_the_kill_switch() {
        let mut c = RiskGate::new(RiskLimits {
            max_daily_loss: 10_000,
            ..Default::default()
        });
        assert!(!c.is_killed());
        c.record_fill(Side::Buy, 9_000, 100);
        c.record_fill(Side::Sell, 8_800, 100); // lỗ 20 000 > trần 10 000
        assert_eq!(c.realized_pnl, -20_000);
        assert!(
            c.is_killed(),
            "vượt trần lỗ phải tự dừng, không chờ người can thiệp"
        );
    }

    #[test]
    fn a_profitable_close_does_not_trip_the_switch() {
        let mut c = RiskGate::new(RiskLimits {
            max_daily_loss: 10_000,
            ..Default::default()
        });
        c.record_fill(Side::Buy, 8_000, 100);
        c.record_fill(Side::Sell, 8_500, 100);
        assert_eq!(c.realized_pnl, 50_000, "mua 80.00 bán 85.00 → lãi");
        assert!(!c.is_killed());
        assert_eq!(c.position, 0);
    }

    #[test]
    fn cost_basis_averages_when_adding() {
        let mut c = sample_gate();
        c.record_fill(Side::Buy, 8_000, 100);
        c.record_fill(Side::Buy, 9_000, 100);
        assert!(
            (c.cost_basis - 8_500.0).abs() < 1e-9,
            "bình quân 8000 và 9000 = 8500"
        );
        c.record_fill(Side::Sell, 8_500, 200);
        assert_eq!(c.realized_pnl, 0, "bán đúng giá vốn thì hoà vốn");
        assert_eq!(c.position, 0);
        assert_eq!(c.cost_basis, 0.0, "đóng hết thì giá vốn phải về 0");
    }

    #[test]
    fn reversing_resets_the_cost_basis() {
        let mut c = sample_gate();
        c.record_fill(Side::Buy, 8_000, 100);
        // Bán 300: đóng 100 (lãi) rồi mở mới 200 ở chiều bán
        c.record_fill(Side::Sell, 8_500, 300);
        assert_eq!(c.position, -200);
        assert_eq!(c.realized_pnl, 50_000, "chỉ phần ĐÓNG mới tính lãi");
        assert!(
            (c.cost_basis - 8_500.0).abs() < 1e-9,
            "phần dư là vị thế mới ở giá 8500"
        );
    }

    #[test]
    fn short_then_cheaper_buyback_is_profitable() {
        let mut c = sample_gate();
        c.record_fill(Side::Sell, 9_000, 100);
        assert_eq!(c.position, -100);
        c.record_fill(Side::Buy, 8_500, 100);
        assert_eq!(
            c.realized_pnl, 50_000,
            "bán khống 90.00 mua lại 85.00 → lãi"
        );
    }

    #[test]
    fn adding_in_the_same_direction_realizes_nothing() {
        let mut c = sample_gate();
        c.record_fill(Side::Buy, 8_000, 100);
        c.record_fill(Side::Buy, 9_000, 100);
        assert_eq!(c.position, 200);
        assert_eq!(c.realized_pnl, 0, "chưa đóng gì thì chưa chốt lãi/lỗ");
    }

    #[test]
    fn counts_passed_and_blocked_orders_correctly() {
        let mut c = sample_gate();
        c.check(Side::Buy, 8_400, 100, 8_400, 1).ok();
        c.check(Side::Buy, 84_000, 100, 8_400, 1).ok();
        c.check(Side::Buy, 8_400, -1, 8_400, 1).ok();
        assert_eq!(c.orders_passed, 1);
        assert_eq!(c.orders_blocked, 2);
    }

    // ---------- Tín hiệu ----------
    #[test]
    fn imbalance_stays_within_minus_one_and_one() {
        assert_eq!(
            imbalance(0, 0),
            0.0,
            "sổ rỗng thì trung tính, không chia cho 0"
        );
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
        let many_buy = micro_price(8_400, 9_000, 8_410, 1_000).unwrap();
        let many_sell = micro_price(8_400, 1_000, 8_410, 9_000).unwrap();
        let balanced = micro_price(8_400, 1_000, 8_410, 1_000).unwrap();
        assert!(many_buy > balanced, "áp lực mua đẩy giá vi mô lên");
        assert!(many_sell < balanced, "áp lực bán kéo xuống");
        assert!(
            (balanced - 8_405.0).abs() < 1e-9,
            "cân bằng thì đúng giá giữa"
        );
        assert!(
            many_buy > 8_400.0 && many_buy < 8_410.0,
            "luôn nằm trong chênh lệch"
        );
    }

    #[test]
    fn micro_price_of_an_empty_book_is_none() {
        assert_eq!(micro_price(8_400, 0, 8_410, 0), None);
    }

    // ---------- Cửa sổ thống kê ----------
    #[test]
    fn window_computes_mean_and_stddev_correctly() {
        let mut c = StatsWindow::new(5);
        for x in [2.0, 4.0, 4.0, 4.0, 5.0] {
            c.push(x);
        }
        assert!((c.mean() - 3.8).abs() < 1e-9);
        // phương sai mẫu của [2,4,4,4,5] = 1.2
        assert!((c.variance() - 1.2).abs() < 1e-9);
        assert!(c.is_full());
    }

    #[test]
    fn the_sliding_window_drops_old_values() {
        let mut c = StatsWindow::new(3);
        for x in [1.0, 2.0, 3.0, 4.0, 5.0] {
            c.push(x);
        }
        assert_eq!(c.len(), 3);
        assert!((c.mean() - 4.0).abs() < 1e-9, "chỉ còn [3,4,5]");
    }

    #[test]
    fn variance_is_never_negative_despite_float_error() {
        let mut c = StatsWindow::new(50);
        for _ in 0..50 {
            c.push(1_000_000.0);
        } // toàn giá trị giống hệt, cỡ lớn
        assert!(c.variance() >= 0.0, "phải chặn sai số làm ra số âm");
        assert!(c.variance() < 1e-3, "dữ liệu không đổi thì phương sai ~0");
        assert_eq!(
            c.z_score(1_000_000.0),
            None,
            "độ lệch ~0 thì điểm z vô nghĩa"
        );
    }

    #[test]
    fn fewer_than_two_points_gives_zero_variance() {
        let mut c = StatsWindow::new(10);
        assert_eq!(c.variance(), 0.0);
        c.push(5.0);
        assert_eq!(c.variance(), 0.0, "một điểm thì không có phương sai mẫu");
    }

    #[test]
    fn z_score_measures_deviation() {
        let mut c = StatsWindow::new(100);
        for i in 0..100 {
            c.push((i % 10) as f64);
        }
        let z = c.z_score(c.mean()).unwrap();
        assert!(z.abs() < 1e-9, "đúng giá trị trung bình thì z = 0");
        let z2 = c.z_score(c.mean() + 2.0 * c.stddev()).unwrap();
        assert!((z2 - 2.0).abs() < 1e-9);
    }

    // ---------- Arbitrage cặp ----------
    #[test]
    fn arb_stays_silent_until_warm() {
        let mut a = PairArb::new(1.5, 100, 2.0, 0.5, 4.0);
        let (ga, gb) = gen_price_pair(50, 1, 1.5);
        for i in 0..50 {
            assert_eq!(
                a.update(ga[i], gb[i]),
                PairSignal::Hold,
                "cửa sổ chưa đầy thì tuyệt đối không được vào lệnh"
            );
        }
    }

    #[test]
    fn arb_enters_on_an_abnormal_spread() {
        let mut a = PairArb::new(1.0, 20, 2.0, 0.5, 10.0);
        // 20 điểm ổn định quanh 0 (có dao động nhỏ để độ lệch chuẩn khác 0)
        for i in 0..20 {
            a.update(10_000 + (i % 3), 10_000);
        }
        // rồi một cú giãn mạnh
        let signal = a.update(10_100, 10_000);
        assert_eq!(
            signal,
            PairSignal::OpenLongB,
            "A đắt bất thường → bán A mua B"
        );
        assert_eq!(a.open_signal, Some(PairSignal::OpenLongB));
    }

    #[test]
    fn arb_never_opens_two_positions_at_once() {
        let mut a = PairArb::new(1.0, 20, 2.0, 0.5, 100.0);
        for i in 0..20 {
            a.update(10_000 + (i % 3), 10_000);
        }
        assert_ne!(a.update(10_100, 10_000), PairSignal::Hold);
        for _ in 0..5 {
            let t = a.update(10_120, 10_000);
            assert!(
                matches!(t, PairSignal::Hold | PairSignal::Close),
                "đang có vị thế thì không được mở thêm"
            );
        }
    }

    #[test]
    fn arb_stops_out_beyond_the_threshold() {
        // Bài học sống còn: chênh lệch giãn mãi nghĩa là quan hệ ĐÃ GÃY,
        // không phải "cơ hội càng tốt hơn". Phải thoát.
        let mut a = PairArb::new(1.0, 20, 2.0, 0.5, 3.0);
        for i in 0..20 {
            a.update(10_000 + (i % 3), 10_000);
        }
        a.update(10_050, 10_000); // vào lệnh
        assert!(a.open_signal.is_some());
        let t = a.update(10_500, 10_000); // giãn cực mạnh
        assert_eq!(t, PairSignal::Close, "vượt ngưỡng dừng phải CẮT LỖ");
        assert_eq!(a.open_signal, None);
    }

    #[test]
    fn the_spread_uses_the_correct_hedge_ratio() {
        let a = PairArb::new(1.5, 10, 2.0, 0.5, 4.0);
        assert!((a.spread(15_000, 10_000) - 0.0).abs() < 1e-9);
        assert!((a.spread(15_150, 10_000) - 150.0).abs() < 1e-9);
    }

    // ---------- Định cỡ ----------
    #[test]
    fn kelly_is_zero_without_an_edge() {
        assert_eq!(
            kelly_fraction(0.5, 1.0),
            0.0,
            "tung đồng xu công bằng → đừng đánh"
        );
        assert_eq!(
            kelly_fraction(0.4, 1.0),
            0.0,
            "lợi thế âm → tuyệt đối đừng đánh"
        );
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
            let full = kelly_fraction(p, 1.0);
            let part = fractional_kelly(p, 1.0, 0.25);
            assert!(part < full);
            assert!((part - full * 0.25).abs() < 1e-9);
        }
    }

    #[test]
    fn kelly_never_divides_by_zero() {
        assert_eq!(kelly_fraction(0.9, 0.0), 0.0);
        assert_eq!(kelly_fraction(0.9, -1.0), 0.0);
    }

    #[test]
    fn size_shrinks_as_volatility_rises() {
        let capital = 1_000_000i64;
        let a = size_by_volatility(capital, 0.10, 0.10, 100);
        let b = size_by_volatility(capital, 0.10, 0.40, 100);
        assert!(b < a, "mã dao động mạnh gấp 4 thì mua ít hơn hẳn");
        assert_eq!(a, 10_000, "biến động khớp mục tiêu → dùng toàn bộ vốn");
        assert_eq!(b, 2_500, "gấp 4 lần biến động → 1/4 tỉ trọng");
    }

    #[test]
    fn vol_sizing_never_levers_beyond_capital() {
        // Mã êm hơn mục tiêu KHÔNG được dẫn tới mua vượt vốn.
        let c = size_by_volatility(1_000_000, 0.40, 0.05, 100);
        assert_eq!(c, 10_000, "tỉ trọng bị chặn ở 1.0, không dùng đòn bẩy ngầm");
    }

    #[test]
    fn vol_sizing_is_safe_on_bad_input() {
        assert_eq!(size_by_volatility(1_000_000, 0.1, 0.0, 100), 0);
        assert_eq!(size_by_volatility(1_000_000, 0.1, 0.1, 0), 0);
        assert_eq!(size_by_volatility(1_000_000, 0.1, -0.5, 100), 0);
    }

    // ---------- Thước đo rủi ro ----------
    #[test]
    fn a_monotonic_equity_curve_has_no_drawdown() {
        let d: Vec<i64> = (0..50).map(|i| 100_000 + i * 100).collect();
        let r = measure_risk(&d);
        assert_eq!(r.max_drawdown, 0);
        assert_eq!(r.losing_sessions, 0);
        assert_eq!(r.total_pnl, 4_900);
    }

    #[test]
    fn drawdown_measures_distance_from_the_peak() {
        let d = vec![100, 150, 120, 80, 130];
        let r = measure_risk(&d);
        assert_eq!(r.max_drawdown, 70, "từ đỉnh 150 xuống đáy 80");
    }

    #[test]
    fn drawdown_ratio_uses_the_peak_before_the_fall() {
        // Sụt từ 100 xuống 50 (−50%), rồi đi lên 1 000. Trước đây tỉ lệ chia
        // cho đỉnh cuối cùng (1 000) nên chỉ báo −5% — che mất cú sụt một nửa.
        let r = measure_risk(&[100, 50, 1_000]);
        assert_eq!(r.max_drawdown, 50);
        assert!(
            (r.ratio_drawdown - 0.5).abs() < 1e-12,
            "{}",
            r.ratio_drawdown
        );
    }

    #[test]
    fn drawdown_is_never_negative() {
        for seed in [1u64, 7, 42] {
            let mut s = seed;
            let d: Vec<i64> = (0..200)
                .map(|_| {
                    s = s.wrapping_mul(6364136223846793005).wrapping_add(1);
                    ((s >> 40) % 200_000) as i64
                })
                .collect();
            assert!(measure_risk(&d).max_drawdown >= 0);
        }
    }

    #[test]
    fn a_smooth_curve_has_a_higher_sharpe() {
        // Cùng đích đến, nhưng đường êm mới là đường người ta đi hết được.
        // Đường "êm" vẫn phải có dao động nhỏ: đường thẳng tuyệt đối cho độ
        // lệch chuẩn 0, và khi đó Sharpe không định nghĩa được (ta trả 0).
        let smooth: Vec<i64> = (0..100).map(|i| 100_000 + i * 500 + (i % 5) * 40).collect();
        let mut choppy = Vec::new();
        let mut v = 100_000i64;
        for i in 0..100 {
            v += if i % 3 == 0 { -8_000 } else { 5_750 };
            choppy.push(v);
        }
        let (a, b) = (measure_risk(&smooth), measure_risk(&choppy));
        assert!(
            a.sharpe_ratio > b.sharpe_ratio,
            "êm {:.2} phải cao hơn xóc {:.2}",
            a.sharpe_ratio,
            b.sharpe_ratio
        );
        assert!(b.max_drawdown > a.max_drawdown);
    }

    #[test]
    fn a_very_short_curve_does_not_panic() {
        assert_eq!(measure_risk(&[]).total_pnl, 0);
        assert_eq!(measure_risk(&[100]).max_drawdown, 0);
        assert_eq!(
            measure_risk(&[100, 100]).sharpe_ratio,
            0.0,
            "không dao động → Sharpe 0"
        );
    }

    // ---------- Sinh dữ liệu ----------
    #[test]
    fn pair_generation_is_deterministic() {
        assert_eq!(gen_price_pair(100, 5, 1.5), gen_price_pair(100, 5, 1.5));
        assert_ne!(gen_price_pair(100, 5, 1.5), gen_price_pair(100, 6, 1.5));
    }

    #[test]
    fn the_two_series_really_do_move_together() {
        // Nếu chúng không đồng biến thì cả chương arbitrage cặp là vô nghĩa.
        let (a, b) = gen_price_pair(2_000, 2024, 1.5);
        let n = a.len() as f64;
        let (mean_a, mean_b) = (
            a.iter().sum::<i64>() as f64 / n,
            b.iter().sum::<i64>() as f64 / n,
        );
        let mut cov = 0.0;
        let (mut sa, mut sb) = (0.0, 0.0);
        for i in 0..a.len() {
            let (da, db) = (a[i] as f64 - mean_a, b[i] as f64 - mean_b);
            cov += da * db;
            sa += da * da;
            sb += db * db;
        }
        let correlation = cov / (sa.sqrt() * sb.sqrt());
        assert!(correlation > 0.8, "tương quan {:.3} phải cao", correlation);
    }
}
```

---

## Bảng tra cứu lỗi biên dịch thường gặp

| Lỗi | Nguyên nhân trong chương này | Cách sửa |
|---|---|---|
| `attempt to subtract with overflow` (panic lúc chạy, bản debug) | `position - quantity` với `u64` | Vị thế phải là `i64` — nó có thể âm |
| Sharpe bằng 0 với chiến lược "hoàn hảo" | Đường vốn đều tuyệt đối → độ lệch chuẩn 0 | Thêm nhiễu nhỏ; đường vốn hoàn hảo không tồn tại |
| Bài kiểm thử không chạm tới nhánh vị thế | Kiểm giá trị lệnh chặn trước | Nới hạn mức giá trị trong dữ liệu kiểm thử |
| Lãi lỗ sai sau khi đảo chiều | Không xử lý riêng trường hợp vượt qua 0 | Chốt phần cũ, đặt giá vốn mới cho phần dư |
| `E0308: mismatched types` (expected `f64`, found `i64`) | Trộn tiền nguyên với thống kê thực, vd `let x: f64 = pnl;` | Ép kiểu tường minh ở đúng biên: `pnl as f64` |

---

## Tóm tắt chương & Bài tập rèn luyện

### 5 điểm cốt lõi

1. **Cổng rủi ro không được có đường vòng.** Nó phải nhanh để không ai muốn tắt, và bắt buộc để không ai tắt được.
2. **Lãi lỗ phải dựa trên giá vốn trung bình**, và trường hợp đảo chiều phải xử lý riêng.
3. **Vi giá dùng trọng số ngược** và dự báo tốt hơn giá giữa — một trong ít tín hiệu đơn giản mà hiệu quả.
4. **Kelly toàn phần là bẫy.** Bạn không biết `p`, và cược quá nhiều bị phạt nặng hơn cược quá ít rất nhiều.
5. **Sụt giảm quan trọng hơn lợi nhuận**, vì nó quyết định bạn có còn ở lại bàn để thu lợi nhuận hay không.

### Bài tập rèn luyện

**Bài 1.** Cài **kiểm soát rủi ro thích ứng**: tự thu hẹp hạn mức khi hiệu suất xấu đi.

<details>
<summary><b>Gợi ý</b></summary>

Hạn mức tĩnh có một vấn đề: chúng đúng cho điều kiện bình thường, và sai đúng lúc bất thường. Rủi ro thích ứng thu hẹp khi thua và nới ra khi thắng — nhưng phải **nới chậm hơn nhiều** so với tốc độ thu, vì phục hồi cần được chứng minh, còn tổn thất thì không.
</details>

<details>
<summary><b>Lời giải</b></summary>

```rust
pub struct AdaptiveRisk {
    pub base_limit: i64,
    pub scale: f64, // 0,25 → 1,0
    pub win_streak: u32,
    pub loss_streak: u32,
}

impl AdaptiveRisk {
    pub fn record(&mut self, pnl: i64) {
        if pnl > 0 {
            self.win_streak += 1;
            self.loss_streak = 0;
            // Nới CHẬM: cần 5 lần thắng liên tiếp mới tăng 10%
            if self.win_streak >= 5 {
                self.scale = (self.scale * 1.1).min(1.0);
                self.win_streak = 0;
            }
        } else if pnl < 0 {
            self.loss_streak += 1;
            self.win_streak = 0;
            // Thu NHANH: 3 lần thua liên tiếp là cắt 30%
            if self.loss_streak >= 3 {
                self.scale = (self.scale * 0.7).max(0.25);
                self.loss_streak = 0;
            }
        }
    }

    pub fn current_limit(&self) -> i64 {
        (self.base_limit as f64 * self.scale) as i64
    }
}
```

Bất đối xứng là chủ ý: 3 lần thua cắt 30%, nhưng phải 5 lần thắng mới nới 10%. Sàn 0,25 bảo đảm hệ thống không tự tắt hoàn toàn — nếu về 0, bạn không bao giờ có dữ liệu để biết chiến lược đã hồi phục chưa.
</details>

**Bài 2.** Cài **hệ thống nhiều tín hiệu có tổ hợp trọng số** và đo tương quan giữa các tín hiệu.

<details>
<summary><b>Gợi ý</b></summary>

Cộng nhiều tín hiệu chỉ hữu ích nếu chúng **độc lập**. Hai tín hiệu có tương quan 0,95 thực chất là một tín hiệu tính hai lần — và bạn sẽ cược gấp đôi vào cùng một ý tưởng mà tưởng mình đang đa dạng hoá.
</details>

<details>
<summary><b>Lời giải</b></summary>

```rust
pub struct SignalBlend {
    pub names: Vec<String>,
    pub weights: Vec<f64>,
    pub history: Vec<Vec<f64>>, // lịch sử giá trị từng tín hiệu
}

impl SignalBlend {
    pub fn blend(&mut self, values: &[f64]) -> f64 {
        for (i, v) in values.iter().enumerate() {
            if i < self.history.len() {
                self.history[i].push(*v);
            }
        }
        let weight_sum: f64 = self.weights.iter().map(|w| w.abs()).sum();
        if weight_sum == 0.0 {
            return 0.0;
        }
        values.iter().zip(&self.weights).map(|(v, w)| v * w).sum::<f64>() / weight_sum
    }

    pub fn correlation(&self, i: usize, j: usize) -> Option<f64> {
        let (a, b) = (self.history.get(i)?, self.history.get(j)?);
        let n = a.len().min(b.len());
        if n < 2 {
            return None;
        }
        let mean_a = a[..n].iter().sum::<f64>() / n as f64;
        let mean_b = b[..n].iter().sum::<f64>() / n as f64;
        let (mut cov, mut var_a, mut var_b) = (0.0, 0.0, 0.0);
        for k in 0..n {
            let (da, db) = (a[k] - mean_a, b[k] - mean_b);
            cov += da * db;
            var_a += da * da;
            var_b += db * db;
        }
        if var_a == 0.0 || var_b == 0.0 {
            return None;
        }
        Some(cov / (var_a * var_b).sqrt())
    }

    /// Các cặp tín hiệu quá giống nhau — chúng KHÔNG đa dạng hoá gì cả.
    pub fn redundant_pairs(&self, threshold: f64) -> Vec<(usize, usize, f64)> {
        let mut out = Vec::new();
        for i in 0..self.history.len() {
            for j in (i + 1)..self.history.len() {
                if let Some(r) = self.correlation(i, j)
                    && r.abs() > threshold
                {
                    out.push((i, j, r));
                }
            }
        }
        out
    }
}
```

`redundant_pairs` là công cụ chẩn đoán quan trọng: nếu hai tín hiệu có tương quan trên 0,8, bạn nên bỏ một hoặc gộp chúng — nếu không, "danh mục tín hiệu" của bạn thực chất chỉ có một ý tưởng được đặt cược nhiều lần.
</details>
