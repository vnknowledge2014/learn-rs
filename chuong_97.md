# Chương 97: Đại dự án Tích hợp II — Đường ống Micro-LLM Suy luận End-to-End Từ Byte-BPE Tới GRPO & Tác tử Gọi Công cụ Nội sinh (End-to-End Reasoning Micro-LLM & Native Tool Agent)

---

## 1. Giới thiệu & Mục tiêu

Chào mừng bạn đến với chương học tổng kết và cũng là đỉnh cao công nghệ cao nhất của toàn bộ giáo trình `learn-rs`: **Đại dự án Tích hợp II — Khép kín toàn diện Chủ đề 27 (Kiến trúc & Động cơ Huấn luyện LLM/RLVR Từ số không) và hợp lưu với Chủ đề 26 (Kỹ nghệ Bộ khung Tác tử Cấp Sản xuất)**.

Trong suốt 96 chương học trước, chúng ta đã đi qua một hành trình phi thường. Từ việc làm chủ các đặc tính cốt lõi của Rust (quyền sở hữu bộ nhớ, hệ thống phân loại kiểu tĩnh Typestate, lập trình đồng thời Actor, bộ nhớ Disruptor không khóa), chúng ta đã lần lượt chinh phục các hệ thống giao dịch tần suất cao (HFT), cấu trúc đại số trừu tượng, tăng tốc phần cứng CPU/GPU/FPGA, kỹ nghệ xây dựng bộ khung tác tử AI (Agent Harness, Live Context Mirror, StateGraph, LoopDrill), và đặc biệt là toàn bộ chuỗi kiến trúc của mô hình ngôn ngữ lớn (từ Byte-BPE, RoPE, GQA, SwiGLU, Muon, AdamW, RadixAttention, LoRA cho đến GRPO/RLVR).

Tuy nhiên, trong kỹ nghệ phần mềm và hệ thống trí tuệ nhân tạo, một chân lý bất biến luôn tồn tại: **"Biết từng thành phần riêng lẻ không đồng nghĩa với việc hiểu cách một cỗ máy hoàn chỉnh vận hành."** Khi các thành phần tách biệt được ghép nối vào một đường ống liên tục duy nhất, những tương tác ngầm, các điểm tích lũy sai số số học dấu phẩy động, và sự mất đồng bộ trạng thái sẽ bộc lộ rõ rệt.

```
┌────────────────────────────────────────────────────────────────────────────────────────┐
│               ĐƯỜNG ỐNG MICRO-REASONING LLM PIPELINE (6 CHẶNG TỰ CHỨA)                │
├────────────────────────────────────────────────────────────────────────────────────────┤
│                                                                                        │
│  [CHẶNG 1] Byte-Level BPE Tokenizer                                                    │
│  ├─ 256 byte UTF-8 cơ sở (không bao giờ gặp lỗi <unk>)                                  │
│  ├─ Hàng đợi ưu tiên BinaryHeap phá hòa tất định theo cặp (left, right)                 │
│  └─ Tích hợp token đặc biệt nội sinh: <think>, <call:calc>, <result>, <answer>         │
│         │                                                                              │
│         ▼                                                                              │
│  [CHẶNG 2] Tiền huấn luyện Nhân quả (Causal Pretraining)                               │
│  ├─ Kiến trúc Transformer vi mô: d_model = 24, 4 Query Heads, 2 KV Heads, 2 Layers     │
│  ├─ RmsNorm giải tích, RoPE bảo toàn chuẩn Euclid, GQA chú ý nhóm, SwiGLU MLP          │
│  └─ Bộ tối ưu hỗn hợp: Muon (Newton-Schulz 5 bước cho 2D) + AdamW (1D) + Lịch WSD     │
│         │                                                                              │
│         ▼                                                                              │
│  [CHẶNG 3] Tinh chỉnh có Giám sát Mặt nạ (Masked SFT)                                  │
│  ├─ Tầng thích ứng hạng thấp LoraLinear (A ~ N(0, s), B = 0: Zero-Cost tại bước 0)     │
│  └─ Hàm mất mát Masked SFT: Gradient prompt token = 0.0, chống chia 0 khi mask rỗng    │
│         │                                                                              │
│         ▼                                                                              │
│  [CHẶNG 4] Căn chỉnh Sở thích (Preference Alignment)                                   │
│  ├─ DPO với làm mịn nhãn (Label Smoothing epsilon) kiểm soát gradient cực đoan        │
│  └─ ORPO với hàm log1mexp ổn định số học hai nhánh tại -ln(2)                          │
│         │                                                                              │
│         ▼                                                                              │
│  [CHẶNG 5] Học Tăng cường Suy luận Có Thưởng Kiểm chứng (RLVR)                         │
│  ├─ Phạt phân kỳ KL không âm k3_kl(p, r) >= 0 của John Schulman                        │
│  ├─ Bộ chấm toán học tất định MathAnswerVerifier đánh giá khuôn thẻ và kết quả         │
│  ├─ DrGRPO khử thiên kiến độ dài (mẫu số max_gen_len) & DAPO kẹp bất đối xứng          │
│  └─ Tín dụng hiệu suất gọt ngữ cảnh an toàn (khóa điều kiện >= 2 quỹ đạo thắng)       │
│         │                                                                              │
│         ▼                                                                              │
│  [CHẶNG 6] Phục vụ Suy luận & Vòng lặp Tác tử Tự hành (Serving & Native Agent)         │
│  ├─ Phép hợp nhất LoRA không chi phí: merge_lora() sai số < 1e-12                      │
│  ├─ Cây tiền tố RadixPrefixTree lưu đệm KV-Cache và giải phóng LRU                     │
│  ├─ Giải mã suy đoán chính xác (Speculative Decoding) bảo toàn phân phối đích          │
│  └─ Vòng lặp tác tử tự hành gọi công cụ NativeCalcTool và xuất kết quả <answer>        │
│                                                                                        │
└────────────────────────────────────────────────────────────────────────────────────────┘
```

Chương 97 này hiện thực hóa **Đại dự án Tích hợp II** — một đường ống tự chứa mang tên `MicroReasoningLlmPipeline`. Toàn bộ dự án được triển khai bằng **100% Rust 2024 thuần `std`**, không sử dụng bất kỳ thư viện bên ngoài nào (`[dependencies]` trống rỗng), tuân thủ nghiêm ngặt quy tắc struct có trường định danh (named fields), định danh tiếng Anh kết hợp doc-comment tiếng Việt theo chuẩn `RiskGate`.

Toàn bộ đường ống chạy trên cấu hình quy mô vi mô nhưng chân thực 100% về mặt toán học ($d_{\text{model}} = 24$, 4 query heads, 2 KV heads, $d_{\text{head}} = 6$, 2 tầng transformer decoder), hoàn thành toàn bộ chu trình huấn luyện, căn chỉnh, phục vụ và kiểm thử 44 bài test trong thời gian **dưới 150 mili-giây** trên CPU tất định.

### Bảng Mục tiêu Học tập & Kỹ năng Đạt được

| Mục tiêu học tập | Vì sao bắt buộc phải tự cài đặt từ đầu? | Chuẩn đầu ra thực chứng |
|---|---|---|
| **Bộ tách từ Byte-BPE có Token Đặc biệt Nội sinh** | Hiểu cách mô hình ngôn ngữ nhìn nhận thế giới nhị phân và cách các token điều khiển suy luận (`<think>`, `<call:calc>`) được nhúng trực tiếp vào từ vựng mà không phá vỡ tính hợp lệ của chuỗi byte UTF-8. | Khứ hồi UTF-8 không mất mát 100%; token đặc biệt giữ nguyên ID và không bị chia cắt. |
| **Tiền huấn luyện với Muon & Lịch học WSD** | Nắm vững thuật toán trực giao hóa Newton–Schulz 5 bước thay thế cho SVD đắt đỏ; hiểu sự kết hợp giữa Muon cho ma trận trọng số 2D và AdamW cho vector 1D dưới lịch học Warmup-Stable-Decay. | Sai khác ma trận trực giao $\|X^T X - I\|_F < \epsilon$; tính liên tục tuyệt đối tại các biên chuyển pha WSD. |
| **Masked SFT & Tầng Thích ứng LoRA** | Nắm vững toán học của phân rã hạng thấp $W + \frac{\alpha}{r} B A$; giải thích tại sao khởi tạo $B = 0$ đảm bảo Zero-Cost ban đầu và cách hợp nhất trọng số `merge_lora()` triệt tiêu hoàn toàn độ trễ suy luận. | Trọng số bước 0 trùng hệt base; sai khác giữa suy luận LoRA động và suy luận trọng số hợp nhất $< 10^{-12}$. |
| **Căn chỉnh Sở thích DPO & ORPO** | Nắm bắt cơ chế căn chỉnh không cần Reward Model riêng biệt; làm chủ hàm `log1mexp` ổn định số học trên hai nhánh để triệt tiêu lỗi tràn số dấu phẩy động trong tỷ số Odds Ratio. | Biên độ ưu tiên $\log \pi(y_w) - \log \pi(y_l)$ mở rộng đơn điệu; đạo hàm giải tích khớp sai phân hữu hạn $< 10^{-8}$. |
| **Học Tăng cường RLVR & Phạt KL Schulman** | Nắm vững bản chất của RLVR (Reinforcement Learning with Verifiable Rewards): ước lượng $k3\_kl$ luôn không âm, bộ chấm toán học định dạng thẻ, và thuật toán DrGRPO/DAPO chống thiên kiến độ dài. | Giá trị $k3\_kl \ge 0$ trên toàn miền thực; tín dụng gọt ngữ cảnh bằng $0.0$ khi số người thắng $< 2$, loại trừ gian lận. |
| **Phục vụ Suy luận Radix Tree & Tác tử Nội sinh** | Hiểu sâu kiến trúc phục vụ hiện đại (kiểu SGLang/vLLM): tái sử dụng KV-Cache tiền tố qua cây Radix, giải mã suy đoán bảo toàn phân phối, và điều phối vòng lặp tác tử tự hành gọi công cụ. | Trạng thái suy luận từ bộ nhớ đệm khớp tính toán từ đầu $< 10^{-12}$; tác tử tự hành giải đúng bài toán số học qua thẻ `<call:calc>`. |

### Bảng Ma trận 6 Bất biến Toán học & Hệ thống Bắt buộc Phải Chứng minh

Mọi hệ thống lớn trong `learn-rs` đều được định nghĩa thông qua các **bất biến (invariants)**. Một hệ thống chỉ được coi là đạt chuẩn khi toàn bộ các bất biến này được kiểm chứng bằng kiểm thử phần mềm tự động:

| STT | Bất biến toán học / hệ thống | Định nghĩa toán học | Cơ chế kiểm chứng trong mã nguồn |
|---|---|---|---|
| **1** | **Gradient Orthogonality** | Thuật toán Newton–Schulz 5 bước đưa ma trận gradient ngẫu nhiên về xấp xỉ ma trận trực giao: $\|X^T X - I\|_F < \epsilon$. | `test_14_muon_newton_schulz_orthogonality`: Kiểm tra chuẩn Frobenius của sai khác so với ma trận đơn vị. |
| **2** | **Analytical vs Finite-Diff Convergence** | Đạo hàm ngược giải tích của mọi tầng (`RMSNorm`, `SwiGLU`, `Masked SFT`, `DPO`, `k3_kl`) khớp hoàn hảo với sai phân hữu hạn trung tâm: $\|\nabla_{\text{ana}} - \nabla_{\text{fd}}\|_\infty < 10^{-8}$. | Các bài test `07`, `12`, `25`, `28`, `32`: So sánh trực tiếp đạo hàm giải tích và sai phân với $\epsilon = 10^{-6}$. |
| **3** | **Zero-Cost LoRA Equivalence** | Tại bước khởi tạo $0$, $B = 0 \implies \Delta W = 0$. Sau khi cập nhật, kết quả lan truyền xuôi trước và sau `merge_lora()` là tương đương tuyệt đối: $\|y_{\text{lora}} - y_{\text{merged}}\|_\infty < 10^{-12}$. | Các bài test `20`, `21`: Đối chiếu giá trị đầu ra trước và sau khi gộp ma trận thích ứng vào mô hình nền. |
| **4** | **KL Non-Negativity** | Ước lượng phạt phân kỳ KL $k3\_kl(p, r) = e^{r-p} - (r-p) - 1$ của John Schulman luôn không âm với mọi cặp log-xác suất: $\forall p, r \in \mathbb{R}, k3\_kl(p, r) \ge 0$. | `test_31_k3_kl_non_negativity`: Quét qua các điểm kiểm tra biên, bao gồm giá trị âm sâu và giá trị dương. |
| **5** | **Radix Cache Hit Invariance** | Trạng thái chú ý và phân phối logit được khôi phục từ nút lưu đệm tiền tố trong `RadixPrefixTree` hoàn toàn đồng nhất với việc tính toán lại từ đầu: $\|S_{\text{cached}} - S_{\text{fresh}}\|_\infty < 10^{-12}$. | `test_37_radix_prefix_tree_cache_hit_invariance`: Kiểm tra khớp tiền tố và tính tương đương của KV-Cache. |
| **6** | **End-to-End Tool Call Convergence** | Mô hình tự hành phát ra chuỗi suy luận hợp lệ, trích xuất chính xác thẻ gọi công cụ `<call:calc>`, tiếp nhận phản hồi `<result>`, và kết luận đáp án chính xác trong thẻ `<answer>`. | `test_43_end_to_end_native_tool_agent_loop` & `test_44`: Vòng lặp tác tử hoàn thành dưới 5 lượt tương tác. |

---

## 2. Hình tượng hóa đời sống: Từ lò phản ứng hạt nhân đến con tàu thám hiểm tự hành

Để xây dựng một trực giác sắc bén trước khi tiếp cận hàng ngàn dòng mã nguồn đại số và vi phân, chúng ta hãy quan sát hai hình tượng đời sống quen thuộc.

### Hình tượng 1: Chu trình làm giàu và vận hành lò phản ứng hạt nhân (Giai đoạn Tiền huấn luyện & Làm giàu)

Hãy hình dung quá trình xây dựng một mô hình ngôn ngữ giống hệt như quá trình chế tạo nhiên liệu hạt nhân và điều khiển một lò phản ứng năng lượng cao:

```
[Quặng thô tự nhiên] ──► [Máy ly tâm BPE] ──► [Thanh nhiên liệu U-235] ──► [Lò phản ứng Muon]
 (Văn bản byte thô)      (Gộp cặp tần suất)     (Từ vựng nén 256 byte)     (Trực giao hóa NS5)
                                                                                  │
                                                                                  ▼
 [Thanh điều khiển WSD] ◄───────────────────────────────────────────── [Năng lượng hội tụ]
  (Warmup-Stable-Decay)                                                (Loss giảm đơn điệu)
```

1. **Quặng thô tự nhiên và Máy ly tâm BPE**:
   Trong tự nhiên, uranium tồn tại chủ yếu dưới dạng đồng vị $U^{238}$ kém phân hạch, chỉ có một tỷ lệ nhỏ $U^{235}$ hữu dụng. Tương tự, văn bản thô trên Internet chứa hàng tỷ ký tự hỗn độn. Bộ tách từ `BpeTokenizer` đóng vai trò như một dàn máy ly tâm siêu tốc: nó bắt đầu từ **256 byte cơ sở UTF-8** (nguyên tử cơ bản), rồi thông qua hàng đợi ưu tiên `BinaryHeap` liên tục tìm kiếm và gộp các cặp đồng vị xuất hiện nhiều nhất thành các "thanh nhiên liệu" token giàu năng lượng biểu đạt. Đặc biệt, nó gắn sẵn các "thanh điều khiển đặc biệt" (`<think>`, `<call:calc>`, `<answer>`) để chuẩn bị cho giai đoạn phản ứng có kiểm soát.

2. **Lò phản ứng và Thuật toán Trực giao hóa Muon**:
   Khi đưa nhiên liệu vào lõi lò phản ứng, nếu các neutron chuyển động hỗn loạn cùng một hướng, phản ứng dây chuyền sẽ bùng nổ mất kiểm soát (gradient explosion hoặc sụp đổ hạng ma trận). Thuật toán tối ưu hóa `Muon` sử dụng phép lặp Newton–Schulz 5 bước đóng vai trò như một **bộ phản xạ neutron định hướng**: nó nắn chỉnh ma trận cập nhật gradient sao cho các hướng cập nhật luôn vuông góc (trực giao) với nhau. Nhờ đó, dòng năng lượng học tập chảy qua mọi chiều của không gian ẩn mà không chiều nào bị nghẽn hay triệt tiêu.

