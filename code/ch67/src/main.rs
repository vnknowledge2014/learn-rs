//! Chương 67 — FPGA & Thiết kế phần cứng số bằng Rust: cổng logic, mạch tổ hợp,
//! mạch tuần tự có xung nhịp, đường ống, và vì sao phần cứng nhanh hơn phần mềm.
//!
//! Tinh thần lấy từ rust-hdl (nay đang được tác giả viết lại thành `rhdl`):
//! mô tả phần cứng bằng KIỂU của Rust, mô phỏng ngay trong `cargo test`,
//! rồi mới sinh Verilog. Sai thiết kế bị bắt lúc biên dịch, không phải sau
//! 40 phút tổng hợp mạch.

use std::collections::HashMap;

// ============================================================================
// 1. TÍN HIỆU & CỔNG LOGIC — vật liệu xây dựng duy nhất
// ============================================================================

/// Trong FPGA thật, tín hiệu còn có trạng thái 'X' (không xác định) và 'Z'
/// (trở kháng cao). Ta mô hình hóa cả 'X' vì nó là nguồn lỗi kinh điển:
/// quên khởi tạo thanh ghi → mạch chạy đúng trong mô phỏng, sai trên chip.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum Signal {
    Low,
    High,
    /// Không xác định — chưa reset, dây chưa nối, hoặc hai nguồn cùng lái.
    X,
}

impl Signal {
    pub fn from_bool(b: bool) -> Signal {
        if b { Signal::High } else { Signal::Low }
    }
    pub fn to_bool(self) -> Option<bool> {
        match self {
            Signal::High => Some(true),
            Signal::Low => Some(false),
            _ => None,
        }
    }
}

pub fn not_gate(a: Signal) -> Signal {
    match a {
        Signal::High => Signal::Low,
        Signal::Low => Signal::High,
        x => x,
    }
}
pub fn and_gate(a: Signal, b: Signal) -> Signal {
    // Lưu ý: 0 AND X = 0, KHÔNG phải X — vì kết quả đã xác định dù X là gì.
    // Đây gọi là "làm ngắn mạch giá trị điều khiển" và có thật trên silicon.
    match (a, b) {
        (Signal::Low, _) | (_, Signal::Low) => Signal::Low,
        (Signal::High, Signal::High) => Signal::High,
        _ => Signal::X,
    }
}
pub fn or_gate(a: Signal, b: Signal) -> Signal {
    match (a, b) {
        (Signal::High, _) | (_, Signal::High) => Signal::High,
        (Signal::Low, Signal::Low) => Signal::Low,
        _ => Signal::X,
    }
}
pub fn xor_gate(a: Signal, b: Signal) -> Signal {
    match (a.to_bool(), b.to_bool()) {
        (Some(x), Some(y)) => Signal::from_bool(x ^ y),
        _ => Signal::X, // XOR KHÔNG có giá trị điều khiển
    }
}
/// NAND là cổng "phổ dụng": mọi hàm logic đều dựng được chỉ từ NAND.
pub fn nand_gate(a: Signal, b: Signal) -> Signal {
    not_gate(and_gate(a, b))
}

/// Bộ chọn kênh 2-1 — viên gạch của mọi thứ có chữ "if" trong phần cứng.
pub fn mux2(sel: Signal, when0: Signal, when1: Signal) -> Signal {
    or_gate(and_gate(not_gate(sel), when0), and_gate(sel, when1))
}

// ============================================================================
// 2. MẠCH TỔ HỢP — đầu ra chỉ phụ thuộc đầu vào HIỆN TẠI
// ============================================================================

/// Bộ cộng bán phần: cộng 2 bit, cho tổng và nhớ.
pub fn half_adder(a: Signal, b: Signal) -> (Signal, Signal) {
    (xor_gate(a, b), and_gate(a, b))
}

