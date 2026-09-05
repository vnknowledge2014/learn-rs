# Chương 69: Hệ thống giao dịch thuật toán — Sổ Lệnh, Khớp Lệnh & Kiểm Định Chiến Lược (Algorithmic Trading Systems)

## Giới thiệu & Mục tiêu học tập

> ⚠️ **Đây là tài liệu kỹ thuật, không phải lời khuyên đầu tư.** Mọi số liệu trong chương là dữ liệu giả lập tất định, sinh bằng bộ số giả ngẫu nhiên có hạt giống cố định. Mục đích duy nhất là dạy kiến trúc phần mềm.

Các nền tảng giao dịch mã nguồn mở như [OpenAlgo](https://github.com/marketcalls/openalgo) là những **sản phẩm** hoàn chỉnh: hàng chục trình kết nối môi giới, giao diện web, trình dựng chiến lược không cần lập trình. Phần lớn khối lượng công việc của chúng là **tích hợp** — quan trọng, nhưng không dạy được nhiều.

Chương này đi vào phần còn lại, phần **kỹ thuật** — và đó cũng chính là phần mà Rust thắng thuyết phục:

| | Ngôn ngữ có bộ dọn rác | Rust |
|---|---|---|
| Độ trễ **trung bình** | tốt | tốt |
| Độ trễ **phân vị 99,9** | tệ (dọn rác chen ngang) | tốt |
| Có **trần** độ trễ không? | không | **có** |

Trong giao dịch, phân vị 99,9 mới là con số quan trọng. Một cú dừng dọn rác 50 ms xảy ra đúng lúc thị trường biến động là lúc bạn mất tiền — và nó luôn xảy ra đúng lúc đó, vì thị trường biến động chính là lúc hệ thống cấp phát nhiều bộ nhớ nhất.

Chương này cũng là **bài tổng hợp** của nhiều chương trước: typestate của Chương 20 dùng cho vòng đời lệnh, vị nhóm của Chương 18 dùng cho gộp lãi/lỗ, `BTreeMap` của Chương 29 làm sổ lệnh, và nguyên tắc hàm thuần túy của Chương 13 làm nền cho bộ kiểm định.

Mục tiêu học tập:
- Hiểu vì sao **tiền phải là số nguyên**, và tự tay thấy `f64` sai ở đâu.
- Xây **sổ lệnh giới hạn** với ưu tiên **giá–thời gian**, và hiểu quy tắc cải thiện giá.
- Cài **động cơ khớp lệnh** và kiểm chứng bất biến sống còn: khối lượng được bảo toàn.
- Dùng **typestate** để lệnh chưa qua kiểm tra rủi ro **không thể** gửi đi được.
- Nhận ra **vị thế là một vị nhóm**, và vì sao điều đó cho phép gộp song song an toàn.
- Viết **bộ kiểm định chiến lược** không nhìn trộm tương lai, có mô hình phí và trượt giá.

---

## Hình tượng hóa đời sống (Intuitive Everyday Analogy)

```
┌──────────────────────────────────────────────────────────────────────────────┐
│   HÌNH TƯỢNG: SỔ LỆNH = BẢNG RAO VẶT Ở CHỢ ĐẦU MỐI                          │
├──────────────────────────────────────────────────────────────────────────────┤
│                                                                              │
│   NGƯỜI MUỐN MUA (ghi giá mình SẴN SÀNG TRẢ — càng cao càng dễ mua)         │
│   ┌────────────────────────────────────────────────────────┐                │
│   │  84.00  ×100  (bác An, dán lúc 9:00)  ← TỐT NHẤT      │                │
│   │  84.00  ×200  (chị Bình, dán lúc 9:05)                 │                │
│   │  83.90  ×500  (anh Cường)                              │                │
│   └────────────────────────────────────────────────────────┘                │
│              ↕  CHÊNH LỆCH = 20 xu — chi phí ẩn của mọi giao dịch           │
│   ┌────────────────────────────────────────────────────────┐                │
│   │  84.20  ×150  (cô Dung)               ← TỐT NHẤT      │                │
│   │  84.30  ×300  (chú Em)                                 │                │
│   └────────────────────────────────────────────────────────┘                │
│   NGƯỜI MUỐN BÁN (ghi giá mình CHỊU BÁN — càng thấp càng dễ bán)            │
│                                                                              │
│   HAI QUY TẮC, THEO ĐÚNG THỨ TỰ NÀY:                                        │
│     1️⃣ ƯU TIÊN GIÁ    — ai trả cao hơn được mua trước. Luôn luôn.           │
│     2️⃣ ƯU TIÊN THỜI GIAN — CÙNG giá thì ai dán trước được trước.            │
│                                                                              │
│   → Bác An (9:00) khớp hết TRƯỚC chị Bình (9:05), dù cùng giá 84.00.        │
│                                                                              │
├──────────────────────────────────────────────────────────────────────────────┤
│   CẢI THIỆN GIÁ = NGƯỜI ĐẾN SAU KHÔNG BAO GIỜ BỊ THIỆT                      │
│                                                                              │
│   Cô Dung dán bán 84.20. Bạn tới, sẵn sàng trả tới 85.00.                   │
│   → Bạn trả 84.20, KHÔNG PHẢI 85.00.                                        │
│   Giá khớp luôn là giá của tờ ĐÃ DÁN SẴN, không phải giá bạn chào.          │
│                                                                              │
├──────────────────────────────────────────────────────────────────────────────┤
│   VÌ SAO TIỀN KHÔNG ĐƯỢC LÀ SỐ THỰC                                          │
│                                                                              │
│   Cộng 0.1 mười lần bằng f64 → 0.99999999999999988898                       │
│   Bằng 1.0? → KHÔNG.                                                        │
│                                                                              │
│   Sai một phần tỉ, nhân với 10 triệu lệnh mỗi ngày = một vụ kiện.           │
│   Ngành tài chính đếm bằng ĐƠN VỊ NHỎ NHẤT: xu, tick, satoshi.              │
│   Số nguyên. Không bao giờ có sai số làm tròn. Không bao giờ tranh cãi.     │
└──────────────────────────────────────────────────────────────────────────────┘
```

---

## Khái niệm & Cơ chế kỹ thuật chuyên sâu (In-Depth Technical Mechanics)

### 1. Vì sao `BTreeMap` là cấu trúc đúng cho sổ lệnh

Sổ lệnh cần bốn thao tác, và `BTreeMap` làm tốt cả bốn:

| Thao tác | Tần suất | `BTreeMap` | `HashMap` | `Vec` đã sắp |
|---|---|---|---|---|
| Lấy giá tốt nhất | **rất cao** | O(log n) | O(n) ❌ | O(1) |
| Chèn mức giá mới | cao | O(log n) | O(1) | O(n) ❌ |
| Xóa mức giá rỗng | cao | O(log n) | O(1) | O(n) ❌ |
| Duyệt theo thứ tự giá | cao | O(1)/bước | không được ❌ | O(1)/bước |

`HashMap` thua vì nó **không có thứ tự** — mà thứ tự giá chính là bản chất của sổ lệnh.

Một mẹo nhỏ nhưng quan trọng: bên mua lưu khóa là **giá âm**. `BTreeMap` sắp tăng dần, nên `-8400 < -8390` khiến giá **cao nhất** (8400) nằm đầu — đúng thứ mà `keys().next()` cần trả về.

Ở mỗi mức giá, `VecDeque` giữ ưu tiên thời gian: `push_back` khi vào, `pop_front` khi khớp. Đó chính xác là ngữ nghĩa hàng đợi vào-trước-ra-trước mà quy tắc sàn đòi hỏi.

### 2. Quy tắc cải thiện giá và vì sao nó công bằng

Khi lệnh mua giá 120 gặp lệnh bán đã nằm sẵn giá 100, giá khớp là **100**, không phải 120. Người đến sau được hưởng giá tốt hơn mình đã chào.

Lý do không phải lòng tốt mà là **khuyến khích đúng đắn**: nếu người đến sau phải trả đúng giá mình chào, sẽ không ai dám chào giá cao — ai cũng chào sát giá thị trường, và thanh khoản biến mất. Quy tắc cải thiện giá cho phép bạn nói "tôi trả tới mức này" mà không sợ bị lợi dụng.

### 3. Typestate cho vòng đời lệnh

Hai lỗi đắt tiền nhất trong hệ thống giao dịch:
- Gửi một lệnh **hai lần** (mua gấp đôi số định mua).
- Gửi lệnh **chưa qua** kiểm tra rủi ro.

Typestate loại bỏ cả hai ở tầng biên dịch:

```rust
Order<DangSoan>  ──check()──►  Order<RiskChecked>  ──send()──►  Order<Sent>
       ▲                                  ▲                            ▲
   vừa tạo ra                      đã qua hạn mức              đã vào sổ lệnh
```

`gui()` chỉ tồn tại trên `Lenh<RiskChecked>` và nhận `self` theo **giá trị**. Nghĩa là: không kiểm tra rủi ro thì không gọi được `gui()`, và gọi rồi thì lệnh cũ bị tiêu thụ, không gửi lại được. Cả hai lỗi trở thành lỗi biên dịch, với chi phí lúc chạy bằng không.

### 4. Vị thế là một vị nhóm — và vì sao điều đó quan trọng

`Position { quantity, tien_mat }` với phép `ghep` cộng từng trường thỏa mãn:
- **Kết hợp**: `(a·b)·c = a·(b·c)`
- **Đơn vị**: `Position::RONG` là phần tử trung hòa hai phía

Đây đúng định nghĩa **vị nhóm** ở Chương 18. Hệ quả thực tế không hề trừu tượng:

- Gộp lãi/lỗ của 10 triệu giao dịch có thể chia cho 16 lõi CPU, mỗi lõi gộp một phần, rồi gộp 16 kết quả lại. Luật kết hợp **bảo đảm** kết quả y hệt tính tuần tự — đây là chứng minh toán học, không phải hy vọng.
- Có thể dựng **cây tổng hợp** để tính lãi/lỗ theo tài khoản, theo bàn giao dịch, theo toàn công ty, mà không cần viết lại logic ở mỗi cấp.

Chương này có bài kiểm thử chia 100 giao dịch thành từng khối 7 rồi gộp lại, so với tính tuần tự — kết quả trùng khớp tuyệt đối.

### 5. Ba cách tự lừa mình khi kiểm định chiến lược

Bộ kiểm định là nơi dễ tự dối nhất trong cả ngành. Ba lỗi phổ biến, chương này chặn cả ba:

**a) Nhìn trộm tương lai (look-ahead bias).** Ra quyết định dựa trên nến hôm nay rồi khớp ở giá **đóng cửa** của chính nến đó — nhưng lúc quyết định, giá đóng cửa chưa tồn tại. Chương này khớp ở giá **mở cửa của nến kế tiếp**, và có bài kiểm thử chứng minh nến cuối cùng **không thể** sinh giao dịch nào.

