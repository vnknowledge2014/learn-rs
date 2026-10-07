# Chương 75: Dữ liệu thị trường — Giao thức nhị phân, Phát hiện khe & Sổ lệnh (Market Data Pipeline)

## Giới thiệu & Mục tiêu học tập

Sổ lệnh (Order book) là **cấu trúc dữ liệu quan trọng nhất trong tài chính**. Mọi giá bạn từng thấy — cổ phiếu, tiền mã hoá, hợp đồng tương lai — đều là kết quả của một sổ lệnh khớp lệnh mua với lệnh bán.

Chương này dựng đường dẫn dữ liệu thị trường đầy đủ:

```
gói UDP → phân tích nhị phân → phát hiện khe → cập nhật sổ lệnh → tín hiệu
```

Ba bài học cốt lõi:

1. **Giao thức nhị phân (Binary protocol), không JSON.** ITCH của Nasdaq nhồi một cập nhật vào 36 byte. Cùng nội dung ở JSON tốn 100–200 byte và mất hàng microsecond để phân tích.
2. **Multicast UDP mất gói.** Không có TCP để sửa hộ. Bạn phải tự phát hiện khe và tự yêu cầu phát lại — **đúng một lần**, không lặp.
3. **L2 hay L3 là quyết định kiến trúc.** L2 (gộp theo mức giá) đủ cho hầu hết chiến lược. L3 (từng lệnh) cho biết **vị trí xếp hàng** — thứ quyết định lãi lỗ của nhà tạo lập.

---

## Hình tượng hóa đời sống

```
┌──────────────────────────────────────────────────────────────────────────────┐
│  GIAO THỨC NHỊ PHÂN = ĐIỀN VÀO Ô CÓ SẴN, KHÔNG VIẾT VĂN                     │
│                                                                              │
│   JSON (≈70 byte cho ví dụ này, phân tích ~1 µs):                           │
│     {"type":"add","order_id":12345,"side":"B","price":10050,"qty":100}      │
│                                                                              │
│   Nhị phân (30 byte, phân tích ~40 ns):                                     │
│     ┌──┬────────┬────────┬──┬────────┬────────┐                            │
│     │01│ 12345  │  ts    │B │ 10050  │  100   │                            │
│     └──┴────────┴────────┴──┴────────┴────────┘                            │
│      loại  u64     u64    u8   u64      u32                                 │
│                                                                              │
│   Không tìm dấu ngoặc, không cấp phát chuỗi. Chỉ đọc theo độ lệch cố định.  │
│                                                                              │
│  PHÁT HIỆN KHE = SỐ THỨ TỰ NHẢY CÓC                                         │
│                                                                              │
│    ...101, 102, 103, ▓▓▓, ▓▓▓, 106, 107...                                 │
│                       └──┬──┘                                               │
│              mất 104,105 → xin phát lại MỘT LẦN                             │
│                                                                              │
│   ⚠ LỖI KINH ĐIỂN: cứ mỗi thông điệp mới lại báo lại cùng một khe.         │
│     107 → "vẫn thiếu 104-105!", 108 → "vẫn thiếu 104-105!"...              │
│     Kết quả: bão yêu cầu phát lại, làm sập chính đường phục hồi.            │
│   → Phải có TRẠNG THÁI "đang chờ khôi phục".                                │
│                                                                              │
│  L2 vs L3                                                                   │
│                                                                              │
│   L2 — gộp theo mức giá        L3 — từng lệnh riêng, có thứ tự              │
│   ┌────────┬─────┐             ┌────────┬───────────────────────┐          │
│   │ 100.50 │ 500 │             │ 100.50 │ #7(200) #9(150) #12(150)│        │
│   │ 100.49 │ 300 │             │ 100.49 │ #3(300)                 │        │
│   └────────┴─────┘             └────────┴───────────────────────┘          │
│                                                                             │
│   Với L3 bạn biết lệnh #9 có 200 đơn vị XẾP TRƯỚC.                         │
│   Phải khớp hết 200 đó thì mới tới lượt bạn.                                │
│   Đó là thông tin quyết định: đứng cuối hàng thì gần như chỉ được khớp     │
│   khi giá sắp đi ngược lại — tức là bị "chọn lọc bất lợi".                 │
└──────────────────────────────────────────────────────────────────────────────┘
```

---

## Khái niệm & Cơ chế kỹ thuật chuyên sâu

### 1. Vì sao trường có độ dài cố định

Trong ITCH, mọi trường đều ở vị trí cố định. Bộ phân tích chỉ việc đọc byte tại độ lệch đã biết — không rẽ nhánh, không tìm kiếm, không cấp phát.

Điểm đánh đổi: mở rộng giao thức khó. Thêm trường mới nghĩa là thêm loại thông điệp mới, không phải thêm khoá vào JSON. Các sàn giải quyết bằng cách đánh phiên bản ở mức phiên kết nối.

Về endianness: các giao thức mạng thường dùng big-endian ("thứ tự byte mạng"), còn x86 là little-endian. Nhớ `from_be_bytes` chứ không phải `from_le_bytes` — nhầm ở đây cho ra giá sai lệch hàng triệu lần mà chương trình vẫn chạy vui vẻ.

### 2. Trạng thái phục hồi: lỗi mà chương này sửa

Bản đầu tiên của bộ phát hiện khe trong chương này có một lỗi thật: nó báo lại **cùng một khe** cho mọi thông điệp tiếp theo. Trong sản xuất, lỗi đó tạo ra bão yêu cầu phát lại — và bão đó làm sập chính đường phục hồi mà bạn đang cần.

Cách chữa là thêm trạng thái `pending_gap: Option<(u64, u64)>` và một biến thể kết quả `SeqOutcome::AwaitingRecovery`. Khi đã yêu cầu phát lại một khe, mọi thông điệp sau đó chỉ báo "đang chờ" chứ không sinh yêu cầu mới. Bộ phát hiện còn nhớ số thứ tự lớn nhất đã thấy (`highest_seen`), nên nếu một khe **mới** xuất hiện phía sau khe cũ, nó vẫn xin phát lại đúng phần mới đó — một lần.

Hai chi tiết nhỏ nhưng dễ sai: khi gói còn thiếu tới trễ trên chính luồng chính (UDP có thể đảo thứ tự), bộ phát hiện phải đẩy số kỳ vọng qua **mọi** bản tin đã đệm liền mạch chứ không chỉ +1; và `drain()` chỉ được rút những bản tin đứng trước khe đầu tiên.

Đây là ví dụ điển hình của một loại lỗi mà **kiểm thử một thông điệp không bao giờ bắt được** — phải kiểm thử một dòng thông điệp mới lộ.

### 3. Vị trí xếp hàng: nơi lãi lỗ của nhà tạo lập được quyết định

Hầu hết sàn khớp theo **giá – thời gian**: cùng mức giá thì ai đặt trước được khớp trước. Nghĩa là khi bạn đặt lệnh mua ở 100.50 mà đã có 500 đơn vị đứng trước, phải khớp hết 500 đơn vị đó rồi mới tới bạn.

Hệ quả kinh tế rất sắc: nếu bạn đứng cuối hàng, lệnh của bạn thường chỉ được khớp khi có **nhiều** người bán — tức là khi giá đang chuẩn bị đi xuống. Bạn được khớp đúng lúc không nên được khớp. Đó là **chọn lọc bất lợi**, và nó là lý do tốc độ có giá trị: đến sớm nghĩa là đứng đầu hàng.

Có một chi tiết thú vị: hủy rồi đặt lại ở cùng mức giá sẽ **mất toàn bộ vị trí xếp hàng**. Nhưng **giảm** khối lượng của lệnh hiện có thì thường **giữ** được vị trí. Đó là lý do các thuật toán tinh vi giảm khối lượng thay vì hủy-và-đặt-lại.

### 4. Vì sao dùng `BTreeMap` cho sổ lệnh

Sổ lệnh cần: giá tốt nhất (min hoặc max), duyệt theo thứ tự giá, và chèn/xoá nhanh. `BTreeMap` cho cả ba với O(log n), và quan trọng nhất — **thứ tự duyệt là tất định**.

`HashMap` nhanh hơn cho tra cứu điểm, nhưng thứ tự duyệt không xác định. Trong hệ thống giao dịch, thứ tự không xác định nghĩa là **phát lại không tái lập được** — bạn không thể gỡ lỗi một sự cố sản xuất. Chương 76 sẽ cho thấy đúng lỗi này xảy ra như thế nào.

