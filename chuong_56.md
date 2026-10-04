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

1. **Cửa sổ có hạn.** Vượt giới hạn thì hoặc yêu cầu bị từ chối, hoặc ứng dụng phải cắt bớt — và nếu cắt máy móc phần cũ nhất, thứ bị cắt thường lại chính là chỉ dẫn hệ thống quan trọng nhất.
2. **Chi phí tuyến tính theo token.** Nhét thừa 10× ngữ cảnh nghĩa là hóa đơn gấp 10 lần, và độ trễ cũng tăng.
3. **Nhiễu làm giảm chất lượng.** Tài liệu không liên quan *làm loãng* tín hiệu; mô hình dễ bám vào chi tiết sai.

Vậy bài toán thực sự là: **chọn tập con ngữ cảnh có giá trị cao nhất trong một ngân sách token cố định**. Đây chính xác là **bài toán xếp ba lô (knapsack)**. Lời giải thực dụng dùng chiến lược tham lam theo *mật độ giá trị* (nhanh, nhưng không đảm bảo tối ưu — mã minh họa có một ví dụ nó chọn hụt; lời giải chính xác cần quy hoạch động):

```
điểm ưu tiên = độ liên quan / số token
```

Một tài liệu 2000 token với điểm liên quan 0.95 có thể **thua** một tài liệu 90 token điểm 0.5 — vì cái nhỏ cho nhiều giá trị hơn trên mỗi token bỏ ra. Mã trong chương này cài đúng chiến lược đó, cộng thêm cơ chế **ghim cứng** cho những mẩu không bao giờ được loại (quy tắc an toàn, định danh phiên) — và luôn đặt chúng ở đầu ngữ cảnh.

### 2. "Lost in the Middle" và cách sắp xếp chống lại nó

Nghiên cứu về mô hình ngôn ngữ cho thấy một hiện tượng nhất quán: **mô hình ghi nhớ tốt phần đầu và phần cuối của ngữ cảnh, kém nhất ở khoảng giữa** — giống hệt trí nhớ con người khi đọc một danh sách dài.

Hệ quả thực hành rất cụ thể: sau khi đã chọn được các mẩu ngữ cảnh, **thứ tự sắp xếp vẫn còn quan trọng**. Chiến lược đơn giản mà hiệu quả: xếp giảm dần theo độ liên quan rồi **rải xen kẽ ra hai đầu**, để những mẩu quan trọng nhất nằm ở đầu và cuối, mẩu ít quan trọng bị đẩy vào giữa.

### 3. Harness Engineering — không gian hành động là một hợp đồng kiểu

Một tác tử không "biết làm mọi thứ". Nó chỉ làm được đúng những gì bạn **cấp công cụ**. Tập công cụ đó gọi là **bộ khung (harness)**, và trong Rust nó được biểu diễn tự nhiên bằng `trait`:

```rust
pub enum ToolResult { Finished(String), Failed(String) }

pub trait Tool {
    fn name(&self) -> &str;
    fn description(&self) -> &str;              // phần này nạp vào ngữ cảnh cho mô hình đọc
    fn run(&self, param: &str) -> ToolResult;
}
```

Ba nguyên tắc thiết kế bộ khung:

1. **Mô tả công cụ chính là giao diện người dùng của tác tử.** Mô hình chỉ biết công cụ qua phần `description`. Viết mô tả mơ hồ thì tác tử gọi sai — đây là "lỗi giao diện", không phải "lỗi mô hình".
2. **Danh sách trắng, không phải danh sách đen.** Tác tử chỉ gọi được thứ đã đăng ký; mọi thứ khác trả lỗi. Trong mã dưới đây, `harness.call("format_disk", "/")` **luôn** thất bại vì công cụ đó chưa từng được đăng ký.
3. **Trả lỗi có nội dung, đừng panic.** `ToolResult::Failed("\"x\" không phải số")` cho tác tử cơ hội **tự sửa** ở lượt sau. Một `panic!` thì giết cả tiến trình — kể cả panic "vô tình" như tràn số khi cộng (vì vậy `CalculatorTool` dùng `checked_add`).

> Đây chính là kiến trúc "lõi thuần túy — vỏ mệnh lệnh" ở Chương 20, áp dụng vào AI: bộ khung là **vỏ** kiểm soát mọi tác dụng phụ, còn logic quyết định là **lõi**.

### 4. Loop Engineering — vòng lặp tác tử phải có phanh

Một tác tử hoạt động theo vòng: *quan sát → quyết định → hành động → quan sát...* Nếu vòng này không có điều kiện dừng, bạn có một chương trình gọi API vô hạn. **Ba cái phanh bắt buộc:**

