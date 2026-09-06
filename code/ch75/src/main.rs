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

pub type Price = i64;      // tick, 1 tick = 0,01 đơn vị tiền
pub type Quantity = u32;
pub type OrderId = u64;

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum Side { Buy, Sell }

#[derive(Debug, Clone, PartialEq)]
pub enum BanTin {
    /// Thêm lệnh mới vào sổ
    AddOrder { id: OrderId, id_chain: u32, side: Side, price: Price, quantity: Quantity },
    /// Lệnh bị huỷ một phần hoặc toàn bộ
    CancelOrder { id: OrderId, cancel_quantity: Quantity },
    /// Lệnh khớp
    Fill { id: OrderId, quantity: Quantity, price: Price },
    /// Thay thế lệnh: huỷ cũ, tạo mới, MẤT ưu tiên thời gian
    Replaced { old_id: OrderId, ma_moi: OrderId, price: Price, quantity: Quantity },
}

#[derive(Debug, Clone, PartialEq)]
pub struct FieldPacket {
    pub nonce: u64,
    pub timestamp_nanos: u64,
    pub ban_tin: BanTin,
}

#[derive(Debug, PartialEq)]
pub enum ErrorAnalyze {
    TooShort { can: usize, co: usize },
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
pub fn analyze(b: &[u8]) -> Result<FieldPacket, ErrorAnalyze> {
    if b.len() < 17 { return Err(ErrorAnalyze::TooShort { can: 17, co: b.len() }); }
    let kind = b[0];
    let nonce = u64::from_be_bytes(b[1..9].try_into().unwrap());
    let timestamp_nanos = u64::from_be_bytes(b[9..17].try_into().unwrap());

    let can = match kind { b'A' => 42, b'X' => 29, b'E' => 37, b'R' => 45, _ => 17 };
    if b.len() < can { return Err(ErrorAnalyze::TooShort { can, co: b.len() }); }

    let doc_u32 = |i: usize| -> u32 { u32::from_be_bytes(b[i..i + 4].try_into().unwrap()) };
    let doc_i64 = |i: usize| -> i64 { i64::from_be_bytes(b[i..i + 8].try_into().unwrap()) };
    let doc_u64 = |i: usize| -> u64 { u64::from_be_bytes(b[i..i + 8].try_into().unwrap()) };

    let ban_tin = match kind {
        b'A' => BanTin::AddOrder {
            id: doc_u64(17), id_chain: doc_u32(25),
            side: match b[29] { b'B' => Side::Buy, b'S' => Side::Sell,
                                 x => return Err(ErrorAnalyze::UnknownSide(x)) },
            price: doc_i64(30), quantity: doc_u32(38),
        },
        b'X' => BanTin::CancelOrder { id: doc_u64(17), cancel_quantity: doc_u32(25) },
        b'E' => BanTin::Fill { id: doc_u64(17), quantity: doc_u32(25), price: doc_i64(29) },
        b'R' => BanTin::Replaced {
            old_id: doc_u64(17), ma_moi: doc_u64(25),
            price: doc_i64(33), quantity: doc_u32(41),
        },
        x => return Err(ErrorAnalyze::UnknownMessageKind(x)),
    };
    Ok(FieldPacket { nonce, timestamp_nanos, ban_tin })
}

/// Mã hoá ngược — dùng để sinh dữ liệu kiểm thử và để ghi lại phiên (Chương 76).
pub fn encode(g: &FieldPacket) -> Vec<u8> {
    let mut v = Vec::with_capacity(48);
    let kind = match g.ban_tin {
        BanTin::AddOrder { .. } => b'A', BanTin::CancelOrder { .. } => b'X',
        BanTin::Fill { .. } => b'E', BanTin::Replaced { .. } => b'R',
    };
    v.push(kind);
    v.extend_from_slice(&g.nonce.to_be_bytes());
    v.extend_from_slice(&g.timestamp_nanos.to_be_bytes());
    match &g.ban_tin {
        BanTin::AddOrder { id, id_chain, side, price, quantity } => {
            v.extend_from_slice(&id.to_be_bytes());
            v.extend_from_slice(&id_chain.to_be_bytes());
            v.push(if *side == Side::Buy { b'B' } else { b'S' });
            v.extend_from_slice(&price.to_be_bytes());
            v.extend_from_slice(&quantity.to_be_bytes());
        }
        BanTin::CancelOrder { id, cancel_quantity } => {
            v.extend_from_slice(&id.to_be_bytes());
            v.extend_from_slice(&cancel_quantity.to_be_bytes());
        }
        BanTin::Fill { id, quantity, price } => {
            v.extend_from_slice(&id.to_be_bytes());
            v.extend_from_slice(&quantity.to_be_bytes());
            v.extend_from_slice(&price.to_be_bytes());
        }
        BanTin::Replaced { old_id, ma_moi, price, quantity } => {
            v.extend_from_slice(&old_id.to_be_bytes());
            v.extend_from_slice(&ma_moi.to_be_bytes());
            v.extend_from_slice(&price.to_be_bytes());
            v.extend_from_slice(&quantity.to_be_bytes());
        }
    }
    v
}

// ============================================================================
// 2. PHÁT HIỆN KHE SỐ THỨ TỰ
// ============================================================================
// Dữ liệu thị trường thường đi qua UDP multicast: fast, nhưng KHÔNG bảo đảm
// tới nơi và KHÔNG bảo đảm đúng thứ tự. Số thứ tự là thứ duy nhất cho ta biết
// mình có đang nhìn bức tranh đầy đủ hay không.

#[derive(Debug, Clone, Copy, PartialEq)]
pub enum KetQuaNhan {
    /// Đúng bản tin kế tiếp — xử lý ngay
    InOrder,
    /// Bản tin cũ (bản sao từ luồng dự phòng) — bỏ qua
    Duplicate,
    /// PHÁT HIỆN khe lần đầu: thiếu `so_ban_tin_mat` bản tin.
    /// Đây là lúc DUY NHẤT ta gửi yêu cầu phát lại.
    MissingMessages { tu: u64, den: u64, so_ban_tin_mat: u64 },
    /// Đã biết có khe rồi, đang chờ dữ liệu phát lại. Bản tin mới vẫn được
    /// đệm lại nhưng KHÔNG xin phát lại nữa.
    AwaitingRecovery,
}

pub struct GapDetector {
    pub expectation: u64,
    /// Bản tin tới sớm được giữ lại, xử lý sau khi khe được lấp.
    count_lai: BTreeMap<u64, FieldPacket>,
    /// Khe đang chờ lấp: (bản tin đầu thiếu, bản tin cuối thiếu).
    /// Có giá trị nghĩa là ta đang ở CHẾ ĐỘ KHÔI PHỤC.
    pending_gap: Option<(u64, u64)>,
    pub slot_count: u64,
    pub num_duplicate_loop: u64,
    pub tong_ban_tin_mat: u64,
}

impl GapDetector {
    pub fn new(start: u64) -> Self {
        GapDetector { expectation: start, count_lai: BTreeMap::new(), pending_gap: None,
                        slot_count: 0, num_duplicate_loop: 0, tong_ban_tin_mat: 0 }
    }