Sổ lệnh sản xuất thực sự thường đi xa hơn: dùng mảng có chỉ số theo giá (vì giá rời rạc theo bước giá), cho O(1) ở mọi thao tác. Nhưng nó tốn bộ nhớ theo dải giá, nên chỉ hợp với thị trường có dải hẹp.

---

## Mã nguồn minh họa thực chiến

Chạy bằng `cargo run -p ch75`, kiểm thử bằng `cargo test -p ch75`.

```rust
#![allow(dead_code)]
//! Chương 75 — Xử lý luồng dữ liệu thị trường: giao thức nhị phân kiểu ITCH,
//! phát hiện khe số thứ tự, dựng sổ lệnh L2/L3 từ bản tin gia tăng, và kiểm
//! tra chất lượng dữ liệu.
//!
//! Đây là chặng đầu tiên trong ngân sách tick-to-trade của Chương 74. Sai ở
//! đây thì mọi thứ phía sau đều tính trên dữ liệu rác.

use std::collections::{BTreeMap, HashMap};

// ============================================================================
// 1. GIAO THỨC NHỊ PHÂN — vì sao sàn không dùng JSON
// ============================================================================
// Một bản tin JSON tốn ~100 byte và mất hàng micro-giây để phân tích. Cùng
// thông tin đó ở dạng nhị phân cố định tốn 42 byte và đọc xong trong vài chục
// nano-giây — chỉ là vài phép đọc số nguyên từ vị trí đã biết trước.

pub type Price = i64; // tick, 1 tick = 0,01 đơn vị tiền
pub type Quantity = u32;
pub type OrderId = u64;

// `PartialOrd, Ord` để `(Side, Price)` dùng được làm khoá `BTreeMap` ở sổ L3.
#[derive(Debug, Clone, Copy, PartialEq, Eq, PartialOrd, Ord)]
pub enum Side {
    Buy,
    Sell,
}

#[derive(Debug, Clone, PartialEq)]
pub enum Message {
    /// Thêm lệnh mới vào sổ
    AddOrder {
        id: OrderId,
        symbol_id: u32,
        side: Side,
        price: Price,
        quantity: Quantity,
    },
    /// Lệnh bị huỷ một phần hoặc toàn bộ
    CancelOrder {
        id: OrderId,
        cancel_quantity: Quantity,
    },
    /// Lệnh khớp
    Fill {
        id: OrderId,
        quantity: Quantity,
        price: Price,
    },
    /// Thay thế lệnh: huỷ cũ, tạo mới, MẤT ưu tiên thời gian
    Replaced {
        old_id: OrderId,
        new_id: OrderId,
        price: Price,
        quantity: Quantity,
    },
}

#[derive(Debug, Clone, PartialEq)]
pub struct FeedPacket {
    pub seq: u64,
    pub timestamp_nanos: u64,
    pub message: Message,
}

#[derive(Debug, PartialEq)]
pub enum ParseError {
    TooShort { needed: usize, got: usize },
    UnknownMessageKind(u8),
    UnknownSide(u8),
}

/// Phân tích một bản tin nhị phân. Không cấp phát, không sao chép — chỉ đọc
/// số nguyên từ các vị trí cố định. Đây là ý nghĩa của "phân tích zero-copy".
///
/// Bố cục dây (big-endian, như mọi giao thức mạng):
/// ```text
///  0        1        9           17     25      29      30       38
///  +--------+--------+-----------+------+-------+-------+--------+
///  | loại   | stt    | thời điểm | mã   | mã ck | chiều | giá    | số lượng
///  | 1 byte | 8 byte | 8 byte    |8 byte| 4 byte| 1 byte| 8 byte | 4 byte
/// ```
pub fn analyze(b: &[u8]) -> Result<FeedPacket, ParseError> {
    if b.len() < 17 {
        return Err(ParseError::TooShort {
            needed: 17,
            got: b.len(),
        });
    }
    let kind = b[0];
    let seq = u64::from_be_bytes(b[1..9].try_into().unwrap());
    let timestamp_nanos = u64::from_be_bytes(b[9..17].try_into().unwrap());

    let needed = match kind {
        b'A' => 42,
        b'X' => 29,
        b'E' => 37,
        b'R' => 45,
        _ => 17,
    };
    if b.len() < needed {
        return Err(ParseError::TooShort {
            needed,
            got: b.len(),
        });
    }

    let read_u32 = |i: usize| -> u32 { u32::from_be_bytes(b[i..i + 4].try_into().unwrap()) };
    let read_i64 = |i: usize| -> i64 { i64::from_be_bytes(b[i..i + 8].try_into().unwrap()) };
    let read_u64 = |i: usize| -> u64 { u64::from_be_bytes(b[i..i + 8].try_into().unwrap()) };

    let message = match kind {
        b'A' => Message::AddOrder {
            id: read_u64(17),
            symbol_id: read_u32(25),
            side: match b[29] {
                b'B' => Side::Buy,
                b'S' => Side::Sell,
                x => return Err(ParseError::UnknownSide(x)),
            },
            price: read_i64(30),
            quantity: read_u32(38),
        },
        b'X' => Message::CancelOrder {
            id: read_u64(17),
            cancel_quantity: read_u32(25),
        },
        b'E' => Message::Fill {
            id: read_u64(17),
            quantity: read_u32(25),
            price: read_i64(29),
        },
        b'R' => Message::Replaced {
            old_id: read_u64(17),
            new_id: read_u64(25),
            price: read_i64(33),
            quantity: read_u32(41),
        },
        x => return Err(ParseError::UnknownMessageKind(x)),
    };
    Ok(FeedPacket {
        seq,
        timestamp_nanos,
        message,
    })
}

/// Mã hoá ngược — dùng để sinh dữ liệu kiểm thử và để ghi lại phiên (Chương 76).
pub fn encode(g: &FeedPacket) -> Vec<u8> {
    let mut v = Vec::with_capacity(48);
    let kind = match g.message {
        Message::AddOrder { .. } => b'A',
        Message::CancelOrder { .. } => b'X',
        Message::Fill { .. } => b'E',
        Message::Replaced { .. } => b'R',
    };
    v.push(kind);
    v.extend_from_slice(&g.seq.to_be_bytes());
    v.extend_from_slice(&g.timestamp_nanos.to_be_bytes());
    match &g.message {
        Message::AddOrder {
            id,
            symbol_id,
            side,
            price,
            quantity,
        } => {
            v.extend_from_slice(&id.to_be_bytes());
            v.extend_from_slice(&symbol_id.to_be_bytes());
            v.push(if *side == Side::Buy { b'B' } else { b'S' });
            v.extend_from_slice(&price.to_be_bytes());
            v.extend_from_slice(&quantity.to_be_bytes());
        }
        Message::CancelOrder {
            id,
            cancel_quantity,
        } => {
            v.extend_from_slice(&id.to_be_bytes());
            v.extend_from_slice(&cancel_quantity.to_be_bytes());
        }
        Message::Fill {
            id,
            quantity,
            price,
        } => {
            v.extend_from_slice(&id.to_be_bytes());
            v.extend_from_slice(&quantity.to_be_bytes());
            v.extend_from_slice(&price.to_be_bytes());
        }
        Message::Replaced {
            old_id,
            new_id,
            price,
            quantity,
        } => {
            v.extend_from_slice(&old_id.to_be_bytes());
            v.extend_from_slice(&new_id.to_be_bytes());
            v.extend_from_slice(&price.to_be_bytes());
            v.extend_from_slice(&quantity.to_be_bytes());
        }
    }
    v
}

// ============================================================================
// 2. PHÁT HIỆN KHE SỐ THỨ TỰ
// ============================================================================
// Dữ liệu thị trường thường đi qua UDP multicast: nhanh, nhưng KHÔNG bảo đảm
// tới nơi và KHÔNG bảo đảm đúng thứ tự. Số thứ tự là thứ duy nhất cho ta biết
// mình có đang nhìn bức tranh đầy đủ hay không.

#[derive(Debug, Clone, Copy, PartialEq)]
pub enum SeqOutcome {
    /// Đúng bản tin kế tiếp — xử lý ngay
    InOrder,
    /// Bản tin cũ (bản sao từ luồng dự phòng) — bỏ qua
    Duplicate,
    /// PHÁT HIỆN một khe mới: thiếu `count` bản tin, từ `from` tới `to`.
    /// Đây là lúc DUY NHẤT ta gửi yêu cầu phát lại.
    MissingMessages { from: u64, to: u64, count: u64 },
    /// Bản tin tới sớm nhưng khe phía trước đã được xin phát lại. Bản tin
    /// vẫn được đệm lại nhưng KHÔNG xin phát lại nữa.
    AwaitingRecovery,
}

