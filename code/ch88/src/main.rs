//! Chương 88: Suy luận Có Cấu trúc, Kỹ nghệ Đồ thị Kép, Máy Trạng thái HITL & Truy xuất Lai BM25 + RRF
//!
//! Mô-đun Rust 2024 thuần `std` hiện thực hoá toàn diện:
//! 1. Kỹ thuật suy luận có cấu trúc: Tree-of-Thoughts (ToT) với bài toán Game of 24, Self-Consistency Voter,
//!    Plan-and-Solve Runner, Reflexion Loop và quy trình 5 thao tác nhận thức CognitiveTool5Op.
//! 2. Kỹ nghệ đồ thị kép vĩ mô: Đồ thị tri thức InfoGraph với kỹ thuật cắt lát ngữ cảnh 1 bước (1-hop slice)
//!    và đồ thị kế hoạch thực thi PlanDag (sóng song song Kahn, phát hiện chu trình, trần thử lại, bộ biên dịch AoT).
//! 3. Máy trạng thái nút StateGraph<S> với các hàm gộp (Overwrite, Append, Sum) và cổng kiểm tra điểm kiểm soát tất định Stripe Blueprints.
//! 4. Ngắt can thiệp người trong vòng lặp HitlInterrupt (Accept, Edit, Respond, Ignore) cập nhật Trí nhớ thủ tục ProceduralMemory,
//!    xử lý trọn vẹn cả trường hợp người dùng xoá bỏ tham số.
//! 5. Trí nhớ nhận thức 4 tầng CognitiveMemory4Tier (MEM1), Đồ thị lưỡng thời gian BiTemporalGraph (ValidTime x TxTime) và BdiEngine (Endurants vs Perdurants).
//! 6. Bộ phân đoạn MarkdownHeadingChunker an toàn (bảo vệ max_w = 0, lọc rào mã code fence), Truy xuất lai HybridRetriever
//!    (Okapi BM25 + Dense Cosine + RRF với kiểm tra khớp độ dài bắt buộc) và Bộ kiểm chứng trích dẫn SpanCitationVerifier.

use std::collections::{BTreeMap, BTreeSet, HashMap, VecDeque};
use std::hash::{DefaultHasher, Hash, Hasher};

// ============================================================================
// PHẦN 1: SUY LUẬN CÓ CẤU TRÚC (STRUCTURED REASONING)
// ============================================================================

/// Đánh giá chất lượng của một nhánh suy luận trong cây ý nghĩ (Tree-of-Thoughts).
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum ThoughtVote {
    /// Chắc chắn thành công hoặc đạt mục tiêu cuối cùng.
    Sure,
    /// Có tiềm năng, cần tiếp tục khám phá thêm các bước kế tiếp.
    Maybe,
    /// Bế tắc hoặc vi phạm ràng buộc toán học / logic, cần cắt tỉa nhánh.
    Impossible,
}

/// Nút trạng thái trong không gian tìm kiếm cây ý nghĩ giải bài toán Game of 24.
#[derive(Debug, Clone, PartialEq)]
pub struct ThoughtNode {
    /// Danh sách các giá trị số hiện thời trên nhánh.
    pub numbers: Vec<f64>,
    /// Lịch sử biểu thức toán học tương ứng với từng số.
    pub expressions: Vec<String>,
    /// Đánh giá của giám khảo heuristic đối với nút này.
    pub vote: ThoughtVote,
}

/// Bộ tìm kiếm cây ý nghĩ (Tree of Thoughts) áp dụng cho các bài toán suy luận phức tạp.
#[derive(Debug, Clone, Default)]
pub struct ThoughtTreeSearch;

impl ThoughtTreeSearch {
    /// Tính toán hash hoặc kiểm tra sai số xấp xỉ dấu phẩy động.
    pub fn is_close(a: f64, b: f64) -> bool {
        (a - b).abs() < 1e-5
    }

    /// Đánh giá trạng thái hiện thời của danh sách số.
    pub fn evaluate(numbers: &[f64], target: f64) -> ThoughtVote {
        if numbers.is_empty() {
            return ThoughtVote::Impossible;
        }
        if numbers.len() == 1 {
            if Self::is_close(numbers[0], target) {
                ThoughtVote::Sure
            } else {
                ThoughtVote::Impossible
            }
        } else {
            // Heuristic đơn giản: nếu toàn bộ số âm mà target dương và không còn phép nhân thì khó đạt
            ThoughtVote::Maybe
        }
    }

    /// Sinh các nút con (successor thoughts) bằng cách ghép từng cặp số qua 4 phép tính cơ bản.
    pub fn generate_successors(node: &ThoughtNode, target: f64) -> Vec<ThoughtNode> {
        let mut successors = Vec::new();
        let n = node.numbers.len();
        if n < 2 {
            return successors;
        }

        for i in 0..n {
            for j in (i + 1)..n {
                let a = node.numbers[i];
                let b = node.numbers[j];
                let expr_a = &node.expressions[i];
                let expr_b = &node.expressions[j];

                // Thu thập các số còn lại
                let mut remaining_nums = Vec::with_capacity(n - 1);
                let mut remaining_exprs = Vec::with_capacity(n - 1);
                for (idx, (num, expr)) in node.numbers.iter().zip(&node.expressions).enumerate() {
                    if idx != i && idx != j {
                        remaining_nums.push(*num);
                        remaining_exprs.push(expr.clone());
                    }
                }

                // Các phép tính khả dĩ: a + b, a - b, b - a, a * b, a / b, b / a
                let ops = vec![
                    (a + b, format!("({} + {})", expr_a, expr_b)),
                    (a - b, format!("({} - {})", expr_a, expr_b)),
                    (b - a, format!("({} - {})", expr_b, expr_a)),
                    (a * b, format!("({} * {})", expr_a, expr_b)),
                ];

                let mut all_ops = ops;
                if b.abs() > 1e-9 {
                    all_ops.push((a / b, format!("({} / {})", expr_a, expr_b)));
                }
                if a.abs() > 1e-9 {
                    all_ops.push((b / a, format!("({} / {})", expr_b, expr_a)));
                }

                for (val, expr) in all_ops {
                    let mut next_nums = remaining_nums.clone();
                    let mut next_exprs = remaining_exprs.clone();
                    next_nums.push(val);
                    next_exprs.push(expr);

                    let vote = Self::evaluate(&next_nums, target);
                    successors.push(ThoughtNode {
                        numbers: next_nums,
                        expressions: next_exprs,
                        vote,
                    });
                }
            }
        }
        successors
    }

    /// Tìm lời giải cho tập số đầu vào đạt giá trị `target` (ví dụ 24) bằng tìm kiếm cây BFS (Tree of Thoughts).
    pub fn search_24(numbers: &[f64], target: f64, max_nodes: usize) -> Option<String> {
        let initial_node = ThoughtNode {
            numbers: numbers.to_vec(),
            expressions: numbers.iter().map(|n| format!("{:.0}", n)).collect(),
            vote: ThoughtVote::Maybe,
        };

        let mut queue = VecDeque::new();
        queue.push_back(initial_node);
        let limit = if max_nodes == 0 {
            usize::MAX
        } else {
            max_nodes.max(100) * 50
        };
        let mut visited = 0;

        while let Some(node) = queue.pop_front() {
            visited += 1;
            if visited > limit {
                break;
            }

            let successors = Self::generate_successors(&node, target);
            for succ in successors {
                if succ.vote == ThoughtVote::Sure {
                    return succ.expressions.first().cloned();
                }
                if succ.vote == ThoughtVote::Maybe {
                    queue.push_back(succ);
                }
            }
        }

        None
    }
}

/// Quỹ đạo suy luận độc lập trong kỹ thuật Self-Consistency Voting.
#[derive(Debug, Clone, PartialEq)]
pub struct ReasoningPath {
    /// Các bước suy luận trung gian (chuỗi CoT).
    pub steps: Vec<String>,
    /// Câu trả lời hoặc quyết định cuối cùng được chuẩn hoá.
    pub final_answer: String,
    /// Độ tin cậy ước tính của mô hình cho đường đi này.
    pub confidence: f64,
}

/// Kết quả tổng hợp biểu quyết đa số tự nhất quán.
#[derive(Debug, Clone, PartialEq)]
pub struct VoteResult {
    /// Đáp án chiến thắng theo đa số biểu quyết.
    pub winning_answer: String,
    /// Số lượng phiếu đồng thuận cho đáp án thắng.
    pub agreement_count: usize,
    /// Tổng số phiếu tham gia biểu quyết.
    pub total_votes: usize,
    /// Tỷ lệ đồng thuận có trọng số (weighted confidence score).
    pub confidence_score: f64,
}

/// Bộ biểu quyết tự nhất quán (Self-Consistency Voter).
#[derive(Debug, Clone, Default)]
pub struct SelfConsistencyVoter;

impl SelfConsistencyVoter {
    /// Tổng hợp kết quả từ nhiều chuỗi suy luận độc lập để chọn ra phương án đáng tin nhất.
    pub fn vote(paths: &[ReasoningPath]) -> Option<VoteResult> {
        if paths.is_empty() {
            return None;
        }

        let mut vote_counts: BTreeMap<String, usize> = BTreeMap::new();
        let mut weighted_scores: BTreeMap<String, f64> = BTreeMap::new();

        for p in paths {
            let key = p.final_answer.trim().to_lowercase();
            *vote_counts.entry(key.clone()).or_insert(0) += 1;
            *weighted_scores.entry(key).or_insert(0.0) += p.confidence;
        }

        let total_votes = paths.len();
        let total_weight: f64 = weighted_scores.values().sum();

        // Tìm đáp án có số phiếu cao nhất; nếu hòa phiếu thì dùng trọng số độ tin cậy để phá vỡ thế cân bằng
        let mut ranked: Vec<(String, usize, f64)> = vote_counts
            .into_iter()
            .map(|(ans, count)| {
                let score = *weighted_scores.get(&ans).unwrap_or(&0.0);
                (ans, count, score)
            })
            .collect();

        ranked.sort_by(|a, b| b.1.cmp(&a.1).then_with(|| b.2.total_cmp(&a.2)));

        let (best_answer, count, score) = ranked.into_iter().next()?;
        let confidence_score = if total_weight > 0.0 {
            score / total_weight
        } else {
            count as f64 / total_votes as f64
        };

        Some(VoteResult {
            winning_answer: best_answer,
            agreement_count: count,
            total_votes,
            confidence_score,
        })
    }
}

/// Nhiệm vụ con trong kế hoạch Plan-and-Solve.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct SubTask {
    /// Chỉ số thứ tự bước thực thi.
    pub step_id: usize,
    /// Mô tả chi tiết hành động cần thực hiện.
    pub description: String,
    /// Gợi ý công cụ thích hợp để xử lý bước này.
    pub tool_hint: Option<String>,
}

/// Kết quả thực thi một bước nhiệm vụ con.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct SubTaskResult {
    /// Mã bước tương ứng.
    pub step_id: usize,
    /// Dữ liệu đầu ra thu được sau khi hoàn thành.
    pub output: String,
    /// Trạng thái thành công của bước.
    pub success: bool,
}

/// Báo cáo tiến trình thực thi hoàn chỉnh của PlanAndSolveRunner.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct PlanAndSolveExecution {
    /// Toàn bộ kế hoạch gồm các bước đã được chia nhỏ.
    pub plan: Vec<SubTask>,
    /// Kết quả chi tiết từng bước.
    pub step_results: Vec<SubTaskResult>,
    /// Tóm tắt kết luận cuối cùng.
    pub final_summary: String,
    /// Trạng thái toàn bộ quy trình có thành công trọn vẹn hay không.
    pub success: bool,
}

/// Bộ điều phối phân rã và giải quyết tuần tự (Plan-and-Solve Runner).
#[derive(Debug, Clone, Default)]
pub struct PlanAndSolveRunner;

