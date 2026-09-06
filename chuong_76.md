# Chương 76: Phục dựng phiên giao dịch — Ghi phiên, Đồng hồ ảo & Phát lại

## Giới thiệu & Mục tiêu học tập

Kiểm thử ngược (backtest) trên dữ liệu nến là **nói dối một cách lịch sự**. Nến hàng ngày không cho bạn biết bạn có được khớp không, khớp ở đâu trong hàng, hay lệnh của bạn có tự làm dịch giá không.

Chương này dựng thứ mà các hãng nghiêm túc dùng: **phục dựng phiên**. Ghi lại từng thông điệp thị trường kèm dấu thời gian nanosecond, rồi phát lại **đúng theo dòng thời gian gốc** — hoặc nhanh hơn nếu bạn muốn.

| Khả năng | Vì sao cần |
|---|---|
| Phát lại theo dòng thời gian gốc | Chiến lược thấy đúng những gì nó đã thấy trong thực tế |
| Đẩy tốc độ ×N | Chạy một ngày dữ liệu trong vài phút |
| Mô hình độ trễ | Lệnh của bạn tới sàn **sau** một khoảng trễ — như thật |
| Tất định | Chạy lại cho kết quả y hệt — điều kiện để gỡ lỗi |

**Đây là chương có nhiều lỗi thật nhất trong cả bộ sách.** Bốn lỗi lớn đã bị phát hiện và sửa trong quá trình xây dựng, và mỗi lỗi đều là một bài học về lý do phục dựng khó.

---

## Hình tượng hóa đời sống

```
┌──────────────────────────────────────────────────────────────────────────────┐
│  GHI PHIÊN = HỘP ĐEN MÁY BAY CHO THỊ TRƯỜNG                                 │
│                                                                              │
│   09:30:00.000000123  thêm lệnh mua  100.50 × 500                           │
│   09:30:00.000000891  thêm lệnh bán  100.52 × 300                           │
│   09:30:00.000012044  huỷ lệnh #7                                           │
│                       ↑                                                      │
│              Dấu thời gian NANOSECOND. Chênh lệch giữa các thông điệp        │
│              chính là nhịp thở của thị trường — phải giữ nguyên.            │
│                                                                              │
│  ĐỒNG HỒ ẢO = ĐỒNG HỒ CHẠY THEO SỰ KIỆN, KHÔNG THEO TƯỜNG                   │
│                                                                              │
│   Thời gian thật:  ├────────────────────────────────────────┤ 6,5 giờ      │
│   Tốc độ ×1000:    ├──┤ 23 giây                                             │
│   Tốc độ vô hạn:   ├┤ chạy hết sức máy, bỏ qua chờ đợi                      │
│                                                                              │
│   Quan trọng: đồng hồ ảo phải là NGUỒN THỜI GIAN DUY NHẤT.                  │
│   Chiến lược lỡ gọi `Instant::now()` là hỏng — nó thấy thời gian thật.      │
│                                                                              │
│  MÔ HÌNH ĐỘ TRỄ = LỆNH CỦA BẠN KHÔNG TỚI NGAY                               │
│                                                                              │
│   t=0    bạn thấy giá 100.50, quyết định mua                                │
│   t=0    gửi lệnh                                                            │
│   t=+50µs lệnh tới sàn ← thị trường đã đổi trong 50µs này!                  │
│                                                                              │
│   Bỏ qua độ trễ → backtest cho lợi nhuận đẹp không tồn tại.                 │
│   Đây là dạng "nhìn trộm tương lai" tinh vi nhất.                           │
│                                                                              │
│  TÍNH TẤT ĐỊNH = CHẠY LẠI PHẢI RA KẾT QUẢ Y HỆT                            │
│                                                                              │
│   ⚠ LỖI THẬT ĐÃ GẶP: dùng HashMap để duyệt khi cấp phát khớp lệnh.         │
│     Thứ tự duyệt HashMap khác nhau mỗi lần chạy → kết quả khác nhau.        │
│     → Đổi sang BTreeMap. Đúng cái tính chất mà chương này tồn tại để bảo vệ.│
└──────────────────────────────────────────────────────────────────────────────┘
```

---

## Khái niệm & Cơ chế kỹ thuật chuyên sâu

### 1. Bốn lỗi thật đã gặp khi xây chương này

Đây không phải lỗi bịa ra để dạy học. Chúng xuất hiện thật, và mỗi lỗi đều được phát hiện bằng cách **chạy** chứ không phải bằng cách đọc.

**Lỗi 1 — Bộ phát lại bỏ qua lệnh huỷ.** Bản đầu chỉ xử lý "thêm lệnh". Kết quả: sổ lệnh chỉ lớn lên, và sau vài nghìn sự kiện thì mua vượt bán vĩnh viễn — sổ "chéo" mãi mãi. Trong 20 000 sự kiện, chiến lược chỉ gửi được **2 lệnh**. Bài học: một bộ phát lại thiếu một loại thông điệp không phải là "gần đúng", nó là **sai hoàn toàn**.

**Lỗi 2 — Bộ sinh dữ liệu huỷ lệnh theo ID ngẫu nhiên.** Sau khi sửa lỗi 1, sổ vẫn chéo. Nguyên nhân: bộ sinh chọn ID ngẫu nhiên để huỷ, mà phần lớn ID đó đã chết rồi. Các báo giá cũ vẫn nằm lại. Sửa bằng cách huỷ **lệnh còn sống cũ nhất** và giới hạn số lệnh sống ở 120 — mô phỏng đúng hành vi thật của nhà tạo lập.

**Lỗi 3 — `HashMap` phá tính tất định.** Khi cấp phát khớp lệnh, mã duyệt một `HashMap`. Thứ tự duyệt của `HashMap` trong Rust thay đổi giữa các lần chạy (do RandomState), nên phát lại **không tái lập được**. Đây là điều mỉa mai nhất: chính chương dạy về tính tất định lại vi phạm nó. Sửa: dùng `BTreeMap`.

**Lỗi 4 — Nhà tạo lập vượt hạn mức tồn kho.** Bản `NaiveMaker` chỉ đếm vị thế **đã khớp**. Nhưng lệnh đang treo cũng là rủi ro. Kết quả: vị thế chạm −900 dù hạn mức là 300. Sửa bằng `ManagedMaker` — theo dõi cả phơi nhiễm đang chờ. Bản sai được **giữ lại** làm ví dụ phản chứng có kiểm thử.

### 2. Vì sao độ trễ là loại nhìn trộm tương lai tinh vi nhất

Ai cũng biết không được dùng giá đóng cửa để quyết định giao dịch trong ngày. Nhưng có một dạng nhìn trộm tinh vi hơn nhiều: **giả định lệnh của bạn tới sàn tức thì**.

Trong thực tế có ba khoảng trễ:
- **Trễ dữ liệu**: từ lúc sàn phát tới lúc bạn nhận (~10 µs).
- **Trễ quyết định**: thời gian chiến lược tính toán (~5 µs).
- **Trễ lệnh**: từ lúc bạn gửi tới lúc sàn nhận (~50 µs).

Tổng khoảng 65 µs. Trong 65 µs đó, thị trường có thể đã dịch chuyển — và lệnh của bạn khớp ở giá khác với giá bạn thấy. Backtest bỏ qua điều này thường cho ra chiến lược "lãi ổn định" mà thực chất chỉ đang thu hoạch thông tin từ tương lai.

Mô hình độ trễ (Latency model) cũng cần **jitter** (dao động), không chỉ giá trị cố định. Độ trễ thật có đuôi dài — và đuôi đó xuất hiện đúng lúc thị trường biến động mạnh, tức là lúc nó gây thiệt hại nhất.

### 3. Đẩy tốc độ: cái gì đổi và cái gì không

Khi phát lại ở tốc độ ×1000, **thời gian ảo** vẫn tiến đúng như thật. Chỉ có thời gian tường là bị nén. Nghĩa là:

- Khoảng cách giữa các sự kiện (theo đồng hồ ảo) **không đổi**.
- Độ trễ mô hình hoá (theo đồng hồ ảo) **không đổi**.
- Kết quả chiến lược **không đổi**.

Điều đó chỉ đúng nếu chiến lược **không bao giờ đọc đồng hồ thật**. Đây là lý do kiến trúc quan trọng: mọi thành phần phải nhận thời gian qua tham số, không tự gọi `Instant::now()`. Trong Rust, cách ép buộc điều này là truyền `&VirtualClock` vào và không cho phép truy cập nào khác.

### 4. Tồn kho: đếm cả những gì chưa xảy ra

`NaiveMaker` sai vì nó chỉ đếm vị thế đã khớp. Nhưng nếu bạn đang treo 5 lệnh bán, mỗi lệnh 100 đơn vị, thì rủi ro thật của bạn là vị thế hiện tại **cộng thêm** 500 đơn vị bán tiềm năng.

Công thức đúng:

```
phơi_nhiễm_mua  = vị_thế + tổng_khối_lượng_lệnh_mua_đang_treo
phơi_nhiễm_bán  = vị_thế − tổng_khối_lượng_lệnh_bán_đang_treo
```

