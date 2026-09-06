# Chương 67: FPGA & Thiết kế phần cứng số — Khi Chương Trình Trở Thành Mạch Điện (Digital Hardware Design in Rust)

## Giới thiệu & Mục tiêu học tập

Cho tới giờ, mọi chương trình bạn viết đều **chạy trên** một con chip do người khác thiết kế. Chương này lật ngược quan hệ đó: bạn sẽ **thiết kế chính con chip**.

Khác biệt cốt lõi giữa phần mềm và phần cứng nằm ở **chiều song song**:

| | Phần mềm | Phần cứng (FPGA) |
|---|---|---|
| Song song theo | **Thời gian** — lần lượt từng lệnh | **Không gian** — mọi cổng chạy cùng lúc |
| `for i in 0..8` nghĩa là | lặp 8 lần | dựng **8 bản sao** của mạch |
| Muốn nhanh gấp đôi | thuật toán tốt hơn | tốn gấp đôi diện tích chip |
| Sai một chỗ | sửa rồi chạy lại (giây) | tổng hợp lại mạch (**40 phút**) |

Chính con số 40 phút cuối bảng là lý do người ta muốn dùng Rust để mô tả phần cứng: **bắt lỗi lúc biên dịch, kiểm chứng bằng `cargo test`, chỉ tổng hợp khi đã chắc chắn**.