| Phanh | Cơ chế | Chặn được gì |
|---|---|---|
| **Hoàn thành** | Tác tử trả về `Action::Answer(...)` | Trường hợp bình thường |
| **Hết ngân sách** | Đếm số lượt, dừng ở `N` | Tác tử lan man mãi không kết luận |
| **Phát hiện lặp** | Ghi `(tên công cụ, tham số)` vào một `HashSet`, thấy trùng thì dừng | Tác tử **kẹt**: gọi đi gọi lại y hệt |

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
pub struct ContextChunk {
    pub label: String,
    pub content: String,
    pub token: usize,
    /// Điểm liên quan tới truy vấn hiện tại (0.0 – 1.0).
    pub relevance: f64,
    /// Ghim cứng: luôn nạp bất kể ngân sách (ví dụ: quy tắc an toàn).
    pub pinned: bool,
}

/// Kết quả sau khi cắt gọt theo ngân sách.
#[derive(Debug, PartialEq)]
pub struct ContextPack {
    pub chunks: Vec<ContextChunk>,
    pub total_tokens: usize,
    pub dropped_count: usize,
}

/// CONTEXT ENGINEERING: chọn tập con ngữ cảnh tốt nhất trong ngân sách token.
/// Đây là bài toán xếp ba lô (knapsack) đơn giản hóa: ưu tiên điểm liên quan
/// trên mỗi token, và luôn giữ các mẩu bị ghim.
pub fn pack_context(mut chunks: Vec<ContextChunk>, budget: usize) -> ContextPack {
    let original_count = chunks.len();

    // 1. Tách phần ghim cứng — luôn được nạp trước
    let (pinned, mut optional): (Vec<_>, Vec<_>) = chunks.drain(..).partition(|m| m.pinned);
    let mut used: usize = pinned.iter().map(|m| m.token).sum();
    let mut picked: Vec<ContextChunk> = Vec::new();

    // 2. Xếp phần còn lại theo MẬT ĐỘ giá trị (liên quan / token) giảm dần
    optional.sort_by(|a, b| {
        let da = a.relevance / a.token.max(1) as f64;
        let db = b.relevance / b.token.max(1) as f64;
        db.partial_cmp(&da)
            .unwrap_or(std::cmp::Ordering::Equal)
            .then(a.label.cmp(&b.label)) // phá hòa tất định
    });

    // 3. Nhồi vào cho tới khi hết ngân sách
    for m in optional {
        if used + m.token <= budget {
            used += m.token;
            picked.push(m);
        }
    }

    // 4. Mẩu ghim (quy tắc hệ thống) luôn đứng ĐẦU; phần còn lại xếp chống
    //    "Lost in the middle": mẩu quan trọng nhất ở hai đầu, kém nhất ở giữa.
    let mut ordered = pinned;
    ordered.extend(lost_in_middle_order(picked));

    ContextPack {
        total_tokens: used,
        dropped_count: original_count - ordered.len(),
        chunks: ordered,
    }
}

/// Chống hiện tượng "Lost in the Middle": mô hình nhớ tốt phần đầu và phần cuối,
/// hay quên phần giữa. Vậy hãy đẩy thứ quan trọng nhất ra hai đầu.
pub fn lost_in_middle_order(mut chunks: Vec<ContextChunk>) -> Vec<ContextChunk> {
    chunks.sort_by(|a, b| {
        b.relevance
            .partial_cmp(&a.relevance)
            .unwrap_or(std::cmp::Ordering::Equal)
            .then(a.label.cmp(&b.label))
    });
    let mut first: Vec<ContextChunk> = Vec::new();
    let mut last: Vec<ContextChunk> = Vec::new();
    for (i, m) in chunks.into_iter().enumerate() {
        if i % 2 == 0 {
            first.push(m)
        } else {
            last.push(m)
        }
    }
    last.reverse();
    first.extend(last);
    first
}

// ============================================================================
// PHẦN 2: HARNESS ENGINEERING — ĐỊNH NGHĨA KHÔNG GIAN HÀNH ĐỘNG CỦA TÁC TỬ
// ============================================================================

#[derive(Debug, Clone, PartialEq)]
pub enum ToolResult {
    Finished(String),
    Failed(String),
}

/// Một CÔNG CỤ mà tác tử được phép gọi. Đây chính là "harness":
/// bạn định nghĩa tác tử ĐƯỢC LÀM GÌ, và mọi thứ khác đều bị cấm.
pub trait Tool {
    fn name(&self) -> &str;
    fn description(&self) -> &str;
    fn run(&self, param: &str) -> ToolResult;
}