pub struct GapDetector {
    pub expected_seq: u64,
    /// Bản tin đã nhận nhưng chưa rút ra — cả bản tin đúng thứ tự lẫn bản tin
    /// tới sớm (giữ lại, xử lý sau khi khe được lấp).
    buffered: BTreeMap<u64, FeedPacket>,
    /// Số thứ tự lớn nhất từng thấy — để nhận ra một khe MỚI nằm sau khe cũ.
    highest_seen: Option<u64>,
    /// Vùng đang chờ lấp: (bản tin đầu thiếu, bản tin cuối thiếu).
    /// Có giá trị nghĩa là ta đang ở CHẾ ĐỘ KHÔI PHỤC.
    pending_gap: Option<(u64, u64)>,
    pub gap_count: u64,
    pub duplicate_count: u64,
    pub total_lost: u64,
}

impl GapDetector {
    pub fn new(start: u64) -> Self {
        GapDetector {
            expected_seq: start,
            buffered: BTreeMap::new(),
            highest_seen: None,
            pending_gap: None,
            gap_count: 0,
            duplicate_count: 0,
            total_lost: 0,
        }
    }

    pub fn is_recovering(&self) -> bool {
        self.pending_gap.is_some()
    }

    pub fn receive(&mut self, g: FeedPacket) -> SeqOutcome {
        let seq_no = g.seq;
        // Đã xử lý rồi, hoặc đã nằm sẵn trong bộ đệm → bản sao từ luồng dự phòng.
        if seq_no < self.expected_seq || self.buffered.contains_key(&seq_no) {
            self.duplicate_count += 1;
            return SeqOutcome::Duplicate;
        }
        // Số thứ tự nhỏ nhất CHƯA từng thấy và chưa từng được xin phát lại.
        let known_until = self
            .highest_seen
            .map_or(self.expected_seq, |h| h + 1)
            .max(self.expected_seq);
        self.highest_seen = Some(self.highest_seen.map_or(seq_no, |h| h.max(seq_no)));
        self.buffered.insert(seq_no, g); // luôn giữ lại, đừng bao giờ vứt

        if seq_no == self.expected_seq {
            // Bản tin này có thể lấp luôn khe (ví dụ gói UDP tới trễ), nên đẩy
            // kỳ vọng qua MỌI bản tin liền mạch đã đệm, không chỉ +1.
            self.advance();
            return SeqOutcome::InOrder;
        }

        // seq_no > expected_seq: có khe phía trước. Chỉ xin phát lại phần
        // CHƯA từng được xin. Nếu báo lại mỗi bản tin, ta sẽ gửi hàng nghìn yêu
        // cầu phát lại cho CÙNG một khe và tự làm sập luồng khôi phục của
        // sàn — lỗi vận hành có thật.
        if seq_no <= known_until {
            // Khe đã biết: bản tin này nằm trong hoặc ngay sau vùng đang chờ.
            return SeqOutcome::AwaitingRecovery;
        }
        let (from, to) = (known_until, seq_no - 1);
        self.pending_gap = Some(match self.pending_gap {
            Some((first, _)) => (first, to),
            None => (from, to),
        });
        self.gap_count += 1;
        self.total_lost += to - from + 1;
        SeqOutcome::MissingMessages {
            from,
            to,
            count: to - from + 1,
        }
    }

    /// Đẩy kỳ vọng qua toàn bộ phần đã liền mạch; rời chế độ khôi phục khi
    /// mọi bản tin thiếu đã về đủ.
    fn advance(&mut self) {
        while self.buffered.contains_key(&self.expected_seq) {
            self.expected_seq += 1;
        }
        if let Some((_, to)) = self.pending_gap
            && self.expected_seq > to
        {
            self.pending_gap = None;
        }
    }

    /// Rút các bản tin liền mạch đã sẵn sàng xử lý, theo đúng thứ tự. Chỉ rút
    /// những bản tin ĐỨNG TRƯỚC khe đầu tiên — phần sau khe phải chờ.
    pub fn drain(&mut self) -> Vec<FeedPacket> {
        let mut out = Vec::new();
        while let Some(entry) = self.buffered.first_entry() {
            if *entry.key() >= self.expected_seq {
                break;
            }
            out.push(entry.remove());
        }
        out
    }

    /// Lấp khe bằng dữ liệu phát lại từ luồng khôi phục. Khi mọi bản tin
    /// thiếu đã về đủ, ta rời chế độ khôi phục và chạy bình thường trở lại.
    pub fn fill_gap(&mut self, packets: Vec<FeedPacket>) {
        for g in packets {
            if g.seq >= self.expected_seq {
                self.buffered.insert(g.seq, g);
            }
        }
        self.advance();
    }

    pub fn buffered_len(&self) -> usize {
        self.buffered.len()
    }
}

// ============================================================================
// 3. SỔ LỆNH L2 — tổng hợp theo MỨC GIÁ
// ============================================================================
// L2 là thứ 95% chiến lược thật sự cần: mỗi mức giá còn bao nhiêu khối lượng.
// Nhẹ hơn L3 rất nhiều, và cập nhật nhanh hơn.

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct PriceLevel {
    pub price: Price,
    pub quantity: u64,
    pub order_count: u32,
}

#[derive(Debug, Default)]
pub struct L2Book {
    /// Bên mua lưu khoá ÂM để `BTreeMap` trả giá cao nhất trước.
    bids: BTreeMap<Price, (u64, u32)>,
    asks: BTreeMap<Price, (u64, u32)>,
}

impl L2Book {
    pub fn new() -> Self {
        L2Book::default()
    }

    pub fn add(&mut self, side: Side, price: Price, kl: Quantity) {
        let (map, key) = match side {
            Side::Buy => (&mut self.bids, -price),
            Side::Sell => (&mut self.asks, price),
        };
        let e = map.entry(key).or_insert((0, 0));
        e.0 += kl as u64;
        e.1 += 1;
    }

    /// Trả `true` nếu mức giá bị xoá hẳn khỏi sổ.
    pub fn reduce(&mut self, side: Side, price: Price, kl: Quantity, remove_order: bool) -> bool {
        let (map, key) = match side {
            Side::Buy => (&mut self.bids, -price),
            Side::Sell => (&mut self.asks, price),
        };
        if let Some(e) = map.get_mut(&key) {
            e.0 = e.0.saturating_sub(kl as u64);
            if remove_order {
                e.1 = e.1.saturating_sub(1);
            }
            // Mức giá hết khối lượng phải BIẾN MẤT, không được để lại mức rỗng —
            // nếu không, "giá tốt nhất" sẽ trỏ vào chỗ không có gì.
            if e.0 == 0 {
                map.remove(&key);
                return true;
            }
        }
        false
    }

    pub fn best_bid(&self) -> Option<Price> {
        self.bids.keys().next().map(|k| -k)
    }
    pub fn best_ask(&self) -> Option<Price> {
        self.asks.keys().next().copied()
    }
    pub fn spread(&self) -> Option<Price> {
        Some(self.best_ask()? - self.best_bid()?)
    }
    pub fn num_levels(&self, side: Side) -> usize {
        match side {
            Side::Buy => self.bids.len(),
            Side::Sell => self.asks.len(),
        }
    }
    pub fn qty_at(&self, side: Side, price: Price) -> u64 {
        let (map, k) = match side {
            Side::Buy => (&self.bids, -price),
            Side::Sell => (&self.asks, price),
        };
        map.get(&k).map_or(0, |e| e.0)
    }

    /// `n` mức giá tốt nhất mỗi bên — đúng thứ mà giao diện và chiến lược cần.
    pub fn top_levels(&self, n: usize) -> (Vec<PriceLevel>, Vec<PriceLevel>) {
        let m = self
            .bids
            .iter()
            .take(n)
            .map(|(k, v)| PriceLevel {
                price: -k,
                quantity: v.0,
                order_count: v.1,
            })
            .collect();
        let b = self
            .asks
            .iter()
            .take(n)
            .map(|(k, v)| PriceLevel {
                price: *k,
                quantity: v.0,
                order_count: v.1,
            })
            .collect();
        (m, b)
    }

    /// Giá bình quân gia quyền theo khối lượng đối ứng — ước lượng "giá trị
    /// thật" tốt hơn giá giữa, vì nó tính cả độ mất cân bằng cung cầu.
    pub fn micro_price(&self) -> Option<f64> {
        let (m, b) = self.top_levels(1);
        let (m, b) = (m.first()?, b.first()?);
        let total = (m.quantity + b.quantity) as f64;
        if total == 0.0 {
            return None;
        }
        // Bên nào NHIỀU khối lượng hơn thì giá cân bằng lệch về phía bên kia
        Some((m.price as f64 * b.quantity as f64 + b.price as f64 * m.quantity as f64) / total)
    }

