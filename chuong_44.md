# Chương 44: Kỹ Nghệ Prompt Kỹ Thuật Hệ Thống & Quản Lý Cửa Sổ Ngữ Cảnh (Systems Prompt Engineering & Context Management)

## Giới thiệu & Mục tiêu học tập

Trong chương trước, chúng ta đã thấu hiểu sự chuyển dịch mang tính thời đại từ "người thợ gõ cú pháp" thành "Tổng đạo diễn kiến trúc" trong làn sóng Vibe Coding. Nhưng làm thế nào để một Tổng đạo diễn có thể truyền đạt chính xác 100% ý đồ của mình cho đoàn làm phim AI mà không bị hiểu lầm, không bị sai lệch, và không tạo ra những đoạn mã rác?

Câu trả lời nằm ở hai kỹ năng mang tính sống còn của kỹ sư hệ thống hiện đại: **Kỹ nghệ Prompt hệ thống (Systems Prompt Engineering)** và **Kiểm soát Cửa sổ ngữ cảnh (Context Window Management)**.

Rất nhiều người mới bắt đầu thường nhầm lẫn rằng: *"Muốn AI viết code giỏi thì cứ quăng toàn bộ mã nguồn của cả dự án vào khung chat"*. Đây là một sai lầm chết người! Các mô hình ngôn ngữ lớn (LLM) không phải là những bộ não vô tận; chúng hoạt động dựa trên cơ chế phân bổ sự chú ý (Attention Mechanism) với dung lượng bộ nhớ làm việc hữu hạn. Khi bạn nhồi nhét quá nhiều thông tin rác, AI sẽ rơi vào trạng thái "suy giảm chú ý" (Attention Degradation), bắt đầu sinh ra ảo giác (hallucination), quên các quy ước đã thống nhất từ trước, và vi phạm nghiêm trọng các nguyên tắc về quyền sở hữu (ownership), mượn (borrow), hoặc thời gian sống (lifetime) của Rust.

Mục tiêu học tập của chương:
- Thấu hiểu cơ chế hoạt động của Cửa sổ ngữ cảnh (Context Window) và hiện tượng suy giảm chú ý trong LLM.
- Nắm vững công thức 5 thành phần để thiết kế một System Prompt kỹ thuật chuẩn công nghiệp, loại bỏ 99% ảo giác của AI.
- Thiết lập tệp quy chuẩn dự án tự động (`.cursorrules` hoặc `AGENTS.md`) để kiểm soát hành vi sinh mã của AI trong các IDE hiện đại.
- Xây dựng tư duy chắt lọc ngữ cảnh: Chỉ cung cấp đúng dữ liệu, đúng kiểu giao ước và đúng thời điểm.

---

## Hình tượng hóa đời sống (Intuitive Everyday Analogy)

### Bàn làm việc của Bác thợ mộc tài hoa và Căn phòng bừa bộn

Hãy tưởng tượng bạn thuê một Bác thợ mộc cực kỳ khéo tay và làm việc siêu nhanh (đại diện cho trợ lý AI) đến đóng cho bạn một chiếc bàn học gỗ sồi. Bác thợ mộc có một chiếc **bàn gia công** trước mặt với diện tích mặt bàn đúng 1 mét vuông (tượng trưng cho **Cửa sổ ngữ cảnh - Context Window**).

#### Kịch bản 1: Người chủ bừa bãi (Bad Context & Bad Prompt)
Người chủ bước vào và nói bâng quơ:
> *"Bác ơi, đóng cho cháu cái bàn đẹp đẹp, chắc chắn nhé, cháu để máy tính với vài cuốn sách!"*

Sau đó, người chủ bê cả một đống đồ cũ từ nhà kho ném bừa bãi lên mặt bàn gia công 1 mét vuông của bác thợ mộc: từ đống quần áo rách, vỏ chai nước ngọt, đĩa CD ca nhạc cũ, đến mấy cái đinh rỉ sét.

Hậu quả là gì?
1. Mặt bàn bị quá tải (Context Overflow): Các tài liệu quan trọng bị đống rác đè lên và rơi xuống đất (mất dấu vết thông tin).
2. Bác thợ mộc bị phân tâm (Attention Degradation): Bác không biết phải dùng cái gì, bắt đầu nhầm lẫn giữa gỗ sồi và thanh củi mục, và đóng ra một chiếc bàn ọp ẹp 3 chân vì lời dặn ban đầu quá mập mờ.

