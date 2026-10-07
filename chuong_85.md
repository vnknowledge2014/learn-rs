# Chương 85: Hệ sinh thái HFT tích hợp — Nối mọi mảnh thành một hệ chạy được (Integrated HFT Ecosystem)

## Giới thiệu & Mục tiêu học tập

Chương 74–78 dựng từng mảnh: đo độ trễ, sổ lệnh, phát lại, cổng rủi ro, AMM. Mỗi mảnh đều chạy và đều có kiểm thử. Nhưng **năm mảnh chạy riêng không phải một hệ thống.**

Chương này nối chúng lại thành một hệ chạy end-to-end, đồng thời trên **hai loại thị trường**:

```
nguồn phiên ──► bộ phát lại (đồng hồ ảo, đẩy tốc độ ×N)
                     │
         ┌───────────┴───────────┐
         ▼                       ▼
 sàn TRUYỀN THỐNG          sàn CHUỖI KHỐI
 (sổ lệnh giá–thời gian)   (bể AMM x·y=k)
         └───────────┬───────────┘
                     ▼
           ảnh chụp thị trường hợp nhất
                     ▼
              chiến lược (nhiều)
                     ▼
                cổng rủi ro
                     ▼
      OMS: gửi lệnh CÓ ĐỘ TRỄ (hàng đợi theo thời điểm đến)
                     ▼
         sàn khớp ──► lãi lỗ, tồn kho, đo lường
```

Điều đáng học nhất ở chương này không phải kiến trúc. Đó là **năm lỗi chỉ lộ ra khi ghép các mảnh lại** — mỗi mảnh riêng lẻ đều đúng, nhưng hệ thống thì sai. Toàn bộ năm lỗi đều được phát hiện bằng cách **chạy**, không phải bằng cách đọc, và mỗi lỗi giờ có một bài kiểm thử canh gác.

Ba bất biến bắt buộc, mỗi bất biến một bài kiểm thử:
1. **Tất định** — chạy hai lần, và chạy ở ba tốc độ khác nhau, cho kết quả trùng khớp từng bit.
2. **Nhân quả** — chiến lược không bao giờ thấy dữ liệu tương lai; lệnh tới sàn sau một khoảng trễ và khớp theo trạng thái sàn **tại thời điểm đến**.
3. **Bất biến rủi ro** — không kịch bản nào vượt hạn mức vị thế, và sổ lệnh không bao giờ chéo.

---

## Hình tượng hóa đời sống

```
┌──────────────────────────────────────────────────────────────────────────────┐
│  NĂM LỖI CHỈ LỘ RA KHI GHÉP                                                 │
│                                                                              │
│  ① ĐOÁN CHIỀU TỪ GIÁ                                                        │
│     Khớp thụ động về, ta không biết nó là mua hay bán → đoán từ giá.        │
│     Đoán sai → vị thế chạy NGƯỢC → mọi hạn mức rủi ro thành vô nghĩa.       │
│     Chữa: Fill MANG THEO chiều. Đừng bao giờ suy ra cái mình biết sẵn.  │
│                                                                              │
│  ② KHỚP TRÀN QUA NHIỀU MỨC GIÁ                                              │
│     Lệnh thị trường 10 đơn vị cắt qua mức 100 → ta gọi hàm khớp toàn sổ     │
│     → nó ăn luôn lệnh của ta ở mức 99, 98, 97... 10 đơn vị "khớp" 200.      │
│     Chữa: khớp ĐÚNG một mức, ĐÚNG khối lượng còn lại.                       │
│                                                                              │
│  ③ BÁO GIÁ KHÔNG BAO GIỜ ĐƯỢC RÚT                                           │
│     MM báo giá mỗi 1 ms, không huỷ báo giá cũ → sau 2 giây có 2000 lệnh     │
│     treo → phơi nhiễm chạm trần → cổng rủi ro chặn 99,3% lệnh mới.          │
│     Hệ thống TỰ BÓP CỔ MÌNH. Chữa: chính sách rút báo giá quá tuổi.         │
│                                                                              │
│  ④ CỔNG RỦI RO KHÔNG THẤY LỆNH ĐANG BAY  ← lỗ hổng đắt nhất                │
│                                                                              │
│     nhịp t:  ý định A ──kiểm──► vị thế 0, treo 0  → CHO QUA                 │
│              ý định B ──kiểm──► vị thế 0, treo 0  → CHO QUA  (!!)           │
│              ý định C ──kiểm──► vị thế 0, treo 0  → CHO QUA  (!!)           │
│              ...cả ba đều thấy CÙNG một trạng thái...                       │
│     nhịp t+50µs: cả ba tới sàn, cả ba khớp → vị thế vượt hạn mức 3 LẦN     │
│                                                                              │
│     Mọi phép kiểm đều đã chạy. Mọi phép kiểm đều trả "OK". Vẫn vỡ.          │
│     Chữa: ĐẶT CHỖ phơi nhiễm ngay lúc phát, không đợi lúc giao.             │
│                                                                              │
│  ⑤ NHÀ TẠO LẬP CẮT QUA SỔ                                                   │
│     Báo giá ở giữa ± 2 tick, nhưng sổ đang hẹp hơn → báo giá CẮT QUA        │
│     → MM trở thành người CHỦ ĐỘNG → TRẢ chênh lệch thay vì THU.             │
│     Đo được: tỉ lệ thụ động 19% thay vì >80%. Chữa: kẹp giá, không cắt.     │
│                                                                              │
│  BẤT ĐỐI XỨNG KHỚP — vì sao "hai chân" vẫn chưa đủ                          │
│                                                                              │
│     chân AMM      : LUÔN khớp đủ (công thức không từ chối ai)               │
│     chân sổ lệnh  : khớp MỘT PHẦN (sổ đã đổi trong 50 µs độ trễ)            │
│                     └── chênh lệch đọng lại thành vị thế ròng               │
│                                                                              │
│     Cách của ngành: chạy chân KHÔNG CHẮC trước, rồi phòng vệ ĐÚNG BẰNG      │
│     khối lượng thực sự khớp được. Vị thế ròng khi đó = 0 chính xác.         │
└──────────────────────────────────────────────────────────────────────────────┘
```

---

## Khái niệm & Cơ chế kỹ thuật chuyên sâu

### 1. Đồng hồ ảo phải là nguồn thời gian duy nhất

Toàn bộ hệ thống đọc thời gian từ `VirtualClock`. Không thành phần nào gọi `Instant::now()`. Đó không phải quy ước lịch sự — nó là điều kiện để hai tính chất cùng tồn tại:

- **Đẩy tốc độ không đổi kết quả.** Phát lại ở ×1, ×1000 hay vô hạn đều cho cùng dòng lệnh, vì thời gian **ảo** không đổi, chỉ thời gian **tường** bị nén. Chương này kiểm thử đúng điều đó.
- **Tất định.** Một lời gọi đồng hồ thật lạc lõng là đủ khiến hai lần chạy khác nhau, và khi đó bạn không thể gỡ lỗi bất kỳ sự cố sản xuất nào.

`VirtualClock::advance` **từ chối** lùi thời gian thay vì im lặng sắp xếp lại. Dữ liệu xếp sai thứ tự là lỗi thu thập; giấu nó đi thì bộ phát lại sẽ nói dối một cách thuyết phục.

### 2. Cổng rủi ro phải đặt chỗ, không chỉ kiểm tra

Đây là bài học đắt nhất của chương.

Một cổng rủi ro "đúng" theo nghĩa thông thường sẽ kiểm mọi lệnh trước khi gửi. Nhưng nếu ba lệnh được phát trong **cùng một nhịp**, cả ba đều được kiểm trên **cùng một trạng thái vị thế** — vì trạng thái chỉ thay đổi khi lệnh tới sàn 50 µs sau. Cả ba đều qua. Cả ba đều khớp. Hạn mức bị vượt gấp ba lần, và **không phép kiểm nào đã thất bại**.

Cách chữa là tách phơi nhiễm thành ba tầng và cộng đủ cả ba:

```
phơi_nhiễm = vị_thế_đã_khớp  +  lệnh_ĐANG_TREO  +  lệnh_ĐANG_BAY
```

và **đặt chỗ ngay khi cho qua**, trước khi xét ý định tiếp theo. Trong chương này đó là hai biến `in_flight_bid`/`in_flight_ask`, tăng lúc phát và chuyển sang `resting_bid`/`resting_ask` lúc giao.

Sau khi sửa, |vị thế| không bao giờ vượt hạn mức ở **bất kỳ** thời điểm nào trong phiên — hệ thống ghi lại `peak_abs_position` sau mỗi lần khớp, và bài kiểm thử canh con số đó trên nhiều hạt giống chứ không chỉ nhìn vị thế cuối phiên. Vị thế cuối nằm trong hạn mức không chứng minh được gì nếu giữa phiên đã vượt.

### 3. Huỷ lệnh phải đi đường ưu tiên

Trong hệ thống này, lệnh **đặt** chịu độ trễ còn lệnh **huỷ** đi thẳng. Đó không phải sự thiên vị tuỳ tiện: sàn thật xử lý huỷ trên đường ưu tiên, và quan trọng hơn — nếu huỷ cũng phải xếp hàng thì **rủi ro tồn kho không bao giờ giảm được**. Bạn sẽ có một hệ thống chỉ biết tăng phơi nhiễm.

Cùng lý do đó, cổng rủi ro **luôn cho lệnh huỷ đi qua**, kể cả khi công tắc ngắt khẩn cấp đã bật. Một công tắc ngắt chặn cả đường rút chân là một cái bẫy, không phải một biện pháp an toàn.

### 4. Nhà tạo lập không được cắt qua sổ

Một nhà tạo lập kiếm tiền bằng cách **thu** chênh lệch: mua ở giá mua, bán ở giá bán, ăn phần giữa. Nếu báo giá của nó cắt qua bên kia, nó trở thành người **chủ động** và **trả** chênh lệch.

Điều này nghe hiển nhiên, nhưng nó xảy ra âm thầm: chiến lược tính giá quanh vi giá với chênh lệch mục tiêu 4 tick, mà sổ lúc đó chỉ rộng 1 tick. Báo giá cắt qua, và không có gì báo lỗi cả.

Cách phát hiện là một chỉ số vận hành: **tỉ lệ thụ động**. Trước khi sửa, chỉ số này là 19%. Sau khi kẹp giá để không bao giờ cắt, nó lên trên 80% khi chạy nhà tạo lập một mình. Một nhà tạo lập có tỉ lệ thụ động thấp là một nhà tạo lập đang lỗ, dù bảng lãi lỗ có nói gì đi nữa.

### 5. Bất đối xứng khớp và phòng vệ theo khối lượng đã khớp

Chênh lệch giá giữa hai sàn nghe như "giao dịch phi rủi ro". Trong một hệ thống thật nó không phải vậy, vì hai chân không khớp giống nhau:

| | Chân sàn truyền thống | Chân bể AMM |
|---|---|---|
| Có thể bị từ chối | Có (sổ đã đổi) | Không |
| Khớp một phần | Có | Không |
| Xếp hàng | Có | Không |

Đặt cứng cả hai chân cùng lúc vẫn hỏng: chân AMM khớp đủ, chân sổ lệnh khớp một phần, và phần chênh đọng lại thành vị thế ròng. Trong chương này, cách làm đó để lại vị thế 111 đơn vị trên một chiến lược đáng lẽ trung tính.

Cách của ngành — và cách chương này cài — là **chạy chân không chắc trước, rồi phòng vệ đúng bằng khối lượng thực sự khớp được**. Sau khi sửa, vị thế ròng của chiến lược chênh lệch là **đúng 0**, và đó là một bài kiểm thử chứ không phải một lời hứa.

