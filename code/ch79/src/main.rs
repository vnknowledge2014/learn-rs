#![allow(dead_code)]
//! Chương 79 — FPGA cho giao dịch: bộ xử lý luồng dữ liệu bằng phần cứng, sổ
//! lệnh trên thanh ghi, đường ống kiểm tra rủi ro, và ngân sách tick-to-trade
//! tính bằng CHU KỲ thay vì micro-giây.
//!
//! Nối hai mạch của giáo trình: Chương 67 (thiết kế phần cứng số) gặp Chương
//! 74–77 (hệ sinh thái HFT). Đây chính là chỗ `hardcaml` của Jane Street và
//! `rhdl` trong hệ sinh thái Rust nhắm tới: mô tả phần cứng bằng một ngôn ngữ
//! có hệ thống kiểu mạnh, mô phỏng ngay trong bộ kiểm thử, rồi mới sinh Verilog.

// ============================================================================
// 1. VÌ SAO GIAO DỊCH DÙNG FPGA
// ============================================================================
// Phần mềm giỏi nhất đạt tick-to-trade khoảng 1–5 µs, nhưng có ĐUÔI DÀI: hệ
// điều hành xen vào, trượt cache, một cú dừng bất chợt. FPGA đạt 20–100 ns và
// quan trọng hơn — độ trễ gần như KHÔNG DAO ĐỘNG. Trong đấu giá theo thứ tự
// tới, người ổn định thắng người fast-nhưng-thất-thường.

/// Chu kỳ xung nhịp của FPGA giao dịch điển hình: 250 MHz → 4 ns mỗi chu kỳ.
pub const NS_MOI_CHU_KY: f64 = 4.0;

pub fn cycles_to_ns(period: u32) -> f64 { period as f64 * NS_MOI_CHU_KY }

// ============================================================================
// 2. TÁCH TRƯỜNG SONG SONG — điều phần mềm không làm được
// ============================================================================
// Phần mềm đọc từng trường một: đọc offset 0, rồi 8, rồi 16… Mỗi lần là một
// lệnh CPU. Phần cứng nối THẲNG dây từ mọi vị trí byte tới mọi thanh ghi đích,
// nên TẤT CẢ trường được tách trong CÙNG MỘT chu kỳ.

#[derive(Debug, Clone, Copy, PartialEq, Eq, Default)]
pub struct PacketField {
    pub kind: u8,
    pub id_chain: u32,
    pub price: i64,
    pub quantity: u32,
    pub is_valid: bool,
}

/// Bố cục gói tin cố định 20 byte:
/// `[loại 1B][mã ck 4B][giá 8B][số lượng 4B][tổng kiểm tra 3B]`
pub const DAI_GOI: usize = 20;

#[derive(Debug, Default)]
pub struct FieldExtractor {
    pub so_goi_da_tach: u64,
    pub so_goi_hong: u64,
}

impl FieldExtractor {
    /// Tách toàn bộ trường trong ĐÚNG MỘT chu kỳ. Trong Rust ta viết tuần tự,
    /// nhưng khi tổng hợp ra mạch thì các phép gán này là dây nối song song —
    /// không có "trước" và "sau", tất cả xảy ra cùng lúc.
    pub fn tach(&mut self, goi: &[u8]) -> Option<PacketField> {
        if goi.len() < DAI_GOI { self.so_goi_hong += 1; return None; }

        let t = PacketField {
            kind: goi[0],
            id_chain: u32::from_be_bytes([goi[1], goi[2], goi[3], goi[4]]),
            price: i64::from_be_bytes([goi[5], goi[6], goi[7], goi[8],
                                     goi[9], goi[10], goi[11], goi[12]]),
            quantity: u32::from_be_bytes([goi[13], goi[14], goi[15], goi[16]]),
            is_valid: true,
        };

        // Tổng kiểm tra cũng tính SONG SONG bằng cây XOR — độ sâu log(n)
        // thay vì n bước cộng dồn như phần mềm.
        let account = xor_tree(&goi[..17]) & 0x00FF_FFFF;
        let expected = u32::from_be_bytes([0, goi[17], goi[18], goi[19]]);
        if account != expected {
            self.so_goi_hong += 1;
            return Some(PacketField { is_valid: false, ..t });
        }
        self.so_goi_da_tach += 1;
        Some(t)
    }

    /// Số chu kỳ để tách một gói. Phần cứng: LUÔN LUÔN 1.
    pub fn period_split(&self) -> u32 { 1 }
}