**b) Bỏ qua chi phí.** Không có phí, không có trượt giá, chiến lược nào cũng đẹp. Thực tế: bạn luôn mua đắt hơn và bán rẻ hơn giá lý thuyết. Chương này có bài kiểm thử khẳng định thêm chi phí **luôn** làm kết quả xấu đi — và demo cho thấy cùng một chiến lược đi từ lỗ 22 000 tick xuống lỗ 45 000 tick chỉ vì thêm 2 tick trượt giá và 3 tick phí.

**c) Chỉ nhìn lợi nhuận.** Con số quan trọng hơn là **sụt giảm tối đa** — mức lỗ sâu nhất tính từ đỉnh. Một chiến lược lãi 50% mỗi năm nhưng có lúc sụt 60% là chiến lược mà **không ai** đủ can đảm đi tới cuối.

Một quan sát trung thực từ chính demo của chương: chiến lược giao cắt trung bình động chạy trên dữ liệu **bước ngẫu nhiên** cho kết quả **lỗ**, và lỗ nặng hơn khi tính đủ chi phí. Điều đó đúng như lý thuyết dự đoán — trên dữ liệu không có xu hướng thật, chiến lược theo xu hướng chỉ tạo ra chi phí giao dịch. Bộ kiểm định làm đúng việc của nó: nói cho bạn sự thật khó nghe.

### 6. Có nên xây một OpenAlgo cho Rust không?

Câu trả lời thẳng thắn, để bạn tự cân nhắc:

| Phần của OpenAlgo | Có đáng viết lại bằng Rust? |
|---|---|
| 36 trình kết nối môi giới | ❌ Công việc tích hợp thuần túy. Python phù hợp hơn, và API môi giới đổi liên tục |
| Giao diện web, trình dựng chiến lược | ❌ Hệ sinh thái web front-end không phải thế mạnh so sánh của Rust |
| **Sổ lệnh, động cơ khớp lệnh** | ✅ Đúng thế mạnh: độ trễ có trần, không dọn rác |
| **Bộ kiểm định trên dữ liệu lớn** | ✅ Nhanh hơn Python hàng chục lần, lại song song hóa được nhờ vị nhóm |
| **Cổng rủi ro** | ✅ Typestate biến lỗi vận hành thành lỗi biên dịch |

Kết luận: đừng chép lại cả sản phẩm. Hãy viết **phần lõi hiệu năng** bằng Rust và để phần tích hợp cho ngôn ngữ hợp với nó hơn — đúng mô hình mà các quỹ định lượng thật đang dùng.

---

## Mã nguồn minh họa thực chiến (Idiomatic Runnable Rust Blueprint)

Chạy bằng `cargo run -p ch69`, kiểm thử bằng `cargo test -p ch69`.