/// Bộ cộng toàn phần: cộng 2 bit CỘNG bit nhớ vào.
pub fn full_adder(a: Signal, b: Signal, carry_in: Signal) -> (Signal, Signal) {
    let (t1, n1) = half_adder(a, b);
    let (sum, n2) = half_adder(t1, carry_in);
    (sum, or_gate(n1, n2))
}

#[derive(Debug, PartialEq)]
pub struct AdderResult {
    pub sum: u16,
    pub overflow: bool,
    /// Số tầng cổng mà tín hiệu phải đi qua — quyết định TẦN SỐ TỐI ĐA của mạch.
    pub gate_depth: usize,
}

/// Bộ cộng nhớ nối tiếp 8 bit — cách dựng đơn giản nhất, và CHẬM nhất.
/// Bit nhớ phải "chảy" tuần tự qua cả 8 tầng: độ trễ tỉ lệ THUẬN với số bit.
pub fn ripple_adder_8bit(a: u8, b: u8) -> AdderResult {
    let mut carry = Signal::Low;
    let mut sum = 0u16;
    for i in 0..8 {
        let bit_a = Signal::from_bool((a >> i) & 1 == 1);
        let bit_b = Signal::from_bool((b >> i) & 1 == 1);
        let (s, c) = full_adder(bit_a, bit_b, carry);
        if s == Signal::High {
            sum |= 1 << i;
        }
        carry = c;
    }
    AdderResult {
        sum,
        overflow: carry == Signal::High,
        gate_depth: 8 * 3, // mỗi bộ cộng toàn phần ~3 tầng cổng, nối tiếp nhau
    }
}

/// Bộ cộng nhìn trước nhớ (carry-lookahead): tính TẤT CẢ bit nhớ SONG SONG
/// từ hai tín hiệu "sinh nhớ" (G = a·b) và "truyền nhớ" (P = a⊕b).
/// Cùng kết quả, nhưng độ sâu chỉ còn ~log(n) thay vì n. Đây là bài học
/// cốt lõi của phần cứng: ĐÁNH ĐỔI DIỆN TÍCH LẤY TỐC ĐỘ.
pub fn lookahead_adder_8bit(a: u8, b: u8) -> AdderResult {
    let g = a & b; // sinh nhớ
    let p = a ^ b; // truyền nhớ
    let mut carry = [false; 9];
    for i in 0..8 {
        // c[i+1] = G[i] + P[i]·c[i] — trong phần cứng, khai triển hết thành
        // một biểu thức phẳng nên tính đồng thời chỉ trong vài tầng cổng.
        carry[i + 1] = ((g >> i) & 1 == 1) || (((p >> i) & 1 == 1) && carry[i]);
    }
    let mut sum = 0u16;
    for (i, &c) in carry[..8].iter().enumerate() {
        if ((p >> i) & 1 == 1) ^ c {
            sum |= 1 << i;
        }
    }
    AdderResult {
        sum,
        overflow: carry[8],
        gate_depth: 5, // ~log2(8) + vài tầng
    }
}

// ============================================================================
// 3. MẠCH TUẦN TỰ — có xung nhịp và TRÍ NHỚ
// ============================================================================

/// Flip-flop D: viên gạch của mọi trí nhớ trong FPGA.
/// Ở MỖI sườn lên của xung nhịp, chốt lấy giá trị đầu vào; giữa hai sườn thì
/// giữ nguyên bất kể đầu vào đổi thế nào.
#[derive(Debug, Clone, Copy, PartialEq)]
pub struct FlipFlopD {
    q: Signal,
}

impl Default for FlipFlopD {
    fn default() -> Self {
        Self::new()
    }
}

impl FlipFlopD {
    /// Chưa reset thì giá trị là KHÔNG XÁC ĐỊNH — đúng như silicon thật.
    pub fn new() -> Self {
        FlipFlopD { q: Signal::X }
    }
    pub fn q(&self) -> Signal {
        self.q
    }
    pub fn rising_edge(&mut self, d: Signal) {
        self.q = d;
    }
    pub fn reset(&mut self) {
        self.q = Signal::Low;
    }
}