#### Kịch bản 2: Vị Kỹ sư trưởng chuyên nghiệp (Systems Prompt & Clean Context)
Vị Kỹ sư trưởng bước vào phòng làm việc, lau sạch mặt bàn gia công 1 mét vuông, và chỉ đặt lên bàn đúng 3 thứ:
1. **Tấm thẻ quy chuẩn an toàn (System Persona & Constraints)**: *"Bác là thợ mộc bậc 7. Tiêu chuẩn xưởng: Bắt buộc dùng mộng gỗ truyền thống, tuyệt đối không dùng đinh sắt rẻ tiền, cạnh bàn phải vát tròn 5mm để không gây trầy xước"*.
2. **Bản vẽ kỹ thuật chi tiết (Domain Contract & Types)**: Một bản vẽ kích thước chuẩn: Dài 120cm, Rộng 60cm, Cao 75cm, chịu tải trọng tối thiểu 50kg.
3. **Mẫu gỗ chuẩn (Few-shot Example)**: Một thanh gỗ mẫu đã chà nhám mịn để làm mốc so sánh chất lượng.

Bác thợ mộc nhìn vào mặt bàn sạch sẽ, hiểu ngay 100% yêu cầu mà không tốn một giây suy nghĩ vẩn vơ. Chiếc bàn gỗ sồi hoàn mỹ được hoàn thành chỉ sau 30 phút, đạt chuẩn xác từng milimet!

---

## Khái niệm & Cơ chế kỹ thuật chuyên sâu

### 1. Bản chất cơ học của Token và Cửa sổ ngữ cảnh (Context Window)
Trong khoa học máy tính, các mô hình ngôn ngữ lớn (LLM) không đọc văn bản theo từng chữ cái hay từng từ nguyên vẹn như con người, mà chia nhỏ văn bản thành các đơn vị gọi là **Tokens** (thường từ 3-4 ký tự tiếng Anh hoặc 1-2 ký tự tiếng Việt có dấu).

Mỗi LLM đều có một giới hạn vật lý nghiêm ngặt gọi là **Cửa sổ ngữ cảnh (Context Window)** — ví dụ: 8,000 tokens, 32,000 tokens hoặc 128,000 tokens. Cửa sổ này tương đương với bộ nhớ truy xuất nhanh (RAM) tạm thời của mô hình trong một phiên làm việc:
- Khi tổng số tokens của Prompt + Lịch sử hội thoại + Mã nguồn tải lên vượt quá giới hạn, những thông tin ở phần đầu sẽ bị đẩy ra ngoài (bị lãng quên vĩnh viễn).
- Ngay cả khi chưa vượt quá giới hạn, hiện tượng **"Lost in the Middle" (Bị lãng quên ở giữa)** vẫn diễn ra: LLM ghi nhớ rất tốt thông tin ở phần đầu (System Prompt) và phần cuối (câu lệnh vừa gõ), nhưng dễ bỏ qua những chỉ thị nằm ở lưng chừng hàng ngàn dòng code.

### 2. Cấu trúc 5 thành phần của một Systems Prompt chuẩn mực
Để biến AI thành một lập trình viên Rust cấp cao tuân thủ tuyệt đối quy chuẩn dự án, System Prompt cần được cấu trúc theo 5 khối rõ ràng:

```markdown
┌─────────────────────────────────────────────────────────────┐
│ 1. PERSONA & ROLE (Định danh vai trò chuyên gia)            │
├─────────────────────────────────────────────────────────────┤
│ 2. HARD CONSTRAINTS (Các điều cấm kỵ bất khả xâm phạm)      │
├─────────────────────────────────────────────────────────────┤
│ 3. DOMAIN CONTRACTS (Kiểu dữ liệu, Structs, Enums & Traits) │
├─────────────────────────────────────────────────────────────┤
│ 4. INPUT / OUTPUT SPEC (Đặc tả dữ liệu đầu vào & đầu ra)    │
├─────────────────────────────────────────────────────────────┤
│ 5. FEW-SHOT EXAMPLES (Ví dụ mẫu chuẩn để AI noi theo)       │
└─────────────────────────────────────────────────────────────┘
```