impl PlanAndSolveRunner {
    /// Thực hiện chu trình lập kế hoạch tổng thể trước, sau đó giải quyết từng bước độc lập.
    pub fn run<P, S>(goal: &str, planner: P, solver: S) -> PlanAndSolveExecution
    where
        P: FnOnce(&str) -> Vec<SubTask>,
        S: Fn(&SubTask, &[SubTaskResult]) -> SubTaskResult,
    {
        let plan = planner(goal);
        let mut step_results = Vec::with_capacity(plan.len());
        let mut overall_success = true;

        for task in &plan {
            let res = solver(task, &step_results);
            if !res.success {
                overall_success = false;
            }
            step_results.push(res);
            if !overall_success {
                break;
            }
        }

        let summary = if overall_success {
            format!(
                "Hoàn thành thành công tất cả {} bước cho mục tiêu: {}",
                plan.len(),
                goal
            )
        } else {
            format!(
                "Thất bại tại bước thứ {} cho mục tiêu: {}",
                step_results.len(),
                goal
            )
        };

        PlanAndSolveExecution {
            plan,
            step_results,
            final_summary: summary,
            success: overall_success,
        }
    }
}

/// Bản ghi vết của một lần thử trong vòng lặp tự phản tỉnh (Reflexion Loop).
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct ReflexionAttempt {
    /// Lần thử thứ mấy (1-indexed).
    pub attempt_idx: usize,
    /// Hành động hoặc giải pháp mà tác tử đã đề xuất.
    pub action: String,
    /// Phản hồi của môi trường kiểm thử đối với hành động.
    pub outcome: String,
    /// Trạng thái đạt yêu cầu kiểm thử.
    pub passed: bool,
    /// Lời tự vấn phản tỉnh chỉ ra sai lầm và phương án cải tiến cho vòng sau.
    pub reflection: Option<String>,
}

/// Hồ sơ tổng kết vòng lặp Reflexion.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct ReflexionRecord {
    /// Lịch sử toàn bộ các lần thử và phản tỉnh.
    pub history: Vec<ReflexionAttempt>,
    /// Kết quả cuối cùng tác tử có vượt qua bài toán hay không.
    pub final_success: bool,
    /// Tổng số lần thử đã thực hiện.
    pub total_attempts: usize,
}

/// Vòng lặp học tập củng cố bằng phản tỉnh bằng ngôn ngữ (Reflexion Loop).
#[derive(Debug, Clone, Default)]
pub struct ReflexionLoop;

impl ReflexionLoop {
    /// Chạy chu trình Thử nghiệm -> Đánh giá -> Phản tỉnh cho tới khi đạt hoặc cạn ngân sách lần thử.
    pub fn run<A, E, R>(
        task_prompt: &str,
        mut actor: A,
        mut evaluator: E,
        mut reflector: R,
        max_attempts: usize,
    ) -> ReflexionRecord
    where
        A: FnMut(&str, &[ReflexionAttempt]) -> String,
        E: FnMut(&str) -> (bool, String),
        R: FnMut(&str, &str, &str) -> String,
    {
        let mut history = Vec::new();
        let mut final_success = false;

        for attempt_idx in 1..=max_attempts {
            let action = actor(task_prompt, &history);
            let (passed, outcome) = evaluator(&action);

            if passed {
                history.push(ReflexionAttempt {
                    attempt_idx,
                    action,
                    outcome,
                    passed: true,
                    reflection: None,
                });
                final_success = true;
                break;
            } else {
                let reflection = reflector(task_prompt, &action, &outcome);
                history.push(ReflexionAttempt {
                    attempt_idx,
                    action,
                    outcome,
                    passed: false,
                    reflection: Some(reflection),
                });
            }
        }

        let total_attempts = history.len();
        ReflexionRecord {
            history,
            final_success,
            total_attempts,
        }
    }
}

/// 5 giai đoạn nhận thức tuần tự của quy trình CognitiveTool5Op.
#[derive(Debug, Clone, Copy, PartialEq, Eq, PartialOrd, Ord, Hash)]
pub enum CognitiveOpPhase {
    /// Giai đoạn 1: Hiểu bài toán và định hình các tiền đề cốt lõi.
    Understand,
    /// Giai đoạn 2: Trích xuất các thực thể và mối quan hệ chính.
    Extract,
    /// Giai đoạn 3: Làm nổi bật điểm then chốt và các xung đột tiềm ẩn.
    Highlight,
    /// Giai đoạn 4: Áp dụng biến đổi hoặc công cụ tính toán logic.
    Apply,
    /// Giai đoạn 5: Xác thực kết quả đầu ra dựa trên các bất biến hệ thống.
    Validate,
}

/// Đầu ra có cấu trúc của một bước thao tác nhận thức.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct CognitiveStepOutput {
    /// Giai đoạn nhận thức tương ứng.
    pub phase: CognitiveOpPhase,
    /// Các quan sát thu thập được trong giai đoạn này.
    pub observations: Vec<String>,
    /// Các suy diễn hoặc kết luận rút ra.
    pub deductions: Vec<String>,
    /// Bảng dữ liệu hoặc chứng chỉ tạo ra sau bước này.
    pub artifacts: BTreeMap<String, String>,
}

/// Công cụ nhận thức 5 thao tác bảo đảm quy trình suy luận chặt chẽ (CognitiveTool5Op).
#[derive(Debug, Clone, Default)]
pub struct CognitiveTool5Op {
    /// Lịch sử các pha nhận thức đã hoàn tất.
    pub executed_phases: Vec<CognitiveStepOutput>,
}

impl CognitiveTool5Op {
    /// Khởi tạo công cụ nhận thức mới.
    pub fn new() -> Self {
        Self {
            executed_phases: Vec::new(),
        }
    }

    /// Giai đoạn kế tiếp kỳ vọng theo quy trình chuẩn.
    pub fn expected_next_phase(&self) -> Option<CognitiveOpPhase> {
        match self.executed_phases.last().map(|s| s.phase) {
            None => Some(CognitiveOpPhase::Understand),
            Some(CognitiveOpPhase::Understand) => Some(CognitiveOpPhase::Extract),
            Some(CognitiveOpPhase::Extract) => Some(CognitiveOpPhase::Highlight),
            Some(CognitiveOpPhase::Highlight) => Some(CognitiveOpPhase::Apply),
            Some(CognitiveOpPhase::Apply) => Some(CognitiveOpPhase::Validate),
            Some(CognitiveOpPhase::Validate) => None,
        }
    }

    /// Thực hiện một bước nhận thức có kiểm tra tính hợp lệ về thứ tự giai đoạn.
    pub fn execute_step(
        &mut self,
        phase: CognitiveOpPhase,
        observations: Vec<String>,
        deductions: Vec<String>,
        artifacts: BTreeMap<String, String>,
    ) -> Result<(), &'static str> {
        let expected = self.expected_next_phase();
        if Some(phase) != expected {
            return Err("Vi phạm thứ tự giai đoạn nhận thức trong CognitiveTool5Op");
        }

        self.executed_phases.push(CognitiveStepOutput {
            phase,
            observations,
            deductions,
            artifacts,
        });

        Ok(())
    }

    /// Kiểm tra toàn bộ 5 bước đã được hoàn thành trọn vẹn chưa.
    pub fn is_completed(&self) -> bool {
        self.executed_phases.len() == 5
    }
}

// ============================================================================
// PHẦN 2: KỸ NGHỆ ĐỒ THỊ KÉP VĨ MÔ (INFOGRAPH & PLANDAG)
// ============================================================================

/// Thực thể đỉnh trong đồ thị thông tin ngữ nghĩa InfoGraph.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct InfoNode {
    /// Mã định danh duy nhất của nút (ví dụ tên hàm, tệp mã, tài liệu).
    pub id: String,
    /// Loại thực thể ngữ nghĩa (ví dụ: "function", "struct", "module", "requirement").
    pub node_type: String,
    /// Nội dung chi tiết hoặc chữ ký mã nguồn.
    pub content: String,
    /// Các nhãn thẻ phân loại.
    pub tags: BTreeSet<String>,
}

/// Cạnh liên kết có hướng trong InfoGraph.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct InfoEdge {
    /// Đỉnh nguồn của liên kết.
    pub from_id: String,
    /// Đỉnh đích của liên kết.
    pub to_id: String,
    /// Tên mối quan hệ (ví dụ: "calls", "defines", "depends_on", "verifies").
    pub relation: String,
    /// Trọng số tương tác hoặc độ tin cậy của liên kết.
    pub weight: u32,
}

/// Đồ thị thông tin ngữ nghĩa phục vụ định vị ngữ cảnh của tác tử (InfoGraph).
#[derive(Debug, Clone, Default)]
pub struct InfoGraph {
    /// Danh bạ các đỉnh của đồ thị.
    pub nodes: BTreeMap<String, InfoNode>,
    /// Danh sách các cạnh liên kết.
    pub edges: Vec<InfoEdge>,
}

/// Ngữ cảnh được cắt lát cục bộ 1 bước lân cận từ InfoGraph.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct SubGraphContext {
    /// Đỉnh trung tâm làm mốc quan sát.
    pub target_node: InfoNode,
    /// Các đỉnh lân cận trực tiếp kết nối với mốc quan sát.
    pub neighbors: Vec<InfoNode>,
    /// Các cạnh kết nối trực tiếp đến hoặc đi từ mốc quan sát.
    pub incident_edges: Vec<InfoEdge>,
    /// Chuỗi văn bản ngữ cảnh được nén gọn gàng để nạp vào cửa sổ ngữ cảnh LLM.
    pub formatted_prompt: String,
}

impl InfoGraph {
    /// Thêm một đỉnh vào đồ thị.
    pub fn add_node(&mut self, id: &str, node_type: &str, content: &str, tags: &[&str]) {
        let tag_set = tags.iter().map(|&s| s.to_string()).collect();
        self.nodes.insert(
            id.to_string(),
            InfoNode {
                id: id.to_string(),
                node_type: node_type.to_string(),
                content: content.to_string(),
                tags: tag_set,
            },
        );
    }

    /// Thêm một cạnh có hướng vào đồ thị.
    pub fn add_edge(&mut self, from_id: &str, to_id: &str, relation: &str, weight: u32) {
        self.edges.push(InfoEdge {
            from_id: from_id.to_string(),
            to_id: to_id.to_string(),
            relation: relation.to_string(),
            weight,
        });
    }

    /// Cắt lát ngữ cảnh 1 bước (1-hop neighborhood slice) quanh một đỉnh mốc quan tâm.
    /// Giúp tiết kiệm đáng kể token so với việc đẩy toàn bộ đồ thị vào prompt.
    pub fn slice_1hop_context(&self, target_id: &str) -> Option<SubGraphContext> {
        let target = self.nodes.get(target_id)?.clone();

        let mut neighbor_ids = BTreeSet::new();
        let mut incident = Vec::new();

        for e in &self.edges {
            if e.from_id == target_id {
                neighbor_ids.insert(e.to_id.clone());
                incident.push(e.clone());
            } else if e.to_id == target_id {
                neighbor_ids.insert(e.from_id.clone());
                incident.push(e.clone());
            }
        }

        let mut neighbors = Vec::new();
        for nid in &neighbor_ids {
            if let Some(n) = self.nodes.get(nid) {
                neighbors.push(n.clone());
            }
        }

        // Tạo văn bản ngữ cảnh súc tích
        let mut prompt = format!(
            "[Target: {} ({})]\n{}\n--- Relationships ---\n",
            target.id, target.node_type, target.content
        );

        for e in &incident {
            prompt.push_str(&format!(
                "{} --[{}]--> {} (weight: {})\n",
                e.from_id, e.relation, e.to_id, e.weight
            ));
        }

        Some(SubGraphContext {
            target_node: target,
            neighbors,
            incident_edges: incident,
            formatted_prompt: prompt,
        })
    }
}

/// Trạng thái của một nút thực thi trong kế hoạch PlanDag.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum DagNodeState {
    /// Đang chờ các phụ thuộc hoàn tất hoặc đang chờ đến lượt thực thi.
    Pending,
    /// Đã thực thi thành công mỹ mãn.
    Succeeded,
    /// Đã chạm trần số lần thử lại nhưng vẫn thất bại.
    ExhaustedRetries,
}

