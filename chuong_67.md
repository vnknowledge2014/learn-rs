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
pub enum TinHieu { Thap, Cao, KhongXacDinh }

impl TinHieu {
    pub fn tu_bool(b: bool) -> TinHieu { if b { TinHieu::Cao } else { TinHieu::Thap } }
    pub fn thanh_bool(self) -> Option<bool> {
        match self { TinHieu::Cao => Some(true), TinHieu::Thap => Some(false), _ => None }
    }
}

pub fn cong_khong(a: TinHieu) -> TinHieu {
    match a { TinHieu::Cao => TinHieu::Thap, TinHieu::Thap => TinHieu::Cao, x => x }
}
pub fn cong_va(a: TinHieu, b: TinHieu) -> TinHieu {
    // Lưu ý: 0 AND X = 0, KHÔNG phải X — vì kết quả đã xác định dù X là gì.
    // Đây gọi là "làm ngắn mạch giá trị điều khiển" và có thật trên silicon.
    match (a, b) {
        (TinHieu::Thap, _) | (_, TinHieu::Thap) => TinHieu::Thap,
        (TinHieu::Cao, TinHieu::Cao) => TinHieu::Cao,
        _ => TinHieu::KhongXacDinh,
    }
}
pub fn cong_hoac(a: TinHieu, b: TinHieu) -> TinHieu {
    match (a, b) {
        (TinHieu::Cao, _) | (_, TinHieu::Cao) => TinHieu::Cao,
        (TinHieu::Thap, TinHieu::Thap) => TinHieu::Thap,
        _ => TinHieu::KhongXacDinh,
    }
}
pub fn cong_xor(a: TinHieu, b: TinHieu) -> TinHieu {
    match (a.thanh_bool(), b.thanh_bool()) {
        (Some(x), Some(y)) => TinHieu::tu_bool(x ^ y),
        _ => TinHieu::KhongXacDinh, // XOR KHÔNG có giá trị điều khiển
    }
}
/// NAND là cổng "phổ dụng": mọi hàm logic đều dựng được chỉ từ NAND.
pub fn cong_nand(a: TinHieu, b: TinHieu) -> TinHieu { cong_khong(cong_va(a, b)) }

/// Bộ chọn kênh 2-1 — viên gạch của mọi thứ có chữ "if" trong phần cứng.
pub fn bo_chon(chon: TinHieu, khi_0: TinHieu, khi_1: TinHieu) -> TinHieu {
    cong_hoac(cong_va(cong_khong(chon), khi_0), cong_va(chon, khi_1))
}

// ============================================================================
// 2. MẠCH TỔ HỢP — đầu ra chỉ phụ thuộc đầu vào HIỆN TẠI
// ============================================================================

/// Bộ cộng bán phần: cộng 2 bit, cho tổng và nhớ.
pub fn cong_ban_phan(a: TinHieu, b: TinHieu) -> (TinHieu, TinHieu) {
    (cong_xor(a, b), cong_va(a, b))
}

/// Bộ cộng toàn phần: cộng 2 bit CỘNG bit nhớ vào.
pub fn cong_toan_phan(a: TinHieu, b: TinHieu, nho_vao: TinHieu) -> (TinHieu, TinHieu) {
    let (t1, n1) = cong_ban_phan(a, b);
    let (tong, n2) = cong_ban_phan(t1, nho_vao);
    (tong, cong_hoac(n1, n2))
}

#[derive(Debug, PartialEq)]
pub struct KetQuaCong {
    pub tong: u16,
    pub tran: bool,
    /// Số tầng cổng mà tín hiệu phải đi qua — quyết định TẦN SỐ TỐI ĐA của mạch.
    pub do_sau_cong: usize,
}

/// Bộ cộng nhớ nối tiếp 8 bit — cách dựng đơn giản nhất, và CHẬM nhất.
/// Bit nhớ phải "chảy" tuần tự qua cả 8 tầng: độ trễ tỉ lệ THUẬN với số bit.
pub fn cong_noi_tiep_8bit(a: u8, b: u8) -> KetQuaCong {
    let mut nho = TinHieu::Thap;
    let mut tong = 0u16;
    for i in 0..8 {
        let bit_a = TinHieu::tu_bool((a >> i) & 1 == 1);
        let bit_b = TinHieu::tu_bool((b >> i) & 1 == 1);
        let (s, n) = cong_toan_phan(bit_a, bit_b, nho);
        if s == TinHieu::Cao { tong |= 1 << i; }
        nho = n;
    }
    KetQuaCong {
        tong,
        tran: nho == TinHieu::Cao,
        do_sau_cong: 8 * 3, // mỗi bộ cộng toàn phần ~3 tầng cổng, nối tiếp nhau
    }
}

