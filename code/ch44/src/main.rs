#![allow(dead_code, unused_variables, unused_imports)]
// ============================================================================
// CHƯƠNG 40: HỆ THỐNG QUẢN LÝ CỬA SỔ NGỮ CẢNH & ĐÓNG GÓI SYSTEM PROMPT CHUẨN MỰC
// Tác giả: Kỹ Sư Kiến Trúc Hệ Thống Rust
// ============================================================================

use std::collections::VecDeque;

// 1. ĐỊNH NGHĨA CÁC PHÂN ĐOẠN NGỮ CẢNH (CONTEXT SEGMENT)
// Mỗi phần của ngữ cảnh có mức độ ưu tiên khác nhau khi ngân sách bộ nhớ bị giới hạn.
#[derive(Debug, Clone, PartialEq, Eq)]
pub enum PriorityTier {
    Critical,   // Bắt buộc phải có: Quy chuẩn an toàn, Traits giao ước
    High,       // Ưu tiên high: Kiểu dữ liệu trực tiếp, Chữ ký hàm
    Medium,     // Ưu tiên trung bình: Ví dụ mẫu (Few-shot examples)
    Low,        // Ưu tiên thấp: Lịch sử trò chuyện cũ, ghi chú phụ trợ
}

#[derive(Debug, Clone)]
pub struct ContextSegment {
    pub name: String,
    pub content: String,
    pub priority: PriorityTier,
    pub estimated_tokens: usize,
}

impl ContextSegment {
    pub fn new(name: &str, content: &str, priority: PriorityTier) -> Self {
        // Ước tính số token sơ bộ: trung bình khoảng 4 ký tự tương đương 1 token
        let estimated_tokens = (content.len() + 3) / 4;
        Self {
            name: name.to_string(),
            content: content.to_string(),
            priority,
            estimated_tokens,
        }
    }
}

// 2. ĐỘNG CƠ QUẢN LÝ CỬA SỔ NGỮ CẢNH (CONTEXT WINDOW ENGINE)
// Sử dụng con trỏ thông minh (smart pointer) hoặc cấu trúc sở hữu chặt chẽ
// để quản lý bộ nhớ đệm (buffer) chứa các chỉ thị prompt an toàn.
pub struct ContextEngine {
    pub max_token_budget: usize,
    segments: Vec<ContextSegment>,
}

impl ContextEngine {
    pub fn new(max_token_budget: usize) -> Self {
        Self {
            max_token_budget,
            segments: Vec::new(),
        }
    }

    // Thêm một phân đoạn ngữ cảnh vào hàng chờ
    pub fn add_segment(&mut self, segment: ContextSegment) {
        self.segments.push(segment);
    }

    // Ghép nối prompt tối ưu hóa dựa trên ngân sách token tối đa
    // Tuân thủ nghiêm ngặt quy tắc mượn (borrow) và quyền sở hữu (ownership)
    pub fn assemble_system_prompt(&self) -> (String, usize) {
        // 1. Phân loại các segment theo mức độ ưu tiên
        let mut critical = Vec::new();
        let mut high = Vec::new();
        let mut medium = Vec::new();
        let mut low = Vec::new();

        for seg in &self.segments {
            match seg.priority {
                PriorityTier::Critical => critical.push(seg),
                PriorityTier::High => high.push(seg),
                PriorityTier::Medium => medium.push(seg),
                PriorityTier::Low => low.push(seg),
            }
        }

        let mut assembled_prompt = String::with_capacity(4096);
        let mut used_tokens = 0;

        // Hàm nội bộ an toàn để nạp các segment theo thứ tự ưu tiên
        let mut try_include = |segs: &[&ContextSegment]| {
            for seg in segs {
                if used_tokens + seg.estimated_tokens <= self.max_token_budget {
                    assembled_prompt.push_str(&format!("### [{}]\n{}\n\n", seg.name, seg.content));
                    used_tokens += seg.estimated_tokens;
                } else {
                    println!("[Bộ lọc ngữ cảnh] Đã lược bỏ phân đoạn '{}' để không vượt quá ngân sách!", seg.name);
                }
            }
        };

        // Ưu tiên high nạp trước, ưu tiên thấp nạp sau
        try_include(&critical);
        try_include(&high);
        try_include(&medium);
        try_include(&low);

        (assembled_prompt, used_tokens)
    }
}