pub struct CalculatorTool;
impl Tool for CalculatorTool {
    fn name(&self) -> &str {
        "sum_all"
    }
    fn description(&self) -> &str {
        "Cộng các số cách nhau bởi dấu phẩy. Ví dụ: \"3,4,5\""
    }
    fn run(&self, param: &str) -> ToolResult {
        let mut sum: i64 = 0;
        for part in param.split(',') {
            let n = match part.trim().parse::<i64>() {
                Ok(n) => n,
                Err(_) => return ToolResult::Failed(format!("{:?} không phải số", part.trim())),
            };
            // Tràn số cũng phải là lỗi có nội dung, không được panic
            sum = match sum.checked_add(n) {
                Some(s) => s,
                None => return ToolResult::Failed("Tổng bị tràn số i64".to_string()),
            };
        }
        ToolResult::Finished(sum.to_string())
    }
}

pub struct LookupTool {
    pub store: HashMap<String, String>,
}
impl Tool for LookupTool {
    fn name(&self) -> &str {
        "lookup"
    }
    fn description(&self) -> &str {
        "Tra cứu định nghĩa một thuật ngữ trong kho tri thức."
    }
    fn run(&self, param: &str) -> ToolResult {
        match self.store.get(param.trim()) {
            Some(v) => ToolResult::Finished(v.clone()),
            None => ToolResult::Failed(format!("Không tìm thấy {:?}", param.trim())),
        }
    }
}

/// Bộ khung (harness) giữ danh mục công cụ và ÁP ĐẶT GIỚI HẠN.
pub struct Harness {
    tools: Vec<Box<dyn Tool>>,
    pub max_calls: usize,
}

impl Harness {
    pub fn new(max_calls: usize) -> Self {
        Harness {
            tools: Vec::new(),
            max_calls,
        }
    }
    pub fn register(mut self, tool: Box<dyn Tool>) -> Self {
        self.tools.push(tool);
        self
    }
    /// Bản mô tả công cụ để nhét vào ngữ cảnh — đây là "giao diện" tác tử nhìn thấy.
    pub fn tool_catalog(&self) -> String {
        self.tools
            .iter()
            .map(|c| format!("- {}: {}", c.name(), c.description()))
            .collect::<Vec<_>>()
            .join("\n")
    }
    pub fn call(&self, name: &str, param: &str) -> ToolResult {
        match self.tools.iter().find(|c| c.name() == name) {
            Some(c) => c.run(param),
            None => ToolResult::Failed(format!("Công cụ {:?} không tồn tại trong bộ khung", name)),
        }
    }
}

// ============================================================================
// PHẦN 3: LOOP ENGINEERING — VÒNG LẶP TÁC TỬ CÓ ĐIỀU KIỆN DỪNG
// ============================================================================

#[derive(Debug, Clone, PartialEq)]
pub enum Action {
    CallTool { name: String, param: String },
    Answer(String),
}

/// Bộ não của tác tử. Trong thực tế đây là lời gọi tới mô hình ngôn ngữ;
/// ở đây ta dùng một bản GIẢ TẤT ĐỊNH để chương trình kiểm thử được.
pub trait Brain {
    fn decide(&self, task: &str, history: &[String]) -> Action;
}

#[derive(Debug, PartialEq)]
pub enum StopReason {
    Done,
    OutOfCalls,
    LoopDetected,
}

#[derive(Debug, PartialEq)]
pub struct LoopResult {
    pub answer: Option<String>,
    pub num_steps: usize,
    pub stop_reason: StopReason,
    pub log: Vec<String>,
}

/// LOOP ENGINEERING: vòng lặp tác tử với BA điều kiện dừng bắt buộc.
/// Một vòng lặp thiếu điều kiện dừng là một hóa đơn API không giới hạn.
pub fn run_agent_loop(task: &str, brain: &dyn Brain, harness: &Harness) -> LoopResult {
    let mut history: Vec<String> = Vec::new();
    let mut seen: HashSet<String> = HashSet::new();

    for step in 1..=harness.max_calls {
        match brain.decide(task, &history) {
            Action::Answer(t) => {
                history.push(format!("[{}] TRẢ LỜI: {}", step, t));
                return LoopResult {
                    answer: Some(t),
                    num_steps: step,
                    stop_reason: StopReason::Done,
                    log: history,
                };
            }
            Action::CallTool { name, param } => {
                // DỪNG #3: phát hiện lặp vô hạn (lặp lại y hệt một lời gọi đã có)
                let call_key = format!("{}::{}", name, param);
                if !seen.insert(call_key.clone()) {
                    history.push(format!("[{}] PHÁT HIỆN LẶP: {}", step, call_key));
                    return LoopResult {
                        answer: None,
                        num_steps: step,
                        stop_reason: StopReason::LoopDetected,
                        log: history,
                    };
                }
                let result = harness.call(&name, &param);
                history.push(match result {
                    ToolResult::Finished(v) => format!("[{}] {}({}) -> {}", step, name, param, v),
                    ToolResult::Failed(e) => {
                        format!("[{}] {}({}) -> LỖI: {}", step, name, param, e)
                    }
                });
            }
        }
    }
    // DỪNG #2: hết ngân sách lượt gọi
    LoopResult {
        answer: None,
        num_steps: harness.max_calls,
        stop_reason: StopReason::OutOfCalls,
        log: history,
    }
}

