# Chương 88: Suy luận Có Cấu trúc, Kỹ nghệ Đồ thị Kép, Máy Trạng thái HITL & Truy xuất Lai BM25 + RRF (Structured Reasoning, Dual-Graph Engineering, HITL StateGraph & Hybrid RAG)

> **Mục tiêu chương học**:
> 1. **Làm chủ kỹ nghệ suy luận có cấu trúc (Structured Reasoning)**: Giải mã cơ chế vượt qua cạm bẫy tham lam của Chain-of-Thought thông qua cây ý nghĩ Tree-of-Thoughts (ToT) với bài toán kinh điển Game of 24, cơ chế biểu quyết tự nhất quán đa số (Self-Consistency Voting), phân rã chiến lược Plan-and-Solve, vòng lặp tự phản tỉnh tăng cường bằng ngôn ngữ (Reflexion Loop), và khung kỷ luật 5 thao tác nhận thức bất biến CognitiveTool5Op (`Understand -> Extract -> Highlight -> Apply -> Validate`).
> 2. **Làm chủ kỹ nghệ đồ thị kép vĩ mô (Dual-Graph Macro-Engineering)**: Xây dựng đồ thị tri thức ngữ nghĩa InfoGraph với giải thuật cắt lát lân cận 1 bước (`slice_1hop_context`) chứng minh toán học giảm thiểu > 60% lượng token rác nạp vào prompt; kết hợp với đồ thị kế hoạch thực thi hướng luồng PlanDag vận hành giải thuật Kahn bóc tách các đợt sóng sẵn sàng song song (`topological_ready_waves`), phát hiện chu trình bế tắc (`DagError::CycleDetected`), cô lập lỗi qua ngân sách thử lại cục bộ (`retry_budget`), và tối ưu hoá tài nguyên bằng bộ biên dịch trước AoT Subgraph Compiler.
> 3. **Thiết kế Máy trạng thái nút và Điểm kiểm soát tất định (StateGraph & Stripe Blueprints)**: Cài đặt máy trạng thái generic `StateGraph<S>` hỗ trợ ba hàm gộp kênh trạng thái (`Overwrite`, `Append`, `Sum`), thiết lập cơ chế hộp đen kiểm toán tất định Stripe Blueprints với ảnh chụp bất biến (Snapshot), mã băm toàn vẹn (Checksum), nhật ký nội bộ từng nút (Private Node Log) và khả năng hoàn tác (Rollback) an toàn khi gặp sự cố.
> 4. **Xây dựng Cổng ngắt duyệt can thiệp người trong vòng lặp (HITL Interrupt & Procedural Memory)**: Triển khai 4 chế độ can thiệp con người (`Accept`, `Edit`, `Respond`, `Ignore`). Đặc biệt, mổ xẻ và sửa chữa triệt để **Lỗi biên Edge Case #5**: phát hiện và xử lý sự khác biệt tập hợp (Set Difference Diff) đối với các tham số bị con người xoá bỏ khỏi yêu cầu ban đầu (`orig.keys() \ new_args.keys()`), ghi nhận vĩnh viễn quy tắc cấm `arg->[DELETED]` vào Trí nhớ thủ tục `ProceduralMemory`.
> 5. **Hiện thực hoá Trí nhớ nhận thức 4 tầng & Đồ thị lưỡng thời gian (MEM1 & BiTemporalGraph)**: Xây dựng kiến trúc bộ nhớ nhận thức MEM1 gồm 4 tầng phân cấp (Working Memory, Episodic Memory, Semantic Memory, Procedural Memory) với thuật toán củng cố tri thức (Consolidation) dựa trên độ tương đồng Jaccard $\ge 0.7$; kiến tạo đồ thị lưỡng thời gian `BiTemporalGraph` hai trục độc lập ($ValidTime \times TxTime$) phục vụ truy vấn du hành thời gian (Time Travel Queries); tích hợp động cơ BDI (Belief-Desire-Intention) phân biệt rạch ròi giữa thực thể trường tồn (Endurants) và tiến trình thời gian (Perdurants).
> 6. **Cài đặt Hệ thống Truy xuất lai & Kiểm chứng trích dẫn (Hybrid RAG & Span Citation Verifier)**: Chế tạo bộ phân đoạn văn bản `MarkdownHeadingChunker` bảo toàn đường dẫn phân cấp (`"H1 > H2 > H3"`), đồng thời giải quyết triệt để **Lỗi biên Edge Case #1** (chống tràn số/chia cho 0 khi `max_w = 0`) và **Lỗi biên Edge Case #2** (bỏ qua dòng chú thích mã nguồn `# comment` trong rào mã code fence); xây dựng bộ truy xuất lai `HybridRetriever` kết hợp Okapi BM25 và Dense Cosine qua giải thuật dung hợp thứ hạng tương hỗ RRF (Reciprocal Rank Fusion), bảo đảm **Lỗi biên Edge Case #3** kiểm tra khớp kích thước nghiêm ngặt (`chunks.len() == embs.len()`); cuối cùng là bộ kiểm chứng trích dẫn toạ độ lát cắt `SpanCitationVerifier` an toàn biên ký tự UTF-8, triệt tiêu 100% ảo giác trích dẫn của mô hình ngôn ngữ.

---

## 1. Giới thiệu & Mục tiêu

Trong kỹ nghệ phần mềm truyền thống, lập trình viên nắm giữ quyền kiểm soát tuyệt đối luồng thực thi: một câu lệnh `if/else` rẽ nhánh theo biến boolean xác định, một vòng lặp `for` duyệt qua tập phần tử hữu hạn, và một lời gọi hàm trên ngăn xếp (stack frame) tuân thủ quy tắc quản lý vùng nhớ nghiêm ngặt của hệ điều hành.

Tuy nhiên, cuộc cách mạng trí tuệ nhân tạo tạo sinh đã đưa các mô hình ngôn ngữ lớn (Large Language Models - LLM) vào trung tâm của các hệ thống tự hành. Ở tầng sâu nhất, một mô hình ngôn ngữ bản chất chỉ là **một cỗ máy dự đoán phân phối xác suất của từ tiếp theo (next-token probability distribution)**:

$$P(w_t \mid w_1, w_2, \dots, w_{t-1}) = \text{Softmax}\left(\frac{Q K^T}{\sqrt{d_k}}\right) V$$

Khi chúng ta trao quyền cho một cỗ máy xác suất tự do quyết định các hành động có tác dụng phụ trong thế giới thực — như chỉnh sửa mã nguồn sản xuất, thực thi câu lệnh cơ sở dữ liệu, xoá bỏ tài nguyên hạ tầng đám mây hay điều phối giao dịch tài chính — việc chỉ dựa vào cơ chế "chuỗi suy nghĩ ngây thơ" (Naive Chain-of-Thought - CoT) sẽ nhanh chóng dẫn tới những thảm hoạ kỹ thuật khôn lường:

1. **Cạm bẫy suy luận tham lam (Greedy Search Trap)**: Mô hình ngôn ngữ tự nhiên luôn có xu hướng chọn token có xác suất cục bộ cao nhất ở bước hiện tại. Khi đối mặt với các bài toán tối ưu hoá tổ hợp hoặc suy luận nhiều bước, bước đi trông có vẻ "ngon ăn" nhất ở hiện tại thường dẫn thẳng vào ngõ cụt. Không có cơ chế quay lui (backtracking) và đánh giá cây tìm kiếm, mô hình sẽ bị mắc kẹt và bắt đầu ảo giác.
2. **Ảo giác quan hệ phụ thuộc (Dependency Hallucination)**: Trong một dự án phần mềm có 50 tệp mã nguồn và hàng trăm hàm liên kết chéo, việc yêu cầu LLM sinh ra một kế hoạch bằng văn bản thuần tuý sẽ dẫn đến việc thực thi tác vụ con trước khi tác vụ cha hoàn tất, hoặc tạo ra các chu trình phụ thuộc vòng tròn (circular dependency deadlocks) khiến hệ thống tê liệt.
3. **Trôi dạt trạng thái và hiệu ứng phụ không hoàn tác (State Drift & Irreversible Side Effects)**: Không có sự phân định ranh giới giữa dữ liệu nháp của từng công cụ và trạng thái cốt lõi của tác tử, một lỗi logic nhỏ ở bước thứ 4 có thể làm ô nhiễm toàn bộ bộ nhớ ngữ cảnh của bước thứ 5, khiến toàn bộ phiên làm việc thất bại mà không có cách nào hoàn tác (rollback) về trạng thái an toàn trước đó.
4. **Ô nhiễm ngữ cảnh và sự mù lòa thông tin (Context Contamination & Token Waste)**: Việc nhồi nhét toàn bộ đồ thị tri thức, nhật ký hệ thống và tài liệu tham khảo vào cửa sổ ngữ cảnh không chỉ làm bùng nổ chi phí tính toán (kinh tế học KV-Cache) mà còn kích hoạt hiện tượng "Lost in the Middle" — mô hình hoàn toàn lãng quên thông tin quan trọng nằm ở giữa ngữ cảnh.
5. **Ảo giác trích dẫn (Citation Fabrication)**: Mô hình tự tin khẳng định một thông tin dựa trên tài liệu tham khảo, nhưng số trang, toạ độ dòng và nội dung trích dẫn hoàn toàn là sản phẩm của sự bịa đặt ngẫu nhiên.