```rust
#![allow(dead_code)]
//! Chương 69 — Hệ thống giao dịch thuật toán: sổ lệnh, động cơ khớp lệnh,
//! quản trị rủi ro bằng kiểu, và bộ kiểm định chiến lược trên dữ liệu quá khứ.
//!
//! Đây là phần LÕI của một nền tảng kiểu OpenAlgo — nhưng viết bằng Rust, nơi
//! không có bộ dọn rác nên độ trễ có TRẦN xác định, chứ không phải trung bình đẹp
//! kèm những cú khựng bất chợt.
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
pub type Price = i64;      // tính bằng tick
pub type Quantity = i64;
pub type OrderId = u64;

pub fn tick_to_string(t: Price) -> String {
    format!("{}.{:02}", t / 100, (t % 100).abs())
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
pub enum Side { Buy, Sell }

impl Side {
    pub fn inverse_lai(self) -> Side {
        match self { Side::Buy => Side::Sell, Side::Sell => Side::Buy }
    }
    /// Dấu của vị thế: mua làm vị thế tăng, bán làm giảm.
    pub fn first(self) -> i64 { match self { Side::Buy => 1, Side::Sell => -1 } }
}

// ============================================================================
// 2. VÒNG ĐỜI LỆNH BẰNG TYPESTATE — trạng thái nằm trong KIỂU
// ============================================================================
// Áp dụng Chương 20 vào nghiệp vụ thật: gửi hai lần cùng một lệnh, hoặc hủy
// một lệnh đã khớp hết, là những lỗi tốn tiền. Ở đây chúng KHÔNG BIÊN DỊCH ĐƯỢC.

// Ba nhãn trạng thái. Chúng là kiểu RỖNG — không chiếm một byte nào lúc chạy;
// toàn bộ tác dụng của chúng diễn ra trong trình biên dịch.
#[derive(Debug, Clone, Copy)] pub struct DangSoan;
#[derive(Debug, Clone, Copy)] pub struct RiskChecked;
#[derive(Debug, Clone, Copy)] pub struct Sent;

#[derive(Debug, Clone)]
pub struct Order<State> {
    pub id: OrderId,
    pub id_chain: String,
    pub side: Side,
    pub price: Price,
    pub quantity: Quantity,
    pub filled: Quantity,
    _tt: PhantomData<State>,
}

impl Order<DangSoan> {
    pub fn new(id: OrderId, id_chain: &str, side: Side, price: Price, quantity: Quantity) -> Self {
        Order { id, id_chain: id_chain.to_string(), side, price, quantity, filled: 0, _tt: PhantomData }
    }
}

impl<TT> Order<TT> {
    pub fn remaining(&self) -> Quantity { self.quantity - self.filled }
    fn transfer<Moi>(self) -> Order<Moi> {
        Order { id: self.id, id_chain: self.id_chain, side: self.side, price: self.price,
               quantity: self.quantity, filled: self.filled, _tt: PhantomData }
    }
}

// Chỉ lệnh ĐÃ QUA kiểm tra rủi ro mới gửi được vào sổ lệnh.
impl Order<RiskChecked> {
    pub fn send(self) -> Order<Sent> { self.transfer() }
}

// ============================================================================
// 3. KIỂM TRA RỦI RO — cổng bắt buộc trước khi lệnh ra thị trường
// ============================================================================

#[derive(Debug, Clone, PartialEq)]
pub enum ErrorRisk {
    SoLuongKhongDuong(Quantity),
    GiaKhongDuong(Price),
    VuotGiaTriToiDa { value: i64, tran: i64 },
    VuotViTheToiDa { next_order: i64, tran: i64 },
    MaChungKhoanLa(String),
}

pub struct Limit {
    pub max_order_value: i64,
    pub max_position: i64,
    pub list_wait_op: Vec<String>,
}

impl Limit {
    /// Trả `Result` chứ không panic: từ chối lệnh là chuyện BÌNH THƯỜNG,
    /// không phải lỗi lập trình. Đây là ranh giới "parse, đừng validate".
    pub fn check(&self, l: Order<DangSoan>, vi_the_hien_tai: i64)
        -> Result<Order<RiskChecked>, ErrorRisk>
    {
        if l.quantity <= 0 { return Err(ErrorRisk::SoLuongKhongDuong(l.quantity)); }
        if l.price <= 0 { return Err(ErrorRisk::GiaKhongDuong(l.price)); }
        if !self.list_wait_op.iter().any(|m| *m == l.id_chain) {
            return Err(ErrorRisk::MaChungKhoanLa(l.id_chain.clone()));
        }
        let value = l.price * l.quantity;
        if value > self.max_order_value {
            return Err(ErrorRisk::VuotGiaTriToiDa { value, tran: self.max_order_value });
        }
        let next_order = vi_the_hien_tai + l.side.first() * l.quantity;
        if next_order.abs() > self.max_position {
            return Err(ErrorRisk::VuotViTheToiDa { next_order, tran: self.max_position });
        }
        Ok(l.transfer())
    }
}

// ============================================================================
// 4. SỔ LỆNH & ĐỘNG CƠ KHỚP LỆNH
// ============================================================================

#[derive(Debug, Clone, PartialEq)]
pub struct Fill {
    pub order_aggressive: OrderId,
    pub order_passive: OrderId,
    pub price: Price,
    pub quantity: Quantity,
}

/// Sổ lệnh giới hạn. `BTreeMap` cho phép lấy giá tốt nhất trong O(log n) và
/// duyệt các mức giá theo THỨ TỰ — đúng thứ động cơ khớp lệnh cần.
/// `VecDeque` ở mỗi mức giá giữ ưu tiên THỜI GIAN: ai đặt trước khớp trước.
pub struct OrderBook {
    /// Bên mua: khóa là giá ÂM để `BTreeMap` (vốn tăng dần) trả giá CAO nhất trước.
    side_buy: BTreeMap<Price, VecDeque<Order<Sent>>>,
    ben_ban: BTreeMap<Price, VecDeque<Order<Sent>>>,
}

impl OrderBook {
    pub fn new() -> Self { OrderBook { side_buy: BTreeMap::new(), ben_ban: BTreeMap::new() } }

    /// Giá mua cao nhất — cái giá tốt nhất mà người bán có thể nhận ngay.
    pub fn best_bid(&self) -> Option<Price> {
        self.side_buy.keys().next().map(|k| -k)
    }
    /// Giá bán thấp nhất.
    pub fn best_ask(&self) -> Option<Price> {
        self.ben_ban.keys().next().copied()
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
        let ban = match side { Side::Buy => &self.side_buy, Side::Sell => &self.ben_ban };
        let key = match side { Side::Buy => -price, Side::Sell => price };
        ban.get(&key).map_or(0, |q| q.iter().map(|l| l.remaining()).sum())
    }
    pub fn total_order_book(&self) -> usize {
        self.side_buy.values().map(|q| q.len()).sum::<usize>()
            + self.ben_ban.values().map(|q| q.len()).sum::<usize>()
    }

    /// Nạp lệnh và khớp ngay phần khớp được; phần dư nằm lại sổ.
    /// Đây là trái tim của sàn: ƯU TIÊN GIÁ trước, rồi ƯU TIÊN THỜI GIAN.
    pub fn nap(&mut self, mut order: Order<Sent>) -> Vec<Fill> {
        let mut all_fill = Vec::new();
        let swap_resp_is_sell = order.side == Side::Buy;

        loop {
            if order.remaining() == 0 { break; }
            // Mức giá đối ứng tốt nhất còn khớp được với giá giới hạn của ta?
            let key_good = {
                let swap_resp = if swap_resp_is_sell { &self.ben_ban } else { &self.side_buy };
                match swap_resp.keys().next().copied() {
                    Some(k) => {
                        let price_true = if swap_resp_is_sell { k } else { -k };
                        let fill_can = if swap_resp_is_sell { price_true <= order.price }
                                        else { price_true >= order.price };
                        if fill_can { Some((k, price_true)) } else { None }
                    }
                    None => None,
                }
            };
            let (key, gia_khop) = match key_good { Some(x) => x, None => break };

            let swap_resp = if swap_resp_is_sell { &mut self.ben_ban } else { &mut self.side_buy };
            let queue = swap_resp.get_mut(&key).unwrap();
            while order.remaining() > 0 {
                let swap_tac = match queue.front_mut() { Some(d) => d, None => break };
                let amount = order.remaining().min(swap_tac.remaining());
                order.filled += amount;
                swap_tac.filled += amount;
                all_fill.push(Fill {
                    order_aggressive: order.id,
                    order_passive: swap_tac.id,
                    // Giá khớp là giá của lệnh ĐÃ NẰM SẴN trong sổ — người
                    // đến sau được hưởng giá tốt hơn nếu có. Đây là quy tắc
                    // "cải thiện giá" của mọi sàn nghiêm túc.
                    price: gia_khop,
                    quantity: amount,
                });
                if swap_tac.remaining() == 0 { queue.pop_front(); }
            }
            if queue.is_empty() { swap_resp.remove(&key); }
        }

        if order.remaining() > 0 {
            let key = if order.side == Side::Buy { -order.price } else { order.price };
            let ban = if order.side == Side::Buy { &mut self.side_buy } else { &mut self.ben_ban };
            ban.entry(key).or_default().push_back(order);
        }
        all_fill
    }

    pub fn cancel(&mut self, id: OrderId) -> bool {
        for ban in [&mut self.side_buy, &mut self.ben_ban] {
            let mut rong = None;
            for (key, queue) in ban.iter_mut() {
                if let Some(i) = queue.iter().position(|l| l.id == id) {
                    queue.remove(i);
                    if queue.is_empty() { rong = Some(*key); }
                    if let Some(k) = rong { ban.remove(&k); }
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
    pub tien_mat: i64,
}

impl Position {
    pub const RONG: Position = Position { quantity: 0, tien_mat: 0 };

    /// Phép `ghep` này KẾT HỢP và có ĐƠN VỊ `RONG` → đúng định nghĩa vị nhóm.
    /// Nhờ vậy có thể gộp lãi/lỗ song song bằng `rayon` mà kết quả không đổi.
    pub fn compose(self, k: Position) -> Position {
        Position { quantity: self.quantity + k.quantity, tien_mat: self.tien_mat + k.tien_mat }
    }
    pub fn from_fill(side: Side, price: Price, quantity: Quantity) -> Position {
        Position {
            quantity: side.first() * quantity,
            tien_mat: -side.first() * price * quantity,
        }
    }
    /// Giá trị ròng khi định giá lại theo giá thị trường hiện tại.
    pub fn value_empty(&self, gia_thi_truong: Price) -> i64 {
        self.tien_mat + self.quantity * gia_thi_truong
    }
}

// ============================================================================
// 6. BỘ KIỂM ĐỊNH CHIẾN LƯỢC (backtest) — hàm thuần túy trên lịch sử
// ============================================================================

#[derive(Debug, Clone, Copy, PartialEq)]
pub struct Candle { pub timestamp: u64, pub mo: Price, pub high: Price, pub low: Price, pub dong: Price }

#[derive(Debug, Clone, Copy, PartialEq)]
pub enum Signal { Buy(Quantity), Sell(Quantity), Giu }

/// Chiến lược là một HÀM THUẦN TÚY: cùng lịch sử → cùng tín hiệu, luôn luôn.
/// Nhờ tính chất này mà kết quả kiểm định tái lập được 100%.
pub trait Strategy {
    fn name(&self) -> &str;
    fn decide(&mut self, history: &[Candle], position: &Position) -> Signal;
}

/// Giao cắt trung bình động: kinh điển, dễ hiểu, và cố tình đơn giản.
pub struct MeanCross { pub nhanh: usize, pub cham: usize, pub don_pos: Quantity }

fn mean(candle: &[Candle], n: usize) -> Option<Price> {
    if candle.len() < n { return None; }
    Some(candle[candle.len() - n..].iter().map(|c| c.dong).sum::<Price>() / n as Price)
}

impl Strategy for MeanCross {
    fn name(&self) -> &str { "Giao cắt trung bình động" }
    fn decide(&mut self, history: &[Candle], position: &Position) -> Signal {
        let (nhanh, cham) = match (mean(history, self.nhanh), mean(history, self.cham)) {
            (Some(a), Some(b)) => (a, b),
            _ => return Signal::Giu, // chưa đủ dữ liệu — KHÔNG đoán mò
        };
        if nhanh > cham && position.quantity <= 0 { Signal::Buy(self.don_pos) }
        else if nhanh < cham && position.quantity > 0 { Signal::Sell(position.quantity) }
        else { Signal::Giu }
    }
}

#[derive(Debug, PartialEq)]
pub struct ResultTest {
    pub last_position: Position,
    pub last_value: i64,
    pub num_trade: usize,
    /// Mức sụt giảm sâu nhất từ đỉnh — con số quan trọng hơn cả lợi nhuận,
    /// vì nó quyết định bạn có chịu nổi để đi hết chiến lược hay không.
    pub max_drawdown: i64,
    pub equity_curve: Vec<i64>,
}

/// Chạy kiểm định. Có mô hình TRƯỢT GIÁ và PHÍ — bỏ hai thứ này là cách
/// nhanh nhất để tự lừa mình bằng một đường vốn đẹp nhưng không có thật.
pub fn run_test(
    data: &[Candle],
    strategy: &mut dyn Strategy,
    slippage_ticks: Price,
    phi_new_don_pos: i64,
) -> ResultTest {
    let mut position = Position::RONG;
    let mut num_trade = 0;
    let mut equity_curve = Vec::with_capacity(data.len());
    let mut peak = i64::MIN;
    let mut max_sut = 0;

    for i in 0..data.len() {
        let history = &data[..=i];
        // Quyết định dựa trên nến ĐÃ ĐÓNG, khớp ở nến KẾ TIẾP.
        // Bỏ qua chi tiết này = "nhìn trộm tương lai", lỗi kinh điển
        // khiến mọi chiến lược trông như in tiền.
        let signal = strategy.decide(history, &position);
        if let Some(nen_sau) = data.get(i + 1) {
            let (side, amount) = match signal {
                Signal::Buy(q) => (Side::Buy, q),
                Signal::Sell(q) => (Side::Sell, q),
                Signal::Giu => { equity_curve.push(position.value_empty(data[i].dong)); continue; }
            };
            if amount > 0 {
                // Trượt giá: ta luôn mua đắt hơn và bán rẻ hơn giá lý thuyết.
                let price = nen_sau.mo + side.first() * slippage_ticks;
                position = position.compose(Position::from_fill(side, price, amount));
                position.tien_mat -= phi_new_don_pos * amount;
                num_trade += 1;
            }
        }
        let gt = position.value_empty(data[i].dong);
        equity_curve.push(gt);
        peak = peak.max(gt);
        max_sut = max_sut.max(peak - gt);
    }

    let last_price = data.last().map_or(0, |n| n.dong);
    ResultTest {
        last_value: position.value_empty(last_price),
        last_position: position,
        num_trade: num_trade,
        max_drawdown: max_sut,
        equity_curve,
    }
}

/// Sinh dữ liệu giá tất định (bước ngẫu nhiên có hạt giống cố định).
/// Tất định là điều kiện BẮT BUỘC để kiểm thử hồi quy có ý nghĩa.
pub fn gen_data(so_nen: usize, gia_dau: Price, hat_giong: u64) -> Vec<Candle> {
    let mut s = hat_giong;
    let mut price = gia_dau;
    (0..so_nen).map(|i| {
        s = s.wrapping_mul(6364136223846793005).wrapping_add(1442695040888963407);
        let step = ((s >> 33) % 41) as i64 - 20; // -20..+20 tick
        let mo = price;
        price = (price + step).max(1);
        Candle {
            timestamp: i as u64,
            mo,
            high: mo.max(price) + 5,
            low: (mo.min(price) - 5).max(1),
            dong: price,
        }
    }).collect()
}

fn main() {
    println!("═══════════════════════════════════════════════════════════");
    println!("   HỆ THỐNG GIAO DỊCH: SỔ LỆNH · KHỚP LỆNH · KIỂM ĐỊNH     ");
    println!("═══════════════════════════════════════════════════════════");

    println!("\n1. VÌ SAO TIỀN PHẢI LÀ SỐ NGUYÊN");
    let sai: f64 = (0..10).map(|_| 0.1f64).sum();
    println!("   Cộng 0.1 mười lần bằng f64 → {:.20}", sai);
    println!("   Bằng nhau với 1.0?          → {}", sai == 1.0);
    println!("   Bằng số nguyên tick         → {} tick = {}", 100, tick_to_string(100));

    println!("\n2. CỔNG RỦI RO");
    let hm = Limit { max_order_value: 1_000_000, max_position: 500,
                      list_wait_op: vec!["VNM".into(), "FPT".into()] };
    for (description, l) in [
        ("hợp lệ         ", Order::new(1, "VNM", Side::Buy, 8_500, 100)),
        ("mã lạ          ", Order::new(2, "XYZ", Side::Buy, 8_500, 100)),
        ("quá to         ", Order::new(3, "VNM", Side::Buy, 8_500, 1_000)),
        ("số lượng âm    ", Order::new(4, "VNM", Side::Buy, 8_500, -5)),
    ] {
        match hm.check(l, 0) {
            Ok(_) => println!("   {} → CHO QUA", description),
            Err(e) => println!("   {} → CHẶN: {:?}", description, e),
        }
    }

    println!("\n3. SỔ LỆNH & ƯU TIÊN GIÁ–THỜI GIAN");
    let mut so = OrderBook::new();
    let send = |id, side, price, sl| {
        Order::<DangSoan>::new(id, "VNM", side, price, sl)
            .transfer::<RiskChecked>().send()
    };
    for (id, price, sl) in [(10u64, 8_400i64, 100i64), (11, 8_400, 200), (12, 8_390, 500)] {
        so.nap(send(id, Side::Buy, price, sl));
    }
    for (id, price, sl) in [(20u64, 8_420i64, 150i64), (21, 8_430, 300)] {
        so.nap(send(id, Side::Sell, price, sl));
    }
    println!("   Mua tốt nhất {} · Bán tốt nhất {} · Chênh lệch {} tick",
             tick_to_string(so.best_bid().unwrap()),
             tick_to_string(so.best_ask().unwrap()),
             so.spread().unwrap());
    println!("   Khối lượng chờ mua ở {}: {}", tick_to_string(8_400), so.qty_at(Side::Buy, 8_400));

    println!("\n4. KHỚP LỆNH — lệnh bán 250 quét qua bên mua");
    let fill = so.nap(send(30, Side::Sell, 8_390, 250));
    for k in &fill {
        println!("   {} đơn vị @ {} (đối tác lệnh #{})",
                 k.quantity, tick_to_string(k.price), k.order_passive);
    }
    println!("   → Lệnh #10 (đặt trước) khớp hết TRƯỚC lệnh #11, dù cùng giá.");
    println!("   → Khớp ở giá {} chứ không phải {} — người đến sau được cải thiện giá.",
             tick_to_string(8_400), tick_to_string(8_390));

    println!("\n5. VỊ THẾ LÀ MỘT VỊ NHÓM");
    let a = Position::from_fill(Side::Buy, 8_400, 100);
    let b = Position::from_fill(Side::Sell, 8_500, 60);
    println!("   Mua 100@84.00 rồi bán 60@85.00 → {:?}", a.compose(b));
    println!("   Kết hợp: (a·b)·c == a·(b·c) → {}",
             a.compose(b).compose(Position::RONG) == a.compose(b.compose(Position::RONG)));

    println!("\n6. KIỂM ĐỊNH CHIẾN LƯỢC — 500 nến, có phí và trượt giá");
    let data = gen_data(500, 8_000, 42);
    for (truot, phi) in [(0i64, 0i64), (2, 3)] {
        let mut cl = MeanCross { nhanh: 5, cham: 20, don_pos: 100 };
        let kq = run_test(&data, &mut cl, truot, phi);
        println!("   trượt {} tick, phí {}/đv → lãi {:>8} tick · {} lệnh · sụt sâu nhất {} tick",
                 truot, phi, kq.last_value, kq.num_trade, kq.max_drawdown);
    }
    println!("   → Cùng một chiến lược: bỏ qua phí và trượt giá là tự lừa mình.");

    println!("\n═══════════════════════════════════════════════════════════");
    println!("   ĐỘ TRỄ CÓ TRẦN XÁC ĐỊNH — LÝ DO NGÀNH NÀY CHỌN RUST      ");
    println!("═══════════════════════════════════════════════════════════");
}

#[cfg(test)]
mod tests {
    use super::*;

    fn orders_sent(id: OrderId, side: Side, price: Price, sl: Quantity) -> Order<Sent> {
        Order::<DangSoan>::new(id, "VNM", side, price, sl).transfer::<RiskChecked>().send()
    }

    // ---------- Tiền & kiểu ----------
    #[test]
    fn tien_so_nguyen_khong_co_sai_so_tich_luy() {
        let f64_tong: f64 = (0..1000).map(|_| 0.01f64).sum();
        assert_ne!(f64_tong, 10.0, "f64 KHÔNG cộng đúng — đây là lý do không dùng nó cho tiền");
        let tick_tong: i64 = (0..1000).map(|_| 1i64).sum();
        assert_eq!(tick_tong, 1000, "số nguyên thì chính xác tuyệt đối");
    }

    #[test]
    fn hien_thi_tick_dung_ca_so_am() {
        assert_eq!(tick_to_string(8_450), "84.50");
        assert_eq!(tick_to_string(5), "0.05");
        assert_eq!(tick_to_string(-8_450), "-84.50");
    }

    // ---------- Rủi ro ----------
    #[test]
    fn cong_rui_ro_chan_dung_tung_loai_vi_pham() {
        let hm = Limit { max_order_value: 1_000_000, max_position: 500,
                          list_wait_op: vec!["VNM".into()] };
        // Dùng `unwrap_err()` chứ không `assert_eq!` cả `Result`: `Lenh` không
        // cài `PartialEq` (so sánh hai lệnh theo giá trị là vô nghĩa — mỗi lệnh
        // có danh tính riêng qua `id`).
        assert!(hm.check(Order::new(1, "VNM", Side::Buy, 8_500, 100), 0).is_ok());
        assert_eq!(hm.check(Order::new(2, "VNM", Side::Buy, 8_500, 0), 0).unwrap_err(),
                   ErrorRisk::SoLuongKhongDuong(0));
        assert_eq!(hm.check(Order::new(3, "VNM", Side::Buy, 0, 10), 0).unwrap_err(),
                   ErrorRisk::GiaKhongDuong(0));
        assert_eq!(hm.check(Order::new(4, "XYZ", Side::Buy, 100, 10), 0).unwrap_err(),
                   ErrorRisk::MaChungKhoanLa("XYZ".into()));
        assert!(matches!(hm.check(Order::new(5, "VNM", Side::Buy, 8_500, 1_000), 0).unwrap_err(),
                         ErrorRisk::VuotGiaTriToiDa { .. }));
    }

    #[test]
    fn limit_position_tinh_all_side_sell_no() {
        let hm = Limit { max_order_value: i64::MAX, max_position: 100,
                          list_wait_op: vec!["VNM".into()] };
        // bán khống 150 khi đang giữ 0 → vị thế -150, vượt trần 100
        assert_eq!(hm.check(Order::new(1, "VNM", Side::Sell, 100, 150), 0).unwrap_err(),
                   ErrorRisk::VuotViTheToiDa { next_order: -150, tran: 100 });
        // nhưng bán 150 khi đang giữ 100 → còn -50, hợp lệ
        assert!(hm.check(Order::new(2, "VNM", Side::Sell, 100, 150), 100).is_ok());
    }

    // ---------- Sổ lệnh ----------
    #[test]
    fn order_book_return_use_price_good_nhat_two_side() {
        let mut s = OrderBook::new();
        s.nap(orders_sent(1, Side::Buy, 100, 10));
        s.nap(orders_sent(2, Side::Buy, 105, 10)); // giá cao hơn = tốt hơn cho bên mua
        s.nap(orders_sent(3, Side::Sell, 120, 10));
        s.nap(orders_sent(4, Side::Sell, 110, 10)); // giá thấp hơn = tốt hơn cho bên bán
        assert_eq!(s.best_bid(), Some(105));
        assert_eq!(s.best_ask(), Some(110));
        assert_eq!(s.spread(), Some(5));
        assert_eq!(s.mid(), Some(107));
    }

    #[test]
    fn order_no_giao_each_thi_nam_lai_num() {
        let mut s = OrderBook::new();
        assert!(s.nap(orders_sent(1, Side::Buy, 100, 10)).is_empty());
        assert!(s.nap(orders_sent(2, Side::Sell, 110, 10)).is_empty());
        assert_eq!(s.total_order_book(), 2);
    }

    #[test]
    fn uu_tien_time_time_cell_same_level_price() {
        let mut s = OrderBook::new();
        s.nap(orders_sent(1, Side::Buy, 100, 50));  // đến TRƯỚC
        s.nap(orders_sent(2, Side::Buy, 100, 50));  // đến SAU
        let fill = s.nap(orders_sent(3, Side::Sell, 100, 60));
        assert_eq!(fill.len(), 2);
        assert_eq!(fill[0].order_passive, 1, "lệnh đến trước phải khớp trước");
        assert_eq!(fill[0].quantity, 50);
        assert_eq!(fill[1].order_passive, 2);
        assert_eq!(fill[1].quantity, 10);
    }

    #[test]
    fn uu_tien_gia_thang_uu_tien_thoi_gian() {
        let mut s = OrderBook::new();
        s.nap(orders_sent(1, Side::Buy, 100, 50));  // đến trước, giá THẤP hơn
        s.nap(orders_sent(2, Side::Buy, 105, 50));  // đến sau, giá CAO hơn
        let fill = s.nap(orders_sent(3, Side::Sell, 100, 10));
        assert_eq!(fill[0].order_passive, 2, "giá tốt hơn thắng, dù đến sau");
        assert_eq!(fill[0].price, 105);
    }

    #[test]
    fn nguoi_den_sau_duoc_cai_thien_gia() {
        let mut s = OrderBook::new();
        s.nap(orders_sent(1, Side::Sell, 100, 10)); // ai đó chào bán rẻ
        // ta sẵn sàng mua tới 120, nhưng chỉ phải trả 100
        let fill = s.nap(orders_sent(2, Side::Buy, 120, 10));
        assert_eq!(fill[0].price, 100, "khớp ở giá của lệnh nằm sẵn trong sổ");
    }

    #[test]
    fn order_lon_scan_qua_many_level_price() {
        let mut s = OrderBook::new();
        s.nap(orders_sent(1, Side::Sell, 100, 10));
        s.nap(orders_sent(2, Side::Sell, 101, 10));
        s.nap(orders_sent(3, Side::Sell, 102, 10));
        let fill = s.nap(orders_sent(4, Side::Buy, 102, 25));
        assert_eq!(fill.len(), 3);
        assert_eq!(fill.iter().map(|k| k.price).collect::<Vec<_>>(), vec![100, 101, 102],
                   "phải ăn từ giá tốt nhất trở đi");
        assert_eq!(fill.iter().map(|k| k.quantity).sum::<i64>(), 25);
        assert_eq!(s.total_order_book(), 1, "mức 102 còn dư 5 đơn vị");
    }

    #[test]
    fn part_data_cua_order_aggressive_nam_lai_num() {
        let mut s = OrderBook::new();
        s.nap(orders_sent(1, Side::Sell, 100, 10));
        let fill = s.nap(orders_sent(2, Side::Buy, 100, 30));
        assert_eq!(fill.iter().map(|k| k.quantity).sum::<i64>(), 10);
        assert_eq!(s.best_bid(), Some(100), "20 đơn vị còn lại thành lệnh chờ mua");
        assert_eq!(s.qty_at(Side::Buy, 100), 20);
    }

    #[test]
    fn report_toan_quantity_qua_new_lan_fill() {
        // BẤT BIẾN SỐNG CÒN của mọi sàn: không đơn vị nào được sinh ra
        // hay biến mất trong quá trình khớp.
        let mut s = OrderBook::new();
        let mut da_nap = 0i64;
        let mut filled = 0i64;
        for i in 0..60u64 {
            let side = if i % 2 == 0 { Side::Buy } else { Side::Sell };
            let price = 100 + ((i * 7) % 11) as i64 - 5;
            let sl = 10 + (i % 13) as i64;
            da_nap += sl;
            filled += s.nap(orders_sent(i, side, price, sl))
                        .iter().map(|k| k.quantity).sum::<i64>();
        }
        let con_weight: i64 = [Side::Buy, Side::Sell].iter()
            .flat_map(|&c| (80..=120).map(move |g| (c, g)))
            .map(|(c, g)| s.qty_at(c, g)).sum();
        // Mỗi lần khớp tiêu thụ khối lượng từ CẢ HAI phía
        assert_eq!(da_nap - 2 * filled, con_weight,
                   "khối lượng phải cân bằng tuyệt đối");
    }

    #[test]
    fn huy_lenh_go_dung_lenh_va_don_muc_gia_rong() {
        let mut s = OrderBook::new();
        s.nap(orders_sent(1, Side::Buy, 100, 10));
        s.nap(orders_sent(2, Side::Buy, 100, 20));
        assert!(s.cancel(1));
        assert_eq!(s.qty_at(Side::Buy, 100), 20);
        assert!(s.cancel(2));
        assert_eq!(s.best_bid(), None, "mức giá rỗng phải bị xóa khỏi sổ");
        assert!(!s.cancel(999), "hủy lệnh không tồn tại phải trả false");
    }

    #[test]
    fn so_rong_khong_co_gia_va_khong_panic() {
        let s = OrderBook::new();
        assert_eq!(s.best_bid(), None);
        assert_eq!(s.spread(), None);
        assert_eq!(s.mid(), None);
        assert_eq!(s.total_order_book(), 0);
    }

    // ---------- Vị thế ----------
    #[test]
    fn vi_the_thoa_luat_vi_nhom() {
        let a = Position::from_fill(Side::Buy, 100, 10);
        let b = Position::from_fill(Side::Sell, 110, 5);
        let c = Position::from_fill(Side::Buy, 90, 3);
        assert_eq!(a.compose(b).compose(c), a.compose(b.compose(c)), "luật kết hợp");
        assert_eq!(a.compose(Position::RONG), a, "luật đơn vị phải");
        assert_eq!(Position::RONG.compose(a), a, "luật đơn vị trái");
    }

    #[test]
    fn coalesce_position_theo_block_wait_same_result() {
        // Vì là vị nhóm, chia nhỏ rồi gộp lại (như khi dùng rayon) cho kết quả
        // Y HỆT tính tuần tự. Đây là bảo chứng toán học, không phải may mắn.
        let fill: Vec<Position> = (0..100).map(|i| {
            let side = if i % 3 == 0 { Side::Sell } else { Side::Buy };
            Position::from_fill(side, 100 + i % 7, 1 + i % 5)
        }).collect();
        let tuan_tu = fill.iter().fold(Position::RONG, |a, &b| a.compose(b));
        let theo_block = fill.chunks(7)
            .map(|k| k.iter().fold(Position::RONG, |a, &b| a.compose(b)))
            .fold(Position::RONG, |a, b| a.compose(b));
        assert_eq!(tuan_tu, theo_block);
    }

    #[test]
    fn mua_roi_ban_cao_hon_thi_co_lai() {
        let v = Position::from_fill(Side::Buy, 8_000, 100)
            .compose(Position::from_fill(Side::Sell, 8_500, 100));
        assert_eq!(v.quantity, 0, "đã đóng hết vị thế");
        assert_eq!(v.value_empty(0), 50_000, "(8500-8000) × 100 tick");
    }

    #[test]
    fn position_open_can_peak_price_lai_theo_market() {
        let v = Position::from_fill(Side::Buy, 8_000, 100);
        assert_eq!(v.value_empty(8_000), 0, "vừa mua xong thì hòa vốn");
        assert_eq!(v.value_empty(8_100), 10_000, "giá lên 100 tick → lãi 10 000");
        assert_eq!(v.value_empty(7_900), -10_000, "giá xuống thì lỗ đối xứng");
    }

    // ---------- Kiểm định ----------
    #[test]
    fn sinh_du_lieu_tat_dinh_theo_hat_giong() {
        assert_eq!(gen_data(50, 8_000, 7), gen_data(50, 8_000, 7));
        assert_ne!(gen_data(50, 8_000, 7), gen_data(50, 8_000, 8));
    }

    #[test]
    fn data_gen_out_always_hop_le() {
        for candle in gen_data(500, 8_000, 99) {
            assert!(candle.high >= candle.mo && candle.high >= candle.dong, "đỉnh phải cao nhất");
            assert!(candle.low <= candle.mo && candle.low <= candle.dong, "đáy phải thấp nhất");
            assert!(candle.low > 0, "giá không bao giờ âm");
        }
    }

    #[test]
    fn chien_luoc_giu_im_khi_chua_du_du_lieu() {
        let mut cl = MeanCross { nhanh: 5, cham: 20, don_pos: 100 };
        let few_candle = gen_data(10, 8_000, 1);
        assert_eq!(cl.decide(&few_candle, &Position::RONG), Signal::Giu,
                   "chưa đủ 20 nến thì KHÔNG được đoán mò");
    }

    #[test]
    fn test_tai_loop_can_hoan_toan() {
        let data = gen_data(300, 8_000, 42);
        let run = || {
            let mut cl = MeanCross { nhanh: 5, cham: 20, don_pos: 100 };
            run_test(&data, &mut cl, 2, 3)
        };
        assert_eq!(run(), run(), "cùng dữ liệu + cùng chiến lược = cùng kết quả, luôn luôn");
    }

    #[test]
    fn phi_va_truot_gia_luon_lam_ket_qua_xau_di() {
        let data = gen_data(400, 8_000, 2024);
        let mut cl1 = MeanCross { nhanh: 5, cham: 20, don_pos: 100 };
        let ly_tuong = run_test(&data, &mut cl1, 0, 0);
        let mut cl2 = MeanCross { nhanh: 5, cham: 20, don_pos: 100 };
        let actual = run_test(&data, &mut cl2, 2, 3);
        assert_eq!(ly_tuong.num_trade, actual.num_trade, "cùng số lệnh");
        assert!(actual.last_value < ly_tuong.last_value,
                "chi phí giao dịch luôn ăn vào lợi nhuận: {} so với {}",
                actual.last_value, ly_tuong.last_value);
    }

    #[test]
    fn sut_giam_toi_da_khong_bao_gio_am() {
        for hat in [1u64, 7, 42, 2024, 31337] {
            let data = gen_data(200, 8_000, hat);
            let mut cl = MeanCross { nhanh: 3, cham: 10, don_pos: 50 };
            let kq = run_test(&data, &mut cl, 1, 1);
            assert!(kq.max_drawdown >= 0, "sụt giảm là khoảng cách, không thể âm");
            assert_eq!(kq.equity_curve.len(), data.len());
        }
    }

    #[test]
    fn no_trade_thi_no_lai_no_lo() {
        struct NoOp;
        impl Strategy for NoOp {
            fn name(&self) -> &str { "đứng ngoài" }
            fn decide(&mut self, _: &[Candle], _: &Position) -> Signal { Signal::Giu }
        }
        let data = gen_data(200, 8_000, 5);
        let kq = run_test(&data, &mut NoOp, 5, 10);
        assert_eq!(kq.num_trade, 0);
        assert_eq!(kq.last_value, 0, "không vào lệnh thì không thể mất tiền");
        assert_eq!(kq.max_drawdown, 0);
    }

    #[test]
    fn chien_luoc_khong_duoc_nhin_trom_tuong_lai() {
        // Nếu bộ kiểm định khớp ở giá ĐÓNG của chính cây nến ra tín hiệu,
        // ta đã dùng thông tin chưa tồn tại. Ở đây khớp ở giá MỞ của nến kế
        // tiếp, nên nến CUỐI CÙNG không thể sinh giao dịch nào.
        let data = gen_data(30, 8_000, 3);
        struct AlwaysBuy;
        impl Strategy for AlwaysBuy {
            fn name(&self) -> &str { "luôn mua" }
            fn decide(&mut self, _: &[Candle], _: &Position) -> Signal { Signal::Buy(1) }
        }
        let kq = run_test(&data, &mut AlwaysBuy, 0, 0);
        assert_eq!(kq.num_trade, data.len() - 1,
                   "nến cuối không có nến kế tiếp để khớp — không được bịa ra giao dịch");
    }
}
```

