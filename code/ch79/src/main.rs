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
// tới, người ổn định thắng người nhanh-nhưng-thất-thường.

/// Chu kỳ xung nhịp của FPGA giao dịch điển hình: 250 MHz → 4 ns mỗi chu kỳ.
pub const NS_PER_CYCLE: f64 = 4.0;

pub fn cycles_to_ns(cycles: u32) -> f64 {
    cycles as f64 * NS_PER_CYCLE
}

// ============================================================================
// 2. TÁCH TRƯỜNG SONG SONG — điều phần mềm không làm được
// ============================================================================
// Phần mềm đọc từng trường một: đọc offset 0, rồi 8, rồi 16… Mỗi lần là một
// lệnh CPU. Phần cứng nối THẲNG dây từ mọi vị trí byte tới mọi thanh ghi đích,
// nên TẤT CẢ trường được tách trong CÙNG MỘT chu kỳ.

#[derive(Debug, Clone, Copy, PartialEq, Eq, Default)]
pub struct PacketField {
    pub kind: u8,
    pub symbol_id: u32,
    pub price: i64,
    pub quantity: u32,
    pub is_valid: bool,
}

/// Bố cục gói tin cố định 20 byte:
/// `[loại 1B][mã ck 4B][giá 8B][số lượng 4B][tổng kiểm tra 3B]`
pub const PACKET_LEN: usize = 20;

#[derive(Debug, Default)]
pub struct FieldExtractor {
    pub packets_parsed: u64,
    pub packets_rejected: u64,
}

impl FieldExtractor {
    /// Tách toàn bộ trường trong ĐÚNG MỘT chu kỳ. Trong Rust ta viết tuần tự,
    /// nhưng khi tổng hợp ra mạch thì các phép gán này là dây nối song song —
    /// không có "trước" và "sau", tất cả xảy ra cùng lúc.
    pub fn extract(&mut self, packet: &[u8]) -> Option<PacketField> {
        if packet.len() < PACKET_LEN {
            self.packets_rejected += 1;
            return None;
        }

        let t = PacketField {
            kind: packet[0],
            symbol_id: u32::from_be_bytes([packet[1], packet[2], packet[3], packet[4]]),
            price: i64::from_be_bytes([
                packet[5], packet[6], packet[7], packet[8], packet[9], packet[10], packet[11],
                packet[12],
            ]),
            quantity: u32::from_be_bytes([packet[13], packet[14], packet[15], packet[16]]),
            is_valid: true,
        };

        // Tổng kiểm tra cũng tính SONG SONG bằng cây XOR — độ sâu log(n)
        // thay vì n bước cộng dồn như phần mềm. (XOR các byte cho ra một byte,
        // nên hai byte đầu của trường 3 byte luôn bằng 0 — giữ cho bố cục đơn
        // giản; giao thức thật dùng CRC.)
        let checksum = xor_tree(&packet[..17]) & 0x00FF_FFFF;
        let expected = u32::from_be_bytes([0, packet[17], packet[18], packet[19]]);
        if checksum != expected {
            self.packets_rejected += 1;
            return Some(PacketField {
                is_valid: false,
                ..t
            });
        }
        self.packets_parsed += 1;
        Some(t)
    }

    /// Số chu kỳ để tách một gói. Phần cứng: LUÔN LUÔN 1.
    pub fn extract_cycles(&self) -> u32 {
        1
    }
}

/// Cây XOR: gộp từng cặp, độ sâu ⌈log₂(n)⌉ tầng cổng thay vì n tầng.
/// Đây là mẫu "rút gọn song song" — nền của mọi phép gộp trên phần cứng và GPU.
pub fn xor_tree(data: &[u8]) -> u32 {
    let mut level: Vec<u32> = data.iter().map(|&b| b as u32).collect();
    while level.len() > 1 {
        let mut next_level = Vec::with_capacity(level.len().div_ceil(2));
        for pair in level.chunks(2) {
            next_level.push(pair[0] ^ pair.get(1).copied().unwrap_or(0));
        }
        level = next_level;
    }
    level.first().copied().unwrap_or(0)
}

pub fn xor_tree_depth(n: usize) -> u32 {
    if n <= 1 {
        return 0;
    }
    (n as f64).log2().ceil() as u32
}

/// Cách phần mềm làm: cộng dồn tuần tự, n bước phụ thuộc nhau.
pub fn xor_sequential(data: &[u8]) -> u32 {
    data.iter().fold(0u32, |a, &b| a ^ b as u32)
}

// ============================================================================
// 3. SỔ LỆNH TRÊN THANH GHI
// ============================================================================
// Phần mềm dùng BTreeMap: O(log n) nhưng có nhảy con trỏ và trượt cache.
// Phần cứng giữ N mức giá tốt nhất trong THANH GHI và so sánh TẤT CẢ cùng lúc
// bằng một mạng so sánh độ sâu log₂(N). Với N nhỏ (8–16 mức), cả cây vừa
// trong MỘT chu kỳ; N lớn hơn thì cây sâu hơn và phải chia thêm tầng ống.