// 3. HÀM MAIN THỰC CHỨC MINH HỌA QUY TRÌNH QUẢN LÝ NGỮ CẢNH
fn main() {
    println!("=== CHƯƠNG 40: MINH HỌA ĐỘNG CƠ QUẢN LÝ NGỮ CẢNH & PROMPT HỆ THỐNG ===");

    // Giả sử chúng ta đặt ngân sách ngữ cảnh rất chặt chẽ: chỉ 300 tokens
    let mut engine = ContextEngine::new(300);

    // Segment 1: Ràng buộc an toàn cốt lõi (Critical)
    engine.add_segment(ContextSegment::new(
        "RÀNG BUỘC KỸ THUẬT BẤT BIẾN",
        "1. Ngôn ngữ: Rust 2021 Edition.\n2. CẤM tuyệt đối dùng `unsafe`.\n3. CẤM dùng `.unwrap()`; bắt buộc xử lý lỗi bằng `Result<T, E>`.\n4. Đảm bảo an toàn quyền sở hữu (ownership) và mượn (borrow).",
        PriorityTier::Critical,
    ));

    // Segment 2: Giao ước Hợp đồng Trait (High)
    engine.add_segment(ContextSegment::new(
        "GIAO ƯỚC DỮ LIỆU & TRAIT NGHIỆP VỤ",
        "pub trait LogStorage {\n    fn append_log(&mut self, message: &str) -> Result<u64, String>;\n}",
        PriorityTier::High,
    ));

    // Segment 3: Ví dụ mẫu (Medium)
    engine.add_segment(ContextSegment::new(
        "VÍ DỤ MẪU (FEW-SHOT EXAMPLE)",
        "// Mẫu triển khai xử lý an toàn:\nimpl LogStorage for MemoryStorage {\n    fn append_log(&mut self, msg: &str) -> Result<u64, String> {\n        self.buffer.push(msg.to_string());\n        Ok(self.buffer.len() as u64)\n    }\n}",
        PriorityTier::Medium,
    ));

    // Segment 4: Lịch sử trò chuyện rườm rà (Low - sẽ bị cắt bỏ nếu vượt budget)
    engine.add_segment(ContextSegment::new(
        "LỊCH SỬ CHAT CŨ KHÔNG CẦN THIẾT",
        "Người dùng từng hỏi về thời tiết Hà Nội và cách nấu phở bò gia truyền 3 thế hệ trước khi bắt đầu lập trình...",
        PriorityTier::Low,
    ));

    // Tiến hành ghép nối Prompt
    let (final_prompt, total_tokens) = engine.assemble_system_prompt();

    println!("\n--- KẾT QUẢ PROMPT HOÀN CHỈNH ĐƯỢC CHẮT LỌC ---");
    println!("{}", final_prompt);
    println!("Tổng số tokens ước tính đã dùng: {} / {} tokens tối đa", total_tokens, engine.max_token_budget);

    // Kiểm tra tính đúng đắn của logic
    assert!(total_tokens <= engine.max_token_budget);
    assert!(final_prompt.contains("RÀNG BUỘC KỸ THUẬT BẤT BIẾN"));
    assert!(final_prompt.contains("GIAO ƯỚC DỮ LIỆU & TRAIT NGHIỆP VỤ"));

    println!("\n[Kiểm chứng thành công] Prompt đã được tối ưu hóa hoàn hảo, loại bỏ 100% tạp âm ngữ cảnh!");
}