    pub fn dang_recovery(&self) -> bool { self.pending_gap.is_some() }

    pub fn nhan(&mut self, g: FieldPacket) -> KetQuaNhan {
        let stt = g.nonce;
        if stt < self.expectation {
            self.num_duplicate_loop += 1;
            return KetQuaNhan::Duplicate;
        }
        if stt > self.expectation {
            self.count_lai.insert(stt, g); // luôn giữ lại, đừng bao giờ vứt
            // Đã biết có khe rồi thì chỉ đệm tiếp. Nếu báo lại mỗi bản tin,
            // ta sẽ gửi hàng nghìn yêu cầu phát lại cho CÙNG một khe và tự
            // làm sập luồng khôi phục của sàn — lỗi vận hành có thật.
            if let Some((_, den)) = &mut self.pending_gap {
                if stt > *den + 1 { *den = stt - 1; }
                return KetQuaNhan::AwaitingRecovery;
            }
            let (tu, den) = (self.expectation, stt - 1);
            self.pending_gap = Some((tu, den));
            self.slot_count += 1;
            self.tong_ban_tin_mat += den - tu + 1;
            return KetQuaNhan::MissingMessages { tu, den, so_ban_tin_mat: den - tu + 1 };
        }
        self.expectation += 1;
        self.count_lai.insert(stt, g);
        KetQuaNhan::InOrder
    }

    /// Rút các bản tin liền mạch đã sẵn sàng xử lý, theo đúng thứ tự.
    pub fn drain(&mut self) -> Vec<FieldPacket> {
        let mut ra = Vec::new();
        let mut mong = match self.count_lai.keys().next() { Some(&k) => k, None => return ra };
        while let Some(g) = self.count_lai.remove(&mong) {
            ra.push(g);
            mong += 1;
        }
        ra
    }

    /// Lấp khe bằng dữ liệu phát lại từ luồng khôi phục. Khi mọi bản tin
    /// thiếu đã về đủ, ta rời chế độ khôi phục và chạy bình thường trở lại.
    pub fn slot_loop(&mut self, cac_goi: Vec<FieldPacket>) {
        for g in cac_goi {
            let stt = g.nonce;
            self.count_lai.insert(stt, g);
        }
        // Đẩy kỳ vọng qua toàn bộ phần đã liền mạch
        while self.count_lai.contains_key(&self.expectation) { self.expectation += 1; }
        if let Some((_, den)) = self.pending_gap {
            if self.expectation > den { self.pending_gap = None; }
        }
    }

    pub fn num_dang_count(&self) -> usize { self.count_lai.len() }
}

// ============================================================================
// 3. SỔ LỆNH L2 — tổng hợp theo MỨC GIÁ
// ============================================================================
// L2 là thứ 95% chiến lược thật sự cần: mỗi mức giá còn bao nhiêu khối lượng.
// Nhẹ hơn L3 rất nhiều, và cập nhật fast hơn.

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct PriceLevel { pub price: Price, pub quantity: u64, pub so_lenh: u32 }

#[derive(Debug, Default)]
pub struct L2Book {
    /// Bên bid lưu khoá ÂM để `BTreeMap` trả giá high nhất trước.
    buy: BTreeMap<Price, (u64, u32)>,
    ban: BTreeMap<Price, (u64, u32)>,
}

impl L2Book {
    pub fn new() -> Self { L2Book::default() }

    pub fn them(&mut self, side: Side, price: Price, kl: Quantity) {
        let (ban_do, key) = match side {
            Side::Buy => (&mut self.buy, -price),
            Side::Sell => (&mut self.ban, price),
        };
        let e = ban_do.entry(key).or_insert((0, 0));
        e.0 += kl as u64;
        e.1 += 1;
    }