pub const HW_LEVELS: usize = 8;

#[derive(Debug, Clone, Copy, PartialEq, Eq, Default)]
pub struct HwPriceLevel {
    pub price: i64,
    pub quantity: u32,
}

/// Sổ lệnh "nông nhưng nhanh": chỉ giữ 8 mức tốt nhất mỗi bên. Đủ cho gần
/// như mọi chiến lược, và vừa trọn trong thanh ghi FPGA.
#[derive(Debug, Clone, Copy)]
pub struct HwOrderBook {
    pub bids: [HwPriceLevel; HW_LEVELS],
    pub asks: [HwPriceLevel; HW_LEVELS],
}

impl Default for HwOrderBook {
    fn default() -> Self {
        HwOrderBook {
            bids: [HwPriceLevel::default(); HW_LEVELS],
            asks: [HwPriceLevel::default(); HW_LEVELS],
        }
    }
}

impl HwOrderBook {
    /// Bộ mã hoá ưu tiên: tìm mức mua có giá CAO nhất. Trên phần cứng đây là
    /// một cây so sánh độ sâu log₂(8) = 3 tầng, chạy trong MỘT chu kỳ.
    /// Phần mềm phải duyệt 8 phần tử — 8 lần so sánh phụ thuộc nhau.
    pub fn best_bid(&self) -> Option<HwPriceLevel> {
        self.bids
            .iter()
            .filter(|m| m.quantity > 0)
            .max_by_key(|m| m.price)
            .copied()
    }
    pub fn best_ask(&self) -> Option<HwPriceLevel> {
        self.asks
            .iter()
            .filter(|m| m.quantity > 0)
            .min_by_key(|m| m.price)
            .copied()
    }
    pub fn spread(&self) -> Option<i64> {
        Some(self.best_ask()?.price - self.best_bid()?.price)
    }

    /// Cập nhật một mức giá. Mọi ô so sánh SONG SONG với giá đầu vào, nên
    /// với vài chục mức trở xuống vẫn tốn đúng một chu kỳ.
    pub fn update(&mut self, is_buy: bool, price: i64, quantity: u32) {
        let side_levels = if is_buy {
            &mut self.bids
        } else {
            &mut self.asks
        };
        // Đã có mức giá này chưa?
        if let Some(m) = side_levels
            .iter_mut()
            .find(|m| m.price == price && m.quantity > 0)
        {
            m.quantity = quantity;
            if quantity == 0 {
                m.price = 0;
            }
            return;
        }
        if quantity == 0 {
            return;
        }
        // Ô trống?
        if let Some(m) = side_levels.iter_mut().find(|m| m.quantity == 0) {
            *m = HwPriceLevel { price, quantity };
            return;
        }
        // Đầy: thay mức TỆ NHẤT nếu mức mới tốt hơn
        let worst = if is_buy {
            side_levels.iter_mut().min_by_key(|m| m.price).unwrap()
        } else {
            side_levels.iter_mut().max_by_key(|m| m.price).unwrap()
        };
        let better = if is_buy {
            price > worst.price
        } else {
            price < worst.price
        };
        if better {
            *worst = HwPriceLevel { price, quantity };
        }
    }

    /// Độ sâu cây so sánh — quyết định tần số tối đa của mạch.
    pub fn comparator_depth() -> u32 {
        xor_tree_depth(HW_LEVELS)
    }

    pub fn levels_in_use(&self, is_buy: bool) -> usize {
        let side_levels = if is_buy { &self.bids } else { &self.asks };
        side_levels.iter().filter(|m| m.quantity > 0).count()
    }
}

// ============================================================================
// 4. MẠCH KIỂM TRA RỦI RO — tổ hợp thuần tuý, 1 chu kỳ
// ============================================================================
// Toàn bộ cổng rủi ro của Chương 77 nén thành logic tổ hợp: mọi điều kiện
// được tính SONG SONG rồi OR lại. Không có `if` tuần tự, không có nhánh dự
// đoán sai — thời gian luôn bằng nhau, kể cả khi lệnh bị từ chối.

#[derive(Debug, Clone, Copy, PartialEq, Eq, Default)]
pub struct RejectFlags {
    pub invalid_quantity: bool,
    pub invalid_price: bool,
    pub exceed_value: bool,
    pub exceed_position: bool,
    pub kill_switch: bool,
}