Một chi tiết kèm theo: chân phòng vệ phải được ghi sổ ở **giá thực nhận trên bể**, không phải giá của sàn kia. Ghi sai chỗ này khiến chênh lệch thu được luôn bằng 0 và chiến lược "phi rủi ro" chỉ còn lại chi phí. Muốn phòng vệ đúng khối lượng thì cần công thức **nghịch đảo** của bể: cần bỏ vào bao nhiêu để nhận đúng ngần này? Mọi giao dịch trên bể — cả chân phòng vệ lẫn lệnh `Intent::Place` gửi thẳng tới `Venue::Chain` — đi qua một hàm duy nhất `trade_on_chain`. Bản trước chỉ chân phòng vệ làm đúng; lệnh mua gửi thẳng lên bể đưa `quantity` vào làm số **Y**, nên "mua 10 X" thực ra chỉ nhận được một phần nghìn X trong khi sổ sách ghi đã mua 10 X — một lỗi đơn vị mà không bài kiểm thử nào bắt được vì không chiến lược nào gửi lệnh như vậy.

Hai chi tiết nữa của sàn truyền thống, cũng chỉ lộ ra khi ghép: phần chủ động của lệnh ta phải tiêu đúng **từng lệnh thị trường** ở mức giá đó (nếu chỉ trừ con số tổng, bản tin huỷ của lệnh đã bị ta ăn sẽ trừ mức giá lần thứ hai), và ta không bao giờ được **tự khớp** với lệnh của chính mình ở chiều kia.

### 6. Vì sao con số lãi lỗ trong chương này không đáng tin

Phiên dùng ở đây là **tổng hợp**, và mối liên kết giữa hai sàn chỉ được mô phỏng một phần. Bộ sinh phiên có một cơ chế kéo giá bể về sát sàn truyền thống — đại diện cho những nhà chênh lệch khác — nhưng cơ chế đó không hoàn hảo. Khe hở còn lại là quà tặng cho chiến lược chênh lệch.

Trên thị trường thật, hàng trăm hãng cùng săn đúng khe hở đó trong vài trăm nanosecond, và nó đóng lại trước khi bạn kịp thấy.

Không có cơ chế kéo về đó, mô hình còn tệ hơn nhiều: bể trôi tự do xuống 9305 trong khi sổ lệnh đứng ở 10000, và chiến lược chênh lệch **in ra 35 triệu** — một con số hoàn toàn giả mà nhìn qua rất thuyết phục.

Đó là bài học cuối và quan trọng nhất: **một backtest có thể đúng về mặt cơ học mà vẫn sai hoàn toàn về mặt kinh tế**, nếu môi trường mô phỏng thiếu một lực mà thị trường thật có. Thứ đáng tin ở chương này là các **bất biến**, không phải lãi lỗ.

---

## Mã nguồn minh họa thực chiến

Chạy bằng `cargo run -p ch85`, kiểm thử bằng `cargo test -p ch85` (53 bài kiểm thử).