    /// Trả `true` nếu mức giá bị xoá hẳn khỏi sổ.
    pub fn bot(&mut self, side: Side, price: Price, kl: Quantity, bot_mot_lenh: bool) -> bool {
        let (ban_do, key) = match side {
            Side::Buy => (&mut self.buy, -price),
            Side::Sell => (&mut self.ban, price),
        };
        if let Some(e) = ban_do.get_mut(&key) {
            e.0 = e.0.saturating_sub(kl as u64);
            if bot_mot_lenh { e.1 = e.1.saturating_sub(1); }
            // Mức giá hết khối lượng phải BIẾN MẤT, không được để lại mức rỗng —
            // nếu không, "giá tốt nhất" sẽ trỏ vào chỗ không có gì.
            if e.0 == 0 { ban_do.remove(&key); return true; }
        }
        false
    }

    pub fn best_bid(&self) -> Option<Price> { self.buy.keys().next().map(|k| -k) }
    pub fn best_ask(&self) -> Option<Price> { self.ban.keys().next().copied() }
    pub fn spread(&self) -> Option<Price> {
        Some(self.best_ask()? - self.best_bid()?)
    }
    pub fn num_level(&self, side: Side) -> usize {
        match side { Side::Buy => self.buy.len(), Side::Sell => self.ban.len() }
    }
    pub fn qty_at(&self, side: Side, price: Price) -> u64 {
        let (bd, k) = match side {
            Side::Buy => (&self.buy, -price), Side::Sell => (&self.ban, price) };
        bd.get(&k).map_or(0, |e| e.0)
    }

    /// `n` mức giá tốt nhất mỗi bên — đúng thứ mà giao diện và chiến lược cần.
    pub fn peak_num(&self, n: usize) -> (Vec<PriceLevel>, Vec<PriceLevel>) {
        let m = self.buy.iter().take(n)
            .map(|(k, v)| PriceLevel { price: -k, quantity: v.0, so_lenh: v.1 }).collect();
        let b = self.ban.iter().take(n)
            .map(|(k, v)| PriceLevel { price: *k, quantity: v.0, so_lenh: v.1 }).collect();
        (m, b)
    }

    /// Giá bình quân gia quyền theo khối lượng đối ứng — ước lượng "giá trị
    /// thật" tốt hơn giá giữa, vì nó tính cả độ mất cân bằng cung cầu.
    pub fn price_can_table(&self) -> Option<f64> {
        let (m, b) = self.peak_num(1);
        let (m, b) = (m.first()?, b.first()?);
        let tong = (m.quantity + b.quantity) as f64;
        if tong == 0.0 { return None; }
        // Bên nào NHIỀU khối lượng hơn thì giá cân bằng lệch về phía bên kia
        Some((m.price as f64 * b.quantity as f64 + b.price as f64 * m.quantity as f64) / tong)
    }

    // ---- Kiểm tra chất lượng dữ liệu ----

    /// Sổ "khoá" (locked): giá bid = giá bán. Hiếm nhưng hợp lệ ở vài thị trường.
    pub fn is_key(&self) -> bool { self.spread() == Some(0) }

    /// Sổ "chéo" (crossed): giá bid > giá bán. LUÔN LUÔN là dấu hiệu dữ liệu
    /// hỏng hoặc mất bản tin — phải dừng giao dịch ngay, đừng cố khai thác.
    pub fn is_crossed(&self) -> bool { self.spread().is_some_and(|c| c < 0) }

    pub fn is_healthy(&self) -> bool { !self.is_crossed() }
}

// ============================================================================
// 4. SỔ LỆNH L3 — theo TỪNG LỆNH
// ============================================================================
// L3 giữ danh tính từng lệnh. Nặng hơn nhiều, nhưng là thứ duy nhất trả lời
// được "lệnh của TÔI đang đứng thứ mấy trong hàng?" — câu hỏi sống còn với
// chiến lược tạo lập thị trường.

#[derive(Debug, Clone, PartialEq)]
pub struct L3Order { pub id: OrderId, pub side: Side, pub price: Price, pub remaining: Quantity }

/// `Chieu` không cài `Ord`, nên dùng bản có thứ tự làm khoá bản đồ.
#[derive(Debug, Clone, Copy, PartialEq, Eq, PartialOrd, Ord)]
pub enum Chieu2 { Buy, Sell }

impl From<Side> for Chieu2 {
    fn from(c: Side) -> Self { match c { Side::Buy => Chieu2::Buy, Side::Sell => Chieu2::Sell } }
}

#[derive(Debug, Default)]
pub struct L3Book {
    pub order: HashMap<OrderId, L3Order>,
    /// Thứ tự tới của từng mức giá — nền của ưu tiên thời gian.
    queue: BTreeMap<(Chieu2, Price), Vec<OrderId>>,
    pub l2: L2Book,
}

impl L3Book {
    pub fn new() -> Self { L3Book::default() }