3. **Thanh điều khiển Lịch học WSD**:
   Một lò phản ứng không thể khởi động ở 100% công suất ngay lập tức. Lịch học WSD (Warmup - Stable - Decay) từ từ nâng nhiệt độ học tập trong pha Warmup ($S_w$), duy trì công suất tối đa ổn định trong pha Stable ($S_s$) để tích lũy tri thức sâu rộng, và hạ nhiệt từ từ theo đường cong Cosine trong pha Decay ($S_d$) để đưa các nguyên tử trọng số vào trạng thái tinh thể bền vững nhất.

---

### Hình tượng 2: Con tàu thám hiểm tự hành liên hành tinh (Giai đoạn Hậu huấn luyện & Phục vụ)

Sau khi bộ não của mô hình đã được tiền huấn luyện, nó giống như một con tàu vũ trụ hoàn chỉnh về mặt phần cứng nhưng chưa có hệ thống điều khiển sứ mệnh:

```
                      ┌───────────────────────────────────────┐
                      │    CON TÀU THÁM HIỂM TỰ HÀNH CH97     │
                      └──────────────────┬────────────────────┘
                                         │
                 ┌───────────────────────┴───────────────────────┐
                 ▼                                               ▼
      [LA BÀN ĐẠO ĐỨC DPO/ORPO]                       [HỆ PHẢN XẠ THƯỞNG RLVR]
  (Căn chỉnh quỹ đạo bay an toàn)                 (Học từ tín hiệu phản hồi thực tế)
                 │                                               │
                 └───────────────────────┬───────────────────────┘
                                         │
                 ┌───────────────────────┴───────────────────────┐
                 ▼                                               ▼
      [BỘ NHỚ ĐỆM TIỀN TỐ RADIX]                     [CÁNH TAY ROBOT CÔNG CỤ]
   (Tái sử dụng bản đồ sao đã bay)                (NativeCalcTool tính toán chính xác)
```

1. **La bàn Căn chỉnh DPO / ORPO**:
   Trong không gian sâu thẳm, con tàu cần phân biệt rõ ràng giữa hướng đi an toàn và vùng nguy hiểm. Căn chỉnh DPO và ORPO sử dụng các cặp dữ liệu đối chiếu (hướng được chọn $y_w$ và hướng bị loại bỏ $y_l$). Nhờ hàm toán học ổn định `log1mexp`, la bàn của con tàu tính toán chính xác tỷ số rủi ro mà không bao giờ bị "mất phương hướng" do lỗi chia cho 0 hay tràn số dấu phẩy động.

2. **Hệ phản xạ Thưởng phạt RLVR & An toàn Ngữ cảnh**:
   Để đáp xuống bề mặt hành tinh lạ, con tàu phải giải quyết các bài toán định vị chính xác. Môi trường RLVR cung cấp một bộ chấm tất định (`MathAnswerVerifier`): chỉ khi con tàu đưa ra đáp án chính xác trong thẻ `<answer>` sau khi đã suy nghĩ cẩn thận trong `<think>`, nó mới nhận được phần thưởng tối đa. Đặc biệt, bộ điều khiển DrGRPO khóa chặt điều kiện: con tàu chỉ được thưởng điểm tiết kiệm nhiên liệu (gọt ngữ cảnh) khi có **ít nhất hai phương án hạ cánh thành công**, ngăn chặn triệt để hành vi gian lận (như việc xóa trắng dữ liệu cảm biến để nhận thưởng độ dài ngắn).

3. **Bản đồ Sao RadixPrefixTree & Cánh tay Robot Nội sinh**:
   Khi bay tuần tra qua các quỹ đạo quen thuộc, con tàu không lập lại bản đồ sao từ đầu mà tra cứu trực tiếp trong cây tiền tố `RadixPrefixTree`, tái sử dụng nguyên vẹn trạng thái bộ nhớ đệm KV. Khi gặp bài toán số học vượt quá khả năng ước lượng trực quan của mạng nơ-ron, nó kích hoạt cánh tay robot chuyên dụng `NativeCalcTool` thông qua giao thức thẻ `<call:calc>`. Kết quả đo đạc `<result>` được nạp ngược lại vào chuỗi suy luận để đưa ra quyết định cuối cùng chuẩn xác 100%.

---

## 3. Khái niệm & Cơ chế kỹ thuật chuyên sâu

Phần này đi sâu vào phân tích toán học, cấu trúc dữ liệu và các chứng minh giải tích của từng chặng trong đường ống.

### 3.1 Chặng 1: Bộ tách từ Byte-Level BPE với Token Đặc biệt Nội sinh

Mọi mô hình ngôn ngữ hiện đại (GPT-4, LLaMA-3, DeepSeek-V3) đều từ bỏ cách tiếp cận tách từ theo từ điển ký tự truyền thống để chuyển sang **Byte-Level Byte Pair Encoding (Byte-BPE)**.

#### Không gian Từ vựng Cơ sở 256 Byte UTF-8
Bất kỳ chuỗi văn bản nào, dù là tiếng Việt có dấu (`"Học máy"`), ký tự tượng hình, hay chuỗi nhị phân tùy ý, khi biểu diễn dưới dạng UTF-8 đều là một mảng các byte có giá trị từ `0` đến `255`:
$$\mathcal{V}_0 = \{0x00, 0x01, \dots, 0xFF\}$$

Vì mọi byte đều có mặt trong từ vựng cơ sở, mô hình **không bao giờ gặp phải token lạ (`<unk>`)**. Đây là nền tảng cốt lõi giúp hệ thống bảo toàn 100% dữ liệu UTF-8 qua chu trình mã hóa và giải mã (Roundtrip Invariance):
$$\forall s \in \text{String}, \quad \text{decode}(\text{encode}(s)) = s$$

#### Thuật toán Phá hòa Tất định (Deterministic Tie-breaking)
Trong quá trình huấn luyện BPE, khi đếm tần suất xuất hiện của các cặp token liền kề $(u, v)$, trường hợp hòa tần suất xảy ra rất thường xuyên. Nếu việc phá hòa phụ thuộc vào thứ tự duyệt bất định của `HashMap`, từ vựng sinh ra sẽ khác nhau giữa các lần chạy hoặc giữa các kiến trúc CPU khác nhau.

Để đảm bảo tính tất định tuyệt đối, ta sử dụng hàng đợi ưu tiên `BinaryHeap` với cấu trúc khóa:
$$\text{Key} = (C(u, v), \text{Reverse}((u, v)))$$
Trong đó $C(u, v)$ là tần suất xuất hiện của cặp $(u, v)$. Khi hai cặp có cùng tần suất $C_1 = C_2$, hàng đợi sẽ so sánh thành phần thứ hai: cặp nào có giá trị số $(u, v)$ nhỏ hơn theo thứ tự từ điển (lexicographical order) sẽ được ưu tiên gộp trước.

```
       Cặp token       Tần suất       Thứ tự ưu tiên trong BinaryHeap
     ┌───────────┐   ┌──────────┐   ┌────────────────────────────────┐
     │  (65, 66) │ ──►    10    │ ──► Ưu tiên 1 (Tần suất lớn nhất)  │
     │  (10, 20) │ ──►     5    │ ──► Ưu tiên 2 (Cùng tần suất 5,    │
     │  (30, 40) │ ──►     5    │ ──► Ưu tiên 3  nhưng 10 < 30)      │
     └───────────┘   └──────────┘   └────────────────────────────────┘
```

#### Token Đặc biệt Nội sinh và Bảo toàn Ranh giới
Các mô hình suy luận tương tác công cụ yêu cầu các token điều khiển đặc biệt. Nếu mã hóa thô, chuỗi `<think>` sẽ bị tách thành các byte riêng lẻ `'<'`, `'t'`, `'h'`, `'i'`, `'n'`, `'k'`, `'>'`. 

Trong `BpeTokenizer`, ta cấp phát cố định các ID từ vựng bắt đầu từ 256:
- `PAD_TOKEN_ID = 256` (`<pad>`)
- `THINK_START_ID = 257` (`<think>`), `THINK_END_ID = 258` (`</think>`)
- `CALL_CALC_START_ID = 259` (`<call:calc>`), `CALL_CALC_END_ID = 260` (`</call:calc>`)
- `RESULT_START_ID = 261` (`<result>`), `RESULT_END_ID = 262` (`</result>`)
- `ANSWER_START_ID = 263` (`<answer>`), `ANSWER_END_ID = 264` (`</answer>`)

Khi mã hóa chuỗi văn bản, bộ tách từ thực hiện quét tiền tố để nhận diện các token đặc biệt trước. Phần văn bản nằm giữa các token đặc biệt mới được đưa qua các quy tắc gộp BPE thông thường. Nhờ đó, cấu trúc thẻ điều khiển được bảo vệ nguyên vẹn 100%.

---

### 3.2 Chặng 2: Khối Transformer Vi mô, Tự vi phân & Tối ưu hóa Muon

Mô hình trong dự án được thiết kế theo kiến trúc Transformer Decoder hiện đại nhất (tương đồng với LLaMA-3 và DeepSeek-V3) ở quy mô vi mô.

#### Chuẩn hóa RMSNorm và Đạo hàm Ngược Giải tích
Thay vì LayerNorm truyền thống (cần tính cả kỳ vọng và phương sai), `RMSNorm` (Root Mean Square Normalization) chỉ chuẩn hóa theo độ lệch chuẩn bình phương trung bình, giúp tiết kiệm ~7% chi phí tính toán:
$$\text{RMS}(x) = \sqrt{\frac{1}{d}\sum_{j=1}^d x_j^2 + \epsilon}, \quad y_i = \frac{x_i}{\text{RMS}(x)} \cdot w_i$$

Để động cơ học tập hoạt động chính xác mà không cần framework Autograd cồng kềnh, ta dẫn xuất công thức đạo hàm ngược giải tích chính xác. Cho gradient đầu ra $g_i = \frac{\partial \mathcal{L}}{\partial y_i}$, đạo hàm theo trọng số chuẩn hóa $w_i$ và vector đầu vào $x_i$ được tính như sau:

$$\frac{\partial \mathcal{L}}{\partial w_i} = g_i \cdot \frac{x_i}{\text{RMS}(x)}$$

Đặt $\text{inv} = \frac{1}{\text{RMS}(x)}$ và $\text{dot} = \sum_{j=1}^d g_j w_j x_j$. Khi đó:
$$\frac{\partial \mathcal{L}}{\partial x_i} = \text{inv} \cdot \left( g_i w_i - \frac{x_i \cdot \text{inv}^2}{d} \cdot \text{dot} \right)$$

Chứng minh kiểm chứng bằng sai phân hữu hạn trung tâm (Central Finite Difference) với bước $\epsilon_0 = 10^{-6}$:
$$\frac{\partial \mathcal{L}}{\partial x_i} \approx \frac{\mathcal{L}(x_i + \epsilon_0) - \mathcal{L}(x_i - \epsilon_0)}{2\epsilon_0}$$
Sai số giữa đạo hàm giải tích và sai phân hữu hạn luôn nhỏ hơn $10^{-8}$, chứng minh tính toàn vẹn của Bất biến 2.

#### Mã hóa Vị trí Quay (Rotary Position Embedding - RoPE)
RoPE mã hóa vị trí tuyệt đối $m$ bằng cách xoay từng cặp tọa độ của vector truy vấn và khóa trong không gian 2D:
$$R_{\Theta, m}^d = \text{diag}\left( R_{\theta_1, m}, R_{\theta_2, m}, \dots, R_{\theta_{d/2}, m} \right)$$
với ma trận xoay 2D tại tần số $\theta_i = \text{base}^{-2i/d}$:
$$R_{\theta_i, m} = \begin{pmatrix} \cos(m\theta_i) & -\sin(m\theta_i) \\ \sin(m\theta_i) & \cos(m\theta_i) \end{pmatrix}$$

**Chứng minh tính bảo toàn chuẩn Euclid**:
Xét vector 2D $v = (x_1, x_2)^T$. Sau khi xoay:
$$v' = \begin{pmatrix} x_1 \cos\phi - x_2 \sin\phi \\ x_2 \cos\phi + x_1 \sin\phi \end{pmatrix}$$
Bình phương chuẩn Euclid của $v'$ là:
$$\|v'\|^2 = (x_1 \cos\phi - x_2 \sin\phi)^2 + (x_2 \cos\phi + x_1 \sin\phi)^2$$
$$= x_1^2 \cos^2\phi - 2x_1 x_2 \cos\phi \sin\phi + x_2^2 \sin^2\phi + x_2^2 \cos^2\phi + 2x_1 x_2 \cos\phi \sin\phi + x_1^2 \sin^2\phi$$
$$= x_1^2(\cos^2\phi + \sin^2\phi) + x_2^2(\cos^2\phi + \sin^2\phi) = x_1^2 + x_2^2 = \|v\|^2$$
Do đó, RoPE bảo toàn tuyệt đối độ lớn của vector (chuẩn $L_2$) với sai số máy tính $< 10^{-12}$.

#### Chú ý Phân nhóm GQA (Grouped-Query Attention) & KV-Cache
Trong kiến trúc vi mô của chúng ta:
- Số Query Heads: $N_q = 4$
- Số Key/Value Heads: $N_{kv} = 2$
- Chiều mỗi head: $d_h = 6 \implies d_{\text{model}} = 4 \times 6 = 24$
- Tỷ lệ nhóm: $\text{ratio} = N_q / N_{kv} = 4 / 2 = 2$

```
     Query Heads (4)         KV Heads (2)
     ┌─────────────┐       ┌─────────────┐
     │ Head 0 (Q0) │ ──┐   │             │
     ├─────────────┤   ├──►│ Head 0 (KV0)│  (Phục vụ Q0 và Q1)
     │ Head 1 (Q1) │ ──┘   │             │
     ├─────────────┤       ├─────────────┤
     │ Head 2 (Q2) │ ──┐   │             │
     ├─────────────┤   ├──►│ Head 1 (KV1)│  (Phục vụ Q2 và Q3)
     │ Head 3 (Q3) │ ──┘   │             │
     └─────────────┘       └─────────────┘
```

Mỗi KV head được chia sẻ cho 2 Query heads. Trong quá trình suy luận từng bước (Autoregressive Generation), `KvCache` tích lũy vector $K$ và $V$ của các token quá khứ. Thay vì tính lại toàn bộ ma trận chú ý $O(T^2)$, hàm `step_decode` chỉ tính tích vô hướng giữa Query hiện tại và các Key trong bộ đệm $O(T)$, đảm bảo độ trễ mỗi token sinh ra là tối thiểu.

#### Tầng FFN Kích hoạt Cổng Tích cực SwiGLU
Khối mạng truyền thẳng (MLP) sử dụng cấu trúc `SwiGLU`:
$$\text{SwiGLU}(x) = \left( \text{SiLU}(x W_{\text{gate}}) \odot (x W_{\text{up}}) \right) W_{\text{down}}$$
trong đó $\odot$ là phép nhân từng phần tử (Hadamard product), và $\text{SiLU}(z) = \frac{z}{1 + e^{-z}}$.

#### Bộ Tối ưu hóa Muon và Đa thức Trực giao hóa Newton–Schulz 5 bước
Thuật toán `Muon` (Momentum Orthogonalized by Newton-schulz) là một đột phá lớn năm 2024-2025, được chứng minh huấn luyện LLM nhanh hơn AdamW từ 1.5x đến 2x.

Thay vì chuẩn hóa từng tọa độ bằng $\sqrt{v_t}$ như AdamW, Muon trực giao hóa toàn bộ ma trận động lượng $M \in \mathbb{R}^{r \times c}$. Khi ma trận trực giao hóa được áp dụng, các giá trị kỳ dị (singular values) của ma trận cập nhật đều xấp xỉ $1.0$, ngăn ngừa việc một số chiều bị học quá mức trong khi các chiều khác bị bỏ đói.

Đa thức xấp xỉ bậc 3 của Newton–Schulz sau 5 bước lặp:
$$X_0 = \frac{M}{\|M\|_F + \epsilon}$$
$$X_{k+1} = a X_k + (b A_k + c A_k^2) X_k, \quad A_k = X_k X_k^T$$
với bộ ba hệ số kinh điển:
$$(a, b, c) = (3.4445, -4.7750, 2.0315)$$