/// Thanh ghi dịch — dùng cho SPI, UART, tính CRC, tạo số giả ngẫu nhiên.
pub struct ShiftRegister<const N: usize> {
    flops: [FlipFlopD; N],
}

impl<const N: usize> Default for ShiftRegister<N> {
    fn default() -> Self {
        Self::new()
    }
}

impl<const N: usize> ShiftRegister<N> {
    pub fn new() -> Self {
        ShiftRegister {
            flops: [FlipFlopD::new(); N],
        }
    }
    pub fn reset(&mut self) {
        for f in self.flops.iter_mut() {
            f.reset();
        }
    }
    /// Đẩy 1 bit vào đầu, bit ở cuối rơi ra. Toàn bộ N flip-flop cập nhật
    /// ĐỒNG THỜI trong một chu kỳ — không có vòng lặp nào chạy trên chip.
    ///
    /// Chú ý vòng lặp chạy NGƯỢC (`(1..N).rev()`): phải chép từ cuối về đầu,
    /// nếu không giá trị mới của flops[i-1] sẽ đè lên giá trị cũ mà flops[i] cần đọc.
    /// Lỗi này khiến cả thanh ghi biến thành một flip-flop duy nhất.
    ///
    /// Đầu ra được lấy SAU sườn xung — đúng như Q của flip-flop cuối đổi
    /// giá trị ngay tại sườn. Đọc trước sườn sẽ trễ một chu kỳ; đây là lỗi
    /// lệch-một kinh điển khi viết mô phỏng HDL.
    pub fn rising_edge(&mut self, input: Signal) -> Signal {
        for i in (1..N).rev() {
            let prev = self.flops[i - 1].q();
            self.flops[i].rising_edge(prev);
        }
        self.flops[0].rising_edge(input);
        self.flops[N - 1].q()
    }
    pub fn contents(&self) -> Vec<Signal> {
        self.flops.iter().map(|f| f.q()).collect()
    }
}

/// Máy trạng thái hữu hạn có xung nhịp — đèn giao thông.
/// Đây là dạng mạch mà FPGA làm tốt nhất: điều khiển tất định, độ trễ đếm được.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum TrafficLight {
    Red,
    RedAmber,
    Green,
    Amber,
}

pub struct TrafficController {
    pub state: TrafficLight,
    pub counter: u8,
    pub durations: [u8; 4],
}

impl Default for TrafficController {
    fn default() -> Self {
        Self::new()
    }
}

impl TrafficController {
    pub fn new() -> Self {
        TrafficController {
            state: TrafficLight::Red,
            counter: 0,
            durations: [5, 1, 4, 2],
        }
    }
    fn phase_index(&self) -> usize {
        match self.state {
            TrafficLight::Red => 0,
            TrafficLight::RedAmber => 1,
            TrafficLight::Green => 2,
            TrafficLight::Amber => 3,
        }
    }
    /// Một sườn xung nhịp. Toàn bộ logic là TỔ HỢP, chỉ `state` và
    /// `counter` nằm trong flip-flop — đây là mẫu "logic tách khỏi thanh ghi".
    pub fn rising_edge(&mut self) -> TrafficLight {
        self.counter += 1;
        if self.counter >= self.durations[self.phase_index()] {
            self.counter = 0;
            self.state = match self.state {
                TrafficLight::Red => TrafficLight::RedAmber,
                TrafficLight::RedAmber => TrafficLight::Green,
                TrafficLight::Green => TrafficLight::Amber,
                TrafficLight::Amber => TrafficLight::Red,
            };
        }
        self.state
    }
    /// Ràng buộc AN TOÀN: không bao giờ được nhảy thẳng Green → Đỏ.
    pub fn is_legal_transition(from: TrafficLight, to: TrafficLight) -> bool {
        use TrafficLight::*;
        matches!(
            (from, to),
            (Red, Red)
                | (Red, RedAmber)
                | (RedAmber, RedAmber)
                | (RedAmber, Green)
                | (Green, Green)
                | (Green, Amber)
                | (Amber, Amber)
                | (Amber, Red)
        )
    }
}