#### Chi tiết 5 thành phần:
1. **Persona & Role**: Xác định tầm nhận thức: *"Bạn là một Kỹ sư Hệ thống Rust cao cấp (Senior Rust Systems Engineer) tuân thủ tiêu chuẩn Rust 2021 Edition"*.
2. **Hard Constraints**: Các lằn ranh đỏ kỹ thuật:
   - Nghiêm cấm sử dụng từ khóa `unsafe` trừ khi có sự phê duyệt tường minh.
   - Nghiêm cấm dùng `.unwrap()` hoặc `.expect()` trong mã nguồn thương mại; bắt buộc lan truyền lỗi bằng `Result<T, E>` và toán tử `?`.
   - Bắt buộc tuân thủ nguyên tắc quyền sở hữu (ownership) và mượn (borrow), không lạm dụng `.clone()` bừa bãi.
   - Luôn sử dụng bộ nhớ đệm (buffer) thích hợp khi thao tác đọc/ghi file hoặc luồng mạng.
3. **Domain Contracts**: Cung cấp các định nghĩa kiểu dữ liệu thuần túy (chỉ gửi chữ ký hàm và cấu trúc struct/trait, không gửi thân hàm cũ làm loãng ngữ cảnh).
4. **Input/Output Spec**: Yêu cầu định dạng đầu ra rõ ràng: Chỉ trả về mã nguồn Rust hợp lệ, có chú thích tiếng Việt cho từng khối logic, không giải thích lan man.
5. **Few-Shot Examples**: Đưa ra một ví dụ mẫu ngắn gọn thể hiện phong cách viết mã bạn mong muốn.

### 3. Tự động hóa quy chuẩn dự án qua `.cursorrules` và `AGENTS.md`
Trong các môi trường làm việc hiện đại (như Cursor IDE, Windsurf, hoặc Antigravity), bạn không cần phải copy-paste System Prompt mỗi lần chat. Bạn có thể lưu trữ chúng vào tệp cấu hình chuyên dụng ở thư mục gốc của dự án:
- `.cursorrules` hoặc `.agent/rules/*.md`: Tự động được IDE đính kèm vào mỗi lượt suy luận của AI.
- Cơ chế phân tầng ngữ cảnh thông minh:
  - **Tầng 1 (Luôn tải)**: Các ràng buộc an toàn bộ nhớ và tiêu chuẩn mã nguồn.
  - **Tầng 2 (Tải theo ngữ cảnh)**: Các kiểu dữ liệu của mô-đun hiện tại đang chỉnh sửa.
  - **Tầng 3 (Chỉ tải khi cần)**: Lịch sử lỗi biên dịch để AI sửa chữa.

---

## Mã nguồn minh họa thực chiến

Dưới đây là một mô-đun Rust hoàn chỉnh, có thể biên dịch và chạy bằng `rustc --edition=2021`. Chương trình này mô phỏng một **Động cơ điều phối ngữ cảnh (Context Engine)** chuyên nghiệp: Tự động phân tích dung lượng token, quản lý ngân sách bộ nhớ ngữ cảnh (token budget), ghép nối các thành phần System Prompt có trọng số, và cắt gọt ngữ cảnh thừa thãi trước khi chuyển giao cho trợ lý AI.

```rust
// ============================================================================
// CHƯƠNG 40: HỆ THỐNG QUẢN LÝ CỬA SỔ NGỮ CẢNH & ĐÓNG GÓI SYSTEM PROMPT CHUẨN MỰC
// Tác giả: Kỹ Sư Kiến Trúc Hệ Thống Rust
// ============================================================================

use std::collections::VecDeque;

// 1. ĐỊNH NGHĨA CÁC PHÂN ĐOẠN NGỮ CẢNH (CONTEXT SEGMENT)
// Mỗi phần của ngữ cảnh có mức độ ưu tiên khác nhau khi ngân sách bộ nhớ bị giới hạn.
#[derive(Debug, Clone, PartialEq, Eq)]
pub enum PriorityTier {
    Critical,   // Bắt buộc phải có: Quy chuẩn an toàn, Traits deliver ước
    High,       // Ưu tiên cao: Kiểu dữ liệu trực tiếp, Chữ ký hàm
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

        // Ưu tiên cao nạp trước, ưu tiên thấp nạp sau
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
```

---

## Bảng tra cứu lỗi biên dịch & Cách khắc phục

Dưới đây là các lỗi biên dịch thường gặp nhất khi lập trình viên cung cấp ngữ cảnh bị thiếu sót hoặc prompt sai lệch khiến AI sinh mã lỗi:

| Mã lỗi `rustc` | Nguyên nhân gốc rễ do sai lệch ngữ cảnh | Đoạn mã vi phạm mẫu | Giải pháp điều chỉnh Prompt & Context |
| :--- | :--- | :--- | :--- |
| **`E0061`** | **Mismatched number of arguments**<br>AI nhớ phiên bản hàm cũ vì trong context bạn không cung cấp chữ ký hàm hiện tại. | ```rust // compile-fail\nfn process_event(id: u64, name: &str) {}\nprocess_event(10);``` | Luôn đưa chữ ký hàm chính xác vào khối `Domain Contracts` trong System Prompt để AI đối chiếu số lượng tham số. |
| **`E0425`** | **Cannot find value/function in this scope**<br>AI gọi một hàm tiện ích mà không biết nó nằm ở mô-đun nào vì context bị thiếu thông tin `use`. | ```rust // compile-fail\nlet data = read_file_to_string("config.json");``` | Bổ sung câu lệnh quy chuẩn vào Prompt: *"Mọi hàm bên ngoài bắt buộc phải ghi rõ đường dẫn đầy đủ hoặc khai báo `use std::...` tường minh"*. |
| **`E0599`** | **No method named found for type**<br>AI gọi một phương thức thuộc về Trait nhưng chưa import Trait đó vào phạm vi tệp tin. | ```rust // compile-fail\nuse std::io;\nlet mut f = io::stdout();\nf.write_all(b"hello");``` | Cung cấp danh sách các Trait cốt lõi trong prompt (ví dụ: `use std::io::Write;`) và nhắc nhở AI luôn đưa Trait vào phạm vi hoạt động. |
| **`E0382`** | **Use of moved value in loop**<br>AI chuyển quyền sở hữu (ownership) của một chuỗi String vào bên trong phân đoạn ngữ cảnh lặp lại. | ```rust // compile-fail\nlet s = String::from("context");\nfor _ in 0..2 { drop(s); }``` | Nhắc nhở AI áp dụng quy tắc mượn (borrow) tham chiếu `&str` hoặc `&ContextSegment` thay vì tiêu thụ quyền sở hữu trong các thao tác lặp. |

---

## Tóm tắt chương & Bài tập rèn luyện

### 4 Điểm cốt lõi cần ghi nhớ
1. **Chất lượng đầu ra của AI tỷ lệ thuận với độ sạch của ngữ cảnh**: Đưa càng nhiều thông tin rác vào prompt thì AI càng dễ sinh ảo giác và quên lãng các quy tắc quan trọng.
2. **Cấu trúc System Prompt 5 phần**: Định danh vai trò -> Lằn ranh đỏ (Hard Constraints) -> Hợp đồng dữ liệu (Contracts) -> Đặc tả I/O -> Ví dụ mẫu (Few-shot).
3. **Ưu tiên phân tầng thông tin**: Luôn ưu tiên các quy tắc an toàn bộ nhớ và giao ước Trait lên hàng đầu (`Critical`); lịch sử hội thoại rườm rà phải được dọn dẹp thường xuyên (`Low`).
4. **Tự động hóa với `.cursorrules`**: Biến các tiêu chuẩn dự án thành luật lệ bất di bất dịch được nạp tự động, giảm thiểu 80% công sức giao tiếp lặp lại.

### Bài tập rèn luyện tư duy

**Bài tập 1 (Phê bình và Nâng cấp Prompt)**:
Một lập trình viên gửi câu lệnh sau cho AI:
> *"Viết cho tôi một hàm đọc file cấu hình config.txt rồi trả về danh sách cổng mạng"*.

Dựa trên 5 thành phần của System Prompt đã học, hãy viết lại câu lệnh trên thành một System Prompt kỹ thuật hoàn chỉnh:
- Có quy định cấm dùng `unwrap()`.
- Có quy định xử lý khi file không tồn tại.
- Có định dạng trả về rõ ràng (`Result<Vec<u16>, std::io::Error>`).

**Bài tập 2 (Tối ưu hóa Cửa sổ ngữ cảnh)**:
Dự án của bạn có 50 tệp tin mã nguồn với tổng cộng 80,000 dòng code. Bạn đang cần AI viết thêm một phương thức mới cho `struct UserSession`.
Hãy nêu chiến lược: Bạn sẽ chọn những tệp tin hoặc thông tin nào để đưa vào cửa sổ ngữ cảnh của AI, và bạn sẽ cố tình bỏ lại những gì để tránh gây quá tải bộ nhớ làm việc của mô hình?