---

## Bảng tra cứu lỗi biên dịch thường gặp

| Lỗi | Nguyên nhân trong chương này | Cách sửa |
|---|---|---|
| `E0277: RiskChecked doesn't implement Debug` | `.unwrap_err()` cần kiểu `Ok` cài `Debug` | Thêm `#[derive(Debug)]` cho các nhãn typestate |
| `E0599: no method named 'gui' found for Lenh<DangSoan>` | **Đây là tính năng!** Chưa qua cổng rủi ro | Gọi `limit.check(lenh, position)?` trước |
| `E0382: use of moved value: 'lenh'` | Gửi cùng một lệnh hai lần | Đúng như thiết kế — mỗi lệnh chỉ gửi được một lần |
| `E0502: cannot borrow self.ben_ban as mutable` | Đọc `keys().next()` rồi `get_mut` cùng lúc | Tách thành hai khối: lấy khóa ra trước trong `{ }` riêng |
| Số dư tài khoản lệch vài xu sau nhiều giao dịch | Dùng `f64` cho tiền | Chuyển hết sang số nguyên tick |
| Chiến lược lãi phi thực tế trong kiểm định | Nhìn trộm tương lai | Khớp ở nến **kế tiếp**, không phải nến ra tín hiệu |

---

## Tóm tắt chương & Bài tập rèn luyện (Summary & Exercises)