// ============================================================================
// PHẦN 4: GRAPH ENGINEERING — ĐỒ THỊ TRI THỨC & TRUY XUẤT NHIỀU BƯỚC
// ============================================================================

/// Đồ thị tri thức: các thực thể nối với nhau bằng quan hệ có nhãn.
/// Đây là nền của GraphRAG — truy xuất theo QUAN HỆ, không chỉ theo từ khóa.
pub struct KnowledgeGraph {
    edge: HashMap<String, Vec<(String, String)>>, // đỉnh -> [(nhãn quan hệ, đỉnh đích)]
    description: HashMap<String, String>,
}

impl Default for KnowledgeGraph {
    fn default() -> Self {
        Self::new()
    }
}

impl KnowledgeGraph {
    pub fn new() -> Self {
        KnowledgeGraph {
            edge: HashMap::new(),
            description: HashMap::new(),
        }
    }
    pub fn add_entity(&mut self, name: &str, description: &str) {
        self.description
            .insert(name.to_string(), description.to_string());
        self.edge.entry(name.to_string()).or_default();
    }
    pub fn add_relation(&mut self, from: &str, label: &str, target: &str) {
        self.edge
            .entry(from.to_string())
            .or_default()
            .push((label.to_string(), target.to_string()));
    }

    /// Truy xuất nhiều bước: từ một điểm xuất phát, đi tối đa `depth` bước
    /// để gom ngữ cảnh liên quan. Đây là điểm khác biệt so với tìm kiếm phẳng.
    pub fn spread_retrieve(&self, start: &str, depth: usize) -> Vec<String> {
        let mut results = Vec::new();
        let mut visited: HashSet<String> = HashSet::new();
        let mut queue: VecDeque<(String, usize)> = VecDeque::new();

        queue.push_back((start.to_string(), 0));
        visited.insert(start.to_string());

        while let Some((node, dist)) = queue.pop_front() {
            if let Some(m) = self.description.get(&node) {
                results.push(format!("{}: {}", node, m));
            }
            if dist >= depth {
                continue;
            }
            if let Some(neighbors) = self.edge.get(&node) {
                let mut sorted = neighbors.clone();
                sorted.sort(); // tất định
                for (label, target) in sorted {
                    if visited.insert(target.clone()) {
                        results.push(format!("  ({} --{}--> {})", node, label, target));
                        queue.push_back((target, dist + 1));
                    }
                }
            }
        }
        results
    }
}

// ============================================================================
// PHẦN 5: BỘ NÃO GIẢ TẤT ĐỊNH (test double cho mô hình ngôn ngữ)
// ============================================================================

/// Bộ não giả: quyết định dựa trên luật cố định, nên chương trình TẤT ĐỊNH
/// và kiểm thử được — không cần khóa API, không cần mạng.
pub struct FakeBrain {
    pub scenarios: Vec<Action>,
}
impl Brain for FakeBrain {
    fn decide(&self, _task: &str, history: &[String]) -> Action {
        self.scenarios
            .get(history.len())
            .cloned()
            .unwrap_or_else(|| Action::Answer("Hết kịch bản".to_string()))
    }
}