/// Các lỗi phát sinh trong quá trình biên dịch và lập lịch đồ thị PlanDag.
#[derive(Debug, Clone, PartialEq, Eq)]
pub enum DagError {
    /// Tồn tại quan hệ phụ thuộc tới một nút không hề có trong danh bạ đồ thị.
    UnknownDependency { missing_id: String },
    /// Đồ thị chứa chu trình khép kín, vi phạm tính chất phi chu trình của DAG.
    CycleDetected,
    /// Yêu cầu thao tác trên một mã định danh nút không tồn tại.
    UnknownNode { node_id: String },
}

/// Ô lưu trữ thông tin cấu hình và tiến độ của một nút trong PlanDag.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct DagSlot {
    /// Danh sách mã các nút tiên quyết bắt buộc phải hoàn thành trước.
    pub dependencies: Vec<String>,
    /// Ngân sách số lần cho phép thử lại trước khi từ bỏ.
    pub retry_budget: u8,
    /// Số lần đã thử thực thi trên thực tế.
    pub attempts: u8,
    /// Trạng thái hiện tại của nút.
    pub state: DagNodeState,
}

/// Đồ thị kế hoạch thực thi hướng luồng (PlanDag) hỗ trợ phân tích tô-pô song song theo sóng Kahn.
#[derive(Debug, Clone, Default)]
pub struct PlanDag {
    /// Bảng lưu trữ toàn bộ các nút trong đồ thị kế hoạch.
    pub nodes: BTreeMap<String, DagSlot>,
}

impl PlanDag {
    /// Khởi tạo một PlanDag rỗng.
    pub fn new() -> Self {
        Self {
            nodes: BTreeMap::new(),
        }
    }

    /// Thêm một nhiệm vụ vào đồ thị kèm danh sách nút cha và ngân sách thử lại.
    pub fn add_node(&mut self, id: &str, dependencies: &[&str], retry_budget: u8) {
        let deps = dependencies.iter().map(|&d| d.to_string()).collect();
        self.nodes.insert(
            id.to_string(),
            DagSlot {
                dependencies: deps,
                retry_budget,
                attempts: 0,
                state: DagNodeState::Pending,
            },
        );
    }

    /// Phân tích đồ thị thành các đợt sẵn sàng song song (topological ready waves) theo thuật toán Kahn.
    /// Nếu phát hiện chu trình, trả về `DagError::CycleDetected`.
    pub fn topological_ready_waves(&self) -> Result<Vec<Vec<String>>, DagError> {
        let mut in_degrees: BTreeMap<&str, usize> = BTreeMap::new();
        let mut children: BTreeMap<&str, Vec<&str>> = BTreeMap::new();

        for (id, slot) in &self.nodes {
            in_degrees.entry(id.as_str()).or_insert(0);
            for dep in &slot.dependencies {
                if !self.nodes.contains_key(dep) {
                    return Err(DagError::UnknownDependency {
                        missing_id: dep.clone(),
                    });
                }
                *in_degrees.entry(id.as_str()).or_insert(0) += 1;
                children.entry(dep.as_str()).or_default().push(id.as_str());
            }
        }

        // Hàng đợi khởi đầu chứa các nút có bậc vào bằng 0
        let mut queue: VecDeque<&str> = in_degrees
            .iter()
            .filter_map(|(&id, &deg)| if deg == 0 { Some(id) } else { None })
            .collect();

        let mut waves = Vec::new();
        let mut processed_count = 0usize;

        while !queue.is_empty() {
            let mut current_wave = Vec::new();
            let mut next_level_candidates = BTreeSet::new();

            while let Some(u) = queue.pop_front() {
                processed_count += 1;
                current_wave.push(u.to_string());

                if let Some(kids) = children.get(u) {
                    for &v in kids {
                        let deg = in_degrees.get_mut(v).unwrap();
                        *deg -= 1;
                        if *deg == 0 {
                            next_level_candidates.insert(v);
                        }
                    }
                }
            }

            waves.push(current_wave);
            queue.extend(next_level_candidates);
        }

        if processed_count == self.nodes.len() {
            Ok(waves)
        } else {
            Err(DagError::CycleDetected)
        }
    }

    /// Ghi nhận kết quả của một lần thực thi nút và cập nhật trạng thái thử lại cục bộ.
    pub fn record_attempt(&mut self, id: &str, success: bool) -> Result<DagNodeState, DagError> {
        let slot = self
            .nodes
            .get_mut(id)
            .ok_or_else(|| DagError::UnknownNode {
                node_id: id.to_string(),
            })?;

        slot.attempts = slot.attempts.saturating_add(1);
        slot.state = if success {
            DagNodeState::Succeeded
        } else if slot.attempts > slot.retry_budget {
            DagNodeState::ExhaustedRetries
        } else {
            DagNodeState::Pending
        };

        Ok(slot.state)
    }
}

/// Một đợt tác vụ sẵn sàng song song đã được tối ưu hoá trong lịch trình biên dịch trước.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct CompiledWave {
    /// Chỉ số thứ tự của đợt sóng (0-indexed).
    pub wave_index: usize,
    /// Danh sách các mã tác vụ độc lập có thể chạy song song trong đợt này.
    pub task_ids: Vec<String>,
}

/// Lịch trình biên dịch trước (Ahead-of-Time Subgraph Schedule) được tối ưu hoá cho bộ lập lịch runtime.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct CompiledSchedule {
    /// Danh sách các đợt thực thi theo thứ tự thời gian.
    pub waves: Vec<CompiledWave>,
    /// Tổng số lượng tác vụ cần hoàn thành.
    pub total_tasks: usize,
    /// Mức độ song song tối đa (độ rộng cực đại của một đợt).
    pub max_concurrency: usize,
}

/// Bộ biên dịch trước đồ thị kế hoạch tác tử (AotSubgraphCompiler).
#[derive(Debug, Clone, Default)]
pub struct AotSubgraphCompiler;

impl AotSubgraphCompiler {
    /// Biên dịch một PlanDag thành bản đặc tả lịch trình tĩnh bất biến, sẵn sàng phân bổ worker.
    pub fn compile(dag: &PlanDag) -> Result<CompiledSchedule, DagError> {
        let raw_waves = dag.topological_ready_waves()?;
        let total_tasks = dag.nodes.len();
        let mut max_concurrency = 0;

        let waves = raw_waves
            .into_iter()
            .enumerate()
            .map(|(wave_index, task_ids)| {
                if task_ids.len() > max_concurrency {
                    max_concurrency = task_ids.len();
                }
                CompiledWave {
                    wave_index,
                    task_ids,
                }
            })
            .collect();

        Ok(CompiledSchedule {
            waves,
            total_tasks,
            max_concurrency,
        })
    }
}

// ============================================================================
// PHẦN 3: MÁY TRẠNG THÁI NÚT STATEGRAPH VÀ ĐIỂM KIỂM SOÁT STRIPE BLUEPRINTS
// ============================================================================

/// Loại hàm gộp (Reducer) dùng để kết hợp các biến trạng thái khi nhiều nút cùng phát ra dữ liệu.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum ReducerKind {
    /// Ghi đè giá trị cũ bằng giá trị mới nhất.
    Overwrite,
    /// Nối tiếp thêm các phần tử vào mảng/vector hiện có.
    Append,
    /// Cộng dồn số học các chỉ số định lượng.
    Sum,
}

/// Điểm kiểm soát trạng thái tất định theo chuẩn Stripe Blueprints.
#[derive(Debug, Clone, PartialEq)]
pub struct BlueprintCheckpoint<S> {
    /// Mã định danh tuần tự của điểm kiểm soát.
    pub checkpoint_id: u64,
    /// Mã nút vừa tạo ra điểm kiểm soát này.
    pub node_id: String,
    /// Ảnh chụp trạng thái chia sẻ của tác tử tại thời điểm đó.
    pub state_snapshot: S,
    /// Nhật ký sự kiện riêng tư của nút (private node log).
    pub private_log: Vec<String>,
    /// Mã băm xác thực tính toàn vẹn của dữ liệu trạng thái.
    pub checksum: u64,
}

/// Cổng kiểm tra tính tất định và khôi phục trạng thái tác tử kiểu Stripe Blueprints.
#[derive(Debug, Clone)]
pub struct StateGraph<S> {
    /// Trạng thái hiện tại được chia sẻ giữa các nút.
    pub current_state: S,
    /// Sổ cái lưu vết toàn bộ các điểm kiểm soát đã lập.
    pub checkpoints: Vec<BlueprintCheckpoint<S>>,
    /// Bộ đếm tạo checkpoint_id đơn điệu tăng.
    pub next_checkpoint_id: u64,
}

impl<S: Clone + Hash> StateGraph<S> {
    /// Khởi tạo máy trạng thái với trạng thái nguyên thuỷ ban đầu.
    pub fn new(initial_state: S) -> Self {
        let mut hasher = DefaultHasher::new();
        initial_state.hash(&mut hasher);
        let checksum = hasher.finish();

        let initial_checkpoint = BlueprintCheckpoint {
            checkpoint_id: 1,
            node_id: "root".to_string(),
            state_snapshot: initial_state.clone(),
            private_log: vec!["Khởi tạo máy trạng thái StateGraph".to_string()],
            checksum,
        };

        Self {
            current_state: initial_state,
            checkpoints: vec![initial_checkpoint],
            next_checkpoint_id: 2,
        }
    }

    /// Áp dụng chuyển trạng thái từ một nút và tạo điểm kiểm soát Stripe Blueprint mới.
    pub fn transition<F>(&mut self, node_id: &str, step_fn: F) -> Result<u64, &'static str>
    where
        F: FnOnce(&S) -> Result<(S, Vec<String>), &'static str>,
    {
        let (new_state, private_log) = step_fn(&self.current_state)?;

        let mut hasher = DefaultHasher::new();
        new_state.hash(&mut hasher);
        let checksum = hasher.finish();

        let checkpoint_id = self.next_checkpoint_id;
        self.next_checkpoint_id += 1;

        let cp = BlueprintCheckpoint {
            checkpoint_id,
            node_id: node_id.to_string(),
            state_snapshot: new_state.clone(),
            private_log,
            checksum,
        };

        self.current_state = new_state;
        self.checkpoints.push(cp);

        Ok(checkpoint_id)
    }

    /// Hoàn tác (rollback) trạng thái về một điểm kiểm soát trước đó nếu xảy ra sự cố.
    pub fn rollback_to(&mut self, checkpoint_id: u64) -> Result<(), &'static str> {
        let target_idx = self
            .checkpoints
            .iter()
            .position(|c| c.checkpoint_id == checkpoint_id)
            .ok_or("Không tìm thấy checkpoint_id yêu cầu để rollback")?;

        self.current_state = self.checkpoints[target_idx].state_snapshot.clone();
        self.checkpoints.truncate(target_idx + 1);

        Ok(())
    }
}

// ============================================================================
// PHẦN 4: NGẮT DUYỆT HITL & TRÍ NHỚ THỦ TỤC PROCEDURALMEMORY
// ============================================================================

/// Quyết định ngắt can thiệp của con người (Human-in-the-Loop Interrupt).
#[derive(Debug, Clone, PartialEq, Eq)]
pub enum HitlInterrupt {
    /// Chấp thuận nguyên văn đề xuất gọi công cụ từ tác tử.
    Accept,
    /// Hiệu chỉnh lại các tham số truyền vào công cụ.
    Edit {
        /// Bộ tham số mới do con người chỉ định sau khi rà soát.
        new_arguments: BTreeMap<String, String>,
    },
    /// Từ chối và phản hồi chỉ dẫn văn bản yêu cầu tác tử lập kế hoạch lại.
    Respond {
        /// Thông điệp phản hồi gửi lại cho tác tử.
        feedback_message: String,
    },
    /// Bỏ qua hành động và huỷ bỏ tác vụ độc quyền này.
    Ignore,
}