```rust
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
//!    (sổ lệnh giá–thời gian)   (bể AMM x·y=k)
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
//!            sàn khớp ──► lãi lỗ, tồn kho, đo lường
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
    pub fn sign(self) -> i64 {
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
    pub fn wall_delay(&self, virtual_gap_ns: Nanos) -> Nanos {
        match self {
            ReplaySpeed::RealTime => virtual_gap_ns,
            ReplaySpeed::Fast(n) => virtual_gap_ns / (*n).max(1) as u64,
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
        LatencyModel {
            inbound_ns: 10_000,
            outbound_ns: 50_000,
            jitter_ns: 5_000,
        }
    }
    pub fn none() -> Self {
        LatencyModel {
            inbound_ns: 0,
            outbound_ns: 0,
            jitter_ns: 0,
        }
    }

    /// Dao động TẤT ĐỊNH theo hạt giống — cần nhiễu thật, nhưng phải tái lập được.
    pub fn order_latency(&self, seed: u64) -> Nanos {
        if self.jitter_ns == 0 {
            return self.outbound_ns;
        }
        self.outbound_ns + hash_in_range(seed, self.jitter_ns)
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

pub fn hash_in_range(seed: u64, bound: u64) -> u64 {
    if bound == 0 { 0 } else { hash64(seed) % bound }
}

// ============================================================================
// 4. SỰ KIỆN PHIÊN
// ============================================================================

#[derive(Debug, Clone, PartialEq)]
pub enum EventKind {
    /// Sàn truyền thống: một lệnh giới hạn mới vào sổ.
    AddOrder {
        id: OrderId,
        side: Side,
        price: Price,
        quantity: Quantity,
    },
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
    pub venue: Venue,
    pub kind: EventKind,
}

/// Phiên đã ghi. Bất biến sống còn: `timestamp` không giảm.
#[derive(Debug, Clone, Default)]
pub struct RecordedSession {
    pub events: Vec<SessionEvent>,
}

impl RecordedSession {
    pub fn new() -> Self {
        RecordedSession::default()
    }

    /// Từ chối sự kiện lùi thời gian thay vì im lặng sắp xếp lại — dữ liệu
    /// xếp sai thứ tự là lỗi thu thập, và giấu nó đi thì phát lại sẽ nói dối.
    pub fn record(&mut self, event: SessionEvent) -> bool {
        if let Some(last) = self.events.last()
            && event.timestamp < last.timestamp
        {
            return false;
        }
        self.events.push(event);
        true
    }

    pub fn event_count(&self) -> usize {
        self.events.len()
    }

    pub fn span_ns(&self) -> Nanos {
        match (self.events.first(), self.events.last()) {
            (Some(a), Some(b)) => b.timestamp - a.timestamp,
            _ => 0,
        }
    }

    pub fn is_ordered(&self) -> bool {
        self.events
            .windows(2)
            .all(|w| w[0].timestamp <= w[1].timestamp)
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
    bids: BTreeMap<Price, Quantity>,
    asks: BTreeMap<Price, Quantity>,
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
    pub queue_ahead: Quantity,
}

impl LitVenue {
    pub fn new() -> Self {
        LitVenue::default()
    }

    pub fn best_bid(&self) -> Option<PriceLevel> {
        self.bids.iter().next_back().map(|(&g, &k)| PriceLevel {
            price: g,
            quantity: k,
        })
    }
    pub fn best_ask(&self) -> Option<PriceLevel> {
        self.asks.iter().next().map(|(&g, &k)| PriceLevel {
            price: g,
            quantity: k,
        })
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
        let total = (m.quantity + b.quantity) as f64;
        if total <= 0.0 {
            return None;
        }
        Some((m.price as f64 * b.quantity as f64 + b.price as f64 * m.quantity as f64) / total)
    }

    pub fn imbalance(&self) -> Option<f64> {
        let (m, b) = (self.best_bid()?, self.best_ask()?);
        let total = (m.quantity + b.quantity) as f64;
        if total <= 0.0 {
            return None;
        }
        Some((m.quantity - b.quantity) as f64 / total)
    }

    pub fn spread(&self) -> Option<Price> {
        Some(self.best_ask()?.price - self.best_bid()?.price)
    }

    /// Tổng khối lượng còn treo cả hai bên — thước đo độ "phình" của sổ.
    pub fn total_qty(&self) -> Quantity {
        self.bids.values().sum::<Quantity>() + self.asks.values().sum::<Quantity>()
    }

    pub fn is_crossed(&self) -> bool {
        match (self.best_bid(), self.best_ask()) {
            (Some(m), Some(b)) => m.price >= b.price,
            _ => false,
        }
    }

    fn side_map(&mut self, c: Side) -> &mut BTreeMap<Price, Quantity> {
        match c {
            Side::Buy => &mut self.bids,
            Side::Sell => &mut self.asks,
        }
    }

    fn add(&mut self, c: Side, g: Price, k: Quantity) {
        if k <= 0 {
            return;
        }
        *self.side_map(c).entry(g).or_insert(0) += k;
    }

    fn reduce(&mut self, c: Side, g: Price, k: Quantity) {
        let side_map = self.side_map(c);
        if let Some(v) = side_map.get_mut(&g) {
            *v -= k;
            if *v <= 0 {
                side_map.remove(&g);
            }
        }
    }

    /// Khối lượng đứng trước một mức giá ở cùng chiều — vị trí xếp hàng.
    pub fn qty_at(&self, c: Side, g: Price) -> Quantity {
        match c {
            Side::Buy => self.bids.get(&g).copied().unwrap_or(0),
            Side::Sell => self.asks.get(&g).copied().unwrap_or(0),
        }
    }

    /// Tiêu thụ `wanted` đơn vị của lệnh THỊ TRƯỜNG tại (chiều, giá), theo FIFO.
    /// Trả về số thực sự tiêu được.
    fn consume_market(&mut self, c: Side, g: Price, mut wanted: Quantity) -> Quantity {
        let mut taken = 0;
        let mut emptied = Vec::new();
        if let Some(q) = self.market_queues.get(&(c, g)) {
            for &m in q.iter() {
                if wanted <= 0 {
                    break;
                }
                let left = match self.market_orders.get(&m) {
                    Some(&(_, _, k)) => k,
                    None => continue,
                };
                let take = wanted.min(left);
                wanted -= take;
                taken += take;
                if let Some(v) = self.market_orders.get_mut(&m) {
                    v.2 -= take;
                    if v.2 <= 0 {
                        emptied.push(m);
                    }
                }
            }
        }
        for m in &emptied {
            self.market_orders.remove(m);
        }
        if let Some(q) = self.market_queues.get_mut(&(c, g)) {
            q.retain(|m| !emptied.contains(m));
            if q.is_empty() {
                self.market_queues.remove(&(c, g));
            }
        }
        self.reduce(c, g, taken);
        taken
    }

    /// Áp dụng một sự kiện thị trường. Bộ phát lại phải xử lý **mọi** loại —
    /// bỏ sót lệnh huỷ khiến sổ chỉ lớn lên rồi chéo vĩnh viễn.
    ///
    /// Lệnh mới cắt qua bên kia được KHỚP, không được chất lên sổ: một sàn thật
    /// không bao giờ để sổ chéo, và mô hình bỏ qua điều này sẽ cho chiến lược
    /// nhìn thấy những mức giá không tồn tại.
    pub fn apply(&mut self, event: &EventKind) {
        match event {
            EventKind::AddOrder {
                id,
                side,
                price,
                quantity,
            } => {
                let mut left = *quantity;

                // Giai đoạn 1: khớp phần cắt qua với bên đối ứng.
                loop {
                    if left <= 0 {
                        break;
                    }
                    let best_contra = match side {
                        Side::Buy => self.asks.keys().next().copied(),
                        Side::Sell => self.bids.keys().next_back().copied(),
                    };
                    let Some(g) = best_contra else { break };
                    let crosses = match side {
                        Side::Buy => *price >= g,
                        Side::Sell => *price <= g,
                    };
                    if !crosses {
                        break;
                    }
                    // Lệnh của TA ở mức này cũng được khớp — đúng ưu tiên giá.
                    let ours: Vec<OrderId> = self
                        .our_orders
                        .values()
                        .filter(|l| l.side == side.inverse() && l.price == g)
                        .map(|l| l.id)
                        .collect();
                    let taken = self.consume_market(side.inverse(), g, left);
                    left -= taken;
                    if taken == 0 && !ours.is_empty() {
                        // Chỉ còn lệnh của ta ở mức này. Khớp ĐÚNG mức đó và
                        // ĐÚNG khối lượng còn lại — gọi hàm khớp toàn sổ ở đây
                        // sẽ ăn cả lệnh của ta ở những mức giá khác, và vị thế
                        // sẽ vọt qua hạn mức mà cổng rủi ro không hề biết.
                        let fill = self.fill_ours_at_level(side.inverse(), g, left);
                        let taken: Quantity = fill.iter().map(|x| x.quantity).sum();
                        self.pending_passive_fills.extend(fill);
                        left -= taken;
                        if taken == 0 {
                            break;
                        }
                    } else if taken == 0 {
                        break;
                    }
                }

                // Giai đoạn 2: phần còn lại nằm chờ trên sổ.
                if left > 0 {
                    self.add(*side, *price, left);
                    self.market_orders.insert(*id, (*side, *price, left));
                    self.market_queues
                        .entry((*side, *price))
                        .or_default()
                        .push_back(*id);
                }
            }
            EventKind::CancelOrder { id } => {
                if let Some((c, g, k)) = self.market_orders.remove(id) {
                    self.reduce(c, g, k);
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

    /// Khớp lệnh của ta tại ĐÚNG một mức giá, không vượt quá `bound` đơn vị.
    /// Ưu tiên thời gian trong nội bộ mức.
    fn fill_ours_at_level(&mut self, side: Side, price: Price, bound: Quantity) -> Vec<Fill> {
        let mut out = Vec::new();
        let mut left = bound;
        let mut candidates: Vec<OurOrder> = self
            .our_orders
            .values()
            .copied()
            .filter(|l| l.side == side && l.price == price)
            .collect();
        candidates.sort_by_key(|l| (l.entered_at, l.id));

        let mut done = Vec::new();
        for l in candidates {
            if left <= 0 {
                break;
            }
            let take = left.min(l.remaining);
            if let Some(m) = self.our_orders.get_mut(&l.id) {
                m.remaining -= take;
                if m.remaining <= 0 {
                    done.push(l.id);
                }
            }
            self.reduce(side, price, take);
            out.push(Fill {
                id: l.id,
                side,
                price,
                quantity: take,
                aggressive: false,
            });
            left -= take;
        }
        for m in done {
            self.our_orders.remove(&m);
        }
        out
    }

    /// Lệnh treo của ta cũ hơn `max_age_ns` — nhà tạo lập thật làm mới báo giá
    /// liên tục, và báo giá cũ là rủi ro chứ không phải cơ hội.
    pub fn our_orders_older_than(&self, now: Nanos, max_age_ns: Nanos) -> Vec<OrderId> {
        self.our_orders
            .values()
            .filter(|l| now.saturating_sub(l.entered_at) > max_age_ns)
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
    ///
    /// Phần chủ động phải tiêu đúng các LỆNH THỊ TRƯỜNG ở mức đó (theo FIFO),
    /// không chỉ trừ con số tổng hợp của mức giá: nếu chỉ trừ con số tổng, lệnh
    /// thị trường đã bị ta ăn vẫn "sống" trong `market_orders`, và khi bản tin
    /// huỷ của nó tới, mức giá bị trừ LẦN THỨ HAI — sổ lệch dần khỏi thực tế.
    ///
    /// Nếu ở mức giá cắt qua chỉ còn lệnh của CHÍNH TA ở chiều kia, ta không tự
    /// khớp với mình (chống tự khớp — self-trade prevention), và phần còn lại
    /// của lệnh mới bị huỷ thay vì nằm chờ làm sổ chéo.
    pub fn place_our_order(&mut self, l: OurOrder) -> Vec<Fill> {
        let mut fill = Vec::new();
        let mut left = l.remaining;

        // Lệnh chủ động: ăn qua các mức đối ứng theo thứ tự giá tốt nhất trước.
        loop {
            if left <= 0 {
                break;
            }
            let best_contra = match l.side {
                Side::Buy => self.asks.keys().next().copied(),
                Side::Sell => self.bids.keys().next_back().copied(),
            };
            let Some(g) = best_contra else { break };
            let crosses = match l.side {
                Side::Buy => l.price >= g,
                Side::Sell => l.price <= g,
            };
            if !crosses {
                break;
            }
            let take = self.consume_market(l.side.inverse(), g, left);
            if take == 0 {
                // Chỉ còn lệnh của ta ở mức này → chống tự khớp: bỏ phần còn lại.
                return fill;
            }
            fill.push(Fill {
                id: l.id,
                side: l.side,
                price: g,
                quantity: take,
                aggressive: true,
            });
            left -= take;
        }

        if left > 0 {
            let ahead = self.qty_at(l.side, l.price);
            self.add(l.side, l.price, left);
            self.our_orders.insert(
                l.id,
                OurOrder {
                    remaining: left,
                    queue_ahead: ahead,
                    ..l
                },
            );
        }
        fill
    }

    pub fn cancel_our_order(&mut self, id: OrderId) -> bool {
        match self.our_orders.remove(&id) {
            Some(l) => {
                self.reduce(l.side, l.price, l.remaining);
                true
            }
            None => false,
        }
    }

    pub fn our_order(&self, id: OrderId) -> Option<&OurOrder> {
        self.our_orders.get(&id)
    }

    pub fn our_resting_orders(&self) -> Vec<OurOrder> {
        self.our_orders.values().copied().collect()
    }

    /// Khi thị trường khớp ở giá `g`, lệnh treo của ta ở giá tốt bằng hoặc hơn
    /// sẽ được khớp — nhưng chỉ sau khi hàng đứng trước đã tiêu hết.
    pub fn on_market_trade(&mut self, price: Price, mut quantity: Quantity) -> Vec<Fill> {
        let mut out = Vec::new();
        let mut done_ids = Vec::new();

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
            let beyond_queue = quantity - l.queue_ahead;
            if beyond_queue <= 0 {
                if let Some(m) = self.our_orders.get_mut(&l.id) {
                    m.queue_ahead -= quantity;
                }
                break;
            }
            let take = beyond_queue.min(l.remaining);
            if let Some(m) = self.our_orders.get_mut(&l.id) {
                m.queue_ahead = 0;
                m.remaining -= take;
                if m.remaining <= 0 {
                    done_ids.push(l.id);
                }
            }
            self.reduce(l.side, l.price, take);
            out.push(Fill {
                id: l.id,
                side: l.side,
                price: l.price,
                quantity: take,
                aggressive: false,
            });
            quantity -= take + l.queue_ahead;
        }
        for m in done_ids {
            self.our_orders.remove(&m);
        }
        out
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
    BelowMinOut { received: u128, required: u128 },
}

impl ChainVenue {
    pub fn new(x: u128, y: u128, fee_bps: u32) -> Self {
        ChainVenue {
            reserve_x: x,
            reserve_y: y,
            fee_bps,
        }
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
        let (reserve_in, reserve_out) = if x_in {
            (self.reserve_x, self.reserve_y)
        } else {
            (self.reserve_y, self.reserve_x)
        };
        let after_fee = amount_in * (10_000 - self.fee_bps as u128);
        // Làm tròn LUÔN có lợi cho bể — đó là chủ ý, không phải cẩu thả.
        Ok((after_fee * reserve_out) / (reserve_in * 10_000 + after_fee))
    }

    pub fn swap(&mut self, x_in: bool, amount_in: u128, min_out: u128) -> Result<u128, SwapError> {
        let out = self.try_swap(x_in, amount_in)?;
        if out < min_out {
            return Err(SwapError::BelowMinOut {
                received: out,
                required: min_out,
            });
        }
        if x_in {
            self.reserve_x += amount_in;
            self.reserve_y -= out;
        } else {
            self.reserve_y += amount_in;
            self.reserve_x -= out;
        }
        Ok(out)
    }

    /// Giá trung bình thực nhận — luôn tệ hơn `price_x()`. Đây mới là con số
    /// dùng để so với sàn truyền thống khi tìm chênh lệch.
    pub fn effective_price(&self, x_in: bool, amount_in: u128) -> Option<f64> {
        let out = self.try_swap(x_in, amount_in).ok()?;
        if out == 0 {
            return None;
        }
        Some(if x_in {
            out as f64 / amount_in as f64
        } else {
            amount_in as f64 / out as f64
        })
    }

    /// Nghịch đảo của `try_swap`: cần bỏ vào bao nhiêu để nhận ĐÚNG `ra`?
    /// Cần thiết cho phòng vệ chính xác — không có nó, chân phòng vệ lệch khối
    /// lượng và vị thế ròng không bao giờ về 0.
    pub fn input_for_output(&self, x_in: bool, desired_out: u128) -> Option<u128> {
        if desired_out == 0 {
            return None;
        }
        let (reserve_in, reserve_out) = if x_in {
            (self.reserve_x, self.reserve_y)
        } else {
            (self.reserve_y, self.reserve_x)
        };
        if desired_out >= reserve_out {
            return None; // không thể rút hết một phía
        }
        let numerator = reserve_in * desired_out * 10_000;
        let denominator = (reserve_out - desired_out) * (10_000 - self.fee_bps as u128);
        Some(numerator / denominator + 1) // +1: làm tròn LÊN, luôn có lợi cho bể
    }

    pub fn apply(&mut self, event: &EventKind) {
        if let EventKind::PoolSwap { x_in, quantity } = event {
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
    pub lit_bid: Option<PriceLevel>,
    pub lit_ask: Option<PriceLevel>,
    pub lit_micro_price: Option<f64>,
    pub lit_imbalance: Option<f64>,
    pub chain_price: f64,
    pub chain_reserve_x: u128,
    pub chain_reserve_y: u128,
}

impl MarketSnapshot {
    pub fn lit_mid(&self) -> Option<f64> {
        match (self.lit_bid, self.lit_ask) {
            (Some(m), Some(b)) => Some((m.price + b.price) as f64 / 2.0),
            _ => None,
        }
    }

    /// Chênh lệch giá giữa hai sàn, tính bằng điểm cơ bản. Dương nghĩa là
    /// sàn chuỗi khối đang đắt hơn → mua truyền thống, bán chuỗi khối.
    pub fn cross_venue_bps(&self) -> Option<f64> {
        let mid = self.lit_mid()?;
        if mid <= 0.0 {
            return None;
        }
        Some((self.chain_price - mid) / mid * 10_000.0)
    }
}

// ============================================================================
// 8. Ý ĐỊNH GIAO DỊCH & CỔNG RỦI RO
// ============================================================================

#[derive(Debug, Clone, Copy, PartialEq)]
pub enum Intent {
    Place {
        venue: Venue,
        side: Side,
        price: Price,
        quantity: Quantity,
    },
    CancelOrder {
        venue: Venue,
        id: OrderId,
    },
    /// Lệnh chính kèm **phòng vệ theo khối lượng đã khớp** trên sàn còn lại.
    ///
    /// Đặt cứng cả hai chân cùng lúc nghe có vẻ đúng nhưng vẫn hỏng: chân AMM
    /// luôn khớp đủ (công thức không bao giờ từ chối), còn chân sổ lệnh chỉ khớp
    /// một phần vì sổ đã đổi trong khoảng độ trễ. Chênh lệch đó đọng lại thành
    /// vị thế ròng — **bất đối xứng khớp**.
    ///
    /// Cách làm của ngành: thực thi chân KHÔNG CHẮC trước, rồi phòng vệ đúng
    /// bằng khối lượng thực sự khớp được.
    PlaceHedged {
        venue: Venue,
        side: Side,
        price: Price,
        quantity: Quantity,
        hedge_on: Venue,
    },
}

impl Intent {
    pub fn place(venue: Venue, side: Side, price: Price, quantity: Quantity) -> Self {
        Intent::Place {
            venue,
            side,
            price,
            quantity,
        }
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
        let d = side.sign() * quantity;

        if prev == 0 || prev.signum() == d.signum() {
            // Mở rộng cùng chiều: cập nhật giá vốn trung bình có trọng số.
            let total = (prev.abs() + quantity) as f64;
            if total > 0.0 {
                self.cost_basis =
                    (self.cost_basis * prev.abs() as f64 + price as f64 * quantity as f64) / total;
            }
            self.quantity = prev + d;
        } else {
            let closed = quantity.min(prev.abs());
            self.realized_pnl +=
                (price as f64 - self.cost_basis) * closed as f64 * prev.signum() as f64;
            self.quantity = prev + d;
            if self.quantity.signum() != prev.signum() && self.quantity != 0 {
                // Đảo chiều: phần dư mở vị thế mới ở đúng giá này.
                self.cost_basis = price as f64;
            } else if self.quantity == 0 {
                self.cost_basis = 0.0;
            }
        }
    }

    pub fn unrealized_pnl(&self, mark_price: f64) -> f64 {
        (mark_price - self.cost_basis) * self.quantity as f64
    }

    pub fn total_pnl(&self, mark_price: f64) -> f64 {
        self.realized_pnl + self.unrealized_pnl(mark_price)
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
            Intent::Place {
                side,
                price,
                quantity,
                ..
            } => (*side, *price, *quantity),
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
        let (after_buy, after_sell) = match side {
            Side::Buy => (position + resting_bid + quantity, position - resting_ask),
            Side::Sell => (position + resting_bid, position - resting_ask - quantity),
        };
        if after_buy.abs() > self.max_position || after_sell.abs() > self.max_position {
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
        "managed_maker"
    }

    fn evaluate(&mut self, snap: &MarketSnapshot, position: Quantity) -> Vec<Intent> {
        if snap.timestamp.saturating_sub(self.last_quote_at) < self.quote_interval_ns {
            return Vec::new();
        }
        let mid = match snap.lit_micro_price.or_else(|| snap.lit_mid()) {
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

        let mut bid_price = (mid - half - skew).round() as Price;
        let mut ask_price = (mid + half - skew).round() as Price;

        // KHÔNG BAO GIỜ cắt qua sổ. Một nhà tạo lập cắt giá sẽ TRẢ chênh lệch
        // thay vì THU nó — nó trở thành người chủ động, và toàn bộ mô hình kinh
        // doanh sụp đổ. Đây là ràng buộc, không phải tối ưu hoá.
        if let Some(b) = snap.lit_ask {
            bid_price = bid_price.min(b.price - 1);
        }
        if let Some(m) = snap.lit_bid {
            ask_price = ask_price.max(m.price + 1);
        }
        if bid_price <= 0 || ask_price <= bid_price {
            return Vec::new();
        }

        let mut out = Vec::new();
        // Chỉ báo giá bên nào chưa chạm hạn mức — hàng phòng vệ thứ nhất,
        // trước cả cổng rủi ro.
        if position < self.inventory_limit {
            out.push(Intent::Place {
                venue: Venue::Lit,
                side: Side::Buy,
                price: bid_price,
                quantity: self.quantity,
            });
        }
        if position > -self.inventory_limit {
            out.push(Intent::Place {
                venue: Venue::Lit,
                side: Side::Sell,
                price: ask_price,
                quantity: self.quantity,
            });
        }
        out
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
        CrossVenueArb {
            threshold_bps,
            quantity: 10,
            opportunities_seen: 0,
        }
    }
}

impl Strategy for CrossVenueArb {
    fn name(&self) -> &str {
        "cross_venue_arb"
    }

    fn evaluate(&mut self, snap: &MarketSnapshot, _position: Quantity) -> Vec<Intent> {
        let strategy = match snap.cross_venue_bps() {
            Some(x) => x,
            None => return Vec::new(),
        };
        if strategy.abs() < self.threshold_bps {
            return Vec::new();
        }
        self.opportunities_seen += 1;

        // Chênh lệch giá là giao dịch HAI CHÂN. Chỉ đặt một chân thì đó không
        // phải chênh lệch giá — đó là cược một chiều đội lốt, và nó sẽ tích luỹ
        // vị thế cho tới khi chạm hạn mức rồi ngồi đó chịu lỗ.
        let (lit_side, level) = if strategy > 0.0 {
            // Chuỗi khối đắt hơn → mua chân rẻ (truyền thống), bán chân đắt.
            (Side::Buy, snap.lit_ask)
        } else {
            (Side::Sell, snap.lit_bid)
        };
        let m = match level {
            Some(m) => m,
            None => return Vec::new(),
        };
        let qty = self.quantity.min(m.quantity);
        if qty <= 0 {
            return Vec::new();
        }
        // Chân truyền thống là chân KHÔNG CHẮC (phải xếp hàng, sổ có thể đã đổi).
        // Chân chuỗi khối là phòng vệ, chỉ chạy đúng bằng phần thực sự khớp.
        vec![Intent::PlaceHedged {
            venue: Venue::Lit,
            side: lit_side,
            price: m.price,
            quantity: qty,
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
    buckets: BTreeMap<u32, u64>,
    pub samples: u64,
    pub total: u64,
    pub max: u64,
}

impl LatencyHistogram {
    pub fn new() -> Self {
        LatencyHistogram::default()
    }

    pub fn record(&mut self, ns: u64) {
        let k = if ns == 0 { 0 } else { 64 - ns.leading_zeros() };
        *self.buckets.entry(k).or_insert(0) += 1;
        self.samples += 1;
        self.total += ns;
        self.max = self.max.max(ns);
    }

    pub fn mean(&self) -> f64 {
        if self.samples == 0 {
            0.0
        } else {
            self.total as f64 / self.samples as f64
        }
    }

    /// Phân vị là con số DUY NHẤT đáng nhìn trong HFT. Trung bình chỉ hữu ích
    /// để phát hiện là mình đã đo sai. Trả CẬN TRÊN của thùng (nhưng không vượt
    /// `max`) như Chương 74 — báo thấp hơn thật thì ta tưởng hệ thống nhanh hơn.
    pub fn percentile(&self, p: f64) -> u64 {
        if self.samples == 0 {
            return 0;
        }
        let level = (self.samples as f64 * p).ceil() as u64;
        let mut cumulative = 0;
        for (&k, &c) in &self.buckets {
            cumulative += c;
            if cumulative >= level {
                let upper = if k == 0 { 0 } else { u64::MAX >> (64 - k) };
                return upper.min(self.max);
            }
        }
        self.max
    }
}

#[derive(Debug, Clone, Default)]
pub struct Metrics {
    pub signal_to_order: LatencyHistogram,
    pub intent_count: u64,
    pub orders_sent: u64,
    pub orders_blocked: u64,
    pub fill_count: u64,
    pub filled_qty: Quantity,
    pub aggressive_qty: Quantity,
    /// |vị thế| lớn nhất từng chạm trong phiên.
    pub peak_abs_position: Quantity,
    pub equity_curve: Vec<f64>,
}

impl Metrics {
    pub fn new() -> Self {
        Metrics::default()
    }

    pub fn block_ratio(&self) -> f64 {
        if self.intent_count == 0 {
            0.0
        } else {
            self.orders_blocked as f64 / self.intent_count as f64
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
            lit_bid: self.venue_lit.best_bid(),
            lit_ask: self.venue_lit.best_ask(),
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
        while self.in_flight.front().is_some_and(|l| l.arrives_at <= now) {
            let l = self.in_flight.pop_front().unwrap();
            match l.intent {
                Intent::CancelOrder {
                    venue: Venue::Lit,
                    id,
                } => {
                    self.cancel_lit_order(id);
                }
                Intent::CancelOrder { .. } => {}
                Intent::PlaceHedged { .. } => {}
                Intent::Place {
                    venue: Venue::Lit,
                    side,
                    price,
                    quantity,
                } => {
                    match side {
                        Side::Buy => self.in_flight_bid = (self.in_flight_bid - quantity).max(0),
                        Side::Sell => self.in_flight_ask = (self.in_flight_ask - quantity).max(0),
                    }
                    let fills = self.venue_lit.place_our_order(OurOrder {
                        id: l.id,
                        side,
                        price,
                        remaining: quantity,
                        entered_at: l.arrives_at,
                        queue_ahead: 0,
                    });
                    // Phơi nhiễm treo = phần thực sự nằm lại trên sổ. Cộng thêm
                    // phần vừa khớp vì `apply_fill` sẽ trừ nó ra ngay sau đây.
                    // (Cộng cả `quantity` là sai: phần bị huỷ do chống tự khớp sẽ
                    // thành phơi nhiễm "ma" không bao giờ giảm.)
                    let rested = self.venue_lit.our_order(l.id).map_or(0, |o| o.remaining);
                    let filled: Quantity = fills.iter().map(|f| f.quantity).sum();
                    match side {
                        Side::Buy => self.resting_bid += rested + filled,
                        Side::Sell => self.resting_ask += rested + filled,
                    }
                    self.metrics
                        .signal_to_order
                        .record(l.arrives_at - l.sent_at);
                    self.order_log
                        .push((l.arrives_at, Venue::Lit, side, price, quantity));
                    for k in fills {
                        self.apply_fill(k, Venue::Lit);
                    }
                }
                Intent::Place {
                    venue: Venue::Chain,
                    side,
                    price,
                    quantity,
                } => {
                    // Sàn AMM khớp tức thì theo công thức — không xếp hàng, nhưng
                    // vẫn phải chịu độ trễ tới lượt được đưa vào khối.
                    match side {
                        Side::Buy => self.in_flight_bid = (self.in_flight_bid - quantity).max(0),
                        Side::Sell => self.in_flight_ask = (self.in_flight_ask - quantity).max(0),
                    }
                    // Giá của ý định là mức tệ nhất chấp nhận được (giới hạn trượt giá).
                    if let Some(px) = self.trade_on_chain(side, quantity, Some(price)) {
                        self.metrics
                            .signal_to_order
                            .record(l.arrives_at - l.sent_at);
                        self.order_log
                            .push((l.arrives_at, Venue::Chain, side, px, quantity));
                        self.apply_fill(
                            Fill {
                                id: l.id,
                                side,
                                price: px,
                                quantity,
                                aggressive: true,
                            },
                            Venue::Chain,
                        );
                    }
                }
            }
        }
    }

    /// Mua/bán ĐÚNG `quantity` đơn vị X trên bể; trả giá thực nhận (Y mỗi X).
    ///
    /// Bán X: bỏ vào `quantity` X. Mua X: phải tính NGƯỢC xem cần bỏ vào bao
    /// nhiêu Y để nhận đúng `quantity` X — đưa thẳng `quantity` vào làm số Y là
    /// lỗi đơn vị (bỏ vào 10 Y thì chỉ nhận được một phần nhỏ của 1 X, trong
    /// khi sổ sách lại ghi đã mua 10 X).
    ///
    /// `limit` (nếu có) là giá tệ nhất chấp nhận được: vượt quá thì không giao dịch.
    fn trade_on_chain(
        &mut self,
        side: Side,
        quantity: Quantity,
        limit: Option<Price>,
    ) -> Option<Price> {
        if quantity <= 0 {
            return None;
        }
        let qty = quantity as u128;
        let (x_in, amount_in) = match side {
            Side::Sell => (true, qty),
            Side::Buy => (false, self.venue_chain.input_for_output(false, qty)?),
        };
        let out = self.venue_chain.try_swap(x_in, amount_in).ok()?;
        let price = match side {
            Side::Sell => out as f64 / qty as f64, // nhận `out` Y cho qty X
            Side::Buy => amount_in as f64 / qty as f64, // trả `amount_in` Y cho qty X
        };
        let px = price.round().max(1.0) as Price;
        let acceptable = match (side, limit) {
            (_, None) => true,
            (Side::Buy, Some(lim)) => px <= lim,
            (Side::Sell, Some(lim)) => px >= lim,
        };
        if !acceptable {
            return None;
        }
        self.venue_chain.swap(x_in, amount_in, 0).ok()?;
        Some(px)
    }

    fn cancel_lit_order(&mut self, id: OrderId) {
        if let Some(o) = self.venue_lit.our_order(id).copied() {
            match o.side {
                Side::Buy => self.resting_bid = (self.resting_bid - o.remaining).max(0),
                Side::Sell => self.resting_ask = (self.resting_ask - o.remaining).max(0),
            }
            self.venue_lit.cancel_our_order(id);
        }
    }

    fn apply_fill(&mut self, k: Fill, venue: Venue) {
        self.position.record(k.side, k.price, k.quantity);

        // Phòng vệ NGAY, đúng bằng khối lượng vừa khớp. Đây là chỗ bất đối xứng
        // khớp được triệt tiêu: chân chắc chắn chỉ chạy sau khi chân không chắc
        // đã cho biết nó khớp được bao nhiêu.
        if let Some(&hedge_venue) = self.pending_hedge.get(&k.id)
            && hedge_venue == Venue::Chain
            && k.quantity > 0
        {
            let inverse = k.side.inverse();
            // Giá phải là giá THỰC NHẬN trên bể, không phải giá của sàn kia.
            // Ghi sổ chân phòng vệ ở giá sàn truyền thống khiến chênh lệch
            // thu được luôn bằng 0 — chiến lược "phi rủi ro" chỉ còn chi phí.
            // Phòng vệ thì không đặt giới hạn giá: nó phải chạy.
            if let Some(px) = self.trade_on_chain(inverse, k.quantity, None) {
                self.position.record(inverse, px, k.quantity);
                self.hedge_count += 1;
                self.order_log
                    .push((self.clock.now(), Venue::Chain, inverse, px, k.quantity));
            }
        }

        self.metrics.fill_count += 1;
        self.metrics.filled_qty += k.quantity;
        if k.aggressive {
            self.metrics.aggressive_qty += k.quantity;
        }
        // Chỉ lệnh trên sổ mới từng "treo"; lệnh AMM khớp ngay khi tới nơi.
        if venue == Venue::Lit {
            match k.side {
                Side::Buy => self.resting_bid = (self.resting_bid - k.quantity).max(0),
                Side::Sell => self.resting_ask = (self.resting_ask - k.quantity).max(0),
            }
        }
        // Bất biến rủi ro phải đúng ở MỌI thời điểm, không chỉ cuối phiên.
        self.metrics.peak_abs_position = self
            .metrics
            .peak_abs_position
            .max(self.position.quantity.abs());
    }

    /// Đưa các ý định qua cổng rủi ro rồi xếp vào hàng đợi bay.
    pub fn publish(&mut self, intents: Vec<Intent>) {
        let now = self.clock.now();
        let pnl = self.position.total_pnl(self.reference_price());

        for y in intents {
            self.metrics.intent_count += 1;

            // --- lệnh chính + phòng vệ theo khối lượng đã khớp ---
            if let Intent::PlaceHedged {
                venue,
                side,
                price,
                quantity,
                hedge_on,
            } = y
            {
                let leg = Intent::place(venue, side, price, quantity);
                if self
                    .gate
                    .check(
                        &leg,
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
                    intent: leg,
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
    pub fn run(&mut self, session: &RecordedSession, strategies: &mut [Box<dyn Strategy>]) {
        for event in &session.events {
            // 1. Thời gian tiến tới thời điểm sự kiện.
            if !self.clock.advance(event.timestamp) {
                continue;
            }
            // 2. Giao mọi lệnh đã tới nơi TRƯỚC sự kiện này.
            self.deliver_due();

            // 2b. Rút báo giá đã quá cũ. Huỷ đi thẳng, không qua độ trễ gửi:
            // sàn thật xử lý huỷ trên đường ưu tiên, và quan trọng hơn — nếu
            // huỷ cũng phải xếp hàng thì rủi ro tồn kho không bao giờ giảm được.
            let stale = self
                .venue_lit
                .our_orders_older_than(self.clock.now(), self.max_quote_age_ns);
            for id in stale {
                self.cancel_lit_order(id);
            }

            // 3. Áp dụng sự kiện lên đúng sàn của nó.
            match event.venue {
                Venue::Lit => {
                    self.venue_lit.apply(&event.kind);
                    // Lệnh thị trường cắt qua lệnh treo của ta → khớp thụ động.
                    for k in self.venue_lit.take_passive_fills() {
                        self.apply_fill(k, Venue::Lit);
                    }
                    if let EventKind::Traded { price, quantity } = event.kind {
                        for k in self.venue_lit.on_market_trade(price, quantity) {
                            self.apply_fill(k, Venue::Lit);
                        }
                    }
                }
                Venue::Chain => self.venue_chain.apply(&event.kind),
            }

            // 4. Chiến lược nhìn ảnh chụp SAU sự kiện — và chỉ ảnh chụp.
            let snap = self.snapshot();
            let mut intent = Vec::new();
            for strategy in strategies.iter_mut() {
                intent.extend(strategy.evaluate(&snap, self.position.quantity));
            }
            self.publish(intent);

            self.metrics
                .equity_curve
                .push(self.position.total_pnl(self.reference_price()));
        }
        // Xả nốt các lệnh còn đang bay.
        self.clock.advance(self.clock.now() + 1_000_000_000);
        self.deliver_due();
    }
}

// ============================================================================
// 12. BỘ SINH PHIÊN TỔNG HỢP
// ============================================================================

/// Bể khởi đầu: 2 triệu X, 20 tỉ Y (giá 10 000), phí 0,30%.
pub const INITIAL_POOL: (u128, u128, u32) = (2_000_000, 20_000_000_000, 30);

/// Sinh một phiên hai sàn tất định. Hai chi tiết quan trọng, cả hai đều là
/// bài học rút ra từ lỗi thật: huỷ **lệnh sống cũ nhất** (không phải mã ngẫu
/// nhiên, vì phần lớn mã ngẫu nhiên đã chết), và **giới hạn số lệnh sống**
/// để sổ không phình ra rồi chéo vĩnh viễn.
pub fn generate_session(event_count: usize, seed: u64, anchor_price: Price) -> RecordedSession {
    let mut p = RecordedSession::new();
    let mut t: Nanos = 1_000_000_000;
    let mut id: OrderId = 1;
    let mut live: VecDeque<OrderId> = VecDeque::new();
    let mut current_price = anchor_price;
    // Bản sao bể để chọn CHIỀU hoán đổi. Nó đại diện cho **phần còn lại của
    // thị trường**: những nhà chênh lệch khác liên tục kéo giá bể về sát sàn
    // truyền thống. Không có lực này, bể trôi tự do và mọi chiến lược chênh
    // lệch trong mô hình sẽ in ra tiền — một kết quả hoàn toàn giả.
    let mut pool = ChainVenue::new(INITIAL_POOL.0, INITIAL_POOL.1, INITIAL_POOL.2);

    for i in 0..event_count {
        let r = hash64(seed ^ (i as u64).wrapping_mul(0x1000193));
        t += 1_000 + (r % 200_000);

        // Bước ngẫu nhiên có neo: kéo giá về `anchor_price` để chuỗi không trôi mất.
        let step = (hash64(r) % 5) as Price - 2;
        current_price = (current_price + step)
            .max(anchor_price - 40)
            .min(anchor_price + 40);

        let roll = r % 100;
        if roll < 8 {
            // Hoán đổi trên bể chuỗi khối. Chiều được chọn để KÉO giá bể về
            // phía giá sàn truyền thống, cộng thêm một phần nhiễu từ người
            // giao dịch thường.
            let gap = pool.price_x() - current_price as f64;
            let noise = hash64(r ^ 0x5A5A).is_multiple_of(5); // 20% là nhiễu thuần
            let x_in = if noise {
                (r >> 8).is_multiple_of(2)
            } else {
                gap > 0.0
            };
            let qty = 1 + (hash64(r ^ 0xABC) % 500) as u128;
            let _ = pool.swap(x_in, qty, 0);
            p.record(SessionEvent {
                timestamp: t,
                venue: Venue::Chain,
                kind: EventKind::PoolSwap {
                    x_in,
                    quantity: qty,
                },
            });
        } else if roll < 20 && !live.is_empty() {
            // Giao dịch đã khớp trên sàn truyền thống.
            let qty = 1 + (hash64(r ^ 0xDEF) % 40) as Quantity;
            p.record(SessionEvent {
                timestamp: t,
                venue: Venue::Lit,
                kind: EventKind::Traded {
                    price: current_price,
                    quantity: qty,
                },
            });
        } else if live.len() >= 120 || (roll < 55 && live.len() > 20) {
            // Huỷ lệnh SỐNG CŨ NHẤT — mô phỏng đúng hành vi nhà tạo lập thật.
            if let Some(m) = live.pop_front() {
                p.record(SessionEvent {
                    timestamp: t,
                    venue: Venue::Lit,
                    kind: EventKind::CancelOrder { id: m },
                });
            }
        } else {
            let is_buy = (r >> 16).is_multiple_of(2);
            let gap = 1 + (hash64(r ^ 0x777) % 6) as Price;
            let (side, price) = if is_buy {
                (Side::Buy, current_price - gap)
            } else {
                (Side::Sell, current_price + gap)
            };
            let qty = 10 + (hash64(r ^ 0x999) % 90) as Quantity;
            p.record(SessionEvent {
                timestamp: t,
                venue: Venue::Lit,
                kind: EventKind::AddOrder {
                    id,
                    side,
                    price,
                    quantity: qty,
                },
            });
            live.push_back(id);
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
    println!(
        "   khoảng thời gian: {:.3} giây",
        session.span_ns() as f64 / 1e9
    );
    println!("   đúng thứ tự    : {}", session.is_ordered());

    println!("\n2. PHÁT LẠI Ở NHIỀU TỐC ĐỘ — kết quả PHẢI trùng nhau");
    println!(
        "   {:<16} {:>10} {:>10} {:>12}",
        "tốc độ", "lệnh gửi", "khớp", "lãi/lỗ"
    );
    let mut first_result = None;
    for speed in [
        ReplaySpeed::Unbounded,
        ReplaySpeed::Fast(1_000),
        ReplaySpeed::RealTime,
    ] {
        let mut eco = Ecosystem::new(
            ChainVenue::new(INITIAL_POOL.0, INITIAL_POOL.1, INITIAL_POOL.2),
            LatencyModel::typical(),
            speed,
        );
        let mut strategies: Vec<Box<dyn Strategy>> = vec![
            Box::new(ManagedMaker::new(200)),
            Box::new(CrossVenueArb::new(150.0)),
        ];
        eco.run(&session, &mut strategies);
        let pnl = eco.position.total_pnl(eco.reference_price());
        let name = match speed {
            ReplaySpeed::Unbounded => "vô hạn".to_string(),
            ReplaySpeed::Fast(n) => format!("×{n}"),
            ReplaySpeed::RealTime => "thời gian thực".to_string(),
        };
        println!(
            "   {:<16} {:>10} {:>10} {:>12.1}",
            name, eco.metrics.orders_sent, eco.metrics.fill_count, pnl
        );
        let this_result = (
            eco.metrics.orders_sent,
            eco.metrics.fill_count,
            eco.order_log.len(),
        );
        match first_result {
            None => first_result = Some(this_result),
            Some(d) => assert_eq!(d, this_result, "phát lại KHÔNG tất định giữa các tốc độ"),
        }
    }

    println!("\n3. HỆ SINH THÁI ĐẦY ĐỦ — hai sàn, hai chiến lược");
    let mut eco = Ecosystem::new(
        ChainVenue::new(INITIAL_POOL.0, INITIAL_POOL.1, INITIAL_POOL.2),
        LatencyModel::typical(),
        ReplaySpeed::Unbounded,
    );
    let mut strategies: Vec<Box<dyn Strategy>> = vec![
        Box::new(ManagedMaker::new(200)),
        Box::new(CrossVenueArb::new(150.0)),
    ];
    eco.run(&session, &mut strategies);

    let m = &eco.metrics;
    println!("   ý định sinh ra     : {}", m.intent_count);
    println!("   lệnh gửi đi        : {}", m.orders_sent);
    println!(
        "   bị cổng rủi ro chặn: {} ({:.1}%)",
        m.orders_blocked,
        m.block_ratio() * 100.0
    );
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
        "   |vị thế| lớn nhất trong phiên = {} ≤ hạn mức {} : {}",
        m.peak_abs_position,
        eco.gate.max_position,
        m.peak_abs_position <= eco.gate.max_position
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
            ChainVenue::new(INITIAL_POOL.0, INITIAL_POOL.1, INITIAL_POOL.2),
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
            venue: Venue::Lit,
            kind: EventKind::Traded {
                price: 10,
                quantity: 1
            },
        }));
        assert!(!p.record(SessionEvent {
            timestamp: 50,
            venue: Venue::Lit,
            kind: EventKind::Traded {
                price: 10,
                quantity: 1
            },
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
        let lit = p.events.iter().filter(|s| s.venue == Venue::Lit).count();
        let chain = p.events.iter().filter(|s| s.venue == Venue::Chain).count();
        assert!(
            lit > 0 && chain > 0,
            "phiên phải phủ cả hai loại thị trường"
        );
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
            s.apply(&EventKind::AddOrder {
                id,
                side: c,
                price: g,
                quantity: k,
            });
        }
        assert_eq!(s.best_bid().unwrap().price, 100);
        assert_eq!(s.best_ask().unwrap().price, 101);
        assert_eq!(s.spread(), Some(1));
        assert!(!s.is_crossed());
    }

    #[test]
    fn cancel_shrinks_book() {
        let mut s = LitVenue::new();
        s.apply(&EventKind::AddOrder {
            id: 1,
            side: Side::Buy,
            price: 100,
            quantity: 50,
        });
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
        let mut with_cancels = LitVenue::new();
        let mut without_cancels = LitVenue::new();
        let p = generate_session(3_000, 42, 10_000);
        for event in &p.events {
            if event.venue != Venue::Lit {
                continue;
            }
            with_cancels.apply(&event.kind);
            if !matches!(event.kind, EventKind::CancelOrder { .. }) {
                without_cancels.apply(&event.kind);
            }
        }
        assert!(
            without_cancels.total_qty() > with_cancels.total_qty() * 2,
            "bỏ lệnh huỷ thì sổ phình lên: {} so với {}",
            without_cancels.total_qty(),
            with_cancels.total_qty()
        );
    }

    #[test]
    fn micro_price_leans_to_thin_side() {
        let mut s = LitVenue::new();
        s.apply(&EventKind::AddOrder {
            id: 1,
            side: Side::Buy,
            price: 100,
            quantity: 900,
        });
        s.apply(&EventKind::AddOrder {
            id: 2,
            side: Side::Sell,
            price: 102,
            quantity: 100,
        });
        let mid = s.mid().unwrap();
        let micro = s.micro_price().unwrap();
        assert!(micro > mid, "bên mua đông → vi giá phải cao hơn giá giữa");
        assert!(micro < 102.0);
    }

    #[test]
    fn imbalance_has_correct_sign() {
        let mut s = LitVenue::new();
        s.apply(&EventKind::AddOrder {
            id: 1,
            side: Side::Buy,
            price: 100,
            quantity: 900,
        });
        s.apply(&EventKind::AddOrder {
            id: 2,
            side: Side::Sell,
            price: 102,
            quantity: 100,
        });
        assert!((s.imbalance().unwrap() - 0.8).abs() < 1e-9);
    }

    #[test]
    fn aggressive_order_fills_immediately() {
        let mut s = LitVenue::new();
        s.apply(&EventKind::AddOrder {
            id: 1,
            side: Side::Sell,
            price: 100,
            quantity: 30,
        });
        s.apply(&EventKind::AddOrder {
            id: 2,
            side: Side::Sell,
            price: 101,
            quantity: 30,
        });
        let fill = s.place_our_order(OurOrder {
            id: 9,
            side: Side::Buy,
            price: 101,
            remaining: 50,
            entered_at: 0,
            queue_ahead: 0,
        });
        assert_eq!(fill.len(), 2);
        assert_eq!(fill[0].price, 100, "phải ăn giá tốt nhất trước");
        assert_eq!(fill.iter().map(|k| k.quantity).sum::<Quantity>(), 50);
        assert!(fill.iter().all(|k| k.aggressive));
    }

    #[test]
    fn passive_order_waits_in_queue() {
        let mut s = LitVenue::new();
        s.apply(&EventKind::AddOrder {
            id: 1,
            side: Side::Buy,
            price: 100,
            quantity: 200,
        });
        let fill = s.place_our_order(OurOrder {
            id: 9,
            side: Side::Buy,
            price: 100,
            remaining: 50,
            entered_at: 0,
            queue_ahead: 0,
        });
        assert!(fill.is_empty(), "không cắt qua thì không khớp ngay");
        let resting = s.our_resting_orders();
        assert_eq!(resting.len(), 1);
        assert_eq!(resting[0].queue_ahead, 200, "phải ghi nhận hàng đứng trước");
    }

    #[test]
    fn queue_ahead_is_served_first() {
        let mut s = LitVenue::new();
        s.apply(&EventKind::AddOrder {
            id: 1,
            side: Side::Buy,
            price: 100,
            quantity: 100,
        });
        s.place_our_order(OurOrder {
            id: 9,
            side: Side::Buy,
            price: 100,
            remaining: 50,
            entered_at: 1,
            queue_ahead: 0,
        });
        // Thị trường khớp 60: 100 đơn vị đứng trước chưa tiêu hết → ta không được gì.
        let k = s.on_market_trade(100, 60);
        assert!(
            k.is_empty(),
            "hàng đứng trước phải tiêu hết trước khi tới lượt ta"
        );
        // Khớp thêm 120: vượt qua 40 còn lại của hàng → ta được khớp phần dư.
        let k2 = s.on_market_trade(100, 120);
        assert!(!k2.is_empty());
        assert!(k2.iter().all(|x| !x.aggressive));
    }

    #[test]
    fn our_aggressive_fill_consumes_market_orders_exactly_once() {
        // Trước đây phần chủ động chỉ trừ con số tổng của mức giá; lệnh thị
        // trường bị ta ăn vẫn "sống", và khi bản tin huỷ của nó tới, mức giá
        // bị trừ lần thứ hai — lệnh #2 còn nguyên biến mất khỏi sổ.
        let mut s = LitVenue::new();
        for id in [1, 2] {
            s.apply(&EventKind::AddOrder {
                id,
                side: Side::Sell,
                price: 100,
                quantity: 30,
            });
        }
        let fill = s.place_our_order(OurOrder {
            id: 9,
            side: Side::Buy,
            price: 100,
            remaining: 30,
            entered_at: 0,
            queue_ahead: 0,
        });
        assert_eq!(fill.iter().map(|f| f.quantity).sum::<Quantity>(), 30);
        assert_eq!(s.qty_at(Side::Sell, 100), 30, "còn nguyên lệnh #2");
        // Lệnh #1 (FIFO) đã bị ta ăn hết; bản tin huỷ của nó không được đụng tới #2.
        s.apply(&EventKind::CancelOrder { id: 1 });
        assert_eq!(s.qty_at(Side::Sell, 100), 30);
        s.apply(&EventKind::CancelOrder { id: 2 });
        assert_eq!(s.qty_at(Side::Sell, 100), 0);
    }

    #[test]
    fn we_never_trade_against_ourselves() {
        let mut s = LitVenue::new();
        s.apply(&EventKind::AddOrder {
            id: 1,
            side: Side::Buy,
            price: 99,
            quantity: 10,
        });
        // Lệnh bán của ta nằm chờ ở 101 …
        assert!(
            s.place_our_order(OurOrder {
                id: 8,
                side: Side::Sell,
                price: 101,
                remaining: 20,
                entered_at: 0,
                queue_ahead: 0,
            })
            .is_empty()
        );
        // … rồi chính ta gửi lệnh mua cắt qua 101: không được tự khớp, và
        // phần dư không được nằm lại làm sổ chéo.
        let fill = s.place_our_order(OurOrder {
            id: 9,
            side: Side::Buy,
            price: 101,
            remaining: 5,
            entered_at: 1,
            queue_ahead: 0,
        });
        assert!(fill.is_empty(), "không tự khớp với chính mình");
        assert!(!s.is_crossed());
        assert!(
            s.our_order(9).is_none(),
            "phần còn lại bị huỷ, không nằm chờ"
        );
        assert_eq!(s.our_order(8).unwrap().remaining, 20);
    }

    #[test]
    fn buying_on_the_pool_receives_exactly_the_requested_quantity() {
        // Trước đây lệnh mua trên bể đưa `quantity` vào làm số Y — mua "10 X"
        // thực ra chỉ nhận được ~0,001 X, trong khi sổ sách ghi đã mua 10 X.
        let mut h = new_ecosystem();
        let x0 = h.venue_chain.reserve_x;
        h.publish(vec![Intent::Place {
            venue: Venue::Chain,
            side: Side::Buy,
            price: 11_000, // giá tệ nhất chấp nhận được
            quantity: 10,
        }]);
        h.clock.advance(1_000_000_000);
        h.deliver_due();
        assert_eq!(h.position.quantity, 10);
        assert_eq!(x0 - h.venue_chain.reserve_x, 10, "bể phải mất đúng 10 X");
        let (_, venue, side, px, qty) = *h.order_log.last().unwrap();
        assert_eq!((venue, side, qty), (Venue::Chain, Side::Buy, 10));
        assert!((10_000..=10_100).contains(&px), "giá thực trả {} Y/X", px);
        assert_eq!(h.resting_bid, 0, "lệnh AMM không bao giờ treo");
    }

    #[test]
    fn a_pool_order_beyond_its_limit_price_does_not_execute() {
        let mut h = new_ecosystem();
        h.publish(vec![Intent::Place {
            venue: Venue::Chain,
            side: Side::Buy,
            price: 9_000, // thấp hơn giá bể ~10 000 → không chấp nhận được
            quantity: 10,
        }]);
        h.clock.advance(1_000_000_000);
        h.deliver_due();
        assert_eq!(h.position.quantity, 0);
        assert_eq!(h.venue_chain.reserve_x, INITIAL_POOL.0);
    }

    #[test]
    fn histogram_percentile_is_an_upper_bound_capped_by_max() {
        let mut h = LatencyHistogram::new();
        for ns in [300u64, 300, 300, 50_000] {
            h.record(ns);
        }
        assert_eq!(h.percentile(0.5), 511, "cận trên của thùng [256, 512)");
        assert_eq!(
            h.percentile(1.0),
            50_000,
            "không vượt giá trị lớn nhất đã thấy"
        );
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
        assert!(
            large < small,
            "khối lượng lớn nhận được ít hơn trên mỗi đơn vị"
        );
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
        assert!(
            (v.cost_basis - 120.0).abs() < 1e-9,
            "phần dư mở ở giá giao dịch"
        );
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
            venue: Venue::Lit,
            side: Side::Buy,
            price: 0,
            quantity: 10,
        };
        assert_eq!(
            c.check(&y, 0, 0, 0, 0.0, 0),
            Err(RejectReason::PriceOutOfBand)
        );
    }

    #[test]
    fn gate_counts_resting_orders() {
        let mut c = RiskGate::typical();
        c.max_position = 100;
        let y = Intent::Place {
            venue: Venue::Lit,
            side: Side::Buy,
            price: 100,
            quantity: 50,
        };
        // Vị thế 0 nhưng đã treo mua 60 → thêm 50 nữa là vượt 100.
        assert_eq!(
            c.check(&y, 0, 60, 0, 0.0, 0),
            Err(RejectReason::PositionLimit)
        );
        // Không có lệnh treo thì cùng lệnh đó qua được.
        assert!(c.check(&y, 0, 0, 0, 0.0, 0).is_ok());
    }

    #[test]
    fn gate_blocks_on_loss_limit() {
        let mut c = RiskGate::typical();
        c.max_loss = 1_000.0;
        let y = Intent::Place {
            venue: Venue::Lit,
            side: Side::Buy,
            price: 100,
            quantity: 10,
        };
        assert_eq!(
            c.check(&y, 0, 0, 0, -1_500.0, 0),
            Err(RejectReason::LossLimit)
        );
    }

    #[test]
    fn kill_switch_blocks_new_orders() {
        let mut c = RiskGate::typical();
        c.kill_switch_on = true;
        let y = Intent::Place {
            venue: Venue::Lit,
            side: Side::Buy,
            price: 100,
            quantity: 1,
        };
        assert_eq!(
            c.check(&y, 0, 0, 0, 0.0, 0),
            Err(RejectReason::KillSwitchOn)
        );
    }

    #[test]
    fn cancels_always_allowed() {
        let mut c = RiskGate::typical();
        c.kill_switch_on = true;
        // Ngắt khẩn cấp phải cho HUỶ qua — nếu không, bạn không rút được chân ra.
        let y = Intent::CancelOrder {
            venue: Venue::Lit,
            id: 1,
        };
        assert!(c.check(&y, 0, 0, 0, 0.0, 0).is_ok());
    }

    #[test]
    fn gate_rate_limits_on_sliding_window() {
        let mut c = RiskGate::typical();
        c.max_orders_per_sec = 3;
        let y = Intent::Place {
            venue: Venue::Lit,
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
        assert!(
            a.iter().any(|&x| x != a[0]),
            "phải có dao động thật, không phải hằng số"
        );
        assert!(a.iter().all(|&x| x >= m.outbound_ns));
    }

    #[test]
    fn hash_is_uniformly_distributed() {
        // Số học chia dư đơn thuần làm giá trị co cụm và phá mọi phép đo phân phối.
        let mut buckets = [0u32; 8];
        for i in 0..8_000u64 {
            buckets[(hash64(i) % 8) as usize] += 1;
        }
        assert!(
            buckets.iter().all(|&c| c > 800 && c < 1_200),
            "phân bố phải đều: {:?}",
            buckets
        );
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
        assert!(
            h.percentile(0.99) <= 512,
            "p99 vẫn nhanh — cái đuôi bị giấu"
        );
        assert_eq!(h.max, 50_000);
        assert!(
            h.max as f64 > h.mean() * 100.0,
            "max lớn hơn trung bình >100×"
        );
    }

    // ---- chiến lược ----

    #[test]
    fn maker_skews_quotes_by_inventory() {
        let snap = MarketSnapshot {
            timestamp: 10_000_000,
            lit_bid: Some(PriceLevel {
                price: 100,
                quantity: 50,
            }),
            lit_ask: Some(PriceLevel {
                price: 104,
                quantity: 50,
            }),
            lit_micro_price: Some(102.0),
            lit_imbalance: Some(0.0),
            chain_price: 102.0,
            chain_reserve_x: 1,
            chain_reserve_y: 102,
        };
        let take = |position| {
            let mut m = ManagedMaker::new(100);
            let y = m.evaluate(&snap, position);
            y.iter()
                .filter_map(|x| match x {
                    Intent::Place {
                        side: Side::Buy,
                        price,
                        ..
                    } => Some(*price),
                    _ => None,
                })
                .next()
        };
        let flat = take(0).unwrap();
        let long = take(80).unwrap();
        assert!(long < flat, "dài vị thế → hạ giá mua để bớt mua thêm");
    }

    #[test]
    fn maker_never_crosses_book() {
        // Sổ hẹp hơn chênh lệch mục tiêu của nhà tạo lập — nếu không có ràng buộc,
        // báo giá sẽ cắt qua và biến nhà tạo lập thành người chủ động.
        let snap = MarketSnapshot {
            timestamp: 10_000_000,
            lit_bid: Some(PriceLevel {
                price: 101,
                quantity: 50,
            }),
            lit_ask: Some(PriceLevel {
                price: 102,
                quantity: 50,
            }),
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
                    Side::Sell => {
                        assert!(price > 101, "giá bán {} cắt qua giá mua tốt nhất", price)
                    }
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
        let mut strategies: Vec<Box<dyn Strategy>> = vec![Box::new(ManagedMaker::new(200))];
        h.run(&p, &mut strategies);
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
            lit_bid: Some(PriceLevel {
                price: 100,
                quantity: 50,
            }),
            lit_ask: Some(PriceLevel {
                price: 104,
                quantity: 50,
            }),
            lit_micro_price: Some(102.0),
            lit_imbalance: Some(0.0),
            chain_price: 102.0,
            chain_reserve_x: 1,
            chain_reserve_y: 102,
        };
        let mut m = ManagedMaker::new(100);
        let y = m.evaluate(&snap, 100);
        assert!(
            y.iter().all(|x| !matches!(
                x,
                Intent::Place {
                    side: Side::Buy,
                    ..
                }
            )),
            "chạm hạn mức dài thì không báo giá mua nữa"
        );
    }

    #[test]
    fn arb_fires_only_above_threshold() {
        let mut snap = MarketSnapshot {
            timestamp: 1,
            lit_bid: Some(PriceLevel {
                price: 10_000,
                quantity: 100,
            }),
            lit_ask: Some(PriceLevel {
                price: 10_002,
                quantity: 100,
            }),
            lit_micro_price: Some(10_001.0),
            lit_imbalance: Some(0.0),
            chain_price: 10_001.0,
            chain_reserve_x: 1,
            chain_reserve_y: 10_001,
        };
        let mut c = CrossVenueArb::new(50.0);
        assert!(
            c.evaluate(&snap, 0).is_empty(),
            "hai sàn ngang giá → không giao dịch"
        );

        snap.chain_price = 10_001.0 * 1.02; // lệch 200 bp
        let y = c.evaluate(&snap, 0);
        assert_eq!(y.len(), 1);
        match y[0] {
            Intent::PlaceHedged {
                venue,
                side,
                hedge_on,
                ..
            } => {
                // Chân KHÔNG CHẮC (sổ lệnh) chạy trước; chân chắc chắn (AMM)
                // chỉ phòng vệ đúng phần thực sự khớp.
                assert_eq!(venue, Venue::Lit);
                assert_eq!(
                    side,
                    Side::Buy,
                    "chuỗi khối đắt hơn → mua chân truyền thống"
                );
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
            ChainVenue::new(INITIAL_POOL.0, INITIAL_POOL.1, INITIAL_POOL.2),
            LatencyModel::typical(),
            ReplaySpeed::Unbounded,
        );
        let mut strategies: Vec<Box<dyn Strategy>> = vec![Box::new(CrossVenueArb::new(20.0))];
        h.run(&p, &mut strategies);
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
        let mut strategies: Vec<Box<dyn Strategy>> = vec![Box::new(ManagedMaker::new(200))];
        h.run(&p, &mut strategies);
        assert!(h.metrics.intent_count > 0, "chiến lược phải sinh ra ý định");
        assert!(
            h.metrics.orders_sent > 0,
            "phải có lệnh ra khỏi cổng rủi ro"
        );
        assert!(h.metrics.fill_count > 0, "phải có lệnh được khớp");
    }

    #[test]
    fn replay_is_deterministic_across_runs() {
        let p = generate_session(8_000, 0xBEEF, 10_000);
        let run = || {
            let mut h = new_ecosystem();
            let mut strategies: Vec<Box<dyn Strategy>> = vec![
                Box::new(ManagedMaker::new(200)),
                Box::new(CrossVenueArb::new(50.0)),
            ];
            h.run(&p, &mut strategies);
            (
                h.order_log.clone(),
                h.position.quantity,
                h.position.realized_pnl.to_bits(),
            )
        };
        assert_eq!(run(), run(), "hai lần chạy phải trùng khớp từng bit");
    }

    #[test]
    fn replay_speed_does_not_change_results() {
        let p = generate_session(6_000, 0xF00D, 10_000);
        let run = |speed| {
            let mut h = Ecosystem::new(
                ChainVenue::new(INITIAL_POOL.0, INITIAL_POOL.1, INITIAL_POOL.2),
                LatencyModel::typical(),
                speed,
            );
            let mut strategies: Vec<Box<dyn Strategy>> = vec![Box::new(ManagedMaker::new(200))];
            h.run(&p, &mut strategies);
            h.order_log.clone()
        };
        // Đẩy tốc độ chỉ nén thời gian TƯỜNG. Thời gian ẢO không đổi, nên
        // kết quả chiến lược phải y hệt — miễn là không ai đọc đồng hồ thật.
        assert_eq!(run(ReplaySpeed::Unbounded), run(ReplaySpeed::Fast(1_000)));
        assert_eq!(run(ReplaySpeed::Unbounded), run(ReplaySpeed::RealTime));
    }

    #[test]
    fn position_limit_is_never_breached() {
        for seed in [1u64, 99, 12345, 0xDEAD] {
            let p = generate_session(8_000, seed, 10_000);
            let mut h = new_ecosystem();
            h.gate.max_position = 150;
            let mut strategies: Vec<Box<dyn Strategy>> = vec![Box::new(ManagedMaker::new(120))];
            h.run(&p, &mut strategies);
            // Kiểm |vị thế| lớn nhất trong CẢ phiên, không chỉ vị thế cuối.
            assert!(
                h.metrics.peak_abs_position <= h.gate.max_position,
                "hạt {}: vị thế chạm {} vượt hạn mức {}",
                seed,
                h.metrics.peak_abs_position,
                h.gate.max_position
            );
        }
    }

    #[test]
    fn latency_delays_order_arrival() {
        let p = generate_session(4_000, 5, 10_000);
        let run = |lat| {
            let mut h = Ecosystem::new(
                ChainVenue::new(INITIAL_POOL.0, INITIAL_POOL.1, INITIAL_POOL.2),
                lat,
                ReplaySpeed::Unbounded,
            );
            let mut strategies: Vec<Box<dyn Strategy>> = vec![Box::new(ManagedMaker::new(200))];
            h.run(&p, &mut strategies);
            h.order_log.first().map(|x| x.0).unwrap_or(0)
        };
        let without_latency = run(LatencyModel::none());
        let with_latency = run(LatencyModel::typical());
        assert!(
            with_latency > without_latency,
            "có độ trễ thì lệnh đầu tiên tới sàn muộn hơn"
        );
    }

    #[test]
    fn ignoring_latency_changes_everything() {
        // Bỏ qua độ trễ là dạng nhìn trộm tương lai tinh vi nhất: không ai gọi
        // tên nó như vậy, nhưng nó cho chiến lược khớp ở giá đã không còn tồn tại.
        let p = generate_session(6_000, 0x1234, 10_000);
        let run = |lat| {
            let mut h = Ecosystem::new(
                ChainVenue::new(INITIAL_POOL.0, INITIAL_POOL.1, INITIAL_POOL.2),
                lat,
                ReplaySpeed::Unbounded,
            );
            let mut strategies: Vec<Box<dyn Strategy>> = vec![
                Box::new(ManagedMaker::new(200)),
                Box::new(CrossVenueArb::new(50.0)),
            ];
            h.run(&p, &mut strategies);
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
        let mut strategies: Vec<Box<dyn Strategy>> = vec![Box::new(ManagedMaker::new(200))];
        h.run(&p, &mut strategies);
        assert_eq!(h.metrics.orders_sent, 0);
        assert!(h.metrics.orders_blocked > 0);
        assert_eq!(h.position.quantity, 0);
    }

    #[test]
    fn both_venues_get_updated() {
        let p = generate_session(6_000, 0x5EED, 10_000);
        let mut h = new_ecosystem();
        let x0 = h.venue_chain.reserve_x;
        let mut strategies: Vec<Box<dyn Strategy>> = vec![Box::new(ManagedMaker::new(200))];
        h.run(&p, &mut strategies);
        assert_ne!(
            h.venue_chain.reserve_x, x0,
            "sự kiện chuỗi khối phải làm bể đổi"
        );
        assert!(
            h.venue_lit.mid().is_some(),
            "sổ truyền thống phải có hai chiều"
        );
    }

    #[test]
    fn log_records_only_gated_orders() {
        let p = generate_session(5_000, 0x99, 10_000);
        let mut h = new_ecosystem();
        let mut strategies: Vec<Box<dyn Strategy>> = vec![Box::new(ManagedMaker::new(200))];
        h.run(&p, &mut strategies);
        assert_eq!(h.order_log.len() as u64, h.metrics.orders_sent);
        assert_eq!(
            h.metrics.intent_count,
            h.metrics.orders_sent + h.metrics.orders_blocked
        );
    }

    #[test]
    fn log_timestamps_never_decrease() {
        let p = generate_session(6_000, 0x2468, 10_000);
        let mut h = new_ecosystem();
        let mut strategies: Vec<Box<dyn Strategy>> = vec![Box::new(ManagedMaker::new(200))];
        h.run(&p, &mut strategies);
        assert!(
            h.order_log.windows(2).all(|w| w[0].0 <= w[1].0),
            "lệnh phải tới sàn theo đúng thứ tự thời gian, dù dao động đảo thứ tự phát"
        );
    }

    #[test]
    fn max_drawdown_is_non_negative() {
        let p = generate_session(5_000, 0x1111, 10_000);
        let mut h = new_ecosystem();
        let mut strategies: Vec<Box<dyn Strategy>> = vec![Box::new(ManagedMaker::new(200))];
        h.run(&p, &mut strategies);
        assert!(h.metrics.max_drawdown() >= 0.0);
    }

    #[test]
    fn two_strategies_emit_more_intents() {
        let p = generate_session(6_000, 0x3333, 10_000);
        let count = |n: usize| {
            let mut h = new_ecosystem();
            let mut strategies: Vec<Box<dyn Strategy>> = if n == 1 {
                vec![Box::new(ManagedMaker::new(200))]
            } else {
                vec![
                    Box::new(ManagedMaker::new(200)),
                    Box::new(CrossVenueArb::new(1.0)),
                ]
            };
            h.run(&p, &mut strategies);
            h.metrics.intent_count
        };
        assert!(
            count(2) > count(1),
            "thêm chiến lược thì phải có thêm ý định"
        );
    }
}
```