fn main() {
    println!("═══════════════════════════════════════════════════════════════");
    println!("   KỸ NGHỆ NGỮ CẢNH · BỘ KHUNG · VÒNG LẶP · ĐỒ THỊ TRI THỨC    ");
    println!("═══════════════════════════════════════════════════════════════");

    // ---- 1. CONTEXT ENGINEERING ----
    println!("\n1. KỸ NGHỆ NGỮ CẢNH — nhồi 4000 token vào cửa sổ 1000 token");
    let chunks = vec![
        ContextChunk {
            label: "safety_rules".into(),
            content: "Không tiết lộ khóa bí mật".into(),
            token: 50,
            relevance: 0.3,
            pinned: true,
        },
        ContextChunk {
            label: "doc_A".into(),
            content: "...".into(),
            token: 800,
            relevance: 0.9,
            pinned: false,
        },
        ContextChunk {
            label: "doc_B".into(),
            content: "...".into(),
            token: 200,
            relevance: 0.85,
            pinned: false,
        },
        ContextChunk {
            label: "doc_C".into(),
            content: "...".into(),
            token: 2000,
            relevance: 0.95,
            pinned: false,
        },
        ContextChunk {
            label: "old_chat_history".into(),
            content: "...".into(),
            token: 900,
            relevance: 0.1,
            pinned: false,
        },
    ];
    let pack = pack_context(chunks, 1000);
    println!(
        "   Dùng {} / 1000 token, loại bỏ {} mẩu",
        pack.total_tokens, pack.dropped_count
    );
    for m in &pack.chunks {
        println!(
            "     [{:>4} tok · lq {:.2}{}] {}",
            m.token,
            m.relevance,
            if m.pinned { " · GHIM" } else { "" },
            m.label
        );
    }
    println!("   → doc_C (2000 tok) bị loại dù liên quan cao nhất: KHÔNG VỪA ngân sách.");
    println!("   → Thứ tự đã đảo để mẩu quan trọng nằm ở ĐẦU và CUỐI (chống Lost-in-the-Middle).");
    println!("   → Tham lam theo mật độ KHÔNG tối ưu: doc_A (800 tok, lq 0.9) cũng bị loại,");
    println!(
        "     dù safety_rules + doc_A = 850 tok vẫn vừa — xếp ba lô chính xác cần quy hoạch động."
    );

    // ---- 2 & 3. HARNESS + LOOP ----
    println!("\n2-3. BỘ KHUNG & VÒNG LẶP TÁC TỬ");
    let mut store = HashMap::new();
    store.insert(
        "Rust".to_string(),
        "Ngôn ngữ hệ thống an toàn bộ nhớ".to_string(),
    );
    let harness = Harness::new(5)
        .register(Box::new(CalculatorTool))
        .register(Box::new(LookupTool { store }));
    println!(
        "   Công cụ tác tử được phép dùng:\n{}",
        harness.tool_catalog()
    );

    let brain = FakeBrain {
        scenarios: vec![
            Action::CallTool {
                name: "lookup".into(),
                param: "Rust".into(),
            },
            Action::CallTool {
                name: "sum_all".into(),
                param: "10,20,12".into(),
            },
            Action::Answer("Rust là ngôn ngữ hệ thống; tổng là 42.".into()),
        ],
    };
    let result = run_agent_loop("Tra cứu Rust rồi cộng 10+20+12", &brain, &harness);
    for d in &result.log {
        println!("   {}", d);
    }
    println!(
        "   Dừng vì: {:?} sau {} bước",
        result.stop_reason, result.num_steps
    );

    // Vòng lặp hỏng: tác tử lặp mãi một lời gọi
    let stuck_brain = FakeBrain {
        scenarios: vec![
            Action::CallTool {
                name: "lookup".into(),
                param: "X".into(),
            },
            Action::CallTool {
                name: "lookup".into(),
                param: "X".into(),
            },
        ],
    };
    let result2 = run_agent_loop("nhiệm vụ hỏng", &stuck_brain, &harness);
    println!(
        "   [Tác tử kẹt] dừng vì: {:?} sau {} bước",
        result2.stop_reason, result2.num_steps
    );

    // ---- 4. GRAPH ENGINEERING ----
    println!("\n4. ĐỒ THỊ TRI THỨC — truy xuất lan tỏa 2 bước");
    let mut g = KnowledgeGraph::new();
    g.add_entity("Order", "Đơn hàng của khách");
    g.add_entity("Customer", "Người mua");
    g.add_entity("Payment", "Giao dịch trừ tiền");
    g.add_entity("Shipment", "Phiếu giao hàng");
    g.add_entity("Warehouse", "Kho hàng vật lý");
    g.add_relation("Order", "belongs_to", "Customer");
    g.add_relation("Order", "paid_by", "Payment");
    g.add_relation("Order", "creates", "Shipment");
    g.add_relation("Shipment", "ships_from", "Warehouse");
    for line in g.spread_retrieve("Order", 2) {
        println!("   {}", line);
    }
    println!("   → Tìm kiếm từ khóa thường sẽ BỎ SÓT \"Warehouse\" (kho) vì nó cách 2 bước.");

    println!("\n═══════════════════════════════════════════════════════════════");
    println!("  NGỮ CẢNH LÀ TÀI NGUYÊN · VÒNG LẶP PHẢI CÓ PHANH · CÔNG CỤ LÀ HỢP ĐỒNG ");
    println!("═══════════════════════════════════════════════════════════════");
}

#[cfg(test)]
mod tests {
    use super::*;

    fn chunk(label: &str, token: usize, lq: f64, pinned: bool) -> ContextChunk {
        ContextChunk {
            label: label.into(),
            content: "x".into(),
            token,
            relevance: lq,
            pinned,
        }
    }