### 6 điểm cốt lõi cần ghi nhớ

1. **Tiền là số nguyên.** Không có ngoại lệ, không có "chỉ chỗ này thôi". Đếm bằng đơn vị nhỏ nhất.
2. **Ưu tiên giá trước, ưu tiên thời gian sau.** Hai quy tắc này định nghĩa mọi sàn giao dịch trên thế giới.
3. **Người đến sau được cải thiện giá.** Giá khớp là giá của lệnh đã nằm sẵn — đó là điều khiến người ta dám đặt lệnh giới hạn.
4. **Typestate biến lỗi vận hành thành lỗi biên dịch.** Lệnh chưa qua rủi ro không gửi được; lệnh đã gửi không gửi lại được.
5. **Vị thế là vị nhóm** — và vì thế gộp song song cho kết quả y hệt gộp tuần tự, có bảo chứng toán học.
6. **Bộ kiểm định trung thực phải làm bạn thất vọng.** Không nhìn trộm tương lai, có phí, có trượt giá, và báo cả sụt giảm tối đa chứ không chỉ lợi nhuận.

### Bài tập rèn luyện tự giải

**Bài 1.** Thêm các **loại lệnh** khác: lệnh thị trường (khớp mọi giá), IOC (khớp được bao nhiêu hay bấy nhiêu, phần dư hủy), và FOK (khớp toàn bộ hoặc không khớp gì).