// ============================================================================
// 4. ĐƯỜNG ỐNG (pipeline) — bí quyết tăng thông lượng của mọi CPU/GPU
// ============================================================================

#[derive(Debug, PartialEq)]
pub struct PipelineResult {
    pub output: Vec<u32>,
    pub num_cycles: usize,
    /// Độ trễ: bao nhiêu chu kỳ từ lúc nạp đến lúc có kết quả ĐẦU TIÊN.
    pub latency: usize,
}

/// Không đường ống: mỗi phần tử phải đi hết `stages` giai đoạn rồi mới
/// nạp phần tử kế. Thông lượng = 1 kết quả / `stages` chu kỳ.
pub fn run_unpipelined(input: &[u32], stages: usize, f: impl Fn(u32) -> u32) -> PipelineResult {
    let output: Vec<u32> = input.iter().map(|&x| f(x)).collect();
    PipelineResult {
        num_cycles: input.len() * stages,
        latency: stages,
        output,
    }
}

/// Có đường ống: mỗi tầng có thanh ghi riêng, nên `stages` phần tử được xử lý
/// ĐỒNG THỜI ở các giai đoạn khác nhau. Sau khi ống đầy: 1 kết quả MỖI chu kỳ.
pub fn run_pipelined(input: &[u32], stages: usize, f: impl Fn(u32) -> u32) -> PipelineResult {
    let mut regs: Vec<Option<u32>> = vec![None; stages]; // thanh ghi giữa các tầng
    let mut output = Vec::new();
    let mut next_input = 0;
    let mut cycle = 0;
    let mut latency = 0;

    while output.len() < input.len() {
        // Dịch từ CUỐI về ĐẦU để không ghi đè dữ liệu chưa dùng —
        // giống hệt cách thanh ghi thật cập nhật đồng thời trên sườn xung.
        if let Some(v) = regs[stages - 1] {
            if output.is_empty() {
                latency = cycle; // ĐO độ trễ: số chu kỳ tới kết quả đầu tiên
            }
            output.push(v);
        }
        for i in (1..stages).rev() {
            regs[i] = regs[i - 1];
        }
        regs[0] = if next_input < input.len() {
            let v = f(input[next_input]);
            next_input += 1;
            Some(v)
        } else {
            None
        };
        cycle += 1;
    }
    PipelineResult {
        output,
        num_cycles: cycle,
        latency,
    }
}

// ============================================================================
// 5. NETLIST — mô tả mạch dưới dạng đồ thị, rồi mô phỏng
// ============================================================================

#[derive(Debug, Clone)]
pub enum Node {
    Input(String),
    Not(usize),
    And(usize, usize),
    Or(usize, usize),
    Xor(usize, usize),
}

/// Danh sách nối (netlist) chính là thứ trình tổng hợp sinh ra từ HDL,
/// và cũng là thứ được nạp xuống FPGA.
#[derive(Default)]
pub struct Circuit {
    pub nodes: Vec<Node>,
}

impl Circuit {
    pub fn new() -> Self {
        Self::default()
    }
    pub fn add(&mut self, n: Node) -> usize {
        self.nodes.push(n);
        self.nodes.len() - 1
    }

    /// Mô phỏng: vì netlist là đồ thị không chu trình, tính lần lượt theo
    /// thứ tự thêm vào là đủ — đó chính là "sắp xếp tô-pô" miễn phí.
    pub fn simulate(&self, input: &HashMap<String, Signal>) -> Vec<Signal> {
        let mut values = vec![Signal::X; self.nodes.len()];
        for (i, n) in self.nodes.iter().enumerate() {
            values[i] = match n {
                Node::Input(name) => *input.get(name).unwrap_or(&Signal::X),
                Node::Not(a) => not_gate(values[*a]),
                Node::And(a, b) => and_gate(values[*a], values[*b]),
                Node::Or(a, b) => or_gate(values[*a], values[*b]),
                Node::Xor(a, b) => xor_gate(values[*a], values[*b]),
            };
        }
        values
    }