| Chiều so sánh | Tác tử CoT Ngây thơ (Naive CoT Agent) | Tác tử Suy luận Có Cấu trúc & Đồ thị Kép (Rust 2024 Harness) |
|---|---|---|
| **Không gian suy luận** | Chuỗi tuyến tính 1 chiều, không thể quay lui khi sai | Cây trạng thái (Tree of Thoughts) + Hàng đợi BFS / Beam Search có đánh giá cắt tỉa |
| **Độ tin cậy của quyết định** | Phụ thuộc vào 1 lần sinh mẫu ngẫu nhiên ($T > 0$) | Biểu quyết tự nhất quán (Self-Consistency) có trọng số độ tin cậy và luật phá hòa |
| **Quản lý kế hoạch** | Văn bản tự do không cấu trúc trong prompt | Đồ thị có hướng phi chu trình (PlanDag) phân tích đợt sóng song song theo giải thuật Kahn |
| **Khả năng phục hồi sự cố** | Sập toàn bộ quy trình nếu 1 bước API lỗi | Cô lập lỗi cục bộ theo ngân sách thử lại (`retry_budget`), chỉ thử lại nút bị lỗi |
| **Kiểm soát trạng thái** | Bộ nhớ biến thiên không lưu vết, khó tái hiện | Máy trạng thái Stripe Blueprints với Checkpoint bất biến, Checksum và Rollback |
| **Can thiệp con người (HITL)**| Bỏ qua hoặc chỉ ghi nhận tham số thêm mới | Xử lý đầy đủ 4 chế độ ngắt + Dò diff hai chiều ghi nhận cả các khoá bị con người xoá bỏ |
| **Lưu trữ tri thức** | Ngăn xếp ngữ cảnh phẳng, nhanh chóng bị tràn | Trí nhớ nhận thức 4 tầng MEM1 + Đồ thị lưỡng thời gian ($Valid \times Tx$) du hành thời gian |
| **Truy xuất thông tin (RAG)** | Tìm kiếm vector thuần tuý (dễ nhiễu ngữ nghĩa) | Truy xuất lai Okapi BM25 + Dense Cosine qua RRF + Kiểm chứng toạ độ lát cắt Span |

Để chuyển đổi từ một đồ chơi thử nghiệm thành một **Bộ khung tác tử tự hành cấp sản xuất (Production-Grade Autonomous Harness)**, chúng ta bắt buộc phải xây dựng một bộ khung bao bọc bằng Rust 2024 thuần `std`. Trong chương này, chúng ta sẽ lần lượt chinh phục 6 trụ cột cốt lõi:

```
┌─────────────────────────────────────────────────────────────────────────────┐
│                 6 TRỤ CỘT KỸ NGHỆ TÁC TỬ TRONG CHƯƠNG 88                    │
├─────────────────────────────────────────────────────────────────────────────┤
│ 1. SUY LUẬN CÓ CẤU TRÚC: Tree-of-Thoughts (Game of 24), Self-Consistency,  │
│    Plan-and-Solve Runner, Reflexion Verbal Loop, CognitiveTool5Op.          │
│ 2. ĐỒ THỊ KÉP VĨ MÔ: InfoGraph (cắt lát 1-hop giảm >60% token) + PlanDag   │
│    (sóng song song Kahn, chống chu trình, retry budget, AoT Compiler).     │
│ 3. MÁY TRẠNG THÁI NÚT: StateGraph<S> (Reducers: Overwrite/Append/Sum) +    │
│    Điểm kiểm soát tất định Stripe Blueprints (Checksum, Rollback).          │
│ 4. NGẮT CAN THIỆP HITL: Accept/Edit/Respond/Ignore + SỬA LỖI EDGE CASE #5  │
│    (truy vết khoá bị xoá vào ProceduralMemory::tool_override_rules).        │
│ 5. BỘ NHỚ NHẬN THỨC & LƯỠNG THỜI GIAN: CognitiveMemory4Tier (MEM1),        │
│    BiTemporalGraph (ValidTime x TxTime), BdiEngine (Endurant vs Perdurant). │
│ 6. TRUY XUẤT LAI HYBRID RAG: MarkdownHeadingChunker (SỬA LỖI #1 & #2) +     │
│    HybridRetriever BM25 + Dense + RRF (SỬA LỖI #3) + SpanCitationVerifier.  │
└─────────────────────────────────────────────────────────────────────────────┘
```

Mọi dòng mã được viết trong chương này đều tuân thủ nguyên tắc phòng sạch: không phụ thuộc thư viện ngoài (`[dependencies]` rỗng 100%), định danh tiếng Anh chuẩn mực (`-F non-ascii-idents`), tài liệu chú thích tiếng Việt chuyên sâu theo chuẩn RiskGate, và toàn bộ cấu trúc dữ liệu đều sử dụng các trường có tên tường minh (named fields).

## 2. Hình tượng hóa đời sống: Tháp điều khiển không lưu và bản đồ bay

Để hình dung một cách sống động và trực quan về cách các thành phần trong chương này tương tác với nhau, hãy tưởng tượng bạn là **Tổng chỉ huy tại Đài kiểm soát không lưu của Sân bay Quốc tế Tân Sơn Nhất vào một buổi chiều giông bão**.

Mỗi chuyến bay chuẩn bị cất cánh, hạ cánh hoặc chuyển hướng giống như một **nhiệm vụ tác tử**. Nếu để mỗi phi công tự nhìn qua kính buồng lái rồi tự quyết định đường bay theo chuỗi suy nghĩ ngẫu hứng của mình (CoT ngây thơ), thảm hoạ va chạm trên không chắc chắn sẽ xảy ra chỉ trong vòng vài phút. Thay vào đó, toàn bộ vùng trời và mặt đất được quản lý bằng một hệ thống điều phối đa tầng vô cùng tinh vi:

```
┌──────────────────────────────────────────────────────────────────────────────┐
│  HÌNH TƯỢNG HÓA: HỆ THỐNG ĐIỀU PHỐI KHÔNG LƯU VÀ BẢN ĐỒ BAY                  │
│                                                                              │
│    [RADAR 1-HOP INFOGRAPH]                 [LỊCH TRÌNH BAY PLANDAG]          │
│    Chỉ hiển thị bán kính 5 dặm quanh       Lập lịch đợt sóng hạ cánh         │
│    đường băng thay vì toàn bộ địa cầu      song song không xung đột          │
│    ==> Giảm > 60% quá tải thị giác!        ==> Phân rã Kahn Ready Waves!     │
│             │                                           │                    │
│             └─────────────────────┬─────────────────────┘                    │
│                                   ▼                                          │
│                      [THÁP CHỈ HUY STATEGRAPH]                               │
│                      Ghi nhật ký hộp đen tất định (Stripe Blueprints)        │
│                      Mỗi giây lập một Checkpoint kèm Checksum & Private Log  │
│                                   │                                          │
│             ┌─────────────────────┴─────────────────────┐                    │
│             ▼                                           ▼                    │
│    [TIẾP CẬN BÌNH THƯỜNG]                      [TÌNH HUỐNG NGUY HIỂM]        │
│    Cho phép hạ cánh                            Kích hoạt ngắt khẩn cấp HITL: │
│    - Giữ nguyên lộ trình                       - ACCEPT: Cho tiếp tục        │
│    - Thuật toán ToT tính                       - EDIT: Đổi đường băng và     │
│      góc hạ cánh tối ưu                          XOÁ BỎ lệnh bay vào mây     │
│    - Gộp dữ liệu hành trình                    - RESPOND: Yêu cầu bay vòng   │
│      (Reducers: Sum, Append)                   - IGNORE: Huỷ quyền hạ cánh   │
│                                                          vào ProceduralMemory│
│                                                         (Hộp đen kinh nghiệm)│
└──────────────────────────────────────────────────────────────────────────────┘
```

### 1. Màn hình Radar Cắt Lát Ngữ Cảnh 1-Hop (InfoGraph Slice)
Trên bầu trời cả nước có hàng nghìn chuyến bay đang di chuyển, nhưng kiểm soát viên phụ trách đường băng tiếp cận chỉ cần quan sát khu vực cục bộ. Hệ thống radar của anh ta kích hoạt chế độ **Cắt lát 1-hop (1-hop neighborhood slice)**:
- Nút trung tâm: Chiếc máy bay VN-244 đang hạ độ cao.
- Các nút lân cận trực tiếp: Đài dẫn đường vô tuyến VOR, trạm đo gió đầu đường băng 25L, và một chiếc máy bay vận tải đang chạy chậm trên đường lăn cắt ngang.
- Toàn bộ các chuyến bay ở Đà Nẵng, Nội Bài hay các trạm radar tầm xa cách đó hàng trăm dặm hoàn toàn bị ẩn đi.

Nhờ cơ chế lọc này, bộ não của kiểm soát viên (cửa sổ ngữ cảnh LLM) không bị bão hoà bởi hàng triệu điểm ảnh vô ích. **Dung lượng thông tin rác giảm ngay lập tức trên 60%**, giúp anh ta tập trung 100% sự chú ý vào các liên kết có thể gây ra va chạm chết người.

### 2. Lập lịch Đợt Sóng Hạ Cánh Song Song (PlanDag & Kahn Waves)
Hai chiếc máy bay không thể cùng chạm bánh xuống một đường băng tại cùng một thời điểm. Nhưng hai chiếc máy bay hoàn toàn có thể hạ cánh đồng thời trên hai đường băng song song 25L và 25R. 

Hệ thống quản lý không lưu mô hình hoá toàn bộ quy trình hạ cánh thành một **Đồ thị kế hoạch PlanDag**:
- Tác vụ $A$: Kiểm tra áp suất khí quyển.
- Tác vụ $B$: Chiếc máy bay 1 hạ cánh trên đường băng 25L (phụ thuộc vào $A$).
- Tác vụ $C$: Chiếc máy bay 2 hạ cánh trên đường băng 25R (phụ thuộc vào $A$).
- Tác vụ $D$: Xe buýt đón khách tiếp cận máy bay 1 (phụ thuộc vào $B$).
- Tác vụ $E$: Xe buýt đón khách tiếp cận máy bay 2 (phụ thuộc vào $C$).

Thuật toán Kahn phân tích tô-pô sẽ chia quy trình trên thành 3 đợt sóng sẵn sàng (Waves):
- **Đợt 1 (Wave 0)**: Chạy tác vụ $A$.
- **Đợt 2 (Wave 1)**: Chạy song song tác vụ $B$ và $C$ trên hai đường băng độc lập mà không hề va chạm nhau.
- **Đợt 3 (Wave 2)**: Chạy song song tác vụ $D$ và $E$.

Nếu một phi công lập kế hoạch bay lộn xộn: "Máy bay chỉ hạ cánh sau khi xe buýt đón khách đã tới chân cầu thang, nhưng xe buýt chỉ được phép xuất phát sau khi máy bay đã hạ cánh", hệ thống sẽ lập tức rung chuông báo động: **Phát hiện chu trình bế tắc (`DagError::CycleDetected`)** ngay trên mặt đất, kiên quyết từ chối cấp phép bay.