impl RejectFlags {
    /// Gộp mọi cờ bằng OR — trên phần cứng là một cổng OR nhiều đầu vào,
    /// độ sâu log₂(số cờ).
    pub fn is_blocked(&self) -> bool {
        self.invalid_quantity
            || self.invalid_price
            || self.exceed_value
            || self.exceed_position
            || self.kill_switch
    }
    pub fn count_raised(&self) -> u32 {
        [
            self.invalid_quantity,
            self.invalid_price,
            self.exceed_value,
            self.exceed_position,
            self.kill_switch,
        ]
        .iter()
        .filter(|&&x| x)
        .count() as u32
    }
}

#[derive(Debug, Clone, Copy)]
pub struct RiskCircuit {
    pub max_value: i64,
    pub max_position: i64,
    pub position: i64,
    pub kill_switch: bool,
}

impl RiskCircuit {
    /// TẤT CẢ điều kiện tính song song. Đây là điểm khác biệt cốt lõi so với
    /// phần mềm: dù lệnh hợp lệ hay bị chặn, mạch vẫn tốn đúng một chu kỳ.
    /// Không có "đường nhanh" và "đường chậm" → độ trễ không dao động, và
    /// thời gian phản hồi không tiết lộ điều gì về nội dung lệnh.
    pub fn check(&self, is_buy: bool, price: i64, quantity: i64) -> RejectFlags {
        let sign = if is_buy { 1i64 } else { -1 };
        RejectFlags {
            invalid_quantity: quantity <= 0,
            invalid_price: price <= 0,
            exceed_value: price.saturating_mul(quantity) > self.max_value,
            exceed_position: self
                .position
                .saturating_add(sign.saturating_mul(quantity))
                .saturating_abs()
                > self.max_position,
            kill_switch: self.kill_switch,
        }
    }
    pub fn check_cycles(&self) -> u32 {
        1
    }
}

// ============================================================================
// 5. ĐƯỜNG ỐNG TICK-TO-TRADE
// ============================================================================

#[derive(Debug, Clone, PartialEq)]
pub struct PipelineStage {
    pub name: String,
    pub cycles: u32,
}

#[derive(Debug, PartialEq)]
pub struct HwPipeline {
    pub stages: Vec<PipelineStage>,
}

impl HwPipeline {
    /// Đường ống điển hình của một hệ thống giao dịch trên FPGA.
    pub fn typical() -> Self {
        HwPipeline {
            stages: vec![
                PipelineStage {
                    name: "MAC/PHY nhận khung".into(),
                    cycles: 3,
                },
                PipelineStage {
                    name: "Tách trường song song".into(),
                    cycles: 1,
                },
                PipelineStage {
                    name: "Cập nhật sổ lệnh".into(),
                    cycles: 1,
                },
                PipelineStage {
                    name: "Tính tín hiệu".into(),
                    cycles: 2,
                },
                PipelineStage {
                    name: "Kiểm tra rủi ro".into(),
                    cycles: 1,
                },
                PipelineStage {
                    name: "Dựng gói lệnh".into(),
                    cycles: 2,
                },
                PipelineStage {
                    name: "MAC/PHY phát khung".into(),
                    cycles: 3,
                },
            ],
        }
    }

    /// ĐỘ TRỄ: một gói tin đi hết đường ống mất bao nhiêu chu kỳ.
    pub fn latency_cycles(&self) -> u32 {
        self.stages.iter().map(|t| t.cycles).sum()
    }
    pub fn latency_nanos(&self) -> f64 {
        cycles_to_ns(self.latency_cycles())
    }

    /// THÔNG LƯỢNG: sau khi ống đầy, cứ mỗi `initiation_interval` là một gói xong.
    /// Bằng chu kỳ của tầng CHẬM NHẤT — không phải tổng các tầng.
    pub fn initiation_interval(&self) -> u32 {
        self.stages.iter().map(|t| t.cycles).max().unwrap_or(1)
    }
    pub fn packets_per_second(&self) -> f64 {
        1e9 / cycles_to_ns(self.initiation_interval())
    }

    /// Xử lý `n` gói mất bao nhiêu chu kỳ (có đường ống).
    pub fn total_cycles_pipelined(&self, n: u32) -> u32 {
        if n == 0 {
            return 0;
        }
        self.latency_cycles() + (n - 1) * self.initiation_interval()
    }

    /// Nếu KHÔNG có đường ống: gói sau phải chờ gói trước ra hẳn.
    pub fn total_cycles_no_pipeline(&self, n: u32) -> u32 {
        n * self.latency_cycles()
    }
}

/// Ngân sách phần mềm tương ứng (đơn vị nano-giây): tổng các chặng trong
/// `main` của Chương 74 — bản gọi hệ thống, KHÔNG dùng kernel bypass.
pub fn software_latency_ns() -> f64 {
    3_400.0
}