/// Bộ cộng nhìn trước nhớ (carry-lookahead): tính TẤT CẢ bit nhớ SONG SONG
/// từ hai tín hiệu "sinh nhớ" (G = a·b) và "truyền nhớ" (P = a⊕b).
/// Cùng kết quả, nhưng độ sâu chỉ còn ~log(n) thay vì n. Đây là bài học
/// cốt lõi của phần cứng: ĐÁNH ĐỔI DIỆN TÍCH LẤY TỐC ĐỘ.
pub fn cong_nhin_truoc_8bit(a: u8, b: u8) -> KetQuaCong {
    let g = a & b;          // sinh nhớ
    let p = a ^ b;          // truyền nhớ
    let mut nho = [false; 9];
    for i in 0..8 {
        // c[i+1] = G[i] + P[i]·c[i] — trong phần cứng, khai triển hết thành
        // một biểu thức phẳng nên tính đồng thời chỉ trong vài tầng cổng.
        nho[i + 1] = ((g >> i) & 1 == 1) || (((p >> i) & 1 == 1) && nho[i]);
    }
    let mut tong = 0u16;
    for i in 0..8 {
        if ((p >> i) & 1 == 1) ^ nho[i] { tong |= 1 << i; }
    }
    KetQuaCong { tong, tran: nho[8], do_sau_cong: 5 } // ~log2(8) + vài tầng
}

// ============================================================================
// 3. MẠCH TUẦN TỰ — có xung nhịp và TRÍ NHỚ
// ============================================================================

/// Flip-flop D: viên gạch của mọi trí nhớ trong FPGA.
/// Ở MỖI sườn lên của xung nhịp, chốt lấy giá trị đầu vào; giữa hai sườn thì
/// giữ nguyên bất kể đầu vào đổi thế nào.
#[derive(Debug, Clone, Copy, PartialEq)]
pub struct FlipFlopD {
    q: TinHieu,
}

impl FlipFlopD {
    /// Chưa reset thì giá trị là KHÔNG XÁC ĐỊNH — đúng như silicon thật.
    pub fn moi() -> Self { FlipFlopD { q: TinHieu::KhongXacDinh } }
    pub fn q(&self) -> TinHieu { self.q }
    pub fn suon_len(&mut self, d: TinHieu) { self.q = d; }
    pub fn dat_lai(&mut self) { self.q = TinHieu::Thap; }
}

/// Thanh ghi dịch — dùng cho SPI, UART, tính CRC, tạo số giả ngẫu nhiên.
pub struct ThanhGhiDich<const N: usize> {
    o: [FlipFlopD; N],
}

impl<const N: usize> ThanhGhiDich<N> {
    pub fn moi() -> Self { ThanhGhiDich { o: [FlipFlopD::moi(); N] } }
    pub fn dat_lai(&mut self) { for f in self.o.iter_mut() { f.dat_lai(); } }
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
    pub fn suon_len(&mut self, vao: TinHieu) -> TinHieu {
        for i in (1..N).rev() {
            let truoc = self.o[i - 1].q();
            self.o[i].suon_len(truoc);
        }
        self.o[0].suon_len(vao);
        self.o[N - 1].q()
    }
    pub fn doc(&self) -> Vec<TinHieu> { self.o.iter().map(|f| f.q()).collect() }
}

/// Máy trạng thái hữu hạn có xung nhịp — đèn giao thông.
/// Đây là dạng mạch mà FPGA làm tốt nhất: điều khiển tất định, độ trễ đếm được.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum DenGiaoThong { Do, DoVang, Xanh, Vang }

pub struct BoDieuKhienDen {
    pub trang_thai: DenGiaoThong,
    pub bo_dem: u8,
    pub thoi_luong: [u8; 4],
}