    #[test]
    fn context_never_exceeds_budget() {
        let list = vec![
            chunk("a", 400, 0.9, false),
            chunk("b", 400, 0.8, false),
            chunk("c", 400, 0.7, false),
        ];
        let g = pack_context(list, 1000);
        assert!(g.total_tokens <= 1000, "vượt ngân sách: {}", g.total_tokens);
        assert_eq!(g.chunks.len(), 2);
    }

    #[test]
    fn pinned_items_are_always_kept() {
        let list = vec![
            chunk("rules", 100, 0.01, true), // liên quan cực thấp nhưng GHIM
            chunk("large", 900, 0.99, false),
        ];
        let g = pack_context(list, 1000);
        assert!(
            g.chunks.iter().any(|m| m.label == "rules"),
            "mẩu ghim bị loại!"
        );
    }

    #[test]
    fn pinned_rules_stay_at_the_front() {
        // Bản cũ xếp cả mẩu ghim theo độ liên quan, đẩy quy tắc an toàn vào GIỮA
        let list = vec![
            chunk("rules", 10, 0.1, true),
            chunk("a", 10, 0.9, false),
            chunk("b", 10, 0.8, false),
            chunk("c", 10, 0.5, false),
        ];
        let pack = pack_context(list, 100);
        assert_eq!(pack.chunks[0].label, "rules");
        assert_eq!(pack.chunks[1].label, "a");
        assert_eq!(pack.chunks.last().unwrap().label, "b");
    }

    #[test]
    fn calculator_overflow_is_an_error_not_a_panic() {
        let input = format!("{},1", i64::MAX);
        assert!(matches!(CalculatorTool.run(&input), ToolResult::Failed(_)));
    }

    #[test]
    fn ranks_by_value_density_not_raw_score() {
        // "small" có điểm thấp hơn nhưng mật độ (lq/token) cao hơn nhiều
        let list = vec![
            chunk("large", 900, 0.9, false),
            chunk("small", 90, 0.5, false),
        ];
        let g = pack_context(list, 500);
        assert_eq!(g.chunks.len(), 1);
        assert_eq!(g.chunks[0].label, "small");
    }

    #[test]
    fn anti_forgetting_puts_key_items_at_both_ends() {
        let list = vec![
            chunk("a", 1, 0.9, false),
            chunk("b", 1, 0.5, false),
            chunk("c", 1, 0.8, false),
        ];
        let sorted = lost_in_middle_order(list);
        // xếp giảm dần: a(.9) c(.8) b(.5) -> chẵn ra đầu, lẻ ra cuối (đảo): a, b, c
        assert_eq!(sorted.first().unwrap().label, "a");
        assert_eq!(sorted.last().unwrap().label, "c");
    }

    #[test]
    fn tool_answers_correctly_and_errors_clearly() {
        let tool = CalculatorTool;
        assert_eq!(tool.run("1,2,3"), ToolResult::Finished("6".into()));
        assert!(matches!(tool.run("1,x"), ToolResult::Failed(_)));
    }

    #[test]
    fn harness_rejects_unregistered_tools() {
        let harness = Harness::new(3).register(Box::new(CalculatorTool));
        // Tác tử KHÔNG THỂ gọi thứ không được đăng ký — đây là ranh giới an toàn.
        assert!(matches!(
            harness.call("format_disk", "/"),
            ToolResult::Failed(_)
        ));
    }

    #[test]
    fn loop_stops_on_completion() {
        let harness = Harness::new(5).register(Box::new(CalculatorTool));
        let brain = FakeBrain {
            scenarios: vec![
                Action::CallTool {
                    name: "sum_all".into(),
                    param: "40,2".into(),
                },
                Action::Answer("42".into()),
            ],
        };
        let result = run_agent_loop("nv", &brain, &harness);
        assert_eq!(result.stop_reason, StopReason::Done);
        assert_eq!(result.answer, Some("42".to_string()));
        assert_eq!(result.num_steps, 2);
    }

    #[test]
    fn loop_stops_when_out_of_calls() {
        let harness = Harness::new(3).register(Box::new(CalculatorTool));
        // Bộ não không bao giờ trả lời, chỉ gọi công cụ với tham số KHÁC nhau
        let brain = FakeBrain {
            scenarios: vec![
                Action::CallTool {
                    name: "sum_all".into(),
                    param: "1".into(),
                },
                Action::CallTool {
                    name: "sum_all".into(),
                    param: "2".into(),
                },
                Action::CallTool {
                    name: "sum_all".into(),
                    param: "3".into(),
                },
                Action::CallTool {
                    name: "sum_all".into(),
                    param: "4".into(),
                },
            ],
        };
        let result = run_agent_loop("nv", &brain, &harness);
        assert_eq!(result.stop_reason, StopReason::OutOfCalls);
        assert_eq!(result.num_steps, 3, "phải dừng đúng ở ngân sách 3 lượt");
    }