// ============================================================================
// 6. VÌ SAO VẪN CẦN PHẦN MỀM — kiến trúc lai
// ============================================================================
// FPGA rất nhanh nhưng rất khó sửa: một thay đổi nhỏ tốn hàng chục phút tổng
// hợp mạch. Thực tế người ta chia đôi: đường CỰC NÓNG nằm trên FPGA, còn
// logic hay đổi thì nằm trên CPU.

#[derive(Debug, Clone, Copy, PartialEq)]
pub enum ExecutionUnit {
    Hardware,
    Software,
}

#[derive(Debug, Clone, PartialEq)]
pub struct Feature {
    pub name: String,
    pub change_frequency: u32, // số lần sửa mỗi năm
    pub on_hot_path: bool,
}

/// Quy tắc chia việc: nằm trên đường nóng VÀ ít thay đổi thì đưa xuống phần
/// cứng. Hay đổi thì giữ trên phần mềm, dù có nóng — vì mỗi lần sửa mạch tốn
/// hàng chục phút, và một chiến lược không thử nghiệm được là chiến lược chết.
pub fn assign_unit(c: &Feature) -> ExecutionUnit {
    if c.on_hot_path && c.change_frequency <= 4 {
        ExecutionUnit::Hardware
    } else {
        ExecutionUnit::Software
    }
}

fn main() {
    println!("═══════════════════════════════════════════════════════════");
    println!("   FPGA CHO GIAO DỊCH: TICK-TO-TRADE TÍNH BẰNG CHU KỲ      ");
    println!("═══════════════════════════════════════════════════════════");

    println!("\n1. TÁCH TRƯỜNG SONG SONG");
    let mut extractor = FieldExtractor::default();
    let mut packet = vec![b'A'];
    packet.extend_from_slice(&7u32.to_be_bytes());
    packet.extend_from_slice(&8_450i64.to_be_bytes());
    packet.extend_from_slice(&100u32.to_be_bytes());
    let checksum = xor_tree(&packet) & 0x00FF_FFFF;
    packet.extend_from_slice(&checksum.to_be_bytes()[1..]);
    let t = extractor.extract(&packet).unwrap();
    println!(
        "   Gói {} byte → loại {:?} · mã ck {} · giá {} · số lượng {} · hợp lệ {}",
        packet.len(),
        t.kind as char,
        t.symbol_id,
        t.price,
        t.quantity,
        t.is_valid
    );
    println!(
        "   Phần cứng tách TẤT CẢ trường trong {} chu kỳ = {} ns",
        extractor.extract_cycles(),
        cycles_to_ns(extractor.extract_cycles())
    );

    println!("\n2. CÂY XOR — rút gọn song song");
    println!(
        "   {:>8} {:>18} {:>18}",
        "số byte", "cây (log n tầng)", "tuần tự (n tầng)"
    );
    for n in [4usize, 16, 64, 256, 1024] {
        println!("   {:>8} {:>18} {:>18}", n, xor_tree_depth(n), n);
    }
    let d: Vec<u8> = (0..=255).collect();
    println!(
        "   Cùng kết quả với cách tuần tự: {}",
        xor_tree(&d) == xor_sequential(&d)
    );

    println!("\n3. SỔ LỆNH TRÊN THANH GHI");
    let mut book = HwOrderBook::default();
    for (g, qty) in [(8_400i64, 500u32), (8_390, 300), (8_380, 200)] {
        book.update(true, g, qty);
    }
    for (g, qty) in [(8_410i64, 400u32), (8_420, 250)] {
        book.update(false, g, qty);
    }
    println!(
        "   Mua tốt nhất {:?} · bán tốt nhất {:?}",
        book.best_bid().unwrap(),
        book.best_ask().unwrap()
    );
    println!(
        "   Chênh lệch {} tick · tìm giá tốt nhất tốn {} tầng so sánh = 1 chu kỳ",
        book.spread().unwrap(),
        HwOrderBook::comparator_depth()
    );

    println!("\n4. MẠCH KIỂM TRA RỦI RO — thời gian KHÔNG đổi");
    let m = RiskCircuit {
        max_value: 1_000_000,
        max_position: 500,
        position: 0,
        kill_switch: false,
    };
    for (description, price, qty) in [
        ("hợp lệ        ", 8_400i64, 100i64),
        ("số lượng âm   ", 8_400, -5),
        ("giá trị quá to", 8_400, 1_000),
        ("cả hai lỗi    ", 0, -1),
    ] {
        let c = m.check(true, price, qty);
        println!(
            "   {} → chặn {:<5} ({} cờ bật) · luôn {} chu kỳ",
            description,
            c.is_blocked(),
            c.count_raised(),
            m.check_cycles()
        );
    }
    println!("   → Hợp lệ hay không cũng tốn đúng một chu kỳ: độ trễ không dao động,");
    println!("     và thời gian phản hồi không tiết lộ gì về nội dung lệnh.");

    println!("\n5. ĐƯỜNG ỐNG TICK-TO-TRADE");
    let pipeline = HwPipeline::typical();
    for t in &pipeline.stages {
        println!(
            "   {:<26} {} chu kỳ = {:>4.0} ns",
            t.name,
            t.cycles,
            cycles_to_ns(t.cycles)
        );
    }
    println!("   ─────────────────────────────────────────");
    println!(
        "   Độ trễ     : {} chu kỳ = {:.0} ns",
        pipeline.latency_cycles(),
        pipeline.latency_nanos()
    );
    println!(
        "   Thông lượng: 1 gói mỗi {} chu kỳ = {:.0} triệu gói/giây",
        pipeline.initiation_interval(),
        pipeline.packets_per_second() / 1e6
    );
    println!(
        "   So với phần mềm ({} ns) → nhanh gấp {:.0} lần",
        software_latency_ns(),
        software_latency_ns() / pipeline.latency_nanos()
    );

    println!("\n6. ĐƯỜNG ỐNG SO VỚI KHÔNG ĐƯỜNG ỐNG (1000 gói)");
    println!(
        "   Có ống   : {:>7} chu kỳ",
        pipeline.total_cycles_pipelined(1_000)
    );
    println!(
        "   Không ống: {:>7} chu kỳ",
        pipeline.total_cycles_no_pipeline(1_000)
    );
    println!(
        "   → Nhanh gấp {:.1} lần về THÔNG LƯỢNG, nhưng ĐỘ TRỄ vẫn y nguyên {} ns.",
        pipeline.total_cycles_no_pipeline(1_000) as f64
            / pipeline.total_cycles_pipelined(1_000) as f64,
        pipeline.latency_nanos()
    );

    println!("\n7. CHIA VIỆC GIỮA PHẦN CỨNG VÀ PHẦN MỀM");
    let features = vec![
        Feature {
            name: "Tách gói tin".into(),
            change_frequency: 1,
            on_hot_path: true,
        },
        Feature {
            name: "Cập nhật sổ lệnh".into(),
            change_frequency: 2,
            on_hot_path: true,
        },
        Feature {
            name: "Kiểm tra rủi ro cứng".into(),
            change_frequency: 3,
            on_hot_path: true,
        },
        Feature {
            name: "Logic chiến lược".into(),
            change_frequency: 200,
            on_hot_path: true,
        },
        Feature {
            name: "Báo cáo cuối ngày".into(),
            change_frequency: 12,
            on_hot_path: false,
        },
        Feature {
            name: "Hiệu chỉnh tham số".into(),
            change_frequency: 500,
            on_hot_path: true,
        },
    ];
    for c in &features {
        println!(
            "   {:<24} đổi {:>3} lần/năm · nóng {:<5} → {:?}",
            c.name,
            c.change_frequency,
            c.on_hot_path,
            assign_unit(c)
        );
    }
    println!("   → Chiến lược ở lại phần mềm dù rất nóng: một chiến lược không");
    println!("     thử nghiệm được là chiến lược chết, dù nó nhanh tới đâu.");

    println!("\n═══════════════════════════════════════════════════════════");
    println!("   PHẦN CỨNG THẮNG Ở SỰ ỔN ĐỊNH, KHÔNG CHỈ Ở TỐC ĐỘ         ");
    println!("═══════════════════════════════════════════════════════════");
}