    // ---- Kiểm tra chất lượng dữ liệu ----

    /// Sổ "khoá" (locked): giá mua = giá bán. Hiếm nhưng hợp lệ ở vài thị trường.
    pub fn is_locked(&self) -> bool {
        self.spread() == Some(0)
    }

    /// Sổ "chéo" (crossed): giá mua > giá bán. LUÔN LUÔN là dấu hiệu dữ liệu
    /// hỏng hoặc mất bản tin — phải dừng giao dịch ngay, đừng cố khai thác.
    pub fn is_crossed(&self) -> bool {
        self.spread().is_some_and(|c| c < 0)
    }

    pub fn is_healthy(&self) -> bool {
        !self.is_crossed()
    }
}

// ============================================================================
// 4. SỔ LỆNH L3 — theo TỪNG LỆNH
// ============================================================================
// L3 giữ danh tính từng lệnh. Nặng hơn nhiều, nhưng là thứ duy nhất trả lời
// được "lệnh của TÔI đang đứng thứ mấy trong hàng?" — câu hỏi sống còn với
// chiến lược tạo lập thị trường.

#[derive(Debug, Clone, PartialEq)]
pub struct L3Order {
    pub id: OrderId,
    pub side: Side,
    pub price: Price,
    pub remaining: Quantity,
}

#[derive(Debug, Default)]
pub struct L3Book {
    pub orders: HashMap<OrderId, L3Order>,
    /// Thứ tự tới của từng mức giá — nền của ưu tiên thời gian.
    queue: BTreeMap<(Side, Price), Vec<OrderId>>,
    pub l2: L2Book,
}

impl L3Book {
    pub fn new() -> Self {
        L3Book::default()
    }

    pub fn apply(&mut self, msg: &Message) {
        match msg {
            Message::AddOrder {
                id,
                side,
                price,
                quantity,
                ..
            } => {
                self.orders.insert(
                    *id,
                    L3Order {
                        id: *id,
                        side: *side,
                        price: *price,
                        remaining: *quantity,
                    },
                );
                self.queue.entry((*side, *price)).or_default().push(*id);
                self.l2.add(*side, *price, *quantity);
            }
            Message::CancelOrder {
                id,
                cancel_quantity,
            } => {
                if let Some(l) = self.orders.get_mut(id) {
                    let actually_cancelled = (*cancel_quantity).min(l.remaining);
                    l.remaining -= actually_cancelled;
                    let (c, g, done) = (l.side, l.price, l.remaining == 0);
                    self.l2.reduce(c, g, actually_cancelled, done);
                    if done {
                        self.remove_from_queue(*id, c, g);
                        self.orders.remove(id);
                    }
                }
            }
            Message::Fill { id, quantity, .. } => {
                if let Some(l) = self.orders.get_mut(id) {
                    let filled = (*quantity).min(l.remaining);
                    l.remaining -= filled;
                    let (c, g, done) = (l.side, l.price, l.remaining == 0);
                    self.l2.reduce(c, g, filled, done);
                    if done {
                        self.remove_from_queue(*id, c, g);
                        self.orders.remove(id);
                    }
                }
            }
            Message::Replaced {
                old_id,
                new_id,
                price,
                quantity,
            } => {
                // Thay thế = huỷ hẳn rồi thêm mới. Lệnh MẤT ưu tiên thời gian,
                // xuống cuối hàng — đây là lý do sửa lệnh rất đắt trong HFT.
                if let Some(l) = self.orders.remove(old_id) {
                    self.l2.reduce(l.side, l.price, l.remaining, true);
                    self.remove_from_queue(*old_id, l.side, l.price);
                    self.orders.insert(
                        *new_id,
                        L3Order {
                            id: *new_id,
                            side: l.side,
                            price: *price,
                            remaining: *quantity,
                        },
                    );
                    self.queue
                        .entry((l.side, *price))
                        .or_default()
                        .push(*new_id);
                    self.l2.add(l.side, *price, *quantity);
                }
            }
        }
    }

    fn remove_from_queue(&mut self, id: OrderId, c: Side, g: Price) {
        if let Some(h) = self.queue.get_mut(&(c, g)) {
            h.retain(|&x| x != id);
            if h.is_empty() {
                self.queue.remove(&(c, g));
            }
        }
    }

    /// Lệnh này đứng thứ mấy trong hàng ở mức giá của nó? (0 = đầu hàng)
    /// Câu trả lời quyết định xác suất được khớp.
    pub fn queue_position(&self, id: OrderId) -> Option<usize> {
        let l = self.orders.get(&id)?;
        self.queue
            .get(&(l.side, l.price))?
            .iter()
            .position(|&x| x == id)
    }

    /// Khối lượng đứng TRƯỚC lệnh này — phải khớp hết chỗ đó thì mới tới lượt ta.
    pub fn queue_ahead(&self, id: OrderId) -> Option<u64> {
        let l = self.orders.get(&id)?;
        let h = self.queue.get(&(l.side, l.price))?;
        let pos = h.iter().position(|&x| x == id)?;
        Some(
            h[..pos]
                .iter()
                .filter_map(|m| self.orders.get(m))
                .map(|x| x.remaining as u64)
                .sum(),
        )
    }

    pub fn open_orders(&self) -> usize {
        self.orders.len()
    }
}

// ============================================================================
// 5. SINH DỮ LIỆU PHIÊN TẤT ĐỊNH
// ============================================================================

pub fn generate_session(count: usize, seed: u64) -> Vec<FeedPacket> {
    let mut s = seed;
    let mut out = Vec::with_capacity(count);
    let mut order_id: u64 = 1;
    let mut open_orders: Vec<(OrderId, Side, Price, Quantity)> = Vec::new();
    let mut t: u64 = 1_000_000_000;

    for seq_no in 0..count as u64 {
        s = s
            .wrapping_mul(6364136223846793005)
            .wrapping_add(1442695040888963407);
        let r = (s >> 33) % 100;
        t += 1_000 + (s >> 20) % 50_000;

        // Giữ sổ có ít nhất vài lệnh trước khi bắt đầu huỷ/khớp
        let msg = if open_orders.len() < 4 || r < 55 {
            let side = if (s >> 40).is_multiple_of(2) {
                Side::Buy
            } else {
                Side::Sell
            };
            // Bên mua đặt dưới 8400, bên bán đặt trên 8400 → sổ không bao giờ chéo
            let offset = ((s >> 44) % 20) as i64;
            let price = match side {
                Side::Buy => 8_400 - 1 - offset,
                Side::Sell => 8_400 + 1 + offset,
            };
            let qty = 100 + ((s >> 48) % 10) as u32 * 100;
            open_orders.push((order_id, side, price, qty));
            let msg = Message::AddOrder {
                id: order_id,
                symbol_id: 1,
                side,
                price,
                quantity: qty,
            };
            order_id += 1;
            msg
        } else {
            let i = ((s >> 52) as usize) % open_orders.len();
            let (id, _, price, qty) = open_orders[i];
            let part = (qty / 2).max(1);
            if r < 80 {
                open_orders.remove(i);
                Message::CancelOrder {
                    id,
                    cancel_quantity: qty,
                }
            } else {
                open_orders[i].3 -= part;
                if open_orders[i].3 == 0 {
                    open_orders.remove(i);
                }
                Message::Fill {
                    id,
                    quantity: part,
                    price,
                }
            }
        };
        out.push(FeedPacket {
            seq: seq_no,
            timestamp_nanos: t,
            message: msg,
        });
    }
    out
}