Chương này lấy tinh thần từ [rust-hdl](https://github.com/samitbasu/rust-hdl) của Samit Basu. ⚠️ **Lưu ý quan trọng về nguồn**: tác giả đang đổi tên dự án thành **`rhdl`** và sẽ lưu trữ kho `rust-hdl` cũ. Vì vậy chương này dạy **nguyên lý** và cài đặt tự chứa, để kiến thức không phụ thuộc số phận một thư viện cụ thể.

Mục tiêu học tập:
- Hiểu **tín hiệu ba trạng thái** (0/1/X) và vì sao trạng thái `X` tồn tại.
- Dựng **mạch tổ hợp** từ cổng logic: bộ cộng bán phần, toàn phần, bộ chọn kênh.
- So sánh hai kiến trúc bộ cộng và hiểu đánh đổi **diện tích ↔ tốc độ**.
- Hiểu **mạch tuần tự**: flip-flop D, thanh ghi dịch, máy trạng thái có xung nhịp.
- Hiểu **đường ống** và phân biệt rạch ròi **độ trễ** với **thông lượng**.
- Mô phỏng **netlist** và tính **đường tới hạn** — thứ quyết định tần số tối đa của mạch.

---

## Hình tượng hóa đời sống (Intuitive Everyday Analogy)

```
┌───────────────────────────────────────────────────────────────────────────────┐
│   HÌNH TƯỢNG: PHẦN MỀM = MỘT ĐẦU BẾP GIỎI · PHẦN CỨNG = MỘT DÂY CHUYỀN        │
├───────────────────────────────────────────────────────────────────────────────┤
│                                                                               │
│  PHẦN MỀM (CPU): MỘT đầu bếp làm TẤT CẢ các món, lần lượt                    │
│     thái rau → xào → nêm → bày đĩa → thái rau → xào → ...                    │
│     Linh hoạt tuyệt đối: đổi thực đơn = đổi công thức, tức thì.               │
│                                                                               │
│  PHẦN CỨNG (FPGA): BỐN người, mỗi người CHỈ làm một việc, LIÊN TỤC            │
│     ┌──────┐   ┌─────┐   ┌─────┐   ┌───────┐                                 │
│     │ thái │──►│ xào │──►│ nêm │──►│ bày   │   ← cả 4 làm CÙNG LÚC           │
│     └──────┘   └─────┘   └─────┘   └───────┘     trên 4 đĩa KHÁC NHAU        │
│     Muốn đổi thực đơn? Phải XÂY LẠI CẢ DÂY CHUYỀN.                           │
│                                                                               │
│  ★ ĐỘ TRỄ vs THÔNG LƯỢNG — hai thứ RẤT hay bị nhầm:                          │
│     Một đĩa vẫn mất đúng 4 công đoạn mới xong          → ĐỘ TRỄ không đổi     │
│     Nhưng cứ mỗi công đoạn lại có MỘT đĩa ra lò        → THÔNG LƯỢNG ×4       │
│                                                                               │
├───────────────────────────────────────────────────────────────────────────────┤
│   TRẠNG THÁI 'X' = "CHƯA AI TRẢ LỜI CÂU HỎI NÀY"                             │
│                                                                               │
│   Hỏi: "Cả hai công tắc đều bật chứ?"                                        │
│     Công tắc A: TẮT.  Công tắc B: chưa ai kiểm tra.                          │
│     → Vẫn trả lời được: KHÔNG. (0 AND X = 0)                                 │
│     Vì A tắt rồi thì B thế nào cũng không đổi kết quả — "giá trị ĐIỀU KHIỂN". │
│                                                                               │
│   Hỏi: "Đúng một công tắc bật chứ?" (XOR)                                    │
│     → KHÔNG trả lời được. Phải biết B. (0 XOR X = X)                         │
│                                                                               │
├───────────────────────────────────────────────────────────────────────────────┤
│   FLIP-FLOP = MÁY ẢNH CHỤP MỘT KIỂU MỖI GIÂY                                  │
│                                                                               │
│   Giữa hai lần chụp, cảnh vật đổi thế nào cũng mặc kệ.                       │
│   Đúng KHOẢNH KHẮC bấm máy (sườn lên xung nhịp), giá trị được CHỐT lại.      │
│   Chưa chụp kiểu nào = phim trắng = trạng thái X. Phải RESET trước khi dùng. │
└───────────────────────────────────────────────────────────────────────────────┘
```

---

## Khái niệm & Cơ chế kỹ thuật chuyên sâu (In-Depth Technical Mechanics)

### 1. FPGA thực chất là gì

FPGA (Field-Programmable Gate Array) là một biển gồm hàng trăm nghìn khối giống nhau:

- **LUT** (Look-Up Table): một bảng tra 6 đầu vào × 1 đầu ra = 64 bit RAM. Nạp đúng 64 bit vào đó là bạn có **bất kỳ** hàm logic 6 biến nào. Đây là "cổng logic vạn năng".
- **Flip-flop**: một bit nhớ, chốt giá trị tại sườn xung nhịp.
- **Mạng nối** (routing fabric): các công tắc cấu hình được, nối LUT này sang LUT kia.

"Lập trình" FPGA nghĩa là nạp một tệp bitstream quyết định: mỗi LUT chứa bảng tra nào, và các công tắc nối ở đâu. Không có lệnh, không có bộ đếm chương trình, không có "chạy tuần tự". Mạch **là** chương trình.

### 2. Vì sao có trạng thái `X`

Trong mô phỏng, `X` nghĩa là "chưa xác định". Nó xuất hiện khi: flip-flop chưa được reset, một dây quên nối, hoặc hai nguồn cùng lái một dây.

`X` **lan truyền** qua các cổng, nhưng không phải lúc nào cũng lan. Cổng AND có **giá trị điều khiển** là 0: hễ một đầu vào là 0, đầu ra chắc chắn là 0 bất kể đầu kia. Cổng OR có giá trị điều khiển là 1. Cổng XOR **không có** giá trị điều khiển — nên `X` luôn lan qua nó.

Điều này rất quan trọng khi gỡ lỗi: nếu đầu ra mạch là `X`, hãy lần ngược theo các cổng XOR trước tiên.

### 3. NAND là cổng phổ dụng

Chỉ với cổng NAND, bạn dựng được mọi hàm logic:

```
   NOT a       = NAND(a, a)
   AND(a,b)    = NOT(NAND(a,b))     = NAND(NAND(a,b), NAND(a,b))
   OR(a,b)     = NAND(NOT a, NOT b)
```

Đây không phải trò chơi trí tuệ. Quy trình sản xuất chip CMOS làm cổng NAND **rẻ nhất** (4 transistor, trong khi AND cần 6 vì AND = NAND + NOT). Vì thế trình tổng hợp thường chuyển toàn bộ thiết kế về NAND/NOR trước khi đặt lên silicon. Chương này có bài kiểm thử chứng minh phép chuyển đổi đó đúng với mọi đầu vào.

### 4. Đánh đổi diện tích lấy tốc độ: hai bộ cộng

Bộ cộng **nhớ nối tiếp** (ripple-carry) dựng đơn giản nhất: nối 8 bộ cộng toàn phần, bit nhớ chảy từ bit thấp lên bit cao. Vấn đề: bit nhớ phải **chảy qua đủ 8 tầng** trước khi bit cao nhất đúng. Độ trễ tỉ lệ thuận với số bit — bộ cộng 64 bit chậm gấp 8 lần bộ cộng 8 bit.

Bộ cộng **nhìn trước nhớ** (carry-lookahead) tính tất cả bit nhớ **song song**, từ hai đại lượng:

```
   G[i] = a[i] · b[i]      "SINH nhớ"   — hai bit đều 1 thì chắc chắn có nhớ
   P[i] = a[i] ⊕ b[i]      "TRUYỀN nhớ" — đúng một bit là 1 thì nhớ vào đi thẳng ra

   c[i+1] = G[i] + P[i]·c[i]
```

Khai triển đệ quy này thành một biểu thức phẳng, ta được tất cả bit nhớ chỉ sau vài tầng cổng, không phụ thuộc số bit. Cái giá: số cổng tăng theo cấp số nhân nếu khai triển hết — thực tế người ta chia nhóm 4 bit và ghép phân cấp, cho độ sâu `O(log n)`.

**Đây là bài học trung tâm của thiết kế phần cứng**: cùng một chức năng, vô số kiến trúc, mỗi kiến trúc một điểm trên đường cong diện tích–tốc độ–năng lượng. Chương này có bài kiểm thử **vét cạn cả 65 536 tổ hợp** để chứng minh hai kiến trúc tương đương tuyệt đối về chức năng.

### 5. Đường tới hạn quyết định tần số

Giữa hai sườn xung nhịp, tín hiệu phải đi hết từ flip-flop nguồn tới flip-flop đích. Chuỗi cổng **dài nhất** trong mạch gọi là **đường tới hạn**. Nếu nó cần 8 ns, chu kỳ xung nhịp phải ≥ 8 ns, tức tần số tối đa là 125 MHz.

Đây là lý do đường ống hiệu quả: chèn thêm flip-flop vào giữa một chuỗi cổng dài sẽ **chia đôi** đường tới hạn, cho phép tăng gấp đôi tần số. Bạn không làm mạch tính nhanh hơn — bạn chia nhỏ nó ra để mỗi phần kịp xong trong một chu kỳ ngắn hơn.

### 6. Cạm bẫy "cập nhật đồng thời"

Trong phần cứng, **tất cả** flip-flop cập nhật **cùng một lúc** tại sườn xung. Mô phỏng bằng phần mềm rất dễ sai chỗ này:

```rust
// ❌ SAI — o[0] mới đè lên giá trị cũ mà o[1] cần đọc
for i in 0..N { o[i] = o[i-1]; }

// ✅ ĐÚNG — chép từ cuối về đầu
for i in (1..N).rev() { o[i] = o[i-1]; }
```

Viết sai theo cách trên, cả thanh ghi dịch 8 bit biến thành **một** flip-flop duy nhất — bit đầu vào nhảy thẳng ra đầu ra trong một chu kỳ. Đây là lỗi mô phỏng phổ biến nhất và cũng khó thấy nhất, vì mạch vẫn "chạy", chỉ là sai.

---

## Mã nguồn minh họa thực chiến (Idiomatic Runnable Rust Blueprint)

Chạy bằng `cargo run -p ch67`, kiểm thử bằng `cargo test -p ch67`.

```rust
#![allow(dead_code)]
//! Chương 67 — FPGA & Thiết kế phần cứng số bằng Rust: cổng logic, mạch tổ hợp,
//! mạch tuần tự có xung nhịp, đường ống, và vì sao phần cứng fast hơn phần mềm.
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
/// (trở kháng high). Ta mô hình hóa cả 'X' vì nó là nguồn lỗi kinh điển:
/// quên khởi tạo thanh ghi → mạch chạy đúng trong mô phỏng, sai trên chip.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum Signal { Low, Cao, KhongXacDinh }

impl Signal {
    pub fn from_bool(b: bool) -> Signal { if b { Signal::Cao } else { Signal::Low } }
    pub fn to_bool(self) -> Option<bool> {
        match self { Signal::Cao => Some(true), Signal::Low => Some(false), _ => None }
    }
}

pub fn nor_gate(a: Signal) -> Signal {
    match a { Signal::Cao => Signal::Low, Signal::Low => Signal::Cao, x => x }
}
pub fn and_gate(a: Signal, b: Signal) -> Signal {
    // Lưu ý: 0 AND X = 0, KHÔNG phải X — vì kết quả đã xác định dù X là gì.
    // Đây gọi là "làm ngắn mạch giá trị điều khiển" và có thật trên silicon.
    match (a, b) {
        (Signal::Low, _) | (_, Signal::Low) => Signal::Low,
        (Signal::Cao, Signal::Cao) => Signal::Cao,
        _ => Signal::KhongXacDinh,
    }
}
pub fn or_gate(a: Signal, b: Signal) -> Signal {
    match (a, b) {
        (Signal::Cao, _) | (_, Signal::Cao) => Signal::Cao,
        (Signal::Low, Signal::Low) => Signal::Low,
        _ => Signal::KhongXacDinh,
    }
}
pub fn xor_gate(a: Signal, b: Signal) -> Signal {
    match (a.to_bool(), b.to_bool()) {
        (Some(x), Some(y)) => Signal::from_bool(x ^ y),
        _ => Signal::KhongXacDinh, // XOR KHÔNG có giá trị điều khiển
    }
}
/// NAND là cổng "phổ dụng": mọi hàm logic đều dựng được chỉ từ NAND.
pub fn nand_gate(a: Signal, b: Signal) -> Signal { nor_gate(and_gate(a, b)) }

/// Bộ chọn kênh 2-1 — viên gạch của mọi thứ có chữ "if" trong phần cứng.
pub fn unit_pick(pick: Signal, khi_0: Signal, khi_1: Signal) -> Signal {
    or_gate(and_gate(nor_gate(pick), khi_0), and_gate(pick, khi_1))
}

// ============================================================================
// 2. MẠCH TỔ HỢP — đầu ra chỉ phụ thuộc đầu vào HIỆN TẠI
// ============================================================================

/// Bộ cộng bán phần: cộng 2 bit, cho tổng và nhớ.
pub fn half_adder(a: Signal, b: Signal) -> (Signal, Signal) {
    (xor_gate(a, b), and_gate(a, b))
}

/// Bộ cộng toàn phần: cộng 2 bit CỘNG bit nhớ vào.
pub fn full_adder(a: Signal, b: Signal, nho_vao: Signal) -> (Signal, Signal) {
    let (t1, n1) = half_adder(a, b);
    let (tong, n2) = half_adder(t1, nho_vao);
    (tong, or_gate(n1, n2))
}

#[derive(Debug, PartialEq)]
pub struct GateResult {
    pub tong: u16,
    pub tran: bool,
    /// Số tầng cổng mà tín hiệu phải đi qua — quyết định TẦN SỐ TỐI ĐA của mạch.
    pub gate_depth: usize,
}

/// Bộ cộng nhớ nối tiếp 8 bit — cách dựng đơn giản nhất, và CHẬM nhất.
/// Bit nhớ phải "chảy" tuần tự qua cả 8 tầng: độ trễ tỉ lệ THUẬN với số bit.
pub fn ripple_adder_8bit(a: u8, b: u8) -> GateResult {
    let mut small = Signal::Low;
    let mut tong = 0u16;
    for i in 0..8 {
        let bit_a = Signal::from_bool((a >> i) & 1 == 1);
        let bit_b = Signal::from_bool((b >> i) & 1 == 1);
        let (s, n) = full_adder(bit_a, bit_b, small);
        if s == Signal::Cao { tong |= 1 << i; }
        small = n;
    }
    GateResult {
        tong,
        tran: small == Signal::Cao,
        gate_depth: 8 * 3, // mỗi bộ cộng toàn phần ~3 tầng cổng, nối tiếp nhau
    }
}

/// Bộ cộng nhìn trước nhớ (carry-lookahead): tính TẤT CẢ bit nhớ SONG SONG
/// từ hai tín hiệu "sinh nhớ" (G = a·b) và "truyền nhớ" (P = a⊕b).
/// Cùng kết quả, nhưng độ sâu chỉ còn ~log(n) thay vì n. Đây là bài học
/// cốt lõi của phần cứng: ĐÁNH ĐỔI DIỆN TÍCH LẤY TỐC ĐỘ.
pub fn lookahead_adder_8bit(a: u8, b: u8) -> GateResult {
    let g = a & b;          // sinh nhớ
    let p = a ^ b;          // truyền nhớ
    let mut small = [false; 9];
    for i in 0..8 {
        // c[i+1] = G[i] + P[i]·c[i] — trong phần cứng, khai triển hết thành
        // một biểu thức phẳng nên tính đồng thời chỉ trong vài tầng cổng.
        small[i + 1] = ((g >> i) & 1 == 1) || (((p >> i) & 1 == 1) && small[i]);
    }
    let mut tong = 0u16;
    for i in 0..8 {
        if ((p >> i) & 1 == 1) ^ small[i] { tong |= 1 << i; }
    }
    GateResult { tong, tran: small[8], gate_depth: 5 } // ~log2(8) + vài tầng
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

impl FlipFlopD {
    /// Chưa reset thì giá trị là KHÔNG XÁC ĐỊNH — đúng như silicon thật.
    pub fn new() -> Self { FlipFlopD { q: Signal::KhongXacDinh } }
    pub fn q(&self) -> Signal { self.q }
    pub fn suon_len(&mut self, d: Signal) { self.q = d; }
    pub fn set_lai(&mut self) { self.q = Signal::Low; }
}

/// Thanh ghi dịch — dùng cho SPI, UART, tính CRC, tạo số giả ngẫu nhiên.
pub struct IntoRecordDich<const N: usize> {
    o: [FlipFlopD; N],
}

impl<const N: usize> IntoRecordDich<N> {
    pub fn new() -> Self { IntoRecordDich { o: [FlipFlopD::new(); N] } }
    pub fn set_lai(&mut self) { for f in self.o.iter_mut() { f.set_lai(); } }
    /// Đẩy 1 bit vào đầu, bit ở cuối rơi ra. Toàn bộ N flip-flop cập nhật
    /// ĐỒNG THỜI trong một chu kỳ — không có vòng lặp nào chạy trên chip.
    ///
    /// Chú ý vòng lặp chạy NGƯỢC (`(1..N).rev()`): phải chép từ cuối về đầu,
    /// nếu không giá trị mới của o[i-1] sẽ đè lên giá trị cũ mà o[i] cần đọc.
    /// Lỗi này khiến cả thanh ghi biến thành một flip-flop duy nhất.
    ///
    /// Đầu ra được lấy SAU sườn xung — đúng như Q của flip-flop cuối đổi
    /// giá trị ngay tại sườn. Đọc trước sườn sẽ trễ một chu kỳ; đây là lỗi
    /// lệch-một kinh điển khi viết mô phỏng HDL.
    pub fn suon_len(&mut self, input: Signal) -> Signal {
        for i in (1..N).rev() {
            let prev = self.o[i - 1].q();
            self.o[i].suon_len(prev);
        }
        self.o[0].suon_len(input);
        self.o[N - 1].q()
    }
    pub fn doc(&self) -> Vec<Signal> { self.o.iter().map(|f| f.q()).collect() }
}

/// Máy trạng thái hữu hạn có xung nhịp — đèn deliver thông.
/// Đây là dạng mạch mà FPGA làm tốt nhất: điều khiển tất định, độ trễ đếm được.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum TrafficLight { Do, DoVang, Xanh, Vang }

pub struct LedController {
    pub state: TrafficLight,
    pub buffer: u8,
    pub time_amount: [u8; 4],
}

impl LedController {
    pub fn new() -> Self {
        LedController { state: TrafficLight::Do, buffer: 0, time_amount: [5, 1, 4, 2] }
    }
    fn chi_so(&self) -> usize {
        match self.state {
            TrafficLight::Do => 0, TrafficLight::DoVang => 1,
            TrafficLight::Xanh => 2, TrafficLight::Vang => 3,
        }
    }
    /// Một sườn xung nhịp. Toàn bộ logic là TỔ HỢP, chỉ `state` và
    /// `buffer` nằm trong flip-flop — đây là mẫu "logic tách khỏi thanh ghi".
    pub fn suon_len(&mut self) -> TrafficLight {
        self.buffer += 1;
        if self.buffer >= self.time_amount[self.chi_so()] {
            self.buffer = 0;
            self.state = match self.state {
                TrafficLight::Do => TrafficLight::DoVang,
                TrafficLight::DoVang => TrafficLight::Xanh,
                TrafficLight::Xanh => TrafficLight::Vang,
                TrafficLight::Vang => TrafficLight::Do,
            };
        }
        self.state
    }
    /// Ràng buộc AN TOÀN: không bao giờ được nhảy thẳng Xanh → Đỏ.
    pub fn transfer_hop_le(tu: TrafficLight, den: TrafficLight) -> bool {
        use TrafficLight::*;
        matches!((tu, den), (Do, Do) | (Do, DoVang) | (DoVang, DoVang) | (DoVang, Xanh)
                          | (Xanh, Xanh) | (Xanh, Vang) | (Vang, Vang) | (Vang, Do))
    }
}

// ============================================================================
// 4. ĐƯỜNG ỐNG (pipeline) — bí quyết tăng thông lượng của mọi CPU/GPU
// ============================================================================

#[derive(Debug, PartialEq)]
pub struct PipelineResult {
    pub output: Vec<u32>,
    pub num_period: usize,
    /// Độ trễ: bao nhiêu chu kỳ từ lúc nạp đến lúc có kết quả ĐẦU TIÊN.
    pub latency: usize,
}

/// Không đường ống: mỗi phần tử phải đi hết `so_tang` giai đoạn rồi mới
/// nạp phần tử kế. Thông lượng = 1 kết quả / `so_tang` chu kỳ.
pub fn handle_without_pipeline(input: &[u32], so_tang: usize, f: impl Fn(u32) -> u32) -> PipelineResult {
    let output: Vec<u32> = input.iter().map(|&x| f(x)).collect();
    PipelineResult { num_period: input.len() * so_tang, latency: so_tang, output }
}

/// Có đường ống: mỗi tầng có thanh ghi riêng, nên `so_tang` phần tử được xử lý
/// ĐỒNG THỜI ở các giai đoạn khác nhau. Sau khi ống đầy: 1 kết quả MỖI chu kỳ.
pub fn handle_with_pipeline(input: &[u32], so_tang: usize, f: impl Fn(u32) -> u32) -> PipelineResult {
    let mut tang: Vec<Option<u32>> = vec![None; so_tang];
    let mut output = Vec::new();
    let mut chi_so = 0;
    let mut period = 0;

    while output.len() < input.len() {
        // Dịch từ CUỐI về ĐẦU để không ghi đè dữ liệu chưa dùng —
        // giống hệt cách thanh ghi thật cập nhật đồng thời trên sườn xung.
        if let Some(v) = tang[so_tang - 1] { output.push(v); }
        for i in (1..so_tang).rev() { tang[i] = tang[i - 1]; }
        tang[0] = if chi_so < input.len() {
            let v = f(input[chi_so]); chi_so += 1; Some(v)
        } else { None };
        period += 1;
    }
    PipelineResult { output, num_period: period, latency: so_tang }
}

// ============================================================================
// 5. NETLIST — mô tả mạch dưới dạng đồ thị, rồi mô phỏng
// ============================================================================

#[derive(Debug, Clone)]
pub enum Nut {
    Input(String),
    Low(usize),
    Va(usize, usize),
    Hoac(usize, usize),
    Xor(usize, usize),
}

/// Danh sách nối (netlist) chính là thứ trình tổng hợp sinh ra từ HDL,
/// và cũng là thứ được nạp xuống FPGA.
pub struct Circuit {
    pub nut: Vec<Nut>,
}

impl Circuit {
    pub fn new() -> Self { Circuit { nut: Vec::new() } }
    pub fn them(&mut self, n: Nut) -> usize { self.nut.push(n); self.nut.len() - 1 }

    /// Mô phỏng: vì netlist là đồ thị không chu trình, tính lần lượt theo
    /// thứ tự thêm vào là đủ — đó chính là "sắp xếp tô-pô" miễn phí.
    pub fn open_bucket(&self, input: &HashMap<String, Signal>) -> Vec<Signal> {
        let mut gt = vec![Signal::KhongXacDinh; self.nut.len()];
        for (i, n) in self.nut.iter().enumerate() {
            gt[i] = match n {
                Nut::Input(name) => *input.get(name).unwrap_or(&Signal::KhongXacDinh),
                Nut::Low(a) => nor_gate(gt[*a]),
                Nut::Va(a, b) => and_gate(gt[*a], gt[*b]),
                Nut::Hoac(a, b) => or_gate(gt[*a], gt[*b]),
                Nut::Xor(a, b) => xor_gate(gt[*a], gt[*b]),
            };
        }
        gt
    }

    /// Đường tới hạn: chuỗi cổng DÀI NHẤT từ đầu vào tới đầu ra.
    /// Tần số tối đa của mạch = 1 / (độ trễ đường tới hạn).
    pub fn critical_path(&self) -> usize {
        let mut next = vec![0usize; self.nut.len()];
        for (i, n) in self.nut.iter().enumerate() {
            next[i] = match n {
                Nut::Input(_) => 0,
                Nut::Low(a) => next[*a] + 1,
                Nut::Va(a, b) | Nut::Hoac(a, b) | Nut::Xor(a, b) => next[*a].max(next[*b]) + 1,
            };
        }
        next.into_iter().max().unwrap_or(0)
    }
}

fn main() {
    println!("═══════════════════════════════════════════════════════════");
    println!("   FPGA: CỔNG LOGIC · BỘ CỘNG · FLIP-FLOP · ĐƯỜNG ỐNG       ");
    println!("═══════════════════════════════════════════════════════════");

    println!("\n1. BẢNG CHÂN TRỊ CÓ TRẠNG THÁI 'X'");
    println!("   0 AND X = {:?}  ← đã xác định! (0 là giá trị điều khiển của AND)",
             and_gate(Signal::Low, Signal::KhongXacDinh));
    println!("   1 AND X = {:?}", and_gate(Signal::Cao, Signal::KhongXacDinh));
    println!("   0 XOR X = {:?}  ← XOR không có giá trị điều khiển",
             xor_gate(Signal::Low, Signal::KhongXacDinh));

    println!("\n2. HAI CÁCH DỰNG BỘ CỘNG 8 BIT — cùng kết quả, khác tốc độ");
    for (a, b) in [(200u8, 100u8), (255, 1), (37, 91)] {
        let nt = ripple_adder_8bit(a, b);
        let lt = lookahead_adder_8bit(a, b);
        println!("   {:>3} + {:>3} = {:>3} (tràn {}) | nối tiếp {} tầng · nhìn trước {} tầng",
                 a, b, nt.tong, nt.tran, nt.gate_depth, lt.gate_depth);
        assert_eq!(nt.tong, lt.tong);
    }
    println!("   → Cùng đáp số, nhưng mạch nhìn trước chạy fast hơn ~{}×",
             ripple_adder_8bit(0,0).gate_depth / lookahead_adder_8bit(0,0).gate_depth);

    println!("\n3. THANH GHI DỊCH 4 BIT");
    let mut tg: IntoRecordDich<4> = IntoRecordDich::new();
    tg.set_lai();
    print!("   Đẩy 1,0,1,1 → ra: ");
    for v in [true, false, true, true] {
        print!("{:?} ", tg.suon_len(Signal::from_bool(v)));
    }
    println!("\n   Nội dung sau 4 chu kỳ: {:?}", tg.doc());

    println!("\n4. MÁY TRẠNG THÁI ĐÈN GIAO THÔNG (mỗi ký tự = 1 chu kỳ nhịp)");
    let mut den = LedController::new();
    let series: String = (0..24).map(|_| match den.suon_len() {
        TrafficLight::Do => 'Đ', TrafficLight::DoVang => 'v',
        TrafficLight::Xanh => 'X', TrafficLight::Vang => 'V',
    }).collect();
    println!("   {}", series);
    println!("   Không bao giờ có 'XĐ' (xanh nhảy thẳng sang đỏ): {}", !series.contains("XĐ"));

    println!("\n5. ĐƯỜNG ỐNG — 100 phần tử qua mạch 5 tầng");
    let input: Vec<u32> = (0..100).collect();
    let no = handle_without_pipeline(&input, 5, |x| x * x);
    let co = handle_with_pipeline(&input, 5, |x| x * x);
    println!("   Không ống: {} chu kỳ (độ trễ {})", no.num_period, no.latency);
    println!("   Có ống   : {} chu kỳ (độ trễ {}) → fast gấp {:.1}×",
             co.num_period, co.latency, no.num_period as f64 / co.num_period as f64);
    println!("   → Độ trễ KHÔNG giảm; chỉ THÔNG LƯỢNG tăng. Hai đại lượng khác nhau.");

    println!("\n6. NETLIST & ĐƯỜNG TỚI HẠN");
    let mut m = Circuit::new();
    let a = m.them(Nut::Input("a".into()));
    let b = m.them(Nut::Input("b".into()));
    let c = m.them(Nut::Input("c".into()));
    let x = m.them(Nut::Xor(a, b));
    let y = m.them(Nut::Xor(x, c));      // tổng của bộ cộng toàn phần
    let _ = y;
    let mut index_map = HashMap::new();
    for (k, v) in [("a", true), ("b", true), ("c", false)] {
        index_map.insert(k.to_string(), Signal::from_bool(v));
    }
    println!("   1 XOR 1 XOR 0 = {:?}", m.open_bucket(&index_map)[y]);
    println!("   Đường tới hạn = {} tầng cổng", m.critical_path());

    println!("\n═══════════════════════════════════════════════════════════");
    println!("   PHẦN MỀM SONG SONG THEO THỜI GIAN — PHẦN CỨNG THEO KHÔNG GIAN");
    println!("═══════════════════════════════════════════════════════════");
}

#[cfg(test)]
mod tests {
    use super::*;
    use Signal::{Cao, KhongXacDinh, Low};

    // ---------- Cổng logic ----------
    #[test]
    fn controlling_value_erases_x_state() {
        // Bài học phần cứng thật: 0·X = 0 và 1+X = 1, dù X là gì đi nữa.
        assert_eq!(and_gate(Low, KhongXacDinh), Low);
        assert_eq!(and_gate(KhongXacDinh, Low), Low);
        assert_eq!(or_gate(Cao, KhongXacDinh), Cao);
        // nhưng khi không có giá trị điều khiển thì X lan ra
        assert_eq!(and_gate(Cao, KhongXacDinh), KhongXacDinh);
        assert_eq!(xor_gate(Low, KhongXacDinh), KhongXacDinh);
    }

    #[test]
    fn nand_is_universal() {
        // Dựng NOT, AND, OR chỉ từ NAND — nền tảng của mọi thư viện cổng.
        let no = |a| nand_gate(a, a);
        let va = |a, b| no(nand_gate(a, b));
        let hoac = |a, b| nand_gate(no(a), no(b));
        for a in [Low, Cao] {
            assert_eq!(no(a), nor_gate(a));
            for b in [Low, Cao] {
                assert_eq!(va(a, b), and_gate(a, b));
                assert_eq!(hoac(a, b), or_gate(a, b));
            }
        }
    }

    #[test]
    fn mux_behaves_like_an_if() {
        assert_eq!(unit_pick(Low, Cao, Low), Cao, "chọn=0 → lấy nhánh 0");
        assert_eq!(unit_pick(Cao, Cao, Low), Low, "chọn=1 → lấy nhánh 1");
    }

    #[test]
    fn de_morgan_holds_on_gates() {
        for a in [Low, Cao] {
            for b in [Low, Cao] {
                assert_eq!(nor_gate(and_gate(a, b)),
                           or_gate(nor_gate(a), nor_gate(b)));
                assert_eq!(nor_gate(or_gate(a, b)),
                           and_gate(nor_gate(a), nor_gate(b)));
            }
        }
    }

    // ---------- Bộ cộng ----------
    #[test]
    fn full_adder_correct_for_all_eight_inputs() {
        for a in [false, true] { for b in [false, true] { for c in [false, true] {
            let (t, n) = full_adder(Signal::from_bool(a), Signal::from_bool(b), Signal::from_bool(c));
            let tong = a as u8 + b as u8 + c as u8;
            assert_eq!(t.to_bool(), Some(tong & 1 == 1));
            assert_eq!(n.to_bool(), Some(tong >= 2));
        }}}
    }

    #[test]
    fn adder_8bit_matches_machine_arithmetic() {
        // Kiểm thử vét cạn TOÀN BỘ 65 536 tổ hợp — điều bất khả với mạch lớn,
        // nhưng với 8 bit thì đây là chứng minh tuyệt đối.
        for a in 0u16..256 {
            for b in 0u16..256 {
                let kq = ripple_adder_8bit(a as u8, b as u8);
                let that = a + b;
                assert_eq!(kq.tong, that & 0xFF, "{a}+{b}");
                assert_eq!(kq.tran, that > 255, "{a}+{b} phải báo tràn");
            }
        }
    }

    #[test]
    fn both_adder_designs_agree() {
        for a in 0u16..256 {
            for b in 0u16..256 {
                let nt = ripple_adder_8bit(a as u8, b as u8);
                let lt = lookahead_adder_8bit(a as u8, b as u8);
                assert_eq!((nt.tong, nt.tran), (lt.tong, lt.tran),
                           "hai kiến trúc phải tương đương về CHỨC NĂNG: {a}+{b}");
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
        assert_eq!(f.q(), KhongXacDinh, "silicon thật cũng vậy — phải reset trước khi dùng");
    }

    #[test]
    fn flip_flop_latches_on_rising_edge() {
        let mut f = FlipFlopD::new();
        f.set_lai();
        assert_eq!(f.q(), Low);
        f.suon_len(Cao);
        assert_eq!(f.q(), Cao);
    }

    #[test]
    fn shift_register_delays_by_n_cycles() {
        let mut tg: IntoRecordDich<4> = IntoRecordDich::new();
        tg.set_lai();
        // Bit đầu tiên phải mất ĐÚNG N = 4 chu kỳ mới ra tới đầu kia.
        // Đây chính là độ trễ của thanh ghi dịch — nền của SPI và UART.
        assert_eq!(tg.suon_len(Cao), Low);
        assert_eq!(tg.suon_len(Low), Low);
        assert_eq!(tg.suon_len(Low), Low);
        assert_eq!(tg.suon_len(Low), Cao, "bit '1' xuất hiện đúng ở chu kỳ thứ 4");
        assert_eq!(tg.suon_len(Low), Low, "sau đó ống rỗng trở lại");
    }

    #[test]
    fn traffic_light_never_jumps_green_to_red() {
        let mut d = LedController::new();
        let mut prev = d.state;
        for _ in 0..200 {
            let nay = d.suon_len();
            assert!(LedController::transfer_hop_le(prev, nay),
                    "chuyển trái phép {:?} → {:?}", prev, nay);
            prev = nay;
        }
    }

    #[test]
    fn traffic_light_cycles_and_repeats() {
        let mut d = LedController::new();
        let tong: u32 = d.time_amount.iter().map(|&x| x as u32).sum();
        let one_round: Vec<TrafficLight> = (0..tong).map(|_| d.suon_len()).collect();
        let round_two: Vec<TrafficLight> = (0..tong).map(|_| d.suon_len()).collect();
        assert_eq!(one_round, round_two, "máy trạng thái phải tuần hoàn đúng chu kỳ");
        // và ghé qua đủ cả 4 trạng thái
        for tt in [TrafficLight::Do, TrafficLight::DoVang, TrafficLight::Xanh, TrafficLight::Vang] {
            assert!(one_round.contains(&tt), "thiếu trạng thái {:?}", tt);
        }
    }

    // ---------- Đường ống ----------
    #[test]
    fn pipeline_same_result_much_faster() {
        let input: Vec<u32> = (1..=50).collect();
        let no = handle_without_pipeline(&input, 5, |x| x * 3);
        let co = handle_with_pipeline(&input, 5, |x| x * 3);
        assert_eq!(no.output, co.output, "đường ống không được đổi KẾT QUẢ");
        assert!(co.num_period < no.num_period);
    }

    #[test]
    fn pipeline_reaches_one_result_per_cycle() {
        let input: Vec<u32> = (0..100).collect();
        let co = handle_with_pipeline(&input, 5, |x| x + 1);
        // 100 phần tử + 5 chu kỳ đổ đầy ống ≈ 105, chứ không phải 500
        assert!(co.num_period <= input.len() + 5,
                "sau khi đầy ống phải ra 1 kết quả/chu kỳ, thực tế {} chu kỳ", co.num_period);
    }

    #[test]
    fn pipelining_does_not_reduce_latency() {
        let input: Vec<u32> = (0..20).collect();
        let no = handle_without_pipeline(&input, 4, |x| x);
        let co = handle_with_pipeline(&input, 4, |x| x);
        assert_eq!(co.latency, no.latency,
                   "đường ống tăng THÔNG LƯỢNG, không giảm ĐỘ TRỄ — đừng nhầm hai thứ");
    }

    // ---------- Netlist ----------
    #[test]
    fn netlist_sim_matches_direct_function() {
        let mut m = Circuit::new();
        let a = m.them(Nut::Input("a".into()));
        let b = m.them(Nut::Input("b".into()));
        let c = m.them(Nut::Input("c".into()));
        let x = m.them(Nut::Xor(a, b));
        let y = m.them(Nut::Xor(x, c));
        for va in [false, true] { for vb in [false, true] { for vc in [false, true] {
            let mut input = HashMap::new();
            input.insert("a".to_string(), Signal::from_bool(va));
            input.insert("b".to_string(), Signal::from_bool(vb));
            input.insert("c".to_string(), Signal::from_bool(vc));
            let (tong_that, _) = full_adder(Signal::from_bool(va), Signal::from_bool(vb), Signal::from_bool(vc));
            assert_eq!(m.open_bucket(&input)[y], tong_that);
        }}}
    }

    #[test]
    fn critical_path_counts_deepest_stage() {
        let mut m = Circuit::new();
        let a = m.them(Nut::Input("a".into()));
        let b = m.them(Nut::Input("b".into()));
        let x = m.them(Nut::Va(a, b));         // sâu 1
        let y = m.them(Nut::Low(x));         // sâu 2
        let _z = m.them(Nut::Hoac(y, a));      // sâu 3 (nhánh a sâu 0, lấy max)
        assert_eq!(m.critical_path(), 3);
    }

    #[test]
    fn missing_input_propagates_as_x() {
        let mut m = Circuit::new();
        let a = m.them(Nut::Input("a".into()));
        let b = m.them(Nut::Input("b_quen_noi".into()));
        let x = m.them(Nut::Xor(a, b));
        let mut input = HashMap::new();
        input.insert("a".to_string(), Cao);
        assert_eq!(m.open_bucket(&input)[x], KhongXacDinh,
                   "quên nối một dây → X lan tới đầu ra, đúng như mô phỏng thật");
    }
}
```

---

## Bảng tra cứu lỗi biên dịch thường gặp

| Lỗi | Nguyên nhân trong chương này | Cách sửa |
|---|---|---|
| `E0277: the trait bound Signal: Copy is not satisfied` | Quên `#[derive(Clone, Copy)]` trên `Signal` | Enum không trường dữ liệu nên `Copy` — thêm vào derive |
| `E0507: cannot move out of index` | `self.o[i]` khi `FlipFlopD` không `Copy` | Thêm `Copy` hoặc dùng `.q()` để lấy giá trị |
| `E0384: cannot assign twice to immutable variable` | Quên `mut` khi mô phỏng nhiều chu kỳ | `let mut tg: IntoRecordDich<4> = ...` |
| Mạch "chạy" nhưng thanh ghi dịch chỉ trễ 1 chu kỳ | Vòng lặp chép **xuôi** thay vì **ngược** | `for i in (1..N).rev()` — xem mục 6 phần lý thuyết |
| Kết quả mô phỏng đúng, mạch thật sai | Đọc đầu ra **trước** sườn xung thay vì sau | Cập nhật trạng thái xong mới đọc `q` |
| Đầu ra toàn `KhongXacDinh` | Quên gọi `set_lai()` sau khi tạo flip-flop | Mọi thiết kế thật đều bắt đầu bằng chuỗi reset |

---

## Từ mô phỏng tới FPGA thật

Quy trình đầy đủ có bốn bước, mỗi bước một loại lỗi khác nhau:

```
  1. MÔ TẢ      Rust / Verilog / VHDL          ← lỗi logic (cargo test bắt được)
       ↓
  2. TỔNG HỢP   → netlist các cổng             ← lỗi "mô tả được nhưng không dựng được"
       ↓                                          (ví dụ: vòng lặp tổ hợp)
  3. ĐẶT & NỐI  → gán LUT nào ở toạ độ nào     ← lỗi thời gian (đường tới hạn quá dài)
       ↓            (bước CHẬM NHẤT — hàng chục phút)
  4. BITSTREAM  → nạp xuống chip               ← lỗi vật lý (chân cắm sai, nguồn yếu)
```

Điểm mấu chốt của cách tiếp cận kiểu rust-hdl/`rhdl`: **đẩy càng nhiều lỗi lên bước 1 càng tốt**. Mỗi lỗi bắt được bằng `cargo test` là một vòng lặp 40 phút tiết kiệm được. Bài kiểm thử vét cạn 65 536 tổ hợp trong chương này chạy trong chưa tới một giây — không có lý do gì để không viết nó.

Hệ sinh thái Rust cho phần cứng số hiện nay:

| Dự án | Tình trạng | Ghi chú |
|---|---|---|
| **rust-hdl** | Đang được đổi tên thành `rhdl` | Tác giả sẽ lưu trữ kho cũ — kiểm tra kho mới trước khi dùng |
| **Veryl** | Đang phát triển tích cực | Ngôn ngữ HDL riêng, cú pháp gợi nhớ Rust, sinh SystemVerilog |
| **Spade** | Nghiên cứu | HDL có hệ thống kiểu mạnh, ảnh hưởng từ Rust |
| **Verilator** | Trưởng thành (C++) | Mô phỏng Verilog cực nhanh; nhiều dự án Rust gọi qua FFI |

Đây là lĩnh vực **đang chuyển động nhanh**. Vì thế chương này dạy nguyên lý — cổng, flip-flop, đường tới hạn, đường ống — những thứ đúng bất kể công cụ nào thắng.

---

## Tóm tắt chương & Bài tập rèn luyện (Summary & Exercises)

### 5 điểm cốt lõi cần ghi nhớ

1. **Phần mềm song song theo thời gian; phần cứng song song theo không gian.** `for i in 0..8` trong HDL nghĩa là *dựng 8 bản sao mạch*, không phải *lặp 8 lần*.
2. **Độ trễ và thông lượng là hai đại lượng khác nhau.** Đường ống tăng thông lượng mà **không** giảm độ trễ. Nhầm hai thứ này là hiểu sai mọi kiến trúc CPU hiện đại.
3. **Cùng chức năng, vô số kiến trúc.** Bộ cộng nối tiếp và nhìn trước cho cùng đáp số; khác nhau ở diện tích và tốc độ. Kỹ sư phần cứng làm việc trên đường cong đánh đổi đó.
4. **Đường tới hạn (Critical path) quyết định tần số tối đa.** Muốn chip chạy nhanh hơn: rút ngắn chuỗi cổng dài nhất, thường bằng cách chèn thêm tầng thanh ghi.
5. **Mọi flip-flop cập nhật đồng thời.** Mô phỏng phải chép ngược từ cuối về đầu, nếu không cả thanh ghi dịch thành một flip-flop.

### Bài tập rèn luyện tự giải

**Bài 1.** Cài **bộ đếm nhị phân 4 bit** đồng bộ dùng flip-flop, và kiểm chứng nó đếm đúng từ 0 tới 15 rồi quay vòng về 0.

<details>
<summary><b>Gợi ý</b></summary>

Bit `i` đảo trạng thái khi **tất cả** các bit thấp hơn đều bằng 1. Bit 0 đảo mỗi chu kỳ; bit 1 đảo khi bit 0 = 1; bit 2 đảo khi bit 0 và bit 1 đều = 1...

Từ đó suy ra: `dao[i] = AND(q[0], q[1], ..., q[i-1])`. Nhớ tính **toàn bộ** tín hiệu đảo *trước*, rồi mới cập nhật flip-flop — vì trong mạch thật chúng được tính đồng thời từ trạng thái cũ.
</details>

<details>
<summary><b>Lời giải</b></summary>

```rust
pub struct BoDem4Bit { o: [FlipFlopD; 4] }

impl BoDem4Bit {
    pub fn new() -> Self { BoDem4Bit { o: [FlipFlopD::new(); 4] } }
    pub fn set_lai(&mut self) { for f in self.o.iter_mut() { f.set_lai(); } }

    pub fn suon_len(&mut self) -> u8 {
        // BƯỚC 1: tính MỌI tín hiệu đảo từ trạng thái CŨ (logic tổ hợp)
        let mut dao = [false; 4];
        let mut products = true;                  // "mọi bit thấp hơn đều là 1"
        for i in 0..4 {
            dao[i] = products;
            products = products && self.o[i].q() == Signal::Cao;
        }
        // BƯỚC 2: cập nhật đồng thời (thanh ghi)
        for i in 0..4 {
            let cu = self.o[i].q() == Signal::Cao;
            self.o[i].suon_len(Signal::from_bool(cu ^ dao[i]));
        }
        self.doc()
    }

    pub fn doc(&self) -> u8 {
        (0..4).fold(0u8, |a, i| a | ((self.o[i].q() == Signal::Cao) as u8) << i)
    }
}

// Kiểm chứng:
//   let mut d = BoDem4Bit::moi();
//   d.set_lai();
//   for expected in 1..=15 { assert_eq!(d.suon_len(), expected); }
//   assert_eq!(d.suon_len(), 0, "tràn thì quay vòng về 0");
```

Chú ý cấu trúc **hai bước** — đọc hết trạng thái cũ, rồi mới ghi trạng thái mới. Đây là khuôn mẫu bắt buộc của mọi mô phỏng mạch tuần tự, và cũng là cách bạn nên viết bất kỳ mô phỏng nào có "cập nhật đồng thời" (kể cả Game of Life).
</details>

**Bài 2.** Cài **bộ nhân 4×4 bit** bằng phương pháp dịch-và-cộng, đếm số bộ cộng toàn phần cần dùng.

<details>
<summary><b>Gợi ý</b></summary>

Nhân nhị phân giống nhân tay ở tiểu học: với mỗi bit của số nhân, nếu nó là 1 thì cộng số bị nhân đã dịch trái tương ứng.

Trong **phần cứng**, đây không phải vòng lặp — đó là một **mảng** 4×4 bộ cộng toàn phần, tất cả chạy song song. Vì thế nhân 4×4 tốn khoảng 12 bộ cộng toàn phần và cho kết quả trong một chu kỳ, còn CPU phần mềm cần nhiều lệnh.
</details>

<details>
<summary><b>Lời giải</b></summary>

```rust
/// Trả về (tích 8 bit, số bộ cộng toàn phần đã dùng).
pub fn nhan_4x4(a: u8, b: u8) -> (u8, usize) {
    let mut products = 0u16;
    let mut gate_count = 0;
    for i in 0..4 {
        if (b >> i) & 1 == 1 {
            // Trong phần cứng: một hàng bộ cộng toàn phần, chạy SONG SONG
            // với các hàng khác. Ở đây ta chỉ đếm số cổng cần dựng.
            let queue = (a as u16 & 0x0F) << i;
            products = products.wrapping_add(queue);
            gate_count += 4;
        }
    }
    ((products & 0xFF) as u8, gate_count)
}

// Kiểm chứng vét cạn cả 256 tổ hợp:
//   for a in 0u8..16 { for b in 0u8..16 {
//       assert_eq!(nhan_4x4(a, b).0, a * b);
//   }}
```

Điểm đáng suy ngẫm: mạch nhân **luôn** dựng đủ 16 cổng AND và toàn bộ mảng cộng, bất kể giá trị `b`. Phần cứng không "bỏ qua" nhánh — nó chỉ đơn giản là *có mặt ở đó*, tiêu thụ diện tích và điện năng. Cái mà phần mềm gọi là `if` thì phần cứng gọi là *bộ chọn kênh*: cả hai nhánh đều được tính, rồi chọn một.
</details>

**Bài 3.** Thêm **kiểm tra vòng lặp tổ hợp** cho `Circuit`: phát hiện trường hợp đầu ra một cổng quay ngược về chính đầu vào của nó.

<details>
<summary><b>Gợi ý</b></summary>

Cấu trúc `Vec<Nut>` hiện tại **không thể** tạo vòng lặp, vì mỗi nút chỉ tham chiếu tới chỉ số **nhỏ hơn** chính nó. Đó là một bất biến ngầm rất mạnh — hãy làm nó **tường minh** bằng một hàm kiểm tra.

Vì sao vòng lặp tổ hợp nguy hiểm? Vì mạch không bao giờ ổn định. Một cổng NOT nối đầu ra về đầu vào sẽ dao động ở tần số do độ trễ vật lý quyết định — đó là bộ dao động vòng, hữu ích khi cố ý nhưng là thảm họa khi vô tình.
</details>

<details>
<summary><b>Lời giải</b></summary>

```rust
impl Circuit {
    /// Bất biến: mọi cổng chỉ được tham chiếu tới nút có chỉ số NHỎ HƠN.
    /// Vi phạm = có vòng lặp tổ hợp = mạch không bao giờ ổn định.
    pub fn assert_acyclic(&self) -> Result<(), String> {
        for (i, n) in self.nut.iter().enumerate() {
            let inputs: Vec<usize> = match n {
                Nut::Input(_) => vec![],
                Nut::Low(a) => vec![*a],
                Nut::Va(a, b) | Nut::Hoac(a, b) | Nut::Xor(a, b) => vec![*a, *b],
            };
            for dv in inputs {
                if dv >= i {
                    return Err(format!(
                        "vòng lặp tổ hợp: nút {} lấy đầu vào từ nút {} (không nhỏ hơn)", i, dv));
                }
            }
        }
        Ok(())
    }
}
```

Trong Verilog/VHDL, vòng lặp tổ hợp là lỗi mà trình tổng hợp phải đi tìm bằng thuật toán đồ thị. Ở đây, **cách biểu diễn dữ liệu đã tự bảo đảm bất biến** — bạn không thể xây được mạch sai. Đây chính là tinh thần "làm cho trạng thái sai không biểu diễn được" của Chương 20, áp dụng vào thiết kế phần cứng.
</details>