    pub fn apply(&mut self, bt: &BanTin) {
        match bt {
            BanTin::AddOrder { id, side, price, quantity, .. } => {
                self.order.insert(*id,
                    L3Order { id: *id, side: *side, price: *price, remaining: *quantity });
                self.queue.entry(((*side).into(), *price)).or_default().push(*id);
                self.l2.them(*side, *price, *quantity);
            }
            BanTin::CancelOrder { id, cancel_quantity } => {
                if let Some(l) = self.order.get_mut(id) {
                    let actually_cancelled = (*cancel_quantity).min(l.remaining);
                    l.remaining -= actually_cancelled;
                    let (c, g, het) = (l.side, l.price, l.remaining == 0);
                    self.l2.bot(c, g, actually_cancelled, het);
                    if het { self.remove_from_queue(*id, c, g); self.order.remove(id); }
                }
            }
            BanTin::Fill { id, quantity, .. } => {
                if let Some(l) = self.order.get_mut(id) {
                    let thuc = (*quantity).min(l.remaining);
                    l.remaining -= thuc;
                    let (c, g, het) = (l.side, l.price, l.remaining == 0);
                    self.l2.bot(c, g, thuc, het);
                    if het { self.remove_from_queue(*id, c, g); self.order.remove(id); }
                }
            }
            BanTin::Replaced { old_id, ma_moi, price, quantity } => {
                // Thay thế = huỷ hẳn rồi thêm mới. Lệnh MẤT ưu tiên thời gian,
                // xuống cuối hàng — đây là lý do sửa lệnh rất đắt trong HFT.
                if let Some(l) = self.order.remove(old_id) {
                    self.l2.bot(l.side, l.price, l.remaining, true);
                    self.remove_from_queue(*old_id, l.side, l.price);
                    self.order.insert(*ma_moi,
                        L3Order { id: *ma_moi, side: l.side, price: *price, remaining: *quantity });
                    self.queue.entry((l.side.into(), *price)).or_default().push(*ma_moi);
                    self.l2.them(l.side, *price, *quantity);
                }
            }
        }
    }

    fn remove_from_queue(&mut self, id: OrderId, c: Side, g: Price) {
        if let Some(h) = self.queue.get_mut(&(c.into(), g)) {
            h.retain(|&x| x != id);
            if h.is_empty() { self.queue.remove(&(c.into(), g)); }
        }
    }

    /// Lệnh này đứng thứ mấy trong hàng ở mức giá của nó? (0 = đầu hàng)
    /// Câu trả lời quyết định xác suất được khớp.
    pub fn queue_position(&self, id: OrderId) -> Option<usize> {
        let l = self.order.get(&id)?;
        self.queue.get(&(l.side.into(), l.price))?.iter().position(|&x| x == id)
    }

    /// Khối lượng đứng TRƯỚC lệnh này — phải khớp hết chỗ đó thì mới tới lượt ta.
    pub fn queue_ahead(&self, id: OrderId) -> Option<u64> {
        let l = self.order.get(&id)?;
        let h = self.queue.get(&(l.side.into(), l.price))?;
        let vt = h.iter().position(|&x| x == id)?;
        Some(h[..vt].iter().filter_map(|m| self.order.get(m)).map(|x| x.remaining as u64).sum())
    }