### 3. Ngân Sách Nhiên Liệu Thử Lại (Retry Budget)
Mỗi chiếc máy bay khi tiếp cận đường băng đều mang theo một lượng dầu dự trữ cho trường hợp thời tiết xấu. Đây chính là **Ngân sách thử lại (`retry_budget`)**:
- Lần 1: Gặp luồng gió đứt (wind shear), phi công kéo ga bay vòng lại (`attempts = 1 <= budget`). Hệ thống vẫn coi nhiệm vụ là `Pending`.
- Lần 2: Tiếp tục thử lại lần hai và hạ cánh thành công (`state = Succeeded`).
- Nếu sau 3 lần bay vòng mà vẫn không thể hạ cánh (`attempts > budget`), trạng thái chuyển thành `ExhaustedRetries`. Máy bay bắt buộc phải kích hoạt quy trình hạ cánh khẩn cấp tại sân bay dự phòng Liên Khương, tuyệt đối không để máy bay cạn kiệt nhiên liệu trên không.

### 4. Chiếc Phanh Khẩn Cấp của Trưởng Đài Chỉ Huy (HITL & Key Deletion Diff)
Hệ thống máy tính tự động có thể đề xuất một kế hoạch điều hướng:
`proposed_args = {"runway": "25L", "speed": "160kts", "terrain_warning_override": "true"}`.

Khi nhìn thấy tham số `terrain_warning_override = "true"` (bỏ qua cảnh báo va chạm địa hình), Trưởng đài chỉ huy lập tức kích hoạt **Ngắt can thiệp khẩn cấp (HITL Interrupt)**:
- Anh ta chọn chế độ `Edit`.
- Anh ta sửa tốc độ xuống `140kts`, và quan trọng nhất: **Anh ta dùng chuột bôi đen và XOÁ BỎ hoàn toàn tham số `terrain_warning_override`** khỏi bảng điều khiển.
- Chiếc máy bay thực thi lệnh với bộ tham số an toàn đã được gọt giũa.
- **Điểm mấu chốt kỹ thuật**: Hệ thống hộp đen không lưu (ProceduralMemory) so sánh tập khoá cũ và mới, phát hiện ra khoá `terrain_warning_override` đã bị xoá bỏ, liền ghi khắc một quy tắc vĩnh viễn vào bộ nhớ: `tool=landing arg=terrain_warning_override->[DELETED]`. Trong tất cả các chuyến bay tương lai, hệ thống tự động loại bỏ tham số này, không bao giờ để AI tự ý đề xuất vượt quyền an toàn địa hình thêm một lần nào nữa!

## 3. Khái niệm & Cơ chế kỹ thuật chuyên sâu

### 3.1. Kỹ nghệ Suy luận Có Cấu trúc (Structured Reasoning)

#### 1. Cạm bẫy Suy luận Tham lam & Tree-of-Thoughts (ToT)
Trong kỹ nghệ mô hình ngôn ngữ, chuỗi suy nghĩ (Chain-of-Thought - CoT) đơn thuần giải quyết bài toán theo một đường thẳng duy nhất. Tuy nhiên, với các bài toán đòi hỏi lập kế hoạch có nhìn trước và quay lui, CoT bộc lộ nhược điểm chết người: tính tham lam cục bộ.

Xét bài toán **Game of 24**: Cho trước 4 số thực, hãy sử dụng các phép tính $+ , -, \times, \div$ và dấu ngoặc đơn để tạo ra kết quả đúng bằng $24.0$. 
Xét tập số đầu vào: `[4.0, 9.0, 10.0, 13.0]`.

Hãy phân tích không gian tìm kiếm:
1. **Lối mòn của thuật toán tham lam (Greedy Failure)**:
   - Mô hình nhìn thấy hai số $10$ và $13$. Phép cộng $10 + 13 = 23$. Số $23$ chỉ cách mục tiêu $24$ đúng một khoảng cách $\Delta = |23 - 24| = 1.0$.
   - Bộ đánh giá tham lam sẽ chấm điểm nhánh $10 + 13 = 23$ số điểm cao nhất.
   - Tập số còn lại sau bước 1: `[4, 9, 23]`.
   - Đến bước này, không có bất kỳ phép toán nào giữa $4, 9$ và $23$ có thể tạo ra $24$:
     $23 + 4 = 27$, $23 - 9 = 14$, $23 \times 4 = 92$, $(23 + 9) \div 4 = 8$, $(23 - 4) = 19$...
   - Toàn bộ nhánh suy luận sụp đổ hoàn toàn. Mô hình CoT tuyến tính không có cơ chế quay lui sẽ bắt đầu bịa đặt hoặc chịu thua.

2. **Cách Tree-of-Thoughts (ToT) chinh phục bài toán**:
   ToT mô hình hoá bài toán thành một cây quyết định. Tại mỗi trạng thái `ThoughtNode`:
   - Trạng thái bắt đầu: `numbers = [4.0, 9.0, 10.0, 13.0]`, nhãn `vote = ThoughtVote::Maybe`.
   - Hàm `generate_successors` sinh ra tất cả các cặp chỉ số $(i, j)$ với $i < j$. Với 4 số, có $\binom{4}{2} = 6$ cặp số. Mỗi cặp có thể thực hiện tối đa 6 phép tính: $a + b$, $a - b$, $b - a$, $a \times b$, $a \div b$ (nếu $b \neq 0$), $b \div a$ (nếu $a \neq 0$). Tổng cộng có $6 \times 6 = 36$ trạng thái con ở tầng 1.
   - Hàm đánh giá `evaluate` gán nhãn cho từng nhánh:
     - `ThoughtVote::Sure`: Khi chỉ còn đúng 1 số duy nhất và sai số $|x - 24.0| < 10^{-5}$.
     - `ThoughtVote::Impossible`: Khi bài toán rơi vào trạng thái vô nghiệm chắc chắn.
     - `ThoughtVote::Maybe`: Khi còn nhiều hơn 1 số và cần tiếp tục phân nhánh.
   - Nhánh thành công thực sự lại đi qua một trạng thái trông có vẻ rất "tệ" đối với thuật toán tham lam:
     - Bước 1: Ghép cặp $9$ và $13$ qua phép trừ: $13 - 9 = 4$. Tập số trở thành `[4, 10, 4]`. Khoảng cách tới 24 lúc này là $|10 - 24| = 14$ (rất xa so với 1 của số 23, nhưng ToT vẫn giữ lại vì nó là `Maybe`).
     - Bước 2: Ghép cặp $10$ và $4$ qua phép trừ: $10 - 4 = 6$. Tập số trở thành `[4, 6]`.
     - Bước 3: Ghép cặp $4$ và $6$ qua phép nhân: $4 \times 6 = 24$. Tập số trở thành `[24]`.
     - Bộ đánh giá nhận diện trạng thái `ThoughtVote::Sure`, thuật toán dừng lại và trả về biểu thức hoàn chỉnh: `((10 - (13 - 9)) * 4) = 24` hoặc `((4 - 10) * (9 - 13)) = 24`.

Bằng cách sử dụng hàng đợi duyệt theo chiều rộng (`VecDeque<ThoughtNode>`), ToT bảo đảm tìm ra nghiệm đúng trong vòng dưới 2 mili-giây mà không bị đánh lừa bởi ảo ảnh khoảng cách của giải thuật tham lam.

#### 2. Biểu quyết Tự Nhất quán Đa số (Self-Consistency Voting)
Trong môi trường thực tế, câu trả lời của mô hình ngôn ngữ phụ thuộc vào nhiệt độ lấy mẫu ($T$). Kỹ thuật Self-Consistency khắc phục điều này bằng cách lấy mẫu $K$ quỹ đạo suy luận độc lập:
$$\mathcal{P} = \{P_1, P_2, \dots, P_K\}$$
Mỗi quỹ đạo $P_i$ có một câu trả lời cuối cùng $A_i$ và một độ tin cậy ước tính $C_i \in [0.0, 1.0]$.

Bộ biểu quyết `SelfConsistencyVoter` thực hiện:
1. Chuẩn hoá chuỗi văn bản của đáp án: chuyển về chữ thường, cắt tỉa khoảng trắng đầu cuối (`text.trim().to_lowercase()`).
2. Nhóm các đáp án giống nhau, tính số lượng phiếu bầu $N(A)$ và tổng trọng số tin cậy:
$$W(A) = \sum_{P_i \mid \text{Norm}(A_i) = A} C_i$$
3. Sắp xếp các ứng viên theo tiêu chí đa số biểu quyết: ứng viên có số lượng phiếu $N(A)$ cao nhất sẽ thắng. Trong trường hợp hòa phiếu ($N(A_1) = N(A_2)$), ứng viên có tổng trọng số tin cậy $W(A)$ lớn hơn sẽ phá vỡ thế cân bằng (tie-breaking) một cách tất định.
4. Trả về `VoteResult` chứa đáp án thắng cuộc, tỷ lệ đồng thuận và điểm tin cậy chuẩn hoá:
$$\text{ConfidenceScore} = \frac{W(A_{\text{win}})}{\sum_{A} W(A)}$$

#### 3. Phân tách Kế hoạch và Giải quyết (Plan-and-Solve Runner)
Mô hình ngôn ngữ thường bị quá tải nhận thức khi phải vừa đóng vai trò kiến trúc sư lập chiến lược, vừa đóng vai trò thợ gõ mã nguồn chi tiết trong cùng một lượt sinh. 

Kiến trúc `PlanAndSolveRunner` phân rã quy trình thành hai pha có ranh giới kiểu rõ ràng:
- **Pha 1: Lập kế hoạch (Planner Closure)**: Nhận mục tiêu tổng quát `&str`, trả về một danh sách các nhiệm vụ con có thứ tự `Vec<SubTask>`, trong đó mỗi nhiệm vụ có `step_id`, mô tả hành động `description`, và gợi ý công cụ `tool_hint`.
- **Pha 2: Giải quyết tuần tự (Solver Closure)**: Nhận nhiệm vụ con hiện tại `&SubTask` và toàn bộ lịch sử các bước đã giải quyết trước đó `&[SubTaskResult]`. Nếu bất kỳ bước nào trả về `success = false`, toàn bộ quy trình dừng lại ngay lập tức (fail-fast), báo cáo chính xác bước gặp sự cố và bảo toàn các kết quả trung gian đã hoàn tất.