impl BoDieuKhienDen {
    pub fn moi() -> Self {
        BoDieuKhienDen { trang_thai: DenGiaoThong::Do, bo_dem: 0, thoi_luong: [5, 1, 4, 2] }
    }
    fn chi_so(&self) -> usize {
        match self.trang_thai {
            DenGiaoThong::Do => 0, DenGiaoThong::DoVang => 1,
            DenGiaoThong::Xanh => 2, DenGiaoThong::Vang => 3,
        }
    }
    /// Một sườn xung nhịp. Toàn bộ logic là TỔ HỢP, chỉ `trang_thai` và
    /// `bo_dem` nằm trong flip-flop — đây là mẫu "logic tách khỏi thanh ghi".
    pub fn suon_len(&mut self) -> DenGiaoThong {
        self.bo_dem += 1;
        if self.bo_dem >= self.thoi_luong[self.chi_so()] {
            self.bo_dem = 0;
            self.trang_thai = match self.trang_thai {
                DenGiaoThong::Do => DenGiaoThong::DoVang,
                DenGiaoThong::DoVang => DenGiaoThong::Xanh,
                DenGiaoThong::Xanh => DenGiaoThong::Vang,
                DenGiaoThong::Vang => DenGiaoThong::Do,
            };
        }
        self.trang_thai
    }
    /// Ràng buộc AN TOÀN: không bao giờ được nhảy thẳng Xanh → Đỏ.
    pub fn chuyen_hop_le(tu: DenGiaoThong, den: DenGiaoThong) -> bool {
        use DenGiaoThong::*;
        matches!((tu, den), (Do, Do) | (Do, DoVang) | (DoVang, DoVang) | (DoVang, Xanh)
                          | (Xanh, Xanh) | (Xanh, Vang) | (Vang, Vang) | (Vang, Do))
    }
}

// ============================================================================
// 4. ĐƯỜNG ỐNG (pipeline) — bí quyết tăng thông lượng của mọi CPU/GPU
// ============================================================================

#[derive(Debug, PartialEq)]
pub struct KetQuaOng {
    pub dau_ra: Vec<u32>,
    pub so_chu_ky: usize,
    /// Độ trễ: bao nhiêu chu kỳ từ lúc nạp đến lúc có kết quả ĐẦU TIÊN.
    pub do_tre: usize,
}

/// Không đường ống: mỗi phần tử phải đi hết `so_tang` giai đoạn rồi mới
/// nạp phần tử kế. Thông lượng = 1 kết quả / `so_tang` chu kỳ.
pub fn xu_ly_khong_ong(dau_vao: &[u32], so_tang: usize, f: impl Fn(u32) -> u32) -> KetQuaOng {
    let dau_ra: Vec<u32> = dau_vao.iter().map(|&x| f(x)).collect();
    KetQuaOng { so_chu_ky: dau_vao.len() * so_tang, do_tre: so_tang, dau_ra }
}

/// Có đường ống: mỗi tầng có thanh ghi riêng, nên `so_tang` phần tử được xử lý
/// ĐỒNG THỜI ở các giai đoạn khác nhau. Sau khi ống đầy: 1 kết quả MỖI chu kỳ.
pub fn xu_ly_co_ong(dau_vao: &[u32], so_tang: usize, f: impl Fn(u32) -> u32) -> KetQuaOng {
    let mut tang: Vec<Option<u32>> = vec![None; so_tang];
    let mut dau_ra = Vec::new();
    let mut chi_so = 0;
    let mut chu_ky = 0;

    while dau_ra.len() < dau_vao.len() {
        // Dịch từ CUỐI về ĐẦU để không ghi đè dữ liệu chưa dùng —
        // giống hệt cách thanh ghi thật cập nhật đồng thời trên sườn xung.
        if let Some(v) = tang[so_tang - 1] { dau_ra.push(v); }
        for i in (1..so_tang).rev() { tang[i] = tang[i - 1]; }
        tang[0] = if chi_so < dau_vao.len() {
            let v = f(dau_vao[chi_so]); chi_so += 1; Some(v)
        } else { None };
        chu_ky += 1;
    }
    KetQuaOng { dau_ra, so_chu_ky: chu_ky, do_tre: so_tang }
}

// ============================================================================
// 5. NETLIST — mô tả mạch dưới dạng đồ thị, rồi mô phỏng
// ============================================================================

#[derive(Debug, Clone)]
pub enum Nut {
    DauVao(String),
    Khong(usize),
    Va(usize, usize),
    Hoac(usize, usize),
    Xor(usize, usize),
}

/// Danh sách nối (netlist) chính là thứ trình tổng hợp sinh ra từ HDL,
/// và cũng là thứ được nạp xuống FPGA.
pub struct MachDien {
    pub nut: Vec<Nut>,
}

impl MachDien {
    pub fn moi() -> Self { MachDien { nut: Vec::new() } }
    pub fn them(&mut self, n: Nut) -> usize { self.nut.push(n); self.nut.len() - 1 }