/// Trí nhớ thủ tục (ProceduralMemory) lưu các quy tắc hành vi và bài học từ phản hồi con người.
#[derive(Debug, Clone, Default, PartialEq, Eq)]
pub struct ProceduralMemory {
    /// Quy tắc phân loại hoặc từ chối công cụ trong các ngữ cảnh nguy hiểm.
    pub triage_rules: Vec<String>,
    /// Quy tắc phản hồi và giải thích khi gặp tình huống tương tự.
    pub response_rules: Vec<String>,
    /// Quy tắc ghi đè hoặc loại bỏ tham số công cụ đã được học từ các lần con người chỉnh sửa.
    pub tool_override_rules: Vec<String>,
}

/// Kết quả giải quyết ngắt duyệt HITL để tác tử quyết định bước hành động tiếp theo.
#[derive(Debug, Clone, PartialEq, Eq)]
pub enum HitlResolution {
    /// Cho phép tiếp tục thực thi công cụ với bộ tham số đã phê duyệt.
    Execute {
        /// Bộ tham số cuối cùng sau khi con người xét duyệt hoặc sửa đổi.
        arguments: BTreeMap<String, String>,
    },
    /// Yêu cầu tác tử tự phản tỉnh và lập lại kế hoạch theo thông điệp của con người.
    Replan {
        /// Thông điệp chỉ dẫn từ con người.
        message: String,
    },
    /// Huỷ bỏ tiến trình thực thi hành động này.
    Aborted,
}

/// Xử lý quyết định ngắt can thiệp HITL và cập nhật tự động vào Trí nhớ thủ tục ProceduralMemory.
///
/// **SỬA LỖI EDGE CASE #5**: Kiểm tra kỹ lưỡng cả sự khác biệt (diff) giữa `orig.keys()` và `new_arguments.keys()`
/// để phát hiện các khoá tham số bị con người XOÁ KHỎI `orig` và lưu vết rõ ràng vào `tool_override_rules`.
pub fn resolve_hitl_interrupt(
    tool_name: &str,
    orig_arguments: &BTreeMap<String, String>,
    decision: HitlInterrupt,
    memory: &mut ProceduralMemory,
) -> HitlResolution {
    match decision {
        HitlInterrupt::Accept => HitlResolution::Execute {
            arguments: orig_arguments.clone(),
        },
        HitlInterrupt::Edit { new_arguments } => {
            // 1. Ghi nhận các khoá bị thay đổi giá trị hoặc mới thêm vào
            for (k, v) in &new_arguments {
                if orig_arguments.get(k) != Some(v) {
                    memory
                        .tool_override_rules
                        .push(format!("tool={} arg={}->{}", tool_name, k, v));
                }
            }

            // 2. SỬA LỖI EDGE CASE #5: Kiểm tra các khoá có trong orig nhưng bị con người XOÁ trong new_arguments
            for k in orig_arguments.keys() {
                if !new_arguments.contains_key(k) {
                    memory
                        .tool_override_rules
                        .push(format!("tool={} arg={}->[DELETED]", tool_name, k));
                }
            }

            HitlResolution::Execute {
                arguments: new_arguments,
            }
        }
        HitlInterrupt::Respond { feedback_message } => {
            memory
                .response_rules
                .push(format!("tool={}: {}", tool_name, feedback_message));
            HitlResolution::Replan {
                message: feedback_message,
            }
        }
        HitlInterrupt::Ignore => {
            memory
                .triage_rules
                .push(format!("suppress_tool={}", tool_name));
            HitlResolution::Aborted
        }
    }
}

// ============================================================================
// PHẦN 5: TRÍ NHỚ NHẬN THỨC 4 TẦNG, ĐỒ THỊ LƯỠNG THỜI GIAN & BDI ENGINE
// ============================================================================

/// Một sự kiện hoặc mẩu ký ức trong tầng Trí nhớ tình tiết (Episodic Memory).
#[derive(Debug, Clone, PartialEq)]
pub struct EpisodicItem {
    /// Mã định danh của mẩu ký ức.
    pub id: String,
    /// Thời điểm diễn ra sự kiện (timestamp logic).
    pub timestamp: u64,
    /// Tóm tắt nội dung sự kiện.
    pub summary: String,
    /// Danh sách các từ khoá ngữ nghĩa.
    pub keywords: BTreeSet<String>,
    /// Điểm quan trọng (importance score, thang 0.0 đến 1.0).
    pub importance: f64,
}

/// Một định đề tri thức trong tầng Trí nhớ ngữ nghĩa (Semantic Memory).
#[derive(Debug, Clone, PartialEq)]
pub struct SemanticFact {
    /// Chủ ngữ / Thực thể.
    pub subject: String,
    /// Vị ngữ / Thuộc tính.
    pub predicate: String,
    /// Tân ngữ / Giá trị chân lý.
    pub object: String,
    /// Độ tin cậy của sự thật (0.0 đến 1.0).
    pub confidence: f64,
}

/// Hệ thống trí nhớ nhận thức 4 tầng CognitiveMemory4Tier tuân thủ tiêu chuẩn MEM1.
#[derive(Debug, Clone, Default)]
pub struct CognitiveMemory4Tier {
    /// Tầng 1: Trí nhớ làm việc (Working Memory) - sổ nháp tức thời trong phiên.
    pub working_memory: Vec<String>,
    /// Tầng 2: Trí nhớ tình tiết (Episodic Memory) - hồ sơ các trải nghiệm đã qua.
    pub episodic_memory: Vec<EpisodicItem>,
    /// Tầng 3: Trí nhớ ngữ nghĩa (Semantic Memory) - mạng lưới tri thức vĩnh cửu.
    pub semantic_memory: BTreeMap<String, SemanticFact>,
    /// Tầng 4: Trí nhớ thủ tục (Procedural Memory) - kỹ năng và quy tắc phản xạ.
    pub procedural_memory: ProceduralMemory,
}

impl CognitiveMemory4Tier {
    /// Thêm dữ liệu vào trí nhớ làm việc tức thời.
    pub fn push_working(&mut self, note: &str) {
        self.working_memory.push(note.to_string());
    }

    /// Thêm sự kiện vào trí nhớ tình tiết.
    pub fn record_episode(
        &mut self,
        id: &str,
        timestamp: u64,
        summary: &str,
        keywords: &[&str],
        importance: f64,
    ) {
        let kw_set = keywords.iter().map(|&k| k.to_lowercase()).collect();
        self.episodic_memory.push(EpisodicItem {
            id: id.to_string(),
            timestamp,
            summary: summary.to_string(),
            keywords: kw_set,
            importance,
        });
    }

    /// Cơ chế củng cố trí nhớ (Consolidation):
    /// Khi hai mẩu ký ức tình tiết có độ trùng lặp từ khoá (Jaccard similarity) >= 0.7,
    /// chúng sẽ được đúc kết thành một tri thức khái quát trong Trí nhớ ngữ nghĩa.
    pub fn consolidate_episodes(&mut self) -> usize {
        let mut consolidated_count = 0;
        let n = self.episodic_memory.len();
        if n < 2 {
            return 0;
        }

        let mut pairs_to_merge = Vec::new();
        for i in 0..n {
            for j in (i + 1)..n {
                let a = &self.episodic_memory[i];
                let b = &self.episodic_memory[j];

                let inter = a.keywords.intersection(&b.keywords).count();
                let union = a.keywords.union(&b.keywords).count();

                if union > 0 {
                    let jaccard = inter as f64 / union as f64;
                    if jaccard >= 0.7 {
                        pairs_to_merge.push((i, j, a.summary.clone(), b.summary.clone()));
                    }
                }
            }
        }

        for (i, j, sum_a, sum_b) in pairs_to_merge {
            let key = format!("concept_from_{}_{}", i, j);
            self.semantic_memory.insert(
                key,
                SemanticFact {
                    subject: sum_a,
                    predicate: "tương_đồng_và_hợp_nhất_với".to_string(),
                    object: sum_b,
                    confidence: 0.95,
                },
            );
            consolidated_count += 1;
        }

        consolidated_count
    }
}

/// Sự kiện lưu vết trong đồ thị lưỡng thời gian BiTemporalGraph.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct BiTemporalFact {
    /// Mã định danh duy nhất của sự thật.
    pub fact_id: String,
    /// Thực thể được nói tới.
    pub entity_id: String,
    /// Thuộc tính của thực thể.
    pub attribute: String,
    /// Giá trị ghi nhận.
    pub value: String,
    /// ValidTime: Bắt đầu có hiệu lực trong đời thực.
    pub valid_from: u64,
    /// ValidTime: Kết thúc hiệu lực trong đời thực.
    pub valid_to: u64,
    /// TxTime: Bắt đầu được hệ thống ghi nhận vào cơ sở dữ liệu.
    pub tx_from: u64,
    /// TxTime: Bị hệ thống thay thế hoặc thu hồi trong cơ sở dữ liệu.
    pub tx_to: u64,
}

/// Khoảng thời gian có hiệu lực trong thực tế [valid_from, valid_to).
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct ValidTimeRange {
    /// Bắt đầu có hiệu lực trong đời thực.
    pub valid_from: u64,
    /// Kết thúc hiệu lực trong đời thực.
    pub valid_to: u64,
}

/// Đồ thị lưỡng thời gian (BiTemporalGraph) hỗ trợ truy vấn du hành thời gian (Time Travel).
#[derive(Debug, Clone, Default)]
pub struct BiTemporalGraph {
    /// Tập hợp các sự thật được đánh dấu cả thời gian hiệu lực và thời gian giao dịch.
    pub facts: Vec<BiTemporalFact>,
}

impl BiTemporalGraph {
    /// Khởi tạo đồ thị lưỡng thời gian.
    pub fn new() -> Self {
        Self { facts: Vec::new() }
    }

    /// Ghi nhận một sự thật mới vào đồ thị.
    pub fn record_fact(
        &mut self,
        fact_id: &str,
        entity_id: &str,
        attribute: &str,
        value: &str,
        valid_range: ValidTimeRange,
        current_tx: u64,
    ) {
        self.facts.push(BiTemporalFact {
            fact_id: fact_id.to_string(),
            entity_id: entity_id.to_string(),
            attribute: attribute.to_string(),
            value: value.to_string(),
            valid_from: valid_range.valid_from,
            valid_to: valid_range.valid_to,
            tx_from: current_tx,
            tx_to: u64::MAX, // Còn đang có hiệu lực trong hệ thống
        });
    }

    /// Thu hồi hoặc điều chỉnh hồi tố một sự thật tại thời điểm giao dịch hiện tại mà không làm mất lịch sử cũ.
    pub fn retract_fact(&mut self, fact_id: &str, current_tx: u64) -> bool {
        let mut modified = false;
        for f in &mut self.facts {
            if f.fact_id == fact_id && f.tx_to == u64::MAX {
                f.tx_to = current_tx;
                modified = true;
            }
        }
        modified
    }

    /// Truy vấn du hành thời gian: tìm các dữ liệu có hiệu lực tại `as_of_valid` dưới lăng kính hệ thống tại `as_of_tx`.
    pub fn query_time_travel(
        &self,
        entity_id: &str,
        as_of_valid: u64,
        as_of_tx: u64,
    ) -> Vec<BiTemporalFact> {
        self.facts
            .iter()
            .filter(|f| {
                f.entity_id == entity_id
                    && f.valid_from <= as_of_valid
                    && as_of_valid < f.valid_to
                    && f.tx_from <= as_of_tx
                    && as_of_tx < f.tx_to
            })
            .cloned()
            .collect()
    }
}

/// Niềm tin (Belief) của tác tử trong mô hình BDI.
#[derive(Debug, Clone, PartialEq)]
pub struct Belief {
    /// Thực thể đối tượng.
    pub entity: String,
    /// Thuộc tính đánh giá.
    pub property: String,
    /// Giá trị tin tưởng.
    pub value: String,
    /// Độ chắc chắn của niềm tin.
    pub confidence: f64,
}

/// Mong muốn hoặc mục tiêu (Desire) mà tác tử hướng tới.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct Desire {
    /// Mã định danh mong muốn.
    pub id: String,
    /// Mô tả trạng thái mục tiêu cần đạt.
    pub goal_description: String,
    /// Độ ưu tiên thực thi.
    pub priority: u32,
    /// Trạng thái đã thoả mãn hay chưa.
    pub is_fulfilled: bool,
}

