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

**Đây là chương có nhiều lỗi thật nhất trong cả bộ sách.** Năm lỗi lớn đã bị phát hiện và sửa trong quá trình xây dựng, và mỗi lỗi đều là một bài học về lý do phục dựng khó.

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

### 1. Năm lỗi thật đã gặp khi xây chương này

Đây không phải lỗi bịa ra để dạy học. Chúng xuất hiện thật, và mỗi lỗi đều được phát hiện bằng cách **chạy** chứ không phải bằng cách đọc.

**Lỗi 1 — Bộ phát lại bỏ qua lệnh huỷ.** Bản đầu chỉ xử lý "thêm lệnh". Kết quả: sổ lệnh chỉ lớn lên, và sau vài nghìn sự kiện thì mua vượt bán vĩnh viễn — sổ "chéo" mãi mãi. Trong 20 000 sự kiện, chiến lược chỉ gửi được **2 lệnh**. Bài học: một bộ phát lại thiếu một loại thông điệp không phải là "gần đúng", nó là **sai hoàn toàn**.

**Lỗi 2 — Bộ sinh dữ liệu huỷ lệnh theo ID ngẫu nhiên.** Sau khi sửa lỗi 1, sổ vẫn chéo. Nguyên nhân: bộ sinh chọn ID ngẫu nhiên để huỷ, mà phần lớn ID đó đã chết rồi. Các báo giá cũ vẫn nằm lại. Sửa bằng cách huỷ **lệnh còn sống cũ nhất** và giới hạn số lệnh sống ở 120 — mô phỏng đúng hành vi thật của nhà tạo lập.

**Lỗi 3 — `HashMap` phá tính tất định.** Khi cấp phát khớp lệnh, mã duyệt một `HashMap`. Thứ tự duyệt của `HashMap` trong Rust thay đổi giữa các lần chạy (do RandomState), nên phát lại **không tái lập được**. Đây là điều mỉa mai nhất: chính chương dạy về tính tất định lại vi phạm nó. Sửa: dùng `BTreeMap`.

**Lỗi 4 — Nhà tạo lập vượt hạn mức tồn kho.** Bản `NaiveMaker` chỉ đếm vị thế **đã khớp**. Nhưng lệnh đang treo cũng là rủi ro. Kết quả: trong phiên mẫu, |vị thế| chạm tới 5 600 dù hạn mức là 300. Sửa bằng `ManagedMaker` — theo dõi cả phơi nhiễm đang chờ. Bản sai được **giữ lại** làm ví dụ phản chứng có kiểm thử. Lưu ý cách kiểm: phải đo |vị thế| **lớn nhất trong cả phiên** (`peak_abs_position`), không chỉ vị thế cuối — vị thế cuối nằm trong trần không chứng minh được gì nếu giữa phiên đã vượt.

**Lỗi 5 — Bộ sinh dữ liệu tự tạo ra sổ chéo.** Sau khi sửa lỗi 1 và 2, sổ phát lại *vẫn* chéo ở gần 90% số sự kiện — chỉ là không ai đo. Bộ sinh đặt lệnh quanh một giá giữa trôi ngẫu nhiên, nên lệnh mua mới có thể nằm **trên** lệnh bán cũ còn treo; và các bản tin khớp "ăn" ở giá giữa ± 1 dù ở đó không có thanh khoản nào. Trên sàn thật cả hai điều đó không thể xảy ra: lệnh vượt giá bên kia sẽ khớp ngay. Sửa: bộ sinh tự giữ một sổ lệnh, không bao giờ đặt lệnh vượt giá tốt nhất bên kia, và mỗi lần khớp ăn đúng mức giá tốt nhất đang có với khối lượng không vượt quá thực tế. Kiểm thử `replayed_book_is_never_crossed` canh bất biến này.

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
//! Chương 76 — Ghi & Phát lại phiên giao dịch: định dạng bản ghi, đồng hồ ảo,
//! phát lại đúng dòng thời gian hoặc tua nhanh, mô hình độ trễ, và mô phỏng
//! khớp lệnh có xét vị trí hàng đợi.
//!
//! Đây là "phòng thí nghiệm" của mọi đội giao dịch nghiêm túc: ghi lại phiên
//! thật một lần, rồi chạy lại hàng nghìn lần với các chiến lược khác nhau,
//! kết quả TÁI LẬP TUYỆT ĐỐI.

use std::collections::BTreeMap;

// ============================================================================
// 1. ĐỊNH DẠNG BẢN GHI — khung có tiền tố độ dài
// ============================================================================
// Mỗi khung: [độ dài u32 BE][thời điểm ns u64 BE][thân bản tin].
// Tiền tố độ dài cho phép đọc tuần tự mà không cần phân tích thân — nên bộ
// ghi có thể lưu BẤT KỲ giao thức nào mà không cần hiểu nó.

pub type Price = i64;
pub type Quantity = u32;
pub type OrderId = u64;

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum Side {
    Buy,
    Sell,
}

impl Side {
    pub fn inverse(self) -> Side {
        match self {
            Side::Buy => Side::Sell,
            Side::Sell => Side::Buy,
        }
    }
}

#[derive(Debug, Clone, PartialEq)]
pub enum MarketEvent {
    AddOrder {
        id: OrderId,
        side: Side,
        price: Price,
        quantity: Quantity,
    },
    CancelOrder {
        id: OrderId,
    },
    Fill {
        price: Price,
        quantity: Quantity,
        side_aggressive: Side,
    },
}

#[derive(Debug, Clone, PartialEq)]
pub struct RecordedFrame {
    /// Nano-giây kể từ mốc bắt đầu phiên. KHÔNG dùng ngày lịch — múi giờ,
    /// giờ mùa hè và giây nhuận đều là nguồn lỗi không đáng chuốc vào.
    pub timestamp_nanos: u64,
    pub event: MarketEvent,
}

#[derive(Debug, PartialEq)]
pub enum ReadError {
    TruncatedFrame,
    InvalidLength(u32),
    UnknownEventCode(u8),
}

/// Bộ ghi phiên. Trong hệ thống thật, `content` được xả xuống đĩa theo lô;
/// ở đây giữ trong bộ nhớ để kiểm thử được.
#[derive(Debug, Default)]
pub struct SessionRecorder {
    pub content: Vec<u8>,
    pub num_frames: u64,
    pub first_timestamp: Option<u64>,
    pub last_timestamp: u64,
}

impl SessionRecorder {
    pub fn new() -> Self {
        SessionRecorder::default()
    }

    pub fn record(&mut self, k: &RecordedFrame) {
        let body = encode_event(&k.event);
        let length = (8 + body.len()) as u32;
        self.content.extend_from_slice(&length.to_be_bytes());
        self.content
            .extend_from_slice(&k.timestamp_nanos.to_be_bytes());
        self.content.extend_from_slice(&body);
        self.num_frames += 1;
        if self.first_timestamp.is_none() {
            self.first_timestamp = Some(k.timestamp_nanos);
        }
        self.last_timestamp = k.timestamp_nanos;
    }

    pub fn duration_nanos(&self) -> u64 {
        self.last_timestamp - self.first_timestamp.unwrap_or(0)
    }
    pub fn byte_len(&self) -> usize {
        self.content.len()
    }

    /// Đọc lại toàn bộ. Trả lỗi nếu bản ghi bị cắt cụt — chuyện thường gặp khi
    /// tiến trình ghi bị giết giữa chừng, và phải xử lý được chứ không panic.
    pub fn read_back(&self) -> Result<Vec<RecordedFrame>, ReadError> {
        let mut out = Vec::new();
        let b = &self.content;
        let mut i = 0usize;
        while i < b.len() {
            if i + 4 > b.len() {
                return Err(ReadError::TruncatedFrame);
            }
            let length = u32::from_be_bytes(b[i..i + 4].try_into().unwrap()) as usize;
            if length < 8 {
                return Err(ReadError::InvalidLength(length as u32));
            }
            if i + 4 + length > b.len() {
                return Err(ReadError::TruncatedFrame);
            }
            let timestamp_nanos = u64::from_be_bytes(b[i + 4..i + 12].try_into().unwrap());
            let event = decode_event(&b[i + 12..i + 4 + length])?;
            out.push(RecordedFrame {
                timestamp_nanos,
                event,
            });
            i += 4 + length;
        }
        Ok(out)
    }
}