/// Cây XOR: gộp từng cặp, độ sâu ⌈log₂(n)⌉ tầng cổng thay vì n tầng.
/// Đây là mẫu "rút gọn song song" — nền của mọi phép gộp trên phần cứng và GPU.
pub fn xor_tree(data: &[u8]) -> u32 {
    let mut tang: Vec<u32> = data.iter().map(|&b| b as u32).collect();
    while tang.len() > 1 {
        let mut above = Vec::with_capacity(tang.len().div_ceil(2));
        for cap in tang.chunks(2) {
            above.push(cap[0] ^ cap.get(1).copied().unwrap_or(0));
        }
        tang = above;
    }
    tang.first().copied().unwrap_or(0)
}

pub fn xor_tree_depth(n: usize) -> u32 {
    if n <= 1 { return 0; }
    (n as f64).log2().ceil() as u32
}

/// Cách phần mềm làm: cộng dồn tuần tự, n bước phụ thuộc nhau.
pub fn xor_tuan_tu(data: &[u8]) -> u32 {
    data.iter().fold(0u32, |a, &b| a ^ b as u32)
}

// ============================================================================
// 3. SỔ LỆNH TRÊN THANH GHI
// ============================================================================
// Phần mềm dùng BTreeMap: O(log n) nhưng có nhảy con trỏ và trượt cache.
// Phần cứng giữ N mức giá tốt nhất trong THANH GHI và so sánh TẤT CẢ cùng lúc
// bằng một mạng so sánh. Tìm giá tốt nhất tốn đúng 1 chu kỳ, bất kể N.

pub const SO_MUC_PHAN_CUNG: usize = 8;

#[derive(Debug, Clone, Copy, PartialEq, Eq, Default)]
pub struct HwPriceLevel { pub price: i64, pub quantity: u32 }

/// Sổ lệnh "nông nhưng fast": chỉ giữ 8 mức tốt nhất mỗi bên. Đủ cho gần
/// như mọi chiến lược, và vừa trọn trong thanh ghi FPGA.
#[derive(Debug, Clone, Copy)]
pub struct OrderBookHardware {
    pub buy: [HwPriceLevel; SO_MUC_PHAN_CUNG],
    pub ban: [HwPriceLevel; SO_MUC_PHAN_CUNG],
}

impl Default for OrderBookHardware {
    fn default() -> Self {
        OrderBookHardware { buy: [HwPriceLevel::default(); SO_MUC_PHAN_CUNG],
                         ban: [HwPriceLevel::default(); SO_MUC_PHAN_CUNG] }
    }
}

impl OrderBookHardware {
    /// Bộ mã hoá ưu tiên: tìm mức bid có giá CAO nhất. Trên phần cứng đây là
    /// một cây so sánh độ sâu log₂(8) = 3 tầng, chạy trong MỘT chu kỳ.
    /// Phần mềm phải duyệt 8 phần tử — 8 lần so sánh phụ thuộc nhau.
    pub fn best_bid(&self) -> Option<HwPriceLevel> {
        self.buy.iter().filter(|m| m.quantity > 0).max_by_key(|m| m.price).copied()
    }
    pub fn best_ask(&self) -> Option<HwPriceLevel> {
        self.ban.iter().filter(|m| m.quantity > 0).min_by_key(|m| m.price).copied()
    }
    pub fn spread(&self) -> Option<i64> {
        Some(self.best_ask()?.price - self.best_bid()?.price)
    }

    /// Cập nhật một mức giá. Mọi ô so sánh SONG SONG với giá đầu vào, nên
    /// dù có 8 hay 64 mức thì vẫn tốn đúng một chu kỳ.
    pub fn update(&mut self, la_mua: bool, price: i64, quantity: u32) {
        let o = if la_mua { &mut self.buy } else { &mut self.ban };
        // Đã có mức giá này chưa?
        if let Some(m) = o.iter_mut().find(|m| m.price == price && m.quantity > 0) {
            m.quantity = quantity;
            if quantity == 0 { m.price = 0; }
            return;
        }
        if quantity == 0 { return; }
        // Ô trống?
        if let Some(m) = o.iter_mut().find(|m| m.quantity == 0) {
            *m = HwPriceLevel { price, quantity };
            return;
        }
        // Đầy: thay mức TỆ NHẤT nếu mức mới tốt hơn
        let te_nhat = if la_mua {
            o.iter_mut().min_by_key(|m| m.price).unwrap()
        } else {
            o.iter_mut().max_by_key(|m| m.price).unwrap()
        };
        let tot_hon = if la_mua { price > te_nhat.price } else { price < te_nhat.price };
        if tot_hon { *te_nhat = HwPriceLevel { price, quantity }; }
    }

    /// Độ sâu cây so sánh — quyết định tần số tối đa của mạch.
    pub fn comparator_depth() -> u32 { xor_tree_depth(SO_MUC_PHAN_CUNG) }