#### 4. Vòng lặp Phản tỉnh Ngôn ngữ (Reflexion Verbal Loop)
Học tăng cường truyền thống đòi hỏi việc cập nhật hàng tỷ trọng số mô hình thông qua gradient descent. Ngược lại, kiến trúc Reflexion áp dụng nguyên lý **Học tăng cường bằng ngôn ngữ tự nhiên (Verbal Reinforcement Learning)**:
- Tác tử hành động (Actor) đề xuất giải pháp $a_t$ dựa trên đề bài và lịch sử các lần thử trước.
- Bộ thẩm định môi trường (Evaluator) kiểm tra tính đúng đắn của giải pháp, trả về $(passed, outcome)$.
- Nếu $passed = false$, bộ phản tỉnh (Reflector) phân tích sai lầm và sinh ra một lời tự vấn có cấu trúc $r_t$:
  $$r_t = \text{Reflect}(\text{Goal}, a_t, \text{Outcome})$$
- Lời tự vấn $r_t$ được lưu vào lịch sử `ReflexionAttempt` và nạp thẳng vào prompt của lần thử tiếp theo $a_{t+1}$. Thử nghiệm cho thấy tác tử có khả năng tự sửa lỗi biên dịch Rust hoặc lỗi thiếu tham số chỉ sau 1 đến 2 vòng phản tỉnh.

#### 5. Kỷ luật 5 Thao tác Nhận thức (CognitiveTool5Op)
Để ngăn chặn tác tử AI "nhảy cóc" kết luận mà không có căn cứ, quy trình nhận thức bắt buộc phải đi qua 5 trạng thái đơn điệu tăng:

$$\begin{aligned}
\text{Understand} &\implies \text{Hiểu rõ các giả định và tiền đề của bài toán.} \\
\text{Extract} &\implies \text{Trích xuất các thực thể, kiểu dữ liệu và mối liên kết.} \\
\text{Highlight} &\implies \text{Làm nổi bật các điểm thắt cổ chai và xung đột logic.} \\
\text{Apply} &\implies \text{Áp dụng công cụ hoặc phép biến đổi dữ liệu cụ thể.} \\
\text{Validate} &\implies \text{Xác thực kết quả đầu ra đối chiếu với các bất biến ban đầu.}
\end{aligned}$$

Hệ thống Rust kiểm soát chặt chẽ: phương thức `execute_step` sẽ trả về `Err` nếu tác tử cố tình gọi một pha không khớp với `expected_next_phase()`.

---

### 3.2. Kỹ nghệ Đồ thị kép vĩ mô (Dual-Graph Engineering)

Hệ thống tác tử cấp cao được vận hành bởi hai đồ thị đối ngẫu: một đồ thị mô tả không gian tri thức (InfoGraph) và một đồ thị điều phối luồng thực thi (PlanDag).

```
┌─────────────────────────────────────────────────────────────────────────────┐
│                       SO SÁNH BẢN CHẤT ĐỒ THỊ KÉP                          │
├───────────────────────────────────┬─────────────────────────────────────────┤
│    ĐỒ THỊ TRI THỨC (INFOGRAPH)    │       ĐỒ THỊ KẾ HOẠCH (PLANDAG)         │
├───────────────────────────────────┼─────────────────────────────────────────┤
│ Biểu diễn: Thực thể tri thức      │ Biểu diễn: Tác vụ thực thi có thứ tự    │
│ Đỉnh (Nodes): Hàm, struct, bảng DB│ Đỉnh (Nodes): Nhiệm vụ con, lệnh gọi tool│
│ Cạnh (Edges): "calls", "defines"  │ Cạnh (Edges): "depends_on"              │
│ Thuật toán: 1-Hop Neighborhood    │ Thuật toán: Kahn Topological Waves      │
│ Mục tiêu: Tiết kiệm >60% token rác│ Mục tiêu: Song song hoá và chống bế tắc │
└───────────────────────────────────┴─────────────────────────────────────────┘
```

#### 1. InfoGraph & Kỹ thuật Cắt lát Ngữ cảnh 1 bước (1-Hop Slice)
Đồ thị `InfoGraph` lưu trữ các đỉnh `InfoNode` (mã định danh, loại thực thể, nội dung văn bản, tập nhãn) và danh sách cạnh có hướng `InfoEdge` (đỉnh nguồn, đỉnh đích, loại quan hệ, trọng số).

Khi tác tử cần xử lý một đỉnh trung tâm $u$ (ví dụ: hàm `authenticate_user`):
1. Thuật toán `slice_1hop_context` quét toàn bộ tập cạnh $E$ để tìm tất cả các đỉnh lân cận $v$ thoả mãn:
$$\exists e \in E \mid (e.\text{from} = u \land e.\text{to} = v) \lor (e.\text{from} = v \land e.\text{to} = u)$$
2. Thu thập thông tin chi tiết của $u$, danh sách các nút lân cận trực tiếp $v$, và các cạnh kết nối trực tiếp giữa chúng.
3. Sinh ra chuỗi văn bản ngữ cảnh có cấu trúc `formatted_prompt`:
```
[Target: authenticate_user (function)]
Xác thực mã thông báo JWT và quyền hạn
--- Relationships ---
authenticate_user --[calls]--> verify_jwt_signature (weight: 10)
user_controller --[calls]--> authenticate_user (weight: 8)
```

**Chứng minh định lượng giảm thiểu token**:
Xét một đồ thị gồm $N = 20$ đỉnh, mỗi đỉnh chứa nội dung mã nguồn trung bình $L = 80$ ký tự. Tổng dung lượng văn bản của toàn bộ đồ thị là:
$$D_{\text{full}} = 20 \times 80 = 1.600 \text{ ký tự}$$
Khi thực hiện cắt lát 1-hop quanh một đỉnh có 2 liên kết lân cận, dung lượng văn bản của ngữ cảnh cắt lát chỉ bao gồm 3 đỉnh ($1$ đỉnh trung tâm + $2$ đỉnh lân cận) và $2$ dòng mô tả cạnh:
$$D_{\text{slice}} = 3 \times 80 + 2 \times 40 = 320 \text{ ký tự}$$
Tỷ lệ cắt giảm dung lượng đạt:
$$\text{Reduction} = 1.0 - \frac{320}{1.600} = 80.0\% > 60\%$$
Kỹ thuật này bảo đảm tác tử chỉ nhìn thấy đúng những gì cần thiết để ra quyết định, loại bỏ hoàn toàn hiện tượng phân tâm chú ý.

#### 2. PlanDag & Giải thuật Kahn Bóc tách Đợt Sóng Song Song
Một kế hoạch phức tạp gồm nhiều tác vụ phụ thuộc lẫn nhau được mô hình hoá thành một đồ thị có hướng phi chu trình (DAG). Mỗi tác vụ được lưu trữ trong một ô `DagSlot`:
- `dependencies: Vec<String>`: Danh sách các tác vụ bắt buộc phải hoàn thành trước.
- `retry_budget: u8`: Số lần cho phép thử lại.
- `attempts: u8`: Số lần đã thử thực tế.
- `state: DagNodeState`: Trạng thái (`Pending`, `Succeeded`, `ExhaustedRetries`).

Thuật toán Kahn bóc tách các đợt sóng thực thi song song (`topological_ready_waves`) vận hành theo 4 bước toán học:
1. Xây dựng bảng bán bậc vào (In-Degree) cho mọi đỉnh trong đồ thị:
$$\text{in\_degree}(v) = |\{u \in V \mid u \in v.\text{dependencies}\}|$$
Đồng thời, kiểm tra tính toàn vẹn tham chiếu: nếu $v$ phụ thuộc vào một mã định danh không tồn tại trong danh bạ đỉnh, trả về ngay lỗi `DagError::UnknownDependency`.
2. Khởi tạo hàng đợi chứa tất cả các đỉnh có bán bậc vào bằng 0:
$$Q_0 = \{v \in V \mid \text{in\_degree}(v) = 0\}$$
3. Vòng lặp bóc tách đợt sóng:
   - Tại đợt sóng thứ $k$, toàn bộ các đỉnh trong hàng đợi hiện tại $Q_k$ được gom thành đợt thực thi song song: $\text{Wave}_k = Q_k$.
   - Với mỗi đỉnh $u \in \text{Wave}_k$, duyệt qua tất cả các đỉnh con $w$ phụ thuộc vào nó, giảm bán bậc vào của $w$:
     $$\text{in\_degree}(w) \leftarrow \text{in\_degree}(w) - 1$$
   - Nếu $\text{in\_degree}(w) = 0$, đưa $w$ vào danh sách ứng viên cho đợt sóng tiếp theo $Q_{k+1}$.
   - Tăng biến đếm tổng số đỉnh đã xử lý: $C \leftarrow C + |\text{Wave}_k|$.
4. **Kiểm tra Chu trình**: Sau khi hàng đợi rỗng, so sánh biến đếm $C$ với tổng số đỉnh $|V|$. Nếu $C < |V|$, điều đó chứng minh rằng các đỉnh còn lại có bán bậc vào $> 0$ và tạo thành ít nhất một chu trình khép kín. Thuật toán trả về `Err(DagError::CycleDetected)`.

```
        A (deg: 0)                     Wave 0: [A]
       / \
      ▼   ▼
     B     C   (deg: 1)                Wave 1: [B, C]  (Chạy song song!)
      \   /
       ▼ ▼
        D      (deg: 2)                Wave 2: [D]
```