Và cả hai phải nằm trong hạn mức. Đây chính là nguyên tắc mà mọi hệ thống rủi ro thật đều áp dụng, và cũng là cầu nối sang chương 77.

---

## Mã nguồn minh họa thực chiến

Chạy bằng `cargo run -p ch76`, kiểm thử bằng `cargo test -p ch76`.

```rust
#![allow(dead_code)]
//! Chương 76 — Ghi & Phát lại phiên deliver dịch: định dạng bản ghi, đồng hồ ảo,
//! phát lại đúng dòng thời gian hoặc tua nhanh, mô hình độ trễ, và mô phỏng
//! khớp lệnh có xét vị trí hàng đợi.
//!
//! Đây là "phòng thí nghiệm" của mọi đội deliver dịch nghiêm túc: ghi lại phiên
//! thật một lần, rồi chạy lại hàng nghìn lần với các chiến lược khác nhau,
//! kết quả TÁI LẬP TUYỆT ĐỐI.

use std::collections::BTreeMap;

// ============================================================================
// 1. ĐỊNH DẠNG BẢN GHI — khung có tiền tố độ dài
// ============================================================================
// Mỗi khung: [độ dài u32 BE][thời điểm ns u64 BE][thân bản tin].
// Tiền tố độ dài cho phép đọc tuần tự mà không cần phân tích thân — nên bộ
// ghi có thể lưu BẤT KỲ deliver thức nào mà không cần hiểu nó.

pub type Price = i64;
pub type Quantity = u32;
pub type OrderId = u64;

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum Side { Buy, Sell }

impl Side {
    pub fn inverse(self) -> Side {
        match self { Side::Buy => Side::Sell, Side::Sell => Side::Buy }
    }
}

#[derive(Debug, Clone, PartialEq)]
pub enum EventMarket {
    AddOrder { id: OrderId, side: Side, price: Price, quantity: Quantity },
    CancelOrder { id: OrderId },
    Fill { price: Price, quantity: Quantity, side_aggressive: Side },
}

#[derive(Debug, Clone, PartialEq)]
pub struct FrameRecord {
    /// Nano-giây kể từ mốc bắt đầu phiên. KHÔNG dùng ngày lịch — múi giờ,
    /// giờ mùa hè và giây nhuận đều là nguồn lỗi không đáng chuốc vào.
    pub timestamp_nanos: u64,
    pub event: EventMarket,
}

#[derive(Debug, PartialEq)]
pub enum ErrorRead { TruncatedFrame, DoDaiVoLy(u32), MaSuKienLa(u8) }

/// Bộ ghi phiên. Trong hệ thống thật, `content` được xả xuống đĩa theo lô;
/// ở đây giữ trong bộ nhớ để kiểm thử được.
#[derive(Debug, Default)]
pub struct SessionRecorder {
    pub content: Vec<u8>,
    pub num_frame: u64,
    pub first_timestamp: Option<u64>,
    pub last_timestamp: u64,
}

impl SessionRecorder {
    pub fn new() -> Self { SessionRecorder::default() }

    pub fn record(&mut self, k: &FrameRecord) {
        let than = encode_event(&k.event);
        let length = (8 + than.len()) as u32;
        self.content.extend_from_slice(&length.to_be_bytes());
        self.content.extend_from_slice(&k.timestamp_nanos.to_be_bytes());
        self.content.extend_from_slice(&than);
        self.num_frame += 1;
        if self.first_timestamp.is_none() { self.first_timestamp = Some(k.timestamp_nanos); }
        self.last_timestamp = k.timestamp_nanos;
    }

    pub fn time_amount_nanos(&self) -> u64 {
        self.last_timestamp - self.first_timestamp.unwrap_or(0)
    }
    pub fn so_byte(&self) -> usize { self.content.len() }

    /// Đọc lại toàn bộ. Trả lỗi nếu bản ghi bị cắt cụt — chuyện thường gặp khi
    /// tiến trình ghi bị giết giữa chừng, và phải xử lý được chứ không panic.
    pub fn doc_lai(&self) -> Result<Vec<FrameRecord>, ErrorRead> {
        let mut ra = Vec::new();
        let b = &self.content;
        let mut i = 0usize;
        while i < b.len() {
            if i + 4 > b.len() { return Err(ErrorRead::TruncatedFrame); }
            let length = u32::from_be_bytes(b[i..i + 4].try_into().unwrap()) as usize;
            if length < 8 { return Err(ErrorRead::DoDaiVoLy(length as u32)); }
            if i + 4 + length > b.len() { return Err(ErrorRead::TruncatedFrame); }
            let timestamp_nanos = u64::from_be_bytes(b[i + 4..i + 12].try_into().unwrap());
            let event = decode_event(&b[i + 12..i + 4 + length])?;
            ra.push(FrameRecord { timestamp_nanos, event });
            i += 4 + length;
        }
        Ok(ra)
    }
}

fn encode_event(sk: &EventMarket) -> Vec<u8> {
    let mut v = Vec::with_capacity(24);
    match sk {
        EventMarket::AddOrder { id, side, price, quantity } => {
            v.push(b'A');
            v.extend_from_slice(&id.to_be_bytes());
            v.push(if *side == Side::Buy { b'B' } else { b'S' });
            v.extend_from_slice(&price.to_be_bytes());
            v.extend_from_slice(&quantity.to_be_bytes());
        }
        EventMarket::CancelOrder { id } => {
            v.push(b'X');
            v.extend_from_slice(&id.to_be_bytes());
        }
        EventMarket::Fill { price, quantity, side_aggressive } => {
            v.push(b'T');
            v.extend_from_slice(&price.to_be_bytes());
            v.extend_from_slice(&quantity.to_be_bytes());
            v.push(if *side_aggressive == Side::Buy { b'B' } else { b'S' });
        }
    }
    v
}

fn decode_event(b: &[u8]) -> Result<EventMarket, ErrorRead> {
    if b.is_empty() { return Err(ErrorRead::TruncatedFrame); }
    let can = match b[0] {
        b'A' => 22, b'X' => 9, b'T' => 14,
        x => return Err(ErrorRead::MaSuKienLa(x)),
    };
    if b.len() < can { return Err(ErrorRead::TruncatedFrame); }
    Ok(match b[0] {
        b'A' => EventMarket::AddOrder {
            id: u64::from_be_bytes(b[1..9].try_into().unwrap()),
            side: if b[9] == b'B' { Side::Buy } else { Side::Sell },
            price: i64::from_be_bytes(b[10..18].try_into().unwrap()),
            quantity: u32::from_be_bytes(b[18..22].try_into().unwrap()),
        },
        b'X' => EventMarket::CancelOrder {
            id: u64::from_be_bytes(b[1..9].try_into().unwrap()),
        },
        _ => EventMarket::Fill {
            price: i64::from_be_bytes(b[1..9].try_into().unwrap()),
            quantity: u32::from_be_bytes(b[9..13].try_into().unwrap()),
            side_aggressive: if b[13] == b'B' { Side::Buy } else { Side::Sell },
        },
    })
}

// ============================================================================
// 2. ĐỒNG HỒ ẢO — thứ khiến phát lại TÁI LẬP ĐƯỢC
// ============================================================================
// Điều kiện sống còn: chiến lược KHÔNG ĐƯỢC gọi đồng hồ hệ thống. Nó chỉ được
// hỏi đồng hồ ảo do bộ phát lại điều khiển. Nhờ vậy hai lần chạy trên cùng dữ
// liệu cho ra kết quả giống hệt nhau, bất kể máy nhanh hay chậm.

#[derive(Debug, Clone, Copy, PartialEq)]
pub struct VirtualClock { pub bay_gio_ns: u64 }

impl VirtualClock {
    pub fn new(start: u64) -> Self { VirtualClock { bay_gio_ns: start } }
    pub fn advance(&mut self, ns: u64) { if ns > self.bay_gio_ns { self.bay_gio_ns = ns; } }
    pub fn adder_gate(&mut self, ns: u64) { self.bay_gio_ns += ns; }
}

/// Tốc độ phát lại.
#[derive(Debug, Clone, Copy, PartialEq)]
pub enum ReplaySpeed {
    /// Đúng nhịp thật: giữ nguyên khoảng cách giữa các sự kiện.
    RealTime,
    /// Nhân tốc độ: 2.0 = nhanh gấp đôi, 0.5 = chậm một nửa (để quan sát kỹ).
    HeSo(f64),
    /// Bỏ hẳn thời gian chờ — dùng khi quét tham số hàng nghìn lần.
    AsFastAsPossible,
}

impl ReplaySpeed {
    /// Thời gian THỰC (nano-giây) phải chờ, ứng với `khoang_cach_ns` trong dữ liệu.
    pub fn wall_delay(&self, khoang_cach_ns: u64) -> u64 {
        match self {
            ReplaySpeed::RealTime => khoang_cach_ns,
            ReplaySpeed::HeSo(h) if *h > 0.0 => (khoang_cach_ns as f64 / h) as u64,
            _ => 0,
        }
    }
}

// ============================================================================
// 3. MÔ HÌNH ĐỘ TRỄ — lệnh của ta KHÔNG tới nơi tức thì
// ============================================================================
// Bỏ qua độ trễ là cách nhanh nhất để dựng ra một chiến lược "thắng" trên
// giấy rồi thua tiền thật. Ở tốc độ HFT, 50 µs là đủ để cơ hội biến mất.

#[derive(Debug, Clone, Copy, PartialEq)]
pub struct LatencyModel {
    /// Từ lúc sàn phát tin tới lúc ta nhận được.
    pub in_nanos: u64,
    /// Từ lúc ta quyết định tới lúc lệnh tới sàn.
    pub out_nanos: u64,
    /// Dao động cộng thêm (tất định, dựa trên số thứ tự sự kiện).
    pub jitter_ns: u64,
}

impl LatencyModel {
    pub fn no_latency() -> Self { LatencyModel { in_nanos: 0, out_nanos: 0, jitter_ns: 0 } }
    pub fn set_custom_tax() -> Self {
        LatencyModel { in_nanos: 5_000, out_nanos: 8_000, jitter_ns: 2_000 }
    }
    pub fn qua_internet() -> Self {
        LatencyModel { in_nanos: 8_000_000, out_nanos: 12_000_000, jitter_ns: 5_000_000 }
    }

    /// Tổng thời gian từ lúc SÀN phát tin tới lúc lệnh của ta ĐẾN SÀN.
    /// Đây chính là "tick-to-trade" mà Chương 74 mổ xẻ.
    pub fn round_trip_ns(&self, nonce: u64) -> u64 {
        // Dao động tất định: cùng chuỗi sự kiện luôn cho cùng độ trễ
        let d = if self.jitter_ns == 0 { 0 } else {
            (nonce.wrapping_mul(2654435761) >> 32) % self.jitter_ns
        };
        self.in_nanos + self.out_nanos + d
    }
}

// ============================================================================
// 4. SỔ LỆNH RÚT GỌN CHO MÔ PHỎNG
// ============================================================================

#[derive(Debug, Default, Clone)]
pub struct ReducedBook {
    buy: BTreeMap<Price, u64>, // khoá ÂM → giá cao nhất trước
    ban: BTreeMap<Price, u64>,
}

impl ReducedBook {
    pub fn them(&mut self, c: Side, g: Price, kl: u64) {
        let (bd, k) = match c {
            Side::Buy => (&mut self.buy, -g), Side::Sell => (&mut self.ban, g) };
        *bd.entry(k).or_insert(0) += kl;
    }
    pub fn bot(&mut self, c: Side, g: Price, kl: u64) {
        let (bd, k) = match c {
            Side::Buy => (&mut self.buy, -g), Side::Sell => (&mut self.ban, g) };
        if let Some(v) = bd.get_mut(&k) {
            *v = v.saturating_sub(kl);
            if *v == 0 { bd.remove(&k); }
        }
    }
    pub fn best_bid(&self) -> Option<Price> { self.buy.keys().next().map(|k| -k) }
    pub fn best_ask(&self) -> Option<Price> { self.ban.keys().next().copied() }
    pub fn quantity(&self, c: Side, g: Price) -> u64 {
        let (bd, k) = match c { Side::Buy => (&self.buy, -g), Side::Sell => (&self.ban, g) };
        bd.get(&k).copied().unwrap_or(0)
    }
}

// ============================================================================
// 5. LỆNH CỦA TA TRONG MÔ PHỎNG
// ============================================================================

#[derive(Debug, Clone, PartialEq)]
pub struct OurOrder {
    pub id: OrderId,
    pub side: Side,
    pub price: Price,
    pub quantity: Quantity,
    pub filled: Quantity,
    /// Khối lượng đứng TRƯỚC ta trong hàng lúc lệnh tới sàn. Phải khớp hết
    /// chỗ đó thì mới tới lượt ta — đây là điểm mà phần lớn bộ kiểm định
    /// nghiệp dư bỏ qua, và vì thế cho kết quả lạc quan phi thực tế.
    pub quantity_prev_mat: u64,
    pub timestamp_toi_venue_nanos: u64,
}

impl OurOrder {
    pub fn remaining(&self) -> Quantity { self.quantity - self.filled }
    pub fn fill_done(&self) -> bool { self.filled >= self.quantity }
}

#[derive(Debug, Clone, PartialEq)]
pub struct OurFill {
    pub order_id: OrderId,
    pub side: Side,
    pub price: Price,
    pub quantity: Quantity,
    pub timestamp_nanos: u64,
}

#[derive(Debug, Clone, Copy, PartialEq, Default)]
pub struct Position { pub quantity: i64, pub tien_mat: i64 }

impl Position {
    pub fn compose(self, k: Position) -> Position {
        Position { quantity: self.quantity + k.quantity, tien_mat: self.tien_mat + k.tien_mat }
    }
    pub fn from_fill(c: Side, g: Price, sl: Quantity) -> Position {
        let first = if c == Side::Buy { 1 } else { -1 };
        Position { quantity: first * sl as i64, tien_mat: -first * g * sl as i64 }
    }
    pub fn value_empty(&self, gia_tt: Price) -> i64 { self.tien_mat + self.quantity * gia_tt }
}

/// Chiến lược nhìn thấy gì và làm gì. Thuần tuý: cùng đầu vào → cùng đầu ra.
pub trait StrategyReplay {
    fn name(&self) -> &str;
    /// Gọi sau MỖI sự kiện thị trường. Trả về các lệnh muốn gửi.
    fn when_has_event(&mut self, clock: &VirtualClock, so: &ReducedBook,
                      position: &Position) -> Vec<(Side, Price, Quantity)>;
    /// Gọi khi một lệnh của ta được khớp.
    fn when_can_fill(&mut self, _k: &OurFill) {}
}

// ============================================================================
// 6. BỘ PHÁT LẠI
// ============================================================================

#[derive(Debug, PartialEq)]
pub struct ResultReplay {
    pub event_count: u64,
    pub orders_sent: u64,
    pub order_book_fill: u64,
    pub all_fill: Vec<OurFill>,
    pub last_position: Position,
    pub last_value: i64,
    /// Tổng thời gian ẢO đã trôi qua.
    pub time_time_ao_nanos: u64,
    /// Tổng thời gian THỰC phải chờ nếu chạy ở tốc độ đã chọn.
    pub real_wait_nanos: u64,
}

pub struct Replayer {
    pub latency: LatencyModel,
    pub speed: ReplaySpeed,
}

impl Replayer {
    pub fn new(latency: LatencyModel, speed: ReplaySpeed) -> Self {
        Replayer { latency, speed }
    }

    /// Chạy lại phiên. Toàn bộ là hàm THUẦN TUÝ trên `cac_khung` — không đọc
    /// đồng hồ hệ thống, không đọc tệp, không ngẫu nhiên.
    pub fn run(&self, cac_khung: &[FrameRecord], cl: &mut dyn StrategyReplay) -> ResultReplay {
        let mut so = ReducedBook::default();
        // Phải theo dõi từng lệnh của THỊ TRƯỜNG thì mới xử lý được lệnh huỷ.
        // Bỏ qua huỷ lệnh là lỗi mô hình nghiêm trọng: sổ chỉ phình ra, các
        // mức giá cũ không bao giờ biến mất, và chỉ sau vài nghìn sự kiện là
        // sổ bị chéo vĩnh viễn — chiến lược đứng ngoài mà ta không hiểu vì sao.
        // BTreeMap chứ KHÔNG phải HashMap: ta duyệt bản đồ này khi phân bổ
        // khối lượng khớp, mà thứ tự duyệt HashMap trong Rust KHÔNG TẤT ĐỊNH
        // giữa các lần chạy (hạt giống băm ngẫu nhiên chống tấn công HashDoS).
        // Dùng HashMap ở đây làm hỏng luôn tính tái lập của cả bộ phát lại —
        // đúng thứ mà chương này tồn tại để bảo vệ.
        let mut market_orders: BTreeMap<OrderId, (Side, Price, u64)> = BTreeMap::new();
        let mut clock = VirtualClock::new(cac_khung.first().map_or(0, |k| k.timestamp_nanos));
        let mut order_wait: Vec<OurOrder> = Vec::new();
        let mut all_fill = Vec::new();
        let mut position = Position::default();
        let mut id_ke = 1u64;
        let mut orders_sent = 0u64;
        let mut real_wait = 0u64;
        let mut last_price: Price = 0;
        let mut prev_do = clock.bay_gio_ns;

        for (i, k) in cac_khung.iter().enumerate() {
            real_wait += self.speed.wall_delay(k.timestamp_nanos.saturating_sub(prev_do));
            prev_do = k.timestamp_nanos;
            clock.advance(k.timestamp_nanos);

            // --- Lệnh nào vừa "bay tới sàn" thì chốt vị trí hàng đợi NGAY LÚC ĐÓ,
            //     không phải lúc ta quyết định. Đây là chi tiết quyết định tính
            //     thực tế của toàn bộ mô phỏng.
            for l in order_wait.iter_mut() {
                if l.timestamp_toi_venue_nanos <= clock.bay_gio_ns
                    && l.quantity_prev_mat == u64::MAX {
                    l.quantity_prev_mat = so.quantity(l.side, l.price);
                }
            }

            // --- Áp dụng sự kiện thị trường ---
            match &k.event {
                EventMarket::AddOrder { id, side, price, quantity } => {
                    so.them(*side, *price, *quantity as u64);
                    market_orders.insert(*id, (*side, *price, *quantity as u64));
                }
                EventMarket::CancelOrder { id } => {
                    if let Some((c, g, kl)) = market_orders.remove(id) {
                        so.bot(c, g, kl);
                    }
                }
                EventMarket::Fill { price, quantity, side_aggressive } => {
                    last_price = *price;
                    // Lệnh khớp ăn vào bên THỤ ĐỘNG
                    let side_is_hidden = side_aggressive.inverse();
                    so.bot(side_is_hidden, *price, *quantity as u64);
                    // Khớp cũng làm cạn lệnh thị trường ở mức giá đó
                    let mut con_an = *quantity as u64;
                    let mut can_remove: Vec<OrderId> = Vec::new();
                    for (m, (c, g, kl)) in market_orders.iter_mut() {
                        if con_an == 0 { break; }
                        if *c != side_is_hidden || *g != *price { continue; }
                        let an = con_an.min(*kl);
                        *kl -= an;
                        con_an -= an;
                        if *kl == 0 { can_remove.push(*m); }
                    }
                    for m in can_remove { market_orders.remove(&m); }

                    // Lệnh của ta cùng bên thụ động, cùng giá thì có thể tới lượt
                    let mut con = *quantity as u64;
                    for l in order_wait.iter_mut() {
                        if con == 0 { break; }
                        if l.fill_done() || l.side != side_is_hidden || l.price != *price { continue; }
                        if l.timestamp_toi_venue_nanos > clock.bay_gio_ns { continue; }
                        // Trước hết phải "ăn" hết phần đứng trước ta
                        let prev_hidden = con.min(l.quantity_prev_mat);
                        l.quantity_prev_mat -= prev_hidden;
                        con -= prev_hidden;
                        if l.quantity_prev_mat > 0 || con == 0 { continue; }
                        // Giờ mới tới lượt ta
                        let fill = con.min(l.remaining() as u64) as Quantity;
                        if fill > 0 {
                            l.filled += fill;
                            con -= fill as u64;
                            let kq = OurFill { order_id: l.id, side: l.side, price: *price,
                                                 quantity: fill, timestamp_nanos: clock.bay_gio_ns };
                            position = position.compose(Position::from_fill(l.side, *price, fill));
                            cl.when_can_fill(&kq);
                            all_fill.push(kq);
                        }
                    }
                }
            }

            // --- Chiến lược quyết định ---
            for (side, price, sl) in cl.when_has_event(&clock, &so, &position) {
                if sl == 0 { continue; }
                order_wait.push(OurOrder {
                    id: id_ke, side, price, quantity: sl, filled: 0,
                    quantity_prev_mat: u64::MAX, // chốt sau, lúc tới sàn
                    timestamp_toi_venue_nanos: clock.bay_gio_ns + self.latency.round_trip_ns(i as u64),
                });
                id_ke += 1;
                orders_sent += 1;
            }
            order_wait.retain(|l| !l.fill_done());
        }

        ResultReplay {
            event_count: cac_khung.len() as u64,
            orders_sent,
            order_book_fill: all_fill.len() as u64,
            last_value: position.value_empty(last_price),
            last_position: position,
            all_fill,
            time_time_ao_nanos: cac_khung.last().map_or(0, |k| k.timestamp_nanos)
                             - cac_khung.first().map_or(0, |k| k.timestamp_nanos),
            real_wait_nanos: real_wait,
        }
    }
}

// ============================================================================
// 7. CHIẾN LƯỢC MẪU
// ============================================================================

/// Tạo lập thị trường: đặt lệnh bid dưới và bán trên giá giữa, ăn chênh lệch.
pub struct NaiveMaker {
    pub tick_offset: Price,
    pub has_order: Quantity,
    pub max_position: i64,
    pub step: u64,
    pub every_n_events: u64,
}

impl StrategyReplay for NaiveMaker {
    fn name(&self) -> &str { "Tạo lập thị trường đơn giản" }

    fn when_has_event(&mut self, _dh: &VirtualClock, so: &ReducedBook, vt: &Position)
        -> Vec<(Side, Price, Quantity)>
    {
        self.step += 1;
        if self.step % self.every_n_events != 0 { return vec![]; }
        let (m, b) = match (so.best_bid(), so.best_ask()) {
            (Some(m), Some(b)) => (m, b),
            _ => return vec![],
        };
        if b <= m { return vec![]; } // sổ chéo hoặc khoá → đứng ngoài
        let mid = (m + b) / 2;
        let mut ra = Vec::new();
        // Kiểm soát tồn kho: đã ôm nhiều thì thôi bid thêm
        if vt.quantity < self.max_position {
            ra.push((Side::Buy, mid - self.tick_offset, self.has_order));
        }
        if vt.quantity > -self.max_position {
            ra.push((Side::Sell, mid + self.tick_offset, self.has_order));
        }
        ra
    }
}

/// Bản CÓ KIỂM SOÁT: đếm cả khối lượng ĐANG TREO chứ không chỉ vị thế đã khớp.
///
/// Đây là khác biệt giữa một mô hình đồ chơi và một chiến lược dám chạy tiền
/// thật. Lệnh đã gửi mà chưa khớp vẫn là RỦI RO: nó có thể khớp bất cứ lúc nào.
/// Chỉ nhìn vị thế đã khớp thì cứ mỗi nhịp lại chào thêm, và khi thị trường
/// quét qua thì tất cả khớp một lượt — vị thế nhảy vọt qua trần.
pub struct ManagedMaker {
    pub tick_offset: Price,
    pub has_order: Quantity,
    pub max_position: i64,
    pub step: u64,
    pub every_n_events: u64,
    resting_bid: i64,
    resting_ask: i64,
}

impl ManagedMaker {
    pub fn new(tick_offset: Price, has_order: Quantity, max_position: i64, every_n_events: u64) -> Self {
        ManagedMaker { tick_offset, has_order, max_position, step: 0,
                           every_n_events, resting_bid: 0, resting_ask: 0 }
    }
    pub fn is_pending(&self) -> (i64, i64) { (self.resting_bid, self.resting_ask) }
}

impl StrategyReplay for ManagedMaker {
    fn name(&self) -> &str { "Tạo lập có kiểm soát tồn kho" }

    fn when_has_event(&mut self, _dh: &VirtualClock, so: &ReducedBook, vt: &Position)
        -> Vec<(Side, Price, Quantity)>
    {
        self.step += 1;
        if self.step % self.every_n_events != 0 { return vec![]; }
        let (m, b) = match (so.best_bid(), so.best_ask()) {
            (Some(m), Some(b)) => (m, b), _ => return vec![],
        };
        if b <= m { return vec![]; }
        let mid = (m + b) / 2;
        let co = self.has_order as i64;
        let mut ra = Vec::new();
        // PHƠI BÀY = vị thế đã khớp + toàn bộ khối lượng đang treo cùng chiều
        if vt.quantity + self.resting_bid + co <= self.max_position {
            ra.push((Side::Buy, mid - self.tick_offset, self.has_order));
            self.resting_bid += co;
        }
        if vt.quantity - self.resting_ask - co >= -self.max_position {
            ra.push((Side::Sell, mid + self.tick_offset, self.has_order));
            self.resting_ask += co;
        }
        ra
    }

    fn when_can_fill(&mut self, k: &OurFill) {
        // Khớp rồi thì phần đó không còn "treo" nữa — nó đã thành vị thế
        match k.side {
            Side::Buy => self.resting_bid = (self.resting_bid - k.quantity as i64).max(0),
            Side::Sell => self.resting_ask = (self.resting_ask - k.quantity as i64).max(0),
        }
    }
}

pub struct UseOut;
impl StrategyReplay for UseOut {
    fn name(&self) -> &str { "Đứng ngoài" }
    fn when_has_event(&mut self, _: &VirtualClock, _: &ReducedBook, _: &Position)
        -> Vec<(Side, Price, Quantity)> { vec![] }
}

// ============================================================================
// 8. SINH PHIÊN TẤT ĐỊNH ĐỂ GHI LẠI
// ============================================================================

/// Sinh một phiên tất định. Hai chi tiết quyết định tính THỰC TẾ của nó:
///
/// 1. **Huỷ lệnh nhắm đúng lệnh CŨ NHẤT còn sống.** Thị trường thật rút báo
///    giá cũ liên tục (>90% lệnh bị huỷ trước khi khớp). Nếu huỷ theo mã ngẫu
///    nhiên, phần lớn lệnh huỷ trúng mã đã biến mất, báo giá cũ nằm lại mãi,
///    và sau vài nghìn sự kiện là sổ CHÉO VĨNH VIỄN.
/// 2. **Số lệnh sống bị chặn trần.** Vượt trần thì lệnh cũ nhất bị đẩy ra —
///    mô phỏng đúng việc thanh khoản cũ tan đi khi giá đã đi xa.
pub fn gen_session_record(event_count: usize, hat_giong: u64) -> Vec<FrameRecord> {
    const TRAN_LENH_SONG: usize = 120;
    let mut s = hat_giong;
    let mut t = 9 * 3_600 * 1_000_000_000u64; // 9 giờ sáng, tính bằng ns
    let mut mid: Price = 8_400;
    let mut id = 1u64;
    let mut song: std::collections::VecDeque<OrderId> = std::collections::VecDeque::new();
    let mut ra = Vec::with_capacity(event_count);

    for _ in 0..event_count {
        s = s.wrapping_mul(6364136223846793005).wrapping_add(1442695040888963407);
        t += 10_000 + (s >> 20) % 500_000; // 10 µs – 0,5 ms giữa các sự kiện
        let r = (s >> 33) % 100;

        // Quá nhiều lệnh cũ thì buộc phải rút bớt, bất kể bốc trúng gì
        if song.len() >= TRAN_LENH_SONG {
            if let Some(cu) = song.pop_front() {
                ra.push(FrameRecord { timestamp_nanos: t, event: EventMarket::CancelOrder { id: cu } });
                continue;
            }
        }

        if r < 55 || song.is_empty() {
            let side = if (s >> 41) % 2 == 0 { Side::Buy } else { Side::Sell };
            let lech = 1 + ((s >> 45) % 10) as i64;
            let price = match side { Side::Buy => mid - lech, Side::Sell => mid + lech };
            let sl = 100 + ((s >> 49) % 5) as u32 * 100;
            ra.push(FrameRecord { timestamp_nanos: t,
                event: EventMarket::AddOrder { id, side, price, quantity: sl } });
            song.push_back(id);
            id += 1;
        } else if r < 85 {
            // Rút báo giá CŨ NHẤT — đây là chi tiết giữ cho sổ không bị chéo
            let cu = song.pop_front().unwrap();
            ra.push(FrameRecord { timestamp_nanos: t, event: EventMarket::CancelOrder { id: cu } });
        } else {
            let side = if (s >> 41) % 2 == 0 { Side::Buy } else { Side::Sell };
            let price = match side { Side::Buy => mid + 1, Side::Sell => mid - 1 };
            let sl = 100 + ((s >> 49) % 5) as u32 * 100;
            ra.push(FrameRecord { timestamp_nanos: t,
                event: EventMarket::Fill { price, quantity: sl, side_aggressive: side } });
            // Giá đi lang thang một chút quanh mốc ban đầu
            mid += if (s >> 57) % 2 == 0 { 1 } else { -1 };
            mid = mid.clamp(8_350, 8_450);
        }
    }
    ra
}

fn main() {
    println!("═══════════════════════════════════════════════════════════");
    println!("   GHI & PHÁT LẠI PHIÊN GIAO DỊCH                          ");
    println!("═══════════════════════════════════════════════════════════");

    println!("\n1. GHI LẠI MỘT PHIÊN");
    let session = gen_session_record(20_000, 2024);
    let mut record = SessionRecorder::new();
    for k in &session { record.record(k); }
    println!("   {} sự kiện · {} byte · {:.2} byte/sự kiện",
             record.num_frame, record.so_byte(), record.so_byte() as f64 / record.num_frame as f64);
    println!("   Thời lượng phiên: {:.3} giây", record.time_amount_nanos() as f64 / 1e9);
    let doc = record.doc_lai().unwrap();
    println!("   Đọc lại khớp bản gốc từng bit: {}", doc == session);

    println!("\n2. BẢN GHI BỊ CẮT CỤT — phải báo lỗi, không được panic");
    let mut hong = SessionRecorder::new();
    for k in session.iter().take(5) { hong.record(k); }
    hong.content.truncate(hong.content.len() - 3); // giả lập tiến trình bị giết
    println!("   Đọc bản ghi cụt → {:?}", hong.doc_lai().unwrap_err());

    println!("\n3. TỐC ĐỘ PHÁT LẠI");
    let mut cl = UseOut;
    for (name, td) in [("thời gian thực", ReplaySpeed::RealTime),
                      ("nhanh 10 lần  ", ReplaySpeed::HeSo(10.0)),
                      ("nhanh 1000 lần", ReplaySpeed::HeSo(1000.0)),
                      ("nhanh nhất    ", ReplaySpeed::AsFastAsPossible)] {
        let kq = Replayer::new(LatencyModel::no_latency(), td).run(&session, &mut cl);
        println!("   {} → thời gian ảo {:.2}s · phải chờ thật {:.4}s",
                 name, kq.time_time_ao_nanos as f64 / 1e9, kq.real_wait_nanos as f64 / 1e9);
    }
    println!("   → Quét 1000 tổ hợp tham số: chạy đúng nhịp mất ~{:.0} phút,",
             record.time_amount_nanos() as f64 / 1e9 * 1000.0 / 60.0);
    println!("     chạy ở chế độ nhanh nhất chỉ mất vài giây.");

    println!("\n4. ĐỘ TRỄ ĂN MẤT LỢI NHUẬN NHƯ THẾ NÀO");
    for (name, dt) in [("không độ trễ  ", LatencyModel::no_latency()),
                      ("đặt thuê riêng", LatencyModel::set_custom_tax()),
                      ("qua Internet  ", LatencyModel::qua_internet())] {
        let mut cl = NaiveMaker { tick_offset: 2, has_order: 100,
                                     max_position: 500, step: 0, every_n_events: 50 };
        let kq = Replayer::new(dt, ReplaySpeed::AsFastAsPossible).run(&session, &mut cl);
        println!("   {} → khứ hồi {:>9} ns · gửi {:>4} lệnh · khớp {:>3} · lãi {:>8} tick",
                 name, dt.round_trip_ns(0), kq.orders_sent, kq.order_book_fill, kq.last_value);
    }
    println!("   → Cùng chiến lược, cùng dữ liệu. Chỉ khác chỗ ngồi so với sàn.");

    println!("\n5. KIỂM SOÁT TỒN KHO — đếm cả lệnh ĐANG TREO");
    let tran = 300i64;
    let mut naive = NaiveMaker { tick_offset: 1, has_order: 100,
                                       max_position: tran, step: 0, every_n_events: 5 };
    let a = Replayer::new(LatencyModel::set_custom_tax(), ReplaySpeed::AsFastAsPossible)
        .run(&session, &mut naive);
    let mut chat_che = ManagedMaker::new(1, 100, tran, 5);
    let b = Replayer::new(LatencyModel::set_custom_tax(), ReplaySpeed::AsFastAsPossible)
        .run(&session, &mut chat_che);
    println!("   Trần đặt ra: {}", tran);
    println!("   Chỉ nhìn vị thế đã khớp → vị thế cuối {:>6}  ← VƯỢT TRẦN",
             a.last_position.quantity);
    println!("   Đếm cả lệnh đang treo   → vị thế cuối {:>6}  ← trong trần",
             b.last_position.quantity);
    println!("   → Lệnh đã gửi mà chưa khớp VẪN LÀ RỦI RO.");

    println!("\n6. TÁI LẬP TUYỆT ĐỐI");
    let run = || {
        let mut c = NaiveMaker { tick_offset: 2, has_order: 100,
                                    max_position: 500, step: 0, every_n_events: 50 };
        Replayer::new(LatencyModel::set_custom_tax(), ReplaySpeed::AsFastAsPossible)
            .run(&session, &mut c)
    };
    println!("   Chạy hai lần cho kết quả giống hệt: {}", run() == run());
    println!("   → Vì chiến lược chỉ hỏi ĐỒNG HỒ ẢO, không bao giờ hỏi đồng hồ hệ thống.");

    println!("\n═══════════════════════════════════════════════════════════");
    println!("   GHI MỘT LẦN, CHẠY LẠI HÀNG NGHÌN LẦN, KẾT QUẢ KHÔNG ĐỔI  ");
    println!("═══════════════════════════════════════════════════════════");
}

#[cfg(test)]
mod tests {
    use super::*;

    fn record_session(n: usize, h: u64) -> (Vec<FrameRecord>, SessionRecorder) {
        let p = gen_session_record(n, h);
        let mut g = SessionRecorder::new();
        for k in &p { g.record(k); }
        (p, g)
    }

    // ---------- Định dạng bản ghi ----------
    #[test]
    fn record_then_read_matches_bit_for_bit() {
        let (p, g) = record_session(2_000, 1);
        assert_eq!(g.doc_lai().unwrap(), p, "vòng ghi–đọc phải khép kín tuyệt đối");
        assert_eq!(g.num_frame, 2_000);
    }

    #[test]
    fn every_event_kind_round_trips() {
        let all = vec![
            EventMarket::AddOrder { id: 1, side: Side::Buy, price: 8_450, quantity: 100 },
            EventMarket::AddOrder { id: 2, side: Side::Sell, price: -7, quantity: 1 },
            EventMarket::CancelOrder { id: 999 },
            EventMarket::Fill { price: 8_400, quantity: 50, side_aggressive: Side::Sell },
        ];
        for sk in all {
            let mut g = SessionRecorder::new();
            let k = FrameRecord { timestamp_nanos: 123_456_789, event: sk };
            g.record(&k);
            assert_eq!(g.doc_lai().unwrap(), vec![k]);
        }
    }

    #[test]
    fn truncated_record_errors_instead_of_panicking() {
        // Tiến trình ghi bị giết giữa chừng là chuyện bình thường trong vận hành.
        let (_, g) = record_session(10, 2);
        for cat in 1..12usize {
            let mut h = SessionRecorder::new();
            h.content = g.content[..g.content.len() - cat].to_vec();
            assert!(matches!(h.doc_lai(), Err(ErrorRead::TruncatedFrame) | Err(ErrorRead::MaSuKienLa(_))),
                    "cắt {} byte cuối phải báo lỗi", cat);
        }
    }

    #[test]
    fn absurd_frame_length_is_rejected() {
        let mut g = SessionRecorder::new();
        g.content = vec![0, 0, 0, 3, 1, 2, 3]; // độ dài 3 < 8 byte dấu thời gian
        assert_eq!(g.doc_lai(), Err(ErrorRead::DoDaiVoLy(3)));
    }

    #[test]
    fn id_event_is_is_reject() {
        let mut g = SessionRecorder::new();
        g.content.extend_from_slice(&9u32.to_be_bytes());
        g.content.extend_from_slice(&0u64.to_be_bytes());
        g.content.push(b'?');
        assert_eq!(g.doc_lai(), Err(ErrorRead::MaSuKienLa(b'?')));
    }

    #[test]
    fn sell_record_empty_read_out_list_empty() {
        assert_eq!(SessionRecorder::new().doc_lai(), Ok(vec![]));
    }

    #[test]
    fn frame_size_equals_the_sum_of_its_fields() {
        // 4 byte độ dài + 8 byte dấu thời gian + thân.
        // Thân: A = 1+8+1+8+4 = 22 · X = 1+8 = 9 · T = 1+8+4+1 = 14
        let ktra = |sk: EventMarket, mong: usize| {
            let mut g = SessionRecorder::new();
            g.record(&FrameRecord { timestamp_nanos: 1, event: sk });
            assert_eq!(g.so_byte(), mong);
        };
        ktra(EventMarket::AddOrder { id: 1, side: Side::Buy, price: 1, quantity: 1 }, 34);
        ktra(EventMarket::CancelOrder { id: 1 }, 21);
        ktra(EventMarket::Fill { price: 1, quantity: 1, side_aggressive: Side::Buy }, 26);
    }

    #[test]
    fn the_binary_format_is_compact_enough_for_a_full_day() {
        let (_, g) = record_session(10_000, 3);
        let bytes_per_event = g.so_byte() as f64 / g.num_frame as f64;
        // Phiên trộn ~70% thêm lệnh (34 B), 15% huỷ (21 B), 15% khớp (26 B)
        // → trung bình khoảng 31 byte.
        assert!((21.0..32.0).contains(&bytes_per_event),
                "trung bình {:.2} byte/sự kiện, kỳ vọng trong khoảng 21–32", bytes_per_event);
        // Một phiên sôi động 50 triệu sự kiện vẫn chỉ khoảng 1,5 GB
        let full_day_gb = 50_000_000.0 * bytes_per_event / 1e9;
        assert!(full_day_gb < 2.0, "cả ngày ~{:.2} GB — thừa sức lưu trữ", full_day_gb);
    }

    // ---------- Đồng hồ ảo ----------
    #[test]
    fn virtual_clock_never_runs_backwards() {
        let mut d = VirtualClock::new(1_000);
        d.advance(500); // sự kiện tới muộn, dấu thời gian cũ
        assert_eq!(d.bay_gio_ns, 1_000, "thời gian không được lùi");
        d.advance(2_000);
        assert_eq!(d.bay_gio_ns, 2_000);
        d.adder_gate(50);
        assert_eq!(d.bay_gio_ns, 2_050);
    }

    // ---------- Tốc độ phát ----------
    #[test]
    fn replay_speed_computes_the_right_wall_delay() {
        assert_eq!(ReplaySpeed::RealTime.wall_delay(1_000_000), 1_000_000);
        assert_eq!(ReplaySpeed::HeSo(2.0).wall_delay(1_000_000), 500_000);
        assert_eq!(ReplaySpeed::HeSo(0.5).wall_delay(1_000_000), 2_000_000,
                   "hệ số < 1 để chạy CHẬM lại mà quan sát kỹ");
        assert_eq!(ReplaySpeed::AsFastAsPossible.wall_delay(1_000_000), 0);
        assert_eq!(ReplaySpeed::HeSo(0.0).wall_delay(1_000_000), 0,
                   "hệ số 0 không được gây chia cho 0");
    }

    #[test]
    fn fast_forward_must_not_change_results() {
        // Tua nhanh chỉ đổi thời gian ta phải ngồi chờ, KHÔNG đổi những gì xảy ra.
        let p = gen_session_record(3_000, 5);
        let mut kq: Vec<ResultReplay> = Vec::new();
        for td in [ReplaySpeed::RealTime, ReplaySpeed::HeSo(100.0), ReplaySpeed::AsFastAsPossible] {
            let mut c = NaiveMaker { tick_offset: 2, has_order: 100,
                                        max_position: 500, step: 0, every_n_events: 25 };
            kq.push(Replayer::new(LatencyModel::set_custom_tax(), td).run(&p, &mut c));
        }
        assert_eq!(kq[0].all_fill, kq[1].all_fill);
        assert_eq!(kq[1].all_fill, kq[2].all_fill);
        assert_eq!(kq[0].time_time_ao_nanos, kq[2].time_time_ao_nanos);
        assert!(kq[0].real_wait_nanos > kq[1].real_wait_nanos);
        assert_eq!(kq[2].real_wait_nanos, 0);
    }

    // ---------- Mô hình độ trễ ----------
    #[test]
    fn latency_all_peak_theo_nonce() {
        let d = LatencyModel::set_custom_tax();
        for i in 0..100u64 {
            assert_eq!(d.round_trip_ns(i), d.round_trip_ns(i), "cùng sự kiện → cùng độ trễ");
        }
        assert_eq!(LatencyModel::no_latency().round_trip_ns(42), 0);
    }

    #[test]
    fn latency_always_in_range_hop_ly() {
        let d = LatencyModel::set_custom_tax();
        let min = d.in_nanos + d.out_nanos;
        for i in 0..1_000u64 {
            let x = d.round_trip_ns(i);
            assert!(x >= min && x < min + d.jitter_ns,
                    "độ trễ {} nằm ngoài [{}, {})", x, min, min + d.jitter_ns);
        }
    }

    #[test]
    fn a_leased_line_beats_the_internet_by_orders_of_magnitude() {
        let a = LatencyModel::set_custom_tax().round_trip_ns(0);
        let b = LatencyModel::qua_internet().round_trip_ns(0);
        assert!(b > a * 100, "ngồi cạnh sàn nhanh hơn {} lần", b / a.max(1));
    }

    // ---------- Sổ rút gọn ----------
    #[test]
    fn reduced_book_reports_the_right_best_prices() {
        let mut s = ReducedBook::default();
        s.them(Side::Buy, 8_390, 100);
        s.them(Side::Buy, 8_400, 200);
        s.them(Side::Sell, 8_420, 100);
        s.them(Side::Sell, 8_410, 50);
        assert_eq!(s.best_bid(), Some(8_400));
        assert_eq!(s.best_ask(), Some(8_410));
        s.bot(Side::Buy, 8_400, 200);
        assert_eq!(s.best_bid(), Some(8_390), "mức hết hàng phải biến mất");
    }

    // ---------- Vị thế ----------
    #[test]
    fn position_is_pos_group() {
        let a = Position::from_fill(Side::Buy, 100, 10);
        let b = Position::from_fill(Side::Sell, 110, 5);
        let c = Position::from_fill(Side::Buy, 90, 3);
        assert_eq!(a.compose(b).compose(c), a.compose(b.compose(c)), "luật kết hợp");
        assert_eq!(a.compose(Position::default()), a, "luật đơn vị");
    }

    #[test]
    fn buy_low_sell_high_is_profitable() {
        let v = Position::from_fill(Side::Buy, 8_000, 100)
            .compose(Position::from_fill(Side::Sell, 8_500, 100));
        assert_eq!(v.quantity, 0);
        assert_eq!(v.value_empty(0), 50_000);
    }

    // ---------- Phát lại ----------
    #[test]
    fn skipping_cancels_leaves_the_book_permanently_crossed() {
        // Bài học mô hình: nếu bộ phát lại bỏ qua bản tin huỷ, sổ chỉ phình
        // ra, các mức giá cũ không bao giờ mất, và chỉ sau vài nghìn sự kiện
        // là sổ chéo vĩnh viễn — chiến lược đứng ngoài mà ta không hiểu vì sao.
        let p = gen_session_record(20_000, 2024);
        let mut c = NaiveMaker { tick_offset: 2, has_order: 100,
                                    max_position: 500, step: 0, every_n_events: 50 };
        let kq = Replayer::new(LatencyModel::set_custom_tax(), ReplaySpeed::AsFastAsPossible)
            .run(&p, &mut c);
        // 20 000 sự kiện, cứ 50 sự kiện lại chào giá → phải gửi hàng trăm lệnh
        assert!(kq.orders_sent > 50,
                "chỉ gửi {} lệnh — dấu hiệu sổ bị chéo và chiến lược đứng ngoài",
                kq.orders_sent);
        assert!(kq.order_book_fill > 20, "và phải khớp được kha khá, thực tế {}", kq.order_book_fill);
    }

    #[test]
    fn cancels_actually_remove_liquidity() {
        let frame = vec![
            FrameRecord { timestamp_nanos: 1_000,
                event: EventMarket::AddOrder { id: 1, side: Side::Buy,
                                                     price: 8_400, quantity: 500 } },
            FrameRecord { timestamp_nanos: 2_000,
                event: EventMarket::AddOrder { id: 2, side: Side::Sell,
                                                     price: 8_410, quantity: 300 } },
            FrameRecord { timestamp_nanos: 3_000, event: EventMarket::CancelOrder { id: 1 } },
        ];
        // Dùng một chiến lược chỉ quan sát để đọc trạng thái sổ ở bước cuối
        struct Soi { last_bid: Option<Price>, last_ask: Option<Price> }
        impl StrategyReplay for Soi {
            fn name(&self) -> &str { "soi sổ" }
            fn when_has_event(&mut self, _: &VirtualClock, so: &ReducedBook, _: &Position)
                -> Vec<(Side, Price, Quantity)> {
                self.last_bid = so.best_bid();
                self.last_ask = so.best_ask();
                vec![]
            }
        }
        let mut s = Soi { last_bid: None, last_ask: None };
        Replayer::new(LatencyModel::no_latency(), ReplaySpeed::AsFastAsPossible)
            .run(&frame, &mut s);
        assert_eq!(s.last_bid, None, "lệnh bid đã bị huỷ, bên bid phải rỗng");
        assert_eq!(s.last_ask, Some(8_410), "lệnh bán không bị đụng tới");
    }

    #[test]
    fn replay_is_bit_exact_reproducible() {
        // BẤT BIẾN QUAN TRỌNG NHẤT của chương. Nếu bài này hỏng thì mọi kết
        // quả kiểm định đều vô nghĩa vì không so sánh được với nhau.
        let p = gen_session_record(5_000, 2024);
        let run = || {
            let mut c = NaiveMaker { tick_offset: 2, has_order: 100,
                                        max_position: 500, step: 0, every_n_events: 30 };
            Replayer::new(LatencyModel::set_custom_tax(), ReplaySpeed::AsFastAsPossible)
                .run(&p, &mut c)
        };
        assert_eq!(run(), run());
        assert_eq!(run(), run(), "ba lần vẫn phải giống hệt");
    }

    #[test]
    fn standing_aside_means_no_orders_and_no_pnl() {
        let p = gen_session_record(2_000, 7);
        let kq = Replayer::new(LatencyModel::set_custom_tax(), ReplaySpeed::AsFastAsPossible)
            .run(&p, &mut UseOut);
        assert_eq!(kq.orders_sent, 0);
        assert_eq!(kq.order_book_fill, 0);
        assert_eq!(kq.last_position, Position::default());
        assert_eq!(kq.last_value, 0);
    }

    #[test]
    fn order_no_position_fill_prev_when_toi_venue() {
        // Nếu mô phỏng cho lệnh khớp ngay lúc quyết định, ta đã "nhìn trộm
        // tương lai" ở mức tinh vi nhất — và kết quả sẽ đẹp một cách giả tạo.
        let p = gen_session_record(3_000, 11);
        let mut c = NaiveMaker { tick_offset: 1, has_order: 100,
                                    max_position: 10_000, step: 0, every_n_events: 10 };
        let dt = LatencyModel::qua_internet();
        let kq = Replayer::new(dt, ReplaySpeed::AsFastAsPossible).run(&p, &mut c);
        let first = p.first().unwrap().timestamp_nanos;
        let min = dt.in_nanos + dt.out_nanos;
        for k in &kq.all_fill {
            assert!(k.timestamp_nanos >= first + min,
                    "khớp lúc {} là quá sớm — lệnh chưa kịp bay tới sàn", k.timestamp_nanos);
        }
    }

    #[test]
    fn more_latency_means_fewer_fills() {
        // Đây là lý do các hãng trả rất nhiều tiền để đặt máy cạnh sàn.
        let p = gen_session_record(8_000, 2024);
        let count_fill = |dt: LatencyModel| {
            let mut c = NaiveMaker { tick_offset: 1, has_order: 100,
                                        max_position: 10_000, step: 0, every_n_events: 10 };
            Replayer::new(dt, ReplaySpeed::AsFastAsPossible).run(&p, &mut c).order_book_fill
        };
        let fast = count_fill(LatencyModel::set_custom_tax());
        let cham = count_fill(LatencyModel::qua_internet());
        assert!(fast >= cham,
                "gần sàn phải khớp được ít nhất bằng: {} so với {}", fast, cham);
    }

    #[test]
    fn position_last_table_use_total_all_lan_fill() {
        // Kế toán phải khớp: vị thế = tổng mọi lần khớp, không thừa không thiếu.
        let p = gen_session_record(5_000, 17);
        let mut c = NaiveMaker { tick_offset: 2, has_order: 100,
                                    max_position: 1_000, step: 0, every_n_events: 20 };
        let kq = Replayer::new(LatencyModel::set_custom_tax(), ReplaySpeed::AsFastAsPossible)
            .run(&p, &mut c);
        let dung_lai = kq.all_fill.iter()
            .fold(Position::default(), |a, k| a.compose(Position::from_fill(k.side, k.price, k.quantity)));
        assert_eq!(dung_lai, kq.last_position,
                   "dựng lại vị thế từ nhật ký khớp phải ra đúng vị thế cuối");
        assert_eq!(kq.order_book_fill as usize, kq.all_fill.len());
    }

    #[test]
    fn every_fill_is_valid() {
        let p = gen_session_record(5_000, 13);
        let mut c = NaiveMaker { tick_offset: 2, has_order: 100,
                                    max_position: 1_000, step: 0, every_n_events: 20 };
        let kq = Replayer::new(LatencyModel::set_custom_tax(), ReplaySpeed::AsFastAsPossible)
            .run(&p, &mut c);
        for k in &kq.all_fill {
            assert!(k.quantity > 0, "không được ghi nhận khớp khối lượng 0");
            assert!(k.price > 0);
        }
    }

    #[test]
    fn counting_only_filled_position_breaches_the_cap() {
        // Bài học đắt tiền, và bài kiểm thử này CỐ Ý ghi lại cái sai:
        // `NaiveMaker` chỉ kiểm tra vị thế ĐÃ KHỚP, nên cứ mỗi nhịp lại
        // chào thêm một lệnh nữa. Khi thị trường quét qua, tất cả khớp một
        // lượt và vị thế nhảy vọt qua trần.
        let p = gen_session_record(10_000, 23);
        let tran = 300i64;
        let mut c = NaiveMaker { tick_offset: 1, has_order: 100,
                                    max_position: tran, step: 0, every_n_events: 5 };
        let kq = Replayer::new(LatencyModel::set_custom_tax(), ReplaySpeed::AsFastAsPossible)
            .run(&p, &mut c);
        assert!(kq.last_position.quantity.abs() > tran,
                "chính vì bỏ qua lệnh đang treo mà vị thế {} vượt trần {}",
                kq.last_position.quantity, tran);
    }

    #[test]
    fn counting_resting_orders_keeps_the_cap() {
        // Bản đúng: phơi bày = vị thế đã khớp + khối lượng đang treo.
        let p = gen_session_record(10_000, 23);
        let tran = 300i64;
        let mut c = ManagedMaker::new(1, 100, tran, 5);
        let kq = Replayer::new(LatencyModel::set_custom_tax(), ReplaySpeed::AsFastAsPossible)
            .run(&p, &mut c);
        assert!(kq.last_position.quantity.abs() <= tran,
                "vị thế cuối {} phải nằm trong trần {}", kq.last_position.quantity, tran);
        assert!(kq.orders_sent > 0, "vẫn phải deliver dịch được, không phải đứng im");
    }

    #[test]
    fn inventory_control_holds_for_every_seed() {
        for hat in [1u64, 7, 23, 42, 2024] {
            let p = gen_session_record(8_000, hat);
            let tran = 200i64;
            let mut c = ManagedMaker::new(1, 100, tran, 5);
            let kq = Replayer::new(LatencyModel::set_custom_tax(), ReplaySpeed::AsFastAsPossible)
                .run(&p, &mut c);
            assert!(kq.last_position.quantity.abs() <= tran,
                    "hạt giống {}: vị thế {} vượt trần {}", hat, kq.last_position.quantity, tran);
        }
    }

    #[test]
    fn strategy_stands_aside_on_a_crossed_book() {
        let mut s = ReducedBook::default();
        s.them(Side::Buy, 8_500, 100);
        s.them(Side::Sell, 8_400, 100);
        let mut c = NaiveMaker { tick_offset: 2, has_order: 100,
                                    max_position: 500, step: 0, every_n_events: 1 };
        let order = c.when_has_event(&VirtualClock::new(0), &s, &Position::default());
        assert!(order.is_empty(), "sổ chéo → phải đứng ngoài, không được coi là cơ hội");
    }

    #[test]
    fn strategy_sends_nothing_on_an_empty_book() {
        let mut c = NaiveMaker { tick_offset: 2, has_order: 100,
                                    max_position: 500, step: 0, every_n_events: 1 };
        assert!(c.when_has_event(&VirtualClock::new(0), &ReducedBook::default(),
                                 &Position::default()).is_empty());
    }

    // ---------- Sinh phiên ----------
    #[test]
    fn gen_session_all_peak_and_time_time_up() {
        assert_eq!(gen_session_record(100, 5), gen_session_record(100, 5));
        assert_ne!(gen_session_record(100, 5), gen_session_record(100, 6));
        let p = gen_session_record(1_000, 1);
        for w in p.windows(2) {
            assert!(w[1].timestamp_nanos > w[0].timestamp_nanos);
        }
    }

    #[test]
    fn session_gen_out_has_data_three_event_kind() {
        let p = gen_session_record(5_000, 3);
        let them = p.iter()
            .filter(|k| matches!(k.event, EventMarket::AddOrder { .. })).count();
        let cancel = p.iter()
            .filter(|k| matches!(k.event, EventMarket::CancelOrder { .. })).count();
        let fill = p.iter()
            .filter(|k| matches!(k.event, EventMarket::Fill { .. })).count();
        assert!(them > 0 && cancel > 0 && fill > 0, "phiên phải có cả ba loại sự kiện");
        assert_eq!(them + cancel + fill, p.len());
        assert!(them > fill, "thực tế: đặt lệnh nhiều hơn khớp lệnh rất nhiều");
    }
}
```