    pub fn num_level_dang_use(&self, la_mua: bool) -> usize {
        let o = if la_mua { &self.buy } else { &self.ban };
        o.iter().filter(|m| m.quantity > 0).count()
    }
}

// ============================================================================
// 4. MẠCH KIỂM TRA RỦI RO — tổ hợp thuần tuý, 1 chu kỳ
// ============================================================================
// Toàn bộ cổng rủi ro của Chương 77 nén thành logic tổ hợp: mọi điều kiện
// được tính SONG SONG rồi OR lại. Không có `if` tuần tự, không có nhánh dự
// đoán sai — thời gian luôn bằng nhau, kể cả khi lệnh bị từ chối.

#[derive(Debug, Clone, Copy, PartialEq, Eq, Default)]
pub struct HasReject {
    pub quantity_no: bool,
    pub price_no: bool,
    pub exceed_value: bool,
    pub exceed_position: bool,
    pub switch_all: bool,
}

impl HasReject {
    /// Gộp mọi cờ bằng OR — trên phần cứng là một cổng OR nhiều đầu vào,
    /// độ sâu log₂(số cờ).
    pub fn is_block(&self) -> bool {
        self.quantity_no || self.price_no || self.exceed_value
            || self.exceed_position || self.switch_all
    }
    pub fn num_has_enable(&self) -> u32 {
        [self.quantity_no, self.price_no, self.exceed_value,
         self.exceed_position, self.switch_all].iter().filter(|&&x| x).count() as u32
    }
}

#[derive(Debug, Clone, Copy)]
pub struct RiskCircuit {
    pub max_value: i64,
    pub max_position: i64,
    pub position: i64,
    pub switch_all: bool,
}

impl RiskCircuit {
    /// TẤT CẢ điều kiện tính song song. Đây là điểm khác biệt cốt lõi so với
    /// phần mềm: dù lệnh hợp lệ hay bị chặn, mạch vẫn tốn đúng một chu kỳ.
    /// Không có "đường fast" và "đường chậm" → độ trễ không dao động, và
    /// thời gian phản hồi không tiết lộ điều gì về nội dung lệnh.
    pub fn check(&self, la_mua: bool, price: i64, quantity: i64) -> HasReject {
        let first = if la_mua { 1i64 } else { -1 };
        HasReject {
            quantity_no: quantity <= 0,
            price_no: price <= 0,
            exceed_value: price.saturating_mul(quantity) > self.max_value,
            exceed_position: self.position.saturating_add(first.saturating_mul(quantity))
                             .saturating_abs() > self.max_position,
            switch_all: self.switch_all,
        }
    }
    pub fn period_check(&self) -> u32 { 1 }
}

// ============================================================================
// 5. ĐƯỜNG ỐNG TICK-TO-TRADE
// ============================================================================

#[derive(Debug, Clone, PartialEq)]
pub struct PipelineStage { pub name: String, pub period: u32 }

#[derive(Debug, PartialEq)]
pub struct HwPipeline { pub tang: Vec<PipelineStage> }

impl HwPipeline {
    /// Đường ống điển hình của một hệ thống giao dịch trên FPGA.
    pub fn typical() -> Self {
        HwPipeline {
            tang: vec![
                PipelineStage { name: "MAC/PHY nhận khung".into(), period: 3 },
                PipelineStage { name: "Tách trường song song".into(), period: 1 },
                PipelineStage { name: "Cập nhật sổ lệnh".into(), period: 1 },
                PipelineStage { name: "Tính tín hiệu".into(), period: 2 },
                PipelineStage { name: "Kiểm tra rủi ro".into(), period: 1 },
                PipelineStage { name: "Dựng gói lệnh".into(), period: 2 },
                PipelineStage { name: "MAC/PHY phát khung".into(), period: 3 },
            ],
        }
    }

    /// ĐỘ TRỄ: một gói tin đi hết đường ống mất bao nhiêu chu kỳ.
    pub fn latency_period(&self) -> u32 { self.tang.iter().map(|t| t.period).sum() }
    pub fn latency_nanos(&self) -> f64 { cycles_to_ns(self.latency_period()) }

    /// THÔNG LƯỢNG: sau khi ống đầy, cứ mỗi `first_period_block` là một gói xong.
    /// Bằng chu kỳ của tầng CHẬM NHẤT — không phải tổng các tầng.
    pub fn first_period_block(&self) -> u32 {
        self.tang.iter().map(|t| t.period).max().unwrap_or(1)
    }
    pub fn packets_per_second(&self) -> f64 {
        1e9 / cycles_to_ns(self.first_period_block())
    }