#### 3. Quản trị Ngân sách Thử lại Cục bộ (Local Retry Budget)
Khi một tác vụ trong đồ thị thất bại, phương thức `record_attempt(id, success: false)` can thiệp:
- Tăng biến đếm `slot.attempts += 1`.
- Nếu `slot.attempts <= slot.retry_budget`: Trạng thái của nút vẫn giữ nguyên là `DagNodeState::Pending`. Điều này cho phép bộ lập lịch runtime tiếp tục thử lại nút này ở chu kỳ tiếp theo mà không làm sụp đổ toàn bộ kế hoạch.
- Nếu `slot.attempts > slot.retry_budget`: Trạng thái chuyển thành `DagNodeState::ExhaustedRetries`. Lúc này đồ thị mới chính thức dừng lại để yêu cầu can thiệp cấp cao.

#### 4. Bộ biên dịch trước Đồ thị AoT (Ahead-of-Time Subgraph Compiler)
Cấu trúc `AotSubgraphCompiler` lấy một `PlanDag` tĩnh và biên dịch trước thành `CompiledSchedule`:
- Danh sách các đợt sóng `CompiledWave` kèm chỉ số thứ tự.
- Tổng số tác vụ `total_tasks`.
- Mức độ đồng thời cực đại:
$$\text{max\_concurrency} = \max_{w \in \text{waves}} |w.\text{task\_ids}|$$
Thông tin này cho phép hệ thống cấp phát chính xác số lượng tài nguyên tính toán (thread pool hoặc worker pool), tối ưu hoá tuyệt đối chi phí vận hành.

---

### 3.3. Máy trạng thái nút StateGraph và Điểm kiểm soát Stripe Blueprints

Để quản lý trạng thái chia sẻ giữa các nút trong một chu trình suy luận dài, chúng ta sử dụng `StateGraph<S>`.

#### 1. Ba Hàm Gộp Trạng Thái (State Reducers)
Khi nhiều nút thực thi song song cùng cập nhật vào trạng thái toàn cục, hệ thống hỗ trợ 3 cơ chế gộp toán học:
- **`Overwrite`**: Giá trị mới nhất ghi đè hoàn toàn giá trị cũ. Thích hợp cho các cờ trạng thái nhị phân, cấu hình môi trường.
- **`Append`**: Nối các phần tử mới vào cuối vector. Thích hợp cho việc thu thập lịch sử thông điệp, danh sách tệp mã đã sửa đổi.
- **`Sum`**: Cộng dồn số học. Thích hợp cho việc đo lường chi phí tài nguyên, số lượt gọi API, độ trễ tích luỹ.

#### 2. Cơ chế Điểm Kiểm Soát Tất Định Stripe Blueprints
Mỗi khi một nút thực thi xong thông qua phương thức `transition`, hệ thống tạo ra một điểm kiểm soát bất biến `BlueprintCheckpoint<S>`:
- `checkpoint_id: u64`: Định danh tuần tự đơn điệu tăng.
- `node_id: String`: Tên nút vừa thực thi.
- `state_snapshot: S`: Bản sao lưu trạng thái toàn vẹn tại thời điểm đó.
- `private_log: Vec<String>`: Nhật ký nội bộ của nút (chỉ phục vụ kiểm toán sau sự cố).
- `checksum: u64`: Mã băm toàn vẹn được tính toán bằng cách băm trạng thái qua `DefaultHasher`:
```rust
let mut hasher = DefaultHasher::new();
new_state.hash(&mut hasher);
let checksum = hasher.finish();
```

Khi phát hiện một hành động của tác tử vi phạm chính sách an toàn, hệ thống gọi `rollback_to(target_checkpoint_id)`:
- Tìm kiếm điểm kiểm soát mục tiêu trong danh sách.
- Khôi phục trạng thái `current_state` về đúng bản sao lưu của điểm kiểm soát đó.
- Cắt tỉa (`truncate`) toàn bộ các điểm kiểm soát sinh ra sau mốc đó.

---

### 3.4. Ngắt can thiệp HITL và Sửa lỗi Edge Case #5 (Key Deletion Diff)

Khi tác tử chuẩn bị thực thi một công cụ nguy hiểm, quy trình bị ngắt để xin ý kiến con người qua `HitlInterrupt`:
- `Accept`: Cho phép chạy nguyên văn các tham số.
- `Edit { new_arguments }`: Con người điều chỉnh các tham số.
- `Respond { feedback_message }`: Từ chối và gửi phản hồi văn bản yêu cầu lập lại kế hoạch.
- `Ignore`: Huỷ bỏ tác vụ và ghi vào danh sách cấm.

#### Mổ xẻ Chi tiết Sửa lỗi Edge Case #5:
Trong các thiết kế ngây thơ, khi con người chọn chế độ `Edit`, lập trình viên thường chỉ so sánh các khoá xuất hiện trong `new_arguments` đối chiếu với `orig_arguments`:
```rust
// CÁCH CÀI ĐẶT CŨ CÓ LỖI (BỎ SÓT KHOÁ BỊ CON NGƯỜI XOÁ):
for (k, v) in &new_arguments {
    if orig_arguments.get(k) != Some(v) {
        memory.tool_override_rules.push(format!("tool={} arg={}->{}", tool_name, k, v));
    }
}
```

**Nguy cơ thực tế**:
Giả sử tác tử gọi công cụ quản lý cơ sở dữ liệu:
`orig_arguments = {"table": "orders", "action": "delete", "cascade_drop_all": "true"}`.
Người giám sát nhìn thấy tham số `cascade_drop_all` quá nguy hiểm, liền xoá bỏ nó khỏi bảng cấu hình:
`new_arguments = {"table": "orders", "action": "delete"}`.

Nếu chỉ duyệt qua `new_arguments`, hệ thống thấy hai khoá `table` và `action` không hề thay đổi giá trị. Nó kết luận: *"Không có quy tắc ghi đè nào mới!"* Tham số `cascade_drop_all` đã bị con người xoá bỏ trong phiên này, nhưng tác tử **hoàn toàn không học được bài học nào cho tương lai**. Ở phiên làm việc sau, nó sẽ tiếp tục sinh ra tham số `cascade_drop_all = true`.

**Giải pháp Sửa lỗi Edge Case #5 Hoàn chỉnh**:
Chúng ta bổ sung một vòng lặp kiểm tra tập hiệu của các tập khoá:
$$\Delta_{\text{deleted}} = \text{Keys}(\text{orig\_arguments}) \setminus \text{Keys}(\text{new\_arguments})$$
```rust
// SỬA LỖI EDGE CASE #5: Kiểm tra các khoá có trong orig nhưng bị con người XOÁ trong new_arguments
for k in orig_arguments.keys() {
    if !new_arguments.contains_key(k) {
        memory.tool_override_rules.push(format!(
            "tool={} arg={}->[DELETED]",
            tool_name, k
        ));
    }
}
```
Nhờ cơ chế diff này, quy tắc `tool=database arg=cascade_drop_all->[DELETED]` được lưu trữ vĩnh viễn vào `ProceduralMemory`. Trong tất cả các lần gọi công cụ sau này, hệ thống sẽ tự động lọc bỏ khoá này trước khi thực thi.

---

### 3.5. Trí nhớ nhận thức 4 tầng, Đồ thị lưỡng thời gian và BDI Engine

#### 1. Hệ thống Trí nhớ Nhận thức 4 tầng (CognitiveMemory4Tier - MEM1)
Bộ nhớ tác tử được tổ chức thành 4 tầng mô phỏng tâm lý học nhận thức:
- **Tầng 1: Trí nhớ làm việc (Working Memory)**: Sổ nháp tức thời lưu các chuỗi văn bản suy nghĩ trong phiên làm việc hiện tại (`Vec<String>`).
- **Tầng 2: Trí nhớ tình tiết (Episodic Memory)**: Lưu giữ các sự kiện cụ thể đã trải qua trong quá khứ (`EpisodicItem`) gồm mã định danh, mốc thời gian logic `timestamp`, bản tóm tắt, tập từ khoá và điểm số quan trọng `importance`.
- **Tầng 3: Trí nhớ ngữ nghĩa (Semantic Memory)**: Mạng lưới tri thức vĩnh viễn lưu dưới dạng các định đề bộ ba (`SemanticFact` gồm `subject`, `predicate`, `object`, `confidence`).
- **Tầng 4: Trí nhớ thủ tục (Procedural Memory)**: Lưu trữ các phản xạ và quy tắc hành vi (`triage_rules`, `response_rules`, `tool_override_rules`).

**Cơ chế Củng cố Ký ức (Memory Consolidation)**:
Khi hai mẩu ký ức tình tiết $A$ và $B$ có tập từ khoá ngữ nghĩa tương đồng theo chỉ số Jaccard:
$$J(A, B) = \frac{|K_A \cap K_B|}{|K_A \cup K_B|} \ge 0.7$$
Phương thức `consolidate_episodes` sẽ tự động đúc kết hai trải nghiệm này thành một sự thật ngữ nghĩa chung trong Trí nhớ ngữ nghĩa với độ tin cậy $0.95$, giúp tác tử chuyển hoá kinh nghiệm rời rạc thành tri thức khái quát.

#### 2. Đồ thị Lưỡng thời gian (BiTemporalGraph)
Trong các hệ thống lưu trữ tri thức phức tạp, một sự thật luôn gắn liền với hai trục thời gian độc lập:
1. **ValidTime (Thời gian hiệu lực trong thế giới thực)**: Khoảng thời gian $[\text{valid\_from}, \text{valid\_to})$ mà sự kiện thực sự đúng trên thực tế.
2. **TxTime (Thời gian giao dịch trong hệ thống)**: Khoảng thời gian $[\text{tx\_from}, \text{tx\_to})$ mà hệ thống tin rằng sự kiện đó là đúng.

```
TxTime (Thời điểm hệ thống ghi nhận)
  ▲
  │
20│                        [Sự thật đính chính: IP = 10.0.0.2]
  │                        (tx_from: 20, tx_to: MAX)
  │                        (valid_from: 50, valid_to: 100)
10│    [Sự thật cũ: IP = 10.0.0.1]
  │    (tx_from: 10, tx_to: 20)
  │    (valid_from: 0, valid_to: 100)
  └─────────────────────────────────────────────────────────► ValidTime (Thực tế xảy ra)
       0                   50               100
```