#[cfg(test)]
mod tests {
    use super::*;

    fn valid_packet(kind: u8, symbol_id: u32, price: i64, qty: u32) -> Vec<u8> {
        let mut g = vec![kind];
        g.extend_from_slice(&symbol_id.to_be_bytes());
        g.extend_from_slice(&price.to_be_bytes());
        g.extend_from_slice(&qty.to_be_bytes());
        let checksum = xor_tree(&g) & 0x00FF_FFFF;
        g.extend_from_slice(&checksum.to_be_bytes()[1..]);
        g
    }

    // ---------- Cây XOR ----------
    #[test]
    fn the_xor_tree_matches_sequential_xor() {
        // Bất biến: song song hoá KHÔNG được đổi kết quả. XOR có tính kết hợp
        // và giao hoán nên gộp theo cây hay theo chuỗi đều như nhau.
        for n in [0usize, 1, 2, 3, 4, 7, 16, 17, 64, 255, 256] {
            let d: Vec<u8> = (0..n).map(|i| (i * 37 % 251) as u8).collect();
            assert_eq!(xor_tree(&d), xor_sequential(&d), "n={}", n);
        }
    }

    #[test]
    fn tree_depth_is_logarithmic_not_linear() {
        assert_eq!(xor_tree_depth(1), 0);
        assert_eq!(xor_tree_depth(2), 1);
        assert_eq!(xor_tree_depth(4), 2);
        assert_eq!(xor_tree_depth(256), 8);
        assert_eq!(
            xor_tree_depth(1024),
            10,
            "1024 byte chỉ cần 10 tầng, không phải 1024"
        );
    }