fn main() {
    println!("═══════════════════════════════════════════════════════════");
    println!("   LUỒNG DỮ LIỆU THỊ TRƯỜNG: NHỊ PHÂN · KHE · SỔ L2/L3     ");
    println!("═══════════════════════════════════════════════════════════");

    println!("\n1. GIAO THỨC NHỊ PHÂN vs JSON");
    let g = FeedPacket {
        seq: 12345,
        timestamp_nanos: 1_700_000_000_000_000_000,
        message: Message::AddOrder {
            id: 999,
            symbol_id: 1,
            side: Side::Buy,
            price: 8_450,
            quantity: 100,
        },
    };
    let b = encode(&g);
    let json = r#"{"seq":12345,"ts":1700000000000000000,"type":"add","id":999,"sym":"VNM","side":"B","px":84.50,"qty":100}"#;
    println!("   Nhị phân: {} byte", b.len());
    println!(
        "   JSON    : {} byte → gấp {:.1} lần",
        json.len(),
        json.len() as f64 / b.len() as f64
    );
    println!(
        "   Phân tích ngược ra đúng bản gốc: {}",
        analyze(&b).unwrap() == g
    );

    println!("\n2. PHÁT HIỆN KHE SỐ THỨ TỰ");
    let mut detector = GapDetector::new(0);
    let session = generate_session(10, 7);
    for (i, pkt) in session.iter().enumerate() {
        if i == 3 || i == 4 {
            continue;
        } // giả lập mất 2 gói UDP
        let outcome = detector.receive(pkt.clone());
        if outcome != SeqOutcome::InOrder {
            println!("   seq {} → {:?}", pkt.seq, outcome);
        }
    }
    println!(
        "   Tổng khe: {} · tổng bản tin mất: {} · đang đệm: {}",
        detector.gap_count,
        detector.total_lost,
        detector.buffered_len()
    );
    println!("   Đang ở chế độ khôi phục: {}", detector.is_recovering());
    detector.fill_gap(vec![session[3].clone(), session[4].clone()]);
    println!(
        "   Sau khi phát lại → còn khôi phục: {} · rút liền mạch được {} bản tin",
        detector.is_recovering(),
        detector.drain().len()
    );

    println!("\n3. DỰNG SỔ L2 TỪ 5000 BẢN TIN");
    let mut book = L3Book::new();
    for g in generate_session(5_000, 42) {
        book.apply(&g.message);
    }
    let (bids, asks) = book.l2.top_levels(5);
    println!(
        "   {} lệnh đang mở · {} mức mua · {} mức bán",
        book.open_orders(),
        book.l2.num_levels(Side::Buy),
        book.l2.num_levels(Side::Sell)
    );
    println!("   ── 5 MỨC TỐT NHẤT ──");
    for m in asks.iter().rev() {
        println!(
            "        BÁN {:>7.2}  {:>6} ({} lệnh)",
            m.price as f64 / 100.0,
            m.quantity,
            m.order_count
        );
    }
    println!(
        "        ─────────────  chênh lệch {} tick",
        book.l2.spread().unwrap_or(0)
    );
    for m in &bids {
        println!(
            "        MUA {:>7.2}  {:>6} ({} lệnh)",
            m.price as f64 / 100.0,
            m.quantity,
            m.order_count
        );
    }
    println!(
        "   Giá cân bằng theo khối lượng: {:.2}",
        book.l2.micro_price().unwrap_or(0.0) / 100.0
    );

    println!("\n4. KIỂM TRA CHẤT LƯỢNG DỮ LIỆU");
    println!(
        "   Sổ lành mạnh: {} · bị khoá: {} · bị chéo: {}",
        book.l2.is_healthy(),
        book.l2.is_locked(),
        book.l2.is_crossed()
    );
    let mut broken = L2Book::new();
    broken.add(Side::Buy, 8_500, 100);
    broken.add(Side::Sell, 8_400, 100); // giá mua CAO hơn giá bán → vô lý
    println!(
        "   Sổ dựng sai (mua 85.00 > bán 84.00) → bị chéo: {} · lành mạnh: {}",
        broken.is_crossed(),
        broken.is_healthy()
    );
    println!("   → Gặp sổ chéo phải NGỪNG giao dịch, không được coi là cơ hội.");

    println!("\n5. VỊ TRÍ TRONG HÀNG — câu hỏi sống còn của tạo lập thị trường");
    let mut s3 = L3Book::new();
    for (id, qty) in [(1u64, 500u32), (2, 300), (3, 200)] {
        s3.apply(&Message::AddOrder {
            id,
            symbol_id: 1,
            side: Side::Buy,
            price: 8_400,
            quantity: qty,
        });
    }
    for id in [1u64, 2, 3] {
        println!(
            "   Lệnh #{} → đứng thứ {} · phải chờ {} đơn vị khớp trước",
            id,
            s3.queue_position(id).unwrap(),
            s3.queue_ahead(id).unwrap()
        );
    }
    s3.apply(&Message::Replaced {
        old_id: 1,
        new_id: 4,
        price: 8_400,
        quantity: 500,
    });
    println!(
        "   Sửa lệnh #1 (thành #4) → giờ đứng thứ {} — MẤT SẠCH ưu tiên thời gian",
        s3.queue_position(4).unwrap()
    );

    println!("\n═══════════════════════════════════════════════════════════");
    println!("   SAI MỘT BẢN TIN LÀ SAI TOÀN BỘ QUYẾT ĐỊNH SAU ĐÓ         ");
    println!("═══════════════════════════════════════════════════════════");
}

#[cfg(test)]
mod tests {
    use super::*;

    // ---------- Giao thức nhị phân ----------
    #[test]
    fn encode_then_parse_round_trips() {
        let all_msgs = vec![
            Message::AddOrder {
                id: 1,
                symbol_id: 7,
                side: Side::Buy,
                price: 8_450,
                quantity: 100,
            },
            Message::AddOrder {
                id: 2,
                symbol_id: 7,
                side: Side::Sell,
                price: -50,
                quantity: 1,
            },
            Message::CancelOrder {
                id: 3,
                cancel_quantity: 250,
            },
            Message::Fill {
                id: 4,
                quantity: 75,
                price: 8_400,
            },
            Message::Replaced {
                old_id: 5,
                new_id: 6,
                price: 8_390,
                quantity: 999,
            },
        ];
        for msg in all_msgs {
            let g = FeedPacket {
                seq: 42,
                timestamp_nanos: 1_700_000_000_000_000_000,
                message: msg,
            };
            assert_eq!(
                analyze(&encode(&g)),
                Ok(g.clone()),
                "vòng mã hoá phải khép kín"
            );
        }
    }

    #[test]
    fn parser_rejects_short_packets() {
        assert_eq!(
            analyze(&[]),
            Err(ParseError::TooShort { needed: 17, got: 0 })
        );
        assert_eq!(
            analyze(&[b'A'; 10]),
            Err(ParseError::TooShort {
                needed: 17,
                got: 10
            })
        );
        // Đủ phần đầu chung nhưng thiếu thân bản tin 'A'
        let mut b = vec![b'A'];
        b.extend_from_slice(&[0u8; 20]);
        assert!(matches!(analyze(&b), Err(ParseError::TooShort { .. })));
    }

    #[test]
    fn parser_rejects_unknown_message_type() {
        let mut b = vec![b'Z'];
        b.extend_from_slice(&[0u8; 60]);
        assert_eq!(analyze(&b), Err(ParseError::UnknownMessageKind(b'Z')));
    }

    #[test]
    fn parser_rejects_unknown_side_code() {
        let g = FeedPacket {
            seq: 1,
            timestamp_nanos: 1,
            message: Message::AddOrder {
                id: 1,
                symbol_id: 1,
                side: Side::Buy,
                price: 100,
                quantity: 1,
            },
        };
        let mut b = encode(&g);
        b[29] = b'?'; // phá byte chiều
        assert_eq!(analyze(&b), Err(ParseError::UnknownSide(b'?')));
    }

    #[test]
    fn binary_is_far_smaller_than_json() {
        let g = FeedPacket {
            seq: 12345,
            timestamp_nanos: 1_700_000_000_000_000_000,
            message: Message::AddOrder {
                id: 999,
                symbol_id: 1,
                side: Side::Buy,
                price: 8_450,
                quantity: 100,
            },
        };
        assert_eq!(
            encode(&g).len(),
            42,
            "bản tin thêm lệnh dài đúng 42 byte cố định"
        );
        assert!(
            encode(&g).len() * 2 < 105,
            "nhị phân phải gọn hơn JSON ít nhất 2 lần"
        );
    }

    #[test]
    fn uses_big_endian_byte_order() {
        // Giao thức mạng LUÔN dùng big-endian. Nhầm sang little-endian thì
        // số nhỏ vẫn "chạy" nhưng giá trị hoàn toàn sai.
        let g = FeedPacket {
            seq: 0x0102030405060708,
            timestamp_nanos: 0,
            message: Message::CancelOrder {
                id: 1,
                cancel_quantity: 1,
            },
        };
        let b = encode(&g);
        assert_eq!(&b[1..9], &[1, 2, 3, 4, 5, 6, 7, 8], "byte cao đứng TRƯỚC");
    }

    // ---------- Phát hiện khe ----------
    #[test]
    fn a_contiguous_stream_reports_no_gap() {
        let mut p = GapDetector::new(0);
        for g in generate_session(100, 1) {
            assert_eq!(p.receive(g), SeqOutcome::InOrder);
        }
        assert_eq!(p.gap_count, 0);
        assert_eq!(p.expected_seq, 100);
    }