---

## Bảng tra cứu lỗi biên dịch thường gặp

| Lỗi | Nguyên nhân trong chương này | Cách sửa |
|---|---|---|
| `E0502: cannot borrow *self as mutable because it is also borrowed as immutable` | Duyệt `self.our_orders.iter()` rồi gọi `self.reduce(..)` trong vòng lặp | Thu mã cần sửa vào `Vec` trước, sửa sau vòng lặp (như `fill_ours_at_level`) |
| `E0499: cannot borrow *self as mutable more than once` | Giữ cùng lúc kết quả của hai phương thức `&mut self`, như `self.lit_mut()` và `self.pos_mut()` | Mượn thẳng hai trường (`&mut self.venue_lit`, `&mut self.position`) — trình kiểm tra mượn tách được trường riêng biệt; hoặc lấy giá trị cần dùng ra trước |
| `E0038: the trait Strategy is not dyn compatible` | `Strategy` có phương thức generic | Giữ trait "dyn compatible" (trước Rust 1.83 gọi là "object safe"): không phương thức generic, không `Self` ở vị trí trả về |
| `E0277: the size for values of type dyn Strategy cannot be known at compilation time` | Chứa trait object trực tiếp trong `Vec<dyn Strategy>` | `Vec<Box<dyn Strategy>>` |
| Kết quả khác nhau giữa hai lần chạy | `HashMap`, RNG chưa gieo hạt, hoặc `Instant::now()` | `BTreeMap` + hạt giống cố định + đồng hồ ảo |
| Vị thế vượt hạn mức dù cổng "đã kiểm" | Không đếm lệnh đang bay | Đặt chỗ phơi nhiễm lúc **phát**, không lúc giao |