    // ---------- Tách trường ----------
    #[test]
    fn extracts_every_field_correctly() {
        let mut extractor = FieldExtractor::default();
        let g = valid_packet(b'A', 12_345, 8_450, 100);
        let t = extractor.extract(&g).unwrap();
        assert_eq!(t.kind, b'A');
        assert_eq!(t.symbol_id, 12_345);
        assert_eq!(t.price, 8_450);
        assert_eq!(t.quantity, 100);
        assert!(t.is_valid);
        assert_eq!(extractor.packets_parsed, 1);
        assert_eq!(extractor.packets_rejected, 0);
    }

    #[test]
    fn extracts_negative_and_extreme_values() {
        let mut extractor = FieldExtractor::default();
        for (price, qty) in [(-1i64, 0u32), (i64::MIN, u32::MAX), (i64::MAX, 1)] {
            let g = valid_packet(b'X', 0, price, qty);
            let t = extractor.extract(&g).unwrap();
            assert_eq!(t.price, price, "giá {} phải tách đúng", price);
            assert_eq!(t.quantity, qty);
        }
    }

    #[test]
    fn short_packets_are_rejected() {
        let mut extractor = FieldExtractor::default();
        for n in 0..PACKET_LEN {
            assert_eq!(
                extractor.extract(&vec![0u8; n]),
                None,
                "gói {} byte phải bị từ chối",
                n
            );
        }
        assert_eq!(extractor.packets_rejected, PACKET_LEN as u64);
    }

    #[test]
    fn a_bad_checksum_marks_the_packet_invalid() {
        let mut extractor = FieldExtractor::default();
        let mut g = valid_packet(b'A', 1, 100, 10);
        g[19] ^= 0xFF; // phá tổng kiểm tra
        let t = extractor.extract(&g).unwrap();
        assert!(
            !t.is_valid,
            "gói hỏng phải bị đánh dấu, KHÔNG được im lặng cho qua"
        );
        assert_eq!(extractor.packets_rejected, 1);
        assert_eq!(extractor.packets_parsed, 0);
    }

    #[test]
    fn a_single_bit_flip_in_the_body_is_caught() {
        let mut extractor = FieldExtractor::default();
        for index in 0..17usize {
            let mut g = valid_packet(b'A', 999, 8_400, 500);
            g[index] ^= 1;
            let t = extractor.extract(&g).unwrap();
            assert!(
                !t.is_valid,
                "lật bit ở byte {} mà không bị phát hiện",
                index
            );
        }
    }

    #[test]
    fn extraction_always_costs_exactly_one_cycle() {
        let extractor = FieldExtractor::default();
        assert_eq!(
            extractor.extract_cycles(),
            1,
            "phần cứng tách mọi trường song song"
        );
    }

    // ---------- Sổ lệnh phần cứng ----------
    #[test]
    fn an_empty_book_has_no_best_price() {
        let s = HwOrderBook::default();
        assert_eq!(s.best_bid(), None);
        assert_eq!(s.best_ask(), None);
        assert_eq!(s.spread(), None);
    }

    #[test]
    fn reports_best_on_both_sides() {
        let mut s = HwOrderBook::default();
        for (g, qty) in [(8_380i64, 100u32), (8_400, 500), (8_390, 300)] {
            s.update(true, g, qty);
        }
        for (g, qty) in [(8_430i64, 100u32), (8_410, 400), (8_420, 250)] {
            s.update(false, g, qty);
        }
        assert_eq!(
            s.best_bid().unwrap().price,
            8_400,
            "bên mua lấy giá CAO nhất"
        );
        assert_eq!(
            s.best_ask().unwrap().price,
            8_410,
            "bên bán lấy giá THẤP nhất"
        );
        assert_eq!(s.spread(), Some(10));
    }

    #[test]
    fn updating_an_existing_level_overwrites_its_size() {
        let mut s = HwOrderBook::default();
        s.update(true, 8_400, 500);
        s.update(true, 8_400, 700);
        assert_eq!(s.levels_in_use(true), 1, "không được tạo mức trùng");
        assert_eq!(s.best_bid().unwrap().quantity, 700);
    }

    #[test]
    fn zeroing_the_size_removes_the_level() {
        let mut s = HwOrderBook::default();
        s.update(true, 8_400, 500);
        s.update(true, 8_390, 300);
        s.update(true, 8_400, 0);
        assert_eq!(
            s.best_bid().unwrap().price,
            8_390,
            "đỉnh phải tụt xuống mức kế"
        );
        assert_eq!(s.levels_in_use(true), 1);
    }