    /// Đường tới hạn: chuỗi cổng DÀI NHẤT từ đầu vào tới đầu ra.
    /// Tần số tối đa của mạch = 1 / (độ trễ đường tới hạn).
    pub fn critical_path(&self) -> usize {
        let mut depth = vec![0usize; self.nodes.len()];
        for (i, n) in self.nodes.iter().enumerate() {
            depth[i] = match n {
                Node::Input(_) => 0,
                Node::Not(a) => depth[*a] + 1,
                Node::And(a, b) | Node::Or(a, b) | Node::Xor(a, b) => depth[*a].max(depth[*b]) + 1,
            };
        }
        depth.into_iter().max().unwrap_or(0)
    }
}

fn main() {
    println!("═══════════════════════════════════════════════════════════");
    println!("   FPGA: CỔNG LOGIC · BỘ CỘNG · FLIP-FLOP · ĐƯỜNG ỐNG       ");
    println!("═══════════════════════════════════════════════════════════");

    println!("\n1. BẢNG CHÂN TRỊ CÓ TRẠNG THÁI 'X'");
    println!(
        "   0 AND X = {:?}  ← đã xác định! (0 là giá trị điều khiển của AND)",
        and_gate(Signal::Low, Signal::X)
    );
    println!("   1 AND X = {:?}", and_gate(Signal::High, Signal::X));
    println!(
        "   0 XOR X = {:?}  ← XOR không có giá trị điều khiển",
        xor_gate(Signal::Low, Signal::X)
    );

    println!("\n2. HAI CÁCH DỰNG BỘ CỘNG 8 BIT — cùng kết quả, khác tốc độ");
    for (a, b) in [(200u8, 100u8), (255, 1), (37, 91)] {
        let rc = ripple_adder_8bit(a, b);
        let la = lookahead_adder_8bit(a, b);
        println!(
            "   {:>3} + {:>3} = {:>3} (tràn {}) | nối tiếp {} tầng · nhìn trước {} tầng",
            a, b, rc.sum, rc.overflow, rc.gate_depth, la.gate_depth
        );
        assert_eq!(rc.sum, la.sum);
    }
    println!(
        "   → Cùng đáp số, nhưng mạch nhìn trước chạy nhanh hơn ~{}×",
        ripple_adder_8bit(0, 0).gate_depth / lookahead_adder_8bit(0, 0).gate_depth
    );

    println!("\n3. THANH GHI DỊCH 4 BIT");
    let mut sr: ShiftRegister<4> = ShiftRegister::new();
    sr.reset();
    print!("   Đẩy 1,0,1,1 → ra: ");
    for v in [true, false, true, true] {
        print!("{:?} ", sr.rising_edge(Signal::from_bool(v)));
    }
    println!("\n   Nội dung sau 4 chu kỳ: {:?}", sr.contents());

    println!("\n4. MÁY TRẠNG THÁI ĐÈN GIAO THÔNG (mỗi ký tự = 1 chu kỳ nhịp)");
    let mut light = TrafficController::new();
    let text: String = (0..24)
        .map(|_| match light.rising_edge() {
            TrafficLight::Red => 'Đ',
            TrafficLight::RedAmber => 'v',
            TrafficLight::Green => 'X',
            TrafficLight::Amber => 'V',
        })
        .collect();
    println!("   {}", text);
    println!(
        "   Không bao giờ có 'XĐ' (xanh nhảy thẳng sang đỏ): {}",
        !text.contains("XĐ")
    );

    println!("\n5. ĐƯỜNG ỐNG — 100 phần tử qua mạch 5 tầng");
    let input: Vec<u32> = (0..100).collect();
    let plain = run_unpipelined(&input, 5, |x| x * x);
    let piped = run_pipelined(&input, 5, |x| x * x);
    println!(
        "   Không ống: {} chu kỳ (độ trễ {})",
        plain.num_cycles, plain.latency
    );
    println!(
        "   Có ống   : {} chu kỳ (độ trễ {}) → nhanh gấp {:.1}×",
        piped.num_cycles,
        piped.latency,
        plain.num_cycles as f64 / piped.num_cycles as f64
    );
    println!("   → Độ trễ KHÔNG giảm; chỉ THÔNG LƯỢNG tăng. Hai đại lượng khác nhau.");

    println!("\n6. NETLIST & ĐƯỜNG TỚI HẠN");
    let mut circuit = Circuit::new();
    let a = circuit.add(Node::Input("a".into()));
    let b = circuit.add(Node::Input("b".into()));
    let c = circuit.add(Node::Input("c".into()));
    let x = circuit.add(Node::Xor(a, b));
    let y = circuit.add(Node::Xor(x, c)); // tổng của bộ cộng toàn phần
    let _ = y;
    let mut inputs = HashMap::new();
    for (k, v) in [("a", true), ("b", true), ("c", false)] {
        inputs.insert(k.to_string(), Signal::from_bool(v));
    }
    println!("   1 XOR 1 XOR 0 = {:?}", circuit.simulate(&inputs)[y]);
    println!("   Đường tới hạn = {} tầng cổng", circuit.critical_path());

    println!("\n═══════════════════════════════════════════════════════════");
    println!("   PHẦN MỀM SONG SONG THEO THỜI GIAN — PHẦN CỨNG THEO KHÔNG GIAN");
    println!("═══════════════════════════════════════════════════════════");
}