    #[test]
    fn loop_detects_stuck_agent() {
        let harness = Harness::new(50).register(Box::new(CalculatorTool));
        let brain = FakeBrain {
            scenarios: vec![
                Action::CallTool {
                    name: "sum_all".into(),
                    param: "1".into(),
                },
                Action::CallTool {
                    name: "sum_all".into(),
                    param: "1".into(),
                }, // y hệt
            ],
        };
        let result = run_agent_loop("nv", &brain, &harness);
        assert_eq!(result.stop_reason, StopReason::LoopDetected);
        assert!(
            result.num_steps < 50,
            "phải dừng SỚM, không chạy hết 50 lượt"
        );
    }

    #[test]
    fn graph_retrieval_respects_depth() {
        let mut g = KnowledgeGraph::new();
        g.add_entity("A", "a");
        g.add_entity("B", "b");
        g.add_entity("C", "c");
        g.add_entity("D", "d");
        g.add_relation("A", "r1", "B");
        g.add_relation("B", "r2", "C");
        g.add_relation("C", "r3", "D");

        let depth1 = g.spread_retrieve("A", 1);
        assert!(depth1.iter().any(|s| s.starts_with("B:")));
        assert!(
            !depth1.iter().any(|s| s.starts_with("C:")),
            "độ sâu 1 không được tới C"
        );

        let depth2 = g.spread_retrieve("A", 2);
        assert!(
            depth2.iter().any(|s| s.starts_with("C:")),
            "độ sâu 2 phải tới được C"
        );
        assert!(!depth2.iter().any(|s| s.starts_with("D:")));
    }

    #[test]
    fn graph_walk_terminates_on_cycles() {
        let mut g = KnowledgeGraph::new();
        g.add_entity("A", "a");
        g.add_entity("B", "b");
        g.add_relation("A", "r", "B");
        g.add_relation("B", "r", "A"); // chu trình
        let result = g.spread_retrieve("A", 10);
        assert!(
            result.len() < 10,
            "phải dừng nhờ tập đã thăm, không lặp vô hạn"
        );
    }
}
```

---

## Từ mô hình giả tới mô hình thật

Mã trên dùng `FakeBrain` để mọi thứ tất định và kiểm thử được. Khi nối vào mô hình thật, bạn **chỉ thay đúng một cài đặt trait**:

```rust
pub struct RealBrain { pub api_key: String, pub model: String }