    pub fn order_book_dang_open(&self) -> usize { self.order.len() }
}

// ============================================================================
// 5. SINH DỮ LIỆU PHIÊN TẤT ĐỊNH
// ============================================================================

pub fn generate_session(so_ban_tin: usize, hat_giong: u64) -> Vec<FieldPacket> {
    let mut s = hat_giong;
    let mut ra = Vec::with_capacity(so_ban_tin);
    let mut order_id: u64 = 1;
    let mut is_open: Vec<(OrderId, Side, Price, Quantity)> = Vec::new();
    let mut t: u64 = 1_000_000_000;

    for stt in 0..so_ban_tin as u64 {
        s = s.wrapping_mul(6364136223846793005).wrapping_add(1442695040888963407);
        let r = (s >> 33) % 100;
        t += 1_000 + (s >> 20) % 50_000;

        // Giữ sổ có ít nhất vài lệnh trước khi bắt đầu huỷ/khớp
        let bt = if is_open.len() < 4 || r < 55 {
            let side = if (s >> 40) % 2 == 0 { Side::Buy } else { Side::Sell };
            // Bên bid đặt dưới 8400, bên bán đặt trên 8400 → sổ không bao giờ chéo
            let lech = ((s >> 44) % 20) as i64;
            let price = match side {
                Side::Buy => 8_400 - 1 - lech,
                Side::Sell => 8_400 + 1 + lech,
            };
            let sl = 100 + ((s >> 48) % 10) as u32 * 100;
            is_open.push((order_id, side, price, sl));
            let bt = BanTin::AddOrder { id: order_id, id_chain: 1, side, price, quantity: sl };
            order_id += 1;
            bt
        } else {
            let i = ((s >> 52) as usize) % is_open.len();
            let (id, _, price, sl) = is_open[i];
            let part = (sl / 2).max(1);
            if r < 80 {
                is_open.remove(i);
                BanTin::CancelOrder { id, cancel_quantity: sl }
            } else {
                is_open[i].3 -= part;
                if is_open[i].3 == 0 { is_open.remove(i); }
                BanTin::Fill { id, quantity: part, price }
            }
        };
        ra.push(FieldPacket { nonce: stt, timestamp_nanos: t, ban_tin: bt });
    }
    ra
}

fn main() {
    println!("═══════════════════════════════════════════════════════════");
    println!("   LUỒNG DỮ LIỆU THỊ TRƯỜNG: NHỊ PHÂN · KHE · SỔ L2/L3     ");
    println!("═══════════════════════════════════════════════════════════");

    println!("\n1. GIAO THỨC NHỊ PHÂN vs JSON");
    let g = FieldPacket {
        nonce: 12345, timestamp_nanos: 1_700_000_000_000_000_000,
        ban_tin: BanTin::AddOrder { id: 999, id_chain: 1, side: Side::Buy,
                                    price: 8_450, quantity: 100 },
    };
    let b = encode(&g);
    let json = r#"{"seq":12345,"ts":1700000000000000000,"type":"add","id":999,"sym":"VNM","side":"B","px":84.50,"qty":100}"#;
    println!("   Nhị phân: {} byte", b.len());
    println!("   JSON    : {} byte → gấp {:.1} lần", json.len(), json.len() as f64 / b.len() as f64);
    println!("   Phân tích ngược ra đúng bản gốc: {}", analyze(&b).unwrap() == g);

    println!("\n2. PHÁT HIỆN KHE SỐ THỨ TỰ");
    let mut pd = GapDetector::new(0);
    let session = generate_session(10, 7);
    for (i, gt) in session.iter().enumerate() {
        if i == 3 || i == 4 { continue; } // giả lập mất 2 gói UDP
        let kq = pd.nhan(gt.clone());
        if kq != KetQuaNhan::InOrder { println!("   stt {} → {:?}", gt.nonce, kq); }
    }
    println!("   Tổng khe: {} · tổng bản tin mất: {} · đang đệm: {}",
             pd.slot_count, pd.tong_ban_tin_mat, pd.num_dang_count());
    pd.slot_loop(vec![session[3].clone(), session[4].clone()]);
    println!("   Đang ở chế độ khôi phục: {}", pd.dang_recovery());
    println!("   Sau khi phát lại → còn khôi phục: {} · rút liền mạch được {} bản tin",
             pd.dang_recovery(), pd.drain().len());

    println!("\n3. DỰNG SỔ L2 TỪ 5000 BẢN TIN");
    let mut so = L3Book::new();
    for g in generate_session(5_000, 42) { so.apply(&g.ban_tin); }
    let (buy, ban) = so.l2.peak_num(5);
    println!("   {} lệnh đang mở · {} mức bid · {} mức bán",
             so.order_book_dang_open(), so.l2.num_level(Side::Buy), so.l2.num_level(Side::Sell));
    println!("   ── 5 MỨC TỐT NHẤT ──");
    for m in ban.iter().rev() {
        println!("        BÁN {:>7.2}  {:>6} ({} lệnh)",
                 m.price as f64 / 100.0, m.quantity, m.so_lenh);
    }
    println!("        ─────────────  chênh lệch {} tick", so.l2.spread().unwrap_or(0));
    for m in &buy {
        println!("        MUA {:>7.2}  {:>6} ({} lệnh)",
                 m.price as f64 / 100.0, m.quantity, m.so_lenh);
    }
    println!("   Giá cân bằng theo khối lượng: {:.2}",
             so.l2.price_can_table().unwrap_or(0.0) / 100.0);

    println!("\n4. KIỂM TRA CHẤT LƯỢNG DỮ LIỆU");
    println!("   Sổ lành mạnh: {} · bị khoá: {} · bị chéo: {}",
             so.l2.is_healthy(), so.l2.is_key(), so.l2.is_crossed());
    let mut hong = L2Book::new();
    hong.them(Side::Buy, 8_500, 100);
    hong.them(Side::Sell, 8_400, 100); // bid CAO hơn bán → vô lý
    println!("   Sổ dựng sai (bid 85.00 > bán 84.00) → bị chéo: {} · lành mạnh: {}",
             hong.is_crossed(), hong.is_healthy());
    println!("   → Gặp sổ chéo phải NGỪNG giao dịch, không được coi là cơ hội.");

    println!("\n5. VỊ TRÍ TRONG HÀNG — câu hỏi sống còn của tạo lập thị trường");
    let mut s3 = L3Book::new();
    for (id, sl) in [(1u64, 500u32), (2, 300), (3, 200)] {
        s3.apply(&BanTin::AddOrder { id, id_chain: 1, side: Side::Buy,
                                       price: 8_400, quantity: sl });
    }
    for id in [1u64, 2, 3] {
        println!("   Lệnh #{} → đứng thứ {} · phải chờ {} đơn vị khớp trước",
                 id, s3.queue_position(id).unwrap(),
                 s3.queue_ahead(id).unwrap());
    }
    s3.apply(&BanTin::Replaced { old_id: 1, ma_moi: 4, price: 8_400, quantity: 500 });
    println!("   Sửa lệnh #1 (thành #4) → giờ đứng thứ {} — MẤT SẠCH ưu tiên thời gian",
             s3.queue_position(4).unwrap());

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
        let all_bt = vec![
            BanTin::AddOrder { id: 1, id_chain: 7, side: Side::Buy, price: 8_450, quantity: 100 },
            BanTin::AddOrder { id: 2, id_chain: 7, side: Side::Sell, price: -50, quantity: 1 },
            BanTin::CancelOrder { id: 3, cancel_quantity: 250 },
            BanTin::Fill { id: 4, quantity: 75, price: 8_400 },
            BanTin::Replaced { old_id: 5, ma_moi: 6, price: 8_390, quantity: 999 },
        ];
        for bt in all_bt {
            let g = FieldPacket { nonce: 42, timestamp_nanos: 1_700_000_000_000_000_000,
                                   ban_tin: bt };
            assert_eq!(analyze(&encode(&g)), Ok(g.clone()), "vòng mã hoá phải khép kín");
        }
    }

    #[test]
    fn parser_rejects_short_packets() {
        assert_eq!(analyze(&[]), Err(ErrorAnalyze::TooShort { can: 17, co: 0 }));
        assert_eq!(analyze(&[b'A'; 10]), Err(ErrorAnalyze::TooShort { can: 17, co: 10 }));
        // Đủ phần đầu chung nhưng thiếu thân bản tin 'A'
        let mut b = vec![b'A']; b.extend_from_slice(&[0u8; 20]);
        assert!(matches!(analyze(&b), Err(ErrorAnalyze::TooShort { .. })));
    }