#[cfg(test)]
mod tests {
    use super::*;
    use Signal::{High, Low, X};

    // ---------- Cổng logic ----------
    #[test]
    fn controlling_value_erases_x_state() {
        // Bài học phần cứng thật: 0·X = 0 và 1+X = 1, dù X là gì đi nữa.
        assert_eq!(and_gate(Low, X), Low);
        assert_eq!(and_gate(X, Low), Low);
        assert_eq!(or_gate(High, X), High);
        // nhưng khi không có giá trị điều khiển thì X lan ra
        assert_eq!(and_gate(High, X), X);
        assert_eq!(xor_gate(Low, X), X);
    }

    #[test]
    fn nand_is_universal() {
        // Dựng NOT, AND, OR chỉ từ NAND — nền tảng của mọi thư viện cổng.
        let not = |a| nand_gate(a, a);
        let and = |a, b| not(nand_gate(a, b));
        let or = |a, b| nand_gate(not(a), not(b));
        for a in [Low, High] {
            assert_eq!(not(a), not_gate(a));
            for b in [Low, High] {
                assert_eq!(and(a, b), and_gate(a, b));
                assert_eq!(or(a, b), or_gate(a, b));
            }
        }
    }

    #[test]
    fn mux_behaves_like_an_if() {
        assert_eq!(mux2(Low, High, Low), High, "chọn=0 → lấy nhánh 0");
        assert_eq!(mux2(High, High, Low), Low, "chọn=1 → lấy nhánh 1");
    }

    #[test]
    fn de_morgan_holds_on_gates() {
        for a in [Low, High] {
            for b in [Low, High] {
                assert_eq!(not_gate(and_gate(a, b)), or_gate(not_gate(a), not_gate(b)));
                assert_eq!(not_gate(or_gate(a, b)), and_gate(not_gate(a), not_gate(b)));
            }
        }
    }