<details>
<summary><b>Gợi ý</b></summary>

- **Thị trường** = lệnh giới hạn với giá `i64::MAX` (mua) hoặc `1` (bán). Không cần logic mới.
- **IOC** = nạp bình thường, nhưng nếu còn dư thì **không** để lại sổ.
- **FOK** cần *kiểm tra trước*: duyệt sổ tính xem có đủ khối lượng khớp được không, nếu không thì từ chối luôn mà **không** khớp phần nào.

FOK khó nhất vì phải "thử mà không làm". Cách sạch nhất: viết một hàm `khoi_luong_khop_duoc()` chỉ đọc, không sửa sổ.
</details>

<details>
<summary><b>Lời giải</b></summary>

```rust
#[derive(Debug, Clone, Copy, PartialEq)]
pub enum LoaiLenh { GioiHan, ThiTruong, Ioc, Fok }

impl OrderBook {
    /// Tính trước khối lượng khớp được mà KHÔNG sửa sổ — cần cho FOK.
    pub fn khoi_luong_khop_duoc(&self, side: Side, price: Price) -> Quantity {
        let swap_resp = match side { Side::Buy => &self.ben_ban, Side::Sell => &self.side_buy };
        swap_resp.iter()
            .take_while(|(key, _)| {
                let price_true = if side == Side::Buy { **key } else { -**key };
                if side == Side::Buy { price_true <= price } else { price_true >= price }
            })
            .flat_map(|(_, queue)| queue.iter().map(|l| l.remaining()))
            .sum()
    }

    pub fn nap_voi_loai(&mut self, mut order: Order<Sent>, kind: LoaiLenh) -> Vec<Fill> {
        // Lệnh thị trường = lệnh giới hạn với giá cực đoan
        if kind == LoaiLenh::ThiTruong {
            order.price = if order.side == Side::Buy { i64::MAX } else { 1 };
        }
        // FOK: kiểm tra TRƯỚC, không khớp một phần nào
        if kind == LoaiLenh::Fok
            && self.khoi_luong_khop_duoc(order.side, order.price) < order.quantity {
            return Vec::new();
        }

        let id = order.id;
        let fill = self.nap(order);

        // IOC và thị trường: phần dư KHÔNG được nằm lại sổ
        if matches!(kind, LoaiLenh::Ioc | LoaiLenh::ThiTruong | LoaiLenh::Fok) {
            self.cancel(id);
        }
        fill
    }
}
```