    #[test]
    fn parser_rejects_unknown_message_type() {
        let mut b = vec![b'Z']; b.extend_from_slice(&[0u8; 60]);
        assert_eq!(analyze(&b), Err(ErrorAnalyze::UnknownMessageKind(b'Z')));
    }

    #[test]
    fn parser_rejects_unknown_side_code() {
        let g = FieldPacket { nonce: 1, timestamp_nanos: 1,
            ban_tin: BanTin::AddOrder { id: 1, id_chain: 1, side: Side::Buy,
                                        price: 100, quantity: 1 } };
        let mut b = encode(&g);
        b[29] = b'?'; // phá byte chiều
        assert_eq!(analyze(&b), Err(ErrorAnalyze::UnknownSide(b'?')));
    }

    #[test]
    fn binary_is_far_smaller_than_json() {
        let g = FieldPacket { nonce: 12345, timestamp_nanos: 1_700_000_000_000_000_000,
            ban_tin: BanTin::AddOrder { id: 999, id_chain: 1, side: Side::Buy,
                                        price: 8_450, quantity: 100 } };
        assert_eq!(encode(&g).len(), 42, "bản tin thêm lệnh dài đúng 42 byte cố định");
        assert!(encode(&g).len() * 2 < 105, "nhị phân phải gọn hơn JSON ít nhất 2 lần");
    }

    #[test]
    fn uses_big_endian_byte_order() {
        // Giao thức mạng LUÔN dùng big-endian. Nhầm sang little-endian thì
        // số nhỏ vẫn "chạy" nhưng giá trị hoàn toàn sai.
        let g = FieldPacket { nonce: 0x0102030405060708, timestamp_nanos: 0,
            ban_tin: BanTin::CancelOrder { id: 1, cancel_quantity: 1 } };
        let b = encode(&g);
        assert_eq!(&b[1..9], &[1, 2, 3, 4, 5, 6, 7, 8], "byte high đứng TRƯỚC");
    }

    // ---------- Phát hiện khe ----------
    #[test]
    fn a_contiguous_stream_reports_no_gap() {
        let mut p = GapDetector::new(0);
        for g in generate_session(100, 1) {
            assert_eq!(p.nhan(g), KetQuaNhan::InOrder);
        }
        assert_eq!(p.slot_count, 0);
        assert_eq!(p.expectation, 100);
    }

    #[test]
    fn detects_the_gap_and_counts_lost_messages() {
        let session = generate_session(10, 2);
        let mut p = GapDetector::new(0);
        for (i, g) in session.iter().enumerate() {
            if (3..=5).contains(&i) { continue; } // mất gói 3,4,5
            let kq = p.nhan(g.clone());
            if i == 6 {
                assert_eq!(kq, KetQuaNhan::MissingMessages { tu: 3, den: 5, so_ban_tin_mat: 3 });
            } else if i > 6 {
                assert_eq!(kq, KetQuaNhan::AwaitingRecovery,
                           "các bản tin sau chỉ được đệm, không xin phát lại nữa");
            }
        }
        assert_eq!(p.slot_count, 1);
        assert_eq!(p.tong_ban_tin_mat, 3);
    }

    #[test]
    fn requests_retransmission_only_once_per_gap() {
        // Nếu báo khe ở mọi bản tin sau đó, ta sẽ gửi hàng nghìn yêu cầu phát
        // lại cho cùng một khe và tự làm sập luồng khôi phục của sàn.
        let session = generate_session(20, 8);
        let mut p = GapDetector::new(0);
        let mut gap_count = 0;
        for (i, g) in session.iter().enumerate() {
            if (3..=5).contains(&i) { continue; }
            if matches!(p.nhan(g.clone()), KetQuaNhan::MissingMessages { .. }) {
                gap_count += 1;
            }
        }
        assert_eq!(gap_count, 1, "một khe chỉ được xin phát lại đúng một lần");
        assert_eq!(p.slot_count, 1);
        assert_eq!(p.tong_ban_tin_mat, 3);
        assert!(p.dang_recovery(), "vẫn đang chờ dữ liệu phát lại");
    }

    #[test]
    fn leaves_recovery_once_the_gap_is_filled() {
        let session = generate_session(20, 8);
        let mut p = GapDetector::new(0);
        for (i, g) in session.iter().enumerate() {
            if (3..=5).contains(&i) { continue; }
            p.nhan(g.clone());
        }
        assert!(p.dang_recovery());
        p.slot_loop(vec![session[3].clone(), session[4].clone()]);
        assert!(p.dang_recovery(), "còn thiếu bản tin 5 thì vẫn đang khôi phục");
        p.slot_loop(vec![session[5].clone()]);
        assert!(!p.dang_recovery(), "đủ rồi thì phải trở lại bình thường");
        assert_eq!(p.drain().len(), 20);
    }

    #[test]
    fn sell_info_duplicate_loop_is_unit_qua() {
        // Sàn thường phát hai luồng giống hệt (A và B) để chống mất gói.
        // Bản sao đến sau phải bị loại, không được xử lý hai lần.
        let session = generate_session(5, 3);
        let mut p = GapDetector::new(0);
        for g in &session { p.nhan(g.clone()); }
        for g in &session {
            assert_eq!(p.nhan(g.clone()), KetQuaNhan::Duplicate);
        }
        assert_eq!(p.num_duplicate_loop, 5);
        assert_eq!(p.expectation, 5, "trùng lặp không được đẩy kỳ vọng đi");
    }