impl Brain for RealBrain {
    fn decide(&self, task: &str, history: &[String]) -> Action {
        // 1. Dựng ngữ cảnh bằng `pack_context` (tôn trọng ngân sách token)
        // 2. Gửi HTTP tới nhà cung cấp mô hình (reqwest + serde_json)
        // 3. Phân tích phản hồi thành Action::CallTool hoặc Action::Answer
        todo!("gọi mô hình thật")
    }
}
```

Toàn bộ phần còn lại — bộ khung, vòng lặp, đồ thị, và **tất cả bài kiểm thử** — giữ nguyên không đổi. Đó chính là lợi ích của việc đặt ranh giới bằng `trait` (Chương 12) và tiêm phụ thuộc (Chương 14).

**Hệ sinh thái Rust cho AI** đáng theo dõi:

| Crate | Vai trò |
|---|---|
| [`rig`](https://github.com/0xPlaygrounds/rig) | Khung xây tác tử LLM: nhà cung cấp mô hình, công cụ, RAG, kho vector |
| `async-openai` và các client cộng đồng khác | Client cho từng nhà cung cấp (kiểm tra tình trạng bảo trì trước khi dùng) |
| `qdrant-client`, `lancedb` | Kho vector cho truy xuất theo độ tương tự |
| `tiktoken-rs` | Đếm token chính xác — cần cho `pack_context` phiên bản thật |
| `tokio` + `reqwest` | Bất đồng bộ và HTTP (Chương 49) |

> **Vì sao dùng Rust cho tác tử AI?** Ba lý do rất thực tế: (1) một tiến trình tác tử Rust thường chỉ tốn vài chục MB RAM thay vì hàng trăm MB, quan trọng khi chạy hàng nghìn tác tử song song (Chương 48); (2) hệ thống kiểu biến "công cụ" thành hợp đồng kiểm tra được lúc biên dịch, thay vì dictionary lỏng lẻo; (3) `tokio` cho phép chạy hàng nghìn tác tử đồng thời trên một máy (Chương 49).

---

## Bảng tra cứu lỗi biên dịch thường gặp

| Lỗi | Nguyên nhân trong chương này | Cách sửa |
|---|---|---|
| `` E0038: the trait `Tool` is not dyn compatible `` | `Box<dyn Tool>` nhưng trait có phương thức generic hoặc trả `Self` | Giữ trait tương thích `dyn` (trước gọi là "object-safe"): bỏ generic khỏi phương thức, trả `Box<dyn ...>` thay vì `Self`, hoặc gắn `where Self: Sized` cho phương thức đó |
| `` E0277: the size for values of type `(dyn Tool + 'static)` cannot be known at compilation time `` | Chứa `dyn Tool` trực tiếp trong `Vec` | `Vec<Box<dyn Tool>>` — trait object không có kích thước biết trước |
| `E0502: cannot borrow as mutable` | Vừa duyệt danh mục công cụ vừa muốn thêm kết quả vào nó | Thu kết quả vào `Vec` cục bộ, gộp lại sau vòng lặp |
| `E0716: temporary value dropped while borrowed` | Mượn kết quả của một biểu thức tạm khi dựng ngữ cảnh | Gán vào biến `let` trước rồi mới mượn |
| Ngân sách ngữ cảnh vượt hạn mà không báo | Cộng token sau khi đã thêm vào danh sách | Kiểm ngân sách **trước** khi thêm, không phải sau |

---

## Tóm tắt chương & Bài tập rèn luyện (Summary & Exercises)

### 4 Điểm cốt lõi cần ghi nhớ:
1. **Ngữ cảnh là tài nguyên có hạn** — bài toán chọn ngữ cảnh là bài toán xếp ba lô, ưu tiên theo *mật độ giá trị* chứ không theo điểm thô. Ghim cứng những gì không được phép mất.
2. **Bộ khung là hợp đồng kiểu**: tác tử chỉ làm được những gì `trait Tool` cho phép. Danh sách trắng, lỗi có nội dung, mô tả rõ ràng.
3. **Vòng lặp phải có ba cái phanh**: hoàn thành, hết ngân sách, phát hiện lặp. Thiếu cái thứ ba là thiếu cái quan trọng nhất.
4. **Đồ thị tri thức giải được lớp câu hỏi nhiều bước** mà tìm kiếm theo độ tương tự bó tay — nhưng bắt buộc phải giới hạn độ sâu và giữ tập đã thăm.

### Bài tập rèn luyện tự giải:

**Bài tập 1 (Công cụ mới trong bộ khung)**
Viết `WeatherTool` trả về nhiệt độ cho một thành phố từ một `HashMap` cố định, và trả lỗi rõ ràng cho thành phố không có. Đăng ký vào `Harness` rồi viết test chứng minh tác tử gọi được nó.

<details>
<summary><b>Lời giải</b></summary>

```rust
pub struct WeatherTool { pub data: HashMap<String, i32> }

impl Tool for WeatherTool {
    fn name(&self) -> &str { "weather" }
    fn description(&self) -> &str { "Trả về nhiệt độ (°C) của một thành phố. Ví dụ: \"Hà Nội\"" }
    fn run(&self, param: &str) -> ToolResult {
        match self.data.get(param.trim()) {
            Some(t) => ToolResult::Finished(format!("{}°C", t)),
            None => ToolResult::Failed(format!("Chưa có dữ liệu cho {:?}", param.trim())),
        }
    }
}

#[cfg(test)]
mod exercise_1 {
    use super::*;
    #[test]
    fn agent_can_call_weather_tool() {
        let mut data = HashMap::new();
        data.insert("Hà Nội".to_string(), 28);
        let harness = Harness::new(3).register(Box::new(WeatherTool { data }));
        assert_eq!(harness.call("weather", "Hà Nội"), ToolResult::Finished("28°C".into()));
        assert!(matches!(harness.call("weather", "Sao Hỏa"), ToolResult::Failed(_)));
    }
}
```
</details>

**Bài tập 2 (Phanh thứ tư: ngân sách token)**
Thêm vào `Harness` một trường `max_tokens: usize` và vào `StopReason` một biến thể `StopReason::OutOfTokens`. Mỗi lượt gọi công cụ cộng dồn độ dài kết quả vào bộ đếm; vượt ngưỡng thì dừng.

<details>
<summary><b>Gợi ý</b></summary>

Đây là *cái phanh mà các đội thực chiến hay quên nhất*: tác tử có thể dừng đúng 5 lượt nhưng mỗi lượt kéo về 100.000 token. Đếm lượt là chưa đủ, phải đếm cả **khối lượng**.
</details>

<details>
<summary><b>Lời giải</b></summary>

```rust
// Thêm vào enum:  StopReason::OutOfTokens
// Trong run_agent_loop, sau mỗi lời gọi công cụ:
//     used_tokens += result_text.len() / 4;   // xấp xỉ: 4 ký tự ~ 1 token
//     if used_tokens > harness.max_tokens {
//         return LoopResult { answer: None, num_steps: step,
//                                stop_reason: StopReason::OutOfTokens, log: history };
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