    /// Mô phỏng: vì netlist là đồ thị không chu trình, tính lần lượt theo
    /// thứ tự thêm vào là đủ — đó chính là "sắp xếp tô-pô" miễn phí.
    pub fn mo_phong(&self, dau_vao: &HashMap<String, TinHieu>) -> Vec<TinHieu> {
        let mut gt = vec![TinHieu::KhongXacDinh; self.nut.len()];
        for (i, n) in self.nut.iter().enumerate() {
            gt[i] = match n {
                Nut::DauVao(ten) => *dau_vao.get(ten).unwrap_or(&TinHieu::KhongXacDinh),
                Nut::Khong(a) => cong_khong(gt[*a]),
                Nut::Va(a, b) => cong_va(gt[*a], gt[*b]),
                Nut::Hoac(a, b) => cong_hoac(gt[*a], gt[*b]),
                Nut::Xor(a, b) => cong_xor(gt[*a], gt[*b]),
            };
        }
        gt
    }

    /// Đường tới hạn: chuỗi cổng DÀI NHẤT từ đầu vào tới đầu ra.
    /// Tần số tối đa của mạch = 1 / (độ trễ đường tới hạn).
    pub fn duong_toi_han(&self) -> usize {
        let mut sau = vec![0usize; self.nut.len()];
        for (i, n) in self.nut.iter().enumerate() {
            sau[i] = match n {
                Nut::DauVao(_) => 0,
                Nut::Khong(a) => sau[*a] + 1,
                Nut::Va(a, b) | Nut::Hoac(a, b) | Nut::Xor(a, b) => sau[*a].max(sau[*b]) + 1,
            };
        }
        sau.into_iter().max().unwrap_or(0)
    }
}

fn main() {
    println!("═══════════════════════════════════════════════════════════");
    println!("   FPGA: CỔNG LOGIC · BỘ CỘNG · FLIP-FLOP · ĐƯỜNG ỐNG       ");
    println!("═══════════════════════════════════════════════════════════");

    println!("\n1. BẢNG CHÂN TRỊ CÓ TRẠNG THÁI 'X'");
    println!("   0 AND X = {:?}  ← đã xác định! (0 là giá trị điều khiển của AND)",
             cong_va(TinHieu::Thap, TinHieu::KhongXacDinh));
    println!("   1 AND X = {:?}", cong_va(TinHieu::Cao, TinHieu::KhongXacDinh));
    println!("   0 XOR X = {:?}  ← XOR không có giá trị điều khiển",
             cong_xor(TinHieu::Thap, TinHieu::KhongXacDinh));

    println!("\n2. HAI CÁCH DỰNG BỘ CỘNG 8 BIT — cùng kết quả, khác tốc độ");
    for (a, b) in [(200u8, 100u8), (255, 1), (37, 91)] {
        let nt = cong_noi_tiep_8bit(a, b);
        let lt = cong_nhin_truoc_8bit(a, b);
        println!("   {:>3} + {:>3} = {:>3} (tràn {}) | nối tiếp {} tầng · nhìn trước {} tầng",
                 a, b, nt.tong, nt.tran, nt.do_sau_cong, lt.do_sau_cong);
        assert_eq!(nt.tong, lt.tong);
    }
    println!("   → Cùng đáp số, nhưng mạch nhìn trước chạy nhanh hơn ~{}×",
             cong_noi_tiep_8bit(0,0).do_sau_cong / cong_nhin_truoc_8bit(0,0).do_sau_cong);

    println!("\n3. THANH GHI DỊCH 4 BIT");
    let mut tg: ThanhGhiDich<4> = ThanhGhiDich::moi();
    tg.dat_lai();
    print!("   Đẩy 1,0,1,1 → ra: ");
    for v in [true, false, true, true] {
        print!("{:?} ", tg.suon_len(TinHieu::tu_bool(v)));
    }
    println!("\n   Nội dung sau 4 chu kỳ: {:?}", tg.doc());

    println!("\n4. MÁY TRẠNG THÁI ĐÈN GIAO THÔNG (mỗi ký tự = 1 chu kỳ nhịp)");
    let mut den = BoDieuKhienDen::moi();
    let chuoi: String = (0..24).map(|_| match den.suon_len() {
        DenGiaoThong::Do => 'Đ', DenGiaoThong::DoVang => 'v',
        DenGiaoThong::Xanh => 'X', DenGiaoThong::Vang => 'V',
    }).collect();
    println!("   {}", chuoi);
    println!("   Không bao giờ có 'XĐ' (xanh nhảy thẳng sang đỏ): {}", !chuoi.contains("XĐ"));

    println!("\n5. ĐƯỜNG ỐNG — 100 phần tử qua mạch 5 tầng");
    let vao: Vec<u32> = (0..100).collect();
    let khong = xu_ly_khong_ong(&vao, 5, |x| x * x);
    let co = xu_ly_co_ong(&vao, 5, |x| x * x);
    println!("   Không ống: {} chu kỳ (độ trễ {})", khong.so_chu_ky, khong.do_tre);
    println!("   Có ống   : {} chu kỳ (độ trễ {}) → nhanh gấp {:.1}×",
             co.so_chu_ky, co.do_tre, khong.so_chu_ky as f64 / co.so_chu_ky as f64);
    println!("   → Độ trễ KHÔNG giảm; chỉ THÔNG LƯỢNG tăng. Hai đại lượng khác nhau.");

    println!("\n6. NETLIST & ĐƯỜNG TỚI HẠN");
    let mut m = MachDien::moi();
    let a = m.them(Nut::DauVao("a".into()));
    let b = m.them(Nut::DauVao("b".into()));
    let c = m.them(Nut::DauVao("c".into()));
    let x = m.them(Nut::Xor(a, b));
    let y = m.them(Nut::Xor(x, c));      // tổng của bộ cộng toàn phần
    let _ = y;
    let mut vao_map = HashMap::new();
    for (k, v) in [("a", true), ("b", true), ("c", false)] {
        vao_map.insert(k.to_string(), TinHieu::tu_bool(v));
    }
    println!("   1 XOR 1 XOR 0 = {:?}", m.mo_phong(&vao_map)[y]);
    println!("   Đường tới hạn = {} tầng cổng", m.duong_toi_han());

    println!("\n═══════════════════════════════════════════════════════════");
    println!("   PHẦN MỀM SONG SONG THEO THỜI GIAN — PHẦN CỨNG THEO KHÔNG GIAN");
    println!("═══════════════════════════════════════════════════════════");
}