**Bài tập 3 (Sửa lỗi thiếu Trait Scope của AI)**:
Đoạn mã sau do AI sinh ra bị lỗi biên dịch `E0599` vì thiếu khai báo Trait trong phạm vi:
```rust
use std::fs::File;

fn save_data_to_file(path: &str, content: &[u8]) -> Result<(), std::io::Error> {
    let mut file = File::create(path)?;
    // Trình biên dịch báo lỗi: no method named `write_all` found for struct `File`
    file.write_all(content)?;
    Ok(())
}
```
Hãy giải thích vì sao lỗi xảy ra và thêm đúng dòng lệnh `use` còn thiếu để mã nguồn biên dịch thành công.
*(Gợi ý: Phương thức `write_all` nằm trong Trait `std::io::Write`)*.

---

### Gợi ý & Lời giải

<details>
<summary><b>Bài tập 1 — Gợi ý</b></summary>

System Prompt tốt phải nêu rõ: vai trò, ràng buộc cứng (cấm gì), cách xử lý ca lỗi, và định dạng đầu ra chính xác. Câu lệnh gốc thiếu tất cả những thứ đó.
</details>

<details>
<summary><b>Bài tập 1 — Lời giải</b></summary>

Câu lệnh gốc *"Viết cho tôi một hàm đọc file config.txt rồi trả về danh sách cổng mạng"* mơ hồ ở mọi khía cạnh quan trọng. Viết lại thành một System Prompt kỹ thuật hoàn chỉnh:

```text
VAI TRÒ: Bạn là một kỹ sư Rust cấp cao, viết mã sản xuất an toàn.

NHIỆM VỤ: Viết hàm đọc tệp cấu hình và trả về danh sách cổng mạng.

CHỮ KÝ HÀM BẮT BUỘC:
    fn doc_danh_sach_cong(path: &str) -> Result<Vec<u16>, std::io::Error>

RÀNG BUỘC CỨNG:
- TUYỆT ĐỐI KHÔNG dùng .unwrap() hay .expect(). Mọi lỗi phải lan qua toán tử `?`.
- Cổng mạng là u16 (0-65535). Bỏ qua dòng trống và dòng bắt đầu bằng '#' (chú thích).
- Dòng không phân tích được thành số hợp lệ: bỏ qua, không làm sập hàm.

XỬ LÝ KHI TỆP KHÔNG TỒN TẠI:
- Không tự tạo tệp. Trả về Err(io::Error) để người gọi quyết định.

ĐỊNH DẠNG ĐẦU RA:
- Chỉ trả về mã Rust trong một khối ```rust, kèm 3 test đơn vị:
  tệp hợp lệ, tệp không tồn tại, và tệp chứa dòng rác.
