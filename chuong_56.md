# Chương 56: Kỹ nghệ Ngữ cảnh, Bộ khung, Vòng lặp và Đồ thị cho Tác tử AI (Context, Harness, Loop & Graph Engineering)

## Giới thiệu & Mục tiêu học tập

Chủ đề 8 (Chương 43–47) dạy bạn **Prompt Engineering** — nghệ thuật viết câu lệnh cho mô hình ngôn ngữ. Nhưng nghề này đã dịch chuyển rất nhanh. Năm 2023 người ta tuyển "Prompt Engineer"; đến nay, prompt chỉ còn là **một phần nhỏ** của bài toán. Thứ quyết định một ứng dụng AI chạy được hay không nằm ở bốn tầng kỹ nghệ khác:

| Tầng | Câu hỏi cốt lõi | Hỏng thì sao? |
|---|---|---|
| **Context Engineering** | Nhét *cái gì* vào cửa sổ ngữ cảnh có hạn? | Mô hình bỏ sót thông tin quan trọng, hoặc hóa đơn token bùng nổ |
| **Harness Engineering** | Tác tử được phép *làm gì*? | Tác tử gọi hàm không tồn tại, hoặc xóa nhầm dữ liệu thật |
| **Loop Engineering** | Khi nào thì *dừng*? | Vòng lặp vô hạn — hóa đơn API không đáy |
| **Graph Engineering** | Tri thức *liên kết* với nhau ra sao? | Truy xuất bỏ sót thông tin cách 2–3 bước quan hệ |

Chương này dạy cả bốn tầng bằng Rust, và điểm mấu chốt về mặt kỹ thuật: **toàn bộ mã chạy offline**. Mô hình ngôn ngữ được thay bằng một bản giả tất định — đúng kỹ thuật *test double* ở Chương 55. Nhờ vậy bạn học được kiến trúc mà không cần khóa API, và quan trọng hơn: **hệ thống tác tử của bạn trở nên kiểm thử được**, điều mà phần lớn dự án AI ngoài kia không làm được.

Mục tiêu học tập:
- Xem **ngữ cảnh là tài nguyên có hạn** và biết cách phân bổ nó như bài toán xếp ba lô.
- Hiểu hiện tượng **"Lost in the Middle"** và cách sắp xếp ngữ cảnh để chống lại nó.
- Thiết kế **bộ khung (harness)**: định nghĩa không gian hành động của tác tử bằng `trait`, biến "tác tử được phép làm gì" thành một hợp đồng kiểu.
- Viết **vòng lặp tác tử có phanh**: ba điều kiện dừng bắt buộc (hoàn thành, hết ngân sách, phát hiện lặp).
- Xây **đồ thị tri thức** và truy xuất lan tỏa nhiều bước (nền tảng của GraphRAG).
- Biết vì sao mọi thành phần trên đều phải **kiểm thử được**, và cách đạt điều đó bằng test double.

---

## Hình tượng hóa đời sống (Intuitive Everyday Analogy)

```
┌──────────────────────────────────────────────────────────────────────────────────┐
│      HÌNH TƯỢNG: THUÊ MỘT TRỢ LÝ GIỎI NHƯNG MẤT TRÍ NHỚ SAU MỖI CUỘC HỌP         │
├──────────────────────────────────────────────────────────────────────────────────┤
│                                                                                  │
│  Bạn thuê một chuyên gia cực giỏi. Nhưng anh ta có ba đặc điểm kỳ lạ:            │
│                                                                                  │
│  1. CHIẾC CẶP CÓ HẠN (Context Engineering)                                       │
│     Anh ta chỉ mang được 1 chiếc cặp vào phòng họp. Bạn có 400 trang tài liệu    │
│     nhưng cặp chỉ nhét vừa 100 trang. → Chọn 100 trang NÀO?                      │
│     Và: anh ta đọc kỹ trang đầu, trang cuối, còn phần giữa thì lướt.             │
│                                                                                  │
│  2. THẺ RA VÀO CÓ GIỚI HẠN (Harness Engineering)                                 │
│     Anh ta chỉ mở được những cánh cửa bạn cấp thẻ: phòng kho, phòng kế toán.     │
│     KHÔNG có thẻ phòng máy chủ. → Bạn quyết định anh ta LÀM ĐƯỢC GÌ.             │
│                                                                                  │
│  3. KHÔNG BIẾT KHI NÀO NÊN NGHỈ (Loop Engineering)                               │
│     Nếu không ai bảo dừng, anh ta sẽ đi tra cứu mãi — mỗi lần tra tốn tiền.      │
│     → Phải đặt: "tối đa 5 lần tra" và "nếu tra lại đúng thứ vừa tra, dừng ngay". │
│                                                                                  │
│  4. TẤM BẢN ĐỒ QUAN HỆ (Graph Engineering)                                       │
│     Hỏi "đơn hàng này ở kho nào?" — hồ sơ đơn hàng KHÔNG ghi kho.                │
│     Phải đi: Đơn hàng → Vận đơn → Kho. Tìm kiếm từ khóa phẳng sẽ bó tay.         │
└──────────────────────────────────────────────────────────────────────────────────┘
```

---

## Khái niệm & Cơ chế kỹ thuật chuyên sâu (In-Depth Technical Mechanics)

### 1. Context Engineering — ngữ cảnh là tài nguyên, không phải thùng rác

Sai lầm phổ biến nhất khi xây ứng dụng AI: *"cứ nhét hết mọi thứ vào cho chắc"*. Ba lý do khiến cách này hỏng:

1. **Cửa sổ có hạn.** Vượt giới hạn thì phần đầu bị cắt — thường lại chính là chỉ dẫn hệ thống quan trọng nhất.
2. **Chi phí tuyến tính theo token.** Nhét thừa 10× ngữ cảnh nghĩa là hóa đơn gấp 10 lần, và độ trễ cũng tăng.
3. **Nhiễu làm giảm chất lượng.** Tài liệu không liên quan *làm loãng* tín hiệu; mô hình dễ bám vào chi tiết sai.

Vậy bài toán thực sự là: **chọn tập con ngữ cảnh có giá trị cao nhất trong một ngân sách token cố định**. Đây chính xác là **bài toán xếp ba lô (knapsack)**. Lời giải thực dụng dùng chiến lược tham lam theo *mật độ giá trị*:

```
điểm ưu tiên = độ liên quan / số token
```

Một tài liệu 2000 token với điểm liên quan 0.95 có thể **thua** một tài liệu 90 token điểm 0.5 — vì cái nhỏ cho nhiều giá trị hơn trên mỗi token bỏ ra. Mã trong chương này cài đúng chiến lược đó, cộng thêm cơ chế **ghim cứng** cho những mẩu không bao giờ được loại (quy tắc an toàn, định danh phiên).

### 2. "Lost in the Middle" và cách sắp xếp chống lại nó

Nghiên cứu về mô hình ngôn ngữ cho thấy một hiện tượng nhất quán: **mô hình ghi nhớ tốt phần đầu và phần cuối của ngữ cảnh, kém nhất ở khoảng giữa** — giống hệt trí nhớ con người khi đọc một danh sách dài.

Hệ quả thực hành rất cụ thể: sau khi đã chọn được các mẩu ngữ cảnh, **thứ tự sắp xếp vẫn còn quan trọng**. Chiến lược đơn giản mà hiệu quả: xếp giảm dần theo độ liên quan rồi **rải xen kẽ ra hai đầu**, để những mẩu quan trọng nhất nằm ở đầu và cuối, mẩu ít quan trọng bị đẩy vào giữa.

### 3. Harness Engineering — không gian hành động là một hợp đồng kiểu

Một tác tử không "biết làm mọi thứ". Nó chỉ làm được đúng những gì bạn **cấp công cụ**. Tập công cụ đó gọi là **bộ khung (harness)**, và trong Rust nó được biểu diễn tự nhiên bằng `trait`:

```rust
pub trait LegacyTool {
    fn name(&self) -> &str;
    fn description(&self) -> &str;              // phần này nạp vào ngữ cảnh cho mô hình đọc
    fn run(&self, param: &str) -> LegacyToolResult;
}
```

Ba nguyên tắc thiết kế bộ khung:

1. **Mô tả công cụ chính là giao diện người dùng của tác tử.** Mô hình chỉ biết công cụ qua phần `description`. Viết mô tả mơ hồ thì tác tử gọi sai — đây là "lỗi giao diện", không phải "lỗi mô hình".
2. **Danh sách trắng, không phải danh sách đen.** Tác tử chỉ gọi được thứ đã đăng ký; mọi thứ khác trả lỗi. Trong mã dưới đây, `khung.goi("xoa_o_cung", "/")` **luôn** thất bại vì công cụ đó chưa từng được đăng ký.
3. **Trả lỗi có nội dung, đừng panic.** `LegacyToolResult::Loi("\"x\" không phải số")` cho tác tử cơ hội **tự sửa** ở lượt sau. Một `panic!` thì giết cả tiến trình.

> Đây chính là kiến trúc "lõi thuần túy — vỏ mệnh lệnh" ở Chương 20, áp dụng vào AI: bộ khung là **vỏ** kiểm soát mọi tác dụng phụ, còn logic quyết định là **lõi**.

### 4. Loop Engineering — vòng lặp tác tử phải có phanh

Một tác tử hoạt động theo vòng: *quan sát → quyết định → hành động → quan sát...* Nếu vòng này không có điều kiện dừng, bạn có một chương trình gọi API vô hạn. **Ba cái phanh bắt buộc:**

| Phanh | Cơ chế | Chặn được gì |
|---|---|---|
| **Hoàn thành** | Tác tử trả về `TraLoi(...)` | Trường hợp bình thường |
| **Hết ngân sách** | Đếm số lượt, dừng ở `N` | Tác tử lan man mãi không kết luận |
| **Phát hiện lặp** | Băm `(tên công cụ, tham số)`, thấy trùng thì dừng | Tác tử **kẹt**: gọi đi gọi lại y hệt |

Cái phanh thứ ba quan trọng hơn người ta tưởng. Một tác tử kẹt thường **không** vượt ngân sách ngay — nó chỉ đốt tiền chậm rãi trong khi chẳng tiến triển gì. Bài test `loop_detects_stuck_agent` dưới đây chứng minh: với ngân sách 50 lượt, tác tử kẹt bị chặn ngay ở bước thứ 2.

### 5. Graph Engineering — khi tìm kiếm phẳng không đủ

Cách truy xuất phổ biến (RAG cơ bản) là: nhúng tài liệu thành vector, tìm k tài liệu *giống nhất* với câu hỏi. Cách này hỏng ở một lớp câu hỏi cụ thể: **câu hỏi cần đi qua nhiều bước quan hệ**.

> *"Đơn hàng ORD-88 xuất từ kho nào?"*

Hồ sơ đơn hàng **không chứa chữ "kho"**. Đường đi thật là: `Đơn hàng → Vận đơn → Kho`. Tìm kiếm theo độ tương tự sẽ không bao giờ tìm ra, vì không có tài liệu nào vừa nói về đơn hàng vừa nói về kho.