    /// Xử lý `n` gói mất bao nhiêu chu kỳ (có đường ống).
    pub fn total_period_wait(&self, n: u32) -> u32 {
        if n == 0 { return 0; }
        self.latency_period() + (n - 1) * self.first_period_block()
    }

    /// Nếu KHÔNG có đường ống: gói sau phải chờ gói trước ra hẳn.
    pub fn total_cycles_no_pipeline(&self, n: u32) -> u32 { n * self.latency_period() }
}

/// Ngân sách phần mềm tương ứng, lấy từ Chương 74 (đơn vị nano-giây).
pub fn software_latency_ns() -> f64 { 3_400.0 }

// ============================================================================
// 6. VÌ SAO VẪN CẦN PHẦN MỀM — kiến trúc lai
// ============================================================================
// FPGA rất fast nhưng rất khó sửa: một thay đổi nhỏ tốn hàng chục phút tổng
// hợp mạch. Thực tế người ta chia đôi: đường CỰC NÓNG nằm trên FPGA, còn
// logic hay đổi thì nằm trên CPU.

#[derive(Debug, Clone, Copy, PartialEq)]
pub enum ExecutionUnit { Hardware, PhanMem }

#[derive(Debug, Clone, PartialEq)]
pub struct HeavyStage {
    pub name: String,
    pub rate_swap: u32, // số lần sửa mỗi năm
    pub on_hot_path: bool,
}

/// Quy tắc chia việc: nằm trên đường nóng VÀ ít thay đổi thì đưa xuống phần
/// cứng. Hay đổi thì giữ trên phần mềm, dù có nóng — vì mỗi lần sửa mạch tốn
/// hàng chục phút, và một chiến lược không thử nghiệm được là chiến lược chết.
pub fn partial_sum(c: &HeavyStage) -> ExecutionUnit {
    if c.on_hot_path && c.rate_swap <= 4 {
        ExecutionUnit::Hardware
    } else {
        ExecutionUnit::PhanMem
    }
}

