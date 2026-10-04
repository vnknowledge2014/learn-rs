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