Khi phát hiện sai sót trong quá khứ, chúng ta không thực hiện thao tác ghi đè huỷ hoại (destructive overwrite). Thay vào đó, hàm `retract_fact` đóng phiên bản giao dịch cũ bằng cách gán `tx_to = current_tx`, sau đó chèn các bản ghi sự thật mới.

Phương thức `query_time_travel(entity_id, as_of_valid, as_of_tx)` cho phép chúng ta trả lời chính xác hai câu hỏi kiểm toán quan trọng:
- *"Hôm nay, chúng ta biết rằng vào ngày hôm qua sự kiện gì đã xảy ra?"*
- *"Vào tuần trước, hệ thống của chúng ta tin rằng sự kiện gì đã diễn ra vào ngày hôm qua?"*

#### 3. Động cơ Niềm tin - Mong muốn - Ý định (BDI Engine)
- **Endurants**: Các thực thể trường tồn qua thời gian và duy trì danh tính liên tục (ví dụ: máy chủ, người dùng, cơ sở dữ liệu).
- **Perdurants**: Các thực thể biến thiên theo thời gian hoặc các tiến trình diễn ra theo giai đoạn (ví dụ: quy trình vá lỗi phần mềm, phiên giao dịch).
- `Belief`: Niềm tin hiện tại của tác tử về các thực thể.
- `Desire`: Mục tiêu mà tác tử mong muốn đạt được.
- `Intention`: Kế hoạch hành động cụ thể gồm các bước `action_steps`. Khi tác tử lần lượt hoàn thành từng bước qua `advance_intention`, mục tiêu tương ứng mới chính thức được đánh dấu là `fulfilled = true`.

---

### 3.6. Phân đoạn Markdown, Truy xuất lai RRF & Kiểm chứng trích dẫn

#### 1. Bộ phân đoạn MarkdownHeadingChunker và Sửa lỗi Edge Cases #1 & #2
Bộ phân đoạn tài liệu kỹ thuật có nhiệm vụ bóc tách văn bản Markdown thành các đoạn trích nhỏ `DocChunk` mà vẫn giữ nguyên đường dẫn cây tiêu đề phân cấp (ví dụ: `"Chương 1 > Cấu Trúc > Bộ Nhớ"`).

Quá trình cài đặt thực tế đòi hỏi xử lý triệt để hai lỗi biên kinh điển:
- **SỬA LỖI EDGE CASE #1: Bảo vệ `max_w = 0`**:
  Nếu người dùng cấu hình nhầm `max_w = 0`, các phép tính trừ không an toàn như `max_w - overlap_w` sẽ dẫn đến lỗi hoảng loạn tràn số nguyên (integer underflow panic) hoặc lỗi chia cho 0.
  Giải pháp: Cưỡng chế `let mw = max_w.max(1);` và sử dụng các phép toán bão hoà `saturating_sub`:
  ```rust
  let mw = max_w.max(1);
  let step = mw.saturating_sub(overlap_w.min(mw.saturating_sub(1))).max(1);
  ```
- **SỬA LỖI EDGE CASE #2: Bỏ qua dòng chú thích mã nguồn `# comment` trong rào mã (Code Fence)**:
  Trong các tệp tài liệu kỹ thuật, các khối mã Python hay Bash thường chứa các dòng chú thích bắt đầu bằng `#` (ví dụ: `# Đây là comment`). Nếu chỉ kiểm tra `line.starts_with('#')`, toàn bộ các dòng chú thích mã nguồn sẽ bị nhận diện sai thành thẻ tiêu đề Markdown H1!
  Giải pháp: Sử dụng máy trạng thái theo dõi cờ `in_code_fence`. Khi gặp dòng mở rào mã ```` ``` ```` hoặc `~~~`, lật trạng thái `in_code_fence = !in_code_fence`. Khi đang ở trong rào mã, mọi ký tự `#` đều được xem là văn bản thông thường, bảo vệ tuyệt đối tính toàn vẹn của ngăn xếp tiêu đề.

#### 2. Hệ thống Truy xuất lai HybridRetriever và Sửa lỗi Edge Case #3
Một hệ thống tìm kiếm thông tin hiện đại không thể chỉ dựa vào một phương pháp duy nhất:
- **Okapi BM25**: Xuất sắc trong việc bắt trúng các từ khoá kỹ thuật chính xác (mã lỗi `E0382`, tên hàm `slice_1hop_context`).
  Công thức điểm BM25 cho tài liệu $D$ và câu truy vấn $Q$:
  $$\text{Score}(D, Q) = \sum_{q \in Q} \text{IDF}(q) \cdot \frac{\text{TF}(q, D) \cdot (k_1 + 1)}{\text{TF}(q, D) + k_1 \cdot \left(1 - b + b \cdot \frac{|D|}{\text{avgdl}}\right)}$$
  Trong đó: $k_1 = 1.5$, $b = 0.75$, và nghịch đảo tần suất tài liệu:
  $$\text{IDF}(q) = \ln\left(1 + \frac{N - \text{DF}(q) + 0.5}{\text{DF}(q) + 0.5}\right)$$
- **Dense Cosine**: Xuất sắc trong việc nắm bắt sự tương đồng ngữ nghĩa trừu tượng:
  $$\text{Cosine}(u, v) = \frac{u \cdot v}{\|u\|_2 \|v\|_2}$$