fn main() {
    println!("═══════════════════════════════════════════════════════════");
    println!("   FPGA CHO GIAO DỊCH: TICK-TO-TRADE TÍNH BẰNG CHU KỲ      ");
    println!("═══════════════════════════════════════════════════════════");

    println!("\n1. TÁCH TRƯỜNG SONG SONG");
    let mut bt = FieldExtractor::default();
    let mut goi = vec![b'A'];
    goi.extend_from_slice(&7u32.to_be_bytes());
    goi.extend_from_slice(&8_450i64.to_be_bytes());
    goi.extend_from_slice(&100u32.to_be_bytes());
    let account = xor_tree(&goi) & 0x00FF_FFFF;
    goi.extend_from_slice(&account.to_be_bytes()[1..]);
    let t = bt.tach(&goi).unwrap();
    println!("   Gói {} byte → loại {:?} · mã ck {} · giá {} · số lượng {} · hợp lệ {}",
             goi.len(), t.kind as char, t.id_chain, t.price, t.quantity, t.is_valid);
    println!("   Phần cứng tách TẤT CẢ trường trong {} chu kỳ = {} ns",
             bt.period_split(), cycles_to_ns(bt.period_split()));

    println!("\n2. CÂY XOR — rút gọn song song");
    println!("   {:>8} {:>18} {:>18}", "số byte", "cây (log n tầng)", "tuần tự (n tầng)");
    for n in [4usize, 16, 64, 256, 1024] {
        println!("   {:>8} {:>18} {:>18}", n, xor_tree_depth(n), n);
    }
    let d: Vec<u8> = (0..=255).collect();
    println!("   Cùng kết quả với cách tuần tự: {}", xor_tree(&d) == xor_tuan_tu(&d));

    println!("\n3. SỔ LỆNH TRÊN THANH GHI");
    let mut so = OrderBookHardware::default();
    for (g, kl) in [(8_400i64, 500u32), (8_390, 300), (8_380, 200)] {
        so.update(true, g, kl);
    }
    for (g, kl) in [(8_410i64, 400u32), (8_420, 250)] { so.update(false, g, kl); }
    println!("   Mua tốt nhất {:?} · bán tốt nhất {:?}",
             so.best_bid().unwrap(), so.best_ask().unwrap());
    println!("   Chênh lệch {} tick · tìm giá tốt nhất tốn {} tầng so sánh = 1 chu kỳ",
             so.spread().unwrap(), OrderBookHardware::comparator_depth());

    println!("\n4. MẠCH KIỂM TRA RỦI RO — thời gian KHÔNG đổi");
    let m = RiskCircuit { max_value: 1_000_000, max_position: 500,
                        position: 0, switch_all: false };
    for (description, price, sl) in [("hợp lệ        ", 8_400i64, 100i64),
                             ("số lượng âm   ", 8_400, -5),
                             ("giá trị quá to", 8_400, 1_000),
                             ("cả hai lỗi    ", 0, -1)] {
        let c = m.check(true, price, sl);
        println!("   {} → chặn {:<5} ({} cờ bật) · luôn {} chu kỳ",
                 description, c.is_block(), c.num_has_enable(), m.period_check());
    }
    println!("   → Hợp lệ hay không cũng tốn đúng một chu kỳ: độ trễ không dao động,");
    println!("     và thời gian phản hồi không tiết lộ gì về nội dung lệnh.");

    println!("\n5. ĐƯỜNG ỐNG TICK-TO-TRADE");
    let ong = HwPipeline::typical();
    for t in &ong.tang {
        println!("   {:<26} {} chu kỳ = {:>4.0} ns", t.name, t.period, cycles_to_ns(t.period));
    }
    println!("   ─────────────────────────────────────────");
    println!("   Độ trễ     : {} chu kỳ = {:.0} ns", ong.latency_period(), ong.latency_nanos());
    println!("   Thông lượng: 1 gói mỗi {} chu kỳ = {:.0} triệu gói/giây",
             ong.first_period_block(), ong.packets_per_second() / 1e6);
    println!("   So với phần mềm ({} ns) → fast gấp {:.0} lần",
             software_latency_ns(), software_latency_ns() / ong.latency_nanos());

    println!("\n6. ĐƯỜNG ỐNG SO VỚI KHÔNG ĐƯỜNG ỐNG (1000 gói)");
    println!("   Có ống   : {:>7} chu kỳ", ong.total_period_wait(1_000));
    println!("   Không ống: {:>7} chu kỳ", ong.total_cycles_no_pipeline(1_000));
    println!("   → Nhanh gấp {:.1} lần về THÔNG LƯỢNG, nhưng ĐỘ TRỄ vẫn y nguyên {} ns.",
             ong.total_cycles_no_pipeline(1_000) as f64 / ong.total_period_wait(1_000) as f64,
             ong.latency_nanos());

    println!("\n7. CHIA VIỆC GIỮA PHẦN CỨNG VÀ PHẦN MỀM");
    let cn = vec![
        HeavyStage { name: "Tách gói tin".into(), rate_swap: 1, on_hot_path: true },
        HeavyStage { name: "Cập nhật sổ lệnh".into(), rate_swap: 2, on_hot_path: true },
        HeavyStage { name: "Kiểm tra rủi ro cứng".into(), rate_swap: 3, on_hot_path: true },
        HeavyStage { name: "Logic chiến lược".into(), rate_swap: 200, on_hot_path: true },
        HeavyStage { name: "Báo cáo cuối ngày".into(), rate_swap: 12, on_hot_path: false },
        HeavyStage { name: "Hiệu chỉnh tham số".into(), rate_swap: 500, on_hot_path: true },
    ];
    for c in &cn {
        println!("   {:<24} đổi {:>3} lần/năm · nóng {:<5} → {:?}",
                 c.name, c.rate_swap, c.on_hot_path, partial_sum(c));
    }
    println!("   → Chiến lược ở lại phần mềm dù rất nóng: một chiến lược không");
    println!("     thử nghiệm được là chiến lược chết, dù nó fast tới đâu.");

    println!("\n═══════════════════════════════════════════════════════════");
    println!("   PHẦN CỨNG THẮNG Ở SỰ ỔN ĐỊNH, KHÔNG CHỈ Ở TỐC ĐỘ         ");
    println!("═══════════════════════════════════════════════════════════");
}

#[cfg(test)]
mod tests {
    use super::*;

    fn call_hop_le(kind: u8, id_chain: u32, price: i64, sl: u32) -> Vec<u8> {
        let mut g = vec![kind];
        g.extend_from_slice(&id_chain.to_be_bytes());
        g.extend_from_slice(&price.to_be_bytes());
        g.extend_from_slice(&sl.to_be_bytes());
        let account = xor_tree(&g) & 0x00FF_FFFF;
        g.extend_from_slice(&account.to_be_bytes()[1..]);
        g
    }

    // ---------- Cây XOR ----------
    #[test]
    fn the_xor_tree_matches_sequential_xor() {
        // Bất biến: song song hoá KHÔNG được đổi kết quả. XOR có tính kết hợp
        // và giao hoán nên gộp theo cây hay theo chuỗi đều như nhau.
        for n in [0usize, 1, 2, 3, 4, 7, 16, 17, 64, 255, 256] {
            let d: Vec<u8> = (0..n).map(|i| (i * 37 % 251) as u8).collect();
            assert_eq!(xor_tree(&d), xor_tuan_tu(&d), "n={}", n);
        }
    }