#[cfg(test)]
mod kiem_thu {
    use super::*;
    use TinHieu::{Cao, KhongXacDinh, Thap};

    // ---------- Cổng logic ----------
    #[test]
    fn gia_tri_dieu_khien_lam_tan_bien_trang_thai_x() {
        // Bài học phần cứng thật: 0·X = 0 và 1+X = 1, dù X là gì đi nữa.
        assert_eq!(cong_va(Thap, KhongXacDinh), Thap);
        assert_eq!(cong_va(KhongXacDinh, Thap), Thap);
        assert_eq!(cong_hoac(Cao, KhongXacDinh), Cao);
        // nhưng khi không có giá trị điều khiển thì X lan ra
        assert_eq!(cong_va(Cao, KhongXacDinh), KhongXacDinh);
        assert_eq!(cong_xor(Thap, KhongXacDinh), KhongXacDinh);
    }

    #[test]
    fn nand_la_cong_pho_dung() {
        // Dựng NOT, AND, OR chỉ từ NAND — nền tảng của mọi thư viện cổng.
        let khong = |a| cong_nand(a, a);
        let va = |a, b| khong(cong_nand(a, b));
        let hoac = |a, b| cong_nand(khong(a), khong(b));
        for a in [Thap, Cao] {
            assert_eq!(khong(a), cong_khong(a));
            for b in [Thap, Cao] {
                assert_eq!(va(a, b), cong_va(a, b));
                assert_eq!(hoac(a, b), cong_hoac(a, b));
            }
        }
    }

    #[test]
    fn bo_chon_hoat_dong_nhu_lenh_if() {
        assert_eq!(bo_chon(Thap, Cao, Thap), Cao, "chọn=0 → lấy nhánh 0");
        assert_eq!(bo_chon(Cao, Cao, Thap), Thap, "chọn=1 → lấy nhánh 1");
    }

    #[test]
    fn luat_de_morgan_dung_tren_mach() {
        for a in [Thap, Cao] {
            for b in [Thap, Cao] {
                assert_eq!(cong_khong(cong_va(a, b)),
                           cong_hoac(cong_khong(a), cong_khong(b)));
                assert_eq!(cong_khong(cong_hoac(a, b)),
                           cong_va(cong_khong(a), cong_khong(b)));
            }
        }
    }

    // ---------- Bộ cộng ----------
    #[test]
    fn cong_toan_phan_dung_ca_8_to_hop() {
        for a in [false, true] { for b in [false, true] { for c in [false, true] {
            let (t, n) = cong_toan_phan(TinHieu::tu_bool(a), TinHieu::tu_bool(b), TinHieu::tu_bool(c));
            let tong = a as u8 + b as u8 + c as u8;
            assert_eq!(t.thanh_bool(), Some(tong & 1 == 1));
            assert_eq!(n.thanh_bool(), Some(tong >= 2));
        }}}
    }