---

## Tóm tắt chương & Bài tập rèn luyện

### 6 điểm cốt lõi

1. **Năm mảnh đúng riêng lẻ vẫn ghép thành một hệ sai.** Toàn bộ năm lỗi của chương này chỉ lộ ra ở mức tích hợp.
2. **Cổng rủi ro phải đặt chỗ, không chỉ kiểm tra.** Nhiều lệnh trong cùng một nhịp đều thấy cùng một trạng thái — mọi phép kiểm đều "OK" mà hạn mức vẫn vỡ.
3. **Đừng suy ra cái bạn đã biết.** Đoán chiều khớp từ giá là một dòng code trông vô hại làm hỏng toàn bộ quản trị rủi ro.
4. **Huỷ lệnh phải đi đường ưu tiên**, kể cả khi đã ngắt khẩn cấp — nếu không, bạn không có đường rút chân.
5. **Chênh lệch giá là phòng vệ theo khối lượng đã khớp**, không phải đặt cứng hai chân. Chân chắc chắn chỉ chạy sau khi chân không chắc đã trả lời.
6. **Một backtest đúng cơ học vẫn có thể sai kinh tế.** Hãy hỏi môi trường mô phỏng của bạn thiếu lực nào mà thị trường thật có.

### Bài tập rèn luyện