- **Dung hợp thứ hạng tương hỗ (Reciprocal Rank Fusion - RRF)**:
  $$\text{RRF}(d) = \sum_{m \in \{\text{BM25}, \text{Dense}\}} \frac{1}{k + \text{Rank}_m(d)}$$
  Với $k = 60$. Một tài liệu chỉ có thể giành vị trí quán quân (#1) nếu nó đồng thời đạt thứ hạng cao ở cả hai bảng xếp hạng từ khoá lẫn ngữ nghĩa, đánh bại hoàn toàn các tài liệu chỉ là nhiễu từ khoá hoặc nhiễu ngữ nghĩa đơn lẻ.

- **SỬA LỖI EDGE CASE #3: Khẳng định tính tương thích về kích thước**:
  Hàm `HybridRetriever::search` nhận vào hai lát cắt `chunks: &[DocChunk]` và `embs: &[[f64; 3]]`. Nếu độ dài của hai lát cắt này không khớp nhau, vòng lặp duyệt chỉ số chắc chắn sẽ dẫn đến lỗi truy cập ngoài biên bộ nhớ (out-of-bounds panic).
  Giải pháp: Cưỡng chế kiểm tra bất biến ngay ở dòng đầu tiên của hàm:
  ```rust
  assert_eq!(
      chunks.len(),
      embs.len(),
      "Độ dài mảng chunks ({}) và embs ({}) bắt buộc phải bằng nhau",
      chunks.len(),
      embs.len()
  );
  ```

#### 3. Bộ kiểm chứng trích dẫn văn bản SpanCitationVerifier
Để triệt tiêu hoàn toàn ảo giác trích dẫn của LLM, `SpanCitationVerifier` kiểm chứng từng toạ độ byte lát cắt `CitationSpan`:
- Kiểm tra chỉ số `chunk_id` có tồn tại trong danh mục tài liệu không.
- Kiểm tra toạ độ `start_offset <= end_offset <= chunk.content.len()`.
- Kiểm tra tính an toàn ranh giới ký tự UTF-8 đa byte thông qua `is_char_boundary`.
- So khớp lát cắt chuỗi thực tế `&chunk.content[start..end]` với nội dung tuyên bố `claimed_text`. Nếu có bất kỳ sự sai lệch nào, trả về `CitationVerificationResult::ContentMismatch`.

## 4. Mã nguồn minh họa thực chiến

Dưới đây là mã nguồn minh hoạ các thành phần cốt lõi của Chương 88, được hiện thực hoá bằng 100% Rust 2024 thuần thư viện chuẩn `std`. Toàn bộ các cấu trúc đều sử dụng trường có tên (named fields) và doc-comment tiếng Việt theo chuẩn RiskGate.

### 4.1. Module Suy luận Có Cấu trúc (Tree of Thoughts)

```rust
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
```

### 4.2. Module Đồ thị Kế hoạch (PlanDag & Kahn Waves)

```rust
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
```

### 4.3. Module Máy Trạng thái Nút & Điểm Kiểm Soát Stripe Blueprints

```rust
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
```

### 4.4. Module Ngắt Duyệt Can Thiệp Con Người (HITL & Sửa lỗi Edge Case #5)

```rust
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
```

### 4.5. Module Đồ thị Lưỡng Thời Gian (BiTemporalGraph & Time Travel)

```rust
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
```

### 4.6. Module Phân Đoạn Markdown An Toàn (Sửa lỗi Edge Cases #1 & #2)

```rust
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

// ... (rút gọn — xem code/ch88/src/main.rs)
```

### 4.7. Module Truy Xuất Lai RRF (Sửa lỗi Edge Case #3) & Kiểm Chứng Trích Dẫn

```rust
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
```

### 4.8. Bộ Kiểm Thử Đơn Vị Mẫu (Invariants Verification Suite)

Dưới đây là các ca kiểm thử tiêu biểu trích từ bộ kiểm thử 38 bài của `code/ch88`, chứng minh bằng thực nghiệm tính tất định và khả năng tự sửa lỗi của tác tử:

```rust
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
```

## 5. Bảng tra cứu lỗi biên dịch thường gặp

Khi xây dựng các hệ thống đồ thị phức tạp, cấu trúc bitemporal và máy trạng thái trong Rust 2024, lập trình viên thường xuyên đối mặt với các lỗi liên quan đến quyền sở hữu (ownership), vay mượn (borrowing), và an toàn bộ nhớ. Dưới đây là bảng tra cứu chi tiết 8 lỗi kinh điển nhất:

| Mã lỗi | Triệu chứng & Thông báo của Rustc | Nguyên nhân cốt lõi | Cách khắc phục chuẩn xác |
|---|---|---|---|
| **E0502** | `cannot borrow *self as mutable more than once at a time` khi duyệt danh sách cạnh và cập nhật đỉnh đồ thị | Đang giữ một tham chiếu bất biến duyệt qua `self.edges` nhưng bên trong vòng lặp lại gọi `self.nodes.get_mut(...)` | Tách rời pha truy vấn và pha biến đổi: thu thập các mã đỉnh cần cập nhật vào một `Vec<String>` tạm thời, sau khi kết thúc duyệt cạnh mới tiến hành mượn khả biến `nodes` để cập nhật. |
| **E0382** | `use of moved value: state` khi chuyển trạng thái qua các nút của `StateGraph` | Cố gắng chuyển quyền sở hữu của `state` vào closure hoặc điểm kiểm soát `BlueprintCheckpoint` mà kiểu dữ liệu không thoả mãn ràng buộc `Clone` | Định nghĩa ràng buộc `S: Clone + Hash` trên `StateGraph<S>`. Tại mỗi bước checkpoint, chủ động gọi `state.clone()` để tạo bản sao lưu ảnh chụp bất biến mà không làm mất trạng thái đang thực thi. |
| **E0277** | `the trait bound S: Hash is not satisfied` khi tạo `checksum` cho `BlueprintCheckpoint` | Cấu trúc dữ liệu trạng thái tự định nghĩa quên derive trait `Hash` cần thiết cho thuật toán kiểm tra tính toàn vẹn | Bổ sung `#[derive(Hash, PartialEq, Eq)]` cho struct trạng thái của bạn, hoặc tự hiện thực hoá trait `Hash` nếu struct chứa các kiểu dữ liệu không có sẵn `Hash` (như `f64`). |
| **E0597** | `borrowed value does not live long enough` khi lưu các lát cắt chuỗi `&'a str` trong `DocChunk` | Cố gắng lưu tham chiếu `&str` mượn từ chuỗi Markdown gốc vào struct `DocChunk` có vòng đời dài hơn | Chuyển đổi toàn bộ các trường chuỗi trong struct dữ liệu lâu dài sang `String` sở hữu độc lập (owned string). Việc sao chép chuỗi tốn một lượng nhỏ chi phí cấp phát heap nhưng đổi lại sự an toàn tuyệt đối và tính linh hoạt về vòng đời. |
| **E0499** | `cannot borrow self.nodes as mutable more than once` khi chạy giải thuật Kahn trong `PlanDag` | Hai nút khác nhau trong cùng một đồ thị cố gắng mượn khả biến bảng băm `self.nodes` đồng thời | Sử dụng mô hình `BTreeMap<String, DagSlot>` với chỉ mục bằng chuỗi hoặc số nguyên (`usize`). Không bao giờ lưu con trỏ tham chiếu trực tiếp giữa các nút đồ thị trong Rust an toàn mà hãy sử dụng khoá định danh gián tiếp (Adjacency Indexing). |
| **E0061** | `this function takes 7 arguments but 8 arguments were supplied` trong lời gọi `record_fact` | Trình biên dịch Clippy cảnh báo `-D clippy::too-many-arguments` khi hàm có quá 7 tham số | Gom nhóm các tham số có quan hệ chặt chẽ thành một struct có tên riêng (ví dụ: `ValidTimeRange { valid_from, valid_to }`), giúp chữ ký hàm súc tích và tuân thủ quy tắc named fields. |
| **E0308** | `mismatched types: expected Option<CognitiveOpPhase>, found CognitiveOpPhase` trong `execute_step` | Quên bọc giá trị enum trong `Some(...)` khi so sánh với đầu ra của `expected_next_phase()` | Sử dụng so sánh tường minh `if Some(phase) != expected` để kiểm tra khớp kiểu `Option<T>` an toàn. |
| **E0658** | `use of unstable library feature` khi cố gắng sử dụng các API chưa ổn định của Rust 2024 | Lời gọi các phương thức thử nghiệm chưa được chuẩn hoá trong thư viện chuẩn `std` | Chỉ sử dụng các API ổn định (stable) của Rust 2024: các phương thức `saturating_sub`, `is_char_boundary`, `total_cmp`, và `BTreeMap`. |

## 6. Tóm tắt & Bài tập rèn luyện

### Tóm tắt cốt lõi
1. **Suy luận có cấu trúc**: Mô hình hoá bài toán thành cây trạng thái (Tree of Thoughts) kết hợp tìm kiếm BFS giúp giải quyết các bài toán có tính phi tuyến tính (như Game of 24) mà suy luận tham lam CoT chắc chắn thất bại.
2. **Kỹ nghệ Đồ thị kép**: InfoGraph cô lập ngữ cảnh qua cắt lát 1-hop lân cận giúp tiết kiệm > 60% token rác. PlanDag phân rã luồng thực thi thành các đợt sóng song song theo giải thuật Kahn, phát hiện chu trình bế tắc và bảo vệ hệ thống bằng ngân sách thử lại cục bộ.
3. **Tính tất định của Máy trạng thái**: Cổng kiểm soát Stripe Blueprints đảm bảo mọi thay đổi trạng thái đều có thể truy vết, kiểm toán bằng mã băm toàn vẹn, và hoàn tác an toàn khi gặp sự cố.
4. **Bảo vệ ranh giới can thiệp con người**: Ngắt duyệt HITL bắt buộc phải kiểm tra cả chiều xoá bỏ tham số (Key Deletion Diff) để ghi nhận các khoá bị con người loại trừ vào Trí nhớ thủ tục.
5. **Truy xuất lai và Kiểm chứng đa tầng**: Kết hợp từ khoá chính xác (BM25) và ngữ nghĩa sâu (Dense Cosine) qua RRF mang lại độ chính xác vượt trội. Kiểm chứng toạ độ lát cắt (Span Citation) triệt tiêu hoàn toàn ảo giác trích dẫn của LLM.

---

### Bài tập rèn luyện

#### Bài tập 1: Mở rộng Tree-of-Thoughts với Tập Trạng Thái Đã Duyệt (Memoization & Visited Set)
**Yêu cầu**: Trong bài toán Game of 24, nhiều nhánh suy luận khác nhau có thể dẫn đến cùng một tập số (ví dụ: $4 + 9 = 13$ và $9 + 4 = 13$). Hiện tại, thuật toán duyệt lại toàn bộ các trạng thái tương đương này gây lãng phí bộ nhớ. Hãy viết một cấu trúc `MemoizedThoughtTree` sử dụng một bảng băm `HashSet<Vec<u64>>` để lưu trữ chữ ký chuẩn hoá của các tập số đã duyệt (chuyển đổi số thực sang biểu diễn bit `f64::to_bits` và sắp xếp tăng dần). Nếu một trạng thái đã từng xuất hiện trong tập đã duyệt, hãy cắt tỉa nhánh đó ngay lập tức.

<details>
<summary>Gợi ý & Lời giải mẫu Bài tập 1</summary>

```rust
use std::collections::{HashSet, VecDeque};

#[derive(Debug, Clone, PartialEq)]
pub struct FastThoughtNode {
    pub numbers: Vec<f64>,
    pub expression: String,
}

pub struct MemoizedThoughtTree;

impl MemoizedThoughtTree {
    /// Chuẩn hoá tập số thành chữ ký nhị phân bất biến đối với thứ tự xuất hiện
    pub fn state_signature(nums: &[f64]) -> Vec<u64> {
        let mut bits: Vec<u64> = nums.iter().map(|x| (x * 1000.0).round() as i64 as u64).collect();
        bits.sort_unstable();
        bits
    }

    pub fn solve(numbers: &[f64], target: f64) -> Option<String> {
        let mut visited = HashSet::new();
        let mut queue = VecDeque::new();

        queue.push_back(FastThoughtNode {
            numbers: numbers.to_vec(),
            expression: numbers.iter().map(|n| format!("{:.0}", n)).collect::<Vec<_>>().join(", "),
        });

        visited.insert(Self::state_signature(numbers));

        while let Some(current) = queue.pop_front() {
            let n = current.numbers.len();
            if n == 1 {
                if (current.numbers[0] - target).abs() < 1e-4 {
                    return Some(current.expression);
                }
                continue;
            }

            for i in 0..n {
                for j in (i + 1)..n {
                    let a = current.numbers[i];
                    let b = current.numbers[j];

                    let mut rest = Vec::with_capacity(n - 1);
                    for (idx, &num) in current.numbers.iter().enumerate() {
                        if idx != i && idx != j {
                            rest.push(num);
                        }
                    }

                    let ops = vec![
                        (a + b, format!("({} + {})", a, b)),
                        (a - b, format!("({} - {})", a, b)),
                        (b - a, format!("({} - {})", b, a)),
                        (a * b, format!("({} * {})", a, b)),
                    ];

                    for (res, expr) in ops {
                        let mut next_nums = rest.clone();
                        next_nums.push(res);

                        let sig = Self::state_signature(&next_nums);
                        if visited.insert(sig) {
                            queue.push_back(FastThoughtNode {
                                numbers: next_nums,
                                expression: expr,
                            });
                        }
                    }
                }
            }
        }
        None
    }
}

#[test]
fn test_memoized_tree() {
    let sol = MemoizedThoughtTree::solve(&[4.0, 9.0, 10.0, 13.0], 24.0);
    assert!(sol.is_some());
}
```
</details>

---

#### Bài tập 2: Bộ Phá Bẻ Chu Trình Tự Động (Cycle Breaker) cho PlanDag
**Yêu cầu**: Trong các đồ thị kế hoạch phức tạp do tác tử tự động sinh ra, việc xuất hiện chu trình phụ thuộc là lỗi phổ biến. Thay vì chỉ báo lỗi và dừng hệ thống, hãy cài đặt cấu trúc `AutoHealingDag` với hàm `break_cycles(&mut self) -> Vec<(String, String)>`. Hàm này sẽ phát hiện các cạnh tạo chu trình (bằng thuật toán DFS tô màu 3 trạng thái tìm kiếm cạnh ngược - Back Edge) và tự động loại bỏ cạnh đó khỏi danh sách phụ thuộc để khôi phục đồ thị trở lại thành một DAG hợp lệ, đồng thời trả về danh sách các cạnh đã bị cắt bỏ để phục vụ ghi nhật ký cảnh báo.

<details>
<summary>Gợi ý & Lời giải mẫu Bài tập 2</summary>

```rust
use std::collections::BTreeMap;

#[derive(Debug, Clone, Default)]
pub struct AutoHealingDag {
    // node_id -> danh sách dependencies (cạnh đến từ các node khác)
    pub adj: BTreeMap<String, Vec<String>>,
}

impl AutoHealingDag {
    pub fn new() -> Self {
        Self { adj: BTreeMap::new() }
    }

    pub fn add_task(&mut self, id: &str, deps: &[&str]) {
        self.adj.insert(
            id.to_string(),
            deps.iter().map(|&d| d.to_string()).collect(),
        );
    }

    /// Tự động phát hiện và bẻ gãy các cạnh tạo chu trình bằng DFS 3 màu
    pub fn break_cycles(&mut self) -> Vec<(String, String)> {
        let mut broken_edges = Vec::new();
        // 0: Chưa duyệt (White), 1: Đang duyệt trên ngăn xếp đệ quy (Gray), 2: Đã hoàn tất (Black)
        let mut color: BTreeMap<String, u8> = BTreeMap::new();
        for k in self.adj.keys() {
            color.insert(k.clone(), 0);
        }

        let nodes: Vec<String> = self.adj.keys().cloned().collect();
        for node in nodes {
            if *color.get(&node).unwrap_or(&0) == 0 {
                Self::dfs_heal(&node, &mut self.adj, &mut color, &mut broken_edges);
            }
        }

        broken_edges
    }

    fn dfs_heal(
        u: &str,
        adj: &mut BTreeMap<String, Vec<String>>,
        color: &mut BTreeMap<String, u8>,
        broken: &mut Vec<(String, String)>,
    ) {
        color.insert(u.to_string(), 1); // Đang duyệt

        let deps = adj.get(u).cloned().unwrap_or_default();
        let mut safe_deps = Vec::new();

        for v in deps {
            let c = *color.get(&v).unwrap_or(&0);
            if c == 1 {
                // Phát hiện cạnh ngược u -> v tạo chu trình! Loại bỏ cạnh này.
                broken.push((u.to_string(), v.clone()));
            } else {
                if c == 0 {
                    Self::dfs_heal(&v, adj, color, broken);
                }
                safe_deps.push(v);
            }
        }

        adj.insert(u.to_string(), safe_deps);
        color.insert(u.to_string(), 2); // Hoàn tất
    }
}

#[test]
fn test_cycle_breaker() {
    let mut dag = AutoHealingDag::new();
    dag.add_task("A", &["B"]);
    dag.add_task("B", &["C"]);
    dag.add_task("C", &["A"]); // Chu trình A -> B -> C -> A

    let broken = dag.break_cycles();
    assert!(!broken.is_empty(), "Phải phát hiện và bẻ gãy ít nhất một cạnh chu trình");
    
    // Đồ thị sau khi sửa không còn chu trình
    let second_pass = dag.break_cycles();
    assert!(second_pass.is_empty(), "Đồ thị sau khi sửa phải hoàn toàn phi chu trình");
}
```
</details>

---

#### Bài tập 3: Bộ Truy Xuất Lai 3 Nhánh (BM25 + Dense + HyDE)
**Yêu cầu**: Hãy nâng cấp hệ thống `HybridRetriever` thành bộ truy xuất lai 3 nguồn (Tri-Hybrid):
1. Nhánh 1: Điểm từ khoá Okapi BM25 từ câu truy vấn gốc.
2. Nhánh 2: Điểm Dense Cosine giữa embedding câu truy vấn gốc và tài liệu.
3. Nhánh 3: Điểm Dense Cosine giữa embedding của một đoạn tài liệu giả lập (Hypothetical Document Embeddings - HyDE do LLM tự sinh) và tài liệu.
Tổng hợp cả 3 nguồn bằng công thức RRF 3 nhánh:
$$\text{RRF}(d) = \frac{1}{60 + \text{Rank}_{\text{BM25}}(d)} + \frac{1}{60 + \text{Rank}_{\text{Dense}}(d)} + \frac{1}{60 + \text{Rank}_{\text{HyDE}}(d)}$$

<details>
<summary>Gợi ý & Lời giải mẫu Bài tập 3</summary>

```rust
#[derive(Debug, Clone)]
pub struct TriHybridRetriever;

impl TriHybridRetriever {
    pub fn rank_rrf_3way(
        doc_count: usize,
        bm25_ranks: &[usize], // doc_id xếp theo thứ tự BM25
        dense_ranks: &[usize], // doc_id xếp theo thứ tự Dense
        hyde_ranks: &[usize],  // doc_id xếp theo thứ tự HyDE
        rrf_k: usize,
    ) -> Vec<(usize, f64)> {
        let mut fused_scores = vec![0.0; doc_count];

        for (rank, &id) in bm25_ranks.iter().enumerate() {
            if id < doc_count {
                fused_scores[id] += 1.0 / ((rrf_k + rank + 1) as f64);
            }
        }

        for (rank, &id) in dense_ranks.iter().enumerate() {
            if id < doc_count {
                fused_scores[id] += 1.0 / ((rrf_k + rank + 1) as f64);
            }
        }

        for (rank, &id) in hyde_ranks.iter().enumerate() {
            if id < doc_count {
                fused_scores[id] += 1.0 / ((rrf_k + rank + 1) as f64);
            }
        }

        let mut results: Vec<(usize, f64)> = fused_scores.into_iter().enumerate().collect();
        results.sort_by(|a, b| b.1.total_cmp(&a.1).then_with(|| a.0.cmp(&b.0)));
        results
    }
}

#[test]
fn test_tri_hybrid_rrf() {
    let bm25_ranks = vec![2, 0, 1];
    let dense_ranks = vec![1, 0, 2];
    let hyde_ranks = vec![0, 1, 2];

    let scores = TriHybridRetriever::rank_rrf_3way(3, &bm25_ranks, &dense_ranks, &hyde_ranks, 60);
    // Tài liệu số 0 đạt thứ hạng đều ở cả 3 nhánh (hạng nhì ở bm25, hạng nhì ở dense, quán quân ở hyde)
    assert_eq!(scores[0].0, 0, "Tài liệu 0 có phong độ đồng đều ở cả 3 nhánh phải xếp hạng nhất chung cuộc");
}
```
</details>

---

#### Bài tập 4: Trình Kiểm Soát Giới Hạn Tần Suất Token Phân Tán (Token Bucket Rate Limiter)
**Yêu cầu**: Trong các hệ sinh thái tác tử quy mô lớn, nhiều tác tử con (subagents) cùng truy cập vào cổng LLM Gateway dùng chung. Hãy xây dựng cấu trúc `AgentTokenBucket` với thuật toán Token Bucket chuẩn:
- Dung lượng xô cực đại `capacity: f64` (số token tối đa được phép bùng nổ).
- Tốc độ nạp lại `refill_rate_per_sec: f64` (số token được nạp vào xô mỗi giây).
- Phương thức `try_acquire(&mut self, requested_tokens: f64, current_timestamp_secs: f64) -> bool`.
Đảm bảo thuật toán hoạt động hoàn toàn trên đồng hồ ảo hoặc timestamp logic, không gây race condition và có kiểm thử đơn vị chứng minh tính chính xác toán học.

<details>
<summary>Gợi ý & Lời giải mẫu Bài tập 4</summary>

```rust
#[derive(Debug, Clone)]
pub struct AgentTokenBucket {
    pub capacity: f64,
    pub current_tokens: f64,
    pub refill_rate_per_sec: f64,
    pub last_refill_timestamp: f64,
}

impl AgentTokenBucket {
    pub fn new(capacity: f64, refill_rate_per_sec: f64, start_time: f64) -> Self {
        Self {
            capacity,
            current_tokens: capacity,
            refill_rate_per_sec,
            last_refill_timestamp: start_time,
        }
    }

    /// Thử lấy số lượng token yêu cầu tại thời điểm logic chỉ định
    pub fn try_acquire(&mut self, tokens: f64, now: f64) -> bool {
        if now < self.last_refill_timestamp {
            return false; // Chặn lùi thời gian
        }

        // 1. Nạp token tích luỹ theo khoảng thời gian đã trôi qua
        let elapsed = now - self.last_refill_timestamp;
        let refilled = elapsed * self.refill_rate_per_sec;
        self.current_tokens = (self.current_tokens + refilled).min(self.capacity);
        self.last_refill_timestamp = now;

        // 2. Kiểm tra khả năng cấp phát
        if self.current_tokens >= tokens {
            self.current_tokens -= tokens;
            true
        } else {
            false
        }
    }
}

#[test]
fn test_token_bucket_limiter() {
    let mut bucket = AgentTokenBucket::new(100.0, 10.0, 0.0);

    // Lấy hết 100 token ban đầu
    assert!(bucket.try_acquire(100.0, 0.0));
    // Hết token => từ chối
    assert!(!bucket.try_acquire(1.0, 0.0));

    // Sau 5 giây, xô được nạp thêm 5 * 10 = 50 token
    assert!(bucket.try_acquire(50.0, 5.0));
    // Đòi thêm 1 token nữa => từ chối
    assert!(!bucket.try_acquire(1.0, 5.0));
}
```
</details>
---

### Lời kết & Hướng tới Chương 89
Chương 88 đã trang bị cho bạn toàn bộ bộ khung nhận thức và cấu trúc điều phối vĩ mô: từ suy luận cây ý nghĩ ToT giải quyết bế tắc tham lam của CoT, phân rã đợt sóng song song qua đồ thị PlanDag của Kahn, đến điểm kiểm soát tất định Stripe Blueprints, ngắt duyệt HITL phát hiện xoá khoá tham số, và hệ thống truy xuất lai BM25 + Dense + RRF chuẩn xác.

Những mảnh ghép kiến trúc này chính là nền móng để bước sang **Chương 89: Độ bền Phục hồi Sự cố — Cổng Mô hình, Nhật ký WAL, Cây Phiên, Outbox, Saga & Giao thức MCP + A2A + ANP**, nơi chúng ta sẽ biến các đồ thị và máy trạng thái này thành các dịch vụ bền vững, miễn nhiễm với sự cố sập nguồn và sẵn sàng giao tiếp liên tác tử qua các giao thức mở.