    #[test]
    fn bo_cong_8bit_khop_voi_so_hoc_may_tinh() {
        // Kiểm thử vét cạn TOÀN BỘ 65 536 tổ hợp — điều bất khả với mạch lớn,
        // nhưng với 8 bit thì đây là chứng minh tuyệt đối.
        for a in 0u16..256 {
            for b in 0u16..256 {
                let kq = cong_noi_tiep_8bit(a as u8, b as u8);
                let that = a + b;
                assert_eq!(kq.tong, that & 0xFF, "{a}+{b}");
                assert_eq!(kq.tran, that > 255, "{a}+{b} phải báo tràn");
            }
        }
    }

    #[test]
    fn hai_kien_truc_cong_cho_ket_qua_y_het_nhau() {
        for a in 0u16..256 {
            for b in 0u16..256 {
                let nt = cong_noi_tiep_8bit(a as u8, b as u8);
                let lt = cong_nhin_truoc_8bit(a as u8, b as u8);
                assert_eq!((nt.tong, nt.tran), (lt.tong, lt.tran),
                           "hai kiến trúc phải tương đương về CHỨC NĂNG: {a}+{b}");
            }
        }
    }

    #[test]
    fn nhin_truoc_nong_hon_noi_tiep() {
        // Đây là toàn bộ lý do người ta chịu tốn thêm cổng cho carry-lookahead.
        assert!(cong_nhin_truoc_8bit(0, 0).do_sau_cong < cong_noi_tiep_8bit(0, 0).do_sau_cong);
    }

    // ---------- Mạch tuần tự ----------
    #[test]
    fn flip_flop_chua_reset_la_khong_xac_dinh() {
        let f = FlipFlopD::moi();
        assert_eq!(f.q(), KhongXacDinh, "silicon thật cũng vậy — phải reset trước khi dùng");
    }

    #[test]
    fn flip_flop_chot_gia_tri_tai_suon_len() {
        let mut f = FlipFlopD::moi();
        f.dat_lai();
        assert_eq!(f.q(), Thap);
        f.suon_len(Cao);
        assert_eq!(f.q(), Cao);
    }

    #[test]
    fn thanh_ghi_dich_tra_bit_sau_dung_n_chu_ky() {
        let mut tg: ThanhGhiDich<4> = ThanhGhiDich::moi();
        tg.dat_lai();
        // Bit đầu tiên phải mất ĐÚNG N = 4 chu kỳ mới ra tới đầu kia.
        // Đây chính là độ trễ của thanh ghi dịch — nền của SPI và UART.
        assert_eq!(tg.suon_len(Cao), Thap);
        assert_eq!(tg.suon_len(Thap), Thap);
        assert_eq!(tg.suon_len(Thap), Thap);
        assert_eq!(tg.suon_len(Thap), Cao, "bit '1' xuất hiện đúng ở chu kỳ thứ 4");
        assert_eq!(tg.suon_len(Thap), Thap, "sau đó ống rỗng trở lại");
    }

    #[test]
    fn den_giao_thong_khong_bao_gio_nhay_xanh_sang_do() {
        let mut d = BoDieuKhienDen::moi();
        let mut truoc = d.trang_thai;
        for _ in 0..200 {
            let nay = d.suon_len();
            assert!(BoDieuKhienDen::chuyen_hop_le(truoc, nay),
                    "chuyển trái phép {:?} → {:?}", truoc, nay);
            truoc = nay;
        }
    }

    #[test]
    fn den_giao_thong_di_het_chu_trinh_va_lap_lai() {
        let mut d = BoDieuKhienDen::moi();
        let tong: u32 = d.thoi_luong.iter().map(|&x| x as u32).sum();
        let mot_vong: Vec<DenGiaoThong> = (0..tong).map(|_| d.suon_len()).collect();
        let vong_hai: Vec<DenGiaoThong> = (0..tong).map(|_| d.suon_len()).collect();
        assert_eq!(mot_vong, vong_hai, "máy trạng thái phải tuần hoàn đúng chu kỳ");
        // và ghé qua đủ cả 4 trạng thái
        for tt in [DenGiaoThong::Do, DenGiaoThong::DoVang, DenGiaoThong::Xanh, DenGiaoThong::Vang] {
            assert!(mot_vong.contains(&tt), "thiếu trạng thái {:?}", tt);
        }
    }

    // ---------- Đường ống ----------
    #[test]
    fn duong_ong_cho_cung_ket_qua_nhung_nhanh_hon_nhieu() {
        let vao: Vec<u32> = (1..=50).collect();
        let khong = xu_ly_khong_ong(&vao, 5, |x| x * 3);
        let co = xu_ly_co_ong(&vao, 5, |x| x * 3);
        assert_eq!(khong.dau_ra, co.dau_ra, "đường ống không được đổi KẾT QUẢ");
        assert!(co.so_chu_ky < khong.so_chu_ky);
    }