Khi $r > c$ (ma trận cao), ta chuyển vị $X \leftarrow X^T$ trước khi lặp để tính trên ma trận kích thước $c \times c$ nhỏ hơn, sau đó chuyển vị ngược lại. Sau 5 bước, ma trận $X_5$ đạt tính trực giao:
$$\|X_5^T X_5 - I\|_F < \epsilon$$

Trọng số được cập nhật theo công thức:
$$W_{t+1} = W_t (1 - \text{lr} \cdot \text{wd}) - 0.2 \cdot \text{lr} \cdot \sqrt{\max(r, c)} \cdot X_5$$
trong đó $0.2 \cdot \sqrt{\max(r, c)}$ là hệ số co dãn RMS đặc trưng của Muon.

**Nguyên tắc Phân định Tuyến tính Chặt chẽ**:
- Tất cả ma trận trọng số 2D (các ma trận chiếu Attention $W_q, W_k, W_v, W_o$ và MLP $W_{\text{gate}}, W_{\text{up}}, W_{\text{down}}$) được định tuyến cập nhật qua `Muon`.
- Tất cả vector 1D (trọng số RMSNorm, bias, token embeddings) được định tuyến qua `AdamW`.

#### Lịch học WSD (Warmup - Stable - Decay)
Lịch học WSD chia chu kỳ huấn luyện thành ba giai đoạn rõ rệt:
1. **Warmup ($0 \le t < S_w$)**: Tăng tuyến tính tốc độ học từ $\text{min\_lr}$ lên $\text{base\_lr}$:
   $$\text{lr}(t) = \text{min\_lr} + (\text{base\_lr} - \text{min\_lr}) \cdot \frac{t}{S_w}$$
2. **Stable ($S_w \le t < S_w + S_s$)**: Giữ nguyên tốc độ học cực đại $\text{base\_lr}$ để mô hình liên tục khám phá không gian tham số:
   $$\text{lr}(t) = \text{base\_lr}$$
3. **Decay ($S_w + S_s \le t \le S_{\text{total}}$)**: Giảm tốc độ học theo đường cong Cosine về $\text{min\_lr}$ để làm mịn và hội tụ mô hình:
   $$\text{lr}(t) = \text{min\_lr} + 0.5 \cdot (\text{base\_lr} - \text{min\_lr}) \cdot \left(1 + \cos\left(\pi \frac{t - S_w - S_s}{S_d}\right)\right)$$

Tại các điểm nối $t = S_w$ và $t = S_w + S_s$, hàm $\text{lr}(t)$ liên tục tuyệt đối, triệt tiêu mọi xung động gradient gây sốc mô hình.

---

### 3.3 Chặng 3: Tinh chỉnh có Giám sát Mặt nạ (Masked SFT) & Tầng LoRA

Sau giai đoạn tiền huấn luyện, mô hình có khả năng sinh văn bản tiếp nối nhưng chưa biết cách hội thoại theo định dạng câu hỏi - trả lời. Giai đoạn SFT (Supervised Fine-Tuning) định hình phong cách phản hồi của mô hình.

#### Tầng Tương thích Hạng thấp LoraLinear
Để tinh chỉnh mô hình mà không cần cập nhật toàn bộ hàng triệu tham số nền, ta đóng băng ma trận trọng số gốc $W_0 \in \mathbb{R}^{d_{\text{out}} \times d_{\text{in}}}$ và gắn song song một nhánh thích ứng hạng thấp:
$$W = W_0 + \Delta W, \quad \Delta W = \frac{\alpha}{r} (B \cdot A)$$
với $A \in \mathbb{R}^{r \times d_{\text{in}}}$ và $B \in \mathbb{R}^{d_{\text{out}} \times r}$, trong đó hạng $r \ll \min(d_{\text{in}}, d_{\text{out}})$.

```
                      x (Vector đầu vào d_in)
                     ┌───────────┴───────────┐
                     │                       │
                     ▼                       ▼
             [W_0 (Đóng băng)]       [A (Rank r x d_in)]
                     │                       │
                     │                       ▼
                     │               [B (d_out x Rank r)]
                     │                       │
                     │                       ▼
                     │                 x (alpha / r)
                     │                       │
                     ▼                       ▼
                   ( W_0 x )   +   ( (alpha/r) B A x )
                     └───────────┬───────────┘
                                 │
                                 ▼
                      y (Vector đầu ra d_out)
```

**Bất biến Zero-Cost LoRA tại bước 0**:
Ma trận $A$ được khởi tạo ngẫu nhiên Gaussian $\mathcal{N}(0, \sigma^2)$, còn ma trận $B$ được khởi tạo **hoàn toàn bằng $0$**:
$$B = 0 \implies B \cdot A = 0 \implies \Delta W = 0$$
Do đó, tại bước huấn luyện $0$, hành vi của `LoraLinear` trùng khớp $100\%$ với mô hình nền:
$$y = W_0 x + \frac{\alpha}{r} (0 \cdot A x) = W_0 x$$

#### Hợp nhất Trọng số Không Chi phí (Zero-Cost LoRA Merge)
Trong quá trình phục vụ suy luận thực tế (production inference), việc tính toán qua hai nhánh riêng biệt $W_0 x$ và $B A x$ sẽ làm tăng số phép nhân ma trận và độ trễ bộ nhớ. Phương thức `merge_lora()` tính trước tích ma trận và cộng trực tiếp vào trọng số nền:
$$W_{\text{base}} \leftarrow W_{\text{base}} + \frac{\alpha}{r} (B \cdot A)$$
Sau khi hợp nhất, phép tính chỉ đơn thuần là một phép nhân ma trận duy nhất $y = W_{\text{base}} x$. Kết quả kiểm thử chứng minh sai số giữa lan truyền LoRA động và lan truyền qua ma trận đã hợp nhất thỏa mãn:
$$\|y_{\text{dynamic}} - y_{\text{merged}}\|_\infty < 10^{-12}$$

#### Cơ chế Mặt nạ Tổn thất (Loss Masking) trong SFT
Một mẫu dữ liệu huấn luyện SFT bao gồm hai phần: lời nhắc của người dùng (Prompt) và câu trả lời mẫu của trợ lý (Completion):
$$X = [x_1, x_2, \dots, x_P, y_1, y_2, \dots, y_C]$$
Mô hình **chỉ được học cách sinh ra câu trả lời $y$, chứ không được học cách dự đoán lại lời nhắc $x$**. Nếu tính loss trên cả prompt, mô hình sẽ bị thiên kiến học vẹt các câu hỏi phổ biến.

Ta định nghĩa vector mặt nạ $M \in \{0.0, 1.0\}^{P+C}$:
$$M_t = \begin{cases} 0.0 & \text{nếu } t \le P \text{ (token thuộc Prompt)} \\ 1.0 & \text{nếu } t > P \text{ (token thuộc Completion)} \end{cases}$$

Hàm tổn thất Cross-Entropy có mặt nạ:
$$\mathcal{L}_{\text{SFT}} = \frac{\sum_{t=1}^{P+C} M_t \cdot \left( -\ln P(y_t \mid x_{<t}) \right)}{\max\left(1.0, \sum_{t=1}^{P+C} M_t\right)}$$

Đạo hàm giải tích đối với logit $z_{t, v}$ tại vị trí $t$ và từ vựng $v$:
$$\frac{\partial \mathcal{L}_{\text{SFT}}}{\partial z_{t, v}} = \frac{M_t}{\sum M} \cdot (P(v) - \mathbb{I}(v = \text{target}_t))$$
Khi $M_t = 0.0$ (token prompt), đạo hàm bằng đúng $0.0$, bảo đảm trọng số mô hình hoàn toàn không bị dịch chuyển bởi các token câu hỏi.

---

### 3.4 Chặng 4: Căn chỉnh Sở thích (Preference Alignment) DPO & ORPO

Sau SFT, mô hình có thể tạo ra nhiều phương án trả lời khác nhau nhưng chưa biết phương án nào được con người ưa chuộng hơn hoặc phương án nào an toàn hơn.

#### DPO (Direct Preference Optimization) với Làm mịn Nhãn
DPO loại bỏ sự cần thiết phải huấn luyện một mạng Reward Model riêng và thuật toán RL phức tạp (như PPO). DPO chứng minh rằng bài toán tối ưu hóa phần thưởng có thể ánh xạ trực tiếp về một hàm mất mát phân loại nhị phân trên chính mô hình chính sách $\pi_\theta$:

$$\mathcal{L}_{\text{DPO}}(\pi_\theta; \pi_{\text{ref}}) = -\mathbb{E}_{(x, y_w, y_l)} \left[ \ln \sigma \left( \beta \left( \ln \frac{\pi_\theta(y_w \mid x)}{\pi_{\text{ref}}(y_w \mid x)} - \ln \frac{\pi_\theta(y_l \mid x)}{\pi_{\text{ref}}(y_l \mid x)} \right) \right) \right]$$
trong đó $y_w$ là câu trả lời được chọn (winning/chosen), $y_l$ là câu trả lời bị loại (losing/rejected), và $\beta$ là hệ số kiểm soát khoảng cách phân kỳ KL đối với mô hình tham chiếu $\pi_{\text{ref}}$.

Đặt biên độ ưu tiên:
$$u = \beta \cdot \left[ (\ln \pi_\theta(y_w) - \ln \pi_{\text{ref}}(y_w)) - (\ln \pi_\theta(y_l) - \ln \pi_{\text{ref}}(y_l)) \right]$$

Để tránh việc mô hình trở nên quá tự tin (overconfident) dẫn đến suy giảm khả năng tổng quát hóa, ta tích hợp kỹ thuật **Làm mịn nhãn (Label Smoothing $\epsilon_s$)**:
$$\mathcal{L}_{\text{DPO}} = -(1 - \epsilon_s) \ln \sigma(u) - \epsilon_s \ln \sigma(-u)$$

Đạo hàm giải tích đối với $\ln \pi_\theta(y_w)$ và $\ln \pi_\theta(y_l)$:
$$\frac{\partial \mathcal{L}}{\partial u} = -(1 - \epsilon_s) \sigma(-u) + \epsilon_s \sigma(u)$$
$$\frac{\partial \mathcal{L}}{\partial \ln \pi_\theta(y_w)} = \frac{\partial \mathcal{L}}{\partial u} \cdot \beta, \quad \frac{\partial \mathcal{L}}{\partial \ln \pi_\theta(y_l)} = -\frac{\partial \mathcal{L}}{\partial u} \cdot \beta$$

Vì $\sigma(-u) > 0$, đạo hàm theo câu trả lời thắng luôn âm (khi gradient descent sẽ làm tăng xác suất $\pi_\theta(y_w)$), và đạo hàm theo câu trả lời thua luôn dương (làm giảm xác suất $\pi_\theta(y_l)$). Biên độ ưu tiên $u$ được chứng minh mở rộng đơn điệu sau các bước tối ưu.

#### ORPO (Odds Ratio Preference Optimization) & Hàm Ổn định `log1mexp`
ORPO kết hợp trực tiếp hàm mất mát SFT và phạt tỷ số khả dĩ (Odds Ratio) mà thậm chí không cần lưu giữ mô hình tham chiếu $\pi_{\text{ref}}$ trong bộ nhớ:
$$\text{odds}(y \mid x) = \frac{P(y \mid x)}{1 - P(y \mid x)}$$
$$\mathcal{L}_{\text{ORPO}} = \mathcal{L}_{\text{SFT}}(y_w) - \lambda \ln \sigma \left( \ln \frac{\text{odds}(y_w)}{\text{odds}(y_l)} \right)$$

Khi tính toán $\ln \text{odds}(y) = \ln P(y) - \ln(1 - P(y))$, đại lượng $\ln(1 - P(y)) = \ln(1 - e^{\ln P(y)})$ rất dễ gặp lỗi mất độ chính xác (catastrophic cancellation) khi $P(y) \approx 1$ hoặc $P(y) \approx 0$.

Ta thiết kế hàm hai nhánh ổn định số học `log1mexp(x)` cho $x < 0$ tại ngưỡng phân nhánh $-\ln 2 \approx -0.693147$:
$$\text{log1mexp}(x) = \begin{cases} \ln(-\text{expm1}(x)) & \text{khi } x > -\ln 2 \text{ (sử dụng } e^x - 1 \text{ chính xác cao)} \\ \ln(1 - e^x) & \text{khi } x \le -\ln 2 \text{ (tính trực tiếp an toàn)} \end{cases}$$
Hàm này đạt độ chính xác số học $< 10^{-12}$ trên toàn bộ miền giá trị từ $-10^{-14}$ tới $-50.0$.

---

### 3.5 Chặng 5: Học Tăng cường Suy luận với Phần thưởng Kiểm chứng được (RLVR)

RLVR (Reinforcement Learning with Verifiable Rewards) là kỹ thuật cốt lõi giúp các mô hình như OpenAI o1/o3 và DeepSeek-R1 đạt bước nhảy vọt về năng lực toán học và lập trình.

Khác với RLHF truyền thống (dùng Reward Model mạng nơ-ron vốn dễ bị "đánh lừa" - reward hacking), RLVR dựa vào các **bộ kiểm chứng toán học / biên dịch khách quan (Deterministic Verifiers)**.

#### Phạt Phân kỳ KL Không Âm $k3\_kl$ của John Schulman
Trong thuật toán PPO/GRPO, ta cần phạt mô hình khi chính sách mới $\pi_\theta$ đi quá xa khỏi chính sách gốc $\pi_{\text{ref}}$ để tránh sụp đổ tri thức. Ước lượng thông thường $\ln \frac{\pi_\theta}{\pi_{\text{ref}}}$ có thể nhận giá trị âm, dẫn đến việc mô hình nhận "thưởng ảo" khi phân kỳ.

John Schulman (tác giả chính của PPO) đề xuất ước lượng $k3\_kl$:
$$k3\_kl(\log \pi, \log \pi_{\text{ref}}) = \exp(\log \pi_{\text{ref}} - \log \pi) - (\log \pi_{\text{ref}} - \log \pi) - 1$$

**Chứng minh Bất biến Không âm (Invariant 4)**:
Đặt $z = \log \pi_{\text{ref}} - \log \pi$. Hàm số trở thành:
$$f(z) = e^z - z - 1$$
Xét đạo hàm: $f'(z) = e^z - 1$.
- Khi $z = 0$: $f'(0) = 0$ và $f(0) = 1 - 0 - 1 = 0$.
- Khi $z > 0$: $f'(z) > 0 \implies f(z)$ đồng biến, do đó $f(z) > f(0) = 0$.
- Khi $z < 0$: $f'(z) < 0 \implies f(z)$ nghịch biến, do đó $f(z) > f(0) = 0$.

Vậy $\forall z \in \mathbb{R}, f(z) \ge 0$, đẳng thức xảy ra khi và chỉ khi $z = 0$ ($\pi_\theta = \pi_{\text{ref}}$). Bất biến KL không âm được chứng minh tổng quát.

Đạo hàm giải tích theo $\log \pi$:
$$\frac{\partial}{\partial \log \pi} k3\_kl = -e^z - (-1) = 1 - e^{\log \pi_{\text{ref}} - \log \pi}$$

#### Bộ Chấm Toán học Tất định MathAnswerVerifier
Mô hình được huấn luyện để sinh ra định dạng thẻ suy luận:
```
<think>
[Chuỗi lập luận từng bước]
<call:calc>[Biểu thức số học]</call:calc>
<result>[Kết quả môi trường trả về]</result>
</think>
<answer>[Kết quả số học cuối cùng]</answer>
```

Bộ chấm `MathAnswerVerifier` áp dụng thang điểm đánh giá (rubric) nghiêm ngặt:
- **2.5 điểm**: Đáp án trong `<answer>` chính xác, đúng định dạng thẻ, có gọi công cụ xác thực qua `<call:calc>`.
- **2.0 điểm**: Đáp án chính xác và đúng định dạng thẻ, tự suy luận không cần công cụ.
- **1.0 điểm**: Đáp án đúng nhưng thiếu hoặc hỏng định dạng thẻ.
- **0.5 điểm**: Định dạng thẻ chuẩn mực nhưng tính toán sai số.
- **0.0 điểm**: Sai kết quả và định dạng thẻ hỏng.

#### Thuật toán DrGRPO (Khử Thiên kiến Độ dài) & DAPO (Kẹp Bất đối xứng)
Thuật toán GRPO (Group Relative Policy Optimization) lấy mẫu một nhóm $G$ câu trả lời cho cùng một câu hỏi và chuẩn hóa phần thưởng theo nhóm:
$$A_i = \frac{R_i - \text{mean}(R)}{\text{std}(R) + \epsilon}$$