fn encode_event(event: &MarketEvent) -> Vec<u8> {
    let mut v = Vec::with_capacity(24);
    match event {
        MarketEvent::AddOrder {
            id,
            side,
            price,
            quantity,
        } => {
            v.push(b'A');
            v.extend_from_slice(&id.to_be_bytes());
            v.push(if *side == Side::Buy { b'B' } else { b'S' });
            v.extend_from_slice(&price.to_be_bytes());
            v.extend_from_slice(&quantity.to_be_bytes());
        }
        MarketEvent::CancelOrder { id } => {
            v.push(b'X');
            v.extend_from_slice(&id.to_be_bytes());
        }
        MarketEvent::Fill {
            price,
            quantity,
            side_aggressive,
        } => {
            v.push(b'T');
            v.extend_from_slice(&price.to_be_bytes());
            v.extend_from_slice(&quantity.to_be_bytes());
            v.push(if *side_aggressive == Side::Buy {
                b'B'
            } else {
                b'S'
            });
        }
    }
    v
}

fn decode_event(b: &[u8]) -> Result<MarketEvent, ReadError> {
    if b.is_empty() {
        return Err(ReadError::TruncatedFrame);
    }
    let needed = match b[0] {
        b'A' => 22,
        b'X' => 9,
        b'T' => 14,
        x => return Err(ReadError::UnknownEventCode(x)),
    };
    if b.len() < needed {
        return Err(ReadError::TruncatedFrame);
    }
    Ok(match b[0] {
        b'A' => MarketEvent::AddOrder {
            id: u64::from_be_bytes(b[1..9].try_into().unwrap()),
            side: if b[9] == b'B' { Side::Buy } else { Side::Sell },
            price: i64::from_be_bytes(b[10..18].try_into().unwrap()),
            quantity: u32::from_be_bytes(b[18..22].try_into().unwrap()),
        },
        b'X' => MarketEvent::CancelOrder {
            id: u64::from_be_bytes(b[1..9].try_into().unwrap()),
        },
        _ => MarketEvent::Fill {
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
pub struct VirtualClock {
    pub now_ns: u64,
}

impl VirtualClock {
    pub fn new(start: u64) -> Self {
        VirtualClock { now_ns: start }
    }
    pub fn advance(&mut self, ns: u64) {
        if ns > self.now_ns {
            self.now_ns = ns;
        }
    }
    pub fn advance_by(&mut self, ns: u64) {
        self.now_ns += ns;
    }
}

/// Tốc độ phát lại.
#[derive(Debug, Clone, Copy, PartialEq)]
pub enum ReplaySpeed {
    /// Đúng nhịp thật: giữ nguyên khoảng cách giữa các sự kiện.
    RealTime,
    /// Nhân tốc độ: 2.0 = nhanh gấp đôi, 0.5 = chậm một nửa (để quan sát kỹ).
    Factor(f64),
    /// Bỏ hẳn thời gian chờ — dùng khi quét tham số hàng nghìn lần.
    AsFastAsPossible,
}

impl ReplaySpeed {
    /// Thời gian THỰC (nano-giây) phải chờ, ứng với `khoang_cach_ns` trong dữ liệu.
    pub fn wall_delay(&self, gap_ns: u64) -> u64 {
        match self {
            ReplaySpeed::RealTime => gap_ns,
            ReplaySpeed::Factor(h) if *h > 0.0 => (gap_ns as f64 / h) as u64,
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
    pub fn no_latency() -> Self {
        LatencyModel {
            in_nanos: 0,
            out_nanos: 0,
            jitter_ns: 0,
        }
    }
    pub fn colocated() -> Self {
        LatencyModel {
            in_nanos: 5_000,
            out_nanos: 8_000,
            jitter_ns: 2_000,
        }
    }
    pub fn over_internet() -> Self {
        LatencyModel {
            in_nanos: 8_000_000,
            out_nanos: 12_000_000,
            jitter_ns: 5_000_000,
        }
    }

    /// Tổng thời gian từ lúc SÀN phát tin tới lúc lệnh của ta ĐẾN SÀN.
    /// Đây chính là "tick-to-trade" mà Chương 74 mổ xẻ.
    pub fn round_trip_ns(&self, nonce: u64) -> u64 {
        // Dao động tất định: cùng chuỗi sự kiện luôn cho cùng độ trễ
        let d = if self.jitter_ns == 0 {
            0
        } else {
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
    bids: BTreeMap<Price, u64>, // khoá ÂM → giá cao nhất trước
    asks: BTreeMap<Price, u64>,
}

impl ReducedBook {
    pub fn add(&mut self, c: Side, g: Price, kl: u64) {
        let (bd, k) = match c {
            Side::Buy => (&mut self.bids, -g),
            Side::Sell => (&mut self.asks, g),
        };
        *bd.entry(k).or_insert(0) += kl;
    }
    pub fn reduce(&mut self, c: Side, g: Price, kl: u64) {
        let (bd, k) = match c {
            Side::Buy => (&mut self.bids, -g),
            Side::Sell => (&mut self.asks, g),
        };
        if let Some(v) = bd.get_mut(&k) {
            *v = v.saturating_sub(kl);
            if *v == 0 {
                bd.remove(&k);
            }
        }
    }
    pub fn best_bid(&self) -> Option<Price> {
        self.bids.keys().next().map(|k| -k)
    }
    pub fn best_ask(&self) -> Option<Price> {
        self.asks.keys().next().copied()
    }
    pub fn qty_at(&self, c: Side, g: Price) -> u64 {
        let (bd, k) = match c {
            Side::Buy => (&self.bids, -g),
            Side::Sell => (&self.asks, g),
        };
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
    pub queue_ahead: u64,
    pub arrives_at_venue_nanos: u64,
}

impl OurOrder {
    pub fn remaining(&self) -> Quantity {
        self.quantity - self.filled
    }
    pub fn is_filled(&self) -> bool {
        self.filled >= self.quantity
    }
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
pub struct Position {
    pub quantity: i64,
    pub cash: i64,
}

impl Position {
    pub fn compose(self, k: Position) -> Position {
        Position {
            quantity: self.quantity + k.quantity,
            cash: self.cash + k.cash,
        }
    }
    pub fn from_fill(c: Side, g: Price, qty: Quantity) -> Position {
        let first = if c == Side::Buy { 1 } else { -1 };
        Position {
            quantity: first * qty as i64,
            cash: -first * g * qty as i64,
        }
    }
    pub fn net_value(&self, mark_price: Price) -> i64 {
        self.cash + self.quantity * mark_price
    }
}

/// Chiến lược nhìn thấy gì và làm gì. Thuần tuý: cùng đầu vào → cùng đầu ra.
pub trait ReplayStrategy {
    fn name(&self) -> &str;
    /// Gọi sau MỖI sự kiện thị trường. Trả về các lệnh muốn gửi.
    fn on_event(
        &mut self,
        clock: &VirtualClock,
        book: &ReducedBook,
        position: &Position,
    ) -> Vec<(Side, Price, Quantity)>;
    /// Gọi khi một lệnh của ta được khớp.
    fn on_fill(&mut self, _k: &OurFill) {}
}

// ============================================================================
// 6. BỘ PHÁT LẠI
// ============================================================================

#[derive(Debug, PartialEq)]
pub struct ReplayResult {
    pub event_count: u64,
    pub orders_sent: u64,
    pub filled_orders: u64,
    pub fills: Vec<OurFill>,
    pub last_position: Position,
    /// |vị thế| lớn nhất từng chạm trong phiên — thứ hạn mức rủi ro thật sự
    /// quan tâm. Vị thế CUỐI phiên có thể nằm trong trần dù giữa phiên đã vượt.
    pub peak_abs_position: i64,
    pub last_value: i64,
    /// Tổng thời gian ẢO đã trôi qua.
    pub virtual_time_nanos: u64,
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

    /// Chạy lại phiên. Toàn bộ là hàm THUẦN TUÝ trên `frames` — không đọc
    /// đồng hồ hệ thống, không đọc tệp, không ngẫu nhiên.
    pub fn run(&self, frames: &[RecordedFrame], strategy: &mut dyn ReplayStrategy) -> ReplayResult {
        let mut book = ReducedBook::default();
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
        let mut clock = VirtualClock::new(frames.first().map_or(0, |k| k.timestamp_nanos));
        let mut resting_orders: Vec<OurOrder> = Vec::new();
        let mut fills = Vec::new();
        let mut position = Position::default();
        let mut peak_abs_position = 0i64;
        let mut next_id = 1u64;
        let mut orders_sent = 0u64;
        let mut real_wait = 0u64;
        let mut last_price: Price = 0;
        let mut prev_ts = clock.now_ns;

        for (i, k) in frames.iter().enumerate() {
            real_wait += self
                .speed
                .wall_delay(k.timestamp_nanos.saturating_sub(prev_ts));
            prev_ts = k.timestamp_nanos;
            clock.advance(k.timestamp_nanos);

            // --- Lệnh nào vừa "bay tới sàn" thì chốt vị trí hàng đợi NGAY LÚC ĐÓ,
            //     không phải lúc ta quyết định. Đây là chi tiết quyết định tính
            //     thực tế của toàn bộ mô phỏng.
            for l in resting_orders.iter_mut() {
                if l.arrives_at_venue_nanos <= clock.now_ns && l.queue_ahead == u64::MAX {
                    l.queue_ahead = book.qty_at(l.side, l.price);
                }
            }

            // --- Áp dụng sự kiện thị trường ---
            match &k.event {
                MarketEvent::AddOrder {
                    id,
                    side,
                    price,
                    quantity,
                } => {
                    book.add(*side, *price, *quantity as u64);
                    market_orders.insert(*id, (*side, *price, *quantity as u64));
                }
                MarketEvent::CancelOrder { id } => {
                    if let Some((c, g, kl)) = market_orders.remove(id) {
                        book.reduce(c, g, kl);
                    }
                }
                MarketEvent::Fill {
                    price,
                    quantity,
                    side_aggressive,
                } => {
                    last_price = *price;
                    // Lệnh khớp ăn vào bên THỤ ĐỘNG
                    let passive_side = side_aggressive.inverse();
                    book.reduce(passive_side, *price, *quantity as u64);
                    // Khớp cũng làm cạn lệnh thị trường ở mức giá đó
                    let mut to_consume = *quantity as u64;
                    let mut to_remove: Vec<OrderId> = Vec::new();
                    for (m, (c, g, kl)) in market_orders.iter_mut() {
                        if to_consume == 0 {
                            break;
                        }
                        if *c != passive_side || *g != *price {
                            continue;
                        }
                        let take = to_consume.min(*kl);
                        *kl -= take;
                        to_consume -= take;
                        if *kl == 0 {
                            to_remove.push(*m);
                        }
                    }
                    for m in to_remove {
                        market_orders.remove(&m);
                    }

                    // Lệnh của ta cùng bên thụ động, cùng giá thì có thể tới lượt
                    let mut left = *quantity as u64;
                    for l in resting_orders.iter_mut() {
                        if left == 0 {
                            break;
                        }
                        if l.is_filled() || l.side != passive_side || l.price != *price {
                            continue;
                        }
                        if l.arrives_at_venue_nanos > clock.now_ns {
                            continue;
                        }
                        // Trước hết phải "ăn" hết phần đứng trước ta
                        let ahead_consumed = left.min(l.queue_ahead);
                        l.queue_ahead -= ahead_consumed;
                        left -= ahead_consumed;
                        if l.queue_ahead > 0 || left == 0 {
                            continue;
                        }
                        // Giờ mới tới lượt ta
                        let fill = left.min(l.remaining() as u64) as Quantity;
                        if fill > 0 {
                            l.filled += fill;
                            left -= fill as u64;
                            let result = OurFill {
                                order_id: l.id,
                                side: l.side,
                                price: *price,
                                quantity: fill,
                                timestamp_nanos: clock.now_ns,
                            };
                            position = position.compose(Position::from_fill(l.side, *price, fill));
                            peak_abs_position = peak_abs_position.max(position.quantity.abs());
                            strategy.on_fill(&result);
                            fills.push(result);
                        }
                    }
                }
            }

            // --- Chiến lược quyết định ---
            for (side, price, qty) in strategy.on_event(&clock, &book, &position) {
                if qty == 0 {
                    continue;
                }
                resting_orders.push(OurOrder {
                    id: next_id,
                    side,
                    price,
                    quantity: qty,
                    filled: 0,
                    queue_ahead: u64::MAX, // chốt sau, lúc tới sàn
                    arrives_at_venue_nanos: clock.now_ns + self.latency.round_trip_ns(i as u64),
                });
                next_id += 1;
                orders_sent += 1;
            }
            resting_orders.retain(|l| !l.is_filled());
        }

        ReplayResult {
            event_count: frames.len() as u64,
            orders_sent,
            filled_orders: fills.len() as u64,
            last_value: position.net_value(last_price),
            last_position: position,
            peak_abs_position,
            fills,
            virtual_time_nanos: frames.last().map_or(0, |k| k.timestamp_nanos)
                - frames.first().map_or(0, |k| k.timestamp_nanos),
            real_wait_nanos: real_wait,
        }
    }
}

// ============================================================================
// 7. CHIẾN LƯỢC MẪU
// ============================================================================

/// Tạo lập thị trường: đặt lệnh mua dưới và bán trên giá giữa, ăn chênh lệch.
pub struct NaiveMaker {
    pub tick_offset: Price,
    pub order_size: Quantity,
    pub max_position: i64,
    pub step: u64,
    pub every_n_events: u64,
}

impl ReplayStrategy for NaiveMaker {
    fn name(&self) -> &str {
        "Tạo lập thị trường đơn giản"
    }

    fn on_event(
        &mut self,
        _clock: &VirtualClock,
        book: &ReducedBook,
        pos: &Position,
    ) -> Vec<(Side, Price, Quantity)> {
        self.step += 1;
        if !self.step.is_multiple_of(self.every_n_events) {
            return vec![];
        }
        let (m, b) = match (book.best_bid(), book.best_ask()) {
            (Some(m), Some(b)) => (m, b),
            _ => return vec![],
        };
        if b <= m {
            return vec![];
        } // sổ chéo hoặc khoá → đứng ngoài
        let mid = (m + b) / 2;
        let mut out = Vec::new();
        // Kiểm soát tồn kho: đã ôm nhiều thì thôi mua thêm
        if pos.quantity < self.max_position {
            out.push((Side::Buy, mid - self.tick_offset, self.order_size));
        }
        if pos.quantity > -self.max_position {
            out.push((Side::Sell, mid + self.tick_offset, self.order_size));
        }
        out
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
    pub order_size: Quantity,
    pub max_position: i64,
    pub step: u64,
    pub every_n_events: u64,
    resting_bid: i64,
    resting_ask: i64,
}

impl ManagedMaker {
    pub fn new(
        tick_offset: Price,
        order_size: Quantity,
        max_position: i64,
        every_n_events: u64,
    ) -> Self {
        ManagedMaker {
            tick_offset,
            order_size,
            max_position,
            step: 0,
            every_n_events,
            resting_bid: 0,
            resting_ask: 0,
        }
    }
    pub fn pending_exposure(&self) -> (i64, i64) {
        (self.resting_bid, self.resting_ask)
    }
}

impl ReplayStrategy for ManagedMaker {
    fn name(&self) -> &str {
        "Tạo lập có kiểm soát tồn kho"
    }

    fn on_event(
        &mut self,
        _clock: &VirtualClock,
        book: &ReducedBook,
        pos: &Position,
    ) -> Vec<(Side, Price, Quantity)> {
        self.step += 1;
        if !self.step.is_multiple_of(self.every_n_events) {
            return vec![];
        }
        let (m, b) = match (book.best_bid(), book.best_ask()) {
            (Some(m), Some(b)) => (m, b),
            _ => return vec![],
        };
        if b <= m {
            return vec![];
        }
        let mid = (m + b) / 2;
        let size = self.order_size as i64;
        let mut out = Vec::new();
        // PHƠI BÀY = vị thế đã khớp + toàn bộ khối lượng đang treo cùng chiều
        if pos.quantity + self.resting_bid + size <= self.max_position {
            out.push((Side::Buy, mid - self.tick_offset, self.order_size));
            self.resting_bid += size;
        }
        if pos.quantity - self.resting_ask - size >= -self.max_position {
            out.push((Side::Sell, mid + self.tick_offset, self.order_size));
            self.resting_ask += size;
        }
        out
    }

    fn on_fill(&mut self, k: &OurFill) {
        // Khớp rồi thì phần đó không còn "treo" nữa — nó đã thành vị thế
        match k.side {
            Side::Buy => self.resting_bid = (self.resting_bid - k.quantity as i64).max(0),
            Side::Sell => self.resting_ask = (self.resting_ask - k.quantity as i64).max(0),
        }
    }
}

pub struct SitOut;
impl ReplayStrategy for SitOut {
    fn name(&self) -> &str {
        "Đứng ngoài"
    }
    fn on_event(
        &mut self,
        _: &VirtualClock,
        _: &ReducedBook,
        _: &Position,
    ) -> Vec<(Side, Price, Quantity)> {
        vec![]
    }
}

// ============================================================================
// 8. SINH PHIÊN TẤT ĐỊNH ĐỂ GHI LẠI
// ============================================================================

/// Sinh một phiên tất định. Ba chi tiết quyết định tính THỰC TẾ của nó:
///
/// 1. **Huỷ lệnh nhắm đúng lệnh CŨ NHẤT còn sống.** Thị trường thật rút báo
///    giá cũ liên tục (>90% lệnh bị huỷ trước khi khớp). Nếu huỷ theo mã ngẫu
///    nhiên, phần lớn lệnh huỷ trúng mã đã biến mất, báo giá cũ nằm lại mãi,
///    và sau vài nghìn sự kiện là sổ CHÉO VĨNH VIỄN.
/// 2. **Số lệnh sống bị chặn trần.** Vượt trần thì lệnh cũ nhất bị đẩy ra —
///    mô phỏng đúng việc thanh khoản cũ tan đi khi giá đã đi xa.
/// 3. **Bộ sinh tự giữ một sổ lệnh và tôn trọng nó.** Lệnh mới không bao giờ
///    đặt vượt giá tốt nhất bên kia (trên sàn thật, lệnh như vậy đã khớp ngay),
///    và mỗi lần khớp ăn đúng vào mức giá tốt nhất ĐANG CÓ, với khối lượng
///    không vượt quá khối lượng thật ở đó. Thiếu điều này, giá giữa trôi đi
///    còn báo giá cũ nằm lại, và sổ phát lại bị chéo gần 90% thời gian.
pub fn gen_recorded_session(event_count: usize, seed: u64) -> Vec<RecordedFrame> {
    const MAX_LIVE_ORDERS: usize = 120;
    let mut s = seed;
    let mut t = 9 * 3_600 * 1_000_000_000u64; // 9 giờ sáng, tính bằng ns
    let mut mid: Price = 8_400;
    let mut next_id = 1u64;
    // Sổ của bộ sinh: mã → (chiều, giá, khối lượng còn lại). BTreeMap theo
    // mã = theo thứ tự tới, nên khớp tại một mức giá đi đúng ưu tiên thời gian.
    let mut live: BTreeMap<OrderId, (Side, Price, u64)> = BTreeMap::new();
    let mut age: std::collections::VecDeque<OrderId> = std::collections::VecDeque::new();
    let mut out = Vec::with_capacity(event_count);

    let best = |live: &BTreeMap<OrderId, (Side, Price, u64)>, side: Side| {
        let prices = live.values().filter(|o| o.0 == side).map(|o| o.1);
        match side {
            Side::Buy => prices.max(),
            Side::Sell => prices.min(),
        }
    };

    for _ in 0..event_count {
        s = s
            .wrapping_mul(6364136223846793005)
            .wrapping_add(1442695040888963407);
        t += 10_000 + (s >> 20) % 500_000; // 10 µs – 0,5 ms giữa các sự kiện
        let r = (s >> 33) % 100;
        let side = if (s >> 41).is_multiple_of(2) {
            Side::Buy
        } else {
            Side::Sell
        };
        let qty = 100 + ((s >> 49) % 5) * 100;

        // Lệnh cũ nhất còn sống (bỏ qua những lệnh đã khớp hết).
        while age.front().is_some_and(|id| !live.contains_key(id)) {
            age.pop_front();
        }

        // Quá nhiều lệnh cũ, hoặc bốc trúng nhánh huỷ → rút báo giá CŨ NHẤT.
        let cancel = live.len() >= MAX_LIVE_ORDERS || ((55..85).contains(&r) && !live.is_empty());
        if cancel && let Some(oldest) = age.pop_front() {
            live.remove(&oldest);
            out.push(RecordedFrame {
                timestamp_nanos: t,
                event: MarketEvent::CancelOrder { id: oldest },
            });
            continue;
        }

        // Nhánh khớp: bên chủ động `side` ăn vào mức giá tốt nhất bên kia.
        let passive = side.inverse();
        if r >= 85
            && let Some(price) = best(&live, passive)
        {
            let available: u64 = live
                .values()
                .filter(|o| o.0 == passive && o.1 == price)
                .map(|o| o.2)
                .sum();
            let traded = qty.min(available);
            let mut left = traded;
            live.retain(|_, o| {
                if left > 0 && o.0 == passive && o.1 == price {
                    let take = left.min(o.2);
                    o.2 -= take;
                    left -= take;
                }
                o.2 > 0
            });
            out.push(RecordedFrame {
                timestamp_nanos: t,
                event: MarketEvent::Fill {
                    price,
                    quantity: traded as Quantity,
                    side_aggressive: side,
                },
            });
            // Giá đi lang thang một chút quanh mốc ban đầu
            mid += if (s >> 57).is_multiple_of(2) { 1 } else { -1 };
            mid = mid.clamp(8_350, 8_450);
            continue;
        }

        // Nhánh thêm lệnh (cũng là chỗ rơi vào khi không có gì để huỷ/khớp).
        // Không bao giờ đặt vượt giá tốt nhất bên kia → sổ không bao giờ chéo.
        let offset = 1 + ((s >> 45) % 10) as i64;
        let price = match side {
            Side::Buy => (mid - offset).min(best(&live, Side::Sell).map_or(Price::MAX, |a| a - 1)),
            Side::Sell => (mid + offset).max(best(&live, Side::Buy).map_or(Price::MIN, |b| b + 1)),
        };
        out.push(RecordedFrame {
            timestamp_nanos: t,
            event: MarketEvent::AddOrder {
                id: next_id,
                side,
                price,
                quantity: qty as Quantity,
            },
        });
        live.insert(next_id, (side, price, qty));
        age.push_back(next_id);
        next_id += 1;
    }
    out
}

fn main() {
    println!("═══════════════════════════════════════════════════════════");
    println!("   GHI & PHÁT LẠI PHIÊN GIAO DỊCH                          ");
    println!("═══════════════════════════════════════════════════════════");

    println!("\n1. GHI LẠI MỘT PHIÊN");
    let session = gen_recorded_session(20_000, 2024);
    let mut recorder = SessionRecorder::new();
    for k in &session {
        recorder.record(k);
    }
    println!(
        "   {} sự kiện · {} byte · {:.2} byte/sự kiện",
        recorder.num_frames,
        recorder.byte_len(),
        recorder.byte_len() as f64 / recorder.num_frames as f64
    );
    println!(
        "   Thời lượng phiên: {:.3} giây",
        recorder.duration_nanos() as f64 / 1e9
    );
    let decoded = recorder.read_back().unwrap();
    println!("   Đọc lại khớp bản gốc từng bit: {}", decoded == session);

    println!("\n2. BẢN GHI BỊ CẮT CỤT — phải báo lỗi, không được panic");
    let mut broken = SessionRecorder::new();
    for k in session.iter().take(5) {
        broken.record(k);
    }
    broken.content.truncate(broken.content.len() - 3); // giả lập tiến trình bị giết
    println!("   Đọc bản ghi cụt → {:?}", broken.read_back().unwrap_err());

    println!("\n3. TỐC ĐỘ PHÁT LẠI");
    let mut strategy = SitOut;
    for (name, speed) in [
        ("thời gian thực", ReplaySpeed::RealTime),
        ("nhanh 10 lần  ", ReplaySpeed::Factor(10.0)),
        ("nhanh 1000 lần", ReplaySpeed::Factor(1000.0)),
        ("nhanh nhất    ", ReplaySpeed::AsFastAsPossible),
    ] {
        let result = Replayer::new(LatencyModel::no_latency(), speed).run(&session, &mut strategy);
        println!(
            "   {} → thời gian ảo {:.2}s · phải chờ thật {:.4}s",
            name,
            result.virtual_time_nanos as f64 / 1e9,
            result.real_wait_nanos as f64 / 1e9
        );
    }
    println!(
        "   → Quét 1000 tổ hợp tham số: chạy đúng nhịp mất ~{:.0} phút,",
        recorder.duration_nanos() as f64 / 1e9 * 1000.0 / 60.0
    );
    println!("     chạy ở chế độ nhanh nhất chỉ mất vài giây.");

    println!("\n4. ĐỘ TRỄ ĂN MẤT LỢI NHUẬN NHƯ THẾ NÀO");
    for (name, latency) in [
        ("không độ trễ  ", LatencyModel::no_latency()),
        ("đặt thuê riêng", LatencyModel::colocated()),
        ("qua Internet  ", LatencyModel::over_internet()),
    ] {
        let mut strategy = NaiveMaker {
            tick_offset: 2,
            order_size: 100,
            max_position: 500,
            step: 0,
            every_n_events: 50,
        };
        let result =
            Replayer::new(latency, ReplaySpeed::AsFastAsPossible).run(&session, &mut strategy);
        println!(
            "   {} → khứ hồi {:>9} ns · gửi {:>4} lệnh · khớp {:>3} · lãi {:>8} tick",
            name,
            latency.round_trip_ns(0),
            result.orders_sent,
            result.filled_orders,
            result.last_value
        );
    }
    println!("   → Cùng chiến lược, cùng dữ liệu. Chỉ khác chỗ ngồi so với sàn.");

    println!("\n5. KIỂM SOÁT TỒN KHO — đếm cả lệnh ĐANG TREO");
    let cap = 300i64;
    let mut naive = NaiveMaker {
        tick_offset: 1,
        order_size: 100,
        max_position: cap,
        step: 0,
        every_n_events: 5,
    };
    let a = Replayer::new(LatencyModel::colocated(), ReplaySpeed::AsFastAsPossible)
        .run(&session, &mut naive);
    let mut managed = ManagedMaker::new(1, 100, cap, 5);
    let b = Replayer::new(LatencyModel::colocated(), ReplaySpeed::AsFastAsPossible)
        .run(&session, &mut managed);
    println!("   Trần đặt ra: {}", cap);
    println!(
        "   Chỉ nhìn vị thế đã khớp → |vị thế| lớn nhất {:>6}  ← VƯỢT TRẦN",
        a.peak_abs_position
    );
    println!(
        "   Đếm cả lệnh đang treo   → |vị thế| lớn nhất {:>6}  ← trong trần",
        b.peak_abs_position
    );
    println!("   → Lệnh đã gửi mà chưa khớp VẪN LÀ RỦI RO.");

    println!("\n6. TÁI LẬP TUYỆT ĐỐI");
    let run = || {
        let mut c = NaiveMaker {
            tick_offset: 2,
            order_size: 100,
            max_position: 500,
            step: 0,
            every_n_events: 50,
        };
        Replayer::new(LatencyModel::colocated(), ReplaySpeed::AsFastAsPossible)
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

    fn record_session(n: usize, h: u64) -> (Vec<RecordedFrame>, SessionRecorder) {
        let p = gen_recorded_session(n, h);
        let mut g = SessionRecorder::new();
        for k in &p {
            g.record(k);
        }
        (p, g)
    }

    // ---------- Định dạng bản ghi ----------
    #[test]
    fn record_then_read_matches_bit_for_bit() {
        let (p, g) = record_session(2_000, 1);
        assert_eq!(
            g.read_back().unwrap(),
            p,
            "vòng ghi–đọc phải khép kín tuyệt đối"
        );
        assert_eq!(g.num_frames, 2_000);
    }

    #[test]
    fn every_event_kind_round_trips() {
        let all = vec![
            MarketEvent::AddOrder {
                id: 1,
                side: Side::Buy,
                price: 8_450,
                quantity: 100,
            },
            MarketEvent::AddOrder {
                id: 2,
                side: Side::Sell,
                price: -7,
                quantity: 1,
            },
            MarketEvent::CancelOrder { id: 999 },
            MarketEvent::Fill {
                price: 8_400,
                quantity: 50,
                side_aggressive: Side::Sell,
            },
        ];
        for event in all {
            let mut g = SessionRecorder::new();
            let k = RecordedFrame {
                timestamp_nanos: 123_456_789,
                event,
            };
            g.record(&k);
            assert_eq!(g.read_back().unwrap(), vec![k]);
        }
    }

    #[test]
    fn truncated_record_errors_instead_of_panicking() {
        // Tiến trình ghi bị giết giữa chừng là chuyện bình thường trong vận hành.
        let (_, g) = record_session(10, 2);
        for cut in 1..12usize {
            let mut h = SessionRecorder::new();
            h.content = g.content[..g.content.len() - cut].to_vec();
            assert!(
                matches!(
                    h.read_back(),
                    Err(ReadError::TruncatedFrame) | Err(ReadError::UnknownEventCode(_))
                ),
                "cắt {} byte cuối phải báo lỗi",
                cut
            );
        }
    }

    #[test]
    fn absurd_frame_length_is_rejected() {
        let mut g = SessionRecorder::new();
        g.content = vec![0, 0, 0, 3, 1, 2, 3]; // độ dài 3 < 8 byte dấu thời gian
        assert_eq!(g.read_back(), Err(ReadError::InvalidLength(3)));
    }

    #[test]
    fn unknown_event_code_is_rejected() {
        let mut g = SessionRecorder::new();
        g.content.extend_from_slice(&9u32.to_be_bytes());
        g.content.extend_from_slice(&0u64.to_be_bytes());
        g.content.push(b'?');
        assert_eq!(g.read_back(), Err(ReadError::UnknownEventCode(b'?')));
    }

    #[test]
    fn empty_recording_reads_back_empty() {
        assert_eq!(SessionRecorder::new().read_back(), Ok(vec![]));
    }

    #[test]
    fn frame_size_equals_the_sum_of_its_fields() {
        // 4 byte độ dài + 8 byte dấu thời gian + thân.
        // Thân: A = 1+8+1+8+4 = 22 · X = 1+8 = 9 · T = 1+8+4+1 = 14
        let check = |event: MarketEvent, expected: usize| {
            let mut g = SessionRecorder::new();
            g.record(&RecordedFrame {
                timestamp_nanos: 1,
                event,
            });
            assert_eq!(g.byte_len(), expected);
        };
        check(
            MarketEvent::AddOrder {
                id: 1,
                side: Side::Buy,
                price: 1,
                quantity: 1,
            },
            34,
        );
        check(MarketEvent::CancelOrder { id: 1 }, 21);
        check(
            MarketEvent::Fill {
                price: 1,
                quantity: 1,
                side_aggressive: Side::Buy,
            },
            26,
        );
    }

    #[test]
    fn the_binary_format_is_compact_enough_for_a_full_day() {
        let (_, g) = record_session(10_000, 3);
        let bytes_per_event = g.byte_len() as f64 / g.num_frames as f64;
        // Phiên trộn ~70% thêm lệnh (34 B), 15% huỷ (21 B), 15% khớp (26 B)
        // → trung bình khoảng 31 byte.
        assert!(
            (21.0..32.0).contains(&bytes_per_event),
            "trung bình {:.2} byte/sự kiện, kỳ vọng trong khoảng 21–32",
            bytes_per_event
        );
        // Một phiên sôi động 50 triệu sự kiện vẫn chỉ khoảng 1,5 GB
        let full_day_gb = 50_000_000.0 * bytes_per_event / 1e9;
        assert!(
            full_day_gb < 2.0,
            "cả ngày ~{:.2} GB — thừa sức lưu trữ",
            full_day_gb
        );
    }

    // ---------- Đồng hồ ảo ----------
    #[test]
    fn virtual_clock_never_runs_backwards() {
        let mut d = VirtualClock::new(1_000);
        d.advance(500); // sự kiện tới muộn, dấu thời gian cũ
        assert_eq!(d.now_ns, 1_000, "thời gian không được lùi");
        d.advance(2_000);
        assert_eq!(d.now_ns, 2_000);
        d.advance_by(50);
        assert_eq!(d.now_ns, 2_050);
    }

    // ---------- Tốc độ phát ----------
    #[test]
    fn replay_speed_computes_the_right_wall_delay() {
        assert_eq!(ReplaySpeed::RealTime.wall_delay(1_000_000), 1_000_000);
        assert_eq!(ReplaySpeed::Factor(2.0).wall_delay(1_000_000), 500_000);
        assert_eq!(
            ReplaySpeed::Factor(0.5).wall_delay(1_000_000),
            2_000_000,
            "hệ số < 1 để chạy CHẬM lại mà quan sát kỹ"
        );
        assert_eq!(ReplaySpeed::AsFastAsPossible.wall_delay(1_000_000), 0);
        assert_eq!(
            ReplaySpeed::Factor(0.0).wall_delay(1_000_000),
            0,
            "hệ số 0 không được gây chia cho 0"
        );
    }

    #[test]
    fn fast_forward_must_not_change_results() {
        // Tua nhanh chỉ đổi thời gian ta phải ngồi chờ, KHÔNG đổi những gì xảy ra.
        let p = gen_recorded_session(3_000, 5);
        let mut result: Vec<ReplayResult> = Vec::new();
        for speed in [
            ReplaySpeed::RealTime,
            ReplaySpeed::Factor(100.0),
            ReplaySpeed::AsFastAsPossible,
        ] {
            let mut c = NaiveMaker {
                tick_offset: 2,
                order_size: 100,
                max_position: 500,
                step: 0,
                every_n_events: 25,
            };
            result.push(Replayer::new(LatencyModel::colocated(), speed).run(&p, &mut c));
        }
        assert_eq!(result[0].fills, result[1].fills);
        assert_eq!(result[1].fills, result[2].fills);
        assert_eq!(result[0].virtual_time_nanos, result[2].virtual_time_nanos);
        assert!(result[0].real_wait_nanos > result[1].real_wait_nanos);
        assert_eq!(result[2].real_wait_nanos, 0);
    }

    // ---------- Mô hình độ trễ ----------
    #[test]
    fn latency_is_deterministic_per_sequence_number() {
        let d = LatencyModel::colocated();
        for i in 0..100u64 {
            assert_eq!(
                d.round_trip_ns(i),
                d.round_trip_ns(i),
                "cùng sự kiện → cùng độ trễ"
            );
        }
        assert_eq!(LatencyModel::no_latency().round_trip_ns(42), 0);
    }

    #[test]
    fn latency_always_stays_in_a_sane_range() {
        let d = LatencyModel::colocated();
        let min = d.in_nanos + d.out_nanos;
        for i in 0..1_000u64 {
            let x = d.round_trip_ns(i);
            assert!(
                x >= min && x < min + d.jitter_ns,
                "độ trễ {} nằm ngoài [{}, {})",
                x,
                min,
                min + d.jitter_ns
            );
        }
    }

    #[test]
    fn a_leased_line_beats_the_internet_by_orders_of_magnitude() {
        let a = LatencyModel::colocated().round_trip_ns(0);
        let b = LatencyModel::over_internet().round_trip_ns(0);
        assert!(b > a * 100, "ngồi cạnh sàn nhanh hơn {} lần", b / a.max(1));
    }

    // ---------- Sổ rút gọn ----------
    #[test]
    fn reduced_book_reports_the_right_best_prices() {
        let mut s = ReducedBook::default();
        s.add(Side::Buy, 8_390, 100);
        s.add(Side::Buy, 8_400, 200);
        s.add(Side::Sell, 8_420, 100);
        s.add(Side::Sell, 8_410, 50);
        assert_eq!(s.best_bid(), Some(8_400));
        assert_eq!(s.best_ask(), Some(8_410));
        s.reduce(Side::Buy, 8_400, 200);
        assert_eq!(s.best_bid(), Some(8_390), "mức hết hàng phải biến mất");
    }

    // ---------- Vị thế ----------
    #[test]
    fn position_is_a_monoid() {
        let a = Position::from_fill(Side::Buy, 100, 10);
        let b = Position::from_fill(Side::Sell, 110, 5);
        let c = Position::from_fill(Side::Buy, 90, 3);
        assert_eq!(
            a.compose(b).compose(c),
            a.compose(b.compose(c)),
            "luật kết hợp"
        );
        assert_eq!(a.compose(Position::default()), a, "luật đơn vị");
    }

    #[test]
    fn buy_low_sell_high_is_profitable() {
        let v = Position::from_fill(Side::Buy, 8_000, 100).compose(Position::from_fill(
            Side::Sell,
            8_500,
            100,
        ));
        assert_eq!(v.quantity, 0);
        assert_eq!(v.net_value(0), 50_000);
    }

    // ---------- Phát lại ----------
    #[test]
    fn replayed_book_is_never_crossed() {
        // Bài học mô hình: nếu bộ phát lại bỏ qua bản tin huỷ, hoặc bộ sinh
        // dữ liệu đặt lệnh vượt giá bên kia, sổ bị chéo và chiến lược đứng
        // ngoài mà ta không hiểu vì sao. Trước khi sửa bộ sinh, sổ phát lại bị
        // chéo ở khoảng 89% số sự kiện của phiên này.
        struct CrossWatch {
            crossed: u64,
            seen: u64,
        }
        impl ReplayStrategy for CrossWatch {
            fn name(&self) -> &str {
                "canh sổ chéo"
            }
            fn on_event(
                &mut self,
                _: &VirtualClock,
                book: &ReducedBook,
                _: &Position,
            ) -> Vec<(Side, Price, Quantity)> {
                self.seen += 1;
                if let (Some(b), Some(a)) = (book.best_bid(), book.best_ask())
                    && b >= a
                {
                    self.crossed += 1;
                }
                vec![]
            }
        }
        let p = gen_recorded_session(20_000, 2024);
        let mut w = CrossWatch {
            crossed: 0,
            seen: 0,
        };
        Replayer::new(LatencyModel::no_latency(), ReplaySpeed::AsFastAsPossible).run(&p, &mut w);
        assert_eq!(w.seen, 20_000);
        assert_eq!(w.crossed, 0, "sổ phát lại không bao giờ được chéo hay khoá");
    }

    #[test]
    fn a_market_maker_gets_to_quote_most_of_the_time() {
        // 20 000 sự kiện, cứ 50 sự kiện lại chào giá hai phía → tối đa 800 lệnh.
        // Trước khi sửa, sổ chéo nên chỉ gửi được 76.
        let p = gen_recorded_session(20_000, 2024);
        let mut c = NaiveMaker {
            tick_offset: 2,
            order_size: 100,
            max_position: 500,
            step: 0,
            every_n_events: 50,
        };
        let result =
            Replayer::new(LatencyModel::colocated(), ReplaySpeed::AsFastAsPossible).run(&p, &mut c);
        assert!(
            result.orders_sent > 400,
            "chỉ gửi {} lệnh — dấu hiệu sổ bị chéo và chiến lược đứng ngoài",
            result.orders_sent
        );
        assert!(
            result.filled_orders > 100,
            "và phải khớp được kha khá, thực tế {}",
            result.filled_orders
        );
    }

    #[test]
    fn cancels_actually_remove_liquidity() {
        let frame = vec![
            RecordedFrame {
                timestamp_nanos: 1_000,
                event: MarketEvent::AddOrder {
                    id: 1,
                    side: Side::Buy,
                    price: 8_400,
                    quantity: 500,
                },
            },
            RecordedFrame {
                timestamp_nanos: 2_000,
                event: MarketEvent::AddOrder {
                    id: 2,
                    side: Side::Sell,
                    price: 8_410,
                    quantity: 300,
                },
            },
            RecordedFrame {
                timestamp_nanos: 3_000,
                event: MarketEvent::CancelOrder { id: 1 },
            },
        ];
        // Dùng một chiến lược chỉ quan sát để đọc trạng thái sổ ở bước cuối
        struct Observer {
            last_bid: Option<Price>,
            last_ask: Option<Price>,
        }
        impl ReplayStrategy for Observer {
            fn name(&self) -> &str {
                "soi sổ"
            }
            fn on_event(
                &mut self,
                _: &VirtualClock,
                book: &ReducedBook,
                _: &Position,
            ) -> Vec<(Side, Price, Quantity)> {
                self.last_bid = book.best_bid();
                self.last_ask = book.best_ask();
                vec![]
            }
        }
        let mut s = Observer {
            last_bid: None,
            last_ask: None,
        };
        Replayer::new(LatencyModel::no_latency(), ReplaySpeed::AsFastAsPossible)
            .run(&frame, &mut s);
        assert_eq!(s.last_bid, None, "lệnh mua đã bị huỷ, bên mua phải rỗng");
        assert_eq!(s.last_ask, Some(8_410), "lệnh bán không bị đụng tới");
    }

    #[test]
    fn replay_is_bit_exact_reproducible() {
        // BẤT BIẾN QUAN TRỌNG NHẤT của chương. Nếu bài này hỏng thì mọi kết
        // quả kiểm định đều vô nghĩa vì không so sánh được với nhau.
        let p = gen_recorded_session(5_000, 2024);
        let run = || {
            let mut c = NaiveMaker {
                tick_offset: 2,
                order_size: 100,
                max_position: 500,
                step: 0,
                every_n_events: 30,
            };
            Replayer::new(LatencyModel::colocated(), ReplaySpeed::AsFastAsPossible).run(&p, &mut c)
        };
        assert_eq!(run(), run());
        assert_eq!(run(), run(), "ba lần vẫn phải giống hệt");
    }

    #[test]
    fn standing_aside_means_no_orders_and_no_pnl() {
        let p = gen_recorded_session(2_000, 7);
        let result = Replayer::new(LatencyModel::colocated(), ReplaySpeed::AsFastAsPossible)
            .run(&p, &mut SitOut);
        assert_eq!(result.orders_sent, 0);
        assert_eq!(result.filled_orders, 0);
        assert_eq!(result.last_position, Position::default());
        assert_eq!(result.last_value, 0);
    }

    #[test]
    fn an_order_cannot_fill_before_it_reaches_the_venue() {
        // Nếu mô phỏng cho lệnh khớp ngay lúc quyết định, ta đã "nhìn trộm
        // tương lai" ở mức tinh vi nhất — và kết quả sẽ đẹp một cách giả tạo.
        let p = gen_recorded_session(3_000, 11);
        let mut c = NaiveMaker {
            tick_offset: 1,
            order_size: 100,
            max_position: 10_000,
            step: 0,
            every_n_events: 10,
        };
        let latency = LatencyModel::over_internet();
        let result = Replayer::new(latency, ReplaySpeed::AsFastAsPossible).run(&p, &mut c);
        let first = p.first().unwrap().timestamp_nanos;
        let min = latency.in_nanos + latency.out_nanos;
        for k in &result.fills {
            assert!(
                k.timestamp_nanos >= first + min,
                "khớp lúc {} là quá sớm — lệnh chưa kịp bay tới sàn",
                k.timestamp_nanos
            );
        }
    }

    #[test]
    fn more_latency_means_fewer_fills() {
        // Đây là lý do các hãng trả rất nhiều tiền để đặt máy cạnh sàn.
        let p = gen_recorded_session(8_000, 2024);
        let count_fill = |latency: LatencyModel| {
            let mut c = NaiveMaker {
                tick_offset: 1,
                order_size: 100,
                max_position: 10_000,
                step: 0,
                every_n_events: 10,
            };
            Replayer::new(latency, ReplaySpeed::AsFastAsPossible)
                .run(&p, &mut c)
                .filled_orders
        };
        let fast = count_fill(LatencyModel::colocated());
        let slow = count_fill(LatencyModel::over_internet());
        assert!(
            fast >= slow,
            "gần sàn phải khớp được ít nhất bằng: {} so với {}",
            fast,
            slow
        );
    }

    #[test]
    fn final_position_equals_the_sum_of_fills() {
        // Kế toán phải khớp: vị thế = tổng mọi lần khớp, không thừa không thiếu.
        let p = gen_recorded_session(5_000, 17);
        let mut c = NaiveMaker {
            tick_offset: 2,
            order_size: 100,
            max_position: 1_000,
            step: 0,
            every_n_events: 20,
        };
        let result =
            Replayer::new(LatencyModel::colocated(), ReplaySpeed::AsFastAsPossible).run(&p, &mut c);
        let rebuilt = result.fills.iter().fold(Position::default(), |a, k| {
            a.compose(Position::from_fill(k.side, k.price, k.quantity))
        });
        assert_eq!(
            rebuilt, result.last_position,
            "dựng lại vị thế từ nhật ký khớp phải ra đúng vị thế cuối"
        );
        assert_eq!(result.filled_orders as usize, result.fills.len());
    }

    #[test]
    fn every_fill_is_valid() {
        let p = gen_recorded_session(5_000, 13);
        let mut c = NaiveMaker {
            tick_offset: 2,
            order_size: 100,
            max_position: 1_000,
            step: 0,
            every_n_events: 20,
        };
        let result =
            Replayer::new(LatencyModel::colocated(), ReplaySpeed::AsFastAsPossible).run(&p, &mut c);
        for k in &result.fills {
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
        let p = gen_recorded_session(10_000, 23);
        let cap = 300i64;
        let mut c = NaiveMaker {
            tick_offset: 1,
            order_size: 100,
            max_position: cap,
            step: 0,
            every_n_events: 5,
        };
        let result =
            Replayer::new(LatencyModel::colocated(), ReplaySpeed::AsFastAsPossible).run(&p, &mut c);
        assert!(
            result.peak_abs_position > cap,
            "chính vì bỏ qua lệnh đang treo mà vị thế chạm {} — vượt trần {}",
            result.peak_abs_position,
            cap
        );
    }

    #[test]
    fn counting_resting_orders_keeps_the_cap() {
        // Bản đúng: phơi bày = vị thế đã khớp + khối lượng đang treo.
        let p = gen_recorded_session(10_000, 23);
        let cap = 300i64;
        let mut c = ManagedMaker::new(1, 100, cap, 5);
        let result =
            Replayer::new(LatencyModel::colocated(), ReplaySpeed::AsFastAsPossible).run(&p, &mut c);
        // Kiểm |vị thế| lớn nhất trong CẢ phiên, không chỉ vị thế cuối:
        // vị thế cuối nằm trong trần không chứng minh gì nếu giữa phiên đã vượt.
        assert!(
            result.peak_abs_position <= cap,
            "|vị thế| lớn nhất {} phải nằm trong trần {}",
            result.peak_abs_position,
            cap
        );
        assert!(
            result.orders_sent > 0,
            "vẫn phải giao dịch được, không phải đứng im"
        );
    }

    #[test]
    fn inventory_control_holds_for_every_seed() {
        for seed in [1u64, 7, 23, 42, 2024] {
            let p = gen_recorded_session(8_000, seed);
            let cap = 200i64;
            let mut c = ManagedMaker::new(1, 100, cap, 5);
            let result = Replayer::new(LatencyModel::colocated(), ReplaySpeed::AsFastAsPossible)
                .run(&p, &mut c);
            assert!(
                result.peak_abs_position <= cap,
                "hạt giống {}: vị thế chạm {} vượt trần {}",
                seed,
                result.peak_abs_position,
                cap
            );
        }
    }

    #[test]
    fn strategy_stands_aside_on_a_crossed_book() {
        let mut s = ReducedBook::default();
        s.add(Side::Buy, 8_500, 100);
        s.add(Side::Sell, 8_400, 100);
        let mut c = NaiveMaker {
            tick_offset: 2,
            order_size: 100,
            max_position: 500,
            step: 0,
            every_n_events: 1,
        };
        let order = c.on_event(&VirtualClock::new(0), &s, &Position::default());
        assert!(
            order.is_empty(),
            "sổ chéo → phải đứng ngoài, không được coi là cơ hội"
        );
    }

    #[test]
    fn strategy_sends_nothing_on_an_empty_book() {
        let mut c = NaiveMaker {
            tick_offset: 2,
            order_size: 100,
            max_position: 500,
            step: 0,
            every_n_events: 1,
        };
        assert!(
            c.on_event(
                &VirtualClock::new(0),
                &ReducedBook::default(),
                &Position::default()
            )
            .is_empty()
        );
    }

    // ---------- Sinh phiên ----------
    #[test]
    fn generated_session_is_deterministic_and_monotonic() {
        assert_eq!(gen_recorded_session(100, 5), gen_recorded_session(100, 5));
        assert_ne!(gen_recorded_session(100, 5), gen_recorded_session(100, 6));
        let p = gen_recorded_session(1_000, 1);
        for w in p.windows(2) {
            assert!(w[1].timestamp_nanos > w[0].timestamp_nanos);
        }
    }

    #[test]
    fn generated_session_covers_all_three_event_kinds() {
        let p = gen_recorded_session(5_000, 3);
        let adds = p
            .iter()
            .filter(|k| matches!(k.event, MarketEvent::AddOrder { .. }))
            .count();
        let cancel = p
            .iter()
            .filter(|k| matches!(k.event, MarketEvent::CancelOrder { .. }))
            .count();
        let fill = p
            .iter()
            .filter(|k| matches!(k.event, MarketEvent::Fill { .. }))
            .count();
        assert!(
            adds > 0 && cancel > 0 && fill > 0,
            "phiên phải có cả ba loại sự kiện"
        );
        assert_eq!(adds + cancel + fill, p.len());
        assert!(
            adds > fill,
            "thực tế: đặt lệnh nhiều hơn khớp lệnh rất nhiều"
        );
    }
}
```

---

## Bảng tra cứu lỗi biên dịch thường gặp

| Lỗi | Nguyên nhân trong chương này | Cách sửa |
|---|---|---|
| Phát lại ra kết quả khác mỗi lần | Duyệt `HashMap` khi cấp phát khớp | Đổi sang `BTreeMap` — bắt buộc cho tính tất định |
| `E0502: cannot borrow as mutable because it is also borrowed as immutable` | Duyệt `market_orders.iter()` rồi gọi `market_orders.remove(..)` ngay trong vòng lặp | Thu mã cần xoá vào `Vec` cục bộ (`to_remove`), xoá sau vòng lặp |
| `E0499: cannot borrow *self as mutable more than once` | Giữ cùng lúc kết quả của hai phương thức `&mut self` như `self.book_mut()` và `self.strategy_mut()` | Mượn thẳng hai trường (`&mut self.book`, `&mut self.strategy`) — trình kiểm tra mượn tách được các trường riêng biệt; hoặc truyền chúng làm tham số như `Replayer::run` |
| Sổ lệnh chéo vĩnh viễn | Bộ phát lại bỏ qua thông điệp huỷ | Xử lý **mọi** loại thông điệp, không chọn lọc |
| Vị thế vượt hạn mức | Chỉ đếm vị thế đã khớp | Cộng cả phơi nhiễm từ lệnh đang treo |

---

## Tóm tắt chương & Bài tập rèn luyện

### 5 điểm cốt lõi

1. **Backtest trên nến là nói dối.** Phục dựng theo thông điệp là cách duy nhất biết mình có được khớp hay không.
2. **Đồng hồ ảo (Virtual clock) phải là nguồn thời gian duy nhất.** Một lời gọi `Instant::now()` lạc lõng là đủ phá cả hệ thống.
3. **Bỏ qua độ trễ là nhìn trộm tương lai.** Và nó là dạng tinh vi nhất, vì không ai gọi tên nó như vậy.
4. **`HashMap` phá tính tất định.** Đây là lỗi có thật đã xảy ra ngay trong chương này.
5. **Hạn mức tồn kho phải tính cả lệnh đang treo.** Đếm thiếu thì vị thế vượt hạn mức gấp nhiều lần.

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
    /// Hệ số cho phần tạm thời (đã gộp độ biến động), thường 0,1–1,0 tuỳ thị trường.
    pub temporary_coef: f64,
    /// Phần tác động ở lại vĩnh viễn, thường 0,3–0,5.
    pub permanent_fraction: f64,
    pub daily_volume: f64,
}

impl ImpactModel {
    /// Quy luật căn bậc hai — chuẩn công nghiệp cho tác động tạm thời.
    pub fn impact_bps(&self, quantity: u64) -> f64 {
        if self.daily_volume <= 0.0 {
            return 0.0;
        }
        let ratio = quantity as f64 / self.daily_volume;
        self.temporary_coef * ratio.sqrt() * 10_000.0
    }

    /// Giá khớp thực tế sau khi tính tác động.
    pub fn fill_price(&self, quoted_price: Price, side: Side, quantity: u64) -> Price {
        let bps = self.impact_bps(quantity);
        let shift = quoted_price as f64 * bps / 10_000.0;
        match side {
            Side::Buy => (quoted_price as f64 + shift) as Price, // mua thì đẩy giá lên
            Side::Sell => (quoted_price as f64 - shift) as Price,
        }
    }

    /// Phần tác động KHÔNG hồi lại — cái này mới thực sự đắt.
    pub fn permanent_shift(&self, quoted_price: Price, quantity: u64) -> f64 {
        quoted_price as f64 * self.impact_bps(quantity) / 10_000.0 * self.permanent_fraction
    }
}
```

Quy luật căn bậc hai có hệ quả quan trọng: tác động trên mỗi đơn vị tỉ lệ √Q, nên tổng chi phí của cả lệnh tỉ lệ Q·√Q. Chia lệnh thành 4 phần (giả sử phần tác động tạm thời kịp hồi giữa các lần) thì mỗi phần chỉ chịu √(Q/4) = ½√Q — tổng chi phí giảm còn một nửa. Đó là cơ sở toán học của các thuật toán thực thi kiểu VWAP và TWAP.
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
fn hash_fills(fills: &[OurFill]) -> u64 {
    let mut h: u64 = 0xcbf29ce484222325;
    for f in fills {
        for b in f
            .timestamp_nanos
            .to_le_bytes()
            .iter()
            .chain(&[f.side as u8])
            .chain(f.price.to_le_bytes().iter())
            .chain(f.quantity.to_le_bytes().iter())
        {
            h ^= *b as u64;
            h = h.wrapping_mul(0x100000001b3);
        }
    }
    h
}

#[test]
fn replay_must_be_deterministic() {
    let session = gen_recorded_session(20_000, 42);

    let run = || {
        let replayer = Replayer::new(LatencyModel::colocated(), ReplaySpeed::AsFastAsPossible);
        let mut maker = ManagedMaker::new(1, 100, 300, 5);
        hash_fills(&replayer.run(&session, &mut maker).fills)
    };

    let a = run();
    let b = run();
    assert_eq!(a, b, "phát lại KHÔNG tất định — kiểm HashMap, RNG, Instant::now()");
}
```

Nếu bài kiểm thử này trượt, đừng sửa bài kiểm thử — hãy đi tìm nguồn bất định. Ba nghi phạm theo thứ tự khả năng: duyệt `HashMap`/`HashSet`, bộ sinh ngẫu nhiên không gieo hạt cố định, và lời gọi đồng hồ thật.
</details>