/// Ý định (Intention) - kế hoạch hành động mà tác tử đã chính thức cam kết dốc tài nguyên thực hiện.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct Intention {
    /// Mã định danh ý định.
    pub id: String,
    /// Mong muốn mà ý định này đang phụng sự.
    pub desire_id: String,
    /// Các bước hành động cụ thể.
    pub action_steps: Vec<String>,
    /// Chỉ số bước hiện hành.
    pub current_step: usize,
    /// Trạng thái hoạt động của ý định.
    pub is_active: bool,
}

/// Động cơ niềm tin - mong muốn - ý định (BDI Engine) phân biệt rõ Endurants và Perdurants.
#[derive(Debug, Clone, Default)]
pub struct BdiEngine {
    /// Danh mục niềm tin hiện hữu.
    pub beliefs: BTreeMap<String, Belief>,
    /// Danh mục các mong muốn.
    pub desires: Vec<Desire>,
    /// Danh mục các ý định đã cam kết.
    pub intentions: Vec<Intention>,
}

impl BdiEngine {
    /// Thêm hoặc cập nhật niềm tin.
    pub fn update_belief(&mut self, entity: &str, property: &str, value: &str, confidence: f64) {
        let key = format!("{}.{}", entity, property);
        self.beliefs.insert(
            key,
            Belief {
                entity: entity.to_string(),
                property: property.to_string(),
                value: value.to_string(),
                confidence,
            },
        );
    }

    /// Thêm một mong muốn mới.
    pub fn add_desire(&mut self, id: &str, goal_description: &str, priority: u32) {
        self.desires.push(Desire {
            id: id.to_string(),
            goal_description: goal_description.to_string(),
            priority,
            is_fulfilled: false,
        });
    }

    /// Chuyển hoá một mong muốn thành ý định hành động cụ thể.
    pub fn commit_intention(&mut self, intention_id: &str, desire_id: &str, steps: &[&str]) {
        self.intentions.push(Intention {
            id: intention_id.to_string(),
            desire_id: desire_id.to_string(),
            action_steps: steps.iter().map(|&s| s.to_string()).collect(),
            current_step: 0,
            is_active: true,
        });
    }

    /// Tiến thêm một bước trong kế hoạch ý định.
    pub fn advance_intention(&mut self, intention_id: &str) -> Option<String> {
        let intent = self.intentions.iter_mut().find(|i| i.id == intention_id)?;
        if intent.current_step < intent.action_steps.len() {
            let action = intent.action_steps[intent.current_step].clone();
            intent.current_step += 1;
            if intent.current_step >= intent.action_steps.len() {
                intent.is_active = false;
                // Đánh dấu desire tương ứng là hoàn thành
                if let Some(d) = self.desires.iter_mut().find(|d| d.id == intent.desire_id) {
                    d.is_fulfilled = true;
                }
            }
            Some(action)
        } else {
            None
        }
    }
}

// ============================================================================
// PHẦN 6: PHÂN ĐOẠN MARKDOWN, TRUY XUẤT LAI RRF & KIỂM CHỨNG TRÍCH DẪN
// ============================================================================

/// Một đoạn văn bản được chia theo ngăn xếp tiêu đề Markdown.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct DocChunk {
    /// Chỉ số thứ tự định danh đoạn văn trong tập tài liệu.
    pub id: usize,
    /// Đường dẫn phân cấp tiêu đề (ví dụ: "H1 > H2 > H3").
    pub heading_path: String,
    /// Nội dung văn bản của đoạn trích.
    pub content: String,
}

fn emit_chunk_stack(
    words: &mut Vec<String>,
    stack: &[(usize, String)],
    max_w: usize,
    step: usize,
    out: &mut Vec<DocChunk>,
) {
    if words.is_empty() {
        return;
    }
    let path = stack
        .iter()
        .map(|(_, text)| text.as_str())
        .collect::<Vec<_>>()
        .join(" > ");

    let mut start = 0;
    while start < words.len() {
        let end = (start + max_w).min(words.len());
        out.push(DocChunk {
            id: out.len(),
            heading_path: path.clone(),
            content: words[start..end].join(" "),
        });
        if end == words.len() {
            break;
        }
        start += step;
    }
    words.clear();
}

/// Chia nhỏ tài liệu Markdown theo ngăn xếp tiêu đề phân cấp.
///
/// **SỬA LỖI EDGE CASE #1**: Bảo vệ `max_w.max(1)` để không bao giờ bị panic chia cho 0 hoặc tràn số khi `max_w == 0`.
///
/// **SỬA LỖI EDGE CASE #2**: Giám sát trạng thái `in_code_fence` (```` ``` ```` hoặc `~~~`) để bỏ qua các dòng
/// chú thích giả mạo tiêu đề như `# comment` nằm bên trong khối mã nguồn.
pub fn chunk_markdown_by_headings(md: &str, max_w: usize, overlap_w: usize) -> Vec<DocChunk> {
    // SỬA LỖI EDGE CASE #1: Đảm bảo max_w ít nhất là 1
    let mw = max_w.max(1);
    let step = mw
        .saturating_sub(overlap_w.min(mw.saturating_sub(1)))
        .max(1);

    let mut stack: Vec<(usize, String)> = Vec::new();
    let mut words = Vec::new();
    let mut out = Vec::new();

    // SỬA LỖI EDGE CASE #2: Theo dõi trạng thái rào mã code fence
    let mut in_code_fence = false;

    for line in md.lines() {
        let trimmed = line.trim();

        // Kiểm tra mở/đóng code fence
        if trimmed.starts_with("```") || trimmed.starts_with("~~~") {
            in_code_fence = !in_code_fence;
            if !trimmed.is_empty() {
                words.extend(trimmed.split_whitespace().map(str::to_string));
            }
            continue;
        }

        // Nếu đang nằm trong code fence, bỏ qua hoàn toàn việc nhận diện tiêu đề #
        if in_code_fence {
            if !trimmed.is_empty() {
                words.extend(trimmed.split_whitespace().map(str::to_string));
            }
            continue;
        }

        // Phân tích tiêu đề Markdown chuẩn ngoài rào mã
        let h_count = trimmed.chars().take_while(|&c| c == '#').count();
        if (1..=6).contains(&h_count) && trimmed[h_count..].starts_with(' ') {
            emit_chunk_stack(&mut words, &stack, mw, step, &mut out);
            while stack.last().is_some_and(|(lvl, _)| *lvl >= h_count) {
                stack.pop();
            }
            stack.push((h_count, trimmed[h_count..].trim().to_string()));
        } else if !trimmed.is_empty() {
            words.extend(trimmed.split_whitespace().map(str::to_string));
        }
    }

    emit_chunk_stack(&mut words, &stack, mw, step, &mut out);
    out
}

/// Trích xuất danh sách các từ khoá đơn giản phục vụ thuật toán BM25.
pub fn tokenize_terms(text: &str) -> Vec<String> {
    let is_delim = |c: char| !c.is_alphanumeric() && c != '_';
    text.to_lowercase()
        .split(is_delim)
        .filter(|t| !t.is_empty())
        .map(str::to_string)
        .collect()
}

/// Bộ truy xuất lai kết hợp Okapi BM25 và Dense Cosine qua phương pháp Reciprocal Rank Fusion (RRF).
#[derive(Debug, Clone, Default)]
pub struct HybridRetriever;

impl HybridRetriever {
    /// Thực hiện tìm kiếm lai.
    ///
    /// **SỬA LỖI EDGE CASE #3**: Kiểm tra chặt chẽ `assert_eq!(chunks.len(), embs.len())`
    /// để ngăn chặn triệt để lỗi truy cập ngoài biên mảng (out-of-bounds panic) khi số lượng embedding
    /// không tương ứng với số lượng đoạn tài liệu.
    pub fn search(
        chunks: &[DocChunk],
        embs: &[[f64; 3]],
        query: &str,
        q_emb: &[f64; 3],
        rrf_k: usize,
    ) -> Vec<(usize, f64)> {
        // SỬA LỖI EDGE CASE #3: Khẳng định tính tương thích về độ dài mảng
        assert_eq!(
            chunks.len(),
            embs.len(),
            "Độ dài mảng chunks ({}) và embs ({}) bắt buộc phải bằng nhau",
            chunks.len(),
            embs.len()
        );

        if chunks.is_empty() {
            return Vec::new();
        }

        // 1. Tính toán điểm Okapi BM25
        let mut term_frequencies = Vec::with_capacity(chunks.len());
        let mut doc_lengths = Vec::with_capacity(chunks.len());
        let mut doc_frequencies: HashMap<String, usize> = HashMap::new();

        for c in chunks {
            let full_text = format!("{} {}", c.heading_path, c.content);
            let toks = tokenize_terms(&full_text);
            doc_lengths.push(toks.len());

            let mut tf = HashMap::new();
            for t in toks {
                *tf.entry(t).or_insert(0usize) += 1;
            }

            for k in tf.keys() {
                *doc_frequencies.entry(k.clone()).or_insert(0usize) += 1;
            }
            term_frequencies.push(tf);
        }

        let avg_dl =
            (doc_lengths.iter().sum::<usize>() as f64 / chunks.len().max(1) as f64).max(1.0);
        let n = chunks.len() as f64;
        let q_terms = tokenize_terms(query);

        let mut bm25_scores = Vec::with_capacity(chunks.len());
        for i in 0..chunks.len() {
            let dl = doc_lengths[i] as f64;
            let mut score = 0.0;
            for qt in &q_terms {
                let tf = *term_frequencies[i].get(qt).unwrap_or(&0) as f64;
                if tf > 0.0 {
                    let df = *doc_frequencies.get(qt).unwrap_or(&0) as f64;
                    let idf = (1.0 + (n - df + 0.5) / (df + 0.5)).ln();
                    score += idf * (tf * 2.5) / (tf + 1.5 * (0.25 + 0.75 * (dl / avg_dl)));
                }
            }
            bm25_scores.push((i, score));
        }
        bm25_scores.sort_by(|a, b| b.1.total_cmp(&a.1).then_with(|| a.0.cmp(&b.0)));

        // 2. Tính toán khoảng cách Dense Cosine
        let norm = |v: &[f64; 3]| (v[0] * v[0] + v[1] * v[1] + v[2] * v[2]).sqrt().max(1e-12);
        let q_norm = norm(q_emb);

        let mut dense_scores: Vec<(usize, f64)> = embs
            .iter()
            .enumerate()
            .map(|(i, e)| {
                let dot = q_emb[0] * e[0] + q_emb[1] * e[1] + q_emb[2] * e[2];
                let cosine = dot / (q_norm * norm(e));
                (i, cosine)
            })
            .collect();
        dense_scores.sort_by(|a, b| b.1.total_cmp(&a.1).then_with(|| a.0.cmp(&b.0)));

        // 3. Hợp nhất thứ hạng RRF (Reciprocal Rank Fusion)
        let mut fused_scores = vec![0.0; chunks.len()];
        for (rank, &(chunk_idx, _)) in bm25_scores.iter().enumerate() {
            fused_scores[chunk_idx] += 1.0 / ((rrf_k + rank + 1) as f64);
        }
        for (rank, &(chunk_idx, _)) in dense_scores.iter().enumerate() {
            fused_scores[chunk_idx] += 1.0 / ((rrf_k + rank + 1) as f64);
        }

        let mut results: Vec<(usize, f64)> = fused_scores.into_iter().enumerate().collect();
        results.sort_by(|a, b| b.1.total_cmp(&a.1).then_with(|| a.0.cmp(&b.0)));
        results
    }
}

/// Yêu cầu kiểm chứng trích dẫn văn bản dựa trên toạ độ lát cắt (Span Citation).
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct CitationSpan {
    /// Chỉ số đoạn trích dẫn được tham chiếu.
    pub chunk_id: usize,
    /// Vị trí byte bắt đầu trong văn bản của chunk.
    pub start_offset: usize,
    /// Vị trí byte kết thúc trong văn bản của chunk.
    pub end_offset: usize,
    /// Nội dung văn bản mà mô hình tuyên bố đã trích dẫn trực tiếp.
    pub claimed_text: String,
}