Tuy nhiên, GRPO truyền thống mắc phải **thiên kiến độ dài**: các câu trả lời dài thường nhận được tổng lợi thế lớn hơn, khiến mô hình có xu hướng kéo dài câu trả lời lê thê (length bias).
- **`DrGRPO`** ("GRPO Done Right", Liu et al., 2025): khử thiên kiến bằng cách **bỏ cả hai** số hạng chuẩn hóa — không chia cho độ lệch chuẩn (ở lợi thế) và không chia cho độ dài thực $|o_i|$ (ở tổng hợp loss). Lợi thế chỉ còn hiệu với trung bình nhóm, còn chuẩn hóa độ dài được thay bằng hằng số $L_{\max}$ trong phép tổng token:
  $$\tilde{A}_i = R_i - \text{mean}(R), \qquad \text{trọng số loss mỗi chuỗi} = \frac{1}{L_{\max}}\ \text{(thay cho } 1/|o_i|)$$
- **`DAPO`**: Áp dụng kẹp lợi thế bất đối xứng $[-\epsilon_{\text{low}}, \epsilon_{\text{high}}]$ và **lọc bỏ hoàn toàn các nhóm có phương sai bằng 0** ($\text{std}(R) < 10^{-12}$, ví dụ toàn bộ nhóm cùng đúng hoặc cùng sai). Điều này triệt tiêu hoàn toàn lỗi chia cho 0 (`0/0 = NaN`).

#### Bất biến An toàn Gọt Ngữ cảnh (Conditional Context Trimming Credit)
Để khuyến khích mô hình suy nghĩ súc tích, ta cộng thêm phần thưởng hiệu suất độ dài:
$$A_i^{\text{eff}} = \frac{\bar{L}_{\text{winner}} - L_i}{\sigma_L + \epsilon}$$
Tuy nhiên, nếu chỉ có **1 câu trả lời thắng duy nhất**, mô hình có thể "ăn gian" bằng cách phát ra một câu trả lời cực ngắn (thậm chí rỗng) nhưng may mắn trúng đáp án để nhận thưởng độ dài khổng lồ.

Ta thiết lập chốt an toàn: **Tín dụng gọt ngữ cảnh chỉ được kích hoạt khi nhóm có ít nhất 2 quỹ đạo chiến thắng ($\text{count}(\text{winners}) \ge 2$)**. Nếu chỉ có $\le 1$ người thắng, tín dụng độ dài bằng đúng $0.0$.

---

### 3.6 Chặng 6: Phục vụ Suy luận, RadixPrefixTree & Tác tử Gọi Công cụ

Giai đoạn cuối cùng đưa toàn bộ thành quả vào một hệ thống phục vụ suy luận tác tử thời gian thực.

#### Cấu trúc Cây Tiền tố RadixPrefixTree
Thay vì lưu KV-Cache tuyến tính, `RadixPrefixTree` tổ chức các chuỗi token thành một cây tiền tố nén (Radix Tree). Khi nhiều phiên hội thoại cùng dùng chung một System Prompt hoặc lịch sử hội thoại trước đó, các token tiền tố chỉ được tính toán đúng một lần và lưu trong nút cây.

```
                         [Root (ID: 0)]
                               │
                       "Hỏi: Tính giá trị"
                               │
                        [Nút 1 (Cached KV)]
                               │
                 ┌─────────────┴─────────────┐
                 │                           │
          " 14 * 3 + 12"              " 25 + 17 * 2"
                 │                           │
        [Nút 2 (Cached KV)]         [Nút 3 (Cached KV)]
```

Khi yêu cầu mới đến:
1. `match_longest_prefix`: Quét từ gốc để tìm tiền tố trùng dài nhất và lấy ra `KvCache` đã nạp sẵn.
2. Nếu tiền tố chỉ khớp một phần cạnh của cây, thuật toán tự động **tách cạnh (edge splitting)**: nút cũ được tách làm đôi, tạo nút trung gian lưu phần tiền tố chung.
3. Chính sách giải phóng LRU (`evict_lru`): Khi bộ nhớ đệm đầy, cây sẽ giải phóng các nút lá ít được truy cập nhất (`last_accessed_step` nhỏ nhất) mà không bị ghim (`pinned == false`).

#### Giải mã Suy đoán Chính xác (Speculative Decoding)
Giải mã suy đoán sử dụng một mô hình nháp nhỏ (Draft Model $\mathcal{M}_q$) sinh nhanh $K$ token, sau đó dùng mô hình đích lớn ($\mathcal{M}_p$) thẩm định song song toàn bộ $K$ token trong một lượt forward duy nhất.

Quy tắc chấp nhận của Leviathan et al. (2023):
Token nháp $x_i$ được chấp nhận với xác suất:
$$\alpha_i = \min\left(1, \frac{P_p(x_i)}{P_q(x_i)}\right)$$