**Đồ thị tri thức** giải bài này: mô hình hóa thực thể thành đỉnh, quan hệ thành cạnh có nhãn, rồi **truy xuất lan tỏa** (BFS theo độ sâu) từ điểm xuất phát. Đây là ý tưởng cốt lõi của **GraphRAG**. Hai chi tiết kỹ thuật bắt buộc:
- **Giới hạn độ sâu**: đi 3 bước trên đồ thị dày có thể kéo về nửa cơ sở tri thức.
- **Tập đã thăm**: đồ thị thật luôn có chu trình; thiếu `HashSet` là lặp vô hạn.

Bạn đã có sẵn toàn bộ công cụ cho phần này từ **Chương 30** (đồ thị, BFS, danh sách kề dùng chỉ số).

---

## Mã nguồn minh họa thực chiến (Idiomatic Runnable Rust Blueprint)

```bash
cd code
cargo run  -p ch56
cargo test -p ch56
```

```rust
//! Chương 56 — Kỹ nghệ Ngữ cảnh & Tác tử: Context, Harness, Loop, Graph Engineering.
//! Toàn bộ chạy offline: mô hình ngôn ngữ được thay bằng một bản giả tất định,
//! đúng tinh thần "test double" ở Chương 55 — nhờ vậy mọi thứ kiểm thử được.

use std::collections::{HashMap, HashSet, VecDeque};

// ============================================================================
// PHẦN 1: NGÂN SÁCH NGỮ CẢNH — CONTEXT ENGINEERING
// ============================================================================

/// Một mẩu ngữ cảnh có thể nạp vào cửa sổ của mô hình.
#[derive(Debug, Clone, PartialEq)]
pub struct EdgePattern {
    pub nhan: String,
    pub content: String,
    pub token: usize,
    /// Điểm liên quan tới truy vấn hiện tại (0.0 – 1.0).
    pub lien_quan: f64,
    /// Ghim cứng: luôn nạp bất kể ngân sách (ví dụ: quy tắc an toàn).
    pub ghim: bool,
}

/// Kết quả sau khi cắt gọt theo ngân sách.
#[derive(Debug, PartialEq)]
pub struct EdgeCall {
    pub all_mau: Vec<EdgePattern>,
    pub tong_token: usize,
    pub samples_is_kind: usize,
}

/// CONTEXT ENGINEERING: chọn tập con ngữ cảnh tốt nhất trong ngân sách token.
/// Đây là bài toán xếp ba lô (knapsack) đơn giản hóa: ưu tiên điểm liên quan
/// trên mỗi token, và luôn giữ các mẩu bị ghim.
pub fn close_edge_call(mut mau: Vec<EdgePattern>, ngan_sach: usize) -> EdgeCall {
    let first_total_sell = mau.len();

    // 1. Tách phần ghim cứng — luôn được nạp trước
    let (ghim, mut tuy_chon): (Vec<_>, Vec<_>) = mau.drain(..).partition(|m| m.ghim);
    let mut da_dung: usize = ghim.iter().map(|m| m.token).sum();
    let mut pick: Vec<EdgePattern> = ghim;

    // 2. Xếp phần còn lại theo MẬT ĐỘ giá trị (liên quan / token) giảm dần
    tuy_chon.sort_by(|a, b| {
        let id = a.lien_quan / a.token.max(1) as f64;
        let mb = b.lien_quan / b.token.max(1) as f64;
        mb.partial_cmp(&id).unwrap_or(std::cmp::Ordering::Equal)
            .then(a.nhan.cmp(&b.nhan)) // phá hòa tất định
    });

    // 3. Nhồi vào cho tới khi hết ngân sách
    for m in tuy_chon {
        if da_dung + m.token <= ngan_sach {
            da_dung += m.token;
            pick.push(m);
        }
    }

    // 4. "Lost in the middle": đặt mẩu quan trọng nhất ở ĐẦU và CUỐI
    pick = forgetting_resistant_sort(pick);

    EdgeCall {
        tong_token: da_dung,
        samples_is_kind: first_total_sell - pick.len(),
        all_mau: pick,
    }
}

/// Chống hiện tượng "Lost in the Middle": mô hình nhớ tốt phần đầu và phần cuối,
/// hay quên phần giữa. Vậy hãy đẩy thứ quan trọng nhất ra hai đầu.
pub fn forgetting_resistant_sort(mut mau: Vec<EdgePattern>) -> Vec<EdgePattern> {
    mau.sort_by(|a, b| {
        b.lien_quan.partial_cmp(&a.lien_quan).unwrap_or(std::cmp::Ordering::Equal)
            .then(a.nhan.cmp(&b.nhan))
    });
    let mut first: Vec<EdgePattern> = Vec::new();
    let mut last: Vec<EdgePattern> = Vec::new();
    for (i, m) in mau.into_iter().enumerate() {
        if i % 2 == 0 { first.push(m) } else { last.push(m) }
    }
    last.reverse();
    first.extend(last);
    first
}

// ============================================================================
// PHẦN 2: HARNESS ENGINEERING — ĐỊNH NGHĨA KHÔNG GIAN HÀNH ĐỘNG CỦA TÁC TỬ
// ============================================================================

#[derive(Debug, Clone, PartialEq)]
pub enum LegacyToolResult {
    Xong(String),
    Loi(String),
}

/// Một CÔNG CỤ mà tác tử được phép gọi. Đây chính là "harness":
/// bạn định nghĩa tác tử ĐƯỢC LÀM GÌ, và mọi thứ khác đều bị cấm.
pub trait LegacyTool {
    fn name(&self) -> &str;
    fn description(&self) -> &str;
    fn run(&self, param: &str) -> LegacyToolResult;
}

pub struct LegacyComputeTool;
impl LegacyTool for LegacyComputeTool {
    fn name(&self) -> &str { "tinh_tong" }
    fn description(&self) -> &str { "Cộng các số cách nhau bởi dấu phẩy. Ví dụ: \"3,4,5\"" }
    fn run(&self, param: &str) -> LegacyToolResult {
        let mut tong: i64 = 0;
        for part in param.split(',') {
            match part.trim().parse::<i64>() {
                Ok(n) => tong += n,
                Err(_) => return LegacyToolResult::Loi(format!("{:?} không phải số", part.trim())),
            }
        }
        LegacyToolResult::Xong(tong.to_string())
    }
}

pub struct LookupTool {
    pub store: HashMap<String, String>,
}
impl LegacyTool for LookupTool {
    fn name(&self) -> &str { "tra_cuu" }
    fn description(&self) -> &str { "Tra cứu định nghĩa một thuật ngữ trong kho tri thức." }
    fn run(&self, param: &str) -> LegacyToolResult {
        match self.store.get(param.trim()) {
            Some(v) => LegacyToolResult::Xong(v.clone()),
            None => LegacyToolResult::Loi(format!("Không tìm thấy {:?}", param.trim())),
        }
    }
}

/// Bộ khung (harness) giữ danh mục công cụ và ÁP ĐẶT GIỚI HẠN.
pub struct UnitFrame {
    legacy_tool: Vec<Box<dyn LegacyTool>>,
    pub so_lan_goi_toi_da: usize,
}

impl UnitFrame {
    pub fn new(so_lan_goi_toi_da: usize) -> Self {
        UnitFrame { legacy_tool: Vec::new(), so_lan_goi_toi_da }
    }
    pub fn register(mut self, cc: Box<dyn LegacyTool>) -> Self {
        self.legacy_tool.push(cc);
        self
    }
    /// Bản mô tả công cụ để nhét vào ngữ cảnh — đây là "giao diện" tác tử nhìn thấy.
    pub fn legacy_open_gate(&self) -> String {
        self.legacy_tool.iter()
            .map(|c| format!("- {}: {}", c.name(), c.description()))
            .collect::<Vec<_>>().join("\n")
    }
    pub fn goi(&self, name: &str, param: &str) -> LegacyToolResult {
        match self.legacy_tool.iter().find(|c| c.name() == name) {
            Some(c) => c.run(param),
            None => LegacyToolResult::Loi(format!("Công cụ {:?} không tồn tại trong bộ khung", name)),
        }
    }
}

// ============================================================================
// PHẦN 3: LOOP ENGINEERING — VÒNG LẶP TÁC TỬ CÓ ĐIỀU KIỆN DỪNG
// ============================================================================

#[derive(Debug, Clone, PartialEq)]
pub enum ExecClose {
    GoiCongCu { name: String, param: String },
    TraLoi(String),
}

/// Bộ não của tác tử. Trong thực tế đây là lời gọi tới mô hình ngôn ngữ;
/// ở đây ta dùng một bản GIẢ TẤT ĐỊNH để chương trình kiểm thử được.
pub trait UnitWhich {
    fn decide(&self, nhiem_vu: &str, history: &[String]) -> ExecClose;
}

#[derive(Debug, PartialEq)]
pub enum StopReason {
    HoanThanh,
    HetLuotGoi,
    LapVoHan,
}

#[derive(Debug, PartialEq)]
pub struct ResultRoundLoop {
    pub return_error: Option<String>,
    pub num_step: usize,
    pub stop_reason: StopReason,
    pub order_log: Vec<String>,
}

/// LOOP ENGINEERING: vòng lặp tác tử với BA điều kiện dừng bắt buộc.
/// Một vòng lặp thiếu điều kiện dừng là một hóa đơn API không giới hạn.
pub fn run_round_loop(nhiem_vu: &str, which: &dyn UnitWhich, frame: &UnitFrame) -> ResultRoundLoop {
    let mut history: Vec<String> = Vec::new();
    let mut seen: HashSet<String> = HashSet::new();

    for step in 1..=frame.so_lan_goi_toi_da {
        match which.decide(nhiem_vu, &history) {
            ExecClose::TraLoi(t) => {
                history.push(format!("[{}] TRẢ LỜI: {}", step, t));
                return ResultRoundLoop {
                    return_error: Some(t), num_step: step,
                    stop_reason: StopReason::HoanThanh, order_log: history,
                };
            }
            ExecClose::GoiCongCu { name, param } => {
                // DỪNG #3: phát hiện lặp vô hạn (gọi y hệt lần trước)
                let first_van_manual = format!("{}::{}", name, param);
                if !seen.insert(first_van_manual.clone()) {
                    history.push(format!("[{}] PHÁT HIỆN LẶP: {}", step, first_van_manual));
                    return ResultRoundLoop {
                        return_error: None, num_step: step,
                        stop_reason: StopReason::LapVoHan, order_log: history,
                    };
                }
                let kq = frame.goi(&name, &param);
                history.push(match kq {
                    LegacyToolResult::Xong(v) => format!("[{}] {}({}) -> {}", step, name, param, v),
                    LegacyToolResult::Loi(e) => format!("[{}] {}({}) -> LỖI: {}", step, name, param, e),
                });
            }
        }
    }
    // DỪNG #2: hết ngân sách lượt gọi
    ResultRoundLoop {
        return_error: None, num_step: frame.so_lan_goi_toi_da,
        stop_reason: StopReason::HetLuotGoi, order_log: history,
    }
}

// ============================================================================
// PHẦN 4: GRAPH ENGINEERING — ĐỒ THỊ TRI THỨC & TRUY XUẤT NHIỀU BƯỚC
// ============================================================================

/// Đồ thị tri thức: các thực thể nối với nhau bằng quan hệ có nhãn.
/// Đây là nền của GraphRAG — truy xuất theo QUAN HỆ, không chỉ theo từ khóa.
pub struct RealValueGraph {
    edge: HashMap<String, Vec<(String, String)>>, // đỉnh -> [(nhãn quan hệ, đỉnh đích)]
    description: HashMap<String, String>,
}

impl RealValueGraph {
    pub fn new() -> Self {
        RealValueGraph { edge: HashMap::new(), description: HashMap::new() }
    }
    pub fn add_entity(&mut self, name: &str, description: &str) {
        self.description.insert(name.to_string(), description.to_string());
        self.edge.entry(name.to_string()).or_default();
    }
    pub fn add_relation(&mut self, tu: &str, nhan: &str, den: &str) {
        self.edge.entry(tu.to_string()).or_default()
            .push((nhan.to_string(), den.to_string()));
    }

    /// Truy xuất nhiều bước: từ một điểm xuất phát, đi tối đa `do_sau` bước
    /// để gom ngữ cảnh liên quan. Đây là điểm khác biệt so với tìm kiếm phẳng.
    pub fn broadcast_access(&self, start: &str, do_sau: usize) -> Vec<String> {
        let mut ket_qua = Vec::new();
        let mut da_tham: HashSet<String> = HashSet::new();
        let mut queue: VecDeque<(String, usize)> = VecDeque::new();

        queue.push_back((start.to_string(), 0));
        da_tham.insert(start.to_string());

        while let Some((peak, next)) = queue.pop_front() {
            if let Some(m) = self.description.get(&peak) {
                ket_qua.push(format!("{}: {}", peak, m));
            }
            if next >= do_sau { continue; }
            if let Some(neighbors) = self.edge.get(&peak) {
                let mut sx = neighbors.clone();
                sx.sort(); // tất định
                for (nhan, den) in sx {
                    if da_tham.insert(den.clone()) {
                        ket_qua.push(format!("  ({} --{}--> {})", peak, nhan, den));
                        queue.push_back((den, next + 1));
                    }
                }
            }
        }
        ket_qua
    }
}

// ============================================================================
// PHẦN 5: BỘ NÃO GIẢ TẤT ĐỊNH (test double cho mô hình ngôn ngữ)
// ============================================================================

/// Bộ não giả: quyết định dựa trên luật cố định, nên chương trình TẤT ĐỊNH
/// và kiểm thử được — không cần khóa API, không cần mạng.
pub struct UnitWhichPrice {
    pub size_sell: Vec<ExecClose>,
}
impl UnitWhich for UnitWhichPrice {
    fn decide(&self, _nhiem_vu: &str, history: &[String]) -> ExecClose {
        self.size_sell
            .get(history.len())
            .cloned()
            .unwrap_or_else(|| ExecClose::TraLoi("Hết kịch bản".to_string()))
    }
}

fn main() {
    println!("═══════════════════════════════════════════════════════════════");
    println!("   KỸ NGHỆ NGỮ CẢNH · BỘ KHUNG · VÒNG LẶP · ĐỒ THỊ TRI THỨC    ");
    println!("═══════════════════════════════════════════════════════════════");

    // ---- 1. CONTEXT ENGINEERING ----
    println!("\n1. KỸ NGHỆ NGỮ CẢNH — nhồi 4000 token vào cửa sổ 1000 token");
    let mau = vec![
        EdgePattern { nhan: "quy_tac_an_toan".into(), content: "Không tiết lộ khóa bí mật".into(), token: 50, lien_quan: 0.3, ghim: true },
        EdgePattern { nhan: "tai_lieu_A".into(), content: "...".into(), token: 800, lien_quan: 0.9, ghim: false },
        EdgePattern { nhan: "tai_lieu_B".into(), content: "...".into(), token: 200, lien_quan: 0.85, ghim: false },
        EdgePattern { nhan: "tai_lieu_C".into(), content: "...".into(), token: 2000, lien_quan: 0.95, ghim: false },
        EdgePattern { nhan: "lich_su_chat_cu".into(), content: "...".into(), token: 900, lien_quan: 0.1, ghim: false },
    ];
    let goi = close_edge_call(mau, 1000);
    println!("   Dùng {} / 1000 token, loại bỏ {} mẩu", goi.tong_token, goi.samples_is_kind);
    for m in &goi.all_mau {
        println!("     [{:>4} tok · lq {:.2}{}] {}", m.token, m.lien_quan,
                 if m.ghim { " · GHIM" } else { "" }, m.nhan);
    }
    println!("   → tai_lieu_C (2000 tok) bị loại dù liên quan cao nhất: KHÔNG VỪA ngân sách.");
    println!("   → Thứ tự đã đảo để mẩu quan trọng nằm ở ĐẦU và CUỐI (chống Lost-in-the-Middle).");

    // ---- 2 & 3. HARNESS + LOOP ----
    println!("\n2-3. BỘ KHUNG & VÒNG LẶP TÁC TỬ");
    let mut store = HashMap::new();
    store.insert("Rust".to_string(), "Ngôn ngữ hệ thống an toàn bộ nhớ".to_string());
    let frame = UnitFrame::new(5)
        .register(Box::new(LegacyComputeTool))
        .register(Box::new(LookupTool { store }));
    println!("   Công cụ tác tử được phép dùng:\n{}", frame.legacy_open_gate());

    let which = UnitWhichPrice { size_sell: vec![
        ExecClose::GoiCongCu { name: "tra_cuu".into(), param: "Rust".into() },
        ExecClose::GoiCongCu { name: "tinh_tong".into(), param: "10,20,12".into() },
        ExecClose::TraLoi("Rust là ngôn ngữ hệ thống; tổng là 42.".into()),
    ]};
    let kq = run_round_loop("Tra cứu Rust rồi cộng 10+20+12", &which, &frame);
    for d in &kq.order_log { println!("   {}", d); }
    println!("   Dừng vì: {:?} sau {} bước", kq.stop_reason, kq.num_step);

    // Vòng lặp hỏng: tác tử lặp mãi một lời gọi
    let which_link = UnitWhichPrice { size_sell: vec![
        ExecClose::GoiCongCu { name: "tra_cuu".into(), param: "X".into() },
        ExecClose::GoiCongCu { name: "tra_cuu".into(), param: "X".into() },
    ]};
    let kq2 = run_round_loop("nhiệm vụ hỏng", &which_link, &frame);
    println!("   [Tác tử kẹt] dừng vì: {:?} sau {} bước", kq2.stop_reason, kq2.num_step);

    // ---- 4. GRAPH ENGINEERING ----
    println!("\n4. ĐỒ THỊ TRI THỨC — truy xuất lan tỏa 2 bước");
    let mut g = RealValueGraph::new();
    g.add_entity("DonHang", "Đơn hàng của khách");
    g.add_entity("KhachHang", "Người mua");
    g.add_entity("ThanhToan", "Giao dịch trừ tiền");
    g.add_entity("VanDon", "Phiếu giao hàng");
    g.add_entity("Kho", "Kho hàng vật lý");
    g.add_relation("DonHang", "thuoc_ve", "KhachHang");
    g.add_relation("DonHang", "duoc_tra_boi", "ThanhToan");
    g.add_relation("DonHang", "sinh_ra", "VanDon");
    g.add_relation("VanDon", "xuat_tu", "Kho");
    for dong in g.broadcast_access("DonHang", 2) {
        println!("   {}", dong);
    }
    println!("   → Tìm kiếm từ khóa thường sẽ BỎ SÓT \"Kho\" vì nó cách 2 bước.");

    println!("\n═══════════════════════════════════════════════════════════════");
    println!("  NGỮ CẢNH LÀ TÀI NGUYÊN · VÒNG LẶP PHẢI CÓ PHANH · CÔNG CỤ LÀ HỢP ĐỒNG ");
    println!("═══════════════════════════════════════════════════════════════");
}

#[cfg(test)]
mod tests {
    use super::*;

    fn mau(nhan: &str, token: usize, lq: f64, ghim: bool) -> EdgePattern {
        EdgePattern { nhan: nhan.into(), content: "x".into(), token, lien_quan: lq, ghim }
    }

    #[test]
    fn context_never_exceeds_budget() {
        let list = vec![mau("a", 400, 0.9, false), mau("b", 400, 0.8, false), mau("c", 400, 0.7, false)];
        let g = close_edge_call(list, 1000);
        assert!(g.tong_token <= 1000, "vượt ngân sách: {}", g.tong_token);
        assert_eq!(g.all_mau.len(), 2);
    }

    #[test]
    fn pinned_items_are_always_kept() {
        let list = vec![
            mau("quy_tac", 100, 0.01, true),  // liên quan cực thấp nhưng GHIM
            mau("to", 900, 0.99, false),
        ];
        let g = close_edge_call(list, 1000);
        assert!(g.all_mau.iter().any(|m| m.nhan == "quy_tac"), "mẩu ghim bị loại!");
    }

    #[test]
    fn ranks_by_value_density_not_raw_score() {
        // "nho" có điểm thấp hơn nhưng mật độ (lq/token) cao hơn nhiều
        let list = vec![mau("to", 900, 0.9, false), mau("nho", 90, 0.5, false)];
        let g = close_edge_call(list, 500);
        assert_eq!(g.all_mau.len(), 1);
        assert_eq!(g.all_mau[0].nhan, "nho");
    }

    #[test]
    fn anti_forgetting_puts_key_items_at_both_ends() {
        let list = vec![mau("a", 1, 0.9, false), mau("b", 1, 0.5, false), mau("c", 1, 0.8, false)];
        let sx = forgetting_resistant_sort(list);
        // xếp giảm dần: a(.9) c(.8) b(.5) -> chẵn ra đầu, lẻ ra cuối (đảo): a, b, c
        assert_eq!(sx.first().unwrap().nhan, "a");
        assert_eq!(sx.last().unwrap().nhan, "c");
    }

    #[test]
    fn tool_answers_correctly_and_errors_clearly() {
        let cc = LegacyComputeTool;
        assert_eq!(cc.run("1,2,3"), LegacyToolResult::Xong("6".into()));
        assert!(matches!(cc.run("1,x"), LegacyToolResult::Loi(_)));
    }

    #[test]
    fn harness_rejects_unregistered_tools() {
        let frame = UnitFrame::new(3).register(Box::new(LegacyComputeTool));
        // Tác tử KHÔNG THỂ gọi thứ không được đăng ký — đây là ranh giới an toàn.
        assert!(matches!(frame.goi("xoa_o_cung", "/"), LegacyToolResult::Loi(_)));
    }

    #[test]
    fn loop_stops_on_completion() {
        let frame = UnitFrame::new(5).register(Box::new(LegacyComputeTool));
        let which = UnitWhichPrice { size_sell: vec![
            ExecClose::GoiCongCu { name: "tinh_tong".into(), param: "40,2".into() },
            ExecClose::TraLoi("42".into()),
        ]};
        let kq = run_round_loop("nv", &which, &frame);
        assert_eq!(kq.stop_reason, StopReason::HoanThanh);
        assert_eq!(kq.return_error, Some("42".to_string()));
        assert_eq!(kq.num_step, 2);
    }

    #[test]
    fn loop_stops_when_out_of_calls() {
        let frame = UnitFrame::new(3).register(Box::new(LegacyComputeTool));
        // Bộ não không bao giờ trả lời, chỉ gọi công cụ với tham số KHÁC nhau
        let which = UnitWhichPrice { size_sell: vec![
            ExecClose::GoiCongCu { name: "tinh_tong".into(), param: "1".into() },
            ExecClose::GoiCongCu { name: "tinh_tong".into(), param: "2".into() },
            ExecClose::GoiCongCu { name: "tinh_tong".into(), param: "3".into() },
            ExecClose::GoiCongCu { name: "tinh_tong".into(), param: "4".into() },
        ]};
        let kq = run_round_loop("nv", &which, &frame);
        assert_eq!(kq.stop_reason, StopReason::HetLuotGoi);
        assert_eq!(kq.num_step, 3, "phải dừng đúng ở ngân sách 3 lượt");
    }

    #[test]
    fn loop_detects_stuck_agent() {
        let frame = UnitFrame::new(50).register(Box::new(LegacyComputeTool));
        let which = UnitWhichPrice { size_sell: vec![
            ExecClose::GoiCongCu { name: "tinh_tong".into(), param: "1".into() },
            ExecClose::GoiCongCu { name: "tinh_tong".into(), param: "1".into() }, // y hệt
        ]};
        let kq = run_round_loop("nv", &which, &frame);
        assert_eq!(kq.stop_reason, StopReason::LapVoHan);
        assert!(kq.num_step < 50, "phải dừng SỚM, không chạy hết 50 lượt");
    }

    #[test]
    fn graph_retrieval_respects_depth() {
        let mut g = RealValueGraph::new();
        g.add_entity("A", "a"); g.add_entity("B", "b");
        g.add_entity("C", "c"); g.add_entity("D", "d");
        g.add_relation("A", "r1", "B");
        g.add_relation("B", "r2", "C");
        g.add_relation("C", "r3", "D");

        let sau1 = g.broadcast_access("A", 1);
        assert!(sau1.iter().any(|s| s.starts_with("B:")));
        assert!(!sau1.iter().any(|s| s.starts_with("C:")), "độ sâu 1 không được tới C");

        let sau2 = g.broadcast_access("A", 2);
        assert!(sau2.iter().any(|s| s.starts_with("C:")), "độ sâu 2 phải tới được C");
        assert!(!sau2.iter().any(|s| s.starts_with("D:")));
    }

    #[test]
    fn graph_walk_terminates_on_cycles() {
        let mut g = RealValueGraph::new();
        g.add_entity("A", "a"); g.add_entity("B", "b");
        g.add_relation("A", "r", "B");
        g.add_relation("B", "r", "A"); // chu trình
        let kq = g.broadcast_access("A", 10);
        assert!(kq.len() < 10, "phải dừng nhờ tập đã thăm, không lặp vô hạn");
    }
}
```