    #[test]
    fn tree_depth_is_logarithmic_not_linear() {
        assert_eq!(xor_tree_depth(1), 0);
        assert_eq!(xor_tree_depth(2), 1);
        assert_eq!(xor_tree_depth(4), 2);
        assert_eq!(xor_tree_depth(256), 8);
        assert_eq!(xor_tree_depth(1024), 10, "1024 byte chỉ cần 10 tầng, không phải 1024");
    }

    // ---------- Tách trường ----------
    #[test]
    fn extracts_every_field_correctly() {
        let mut bt = FieldExtractor::default();
        let g = call_hop_le(b'A', 12_345, 8_450, 100);
        let t = bt.tach(&g).unwrap();
        assert_eq!(t.kind, b'A');
        assert_eq!(t.id_chain, 12_345);
        assert_eq!(t.price, 8_450);
        assert_eq!(t.quantity, 100);
        assert!(t.is_valid);
        assert_eq!(bt.so_goi_da_tach, 1);
        assert_eq!(bt.so_goi_hong, 0);
    }

    #[test]
    fn tach_dung_ca_gia_am_va_gia_tri_bien() {
        let mut bt = FieldExtractor::default();
        for (price, sl) in [(-1i64, 0u32), (i64::MIN, u32::MAX), (i64::MAX, 1)] {
            let g = call_hop_le(b'X', 0, price, sl);
            let t = bt.tach(&g).unwrap();
            assert_eq!(t.price, price, "giá {} phải tách đúng", price);
            assert_eq!(t.quantity, sl);
        }
    }

    #[test]
    fn short_packets_are_rejected() {
        let mut bt = FieldExtractor::default();
        for n in 0..DAI_GOI {
            assert_eq!(bt.tach(&vec![0u8; n]), None, "gói {} byte phải bị từ chối", n);
        }
        assert_eq!(bt.so_goi_hong, DAI_GOI as u64);
    }

    #[test]
    fn a_bad_checksum_marks_the_packet_invalid() {
        let mut bt = FieldExtractor::default();
        let mut g = call_hop_le(b'A', 1, 100, 10);
        g[19] ^= 0xFF; // phá tổng kiểm tra
        let t = bt.tach(&g).unwrap();
        assert!(!t.is_valid, "gói hỏng phải bị đánh dấu, KHÔNG được im lặng cho qua");
        assert_eq!(bt.so_goi_hong, 1);
        assert_eq!(bt.so_goi_da_tach, 0);
    }

    #[test]
    fn a_single_bit_flip_in_the_body_is_caught() {
        let mut bt = FieldExtractor::default();
        for pos_value in 0..17usize {
            let mut g = call_hop_le(b'A', 999, 8_400, 500);
            g[pos_value] ^= 1;
            let t = bt.tach(&g).unwrap();
            assert!(!t.is_valid, "lật bit ở byte {} mà không bị phát hiện", pos_value);
        }
    }

    #[test]
    fn extraction_always_costs_exactly_one_cycle() {
        let bt = FieldExtractor::default();
        assert_eq!(bt.period_split(), 1, "phần cứng tách mọi trường song song");
    }

    // ---------- Sổ lệnh phần cứng ----------
    #[test]
    fn an_empty_book_has_no_best_price() {
        let s = OrderBookHardware::default();
        assert_eq!(s.best_bid(), None);
        assert_eq!(s.best_ask(), None);
        assert_eq!(s.spread(), None);
    }

    #[test]
    fn reports_best_on_both_sides() {
        let mut s = OrderBookHardware::default();
        for (g, kl) in [(8_380i64, 100u32), (8_400, 500), (8_390, 300)] {
            s.update(true, g, kl);
        }
        for (g, kl) in [(8_430i64, 100u32), (8_410, 400), (8_420, 250)] {
            s.update(false, g, kl);
        }
        assert_eq!(s.best_bid().unwrap().price, 8_400, "bên bid lấy giá CAO nhất");
        assert_eq!(s.best_ask().unwrap().price, 8_410, "bên bán lấy giá THẤP nhất");
        assert_eq!(s.spread(), Some(10));
    }

    #[test]
    fn updating_an_existing_level_overwrites_its_size() {
        let mut s = OrderBookHardware::default();
        s.update(true, 8_400, 500);
        s.update(true, 8_400, 700);
        assert_eq!(s.num_level_dang_use(true), 1, "không được tạo mức trùng");
        assert_eq!(s.best_bid().unwrap().quantity, 700);
    }

