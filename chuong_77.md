# Chương 77: Chiến lược & Quản trị rủi ro — Cổng rủi ro, Tín hiệu & Định cỡ vị thế

## Giới thiệu & Mục tiêu học tập

Một chiến lược lãi mà không có kiểm soát rủi ro thì chỉ là **quả bom hẹn giờ**. Lịch sử ngành có sẵn ví dụ: Knight Capital mất 440 triệu đô trong 45 phút năm 2012, vì một đoạn mã cũ được bật nhầm và **không có gì chặn nó lại**.

Chương này dựng ba lớp mà mọi bàn giao dịch chuyên nghiệp đều có:

| Lớp | Nhiệm vụ | Đặc điểm |
|---|---|---|
| Cổng rủi ro trước lệnh | Chặn lệnh xấu trước khi ra khỏi máy | Phải chạy trên **mọi** lệnh, không ngoại lệ |
| Tín hiệu | Biến sổ lệnh thành dự báo | Đơn giản và giải thích được, không phải hộp đen |
| Định cỡ vị thế | Quyết định đặt bao nhiêu | Sai chỗ này thì tín hiệu tốt vẫn phá sản |

Nguyên tắc xuyên suốt: **cổng rủi ro nằm trên đường nóng và không được phép bỏ qua**. Nó phải nhanh (chương này đo được vài chục nanosecond) để không ai có động cơ tắt nó đi.

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
│     là đủ để Kelly toàn phần dẫn tới phá sản.                              │
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

Trong khi xây chương này, một lỗi compute vi xuất hiện: kiểm tra **giá trị lệnh** đứng trước kiểm tra **vị thế**, nên nhiều bài kiểm thử không bao giờ chạm tới nhánh vị thế — chúng bị chặn sớm hơn.

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

Công thức Kelly `f* = (p·b − q)/b` tối đa hoá tốc độ tăng trưởng logarit dài hạn. Về mặt toán học nó tối ưu. Về mặt thực hành nó nguy hiểm, vì hai lý do:

- **Dao động khủng khiếp.** Với Kelly toàn phần, sụt giảm 50% từ đỉnh là chuyện bình thường, không phải bất thường.
- **Bạn không biết `p`.** Nếu ước lượng xác suất thắng cao hơn thật chỉ vài phần trăm, Kelly toàn phần trở thành cược vượt mức, và cược vượt mức dẫn tới tăng trưởng **âm** — dù mỗi cược đều có kỳ vọng dương.

Điểm thứ hai là điểm quan trọng: sai lầm theo hướng cược quá nhiều bị phạt **nặng hơn nhiều** so với cược quá ít. Vì thế thực tế dùng ¼ đến ½ Kelly, chấp nhận tăng trưởng chậm hơn để đổi lấy khả năng sống sót.

### 5. Sụt giảm quan trọng hơn lợi nhuận

Một chiến lược lãi 20%/năm với sụt giảm tối đa 5% thì đầu tư được. Cùng chiến lược đó với sụt giảm 40% thì không — không phải vì toán học, mà vì **không ai chịu được**: nhà đầu tư rút vốn, ban lãnh đạo cắt hạn mức, và người vận hành mất niềm tin đúng lúc đáy.

Tỉ số Sharpe đo lợi nhuận trên đơn vị biến động. Nhưng nó phạt biến động **tăng** giống hệt biến động **giảm** — điều mà không nhà đầu tư nào đồng ý. Đó là lý do phải nhìn cả sụt giảm tối đa, và vì sao chương này tính cả hai.

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
// tra cho nhanh". Lịch sử ngành đầy những vụ sập vì ai đó mở một đường vòng.