    #[test]
    fn duong_ong_dat_thong_luong_mot_ket_qua_moi_chu_ky() {
        let vao: Vec<u32> = (0..100).collect();
        let co = xu_ly_co_ong(&vao, 5, |x| x + 1);
        // 100 phần tử + 5 chu kỳ đổ đầy ống ≈ 105, chứ không phải 500
        assert!(co.so_chu_ky <= vao.len() + 5,
                "sau khi đầy ống phải ra 1 kết quả/chu kỳ, thực tế {} chu kỳ", co.so_chu_ky);
    }

    #[test]
    fn duong_ong_khong_lam_giam_do_tre() {
        let vao: Vec<u32> = (0..20).collect();
        let khong = xu_ly_khong_ong(&vao, 4, |x| x);
        let co = xu_ly_co_ong(&vao, 4, |x| x);
        assert_eq!(co.do_tre, khong.do_tre,
                   "đường ống tăng THÔNG LƯỢNG, không giảm ĐỘ TRỄ — đừng nhầm hai thứ");
    }

    // ---------- Netlist ----------
    #[test]
    fn mo_phong_netlist_khop_voi_ham_truc_tiep() {
        let mut m = MachDien::moi();
        let a = m.them(Nut::DauVao("a".into()));
        let b = m.them(Nut::DauVao("b".into()));
        let c = m.them(Nut::DauVao("c".into()));
        let x = m.them(Nut::Xor(a, b));
        let y = m.them(Nut::Xor(x, c));
        for va in [false, true] { for vb in [false, true] { for vc in [false, true] {
            let mut vao = HashMap::new();
            vao.insert("a".to_string(), TinHieu::tu_bool(va));
            vao.insert("b".to_string(), TinHieu::tu_bool(vb));
            vao.insert("c".to_string(), TinHieu::tu_bool(vc));
            let (tong_that, _) = cong_toan_phan(TinHieu::tu_bool(va), TinHieu::tu_bool(vb), TinHieu::tu_bool(vc));
            assert_eq!(m.mo_phong(&vao)[y], tong_that);
        }}}
    }

    #[test]
    fn duong_toi_han_dem_dung_so_tang_sau_nhat() {
        let mut m = MachDien::moi();
        let a = m.them(Nut::DauVao("a".into()));
        let b = m.them(Nut::DauVao("b".into()));
        let x = m.them(Nut::Va(a, b));         // sâu 1
        let y = m.them(Nut::Khong(x));         // sâu 2
        let _z = m.them(Nut::Hoac(y, a));      // sâu 3 (nhánh a sâu 0, lấy max)
        assert_eq!(m.duong_toi_han(), 3);
    }

    #[test]
    fn dau_vao_thieu_lan_truyen_thanh_x() {
        let mut m = MachDien::moi();
        let a = m.them(Nut::DauVao("a".into()));
        let b = m.them(Nut::DauVao("b_quen_noi".into()));
        let x = m.them(Nut::Xor(a, b));
        let mut vao = HashMap::new();
        vao.insert("a".to_string(), Cao);
        assert_eq!(m.mo_phong(&vao)[x], KhongXacDinh,
                   "quên nối một dây → X lan tới đầu ra, đúng như mô phỏng thật");
    }
}
```

---

## Bảng tra cứu lỗi biên dịch thường gặp

| Lỗi | Nguyên nhân trong chương này | Cách sửa |
|---|---|---|
| `E0277: the trait bound TinHieu: Copy is not satisfied` | Quên `#[derive(Clone, Copy)]` trên `TinHieu` | Enum không trường dữ liệu nên `Copy` — thêm vào derive |
| `E0507: cannot move out of index` | `self.o[i]` khi `FlipFlopD` không `Copy` | Thêm `Copy` hoặc dùng `.q()` để lấy giá trị |
| `E0384: cannot assign twice to immutable variable` | Quên `mut` khi mô phỏng nhiều chu kỳ | `let mut tg: ThanhGhiDich<4> = ...` |
| Mạch "chạy" nhưng thanh ghi dịch chỉ trễ 1 chu kỳ | Vòng lặp chép **xuôi** thay vì **ngược** | `for i in (1..N).rev()` — xem mục 6 phần lý thuyết |
| Kết quả mô phỏng đúng, mạch thật sai | Đọc đầu ra **trước** sườn xung thay vì sau | Cập nhật trạng thái xong mới đọc `q` |
| Đầu ra toàn `KhongXacDinh` | Quên gọi `dat_lai()` sau khi tạo flip-flop | Mọi thiết kế thật đều bắt đầu bằng chuỗi reset |

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
4. **Đường tới hạn quyết định tần số tối đa.** Muốn chip chạy nhanh hơn: rút ngắn chuỗi cổng dài nhất, thường bằng cách chèn thêm tầng thanh ghi.
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
    pub fn moi() -> Self { BoDem4Bit { o: [FlipFlopD::moi(); 4] } }
    pub fn dat_lai(&mut self) { for f in self.o.iter_mut() { f.dat_lai(); } }

    pub fn suon_len(&mut self) -> u8 {
        // BƯỚC 1: tính MỌI tín hiệu đảo từ trạng thái CŨ (logic tổ hợp)
        let mut dao = [false; 4];
        let mut tich = true;                  // "mọi bit thấp hơn đều là 1"
        for i in 0..4 {
            dao[i] = tich;
            tich = tich && self.o[i].q() == TinHieu::Cao;
        }
        // BƯỚC 2: cập nhật đồng thời (thanh ghi)
        for i in 0..4 {
            let cu = self.o[i].q() == TinHieu::Cao;
            self.o[i].suon_len(TinHieu::tu_bool(cu ^ dao[i]));
        }
        self.doc()
    }

    pub fn doc(&self) -> u8 {
        (0..4).fold(0u8, |a, i| a | ((self.o[i].q() == TinHieu::Cao) as u8) << i)
    }
}