    #[test]
    fn detects_the_gap_and_counts_lost_messages() {
        let session = generate_session(10, 2);
        let mut p = GapDetector::new(0);
        for (i, g) in session.iter().enumerate() {
            if (3..=5).contains(&i) {
                continue;
            } // mất gói 3,4,5
            let outcome = p.receive(g.clone());
            if i == 6 {
                assert_eq!(
                    outcome,
                    SeqOutcome::MissingMessages {
                        from: 3,
                        to: 5,
                        count: 3
                    }
                );
            } else if i > 6 {
                assert_eq!(
                    outcome,
                    SeqOutcome::AwaitingRecovery,
                    "các bản tin sau chỉ được đệm, không xin phát lại nữa"
                );
            }
        }
        assert_eq!(p.gap_count, 1);
        assert_eq!(p.total_lost, 3);
    }

    #[test]
    fn requests_retransmission_only_once_per_gap() {
        // Nếu báo khe ở mọi bản tin sau đó, ta sẽ gửi hàng nghìn yêu cầu phát
        // lại cho cùng một khe và tự làm sập luồng khôi phục của sàn.
        let session = generate_session(20, 8);
        let mut p = GapDetector::new(0);
        let mut gap_count = 0;
        for (i, g) in session.iter().enumerate() {
            if (3..=5).contains(&i) {
                continue;
            }
            if matches!(p.receive(g.clone()), SeqOutcome::MissingMessages { .. }) {
                gap_count += 1;
            }
        }
        assert_eq!(gap_count, 1, "một khe chỉ được xin phát lại đúng một lần");
        assert_eq!(p.gap_count, 1);
        assert_eq!(p.total_lost, 3);
        assert!(p.is_recovering(), "vẫn đang chờ dữ liệu phát lại");
    }

    #[test]
    fn leaves_recovery_once_the_gap_is_filled() {
        let session = generate_session(20, 8);
        let mut p = GapDetector::new(0);
        for (i, g) in session.iter().enumerate() {
            if (3..=5).contains(&i) {
                continue;
            }
            p.receive(g.clone());
        }
        assert!(p.is_recovering());
        p.fill_gap(vec![session[3].clone(), session[4].clone()]);
        assert!(
            p.is_recovering(),
            "còn thiếu bản tin 5 thì vẫn đang khôi phục"
        );
        p.fill_gap(vec![session[5].clone()]);
        assert!(!p.is_recovering(), "đủ rồi thì phải trở lại bình thường");
        assert_eq!(p.drain().len(), 20);
    }

    #[test]
    fn duplicates_from_the_redundant_feed_are_skipped() {
        // Sàn thường phát hai luồng giống hệt (A và B) để chống mất gói.
        // Bản sao đến sau phải bị loại, không được xử lý hai lần.
        let session = generate_session(5, 3);
        let mut p = GapDetector::new(0);
        for g in &session {
            p.receive(g.clone());
        }
        for g in &session {
            assert_eq!(p.receive(g.clone()), SeqOutcome::Duplicate);
        }
        assert_eq!(p.duplicate_count, 5);
        assert_eq!(p.expected_seq, 5, "trùng lặp không được đẩy kỳ vọng đi");
    }

    #[test]
    fn early_messages_are_buffered_not_dropped() {
        let session = generate_session(10, 4);
        let mut p = GapDetector::new(0);
        p.receive(session[0].clone());
        p.receive(session[5].clone()); // nhảy cóc
        assert_eq!(p.buffered_len(), 2, "cả hai đều phải được giữ lại");
        assert_eq!(p.drain().len(), 1, "chỉ rút được phần liền mạch từ đầu");
    }

    #[test]
    fn filling_the_gap_drains_everything() {
        let session = generate_session(10, 5);
        let mut p = GapDetector::new(0);
        for (i, g) in session.iter().enumerate() {
            if i == 3 || i == 4 {
                continue;
            }
            p.receive(g.clone());
        }
        p.fill_gap(vec![session[3].clone(), session[4].clone()]);
        let out = p.drain();
        assert_eq!(out.len(), 10, "sau khi lấp khe phải rút được đủ 10 bản tin");
        for (i, g) in out.iter().enumerate() {
            assert_eq!(g.seq, i as u64, "và đúng thứ tự");
        }
    }

    #[test]
    fn a_late_packet_closes_the_gap_by_itself() {
        // Trước đây bản tin đúng thứ tự chỉ đẩy kỳ vọng +1, nên khi gói 3 tới
        // trễ (UDP đảo thứ tự) bộ phát hiện bỏ qua các gói 4.. đã đệm và kẹt
        // mãi trong chế độ khôi phục.
        let session = generate_session(8, 6);
        let mut p = GapDetector::new(0);
        for i in [0, 1, 2, 4, 5, 3, 6, 7] {
            p.receive(session[i].clone());
        }
        assert!(
            !p.is_recovering(),
            "khe đã tự lấp, phải rời chế độ khôi phục"
        );
        assert_eq!(p.expected_seq, 8);
        assert_eq!(p.gap_count, 1);
        let seqs: Vec<u64> = p.drain().iter().map(|g| g.seq).collect();
        assert_eq!(seqs, (0..8).collect::<Vec<u64>>());
    }

    #[test]
    fn drain_stops_at_the_first_gap() {
        // Trước đây `drain` bắt đầu từ khoá nhỏ nhất trong bộ đệm, nên sau khi
        // phần đầu đã rút, nó rút luôn các bản tin NẰM SAU khe.
        let session = generate_session(10, 4);
        let mut p = GapDetector::new(0);
        p.receive(session[0].clone());
        assert_eq!(p.drain().len(), 1);
        p.receive(session[5].clone());
        p.receive(session[6].clone());
        assert!(p.drain().is_empty(), "thiếu 1..=4 thì chưa được xử lý 5, 6");
        assert_eq!(p.buffered_len(), 2);
    }

    #[test]
    fn a_second_gap_behind_the_first_is_also_requested() {
        // Khe 3..=5 đang chờ, rồi lại mất gói 8. Gói 8 phải được xin phát lại
        // (đúng một lần) — nếu không ta chờ mãi một thứ chưa từng xin.
        let session = generate_session(12, 8);
        let mut p = GapDetector::new(0);
        let mut requests = Vec::new();
        for (i, g) in session.iter().enumerate() {
            if (3..=5).contains(&i) || i == 8 {
                continue;
            }
            if let SeqOutcome::MissingMessages { from, to, .. } = p.receive(g.clone()) {
                requests.push((from, to));
            }
        }
        assert_eq!(requests, vec![(3, 5), (8, 8)]);
        assert_eq!(p.total_lost, 4);
        p.fill_gap(vec![
            session[3].clone(),
            session[4].clone(),
            session[5].clone(),
        ]);
        assert!(p.is_recovering(), "còn thiếu gói 8");
        p.fill_gap(vec![session[8].clone()]);
        assert!(!p.is_recovering());
        assert_eq!(p.drain().len(), 12);
    }

    #[test]
    fn a_duplicate_of_a_buffered_packet_is_counted_once() {
        let session = generate_session(10, 9);
        let mut p = GapDetector::new(0);
        p.receive(session[0].clone());
        p.receive(session[5].clone());
        assert_eq!(p.receive(session[5].clone()), SeqOutcome::Duplicate);
        assert_eq!(p.duplicate_count, 1);
        assert_eq!(p.buffered_len(), 2);
    }

    // ---------- Sổ L2 ----------
    #[test]
    fn l2_reports_best_on_both_sides() {
        let mut s = L2Book::new();
        s.add(Side::Buy, 8_390, 100);
        s.add(Side::Buy, 8_400, 200); // cao hơn = tốt hơn cho bên mua
        s.add(Side::Sell, 8_420, 150);
        s.add(Side::Sell, 8_410, 50); // thấp hơn = tốt hơn cho bên bán
        assert_eq!(s.best_bid(), Some(8_400));
        assert_eq!(s.best_ask(), Some(8_410));
        assert_eq!(s.spread(), Some(10));
    }

    #[test]
    fn l2_aggregates_size_and_order_count_per_level() {
        let mut s = L2Book::new();
        for _ in 0..3 {
            s.add(Side::Buy, 8_400, 100);
        }
        let (m, _) = s.top_levels(1);
        assert_eq!(m[0].quantity, 300);
        assert_eq!(m[0].order_count, 3);
    }