---

## Từ mô hình giả tới mô hình thật

Mã trên dùng `UnitWhichPrice` để mọi thứ tất định và kiểm thử được. Khi nối vào mô hình thật, bạn **chỉ thay đúng một cài đặt trait**:

```rust
pub struct BoNaoThat { pub khoa_api: String, pub mo_hinh: String }

impl UnitWhich for BoNaoThat {
    fn decide(&self, nhiem_vu: &str, history: &[String]) -> ExecClose {
        // 1. Dựng ngữ cảnh bằng `close_edge_call` (tôn trọng ngân sách token)
        // 2. Gửi HTTP tới nhà cung cấp mô hình (reqwest + serde_json)
        // 3. Phân tích phản hồi thành ExecClose::GoiCongCu hoặc ExecClose::TraLoi
        todo!("gọi mô hình thật")
    }
}
```

Toàn bộ phần còn lại — bộ khung, vòng lặp, đồ thị, và **tất cả bài kiểm thử** — giữ nguyên không đổi. Đó chính là lợi ích của việc đặt ranh giới bằng `trait` (Chương 12) và tiêm phụ thuộc (Chương 14).

**Hệ sinh thái Rust cho AI** đáng theo dõi:

| Crate | Vai trò |
|---|---|
| [`rig`](https://github.com/0xPlaygrounds/rig) | Khung xây tác tử LLM: nhà cung cấp mô hình, công cụ, RAG, kho vector |
| `async-openai` / `anthropic-sdk` | Client cho từng nhà cung cấp |
| `qdrant-client`, `lancedb` | Kho vector cho truy xuất theo độ tương tự |
| `tiktoken-rs` | Đếm token chính xác — cần cho `close_edge_call` phiên bản thật |
| `tokio` + `reqwest` | Bất đồng bộ và HTTP (Chương 49) |

> **Vì sao dùng Rust cho tác tử AI?** Ba lý do rất thực tế: (1) một tiến trình tác tử Rust tốn ~15MB RAM thay vì 500MB, quan trọng khi chạy hàng nghìn tác tử song song (Chương 48); (2) hệ thống kiểu biến "công cụ" thành hợp đồng kiểm tra được lúc biên dịch, thay vì dictionary lỏng lẻo; (3) `tokio` cho phép chạy hàng nghìn tác tử đồng thời trên một máy (Chương 49).

---

## Tóm tắt chương & Bài tập rèn luyện (Summary & Exercises)

### 4 Điểm cốt lõi cần ghi nhớ:
1. **Ngữ cảnh là tài nguyên có hạn** — bài toán chọn ngữ cảnh là bài toán xếp ba lô, ưu tiên theo *mật độ giá trị* chứ không theo điểm thô. Ghim cứng những gì không được phép mất.
2. **Bộ khung là hợp đồng kiểu**: tác tử chỉ làm được những gì `trait LegacyTool` cho phép. Danh sách trắng, lỗi có nội dung, mô tả rõ ràng.
3. **Vòng lặp phải có ba cái phanh**: hoàn thành, hết ngân sách, phát hiện lặp. Thiếu cái thứ ba là thiếu cái quan trọng nhất.
4. **Đồ thị tri thức giải được lớp câu hỏi nhiều bước** mà tìm kiếm theo độ tương tự bó tay — nhưng bắt buộc phải giới hạn độ sâu và giữ tập đã thăm.

### Bài tập rèn luyện tự giải:

**Bài tập 1 (Công cụ mới trong bộ khung)**
Viết `CongCuThoiTiet` trả về nhiệt độ cho một thành phố từ một `HashMap` cố định, và trả lỗi rõ ràng cho thành phố không có. Đăng ký vào `UnitFrame` rồi viết test chứng minh tác tử gọi được nó.

<details>
<summary><b>Lời giải</b></summary>

```rust
pub struct CongCuThoiTiet { pub data: HashMap<String, i32> }

impl LegacyTool for CongCuThoiTiet {
    fn name(&self) -> &str { "thoi_tiet" }
    fn description(&self) -> &str { "Trả về nhiệt độ (°C) của một thành phố. Ví dụ: \"Hà Nội\"" }
    fn run(&self, param: &str) -> LegacyToolResult {
        match self.data.get(param.trim()) {
            Some(t) => LegacyToolResult::Xong(format!("{}°C", t)),
            None => LegacyToolResult::Loi(format!("Chưa có dữ liệu cho {:?}", param.trim())),
        }
    }
}

#[cfg(test)]
mod bt1 {
    use super::*;
    #[test]
    fn tac_tu_goi_duoc_cong_cu_thoi_tiet() {
        let mut dl = HashMap::new();
        dl.insert("Hà Nội".to_string(), 28);
        let frame = UnitFrame::new(3).register(Box::new(CongCuThoiTiet { data: dl }));
        assert_eq!(frame.goi("thoi_tiet", "Hà Nội"), LegacyToolResult::Xong("28°C".into()));
        assert!(matches!(frame.goi("thoi_tiet", "Sao Hỏa"), LegacyToolResult::Loi(_)));
    }
}
```
</details>

**Bài tập 2 (Phanh thứ tư: ngân sách token)**
Thêm vào `UnitFrame` một trường `token_toi_da: usize` và vào `ResultRoundLoop` một biến thể `StopReason::HetToken`. Mỗi lượt gọi công cụ cộng dồn độ dài kết quả vào bộ đếm; vượt ngưỡng thì dừng.

<details>
<summary><b>Gợi ý</b></summary>

Đây là *cái phanh mà các đội thực chiến hay quên nhất*: tác tử có thể dừng đúng 5 lượt nhưng mỗi lượt kéo về 100.000 token. Đếm lượt là chưa đủ, phải đếm cả **khối lượng**.
</details>

<details>
<summary><b>Lời giải</b></summary>

```rust
// Thêm vào enum:  StopReason::HetToken
// Trong run_round_loop, sau mỗi lời gọi công cụ:
//     da_dung_token += ket_qua_text.len() / 4;   // xấp xỉ: 4 ký tự ~ 1 token
//     if da_dung_token > khung.token_toi_da {
//         return ResultRoundLoop { return_error: None, num_step: buoc,
//                                stop_reason: StopReason::HetToken, order_log: history };
//     }
```

Trong sản phẩm thật, thay phép chia 4 bằng `tiktoken-rs` để đếm token chính xác theo đúng bộ mã hóa của mô hình.
</details>

**Bài tập 3 (Tư duy: chọn chiến lược truy xuất)**
Với mỗi câu hỏi, chọn **tìm kiếm theo độ tương tự (RAG phẳng)** hay **truy xuất theo đồ thị (GraphRAG)**, và giải thích:
1. "Chính sách đổi trả hàng của công ty là gì?"
2. "Đơn hàng ORD-88 do nhân viên nào ở kho nào xử lý?"
3. "Tóm tắt các khiếu nại về sản phẩm tai nghe."
4. "Nếu nhà cung cấp X ngừng hoạt động, những đơn hàng nào bị ảnh hưởng?"

<details>
<summary><b>Lời giải tham khảo</b></summary>

1. **RAG phẳng.** Câu trả lời nằm gọn trong một tài liệu chính sách; tương tự ngữ nghĩa là đủ.
2. **GraphRAG.** Phải đi nhiều bước quan hệ: `Đơn hàng → Vận đơn → Kho → Nhân viên`. Không tài liệu đơn lẻ nào chứa cả chuỗi này.
3. **RAG phẳng** (có lọc). Gom nhiều tài liệu tương tự rồi tóm tắt — đúng thế mạnh của tìm kiếm theo vector.
4. **GraphRAG.** Đây là câu hỏi *lan tỏa ngược*: `Nhà cung cấp → Sản phẩm → Đơn hàng`. Tìm kiếm tương tự sẽ trả về tài liệu *nói về* nhà cung cấp X, chứ không liệt kê được các đơn hàng bị ảnh hưởng.

**Quy tắc rút ra**: nếu câu hỏi chứa chữ *"nào"*, *"ảnh hưởng"*, *"liên quan tới"* và câu trả lời đòi bắc cầu qua nhiều thực thể — hãy nghĩ tới đồ thị. Nếu câu trả lời nằm gọn trong một đoạn văn — RAG phẳng nhanh hơn và rẻ hơn nhiều.
</details>