Chú ý `take_while` chứ không phải `filter`: vì `BTreeMap` đã sắp thứ tự, gặp mức giá đầu tiên **không** khớp được nghĩa là mọi mức sau cũng không khớp được. Dừng sớm thay vì duyệt hết sổ — chi tiết nhỏ nhưng ở tốc độ hàng triệu lệnh mỗi giây thì nó quyết định.
</details>

**Bài 2.** Cài chiến lược **hồi quy về trung bình** (mua khi giá thấp hơn trung bình một độ lệch chuẩn, bán khi cao hơn) và so sánh với giao cắt trung bình động trên cùng dữ liệu.

<details>
<summary><b>Gợi ý</b></summary>

Tính trung bình và độ lệch chuẩn trên cửa sổ `n` nến gần nhất. Mua khi `gia < trung_bình - k·độ_lệch`, bán khi `gia > trung_bình + k·độ_lệch`. Đây chính là dải Bollinger.

Vì đang dùng số nguyên tick, hãy tính phương sai bằng số nguyên rồi lấy căn bằng `(x as f64).sqrt() as i64` — hoặc dùng độ lệch tuyệt đối trung bình (MAD) để tránh hẳn dấu phẩy động, như Chương 58 đã bàn về thống kê bền vững.

Dự đoán trước khi chạy: trên dữ liệu **bước ngẫu nhiên**, hồi quy về trung bình thường trông tốt hơn theo xu hướng — nhưng cả hai đều thua chi phí về dài hạn. Đó là bài học chứ không phải thất bại.
</details>