**Bài 1.** Thêm **bộ giám sát sức khoẻ** phát hiện hệ thống đang tự bóp cổ mình.

<details>
<summary><b>Gợi ý</b></summary>

Ba triệu chứng đã gặp trong chính chương này, và cả ba đều đo được **trước khi** gây thiệt hại: tỉ lệ bị cổng chặn tăng vọt (phơi nhiễm kẹt), tỉ lệ thụ động sụt (đang cắt qua sổ), và số lệnh treo phình ra (không rút báo giá). Giám sát chúng theo cửa sổ trượt để bắt được xu hướng chứ không chỉ mức tuyệt đối.
</details>

<details>
<summary><b>Lời giải</b></summary>

```rust
#[derive(Debug, PartialEq)]
pub enum HealthAlert {
    GateBlockingTooMuch { ratio: f64 },
    TooAggressive { passive_ratio: f64 },
    RestingOrdersBloated { count: usize },
    NoFills,
}

pub struct HealthMonitor {
    pub block_threshold: f64,
    pub passive_threshold: f64,
    pub resting_threshold: usize,
}

impl HealthMonitor {
    pub fn typical() -> Self {
        HealthMonitor {
            block_threshold: 0.5,
            passive_threshold: 0.5,
            resting_threshold: 200,
        }
    }

    pub fn check(&self, eco: &Ecosystem) -> Vec<HealthAlert> {
        let m = &eco.metrics;
        let mut out = Vec::new();

        // Triệu chứng ③+④: phơi nhiễm kẹt, cổng chặn gần như mọi thứ.
        if m.intent_count > 100 && m.block_ratio() > self.block_threshold {
            out.push(HealthAlert::GateBlockingTooMuch {
                ratio: m.block_ratio(),
            });
        }
        // Triệu chứng ⑤: nhà tạo lập đang cắt qua sổ.
        if m.filled_qty > 0 && m.passive_ratio() < self.passive_threshold {
            out.push(HealthAlert::TooAggressive {
                passive_ratio: m.passive_ratio(),
            });
        }
        // Triệu chứng ③: báo giá không bao giờ được rút.
        let resting = eco.venue_lit.our_resting_orders().len();
        if resting > self.resting_threshold {
            out.push(HealthAlert::RestingOrdersBloated { count: resting });
        }
        if m.orders_sent > 100 && m.fill_count == 0 {
            out.push(HealthAlert::NoFills);
        }
        out
    }
}
```