```

Đối chiếu với **5 thành phần của một System Prompt tốt**, câu gốc thiếu cả năm:

| Thành phần | Câu gốc | Bản nâng cấp |
|---|---|---|
| **Vai trò** (persona) | không | "kỹ sư Rust cấp cao, mã sản xuất" |
| **Nhiệm vụ** rõ ràng | mơ hồ | chữ ký hàm chính xác |
| **Ràng buộc** cứng | không | cấm unwrap, u16, bỏ qua chú thích |
| **Xử lý ca biên** | không | tệp thiếu -> trả Err |
| **Định dạng đầu ra** | không | kiểu trả về + test bắt buộc |

Bài học: chất lượng đầu ra của AI **tỉ lệ thuận với độ rõ của ràng buộc bạn đặt ra**. Câu mơ hồ "đọc file trả về cổng" để AI tự đoán kiểu trả về (Vec<String>? Vec<i32>?), tự đoán cách xử lý lỗi (thường là rải `.unwrap()` — đúng thứ ta muốn cấm). Một System Prompt chặt biến AI từ "đoán ý" thành "thực thi đặc tả".
</details>

<details>
<summary><b>Bài tập 2 — Gợi ý</b></summary>

Cửa sổ ngữ cảnh là bộ nhớ làm việc hữu hạn của mô hình. Chiến lược đúng: chỉ đưa vào những gì *liên quan trực tiếp* tới việc sửa, cố tình bỏ 49 tệp còn lại.
</details>

<details>
<summary><b>Bài tập 2 — Lời giải</b></summary>

**Chiến lược chọn ngữ cảnh cho việc "thêm phương thức mới cho `struct UserSession`":**

**ĐƯA VÀO cửa sổ ngữ cảnh (liên quan trực tiếp):**
1. **Tệp định nghĩa `struct UserSession`** — bắt buộc. AI cần thấy các trường và khối `impl` hiện có để phương thức mới khớp kiểu và quy ước.
2. **Định nghĩa các kiểu mà `UserSession` phụ thuộc** — nếu nó chứa trường kiểu `User`, `Token`, `Permission`, thì đưa *định nghĩa* của những kiểu đó (không phải toàn bộ tệp chứa chúng).
3. **1-2 phương thức mẫu đã có trong `impl UserSession`** — để AI bắt chước phong cách xử lý lỗi, đặt tên, quy ước của dự án.
4. **Đặc tả ngắn của phương thức mới** — nó làm gì, chữ ký, ca lỗi.

**CỐ TÌNH BỎ LẠI (nhiễu, gây quá tải):**
- **47 tệp còn lại không liên quan** — tầng giao diện, cấu hình build, các module nghiệp vụ khác. Chúng không ảnh hưởng tới một phương thức của `UserSession`.
- **Thân các hàm dài không liên quan** — kể cả trong tệp đúng, những hàm khác chỉ cần *chữ ký* là đủ, không cần cả thân.
- **Tệp test, tài liệu, lịch sử git** — trừ khi việc này đụng tới chúng.

**Vì sao cố tình bỏ bớt lại tốt hơn "đưa hết cho chắc":**
- Cửa sổ ngữ cảnh **hữu hạn**: nhồi 80.000 dòng thì hoặc tràn, hoặc đẩy mất phần *thực sự quan trọng* ra ngoài.
- **Nhiễu làm loãng tín hiệu.** Mô hình chú ý kém đi khi 95% ngữ cảnh không liên quan — hiện tượng "lạc giữa đống thông tin" (lost in the middle): chi tiết ở giữa một ngữ cảnh dài dễ bị bỏ sót.
- **Tốn kém và chậm** tỉ lệ với lượng ngữ cảnh, mà chẳng đổi lại chất lượng.

Nguyên tắc: **ngữ cảnh là dao mổ, không phải xẻng xúc.** Đưa đúng cái liên quan giúp mô hình tập trung; đưa cả kho buộc nó tự lọc — và nó lọc kém hơn bạn. Kỹ năng "quản lý ngữ cảnh" chính là biết *bỏ đi cái gì*.
</details>

<details>
<summary><b>Bài tập 3 — Gợi ý</b></summary>

Lỗi E0599 (`no method named write_all`) không phải vì thiếu phương thức, mà vì **trait chứa phương thức đó chưa được kéo vào phạm vi**. Phải `use std::io::Write`.
</details>

<details>
<summary><b>Bài tập 3 — Lời giải</b></summary>

**Nguyên nhân E0599:** `write_all` *có tồn tại* cho `File`, nhưng nó được cung cấp qua **trait `std::io::Write`**, không phải phương thức vốn có của `File`. Trong Rust, **phương thức của một trait chỉ dùng được khi trait đó đang trong phạm vi (`use`)**. Thiếu `use std::io::Write;` thì trình biên dịch không "nhìn thấy" `write_all`, dù `File` có triển khai nó.

**Cách sửa — thêm đúng một dòng `use`:**
```rust
use std::fs::File;
use std::io::Write;   // <-- DÒNG CÒN THIẾU: kéo trait Write vào phạm vi

fn save_data_to_file(path: &str, content: &[u8]) -> Result<(), std::io::Error> {
    let mut file = File::create(path)?;
    file.write_all(content)?;   // giờ thấy write_all vì Write đã trong phạm vi
    Ok(())
}
```

**Vì sao Rust thiết kế "phải import trait mới gọi được phương thức":** đây là cơ chế **coherence** giữ cho không gian tên phương thức sạch sẽ. Nhiều trait khác nhau có thể cùng đặt một phương thức tên `write` cho cùng một kiểu; nếu mọi phương thức trait đều tự động hiện diện, sẽ có xung đột tên khắp nơi. Buộc `use` trait làm cho *nguồn gốc* mỗi phương thức trở nên tường minh — đọc phần `use` đầu tệp là biết những khả năng nào đang được kích hoạt.

Đây là lỗi AI mắc rất thường xuyên: nó sinh mã gọi `write_all`, `read_to_string`, `flush`... nhưng quên dòng `use` cho trait tương ứng (`Write`, `Read`). Mẹo nhận diện: khi thấy `no method named X found for struct Y` mà bạn *biết* phương thức đó tồn tại, gần như chắc chắn là **thiếu `use` trait** — trình biên dịch Rust thường gợi ý ngay dòng `use` cần thêm ở cuối thông báo lỗi.
</details>