#[derive(Debug, Clone, PartialEq)]
pub enum RejectReason {
    SoLuongKhongDuong(Quantity),
    GiaKhongDuong(Price),
    /// Ngón tay béo: giá lệch quá xa giá thị trường — gần như chắc chắn gõ nhầm.
    NgonTayBeo { price: Price, tham_chieu: Price, lech_percent: f64 },
    VuotGiaTriLenh { value: i64, tran: i64 },
    VuotViThe { next_order: i64, tran: i64 },
    VuotLoTrongNgay { lo: i64, tran: i64 },
    VuotSoLenhMoiGiay { count: u32, tran: u32 },
    CongTacTatDaBat,
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
        if self.switch_all { return Err(RejectReason::CongTacTatDaBat); }
        if quantity <= 0 { return Err(RejectReason::SoLuongKhongDuong(quantity)); }
        if price <= 0 { return Err(RejectReason::GiaKhongDuong(price)); }

        // Ngón tay béo: gõ 8400 thành 84000 là chuyện xảy ra hằng năm
        if reference_price > 0 {
            let lech = (price - reference_price).abs() as f64 / reference_price as f64;
            if lech > self.limit.fat_finger_threshold {
                return Err(RejectReason::NgonTayBeo { price, tham_chieu: reference_price,
                                                lech_percent: lech * 100.0 });
            }
        }

        let value = price * quantity;
        if value > self.limit.max_order_value {
            return Err(RejectReason::VuotGiaTriLenh { value, tran: self.limit.max_order_value });
        }

        let next_order = self.position + side.first() * quantity;
        if next_order.abs() > self.limit.max_position {
            return Err(RejectReason::VuotViThe { next_order, tran: self.limit.max_position });
        }

        if self.realized_pnl < -self.limit.max_daily_loss {
            return Err(RejectReason::VuotLoTrongNgay { lo: -self.realized_pnl,
                                                 tran: self.limit.max_daily_loss });
        }

        // Cửa sổ trượt một giây
        while let Some(&t) = self.window_order.front() {
            if bay_gio_ns.saturating_sub(t) >= 1_000_000_000 { self.window_order.pop_front(); }
            else { break; }
        }
        let count = self.window_order.len() as u32;
        if count >= self.limit.so_lenh_moi_giay_toi_da {
            return Err(RejectReason::VuotSoLenhMoiGiay { count,
                                                   tran: self.limit.so_lenh_moi_giay_toi_da });
        }
        Ok(())
    }

    /// Ghi nhận một lần khớp — cập nhật vị thế, giá vốn và lãi/lỗ đã chốt.
    ///
    /// Điểm dễ sai nhất trong cả chương: lãi/lỗ KHÔNG phải dòng tiền của lệnh
    /// đóng. Bán 100 cổ giá 88,00 mang về tiền, nhưng nếu mua vào ở 90,00 thì
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
/// Dương = áp lực mua. Đây là tín hiệu đơn giản nhất mà vẫn có sức dự báo thật.
pub fn imbalance(qty_buy: u64, qty_sell: u64) -> f64 {
    let tong = qty_buy + qty_sell;
    if tong == 0 { return 0.0; }
    (qty_buy as f64 - qty_sell as f64) / tong as f64
}