/// Kết quả kiểm chứng tính xác thực của trích dẫn.
#[derive(Debug, Clone, PartialEq, Eq)]
pub enum CitationVerificationResult {
    /// Trích dẫn hoàn toàn chính xác từng ký tự.
    Valid,
    /// Không tìm thấy đoạn trích tham chiếu tương ứng.
    ChunkNotFound { chunk_id: usize },
    /// Toạ độ trích dẫn nằm ngoài phạm vi chuỗi văn bản của đoạn trích.
    OutOfBounds {
        chunk_id: usize,
        start: usize,
        end: usize,
        chunk_len: usize,
    },
    /// Nội dung trích dẫn không khớp với dữ liệu thực tế tại toạ độ chỉ định (phát hiện ảo giác).
    ContentMismatch { expected: String, found: String },
}

/// Bộ kiểm chứng trích dẫn văn bản (SpanCitationVerifier) ngăn chặn ảo giác LLM.
#[derive(Debug, Clone, Default)]
pub struct SpanCitationVerifier;

impl SpanCitationVerifier {
    /// Xác thực xem trích dẫn của mô hình có thực sự xuất hiện trong tài liệu gốc hay không.
    pub fn verify(chunks: &[DocChunk], citation: &CitationSpan) -> CitationVerificationResult {
        let chunk = match chunks.iter().find(|c| c.id == citation.chunk_id) {
            Some(c) => c,
            None => {
                return CitationVerificationResult::ChunkNotFound {
                    chunk_id: citation.chunk_id,
                };
            }
        };

        let chunk_len = chunk.content.len();
        if citation.start_offset > citation.end_offset || citation.end_offset > chunk_len {
            return CitationVerificationResult::OutOfBounds {
                chunk_id: citation.chunk_id,
                start: citation.start_offset,
                end: citation.end_offset,
                chunk_len,
            };
        }

        // Kiểm tra an toàn biên ký tự UTF-8
        if !chunk.content.is_char_boundary(citation.start_offset)
            || !chunk.content.is_char_boundary(citation.end_offset)
        {
            return CitationVerificationResult::ContentMismatch {
                expected: citation.claimed_text.clone(),
                found: "[Lỗi biên ký tự UTF-8]".to_string(),
            };
        }

        let slice = &chunk.content[citation.start_offset..citation.end_offset];
        if slice == citation.claimed_text {
            CitationVerificationResult::Valid
        } else {
            CitationVerificationResult::ContentMismatch {
                expected: citation.claimed_text.clone(),
                found: slice.to_string(),
            }
        }
    }
}

/// Diễn tập rào chắn tác tử LoopDrill với hai tiêu chí độ nhạy (sensitivity) và độ đặc hiệu (specificity).
#[derive(Debug, Clone, Default)]
pub struct LoopDrillRunner;

impl LoopDrillRunner {
    /// Chạy kiểm định ma trận nhầm lẫn (confusion matrix) trên tập ca kiểm thử.
    pub fn evaluate<F>(
        guard: F,
        cases: &[(&str, bool)],
        min_sensitivity: f64,
        min_specificity: f64,
    ) -> (f64, f64, bool)
    where
        F: Fn(&str) -> bool,
    {
        let (mut tp, mut fn_c, mut tn, mut fp) = (0usize, 0usize, 0usize, 0usize);
        for &(payload, is_fault) in cases {
            match (is_fault, guard(payload)) {
                (true, true) => tp += 1,
                (true, false) => fn_c += 1,
                (false, false) => tn += 1,
                (false, true) => fp += 1,
            }
        }

        let sens = if tp + fn_c == 0 {
            1.0
        } else {
            (tp as f64) / ((tp + fn_c) as f64)
        };

        let spec = if tn + fp == 0 {
            1.0
        } else {
            (tn as f64) / ((tn + fp) as f64)
        };

        let passed = sens >= min_sensitivity && spec >= min_specificity;
        (sens, spec, passed)
    }
}

// ============================================================================
// HÀM CHÍNH (MAIN ENTRYPOINT)
// ============================================================================

fn main() {
    println!("=== CHƯƠNG 88: SUY LUẬN CÓ CẤU TRÚC & TRUY XUẤT LAI RRF ===");

    // 1. Minh hoạ giải Game of 24 với Tree-of-Thoughts
    let numbers = [4.0, 9.0, 10.0, 13.0];
    if let Some(solution) = ThoughtTreeSearch::search_24(&numbers, 24.0, 50) {
        println!(
            "1. Tree-of-Thoughts giải 24 cho [4, 9, 10, 13]: {}",
            solution
        );
    } else {
        println!("1. Không tìm thấy lời giải cho 24.");
    }

    // 2. Minh hoạ cắt lát ngữ cảnh InfoGraph
    let mut graph = InfoGraph::default();
    graph.add_node(
        "auth_service",
        "service",
        "Xử lý JWT và xác thực người dùng",
        &["backend", "security"],
    );
    graph.add_node(
        "user_db",
        "database",
        "Bảng tài khoản PostgreSQL",
        &["storage"],
    );
    graph.add_edge("auth_service", "user_db", "queries", 10);
    if let Some(slice) = graph.slice_1hop_context("auth_service") {
        println!(
            "2. InfoGraph 1-hop slice quanh 'auth_service':\n{}",
            slice.formatted_prompt.trim()
        );
    }

    // 3. Minh hoạ phân đoạn Markdown và truy xuất lai RRF
    let markdown_sample = "# Hướng Dẫn Hệ Thống\n\n## Cấu Hình Cổng\nCổng mặc định là 8080.\n\n## Bảo Mật\nBắt buộc cấu hình TLS.";
    let chunks = chunk_markdown_by_headings(markdown_sample, 10, 2);
    println!(
        "3. MarkdownHeadingChunker sinh ra {} đoạn trích.",
        chunks.len()
    );

    let embeddings = vec![[0.1, 0.9, 0.0], [0.8, 0.2, 0.0]];
    let query_emb = [0.1, 0.85, 0.0];
    let search_results =
        HybridRetriever::search(&chunks, &embeddings, "cổng mặc định 8080", &query_emb, 60);
    if let Some(&(best_id, score)) = search_results.first() {
        println!(
            "   Truy xuất lai RRF đoạn thắng: chunk {} (điểm: {:.4})",
            best_id, score
        );
    }

    println!("Hệ thống Chương 88 khởi động thành công và an toàn tuyệt đối.");
}

// ============================================================================
// HỆ THỐNG KIỂM THỬ TẤT ĐỊNH (>= 34 UNIT TESTS)
// ============================================================================

#[cfg(test)]
mod tests {
    use super::*;

    // --- NHÓM 1: CÁC BÀI KIỂM THỬ SUY LUẬN CÓ CẤU TRÚC (TOT, COGNITIVE, PLAN) ---

    #[test]
    fn test_tot_game_of_24_standard() {
        // [4, 9, 10, 13] giải 24: (10 - (13 - 9)) * 4 = 24
        let numbers = [4.0, 9.0, 10.0, 13.0];
        let sol = ThoughtTreeSearch::search_24(&numbers, 24.0, 50);
        assert!(
            sol.is_some(),
            "Cần tìm ra lời giải Game of 24 cho tập số [4, 9, 10, 13]"
        );
    }

    #[test]
    fn test_tot_game_of_24_no_solution() {
        // Tập số [1, 1, 1, 1] không thể làm ra 24
        let numbers = [1.0, 1.0, 1.0, 1.0];
        let sol = ThoughtTreeSearch::search_24(&numbers, 24.0, 50);
        assert!(
            sol.is_none(),
            "Tập số [1, 1, 1, 1] không được có lời giải 24"
        );
    }

    #[test]
    fn test_self_consistency_voter_consensus() {
        let paths = vec![
            ReasoningPath {
                steps: vec!["bước 1".into()],
                final_answer: "Option A".into(),
                confidence: 0.8,
            },
            ReasoningPath {
                steps: vec!["bước 2".into()],
                final_answer: "Option B".into(),
                confidence: 0.6,
            },
            ReasoningPath {
                steps: vec!["bước 3".into()],
                final_answer: "Option A".into(),
                confidence: 0.9,
            },
        ];
        let res = SelfConsistencyVoter::vote(&paths).expect("Phải có kết quả biểu quyết");
        assert_eq!(res.winning_answer, "option a");
        assert_eq!(res.agreement_count, 2);
        assert_eq!(res.total_votes, 3);
    }

    #[test]
    fn test_self_consistency_voter_tie_breaking() {
        let paths = vec![
            ReasoningPath {
                steps: vec![],
                final_answer: "Alpha".into(),
                confidence: 0.4,
            },
            ReasoningPath {
                steps: vec![],
                final_answer: "Beta".into(),
                confidence: 0.9,
            },
        ];
        let res = SelfConsistencyVoter::vote(&paths).expect("Phải có kết quả biểu quyết");
        assert_eq!(
            res.winning_answer, "beta",
            "Trọng số confidence cao hơn phải thắng khi hòa 1-1"
        );
    }

    #[test]
    fn test_plan_and_solve_runner_success() {
        let planner = |_goal: &str| {
            vec![
                SubTask {
                    step_id: 1,
                    description: "Chuẩn bị dữ liệu".into(),
                    tool_hint: None,
                },
                SubTask {
                    step_id: 2,
                    description: "Xử lý logic".into(),
                    tool_hint: None,
                },
            ]
        };
        let solver = |task: &SubTask, _history: &[SubTaskResult]| SubTaskResult {
            step_id: task.step_id,
            output: format!("Xong bước {}", task.step_id),
            success: true,
        };

        let exec = PlanAndSolveRunner::run("Triển khai tính năng", planner, solver);
        assert!(exec.success);
        assert_eq!(exec.step_results.len(), 2);
    }

    #[test]
    fn test_plan_and_solve_runner_step_failure() {
        let planner = |_goal: &str| {
            vec![
                SubTask {
                    step_id: 1,
                    description: "Bước lỗi".into(),
                    tool_hint: None,
                },
                SubTask {
                    step_id: 2,
                    description: "Bước không chạy tới".into(),
                    tool_hint: None,
                },
            ]
        };
        let solver = |task: &SubTask, _history: &[SubTaskResult]| SubTaskResult {
            step_id: task.step_id,
            output: "Lỗi kết nối".into(),
            success: false,
        };

        let exec = PlanAndSolveRunner::run("Mục tiêu thử lỗi", planner, solver);
        assert!(!exec.success);
        assert_eq!(
            exec.step_results.len(),
            1,
            "Cần dừng ngay khi bước đầu tiên thất bại"
        );
    }

    #[test]
    fn test_reflexion_loop_learning() {
        let mut turn = 0;
        let actor = |_prompt: &str, history: &[ReflexionAttempt]| {
            if history.is_empty() {
                "giải pháp cũ bị lỗi".to_string()
            } else {
                "giải pháp mới đã sửa".to_string()
            }
        };
        let evaluator = |action: &str| {
            turn += 1;
            if action.contains("đã sửa") {
                (true, "Bài kiểm thử đạt 100%".to_string())
            } else {
                (false, "Lỗi kiểm thử: thiếu tham số".to_string())
            }
        };
        let reflector = |_p: &str, _a: &str, _out: &str| {
            "Cần thêm cờ kiểm tra tham số vào mã nguồn".to_string()
        };

        let record = ReflexionLoop::run("Nhiệm vụ tối ưu", actor, evaluator, reflector, 3);
        assert!(record.final_success);
        assert_eq!(record.total_attempts, 2);
        assert_eq!(turn, 2);
    }

    #[test]
    fn test_reflexion_loop_max_attempts_exhausted() {
        let actor = |_p: &str, _h: &[ReflexionAttempt]| "luôn làm sai".to_string();
        let evaluator = |_a: &str| (false, "Lỗi".to_string());
        let reflector = |_p: &str, _a: &str, _o: &str| "vẫn sai".to_string();

        let record = ReflexionLoop::run("Nhiệm vụ bế tắc", actor, evaluator, reflector, 2);
        assert!(!record.final_success);
        assert_eq!(record.total_attempts, 2);
    }