Điểm quan trọng: ba trong bốn cảnh báo dựa trên **tỉ lệ**, không phải số tuyệt đối (cảnh báo số lệnh treo là ngưỡng tuyệt đối, nên phải chỉnh theo quy mô chiến lược). Số tuyệt đối phụ thuộc vào phiên; tỉ lệ thì so sánh được giữa các ngày, và đó mới là thứ dùng để đặt ngưỡng cảnh báo trong sản xuất.
</details>

**Bài 2.** Cài **chiến lược lấy tín hiệu từ sàn chuỗi khối để giao dịch sàn truyền thống** — dùng bể AMM làm chỉ báo dẫn dắt.

<details>
<summary><b>Gợi ý</b></summary>

Trên nhiều tài sản, một sàn "dẫn" và sàn kia "theo" — hiện tượng khám phá giá. Nếu bể AMM phản ứng trước, thì độ lệch giữa giá bể và giá giữa sàn truyền thống là dự báo cho bước tiếp theo của sàn truyền thống.

Điểm cần cẩn thận: đây không còn là chênh lệch giá phi rủi ro nữa mà là **cược có hướng**. Nó cần dừng lỗ và cần giới hạn thời gian giữ vị thế, thứ mà chiến lược chênh lệch có phòng vệ không cần.
</details>