    #[test]
    fn a_full_book_keeps_the_best_levels() {
        // Sổ phần cứng chỉ có 8 ô. Khi đầy, mức tệ nhất phải bị đẩy ra —
        // nếu không, ta sẽ giữ những mức giá vô dụng và bỏ mất mức tốt.
        let mut s = HwOrderBook::default();
        for i in 0..HW_LEVELS as i64 {
            s.update(true, 8_000 + i, 100);
        }
        assert_eq!(s.levels_in_use(true), HW_LEVELS);
        assert_eq!(s.best_bid().unwrap().price, 8_007);
        // Mức tốt hơn hẳn → phải chen vào được
        s.update(true, 9_000, 100);
        assert_eq!(s.best_bid().unwrap().price, 9_000);
        assert_eq!(s.levels_in_use(true), HW_LEVELS, "vẫn đúng 8 ô");
        // Mức tệ hơn tất cả → phải bị bỏ qua
        s.update(true, 1, 100);
        assert!(
            s.bids.iter().all(|m| m.price != 1),
            "mức tệ không được chiếm chỗ"
        );
    }

    #[test]
    fn the_ask_side_also_keeps_its_best_levels() {
        let mut s = HwOrderBook::default();
        for i in 0..HW_LEVELS as i64 {
            s.update(false, 9_000 - i, 100);
        }
        assert_eq!(s.best_ask().unwrap().price, 8_993);
        s.update(false, 8_000, 100); // rẻ hơn hẳn = tốt hơn cho bên bán
        assert_eq!(s.best_ask().unwrap().price, 8_000);
        s.update(false, 99_999, 100); // đắt vô lý
        assert!(s.asks.iter().all(|m| m.price != 99_999));
    }

    #[test]
    fn comparator_depth_is_logarithmic_in_levels() {
        assert_eq!(
            HwOrderBook::comparator_depth(),
            3,
            "8 mức → 3 tầng cây so sánh"
        );
    }

    // ---------- Mạch rủi ro ----------
    fn circuit() -> RiskCircuit {
        RiskCircuit {
            max_value: 1_000_000,
            max_position: 500,
            position: 0,
            kill_switch: false,
        }
    }

    #[test]
    fn a_valid_order_raises_no_flag() {
        let c = circuit().check(true, 8_400, 100);
        assert!(!c.is_blocked());
        assert_eq!(c.count_raised(), 0);
    }

    #[test]
    fn each_condition_raises_its_own_flag() {
        let m = circuit();
        assert!(m.check(true, 8_400, 0).invalid_quantity);
        assert!(m.check(true, 0, 100).invalid_price);
        assert!(m.check(true, 8_400, 1_000).exceed_value);
        assert!(m.check(true, 100, 600).exceed_position);
        let killed = RiskCircuit {
            kill_switch: true,
            ..m
        };
        assert!(killed.check(true, 8_400, 100).kill_switch);
    }

    #[test]
    fn multiple_violations_raise_multiple_flags() {
        // Đây là điểm khác biệt thật so với phần mềm: phần mềm `return` ở lỗi
        // ĐẦU TIÊN nên chỉ biết một lỗi; mạch tính song song nên thấy HẾT.
        let c = circuit().check(true, 0, -1);
        assert!(c.invalid_quantity && c.invalid_price);
        assert!(
            c.count_raised() >= 2,
            "phần cứng thấy mọi lỗi cùng lúc, không dừng ở lỗi đầu"
        );
    }

    #[test]
    fn the_short_side_is_bounded_by_the_position_limit_too() {
        let m = circuit();
        assert!(
            m.check(false, 100, 600).exceed_position,
            "chiều bán cũng phải bị chặn"
        );
    }

    #[test]
    fn current_position_is_counted_in() {
        let m = RiskCircuit {
            position: 450,
            ..circuit()
        };
        assert!(
            !m.check(true, 100, 50).exceed_position,
            "450+50 = 500, vừa trần"
        );
        assert!(m.check(true, 100, 51).exceed_position, "450+51 vượt trần");
        assert!(
            !m.check(false, 100, 500).exceed_position,
            "bán thì giảm vị thế"
        );
    }

    #[test]
    fn the_multiply_never_overflows() {
        let m = circuit();
        // Toàn bộ dùng phép bão hoà: không được panic, và phải báo vượt hạn mức
        let c = m.check(true, i64::MAX, i64::MAX);
        assert!(c.exceed_value);
        assert!(c.is_blocked());
        let c2 = m.check(false, 1, i64::MAX);
        assert!(c2.is_blocked());
    }