    #[test]
    fn zeroing_the_size_removes_the_level() {
        let mut s = OrderBookHardware::default();
        s.update(true, 8_400, 500);
        s.update(true, 8_390, 300);
        s.update(true, 8_400, 0);
        assert_eq!(s.best_bid().unwrap().price, 8_390, "đỉnh phải tụt xuống mức kế");
        assert_eq!(s.num_level_dang_use(true), 1);
    }

    #[test]
    fn a_full_book_keeps_the_best_levels() {
        // Sổ phần cứng chỉ có 8 ô. Khi đầy, mức tệ nhất phải bị đẩy ra —
        // nếu không, ta sẽ giữ những mức giá vô dụng và bỏ mất mức tốt.
        let mut s = OrderBookHardware::default();
        for i in 0..SO_MUC_PHAN_CUNG as i64 {
            s.update(true, 8_000 + i, 100);
        }
        assert_eq!(s.num_level_dang_use(true), SO_MUC_PHAN_CUNG);
        assert_eq!(s.best_bid().unwrap().price, 8_007);
        // Mức tốt hơn hẳn → phải chen vào được
        s.update(true, 9_000, 100);
        assert_eq!(s.best_bid().unwrap().price, 9_000);
        assert_eq!(s.num_level_dang_use(true), SO_MUC_PHAN_CUNG, "vẫn đúng 8 ô");
        // Mức tệ hơn tất cả → phải bị bỏ qua
        s.update(true, 1, 100);
        assert!(s.buy.iter().all(|m| m.price != 1), "mức tệ không được chiếm chỗ");
    }

    #[test]
    fn the_ask_side_also_keeps_its_best_levels() {
        let mut s = OrderBookHardware::default();
        for i in 0..SO_MUC_PHAN_CUNG as i64 {
            s.update(false, 9_000 - i, 100);
        }
        assert_eq!(s.best_ask().unwrap().price, 8_993);
        s.update(false, 8_000, 100); // rẻ hơn hẳn = tốt hơn cho bên bán
        assert_eq!(s.best_ask().unwrap().price, 8_000);
        s.update(false, 99_999, 100); // đắt vô lý
        assert!(s.ban.iter().all(|m| m.price != 99_999));
    }

    #[test]
    fn comparator_depth_is_logarithmic_in_levels() {
        assert_eq!(OrderBookHardware::comparator_depth(), 3, "8 mức → 3 tầng cây so sánh");
    }

    // ---------- Mạch rủi ro ----------
    fn circuit() -> RiskCircuit {
        RiskCircuit { max_value: 1_000_000, max_position: 500,
                    position: 0, switch_all: false }
    }

    #[test]
    fn a_valid_order_raises_no_flag() {
        let c = circuit().check(true, 8_400, 100);
        assert!(!c.is_block());
        assert_eq!(c.num_has_enable(), 0);
    }

    #[test]
    fn each_condition_raises_its_own_flag() {
        let m = circuit();
        assert!(m.check(true, 8_400, 0).quantity_no);
        assert!(m.check(true, 0, 100).price_no);
        assert!(m.check(true, 8_400, 1_000).exceed_value);
        assert!(m.check(true, 100, 600).exceed_position);
        let tat = RiskCircuit { switch_all: true, ..m };
        assert!(tat.check(true, 8_400, 100).switch_all);
    }

    #[test]
    fn multiple_violations_raise_multiple_flags() {
        // Đây là điểm khác biệt thật so với phần mềm: phần mềm `return` ở lỗi
        // ĐẦU TIÊN nên chỉ biết một lỗi; mạch tính song song nên thấy HẾT.
        let c = circuit().check(true, 0, -1);
        assert!(c.quantity_no && c.price_no);
        assert!(c.num_has_enable() >= 2, "phần cứng thấy mọi lỗi cùng lúc, không dừng ở lỗi đầu");
    }

    #[test]
    fn the_short_side_is_bounded_by_the_position_limit_too() {
        let m = circuit();
        assert!(m.check(false, 100, 600).exceed_position, "chiều bán cũng phải bị chặn");
    }

    #[test]
    fn current_position_is_counted_in() {
        let m = RiskCircuit { position: 450, ..circuit() };
        assert!(!m.check(true, 100, 50).exceed_position, "450+50 = 500, vừa trần");
        assert!(m.check(true, 100, 51).exceed_position, "450+51 vượt trần");
        assert!(!m.check(false, 100, 500).exceed_position, "bán thì giảm vị thế");
    }

    #[test]
    fn the_multiply_never_overflows() {
        let m = circuit();
        // Toàn bộ dùng phép bão hoà: không được panic, và phải báo vượt hạn mức
        let c = m.check(true, i64::MAX, i64::MAX);
        assert!(c.exceed_value);
        assert!(c.is_block());
        let c2 = m.check(false, 1, i64::MAX);
        assert!(c2.is_block());
    }