<details>
<summary><b>Lời giải</b></summary>

```rust
pub struct PoolLeadFollower {
    pub threshold_bps: f64,
    pub quantity: Quantity,
    pub limit: Quantity,
    /// Thời điểm vào lệnh gần nhất — nền của giới hạn thời gian giữ.
    pub entered_at: Option<Nanos>,
    pub max_hold_ns: Nanos,
}

impl PoolLeadFollower {
    pub fn new(threshold_bps: f64, limit: Quantity) -> Self {
        PoolLeadFollower {
            threshold_bps,
            quantity: 10,
            limit,
            entered_at: None,
            max_hold_ns: 50_000_000, // 50 ms
        }
    }
}

impl Strategy for PoolLeadFollower {
    fn name(&self) -> &str {
        "pool_lead_follower"
    }

    fn evaluate(&mut self, snap: &MarketSnapshot, position: Quantity) -> Vec<Intent> {
        // Hết thời gian giữ → thoát, bất kể lãi hay lỗ. Một cược có hướng
        // không có giới hạn thời gian sẽ biến thành khoản đầu tư dài hạn
        // ngoài ý muốn.
        if let Some(t0) = self.entered_at
            && snap.timestamp.saturating_sub(t0) > self.max_hold_ns
            && position != 0
        {
            self.entered_at = None;
            let (side, level) = if position > 0 {
                (Side::Sell, snap.lit_bid)
            } else {
                (Side::Buy, snap.lit_ask)
            };
            if let Some(m) = level {
                return vec![Intent::Place {
                    venue: Venue::Lit,
                    side,
                    price: m.price,
                    quantity: position.abs().min(m.quantity),
                }];
            }
        }

        let Some(gap) = snap.cross_venue_bps() else {
            return Vec::new();
        };
        if gap.abs() < self.threshold_bps {
            return Vec::new();
        }

        // Bể đắt hơn → dự báo sàn truyền thống sẽ đi LÊN → mua.
        let (side, level) = if gap > 0.0 {
            (Side::Buy, snap.lit_ask)
        } else {
            (Side::Sell, snap.lit_bid)
        };
        if (side == Side::Buy && position >= self.limit)
            || (side == Side::Sell && position <= -self.limit)
        {
            return Vec::new();
        }
        match level {
            Some(m) => {
                self.entered_at = Some(snap.timestamp);
                vec![Intent::Place {
                    venue: Venue::Lit,
                    side,
                    price: m.price,
                    quantity: self.quantity.min(m.quantity),
                }]
            }
            None => Vec::new(),
        }
    }
}
```

Khác biệt then chốt so với `CrossVenueArb`: chiến lược này **không phòng vệ**, nên nó mang rủi ro hướng thật. Bù lại nó phải có hạn mức vị thế riêng và giới hạn thời gian giữ. Đó là đánh đổi cơ bản — bỏ phòng vệ để lấy kỳ vọng lợi nhuận cao hơn, và trả bằng rủi ro phải quản lý bằng tay.
</details>