// Kiểm chứng:
//   let mut d = BoDem4Bit::moi();
//   d.dat_lai();
//   for mong_doi in 1..=15 { assert_eq!(d.suon_len(), mong_doi); }
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
    let mut tich = 0u16;
    let mut so_bo_cong = 0;
    for i in 0..4 {
        if (b >> i) & 1 == 1 {
            // Trong phần cứng: một hàng bộ cộng toàn phần, chạy SONG SONG
            // với các hàng khác. Ở đây ta chỉ đếm số cổng cần dựng.
            let hang = (a as u16 & 0x0F) << i;
            tich = tich.wrapping_add(hang);
            so_bo_cong += 4;
        }
    }
    ((tich & 0xFF) as u8, so_bo_cong)
}

// Kiểm chứng vét cạn cả 256 tổ hợp:
//   for a in 0u8..16 { for b in 0u8..16 {
//       assert_eq!(nhan_4x4(a, b).0, a * b);
//   }}
```

Điểm đáng suy ngẫm: mạch nhân **luôn** dựng đủ 16 cổng AND và toàn bộ mảng cộng, bất kể giá trị `b`. Phần cứng không "bỏ qua" nhánh — nó chỉ đơn giản là *có mặt ở đó*, tiêu thụ diện tích và điện năng. Cái mà phần mềm gọi là `if` thì phần cứng gọi là *bộ chọn kênh*: cả hai nhánh đều được tính, rồi chọn một.
</details>

**Bài 3.** Thêm **kiểm tra vòng lặp tổ hợp** cho `MachDien`: phát hiện trường hợp đầu ra một cổng quay ngược về chính đầu vào của nó.

<details>
<summary><b>Gợi ý</b></summary>

Cấu trúc `Vec<Nut>` hiện tại **không thể** tạo vòng lặp, vì mỗi nút chỉ tham chiếu tới chỉ số **nhỏ hơn** chính nó. Đó là một bất biến ngầm rất mạnh — hãy làm nó **tường minh** bằng một hàm kiểm tra.

Vì sao vòng lặp tổ hợp nguy hiểm? Vì mạch không bao giờ ổn định. Một cổng NOT nối đầu ra về đầu vào sẽ dao động ở tần số do độ trễ vật lý quyết định — đó là bộ dao động vòng, hữu ích khi cố ý nhưng là thảm họa khi vô tình.
</details>

<details>
<summary><b>Lời giải</b></summary>

```rust
impl MachDien {
    /// Bất biến: mọi cổng chỉ được tham chiếu tới nút có chỉ số NHỎ HƠN.
    /// Vi phạm = có vòng lặp tổ hợp = mạch không bao giờ ổn định.
    pub fn kiem_tra_khong_chu_trinh(&self) -> Result<(), String> {
        for (i, n) in self.nut.iter().enumerate() {
            let cac_dau_vao: Vec<usize> = match n {
                Nut::DauVao(_) => vec![],
                Nut::Khong(a) => vec![*a],
                Nut::Va(a, b) | Nut::Hoac(a, b) | Nut::Xor(a, b) => vec![*a, *b],
            };
            for dv in cac_dau_vao {
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