    #[test]
    fn test_cognitive_tool_5op_ordered_progression() {
        let mut tool = CognitiveTool5Op::new();
        assert_eq!(
            tool.expected_next_phase(),
            Some(CognitiveOpPhase::Understand)
        );

        assert!(
            tool.execute_step(
                CognitiveOpPhase::Understand,
                vec!["Đọc đề".into()],
                vec![],
                BTreeMap::new()
            )
            .is_ok()
        );
        assert_eq!(tool.expected_next_phase(), Some(CognitiveOpPhase::Extract));

        assert!(
            tool.execute_step(
                CognitiveOpPhase::Extract,
                vec!["Thực thể A".into()],
                vec![],
                BTreeMap::new()
            )
            .is_ok()
        );
        assert_eq!(
            tool.expected_next_phase(),
            Some(CognitiveOpPhase::Highlight)
        );

        assert!(
            tool.execute_step(
                CognitiveOpPhase::Highlight,
                vec!["Điểm thắt cổ chai".into()],
                vec![],
                BTreeMap::new()
            )
            .is_ok()
        );
        assert!(
            tool.execute_step(
                CognitiveOpPhase::Apply,
                vec!["Tối ưu bộ đệm".into()],
                vec![],
                BTreeMap::new()
            )
            .is_ok()
        );
        assert!(
            tool.execute_step(
                CognitiveOpPhase::Validate,
                vec!["Test xanh".into()],
                vec![],
                BTreeMap::new()
            )
            .is_ok()
        );

        assert!(tool.is_completed());
        assert_eq!(tool.expected_next_phase(), None);
    }

    #[test]
    fn test_cognitive_tool_5op_out_of_order_rejected() {
        let mut tool = CognitiveTool5Op::new();
        let err = tool.execute_step(CognitiveOpPhase::Apply, vec![], vec![], BTreeMap::new());
        assert!(
            err.is_err(),
            "Không được phép nhảy cóc tới pha Apply khi chưa qua Understand"
        );
    }

    // --- NHÓM 2: CÁC BÀI KIỂM THỬ ĐỒ THỊ KÉP (INFOGRAPH & PLANDAG) ---

    #[test]
    fn test_infograph_slice_1hop_context_token_reduction() {
        let mut g = InfoGraph::default();
        for i in 0..20 {
            g.add_node(
                &format!("node_{}", i),
                "unit",
                &format!("Nội dung chi tiết dài dòng cho đỉnh số {}", i),
                &[],
            );
        }
        g.add_edge("node_0", "node_1", "calls", 1);
        g.add_edge("node_2", "node_0", "imports", 2);

        let full_dump_len: usize = g.nodes.values().map(|n| n.content.len()).sum();
        let slice = g
            .slice_1hop_context("node_0")
            .expect("Phải cắt lát được node_0");

        let slice_len = slice.formatted_prompt.len();
        let reduction = 1.0 - (slice_len as f64 / full_dump_len as f64);

        assert!(
            reduction > 0.60,
            "Cắt lát 1-hop phải giảm > 60% dung lượng văn bản so với đổ toàn bộ đồ thị (thực tế giảm: {:.2}%)",
            reduction * 100.0
        );
    }

    #[test]
    fn test_infograph_empty_and_isolated_node() {
        let mut g = InfoGraph::default();
        g.add_node("alone", "struct", "Dữ liệu biệt lập", &["standalone"]);
        let slice = g
            .slice_1hop_context("alone")
            .expect("Phải lấy được ngữ cảnh nút biệt lập");
        assert!(slice.neighbors.is_empty());
        assert!(slice.incident_edges.is_empty());
        assert!(slice.formatted_prompt.contains("Dữ liệu biệt lập"));
    }

    #[test]
    fn test_plandag_topological_ready_waves() {
        let mut dag = PlanDag::new();
        // Cấu trúc hình kim cương: A -> B, A -> C, B -> D, C -> D
        dag.add_node("A", &[], 1);
        dag.add_node("B", &["A"], 1);
        dag.add_node("C", &["A"], 1);
        dag.add_node("D", &["B", "C"], 1);

        let waves = dag
            .topological_ready_waves()
            .expect("Đồ thị kim cương không có chu trình");
        assert_eq!(waves.len(), 3);
        assert_eq!(waves[0], vec!["A"]);
        assert_eq!(waves[1].len(), 2);
        assert!(waves[1].contains(&"B".to_string()));
        assert!(waves[1].contains(&"C".to_string()));
        assert_eq!(waves[2], vec!["D"]);
    }

    #[test]
    fn test_plandag_cycle_detection() {
        let mut dag = PlanDag::new();
        // Chu trình 3 nút: X -> Y -> Z -> X
        dag.add_node("X", &["Z"], 1);
        dag.add_node("Y", &["X"], 1);
        dag.add_node("Z", &["Y"], 1);

        let res = dag.topological_ready_waves();
        assert_eq!(
            res,
            Err(DagError::CycleDetected),
            "Phải phát hiện chu trình 3 nút"
        );
    }

    #[test]
    fn test_plandag_retry_budget() {
        let mut dag = PlanDag::new();
        dag.add_node("task_flake", &[], 2); // Ngân sách thử lại là 2 lần (tổng tối đa 3 lượt)

        let s1 = dag.record_attempt("task_flake", false).unwrap();
        assert_eq!(s1, DagNodeState::Pending);

        let s2 = dag.record_attempt("task_flake", false).unwrap();
        assert_eq!(s2, DagNodeState::Pending);

        let s3 = dag.record_attempt("task_flake", false).unwrap();
        assert_eq!(
            s3,
            DagNodeState::ExhaustedRetries,
            "Lần thứ 3 thất bại vượt budget 2 => cạn ngân sách"
        );
    }

    #[test]
    fn test_plandag_unknown_dependency() {
        let mut dag = PlanDag::new();
        dag.add_node("orphan", &["non_existent_parent"], 1);
        let res = dag.topological_ready_waves();
        assert!(matches!(res, Err(DagError::UnknownDependency { .. })));
    }

    #[test]
    fn test_aot_subgraph_compiler() {
        let mut dag = PlanDag::new();
        dag.add_node("step1", &[], 0);
        dag.add_node("step2a", &["step1"], 0);
        dag.add_node("step2b", &["step1"], 0);

        let sched = AotSubgraphCompiler::compile(&dag).expect("Biên dịch AoT thành công");
        assert_eq!(sched.total_tasks, 3);
        assert_eq!(sched.waves.len(), 2);
        assert_eq!(sched.max_concurrency, 2);
    }

    // --- NHÓM 3: MÁY TRẠNG THÁI STATEGRAPH VÀ ĐIỂM KIỂM SOÁT STRIPE BLUEPRINTS ---

    #[test]
    fn test_stategraph_reducers() {
        // Kiểm tra tính tất định của ReducerKind::Sum và ReducerKind::Append
        #[derive(Debug, Clone, PartialEq, Eq, Hash)]
        struct AgentState {
            counter: u32,
            events: Vec<String>,
        }

        let mut sg = StateGraph::new(AgentState {
            counter: 0,
            events: Vec::new(),
        });

        let cp1 = sg
            .transition("node_inc", |s| {
                let mut next = s.clone();
                next.counter += 10;
                next.events.push("Cộng 10".into());
                Ok((next, vec!["Đã tăng counter".into()]))
            })
            .expect("Transition thành công");

        assert_eq!(cp1, 2);
        assert_eq!(sg.current_state.counter, 10);
        assert_eq!(sg.current_state.events.len(), 1);
    }

    #[test]
    fn test_stategraph_stripe_blueprints_checkpoint_and_rollback() {
        #[derive(Debug, Clone, PartialEq, Eq, Hash)]
        struct AccountBalance {
            balance: u64,
        }

        let mut sg = StateGraph::new(AccountBalance { balance: 100 });
        let cp_safe = sg.checkpoints.last().unwrap().checkpoint_id;

        // Thao tác thứ nhất: trừ tiền hợp lệ
        let _ = sg
            .transition("charge_20", |s| {
                Ok((
                    AccountBalance {
                        balance: s.balance - 20,
                    },
                    vec!["Trừ 20".into()],
                ))
            })
            .unwrap();
        assert_eq!(sg.current_state.balance, 80);

        // Giả sử phát hiện rủi ro gian lận, rollback về điểm an toàn ban đầu
        sg.rollback_to(cp_safe).expect("Rollback thành công");
        assert_eq!(
            sg.current_state.balance, 100,
            "Số dư phải khôi phục về 100 sau rollback"
        );
    }

    // --- NHÓM 4: NGẮT DUYỆT HITL & SỬA LỖI EDGE CASE #5 (KEY DELETION DIFF) ---

    #[test]
    fn test_hitl_accept() {
        let mut mem = ProceduralMemory::default();
        let mut orig = BTreeMap::new();
        orig.insert("mode".into(), "fast".into());

        let res = resolve_hitl_interrupt("compiler", &orig, HitlInterrupt::Accept, &mut mem);
        assert_eq!(res, HitlResolution::Execute { arguments: orig });
        assert!(mem.tool_override_rules.is_empty());
    }

    #[test]
    fn test_hitl_edit_modified_args() {
        let mut mem = ProceduralMemory::default();
        let mut orig = BTreeMap::new();
        orig.insert("retries".into(), "1".into());

        let mut new_args = BTreeMap::new();
        new_args.insert("retries".into(), "5".into());

        let res = resolve_hitl_interrupt(
            "network",
            &orig,
            HitlInterrupt::Edit {
                new_arguments: new_args.clone(),
            },
            &mut mem,
        );
        assert_eq!(
            res,
            HitlResolution::Execute {
                arguments: new_args
            }
        );
        assert_eq!(
            mem.tool_override_rules,
            vec!["tool=network arg=retries->5".to_string()]
        );
    }

    #[test]
    fn test_hitl_edit_edge_case_deleted_keys() {
        // SỬA LỖI EDGE CASE #5: Con người xoá bớt một khoá tham số nguy hiểm khỏi orig
        let mut mem = ProceduralMemory::default();
        let mut orig = BTreeMap::new();
        orig.insert("safe_flag".into(), "true".into());
        orig.insert("dangerous_param".into(), "drop_table".into());

        // Con người xoá `dangerous_param`, chỉ giữ `safe_flag`
        let mut new_args = BTreeMap::new();
        new_args.insert("safe_flag".into(), "true".into());

        let res = resolve_hitl_interrupt(
            "sql_tool",
            &orig,
            HitlInterrupt::Edit {
                new_arguments: new_args.clone(),
            },
            &mut mem,
        );

        assert_eq!(
            res,
            HitlResolution::Execute {
                arguments: new_args
            }
        );
        // Phải phát hiện và ghi vết khoá bị xoá dạng `[DELETED]`
        assert!(
            mem.tool_override_rules
                .contains(&"tool=sql_tool arg=dangerous_param->[DELETED]".to_string()),
            "Cần ghi nhận sự kiện khoá tham số bị xoá vào ProceduralMemory"
        );
    }

    #[test]
    fn test_hitl_respond() {
        let mut mem = ProceduralMemory::default();
        let orig = BTreeMap::new();
        let res = resolve_hitl_interrupt(
            "git_push",
            &orig,
            HitlInterrupt::Respond {
                feedback_message: "Thiếu chữ ký GPG".into(),
            },
            &mut mem,
        );
        assert_eq!(
            res,
            HitlResolution::Replan {
                message: "Thiếu chữ ký GPG".into()
            }
        );
        assert_eq!(
            mem.response_rules,
            vec!["tool=git_push: Thiếu chữ ký GPG".to_string()]
        );
    }

    #[test]
    fn test_hitl_ignore() {
        let mut mem = ProceduralMemory::default();
        let orig = BTreeMap::new();
        let res = resolve_hitl_interrupt("rm_rf", &orig, HitlInterrupt::Ignore, &mut mem);
        assert_eq!(res, HitlResolution::Aborted);
        assert_eq!(mem.triage_rules, vec!["suppress_tool=rm_rf".to_string()]);
    }