    #[test]
    fn an_emptied_level_disappears_from_the_book() {
        // Nếu để lại mức rỗng, `best_bid` sẽ trỏ vào chỗ không có gì —
        // và chiến lược sẽ gửi lệnh vào hư không.
        let mut s = L2Book::new();
        s.add(Side::Buy, 8_400, 100);
        s.add(Side::Buy, 8_390, 50);
        assert!(
            s.reduce(Side::Buy, 8_400, 100, true),
            "phải báo mức giá đã bị xoá"
        );
        assert_eq!(s.best_bid(), Some(8_390), "đỉnh sổ phải tụt xuống mức kế");
        assert_eq!(s.num_levels(Side::Buy), 1);
    }

    #[test]
    fn over_reducing_never_makes_size_negative() {
        let mut s = L2Book::new();
        s.add(Side::Buy, 8_400, 100);
        assert!(
            s.reduce(Side::Buy, 8_400, 99_999, true),
            "trừ quá cũng chỉ về 0"
        );
        assert_eq!(s.qty_at(Side::Buy, 8_400), 0);
        assert_eq!(s.num_levels(Side::Buy), 0);
    }

    #[test]
    fn empty_book_neither_panics_nor_reports_crossed() {
        let s = L2Book::new();
        assert_eq!(s.best_bid(), None);
        assert_eq!(s.spread(), None);
        assert!(!s.is_crossed() && !s.is_locked() && s.is_healthy());
        assert_eq!(s.micro_price(), None);
    }

    #[test]
    fn detects_crossed_and_locked_books() {
        let mut crossed = L2Book::new();
        crossed.add(Side::Buy, 8_500, 100);
        crossed.add(Side::Sell, 8_400, 100);
        assert!(
            crossed.is_crossed(),
            "mua 85.00 > bán 84.00 là dữ liệu hỏng"
        );
        assert!(!crossed.is_healthy());

        let mut locked = L2Book::new();
        locked.add(Side::Buy, 8_400, 100);
        locked.add(Side::Sell, 8_400, 100);
        assert!(
            locked.is_locked() && !locked.is_crossed(),
            "sổ khoá là hiếm nhưng hợp lệ, khác hẳn sổ chéo"
        );
        assert!(locked.is_healthy());
    }

    #[test]
    fn fair_price_leans_toward_the_thin_side() {
        // Nhiều người muốn mua hơn bán → áp lực đẩy giá lên → giá cân bằng
        // phải gần giá BÁN hơn.
        let mut s = L2Book::new();
        s.add(Side::Buy, 8_400, 900);
        s.add(Side::Sell, 8_410, 100);
        let fair = s.micro_price().unwrap();
        assert!(
            fair > 8_405.0,
            "áp lực mua mạnh → giá cân bằng {} phải lệch lên trên",
            fair
        );
        assert!(fair < 8_410.0);
    }

    #[test]
    fn top_of_book_respects_priority_order() {
        let mut s = L2Book::new();
        for g in [8_380, 8_390, 8_400] {
            s.add(Side::Buy, g, 100);
        }
        for g in [8_430, 8_420, 8_410] {
            s.add(Side::Sell, g, 100);
        }
        let (m, b) = s.top_levels(3);
        assert_eq!(
            m.iter().map(|x| x.price).collect::<Vec<_>>(),
            vec![8_400, 8_390, 8_380],
            "bên mua: giá cao xuống thấp"
        );
        assert_eq!(
            b.iter().map(|x| x.price).collect::<Vec<_>>(),
            vec![8_410, 8_420, 8_430],
            "bên bán: giá thấp lên cao"
        );
    }

    // ---------- Sổ L3 ----------
    #[test]
    fn l3_and_l2_stay_consistent_over_a_long_session() {
        // BẤT BIẾN QUAN TRỌNG NHẤT của chương: L2 phải luôn là bản tổng hợp
        // đúng của L3. Lệch nhau nghĩa là có bản tin bị xử lý sai.
        let mut s = L3Book::new();
        for g in generate_session(3_000, 99) {
            s.apply(&g.message);
            assert!(
                s.l2.is_healthy(),
                "sổ không bao giờ được chéo khi dữ liệu sạch"
            );
        }
        // Dựng lại L2 từ L3 rồi so
        let mut check = L2Book::new();
        for l in s.orders.values() {
            check.add(l.side, l.price, l.remaining);
        }
        assert_eq!(check.best_bid(), s.l2.best_bid());
        assert_eq!(check.best_ask(), s.l2.best_ask());
        assert_eq!(check.num_levels(Side::Buy), s.l2.num_levels(Side::Buy));
        assert_eq!(check.num_levels(Side::Sell), s.l2.num_levels(Side::Sell));
        for l in s.orders.values() {
            assert_eq!(check.qty_at(l.side, l.price), s.l2.qty_at(l.side, l.price));
        }
    }

    #[test]
    fn l3_preserves_time_priority() {
        let mut s = L3Book::new();
        for (id, qty) in [(1u64, 500u32), (2, 300), (3, 200)] {
            s.apply(&Message::AddOrder {
                id,
                symbol_id: 1,
                side: Side::Buy,
                price: 8_400,
                quantity: qty,
            });
        }
        assert_eq!(s.queue_position(1), Some(0));
        assert_eq!(s.queue_position(2), Some(1));
        assert_eq!(s.queue_position(3), Some(2));
        assert_eq!(s.queue_ahead(1), Some(0), "đầu hàng thì không chờ ai");
        assert_eq!(s.queue_ahead(2), Some(500));
        assert_eq!(s.queue_ahead(3), Some(800));
    }

    #[test]
    fn filling_the_head_advances_the_whole_queue() {
        let mut s = L3Book::new();
        for (id, qty) in [(1u64, 500u32), (2, 300)] {
            s.apply(&Message::AddOrder {
                id,
                symbol_id: 1,
                side: Side::Buy,
                price: 8_400,
                quantity: qty,
            });
        }
        s.apply(&Message::Fill {
            id: 1,
            quantity: 500,
            price: 8_400,
        });
        assert_eq!(s.queue_position(2), Some(0), "lệnh #2 lên đầu hàng");
        assert_eq!(s.queue_ahead(2), Some(0));
        assert_eq!(s.open_orders(), 1);
    }

    #[test]
    fn a_partial_fill_keeps_queue_position() {
        let mut s = L3Book::new();
        for (id, qty) in [(1u64, 500u32), (2, 300)] {
            s.apply(&Message::AddOrder {
                id,
                symbol_id: 1,
                side: Side::Buy,
                price: 8_400,
                quantity: qty,
            });
        }
        s.apply(&Message::Fill {
            id: 1,
            quantity: 200,
            price: 8_400,
        });
        assert_eq!(s.queue_position(1), Some(0), "khớp một phần KHÔNG mất chỗ");
        assert_eq!(s.queue_ahead(2), Some(300), "chỉ còn 300 đứng trước");
        assert_eq!(s.l2.qty_at(Side::Buy, 8_400), 600);
    }

    #[test]
    fn replacing_an_order_forfeits_time_priority() {
        // Bài học đắt tiền: sửa giá/khối lượng một lệnh = xuống cuối hàng.
        // Đó là lý do chiến lược tốt cân nhắc rất kỹ trước khi sửa lệnh.
        let mut s = L3Book::new();
        for (id, qty) in [(1u64, 500u32), (2, 300), (3, 200)] {
            s.apply(&Message::AddOrder {
                id,
                symbol_id: 1,
                side: Side::Buy,
                price: 8_400,
                quantity: qty,
            });
        }
        assert_eq!(s.queue_position(1), Some(0));
        s.apply(&Message::Replaced {
            old_id: 1,
            new_id: 4,
            price: 8_400,
            quantity: 500,
        });
        assert_eq!(s.queue_position(1), None, "mã cũ biến mất");
        assert_eq!(s.queue_position(4), Some(2), "mã mới xuống CUỐI hàng");
        assert_eq!(s.queue_ahead(4), Some(500));
    }

    #[test]
    fn cancelling_an_unknown_order_leaves_the_book_intact() {
        let mut s = L3Book::new();
        s.apply(&Message::AddOrder {
            id: 1,
            symbol_id: 1,
            side: Side::Buy,
            price: 8_400,
            quantity: 100,
        });
        s.apply(&Message::CancelOrder {
            id: 999,
            cancel_quantity: 50,
        }); // mã lạ
        assert_eq!(s.open_orders(), 1);
        assert_eq!(s.l2.qty_at(Side::Buy, 8_400), 100, "sổ phải nguyên vẹn");
    }

    #[test]
    fn cancelling_more_than_remaining_is_safe() {
        let mut s = L3Book::new();
        s.apply(&Message::AddOrder {
            id: 1,
            symbol_id: 1,
            side: Side::Buy,
            price: 8_400,
            quantity: 100,
        });
        s.apply(&Message::CancelOrder {
            id: 1,
            cancel_quantity: 99_999,
        });
        assert_eq!(s.open_orders(), 0);
        assert_eq!(s.l2.num_levels(Side::Buy), 0);
    }