    #[test]
    fn early_messages_are_buffered_not_dropped() {
        let session = generate_session(10, 4);
        let mut p = GapDetector::new(0);
        p.nhan(session[0].clone());
        p.nhan(session[5].clone()); // nhảy cóc
        assert_eq!(p.num_dang_count(), 2, "cả hai đều phải được giữ lại");
        assert_eq!(p.drain().len(), 1, "chỉ rút được phần liền mạch từ đầu");
    }

    #[test]
    fn filling_the_gap_drains_everything() {
        let session = generate_session(10, 5);
        let mut p = GapDetector::new(0);
        for (i, g) in session.iter().enumerate() {
            if i == 3 || i == 4 { continue; }
            p.nhan(g.clone());
        }
        p.slot_loop(vec![session[3].clone(), session[4].clone()]);
        let ra = p.drain();
        assert_eq!(ra.len(), 10, "sau khi lấp khe phải rút được đủ 10 bản tin");
        for (i, g) in ra.iter().enumerate() {
            assert_eq!(g.nonce, i as u64, "và đúng thứ tự");
        }
    }

    // ---------- Sổ L2 ----------
    #[test]
    fn l2_reports_best_on_both_sides() {
        let mut s = L2Book::new();
        s.them(Side::Buy, 8_390, 100);
        s.them(Side::Buy, 8_400, 200); // high hơn = tốt hơn cho bên bid
        s.them(Side::Sell, 8_420, 150);
        s.them(Side::Sell, 8_410, 50);  // thấp hơn = tốt hơn cho bên bán
        assert_eq!(s.best_bid(), Some(8_400));
        assert_eq!(s.best_ask(), Some(8_410));
        assert_eq!(s.spread(), Some(10));
    }

    #[test]
    fn l2_aggregates_size_and_order_count_per_level() {
        let mut s = L2Book::new();
        for _ in 0..3 { s.them(Side::Buy, 8_400, 100); }
        let (m, _) = s.peak_num(1);
        assert_eq!(m[0].quantity, 300);
        assert_eq!(m[0].so_lenh, 3);
    }

    #[test]
    fn an_emptied_level_disappears_from_the_book() {
        // Nếu để lại mức rỗng, `best_bid` sẽ trỏ vào chỗ không có gì —
        // và chiến lược sẽ gửi lệnh vào hư không.
        let mut s = L2Book::new();
        s.them(Side::Buy, 8_400, 100);
        s.them(Side::Buy, 8_390, 50);
        assert!(s.bot(Side::Buy, 8_400, 100, true), "phải báo mức giá đã bị xoá");
        assert_eq!(s.best_bid(), Some(8_390), "đỉnh sổ phải tụt xuống mức kế");
        assert_eq!(s.num_level(Side::Buy), 1);
    }

    #[test]
    fn over_reducing_never_makes_size_negative() {
        let mut s = L2Book::new();
        s.them(Side::Buy, 8_400, 100);
        assert!(s.bot(Side::Buy, 8_400, 99_999, true), "trừ quá cũng chỉ về 0");
        assert_eq!(s.qty_at(Side::Buy, 8_400), 0);
        assert_eq!(s.num_level(Side::Buy), 0);
    }

    #[test]
    fn empty_book_neither_panics_nor_reports_crossed() {
        let s = L2Book::new();
        assert_eq!(s.best_bid(), None);
        assert_eq!(s.spread(), None);
        assert!(!s.is_crossed() && !s.is_key() && s.is_healthy());
        assert_eq!(s.price_can_table(), None);
    }

    #[test]
    fn detects_crossed_and_locked_books() {
        let mut cheo = L2Book::new();
        cheo.them(Side::Buy, 8_500, 100);
        cheo.them(Side::Sell, 8_400, 100);
        assert!(cheo.is_crossed(), "bid 85.00 > bán 84.00 là dữ liệu hỏng");
        assert!(!cheo.is_healthy());

        let mut key = L2Book::new();
        key.them(Side::Buy, 8_400, 100);
        key.them(Side::Sell, 8_400, 100);
        assert!(key.is_key() && !key.is_crossed(),
                "sổ khoá là hiếm nhưng hợp lệ, khác hẳn sổ chéo");
        assert!(key.is_healthy());
    }

    #[test]
    fn fair_price_leans_toward_the_thin_side() {
        // Nhiều người muốn bid hơn bán → áp lực đẩy giá lên → giá cân bằng
        // phải gần giá BÁN hơn.
        let mut s = L2Book::new();
        s.them(Side::Buy, 8_400, 900);
        s.them(Side::Sell, 8_410, 100);
        let cb = s.price_can_table().unwrap();
        assert!(cb > 8_405.0, "áp lực bid mạnh → giá cân bằng {} phải lệch lên trên", cb);
        assert!(cb < 8_410.0);
    }

    #[test]
    fn top_of_book_respects_priority_order() {
        let mut s = L2Book::new();
        for g in [8_380, 8_390, 8_400] { s.them(Side::Buy, g, 100); }
        for g in [8_430, 8_420, 8_410] { s.them(Side::Sell, g, 100); }
        let (m, b) = s.peak_num(3);
        assert_eq!(m.iter().map(|x| x.price).collect::<Vec<_>>(), vec![8_400, 8_390, 8_380],
                   "bên bid: giá high xuống thấp");
        assert_eq!(b.iter().map(|x| x.price).collect::<Vec<_>>(), vec![8_410, 8_420, 8_430],
                   "bên bán: giá thấp lên high");
    }