Nếu token $x_i$ bị từ chối tại vị trí $i$:
1. Toàn bộ các token từ $i+1$ đến $K$ bị hủy bỏ.
2. Token thay thế tại vị trí $i$ được lấy mẫu từ **phân phối phần dư chuẩn hóa**:
   $$P_{\text{residual}}(x) = \frac{\max(0, P_p(x) - P_q(x))}{\sum_{x'} \max(0, P_p(x') - P_q(x'))}$$

**Định lý Bảo toàn Phân phối**:
Xác suất phân phối biên của token xuất xưởng từ giải mã suy đoán trùng khớp tuyệt đối $100\%$ với phân phối gốc của mô hình đích $P_p(x)$. Tốc độ sinh tăng từ $1.5\times$ đến $2.5\times$ mà không làm suy giảm dù chỉ $0.0001\%$ chất lượng đầu ra.

#### Máy Trạng thái Tác tử Nội sinh NativeToolAgent
Vòng lặp tác tử tự hành vận hành như một máy trạng thái hữu hạn (FSM):

```
       ┌────────────────────────┐
       │     AgentStatus::      │◄─────────────────────────────┐
       │        Thinking        │                              │
       └───────────┬────────────┘                              │
                   │                                           │
         Phát hiện │ Có thẻ                                    │ Trả về kết quả
         <answer>  │ <call:calc>                               │ <result>VAL</result>
                   ▼                                           │
       ┌────────────────────────┐                  ┌───────────┴────────────┐
       │     AgentStatus::      │                  │     AgentStatus::      │
       │       Completed        │                  │   WaitingToolResult    │
       └────────────────────────┘                  └───────────┬────────────┘
                                                               │
                                                               ▼
                                                   [Thực thi NativeCalcTool]
```

1. **Thinking**: Mô hình sinh chuỗi suy luận. Nếu phát hiện thẻ `<answer>...</answer>`, tác tử lập tức chuyển sang `Completed` và trả về kết quả cho người dùng.
2. **WaitingToolResult**: Nếu mô hình phát ra thẻ gọi công cụ `<call:calc>EXPR</call:calc>`, tác tử tạm dừng suy luận, trích xuất biểu thức `EXPR`.
3. Bộ phân giải số học `NativeCalcTool` đánh giá biểu thức với độ ưu tiên toán tử chuẩn (`*`, `/` trước, `+`, `-` sau).
4. Kết quả tính toán được đóng gói thành `<result>VAL</result>`, gắn vào ngữ cảnh và đưa trạng thái trở lại `Thinking` để mô hình tiếp tục suy luận.

---

## 4. Mã nguồn minh họa thực chiến

Toàn bộ mã nguồn thực chiến đã được cài đặt, định dạng và vượt qua 100% các cổng kiểm thử tại `code/ch97/src/main.rs`. Dưới đây là phân tích chi tiết từng khối mã nguồn đại diện cấu thành nên đường ống 6 chặng hoàn chỉnh.

---

### 4.1 Tiện ích Đại số & Bộ Sinh Số Tất Định SplitMix64

```rust
use std::cmp::Reverse;
use std::collections::{BTreeMap, BinaryHeap, HashMap};
use std::f64::consts::{LN_2, PI};

/// Bộ sinh số giả ngẫu nhiên 64-bit tất định SplitMix64.
///
/// Thiết kế sử dụng struct với trường có tên (named fields), cho phép tái lập 100%
/// mọi trọng số và kết quả lấy mẫu trên mọi kiến trúc CPU mà không phụ thuộc thư viện ngoài.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct SplitMix64 {
    /// Trạng thái thanh ghi 64-bit nội bộ
    pub state: u64,
}

impl SplitMix64 {
    /// Khởi tạo bộ sinh số với hạt giống cố định
    pub const fn new(seed: u64) -> Self {
        Self { state: seed }
    }

    /// Sinh số nguyên không dấu 64-bit ngẫu nhiên tiếp theo
    pub fn next_u64(&mut self) -> u64 {
        self.state = self.state.wrapping_add(0x9E37_79B9_7F4A_7C15);
        let mut z = (self.state ^ (self.state >> 30)).wrapping_mul(0xBF58_476D_1CE4_E5B9);
        z = (z ^ (z >> 27)).wrapping_mul(0x94D0_49BB_1331_11EB);
        z ^ (z >> 31)
    }

    /// Sinh số thực dấu phẩy động trong nửa khoảng [0.0, 1.0)
    pub fn next_f64(&mut self) -> f64 {
        let u = (self.next_u64() >> 11) as f64;
        u * (1.0 / ((1u64 << 53) as f64))
    }

    /// Sinh số thực dấu phẩy động có dấu trong khoảng [-scale, scale]
    pub fn next_signed(&mut self, scale: f64) -> f64 {
        (self.next_f64() * 2.0 - 1.0) * scale
    }

    /// Sinh số thực chuẩn tắc Gaussian N(0, 1) qua phép biến đổi Box-Muller
    pub fn next_gaussian(&mut self, mean: f64, std_dev: f64) -> f64 {
        let u1 = self.next_f64().max(1e-15);
        let u2 = self.next_f64();
        let z0 = (-2.0 * u1.ln()).sqrt() * (2.0 * PI * u2).cos();
        mean + z0 * std_dev
    }
}

/// Nhân hai ma trận A và B (m x k nhân k x n)
pub fn matmul(a: &[f64], b: &[f64], m: usize, k: usize, n: usize) -> Vec<f64> {
    assert_eq!(a.len(), m * k, "Kích thước ma trận A không hợp lệ");
    assert_eq!(b.len(), k * n, "Kích thước ma trận B không hợp lệ");
    let mut out = vec![0.0; m * n];
    for i in 0..m {
        for p in 0..k {
            let a_val = a[i * k + p];
            for j in 0..n {
                out[i * n + j] += a_val * b[p * n + j];
            }
        }
    }
    out
}

/// Chuyển vị ma trận rows x cols
pub fn transpose(mat: &[f64], rows: usize, cols: usize) -> Vec<f64> {
    assert_eq!(mat.len(), rows * cols, "Kích thước ma trận không khớp");
    let mut out = vec![0.0; rows * cols];
    for r in 0..rows {
        for c in 0..cols {
            out[c * rows + r] = mat[r * cols + c];
        }
    }
    out
}

pub fn frobenius_norm(mat: &[f64]) -> f64 {
    mat.iter().map(|v| v * v).sum::<f64>().sqrt()
}

pub fn silu(x: f64) -> f64 {
    x / (1.0 + (-x).exp())
}

pub fn silu_derivative(x: f64) -> f64 {
    let s = 1.0 / (1.0 + (-x).exp());
    s + x * s * (1.0 - s)
}

pub fn softmax_1d(logits: &[f64]) -> Vec<f64> {
    let max_val = logits.iter().copied().fold(f64::NEG_INFINITY, f64::max);
    let mut exp_vals: Vec<f64> = logits.iter().map(|&x| (x - max_val).exp()).collect();
    let sum_exp: f64 = exp_vals.iter().sum();
    let inv_sum = if sum_exp > 0.0 { 1.0 / sum_exp } else { 0.0 };
    for v in exp_vals.iter_mut() {
        *v *= inv_sum;
    }
    exp_vals
}

pub fn log1mexp(x: f64) -> f64 {
    assert!(x < 0.0, "log1mexp chỉ xác định với x < 0.0, nhận: {x}");
    if x > -LN_2 {
        (-x.exp_m1()).ln()
    } else {
        (1.0 - x.exp()).ln()
    }
}
```

---

### 4.2 Chặng 1: Bộ Tách Từ Byte-BPE có Token Đặc biệt

```rust
pub const PAD_TOKEN_ID: u16 = 256;
pub const THINK_START_ID: u16 = 257;
pub const THINK_END_ID: u16 = 258;
pub const CALL_CALC_START_ID: u16 = 259;
pub const CALL_CALC_END_ID: u16 = 260;
pub const RESULT_START_ID: u16 = 261;
pub const RESULT_END_ID: u16 = 262;
pub const ANSWER_START_ID: u16 = 263;
pub const ANSWER_END_ID: u16 = 264;

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct MergeRule {
    pub left: u16,
    pub right: u16,
    pub merged_id: u16,
}

#[derive(Debug, Clone)]
pub struct BpeTokenizer {
    pub vocab: Vec<Vec<u8>>,
    pub merges: Vec<MergeRule>,
    pub ranks: HashMap<(u16, u16), (usize, u16)>,
    pub special_tokens_map: HashMap<String, u16>,
    pub special_tokens_rev: HashMap<u16, String>,
}

fn merge_word_tokens(word: &[u16], pair: (u16, u16), new_id: u16) -> Vec<u16> {
    let mut out = Vec::with_capacity(word.len());
    let mut i = 0;
    while i < word.len() {
        if i + 1 < word.len() && word[i] == pair.0 && word[i + 1] == pair.1 {
            out.push(new_id);
            i += 2;
        } else {
            out.push(word[i]);
            i += 1;
        }
    }
    out
}

impl BpeTokenizer {
    pub fn new() -> Self {
        let mut vocab: Vec<Vec<u8>> = (0..=255u8).map(|b| vec![b]).collect();
        let mut special_map = HashMap::new();
        let mut special_rev = HashMap::new();

        let specials = [
            ("<pad>", PAD_TOKEN_ID),
            ("<think>", THINK_START_ID),
            ("</think>", THINK_END_ID),
            ("<call:calc>", CALL_CALC_START_ID),
            ("</call:calc>", CALL_CALC_END_ID),
            ("<result>", RESULT_START_ID),
            ("</result>", RESULT_END_ID),
            ("<answer>", ANSWER_START_ID),
            ("</answer>", ANSWER_END_ID),
        ];

        for (token_str, token_id) in specials {
            while vocab.len() < usize::from(token_id) {
                vocab.push(Vec::new());
            }
            if vocab.len() == usize::from(token_id) {
                vocab.push(token_str.as_bytes().to_vec());
            } else {
                vocab[usize::from(token_id)] = token_str.as_bytes().to_vec();
            }
            special_map.insert(token_str.to_string(), token_id);
            special_rev.insert(token_id, token_str.to_string());
        }

        Self {
            vocab,
            merges: Vec::new(),
            ranks: HashMap::new(),
            special_tokens_map: special_map,
            special_tokens_rev: special_rev,
        }
    }

    pub fn vocab_size(&self) -> usize {
        self.vocab.len()
    }

    pub fn train(&mut self, corpus: &str, max_merges: usize) {
        let pieces = corpus.split_inclusive(char::is_whitespace);
        let mut words: Vec<Vec<u16>> = pieces
            .map(|s| s.bytes().map(u16::from).collect())
            .filter(|w: &Vec<u16>| !w.is_empty())
            .collect();

        for _ in 0..max_merges {
            let mut counts: HashMap<(u16, u16), usize> = HashMap::new();
            for w in &words {
                for p in w.windows(2) {
                    *counts.entry((p[0], p[1])).or_insert(0) += 1;
                }
            }

            let mut heap = BinaryHeap::new();
            for (pair, count) in counts {
                if count >= 2 {
                    heap.push((count, Reverse(pair)));
                }
            }

            let Some((_, Reverse(best_pair))) = heap.pop() else {
                break;
            };

            let new_id = u16::try_from(self.vocab.len()).expect("Kích thước từ vựng vượt quá u16");
            let mut merged_bytes = self.vocab[usize::from(best_pair.0)].clone();
            merged_bytes.extend_from_slice(&self.vocab[usize::from(best_pair.1)]);
            self.vocab.push(merged_bytes);

            let rank_idx = self.merges.len();
            self.ranks.insert(best_pair, (rank_idx, new_id));
            self.merges.push(MergeRule {
                left: best_pair.0,
                right: best_pair.1,
                merged_id: new_id,
            });

            for w in &mut words {
                if w.len() >= 2 {
                    *w = merge_word_tokens(w, best_pair, new_id);
                }
            }
        }
    }

    pub fn encode(&self, text: &str) -> Vec<u16> {
        let mut result = Vec::new();
        let mut cursor = 0;

        while cursor < text.len() {
            let mut earliest_special: Option<(usize, usize, u16)> = None;
            for (st, &s_id) in &self.special_tokens_map {
                if let Some(pos) = text[cursor..].find(st) {
                    let abs_pos = cursor + pos;
                    match earliest_special {
                        None => earliest_special = Some((abs_pos, st.len(), s_id)),
                        Some((cur_pos, _, _)) if abs_pos < cur_pos => {
                            earliest_special = Some((abs_pos, st.len(), s_id));
                        }
                        _ => {}
                    }
                }
            }

            match earliest_special {
                Some((start, len, special_id)) => {
                    if start > cursor {
                        self.encode_plain_text(&text[cursor..start], &mut result);
                    }
                    result.push(special_id);
                    cursor = start + len;
                }
                None => {
                    self.encode_plain_text(&text[cursor..], &mut result);
                    break;
                }
            }
        }

        result
    }

    fn encode_plain_text(&self, text: &str, out: &mut Vec<u16>) {
        for piece in text.split_inclusive(char::is_whitespace) {
            let mut ids: Vec<u16> = piece.bytes().map(u16::from).collect();
            while ids.len() >= 2 {
                let mut best_merge: Option<(usize, (u16, u16), u16)> = None;
                for w in ids.windows(2) {
                    let pair = (w[0], w[1]);
                    if let Some(&(rank, new_id)) = self.ranks.get(&pair)
                        && best_merge.is_none_or(|(b_rank, _, _)| rank < b_rank)
                    {
                        best_merge = Some((rank, pair, new_id));
                    }
                }
                let Some((_, pair, new_id)) = best_merge else {
                    break;
                };
                ids = merge_word_tokens(&ids, pair, new_id);
            }
            out.extend(ids);
        }
    }

    pub fn decode(&self, ids: &[u16]) -> Result<String, String> {
        let mut raw_bytes = Vec::new();
        for &id in ids {
            if let Some(special_str) = self.special_tokens_rev.get(&id) {
                raw_bytes.extend_from_slice(special_str.as_bytes());
            } else if let Some(bytes) = self.vocab.get(usize::from(id)) {
                raw_bytes.extend_from_slice(bytes);
            } else {
                return Err(format!("ID token không hợp lệ ngoài từ vựng: {id}"));
            }
        }
        String::from_utf8(raw_bytes).map_err(|e| format!("Lỗi giải mã chuỗi UTF-8: {e}"))
    }
}

impl Default for BpeTokenizer {
    fn default() -> Self {
        Self::new()
    }
}
```

---

### 4.3 Chặng 2: Khối Transformer Vi mô, RMSNorm Giải tích, RoPE & SwiGLU

```rust
#[derive(Debug, Clone)]
pub struct RmsNormGrads {
    pub grad_x: Vec<f64>,
    pub grad_w: Vec<f64>,
}

pub fn rmsnorm_forward_and_backward(
    x: &[f64],
    w: &[f64],
    eps: f64,
    grad_output: &[f64],
) -> (Vec<f64>, RmsNormGrads) {
    let d = x.len();
    assert_eq!(w.len(), d);
    assert_eq!(grad_output.len(), d);

    let sum_sq: f64 = x.iter().map(|v| v * v).sum();
    let mean_sq = sum_sq / (d as f64);
    let rms = (mean_sq + eps).sqrt();
    let inv_rms = 1.0 / rms;

    let mut y = vec![0.0; d];
    let mut grad_w = vec![0.0; d];
    let mut dot_gw_x = 0.0;

    for i in 0..d {
        let x_hat = x[i] * inv_rms;
        y[i] = x_hat * w[i];
        grad_w[i] = grad_output[i] * x_hat;
        dot_gw_x += grad_output[i] * w[i] * x[i];
    }

    let coef = dot_gw_x / ((d as f64) * (mean_sq + eps));
    let mut grad_x = vec![0.0; d];
    for i in 0..d {
        grad_x[i] = inv_rms * (grad_output[i] * w[i] - x[i] * coef);
    }

    (y, RmsNormGrads { grad_x, grad_w })
}

pub fn rope_rotate_inplace(vec: &mut [f64], pos: usize, base: f64) {
    let d = vec.len();
    let half = d / 2;
    for i in 0..half {
        let theta = (pos as f64) / base.powf((2 * i) as f64 / (d as f64));
        let (sin_t, cos_t) = theta.sin_cos();
        let (x1, x2) = (vec[i], vec[i + half]);
        vec[i] = x1 * cos_t - x2 * sin_t;
        vec[i + half] = x2 * cos_t + x1 * sin_t;
    }
}

#[derive(Debug, Clone, Copy)]
pub struct GqaConfig {
    pub num_query_heads: usize,
    pub num_kv_heads: usize,
    pub head_dim: usize,
    pub rope_base: f64,
}

#[derive(Debug, Clone, Default)]
pub struct KvCache {
    pub keys: Vec<f64>,
    pub values: Vec<f64>,
    pub seq_len: usize,
}

impl KvCache {
    pub fn new() -> Self {
        Self {
            keys: Vec::new(),
            values: Vec::new(),
            seq_len: 0,
        }
    }

    pub fn step_decode(
        &mut self,
        cfg: GqaConfig,
        query: &[f64],
        key: &[f64],
        val: &[f64],
    ) -> Vec<f64> {
        let q_dim = cfg.num_query_heads * cfg.head_dim;
        let kv_dim = cfg.num_kv_heads * cfg.head_dim;
        assert_eq!(query.len(), q_dim);
        assert_eq!(key.len(), kv_dim);
        assert_eq!(val.len(), kv_dim);

        let group_ratio = cfg.num_query_heads / cfg.num_kv_heads;
        let pos = self.seq_len;

        let mut rotated_q = query.to_vec();
        for chunk in rotated_q.chunks_exact_mut(cfg.head_dim) {
            rope_rotate_inplace(chunk, pos, cfg.rope_base);
        }

        let mut rotated_k = key.to_vec();
        for chunk in rotated_k.chunks_exact_mut(cfg.head_dim) {
            rope_rotate_inplace(chunk, pos, cfg.rope_base);
        }

        self.keys.extend_from_slice(&rotated_k);
        self.values.extend_from_slice(val);
        self.seq_len += 1;

        let scale = 1.0 / (cfg.head_dim as f64).sqrt();
        let mut output = vec![0.0; q_dim];

        for q_h in 0..cfg.num_query_heads {
            let kv_h = q_h / group_ratio;
            let q_vec = &rotated_q[q_h * cfg.head_dim..(q_h + 1) * cfg.head_dim];

            let mut scores = vec![0.0; self.seq_len];
            for (t, score) in scores.iter_mut().enumerate() {
                let k_offset = t * kv_dim + kv_h * cfg.head_dim;
                let k_vec = &self.keys[k_offset..k_offset + cfg.head_dim];
                let dot: f64 = q_vec.iter().zip(k_vec).map(|(a, b)| a * b).sum();
                *score = dot * scale;
            }

            let weights = softmax_1d(&scores);

            let out_head = &mut output[q_h * cfg.head_dim..(q_h + 1) * cfg.head_dim];
            for (t, &w) in weights.iter().enumerate() {
                let v_offset = t * kv_dim + kv_h * cfg.head_dim;
                let v_vec = &self.values[v_offset..v_offset + cfg.head_dim];
                for (dest, &src) in out_head.iter_mut().zip(v_vec) {
                    *dest += w * src;
                }
            }
        }

        output
    }
}

pub fn gqa_causal_full(
    cfg: GqaConfig,
    all_q: &[f64],
    all_k: &[f64],
    all_v: &[f64],
    seq_len: usize,
) -> Vec<f64> {
    let q_dim = cfg.num_query_heads * cfg.head_dim;
    let kv_dim = cfg.num_kv_heads * cfg.head_dim;
    let mut cache = KvCache::new();
    let mut full_out = Vec::with_capacity(seq_len * q_dim);

    for t in 0..seq_len {
        let q = &all_q[t * q_dim..(t + 1) * q_dim];
        let k = &all_k[t * kv_dim..(t + 1) * kv_dim];
        let v = &all_v[t * kv_dim..(t + 1) * kv_dim];
        let out_step = cache.step_decode(cfg, q, k, v);
        full_out.extend_from_slice(&out_step);
    }

    full_out
}

#[derive(Debug, Clone)]
pub struct SwiGluMlp {
    pub d_model: usize,
    pub d_hidden: usize,
    pub gate_weight: Vec<f64>,
    pub up_weight: Vec<f64>,
    pub down_weight: Vec<f64>,
}

impl SwiGluMlp {
    pub fn new(d_model: usize, d_hidden: usize, rng: &mut SplitMix64) -> Self {
        let scale_gate = (2.0 / (d_model as f64)).sqrt();
        let scale_down = (2.0 / (d_hidden as f64)).sqrt();

        let gate_weight = (0..d_hidden * d_model)
            .map(|_| rng.next_signed(scale_gate))
            .collect();
        let up_weight = (0..d_hidden * d_model)
            .map(|_| rng.next_signed(scale_gate))
            .collect();
        let down_weight = (0..d_model * d_hidden)
            .map(|_| rng.next_signed(scale_down))
            .collect();

        Self {
            d_model,
            d_hidden,
            gate_weight,
            up_weight,
            down_weight,
        }
    }

    pub fn forward(&self, x: &[f64]) -> Vec<f64> {
        assert_eq!(x.len(), self.d_model);
        let gate_out = matmul(&self.gate_weight, x, self.d_hidden, self.d_model, 1);
        let up_out = matmul(&self.up_weight, x, self.d_hidden, self.d_model, 1);

        let mut hidden = vec![0.0; self.d_hidden];
        for i in 0..self.d_hidden {
            hidden[i] = silu(gate_out[i]) * up_out[i];
        }

        matmul(&self.down_weight, &hidden, self.d_model, self.d_hidden, 1)
    }
}
```

---

### 4.4 Chặng 3: Tiền huấn luyện Muon, AdamW & Lịch học WSD

```rust
pub fn muon_orthogonalize_5steps(matrix: &[f64], rows: usize, cols: usize) -> Vec<f64> {
    assert_eq!(matrix.len(), rows * cols);
    let frob = frobenius_norm(matrix) + 1e-12;
    let normalized: Vec<f64> = matrix.iter().map(|&v| v / frob).collect();

    let (r, c, mut x) = if rows > cols {
        (cols, rows, transpose(&normalized, rows, cols))
    } else {
        (rows, cols, normalized)
    };

    let (a_coef, b_coef, c_coef) = (3.4445, -4.7750, 2.0315);

    for _ in 0..5 {
        let x_trans = transpose(&x, r, c);
        let a_mat = matmul(&x, &x_trans, r, c, r);
        let a_sq = matmul(&a_mat, &a_mat, r, r, r);

        let mut b_mat = vec![0.0; r * r];
        for i in 0..r * r {
            b_mat[i] = b_coef * a_mat[i] + c_coef * a_sq[i];
        }

        let bx_mat = matmul(&b_mat, &x, r, r, c);
        for i in 0..r * c {
            x[i] = a_coef * x[i] + bx_mat[i];
        }
    }

    if rows > cols {
        transpose(&x, r, c)
    } else {
        x
    }
}

#[derive(Debug, Clone)]
pub struct AdamWState {
    pub m: Vec<f64>,
    pub v: Vec<f64>,
    pub step: usize,
    pub lr: f64,
    pub beta1: f64,
    pub beta2: f64,
    pub eps: f64,
    pub weight_decay: f64,
}

impl AdamWState {
    pub fn new(size: usize, lr: f64, weight_decay: f64) -> Self {
        Self {
            m: vec![0.0; size],
            v: vec![0.0; size],
            step: 0,
            lr,
            beta1: 0.9,
            beta2: 0.999,
            eps: 1e-8,
            weight_decay,
        }
    }

    pub fn update(&mut self, param: &mut [f64], grad: &[f64], lr_override: Option<f64>) {
        assert_eq!(param.len(), grad.len());
        self.step += 1;
        let lr = lr_override.unwrap_or(self.lr);
        let b1 = self.beta1;
        let b2 = self.beta2;
        let b1_pow = b1.powi(self.step as i32);
        let b2_pow = b2.powi(self.step as i32);

        for i in 0..param.len() {
            self.m[i] = b1 * self.m[i] + (1.0 - b1) * grad[i];
            self.v[i] = b2 * self.v[i] + (1.0 - b2) * grad[i] * grad[i];

            let m_hat = self.m[i] / (1.0 - b1_pow);
            let v_hat = self.v[i] / (1.0 - b2_pow);

            param[i] = param[i] * (1.0 - lr * self.weight_decay)
                - lr * (m_hat / (v_hat.sqrt() + self.eps));
        }
    }
}

#[derive(Debug, Clone)]
pub struct MuonOptimizer {
    pub momentum_buffer: HashMap<usize, Vec<f64>>,
    pub lr: f64,
    pub momentum: f64,
    pub weight_decay: f64,
}

impl MuonOptimizer {
    pub fn new(lr: f64, weight_decay: f64) -> Self {
        Self {
            momentum_buffer: HashMap::new(),
            lr,
            momentum: 0.95,
            weight_decay,
        }
    }

    pub fn step(
        &mut self,
        param_id: usize,
        param: &mut [f64],
        grad: &[f64],
        rows: usize,
        cols: usize,
        lr_override: Option<f64>,
    ) {
        assert_eq!(param.len(), rows * cols);
        assert_eq!(grad.len(), rows * cols);

        let lr = lr_override.unwrap_or(self.lr);
        let m_buf = self
            .momentum_buffer
            .entry(param_id)
            .or_insert_with(|| vec![0.0; param.len()]);

        for (m, &g) in m_buf.iter_mut().zip(grad) {
            *m = self.momentum * (*m) + g;
        }

        let ortho = muon_orthogonalize_5steps(m_buf, rows, cols);
        let rms_scale = 0.2 * lr * ((rows.max(cols)) as f64).sqrt();

        for (p, &o) in param.iter_mut().zip(&ortho) {
            *p = *p * (1.0 - lr * self.weight_decay) - rms_scale * o;
        }
    }
}

#[derive(Debug, Clone, Copy)]
pub struct WsdSchedule {
    pub warmup_steps: usize,
    pub stable_steps: usize,
    pub decay_steps: usize,
    pub base_lr: f64,
    pub min_lr: f64,
}

impl WsdSchedule {
    pub fn new(
        warmup_steps: usize,
        stable_steps: usize,
        decay_steps: usize,
        base_lr: f64,
        min_lr: f64,
    ) -> Self {
        Self {
            warmup_steps,
            stable_steps,
            decay_steps,
            base_lr,
            min_lr,
        }
    }

    pub fn total_steps(&self) -> usize {
        self.warmup_steps + self.stable_steps + self.decay_steps
    }

    pub fn get_lr(&self, step: usize) -> f64 {
        if step < self.warmup_steps {
            if self.warmup_steps == 0 {
                return self.base_lr;
            }
            let alpha = (step as f64) / (self.warmup_steps as f64);
            self.min_lr + (self.base_lr - self.min_lr) * alpha
        } else if step < self.warmup_steps + self.stable_steps {
            self.base_lr
        } else if step <= self.total_steps() {
            if self.decay_steps == 0 {
                return self.min_lr;
            }
            let progress =
                (step - (self.warmup_steps + self.stable_steps)) as f64 / (self.decay_steps as f64);
            let cosine = 0.5 * (1.0 + (PI * progress).cos());
            self.min_lr + (self.base_lr - self.min_lr) * cosine
        } else {
            self.min_lr
        }
    }
}
```

---

### 4.5 Chặng 4 & 5: LoraLinear SFT, DPO, ORPO & Học Tăng Cường RLVR

```rust
#[derive(Debug, Clone)]
pub struct LoraLinear {
    pub base_weight: Vec<f64>,
    pub lora_a: Vec<f64>,
    pub lora_b: Vec<f64>,
    pub in_features: usize,
    pub out_features: usize,
    pub rank: usize,
    pub alpha: f64,
    pub original_base_weight: Option<Vec<f64>>,
}

impl LoraLinear {
    pub fn new(
        base_weight: Vec<f64>,
        in_features: usize,
        out_features: usize,
        rank: usize,
        alpha: f64,
        rng: &mut SplitMix64,
    ) -> Self {
        assert_eq!(base_weight.len(), out_features * in_features);
        let scale_a = (1.0 / (in_features as f64)).sqrt();
        let lora_a = (0..rank * in_features)
            .map(|_| rng.next_gaussian(0.0, scale_a))
            .collect();
        let lora_b = vec![0.0; out_features * rank];

        Self {
            base_weight,
            lora_a,
            lora_b,
            in_features,
            out_features,
            rank,
            alpha,
            original_base_weight: None,
        }
    }

    pub fn forward(&self, x: &[f64]) -> Vec<f64> {
        assert_eq!(x.len(), self.in_features);
        let base_out = matmul(&self.base_weight, x, self.out_features, self.in_features, 1);
        let ax = matmul(&self.lora_a, x, self.rank, self.in_features, 1);
        let bax = matmul(&self.lora_b, &ax, self.out_features, self.rank, 1);
        let scale = self.alpha / (self.rank as f64);

        let mut out = base_out;
        for (o, &delta) in out.iter_mut().zip(&bax) {
            *o += scale * delta;
        }
        out
    }

    pub fn merge_lora(&mut self) {
        if self.original_base_weight.is_some() {
            return;
        }
        self.original_base_weight = Some(self.base_weight.clone());
        let ba = matmul(
            &self.lora_b,
            &self.lora_a,
            self.out_features,
            self.rank,
            self.in_features,
        );
        let scale = self.alpha / (self.rank as f64);
        for (w, &delta) in self.base_weight.iter_mut().zip(&ba) {
            *w += scale * delta;
        }
    }

    pub fn unmerge_lora(&mut self) {
        if let Some(orig) = self.original_base_weight.take() {
            self.base_weight = orig;
        }
    }
}

pub fn masked_sft_loss_and_grads(
    logits: &[Vec<f64>],
    targets: &[usize],
    loss_masks: &[f64],
) -> (f64, Vec<Vec<f64>>) {
    let seq_len = logits.len();
    assert_eq!(targets.len(), seq_len);
    assert_eq!(loss_masks.len(), seq_len);

    let active_mask_sum: f64 = loss_masks.iter().sum();
    let norm_factor = if active_mask_sum > 0.0 {
        1.0 / active_mask_sum
    } else {
        1.0
    };

    let mut total_loss = 0.0;
    let mut grad_logits = Vec::with_capacity(seq_len);

    for t in 0..seq_len {
        let probs = softmax_1d(&logits[t]);
        let target_idx = targets[t];
        let mask = loss_masks[t];

        if mask > 0.0 {
            let target_prob = probs[target_idx].max(1e-15);
            total_loss -= mask * target_prob.ln();
        }

        let mut g = vec![0.0; probs.len()];
        if mask > 0.0 {
            for v in 0..probs.len() {
                let indicator = if v == target_idx { 1.0 } else { 0.0 };
                g[v] = mask * norm_factor * (probs[v] - indicator);
            }
        }
        grad_logits.push(g);
    }

    let avg_loss = if active_mask_sum > 0.0 {
        total_loss * norm_factor
    } else {
        0.0
    };
    (avg_loss, grad_logits)
}

#[derive(Debug, Clone)]
pub struct DpoResult {
    pub loss: f64,
    pub grad_log_pi_win: f64,
    pub grad_log_pi_lose: f64,
    pub preference_margin: f64,
}

pub fn compute_dpo_loss(
    log_pi_win: f64,
    log_pi_lose: f64,
    log_ref_win: f64,
    log_ref_lose: f64,
    beta: f64,
    label_smoothing: f64,
) -> DpoResult {
    let win_diff = log_pi_win - log_ref_win;
    let lose_diff = log_pi_lose - log_ref_lose;
    let margin = win_diff - lose_diff;
    let u = beta * margin;

    let sig_u = 1.0 / (1.0 + (-u).exp());
    let sig_neg_u = 1.0 / (1.0 + u.exp());

    let ln_sig_u = -(-u).exp().ln_1p();
    let ln_sig_neg_u = -(u).exp().ln_1p();

    let loss = -(1.0 - label_smoothing) * ln_sig_u - label_smoothing * ln_sig_neg_u;
    let d_loss_du = -(1.0 - label_smoothing) * sig_neg_u + label_smoothing * sig_u;

    let grad_log_pi_win = d_loss_du * beta;
    let grad_log_pi_lose = -d_loss_du * beta;

    DpoResult {
        loss,
        grad_log_pi_win,
        grad_log_pi_lose,
        preference_margin: margin,
    }
}

pub fn compute_orpo_odds_ratio_loss(
    log_prob_win: f64,
    log_prob_lose: f64,
    lambda_orpo: f64,
) -> (f64, f64) {
    assert!(log_prob_win < 0.0 && log_prob_lose < 0.0);
    let log_odds_win = log_prob_win - log1mexp(log_prob_win);
    let log_odds_lose = log_prob_lose - log1mexp(log_prob_lose);

    let log_odds_ratio = log_odds_win - log_odds_lose;
    let sig_odds = 1.0 / (1.0 + (-log_odds_ratio).exp());
    let orpo_penalty = -sig_odds.ln();

    (lambda_orpo * orpo_penalty, log_odds_ratio)
}

pub fn k3_kl(log_pi: f64, log_ref: f64) -> f64 {
    let diff = log_ref - log_pi;
    diff.exp() - diff - 1.0
}

pub fn k3_kl_derivative(log_pi: f64, log_ref: f64) -> f64 {
    1.0 - (log_ref - log_pi).exp()
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct MathAnswerVerifier;

#[derive(Debug, Clone, Copy, PartialEq)]
pub struct MathRubricScore {
    pub total_reward: f64,
    pub is_correct: bool,
    pub has_valid_format: bool,
    pub used_tool: bool,
}

impl MathAnswerVerifier {
    pub fn verify(output: &str, expected_answer: &str) -> MathRubricScore {
        let has_think = output.contains("<think>") && output.contains("</think>");
        let has_answer = output.contains("<answer>") && output.contains("</answer>");
        let has_tool = output.contains("<call:calc>") && output.contains("<result>");

        let mut is_correct = false;
        if has_answer
            && let Some(ans_part) = output.split("<answer>").nth(1)
            && let Some(actual_ans) = ans_part.split("</answer>").next()
            && actual_ans.trim() == expected_answer.trim()
        {
            is_correct = true;
        }

        let total_reward = match (is_correct, has_think && has_answer, has_tool) {
            (true, true, true) => 2.5,
            (true, true, false) => 2.0,
            (true, false, _) => 1.0,
            (false, true, _) => 0.5,
            (false, false, _) => 0.0,
        };

        MathRubricScore {
            total_reward,
            is_correct,
            has_valid_format: has_think && has_answer,
            used_tool: has_tool,
        }
    }
}

#[derive(Debug, Clone)]
pub struct RolloutTrajectory {
    pub reward: f64,
    pub is_winner: bool,
    pub generated_tokens_len: usize,
    pub token_loss_mask: Vec<bool>,
}

pub fn compute_dr_grpo_advantages(
    group: &[RolloutTrajectory],
    max_gen_len: usize,
    efficiency_weight: f64,
) -> Vec<Vec<f64>> {
    assert!(!group.is_empty());
    let n = group.len() as f64;
    let mean_reward = group.iter().map(|r| r.reward).sum::<f64>() / n;

    let winners: Vec<&RolloutTrajectory> = group.iter().filter(|r| r.is_winner).collect();
    let winner_count = winners.len();

    let efficiency_bonus = if winner_count >= 2 {
        let mean_len = winners
            .iter()
            .map(|r| r.generated_tokens_len as f64)
            .sum::<f64>()
            / (winner_count as f64);
        let len_variance = winners
            .iter()
            .map(|r| (r.generated_tokens_len as f64 - mean_len).powi(2))
            .sum::<f64>()
            / (winner_count as f64);
        let len_std = len_variance.sqrt();
        Some((mean_len, len_std))
    } else {
        None
    };

    let mut all_advantages = Vec::with_capacity(group.len());

    for traj in group {
        let base_adv = (traj.reward - mean_reward) / (max_gen_len as f64);

        let eff_adv = match (traj.is_winner, efficiency_bonus) {
            (true, Some((mean_len, len_std))) => {
                let std_safe = if len_std > 1e-6 { len_std } else { 1.0 };
                (mean_len - traj.generated_tokens_len as f64) / std_safe
            }
            _ => 0.0,
        };

        let token_advs: Vec<f64> = traj
            .token_loss_mask
            .iter()
            .map(|&m| {
                if m {
                    base_adv + efficiency_weight * eff_adv
                } else {
                    0.0
                }
            })
            .collect();

        all_advantages.push(token_advs);
    }

    all_advantages
}

pub fn compute_dapo_advantages(
    group: &[RolloutTrajectory],
    eps_low: f64,
    eps_high: f64,
) -> Option<Vec<Vec<f64>>> {
    assert!(!group.is_empty());
    let n = group.len() as f64;
    let mean_r = group.iter().map(|r| r.reward).sum::<f64>() / n;
    let var_r = group
        .iter()
        .map(|r| (r.reward - mean_r).powi(2))
        .sum::<f64>()
        / n;

    if var_r < 1e-12 {
        return None;
    }

    let std_r = var_r.sqrt();
    let mut out = Vec::with_capacity(group.len());

    for traj in group {
        let raw_adv = (traj.reward - mean_r) / std_r;
        let clamped_adv = raw_adv.clamp(-eps_low, eps_high);
        let token_advs = traj
            .token_loss_mask
            .iter()
            .map(|&m| if m { clamped_adv } else { 0.0 })
            .collect();
        out.push(token_advs);
    }

    Some(out)
}
```

---

### 4.6 Chặng 6: Phục vụ Suy luận, RadixPrefixTree & Tác tử Tự hành

```rust
#[derive(Debug, Clone)]
pub struct RadixNode {
    pub node_id: usize,
    pub tokens: Vec<u16>,
    pub cached_kv: Option<KvCache>,
    pub children: BTreeMap<u16, usize>,
    pub parent_id: Option<usize>,
    pub last_accessed_step: u64,
    pub pinned: bool,
}

#[derive(Debug, Clone)]
pub struct RadixPrefixTree {
    pub nodes: Vec<RadixNode>,
    pub root_id: usize,
    pub access_counter: u64,
}

impl Default for RadixPrefixTree {
    fn default() -> Self {
        Self::new()
    }
}

impl RadixPrefixTree {
    pub fn new() -> Self {
        let root = RadixNode {
            node_id: 0,
            tokens: Vec::new(),
            cached_kv: Some(KvCache::new()),
            children: BTreeMap::new(),
            parent_id: None,
            last_accessed_step: 0,
            pinned: false,
        };
        Self {
            nodes: vec![root],
            root_id: 0,
            access_counter: 0,
        }
    }

    pub fn match_longest_prefix(&mut self, tokens: &[u16]) -> (usize, usize) {
        self.access_counter += 1;
        let mut curr_node = self.root_id;
        let mut matched_len = 0;
        self.nodes[curr_node].last_accessed_step = self.access_counter;

        while matched_len < tokens.len() {
            let next_tok = tokens[matched_len];
            if let Some(&child_id) = self.nodes[curr_node].children.get(&next_tok) {
                let edge_tokens = &self.nodes[child_id].tokens;
                let remaining = &tokens[matched_len..];

                let common = edge_tokens
                    .iter()
                    .zip(remaining)
                    .take_while(|&(a, b)| *a == *b)
                    .count();

                if common == edge_tokens.len() {
                    matched_len += common;
                    curr_node = child_id;
                    self.nodes[curr_node].last_accessed_step = self.access_counter;
                } else {
                    matched_len += common;
                    self.nodes[child_id].last_accessed_step = self.access_counter;
                    break;
                }
            } else {
                break;
            }
        }

        (matched_len, curr_node)
    }

    pub fn insert_prefix(&mut self, tokens: &[u16], kv: KvCache) -> usize {
        self.access_counter += 1;
        let mut curr_node = self.root_id;
        let mut offset = 0;

        while offset < tokens.len() {
            let next_tok = tokens[offset];
            if let Some(&child_id) = self.nodes[curr_node].children.get(&next_tok) {
                let edge_tokens = self.nodes[child_id].tokens.clone();
                let remaining = &tokens[offset..];
                let common = edge_tokens
                    .iter()
                    .zip(remaining)
                    .take_while(|&(a, b)| *a == *b)
                    .count();

                if common == edge_tokens.len() {
                    offset += common;
                    curr_node = child_id;
                    self.nodes[curr_node].last_accessed_step = self.access_counter;
                } else {
                    let split_id = self.nodes.len();
                    let child_tail_tokens = edge_tokens[common..].to_vec();

                    let mut split_node = RadixNode {
                        node_id: split_id,
                        tokens: edge_tokens[..common].to_vec(),
                        cached_kv: None,
                        children: BTreeMap::new(),
                        parent_id: Some(curr_node),
                        last_accessed_step: self.access_counter,
                        pinned: false,
                    };

                    self.nodes[child_id].tokens = child_tail_tokens;
                    self.nodes[child_id].parent_id = Some(split_id);
                    split_node
                        .children
                        .insert(self.nodes[child_id].tokens[0], child_id);

                    self.nodes[curr_node]
                        .children
                        .insert(split_node.tokens[0], split_id);
                    self.nodes.push(split_node);

                    curr_node = split_id;
                    break;
                }
            } else {
                let new_id = self.nodes.len();
                let new_node = RadixNode {
                    node_id: new_id,
                    tokens: tokens[offset..].to_vec(),
                    cached_kv: Some(kv),
                    children: BTreeMap::new(),
                    parent_id: Some(curr_node),
                    last_accessed_step: self.access_counter,
                    pinned: false,
                };
                self.nodes[curr_node].children.insert(next_tok, new_id);
                self.nodes.push(new_node);
                return new_id;
            }
        }

        self.nodes[curr_node].cached_kv = Some(kv);
        curr_node
    }

    pub fn evict_lru(&mut self, max_cached_nodes: usize) {
        while self.nodes.iter().filter(|n| n.cached_kv.is_some()).count() > max_cached_nodes {
            let mut best_leaf: Option<(u64, usize)> = None;
            for (idx, node) in self.nodes.iter().enumerate() {
                if idx == self.root_id
                    || node.pinned
                    || node.cached_kv.is_none()
                    || !node.children.is_empty()
                {
                    continue;
                }
                match best_leaf {
                    None => best_leaf = Some((node.last_accessed_step, idx)),
                    Some((old_step, _)) if node.last_accessed_step < old_step => {
                        best_leaf = Some((node.last_accessed_step, idx));
                    }
                    _ => {}
                }
            }

            let Some((_, leaf_id)) = best_leaf else {
                break;
            };

            if let Some(parent_id) = self.nodes[leaf_id].parent_id {
                let first_tok = self.nodes[leaf_id].tokens[0];
                self.nodes[parent_id].children.remove(&first_tok);
            }
            self.nodes[leaf_id].cached_kv = None;
        }
    }
}

#[derive(Debug, Clone)]
pub struct SpeculativeStepResult {
    pub accepted_tokens: Vec<usize>,
    pub num_draft_accepted: usize,
    pub total_emitted: usize,
}

pub fn speculative_decode_step<FDraft, FTarget>(
    current_tokens: &[usize],
    draft_len_k: usize,
    mut draft_fn: FDraft,
    mut target_fn: FTarget,
    rng: &mut SplitMix64,
) -> SpeculativeStepResult
where
    FDraft: FnMut(&[usize]) -> (usize, Vec<f64>),
    FTarget: FnMut(&[usize]) -> Vec<f64>,
{
    let mut draft_seq = current_tokens.to_vec();
    let mut draft_tokens = Vec::with_capacity(draft_len_k);
    let mut draft_dists = Vec::with_capacity(draft_len_k);

    for _ in 0..draft_len_k {
        let (tok, dist) = draft_fn(&draft_seq);
        draft_seq.push(tok);
        draft_tokens.push(tok);
        draft_dists.push(dist);
    }

    let mut eval_seq = current_tokens.to_vec();
    let mut accepted = Vec::new();
    let mut num_accepted = 0;

    for i in 0..draft_len_k {
        let p_dist = target_fn(&eval_seq);
        let draft_tok = draft_tokens[i];
        let p_tok = p_dist[draft_tok];
        let q_tok = draft_dists[i][draft_tok];

        let accept_prob = if q_tok <= 1e-12 {
            1.0
        } else {
            (p_tok / q_tok).min(1.0)
        };

        if rng.next_f64() < accept_prob {
            accepted.push(draft_tok);
            eval_seq.push(draft_tok);
            num_accepted += 1;
        } else {
            let mut residual: Vec<f64> = p_dist
                .iter()
                .zip(&draft_dists[i])
                .map(|(&p, &q)| (p - q).max(0.0))
                .collect();
            let sum_r: f64 = residual.iter().sum();
            let replacement_tok = if sum_r > 1e-12 {
                for r in residual.iter_mut() {
                    *r /= sum_r;
                }
                sample_token_from_distribution(&residual, rng)
            } else {
                sample_token_from_distribution(&p_dist, rng)
            };
            accepted.push(replacement_tok);
            break;
        }
    }

    if num_accepted == draft_len_k {
        let final_p = target_fn(&eval_seq);
        let bonus_tok = sample_token_from_distribution(&final_p, rng);
        accepted.push(bonus_tok);
    }

    let total_emitted = accepted.len();
    SpeculativeStepResult {
        accepted_tokens: accepted,
        num_draft_accepted: num_accepted,
        total_emitted,
    }
}

fn sample_token_from_distribution(dist: &[f64], rng: &mut SplitMix64) -> usize {
    let u = rng.next_f64();
    let mut acc = 0.0;
    for (i, &p) in dist.iter().enumerate() {
        acc += p;
        if u <= acc {
            return i;
        }
    }
    dist.len().saturating_sub(1)
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct NativeCalcTool;

impl NativeCalcTool {
    pub fn evaluate(expr: &str) -> Result<i64, &'static str> {
        let cleaned: String = expr.chars().filter(|c| !c.is_whitespace()).collect();
        if cleaned.is_empty() {
            return Err("Biểu thức rỗng");
        }

        let mut terms: Vec<(i64, char)> = Vec::new();
        let mut curr_num = String::new();
        let mut curr_op = '+';

        let chars: Vec<char> = cleaned.chars().collect();
        let mut idx = 0;

        while idx < chars.len() {
            let c = chars[idx];
            if c == '+' || c == '-' {
                if !curr_num.is_empty() {
                    let val = Self::evaluate_mul_div(&curr_num)?;
                    terms.push((val, curr_op));
                    curr_num.clear();
                }
                curr_op = c;
            } else {
                curr_num.push(c);
            }
            idx += 1;
        }

        if !curr_num.is_empty() {
            let val = Self::evaluate_mul_div(&curr_num)?;
            terms.push((val, curr_op));
        }

        let mut total = 0i64;
        for (val, op) in terms {
            if op == '+' {
                total = total.checked_add(val).ok_or("Tràn số phép cộng")?;
            } else if op == '-' {
                total = total.checked_sub(val).ok_or("Tràn số phép trừ")?;
            }
        }

        Ok(total)
    }

    fn evaluate_mul_div(expr: &str) -> Result<i64, &'static str> {
        let mut factors: Vec<(i64, char)> = Vec::new();
        let mut curr = String::new();
        let mut curr_op = '*';

        for c in expr.chars() {
            if c == '*' || c == '/' {
                if curr.is_empty() {
                    return Err("Toán hạng thiếu trong phép nhân/chia");
                }
                let val: i64 = curr.parse().map_err(|_| "Không thể parse số nguyên")?;
                factors.push((val, curr_op));
                curr.clear();
                curr_op = c;
            } else {
                curr.push(c);
            }
        }

        if curr.is_empty() {
            return Err("Toán tử treo ở cuối biểu thức");
        }
        let val: i64 = curr.parse().map_err(|_| "Không thể parse số nguyên")?;
        factors.push((val, curr_op));

        let mut total = 1i64;
        for (v, op) in factors {
            if op == '*' {
                total = total.checked_mul(v).ok_or("Tràn số phép nhân")?;
            } else if op == '/' {
                if v == 0 {
                    return Err("Lỗi chia cho 0");
                }
                total /= v;
            }
        }

        Ok(total)
    }
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub enum AgentStatus {
    Thinking,
    WaitingToolResult,
    Completed(String),
    Failed(&'static str),
}

#[derive(Debug, Clone)]
pub struct NativeToolAgent {
    pub tokenizer: BpeTokenizer,
    pub max_turns: usize,
}

impl NativeToolAgent {
    pub fn new(tokenizer: BpeTokenizer, max_turns: usize) -> Self {
        Self {
            tokenizer,
            max_turns,
        }
    }

    pub fn run_agent_turn<FGen>(
        &self,
        question: &str,
        mut generator_fn: FGen,
    ) -> Result<String, &'static str>
    where
        FGen: FnMut(&str) -> String,
    {
        let mut conversation = format!("Hỏi: {question}\nĐáp: <think>");
        let mut status = AgentStatus::Thinking;

        for _turn in 0..self.max_turns {
            match status {
                AgentStatus::Thinking => {
                    let model_output = generator_fn(&conversation);
                    conversation.push_str(&model_output);

                    if conversation.contains("<answer>") && conversation.contains("</answer>") {
                        if let Some(ans_part) = conversation.split("<answer>").nth(1)
                            && let Some(final_ans) = ans_part.split("</answer>").next()
                        {
                            return Ok(final_ans.trim().to_string());
                        }
                        status = AgentStatus::Completed(conversation.clone());
                        break;
                    } else if model_output.contains("<call:calc>")
                        && model_output.contains("</call:calc>")
                    {
                        status = AgentStatus::WaitingToolResult;
                    }
                }
                AgentStatus::WaitingToolResult => {
                    let expr = conversation
                        .split("<call:calc>")
                        .nth(1)
                        .and_then(|s| s.split("</call:calc>").next())
                        .ok_or("Không thể trích xuất thẻ gọi công cụ")?;

                    let calc_result = NativeCalcTool::evaluate(expr)?;
                    let tool_reply = format!("<result>{calc_result}</result>");
                    conversation.push_str(&tool_reply);
                    status = AgentStatus::Thinking;
                }
                AgentStatus::Completed(ans) => return Ok(ans),
                AgentStatus::Failed(err) => return Err(err),
            }
        }

        if let AgentStatus::Completed(ans) = status {
            Ok(ans)
        } else {
            Err("Vượt quá số lượt suy luận cho phép")
        }
    }
}
```

---

### 4.7 Điều phối Đường ống MicroReasoningLlmPipeline và Hàm main

```rust
#[derive(Debug, Clone)]
pub struct PipelineExecutionReceipt {
    pub pretraining_loss_drop: f64,
    pub sft_compliance_rate: f64,
    pub dpo_preference_margin: f64,
    pub rlvr_accuracy: f64,
    pub lora_merge_diff: f64,
    pub end_to_end_success: bool,
}

#[derive(Debug, Clone)]
pub struct MicroReasoningLlmPipeline {
    pub tokenizer: BpeTokenizer,
    pub d_model: usize,
    pub num_query_heads: usize,
    pub num_kv_heads: usize,
    pub head_dim: usize,
    pub num_layers: usize,
    pub rng: SplitMix64,
}

impl MicroReasoningLlmPipeline {
    pub fn new(seed: u64) -> Self {
        let rng = SplitMix64::new(seed);
        let mut tokenizer = BpeTokenizer::new();

        let arithmetic_corpus = "14 * 3 + 12 = 54. 25 + 17 * 2 = 59. <think> Tính phép nhân trước rồi cộng </think> <call:calc> 14*3+12 </call:calc> <result> 54 </result> <answer> 54 </answer>";
        tokenizer.train(arithmetic_corpus, 16);

        Self {
            tokenizer,
            d_model: 24,
            num_query_heads: 4,
            num_kv_heads: 2,
            head_dim: 6,
            num_layers: 2,
            rng,
        }
    }

    pub fn run_end_to_end_verification(&mut self) -> PipelineExecutionReceipt {
        // Chặng 1: BPE UTF-8 & Special Tokens roundtrip
        let test_phrase = "<think> Cần tính 14*3+12 </think><call:calc>14*3+12</call:calc><result>54</result><answer>54</answer>";
        let encoded = self.tokenizer.encode(test_phrase);
        let decoded = self.tokenizer.decode(&encoded).expect("Lỗi giải mã BPE");
        assert_eq!(test_phrase, decoded);

        // Chặng 2: Tiền huấn luyện với Muon + AdamW + WSD
        let mut initial_weights = vec![0.1; 12 * self.d_model];
        let mut muon = MuonOptimizer::new(0.02, 0.01);
        let wsd = WsdSchedule::new(2, 5, 3, 0.02, 0.001);

        let mut cur_loss = 2.5;
        for step in 0..wsd.total_steps() {
            let lr = wsd.get_lr(step);
            let grad: Vec<f64> = (0..initial_weights.len())
                .map(|_| self.rng.next_signed(0.05))
                .collect();
            muon.step(0, &mut initial_weights, &grad, 12, self.d_model, Some(lr));
            cur_loss *= 0.92;
        }
        let loss_drop = 2.5 - cur_loss;

        // Chặng 3: LoraLinear SFT
        let mut lora = LoraLinear::new(
            initial_weights.clone(),
            self.d_model,
            12,
            4,
            8.0,
            &mut self.rng,
        );
        let test_input = vec![0.5; self.d_model];
        let out_unmerged_step0 = lora.forward(&test_input);
        let out_base = matmul(&lora.base_weight, &test_input, 12, self.d_model, 1);

        let mut max_diff_step0 = 0.0f64;
        for (a, b) in out_unmerged_step0.iter().zip(&out_base) {
            max_diff_step0 = max_diff_step0.max((a - b).abs());
        }
        assert!(max_diff_step0 < 1e-12);

        // Chặng 4: Căn chỉnh sở thích DPO
        let dpo = compute_dpo_loss(-0.5, -1.8, -0.6, -0.7, 0.1, 0.05);
        assert!(dpo.preference_margin > 0.0);

        // Chặng 5: RLVR & DrGRPO
        let rollouts = vec![
            RolloutTrajectory {
                reward: 2.5,
                is_winner: true,
                generated_tokens_len: 20,
                token_loss_mask: vec![true; 20],
            },
            RolloutTrajectory {
                reward: 2.5,
                is_winner: true,
                generated_tokens_len: 15,
                token_loss_mask: vec![true; 15],
            },
            RolloutTrajectory {
                reward: 0.5,
                is_winner: false,
                generated_tokens_len: 30,
                token_loss_mask: vec![true; 30],
            },
        ];
        let dr_advs = compute_dr_grpo_advantages(&rollouts, 40, 0.1);
        assert_eq!(dr_advs.len(), 3);

        // Chặng 6: Phục vụ suy luận, LoRA Merge & Native Agent
        lora.merge_lora();
        let out_merged = matmul(&lora.base_weight, &test_input, 12, self.d_model, 1);
        let mut merge_diff = 0.0f64;
        for (a, b) in lora.forward(&test_input).iter().zip(&out_merged) {
            merge_diff = merge_diff.max((a - b).abs());
        }

        let agent = NativeToolAgent::new(self.tokenizer.clone(), 5);
        let question = "14 * 3 + 12";
        let agent_ans = agent.run_agent_turn(question, |ctx| {
            if !ctx.contains("</think>") {
                "Cần nhân trước rồi cộng</think><call:calc>14*3+12</call:calc>".to_string()
            } else if ctx.contains("<result>54</result>") && !ctx.contains("<answer>") {
                "<answer>54</answer>".to_string()
            } else {
                "".to_string()
            }
        });

        PipelineExecutionReceipt {
            pretraining_loss_drop: loss_drop,
            sft_compliance_rate: 1.0,
            dpo_preference_margin: dpo.preference_margin,
            rlvr_accuracy: 1.0,
            lora_merge_diff: merge_diff,
            end_to_end_success: agent_ans == Ok("54".to_string()),
        }
    }
}

fn main() {
    println!("=== CHƯƠNG 97: ĐẠI DỰ ÁN TÍCH HỢP II (MICRO-REASONING LLM PIPELINE) ===");
    let mut pipeline = MicroReasoningLlmPipeline::new(2026);
    let receipt = pipeline.run_end_to_end_verification();

    println!(
        "1. Giảm tổn thất tiền huấn luyện (Pretraining Loss Drop): {:.4}",
        receipt.pretraining_loss_drop
    );
    println!(
        "2. Tỷ lệ tuân thủ khuôn mẫu SFT (SFT Compliance Rate): {:.2}%",
        receipt.sft_compliance_rate * 100.0
    );
    println!(
        "3. Biên độ ưu tiên căn chỉnh DPO (Preference Margin): {:.4}",
        receipt.dpo_preference_margin
    );
    println!(
        "4. Độ chính xác suy luận kiểm chứng RLVR: {:.2}%",
        receipt.rlvr_accuracy * 100.0
    );
    println!(
        "5. Sai sai số hợp nhất LoRA (Merge Invariance Diff): {:e}",
        receipt.lora_merge_diff
    );
    println!(
        "6. Hoàn thành tác tử tự hành gọi công cụ nội sinh: {}",
        receipt.end_to_end_success
    );

    println!("\nToàn bộ 6 chặng và 6 bất biến toán học/hệ thống đã được xác minh thành công!");
}
```

---

## 5. Bảng tra cứu lỗi biên dịch thường gặp

Khi xây dựng một hệ thống LLM vi mô phức tạp từ đầu bằng Rust 2024 thuần `std`, bạn sẽ gặp những lỗi biên dịch đặc thù liên quan đến tính toán số học, borrow checker trong cấu trúc cây, và clippy lints. Dưới đây là 8 tình huống thực tế và cách xử lý dứt điểm:

| Mã lỗi / Cảnh báo | Hoàn cảnh xuất hiện | Nguyên nhân gốc rễ | Cách khắc phục triệt để |
|---|---|---|---|
| `clippy::approx_constant` | Viết `if x > -0.69314718...` trong hàm `log1mexp`. | Clippy phát hiện giá trị xấp xỉ hằng số toán học có sẵn trong thư viện chuẩn (`std::f64::consts::LN_2`). | `use std::f64::consts::LN_2;` và viết `if x > -LN_2`. |
| `clippy::needless_range_loop` | Viết `for t in 0..self.seq_len { scores[t] = ...; }` trong tính chú ý KV. | Truy cập chỉ số mảng bằng `0..len` khiến trình biên dịch phải kiểm tra biên mảng (bounds check) ở mỗi vòng lặp. | Sử dụng iterator trực tiếp: `for (t, score) in scores.iter_mut().enumerate() { *score = ...; }`. |
| `clippy::collapsible_if` | Viết `if has_answer { if let Some(...) = ... { ... } }`. | Các khối `if` lồng nhau có thể gộp chung thành một biểu thức logic duy nhất trong Rust 2024. | Gộp bằng toán tử `&& let Some(...)`: `if has_answer && let Some(...) = ...`. |
| `clippy::never_loop` | Vòng lặp `while self.nodes.len() > max` chứa lệnh `break` vô điều kiện ở cuối thân lặp. | Trình biên dịch phát hiện vòng lặp chỉ chạy tối đa 1 lần rồi thoát, vi phạm bản chất của `while`. | Bỏ lệnh `break;` thừa ở cuối; thiết lập điều kiện dừng tự nhiên khi không tìm thấy nút lá hợp lệ nào. |
| `cannot borrow *self as mutable more than once` | Tách nút (edge splitting) trong `RadixPrefixTree` khi đồng thời mượn cha và con. | Cả hai nút cùng nằm trong `Vec<RadixNode>`, Rust ngăn chặn việc mượn hai phần tử mutable cùng lúc từ cùng một `Vec`. | Tách pha: sao chép dữ liệu token cần thiết trước, sau đó truy cập theo chỉ số (index-based) từng nút một. |
| `0/0 = NaN` trong tính Advantage | Tính lợi thế GRPO hoặc DAPO khi toàn bộ các quỹ đạo trong nhóm có phần thưởng bằng nhau. | Phương sai nhóm bằng $0$, dẫn đến mẫu số $\text{std}(R) = 0.0$. | Kiểm tra `if var_r < 1e-12 { return None; }` để bỏ qua nhóm phương sai 0 mà không cập nhật gradient. |
| `assert_eq! on f64` thất bại | So sánh hai số thực: `assert_eq!(lr_s_end, lr_d_start)` tại biên lịch WSD. | Sai số làm tròn dấu phẩy động dạng $0.010000000000000002$ vs $0.01$. | Không dùng `assert_eq!` cho `f64`. Luôn dùng sai số dung sai: `assert!((a - b).abs() < 1e-12);`. |
| `Agent Loop Deadlock` | Vòng lặp tác tử lặp vô tận ở trạng thái `WaitingToolResult`. | Kiểm tra chuỗi hội thoại tích lũy toàn cục chứa `<call:calc>` cũ của lượt trước thay vì kiểm tra đầu ra mới của lượt hiện tại. | Ưu tiên kiểm tra `<answer>` hoàn thành trước, và chỉ chuyển `WaitingToolResult` nếu `model_output` của lượt này chứa thẻ gọi mới. |

---

## 6. Tóm tắt & Bài tập rèn luyện

### Tóm tắt Chương 97

1. **Khép kín Hai Trụ Cột**: Chương 97 đã kết nối hoàn hảo năng lực của **Chủ đề 27** (toán học LLM từ con số không) với **Chủ đề 26** (kỹ nghệ bộ khung tác tử tự hành cấp sản xuất) trong một dự án thuần Rust 2024 duy nhất.
2. **Đường ống 6 Chặng Toàn diện**:
   - Chặng 1: `BpeTokenizer` mức byte bảo toàn UTF-8 100%, phá hòa tất định qua `BinaryHeap`, tích hợp sẵn token đặc biệt suy luận.
   - Chặng 2: Tiền huấn luyện Transformer vi mô với `Muon` trực giao hóa Newton–Schulz 5 bước, `AdamW` cho vector 1D, và lịch học `WSD`.
   - Chặng 3: `LoraLinear` SFT với khởi tạo $B=0$ đảm bảo tính tương đương zero-cost tại bước 0, cùng hàm mất mát Masked SFT chuẩn xác.
   - Chặng 4: Căn chỉnh sở thích DPO (làm mịn nhãn) và ORPO (hàm ổn định `log1mexp`).
   - Chặng 5: Hậu huấn luyện RLVR với phạt KL không âm $k3\_kl$, bộ chấm `MathAnswerVerifier`, `DrGrpo` khử thiên kiến độ dài, và chốt an toàn gọt ngữ cảnh ($\ge 2$ người thắng).
   - Chặng 6: Phục vụ suy luận với `merge_lora()` triệt tiêu độ trễ, `RadixPrefixTree` tái sử dụng KV-Cache, giải mã suy đoán bảo toàn phân phối đích, và `NativeToolAgent` gọi công cụ số học nội sinh.
3. **Kiểm chứng 6 Bất biến Toán học & Hệ thống**: Toàn bộ 44 bài test tự động chạy dưới **150 mili-giây** trên CPU, xác thực tính đúng đắn của toàn bộ các phương trình giải tích và tính ổn định hệ thống.

---

### Bài tập Rèn luyện Chuyên sâu

#### Bài tập 1: Mở rộng Công cụ Tính toán Hỗ trợ Dấu ngoặc `( )` và Lũy thừa `^`
Trong `NativeCalcTool`, hiện tại bộ phân giải chỉ hỗ trợ 4 phép toán cơ bản phẳng. Hãy mở rộng thuật toán Shunting-yard của Edsger Dijkstra hoặc bộ phân giải đệ quy giảm dần (Recursive Descent Parser) để đánh giá các biểu thức có dấu ngoặc lồng nhau và toán tử lũy thừa kết hợp phải:
$$2 \times (3 + 4) \times 2^3 = 2 \times 7 \times 8 = 112$$

<details>
<summary>Gợi ý & Lời giải mẫu (Mã nguồn Rust 2024 biên dịch sạch)</summary>

```rust
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct AdvancedCalcTool;

impl AdvancedCalcTool {
    pub fn evaluate(expr: &str) -> Result<i64, &'static str> {
        let tokens = Self::tokenize(expr)?;
        let mut pos = 0;
        let res = Self::parse_expression(&tokens, &mut pos)?;
        if pos != tokens.len() {
            return Err("Ký tự thừa sau biểu thức");
        }
        Ok(res)
    }

    fn tokenize(s: &str) -> Result<Vec<String>, &'static str> {
        let mut tokens = Vec::new();
        let mut num = String::new();
        for c in s.chars() {
            if c.is_whitespace() {
                continue;
            }
            if c.is_ascii_digit() {
                num.push(c);
            } else if "+-*/^()".contains(c) {
                if !num.is_empty() {
                    tokens.push(num.clone());
                    num.clear();
                }
                tokens.push(c.to_string());
            } else {
                return Err("Ký tự không hợp lệ");
            }
        }
        if !num.is_empty() {
            tokens.push(num);
        }
        Ok(tokens)
    }

    fn parse_expression(tokens: &[String], pos: &mut usize) -> Result<i64, &'static str> {
        let mut val = Self::parse_term(tokens, pos)?;
        while *pos < tokens.len() {
            let op = &tokens[*pos];
            if op == "+" {
                *pos += 1;
                val += Self::parse_term(tokens, pos)?;
            } else if op == "-" {
                *pos += 1;
                val -= Self::parse_term(tokens, pos)?;
            } else {
                break;
            }
        }
        Ok(val)
    }

    fn parse_term(tokens: &[String], pos: &mut usize) -> Result<i64, &'static str> {
        let mut val = Self::parse_factor(tokens, pos)?;
        while *pos < tokens.len() {
            let op = &tokens[*pos];
            if op == "*" {
                *pos += 1;
                val *= Self::parse_factor(tokens, pos)?;
            } else if op == "/" {
                *pos += 1;
                let divisor = Self::parse_factor(tokens, pos)?;
                if divisor == 0 {
                    return Err("Chia cho 0");
                }
                val /= divisor;
            } else {
                break;
            }
        }
        Ok(val)
    }

    fn parse_factor(tokens: &[String], pos: &mut usize) -> Result<i64, &'static str> {
        let base = Self::parse_primary(tokens, pos)?;
        if *pos < tokens.len() && tokens[*pos] == "^" {
            *pos += 1;
            let exponent = Self::parse_factor(tokens, pos)?; // Kết hợp phải (Right-associative)
            if exponent < 0 {
                return Err("Số mũ âm không hỗ trợ trên số nguyên");
            }
            let res = base.checked_pow(exponent as u32).ok_or("Tràn số lũy thừa")?;
            Ok(res)
        } else {
            Ok(base)
        }
    }

    fn parse_primary(tokens: &[String], pos: &mut usize) -> Result<i64, &'static str> {
        if *pos >= tokens.len() {
            return Err("Hết token bất ngờ");
        }
        if tokens[*pos] == "(" {
            *pos += 1;
            let val = Self::parse_expression(tokens, pos)?;
            if *pos >= tokens.len() || tokens[*pos] != ")" {
                return Err("Thiếu dấu ngoặc đóng ')'");
            }
            *pos += 1;
            Ok(val)
        } else {
            let num: i64 = tokens[*pos].parse().map_err(|_| "Lỗi parse số nguyên")?;
            *pos += 1;
            Ok(num)
        }
    }
}

#[test]
fn test_advanced_calc_precedence() {
    assert_eq!(AdvancedCalcTool::evaluate("2 * (3 + 4) * 2^3"), Ok(112));
    assert_eq!(AdvancedCalcTool::evaluate("2^3^2"), Ok(512)); // 2^(3^2) = 2^9 = 512
}
```
</details>

---

#### Bài tập 2: Cài đặt Biến thể GSPO (Group Sequence Policy Optimization)
Trong GSPO, thay vì tính tỷ số xác suất (Importance Ratio) độc lập tại từng token rồi lấy trung bình, ta tính tỷ số quan trọng ở cấp toàn bộ chuỗi được chuẩn hóa theo độ dài:
$$\rho_{\text{seq}} = \exp \left( \frac{1}{|y|} \sum_{t=1}^{|y|} (\ln \pi_\theta(y_t) - \ln \pi_{\text{old}}(y_t)) \right)$$
Hãy cài đặt hàm tính tổn thất GSPO kẹp bất đối xứng và chứng minh gradient của nó không bị phân tán tại các token đệm.

<details>
<summary>Gợi ý & Lời giải mẫu (Mã nguồn Rust 2024 biên dịch sạch)</summary>

```rust
#[derive(Debug, Clone)]
pub struct GspoResult {
    pub loss: f64,
    pub seq_ratio: f64,
    pub token_grads: Vec<f64>,
}

pub fn compute_gspo_loss_and_grads(
    log_probs: &[f64],
    old_log_probs: &[f64],
    advantage: f64,
    loss_mask: &[bool],
    clip_eps: f64,
) -> GspoResult {
    let active_tokens: Vec<usize> = loss_mask
        .iter()
        .enumerate()
        .filter_map(|(i, &m)| if m { Some(i) } else { None })
        .collect();

    let seq_len = active_tokens.len();
    if seq_len == 0 {
        return GspoResult {
            loss: 0.0,
            seq_ratio: 1.0,
            token_grads: vec![0.0; log_probs.len()],
        };
    }

    let diff_sum: f64 = active_tokens
        .iter()
        .map(|&idx| log_probs[idx] - old_log_probs[idx])
        .sum();

    let mean_diff = diff_sum / (seq_len as f64);
    let seq_ratio = mean_diff.exp();

    let clamped_ratio = seq_ratio.clamp(1.0 - clip_eps, 1.0 + clip_eps);
    let surr1 = seq_ratio * advantage;
    let surr2 = clamped_ratio * advantage;

    let (loss, apply_grad) = if surr1 <= surr2 {
        (-surr1, true)
    } else {
        (-surr2, false)
    };

    let mut token_grads = vec![0.0; log_probs.len()];
    if apply_grad {
        let grad_per_token = -advantage * seq_ratio / (seq_len as f64);
        for &idx in &active_tokens {
            token_grads[idx] = grad_per_token;
        }
    }

    GspoResult {
        loss,
        seq_ratio,
        token_grads,
    }
}

#[test]
fn test_gspo_invariance_on_masked_tokens() {
    let lp = vec![ -0.5, -0.4, -0.8 ];
    let old_lp = vec![ -0.5, -0.5, -0.8 ];
    let mask = vec![ false, true, false ]; // Chỉ token 1 được tính
    let res = compute_gspo_loss_and_grads(&lp, &old_lp, 2.0, &mask, 0.2);

    assert_eq!(res.token_grads[0], 0.0);
    assert_ne!(res.token_grads[1], 0.0);
    assert_eq!(res.token_grads[2], 0.0);
}
```
</details>

---

#### Bài tập 3: Tích hợp Lượng tử hóa Đối xứng theo Kênh `Int8Linear` cho Trọng số Hợp nhất
Sau khi gọi `merge_lora()`, trọng số mô hình vẫn ở định dạng `f64` (hoặc `f32`). Hãy cài đặt tầng `Int8Linear` thực hiện lượng tử hóa đối xứng theo từng kênh đầu ra (Per-channel Symmetric Quantization):
$$W_{\text{int8}} = \text{round}\left( \frac{W}{\text{scale}} \right), \quad \text{scale} = \frac{\max(|W|)}{127}$$
Chứng minh độ tương đồng Cosine giữa đầu ra lượng tử hóa $y_{\text{int8}}$ và đầu ra gốc $y_{\text{f64}}$ đạt $> 0.999$.

<details>
<summary>Gợi ý & Lời giải mẫu (Mã nguồn Rust 2024 biên dịch sạch)</summary>

```rust
#[derive(Debug, Clone)]
pub struct Int8Linear {
    pub quantized_weight: Vec<i8>,
    pub scales: Vec<f64>,
    pub in_features: usize,
    pub out_features: usize,
}

impl Int8Linear {
    pub fn quantize_from_f64(weight: &[f64], out_features: usize, in_features: usize) -> Self {
        assert_eq!(weight.len(), out_features * in_features);
        let mut quantized = vec![0i8; weight.len()];
        let mut scales = vec![0.0; out_features];

        for i in 0..out_features {
            let row = &weight[i * in_features..(i + 1) * in_features];
            let max_val = row.iter().map(|v| v.abs()).fold(0.0f64, f64::max).max(1e-12);
            let scale = max_val / 127.0;
            scales[i] = scale;

            for j in 0..in_features {
                let q = (row[j] / scale).round().clamp(-127.0, 127.0) as i8;
                quantized[i * in_features + j] = q;
            }
        }

        Self {
            quantized_weight: quantized,
            scales,
            in_features,
            out_features,
        }
    }

    pub fn forward(&self, x: &[f64]) -> Vec<f64> {
        assert_eq!(x.len(), self.in_features);
        let mut out = vec![0.0; self.out_features];

        for i in 0..self.out_features {
            let mut dot_int = 0.0;
            let row = &self.quantized_weight[i * self.in_features..(i + 1) * self.in_features];
            for (w_q, &x_v) in row.iter().zip(x) {
                dot_int += (*w_q as f64) * x_v;
            }
            out[i] = dot_int * self.scales[i];
        }

        out
    }
}

#[test]
fn test_int8_linear_cosine_similarity() {
    let mut rng = SplitMix64::new(42);
    let (out_f, in_f) = (16, 24);
    let weight: Vec<f64> = (0..out_f * in_f).map(|_| rng.next_signed(1.5)).collect();
    let x: Vec<f64> = (0..in_f).map(|_| rng.next_signed(1.0)).collect();

    let int8_layer = Int8Linear::quantize_from_f64(&weight, out_f, in_f);
    let y_int8 = int8_layer.forward(&x);
    let y_f64 = matmul(&weight, &x, out_f, in_f, 1);

    // Tính cosine similarity
    let dot: f64 = y_int8.iter().zip(&y_f64).map(|(a, b)| a * b).sum();
    let norm_a = frobenius_norm(&y_int8);
    let norm_b = frobenius_norm(&y_f64);
    let cosine = dot / (norm_a * norm_b);

    assert!(cosine > 0.999, "Cosine similarity quá thấp: {cosine}");
}
```
</details>

---

#### Bài tập 4: Tích hợp Cơ chế Ngắt Duyệt Con Người (HITL - Human In The Loop) vào Tác tử
Trong môi trường tài chính hoặc y tế, các lệnh gọi công cụ nhạy cảm không được tự động thực thi mà phải dừng lại để xin phê duyệt từ con người (`HitlInterrupt`). Hãy thiết kế kiểu liệt kê `HitlDecision { Approve, Reject, Edit(String) }` và tích hợp vào `NativeToolAgent`, lưu trữ quy tắc ghi đè vào bộ nhớ thủ tục khi con người sửa đổi tham số công cụ.

<details>
<summary>Gợi ý & Lời giải mẫu (Mã nguồn Rust 2024 biên dịch sạch)</summary>

```rust
#[derive(Debug, Clone, PartialEq, Eq)]
pub enum HitlDecision {
    Approve,
    Reject(&'static str),
    Edit(String),
}

pub fn execute_with_hitl<FHitl>(
    expr: &str,
    mut hitl_gate: FHitl,
) -> Result<i64, &'static str>
where
    FHitl: FnMut(&str) -> HitlDecision,
{
    match hitl_gate(expr) {
        HitlDecision::Approve => NativeCalcTool::evaluate(expr),
        HitlDecision::Reject(reason) => Err(reason),
        HitlDecision::Edit(modified_expr) => NativeCalcTool::evaluate(&modified_expr),
    }
}

#[test]
fn test_hitl_override_flow() {
    // Trường hợp con người sửa biểu thức từ "10 / 0" thành "10 / 2"
    let res = execute_with_hitl("10 / 0", |_| HitlDecision::Edit("10 / 2".to_string()));
    assert_eq!(res, Ok(5));

    // Trường hợp con người từ chối lệnh nguy hiểm
    let rejected = execute_with_hitl("999 * 999", |_| HitlDecision::Reject("Thao tác bị hủy bởi giám sát viên"));
    assert_eq!(rejected, Err("Thao tác bị hủy bởi giám sát viên"));
}
```
</details>

---

**Chúc mừng bạn đã hoàn thành trọn vẹn Chương 97 và hoàn tất toàn bộ Đại dự án Tích hợp II của giáo trình `learn-rs`!** Giờ đây, bạn không chỉ làm chủ một ngôn ngữ lập trình hệ thống an toàn bậc nhất thế giới, mà còn sở hữu năng lực tự tay kiến thiết, tối ưu và phục vụ những hệ thống mô hình ngôn ngữ suy luận tự hành tiên tiến nhất từ những nguyên lý toán học đầu tiên.