    // ---------- Bộ cộng ----------
    #[test]
    fn full_adder_correct_for_all_eight_inputs() {
        for a in [false, true] {
            for b in [false, true] {
                for c in [false, true] {
                    let (t, n) = full_adder(
                        Signal::from_bool(a),
                        Signal::from_bool(b),
                        Signal::from_bool(c),
                    );
                    let total = a as u8 + b as u8 + c as u8;
                    assert_eq!(t.to_bool(), Some(total & 1 == 1));
                    assert_eq!(n.to_bool(), Some(total >= 2));
                }
            }
        }
    }

    #[test]
    fn adder_8bit_matches_machine_arithmetic() {
        // Kiểm thử vét cạn TOÀN BỘ 65 536 tổ hợp — điều bất khả với mạch lớn,
        // nhưng với 8 bit thì đây là chứng minh tuyệt đối.
        for a in 0u16..256 {
            for b in 0u16..256 {
                let r = ripple_adder_8bit(a as u8, b as u8);
                let exact = a + b;
                assert_eq!(r.sum, exact & 0xFF, "{a}+{b}");
                assert_eq!(r.overflow, exact > 255, "{a}+{b} phải báo tràn");
            }
        }
    }

    #[test]
    fn both_adder_designs_agree() {
        for a in 0u16..256 {
            for b in 0u16..256 {
                let rc = ripple_adder_8bit(a as u8, b as u8);
                let la = lookahead_adder_8bit(a as u8, b as u8);
                assert_eq!(
                    (rc.sum, rc.overflow),
                    (la.sum, la.overflow),
                    "hai kiến trúc phải tương đương về CHỨC NĂNG: {a}+{b}"
                );
            }
        }
    }

    #[test]
    fn lookahead_is_shallower_than_ripple() {
        // Đây là toàn bộ lý do người ta chịu tốn thêm cổng cho carry-lookahead.
        assert!(lookahead_adder_8bit(0, 0).gate_depth < ripple_adder_8bit(0, 0).gate_depth);
    }

    // ---------- Mạch tuần tự ----------
    #[test]
    fn flip_flop_is_undefined_before_reset() {
        let f = FlipFlopD::new();
        assert_eq!(
            f.q(),
            X,
            "silicon thật cũng vậy — phải reset trước khi dùng"
        );
    }

    #[test]
    fn flip_flop_latches_on_rising_edge() {
        let mut f = FlipFlopD::new();
        f.reset();
        assert_eq!(f.q(), Low);
        f.rising_edge(High);
        assert_eq!(f.q(), High);
    }

    #[test]
    fn shift_register_delays_by_n_cycles() {
        let mut sr: ShiftRegister<4> = ShiftRegister::new();
        sr.reset();
        // Bit đầu tiên phải mất ĐÚNG N = 4 chu kỳ mới ra tới đầu kia.
        // Đây chính là độ trễ của thanh ghi dịch — nền của SPI và UART.
        assert_eq!(sr.rising_edge(High), Low);
        assert_eq!(sr.rising_edge(Low), Low);
        assert_eq!(sr.rising_edge(Low), Low);
        assert_eq!(
            sr.rising_edge(Low),
            High,
            "bit '1' xuất hiện đúng ở chu kỳ thứ 4"
        );
        assert_eq!(sr.rising_edge(Low), Low, "sau đó ống rỗng trở lại");
    }

    #[test]
    fn traffic_light_never_jumps_green_to_red() {
        let mut d = TrafficController::new();
        let mut prev = d.state;
        for _ in 0..200 {
            let now = d.rising_edge();
            assert!(
                TrafficController::is_legal_transition(prev, now),
                "chuyển trái phép {:?} → {:?}",
                prev,
                now
            );
            prev = now;
        }
    }

    #[test]
    fn traffic_light_cycles_and_repeats() {
        let mut d = TrafficController::new();
        let period: u32 = d.durations.iter().map(|&x| x as u32).sum();
        let one_round: Vec<TrafficLight> = (0..period).map(|_| d.rising_edge()).collect();
        let round_two: Vec<TrafficLight> = (0..period).map(|_| d.rising_edge()).collect();
        assert_eq!(
            one_round, round_two,
            "máy trạng thái phải tuần hoàn đúng chu kỳ"
        );
        // và ghé qua đủ cả 4 trạng thái
        for state in [
            TrafficLight::Red,
            TrafficLight::RedAmber,
            TrafficLight::Green,
            TrafficLight::Amber,
        ] {
            assert!(one_round.contains(&state), "thiếu trạng thái {:?}", state);
        }
    }