---

## Bảng tra cứu lỗi biên dịch thường gặp

| Lỗi | Nguyên nhân trong chương này | Cách sửa |
|---|---|---|
| Phát lại ra kết quả khác mỗi lần | Duyệt `HashMap` khi cấp phát khớp | Đổi sang `BTreeMap` — bắt buộc cho tính tất định |
| `E0502: cannot borrow as mutable` | Duyệt `self.market_orders` rồi muốn sửa | Thu thay đổi vào `Vec` cục bộ, áp dụng sau vòng lặp |
| `E0499: two mutable borrows` | `self.so_lenh` và `self.strategy` cùng lúc | Tách thành hàm nhận hai `&mut` riêng, hoặc `split_at_mut` |
| Sổ lệnh chéo vĩnh viễn | Bộ phát lại bỏ qua thông điệp huỷ | Xử lý **mọi** loại thông điệp, không chọn lọc |
| Vị thế vượt hạn mức | Chỉ đếm vị thế đã khớp | Cộng cả phơi nhiễm từ lệnh đang treo |

---

## Tóm tắt chương & Bài tập rèn luyện

### 5 điểm cốt lõi

1. **Backtest trên nến là nói dối.** Phục dựng theo thông điệp là cách duy nhất biết mình có được khớp hay không.
2. **Đồng hồ ảo (Virtual clock) phải là nguồn thời gian duy nhất.** Một lời gọi `Instant::now()` lạc lõng là đủ phá cả hệ thống.
3. **Bỏ qua độ trễ là nhìn trộm tương lai.** Và nó là dạng tinh vi nhất, vì không ai gọi tên nó như vậy.
4. **`HashMap` phá tính tất định.** Đây là lỗi có thật đã xảy ra ngay trong chương này.
5. **Hạn mức tồn kho phải tính cả lệnh đang treo.** Đếm thiếu thì vị thế vượt hạn mức gấp ba lần.