    #[test]
    fn the_check_always_costs_exactly_one_cycle() {
        // Bất biến quan trọng nhất của mạch rủi ro: thời gian KHÔNG phụ thuộc
        // dữ liệu. Nhờ vậy độ trễ không dao động và không rò rỉ thông tin.
        let m = circuit();
        assert_eq!(m.period_check(), 1);
        for (g, sl) in [(8_400i64, 100i64), (0, 0), (-1, -1), (i64::MAX, i64::MAX)] {
            let _ = m.check(true, g, sl);
            assert_eq!(m.period_check(), 1, "mọi đầu vào đều tốn đúng 1 chu kỳ");
        }
    }

    // ---------- Đường ống ----------
    #[test]
    fn latency_is_the_sum_of_the_stages() {
        let o = HwPipeline::typical();
        assert_eq!(o.latency_period(), 3 + 1 + 1 + 2 + 1 + 2 + 3);
        assert!((o.latency_nanos() - 13.0 * NS_MOI_CHU_KY).abs() < 1e-9);
    }

    #[test]
    fn throughput_is_set_by_the_slowest_stage_not_the_sum() {
        // Nhầm hai đại lượng này là hiểu sai toàn bộ kiến trúc đường ống.
        let o = HwPipeline::typical();
        assert_eq!(o.first_period_block(), 3, "tầng chậm nhất là 3 chu kỳ");
        assert!(o.first_period_block() < o.latency_period());
    }

    #[test]
    fn pipelining_raises_throughput_without_cutting_latency() {
        let o = HwPipeline::typical();
        // Một gói: y hệt nhau
        assert_eq!(o.total_period_wait(1), o.latency_period());
        assert_eq!(o.total_cycles_no_pipeline(1), o.latency_period());
        // Nhiều gói: đường ống thắng đậm
        assert!(o.total_period_wait(1_000) * 4 < o.total_cycles_no_pipeline(1_000));
        // Nhưng độ trễ của MỘT gói vẫn y nguyên
        assert_eq!(o.latency_period(), 13);
    }

    #[test]
    fn no_packets_means_no_cycles() {
        let o = HwPipeline::typical();
        assert_eq!(o.total_period_wait(0), 0);
        assert_eq!(o.total_cycles_no_pipeline(0), 0);
    }

    #[test]
    fn hardware_beats_software_by_an_order_of_magnitude() {
        let o = HwPipeline::typical();
        let ratio = software_latency_ns() / o.latency_nanos();
        assert!(ratio > 50.0, "phải fast hơn ít nhất 50 lần, thực tế {:.0}", ratio);
        assert!(o.latency_nanos() < 100.0, "tick-to-trade phải dưới 100 ns");
    }

    #[test]
    fn throughput_reaches_hundreds_of_millions_of_packets() {
        let o = HwPipeline::typical();
        assert!(o.packets_per_second() > 50e6,
                "phải trên 50 triệu gói/giây, thực tế {:.0}", o.packets_per_second());
    }

    // ---------- Phân công phần cứng/phần mềm ----------
    #[test]
    fn hot_and_stable_work_belongs_in_hardware() {
        let c = HeavyStage { name: "tách gói".into(), rate_swap: 1, on_hot_path: true };
        assert_eq!(partial_sum(&c), ExecutionUnit::Hardware);
    }

    #[test]
    fn volatile_work_stays_in_software_even_if_hot() {
        // Bài học kiến trúc quan trọng nhất của chương: tốc độ không đáng giá
        // bằng khả năng thay đổi. Chiến lược sửa 200 lần/năm mà nằm trên FPGA
        // thì mỗi lần thử nghiệm tốn hàng chục phút tổng hợp mạch.
        let c = HeavyStage { name: "chiến lược".into(), rate_swap: 200,
                           on_hot_path: true };
        assert_eq!(partial_sum(&c), ExecutionUnit::PhanMem);
    }

    #[test]
    fn cold_work_stays_in_software_even_if_stable() {
        let c = HeavyStage { name: "báo cáo".into(), rate_swap: 1,
                           on_hot_path: false };
        assert_eq!(partial_sum(&c), ExecutionUnit::PhanMem,
                   "không nằm trên đường nóng thì đưa xuống phần cứng là lãng phí");
    }

    #[test]
    fn cycles_convert_to_nanoseconds_correctly() {
        assert!((cycles_to_ns(1) - 4.0).abs() < 1e-9);
        assert!((cycles_to_ns(250) - 1_000.0).abs() < 1e-9, "250 chu kỳ ở 250 MHz = 1 µs");
        assert_eq!(cycles_to_ns(0), 0.0);
    }
}