/// Giá vi mô: giá giữa có gia quyền theo khối lượng ĐỐI ỨNG.
/// Nhiều người muốn mua → giá vi mô lệch về phía giá bán.
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
pub enum SignalCap { MoDaiA, MoDaiB, Dong, KhongLam }

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
        let day_truoc_do = self.window.day();
        let z = self.window.diem_z(cl);
        self.window.them(cl);

        let z = match z {
            Some(z) if day_truoc_do => z,
            _ => return SignalCap::KhongLam,
        };

        match self.is_open {
            None => {
                if z > self.threshold_in {
                    // A đắt bất thường so với B → bán A, mua B
                    self.is_open = Some(SignalCap::MoDaiB);
                    SignalCap::MoDaiB
                } else if z < -self.threshold_in {
                    self.is_open = Some(SignalCap::MoDaiA);
                    SignalCap::MoDaiA
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

/// Định cỡ theo mục tiêu biến động: mã càng dao động mạnh thì mua càng ít,
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
    /// Tỉ số lợi nhuận trên độ dao động — càng cao càng "êm".
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
    let thay_swap: Vec<f64> = equity_curve.windows(2).map(|w| (w[1] - w[0]) as f64).collect();
    let n = thay_swap.len() as f64;
    let tb = thay_swap.iter().sum::<f64>() / n;
    let ps = thay_swap.iter().map(|x| (x - tb).powi(2)).sum::<f64>() / (n - 1.0).max(1.0);
    let sd = ps.sqrt();
    RiskOwned {
        total_pnl: equity_curve[equity_curve.len() - 1] - equity_curve[0],
        max_drawdown: dd,
        ratio_drawdown: if peak.abs() > 0 { dd as f64 / peak.abs() as f64 } else { 0.0 },
        num_session_lai: thay_swap.iter().filter(|&&x| x > 0.0).count(),
        num_session_lo: thay_swap.iter().filter(|&&x| x < 0.0).count(),
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
        println!("   mua {:>4} / bán {:>4} → mất cân bằng {:>6.2} · giá vi mô {:>8.2}",
                 m, b, imbalance(m, b), price_pos_open(8_400, m, 8_410, b).unwrap());
    }
    println!("   → Nhiều người chờ mua thì giá vi mô lệch LÊN phía giá bán.");

    println!("\n5. ARBITRAGE CẶP");
    let (ga, gb) = gen_cap_price(3_000, 2024, 1.5);
    let mut arb = ArbCap::new(1.5, 100, 2.0, 0.5, 4.0);
    let (mut in_, mut ra) = (0, 0);
    for i in 0..ga.len() {
        match arb.update(ga[i], gb[i]) {
            SignalCap::MoDaiA | SignalCap::MoDaiB => in_ += 1,
            SignalCap::Dong => ra += 1,
            SignalCap::KhongLam => {}
        }
    }
    println!("   {} điểm dữ liệu → vào lệnh {} lần · thoát {} lần", ga.len(), in_, ra);
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
    fn order_hop_le_can_wait_qua() {
        let mut c = sample_gate();
        assert_eq!(c.check(Side::Buy, 8_400, 100, 8_400, 1_000_000_000), Ok(()));
        assert_eq!(c.order_book_qua, 1);
        assert_eq!(c.orders_blocked, 0);
    }

    #[test]
    fn chan_ngon_tay_beo() {
        // Gõ 8400 thành 84000 — lỗi có thật, xảy ra hằng năm ở mọi thị trường.
        let mut c = sample_gate();
        let e = c.check(Side::Buy, 84_000, 1, 8_400, 1_000_000_000).unwrap_err();
        assert!(matches!(e, RejectReason::NgonTayBeo { .. }));
        // Lệch nhỏ trong ngưỡng thì vẫn cho qua
        assert!(c.check(Side::Buy, 8_800, 1, 8_400, 1_000_000_000).is_ok());
    }

    #[test]
    fn khong_co_gia_tham_chieu_thi_bo_qua_kiem_tra_ngon_tay_beo() {
        // Mã mới niêm yết chưa có giá tham chiếu — không được chặn oan.
        let mut c = sample_gate();
        assert!(c.check(Side::Buy, 9_000, 1, 0, 1_000_000_000).is_ok());
    }

    #[test]
    fn block_quantity_and_price_no_hop_le() {
        let mut c = sample_gate();
        assert_eq!(c.check(Side::Buy, 8_400, 0, 8_400, 1).unwrap_err(),
                   RejectReason::SoLuongKhongDuong(0));
        assert_eq!(c.check(Side::Buy, 8_400, -5, 8_400, 1).unwrap_err(),
                   RejectReason::SoLuongKhongDuong(-5));
        assert_eq!(c.check(Side::Buy, 0, 10, 0, 1).unwrap_err(),
                   RejectReason::GiaKhongDuong(0));
    }

    #[test]
    fn block_value_order_qua_lon() {
        let mut c = sample_gate();
        assert!(matches!(c.check(Side::Buy, 8_400, 100_000, 8_400, 1).unwrap_err(),
                         RejectReason::VuotGiaTriLenh { .. }));
    }

    #[test]
    fn block_exceed_cap_position_all_two_side() {
        let mut c = sample_gate();
        assert!(matches!(c.check(Side::Buy, 8_400, 501, 8_400, 1).unwrap_err(),
                         RejectReason::VuotViThe { next_order: 501, tran: 500 }));
        assert!(matches!(c.check(Side::Sell, 8_400, 501, 8_400, 1).unwrap_err(),
                         RejectReason::VuotViThe { next_order: -501, tran: 500 }),
                "bán khống cũng phải bị chặn, không chỉ mua");
    }

    #[test]
    fn position_current_can_tinh_in_limit() {
        let mut c = sample_gate();
        c.record_recv_fill(Side::Buy, 8_400, 400);
        assert!(c.check(Side::Buy, 8_400, 100, 8_400, 1).is_ok(), "400+100 = 500, vừa trần");
        assert!(c.check(Side::Buy, 8_400, 101, 8_400, 1).is_err(), "400+101 vượt trần");
        assert!(c.check(Side::Sell, 8_400, 400, 8_400, 1).is_ok(), "bán thì giảm vị thế");
    }

    #[test]
    fn limit_rate_block_use_order_book() {
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
    fn window_rate_truot_theo_time_time() {
        let mut c = sample_gate();
        for i in 0..5u64 {
            assert!(c.check(Side::Buy, 8_400, 1, 8_400, 1_000_000_000 + i).is_ok());
        }
        assert!(c.check(Side::Buy, 8_400, 1, 8_400, 1_000_000_100).is_err(), "đã đủ 5");
        // Sang giây sau thì cửa sổ trượt qua, lại cho phép
        assert!(c.check(Side::Buy, 8_400, 1, 8_400, 2_500_000_000).is_ok());
    }

    #[test]
    fn cong_tac_tat_chan_moi_thu_va_khong_tu_go_duoc() {
        let mut c = sample_gate();
        c.enable_all_switches();
        // Kể cả lệnh hoàn toàn hợp lệ cũng không lọt
        assert_eq!(c.check(Side::Buy, 8_400, 1, 8_400, 1).unwrap_err(),
                   RejectReason::CongTacTatDaBat);
        assert!(c.da_tat(), "công tắc KHÔNG được tự tắt sau khi chặn");
        c.operator_flips_switch();
        assert!(c.check(Side::Buy, 8_400, 1, 8_400, 1).is_ok());
    }

    #[test]
    fn lo_slow_cap_thi_from_enable_cong_tac_all() {
        let mut c = RiskGate::new(LimitRisk { max_daily_loss: 10_000,
                                                 ..Default::default() });
        assert!(!c.da_tat());
        c.record_recv_fill(Side::Buy, 9_000, 100);
        c.record_recv_fill(Side::Sell, 8_800, 100); // lỗ 20 000 > trần 10 000
        assert_eq!(c.realized_pnl, -20_000);
        assert!(c.da_tat(), "vượt trần lỗ phải tự dừng, không chờ người can thiệp");
    }

    #[test]
    fn close_position_has_lai_thi_no_enable_cong_tac() {
        let mut c = RiskGate::new(LimitRisk { max_daily_loss: 10_000,
                                                 ..Default::default() });
        c.record_recv_fill(Side::Buy, 8_000, 100);
        c.record_recv_fill(Side::Sell, 8_500, 100);
        assert_eq!(c.realized_pnl, 50_000, "mua 80.00 bán 85.00 → lãi");
        assert!(!c.da_tat());
        assert_eq!(c.position, 0);
    }

    #[test]
    fn cost_basis_can_binh_quan_when_open_add() {
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
    fn reverse_side_position_set_lai_cost_basis() {
        let mut c = sample_gate();
        c.record_recv_fill(Side::Buy, 8_000, 100);
        // Bán 300: đóng 100 (lãi) rồi mở mới 200 ở chiều bán
        c.record_recv_fill(Side::Sell, 8_500, 300);
        assert_eq!(c.position, -200);
        assert_eq!(c.realized_pnl, 50_000, "chỉ phần ĐÓNG mới tính lãi");
        assert!((c.cost_basis - 8_500.0).abs() < 1e-9, "phần dư là vị thế mới ở giá 8500");
    }

    #[test]
    fn ban_khong_roi_mua_lai_re_hon_thi_co_lai() {
        let mut c = sample_gate();
        c.record_recv_fill(Side::Sell, 9_000, 100);
        assert_eq!(c.position, -100);
        c.record_recv_fill(Side::Buy, 8_500, 100);
        assert_eq!(c.realized_pnl, 50_000, "bán khống 90.00 mua lại 85.00 → lãi");
    }

    #[test]
    fn open_add_same_side_thi_chua_show_thuc_hoa_pnl() {
        let mut c = sample_gate();
        c.record_recv_fill(Side::Buy, 8_000, 100);
        c.record_recv_fill(Side::Buy, 9_000, 100);
        assert_eq!(c.position, 200);
        assert_eq!(c.realized_pnl, 0, "chưa đóng gì thì chưa chốt lãi/lỗ");
    }

    #[test]
    fn count_use_order_book_qua_and_is_block() {
        let mut c = sample_gate();
        c.check(Side::Buy, 8_400, 100, 8_400, 1).ok();
        c.check(Side::Buy, 84_000, 100, 8_400, 1).ok();
        c.check(Side::Buy, 8_400, -1, 8_400, 1).ok();
        assert_eq!(c.order_book_qua, 1);
        assert_eq!(c.orders_blocked, 2);
    }

    // ---------- Tín hiệu ----------
    #[test]
    fn mat_can_bang_nam_trong_khoang_am_mot_den_mot() {
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
    fn gia_vi_mo_lech_ve_phia_ben_it_khoi_luong() {
        // Nhiều người chờ MUA → áp lực đẩy giá lên → giá vi mô gần giá BÁN.
        let many_buy = price_pos_open(8_400, 9_000, 8_410, 1_000).unwrap();
        let many_sell = price_pos_open(8_400, 1_000, 8_410, 9_000).unwrap();
        let can_bang = price_pos_open(8_400, 1_000, 8_410, 1_000).unwrap();
        assert!(many_buy > can_bang, "áp lực mua đẩy giá vi mô lên");
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
    fn window_tinh_use_mean_and_do_lech() {
        let mut c = StatsWindow::new(5);
        for x in [2.0, 4.0, 4.0, 4.0, 5.0] { c.them(x); }
        assert!((c.mean() - 3.8).abs() < 1e-9);
        // phương sai mẫu của [2,4,4,4,5] = 1.2
        assert!((c.variance() - 1.2).abs() < 1e-9);
        assert!(c.day());
    }

    #[test]
    fn old_window_truot_unit_value() {
        let mut c = StatsWindow::new(3);
        for x in [1.0, 2.0, 3.0, 4.0, 5.0] { c.them(x); }
        assert_eq!(c.quantity(), 3);
        assert!((c.mean() - 4.0).abs() < 1e-9, "chỉ còn [3,4,5]");
    }

    #[test]
    fn phuong_sai_khong_bao_gio_am_du_sai_so_dau_phay_dong() {
        let mut c = StatsWindow::new(50);
        for _ in 0..50 { c.them(1_000_000.0); } // toàn giá trị giống hệt, cỡ lớn
        assert!(c.variance() >= 0.0, "phải chặn sai số làm ra số âm");
        assert!(c.variance() < 1e-3, "dữ liệu không đổi thì phương sai ~0");
        assert_eq!(c.diem_z(1_000_000.0), None, "độ lệch ~0 thì điểm z vô nghĩa");
    }

    #[test]
    fn window_chua_data_two_point_thi_variance_table_no() {
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
    fn arb_khong_ra_tin_hieu_khi_chua_du_du_lieu() {
        let mut a = ArbCap::new(1.5, 100, 2.0, 0.5, 4.0);
        let (ga, gb) = gen_cap_price(50, 1, 1.5);
        for i in 0..50 {
            assert_eq!(a.update(ga[i], gb[i]), SignalCap::KhongLam,
                       "cửa sổ chưa đầy thì tuyệt đối không được vào lệnh");
        }
    }

    #[test]
    fn arb_vao_lenh_khi_chenh_lech_gian_bat_thuong() {
        let mut a = ArbCap::new(1.0, 20, 2.0, 0.5, 10.0);
        // 20 điểm ổn định quanh 0 (có dao động nhỏ để độ lệch chuẩn khác 0)
        for i in 0..20 { a.update(10_000 + (i % 3), 10_000); }
        // rồi một cú giãn mạnh
        let th = a.update(10_100, 10_000);
        assert_eq!(th, SignalCap::MoDaiB, "A đắt bất thường → bán A mua B");
        assert_eq!(a.is_open, Some(SignalCap::MoDaiB));
    }

    #[test]
    fn arb_khong_mo_hai_vi_the_cung_luc() {
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
    fn arb_cat_lo_khi_chenh_lech_gian_qua_nguong_dung() {
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
    fn spread_tinh_use_theo_ratio_phong_proxy() {
        let a = ArbCap::new(1.5, 10, 2.0, 0.5, 4.0);
        assert!((a.spread(15_000, 10_000) - 0.0).abs() < 1e-9);
        assert!((a.spread(15_150, 10_000) - 150.0).abs() < 1e-9);
    }

    // ---------- Định cỡ ----------
    #[test]
    fn kelly_bang_khong_khi_khong_co_loi_the() {
        assert_eq!(kelly_fraction(0.5, 1.0), 0.0, "tung đồng xu công bằng → đừng đánh");
        assert_eq!(kelly_fraction(0.4, 1.0), 0.0, "lợi thế âm → tuyệt đối đừng đánh");
        assert_eq!(kelly_fraction(0.3, 0.5), 0.0);
    }

    #[test]
    fn kelly_tang_theo_loi_the() {
        let mut prev = 0.0;
        for p in [0.55, 0.60, 0.65, 0.70, 0.80] {
            let f = kelly_fraction(p, 1.0);
            assert!(f > prev, "lợi thế lớn hơn phải cho cỡ lớn hơn");
            assert!(f <= 1.0);
            prev = f;
        }
    }

    #[test]
    fn kelly_dung_gia_tri_kinh_dien() {
        // 60% thắng, ăn 1 thua 1 → Kelly = 2p − 1 = 0.20
        assert!((kelly_fraction(0.60, 1.0) - 0.20).abs() < 1e-9);
        // 40% thắng, ăn 2 thua 1 → (0.4·2 − 0.6)/2 = 0.10
        assert!((kelly_fraction(0.40, 2.0) - 0.10).abs() < 1e-9);
    }

    #[test]
    fn kelly_mot_phan_luon_nho_hon_kelly_toan_phan() {
        for p in [0.55, 0.60, 0.75] {
            let toan = kelly_fraction(p, 1.0);
            let part = fractional_kelly(p, 1.0, 0.25);
            assert!(part < toan);
            assert!((part - toan * 0.25).abs() < 1e-9);
        }
    }

    #[test]
    fn kelly_khong_chia_cho_khong() {
        assert_eq!(kelly_fraction(0.9, 0.0), 0.0);
        assert_eq!(kelly_fraction(0.9, -1.0), 0.0);
    }

    #[test]
    fn has_theo_volatility_down_when_volatility_up() {
        let von = 1_000_000i64;
        let a = has_theo_volatility(von, 0.10, 0.10, 100);
        let b = has_theo_volatility(von, 0.10, 0.40, 100);
        assert!(b < a, "mã dao động mạnh gấp 4 thì mua ít hơn hẳn");
        assert_eq!(a, 10_000, "biến động khớp mục tiêu → dùng toàn bộ vốn");
        assert_eq!(b, 2_500, "gấp 4 lần biến động → 1/4 tỉ trọng");
    }

    #[test]
    fn co_theo_bien_dong_khong_bao_gio_don_bay_qua_von() {
        // Mã êm hơn mục tiêu KHÔNG được dẫn tới mua vượt vốn.
        let c = has_theo_volatility(1_000_000, 0.40, 0.05, 100);
        assert_eq!(c, 10_000, "tỉ trọng bị chặn ở 1.0, không dùng đòn bẩy ngầm");
    }

    #[test]
    fn co_theo_bien_dong_an_toan_voi_dau_vao_xau() {
        assert_eq!(has_theo_volatility(1_000_000, 0.1, 0.0, 100), 0);
        assert_eq!(has_theo_volatility(1_000_000, 0.1, 0.1, 0), 0);
        assert_eq!(has_theo_volatility(1_000_000, 0.1, -0.5, 100), 0);
    }

    // ---------- Thước đo rủi ro ----------
    #[test]
    fn duong_von_di_len_deu_thi_khong_sut_giam() {
        let d: Vec<i64> = (0..50).map(|i| 100_000 + i * 100).collect();
        let r = risk_level(&d);
        assert_eq!(r.max_drawdown, 0);
        assert_eq!(r.num_session_lo, 0);
        assert_eq!(r.total_pnl, 4_900);
    }

    #[test]
    fn drawdown_do_use_distance_from_peak() {
        let d = vec![100, 150, 120, 80, 130];
        let r = risk_level(&d);
        assert_eq!(r.max_drawdown, 70, "từ đỉnh 150 xuống đáy 80");
    }

    #[test]
    fn sut_giam_khong_bao_gio_am() {
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
    fn duong_von_em_co_sharpe_cao_hon_duong_xoc() {
        // Cùng đích đến, nhưng đường êm mới là đường người ta đi hết được.
        // Đường "êm" vẫn phải có dao động nhỏ: đường thẳng tuyệt đối cho độ
        // lệch chuẩn 0, và khi đó Sharpe không định nghĩa được (ta trả 0).
        let em: Vec<i64> = (0..100).map(|i| 100_000 + i * 500 + (i % 5) * 40).collect();
        let mut xoc = Vec::new();
        let mut v = 100_000i64;
        for i in 0..100 { v += if i % 3 == 0 { -8_000 } else { 5_750 }; xoc.push(v); }
        let (a, b) = (risk_level(&em), risk_level(&xoc));
        assert!(a.sharpe_ratio > b.sharpe_ratio,
                "êm {:.2} phải cao hơn xóc {:.2}", a.sharpe_ratio, b.sharpe_ratio);
        assert!(b.max_drawdown > a.max_drawdown);
    }

    #[test]
    fn duong_von_qua_ngan_khong_panic() {
        assert_eq!(risk_level(&[]).total_pnl, 0);
        assert_eq!(risk_level(&[100]).max_drawdown, 0);
        assert_eq!(risk_level(&[100, 100]).sharpe_ratio, 0.0, "không dao động → Sharpe 0");
    }

    // ---------- Sinh dữ liệu ----------
    #[test]
    fn gen_cap_price_all_peak() {
        assert_eq!(gen_cap_price(100, 5, 1.5), gen_cap_price(100, 5, 1.5));
        assert_ne!(gen_cap_price(100, 5, 1.5), gen_cap_price(100, 6, 1.5));
    }

    #[test]
    fn two_series_price_true_su_di_same_each() {
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
        assert!(correlation > 0.8, "tương quan {:.3} phải cao", correlation);
    }
}
```

---

## Bảng tra cứu lỗi biên dịch thường gặp

| Lỗi | Nguyên nhân trong chương này | Cách sửa |
|---|---|---|
| `attempt to subtract with overflow` | `position - quantity` với `u64` | Vị thế phải là `i64` — nó có thể âm |
| Sharpe bằng 0 với chiến lược "hoàn hảo" | Đường vốn đều tuyệt đối → độ lệch chuẩn 0 | Thêm nhiễu nhỏ; đường vốn hoàn hảo không tồn tại |
| Bài kiểm thử không chạm tới nhánh vị thế | Kiểm giá trị lệnh chặn trước | Nới hạn mức giá trị trong dữ liệu kiểm thử |
| Lãi lỗ sai sau khi đảo chiều | Không xử lý riêng trường hợp vượt qua 0 | Chốt phần cũ, đặt giá vốn mới cho phần dư |
| `E0308: expected f64, found i64` | Trộn tiền nguyên với thống kê thực | Ép kiểu tường minh ở đúng biên |

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
pub struct RuiRoThichUng {
    pub han_muc_co_ban: i64,
    pub he_so: f64,           // 0,25 → 1,0
    pub chuoi_thang: u32,
    pub chuoi_thua: u32,
}

impl RuiRoThichUng {
    pub fn record(&mut self, pnl: i64) {
        if pnl > 0 {
            self.chuoi_thang += 1;
            self.chuoi_thua = 0;
            // Nới CHẬM: cần 5 lần thắng liên tiếp mới tăng 10%
            if self.chuoi_thang >= 5 {
                self.he_so = (self.he_so * 1.1).min(1.0);
                self.chuoi_thang = 0;
            }
        } else if pnl < 0 {
            self.chuoi_thua += 1;
            self.chuoi_thang = 0;
            // Thu NHANH: 3 lần thua liên tiếp là cắt 30%
            if self.chuoi_thua >= 3 {
                self.he_so = (self.he_so * 0.7).max(0.25);
                self.chuoi_thua = 0;
            }
        }
    }

    pub fn han_muc_hien_tai(&self) -> i64 {
        (self.han_muc_co_ban as f64 * self.he_so) as i64
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
pub struct ToHopTinHieu {
    pub name: Vec<String>,
    pub weight: Vec<f64>,
    pub history: Vec<Vec<f64>>,   // lịch sử giá trị từng tín hiệu
}

impl ToHopTinHieu {
    pub fn ket_hop(&mut self, value: &[f64]) -> f64 {
        for (i, v) in value.iter().enumerate() {
            if i < self.history.len() { self.history[i].push(*v); }
        }
        let tong_ts: f64 = self.weight.iter().map(|w| w.abs()).sum();
        if tong_ts == 0.0 { return 0.0; }
        value.iter().zip(&self.weight).map(|(v, w)| v * w).sum::<f64>() / tong_ts
    }

    pub fn correlation(&self, i: usize, j: usize) -> Option<f64> {
        let (a, b) = (self.history.get(i)?, self.history.get(j)?);
        let n = a.len().min(b.len());
        if n < 2 { return None; }
        let (id, mb) = (a[..n].iter().sum::<f64>() / n as f64,
                        b[..n].iter().sum::<f64>() / n as f64);
        let (mut cov, mut va, mut vb) = (0.0, 0.0, 0.0);
        for k in 0..n {
            let (da, db) = (a[k] - id, b[k] - mb);
            cov += da * db; va += da * da; vb += db * db;
        }
        if va == 0.0 || vb == 0.0 { return None; }
        Some(cov / (va * vb).sqrt())
    }

    /// Các cặp tín hiệu quá giống nhau — chúng KHÔNG đa dạng hoá gì cả.
    pub fn cap_du_thua(&self, threshold: f64) -> Vec<(usize, usize, f64)> {
        let mut ra = Vec::new();
        for i in 0..self.history.len() {
            for j in (i + 1)..self.history.len() {
                if let Some(r) = self.correlation(i, j) {
                    if r.abs() > threshold { ra.push((i, j, r)); }
                }
            }
        }
        ra
    }
}
```

`cap_du_thua` là công cụ chẩn đoán quan trọng: nếu hai tín hiệu có tương quan trên 0,8, bạn nên bỏ một hoặc gộp chúng — nếu không, "danh mục tín hiệu" của bạn thực chất chỉ có một ý tưởng được đặt cược nhiều lần.
</details>
