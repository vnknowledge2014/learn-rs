#![allow(dead_code, unused_variables)]
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
pub struct ContextPack {
    pub chunks: Vec<ContextChunk>,
    pub tong_token: usize,
    pub samples_is_kind: usize,
}

/// CONTEXT ENGINEERING: chọn tập con ngữ cảnh tốt nhất trong ngân sách token.
/// Đây là bài toán xếp ba lô (knapsack) đơn giản hóa: ưu tiên điểm liên quan
/// trên mỗi token, và luôn giữ các mẩu bị ghim.
pub fn pack_context(mut mau: Vec<ContextChunk>, ngan_sach: usize) -> ContextPack {
    let first_total_sell = mau.len();

    // 1. Tách phần ghim cứng — luôn được nạp trước
    let (ghim, mut tuy_chon): (Vec<_>, Vec<_>) = mau.drain(..).partition(|m| m.ghim);
    let mut da_dung: usize = ghim.iter().map(|m| m.token).sum();
    let mut pick: Vec<ContextChunk> = ghim;

    // 2. Xếp phần còn lại theo MẬT ĐỘ giá trị (liên quan / token) giảm dần
    tuy_chon.sort_by(|a, b| {
        let id = a.lien_quan / a.token.max(1) as f64;
        let mb = b.lien_quan / b.token.max(1) as f64;
        mb.partial_cmp(&id)
            .unwrap_or(std::cmp::Ordering::Equal)
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

    ContextPack {
        tong_token: da_dung,
        samples_is_kind: first_total_sell - pick.len(),
        chunks: pick,
    }
}

/// Chống hiện tượng "Lost in the Middle": mô hình nhớ tốt phần đầu và phần cuối,
/// hay quên phần giữa. Vậy hãy đẩy thứ quan trọng nhất ra hai đầu.
pub fn forgetting_resistant_sort(mut mau: Vec<ContextChunk>) -> Vec<ContextChunk> {
    mau.sort_by(|a, b| {
        b.lien_quan
            .partial_cmp(&a.lien_quan)
            .unwrap_or(std::cmp::Ordering::Equal)
            .then(a.nhan.cmp(&b.nhan))
    });
    let mut first: Vec<ContextChunk> = Vec::new();
    let mut last: Vec<ContextChunk> = Vec::new();
    for (i, m) in mau.into_iter().enumerate() {
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
        "tinh_tong"
    }
    fn description(&self) -> &str {
        "Cộng các số cách nhau bởi dấu phẩy. Ví dụ: \"3,4,5\""
    }
    fn run(&self, param: &str) -> ToolResult {
        let mut tong: i64 = 0;
        for part in param.split(',') {
            match part.trim().parse::<i64>() {
                Ok(n) => tong += n,
                Err(_) => return ToolResult::Failed(format!("{:?} không phải số", part.trim())),
            }
        }
        ToolResult::Finished(tong.to_string())
    }
}

pub struct LookupTool {
    pub store: HashMap<String, String>,
}
impl Tool for LookupTool {
    fn name(&self) -> &str {
        "tra_cuu"
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
    pub so_lan_goi_toi_da: usize,
}

impl Harness {
    pub fn new(so_lan_goi_toi_da: usize) -> Self {
        Harness {
            tools: Vec::new(),
            so_lan_goi_toi_da,
        }
    }
    pub fn register(mut self, cc: Box<dyn Tool>) -> Self {
        self.tools.push(cc);
        self
    }
    /// Bản mô tả công cụ để nhét vào ngữ cảnh — đây là "giao diện" tác tử nhìn thấy.
    pub fn legacy_open_gate(&self) -> String {
        self.tools
            .iter()
            .map(|c| format!("- {}: {}", c.name(), c.description()))
            .collect::<Vec<_>>()
            .join("\n")
    }
    pub fn goi(&self, name: &str, param: &str) -> ToolResult {
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
    fn decide(&self, nhiem_vu: &str, history: &[String]) -> Action;
}

#[derive(Debug, PartialEq)]
pub enum StopReason {
    Done,
    HetLuotGoi,
    LapVoHan,
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
pub fn run_agent_loop(nhiem_vu: &str, which: &dyn Brain, frame: &Harness) -> LoopResult {
    let mut history: Vec<String> = Vec::new();
    let mut seen: HashSet<String> = HashSet::new();

    for step in 1..=frame.so_lan_goi_toi_da {
        match which.decide(nhiem_vu, &history) {
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
                // DỪNG #3: phát hiện lặp vô hạn (gọi y hệt lần trước)
                let first_van_manual = format!("{}::{}", name, param);
                if !seen.insert(first_van_manual.clone()) {
                    history.push(format!("[{}] PHÁT HIỆN LẶP: {}", step, first_van_manual));
                    return LoopResult {
                        answer: None,
                        num_steps: step,
                        stop_reason: StopReason::LapVoHan,
                        log: history,
                    };
                }
                let kq = frame.goi(&name, &param);
                history.push(match kq {
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
        num_steps: frame.so_lan_goi_toi_da,
        stop_reason: StopReason::HetLuotGoi,
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
    pub fn add_relation(&mut self, tu: &str, nhan: &str, den: &str) {
        self.edge
            .entry(tu.to_string())
            .or_default()
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
            if next >= do_sau {
                continue;
            }
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
pub struct FakeBrain {
    pub scenarios: Vec<Action>,
}
impl Brain for FakeBrain {
    fn decide(&self, _nhiem_vu: &str, history: &[String]) -> Action {
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
    let mau = vec![
        ContextChunk {
            nhan: "quy_tac_an_toan".into(),
            content: "Không tiết lộ khóa bí mật".into(),
            token: 50,
            lien_quan: 0.3,
            ghim: true,
        },
        ContextChunk {
            nhan: "tai_lieu_A".into(),
            content: "...".into(),
            token: 800,
            lien_quan: 0.9,
            ghim: false,
        },
        ContextChunk {
            nhan: "tai_lieu_B".into(),
            content: "...".into(),
            token: 200,
            lien_quan: 0.85,
            ghim: false,
        },
        ContextChunk {
            nhan: "tai_lieu_C".into(),
            content: "...".into(),
            token: 2000,
            lien_quan: 0.95,
            ghim: false,
        },
        ContextChunk {
            nhan: "lich_su_chat_cu".into(),
            content: "...".into(),
            token: 900,
            lien_quan: 0.1,
            ghim: false,
        },
    ];
    let goi = pack_context(mau, 1000);
    println!(
        "   Dùng {} / 1000 token, loại bỏ {} mẩu",
        goi.tong_token, goi.samples_is_kind
    );
    for m in &goi.chunks {
        println!(
            "     [{:>4} tok · lq {:.2}{}] {}",
            m.token,
            m.lien_quan,
            if m.ghim { " · GHIM" } else { "" },
            m.nhan
        );
    }
    println!("   → tai_lieu_C (2000 tok) bị loại dù liên quan cao nhất: KHÔNG VỪA ngân sách.");
    println!("   → Thứ tự đã đảo để mẩu quan trọng nằm ở ĐẦU và CUỐI (chống Lost-in-the-Middle).");

    // ---- 2 & 3. HARNESS + LOOP ----
    println!("\n2-3. BỘ KHUNG & VÒNG LẶP TÁC TỬ");
    let mut store = HashMap::new();
    store.insert(
        "Rust".to_string(),
        "Ngôn ngữ hệ thống an toàn bộ nhớ".to_string(),
    );
    let frame = Harness::new(5)
        .register(Box::new(CalculatorTool))
        .register(Box::new(LookupTool { store }));
    println!(
        "   Công cụ tác tử được phép dùng:\n{}",
        frame.legacy_open_gate()
    );

    let which = FakeBrain {
        scenarios: vec![
            Action::CallTool {
                name: "tra_cuu".into(),
                param: "Rust".into(),
            },
            Action::CallTool {
                name: "tinh_tong".into(),
                param: "10,20,12".into(),
            },
            Action::Answer("Rust là ngôn ngữ hệ thống; tổng là 42.".into()),
        ],
    };
    let kq = run_agent_loop("Tra cứu Rust rồi cộng 10+20+12", &which, &frame);
    for d in &kq.log {
        println!("   {}", d);
    }
    println!("   Dừng vì: {:?} sau {} bước", kq.stop_reason, kq.num_steps);

    // Vòng lặp hỏng: tác tử lặp mãi một lời gọi
    let which_link = FakeBrain {
        scenarios: vec![
            Action::CallTool {
                name: "tra_cuu".into(),
                param: "X".into(),
            },
            Action::CallTool {
                name: "tra_cuu".into(),
                param: "X".into(),
            },
        ],
    };
    let kq2 = run_agent_loop("nhiệm vụ hỏng", &which_link, &frame);
    println!(
        "   [Tác tử kẹt] dừng vì: {:?} sau {} bước",
        kq2.stop_reason, kq2.num_steps
    );

    // ---- 4. GRAPH ENGINEERING ----
    println!("\n4. ĐỒ THỊ TRI THỨC — truy xuất lan tỏa 2 bước");
    let mut g = KnowledgeGraph::new();
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

    fn mau(nhan: &str, token: usize, lq: f64, ghim: bool) -> ContextChunk {
        ContextChunk {
            nhan: nhan.into(),
            content: "x".into(),
            token,
            lien_quan: lq,
            ghim,
        }
    }

    #[test]
    fn context_never_exceeds_budget() {
        let list = vec![
            mau("a", 400, 0.9, false),
            mau("b", 400, 0.8, false),
            mau("c", 400, 0.7, false),
        ];
        let g = pack_context(list, 1000);
        assert!(g.tong_token <= 1000, "vượt ngân sách: {}", g.tong_token);
        assert_eq!(g.chunks.len(), 2);
    }

    #[test]
    fn pinned_items_are_always_kept() {
        let list = vec![
            mau("quy_tac", 100, 0.01, true), // liên quan cực thấp nhưng GHIM
            mau("to", 900, 0.99, false),
        ];
        let g = pack_context(list, 1000);
        assert!(
            g.chunks.iter().any(|m| m.nhan == "quy_tac"),
            "mẩu ghim bị loại!"
        );
    }

    #[test]
    fn ranks_by_value_density_not_raw_score() {
        // "nho" có điểm thấp hơn nhưng mật độ (lq/token) cao hơn nhiều
        let list = vec![mau("to", 900, 0.9, false), mau("nho", 90, 0.5, false)];
        let g = pack_context(list, 500);
        assert_eq!(g.chunks.len(), 1);
        assert_eq!(g.chunks[0].nhan, "nho");
    }

    #[test]
    fn anti_forgetting_puts_key_items_at_both_ends() {
        let list = vec![
            mau("a", 1, 0.9, false),
            mau("b", 1, 0.5, false),
            mau("c", 1, 0.8, false),
        ];
        let sx = forgetting_resistant_sort(list);
        // xếp giảm dần: a(.9) c(.8) b(.5) -> chẵn ra đầu, lẻ ra cuối (đảo): a, b, c
        assert_eq!(sx.first().unwrap().nhan, "a");
        assert_eq!(sx.last().unwrap().nhan, "c");
    }

    #[test]
    fn tool_answers_correctly_and_errors_clearly() {
        let cc = CalculatorTool;
        assert_eq!(cc.run("1,2,3"), ToolResult::Finished("6".into()));
        assert!(matches!(cc.run("1,x"), ToolResult::Failed(_)));
    }

    #[test]
    fn harness_rejects_unregistered_tools() {
        let frame = Harness::new(3).register(Box::new(CalculatorTool));
        // Tác tử KHÔNG THỂ gọi thứ không được đăng ký — đây là ranh giới an toàn.
        assert!(matches!(
            frame.goi("xoa_o_cung", "/"),
            ToolResult::Failed(_)
        ));
    }

    #[test]
    fn loop_stops_on_completion() {
        let frame = Harness::new(5).register(Box::new(CalculatorTool));
        let which = FakeBrain {
            scenarios: vec![
                Action::CallTool {
                    name: "tinh_tong".into(),
                    param: "40,2".into(),
                },
                Action::Answer("42".into()),
            ],
        };
        let kq = run_agent_loop("nv", &which, &frame);
        assert_eq!(kq.stop_reason, StopReason::Done);
        assert_eq!(kq.answer, Some("42".to_string()));
        assert_eq!(kq.num_steps, 2);
    }

    #[test]
    fn loop_stops_when_out_of_calls() {
        let frame = Harness::new(3).register(Box::new(CalculatorTool));
        // Bộ não không bao giờ trả lời, chỉ gọi công cụ với tham số KHÁC nhau
        let which = FakeBrain {
            scenarios: vec![
                Action::CallTool {
                    name: "tinh_tong".into(),
                    param: "1".into(),
                },
                Action::CallTool {
                    name: "tinh_tong".into(),
                    param: "2".into(),
                },
                Action::CallTool {
                    name: "tinh_tong".into(),
                    param: "3".into(),
                },
                Action::CallTool {
                    name: "tinh_tong".into(),
                    param: "4".into(),
                },
            ],
        };
        let kq = run_agent_loop("nv", &which, &frame);
        assert_eq!(kq.stop_reason, StopReason::HetLuotGoi);
        assert_eq!(kq.num_steps, 3, "phải dừng đúng ở ngân sách 3 lượt");
    }

    #[test]
    fn loop_detects_stuck_agent() {
        let frame = Harness::new(50).register(Box::new(CalculatorTool));
        let which = FakeBrain {
            scenarios: vec![
                Action::CallTool {
                    name: "tinh_tong".into(),
                    param: "1".into(),
                },
                Action::CallTool {
                    name: "tinh_tong".into(),
                    param: "1".into(),
                }, // y hệt
            ],
        };
        let kq = run_agent_loop("nv", &which, &frame);
        assert_eq!(kq.stop_reason, StopReason::LapVoHan);
        assert!(kq.num_steps < 50, "phải dừng SỚM, không chạy hết 50 lượt");
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

        let sau1 = g.broadcast_access("A", 1);
        assert!(sau1.iter().any(|s| s.starts_with("B:")));
        assert!(
            !sau1.iter().any(|s| s.starts_with("C:")),
            "độ sâu 1 không được tới C"
        );

        let sau2 = g.broadcast_access("A", 2);
        assert!(
            sau2.iter().any(|s| s.starts_with("C:")),
            "độ sâu 2 phải tới được C"
        );
        assert!(!sau2.iter().any(|s| s.starts_with("D:")));
    }

    #[test]
    fn graph_walk_terminates_on_cycles() {
        let mut g = KnowledgeGraph::new();
        g.add_entity("A", "a");
        g.add_entity("B", "b");
        g.add_relation("A", "r", "B");
        g.add_relation("B", "r", "A"); // chu trình
        let kq = g.broadcast_access("A", 10);
        assert!(kq.len() < 10, "phải dừng nhờ tập đã thăm, không lặp vô hạn");
    }
}