    #[test]
    fn the_check_always_costs_exactly_one_cycle() {
        // Bất biến quan trọng nhất của mạch rủi ro: thời gian KHÔNG phụ thuộc
        // dữ liệu. Nhờ vậy độ trễ không dao động và không rò rỉ thông tin.
        let m = circuit();
        assert_eq!(m.check_cycles(), 1);
        for (g, qty) in [(8_400i64, 100i64), (0, 0), (-1, -1), (i64::MAX, i64::MAX)] {
            let _ = m.check(true, g, qty);
            assert_eq!(m.check_cycles(), 1, "mọi đầu vào đều tốn đúng 1 chu kỳ");
        }
    }

    // ---------- Đường ống ----------
    #[test]
    fn latency_is_the_sum_of_the_stages() {
        let side_levels = HwPipeline::typical();
        assert_eq!(side_levels.latency_cycles(), 3 + 1 + 1 + 2 + 1 + 2 + 3);
        assert!((side_levels.latency_nanos() - 13.0 * NS_PER_CYCLE).abs() < 1e-9);
    }

    #[test]
    fn throughput_is_set_by_the_slowest_stage_not_the_sum() {
        // Nhầm hai đại lượng này là hiểu sai toàn bộ kiến trúc đường ống.
        let side_levels = HwPipeline::typical();
        assert_eq!(
            side_levels.initiation_interval(),
            3,
            "tầng chậm nhất là 3 chu kỳ"
        );
        assert!(side_levels.initiation_interval() < side_levels.latency_cycles());
    }

    #[test]
    fn pipelining_raises_throughput_without_cutting_latency() {
        let side_levels = HwPipeline::typical();
        // Một gói: y hệt nhau
        assert_eq!(
            side_levels.total_cycles_pipelined(1),
            side_levels.latency_cycles()
        );
        assert_eq!(
            side_levels.total_cycles_no_pipeline(1),
            side_levels.latency_cycles()
        );
        // Nhiều gói: đường ống thắng đậm
        assert!(
            side_levels.total_cycles_pipelined(1_000) * 4
                < side_levels.total_cycles_no_pipeline(1_000)
        );
        // Nhưng độ trễ của MỘT gói vẫn y nguyên
        assert_eq!(side_levels.latency_cycles(), 13);
    }

    #[test]
    fn no_packets_means_no_cycles() {
        let side_levels = HwPipeline::typical();
        assert_eq!(side_levels.total_cycles_pipelined(0), 0);
        assert_eq!(side_levels.total_cycles_no_pipeline(0), 0);
    }

    #[test]
    fn hardware_beats_software_by_an_order_of_magnitude() {
        let side_levels = HwPipeline::typical();
        let ratio = software_latency_ns() / side_levels.latency_nanos();
        assert!(
            ratio > 50.0,
            "phải nhanh hơn ít nhất 50 lần, thực tế {:.0}",
            ratio
        );
        assert!(
            side_levels.latency_nanos() < 100.0,
            "tick-to-trade phải dưới 100 ns"
        );
    }

    #[test]
    fn throughput_reaches_hundreds_of_millions_of_packets() {
        let side_levels = HwPipeline::typical();
        assert!(
            side_levels.packets_per_second() > 50e6,
            "phải trên 50 triệu gói/giây, thực tế {:.0}",
            side_levels.packets_per_second()
        );
    }

    // ---------- Phân công phần cứng/phần mềm ----------
    #[test]
    fn hot_and_stable_work_belongs_in_hardware() {
        let c = Feature {
            name: "tách gói".into(),
            change_frequency: 1,
            on_hot_path: true,
        };
        assert_eq!(assign_unit(&c), ExecutionUnit::Hardware);
    }

    #[test]
    fn volatile_work_stays_in_software_even_if_hot() {
        // Bài học kiến trúc quan trọng nhất của chương: tốc độ không đáng giá
        // bằng khả năng thay đổi. Chiến lược sửa 200 lần/năm mà nằm trên FPGA
        // thì mỗi lần thử nghiệm tốn hàng chục phút tổng hợp mạch.
        let c = Feature {
            name: "chiến lược".into(),
            change_frequency: 200,
            on_hot_path: true,
        };
        assert_eq!(assign_unit(&c), ExecutionUnit::Software);
    }

    #[test]
    fn cold_work_stays_in_software_even_if_stable() {
        let c = Feature {
            name: "báo cáo".into(),
            change_frequency: 1,
            on_hot_path: false,
        };
        assert_eq!(
            assign_unit(&c),
            ExecutionUnit::Software,
            "không nằm trên đường nóng thì đưa xuống phần cứng là lãng phí"
        );
    }

    #[test]
    fn cycles_convert_to_nanoseconds_correctly() {
        assert!((cycles_to_ns(1) - 4.0).abs() < 1e-9);
        assert!(
            (cycles_to_ns(250) - 1_000.0).abs() < 1e-9,
            "250 chu kỳ ở 250 MHz = 1 µs"
        );
        assert_eq!(cycles_to_ns(0), 0.0);
    }
}