### Bài tập rèn luyện

**Bài 1.** Thêm **mô hình tác động thị trường**: lệnh lớn của bạn tự làm dịch giá.

<details>
<summary><b>Gợi ý</b></summary>

Có hai loại tác động. **Tạm thời** — bạn ăn qua vài mức giá của sổ, rồi sổ hồi lại. **Vĩnh viễn** — thị trường suy ra bạn biết điều gì đó và điều chỉnh theo. Mô hình kinh điển cho tác động tạm thời là căn bậc hai: tác động tỉ lệ với √(khối lượng / khối lượng ngày).
</details>

<details>
<summary><b>Lời giải</b></summary>

```rust
pub struct ImpactModel {
    /// Hệ số cho phần tạm thời, thường 0,1–1,0 tuỳ thị trường.
    pub temporary_coef: f64,
    /// Phần tác động ở lại vĩnh viễn, thường 0,3–0,5.
    pub ty_le_vinh_vien: f64,
    pub daily_volume: f64,
}

impl ImpactModel {
    /// Quy luật căn bậc hai — chuẩn công nghiệp cho tác động tạm thời.
    pub fn impact_bps(&self, quantity: u64) -> f64 {
        if self.daily_volume <= 0.0 { return 0.0; }
        let ratio = quantity as f64 / self.daily_volume;
        self.temporary_coef * ratio.sqrt() * 10_000.0
    }

    /// Giá khớp thực tế sau khi tính tác động.
    pub fn fill_price(&self, gia_yet: Price, side: Side, quantity: u64) -> Price {
        let bp = self.impact_bps(quantity);
        let dich = gia_yet as f64 * bp / 10_000.0;
        match side {
            Side::Buy => (gia_yet as f64 + dich) as Price,   // bid thì đẩy giá lên
            Side::Sell => (gia_yet as f64 - dich) as Price,
        }
    }

    /// Phần tác động KHÔNG hồi lại — cái này mới thực sự đắt.
    pub fn dich_vinh_vien(&self, gia_yet: Price, quantity: u64) -> f64 {
        gia_yet as f64 * self.impact_bps(quantity)
            / 10_000.0 * self.ty_le_vinh_vien
    }
}
```