    #[test]
    fn position_of_an_unknown_order_is_none() {
        let s = L3Book::new();
        assert_eq!(s.queue_position(123), None);
        assert_eq!(s.queue_ahead(123), None);
    }

    // ---------- Sinh dữ liệu ----------
    #[test]
    fn generated_session_is_deterministic_and_gapless() {
        assert_eq!(generate_session(50, 9), generate_session(50, 9));
        assert_ne!(generate_session(50, 9), generate_session(50, 10));
        let p = generate_session(200, 1);
        for (i, g) in p.iter().enumerate() {
            assert_eq!(g.seq, i as u64);
        }
    }

    #[test]
    fn session_timestamps_are_monotonic() {
        let p = generate_session(500, 3);
        for w in p.windows(2) {
            assert!(
                w[1].timestamp_nanos > w[0].timestamp_nanos,
                "dấu thời gian phải tăng — nền tảng cho phát lại ở Chương 76"
            );
        }
    }

    #[test]
    fn every_generated_message_round_trips() {
        for g in generate_session(500, 11) {
            assert_eq!(analyze(&encode(&g)), Ok(g.clone()));
        }
    }
}
```

---

## Bảng tra cứu lỗi biên dịch thường gặp

| Lỗi | Nguyên nhân trong chương này | Cách sửa |
|---|---|---|
| `E0308: mismatched types` (expected `[u8; 8]`, found `&[u8]`) | `from_be_bytes` cần mảng cố định | Kiểm độ dài trước rồi `b[i..i + 8].try_into().unwrap()` như `analyze`, hoặc `.try_into().map_err(...)?` |
| Giá sai lệch hàng triệu lần | Dùng `from_le_bytes` cho giao thức mạng | Giao thức mạng là big-endian: `from_be_bytes` |
| `E0502: cannot borrow as mutable because it is also borrowed as immutable` | Duyệt `self.levels` rồi muốn `remove` trong vòng lặp | Thu khoá cần xoá vào `Vec` trước, xoá sau — hoặc dùng `retain` |
| Khe bị báo lại vô hạn | Thiếu trạng thái "đang chờ khôi phục" | Thêm `pending_gap: Option<(u64,u64)>` |
| `E0507: cannot move out of index of BTreeMap` | `let v = map[&k];` với giá trị là `Vec` | `.remove(&k)` để lấy quyền sở hữu, hoặc mượn `&map[&k]` |

---

## Tóm tắt chương & Bài tập rèn luyện

### 5 điểm cốt lõi

1. **Giao thức nhị phân trường cố định nhanh hơn JSON khoảng 25 lần** — không tìm kiếm, không cấp phát.
2. **UDP multicast mất gói và bạn phải tự xử.** Yêu cầu phát lại **đúng một lần** — bão yêu cầu còn tệ hơn mất gói.
3. **L3 cho biết vị trí xếp hàng**; vị trí xếp hàng quyết định bạn có bị chọn lọc bất lợi hay không.
4. **`BTreeMap` cho tính tất định**, và tính tất định là điều kiện để phát lại và gỡ lỗi được.
5. **Giảm khối lượng giữ vị trí, huỷ-đặt-lại thì mất.** Một chi tiết nhỏ nhưng ảnh hưởng trực tiếp tới lợi nhuận.

### Bài tập rèn luyện

**Bài 1.** Cài **sổ lệnh gia tăng có kiểm tra bằng ảnh chụp**: dựng sổ từ luồng cập nhật rồi định kỳ đối chiếu với ảnh chụp đầy đủ từ sàn.

<details>
<summary><b>Gợi ý</b></summary>

Sổ lệnh dựng gia tăng sẽ **trôi** theo thời gian — vì gói mất, vì lỗi cài đặt, vì trường hợp biên. Các sàn phát ảnh chụp định kỳ đúng để bạn phát hiện điều đó. Phát hiện lệch thì phải xây lại từ ảnh chụp, không cố "vá".
</details>

<details>
<summary><b>Lời giải</b></summary>

```rust
#[derive(Debug, PartialEq)]
pub enum ReconcileOutcome {
    Match,
    Mismatch { levels: usize, details: Vec<String> },
}

impl L2Book {
    /// Một bên của sổ dưới dạng bản đồ khoá → (khối lượng, số lệnh).
    /// Lưu ý: khoá bên mua là giá ĐẢO DẤU (xem `L2Book`).
    pub fn levels(&self, side: Side) -> &BTreeMap<Price, (u64, u32)> {
        match side {
            Side::Buy => &self.bids,
            Side::Sell => &self.asks,
        }
    }

    pub fn reconcile(&self, snap: &L2Book) -> ReconcileOutcome {
        let mut details = Vec::new();
        for side in [Side::Buy, Side::Sell] {
            let (ours, theirs) = (self.levels(side), snap.levels(side));
            for (key, level) in ours {
                match theirs.get(key) {
                    Some(t) if t == level => {}
                    Some(t) => details.push(format!(
                        "{side:?} {key}: của ta={level:?} ảnh chụp={t:?}"
                    )),
                    None => details.push(format!(
                        "{side:?} {key}: của ta={level:?} ảnh chụp=THIẾU"
                    )),
                }
            }
            for key in theirs.keys() {
                if !ours.contains_key(key) {
                    details.push(format!("{side:?} {key}: của ta=THIẾU"));
                }
            }
        }
        if details.is_empty() {
            ReconcileOutcome::Match
        } else {
            ReconcileOutcome::Mismatch {
                levels: details.len(),
                details,
            }
        }
    }

    /// Khi lệch: XÂY LẠI, không vá. Sổ đã sai thì mọi phép vá đều đoán mò.
    pub fn rebuild_from(&mut self, snap: &L2Book) {
        self.bids = snap.bids.clone();
        self.asks = snap.asks.clone();
    }
}
```

Nguyên tắc vận hành: **phát hiện lệch → xây lại → ghi nhật ký → cảnh báo**. Đừng bao giờ cố vá một sổ đã lệch; bạn không biết nó sai từ đâu.
</details>

**Bài 2.** Cài **bộ theo dõi vị trí xếp hàng** cho lệnh của chính mình khi có luồng L3.

<details>
<summary><b>Gợi ý</b></summary>

Vị trí xếp hàng (Queue position) giảm khi lệnh đứng trước bị khớp **hoặc bị huỷ**. Nó không đổi khi có lệnh mới xếp sau bạn. Theo dõi số này cho phép ước lượng xác suất được khớp — và quyết định có nên đặt lại lệnh hay không.
</details>

<details>
<summary><b>Lời giải</b></summary>

```rust
pub struct QueueTracker {
    pub our_order_id: OrderId,
    pub price: Price,
    pub queue_ahead: u64,
    pub initial_queue_ahead: u64,
}

impl QueueTracker {
    /// Sổ L3 đã cho sẵn `queue_ahead` — ta chỉ chụp lại giá trị đó
    /// tại thời điểm đặt lệnh để về sau đo được tiến độ.
    pub fn new(book: &L3Book, id: OrderId) -> Option<Self> {
        let order = book.orders.get(&id)?;
        let ahead = book.queue_ahead(id)?;
        Some(QueueTracker {
            our_order_id: id,
            price: order.price,
            queue_ahead: ahead,
            initial_queue_ahead: ahead,
        })
    }

    /// Lệnh đứng trước bị khớp HOẶC bị huỷ → hàng ngắn lại.
    pub fn queue_shrank(&mut self, quantity: u64) {
        self.queue_ahead = self.queue_ahead.saturating_sub(quantity);
    }

    /// Tỉ lệ đã tiến được, 0.0 → 1.0.
    pub fn progress(&self) -> f64 {
        if self.initial_queue_ahead == 0 {
            return 1.0;
        }
        1.0 - self.queue_ahead as f64 / self.initial_queue_ahead as f64
    }

    /// Ước lượng thô xác suất được khớp trước khi giá đi mất.
    pub fn fill_probability(&self, expected_volume: u64) -> f64 {
        if self.queue_ahead == 0 {
            return 1.0;
        }
        (expected_volume as f64 / self.queue_ahead as f64).min(1.0)
    }
}
```

Con số `fill_probability` là đầu vào trực tiếp cho quyết định giao dịch: nếu xác suất quá thấp, tốt hơn là huỷ và đặt ở mức giá tốt hơn — chấp nhận chênh lệch nhỏ hơn để đổi lấy khả năng được khớp.
</details>