    // ---------- Đường ống ----------
    #[test]
    fn pipeline_same_result_much_faster() {
        let input: Vec<u32> = (1..=50).collect();
        let plain = run_unpipelined(&input, 5, |x| x * 3);
        let piped = run_pipelined(&input, 5, |x| x * 3);
        assert_eq!(
            plain.output, piped.output,
            "đường ống không được đổi KẾT QUẢ"
        );
        assert!(piped.num_cycles < plain.num_cycles);
    }

    #[test]
    fn pipeline_reaches_one_result_per_cycle() {
        let input: Vec<u32> = (0..100).collect();
        let piped = run_pipelined(&input, 5, |x| x + 1);
        // 100 phần tử + 5 chu kỳ đổ đầy ống ≈ 105, chứ không phải 500
        assert!(
            piped.num_cycles <= input.len() + 5,
            "sau khi đầy ống phải ra 1 kết quả/chu kỳ, thực tế {} chu kỳ",
            piped.num_cycles
        );
    }

    #[test]
    fn pipelining_does_not_reduce_latency() {
        let input: Vec<u32> = (0..20).collect();
        let plain = run_unpipelined(&input, 4, |x| x);
        let piped = run_pipelined(&input, 4, |x| x);
        assert_eq!(
            piped.latency, plain.latency,
            "đường ống tăng THÔNG LƯỢNG, không giảm ĐỘ TRỄ — đừng nhầm hai thứ"
        );
    }

    // ---------- Netlist ----------
    #[test]
    fn netlist_sim_matches_direct_function() {
        let mut circuit = Circuit::new();
        let a = circuit.add(Node::Input("a".into()));
        let b = circuit.add(Node::Input("b".into()));
        let c = circuit.add(Node::Input("c".into()));
        let x = circuit.add(Node::Xor(a, b));
        let y = circuit.add(Node::Xor(x, c));
        for va in [false, true] {
            for vb in [false, true] {
                for vc in [false, true] {
                    let mut input = HashMap::new();
                    input.insert("a".to_string(), Signal::from_bool(va));
                    input.insert("b".to_string(), Signal::from_bool(vb));
                    input.insert("c".to_string(), Signal::from_bool(vc));
                    let (expected, _) = full_adder(
                        Signal::from_bool(va),
                        Signal::from_bool(vb),
                        Signal::from_bool(vc),
                    );
                    assert_eq!(circuit.simulate(&input)[y], expected);
                }
            }
        }
    }

    #[test]
    fn critical_path_counts_deepest_stage() {
        let mut circuit = Circuit::new();
        let a = circuit.add(Node::Input("a".into()));
        let b = circuit.add(Node::Input("b".into()));
        let x = circuit.add(Node::And(a, b)); // sâu 1
        let y = circuit.add(Node::Not(x)); // sâu 2
        let _z = circuit.add(Node::Or(y, a)); // sâu 3 (nhánh a sâu 0, lấy max)
        assert_eq!(circuit.critical_path(), 3);
    }

    #[test]
    fn missing_input_propagates_as_x() {
        let mut circuit = Circuit::new();
        let a = circuit.add(Node::Input("a".into()));
        let b = circuit.add(Node::Input("b_unconnected".into()));
        let x = circuit.add(Node::Xor(a, b));
        let mut input = HashMap::new();
        input.insert("a".to_string(), High);
        assert_eq!(
            circuit.simulate(&input)[x],
            X,
            "quên nối một dây → X lan tới đầu ra, đúng như mô phỏng thật"
        );
    }
}