    // ---------- Sổ L3 ----------
    #[test]
    fn l3_and_l2_stay_consistent_over_a_long_session() {
        // BẤT BIẾN QUAN TRỌNG NHẤT của chương: L2 phải luôn là bản tổng hợp
        // đúng của L3. Lệch nhau nghĩa là có bản tin bị xử lý sai.
        let mut s = L3Book::new();
        for g in generate_session(3_000, 99) {
            s.apply(&g.ban_tin);
            assert!(s.l2.is_healthy(), "sổ không bao giờ được chéo khi dữ liệu sạch");
        }
        // Dựng lại L2 từ L3 rồi so
        let mut check = L2Book::new();
        for l in s.order.values() { check.them(l.side, l.price, l.remaining); }
        assert_eq!(check.best_bid(), s.l2.best_bid());
        assert_eq!(check.best_ask(), s.l2.best_ask());
        assert_eq!(check.num_level(Side::Buy), s.l2.num_level(Side::Buy));
        assert_eq!(check.num_level(Side::Sell), s.l2.num_level(Side::Sell));
        for l in s.order.values() {
            assert_eq!(check.qty_at(l.side, l.price),
                       s.l2.qty_at(l.side, l.price));
        }
    }

    #[test]
    fn l3_preserves_time_priority() {
        let mut s = L3Book::new();
        for (id, sl) in [(1u64, 500u32), (2, 300), (3, 200)] {
            s.apply(&BanTin::AddOrder { id, id_chain: 1, side: Side::Buy,
                                          price: 8_400, quantity: sl });
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
        for (id, sl) in [(1u64, 500u32), (2, 300)] {
            s.apply(&BanTin::AddOrder { id, id_chain: 1, side: Side::Buy,
                                          price: 8_400, quantity: sl });
        }
        s.apply(&BanTin::Fill { id: 1, quantity: 500, price: 8_400 });
        assert_eq!(s.queue_position(2), Some(0), "lệnh #2 lên đầu hàng");
        assert_eq!(s.queue_ahead(2), Some(0));
        assert_eq!(s.order_book_dang_open(), 1);
    }

    #[test]
    fn a_partial_fill_keeps_queue_position() {
        let mut s = L3Book::new();
        for (id, sl) in [(1u64, 500u32), (2, 300)] {
            s.apply(&BanTin::AddOrder { id, id_chain: 1, side: Side::Buy,
                                          price: 8_400, quantity: sl });
        }
        s.apply(&BanTin::Fill { id: 1, quantity: 200, price: 8_400 });
        assert_eq!(s.queue_position(1), Some(0), "khớp một phần KHÔNG mất chỗ");
        assert_eq!(s.queue_ahead(2), Some(300), "chỉ còn 300 đứng trước");
        assert_eq!(s.l2.qty_at(Side::Buy, 8_400), 600);
    }

    #[test]
    fn replacing_an_order_forfeits_time_priority() {
        // Bài học đắt tiền: sửa giá/khối lượng một lệnh = xuống cuối hàng.
        // Đó là lý do chiến lược tốt cân nhắc rất kỹ trước khi sửa lệnh.
        let mut s = L3Book::new();
        for (id, sl) in [(1u64, 500u32), (2, 300), (3, 200)] {
            s.apply(&BanTin::AddOrder { id, id_chain: 1, side: Side::Buy,
                                          price: 8_400, quantity: sl });
        }
        assert_eq!(s.queue_position(1), Some(0));
        s.apply(&BanTin::Replaced { old_id: 1, ma_moi: 4, price: 8_400, quantity: 500 });
        assert_eq!(s.queue_position(1), None, "mã cũ biến mất");
        assert_eq!(s.queue_position(4), Some(2), "mã mới xuống CUỐI hàng");
        assert_eq!(s.queue_ahead(4), Some(500));
    }

    #[test]
    fn cancelling_an_unknown_order_leaves_the_book_intact() {
        let mut s = L3Book::new();
        s.apply(&BanTin::AddOrder { id: 1, id_chain: 1, side: Side::Buy,
                                      price: 8_400, quantity: 100 });
        s.apply(&BanTin::CancelOrder { id: 999, cancel_quantity: 50 }); // mã lạ
        assert_eq!(s.order_book_dang_open(), 1);
        assert_eq!(s.l2.qty_at(Side::Buy, 8_400), 100, "sổ phải nguyên vẹn");
    }

    #[test]
    fn cancelling_more_than_remaining_is_safe() {
        let mut s = L3Book::new();
        s.apply(&BanTin::AddOrder { id: 1, id_chain: 1, side: Side::Buy,
                                      price: 8_400, quantity: 100 });
        s.apply(&BanTin::CancelOrder { id: 1, cancel_quantity: 99_999 });
        assert_eq!(s.order_book_dang_open(), 0);
        assert_eq!(s.l2.num_level(Side::Buy), 0);
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
        for (i, g) in p.iter().enumerate() { assert_eq!(g.nonce, i as u64); }
    }

    #[test]
    fn session_timestamps_are_monotonic() {
        let p = generate_session(500, 3);
        for w in p.windows(2) {
            assert!(w[1].timestamp_nanos > w[0].timestamp_nanos,
                    "dấu thời gian phải tăng — nền tảng cho phát lại ở Chương 76");
        }
    }

    #[test]
    fn every_generated_message_round_trips() {
        for g in generate_session(500, 11) {
            assert_eq!(analyze(&encode(&g)), Ok(g.clone()));
        }
    }
}