<details>
<summary><b>Lời giải</b></summary>

```rust
pub struct HoiQuyTrungBinh { pub window: usize, pub he_so: i64, pub don_pos: Quantity }

impl Strategy for HoiQuyTrungBinh {
    fn name(&self) -> &str { "Hồi quy về trung bình" }

    fn decide(&mut self, history: &[Candle], position: &Position) -> Signal {
        if history.len() < self.window { return Signal::Giu; }
        let window = &history[history.len() - self.window..];
        let n = self.window as i64;
        let tb: Price = window.iter().map(|c| c.dong).sum::<Price>() / n;

        // Độ lệch tuyệt đối trung bình — toàn số nguyên, bền với giá trị dị biệt
        let dltb: i64 = window.iter().map(|c| (c.dong - tb).abs()).sum::<i64>() / n;
        let price = history.last().unwrap().dong;
        let threshold = self.he_so * dltb;

        if price < tb - threshold && position.quantity <= 0 {
            Signal::Buy(self.don_pos)          // rẻ bất thường → mua
        } else if price > tb + threshold && position.quantity > 0 {
            Signal::Sell(position.quantity)      // đắt bất thường → chốt
        } else {
            Signal::Giu
        }
    }
}
```

Chạy cả hai chiến lược trên cùng dữ liệu và cùng chi phí, rồi so **ba** con số: lợi nhuận, số giao dịch, và sụt giảm tối đa. Bạn sẽ thấy hồi quy về trung bình giao dịch **nhiều hơn** (nên trả phí nhiều hơn) nhưng sụt giảm **nông hơn**.

Bài học quan trọng nhất: đừng chọn chiến lược chỉ vì nó lãi nhất trên một bộ dữ liệu. Đổi hạt giống sinh dữ liệu và chạy lại — nếu thứ hạng đảo lộn, bạn vừa **khớp quá mức** (overfit) chứ không phát hiện ra quy luật nào cả.
</details>

**Bài 3.** Thêm **nhật ký sự kiện** (event sourcing) vào sổ lệnh: ghi mọi lệnh và mọi lần khớp, rồi chứng minh phát lại nhật ký từ đầu tái tạo **chính xác** trạng thái sổ.

<details>
<summary><b>Gợi ý</b></summary>

Đây là kiến trúc mà mọi sàn giao dịch thật đều dùng, vì ba lý do: kiểm toán pháp lý, khôi phục sau sự cố, và gỡ lỗi ("tại sao lệnh này khớp ở giá đó?").

Điều kiện tiên quyết: động cơ khớp lệnh phải **hoàn toàn tất định** — cùng chuỗi sự kiện đầu vào luôn cho cùng trạng thái đầu ra. Không dùng thời gian hệ thống, không dùng số ngẫu nhiên, không dùng thứ tự duyệt `HashMap`.

Bài kiểm thử là phần đắt giá nhất: nạp ngẫu nhiên 1 000 lệnh vào hai sổ — một sổ nạp trực tiếp, một sổ phát lại từ nhật ký — rồi khẳng định trạng thái hai sổ giống hệt nhau.
</details>

<details>
<summary><b>Lời giải</b></summary>

```rust
#[derive(Debug, Clone, PartialEq)]
pub enum Event {
    NhanLenh { id: OrderId, side: Side, price: Price, quantity: Quantity },
    CancelOrder { id: OrderId },
}

pub struct SoLenhCoNhatKy {
    pub so: OrderBook,
    pub order_log: Vec<Event>,
}

impl SoLenhCoNhatKy {
    pub fn new() -> Self { SoLenhCoNhatKy { so: OrderBook::new(), order_log: Vec::new() } }

    pub fn nap(&mut self, order: Order<Sent>) -> Vec<Fill> {
        // GHI NHẬT KÝ TRƯỚC khi thay đổi trạng thái — nếu sập giữa chừng,
        // nhật ký vẫn đủ để dựng lại. Đây là nguyên tắc WAL của Chương 34.
        self.order_log.push(Event::NhanLenh {
            id: order.id, side: order.side, price: order.price, quantity: order.quantity,
        });
        self.so.nap(order)
    }

    pub fn cancel(&mut self, id: OrderId) -> bool {
        self.order_log.push(Event::CancelOrder { id });
        self.so.cancel(id)
    }

    /// Dựng lại toàn bộ sổ chỉ từ nhật ký. Không cần ảnh chụp trạng thái nào.
    pub fn phat_lai(order_log: &[Event]) -> OrderBook {
        let mut so = OrderBook::new();
        for sk in order_log {
            match sk {
                Event::NhanLenh { id, side, price, quantity } => {
                    let l = Order::<DangSoan>::new(*id, "VNM", *side, *price, *quantity)
                        .transfer::<RiskChecked>().send();
                    so.nap(l);
                }
                Event::CancelOrder { id } => { so.cancel(*id); }
            }
        }
        so
    }
}

// Bài kiểm thử quan trọng nhất:
//   let mut s = SoLenhCoNhatKy::moi();
//   for i in 0..1000 { s.nap(sinh_lenh_tat_dinh(i)); }
//   let dung_lai = SoLenhCoNhatKy::phat_lai(&s.order_log);
//   assert_eq!(dung_lai.best_bid(), s.so.best_bid());
//   assert_eq!(dung_lai.best_ask(), s.so.best_ask());
//   assert_eq!(dung_lai.total_order_book(), s.so.total_order_book());
```

Chú ý thứ tự trong `nap`: **ghi nhật ký trước, sửa trạng thái sau**. Đây chính là nguyên tắc ghi-trước (Write-Ahead Logging) của Chương 34, áp dụng nguyên vẹn. Nếu tiến trình sập ngay sau khi ghi nhật ký, phát lại vẫn cho trạng thái đúng. Nếu sập trước khi ghi, sự kiện coi như chưa từng xảy ra — cũng nhất quán.

Đây là chỗ ba chương gặp nhau: WAL từ cơ sở dữ liệu (Ch34), tính tất định từ lập trình hàm (Ch13), và sổ lệnh của chương này.
</details>