Quy luật căn bậc hai có hệ quả quan trọng: chia nhỏ lệnh **giảm** tổng tác động, vì √(4x) = 2√x chứ không phải 4√x. Đó là toàn bộ cơ sở toán học của các thuật toán thực thi kiểu VWAP và TWAP.
</details>

**Bài 2.** Cài **phát lại có kiểm tra tính tất định**: chạy hai lần và khẳng định kết quả trùng khớp bit-với-bit.

<details>
<summary><b>Gợi ý</b></summary>

Đây là bài kiểm thử quan trọng nhất của cả hệ thống phục dựng. Cách làm: băm toàn bộ chuỗi sự kiện đầu ra của mỗi lần chạy, rồi so hai giá trị băm. Nếu khác nhau, ở đâu đó có nguồn bất định — `HashMap`, số ngẫu nhiên chưa gieo hạt, hoặc `Instant::now()` lọt lưới.
</details>

<details>
<summary><b>Lời giải</b></summary>

```rust
/// Băm FNV-1a — đủ tốt để phát hiện khác biệt, đủ nhanh để chạy mọi lần.
fn bam_ket_qua(cac_lenh: &[(u64, Side, Price, u64)]) -> u64 {
    let mut h: u64 = 0xcbf29ce484222325;
    for (t, c, g, kl) in cac_lenh {
        for b in t.to_le_bytes().iter()
            .chain(&[*c as u8])
            .chain(g.to_le_bytes().iter())
            .chain(kl.to_le_bytes().iter())
        {
            h ^= *b as u64;
            h = h.wrapping_mul(0x100000001b3);
        }
    }
    h
}

#[test]
fn replay_must_be_deterministic() {
    let session = generate_session(20_000, 42);

    let run = || {
        let mut bpl = Replayer::new(session.clone(), ReplaySpeed::Unbounded,
                                     LatencyModel::typical());
        let mut cl = ManagedMaker::new(300);
        bpl.run(&mut cl);
        bam_ket_qua(&cl.nhat_ky_lenh)
    };

    let a = run();
    let b = run();
    assert_eq!(a, b, "phát lại KHÔNG tất định — kiểm HashMap, RNG, Instant::now()");
}
```

Nếu bài kiểm thử này trượt, đừng sửa bài kiểm thử — hãy đi tìm nguồn bất định. Ba nghi phạm theo thứ tự khả năng: duyệt `HashMap`/`HashSet`, bộ sinh ngẫu nhiên không gieo hạt cố định, và lời gọi đồng hồ thật.
</details>