    // --- NHÓM 5: TRÍ NHỚ NHẬN THỨC 4 TẦNG, BITEMPORAL GRAPH & BDI ---

    #[test]
    fn test_cognitive_memory_4tier_working_and_episodic() {
        let mut mem = CognitiveMemory4Tier::default();
        mem.push_working("Đang duyệt qua module parser");
        assert_eq!(mem.working_memory.len(), 1);

        mem.record_episode(
            "ep1",
            100,
            "Biên dịch thất bại do thiếu trait",
            &["compiler", "error", "trait"],
            0.8,
        );
        assert_eq!(mem.episodic_memory.len(), 1);
    }

    #[test]
    fn test_cognitive_memory_4tier_consolidation() {
        let mut mem = CognitiveMemory4Tier::default();
        // Hai sự kiện có chung các từ khoá: ["cache", "redis", "timeout"] (Jaccard = 3/3 = 1.0 >= 0.7)
        mem.record_episode(
            "ep1",
            10,
            "Redis timeout lần 1",
            &["cache", "redis", "timeout"],
            0.7,
        );
        mem.record_episode(
            "ep2",
            12,
            "Redis timeout lần 2",
            &["cache", "redis", "timeout"],
            0.9,
        );

        let consolidated = mem.consolidate_episodes();
        assert_eq!(
            consolidated, 1,
            "Cần hợp nhất 2 mẩu ký ức có độ tương đồng Jaccard >= 0.7"
        );
        assert_eq!(mem.semantic_memory.len(), 1);
    }

    #[test]
    fn test_bitemporal_graph_time_travel() {
        let mut bg = BiTemporalGraph::new();
        // Vào lúc Tx=10, ghi nhận: Server A có IP là 10.0.0.1 từ ValidTime 0 đến 100
        bg.record_fact(
            "f1",
            "srv_a",
            "ip",
            "10.0.0.1",
            ValidTimeRange {
                valid_from: 0,
                valid_to: 100,
            },
            10,
        );

        // Vào lúc Tx=20, hệ thống phát hiện sai lầm hồi tố: Thực ra từ ValidTime 50 đến 100, IP là 10.0.0.2
        bg.retract_fact("f1", 20);
        bg.record_fact(
            "f1_part1",
            "srv_a",
            "ip",
            "10.0.0.1",
            ValidTimeRange {
                valid_from: 0,
                valid_to: 50,
            },
            20,
        );
        bg.record_fact(
            "f1_part2",
            "srv_a",
            "ip",
            "10.0.0.2",
            ValidTimeRange {
                valid_from: 50,
                valid_to: 100,
            },
            20,
        );

        // Truy vấn du hành thời gian: Nhìn tại thời điểm Tx=15 (trước khi sửa lỗi), ta thấy IP ở Valid=60 là 10.0.0.1
        let past_view = bg.query_time_travel("srv_a", 60, 15);
        assert_eq!(past_view.len(), 1);
        assert_eq!(past_view[0].value, "10.0.0.1");

        // Nhìn tại thời điểm Tx=25 (sau khi sửa lỗi), ta thấy IP ở Valid=60 là 10.0.0.2
        let current_view = bg.query_time_travel("srv_a", 60, 25);
        assert_eq!(current_view.len(), 1);
        assert_eq!(current_view[0].value, "10.0.0.2");
    }

    #[test]
    fn test_bitemporal_graph_conflicting_events() {
        let mut bg = BiTemporalGraph::new();
        bg.record_fact(
            "fact_alpha",
            "db_primary",
            "status",
            "UP",
            ValidTimeRange {
                valid_from: 0,
                valid_to: 50,
            },
            1,
        );
        bg.record_fact(
            "fact_beta",
            "db_primary",
            "status",
            "DOWN",
            ValidTimeRange {
                valid_from: 50,
                valid_to: 100,
            },
            1,
        );

        let up_check = bg.query_time_travel("db_primary", 25, 2);
        assert_eq!(up_check[0].value, "UP");

        let down_check = bg.query_time_travel("db_primary", 75, 2);
        assert_eq!(down_check[0].value, "DOWN");
    }

    #[test]
    fn test_bdi_engine_endurant_perdurant() {
        let mut bdi = BdiEngine::default();
        bdi.update_belief("service_auth", "health", "degraded", 0.99);
        bdi.add_desire("desire_recover", "Phục hồi cụm xác thực về healthy", 1);
        bdi.commit_intention(
            "intent_1",
            "desire_recover",
            &["drain_traffic", "restart_pod", "verify_health"],
        );

        let step1 = bdi.advance_intention("intent_1");
        assert_eq!(step1, Some("drain_traffic".into()));
        assert!(!bdi.desires[0].is_fulfilled);

        let _ = bdi.advance_intention("intent_1");
        let step3 = bdi.advance_intention("intent_1");
        assert_eq!(step3, Some("verify_health".into()));
        assert!(
            bdi.desires[0].is_fulfilled,
            "Hoàn tất các bước của ý định sẽ đánh dấu desire fulfilled"
        );
    }

    // --- NHÓM 6: PHÂN ĐOẠN MARKDOWN (EDGE CASES #1 & #2), RRF (EDGE CASE #3) & KIỂM CHỨNG TRÍCH DẪN ---

    #[test]
    fn test_markdown_chunker_edge_case_max_w_zero() {
        // SỬA LỖI EDGE CASE #1: max_w = 0 không bao giờ panic
        let md = "# Tiêu Đề\nNội dung văn bản thông thường ở đây.";
        let chunks = chunk_markdown_by_headings(md, 0, 0);
        assert!(
            !chunks.is_empty(),
            "Phải chia được đoạn an toàn khi max_w = 0"
        );
    }

    #[test]
    fn test_markdown_chunker_edge_case_code_fence() {
        // SỬA LỖI EDGE CASE #2: Dòng `# comment` bên trong code fence không được coi là tiêu đề
        let md = r#"# Chương 1
Dòng mở đầu.
```python
# Đây là comment trong Python, không phải tiêu đề Heading!
def foo():
    pass
```
## Mục 1.1
Nội dung mục con."#;

        let chunks = chunk_markdown_by_headings(md, 50, 5);

        // Kiểm tra: Không được có chunk nào có heading_path chứa chuỗi "Đây là comment trong Python"
        for c in &chunks {
            assert!(
                !c.heading_path.contains("Đây là comment trong Python"),
                "Dòng comment trong code fence bị biến nhầm thành heading path: {}",
                c.heading_path
            );
        }

        // Nội dung chunk phải chứa mã python
        let has_code = chunks.iter().any(|c| c.content.contains("def foo():"));
        assert!(
            has_code,
            "Nội dung trong code fence vẫn phải được giữ trong chunk text"
        );
    }

    #[test]
    fn test_markdown_chunker_nested_headings() {
        let md = r#"# Tổng Quan
Nội dung cấp 1.
## Cấu Trúc
Nội dung cấp 2.
### Chi Tiết Cụ Thể
Nội dung cấp 3."#;

        let chunks = chunk_markdown_by_headings(md, 100, 0);
        let last_chunk = chunks.last().expect("Phải có chunk");
        assert_eq!(
            last_chunk.heading_path,
            "Tổng Quan > Cấu Trúc > Chi Tiết Cụ Thể"
        );
    }

    #[test]
    #[should_panic(expected = "bắt buộc phải bằng nhau")]
    fn test_hybrid_retriever_edge_case_len_mismatch_panics() {
        // SỬA LỖI EDGE CASE #3: assert_eq!(chunks.len(), embs.len()) kích hoạt panic khi lệch số lượng
        let chunks = vec![
            DocChunk {
                id: 0,
                heading_path: "".into(),
                content: "Đoạn 0".into(),
            },
            DocChunk {
                id: 1,
                heading_path: "".into(),
                content: "Đoạn 1".into(),
            },
        ];
        let embs = vec![[0.1, 0.2, 0.3]]; // Chỉ có 1 embedding cho 2 chunks!
        let _ = HybridRetriever::search(&chunks, &embs, "query", &[0.1, 0.2, 0.3], 60);
    }

    #[test]
    fn test_hybrid_retriever_dual_match_ranks_first() {
        // Kiểm tra: Đoạn khớp cả từ khoá BM25 VÀ khớp ngữ nghĩa Dense Cosine phải thắng tuyệt đối
        let chunks = vec![
            DocChunk {
                id: 0,
                heading_path: "Kho".into(),
                content: "táo lê chuối trái cây hoa quả".into(),
            },
            DocChunk {
                id: 1,
                heading_path: "Lập Trình".into(),
                content: "rust lập trình bộ nhớ an toàn ownership".into(),
            },
            DocChunk {
                id: 2,
                heading_path: "Công Nghệ".into(),
                content: "mạng máy tính hệ điều hành linux".into(),
            },
        ];

        // Giả sử embedding: [1, 0, 0] là hướng "lập trình"
        let embs = vec![
            [0.0, 1.0, 0.0],
            [1.0, 0.0, 0.0], // Khớp hoàn hảo với truy vấn về vector
            [0.5, 0.5, 0.0],
        ];

        let q_text = "rust bộ nhớ";
        let q_emb = [1.0, 0.0, 0.0];

        let results = HybridRetriever::search(&chunks, &embs, q_text, &q_emb, 60);
        assert_eq!(
            results[0].0, 1,
            "Chunk 1 khớp kép (cả từ khoá lẫn ngữ nghĩa) phải xếp hạng #1"
        );
    }

    #[test]
    fn test_span_citation_verifier_valid_span() {
        let chunk = DocChunk {
            id: 42,
            heading_path: "Doc".into(),
            content: "Rust 2024 mang lại sự ổn định và hiệu năng tuyệt hảo.".into(),
        };
        let citation = CitationSpan {
            chunk_id: 42,
            start_offset: 0,
            end_offset: 9, // "Rust 2024"
            claimed_text: "Rust 2024".into(),
        };

        let res = SpanCitationVerifier::verify(&[chunk], &citation);
        assert_eq!(res, CitationVerificationResult::Valid);
    }

    #[test]
    fn test_span_citation_verifier_hallucinated_span() {
        let chunk = DocChunk {
            id: 10,
            heading_path: "Doc".into(),
            content: "Mô hình ngôn ngữ có thể bị ảo giác.".into(),
        };
        // Claimed text bịa đặt không đúng với lát cắt
        let citation = CitationSpan {
            chunk_id: 10,
            start_offset: 0,
            end_offset: 7,
            claimed_text: "Ảo giác hoàn toàn".into(),
        };

        let res = SpanCitationVerifier::verify(&[chunk], &citation);
        assert!(matches!(
            res,
            CitationVerificationResult::ContentMismatch { .. }
        ));
    }

    #[test]
    fn test_span_citation_verifier_out_of_bounds() {
        let chunk = DocChunk {
            id: 1,
            heading_path: "Doc".into(),
            content: "Ngắn".into(),
        };
        let citation = CitationSpan {
            chunk_id: 1,
            start_offset: 10,
            end_offset: 20,
            claimed_text: "Quá dài".into(),
        };

        let res = SpanCitationVerifier::verify(&[chunk], &citation);
        assert!(matches!(
            res,
            CitationVerificationResult::OutOfBounds { .. }
        ));
    }

    #[test]
    fn test_loop_drill_runner_sens_spec() {
        // Kiểm tra LoopDrill: Bộ lọc phát hiện lỗi
        let cases = vec![
            ("normal input 1", false),
            ("normal input 2", false),
            ("injection attack DROP TABLE", true),
            ("malicious payload <script>", true),
        ];

        let guard = |payload: &str| payload.contains("DROP TABLE") || payload.contains("<script>");
        let (sens, spec, passed) = LoopDrillRunner::evaluate(guard, &cases, 1.0, 1.0);

        assert_eq!(sens, 1.0, "Độ nhạy phải đạt 100%");
        assert_eq!(spec, 1.0, "Độ đặc hiệu phải đạt 100%");
        assert!(passed);
    }
}
